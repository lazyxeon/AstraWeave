//! Terrain Panel - Advanced procedural terrain generation and editing UI
//!
//! Provides comprehensive controls for:
//! - Procedural terrain generation (seed, biome, chunk radius)
//! - Noise parameter tweaking (octaves, lacunarity, persistence)
//! - Advanced erosion simulation (hydraulic, thermal, wind)
//! - Biome blending with smooth transitions
//! - Texture splatting and material rules
//! - Fluid simulation and water body detection
//! - Real-time preview and regeneration
//! - Voxel brush tools for sculpting

use super::Panel;
use crate::active_tool::{
    ActiveTool, EventDisposition, KeyEvent, MouseEvent, ToolContext,
};
use crate::terrain_integration::{cached_biome_options, TerrainState};
use egui::{Color32, RichText, Ui};
use uuid::{uuid, Uuid};

/// Phase 1.X-Editor-Multi-Tool-Architecture-Sub-phase-3: stable first-party
/// UUID for `TerrainPanel` per ActiveTool registration model (campaign doc
/// §2.5 + Andrew Q5 mod-friendliness). UUID generated 2026-05-04; documented
/// constant; third-party tools generate their own UUIDs that won't collide.
pub const TERRAIN_PANEL_UUID: Uuid = uuid!("a3f1b8c2-7e4d-4a5b-9f3c-1d2e8b7a4c6f");

/// Erosion preset types for quick configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErosionPresetType {
    Custom,
    Desert,
    Mountain,
    Coastal,
    Alpine,
    Canyon,
}

impl std::fmt::Display for ErosionPresetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ErosionPresetType {
    pub fn name(&self) -> &'static str {
        match self {
            ErosionPresetType::Custom => "Custom",
            ErosionPresetType::Desert => "Desert",
            ErosionPresetType::Mountain => "Mountain",
            ErosionPresetType::Coastal => "Coastal",
            ErosionPresetType::Alpine => "Alpine",
            ErosionPresetType::Canyon => "Canyon",
        }
    }

    pub fn all() -> &'static [ErosionPresetType] {
        &[
            ErosionPresetType::Custom,
            ErosionPresetType::Desert,
            ErosionPresetType::Mountain,
            ErosionPresetType::Coastal,
            ErosionPresetType::Alpine,
            ErosionPresetType::Canyon,
        ]
    }
}

/// Configuration for hydraulic erosion
#[derive(Debug, Clone)]
pub struct HydraulicErosionParams {
    pub enabled: bool,
    pub iterations: u32,
    pub inertia: f32,
    pub capacity: f32,
    pub deposition: f32,
    pub erosion: f32,
    pub evaporation: f32,
    pub min_slope: f32,
    pub gravity: f32,
}

impl Default for HydraulicErosionParams {
    fn default() -> Self {
        Self {
            enabled: true,
            iterations: 50000,
            inertia: 0.3,
            capacity: 8.0,
            deposition: 0.2,
            erosion: 0.5,
            evaporation: 0.02,
            min_slope: 0.01,
            gravity: 10.0,
        }
    }
}

/// Configuration for thermal erosion
#[derive(Debug, Clone)]
pub struct ThermalErosionParams {
    pub enabled: bool,
    pub iterations: u32,
    pub talus_angle: f32,
    pub erosion_rate: f32,
}

impl Default for ThermalErosionParams {
    fn default() -> Self {
        Self {
            enabled: true,
            iterations: 50,
            talus_angle: 40.0,
            erosion_rate: 0.5,
        }
    }
}

/// Configuration for wind erosion
#[derive(Debug, Clone)]
pub struct WindErosionParams {
    pub enabled: bool,
    pub iterations: u32,
    pub wind_direction: [f32; 2],
    pub wind_strength: f32,
    pub suspension: f32,
    pub abrasion: f32,
}

impl Default for WindErosionParams {
    fn default() -> Self {
        Self {
            enabled: false,
            iterations: 20,
            wind_direction: [1.0, 0.0],
            wind_strength: 0.5,
            suspension: 0.3,
            abrasion: 0.2,
        }
    }
}

/// Biome blending configuration
#[derive(Debug, Clone)]
pub struct BiomeBlendParams {
    pub enabled: bool,
    pub blend_radius: f32,
    pub falloff_power: f32,
    pub noise_influence: f32,
    pub secondary_biome: String,
    pub tertiary_biome: String,
    pub show_blend_preview: bool,
}

impl Default for BiomeBlendParams {
    fn default() -> Self {
        Self {
            enabled: true,
            blend_radius: 32.0,
            falloff_power: 2.0,
            noise_influence: 0.3,
            secondary_biome: "desert".to_string(),
            tertiary_biome: "mountains".to_string(),
            show_blend_preview: false,
        }
    }
}

/// Texture splatting configuration
#[derive(Debug, Clone)]
pub struct SplatParams {
    pub enabled: bool,
    pub show_splat_preview: bool,
    pub grass_height_min: f32,
    pub grass_height_max: f32,
    pub rock_slope_threshold: f32,
    pub snow_height_threshold: f32,
    pub sand_height_max: f32,
    pub triplanar_sharpness: f32,
}

impl Default for SplatParams {
    fn default() -> Self {
        Self {
            enabled: true,
            show_splat_preview: false,
            grass_height_min: 0.0,
            grass_height_max: 0.7,
            rock_slope_threshold: 0.6,
            snow_height_threshold: 0.85,
            sand_height_max: 0.15,
            triplanar_sharpness: 8.0,
        }
    }
}

/// Water body type for fluid placement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WaterBodyPreset {
    Custom,
    CalmLake,
    MountainStream,
    RagingRiver,
    Ocean,
    Waterfall,
    SwampWetland,
}

impl std::fmt::Display for WaterBodyPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl WaterBodyPreset {
    pub fn name(&self) -> &'static str {
        match self {
            WaterBodyPreset::Custom => "Custom",
            WaterBodyPreset::CalmLake => "Calm Lake",
            WaterBodyPreset::MountainStream => "Mountain Stream",
            WaterBodyPreset::RagingRiver => "Raging River",
            WaterBodyPreset::Ocean => "Ocean",
            WaterBodyPreset::Waterfall => "Waterfall",
            WaterBodyPreset::SwampWetland => "Swamp/Wetland",
        }
    }

    pub fn all() -> &'static [WaterBodyPreset] {
        &[
            WaterBodyPreset::Custom,
            WaterBodyPreset::CalmLake,
            WaterBodyPreset::MountainStream,
            WaterBodyPreset::RagingRiver,
            WaterBodyPreset::Ocean,
            WaterBodyPreset::Waterfall,
            WaterBodyPreset::SwampWetland,
        ]
    }

    pub fn is_flowing(&self) -> bool {
        matches!(
            self,
            WaterBodyPreset::MountainStream
                | WaterBodyPreset::RagingRiver
                | WaterBodyPreset::Waterfall
        )
    }
}

/// Fluid simulation quality preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FluidQualityPreset {
    Performance,
    Balanced,
    Quality,
    Cinematic,
}

impl std::fmt::Display for FluidQualityPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FluidQualityPreset {
    pub fn name(&self) -> &'static str {
        match self {
            FluidQualityPreset::Performance => "Performance",
            FluidQualityPreset::Balanced => "Balanced",
            FluidQualityPreset::Quality => "Quality",
            FluidQualityPreset::Cinematic => "Cinematic",
        }
    }

    pub fn all() -> &'static [FluidQualityPreset] {
        &[
            FluidQualityPreset::Performance,
            FluidQualityPreset::Balanced,
            FluidQualityPreset::Quality,
            FluidQualityPreset::Cinematic,
        ]
    }
}

/// Fluid simulation parameters for terrain integration
#[derive(Debug, Clone)]
pub struct FluidSimParams {
    pub enabled: bool,
    pub quality_preset: FluidQualityPreset,
    pub water_body_preset: WaterBodyPreset,

    // Physics
    pub particle_count: u32,
    pub smoothing_radius: f32,
    pub target_density: f32,
    pub pressure_multiplier: f32,
    pub viscosity: f32,
    pub surface_tension: f32,
    pub gravity: f32,
    pub solver_iterations: u32,

    // Flow
    pub flow_enabled: bool,
    pub flow_speed: f32,
    pub flow_direction: [f32; 2],
    pub turbulence: f32,

    // Rendering
    pub water_color: [f32; 4],
    pub transparency: f32,
    pub refraction_strength: f32,
    pub caustics_enabled: bool,
    pub caustics_intensity: f32,
    pub foam_enabled: bool,
    pub foam_threshold: f32,

    // Thermal
    pub thermal_enabled: bool,
    pub thermal_diffusivity: f32,
    pub buoyancy: f32,

    // Detection
    pub auto_detect_water_bodies: bool,
    pub min_river_flow_threshold: f32,
    pub lake_depth_threshold: f32,
    pub waterfall_height_threshold: f32,

    // Emitters
    pub emitter_count: u32,
    pub spawn_rate: f32,
    pub initial_velocity: f32,
}

impl Default for FluidSimParams {
    fn default() -> Self {
        Self {
            enabled: true,
            quality_preset: FluidQualityPreset::Balanced,
            water_body_preset: WaterBodyPreset::CalmLake,

            // Physics
            particle_count: 65536,
            smoothing_radius: 1.0,
            target_density: 12.0,
            pressure_multiplier: 300.0,
            viscosity: 10.0,
            surface_tension: 0.02,
            gravity: -9.8,
            solver_iterations: 4,

            // Flow
            flow_enabled: false,
            flow_speed: 1.0,
            flow_direction: [1.0, 0.0],
            turbulence: 0.1,

            // Rendering
            water_color: [0.2, 0.5, 0.8, 0.9],
            transparency: 0.7,
            refraction_strength: 0.5,
            caustics_enabled: true,
            caustics_intensity: 1.0,
            foam_enabled: true,
            foam_threshold: 0.3,

            // Thermal
            thermal_enabled: false,
            thermal_diffusivity: 0.1,
            buoyancy: 0.0002,

            // Detection
            auto_detect_water_bodies: true,
            min_river_flow_threshold: 500.0,
            lake_depth_threshold: 2.0,
            waterfall_height_threshold: 5.0,

            // Emitters
            emitter_count: 1,
            spawn_rate: 1000.0,
            initial_velocity: 0.0,
        }
    }
}

/// Detected water body information for display
#[derive(Debug, Clone)]
pub struct DetectedWaterBodyInfo {
    pub name: String,
    pub body_type: String,
    pub center: [f32; 3],
    pub volume: f32,
    pub particle_count: u32,
    pub flow_speed: Option<f32>,
    pub selected: bool,
}

/// Statistics for fluid simulation
#[derive(Default, Clone)]
pub struct FluidStats {
    pub active_particles: u32,
    pub emitter_count: u32,
    pub detected_bodies: u32,
    pub simulation_time_ms: f32,
    pub render_time_ms: f32,
}

/// Terrain generation and editing panel
pub struct TerrainPanel {
    /// Terrain generation state
    terrain_state: TerrainState,

