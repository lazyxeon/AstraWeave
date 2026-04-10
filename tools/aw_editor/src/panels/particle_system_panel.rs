//! Particle System Panel for the editor UI
//!
//! Provides comprehensive VFX and particle editing:
//! - Emitter configuration (shape, rate, lifetime)
//! - Particle properties (size, color, velocity)
//! - Modules (forces, collisions, sub-emitters)
//! - GPU particle system settings
//! - Real-time preview and performance metrics

use egui::{Color32, RichText, Ui, Vec2};
use tracing::info;

use crate::panels::Panel;

/// Emitter shape type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EmitterShape {
    #[default]
    Point,
    Sphere,
    Hemisphere,
    Cone,
    Box,
    Circle,
    Edge,
    Mesh,
}

impl std::fmt::Display for EmitterShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl EmitterShape {
    pub fn all() -> &'static [EmitterShape] {
        &[
            EmitterShape::Point,
            EmitterShape::Sphere,
            EmitterShape::Hemisphere,
            EmitterShape::Cone,
            EmitterShape::Box,
            EmitterShape::Circle,
            EmitterShape::Edge,
            EmitterShape::Mesh,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            EmitterShape::Point => "Point",
            EmitterShape::Sphere => "Sphere",
            EmitterShape::Hemisphere => "Hemisphere",
            EmitterShape::Cone => "Cone",
            EmitterShape::Box => "Box",
            EmitterShape::Circle => "Circle",
            EmitterShape::Edge => "Edge",
            EmitterShape::Mesh => "Mesh",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            EmitterShape::Point => "•",
            EmitterShape::Sphere => "⚪",
            EmitterShape::Hemisphere => "◗",
            EmitterShape::Cone => "▲",
            EmitterShape::Box => "⬜",
            EmitterShape::Circle => "○",
            EmitterShape::Edge => "―",
            EmitterShape::Mesh => "🔺",
        }
    }

    /// Returns true if this shape is volumetric (3D)
    pub fn is_volumetric(&self) -> bool {
        matches!(
            self,
            EmitterShape::Sphere
                | EmitterShape::Hemisphere
                | EmitterShape::Cone
                | EmitterShape::Box
        )
    }
}

/// Simulation space
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SimulationSpace {
    #[default]
    Local,
    World,
}

impl std::fmt::Display for SimulationSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SimulationSpace {
    pub fn all() -> &'static [SimulationSpace] {
        &[SimulationSpace::Local, SimulationSpace::World]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SimulationSpace::Local => "Local",
            SimulationSpace::World => "World",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SimulationSpace::Local => "📍",
            SimulationSpace::World => "[Glb]",
        }
    }
}

/// Particle blend mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParticleBlendMode {
    #[default]
    Alpha,
    Additive,
    Multiply,
    Premultiply,
}

impl std::fmt::Display for ParticleBlendMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ParticleBlendMode {
    pub fn all() -> &'static [ParticleBlendMode] {
        &[
            ParticleBlendMode::Alpha,
            ParticleBlendMode::Additive,
            ParticleBlendMode::Multiply,
            ParticleBlendMode::Premultiply,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ParticleBlendMode::Alpha => "Alpha",
            ParticleBlendMode::Additive => "Additive",
            ParticleBlendMode::Multiply => "Multiply",
            ParticleBlendMode::Premultiply => "Premultiply",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ParticleBlendMode::Alpha => "[Sq]",
            ParticleBlendMode::Additive => "[Fx]",
            ParticleBlendMode::Multiply => "✖️",
            ParticleBlendMode::Premultiply => "🔳",
        }
    }

    /// Returns true if this blend mode uses additive blending
    pub fn is_additive(&self) -> bool {
        matches!(self, ParticleBlendMode::Additive)
    }
}

/// Particle render mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParticleRenderMode {
    #[default]
    Billboard,
    StretchedBillboard,
    HorizontalBillboard,
    VerticalBillboard,
    Mesh,
    Trail,
}

impl std::fmt::Display for ParticleRenderMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ParticleRenderMode {
    pub fn all() -> &'static [ParticleRenderMode] {
        &[
            ParticleRenderMode::Billboard,
            ParticleRenderMode::StretchedBillboard,
            ParticleRenderMode::HorizontalBillboard,
            ParticleRenderMode::VerticalBillboard,
            ParticleRenderMode::Mesh,
            ParticleRenderMode::Trail,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ParticleRenderMode::Billboard => "Billboard",
            ParticleRenderMode::StretchedBillboard => "Stretched Billboard",
            ParticleRenderMode::HorizontalBillboard => "Horizontal Billboard",
            ParticleRenderMode::VerticalBillboard => "Vertical Billboard",
            ParticleRenderMode::Mesh => "Mesh",
            ParticleRenderMode::Trail => "Trail",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ParticleRenderMode::Billboard => "[Doc]",
            ParticleRenderMode::StretchedBillboard => "📏",
            ParticleRenderMode::HorizontalBillboard => "[Rt]",
            ParticleRenderMode::VerticalBillboard => "[Up]",
            ParticleRenderMode::Mesh => "[D]",
            ParticleRenderMode::Trail => "〰️",
        }
    }

    /// Returns true if this render mode is a billboard type
    pub fn is_billboard(&self) -> bool {
        matches!(
            self,
            ParticleRenderMode::Billboard
                | ParticleRenderMode::StretchedBillboard
                | ParticleRenderMode::HorizontalBillboard
                | ParticleRenderMode::VerticalBillboard
        )
    }
}

/// Value over lifetime curve type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CurveType {
    #[default]
    Constant,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Random,
    Curve,
}

impl std::fmt::Display for CurveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl CurveType {
    pub fn all() -> &'static [CurveType] {
        &[
            CurveType::Constant,
            CurveType::Linear,
            CurveType::EaseIn,
            CurveType::EaseOut,
            CurveType::EaseInOut,
            CurveType::Random,
            CurveType::Curve,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            CurveType::Constant => "Constant",
            CurveType::Linear => "Linear",
            CurveType::EaseIn => "Ease In",
            CurveType::EaseOut => "Ease Out",
            CurveType::EaseInOut => "Ease In Out",
            CurveType::Random => "Random",
            CurveType::Curve => "Curve",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            CurveType::Constant => "-",
            CurveType::Linear => "📈",
            CurveType::EaseIn => "⤴️",
            CurveType::EaseOut => "⤵️",
            CurveType::EaseInOut => "〰️",
            CurveType::Random => "[Dice]",
            CurveType::Curve => "✏️",
        }
    }

    /// Returns true if this curve type is an easing function
    pub fn is_easing(&self) -> bool {
        matches!(
            self,
            CurveType::EaseIn | CurveType::EaseOut | CurveType::EaseInOut
        )
    }
}

/// Range value (min-max)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeValue {
    pub min: f32,
    pub max: f32,
}

impl Default for RangeValue {
    fn default() -> Self {
        Self { min: 1.0, max: 1.0 }
    }
}

impl RangeValue {
    pub fn constant(value: f32) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    pub fn range(min: f32, max: f32) -> Self {
        Self { min, max }
    }
}

/// Color gradient stop
#[derive(Debug, Clone)]
pub struct GradientStop {
    pub position: f32,
    pub color: [f32; 4],
}

/// Color over lifetime gradient
#[derive(Debug, Clone)]
pub struct ColorGradient {
    pub stops: Vec<GradientStop>,
}

impl Default for ColorGradient {
    fn default() -> Self {
        Self {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                },
                GradientStop {
                    position: 1.0,
                    color: [1.0, 1.0, 1.0, 0.0],
                },
            ],
        }
    }
}

/// Emitter module configuration
#[derive(Debug, Clone)]
pub struct EmitterModule {
    pub enabled: bool,
    pub name: String,
    pub module_type: ModuleType,
}

/// Module types
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ModuleType {
    Velocity {
        direction: [f32; 3],
        speed: RangeValue,
    },
    Force {
        force: [f32; 3],
        space: SimulationSpace,
    },
    Gravity {
        multiplier: f32,
    },
    Noise {
        strength: f32,
        frequency: f32,
        scroll_speed: f32,
    },
    Collision {
        bounce: f32,
        lifetime_loss: f32,
        radius_scale: f32,
    },
    SubEmitter {
        event: SubEmitterEvent,
        emitter_id: u32,
    },
    TextureAnimation {
        tiles_x: u32,
        tiles_y: u32,
        fps: f32,
    },
    Trail {
        width: RangeValue,
        lifetime: f32,
        min_vertex_distance: f32,
    },
    Light {
        color: [f32; 3],
        intensity: RangeValue,
        range: RangeValue,
    },
    Rotation {
        speed: RangeValue,
        random_start: bool,
    },
}

impl std::fmt::Display for ModuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ModuleType {
    /// Returns all module types with default values for demonstration
    pub fn all_variants() -> &'static [&'static str] {
        &[
            "Velocity",
            "Force",
            "Gravity",
            "Noise",
            "Collision",
            "SubEmitter",
            "TextureAnimation",
            "Trail",
            "Light",
            "Rotation",
        ]
    }

    /// Returns the name of the module type
    pub fn name(&self) -> &'static str {
        match self {
            ModuleType::Velocity { .. } => "Velocity",
            ModuleType::Force { .. } => "Force",
            ModuleType::Gravity { .. } => "Gravity",
            ModuleType::Noise { .. } => "Noise",
            ModuleType::Collision { .. } => "Collision",
            ModuleType::SubEmitter { .. } => "Sub Emitter",
            ModuleType::TextureAnimation { .. } => "Texture Animation",
            ModuleType::Trail { .. } => "Trail",
            ModuleType::Light { .. } => "Light",
            ModuleType::Rotation { .. } => "Rotation",
        }
    }

    /// Returns the icon for the module type
    pub fn icon(&self) -> &'static str {
        match self {
            ModuleType::Velocity { .. } => "[Rt]",
            ModuleType::Force { .. } => "[Dash]",
            ModuleType::Gravity { .. } => "[Dn]",
            ModuleType::Noise { .. } => "[Wave]",
            ModuleType::Collision { .. } => "[Hit]",
            ModuleType::SubEmitter { .. } => "🎇",
            ModuleType::TextureAnimation { .. } => "[Anim]",
            ModuleType::Trail { .. } => "[Fx]",
            ModuleType::Light { .. } => "[Lgt]",
            ModuleType::Rotation { .. } => "[Sync]",
        }
    }

    /// Returns true if this is a physics-related module
    pub fn is_physics(&self) -> bool {
        matches!(
            self,
            ModuleType::Velocity { .. }
                | ModuleType::Force { .. }
                | ModuleType::Gravity { .. }
                | ModuleType::Collision { .. }
                | ModuleType::Rotation { .. }
        )
    }

    /// Returns true if this is a visual-related module
    pub fn is_visual(&self) -> bool {
        matches!(
            self,
            ModuleType::TextureAnimation { .. }
                | ModuleType::Trail { .. }
                | ModuleType::Light { .. }
        )
    }

    /// Returns true if this module spawns sub-effects
    pub fn is_spawner(&self) -> bool {
        matches!(self, ModuleType::SubEmitter { .. })
    }
}

