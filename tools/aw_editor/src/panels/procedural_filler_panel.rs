//! Procedural Filler Panel for the editor
//!
//! Provides comprehensive one-click scene population:
//! - **Procedural Scatter**: Rocks, trees, bushes, grass with density maps
//! - **Spline Roads**: Auto-generated road networks with terrain conformance
//! - **Terrain Generation**: Integration with existing PCG terrain systems
//! - **Environment Presets**: Sky, fog, tonemap combinations for instant mood
//!
//! The goal is to let users quickly fill a scene with believable content
//! by selecting presets and clicking "Generate" rather than manual placement.

use egui::{Color32, RichText, Ui, Vec2};

use crate::panels::Panel;

// ============================================================================
// PANEL ACTIONS - Events produced by the panel for external handling
// ============================================================================

/// Actions emitted by the procedural filler panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FillerAction {
    /// Request to generate scatter objects
    GenerateScatter {
        seed: u64,
        biome: BiomePreset,
        area_radius: f32,
        settings: Vec<ScatterSettings>,
    },
    /// Request to generate a spline road
    GenerateRoad {
        seed: u64,
        preset: RoadPreset,
        width: f32,
        segments: u32,
        smoothing: f32,
    },
    /// Request to generate terrain
    GenerateTerrain {
        seed: u64,
        biome: BiomePreset,
        area_radius: f32,
    },
    /// Request to apply environment preset
    ApplyEnvironment { preset: EnvironmentPreset },
    /// Request to generate full scene
    GenerateFullScene {
        seed: u64,
        biome: BiomePreset,
        environment: EnvironmentPreset,
        area_radius: f32,
    },
    /// Cancel current generation
    CancelGeneration,
    /// Clear all generated content
    ClearGenerated,
}

// ============================================================================
// FILLER MODE - What type of procedural content to generate
// ============================================================================

/// Procedural filler operation mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FillerMode {
    #[default]
    ScatterFill,
    SplineRoad,
    TerrainGen,
    EnvironmentPreset,
    FullScene,
}

impl std::fmt::Display for FillerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl FillerMode {
    /// All available filler modes
    pub fn all() -> &'static [FillerMode] {
        &[
            FillerMode::ScatterFill,
            FillerMode::SplineRoad,
            FillerMode::TerrainGen,
            FillerMode::EnvironmentPreset,
            FillerMode::FullScene,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            FillerMode::ScatterFill => "Scatter Fill",
            FillerMode::SplineRoad => "Spline Road",
            FillerMode::TerrainGen => "Terrain Gen",
            FillerMode::EnvironmentPreset => "Environment",
            FillerMode::FullScene => "Full Scene",
        }
    }

    /// Icon for UI display
    pub fn icon(&self) -> &'static str {
        match self {
            FillerMode::ScatterFill => "🌲",
            FillerMode::SplineRoad => "🛣️",
            FillerMode::TerrainGen => "🏔️",
            FillerMode::EnvironmentPreset => "[Dawn]",
            FillerMode::FullScene => "[Anim]",
        }
    }

    /// Detailed description
    pub fn description(&self) -> &'static str {
        match self {
            FillerMode::ScatterFill => "Procedurally scatter rocks, trees, bushes, and grass",
            FillerMode::SplineRoad => "Generate road networks with terrain conformance",
            FillerMode::TerrainGen => "Generate terrain heightmaps and biomes",
            FillerMode::EnvironmentPreset => "Apply sky, fog, and tonemapping presets",
            FillerMode::FullScene => "Generate complete scene with all elements",
        }
    }

    /// Whether this mode affects terrain
    pub fn affects_terrain(&self) -> bool {
        matches!(
            self,
            FillerMode::TerrainGen | FillerMode::SplineRoad | FillerMode::FullScene
        )
    }

    /// Whether this mode spawns entities
    pub fn spawns_entities(&self) -> bool {
        matches!(
            self,
            FillerMode::ScatterFill | FillerMode::SplineRoad | FillerMode::FullScene
        )
    }
}

// ============================================================================
// SCATTER CATEGORY - Types of objects to scatter
// ============================================================================

/// Category of objects to scatter
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScatterCategory {
    #[default]
    All,
    Trees,
    Rocks,
    Bushes,
    Grass,
    Flowers,
    Debris,
    Props,
}

impl std::fmt::Display for ScatterCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ScatterCategory {
    /// All scatter categories
    pub fn all() -> &'static [ScatterCategory] {
        &[
            ScatterCategory::All,
            ScatterCategory::Trees,
            ScatterCategory::Rocks,
            ScatterCategory::Bushes,
            ScatterCategory::Grass,
            ScatterCategory::Flowers,
            ScatterCategory::Debris,
            ScatterCategory::Props,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            ScatterCategory::All => "All",
            ScatterCategory::Trees => "Trees",
            ScatterCategory::Rocks => "Rocks",
            ScatterCategory::Bushes => "Bushes",
            ScatterCategory::Grass => "Grass",
            ScatterCategory::Flowers => "Flowers",
            ScatterCategory::Debris => "Debris",
            ScatterCategory::Props => "Props",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            ScatterCategory::All => "[Pkg]",
            ScatterCategory::Trees => "🌲",
            ScatterCategory::Rocks => "[Rock]",
            ScatterCategory::Bushes => "[Tree]",
            ScatterCategory::Grass => "[Leaf]",
            ScatterCategory::Flowers => "[Flwr]",
            ScatterCategory::Debris => "[Log]",
            ScatterCategory::Props => "🪑",
        }
    }

    /// Default density (instances per 10m²)
    pub fn default_density(&self) -> f32 {
        match self {
            ScatterCategory::All => 10.0,
            ScatterCategory::Trees => 0.5,
            ScatterCategory::Rocks => 2.0,
            ScatterCategory::Bushes => 5.0,
            ScatterCategory::Grass => 50.0,
            ScatterCategory::Flowers => 20.0,
            ScatterCategory::Debris => 3.0,
            ScatterCategory::Props => 0.1,
        }
    }

    /// Whether this category needs LOD system
    pub fn needs_lod(&self) -> bool {
        matches!(
            self,
            ScatterCategory::Trees | ScatterCategory::Rocks | ScatterCategory::Bushes
        )
    }

    /// Whether this category should cast shadows
    pub fn casts_shadows(&self) -> bool {
        matches!(
            self,
            ScatterCategory::All
                | ScatterCategory::Trees
                | ScatterCategory::Rocks
                | ScatterCategory::Bushes
                | ScatterCategory::Props
        )
    }
}