    /// Generation parameters
    seed: u64,
    seed_string: String,
    primary_biome: String,
    /// Phase 1.6-F.4.B.3.D.5b: world archetype selector. Replaces the
    /// "Primary Biome" dropdown's role of driving the climate-field
    /// envelope (the dropdown stays for legacy splat-rule biome configs
    /// in `biomes_for_primary`, but no longer controls climate). Default:
    /// Continental Temperate.
    world_archetype_id: astraweave_terrain::world_archetypes::WorldArchetypeId,
    /// Phase 1.6-F.4.B.3.D.5b: editable parameters for the Custom
    /// archetype. Initialized to Continental Temperate values per §1.1.
    /// Sliders surface this struct only when `world_archetype_id ==
    /// Custom`.
    custom_archetype: astraweave_terrain::climate::WorldArchetype,
    chunk_radius: i32,

    /// Noise parameters
    octaves: u32,
    lacunarity: f32,
    persistence: f32,
    base_amplitude: f32,

    // Phase 1.6-F.4.B.3.D.5b: Mountain Drama slider REMOVED per §1.4.
    // The slider was inert since D.3c (no preset to multiply). Per-biome
    // `mountains_amplitude` parameters cover the design space without an
    // extra global knob; if global tuning becomes desirable, it can come
    // back as a Custom-archetype field.

    /// Erosion parameters
    erosion_preset: ErosionPresetType,
    hydraulic_erosion: HydraulicErosionParams,
    thermal_erosion: ThermalErosionParams,
    wind_erosion: WindErosionParams,

    /// Biome blending parameters
    biome_blend: BiomeBlendParams,

    /// Texture splatting parameters
    splat_params: SplatParams,

    /// Water surface level (world Y height)
    pub water_level: f32,

    /// Fluid simulation parameters
    fluid_params: FluidSimParams,
    fluid_stats: FluidStats,
    detected_water_bodies: Vec<DetectedWaterBodyInfo>,
    show_fluid_debug: bool,

    /// UI state
    auto_regenerate: bool,
    show_advanced: bool,
    last_generation_time_ms: f32,
    generation_stats: GenerationStats,
    /// True while background terrain generation is in progress
    generating: bool,
    /// Receiver for completed background terrain generation
    gen_receiver: Option<std::sync::mpsc::Receiver<TerrainGenResult>>,
    /// Receiver for deferred scatter placements (generated after terrain is sent)
    scatter_receiver:
        Option<std::sync::mpsc::Receiver<Vec<crate::terrain_integration::ScatterPlacement>>>,

    /// Brush settings for voxel editing
    brush_enabled: bool,
    brush_mode: BrushMode,
    brush_radius: f32,
    brush_strength: f32,
    brush_falloff: FalloffCurve,
    /// For Flatten brush: captured target height on first click (None = not yet captured)
    flatten_target_height: Option<f32>,
    /// Noise brush scale (world-space frequency)
    noise_scale: f32,
    selected_material: usize,
    /// Lazy-loaded material thumbnail textures (64x64 each)
    material_thumbnails: Vec<Option<egui::TextureHandle>>,
    /// Whether thumbnails have been loaded yet
    thumbnails_loaded: bool,
    /// Brush world position (X, Z) for applying brush strokes
    brush_pos_x: f32,
    brush_pos_z: f32,

    /// Pre-computed scatter placements from background thread
    cached_scatter_placements: Vec<crate::terrain_integration::ScatterPlacement>,

    /// Height stats from last terrain generation: (min, max, avg)
    last_height_stats: (f32, f32, f32),

    /// Action queue
    pending_actions: Vec<TerrainAction>,
}

/// Result sent back from the background terrain generation thread
struct TerrainGenResult {
    terrain_state: TerrainState,
    chunk_count: usize,
    elapsed_ms: f32,
    scatter_placements: Vec<crate::terrain_integration::ScatterPlacement>,
    /// (min_height, max_height, avg_height)
    height_stats: (f32, f32, f32),
}

/// Brush modes for terrain sculpting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BrushMode {
    Sculpt,
    Lower,
    Smooth,
    Flatten,
    Paint,
    Erode,
    Noise,
    ZoneBlend,
}

/// Falloff curve for terrain brush strength attenuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FalloffCurve {
    Linear,
    Smooth,
    Gaussian,
}

impl FalloffCurve {
    /// Compute falloff factor for a normalized distance (0.0 = center, 1.0 = edge).
    pub fn eval(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            FalloffCurve::Linear => 1.0 - t,
            FalloffCurve::Smooth => {
                let s = 1.0 - t;
                s * s * (3.0 - 2.0 * s) // smoothstep
            }
            FalloffCurve::Gaussian => {
                // Gaussian with sigma ~0.33 so it reaches ~0 at t=1
                (-4.5 * t * t).exp()
            }
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            FalloffCurve::Linear => "Linear",
            FalloffCurve::Smooth => "Smooth",
            FalloffCurve::Gaussian => "Gaussian",
        }
    }
}

impl std::fmt::Display for BrushMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl BrushMode {
    pub fn name(&self) -> &'static str {
        match self {
            BrushMode::Sculpt => "Sculpt",
            BrushMode::Lower => "Lower",
            BrushMode::Smooth => "Smooth",
            BrushMode::Flatten => "Flatten",
            BrushMode::Paint => "Paint",
            BrushMode::Erode => "Erode",
            BrushMode::Noise => "Noise",
            BrushMode::ZoneBlend => "Zone Blend",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BrushMode::Sculpt => "🏔️",
            BrushMode::Lower => "⬇️",
            BrushMode::Smooth => "〰️",
            BrushMode::Flatten => "-",
            BrushMode::Paint => "🖌️",
            BrushMode::Erode => "💧",
            BrushMode::Noise => "🌊",
            BrushMode::ZoneBlend => "🔀",
        }
    }

    pub fn all() -> &'static [BrushMode] {
        &[
            BrushMode::Sculpt,
            BrushMode::Lower,
            BrushMode::Smooth,
            BrushMode::Flatten,
            BrushMode::Paint,
            BrushMode::Erode,
            BrushMode::Noise,
            BrushMode::ZoneBlend,
        ]
    }
}

/// Actions that can be performed on the terrain panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TerrainAction {
    /// Generate terrain with current settings
    Generate,
    /// Regenerate terrain with a new random seed
    RandomizeSeed,
    /// Set a specific seed value
    SetSeed(u64),
    /// Set the primary biome
    SetBiome(String),
    /// Set the chunk radius
    SetChunkRadius(i32),
    /// Apply an erosion preset
    ApplyErosionPreset(ErosionPresetType),
    /// Run hydraulic erosion
    RunHydraulicErosion,
    /// Run thermal erosion
    RunThermalErosion,
    /// Run wind erosion
    RunWindErosion,
    /// Set brush mode
    SetBrushMode(BrushMode),
    /// Set brush radius
    SetBrushRadius(f32),
    /// Set brush strength
    SetBrushStrength(f32),
    /// Apply brush at position
    ApplyBrush { position: [f32; 3] },
    /// Brush update: only dirty chunks need GPU buffer writes (no full re-upload)
    BrushUpdate,
    /// Toggle fluid simulation
    ToggleFluidSimulation(bool),
    /// Reset fluid simulation
    ResetFluidSimulation,
    /// Export heightmap
    ExportHeightmap { path: String },
    /// Import heightmap
    ImportHeightmap { path: String },
    /// Toggle auto-regenerate
    ToggleAutoRegenerate(bool),
    /// Phase 1.X-Editor-Multi-Tool-Architecture-Sub-phase-3: route active-tool
    /// transition through the dispatcher. Emitted when the brush-mode toggle
    /// flips `brush_enabled` (terrain_panel.rs:1180); drained in
    /// `tab_viewer/mod.rs` and forwarded to main.rs which calls
    /// `dispatcher.set_active_tool(uuid, &mut tool_context)`. `Some(uuid)`
    /// activates this tool; `None` deactivates the active tool.
    SetActiveTool { uuid: Option<Uuid> },
}

#[derive(Default, Clone)]
struct GenerationStats {
    chunks_generated: usize,
    total_vertices: usize,
    total_triangles: usize,
    memory_estimate_mb: f32,
    erosion_time_ms: f32,
    splatmap_time_ms: f32,
    scatter_placements: usize,
}

impl Default for TerrainPanel {
    fn default() -> Self {
        Self {
            terrain_state: TerrainState::new(),
            seed: 12345,
            seed_string: "12345".to_string(),
            primary_biome: "grassland".to_string(),
            // Phase 1.6-F.4.B.3.D.5b: world archetype defaults to
            // Continental Temperate (Veilweaver default).
            world_archetype_id:
                astraweave_terrain::world_archetypes::WorldArchetypeId::default(),
            // Custom archetype starts from Continental Temperate per §1.1.
            custom_archetype: astraweave_terrain::world_archetypes::continental_temperate(),
            // Phase 1.6-F.4.B.2.C: Target B world extent. Radius 10 × 512 WU
            // chunks = 21 × 21 = 441 chunks = ~10.75 km per side = 115.5 km²
            // per plan §2.3 Target B = 10-50 km² bracket matched.
            chunk_radius: 10,
            octaves: 6,
            lacunarity: 2.0,
            persistence: 0.5,
            base_amplitude: 50.0,
            erosion_preset: ErosionPresetType::Mountain,
            hydraulic_erosion: HydraulicErosionParams::default(),
            thermal_erosion: ThermalErosionParams::default(),
            wind_erosion: WindErosionParams::default(),
            biome_blend: BiomeBlendParams::default(),
            splat_params: SplatParams::default(),
            water_level: 0.0,
            fluid_params: FluidSimParams::default(),
            fluid_stats: FluidStats::default(),
            detected_water_bodies: Vec::new(),
            show_fluid_debug: false,
            auto_regenerate: false,
            show_advanced: false,
            last_generation_time_ms: 0.0,
            generation_stats: GenerationStats::default(),
            generating: false,
            gen_receiver: None,
            scatter_receiver: None,
            brush_enabled: false,
            brush_mode: BrushMode::Sculpt,
            brush_radius: 5.0,
            brush_strength: 0.5,
            brush_falloff: FalloffCurve::Smooth,
            flatten_target_height: None,
            noise_scale: 0.05,
            selected_material: 0,
            material_thumbnails: Vec::new(),
            thumbnails_loaded: false,
            brush_pos_x: 0.0,
            brush_pos_z: 0.0,
            cached_scatter_placements: Vec::new(),
            last_height_stats: (0.0, 0.0, 0.0),
            pending_actions: Vec::new(),
        }
    }
}

