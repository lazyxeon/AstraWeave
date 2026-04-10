//! Biome definitions and classification system

use astraweave_gameplay::types::ResourceKind;
use astraweave_gameplay::BiomeRule;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Types of biomes available in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BiomeType {
    Grassland,
    Desert,
    Forest,
    Mountain,
    Tundra,
    Swamp,
    Beach,
    River,
}

impl BiomeType {
    /// Get a string representation of the biome type
    pub fn as_str(&self) -> &'static str {
        match self {
            BiomeType::Grassland => "grassland",
            BiomeType::Desert => "desert",
            BiomeType::Forest => "forest",
            BiomeType::Mountain => "mountain",
            BiomeType::Tundra => "tundra",
            BiomeType::Swamp => "swamp",
            BiomeType::Beach => "beach",
            BiomeType::River => "river",
        }
    }

    /// Get the materials directory path for this biome, relative to the assets root.
    ///
    /// # Example
    /// ```
    /// # use astraweave_terrain::biome::BiomeType;
    /// # use std::path::Path;
    /// assert_eq!(BiomeType::Forest.material_dir(), Path::new("assets/materials/forest"));
    /// ```
    pub fn material_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("assets/materials/{}", self.as_str()))
    }

    /// Get the materials directory path for the generic terrain fallback.
    ///
    /// Used when no biome-specific material set is available or during
    /// cross-biome blending transitions.
    pub fn terrain_fallback_material_dir() -> std::path::PathBuf {
        std::path::PathBuf::from("assets/materials/terrain")
    }

    /// Parse from string (case-insensitive)
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Get all available biome types
    pub fn all() -> &'static [BiomeType] {
        &[
            BiomeType::Grassland,
            BiomeType::Desert,
            BiomeType::Forest,
            BiomeType::Mountain,
            BiomeType::Tundra,
            BiomeType::Swamp,
            BiomeType::Beach,
            BiomeType::River,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseBiomeTypeError;

impl std::fmt::Display for ParseBiomeTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid biome type")
    }
}

impl std::error::Error for ParseBiomeTypeError {}

impl FromStr for BiomeType {
    type Err = ParseBiomeTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.eq_ignore_ascii_case("grassland") {
            return Ok(BiomeType::Grassland);
        }
        if s.eq_ignore_ascii_case("desert") {
            return Ok(BiomeType::Desert);
        }
        if s.eq_ignore_ascii_case("forest") {
            return Ok(BiomeType::Forest);
        }
        if s.eq_ignore_ascii_case("mountain") {
            return Ok(BiomeType::Mountain);
        }
        if s.eq_ignore_ascii_case("tundra") {
            return Ok(BiomeType::Tundra);
        }
        if s.eq_ignore_ascii_case("swamp") {
            return Ok(BiomeType::Swamp);
        }
        if s.eq_ignore_ascii_case("beach") {
            return Ok(BiomeType::Beach);
        }
        if s.eq_ignore_ascii_case("river") {
            return Ok(BiomeType::River);
        }

        Err(ParseBiomeTypeError)
    }
}

/// Environmental conditions for biome classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeConditions {
    /// Height range (min, max) in world units
    pub height_range: (f32, f32),
    /// Temperature range (0.0 = frozen, 1.0 = hot)
    pub temperature_range: (f32, f32),
    /// Moisture range (0.0 = dry, 1.0 = wet)
    pub moisture_range: (f32, f32),
    /// Slope tolerance (max slope in degrees)
    pub max_slope: f32,
}

impl Default for BiomeConditions {
    fn default() -> Self {
        Self {
            height_range: (0.0, 1000.0),
            temperature_range: (0.0, 1.0),
            moisture_range: (0.0, 1.0),
            max_slope: 90.0,
        }
    }
}

/// Sky and weather parameters for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeSky {
    /// Horizon color (RGB)
    pub horizon_color: Vec3,
    /// Zenith color (RGB)
    pub zenith_color: Vec3,
    /// Sun color (RGB)
    pub sun_color: Vec3,
    /// Fog density (0.0 = no fog, 1.0 = thick fog)
    pub fog_density: f32,
    /// Fog color (RGB)
    pub fog_color: Vec3,
    /// Cloud coverage (0.0 = clear, 1.0 = overcast)
    pub cloud_coverage: f32,
    /// Precipitation type
    pub precipitation: PrecipitationType,
}

impl Default for BiomeSky {
    fn default() -> Self {
        Self {
            horizon_color: Vec3::new(0.5, 0.7, 0.9),
            zenith_color: Vec3::new(0.2, 0.4, 0.8),
            sun_color: Vec3::new(1.0, 0.9, 0.7),
            fog_density: 0.0,
            fog_color: Vec3::new(0.8, 0.8, 0.9),
            cloud_coverage: 0.3,
            precipitation: PrecipitationType::None,
        }
    }
}