/// Sub-emitter trigger event
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SubEmitterEvent {
    #[default]
    Birth,
    Death,
    Collision,
    Trigger,
}

impl std::fmt::Display for SubEmitterEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SubEmitterEvent {
    pub fn all() -> &'static [SubEmitterEvent] {
        &[
            SubEmitterEvent::Birth,
            SubEmitterEvent::Death,
            SubEmitterEvent::Collision,
            SubEmitterEvent::Trigger,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SubEmitterEvent::Birth => "Birth",
            SubEmitterEvent::Death => "Death",
            SubEmitterEvent::Collision => "Collision",
            SubEmitterEvent::Trigger => "Trigger",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SubEmitterEvent::Birth => "🌟",
            SubEmitterEvent::Death => "[Skull]",
            SubEmitterEvent::Collision => "[Hit]",
            SubEmitterEvent::Trigger => "[Zap]",
        }
    }
}

/// Particle system configuration
#[derive(Debug, Clone)]
pub struct ParticleSystem {
    pub id: u32,
    pub name: String,
    pub enabled: bool,

    // Emission
    pub duration: f32,
    pub looping: bool,
    pub prewarm: bool,
    pub start_delay: RangeValue,
    pub emission_rate: RangeValue,
    pub bursts: Vec<EmissionBurst>,

    // Shape
    pub shape: EmitterShape,
    pub shape_radius: f32,
    pub shape_angle: f32,
    pub shape_arc: f32,
    pub emit_from_edge: bool,

    // Particle properties
    pub start_lifetime: RangeValue,
    pub start_speed: RangeValue,
    pub start_size: RangeValue,
    pub start_rotation: RangeValue,
    pub start_color: [f32; 4],
    pub gravity_modifier: f32,
    pub simulation_space: SimulationSpace,
    pub max_particles: u32,

    // Over lifetime
    pub size_over_lifetime: CurveType,
    pub size_curve_end: f32,
    pub color_over_lifetime: ColorGradient,
    pub velocity_over_lifetime: [f32; 3],
    pub rotation_over_lifetime: f32,

    // Rendering
    pub render_mode: ParticleRenderMode,
    pub blend_mode: ParticleBlendMode,
    pub texture_path: String,
    pub sort_mode: SortMode,
    pub cast_shadows: bool,
    pub receive_shadows: bool,

    // Modules
    pub modules: Vec<EmitterModule>,
}

/// Emission burst
#[derive(Debug, Clone)]
pub struct EmissionBurst {
    pub time: f32,
    pub count: RangeValue,
    pub cycles: u32,
    pub interval: f32,
    pub probability: f32,
}

impl Default for EmissionBurst {
    fn default() -> Self {
        Self {
            time: 0.0,
            count: RangeValue::constant(10.0),
            cycles: 1,
            interval: 0.0,
            probability: 1.0,
        }
    }
}

/// Sort mode for particles
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SortMode {
    #[default]
    None,
    ByDistance,
    OldestFirst,
    YoungestFirst,
}

impl std::fmt::Display for SortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SortMode {
    pub fn all() -> &'static [SortMode] {
        &[
            SortMode::None,
            SortMode::ByDistance,
            SortMode::OldestFirst,
            SortMode::YoungestFirst,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SortMode::None => "None",
            SortMode::ByDistance => "By Distance",
            SortMode::OldestFirst => "Oldest First",
            SortMode::YoungestFirst => "Youngest First",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SortMode::None => "-",
            SortMode::ByDistance => "📏",
            SortMode::OldestFirst => "<<",
            SortMode::YoungestFirst => ">>",
        }
    }

    /// Returns true if this mode applies sorting
    pub fn is_sorted(&self) -> bool {
        !matches!(self, SortMode::None)
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Particle System".to_string(),
            enabled: true,

            duration: 5.0,
            looping: true,
            prewarm: false,
            start_delay: RangeValue::constant(0.0),
            emission_rate: RangeValue::constant(10.0),
            bursts: Vec::new(),

            shape: EmitterShape::Cone,
            shape_radius: 1.0,
            shape_angle: 25.0,
            shape_arc: 360.0,
            emit_from_edge: false,

            start_lifetime: RangeValue::range(3.0, 5.0),
            start_speed: RangeValue::range(1.0, 3.0),
            start_size: RangeValue::range(0.1, 0.3),
            start_rotation: RangeValue::range(0.0, 360.0),
            start_color: [1.0, 1.0, 1.0, 1.0],
            gravity_modifier: 0.0,
            simulation_space: SimulationSpace::Local,
            max_particles: 1000,

            size_over_lifetime: CurveType::Linear,
            size_curve_end: 0.0,
            color_over_lifetime: ColorGradient::default(),
            velocity_over_lifetime: [0.0, 0.0, 0.0],
            rotation_over_lifetime: 0.0,

            render_mode: ParticleRenderMode::Billboard,
            blend_mode: ParticleBlendMode::Additive,
            texture_path: String::new(),
            sort_mode: SortMode::None,
            cast_shadows: false,
            receive_shadows: false,

            modules: Vec::new(),
        }
    }
}

/// Particle system preset
#[derive(Debug, Clone)]
pub struct ParticlePreset {
    pub name: String,
    pub category: String,
    pub description: String,
}

impl ParticlePreset {
    fn presets() -> Vec<ParticlePreset> {
        vec![
            ParticlePreset {
                name: "Fire".to_string(),
                category: "Elements".to_string(),
                description: "Flickering flame effect".to_string(),
            },
            ParticlePreset {
                name: "Smoke".to_string(),
                category: "Elements".to_string(),
                description: "Rising smoke plume".to_string(),
            },
            ParticlePreset {
                name: "Sparks".to_string(),
                category: "Elements".to_string(),
                description: "Flying sparks".to_string(),
            },
            ParticlePreset {
                name: "Explosion".to_string(),
                category: "Combat".to_string(),
                description: "Burst explosion".to_string(),
            },
            ParticlePreset {
                name: "Muzzle Flash".to_string(),
                category: "Combat".to_string(),
                description: "Gun muzzle flash".to_string(),
            },
            ParticlePreset {
                name: "Blood Splatter".to_string(),
                category: "Combat".to_string(),
                description: "Impact blood effect".to_string(),
            },
            ParticlePreset {
                name: "Magic Sparkle".to_string(),
                category: "Magic".to_string(),
                description: "Magical sparkle trail".to_string(),
            },
            ParticlePreset {
                name: "Heal Aura".to_string(),
                category: "Magic".to_string(),
                description: "Healing particle ring".to_string(),
            },
            ParticlePreset {
                name: "Portal".to_string(),
                category: "Magic".to_string(),
                description: "Swirling portal effect".to_string(),
            },
            ParticlePreset {
                name: "Rain".to_string(),
                category: "Weather".to_string(),
                description: "Falling rain drops".to_string(),
            },
            ParticlePreset {
                name: "Snow".to_string(),
                category: "Weather".to_string(),
                description: "Drifting snowflakes".to_string(),
            },
            ParticlePreset {
                name: "Dust".to_string(),
                category: "Environment".to_string(),
                description: "Ambient dust motes".to_string(),
            },
            ParticlePreset {
                name: "Leaves".to_string(),
                category: "Environment".to_string(),
                description: "Falling leaves".to_string(),
            },
            ParticlePreset {
                name: "Waterfall".to_string(),
                category: "Environment".to_string(),
                description: "Waterfall spray".to_string(),
            },
        ]
    }
}

/// Performance stats
#[derive(Debug, Clone, Default)]
pub struct ParticleStats {
    pub active_particles: u32,
    pub total_emitters: u32,
    pub draw_calls: u32,
    pub gpu_memory_mb: f32,
    pub simulation_time_ms: f32,
    pub render_time_ms: f32,
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParticleTab {
    #[default]
    Emitter,
    Shape,
    Particles,
    Lifetime,
    Rendering,
    Modules,
    Presets,
    Stats,
}

impl std::fmt::Display for ParticleTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ParticleTab {
    pub fn all() -> &'static [ParticleTab] {
        &[
            ParticleTab::Emitter,
            ParticleTab::Shape,
            ParticleTab::Particles,
            ParticleTab::Lifetime,
            ParticleTab::Rendering,
            ParticleTab::Modules,
            ParticleTab::Presets,
            ParticleTab::Stats,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ParticleTab::Emitter => "Emitter",
            ParticleTab::Shape => "Shape",
            ParticleTab::Particles => "Particles",
            ParticleTab::Lifetime => "Lifetime",
            ParticleTab::Rendering => "Rendering",
            ParticleTab::Modules => "Modules",
            ParticleTab::Presets => "Presets",
            ParticleTab::Stats => "Stats",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ParticleTab::Emitter => "[Dash]",
            ParticleTab::Shape => "▲",
            ParticleTab::Particles => "[Fx]",
            ParticleTab::Lifetime => "⌛",
            ParticleTab::Rendering => "[Art]",
            ParticleTab::Modules => "[Wrn]",
            ParticleTab::Presets => "[Save]",
            ParticleTab::Stats => "[Chart]",
        }
    }
}

