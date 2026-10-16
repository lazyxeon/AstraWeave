//! Biome Material System
//!
//! Bridges the terrain `BiomeType` system with the renderer's `MaterialManager`
//! and `IblManager`. Provides a unified API for loading biome-specific materials
//! and environment lighting when the player enters a new biome.
//!
//! # Architecture
//!
//! ```text
//! BiomeType ──┬──► MaterialManager::load_biome()  (terrain textures)
//!             └──► IblManager::set_mode()          (environment HDRI)
//!                     ▲
//!                     │
//!              HdriCatalog::resolve()  (biome × time-of-day lookup)
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! # use astraweave_render::biome_material::{BiomeMaterialSystem, BiomeMaterialConfig};
//! let config = BiomeMaterialConfig::default();
//! let mut system = BiomeMaterialSystem::new(config);
//!
//! // When the player enters a new biome:
//! // system.transition_biome(BiomeType::Forest, device, queue).await?;
//! ```

use crate::hdri_catalog::{DayPeriod, HdriCatalog};
use crate::ibl::SkyMode;
use anyhow::{Context, Result};
use astraweave_terrain::biome::BiomeType;
use std::path::PathBuf;

/// Configuration for the biome material system
#[derive(Debug, Clone)]
pub struct BiomeMaterialConfig {
    /// Root path for assets (default: "assets")
    pub assets_root: PathBuf,
    /// Whether to preload adjacent biome materials for seamless transitions
    pub preload_adjacent: bool,
    /// Current time of day for HDRI selection
    pub time_of_day: DayPeriod,
}

impl Default for BiomeMaterialConfig {
    fn default() -> Self {
        Self {
            assets_root: PathBuf::from("assets"),
            preload_adjacent: false,
            time_of_day: DayPeriod::Day,
        }
    }
}

/// Tracks the current biome state and manages material/HDRI transitions
pub struct BiomeMaterialSystem {
    /// Configuration
    config: BiomeMaterialConfig,
    /// Currently loaded biome (None if no biome loaded yet)
    current_biome: Option<BiomeType>,
    /// HDRI catalog (lazily loaded)
    hdri_catalog: Option<HdriCatalog>,
    /// Path to the currently loaded HDRI (for avoiding redundant reloads)
    current_hdri_path: Option<PathBuf>,
}

impl BiomeMaterialSystem {
    /// Create a new biome material system with the given configuration.
    pub fn new(config: BiomeMaterialConfig) -> Self {
        Self {
            config,
            current_biome: None,
            hdri_catalog: None,
            current_hdri_path: None,
        }
    }

    /// Get the currently loaded biome, if any.
    pub fn current_biome(&self) -> Option<BiomeType> {
        self.current_biome
    }

    /// Get the current time of day.
    pub fn time_of_day(&self) -> DayPeriod {
        self.config.time_of_day
    }

    /// Update the time of day. Returns `true` if the HDRI should be reloaded.
    pub fn set_time_of_day(&mut self, time: DayPeriod) -> bool {
        if self.config.time_of_day != time {
            self.config.time_of_day = time;
            true // caller should call `reload_hdri()`
        } else {
            false
        }
    }

    /// Lazily load and cache the HDRI catalog.
    fn ensure_hdri_catalog(&mut self) -> Result<&HdriCatalog> {
        if self.hdri_catalog.is_none() {
            let catalog_path = self.config.assets_root.join("hdri/hdri_catalog.toml");
            let hdri_root = self.config.assets_root.join("hdri");
            let catalog = HdriCatalog::load(&catalog_path, &hdri_root)
                .context("Failed to load HDRI catalog")?;
            self.hdri_catalog = Some(catalog);
        }
        self.hdri_catalog
            .as_ref()
            // INVARIANT: hdri_catalog set to Some in the block above
            .ok_or_else(|| anyhow::anyhow!("hdri_catalog failed to initialize"))
    }