/// Types of precipitation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PrecipitationType {
    None,
    Rain,
    Snow,
    Fog,
    Sandstorm,
}

/// Vegetation parameters for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeVegetation {
    /// Base vegetation density (objects per unit area)
    pub density: f32,
    /// Available vegetation types with weights
    pub vegetation_types: Vec<VegetationType>,
    /// Size variation for vegetation (min, max scale multipliers)
    pub size_variation: (f32, f32),
    /// Rotation randomization
    pub random_rotation: bool,
}

impl Default for BiomeVegetation {
    fn default() -> Self {
        Self {
            density: 0.005,
            vegetation_types: vec![VegetationType {
                name: "grass".to_string(),
                weight: 1.0,
                model_path: "assets/models/grass.glb".to_string(),
                scale_range: (0.8, 1.2),
                slope_tolerance: 45.0,
            }],
            size_variation: (0.8, 1.5),
            random_rotation: true,
        }
    }
}

/// A single vegetation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VegetationType {
    /// Name of the vegetation type
    pub name: String,
    /// Weight for random selection
    pub weight: f32,
    /// Path to the 3D model
    pub model_path: String,
    /// Scale range (min, max)
    pub scale_range: (f32, f32),
    /// Maximum slope this vegetation can grow on (degrees)
    pub slope_tolerance: f32,
}

/// Complete biome configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeConfig {
    /// The biome type
    pub biome_type: BiomeType,
    /// Human-readable name
    pub name: String,
    /// Description of the biome
    pub description: String,
    /// Environmental conditions for this biome
    pub conditions: BiomeConditions,
    /// Sky and weather parameters
    pub sky: BiomeSky,
    /// Vegetation configuration
    pub vegetation: BiomeVegetation,
    /// Resource spawning (reusing existing astraweave-gameplay system)
    pub resource_weights: Vec<(ResourceKind, f32)>,
    /// Base resource amounts
    pub base_resource_amount: (u32, u32),
    /// Respawn timing for resources
    pub resource_respawn: (f32, f32),
    /// Ground textures for rendering
    pub ground_textures: Vec<String>,
    /// Priority for biome selection (higher wins in conflicts)
    pub priority: i32,
}

impl BiomeConfig {
    /// Create a default grassland biome
    pub fn grassland() -> Self {
        Self {
            biome_type: BiomeType::Grassland,
            name: "Temperate Grassland".to_string(),
            description: "Rolling hills covered in grass with scattered trees".to_string(),
            conditions: BiomeConditions {
                height_range: (0.0, 50.0),
                temperature_range: (0.3, 0.8),
                moisture_range: (0.4, 0.8),
                max_slope: 30.0,
            },
            sky: BiomeSky {
                horizon_color: Vec3::new(0.6, 0.8, 0.9),
                zenith_color: Vec3::new(0.3, 0.5, 0.9),
                sun_color: Vec3::new(1.0, 0.95, 0.8),
                fog_density: 0.05,
                fog_color: Vec3::new(0.9, 0.95, 1.0),
                cloud_coverage: 0.4,
                precipitation: PrecipitationType::Rain,
            },
            vegetation: BiomeVegetation {
                density: 0.003,
                vegetation_types: vec![
                    VegetationType {
                        name: "grass".to_string(),
                        weight: 4.0,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/grass.glb"
                            .to_string(),
                        scale_range: (0.8, 1.4),
                        slope_tolerance: 45.0,
                    },
                    VegetationType {
                        name: "grass_large".to_string(),
                        weight: 2.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/grass_large.glb"
                                .to_string(),
                        scale_range: (0.9, 1.3),
                        slope_tolerance: 40.0,
                    },
                    VegetationType {
                        name: "flower_yellow".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/flower_yellowA.glb"
                                .to_string(),
                        scale_range: (0.7, 1.1),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "flower_red".to_string(),
                        weight: 1.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/flower_redA.glb"
                                .to_string(),
                        scale_range: (0.7, 1.1),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "flower_purple".to_string(),
                        weight: 0.8,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/flower_purpleA.glb"
                                .to_string(),
                        scale_range: (0.7, 1.0),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "oak_tree".to_string(),
                        weight: 0.08,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_2_A_Color1.gltf"
                            .to_string(),
                        scale_range: (0.7, 1.3),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_default".to_string(),
                        weight: 0.06,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_5_A_Color1.gltf"
                                .to_string(),
                        scale_range: (0.7, 1.3),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_round".to_string(),
                        weight: 0.04,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_1_A_Color1.gltf"
                                .to_string(),
                        scale_range: (0.7, 1.3),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_willow".to_string(),
                        weight: 0.02,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_7_B_Color1.gltf"
                                .to_string(),
                        scale_range: (0.7, 1.3),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "plant_bush".to_string(),
                        weight: 0.25,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Bush_1_A_Color1.gltf"
                            .to_string(),
                        scale_range: (0.7, 1.2),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "bush_round".to_string(),
                        weight: 0.15,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Bush_2_C_Color1.gltf"
                                .to_string(),
                        scale_range: (0.6, 1.1),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "rock_small".to_string(),
                        weight: 0.6,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Rock_1_A_Color1.gltf"
                                .to_string(),
                        scale_range: (0.8, 1.5),
                        slope_tolerance: 30.0,
                    },
                ],
                size_variation: (0.8, 1.5),
                random_rotation: true,
            },
            resource_weights: vec![
                (ResourceKind::Fiber, 3.0),
                (ResourceKind::Wood, 2.0),
                (ResourceKind::Essence, 1.0),
            ],
            base_resource_amount: (3, 8),
            resource_respawn: (30.0, 120.0),
            ground_textures: vec![
                "assets/textures/grass_diffuse.png".to_string(),
                "assets/textures/dirt_diffuse.png".to_string(),
            ],
            priority: 1,
        }
    }