// ============================================================================
// BIOME PRESET - Pre-configured scatter combinations
// ============================================================================

/// Biome-based scatter presets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BiomePreset {
    #[default]
    Custom,
    TemperateForest,
    TropicalJungle,
    DesertDunes,
    ArcticTundra,
    MediterraneanCoast,
    AlpineMeadow,
    Swampland,
    GrasslandPrairie,
    VolcanicWasteland,
    MysticWoods,
}

impl std::fmt::Display for BiomePreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl BiomePreset {
    /// All biome presets
    pub fn all() -> &'static [BiomePreset] {
        &[
            BiomePreset::Custom,
            BiomePreset::TemperateForest,
            BiomePreset::TropicalJungle,
            BiomePreset::DesertDunes,
            BiomePreset::ArcticTundra,
            BiomePreset::MediterraneanCoast,
            BiomePreset::AlpineMeadow,
            BiomePreset::Swampland,
            BiomePreset::GrasslandPrairie,
            BiomePreset::VolcanicWasteland,
            BiomePreset::MysticWoods,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            BiomePreset::Custom => "Custom",
            BiomePreset::TemperateForest => "Temperate Forest",
            BiomePreset::TropicalJungle => "Tropical Jungle",
            BiomePreset::DesertDunes => "Desert Dunes",
            BiomePreset::ArcticTundra => "Arctic Tundra",
            BiomePreset::MediterraneanCoast => "Mediterranean Coast",
            BiomePreset::AlpineMeadow => "Alpine Meadow",
            BiomePreset::Swampland => "Swampland",
            BiomePreset::GrasslandPrairie => "Grassland Prairie",
            BiomePreset::VolcanicWasteland => "Volcanic Wasteland",
            BiomePreset::MysticWoods => "Mystic Woods",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            BiomePreset::Custom => "⚙️",
            BiomePreset::TemperateForest => "🌲",
            BiomePreset::TropicalJungle => "🌴",
            BiomePreset::DesertDunes => "🏜️",
            BiomePreset::ArcticTundra => "❄️",
            BiomePreset::MediterraneanCoast => "[Wave]",
            BiomePreset::AlpineMeadow => "⛰️",
            BiomePreset::Swampland => "🐊",
            BiomePreset::GrasslandPrairie => "🌾",
            BiomePreset::VolcanicWasteland => "🌋",
            BiomePreset::MysticWoods => "[Orb]",
        }
    }

    /// Map to the terrain system's biome string identifier.
    pub fn terrain_biome_key(&self) -> &'static str {
        match self {
            BiomePreset::Custom => "grassland",
            BiomePreset::TemperateForest => "forest",
            BiomePreset::TropicalJungle => "forest",
            BiomePreset::DesertDunes => "desert",
            BiomePreset::ArcticTundra => "tundra",
            BiomePreset::MediterraneanCoast => "beach",
            BiomePreset::AlpineMeadow => "mountain",
            BiomePreset::Swampland => "swamp",
            BiomePreset::GrasslandPrairie => "grassland",
            BiomePreset::VolcanicWasteland => "desert",
            BiomePreset::MysticWoods => "forest",
        }
    }

    /// Suggested tree density for this biome
    pub fn tree_density(&self) -> f32 {
        match self {
            BiomePreset::Custom => 1.0,
            BiomePreset::TemperateForest => 2.5,
            BiomePreset::TropicalJungle => 4.0,
            BiomePreset::DesertDunes => 0.01,
            BiomePreset::ArcticTundra => 0.1,
            BiomePreset::MediterraneanCoast => 0.8,
            BiomePreset::AlpineMeadow => 0.3,
            BiomePreset::Swampland => 1.5,
            BiomePreset::GrasslandPrairie => 0.05,
            BiomePreset::VolcanicWasteland => 0.0,
            BiomePreset::MysticWoods => 3.0,
        }
    }

    /// Suggested rock density for this biome
    pub fn rock_density(&self) -> f32 {
        match self {
            BiomePreset::Custom => 1.0,
            BiomePreset::TemperateForest => 1.5,
            BiomePreset::TropicalJungle => 0.5,
            BiomePreset::DesertDunes => 0.8,
            BiomePreset::ArcticTundra => 2.0,
            BiomePreset::MediterraneanCoast => 1.0,
            BiomePreset::AlpineMeadow => 3.0,
            BiomePreset::Swampland => 0.2,
            BiomePreset::GrasslandPrairie => 0.3,
            BiomePreset::VolcanicWasteland => 5.0,
            BiomePreset::MysticWoods => 1.0,
        }
    }

    /// Primary color for environment
    pub fn primary_color(&self) -> [f32; 3] {
        match self {
            BiomePreset::Custom => [0.5, 0.5, 0.5],
            BiomePreset::TemperateForest => [0.2, 0.6, 0.3],
            BiomePreset::TropicalJungle => [0.1, 0.7, 0.2],
            BiomePreset::DesertDunes => [0.9, 0.8, 0.5],
            BiomePreset::ArcticTundra => [0.9, 0.95, 1.0],
            BiomePreset::MediterraneanCoast => [0.3, 0.6, 0.8],
            BiomePreset::AlpineMeadow => [0.4, 0.7, 0.4],
            BiomePreset::Swampland => [0.3, 0.4, 0.2],
            BiomePreset::GrasslandPrairie => [0.7, 0.75, 0.4],
            BiomePreset::VolcanicWasteland => [0.3, 0.2, 0.2],
            BiomePreset::MysticWoods => [0.4, 0.3, 0.6],
        }
    }

    /// Whether this biome has water features
    pub fn has_water(&self) -> bool {
        matches!(
            self,
            BiomePreset::MediterraneanCoast | BiomePreset::Swampland | BiomePreset::TropicalJungle
        )
    }
}