/// Actions that can be performed on the particle system panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParticleAction {
    /// Play preview
    Play,
    /// Pause preview
    Pause,
    /// Stop and reset preview
    Stop,
    /// Restart preview from beginning
    Restart,
    /// Set preview playback speed
    SetSpeed(f32),
    /// Create a new particle system
    CreateSystem { name: String },
    /// Delete a particle system
    DeleteSystem { id: u32 },
    /// Duplicate a particle system
    DuplicateSystem { id: u32 },
    /// Select a particle system
    SelectSystem { id: u32 },
    /// Add emission burst
    AddBurst { time: f32, count: u32 },
    /// Remove emission burst
    RemoveBurst { index: usize },
    /// Add a module to the system
    AddModule { module_type: ModuleType },
    /// Remove a module from the system
    RemoveModule { index: usize },
    /// Apply a preset to the current system
    ApplyPreset { preset_name: String },
    /// Save current system as a preset
    SaveAsPreset { name: String },
    /// Set emitter shape
    SetShape(EmitterShape),
}

impl std::fmt::Display for ParticleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ParticleAction {
    /// Returns the name of this action
    pub fn name(&self) -> &'static str {
        match self {
            ParticleAction::Play => "Play",
            ParticleAction::Pause => "Pause",
            ParticleAction::Stop => "Stop",
            ParticleAction::Restart => "Restart",
            ParticleAction::SetSpeed(_) => "Set Speed",
            ParticleAction::CreateSystem { .. } => "Create System",
            ParticleAction::DeleteSystem { .. } => "Delete System",
            ParticleAction::DuplicateSystem { .. } => "Duplicate System",
            ParticleAction::SelectSystem { .. } => "Select System",
            ParticleAction::AddBurst { .. } => "Add Burst",
            ParticleAction::RemoveBurst { .. } => "Remove Burst",
            ParticleAction::AddModule { .. } => "Add Module",
            ParticleAction::RemoveModule { .. } => "Remove Module",
            ParticleAction::ApplyPreset { .. } => "Apply Preset",
            ParticleAction::SaveAsPreset { .. } => "Save As Preset",
            ParticleAction::SetShape(_) => "Set Shape",
        }
    }

    /// Returns true if this is a playback control action
    pub fn is_playback(&self) -> bool {
        matches!(
            self,
            ParticleAction::Play
                | ParticleAction::Pause
                | ParticleAction::Stop
                | ParticleAction::Restart
                | ParticleAction::SetSpeed(_)
        )
    }

    /// Returns true if this is a system management action
    pub fn is_system_management(&self) -> bool {
        matches!(
            self,
            ParticleAction::CreateSystem { .. }
                | ParticleAction::DeleteSystem { .. }
                | ParticleAction::DuplicateSystem { .. }
                | ParticleAction::SelectSystem { .. }
        )
    }

    /// Returns true if this modifies the system configuration
    pub fn is_config_change(&self) -> bool {
        matches!(
            self,
            ParticleAction::AddBurst { .. }
                | ParticleAction::RemoveBurst { .. }
                | ParticleAction::AddModule { .. }
                | ParticleAction::RemoveModule { .. }
                | ParticleAction::SetShape(_)
        )
    }
}

/// Main Particle System Panel
pub struct ParticleSystemPanel {
    // Tab state
    active_tab: ParticleTab,

    // Systems
    particle_systems: Vec<ParticleSystem>,
    selected_system: Option<u32>,
    current_system: ParticleSystem,

    // Presets
    presets: Vec<ParticlePreset>,
    preset_filter: String,

    // Preview
    preview_playing: bool,
    preview_time: f32,
    preview_speed: f32,

    // Stats
    stats: ParticleStats,

    // ID counter
    next_id: u32,

    // Action queue
    pending_actions: Vec<ParticleAction>,
}

impl Default for ParticleSystemPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: ParticleTab::Emitter,

            particle_systems: Vec::new(),
            selected_system: None,
            current_system: ParticleSystem::default(),

            presets: ParticlePreset::presets(),
            preset_filter: String::new(),

            preview_playing: false,
            preview_time: 0.0,
            preview_speed: 1.0,

            stats: ParticleStats {
                active_particles: 1250,
                total_emitters: 8,
                draw_calls: 12,
                gpu_memory_mb: 24.5,
                simulation_time_ms: 0.45,
                render_time_ms: 0.82,
            },

            next_id: 1,

            pending_actions: Vec::new(),
        };

        panel
    }
}

