//! PanelEvent enum and related helpers.
//!
//! Extracted from the monolithic `tab_viewer.rs` for maintainability.

use super::EditorTheme;
use crate::panel_type::PanelType;

/// Panel events that can be emitted from the tab viewer
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PanelEvent {
    /// A panel was closed
    PanelClosed(PanelType),
    /// A panel was focused
    PanelFocused(PanelType),
    /// Request to add a panel
    AddPanel(PanelType),
    /// An entity was selected in a panel
    EntitySelected(u64),
    /// An entity was deselected
    EntityDeselected,
    /// Transform position changed
    TransformPositionChanged {
        entity_id: u64,
        x: f32,
        y: f32,
        z: f32,
    },
    /// Transform rotation changed
    TransformRotationChanged { entity_id: u64, rotation: f32 },
    /// Transform scale changed
    TransformScaleChanged {
        entity_id: u64,
        scale_x: f32,
        scale_y: f32,
        scale_z: f32,
    },
    /// Request to create a new empty entity
    CreateEntity,
    /// Request to spawn an entity from a named archetype (e.g. "Player", "Light", "Camera")
    SpawnArchetype { archetype: String },
    /// Request to spawn an entity from a 3D model asset path
    SpawnModel { name: String, path: String },
    /// Request to delete an entity
    DeleteEntity(u64),
    /// Request to duplicate an entity
    DuplicateEntity(u64),
    /// Material property changed
    MaterialChanged {
        name: String,
        property: String,
        value: f32,
    },
    /// Animation playback state changed
    AnimationPlayStateChanged { is_playing: bool },
    /// Animation frame changed
    AnimationFrameChanged { frame: u32 },
    /// Animation keyframe added
    AnimationKeyframeAdded {
        track_index: usize,
        frame: u32,
        value: f32,
    },
    /// Theme changed
    ThemeChanged(EditorTheme),
    /// Build requested
    BuildRequested { target: String, profile: String },
    /// Console cleared
    ConsoleCleared,
    /// Asset selected in browser
    AssetSelected(String),
    /// Behavior node selected
    BehaviorNodeSelected(u32),
    /// Graph node selected
    GraphNodeSelected(u32),
    /// Hierarchy search changed
    HierarchySearchChanged(String),
    /// Console search changed
    ConsoleSearchChanged(String),
    /// Request to refresh scene statistics
    RefreshSceneStats,
    /// Request to add a component to an entity
    AddComponent {
        entity_id: u64,
        component_type: String,
    },
    /// Request to remove a component from an entity
    RemoveComponent {
        entity_id: u64,
        component_type: String,
    },
    /// Health component value changed
    HealthChanged { entity_id: u64, new_hp: i32 },
    /// Team component value changed
    TeamChanged { entity_id: u64, new_team_id: u8 },
    /// Ammo component value changed
    AmmoChanged { entity_id: u64, new_ammo: i32 },
    /// Generic component data changed (Light, Collider, RigidBody, Audio, Camera, Script, etc.)
    ComponentDataChanged {
        entity_id: u64,
        component_type: String,
        data: serde_json::Value,
    },
    /// Material PBR property changed
    MaterialPropertyChanged {
        entity_id: u64,
        property: String,
        value: serde_json::Value,
    },
    /// Material texture slot changed
    MaterialTextureChanged {
        entity_id: u64,
        slot: String,
        path: String,
    },
    /// Entity name changed
    EntityRenamed { entity_id: u64, new_name: String },
    /// Viewport view mode changed (0=Shaded, 1=Wireframe, 2=Unlit, 3=Normals, 4=UVs)
    ViewportViewModeChanged(usize),
    /// Viewport gizmo mode changed (0=Translate, 1=Rotate, 2=Scale)
    ViewportGizmoModeChanged(usize),
    /// Viewport gizmo space changed (0=Local, 1=World)
    ViewportGizmoSpaceChanged(usize),
    /// Viewport overlay toggled
    ViewportOverlayToggled { overlay: String, enabled: bool },
    /// Viewport camera settings changed
    ViewportCameraChanged {
        fov: f32,
        near: f32,
        far: f32,
        speed: f32,
    },
    /// Request to focus viewport on selected entity
    ViewportFocusOnSelection,
    /// Request to reset viewport camera
    ViewportResetCamera,
    /// Viewport camera preset applied (front, top, side, perspective)
    ViewportCameraPreset(String),
    /// HDRI skybox loaded from file
    HdriLoaded { path: std::path::PathBuf },
    /// HDRI skybox removed
    HdriCleared,
    /// Terrain was generated and is ready for viewport upload
    TerrainReady,
    /// Terrain brush stroke applied — only dirty chunks need GPU update
    TerrainBrushUpdate,
    /// Request to reset panel layout to default
    ResetLayout,
}

