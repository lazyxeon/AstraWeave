//! BiomePack: a data-driven format for importing decomposed .blend scene
//! assets as biome vegetation/scatter profiles.
//!
//! A BiomePack is created from the `manifest.json` produced by the scene
//! decomposition pipeline (astraweave-blend) and describes all assets,
//! their categories, textures, and scatter parameters needed to generate
//! a custom biome.

use crate::biome::{
    BiomeConditions, BiomeConfig, BiomeSky, BiomeType, BiomeVegetation, VegetationType,
};
use crate::scatter::ScatterConfig;
use crate::zone_scatter::FixedPlacement;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ============================================================================
// BiomePack data types
// ============================================================================

/// A complete biome asset pack generated from a decomposed .blend scene.
///
/// This is the bridge between the blend decomposition pipeline and the
/// terrain biome/scatter systems. It can be:
/// 1. Auto-generated from a `manifest.json` via [`BiomePack::from_manifest`]
/// 2. Hand-edited in JSON/RON and loaded from disk
/// 3. Constructed programmatically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomePack {
    /// Human-readable name for this biome pack.
    pub name: String,
    /// Description of the biome this pack represents.
    pub description: String,
    /// Root directory where all asset files live (meshes/, textures/, hdri/).
    pub root_dir: PathBuf,
    /// SHA-256 hash of the source .blend file (for cache invalidation).
    pub blend_hash: String,
    /// All mesh assets in this pack.
    pub assets: Vec<BiomePackAsset>,
    /// HDRI environment maps.
    pub hdris: Vec<BiomePackHdri>,
    /// Ground/terrain textures extracted from the scene.
    pub ground_textures: Vec<BiomePackGroundTexture>,
    /// Scatter configuration overrides.
    pub scatter: BiomePackScatter,
    /// Environmental conditions for biome classification.
    pub conditions: BiomeConditions,
    /// Sky parameters.
    pub sky: BiomeSky,
    /// Path to the rasterized terrain heightmap JSON file (relative to root_dir).
    /// Produced by the heightmap rasterization pipeline from .blend terrain meshes.
    #[serde(default)]
    pub terrain_heightmap_path: Option<PathBuf>,
    /// Path to the fixed placements JSON file (relative to root_dir).
    /// Contains exact object transforms for Replica mode.
    #[serde(default)]
    pub fixed_placements_path: Option<PathBuf>,
    /// Estimated XZ footprint area of the original .blend scene in world units².
    /// Used by adaptive scaling to compute density/scale ratios.
    #[serde(default)]
    pub scene_footprint_area: Option<f32>,
}

/// A single mesh asset within a biome pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomePackAsset {
    /// Blender object name.
    pub name: String,
    /// Relative path to the GLB file (from `root_dir`).
    pub mesh_path: String,
    /// Asset category: `vegetation`, `rock`, `terrain`, `billboard`, `structure`,
    /// `furniture`, `light`, or `prop` (fallback).
    pub category: String,
    /// Scatter weight — higher = more frequent placement.
    pub weight: f32,
    /// Scale range (min, max) for random scatter placement.
    pub scale_range: (f32, f32),
    /// Maximum terrain slope (degrees) where this asset can be placed.
    pub slope_tolerance: f32,
    /// Object dimensions [width, depth, height] for scatter spacing.
    pub dimensions: Option<[f64; 3]>,
    /// Vertex count (useful for LOD decisions).
    pub vertex_count: u64,
    /// Textures associated with this asset.
    pub textures: Vec<BiomePackTexture>,
}

/// A texture reference within a biome pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomePackTexture {
    /// Filename relative to root_dir/textures/.
    pub filename: String,
    /// PBR channel: `diffuse`, `normal`, `roughness`, `metallic`, `alpha`, `displacement`.
    pub channel: String,
    /// Original resolution.
    pub width: u32,
    /// Original resolution.
    pub height: u32,
}

/// An HDRI environment map in the pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomePackHdri {
    /// Filename relative to root_dir/hdri/.
    pub filename: String,
    /// Original image name.
    pub original_name: String,
    /// Resolution.
    pub width: u32,
    /// Resolution.
    pub height: u32,
}

/// A ground/terrain texture set for splatmap rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomePackGroundTexture {
    /// Name identifier.
    pub name: String,
    /// Diffuse/albedo texture path.
    pub diffuse: Option<String>,
    /// Normal map path.
    pub normal: Option<String>,
    /// Roughness map path.
    pub roughness: Option<String>,
    /// Displacement/height map path.
    pub displacement: Option<String>,
}

/// Scatter configuration for the biome pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomePackScatter {
    /// Base vegetation density (objects per unit area).
    pub density: f32,
    /// Use Poisson disk sampling.
    pub use_poisson_disk: bool,
    /// Minimum distance between scatter instances.
    pub min_distance: f32,
    /// Maximum terrain slope for scatter placement.
    pub max_slope: f32,
    /// Size variation range (min, max scale multiplier).
    pub size_variation: (f32, f32),
    /// Enable random Y-axis rotation.
    pub random_rotation: bool,
}

impl Default for BiomePackScatter {
    fn default() -> Self {
        Self {
            density: 0.003,
            use_poisson_disk: true,
            min_distance: 2.0,
            max_slope: 35.0,
            size_variation: (0.8, 1.5),
            random_rotation: true,
        }
    }
}

// ============================================================================
// Manifest parsing
// ============================================================================