    /// Create a default desert biome
    pub fn desert() -> Self {
        Self {
            biome_type: BiomeType::Desert,
            name: "Arid Desert".to_string(),
            description: "Sandy dunes with sparse vegetation and extreme temperatures".to_string(),
            conditions: BiomeConditions {
                height_range: (0.0, 120.0),
                temperature_range: (0.7, 1.0),
                moisture_range: (0.0, 0.3),
                max_slope: 35.0,
            },
            sky: BiomeSky {
                horizon_color: Vec3::new(0.9, 0.7, 0.5),
                zenith_color: Vec3::new(0.6, 0.8, 0.9),
                sun_color: Vec3::new(1.0, 0.9, 0.6),
                fog_density: 0.02,
                fog_color: Vec3::new(0.9, 0.8, 0.6),
                cloud_coverage: 0.1,
                precipitation: PrecipitationType::Sandstorm,
            },
            vegetation: BiomeVegetation {
                density: 0.002,
                vegetation_types: vec![
                    VegetationType {
                        name: "cactus_tall".to_string(),
                        weight: 2.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/cactus_tall.glb"
                                .to_string(),
                        scale_range: (0.7, 1.8),
                        slope_tolerance: 15.0,
                    },
                    VegetationType {
                        name: "cactus_short".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/cactus_short.glb"
                                .to_string(),
                        scale_range: (0.6, 1.4),
                        slope_tolerance: 20.0,
                    },
                    VegetationType {
                        name: "rock_large".to_string(),
                        weight: 1.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/rock_largeA.glb"
                                .to_string(),
                        scale_range: (0.8, 2.0),
                        slope_tolerance: 30.0,
                    },
                    VegetationType {
                        name: "stone_small".to_string(),
                        weight: 2.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/stone_smallA.glb"
                                .to_string(),
                        scale_range: (0.5, 1.2),
                        slope_tolerance: 40.0,
                    },
                    VegetationType {
                        name: "plant_flat".to_string(),
                        weight: 0.8,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/plant_flatShort.glb"
                                .to_string(),
                        scale_range: (0.5, 1.0),
                        slope_tolerance: 20.0,
                    },
                ],
                size_variation: (0.5, 2.0),
                random_rotation: true,
            },
            resource_weights: vec![
                (ResourceKind::Crystal, 2.0),
                (ResourceKind::Ore, 1.5),
                (ResourceKind::Fiber, 0.5),
            ],
            base_resource_amount: (1, 4),
            resource_respawn: (60.0, 300.0),
            ground_textures: vec![
                "assets/textures/sand_diffuse.png".to_string(),
                "assets/textures/sandstone_diffuse.png".to_string(),
            ],
            priority: 2,
        }
    }

