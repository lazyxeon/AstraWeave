//! HDRI Catalog Loader
//!
//! Loads and queries the HDRI environment catalog (`hdri/hdri_catalog.toml`)
//! to determine which HDRI environment map to use based on biome type and
//! time of day.
//!
//! The catalog provides:
//! - A mapping of HDRI names → file paths + metadata (biome affinity, time-of-day, intensity)
//! - A fallback matrix ensuring every biome×time combination has a valid HDRI
//! - Sky color hints for procedural sky blending

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Discrete time-of-day categories for HDRI selection.
///
/// Unlike [`crate::environment::TimeOfDay`] which tracks continuous game-hours (0-24),
/// `DayPeriod` categorizes time into four discrete buckets for HDRI lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DayPeriod {
    Day,
    Morning,
    Evening,
    Night,
}

impl DayPeriod {
    /// Parse from string (case-insensitive)
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "day" => Some(Self::Day),
            "morning" | "sunrise" | "dawn" => Some(Self::Morning),
            "evening" | "sunset" | "dusk" => Some(Self::Evening),
            "night" | "midnight" => Some(Self::Night),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Morning => "morning",
            Self::Evening => "evening",
            Self::Night => "night",
        }
    }

    /// All day period variants
    pub fn all() -> &'static [DayPeriod] {
        &[Self::Day, Self::Morning, Self::Evening, Self::Night]
    }

    /// Convert from continuous game-hours (0.0 - 24.0) into a discrete period.
    ///
    /// - Morning: 5:00 – 9:59
    /// - Day: 10:00 – 16:59
    /// - Evening: 17:00 – 20:59
    /// - Night: 21:00 – 4:59
    pub fn from_game_hours(hours: f32) -> Self {
        let h = hours.rem_euclid(24.0);
        if (5.0..10.0).contains(&h) {
            Self::Morning
        } else if (10.0..17.0).contains(&h) {
            Self::Day
        } else if (17.0..21.0).contains(&h) {
            Self::Evening
        } else {
            Self::Night
        }
    }
}

/// A single HDRI entry from the catalog
#[derive(Debug, Clone)]
pub struct HdriEntry {
    /// Logical name (e.g., "kloppenheim_daytime")
    pub name: String,
    /// File path relative to the HDRI root (e.g., "polyhaven/kloppenheim_02_puresky_2k.hdr")
    pub file: String,
    /// Time of day this HDRI is best suited for
    pub time_of_day: DayPeriod,
    /// Biomes this HDRI matches
    pub biomes: Vec<String>,
    /// Exposure/intensity multiplier
    pub intensity: f32,
    /// Sky color hint \[R, G, B\] for procedural blending
    pub sky_color: [f32; 3],
    /// SHA-256 of the file bytes. Empty until the catalog row carries a digest.
    /// `catalog_files_match_sha256` checks every non-empty value.
    pub sha256: String,
}

/// Raw TOML structure (private, for deserialization only)
#[derive(Deserialize)]
struct CatalogDoc {
    catalog: CatalogMeta,
    hdri: Vec<HdriRaw>,
    fallback: HashMap<String, String>,
}

#[derive(Deserialize)]
struct CatalogMeta {
    #[allow(dead_code)]
    version: u32,
    default: String,
}

#[derive(Deserialize)]
struct HdriRaw {
    name: String,
    file: String,
    time_of_day: String,
    biomes: Vec<String>,
    intensity: f32,
    sky_color: [f32; 3],
    #[serde(default)]
    sha256: String,
}

/// Loaded HDRI catalog providing biome × time-of-day lookups
#[derive(Debug, Clone)]
pub struct HdriCatalog {
    /// All HDRI entries keyed by name
    entries: HashMap<String, HdriEntry>,
    /// Fallback matrix: "biome_time" → HDRI name
    fallbacks: HashMap<String, String>,
    /// Default HDRI name (used when no match found)
    default_name: String,
    /// Root directory for resolving HDRI file paths
    hdri_root: PathBuf,
}

