//! Physics Debug Panel for the editor UI
//!
//! Provides comprehensive debugging and configuration for physics systems:
//! - Collider visualization settings
//! - Ragdoll configuration and testing
//! - Vehicle physics tuning
//! - Cloth simulation parameters
//! - Destruction system configuration
//! - Gravity zones and environment effects
//! - Physics statistics and profiling

use egui::{Color32, RichText, Ui, Vec2};
use std::collections::VecDeque;
use tracing::info;

use crate::panels::Panel;

/// Physics visualization modes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PhysicsVisualization {
    #[default]
    None,
    Colliders,
    ContactPoints,
    Joints,
    VelocityVectors,
    BroadPhase,
    Islands,
    All,
}

impl std::fmt::Display for PhysicsVisualization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl PhysicsVisualization {
    pub fn all() -> &'static [PhysicsVisualization] {
        &[
            PhysicsVisualization::None,
            PhysicsVisualization::Colliders,
            PhysicsVisualization::ContactPoints,
            PhysicsVisualization::Joints,
            PhysicsVisualization::VelocityVectors,
            PhysicsVisualization::BroadPhase,
            PhysicsVisualization::Islands,
            PhysicsVisualization::All,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PhysicsVisualization::None => "None",
            PhysicsVisualization::Colliders => "Colliders",
            PhysicsVisualization::ContactPoints => "Contact Points",
            PhysicsVisualization::Joints => "Joints",
            PhysicsVisualization::VelocityVectors => "Velocity Vectors",
            PhysicsVisualization::BroadPhase => "Broad Phase",
            PhysicsVisualization::Islands => "Islands",
            PhysicsVisualization::All => "All",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PhysicsVisualization::None => "⚫",
            PhysicsVisualization::Colliders => "[Pkg]",
            PhysicsVisualization::ContactPoints => "📍",
            PhysicsVisualization::Joints => "[Link]",
            PhysicsVisualization::VelocityVectors => "[Rt]",
            PhysicsVisualization::BroadPhase => "[Files]",
            PhysicsVisualization::Islands => "🏝️",
            PhysicsVisualization::All => "👁️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PhysicsVisualization::None => "No debug visualization",
            PhysicsVisualization::Colliders => "Show collider wireframes",
            PhysicsVisualization::ContactPoints => "Show contact points and normals",
            PhysicsVisualization::Joints => "Show joint connections and limits",
            PhysicsVisualization::VelocityVectors => "Show linear/angular velocity",
            PhysicsVisualization::BroadPhase => "Show spatial partitioning grid",
            PhysicsVisualization::Islands => "Color bodies by simulation island",
            PhysicsVisualization::All => "Show all debug visualizations",
        }
    }

    pub fn is_visible(&self) -> bool {
        !matches!(self, PhysicsVisualization::None)
    }
}

/// Collision layer definition
#[derive(Debug, Clone)]
pub struct CollisionLayer {
    pub id: u32,
    pub name: String,
    pub color: Color32,
    pub enabled: bool,
}

/// Physics simulation mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SimulationMode {
    #[default]
    Running,
    Paused,
    StepOnce,
    SlowMotion,
}

impl std::fmt::Display for SimulationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SimulationMode {
    pub fn all() -> &'static [SimulationMode] {
        &[
            SimulationMode::Running,
            SimulationMode::Paused,
            SimulationMode::StepOnce,
            SimulationMode::SlowMotion,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SimulationMode::Running => "Running",
            SimulationMode::Paused => "Paused",
            SimulationMode::StepOnce => "Step Once",
            SimulationMode::SlowMotion => "Slow Motion",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SimulationMode::Running => ">",
            SimulationMode::Paused => "||",
            SimulationMode::StepOnce => ">|",
            SimulationMode::SlowMotion => "🐌",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, SimulationMode::Running | SimulationMode::SlowMotion)
    }

    pub fn time_scale(&self) -> f32 {
        match self {
            SimulationMode::Running => 1.0,
            SimulationMode::Paused => 0.0,
            SimulationMode::StepOnce => 0.0,
            SimulationMode::SlowMotion => 0.2,
        }
    }
}

/// Ragdoll preset configuration
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RagdollPreset {
    #[default]
    Humanoid,
    Quadruped,
    Bird,
    Insect,
    Custom,
}

impl std::fmt::Display for RagdollPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl RagdollPreset {
    pub fn all() -> &'static [RagdollPreset] {
        &[
            RagdollPreset::Humanoid,
            RagdollPreset::Quadruped,
            RagdollPreset::Bird,
            RagdollPreset::Insect,
            RagdollPreset::Custom,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            RagdollPreset::Humanoid => "Humanoid",
            RagdollPreset::Quadruped => "Quadruped",
            RagdollPreset::Bird => "Bird",
            RagdollPreset::Insect => "Insect",
            RagdollPreset::Custom => "Custom",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            RagdollPreset::Humanoid => "🚶",
            RagdollPreset::Quadruped => "🐕",
            RagdollPreset::Bird => "🦅",
            RagdollPreset::Insect => "🐛",
            RagdollPreset::Custom => "⚙️",
        }
    }

    pub fn bone_count(&self) -> usize {
        match self {
            RagdollPreset::Humanoid => 15,
            RagdollPreset::Quadruped => 18,
            RagdollPreset::Bird => 12,
            RagdollPreset::Insect => 8,
            RagdollPreset::Custom => 0,
        }
    }
}

/// Vehicle type presets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VehicleType {
    #[default]
    Sedan,
    SportsCar,
    Truck,
    Motorcycle,
    Tank,
    Custom,
}

impl std::fmt::Display for VehicleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl VehicleType {
    pub fn all() -> &'static [VehicleType] {
        &[
            VehicleType::Sedan,
            VehicleType::SportsCar,
            VehicleType::Truck,
            VehicleType::Motorcycle,
            VehicleType::Tank,
            VehicleType::Custom,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            VehicleType::Sedan => "Sedan",
            VehicleType::SportsCar => "Sports Car",
            VehicleType::Truck => "Truck",
            VehicleType::Motorcycle => "Motorcycle",
            VehicleType::Tank => "Tank",
            VehicleType::Custom => "Custom",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            VehicleType::Sedan => "🚗",
            VehicleType::SportsCar => "🏎️",
            VehicleType::Truck => "🚚",
            VehicleType::Motorcycle => "🏍️",
            VehicleType::Tank => "[Shld]",
            VehicleType::Custom => "⚙️",
        }
    }

    pub fn wheel_count(&self) -> u8 {
        match self {
            VehicleType::Sedan | VehicleType::SportsCar => 4,
            VehicleType::Truck => 6,
            VehicleType::Motorcycle => 2,
            VehicleType::Tank => 0, // Tracks
            VehicleType::Custom => 4,
        }
    }
}

/// Cloth simulation quality
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ClothQuality {
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

impl std::fmt::Display for ClothQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ClothQuality {
    pub fn all() -> &'static [ClothQuality] {
        &[
            ClothQuality::Low,
            ClothQuality::Medium,
            ClothQuality::High,
            ClothQuality::Ultra,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ClothQuality::Low => "Low",
            ClothQuality::Medium => "Medium",
            ClothQuality::High => "High",
            ClothQuality::Ultra => "Ultra",
        }
    }

    pub fn iterations(&self) -> u32 {
        match self {
            ClothQuality::Low => 4,
            ClothQuality::Medium => 8,
            ClothQuality::High => 16,
            ClothQuality::Ultra => 32,
        }
    }

    pub fn cpu_cost(&self) -> u8 {
        match self {
            ClothQuality::Low => 1,
            ClothQuality::Medium => 2,
            ClothQuality::High => 4,
            ClothQuality::Ultra => 8,
        }
    }
}

/// Destruction pattern type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DestructionPattern {
    #[default]
    Voronoi,
    Radial,
    Slice,
    Shatter,
    Crumble,
}