impl TerrainPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes all pending actions, leaving the queue empty
    pub fn take_actions(&mut self) -> Vec<TerrainAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Get terrain chunks ready for GPU upload
    pub fn get_gpu_chunks(
        &self,
    ) -> Vec<(Vec<crate::terrain_integration::TerrainVertex>, Vec<u32>)> {
        self.terrain_state.get_gpu_chunks()
    }

    /// Take dirty chunk vertex data after a brush stroke for incremental GPU update.
    pub fn take_dirty_chunks(
        &mut self,
    ) -> Vec<(usize, Vec<crate::terrain_integration::TerrainVertex>)> {
        self.terrain_state.take_dirty_chunks()
    }

    /// Returns (min_height, max_height, avg_height) from the last generation.
    pub fn height_stats(&self) -> (f32, f32, f32) {
        self.last_height_stats
    }

    /// Return the currently loaded BiomePack (if any) for texture injection.
    pub fn cached_biome_pack(&self) -> Option<&astraweave_terrain::BiomePack> {
        self.terrain_state.cached_biome_pack()
    }

    /// Returns the current primary biome name (e.g. "mountain", "swamp", "grassland").
    pub fn primary_biome(&self) -> &str {
        &self.primary_biome
    }

    /// Generate scatter placements from the terrain vegetation system
    pub fn generate_scatter_placements(&self) -> Vec<crate::terrain_integration::ScatterPlacement> {
        self.terrain_state.generate_scatter_placements()
    }

    /// Take pre-computed scatter placements (generated on background thread).
    /// Returns the cached placements and clears the cache.
    pub fn take_cached_scatter_placements(
        &mut self,
    ) -> Vec<crate::terrain_integration::ScatterPlacement> {
        let placements = std::mem::take(&mut self.cached_scatter_placements);
        tracing::info!(
            "take_cached_scatter_placements: returning {} placements",
            placements.len()
        );
        placements
    }

    /// Queue an action for later processing
    pub fn queue_action(&mut self, action: TerrainAction) {
        self.pending_actions.push(action);
    }

    /// Get reference to terrain state for rendering
    pub fn terrain_state(&self) -> &TerrainState {
        &self.terrain_state
    }

    /// Get mutable reference to terrain state
    pub fn terrain_state_mut(&mut self) -> &mut TerrainState {
        &mut self.terrain_state
    }

    /// Check if terrain needs regeneration
    pub fn needs_regeneration(&self) -> bool {
        self.terrain_state.is_dirty()
    }

    /// Returns true if a sculpting brush mode is active and terrain exists
    pub fn is_brush_active(&self) -> bool {
        self.brush_enabled && self.terrain_state.has_terrain()
    }

    /// Configure seed, biome, and chunk radius then trigger background generation.
    /// Used by the World Wizard to drive terrain creation from a single action.
    pub fn configure_and_generate(&mut self, seed: u64, biome: &str, chunk_radius: i32) {
        self.seed = seed;
        self.seed_string = seed.to_string();
        self.primary_biome = biome.to_string();
        self.chunk_radius = chunk_radius;
        self.terrain_state.configure(seed, biome);
        self.regenerate_terrain();
    }

    /// Returns true if terrain has been generated (chunks > 0).
    pub fn has_generated(&self) -> bool {
        self.generation_stats.chunks_generated > 0
    }

    /// Apply brush at the given world position using current brush settings.
    /// Called from viewport mouse interaction.
    pub fn apply_brush_at(&mut self, world_x: f32, world_z: f32) {
        self.brush_pos_x = world_x;
        self.brush_pos_z = world_z;

        // Auto-begin stroke on first brush application
        if !self.terrain_state.is_stroking() {
            self.terrain_state.begin_stroke();
        }

        let modified = if self.brush_mode == BrushMode::Paint {
            self.terrain_state.apply_brush_paint_material(
                world_x,
                world_z,
                self.brush_radius,
                self.brush_strength,
                self.selected_material as u32,
                self.brush_falloff,
            )
        } else {
            // Capture flatten target on first click
            if self.brush_mode == BrushMode::Flatten && self.flatten_target_height.is_none() {
                self.flatten_target_height = Some(
                    self.terrain_state
                        .sample_height_at(world_x, world_z)
                        .unwrap_or(0.0),
                );
            }
            self.terrain_state.apply_brush(
                world_x,
                world_z,
                self.brush_radius,
                self.brush_strength,
                self.brush_mode,
                self.brush_falloff,
                self.flatten_target_height,
                self.noise_scale,
            )
        };
        if modified {
            self.pending_actions.push(TerrainAction::BrushUpdate);
        }
    }

    /// Called when the brush stroke ends (mouse released).
    /// Returns undo data if any chunks were modified during the stroke.
    pub fn end_brush_stroke(
        &mut self,
    ) -> Option<Vec<(astraweave_terrain::ChunkId, Vec<f32>, Vec<f32>)>> {
        self.flatten_target_height = None;
        self.terrain_state.end_stroke()
    }

    /// Returns the current brush mode name (for undo descriptions).
    pub fn brush_mode_name(&self) -> &'static str {
        self.brush_mode.name()
    }

    /// Returns the current brush radius.
    pub fn brush_radius(&self) -> f32 {
        self.brush_radius
    }

    /// Returns true if the current brush mode is Paint.
    pub fn is_paint_mode(&self) -> bool {
        self.brush_mode == BrushMode::Paint
    }

    /// Apply a heightmap snapshot from undo/redo.
    pub fn apply_height_snapshot(&mut self, snapshot: &[(astraweave_terrain::ChunkId, Vec<f32>)]) {
        self.terrain_state.apply_height_snapshot(snapshot);
    }

    fn show_generation_section(&mut self, ui: &mut Ui) {
        ui.heading("🏔️ Terrain Generation");
        ui.separator();

        // Seed input
        ui.horizontal(|ui| {
            ui.label("Seed:");
            if ui.text_edit_singleline(&mut self.seed_string).changed() {
                if let Ok(new_seed) = self.seed_string.parse::<u64>() {
                    self.seed = new_seed;
                    self.terrain_state.configure(self.seed, &self.primary_biome);
                }
            }
            if ui.button("[Dice]").on_hover_text("Random seed").clicked() {
                self.seed = rand::random();
                self.seed_string = self.seed.to_string();
                self.terrain_state.configure(self.seed, &self.primary_biome);
            }
        });

        // Phase 1.6-F.4.B.3.D.5b: World Archetype dropdown.
        //
        // Replaces the legacy "Primary Biome" dropdown's terrain-shaping
        // role. Each archetype is a climate envelope (mean + variance for
        // temperature / moisture / continentalness, plus latitude
        // strength) — biomes emerge per-vertex from the climate field
        // shaped by the archetype, not from a single "this world is
        // X biome" assignment.
        ui.horizontal(|ui| {
            ui.label("World Archetype:")
                .on_hover_text(self.world_archetype_id.description());
            let current_id = self.world_archetype_id;
            let mut new_id = current_id;
            egui::ComboBox::from_id_salt("world_archetype")
                .selected_text(current_id.display_name())
                .show_ui(ui, |ui| {
                    for &id in
                        astraweave_terrain::world_archetypes::WorldArchetypeId::all()
                    {
                        ui.selectable_value(&mut new_id, id, id.display_name())
                            .on_hover_text(id.description());
                    }
                });
            if new_id != current_id {
                self.world_archetype_id = new_id;
                let archetype = if new_id
                    == astraweave_terrain::world_archetypes::WorldArchetypeId::Custom
                {
                    self.custom_archetype.clone()
                } else {
                    new_id.default_archetype()
                };
                self.terrain_state.set_world_archetype(archetype);
                self.terrain_state.configure(self.seed, &self.primary_biome);
                self.regenerate_terrain();
            }
        });

        // Custom archetype parameter sliders (visible only when Custom is
        // selected). Per §1.3 plan: the user can directly tune the climate
        // envelope.
        if self.world_archetype_id
            == astraweave_terrain::world_archetypes::WorldArchetypeId::Custom
        {
            let mut changed = false;
            ui.indent("custom_archetype_params", |ui| {
                ui.label("Custom climate envelope:");
                let prev = self.custom_archetype.clone();

                ui.horizontal(|ui| {
                    ui.label("Temp mean (°C):");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.custom_archetype.temperature_mean_c,
                            -30.0..=40.0,
                        ))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Temp variance (±°C):");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.custom_archetype.temperature_variance_c,
                            0.0..=20.0,
                        ))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Latitude drop (°C):");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.custom_archetype.latitude_temperature_drop_c,
                            0.0..=30.0,
                        ))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Moisture mean (mm):");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.custom_archetype.moisture_mean_mm,
                            0.0..=4000.0,
                        ))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Moisture variance (±mm):");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.custom_archetype.moisture_variance_mm,
                            0.0..=1500.0,
                        ))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Continentalness mean:");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.custom_archetype.continentalness_mean,
                            0.0..=1.0,
                        ))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Continentalness variance:");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.custom_archetype.continentalness_variance,
                            0.0..=0.5,
                        ))
                        .changed();
                });

                if changed
                    && self.custom_archetype.validate().is_err()
                {
                    // Validation failed (slider produced out-of-range
                    // value somehow). Revert; sliders are bounded so this
                    // branch should be unreachable, but defend against
                    // future range changes.
                    self.custom_archetype = prev;
                }
            });
            if changed {
                self.terrain_state.set_world_archetype(self.custom_archetype.clone());
                self.regenerate_terrain();
            }
        }

        // Chunk radius slider
        ui.horizontal(|ui| {
            ui.label("Chunk Radius:");
            if ui
                .add(egui::Slider::new(&mut self.chunk_radius, 1..=12))
                .changed()
            {
                self.terrain_state.configure(self.seed, &self.primary_biome);
            }
            ui.label(format!("({} chunks)", (self.chunk_radius * 2 + 1).pow(2)));
        });

        ui.add_space(10.0);

        // Poll for completed background generation
        self.poll_generation();

        // Generate button
        if self.generating {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Generating terrain...");
            });
        } else {
            let generate_text = if self.terrain_state.is_dirty() {
                RichText::new("Generate Terrain").color(Color32::YELLOW)
            } else {
                RichText::new("Generate Terrain")
            };

            if ui.button(generate_text).clicked() {
                self.regenerate_terrain();
            }
        }

        ui.checkbox(&mut self.auto_regenerate, "Auto-regenerate on change");

        // Stats
        if self.generation_stats.chunks_generated > 0 {
            ui.add_space(5.0);
            ui.group(|ui| {
                ui.label(RichText::new("Generation Stats").strong());
                ui.label(format!(
                    "Chunks: {}",
                    self.generation_stats.chunks_generated
                ));
                ui.label(format!(
                    "Vertices: {}",
                    self.generation_stats.total_vertices
                ));
                ui.label(format!(
                    "Triangles: {}",
                    self.generation_stats.total_triangles
                ));
                ui.label(format!(
                    "Memory: {:.2} MB",
                    self.generation_stats.memory_estimate_mb
                ));
                ui.label(format!("Time: {:.1} ms", self.last_generation_time_ms));
                if self.generation_stats.scatter_placements > 0 {
                    ui.label(format!(
                        "Scatter: {} placements",
                        self.generation_stats.scatter_placements
                    ));
                }
            });
        }
    }

    fn show_noise_section(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ui.collapsing("🎛️ Noise Parameters", |ui| {
            let mut changed = false;

            ui.horizontal(|ui| {
                ui.label("Octaves:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.octaves, 1..=8))
                    .changed();
            });

            ui.horizontal(|ui| {
                ui.label("Lacunarity:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.lacunarity, 1.5..=3.0))
                    .changed();
            });

            ui.horizontal(|ui| {
                ui.label("Persistence:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.persistence, 0.1..=0.9))
                    .changed();
            });

            ui.horizontal(|ui| {
                ui.label("Amplitude:");
                changed |= ui
                    .add(egui::Slider::new(&mut self.base_amplitude, 10.0..=200.0))
                    .changed();
            });

            // Phase 1.6-F.4.B.3.D.5b: Mountain Drama slider REMOVED per
            // §1.4. The slider was inert since D.3c (no preset to
            // multiply); D.5 ships per-biome `mountains_amplitude`
            // values inside `BiomeParameters` which cover the design
            // space without an extra global knob. If global tuning
            // becomes desirable later, it can come back as a Custom
            // archetype field.

            if changed {
                self.terrain_state.configure(self.seed, &self.primary_biome);
                if self.auto_regenerate {
                    self.regenerate_terrain();
                }
            }

            if ui.button("Reset to Defaults").clicked() {
                self.octaves = 6;
                self.lacunarity = 2.0;
                self.persistence = 0.5;
                self.base_amplitude = 50.0;
                self.terrain_state.configure(self.seed, &self.primary_biome);
            }
        });
    }

    fn show_brush_section(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ui.collapsing("🖌️ Sculpting Brushes", |ui| {
            // On/off toggle for brush tool
            let toggle_label = if self.brush_enabled {
                "🔴 Brush Active"
            } else {
                "⚪ Brush Inactive"
            };
            if ui
                .selectable_label(self.brush_enabled, toggle_label)
                .clicked()
            {
                self.brush_enabled = !self.brush_enabled;
                // Phase 1.X-Editor-Multi-Tool-Architecture-Sub-phase-3:
                // route active-tool transition through dispatcher per
                // campaign doc §5.2. Drained in tab_viewer + forwarded
                // to main.rs which calls dispatcher.set_active_tool.
                let new_active = if self.brush_enabled {
                    Some(TERRAIN_PANEL_UUID)
                } else {
                    None
                };
                self.pending_actions
                    .push(TerrainAction::SetActiveTool { uuid: new_active });
            }
            if self.brush_enabled {
                ui.label(
                    RichText::new("Left-click on terrain to sculpt/paint")
                        .small()
                        .italics()
                        .color(egui::Color32::from_rgb(200, 140, 140)),
                );
            }
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.selectable_value(&mut self.brush_mode, BrushMode::Sculpt, "Sculpt");
                ui.selectable_value(&mut self.brush_mode, BrushMode::Lower, "Lower");
                ui.selectable_value(&mut self.brush_mode, BrushMode::Smooth, "Smooth");
                ui.selectable_value(&mut self.brush_mode, BrushMode::Flatten, "Flatten");
                ui.selectable_value(&mut self.brush_mode, BrushMode::Paint, "Paint");
                ui.selectable_value(&mut self.brush_mode, BrushMode::Erode, "Erode");
                ui.selectable_value(&mut self.brush_mode, BrushMode::Noise, "Noise");
                ui.selectable_value(&mut self.brush_mode, BrushMode::ZoneBlend, "Zone Blend");
            });

            ui.horizontal(|ui| {
                ui.label("Radius:");
                ui.add(egui::Slider::new(&mut self.brush_radius, 1.0..=50.0));
            });

            ui.horizontal(|ui| {
                ui.label("Strength:");
                ui.add(egui::Slider::new(&mut self.brush_strength, 0.0..=1.0));
            });

            ui.horizontal(|ui| {
                ui.label("Falloff:");
                ui.selectable_value(&mut self.brush_falloff, FalloffCurve::Linear, "Linear");
                ui.selectable_value(&mut self.brush_falloff, FalloffCurve::Smooth, "Smooth");
                ui.selectable_value(&mut self.brush_falloff, FalloffCurve::Gaussian, "Gaussian");
            });

            if self.brush_mode == BrushMode::Noise {
                ui.horizontal(|ui| {
                    ui.label("Noise Scale:");
                    ui.add(egui::Slider::new(&mut self.noise_scale, 0.005..=0.5).logarithmic(true));
                });
            }

            if self.brush_mode == BrushMode::Paint {
                self.ensure_thumbnails_loaded(ui.ctx());
                let display_names = &crate::viewport::types::MATERIAL_DISPLAY_NAMES;

                ui.label("Material:");
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        let thumb_size = 48.0;
                        let cols = 3;
                        egui::Grid::new("material_grid")
                            .spacing([4.0, 4.0])
                            .show(ui, |ui| {
                                for (i, name) in display_names.iter().enumerate() {
                                    let is_selected = self.selected_material == i;

                                    let response = ui.vertical(|ui| {
                                        // Thumbnail or colored fallback
                                        let thumb = self
                                            .material_thumbnails
                                            .get(i)
                                            .and_then(|t| t.as_ref());
                                        if let Some(tex) = thumb {
                                            let img = egui::Image::new(tex).fit_to_exact_size(
                                                egui::vec2(thumb_size, thumb_size),
                                            );
                                            ui.add(img);
                                        } else {
                                            // Fallback: colored rectangle
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(thumb_size, thumb_size),
                                                egui::Sense::hover(),
                                            );
                                            let hue = (i as f32 / 22.0) * 360.0;
                                            ui.painter().rect_filled(
                                                rect,
                                                2.0,
                                                egui::Color32::from_rgb(
                                                    (100.0 + hue * 0.4) as u8,
                                                    (80.0 + hue * 0.3) as u8,
                                                    (60.0 + hue * 0.2) as u8,
                                                ),
                                            );
                                        }
                                        ui.label(RichText::new(*name).small().strong());
                                    });

                                    // Highlight selected
                                    if is_selected {
                                        ui.painter().rect_stroke(
                                            response.response.rect,
                                            4.0,
                                            egui::Stroke::new(
                                                2.0,
                                                egui::Color32::from_rgb(100, 200, 255),
                                            ),
                                            egui::StrokeKind::Outside,
                                        );
                                    }

                                    if response.response.interact(egui::Sense::click()).clicked() {
                                        self.selected_material = i;
                                    }

                                    if (i + 1) % cols == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            }

            if self.brush_mode == BrushMode::Erode {
                ui.label(
                    RichText::new("Applies localized hydraulic erosion")
                        .small()
                        .italics(),
                );
            }

            ui.label(
                RichText::new("💡 Click or drag in viewport to apply brush")
                    .small()
                    .italics()
                    .color(egui::Color32::from_rgb(130, 170, 220)),
            );

            ui.add_space(5.0);

            // Brush position input
            ui.horizontal(|ui| {
                ui.label("Position X:");
                ui.add(egui::DragValue::new(&mut self.brush_pos_x).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Position Z:");
                ui.add(egui::DragValue::new(&mut self.brush_pos_z).speed(1.0));
            });

            ui.add_space(5.0);

            let has_terrain = self.terrain_state.has_terrain();
            let apply_text = if has_terrain {
                "Apply Brush"
            } else {
                "Generate terrain first"
            };

            if ui
                .add_enabled(has_terrain, egui::Button::new(apply_text))
                .clicked()
            {
                let modified = if self.brush_mode == BrushMode::Paint {
                    // Paint mode: directly modify vertex material slots
                    self.terrain_state.apply_brush_paint_material(
                        self.brush_pos_x,
                        self.brush_pos_z,
                        self.brush_radius,
                        self.brush_strength,
                        self.selected_material as u32,
                        self.brush_falloff,
                    )
                } else {
                    self.terrain_state.apply_brush(
                        self.brush_pos_x,
                        self.brush_pos_z,
                        self.brush_radius,
                        self.brush_strength,
                        self.brush_mode,
                        self.brush_falloff,
                        self.flatten_target_height,
                        self.noise_scale,
                    )
                };
                if modified {
                    self.pending_actions.push(TerrainAction::BrushUpdate);
                }
            }
        });
    }

    fn show_erosion_section(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ui.collapsing("[Wave] Erosion Simulation", |ui| {
            // Preset selection
            ui.horizontal(|ui| {
                ui.label("Preset:");
                egui::ComboBox::from_id_salt("erosion_preset")
                    .selected_text(self.erosion_preset.name())
                    .show_ui(ui, |ui| {
                        for preset in ErosionPresetType::all() {
                            if ui
                                .selectable_value(&mut self.erosion_preset, *preset, preset.name())
                                .clicked()
                            {
                                self.apply_erosion_preset(*preset);
                            }
                        }
                    });
            });

            ui.separator();

            // Hydraulic erosion
            ui.collapsing("💧 Hydraulic Erosion", |ui| {
                ui.checkbox(&mut self.hydraulic_erosion.enabled, "Enabled");

                if self.hydraulic_erosion.enabled {
                    ui.horizontal(|ui| {
                        ui.label("Iterations:");
                        ui.add(
                            egui::Slider::new(
                                &mut self.hydraulic_erosion.iterations,
                                1000..=200000,
                            )
                            .logarithmic(true),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Inertia:");
                        ui.add(egui::Slider::new(
                            &mut self.hydraulic_erosion.inertia,
                            0.0..=1.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Capacity:");
                        ui.add(egui::Slider::new(
                            &mut self.hydraulic_erosion.capacity,
                            1.0..=20.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Deposition:");
                        ui.add(egui::Slider::new(
                            &mut self.hydraulic_erosion.deposition,
                            0.0..=1.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Erosion Rate:");
                        ui.add(egui::Slider::new(
                            &mut self.hydraulic_erosion.erosion,
                            0.0..=1.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Evaporation:");
                        ui.add(egui::Slider::new(
                            &mut self.hydraulic_erosion.evaporation,
                            0.0..=0.1,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Min Slope:");
                        ui.add(
                            egui::Slider::new(&mut self.hydraulic_erosion.min_slope, 0.001..=0.1)
                                .logarithmic(true),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Gravity:");
                        ui.add(egui::Slider::new(
                            &mut self.hydraulic_erosion.gravity,
                            1.0..=20.0,
                        ));
                    });
                }
            });

            // Thermal erosion
            ui.collapsing("[Fire] Thermal Erosion", |ui| {
                ui.checkbox(&mut self.thermal_erosion.enabled, "Enabled");

                if self.thermal_erosion.enabled {
                    ui.horizontal(|ui| {
                        ui.label("Iterations:");
                        ui.add(egui::Slider::new(
                            &mut self.thermal_erosion.iterations,
                            1..=200,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Talus Angle (°):");
                        ui.add(egui::Slider::new(
                            &mut self.thermal_erosion.talus_angle,
                            20.0..=60.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Erosion Rate:");
                        ui.add(egui::Slider::new(
                            &mut self.thermal_erosion.erosion_rate,
                            0.0..=1.0,
                        ));
                    });
                }
            });

            // Wind erosion
            ui.collapsing("[Dash] Wind Erosion", |ui| {
                ui.checkbox(&mut self.wind_erosion.enabled, "Enabled");

                if self.wind_erosion.enabled {
                    ui.horizontal(|ui| {
                        ui.label("Iterations:");
                        ui.add(egui::Slider::new(
                            &mut self.wind_erosion.iterations,
                            1..=100,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Direction X:");
                        ui.add(egui::Slider::new(
                            &mut self.wind_erosion.wind_direction[0],
                            -1.0..=1.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Direction Y:");
                        ui.add(egui::Slider::new(
                            &mut self.wind_erosion.wind_direction[1],
                            -1.0..=1.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Strength:");
                        ui.add(egui::Slider::new(
                            &mut self.wind_erosion.wind_strength,
                            0.0..=1.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Suspension:");
                        ui.add(egui::Slider::new(
                            &mut self.wind_erosion.suspension,
                            0.0..=1.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Abrasion:");
                        ui.add(egui::Slider::new(
                            &mut self.wind_erosion.abrasion,
                            0.0..=1.0,
                        ));
                    });
                }
            });

            ui.add_space(5.0);

            // Apply erosion button
            if ui
                .button(RichText::new("Apply Erosion").color(Color32::LIGHT_BLUE))
                .clicked()
            {
                self.apply_erosion();
            }

            if self.generation_stats.erosion_time_ms > 0.0 {
                ui.label(format!(
                    "Last erosion: {:.1} ms",
                    self.generation_stats.erosion_time_ms
                ));
            }
        });
    }

    fn show_biome_blend_section(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ui.collapsing("[Glb] Biome Blending", |ui| {
            ui.checkbox(&mut self.biome_blend.enabled, "Enable Biome Blending");

            if self.biome_blend.enabled {
                // Secondary biome
                ui.horizontal(|ui| {
                    ui.label("Secondary Biome:");
                    let options = cached_biome_options();
                    egui::ComboBox::from_id_salt("secondary_biome")
                        .selected_text(&self.biome_blend.secondary_biome)
                        .show_ui(ui, |ui| {
                            for opt in &options {
                                ui.selectable_value(
                                    &mut self.biome_blend.secondary_biome,
                                    opt.value.clone(),
                                    &opt.display,
                                );
                            }
                        });
                });

                // Tertiary biome
                ui.horizontal(|ui| {
                    ui.label("Tertiary Biome:");
                    let options = cached_biome_options();
                    egui::ComboBox::from_id_salt("tertiary_biome")
                        .selected_text(&self.biome_blend.tertiary_biome)
                        .show_ui(ui, |ui| {
                            for opt in &options {
                                ui.selectable_value(
                                    &mut self.biome_blend.tertiary_biome,
                                    opt.value.clone(),
                                    &opt.display,
                                );
                            }
                        });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Blend Radius:");
                    ui.add(
                        egui::Slider::new(&mut self.biome_blend.blend_radius, 4.0..=128.0)
                            .logarithmic(true),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Falloff Power:");
                    ui.add(egui::Slider::new(
                        &mut self.biome_blend.falloff_power,
                        0.5..=4.0,
                    ));
                });

                ui.horizontal(|ui| {
                    ui.label("Noise Influence:");
                    ui.add(egui::Slider::new(
                        &mut self.biome_blend.noise_influence,
                        0.0..=1.0,
                    ));
                });

                ui.checkbox(
                    &mut self.biome_blend.show_blend_preview,
                    "Show Blend Preview",
                );

                if self.biome_blend.show_blend_preview {
                    ui.group(|ui| {
                        ui.label(RichText::new("Blend Preview").strong());
                        // Preview visualization would be rendered in viewport
                        ui.label("Preview overlay enabled in viewport");

                        // Color legend
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(100, 200, 100), "■");
                            ui.label(&self.primary_biome);
                        });
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(200, 180, 100), "■");
                            ui.label(&self.biome_blend.secondary_biome);
                        });
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(150, 150, 180), "■");
                            ui.label(&self.biome_blend.tertiary_biome);
                        });
                    });
                }
            }
        });
    }

    fn show_splatting_section(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ui.collapsing("Texture Splatting", |ui| {
            ui.checkbox(&mut self.splat_params.enabled, "Enable Texture Splatting");

            if self.splat_params.enabled {
                ui.separator();
                ui.label(RichText::new("Material Rules").strong());

                // Grass rules
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(100, 180, 80), "[Leaf]");
                        ui.label("Grass");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height Range:");
                        ui.add(
                            egui::DragValue::new(&mut self.splat_params.grass_height_min)
                                .speed(0.01)
                                .prefix("min: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.splat_params.grass_height_max)
                                .speed(0.01)
                                .prefix("max: "),
                        );
                    });
                });

                // Rock rules
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(120, 110, 100), "[Rock]");
                        ui.label("Rock");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Slope Threshold:");
                        ui.add(egui::Slider::new(
                            &mut self.splat_params.rock_slope_threshold,
                            0.0..=1.0,
                        ));
                    });
                });

                // Snow rules
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::WHITE, "❄️");
                        ui.label("Snow");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height Threshold:");
                        ui.add(egui::Slider::new(
                            &mut self.splat_params.snow_height_threshold,
                            0.0..=1.0,
                        ));
                    });
                });

                // Sand rules
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(220, 200, 150), "🏖️");
                        ui.label("Sand");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Max Height:");
                        ui.add(egui::Slider::new(
                            &mut self.splat_params.sand_height_max,
                            0.0..=0.5,
                        ));
                    });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Triplanar Sharpness:");
                    ui.add(egui::Slider::new(
                        &mut self.splat_params.triplanar_sharpness,
                        1.0..=16.0,
                    ));
                });

                ui.checkbox(
                    &mut self.splat_params.show_splat_preview,
                    "Show Splat Preview",
                );

                if self.splat_params.show_splat_preview {
                    ui.label(
                        RichText::new("Splatmap visualization enabled in viewport")
                            .small()
                            .italics(),
                    );
                }

                ui.add_space(5.0);

                if ui.button(RichText::new("Regenerate Splatmaps")).clicked() {
                    self.regenerate_splatmaps();
                }

                if self.generation_stats.splatmap_time_ms > 0.0 {
                    ui.label(format!(
                        "Last splatmap: {:.1} ms",
                        self.generation_stats.splatmap_time_ms
                    ));
                }
            }
        });
    }

    fn apply_erosion_preset(&mut self, preset: ErosionPresetType) {
        match preset {
            ErosionPresetType::Custom => {
                // Don't modify anything for custom
            }
            ErosionPresetType::Desert => {
                self.hydraulic_erosion.enabled = false;
                self.thermal_erosion.enabled = true;
                self.thermal_erosion.iterations = 100;
                self.thermal_erosion.talus_angle = 35.0;
                self.wind_erosion.enabled = true;
                self.wind_erosion.iterations = 50;
                self.wind_erosion.wind_strength = 0.7;
            }
            ErosionPresetType::Mountain => {
                self.hydraulic_erosion.enabled = true;
                self.hydraulic_erosion.iterations = 100000;
                self.hydraulic_erosion.capacity = 10.0;
                self.thermal_erosion.enabled = true;
                self.thermal_erosion.iterations = 80;
                self.thermal_erosion.talus_angle = 45.0;
                self.wind_erosion.enabled = false;
            }
            ErosionPresetType::Coastal => {
                self.hydraulic_erosion.enabled = true;
                self.hydraulic_erosion.iterations = 80000;
                self.hydraulic_erosion.capacity = 6.0;
                self.hydraulic_erosion.erosion = 0.7;
                self.thermal_erosion.enabled = false;
                self.wind_erosion.enabled = true;
                self.wind_erosion.wind_strength = 0.4;
            }
            ErosionPresetType::Alpine => {
                self.hydraulic_erosion.enabled = true;
                self.hydraulic_erosion.iterations = 150000;
                self.hydraulic_erosion.capacity = 12.0;
                self.thermal_erosion.enabled = true;
                self.thermal_erosion.iterations = 60;
                self.thermal_erosion.talus_angle = 50.0;
                self.wind_erosion.enabled = false;
            }
            ErosionPresetType::Canyon => {
                self.hydraulic_erosion.enabled = true;
                self.hydraulic_erosion.iterations = 200000;
                self.hydraulic_erosion.capacity = 15.0;
                self.hydraulic_erosion.erosion = 0.8;
                self.thermal_erosion.enabled = true;
                self.thermal_erosion.iterations = 40;
                self.thermal_erosion.talus_angle = 55.0;
                self.wind_erosion.enabled = false;
            }
        }
    }

    fn apply_erosion(&mut self) {
        let start = std::time::Instant::now();

        // In a real implementation, this would call the erosion systems
        // For now, just track the timing
        tracing::info!("Applying erosion with preset: {:?}", self.erosion_preset);
        tracing::info!(
            "Hydraulic: enabled={}, iterations={}",
            self.hydraulic_erosion.enabled,
            self.hydraulic_erosion.iterations
        );
        tracing::info!(
            "Thermal: enabled={}, iterations={}",
            self.thermal_erosion.enabled,
            self.thermal_erosion.iterations
        );
        tracing::info!(
            "Wind: enabled={}, iterations={}",
            self.wind_erosion.enabled,
            self.wind_erosion.iterations
        );

        self.generation_stats.erosion_time_ms = start.elapsed().as_secs_f32() * 1000.0;
    }

    fn regenerate_splatmaps(&mut self) {
        let start = std::time::Instant::now();

        self.terrain_state.regenerate_splatmaps(
            self.splat_params.rock_slope_threshold * 45.0, // normalize 0-1 → degrees
            self.splat_params.snow_height_threshold,
            self.splat_params.sand_height_max * 200.0, // normalize 0-1 → world height
        );

        // Queue a terrain update so dirty chunks get re-uploaded to GPU
        self.pending_actions.push(TerrainAction::BrushUpdate);

        self.generation_stats.splatmap_time_ms = start.elapsed().as_secs_f32() * 1000.0;
        tracing::info!(
            "Splatmaps regenerated in {:.1}ms with params: {:?}",
            self.generation_stats.splatmap_time_ms,
            self.splat_params,
        );
    }

    fn material_name(id: usize) -> &'static str {
        match id {
            0 => "Grass",
            1 => "Sand",
            2 => "Rock",
            3 => "Snow",
            4 => "Dirt",
            5 => "Mud",
            6 => "Gravel",
            7 => "Clay",
            _ => "Unknown",
        }
    }

    fn biome_paint_name(id: usize) -> &'static str {
        match id {
            0 => "Grassland",
            1 => "Desert",
            2 => "Forest",
            3 => "Mountain",
            4 => "Tundra",
            5 => "Swamp",
            6 => "Beach",
            7 => "River",
            _ => "Grassland",
        }
    }

    /// On first Paint mode display, load 64×64 thumbnails from assets/materials/.
    fn ensure_thumbnails_loaded(&mut self, ctx: &egui::Context) {
        if self.thumbnails_loaded {
            return;
        }
        self.thumbnails_loaded = true;

        let assets_dir = crate::viewport::types::find_assets_dir();
        let names = &crate::viewport::types::MATERIAL_NAMES;

        self.material_thumbnails = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let path = assets_dir.join(format!("materials/{name}.png"));
                match image::open(&path) {
                    Ok(img) => {
                        let thumb = img.resize_exact(64, 64, image::imageops::FilterType::Triangle);
                        let rgba = thumb.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                        Some(ctx.load_texture(
                            format!("mat_thumb_{i}"),
                            color_image,
                            egui::TextureOptions::LINEAR,
                        ))
                    }
                    Err(_) => None,
                }
            })
            .collect();
    }

    // Phase 1.6-F.4.B.3.D.3c: `noise_preset_for_biome` REMOVED. Replaced
    // by per-vertex BiomeId lookup in WorldGenerator::generate_chunk_with_climate
    // (terrain crate). Editor terrain generation no longer mutates the
    // global noise config based on the "Primary Biome" dropdown — every
    // vertex looks up its own biome from the climate field.
    // D.5 will replace the dropdown with a "World Archetype" selector that
    // drives the climate-field envelope per-vertex.

    fn regenerate_terrain(&mut self) {
        if self.generating {
            return;
        }

        tracing::info!(
            "regenerate_terrain: biome='{}', seed={}, chunk_radius={}, base_amp={}",
            self.primary_biome,
            self.seed,
            self.chunk_radius,
            self.base_amplitude
        );

        // Prepare a fresh TerrainState with current config on the background thread.
        //
        // Phase 1.6-F.4.B.3.D.3c: legacy `apply_biome_noise_preset` REMOVED.
        // The new climate-field architecture (D.1/D.2/D.3a/D.3b) drives
        // per-vertex biome assignment inside
        // `WorldGenerator::generate_chunk_with_climate`. The editor no
        // longer applies a single per-biome preset to the entire world;
        // each vertex looks up its own `BiomeId` and per-biome
        // `BiomeParameters`.
        //
        // UI slider values still flow through `set_noise_params` (octaves,
        // lacunarity, persistence, base_amplitude) so the user can still
        // tune the global base-noise character. D.5 will replace these
        // sliders with archetype-driven controls.
        //
        // Phase 1.6-F.4.B.3.D.5b: Mountain Drama slider REMOVED;
        // per-biome `mountains_amplitude` parameters in `BiomeParameters`
        // cover the global-amplitude design space. World archetype
        // selection drives the climate envelope which determines per-vertex
        // biome distribution → per-biome parameter selection.
        let mut state = TerrainState::new();
        state.configure(self.seed, &self.primary_biome);
        state.set_noise_params(
            self.octaves as usize,
            self.lacunarity as f64,
            self.persistence as f64,
            self.base_amplitude,
        );
        // Apply the selected world archetype to the climate config.
        let archetype = if self.world_archetype_id
            == astraweave_terrain::world_archetypes::WorldArchetypeId::Custom
        {
            self.custom_archetype.clone()
        } else {
            self.world_archetype_id.default_archetype()
        };
        state.set_world_archetype(archetype);

        tracing::info!(
            "regenerate_terrain: climate-field architecture active (D.1+D.2+D.3); \
             primary_biome dropdown is informational until D.5 wires archetypes"
        );

        let chunk_radius = self.chunk_radius;

        let (tx, rx) = std::sync::mpsc::channel();
        let (scatter_tx, scatter_rx) = std::sync::mpsc::channel();
        self.generating = true;
        self.gen_receiver = Some(rx);
        self.scatter_receiver = Some(scatter_rx);

        std::thread::spawn(move || {
            // Move scatter_tx into this thread so it drops when we exit
            // (receiver sees Disconnected and cleans up).
            let _scatter_tx = scatter_tx;

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let start = std::time::Instant::now();
                match state.generate_terrain(chunk_radius) {
                    Ok(count) => {
                        let height_stats = state.height_stats();
                        let terrain_ms = start.elapsed().as_secs_f32() * 1000.0;

                        // Generate scatter placements while we still own state
                        let scatter_start = std::time::Instant::now();
                        let scatter_placements = state.generate_scatter_placements();
                        let scatter_ms = scatter_start.elapsed().as_secs_f32() * 1000.0;
                        let scatter_count = scatter_placements.len();

                        let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;
                        tracing::info!(
                            "Terrain gen OK: {count} chunks, {scatter_count} scatter placements, heights=({:.1}, {:.1}, {:.1}), terrain={:.0}ms scatter={:.0}ms total={:.0}ms",
                            height_stats.0, height_stats.1, height_stats.2,
                            terrain_ms, scatter_ms, elapsed_ms,
                        );

                        let _ = tx.send(TerrainGenResult {
                            terrain_state: state,
                            chunk_count: count,
                            elapsed_ms,
                            scatter_placements,
                            height_stats,
                        });
                    }
                    Err(e) => {
                        tracing::error!("Terrain generation FAILED for biome: {}", e);
                    }
                }
            }));
            if let Err(panic) = result {
                let msg = panic
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
                    .or_else(|| panic.downcast_ref::<&str>().copied())
                    .unwrap_or("unknown panic");
                tracing::error!("Terrain generation thread PANICKED: {msg}");
            }
        });
    }

    /// Check if the background generation thread has finished and apply results.
    pub fn poll_generation(&mut self) {
        if let Some(rx) = &self.gen_receiver {
            match rx.try_recv() {
                Ok(result) => {
                    let vertex_count = result.terrain_state.total_vertex_count();
                    let triangle_count = result.terrain_state.total_triangle_count();

                    self.terrain_state = result.terrain_state;
                    self.cached_scatter_placements = result.scatter_placements;
                    self.last_height_stats = result.height_stats;
                    self.last_generation_time_ms = result.elapsed_ms;
                    self.generation_stats = GenerationStats {
                        chunks_generated: result.chunk_count,
                        total_vertices: vertex_count,
                        total_triangles: triangle_count,
                        memory_estimate_mb: (vertex_count
                            * std::mem::size_of::<crate::terrain_integration::TerrainVertex>())
                            as f32
                            / (1024.0 * 1024.0),
                        erosion_time_ms: 0.0,
                        splatmap_time_ms: 0.0,
                        scatter_placements: self.cached_scatter_placements.len(),
                    };

                    // Queue action so tab_viewer/main.rs can upload chunks to viewport
                    self.pending_actions.push(TerrainAction::Generate);

                    self.generating = false;
                    self.gen_receiver = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still generating — do nothing
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Thread finished without sending (error case)
                    self.generating = false;
                    self.gen_receiver = None;
                }
            }
        }

        // Poll deferred scatter placements (non-blocking)
        if let Some(rx) = &self.scatter_receiver {
            match rx.try_recv() {
                Ok(placements) => {
                    if !placements.is_empty() {
                        self.cached_scatter_placements = placements;
                        // Re-queue Generate so main.rs picks up scatter
                        self.pending_actions.push(TerrainAction::Generate);
                    }
                    self.scatter_receiver = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.scatter_receiver = None;
                }
            }
        }
    }
}