// ============================================================================
// ENVIRONMENT PRESET - Sky/Fog/Tonemapping combinations
// ============================================================================

/// Pre-configured environment lighting/atmosphere presets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EnvironmentPreset {
    #[default]
    Custom,
    SunnyDay,
    GoldenHour,
    Overcast,
    Foggy,
    Night,
    Moonlit,
    Stormy,
    Sunset,
    Sunrise,
    Industrial,
    Fantasy,
    SciFi,
    Horror,
    Underwater,
}

impl std::fmt::Display for EnvironmentPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl EnvironmentPreset {
    /// All environment presets
    pub fn all() -> &'static [EnvironmentPreset] {
        &[
            EnvironmentPreset::Custom,
            EnvironmentPreset::SunnyDay,
            EnvironmentPreset::GoldenHour,
            EnvironmentPreset::Overcast,
            EnvironmentPreset::Foggy,
            EnvironmentPreset::Night,
            EnvironmentPreset::Moonlit,
            EnvironmentPreset::Stormy,
            EnvironmentPreset::Sunset,
            EnvironmentPreset::Sunrise,
            EnvironmentPreset::Industrial,
            EnvironmentPreset::Fantasy,
            EnvironmentPreset::SciFi,
            EnvironmentPreset::Horror,
            EnvironmentPreset::Underwater,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            EnvironmentPreset::Custom => "Custom",
            EnvironmentPreset::SunnyDay => "Sunny Day",
            EnvironmentPreset::GoldenHour => "Golden Hour",
            EnvironmentPreset::Overcast => "Overcast",
            EnvironmentPreset::Foggy => "Foggy",
            EnvironmentPreset::Night => "Night",
            EnvironmentPreset::Moonlit => "Moonlit",
            EnvironmentPreset::Stormy => "Stormy",
            EnvironmentPreset::Sunset => "Sunset",
            EnvironmentPreset::Sunrise => "Sunrise",
            EnvironmentPreset::Industrial => "Industrial",
            EnvironmentPreset::Fantasy => "Fantasy",
            EnvironmentPreset::SciFi => "Sci-Fi",
            EnvironmentPreset::Horror => "Horror",
            EnvironmentPreset::Underwater => "Underwater",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            EnvironmentPreset::Custom => "⚙️",
            EnvironmentPreset::SunnyDay => "[Sun]",
            EnvironmentPreset::GoldenHour => "[Dawn]",
            EnvironmentPreset::Overcast => "[Cld]",
            EnvironmentPreset::Foggy => "[Fog]",
            EnvironmentPreset::Night => "🌑",
            EnvironmentPreset::Moonlit => "[Moon]",
            EnvironmentPreset::Stormy => "⛈️",
            EnvironmentPreset::Sunset => "🌇",
            EnvironmentPreset::Sunrise => "🌄",
            EnvironmentPreset::Industrial => "🏭",
            EnvironmentPreset::Fantasy => "[Fx]",
            EnvironmentPreset::SciFi => "[Rkt]",
            EnvironmentPreset::Horror => "[Skull]",
            EnvironmentPreset::Underwater => "🐟",
        }
    }

    /// Suggested sun intensity (0.0-2.0)
    pub fn sun_intensity(&self) -> f32 {
        match self {
            EnvironmentPreset::Custom => 1.0,
            EnvironmentPreset::SunnyDay => 1.5,
            EnvironmentPreset::GoldenHour => 1.2,
            EnvironmentPreset::Overcast => 0.5,
            EnvironmentPreset::Foggy => 0.3,
            EnvironmentPreset::Night => 0.0,
            EnvironmentPreset::Moonlit => 0.1,
            EnvironmentPreset::Stormy => 0.2,
            EnvironmentPreset::Sunset => 0.8,
            EnvironmentPreset::Sunrise => 0.6,
            EnvironmentPreset::Industrial => 0.7,
            EnvironmentPreset::Fantasy => 1.0,
            EnvironmentPreset::SciFi => 0.8,
            EnvironmentPreset::Horror => 0.2,
            EnvironmentPreset::Underwater => 0.4,
        }
    }

    /// Suggested ambient intensity (0.0-1.0)
    pub fn ambient_intensity(&self) -> f32 {
        match self {
            EnvironmentPreset::Custom => 0.3,
            EnvironmentPreset::SunnyDay => 0.4,
            EnvironmentPreset::GoldenHour => 0.3,
            EnvironmentPreset::Overcast => 0.5,
            EnvironmentPreset::Foggy => 0.6,
            EnvironmentPreset::Night => 0.05,
            EnvironmentPreset::Moonlit => 0.1,
            EnvironmentPreset::Stormy => 0.2,
            EnvironmentPreset::Sunset => 0.25,
            EnvironmentPreset::Sunrise => 0.25,
            EnvironmentPreset::Industrial => 0.3,
            EnvironmentPreset::Fantasy => 0.4,
            EnvironmentPreset::SciFi => 0.35,
            EnvironmentPreset::Horror => 0.1,
            EnvironmentPreset::Underwater => 0.5,
        }
    }

    /// Suggested fog density (0.0-1.0)
    pub fn fog_density(&self) -> f32 {
        match self {
            EnvironmentPreset::Custom => 0.0,
            EnvironmentPreset::SunnyDay => 0.01,
            EnvironmentPreset::GoldenHour => 0.05,
            EnvironmentPreset::Overcast => 0.1,
            EnvironmentPreset::Foggy => 0.5,
            EnvironmentPreset::Night => 0.02,
            EnvironmentPreset::Moonlit => 0.08,
            EnvironmentPreset::Stormy => 0.3,
            EnvironmentPreset::Sunset => 0.05,
            EnvironmentPreset::Sunrise => 0.1,
            EnvironmentPreset::Industrial => 0.2,
            EnvironmentPreset::Fantasy => 0.15,
            EnvironmentPreset::SciFi => 0.05,
            EnvironmentPreset::Horror => 0.4,
            EnvironmentPreset::Underwater => 0.6,
        }
    }

    /// Sky color (RGB normalized)
    pub fn sky_color(&self) -> [f32; 3] {
        match self {
            EnvironmentPreset::Custom => [0.5, 0.6, 0.8],
            EnvironmentPreset::SunnyDay => [0.4, 0.6, 0.9],
            EnvironmentPreset::GoldenHour => [0.9, 0.7, 0.4],
            EnvironmentPreset::Overcast => [0.6, 0.65, 0.7],
            EnvironmentPreset::Foggy => [0.75, 0.75, 0.8],
            EnvironmentPreset::Night => [0.02, 0.02, 0.05],
            EnvironmentPreset::Moonlit => [0.05, 0.08, 0.15],
            EnvironmentPreset::Stormy => [0.2, 0.22, 0.25],
            EnvironmentPreset::Sunset => [0.95, 0.5, 0.3],
            EnvironmentPreset::Sunrise => [0.9, 0.6, 0.5],
            EnvironmentPreset::Industrial => [0.5, 0.45, 0.4],
            EnvironmentPreset::Fantasy => [0.6, 0.4, 0.8],
            EnvironmentPreset::SciFi => [0.2, 0.3, 0.5],
            EnvironmentPreset::Horror => [0.15, 0.1, 0.1],
            EnvironmentPreset::Underwater => [0.1, 0.3, 0.5],
        }
    }

    /// Fog color (RGB normalized)
    pub fn fog_color(&self) -> [f32; 3] {
        match self {
            EnvironmentPreset::Custom => [0.7, 0.7, 0.7],
            EnvironmentPreset::SunnyDay => [0.8, 0.85, 0.9],
            EnvironmentPreset::GoldenHour => [0.95, 0.8, 0.5],
            EnvironmentPreset::Overcast => [0.7, 0.7, 0.75],
            EnvironmentPreset::Foggy => [0.85, 0.85, 0.9],
            EnvironmentPreset::Night => [0.05, 0.05, 0.08],
            EnvironmentPreset::Moonlit => [0.1, 0.12, 0.2],
            EnvironmentPreset::Stormy => [0.3, 0.3, 0.35],
            EnvironmentPreset::Sunset => [0.9, 0.6, 0.4],
            EnvironmentPreset::Sunrise => [0.85, 0.7, 0.6],
            EnvironmentPreset::Industrial => [0.6, 0.55, 0.5],
            EnvironmentPreset::Fantasy => [0.7, 0.5, 0.8],
            EnvironmentPreset::SciFi => [0.3, 0.4, 0.6],
            EnvironmentPreset::Horror => [0.2, 0.15, 0.15],
            EnvironmentPreset::Underwater => [0.15, 0.35, 0.5],
        }
    }

    /// Recommended tonemapper for this preset
    pub fn tonemapper(&self) -> &'static str {
        match self {
            EnvironmentPreset::Custom => "ACES",
            EnvironmentPreset::SunnyDay => "ACES",
            EnvironmentPreset::GoldenHour => "Filmic",
            EnvironmentPreset::Overcast => "Neutral",
            EnvironmentPreset::Foggy => "Neutral",
            EnvironmentPreset::Night => "AgX",
            EnvironmentPreset::Moonlit => "AgX",
            EnvironmentPreset::Stormy => "Filmic",
            EnvironmentPreset::Sunset => "Filmic",
            EnvironmentPreset::Sunrise => "Filmic",
            EnvironmentPreset::Industrial => "ACES",
            EnvironmentPreset::Fantasy => "ACES",
            EnvironmentPreset::SciFi => "ACES",
            EnvironmentPreset::Horror => "AgX",
            EnvironmentPreset::Underwater => "Neutral",
        }
    }

    /// Whether this preset uses a procedural sky or HDRI
    pub fn uses_procedural_sky(&self) -> bool {
        !matches!(self, EnvironmentPreset::Custom | EnvironmentPreset::SciFi)
    }

    /// Is this a daytime preset?
    pub fn is_daytime(&self) -> bool {
        matches!(
            self,
            EnvironmentPreset::SunnyDay
                | EnvironmentPreset::GoldenHour
                | EnvironmentPreset::Overcast
                | EnvironmentPreset::Foggy
                | EnvironmentPreset::Sunset
                | EnvironmentPreset::Sunrise
        )
    }
}