impl std::fmt::Display for DestructionPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl DestructionPattern {
    pub fn all() -> &'static [DestructionPattern] {
        &[
            DestructionPattern::Voronoi,
            DestructionPattern::Radial,
            DestructionPattern::Slice,
            DestructionPattern::Shatter,
            DestructionPattern::Crumble,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            DestructionPattern::Voronoi => "Voronoi",
            DestructionPattern::Radial => "Radial",
            DestructionPattern::Slice => "Slice",
            DestructionPattern::Shatter => "Shatter",
            DestructionPattern::Crumble => "Crumble",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DestructionPattern::Voronoi => "⬡",
            DestructionPattern::Radial => "[Hit]",
            DestructionPattern::Slice => "🔪",
            DestructionPattern::Shatter => "[Gem]",
            DestructionPattern::Crumble => "[Brk]",
        }
    }

    pub fn fragment_count_range(&self) -> (usize, usize) {
        match self {
            DestructionPattern::Voronoi => (10, 30),
            DestructionPattern::Radial => (8, 16),
            DestructionPattern::Slice => (2, 4),
            DestructionPattern::Shatter => (20, 50),
            DestructionPattern::Crumble => (30, 100),
        }
    }
}

/// Gravity zone shape
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GravityZoneShape {
    #[default]
    Box,
    Sphere,
    Cylinder,
    Global,
}

impl std::fmt::Display for GravityZoneShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl GravityZoneShape {
    pub fn all() -> &'static [GravityZoneShape] {
        &[
            GravityZoneShape::Box,
            GravityZoneShape::Sphere,
            GravityZoneShape::Cylinder,
            GravityZoneShape::Global,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            GravityZoneShape::Box => "Box",
            GravityZoneShape::Sphere => "Sphere",
            GravityZoneShape::Cylinder => "Cylinder",
            GravityZoneShape::Global => "Global",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            GravityZoneShape::Box => "[Pkg]",
            GravityZoneShape::Sphere => "⚪",
            GravityZoneShape::Cylinder => "🛢️",
            GravityZoneShape::Global => "[Glb]",
        }
    }

    pub fn is_volumetric(&self) -> bool {
        matches!(
            self,
            GravityZoneShape::Box | GravityZoneShape::Sphere | GravityZoneShape::Cylinder
        )
    }
}

/// Physics statistics
#[derive(Debug, Clone, Default)]
pub struct PhysicsStats {
    pub rigid_body_count: usize,
    pub collider_count: usize,
    pub joint_count: usize,
    pub contact_pair_count: usize,
    pub active_body_count: usize,
    pub island_count: usize,
    pub step_time_ms: f32,
    pub broad_phase_time_ms: f32,
    pub narrow_phase_time_ms: f32,
    pub solver_time_ms: f32,
    pub ccd_time_ms: f32,
    pub ragdoll_count: usize,
    pub vehicle_count: usize,
    pub cloth_count: usize,
    pub destructible_count: usize,
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PhysicsTab {
    #[default]
    Overview,
    Visualization,
    Ragdoll,
    Vehicle,
    Cloth,
    Destruction,
    Environment,
}

impl std::fmt::Display for PhysicsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl PhysicsTab {
    pub fn all() -> &'static [PhysicsTab] {
        &[
            PhysicsTab::Overview,
            PhysicsTab::Visualization,
            PhysicsTab::Ragdoll,
            PhysicsTab::Vehicle,
            PhysicsTab::Cloth,
            PhysicsTab::Destruction,
            PhysicsTab::Environment,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PhysicsTab::Overview => "Overview",
            PhysicsTab::Visualization => "Visualization",
            PhysicsTab::Ragdoll => "Ragdoll",
            PhysicsTab::Vehicle => "Vehicle",
            PhysicsTab::Cloth => "Cloth",
            PhysicsTab::Destruction => "Destruction",
            PhysicsTab::Environment => "Environment",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PhysicsTab::Overview => "[Chart]",
            PhysicsTab::Visualization => "👁️",
            PhysicsTab::Ragdoll => "🚶",
            PhysicsTab::Vehicle => "🚗",
            PhysicsTab::Cloth => "👕",
            PhysicsTab::Destruction => "[Hit]",
            PhysicsTab::Environment => "[Glb]",
        }
    }
}

/// Actions that can be performed on the physics panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PhysicsAction {
    /// Start physics simulation
    Play,
    /// Pause physics simulation
    Pause,
    /// Step simulation by one frame
    Step,
    /// Reset simulation to initial state
    Reset,
    /// Set time scale
    SetTimeScale(f32),
    /// Set simulation mode
    SetSimulationMode(SimulationMode),
    /// Set visualization mode
    SetVisualization(PhysicsVisualization),
    /// Apply ragdoll preset
    ApplyRagdollPreset(RagdollPreset),
    /// Spawn test ragdoll
    SpawnTestRagdoll,
    /// Apply test impulse
    ApplyTestImpulse { strength: f32 },
    /// Set vehicle type
    SetVehicleType(VehicleType),
    /// Set cloth quality
    SetClothQuality(ClothQuality),
    /// Set destruction pattern
    SetDestructionPattern(DestructionPattern),
    /// Add gravity zone
    AddGravityZone { name: String },
    /// Remove gravity zone
    RemoveGravityZone { index: usize },
    /// Toggle wind
    ToggleWind(bool),
}

impl std::fmt::Display for PhysicsAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl PhysicsAction {
    /// Returns the name of this action
    pub fn name(&self) -> &'static str {
        match self {
            PhysicsAction::Play => "Play",
            PhysicsAction::Pause => "Pause",
            PhysicsAction::Step => "Step",
            PhysicsAction::Reset => "Reset",
            PhysicsAction::SetTimeScale(_) => "Set Time Scale",
            PhysicsAction::SetSimulationMode(_) => "Set Simulation Mode",
            PhysicsAction::SetVisualization(_) => "Set Visualization",
            PhysicsAction::ApplyRagdollPreset(_) => "Apply Ragdoll Preset",
            PhysicsAction::SpawnTestRagdoll => "Spawn Test Ragdoll",
            PhysicsAction::ApplyTestImpulse { .. } => "Apply Test Impulse",
            PhysicsAction::SetVehicleType(_) => "Set Vehicle Type",
            PhysicsAction::SetClothQuality(_) => "Set Cloth Quality",
            PhysicsAction::SetDestructionPattern(_) => "Set Destruction Pattern",
            PhysicsAction::AddGravityZone { .. } => "Add Gravity Zone",
            PhysicsAction::RemoveGravityZone { .. } => "Remove Gravity Zone",
            PhysicsAction::ToggleWind(_) => "Toggle Wind",
        }
    }

    /// Returns true if this is a simulation control action
    pub fn is_simulation_control(&self) -> bool {
        matches!(
            self,
            PhysicsAction::Play
                | PhysicsAction::Pause
                | PhysicsAction::Step
                | PhysicsAction::Reset
                | PhysicsAction::SetTimeScale(_)
                | PhysicsAction::SetSimulationMode(_)
        )
    }

    /// Returns true if this is a visualization action
    pub fn is_visualization(&self) -> bool {
        matches!(self, PhysicsAction::SetVisualization(_))
    }

    /// Returns true if this is a system configuration action
    pub fn is_system_config(&self) -> bool {
        matches!(
            self,
            PhysicsAction::ApplyRagdollPreset(_)
                | PhysicsAction::SetVehicleType(_)
                | PhysicsAction::SetClothQuality(_)
                | PhysicsAction::SetDestructionPattern(_)
        )
    }

    /// Returns true if this is an environment action
    pub fn is_environment(&self) -> bool {
        matches!(
            self,
            PhysicsAction::AddGravityZone { .. }
                | PhysicsAction::RemoveGravityZone { .. }
                | PhysicsAction::ToggleWind(_)
        )
    }
}

/// Main Physics Debug Panel
pub struct PhysicsPanel {
    // Tab state
    active_tab: PhysicsTab,

    // Simulation settings
    simulation_mode: SimulationMode,
    time_scale: f32,
    gravity: [f32; 3],
    substeps: u32,
    solver_iterations: u32,
    ccd_enabled: bool,

    // Visualization
    visualization_mode: PhysicsVisualization,
    collider_alpha: f32,
    show_sleeping_bodies: bool,
    show_aabbs: bool,
    show_mass_centers: bool,
    collision_layers: Vec<CollisionLayer>,