// =============================================================================
// Phase 1.X-Editor-Multi-Tool-Architecture-Sub-phase-3:
// `impl ActiveTool for TerrainPanel` (additive coexistence with main.rs:3833-3877
// mediator code per Andrew Q2 risk-bounding). Both ActiveTool path + main.rs
// mediator path are active simultaneously through Sub-phase 3 + Sub-phase 5;
// Mediator Removal session (per campaign doc §6 + Q6) removes the mediator code
// once both tools are registered + Andrew-gate verifies the dispatcher path.
//
// Per Sub-phase 1 Diagnostic audit §2.2.3 verification: TerrainPanel's
// `apply_brush_at(world_x, world_z)` (line 819) + `end_brush_stroke()` (line 864)
// are the integration targets. Per-event handlers route to these existing methods
// with world-XZ coordinates from `ToolContext::world_xz_at_pointer()` (Sub-phase 2
// §2.7 resolution; depth-buffer-based projection per viewport/widget.rs:1219-1234).
// =============================================================================

impl ActiveTool for TerrainPanel {
    fn uuid(&self) -> Uuid {
        TERRAIN_PANEL_UUID
    }

    fn name(&self) -> &str {
        "Terrain Brush"
    }

    /// Lifecycle: tool activated when dispatcher's set_active_tool selects this.
    /// Coexistence note: existing `is_brush_active()` at line 797 returns
    /// `self.brush_enabled && self.terrain_state.has_terrain()`. The UI-side
    /// toggle at line ~1180 sets `brush_enabled` and emits SetActiveTool
    /// action; the dispatcher's set_active_tool then calls activate() here.
    /// activate() does NOT modify brush_enabled (already set by UI toggle);
    /// this is intentional to keep both ActiveTool path + main.rs mediator
    /// path in sync per Q2 additive coexistence.
    fn activate(&mut self, _context: &mut ToolContext) {
        // Brush_enabled already set by UI toggle that emitted SetActiveTool.
        // No additional state change needed here for additive coexistence.
    }