impl std::fmt::Display for PanelEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PanelEvent::PanelClosed(panel) => write!(f, "Panel Closed: {}", panel),
            PanelEvent::PanelFocused(panel) => write!(f, "Panel Focused: {}", panel),
            PanelEvent::AddPanel(panel) => write!(f, "Add Panel: {}", panel),
            PanelEvent::EntitySelected(id) => write!(f, "Entity Selected: {}", id),
            PanelEvent::EntityDeselected => write!(f, "Entity Deselected"),
            PanelEvent::TransformPositionChanged { entity_id, .. } => {
                write!(f, "Transform Position Changed: {}", entity_id)
            }
            PanelEvent::TransformRotationChanged { entity_id, .. } => {
                write!(f, "Transform Rotation Changed: {}", entity_id)
            }
            PanelEvent::TransformScaleChanged { entity_id, .. } => {
                write!(f, "Transform Scale Changed: {}", entity_id)
            }
            PanelEvent::CreateEntity => write!(f, "Create Entity"),
            PanelEvent::SpawnArchetype { ref archetype } => {
                write!(f, "Spawn Archetype: {}", archetype)
            }
            PanelEvent::SpawnModel { ref name, .. } => {
                write!(f, "Spawn Model: {}", name)
            }
            PanelEvent::DeleteEntity(id) => write!(f, "Delete Entity: {}", id),
            PanelEvent::DuplicateEntity(id) => write!(f, "Duplicate Entity: {}", id),
            PanelEvent::MaterialChanged { name, property, .. } => {
                write!(f, "Material Changed: {}.{}", name, property)
            }
            PanelEvent::AnimationPlayStateChanged { is_playing } => {
                write!(
                    f,
                    "Animation {}",
                    if *is_playing { "Playing" } else { "Paused" }
                )
            }
            PanelEvent::AnimationFrameChanged { frame } => {
                write!(f, "Animation Frame: {}", frame)
            }
            PanelEvent::AnimationKeyframeAdded {
                track_index, frame, ..
            } => {
                write!(f, "Keyframe Added: Track {} @ {}", track_index, frame)
            }
            PanelEvent::ThemeChanged(theme) => write!(f, "Theme Changed: {}", theme),
            PanelEvent::BuildRequested { target, profile } => {
                write!(f, "Build Requested: {} ({})", target, profile)
            }
            PanelEvent::ConsoleCleared => write!(f, "Console Cleared"),
            PanelEvent::AssetSelected(path) => write!(f, "Asset Selected: {}", path),
            PanelEvent::BehaviorNodeSelected(id) => write!(f, "Behavior Node Selected: {}", id),
            PanelEvent::GraphNodeSelected(id) => write!(f, "Graph Node Selected: {}", id),
            PanelEvent::HierarchySearchChanged(query) => {
                write!(f, "Hierarchy Search: {}", query)
            }
            PanelEvent::ConsoleSearchChanged(query) => write!(f, "Console Search: {}", query),
            PanelEvent::RefreshSceneStats => write!(f, "Refresh Scene Stats"),
            PanelEvent::AddComponent { component_type, .. } => {
                write!(f, "Add Component: {}", component_type)
            }
            PanelEvent::RemoveComponent { component_type, .. } => {
                write!(f, "Remove Component: {}", component_type)
            }
            PanelEvent::HealthChanged { entity_id, new_hp } => {
                write!(f, "Health Changed: {} -> {}", entity_id, new_hp)
            }
            PanelEvent::TeamChanged {
                entity_id,
                new_team_id,
            } => {
                write!(f, "Team Changed: {} -> {}", entity_id, new_team_id)
            }
            PanelEvent::AmmoChanged {
                entity_id,
                new_ammo,
            } => {
                write!(f, "Ammo Changed: {} -> {}", entity_id, new_ammo)
            }
            PanelEvent::ComponentDataChanged {
                entity_id,
                ref component_type,
                ..
            } => {
                write!(
                    f,
                    "Component Data Changed: {} on {}",
                    component_type, entity_id
                )
            }
            PanelEvent::MaterialPropertyChanged {
                entity_id,
                ref property,
                ..
            } => {
                write!(
                    f,
                    "Material Property Changed: {} on {}",
                    property, entity_id
                )
            }
            PanelEvent::MaterialTextureChanged {
                entity_id,
                ref slot,
                ref path,
            } => {
                write!(
                    f,
                    "Material Texture Changed: {} = {} on {}",
                    slot, path, entity_id
                )
            }
            PanelEvent::EntityRenamed {
                entity_id,
                ref new_name,
            } => {
                write!(f, "Entity Renamed: {} -> {}", entity_id, new_name)
            }
            PanelEvent::ViewportViewModeChanged(mode) => {
                write!(f, "Viewport View Mode: {}", mode)
            }
            PanelEvent::ViewportGizmoModeChanged(mode) => {
                write!(f, "Viewport Gizmo Mode: {}", mode)
            }
            PanelEvent::ViewportGizmoSpaceChanged(space) => {
                write!(f, "Viewport Gizmo Space: {}", space)
            }
            PanelEvent::ViewportOverlayToggled { overlay, enabled } => {
                write!(f, "Viewport Overlay {}: {}", overlay, enabled)
            }
            PanelEvent::ViewportCameraChanged { .. } => write!(f, "Viewport Camera Changed"),
            PanelEvent::ViewportFocusOnSelection => write!(f, "Viewport Focus On Selection"),
            PanelEvent::ViewportResetCamera => write!(f, "Viewport Reset Camera"),
            PanelEvent::ViewportCameraPreset(preset) => {
                write!(f, "Viewport Camera Preset: {}", preset)
            }
            PanelEvent::HdriLoaded { ref path } => {
                write!(f, "HDRI Loaded: {}", path.display())
            }
            PanelEvent::HdriCleared => write!(f, "HDRI Cleared"),
            PanelEvent::TerrainReady => write!(f, "Terrain Ready"),
            PanelEvent::TerrainBrushUpdate => write!(f, "Terrain Brush Update"),
            PanelEvent::ResetLayout => write!(f, "Reset Layout"),
        }
    }
}