    // Ragdoll settings
    ragdoll_preset: RagdollPreset,
    ragdoll_joint_stiffness: f32,
    ragdoll_joint_damping: f32,
    ragdoll_impulse_multiplier: f32,
    ragdoll_blend_time: f32,

    // Vehicle settings
    vehicle_type: VehicleType,
    engine_torque: f32,
    suspension_stiffness: f32,
    suspension_damping: f32,
    wheel_friction: f32,
    downforce_coefficient: f32,

    // Cloth settings
    cloth_quality: ClothQuality,
    cloth_gravity_scale: f32,
    cloth_damping: f32,
    cloth_stiffness: f32,
    cloth_wind_influence: f32,
    cloth_self_collision: bool,

    // Destruction settings
    destruction_pattern: DestructionPattern,
    debris_lifetime: f32,
    debris_count_max: u32,
    destruction_impulse_threshold: f32,
    destruction_chain_reaction: bool,

    // Environment settings
    wind_enabled: bool,
    wind_direction: [f32; 3],
    wind_strength: f32,
    wind_turbulence: f32,
    gravity_zones: Vec<GravityZoneConfig>,
    selected_gravity_zone: Option<usize>,

    // Statistics
    stats: PhysicsStats,
    step_time_history: VecDeque<f32>,

    // Test tools
    spawn_test_objects: bool,
    test_impulse_strength: f32,

    // Action queue
    pending_actions: Vec<PhysicsAction>,
}

/// Gravity zone configuration
#[derive(Debug, Clone)]
pub struct GravityZoneConfig {
    pub name: String,
    pub shape: GravityZoneShape,
    pub strength: f32,
    pub direction: [f32; 3],
    pub falloff: bool,
    pub enabled: bool,
}

impl Default for GravityZoneConfig {
    fn default() -> Self {
        Self {
            name: "New Zone".to_string(),
            shape: GravityZoneShape::Sphere,
            strength: 9.81,
            direction: [0.0, -1.0, 0.0],
            falloff: true,
            enabled: true,
        }
    }
}

impl Default for PhysicsPanel {
    fn default() -> Self {
        let default_layers = vec![
            CollisionLayer {
                id: 1,
                name: "Default".to_string(),
                color: Color32::WHITE,
                enabled: true,
            },
            CollisionLayer {
                id: 2,
                name: "Characters".to_string(),
                color: Color32::GREEN,
                enabled: true,
            },
            CollisionLayer {
                id: 4,
                name: "Projectiles".to_string(),
                color: Color32::RED,
                enabled: true,
            },
            CollisionLayer {
                id: 8,
                name: "Triggers".to_string(),
                color: Color32::YELLOW,
                enabled: true,
            },
            CollisionLayer {
                id: 16,
                name: "Debris".to_string(),
                color: Color32::GRAY,
                enabled: true,
            },
        ];

        Self {
            active_tab: PhysicsTab::Overview,

            simulation_mode: SimulationMode::Running,
            time_scale: 1.0,
            gravity: [0.0, -9.81, 0.0],
            substeps: 1,
            solver_iterations: 4,
            ccd_enabled: true,

            visualization_mode: PhysicsVisualization::None,
            collider_alpha: 0.5,
            show_sleeping_bodies: false,
            show_aabbs: false,
            show_mass_centers: false,
            collision_layers: default_layers,

            ragdoll_preset: RagdollPreset::Humanoid,
            ragdoll_joint_stiffness: 100.0,
            ragdoll_joint_damping: 10.0,
            ragdoll_impulse_multiplier: 1.0,
            ragdoll_blend_time: 0.2,

            vehicle_type: VehicleType::Sedan,
            engine_torque: 500.0,
            suspension_stiffness: 50000.0,
            suspension_damping: 4500.0,
            wheel_friction: 1.0,
            downforce_coefficient: 0.3,

            cloth_quality: ClothQuality::Medium,
            cloth_gravity_scale: 1.0,
            cloth_damping: 0.01,
            cloth_stiffness: 1.0,
            cloth_wind_influence: 1.0,
            cloth_self_collision: false,

            destruction_pattern: DestructionPattern::Voronoi,
            debris_lifetime: 5.0,
            debris_count_max: 50,
            destruction_impulse_threshold: 100.0,
            destruction_chain_reaction: true,

            wind_enabled: false,
            wind_direction: [1.0, 0.0, 0.0],
            wind_strength: 10.0,
            wind_turbulence: 0.3,
            gravity_zones: Vec::new(),
            selected_gravity_zone: None,

            stats: PhysicsStats::default(),
            step_time_history: VecDeque::with_capacity(120),

            spawn_test_objects: false,
            test_impulse_strength: 500.0,

            pending_actions: Vec::new(),
        }
    }
}