// ============================================================================
// ROAD PRESET - Spline road generation presets
// ============================================================================

/// Road generation style presets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RoadPreset {
    #[default]
    Custom,
    Asphalt,
    DirtPath,
    CobbleStone,
    GravelRoad,
    ForestTrail,
    DesertTrack,
    SnowPath,
    WoodenBridge,
    StoneRoad,
}

impl std::fmt::Display for RoadPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl RoadPreset {
    /// All road presets
    pub fn all() -> &'static [RoadPreset] {
        &[
            RoadPreset::Custom,
            RoadPreset::Asphalt,
            RoadPreset::DirtPath,
            RoadPreset::CobbleStone,
            RoadPreset::GravelRoad,
            RoadPreset::ForestTrail,
            RoadPreset::DesertTrack,
            RoadPreset::SnowPath,
            RoadPreset::WoodenBridge,
            RoadPreset::StoneRoad,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            RoadPreset::Custom => "Custom",
            RoadPreset::Asphalt => "Asphalt",
            RoadPreset::DirtPath => "Dirt Path",
            RoadPreset::CobbleStone => "Cobblestone",
            RoadPreset::GravelRoad => "Gravel Road",
            RoadPreset::ForestTrail => "Forest Trail",
            RoadPreset::DesertTrack => "Desert Track",
            RoadPreset::SnowPath => "Snow Path",
            RoadPreset::WoodenBridge => "Wooden Bridge",
            RoadPreset::StoneRoad => "Stone Road",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            RoadPreset::Custom => "⚙️",
            RoadPreset::Asphalt => "🛣️",
            RoadPreset::DirtPath => "🚶",
            RoadPreset::CobbleStone => "[Brk]",
            RoadPreset::GravelRoad => "[Rock]",
            RoadPreset::ForestTrail => "🌲",
            RoadPreset::DesertTrack => "🏜️",
            RoadPreset::SnowPath => "❄️",
            RoadPreset::WoodenBridge => "🌉",
            RoadPreset::StoneRoad => "🏛️",
        }
    }

    /// Default road width in meters
    pub fn default_width(&self) -> f32 {
        match self {
            RoadPreset::Custom => 4.0,
            RoadPreset::Asphalt => 8.0,
            RoadPreset::DirtPath => 2.0,
            RoadPreset::CobbleStone => 5.0,
            RoadPreset::GravelRoad => 4.0,
            RoadPreset::ForestTrail => 1.5,
            RoadPreset::DesertTrack => 3.0,
            RoadPreset::SnowPath => 2.0,
            RoadPreset::WoodenBridge => 3.0,
            RoadPreset::StoneRoad => 6.0,
        }
    }

    /// Whether this road type conforms to terrain
    pub fn conforms_to_terrain(&self) -> bool {
        !matches!(self, RoadPreset::WoodenBridge)
    }

    /// Whether to add edge decorations (grass, rocks, etc.)
    pub fn has_edge_decoration(&self) -> bool {
        matches!(
            self,
            RoadPreset::DirtPath
                | RoadPreset::ForestTrail
                | RoadPreset::CobbleStone
                | RoadPreset::StoneRoad
        )
    }
}

