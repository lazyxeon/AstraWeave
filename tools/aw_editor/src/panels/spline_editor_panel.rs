//! Spline Editor Panel for the editor
//!
//! Provides comprehensive spline/path editing:
//! - Bezier, Catmull-Rom, and linear spline types
//! - Point manipulation and tangent controls
//! - Path extrusion for roads, rails, rivers
//! - Animation paths and camera rails
//! - Mesh generation along splines
//! - Spline-based terrain modifications

use egui::{Color32, RichText, Ui, Vec2};
use tracing::info;

use crate::panels::Panel;

/// Spline type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SplineType {
    #[default]
    CatmullRom,
    Bezier,
    Linear,
    Hermite,
    BSpline,
}

impl std::fmt::Display for SplineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl SplineType {
    pub fn name(&self) -> &'static str {
        match self {
            SplineType::CatmullRom => "Catmull-Rom",
            SplineType::Bezier => "Bezier",
            SplineType::Linear => "Linear",
            SplineType::Hermite => "Hermite",
            SplineType::BSpline => "B-Spline",
        }
    }

    pub fn all() -> &'static [SplineType] {
        &[
            SplineType::CatmullRom,
            SplineType::Bezier,
            SplineType::Linear,
            SplineType::Hermite,
            SplineType::BSpline,
        ]
    }

    pub fn is_smooth(&self) -> bool {
        !matches!(self, SplineType::Linear)
    }
}

/// Spline preset category
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SplinePreset {
    #[default]
    Custom,
    Road,
    Rail,
    River,
    Fence,
    Cable,
    AnimationPath,
    CameraRail,
}

impl std::fmt::Display for SplinePreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SplinePreset {
    pub fn name(&self) -> &'static str {
        match self {
            SplinePreset::Custom => "Custom",
            SplinePreset::Road => "Road",
            SplinePreset::Rail => "Rail",
            SplinePreset::River => "River",
            SplinePreset::Fence => "Fence",
            SplinePreset::Cable => "Cable",
            SplinePreset::AnimationPath => "Animation Path",
            SplinePreset::CameraRail => "Camera Rail",
        }
    }

    pub fn all() -> &'static [SplinePreset] {
        &[
            SplinePreset::Custom,
            SplinePreset::Road,
            SplinePreset::Rail,
            SplinePreset::River,
            SplinePreset::Fence,
            SplinePreset::Cable,
            SplinePreset::AnimationPath,
            SplinePreset::CameraRail,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SplinePreset::Custom => "[Sq]",
            SplinePreset::Road => "🛣️",
            SplinePreset::Rail => "🚃",
            SplinePreset::River => "[Wave]",
            SplinePreset::Fence => "[Brk]",
            SplinePreset::Cable => "🔌",
            SplinePreset::AnimationPath => "[Anim]",
            SplinePreset::CameraRail => "🎥",
        }
    }

    pub fn is_infrastructure(&self) -> bool {
        matches!(
            self,
            SplinePreset::Road | SplinePreset::Rail | SplinePreset::Fence | SplinePreset::Cable
        )
    }
}

/// Control point on the spline
#[derive(Debug, Clone)]
pub struct SplinePoint {
    pub id: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],

    // Tangent control (for Bezier)
    pub in_tangent: [f32; 3],
    pub out_tangent: [f32; 3],
    pub tangent_mode: TangentMode,

    // Per-point settings
    pub roll: f32,
    pub width: f32,
    pub custom_data: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TangentMode {
    #[default]
    Auto,
    Smooth,
    Linear,
    Free,
    Aligned,
}

impl std::fmt::Display for TangentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TangentMode {
    pub fn name(&self) -> &'static str {
        match self {
            TangentMode::Auto => "Auto",
            TangentMode::Smooth => "Smooth",
            TangentMode::Linear => "Linear",
            TangentMode::Free => "Free",
            TangentMode::Aligned => "Aligned",
        }
    }

    pub fn all() -> &'static [TangentMode] {
        &[
            TangentMode::Auto,
            TangentMode::Smooth,
            TangentMode::Linear,
            TangentMode::Free,
            TangentMode::Aligned,
        ]
    }
}

impl Default for SplinePoint {
    fn default() -> Self {
        Self {
            id: 0,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            in_tangent: [-1.0, 0.0, 0.0],
            out_tangent: [1.0, 0.0, 0.0],
            tangent_mode: TangentMode::Auto,
            roll: 0.0,
            width: 1.0,
            custom_data: 0.0,
        }
    }
}

/// Spline definition
#[derive(Debug, Clone)]
pub struct Spline {
    pub id: u32,
    pub name: String,
    pub spline_type: SplineType,
    pub preset: SplinePreset,
    pub closed: bool,
    pub points: Vec<SplinePoint>,

    // Visual
    pub color: [f32; 3],
    pub thickness: f32,
    pub show_points: bool,
    pub show_tangents: bool,

    // Mesh generation
    pub generate_mesh: bool,
    pub mesh_profile: MeshProfile,
    pub uv_mode: UvMode,
    pub segment_length: f32,

    // Deformation
    pub deform_terrain: bool,
    pub terrain_width: f32,
    pub terrain_falloff: f32,
    pub terrain_height_offset: f32,

    // Animation
    pub animation_duration: f32,
    pub loop_animation: bool,

    // Computed
    pub total_length: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MeshProfile {
    #[default]
    Flat,
    Round,
    Square,
    RoadWithSidewalk,
    RiverBed,
    Custom,
}

impl std::fmt::Display for MeshProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl MeshProfile {
    pub fn name(&self) -> &'static str {
        match self {
            MeshProfile::Flat => "Flat",
            MeshProfile::Round => "Round",
            MeshProfile::Square => "Square",
            MeshProfile::RoadWithSidewalk => "Road with Sidewalk",
            MeshProfile::RiverBed => "River Bed",
            MeshProfile::Custom => "Custom",
        }
    }

    pub fn all() -> &'static [MeshProfile] {
        &[
            MeshProfile::Flat,
            MeshProfile::Round,
            MeshProfile::Square,
            MeshProfile::RoadWithSidewalk,
            MeshProfile::RiverBed,
            MeshProfile::Custom,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UvMode {
    #[default]
    Stretch,
    Tile,
    TileWorld,
}

impl std::fmt::Display for UvMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl UvMode {
    pub fn name(&self) -> &'static str {
        match self {
            UvMode::Stretch => "Stretch",
            UvMode::Tile => "Tile",
            UvMode::TileWorld => "Tile World",
        }
    }

    pub fn all() -> &'static [UvMode] {
        &[UvMode::Stretch, UvMode::Tile, UvMode::TileWorld]
    }
}

impl Default for Spline {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Spline".to_string(),
            spline_type: SplineType::CatmullRom,
            preset: SplinePreset::Custom,
            closed: false,
            points: Vec::new(),

            color: [0.2, 0.6, 1.0],
            thickness: 2.0,
            show_points: true,
            show_tangents: true,

            generate_mesh: false,
            mesh_profile: MeshProfile::Flat,
            uv_mode: UvMode::Tile,
            segment_length: 1.0,

            deform_terrain: false,
            terrain_width: 5.0,
            terrain_falloff: 2.0,
            terrain_height_offset: 0.0,

            animation_duration: 10.0,
            loop_animation: true,

            total_length: 0.0,
        }
    }
}

/// Editor tool
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SplineTool {
    #[default]
    Select,
    AddPoint,
    InsertPoint,
    DeletePoint,
    MoveTangent,
    Draw,
}

impl std::fmt::Display for SplineTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SplineTool {
    pub fn name(&self) -> &'static str {
        match self {
            SplineTool::Select => "Select",
            SplineTool::AddPoint => "Add Point",
            SplineTool::InsertPoint => "Insert Point",
            SplineTool::DeletePoint => "Delete Point",
            SplineTool::MoveTangent => "Move Tangent",
            SplineTool::Draw => "Draw",
        }
    }

    pub fn all() -> &'static [SplineTool] {
        &[
            SplineTool::Select,
            SplineTool::AddPoint,
            SplineTool::InsertPoint,
            SplineTool::DeletePoint,
            SplineTool::MoveTangent,
            SplineTool::Draw,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SplineTool::Select => "👆",
            SplineTool::AddPoint => "+",
            SplineTool::InsertPoint => "📍",
            SplineTool::DeletePoint => "[x]",
            SplineTool::MoveTangent => "↗️",
            SplineTool::Draw => "✏️",
        }
    }

    pub fn is_destructive(&self) -> bool {
        matches!(self, SplineTool::DeletePoint)
    }
}