impl ParticleSystemPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes all pending actions and clears the internal queue
    pub fn take_actions(&mut self) -> Vec<ParticleAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for external processing
    pub fn queue_action(&mut self, action: ParticleAction) {
        info!(target: "aw_editor::panels::particle_system_panel", action = %action, "Particle action queued");
        self.pending_actions.push(action);
    }

    /// Returns the current preview playback speed
    pub fn preview_speed(&self) -> f32 {
        self.preview_speed
    }

    /// Returns true if the preview is currently playing
    pub fn is_playing(&self) -> bool {
        self.preview_playing
    }

    /// Returns the current preview time
    pub fn preview_time(&self) -> f32 {
        self.preview_time
    }

    /// Returns the currently selected system ID
    pub fn selected_system(&self) -> Option<u32> {
        self.selected_system
    }

    fn create_sample_data(&mut self) {
        // Fire effect
        let id = self.next_id();
        self.particle_systems.push(ParticleSystem {
            id,
            name: "Torch Fire".to_string(),
            shape: EmitterShape::Cone,
            shape_radius: 0.1,
            shape_angle: 15.0,
            emission_rate: RangeValue::constant(50.0),
            start_lifetime: RangeValue::range(0.5, 1.0),
            start_speed: RangeValue::range(1.0, 2.0),
            start_size: RangeValue::range(0.1, 0.2),
            start_color: [1.0, 0.5, 0.1, 1.0],
            gravity_modifier: -0.5,
            blend_mode: ParticleBlendMode::Additive,
            ..Default::default()
        });
        self.next_id += 1;

        // Smoke effect
        let id = self.next_id();
        self.particle_systems.push(ParticleSystem {
            id,
            name: "Campfire Smoke".to_string(),
            shape: EmitterShape::Circle,
            shape_radius: 0.3,
            emission_rate: RangeValue::constant(15.0),
            start_lifetime: RangeValue::range(3.0, 5.0),
            start_speed: RangeValue::range(0.5, 1.0),
            start_size: RangeValue::range(0.3, 0.5),
            start_color: [0.3, 0.3, 0.3, 0.5],
            gravity_modifier: -0.2,
            blend_mode: ParticleBlendMode::Alpha,
            size_over_lifetime: CurveType::Linear,
            size_curve_end: 2.0,
            ..Default::default()
        });
        self.next_id += 1;

        // Magic sparkles
        let id = self.next_id();
        self.particle_systems.push(ParticleSystem {
            id,
            name: "Magic Sparkles".to_string(),
            shape: EmitterShape::Sphere,
            shape_radius: 0.5,
            emission_rate: RangeValue::constant(30.0),
            start_lifetime: RangeValue::range(0.5, 1.5),
            start_speed: RangeValue::range(0.1, 0.5),
            start_size: RangeValue::range(0.02, 0.05),
            start_color: [0.5, 0.8, 1.0, 1.0],
            blend_mode: ParticleBlendMode::Additive,
            ..Default::default()
        });
        self.next_id += 1;

        self.current_system = self.particle_systems[0].clone();
        self.selected_system = Some(self.particle_systems[0].id);
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (ParticleTab::Emitter, "[Dash] Emitter"),
                (ParticleTab::Shape, "[Sq] Shape"),
                (ParticleTab::Particles, "Particles"),
                (ParticleTab::Lifetime, "[Time] Lifetime"),
                (ParticleTab::Rendering, "Rendering"),
                (ParticleTab::Modules, "Modules"),
                (ParticleTab::Presets, "Presets"),
                (ParticleTab::Stats, "Stats"),
            ];

            for (tab, label) in tabs {
                let is_selected = self.active_tab == tab;
                let button = egui::Button::new(label).fill(if is_selected {
                    Color32::from_rgb(60, 100, 160)
                } else {
                    Color32::from_rgb(50, 50, 55)
                });

                if ui.add(button).clicked() {
                    self.active_tab = tab;
                }
            }
        });

        // System info and preview controls
        ui.horizontal(|ui| {
            ui.label(format!("{}", self.current_system.name));

            ui.separator();

            if ui
                .button(if self.preview_playing { "||" } else { ">" })
                .clicked()
            {
                self.preview_playing = !self.preview_playing;
                info!(target: "aw_editor::panels::particle_system_panel", playing = self.preview_playing, "Particle preview playback toggled");
            }
            if ui.button("[]").clicked() {
                info!(target: "aw_editor::panels::particle_system_panel", "Particle preview stopped");
                self.preview_playing = false;
                self.preview_time = 0.0;
            }
            if ui.button("[Sync]").clicked() {
                info!(target: "aw_editor::panels::particle_system_panel", "Particle preview resynced");
                self.preview_time = 0.0;
            }

            ui.label(format!("t: {:.1}s", self.preview_time));
        });

        ui.separator();
    }

    fn show_emitter_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Dash] Emitter Settings");
        ui.add_space(10.0);

        // System selector
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("system_select")
                .selected_text(&self.current_system.name)
                .show_ui(ui, |ui| {
                    for sys in &self.particle_systems.clone() {
                        if ui
                            .selectable_value(&mut self.selected_system, Some(sys.id), &sys.name)
                            .clicked()
                        {
                            self.current_system = sys.clone();
                        }
                    }
                });

            if ui.button("+ New").clicked() {
                let id = self.next_id();
                info!(target: "aw_editor::panels::particle_system_panel", system_id = id, "New particle system created");
                let new_sys = ParticleSystem {
                    id,
                    name: format!("Particle System {}", id),
                    ..Default::default()
                };
                self.particle_systems.push(new_sys.clone());
                self.current_system = new_sys;
                self.selected_system = Some(id);
            }

            if ui.button("Duplicate").clicked() {
                let id = self.next_id();
                info!(target: "aw_editor::panels::particle_system_panel", system_id = id, source = %self.current_system.name, "Particle system duplicated");
                let mut dup = self.current_system.clone();
                dup.id = id;
                dup.name = format!("{} (Copy)", dup.name);
                self.particle_systems.push(dup);
            }
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                // Basic settings
                ui.group(|ui| {
                    ui.label(RichText::new("[Edit] Basic").strong());

                    egui::Grid::new("emitter_basic")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.current_system.name);
                            ui.end_row();

                            ui.label("Enabled:");
                            ui.checkbox(&mut self.current_system.enabled, "");
                            ui.end_row();

                            ui.label("Duration:");
                            ui.add(
                                egui::DragValue::new(&mut self.current_system.duration)
                                    .speed(0.1)
                                    .range(0.1..=60.0)
                                    .suffix("s"),
                            );
                            ui.end_row();

                            ui.label("Looping:");
                            ui.checkbox(&mut self.current_system.looping, "");
                            ui.end_row();

                            ui.label("Prewarm:");
                            ui.checkbox(&mut self.current_system.prewarm, "");
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Emission settings
                ui.group(|ui| {
                    ui.label(RichText::new("📤 Emission").strong());

                    egui::Grid::new("emitter_emission")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Rate:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.current_system.emission_rate.min,
                                    )
                                    .prefix("Min:")
                                    .speed(1.0)
                                    .range(0.0..=1000.0),
                                );
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.current_system.emission_rate.max,
                                    )
                                    .prefix("Max:")
                                    .speed(1.0)
                                    .range(0.0..=1000.0),
                                );
                            });
                            ui.end_row();

                            ui.label("Start Delay:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.current_system.start_delay.min)
                                        .prefix("Min:")
                                        .speed(0.1)
                                        .range(0.0..=10.0),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.current_system.start_delay.max)
                                        .prefix("Max:")
                                        .speed(0.1)
                                        .range(0.0..=10.0),
                                );
                            });
                            ui.end_row();

                            ui.label("Max Particles:");
                            ui.add(
                                egui::DragValue::new(&mut self.current_system.max_particles)
                                    .range(1..=100000),
                            );
                            ui.end_row();
                        });

                    ui.add_space(5.0);

                    // Bursts
                    ui.collapsing("[Hit] Bursts", |ui| {
                        if ui.button("+ Add Burst").clicked() {
                            info!(target: "aw_editor::panels::particle_system_panel", system = %self.current_system.name, "Emission burst added");
                            self.current_system.bursts.push(EmissionBurst::default());
                        }

                        for (idx, burst) in self.current_system.bursts.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("#{}", idx + 1));
                                ui.add(
                                    egui::DragValue::new(&mut burst.time)
                                        .prefix("t:")
                                        .speed(0.1),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut burst.count.min)
                                        .prefix("n:")
                                        .speed(1.0)
                                        .range(1.0..=1000.0),
                                );
                            });
                        }
                    });
                });
            });
    }

    fn show_shape_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Sq] Emitter Shape");
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Shape selector
                ui.group(|ui| {
                    ui.label(RichText::new("Shape Type").strong());

                    ui.horizontal_wrapped(|ui| {
                        for shape in EmitterShape::all() {
                            let is_selected = self.current_system.shape == *shape;
                            let button = egui::Button::new(format!("{} {:?}", shape.icon(), shape))
                                .fill(if is_selected {
                                    Color32::from_rgb(60, 100, 160)
                                } else {
                                    Color32::from_rgb(50, 50, 55)
                                });

                            if ui.add(button).clicked() {
                                self.current_system.shape = *shape;
                            }
                        }
                    });
                });

                ui.add_space(10.0);

                // Shape parameters
                ui.group(|ui| {
                    ui.label(RichText::new("Shape Parameters").strong());

                    egui::Grid::new("shape_params")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            match self.current_system.shape {
                                EmitterShape::Sphere
                                | EmitterShape::Hemisphere
                                | EmitterShape::Circle => {
                                    ui.label("Radius:");
                                    ui.add(
                                        egui::DragValue::new(&mut self.current_system.shape_radius)
                                            .speed(0.1)
                                            .range(0.01..=100.0),
                                    );
                                    ui.end_row();
                                }
                                EmitterShape::Cone => {
                                    ui.label("Radius:");
                                    ui.add(
                                        egui::DragValue::new(&mut self.current_system.shape_radius)
                                            .speed(0.1)
                                            .range(0.01..=100.0),
                                    );
                                    ui.end_row();

                                    ui.label("Angle:");
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.current_system.shape_angle,
                                            0.0..=90.0,
                                        )
                                        .suffix("°"),
                                    );
                                    ui.end_row();
                                }
                                _ => {}
                            }

                            if matches!(
                                self.current_system.shape,
                                EmitterShape::Circle | EmitterShape::Cone
                            ) {
                                ui.label("Arc:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.current_system.shape_arc,
                                        0.0..=360.0,
                                    )
                                    .suffix("°"),
                                );
                                ui.end_row();
                            }

                            ui.label("Emit From Edge:");
                            ui.checkbox(&mut self.current_system.emit_from_edge, "");
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Shape preview (simple visual)
                let preview_size = Vec2::new(ui.available_width().min(200.0), 100.0);
                let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());

                let painter = ui.painter();
                painter.rect_filled(rect, 5.0, Color32::from_rgb(30, 30, 35));

                let center = rect.center();
                let shape_color = Color32::from_rgb(100, 150, 255);

                match self.current_system.shape {
                    EmitterShape::Point => {
                        painter.circle_filled(center, 4.0, shape_color);
                    }
                    EmitterShape::Sphere | EmitterShape::Circle => {
                        let r = 30.0 * self.current_system.shape_radius.min(2.0);
                        painter.circle_stroke(center, r, egui::Stroke::new(2.0, shape_color));
                    }
                    EmitterShape::Cone => {
                        let angle = self.current_system.shape_angle.to_radians();
                        let length = 40.0;
                        let width = length * angle.tan();
                        painter.line_segment(
                            [center, egui::Pos2::new(center.x - width, center.y - length)],
                            egui::Stroke::new(2.0, shape_color),
                        );
                        painter.line_segment(
                            [center, egui::Pos2::new(center.x + width, center.y - length)],
                            egui::Stroke::new(2.0, shape_color),
                        );
                    }
                    EmitterShape::Box => {
                        let size = 40.0;
                        painter.rect_stroke(
                            egui::Rect::from_center_size(center, Vec2::splat(size)),
                            0.0,
                            egui::Stroke::new(2.0, shape_color),
                            egui::StrokeKind::Outside,
                        );
                    }
                    _ => {
                        painter.circle_filled(center, 4.0, shape_color);
                    }
                }
            });
    }

    fn show_particles_tab(&mut self, ui: &mut Ui) {
        ui.heading("Particle Properties");
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Start properties
                ui.group(|ui| {
                    ui.label(RichText::new("Start Properties").strong());

                    egui::Grid::new("start_props")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Lifetime:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.current_system.start_lifetime.min,
                                    )
                                    .prefix("Min:")
                                    .speed(0.1)
                                    .range(0.01..=60.0),
                                );
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.current_system.start_lifetime.max,
                                    )
                                    .prefix("Max:")
                                    .speed(0.1)
                                    .range(0.01..=60.0),
                                );
                            });
                            ui.end_row();

                            ui.label("Speed:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.current_system.start_speed.min)
                                        .prefix("Min:")
                                        .speed(0.1)
                                        .range(0.0..=100.0),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.current_system.start_speed.max)
                                        .prefix("Max:")
                                        .speed(0.1)
                                        .range(0.0..=100.0),
                                );
                            });
                            ui.end_row();

                            ui.label("Size:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.current_system.start_size.min)
                                        .prefix("Min:")
                                        .speed(0.01)
                                        .range(0.001..=10.0),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.current_system.start_size.max)
                                        .prefix("Max:")
                                        .speed(0.01)
                                        .range(0.001..=10.0),
                                );
                            });
                            ui.end_row();

                            ui.label("Rotation:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.current_system.start_rotation.min,
                                    )
                                    .prefix("Min:")
                                    .speed(1.0)
                                    .suffix("°"),
                                );
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.current_system.start_rotation.max,
                                    )
                                    .prefix("Max:")
                                    .speed(1.0)
                                    .suffix("°"),
                                );
                            });
                            ui.end_row();

                            ui.label("Color:");
                            let mut color = Color32::from_rgba_unmultiplied(
                                (self.current_system.start_color[0] * 255.0) as u8,
                                (self.current_system.start_color[1] * 255.0) as u8,
                                (self.current_system.start_color[2] * 255.0) as u8,
                                (self.current_system.start_color[3] * 255.0) as u8,
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                self.current_system.start_color = [
                                    color.r() as f32 / 255.0,
                                    color.g() as f32 / 255.0,
                                    color.b() as f32 / 255.0,
                                    color.a() as f32 / 255.0,
                                ];
                            }
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Physics
                ui.group(|ui| {
                    ui.label(RichText::new("[Glb] Physics").strong());

                    egui::Grid::new("particle_physics")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Gravity Modifier:");
                            ui.add(egui::Slider::new(
                                &mut self.current_system.gravity_modifier,
                                -2.0..=2.0,
                            ));
                            ui.end_row();

                            ui.label("Simulation Space:");
                            egui::ComboBox::from_id_salt("sim_space")
                                .selected_text(format!(
                                    "{:?}",
                                    self.current_system.simulation_space
                                ))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.current_system.simulation_space,
                                        SimulationSpace::Local,
                                        "Local",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.simulation_space,
                                        SimulationSpace::World,
                                        "World",
                                    );
                                });
                            ui.end_row();
                        });
                });
            });
    }

    fn show_lifetime_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Time] Over Lifetime");
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Size over lifetime
                ui.group(|ui| {
                    ui.label(RichText::new("📏 Size Over Lifetime").strong());

                    egui::Grid::new("size_lifetime")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Curve:");
                            egui::ComboBox::from_id_salt("size_curve")
                                .selected_text(format!(
                                    "{:?}",
                                    self.current_system.size_over_lifetime
                                ))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.current_system.size_over_lifetime,
                                        CurveType::Constant,
                                        "Constant",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.size_over_lifetime,
                                        CurveType::Linear,
                                        "Linear",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.size_over_lifetime,
                                        CurveType::EaseIn,
                                        "Ease In",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.size_over_lifetime,
                                        CurveType::EaseOut,
                                        "Ease Out",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.size_over_lifetime,
                                        CurveType::EaseInOut,
                                        "Ease In/Out",
                                    );
                                });
                            ui.end_row();

                            ui.label("End Scale:");
                            ui.add(egui::Slider::new(
                                &mut self.current_system.size_curve_end,
                                0.0..=5.0,
                            ));
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Color over lifetime
                ui.group(|ui| {
                    ui.label(RichText::new("Color Over Lifetime").strong());

                    // Display gradient stops
                    let gradient_height = 20.0;
                    let gradient_width = ui.available_width().min(300.0);
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(gradient_width, gradient_height),
                        egui::Sense::hover(),
                    );

                    let painter = ui.painter();

                    // Draw gradient preview
                    for i in 0..100 {
                        let t = i as f32 / 99.0;
                        let x = rect.min.x + t * gradient_width;
                        let color = self.sample_gradient(t);
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::Pos2::new(x, rect.min.y),
                                Vec2::new(gradient_width / 99.0 + 1.0, gradient_height),
                            ),
                            0.0,
                            color,
                        );
                    }

                    ui.add_space(5.0);

                    // Gradient stops
                    for (idx, stop) in self
                        .current_system
                        .color_over_lifetime
                        .stops
                        .iter_mut()
                        .enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.label(format!("Stop {}:", idx + 1));
                            ui.add(
                                egui::Slider::new(&mut stop.position, 0.0..=1.0).show_value(true),
                            );

                            let mut color = Color32::from_rgba_unmultiplied(
                                (stop.color[0] * 255.0) as u8,
                                (stop.color[1] * 255.0) as u8,
                                (stop.color[2] * 255.0) as u8,
                                (stop.color[3] * 255.0) as u8,
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                stop.color = [
                                    color.r() as f32 / 255.0,
                                    color.g() as f32 / 255.0,
                                    color.b() as f32 / 255.0,
                                    color.a() as f32 / 255.0,
                                ];
                            }
                        });
                    }

                    if ui.button("+ Add Stop").clicked() {
                        self.current_system
                            .color_over_lifetime
                            .stops
                            .push(GradientStop {
                                position: 0.5,
                                color: [1.0, 1.0, 1.0, 1.0],
                            });
                    }
                });

                ui.add_space(10.0);

                // Velocity over lifetime
                ui.group(|ui| {
                    ui.label(RichText::new("[Rkt] Velocity Over Lifetime").strong());

                    ui.horizontal(|ui| {
                        ui.label("X:");
                        ui.add(
                            egui::DragValue::new(
                                &mut self.current_system.velocity_over_lifetime[0],
                            )
                            .speed(0.1),
                        );
                        ui.label("Y:");
                        ui.add(
                            egui::DragValue::new(
                                &mut self.current_system.velocity_over_lifetime[1],
                            )
                            .speed(0.1),
                        );
                        ui.label("Z:");
                        ui.add(
                            egui::DragValue::new(
                                &mut self.current_system.velocity_over_lifetime[2],
                            )
                            .speed(0.1),
                        );
                    });
                });

                ui.add_space(10.0);

                // Rotation over lifetime
                ui.group(|ui| {
                    ui.label(RichText::new("Rotation Over Lifetime").strong());

                    ui.horizontal(|ui| {
                        ui.label("Angular Velocity:");
                        ui.add(
                            egui::Slider::new(
                                &mut self.current_system.rotation_over_lifetime,
                                -360.0..=360.0,
                            )
                            .suffix("°/s"),
                        );
                    });
                });
            });
    }

    fn show_rendering_tab(&mut self, ui: &mut Ui) {
        ui.heading("Rendering");
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Render mode
                ui.group(|ui| {
                    ui.label(RichText::new("📺 Render Mode").strong());

                    egui::Grid::new("render_mode")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Mode:");
                            egui::ComboBox::from_id_salt("render_mode")
                                .selected_text(format!("{:?}", self.current_system.render_mode))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.current_system.render_mode,
                                        ParticleRenderMode::Billboard,
                                        "Billboard",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.render_mode,
                                        ParticleRenderMode::StretchedBillboard,
                                        "Stretched Billboard",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.render_mode,
                                        ParticleRenderMode::HorizontalBillboard,
                                        "Horizontal Billboard",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.render_mode,
                                        ParticleRenderMode::VerticalBillboard,
                                        "Vertical Billboard",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.render_mode,
                                        ParticleRenderMode::Mesh,
                                        "Mesh",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.render_mode,
                                        ParticleRenderMode::Trail,
                                        "Trail",
                                    );
                                });
                            ui.end_row();

                            ui.label("Blend Mode:");
                            egui::ComboBox::from_id_salt("blend_mode")
                                .selected_text(format!("{:?}", self.current_system.blend_mode))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.current_system.blend_mode,
                                        ParticleBlendMode::Alpha,
                                        "Alpha",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.blend_mode,
                                        ParticleBlendMode::Additive,
                                        "Additive",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.blend_mode,
                                        ParticleBlendMode::Multiply,
                                        "Multiply",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.blend_mode,
                                        ParticleBlendMode::Premultiply,
                                        "Premultiply",
                                    );
                                });
                            ui.end_row();

                            ui.label("Sort Mode:");
                            egui::ComboBox::from_id_salt("sort_mode")
                                .selected_text(format!("{:?}", self.current_system.sort_mode))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.current_system.sort_mode,
                                        SortMode::None,
                                        "None",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.sort_mode,
                                        SortMode::ByDistance,
                                        "By Distance",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.sort_mode,
                                        SortMode::OldestFirst,
                                        "Oldest First",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_system.sort_mode,
                                        SortMode::YoungestFirst,
                                        "Youngest First",
                                    );
                                });
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Texture
                ui.group(|ui| {
                    ui.label(RichText::new("Texture").strong());

                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        ui.text_edit_singleline(&mut self.current_system.texture_path);
                        if ui.button("[Open]").clicked() {
                            // Open file dialog
                        }
                    });
                });

                ui.add_space(10.0);

                // Shadows
                ui.group(|ui| {
                    ui.label(RichText::new("🌑 Shadows").strong());

                    ui.checkbox(&mut self.current_system.cast_shadows, "Cast Shadows");
                    ui.checkbox(&mut self.current_system.receive_shadows, "Receive Shadows");
                });
            });
    }

    fn show_modules_tab(&mut self, ui: &mut Ui) {
        ui.heading("Modules");
        ui.add_space(10.0);

        // Add module button
        ui.horizontal(|ui| {
            if ui.button("+ Velocity").clicked() {
                info!(target: "aw_editor::panels::particle_system_panel", module = "Velocity", "Particle module added");
                self.current_system.modules.push(EmitterModule {
                    enabled: true,
                    name: "Velocity".to_string(),
                    module_type: ModuleType::Velocity {
                        direction: [0.0, 1.0, 0.0],
                        speed: RangeValue::constant(1.0),
                    },
                });
            }
            if ui.button("+ Force").clicked() {
                info!(target: "aw_editor::panels::particle_system_panel", module = "Force", "Particle module added");
                self.current_system.modules.push(EmitterModule {
                    enabled: true,
                    name: "Force".to_string(),
                    module_type: ModuleType::Force {
                        force: [0.0, 0.0, 0.0],
                        space: SimulationSpace::World,
                    },
                });
            }
            if ui.button("+ Noise").clicked() {
                info!(target: "aw_editor::panels::particle_system_panel", module = "Noise", "Particle module added");
                self.current_system.modules.push(EmitterModule {
                    enabled: true,
                    name: "Noise".to_string(),
                    module_type: ModuleType::Noise {
                        strength: 1.0,
                        frequency: 1.0,
                        scroll_speed: 0.5,
                    },
                });
            }
            if ui.button("+ Collision").clicked() {
                info!(target: "aw_editor::panels::particle_system_panel", module = "Collision", "Particle module added");
                self.current_system.modules.push(EmitterModule {
                    enabled: true,
                    name: "Collision".to_string(),
                    module_type: ModuleType::Collision {
                        bounce: 0.5,
                        lifetime_loss: 0.0,
                        radius_scale: 1.0,
                    },
                });
            }
        });

        ui.add_space(10.0);

        if self.current_system.modules.is_empty() {
            ui.label("No modules added. Click a button above to add a module.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    let mut to_remove = None;

                    for (idx, module) in self.current_system.modules.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut module.enabled, "");
                                ui.label(RichText::new(&module.name).strong());

                                if ui.button("[Del]").clicked() {
                                    info!(target: "aw_editor::panels::particle_system_panel", module_index = idx, module = %module.name, "Particle module removed");
                                    to_remove = Some(idx);
                                }
                            });

                            if module.enabled {
                                match &mut module.module_type {
                                    ModuleType::Velocity { direction, speed } => {
                                        ui.horizontal(|ui| {
                                            ui.label("Dir:");
                                            ui.add(
                                                egui::DragValue::new(&mut direction[0])
                                                    .prefix("X:")
                                                    .speed(0.1),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut direction[1])
                                                    .prefix("Y:")
                                                    .speed(0.1),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut direction[2])
                                                    .prefix("Z:")
                                                    .speed(0.1),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Speed:");
                                            ui.add(
                                                egui::DragValue::new(&mut speed.min)
                                                    .prefix("Min:")
                                                    .speed(0.1),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut speed.max)
                                                    .prefix("Max:")
                                                    .speed(0.1),
                                            );
                                        });
                                    }
                                    ModuleType::Force { force, space } => {
                                        ui.horizontal(|ui| {
                                            ui.label("Force:");
                                            ui.add(
                                                egui::DragValue::new(&mut force[0])
                                                    .prefix("X:")
                                                    .speed(0.1),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut force[1])
                                                    .prefix("Y:")
                                                    .speed(0.1),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut force[2])
                                                    .prefix("Z:")
                                                    .speed(0.1),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Space:");
                                            egui::ComboBox::from_id_salt(format!(
                                                "force_space_{}",
                                                idx
                                            ))
                                            .selected_text(format!("{:?}", space))
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    ui.selectable_value(
                                                        space,
                                                        SimulationSpace::Local,
                                                        "Local",
                                                    );
                                                    ui.selectable_value(
                                                        space,
                                                        SimulationSpace::World,
                                                        "World",
                                                    );
                                                },
                                            );
                                        });
                                    }
                                    ModuleType::Noise {
                                        strength,
                                        frequency,
                                        scroll_speed,
                                    } => {
                                        ui.horizontal(|ui| {
                                            ui.label("Strength:");
                                            ui.add(egui::Slider::new(strength, 0.0..=5.0));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Frequency:");
                                            ui.add(egui::Slider::new(frequency, 0.1..=10.0));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Scroll:");
                                            ui.add(egui::Slider::new(scroll_speed, 0.0..=5.0));
                                        });
                                    }
                                    ModuleType::Collision {
                                        bounce,
                                        lifetime_loss,
                                        radius_scale,
                                    } => {
                                        ui.horizontal(|ui| {
                                            ui.label("Bounce:");
                                            ui.add(egui::Slider::new(bounce, 0.0..=1.0));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Lifetime Loss:");
                                            ui.add(egui::Slider::new(lifetime_loss, 0.0..=1.0));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Radius Scale:");
                                            ui.add(egui::Slider::new(radius_scale, 0.1..=2.0));
                                        });
                                    }
                                    _ => {
                                        ui.label("(Module parameters)");
                                    }
                                }
                            }
                        });
                    }

                    if let Some(idx) = to_remove {
                        self.current_system.modules.remove(idx);
                    }
                });
        }
    }

    fn show_presets_tab(&mut self, ui: &mut Ui) {
        ui.heading("Presets");
        ui.add_space(10.0);

        // Filter
        ui.horizontal(|ui| {
            ui.label("[Srch]");
            ui.text_edit_singleline(&mut self.preset_filter);
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                let mut current_category = String::new();

                for preset in &self.presets {
                    if !self.preset_filter.is_empty()
                        && !preset
                            .name
                            .to_lowercase()
                            .contains(&self.preset_filter.to_lowercase())
                    {
                        continue;
                    }

                    if preset.category != current_category {
                        current_category = preset.category.clone();
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(&current_category)
                                .strong()
                                .color(Color32::from_rgb(150, 150, 200)),
                        );
                    }

                    ui.horizontal(|ui| {
                        ui.label(&preset.name);
                        ui.label(
                            RichText::new(&preset.description)
                                .small()
                                .color(Color32::GRAY),
                        );

                        if ui.button("Apply").clicked() {
                            info!(target: "aw_editor::panels::particle_system_panel", "Particle preset applied");
                            // Apply preset configuration
                        }
                    });
                }
            });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("Save as Preset").clicked() {
                info!(target: "aw_editor::panels::particle_system_panel", system = %self.current_system.name, "Particle system saved as preset");
                // Save current system as preset
            }
        });
    }

    fn show_stats_tab(&mut self, ui: &mut Ui) {
        ui.heading("Performance Stats");
        ui.add_space(10.0);

        // Live stats
        ui.group(|ui| {
            ui.label(RichText::new("📈 Live Statistics").strong());

            egui::Grid::new("live_stats")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Active Particles:");
                    ui.label(RichText::new(format!("{}", self.stats.active_particles)).monospace());
                    ui.end_row();

                    ui.label("Total Emitters:");
                    ui.label(RichText::new(format!("{}", self.stats.total_emitters)).monospace());
                    ui.end_row();

                    ui.label("Draw Calls:");
                    ui.label(RichText::new(format!("{}", self.stats.draw_calls)).monospace());
                    ui.end_row();

                    ui.label("GPU Memory:");
                    ui.label(
                        RichText::new(format!("{:.1} MB", self.stats.gpu_memory_mb)).monospace(),
                    );
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Timing
        ui.group(|ui| {
            ui.label(RichText::new("[Time] Timing").strong());

            egui::Grid::new("timing_stats")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Simulation:");
                    let sim_color = if self.stats.simulation_time_ms < 1.0 {
                        Color32::GREEN
                    } else {
                        Color32::YELLOW
                    };
                    ui.label(
                        RichText::new(format!("{:.2} ms", self.stats.simulation_time_ms))
                            .monospace()
                            .color(sim_color),
                    );
                    ui.end_row();

                    ui.label("Rendering:");
                    let render_color = if self.stats.render_time_ms < 2.0 {
                        Color32::GREEN
                    } else {
                        Color32::YELLOW
                    };
                    ui.label(
                        RichText::new(format!("{:.2} ms", self.stats.render_time_ms))
                            .monospace()
                            .color(render_color),
                    );
                    ui.end_row();

                    ui.label("Total:");
                    let total = self.stats.simulation_time_ms + self.stats.render_time_ms;
                    let total_color = if total < 3.0 {
                        Color32::GREEN
                    } else if total < 5.0 {
                        Color32::YELLOW
                    } else {
                        Color32::RED
                    };
                    ui.label(
                        RichText::new(format!("{:.2} ms", total))
                            .monospace()
                            .color(total_color),
                    );
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Current system stats
        ui.group(|ui| {
            ui.label(RichText::new("Current System").strong());

            ui.label(format!("Name: {}", self.current_system.name));
            ui.label(format!(
                "Max Particles: {}",
                self.current_system.max_particles
            ));
            ui.label(format!(
                "Emission Rate: {:.0}-{:.0}/s",
                self.current_system.emission_rate.min, self.current_system.emission_rate.max
            ));
            ui.label(format!("Modules: {}", self.current_system.modules.len()));
            ui.label(format!("Bursts: {}", self.current_system.bursts.len()));
        });
    }

    fn sample_gradient(&self, t: f32) -> Color32 {
        let stops = &self.current_system.color_over_lifetime.stops;
        if stops.is_empty() {
            return Color32::WHITE;
        }
        if stops.len() == 1 {
            let c = &stops[0].color;
            return Color32::from_rgba_unmultiplied(
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                (c[3] * 255.0) as u8,
            );
        }

        // Find surrounding stops
        let mut left = &stops[0];
        let mut right = &stops[stops.len() - 1];

        for i in 0..stops.len() - 1 {
            if stops[i].position <= t && stops[i + 1].position >= t {
                left = &stops[i];
                right = &stops[i + 1];
                break;
            }
        }

        // Interpolate
        let range = right.position - left.position;
        let local_t = if range > 0.001 {
            (t - left.position) / range
        } else {
            0.0
        };

        let r = left.color[0] + (right.color[0] - left.color[0]) * local_t;
        let g = left.color[1] + (right.color[1] - left.color[1]) * local_t;
        let b = left.color[2] + (right.color[2] - left.color[2]) * local_t;
        let a = left.color[3] + (right.color[3] - left.color[3]) * local_t;

        Color32::from_rgba_unmultiplied(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            (a * 255.0) as u8,
        )
    }

    // Getters for testing
    pub fn system_count(&self) -> usize {
        self.particle_systems.len()
    }

    pub fn preset_count(&self) -> usize {
        self.presets.len()
    }

    pub fn current_system_name(&self) -> &str {
        &self.current_system.name
    }

    pub fn module_count(&self) -> usize {
        self.current_system.modules.len()
    }

    pub fn add_system(&mut self, name: &str) -> u32 {
        let id = self.next_id();
        self.particle_systems.push(ParticleSystem {
            id,
            name: name.to_string(),
            ..Default::default()
        });
        id
    }

    pub fn set_emission_rate(&mut self, min: f32, max: f32) {
        self.current_system.emission_rate = RangeValue::range(min, max);
    }

    pub fn set_max_particles(&mut self, count: u32) {
        self.current_system.max_particles = count;
    }
}

impl Panel for ParticleSystemPanel {
    fn name(&self) -> &'static str {
        "Particle System"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            ParticleTab::Emitter => self.show_emitter_tab(ui),
            ParticleTab::Shape => self.show_shape_tab(ui),
            ParticleTab::Particles => self.show_particles_tab(ui),
            ParticleTab::Lifetime => self.show_lifetime_tab(ui),
            ParticleTab::Rendering => self.show_rendering_tab(ui),
            ParticleTab::Modules => self.show_modules_tab(ui),
            ParticleTab::Presets => self.show_presets_tab(ui),
            ParticleTab::Stats => self.show_stats_tab(ui),
        }
    }

    fn update(&mut self) {
        if self.preview_playing {
            self.preview_time += 0.016 * self.preview_speed; // ~60 FPS
            if self.current_system.looping && self.preview_time > self.current_system.duration {
                self.preview_time = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_system_panel_creation() {
        let panel = ParticleSystemPanel::new();
        assert!(!panel.current_system_name().is_empty());
    }

    #[test]
    fn test_default_sample_data() {
        let mut panel = ParticleSystemPanel::new();
        panel.create_sample_data();
        assert!(panel.system_count() >= 3);
        assert!(panel.preset_count() >= 10);
    }

    #[test]
    fn test_add_system() {
        let mut panel = ParticleSystemPanel::new();
        let initial_count = panel.system_count();

        let id = panel.add_system("Test VFX");
        assert!(id > 0);
        assert_eq!(panel.system_count(), initial_count + 1);
    }

    #[test]
    fn test_set_emission_rate() {
        let mut panel = ParticleSystemPanel::new();
        panel.set_emission_rate(50.0, 100.0);
        assert!((panel.current_system.emission_rate.min - 50.0).abs() < 0.001);
        assert!((panel.current_system.emission_rate.max - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_set_max_particles() {
        let mut panel = ParticleSystemPanel::new();
        panel.set_max_particles(5000);
        assert_eq!(panel.current_system.max_particles, 5000);
    }

    #[test]
    fn test_emitter_shape_icons() {
        assert_eq!(EmitterShape::Point.icon(), "•");
        assert_eq!(EmitterShape::Sphere.icon(), "⚪");
        assert_eq!(EmitterShape::Cone.icon(), "▲");
    }

    #[test]
    fn test_range_value() {
        let constant = RangeValue::constant(5.0);
        assert!((constant.min - 5.0).abs() < 0.001);
        assert!((constant.max - 5.0).abs() < 0.001);

        let range = RangeValue::range(1.0, 10.0);
        assert!((range.min - 1.0).abs() < 0.001);
        assert!((range.max - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_color_gradient_default() {
        let gradient = ColorGradient::default();
        assert_eq!(gradient.stops.len(), 2);
        assert!((gradient.stops[0].position - 0.0).abs() < 0.001);
        assert!((gradient.stops[1].position - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gradient_sampling() {
        let panel = ParticleSystemPanel::new();
        let color_start = panel.sample_gradient(0.0);
        let color_end = panel.sample_gradient(1.0);

        // Start should be opaque white
        assert_eq!(color_start.a(), 255);
        // End should be transparent
        assert_eq!(color_end.a(), 0);
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = ParticleSystemPanel::new();
        assert_eq!(panel.name(), "Particle System");
    }

    // ========== EmitterShape Tests ==========

    #[test]
    fn test_emitter_shape_display() {
        for shape in EmitterShape::all() {
            let display = format!("{}", shape);
            assert!(display.contains(shape.name()));
        }
    }

    #[test]
    fn test_emitter_shape_all_variants() {
        let all = EmitterShape::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&EmitterShape::Point));
        assert!(all.contains(&EmitterShape::Mesh));
    }

    #[test]
    fn test_emitter_shape_hash() {
        use std::collections::HashSet;
        let set: HashSet<EmitterShape> = EmitterShape::all().iter().copied().collect();
        assert_eq!(set.len(), 8);
    }

    #[test]
    fn test_emitter_shape_is_volumetric() {
        assert!(!EmitterShape::Point.is_volumetric());
        assert!(EmitterShape::Sphere.is_volumetric());
        assert!(EmitterShape::Hemisphere.is_volumetric());
        assert!(EmitterShape::Cone.is_volumetric());
        assert!(EmitterShape::Box.is_volumetric());
        assert!(!EmitterShape::Circle.is_volumetric());
        assert!(!EmitterShape::Edge.is_volumetric());
        assert!(!EmitterShape::Mesh.is_volumetric());
    }

    // ========== SimulationSpace Tests ==========

    #[test]
    fn test_simulation_space_display() {
        for space in SimulationSpace::all() {
            let display = format!("{}", space);
            assert!(display.contains(space.name()));
        }
    }

    #[test]
    fn test_simulation_space_all_variants() {
        let all = SimulationSpace::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&SimulationSpace::Local));
        assert!(all.contains(&SimulationSpace::World));
    }

    #[test]
    fn test_simulation_space_hash() {
        use std::collections::HashSet;
        let set: HashSet<SimulationSpace> = SimulationSpace::all().iter().copied().collect();
        assert_eq!(set.len(), 2);
    }

    // ========== ParticleBlendMode Tests ==========

    #[test]
    fn test_particle_blend_mode_display() {
        for mode in ParticleBlendMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_particle_blend_mode_all_variants() {
        let all = ParticleBlendMode::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&ParticleBlendMode::Alpha));
        assert!(all.contains(&ParticleBlendMode::Additive));
    }

    #[test]
    fn test_particle_blend_mode_hash() {
        use std::collections::HashSet;
        let set: HashSet<ParticleBlendMode> = ParticleBlendMode::all().iter().copied().collect();
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_particle_blend_mode_is_additive() {
        assert!(!ParticleBlendMode::Alpha.is_additive());
        assert!(ParticleBlendMode::Additive.is_additive());
        assert!(!ParticleBlendMode::Multiply.is_additive());
        assert!(!ParticleBlendMode::Premultiply.is_additive());
    }

    // ========== ParticleRenderMode Tests ==========

    #[test]
    fn test_particle_render_mode_display() {
        for mode in ParticleRenderMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_particle_render_mode_all_variants() {
        let all = ParticleRenderMode::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&ParticleRenderMode::Billboard));
        assert!(all.contains(&ParticleRenderMode::Trail));
    }

    #[test]
    fn test_particle_render_mode_hash() {
        use std::collections::HashSet;
        let set: HashSet<ParticleRenderMode> = ParticleRenderMode::all().iter().copied().collect();
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn test_particle_render_mode_is_billboard() {
        assert!(ParticleRenderMode::Billboard.is_billboard());
        assert!(ParticleRenderMode::StretchedBillboard.is_billboard());
        assert!(ParticleRenderMode::HorizontalBillboard.is_billboard());
        assert!(ParticleRenderMode::VerticalBillboard.is_billboard());
        assert!(!ParticleRenderMode::Mesh.is_billboard());
        assert!(!ParticleRenderMode::Trail.is_billboard());
    }

    // ========== CurveType Tests ==========

    #[test]
    fn test_curve_type_display() {
        for curve in CurveType::all() {
            let display = format!("{}", curve);
            assert!(display.contains(curve.name()));
        }
    }

    #[test]
    fn test_curve_type_all_variants() {
        let all = CurveType::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&CurveType::Constant));
        assert!(all.contains(&CurveType::Curve));
    }

    #[test]
    fn test_curve_type_hash() {
        use std::collections::HashSet;
        let set: HashSet<CurveType> = CurveType::all().iter().copied().collect();
        assert_eq!(set.len(), 7);
    }

    #[test]
    fn test_curve_type_is_easing() {
        assert!(!CurveType::Constant.is_easing());
        assert!(!CurveType::Linear.is_easing());
        assert!(CurveType::EaseIn.is_easing());
        assert!(CurveType::EaseOut.is_easing());
        assert!(CurveType::EaseInOut.is_easing());
        assert!(!CurveType::Random.is_easing());
        assert!(!CurveType::Curve.is_easing());
    }

    // ========== SubEmitterEvent Tests ==========

    #[test]
    fn test_sub_emitter_event_display() {
        for event in SubEmitterEvent::all() {
            let display = format!("{}", event);
            assert!(display.contains(event.name()));
        }
    }

    #[test]
    fn test_sub_emitter_event_all_variants() {
        let all = SubEmitterEvent::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&SubEmitterEvent::Birth));
        assert!(all.contains(&SubEmitterEvent::Death));
        assert!(all.contains(&SubEmitterEvent::Collision));
        assert!(all.contains(&SubEmitterEvent::Trigger));
    }

    #[test]
    fn test_sub_emitter_event_hash() {
        use std::collections::HashSet;
        let set: HashSet<SubEmitterEvent> = SubEmitterEvent::all().iter().copied().collect();
        assert_eq!(set.len(), 4);
    }

    // ========== SortMode Tests ==========

    #[test]
    fn test_sort_mode_display() {
        for mode in SortMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_sort_mode_all_variants() {
        let all = SortMode::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&SortMode::None));
        assert!(all.contains(&SortMode::YoungestFirst));
    }

    #[test]
    fn test_sort_mode_hash() {
        use std::collections::HashSet;
        let set: HashSet<SortMode> = SortMode::all().iter().copied().collect();
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_sort_mode_is_sorted() {
        assert!(!SortMode::None.is_sorted());
        assert!(SortMode::ByDistance.is_sorted());
        assert!(SortMode::OldestFirst.is_sorted());
        assert!(SortMode::YoungestFirst.is_sorted());
    }

    // ========== ParticleTab Tests ==========

    #[test]
    fn test_particle_tab_display() {
        for tab in ParticleTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()));
        }
    }

    #[test]
    fn test_particle_tab_all_variants() {
        let all = ParticleTab::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&ParticleTab::Emitter));
        assert!(all.contains(&ParticleTab::Stats));
    }

    #[test]
    fn test_particle_tab_hash() {
        use std::collections::HashSet;
        let set: HashSet<ParticleTab> = ParticleTab::all().iter().copied().collect();
        assert_eq!(set.len(), 8);
    }

    #[test]
    fn test_particle_tab_default() {
        assert_eq!(ParticleTab::default(), ParticleTab::Emitter);
    }

    // ========== ModuleType Tests ==========

    #[test]
    fn test_module_type_display() {
        let velocity = ModuleType::Velocity {
            direction: [1.0, 0.0, 0.0],
            speed: RangeValue::constant(5.0),
        };
        let display = format!("{}", velocity);
        assert!(display.contains(velocity.name()));
    }

    #[test]
    fn test_module_type_all_variants() {
        let all = ModuleType::all_variants();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&"Velocity"));
        assert!(all.contains(&"Force"));
        assert!(all.contains(&"Gravity"));
        assert!(all.contains(&"Noise"));
        assert!(all.contains(&"Collision"));
        assert!(all.contains(&"SubEmitter"));
        assert!(all.contains(&"TextureAnimation"));
        assert!(all.contains(&"Trail"));
        assert!(all.contains(&"Light"));
        assert!(all.contains(&"Rotation"));
    }

    #[test]
    fn test_module_type_names() {
        let gravity = ModuleType::Gravity { multiplier: 1.0 };
        assert_eq!(gravity.name(), "Gravity");

        let sub_emitter = ModuleType::SubEmitter {
            event: SubEmitterEvent::Birth,
            emitter_id: 1,
        };
        assert_eq!(sub_emitter.name(), "Sub Emitter");

        let tex_anim = ModuleType::TextureAnimation {
            tiles_x: 4,
            tiles_y: 4,
            fps: 30.0,
        };
        assert_eq!(tex_anim.name(), "Texture Animation");
    }

    #[test]
    fn test_module_type_icons() {
        let velocity = ModuleType::Velocity {
            direction: [1.0, 0.0, 0.0],
            speed: RangeValue::constant(5.0),
        };
        assert_eq!(velocity.icon(), "[Rt]");

        let light = ModuleType::Light {
            color: [1.0, 1.0, 1.0],
            intensity: RangeValue::constant(1.0),
            range: RangeValue::constant(10.0),
        };
        assert_eq!(light.icon(), "[Lgt]");

        let trail = ModuleType::Trail {
            width: RangeValue::constant(0.1),
            lifetime: 1.0,
            min_vertex_distance: 0.1,
        };
        assert_eq!(trail.icon(), "[Fx]");
    }

    #[test]
    fn test_module_type_is_physics() {
        // Physics-related modules
        let velocity = ModuleType::Velocity {
            direction: [1.0, 0.0, 0.0],
            speed: RangeValue::constant(5.0),
        };
        assert!(velocity.is_physics());

        let force = ModuleType::Force {
            force: [0.0, 1.0, 0.0],
            space: SimulationSpace::World,
        };
        assert!(force.is_physics());

        let gravity = ModuleType::Gravity { multiplier: 1.0 };
        assert!(gravity.is_physics());

        let collision = ModuleType::Collision {
            bounce: 0.5,
            lifetime_loss: 0.1,
            radius_scale: 1.0,
        };
        assert!(collision.is_physics());

        let rotation = ModuleType::Rotation {
            speed: RangeValue::constant(90.0),
            random_start: false,
        };
        assert!(rotation.is_physics());

        // Non-physics modules
        let light = ModuleType::Light {
            color: [1.0, 1.0, 1.0],
            intensity: RangeValue::constant(1.0),
            range: RangeValue::constant(10.0),
        };
        assert!(!light.is_physics());
    }

    #[test]
    fn test_module_type_is_visual() {
        // Visual-related modules
        let tex_anim = ModuleType::TextureAnimation {
            tiles_x: 4,
            tiles_y: 4,
            fps: 30.0,
        };
        assert!(tex_anim.is_visual());

        let trail = ModuleType::Trail {
            width: RangeValue::constant(0.1),
            lifetime: 1.0,
            min_vertex_distance: 0.1,
        };
        assert!(trail.is_visual());

        let light = ModuleType::Light {
            color: [1.0, 1.0, 1.0],
            intensity: RangeValue::constant(1.0),
            range: RangeValue::constant(10.0),
        };
        assert!(light.is_visual());

        // Non-visual modules
        let gravity = ModuleType::Gravity { multiplier: 1.0 };
        assert!(!gravity.is_visual());
    }

    #[test]
    fn test_module_type_is_spawner() {
        let sub_emitter = ModuleType::SubEmitter {
            event: SubEmitterEvent::Death,
            emitter_id: 2,
        };
        assert!(sub_emitter.is_spawner());

        let velocity = ModuleType::Velocity {
            direction: [1.0, 0.0, 0.0],
            speed: RangeValue::constant(5.0),
        };
        assert!(!velocity.is_spawner());

        let gravity = ModuleType::Gravity { multiplier: 1.0 };
        assert!(!gravity.is_spawner());
    }

    #[test]
    fn test_module_type_display_all_variants() {
        // Test that all variant types can be displayed
        let modules: Vec<ModuleType> = vec![
            ModuleType::Velocity {
                direction: [1.0, 0.0, 0.0],
                speed: RangeValue::constant(5.0),
            },
            ModuleType::Force {
                force: [0.0, 1.0, 0.0],
                space: SimulationSpace::World,
            },
            ModuleType::Gravity { multiplier: 9.8 },
            ModuleType::Noise {
                strength: 1.0,
                frequency: 2.0,
                scroll_speed: 0.5,
            },
            ModuleType::Collision {
                bounce: 0.5,
                lifetime_loss: 0.1,
                radius_scale: 1.0,
            },
            ModuleType::SubEmitter {
                event: SubEmitterEvent::Birth,
                emitter_id: 1,
            },
            ModuleType::TextureAnimation {
                tiles_x: 4,
                tiles_y: 4,
                fps: 30.0,
            },
            ModuleType::Trail {
                width: RangeValue::constant(0.1),
                lifetime: 1.0,
                min_vertex_distance: 0.1,
            },
            ModuleType::Light {
                color: [1.0, 1.0, 1.0],
                intensity: RangeValue::constant(1.0),
                range: RangeValue::constant(10.0),
            },
            ModuleType::Rotation {
                speed: RangeValue::constant(90.0),
                random_start: true,
            },
        ];

        for module in &modules {
            let display = format!("{}", module);
            assert!(display.contains(module.name()));
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_module_type_partial_eq() {
        let g1 = ModuleType::Gravity { multiplier: 1.0 };
        let g2 = ModuleType::Gravity { multiplier: 1.0 };
        let g3 = ModuleType::Gravity { multiplier: 2.0 };

        assert_eq!(g1, g2);
        assert_ne!(g1, g3);
    }

    // ============================================================
    // PARTICLE ACTION TESTS
    // ============================================================

    #[test]
    fn test_action_system_initial_state() {
        let panel = ParticleSystemPanel::default();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_action_queue_and_take() {
        let mut panel = ParticleSystemPanel::default();
        panel.queue_action(ParticleAction::Play);
        panel.queue_action(ParticleAction::SetSpeed(2.0));

        assert!(panel.has_pending_actions());

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());

        // Verify actions were drained
        let empty = panel.take_actions();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_action_names() {
        let actions = vec![
            (ParticleAction::Play, "Play"),
            (ParticleAction::Pause, "Pause"),
            (ParticleAction::Stop, "Stop"),
            (ParticleAction::Restart, "Restart"),
            (ParticleAction::SetSpeed(1.5), "Set Speed"),
            (
                ParticleAction::CreateSystem {
                    name: "test".to_string(),
                },
                "Create System",
            ),
            (ParticleAction::DeleteSystem { id: 1 }, "Delete System"),
            (
                ParticleAction::DuplicateSystem { id: 1 },
                "Duplicate System",
            ),
            (ParticleAction::SelectSystem { id: 1 }, "Select System"),
            (
                ParticleAction::AddBurst {
                    time: 0.0,
                    count: 10,
                },
                "Add Burst",
            ),
            (ParticleAction::RemoveBurst { index: 0 }, "Remove Burst"),
            (
                ParticleAction::AddModule {
                    module_type: ModuleType::Gravity { multiplier: 1.0 },
                },
                "Add Module",
            ),
            (ParticleAction::RemoveModule { index: 0 }, "Remove Module"),
            (
                ParticleAction::ApplyPreset {
                    preset_name: "fire".to_string(),
                },
                "Apply Preset",
            ),
            (
                ParticleAction::SaveAsPreset {
                    name: "custom".to_string(),
                },
                "Save As Preset",
            ),
            (ParticleAction::SetShape(EmitterShape::Sphere), "Set Shape"),
        ];

        for (action, expected_name) in actions {
            assert_eq!(action.name(), expected_name);
        }
    }

    #[test]
    fn test_action_is_playback() {
        assert!(ParticleAction::Play.is_playback());
        assert!(ParticleAction::Pause.is_playback());
        assert!(ParticleAction::Stop.is_playback());
        assert!(ParticleAction::Restart.is_playback());
        assert!(ParticleAction::SetSpeed(1.0).is_playback());

        assert!(!ParticleAction::CreateSystem {
            name: "test".to_string()
        }
        .is_playback());
        assert!(!ParticleAction::DeleteSystem { id: 1 }.is_playback());
        assert!(!ParticleAction::AddModule {
            module_type: ModuleType::Gravity { multiplier: 1.0 }
        }
        .is_playback());
    }

    #[test]
    fn test_action_is_system_management() {
        assert!(ParticleAction::CreateSystem {
            name: "test".to_string()
        }
        .is_system_management());
        assert!(ParticleAction::DeleteSystem { id: 1 }.is_system_management());
        assert!(ParticleAction::DuplicateSystem { id: 1 }.is_system_management());
        assert!(ParticleAction::SelectSystem { id: 1 }.is_system_management());

        assert!(!ParticleAction::Play.is_system_management());
        assert!(!ParticleAction::AddModule {
            module_type: ModuleType::Gravity { multiplier: 1.0 }
        }
        .is_system_management());
    }

    #[test]
    fn test_action_is_config_change() {
        assert!(ParticleAction::AddBurst {
            time: 0.0,
            count: 10
        }
        .is_config_change());
        assert!(ParticleAction::RemoveBurst { index: 0 }.is_config_change());
        assert!(ParticleAction::AddModule {
            module_type: ModuleType::Gravity { multiplier: 1.0 }
        }
        .is_config_change());
        assert!(ParticleAction::RemoveModule { index: 0 }.is_config_change());
        assert!(ParticleAction::SetShape(EmitterShape::Cone).is_config_change());

        assert!(!ParticleAction::Play.is_config_change());
        assert!(!ParticleAction::CreateSystem {
            name: "test".to_string()
        }
        .is_config_change());
    }

    #[test]
    fn test_action_display() {
        let action = ParticleAction::Play;
        let display = format!("{}", action);
        assert_eq!(display, "Play");

        let action = ParticleAction::SetSpeed(2.5);
        let display = format!("{}", action);
        assert_eq!(display, "Set Speed");
    }

    #[test]
    fn test_preview_speed_initial() {
        let panel = ParticleSystemPanel::default();
        assert!((panel.preview_speed() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_is_playing_initial() {
        let panel = ParticleSystemPanel::default();
        assert!(!panel.is_playing());
    }

    #[test]
    fn test_preview_time_initial() {
        let panel = ParticleSystemPanel::default();
        assert!((panel.preview_time()).abs() < 0.001);
    }

    #[test]
    fn test_selected_system_with_sample_data() {
        let mut panel = ParticleSystemPanel::default();
        panel.create_sample_data();
        // Default panel has sample data, so a system should be selected
        assert!(panel.system_count() > 0);
    }

    #[test]
    fn test_multiple_playback_actions() {
        let mut panel = ParticleSystemPanel::default();
        panel.queue_action(ParticleAction::Play);
        panel.queue_action(ParticleAction::SetSpeed(2.0));
        panel.queue_action(ParticleAction::Restart);

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.iter().all(|a| a.is_playback()));
    }

    #[test]
    fn test_multiple_system_management_actions() {
        let mut panel = ParticleSystemPanel::default();
        panel.queue_action(ParticleAction::CreateSystem {
            name: "new_system".to_string(),
        });
        panel.queue_action(ParticleAction::SelectSystem { id: 1 });
        panel.queue_action(ParticleAction::DuplicateSystem { id: 1 });

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.iter().all(|a| a.is_system_management()));
    }

    #[test]
    fn test_mixed_action_types() {
        let mut panel = ParticleSystemPanel::default();
        panel.queue_action(ParticleAction::Play);
        panel.queue_action(ParticleAction::CreateSystem {
            name: "effect".to_string(),
        });
        panel.queue_action(ParticleAction::AddModule {
            module_type: ModuleType::Gravity { multiplier: -1.0 },
        });

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 3);

        // Check categories
        let playback_count = actions.iter().filter(|a| a.is_playback()).count();
        let management_count = actions.iter().filter(|a| a.is_system_management()).count();
        let config_count = actions.iter().filter(|a| a.is_config_change()).count();

        assert_eq!(playback_count, 1);
        assert_eq!(management_count, 1);
        assert_eq!(config_count, 1);
    }

    #[test]
    fn test_action_partial_eq() {
        let a1 = ParticleAction::Play;
        let a2 = ParticleAction::Play;
        let a3 = ParticleAction::Pause;

        assert_eq!(a1, a2);
        assert_ne!(a1, a3);

        let s1 = ParticleAction::SetSpeed(1.0);
        let s2 = ParticleAction::SetSpeed(1.0);
        let s3 = ParticleAction::SetSpeed(2.0);

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }
}