    /// Create a default forest biome
    pub fn forest() -> Self {
        Self {
            biome_type: BiomeType::Forest,
            name: "Dense Forest".to_string(),
            description: "Thick woodland with towering trees and rich undergrowth".to_string(),
            conditions: BiomeConditions {
                height_range: (10.0, 80.0),
                temperature_range: (0.4, 0.9),
                moisture_range: (0.6, 1.0),
                max_slope: 40.0,
            },
            sky: BiomeSky {
                horizon_color: Vec3::new(0.4, 0.6, 0.4),
                zenith_color: Vec3::new(0.2, 0.4, 0.6),
                sun_color: Vec3::new(0.9, 1.0, 0.8),
                fog_density: 0.15,
                fog_color: Vec3::new(0.7, 0.8, 0.7),
                cloud_coverage: 0.6,
                precipitation: PrecipitationType::Rain,
            },
            vegetation: BiomeVegetation {
                density: 0.012,
                vegetation_types: vec![
                    VegetationType {
                        name: "pine_tall".to_string(),
                        weight: 2.0,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/tree_pineTallA_detailed.glb".to_string(),
                        scale_range: (0.8, 1.5),
                        slope_tolerance: 30.0,
                    },
                    VegetationType {
                        name: "pine_round".to_string(),
                        weight: 1.5,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/tree_pineRoundA.glb".to_string(),
                        scale_range: (0.75, 1.4),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_detailed".to_string(),
                        weight: 1.5,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_2_C_Color1.gltf".to_string(),
                        scale_range: (0.7, 1.35),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_conifer".to_string(),
                        weight: 1.5,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_3_A_Color1.gltf".to_string(),
                        scale_range: (0.75, 1.4),
                        slope_tolerance: 30.0,
                    },
                    VegetationType {
                        name: "tree_broad".to_string(),
                        weight: 1.5,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_4_B_Color1.gltf".to_string(),
                        scale_range: (0.7, 1.35),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_canopy".to_string(),
                        weight: 0.8,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_6_A_Color1.gltf".to_string(),
                        scale_range: (0.7, 1.3),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "plant_bush_detailed".to_string(),
                        weight: 3.5,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Bush_4_A_Color1.gltf".to_string(),
                        scale_range: (0.7, 1.3),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "grass_leafs".to_string(),
                        weight: 4.0,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/grass_leafs.glb".to_string(),
                        scale_range: (0.8, 1.4),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "mushroom_red".to_string(),
                        weight: 1.0,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/mushroom_redGroup.glb".to_string(),
                        scale_range: (0.5, 1.0),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "mushroom_tan".to_string(),
                        weight: 0.8,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/mushroom_tanGroup.glb".to_string(),
                        scale_range: (0.5, 1.0),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "stump".to_string(),
                        weight: 0.4,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/stump_roundDetailed.glb".to_string(),
                        scale_range: (0.7, 1.2),
                        slope_tolerance: 40.0,
                    },
                    VegetationType {
                        name: "log".to_string(),
                        weight: 0.3,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/log.glb".to_string(),
                        scale_range: (0.8, 1.3),
                        slope_tolerance: 30.0,
                    },
                ],
                size_variation: (0.7, 1.8),
                random_rotation: true,
            },
            resource_weights: vec![
                (ResourceKind::Wood, 4.0),
                (ResourceKind::Fiber, 2.0),
                (ResourceKind::Essence, 2.5),
            ],
            base_resource_amount: (4, 12),
            resource_respawn: (15.0, 60.0),
            ground_textures: vec![
                "assets/textures/forest_floor_diffuse.png".to_string(),
                "assets/textures/moss_diffuse.png".to_string(),
                "assets/textures/bark_diffuse.png".to_string(),
            ],
            priority: 3,
        }
    }

