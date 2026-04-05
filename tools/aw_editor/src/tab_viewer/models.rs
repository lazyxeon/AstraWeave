//! Data model structs for the tab viewer.
//!
//! These are pure data types with no UI logic, extracted from `tab_viewer.rs`.

/// Entity info for hierarchy display
#[derive(Debug, Clone)]
pub struct EntityInfo {
    pub id: u64,
    pub name: String,
    pub components: Vec<String>,
    pub entity_type: String,
    /// World component values for inspector editing
    pub hp: Option<i32>,
    pub team_id: Option<u8>,
    pub ammo: Option<i32>,
    pub pos_x: Option<i32>,
    pub pos_y: Option<i32>,
    pub rotation: Option<f32>,
    pub scale: Option<f32>,
    /// Component JSON data for rich inspector editing
    pub component_data: std::collections::HashMap<String, serde_json::Value>,
    /// Material info for PBR editing
    pub material_base_color: [f32; 4],
    pub material_metallic: f32,
    pub material_roughness: f32,
    pub material_emissive: [f32; 3],
    pub material_textures: std::collections::HashMap<String, String>,
}

/// Runtime statistics for profiler display
#[derive(Debug, Clone, Default)]
pub struct RuntimeStatsInfo {
    pub frame_time_ms: f32,
    pub fps: f32,
    pub entity_count: usize,
    pub tick_count: u64,
    pub is_playing: bool,
    pub is_paused: bool,
    /// Render subsystem time in ms
    pub render_time_ms: f32,
    /// Physics subsystem time in ms
    pub physics_time_ms: f32,
    /// AI subsystem time in ms
    pub ai_time_ms: f32,
    /// Script/logic time in ms
    pub script_time_ms: f32,
    /// Audio subsystem time in ms
    pub audio_time_ms: f32,
    /// Draw calls this frame
    pub draw_calls: usize,
    /// Triangles rendered this frame
    pub triangles: usize,
    /// GPU memory usage in bytes
    pub gpu_memory_bytes: usize,
}

/// Scene statistics for stats panel
#[derive(Debug, Clone, Default)]
pub struct SceneStatsInfo {
    pub total_entities: usize,
    pub total_components: usize,
    pub prefab_instances: usize,
    pub selected_count: usize,
    /// Memory usage in bytes (estimated)
    pub memory_usage_bytes: usize,
    /// Number of active systems
    pub active_systems: usize,
    /// Number of loaded assets
    pub loaded_assets: usize,
    /// Number of active lights
    pub light_count: usize,
    /// Number of mesh renderers
    pub mesh_count: usize,
    /// Number of physics bodies
    pub physics_bodies: usize,
    /// Scene modified flag
    pub is_modified: bool,
    /// Number of audio sources
    pub audio_sources: usize,
    /// Number of particle systems
    pub particle_systems: usize,
    /// Number of cameras
    pub camera_count: usize,
    /// Number of colliders
    pub collider_count: usize,
    /// Number of scripts/behaviors
    pub script_count: usize,
    /// Number of UI elements
    pub ui_element_count: usize,
    /// Scene file path (if saved)
    pub scene_path: Option<String>,
    /// Last save timestamp
    pub last_save_time: Option<String>,
}

/// Asset folder entry
#[derive(Debug, Clone)]
pub struct AssetEntry {
    pub name: String,
    pub is_folder: bool,
    pub file_type: String,
}

/// Material properties for material editor
#[derive(Debug, Clone)]
pub struct MaterialInfo {
    pub name: String,
    pub albedo_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub emission: [f32; 3],
    /// Emission strength multiplier
    pub emission_strength: f32,
    /// Normal map strength (0.0 = disabled)
    pub normal_strength: f32,
    /// Ambient occlusion strength
    pub ao_strength: f32,
    /// Alpha/opacity value
    pub alpha: f32,
    /// Two-sided rendering
    pub double_sided: bool,
    /// Texture paths (optional)
    pub albedo_texture: Option<String>,
    pub normal_texture: Option<String>,
    pub metallic_roughness_texture: Option<String>,
    pub emission_texture: Option<String>,
    pub ao_texture: Option<String>,
}

impl Default for MaterialInfo {
    fn default() -> Self {
        Self {
            name: "Default Material".to_string(),
            albedo_color: [0.8, 0.8, 0.8],
            metallic: 0.0,
            roughness: 0.5,
            emission: [0.0, 0.0, 0.0],
            emission_strength: 1.0,
            normal_strength: 1.0,
            ao_strength: 1.0,
            alpha: 1.0,
            double_sided: false,
            albedo_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            emission_texture: None,
            ao_texture: None,
        }
    }
}

/// Theme settings for theme manager
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EditorTheme {
    Dark,
    Light,
    Nord,
    Solarized,
}

impl std::fmt::Display for EditorTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorTheme::Dark => write!(f, "Dark"),
            EditorTheme::Light => write!(f, "Light"),
            EditorTheme::Nord => write!(f, "Nord"),
            EditorTheme::Solarized => write!(f, "Solarized"),
        }
    }
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self::Dark
    }
}