/// Raw manifest.json as produced by the scene decomposition Python script.
#[derive(Debug, Deserialize)]
struct RawManifest {
    #[serde(default)]
    blend_hash: String,
    #[serde(default)]
    source_file: String,
    #[serde(default)]
    assets: Vec<RawManifestAsset>,
    #[serde(default)]
    hdris: Vec<RawManifestHdri>,
    #[serde(default)]
    total_objects: usize,
}

#[derive(Debug, Deserialize)]
struct RawManifestAsset {
    name: String,
    filename: String,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    vertex_count: u64,
    #[serde(default)]
    dimensions: Option<[f64; 3]>,
    #[serde(default)]
    position: Option<[f64; 3]>,
    #[serde(default)]
    rotation: Option<[f64; 3]>,
    #[serde(default)]
    scale: Option<[f64; 3]>,
    #[serde(default)]
    textures: Vec<RawManifestTexture>,
    #[serde(default)]
    #[allow(dead_code)]
    materials: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    collections: Vec<String>,
}

fn default_category() -> String {
    "prop".to_string()
}

#[derive(Debug, Deserialize)]
struct RawManifestTexture {
    filename: String,
    #[serde(default)]
    channel: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
}

#[derive(Debug, Deserialize)]
struct RawManifestHdri {
    filename: String,
    #[serde(default)]
    original_name: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
}

// ============================================================================
// Implementation
// ============================================================================

impl BiomePack {
    /// Load a BiomePack from a manifest.json file produced by scene decomposition.
    ///
    /// The `root_dir` is the directory containing the manifest and the
    /// meshes/, textures/, hdri/ subdirectories.
    pub fn from_manifest(manifest_path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(manifest_path)?;
        let raw: RawManifest = serde_json::from_str(&content)?;

        let root_dir = manifest_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();

        // Derive pack name from source file or directory name
        let name = root_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Convert raw assets to BiomePackAssets with scatter defaults,
        // and collect fixed placements from transform data.
        let mut fixed_placements: Vec<FixedPlacement> = Vec::new();
        let mut xs: Vec<f64> = Vec::new();
        let mut zs: Vec<f64> = Vec::new();

        let assets: Vec<BiomePackAsset> = raw
            .assets
            .into_iter()
            .map(|a| {
                // Record fixed placement if position data exists
                if let Some(pos) = &a.position {
                    let has_nonzero_transform = pos.iter().any(|v| *v != 0.0)
                        || a.rotation
                            .as_ref()
                            .is_some_and(|r| r.iter().any(|v| *v != 0.0))
                        || a.scale.as_ref().is_some_and(|s| *s != [1.0, 1.0, 1.0]);

                    // Include all assets with transform data (even at origin —
                    // origin placement is intentional in authored scenes)
                    if has_nonzero_transform || a.category != "terrain" {
                        fixed_placements.push(FixedPlacement {
                            position: *pos,
                            rotation: a.rotation.unwrap_or([0.0; 3]),
                            scale: a.scale.unwrap_or([1.0; 3]),
                            mesh_path: a.filename.clone(),
                            category: a.category.clone(),
                            name: a.name.clone(),
                        });
                        xs.push(pos[0]);
                        zs.push(pos[2]);
                    }
                }

                let (weight, scale_range, slope_tolerance) =
                    default_scatter_params_for_category(&a.category, a.dimensions.as_ref());

                BiomePackAsset {
                    name: a.name,
                    mesh_path: a.filename,
                    category: a.category,
                    weight,
                    scale_range,
                    slope_tolerance,
                    dimensions: a.dimensions,
                    vertex_count: a.vertex_count,
                    textures: a
                        .textures
                        .into_iter()
                        .map(|t| BiomePackTexture {
                            filename: t.filename,
                            channel: t.channel,
                            width: t.width,
                            height: t.height,
                        })
                        .collect(),
                }
            })
            .collect();

        // Write fixed_placements.json if we found transform data
        let fixed_placements_path = if !fixed_placements.is_empty() {
            let fp_path = root_dir.join("fixed_placements.json");
            let fp_json = serde_json::to_string_pretty(&fixed_placements)
                .unwrap_or_else(|_| "[]".to_string());
            std::fs::write(&fp_path, fp_json)?;
            Some(PathBuf::from("fixed_placements.json"))
        } else {
            detect_placements_file(&root_dir)
        };

        // Compute scene footprint area from XZ bounding box of positions
        let scene_footprint_area = if xs.len() >= 2 {
            let x_min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
            let x_max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let z_min = zs.iter().cloned().fold(f64::INFINITY, f64::min);
            let z_max = zs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let width = (x_max - x_min).max(1.0);
            let depth = (z_max - z_min).max(1.0);
            Some((width * depth) as f32)
        } else {
            None
        };

        let hdris = raw
            .hdris
            .into_iter()
            .map(|h| BiomePackHdri {
                filename: h.filename,
                original_name: h.original_name,
                width: h.width,
                height: h.height,
            })
            .collect();

        // Detect ground textures: assets with category "terrain" or materials
        // whose names contain terrain-related keywords
        let ground_textures = detect_ground_textures(&root_dir);

        Ok(Self {
            name,
            description: format!(
                "Auto-generated biome pack from {} ({} assets)",
                raw.source_file, raw.total_objects
            ),
            root_dir: root_dir.clone(),
            blend_hash: raw.blend_hash,
            assets,
            hdris,
            ground_textures,
            scatter: BiomePackScatter::default(),
            conditions: BiomeConditions::default(),
            sky: BiomeSky::default(),
            terrain_heightmap_path: detect_heightmap_file(&root_dir),
            fixed_placements_path,
            scene_footprint_area,
        })
    }