    /// Create a default mountain biome
    pub fn mountain() -> Self {
        Self {
            biome_type: BiomeType::Mountain,
            name: "Rocky Mountains".to_string(),
            description: "High altitude peaks with rocky terrain and sparse vegetation".to_string(),
            conditions: BiomeConditions {
                height_range: (40.0, 350.0),
                temperature_range: (0.0, 0.5),
                moisture_range: (0.2, 0.7),
                max_slope: 70.0,
            },
            sky: BiomeSky {
                horizon_color: Vec3::new(0.7, 0.8, 0.9),
                zenith_color: Vec3::new(0.4, 0.6, 0.9),
                sun_color: Vec3::new(1.0, 0.95, 0.9),
                fog_density: 0.3,
                fog_color: Vec3::new(0.9, 0.9, 1.0),
                cloud_coverage: 0.7,
                precipitation: PrecipitationType::Snow,
            },
            vegetation: BiomeVegetation {
                density: 0.004,
                vegetation_types: vec![
                    VegetationType {
                        name: "pine_small".to_string(),
                        weight: 1.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/tree_pineSmallA.glb"
                                .to_string(),
                        scale_range: (0.65, 1.25),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_cone".to_string(),
                        weight: 0.8,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_3_C_Color1.gltf"
                            .to_string(),
                        scale_range: (0.65, 1.2),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_bare".to_string(),
                        weight: 0.6,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_Bare_1_A_Color1.gltf"
                            .to_string(),
                        scale_range: (0.65, 1.25),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "grass_leafs".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/grass_leafs.glb"
                                .to_string(),
                        scale_range: (0.5, 0.9),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "rock_large".to_string(),
                        weight: 2.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/rock_largeB.glb"
                                .to_string(),
                        scale_range: (1.0, 2.0),
                        slope_tolerance: 40.0,
                    },
                    VegetationType {
                        name: "rock_tall".to_string(),
                        weight: 1.5,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/rock_tallA.glb"
                            .to_string(),
                        scale_range: (0.9, 1.6),
                        slope_tolerance: 40.0,
                    },
                    VegetationType {
                        name: "stone_large".to_string(),
                        weight: 1.2,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/stone_largeA.glb"
                                .to_string(),
                        scale_range: (0.8, 1.5),
                        slope_tolerance: 40.0,
                    },
                    VegetationType {
                        name: "rock_kaykit".to_string(),
                        weight: 1.0,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Rock_3_A_Color1.gltf"
                                .to_string(),
                        scale_range: (1.0, 1.8),
                        slope_tolerance: 40.0,
                    },
                ],
                size_variation: (0.6, 2.0),
                random_rotation: true,
            },
            resource_weights: vec![
                (ResourceKind::Ore, 3.0),
                (ResourceKind::Crystal, 2.0),
                (ResourceKind::Wood, 0.5),
            ],
            base_resource_amount: (2, 6),
            resource_respawn: (45.0, 180.0),
            ground_textures: vec![
                "assets/textures/rock_diffuse.png".to_string(),
                "assets/textures/snow_diffuse.png".to_string(),
                "assets/textures/gravel_diffuse.png".to_string(),
            ],
            priority: 4,
        }
    }

    pub fn tundra() -> Self {
        Self {
            biome_type: BiomeType::Tundra,
            name: "Frozen Tundra".to_string(),
            description: "Frozen wasteland with permafrost and sparse vegetation".to_string(),
            conditions: BiomeConditions {
                height_range: (0.0, 50.0),
                temperature_range: (0.0, 0.2),
                moisture_range: (0.1, 0.5),
                max_slope: 30.0,
            },
            sky: BiomeSky {
                horizon_color: Vec3::new(0.85, 0.9, 0.95),
                zenith_color: Vec3::new(0.6, 0.75, 0.9),
                sun_color: Vec3::new(1.0, 0.95, 0.85),
                fog_density: 0.4,
                fog_color: Vec3::new(0.9, 0.95, 1.0),
                cloud_coverage: 0.6,
                precipitation: PrecipitationType::Snow,
            },
            vegetation: BiomeVegetation {
                density: 0.003,
                vegetation_types: vec![
                    VegetationType {
                        name: "pine_ground".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/tree_pineGroundA.glb"
                                .to_string(),
                        scale_range: (0.5, 0.9),
                        slope_tolerance: 30.0,
                    },
                    VegetationType {
                        name: "rock_small_flat".to_string(),
                        weight: 2.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/rock_smallFlatA.glb"
                                .to_string(),
                        scale_range: (0.6, 1.5),
                        slope_tolerance: 50.0,
                    },
                    VegetationType {
                        name: "stone_small".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/stone_smallA.glb"
                                .to_string(),
                        scale_range: (0.4, 1.0),
                        slope_tolerance: 45.0,
                    },
                    VegetationType {
                        name: "plant_bush_small".to_string(),
                        weight: 1.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/plant_bushSmall.glb"
                                .to_string(),
                        scale_range: (0.4, 0.8),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "tree_bare_tall".to_string(),
                        weight: 0.8,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/Tree_Bare_2_B_Color2.gltf"
                                .to_string(),
                        scale_range: (0.6, 1.0),
                        slope_tolerance: 35.0,
                    },
                    VegetationType {
                        name: "tree_bare_small".to_string(),
                        weight: 0.6,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/Tree_Bare_1_C_Color2.gltf"
                                .to_string(),
                        scale_range: (0.5, 0.9),
                        slope_tolerance: 30.0,
                    },
                ],
                size_variation: (0.4, 1.0),
                random_rotation: true,
            },
            resource_weights: vec![(ResourceKind::Ore, 1.5), (ResourceKind::Crystal, 2.0)],
            base_resource_amount: (1, 3),
            resource_respawn: (60.0, 240.0),
            ground_textures: vec![
                "assets/textures/snow_diffuse.png".to_string(),
                "assets/textures/ice_diffuse.png".to_string(),
            ],
            priority: 5,
        }
    }