// ============================================================================
// SCATTER SETTINGS - Configuration for scatter operations
// ============================================================================

/// Settings for procedural scatter operations
#[derive(Debug, Clone, PartialEq)]
pub struct ScatterSettings {
    pub enabled: bool,
    pub category: ScatterCategory,
    pub density: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    pub random_rotation: bool,
    pub align_to_normal: bool,
    pub min_slope: f32,
    pub max_slope: f32,
    pub min_altitude: f32,
    pub max_altitude: f32,
    pub exclusion_radius: f32,
    pub collision_enabled: bool,
    pub cast_shadows: bool,
}

impl Default for ScatterSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            category: ScatterCategory::All,
            density: 10.0,
            min_scale: 0.8,
            max_scale: 1.2,
            random_rotation: true,
            align_to_normal: true,
            min_slope: 0.0,
            max_slope: 45.0,
            min_altitude: -1000.0,
            max_altitude: 1000.0,
            exclusion_radius: 0.5,
            collision_enabled: false,
            cast_shadows: true,
        }
    }
}

// ============================================================================
// FILLER STATE - Panel state
// ============================================================================

/// State for the procedural filler panel
#[derive(Debug)]
pub struct ProceduralFillerPanel {
    pub mode: FillerMode,
    pub biome_preset: BiomePreset,
    pub environment_preset: EnvironmentPreset,
    pub road_preset: RoadPreset,

    // Scatter settings per category
    pub scatter_trees: ScatterSettings,
    pub scatter_rocks: ScatterSettings,
    pub scatter_bushes: ScatterSettings,
    pub scatter_grass: ScatterSettings,

    // General settings
    pub seed: u64,
    pub area_radius: f32,
    pub preview_enabled: bool,
    pub generation_progress: f32,
    pub is_generating: bool,

    // Road settings
    pub road_width: f32,
    pub road_segments: u32,
    pub road_smoothing: f32,

    // Statistics
    pub last_generated_count: u32,
    pub last_generation_time_ms: u64,

    // Pending actions to be consumed by the editor
    pub pending_actions: Vec<FillerAction>,
}

impl Default for ProceduralFillerPanel {
    fn default() -> Self {
        Self {
            mode: FillerMode::ScatterFill,
            biome_preset: BiomePreset::TemperateForest,
            environment_preset: EnvironmentPreset::SunnyDay,
            road_preset: RoadPreset::DirtPath,

            scatter_trees: ScatterSettings {
                category: ScatterCategory::Trees,
                density: 0.5,
                ..Default::default()
            },
            scatter_rocks: ScatterSettings {
                category: ScatterCategory::Rocks,
                density: 2.0,
                ..Default::default()
            },
            scatter_bushes: ScatterSettings {
                category: ScatterCategory::Bushes,
                density: 5.0,
                ..Default::default()
            },
            scatter_grass: ScatterSettings {
                category: ScatterCategory::Grass,
                density: 50.0,
                ..Default::default()
            },

            seed: 12345,
            area_radius: 100.0,
            preview_enabled: true,
            generation_progress: 0.0,
            is_generating: false,

            road_width: 4.0,
            road_segments: 32,
            road_smoothing: 0.5,

            last_generated_count: 0,
            last_generation_time_ms: 0,

            pending_actions: Vec::new(),
        }
    }
}

impl ProceduralFillerPanel {
    /// Create a new procedural filler panel
    pub fn new() -> Self {
        Self::default()
    }

    /// Take all pending actions (clears the queue)
    pub fn take_actions(&mut self) -> Vec<FillerAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue a filler action
    fn queue_action(&mut self, action: FillerAction) {
        self.pending_actions.push(action);
    }

    /// Collect current scatter settings into a vec
    fn collect_scatter_settings(&self) -> Vec<ScatterSettings> {
        let mut settings = Vec::new();
        if self.scatter_trees.enabled {
            settings.push(self.scatter_trees.clone());
        }
        if self.scatter_rocks.enabled {
            settings.push(self.scatter_rocks.clone());
        }
        if self.scatter_bushes.enabled {
            settings.push(self.scatter_bushes.clone());
        }
        if self.scatter_grass.enabled {
            settings.push(self.scatter_grass.clone());
        }
        settings
    }