    /// Lifecycle: tool deactivated when dispatcher selects another tool or None.
    /// Symmetric with activate(): UI toggle clears brush_enabled before emitting
    /// SetActiveTool { uuid: None }.
    fn deactivate(&mut self, _context: &mut ToolContext) {
        // Brush_enabled already cleared by UI toggle. No additional state change.
    }

    /// Per-event handler: stroke start. Route to existing `apply_brush_at` API
    /// with world-XZ from depth-buffer-based projection (`world_xz_at_pointer`).
    /// Falls back to PassThrough if pointer projection fails (sky depth = 1.0,
    /// pointer outside viewport, depth buffer unavailable).
    ///
    /// Coexistence note: existing main.rs mediator path at viewport/widget.rs:
    /// 1200-1255 also calls `apply_brush_at` via `take_terrain_brush_hits` drain
    /// (main.rs:3862-3877). Sub-phase 3 keeps both paths active; the mediator
    /// path remains the primary functional path during Andrew-gate verification.
    /// `apply_brush_at` is idempotent for repeated calls at the same world
    /// position within the same frame (existing behavior; not modified).
    fn on_left_mouse_button_down(
        &mut self,
        _event: &MouseEvent,
        context: &mut ToolContext,
    ) -> EventDisposition {
        if let Some((world_x, world_z)) = context.world_xz_at_pointer() {
            self.apply_brush_at(world_x, world_z);
            EventDisposition::Consumed
        } else {
            EventDisposition::PassThrough
        }
    }