impl PhysicsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes all pending actions and clears the internal queue
    pub fn take_actions(&mut self) -> Vec<PhysicsAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for external processing
    pub fn queue_action(&mut self, action: PhysicsAction) {
        info!(target: "aw_editor::panels::physics_panel", action = %action, "Physics action queued");
        self.pending_actions.push(action);
    }

    /// Returns the current time scale
    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    /// Returns true if wind is enabled
    pub fn wind_enabled(&self) -> bool {
        self.wind_enabled
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (PhysicsTab::Overview, "Overview"),
                (PhysicsTab::Visualization, "👁 Viz"),
                (PhysicsTab::Ragdoll, "🦴 Ragdoll"),
                (PhysicsTab::Vehicle, "🚗 Vehicle"),
                (PhysicsTab::Cloth, "🧵 Cloth"),
                (PhysicsTab::Destruction, "[Hit] Destruction"),
                (PhysicsTab::Environment, "[Glb] Environment"),
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

        // Simulation controls
        ui.horizontal(|ui| {
            let mode_icon = match self.simulation_mode {
                SimulationMode::Running => ">",
                SimulationMode::Paused => "||",
                SimulationMode::StepOnce => ">|",
                SimulationMode::SlowMotion => "🐢",
            };
            ui.label(format!("{} {:?}", mode_icon, self.simulation_mode));
            ui.label(format!("| {} bodies", self.stats.rigid_body_count));
            ui.label(format!("| {:.2}ms", self.stats.step_time_ms));
        });

        ui.separator();
    }

    fn show_overview_tab(&mut self, ui: &mut Ui) {
        ui.heading("Physics Overview");
        ui.add_space(10.0);

        // Simulation controls
        ui.group(|ui| {
            ui.label(RichText::new("Simulation Controls").strong());

            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.simulation_mode == SimulationMode::Running, "> Run")
                    .clicked()
                {
                    info!(target: "aw_editor::panels::physics_panel", "Physics simulation started");
                    self.simulation_mode = SimulationMode::Running;
                }
                if ui
                    .selectable_label(self.simulation_mode == SimulationMode::Paused, "|| Pause")
                    .clicked()
                {
                    info!(target: "aw_editor::panels::physics_panel", "Physics simulation paused");
                    self.simulation_mode = SimulationMode::Paused;
                }
                if ui.button(">| Step").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Physics simulation step");
                    self.simulation_mode = SimulationMode::StepOnce;
                }
                if ui
                    .selectable_label(
                        self.simulation_mode == SimulationMode::SlowMotion,
                        "🐢 Slow",
                    )
                    .clicked()
                {
                    self.simulation_mode = SimulationMode::SlowMotion;
                }
            });

            ui.add(
                egui::Slider::new(&mut self.time_scale, 0.1..=2.0)
                    .text("Time Scale")
                    .logarithmic(true),
            );
        });

        ui.add_space(10.0);

        // World settings
        ui.group(|ui| {
            ui.label(RichText::new("World Settings").strong());

            egui::Grid::new("world_settings_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Gravity X:");
                    ui.add(egui::DragValue::new(&mut self.gravity[0]).speed(0.1));
                    ui.end_row();

                    ui.label("Gravity Y:");
                    ui.add(egui::DragValue::new(&mut self.gravity[1]).speed(0.1));
                    ui.end_row();

                    ui.label("Gravity Z:");
                    ui.add(egui::DragValue::new(&mut self.gravity[2]).speed(0.1));
                    ui.end_row();

                    ui.label("Substeps:");
                    ui.add(egui::DragValue::new(&mut self.substeps).range(1..=8));
                    ui.end_row();

                    ui.label("Solver Iterations:");
                    ui.add(egui::DragValue::new(&mut self.solver_iterations).range(1..=16));
                    ui.end_row();

                    ui.label("CCD Enabled:");
                    ui.checkbox(&mut self.ccd_enabled, "Continuous collision");
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Statistics
        ui.group(|ui| {
            ui.label(RichText::new("Statistics").strong());

            egui::Grid::new("physics_stats_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Rigid Bodies:");
                    ui.label(format!(
                        "{} ({} active)",
                        self.stats.rigid_body_count, self.stats.active_body_count
                    ));
                    ui.end_row();

                    ui.label("Colliders:");
                    ui.label(format!("{}", self.stats.collider_count));
                    ui.end_row();

                    ui.label("Joints:");
                    ui.label(format!("{}", self.stats.joint_count));
                    ui.end_row();

                    ui.label("Contact Pairs:");
                    ui.label(format!("{}", self.stats.contact_pair_count));
                    ui.end_row();

                    ui.label("Islands:");
                    ui.label(format!("{}", self.stats.island_count));
                    ui.end_row();
                });

            ui.add_space(5.0);

            // Timing breakdown
            ui.label("Step Time Breakdown:");
            self.draw_timing_bar(ui);
        });

        ui.add_space(10.0);

        // Subsystem counts
        ui.group(|ui| {
            ui.label(RichText::new("Active Subsystems").strong());

            ui.horizontal(|ui| {
                ui.label(format!("🦴 {} Ragdolls", self.stats.ragdoll_count));
                ui.separator();
                ui.label(format!("🚗 {} Vehicles", self.stats.vehicle_count));
                ui.separator();
                ui.label(format!("🧵 {} Cloth", self.stats.cloth_count));
                ui.separator();
                ui.label(format!(
                    "[Hit] {} Destructibles",
                    self.stats.destructible_count
                ));
            });
        });
    }

    fn show_visualization_tab(&mut self, ui: &mut Ui) {
        ui.heading("👁 Debug Visualization");
        ui.add_space(10.0);

        // Visualization mode
        ui.group(|ui| {
            ui.label(RichText::new("Visualization Mode").strong());

            for mode in PhysicsVisualization::all() {
                if ui
                    .selectable_label(self.visualization_mode == *mode, format!("{:?}", mode))
                    .clicked()
                {
                    self.visualization_mode = *mode;
                }
            }

            ui.label(self.visualization_mode.description());
        });

        ui.add_space(10.0);

        // Visualization options
        ui.group(|ui| {
            ui.label(RichText::new("Options").strong());

            ui.add(egui::Slider::new(&mut self.collider_alpha, 0.1..=1.0).text("Collider Alpha"));
            ui.checkbox(
                &mut self.show_sleeping_bodies,
                "Show sleeping bodies (dimmed)",
            );
            ui.checkbox(&mut self.show_aabbs, "Show AABBs");
            ui.checkbox(&mut self.show_mass_centers, "Show center of mass");
        });

        ui.add_space(10.0);

        // Collision layers
        ui.group(|ui| {
            ui.label(RichText::new("Collision Layers").strong());

            for layer in &mut self.collision_layers {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut layer.enabled, "");
                    ui.colored_label(layer.color, &layer.name);
                    ui.label(format!("({})", layer.id));
                });
            }
        });
    }

    fn show_ragdoll_tab(&mut self, ui: &mut Ui) {
        ui.heading("🦴 Ragdoll Physics");
        ui.add_space(10.0);

        // Preset selection
        ui.group(|ui| {
            ui.label(RichText::new("Ragdoll Preset").strong());

            ui.horizontal_wrapped(|ui| {
                for preset in RagdollPreset::all() {
                    if ui
                        .selectable_label(self.ragdoll_preset == *preset, format!("{:?}", preset))
                        .clicked()
                    {
                        self.ragdoll_preset = *preset;
                    }
                }
            });

            ui.label(format!("Bone count: {}", self.ragdoll_preset.bone_count()));
        });

        ui.add_space(10.0);

        // Joint settings
        ui.group(|ui| {
            ui.label(RichText::new("Joint Settings").strong());

            ui.add(
                egui::Slider::new(&mut self.ragdoll_joint_stiffness, 0.0..=500.0).text("Stiffness"),
            );
            ui.add(egui::Slider::new(&mut self.ragdoll_joint_damping, 0.0..=50.0).text("Damping"));
            ui.add(
                egui::Slider::new(&mut self.ragdoll_impulse_multiplier, 0.1..=5.0)
                    .text("Impulse Multiplier"),
            );
            ui.add(
                egui::Slider::new(&mut self.ragdoll_blend_time, 0.0..=1.0)
                    .text("Blend Time")
                    .suffix(" s"),
            );
        });

        ui.add_space(10.0);

        // Test tools
        ui.group(|ui| {
            ui.label(RichText::new("Test Tools").strong());

            ui.horizontal(|ui| {
                if ui.button("🦴 Spawn Ragdoll").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Ragdoll spawned");
                    // Spawn test ragdoll
                }
                if ui.button("[Dash] Apply Impulse").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", strength = self.test_impulse_strength, "Test impulse applied");
                    // Apply test impulse
                }
                if ui.button("Reset All").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "All ragdolls reset");
                    // Reset all ragdolls
                }
            });

            ui.add(
                egui::Slider::new(&mut self.test_impulse_strength, 100.0..=2000.0)
                    .text("Impulse Strength"),
            );
        });
    }

    fn show_vehicle_tab(&mut self, ui: &mut Ui) {
        ui.heading("🚗 Vehicle Physics");
        ui.add_space(10.0);

        // Vehicle type
        ui.group(|ui| {
            ui.label(RichText::new("Vehicle Type").strong());

            ui.horizontal_wrapped(|ui| {
                for vtype in VehicleType::all() {
                    if ui
                        .selectable_label(self.vehicle_type == *vtype, format!("{:?}", vtype))
                        .clicked()
                    {
                        self.vehicle_type = *vtype;
                        self.apply_vehicle_preset(*vtype);
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Engine settings
        ui.group(|ui| {
            ui.label(RichText::new("Engine").strong());

            ui.add(
                egui::Slider::new(&mut self.engine_torque, 100.0..=2000.0)
                    .text("Max Torque")
                    .suffix(" Nm"),
            );
        });

        ui.add_space(10.0);

        // Suspension settings
        ui.group(|ui| {
            ui.label(RichText::new("Suspension").strong());

            ui.add(
                egui::Slider::new(&mut self.suspension_stiffness, 10000.0..=100000.0)
                    .text("Stiffness")
                    .logarithmic(true),
            );
            ui.add(
                egui::Slider::new(&mut self.suspension_damping, 1000.0..=10000.0).text("Damping"),
            );
        });

        ui.add_space(10.0);

        // Wheels and aerodynamics
        ui.group(|ui| {
            ui.label(RichText::new("Wheels & Aero").strong());

            ui.add(egui::Slider::new(&mut self.wheel_friction, 0.5..=2.0).text("Wheel Friction"));
            ui.add(egui::Slider::new(&mut self.downforce_coefficient, 0.0..=1.0).text("Downforce"));
        });

        ui.add_space(10.0);

        // Test tools
        ui.group(|ui| {
            ui.label(RichText::new("Test Tools").strong());

            ui.horizontal(|ui| {
                if ui.button("🚗 Spawn Vehicle").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Test vehicle spawned");
                    // Spawn test vehicle
                }
                if ui.button("Reset Position").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Vehicle position reset");
                    // Reset vehicle position
                }
            });
        });
    }

    fn show_cloth_tab(&mut self, ui: &mut Ui) {
        ui.heading("🧵 Cloth Simulation");
        ui.add_space(10.0);

        // Quality settings
        ui.group(|ui| {
            ui.label(RichText::new("Quality").strong());

            ui.horizontal(|ui| {
                for quality in ClothQuality::all() {
                    if ui
                        .selectable_label(self.cloth_quality == *quality, format!("{:?}", quality))
                        .clicked()
                    {
                        self.cloth_quality = *quality;
                    }
                }
            });

            ui.label(format!(
                "Solver iterations: {}",
                self.cloth_quality.iterations()
            ));
        });

        ui.add_space(10.0);

        // Cloth physics
        ui.group(|ui| {
            ui.label(RichText::new("Physics").strong());

            ui.add(
                egui::Slider::new(&mut self.cloth_gravity_scale, 0.0..=2.0).text("Gravity Scale"),
            );
            ui.add(egui::Slider::new(&mut self.cloth_damping, 0.0..=0.1).text("Damping"));
            ui.add(egui::Slider::new(&mut self.cloth_stiffness, 0.1..=2.0).text("Stiffness"));
            ui.add(
                egui::Slider::new(&mut self.cloth_wind_influence, 0.0..=2.0).text("Wind Influence"),
            );
            ui.checkbox(&mut self.cloth_self_collision, "Self Collision");
        });

        ui.add_space(10.0);

        // Test tools
        ui.group(|ui| {
            ui.label(RichText::new("Test Tools").strong());

            ui.horizontal(|ui| {
                if ui.button("🧵 Spawn Cloth").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Cloth simulation spawned");
                    // Spawn test cloth
                }
                if ui.button("[Dash] Apply Wind Gust").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Wind gust applied to cloth");
                    // Apply wind gust
                }
                if ui.button("[Pin] Pin Corners").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Cloth corners pinned");
                    // Pin cloth corners
                }
            });
        });
    }

    fn show_destruction_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Hit] Destruction System");
        ui.add_space(10.0);

        // Fracture pattern
        ui.group(|ui| {
            ui.label(RichText::new("Fracture Pattern").strong());

            ui.horizontal_wrapped(|ui| {
                for pattern in DestructionPattern::all() {
                    if ui
                        .selectable_label(
                            self.destruction_pattern == *pattern,
                            format!("{:?}", pattern),
                        )
                        .clicked()
                    {
                        self.destruction_pattern = *pattern;
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Debris settings
        ui.group(|ui| {
            ui.label(RichText::new("Debris").strong());

            ui.add(
                egui::Slider::new(&mut self.debris_lifetime, 1.0..=30.0)
                    .text("Lifetime")
                    .suffix(" s"),
            );
            ui.add(
                egui::DragValue::new(&mut self.debris_count_max)
                    .range(10..=200)
                    .prefix("Max Count: "),
            );
        });

        ui.add_space(10.0);

        // Trigger settings
        ui.group(|ui| {
            ui.label(RichText::new("Triggers").strong());

            ui.add(
                egui::Slider::new(&mut self.destruction_impulse_threshold, 10.0..=500.0)
                    .text("Impulse Threshold"),
            );
            ui.checkbox(&mut self.destruction_chain_reaction, "Chain Reaction");
        });

        ui.add_space(10.0);

        // Test tools
        ui.group(|ui| {
            ui.label(RichText::new("Test Tools").strong());

            ui.horizontal(|ui| {
                if ui.button("[Brk] Spawn Destructible").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Destructible object spawned");
                    // Spawn destructible
                }
                if ui.button("[Hit] Trigger Explosion").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Explosion triggered");
                    // Trigger explosion
                }
                if ui.button("🧹 Clear Debris").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Debris cleared");
                    // Clear all debris
                }
            });
        });
    }

    fn show_environment_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Glb] Environment Physics");
        ui.add_space(10.0);

        // Wind settings
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("[Dash] Wind").strong());
                ui.checkbox(&mut self.wind_enabled, "Enabled");
            });

            if self.wind_enabled {
                ui.add_space(5.0);

                egui::Grid::new("wind_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Direction X:");
                        ui.add(egui::DragValue::new(&mut self.wind_direction[0]).speed(0.1));
                        ui.end_row();

                        ui.label("Direction Y:");
                        ui.add(egui::DragValue::new(&mut self.wind_direction[1]).speed(0.1));
                        ui.end_row();

                        ui.label("Direction Z:");
                        ui.add(egui::DragValue::new(&mut self.wind_direction[2]).speed(0.1));
                        ui.end_row();
                    });

                ui.add(egui::Slider::new(&mut self.wind_strength, 0.0..=50.0).text("Strength"));
                ui.add(egui::Slider::new(&mut self.wind_turbulence, 0.0..=1.0).text("Turbulence"));
            }
        });

        ui.add_space(10.0);

        // Gravity zones
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Gravity Zones").strong());
                if ui.button("Add").clicked() {
                    info!(target: "aw_editor::panels::physics_panel", "Gravity zone added");
                    self.gravity_zones.push(GravityZoneConfig::default());
                }
            });

            ui.separator();

            if self.gravity_zones.is_empty() {
                ui.label("No gravity zones defined.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        let mut to_remove = None;

                        for (i, zone) in self.gravity_zones.iter_mut().enumerate() {
                            let is_selected = self.selected_gravity_zone == Some(i);

                            ui.horizontal(|ui| {
                                ui.checkbox(&mut zone.enabled, "");
                                if ui.selectable_label(is_selected, &zone.name).clicked() {
                                    self.selected_gravity_zone = Some(i);
                                }
                                ui.label(format!("{:?}", zone.shape));
                                ui.label(format!("{:.1} m/s²", zone.strength));

                                if ui.small_button("[Del]").clicked() {
                                    info!(target: "aw_editor::panels::physics_panel", zone_index = i, zone_name = %zone.name, "Gravity zone removed");
                                    to_remove = Some(i);
                                }
                            });
                        }

                        if let Some(i) = to_remove {
                            self.gravity_zones.remove(i);
                            self.selected_gravity_zone = None;
                        }
                    });
            }
        });

        // Selected zone editor
        if let Some(idx) = self.selected_gravity_zone {
            if let Some(zone) = self.gravity_zones.get_mut(idx) {
                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label(RichText::new("Zone Editor").strong());

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut zone.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Shape:");
                        for shape in GravityZoneShape::all() {
                            if ui
                                .selectable_label(zone.shape == *shape, format!("{:?}", shape))
                                .clicked()
                            {
                                zone.shape = *shape;
                            }
                        }
                    });

                    ui.add(
                        egui::Slider::new(&mut zone.strength, 0.0..=30.0)
                            .text("Strength")
                            .suffix(" m/s²"),
                    );

                    ui.checkbox(&mut zone.falloff, "Distance Falloff");
                });
            }
        }
    }

    fn draw_timing_bar(&self, ui: &mut Ui) {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), egui::Sense::hover());

        let total = self.stats.step_time_ms.max(0.001);
        let bar_width = rect.width();

        // Calculate widths
        let broad_width = (self.stats.broad_phase_time_ms / total) * bar_width;
        let narrow_width = (self.stats.narrow_phase_time_ms / total) * bar_width;
        let solver_width = (self.stats.solver_time_ms / total) * bar_width;
        let ccd_width = (self.stats.ccd_time_ms / total) * bar_width;

        let mut x = rect.left();

        // Broad phase (blue)
        let broad_rect = egui::Rect::from_min_size(
            egui::Pos2::new(x, rect.top()),
            Vec2::new(broad_width, rect.height()),
        );
        ui.painter()
            .rect_filled(broad_rect, 0.0, Color32::from_rgb(100, 150, 255));
        x += broad_width;

        // Narrow phase (green)
        let narrow_rect = egui::Rect::from_min_size(
            egui::Pos2::new(x, rect.top()),
            Vec2::new(narrow_width, rect.height()),
        );
        ui.painter()
            .rect_filled(narrow_rect, 0.0, Color32::from_rgb(100, 200, 100));
        x += narrow_width;

        // Solver (orange)
        let solver_rect = egui::Rect::from_min_size(
            egui::Pos2::new(x, rect.top()),
            Vec2::new(solver_width, rect.height()),
        );
        ui.painter()
            .rect_filled(solver_rect, 0.0, Color32::from_rgb(255, 165, 0));
        x += solver_width;

        // CCD (red)
        let ccd_rect = egui::Rect::from_min_size(
            egui::Pos2::new(x, rect.top()),
            Vec2::new(ccd_width, rect.height()),
        );
        ui.painter()
            .rect_filled(ccd_rect, 0.0, Color32::from_rgb(255, 100, 100));

        // Legend
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(100, 150, 255), "Broad");
            ui.colored_label(Color32::from_rgb(100, 200, 100), "Narrow");
            ui.colored_label(Color32::from_rgb(255, 165, 0), "Solver");
            ui.colored_label(Color32::from_rgb(255, 100, 100), "CCD");
        });
    }

    fn apply_vehicle_preset(&mut self, vtype: VehicleType) {
        match vtype {
            VehicleType::Sedan => {
                self.engine_torque = 300.0;
                self.suspension_stiffness = 35000.0;
                self.suspension_damping = 4000.0;
                self.wheel_friction = 1.0;
                self.downforce_coefficient = 0.1;
            }
            VehicleType::SportsCar => {
                self.engine_torque = 600.0;
                self.suspension_stiffness = 60000.0;
                self.suspension_damping = 5000.0;
                self.wheel_friction = 1.2;
                self.downforce_coefficient = 0.5;
            }
            VehicleType::Truck => {
                self.engine_torque = 800.0;
                self.suspension_stiffness = 80000.0;
                self.suspension_damping = 8000.0;
                self.wheel_friction = 0.9;
                self.downforce_coefficient = 0.05;
            }
            VehicleType::Motorcycle => {
                self.engine_torque = 200.0;
                self.suspension_stiffness = 25000.0;
                self.suspension_damping = 3000.0;
                self.wheel_friction = 1.1;
                self.downforce_coefficient = 0.0;
            }
            VehicleType::Tank => {
                self.engine_torque = 1500.0;
                self.suspension_stiffness = 100000.0;
                self.suspension_damping = 12000.0;
                self.wheel_friction = 1.5;
                self.downforce_coefficient = 0.0;
            }
            VehicleType::Custom => {}
        }
    }

    // Getters for testing
    pub fn visualization_mode(&self) -> PhysicsVisualization {
        self.visualization_mode
    }

    pub fn simulation_mode(&self) -> SimulationMode {
        self.simulation_mode
    }

    pub fn ragdoll_preset(&self) -> RagdollPreset {
        self.ragdoll_preset
    }

    pub fn vehicle_type(&self) -> VehicleType {
        self.vehicle_type
    }

    pub fn cloth_quality(&self) -> ClothQuality {
        self.cloth_quality
    }

    pub fn destruction_pattern(&self) -> DestructionPattern {
        self.destruction_pattern
    }

    pub fn gravity_zone_count(&self) -> usize {
        self.gravity_zones.len()
    }

    pub fn is_wind_enabled(&self) -> bool {
        self.wind_enabled
    }

    pub fn set_visualization_mode(&mut self, mode: PhysicsVisualization) {
        self.visualization_mode = mode;
    }

    pub fn set_simulation_mode(&mut self, mode: SimulationMode) {
        self.simulation_mode = mode;
    }

    pub fn add_gravity_zone(&mut self, zone: GravityZoneConfig) {
        self.gravity_zones.push(zone);
    }

    pub fn set_wind(&mut self, enabled: bool, strength: f32) {
        self.wind_enabled = enabled;
        self.wind_strength = strength;
    }

    pub fn collision_layer_count(&self) -> usize {
        self.collision_layers.len()
    }
}