impl EditorTheme {
    /// Returns all available themes.
    pub fn all() -> &'static [Self] {
        &[
            EditorTheme::Dark,
            EditorTheme::Light,
            EditorTheme::Nord,
            EditorTheme::Solarized,
        ]
    }

    /// Returns the display name of this theme.
    pub fn name(&self) -> &'static str {
        match self {
            EditorTheme::Dark => "Dark",
            EditorTheme::Light => "Light",
            EditorTheme::Nord => "Nord",
            EditorTheme::Solarized => "Solarized",
        }
    }

    /// Returns true if this is a dark theme.
    pub fn is_dark(&self) -> bool {
        matches!(self, EditorTheme::Dark | EditorTheme::Nord)
    }

    /// Returns true if this is a light theme.
    pub fn is_light(&self) -> bool {
        matches!(self, EditorTheme::Light | EditorTheme::Solarized)
    }
}

/// Animation timeline state
#[derive(Debug, Clone)]
pub struct AnimationState {
    pub is_playing: bool,
    pub current_frame: u32,
    pub total_frames: u32,
    pub fps: f32,
    pub selected_track: Option<usize>,
    pub tracks: Vec<AnimationTrack>,
    /// Playback speed multiplier (0.25, 0.5, 1.0, 2.0, etc.)
    pub playback_speed: f32,
    /// Loop mode enabled
    pub loop_enabled: bool,
    /// Ping-pong mode (play forward then backward)
    pub ping_pong: bool,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            is_playing: false,
            current_frame: 0,
            total_frames: 120,
            fps: 30.0,
            selected_track: None,
            tracks: Vec::new(),
            playback_speed: 1.0,
            loop_enabled: true,
            ping_pong: false,
        }
    }
}

/// Single animation track
#[derive(Debug, Clone)]
pub struct AnimationTrack {
    pub name: String,
    pub keyframes: Vec<Keyframe>,
    pub is_visible: bool,
    pub is_locked: bool,
}

/// Animation keyframe
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub frame: u32,
    pub value: f32,
}

/// Graph editor node
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: u32,
    pub name: String,
    pub node_type: String,
    pub position: (f32, f32),
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

/// Behavior graph state
#[derive(Debug, Clone)]
pub struct BehaviorGraphState {
    pub nodes: Vec<BehaviorNode>,
    pub connections: Vec<(u32, u32)>,
    pub selected_node: Option<u32>,
}

/// Behavior tree node types
#[derive(Debug, Clone)]
pub struct BehaviorNode {
    pub id: u32,
    pub name: String,
    pub node_type: BehaviorNodeType,
    pub position: (f32, f32),
    pub children: Vec<u32>,
}

/// Behavior node categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BehaviorNodeType {
    Root,
    Sequence,
    Selector,
    Condition,
    Action,
    Decorator,
}

impl std::fmt::Display for BehaviorNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BehaviorNodeType::Root => write!(f, "Root"),
            BehaviorNodeType::Sequence => write!(f, "Sequence"),
            BehaviorNodeType::Selector => write!(f, "Selector"),
            BehaviorNodeType::Condition => write!(f, "Condition"),
            BehaviorNodeType::Action => write!(f, "Action"),
            BehaviorNodeType::Decorator => write!(f, "Decorator"),
        }
    }
}

impl Default for BehaviorNodeType {
    fn default() -> Self {
        Self::Action
    }
}

impl BehaviorNodeType {
    /// Returns all behavior node types.
    pub fn all() -> &'static [Self] {
        &[
            BehaviorNodeType::Root,
            BehaviorNodeType::Sequence,
            BehaviorNodeType::Selector,
            BehaviorNodeType::Condition,
            BehaviorNodeType::Action,
            BehaviorNodeType::Decorator,
        ]
    }

    /// Returns the display name of this node type.
    pub fn name(&self) -> &'static str {
        match self {
            BehaviorNodeType::Root => "Root",
            BehaviorNodeType::Sequence => "Sequence",
            BehaviorNodeType::Selector => "Selector",
            BehaviorNodeType::Condition => "Condition",
            BehaviorNodeType::Action => "Action",
            BehaviorNodeType::Decorator => "Decorator",
        }
    }

    /// Returns an icon for this node type.
    pub fn icon(&self) -> &'static str {
        match self {
            BehaviorNodeType::Root => "R",
            BehaviorNodeType::Sequence => ">",
            BehaviorNodeType::Selector => "?",
            BehaviorNodeType::Condition => "?!",
            BehaviorNodeType::Action => "A",
            BehaviorNodeType::Decorator => "D",
        }
    }

    /// Returns true if this is a composite node type.
    pub fn is_composite(&self) -> bool {
        matches!(
            self,
            BehaviorNodeType::Sequence | BehaviorNodeType::Selector
        )
    }

    /// Returns true if this is a leaf node type.
    pub fn is_leaf(&self) -> bool {
        matches!(self, BehaviorNodeType::Condition | BehaviorNodeType::Action)
    }

    /// Returns true if this node can have children.
    pub fn can_have_children(&self) -> bool {
        matches!(
            self,
            BehaviorNodeType::Root
                | BehaviorNodeType::Sequence
                | BehaviorNodeType::Selector
                | BehaviorNodeType::Decorator
        )
    }
}