    /// Get the material directory for a biome, resolved from the assets root.
    pub fn material_dir_for(&self, biome: BiomeType) -> PathBuf {
        self.config
            .assets_root
            .join(format!("materials/{}", biome.as_str()))
    }

    /// Get the terrain fallback material directory.
    pub fn terrain_fallback_dir(&self) -> PathBuf {
        self.config.assets_root.join("materials/terrain")
    }

    /// Resolve the best HDRI for the given biome and return a `SkyMode`.
    ///
    /// Returns `SkyMode::HdrPath` with the resolved file path, or falls back
    /// to `SkyMode::Procedural` if no HDRI is found.
    pub fn resolve_sky_mode(&mut self, biome: BiomeType) -> Result<SkyMode> {
        let time = self.config.time_of_day;
        let catalog = self.ensure_hdri_catalog()?;

        if let Some(entry) = catalog.resolve(biome.as_str(), time) {
            let path = catalog.resolve_path(entry);
            if path.is_file() {
                Ok(SkyMode::HdrPath {
                    biome: biome.as_str().to_string(),
                    path: path.to_string_lossy().to_string(),
                })
            } else {
                log::error!(
                    "HDRI catalog entry {} points at missing {}; using procedural sky",
                    entry.name,
                    path.display()
                );
                Ok(SkyMode::Procedural {
                    last_capture_time: 0.0,
                    recapture_interval: 60.0,
                })
            }
        } else {
            // No HDRI available — use procedural sky
            Ok(SkyMode::Procedural {
                last_capture_time: 0.0,
                recapture_interval: 60.0,
            })
        }
    }

    /// Resolve the HDRI file path for a biome + time combination.
    /// Returns `None` if no HDRI is cataloged for that combination.
    pub fn resolve_hdri_path(&mut self, biome: BiomeType) -> Result<Option<PathBuf>> {
        let time = self.config.time_of_day;
        let catalog = self.ensure_hdri_catalog()?;
        Ok(catalog.resolve_hdri_path(biome.as_str(), time))
    }

    /// Check if a biome transition is needed (different from currently loaded).
    pub fn needs_transition(&self, biome: BiomeType) -> bool {
        self.current_biome != Some(biome)
    }

    /// Mark a biome as loaded (call after `MaterialManager::load_biome()` succeeds).
    pub fn mark_loaded(&mut self, biome: BiomeType, hdri_path: Option<PathBuf>) {
        self.current_biome = Some(biome);
        self.current_hdri_path = hdri_path;
    }

    /// Check if the HDRI needs changing (time-of-day changed or biome changed).
    pub fn needs_hdri_update(&mut self, biome: BiomeType) -> Result<bool> {
        let new_path = self.resolve_hdri_path(biome)?;
        Ok(new_path != self.current_hdri_path)
    }

    /// Get the HDRI catalog (loads lazily if needed).
    pub fn hdri_catalog(&mut self) -> Result<&HdriCatalog> {
        self.ensure_hdri_catalog()
    }

    /// Validate that all required biome material directories exist.
    ///
    /// Returns a list of biomes whose material directories are missing.
    pub fn validate_material_dirs(&self) -> Vec<BiomeType> {
        BiomeType::all()
            .iter()
            .filter(|bt| {
                let dir = self.material_dir_for(**bt);
                !dir.join("materials.toml").exists()
            })
            .copied()
            .collect()
    }