impl HdriCatalog {
    /// Load the HDRI catalog from a TOML file.
    ///
    /// # Arguments
    /// * `catalog_path` — Path to `hdri_catalog.toml`
    /// * `hdri_root` — Root directory for resolving relative HDRI file paths
    ///   (typically `assets/hdri/`)
    pub fn load(catalog_path: &Path, hdri_root: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(catalog_path)
            .with_context(|| format!("Failed to read HDRI catalog: {}", catalog_path.display()))?;

        let doc: CatalogDoc = toml::from_str(&content)
            .with_context(|| format!("Failed to parse HDRI catalog: {}", catalog_path.display()))?;

        let mut entries = HashMap::new();

        for raw in doc.hdri {
            let time_of_day = DayPeriod::from_str_loose(&raw.time_of_day).ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid time_of_day '{}' for HDRI '{}'",
                    raw.time_of_day,
                    raw.name
                )
            })?;

            entries.insert(
                raw.name.clone(),
                HdriEntry {
                    name: raw.name,
                    file: raw.file,
                    time_of_day,
                    biomes: raw.biomes,
                    intensity: raw.intensity,
                    sky_color: raw.sky_color,
                    sha256: raw.sha256,
                },
            );
        }

        Ok(Self {
            entries,
            fallbacks: doc.fallback,
            default_name: doc.catalog.default,
            hdri_root: hdri_root.to_path_buf(),
        })
    }

    /// Find the best HDRI for the given biome and time of day.
    ///
    /// Lookup order:
    /// 1. Direct match: HDRI whose `biomes` list contains `biome` AND `time_of_day` matches
    /// 2. Fallback matrix: `"{biome}_{time}"` key in `[fallback]` section
    /// 3. Default HDRI from `[catalog].default`
    pub fn resolve(&self, biome: &str, time: DayPeriod) -> Option<&HdriEntry> {
        // 1. Direct match — find the first HDRI that matches both biome and time
        let direct = self.entries.values().find(|e| {
            e.time_of_day == time && e.biomes.iter().any(|b| b.eq_ignore_ascii_case(biome))
        });
        if direct.is_some() {
            return direct;
        }

        // 2. Fallback matrix
        let key = format!("{}_{}", biome, time.as_str());
        if let Some(name) = self.fallbacks.get(&key) {
            if let Some(entry) = self.entries.get(name) {
                return Some(entry);
            }
        }

        // 3. Default
        self.entries.get(&self.default_name)
    }

    /// Get the absolute file path for an HDRI entry
    pub fn resolve_path(&self, entry: &HdriEntry) -> PathBuf {
        self.hdri_root.join(&entry.file)
    }

    /// Convenience: resolve biome + time → absolute file path
    pub fn resolve_hdri_path(&self, biome: &str, time: DayPeriod) -> Option<PathBuf> {
        self.resolve(biome, time)
            .map(|entry| self.resolve_path(entry))
    }

    /// Get all catalog entries
    pub fn entries(&self) -> impl Iterator<Item = &HdriEntry> {
        self.entries.values()
    }

    /// Get entry by name
    pub fn get(&self, name: &str) -> Option<&HdriEntry> {
        self.entries.get(name)
    }

    /// Check if an HDRI file exists on disk
    pub fn validate_entry(&self, entry: &HdriEntry) -> bool {
        self.resolve_path(entry).exists()
    }

    /// Validate all entries exist on disk, returning names of missing HDRIs
    pub fn validate_all(&self) -> Vec<String> {
        self.entries
            .values()
            .filter(|e| !self.validate_entry(e))
            .map(|e| e.name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_period_parse() {
        assert_eq!(DayPeriod::from_str_loose("day"), Some(DayPeriod::Day));
        assert_eq!(DayPeriod::from_str_loose("Day"), Some(DayPeriod::Day));
        assert_eq!(
            DayPeriod::from_str_loose("morning"),
            Some(DayPeriod::Morning)
        );
        assert_eq!(
            DayPeriod::from_str_loose("sunrise"),
            Some(DayPeriod::Morning)
        );
        assert_eq!(
            DayPeriod::from_str_loose("evening"),
            Some(DayPeriod::Evening)
        );
        assert_eq!(
            DayPeriod::from_str_loose("sunset"),
            Some(DayPeriod::Evening)
        );
        assert_eq!(DayPeriod::from_str_loose("night"), Some(DayPeriod::Night));
        assert_eq!(
            DayPeriod::from_str_loose("midnight"),
            Some(DayPeriod::Night)
        );
        assert_eq!(DayPeriod::from_str_loose("invalid"), None);
    }

    #[test]
    fn test_day_period_as_str() {
        assert_eq!(DayPeriod::Day.as_str(), "day");
        assert_eq!(DayPeriod::Morning.as_str(), "morning");
        assert_eq!(DayPeriod::Evening.as_str(), "evening");
        assert_eq!(DayPeriod::Night.as_str(), "night");
    }

    #[test]
    fn test_day_period_all() {
        assert_eq!(DayPeriod::all().len(), 4);
    }

    #[test]
    fn test_from_game_hours() {
        assert_eq!(DayPeriod::from_game_hours(12.0), DayPeriod::Day);
        assert_eq!(DayPeriod::from_game_hours(7.0), DayPeriod::Morning);
        assert_eq!(DayPeriod::from_game_hours(18.0), DayPeriod::Evening);
        assert_eq!(DayPeriod::from_game_hours(23.0), DayPeriod::Night);
        assert_eq!(DayPeriod::from_game_hours(2.0), DayPeriod::Night);
        // Edge cases
        assert_eq!(DayPeriod::from_game_hours(5.0), DayPeriod::Morning);
        assert_eq!(DayPeriod::from_game_hours(10.0), DayPeriod::Day);
        assert_eq!(DayPeriod::from_game_hours(17.0), DayPeriod::Evening);
        assert_eq!(DayPeriod::from_game_hours(21.0), DayPeriod::Night);
    }

    /// Integration test: load the real catalog from the assets directory
    #[test]
    fn test_load_real_catalog() {
        let catalog_path = Path::new("assets/hdri/hdri_catalog.toml");
        if !catalog_path.exists() {
            // Skip if not running from workspace root
            return;
        }

        let hdri_root = Path::new("assets/hdri");
        let catalog =
            HdriCatalog::load(catalog_path, hdri_root).expect("Failed to load HDRI catalog");

        // Should have entries
        assert!(catalog.entries().count() > 0, "Catalog should have entries");

        // Should resolve forest+day
        let entry = catalog.resolve("forest", DayPeriod::Day);
        assert!(entry.is_some(), "Should resolve forest+day");

        // Should have a fallback for every biome+time
        for biome in &[
            "grassland",
            "desert",
            "forest",
            "mountain",
            "tundra",
            "swamp",
            "beach",
            "river",
        ] {
            for time in DayPeriod::all() {
                let entry = catalog.resolve(biome, *time);
                assert!(
                    entry.is_some(),
                    "Missing HDRI for {}+{}",
                    biome,
                    time.as_str()
                );
            }
        }
    }

    #[test]
    fn test_resolve_hdri_path() {
        let catalog_path = Path::new("assets/hdri/hdri_catalog.toml");
        if !catalog_path.exists() {
            return;
        }

        let hdri_root = Path::new("assets/hdri");
        let catalog = HdriCatalog::load(catalog_path, hdri_root).unwrap();

        let path = catalog.resolve_hdri_path("forest", DayPeriod::Day);
        assert!(path.is_some(), "Should resolve a path for forest+day");

        let path = path.unwrap();
        assert!(
            path.to_str().unwrap().contains("hdri"),
            "Path should contain hdri directory"
        );
    }

    #[test]
    fn catalog_files_match_sha256() {
        use sha2::{Digest, Sha256};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/hdri");
        let catalog =
            HdriCatalog::load(&root.join("hdri_catalog.toml"), &root).expect("hdri catalog");
        let equirect = catalog
            .get("sky_equirect")
            .expect("main catalog includes sky_equirect");
        assert_eq!(equirect.file, "sky_equirect.png");
        let spruit = catalog.get("spruit_sunrise").expect("spruit entry");
        assert!(
            spruit
                .file
                .ends_with("pine_forest/hdri/spruit_sunrise_2k.exr"),
            "spruit must use the terrain pack EXR, not a second copy"
        );
        for entry in catalog.entries() {
            if entry.sha256.is_empty() {
                continue;
            }
            let path = root.join(&entry.file);
            // The EXR bytes are vendored by the terrain pack (PR #215), not this branch.
            if entry.name == "spruit_sunrise" && !path.is_file() {
                continue;
            }
            let bytes =
                std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
            let digest = Sha256::digest(&bytes);
            let hex = digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert_eq!(hex, entry.sha256, "{}", entry.name);
        }
    }
}