    /// Apply biome preset to scatter settings
    pub fn apply_biome_preset(&mut self) {
        let preset = self.biome_preset;
        self.scatter_trees.density = preset.tree_density();
        self.scatter_rocks.density = preset.rock_density();

        // Apply matching environment preset
        self.environment_preset = match preset {
            BiomePreset::TemperateForest => EnvironmentPreset::SunnyDay,
            BiomePreset::TropicalJungle => EnvironmentPreset::Foggy,
            BiomePreset::DesertDunes => EnvironmentPreset::GoldenHour,
            BiomePreset::ArcticTundra => EnvironmentPreset::Overcast,
            BiomePreset::MediterraneanCoast => EnvironmentPreset::SunnyDay,
            BiomePreset::AlpineMeadow => EnvironmentPreset::SunnyDay,
            BiomePreset::Swampland => EnvironmentPreset::Foggy,
            BiomePreset::GrasslandPrairie => EnvironmentPreset::GoldenHour,
            BiomePreset::VolcanicWasteland => EnvironmentPreset::Stormy,
            BiomePreset::MysticWoods => EnvironmentPreset::Fantasy,
            BiomePreset::Custom => self.environment_preset,
        };
    }

    fn render_mode_selector(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for mode in FillerMode::all() {
                let selected = self.mode == *mode;
                let button = if selected {
                    egui::Button::new(RichText::new(format!("{}", mode)).strong())
                        .fill(Color32::from_rgb(80, 120, 80))
                } else {
                    egui::Button::new(format!("{}", mode))
                };

                if ui.add(button).clicked() {
                    self.mode = *mode;
                }
            }
        });
    }

    fn render_scatter_mode(&mut self, ui: &mut Ui) {
        ui.heading("🌲 Scatter Fill");

        // Biome preset
        ui.horizontal(|ui| {
            ui.label("Biome Preset:");
            egui::ComboBox::from_id_salt("biome_preset")
                .selected_text(format!("{}", self.biome_preset))
                .show_ui(ui, |ui| {
                    for preset in BiomePreset::all() {
                        if ui
                            .selectable_label(self.biome_preset == *preset, format!("{}", preset))
                            .clicked()
                        {
                            self.biome_preset = *preset;
                            self.apply_biome_preset();
                        }
                    }
                });
        });

        ui.separator();

        // Scatter category settings
        egui::CollapsingHeader::new("🌲 Trees")
            .default_open(true)
            .show(ui, |ui| {
                self.render_scatter_settings(ui, &mut self.scatter_trees.clone());
            });

        egui::CollapsingHeader::new("[Rock] Rocks")
            .default_open(false)
            .show(ui, |ui| {
                self.render_scatter_settings(ui, &mut self.scatter_rocks.clone());
            });

        egui::CollapsingHeader::new("Bushes")
            .default_open(false)
            .show(ui, |ui| {
                self.render_scatter_settings(ui, &mut self.scatter_bushes.clone());
            });

        egui::CollapsingHeader::new("[Leaf] Grass")
            .default_open(false)
            .show(ui, |ui| {
                self.render_scatter_settings(ui, &mut self.scatter_grass.clone());
            });
    }

    fn render_scatter_settings(&mut self, ui: &mut Ui, _settings: &mut ScatterSettings) {
        // Note: settings parameter is currently unused but kept for future use
        // when per-category settings become editable

        ui.horizontal(|ui| {
            ui.label("Density:");
            ui.add(egui::DragValue::new(&mut self.scatter_trees.density).range(0.0..=100.0));
        });

        ui.horizontal(|ui| {
            ui.label("Scale Range:");
            ui.add(egui::DragValue::new(&mut self.scatter_trees.min_scale).range(0.1..=5.0));
            ui.label("-");
            ui.add(egui::DragValue::new(&mut self.scatter_trees.max_scale).range(0.1..=5.0));
        });

        ui.checkbox(&mut self.scatter_trees.random_rotation, "Random Rotation");
        ui.checkbox(&mut self.scatter_trees.align_to_normal, "Align to Normal");
        ui.checkbox(&mut self.scatter_trees.cast_shadows, "Cast Shadows");
    }

    fn render_road_mode(&mut self, ui: &mut Ui) {
        ui.heading("🛣️ Spline Road");

        ui.horizontal(|ui| {
            ui.label("Road Preset:");
            egui::ComboBox::from_id_salt("road_preset")
                .selected_text(format!("{}", self.road_preset))
                .show_ui(ui, |ui| {
                    for preset in RoadPreset::all() {
                        if ui
                            .selectable_label(self.road_preset == *preset, format!("{}", preset))
                            .clicked()
                        {
                            self.road_preset = *preset;
                            self.road_width = preset.default_width();
                        }
                    }
                });
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Width:");
            ui.add(
                egui::DragValue::new(&mut self.road_width)
                    .range(0.5..=20.0)
                    .suffix(" m"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Segments:");
            ui.add(egui::DragValue::new(&mut self.road_segments).range(4..=128));
        });

        ui.horizontal(|ui| {
            ui.label("Smoothing:");
            ui.add(egui::Slider::new(&mut self.road_smoothing, 0.0..=1.0));
        });

        ui.separator();

        ui.label(format!(
            "Conforms to terrain: {}",
            if self.road_preset.conforms_to_terrain() {
                "Yes"
            } else {
                "No"
            }
        ));
        ui.label(format!(
            "Edge decoration: {}",
            if self.road_preset.has_edge_decoration() {
                "Yes"
            } else {
                "No"
            }
        ));
    }

    fn render_environment_mode(&mut self, ui: &mut Ui) {
        ui.heading("[Dawn] Environment Preset");

        egui::ComboBox::from_id_salt("env_preset")
            .selected_text(format!("{}", self.environment_preset))
            .show_ui(ui, |ui| {
                for preset in EnvironmentPreset::all() {
                    if ui
                        .selectable_label(self.environment_preset == *preset, format!("{}", preset))
                        .clicked()
                    {
                        self.environment_preset = *preset;
                    }
                }
            });

        ui.separator();

        let preset = self.environment_preset;

        ui.label(format!("Sun Intensity: {:.2}", preset.sun_intensity()));
        ui.label(format!("Ambient: {:.2}", preset.ambient_intensity()));
        ui.label(format!("Fog Density: {:.2}", preset.fog_density()));
        ui.label(format!("Tonemapper: {}", preset.tonemapper()));

        ui.separator();

        // Sky color preview
        let sky = preset.sky_color();
        let sky_color = Color32::from_rgb(
            (sky[0] * 255.0) as u8,
            (sky[1] * 255.0) as u8,
            (sky[2] * 255.0) as u8,
        );
        ui.horizontal(|ui| {
            ui.label("Sky Color:");
            let (rect, _response) =
                ui.allocate_exact_size(Vec2::new(60.0, 20.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, sky_color);
        });

        // Fog color preview
        let fog = preset.fog_color();
        let fog_color = Color32::from_rgb(
            (fog[0] * 255.0) as u8,
            (fog[1] * 255.0) as u8,
            (fog[2] * 255.0) as u8,
        );
        ui.horizontal(|ui| {
            ui.label("Fog Color:");
            let (rect, _response) =
                ui.allocate_exact_size(Vec2::new(60.0, 20.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, fog_color);
        });
    }

    fn render_generation_controls(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.heading("Generation");

        ui.horizontal(|ui| {
            ui.label("Seed:");
            ui.add(egui::DragValue::new(&mut self.seed));
            if ui.button("[Dice]").clicked() {
                self.seed = rand::random();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Area Radius:");
            ui.add(
                egui::DragValue::new(&mut self.area_radius)
                    .range(10.0..=1000.0)
                    .suffix(" m"),
            );
        });

        ui.checkbox(&mut self.preview_enabled, "Show Preview");

        ui.separator();

        if self.is_generating {
            ui.add(egui::ProgressBar::new(self.generation_progress).text("Generating..."));
            if ui.button("Cancel").clicked() {
                self.is_generating = false;
                self.queue_action(FillerAction::CancelGeneration);
            }
        } else {
            let generate_text = format!("Generate {}", self.mode.name());
            if ui.button(RichText::new(generate_text).strong()).clicked() {
                self.is_generating = true;
                self.generation_progress = 0.0;

                // Emit the appropriate action based on current mode
                let action = match self.mode {
                    FillerMode::ScatterFill => FillerAction::GenerateScatter {
                        seed: self.seed,
                        biome: self.biome_preset,
                        area_radius: self.area_radius,
                        settings: self.collect_scatter_settings(),
                    },
                    FillerMode::SplineRoad => FillerAction::GenerateRoad {
                        seed: self.seed,
                        preset: self.road_preset,
                        width: self.road_width,
                        segments: self.road_segments,
                        smoothing: self.road_smoothing,
                    },
                    FillerMode::TerrainGen => FillerAction::GenerateTerrain {
                        seed: self.seed,
                        biome: self.biome_preset,
                        area_radius: self.area_radius,
                    },
                    FillerMode::EnvironmentPreset => FillerAction::ApplyEnvironment {
                        preset: self.environment_preset,
                    },
                    FillerMode::FullScene => FillerAction::GenerateFullScene {
                        seed: self.seed,
                        biome: self.biome_preset,
                        environment: self.environment_preset,
                        area_radius: self.area_radius,
                    },
                };
                self.queue_action(action);
            }
        }

        if self.last_generated_count > 0 {
            ui.separator();
            ui.label(format!(
                "Last: {} objects in {} ms",
                self.last_generated_count, self.last_generation_time_ms
            ));
        }
    }
}

impl Panel for ProceduralFillerPanel {
    fn name(&self) -> &'static str {
        "Procedural Filler"
    }

    fn show(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.render_mode_selector(ui);

            ui.separator();

            match self.mode {
                FillerMode::ScatterFill => self.render_scatter_mode(ui),
                FillerMode::SplineRoad => self.render_road_mode(ui),
                FillerMode::TerrainGen => {
                    ui.heading("🏔️ Terrain Generation");
                    ui.label("Use the Terrain Panel for advanced terrain generation.");
                    ui.label("This mode provides quick preset-based terrain.");
                }
                FillerMode::EnvironmentPreset => self.render_environment_mode(ui),
                FillerMode::FullScene => {
                    ui.heading("Full Scene Generation");
                    ui.label("Generates terrain, scatter, roads, and environment in one click.");
                    self.render_scatter_mode(ui);
                    ui.separator();
                    self.render_environment_mode(ui);
                }
            }

            self.render_generation_controls(ui);
        });
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filler_mode_display() {
        assert!(format!("{}", FillerMode::ScatterFill).contains("Scatter Fill"));
        assert!(format!("{}", FillerMode::SplineRoad).contains("Spline Road"));
        assert!(format!("{}", FillerMode::EnvironmentPreset).contains("Environment"));
    }

    #[test]
    fn test_filler_mode_all() {
        let modes = FillerMode::all();
        assert_eq!(modes.len(), 5);
        assert!(modes.contains(&FillerMode::ScatterFill));
        assert!(modes.contains(&FillerMode::FullScene));
    }

    #[test]
    fn test_filler_mode_properties() {
        assert!(FillerMode::TerrainGen.affects_terrain());
        assert!(!FillerMode::EnvironmentPreset.affects_terrain());
        assert!(FillerMode::ScatterFill.spawns_entities());
        assert!(!FillerMode::EnvironmentPreset.spawns_entities());
    }

    #[test]
    fn test_scatter_category_display() {
        assert!(format!("{}", ScatterCategory::Trees).contains("Trees"));
        assert!(format!("{}", ScatterCategory::Rocks).contains("Rocks"));
    }

    #[test]
    fn test_scatter_category_all() {
        let cats = ScatterCategory::all();
        assert_eq!(cats.len(), 8);
    }

    #[test]
    fn test_scatter_category_properties() {
        assert!(ScatterCategory::Trees.needs_lod());
        assert!(!ScatterCategory::Grass.needs_lod());
        assert!(ScatterCategory::Trees.casts_shadows());
        assert!(!ScatterCategory::Grass.casts_shadows());
    }

    #[test]
    fn test_biome_preset_display() {
        assert!(format!("{}", BiomePreset::TemperateForest).contains("Temperate Forest"));
        assert!(format!("{}", BiomePreset::DesertDunes).contains("Desert"));
    }

    #[test]
    fn test_biome_preset_all() {
        let biomes = BiomePreset::all();
        assert_eq!(biomes.len(), 11);
    }

    #[test]
    fn test_biome_preset_densities() {
        assert!(
            BiomePreset::TemperateForest.tree_density() > BiomePreset::DesertDunes.tree_density()
        );
        assert!(
            BiomePreset::VolcanicWasteland.rock_density() > BiomePreset::Swampland.rock_density()
        );
    }

    #[test]
    fn test_biome_preset_water() {
        assert!(BiomePreset::Swampland.has_water());
        assert!(BiomePreset::MediterraneanCoast.has_water());
        assert!(!BiomePreset::DesertDunes.has_water());
    }

    #[test]
    fn test_environment_preset_display() {
        assert!(format!("{}", EnvironmentPreset::SunnyDay).contains("Sunny Day"));
        assert!(format!("{}", EnvironmentPreset::Horror).contains("Horror"));
    }

    #[test]
    fn test_environment_preset_all() {
        let presets = EnvironmentPreset::all();
        assert_eq!(presets.len(), 15);
    }

    #[test]
    fn test_environment_preset_lighting() {
        assert!(
            EnvironmentPreset::SunnyDay.sun_intensity() > EnvironmentPreset::Night.sun_intensity()
        );
        assert!(EnvironmentPreset::Foggy.fog_density() > EnvironmentPreset::SunnyDay.fog_density());
    }

    #[test]
    fn test_environment_preset_daytime() {
        assert!(EnvironmentPreset::SunnyDay.is_daytime());
        assert!(EnvironmentPreset::GoldenHour.is_daytime());
        assert!(!EnvironmentPreset::Night.is_daytime());
        assert!(!EnvironmentPreset::Moonlit.is_daytime());
    }

    #[test]
    fn test_road_preset_display() {
        assert!(format!("{}", RoadPreset::Asphalt).contains("Asphalt"));
        assert!(format!("{}", RoadPreset::DirtPath).contains("Dirt"));
    }

    #[test]
    fn test_road_preset_all() {
        let roads = RoadPreset::all();
        assert_eq!(roads.len(), 10);
    }

    #[test]
    fn test_road_preset_properties() {
        assert!(RoadPreset::DirtPath.conforms_to_terrain());
        assert!(!RoadPreset::WoodenBridge.conforms_to_terrain());
        assert!(RoadPreset::ForestTrail.has_edge_decoration());
        assert!(!RoadPreset::Asphalt.has_edge_decoration());
    }

    #[test]
    fn test_road_preset_widths() {
        assert!(RoadPreset::Asphalt.default_width() > RoadPreset::ForestTrail.default_width());
    }

    #[test]
    fn test_scatter_settings_default() {
        let settings = ScatterSettings::default();
        assert!(settings.enabled);
        assert!(settings.random_rotation);
        assert!(settings.cast_shadows);
    }

    #[test]
    fn test_panel_default() {
        let panel = ProceduralFillerPanel::new();
        assert_eq!(panel.mode, FillerMode::ScatterFill);
        assert_eq!(panel.biome_preset, BiomePreset::TemperateForest);
        assert!(!panel.is_generating);
    }

    #[test]
    fn test_apply_biome_preset() {
        let mut panel = ProceduralFillerPanel::new();
        panel.biome_preset = BiomePreset::DesertDunes;
        panel.apply_biome_preset();

        assert!(panel.scatter_trees.density < 0.1); // Desert has very few trees
        assert_eq!(panel.environment_preset, EnvironmentPreset::GoldenHour);
    }

    // ==========================================================================
    // Action System Tests
    // ==========================================================================

    #[test]
    fn test_action_queue_initially_empty() {
        let panel = ProceduralFillerPanel::new();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_take_actions_drains_queue() {
        let mut panel = ProceduralFillerPanel::new();
        panel.pending_actions.push(FillerAction::CancelGeneration);
        panel.pending_actions.push(FillerAction::ClearGenerated);

        assert!(panel.has_pending_actions());
        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_filler_action_variants() {
        let scatter_action = FillerAction::GenerateScatter {
            seed: 42,
            biome: BiomePreset::TemperateForest,
            area_radius: 100.0,
            settings: vec![],
        };
        assert!(matches!(
            scatter_action,
            FillerAction::GenerateScatter { .. }
        ));

        let road_action = FillerAction::GenerateRoad {
            seed: 123,
            preset: RoadPreset::DirtPath,
            width: 3.0,
            segments: 10,
            smoothing: 0.5,
        };
        assert!(matches!(road_action, FillerAction::GenerateRoad { .. }));

        let terrain_action = FillerAction::GenerateTerrain {
            seed: 99,
            biome: BiomePreset::DesertDunes,
            area_radius: 50.0,
        };
        assert!(matches!(
            terrain_action,
            FillerAction::GenerateTerrain { .. }
        ));

        let env_action = FillerAction::ApplyEnvironment {
            preset: EnvironmentPreset::Night,
        };
        assert!(matches!(env_action, FillerAction::ApplyEnvironment { .. }));

        let full_scene = FillerAction::GenerateFullScene {
            seed: 1,
            biome: BiomePreset::TropicalJungle,
            environment: EnvironmentPreset::Sunset,
            area_radius: 200.0,
        };
        assert!(matches!(full_scene, FillerAction::GenerateFullScene { .. }));
    }

    #[test]
    fn test_collect_scatter_settings() {
        let mut panel = ProceduralFillerPanel::new();
        panel.scatter_trees.enabled = true;
        panel.scatter_trees.density = 0.5;
        panel.scatter_rocks.enabled = true;
        panel.scatter_bushes.enabled = false;
        panel.scatter_grass.enabled = false;

        let settings = panel.collect_scatter_settings();
        assert_eq!(settings.len(), 2); // Only enabled scatter layers
    }
}
