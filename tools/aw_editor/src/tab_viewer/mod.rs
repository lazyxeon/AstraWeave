//! Tab Viewer Implementation for egui_dock
//!
//! This module provides the `SimpleTabViewer` struct which implements
//! `egui_dock::TabViewer` to render each panel type in the docking system.
//!
//! # Architecture
//!
//! The `SimpleTabViewer` acts as a bridge between the docking system and
//! the actual panel implementations. It renders panel placeholders and
//! can be extended to render actual panel content.
//!
//! # Module Structure
//!
//! - `panel_event` — PanelEvent enum, Display impl, category helpers
//! - `models` — Data-only structs (EntityInfo, RuntimeStatsInfo, etc.)
//! - `inspectors` — Component inspector UI functions
//! - `sky_colors` — Pure computation for sky colors, lighting, fog params

mod inspectors;
mod models;
mod panel_event;
mod sky_colors;

// Re-export all public types so external code (lib.rs, tests) continues to work.
pub use models::{
    AnimationState, AnimationTrack, AssetEntry, BehaviorGraphState, BehaviorNode, BehaviorNodeType,
    EditorTheme, EntityInfo, GraphNode, Keyframe, MaterialInfo, RuntimeStatsInfo, SceneStatsInfo,
};
#[allow(unused_imports)] // PanelEventCallback is part of library API, unused in binary
pub use panel_event::{PanelEvent, PanelEventCallback};

use crate::panel_type::PanelType;
use egui_dock::tab_viewer::OnCloseResponse;
use egui_dock::TabViewer;
// Use egui types from egui_dock to ensure compatibility
use egui_dock::egui;

use crate::command::UndoStack;
use crate::entity_manager::EntityManager;
use crate::panels::entity_catalog::EntityCatalogState;
use crate::panels::{
    AssetBrowser, AudioPanel, BlendImportPanel, CinematicsPanel, DialogueEditorPanel, FoliagePanel,
    InputBindingsPanel, LightingPanel, LocalizationPanel, LodConfigPanel, MaterialEditorPanel,
    NavigationPanel, NetworkingPanel, ParticleSystemPanel, PcgPanel, PhysicsPanel,
    PostProcessPanel, ProjectSettingsPanel, SplineEditorPanel, TerrainPanel, UiEditorPanel,
};
use crate::prefab::PrefabManager;
use crate::viewport::ViewportWidget;
use astraweave_core::World;

/// Context wrapper for rendering tabs with access to editor resources.
///
/// This struct temporarily holds mutable references to editor components
/// during the dock rendering phase, allowing the Viewport panel to access
/// `ViewportWidget` while other panels use `EditorTabViewer`.
pub struct EditorDrawContext<'a> {
    /// The persistent tab viewer state
    pub tab_viewer: &'a mut EditorTabViewer,
    /// Optional viewport widget for 3D rendering
    pub viewport: Option<&'a mut ViewportWidget>,
    /// World for entity data
    pub world: Option<&'a mut World>,
    /// Entity manager for transforms
    pub entity_manager: Option<&'a mut EntityManager>,
    /// Undo stack for command history
    pub undo_stack: Option<&'a mut UndoStack>,
    /// Prefab manager for instantiation
    pub prefab_manager: Option<&'a mut PrefabManager>,
    /// Additional viewports for multi-viewport layouts
    pub extra_viewports: &'a mut Vec<ViewportWidget>,
    /// Current multi-viewport layout mode
    pub viewport_layout: crate::viewport::ViewportLayout,
}

impl<'a> EditorDrawContext<'a> {
    /// Create a new draw context with just the tab viewer (minimal)
    pub fn new(
        tab_viewer: &'a mut EditorTabViewer,
        extra_viewports: &'a mut Vec<ViewportWidget>,
    ) -> Self {
        Self {
            tab_viewer,
            viewport: None,
            world: None,
            entity_manager: None,
            undo_stack: None,
            prefab_manager: None,
            extra_viewports,
            viewport_layout: crate::viewport::ViewportLayout::Single,
        }
    }

    /// Set the viewport widget for rendering
    pub fn with_viewport(mut self, viewport: &'a mut ViewportWidget) -> Self {
        self.viewport = Some(viewport);
        self
    }

    /// Set the world for entity access
    pub fn with_world(mut self, world: &'a mut World) -> Self {
        self.world = Some(world);
        self
    }

    /// Set the entity manager
    pub fn with_entity_manager(mut self, entity_manager: &'a mut EntityManager) -> Self {
        self.entity_manager = Some(entity_manager);
        self
    }

    /// Set the undo stack
    pub fn with_undo_stack(mut self, undo_stack: &'a mut UndoStack) -> Self {
        self.undo_stack = Some(undo_stack);
        self
    }

    /// Set the prefab manager
    pub fn with_prefab_manager(mut self, prefab_manager: &'a mut PrefabManager) -> Self {
        self.prefab_manager = Some(prefab_manager);
        self
    }

    /// Set viewport layout mode
    pub fn with_viewport_layout(mut self, layout: crate::viewport::ViewportLayout) -> Self {
        self.viewport_layout = layout;
        self
    }
}

impl<'a> TabViewer for EditorDrawContext<'a> {
    type Tab = PanelType;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        // Delegate to EditorTabViewer
        self.tab_viewer.title(tab)
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            PanelType::Viewport => {
                // Multi-viewport rendering: split the available rect based on layout
                if let (Some(viewport), Some(world), Some(entity_manager), Some(undo_stack)) = (
                    self.viewport.as_mut(),
                    self.world.as_mut(),
                    self.entity_manager.as_mut(),
                    self.undo_stack.as_mut(),
                ) {
                    let prefab_mgr = self.prefab_manager.as_deref_mut();
                    let layout = self.viewport_layout;

                    use crate::viewport::ViewportLayout;
                    match layout {
                        ViewportLayout::Single => {
                            if let Err(e) = viewport.ui(
                                ui,
                                world,
                                entity_manager,
                                undo_stack,
                                prefab_mgr,
                                false,
                            ) {
                                ui.centered_and_justified(|ui| {
                                    ui.colored_label(
                                        egui::Color32::RED,
                                        format!("Viewport Error: {}", e),
                                    );
                                });
                            }
                        }
                        ViewportLayout::SideBySide => {
                            let columns = [ui
                                .available_rect_before_wrap()
                                .split_left_right_at_fraction(0.5)];
                            let (left_rect, right_rect) = columns[0];
                            // Primary viewport (left)
                            let mut left_ui =
                                ui.new_child(egui::UiBuilder::new().max_rect(left_rect));
                            if let Err(e) = viewport.ui(
                                &mut left_ui,
                                world,
                                entity_manager,
                                undo_stack,
                                prefab_mgr,
                                false,
                            ) {
                                left_ui.colored_label(
                                    egui::Color32::RED,
                                    format!("Viewport 1 Error: {}", e),
                                );
                            }
                            // Secondary viewport (right)
                            if let Some(vp2) = self.extra_viewports.get_mut(0) {
                                let mut right_ui =
                                    ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
                                if let Err(e) = vp2.ui(
                                    &mut right_ui,
                                    world,
                                    entity_manager,
                                    undo_stack,
                                    None,
                                    false,
                                ) {
                                    right_ui.colored_label(
                                        egui::Color32::RED,
                                        format!("Viewport 2 Error: {}", e),
                                    );
                                }
                            }
                        }
                        ViewportLayout::TopBottom => {
                            let avail = ui.available_rect_before_wrap();
                            let (top_rect, bottom_rect) = avail.split_top_bottom_at_fraction(0.5);
                            // Primary viewport (top)
                            let mut top_ui =
                                ui.new_child(egui::UiBuilder::new().max_rect(top_rect));
                            if let Err(e) = viewport.ui(
                                &mut top_ui,
                                world,
                                entity_manager,
                                undo_stack,
                                prefab_mgr,
                                false,
                            ) {
                                top_ui.colored_label(
                                    egui::Color32::RED,
                                    format!("Viewport 1 Error: {}", e),
                                );
                            }
                            // Secondary viewport (bottom)
                            if let Some(vp2) = self.extra_viewports.get_mut(0) {
                                let mut bottom_ui =
                                    ui.new_child(egui::UiBuilder::new().max_rect(bottom_rect));
                                if let Err(e) = vp2.ui(
                                    &mut bottom_ui,
                                    world,
                                    entity_manager,
                                    undo_stack,
                                    None,
                                    false,
                                ) {
                                    bottom_ui.colored_label(
                                        egui::Color32::RED,
                                        format!("Viewport 2 Error: {}", e),
                                    );
                                }
                            }
                        }
                        ViewportLayout::Quad => {
                            let avail = ui.available_rect_before_wrap();
                            let (top, bottom) = avail.split_top_bottom_at_fraction(0.5);
                            let (tl, tr) = top.split_left_right_at_fraction(0.5);
                            let (bl, br) = bottom.split_left_right_at_fraction(0.5);

                            // Viewport 1 (top-left) — primary
                            let mut ui_tl = ui.new_child(egui::UiBuilder::new().max_rect(tl));
                            if let Err(e) = viewport.ui(
                                &mut ui_tl,
                                world,
                                entity_manager,
                                undo_stack,
                                prefab_mgr,
                                false,
                            ) {
                                ui_tl.colored_label(egui::Color32::RED, format!("VP1: {}", e));
                            }

                            // Viewport 2 (top-right)
                            if let Some(vp2) = self.extra_viewports.get_mut(0) {
                                let mut ui_tr = ui.new_child(egui::UiBuilder::new().max_rect(tr));
                                if let Err(e) = vp2.ui(
                                    &mut ui_tr,
                                    world,
                                    entity_manager,
                                    undo_stack,
                                    None,
                                    false,
                                ) {
                                    ui_tr.colored_label(egui::Color32::RED, format!("VP2: {}", e));
                                }
                            }

                            // Viewport 3 (bottom-left)
                            if let Some(vp3) = self.extra_viewports.get_mut(1) {
                                let mut ui_bl = ui.new_child(egui::UiBuilder::new().max_rect(bl));
                                if let Err(e) = vp3.ui(
                                    &mut ui_bl,
                                    world,
                                    entity_manager,
                                    undo_stack,
                                    None,
                                    false,
                                ) {
                                    ui_bl.colored_label(egui::Color32::RED, format!("VP3: {}", e));
                                }
                            }

                            // Viewport 4 (bottom-right)
                            if let Some(vp4) = self.extra_viewports.get_mut(2) {
                                let mut ui_br = ui.new_child(egui::UiBuilder::new().max_rect(br));
                                if let Err(e) = vp4.ui(
                                    &mut ui_br,
                                    world,
                                    entity_manager,
                                    undo_stack,
                                    None,
                                    false,
                                ) {
                                    ui_br.colored_label(egui::Color32::RED, format!("VP4: {}", e));
                                }
                            }
                        }
                    }
                } else {
                    // Fallback to placeholder if resources unavailable
                    self.tab_viewer.ui(ui, tab);
                }
            }
            _ => {
                // Delegate all other panels to EditorTabViewer
                self.tab_viewer.ui(ui, tab);
            }
        }
    }

    fn is_closeable(&self, tab: &Self::Tab) -> bool {
        tab.is_closable()
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        // Viewport should not have scroll bars
        if matches!(tab, PanelType::Viewport) {
            [false, false]
        } else {
            self.tab_viewer.scroll_bars(tab)
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        self.tab_viewer.id(tab)
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        self.tab_viewer.on_close(tab)
    }

    fn add_popup(
        &mut self,
        ui: &mut egui::Ui,
        surface: egui_dock::SurfaceIndex,
        node: egui_dock::NodeIndex,
    ) {
        self.tab_viewer.add_popup(ui, surface, node)
    }
}

/// Get a simple HH:MM:SS timestamp string for build logs
fn chrono_lite_time() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Simplified tab viewer for basic docking functionality
///
/// This viewer renders panel placeholders and can be extended
/// to render actual panel content by providing panel references.
pub struct SimpleTabViewer {
    /// Currently selected entity ID (if any)
    pub selected_entity: Option<u64>,
    /// Whether the editor is in play mode
    pub is_playing: bool,
    /// Panels that were closed this frame (for handling)
    pub panels_to_close: Vec<PanelType>,
}

impl Default for SimpleTabViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleTabViewer {
    /// Create a new simple tab viewer
    pub fn new() -> Self {
        Self {
            selected_entity: None,
            is_playing: false,
            panels_to_close: Vec::new(),
        }
    }
}

impl TabViewer for SimpleTabViewer {
    type Tab = PanelType;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        format!("{} {}", tab.icon(), tab.title()).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // Render a placeholder for each panel type
        // In the full implementation, this dispatches to actual panel implementations
        match tab {
            PanelType::Viewport => {
                // Viewport needs special handling - show placeholder
                ui.centered_and_justified(|ui| {
                    ui.label("3D Viewport");
                    ui.label("(Rendering not yet integrated with docking)");
                });
            }
            PanelType::Inspector => {
                if let Some(entity_id) = self.selected_entity {
                    ui.heading(format!("Entity {}", entity_id));
                    ui.separator();
                    ui.label("Transform, components, and properties");
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No entity selected");
                    });
                }
            }
            _ => {
                // Generic panel placeholder
                ui.heading(tab.title());
                ui.separator();
                ui.label(format!("{} panel content", tab.title()));
                if self.is_playing {
                    ui.label("Play mode active");
                }
            }
        }
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        tab.is_closable()
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        if tab.has_scroll() {
            [true, true] // horizontal, vertical
        } else {
            [false, false]
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(format!("panel_{:?}", tab))
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        // Return whether closing is allowed
        if tab.is_closable() {
            self.panels_to_close.push(*tab);
            OnCloseResponse::Close
        } else {
            OnCloseResponse::Ignore
        }
    }

    fn add_popup(
        &mut self,
        ui: &mut egui::Ui,
        _surface: egui_dock::SurfaceIndex,
        _node: egui_dock::NodeIndex,
    ) {
        // Show popup menu for adding new panels
        ui.label("Add Panel:");
        ui.separator();

        for panel_type in PanelType::all() {
            if ui.button(format!("{}", panel_type)).clicked() {
                // The actual adding would be handled by the dock system
                ui.close();
            }
        }
    }
}

// PanelEvent enum, Display impl, category/entity_id helpers, and PanelEventCallback
// have been moved to `panel_event.rs`. Re-exported above.

// EntityInfo, RuntimeStatsInfo, SceneStatsInfo, AssetEntry, MaterialInfo, EditorTheme,
// AnimationState, AnimationTrack, Keyframe, GraphNode, BehaviorGraphState, BehaviorNode,
// BehaviorNodeType have been moved to `models.rs`. Re-exported above.

/// Full-featured editor tab viewer
///
/// This viewer can be connected to actual panel state for real rendering.
/// It supports event callbacks to notify the main editor of panel actions.
pub struct EditorTabViewer {
    /// Currently selected entity ID (if any)
    pub selected_entity: Option<u64>,
    /// Whether the editor is in play mode
    pub is_playing: bool,
    /// Panels that were closed this frame
    pub panels_to_close: Vec<PanelType>,
    /// Panels that should be added this frame
    pub panels_to_add: Vec<PanelType>,
    /// Events emitted this frame
    events: Vec<PanelEvent>,
    /// Cached entity list for hierarchy display
    entity_list: Vec<EntityInfo>,
    /// Number of terrain chunks currently generated (for hierarchy display)
    terrain_chunk_count: usize,
    /// Cached console logs
    console_logs: std::collections::VecDeque<String>,
    /// Cached console error count (updated when logs change)
    console_error_count: usize,
    /// Cached console warning count (updated when logs change)
    console_warn_count: usize,
    /// Selected entity transform data (position, rotation, scale) - MUTABLE for editing
    pub selected_transform: Option<(f32, f32, f32, f32, f32, f32, f32)>, // x, y, z, rotation, scale_x, scale_y, scale_z
    /// Previous transform for change detection
    previous_transform: Option<(f32, f32, f32, f32, f32, f32, f32)>,
    /// Selected entity info
    selected_entity_info: Option<EntityInfo>,
    /// Hierarchy search filter
    hierarchy_search: String,
    /// Console search filter
    console_search: String,
    /// Show error logs
    show_errors: bool,
    /// Show warning logs
    show_warnings: bool,
    /// Show info logs
    show_info: bool,
    /// Runtime statistics
    runtime_stats: RuntimeStatsInfo,
    /// Scene statistics
    scene_stats: SceneStatsInfo,
    /// Asset browser entries
    asset_entries: Vec<AssetEntry>,
    /// Current asset path
    asset_current_path: String,
    /// Undo stack size
    undo_count: usize,
    /// Redo stack size  
    redo_count: usize,
    /// Frame time history for graph
    frame_time_history: Vec<f32>,
    /// Current material being edited
    current_material: MaterialInfo,
    /// Current editor theme
    current_theme: EditorTheme,
    /// UI scale factor
    ui_scale: f32,
    /// Grid enabled
    grid_enabled: bool,
    /// Snap to grid
    snap_enabled: bool,
    /// Animation timeline state
    animation_state: AnimationState,
    /// Behavior graph state
    behavior_graph: BehaviorGraphState,
    /// Graph nodes for visual scripting
    graph_nodes: Vec<GraphNode>,
    /// Build target (0=Windows, 1=Linux, 2=macOS, 3=WebGL)
    build_target: usize,
    /// Build profile (0=Debug, 1=Release)
    build_profile: usize,
    /// Build options
    build_include_debug_symbols: bool,
    build_strip_unused: bool,
    build_compress_textures: bool,
    /// Texture quality index (0=Low, 1=Medium, 2=High, 3=Ultra)
    build_texture_quality: usize,
    /// Build output log
    build_output: Vec<String>,
    /// Build status (0=Idle, 1=Building, 2=Success, 3=Failed)
    build_status: usize,
    /// Build progress (0.0 to 1.0)
    build_progress: f32,
    /// Build start time for elapsed time calculation
    build_start_time: Option<std::time::Instant>,
    /// Viewport view mode (0=Shaded, 1=Wireframe, 2=Unlit, 3=Normals, 4=UVs)
    viewport_view_mode: usize,
    /// Viewport gizmo mode (0=Translate, 1=Rotate, 2=Scale)
    viewport_gizmo_mode: usize,
    /// Viewport gizmo space (0=Local, 1=World)
    viewport_gizmo_space: usize,
    /// Viewport show grid
    viewport_show_grid: bool,
    /// Viewport show bounds
    viewport_show_bounds: bool,
    /// Viewport show wireframe overlay
    viewport_show_wireframe: bool,
    /// Viewport show stats overlay
    viewport_show_stats: bool,
    /// Viewport camera speed
    viewport_camera_speed: f32,
    /// Viewport field of view
    viewport_fov: f32,
    /// Viewport near clip
    viewport_near_clip: f32,
    /// Viewport far clip
    viewport_far_clip: f32,
    /// Editor font size
    font_size: f32,
    /// Panel padding
    panel_padding: f32,
    /// Show tooltips
    show_tooltips: bool,
    /// Animation panel open
    animations_panel_open: bool,
    /// Auto-save interval (0 = disabled)
    auto_save_interval: u32,
    /// Gizmo size
    gizmo_size: f32,
    /// Icon size
    icon_size: f32,
    /// Asset browser search filter
    asset_search: String,
    /// Asset browser type filter (0=All, 1=Textures, 2=Models, 3=Audio, 4=Scripts)
    asset_type_filter: usize,
    /// Asset browser view mode (0=List, 1=Grid)
    asset_view_mode: usize,
    /// World ambient light color
    world_ambient_color: [f32; 3],
    /// World fog enabled
    world_fog_enabled: bool,
    /// World fog density
    world_fog_density: f32,
    /// World fog start distance
    world_fog_start: f32,
    /// World fog end distance
    world_fog_end: f32,
    /// World sun intensity
    world_sun_intensity: f32,
    /// World sun elevation (degrees)
    world_sun_elevation: f32,
    /// World sun azimuth (degrees)
    world_sun_azimuth: f32,
    /// World sun color
    world_sun_color: [f32; 3],
    /// World ambient intensity
    world_ambient_intensity: f32,
    /// World exposure
    world_exposure: f32,
    /// World gravity
    world_gravity: f32,
    /// Transform snap value (0 = disabled)
    transform_snap_value: f32,
    /// World skybox preset
    world_skybox_preset: usize,
    /// Custom HDRI file path (if loaded)
    world_hdri_path: Option<std::path::PathBuf>,
    /// World time of day (0-24 hours)
    world_time_of_day: f32,
    /// World weather preset
    world_weather_preset: usize,
    /// Weather particle count override (None = use default for weather type)
    world_particle_count_override_enabled: bool,
    /// The particle count value when override is enabled
    world_particle_count_value: u32,
    /// Scene has unsaved changes
    scene_modified: bool,
    /// Current scene name
    scene_name: String,
    /// Terrain generation panel state
    terrain_panel: TerrainPanel,
    // === New Phase 8 SOTA Panels ===
    /// UI Editor panel
    ui_editor_panel: UiEditorPanel,
    /// Foliage painting panel
    foliage_panel: FoliagePanel,
    /// Spline editor panel
    spline_editor_panel: SplineEditorPanel,
    /// LOD configuration panel
    lod_config_panel: LodConfigPanel,
    /// Localization panel
    localization_panel: LocalizationPanel,
    /// Project settings panel
    project_settings_panel: ProjectSettingsPanel,
    /// Audio mixer panel
    audio_panel: AudioPanel,
    /// Cinematics timeline panel
    cinematics_panel: CinematicsPanel,
    /// Dialogue editor panel
    dialogue_editor_panel: DialogueEditorPanel,
    /// Lighting configuration panel
    lighting_panel: LightingPanel,
    /// Navigation mesh panel
    navigation_panel: NavigationPanel,
    /// Networking configuration panel
    networking_panel: NetworkingPanel,
    /// Particle system panel
    particle_system_panel: ParticleSystemPanel,
    /// PCG (procedural content generation) panel
    pcg_panel: PcgPanel,
    /// Physics settings panel
    physics_panel: PhysicsPanel,
    /// Post-processing panel
    post_process_panel: PostProcessPanel,
    /// Input bindings panel
    input_bindings_panel: InputBindingsPanel,
    /// Frame debugger panel
    frame_debugger_panel: crate::panels::FrameDebuggerPanel,
    /// Blend import panel
    blend_import_panel: BlendImportPanel,
    /// Blueprint zone editor panel
    blueprint_panel: crate::panels::BlueprintPanel,
    /// Phase 1.X-F.5-paint: paintable regional archetype mask editor panel
    regional_archetype_panel: crate::panels::regional_archetype_panel::RegionalArchetypePanel,
    /// Blend asset scanner (discovers .blend files in configured directories)
    blend_scanner: crate::blend_scanner::BlendAssetScanner,
    /// Zone registry (manages all active blueprint zones)
    zone_registry: astraweave_terrain::ZoneRegistry,
    /// Material editor panel
    material_editor_panel: MaterialEditorPanel,
    /// Available 3D models from asset packs (name, path)
    spawnable_models: Vec<(String, String)>,
    /// Entity catalog (KayKit character/enemy/boss thumbnail picker)
    entity_catalog: EntityCatalogState,
    /// Real filesystem-backed asset browser panel
    asset_browser_panel: AssetBrowser,
    /// Pending events from component inspectors (drained each frame)
    pending_events: Vec<PanelEvent>,
    // === Play Session Profiling ===
    /// When the current play session started
    play_session_start: Option<std::time::Instant>,
    /// Total frames during this play session
    play_total_frames: u64,
    /// Frames that exceeded the 16.67ms budget
    play_budget_violations: u64,
    /// Peak frame time during this session
    play_peak_frame_ms: f32,
    /// Accumulated frame time for computing average
    play_total_frame_ms: f64,
    /// Previous is_playing state for detecting transitions
    play_was_playing: bool,
    /// Confirmation flag for "Clear All" entities action
    confirm_clear_all: bool,
    /// Separate search field for the Add Panel popup (avoids sharing hierarchy_search)
    panel_search: String,
    /// Currently selected graph node ID (visual scripting)
    selected_graph_node: Option<u32>,
}

impl Default for EditorTabViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorTabViewer {
    /// Create a new editor tab viewer
    pub fn new() -> Self {
        Self {
            selected_entity: None,
            is_playing: false,
            panels_to_close: Vec::new(),
            panels_to_add: Vec::new(),
            events: Vec::new(),
            entity_list: Vec::new(),
            terrain_chunk_count: 0,
            console_logs: std::collections::VecDeque::new(),
            console_error_count: 0,
            console_warn_count: 0,
            selected_transform: None,
            previous_transform: None,
            selected_entity_info: None,
            hierarchy_search: String::new(),
            console_search: String::new(),
            show_errors: true,
            show_warnings: true,
            show_info: true,
            runtime_stats: RuntimeStatsInfo::default(),
            scene_stats: SceneStatsInfo::default(),
            asset_entries: Vec::new(),
            asset_current_path: "assets/".to_string(),
            undo_count: 0,
            redo_count: 0,
            frame_time_history: Vec::with_capacity(120),
            current_material: MaterialInfo::default(),
            current_theme: EditorTheme::default(),
            ui_scale: 1.0,
            grid_enabled: true,
            snap_enabled: true,
            animation_state: AnimationState {
                is_playing: false,
                current_frame: 0,
                total_frames: 120,
                fps: 30.0,
                selected_track: None,
                playback_speed: 1.0,
                loop_enabled: true,
                ping_pong: false,
                tracks: vec![
                    AnimationTrack {
                        name: "Position X".to_string(),
                        keyframes: vec![
                            Keyframe {
                                frame: 0,
                                value: 0.0,
                            },
                            Keyframe {
                                frame: 30,
                                value: 50.0,
                            },
                            Keyframe {
                                frame: 60,
                                value: 100.0,
                            },
                        ],
                        is_visible: true,
                        is_locked: false,
                    },
                    AnimationTrack {
                        name: "Position Y".to_string(),
                        keyframes: vec![
                            Keyframe {
                                frame: 0,
                                value: 0.0,
                            },
                            Keyframe {
                                frame: 60,
                                value: 75.0,
                            },
                        ],
                        is_visible: true,
                        is_locked: false,
                    },
                    AnimationTrack {
                        name: "Rotation".to_string(),
                        keyframes: vec![
                            Keyframe {
                                frame: 0,
                                value: 0.0,
                            },
                            Keyframe {
                                frame: 120,
                                value: 360.0,
                            },
                        ],
                        is_visible: true,
                        is_locked: false,
                    },
                ],
            },
            behavior_graph: BehaviorGraphState {
                nodes: vec![
                    BehaviorNode {
                        id: 0,
                        name: "Root".to_string(),
                        node_type: BehaviorNodeType::Root,
                        position: (200.0, 50.0),
                        children: vec![1],
                    },
                    BehaviorNode {
                        id: 1,
                        name: "Selector".to_string(),
                        node_type: BehaviorNodeType::Selector,
                        position: (200.0, 120.0),
                        children: vec![2, 3],
                    },
                    BehaviorNode {
                        id: 2,
                        name: "Attack".to_string(),
                        node_type: BehaviorNodeType::Action,
                        position: (100.0, 200.0),
                        children: vec![],
                    },
                    BehaviorNode {
                        id: 3,
                        name: "Patrol".to_string(),
                        node_type: BehaviorNodeType::Action,
                        position: (300.0, 200.0),
                        children: vec![],
                    },
                ],
                connections: vec![(0, 1), (1, 2), (1, 3)],
                selected_node: None,
            },
            graph_nodes: vec![
                GraphNode {
                    id: 0,
                    name: "Start".to_string(),
                    node_type: "Event".to_string(),
                    position: (50.0, 100.0),
                    inputs: vec![],
                    outputs: vec!["OnStart".to_string()],
                },
                GraphNode {
                    id: 1,
                    name: "Get Position".to_string(),
                    node_type: "Function".to_string(),
                    position: (200.0, 100.0),
                    inputs: vec!["Entity".to_string()],
                    outputs: vec!["X".to_string(), "Y".to_string()],
                },
                GraphNode {
                    id: 2,
                    name: "Add".to_string(),
                    node_type: "Math".to_string(),
                    position: (350.0, 100.0),
                    inputs: vec!["A".to_string(), "B".to_string()],
                    outputs: vec!["Result".to_string()],
                },
            ],
            build_target: 0,
            build_profile: 1,
            build_include_debug_symbols: true,
            build_strip_unused: false,
            build_compress_textures: true,
            build_texture_quality: 2, // High
            build_output: Vec::new(),
            build_status: 0, // Idle
            build_progress: 0.0,
            build_start_time: None,
            // Viewport settings
            viewport_view_mode: 0,
            viewport_gizmo_mode: 0,
            viewport_gizmo_space: 1,
            viewport_show_grid: true,
            viewport_show_bounds: false,
            viewport_show_wireframe: false,
            viewport_show_stats: true,
            viewport_camera_speed: 10.0,
            viewport_fov: 60.0,
            viewport_near_clip: 0.1,
            viewport_far_clip: 1000.0,
            // Editor appearance settings
            font_size: 13.0,
            panel_padding: 8.0,
            show_tooltips: true,
            animations_panel_open: false,
            auto_save_interval: 0, // Disabled by default
            gizmo_size: 1.0,
            icon_size: 16.0,
            // Asset browser settings
            asset_search: String::new(),
            asset_type_filter: 0, // All
            asset_view_mode: 0,   // List
            // World settings
            world_ambient_color: [0.65, 0.58, 0.50],
            world_fog_enabled: false,
            world_fog_density: 0.0,
            world_fog_start: 800.0,
            world_fog_end: 1800.0,
            world_sun_intensity: 2.2,
            world_sun_elevation: 55.0,
            world_sun_azimuth: 35.0,
            world_sun_color: [1.0, 0.96, 0.88],
            world_ambient_intensity: 0.45,
            world_exposure: 1.3,
            world_gravity: -9.81,
            // Transform settings
            transform_snap_value: 1.0,
            // Additional world settings
            world_skybox_preset: 0, // Clear Sky
            world_hdri_path: None,
            world_time_of_day: 12.0, // Noon
            world_weather_preset: 0, // Clear
            world_particle_count_override_enabled: false,
            world_particle_count_value: 20000,
            // Scene state
            scene_modified: false,
            scene_name: "Untitled".to_string(),
            // Terrain panel
            terrain_panel: TerrainPanel::new(),
            // === New Phase 8 SOTA Panels ===
            ui_editor_panel: UiEditorPanel::new(),
            foliage_panel: FoliagePanel::new(),
            spline_editor_panel: SplineEditorPanel::new(),
            lod_config_panel: LodConfigPanel::new(),
            localization_panel: LocalizationPanel::new(),
            project_settings_panel: ProjectSettingsPanel::new(),
            audio_panel: AudioPanel::new(),
            cinematics_panel: CinematicsPanel::new(),
            dialogue_editor_panel: DialogueEditorPanel::new(),
            lighting_panel: LightingPanel::new(),
            navigation_panel: NavigationPanel::new(),
            networking_panel: NetworkingPanel::new(),
            particle_system_panel: ParticleSystemPanel::new(),
            pcg_panel: PcgPanel::new(),
            physics_panel: PhysicsPanel::new(),
            post_process_panel: PostProcessPanel::new(),
            input_bindings_panel: InputBindingsPanel::new(),
            frame_debugger_panel: crate::panels::FrameDebuggerPanel::new(),
            blend_import_panel: BlendImportPanel::new(),
            blueprint_panel: crate::panels::BlueprintPanel::new(),
            regional_archetype_panel:
                crate::panels::regional_archetype_panel::RegionalArchetypePanel::new(),
            blend_scanner: crate::blend_scanner::BlendAssetScanner::new(Vec::new()),
            zone_registry: astraweave_terrain::ZoneRegistry::new(),
            material_editor_panel: MaterialEditorPanel::new(),
            spawnable_models: Vec::new(),
            entity_catalog: EntityCatalogState::new(),
            asset_browser_panel: AssetBrowser::new(std::path::PathBuf::from("assets")),
            pending_events: Vec::new(),
            play_session_start: None,
            play_total_frames: 0,
            play_budget_violations: 0,
            play_peak_frame_ms: 0.0,
            play_total_frame_ms: 0.0,
            play_was_playing: false,
            confirm_clear_all: false,
            panel_search: String::new(),
            selected_graph_node: None,
        }
    }