    pub fn swamp() -> Self {
        Self {
            biome_type: BiomeType::Swamp,
            name: "Murky Swamp".to_string(),
            description: "Wetlands with murky waters and dense undergrowth".to_string(),
            conditions: BiomeConditions {
                height_range: (-5.0, 15.0),
                temperature_range: (0.4, 0.8),
                moisture_range: (0.8, 1.0),
                max_slope: 15.0,
            },
            sky: BiomeSky {
                horizon_color: Vec3::new(0.5, 0.55, 0.4),
                zenith_color: Vec3::new(0.4, 0.5, 0.4),
                sun_color: Vec3::new(0.9, 0.85, 0.7),
                fog_density: 0.6,
                fog_color: Vec3::new(0.5, 0.55, 0.45),
                cloud_coverage: 0.8,
                precipitation: PrecipitationType::Rain,
            },
            vegetation: BiomeVegetation {
                density: 0.008,
                vegetation_types: vec![
                    VegetationType {
                        name: "tree_default_dark".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/Tree_Bare_1_B_Color2.gltf"
                                .to_string(),
                        scale_range: (0.8, 1.5),
                        slope_tolerance: 20.0,
                    },
                    VegetationType {
                        name: "tree_swamp".to_string(),
                        weight: 1.0,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/Tree_6_C_Color2.gltf"
                                .to_string(),
                        scale_range: (0.8, 1.4),
                        slope_tolerance: 20.0,
                    },
                    VegetationType {
                        name: "stump_old".to_string(),
                        weight: 1.0,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/stump_old.glb"
                            .to_string(),
                        scale_range: (0.7, 1.3),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "grass_leafs_large".to_string(),
                        weight: 3.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/grass_leafsLarge.glb"
                                .to_string(),
                        scale_range: (0.6, 1.2),
                        slope_tolerance: 30.0,
                    },
                    VegetationType {
                        name: "mushroom_red_tall".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/mushroom_redTall.glb"
                                .to_string(),
                        scale_range: (0.5, 1.0),
                        slope_tolerance: 40.0,
                    },
                    VegetationType {
                        name: "mushroom_tan_tall".to_string(),
                        weight: 1.2,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/mushroom_tanTall.glb"
                                .to_string(),
                        scale_range: (0.5, 1.0),
                        slope_tolerance: 40.0,
                    },
                    VegetationType {
                        name: "lily_large".to_string(),
                        weight: 2.0,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/lily_large.glb"
                            .to_string(),
                        scale_range: (0.6, 1.1),
                        slope_tolerance: 15.0,
                    },
                    VegetationType {
                        name: "hanging_moss".to_string(),
                        weight: 0.8,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/hanging_moss.glb"
                                .to_string(),
                        scale_range: (0.7, 1.2),
                        slope_tolerance: 30.0,
                    },
                ],
                size_variation: (0.6, 1.4),
                random_rotation: true,
            },
            resource_weights: vec![(ResourceKind::Fiber, 3.0), (ResourceKind::Wood, 1.5)],
            base_resource_amount: (2, 5),
            resource_respawn: (30.0, 90.0),
            ground_textures: vec![
                "assets/textures/mud_diffuse.png".to_string(),
                "assets/textures/moss_diffuse.png".to_string(),
            ],
            priority: 6,
        }
    }

    pub fn beach() -> Self {
        Self {
            biome_type: BiomeType::Beach,
            name: "Sandy Beach".to_string(),
            description: "Coastal area with sandy shores".to_string(),
            conditions: BiomeConditions {
                height_range: (-2.0, 5.0),
                temperature_range: (0.5, 1.0),
                moisture_range: (0.4, 0.7),
                max_slope: 20.0,
            },
            sky: BiomeSky {
                horizon_color: Vec3::new(0.7, 0.85, 0.95),
                zenith_color: Vec3::new(0.4, 0.6, 0.9),
                sun_color: Vec3::new(1.0, 0.98, 0.9),
                fog_density: 0.15,
                fog_color: Vec3::new(0.8, 0.9, 1.0),
                cloud_coverage: 0.3,
                precipitation: PrecipitationType::None,
            },
            vegetation: BiomeVegetation {
                density: 0.002,
                vegetation_types: vec![
                    VegetationType {
                        name: "palm_tall".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/tree_palmTall.glb"
                                .to_string(),
                        scale_range: (0.8, 1.3),
                        slope_tolerance: 15.0,
                    },
                    VegetationType {
                        name: "palm_bend".to_string(),
                        weight: 1.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/tree_palmBend.glb"
                                .to_string(),
                        scale_range: (0.7, 1.2),
                        slope_tolerance: 15.0,
                    },
                    VegetationType {
                        name: "palm_short".to_string(),
                        weight: 0.8,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/tree_palmShort.glb"
                                .to_string(),
                        scale_range: (0.7, 1.1),
                        slope_tolerance: 20.0,
                    },
                    VegetationType {
                        name: "rock_small_flat".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/rock_smallFlatB.glb"
                                .to_string(),
                        scale_range: (0.5, 1.2),
                        slope_tolerance: 30.0,
                    },
                    VegetationType {
                        name: "grass".to_string(),
                        weight: 2.0,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/grass.glb"
                            .to_string(),
                        scale_range: (0.6, 1.0),
                        slope_tolerance: 25.0,
                    },
                ],
                size_variation: (0.7, 1.2),
                random_rotation: true,
            },
            resource_weights: vec![(ResourceKind::Fiber, 0.5)],
            base_resource_amount: (1, 2),
            resource_respawn: (60.0, 180.0),
            ground_textures: vec!["assets/textures/sand_diffuse.png".to_string()],
            priority: 7,
        }
    }