/// Mesh profile definition (for extrusion)
#[derive(Debug, Clone)]
pub struct ProfilePoint {
    pub x: f32,
    pub y: f32,
    pub uv: f32,
}

#[derive(Debug, Clone)]
pub struct CustomProfile {
    pub name: String,
    pub points: Vec<ProfilePoint>,
}

impl Default for CustomProfile {
    fn default() -> Self {
        Self {
            name: "Custom".to_string(),
            points: vec![
                ProfilePoint {
                    x: -1.0,
                    y: 0.0,
                    uv: 0.0,
                },
                ProfilePoint {
                    x: 1.0,
                    y: 0.0,
                    uv: 1.0,
                },
            ],
        }
    }
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SplineTab {
    #[default]
    Splines,
    Points,
    Mesh,
    Terrain,
    Animation,
    Profiles,
}

impl std::fmt::Display for SplineTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SplineTab {
    pub fn name(&self) -> &'static str {
        match self {
            SplineTab::Splines => "Splines",
            SplineTab::Points => "Points",
            SplineTab::Mesh => "Mesh",
            SplineTab::Terrain => "Terrain",
            SplineTab::Animation => "Animation",
            SplineTab::Profiles => "Profiles",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SplineTab::Splines => "〰️",
            SplineTab::Points => "📍",
            SplineTab::Mesh => "[D]",
            SplineTab::Terrain => "🏔️",
            SplineTab::Animation => "[Anim]",
            SplineTab::Profiles => "[Sq]",
        }
    }

    pub fn all() -> &'static [SplineTab] {
        &[
            SplineTab::Splines,
            SplineTab::Points,
            SplineTab::Mesh,
            SplineTab::Terrain,
            SplineTab::Animation,
            SplineTab::Profiles,
        ]
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// SPLINE EDITOR ACTION
// ═══════════════════════════════════════════════════════════════════════════════════

/// Actions that can be triggered from the spline editor panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SplineEditorAction {
    // Tab navigation
    SetActiveTab(SplineTab),

    // Tool selection
    SetTool(SplineTool),

    // Spline management
    CreateSpline,
    DeleteSpline(u32),
    DuplicateSpline(u32),
    SelectSpline(u32),
    DeselectSpline,
    RenameSpline(u32, String),

    // Spline properties
    SetSplineType(u32, SplineType),
    SetSplinePreset(u32, SplinePreset),
    ToggleClosed(u32, bool),
    SetSplineColor(u32, [f32; 3]),
    SetSplineThickness(u32, f32),
    ToggleShowPoints(u32, bool),
    ToggleShowTangents(u32, bool),

    // Point operations
    AddPoint(u32, [f32; 3]),
    InsertPoint(u32, usize, [f32; 3]),
    DeletePoint(u32, u32),
    SelectPoint(u32),
    DeselectPoint,
    SetPointPosition(u32, u32, [f32; 3]),
    SetPointRotation(u32, u32, [f32; 3]),
    SetPointScale(u32, u32, [f32; 3]),
    SetPointTangentMode(u32, u32, TangentMode),
    SetPointInTangent(u32, u32, [f32; 3]),
    SetPointOutTangent(u32, u32, [f32; 3]),
    SetPointRoll(u32, u32, f32),
    SetPointWidth(u32, u32, f32),

    // Mesh generation
    ToggleGenerateMesh(u32, bool),
    SetMeshProfile(u32, MeshProfile),
    SetUvMode(u32, UvMode),
    SetSegmentLength(u32, f32),
    RebuildMesh(u32),

    // Terrain deformation
    ToggleDeformTerrain(u32, bool),
    SetTerrainWidth(u32, f32),
    SetTerrainFalloff(u32, f32),
    SetTerrainHeightOffset(u32, f32),
    ApplyTerrainDeformation(u32),

    // Animation
    SetAnimationDuration(u32, f32),
    ToggleLoopAnimation(u32, bool),
    PreviewAnimation(u32),
    StopAnimationPreview,

    // Custom profiles
    AddCustomProfile,
    DeleteCustomProfile(usize),
    RenameProfile(usize, String),
    AddProfilePoint(usize, f32, f32),
    RemoveProfilePoint(usize, usize),

    // Editor settings
    ToggleSnapToGrid(bool),
    SetGridSize(f32),
    ToggleAutoSmooth(bool),
    TogglePreviewMesh(bool),

    // Bulk operations
    SelectAllPoints(u32),
    ReverseSpline(u32),
    SimplifySpline(u32, f32),
    SubdivideSpline(u32),
    SmoothSpline(u32),

    // Import/Export
    ExportSpline(u32),
    ImportSpline,
    ExportAllSplines,
}