    /// Per-event handler: stroke continuation. Routes to `apply_brush_at`
    /// identically to on_left_mouse_button_down. Throttling is preserved by
    /// `apply_brush_at`'s internal `last_brush_time` check (per audit §2.2.2);
    /// no throttling logic duplicated at ActiveTool layer.
    fn on_mouse_move(
        &mut self,
        _event: &MouseEvent,
        context: &mut ToolContext,
    ) -> EventDisposition {
        if let Some((world_x, world_z)) = context.world_xz_at_pointer() {
            self.apply_brush_at(world_x, world_z);
            EventDisposition::Consumed
        } else {
            EventDisposition::PassThrough
        }
    }

    /// Per-event handler: stroke end. Per Sub-phase 3 prompt §2.1 option (b):
    /// does NOT call `end_brush_stroke()` here. The existing main.rs mediator
    /// path at main.rs:3867-3877 detects stroke-end via
    /// `viewport.take_terrain_brush_stroke_ended()` and emits
    /// TerrainBrushCommand to undo_stack. Calling end_brush_stroke() here too
    /// would produce duplicate undo entries; deferring to mediator preserves
    /// Sub-phase 5's §2.11 undo_stack-via-ToolContext deferral discipline.
    ///
    /// The Mediator Removal session (per campaign doc §6 + Q6) will resolve
    /// stroke-end coordination by either (a) moving end_brush_stroke +
    /// TerrainBrushCommand emission into this method, or (b) routing through
    /// the resolved ToolContext.undo_stack mechanism per Sub-phase 5.
    fn on_left_mouse_button_up(
        &mut self,
        _event: &MouseEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        // Mediator path handles stroke-end + undo emission per coexistence pattern.
        EventDisposition::PassThrough
    }