    /// Validate the HDRI catalog covers all biome × time-of-day combinations.
    ///
    /// Returns `(biome, time)` pairs that have no HDRI assigned.
    pub fn validate_hdri_coverage(&mut self) -> Result<Vec<(String, DayPeriod)>> {
        let catalog = self.ensure_hdri_catalog()?;
        let mut gaps = Vec::new();

        for bt in BiomeType::all() {
            for time in DayPeriod::all() {
                if catalog.resolve(bt.as_str(), *time).is_none() {
                    gaps.push((bt.as_str().to_string(), *time));
                }
            }
        }

        Ok(gaps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_default_config() {
        let config = BiomeMaterialConfig::default();
        assert_eq!(config.assets_root, PathBuf::from("assets"));
        assert_eq!(config.time_of_day, DayPeriod::Day);
        assert!(!config.preload_adjacent);
    }

    #[test]
    fn test_material_dir_for() {
        let system = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
        assert_eq!(
            system.material_dir_for(BiomeType::Forest),
            PathBuf::from("assets/materials/forest")
        );
        assert_eq!(
            system.material_dir_for(BiomeType::Mountain),
            PathBuf::from("assets/materials/mountain")
        );
        assert_eq!(
            system.material_dir_for(BiomeType::Tundra),
            PathBuf::from("assets/materials/tundra")
        );
    }

    #[test]
    fn test_terrain_fallback_dir() {
        let system = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
        assert_eq!(
            system.terrain_fallback_dir(),
            PathBuf::from("assets/materials/terrain")
        );
    }

    #[test]
    fn test_needs_transition() {
        let mut system = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
        assert!(system.needs_transition(BiomeType::Forest));

        system.mark_loaded(BiomeType::Forest, None);
        assert!(!system.needs_transition(BiomeType::Forest));
        assert!(system.needs_transition(BiomeType::Desert));
    }

    #[test]
    fn test_set_time_of_day() {
        let mut system = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
        assert!(!system.set_time_of_day(DayPeriod::Day)); // same as default
        assert!(system.set_time_of_day(DayPeriod::Night)); // changed
        assert_eq!(system.time_of_day(), DayPeriod::Night);
    }

    #[test]
    fn test_validate_material_dirs() {
        let config = BiomeMaterialConfig {
            assets_root: PathBuf::from("assets"),
            ..Default::default()
        };
        let system = BiomeMaterialSystem::new(config);

        // This test checks that validate_material_dirs runs without panic.
        // In the actual workspace, all dirs should exist.
        let missing = system.validate_material_dirs();
        // If running from workspace root, all biomes should have material dirs
        if Path::new("assets/materials/forest/materials.toml").exists() {
            assert!(
                missing.is_empty(),
                "Expected all biome material dirs to exist, missing: {:?}",
                missing
            );
        }
    }

    /// Integration test: load the real HDRI catalog and validate coverage
    #[test]
    fn test_hdri_coverage_integration() {
        let catalog_path = Path::new("assets/hdri/hdri_catalog.toml");
        if !catalog_path.exists() {
            return; // skip if not in workspace root
        }

        let config = BiomeMaterialConfig::default();
        let mut system = BiomeMaterialSystem::new(config);

        let gaps = system
            .validate_hdri_coverage()
            .expect("Failed to validate HDRI coverage");
        assert!(gaps.is_empty(), "HDRI coverage gaps found: {:?}", gaps);
    }

    /// Integration test: resolve sky mode for all biomes
    #[test]
    fn test_resolve_sky_mode_all_biomes() {
        let catalog_path = Path::new("assets/hdri/hdri_catalog.toml");
        if !catalog_path.exists() {
            return;
        }

        let config = BiomeMaterialConfig::default();
        let mut system = BiomeMaterialSystem::new(config);

        for bt in BiomeType::all() {
            let mode = system
                .resolve_sky_mode(*bt)
                .unwrap_or_else(|_| panic!("Failed to resolve sky mode for {:?}", bt));
            match mode {
                SkyMode::HdrPath { biome, path } => {
                    assert_eq!(biome, bt.as_str());
                    assert!(!path.is_empty(), "Empty HDRI path for {:?}", bt);
                }
                SkyMode::Procedural { .. } => {
                    panic!(
                        "Expected HdrPath for {:?} but got Procedural (missing HDRI?)",
                        bt
                    );
                }
            }
        }
    }
}