    pub fn river() -> Self {
        Self {
            biome_type: BiomeType::River,
            name: "River Basin".to_string(),
            description: "Flowing water with lush riverbanks".to_string(),
            conditions: BiomeConditions {
                height_range: (-10.0, 10.0),
                temperature_range: (0.3, 0.8),
                moisture_range: (0.9, 1.0),
                max_slope: 10.0,
            },
            sky: BiomeSky {
                horizon_color: Vec3::new(0.6, 0.75, 0.85),
                zenith_color: Vec3::new(0.4, 0.6, 0.85),
                sun_color: Vec3::new(1.0, 0.95, 0.9),
                fog_density: 0.2,
                fog_color: Vec3::new(0.7, 0.8, 0.9),
                cloud_coverage: 0.4,
                precipitation: PrecipitationType::None,
            },
            vegetation: BiomeVegetation {
                density: 0.006,
                vegetation_types: vec![
                    VegetationType {
                        name: "tree_detailed".to_string(),
                        weight: 1.5,
                        model_path:
                            "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_5_C_Color1.gltf"
                                .to_string(),
                        scale_range: (0.9, 1.4),
                        slope_tolerance: 15.0,
                    },
                    VegetationType {
                        name: "tree_fat".to_string(),
                        weight: 0.8,
                        model_path: "assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_7_A_Color1.gltf"
                            .to_string(),
                        scale_range: (0.8, 1.3),
                        slope_tolerance: 15.0,
                    },
                    VegetationType {
                        name: "grass_large".to_string(),
                        weight: 3.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/grass_large.glb"
                                .to_string(),
                        scale_range: (0.6, 1.1),
                        slope_tolerance: 25.0,
                    },
                    VegetationType {
                        name: "grass_leafs".to_string(),
                        weight: 2.5,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/grass_leafsLarge.glb"
                                .to_string(),
                        scale_range: (0.5, 1.0),
                        slope_tolerance: 20.0,
                    },
                    VegetationType {
                        name: "lily_small".to_string(),
                        weight: 1.5,
                        model_path: "assets/3D assets/Nature Kit/Models/GLTF format/lily_small.glb"
                            .to_string(),
                        scale_range: (0.5, 0.9),
                        slope_tolerance: 10.0,
                    },
                    VegetationType {
                        name: "rock_small".to_string(),
                        weight: 1.0,
                        model_path:
                            "assets/3D assets/Nature Kit/Models/GLTF format/rock_smallB.glb"
                                .to_string(),
                        scale_range: (0.5, 1.2),
                        slope_tolerance: 30.0,
                    },
                ],
                size_variation: (0.6, 1.3),
                random_rotation: true,
            },
            resource_weights: vec![(ResourceKind::Fiber, 2.0), (ResourceKind::Wood, 1.0)],
            base_resource_amount: (2, 4),
            resource_respawn: (40.0, 120.0),
            ground_textures: vec![
                "assets/textures/river_mud_diffuse.png".to_string(),
                "assets/textures/wet_grass_diffuse.png".to_string(),
            ],
            priority: 8,
        }
    }