impl PanelEvent {
    /// Returns the event category name.
    pub fn category(&self) -> &'static str {
        match self {
            PanelEvent::PanelClosed(_)
            | PanelEvent::PanelFocused(_)
            | PanelEvent::AddPanel(_)
            | PanelEvent::ResetLayout => "Panel",
            PanelEvent::EntitySelected(_)
            | PanelEvent::EntityDeselected
            | PanelEvent::CreateEntity
            | PanelEvent::SpawnArchetype { .. }
            | PanelEvent::SpawnModel { .. }
            | PanelEvent::DeleteEntity(_)
            | PanelEvent::DuplicateEntity(_) => "Entity",
            PanelEvent::TransformPositionChanged { .. }
            | PanelEvent::TransformRotationChanged { .. }
            | PanelEvent::TransformScaleChanged { .. } => "Transform",
            PanelEvent::MaterialChanged { .. } => "Material",
            PanelEvent::AnimationPlayStateChanged { .. }
            | PanelEvent::AnimationFrameChanged { .. }
            | PanelEvent::AnimationKeyframeAdded { .. } => "Animation",
            PanelEvent::ThemeChanged(_) => "Theme",
            PanelEvent::BuildRequested { .. } => "Build",
            PanelEvent::ConsoleCleared | PanelEvent::ConsoleSearchChanged(_) => "Console",
            PanelEvent::AssetSelected(_) => "Asset",
            PanelEvent::BehaviorNodeSelected(_) | PanelEvent::GraphNodeSelected(_) => "Graph",
            PanelEvent::HierarchySearchChanged(_) => "Hierarchy",
            PanelEvent::RefreshSceneStats => "Scene",
            PanelEvent::AddComponent { .. }
            | PanelEvent::RemoveComponent { .. }
            | PanelEvent::HealthChanged { .. }
            | PanelEvent::TeamChanged { .. }
            | PanelEvent::AmmoChanged { .. }
            | PanelEvent::ComponentDataChanged { .. } => "Component",
            PanelEvent::MaterialPropertyChanged { .. }
            | PanelEvent::MaterialTextureChanged { .. } => "Material",
            PanelEvent::EntityRenamed { .. } => "Entity",
            PanelEvent::ViewportViewModeChanged(_)
            | PanelEvent::ViewportGizmoModeChanged(_)
            | PanelEvent::ViewportGizmoSpaceChanged(_)
            | PanelEvent::ViewportOverlayToggled { .. }
            | PanelEvent::ViewportCameraChanged { .. }
            | PanelEvent::ViewportFocusOnSelection
            | PanelEvent::ViewportResetCamera
            | PanelEvent::ViewportCameraPreset(_) => "Viewport",
            PanelEvent::HdriLoaded { .. } | PanelEvent::HdriCleared => "Skybox",
            PanelEvent::TerrainReady | PanelEvent::TerrainBrushUpdate => "Terrain",
        }
    }

    /// Returns true if this is a panel management event.
    pub fn is_panel_event(&self) -> bool {
        matches!(
            self,
            PanelEvent::PanelClosed(_)
                | PanelEvent::PanelFocused(_)
                | PanelEvent::AddPanel(_)
                | PanelEvent::ResetLayout
        )
    }

    /// Returns true if this is an entity-related event.
    pub fn is_entity_event(&self) -> bool {
        matches!(
            self,
            PanelEvent::EntitySelected(_)
                | PanelEvent::EntityDeselected
                | PanelEvent::CreateEntity
                | PanelEvent::DeleteEntity(_)
                | PanelEvent::DuplicateEntity(_)
        )
    }

    /// Returns true if this is a transform event.
    pub fn is_transform_event(&self) -> bool {
        matches!(
            self,
            PanelEvent::TransformPositionChanged { .. }
                | PanelEvent::TransformRotationChanged { .. }
                | PanelEvent::TransformScaleChanged { .. }
        )
    }

    /// Returns true if this is a viewport event.
    pub fn is_viewport_event(&self) -> bool {
        matches!(
            self,
            PanelEvent::ViewportViewModeChanged(_)
                | PanelEvent::ViewportGizmoModeChanged(_)
                | PanelEvent::ViewportGizmoSpaceChanged(_)
                | PanelEvent::ViewportOverlayToggled { .. }
                | PanelEvent::ViewportCameraChanged { .. }
                | PanelEvent::ViewportFocusOnSelection
                | PanelEvent::ViewportResetCamera
                | PanelEvent::ViewportCameraPreset(_)
        )
    }

    /// Returns the entity ID if this event references one.
    pub fn entity_id(&self) -> Option<u64> {
        match self {
            PanelEvent::EntitySelected(id)
            | PanelEvent::DeleteEntity(id)
            | PanelEvent::DuplicateEntity(id)
            | PanelEvent::TransformPositionChanged { entity_id: id, .. }
            | PanelEvent::TransformRotationChanged { entity_id: id, .. }
            | PanelEvent::TransformScaleChanged { entity_id: id, .. }
            | PanelEvent::AddComponent { entity_id: id, .. }
            | PanelEvent::RemoveComponent { entity_id: id, .. }
            | PanelEvent::ComponentDataChanged { entity_id: id, .. }
            | PanelEvent::MaterialPropertyChanged { entity_id: id, .. }
            | PanelEvent::MaterialTextureChanged { entity_id: id, .. } => Some(*id),
            _ => None,
        }
    }
}

/// Callback type for panel events
pub type PanelEventCallback = Box<dyn FnMut(PanelEvent) + Send>;