    /// Load a BiomePack from a serialized JSON file (not a raw manifest).
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut pack: Self = serde_json::from_str(&content)?;

        // Always derive root_dir from the JSON file's actual location on disk.
        // The serialized root_dir may contain stale absolute paths from when the
        // .blend file was originally decomposed (e.g., on a different drive or
        // machine). The JSON file's parent directory is the authoritative root.
        //
        // Use dunce::canonicalize or manual resolution to avoid Windows \\?\ prefix
        // that breaks downstream path consumers (glTF loaders, etc.).
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        let actual_root = {
            let canonical = parent
                .canonicalize()
                .unwrap_or_else(|_| parent.to_path_buf());
            // Strip Windows extended-length path prefix (\\?\) if present.
            // Rust's canonicalize() adds this on Windows, but it breaks
            // path string comparisons and some C-based file loaders.
            let s = canonical.to_string_lossy();
            if let Some(stripped) = s.strip_prefix(r"\\?\") {
                std::path::PathBuf::from(stripped)
            } else {
                canonical
            }
        };
        if pack.root_dir != actual_root {
            eprintln!(
                "BiomePack '{}': overriding root_dir '{}' -> '{}'",
                pack.name,
                pack.root_dir.display(),
                actual_root.display(),
            );
            pack.root_dir = actual_root;
        }