impl std::fmt::Display for SplineEditorAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SplineEditorAction::SetActiveTab(tab) => write!(f, "Set tab: {}", tab),
            SplineEditorAction::SetTool(tool) => write!(f, "Set tool: {}", tool),
            SplineEditorAction::CreateSpline => write!(f, "Create spline"),
            SplineEditorAction::DeleteSpline(id) => write!(f, "Delete spline {}", id),
            SplineEditorAction::DuplicateSpline(id) => write!(f, "Duplicate spline {}", id),
            SplineEditorAction::SelectSpline(id) => write!(f, "Select spline {}", id),
            SplineEditorAction::DeselectSpline => write!(f, "Deselect spline"),
            SplineEditorAction::RenameSpline(id, n) => write!(f, "Rename spline {}: {}", id, n),
            SplineEditorAction::SetSplineType(id, t) => write!(f, "Set spline {} type: {}", id, t),
            SplineEditorAction::SetSplinePreset(id, p) => {
                write!(f, "Set spline {} preset: {}", id, p)
            }
            SplineEditorAction::ToggleClosed(id, b) => {
                write!(f, "Toggle spline {} closed: {}", id, b)
            }
            SplineEditorAction::SetSplineColor(id, _) => write!(f, "Set spline {} color", id),
            SplineEditorAction::SetSplineThickness(id, t) => {
                write!(f, "Set spline {} thickness: {:.1}", id, t)
            }
            SplineEditorAction::ToggleShowPoints(id, b) => {
                write!(f, "Toggle spline {} points: {}", id, b)
            }
            SplineEditorAction::ToggleShowTangents(id, b) => {
                write!(f, "Toggle spline {} tangents: {}", id, b)
            }
            SplineEditorAction::AddPoint(id, pos) => write!(
                f,
                "Add point to spline {} at ({:.1}, {:.1}, {:.1})",
                id, pos[0], pos[1], pos[2]
            ),
            SplineEditorAction::InsertPoint(id, i, _) => {
                write!(f, "Insert point to spline {} at index {}", id, i)
            }
            SplineEditorAction::DeletePoint(sid, pid) => {
                write!(f, "Delete point {} from spline {}", pid, sid)
            }
            SplineEditorAction::SelectPoint(id) => write!(f, "Select point {}", id),
            SplineEditorAction::DeselectPoint => write!(f, "Deselect point"),
            SplineEditorAction::SetPointPosition(sid, pid, _) => {
                write!(f, "Set point {}/{} position", sid, pid)
            }
            SplineEditorAction::SetPointRotation(sid, pid, _) => {
                write!(f, "Set point {}/{} rotation", sid, pid)
            }
            SplineEditorAction::SetPointScale(sid, pid, _) => {
                write!(f, "Set point {}/{} scale", sid, pid)
            }
            SplineEditorAction::SetPointTangentMode(sid, pid, m) => {
                write!(f, "Set point {}/{} tangent: {}", sid, pid, m)
            }
            SplineEditorAction::SetPointInTangent(sid, pid, _) => {
                write!(f, "Set point {}/{} in-tangent", sid, pid)
            }
            SplineEditorAction::SetPointOutTangent(sid, pid, _) => {
                write!(f, "Set point {}/{} out-tangent", sid, pid)
            }
            SplineEditorAction::SetPointRoll(sid, pid, r) => {
                write!(f, "Set point {}/{} roll: {:.1}°", sid, pid, r)
            }
            SplineEditorAction::SetPointWidth(sid, pid, w) => {
                write!(f, "Set point {}/{} width: {:.2}", sid, pid, w)
            }
            SplineEditorAction::ToggleGenerateMesh(id, b) => {
                write!(f, "Toggle spline {} mesh gen: {}", id, b)
            }
            SplineEditorAction::SetMeshProfile(id, p) => {
                write!(f, "Set spline {} profile: {}", id, p)
            }
            SplineEditorAction::SetUvMode(id, m) => write!(f, "Set spline {} UV mode: {}", id, m),
            SplineEditorAction::SetSegmentLength(id, l) => {
                write!(f, "Set spline {} segment: {:.2}", id, l)
            }
            SplineEditorAction::RebuildMesh(id) => write!(f, "Rebuild mesh for spline {}", id),
            SplineEditorAction::ToggleDeformTerrain(id, b) => {
                write!(f, "Toggle spline {} terrain: {}", id, b)
            }
            SplineEditorAction::SetTerrainWidth(id, w) => {
                write!(f, "Set spline {} terrain width: {:.1}", id, w)
            }
            SplineEditorAction::SetTerrainFalloff(id, v) => {
                write!(f, "Set spline {} terrain falloff: {:.1}", id, v)
            }
            SplineEditorAction::SetTerrainHeightOffset(id, h) => {
                write!(f, "Set spline {} terrain height: {:.1}", id, h)
            }
            SplineEditorAction::ApplyTerrainDeformation(id) => {
                write!(f, "Apply terrain deformation for spline {}", id)
            }
            SplineEditorAction::SetAnimationDuration(id, d) => {
                write!(f, "Set spline {} anim duration: {:.1}s", id, d)
            }
            SplineEditorAction::ToggleLoopAnimation(id, b) => {
                write!(f, "Toggle spline {} anim loop: {}", id, b)
            }
            SplineEditorAction::PreviewAnimation(id) => {
                write!(f, "Preview animation for spline {}", id)
            }
            SplineEditorAction::StopAnimationPreview => write!(f, "Stop animation preview"),
            SplineEditorAction::AddCustomProfile => write!(f, "Add custom profile"),
            SplineEditorAction::DeleteCustomProfile(i) => write!(f, "Delete custom profile {}", i),
            SplineEditorAction::RenameProfile(i, n) => write!(f, "Rename profile {}: {}", i, n),
            SplineEditorAction::AddProfilePoint(i, _, _) => write!(f, "Add point to profile {}", i),
            SplineEditorAction::RemoveProfilePoint(pi, pti) => {
                write!(f, "Remove point {} from profile {}", pti, pi)
            }
            SplineEditorAction::ToggleSnapToGrid(b) => write!(f, "Toggle snap to grid: {}", b),
            SplineEditorAction::SetGridSize(s) => write!(f, "Set grid size: {:.2}", s),
            SplineEditorAction::ToggleAutoSmooth(b) => write!(f, "Toggle auto smooth: {}", b),
            SplineEditorAction::TogglePreviewMesh(b) => write!(f, "Toggle preview mesh: {}", b),
            SplineEditorAction::SelectAllPoints(id) => {
                write!(f, "Select all points in spline {}", id)
            }
            SplineEditorAction::ReverseSpline(id) => write!(f, "Reverse spline {}", id),
            SplineEditorAction::SimplifySpline(id, t) => {
                write!(f, "Simplify spline {} (tolerance: {:.2})", id, t)
            }
            SplineEditorAction::SubdivideSpline(id) => write!(f, "Subdivide spline {}", id),
            SplineEditorAction::SmoothSpline(id) => write!(f, "Smooth spline {}", id),
            SplineEditorAction::ExportSpline(id) => write!(f, "Export spline {}", id),
            SplineEditorAction::ImportSpline => write!(f, "Import spline"),
            SplineEditorAction::ExportAllSplines => write!(f, "Export all splines"),
        }
    }
}

/// Main Spline Editor Panel
pub struct SplineEditorPanel {
    active_tab: SplineTab,

    // Tools
    current_tool: SplineTool,

    // Splines
    splines: Vec<Spline>,
    selected_spline: Option<u32>,
    current_spline: Spline,

    // Points
    selected_point: Option<u32>,
    current_point: SplinePoint,

    // Profiles
    custom_profiles: Vec<CustomProfile>,

    // Editor settings
    snap_to_grid: bool,
    grid_size: f32,
    auto_smooth: bool,
    preview_mesh: bool,

    // ID counters
    next_spline_id: u32,
    next_point_id: u32,

    // Actions
    pending_actions: Vec<SplineEditorAction>,
}

impl Default for SplineEditorPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: SplineTab::Splines,

            current_tool: SplineTool::Select,

            splines: Vec::new(),
            selected_spline: None,
            current_spline: Spline::default(),

            selected_point: None,
            current_point: SplinePoint::default(),

            custom_profiles: Vec::new(),

            snap_to_grid: true,
            grid_size: 0.5,
            auto_smooth: true,
            preview_mesh: true,

            next_spline_id: 1,
            next_point_id: 1,

            pending_actions: Vec::new(),
        };

        panel
    }
}