    fn classify_console_line(line: &str) -> (bool, bool) {
        let is_error = line.contains("[ERROR]");
        let is_warn = line.contains("[WARN]");
        (is_error, is_warn)
    }

    fn push_console_log(&mut self, message: String) {
        let (is_error, is_warn) = Self::classify_console_line(&message);
        self.console_logs.push_back(message);
        if is_error {
            self.console_error_count = self.console_error_count.saturating_add(1);
        } else if is_warn {
            self.console_warn_count = self.console_warn_count.saturating_add(1);
        }

        // Keep last 1000 logs.
        if self.console_logs.len() > 1000 {
            if let Some(removed) = self.console_logs.pop_front() {
                let (was_error, was_warn) = Self::classify_console_line(&removed);
                if was_error {
                    self.console_error_count = self.console_error_count.saturating_sub(1);
                } else if was_warn {
                    self.console_warn_count = self.console_warn_count.saturating_sub(1);
                }
            }
        }
    }

    fn set_console_logs_inner(&mut self, mut logs: Vec<String>) {
        // Keep last 1000 logs (preserve chronological order).
        if logs.len() > 1000 {
            logs.drain(..(logs.len() - 1000));
        }

        let mut error_count = 0usize;
        let mut warn_count = 0usize;
        for line in &logs {
            let (is_error, is_warn) = Self::classify_console_line(line);
            if is_error {
                error_count = error_count.saturating_add(1);
            } else if is_warn {
                warn_count = warn_count.saturating_add(1);
            }
        }

        self.console_logs = logs.into();
        self.console_error_count = error_count;
        self.console_warn_count = warn_count;
    }

    /// Set the selected entity
    pub fn set_selected_entity(&mut self, entity: Option<u64>) {
        self.selected_entity = entity;
    }

    /// Set play mode state
    pub fn set_is_playing(&mut self, playing: bool) {
        self.is_playing = playing;
    }

    /// Update entity list for hierarchy panel
    pub fn set_entity_list(&mut self, entities: Vec<EntityInfo>) {
        self.entity_list = entities;
    }

    /// Set the number of terrain chunks for hierarchy display.
    /// When > 0, the hierarchy shows a "Terrain" group and the scene
    /// is no longer considered empty.
    pub fn set_terrain_chunk_count(&mut self, count: usize) {
        self.terrain_chunk_count = count;
    }

    /// Add a console log entry
    pub fn add_log(&mut self, message: String) {
        self.push_console_log(message);
    }

    /// Set scene modified state
    pub fn set_scene_modified(&mut self, modified: bool) {
        self.scene_modified = modified;
    }

    /// Set scene name
    pub fn set_scene_name(&mut self, name: String) {
        self.scene_name = name;
    }

    /// Set console logs
    pub fn set_console_logs(&mut self, logs: Vec<String>) {
        self.set_console_logs_inner(logs);
    }

    /// Update selected entity's transform (synced from main editor)
    pub fn set_selected_transform(
        &mut self,
        transform: Option<(f32, f32, f32, f32, f32, f32, f32)>,
    ) {
        // Only update if this is an external sync, not our own edit
        if self.selected_transform != transform {
            self.selected_transform = transform;
            self.previous_transform = transform;
        }
    }

    /// Check for transform changes and emit events
    pub fn check_transform_changes(&mut self) {
        if let (Some(entity_id), Some(current), Some(prev)) = (
            self.selected_entity,
            self.selected_transform,
            self.previous_transform,
        ) {
            let (x, y, z, rot, sx, sy, sz) = current;
            let (px, py, pz, prot, psx, psy, psz) = prev;

            // Check position change
            if (x - px).abs() > 0.001 || (y - py).abs() > 0.001 || (z - pz).abs() > 0.001 {
                self.events
                    .push(PanelEvent::TransformPositionChanged { entity_id, x, y, z });
            }

            // Check rotation change
            if (rot - prot).abs() > 0.001 {
                self.events.push(PanelEvent::TransformRotationChanged {
                    entity_id,
                    rotation: rot,
                });
            }

            // Check scale change
            if (sx - psx).abs() > 0.001 || (sy - psy).abs() > 0.001 || (sz - psz).abs() > 0.001 {
                self.events.push(PanelEvent::TransformScaleChanged {
                    entity_id,
                    scale_x: sx,
                    scale_y: sy,
                    scale_z: sz,
                });
            }

            self.previous_transform = self.selected_transform;
        }
    }

    /// Update selected entity info
    pub fn set_selected_entity_info(&mut self, info: Option<EntityInfo>) {
        self.selected_entity_info = info;
    }

    /// Update runtime statistics
    pub fn set_runtime_stats(&mut self, stats: RuntimeStatsInfo) {
        self.runtime_stats = stats;
    }

    /// Track play session metrics each frame.
    /// Call after `set_runtime_stats` so the latest stats are available.
    pub fn update_play_session(&mut self) {
        let is_playing = self.runtime_stats.is_playing;
        // Detect play → stop transition: reset session
        if self.play_was_playing && !is_playing {
            self.play_session_start = None;
            self.play_total_frames = 0;
            self.play_budget_violations = 0;
            self.play_peak_frame_ms = 0.0;
            self.play_total_frame_ms = 0.0;
        }
        // Detect stop → play transition: start session
        if !self.play_was_playing && is_playing {
            self.play_session_start = Some(std::time::Instant::now());
            self.play_total_frames = 0;
            self.play_budget_violations = 0;
            self.play_peak_frame_ms = 0.0;
            self.play_total_frame_ms = 0.0;
        }
        // Accumulate while playing (and not paused)
        if is_playing && !self.runtime_stats.is_paused {
            let ft = self.runtime_stats.frame_time_ms;
            self.play_total_frames += 1;
            self.play_total_frame_ms += ft as f64;
            if ft > self.play_peak_frame_ms {
                self.play_peak_frame_ms = ft;
            }
            if ft > 16.67 {
                self.play_budget_violations += 1;
            }
        }
        self.play_was_playing = is_playing;
    }

    /// Push a frame time to the history graph
    pub fn push_frame_time(&mut self, frame_time_ms: f32) {
        if self.frame_time_history.len() >= 120 {
            self.frame_time_history.remove(0);
        }
        self.frame_time_history.push(frame_time_ms);
    }

    /// Feed the frame debugger with current-frame render timing data.
    pub fn update_frame_debugger(
        &mut self,
        render_ms: f32,
        entity_count: usize,
        terrain_active: bool,
    ) {
        self.frame_debugger_panel
            .build_snapshot(render_ms, entity_count, terrain_active);
    }

    /// Update scene statistics
    pub fn set_scene_stats(&mut self, stats: SceneStatsInfo) {
        self.scene_stats = stats;
    }

    /// Access scene stats mutably
    pub fn scene_stats_mut(&mut self) -> &mut SceneStatsInfo {
        &mut self.scene_stats
    }

    /// Set hierarchy search text
    pub fn set_hierarchy_search(&mut self, search: String) {
        self.hierarchy_search = search;
    }

    /// Set console search text
    pub fn set_console_search(&mut self, search: String) {
        self.console_search = search;
    }

    /// Set available 3D models for spawning from asset packs
    pub fn set_spawnable_models(&mut self, models: Vec<(String, String)>) {
        self.spawnable_models = models;
    }

    /// Update asset browser entries
    pub fn set_asset_entries(&mut self, entries: Vec<AssetEntry>, path: String) {
        self.asset_entries = entries;
        self.asset_current_path = path;
    }

    /// Update undo/redo counts
    pub fn set_undo_redo_counts(&mut self, undo: usize, redo: usize) {
        self.undo_count = undo;
        self.redo_count = redo;
    }

    /// Drain and return panels that were closed this frame
    pub fn take_closed_panels(&mut self) -> Vec<PanelType> {
        std::mem::take(&mut self.panels_to_close)
    }

    /// Drain and return panels that should be added this frame
    pub fn take_panels_to_add(&mut self) -> Vec<PanelType> {
        std::mem::take(&mut self.panels_to_add)
    }

    /// Drain and return events that were emitted this frame
    pub fn take_events(&mut self) -> Vec<PanelEvent> {
        // Always poll terrain generation so wizard-triggered terrain
        // completes even when the Terrain tab is not visible.
        self.terrain_panel.poll_generation();
        if self.terrain_panel.has_pending_actions() {
            for action in self.terrain_panel.take_actions() {
                match action {
                    crate::panels::terrain_panel::TerrainAction::Generate => {
                        self.emit_event(PanelEvent::TerrainReady);
                    }
                    crate::panels::terrain_panel::TerrainAction::BrushUpdate => {
                        self.emit_event(PanelEvent::TerrainBrushUpdate);
                    }
                    _ => {}
                }
            }
        }

        // Merge pending inspector events
        self.events.append(&mut self.pending_events);
        std::mem::take(&mut self.events)
    }

    /// Get GPU-ready terrain chunks from the terrain panel
    pub fn terrain_gpu_chunks(
        &self,
    ) -> Vec<(Vec<crate::terrain_integration::TerrainVertex>, Vec<u32>)> {
        self.terrain_panel.get_gpu_chunks()
    }

    /// Take dirty terrain chunks for incremental GPU update after brush strokes.
    pub fn take_terrain_dirty_chunks(
        &mut self,
    ) -> Vec<(usize, Vec<crate::terrain_integration::TerrainVertex>)> {
        self.terrain_panel.take_dirty_chunks()
    }

    /// Returns (min_height, max_height, avg_height) from the last terrain generation.
    pub fn terrain_height_stats(&self) -> (f32, f32, f32) {
        self.terrain_panel.height_stats()
    }

    /// Return the currently loaded BiomePack (if any) for texture injection.
    pub fn cached_biome_pack(&self) -> Option<&astraweave_terrain::BiomePack> {
        self.terrain_panel.cached_biome_pack()
    }

    /// Current water surface level from terrain panel.
    pub fn water_level(&self) -> f32 {
        self.terrain_panel.water_level
    }

    /// Sample the terrain height at a world (x, z) position.
    /// Returns the height if terrain exists at that location.
    pub fn sample_terrain_height_at(&self, world_x: f32, world_z: f32) -> Option<f32> {
        self.terrain_panel
            .terrain_state()
            .sample_height_at(world_x, world_z)
    }

    /// Returns the current primary biome name from the terrain panel.
    pub fn terrain_primary_biome(&self) -> &str {
        self.terrain_panel.primary_biome()
    }

    /// Generate scatter placements from current terrain (vegetation, rocks, etc.)
    pub fn generate_scatter_placements(&self) -> Vec<crate::terrain_integration::ScatterPlacement> {
        self.terrain_panel.generate_scatter_placements()
    }

    /// Take pre-computed scatter placements (generated on background thread).
    pub fn take_cached_scatter_placements(
        &mut self,
    ) -> Vec<crate::terrain_integration::ScatterPlacement> {
        self.terrain_panel.take_cached_scatter_placements()
    }

    /// Returns true if terrain brush is active and terrain exists
    pub fn is_terrain_brush_active(&self) -> bool {
        self.terrain_panel.is_brush_active()
    }

    /// Whether terrain rendering should be active (has generated terrain).
    pub fn is_terrain_active(&self) -> bool {
        self.terrain_panel.has_generated()
    }

    /// Collect terrain chunks for zone scatter generation.
    /// Ensure terrain chunks exist — auto-generates with default grassland if empty.
    /// Returns the number of chunks generated (0 if terrain already existed).
    pub fn ensure_terrain_exists(&mut self) -> usize {
        self.ensure_terrain_exists_with_biome("grassland", 12345)
    }

    /// Get the terrain generator seed from the terrain panel.
    pub fn terrain_seed(&self) -> u64 {
        self.terrain_panel.terrain_state().seed()
    }

    /// Ensure terrain chunks exist, generating with the specified biome if empty.
    /// Used by blueprint zone generation to create terrain matching the zone's biome.
    pub fn ensure_terrain_exists_with_biome(&mut self, biome: &str, seed: u64) -> usize {
        let ts = self.terrain_panel.terrain_state_mut();
        if ts.chunks().next().is_some() {
            return 0; // Already have terrain
        }
        ts.configure(seed, biome);
        match ts.generate_terrain(2) {
            Ok(count) => count,
            Err(_) => 0,
        }
    }

    /// Returns borrowed TerrainChunks from the terrain state.
    pub fn collect_terrain_chunks(&self) -> Vec<&astraweave_terrain::TerrainChunk> {
        self.terrain_panel
            .terrain_state()
            .chunks()
            .map(|(_id, gen)| &gen.chunk)
            .collect()
    }

    /// Configure terrain panel and trigger background generation.
    /// Used by the World Wizard pipeline.
    pub fn trigger_terrain_generation(&mut self, seed: u64, biome: &str, chunk_radius: i32) {
        self.terrain_panel
            .configure_and_generate(seed, biome, chunk_radius);
    }

    /// Apply heightmap patches from zone scatter generation to the terrain state.
    /// Mutates affected chunks and marks them dirty for GPU re-upload.
    pub fn apply_zone_heightmap_patches(&mut self, patches: &[astraweave_terrain::HeightmapPatch]) {
        self.terrain_panel
            .terrain_state_mut()
            .apply_zone_heightmap_patches(patches);
    }

    /// Apply terrain brush at world coordinates (forwarded from viewport)
    pub fn apply_terrain_brush_at(&mut self, world_x: f32, world_z: f32) {
        self.terrain_panel.apply_brush_at(world_x, world_z);
    }

    /// Signal end of terrain brush stroke (mouse released).
    /// Returns undo data if any chunks were modified.
    pub fn end_terrain_brush_stroke(
        &mut self,
    ) -> Option<Vec<(astraweave_terrain::ChunkId, Vec<f32>, Vec<f32>)>> {
        self.terrain_panel.end_brush_stroke()
    }

    /// Returns the current brush mode name (for undo descriptions).
    pub fn terrain_brush_mode_name(&self) -> &'static str {
        self.terrain_panel.brush_mode_name()
    }

    /// Returns the current terrain brush radius.
    pub fn terrain_brush_radius(&self) -> f32 {
        self.terrain_panel.brush_radius()
    }

    /// Returns true if the current terrain brush is in Paint mode.
    pub fn terrain_brush_is_paint(&self) -> bool {
        self.terrain_panel.is_paint_mode()
    }

    /// Apply a heightmap snapshot from undo/redo to the terrain state.
    pub fn apply_terrain_height_snapshot(
        &mut self,
        snapshot: &[(astraweave_terrain::ChunkId, Vec<f32>)],
    ) {
        self.terrain_panel.apply_height_snapshot(snapshot);
    }

    /// Returns (fog_enabled, fog_density, fog_start, fog_end, weather_preset, particle_count_override) for the current world settings.
    pub fn fog_weather_params(&self) -> (bool, f32, f32, f32, u32, Option<u32>) {
        sky_colors::fog_weather_params(
            self.world_fog_enabled,
            self.world_fog_density,
            self.world_fog_start,
            self.world_fog_end,
            self.world_weather_preset,
            self.world_particle_count_override_enabled,
            self.world_particle_count_value,
        )
    }

    /// Map the world weather preset index to an engine WeatherKind.
    pub fn weather_kind(&self) -> astraweave_render::WeatherKind {
        match self.world_weather_preset {
            2 | 3 => astraweave_render::WeatherKind::Rain, // Rain, Storm
            4 => astraweave_render::WeatherKind::Snow,
            6 => astraweave_render::WeatherKind::Sandstorm,
            _ => astraweave_render::WeatherKind::None, // Clear, Cloudy, Fog
        }
    }

    /// Returns lighting parameters for the terrain shader.
    pub fn lighting_params(&self) -> crate::viewport::types::TerrainLightingParams {
        sky_colors::lighting_params(
            self.world_sun_elevation,
            self.world_sun_azimuth,
            self.world_sun_color,
            self.world_sun_intensity,
            self.world_ambient_color,
            self.world_ambient_intensity,
            self.world_exposure,
        )
    }

    /// Compute the current sky colors from skybox preset, time-of-day, and weather settings.
    ///
    /// Returns `(sky_top, sky_horizon, ground_color)` as `[f32; 4]` RGBA arrays.
    pub fn compute_sky_colors(&self) -> ([f32; 4], [f32; 4], [f32; 4]) {
        sky_colors::compute_sky_colors(
            self.world_skybox_preset,
            self.world_time_of_day,
            self.world_weather_preset,
            self.world_fog_enabled,
            self.world_fog_density,
            self.world_ambient_color,
        )
    }

    /// Drain pending asset browser actions (import, apply texture, etc.)
    pub fn take_asset_browser_actions(&mut self) -> Vec<crate::panels::AssetAction> {
        self.asset_browser_panel.take_pending_actions()
    }

    /// Drain pending blend import panel actions
    pub fn take_blend_import_actions(
        &mut self,
    ) -> Vec<crate::panels::blend_import_panel::BlendImportAction> {
        self.blend_import_panel.take_actions()
    }

    /// Get a mutable reference to the blend import panel (for setting results from async ops)
    pub fn blend_import_panel_mut(&mut self) -> &mut BlendImportPanel {
        &mut self.blend_import_panel
    }

    /// Drain pending blueprint panel actions (zone generation, save/load)
    pub fn take_blueprint_actions(
        &mut self,
    ) -> Vec<crate::panels::blueprint_panel::BlueprintAction> {
        self.blueprint_panel.take_actions()
    }

    /// Get immutable access to the blueprint panel's zones
    pub fn blueprint_zones(&self) -> &[crate::panels::blueprint_panel::ZoneState] {
        self.blueprint_panel.zones()
    }

    /// Replace all zones in the blueprint panel (e.g., after loading from file)
    pub fn set_blueprint_zones(&mut self, zones: Vec<crate::panels::blueprint_panel::ZoneState>) {
        self.blueprint_panel.set_zones(zones);
    }

    /// Add a single zone to the blueprint panel (e.g., from blend import flow).
    pub fn add_blueprint_zone(&mut self, zone: crate::panels::blueprint_panel::ZoneState) {
        self.blueprint_panel.add_zone(zone);
    }

    /// Get mutable access to the zone registry
    pub fn zone_registry_mut(&mut self) -> &mut astraweave_terrain::ZoneRegistry {
        &mut self.zone_registry
    }

    /// Get immutable access to the zone registry
    pub fn zone_registry(&self) -> &astraweave_terrain::ZoneRegistry {
        &self.zone_registry
    }

    /// Get mutable access to the blend scanner
    pub fn blend_scanner_mut(&mut self) -> &mut crate::blend_scanner::BlendAssetScanner {
        &mut self.blend_scanner
    }

    /// Drain pending audio panel actions for the audio bridge.
    pub fn take_audio_actions(&mut self) -> Vec<crate::panels::audio_panel::AudioAction> {
        self.audio_panel.take_actions()
    }

    /// Drain pending lighting panel actions.
    pub fn take_lighting_actions(&mut self) -> Vec<crate::panels::lighting_panel::LightingAction> {
        self.lighting_panel.take_actions()
    }

    /// If post-process settings changed this frame, returns the pending settings
    /// snapshot and clears the dirty flag.
    pub fn take_post_process_update(
        &mut self,
    ) -> Option<crate::panels::post_process_panel::PostProcessPendingSettings> {
        self.post_process_panel.take_pending_settings()
    }

    /// Push live audio stats back into the panel for display.
    pub fn set_audio_stats(&mut self, stats: crate::panels::audio_panel::AudioStats) {
        self.audio_panel.set_audio_stats(stats);
    }

    /// Push discovered music tracks into the audio panel's playlist.
    pub fn set_audio_tracks(&mut self, tracks: Vec<crate::panels::audio_panel::MusicTrackEntry>) {
        self.audio_panel.set_music_tracks(tracks);
    }

    /// Take dragged prefab from the asset browser
    pub fn take_asset_browser_dragged_prefab(&mut self) -> Option<std::path::PathBuf> {
        self.asset_browser_panel.take_dragged_prefab()
    }

    /// Emit a panel event
    fn emit_event(&mut self, event: PanelEvent) {
        self.events.push(event);
    }

    /// Clear frame state (call at start of each frame)
    pub fn begin_frame(&mut self) {
        self.panels_to_close.clear();
        self.panels_to_add.clear();
        self.events.clear();
    }

    /// Add an entity to the entity list
    pub fn add_entity(&mut self, entity: EntityInfo) {
        self.entity_list.push(entity);
    }

    /// Remove an entity from the list by ID
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.entity_list.retain(|e| e.id != entity_id);
    }

    /// Find an entity by ID
    pub fn find_entity(&self, entity_id: u64) -> Option<&EntityInfo> {
        self.entity_list.iter().find(|e| e.id == entity_id)
    }

    /// Get a mutable reference to the entity list
    pub fn entity_list_mut(&mut self) -> &mut Vec<EntityInfo> {
        &mut self.entity_list
    }

    /// Update animation playback (call each frame to advance if playing)
    pub fn update_animation(&mut self, delta_time: f32) {
        if self.animation_state.is_playing {
            let speed = self.animation_state.playback_speed;
            let frames_per_second = self.animation_state.fps;
            let frame_duration = 1.0 / frames_per_second;

            // Apply playback speed multiplier to delta_time
            let effective_dt = delta_time * speed;

            // Calculate how many frames to advance (assuming delta_time is in seconds)
            let frames_to_advance = (effective_dt / frame_duration) as u32;
            let advance = frames_to_advance.max(if effective_dt >= frame_duration { 1 } else { 0 });
            if advance > 0 {
                self.animation_state.current_frame = (self.animation_state.current_frame + advance)
                    % (self.animation_state.total_frames + 1);
            }
        }
    }

    /// Get current animation state
    pub fn animation_state(&self) -> &AnimationState {
        &self.animation_state
    }

    /// Set animation playing state
    pub fn set_animation_playing(&mut self, playing: bool) {
        self.animation_state.is_playing = playing;
        self.emit_event(PanelEvent::AnimationPlayStateChanged {
            is_playing: playing,
        });
    }

    /// Set animation current frame
    pub fn set_animation_frame(&mut self, frame: u32) {
        self.animation_state.current_frame = frame.min(self.animation_state.total_frames);
        self.emit_event(PanelEvent::AnimationFrameChanged { frame });
    }

    /// Get current material
    pub fn current_material(&self) -> &MaterialInfo {
        &self.current_material
    }

    /// Set current material
    pub fn set_current_material(&mut self, material: MaterialInfo) {
        self.current_material = material;
    }

    /// Get current theme
    pub fn current_theme(&self) -> EditorTheme {
        self.current_theme
    }

    /// Set current theme
    pub fn set_current_theme(&mut self, theme: EditorTheme) {
        if self.current_theme != theme {
            self.current_theme = theme;
            self.emit_event(PanelEvent::ThemeChanged(theme));
        }
    }

    /// Start a build (sets status to Building and records start time)
    pub fn start_build(&mut self) {
        self.build_status = 1; // Building
        self.build_progress = 0.0;
        self.build_start_time = Some(std::time::Instant::now());
    }

    /// Update build progress (0.0 to 1.0)
    pub fn set_build_progress(&mut self, progress: f32) {
        self.build_progress = progress.clamp(0.0, 1.0);
    }

    /// Complete build with success or failure
    pub fn complete_build(&mut self, success: bool) {
        self.build_status = if success { 2 } else { 3 }; // Success or Failed
        self.build_progress = 1.0;
    }

    /// Reset build status to idle
    pub fn reset_build_status(&mut self) {
        self.build_status = 0; // Idle
        self.build_progress = 0.0;
        self.build_start_time = None;
    }

    /// Set build target (0=Windows, 1=Linux, 2=macOS, 3=WebGL) and profile (0=Debug, 1=Release)
    pub fn set_build_config(&mut self, target: usize, profile: usize) {
        self.build_target = target.min(3);
        self.build_profile = profile.min(1);
    }

    /// Add build output message
    pub fn add_build_output(&mut self, message: String) {
        self.build_output.push(message);
    }

    /// Get behavior graph state
    pub fn behavior_graph(&self) -> &BehaviorGraphState {
        &self.behavior_graph
    }

    /// Get mutable behavior graph state
    pub fn behavior_graph_mut(&mut self) -> &mut BehaviorGraphState {
        &mut self.behavior_graph
    }

    /// Select a behavior node
    pub fn select_behavior_node(&mut self, node_id: Option<u32>) {
        self.behavior_graph.selected_node = node_id;
        if let Some(id) = node_id {
            self.emit_event(PanelEvent::BehaviorNodeSelected(id));
        }
    }

    /// Select a graph node (visual scripting)
    pub fn select_graph_node(&mut self, node_id: Option<u32>) {
        self.selected_graph_node = node_id;
        if let Some(id) = node_id {
            self.emit_event(PanelEvent::GraphNodeSelected(id));
        }
    }

    /// Get selected graph node ID
    pub fn selected_graph_node(&self) -> Option<u32> {
        self.selected_graph_node
    }

    /// Set viewport gizmo mode (0=Translate, 1=Rotate, 2=Scale)
    pub fn set_viewport_gizmo_mode(&mut self, mode: usize) {
        self.viewport_gizmo_mode = mode.min(2);
    }

    /// Set viewport gizmo space (0=Local, 1=World)
    pub fn set_viewport_gizmo_space(&mut self, space: usize) {
        self.viewport_gizmo_space = space.min(1);
    }

    /// Toggle a viewport overlay by name, returning whether it was recognized
    pub fn set_viewport_overlay(&mut self, overlay: &str, enabled: bool) -> bool {
        match overlay {
            "Grid" | "grid" => {
                self.viewport_show_grid = enabled;
                true
            }
            "Bounds" | "bounds" => {
                self.viewport_show_bounds = enabled;
                true
            }
            "Wire" | "wire" | "Wireframe" | "wireframe" => {
                self.viewport_show_wireframe = enabled;
                true
            }
            "Stats" | "stats" => {
                self.viewport_show_stats = enabled;
                true
            }
            _ => false,
        }
    }

    /// Get graph nodes
    pub fn graph_nodes(&self) -> &[GraphNode] {
        &self.graph_nodes
    }

    /// Add a console log message
    pub fn log(&mut self, message: String) {
        self.push_console_log(message);
    }

    /// Log an info message
    pub fn log_info(&mut self, message: &str) {
        self.log(format!("[INFO] {}", message));
    }

    /// Log a warning message
    pub fn log_warn(&mut self, message: &str) {
        self.log(format!("[WARN] {}", message));
    }

    /// Log an error message
    pub fn log_error(&mut self, message: &str) {
        self.log(format!("[ERROR] {}", message));
    }

    /// Get UI scale
    pub fn ui_scale(&self) -> f32 {
        self.ui_scale
    }

    /// Set UI scale
    pub fn set_ui_scale(&mut self, scale: f32) {
        self.ui_scale = scale.clamp(0.75, 2.0);
    }

    /// Check if grid is enabled
    pub fn is_grid_enabled(&self) -> bool {
        self.grid_enabled
    }

    /// Set grid enabled
    pub fn set_grid_enabled(&mut self, enabled: bool) {
        self.grid_enabled = enabled;
    }

    /// Check if snap is enabled  
    pub fn is_snap_enabled(&self) -> bool {
        self.snap_enabled
    }

    /// Set snap enabled
    pub fn set_snap_enabled(&mut self, enabled: bool) {
        self.snap_enabled = enabled;
    }
}