    /// Score how well this biome fits the given environmental conditions
    pub fn score_conditions(&self, height: f32, temperature: f32, moisture: f32) -> f32 {
        let mut score = 0.0;

        // Height score
        if height >= self.conditions.height_range.0 && height <= self.conditions.height_range.1 {
            score += 1.0;
        } else {
            let height_distance = if height < self.conditions.height_range.0 {
                self.conditions.height_range.0 - height
            } else {
                height - self.conditions.height_range.1
            };
            score -= height_distance * 0.5; // Strong penalty — height is the primary biome signal
        }

        // Temperature score
        if temperature >= self.conditions.temperature_range.0
            && temperature <= self.conditions.temperature_range.1
        {
            score += 1.0;
        } else {
            let temp_distance = if temperature < self.conditions.temperature_range.0 {
                self.conditions.temperature_range.0 - temperature
            } else {
                temperature - self.conditions.temperature_range.1
            };
            score -= temp_distance * 2.0; // Higher penalty for temperature mismatch
        }

        // Moisture score
        if moisture >= self.conditions.moisture_range.0
            && moisture <= self.conditions.moisture_range.1
        {
            score += 1.0;
        } else {
            let moisture_distance = if moisture < self.conditions.moisture_range.0 {
                self.conditions.moisture_range.0 - moisture
            } else {
                moisture - self.conditions.moisture_range.1
            };
            score -= moisture_distance * 1.5; // Moderate penalty for moisture mismatch
        }

        // Add priority bonus
        score += self.priority as f32 * 0.1;

        score
    }

    /// Check if a slope is suitable for this biome
    pub fn is_slope_suitable(&self, slope_degrees: f32) -> bool {
        slope_degrees <= self.conditions.max_slope
    }

    /// Create a BiomeConfig from a [`BiomePack`](crate::biome_pack::BiomePack).
    ///
    /// The `biome_type` parameter selects the closest natural biome classification.
    /// For Namaqualand-style semi-arid packs, use `BiomeType::Desert`.
    /// For temperate forest packs, use `BiomeType::Forest`, etc.
    ///
    /// All vegetation types, scatter parameters, ground textures, conditions,
    /// and sky settings are populated from the pack.
    pub fn from_biome_pack(pack: &crate::biome_pack::BiomePack, biome_type: BiomeType) -> Self {
        pack.to_biome_config(biome_type)
    }
}

/// Runtime biome data for a specific location
#[derive(Debug, Clone)]
pub struct Biome {
    pub biome_type: BiomeType,
    pub config: BiomeConfig,
    pub local_conditions: BiomeConditions,
}

impl Biome {
    /// Create a new biome instance
    pub fn new(biome_type: BiomeType, config: BiomeConfig) -> Self {
        let local_conditions = config.conditions.clone();
        Self {
            biome_type,
            config,
            local_conditions,
        }
    }

    /// Get the vegetation density at this location
    pub fn vegetation_density(&self) -> f32 {
        self.config.vegetation.density
    }

    /// Get resource spawning weights for integration with existing system
    pub fn resource_weights(&self) -> &[(ResourceKind, f32)] {
        &self.config.resource_weights
    }

    /// Convert to the existing BiomeRule format for compatibility
    pub fn to_biome_rule(&self) -> BiomeRule {
        BiomeRule {
            name: self.config.name.clone(),
            weights: self.config.resource_weights.clone(),
            base_amount: self.config.base_resource_amount,
            respawn: self.config.resource_respawn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_type_string_conversion() {
        assert_eq!(BiomeType::Grassland.as_str(), "grassland");
        assert_eq!(BiomeType::parse("desert"), Some(BiomeType::Desert));
        assert_eq!(BiomeType::parse("invalid"), None);
    }

    #[test]
    fn test_biome_config_creation() {
        let grassland = BiomeConfig::grassland();
        assert_eq!(grassland.biome_type, BiomeType::Grassland);
        assert!(!grassland.resource_weights.is_empty());
    }

    #[test]
    fn test_biome_scoring() {
        let grassland = BiomeConfig::grassland();

        // Perfect conditions
        let score1 = grassland.score_conditions(25.0, 0.5, 0.6);

        // Poor conditions
        let score2 = grassland.score_conditions(100.0, 0.1, 0.1);

        assert!(score1 > score2);
    }

    #[test]
    fn test_slope_suitability() {
        let mountain = BiomeConfig::mountain();
        assert!(mountain.is_slope_suitable(45.0));
        assert!(!mountain.is_slope_suitable(80.0));
    }

    #[test]
    fn test_biome_rule_conversion() {
        let forest_config = BiomeConfig::forest();
        let biome = Biome::new(BiomeType::Forest, forest_config);
        let biome_rule = biome.to_biome_rule();

        assert_eq!(biome_rule.name, "Dense Forest");
        assert!(!biome_rule.weights.is_empty());
    }

    #[test]
    fn test_default_biomes() {
        let biomes = vec![
            BiomeConfig::grassland(),
            BiomeConfig::desert(),
            BiomeConfig::forest(),
            BiomeConfig::mountain(),
        ];

        for biome in biomes {
            assert!(!biome.name.is_empty());
            assert!(!biome.description.is_empty());
            assert!(!biome.resource_weights.is_empty());
            assert!(!biome.ground_textures.is_empty());
        }
    }
}