impl SplineEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Take all pending actions, leaving the internal queue empty
    pub fn take_actions(&mut self) -> Vec<SplineEditorAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for later processing
    pub fn queue_action(&mut self, action: SplineEditorAction) {
        self.pending_actions.push(action);
    }

    /// Get reference to pending actions without removing them
    pub fn pending_actions(&self) -> &[SplineEditorAction] {
        &self.pending_actions
    }

    fn create_sample_data(&mut self) {
        // Create a sample road spline
        let spline_id = self.next_spline_id();
        let mut road_spline = Spline {
            id: spline_id,
            name: "Main Road".to_string(),
            preset: SplinePreset::Road,
            color: [0.3, 0.3, 0.35],
            generate_mesh: true,
            mesh_profile: MeshProfile::RoadWithSidewalk,
            deform_terrain: true,
            terrain_width: 8.0,
            ..Default::default()
        };

        // Add sample points
        let positions = [
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 5.0],
            [20.0, 0.0, 5.0],
            [30.0, 0.0, 0.0],
            [40.0, 0.0, -5.0],
        ];

        for pos in positions {
            let point_id = self.next_point_id();
            road_spline.points.push(SplinePoint {
                id: point_id,
                position: pos,
                width: 4.0,
                ..Default::default()
            });
        }

        road_spline.total_length = 45.0; // Approximate
        self.splines.push(road_spline.clone());
        self.current_spline = road_spline;
        self.selected_spline = Some(spline_id);

        // Create a camera rail
        let spline_id = self.next_spline_id();
        let mut camera_spline = Spline {
            id: spline_id,
            name: "Cinematic Camera".to_string(),
            preset: SplinePreset::CameraRail,
            spline_type: SplineType::Bezier,
            color: [1.0, 0.3, 0.3],
            animation_duration: 15.0,
            ..Default::default()
        };

        let camera_positions = [[0.0, 5.0, 0.0], [10.0, 8.0, 10.0], [20.0, 5.0, 20.0]];

        for pos in camera_positions {
            let point_id = self.next_point_id();
            camera_spline.points.push(SplinePoint {
                id: point_id,
                position: pos,
                ..Default::default()
            });
        }

        camera_spline.total_length = 30.0;
        self.splines.push(camera_spline);

        // Create a river
        let spline_id = self.next_spline_id();
        let mut river_spline = Spline {
            id: spline_id,
            name: "River".to_string(),
            preset: SplinePreset::River,
            color: [0.2, 0.5, 0.8],
            generate_mesh: true,
            mesh_profile: MeshProfile::RiverBed,
            deform_terrain: true,
            terrain_width: 12.0,
            terrain_height_offset: -1.5,
            ..Default::default()
        };

        let river_positions = [
            [-20.0, -1.0, 0.0],
            [-10.0, -1.0, 8.0],
            [0.0, -1.0, 12.0],
            [15.0, -1.0, 10.0],
            [30.0, -1.0, 15.0],
        ];

        for pos in river_positions {
            let point_id = self.next_point_id();
            river_spline.points.push(SplinePoint {
                id: point_id,
                position: pos,
                width: 6.0,
                ..Default::default()
            });
        }

        river_spline.total_length = 55.0;
        self.splines.push(river_spline);

        // Set selected point if points exist
        if !self.current_spline.points.is_empty() {
            self.current_point = self.current_spline.points[0].clone();
            self.selected_point = Some(self.current_spline.points[0].id);
        }

        // Add default profiles
        self.custom_profiles.push(CustomProfile::default());
    }

    fn next_spline_id(&mut self) -> u32 {
        let id = self.next_spline_id;
        self.next_spline_id += 1;
        id
    }

    fn next_point_id(&mut self) -> u32 {
        let id = self.next_point_id;
        self.next_point_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (SplineTab::Splines, "[Sq] Splines"),
                (SplineTab::Points, "Points"),
                (SplineTab::Mesh, "[D] Mesh"),
                (SplineTab::Terrain, "🏔️ Terrain"),
                (SplineTab::Animation, "Animation"),
                (SplineTab::Profiles, "Profiles"),
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

        ui.horizontal(|ui| {
            ui.label(format!("[Sq] {} splines", self.splines.len()));
            ui.separator();
            ui.label(format!("{} points", self.current_spline.points.len()));
            ui.separator();
            ui.label(format!("📏 {:.1}m", self.current_spline.total_length));
        });

        ui.separator();
    }

    fn show_splines_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Sq] Splines");
        ui.add_space(10.0);

        // Tool bar
        ui.group(|ui| {
            ui.label(RichText::new("Tools").strong());
            ui.horizontal(|ui| {
                let tools = [
                    SplineTool::Select,
                    SplineTool::AddPoint,
                    SplineTool::InsertPoint,
                    SplineTool::DeletePoint,
                    SplineTool::MoveTangent,
                    SplineTool::Draw,
                ];

                for tool in tools {
                    let is_selected = self.current_tool == tool;
                    let btn = egui::Button::new(format!("{} {:?}", tool.icon(), tool)).fill(
                        if is_selected {
                            Color32::from_rgb(80, 120, 180)
                        } else {
                            Color32::from_rgb(50, 50, 55)
                        },
                    );

                    if ui.add(btn).clicked() {
                        self.current_tool = tool;
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Spline list
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Spline List").strong());
                if ui.button("+ New").clicked() {
                    let id = self.next_spline_id();
                    info!(spline_id = id, "Spline: created new spline");
                    let new_spline = Spline {
                        id,
                        name: format!("Spline {}", id),
                        ..Default::default()
                    };
                    self.splines.push(new_spline.clone());
                    self.current_spline = new_spline;
                    self.selected_spline = Some(id);
                }
            });

            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    for spline in &self.splines.clone() {
                        let is_selected = self.selected_spline == Some(spline.id);
                        let label = format!(
                            "{} {} ({} pts)",
                            spline.preset.icon(),
                            spline.name,
                            spline.points.len()
                        );

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selected_spline = Some(spline.id);
                            self.current_spline = spline.clone();
                            if !spline.points.is_empty() {
                                self.current_point = spline.points[0].clone();
                                self.selected_point = Some(spline.points[0].id);
                            }
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // Current spline properties
        ui.group(|ui| {
            ui.label(RichText::new("Properties").strong());

            egui::Grid::new("spline_props")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.current_spline.name);
                    ui.end_row();

                    ui.label("Type:");
                    let prev_type = self.current_spline.spline_type;
                    egui::ComboBox::from_id_salt("spline_type")
                        .selected_text(format!("{:?}", self.current_spline.spline_type))
                        .show_ui(ui, |ui| {
                            for st in SplineType::all() {
                                ui.selectable_value(
                                    &mut self.current_spline.spline_type,
                                    *st,
                                    format!("{:?}", st),
                                );
                            }
                        });
                    if self.current_spline.spline_type != prev_type {
                        info!(old = ?prev_type, new = ?self.current_spline.spline_type, "Spline: type changed");
                    }
                    ui.end_row();

                    ui.label("Preset:");
                    egui::ComboBox::from_id_salt("spline_preset")
                        .selected_text(format!(
                            "{} {:?}",
                            self.current_spline.preset.icon(),
                            self.current_spline.preset
                        ))
                        .show_ui(ui, |ui| {
                            for preset in SplinePreset::all() {
                                ui.selectable_value(
                                    &mut self.current_spline.preset,
                                    *preset,
                                    format!("{} {:?}", preset.icon(), preset),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label("Closed:");
                    ui.checkbox(&mut self.current_spline.closed, "");
                    ui.end_row();

                    ui.label("Color:");
                    let mut color = Color32::from_rgb(
                        (self.current_spline.color[0] * 255.0) as u8,
                        (self.current_spline.color[1] * 255.0) as u8,
                        (self.current_spline.color[2] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        self.current_spline.color = [
                            color.r() as f32 / 255.0,
                            color.g() as f32 / 255.0,
                            color.b() as f32 / 255.0,
                        ];
                    }
                    ui.end_row();
                });
        });
    }

    fn show_points_tab(&mut self, ui: &mut Ui) {
        ui.heading("Control Points");
        ui.add_space(10.0);

        // Point list
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Points").strong());
                if ui.button("+ Add").clicked() {
                    let id = self.next_point_id();
                    info!(point_id = id, "Spline: added control point");
                    let last_pos = self
                        .current_spline
                        .points
                        .last()
                        .map(|p| p.position)
                        .unwrap_or([0.0, 0.0, 0.0]);

                    let new_point = SplinePoint {
                        id,
                        position: [last_pos[0] + 5.0, last_pos[1], last_pos[2]],
                        ..Default::default()
                    };
                    self.current_spline.points.push(new_point.clone());
                    self.current_point = new_point;
                    self.selected_point = Some(id);
                }
            });

            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    for (i, point) in self.current_spline.points.iter().enumerate() {
                        let is_selected = self.selected_point == Some(point.id);
                        let label = format!(
                            "Point {} ({:.1}, {:.1}, {:.1})",
                            i + 1,
                            point.position[0],
                            point.position[1],
                            point.position[2]
                        );

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selected_point = Some(point.id);
                            self.current_point = point.clone();
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // Point properties
        ui.group(|ui| {
            ui.label(RichText::new("[Sq] Point Properties").strong());

            egui::Grid::new("point_props")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Position:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.current_point.position[0])
                                .speed(0.1)
                                .prefix("X:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.current_point.position[1])
                                .speed(0.1)
                                .prefix("Y:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.current_point.position[2])
                                .speed(0.1)
                                .prefix("Z:"),
                        );
                    });
                    ui.end_row();

                    ui.label("Rotation:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.current_point.rotation[0])
                                .speed(1.0)
                                .suffix("°"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.current_point.rotation[1])
                                .speed(1.0)
                                .suffix("°"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.current_point.rotation[2])
                                .speed(1.0)
                                .suffix("°"),
                        );
                    });
                    ui.end_row();

                    ui.label("Scale:");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.current_point.scale[0]).speed(0.01));
                        ui.add(egui::DragValue::new(&mut self.current_point.scale[1]).speed(0.01));
                        ui.add(egui::DragValue::new(&mut self.current_point.scale[2]).speed(0.01));
                    });
                    ui.end_row();

                    ui.label("Roll:");
                    ui.add(
                        egui::Slider::new(&mut self.current_point.roll, -180.0..=180.0).suffix("°"),
                    );
                    ui.end_row();

                    ui.label("Width:");
                    ui.add(
                        egui::DragValue::new(&mut self.current_point.width)
                            .speed(0.1)
                            .suffix("m"),
                    );
                    ui.end_row();

                    ui.label("Tangent Mode:");
                    egui::ComboBox::from_id_salt("tangent_mode")
                        .selected_text(format!("{:?}", self.current_point.tangent_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.current_point.tangent_mode,
                                TangentMode::Auto,
                                "Auto",
                            );
                            ui.selectable_value(
                                &mut self.current_point.tangent_mode,
                                TangentMode::Smooth,
                                "Smooth",
                            );
                            ui.selectable_value(
                                &mut self.current_point.tangent_mode,
                                TangentMode::Linear,
                                "Linear",
                            );
                            ui.selectable_value(
                                &mut self.current_point.tangent_mode,
                                TangentMode::Free,
                                "Free",
                            );
                            ui.selectable_value(
                                &mut self.current_point.tangent_mode,
                                TangentMode::Aligned,
                                "Aligned",
                            );
                        });
                    ui.end_row();
                });
        });

        // Tangent controls (only for Bezier)
        if self.current_spline.spline_type == SplineType::Bezier {
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(RichText::new("↗️ Tangents").strong());

                egui::Grid::new("tangent_props")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("In Tangent:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.current_point.in_tangent[0])
                                    .speed(0.1),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.current_point.in_tangent[1])
                                    .speed(0.1),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.current_point.in_tangent[2])
                                    .speed(0.1),
                            );
                        });
                        ui.end_row();

                        ui.label("Out Tangent:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.current_point.out_tangent[0])
                                    .speed(0.1),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.current_point.out_tangent[1])
                                    .speed(0.1),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.current_point.out_tangent[2])
                                    .speed(0.1),
                            );
                        });
                        ui.end_row();
                    });
            });
        }
    }

    fn show_mesh_tab(&mut self, ui: &mut Ui) {
        ui.heading("[D] Mesh Generation");
        ui.add_space(10.0);

        ui.checkbox(
            &mut self.current_spline.generate_mesh,
            "Enable Mesh Generation",
        );

        if self.current_spline.generate_mesh {
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(RichText::new("Profile").strong());

                egui::Grid::new("mesh_profile")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Profile:");
                        egui::ComboBox::from_id_salt("mesh_profile")
                            .selected_text(format!("{:?}", self.current_spline.mesh_profile))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.current_spline.mesh_profile,
                                    MeshProfile::Flat,
                                    "Flat",
                                );
                                ui.selectable_value(
                                    &mut self.current_spline.mesh_profile,
                                    MeshProfile::Round,
                                    "Round",
                                );
                                ui.selectable_value(
                                    &mut self.current_spline.mesh_profile,
                                    MeshProfile::Square,
                                    "Square",
                                );
                                ui.selectable_value(
                                    &mut self.current_spline.mesh_profile,
                                    MeshProfile::RoadWithSidewalk,
                                    "Road + Sidewalk",
                                );
                                ui.selectable_value(
                                    &mut self.current_spline.mesh_profile,
                                    MeshProfile::RiverBed,
                                    "River Bed",
                                );
                                ui.selectable_value(
                                    &mut self.current_spline.mesh_profile,
                                    MeshProfile::Custom,
                                    "Custom",
                                );
                            });
                        ui.end_row();

                        ui.label("UV Mode:");
                        egui::ComboBox::from_id_salt("uv_mode")
                            .selected_text(format!("{:?}", self.current_spline.uv_mode))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.current_spline.uv_mode,
                                    UvMode::Stretch,
                                    "Stretch",
                                );
                                ui.selectable_value(
                                    &mut self.current_spline.uv_mode,
                                    UvMode::Tile,
                                    "Tile",
                                );
                                ui.selectable_value(
                                    &mut self.current_spline.uv_mode,
                                    UvMode::TileWorld,
                                    "Tile World",
                                );
                            });
                        ui.end_row();

                        ui.label("Segment Length:");
                        ui.add(
                            egui::Slider::new(&mut self.current_spline.segment_length, 0.1..=10.0)
                                .suffix("m"),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Regenerate Mesh").clicked() {
                    // Trigger mesh regeneration
                }
                if ui.button("Export Mesh").clicked() {
                    // Export mesh to file
                }
            });
        }
    }

    fn show_terrain_tab(&mut self, ui: &mut Ui) {
        ui.heading("🏔️ Terrain Deformation");
        ui.add_space(10.0);

        ui.checkbox(
            &mut self.current_spline.deform_terrain,
            "Enable Terrain Deformation",
        );

        if self.current_spline.deform_terrain {
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(RichText::new("[Sq] Deformation Settings").strong());

                egui::Grid::new("terrain_settings")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Width:");
                        ui.add(
                            egui::Slider::new(&mut self.current_spline.terrain_width, 1.0..=50.0)
                                .suffix("m"),
                        );
                        ui.end_row();

                        ui.label("Falloff:");
                        ui.add(
                            egui::Slider::new(&mut self.current_spline.terrain_falloff, 0.0..=20.0)
                                .suffix("m"),
                        );
                        ui.end_row();

                        ui.label("Height Offset:");
                        ui.add(
                            egui::Slider::new(
                                &mut self.current_spline.terrain_height_offset,
                                -10.0..=10.0,
                            )
                            .suffix("m"),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Apply to Terrain").clicked() {
                    // Apply terrain deformation
                }
                if ui.button("Undo Deformation").clicked() {
                    // Undo terrain changes
                }
            });
        }
    }

    fn show_animation_tab(&mut self, ui: &mut Ui) {
        ui.heading("Animation Path");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("[Time] Timing").strong());

            egui::Grid::new("animation_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Duration:");
                    ui.add(
                        egui::DragValue::new(&mut self.current_spline.animation_duration)
                            .speed(0.1)
                            .suffix("s"),
                    );
                    ui.end_row();

                    ui.label("Loop:");
                    ui.checkbox(&mut self.current_spline.loop_animation, "");
                    ui.end_row();

                    ui.label("Length:");
                    ui.label(format!("{:.1} m", self.current_spline.total_length));
                    ui.end_row();

                    ui.label("Speed:");
                    let speed = if self.current_spline.animation_duration > 0.0 {
                        self.current_spline.total_length / self.current_spline.animation_duration
                    } else {
                        0.0
                    };
                    ui.label(format!("{:.1} m/s", speed));
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Preview controls
        ui.group(|ui| {
            ui.label(RichText::new("> Preview").strong());

            ui.horizontal(|ui| {
                ui.button("⏮️").clicked();
                ui.button(">").clicked();
                ui.button("||").clicked();
                ui.button("[]").clicked();
                ui.button(">|").clicked();
            });

            // Timeline
            let mut progress = 0.5_f32;
            ui.add(egui::Slider::new(&mut progress, 0.0..=1.0).show_value(false));
        });
    }

    fn show_profiles_tab(&mut self, ui: &mut Ui) {
        ui.heading("Custom Profiles");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("+ New Profile").clicked() {
                self.custom_profiles.push(CustomProfile {
                    name: format!("Profile {}", self.custom_profiles.len() + 1),
                    ..Default::default()
                });
            }
        });

        ui.add_space(10.0);

        if self.custom_profiles.is_empty() {
            ui.label("No custom profiles.");
        } else {
            for profile in &mut self.custom_profiles {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut profile.name);
                        ui.label(format!("({} points)", profile.points.len()));
                    });

                    // Simple profile visualization
                    let height = 60.0;
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), height),
                        egui::Sense::hover(),
                    );

                    let painter = ui.painter();
                    painter.rect_filled(rect, 2.0, Color32::from_rgb(40, 40, 45));

                    // Draw profile curve
                    if profile.points.len() >= 2 {
                        let center_y = rect.center().y;
                        let scale_x = rect.width() / 2.0;
                        let scale_y = height / 2.0 * 0.8;

                        for i in 0..profile.points.len() - 1 {
                            let p1 = &profile.points[i];
                            let p2 = &profile.points[i + 1];

                            let x1 = rect.center().x + p1.x * scale_x;
                            let y1 = center_y - p1.y * scale_y;
                            let x2 = rect.center().x + p2.x * scale_x;
                            let y2 = center_y - p2.y * scale_y;

                            painter.line_segment(
                                [egui::Pos2::new(x1, y1), egui::Pos2::new(x2, y2)],
                                egui::Stroke::new(2.0, Color32::from_rgb(100, 180, 255)),
                            );
                        }
                    }
                });
            }
        }
    }

    // Getters for testing
    pub fn spline_count(&self) -> usize {
        self.splines.len()
    }

    pub fn point_count(&self) -> usize {
        self.current_spline.points.len()
    }

    pub fn profile_count(&self) -> usize {
        self.custom_profiles.len()
    }

    pub fn add_spline(&mut self, name: &str, preset: SplinePreset) -> u32 {
        let id = self.next_spline_id();
        self.splines.push(Spline {
            id,
            name: name.to_string(),
            preset,
            ..Default::default()
        });
        id
    }

    pub fn add_point(&mut self, position: [f32; 3]) -> u32 {
        let id = self.next_point_id();
        info!(point_id = id, pos = ?position, "Spline: added point");
        self.current_spline.points.push(SplinePoint {
            id,
            position,
            ..Default::default()
        });
        id
    }
}