impl TabViewer for EditorTabViewer {
    type Tab = PanelType;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        // Clean tab titles — no emoji prefixes, contextual suffixes only
        match tab {
            PanelType::Viewport => {
                if self.is_playing {
                    format!("{} >", tab.title()).into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::Hierarchy => {
                let count = self.entity_list.len() + self.terrain_chunk_count;
                let modified = if self.scene_modified { " •" } else { "" };
                format!("{} ({}){}", tab.title(), count, modified).into()
            }
            PanelType::Console => {
                let total = self.console_logs.len();
                let error_count = self.console_error_count;
                if error_count > 0 {
                    format!("{} ({}) !{}", tab.title(), total, error_count).into()
                } else if total > 0 {
                    format!("{} ({})", tab.title(), total).into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::Inspector => {
                if let Some(ref info) = self.selected_entity_info {
                    format!("{} - {}", tab.title(), info.name).into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::Performance => {
                let grade = if self.runtime_stats.fps >= 60.0 {
                    "A+"
                } else if self.runtime_stats.fps >= 55.0 {
                    "A"
                } else if self.runtime_stats.fps >= 45.0 {
                    "B"
                } else if self.runtime_stats.fps >= 30.0 {
                    "C"
                } else {
                    "D"
                };
                if self.runtime_stats.is_playing {
                    format!(
                        "{} [{} {:.0}fps]",
                        tab.title(),
                        grade,
                        self.runtime_stats.fps
                    )
                    .into()
                } else {
                    format!("{} [{}]", tab.title(), grade).into()
                }
            }
            PanelType::Profiler => {
                if self.runtime_stats.tick_count > 0 {
                    format!("{} T{}", tab.title(), self.runtime_stats.tick_count).into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::Animation => {
                if self.animation_state.total_frames > 0 {
                    format!(
                        "{} {}/{}",
                        tab.title(),
                        self.animation_state.current_frame,
                        self.animation_state.total_frames
                    )
                    .into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::BuildManager => {
                let status = match self.build_status {
                    1 => format!(" {:.0}%", self.build_progress * 100.0),
                    2 => " OK".to_string(),
                    3 => " ERR".to_string(),
                    _ => String::new(),
                };
                format!("{}{}", tab.title(), status).into()
            }
            PanelType::AssetBrowser => {
                let count = self.asset_entries.len();
                if count > 0 {
                    format!("{} ({})", tab.title(), count).into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::Graph => {
                let count = self.graph_nodes.len();
                if count > 0 {
                    format!("{} ({})", tab.title(), count).into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::BehaviorGraph => {
                let count = self.behavior_graph.nodes.len();
                if count > 0 {
                    format!("{} ({})", tab.title(), count).into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::SceneStats => {
                let entity_count = self.entity_list.len();
                let modified = if self.scene_modified { " •" } else { "" };
                format!("{} ({}){}", tab.title(), entity_count, modified).into()
            }
            PanelType::EntityPanel => {
                if let Some(ref info) = self.selected_entity_info {
                    format!("{} - {}", tab.title(), info.name).into()
                } else {
                    tab.title().into()
                }
            }
            PanelType::MaterialEditor => {
                let modified = if self.current_material.name != "Default" {
                    " •"
                } else {
                    ""
                };
                format!(
                    "{} - {}{}",
                    tab.title(),
                    self.current_material.name,
                    modified
                )
                .into()
            }
            PanelType::ThemeManager => tab.title().into(),
            PanelType::World => {
                let modified = if self.scene_modified { " •" } else { "" };
                format!("{} - {}{}", tab.title(), self.scene_name, modified).into()
            }
            PanelType::Transform => tab.title().into(),
            _ => tab.title().into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // Ensure text wraps within panel boundaries (prevents overflow)
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

        // Play mode indicator
        if self.is_playing {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "> Play Mode");
            });
            ui.separator();
        }

        // Render panel content based on type
        match tab {
            PanelType::Viewport => {
                // Collect events to emit after UI updates
                let mut viewport_events: Vec<PanelEvent> = Vec::new();

                // Viewport toolbar
                ui.horizontal(|ui| {
                    // View mode selector
                    let view_modes = ["Shaded", "Wireframe", "Unlit", "Normals", "UVs"];
                    let prev_mode = self.viewport_view_mode;
                    egui::ComboBox::from_id_salt("view_mode")
                        .width(80.0)
                        .selected_text(view_modes[self.viewport_view_mode])
                        .show_ui(ui, |ui| {
                            for (i, mode) in view_modes.iter().enumerate() {
                                ui.selectable_value(&mut self.viewport_view_mode, i, *mode);
                            }
                        });
                    if self.viewport_view_mode != prev_mode {
                        viewport_events
                            .push(PanelEvent::ViewportViewModeChanged(self.viewport_view_mode));
                    }

                    ui.separator();

                    // Gizmo mode buttons with keyboard shortcuts
                    let gizmo_labels = ["Move", "Rotate", "Scale"];
                    let gizmo_shortcuts = ["G", "R", "S"];
                    for (i, (label, shortcut)) in
                        gizmo_labels.iter().zip(gizmo_shortcuts.iter()).enumerate()
                    {
                        let is_selected = self.viewport_gizmo_mode == i;
                        if ui
                            .selectable_label(is_selected, *label)
                            .on_hover_text(format!("Keyboard: {}", shortcut))
                            .clicked()
                            && !is_selected
                        {
                            self.viewport_gizmo_mode = i;
                            viewport_events.push(PanelEvent::ViewportGizmoModeChanged(i));
                        }
                    }

                    ui.separator();

                    // Gizmo space toggle
                    let space_text = if self.viewport_gizmo_space == 0 {
                        "Local"
                    } else {
                        "World"
                    };
                    if ui
                        .button(space_text)
                        .on_hover_text("Toggle local/world space")
                        .clicked()
                    {
                        self.viewport_gizmo_space = 1 - self.viewport_gizmo_space;
                        viewport_events.push(PanelEvent::ViewportGizmoSpaceChanged(
                            self.viewport_gizmo_space,
                        ));
                    }

                    ui.separator();

                    // Quick toggles
                    if ui
                        .selectable_label(self.viewport_show_grid, "Grid")
                        .clicked()
                    {
                        self.viewport_show_grid = !self.viewport_show_grid;
                        viewport_events.push(PanelEvent::ViewportOverlayToggled {
                            overlay: "grid".to_string(),
                            enabled: self.viewport_show_grid,
                        });
                    }
                    if ui
                        .selectable_label(self.viewport_show_bounds, "Bounds")
                        .clicked()
                    {
                        self.viewport_show_bounds = !self.viewport_show_bounds;
                        viewport_events.push(PanelEvent::ViewportOverlayToggled {
                            overlay: "bounds".to_string(),
                            enabled: self.viewport_show_bounds,
                        });
                    }
                    if ui
                        .selectable_label(self.viewport_show_wireframe, "Wire")
                        .clicked()
                    {
                        self.viewport_show_wireframe = !self.viewport_show_wireframe;
                        viewport_events.push(PanelEvent::ViewportOverlayToggled {
                            overlay: "wireframe".to_string(),
                            enabled: self.viewport_show_wireframe,
                        });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Focus and reset buttons
                        if ui
                            .small_button("H")
                            .on_hover_text("Reset camera (Home)")
                            .clicked()
                        {
                            viewport_events.push(PanelEvent::ViewportResetCamera);
                        }
                        if ui
                            .small_button("F")
                            .on_hover_text("Focus on selection (F)")
                            .clicked()
                        {
                            viewport_events.push(PanelEvent::ViewportFocusOnSelection);
                        }

                        // Stats toggle
                        if ui
                            .selectable_label(self.viewport_show_stats, "Stats")
                            .clicked()
                        {
                            self.viewport_show_stats = !self.viewport_show_stats;
                            viewport_events.push(PanelEvent::ViewportOverlayToggled {
                                overlay: "stats".to_string(),
                                enabled: self.viewport_show_stats,
                            });
                        }
                    });
                });

                ui.separator();

                // Main viewport area (placeholder for actual 3D rendering)
                let available = ui.available_size();
                let (viewport_rect, _response) = ui.allocate_exact_size(
                    egui::vec2(available.x, available.y - 30.0),
                    egui::Sense::click_and_drag(),
                );

                if ui.is_rect_visible(viewport_rect) {
                    let painter = ui.painter();

                    // Draw viewport background
                    painter.rect_filled(viewport_rect, 0.0, egui::Color32::from_rgb(40, 40, 45));

                    // Draw grid (if enabled)
                    if self.viewport_show_grid {
                        let grid_spacing = 50.0;
                        let grid_color = egui::Color32::from_rgba_unmultiplied(100, 100, 100, 40);

                        let start_x = (viewport_rect.left() / grid_spacing).floor() * grid_spacing;
                        let mut x = start_x;
                        while x < viewport_rect.right() {
                            painter.line_segment(
                                [
                                    egui::pos2(x, viewport_rect.top()),
                                    egui::pos2(x, viewport_rect.bottom()),
                                ],
                                egui::Stroke::new(1.0, grid_color),
                            );
                            x += grid_spacing;
                        }

                        let start_y = (viewport_rect.top() / grid_spacing).floor() * grid_spacing;
                        let mut y = start_y;
                        while y < viewport_rect.bottom() {
                            painter.line_segment(
                                [
                                    egui::pos2(viewport_rect.left(), y),
                                    egui::pos2(viewport_rect.right(), y),
                                ],
                                egui::Stroke::new(1.0, grid_color),
                            );
                            y += grid_spacing;
                        }
                    }

                    // Draw center text
                    painter.text(
                        viewport_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "3D Viewport\n(Rendered separately)",
                        egui::FontId::proportional(18.0),
                        egui::Color32::from_gray(120),
                    );

                    // Draw play state indicator
                    if self.is_playing {
                        let indicator_pos =
                            egui::pos2(viewport_rect.left() + 10.0, viewport_rect.top() + 10.0);
                        painter.circle_filled(indicator_pos, 6.0, egui::Color32::GREEN);
                        painter.text(
                            egui::pos2(indicator_pos.x + 12.0, indicator_pos.y),
                            egui::Align2::LEFT_CENTER,
                            "Playing",
                            egui::FontId::proportional(12.0),
                            egui::Color32::GREEN,
                        );
                    }

                    // Draw stats overlay (if enabled)
                    if self.viewport_show_stats {
                        let stats_pos =
                            egui::pos2(viewport_rect.right() - 10.0, viewport_rect.top() + 10.0);
                        let stats_text = format!(
                            "FPS: {:.0}\nDraw Calls: {}\nTris: {}",
                            self.runtime_stats.fps,
                            self.runtime_stats.draw_calls,
                            self.runtime_stats.triangles
                        );
                        painter.text(
                            stats_pos,
                            egui::Align2::RIGHT_TOP,
                            stats_text,
                            egui::FontId::monospace(11.0),
                            egui::Color32::from_gray(200),
                        );
                    }

                    // Draw gizmo hint
                    let gizmo_names = ["Move (W)", "Rotate (E)", "Scale (R)"];
                    let gizmo_pos =
                        egui::pos2(viewport_rect.left() + 10.0, viewport_rect.bottom() - 20.0);
                    painter.text(
                        gizmo_pos,
                        egui::Align2::LEFT_CENTER,
                        format!(
                            "Gizmo: {} | Space: {}",
                            gizmo_names[self.viewport_gizmo_mode],
                            if self.viewport_gizmo_space == 0 {
                                "Local"
                            } else {
                                "World"
                            }
                        ),
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_gray(150),
                    );
                }

                // Camera settings (collapsible at bottom)
                let mut camera_changed = false;
                ui.collapsing("Camera Settings", |ui| {
                    // Camera presets
                    ui.horizontal(|ui| {
                        ui.label("Preset:");
                        if ui
                            .small_button("Front")
                            .on_hover_text("View from front (Numpad 1)")
                            .clicked()
                        {
                            self.viewport_fov = 60.0;
                            camera_changed = true;
                            viewport_events
                                .push(PanelEvent::ViewportCameraPreset("front".to_string()));
                        }
                        if ui
                            .small_button("Top")
                            .on_hover_text("View from top (Numpad 7)")
                            .clicked()
                        {
                            self.viewport_fov = 60.0;
                            camera_changed = true;
                            viewport_events
                                .push(PanelEvent::ViewportCameraPreset("top".to_string()));
                        }
                        if ui
                            .small_button("Side")
                            .on_hover_text("View from side (Numpad 3)")
                            .clicked()
                        {
                            self.viewport_fov = 60.0;
                            camera_changed = true;
                            viewport_events
                                .push(PanelEvent::ViewportCameraPreset("side".to_string()));
                        }
                        if ui
                            .small_button("Persp")
                            .on_hover_text("Perspective view")
                            .clicked()
                        {
                            self.viewport_fov = 60.0;
                            camera_changed = true;
                            viewport_events
                                .push(PanelEvent::ViewportCameraPreset("perspective".to_string()));
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.viewport_camera_speed)
                                    .speed(0.1)
                                    .range(0.1..=100.0),
                            )
                            .changed()
                        {
                            camera_changed = true;
                        }
                        // Speed presets
                        if ui.small_button("1x").clicked() {
                            self.viewport_camera_speed = 5.0;
                            camera_changed = true;
                        }
                        if ui.small_button("2x").clicked() {
                            self.viewport_camera_speed = 10.0;
                            camera_changed = true;
                        }
                        if ui.small_button("4x").clicked() {
                            self.viewport_camera_speed = 20.0;
                            camera_changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("FOV:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.viewport_fov)
                                    .speed(1.0)
                                    .range(10.0..=120.0)
                                    .suffix("°"),
                            )
                            .changed()
                        {
                            camera_changed = true;
                        }
                        // FOV presets
                        if ui.small_button("45°").clicked() {
                            self.viewport_fov = 45.0;
                            camera_changed = true;
                        }
                        if ui.small_button("60°").clicked() {
                            self.viewport_fov = 60.0;
                            camera_changed = true;
                        }
                        if ui.small_button("90°").clicked() {
                            self.viewport_fov = 90.0;
                            camera_changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Clip:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.viewport_near_clip)
                                    .speed(0.01)
                                    .range(0.001..=10.0)
                                    .prefix("Near: "),
                            )
                            .changed()
                        {
                            camera_changed = true;
                        }
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.viewport_far_clip)
                                    .speed(10.0)
                                    .range(100.0..=100000.0)
                                    .prefix("Far: "),
                            )
                            .changed()
                        {
                            camera_changed = true;
                        }
                    });

                    // Clip presets
                    ui.horizontal(|ui| {
                        ui.label("Range:");
                        if ui
                            .small_button("Close")
                            .on_hover_text("0.01 - 100")
                            .clicked()
                        {
                            self.viewport_near_clip = 0.01;
                            self.viewport_far_clip = 100.0;
                            camera_changed = true;
                        }
                        if ui
                            .small_button("Default")
                            .on_hover_text("0.1 - 1000")
                            .clicked()
                        {
                            self.viewport_near_clip = 0.1;
                            self.viewport_far_clip = 1000.0;
                            camera_changed = true;
                        }
                        if ui
                            .small_button("Far")
                            .on_hover_text("1.0 - 10000")
                            .clicked()
                        {
                            self.viewport_near_clip = 1.0;
                            self.viewport_far_clip = 10000.0;
                            camera_changed = true;
                        }
                    });
                });

                if camera_changed {
                    viewport_events.push(PanelEvent::ViewportCameraChanged {
                        fov: self.viewport_fov,
                        near: self.viewport_near_clip,
                        far: self.viewport_far_clip,
                        speed: self.viewport_camera_speed,
                    });
                }

                // Emit all collected events
                for event in viewport_events {
                    self.emit_event(event);
                }
            }
            PanelType::Inspector => {
                // Enhanced header with entity name and component count
                ui.horizontal(|ui| {
                    if let Some(ref info) = self.selected_entity_info {
                        ui.heading(format!("Inspector - {}", info.name));
                        ui.weak(format!("• {} components", info.components.len()));
                    } else if self.selected_entity.is_some() {
                        ui.heading("Inspector");
                        ui.weak("• Loading...");
                    } else {
                        ui.heading("Inspector");
                    }
                });
                ui.separator();
                if let Some(entity_id) = self.selected_entity {
                    // Display entity info if available
                    if let Some(ref info) = self.selected_entity_info {
                        ui.label(format!("Entity: {} (ID: {})", info.name, entity_id));
                    } else {
                        ui.label(format!("Selected Entity: {}", entity_id));
                    }
                    ui.add_space(8.0);

                    // Editable transform data
                    let mut transform_events = Vec::new();

                    ui.collapsing("Transform", |ui| {
                        if let Some((
                            mut x,
                            mut y,
                            mut z,
                            mut rotation,
                            mut scale_x,
                            mut scale_y,
                            mut scale_z,
                        )) = self.selected_transform
                        {
                            let orig_x = x;
                            let orig_y = y;
                            let orig_z = z;
                            let _orig_rot_rad = rotation;
                            let orig_sx = scale_x;
                            let orig_sy = scale_y;
                            let orig_sz = scale_z;

                            // Position editing
                            ui.horizontal(|ui| {
                                ui.label("Position:");
                                ui.label("X");
                                ui.add(
                                    egui::DragValue::new(&mut x)
                                        .speed(0.1)
                                        .range(-10000.0..=10000.0),
                                );
                                ui.label("Y");
                                ui.add(
                                    egui::DragValue::new(&mut y)
                                        .speed(0.1)
                                        .range(-10000.0..=10000.0),
                                );
                                ui.label("Z");
                                ui.add(
                                    egui::DragValue::new(&mut z)
                                        .speed(0.1)
                                        .range(-10000.0..=10000.0),
                                );
                            });

                            // Rotation editing (convert from radians to degrees for UI)
                            let mut rot_degrees = rotation.to_degrees();
                            let orig_rot_degrees = rot_degrees;
                            ui.horizontal(|ui| {
                                ui.label("Rotation:");
                                ui.add(
                                    egui::DragValue::new(&mut rot_degrees)
                                        .speed(1.0)
                                        .suffix("°")
                                        .range(-360.0..=360.0),
                                );
                            });
                            rotation = rot_degrees.to_radians();

                            // Scale editing
                            ui.horizontal(|ui| {
                                ui.label("Scale:");
                                ui.label("X");
                                ui.add(
                                    egui::DragValue::new(&mut scale_x)
                                        .speed(0.01)
                                        .range(0.01..=100.0),
                                );
                                ui.label("Y");
                                ui.add(
                                    egui::DragValue::new(&mut scale_y)
                                        .speed(0.01)
                                        .range(0.01..=100.0),
                                );
                                ui.label("Z");
                                ui.add(
                                    egui::DragValue::new(&mut scale_z)
                                        .speed(0.01)
                                        .range(0.01..=100.0),
                                );
                            });

                            // Update transform and collect events
                            if (x - orig_x).abs() > 0.0001
                                || (y - orig_y).abs() > 0.0001
                                || (z - orig_z).abs() > 0.0001
                            {
                                transform_events.push(PanelEvent::TransformPositionChanged {
                                    entity_id,
                                    x,
                                    y,
                                    z,
                                });
                            }
                            if (rot_degrees - orig_rot_degrees).abs() > 0.0001 {
                                transform_events.push(PanelEvent::TransformRotationChanged {
                                    entity_id,
                                    rotation,
                                });
                            }
                            if (scale_x - orig_sx).abs() > 0.0001
                                || (scale_y - orig_sy).abs() > 0.0001
                                || (scale_z - orig_sz).abs() > 0.0001
                            {
                                transform_events.push(PanelEvent::TransformScaleChanged {
                                    entity_id,
                                    scale_x,
                                    scale_y,
                                    scale_z,
                                });
                            }

                            // Update cached transform
                            self.selected_transform =
                                Some((x, y, z, rotation, scale_x, scale_y, scale_z));
                        } else {
                            // Default transform when none set
                            ui.horizontal(|ui| {
                                ui.label("Position:");
                                ui.label("X: 0.0  Y: 0.0  Z: 0.0");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Rotation:");
                                ui.label("0°");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Scale:");
                                ui.label("X: 1.0  Y: 1.0  Z: 1.0");
                            });
                        }
                    });

                    // Emit transform events after UI to avoid borrow issues
                    for event in transform_events {
                        self.emit_event(event);
                    }

                    // Display components with add/remove functionality
                    let mut component_events: Vec<PanelEvent> = Vec::new();

                    ui.collapsing("Components", |ui| {
                        if let Some(ref info) = self.selected_entity_info {
                            for comp in &info.components {
                                ui.horizontal(|ui| {
                                    ui.label(format!("• {}", comp));
                                    if ui
                                        .small_button("x")
                                        .on_hover_text("Remove component")
                                        .clicked()
                                    {
                                        component_events.push(PanelEvent::RemoveComponent {
                                            entity_id,
                                            component_type: comp.clone(),
                                        });
                                    }
                                });
                            }
                            if info.components.is_empty() {
                                ui.label("No components");
                            }

                            // Add component menu
                            ui.separator();
                            ui.menu_button("Add Component", |ui| {
                                let available = [
                                    "Sprite",
                                    "Collider",
                                    "RigidBody",
                                    "Script",
                                    "MovementScript",
                                    "Audio",
                                    "Light",
                                    "Camera",
                                    "Particle",
                                ];
                                for comp_type in available {
                                    if ui.button(comp_type).clicked() {
                                        component_events.push(PanelEvent::AddComponent {
                                            entity_id,
                                            component_type: comp_type.to_string(),
                                        });
                                        ui.close();
                                    }
                                }
                            });
                        } else {
                            ui.label("• Transform");
                        }
                    });

                    // Emit component events
                    for event in component_events {
                        self.emit_event(event);
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No entity selected");
                    });
                }
            }
            PanelType::Hierarchy => {
                // Count filtered entities for header
                let search_lower = self.hierarchy_search.to_lowercase();
                let visible_count = if search_lower.is_empty() {
                    self.entity_list.len()
                } else {
                    self.entity_list
                        .iter()
                        .filter(|e| e.name.to_lowercase().contains(&search_lower))
                        .count()
                };
                let total_count = self.entity_list.len();

                ui.horizontal(|ui| {
                    if visible_count == total_count {
                        ui.heading(format!("Hierarchy ({})", total_count));
                    } else {
                        ui.heading(format!("Hierarchy ({}/{})", visible_count, total_count));
                    }
                    if let Some(entity_id) = self.selected_entity {
                        ui.weak(format!("• Selected: {}", entity_id));
                    }
                });
                ui.separator();

                // Toolbar row with create options
                ui.horizontal(|ui| {
                    // Create dropdown with entity types
                    ui.menu_button("+ Add", |ui| {
                        ui.label(egui::RichText::new("Game Objects").strong().small());
                        if ui
                            .button("  Empty Entity")
                            .on_hover_text("Create a blank entity")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::CreateEntity);
                            ui.close();
                        }
                        if ui
                            .button("  Player")
                            .on_hover_text("Spawn player entity (HP: 100, Team: 0)")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "Player".into(),
                            });
                            ui.close();
                        }
                        if ui
                            .button("  Companion")
                            .on_hover_text("Spawn companion entity (HP: 80, Team: 0)")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "Companion".into(),
                            });
                            ui.close();
                        }
                        if ui
                            .button("  Enemy")
                            .on_hover_text("Spawn enemy entity (HP: 50, Team: 1)")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "Enemy".into(),
                            });
                            ui.close();
                        }
                        if ui
                            .button("  Boss")
                            .on_hover_text("Spawn boss entity (HP: 500, Team: 1)")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "Boss".into(),
                            });
                            ui.close();
                        }
                        if ui
                            .button("  NPC")
                            .on_hover_text("Spawn NPC entity (HP: 100, Team: 2)")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "NPC".into(),
                            });
                            ui.close();
                        }
                        if ui
                            .button("  Prop")
                            .on_hover_text("Spawn a static prop entity")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "Prop".into(),
                            });
                            ui.close();
                        }

                        ui.separator();
                        ui.label(egui::RichText::new("Environment").strong().small());
                        if ui
                            .button("  Light")
                            .on_hover_text("Add a point light to the scene")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "Light".into(),
                            });
                            ui.close();
                        }
                        if ui
                            .button("  Camera")
                            .on_hover_text("Add a camera to the scene")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "Camera".into(),
                            });
                            ui.close();
                        }
                        if ui
                            .button("  Trigger")
                            .on_hover_text("Add a trigger volume")
                            .clicked()
                        {
                            self.emit_event(PanelEvent::SpawnArchetype {
                                archetype: "Trigger".into(),
                            });
                            ui.close();
                        }

                        if !self.spawnable_models.is_empty() {
                            ui.separator();
                            ui.label(egui::RichText::new("3D Models (Kenney)").strong().small());
                            let models: Vec<_> = self.spawnable_models.clone();
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    for (name, path) in &models {
                                        if ui.button(format!("  🎲 {}", name)).clicked() {
                                            self.emit_event(PanelEvent::SpawnModel {
                                                name: name.clone(),
                                                path: path.clone(),
                                            });
                                            ui.close();
                                        }
                                    }
                                });
                        }
                    });

                    // Quick filter buttons
                    ui.separator();
                    if ui
                        .small_button("All")
                        .on_hover_text("Show all entities")
                        .clicked()
                    {
                        self.hierarchy_search.clear();
                    }
                    if ui
                        .small_button("C")
                        .on_hover_text("Show only cameras")
                        .clicked()
                    {
                        self.hierarchy_search = "camera".to_string();
                    }
                    if ui
                        .small_button("L")
                        .on_hover_text("Show only lights")
                        .clicked()
                    {
                        self.hierarchy_search = "light".to_string();
                    }
                });

                // Search bar
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 30.0, 20.0],
                        egui::TextEdit::singleline(&mut self.hierarchy_search)
                            .hint_text("Search entities..."),
                    );
                    if ui.small_button("✕").on_hover_text("Clear search").clicked() {
                        self.hierarchy_search.clear();
                    }
                });
                ui.separator();

                // ── Static split: hierarchy (top half) / entity library (bottom half) ──
                // Reserve space for each section proportionally so the library
                // never gets pushed off-screen by a growing hierarchy.
                let available_h = ui.available_height();
                let hierarchy_h = (available_h * 0.45).max(120.0);
                let library_h = (available_h - hierarchy_h - 8.0).max(120.0);

                // ═══════════════════ HIERARCHY SECTION ═══════════════════
                // Collect clicked entity before iterating
                let mut clicked_entity = None;
                let mut entity_to_delete = None;
                let mut entity_to_duplicate = None;
                let mut entity_to_rename = None;
                let mut entity_to_focus = None;

                egui::ScrollArea::vertical()
                    .id_salt("hierarchy_scroll")
                    .max_height(hierarchy_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Show terrain group if chunks exist
                        if self.terrain_chunk_count > 0 {
                            let header =
                                format!("\u{1F3D4} Terrain ({} chunks)", self.terrain_chunk_count);
                            egui::CollapsingHeader::new(header)
                                .default_open(false)
                                .show(ui, |ui| {
                                    for i in 0..self.terrain_chunk_count {
                                        ui.label(format!("  Chunk {i}"));
                                    }
                                });
                        }

                        if self.entity_list.is_empty() && self.terrain_chunk_count == 0 {
                            ui.add_space(20.0);
                            ui.vertical_centered(|ui| {
                                ui.heading("Scene is Empty");
                                ui.add_space(12.0);
                                ui.label("Get started by adding entities to your scene:");
                                ui.add_space(8.0);

                                let btn_width = 180.0;
                                if ui
                                    .add_sized([btn_width, 28.0], egui::Button::new("+ Add Player"))
                                    .clicked()
                                {
                                    self.emit_event(PanelEvent::SpawnArchetype {
                                        archetype: "Player".into(),
                                    });
                                }
                                ui.add_space(2.0);
                                if ui
                                    .add_sized([btn_width, 28.0], egui::Button::new("+ Add Light"))
                                    .clicked()
                                {
                                    self.emit_event(PanelEvent::SpawnArchetype {
                                        archetype: "Light".into(),
                                    });
                                }
                                ui.add_space(2.0);
                                if ui
                                    .add_sized([btn_width, 28.0], egui::Button::new("+ Add Camera"))
                                    .clicked()
                                {
                                    self.emit_event(PanelEvent::SpawnArchetype {
                                        archetype: "Camera".into(),
                                    });
                                }
                                ui.add_space(2.0);
                                if ui
                                    .add_sized(
                                        [btn_width, 28.0],
                                        egui::Button::new("+ Empty Entity"),
                                    )
                                    .clicked()
                                {
                                    self.emit_event(PanelEvent::CreateEntity);
                                }

                                ui.add_space(16.0);
                                ui.separator();
                                ui.add_space(8.0);
                                ui.weak("Or use the + Add dropdown above for more options.");
                                ui.add_space(4.0);
                                ui.weak("Ctrl+S to save  |  Ctrl+O to open  |  F5 to play");
                            });
                        } else {
                            let search_lower = self.hierarchy_search.to_lowercase();
                            let filtered: Vec<_> = self
                                .entity_list
                                .iter()
                                .filter(|e| {
                                    search_lower.is_empty()
                                        || e.name.to_lowercase().contains(&search_lower)
                                })
                                .collect();

                            if filtered.is_empty() && !self.hierarchy_search.is_empty() {
                                ui.centered_and_justified(|ui| {
                                    ui.weak(format!(
                                        "No entities matching \"{}\"",
                                        self.hierarchy_search
                                    ));
                                });
                            }

                            // Entity count summary
                            if !search_lower.is_empty() {
                                ui.weak(format!(
                                    "Showing {} of {} entities",
                                    filtered.len(),
                                    self.entity_list.len()
                                ));
                                ui.add_space(4.0);
                            }

                            for entity in filtered {
                                let selected = self.selected_entity == Some(entity.id);

                                // Determine entity icon based on name patterns (cache lowercase)
                                let name_lower = entity.name.to_lowercase();
                                let icon = if name_lower.contains("light") {
                                    "[L]"
                                } else if name_lower.contains("camera") {
                                    "[C]"
                                } else if name_lower.contains("player") {
                                    "[P]"
                                } else if name_lower.contains("enemy") {
                                    "[E]"
                                } else if name_lower.contains("group")
                                    || name_lower.contains("folder")
                                {
                                    "[G]"
                                } else {
                                    "[.]"
                                };

                                let label = format!("{} {} ({})", icon, entity.name, entity.id);

                                ui.horizontal(|ui| {
                                    // Drag indicator (visual only)
                                    ui.weak("⋮⋮");

                                    // Entity button with selection highlight
                                    let response = ui.selectable_label(selected, &label);
                                    if response.clicked() {
                                        clicked_entity = Some(entity.id);
                                    }

                                    // Double-click to rename (just emit event)
                                    if response.double_clicked() {
                                        entity_to_rename = Some(entity.id);
                                    }

                                    // Context menu
                                    response.context_menu(|ui| {
                                        if ui.button("Rename").clicked() {
                                            entity_to_rename = Some(entity.id);
                                            ui.close();
                                        }
                                        if ui.button("Duplicate").clicked() {
                                            entity_to_duplicate = Some(entity.id);
                                            ui.close();
                                        }
                                        ui.separator();
                                        if ui.button("Focus in Viewport").clicked() {
                                            entity_to_focus = Some(entity.id);
                                            ui.close();
                                        }
                                        ui.separator();
                                        if ui.button("Delete").clicked() {
                                            entity_to_delete = Some(entity.id);
                                            ui.close();
                                        }
                                    });

                                    // Alternative menu button
                                    ui.menu_button("⋮", |ui| {
                                        if ui.button("Duplicate").clicked() {
                                            entity_to_duplicate = Some(entity.id);
                                            ui.close();
                                        }
                                        if ui.button("Delete").clicked() {
                                            entity_to_delete = Some(entity.id);
                                            ui.close();
                                        }
                                    });
                                });
                            }
                        }
                    });

                // Handle entity events outside the borrow
                if let Some(entity_id) = clicked_entity {
                    self.selected_entity = Some(entity_id);
                    self.emit_event(PanelEvent::EntitySelected(entity_id));
                }
                if let Some(entity_id) = entity_to_duplicate {
                    self.emit_event(PanelEvent::DuplicateEntity(entity_id));
                }
                if let Some(entity_id) = entity_to_delete {
                    if self.selected_entity == Some(entity_id) {
                        self.selected_entity = None;
                    }
                    self.emit_event(PanelEvent::DeleteEntity(entity_id));
                }
                if let Some(entity_id) = entity_to_rename {
                    // Emit rename event - the rename UI is handled by the entity
                    // panel when it receives this event
                    self.emit_event(PanelEvent::EntityRenamed {
                        entity_id,
                        new_name: String::new(), // Empty signals "start rename mode"
                    });
                }
                if let Some(entity_id) = entity_to_focus {
                    self.selected_entity = Some(entity_id);
                    self.emit_event(PanelEvent::EntitySelected(entity_id));
                    self.emit_event(PanelEvent::ViewportFocusOnSelection);
                }

                // ═══════════════════ ENTITY LIBRARY SECTION ═══════════════════
                ui.add_space(4.0);
                ui.separator();
                let catalog_count = self.entity_catalog.entry_count();
                ui.horizontal(|ui| {
                    ui.strong(format!("Entity Library ({catalog_count})"));
                });

                egui::ScrollArea::vertical()
                    .id_salt("entity_library_scroll")
                    .max_height(library_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let ctx = ui.ctx().clone();
                        self.entity_catalog.show(ui, &ctx);
                    });

                // Drain catalog spawn events → PanelEvent::SpawnModel
                let spawns = self.entity_catalog.take_spawns();
                for (name, path) in spawns {
                    self.emit_event(PanelEvent::SpawnModel { name, path });
                }
            }
            PanelType::Console => {
                // Count errors, warnings, info for header badges
                let error_count = self.console_error_count;
                let warn_count = self.console_warn_count;
                let info_count = self
                    .console_logs
                    .len()
                    .saturating_sub(error_count)
                    .saturating_sub(warn_count);

                ui.horizontal(|ui| {
                    ui.heading(format!("Console ({})", self.console_logs.len()));
                    ui.separator();
                    // Message count badges
                    if error_count > 0 {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 100, 100),
                            format!("{}", error_count),
                        );
                    }
                    if warn_count > 0 {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 200, 100),
                            format!("W: {}", warn_count),
                        );
                    }
                    if info_count > 0 {
                        ui.weak(format!("i: {}", info_count));
                    }
                });
                ui.separator();

                // Console controls - first row
                let mut should_clear = false;
                let mut copy_all = false;
                let mut copy_filtered = false;
                ui.horizontal(|ui| {
                    if ui.button("Clear").on_hover_text("Clear all logs").clicked() {
                        should_clear = true;
                    }
                    if ui
                        .button("Copy All")
                        .on_hover_text("Copy all logs to clipboard")
                        .clicked()
                    {
                        copy_all = true;
                    }
                    if ui
                        .button("Copy Filtered")
                        .on_hover_text("Copy only filtered logs")
                        .clicked()
                    {
                        copy_filtered = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak(format!("Lines: {}", self.console_logs.len()));
                    });
                });

                // Console controls - second row (filters)
                ui.horizontal(|ui| {
                    // Log level filters
                    ui.toggle_value(&mut self.show_errors, "Errors");
                    ui.toggle_value(&mut self.show_warnings, "Warnings");
                    ui.toggle_value(&mut self.show_info, "Info");
                    ui.separator();
                    ui.add_sized(
                        [ui.available_width(), 20.0],
                        egui::TextEdit::singleline(&mut self.console_search).hint_text("Filter..."),
                    );
                });

                // Handle actions outside borrow
                if should_clear {
                    self.console_logs.clear();
                    self.console_error_count = 0;
                    self.console_warn_count = 0;
                    self.emit_event(PanelEvent::ConsoleCleared);
                }

                // Handle copy actions
                if copy_all || copy_filtered {
                    let logs_to_copy: Vec<&String> = if copy_all {
                        self.console_logs.iter().collect()
                    } else {
                        let search_lower = self.console_search.to_lowercase();
                        self.console_logs
                            .iter()
                            .filter(|log| {
                                // Filter by search text
                                if !search_lower.is_empty()
                                    && !log.to_lowercase().contains(&search_lower)
                                {
                                    return false;
                                }
                                // Filter by log level
                                let is_error = log.contains("[ERROR]");
                                let is_warn = log.contains("[WARN]");
                                let is_info = !is_error && !is_warn;

                                if is_error && !self.show_errors {
                                    return false;
                                }
                                if is_warn && !self.show_warnings {
                                    return false;
                                }
                                if is_info && !self.show_info {
                                    return false;
                                }
                                true
                            })
                            .collect()
                    };
                    let text = logs_to_copy
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.ctx().copy_text(text);
                }
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.console_logs.is_empty() {
                            ui.label("[INFO] Console ready");
                        } else {
                            let search_lower = self.console_search.to_lowercase();
                            for log in &self.console_logs {
                                // Filter by search text
                                if !search_lower.is_empty()
                                    && !log.to_lowercase().contains(&search_lower)
                                {
                                    continue;
                                }

                                // Filter by log level
                                let is_error = log.contains("[ERROR]");
                                let is_warn = log.contains("[WARN]");
                                let is_info = !is_error && !is_warn;

                                if is_error && !self.show_errors {
                                    continue;
                                }
                                if is_warn && !self.show_warnings {
                                    continue;
                                }
                                if is_info && !self.show_info {
                                    continue;
                                }

                                // Color-code based on level
                                let color = if is_error {
                                    egui::Color32::from_rgb(255, 100, 100)
                                } else if is_warn {
                                    egui::Color32::from_rgb(255, 200, 100)
                                } else {
                                    egui::Color32::GRAY
                                };
                                ui.colored_label(color, log);
                            }
                        }
                    });
            }
            PanelType::AssetBrowser => {
                self.asset_browser_panel.show(ui);
                // Actions are collected via take_asset_browser_actions()
            }
            PanelType::Profiler => {
                let stats = &self.runtime_stats;

                // Enhanced header with tick count and runtime state
                let state_indicator = if stats.is_playing {
                    if stats.is_paused {
                        "||"
                    } else {
                        ">"
                    }
                } else {
                    "--"
                };
                ui.heading(format!(
                    "Profiler {} (Tick: {})",
                    state_indicator, stats.tick_count
                ));
                ui.separator();

                // Runtime state indicator
                ui.horizontal(|ui| {
                    if stats.is_playing {
                        if stats.is_paused {
                            ui.colored_label(egui::Color32::YELLOW, "Paused");
                        } else {
                            ui.colored_label(egui::Color32::GREEN, "Running");
                        }
                    } else {
                        ui.label("Editing");
                    }
                    ui.separator();
                    ui.label(format!("Tick: {}", stats.tick_count));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Copy").on_hover_text("Copy stats to clipboard").clicked() {
                            let report = format!(
                                "Frame: {:.2}ms | FPS: {:.1} | Draw Calls: {} | Triangles: {} | Entities: {}",
                                stats.frame_time_ms, stats.fps, stats.draw_calls, stats.triangles, stats.entity_count
                            );
                            ui.ctx().copy_text(report);
                        }
                    });
                });
                ui.separator();

                // Frame time with color coding and performance grade
                let (fps_color, grade) = if stats.fps >= 60.0 {
                    (egui::Color32::GREEN, "A+")
                } else if stats.fps >= 55.0 {
                    (egui::Color32::GREEN, "A")
                } else if stats.fps >= 45.0 {
                    (egui::Color32::YELLOW, "B")
                } else if stats.fps >= 30.0 {
                    (egui::Color32::YELLOW, "C")
                } else {
                    (egui::Color32::RED, "D")
                };

                // Main metrics with grade
                ui.horizontal(|ui| {
                    ui.strong("Performance:");
                    ui.colored_label(fps_color, grade);
                });

                ui.columns(2, |cols| {
                    cols[0].horizontal(|ui| {
                        ui.label("Frame:");
                        ui.colored_label(fps_color, format!("{:.2}ms", stats.frame_time_ms));
                    });
                    cols[1].horizontal(|ui| {
                        ui.label("FPS:");
                        ui.colored_label(fps_color, format!("{:.1}", stats.fps));
                    });
                });

                // Frame budget bar
                let budget_pct = (stats.frame_time_ms / 16.67).clamp(0.0, 2.0);
                ui.horizontal(|ui| {
                    ui.label("Budget:");
                    let bar = egui::ProgressBar::new((budget_pct / 2.0).min(1.0))
                        .text(format!("{:.0}%", budget_pct * 100.0))
                        .fill(if budget_pct <= 1.0 {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::RED
                        });
                    ui.add_sized([ui.available_width() - 60.0, 14.0], bar);
                });

                ui.add_space(4.0);

                // Subsystem timing breakdown
                ui.collapsing("Subsystem Timing", |ui| {
                    let total = stats.render_time_ms
                        + stats.physics_time_ms
                        + stats.ai_time_ms
                        + stats.script_time_ms
                        + stats.audio_time_ms;
                    let bar_width = ui.available_width().min(200.0);

                    // Helper to draw timing bar
                    let draw_timing =
                        |ui: &mut egui::Ui, label: &str, time_ms: f32, color: egui::Color32| {
                            ui.horizontal(|ui| {
                                ui.label(format!("{:12}", label));
                                let pct = if total > 0.0 { time_ms / total } else { 0.0 };
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(bar_width * 0.5, 12.0),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter();
                                painter.rect_filled(rect, 2.0, egui::Color32::from_gray(40));
                                let fill_rect = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(rect.width() * pct, rect.height()),
                                );
                                painter.rect_filled(fill_rect, 2.0, color);
                                ui.label(format!("{:.2}ms ({:.0}%)", time_ms, pct * 100.0));
                            });
                        };

                    draw_timing(
                        ui,
                        "Render",
                        stats.render_time_ms,
                        egui::Color32::from_rgb(100, 150, 255),
                    );
                    draw_timing(
                        ui,
                        "Physics",
                        stats.physics_time_ms,
                        egui::Color32::from_rgb(255, 150, 100),
                    );
                    draw_timing(
                        ui,
                        "AI",
                        stats.ai_time_ms,
                        egui::Color32::from_rgb(100, 255, 150),
                    );
                    draw_timing(
                        ui,
                        "Scripts",
                        stats.script_time_ms,
                        egui::Color32::from_rgb(255, 255, 100),
                    );
                    draw_timing(
                        ui,
                        "Audio",
                        stats.audio_time_ms,
                        egui::Color32::from_rgb(200, 150, 255),
                    );

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Total subsystems:");
                        ui.strong(format!("{:.2}ms", total));
                    });
                });

                // GPU Statistics
                ui.collapsing("GPU Stats", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Draw Calls:");
                        ui.strong(format!("{}", stats.draw_calls));
                        if stats.draw_calls > 1000 {
                            ui.colored_label(egui::Color32::YELLOW, "!");
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Triangles:");
                        let tri_str = if stats.triangles >= 1_000_000 {
                            format!("{:.2}M", stats.triangles as f64 / 1_000_000.0)
                        } else if stats.triangles >= 1000 {
                            format!("{:.1}K", stats.triangles as f64 / 1000.0)
                        } else {
                            format!("{}", stats.triangles)
                        };
                        ui.strong(tri_str);
                    });
                    let gpu_mem = if stats.gpu_memory_bytes >= 1_000_000_000 {
                        format!("{:.2} GB", stats.gpu_memory_bytes as f64 / 1_000_000_000.0)
                    } else if stats.gpu_memory_bytes >= 1_000_000 {
                        format!("{:.2} MB", stats.gpu_memory_bytes as f64 / 1_000_000.0)
                    } else {
                        format!("{} KB", stats.gpu_memory_bytes / 1000)
                    };
                    ui.horizontal(|ui| {
                        ui.label("GPU Memory:");
                        ui.label(gpu_mem);
                    });

                    // Batching efficiency
                    if stats.draw_calls > 0 {
                        let tris_per_call = stats.triangles as f64 / stats.draw_calls as f64;
                        let efficiency = if tris_per_call >= 1000.0 {
                            "Good"
                        } else if tris_per_call >= 100.0 {
                            "Fair"
                        } else {
                            "Poor"
                        };
                        ui.horizontal(|ui| {
                            ui.label("Batching:");
                            ui.label(format!("{} ({:.0} tris/call)", efficiency, tris_per_call));
                        });
                    }
                });

                // Memory Statistics
                ui.collapsing("Memory Stats", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Entities:");
                        ui.label(format!("{}", stats.entity_count));
                    });
                    // Estimated memory (rough calculation)
                    let est_mem_mb = (stats.entity_count * 256) as f64 / 1_000_000.0; // ~256 bytes per entity estimate
                    ui.horizontal(|ui| {
                        ui.label("Est. Entity Memory:");
                        ui.label(format!("{:.2} MB", est_mem_mb));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Components/Entity:");
                        let avg = if stats.entity_count > 0 {
                            "varies"
                        } else {
                            "0"
                        };
                        ui.label(avg);
                    });
                });

                // Frame time graph (custom painted)
                if !self.frame_time_history.is_empty() {
                    ui.add_space(8.0);
                    ui.collapsing("Frame History", |ui| {
                        let max_time = self
                            .frame_time_history
                            .iter()
                            .cloned()
                            .fold(16.67f32, f32::max);
                        let avg_time: f32 = self.frame_time_history.iter().sum::<f32>()
                            / self.frame_time_history.len() as f32;
                        let min_time = self
                            .frame_time_history
                            .iter()
                            .cloned()
                            .fold(f32::MAX, f32::min);
                        let max_recorded = self
                            .frame_time_history
                            .iter()
                            .cloned()
                            .fold(0.0f32, f32::max);

                        // Stats line
                        ui.horizontal(|ui| {
                            ui.label("Min:");
                            ui.weak(format!("{:.2}ms", min_time));
                            ui.separator();
                            ui.label("Avg:");
                            ui.weak(format!("{:.2}ms", avg_time));
                            ui.separator();
                            ui.label("Max:");
                            ui.weak(format!("{:.2}ms", max_recorded));
                        });

                        // Draw a simple bar graph
                        let (rect, _response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width().min(300.0), 80.0),
                            egui::Sense::hover(),
                        );

                        if ui.is_rect_visible(rect) {
                            let painter = ui.painter();
                            painter.rect_filled(rect, 2.0, egui::Color32::from_gray(30));

                            // Draw 16.67ms target line
                            let target_y = rect.bottom() - (16.67 / max_time) * rect.height();
                            painter.line_segment(
                                [
                                    egui::pos2(rect.left(), target_y),
                                    egui::pos2(rect.right(), target_y),
                                ],
                                egui::Stroke::new(
                                    1.0,
                                    egui::Color32::from_rgb(100, 255, 100).gamma_multiply(0.5),
                                ),
                            );

                            // Draw average line
                            let avg_y = rect.bottom() - (avg_time / max_time) * rect.height();
                            painter.line_segment(
                                [
                                    egui::pos2(rect.left(), avg_y),
                                    egui::pos2(rect.right(), avg_y),
                                ],
                                egui::Stroke::new(
                                    1.0,
                                    egui::Color32::from_rgb(255, 200, 100).gamma_multiply(0.5),
                                ),
                            );

                            let bar_width = rect.width() / self.frame_time_history.len() as f32;
                            for (i, &time) in self.frame_time_history.iter().enumerate() {
                                let height = (time / max_time) * rect.height();
                                let x = rect.left() + i as f32 * bar_width;
                                let bar_rect = egui::Rect::from_min_max(
                                    egui::pos2(x, rect.bottom() - height),
                                    egui::pos2(x + bar_width - 1.0, rect.bottom()),
                                );
                                let color = if time <= 16.67 {
                                    egui::Color32::from_rgb(100, 200, 100)
                                } else if time <= 33.33 {
                                    egui::Color32::from_rgb(255, 200, 100)
                                } else {
                                    egui::Color32::from_rgb(255, 100, 100)
                                };
                                painter.rect_filled(bar_rect, 0.0, color);
                            }

                            // Legend
                            painter.text(
                                egui::pos2(rect.right() - 60.0, target_y - 2.0),
                                egui::Align2::RIGHT_BOTTOM,
                                "60 FPS",
                                egui::FontId::proportional(9.0),
                                egui::Color32::from_rgb(100, 255, 100),
                            );
                        }

                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "█");
                            ui.weak("< 16.67ms");
                            ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "█");
                            ui.weak("< 33.33ms");
                            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "█");
                            ui.weak("> 33.33ms");
                        });
                    });
                }

                // Play Session Stats (visible only when playing or just paused)
                if self.play_session_start.is_some() {
                    ui.add_space(8.0);
                    ui.collapsing("Play Session", |ui| {
                        // Duration
                        let elapsed = self
                            .play_session_start
                            .map(|s| s.elapsed())
                            .unwrap_or_default();
                        let secs = elapsed.as_secs();
                        let duration_str = if secs >= 3600 {
                            format!(
                                "{}h {:02}m {:02}s",
                                secs / 3600,
                                (secs % 3600) / 60,
                                secs % 60
                            )
                        } else if secs >= 60 {
                            format!("{}m {:02}s", secs / 60, secs % 60)
                        } else {
                            format!("{}s", secs)
                        };
                        ui.horizontal(|ui| {
                            ui.label("Duration:");
                            ui.strong(&duration_str);
                        });

                        // Average FPS
                        let avg_frame_ms = if self.play_total_frames > 0 {
                            self.play_total_frame_ms / self.play_total_frames as f64
                        } else {
                            0.0
                        };
                        let avg_fps = if avg_frame_ms > 0.0 {
                            1000.0 / avg_frame_ms
                        } else {
                            0.0
                        };
                        ui.horizontal(|ui| {
                            ui.label("Avg FPS:");
                            let color = if avg_fps >= 60.0 {
                                egui::Color32::GREEN
                            } else if avg_fps >= 30.0 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(color, format!("{:.1}", avg_fps));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Avg Frame:");
                            ui.strong(format!("{:.2}ms", avg_frame_ms));
                        });

                        // Peak frame time
                        ui.horizontal(|ui| {
                            ui.label("Peak Frame:");
                            let peak_color = if self.play_peak_frame_ms <= 16.67 {
                                egui::Color32::GREEN
                            } else if self.play_peak_frame_ms <= 33.33 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(
                                peak_color,
                                format!("{:.2}ms", self.play_peak_frame_ms),
                            );
                        });

                        // Budget violations
                        let violation_pct = if self.play_total_frames > 0 {
                            (self.play_budget_violations as f64 / self.play_total_frames as f64)
                                * 100.0
                        } else {
                            0.0
                        };
                        ui.horizontal(|ui| {
                            ui.label("Budget Violations:");
                            let violation_color = if violation_pct < 1.0 {
                                egui::Color32::GREEN
                            } else if violation_pct < 5.0 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(
                                violation_color,
                                format!("{} ({:.1}%)", self.play_budget_violations, violation_pct),
                            );
                        });

                        // Frames counted
                        ui.horizontal(|ui| {
                            ui.label("Frames:");
                            ui.weak(format!("{}", self.play_total_frames));
                        });

                        // Stability grade
                        let stability = if violation_pct < 1.0 {
                            ("Excellent", egui::Color32::GREEN)
                        } else if violation_pct < 5.0 {
                            ("Good", egui::Color32::from_rgb(150, 255, 100))
                        } else if violation_pct < 15.0 {
                            ("Fair", egui::Color32::YELLOW)
                        } else {
                            ("Poor", egui::Color32::RED)
                        };
                        ui.horizontal(|ui| {
                            ui.label("Stability:");
                            ui.colored_label(stability.1, stability.0);
                        });
                    });
                }
            }
            PanelType::SceneStats => {
                // Clone the data we need to avoid borrow conflicts
                let stats = self.scene_stats.clone();
                let undo_count = self.undo_count;
                let redo_count = self.redo_count;
                let mut refresh_requested = false;

                // Header with entity count and modified indicator
                ui.horizontal(|ui| {
                    ui.heading(format!("Scene Stats ({} entities)", stats.total_entities));
                    if stats.is_modified {
                        ui.colored_label(egui::Color32::YELLOW, " ● Modified");
                    } else {
                        ui.colored_label(egui::Color32::GREEN, " Saved");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Undo/Redo indicators
                        if undo_count > 0 || redo_count > 0 {
                            ui.small(format!("U:{} R:{}", undo_count, redo_count));
                        }
                        if ui
                            .small_button("R")
                            .on_hover_text("Refresh statistics")
                            .clicked()
                        {
                            refresh_requested = true;
                        }
                    });
                });

                // Emit event after closure ends
                if refresh_requested {
                    self.emit_event(PanelEvent::RefreshSceneStats);
                }

                // Scene file info
                if let Some(ref path) = stats.scene_path {
                    ui.small(format!("{}", path));
                    if let Some(ref time) = stats.last_save_time {
                        ui.small(format!("Last saved: {}", time));
                    }
                }
                ui.separator();

                // Summary bar
                ui.horizontal(|ui| {
                    ui.strong(format!("{}", stats.total_entities));
                    ui.label("entities |");
                    ui.strong(format!("{}", stats.total_components));
                    ui.label("components");
                });

                ui.add_space(4.0);

                // Entity Statistics
                ui.collapsing("Entities", |ui| {
                    let stat_row = |ui: &mut egui::Ui, label: &str, value: usize| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", label));
                            ui.strong(format!("{}", value));
                        });
                    };

                    stat_row(ui, "Total", stats.total_entities);
                    stat_row(ui, "Selected", stats.selected_count);
                    stat_row(ui, "Prefab Instances", stats.prefab_instances);
                });

                // Component Statistics with categories
                ui.collapsing("Components", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Total:");
                        ui.strong(format!("{}", stats.total_components));
                    });

                    ui.add_space(4.0);
                    ui.label("By Category:");

                    // Render components
                    egui::Grid::new("component_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Mesh Renderers:");
                            ui.label(format!("{}", stats.mesh_count));
                            ui.end_row();

                            ui.label("Lights:");
                            ui.label(format!("{}", stats.light_count));
                            ui.end_row();

                            ui.label("Cameras:");
                            ui.label(format!("{}", stats.camera_count));
                            ui.end_row();

                            ui.label("Particle Systems:");
                            ui.label(format!("{}", stats.particle_systems));
                            ui.end_row();

                            ui.label("Physics Bodies:");
                            ui.label(format!("{}", stats.physics_bodies));
                            ui.end_row();

                            ui.label("Colliders:");
                            ui.label(format!("{}", stats.collider_count));
                            ui.end_row();

                            ui.label("Audio Sources:");
                            ui.label(format!("{}", stats.audio_sources));
                            ui.end_row();

                            ui.label("Scripts:");
                            ui.label(format!("{}", stats.script_count));
                            ui.end_row();

                            ui.label("UI Elements:");
                            ui.label(format!("{}", stats.ui_element_count));
                            ui.end_row();
                        });
                });

                // System Statistics
                ui.collapsing("Systems & Resources", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Active Systems:");
                        ui.strong(format!("{}", stats.active_systems));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Loaded Assets:");
                        ui.strong(format!("{}", stats.loaded_assets));
                    });

                    // Memory usage with bar
                    let memory_str = if stats.memory_usage_bytes >= 1_000_000_000 {
                        format!(
                            "{:.2} GB",
                            stats.memory_usage_bytes as f64 / 1_000_000_000.0
                        )
                    } else if stats.memory_usage_bytes >= 1_000_000 {
                        format!("{:.2} MB", stats.memory_usage_bytes as f64 / 1_000_000.0)
                    } else if stats.memory_usage_bytes >= 1_000 {
                        format!("{:.2} KB", stats.memory_usage_bytes as f64 / 1_000.0)
                    } else {
                        format!("{} B", stats.memory_usage_bytes)
                    };

                    ui.add_space(4.0);
                    ui.label("Memory Usage:");
                    let mem_frac = (stats.memory_usage_bytes as f32 / 1_000_000_000.0).min(1.0); // vs 1GB
                    let mem_color = if mem_frac < 0.5 {
                        egui::Color32::GREEN
                    } else if mem_frac < 0.8 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.add(
                        egui::ProgressBar::new(mem_frac)
                            .text(memory_str)
                            .fill(mem_color),
                    );
                });

                ui.add_space(8.0);
                ui.separator();

                // Undo/Redo status
                ui.horizontal(|ui| {
                    ui.label("Undo Stack:");
                    ui.strong(format!("{}", self.undo_count));
                    ui.label("|");
                    ui.label("Redo Stack:");
                    ui.strong(format!("{}", self.redo_count));
                });
            }
            PanelType::Transform => {
                // Enhanced header with selected entity info
                let header = if let Some(id) = self.selected_entity {
                    if let Some(ref info) = self.selected_entity_info {
                        format!("Transform - {} (ID: {})", info.name, id)
                    } else {
                        format!("Transform (ID: {})", id)
                    }
                } else {
                    "Transform".to_string()
                };
                ui.heading(header);
                ui.separator();

                let mut transform_events: Vec<PanelEvent> = Vec::new();
                let entity_id = self.selected_entity.unwrap_or(0);

                if let Some((
                    ref mut x,
                    ref mut y,
                    ref mut z,
                    ref mut rotation,
                    ref mut scale_x,
                    ref mut scale_y,
                    ref mut scale_z,
                )) = self.selected_transform
                {
                    let orig_x = *x;
                    let orig_y = *y;
                    let orig_z = *z;
                    let _orig_rot_rad = *rotation;
                    let orig_sx = *scale_x;
                    let orig_sy = *scale_y;
                    let orig_sz = *scale_z;

                    // Snap settings
                    ui.horizontal(|ui| {
                        ui.label("Snap:");
                        let snaps = [0.0, 0.1, 0.5, 1.0, 5.0, 10.0];
                        let snap_labels = ["Off", "0.1", "0.5", "1", "5", "10"];
                        for (i, &snap) in snaps.iter().enumerate() {
                            let is_current = (self.transform_snap_value - snap).abs() < 0.001;
                            if ui.selectable_label(is_current, snap_labels[i]).clicked() {
                                self.transform_snap_value = snap;
                            }
                        }
                    });
                    ui.separator();

                    // Position with increment buttons
                    ui.label("Position");
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        if ui.small_button("-").on_hover_text("Decrease X").clicked() {
                            let snap = if self.transform_snap_value > 0.0 {
                                self.transform_snap_value
                            } else {
                                1.0
                            };
                            *x -= snap;
                        }
                        ui.add(egui::DragValue::new(x).speed(0.1).min_decimals(2));
                        if ui.small_button("+").on_hover_text("Increase X").clicked() {
                            let snap = if self.transform_snap_value > 0.0 {
                                self.transform_snap_value
                            } else {
                                1.0
                            };
                            *x += snap;
                        }
                        ui.separator();
                        ui.label("Y:");
                        if ui.small_button("-").on_hover_text("Decrease Y").clicked() {
                            let snap = if self.transform_snap_value > 0.0 {
                                self.transform_snap_value
                            } else {
                                1.0
                            };
                            *y -= snap;
                        }
                        ui.add(egui::DragValue::new(y).speed(0.1).min_decimals(2));
                        if ui.small_button("+").on_hover_text("Increase Y").clicked() {
                            let snap = if self.transform_snap_value > 0.0 {
                                self.transform_snap_value
                            } else {
                                1.0
                            };
                            *y += snap;
                        }
                        ui.separator();
                        ui.label("Z:");
                        if ui.small_button("-").on_hover_text("Decrease Z").clicked() {
                            let snap = if self.transform_snap_value > 0.0 {
                                self.transform_snap_value
                            } else {
                                1.0
                            };
                            *z -= snap;
                        }
                        ui.add(egui::DragValue::new(z).speed(0.1).min_decimals(2));
                        if ui.small_button("+").on_hover_text("Increase Z").clicked() {
                            let snap = if self.transform_snap_value > 0.0 {
                                self.transform_snap_value
                            } else {
                                1.0
                            };
                            *z += snap;
                        }
                    });
                    ui.add_space(8.0);

                    // Rotation with quick angles
                    let mut rotation_degrees = rotation.to_degrees();
                    let orig_rot_degrees = rotation_degrees;
                    ui.label("Rotation");
                    ui.horizontal(|ui| {
                        ui.label("Z:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut rotation_degrees)
                                    .speed(1.0)
                                    .suffix("°"),
                            )
                            .changed()
                        {
                            *rotation = rotation_degrees.to_radians();
                        }
                        ui.separator();
                        // Quick rotation buttons
                        if ui.small_button("0°").clicked() {
                            rotation_degrees = 0.0;
                            *rotation = 0.0;
                        }
                        if ui.small_button("90°").clicked() {
                            rotation_degrees = 90.0;
                            *rotation = 90.0_f32.to_radians();
                        }
                        if ui.small_button("180°").clicked() {
                            rotation_degrees = 180.0;
                            *rotation = 180.0_f32.to_radians();
                        }
                        if ui.small_button("-90°").clicked() {
                            rotation_degrees = -90.0;
                            *rotation = (-90.0_f32).to_radians();
                        }
                    });
                    ui.add_space(8.0);

                    // Scale with uniform option
                    ui.label("⤢ Scale");
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        ui.add(
                            egui::DragValue::new(scale_x)
                                .speed(0.01)
                                .min_decimals(2)
                                .range(0.01..=100.0),
                        );
                        ui.label("Y:");
                        ui.add(
                            egui::DragValue::new(scale_y)
                                .speed(0.01)
                                .min_decimals(2)
                                .range(0.01..=100.0),
                        );
                        ui.label("Z:");
                        ui.add(
                            egui::DragValue::new(scale_z)
                                .speed(0.01)
                                .min_decimals(2)
                                .range(0.01..=100.0),
                        );
                        ui.separator();
                        // Uniform scale shortcut
                        if ui
                            .small_button("=")
                            .on_hover_text("Make uniform (Y=Z=X)")
                            .clicked()
                        {
                            *scale_y = *scale_x;
                            *scale_z = *scale_x;
                        }
                    });
                    // Scale presets
                    ui.horizontal(|ui| {
                        ui.label("Presets:");
                        if ui.small_button("0.5x").clicked() {
                            *scale_x = 0.5;
                            *scale_y = 0.5;
                            *scale_z = 0.5;
                        }
                        if ui.small_button("1x").clicked() {
                            *scale_x = 1.0;
                            *scale_y = 1.0;
                            *scale_z = 1.0;
                        }
                        if ui.small_button("2x").clicked() {
                            *scale_x = 2.0;
                            *scale_y = 2.0;
                            *scale_z = 2.0;
                        }
                        if ui.small_button("4x").clicked() {
                            *scale_x = 4.0;
                            *scale_y = 4.0;
                            *scale_z = 4.0;
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();

                    // Copy/Paste and Reset buttons
                    ui.horizontal(|ui| {
                        if ui
                            .button("Copy")
                            .on_hover_text("Copy transform to clipboard")
                            .clicked()
                        {
                            let transform_text = format!(
                                "pos({:.2},{:.2},{:.2}) rot({:.1}°) scale({:.2},{:.2},{:.2})",
                                *x, *y, *z, rotation_degrees, *scale_x, *scale_y, *scale_z
                            );
                            ui.ctx().copy_text(transform_text);
                        }
                        ui.separator();
                        if ui
                            .button("Reset Pos")
                            .on_hover_text("Reset position to origin")
                            .clicked()
                        {
                            *x = 0.0;
                            *y = 0.0;
                            *z = 0.0;
                            transform_events.push(PanelEvent::TransformPositionChanged {
                                entity_id,
                                x: 0.0,
                                y: 0.0,
                                z: 0.0,
                            });
                        }
                        if ui
                            .button("Reset Rot")
                            .on_hover_text("Reset rotation to 0°")
                            .clicked()
                        {
                            *rotation = 0.0;
                            transform_events.push(PanelEvent::TransformRotationChanged {
                                entity_id,
                                rotation: 0.0,
                            });
                        }
                        if ui
                            .button("Reset Scale")
                            .on_hover_text("Reset scale to 1x")
                            .clicked()
                        {
                            *scale_x = 1.0;
                            *scale_y = 1.0;
                            *scale_z = 1.0;
                            transform_events.push(PanelEvent::TransformScaleChanged {
                                entity_id,
                                scale_x: 1.0,
                                scale_y: 1.0,
                                scale_z: 1.0,
                            });
                        }
                    });

                    // Check for changes and emit events
                    if ((*x) - orig_x).abs() > 0.0001
                        || ((*y) - orig_y).abs() > 0.0001
                        || ((*z) - orig_z).abs() > 0.0001
                    {
                        transform_events.push(PanelEvent::TransformPositionChanged {
                            entity_id,
                            x: *x,
                            y: *y,
                            z: *z,
                        });
                    }
                    if (rotation_degrees - orig_rot_degrees).abs() > 0.0001 {
                        transform_events.push(PanelEvent::TransformRotationChanged {
                            entity_id,
                            rotation: *rotation,
                        });
                    }
                    if ((*scale_x) - orig_sx).abs() > 0.0001
                        || ((*scale_y) - orig_sy).abs() > 0.0001
                        || ((*scale_z) - orig_sz).abs() > 0.0001
                    {
                        transform_events.push(PanelEvent::TransformScaleChanged {
                            entity_id,
                            scale_x: *scale_x,
                            scale_y: *scale_y,
                            scale_z: *scale_z,
                        });
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.weak("No entity selected\n\nSelect an entity in the Hierarchy panel");
                    });
                }

                // Emit events outside borrow
                for event in transform_events {
                    self.emit_event(event);
                }
            }
            PanelType::World => {
                // Enhanced header with entity count
                let entity_count = self.entity_list.len();
                let sim_time = if self.runtime_stats.is_playing {
                    format!(" - {:.1}s", self.runtime_stats.tick_count as f32 / 60.0)
                } else {
                    String::new()
                };
                ui.heading(format!("World ({} entities{})", entity_count, sim_time));
                ui.separator();

                // Statistics section
                egui::CollapsingHeader::new("Statistics")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Entities:");
                            ui.strong(format!("{}", self.entity_list.len()));
                        });

                        if self.runtime_stats.is_playing {
                            ui.horizontal(|ui| {
                                ui.label("Simulation Time:");
                                ui.label(format!(
                                    "{:.2}s",
                                    self.runtime_stats.tick_count as f32 / 60.0
                                ));
                            });
                        }

                        // Additional statistics
                        ui.horizontal(|ui| {
                            ui.label("Active Components:");
                            ui.label(format!("{}", self.entity_list.len() * 3));
                            // Estimate
                        });
                    });

                ui.add_space(4.0);

                // Skybox section
                egui::CollapsingHeader::new("Skybox")
                    .default_open(true)
                    .show(ui, |ui| {
                        let skybox_presets = [
                            "Clear Sky",
                            "Overcast",
                            "Sunset",
                            "Night",
                            "Space",
                            "Gradient",
                        ];
                        ui.horizontal(|ui| {
                            ui.label("Preset:");
                            egui::ComboBox::from_id_salt("skybox_preset")
                                .selected_text(
                                    skybox_presets
                                        .get(self.world_skybox_preset)
                                        .copied()
                                        .unwrap_or("Clear Sky"),
                                )
                                .show_ui(ui, |ui| {
                                    for (i, name) in skybox_presets.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut self.world_skybox_preset,
                                            i,
                                            *name,
                                        );
                                    }
                                });
                        });

                        // Skybox preview (simple color representation)
                        let preview_colors = [
                            [0.5, 0.7, 1.0],    // Clear Sky - blue
                            [0.5, 0.5, 0.55],   // Overcast - gray
                            [0.9, 0.5, 0.3],    // Sunset - orange
                            [0.05, 0.05, 0.15], // Night - dark blue
                            [0.02, 0.02, 0.05], // Space - near black
                            [0.4, 0.5, 0.7],    // Gradient - blue-gray
                        ];
                        let preview_color = preview_colors
                            .get(self.world_skybox_preset)
                            .unwrap_or(&[0.5, 0.7, 1.0]);
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width() - 8.0, 30.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::same(4),
                            egui::Color32::from_rgb(
                                (preview_color[0] * 255.0) as u8,
                                (preview_color[1] * 255.0) as u8,
                                (preview_color[2] * 255.0) as u8,
                            ),
                        );

                        ui.add_space(4.0);

                        // HDRI file loader
                        ui.horizontal(|ui| {
                            ui.label("HDRI:");
                            if let Some(path) = &self.world_hdri_path {
                                let name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");
                                ui.label(name);
                                if ui
                                    .small_button("\u{2715}")
                                    .on_hover_text("Remove HDRI")
                                    .clicked()
                                {
                                    let was_some = self.world_hdri_path.is_some();
                                    self.world_hdri_path = None;
                                    if was_some {
                                        self.emit_event(PanelEvent::HdriCleared);
                                    }
                                }
                            } else {
                                ui.weak("None");
                            }
                        });
                        if ui.button("Load HDRI...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("HDR Images", &["hdr", "exr", "hdri"])
                                .add_filter("All Images", &["hdr", "exr", "hdri", "png", "jpg"])
                                .pick_file()
                            {
                                self.emit_event(PanelEvent::HdriLoaded { path: path.clone() });
                                self.world_hdri_path = Some(path);
                            }
                        }
                    });

                ui.add_space(4.0);

                // Time of Day section
                egui::CollapsingHeader::new("Time of Day")
                    .default_open(true)
                    .show(ui, |ui| {
                        // Time slider
                        let hours = self.world_time_of_day.floor() as u32;
                        let minutes = ((self.world_time_of_day.fract()) * 60.0) as u32;
                        ui.horizontal(|ui| {
                            ui.label("Time:");
                            ui.add(
                                egui::Slider::new(&mut self.world_time_of_day, 0.0..=24.0).text(""),
                            );
                            ui.label(format!("{:02}:{:02}", hours % 24, minutes));
                        });

                        // Time presets
                        ui.horizontal(|ui| {
                            if ui.small_button("Dawn").on_hover_text("06:00").clicked() {
                                self.world_time_of_day = 6.0;
                            }
                            if ui.small_button("Morning").on_hover_text("09:00").clicked() {
                                self.world_time_of_day = 9.0;
                            }
                            if ui.small_button("Noon").on_hover_text("12:00").clicked() {
                                self.world_time_of_day = 12.0;
                            }
                            if ui
                                .small_button("Afternoon")
                                .on_hover_text("15:00")
                                .clicked()
                            {
                                self.world_time_of_day = 15.0;
                            }
                            if ui.small_button("Dusk").on_hover_text("18:00").clicked() {
                                self.world_time_of_day = 18.0;
                            }
                            if ui.small_button("Night").on_hover_text("22:00").clicked() {
                                self.world_time_of_day = 22.0;
                            }
                        });

                        // Sun intensity based on time
                        let sun_intensity =
                            if self.world_time_of_day >= 6.0 && self.world_time_of_day <= 18.0 {
                                let mid_day = 12.0;
                                let diff = (self.world_time_of_day - mid_day).abs();
                                1.0 - (diff / 6.0) * 0.5 // 1.0 at noon, 0.5 at 6am/6pm
                            } else {
                                0.1 // Night
                            };
                        ui.horizontal(|ui| {
                            ui.label("Sun Intensity:");
                            ui.weak(format!("{:.0}%", sun_intensity * 100.0));
                        });
                    });

                ui.add_space(4.0);

                // Weather section
                egui::CollapsingHeader::new("Weather")
                    .default_open(true)
                    .show(ui, |ui| {
                        let weather_presets = [
                            "Clear",
                            "Cloudy",
                            "Rain",
                            "Storm",
                            "Snow",
                            "Fog",
                            "Sandstorm",
                        ];
                        let weather_icons = [
                            "\u{2600}",
                            "\u{2601}",
                            "\u{1F327}",
                            "\u{26C8}",
                            "\u{2744}",
                            "\u{1F32B}",
                            "\u{1F3DC}",
                        ];

                        ui.horizontal(|ui| {
                            ui.label("Weather:");
                            let current_weather = weather_presets
                                .get(self.world_weather_preset)
                                .unwrap_or(&"Clear");
                            let current_icon = weather_icons
                                .get(self.world_weather_preset)
                                .unwrap_or(&"Clear");
                            egui::ComboBox::from_id_salt("weather_preset")
                                .selected_text(format!("{} {}", current_icon, current_weather))
                                .show_ui(ui, |ui| {
                                    for (i, name) in weather_presets.iter().enumerate() {
                                        let icon = weather_icons.get(i).unwrap_or(&"Clear");
                                        ui.selectable_value(
                                            &mut self.world_weather_preset,
                                            i,
                                            format!("{} {}", icon, name),
                                        );
                                    }
                                });
                        });

                        // Weather effects info
                        let weather_effects = match self.world_weather_preset {
                            0 => "No weather effects",
                            1 => "Reduced visibility, diffuse lighting",
                            2 => "Rain particles, wet surfaces",
                            3 => "Heavy rain, lightning, thunder",
                            4 => "Snow particles, frost effects",
                            5 => "Reduced visibility, muted colors",
                            6 => "Sand particles, orange tint",
                            _ => "No effects",
                        };
                        ui.horizontal(|ui| {
                            ui.weak("Effects:");
                            ui.label(weather_effects);
                        });

                        // Particle count override slider
                        if self.world_weather_preset >= 2 && self.world_weather_preset != 5 {
                            ui.add_space(4.0);
                            let default_count = match self.world_weather_preset {
                                2 => 20000u32, // Rain
                                3 => 20000,    // Storm (heavy rain)
                                4 => 8000,     // Snow
                                6 => 6000,     // Sandstorm
                                _ => 0,
                            };
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.world_particle_count_override_enabled, "");
                                if self.world_particle_count_override_enabled {
                                    ui.label("Particles:");
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.world_particle_count_value,
                                            100..=50000,
                                        )
                                        .logarithmic(true)
                                        .text(""),
                                    );
                                } else {
                                    ui.weak(format!("Particles: {} (default)", default_count));
                                }
                            });
                        }
                    });

                ui.add_space(4.0);

                // Environment settings section
                egui::CollapsingHeader::new("Environment")
                    .default_open(false)
                    .show(ui, |ui| {
                        // Ambient color
                        ui.horizontal(|ui| {
                            ui.label("Ambient Color:");
                            let mut color = egui::Color32::from_rgb(
                                (self.world_ambient_color[0] * 255.0) as u8,
                                (self.world_ambient_color[1] * 255.0) as u8,
                                (self.world_ambient_color[2] * 255.0) as u8,
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                self.world_ambient_color = [
                                    color.r() as f32 / 255.0,
                                    color.g() as f32 / 255.0,
                                    color.b() as f32 / 255.0,
                                ];
                            }
                            // Hex display
                            ui.weak(format!(
                                "#{:02X}{:02X}{:02X}",
                                color.r(),
                                color.g(),
                                color.b()
                            ));
                        });

                        ui.add_space(4.0);

                        // Fog settings
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.world_fog_enabled, "Enable Fog");
                        });

                        ui.horizontal(|ui| {
                            ui.label("Fog Start:");
                            ui.add(
                                egui::Slider::new(&mut self.world_fog_start, 10.0..=5000.0)
                                    .suffix(" m")
                                    .text(""),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Fog End:");
                            ui.add(
                                egui::Slider::new(&mut self.world_fog_end, 50.0..=10000.0)
                                    .suffix(" m")
                                    .text(""),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Fog Density:");
                            ui.add(
                                egui::Slider::new(&mut self.world_fog_density, 0.0..=0.05).text(""),
                            );
                        });
                    });

                ui.add_space(4.0);

                // Lighting settings section
                egui::CollapsingHeader::new("Lighting")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Sun Intensity:");
                            ui.add(
                                egui::Slider::new(&mut self.world_sun_intensity, 0.0..=3.0)
                                    .text(""),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Sun Elevation:");
                            ui.add(
                                egui::Slider::new(&mut self.world_sun_elevation, 15.0..=85.0)
                                    .suffix("°")
                                    .text(""),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Sun Azimuth:");
                            ui.add(
                                egui::Slider::new(&mut self.world_sun_azimuth, 0.0..=360.0)
                                    .suffix("°")
                                    .text(""),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Sun Color:");
                            let mut color = egui::Color32::from_rgb(
                                (self.world_sun_color[0] * 255.0) as u8,
                                (self.world_sun_color[1] * 255.0) as u8,
                                (self.world_sun_color[2] * 255.0) as u8,
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                self.world_sun_color = [
                                    color.r() as f32 / 255.0,
                                    color.g() as f32 / 255.0,
                                    color.b() as f32 / 255.0,
                                ];
                            }
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Ambient Intensity:");
                            ui.add(
                                egui::Slider::new(&mut self.world_ambient_intensity, 0.0..=2.0)
                                    .text(""),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Exposure:");
                            ui.add(egui::Slider::new(&mut self.world_exposure, 0.1..=3.0).text(""));
                        });

                        // Quick presets
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Presets:");
                            if ui.small_button("Bright Day").clicked() {
                                self.world_sun_intensity = 1.8;
                                self.world_sun_elevation = 50.0;
                                self.world_ambient_intensity = 0.35;
                                self.world_exposure = 1.1;
                                self.world_sun_color = [1.0, 0.95, 0.85];
                                self.world_ambient_color = [0.55, 0.52, 0.48];
                            }
                            if ui.small_button("Golden Hour").clicked() {
                                self.world_sun_intensity = 1.2;
                                self.world_sun_elevation = 20.0;
                                self.world_sun_azimuth = 260.0;
                                self.world_ambient_intensity = 0.30;
                                self.world_exposure = 0.85;
                                self.world_sun_color = [1.0, 0.65, 0.35];
                                self.world_ambient_color = [0.55, 0.42, 0.35];
                            }
                            if ui.small_button("Overcast").clicked() {
                                self.world_sun_intensity = 0.5;
                                self.world_sun_elevation = 45.0;
                                self.world_ambient_intensity = 0.45;
                                self.world_exposure = 0.85;
                                self.world_sun_color = [0.85, 0.85, 0.9];
                                self.world_ambient_color = [0.55, 0.55, 0.58];
                            }
                        });
                    });

                ui.add_space(4.0);

                // Physics settings section
                egui::CollapsingHeader::new("Physics")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Gravity (m/s²):");
                            ui.add(
                                egui::DragValue::new(&mut self.world_gravity)
                                    .range(-50.0..=50.0)
                                    .speed(0.1),
                            );
                        });

                        // Gravity presets
                        ui.horizontal(|ui| {
                            ui.label("Presets:");
                            if ui
                                .small_button("Earth")
                                .on_hover_text("-9.81 m/s²")
                                .clicked()
                            {
                                self.world_gravity = -9.81;
                            }
                            if ui
                                .small_button("Moon")
                                .on_hover_text("-1.62 m/s²")
                                .clicked()
                            {
                                self.world_gravity = -1.62;
                            }
                            if ui
                                .small_button("Mars")
                                .on_hover_text("-3.71 m/s²")
                                .clicked()
                            {
                                self.world_gravity = -3.71;
                            }
                            if ui.small_button("None").on_hover_text("0.0 m/s²").clicked() {
                                self.world_gravity = 0.0;
                            }
                        });
                    });

                ui.add_space(8.0);
                ui.separator();

                // Quick Actions section
                ui.label("Quick Actions:");
                ui.horizontal(|ui| {
                    if ui.button("Add Entity").clicked() {
                        self.emit_event(PanelEvent::CreateEntity);
                    }
                    if !self.confirm_clear_all {
                        if ui
                            .button("Clear All")
                            .on_hover_text("Clear all entities (will ask for confirmation)")
                            .clicked()
                        {
                            self.confirm_clear_all = true;
                        }
                    } else {
                        let confirm_resp = ui.button("Confirm Clear All").on_hover_text(
                            "Click to delete all entities, or press Escape to cancel",
                        );
                        if confirm_resp.clicked() {
                            let ids: Vec<_> = self.entity_list.iter().map(|e| e.id).collect();
                            for id in ids {
                                self.emit_event(PanelEvent::DeleteEntity(id));
                            }
                            self.confirm_clear_all = false;
                        } else if confirm_resp.lost_focus()
                            || ui.input(|i| i.key_pressed(egui::Key::Escape))
                        {
                            self.confirm_clear_all = false;
                        }
                    }
                });
            }
            PanelType::Performance => {
                let stats = &self.runtime_stats;

                // Header with FPS and grade
                let (grade, grade_color) = if stats.fps >= 60.0 {
                    ("A+", egui::Color32::GREEN)
                } else if stats.fps >= 55.0 {
                    ("A", egui::Color32::GREEN)
                } else if stats.fps >= 45.0 {
                    ("B", egui::Color32::YELLOW)
                } else if stats.fps >= 30.0 {
                    ("C", egui::Color32::YELLOW)
                } else {
                    ("D", egui::Color32::RED)
                };

                ui.horizontal(|ui| {
                    ui.heading("Performance");
                    ui.separator();
                    ui.colored_label(grade_color, format!("{:.1} FPS", stats.fps));
                    ui.colored_label(grade_color, format!("[{}]", grade));
                });
                ui.separator();

                // Performance grades
                let grade = if stats.fps >= 60.0 {
                    ("A+", egui::Color32::GREEN, "Excellent")
                } else if stats.fps >= 55.0 {
                    ("A", egui::Color32::GREEN, "Good")
                } else if stats.fps >= 45.0 {
                    ("B", egui::Color32::YELLOW, "Acceptable")
                } else if stats.fps >= 30.0 {
                    ("C", egui::Color32::YELLOW, "Below Target")
                } else {
                    ("D", egui::Color32::RED, "Poor")
                };

                ui.horizontal(|ui| {
                    ui.label("Performance Grade:");
                    ui.colored_label(grade.1, format!("{} - {}", grade.0, grade.2));
                });

                ui.add_space(4.0);

                // Frame budget bar
                let budget_used = (stats.frame_time_ms / 16.67).min(2.0);
                ui.label("Frame Budget (16.67ms = 60 FPS):");
                let bar = egui::ProgressBar::new((budget_used / 2.0).min(1.0))
                    .text(format!(
                        "{:.1}% ({:.2}ms)",
                        budget_used * 100.0,
                        stats.frame_time_ms
                    ))
                    .fill(if budget_used <= 1.0 {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    });
                ui.add(bar);

                ui.add_space(8.0);
                ui.separator();

                // Detailed metrics
                ui.collapsing("Detailed Metrics", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Target FPS:");
                        ui.strong("60");
                        ui.label("|");
                        ui.label("Current:");
                        ui.strong(format!("{:.1}", stats.fps));
                    });

                    // Frame time breakdown
                    ui.add_space(4.0);
                    ui.label("Frame Time Breakdown:");

                    let total_subsystem = stats.render_time_ms
                        + stats.physics_time_ms
                        + stats.ai_time_ms
                        + stats.script_time_ms
                        + stats.audio_time_ms;
                    let other_time = (stats.frame_time_ms - total_subsystem).max(0.0);

                    let draw_metric =
                        |ui: &mut egui::Ui, name: &str, time: f32, color: egui::Color32| {
                            let pct = if stats.frame_time_ms > 0.0 {
                                time / stats.frame_time_ms * 100.0
                            } else {
                                0.0
                            };
                            ui.horizontal(|ui| {
                                ui.colored_label(color, "●");
                                ui.label(format!("{:10}: {:5.2}ms ({:4.1}%)", name, time, pct));
                            });
                        };

                    draw_metric(
                        ui,
                        "Render",
                        stats.render_time_ms,
                        egui::Color32::from_rgb(100, 150, 255),
                    );
                    draw_metric(
                        ui,
                        "Physics",
                        stats.physics_time_ms,
                        egui::Color32::from_rgb(255, 150, 100),
                    );
                    draw_metric(
                        ui,
                        "AI",
                        stats.ai_time_ms,
                        egui::Color32::from_rgb(100, 255, 150),
                    );
                    draw_metric(
                        ui,
                        "Scripts",
                        stats.script_time_ms,
                        egui::Color32::from_rgb(255, 255, 100),
                    );
                    draw_metric(
                        ui,
                        "Audio",
                        stats.audio_time_ms,
                        egui::Color32::from_rgb(200, 150, 255),
                    );
                    draw_metric(ui, "Other", other_time, egui::Color32::GRAY);
                });

                // Rendering stats
                ui.collapsing("Rendering Stats", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Draw Calls:");
                        let dc_color = if stats.draw_calls < 500 {
                            egui::Color32::GREEN
                        } else if stats.draw_calls < 2000 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::RED
                        };
                        ui.colored_label(dc_color, format!("{}", stats.draw_calls));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Triangles:");
                        ui.label(format!("{}", stats.triangles));
                    });
                    let gpu_mem = if stats.gpu_memory_bytes >= 1_000_000 {
                        format!("{:.1} MB", stats.gpu_memory_bytes as f64 / 1_000_000.0)
                    } else {
                        format!("{} KB", stats.gpu_memory_bytes / 1000)
                    };
                    ui.horizontal(|ui| {
                        ui.label("GPU Memory:");
                        ui.label(gpu_mem);
                    });
                });

                ui.add_space(8.0);
                ui.separator();

                ui.label("Entity Count Impact:");
                let entity_impact = (stats.entity_count as f32 / 10000.0).min(1.0);
                let impact_color = if entity_impact < 0.5 {
                    egui::Color32::GREEN
                } else if entity_impact < 0.8 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.add(
                    egui::ProgressBar::new(entity_impact)
                        .text(format!("{} entities", stats.entity_count))
                        .fill(impact_color),
                );
            }
            PanelType::MaterialEditor => {
                // Header with material name
                let has_textures = self.current_material.albedo_texture.is_some()
                    || self.current_material.normal_texture.is_some()
                    || self.current_material.metallic_roughness_texture.is_some();
                let texture_indicator = if has_textures { " [T]" } else { "" };
                let material_type = if self.current_material.metallic > 0.5 {
                    "Metal"
                } else if self.current_material.alpha < 1.0 {
                    "Transparent"
                } else {
                    "Dielectric"
                };
                ui.heading(format!(
                    "Material - {}{} [{}]",
                    self.current_material.name, texture_indicator, material_type
                ));
                ui.separator();

                // Material name with duplicate/delete actions
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add_sized(
                        [150.0, 20.0],
                        egui::TextEdit::singleline(&mut self.current_material.name),
                    );
                    ui.separator();
                    if ui
                        .small_button("C")
                        .on_hover_text("Duplicate material")
                        .clicked()
                    {
                        let mut dup = self.current_material.clone();
                        dup.name = format!("{} (Copy)", dup.name);
                        self.current_material = dup;
                        self.emit_event(PanelEvent::MaterialChanged {
                            name: self.current_material.name.clone(),
                            property: "duplicated".to_string(),
                            value: 0.0,
                        });
                    }
                    if ui
                        .small_button("x")
                        .on_hover_text("Delete material")
                        .clicked()
                    {
                        let deleted_name = self.current_material.name.clone();
                        self.current_material = MaterialInfo::default();
                        self.emit_event(PanelEvent::MaterialChanged {
                            name: deleted_name,
                            property: "deleted".to_string(),
                            value: 0.0,
                        });
                    }
                });

                // Preset materials dropdown
                ui.horizontal(|ui| {
                    ui.label("Preset:");
                    let presets = [
                        "Custom", "Metal", "Wood", "Brick", "Stone", "Water", "Glass",
                    ];
                    ui.menu_button(presets[0], |ui| {
                        for (i, preset) in presets.iter().enumerate().skip(1) {
                            if ui.button(*preset).clicked() {
                                // Apply preset values
                                match i {
                                    1 => {
                                        // Metal
                                        self.current_material.metallic = 1.0;
                                        self.current_material.roughness = 0.3;
                                        self.current_material.albedo_color = [0.8, 0.8, 0.8];
                                    }
                                    2 => {
                                        // Wood
                                        self.current_material.metallic = 0.0;
                                        self.current_material.roughness = 0.7;
                                        self.current_material.albedo_color = [0.6, 0.4, 0.2];
                                    }
                                    3 => {
                                        // Brick
                                        self.current_material.metallic = 0.0;
                                        self.current_material.roughness = 0.9;
                                        self.current_material.albedo_color = [0.7, 0.3, 0.2];
                                    }
                                    4 => {
                                        // Stone
                                        self.current_material.metallic = 0.0;
                                        self.current_material.roughness = 0.85;
                                        self.current_material.albedo_color = [0.5, 0.5, 0.5];
                                    }
                                    5 => {
                                        // Water
                                        self.current_material.metallic = 0.0;
                                        self.current_material.roughness = 0.1;
                                        self.current_material.alpha = 0.6;
                                        self.current_material.albedo_color = [0.2, 0.4, 0.6];
                                    }
                                    6 => {
                                        // Glass
                                        self.current_material.metallic = 0.0;
                                        self.current_material.roughness = 0.05;
                                        self.current_material.alpha = 0.3;
                                        self.current_material.albedo_color = [0.95, 0.95, 0.95];
                                    }
                                    _ => {}
                                }
                                ui.close();
                            }
                        }
                    });
                });
                ui.add_space(8.0);

                // Track material changes for event emission
                let mut material_changed = false;
                let mut changed_property = String::new();
                let mut changed_value = 0.0f32;

                // Albedo color with hex input
                ui.label("Albedo Color");
                ui.horizontal(|ui| {
                    let mut color = self.current_material.albedo_color;
                    if ui.color_edit_button_rgb(&mut color).changed() {
                        self.current_material.albedo_color = color;
                        material_changed = true;
                        changed_property = "albedo".to_string();
                        changed_value = (color[0] + color[1] + color[2]) / 3.0;
                    }
                    // Hex color display
                    let hex = format!(
                        "#{:02X}{:02X}{:02X}",
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8
                    );
                    ui.monospace(hex);
                });
                ui.add_space(8.0);

                // PBR properties
                ui.label("PBR Properties");
                let prev_metallic = self.current_material.metallic;
                ui.horizontal(|ui| {
                    ui.label("Metallic:");
                    ui.add(egui::Slider::new(
                        &mut self.current_material.metallic,
                        0.0..=1.0,
                    ));
                });
                if (self.current_material.metallic - prev_metallic).abs() > 0.001 {
                    material_changed = true;
                    changed_property = "metallic".to_string();
                    changed_value = self.current_material.metallic;
                }

                let prev_roughness = self.current_material.roughness;
                ui.horizontal(|ui| {
                    ui.label("Roughness:");
                    ui.add(egui::Slider::new(
                        &mut self.current_material.roughness,
                        0.0..=1.0,
                    ));
                });
                if (self.current_material.roughness - prev_roughness).abs() > 0.001 {
                    material_changed = true;
                    changed_property = "roughness".to_string();
                    changed_value = self.current_material.roughness;
                }
                ui.add_space(8.0);

                // Emission
                ui.label("Emission");
                ui.horizontal(|ui| {
                    let mut emission = self.current_material.emission;
                    if ui.color_edit_button_rgb(&mut emission).changed() {
                        self.current_material.emission = emission;
                        material_changed = true;
                        changed_property = "emission".to_string();
                        changed_value = (emission[0] + emission[1] + emission[2]) / 3.0;
                    }
                    ui.add(
                        egui::Slider::new(&mut self.current_material.emission_strength, 0.0..=10.0)
                            .text("Strength"),
                    );
                });

                ui.add_space(8.0);

                // Additional properties collapsible
                ui.collapsing("Additional Properties", |ui| {
                    // Normal strength
                    ui.horizontal(|ui| {
                        ui.label("Normal Strength:");
                        ui.add(egui::Slider::new(
                            &mut self.current_material.normal_strength,
                            0.0..=2.0,
                        ));
                    });

                    // AO strength
                    ui.horizontal(|ui| {
                        ui.label("AO Strength:");
                        ui.add(egui::Slider::new(
                            &mut self.current_material.ao_strength,
                            0.0..=1.0,
                        ));
                    });

                    // Alpha
                    ui.horizontal(|ui| {
                        ui.label("Alpha:");
                        ui.add(egui::Slider::new(
                            &mut self.current_material.alpha,
                            0.0..=1.0,
                        ));
                    });

                    // Double-sided
                    ui.checkbox(&mut self.current_material.double_sided, "Double Sided");
                });

                // Texture slots collapsible
                let mut texture_clear_slot: Option<String> = None;
                let mut texture_pick_slot: Option<String> = None;
                ui.collapsing("Texture Slots", |ui| {
                    let clear = &mut texture_clear_slot;
                    let pick = &mut texture_pick_slot;
                    let texture_slot =
                        |ui: &mut egui::Ui,
                         label: &str,
                         texture: &Option<String>,
                         clear: &mut Option<String>,
                         pick: &mut Option<String>| {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", label));
                                if let Some(ref path) = texture {
                                    ui.label(path);
                                    if ui.small_button("✕").clicked() {
                                        *clear = Some(label.to_string());
                                    }
                                } else {
                                    ui.weak("None");
                                    if ui
                                        .small_button("...")
                                        .on_hover_text("Select texture")
                                        .clicked()
                                    {
                                        *pick = Some(label.to_string());
                                    }
                                }
                            });
                        };

                    texture_slot(
                        ui,
                        "Albedo",
                        &self.current_material.albedo_texture,
                        clear,
                        pick,
                    );
                    texture_slot(
                        ui,
                        "Normal",
                        &self.current_material.normal_texture,
                        clear,
                        pick,
                    );
                    texture_slot(
                        ui,
                        "Metallic/Roughness",
                        &self.current_material.metallic_roughness_texture,
                        clear,
                        pick,
                    );
                    texture_slot(
                        ui,
                        "Emission",
                        &self.current_material.emission_texture,
                        clear,
                        pick,
                    );
                    texture_slot(ui, "AO", &self.current_material.ao_texture, clear, pick);
                });
                // Handle texture clear outside closure to avoid borrow conflicts
                if let Some(ref slot) = texture_clear_slot {
                    match slot.as_str() {
                        "Albedo" => self.current_material.albedo_texture = None,
                        "Normal" => self.current_material.normal_texture = None,
                        "Metallic/Roughness" => {
                            self.current_material.metallic_roughness_texture = None
                        }
                        "Emission" => self.current_material.emission_texture = None,
                        "AO" => self.current_material.ao_texture = None,
                        _ => {}
                    }
                    self.emit_event(PanelEvent::MaterialChanged {
                        name: self.current_material.name.clone(),
                        property: format!("texture_{}_cleared", slot),
                        value: 0.0,
                    });
                }
                if let Some(ref slot) = texture_pick_slot {
                    // Open native file dialog for texture selection
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "tga", "bmp", "hdr"])
                        .set_title(&format!("Select {} Texture", slot))
                        .pick_file()
                    {
                        let path_str = path.to_string_lossy().to_string();
                        match slot.as_str() {
                            "Albedo" => {
                                self.current_material.albedo_texture = Some(path_str.clone())
                            }
                            "Normal" => {
                                self.current_material.normal_texture = Some(path_str.clone())
                            }
                            "Metallic/Roughness" => {
                                self.current_material.metallic_roughness_texture =
                                    Some(path_str.clone())
                            }
                            "Emission" => {
                                self.current_material.emission_texture = Some(path_str.clone())
                            }
                            "AO" => self.current_material.ao_texture = Some(path_str.clone()),
                            _ => {}
                        }
                        self.emit_event(PanelEvent::MaterialChanged {
                            name: self.current_material.name.clone(),
                            property: format!("texture_{}_set", slot),
                            value: 0.0,
                        });
                    }
                }

                // Emit event if material changed
                if material_changed {
                    self.emit_event(PanelEvent::MaterialChanged {
                        name: self.current_material.name.clone(),
                        property: changed_property,
                        value: changed_value,
                    });
                }

                ui.add_space(16.0);
                ui.separator();

                // Material Preview with sphere-like shading
                ui.label("Material Preview");
                let preview_size = 100.0;
                let (preview_rect, _) = ui.allocate_exact_size(
                    egui::vec2(preview_size, preview_size),
                    egui::Sense::hover(),
                );

                // Capture values before painting
                let show_alpha_indicator = self.current_material.alpha < 1.0;
                let alpha_val = self.current_material.alpha;
                let show_emission = self.current_material.emission_strength > 0.1;
                let emission_strength = self.current_material.emission_strength;
                let emission_rgb = self.current_material.emission;
                let metallic_val = self.current_material.metallic;
                let roughness_val = self.current_material.roughness;

                // Do all painting first
                {
                    let painter = ui.painter();

                    // Background
                    painter.rect_filled(preview_rect, 8.0, egui::Color32::from_gray(20));

                    // Draw a simple sphere representation
                    let center = preview_rect.center();
                    let radius = preview_size / 2.0 - 8.0;
                    let base_color = egui::Color32::from_rgb(
                        (self.current_material.albedo_color[0] * 255.0) as u8,
                        (self.current_material.albedo_color[1] * 255.0) as u8,
                        (self.current_material.albedo_color[2] * 255.0) as u8,
                    );

                    // Draw gradient circles to simulate 3D sphere
                    let steps = 10;
                    for i in (0..=steps).rev() {
                        let t = i as f32 / steps as f32;
                        let r = radius * t;
                        // Darken towards edges, brighter towards light source (top-left)
                        let brightness = 0.3 + 0.7 * (1.0 - t);
                        let color = egui::Color32::from_rgb(
                            (base_color.r() as f32 * brightness) as u8,
                            (base_color.g() as f32 * brightness) as u8,
                            (base_color.b() as f32 * brightness) as u8,
                        );
                        painter.circle_filled(center, r, color);
                    }

                    // Add specular highlight for metals
                    if metallic_val > 0.3 {
                        let highlight_pos =
                            egui::pos2(center.x - radius * 0.3, center.y - radius * 0.3);
                        let highlight_size = radius * 0.3 * (1.0 - roughness_val);
                        let highlight_alpha = (255.0 * (1.0 - roughness_val) * metallic_val) as u8;
                        painter.circle_filled(
                            highlight_pos,
                            highlight_size,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, highlight_alpha),
                        );
                    }

                    // Emission glow effect
                    if show_emission {
                        let emission_color = egui::Color32::from_rgba_unmultiplied(
                            (emission_rgb[0] * 255.0) as u8,
                            (emission_rgb[1] * 255.0) as u8,
                            (emission_rgb[2] * 255.0) as u8,
                            (50.0 * emission_strength.min(5.0)) as u8,
                        );
                        painter.circle_stroke(
                            center,
                            radius + 4.0,
                            egui::Stroke::new(3.0, emission_color),
                        );
                    }
                }

                // Alpha indicator (after painting scope ends)
                if show_alpha_indicator {
                    ui.weak(format!("Alpha: {:.0}%", alpha_val * 100.0));
                }

                ui.add_space(4.0);

                // Material stats summary
                ui.horizontal(|ui| {
                    ui.weak(format!("M:{:.0}%", metallic_val * 100.0));
                    ui.weak("|");
                    ui.weak(format!("R:{:.0}%", roughness_val * 100.0));
                    if emission_strength > 0.0 {
                        ui.weak("|");
                        ui.weak(format!("E:{:.1}", emission_strength));
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("Save")
                        .on_hover_text("Save material to file")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Material", &["ron", "json"])
                            .set_title("Save Material")
                            .set_file_name(&format!("{}.ron", self.current_material.name))
                            .save_file()
                        {
                            // Serialize material to RON format
                            let content = format!(
                                "// AstraWeave Material: {}\n{:#?}",
                                self.current_material.name, self.current_material
                            );
                            if let Err(e) = std::fs::write(&path, content) {
                                self.push_console_log(format!(
                                    "[ERROR] Failed to save material: {}",
                                    e
                                ));
                            } else {
                                self.push_console_log(format!(
                                    "[INFO] Material saved to {}",
                                    path.display()
                                ));
                            }
                        }
                    }
                    if ui
                        .button("Reset")
                        .on_hover_text("Reset to default values")
                        .clicked()
                    {
                        self.current_material = MaterialInfo::default();
                    }
                    if ui
                        .button("Export")
                        .on_hover_text("Export as .mat file")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Material File", &["mat"])
                            .set_title("Export Material")
                            .set_file_name(&format!("{}.mat", self.current_material.name))
                            .save_file()
                        {
                            let content = format!("{:#?}", self.current_material);
                            if let Err(e) = std::fs::write(&path, content) {
                                self.push_console_log(format!(
                                    "[ERROR] Failed to export material: {}",
                                    e
                                ));
                            } else {
                                self.push_console_log(format!(
                                    "[INFO] Material exported to {}",
                                    path.display()
                                ));
                            }
                        }
                    }
                });
            }
            PanelType::ThemeManager => {
                // Header with current theme name
                let theme_name = match self.current_theme {
                    EditorTheme::Dark => "Dark",
                    EditorTheme::Light => "Light",
                    EditorTheme::Nord => "Nord",
                    EditorTheme::Solarized => "Solarized",
                };
                ui.horizontal(|ui| {
                    ui.heading("Theme Manager");
                    ui.weak(format!("• {}", theme_name));
                });
                ui.separator();

                let prev_theme = self.current_theme;

                ui.label("Editor Theme");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.current_theme, EditorTheme::Dark, "Dark");
                    ui.selectable_value(&mut self.current_theme, EditorTheme::Light, "Light");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.current_theme, EditorTheme::Nord, "Nord");
                    ui.selectable_value(
                        &mut self.current_theme,
                        EditorTheme::Solarized,
                        "Solarized",
                    );
                });

                // Emit event if theme changed
                if self.current_theme != prev_theme {
                    self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                }

                ui.add_space(8.0);

                // Font & Text Settings (collapsible)
                ui.collapsing("Font & Text", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        ui.add(egui::Slider::new(&mut self.font_size, 10.0..=20.0).suffix("px"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Icon Size:");
                        ui.add(egui::Slider::new(&mut self.icon_size, 12.0..=24.0).suffix("px"));
                    });
                });

                // Layout Settings (collapsible)
                ui.collapsing("Layout", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("UI Scale:");
                        ui.add(egui::Slider::new(&mut self.ui_scale, 0.75..=2.0).text("x"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Panel Padding:");
                        ui.add(egui::Slider::new(&mut self.panel_padding, 2.0..=16.0).suffix("px"));
                    });
                });

                // Viewport Settings (collapsible)
                ui.collapsing("Viewport", |ui| {
                    ui.checkbox(&mut self.grid_enabled, "Show Grid");
                    ui.checkbox(&mut self.snap_enabled, "Snap to Grid");
                    ui.horizontal(|ui| {
                        ui.label("Gizmo Size:");
                        ui.add(egui::Slider::new(&mut self.gizmo_size, 0.5..=2.0).text("x"));
                    });
                });

                // Behavior Settings (collapsible)
                ui.collapsing("Behavior", |ui| {
                    ui.checkbox(&mut self.show_tooltips, "Show Tooltips");
                    ui.horizontal(|ui| {
                        ui.label("Auto-Save:");
                        let intervals = ["Disabled", "1 min", "5 min", "10 min", "30 min"];
                        let interval_values = [0, 60, 300, 600, 1800];
                        let current_idx = interval_values
                            .iter()
                            .position(|&v| v == self.auto_save_interval)
                            .unwrap_or(0);
                        egui::ComboBox::from_id_salt("autosave")
                            .width(80.0)
                            .selected_text(intervals[current_idx])
                            .show_ui(ui, |ui| {
                                for (i, &label) in intervals.iter().enumerate() {
                                    if ui
                                        .selectable_value(
                                            &mut self.auto_save_interval,
                                            interval_values[i],
                                            label,
                                        )
                                        .clicked()
                                    {}
                                }
                            });
                    });
                });

                ui.add_space(16.0);
                ui.separator();

                // Theme preview
                ui.label("Theme Preview");
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().min(200.0), 60.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter();
                let (bg, fg, accent) = match self.current_theme {
                    EditorTheme::Dark => (
                        egui::Color32::from_rgb(30, 30, 30),
                        egui::Color32::from_rgb(200, 200, 200),
                        egui::Color32::from_rgb(66, 135, 245),
                    ),
                    EditorTheme::Light => (
                        egui::Color32::from_rgb(245, 245, 245),
                        egui::Color32::from_rgb(30, 30, 30),
                        egui::Color32::from_rgb(33, 150, 243),
                    ),
                    EditorTheme::Nord => (
                        egui::Color32::from_rgb(46, 52, 64),
                        egui::Color32::from_rgb(216, 222, 233),
                        egui::Color32::from_rgb(136, 192, 208),
                    ),
                    EditorTheme::Solarized => (
                        egui::Color32::from_rgb(0, 43, 54),
                        egui::Color32::from_rgb(147, 161, 161),
                        egui::Color32::from_rgb(38, 139, 210),
                    ),
                };
                painter.rect_filled(rect, 4.0, bg);
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        rect.min + egui::vec2(10.0, 10.0),
                        egui::vec2(40.0, 20.0),
                    ),
                    2.0,
                    accent,
                );
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Aa",
                    egui::FontId::proportional(16.0),
                    fg,
                );

                ui.add_space(8.0);

                // Reset button
                ui.horizontal(|ui| {
                    if ui.button("Reset to Defaults").clicked() {
                        self.current_theme = EditorTheme::Dark;
                        self.ui_scale = 1.0;
                        self.font_size = 13.0;
                        self.icon_size = 16.0;
                        self.panel_padding = 8.0;
                        self.gizmo_size = 1.0;
                        self.show_tooltips = true;
                        self.auto_save_interval = 0;
                        self.grid_enabled = true;
                        self.snap_enabled = true;
                    }
                });
            }
            PanelType::EntityPanel => {
                // Enhanced header with selected entity name
                let header = if let Some(ref info) = self.selected_entity_info {
                    format!(
                        "Entity - {} ({} components)",
                        info.name,
                        info.components.len()
                    )
                } else if self.selected_entity.is_some() {
                    "Entity - Loading...".to_string()
                } else {
                    "Entity - None Selected".to_string()
                };
                ui.heading(header);
                ui.separator();

                if let Some(entity_id) = self.selected_entity {
                    // Clone entity info to avoid borrow conflicts with closures
                    let info_snapshot = self.selected_entity_info.clone();
                    if let Some(info) = info_snapshot {
                        // Editable entity name
                        let mut name_buf = info.name.clone();
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            if ui.text_edit_singleline(&mut name_buf).lost_focus()
                                && name_buf != info.name
                            {
                                self.emit_event(PanelEvent::EntityRenamed {
                                    entity_id,
                                    new_name: name_buf,
                                });
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.weak(format!("ID: {}", entity_id));
                            ui.weak(format!("Type: {}", info.entity_type));
                        });

                        ui.add_space(4.0);
                        ui.separator();

                        // -- Transform (Pose) editor --
                        if info.pos_x.is_some()
                            || info.pos_y.is_some()
                            || info.rotation.is_some()
                            || info.scale.is_some()
                        {
                            egui::CollapsingHeader::new("Transform")
                                .default_open(true)
                                .show(ui, |ui| {
                                    if let (Some(px), Some(py)) = (info.pos_x, info.pos_y) {
                                        let mut x = px as f32;
                                        let mut y = py as f32;
                                        ui.horizontal(|ui| {
                                            ui.label("Position:");
                                            let cx = ui
                                                .add(
                                                    egui::DragValue::new(&mut x)
                                                        .prefix("X: ")
                                                        .speed(1.0),
                                                )
                                                .changed();
                                            let cy = ui
                                                .add(
                                                    egui::DragValue::new(&mut y)
                                                        .prefix("Y: ")
                                                        .speed(1.0),
                                                )
                                                .changed();
                                            if cx || cy {
                                                self.emit_event(
                                                    PanelEvent::TransformPositionChanged {
                                                        entity_id,
                                                        x,
                                                        y,
                                                        z: 0.0,
                                                    },
                                                );
                                                if let Some(ref mut sel) = self.selected_entity_info
                                                {
                                                    sel.pos_x = Some(x as i32);
                                                    sel.pos_y = Some(y as i32);
                                                }
                                            }
                                        });
                                    }
                                    if let Some(rot) = info.rotation {
                                        let mut rot_deg = rot.to_degrees();
                                        ui.horizontal(|ui| {
                                            ui.label("Rotation:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut rot_deg)
                                                        .suffix("\u{00b0}")
                                                        .speed(1.0),
                                                )
                                                .changed()
                                            {
                                                let new_rot = rot_deg.to_radians();
                                                self.emit_event(
                                                    PanelEvent::TransformRotationChanged {
                                                        entity_id,
                                                        rotation: new_rot,
                                                    },
                                                );
                                                if let Some(ref mut sel) = self.selected_entity_info
                                                {
                                                    sel.rotation = Some(new_rot);
                                                }
                                            }
                                        });
                                    }
                                    if let Some(sc) = info.scale {
                                        let mut s = sc;
                                        ui.horizontal(|ui| {
                                            ui.label("Scale:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut s)
                                                        .speed(0.01)
                                                        .range(0.01..=100.0),
                                                )
                                                .changed()
                                            {
                                                self.emit_event(
                                                    PanelEvent::TransformScaleChanged {
                                                        entity_id,
                                                        scale_x: s,
                                                        scale_y: s,
                                                        scale_z: s,
                                                    },
                                                );
                                                if let Some(ref mut sel) = self.selected_entity_info
                                                {
                                                    sel.scale = Some(s);
                                                }
                                            }
                                        });
                                    }
                                });
                        }

                        // -- Health editor --
                        if let Some(hp) = info.hp {
                            egui::CollapsingHeader::new("Health")
                                .default_open(true)
                                .show(ui, |ui| {
                                    let mut hp_val = hp;
                                    ui.horizontal(|ui| {
                                        ui.label("HP:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut hp_val)
                                                    .speed(1.0)
                                                    .range(0..=10000),
                                            )
                                            .changed()
                                        {
                                            self.emit_event(PanelEvent::HealthChanged {
                                                entity_id,
                                                new_hp: hp_val,
                                            });
                                        }
                                    });
                                    // Health bar visualization
                                    let pct = (hp_val as f32 / 100.0).clamp(0.0, 1.0);
                                    let bar_color = if pct > 0.6 {
                                        egui::Color32::GREEN
                                    } else if pct > 0.3 {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::RED
                                    };
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width().min(200.0), 8.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter()
                                        .rect_filled(rect, 2.0, egui::Color32::DARK_GRAY);
                                    let filled = egui::Rect::from_min_size(
                                        rect.min,
                                        egui::vec2(rect.width() * pct, rect.height()),
                                    );
                                    ui.painter().rect_filled(filled, 2.0, bar_color);
                                });
                        }

                        // -- Team editor --
                        if let Some(tid) = info.team_id {
                            egui::CollapsingHeader::new("Team")
                                .default_open(true)
                                .show(ui, |ui| {
                                    let mut id_val = tid as i32;
                                    ui.horizontal(|ui| {
                                        ui.label("Team ID:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut id_val)
                                                    .speed(1.0)
                                                    .range(0..=255),
                                            )
                                            .changed()
                                        {
                                            self.emit_event(PanelEvent::TeamChanged {
                                                entity_id,
                                                new_team_id: id_val as u8,
                                            });
                                        }
                                        let team_name = match id_val as u8 {
                                            0 => "Player",
                                            1 => "Enemy",
                                            2 => "Neutral",
                                            _ => "Custom",
                                        };
                                        ui.weak(format!("({})", team_name));
                                    });
                                });
                        }

                        // -- Ammo editor --
                        if let Some(ammo_val) = info.ammo {
                            egui::CollapsingHeader::new("Ammo")
                                .default_open(true)
                                .show(ui, |ui| {
                                    let mut rounds = ammo_val;
                                    ui.horizontal(|ui| {
                                        ui.label("Rounds:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut rounds)
                                                    .speed(1.0)
                                                    .range(0..=10000),
                                            )
                                            .changed()
                                        {
                                            self.emit_event(PanelEvent::AmmoChanged {
                                                entity_id,
                                                new_ammo: rounds,
                                            });
                                        }
                                    });
                                });
                        }

                        // -- Rich component inspectors --
                        let other_comps: Vec<String> = info
                            .components
                            .iter()
                            .filter(|c| {
                                !matches!(c.as_str(), "Transform" | "Health" | "Team" | "Ammo")
                            })
                            .cloned()
                            .collect();
                        if !other_comps.is_empty() {
                            let mut comp_to_remove = None;
                            let comp_data = info.component_data.clone();
                            for comp in &other_comps {
                                let data = comp_data
                                    .get(comp.as_str())
                                    .cloned()
                                    .unwrap_or(serde_json::json!({}));
                                let header_label = match comp.as_str() {
                                    "Light" => "💡 Light",
                                    "Collider" => "⬜ Collider",
                                    "RigidBody" => "🔵 RigidBody",
                                    "Audio" => "🔊 Audio Source",
                                    "Camera" => "📷 Camera",
                                    "Script" => "📜 Script",
                                    "Particle" => "✨ Particle System",
                                    "Sprite" => "🖼️ Sprite",
                                    "MovementScript" => "🏃 Movement Script",
                                    _ => comp.as_str(),
                                };
                                egui::CollapsingHeader::new(header_label)
                                    .default_open(true)
                                    .id_salt(format!("comp_{}", comp))
                                    .show(ui, |ui| {
                                        match comp.as_str() {
                                            "Light" => {
                                                inspectors::show_light_inspector(
                                                    ui,
                                                    entity_id,
                                                    &data,
                                                    &mut self.pending_events,
                                                );
                                            }
                                            "Collider" => {
                                                inspectors::show_collider_inspector(
                                                    ui,
                                                    entity_id,
                                                    &data,
                                                    &mut self.pending_events,
                                                );
                                            }
                                            "RigidBody" => {
                                                inspectors::show_rigidbody_inspector(
                                                    ui,
                                                    entity_id,
                                                    &data,
                                                    &mut self.pending_events,
                                                );
                                            }
                                            "Audio" => {
                                                inspectors::show_audio_inspector(
                                                    ui,
                                                    entity_id,
                                                    &data,
                                                    &mut self.pending_events,
                                                );
                                            }
                                            "Camera" => {
                                                inspectors::show_camera_inspector(
                                                    ui,
                                                    entity_id,
                                                    &data,
                                                    &mut self.pending_events,
                                                );
                                            }
                                            "Script" => {
                                                inspectors::show_script_inspector(
                                                    ui,
                                                    entity_id,
                                                    &data,
                                                    &mut self.pending_events,
                                                );
                                            }
                                            "Particle" => {
                                                inspectors::show_particle_inspector(
                                                    ui,
                                                    entity_id,
                                                    &data,
                                                    &mut self.pending_events,
                                                );
                                            }
                                            _ => {
                                                // Fallback: display raw JSON
                                                ui.label(format!("{}", data));
                                            }
                                        }
                                        ui.horizontal(|ui| {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .small_button("🗑 Remove")
                                                        .on_hover_text(format!("Remove {}", comp))
                                                        .clicked()
                                                    {
                                                        comp_to_remove = Some(comp.clone());
                                                    }
                                                },
                                            );
                                        });
                                    });
                            }
                            if let Some(comp) = comp_to_remove {
                                self.emit_event(PanelEvent::RemoveComponent {
                                    entity_id,
                                    component_type: comp,
                                });
                            }
                        }

                        // -- Material editing --
                        egui::CollapsingHeader::new("🎨 Material")
                            .default_open(false)
                            .show(ui, |ui| {
                                let mut bc = info.material_base_color;
                                ui.horizontal(|ui| {
                                    ui.label("Base Color:");
                                    if ui.color_edit_button_rgba_unmultiplied(&mut bc).changed() {
                                        self.emit_event(PanelEvent::MaterialPropertyChanged {
                                            entity_id,
                                            property: "base_color".to_string(),
                                            value: serde_json::json!([bc[0], bc[1], bc[2], bc[3]]),
                                        });
                                    }
                                });
                                let mut met = info.material_metallic;
                                ui.horizontal(|ui| {
                                    ui.label("Metallic:");
                                    if ui.add(egui::Slider::new(&mut met, 0.0..=1.0)).changed() {
                                        self.emit_event(PanelEvent::MaterialPropertyChanged {
                                            entity_id,
                                            property: "metallic".to_string(),
                                            value: serde_json::json!(met),
                                        });
                                    }
                                });
                                let mut rough = info.material_roughness;
                                ui.horizontal(|ui| {
                                    ui.label("Roughness:");
                                    if ui.add(egui::Slider::new(&mut rough, 0.0..=1.0)).changed() {
                                        self.emit_event(PanelEvent::MaterialPropertyChanged {
                                            entity_id,
                                            property: "roughness".to_string(),
                                            value: serde_json::json!(rough),
                                        });
                                    }
                                });
                                let mut em = info.material_emissive;
                                ui.horizontal(|ui| {
                                    ui.label("Emissive:");
                                    let mut col3 = [em[0], em[1], em[2]];
                                    if ui.color_edit_button_rgb(&mut col3).changed() {
                                        em = col3;
                                        self.emit_event(PanelEvent::MaterialPropertyChanged {
                                            entity_id,
                                            property: "emissive".to_string(),
                                            value: serde_json::json!([em[0], em[1], em[2]]),
                                        });
                                    }
                                });

                                ui.separator();
                                ui.label("Texture Slots:");
                                let tex_slots = [
                                    "Albedo",
                                    "Normal",
                                    "Roughness",
                                    "Metallic",
                                    "AO",
                                    "Emission",
                                ];
                                let tex_snapshot = info.material_textures.clone();
                                for slot in &tex_slots {
                                    let current =
                                        tex_snapshot.get(*slot).cloned().unwrap_or_default();
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}:", slot));
                                        if current.is_empty() {
                                            ui.weak("(none)");
                                        } else {
                                            ui.weak(&current);
                                        }
                                        if ui
                                            .small_button("📂")
                                            .on_hover_text(format!("Browse for {} texture", slot))
                                            .clicked()
                                        {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter(
                                                    "Images",
                                                    &["png", "jpg", "jpeg", "bmp", "tga"],
                                                )
                                                .pick_file()
                                            {
                                                self.emit_event(
                                                    PanelEvent::MaterialTextureChanged {
                                                        entity_id,
                                                        slot: slot.to_string(),
                                                        path: path.to_string_lossy().to_string(),
                                                    },
                                                );
                                            }
                                        }
                                        if !current.is_empty() {
                                            if ui
                                                .small_button("✕")
                                                .on_hover_text("Clear texture")
                                                .clicked()
                                            {
                                                self.emit_event(
                                                    PanelEvent::MaterialTextureChanged {
                                                        entity_id,
                                                        slot: slot.to_string(),
                                                        path: String::new(),
                                                    },
                                                );
                                            }
                                        }
                                    });
                                }
                            });

                        ui.add_space(4.0);

                        // Add Component dropdown
                        let components_snapshot = info.components.clone();
                        let mut add_comp_event = None;
                        ui.menu_button("+ Add Component", |ui| {
                            let available = [
                                "Transform",
                                "Health",
                                "Team",
                                "Ammo",
                                "Sprite",
                                "Collider",
                                "RigidBody",
                                "Script",
                                "MovementScript",
                                "Audio",
                                "Light",
                                "Camera",
                                "Particle",
                            ];
                            for comp_name in &available {
                                let already_has =
                                    components_snapshot.iter().any(|c| c == comp_name);
                                let btn = ui.add_enabled(
                                    !already_has,
                                    egui::Button::new(if already_has {
                                        format!("  {} (added)", comp_name)
                                    } else {
                                        format!("  {}", comp_name)
                                    }),
                                );
                                if btn.clicked() {
                                    add_comp_event = Some(comp_name.to_string());
                                    ui.close();
                                }
                            }
                        });
                        if let Some(comp) = add_comp_event {
                            self.emit_event(PanelEvent::AddComponent {
                                entity_id,
                                component_type: comp,
                            });
                        }

                        ui.add_space(8.0);
                        ui.separator();

                        // Actions
                        ui.horizontal(|ui| {
                            if ui.button("Duplicate").clicked() {
                                self.emit_event(PanelEvent::DuplicateEntity(entity_id));
                            }
                            if ui.button("Delete").on_hover_text("Delete entity").clicked() {
                                self.emit_event(PanelEvent::DeleteEntity(entity_id));
                            }
                        });
                    } else {
                        ui.label(format!("Entity ID: {}", entity_id));
                        ui.label("Loading entity info...");
                    }
                } else {
                    // Empty state with guidance
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.heading("No Entity Selected");
                        ui.add_space(8.0);
                        ui.weak("Select an entity in the Hierarchy panel\nor the 3D Viewport to inspect it.");
                        ui.add_space(16.0);
                        ui.weak("Tip: Use the + Add button in the Hierarchy\npanel to create new entities.");
                    });
                }
            }
            PanelType::BuildManager => {
                // Build status for header
                let (status_icon, status_text, status_color) = match self.build_status {
                    1 => ("...", "Building...", egui::Color32::YELLOW),
                    2 => ("OK", "Success", egui::Color32::GREEN),
                    3 => ("!!", "Failed", egui::Color32::RED),
                    _ => ("--", "Idle", egui::Color32::GRAY),
                };
                let targets = ["Windows", "Linux", "macOS", "WebGL", "Android", "iOS"];
                let target_names = ["windows", "linux", "macos", "webgl", "android", "ios"];
                let profiles = ["Debug", "Release", "Profile"];

                ui.horizontal(|ui| {
                    ui.heading("Build Manager");
                    ui.colored_label(status_color, format!("{} {}", status_icon, status_text));
                    if self.build_status != 1 {
                        // Show current target when not building
                        ui.weak(format!(
                            "• {} {}",
                            targets[self.build_target], profiles[self.build_profile]
                        ));
                    }
                });
                ui.separator();

                // Show progress bar if building
                if self.build_status == 1 {
                    let elapsed = self
                        .build_start_time
                        .map(|t| t.elapsed().as_secs_f32())
                        .unwrap_or(0.0);

                    // Calculate estimated time remaining
                    let eta_text = if self.build_progress > 0.05 && elapsed > 2.0 {
                        let total_estimated = elapsed / self.build_progress;
                        let remaining = (total_estimated - elapsed).max(0.0);
                        format!(" • ETA: {:.0}s", remaining)
                    } else {
                        String::new()
                    };

                    ui.add(
                        egui::ProgressBar::new(self.build_progress)
                            .text(format!(
                                "{:.0}% ({:.1}s{})",
                                self.build_progress * 100.0,
                                elapsed,
                                eta_text
                            ))
                            .animate(true),
                    );
                    ui.add_space(4.0);

                    // Cancel button during build
                    if ui.button("Cancel Build").clicked() {
                        self.build_status = 0;
                        self.build_progress = 0.0;
                        self.build_start_time = None;
                        self.build_output
                            .push(format!("[{}] Build cancelled by user.", chrono_lite_time()));
                    }
                    ui.separator();
                }

                // Build target selection
                egui::CollapsingHeader::new("Target Platform")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("target_grid")
                            .num_columns(3)
                            .show(ui, |ui| {
                                for (i, target) in targets.iter().enumerate() {
                                    let selected = self.build_target == i;
                                    if ui.selectable_label(selected, *target).clicked() {
                                        self.build_target = i;
                                    }
                                    if (i + 1) % 3 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });

                        // Platform-specific notes
                        let note = match self.build_target {
                            0 => "Windows x64, DirectX 12/Vulkan",
                            1 => "Linux x64, Vulkan",
                            2 => "macOS ARM64/x64, Metal",
                            3 => "WebGL 2.0, WASM",
                            4 => "Android ARM64, Vulkan",
                            5 => "iOS ARM64, Metal",
                            _ => "",
                        };
                        ui.weak(note);
                    });

                ui.add_space(4.0);

                // Build configuration
                egui::CollapsingHeader::new("Configuration")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (i, profile) in profiles.iter().enumerate() {
                                if ui
                                    .selectable_label(self.build_profile == i, *profile)
                                    .clicked()
                                {
                                    self.build_profile = i;
                                }
                            }
                        });

                        // Profile description
                        let desc = match self.build_profile {
                            0 => "Debug: Full symbols, no optimization, fast compile",
                            1 => "Release: Optimized, stripped symbols, slow compile",
                            2 => "Profile: Optimized with symbols for profiling",
                            _ => "",
                        };
                        ui.weak(desc);
                    });

                ui.add_space(4.0);

                // Build options
                egui::CollapsingHeader::new("Build Options")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.checkbox(
                            &mut self.build_include_debug_symbols,
                            "Include debug symbols",
                        )
                        .on_hover_text("Include debug info for crash reports");
                        ui.checkbox(&mut self.build_strip_unused, "Strip unused assets")
                            .on_hover_text("Remove unused textures, models, audio");
                        ui.checkbox(&mut self.build_compress_textures, "Compress textures")
                            .on_hover_text("Use GPU-compressed texture formats");

                        ui.add_space(4.0);

                        // Additional options
                        ui.horizontal(|ui| {
                            ui.label("Texture Quality:");
                            let quality_names = ["Low", "Medium", "High", "Ultra"];
                            egui::ComboBox::from_id_salt("tex_quality")
                                .selected_text(quality_names[self.build_texture_quality.min(3)])
                                .show_ui(ui, |ui| {
                                    for (i, quality) in quality_names.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut self.build_texture_quality,
                                            i,
                                            *quality,
                                        );
                                    }
                                });
                        });
                    });

                ui.add_space(8.0);
                ui.separator();

                // Build actions
                ui.horizontal(|ui| {
                    let can_build = self.build_status != 1;

                    ui.add_enabled_ui(can_build, |ui| {
                        if ui
                            .button("Build")
                            .on_hover_text("Build for selected platform")
                            .clicked()
                        {
                            let target = target_names[self.build_target].to_string();
                            let profile = profiles[self.build_profile].to_lowercase();
                            self.build_output.push(format!(
                                "[{}] Starting {} build for {}...",
                                chrono_lite_time(),
                                profile,
                                target
                            ));
                            self.build_status = 1;
                            self.build_progress = 0.0;
                            self.build_start_time = Some(std::time::Instant::now());
                            self.emit_event(PanelEvent::BuildRequested {
                                target: target.clone(),
                                profile: profile.clone(),
                            });
                        }
                        if ui
                            .button("Build & Run")
                            .on_hover_text("Build and launch")
                            .clicked()
                        {
                            let target = target_names[self.build_target].to_string();
                            let profile = profiles[self.build_profile].to_lowercase();
                            self.build_output.push(format!(
                                "[{}] Building and running {} for {}...",
                                chrono_lite_time(),
                                profile,
                                target
                            ));
                            self.build_status = 1;
                            self.build_progress = 0.0;
                            self.build_start_time = Some(std::time::Instant::now());
                            self.emit_event(PanelEvent::BuildRequested {
                                target: target.clone(),
                                profile: profile.clone(),
                            });
                        }
                    });

                    if ui
                        .button("Open Output")
                        .on_hover_text("Open build output folder")
                        .clicked()
                    {
                        let output_dir = std::path::PathBuf::from("target/build_output");
                        if output_dir.exists() {
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("explorer")
                                    .arg(&output_dir)
                                    .spawn();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open").arg(&output_dir).spawn();
                            }
                            #[cfg(target_os = "linux")]
                            {
                                let _ = std::process::Command::new("xdg-open")
                                    .arg(&output_dir)
                                    .spawn();
                            }
                        } else {
                            self.push_console_log(
                                "[WARN] Build output folder does not exist yet.".to_string(),
                            );
                        }
                    }
                });

                ui.add_space(8.0);

                // Build output log
                egui::CollapsingHeader::new(format!("Build Log ({})", self.build_output.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.small_button("Clear").clicked() {
                                self.build_output.clear();
                            }
                            if ui.small_button("Copy").clicked() {
                                let text = self.build_output.join("\n");
                                ui.ctx().copy_text(text);
                            }
                        });

                        if self.build_output.is_empty() {
                            ui.weak("Ready to build...");
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for line in &self.build_output {
                                        // Color code log lines
                                        if line.contains("error") || line.contains("Error") {
                                            ui.colored_label(egui::Color32::RED, line);
                                        } else if line.contains("warning")
                                            || line.contains("Warning")
                                        {
                                            ui.colored_label(egui::Color32::YELLOW, line);
                                        } else if line.contains("Success")
                                            || line.contains("Complete")
                                        {
                                            ui.colored_label(egui::Color32::GREEN, line);
                                        } else {
                                            ui.label(line);
                                        }
                                    }
                                });
                        }
                    });
            }
            PanelType::AdvancedWidgets => {
                ui.horizontal(|ui| {
                    ui.heading("Advanced Widgets");
                    ui.weak("• 5 widget demos");
                });
                ui.separator();

                // Color picker demo
                ui.collapsing("Color Picker", |ui| {
                    let mut color = [0.5f32, 0.3, 0.8];
                    ui.horizontal(|ui| {
                        ui.label("Primary:");
                        ui.color_edit_button_rgb(&mut color);
                    });
                    let mut color2 = [0.8f32, 0.5, 0.2];
                    ui.horizontal(|ui| {
                        ui.label("Secondary:");
                        ui.color_edit_button_rgb(&mut color2);
                    });
                    ui.add_space(4.0);
                    // Show color preview
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width().min(150.0), 20.0),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter();
                    let half_width = rect.width() / 2.0;
                    painter.rect_filled(
                        egui::Rect::from_min_size(rect.min, egui::vec2(half_width, 20.0)),
                        0.0,
                        egui::Color32::from_rgb(
                            (color[0] * 255.0) as u8,
                            (color[1] * 255.0) as u8,
                            (color[2] * 255.0) as u8,
                        ),
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(half_width, 0.0),
                            egui::vec2(half_width, 20.0),
                        ),
                        0.0,
                        egui::Color32::from_rgb(
                            (color2[0] * 255.0) as u8,
                            (color2[1] * 255.0) as u8,
                            (color2[2] * 255.0) as u8,
                        ),
                    );
                });

                // Gradient demo
                ui.collapsing("Gradient Editor", |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(180.0, 24.0), egui::Sense::hover());
                    let painter = ui.painter();
                    for i in 0..180 {
                        let t = i as f32 / 180.0;
                        let color = egui::Color32::from_rgb(
                            (255.0 * (1.0 - t)) as u8,
                            (100.0 + 155.0 * t) as u8,
                            (255.0 * t) as u8,
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                rect.min + egui::vec2(i as f32, 0.0),
                                egui::vec2(1.0, 24.0),
                            ),
                            0.0,
                            color,
                        );
                    }
                    ui.add_space(4.0);
                    // Preset buttons
                    ui.horizontal(|ui| {
                        if ui.small_button("Sunset").clicked() {
                            // TODO: Apply sunset gradient preset to theme accent
                            self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                        }
                        if ui.small_button("Ocean").clicked() {
                            // TODO: Apply ocean gradient preset to theme accent
                            self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                        }
                        if ui.small_button("Forest").clicked() {
                            // TODO: Apply forest gradient preset to theme accent
                            self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                        }
                        if ui.small_button("Fire").clicked() {
                            // TODO: Apply fire gradient preset to theme accent
                            self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                        }
                    });
                });

                // Curve editor placeholder
                ui.collapsing("Curve Editor", |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(180.0, 80.0), egui::Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(30));

                    // Draw bezier curve
                    let points: Vec<egui::Pos2> = (0..50)
                        .map(|i| {
                            let t = i as f32 / 50.0;
                            let x = rect.left() + t * rect.width();
                            let y = rect.bottom() - (t * t * 0.5 + t * 0.3 + 0.1) * rect.height();
                            egui::pos2(x, y)
                        })
                        .collect();

                    for w in points.windows(2) {
                        painter.line_segment(
                            [w[0], w[1]],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 100)),
                        );
                    }

                    ui.add_space(4.0);
                    // Easing presets
                    ui.horizontal(|ui| {
                        if ui.small_button("Linear").clicked() {
                            // TODO: Set curve to linear interpolation
                            self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                        }
                        if ui.small_button("Ease In").clicked() {
                            // TODO: Set curve to ease-in interpolation
                            self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                        }
                        if ui.small_button("Ease Out").clicked() {
                            // TODO: Set curve to ease-out interpolation
                            self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                        }
                        if ui.small_button("Ease In/Out").clicked() {
                            // TODO: Set curve to ease-in/out interpolation
                            self.emit_event(PanelEvent::ThemeChanged(self.current_theme));
                        }
                    });
                });

                // Range Slider
                ui.collapsing("Range Slider", |ui| {
                    ui.label("Range selection demo:");
                    let mut min_val = 20.0f32;
                    let mut max_val = 80.0f32;
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut min_val)
                                .prefix("Min: ")
                                .range(0.0..=100.0),
                        );
                        ui.add(
                            egui::DragValue::new(&mut max_val)
                                .prefix("Max: ")
                                .range(0.0..=100.0),
                        );
                    });
                    // Visual range bar
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(150.0, 16.0), egui::Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(40));
                    let range_start = rect.left() + (min_val / 100.0) * rect.width();
                    let range_end = rect.left() + (max_val / 100.0) * rect.width();
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(range_start, rect.top()),
                            egui::pos2(range_end, rect.bottom()),
                        ),
                        4.0,
                        egui::Color32::from_rgb(100, 150, 255),
                    );
                });

                // Knob / Dial
                ui.collapsing("Dial Controls", |ui| {
                    // Draw two dials side by side
                    let dial_size = 50.0;
                    let total_width = dial_size * 2.0 + 16.0;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(total_width, dial_size),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter();
                    let radius = dial_size / 2.0 - 4.0;

                    // First dial (Volume at 70%)
                    let center1 = egui::pos2(rect.left() + dial_size / 2.0, rect.center().y);
                    painter.circle_filled(center1, radius, egui::Color32::from_gray(40));
                    painter.circle_stroke(
                        center1,
                        radius,
                        egui::Stroke::new(2.0, egui::Color32::from_gray(80)),
                    );
                    let value1 = 0.7;
                    let angle1 = std::f32::consts::PI * (0.75 + value1 * 1.5);
                    let indicator_end1 = egui::pos2(
                        center1.x + (radius - 6.0) * angle1.cos(),
                        center1.y + (radius - 6.0) * angle1.sin(),
                    );
                    painter.line_segment(
                        [center1, indicator_end1],
                        egui::Stroke::new(3.0, egui::Color32::from_rgb(100, 200, 255)),
                    );
                    painter.circle_filled(center1, 4.0, egui::Color32::from_rgb(100, 200, 255));

                    // Second dial (Pan at 30%)
                    let center2 = egui::pos2(
                        rect.left() + dial_size + 16.0 + dial_size / 2.0,
                        rect.center().y,
                    );
                    painter.circle_filled(center2, radius, egui::Color32::from_gray(40));
                    painter.circle_stroke(
                        center2,
                        radius,
                        egui::Stroke::new(2.0, egui::Color32::from_gray(80)),
                    );
                    let value2 = 0.3;
                    let angle2 = std::f32::consts::PI * (0.75 + value2 * 1.5);
                    let indicator_end2 = egui::pos2(
                        center2.x + (radius - 6.0) * angle2.cos(),
                        center2.y + (radius - 6.0) * angle2.sin(),
                    );
                    painter.line_segment(
                        [center2, indicator_end2],
                        egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 150, 100)),
                    );
                    painter.circle_filled(center2, 4.0, egui::Color32::from_rgb(255, 150, 100));

                    // Labels
                    ui.horizontal(|ui| {
                        ui.weak("Volume");
                        ui.add_space(30.0);
                        ui.weak("Pan");
                    });
                });
            }
            PanelType::Animation => {
                // Animation header with track count, current time, and playback state
                let track_count = self.animation_state.tracks.len();
                let current_time =
                    self.animation_state.current_frame as f32 / self.animation_state.fps;
                let state_icon = if self.animation_state.is_playing {
                    ">"
                } else {
                    "||"
                };
                let loop_icon = if self.animation_state.loop_enabled {
                    "L"
                } else {
                    ""
                };
                let speed_text = if (self.animation_state.playback_speed - 1.0).abs() > 0.01 {
                    format!(" {}x", self.animation_state.playback_speed)
                } else {
                    String::new()
                };
                ui.heading(format!(
                    "Animation {} ({} tracks) - {:.2}s{}{}",
                    state_icon, track_count, current_time, speed_text, loop_icon
                ));
                ui.separator();

                // Transport controls
                let mut play_state_changed = None;
                let mut frame_changed = None;

                ui.horizontal(|ui| {
                    if ui
                        .button("|<")
                        .on_hover_text("Go to start (Home)")
                        .clicked()
                    {
                        frame_changed = Some(0);
                    }
                    let play_btn = if self.animation_state.is_playing {
                        "||"
                    } else {
                        ">"
                    };
                    let play_tip = if self.animation_state.is_playing {
                        "Pause (Space)"
                    } else {
                        "Play (Space)"
                    };
                    if ui.button(play_btn).on_hover_text(play_tip).clicked() {
                        play_state_changed = Some(!self.animation_state.is_playing);
                    }
                    if ui.button("[]").on_hover_text("Stop").clicked() {
                        play_state_changed = Some(false);
                        frame_changed = Some(0);
                    }
                    if ui.button(">|").on_hover_text("Go to end (End)").clicked() {
                        frame_changed = Some(self.animation_state.total_frames);
                    }

                    ui.separator();

                    // Frame counter
                    ui.label("Frame:");
                    let mut frame = self.animation_state.current_frame as i32;
                    if ui
                        .add(
                            egui::DragValue::new(&mut frame)
                                .range(0..=self.animation_state.total_frames as i32),
                        )
                        .changed()
                    {
                        frame_changed = Some(frame.max(0) as u32);
                    }
                    ui.weak(format!("/ {}", self.animation_state.total_frames));

                    ui.separator();

                    // FPS
                    ui.label("FPS:");
                    ui.add(egui::DragValue::new(&mut self.animation_state.fps).range(1.0..=120.0));
                });

                // Second row: Speed, Loop controls
                ui.horizontal(|ui| {
                    // Playback speed selector
                    ui.label("Speed:");
                    let speeds = [0.25, 0.5, 1.0, 2.0, 4.0];
                    for speed in speeds {
                        let label = if speed == 1.0 {
                            "1x".to_string()
                        } else {
                            format!("{}x", speed)
                        };
                        if ui
                            .selectable_label(
                                (self.animation_state.playback_speed - speed).abs() < 0.01,
                                &label,
                            )
                            .clicked()
                        {
                            self.animation_state.playback_speed = speed;
                        }
                    }

                    ui.separator();

                    // Loop mode toggle
                    ui.toggle_value(&mut self.animation_state.loop_enabled, "Loop")
                        .on_hover_text("Enable looping playback")
                        .changed();

                    // Ping-pong mode toggle
                    if ui
                        .toggle_value(&mut self.animation_state.ping_pong, "Ping-Pong")
                        .on_hover_text("Play forward then backward")
                        .changed()
                    {}
                });

                // Apply changes after UI to avoid borrow issues
                if let Some(playing) = play_state_changed {
                    self.animation_state.is_playing = playing;
                    self.emit_event(PanelEvent::AnimationPlayStateChanged {
                        is_playing: playing,
                    });
                }
                if let Some(frame) = frame_changed {
                    self.animation_state.current_frame = frame;
                    self.emit_event(PanelEvent::AnimationFrameChanged { frame });
                }

                ui.add_space(4.0);

                // Timeline scrubber
                let timeline_height = 24.0;
                let available_width = ui.available_width().min(500.0);
                let (timeline_rect, timeline_response) = ui.allocate_exact_size(
                    egui::vec2(available_width, timeline_height),
                    egui::Sense::click_and_drag(),
                );

                let painter = ui.painter();
                painter.rect_filled(timeline_rect, 2.0, egui::Color32::from_gray(40));

                // Draw frame markers
                let frames_per_marker = 30;
                let total_frames = self.animation_state.total_frames.max(1);
                for f in (0..=total_frames).step_by(frames_per_marker as usize) {
                    let t = f as f32 / total_frames as f32;
                    let x = timeline_rect.left() + t * timeline_rect.width();
                    painter.line_segment(
                        [
                            egui::pos2(x, timeline_rect.top()),
                            egui::pos2(x, timeline_rect.top() + 8.0),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                    );
                    painter.text(
                        egui::pos2(x, timeline_rect.bottom() - 4.0),
                        egui::Align2::CENTER_BOTTOM,
                        format!("{}", f),
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_gray(120),
                    );
                }

                // Draw playhead
                let playhead_t = self.animation_state.current_frame as f32 / total_frames as f32;
                let playhead_x = timeline_rect.left() + playhead_t * timeline_rect.width();
                painter.line_segment(
                    [
                        egui::pos2(playhead_x, timeline_rect.top()),
                        egui::pos2(playhead_x, timeline_rect.bottom()),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 100)),
                );

                // Timeline interaction
                if timeline_response.dragged() || timeline_response.clicked() {
                    if let Some(pos) = timeline_response.interact_pointer_pos() {
                        let t = ((pos.x - timeline_rect.left()) / timeline_rect.width())
                            .clamp(0.0, 1.0);
                        self.animation_state.current_frame = (t * total_frames as f32) as u32;
                    }
                }

                ui.add_space(8.0);
                ui.separator();

                // Tracks list
                ui.label("Tracks");
                for (i, track) in self.animation_state.tracks.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let icon = if track.is_visible { "*" } else { "o" };
                        ui.label(icon);

                        let selected = self.animation_state.selected_track == Some(i);
                        if ui.selectable_label(selected, &track.name).clicked() {
                            self.animation_state.selected_track = Some(i);
                        }

                        ui.weak(format!("{} keys", track.keyframes.len()));

                        if track.is_locked {
                            ui.label("[L]");
                        }
                    });

                    // Draw mini track preview
                    let track_height = 16.0;
                    let (track_rect, _) = ui.allocate_exact_size(
                        egui::vec2(available_width, track_height),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter();
                    painter.rect_filled(track_rect, 2.0, egui::Color32::from_gray(30));

                    // Draw keyframes as diamonds
                    for kf in &track.keyframes {
                        let kf_t = kf.frame as f32 / total_frames as f32;
                        let kf_x = track_rect.left() + kf_t * track_rect.width();
                        let kf_y = track_rect.center().y;
                        let diamond_size = 4.0;
                        let points = vec![
                            egui::pos2(kf_x, kf_y - diamond_size),
                            egui::pos2(kf_x + diamond_size, kf_y),
                            egui::pos2(kf_x, kf_y + diamond_size),
                            egui::pos2(kf_x - diamond_size, kf_y),
                        ];
                        painter.add(egui::Shape::convex_polygon(
                            points,
                            egui::Color32::from_rgb(100, 180, 255),
                            egui::Stroke::NONE,
                        ));
                    }
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Add Track").clicked() {
                        let track_idx = self.animation_state.tracks.len();
                        self.animation_state.tracks.push(AnimationTrack {
                            name: format!("Track {}", track_idx),
                            keyframes: Vec::new(),
                            is_visible: true,
                            is_locked: false,
                        });
                        self.animation_state.selected_track = Some(track_idx);
                    }
                    if ui.button("Add Keyframe").clicked() {
                        let frame = self.animation_state.current_frame;
                        if let Some(track_idx) = self.animation_state.selected_track {
                            if let Some(track) = self.animation_state.tracks.get_mut(track_idx) {
                                track.keyframes.push(Keyframe { frame, value: 0.0 });
                                self.emit_event(PanelEvent::AnimationKeyframeAdded {
                                    track_index: track_idx,
                                    frame,
                                    value: 0.0,
                                });
                            }
                        }
                    }
                });
            }
            PanelType::Graph => {
                let node_count = self.graph_nodes.len();
                // Count connections (simplified: count sequential pairs)
                let connection_count = if node_count >= 2 { node_count - 1 } else { 0 };
                // Count node types for header
                let event_count = self
                    .graph_nodes
                    .iter()
                    .filter(|n| n.node_type == "Event")
                    .count();
                let func_count = self
                    .graph_nodes
                    .iter()
                    .filter(|n| n.node_type == "Function")
                    .count();

                ui.horizontal(|ui| {
                    ui.heading(format!("Visual Script ({} nodes)", node_count));
                    if node_count > 0 {
                        ui.weak(format!(
                            "E:{} F:{} L:{}",
                            event_count, func_count, connection_count
                        ));
                    }
                });
                ui.separator();

                // Enhanced toolbar with node type quick-add
                ui.horizontal(|ui| {
                    let add_graph_node = |nodes: &mut Vec<GraphNode>, node_type: &str| {
                        let id = nodes.len() as u32;
                        nodes.push(GraphNode {
                            id,
                            name: format!("{}_{}", node_type, id),
                            node_type: node_type.to_string(),
                            position: (80.0 + id as f32 * 40.0, 60.0 + (id % 3) as f32 * 40.0),
                            inputs: vec!["In".to_string()],
                            outputs: vec!["Out".to_string()],
                        });
                        id
                    };
                    if ui.button("Event").on_hover_text("Add Event Node").clicked() {
                        let id = add_graph_node(&mut self.graph_nodes, "Event");
                        self.emit_event(PanelEvent::GraphNodeSelected(id));
                    }
                    if ui
                        .button("Function")
                        .on_hover_text("Add Function Node")
                        .clicked()
                    {
                        let id = add_graph_node(&mut self.graph_nodes, "Function");
                        self.emit_event(PanelEvent::GraphNodeSelected(id));
                    }
                    if ui.button("Math").on_hover_text("Add Math Node").clicked() {
                        let id = add_graph_node(&mut self.graph_nodes, "Math");
                        self.emit_event(PanelEvent::GraphNodeSelected(id));
                    }
                    if ui
                        .button("Variable")
                        .on_hover_text("Add Variable Node")
                        .clicked()
                    {
                        let id = add_graph_node(&mut self.graph_nodes, "Variable");
                        self.emit_event(PanelEvent::GraphNodeSelected(id));
                    }
                    ui.separator();
                    if ui
                        .button("Connect")
                        .on_hover_text("Enter connection mode")
                        .clicked()
                    {
                        // TODO: Implement connection mode toggle for graph editor
                        self.push_console_log(
                            "[INFO] Connection mode toggled (not yet implemented).".to_string(),
                        );
                    }
                    if ui.button("x").on_hover_text("Delete selected").clicked() {
                        // TODO: Track selected graph node for deletion
                        self.push_console_log(
                            "[INFO] Delete selected graph node (not yet implemented).".to_string(),
                        );
                    }
                });

                // Second toolbar row with view controls
                ui.horizontal(|ui| {
                    if ui
                        .button("Align")
                        .on_hover_text("Auto-align nodes")
                        .clicked()
                    {
                        // Auto-align nodes in a grid layout
                        let cols = 4;
                        for (i, node) in self.graph_nodes.iter_mut().enumerate() {
                            let col = i % cols;
                            let row = i / cols;
                            node.position = (80.0 + col as f32 * 120.0, 40.0 + row as f32 * 80.0);
                        }
                    }
                    if ui
                        .button("Fit")
                        .on_hover_text("Fit all nodes in view")
                        .clicked()
                    {
                        // Reset node positions to fit within the visible canvas
                        let total = self.graph_nodes.len().max(1) as f32;
                        for (i, node) in self.graph_nodes.iter_mut().enumerate() {
                            let t = i as f32 / total;
                            node.position = (40.0 + t * 320.0, 60.0 + (i % 3) as f32 * 50.0);
                        }
                    }
                    ui.separator();
                    ui.label("Zoom:");
                    ui.add(egui::Slider::new(&mut 1.0_f32, 0.25..=2.0).show_value(false));
                });

                ui.add_space(8.0);

                // Graph canvas
                let canvas_size = egui::vec2(ui.available_width().min(400.0), 200.0);
                let (canvas_rect, response) =
                    ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());
                let painter = ui.painter_at(canvas_rect);

                // Background grid
                painter.rect_filled(canvas_rect, 4.0, egui::Color32::from_gray(25));
                let grid_spacing = 20.0;
                for x in (0..(canvas_size.x as i32)).step_by(grid_spacing as usize) {
                    painter.line_segment(
                        [
                            egui::pos2(canvas_rect.left() + x as f32, canvas_rect.top()),
                            egui::pos2(canvas_rect.left() + x as f32, canvas_rect.bottom()),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(35)),
                    );
                }
                for y in (0..(canvas_size.y as i32)).step_by(grid_spacing as usize) {
                    painter.line_segment(
                        [
                            egui::pos2(canvas_rect.left(), canvas_rect.top() + y as f32),
                            egui::pos2(canvas_rect.right(), canvas_rect.top() + y as f32),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(35)),
                    );
                }

                // Draw bezier connections (enhanced)
                let conn_color = egui::Color32::from_rgb(150, 150, 150);
                if self.graph_nodes.len() >= 2 {
                    let n0 = &self.graph_nodes[0];
                    let n1 = &self.graph_nodes[1];
                    let p0 = egui::pos2(
                        canvas_rect.left() + n0.position.0 + 60.0,
                        canvas_rect.top() + n0.position.1 + 20.0,
                    );
                    let p1 = egui::pos2(
                        canvas_rect.left() + n1.position.0,
                        canvas_rect.top() + n1.position.1 + 20.0,
                    );
                    // Bezier curve connection
                    let ctrl_offset = (p1.x - p0.x).abs() * 0.5;
                    let ctrl0 = egui::pos2(p0.x + ctrl_offset, p0.y);
                    let ctrl1 = egui::pos2(p1.x - ctrl_offset, p1.y);
                    let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                        [p0, ctrl0, ctrl1, p1],
                        false,
                        egui::Color32::TRANSPARENT,
                        egui::Stroke::new(2.0, conn_color),
                    );
                    painter.add(bezier);
                }
                if self.graph_nodes.len() >= 3 {
                    let n1 = &self.graph_nodes[1];
                    let n2 = &self.graph_nodes[2];
                    let p1 = egui::pos2(
                        canvas_rect.left() + n1.position.0 + 80.0,
                        canvas_rect.top() + n1.position.1 + 20.0,
                    );
                    let p2 = egui::pos2(
                        canvas_rect.left() + n2.position.0,
                        canvas_rect.top() + n2.position.1 + 20.0,
                    );
                    // Bezier curve connection
                    let ctrl_offset = (p2.x - p1.x).abs() * 0.5;
                    let ctrl0 = egui::pos2(p1.x + ctrl_offset, p1.y);
                    let ctrl1 = egui::pos2(p2.x - ctrl_offset, p2.y);
                    let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                        [p1, ctrl0, ctrl1, p2],
                        false,
                        egui::Color32::TRANSPARENT,
                        egui::Stroke::new(2.0, conn_color),
                    );
                    painter.add(bezier);
                }

                // Track which node was clicked
                let mut clicked_node_id: Option<u32> = None;
                let click_pos = response.interact_pointer_pos();

                // Draw nodes and check for clicks
                for node in &self.graph_nodes {
                    let node_pos = egui::pos2(
                        canvas_rect.left() + node.position.0,
                        canvas_rect.top() + node.position.1,
                    );
                    let node_size = egui::vec2(80.0, 40.0);
                    let node_rect = egui::Rect::from_min_size(node_pos, node_size);

                    // Check if this node was clicked
                    if response.clicked() {
                        if let Some(pos) = click_pos {
                            if node_rect.contains(pos) {
                                clicked_node_id = Some(node.id);
                            }
                        }
                    }

                    // Node color based on type with gradient effect
                    let (node_color, header_color) = match node.node_type.as_str() {
                        "Event" => (
                            egui::Color32::from_rgb(140, 60, 60),
                            egui::Color32::from_rgb(180, 80, 80),
                        ),
                        "Function" => (
                            egui::Color32::from_rgb(60, 100, 140),
                            egui::Color32::from_rgb(80, 120, 180),
                        ),
                        "Math" => (
                            egui::Color32::from_rgb(60, 140, 60),
                            egui::Color32::from_rgb(80, 180, 80),
                        ),
                        "Variable" => (
                            egui::Color32::from_rgb(140, 100, 60),
                            egui::Color32::from_rgb(180, 140, 80),
                        ),
                        _ => (egui::Color32::from_gray(50), egui::Color32::from_gray(70)),
                    };

                    // Node body
                    painter.rect_filled(node_rect, 4.0, node_color);
                    // Node header
                    let header_rect = egui::Rect::from_min_max(
                        node_rect.min,
                        egui::pos2(node_rect.max.x, node_rect.min.y + 14.0),
                    );
                    painter.rect_filled(
                        header_rect,
                        egui::CornerRadius {
                            nw: 4,
                            ne: 4,
                            sw: 0,
                            se: 0,
                        },
                        header_color,
                    );
                    painter.rect_stroke(
                        node_rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                        egui::epaint::StrokeKind::Middle,
                    );

                    // Node type icon
                    let type_icon = match node.node_type.as_str() {
                        "Event" => "E",
                        "Function" => "ƒ",
                        "Math" => "∑",
                        "Variable" => "V",
                        _ => "•",
                    };
                    painter.text(
                        egui::pos2(node_rect.left() + 6.0, header_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        type_icon,
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );

                    // Node title
                    painter.text(
                        egui::pos2(node_rect.center().x, node_rect.center().y + 4.0),
                        egui::Align2::CENTER_CENTER,
                        &node.name,
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );

                    // Input/output pins with labels
                    let pin_radius = 4.0;
                    for (i, input) in node.inputs.iter().enumerate() {
                        let pin_y = node_rect.top() + 18.0 + i as f32 * 10.0;
                        painter.circle_filled(
                            egui::pos2(node_rect.left(), pin_y),
                            pin_radius,
                            egui::Color32::from_rgb(100, 200, 100),
                        );
                        painter.text(
                            egui::pos2(node_rect.left() + 8.0, pin_y),
                            egui::Align2::LEFT_CENTER,
                            input,
                            egui::FontId::proportional(8.0),
                            egui::Color32::from_gray(180),
                        );
                    }
                    for (i, output) in node.outputs.iter().enumerate() {
                        let pin_y = node_rect.top() + 18.0 + i as f32 * 10.0;
                        painter.circle_filled(
                            egui::pos2(node_rect.right(), pin_y),
                            pin_radius,
                            egui::Color32::from_rgb(200, 100, 100),
                        );
                        painter.text(
                            egui::pos2(node_rect.right() - 8.0, pin_y),
                            egui::Align2::RIGHT_CENTER,
                            output,
                            egui::FontId::proportional(8.0),
                            egui::Color32::from_gray(180),
                        );
                    }
                }

                // Emit event if a node was clicked
                if let Some(id) = clicked_node_id {
                    self.emit_event(PanelEvent::GraphNodeSelected(id));
                }

                ui.add_space(8.0);

                // Node list (enhanced with grouping)
                egui::CollapsingHeader::new("Node List")
                    .default_open(true)
                    .show(ui, |ui| {
                        // Group by type
                        let node_types = ["Event", "Function", "Math", "Variable"];
                        for node_type in node_types {
                            let type_nodes: Vec<_> = self
                                .graph_nodes
                                .iter()
                                .filter(|n| n.node_type == node_type)
                                .collect();
                            if !type_nodes.is_empty() {
                                ui.collapsing(
                                    format!("{} ({})", node_type, type_nodes.len()),
                                    |ui| {
                                        for node in type_nodes {
                                            ui.horizontal(|ui| {
                                                ui.label(&node.name);
                                                ui.weak(format!(
                                                    "({} in, {} out)",
                                                    node.inputs.len(),
                                                    node.outputs.len()
                                                ));
                                            });
                                        }
                                    },
                                );
                            }
                        }
                    });
            }
            PanelType::BehaviorGraph => {
                let node_count = self.behavior_graph.nodes.len();
                let connection_count = self.behavior_graph.connections.len();
                // Count node types for header info
                let action_count = self
                    .behavior_graph
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == BehaviorNodeType::Action)
                    .count();
                let condition_count = self
                    .behavior_graph
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == BehaviorNodeType::Condition)
                    .count();

                ui.horizontal(|ui| {
                    ui.heading(format!("Behavior Graph ({} nodes)", node_count));
                    if node_count > 0 {
                        ui.weak(format!(
                            "S:{} Q:{} L:{}",
                            action_count, condition_count, connection_count
                        ));
                    }
                });
                ui.separator();

                // Toolbar
                ui.horizontal(|ui| {
                    if ui.button("Sequence").clicked() {
                        let id = self.behavior_graph.nodes.len() as u32;
                        self.behavior_graph.nodes.push(BehaviorNode {
                            id,
                            name: format!("Sequence_{}", id),
                            node_type: BehaviorNodeType::Sequence,
                            position: (100.0 + id as f32 * 30.0, 50.0),
                            children: Vec::new(),
                        });
                        self.emit_event(PanelEvent::BehaviorNodeSelected(id));
                    }
                    if ui.button("Selector").clicked() {
                        let id = self.behavior_graph.nodes.len() as u32;
                        self.behavior_graph.nodes.push(BehaviorNode {
                            id,
                            name: format!("Selector_{}", id),
                            node_type: BehaviorNodeType::Selector,
                            position: (100.0 + id as f32 * 30.0, 50.0),
                            children: Vec::new(),
                        });
                        self.emit_event(PanelEvent::BehaviorNodeSelected(id));
                    }
                    if ui.button("Action").clicked() {
                        let id = self.behavior_graph.nodes.len() as u32;
                        self.behavior_graph.nodes.push(BehaviorNode {
                            id,
                            name: format!("Action_{}", id),
                            node_type: BehaviorNodeType::Action,
                            position: (100.0 + id as f32 * 30.0, 50.0),
                            children: Vec::new(),
                        });
                        self.emit_event(PanelEvent::BehaviorNodeSelected(id));
                    }
                    if ui.button("Condition").clicked() {
                        let id = self.behavior_graph.nodes.len() as u32;
                        self.behavior_graph.nodes.push(BehaviorNode {
                            id,
                            name: format!("Condition_{}", id),
                            node_type: BehaviorNodeType::Condition,
                            position: (100.0 + id as f32 * 30.0, 50.0),
                            children: Vec::new(),
                        });
                        self.emit_event(PanelEvent::BehaviorNodeSelected(id));
                    }
                });

                ui.add_space(8.0);

                // Behavior tree canvas
                let canvas_size = egui::vec2(ui.available_width().min(400.0), 250.0);
                let (canvas_rect, response) =
                    ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());
                let painter = ui.painter_at(canvas_rect);

                // Background
                painter.rect_filled(canvas_rect, 4.0, egui::Color32::from_rgb(30, 35, 40));

                // Track which node was clicked
                let mut clicked_node_id: Option<u32> = None;
                let click_pos = response.interact_pointer_pos();

                // Draw connections first
                for (from_id, to_id) in &self.behavior_graph.connections {
                    let from_node = self.behavior_graph.nodes.iter().find(|n| n.id == *from_id);
                    let to_node = self.behavior_graph.nodes.iter().find(|n| n.id == *to_id);

                    if let (Some(from), Some(to)) = (from_node, to_node) {
                        let from_pos = egui::pos2(
                            canvas_rect.left() + from.position.0 + 40.0,
                            canvas_rect.top() + from.position.1 + 30.0,
                        );
                        let to_pos = egui::pos2(
                            canvas_rect.left() + to.position.0 + 40.0,
                            canvas_rect.top() + to.position.1,
                        );

                        // Draw curved connection
                        let mid_y = (from_pos.y + to_pos.y) / 2.0;
                        let control1 = egui::pos2(from_pos.x, mid_y);
                        let control2 = egui::pos2(to_pos.x, mid_y);

                        // Approximate bezier with line segments
                        let steps = 10;
                        for i in 0..steps {
                            let t1 = i as f32 / steps as f32;
                            let t2 = (i + 1) as f32 / steps as f32;

                            let p1 = bezier_point(from_pos, control1, control2, to_pos, t1);
                            let p2 = bezier_point(from_pos, control1, control2, to_pos, t2);

                            painter.line_segment(
                                [p1, p2],
                                egui::Stroke::new(2.0, egui::Color32::from_gray(120)),
                            );
                        }
                    }
                }

                // Draw nodes and check for clicks
                for node in &self.behavior_graph.nodes {
                    let node_pos = egui::pos2(
                        canvas_rect.left() + node.position.0,
                        canvas_rect.top() + node.position.1,
                    );
                    let node_size = egui::vec2(80.0, 30.0);
                    let node_rect = egui::Rect::from_min_size(node_pos, node_size);

                    // Check if this node was clicked
                    if response.clicked() {
                        if let Some(pos) = click_pos {
                            if node_rect.contains(pos) {
                                clicked_node_id = Some(node.id);
                            }
                        }
                    }

                    // Node styling based on type
                    let (bg_color, icon) = match node.node_type {
                        BehaviorNodeType::Root => (egui::Color32::from_rgb(100, 60, 120), "◉"),
                        BehaviorNodeType::Sequence => (egui::Color32::from_rgb(60, 100, 140), ">"),
                        BehaviorNodeType::Selector => (egui::Color32::from_rgb(140, 100, 60), "?"),
                        BehaviorNodeType::Condition => (egui::Color32::from_rgb(60, 140, 100), "◇"),
                        BehaviorNodeType::Action => (egui::Color32::from_rgb(80, 80, 140), "■"),
                        BehaviorNodeType::Decorator => (egui::Color32::from_rgb(140, 80, 80), "◆"),
                    };

                    let is_selected = self.behavior_graph.selected_node == Some(node.id);
                    let stroke = if is_selected {
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 100))
                    } else {
                        egui::Stroke::new(1.0, egui::Color32::from_gray(80))
                    };

                    painter.rect_filled(node_rect, 4.0, bg_color);
                    painter.rect_stroke(node_rect, 4.0, stroke, egui::epaint::StrokeKind::Middle);

                    // Icon and name
                    painter.text(
                        egui::pos2(node_rect.left() + 8.0, node_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        icon,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                    painter.text(
                        egui::pos2(node_rect.left() + 22.0, node_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &node.name,
                        egui::FontId::proportional(10.0),
                        egui::Color32::WHITE,
                    );
                }

                // Handle node selection outside borrow
                if let Some(id) = clicked_node_id {
                    self.behavior_graph.selected_node = Some(id);
                    self.emit_event(PanelEvent::BehaviorNodeSelected(id));
                }

                ui.add_space(8.0);

                // Node inspector
                if let Some(selected_id) = self.behavior_graph.selected_node {
                    if let Some(node) = self
                        .behavior_graph
                        .nodes
                        .iter()
                        .find(|n| n.id == selected_id)
                    {
                        ui.group(|ui| {
                            ui.label(format!("Selected: {}", node.name));
                            ui.label(format!("Type: {:?}", node.node_type));
                            ui.label(format!("Children: {}", node.children.len()));
                        });
                    }
                } else {
                    ui.weak("Click a node to select it");
                }
            }
            PanelType::Charts => {
                ui.heading("Charts & Graphs");
                ui.separator();

                // Performance chart
                ui.collapsing("Performance Over Time", |ui| {
                    let chart_height = 80.0;
                    let chart_width = ui.available_width().min(300.0);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(chart_width, chart_height),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter();

                    // Background
                    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(30));

                    // Draw axes
                    painter.line_segment(
                        [
                            egui::pos2(rect.left() + 30.0, rect.bottom() - 20.0),
                            egui::pos2(rect.right() - 10.0, rect.bottom() - 20.0),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(rect.left() + 30.0, rect.top() + 10.0),
                            egui::pos2(rect.left() + 30.0, rect.bottom() - 20.0),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                    );

                    // Generate sample data
                    let data: Vec<f32> = (0..20)
                        .map(|i| {
                            let t = i as f32 / 20.0;
                            50.0 + 30.0 * (t * 6.0).sin() + 10.0 * (t * 15.0).sin()
                        })
                        .collect();

                    // Plot line
                    let plot_left = rect.left() + 35.0;
                    let plot_right = rect.right() - 15.0;
                    let plot_top = rect.top() + 15.0;
                    let plot_bottom = rect.bottom() - 25.0;

                    let points: Vec<egui::Pos2> = data
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let x = plot_left
                                + (i as f32 / (data.len() - 1) as f32) * (plot_right - plot_left);
                            let y = plot_bottom - (v / 100.0) * (plot_bottom - plot_top);
                            egui::pos2(x, y)
                        })
                        .collect();

                    for w in points.windows(2) {
                        painter.line_segment(
                            [w[0], w[1]],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 180, 255)),
                        );
                    }

                    // Draw points
                    for p in &points {
                        painter.circle_filled(*p, 3.0, egui::Color32::from_rgb(100, 180, 255));
                    }
                });

                // Pie chart
                ui.collapsing("Resource Distribution", |ui| {
                    let pie_size = 100.0;
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(pie_size + 100.0, pie_size),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter();

                    let center = egui::pos2(rect.left() + pie_size / 2.0, rect.center().y);
                    let radius = pie_size / 2.0 - 5.0;

                    // Pie segments (CPU, GPU, Memory, Other)
                    let segments = [
                        (0.35, egui::Color32::from_rgb(100, 150, 255), "CPU"),
                        (0.25, egui::Color32::from_rgb(255, 150, 100), "GPU"),
                        (0.25, egui::Color32::from_rgb(100, 255, 150), "Memory"),
                        (0.15, egui::Color32::from_rgb(200, 200, 200), "Other"),
                    ];

                    let mut angle = -std::f32::consts::FRAC_PI_2; // Start at top
                    for (portion, color, _label) in segments {
                        let sweep = portion * std::f32::consts::TAU;

                        // Draw pie segment
                        let segments_count = (sweep / 0.1).max(3.0) as usize;
                        let mut points = vec![center];
                        for i in 0..=segments_count {
                            let a = angle + (i as f32 / segments_count as f32) * sweep;
                            points.push(egui::pos2(
                                center.x + radius * a.cos(),
                                center.y + radius * a.sin(),
                            ));
                        }
                        painter.add(egui::Shape::convex_polygon(
                            points,
                            color,
                            egui::Stroke::NONE,
                        ));

                        angle += sweep;
                    }

                    // Legend
                    let mut legend_y = rect.top() + 10.0;
                    for (portion, color, label) in segments {
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(rect.left() + pie_size + 10.0, legend_y),
                                egui::vec2(12.0, 12.0),
                            ),
                            2.0,
                            color,
                        );
                        painter.text(
                            egui::pos2(rect.left() + pie_size + 26.0, legend_y + 6.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{} ({:.0}%)", label, portion * 100.0),
                            egui::FontId::proportional(10.0),
                            egui::Color32::from_gray(200),
                        );
                        legend_y += 18.0;
                    }
                });

                // Bar chart
                ui.collapsing("Entity Types", |ui| {
                    let chart_height = 100.0;
                    let chart_width = ui.available_width().min(250.0);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(chart_width, chart_height),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter();

                    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(30));

                    let bars = [
                        ("Sprites", 45),
                        ("Physics", 30),
                        ("AI", 20),
                        ("Audio", 10),
                        ("UI", 15),
                    ];

                    let bar_width = (rect.width() - 40.0) / bars.len() as f32 - 5.0;
                    let max_val = bars.iter().map(|(_, v)| *v).max().unwrap_or(1) as f32;

                    for (i, (label, value)) in bars.iter().enumerate() {
                        let bar_height = (*value as f32 / max_val) * (rect.height() - 35.0);
                        let bar_x = rect.left() + 25.0 + i as f32 * (bar_width + 5.0);
                        let bar_rect = egui::Rect::from_min_size(
                            egui::pos2(bar_x, rect.bottom() - 20.0 - bar_height),
                            egui::vec2(bar_width, bar_height),
                        );

                        let color = egui::Color32::from_rgb(
                            100 + (i * 30) as u8,
                            150 - (i * 15) as u8,
                            200,
                        );

                        painter.rect_filled(bar_rect, 2.0, color);

                        // Label
                        painter.text(
                            egui::pos2(bar_x + bar_width / 2.0, rect.bottom() - 8.0),
                            egui::Align2::CENTER_CENTER,
                            *label,
                            egui::FontId::proportional(8.0),
                            egui::Color32::from_gray(180),
                        );

                        // Value
                        painter.text(
                            egui::pos2(bar_x + bar_width / 2.0, bar_rect.top() - 4.0),
                            egui::Align2::CENTER_BOTTOM,
                            format!("{}", value),
                            egui::FontId::proportional(9.0),
                            egui::Color32::WHITE,
                        );
                    }
                });
            }
            PanelType::Terrain => {
                // Delegate to TerrainPanel implementation
                use crate::panels::Panel;
                self.terrain_panel.show(ui);

                // Check for completed terrain generation
                if self.terrain_panel.has_pending_actions() {
                    for action in self.terrain_panel.take_actions() {
                        match action {
                            crate::panels::terrain_panel::TerrainAction::Generate => {
                                self.emit_event(PanelEvent::TerrainReady);
                            }
                            crate::panels::terrain_panel::TerrainAction::BrushUpdate => {
                                self.emit_event(PanelEvent::TerrainBrushUpdate);
                            }
                            _ => {} // Other actions (seed, biome, etc.) handled internally
                        }
                    }
                }
            }
            // === New Phase 8 SOTA Panels ===
            PanelType::UiEditor => {
                use crate::panels::Panel;
                self.ui_editor_panel.show(ui);
            }
            PanelType::Foliage => {
                use crate::panels::Panel;
                self.foliage_panel.show(ui);
            }
            PanelType::SplineEditor => {
                use crate::panels::Panel;
                self.spline_editor_panel.show(ui);
            }
            PanelType::LodConfig => {
                use crate::panels::Panel;
                self.lod_config_panel.show(ui);
            }
            PanelType::Localization => {
                use crate::panels::Panel;
                self.localization_panel.show(ui);
            }
            PanelType::ProjectSettings => {
                use crate::panels::Panel;
                self.project_settings_panel.show(ui);
            }
            PanelType::Audio => {
                use crate::panels::Panel;
                self.audio_panel.show(ui);
            }
            PanelType::Cinematics => {
                use crate::panels::Panel;
                self.cinematics_panel.show(ui);
            }
            PanelType::DialogueEditor => {
                use crate::panels::Panel;
                self.dialogue_editor_panel.show(ui);
            }
            PanelType::Lighting => {
                use crate::panels::Panel;
                self.lighting_panel.show(ui);
            }
            PanelType::Navigation => {
                use crate::panels::Panel;
                self.navigation_panel.show(ui);
            }
            PanelType::Networking => {
                use crate::panels::Panel;
                self.networking_panel.show(ui);
            }
            PanelType::ParticleSystem => {
                use crate::panels::Panel;
                self.particle_system_panel.show(ui);
            }
            PanelType::Pcg => {
                use crate::panels::Panel;
                self.pcg_panel.show(ui);
            }
            PanelType::Physics => {
                use crate::panels::Panel;
                self.physics_panel.show(ui);
            }
            PanelType::PostProcess => {
                use crate::panels::Panel;
                self.post_process_panel.show(ui);
            }
            PanelType::InputBindings => {
                use crate::panels::Panel;
                self.input_bindings_panel.show(ui);
            }
            PanelType::FrameDebugger => {
                self.frame_debugger_panel.show(ui);
            }
            PanelType::BlendImport => {
                use crate::panels::Panel;
                self.blend_import_panel.show(ui);
            }
            PanelType::Blueprint => {
                use crate::panels::Panel;
                self.blueprint_panel.show(ui);
            }
            PanelType::RegionalArchetype => {
                use crate::panels::Panel;
                // Phase 1.X-F.5-paint.F-fix: panel registration to make
                // RegionalArchetypePanel reachable via View/Window menu.
                // Apply any pending paint operations to the panel-owned
                // mask before rendering so the persistence section shows
                // up-to-date status. Brush queue is fed by future
                // viewport-pointer-event wiring (out of F.5-paint.F-fix
                // scope per Q1 deferral).
                self.regional_archetype_panel.apply_pending_paint_ops_to_owned();
                self.regional_archetype_panel.show(ui);
            }
        }
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        tab.is_closable()
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        if tab.has_scroll() {
            [true, true]
        } else {
            [false, false]
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(format!("editor_panel_{:?}", tab))
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        if tab.is_closable() {
            self.panels_to_close.push(*tab);
            self.emit_event(PanelEvent::PanelClosed(*tab));
            OnCloseResponse::Close
        } else {
            OnCloseResponse::Ignore
        }
    }

    fn add_popup(
        &mut self,
        ui: &mut egui::Ui,
        _surface: egui_dock::SurfaceIndex,
        _node: egui_dock::NodeIndex,
    ) {
        ui.heading("Add Panel");
        ui.separator();

        // Search filter
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add_sized(
                [ui.available_width() - 10.0, 20.0],
                egui::TextEdit::singleline(&mut self.panel_search).hint_text("Search panels..."),
            );
        });
        ui.separator();

        let filter = self.panel_search.to_lowercase();

        // Helper to show panels with descriptions
        let mut show_panels_with_desc =
            |ui: &mut egui::Ui, header: &str, panels: &[(PanelType, &str, &str)]| {
                let matching: Vec<_> = panels
                    .iter()
                    .filter(|(p, _, _)| {
                        filter.is_empty() || p.title().to_lowercase().contains(&filter)
                    })
                    .collect();

                if !matching.is_empty() {
                    ui.collapsing(header, |ui| {
                        for (panel, desc, shortcut) in matching {
                            ui.horizontal(|ui| {
                                if ui
                                    .button(format!("{} {}", panel.icon(), panel.title()))
                                    .on_hover_text(*desc)
                                    .clicked()
                                {
                                    self.panels_to_add.push(*panel);
                                    self.emit_event(PanelEvent::AddPanel(*panel));
                                    self.panel_search.clear();
                                }
                                if !shortcut.is_empty() {
                                    ui.weak(*shortcut);
                                }
                            });
                        }
                    });
                }
            };

        // Group panels by category with descriptions and shortcuts
        show_panels_with_desc(
            ui,
            "Core",
            &[
                (
                    PanelType::Viewport,
                    "3D scene viewport for visual editing",
                    "",
                ),
                (
                    PanelType::Inspector,
                    "View and edit selected entity properties",
                    "Ctrl+I",
                ),
                (
                    PanelType::Hierarchy,
                    "Scene entity tree and organization",
                    "Ctrl+H",
                ),
                (
                    PanelType::Transform,
                    "Position, rotation, and scale controls",
                    "",
                ),
            ],
        );

        show_panels_with_desc(
            ui,
            "Assets & Scene",
            &[
                (
                    PanelType::AssetBrowser,
                    "Browse and manage project assets",
                    "Ctrl+A",
                ),
                (
                    PanelType::BlendImport,
                    "Import .blend scenes, extract assets, generate biome packs",
                    "",
                ),
                (
                    PanelType::World,
                    "Global scene settings and environment",
                    "",
                ),
                (
                    PanelType::EntityPanel,
                    "Detailed entity information view",
                    "",
                ),
                (
                    PanelType::MaterialEditor,
                    "PBR material creation and editing",
                    "",
                ),
            ],
        );

        show_panels_with_desc(
            ui,
            "Debug & Profiling",
            &[
                (
                    PanelType::Console,
                    "Log output, errors, and warnings",
                    "Ctrl+`",
                ),
                (
                    PanelType::Profiler,
                    "Frame timing and system performance",
                    "",
                ),
                (PanelType::SceneStats, "Entity counts and scene metrics", ""),
                (
                    PanelType::Performance,
                    "FPS graphs and performance grades",
                    "",
                ),
            ],
        );

        show_panels_with_desc(
            ui,
            "Visual Editors",
            &[
                (PanelType::Animation, "Timeline and keyframe animation", ""),
                (PanelType::Graph, "Visual node-based scripting", ""),
                (PanelType::BehaviorGraph, "AI behavior tree editor", ""),
                (PanelType::Charts, "Data visualization and charts", ""),
            ],
        );

        show_panels_with_desc(
            ui,
            "Tools & Settings",
            &[
                (
                    PanelType::ThemeManager,
                    "Editor appearance and preferences",
                    "",
                ),
                (
                    PanelType::BuildManager,
                    "Project build and export settings",
                    "Ctrl+B",
                ),
                (
                    PanelType::AdvancedWidgets,
                    "Demo of advanced UI widgets",
                    "",
                ),
            ],
        );

        ui.add_space(8.0);
        ui.separator();

        // Quick actions section
        ui.horizontal(|ui| {
            ui.weak("Quick:");
            if ui
                .small_button("Reset Layout")
                .on_hover_text("Reset to default panel layout")
                .clicked()
            {
                self.emit_event(PanelEvent::ResetLayout);
            }
        });

        ui.add_space(4.0);

        // Tips
        ui.weak("Tip: Drag tabs to rearrange • Right-click tab for options");
    }
}