    /// Per-event handler: key press. TerrainPanel's brush mode switching is
    /// currently UI-driven (selectable_value buttons at line ~1192-1200).
    /// Future enhancement: keyboard shortcuts for brush mode cycling would land
    /// here. Default PassThrough preserves existing keyboard behavior.
    fn on_key_down(
        &mut self,
        _key: &KeyEvent,
        _context: &mut ToolContext,
    ) -> EventDisposition {
        EventDisposition::PassThrough
    }

    /// UI integration: provide a toolbar button for the dispatcher's tool
    /// palette. Sub-phase 3 uses the default `selectable_label(name())`
    /// pattern from Sub-phase 2's trait default; richer UI deferred until
    /// dispatcher's tool palette UI is built (likely Mediator Removal session
    /// or a UI-polish follow-up).
    ///
    /// Note: clicking this button does NOT directly toggle dispatcher state.
    /// The brush-mode toggle at line ~1180 is the canonical activation site;
    /// this button is for tool palette display only.
    fn make_button(&mut self, ui: &mut Ui, selected: bool) {
        let _ = ui.selectable_label(selected, ActiveTool::name(self));
    }
}

impl Panel for TerrainPanel {
    fn name(&self) -> &str {
        "Terrain"
    }

    fn show(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.show_generation_section(ui);
            self.show_noise_section(ui);
            self.show_erosion_section(ui);
            self.show_biome_blend_section(ui);
            self.show_splatting_section(ui);
            self.show_brush_section(ui);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // EROSION PRESET TYPE TESTS
    // ============================================================

    #[test]
    fn test_erosion_preset_type_all() {
        let all = ErosionPresetType::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_erosion_preset_type_all_coverage() {
        let all = ErosionPresetType::all();
        assert!(all.contains(&ErosionPresetType::Custom));
        assert!(all.contains(&ErosionPresetType::Desert));
        assert!(all.contains(&ErosionPresetType::Mountain));
        assert!(all.contains(&ErosionPresetType::Coastal));
        assert!(all.contains(&ErosionPresetType::Alpine));
        assert!(all.contains(&ErosionPresetType::Canyon));
    }

    #[test]
    fn test_erosion_preset_type_names() {
        assert_eq!(ErosionPresetType::Custom.name(), "Custom");
        assert_eq!(ErosionPresetType::Desert.name(), "Desert");
        assert_eq!(ErosionPresetType::Mountain.name(), "Mountain");
        assert_eq!(ErosionPresetType::Coastal.name(), "Coastal");
        assert_eq!(ErosionPresetType::Alpine.name(), "Alpine");
        assert_eq!(ErosionPresetType::Canyon.name(), "Canyon");
    }

    // ============================================================
    // WATER BODY PRESET TESTS
    // ============================================================

    #[test]
    fn test_water_body_preset_all() {
        let all = WaterBodyPreset::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_water_body_preset_all_coverage() {
        let all = WaterBodyPreset::all();
        assert!(all.contains(&WaterBodyPreset::Custom));
        assert!(all.contains(&WaterBodyPreset::CalmLake));
        assert!(all.contains(&WaterBodyPreset::MountainStream));
        assert!(all.contains(&WaterBodyPreset::RagingRiver));
        assert!(all.contains(&WaterBodyPreset::Ocean));
        assert!(all.contains(&WaterBodyPreset::Waterfall));
        assert!(all.contains(&WaterBodyPreset::SwampWetland));
    }

    #[test]
    fn test_water_body_preset_names() {
        assert_eq!(WaterBodyPreset::Custom.name(), "Custom");
        assert_eq!(WaterBodyPreset::CalmLake.name(), "Calm Lake");
        assert_eq!(WaterBodyPreset::Ocean.name(), "Ocean");
    }

    // ============================================================
    // FLUID QUALITY PRESET TESTS
    // ============================================================

    #[test]
    fn test_fluid_quality_preset_names() {
        assert_eq!(FluidQualityPreset::Performance.name(), "Performance");
        assert_eq!(FluidQualityPreset::Balanced.name(), "Balanced");
        assert_eq!(FluidQualityPreset::Quality.name(), "Quality");
        assert_eq!(FluidQualityPreset::Cinematic.name(), "Cinematic");
    }

    // ============================================================
    // BRUSH MODE TESTS
    // ============================================================

    #[test]
    fn test_brush_mode_all_variants() {
        let variants = [
            BrushMode::Sculpt,
            BrushMode::Smooth,
            BrushMode::Flatten,
            BrushMode::Paint,
            BrushMode::Erode,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // HYDRAULIC EROSION PARAMS TESTS
    // ============================================================

    #[test]
    fn test_hydraulic_erosion_default() {
        let he = HydraulicErosionParams::default();
        assert!(he.enabled);
        assert_eq!(he.iterations, 50000);
    }

    #[test]
    fn test_hydraulic_erosion_physics() {
        let he = HydraulicErosionParams::default();
        assert!((he.inertia - 0.3).abs() < 0.01);
        assert!((he.capacity - 8.0).abs() < 0.01);
        assert!((he.gravity - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_hydraulic_erosion_rates() {
        let he = HydraulicErosionParams::default();
        assert!((he.deposition - 0.2).abs() < 0.01);
        assert!((he.erosion - 0.5).abs() < 0.01);
        assert!((he.evaporation - 0.02).abs() < 0.01);
    }

    #[test]
    fn test_hydraulic_erosion_clone() {
        let he = HydraulicErosionParams::default();
        let cloned = he.clone();
        assert!(cloned.enabled);
    }

    // ============================================================
    // THERMAL EROSION PARAMS TESTS
    // ============================================================

    #[test]
    fn test_thermal_erosion_default() {
        let te = ThermalErosionParams::default();
        assert!(te.enabled);
        assert_eq!(te.iterations, 50);
    }

    #[test]
    fn test_thermal_erosion_params() {
        let te = ThermalErosionParams::default();
        assert!((te.talus_angle - 40.0).abs() < 0.01);
        assert!((te.erosion_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_thermal_erosion_clone() {
        let te = ThermalErosionParams::default();
        let cloned = te.clone();
        assert!(cloned.enabled);
    }

    // ============================================================
    // WIND EROSION PARAMS TESTS
    // ============================================================

    #[test]
    fn test_wind_erosion_default() {
        let we = WindErosionParams::default();
        assert!(!we.enabled);
        assert_eq!(we.iterations, 20);
    }

    #[test]
    fn test_wind_erosion_params() {
        let we = WindErosionParams::default();
        assert!((we.wind_strength - 0.5).abs() < 0.01);
        assert!((we.suspension - 0.3).abs() < 0.01);
        assert!((we.abrasion - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_wind_erosion_direction() {
        let we = WindErosionParams::default();
        assert!((we.wind_direction[0] - 1.0).abs() < 0.01);
        assert!((we.wind_direction[1] - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_wind_erosion_clone() {
        let we = WindErosionParams::default();
        let cloned = we.clone();
        assert!(!cloned.enabled);
    }

    // ============================================================
    // BIOME BLEND PARAMS TESTS
    // ============================================================

    #[test]
    fn test_biome_blend_default() {
        let bb = BiomeBlendParams::default();
        assert!(bb.enabled);
        assert!((bb.blend_radius - 32.0).abs() < 0.01);
    }

    #[test]
    fn test_biome_blend_biomes() {
        let bb = BiomeBlendParams::default();
        assert_eq!(bb.secondary_biome, "desert");
        assert_eq!(bb.tertiary_biome, "mountains");
    }

    #[test]
    fn test_biome_blend_params() {
        let bb = BiomeBlendParams::default();
        assert!((bb.falloff_power - 2.0).abs() < 0.01);
        assert!((bb.noise_influence - 0.3).abs() < 0.01);
        assert!(!bb.show_blend_preview);
    }

    #[test]
    fn test_biome_blend_clone() {
        let bb = BiomeBlendParams::default();
        let cloned = bb.clone();
        assert!(cloned.enabled);
    }

    // ============================================================
    // SPLAT PARAMS TESTS
    // ============================================================

    #[test]
    fn test_splat_params_default() {
        let sp = SplatParams::default();
        assert!(sp.enabled);
        assert!(!sp.show_splat_preview);
    }

    #[test]
    fn test_splat_params_thresholds() {
        let sp = SplatParams::default();
        assert!((sp.rock_slope_threshold - 0.6).abs() < 0.01);
        assert!((sp.snow_height_threshold - 0.85).abs() < 0.01);
        assert!((sp.triplanar_sharpness - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_splat_params_heights() {
        let sp = SplatParams::default();
        assert!((sp.grass_height_min - 0.0).abs() < 0.01);
        assert!((sp.grass_height_max - 0.7).abs() < 0.01);
        assert!((sp.sand_height_max - 0.15).abs() < 0.01);
    }

    #[test]
    fn test_splat_params_clone() {
        let sp = SplatParams::default();
        let cloned = sp.clone();
        assert!(cloned.enabled);
    }

    // ============================================================
    // FLUID SIM PARAMS TESTS
    // ============================================================

    #[test]
    fn test_fluid_sim_default() {
        let fp = FluidSimParams::default();
        assert!(fp.enabled);
        assert_eq!(fp.quality_preset, FluidQualityPreset::Balanced);
        assert_eq!(fp.water_body_preset, WaterBodyPreset::CalmLake);
    }

    #[test]
    fn test_fluid_sim_physics() {
        let fp = FluidSimParams::default();
        assert_eq!(fp.particle_count, 65536);
        assert!((fp.smoothing_radius - 1.0).abs() < 0.01);
        assert!((fp.gravity - (-9.8)).abs() < 0.01);
    }

    #[test]
    fn test_fluid_sim_rendering() {
        let fp = FluidSimParams::default();
        assert!((fp.transparency - 0.7).abs() < 0.01);
        assert!(fp.caustics_enabled);
        assert!(fp.foam_enabled);
    }

    #[test]
    fn test_fluid_sim_clone() {
        let fp = FluidSimParams::default();
        let cloned = fp.clone();
        assert!(cloned.enabled);
    }

    // ============================================================
    // TERRAIN PANEL TESTS
    // ============================================================

    #[test]
    fn test_terrain_panel_creation() {
        let panel = TerrainPanel::new();
        assert_eq!(panel.seed, 12345);
        assert_eq!(panel.primary_biome, "grassland");
        assert_eq!(panel.chunk_radius, 5);
    }

    #[test]
    fn test_default_noise_params() {
        let panel = TerrainPanel::new();
        assert_eq!(panel.octaves, 6);
        assert!((panel.lacunarity - 2.0).abs() < 0.01);
        assert!((panel.persistence - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_brush_modes() {
        let mut panel = TerrainPanel::new();
        assert_eq!(panel.brush_mode, BrushMode::Sculpt);

        panel.brush_mode = BrushMode::Smooth;
        assert_eq!(panel.brush_mode, BrushMode::Smooth);

        panel.brush_mode = BrushMode::Erode;
        assert_eq!(panel.brush_mode, BrushMode::Erode);
    }

    #[test]
    fn test_brush_mode_all_settable() {
        let mut panel = TerrainPanel::new();
        let modes = [
            BrushMode::Sculpt,
            BrushMode::Smooth,
            BrushMode::Flatten,
            BrushMode::Paint,
            BrushMode::Erode,
        ];
        for mode in modes {
            panel.brush_mode = mode;
            assert_eq!(panel.brush_mode, mode);
        }
    }

    #[test]
    fn test_material_names() {
        assert_eq!(TerrainPanel::material_name(0), "Grass");
        assert_eq!(TerrainPanel::material_name(2), "Rock");
        assert_eq!(TerrainPanel::material_name(99), "Unknown");
    }

    #[test]
    fn test_material_names_all() {
        assert_eq!(TerrainPanel::material_name(1), "Sand");
        assert_eq!(TerrainPanel::material_name(3), "Snow");
        assert_eq!(TerrainPanel::material_name(4), "Dirt");
    }

    #[test]
    fn test_erosion_presets() {
        let mut panel = TerrainPanel::new();
        assert_eq!(panel.erosion_preset, ErosionPresetType::Mountain);

        panel.apply_erosion_preset(ErosionPresetType::Desert);
        assert!(!panel.hydraulic_erosion.enabled);
        assert!(panel.thermal_erosion.enabled);
        assert!(panel.wind_erosion.enabled);

        panel.apply_erosion_preset(ErosionPresetType::Coastal);
        assert!(panel.hydraulic_erosion.enabled);
        assert!(!panel.thermal_erosion.enabled);
        assert!(panel.wind_erosion.enabled);
    }

    #[test]
    fn test_erosion_preset_alpine() {
        let mut panel = TerrainPanel::new();
        panel.apply_erosion_preset(ErosionPresetType::Alpine);
        // Alpine should have both hydraulic and thermal active
        assert!(panel.hydraulic_erosion.enabled || panel.thermal_erosion.enabled);
    }

    #[test]
    fn test_default_erosion_params() {
        let panel = TerrainPanel::new();
        assert!(panel.hydraulic_erosion.enabled);
        assert_eq!(panel.hydraulic_erosion.iterations, 50000);
        assert!(panel.thermal_erosion.enabled);
        assert!(!panel.wind_erosion.enabled);
    }

    #[test]
    fn test_biome_blend_defaults() {
        let panel = TerrainPanel::new();
        assert!(panel.biome_blend.enabled);
        assert!((panel.biome_blend.blend_radius - 32.0).abs() < 0.01);
        assert_eq!(panel.biome_blend.secondary_biome, "desert");
        assert_eq!(panel.biome_blend.tertiary_biome, "mountains");
    }

    #[test]
    fn test_splat_params_defaults() {
        let panel = TerrainPanel::new();
        assert!(panel.splat_params.enabled);
        assert!((panel.splat_params.rock_slope_threshold - 0.6).abs() < 0.01);
        assert!((panel.splat_params.snow_height_threshold - 0.85).abs() < 0.01);
        assert!((panel.splat_params.triplanar_sharpness - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_panel_trait() {
        let panel = TerrainPanel::new();
        assert_eq!(panel.name(), "Terrain");
    }

    // ============================================================
    // INTEGRATION TESTS
    // ============================================================

    #[test]
    fn test_erosion_preset_all() {
        let presets = ErosionPresetType::all();
        assert_eq!(presets.len(), 6);
        assert!(presets.contains(&ErosionPresetType::Custom));
        assert!(presets.contains(&ErosionPresetType::Canyon));
    }

    #[test]
    fn test_all_presets_have_names() {
        for preset in ErosionPresetType::all() {
            assert!(!preset.name().is_empty());
        }
    }

    #[test]
    fn test_all_water_body_presets_have_names() {
        for preset in WaterBodyPreset::all() {
            assert!(!preset.name().is_empty());
        }
    }

    #[test]
    fn test_terrain_generation_settings() {
        let panel = TerrainPanel::new();
        assert!((panel.base_amplitude - 50.0).abs() < 0.01);
        assert_eq!(panel.seed, 12345);
    }

    // ============================================================
    // DISPLAY TRAIT TESTS
    // ============================================================

    #[test]
    fn test_erosion_preset_type_display() {
        for preset in ErosionPresetType::all() {
            let display = format!("{}", preset);
            assert!(display.contains(preset.name()));
        }
    }

    #[test]
    fn test_erosion_preset_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for preset in ErosionPresetType::all() {
            set.insert(*preset);
        }
        assert_eq!(set.len(), ErosionPresetType::all().len());
    }

    #[test]
    fn test_water_body_preset_display() {
        for preset in WaterBodyPreset::all() {
            let display = format!("{}", preset);
            assert!(display.contains(preset.name()));
        }
    }

    #[test]
    fn test_water_body_preset_all_count() {
        let all = WaterBodyPreset::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_water_body_preset_is_flowing() {
        assert!(WaterBodyPreset::MountainStream.is_flowing());
        assert!(WaterBodyPreset::RagingRiver.is_flowing());
        assert!(WaterBodyPreset::Waterfall.is_flowing());
        assert!(!WaterBodyPreset::CalmLake.is_flowing());
        assert!(!WaterBodyPreset::Ocean.is_flowing());
    }

    #[test]
    fn test_water_body_preset_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for preset in WaterBodyPreset::all() {
            set.insert(*preset);
        }
        assert_eq!(set.len(), WaterBodyPreset::all().len());
    }

    #[test]
    fn test_fluid_quality_preset_display() {
        for preset in FluidQualityPreset::all() {
            let display = format!("{}", preset);
            assert!(display.contains(preset.name()));
        }
    }

    #[test]
    fn test_fluid_quality_preset_all() {
        let all = FluidQualityPreset::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_fluid_quality_preset_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for preset in FluidQualityPreset::all() {
            set.insert(*preset);
        }
        assert_eq!(set.len(), FluidQualityPreset::all().len());
    }

    #[test]
    fn test_brush_mode_display() {
        for mode in BrushMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_brush_mode_all() {
        let all = BrushMode::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_brush_mode_icon() {
        assert_eq!(BrushMode::Sculpt.icon(), "🏔️");
        assert_eq!(BrushMode::Paint.icon(), "🖌️");
        assert_eq!(BrushMode::Erode.icon(), "💧");
    }

    #[test]
    fn test_brush_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in BrushMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), BrushMode::all().len());
    }

    // TerrainAction tests
    #[test]
    fn test_terrain_action_generate() {
        let action = TerrainAction::Generate;
        assert!(matches!(action, TerrainAction::Generate));
    }

    #[test]
    fn test_terrain_action_set_seed() {
        let action = TerrainAction::SetSeed(12345);
        if let TerrainAction::SetSeed(seed) = action {
            assert_eq!(seed, 12345);
        } else {
            panic!("Expected SetSeed action");
        }
    }

    #[test]
    fn test_terrain_action_set_biome() {
        let action = TerrainAction::SetBiome("desert".to_string());
        if let TerrainAction::SetBiome(biome) = action {
            assert_eq!(biome, "desert");
        } else {
            panic!("Expected SetBiome action");
        }
    }

    #[test]
    fn test_terrain_action_chunk_radius() {
        let action = TerrainAction::SetChunkRadius(3);
        assert!(matches!(action, TerrainAction::SetChunkRadius(3)));
    }

    #[test]
    fn test_terrain_action_erosion_preset() {
        let action = TerrainAction::ApplyErosionPreset(ErosionPresetType::Mountain);
        assert!(matches!(
            action,
            TerrainAction::ApplyErosionPreset(ErosionPresetType::Mountain)
        ));
    }

    #[test]
    fn test_terrain_action_run_erosion() {
        let hydraulic = TerrainAction::RunHydraulicErosion;
        let thermal = TerrainAction::RunThermalErosion;
        let wind = TerrainAction::RunWindErosion;

        assert!(matches!(hydraulic, TerrainAction::RunHydraulicErosion));
        assert!(matches!(thermal, TerrainAction::RunThermalErosion));
        assert!(matches!(wind, TerrainAction::RunWindErosion));
    }

    #[test]
    fn test_terrain_action_brush_mode() {
        let action = TerrainAction::SetBrushMode(BrushMode::Sculpt);
        assert!(matches!(
            action,
            TerrainAction::SetBrushMode(BrushMode::Sculpt)
        ));
    }

    #[test]
    fn test_terrain_action_brush_settings() {
        let radius = TerrainAction::SetBrushRadius(10.0);
        let strength = TerrainAction::SetBrushStrength(0.75);

        if let TerrainAction::SetBrushRadius(r) = radius {
            assert!((r - 10.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected SetBrushRadius");
        }

        if let TerrainAction::SetBrushStrength(s) = strength {
            assert!((s - 0.75).abs() < f32::EPSILON);
        } else {
            panic!("Expected SetBrushStrength");
        }
    }

    #[test]
    fn test_terrain_action_apply_brush() {
        let action = TerrainAction::ApplyBrush {
            position: [1.0, 2.0, 3.0],
        };
        if let TerrainAction::ApplyBrush { position } = action {
            assert_eq!(position, [1.0, 2.0, 3.0]);
        } else {
            panic!("Expected ApplyBrush action");
        }
    }

    #[test]
    fn test_terrain_action_fluid_simulation() {
        let toggle = TerrainAction::ToggleFluidSimulation(true);
        let reset = TerrainAction::ResetFluidSimulation;

        assert!(matches!(toggle, TerrainAction::ToggleFluidSimulation(true)));
        assert!(matches!(reset, TerrainAction::ResetFluidSimulation));
    }

    #[test]
    fn test_terrain_action_export_import() {
        let export = TerrainAction::ExportHeightmap {
            path: "/tmp/height.raw".to_string(),
        };
        let import = TerrainAction::ImportHeightmap {
            path: "/tmp/height.raw".to_string(),
        };

        if let TerrainAction::ExportHeightmap { path } = export {
            assert_eq!(path, "/tmp/height.raw");
        } else {
            panic!("Expected ExportHeightmap");
        }

        if let TerrainAction::ImportHeightmap { path } = import {
            assert_eq!(path, "/tmp/height.raw");
        } else {
            panic!("Expected ImportHeightmap");
        }
    }

    #[test]
    fn test_terrain_action_queue_and_take() {
        let mut panel = TerrainPanel::new();
        assert!(!panel.has_pending_actions());

        panel.queue_action(TerrainAction::Generate);
        panel.queue_action(TerrainAction::SetSeed(999));
        assert!(panel.has_pending_actions());

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_terrain_action_equality() {
        let a1 = TerrainAction::RandomizeSeed;
        let a2 = TerrainAction::RandomizeSeed;
        assert_eq!(a1, a2);
    }

    #[test]
    fn test_terrain_action_debug() {
        let action = TerrainAction::ToggleAutoRegenerate(true);
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("ToggleAutoRegenerate"));
    }
}