impl Panel for PhysicsPanel {
    fn name(&self) -> &'static str {
        "Physics"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            PhysicsTab::Overview => self.show_overview_tab(ui),
            PhysicsTab::Visualization => self.show_visualization_tab(ui),
            PhysicsTab::Ragdoll => self.show_ragdoll_tab(ui),
            PhysicsTab::Vehicle => self.show_vehicle_tab(ui),
            PhysicsTab::Cloth => self.show_cloth_tab(ui),
            PhysicsTab::Destruction => self.show_destruction_tab(ui),
            PhysicsTab::Environment => self.show_environment_tab(ui),
        }
    }

    fn update(&mut self) {
        // Sample step time history
        if self.step_time_history.len() >= 120 {
            self.step_time_history.pop_front();
        }
        self.step_time_history.push_back(self.stats.step_time_ms);

        // Handle step-once mode
        if self.simulation_mode == SimulationMode::StepOnce {
            self.simulation_mode = SimulationMode::Paused;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_panel_creation() {
        let panel = PhysicsPanel::new();
        assert_eq!(panel.visualization_mode(), PhysicsVisualization::None);
        assert_eq!(panel.simulation_mode(), SimulationMode::Running);
    }

    #[test]
    fn test_visualization_modes() {
        let mut panel = PhysicsPanel::new();
        panel.set_visualization_mode(PhysicsVisualization::Colliders);
        assert_eq!(panel.visualization_mode(), PhysicsVisualization::Colliders);
    }

    #[test]
    fn test_simulation_modes() {
        let mut panel = PhysicsPanel::new();
        panel.set_simulation_mode(SimulationMode::Paused);
        assert_eq!(panel.simulation_mode(), SimulationMode::Paused);
    }

    #[test]
    fn test_ragdoll_presets() {
        let panel = PhysicsPanel::new();
        assert_eq!(panel.ragdoll_preset(), RagdollPreset::Humanoid);
        assert_eq!(RagdollPreset::Humanoid.bone_count(), 15);
        assert_eq!(RagdollPreset::Quadruped.bone_count(), 18);
    }

    #[test]
    fn test_vehicle_types() {
        let panel = PhysicsPanel::new();
        assert_eq!(panel.vehicle_type(), VehicleType::Sedan);
    }

    #[test]
    fn test_cloth_quality() {
        let panel = PhysicsPanel::new();
        assert_eq!(panel.cloth_quality(), ClothQuality::Medium);
        assert_eq!(ClothQuality::Medium.iterations(), 8);
        assert_eq!(ClothQuality::Ultra.iterations(), 32);
    }

    #[test]
    fn test_destruction_patterns() {
        let panel = PhysicsPanel::new();
        assert_eq!(panel.destruction_pattern(), DestructionPattern::Voronoi);
    }

    #[test]
    fn test_gravity_zones() {
        let mut panel = PhysicsPanel::new();
        assert_eq!(panel.gravity_zone_count(), 0);

        panel.add_gravity_zone(GravityZoneConfig::default());
        assert_eq!(panel.gravity_zone_count(), 1);
    }

    #[test]
    fn test_wind_settings() {
        let mut panel = PhysicsPanel::new();
        assert!(!panel.is_wind_enabled());

        panel.set_wind(true, 25.0);
        assert!(panel.is_wind_enabled());
        assert_eq!(panel.wind_strength, 25.0);
    }

    #[test]
    fn test_collision_layers() {
        let panel = PhysicsPanel::new();
        assert_eq!(panel.collision_layer_count(), 5); // Default, Characters, Projectiles, Triggers, Debris
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = PhysicsPanel::new();
        assert_eq!(panel.name(), "Physics");
    }

    // NEW: PhysicsVisualization Display and helper tests
    #[test]
    fn test_physics_visualization_all() {
        let modes = PhysicsVisualization::all();
        assert_eq!(modes.len(), 8);
    }

    #[test]
    fn test_physics_visualization_display() {
        assert_eq!(
            format!("{}", PhysicsVisualization::Colliders),
            "[Pkg] Colliders"
        );
        assert_eq!(format!("{}", PhysicsVisualization::Joints), "[Link] Joints");
    }

    #[test]
    fn test_physics_visualization_name() {
        assert_eq!(PhysicsVisualization::ContactPoints.name(), "Contact Points");
        assert_eq!(
            PhysicsVisualization::VelocityVectors.name(),
            "Velocity Vectors"
        );
    }

    #[test]
    fn test_physics_visualization_icon() {
        assert_eq!(PhysicsVisualization::BroadPhase.icon(), "[Files]");
        assert_eq!(PhysicsVisualization::Islands.icon(), "🏝️");
    }

    #[test]
    fn test_physics_visualization_is_visible() {
        assert!(PhysicsVisualization::Colliders.is_visible());
        assert!(PhysicsVisualization::All.is_visible());
        assert!(!PhysicsVisualization::None.is_visible());
    }

    #[test]
    fn test_physics_visualization_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PhysicsVisualization::Colliders);
        set.insert(PhysicsVisualization::Joints);
        assert_eq!(set.len(), 2);
    }

    // NEW: SimulationMode Display and helper tests
    #[test]
    fn test_simulation_mode_all() {
        let modes = SimulationMode::all();
        assert_eq!(modes.len(), 4);
    }

    #[test]
    fn test_simulation_mode_display() {
        assert_eq!(format!("{}", SimulationMode::Running), "> Running");
        assert_eq!(format!("{}", SimulationMode::Paused), "|| Paused");
    }

    #[test]
    fn test_simulation_mode_name() {
        assert_eq!(SimulationMode::StepOnce.name(), "Step Once");
        assert_eq!(SimulationMode::SlowMotion.name(), "Slow Motion");
    }

    #[test]
    fn test_simulation_mode_icon() {
        assert_eq!(SimulationMode::StepOnce.icon(), ">|");
        assert_eq!(SimulationMode::SlowMotion.icon(), "🐌");
    }

    #[test]
    fn test_simulation_mode_is_active() {
        assert!(SimulationMode::Running.is_active());
        assert!(SimulationMode::SlowMotion.is_active());
        assert!(!SimulationMode::Paused.is_active());
    }

    #[test]
    fn test_simulation_mode_time_scale() {
        assert_eq!(SimulationMode::Running.time_scale(), 1.0);
        assert_eq!(SimulationMode::Paused.time_scale(), 0.0);
        assert_eq!(SimulationMode::SlowMotion.time_scale(), 0.2);
    }

    #[test]
    fn test_simulation_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SimulationMode::Running);
        set.insert(SimulationMode::Paused);
        assert_eq!(set.len(), 2);
    }

    // NEW: RagdollPreset Display and helper tests
    #[test]
    fn test_ragdoll_preset_all() {
        let presets = RagdollPreset::all();
        assert_eq!(presets.len(), 5);
    }

    #[test]
    fn test_ragdoll_preset_display() {
        assert_eq!(format!("{}", RagdollPreset::Humanoid), "🚶 Humanoid");
        assert_eq!(format!("{}", RagdollPreset::Quadruped), "🐕 Quadruped");
    }

    #[test]
    fn test_ragdoll_preset_name() {
        assert_eq!(RagdollPreset::Bird.name(), "Bird");
        assert_eq!(RagdollPreset::Insect.name(), "Insect");
    }

    #[test]
    fn test_ragdoll_preset_icon() {
        assert_eq!(RagdollPreset::Bird.icon(), "🦅");
        assert_eq!(RagdollPreset::Custom.icon(), "⚙️");
    }

    #[test]
    fn test_ragdoll_preset_bone_count() {
        assert_eq!(RagdollPreset::Humanoid.bone_count(), 15);
        assert_eq!(RagdollPreset::Quadruped.bone_count(), 18);
        assert_eq!(RagdollPreset::Custom.bone_count(), 0);
    }

    #[test]
    fn test_ragdoll_preset_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(RagdollPreset::Humanoid);
        set.insert(RagdollPreset::Bird);
        assert_eq!(set.len(), 2);
    }

    // NEW: VehicleType Display and helper tests
    #[test]
    fn test_vehicle_type_all() {
        let types = VehicleType::all();
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn test_vehicle_type_display() {
        assert_eq!(format!("{}", VehicleType::Sedan), "🚗 Sedan");
        assert_eq!(format!("{}", VehicleType::SportsCar), "🏎️ Sports Car");
    }

    #[test]
    fn test_vehicle_type_name() {
        assert_eq!(VehicleType::Motorcycle.name(), "Motorcycle");
        assert_eq!(VehicleType::Tank.name(), "Tank");
    }

    #[test]
    fn test_vehicle_type_icon() {
        assert_eq!(VehicleType::Truck.icon(), "🚚");
        assert_eq!(VehicleType::Motorcycle.icon(), "🏍️");
    }

    #[test]
    fn test_vehicle_type_wheel_count() {
        assert_eq!(VehicleType::Sedan.wheel_count(), 4);
        assert_eq!(VehicleType::Motorcycle.wheel_count(), 2);
        assert_eq!(VehicleType::Truck.wheel_count(), 6);
        assert_eq!(VehicleType::Tank.wheel_count(), 0); // Tracks
    }

    #[test]
    fn test_vehicle_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(VehicleType::Sedan);
        set.insert(VehicleType::Tank);
        assert_eq!(set.len(), 2);
    }

    // NEW: ClothQuality Display and helper tests
    #[test]
    fn test_cloth_quality_all() {
        let qualities = ClothQuality::all();
        assert_eq!(qualities.len(), 4);
    }

    #[test]
    fn test_cloth_quality_display() {
        assert_eq!(format!("{}", ClothQuality::Low), "Low");
        assert_eq!(format!("{}", ClothQuality::Ultra), "Ultra");
    }

    #[test]
    fn test_cloth_quality_name() {
        assert_eq!(ClothQuality::Medium.name(), "Medium");
        assert_eq!(ClothQuality::High.name(), "High");
    }

    #[test]
    fn test_cloth_quality_iterations() {
        assert_eq!(ClothQuality::Low.iterations(), 4);
        assert_eq!(ClothQuality::Medium.iterations(), 8);
        assert_eq!(ClothQuality::High.iterations(), 16);
        assert_eq!(ClothQuality::Ultra.iterations(), 32);
    }

    #[test]
    fn test_cloth_quality_cpu_cost() {
        assert_eq!(ClothQuality::Low.cpu_cost(), 1);
        assert_eq!(ClothQuality::Medium.cpu_cost(), 2);
        assert_eq!(ClothQuality::High.cpu_cost(), 4);
        assert_eq!(ClothQuality::Ultra.cpu_cost(), 8);
    }

    #[test]
    fn test_cloth_quality_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ClothQuality::Low);
        set.insert(ClothQuality::High);
        assert_eq!(set.len(), 2);
    }

    // NEW: DestructionPattern Display and helper tests
    #[test]
    fn test_destruction_pattern_all() {
        let patterns = DestructionPattern::all();
        assert_eq!(patterns.len(), 5);
    }

    #[test]
    fn test_destruction_pattern_display() {
        assert_eq!(format!("{}", DestructionPattern::Voronoi), "⬡ Voronoi");
        assert_eq!(format!("{}", DestructionPattern::Shatter), "[Gem] Shatter");
    }

    #[test]
    fn test_destruction_pattern_name() {
        assert_eq!(DestructionPattern::Radial.name(), "Radial");
        assert_eq!(DestructionPattern::Crumble.name(), "Crumble");
    }

    #[test]
    fn test_destruction_pattern_icon() {
        assert_eq!(DestructionPattern::Slice.icon(), "🔪");
        assert_eq!(DestructionPattern::Radial.icon(), "[Hit]");
    }

    #[test]
    fn test_destruction_pattern_fragment_count_range() {
        let (min, max) = DestructionPattern::Voronoi.fragment_count_range();
        assert_eq!(min, 10);
        assert_eq!(max, 30);

        let (min, max) = DestructionPattern::Crumble.fragment_count_range();
        assert_eq!(min, 30);
        assert_eq!(max, 100);
    }

    #[test]
    fn test_destruction_pattern_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(DestructionPattern::Voronoi);
        set.insert(DestructionPattern::Shatter);
        assert_eq!(set.len(), 2);
    }

    // NEW: GravityZoneShape Display and helper tests
    #[test]
    fn test_gravity_zone_shape_all() {
        let shapes = GravityZoneShape::all();
        assert_eq!(shapes.len(), 4);
    }

    #[test]
    fn test_gravity_zone_shape_display() {
        assert_eq!(format!("{}", GravityZoneShape::Box), "[Pkg] Box");
        assert_eq!(format!("{}", GravityZoneShape::Sphere), "⚪ Sphere");
    }

    #[test]
    fn test_gravity_zone_shape_name() {
        assert_eq!(GravityZoneShape::Cylinder.name(), "Cylinder");
        assert_eq!(GravityZoneShape::Global.name(), "Global");
    }

    #[test]
    fn test_gravity_zone_shape_icon() {
        assert_eq!(GravityZoneShape::Cylinder.icon(), "🛢️");
        assert_eq!(GravityZoneShape::Global.icon(), "[Glb]");
    }

    #[test]
    fn test_gravity_zone_shape_is_volumetric() {
        assert!(GravityZoneShape::Box.is_volumetric());
        assert!(GravityZoneShape::Sphere.is_volumetric());
        assert!(GravityZoneShape::Cylinder.is_volumetric());
        assert!(!GravityZoneShape::Global.is_volumetric());
    }

    #[test]
    fn test_gravity_zone_shape_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(GravityZoneShape::Box);
        set.insert(GravityZoneShape::Sphere);
        assert_eq!(set.len(), 2);
    }

    // NEW: PhysicsTab Display and helper tests
    #[test]
    fn test_physics_tab_all() {
        let tabs = PhysicsTab::all();
        assert_eq!(tabs.len(), 7);
    }

    #[test]
    fn test_physics_tab_display() {
        assert_eq!(format!("{}", PhysicsTab::Overview), "[Chart] Overview");
        assert_eq!(format!("{}", PhysicsTab::Vehicle), "🚗 Vehicle");
    }

    #[test]
    fn test_physics_tab_name() {
        assert_eq!(PhysicsTab::Visualization.name(), "Visualization");
        assert_eq!(PhysicsTab::Destruction.name(), "Destruction");
    }

    #[test]
    fn test_physics_tab_icon() {
        assert_eq!(PhysicsTab::Ragdoll.icon(), "🚶");
        assert_eq!(PhysicsTab::Environment.icon(), "[Glb]");
    }

    #[test]
    fn test_physics_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PhysicsTab::Overview);
        set.insert(PhysicsTab::Vehicle);
        assert_eq!(set.len(), 2);
    }

    // ============================================================
    // PHYSICS ACTION TESTS
    // ============================================================

    #[test]
    fn test_action_system_initial_state() {
        let panel = PhysicsPanel::default();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_action_queue_and_take() {
        let mut panel = PhysicsPanel::default();
        panel.queue_action(PhysicsAction::Play);
        panel.queue_action(PhysicsAction::SetTimeScale(2.0));

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
            (PhysicsAction::Play, "Play"),
            (PhysicsAction::Pause, "Pause"),
            (PhysicsAction::Step, "Step"),
            (PhysicsAction::Reset, "Reset"),
            (PhysicsAction::SetTimeScale(1.5), "Set Time Scale"),
            (
                PhysicsAction::SetSimulationMode(SimulationMode::Running),
                "Set Simulation Mode",
            ),
            (
                PhysicsAction::SetVisualization(PhysicsVisualization::Colliders),
                "Set Visualization",
            ),
            (
                PhysicsAction::ApplyRagdollPreset(RagdollPreset::Humanoid),
                "Apply Ragdoll Preset",
            ),
            (PhysicsAction::SpawnTestRagdoll, "Spawn Test Ragdoll"),
            (
                PhysicsAction::ApplyTestImpulse { strength: 100.0 },
                "Apply Test Impulse",
            ),
            (
                PhysicsAction::SetVehicleType(VehicleType::Sedan),
                "Set Vehicle Type",
            ),
            (
                PhysicsAction::SetClothQuality(ClothQuality::High),
                "Set Cloth Quality",
            ),
            (
                PhysicsAction::SetDestructionPattern(DestructionPattern::Voronoi),
                "Set Destruction Pattern",
            ),
            (
                PhysicsAction::AddGravityZone {
                    name: "zone".to_string(),
                },
                "Add Gravity Zone",
            ),
            (
                PhysicsAction::RemoveGravityZone { index: 0 },
                "Remove Gravity Zone",
            ),
            (PhysicsAction::ToggleWind(true), "Toggle Wind"),
        ];

        for (action, expected_name) in actions {
            assert_eq!(action.name(), expected_name);
        }
    }

    #[test]
    fn test_action_is_simulation_control() {
        assert!(PhysicsAction::Play.is_simulation_control());
        assert!(PhysicsAction::Pause.is_simulation_control());
        assert!(PhysicsAction::Step.is_simulation_control());
        assert!(PhysicsAction::Reset.is_simulation_control());
        assert!(PhysicsAction::SetTimeScale(1.0).is_simulation_control());
        assert!(PhysicsAction::SetSimulationMode(SimulationMode::Paused).is_simulation_control());

        assert!(!PhysicsAction::SetVisualization(PhysicsVisualization::All).is_simulation_control());
        assert!(!PhysicsAction::SpawnTestRagdoll.is_simulation_control());
    }

    #[test]
    fn test_action_is_visualization() {
        assert!(
            PhysicsAction::SetVisualization(PhysicsVisualization::Colliders).is_visualization()
        );
        assert!(PhysicsAction::SetVisualization(PhysicsVisualization::All).is_visualization());

        assert!(!PhysicsAction::Play.is_visualization());
        assert!(!PhysicsAction::SetTimeScale(1.0).is_visualization());
    }

    #[test]
    fn test_action_is_system_config() {
        assert!(PhysicsAction::ApplyRagdollPreset(RagdollPreset::Humanoid).is_system_config());
        assert!(PhysicsAction::SetVehicleType(VehicleType::Truck).is_system_config());
        assert!(PhysicsAction::SetClothQuality(ClothQuality::Low).is_system_config());
        assert!(
            PhysicsAction::SetDestructionPattern(DestructionPattern::Shatter).is_system_config()
        );

        assert!(!PhysicsAction::Play.is_system_config());
        assert!(!PhysicsAction::AddGravityZone {
            name: "test".to_string()
        }
        .is_system_config());
    }

    #[test]
    fn test_action_is_environment() {
        assert!(PhysicsAction::AddGravityZone {
            name: "zone".to_string()
        }
        .is_environment());
        assert!(PhysicsAction::RemoveGravityZone { index: 0 }.is_environment());
        assert!(PhysicsAction::ToggleWind(true).is_environment());

        assert!(!PhysicsAction::Play.is_environment());
        assert!(!PhysicsAction::SetClothQuality(ClothQuality::Medium).is_environment());
    }

    #[test]
    fn test_action_display() {
        let action = PhysicsAction::Play;
        let display = format!("{}", action);
        assert_eq!(display, "Play");

        let action = PhysicsAction::SetTimeScale(2.5);
        let display = format!("{}", action);
        assert_eq!(display, "Set Time Scale");
    }

    #[test]
    fn test_time_scale_initial() {
        let panel = PhysicsPanel::default();
        assert!((panel.time_scale() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_wind_enabled_initial() {
        let panel = PhysicsPanel::default();
        assert!(!panel.wind_enabled());
    }

    #[test]
    fn test_multiple_simulation_actions() {
        let mut panel = PhysicsPanel::default();
        panel.queue_action(PhysicsAction::Play);
        panel.queue_action(PhysicsAction::SetTimeScale(2.0));
        panel.queue_action(PhysicsAction::Pause);

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.iter().all(|a| a.is_simulation_control()));
    }

    #[test]
    fn test_mixed_action_types() {
        let mut panel = PhysicsPanel::default();
        panel.queue_action(PhysicsAction::Play);
        panel.queue_action(PhysicsAction::SetVisualization(
            PhysicsVisualization::Colliders,
        ));
        panel.queue_action(PhysicsAction::ApplyRagdollPreset(RagdollPreset::Humanoid));
        panel.queue_action(PhysicsAction::ToggleWind(true));

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 4);

        // Check categories
        let sim_count = actions.iter().filter(|a| a.is_simulation_control()).count();
        let vis_count = actions.iter().filter(|a| a.is_visualization()).count();
        let config_count = actions.iter().filter(|a| a.is_system_config()).count();
        let env_count = actions.iter().filter(|a| a.is_environment()).count();

        assert_eq!(sim_count, 1);
        assert_eq!(vis_count, 1);
        assert_eq!(config_count, 1);
        assert_eq!(env_count, 1);
    }

    #[test]
    fn test_action_partial_eq() {
        let a1 = PhysicsAction::Play;
        let a2 = PhysicsAction::Play;
        let a3 = PhysicsAction::Pause;

        assert_eq!(a1, a2);
        assert_ne!(a1, a3);

        let s1 = PhysicsAction::SetTimeScale(1.0);
        let s2 = PhysicsAction::SetTimeScale(1.0);
        let s3 = PhysicsAction::SetTimeScale(2.0);

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }
}