impl Panel for SplineEditorPanel {
    fn name(&self) -> &'static str {
        "Spline Editor"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            SplineTab::Splines => self.show_splines_tab(ui),
            SplineTab::Points => self.show_points_tab(ui),
            SplineTab::Mesh => self.show_mesh_tab(ui),
            SplineTab::Terrain => self.show_terrain_tab(ui),
            SplineTab::Animation => self.show_animation_tab(ui),
            SplineTab::Profiles => self.show_profiles_tab(ui),
        }
    }

    fn update(&mut self) {
        // Sync current point back to spline
        if let Some(point_id) = self.selected_point {
            if let Some(point) = self
                .current_spline
                .points
                .iter_mut()
                .find(|p| p.id == point_id)
            {
                *point = self.current_point.clone();
            }
        }

        // Sync current spline back to list
        if let Some(spline_id) = self.selected_spline {
            if let Some(spline) = self.splines.iter_mut().find(|s| s.id == spline_id) {
                *spline = self.current_spline.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // SPLINE TYPE TESTS
    // ============================================================

    #[test]
    fn test_spline_type_default() {
        let st = SplineType::default();
        assert_eq!(st, SplineType::CatmullRom);
    }

    #[test]
    fn test_spline_type_all() {
        let all = SplineType::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_spline_type_all_coverage() {
        let all = SplineType::all();
        assert!(all.contains(&SplineType::CatmullRom));
        assert!(all.contains(&SplineType::Bezier));
        assert!(all.contains(&SplineType::Linear));
        assert!(all.contains(&SplineType::Hermite));
        assert!(all.contains(&SplineType::BSpline));
    }

    // ============================================================
    // SPLINE PRESET TESTS
    // ============================================================

    #[test]
    fn test_spline_preset_default() {
        let sp = SplinePreset::default();
        assert_eq!(sp, SplinePreset::Custom);
    }

    #[test]
    fn test_spline_preset_all() {
        let all = SplinePreset::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_spline_preset_icons() {
        assert_eq!(SplinePreset::Custom.icon(), "[Sq]");
        assert_eq!(SplinePreset::Road.icon(), "🛣️");
        assert_eq!(SplinePreset::Rail.icon(), "🚃");
        assert_eq!(SplinePreset::River.icon(), "[Wave]");
        assert_eq!(SplinePreset::Fence.icon(), "[Brk]");
        assert_eq!(SplinePreset::Cable.icon(), "🔌");
        assert_eq!(SplinePreset::AnimationPath.icon(), "[Anim]");
        assert_eq!(SplinePreset::CameraRail.icon(), "🎥");
    }

    #[test]
    fn test_spline_preset_all_have_icons() {
        for preset in SplinePreset::all() {
            assert!(!preset.icon().is_empty());
        }
    }

    // ============================================================
    // TANGENT MODE TESTS
    // ============================================================

    #[test]
    fn test_tangent_mode_default() {
        let tm = TangentMode::default();
        assert_eq!(tm, TangentMode::Auto);
    }

    #[test]
    fn test_tangent_mode_all_variants() {
        let variants = [
            TangentMode::Auto,
            TangentMode::Smooth,
            TangentMode::Linear,
            TangentMode::Free,
            TangentMode::Aligned,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // MESH PROFILE TESTS
    // ============================================================

    #[test]
    fn test_mesh_profile_default() {
        let mp = MeshProfile::default();
        assert_eq!(mp, MeshProfile::Flat);
    }

    #[test]
    fn test_mesh_profile_all_variants() {
        let variants = [
            MeshProfile::Flat,
            MeshProfile::Round,
            MeshProfile::Square,
            MeshProfile::RoadWithSidewalk,
            MeshProfile::RiverBed,
            MeshProfile::Custom,
        ];
        assert_eq!(variants.len(), 6);
    }

    // ============================================================
    // UV MODE TESTS
    // ============================================================

    #[test]
    fn test_uv_mode_default() {
        let uv = UvMode::default();
        assert_eq!(uv, UvMode::Stretch);
    }

    #[test]
    fn test_uv_mode_all_variants() {
        let variants = [UvMode::Stretch, UvMode::Tile, UvMode::TileWorld];
        assert_eq!(variants.len(), 3);
    }

    // ============================================================
    // SPLINE TOOL TESTS
    // ============================================================

    #[test]
    fn test_spline_tool_default() {
        let tool = SplineTool::default();
        assert_eq!(tool, SplineTool::Select);
    }

    #[test]
    fn test_spline_tool_icons() {
        assert_eq!(SplineTool::Select.icon(), "👆");
        assert_eq!(SplineTool::AddPoint.icon(), "+");
        assert_eq!(SplineTool::InsertPoint.icon(), "📍");
        assert_eq!(SplineTool::DeletePoint.icon(), "[x]");
        assert_eq!(SplineTool::MoveTangent.icon(), "↗️");
        assert_eq!(SplineTool::Draw.icon(), "✏️");
    }

    #[test]
    fn test_spline_tool_all_have_icons() {
        let tools = [
            SplineTool::Select,
            SplineTool::AddPoint,
            SplineTool::InsertPoint,
            SplineTool::DeletePoint,
            SplineTool::MoveTangent,
            SplineTool::Draw,
        ];
        for tool in tools {
            assert!(!tool.icon().is_empty());
        }
    }

    // ============================================================
    // SPLINE TAB TESTS
    // ============================================================

    #[test]
    fn test_spline_tab_default() {
        let tab = SplineTab::default();
        assert_eq!(tab, SplineTab::Splines);
    }

    #[test]
    fn test_spline_tab_all_variants() {
        let variants = [
            SplineTab::Splines,
            SplineTab::Points,
            SplineTab::Mesh,
            SplineTab::Terrain,
            SplineTab::Animation,
            SplineTab::Profiles,
        ];
        assert_eq!(variants.len(), 6);
    }

    // ============================================================
    // SPLINE POINT TESTS
    // ============================================================

    #[test]
    fn test_spline_point_default() {
        let sp = SplinePoint::default();
        assert_eq!(sp.id, 0);
        assert_eq!(sp.position, [0.0, 0.0, 0.0]);
        assert_eq!(sp.tangent_mode, TangentMode::Auto);
    }

    #[test]
    fn test_spline_point_default_scale() {
        let sp = SplinePoint::default();
        assert_eq!(sp.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_spline_point_default_tangents() {
        let sp = SplinePoint::default();
        assert_eq!(sp.in_tangent, [-1.0, 0.0, 0.0]);
        assert_eq!(sp.out_tangent, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_spline_point_default_width() {
        let sp = SplinePoint::default();
        assert!((sp.width - 1.0).abs() < 0.001);
        assert!((sp.roll - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_spline_point_clone() {
        let sp = SplinePoint::default();
        let cloned = sp.clone();
        assert_eq!(cloned.position, [0.0, 0.0, 0.0]);
    }

    // ============================================================
    // SPLINE TESTS
    // ============================================================

    #[test]
    fn test_spline_default() {
        let s = Spline::default();
        assert_eq!(s.id, 0);
        assert_eq!(s.name, "New Spline");
        assert_eq!(s.spline_type, SplineType::CatmullRom);
        assert_eq!(s.preset, SplinePreset::Custom);
        assert!(!s.closed);
    }

    #[test]
    fn test_spline_default_visual() {
        let s = Spline::default();
        assert!(s.show_points);
        assert!(s.show_tangents);
        assert!((s.thickness - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_spline_default_mesh() {
        let s = Spline::default();
        assert!(!s.generate_mesh);
        assert_eq!(s.mesh_profile, MeshProfile::Flat);
        assert_eq!(s.uv_mode, UvMode::Tile);
    }

    #[test]
    fn test_spline_default_terrain() {
        let s = Spline::default();
        assert!(!s.deform_terrain);
        assert!((s.terrain_width - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_spline_default_animation() {
        let s = Spline::default();
        assert!((s.animation_duration - 10.0).abs() < 0.001);
        assert!(s.loop_animation);
    }

    #[test]
    fn test_spline_clone() {
        let s = Spline::default();
        let cloned = s.clone();
        assert_eq!(cloned.name, "New Spline");
    }

    // ============================================================
    // CUSTOM PROFILE TESTS
    // ============================================================

    #[test]
    fn test_custom_profile_default() {
        let cp = CustomProfile::default();
        assert_eq!(cp.name, "Custom");
        assert_eq!(cp.points.len(), 2);
    }

    #[test]
    fn test_custom_profile_default_points() {
        let cp = CustomProfile::default();
        assert!((cp.points[0].x - (-1.0)).abs() < 0.001);
        assert!((cp.points[1].x - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_custom_profile_clone() {
        let cp = CustomProfile::default();
        let cloned = cp.clone();
        assert_eq!(cloned.name, "Custom");
    }

    // ============================================================
    // SPLINE EDITOR PANEL TESTS
    // ============================================================

    #[test]
    fn test_spline_editor_creation() {
        let mut panel = SplineEditorPanel::new();
        panel.create_sample_data();
        assert!(panel.spline_count() >= 3);
    }

    #[test]
    fn test_default_points() {
        let mut panel = SplineEditorPanel::new();
        panel.create_sample_data();
        assert!(panel.point_count() >= 5);
    }

    #[test]
    fn test_add_spline() {
        let mut panel = SplineEditorPanel::new();
        let initial = panel.spline_count();
        let id = panel.add_spline("Test Road", SplinePreset::Road);
        assert!(id > 0);
        assert_eq!(panel.spline_count(), initial + 1);
    }

    #[test]
    fn test_add_multiple_splines() {
        let mut panel = SplineEditorPanel::new();
        let initial = panel.spline_count();
        panel.add_spline("Road A", SplinePreset::Road);
        panel.add_spline("River B", SplinePreset::River);
        panel.add_spline("Rail C", SplinePreset::Rail);
        assert_eq!(panel.spline_count(), initial + 3);
    }

    #[test]
    fn test_add_point() {
        let mut panel = SplineEditorPanel::new();
        let initial = panel.point_count();
        let id = panel.add_point([10.0, 0.0, 10.0]);
        assert!(id > 0);
        assert_eq!(panel.point_count(), initial + 1);
    }

    #[test]
    fn test_add_multiple_points() {
        let mut panel = SplineEditorPanel::new();
        let initial = panel.point_count();
        panel.add_point([0.0, 0.0, 0.0]);
        panel.add_point([5.0, 0.0, 5.0]);
        panel.add_point([10.0, 0.0, 10.0]);
        assert_eq!(panel.point_count(), initial + 3);
    }

    #[test]
    fn test_custom_profiles() {
        let mut panel = SplineEditorPanel::new();
        panel.create_sample_data();
        assert!(panel.profile_count() >= 1);
    }

    #[test]
    fn test_panel_trait() {
        let panel = SplineEditorPanel::new();
        assert_eq!(panel.name(), "Spline Editor");
    }

    // ============================================================
    // INTEGRATION TESTS
    // ============================================================

    #[test]
    fn test_all_presets_coverage() {
        let all = SplinePreset::all();
        assert!(all.contains(&SplinePreset::Custom));
        assert!(all.contains(&SplinePreset::Road));
        assert!(all.contains(&SplinePreset::Rail));
        assert!(all.contains(&SplinePreset::River));
        assert!(all.contains(&SplinePreset::Fence));
        assert!(all.contains(&SplinePreset::Cable));
        assert!(all.contains(&SplinePreset::AnimationPath));
        assert!(all.contains(&SplinePreset::CameraRail));
    }

    #[test]
    fn test_spline_with_points() {
        let mut spline = Spline::default();
        spline.points.push(SplinePoint::default());
        spline.points.push(SplinePoint::default());
        assert_eq!(spline.points.len(), 2);
    }

    #[test]
    fn test_profile_point_values() {
        let pp = ProfilePoint {
            x: 0.5,
            y: 1.0,
            uv: 0.5,
        };
        assert!((pp.x - 0.5).abs() < 0.001);
        assert!((pp.y - 1.0).abs() < 0.001);
        assert!((pp.uv - 0.5).abs() < 0.001);
    }

    // ============================================================
    // DISPLAY TRAIT TESTS
    // ============================================================

    #[test]
    fn test_spline_type_display() {
        for spline_type in SplineType::all() {
            let display = format!("{}", spline_type);
            assert!(display.contains(spline_type.name()));
        }
    }

    #[test]
    fn test_spline_type_all_count() {
        let all = SplineType::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_spline_type_is_smooth() {
        assert!(SplineType::CatmullRom.is_smooth());
        assert!(SplineType::Bezier.is_smooth());
        assert!(!SplineType::Linear.is_smooth());
    }

    #[test]
    fn test_spline_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for spline_type in SplineType::all() {
            set.insert(*spline_type);
        }
        assert_eq!(set.len(), SplineType::all().len());
    }

    #[test]
    fn test_spline_preset_display() {
        for preset in SplinePreset::all() {
            let display = format!("{}", preset);
            assert!(display.contains(preset.name()));
        }
    }

    #[test]
    fn test_spline_preset_all_count() {
        let all = SplinePreset::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_spline_preset_is_infrastructure() {
        assert!(SplinePreset::Road.is_infrastructure());
        assert!(SplinePreset::Rail.is_infrastructure());
        assert!(SplinePreset::Fence.is_infrastructure());
        assert!(SplinePreset::Cable.is_infrastructure());
        assert!(!SplinePreset::River.is_infrastructure());
        assert!(!SplinePreset::AnimationPath.is_infrastructure());
    }

    #[test]
    fn test_spline_preset_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for preset in SplinePreset::all() {
            set.insert(*preset);
        }
        assert_eq!(set.len(), SplinePreset::all().len());
    }

    #[test]
    fn test_tangent_mode_display() {
        for mode in TangentMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_tangent_mode_all() {
        let all = TangentMode::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_tangent_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in TangentMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), TangentMode::all().len());
    }

    #[test]
    fn test_mesh_profile_display() {
        for profile in MeshProfile::all() {
            let display = format!("{}", profile);
            assert!(display.contains(profile.name()));
        }
    }

    #[test]
    fn test_mesh_profile_all() {
        let all = MeshProfile::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_mesh_profile_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for profile in MeshProfile::all() {
            set.insert(*profile);
        }
        assert_eq!(set.len(), MeshProfile::all().len());
    }

    #[test]
    fn test_uv_mode_display() {
        for mode in UvMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_uv_mode_all() {
        let all = UvMode::all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_uv_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in UvMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), UvMode::all().len());
    }

    #[test]
    fn test_spline_tool_display() {
        for tool in SplineTool::all() {
            let display = format!("{}", tool);
            assert!(display.contains(tool.name()));
        }
    }

    #[test]
    fn test_spline_tool_all() {
        let all = SplineTool::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_spline_tool_is_destructive() {
        assert!(SplineTool::DeletePoint.is_destructive());
        assert!(!SplineTool::Select.is_destructive());
        assert!(!SplineTool::AddPoint.is_destructive());
    }

    #[test]
    fn test_spline_tool_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tool in SplineTool::all() {
            set.insert(*tool);
        }
        assert_eq!(set.len(), SplineTool::all().len());
    }

    #[test]
    fn test_spline_tab_display() {
        for tab in SplineTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()));
        }
    }

    #[test]
    fn test_spline_tab_all() {
        let all = SplineTab::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_spline_tab_icon() {
        assert_eq!(SplineTab::Splines.icon(), "〰️");
        assert_eq!(SplineTab::Points.icon(), "📍");
        assert_eq!(SplineTab::Animation.icon(), "[Anim]");
    }

    #[test]
    fn test_spline_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in SplineTab::all() {
            set.insert(*tab);
        }
        assert_eq!(set.len(), SplineTab::all().len());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // SplineEditorAction Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_spline_editor_action_display() {
        let action = SplineEditorAction::SetActiveTab(SplineTab::Splines);
        let display = format!("{}", action);
        assert!(display.contains("Splines") || display.contains("tab"));
    }

    #[test]
    fn test_spline_editor_action_display_all_variants() {
        let actions = vec![
            SplineEditorAction::SetActiveTab(SplineTab::Points),
            SplineEditorAction::SetTool(SplineTool::Select),
            SplineEditorAction::CreateSpline,
            SplineEditorAction::DeleteSpline(0),
            SplineEditorAction::SelectSpline(0),
            SplineEditorAction::AddPoint(0, [0.0, 0.0, 0.0]),
            SplineEditorAction::DeletePoint(0, 0),
            SplineEditorAction::ToggleClosed(0, true),
            SplineEditorAction::ReverseSpline(0),
            SplineEditorAction::SimplifySpline(0, 0.1),
        ];

        for action in actions {
            let display = format!("{}", action);
            assert!(
                !display.is_empty(),
                "Display should not be empty for {:?}",
                action
            );
        }
    }

    #[test]
    fn test_spline_editor_action_equality() {
        let action1 = SplineEditorAction::SelectSpline(5);
        let action2 = SplineEditorAction::SelectSpline(5);
        let action3 = SplineEditorAction::SelectSpline(10);

        assert_eq!(action1, action2);
        assert_ne!(action1, action3);
    }

    #[test]
    fn test_spline_editor_action_clone() {
        let action = SplineEditorAction::SetTool(SplineTool::Draw);
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_spline_editor_panel_pending_actions_empty_by_default() {
        let panel = SplineEditorPanel::new();
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_spline_editor_panel_queue_action() {
        let mut panel = SplineEditorPanel::new();
        panel.queue_action(SplineEditorAction::CreateSpline);
        assert!(panel.has_pending_actions());
        assert_eq!(panel.pending_actions().len(), 1);
    }

    #[test]
    fn test_spline_editor_panel_take_actions() {
        let mut panel = SplineEditorPanel::new();
        panel.queue_action(SplineEditorAction::SelectSpline(0));
        panel.queue_action(SplineEditorAction::AddPoint(0, [1.0, 2.0, 3.0]));

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_spline_editor_panel_action_order_preserved() {
        let mut panel = SplineEditorPanel::new();
        panel.queue_action(SplineEditorAction::CreateSpline);
        panel.queue_action(SplineEditorAction::AddPoint(0, [0.0, 0.0, 0.0]));
        panel.queue_action(SplineEditorAction::AddPoint(0, [10.0, 0.0, 0.0]));

        let actions = panel.take_actions();
        assert!(matches!(actions[0], SplineEditorAction::CreateSpline));
        assert!(matches!(actions[1], SplineEditorAction::AddPoint(..)));
        assert!(matches!(actions[2], SplineEditorAction::AddPoint(..)));
    }

    #[test]
    fn test_spline_editor_action_spline_operations() {
        let actions = vec![
            SplineEditorAction::CreateSpline,
            SplineEditorAction::DeleteSpline(0),
            SplineEditorAction::DuplicateSpline(1),
            SplineEditorAction::SelectSpline(2),
            SplineEditorAction::DeselectSpline,
            SplineEditorAction::RenameSpline(0, "Path".to_string()),
            SplineEditorAction::SetSplineColor(0, [1.0, 0.0, 0.0]),
            SplineEditorAction::SetSplineType(0, SplineType::Bezier),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_spline_editor_action_point_operations() {
        let actions = vec![
            SplineEditorAction::AddPoint(0, [1.0, 2.0, 3.0]),
            SplineEditorAction::DeletePoint(0, 0),
            SplineEditorAction::SelectPoint(1),
            SplineEditorAction::DeselectPoint,
            SplineEditorAction::SetPointPosition(0, 0, [5.0, 5.0, 5.0]),
            SplineEditorAction::SetPointTangentMode(0, 0, TangentMode::Auto),
            SplineEditorAction::SetPointInTangent(0, 0, [1.0, 0.0, 0.0]),
            SplineEditorAction::SetPointOutTangent(0, 0, [-1.0, 0.0, 0.0]),
            SplineEditorAction::InsertPoint(0, 1, [2.0, 0.0, 0.0]),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Add") || displays[0].contains("point"));
        assert!(displays[1].contains("Delete") || displays[1].contains("point"));
    }

    #[test]
    fn test_spline_editor_action_mesh_generation() {
        let actions = vec![
            SplineEditorAction::ToggleGenerateMesh(0, true),
            SplineEditorAction::SetMeshProfile(0, MeshProfile::Flat),
            SplineEditorAction::SetSegmentLength(0, 1.0),
            SplineEditorAction::SetUvMode(0, UvMode::Stretch),
            SplineEditorAction::RebuildMesh(0),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_spline_editor_action_terrain_deformation() {
        let actions = vec![
            SplineEditorAction::ToggleDeformTerrain(0, true),
            SplineEditorAction::SetTerrainWidth(0, 10.0),
            SplineEditorAction::SetTerrainFalloff(0, 5.0),
            SplineEditorAction::SetTerrainHeightOffset(0, -0.5),
            SplineEditorAction::ApplyTerrainDeformation(0),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_spline_editor_action_animation() {
        let actions = vec![
            SplineEditorAction::SetAnimationDuration(0, 5.0),
            SplineEditorAction::ToggleLoopAnimation(0, true),
            SplineEditorAction::PreviewAnimation(0),
            SplineEditorAction::StopAnimationPreview,
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_spline_editor_action_ui_toggles() {
        let actions = vec![
            SplineEditorAction::ToggleShowPoints(0, true),
            SplineEditorAction::ToggleShowTangents(0, true),
            SplineEditorAction::ToggleSnapToGrid(true),
            SplineEditorAction::ToggleAutoSmooth(true),
            SplineEditorAction::TogglePreviewMesh(true),
        ];

        for action in actions {
            let display = format!("{}", action);
            assert!(
                display.contains("true"),
                "Display should contain 'true': {}",
                display
            );
        }
    }

    #[test]
    fn test_spline_editor_action_spline_manipulation() {
        let actions = vec![
            SplineEditorAction::ToggleClosed(0, true),
            SplineEditorAction::ReverseSpline(0),
            SplineEditorAction::SimplifySpline(0, 0.5),
            SplineEditorAction::SubdivideSpline(0),
            SplineEditorAction::SmoothSpline(0),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[1].contains("Reverse") || displays[1].contains("spline"));
        assert!(displays[2].contains("Simplify") || displays[2].contains("spline"));
        assert!(displays[3].contains("Subdivide") || displays[3].contains("spline"));
        assert!(displays[4].contains("Smooth") || displays[4].contains("spline"));
    }

    #[test]
    fn test_spline_editor_action_custom_profiles() {
        let actions = vec![
            SplineEditorAction::AddCustomProfile,
            SplineEditorAction::DeleteCustomProfile(0),
            SplineEditorAction::RenameProfile(0, "River".to_string()),
            SplineEditorAction::AddProfilePoint(0, 0.5, 1.0),
            SplineEditorAction::RemoveProfilePoint(0, 1),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_spline_editor_action_import_export() {
        let actions = vec![
            SplineEditorAction::ExportSpline(0),
            SplineEditorAction::ImportSpline,
            SplineEditorAction::ExportAllSplines,
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }
}