        Ok(pack)
    }

    /// Build a map of mesh filename stem → absolute diffuse texture path.
    ///
    /// For each asset with a `diffuse` channel texture, the map entry key is
    /// the GLB filename without extension (e.g. `"boulder_01"` for
    /// `"meshes/boulder_01.glb"`). The value is the absolute path to the
    /// diffuse texture file (`root_dir/textures/<filename>`).
    ///
    /// This allows the scatter upload pipeline to match loaded meshes to
    /// their textures, since the decomposer exports meshes without embedded
    /// materials.
    pub fn build_diffuse_texture_map(&self) -> std::collections::HashMap<String, PathBuf> {
        let mut map = std::collections::HashMap::new();
        for asset in &self.assets {
            // Find the diffuse texture for this asset, skipping normal/roughness
            // maps that may be incorrectly labeled as "diffuse" in the biomepack.
            let tex = asset
                .textures
                .iter()
                .find(|t| t.channel == "diffuse" && !Self::filename_is_non_diffuse(&t.filename));
            // Fall back to any texture labeled "diffuse" if the smart filter
            // rejected all candidates (better than no texture at all).
            let tex = tex.or_else(|| asset.textures.iter().find(|t| t.channel == "diffuse"));
            if let Some(tex) = tex {
                let tex_path = self.root_dir.join("textures").join(&tex.filename);
                // Key by mesh filename stem and also by asset name for flexible matching
                if let Some(stem) = Path::new(&asset.mesh_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                {
                    map.insert(stem.to_string(), tex_path.clone());
                }
                map.insert(asset.name.clone(), tex_path);
            }
        }
        map
    }

    /// Heuristic: detect filenames that look like non-diffuse texture maps
    /// (normal, roughness, metallic, AO, displacement) so that mislabeled
    /// channels in the biomepack JSON don't cause the wrong texture to load.
    fn filename_is_non_diffuse(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        lower.contains("_nor_")
            || lower.contains("_normal")
            || lower.contains("_nrm")
            || lower.contains("_rough")
            || lower.contains("_metal")
            || lower.contains("_ao")
            || lower.contains("_disp")
            || lower.contains("_height")
            || lower.contains("_arm")
            || lower.contains("_orm")
    }

    /// Save the BiomePack as a JSON file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert this BiomePack into a [`BiomeConfig`] usable by the terrain system.
    ///
    /// The returned config uses [`BiomeType::Desert`] as a base (since Namaqualand
    /// is a South African semi-arid biome), but the vegetation types are fully
    /// populated from the pack's assets.
    pub fn to_biome_config(&self, biome_type: BiomeType) -> BiomeConfig {
        // Filter out non-scatterable assets: fog volumes, particle systems,
        // shadow catchers, and other scene-composition objects that should not
        // appear as vegetation instances on the terrain.
        let is_scatterable = |a: &BiomePackAsset| -> bool {
            let n = a.name.to_lowercase();
            // Skip fog, particles, shadow casters, sand planes, etc.
            !(n.contains("fog")
                || n.contains("particle")
                || n.contains("shadow")
                || n.contains("volume")
                || n == "sand"
                || n == "monster"
                || n.contains("background_water")
                || n.contains("bacground_ground"))
        };

        let to_veg = |a: &BiomePackAsset| {
            let full_path = self.root_dir.join(&a.mesh_path);
            // Use dimension-weighted importance: larger assets (trees) get
            // higher effective weight so they aren't drowned out by dozens
            // of tiny branches that all share the same raw weight.
            let max_dim = a
                .dimensions
                .map(|d| d[0].max(d[1]).max(d[2]) as f32)
                .unwrap_or(1.0);
            let dim_boost = (max_dim / 2.0).clamp(0.5, 5.0); // 1m → 0.5x, 4m → 2x, 10m+ → 5x
            VegetationType {
                name: a.name.clone(),
                weight: a.weight * dim_boost,
                model_path: full_path.to_string_lossy().to_string(),
                scale_range: a.scale_range,
                slope_tolerance: a.slope_tolerance,
            }
        };
        let sort_desc = |v: &mut Vec<VegetationType>| {
            v.sort_by(|a, b| {
                b.weight
                    .partial_cmp(&a.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        };

        let mut rocks: Vec<VegetationType> = self
            .assets
            .iter()
            .filter(|a| a.category == "rock" && is_scatterable(a))
            .map(&to_veg)
            .collect();
        let mut veg: Vec<VegetationType> = self
            .assets
            .iter()
            .filter(|a| a.category == "vegetation" && is_scatterable(a))
            .map(&to_veg)
            .collect();
        let mut billboards: Vec<VegetationType> = self
            .assets
            .iter()
            .filter(|a| a.category == "billboard" && is_scatterable(a))
            .map(&to_veg)
            .collect();
        // Props are rarely good scatter candidates — only take root/ivy types
        let mut props: Vec<VegetationType> = self
            .assets
            .iter()
            .filter(|a| {
                a.category == "prop" && is_scatterable(a) && {
                    let n = a.name.to_lowercase();
                    n.contains("root")
                        || n.contains("ivy")
                        || n.contains("stick")
                        || n.contains("debris")
                }
            })
            .map(&to_veg)
            .collect();

        sort_desc(&mut rocks);
        sort_desc(&mut veg);
        sort_desc(&mut billboards);
        sort_desc(&mut props);

        // Stratified selection: reserve slots per category with tree priority.
        // Trees (large vegetation) get dedicated slots so they're never
        // crowded out by dozens of small branches/grass.
        const MAX_VEG_TYPES: usize = 25;
        let rock_slots = rocks.len().min(6);
        let billboard_slots = billboards.len().min(3);
        let prop_slots = props.len().min(3);
        let veg_slots = MAX_VEG_TYPES.saturating_sub(rock_slots + billboard_slots + prop_slots);

        let mut vegetation_types: Vec<VegetationType> = Vec::with_capacity(MAX_VEG_TYPES);
        vegetation_types.extend(rocks.into_iter().take(rock_slots));
        vegetation_types.extend(billboards.into_iter().take(billboard_slots));
        vegetation_types.extend(props.into_iter().take(prop_slots));
        vegetation_types.extend(veg.into_iter().take(veg_slots));

        // Re-normalize weights so they sum correctly
        let total: f32 = vegetation_types.iter().map(|v| v.weight).sum();
        if total > 0.0 {
            for v in &mut vegetation_types {
                v.weight /= total;
            }
        }

        let ground_texture_paths: Vec<String> = self
            .ground_textures
            .iter()
            .filter_map(|gt| {
                gt.diffuse
                    .as_ref()
                    .map(|d| self.root_dir.join(d).to_string_lossy().to_string())
            })
            .collect();

        BiomeConfig {
            biome_type,
            name: self.name.clone(),
            description: self.description.clone(),
            conditions: self.conditions.clone(),
            sky: self.sky.clone(),
            vegetation: BiomeVegetation {
                density: self.scatter.density,
                vegetation_types,
                size_variation: self.scatter.size_variation,
                random_rotation: self.scatter.random_rotation,
            },
            resource_weights: Vec::new(),
            base_resource_amount: (0, 0),
            resource_respawn: (60.0, 300.0),
            ground_textures: ground_texture_paths,
            priority: 5,
        }
    }

    /// Convert this BiomePack's scatter settings into a [`ScatterConfig`].
    pub fn to_scatter_config(&self) -> ScatterConfig {
        ScatterConfig {
            use_poisson_disk: self.scatter.use_poisson_disk,
            min_distance: self.scatter.min_distance,
            max_slope: self.scatter.max_slope,
            max_curvature: 0.15,
            height_filter: None,
            seed_offset: 0,
        }
    }

    /// Get all assets of a specific category.
    pub fn assets_by_category(&self, category: &str) -> Vec<&BiomePackAsset> {
        self.assets
            .iter()
            .filter(|a| a.category == category)
            .collect()
    }

    /// Get total asset count.
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }

    /// Estimate the XZ footprint area of the original scene.
    ///
    /// If `scene_footprint_area` is already set, returns that. Otherwise,
    /// estimates from asset dimensions and positions by computing the convex
    /// hull of all asset footprints.
    pub fn estimate_scene_footprint(&self) -> f32 {
        if let Some(area) = self.scene_footprint_area {
            return area;
        }

        // Estimate from terrain-category assets (the ground meshes)
        let terrain_assets: Vec<&BiomePackAsset> = self.assets_by_category("terrain");
        if !terrain_assets.is_empty() {
            let total: f32 = terrain_assets
                .iter()
                .filter_map(|a| a.dimensions.as_ref())
                .map(|d| d[0] as f32 * d[1] as f32)
                .sum();
            if total > 0.0 {
                return total;
            }
        }

        // Fallback: sum of all asset footprints × density factor
        let total: f32 = self
            .assets
            .iter()
            .filter_map(|a| a.dimensions.as_ref())
            .map(|d| d[0] as f32 * d[1] as f32)
            .sum();
        // Rough estimate: assets cover ~10-30% of scene area
        (total * 5.0).max(100.0)
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Compute default scatter weight, scale range, and slope tolerance
/// based on asset category and dimensions.
fn default_scatter_params_for_category(
    category: &str,
    dimensions: Option<&[f64; 3]>,
) -> (f32, (f32, f32), f32) {
    // Estimate object "size class" from dimensions
    let size_class = dimensions
        .map(|d| {
            let max_dim = d[0].max(d[1]).max(d[2]);
            if max_dim > 5.0 {
                SizeClass::Large
            } else if max_dim > 1.0 {
                SizeClass::Medium
            } else {
                SizeClass::Small
            }
        })
        .unwrap_or(SizeClass::Medium);

    match category {
        "vegetation" => match size_class {
            SizeClass::Large => (0.05, (0.9, 1.15), 25.0), // Trees
            SizeClass::Medium => (0.3, (0.7, 1.3), 35.0),  // Bushes
            SizeClass::Small => (2.0, (0.6, 1.4), 45.0),   // Flowers/grass
        },
        "rock" => match size_class {
            SizeClass::Large => (0.02, (0.8, 1.2), 20.0),  // Cliffs
            SizeClass::Medium => (0.15, (0.7, 1.5), 30.0), // Boulders
            SizeClass::Small => (0.5, (0.8, 2.0), 40.0),   // Stones
        },
        "terrain" => (0.0, (1.0, 1.0), 90.0), // Terrain meshes aren't scattered
        "billboard" => (1.0, (0.8, 1.2), 45.0),
        _ => (0.1, (0.8, 1.2), 35.0), // Generic prop
    }
}

#[derive(Debug, Clone, Copy)]
enum SizeClass {
    Small,
    Medium,
    Large,
}

/// Detect ground/terrain textures from the root directory.
/// Looks for textures whose names contain terrain-related keywords.
fn detect_ground_textures(root_dir: &Path) -> Vec<BiomePackGroundTexture> {
    let textures_dir = root_dir.join("textures");
    if !textures_dir.exists() {
        return Vec::new();
    }

    let terrain_keywords = [
        "cliff", "sand", "gravel", "dirt", "ground", "soil", "terrain",
    ];

    let mut texture_groups: std::collections::HashMap<String, BiomePackGroundTexture> =
        std::collections::HashMap::new();

    if let Ok(entries) = std::fs::read_dir(&textures_dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name().to_string_lossy().to_lowercase();

            // Check if this texture matches terrain keywords
            let is_terrain = terrain_keywords.iter().any(|kw| filename.contains(kw));
            if !is_terrain {
                continue;
            }

            // Determine PBR channel from filename
            let channel = if filename.contains("diff")
                || filename.contains("color")
                || filename.contains("albedo")
            {
                "diffuse"
            } else if filename.contains("nor") || filename.contains("normal") {
                "normal"
            } else if filename.contains("rough") {
                "roughness"
            } else if filename.contains("disp") || filename.contains("height") {
                "displacement"
            } else {
                continue; // Skip unknown channels
            };

            // Group key: strip the channel suffix to group related textures
            let group_key = terrain_keywords
                .iter()
                .find(|kw| filename.contains(*kw))
                .unwrap_or(&"unknown")
                .to_string();

            let rel_path = format!("textures/{}", entry.file_name().to_string_lossy());

            let group =
                texture_groups
                    .entry(group_key.clone())
                    .or_insert_with(|| BiomePackGroundTexture {
                        name: group_key,
                        diffuse: None,
                        normal: None,
                        roughness: None,
                        displacement: None,
                    });

            match channel {
                "diffuse" => group.diffuse = Some(rel_path),
                "normal" => group.normal = Some(rel_path),
                "roughness" => group.roughness = Some(rel_path),
                "displacement" => group.displacement = Some(rel_path),
                _ => {}
            }
        }
    }

    texture_groups.into_values().collect()
}

/// Check if a rasterized terrain heightmap JSON file exists in the root directory.
fn detect_heightmap_file(root_dir: &Path) -> Option<PathBuf> {
    let path = root_dir.join("terrain_heightmap.json");
    if path.exists() {
        Some(PathBuf::from("terrain_heightmap.json"))
    } else {
        None
    }
}

/// Check if a fixed placements JSON file exists in the root directory.
fn detect_placements_file(root_dir: &Path) -> Option<PathBuf> {
    let path = root_dir.join("fixed_placements.json");
    if path.exists() {
        Some(PathBuf::from("fixed_placements.json"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scatter_params() {
        let (w, _s, sl) = default_scatter_params_for_category("vegetation", Some(&[0.5, 0.5, 0.3]));
        assert!(w > 1.0, "Small vegetation should have high weight");
        assert!(sl > 40.0);

        let (w, _, _) = default_scatter_params_for_category("rock", Some(&[8.0, 6.0, 4.0]));
        assert!(w < 0.1, "Large rocks should be rare");

        let (w, _, _) = default_scatter_params_for_category("terrain", None);
        assert_eq!(w, 0.0, "Terrain meshes should not scatter");
    }

    #[test]
    fn test_biome_pack_serde_roundtrip() {
        let pack = BiomePack {
            name: "Namaqualand".to_string(),
            description: "South African semi-arid biome".to_string(),
            root_dir: PathBuf::from("assets/Namaqualand"),
            blend_hash: "abc123".to_string(),
            assets: vec![BiomePackAsset {
                name: "Boulder_01".to_string(),
                mesh_path: "meshes/Boulder_01.glb".to_string(),
                category: "rock".to_string(),
                weight: 0.15,
                scale_range: (0.7, 1.5),
                slope_tolerance: 30.0,
                dimensions: Some([2.0, 2.0, 1.5]),
                vertex_count: 1200,
                textures: vec![],
            }],
            hdris: vec![],
            ground_textures: vec![],
            scatter: BiomePackScatter::default(),
            conditions: BiomeConditions::default(),
            sky: BiomeSky::default(),
            terrain_heightmap_path: None,
            fixed_placements_path: None,
            scene_footprint_area: None,
        };

        let json = serde_json::to_string(&pack).unwrap();
        let parsed: BiomePack = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Namaqualand");
        assert_eq!(parsed.assets.len(), 1);
        assert_eq!(parsed.assets[0].category, "rock");
    }

    #[test]
    fn test_to_biome_config() {
        let pack = BiomePack {
            name: "TestPack".to_string(),
            description: "Test".to_string(),
            root_dir: PathBuf::from("/assets/test"),
            blend_hash: "hash".to_string(),
            assets: vec![
                BiomePackAsset {
                    name: "tree_01".to_string(),
                    mesh_path: "meshes/tree_01.glb".to_string(),
                    category: "vegetation".to_string(),
                    weight: 0.5,
                    scale_range: (0.9, 1.1),
                    slope_tolerance: 25.0,
                    dimensions: Some([3.0, 3.0, 8.0]),
                    vertex_count: 5000,
                    textures: vec![],
                },
                BiomePackAsset {
                    name: "rock_01".to_string(),
                    mesh_path: "meshes/rock_01.glb".to_string(),
                    category: "rock".to_string(),
                    weight: 0.2,
                    scale_range: (0.8, 1.3),
                    slope_tolerance: 30.0,
                    dimensions: Some([2.0, 2.0, 1.5]),
                    vertex_count: 800,
                    textures: vec![],
                },
                BiomePackAsset {
                    name: "terrain_mesh".to_string(),
                    mesh_path: "meshes/terrain.glb".to_string(),
                    category: "terrain".to_string(),
                    weight: 0.0,
                    scale_range: (1.0, 1.0),
                    slope_tolerance: 90.0,
                    dimensions: None,
                    vertex_count: 10000,
                    textures: vec![],
                },
            ],
            hdris: vec![],
            ground_textures: vec![],
            scatter: BiomePackScatter {
                density: 0.005,
                ..Default::default()
            },
            conditions: BiomeConditions::default(),
            sky: BiomeSky::default(),
            terrain_heightmap_path: None,
            fixed_placements_path: None,
            scene_footprint_area: None,
        };

        let config = pack.to_biome_config(BiomeType::Desert);

        assert_eq!(config.biome_type, BiomeType::Desert);
        assert_eq!(config.name, "TestPack");
        // terrain category is filtered out
        assert_eq!(config.vegetation.vegetation_types.len(), 2);
        assert_eq!(config.vegetation.density, 0.005);
    }

    #[test]
    fn test_to_scatter_config() {
        let pack = BiomePack {
            name: "Test".to_string(),
            description: String::new(),
            root_dir: PathBuf::from("."),
            blend_hash: String::new(),
            assets: vec![],
            hdris: vec![],
            ground_textures: vec![],
            scatter: BiomePackScatter {
                density: 0.01,
                use_poisson_disk: true,
                min_distance: 3.0,
                max_slope: 40.0,
                size_variation: (0.5, 2.0),
                random_rotation: true,
            },
            conditions: BiomeConditions::default(),
            sky: BiomeSky::default(),
            terrain_heightmap_path: None,
            fixed_placements_path: None,
            scene_footprint_area: None,
        };

        let config = pack.to_scatter_config();
        assert!(config.use_poisson_disk);
        assert_eq!(config.min_distance, 3.0);
        assert_eq!(config.max_slope, 40.0);
    }

    #[test]
    fn test_assets_by_category() {
        let pack = BiomePack {
            name: "Test".to_string(),
            description: String::new(),
            root_dir: PathBuf::from("."),
            blend_hash: String::new(),
            assets: vec![
                BiomePackAsset {
                    name: "tree".to_string(),
                    mesh_path: "meshes/tree.glb".to_string(),
                    category: "vegetation".to_string(),
                    weight: 0.5,
                    scale_range: (0.9, 1.1),
                    slope_tolerance: 25.0,
                    dimensions: None,
                    vertex_count: 500,
                    textures: vec![],
                },
                BiomePackAsset {
                    name: "rock".to_string(),
                    mesh_path: "meshes/rock.glb".to_string(),
                    category: "rock".to_string(),
                    weight: 0.2,
                    scale_range: (0.8, 1.3),
                    slope_tolerance: 30.0,
                    dimensions: None,
                    vertex_count: 200,
                    textures: vec![],
                },
            ],
            hdris: vec![],
            ground_textures: vec![],
            scatter: BiomePackScatter::default(),
            conditions: BiomeConditions::default(),
            sky: BiomeSky::default(),
            terrain_heightmap_path: None,
            fixed_placements_path: None,
            scene_footprint_area: None,
        };

        assert_eq!(pack.assets_by_category("vegetation").len(), 1);
        assert_eq!(pack.assets_by_category("rock").len(), 1);
        assert_eq!(pack.assets_by_category("terrain").len(), 0);
    }

    #[test]
    fn test_from_manifest_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let manifest = serde_json::json!({
            "blend_hash": "abc123def456",
            "source_file": "Namaqualand.blend",
            "total_objects": 5,
            "assets": [
                {
                    "name": "QuiverTree_01",
                    "filename": "meshes/QuiverTree_01.glb",
                    "category": "vegetation",
                    "vertex_count": 3200,
                    "dimensions": [2.5, 2.5, 7.0],
                    "textures": [
                        {"filename": "QuiverTree_01_diffuse.png", "channel": "diffuse", "width": 2048, "height": 2048}
                    ],
                    "materials": ["QuiverTreeBark"],
                    "collections": ["Trees"]
                },
                {
                    "name": "Boulder_Large",
                    "filename": "meshes/Boulder_Large.glb",
                    "category": "rock",
                    "vertex_count": 800,
                    "dimensions": [6.0, 5.0, 3.5],
                    "textures": [],
                    "materials": ["RockMaterial"],
                    "collections": ["Rocks"]
                },
                {
                    "name": "Gazania_Cluster",
                    "filename": "meshes/Gazania_Cluster.glb",
                    "category": "vegetation",
                    "vertex_count": 200,
                    "dimensions": [0.3, 0.3, 0.15],
                    "textures": [],
                    "materials": ["FlowerMat"],
                    "collections": ["Flowers"]
                },
                {
                    "name": "Terrain_Base",
                    "filename": "meshes/Terrain_Base.glb",
                    "category": "terrain",
                    "vertex_count": 50000,
                    "dimensions": [100.0, 100.0, 20.0],
                    "textures": [],
                    "materials": ["TerrainMat"],
                    "collections": ["Terrain"]
                },
                {
                    "name": "Stone_Small",
                    "filename": "meshes/Stone_Small.glb",
                    "category": "rock",
                    "vertex_count": 120,
                    "dimensions": [0.4, 0.3, 0.2],
                    "textures": [],
                    "materials": ["StoneMat"],
                    "collections": ["Rocks"]
                }
            ],
            "hdris": [
                {"filename": "namaqualand_sunset.hdr", "original_name": "Sunset HDRI", "width": 4096, "height": 2048}
            ]
        });

        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(&manifest_path, manifest.to_string()).unwrap();

        let pack = BiomePack::from_manifest(&manifest_path).unwrap();

        assert_eq!(pack.assets.len(), 5);
        assert_eq!(pack.blend_hash, "abc123def456");
        assert_eq!(pack.hdris.len(), 1);

        // Categories should be correct
        assert_eq!(pack.assets_by_category("vegetation").len(), 2);
        assert_eq!(pack.assets_by_category("rock").len(), 2);
        assert_eq!(pack.assets_by_category("terrain").len(), 1);

        // QuiverTree is large vegetation (7m tall) → low weight
        let tree = pack
            .assets
            .iter()
            .find(|a| a.name == "QuiverTree_01")
            .unwrap();
        assert!(
            tree.weight < 0.1,
            "Large tree should have low scatter weight: {}",
            tree.weight
        );
        assert!(tree.slope_tolerance <= 25.0);

        // Gazania is small vegetation (0.15m) → high weight
        let flower = pack
            .assets
            .iter()
            .find(|a| a.name == "Gazania_Cluster")
            .unwrap();
        assert!(
            flower.weight > 1.0,
            "Small flowers should have high scatter weight: {}",
            flower.weight
        );

        // Boulder is large rock → very low weight
        let boulder = pack
            .assets
            .iter()
            .find(|a| a.name == "Boulder_Large")
            .unwrap();
        assert!(
            boulder.weight < 0.05,
            "Large boulder should be rare: {}",
            boulder.weight
        );

        // Small stone → medium weight
        let stone = pack
            .assets
            .iter()
            .find(|a| a.name == "Stone_Small")
            .unwrap();
        assert!(
            stone.weight > 0.3,
            "Small stones should be common: {}",
            stone.weight
        );

        // Terrain mesh → zero weight (never scattered)
        let terrain = pack
            .assets
            .iter()
            .find(|a| a.name == "Terrain_Base")
            .unwrap();
        assert_eq!(terrain.weight, 0.0);

        // BiomeConfig conversion: terrain assets should be filtered out
        let config = pack.to_biome_config(BiomeType::Desert);
        assert_eq!(config.vegetation.vegetation_types.len(), 4); // 2 veg + 2 rock (not terrain)
        assert_eq!(config.biome_type, BiomeType::Desert);

        // Save and reload
        let save_path = dir.path().join("namaqualand_pack.json");
        pack.save(&save_path).unwrap();
        let reloaded = BiomePack::load(&save_path).unwrap();
        assert_eq!(reloaded.assets.len(), 5);
        assert_eq!(reloaded.blend_hash, "abc123def456");
    }

    #[test]
    fn test_biome_config_from_pack_constructor() {
        let pack = BiomePack {
            name: "TestPack".to_string(),
            description: "Test biome pack".to_string(),
            root_dir: PathBuf::from("assets/test"),
            blend_hash: "test_hash".to_string(),
            assets: vec![BiomePackAsset {
                name: "bush_01".to_string(),
                mesh_path: "meshes/bush_01.glb".to_string(),
                category: "vegetation".to_string(),
                weight: 1.0,
                scale_range: (0.8, 1.2),
                slope_tolerance: 35.0,
                dimensions: Some([1.5, 1.5, 1.0]),
                vertex_count: 500,
                textures: vec![],
            }],
            hdris: vec![],
            ground_textures: vec![],
            scatter: BiomePackScatter::default(),
            conditions: BiomeConditions::default(),
            sky: BiomeSky::default(),
            terrain_heightmap_path: None,
            fixed_placements_path: None,
            scene_footprint_area: None,
        };

        // Test the BiomeConfig::from_biome_pack() constructor path
        let config = BiomeConfig::from_biome_pack(&pack, BiomeType::Desert);
        assert_eq!(config.biome_type, BiomeType::Desert);
        assert_eq!(config.name, "TestPack");
        assert_eq!(config.vegetation.vegetation_types.len(), 1);
        assert_eq!(config.vegetation.vegetation_types[0].name, "bush_01");

        // Test the ScatterConfig::from_biome_pack() constructor path
        let scatter_config = ScatterConfig::from_biome_pack(&pack);
        assert!(scatter_config.use_poisson_disk);
        assert_eq!(scatter_config.min_distance, 2.0);
    }

    #[test]
    fn test_full_pipeline_biome_pack_to_scatter() {
        // Simulate the full Namaqualand pipeline
        let pack = BiomePack {
            name: "Namaqualand".to_string(),
            description: "South African semi-arid biome with quiver trees and wildflowers"
                .to_string(),
            root_dir: PathBuf::from("assets/Namaqualand"),
            blend_hash: "deadbeef".to_string(),
            assets: vec![
                BiomePackAsset {
                    name: "QuiverTree".to_string(),
                    mesh_path: "meshes/QuiverTree.glb".to_string(),
                    category: "vegetation".to_string(),
                    weight: 0.05,
                    scale_range: (0.9, 1.15),
                    slope_tolerance: 25.0,
                    dimensions: Some([2.0, 2.0, 7.0]),
                    vertex_count: 3000,
                    textures: vec![],
                },
                BiomePackAsset {
                    name: "Gazania".to_string(),
                    mesh_path: "meshes/Gazania.glb".to_string(),
                    category: "vegetation".to_string(),
                    weight: 2.0,
                    scale_range: (0.6, 1.4),
                    slope_tolerance: 45.0,
                    dimensions: Some([0.3, 0.3, 0.15]),
                    vertex_count: 200,
                    textures: vec![],
                },
                BiomePackAsset {
                    name: "Boulder".to_string(),
                    mesh_path: "meshes/Boulder.glb".to_string(),
                    category: "rock".to_string(),
                    weight: 0.15,
                    scale_range: (0.7, 1.5),
                    slope_tolerance: 30.0,
                    dimensions: Some([2.0, 2.0, 1.5]),
                    vertex_count: 800,
                    textures: vec![],
                },
            ],
            hdris: vec![BiomePackHdri {
                filename: "sunset.hdr".to_string(),
                original_name: "Namaqualand Sunset".to_string(),
                width: 4096,
                height: 2048,
            }],
            ground_textures: vec![BiomePackGroundTexture {
                name: "sand".to_string(),
                diffuse: Some("textures/sand_diffuse.png".to_string()),
                normal: Some("textures/sand_normal.png".to_string()),
                roughness: None,
                displacement: None,
            }],
            scatter: BiomePackScatter {
                density: 0.004,
                use_poisson_disk: true,
                min_distance: 1.5,
                max_slope: 40.0,
                size_variation: (0.7, 1.5),
                random_rotation: true,
            },
            conditions: BiomeConditions {
                height_range: (0.0, 200.0),
                temperature_range: (0.6, 0.9),
                moisture_range: (0.1, 0.4),
                max_slope: 45.0,
            },
            sky: BiomeSky::default(),
            terrain_heightmap_path: None,
            fixed_placements_path: None,
            scene_footprint_area: None,
        };

        // Step 1: Convert to BiomeConfig
        let config = pack.to_biome_config(BiomeType::Desert);
        assert_eq!(config.biome_type, BiomeType::Desert);
        assert_eq!(config.name, "Namaqualand");
        assert_eq!(config.vegetation.vegetation_types.len(), 3);
        assert_eq!(config.vegetation.density, 0.004);
        assert!(config.vegetation.random_rotation);

        // Verify vegetation types carry correct model paths
        let tree_type = config
            .vegetation
            .vegetation_types
            .iter()
            .find(|v| v.name == "QuiverTree")
            .unwrap();
        assert!(tree_type.model_path.contains("meshes/QuiverTree.glb"));
        assert_eq!(tree_type.slope_tolerance, 25.0);

        let flower_type = config
            .vegetation
            .vegetation_types
            .iter()
            .find(|v| v.name == "Gazania")
            .unwrap();
        assert!(
            flower_type.weight > tree_type.weight,
            "Flowers should scatter more densely than trees"
        );

        // Step 2: Convert to ScatterConfig
        let scatter_config = pack.to_scatter_config();
        assert!(scatter_config.use_poisson_disk);
        assert_eq!(scatter_config.min_distance, 1.5);
        assert_eq!(scatter_config.max_slope, 40.0);

        // Step 3: Verify conditions are inherited
        assert_eq!(config.conditions.temperature_range, (0.6, 0.9));
        assert_eq!(config.conditions.moisture_range, (0.1, 0.4));

        // Step 4: Verify ground textures
        assert_eq!(config.ground_textures.len(), 1);
        assert!(config.ground_textures[0].contains("sand_diffuse.png"));

        // Step 5: The config is now ready for VegetationScatter::scatter_vegetation()
        // (which is tested in scatter.rs tests with real terrain chunks)
    }

    #[test]
    fn test_ground_texture_detection() {
        let dir = tempfile::TempDir::new().unwrap();
        let tex_dir = dir.path().join("textures");
        std::fs::create_dir_all(&tex_dir).unwrap();

        // Create some terrain texture files
        std::fs::write(tex_dir.join("cliff_diffuse.png"), b"fake").unwrap();
        std::fs::write(tex_dir.join("cliff_normal.png"), b"fake").unwrap();
        std::fs::write(tex_dir.join("sand_color.png"), b"fake").unwrap();
        std::fs::write(tex_dir.join("flower_diffuse.png"), b"fake").unwrap(); // NOT terrain

        let textures = detect_ground_textures(dir.path());

        // Should find cliff and sand groups, not flower
        assert!(textures.len() >= 1, "Should detect at least cliff textures");

        let cliff = textures.iter().find(|t| t.name == "cliff");
        assert!(cliff.is_some(), "Should detect cliff ground textures");

        let cliff = cliff.unwrap();
        assert!(cliff.diffuse.is_some());
        assert!(cliff.normal.is_some());
    }
}