/// Helper function for cubic bezier curve calculation
fn bezier_point(
    p0: egui::Pos2,
    p1: egui::Pos2,
    p2: egui::Pos2,
    p3: egui::Pos2,
    t: f32,
) -> egui::Pos2 {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;

    egui::pos2(
        uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
        uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tab_viewer_creation() {
        let viewer = SimpleTabViewer::new();
        assert!(viewer.selected_entity.is_none());
        assert!(!viewer.is_playing);
    }

    #[test]
    fn test_panel_type_closeable() {
        // Viewport should not be closeable
        assert!(!PanelType::Viewport.is_closable());

        // Other panels should be closeable
        assert!(PanelType::Console.is_closable());
        assert!(PanelType::Hierarchy.is_closable());
    }

    #[test]
    fn test_panel_type_scroll() {
        // Viewport should not scroll
        assert!(!PanelType::Viewport.has_scroll());

        // Graph panels should not scroll (they pan/zoom)
        assert!(!PanelType::Graph.has_scroll());
        assert!(!PanelType::BehaviorGraph.has_scroll());

        // Other panels should scroll
        assert!(PanelType::Console.has_scroll());
        assert!(PanelType::Hierarchy.has_scroll());
    }

    // ==================== PanelEvent Tests ====================

    #[test]
    fn test_panel_event_display() {
        let event = PanelEvent::PanelClosed(PanelType::Console);
        assert!(event.to_string().contains("Closed"));

        let event = PanelEvent::EntitySelected(42);
        assert!(event.to_string().contains("Selected"));

        let event = PanelEvent::TransformPositionChanged {
            entity_id: 1,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        assert!(event.to_string().contains("Transform"));
    }

    #[test]
    fn test_panel_event_category() {
        assert!(PanelEvent::PanelClosed(PanelType::Console).is_panel_event());
        assert!(PanelEvent::PanelFocused(PanelType::Console).is_panel_event());
        assert!(PanelEvent::EntitySelected(1).is_entity_event());
        assert!(PanelEvent::EntityDeselected.is_entity_event());
        assert!(PanelEvent::TransformPositionChanged {
            entity_id: 1,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
        .is_transform_event());
        assert!(PanelEvent::ViewportFocusOnSelection.is_viewport_event());
    }

    #[test]
    fn test_panel_event_entity_id() {
        assert_eq!(PanelEvent::EntitySelected(42).entity_id(), Some(42));
        assert_eq!(PanelEvent::EntityDeselected.entity_id(), None);
        assert_eq!(
            PanelEvent::TransformPositionChanged {
                entity_id: 5,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }
            .entity_id(),
            Some(5)
        );
    }

    // ==================== EditorTheme Tests ====================

    #[test]
    fn test_editor_theme_display_and_all() {
        let themes = EditorTheme::all();
        assert_eq!(themes.len(), 4);
        for theme in themes {
            assert!(!theme.name().is_empty());
            assert!(!theme.to_string().is_empty());
        }
    }

    #[test]
    fn test_editor_theme_light_dark() {
        assert!(EditorTheme::Dark.is_dark());
        assert!(!EditorTheme::Dark.is_light());
        assert!(EditorTheme::Light.is_light());
        assert!(!EditorTheme::Light.is_dark());
        assert!(EditorTheme::Nord.is_dark());
        assert!(EditorTheme::Solarized.is_light());
    }

    // ==================== BehaviorNodeType Tests ====================

    #[test]
    fn test_behavior_node_type_display_and_all() {
        let types = BehaviorNodeType::all();
        assert_eq!(types.len(), 6);
        for node_type in types {
            assert!(!node_type.name().is_empty());
            assert!(!node_type.icon().is_empty());
            assert!(!node_type.to_string().is_empty());
        }
    }

    #[test]
    fn test_behavior_node_type_default() {
        assert_eq!(BehaviorNodeType::default(), BehaviorNodeType::Action);
    }

    #[test]
    fn test_behavior_node_type_composite_leaf() {
        // Composite nodes can have children
        assert!(BehaviorNodeType::Sequence.is_composite());
        assert!(BehaviorNodeType::Selector.is_composite());
        assert!(BehaviorNodeType::Sequence.can_have_children());

        // Leaf nodes cannot have children
        assert!(BehaviorNodeType::Action.is_leaf());
        assert!(BehaviorNodeType::Condition.is_leaf());
        assert!(!BehaviorNodeType::Action.can_have_children());
    }

    // =========================================================================
    // Phase 1.X-F.5-paint.F-fix: Pattern A regression test 2 of 2.
    //
    // Asserts that EditorTabViewer::new() instantiates the
    // regional_archetype_panel field with the panel struct correctly
    // populated. Catches the failure mode where the PanelType variant
    // exists but the EditorTabViewer struct field / new() initializer is
    // missing (which would produce compile errors for the dispatch arm,
    // but a paranoid runtime check guards against future copy-paste
    // regressions where the field exists but is mis-wired).
    //
    // Combined with the panel_type.rs Pattern A test, this pair would
    // have caught F.5-paint's gap immediately at unit-test time.
    //
    // Audit reference: docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md §6.
    // =========================================================================

    #[test]
    fn editor_tab_viewer_instantiates_regional_archetype_panel() {
        use crate::panels::Panel;
        let tab_viewer = EditorTabViewer::new();
        assert_eq!(
            tab_viewer.regional_archetype_panel.name(),
            "Regional Archetypes",
            "EditorTabViewer::new() must instantiate regional_archetype_panel \
             with the F.5-paint.A panel struct. Regression catches the F.5-paint.A \
             registration gap that produced the Andrew-gate REGRESS verdict on \
             2026-05-03."
        );
        // Verify the panel was constructed in its expected default state
        // (paint mode, default brush size, no owned mask yet).
        assert_eq!(
            tab_viewer.regional_archetype_panel.brush_size_pixels,
            crate::panels::regional_archetype_panel::RegionalArchetypePanel::DEFAULT_BRUSH_SIZE_PIXELS,
        );
        assert!(
            tab_viewer.regional_archetype_panel.mask.is_none(),
            "regional_archetype_panel.mask should start None until ensure_mask / \
             load_mask_from / set_mask is called"
        );
        assert!(
            tab_viewer.regional_archetype_panel.pending_paint_ops.is_empty(),
            "regional_archetype_panel.pending_paint_ops should start empty"
        );
    }
}
