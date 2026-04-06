// Dead code cleanup (2026-04-03): reduced from 738 to 0 aw_editor warnings.
//
// Strategy: modules shared with lib.rs (where they form the public API for
// tests/benchmarks) get #[allow(dead_code)] on their `mod` declarations in
// main.rs. Items defined directly in main.rs get targeted per-item allows
// with justification comments. The lib target compiles with zero dead_code
// warnings, confirming all suppressed items are exercised through tests.
#![warn(dead_code)]

// These modules are shared with lib.rs where they form the public API used by
// tests and benchmarks. Items appear "dead" in the binary but are exercised
// through the library target. allow(dead_code) suppresses binary-only warnings.
#[allow(dead_code)]
mod animation_bridge;
#[allow(dead_code)]
mod asset_pack;
#[allow(dead_code)]
mod audio_bridge;
#[allow(dead_code)]
mod behavior_graph;
#[allow(dead_code)]
mod blend_scanner; // Blend asset discovery for blueprint zones
#[allow(dead_code)]
mod brdf_preview;
#[allow(dead_code)]
mod clipboard; // Phase 3.4 - Copy/Paste/Duplicate
#[allow(dead_code)]
mod command; // Phase 2.1 - Undo/Redo system
#[allow(dead_code)]
mod component_ui; // Phase 2.3 - Component-based inspector
#[allow(dead_code)]
mod dialogs; // Modal dialog helpers
#[allow(dead_code)]
mod editor_mode; // Phase 4.2 - Play-in-Editor
#[allow(dead_code)]
mod editor_preferences; // Phase 9 - Editor preferences persistence
#[allow(dead_code)]
mod entity_manager;
#[allow(dead_code)]
mod file_helpers;
#[allow(dead_code)]
mod file_watcher;
#[allow(dead_code)]
mod game_project; // Game project configuration (game.toml)
#[allow(dead_code)]
mod gizmo;
#[allow(dead_code)]
mod interaction; // Phase 8.1 Week 5 Day 3 - Gizmo interaction helpers (auto-tracking)
#[allow(dead_code)]
mod level_doc; // Level document types
#[allow(dead_code)]
mod material_inspector;
#[allow(dead_code)]
mod movement_scripts;
#[allow(dead_code)]
mod panels;
#[allow(dead_code)]
mod polish;
#[allow(dead_code)]
mod prefab; // Phase 4.1 - Prefab System
#[allow(dead_code)]
mod recent_files; // Phase 3 - Recent files tracking
#[allow(dead_code)]
mod runtime; // Week 4 - Deterministic runtime integration
#[allow(dead_code)]
mod scene_serialization; // Phase 2.2 - Scene Save/Load
#[allow(dead_code)]
mod scene_state; // Week 1 - Canonical edit-mode world owner
#[allow(dead_code)]
mod splash; // Startup splash screen with logo + cinematic video
#[allow(dead_code)]
mod terrain_integration; // Terrain generation integration
#[allow(dead_code)]
mod tutorial; // First-run tutorial walkthrough
#[allow(dead_code)]
mod ui; // Phase 3 - UI components (StatusBar, etc.)
#[allow(dead_code)]
mod viewport; // Phase 1.1 - 3D Viewport
#[allow(dead_code)]
mod voxel_tools; // Phase 10: Voxel editing tools // Phase 2: Asset packaging and compression

use anyhow::{Context as _, Result};
use astraweave_asset::AssetDatabase;
use astraweave_core::{Entity, IVec2, Team, World};
use astraweave_dialogue::DialogueGraph;
use astraweave_nav::NavMesh;
use astraweave_quests::Quest;
use behavior_graph::{BehaviorGraphDocument, BehaviorGraphEditorUi};
use editor_mode::EditorMode;
use eframe::egui;
use entity_manager::MaterialSlot;
use entity_manager::{EntityManager, SelectionSet};
use gizmo::snapping::SnappingConfig;
use gizmo::state::GizmoMode;
use material_inspector::MaterialInspector;
use panels::{
    AdvancedWidgetsPanel, AnimationPanel, AssetAction, AssetBrowser, BuildManagerPanel,
    ChartsPanel, ConsolePanel, EntityPanel, GraphPanel, HierarchyPanel, Panel, PerformancePanel,
    ProfilerPanel, SceneStats, SceneStatsPanel, TextureType, ThemeManagerPanel, TransformPanel,
    WorldPanel,
};
#[allow(dead_code)]
mod dock_layout;
#[allow(dead_code)]
mod dock_panels;
#[allow(dead_code)]
mod panel_type;
#[allow(dead_code)]
mod plugin;
#[allow(dead_code)]
mod tab_viewer;
use dock_layout::{DockLayout, LayoutPreset};
use panel_type::PanelType;
use prefab::{PrefabData, PrefabManager};
use recent_files::RecentFilesManager;
use runtime::{EditorRuntime, RuntimeState};
use scene_state::{EditorSceneState, TransformableScene};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tab_viewer::{EditorDrawContext, EditorTabViewer, EntityInfo};
use tracing::{debug, error, info, span, warn, Level};
use ui::StatusBar;
use ui::{AlignDirection, DistributeDirection, MenuActionHandler, MenuBar};
use uuid::Uuid;
use viewport::camera::OrbitCamera;
use viewport::ViewportWidget; // Phase 1.1

/// Convert EntityManager EntityId (u64) to World Entity (u32) with overflow check.
/// Returns None and logs a warning if the ID exceeds u32::MAX.
fn entity_id_to_world(id: u64) -> Option<Entity> {
    match u32::try_from(id) {
        Ok(eid) => Some(eid),
        Err(_) => {
            warn!(
                "EntityId {} exceeds u32::MAX — cannot map to World Entity",
                id
            );
            None
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct LevelDoc {
    title: String,
    biome: String,
    seed: u64,
    sky: Sky,
    biome_paints: Vec<BiomePaint>,
    obstacles: Vec<Obstacle>,
    npcs: Vec<NpcSpawn>,
    fate_threads: Vec<FateThread>,
    boss: BossCfg,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct Sky {
    time_of_day: String,
    weather: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum BiomePaint {
    #[serde(rename = "grass_dense")]
    GrassDense { area: Circle },
    #[serde(rename = "moss_path")]
    MossPath { polyline: Vec<[i32; 2]> },
}

#[derive(Clone, Serialize, Deserialize)]
struct Circle {
    cx: i32,
    cz: i32,
    radius: i32,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct Obstacle {
    id: String,
    pos: [f32; 3],
    yaw: f32,
    tags: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct NpcSpawn {
    archetype: String,
    count: u32,
    spawn: Spawn,
    behavior: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct Spawn {
    pos: [f32; 3],
    radius: f32,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct FateThread {
    name: String,
    triggers: Vec<Trigger>,
    ops: Vec<DirectorOp>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum Trigger {
    #[serde(rename = "enter_area")]
    EnterArea { center: [f32; 3], radius: f32 },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
enum DirectorOp {
    Fortify {
        area: FortRegion,
    },
    Collapse {
        area: FortRegion,
    },
    SpawnWave {
        archetype: String,
        count: u32,
        scatter: f32,
    },
}

#[derive(Clone, Serialize, Deserialize)]
struct FortRegion {
    cx: i32,
    cz: i32,
    r: i32,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct BossCfg {
    director_budget_script: String,
    phase_script: String,
}

#[derive(Clone, Debug)]
struct BehaviorGraphBinding {
    entity: Entity,
    name: String,
}

impl BehaviorGraphBinding {
    fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
        }
    }
}

/// Simple asset registry for tracking loaded assets
#[derive(Default)]
struct AssetRegistry {
    count: usize,
}

impl AssetRegistry {
    fn count(&self) -> usize {
        self.count
    }

    #[allow(dead_code)]
    fn set_count(&mut self, count: usize) {
        self.count = count;
    }
}

// Removed local definition of DistributeDirection (moved to ui::menu_bar)

/// Week 4 Day 5: Asset validation result for import operations
#[derive(Default)]
struct AssetValidation {
    is_valid: bool,
    warnings: Vec<String>,
    info: Vec<String>,
}

/// Message from background blend decomposition thread.
enum DecompThreadMsg {
    /// Intermediate progress update (progress 0..1, message)
    Progress(f32, String),
    /// Final result: success with data
    Done(DecompThreadResult),
    /// Final result: error
    Failed(String),
}

/// Successful decomposition result data.
struct DecompThreadResult {
    assets: Vec<panels::blend_import_panel::DecomposedAssetEntry>,
    hdri_paths: Vec<PathBuf>,
    ground_texture_groups: Vec<(String, Vec<PathBuf>)>,
    status_message: String,
}

#[allow(dead_code)] // Panels/managers initialized for dock system; wired through tab viewer
struct EditorApp {
    content_root: PathBuf,
    level: LevelDoc,
    status: String,
    mat_doc: MaterialLiveDoc,
    asset_db: AssetDatabase,
    behavior_graph_doc: BehaviorGraphDocument,
    behavior_graph_ui: BehaviorGraphEditorUi,
    behavior_graph_binding: Option<BehaviorGraphBinding>,
    dialogue_graph: DialogueGraph,
    quest_graph: Quest,
    console_logs: Vec<String>,
    profiler_data: Vec<String>,
    last_runtime_log: std::time::Instant,
    runtime: EditorRuntime,
    terrain_grid: Vec<Vec<String>>,
    selected_biome: String,
    nav_mesh: NavMesh,
    nav_max_step: f32,
    nav_max_slope_deg: f32,
    scene_state: Option<EditorSceneState>,
    material_inspector: MaterialInspector, // NEW - Phase PBR-G Task 2
    // Phase 1: Entity management
    entity_manager: EntityManager,
    selected_entity: Option<u64>,
    // Phase 2.1: Undo/Redo system
    undo_stack: command::UndoStack,
    /// Side-channel for terrain brush undo/redo actions
    terrain_undo_queue: command::TerrainUndoQueue,
    // Phase 2.2: Scene Save/Load
    current_scene_path: Option<PathBuf>,
    // Phase 3.4: Copy/Paste/Duplicate
    clipboard: Option<clipboard::ClipboardData>,
    // Astract panels
    world_panel: WorldPanel,
    entity_panel: EntityPanel,
    performance_panel: PerformancePanel,
    charts_panel: ChartsPanel,
    advanced_widgets_panel: AdvancedWidgetsPanel,
    graph_panel: GraphPanel,
    animation_panel: AnimationPanel,
    transform_panel: TransformPanel,
    asset_browser: AssetBrowser,
    hierarchy_panel: HierarchyPanel,
    // Legacy hierarchy search (was incorrectly using level.title)
    legacy_hierarchy_search: String,
    // Phase 5.2: Build Manager
    build_manager_panel: BuildManagerPanel,
    // Enhanced Console Panel
    console_panel: ConsolePanel,
    // Scene Statistics Panel
    scene_stats_panel: SceneStatsPanel,
    // Performance Profiler Panel
    profiler_panel: ProfilerPanel,
    // Phase 5.3: Plugin System
    plugin_manager: plugin::PluginManager,
    plugin_panel: plugin::PluginManagerPanel,
    // Phase 5.5: Theme & Layout Manager
    theme_manager: ThemeManagerPanel,
    // 3D Viewport (Phase 1.1 - Babylon.js-style editor)
    viewport: Option<ViewportWidget>,
    // Phase 3.5: StatusBar tracking
    current_gizmo_mode: GizmoMode,
    selection_set: SelectionSet,
    snapping_config: SnappingConfig,
    last_frame_time: std::time::Instant,
    current_fps: f32,
    recent_files: RecentFilesManager,
    // Phase 4.1: Prefab System
    prefab_manager: PrefabManager,
    // Phase 4.2: Play-in-Editor
    editor_mode: EditorMode,
    // Hot-reload file watcher
    file_watcher: Option<file_watcher::FileWatcher>,
    // Phase 6: Dirty flag for unsaved changes
    is_dirty: bool,
    show_quit_dialog: bool,
    /// Set to true when user confirms quit (prevents close handler from re-canceling)
    pending_quit: bool,
    // Phase 6: Toast notifications (Week 6 Day 3-4: Enhanced toast manager)
    toast_manager: ui::ToastManager,
    // Phase 7: Help dialog
    show_help_dialog: bool,
    show_about_dialog: bool,
    // Phase 8: Viewport settings
    show_grid: bool,
    // Phase 8: Auto-save
    auto_save_enabled: bool,
    auto_save_interval_secs: f32,
    last_auto_save: std::time::Instant,
    // Week 7: Enhanced auto-save settings
    auto_save_keep_count: usize,
    auto_save_to_separate_dir: bool,
    // Phase 9: Settings dialog
    show_settings_dialog: bool,
    // Phase 9: Panel visibility
    show_hierarchy_panel: bool,
    show_inspector_panel: bool,
    show_console_panel: bool,
    // Phase 10: Confirm dialog for new scene
    show_new_confirm_dialog: bool,
    // Week 7: Confirm dialog for opening scene when dirty
    show_open_confirm_dialog: bool,
    pending_open_path: Option<std::path::PathBuf>,
    // Week 7 Day 5: Crash recovery
    show_recovery_dialog: bool,
    recovery_autosave_path: Option<std::path::PathBuf>,
    lock_file_path: PathBuf,
    // Phase 10: Voxel editing tools
    voxel_editor: voxel_tools::VoxelEditor,
    // Phase 11: Professional Docking System
    dock_layout: DockLayout,
    dock_tab_viewer: EditorTabViewer,
    use_docking: bool,
    /// Asset registry for counting loaded assets
    asset_registry: AssetRegistry,
    /// Last save timestamp
    last_save_time: Option<String>,
    /// Week 6: Progress manager for long-running operations
    progress_manager: ui::ProgressManager,
    /// Week 6 Day 5: Resource usage tracking
    resource_usage: ui::ResourceUsage,
    /// Last time resources were sampled
    last_resource_sample: std::time::Instant,
    /// Startup splash screen (Some while active, None after transition)
    splash: Option<splash::SplashScreen>,
    /// Cached environment params to avoid redundant GPU updates each frame
    cached_fog_params: Option<crate::viewport::types::TerrainFogParams>,
    cached_sky_colors: Option<([f32; 4], [f32; 4], [f32; 4])>,
    /// Measured subsystem timings (from previous frame, in ms)
    measured_render_ms: f32,
    measured_tick_ms: f32,
    measured_ui_setup_ms: f32,
    /// Audio bridge: owns the audio engine and processes panel actions
    audio_bridge: audio_bridge::EditorAudioBridge,
    /// Animation bridge: owns clip library and per-entity animation state
    animation_bridge: animation_bridge::EditorAnimationBridge,
    /// Movement script system: ticks entity movement behaviors in play mode
    movement_system: movement_scripts::MovementSystem,
    /// Prefab editing: path to the prefab being edited in-place (None = normal scene)
    editing_prefab_path: Option<PathBuf>,
    /// Multi-viewport layout mode (Single, SideBySide, TopBottom, Quad)
    viewport_layout: crate::viewport::ViewportLayout,
    /// Additional viewports for multi-viewport layouts (up to 3 extra)
    extra_viewports: Vec<ViewportWidget>,
    /// Cached cursor ground-plane position for position-aware asset dropping
    last_cursor_ground_pos: Option<(i32, i32)>,
    /// World creation wizard (modal dialog)
    world_wizard: panels::WorldWizard,
    /// First-run tutorial walkthrough overlay
    tutorial: tutorial::Tutorial,
    /// Background blend decomposition receiver
    decomp_receiver: Option<std::sync::mpsc::Receiver<DecompThreadMsg>>,
}

impl Default for EditorApp {
    fn default() -> Self {
        let mut asset_db = AssetDatabase::new();
        // Try the fast path: load cached manifest (instant).
        // If no manifest exists, start with empty DB — scanning 10.9 GB / 112K files
        // at startup is prohibitive. User can trigger scan via Asset Inspector.
        if let Ok(()) = asset_db.load_manifest(&PathBuf::from("assets/assets.json")) {
            tracing::info!("Asset manifest loaded: {} assets", asset_db.assets.len());
        } else {
            tracing::info!("No asset manifest found — starting with empty asset DB (use Asset Inspector to scan)");
        }

        let prefs = editor_preferences::EditorPreferences::load();

        Self {
            content_root: PathBuf::from("content"),
            level: LevelDoc {
                title: "Untitled".into(),
                biome: "temperate_forest".into(),
                seed: 42,
                sky: Sky {
                    time_of_day: "dawn".into(),
                    weather: "clear".into(),
                },
                ..Default::default()
            },
            status: "Ready".into(),
            mat_doc: MaterialLiveDoc {
                base_color: [1.0, 1.0, 1.0, 1.0],
                metallic: 0.1,
                roughness: 0.6,
                texture_path: None,
            },
            asset_db,
            behavior_graph_doc: BehaviorGraphDocument::new_default(),
            behavior_graph_ui: BehaviorGraphEditorUi::default(),
            behavior_graph_binding: None,
            dialogue_graph: DialogueGraph {
                nodes: vec![astraweave_dialogue::DialogueNode {
                    id: "start".into(),
                    text: "Hello!".into(),
                    responses: vec![astraweave_dialogue::DialogueResponse {
                        text: "Hi!".into(),
                        next_id: None,
                    }],
                }],
            },
            quest_graph: Quest {
                title: "Sample Quest".into(),
                steps: vec![astraweave_quests::QuestStep {
                    description: "Talk to elder.".into(),
                    completed: false,
                }],
            },
            console_logs: vec!["Editor started.".into()],
            profiler_data: vec![],
            last_runtime_log: std::time::Instant::now(),
            runtime: EditorRuntime::new(),
            terrain_grid: vec![vec!["grass".into(); 10]; 10],
            selected_biome: "grass".into(),
            nav_mesh: NavMesh::bake(&[], 0.4, 60.0),
            nav_max_step: 0.4,
            nav_max_slope_deg: 60.0,
            scene_state: None, // Created in EditorApp::new() — avoid double init
            material_inspector: MaterialInspector::new(), // NEW - Phase PBR-G Task 2
            // Phase 1: Entity management
            entity_manager: EntityManager::new(),
            selected_entity: None,
            // Phase 2.1: Undo/Redo system
            undo_stack: command::UndoStack::new(100), // Store last 100 commands
            terrain_undo_queue: command::new_terrain_undo_queue(),
            // Phase 2.2: Scene Save/Load
            current_scene_path: None,
            // Phase 3.4: Copy/Paste/Duplicate
            clipboard: None,
            // Initialize Astract panels
            world_panel: WorldPanel::new(),
            entity_panel: EntityPanel::new(),
            performance_panel: PerformancePanel::new(),
            charts_panel: ChartsPanel::new(),
            advanced_widgets_panel: AdvancedWidgetsPanel::new(),
            graph_panel: GraphPanel::new(),
            animation_panel: AnimationPanel::default(),
            transform_panel: TransformPanel::new(),
            asset_browser: AssetBrowser::new(PathBuf::from("assets")),
            hierarchy_panel: HierarchyPanel::new(),
            legacy_hierarchy_search: String::new(),
            // Phase 5.2: Build Manager
            build_manager_panel: BuildManagerPanel::new(),
            // Enhanced Console Panel
            console_panel: ConsolePanel::new(),
            // Scene Statistics Panel
            scene_stats_panel: SceneStatsPanel::new(),
            // Performance Profiler Panel
            profiler_panel: ProfilerPanel::new(),
            // Phase 5.3: Plugin System
            plugin_manager: plugin::PluginManager::default(),
            plugin_panel: plugin::PluginManagerPanel::default(),
            // Phase 5.5: Theme & Layout Manager
            theme_manager: ThemeManagerPanel::new(),
            // Viewport initialized in new() method (requires CreationContext)
            viewport: None,
            // Phase 3.5: StatusBar state
            current_gizmo_mode: GizmoMode::Inactive,
            selection_set: SelectionSet::new(),
            snapping_config: SnappingConfig::default(),
            last_frame_time: std::time::Instant::now(),
            current_fps: 60.0,
            recent_files: RecentFilesManager::load(),
            // Phase 4.1: Prefab System
            prefab_manager: PrefabManager::new("prefabs"),
            // Phase 4.2: Play-in-Editor
            editor_mode: EditorMode::default(),
            // Hot-reload file watcher
            // Defer file watcher setup — recursively watching 112K files is expensive.
            // It will be created lazily on first frame instead.
            file_watcher: None,
            // Phase 6: Dirty flag for unsaved changes
            is_dirty: false,
            show_quit_dialog: false,
            pending_quit: false,
            // Phase 6: Toast notifications (Week 6 Day 3-4: Enhanced toast manager)
            toast_manager: ui::ToastManager::new(),
            // Phase 7: Help dialog
            show_help_dialog: false,
            show_about_dialog: false,
            // Phase 8: Viewport settings
            show_grid: prefs.show_grid,
            // Phase 8: Auto-save
            auto_save_enabled: prefs.auto_save_enabled,
            auto_save_interval_secs: prefs.auto_save_interval_secs,
            last_auto_save: std::time::Instant::now(),
            // Week 7: Enhanced auto-save settings
            auto_save_keep_count: prefs.auto_save_keep_count,
            auto_save_to_separate_dir: prefs.auto_save_to_separate_dir,
            // Phase 9: Settings dialog
            show_settings_dialog: false,
            // Phase 9: Panel visibility
            show_hierarchy_panel: prefs.show_hierarchy_panel,
            show_inspector_panel: prefs.show_inspector_panel,
            show_console_panel: prefs.show_console_panel,
            // Phase 10: Confirm dialog for new scene
            show_new_confirm_dialog: false,
            // Week 7: Confirm dialog for opening scene when dirty
            show_open_confirm_dialog: false,
            pending_open_path: None,
            // Week 7 Day 5: Crash recovery
            show_recovery_dialog: false,
            recovery_autosave_path: None,
            lock_file_path: PathBuf::from(".aw_editor.lock"),
            // Phase 10: Voxel editing tools
            voxel_editor: voxel_tools::VoxelEditor::new(),
            // Phase 11: Professional Docking System
            dock_layout: prefs
                .layout_json
                .as_deref()
                .and_then(|json| DockLayout::from_json(json).ok())
                .unwrap_or_else(|| DockLayout::from_preset(LayoutPreset::Default)),
            dock_tab_viewer: EditorTabViewer::new(),
            use_docking: true, // Re-enabled after fixing layout gap
            asset_registry: AssetRegistry::default(),
            last_save_time: None,
            // Week 6: Progress manager for long-running operations
            progress_manager: ui::ProgressManager::new(),
            // Week 6 Day 5: Resource usage tracking
            resource_usage: ui::ResourceUsage::new(),
            last_resource_sample: std::time::Instant::now(),
            splash: Some(splash::SplashScreen::new()),
            cached_fog_params: None,
            cached_sky_colors: None,
            measured_render_ms: 0.0,
            measured_tick_ms: 0.0,
            measured_ui_setup_ms: 0.0,
            audio_bridge: audio_bridge::EditorAudioBridge::new(),
            animation_bridge: animation_bridge::EditorAnimationBridge::new(),
            movement_system: movement_scripts::MovementSystem::new(),
            editing_prefab_path: None,
            viewport_layout: crate::viewport::ViewportLayout::default(),
            extra_viewports: Vec::new(),
            last_cursor_ground_pos: None,
            world_wizard: panels::WorldWizard::new(),
            tutorial: {
                let mut t = tutorial::Tutorial::new();
                if !prefs.tutorial_completed {
                    t.start();
                }
                t
            },
            decomp_receiver: None,
        }
    }
}

#[allow(dead_code)] // Helper methods for toast/progress/world access — used as editor features grow
impl EditorApp {
    fn edit_world(&self) -> Option<&World> {
        self.scene_state.as_ref().map(|state| state.world())
    }

    fn edit_world_mut(&mut self) -> Option<&mut World> {
        self.scene_state.as_mut().map(|state| state.world_mut())
    }

    fn active_world(&self) -> Option<&World> {
        if self.runtime.state() == RuntimeState::Editing {
            self.edit_world()
        } else {
            self.runtime.sim_world()
        }
    }

    // Week 6 Day 3-4: Enhanced toast notification methods
    fn toast_success(&mut self, message: impl Into<String>) {
        self.toast_manager.success(message);
    }

    fn toast_error(&mut self, message: impl Into<String>) {
        self.toast_manager.error(message);
    }

    fn toast_info(&mut self, message: impl Into<String>) {
        self.toast_manager.info(message);
    }

    fn toast_warning(&mut self, message: impl Into<String>) {
        self.toast_manager.warning(message);
    }

    /// Show success toast with undo action
    fn toast_success_with_undo(&mut self, message: impl Into<String>) {
        self.toast_manager.success_with_undo(message);
    }

    /// Show error toast with retry action
    fn toast_error_with_retry(&mut self, message: impl Into<String>) {
        self.toast_manager.error_with_retry(message);
    }

    // Week 6: Progress tracking helper methods

    /// Start a new progress task
    fn start_progress(
        &mut self,
        label: impl Into<String>,
        category: ui::TaskCategory,
    ) -> ui::TaskId {
        self.progress_manager.start_task(label, category)
    }

    /// Start a cancellable progress task
    fn start_progress_cancellable(
        &mut self,
        label: impl Into<String>,
        category: ui::TaskCategory,
    ) -> ui::TaskId {
        self.progress_manager
            .start_cancellable_task(label, category)
    }

    /// Update progress for a task (0.0 - 1.0)
    fn update_progress(&mut self, task_id: ui::TaskId, progress: f32, status: impl Into<String>) {
        self.progress_manager.update(task_id, progress, status);
    }

    /// Complete a progress task
    fn complete_progress(&mut self, task_id: ui::TaskId) {
        self.progress_manager.complete_task(task_id);
    }

    /// Fail a progress task with an error
    fn fail_progress(&mut self, task_id: ui::TaskId, error: impl Into<String>) {
        self.progress_manager.fail_task(task_id, error);
    }

    /// Week 6 Day 5: Sample current resource usage
    ///
    /// This provides rough estimates of memory and GPU usage.
    /// For precise monitoring, consider using platform-specific APIs.
    fn sample_resource_usage(&mut self) {
        // Estimate memory usage from allocator
        // Note: This is a rough estimate. Real implementation would use:
        // - sysinfo crate for system memory
        // - wgpu adapter info for GPU memory

        // For now, use process-level estimates
        #[cfg(windows)]
        {
            use std::mem::MaybeUninit;

            // Get process memory info on Windows
            #[repr(C)]
            struct ProcessMemoryCounters {
                cb: u32,
                page_fault_count: u32,
                peak_working_set_size: usize,
                working_set_size: usize,
                quota_peak_paged_pool_usage: usize,
                quota_paged_pool_usage: usize,
                quota_peak_non_paged_pool_usage: usize,
                quota_non_paged_pool_usage: usize,
                pagefile_usage: usize,
                peak_pagefile_usage: usize,
            }

            #[link(name = "psapi")]
            extern "system" {
                fn GetProcessMemoryInfo(
                    process: *mut std::ffi::c_void,
                    counters: *mut ProcessMemoryCounters,
                    cb: u32,
                ) -> i32;
                fn GetCurrentProcess() -> *mut std::ffi::c_void;
            }

            #[repr(C)]
            struct MemoryStatusEx {
                length: u32,
                memory_load: u32,
                total_phys: u64,
                avail_phys: u64,
                total_page_file: u64,
                avail_page_file: u64,
                total_virtual: u64,
                avail_virtual: u64,
                avail_extended_virtual: u64,
            }

            #[link(name = "kernel32")]
            extern "system" {
                fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
            }

            // SAFETY: Calling Win32 `GetProcessMemoryInfo` and `GlobalMemoryStatusEx`
            // with properly sized and initialized structs. These are safe Win32 API calls
            // that only read process memory statistics.
            unsafe {
                let mut counters = MaybeUninit::<ProcessMemoryCounters>::uninit();
                (*counters.as_mut_ptr()).cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;

                if GetProcessMemoryInfo(
                    GetCurrentProcess(),
                    counters.as_mut_ptr(),
                    std::mem::size_of::<ProcessMemoryCounters>() as u32,
                ) != 0
                {
                    let counters = counters.assume_init();
                    self.resource_usage.memory_used = counters.working_set_size as u64;
                }

                // Get system memory
                let mut status = MaybeUninit::<MemoryStatusEx>::uninit();
                (*status.as_mut_ptr()).length = std::mem::size_of::<MemoryStatusEx>() as u32;

                if GlobalMemoryStatusEx(status.as_mut_ptr()) != 0 {
                    let status = status.assume_init();
                    self.resource_usage.memory_total = status.total_phys;
                }
            }
        }

        #[cfg(not(windows))]
        {
            // Fallback: just use rough estimates
            // Real implementation would use sysinfo crate
            self.resource_usage.memory_used = 0;
            self.resource_usage.memory_total = 0;
        }

        // GPU stats would come from wgpu adapter/device
        // For now, leave as 0 (placeholder)
        // Real implementation:
        // - wgpu::Adapter::get_info() for basic info
        // - Platform-specific APIs for detailed GPU memory
        self.resource_usage.gpu_memory_used = 0;
        self.resource_usage.gpu_memory_total = 0;
        self.resource_usage.gpu_utilization = 0.0;
    }

    fn save_preferences(&self) {
        let prefs = editor_preferences::EditorPreferences {
            show_grid: self.show_grid,
            auto_save_enabled: self.auto_save_enabled,
            auto_save_interval_secs: self.auto_save_interval_secs,
            // Week 7: Enhanced auto-save settings
            auto_save_keep_count: self.auto_save_keep_count,
            auto_save_to_separate_dir: self.auto_save_to_separate_dir,
            show_hierarchy_panel: self.show_hierarchy_panel,
            show_inspector_panel: self.show_inspector_panel,
            show_console_panel: self.show_console_panel,
            camera: self.viewport.as_ref().map(|v| v.camera().clone()),
            snapping: Some(self.snapping_config),
            layout_json: self.dock_layout.to_json().ok(),
            tutorial_completed: !self.tutorial.active,
            blend_asset_directories: editor_preferences::default_blend_asset_directories(),
        };
        prefs.save();
    }

    fn log(&mut self, message: impl Into<String>) {
        use std::time::SystemTime;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs() % 86400;
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;
        let timestamp = format!("[{:02}:{:02}:{:02}]", hours, mins, secs);
        self.console_logs
            .push(format!("{} {}", timestamp, message.into()));
    }

    fn create_new_scene(&mut self) {
        let viewport = self.viewport.take();
        let prefs = editor_preferences::EditorPreferences {
            show_grid: self.show_grid,
            auto_save_enabled: self.auto_save_enabled,
            auto_save_interval_secs: self.auto_save_interval_secs,
            // Week 7: Enhanced auto-save settings
            auto_save_keep_count: self.auto_save_keep_count,
            auto_save_to_separate_dir: self.auto_save_to_separate_dir,
            show_hierarchy_panel: self.show_hierarchy_panel,
            show_inspector_panel: self.show_inspector_panel,
            show_console_panel: self.show_console_panel,
            camera: viewport.as_ref().map(|v| v.camera().clone()),
            snapping: Some(self.snapping_config),
            layout_json: self.dock_layout.to_json().ok(),
            tutorial_completed: !self.tutorial.active,
            blend_asset_directories: editor_preferences::default_blend_asset_directories(),
        };
        *self = Self::default();
        self.viewport = viewport;
        self.show_grid = prefs.show_grid;
        self.auto_save_enabled = prefs.auto_save_enabled;
        self.auto_save_interval_secs = prefs.auto_save_interval_secs;
        self.auto_save_keep_count = prefs.auto_save_keep_count;
        self.auto_save_to_separate_dir = prefs.auto_save_to_separate_dir;
        self.show_hierarchy_panel = prefs.show_hierarchy_panel;
        self.show_inspector_panel = prefs.show_inspector_panel;
        self.show_console_panel = prefs.show_console_panel;
        if let Some(snapping) = prefs.snapping {
            self.snapping_config = snapping;
            if let Some(v) = &mut self.viewport {
                v.set_snapping_config(snapping);
            }
        }
        self.scene_state = Some(scene_state::EditorSceneState::new(World::new()));
        self.console_logs.push("New scene created".into());
        self.status = "New scene created".into();
    }

    /// Week 7: Load scene from path with proper state management
    fn load_scene_from_path(&mut self, path: &std::path::Path) {
        let scene_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("scene")
            .to_string();

        self.status = format!("Loading scene {}...", scene_name);
        self.log(format!("Loading scene: {}...", scene_name));

        match scene_serialization::load_scene(path) {
            Ok(loaded_world) => {
                // Clear old scene state and prefab instances to prevent memory leaks
                self.prefab_manager.clear_instances();
                self.undo_stack.clear();

                // Sync World entities into EntityManager so they appear in hierarchy
                self.sync_entity_manager_from_world(&loaded_world);

                self.scene_state = Some(EditorSceneState::new(loaded_world));
                self.current_scene_path = Some(path.to_path_buf());
                self.is_dirty = false;

                // Sync hierarchy panel with loaded world
                if let Some(scene_state) = self.scene_state.as_mut() {
                    self.hierarchy_panel
                        .sync_with_world(scene_state.world_mut());
                }

                // Rebuild parent-child hierarchy from EntityManager
                let parent_map: Vec<(u32, Option<u32>)> = self
                    .entity_manager
                    .entities()
                    .iter()
                    .filter_map(|(&id, entity)| {
                        entity_id_to_world(id)
                            .map(|eid| (eid, entity.parent.and_then(entity_id_to_world)))
                    })
                    .collect();
                self.hierarchy_panel.rebuild_from_parents(&parent_map);

                info!("Loaded scene: {}", scene_name);
                self.toast_success(format!("Loaded scene: {}", scene_name));
                self.log(format!("Loaded scene: {}", scene_name));
                self.status = format!("Loaded: {}", scene_name);

                // Add to recent files
                self.recent_files.add_file(path.to_path_buf());
            }
            Err(err) => {
                error!("Failed to load scene: {}", err);
                self.toast_error(format!("Failed to load: {}", scene_name));
                self.log(format!("Failed to load scene: {}", err));
                self.status = format!("Load failed: {}", scene_name);
            }
        }
    }

    /// Populate EntityManager from World entities so they appear in the hierarchy
    fn sync_entity_manager_from_world(&mut self, world: &astraweave_core::World) {
        self.entity_manager.clear();
        self.selected_entity = None;
        self.selection_set.primary = None;

        for entity_id in world.entities() {
            let name = world.name(entity_id).unwrap_or("Entity").to_string();
            let em_id: u64 = entity_id.into();

            let mut editor_entity = entity_manager::EditorEntity::new(em_id, name);

            // Populate position from World pose
            if let Some(pose) = world.pose(entity_id) {
                editor_entity.position =
                    glam::Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32);
                editor_entity.components.insert(
                    "Transform".to_string(),
                    serde_json::json!({"x": pose.pos.x, "y": pose.height, "z": pose.pos.y}),
                );
            }

            // Populate Health component
            if let Some(health) = world.health(entity_id) {
                editor_entity
                    .components
                    .insert("Health".to_string(), serde_json::json!({"hp": health.hp}));
            }

            // Populate Team component
            if let Some(team) = world.team(entity_id) {
                editor_entity
                    .components
                    .insert("Team".to_string(), serde_json::json!({"id": team.id}));
            }

            // Populate Ammo component
            if let Some(ammo) = world.ammo(entity_id) {
                editor_entity.components.insert(
                    "Ammo".to_string(),
                    serde_json::json!({"count": ammo.rounds}),
                );
            }

            self.entity_manager.add(editor_entity);
        }

        let count = world.entities().len();
        self.console_logs
            .push(format!("Synced {} entities from loaded scene", count));
    }

    /// Week 7: Request to open a scene, shows confirmation if dirty
    fn request_open_scene(&mut self, path: std::path::PathBuf) {
        if self.is_dirty {
            self.pending_open_path = Some(path);
            self.show_open_confirm_dialog = true;
        } else {
            self.load_scene_from_path(&path);
        }
    }

    /// Week 7: Enhanced auto-save with timestamped backups in .autosave/ directory
    fn perform_auto_save(&mut self) {
        if let Some(world) = self.edit_world() {
            let auto_save_path = if self.auto_save_to_separate_dir {
                // Save to .autosave/ directory with timestamp
                let autosave_dir = self.content_root.join(".autosave");
                if let Err(e) = fs::create_dir_all(&autosave_dir) {
                    self.toast_error(format!("Failed to create autosave dir: {}", e));
                    return;
                }

                // Generate timestamped filename
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let base_name = self
                    .current_scene_path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled");
                let filename = format!("{}_{}.autosave.scene.ron", base_name, timestamp);
                autosave_dir.join(filename)
            } else {
                // Save to scene path or default
                self.current_scene_path
                    .clone()
                    .unwrap_or_else(|| self.content_root.join("scenes/autosave.scene.ron"))
            };

            match scene_serialization::save_scene(world, &auto_save_path) {
                Ok(()) => {
                    self.last_auto_save = std::time::Instant::now();
                    let filename = auto_save_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    self.toast_info(format!("Auto-saved: {}", filename));
                    self.log(format!("Auto-saved to {}", filename));

                    // Cleanup old auto-saves if using separate directory
                    if self.auto_save_to_separate_dir {
                        self.cleanup_old_autosaves();
                    }
                }
                Err(e) => {
                    self.toast_error(format!("Auto-save failed: {}", e));
                    self.log(format!("Auto-save failed: {}", e));
                }
            }
        }
    }

    /// Week 7: Remove old auto-save files, keeping only the most recent N
    fn cleanup_old_autosaves(&mut self) {
        let autosave_dir = self.content_root.join(".autosave");
        if !autosave_dir.exists() {
            return;
        }

        // Get the base name of the current scene
        let base_name = self
            .current_scene_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("untitled");

        // Collect matching auto-save files
        let mut autosaves: Vec<_> = match fs::read_dir(&autosave_dir) {
            Ok(entries) => entries
                .filter_map(|e| {
                    e.map_err(|err| tracing::debug!("Autosave dir entry error: {}", err))
                        .ok()
                })
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.starts_with(base_name) && s.ends_with(".autosave.scene.ron"))
                        .unwrap_or(false)
                })
                .collect(),
            Err(_) => return,
        };

        // Sort by modification time (newest first)
        autosaves.sort_by(|a, b| {
            let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        // Remove older files beyond keep_count
        for old_file in autosaves.iter().skip(self.auto_save_keep_count) {
            if let Err(e) = fs::remove_file(old_file) {
                self.log(format!(
                    "Failed to remove old autosave {:?}: {}",
                    old_file.file_name().unwrap_or_default(),
                    e
                ));
            } else {
                self.log(format!(
                    "Removed old autosave: {:?}",
                    old_file.file_name().unwrap_or_default()
                ));
            }
        }
    }

    /// Week 7: Get the most recent auto-save file for crash recovery
    pub fn get_most_recent_autosave(&self) -> Option<std::path::PathBuf> {
        let autosave_dir = self.content_root.join(".autosave");
        if !autosave_dir.exists() {
            return None;
        }

        // Collect all auto-save files
        let mut autosaves: Vec<_> = match fs::read_dir(&autosave_dir) {
            Ok(entries) => entries
                .filter_map(|e| {
                    e.map_err(|err| tracing::debug!("Autosave dir entry error: {}", err))
                        .ok()
                })
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s == "ron")
                        .unwrap_or(false)
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.contains(".autosave."))
                            .unwrap_or(false)
                })
                .collect(),
            Err(_) => return None,
        };

        // Sort by modification time (newest first)
        autosaves.sort_by(|a, b| {
            let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        autosaves.into_iter().next()
    }

    /// Week 7 Day 5: Create lock file to detect crashes
    fn create_lock_file(&self) {
        use std::io::Write;

        // Create or overwrite lock file with current session info
        if let Ok(mut file) = fs::File::create(&self.lock_file_path) {
            let session_info = format!(
                "pid={}\nstarted={}\nscene={}\n",
                std::process::id(),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                self.current_scene_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("Untitled")
            );
            let _ = file.write_all(session_info.as_bytes());
        }
    }

    /// Week 7 Day 5: Remove lock file on clean exit
    fn remove_lock_file(&self) {
        let _ = fs::remove_file(&self.lock_file_path);
    }

    /// Week 7 Day 5: Check if previous session crashed
    fn check_for_crash_recovery(&mut self) {
        // If lock file exists, previous session may have crashed
        if self.lock_file_path.exists() {
            // Check if there's an auto-save to recover
            if let Some(autosave_path) = self.get_most_recent_autosave() {
                // Read lock file for session info
                let session_info = fs::read_to_string(&self.lock_file_path).unwrap_or_default();

                self.console_logs.push(format!(
                    "Previous session may have crashed. Found auto-save: {}",
                    autosave_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                ));

                // Log session info if available
                if !session_info.is_empty() {
                    for line in session_info.lines().take(3) {
                        self.console_logs.push(format!("   {}", line));
                    }
                }

                self.recovery_autosave_path = Some(autosave_path);
                self.show_recovery_dialog = true;
            } else {
                // Lock file exists but no auto-save - just clean up
                self.console_logs
                    .push("Previous session may have crashed (no auto-save found)".to_string());
                self.remove_lock_file();
            }
        }
    }

    /// Week 7 Day 5: Load from recovery auto-save
    fn recover_from_autosave(&mut self) {
        if let Some(path) = self.recovery_autosave_path.take() {
            // Load the auto-save file
            self.load_scene_from_path(&path);
            self.toast_manager.success(format!(
                "Recovered from auto-save: {}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            ));

            // Clear the recovery state and create new lock file
            self.show_recovery_dialog = false;
            self.remove_lock_file();
            self.create_lock_file();
        }
    }

    /// Week 7 Day 5: Decline recovery and start fresh
    fn decline_recovery(&mut self) {
        self.recovery_autosave_path = None;
        self.show_recovery_dialog = false;
        self.remove_lock_file();
        self.create_lock_file();
        self.console_logs
            .push("Recovery declined - starting fresh".to_string());
    }

    /// Week 4 Day 5: Validate model file before import
    fn validate_model_file(&self, path: &std::path::Path) -> AssetValidation {
        let mut result = AssetValidation {
            is_valid: true,
            warnings: Vec::new(),
            info: Vec::new(),
        };

        // Check file exists
        if !path.exists() {
            result.is_valid = false;
            result.warnings.push("File does not exist".to_string());
            return result;
        }

        // Get file metadata
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                result.is_valid = false;
                result
                    .warnings
                    .push(format!("Cannot read file metadata: {}", e));
                return result;
            }
        };

        let file_size = metadata.len();
        let file_size_mb = file_size as f64 / (1024.0 * 1024.0);

        // Info: File size
        result
            .info
            .push(format!("File size: {:.2} MB", file_size_mb));

        // Warning: Very large files
        if file_size_mb > 100.0 {
            result.warnings.push(format!(
                "Very large model ({:.1} MB) - may cause slow loading",
                file_size_mb
            ));
        } else if file_size_mb > 50.0 {
            result.warnings.push(format!(
                "Large model ({:.1} MB) - consider using LODs",
                file_size_mb
            ));
        }

        // Check extension
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match extension.to_lowercase().as_str() {
            "glb" => {
                result.info.push("Format: GLB (binary glTF)".to_string());
            }
            "gltf" => {
                result.info.push("Format: glTF (JSON)".to_string());
                // Check for external files
                if let Some(parent) = path.parent() {
                    let bin_path = parent.join(format!(
                        "{}.bin",
                        path.file_stem().and_then(|s| s.to_str()).unwrap_or("")
                    ));
                    if bin_path.exists() {
                        result.info.push("External .bin file found".to_string());
                    }
                }
            }
            _ => {
                result.is_valid = false;
                result
                    .warnings
                    .push(format!("Unsupported format: .{}", extension));
            }
        }

        // Basic GLB header validation for binary files
        if extension.to_lowercase() == "glb" {
            if let Ok(file) = std::fs::File::open(path) {
                use std::io::Read;
                let mut reader = std::io::BufReader::new(file);
                let mut magic = [0u8; 4];
                if reader.read_exact(&mut magic).is_ok() {
                    // GLB magic number: "glTF" (0x676C5446)
                    if &magic == b"glTF" {
                        result.info.push("Valid GLB magic header".to_string());
                    } else {
                        result.is_valid = false;
                        result
                            .warnings
                            .push("Invalid GLB header - file may be corrupted".to_string());
                    }
                }
            }
        }

        result
    }

    /// Week 4 Day 3-4: Import texture with GPU-optimized compression
    ///
    /// Validates texture dimensions and saves in GPU-friendly format.
    /// For production BC7/ASTC compression, use the asset pipeline CLI tools.
    fn import_texture_with_compression(&mut self, path: &std::path::Path, file_name: &str) {
        self.log(format!("Importing texture: {}", file_name));

        // Load the image
        let image = match image::open(path) {
            Ok(img) => img,
            Err(e) => {
                self.log(format!("Failed to open image: {}", e));
                self.toast_error(format!("Failed to open: {}", file_name));
                return;
            }
        };

        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let original_size = rgba.len();

        self.log(format!(
            "Image dimensions: {}x{}, {} bytes",
            width, height, original_size
        ));

        // Check if dimensions are divisible by 4 (required for BC7/DXT block compression)
        let gpu_compatible = width % 4 == 0 && height % 4 == 0;
        if !gpu_compatible {
            self.log(format!(
                "GPU block compression requires dimensions divisible by 4 (got {}x{})",
                width, height
            ));
            self.log(
                "Consider resizing to nearest power of 2 (e.g., 512x512, 1024x1024, 2048x2048)"
                    .to_string(),
            );
        }

        // Check power of 2 for best GPU compatibility
        let is_pow2 = width.is_power_of_two() && height.is_power_of_two();
        if !is_pow2 {
            self.log(format!(
                "Non-power-of-2 dimensions ({}x{}) may have reduced GPU compatibility",
                width, height
            ));
        }

        // Save as optimized PNG (lossless, good compression)
        let start_time = std::time::Instant::now();
        let output_path = self.content_root.join("textures").join(format!(
            "{}.png",
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("texture")
        ));

        // Create textures directory if needed
        if let Some(parent) = output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match rgba.save(&output_path) {
            Ok(_) => {
                let elapsed = start_time.elapsed();
                let saved_size = std::fs::metadata(&output_path)
                    .map(|m| m.len() as usize)
                    .unwrap_or(0);
                let ratio = original_size as f32 / saved_size.max(1) as f32;
                let reduction = 100.0 * (1.0 - saved_size as f32 / original_size.max(1) as f32);

                self.log(format!(
                    "PNG saved: {} -> {} bytes ({:.1}:1, {:.1}% reduction) in {:.2}s",
                    original_size,
                    saved_size,
                    ratio,
                    reduction,
                    elapsed.as_secs_f32()
                ));
                self.log(format!("Saved texture: {}", output_path.display()));

                let status = if gpu_compatible && is_pow2 {
                    "GPU-optimal"
                } else if gpu_compatible {
                    "Non-power-of-2"
                } else {
                    "Needs resize for BC7"
                };

                self.toast_success(format!(
                    "Imported {}: {}×{} {}",
                    file_name, width, height, status
                ));
            }
            Err(e) => {
                self.log(format!("Failed to save texture: {}", e));
                self.toast_error(format!("Failed to save: {}", file_name));
            }
        }
    }

    // =========================================================================
    // Week 5 Day 1-2: Multi-Select Operations
    // =========================================================================

    /// Apply current material to all selected entities
    fn apply_material_to_selection(&mut self) {
        let selected_ids: Vec<_> = self.selection_set.entities.iter().copied().collect();

        if selected_ids.is_empty() {
            self.log("No entities selected".to_string());
            return;
        }

        // Use a default material name based on current PBR settings
        let material_name = format!(
            "PBR_{:.0}_{:.0}",
            self.mat_doc.metallic * 100.0,
            self.mat_doc.roughness * 100.0
        );
        let mut applied_count = 0;

        if let Some(scene_state) = self.scene_state.as_mut() {
            for entity_id in &selected_ids {
                if let Some(editor_entity) =
                    scene_state.get_editor_entity_mut(*entity_id as astraweave_core::Entity)
                {
                    let mut material = entity_manager::EntityMaterial::new();
                    material.name = material_name.clone();
                    editor_entity.set_material(material);
                    applied_count += 1;
                }
            }
        }

        if applied_count > 0 {
            self.log(format!(
                "Applied material '{}' to {} entities",
                material_name, applied_count
            ));
            self.toast_success(format!("Material applied to {} entities", applied_count));
            self.status = format!("Applied material to {} entities", applied_count);
        }
    }

    /// Group selected entities under a new parent entity
    fn group_selection(&mut self) {
        let selected_ids: Vec<_> = self.selection_set.entities.iter().copied().collect();

        if selected_ids.len() < 2 {
            self.log("Select at least 2 entities to group".to_string());
            return;
        }

        // Compute average position of selected entities for group pivot
        let mut avg_pos = glam::Vec3::ZERO;
        let mut count = 0u32;
        for &id in &selected_ids {
            if let Some(entity) = self.entity_manager.get(id) {
                avg_pos += entity.position;
                count += 1;
            }
        }
        if count > 0 {
            avg_pos /= count as f32;
        }

        // Create the group parent entity
        let group_id = self
            .entity_manager
            .create(format!("Group ({})", selected_ids.len()));
        if let Some(group_entity) = self.entity_manager.get_mut(group_id) {
            group_entity.position = avg_pos;
            group_entity.components.insert(
                "Group".to_string(),
                serde_json::json!({ "children_count": selected_ids.len() }),
            );
        }

        // Reparent each selected entity under the group
        for &id in &selected_ids {
            if let Some(entity) = self.entity_manager.get_mut(id) {
                entity.parent = Some(group_id);
            }
        }

        // Select the new group
        self.selection_set.primary = Some(group_id);
        self.selection_set.entities.clear();
        self.selection_set.entities.insert(group_id);
        self.selected_entity = Some(group_id);

        self.is_dirty = true;
        self.log(format!(
            "Grouped {} entities under Group #{}",
            selected_ids.len(),
            group_id
        ));
        self.toast_info(format!("Grouped {} entities", selected_ids.len()));
        self.status = format!("Grouped {} entities", selected_ids.len());
    }

    /// Ungroup the selected entity (if it's a group)
    fn ungroup_selection(&mut self) {
        let primary = match self.selection_set.primary {
            Some(id) => id,
            None => {
                self.log("No group selected to ungroup".to_string());
                return;
            }
        };

        // Check if this entity is actually a group (has children parented to it)
        let child_ids: Vec<u64> = self
            .entity_manager
            .entities()
            .values()
            .filter(|e| e.parent == Some(primary))
            .map(|e| e.id)
            .collect();

        if child_ids.is_empty() {
            self.log(format!("Entity #{} has no children to ungroup", primary));
            return;
        }

        // Clear parent on all children (promote to scene root)
        for &child_id in &child_ids {
            if let Some(entity) = self.entity_manager.get_mut(child_id) {
                entity.parent = None;
            }
        }

        // Remove the group entity itself
        self.entity_manager.remove(primary);

        // Clear selection
        self.selection_set.primary = None;
        self.selection_set.entities.clear();
        self.selected_entity = None;

        self.is_dirty = true;
        self.log(format!(
            "Ungrouped {} children from Group #{}",
            child_ids.len(),
            primary
        ));
        self.toast_info(format!("Ungrouped {} children", child_ids.len()));
        self.status = format!("Ungrouped {} children", child_ids.len());
    }

    /// Align selected entities in the specified direction
    fn align_selection(&mut self, direction: AlignDirection) {
        let selected_ids: Vec<_> = self.selection_set.entities.iter().copied().collect();

        if selected_ids.len() < 2 {
            return;
        }

        if let Some(scene_state) = self.scene_state.as_mut() {
            // Gather positions using EditorEntity API
            let mut positions: Vec<(entity_manager::EntityId, glam::Vec3)> = Vec::new();

            for entity_id in &selected_ids {
                if let Some(transform) =
                    scene_state.transform_for(*entity_id as astraweave_core::Entity)
                {
                    positions.push((*entity_id, transform.position));
                }
            }

            if positions.is_empty() {
                return;
            }

            // Calculate target value based on alignment direction
            let target = match direction {
                AlignDirection::Left => positions
                    .iter()
                    .map(|(_, p)| p.x)
                    .fold(f32::INFINITY, f32::min),
                AlignDirection::Right => positions
                    .iter()
                    .map(|(_, p)| p.x)
                    .fold(f32::NEG_INFINITY, f32::max),
                AlignDirection::Top => positions
                    .iter()
                    .map(|(_, p)| p.z)
                    .fold(f32::NEG_INFINITY, f32::max),
                AlignDirection::Bottom => positions
                    .iter()
                    .map(|(_, p)| p.z)
                    .fold(f32::INFINITY, f32::min),
                AlignDirection::CenterX => {
                    let sum: f32 = positions.iter().map(|(_, p)| p.x).sum();
                    sum / positions.len() as f32
                }
                AlignDirection::CenterZ => {
                    let sum: f32 = positions.iter().map(|(_, p)| p.z).sum();
                    sum / positions.len() as f32
                }
            };

            // Apply alignment
            for (entity_id, pos) in &positions {
                let new_pos = match direction {
                    AlignDirection::Left | AlignDirection::Right | AlignDirection::CenterX => {
                        glam::Vec3::new(target, pos.y, pos.z)
                    }
                    AlignDirection::Top | AlignDirection::Bottom | AlignDirection::CenterZ => {
                        glam::Vec3::new(pos.x, pos.y, target)
                    }
                };

                // Get current transform and update position
                if let Some(mut transform) =
                    scene_state.transform_for(*entity_id as astraweave_core::Entity)
                {
                    transform.position = new_pos;
                    scene_state.apply_transform(*entity_id as astraweave_core::Entity, &transform);
                }
            }

            let dir_name = match direction {
                AlignDirection::Left => "left",
                AlignDirection::Right => "right",
                AlignDirection::Top => "top",
                AlignDirection::Bottom => "bottom",
                AlignDirection::CenterX => "center X",
                AlignDirection::CenterZ => "center Z",
            };

            self.log(format!(
                "Aligned {} entities to {}",
                positions.len(),
                dir_name
            ));
            self.status = format!("Aligned {} entities", positions.len());
        }
    }

    /// Distribute selected entities evenly along an axis
    fn distribute_selection(&mut self, direction: DistributeDirection) {
        let selected_ids: Vec<_> = self.selection_set.entities.iter().copied().collect();

        if selected_ids.len() < 3 {
            return;
        }

        if let Some(scene_state) = self.scene_state.as_mut() {
            // Gather positions using EditorEntity API
            let mut positions: Vec<(entity_manager::EntityId, glam::Vec3)> = Vec::new();

            for entity_id in &selected_ids {
                if let Some(transform) =
                    scene_state.transform_for(*entity_id as astraweave_core::Entity)
                {
                    positions.push((*entity_id, transform.position));
                }
            }

            if positions.len() < 3 {
                return;
            }

            // Sort by the distribution axis (use Equal for NaN safety)
            match direction {
                DistributeDirection::X => positions.sort_by(|a, b| {
                    a.1.x
                        .partial_cmp(&b.1.x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                DistributeDirection::Z => positions.sort_by(|a, b| {
                    a.1.z
                        .partial_cmp(&b.1.z)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
            }

            // Calculate spacing
            let first = &positions[0].1;
            let last = &positions[positions.len() - 1].1;
            let spacing = match direction {
                DistributeDirection::X => (last.x - first.x) / (positions.len() - 1) as f32,
                DistributeDirection::Z => (last.z - first.z) / (positions.len() - 1) as f32,
            };

            // Apply distribution (skip first and last which stay in place)
            for (i, (entity_id, pos)) in positions.iter().enumerate() {
                if i == 0 || i == positions.len() - 1 {
                    continue;
                }

                let new_pos = match direction {
                    DistributeDirection::X => {
                        glam::Vec3::new(first.x + spacing * i as f32, pos.y, pos.z)
                    }
                    DistributeDirection::Z => {
                        glam::Vec3::new(pos.x, pos.y, first.z + spacing * i as f32)
                    }
                };

                // Get current transform and update position
                if let Some(mut transform) =
                    scene_state.transform_for(*entity_id as astraweave_core::Entity)
                {
                    transform.position = new_pos;
                    scene_state.apply_transform(*entity_id as astraweave_core::Entity, &transform);
                }
            }

            let axis = match direction {
                DistributeDirection::X => "X",
                DistributeDirection::Z => "Z",
            };

            self.log(format!(
                "Distributed {} entities along {}",
                positions.len(),
                axis
            ));
            self.status = format!("Distributed {} entities", positions.len());
        }
    }

    /// Select all entities in the scene
    fn select_all_entities(&mut self) {
        if let Some(scene_state) = self.scene_state.as_ref() {
            self.selection_set.clear();

            // Get all entity IDs from the world
            let entities = scene_state.world().entities();
            let mut count = 0;

            for entity in entities {
                self.selection_set
                    .add(entity as entity_manager::EntityId, count == 0);
                count += 1;
            }

            self.log(format!("Selected {} entities", count));
            self.status = format!("Selected {} entities", count);
        }
    }

    /// Week 4: Handle files dropped onto the editor window
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        // Show drop overlay when files are being dragged over
        let hovered_files = ctx.input(|i| i.raw.hovered_files.clone());
        if !hovered_files.is_empty() {
            self.show_drop_overlay(ctx, &hovered_files);
        }

        // Check for dropped files
        let dropped_files: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        if dropped_files.is_empty() {
            return;
        }

        for path in dropped_files {
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            match extension.to_lowercase().as_str() {
                // 3D Models - Load to viewport with validation
                "glb" | "gltf" => {
                    self.log(format!("Importing model: {}", file_name));

                    // Week 4 Day 5: Validate asset before import
                    let validation = self.validate_model_file(&path);
                    for warning in &validation.warnings {
                        self.log(format!("{}", warning));
                    }

                    if !validation.is_valid {
                        self.toast_error(format!(
                            "Invalid model: {}",
                            validation
                                .warnings
                                .first()
                                .unwrap_or(&"Unknown error".to_string())
                        ));
                        self.log(format!("Model validation failed: {}", file_name));
                        continue;
                    }

                    // Log validation stats
                    if !validation.info.is_empty() {
                        for info in &validation.info {
                            self.log(format!("{}", info));
                        }
                    }

                    if let Some(viewport) = &self.viewport {
                        match viewport.load_gltf_model(file_name, &path) {
                            Ok(_) => {
                                self.toast_success(format!("Loaded model: {}", file_name));
                                self.log(format!("Model loaded: {}", file_name));
                            }
                            Err(e) => {
                                self.toast_error(format!("Failed to load {}: {}", file_name, e));
                                self.log(format!("Model load failed: {} - {}", file_name, e));
                            }
                        }
                    } else {
                        self.toast_error("No viewport available for model preview");
                        self.log("Cannot import model: viewport not initialized");
                    }
                }

                // Scene files - Load scene
                "ron" => {
                    if path.to_string_lossy().contains("scene") {
                        self.log(format!("Loading scene: {}", file_name));
                        match scene_serialization::load_scene(&path) {
                            Ok(world) => {
                                self.scene_state = Some(scene_state::EditorSceneState::new(world));
                                self.current_scene_path = Some(path.clone());
                                self.recent_files.add_file(path.clone());
                                self.is_dirty = false;
                                self.toast_success(format!("Loaded scene: {}", file_name));
                                self.log(format!("Scene loaded: {}", file_name));
                            }
                            Err(e) => {
                                self.toast_error(format!("Failed to load scene: {}", e));
                                self.log(format!("Scene load failed: {}", e));
                            }
                        }
                    } else {
                        self.log(format!("RON file dropped (not a scene): {}", file_name));
                    }
                }

                // Textures - Import with BC7 compression
                "png" | "jpg" | "jpeg" => {
                    self.import_texture_with_compression(&path, file_name);
                }

                // KTX2 textures - Already compressed, just register
                "ktx2" => {
                    self.log(format!(
                        "KTX2 texture imported: {} (already compressed)",
                        file_name
                    ));
                    self.toast_success(format!("Imported: {} (KTX2)", file_name));
                }

                // Materials - Load material definition
                "toml" => {
                    if path.to_string_lossy().contains("material") {
                        self.log(format!("Loading material definition: {}", file_name));
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                // Parse TOML material definition
                                match toml::from_str::<toml::Value>(&content) {
                                    Ok(material_def) => {
                                        self.log(format!("Material parsed: {}", file_name));
                                        self.toast_success(format!(
                                            "Loaded material: {}",
                                            file_name
                                        ));
                                        // Store in material cache for later use
                                        if let Some(name) =
                                            material_def.get("name").and_then(|v| v.as_str())
                                        {
                                            self.log(format!("   Material name: {}", name));
                                        }
                                    }
                                    Err(e) => {
                                        self.toast_error(format!("Invalid material format: {}", e));
                                        self.log(format!("Material parse failed: {}", e));
                                    }
                                }
                            }
                            Err(e) => {
                                self.toast_error(format!("Failed to read material: {}", e));
                                self.log(format!("Material read failed: {}", e));
                            }
                        }
                    } else {
                        self.log(format!("TOML file dropped: {}", file_name));
                    }
                }

                // Audio files - Import audio asset
                "ogg" | "wav" | "mp3" => {
                    self.log(format!("Importing audio file: {}", file_name));
                    // Validate audio file exists and is readable
                    match std::fs::metadata(&path) {
                        Ok(meta) => {
                            let size_kb = meta.len() / 1024;
                            self.log(format!("   Audio size: {} KB", size_kb));
                            self.toast_success(format!(
                                "Audio imported: {} ({} KB)",
                                file_name, size_kb
                            ));
                            // Copy to assets/audio if not already there
                            let audio_dir = self.content_root.join("audio");
                            if !path.starts_with(&audio_dir) {
                                if let Err(e) = std::fs::create_dir_all(&audio_dir) {
                                    self.log(format!("Could not create audio directory: {}", e));
                                } else {
                                    let dest = audio_dir.join(file_name);
                                    if let Err(e) = std::fs::copy(&path, &dest) {
                                        self.log(format!("Could not copy audio file: {}", e));
                                    } else {
                                        self.log(format!("Audio copied to: {}", dest.display()));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            self.toast_error(format!("Cannot read audio file: {}", e));
                            self.log(format!("Audio import failed: {}", e));
                        }
                    }
                }

                // Unknown file types
                _ => {
                    self.log(format!(
                        "Unknown file type dropped: {} ({})",
                        file_name, extension
                    ));
                    self.toast_info(format!("Unknown file: .{} not supported", extension));
                }
            }
        }
    }

    /// Week 4: Show visual overlay when dragging files over editor
    fn show_drop_overlay(&self, ctx: &egui::Context, hovered_files: &[egui::HoveredFile]) {
        let screen_rect = ctx.screen_rect();

        // Count file types
        let mut model_count = 0;
        let mut scene_count = 0;
        let mut texture_count = 0;
        let mut other_count = 0;

        for file in hovered_files {
            if let Some(path) = &file.path {
                match path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase()
                    .as_str()
                {
                    "glb" | "gltf" => model_count += 1,
                    "ron" => scene_count += 1,
                    "png" | "jpg" | "jpeg" | "ktx2" => texture_count += 1,
                    _ => other_count += 1,
                }
            } else {
                other_count += 1;
            }
        }

        // Build description text
        let mut parts = Vec::new();
        if model_count > 0 {
            parts.push(format!(
                "{} model{}",
                model_count,
                if model_count > 1 { "s" } else { "" }
            ));
        }
        if scene_count > 0 {
            parts.push(format!(
                "{} scene{}",
                scene_count,
                if scene_count > 1 { "s" } else { "" }
            ));
        }
        if texture_count > 0 {
            parts.push(format!(
                "{} texture{}",
                texture_count,
                if texture_count > 1 { "s" } else { "" }
            ));
        }
        if other_count > 0 {
            parts.push(format!(
                "{} file{}",
                other_count,
                if other_count > 1 { "s" } else { "" }
            ));
        }

        let description = if parts.is_empty() {
            "Drop files here".to_string()
        } else {
            parts.join(", ")
        };

        // Paint semi-transparent overlay
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("drop_overlay"),
        ));

        // Background dimming
        painter.rect_filled(
            screen_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 150),
        );

        // Center box with info
        let box_size = egui::vec2(400.0, 150.0);
        let box_rect = egui::Rect::from_center_size(screen_rect.center(), box_size);

        painter.rect_filled(box_rect, 12.0, egui::Color32::from_rgb(40, 60, 80));

        painter.rect_stroke(
            box_rect,
            12.0,
            egui::Stroke::new(3.0, egui::Color32::from_rgb(100, 180, 255)),
            egui::StrokeKind::Outside,
        );

        // Icon
        painter.text(
            box_rect.center() - egui::vec2(0.0, 30.0),
            egui::Align2::CENTER_CENTER,
            "v",
            egui::FontId::proportional(48.0),
            egui::Color32::WHITE,
        );

        // "Drop files here" text
        painter.text(
            box_rect.center() + egui::vec2(0.0, 20.0),
            egui::Align2::CENTER_CENTER,
            "Drop to import",
            egui::FontId::proportional(20.0),
            egui::Color32::from_rgb(200, 200, 200),
        );

        // File type description
        painter.text(
            box_rect.center() + egui::vec2(0.0, 45.0),
            egui::Align2::CENTER_CENTER,
            &description,
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(150, 200, 255),
        );
    }

    /// Week 6 Day 3-4: Render enhanced toast notifications
    fn render_toasts(&mut self, ctx: &egui::Context) {
        // Use the new ToastManager for rendering
        self.toast_manager.show(ctx);

        // Process any pending toast actions
        let actions = self.toast_manager.take_pending_actions();

        // Track state changes for after processing
        let mut do_undo = false;
        let mut do_retry = false;
        let mut retry_toast_id = 0u64;
        let mut details_to_log = Vec::new();
        let mut paths_to_open = Vec::new();
        let mut custom_actions = Vec::new();

        for (toast_id, action) in actions {
            match action {
                ui::ToastAction::Undo => {
                    do_undo = true;
                }
                ui::ToastAction::Retry => {
                    do_retry = true;
                    retry_toast_id = toast_id;
                }
                ui::ToastAction::ViewDetails(details) => {
                    details_to_log.push(details);
                }
                ui::ToastAction::Open(path) => {
                    paths_to_open.push(path);
                }
                ui::ToastAction::Custom { label, action_id } => {
                    custom_actions.push((label, action_id));
                }
            }
        }

        // Process collected actions
        if do_undo && self.undo_stack.can_undo() {
            if let Some(state) = self.scene_state.as_mut() {
                let _ = self
                    .undo_stack
                    .undo(state.world_mut(), Some(&mut self.entity_manager));
                self.status = "Undo successful".into();
                self.toast_manager.info("Undone");
            }
        }

        if do_retry {
            self.console_logs
                .push(format!("Retry requested for toast {}", retry_toast_id));
        }

        for details in details_to_log {
            self.console_logs.push(format!("Details: {}", details));
        }

        for path in paths_to_open {
            self.console_logs.push(format!("Open requested: {}", path));
            self.status = format!("Open: {}", path);
        }

        for (label, action_id) in custom_actions {
            self.console_logs
                .push(format!("Custom action: {} ({})", label, action_id));
        }
    }

    /// Scan asset packs for available 3D models (e.g. Kenney packs)
    fn scan_spawnable_models() -> Vec<(String, String)> {
        let mut models = Vec::new();
        let assets_3d = PathBuf::from("assets").join("3D assets");

        if let Ok(packs) = std::fs::read_dir(&assets_3d) {
            for pack in packs.flatten() {
                let pack_path = pack.path();
                if !pack_path.is_dir() {
                    continue;
                }
                let pack_name = pack_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // Look for GLTF/GLB models in common locations
                let search_dirs = [
                    pack_path.join("Models").join("GLTF format"),
                    pack_path.join("Models").join("GLB format"),
                    pack_path.join("Models"),
                    pack_path.clone(),
                ];

                let pack_start = models.len();
                for search_dir in &search_dirs {
                    if let Ok(files) = std::fs::read_dir(search_dir) {
                        for file in files.flatten() {
                            let file_path = file.path();
                            if let Some(ext) = file_path.extension() {
                                if ext == "glb" || ext == "gltf" {
                                    let file_stem = file_path
                                        .file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string();
                                    let display_name = format!(
                                        "{} / {}",
                                        pack_name,
                                        file_stem.replace('_', " ").replace('-', " ")
                                    );
                                    let rel_path = file_path.display().to_string();
                                    models.push((display_name, rel_path));
                                }
                            }
                        }
                        // If we found models in this search dir for this pack, skip other dirs
                        if models.len() > pack_start {
                            break;
                        }
                    }
                }
            }
        }

        models.sort_by(|a, b| a.0.cmp(&b.0));
        models
    }

    /// Initialize sample entities for viewport testing
    fn init_sample_entities(
        entity_manager: &mut EntityManager,
        scene_state: &mut scene_state::EditorSceneState,
    ) {
        use glam::Vec3;

        let samples = [
            ("Cube_1", Vec3::new(0.0, 0.0, 0.0), Vec3::ONE),
            ("Cube_2", Vec3::new(3.0, 0.0, 0.0), Vec3::ONE),
            ("Cube_3", Vec3::new(0.0, 0.0, 3.0), Vec3::ONE),
            ("Sphere_1", Vec3::new(-3.0, 1.0, 0.0), Vec3::splat(1.5)),
        ];

        for (name, pos, scale) in &samples {
            // Spawn in World
            let world_id = scene_state.world_mut().spawn(
                name,
                astraweave_core::IVec2 {
                    x: pos.x as i32,
                    y: pos.z as i32,
                },
                astraweave_core::Team { id: 0 },
                0,
                0,
            );
            scene_state.sync_entity(world_id);

            // Add to EntityManager with matching ID
            let em_id: u64 = world_id.into();
            let mut em_entity = entity_manager::EditorEntity::new(em_id, name.to_string());
            em_entity.position = *pos;
            em_entity.scale = *scale;
            em_entity.components.insert(
                "Transform".to_string(),
                serde_json::json!({"x": pos.x, "y": pos.y, "z": pos.z}),
            );
            entity_manager.add(em_entity);
        }
    }

    /// Create a default world with sample entities for viewport testing
    ///
    /// Spawns:
    /// - 10 companions (Team 0, blue) in a line at Y=0
    /// - 10 enemies (Team 1, red) in a line at Y=20
    fn create_default_world() -> World {
        let mut world = World::new();

        // Spawn 10 companion entities (blue team)
        for i in 0..10 {
            let pos = IVec2 { x: i * 3, y: 0 }; // Spread along X axis
            world.spawn(
                &format!("Companion_{}", i),
                pos,
                Team { id: 0 }, // Team 0 = companion
                100,            // HP
                30,             // Ammo
            );
        }

        // Spawn 10 enemy entities (red team)
        for i in 0..10 {
            let pos = IVec2 { x: i * 3, y: 20 }; // Spread along X axis, offset in Z
            world.spawn(
                &format!("Enemy_{}", i),
                pos,
                Team { id: 1 }, // Team 1 = enemy
                80,             // HP
                20,             // Ammo
            );
        }

        world
    }

    fn request_play(&mut self) {
        let _span = span!(Level::INFO, "request_play", mode = ?self.editor_mode).entered();

        if self.editor_mode.is_editing() {
            if let Some(scene_state) = self.scene_state.as_ref() {
                match self.runtime.enter_play(scene_state.world()) {
                    Ok(()) => {
                        self.editor_mode = EditorMode::Play;
                        self.status = "Playing".into();
                        info!("Entered Play mode - snapshot captured");
                        self.console_logs
                            .push("Entered Play mode (F6 to pause, F7 to stop)".into());
                    }
                    Err(e) => {
                        error!("Failed to enter play mode: {}", e);
                        self.console_logs
                            .push(format!("Failed to enter play mode: {}", e));
                        self.status = "Failed to enter play".into();
                    }
                }
            } else {
                warn!("No world loaded - cannot enter play mode");
                self.console_logs
                    .push("No world loaded – cannot enter play mode".into());
            }
        } else if self.editor_mode.is_paused() {
            self.runtime.resume();
            self.editor_mode = EditorMode::Play;
            self.status = "Playing".into();
            info!("Resumed playing from pause");
            self.console_logs.push("Resumed playing".into());
        }
    }

    fn request_pause(&mut self) {
        let _span = span!(Level::INFO, "request_pause").entered();

        if self.editor_mode.is_playing() {
            self.runtime.pause();
            self.editor_mode = EditorMode::Paused;
            self.status = "Paused".into();
            info!(
                "Paused simulation at tick {}",
                self.runtime.stats().tick_count
            );
            self.console_logs
                .push("Paused (F5 to resume, F7 to stop)".into());
        }
    }

    fn request_stop(&mut self) {
        let _span = span!(Level::INFO, "request_stop").entered();

        if !self.editor_mode.is_editing() {
            let final_tick = self.runtime.stats().tick_count;
            match self.runtime.exit_play() {
                Ok(restored) => {
                    if let Some(world) = restored {
                        self.scene_state = Some(EditorSceneState::new(world));
                    }
                    self.editor_mode = EditorMode::Edit;
                    self.status = "Stopped (world restored)".into();
                    info!(
                        "Stopped simulation after {} ticks - snapshot restored",
                        final_tick
                    );
                    self.console_logs
                        .push("Stopped play mode (world restored to snapshot)".into());
                    self.performance_panel.clear_runtime_stats();
                }
                Err(e) => {
                    error!("Failed to stop play mode: {}", e);
                    self.console_logs
                        .push(format!("Failed to stop play mode: {}", e));
                    self.status = "Failed to stop".into();
                }
            }
        }
    }

    fn request_step(&mut self) {
        let _span = span!(Level::DEBUG, "request_step").entered();

        if !self.editor_mode.is_editing() {
            if let Err(e) = self.runtime.step_frame() {
                error!("Step frame failed: {}", e);
                self.console_logs.push(format!("Step failed: {}", e));
            } else {
                self.editor_mode = EditorMode::Paused;
                self.status = "Stepped one frame".into();
                debug!(
                    "Stepped one frame to tick {}",
                    self.runtime.stats().tick_count
                );
                self.console_logs.push("Advanced one frame".into());
            }
        }
    }

    /// Create editor with CreationContext (for wgpu access)
    ///
    /// This method initializes the 3D viewport, which requires access to
    /// eframe's wgpu render state.
    ///
    /// # Errors
    ///
    /// Returns error if viewport initialization fails (missing wgpu support).
    fn new(cc: &eframe::CreationContext) -> Result<Self> {
        let mut app = Self::default();

        // Load preferences again to ensure we have the latest
        let prefs = editor_preferences::EditorPreferences::load();

        // Sync hierarchy panel with world (no sample entities — start with clean scene)
        if let Some(scene_state) = app.scene_state.as_mut() {
            app.hierarchy_panel.sync_with_world(scene_state.world_mut());
        }

        // Initialize viewport (requires wgpu render state from CreationContext)
        match ViewportWidget::new(cc) {
            Ok(mut viewport) => {
                // Apply persisted camera and snapping settings
                if let Some(mut camera) = prefs.camera {
                    camera.sanitize();
                    viewport.set_camera(camera);
                }
                if let Some(snapping) = prefs.snapping {
                    viewport.set_snapping_config(snapping);
                    app.snapping_config = snapping;
                }

                app.viewport = Some(viewport);
                app.console_logs.push("3D Viewport initialized".into());
            }
            Err(e) => {
                app.console_logs
                    .push(format!("Viewport init failed: {}", e));
                warn!("Viewport initialization failed: {}", e);
                // Continue without viewport (fallback to 2D mode)
            }
        }

        // Initialize default scene so asset imports work immediately
        let default_world = astraweave_core::World::new();
        app.scene_state = Some(EditorSceneState::new(default_world));
        app.console_logs.push("Default scene created".into());

        // Skip scan_spawnable_models(): entity catalog handles asset scanning lazily
        // on first UI access, avoiding redundant blocking filesystem traversal at startup.

        // Week 7 Day 5: Check for crash recovery
        app.check_for_crash_recovery();

        // Seed discovered audio tracks into the audio panel
        if !app.audio_bridge.discovered_tracks.is_empty() {
            let entries: Vec<panels::audio_panel::MusicTrackEntry> = app
                .audio_bridge
                .discovered_tracks
                .iter()
                .map(|p| panels::audio_panel::MusicTrackEntry {
                    name: p
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    path: p.display().to_string(),
                    ..Default::default()
                })
                .collect();
            app.dock_tab_viewer.set_audio_tracks(entries);
            app.console_logs.push(format!(
                "Audio: discovered {} tracks",
                app.audio_bridge.discovered_tracks.len()
            ));
        }
        if let Some(err) = app.audio_bridge.init_error() {
            app.console_logs
                .push(format!("Audio device unavailable: {}", err));
        }

        // Week 7 Day 5: Create lock file for this session
        app.create_lock_file();

        Ok(app)
    }
}

impl EditorApp {
    /// Render all modal dialogs (called from update loop)
    fn show_dialogs(&mut self, ctx: &egui::Context) {
        self.render_quit_dialog(ctx);
        self.render_help_dialog(ctx);
        self.render_settings_dialog(ctx);
        self.render_new_confirm_dialog(ctx);
        self.render_open_confirm_dialog(ctx);
        self.render_recovery_dialog(ctx);
        self.render_world_wizard(ctx);
        self.render_tutorial(ctx);
        self.render_about_dialog(ctx);
    }

    /// Render the quit confirmation dialog
    fn render_quit_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_quit_dialog {
            return;
        }

        dialogs::show_modal_overlay(ctx, "quit_dialog_overlay");
        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(350.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                // Warning icon and message
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("[!]").size(24.0));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("You have unsaved changes.").strong());
                        ui.add_space(4.0);
                        if let Some(path) = &self.current_scene_path {
                            ui.label(format!("Scene: {}", path.display()));
                        } else {
                            ui.label("Scene: Untitled (never saved)");
                        }
                    });
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Action buttons
                let mut do_save_quit = false;
                let mut do_quit = false;
                let mut do_cancel = false;

                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(egui::RichText::new("Save & Quit").strong())
                        .fill(egui::Color32::from_rgb(45, 125, 45));
                    if ui.add(save_btn).clicked() {
                        do_save_quit = true;
                    }

                    ui.add_space(8.0);

                    let quit_btn = egui::Button::new("Quit Without Saving")
                        .fill(egui::Color32::from_rgb(165, 45, 45));
                    if ui.add(quit_btn).clicked() {
                        do_quit = true;
                    }

                    ui.add_space(8.0);

                    if ui.button("Cancel").clicked() {
                        do_cancel = true;
                    }
                });

                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new(
                        "Press Escape to cancel • Auto-save available in Edit > Preferences",
                    )
                    .weak()
                    .small(),
                );

                // Process actions after UI
                if do_save_quit {
                    if let Some(world) = self.edit_world() {
                        let path = self
                            .current_scene_path
                            .clone()
                            .unwrap_or_else(|| self.content_root.join("scenes/untitled.scene.ron"));
                        if scene_serialization::save_scene(world, &path).is_ok() {
                            self.toast_success("Scene saved");
                            self.pending_quit = true;
                            self.remove_lock_file();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        } else {
                            self.toast_error("Failed to save scene");
                        }
                    } else {
                        self.pending_quit = true;
                        self.remove_lock_file();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }

                if do_quit {
                    self.pending_quit = true;
                    self.remove_lock_file();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if do_cancel {
                    self.show_quit_dialog = false;
                }
            });
    }

    /// Render the keyboard shortcuts help dialog
    fn render_help_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_help_dialog {
            return;
        }

        egui::Window::new("Keyboard Shortcuts")
            .collapsible(false)
            .resizable(true)
            .default_width(400.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                dialogs::show_shortcuts_grid(ui);

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.show_help_dialog = false;
                        }
                    });
                });
            });
    }

    /// Render the settings/preferences dialog
    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_settings_dialog {
            return;
        }

        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(true)
            .default_width(450.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                self.render_settings_content(ui);

                ui.add_space(12.0);
                ui.separator();

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.save_preferences();
                            self.show_settings_dialog = false;
                        }
                    });
                });
            });
    }

    /// Render the settings dialog content sections
    fn render_settings_content(&mut self, ui: &mut egui::Ui) {
        ui.heading("General");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Grid Visible:");
            ui.checkbox(&mut self.show_grid, "");
        });

        ui.add_space(16.0);
        ui.heading("Auto-Save");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Enable Auto-Save:");
            ui.checkbox(&mut self.auto_save_enabled, "");
        });

        ui.horizontal(|ui| {
            ui.label("Interval (seconds):");
            ui.add(
                egui::DragValue::new(&mut self.auto_save_interval_secs)
                    .range(30.0..=3600.0)
                    .speed(10.0),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Save to .autosave/ folder:");
            ui.checkbox(&mut self.auto_save_to_separate_dir, "");
        });

        ui.horizontal(|ui| {
            ui.label("Keep recent backups:");
            ui.add(
                egui::DragValue::new(&mut self.auto_save_keep_count)
                    .range(1..=10)
                    .speed(1.0),
            );
        });

        if let Some(recent) = self.get_most_recent_autosave() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Most recent:").weak());
                if let Some(filename) = recent.file_name().and_then(|n| n.to_str()) {
                    ui.label(egui::RichText::new(filename).weak().small());
                }
            });
        }

        ui.add_space(16.0);
        ui.heading("Panels");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Show Hierarchy:");
            ui.checkbox(&mut self.show_hierarchy_panel, "");
        });
        ui.horizontal(|ui| {
            ui.label("Show Inspector:");
            ui.checkbox(&mut self.show_inspector_panel, "");
        });
        ui.horizontal(|ui| {
            ui.label("Show Console:");
            ui.checkbox(&mut self.show_console_panel, "");
        });

        ui.add_space(16.0);
        ui.heading("Snapping");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Grid Snap:");
            ui.checkbox(&mut self.snapping_config.grid_enabled, "");
        });
        ui.horizontal(|ui| {
            ui.label("Grid Size:");
            ui.add(
                egui::DragValue::new(&mut self.snapping_config.grid_size)
                    .range(0.1..=10.0)
                    .speed(0.1)
                    .suffix(" units"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Angle Snap:");
            ui.checkbox(&mut self.snapping_config.angle_enabled, "");
        });
        ui.horizontal(|ui| {
            ui.label("Angle Increment:");
            ui.add(
                egui::DragValue::new(&mut self.snapping_config.angle_increment)
                    .range(1.0..=90.0)
                    .speed(1.0)
                    .suffix("°"),
            );
        });

        ui.add_space(16.0);
        ui.collapsing("Keyboard Shortcuts", |ui| {
            dialogs::show_shortcuts_compact_grid(ui);
        });
    }

    /// Render the new scene confirmation dialog
    fn render_new_confirm_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_new_confirm_dialog {
            return;
        }

        dialogs::show_modal_overlay(ctx, "new_scene_dialog_overlay");

        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("[!]").size(24.0));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Create a new scene?").strong());
                        ui.add_space(4.0);
                        ui.label("You have unsaved changes that will be lost.");
                        ui.add_space(4.0);
                        if let Some(path) = &self.current_scene_path {
                            ui.label(
                                egui::RichText::new(format!("Current: {}", path.display())).weak(),
                            );
                        } else {
                            ui.label(egui::RichText::new("Current: Untitled (never saved)").weak());
                        }
                    });
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                let mut do_save = false;
                let mut do_discard = false;
                let mut do_cancel = false;

                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(egui::RichText::new("Save First").strong())
                        .fill(egui::Color32::from_rgb(45, 125, 45));
                    if ui.add(save_btn).clicked() {
                        do_save = true;
                    }

                    ui.add_space(8.0);

                    let discard_btn = egui::Button::new("Discard Changes")
                        .fill(egui::Color32::from_rgb(165, 45, 45));
                    if ui.add(discard_btn).clicked() {
                        do_discard = true;
                    }

                    ui.add_space(8.0);

                    if ui.button("Cancel").clicked() {
                        do_cancel = true;
                    }
                });

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Press Escape to cancel").weak().small());

                if do_save {
                    if let Some(world) = self.edit_world() {
                        let path = self
                            .current_scene_path
                            .clone()
                            .unwrap_or_else(|| self.content_root.join("scenes/untitled.scene.ron"));
                        if scene_serialization::save_scene(world, &path).is_ok() {
                            self.toast_success("Scene saved");
                            self.show_new_confirm_dialog = false;
                            self.create_new_scene();
                        } else {
                            self.toast_error("Failed to save scene");
                        }
                    } else {
                        self.show_new_confirm_dialog = false;
                        self.create_new_scene();
                    }
                }

                if do_discard {
                    self.show_new_confirm_dialog = false;
                    self.create_new_scene();
                }

                if do_cancel {
                    self.show_new_confirm_dialog = false;
                }
            });
    }

    /// Render the open scene confirmation dialog
    fn render_open_confirm_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_open_confirm_dialog {
            return;
        }

        dialogs::show_modal_overlay(ctx, "open_scene_dialog_overlay");

        let pending_path_display = self
            .pending_open_path
            .as_ref()
            .map(|p| {
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_else(|| "selected scene".to_string());

        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(400.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("[!]").size(24.0));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Open \"{}\"?", pending_path_display))
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.label("You have unsaved changes that will be lost.");
                        ui.add_space(4.0);
                        if let Some(path) = &self.current_scene_path {
                            ui.label(
                                egui::RichText::new(format!("Current: {}", path.display())).weak(),
                            );
                        } else {
                            ui.label(egui::RichText::new("Current: Untitled (never saved)").weak());
                        }
                    });
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                let mut do_save = false;
                let mut do_discard = false;
                let mut do_cancel = false;

                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(egui::RichText::new("Save First").strong())
                        .fill(egui::Color32::from_rgb(45, 125, 45));
                    if ui.add(save_btn).clicked() {
                        do_save = true;
                    }

                    ui.add_space(8.0);

                    let discard_btn = egui::Button::new("Discard & Open")
                        .fill(egui::Color32::from_rgb(165, 45, 45));
                    if ui.add(discard_btn).clicked() {
                        do_discard = true;
                    }

                    ui.add_space(8.0);

                    if ui.button("Cancel").clicked() {
                        do_cancel = true;
                    }
                });

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Press Escape to cancel").weak().small());

                if do_save {
                    if let Some(world) = self.edit_world() {
                        let save_path = self
                            .current_scene_path
                            .clone()
                            .unwrap_or_else(|| self.content_root.join("scenes/untitled.scene.ron"));
                        if scene_serialization::save_scene(world, &save_path).is_ok() {
                            self.toast_success("Scene saved");
                            if let Some(open_path) = self.pending_open_path.take() {
                                self.load_scene_from_path(&open_path);
                            }
                            self.show_open_confirm_dialog = false;
                        } else {
                            self.toast_error("Failed to save scene");
                        }
                    } else {
                        if let Some(open_path) = self.pending_open_path.take() {
                            self.load_scene_from_path(&open_path);
                        }
                        self.show_open_confirm_dialog = false;
                    }
                }

                if do_discard {
                    if let Some(open_path) = self.pending_open_path.take() {
                        self.load_scene_from_path(&open_path);
                    }
                    self.show_open_confirm_dialog = false;
                }

                if do_cancel {
                    self.pending_open_path = None;
                    self.show_open_confirm_dialog = false;
                }
            });
    }

    /// Render the crash recovery dialog
    fn render_recovery_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_recovery_dialog {
            return;
        }

        dialogs::show_modal_overlay(ctx, "recovery_dialog_overlay");

        let autosave_name = self
            .recovery_autosave_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let recovery_path = self.recovery_autosave_path.clone();

        egui::Window::new("Crash Recovery")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(450.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("[!]").size(32.0));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Previous Session Detected")
                                .strong()
                                .size(16.0),
                        );
                        ui.add_space(4.0);
                        ui.label("The editor may have closed unexpectedly.");
                        ui.label("An auto-save backup was found:");
                    });
                });

                ui.add_space(12.0);

                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(35, 45, 55))
                    .corner_radius(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(&autosave_name).monospace());
                        });

                        if let Some(path) = &recovery_path {
                            if let Ok(metadata) = fs::metadata(path) {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(duration) = modified.elapsed() {
                                        let mins_ago = duration.as_secs() / 60;
                                        let time_str = if mins_ago < 60 {
                                            format!("{} minutes ago", mins_ago)
                                        } else if mins_ago < 1440 {
                                            format!("{} hours ago", mins_ago / 60)
                                        } else {
                                            format!("{} days ago", mins_ago / 1440)
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("Modified: {}", time_str))
                                                .weak()
                                                .small(),
                                        );
                                    }
                                }
                            }
                        }
                    });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                let mut do_restore = false;
                let mut do_fresh = false;

                ui.horizontal(|ui| {
                    let restore_btn =
                        egui::Button::new(egui::RichText::new("Restore Auto-Save").strong())
                            .fill(egui::Color32::from_rgb(45, 125, 45))
                            .min_size(egui::vec2(150.0, 32.0));
                    if ui.add(restore_btn).clicked() {
                        do_restore = true;
                    }

                    ui.add_space(16.0);

                    let fresh_btn =
                        egui::Button::new("Start Fresh").min_size(egui::vec2(120.0, 32.0));
                    if ui.add(fresh_btn).clicked() {
                        do_fresh = true;
                    }
                });

                ui.add_space(12.0);

                ui.label(
                    egui::RichText::new("Tip: Auto-saves are stored in the .autosave/ folder")
                        .weak()
                        .small(),
                );

                if do_restore {
                    self.recover_from_autosave();
                }

                if do_fresh {
                    self.decline_recovery();
                }
            });
    }

    /// Render the World Creation Wizard modal & process its output.
    fn render_world_wizard(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.world_wizard.show(ctx) {
            match action {
                panels::WorldWizardAction::Generate {
                    template,
                    gameplay,
                    filler_action,
                } => {
                    self.toast_success(format!(
                        "Generating world from \"{}\" template (genre: {})...",
                        template.name(),
                        gameplay.name()
                    ));
                    self.console_logs.push(format!(
                        "[WorldWizard] Generate: template={}, gameplay={}, action={:?}",
                        template.name(),
                        gameplay.name(),
                        filler_action
                    ));

                    // --- Terrain generation pipeline ---
                    if let panels::procedural_filler_panel::FillerAction::GenerateFullScene {
                        seed,
                        biome,
                        area_radius,
                        ..
                    } = &filler_action
                    {
                        let biome_key = biome.terrain_biome_key();
                        // Convert area_radius to chunk_radius (each chunk is 256 world units)
                        let chunk_radius = ((*area_radius / 256.0).ceil() as i32).clamp(1, 6);
                        self.dock_tab_viewer.trigger_terrain_generation(
                            *seed,
                            biome_key,
                            chunk_radius,
                        );
                        self.console_logs.push(format!(
                            "[WorldWizard] Terrain: seed={}, biome='{}', chunks={}",
                            seed, biome_key, chunk_radius
                        ));
                    }

                    // --- Spawn starter entities from gameplay preset ---
                    if gameplay != panels::GameplayPreset::Custom {
                        let starters = gameplay.starter_entities();
                        if let Some(scene_state) = self.scene_state.as_mut() {
                            for starter in starters {
                                // Offset each starter entity so they don't overlap
                                let offset_x = starter.position[0] as i32;
                                let offset_z = starter.position[2] as i32;
                                let spawn_height = self
                                    .dock_tab_viewer
                                    .sample_terrain_height_at(offset_x as f32, offset_z as f32)
                                    .unwrap_or(0.0);

                                let entity = scene_state.world_mut().spawn(
                                    starter.name,
                                    IVec2 {
                                        x: offset_x,
                                        y: offset_z,
                                    },
                                    Team { id: 0 },
                                    0,
                                    0,
                                );
                                // Place entity on terrain
                                if let Some(pose) = scene_state.world_mut().pose_mut(entity) {
                                    pose.height = spawn_height;
                                }
                                scene_state.sync_entity(entity);
                                let em_id: u64 = entity.into();
                                let mut em_entity = entity_manager::EditorEntity::new(
                                    em_id,
                                    starter.name.to_string(),
                                );
                                em_entity.components.insert(
                                    "Transform".to_string(),
                                    serde_json::json!({
                                        "x": offset_x,
                                        "y": spawn_height,
                                        "z": offset_z
                                    }),
                                );
                                // Add genre-specific component stubs
                                for schema in gameplay.component_schemas() {
                                    let fields: serde_json::Map<String, serde_json::Value> = schema
                                        .fields
                                        .iter()
                                        .map(|(k, _)| (k.to_string(), serde_json::Value::Null))
                                        .collect();
                                    em_entity.components.insert(
                                        schema.name.to_string(),
                                        serde_json::Value::Object(fields),
                                    );
                                }
                                // Map archetype to a real asset mesh
                                let mesh_path = match starter.archetype {
                                    // Characters — KayKit collection
                                    "PlayerCharacter" | "Player" => Some("assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Rogue.glb"),
                                    "NPC" => Some("assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Mage.glb"),
                                    "Enemy" => Some("assets/The Complete KayKit Collection v4/KayKit Skeletons 1.1/characters/gltf/Skeleton_Warrior.glb"),
                                    "Commander" => Some("assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Knight.glb"),
                                    // Buildings & structures — 3D asset packs
                                    "Building" => Some("assets/3D assets/Castle Kit/Models/GLB format/tower-square.glb"),
                                    // Props / interactables — 3D asset packs
                                    "Interactable" => Some("assets/3D assets/Survival Kit/Models/GLB format/box-large.glb"),
                                    "Placeable" => Some("assets/3D assets/Survival Kit/Models/GLB format/campfire-pit.glb"),
                                    "Resource" | "Harvestable" => Some("assets/3D assets/Survival Kit/Models/GLB format/rock-a.glb"),
                                    // Military / units
                                    "Unit" | "ArmyUnit" => Some("assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Barbarian.glb"),
                                    "Support" => Some("assets/3D assets/Fantasy Town Kit/Models/GLB format/cart.glb"),
                                    _ => None,
                                };
                                if let Some(path) = mesh_path {
                                    em_entity.set_mesh(path.to_string());
                                }
                                self.entity_manager.add(em_entity);
                            }
                            self.hierarchy_panel
                                .sync_with_world(scene_state.world_mut());
                            self.console_logs.push(format!(
                                "[WorldWizard] Spawned {} starter entities for {}",
                                starters.len(),
                                gameplay.name()
                            ));
                        }
                    }

                    self.status = format!("World generated: {}", template.name());
                    self.is_dirty = true;
                }
                panels::WorldWizardAction::Cancelled => {
                    self.toast_info("World wizard cancelled.");
                }
            }
        }
    }

    /// Render the tutorial overlay & process its actions.
    fn render_tutorial(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.tutorial.show(ctx) {
            match action {
                tutorial::TutorialAction::OpenWorldWizard => {
                    self.world_wizard.open();
                }
                tutorial::TutorialAction::Completed | tutorial::TutorialAction::Skipped => {
                    self.save_preferences();
                    self.toast_success("Tutorial complete — happy building!");
                }
            }
        }
    }

    /// Render the About dialog.
    fn render_about_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_about_dialog {
            return;
        }

        dialogs::show_modal_overlay(ctx, "about_dialog_overlay");

        egui::Window::new("About AstraWeave")
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("AstraWeave Engine")
                            .size(22.0)
                            .strong()
                            .color(egui::Color32::from_rgb(80, 160, 255)),
                    );
                    ui.label(
                        egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .color(egui::Color32::from_rgb(160, 160, 170)),
                    );
                    ui.add_space(6.0);
                    ui.label("AI-Native Game Engine");
                    ui.label("Built iteratively by AI — zero human-written code.");
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Rust · wgpu · egui · ECS")
                            .color(egui::Color32::from_rgb(120, 120, 140))
                            .italics(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("License: MIT")
                            .color(egui::Color32::from_rgb(120, 120, 140)),
                    );
                    ui.add_space(12.0);
                    if ui.button("Close").clicked() {
                        self.show_about_dialog = false;
                    }
                });
            });
    }

    fn show_status_bar(&mut self, ctx: &egui::Context) {
        // Week 6 Day 5: Sample resource usage periodically (every 500ms)
        if self.last_resource_sample.elapsed() > std::time::Duration::from_millis(500) {
            self.sample_resource_usage();
            self.last_resource_sample = std::time::Instant::now();
        }

        let bottom_entity_count = self.entity_manager.count();

        // Clone the path string to avoid borrow conflicts
        let bottom_scene_path_str: Option<String> = self
            .current_scene_path
            .as_ref()
            .and_then(|p| p.to_str())
            .map(String::from);

        egui::TopBottomPanel::bottom("status_bar")
            .min_height(24.0)
            .show(ctx, |ui| {
                ui.set_min_size(ui.available_size());
                // Week 6 Day 5: Use enhanced status bar with progress and resource usage
                StatusBar::show_enhanced(
                    ui,
                    &self.editor_mode,
                    &self.current_gizmo_mode,
                    &self.selection_set,
                    &self.undo_stack,
                    &self.snapping_config,
                    self.current_fps,
                    self.is_dirty,
                    bottom_entity_count,
                    bottom_scene_path_str.as_deref(),
                    &mut self.progress_manager,
                    &self.resource_usage,
                );
            });
    }

    fn show_legacy_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("astract_left_panel")
            .resizable(true)
            .min_width(250.0)
            .frame(egui::Frame::NONE.inner_margin(0.0))
            .show(ctx, |ui| {
                ui.set_min_size(ui.available_size());

                ui.vertical(|ui| {
                    ui.set_min_size(ui.available_size());

                    ui.heading("Hierarchy");
                    ui.add_space(4.0);

                    // Search bar
                    ui.horizontal(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut self.legacy_hierarchy_search);
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            self.show_scene_hierarchy(ui);
                        });
                });
            });
    }

    fn show_docking_layout(&mut self, ctx: &egui::Context) {
        // Phase 11: Professional Docking System
        // Sync selected entity to tab viewer
        self.dock_tab_viewer
            .set_selected_entity(self.selected_entity);
        self.dock_tab_viewer
            .set_is_playing(!self.editor_mode.is_editing());
        self.dock_tab_viewer.begin_frame();

        // Sync entity list for hierarchy panel
        // Read World component values for entity list
        let world_ref = self.scene_state.as_ref().map(|s| s.world());
        let entity_list: Vec<EntityInfo> = self
            .entity_manager
            .entities()
            .iter()
            .map(|(id, entity)| {
                let eid = entity_id_to_world(*id);
                let (hp, team_id, ammo, pos_x, pos_y, rotation, scale) =
                    if let (Some(eid), Some(w)) = (eid, &world_ref) {
                        (
                            w.health(eid).map(|h| h.hp),
                            w.team(eid).map(|t| t.id),
                            w.ammo(eid).map(|a| a.rounds),
                            w.pose(eid).map(|p| p.pos.x),
                            w.pose(eid).map(|p| p.pos.y),
                            w.pose(eid).map(|p| p.rotation),
                            w.pose(eid).map(|p| p.scale),
                        )
                    } else {
                        (None, None, None, None, None, None, None)
                    };
                EntityInfo {
                    id: *id,
                    name: entity.name.clone(),
                    components: entity.components.keys().cloned().collect(),
                    entity_type: if entity.components.contains_key("Camera") {
                        "Camera".to_string()
                    } else if entity.components.contains_key("Light") {
                        "Light".to_string()
                    } else if entity.components.contains_key("Mesh") {
                        "Mesh".to_string()
                    } else {
                        "Entity".to_string()
                    },
                    hp,
                    team_id,
                    ammo,
                    pos_x,
                    pos_y,
                    rotation,
                    scale,
                    component_data: entity.components.clone(),
                    material_base_color: [
                        entity.material.base_color.x,
                        entity.material.base_color.y,
                        entity.material.base_color.z,
                        entity.material.base_color.w,
                    ],
                    material_metallic: entity.material.metallic,
                    material_roughness: entity.material.roughness,
                    material_emissive: [
                        entity.material.emissive.x,
                        entity.material.emissive.y,
                        entity.material.emissive.z,
                    ],
                    material_textures: entity
                        .material
                        .texture_slots
                        .iter()
                        .map(|(k, v)| (format!("{:?}", k), v.to_string_lossy().to_string()))
                        .collect(),
                }
            })
            .collect();
        self.dock_tab_viewer.set_entity_list(entity_list);

        // Sync selected entity transform for inspector
        if let Some(entity_id) = self.selected_entity {
            if let Some(entity) = self.entity_manager.get(entity_id) {
                let (pos, rot, scale) = entity.transform();
                // Convert to 3D format: x, y, z, rotation_z, scale_x, scale_y, scale_z
                let angle = rot.to_euler(glam::EulerRot::ZXY).0;
                self.dock_tab_viewer.set_selected_transform(Some((
                    pos.x, pos.y, pos.z, angle, scale.x, scale.y, scale.z,
                )));
                let entity_type = if entity.components.contains_key("Camera") {
                    "Camera".to_string()
                } else if entity.components.contains_key("Light") {
                    "Light".to_string()
                } else if entity.components.contains_key("Mesh") {
                    "Mesh".to_string()
                } else {
                    "Entity".to_string()
                };
                let eid = entity_id_to_world(entity_id);
                let (hp, tid, ammo_v, px, py, rot_v, sc_v) =
                    if let (Some(eid), Some(w)) = (eid, &world_ref) {
                        (
                            w.health(eid).map(|h| h.hp),
                            w.team(eid).map(|t| t.id),
                            w.ammo(eid).map(|a| a.rounds),
                            w.pose(eid).map(|p| p.pos.x),
                            w.pose(eid).map(|p| p.pos.y),
                            w.pose(eid).map(|p| p.rotation),
                            w.pose(eid).map(|p| p.scale),
                        )
                    } else {
                        (None, None, None, None, None, None, None)
                    };
                self.dock_tab_viewer
                    .set_selected_entity_info(Some(EntityInfo {
                        id: entity_id,
                        name: entity.name.clone(),
                        components: entity.components.keys().cloned().collect(),
                        entity_type,
                        hp,
                        team_id: tid,
                        ammo: ammo_v,
                        pos_x: px,
                        pos_y: py,
                        rotation: rot_v,
                        scale: sc_v,
                        component_data: entity.components.clone(),
                        material_base_color: [
                            entity.material.base_color.x,
                            entity.material.base_color.y,
                            entity.material.base_color.z,
                            entity.material.base_color.w,
                        ],
                        material_metallic: entity.material.metallic,
                        material_roughness: entity.material.roughness,
                        material_emissive: [
                            entity.material.emissive.x,
                            entity.material.emissive.y,
                            entity.material.emissive.z,
                        ],
                        material_textures: entity
                            .material
                            .texture_slots
                            .iter()
                            .map(|(k, v)| (format!("{:?}", k), v.to_string_lossy().to_string()))
                            .collect(),
                    }));
            } else if let Some(w) = &world_ref {
                // Entity exists in World but not in entity_manager — read directly
                if let Some((eid, pose)) =
                    entity_id_to_world(entity_id).and_then(|e| w.pose(e).map(|p| (e, p)))
                {
                    let name = w.name(eid).unwrap_or("Entity").to_string();
                    self.dock_tab_viewer.set_selected_transform(Some((
                        pose.pos.x as f32,
                        pose.height,
                        pose.pos.y as f32,
                        pose.rotation,
                        pose.scale,
                        pose.scale,
                        pose.scale,
                    )));
                    self.dock_tab_viewer
                        .set_selected_entity_info(Some(EntityInfo {
                            id: entity_id,
                            name,
                            components: Vec::new(),
                            entity_type: "Entity".to_string(),
                            hp: w.health(eid).map(|h| h.hp),
                            team_id: w.team(eid).map(|t| t.id),
                            ammo: w.ammo(eid).map(|a| a.rounds),
                            pos_x: Some(pose.pos.x),
                            pos_y: Some(pose.pos.y),
                            rotation: Some(pose.rotation),
                            scale: Some(pose.scale),
                            component_data: std::collections::HashMap::new(),
                            material_base_color: [1.0, 1.0, 1.0, 1.0],
                            material_metallic: 0.0,
                            material_roughness: 0.5,
                            material_emissive: [0.0, 0.0, 0.0],
                            material_textures: std::collections::HashMap::new(),
                        }));
                } else {
                    self.dock_tab_viewer.set_selected_transform(None);
                    self.dock_tab_viewer.set_selected_entity_info(None);
                }
            } else {
                self.dock_tab_viewer.set_selected_transform(None);
                self.dock_tab_viewer.set_selected_entity_info(None);
            }
        } else {
            self.dock_tab_viewer.set_selected_transform(None);
            self.dock_tab_viewer.set_selected_entity_info(None);
        }

        // Sync console logs
        self.dock_tab_viewer
            .set_console_logs(self.console_logs.clone());

        // Sync runtime stats for profiler panel
        let entity_count = self.entity_manager.entities().len();

        // Get real render stats from viewport if available
        let (vp_draw_calls, vp_triangles, vp_memory_mb) = if let Some(viewport) = &self.viewport {
            let stats = &viewport.toolbar().stats;
            // Query actual draw calls and triangle counts from terrain + scatter renderers
            if let Ok(renderer) = viewport.renderer().lock() {
                // Base draw calls: skybox(1) + grid(1) + entity_instances + gizmos(1)
                let base_draw_calls = if entity_count > 0 { 4 } else { 2 };
                let terrain_draw_calls = renderer.terrain_triangles().min(1); // 0 or 1 terrain pass
                let scatter_draw_calls = renderer.scatter_draw_calls() as usize;
                let draw_calls = base_draw_calls + terrain_draw_calls + scatter_draw_calls;
                // Real triangle counts from terrain and scatter renderers
                let entity_tris = entity_count * 12; // cubes
                let terrain_tris = renderer.terrain_triangles();
                let scatter_tris = renderer.scatter_triangles();
                let triangles = entity_tris + terrain_tris + scatter_tris + 4; // +4 for grid+skybox
                drop(renderer);
                (draw_calls, triangles, stats.memory_usage_mb)
            } else {
                (0, 0, stats.memory_usage_mb)
            }
        } else {
            (0, 0, 0.0)
        };

        // Real memory: use process working set from resource_usage if available
        let gpu_memory_bytes = if self.resource_usage.memory_used > 0 {
            self.resource_usage.memory_used as usize
        } else {
            (vp_memory_mb * 1024.0 * 1024.0) as usize
        };

        let runtime_stats = tab_viewer::RuntimeStatsInfo {
            frame_time_ms: self.runtime.stats().frame_time_ms,
            fps: self.current_fps,
            entity_count,
            tick_count: self.runtime.stats().tick_count,
            is_playing: self.runtime.is_playing(),
            is_paused: self.runtime.is_paused(),
            // Subsystem timings measured from previous frame
            render_time_ms: self.measured_render_ms,
            physics_time_ms: self.measured_tick_ms,
            ai_time_ms: 0.0,
            script_time_ms: 0.0,
            audio_time_ms: self.audio_bridge.last_tick_ms,
            draw_calls: vp_draw_calls,
            triangles: vp_triangles,
            gpu_memory_bytes,
        };
        self.dock_tab_viewer.set_runtime_stats(runtime_stats);
        self.dock_tab_viewer.update_play_session();

        // Sync scene stats - count actual component types from entities
        let entities = self.entity_manager.entities();
        let total_components: usize = entities.values().map(|e| e.components.len()).sum();
        let entity_count = entities.len();

        // Count actual component types by inspecting entity data
        let mut light_count = 0usize;
        let mut mesh_count = 0usize;
        let mut physics_bodies = 0usize;
        let mut audio_sources = 0usize;
        let mut particle_systems = 0usize;
        let mut camera_count = 0usize;
        let mut collider_count = 0usize;
        let mut script_count = 0usize;

        for entity in entities.values() {
            if entity.mesh.is_some() {
                mesh_count += 1;
            }
            for key in entity.components.keys() {
                let key_lower = key.to_lowercase();
                if key_lower.contains("light") {
                    light_count += 1;
                } else if key_lower.contains("rigidbody") || key_lower.contains("physics") {
                    physics_bodies += 1;
                } else if key_lower.contains("audio") || key_lower.contains("sound") {
                    audio_sources += 1;
                } else if key_lower.contains("particle") {
                    particle_systems += 1;
                } else if key_lower.contains("camera") {
                    camera_count += 1;
                } else if key_lower.contains("collider") {
                    collider_count += 1;
                } else if key_lower.contains("script") || key_lower.contains("behavior") {
                    script_count += 1;
                }
            }
        }

        // Always at least 1 camera (the editor camera)
        if camera_count == 0 {
            camera_count = 1;
        }

        let scene_stats = tab_viewer::SceneStatsInfo {
            total_entities: entity_count,
            total_components,
            prefab_instances: 0,
            selected_count: self.selection_set.count(),
            memory_usage_bytes: entity_count * 1024 + total_components * 256,
            active_systems: if self.runtime.is_playing() { 12 } else { 0 },
            loaded_assets: self.asset_registry.count(),
            light_count,
            mesh_count,
            physics_bodies,
            is_modified: self.is_dirty,
            audio_sources,
            particle_systems,
            camera_count,
            collider_count,
            script_count,
            ui_element_count: 0,
            scene_path: self
                .current_scene_path
                .as_ref()
                .map(|p| p.display().to_string()),
            last_save_time: self.last_save_time.clone(),
        };
        self.dock_tab_viewer.set_scene_stats(scene_stats);

        // Sync undo/redo counts
        self.dock_tab_viewer.set_undo_redo_counts(
            self.undo_stack.len(),
            0, // Redo count would come from redo stack
        );

        // Update frame time history for profiler graph
        self.dock_tab_viewer
            .push_frame_time(self.runtime.stats().frame_time_ms);

        // Feed frame debugger with render timing data
        {
            let entity_count = self.entity_manager.entities().len();
            let terrain_active = self.dock_tab_viewer.is_terrain_active();
            self.dock_tab_viewer.update_frame_debugger(
                self.measured_render_ms,
                entity_count,
                terrain_active,
            );
        }

        // Render the docking layout with EditorDrawContext for viewport integration
        // We need to carefully structure borrows to avoid conflicts
        // Use CentralPanel with no frame to render dock in remaining space (after side panels)
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(0.0))
            .show(ctx, |ui| {
                ui.set_min_size(ui.available_size());

                // Get mutable world from scene state
                let world_opt = self.scene_state.as_mut().map(|s| s.world_mut());

                // Check terrain brush state before borrowing dock_tab_viewer mutably
                let brush_active = self.dock_tab_viewer.is_terrain_brush_active();
                let brush_radius = self.dock_tab_viewer.terrain_brush_radius();
                let brush_is_paint = self.dock_tab_viewer.terrain_brush_is_paint();

                // Unified context rendering to avoid type-switching issues
                let viewport_layout = self.viewport_layout;
                let mut context =
                    EditorDrawContext::new(&mut self.dock_tab_viewer, &mut self.extra_viewports);

                if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut()) {
                    // Sync terrain brush state: tell viewport when brush is active
                    viewport.set_terrain_brush_active(brush_active);
                    viewport.set_terrain_brush_params(brush_radius, brush_is_paint);

                    context = context
                        .with_viewport(viewport)
                        .with_world(world)
                        .with_entity_manager(&mut self.entity_manager)
                        .with_undo_stack(&mut self.undo_stack)
                        .with_prefab_manager(&mut self.prefab_manager)
                        .with_viewport_layout(viewport_layout);
                }

                self.dock_layout.show_inside(ui, &mut context);
            });

        // Forward terrain brush hits from viewport to terrain panel
        if let Some(viewport) = self.viewport.as_mut() {
            let hits = viewport.take_terrain_brush_hits();
            for hit in hits {
                self.dock_tab_viewer.apply_terrain_brush_at(hit[0], hit[1]);
            }
            // Detect end of brush stroke (mouse released) for undo + flatten target reset
            if viewport.take_terrain_brush_stroke_ended() {
                let brush_name = self.dock_tab_viewer.terrain_brush_mode_name().to_string();
                if let Some(deltas) = self.dock_tab_viewer.end_terrain_brush_stroke() {
                    let cmd = command::TerrainBrushCommand::new(
                        deltas,
                        self.terrain_undo_queue.clone(),
                        &brush_name,
                    );
                    self.undo_stack.push_executed(cmd);
                }
            }
        }

        // Drain terrain undo queue (side-channel from undo/redo commands)
        {
            let actions: Vec<command::TerrainUndoAction> = {
                if let Ok(mut q) = self.terrain_undo_queue.lock() {
                    std::mem::take(&mut *q)
                } else {
                    Vec::new()
                }
            };
            for action in actions {
                match action {
                    command::TerrainUndoAction::ApplyHeights(snapshot) => {
                        self.dock_tab_viewer
                            .apply_terrain_height_snapshot(&snapshot);
                        // Upload dirty chunks to GPU
                        let dirty = self.dock_tab_viewer.take_terrain_dirty_chunks();
                        if let Some(viewport) = &self.viewport {
                            for (chunk_index, verts) in &dirty {
                                let gpu_verts: Vec<crate::viewport::types::TerrainVertex> = verts
                                    .iter()
                                    .map(|v| crate::viewport::types::TerrainVertex {
                                        position: v.position,
                                        normal: v.normal,
                                        uv: v.uv,
                                        biome_weights_0: v.biome_weights_0,
                                        biome_weights_1: v.biome_weights_1,
                                        material_ids: v.material_ids,
                                        material_weights: v.material_weights,
                                    })
                                    .collect();
                                viewport.update_terrain_chunk_vertices(*chunk_index, &gpu_verts);
                            }
                        }
                    }
                }
            }
        }

        // Sync environment settings to viewport — skip when params unchanged (perf)
        if let Some(viewport) = &self.viewport {
            let (sky_top, sky_horizon, ground_color) = self.dock_tab_viewer.compute_sky_colors();
            let sky_key = (sky_top, sky_horizon, ground_color);
            if self.cached_sky_colors.as_ref() != Some(&sky_key) {
                viewport.set_sky_colors(sky_top, sky_horizon, ground_color);
                self.cached_sky_colors = Some(sky_key);
            }

            // Compute fog color from sky horizon (fog blends toward sky color)
            let fog_color = [sky_horizon[0], sky_horizon[1], sky_horizon[2]];
            let (fog_enabled, fog_density, weather_type, particle_count_override) =
                self.dock_tab_viewer.fog_weather_params();
            let fog_params = crate::viewport::types::TerrainFogParams {
                fog_enabled,
                fog_density,
                fog_color,
                weather_type,
                particle_count_override,
            };
            if self.cached_fog_params.as_ref() != Some(&fog_params) {
                viewport.set_fog_params(fog_params);
                self.cached_fog_params = Some(fog_params);
            }

            // Sync lighting parameters from world panel to terrain shader
            let lighting_params = self.dock_tab_viewer.lighting_params();
            viewport.set_lighting_params(lighting_params);

            // Only enable water plane for water-related biomes
            let biome = self.dock_tab_viewer.terrain_primary_biome();
            let water_biome = matches!(biome, "swamp" | "beach" | "river");
            viewport.set_water_enabled(water_biome);
            if water_biome {
                viewport.set_water_level(self.dock_tab_viewer.water_level());
            }
        }

        // Check for transform changes and emit events
        self.dock_tab_viewer.check_transform_changes();

        // Handle panel close events (separate from PanelEvent for backward compatibility)
        for panel in self.dock_tab_viewer.take_closed_panels() {
            self.status = format!("Closed {} panel", panel.title());
        }
        for panel in self.dock_tab_viewer.take_panels_to_add() {
            self.dock_layout.add_panel(panel);
            self.status = format!("Added {} panel", panel.title());
        }
        self.dock_tab_viewer.check_transform_changes();

        // Handle panel events from the tab viewer
        for event in self.dock_tab_viewer.take_events() {
            match event {
                tab_viewer::PanelEvent::EntitySelected(entity_id) => {
                    self.selected_entity = Some(entity_id);
                    self.selection_set.primary = Some(entity_id);
                    if let Some(viewport) = &mut self.viewport {
                        viewport.set_selected_entity(Some(entity_id));
                    }
                    self.status = format!("Selected entity {}", entity_id);
                }
                tab_viewer::PanelEvent::EntityDeselected => {
                    self.selected_entity = None;
                    self.selection_set.primary = None;
                    if let Some(viewport) = &mut self.viewport {
                        viewport.set_selected_entity(None);
                    }
                    self.status = "Deselected entity".to_string();
                }
                tab_viewer::PanelEvent::TransformPositionChanged { entity_id, x, y, z } => {
                    // Route through undo stack for undoable position changes
                    if let (Some(scene_state), Some(entity)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let new_pos = astraweave_core::IVec2 {
                            x: x as i32,
                            y: z as i32,
                        };
                        let old_pos = scene_state
                            .world()
                            .pose(entity)
                            .map(|p| p.pos)
                            .unwrap_or(astraweave_core::IVec2 { x: 0, y: 0 });
                        let cmd = command::MoveEntityCommand::new(entity, old_pos, new_pos);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Move failed: {}", e));
                        } else {
                            // MoveEntityCommand only sets pos; also update height
                            if let Some(pose) = scene_state.world_mut().pose_mut(entity) {
                                pose.height = y;
                            }
                            scene_state.sync_entity(entity);
                        }
                    }
                    // EntityManager position is updated by MoveEntityCommand (via undo stack)
                    self.status = format!(
                        "Entity {} position: ({:.2}, {:.2}, {:.2})",
                        entity_id, x, y, z
                    );
                }
                tab_viewer::PanelEvent::TransformRotationChanged {
                    entity_id,
                    rotation,
                } => {
                    // Route through undo stack for undoable rotation changes
                    if let (Some(scene_state), Some(entity)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let old_rot = scene_state
                            .world()
                            .pose(entity)
                            .map(|p| (p.rotation_x, p.rotation, p.rotation_z))
                            .unwrap_or((0.0, 0.0, 0.0));
                        // Only Y-axis rotation comes from this event; preserve X/Z
                        let new_rot = (old_rot.0, rotation, old_rot.2);
                        let cmd = command::RotateEntityCommand::new(entity, old_rot, new_rot);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Rotate failed: {}", e));
                        } else {
                            scene_state.sync_entity(entity);
                        }
                    }
                    self.status = format!(
                        "Entity {} rotation: {:.1}deg",
                        entity_id,
                        rotation.to_degrees()
                    );
                }
                tab_viewer::PanelEvent::TransformScaleChanged {
                    entity_id,
                    scale_x,
                    scale_y,
                    scale_z,
                } => {
                    // Route through undo stack for undoable scale changes
                    if let (Some(scene_state), Some(entity)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let old_scale = scene_state
                            .world()
                            .pose(entity)
                            .map(|p| p.scale)
                            .unwrap_or(1.0);
                        let new_scale = (scale_x + scale_y + scale_z) / 3.0;
                        let cmd = command::ScaleEntityCommand::new(entity, old_scale, new_scale);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Scale failed: {}", e));
                        } else {
                            scene_state.sync_entity(entity);
                        }
                    }
                    // EntityManager scale is updated by ScaleEntityCommand (via undo stack)
                    self.status = format!(
                        "Entity {} scale: ({:.2}, {:.2}, {:.2})",
                        entity_id, scale_x, scale_y, scale_z
                    );
                }
                tab_viewer::PanelEvent::CreateEntity => {
                    // Create an empty entity via undo stack (undoable)
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity_count = scene_state.world().entities().len();
                        let name = format!("Empty_{}", entity_count);
                        let clipboard = crate::clipboard::ClipboardData {
                            version: 2,
                            entities: vec![crate::clipboard::ClipboardEntityData {
                                name: name.clone(),
                                pos: astraweave_core::IVec2 { x: 0, y: 0 },
                                height: 0.0,
                                rotation: 0.0,
                                rotation_x: 0.0,
                                rotation_z: 0.0,
                                scale: 1.0,
                                hp: 0,
                                team_id: 0,
                                ammo: 0,
                                cooldowns: Default::default(),
                                behavior_graph: None,
                                parent: None,
                            }],
                        };
                        let cmd = command::SpawnEntitiesCommand::new(
                            clipboard,
                            astraweave_core::IVec2 { x: 0, y: 0 },
                        );
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs
                                .push(format!("Create entity failed: {}", e));
                        } else {
                            // Select the newly created entity
                            if let Some(&last) = scene_state.world().entities().last() {
                                let em_id: u64 = last.into();
                                self.selected_entity = Some(em_id);
                                self.selection_set.primary = Some(em_id);
                                if let Some(viewport) = &mut self.viewport {
                                    viewport.set_selected_entity(Some(em_id));
                                }
                            }
                            scene_state.sync_all();
                            self.hierarchy_panel
                                .sync_with_world(scene_state.world_mut());
                            self.is_dirty = true;
                            self.console_logs
                                .push(format!("Created empty entity: {}", name));
                            self.status = format!("Created entity: {}", name);
                        }
                    }
                }
                tab_viewer::PanelEvent::SpawnArchetype { ref archetype } => {
                    // Spawn an entity from a named archetype via undo stack (undoable)
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity_count = scene_state.world().entities().len();
                        let name = format!("{}_{}", archetype, entity_count);
                        let (team_id, hp, ammo) = match archetype.as_str() {
                            "Player" => (0u8, 100i32, 30i32),
                            "Companion" => (0, 80, 20),
                            "Enemy" => (1, 50, 20),
                            "Boss" => (1, 500, 50),
                            "NPC" => (2, 100, 0),
                            "Prop" => (2, 10, 0),
                            "Trigger" => (2, 1, 0),
                            "Light" => (2, 1, 0),
                            "Camera" => (2, 1, 0),
                            _ => (0, 1, 0),
                        };
                        let clipboard = crate::clipboard::ClipboardData {
                            version: 2,
                            entities: vec![crate::clipboard::ClipboardEntityData {
                                name: name.clone(),
                                pos: astraweave_core::IVec2 { x: 0, y: 0 },
                                height: 0.0,
                                rotation: 0.0,
                                rotation_x: 0.0,
                                rotation_z: 0.0,
                                scale: 1.0,
                                hp,
                                team_id,
                                ammo,
                                cooldowns: Default::default(),
                                behavior_graph: None,
                                parent: None,
                            }],
                        };
                        let cmd = command::SpawnEntitiesCommand::new(
                            clipboard,
                            astraweave_core::IVec2 { x: 0, y: 0 },
                        );
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs
                                .push(format!("Spawn archetype failed: {}", e));
                        } else {
                            // Select the newly created entity
                            if let Some(&last) = scene_state.world().entities().last() {
                                let em_id: u64 = last.into();
                                self.selected_entity = Some(em_id);
                                self.selection_set.primary = Some(em_id);
                                if let Some(viewport) = &mut self.viewport {
                                    viewport.set_selected_entity(Some(em_id));
                                }
                            }
                            scene_state.sync_all();
                            self.hierarchy_panel
                                .sync_with_world(scene_state.world_mut());
                            self.is_dirty = true;
                            self.console_logs
                                .push(format!("Spawned {} entity: {}", archetype, name));
                            self.status = format!("Spawned {}: {}", archetype, name);
                        }
                    }
                }
                tab_viewer::PanelEvent::SpawnModel { ref name, ref path } => {
                    // Spawn a model entity via undo stack (undoable)
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity_count = scene_state.world().entities().len();
                        let entity_name = format!("{}_{}", name, entity_count);
                        let offset = entity_count as i32 * 3;
                        let clipboard = crate::clipboard::ClipboardData {
                            version: 2,
                            entities: vec![crate::clipboard::ClipboardEntityData {
                                name: entity_name.clone(),
                                pos: astraweave_core::IVec2 { x: offset, y: 0 },
                                height: 0.0,
                                rotation: 0.0,
                                rotation_x: 0.0,
                                rotation_z: 0.0,
                                scale: 1.0,
                                hp: 0,
                                team_id: 0,
                                ammo: 0,
                                cooldowns: Default::default(),
                                behavior_graph: None,
                                parent: None,
                            }],
                        };
                        let cmd = command::SpawnEntitiesCommand::new(
                            clipboard,
                            astraweave_core::IVec2 { x: 0, y: 0 },
                        );
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Spawn model failed: {}", e));
                        } else {
                            // Assign mesh to the newly created entity in EntityManager
                            if let Some(&last) = scene_state.world().entities().last() {
                                let em_id: u64 = last.into();
                                if let Some(em_entity) = self.entity_manager.get_mut(em_id) {
                                    em_entity.mesh = Some(path.clone());
                                }
                                self.selected_entity = Some(em_id);
                                self.selection_set.primary = Some(em_id);
                                if let Some(viewport) = &mut self.viewport {
                                    viewport.set_selected_entity(Some(em_id));
                                }
                            }
                            scene_state.sync_all();
                            self.hierarchy_panel
                                .sync_with_world(scene_state.world_mut());
                            self.is_dirty = true;
                            self.console_logs
                                .push(format!("Spawned model entity: {} ({})", entity_name, path));
                            self.status = format!("Spawned model: {}", entity_name);
                        }
                    }
                }
                tab_viewer::PanelEvent::DeleteEntity(entity_id) => {
                    // Delete entity via undo stack, then sync EntityManager
                    if let (Some(scene_state), Some(entity)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let delete_cmd = command::DeleteEntitiesCommand::new(vec![entity]);
                        if let Err(e) = self.undo_stack.execute(
                            delete_cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Delete failed: {}", e));
                        } else {
                            scene_state.sync_entity(entity);
                            self.hierarchy_panel
                                .sync_with_world(scene_state.world_mut());
                            // Remove from EntityManager AFTER successful delete command
                            // NOTE: undo will not restore EntityManager entry (known limitation)
                            self.entity_manager.remove(entity_id);
                        }
                    }
                    if self.selected_entity == Some(entity_id) {
                        self.selected_entity = None;
                        self.selection_set.primary = None;
                        if let Some(viewport) = &mut self.viewport {
                            viewport.set_selected_entity(None);
                        }
                    }
                    self.is_dirty = true;
                    self.status = format!("Deleted entity {}", entity_id);
                }
                tab_viewer::PanelEvent::DuplicateEntity(entity_id) => {
                    // Duplicate entity via undo stack (undoable)
                    if let (Some(scene_state), Some(src_eid)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let offset = astraweave_core::IVec2 { x: 1, y: 1 };
                        let cmd = command::DuplicateEntitiesCommand::new(vec![src_eid], offset);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Duplicate failed: {}", e));
                        } else {
                            // Select the duplicated entity
                            if let Some(&last) = scene_state.world().entities().last() {
                                let em_id: u64 = last.into();
                                self.selected_entity = Some(em_id);
                                self.selection_set.primary = Some(em_id);
                                if let Some(viewport) = &mut self.viewport {
                                    viewport.set_selected_entity(Some(em_id));
                                }
                            }
                            scene_state.sync_all();
                            self.hierarchy_panel
                                .sync_with_world(scene_state.world_mut());
                            self.is_dirty = true;
                            self.status = format!("Duplicated entity {}", entity_id);
                        }
                    }
                }
                tab_viewer::PanelEvent::PanelClosed(panel) => {
                    self.status = format!("Closed {} panel", panel.title());
                }
                tab_viewer::PanelEvent::PanelFocused(panel) => {
                    self.status = format!("Focused {} panel", panel.title());
                }
                tab_viewer::PanelEvent::AddPanel(panel) => {
                    self.dock_layout.add_panel(panel);
                    self.status = format!("Added {} panel", panel.title());
                }
                tab_viewer::PanelEvent::MaterialChanged {
                    name,
                    property,
                    value,
                } => {
                    self.status = format!("Material '{}': {} = {:.2}", name, property, value);
                    // Apply material property change to the selected entity
                    if let Some(entity_id) = self.selected_entity {
                        if let Some(entity) = self.entity_manager.get_mut(entity_id) {
                            match property.as_str() {
                                "metallic" => entity.material.metallic = value,
                                "roughness" => entity.material.roughness = value,
                                "emission_strength" => {
                                    entity.material.emissive = glam::Vec3::splat(value);
                                }
                                "alpha" | "opacity" => {
                                    entity.material.base_color.w = value;
                                }
                                _ => {}
                            }
                            self.is_dirty = true;
                        }
                    }
                }
                tab_viewer::PanelEvent::AnimationPlayStateChanged { is_playing } => {
                    if is_playing {
                        self.status = "Animation playing".to_string();
                    } else {
                        self.status = "Animation paused".to_string();
                    }
                }
                tab_viewer::PanelEvent::AnimationFrameChanged { frame } => {
                    self.dock_tab_viewer.set_animation_frame(frame);
                    self.status = format!("Animation frame: {}", frame);
                }
                tab_viewer::PanelEvent::AnimationKeyframeAdded {
                    track_index,
                    frame,
                    value,
                } => {
                    self.status = format!(
                        "Added keyframe at frame {} (track {}, value {:.2})",
                        frame, track_index, value
                    );
                }
                tab_viewer::PanelEvent::ThemeChanged(theme) => {
                    self.status = format!("Theme changed to {:?}", theme);
                }
                tab_viewer::PanelEvent::BuildRequested { target, profile } => {
                    self.status = format!("Build requested: {} ({})", target, profile);
                    let target_idx = match target.as_str() {
                        "Windows" => 0,
                        "Linux" => 1,
                        "macOS" => 2,
                        "WebGL" => 3,
                        _ => 0,
                    };
                    let profile_idx = match profile.as_str() {
                        "Debug" => 0,
                        "Release" => 1,
                        _ => 0,
                    };
                    self.dock_tab_viewer
                        .set_build_config(target_idx, profile_idx);
                    self.dock_tab_viewer.start_build();
                    self.dock_tab_viewer
                        .add_build_output(format!("Starting {} build for {}...", profile, target));
                }
                tab_viewer::PanelEvent::ConsoleCleared => {
                    self.console_logs.clear();
                    self.status = "Console cleared".to_string();
                }
                tab_viewer::PanelEvent::AssetSelected(asset) => {
                    self.status = format!("Selected asset: {}", asset);
                }
                tab_viewer::PanelEvent::BehaviorNodeSelected(node_id) => {
                    self.status = format!("Selected behavior node: {}", node_id);
                    self.dock_tab_viewer.select_behavior_node(Some(node_id));
                }
                tab_viewer::PanelEvent::GraphNodeSelected(node_id) => {
                    self.status = format!("Selected graph node: {}", node_id);
                    self.dock_tab_viewer.select_graph_node(Some(node_id));
                }
                tab_viewer::PanelEvent::HierarchySearchChanged(search) => {
                    self.dock_tab_viewer.set_hierarchy_search(search.clone());
                    self.status = format!("Hierarchy search: {}", search);
                }
                tab_viewer::PanelEvent::ConsoleSearchChanged(search) => {
                    self.dock_tab_viewer.set_console_search(search.clone());
                    self.status = format!("Console search: {}", search);
                }
                tab_viewer::PanelEvent::RefreshSceneStats => {
                    let entity_count = self.entity_manager.count();
                    let stats = self.dock_tab_viewer.scene_stats_mut();
                    stats.total_entities = entity_count;
                    stats.loaded_assets = self.asset_registry.count();
                    self.status = format!("Scene refreshed: {} entities", entity_count);
                }
                tab_viewer::PanelEvent::AddComponent {
                    entity_id,
                    component_type,
                } => {
                    // Build default value for the component type
                    let default_value = match component_type.as_str() {
                        "Transform" => serde_json::json!({"x": 0, "y": 0, "z": 0}),
                        "Health" => serde_json::json!({"hp": 100}),
                        "Team" => serde_json::json!({"id": 0}),
                        "Ammo" => serde_json::json!({"count": 30}),
                        "Sprite" => serde_json::json!({"texture": ""}),
                        "Collider" => {
                            serde_json::json!({"shape": "box", "size": [1, 1, 1], "is_trigger": false})
                        }
                        "RigidBody" => {
                            serde_json::json!({"type": "dynamic", "mass": 1.0, "drag": 0.0, "angular_drag": 0.05, "use_gravity": true})
                        }
                        "Script" => serde_json::json!({"path": ""}),
                        "MovementScript" => movement_scripts::MovementScript::default().to_json(),
                        "Audio" => {
                            serde_json::json!({"clip": "", "volume": 1.0, "spatial": true, "looping": false, "play_on_start": false})
                        }
                        "Light" => {
                            serde_json::json!({"type": "point", "intensity": 1.0, "color_r": 1.0, "color_g": 1.0, "color_b": 1.0, "range": 10.0, "cast_shadows": true})
                        }
                        "Camera" => {
                            serde_json::json!({"fov": 60.0, "near": 0.1, "far": 1000.0})
                        }
                        "Particle" => {
                            serde_json::json!({"emission_rate": 10.0, "lifetime": 2.0, "start_size": 0.1, "speed": 5.0, "shape": "cone", "looping": true})
                        }
                        _ => serde_json::json!({}),
                    };
                    // Route through undo stack (undoable)
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let cmd = command::AddComponentCommand::new(
                            entity_id,
                            component_type.clone(),
                            default_value,
                        );
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Add component failed: {e}"));
                        }
                        self.is_dirty = true;
                    }
                    self.status = format!("Added {} to entity {}", component_type, entity_id);
                }
                tab_viewer::PanelEvent::RemoveComponent {
                    entity_id,
                    component_type,
                } => {
                    // Route through undo stack (undoable)
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let cmd =
                            command::RemoveComponentCommand::new(entity_id, component_type.clone());
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs
                                .push(format!("Remove component failed: {e}"));
                        }
                        self.is_dirty = true;
                    }
                    self.status = format!("Removed {} from entity {}", component_type, entity_id);
                }
                tab_viewer::PanelEvent::HealthChanged { entity_id, new_hp } => {
                    // Route through undo stack for undoable health changes
                    if let (Some(scene_state), Some(eid)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let old_hp = scene_state.world().health(eid).map(|h| h.hp).unwrap_or(0);
                        let cmd = command::EditHealthCommand::new(eid, old_hp, new_hp);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Edit health failed: {}", e));
                        } else {
                            scene_state.sync_entity(eid);
                        }
                    }
                    // EntityManager JSON is updated by EditHealthCommand (via undo stack)
                    self.is_dirty = true;
                }
                tab_viewer::PanelEvent::TeamChanged {
                    entity_id,
                    new_team_id,
                } => {
                    // Route through undo stack for undoable team changes
                    if let (Some(scene_state), Some(eid)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let old_team = scene_state
                            .world()
                            .team(eid)
                            .unwrap_or(astraweave_core::Team { id: 0 });
                        let new_team = astraweave_core::Team { id: new_team_id };
                        let cmd = command::EditTeamCommand::new(eid, old_team, new_team);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Edit team failed: {}", e));
                        } else {
                            scene_state.sync_entity(eid);
                        }
                    }
                    // EntityManager JSON is updated by EditTeamCommand (via undo stack)
                    self.is_dirty = true;
                }
                tab_viewer::PanelEvent::AmmoChanged {
                    entity_id,
                    new_ammo,
                } => {
                    // Route through undo stack for undoable ammo changes
                    if let (Some(scene_state), Some(eid)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let old_rounds =
                            scene_state.world().ammo(eid).map(|a| a.rounds).unwrap_or(0);
                        let cmd = command::EditAmmoCommand::new(eid, old_rounds, new_ammo);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Edit ammo failed: {}", e));
                        } else {
                            scene_state.sync_entity(eid);
                        }
                    }
                    // EntityManager JSON is updated by EditAmmoCommand (via undo stack)
                    self.is_dirty = true;
                }
                tab_viewer::PanelEvent::ComponentDataChanged {
                    entity_id,
                    component_type,
                    data,
                } => {
                    // Route through undo stack for undoable component data changes
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let cmd = command::ComponentDataChangedCommand::new(
                            entity_id,
                            component_type.clone(),
                            data,
                        );
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs
                                .push(format!("Component edit failed: {e}"));
                        }
                        self.is_dirty = true;
                    }
                    self.status = format!("Updated {} on entity {}", component_type, entity_id);
                }
                tab_viewer::PanelEvent::MaterialPropertyChanged {
                    entity_id,
                    property,
                    value,
                } => {
                    // Route through undo stack for undoable material property changes
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let cmd = command::MaterialPropertyChangedCommand::new(
                            entity_id,
                            property.clone(),
                            value,
                        );
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Material edit failed: {e}"));
                        }
                        self.is_dirty = true;
                    }
                    self.status = format!("Material {} changed on entity {}", property, entity_id);
                }
                tab_viewer::PanelEvent::MaterialTextureChanged {
                    entity_id,
                    slot,
                    path,
                } => {
                    // Route through undo stack for undoable texture changes
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let cmd = command::MaterialTextureChangedCommand::new(
                            entity_id,
                            slot.clone(),
                            path,
                        );
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs
                                .push(format!("Texture change failed: {e}"));
                        }
                        self.is_dirty = true;
                    }
                    self.status = format!("Texture {} changed on entity {}", slot, entity_id);
                }
                tab_viewer::PanelEvent::EntityRenamed {
                    entity_id,
                    ref new_name,
                } => {
                    // Route through undo stack for undoable rename
                    if let (Some(scene_state), Some(entity)) =
                        (self.scene_state.as_mut(), entity_id_to_world(entity_id))
                    {
                        let old_name = scene_state.world().name(entity).unwrap_or("").to_string();
                        let cmd =
                            command::RenameEntityCommand::new(entity, old_name, new_name.clone());
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.console_logs.push(format!("Rename failed: {}", e));
                        }
                    }
                    // EntityManager name is updated by RenameEntityCommand (via undo stack)
                    self.is_dirty = true;
                    self.console_logs
                        .push(format!("Renamed entity {} to '{}'", entity_id, new_name));
                    self.status = format!("Renamed entity to '{}'", new_name);
                }
                tab_viewer::PanelEvent::ViewportViewModeChanged(mode) => {
                    let mode_names = ["Shaded", "Wireframe", "Unlit", "Normals", "UVs"];
                    self.status = format!(
                        "Viewport view mode: {}",
                        mode_names.get(mode).unwrap_or(&"Unknown")
                    );
                    // Sync to actual viewport shading mode
                    if let Some(viewport) = &mut self.viewport {
                        use crate::viewport::toolbar::ShadingMode;
                        let shading = match mode {
                            0 => ShadingMode::Lit,       // Shaded
                            1 => ShadingMode::Wireframe, // Wireframe
                            2 => ShadingMode::Unlit,     // Unlit
                            _ => ShadingMode::Lit,       // Normals/UVs → fallback to Lit
                        };
                        viewport.toolbar_mut().shading_mode = shading;
                    }
                }
                tab_viewer::PanelEvent::ViewportGizmoModeChanged(mode) => {
                    let mode_names = ["Translate", "Rotate", "Scale"];
                    self.status =
                        format!("Gizmo mode: {}", mode_names.get(mode).unwrap_or(&"Unknown"));
                    self.dock_tab_viewer.set_viewport_gizmo_mode(mode);
                    if let Some(viewport) = &mut self.viewport {
                        let gs = viewport.gizmo_state_mut();
                        match mode {
                            0 => gs.start_translate(),
                            1 => gs.start_rotate(),
                            2 => gs.start_scale(false),
                            _ => {}
                        }
                    }
                }
                tab_viewer::PanelEvent::ViewportGizmoSpaceChanged(space) => {
                    self.status = format!(
                        "Gizmo space: {}",
                        if space == 0 { "Local" } else { "World" }
                    );
                    self.dock_tab_viewer.set_viewport_gizmo_space(space);
                    if let Some(viewport) = &mut self.viewport {
                        viewport.gizmo_state_mut().local_space = space == 0;
                    }
                }
                tab_viewer::PanelEvent::ViewportOverlayToggled { overlay, enabled } => {
                    self.status = format!(
                        "Viewport overlay '{}': {}",
                        overlay,
                        if enabled { "enabled" } else { "disabled" }
                    );
                    self.dock_tab_viewer.set_viewport_overlay(&overlay, enabled);
                    if let Some(viewport) = &mut self.viewport {
                        match overlay.as_str() {
                            "Grid" | "grid" => viewport.toolbar_mut().show_grid = enabled,
                            "Stats" | "stats" => viewport.toolbar_mut().show_stats = enabled,
                            _ => {}
                        }
                    }
                }
                tab_viewer::PanelEvent::ViewportCameraChanged {
                    fov,
                    near,
                    far,
                    speed,
                } => {
                    if let Some(viewport) = &mut self.viewport {
                        let mut cam = viewport.camera().clone();
                        cam.fov = fov;
                        cam.near = near;
                        cam.far = far;
                        viewport.set_camera(cam);
                    }
                    self.status = format!(
                        "Camera: FOV={:.0}°, Clip={:.2}-{:.0}, Speed={:.1}",
                        fov, near, far, speed
                    );
                }
                tab_viewer::PanelEvent::ViewportFocusOnSelection => {
                    if let Some(selected_id) = self.selected_entity {
                        if let Some(entity) = self.entity_manager.get(selected_id) {
                            if let Some(viewport) = &mut self.viewport {
                                let mut cam = viewport.camera().clone();
                                cam.set_focal_point(glam::Vec3::new(
                                    entity.position.x,
                                    entity.position.y,
                                    entity.position.z,
                                ));
                                viewport.set_camera(cam);
                            }
                        }
                    }
                    self.status = "Focusing on selection...".to_string();
                }
                tab_viewer::PanelEvent::ViewportResetCamera => {
                    if let Some(viewport) = &mut self.viewport {
                        viewport.set_camera(OrbitCamera::default());
                    }
                    self.status = "Camera reset to default position".to_string();
                }
                tab_viewer::PanelEvent::ViewportCameraPreset(preset) => {
                    if let Some(viewport) = &mut self.viewport {
                        let mut cam = viewport.camera().clone();
                        match preset.as_str() {
                            "front" => {
                                cam.set_yaw(0.0);
                                cam.set_pitch(0.0);
                            }
                            "back" => {
                                cam.set_yaw(std::f32::consts::PI);
                                cam.set_pitch(0.0);
                            }
                            "top" => {
                                cam.set_yaw(0.0);
                                cam.set_pitch(-std::f32::consts::FRAC_PI_2 + 0.01);
                            }
                            "bottom" => {
                                cam.set_yaw(0.0);
                                cam.set_pitch(std::f32::consts::FRAC_PI_2 - 0.01);
                            }
                            "left" => {
                                cam.set_yaw(std::f32::consts::FRAC_PI_2);
                                cam.set_pitch(0.0);
                            }
                            "right" => {
                                cam.set_yaw(-std::f32::consts::FRAC_PI_2);
                                cam.set_pitch(0.0);
                            }
                            _ => {}
                        }
                        viewport.set_camera(cam);
                    }
                    self.status = format!("Camera preset applied: {}", preset);
                }
                tab_viewer::PanelEvent::ResetLayout => {
                    // Reset dock state to default layout
                    self.dock_layout = DockLayout::from_preset(LayoutPreset::Default);
                    self.status = "Layout reset to default".to_string();
                }
                tab_viewer::PanelEvent::HdriLoaded { ref path } => {
                    if let Some(viewport) = &self.viewport {
                        if let Ok(mut renderer) = viewport.renderer().lock() {
                            match renderer.load_hdri(path) {
                                Ok(()) => {
                                    self.status = format!(
                                        "HDRI loaded: {}",
                                        path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("unknown")
                                    );
                                }
                                Err(e) => {
                                    self.status = format!("Failed to load HDRI: {e}");
                                    tracing::error!("HDRI load error: {e:#}");
                                }
                            }
                        }
                    }
                }
                tab_viewer::PanelEvent::HdriCleared => {
                    if let Some(viewport) = &self.viewport {
                        if let Ok(mut renderer) = viewport.renderer().lock() {
                            renderer.clear_hdri();
                        }
                    }
                    self.status = "HDRI skybox removed".to_string();
                }
                tab_viewer::PanelEvent::TerrainReady => {
                    let src_chunks = self.dock_tab_viewer.terrain_gpu_chunks();
                    if let Some(viewport) = &self.viewport {
                        viewport.upload_terrain_chunks_raw(&src_chunks);
                    }

                    // Use pre-computed scatter placements from background thread
                    // (avoids expensive O(n²) Poisson disk sampling on the main thread)
                    let scatter_placements = self.dock_tab_viewer.take_cached_scatter_placements();
                    // Upload scatter placements to the GPU-instanced scatter renderer
                    // (high-performance: 65K instances @ 60fps with background mesh loading)
                    if !scatter_placements.is_empty() {
                        if let Some(viewport) = &self.viewport {
                            if let Ok(mut renderer) = viewport.renderer().lock() {
                                let count = scatter_placements.len();
                                renderer.set_scatter_placements(scatter_placements);
                                tracing::info!("Scatter: {count} placements uploaded to renderer");
                            }
                        }
                    }

                    // Inject BiomePack ground textures into the GPU texture layers
                    // that the splat generator actually references (layers 0-7).
                    // Map pack texture names to the correct material layer index:
                    //   "sand"/"ground" → 1 (desert primary)
                    //   "cliff"/"rock"  → 3 (mountain rock, used on slopes)
                    //   "mud"/"dirt"    → 5 (depressions)
                    //   "stone"/"gravel"→ 7 (mid-elevation breakup)
                    //   "grass"         → 0 (grassland)
                    //   "snow"          → 4 (tundra)
                    // Unmapped names get injected starting at layer 12 (custom slots).
                    let pack_info = self
                        .dock_tab_viewer
                        .cached_biome_pack()
                        .map(|p| (p.name.clone(), p.ground_textures.len()));
                    tracing::debug!("Biomepack check: {:?}", pack_info);
                    if let Some(pack) = self.dock_tab_viewer.cached_biome_pack() {
                        tracing::debug!(
                            "Biomepack: name='{}', ground_textures={}, root={:?}",
                            pack.name,
                            pack.ground_textures.len(),
                            pack.root_dir
                        );
                        if let Some(viewport) = &self.viewport {
                            if let Ok(mut renderer) = viewport.renderer().lock() {
                                let root = pack.root_dir.clone();
                                let mut next_custom_layer = 12u32;
                                for gt in pack.ground_textures.iter() {
                                    let name_lower = gt.name.to_lowercase();
                                    let layer_index = if name_lower.contains("sand")
                                        || name_lower.contains("ground")
                                        || name_lower.contains("gravel")
                                    {
                                        1u32
                                    } else if name_lower.contains("cliff")
                                        || name_lower.contains("rock")
                                    {
                                        3
                                    } else if name_lower.contains("mud")
                                        || name_lower.contains("dirt")
                                    {
                                        5
                                    } else if name_lower.contains("stone") {
                                        7
                                    } else if name_lower.contains("grass") {
                                        0
                                    } else if name_lower.contains("snow") {
                                        4
                                    } else {
                                        let idx = next_custom_layer;
                                        if next_custom_layer < 17 {
                                            next_custom_layer += 1;
                                        }
                                        idx
                                    };
                                    let load_tex = |rel_path: &Option<String>| -> Vec<u8> {
                                        let gray_fallback =
                                            || vec![128u8, 128, 128, 255].repeat(2048 * 2048);
                                        let Some(rel) = rel_path else {
                                            return gray_fallback();
                                        };
                                        let full = root.join(rel);
                                        let img = match image::open(&full) {
                                            Ok(i) => i,
                                            Err(e) => {
                                                tracing::warn!(
                                                    "BiomePack texture load failed: {:?}: {e}",
                                                    full
                                                );
                                                return gray_fallback();
                                            }
                                        };
                                        // Convert to RGBA and resize to 2048 if needed
                                        // Use Triangle filter (fast) instead of CatmullRom (slow)
                                        let rgba = img.to_rgba8();
                                        if rgba.width() == 2048 && rgba.height() == 2048 {
                                            rgba.into_raw()
                                        } else {
                                            image::imageops::resize(
                                                &rgba,
                                                2048,
                                                2048,
                                                image::imageops::FilterType::Triangle,
                                            )
                                            .into_raw()
                                        }
                                    };

                                    let albedo = load_tex(&gt.diffuse);
                                    // Normal maps: skip .exr.png files — these were
                                    // tonemapped during blend export (Reinhard) which
                                    // destroys normal vector data. Use flat normal fallback.
                                    let normal = if gt
                                        .normal
                                        .as_ref()
                                        .is_some_and(|p| p.contains(".exr."))
                                    {
                                        tracing::info!(
                                            "BiomePack: skipping tonemapped normal map '{:?}' — using flat normal fallback",
                                            gt.normal
                                        );
                                        // Flat tangent-space normal: (0, 0, 1) encoded as (128, 128, 255)
                                        vec![128u8, 128, 255, 255].repeat(2048 * 2048)
                                    } else {
                                        load_tex(&gt.normal)
                                    };
                                    // Build MRA from roughness (no metallic/AO in pack format)
                                    // Skip .exr.png roughness — tonemapped values are wrong for PBR
                                    let mra = if let Some(rough_path) = &gt.roughness {
                                        if rough_path.contains(".exr.") {
                                            tracing::info!(
                                                "BiomePack: skipping tonemapped roughness '{rough_path}' — using default MRA"
                                            );
                                            vec![0u8, 128, 255, 255].repeat(2048 * 2048)
                                        } else {
                                            let full = root.join(rough_path);
                                            match image::open(&full) {
                                                Ok(img) => {
                                                    let converted = img.to_rgba8();
                                                    let rgba = if converted.width() == 2048
                                                        && converted.height() == 2048
                                                    {
                                                        converted
                                                    } else {
                                                        image::imageops::resize(
                                                            &converted,
                                                            2048,
                                                            2048,
                                                            image::imageops::FilterType::Triangle,
                                                        )
                                                    };
                                                    let mut mra_data = vec![0u8; 2048 * 2048 * 4];
                                                    for (j, pixel) in
                                                        rgba.as_raw().chunks_exact(4).enumerate()
                                                    {
                                                        let roughness = pixel[0];
                                                        mra_data[j * 4] = 0; // metallic = 0
                                                        mra_data[j * 4 + 1] = roughness;
                                                        mra_data[j * 4 + 2] = 255; // AO = 1.0
                                                        mra_data[j * 4 + 3] = 255;
                                                    }
                                                    mra_data
                                                }
                                                Err(_) => {
                                                    vec![0u8, 128, 255, 255].repeat(2048 * 2048)
                                                }
                                            }
                                        } // end non-exr roughness branch
                                    } else {
                                        // Default MRA: metallic=0, roughness=0.5, AO=1.0
                                        vec![0u8, 128, 255, 255].repeat(2048 * 2048)
                                    };

                                    renderer.replace_terrain_texture_layer(
                                        layer_index,
                                        &albedo,
                                        &normal,
                                        &mra,
                                    );
                                    tracing::info!(
                                        "BiomePack: injected ground texture '{}' → layer {}",
                                        gt.name,
                                        layer_index
                                    );

                                    // For sand/ground textures, also overwrite layer 0 (grass)
                                    // to prevent green bleed from any code path that samples
                                    // the default grass texture (biome blender edge fallbacks,
                                    // shader defaults, interpolation artifacts).
                                    if layer_index == 1 {
                                        renderer.replace_terrain_texture_layer(
                                            0, &albedo, &normal, &mra,
                                        );
                                        tracing::info!(
                                            "BiomePack: also overwrote layer 0 (grass) with sand to prevent green bleed"
                                        );
                                    }
                                }
                            }
                        }
                    }

                    let chunk_count = src_chunks.len();
                    self.status = format!("Terrain generated: {} chunks uploaded", chunk_count);

                    // Update hierarchy panel so terrain chunks are visible
                    self.dock_tab_viewer.set_terrain_chunk_count(chunk_count);

                    // Auto-adjust camera so the terrain is visible (prevents camera
                    // being submerged inside tall mountain terrain).
                    let (min_h, max_h, avg_h) = self.dock_tab_viewer.terrain_height_stats();
                    if let Some(viewport) = &mut self.viewport {
                        viewport.camera_mut().frame_terrain(min_h, max_h, avg_h);
                    }
                }
                tab_viewer::PanelEvent::TerrainBrushUpdate => {
                    // Incremental GPU update — only re-upload dirty chunk vertex buffers
                    let dirty = self.dock_tab_viewer.take_terrain_dirty_chunks();
                    if let Some(viewport) = &self.viewport {
                        for (chunk_index, verts) in &dirty {
                            let gpu_verts: Vec<crate::viewport::types::TerrainVertex> = verts
                                .iter()
                                .map(|v| crate::viewport::types::TerrainVertex {
                                    position: v.position,
                                    normal: v.normal,
                                    uv: v.uv,
                                    biome_weights_0: v.biome_weights_0,
                                    biome_weights_1: v.biome_weights_1,
                                    material_ids: v.material_ids,
                                    material_weights: v.material_weights,
                                })
                                .collect();
                            viewport.update_terrain_chunk_vertices(*chunk_index, &gpu_verts);
                        }
                    }
                }
            }
        }
    }

    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top")
            .min_height(24.0)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(6, 2)),
            )
            .show(ctx, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.label(egui::RichText::new("AstraWeave").strong().size(14.0));
                    ui.separator();
                    MenuBar::show(ui, self);

                    // Center: play controls
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // FPS indicator (right-most)
                        let frame_time = self.runtime.stats().frame_time_ms;
                        let fps_color = if self.current_fps >= 55.0 {
                            egui::Color32::from_rgb(100, 255, 100)
                        } else if self.current_fps >= 30.0 {
                            egui::Color32::from_rgb(255, 200, 100)
                        } else {
                            egui::Color32::from_rgb(255, 100, 100)
                        };
                        ui.label(
                            egui::RichText::new(format!("{:.0} FPS", self.current_fps))
                                .color(fps_color)
                                .small(),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.1}ms", frame_time))
                                .color(egui::Color32::from_gray(160))
                                .small(),
                        );
                        ui.separator();

                        // Play controls (center-right area)
                        self.show_play_controls_compact(ui);
                    });
                });
            });
    }

    /// Compact play controls for the menu bar row
    fn show_play_controls_compact(&mut self, ui: &mut egui::Ui) {
        let (mode_text, color) = match self.editor_mode {
            EditorMode::Edit => ("Edit", egui::Color32::LIGHT_GRAY),
            EditorMode::Play => ("\u{25b6} Playing", egui::Color32::from_rgb(80, 200, 120)),
            EditorMode::Paused => ("\u{23f8} Paused", egui::Color32::from_rgb(255, 180, 50)),
        };
        ui.colored_label(color, egui::RichText::new(mode_text).small());

        let play_enabled = self.editor_mode.is_editing() || self.editor_mode.is_paused();
        if ui
            .add_enabled(play_enabled, egui::Button::new("\u{25b6}").small())
            .on_hover_text("Play (F5)")
            .clicked()
        {
            self.request_play();
        }

        let pause_enabled = self.editor_mode.is_playing();
        if ui
            .add_enabled(pause_enabled, egui::Button::new("\u{23f8}").small())
            .on_hover_text("Pause (F6)")
            .clicked()
        {
            self.request_pause();
        }

        let stop_enabled = !self.editor_mode.is_editing();
        if ui
            .add_enabled(stop_enabled, egui::Button::new("\u{23f9}").small())
            .on_hover_text("Stop (F7)")
            .clicked()
        {
            self.request_stop();
        }

        let step_enabled = self.editor_mode.is_paused();
        if ui
            .add_enabled(step_enabled, egui::Button::new("\u{23ed}").small())
            .on_hover_text("Step Frame (F8)")
            .clicked()
        {
            self.request_step();
        }
    }

    fn show_legacy_central_panel(&mut self, ctx: &egui::Context) {
        // Legacy layout - original CentralPanel rendering
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(0.0).fill(egui::Color32::from_rgb(0, 255, 0))) // GREEN for legacy
            .show(ctx, |ui| {
                // 3D Viewport (Phase 1.1 - Babylon.js-style editor)
                if let Some(viewport) = &mut self.viewport {
                    // Phase 14: Update viewport HUD with selection count
                    viewport.set_selection_count(self.selection_set.count());

                    // Phase 4: Visual indicator for play mode
                    let viewport_frame = if !self.editor_mode.is_editing() {
                        let border_color = if self.editor_mode.is_playing() {
                            egui::Color32::from_rgb(100, 200, 100)
                        } else {
                            egui::Color32::from_rgb(255, 180, 50)
                        };

                        egui::Frame::NONE
                            .stroke(egui::Stroke::new(3.0, border_color))
                            .inner_margin(4.0)
                    } else {
                        egui::Frame::NONE
                    };

                    viewport_frame.show(ui, |ui| {
                        ui.heading("3D Viewport");
                        ui.label(
                            "Phase 1.1 Complete: Grid rendering active, texture display in progress",
                        );

                        ui.horizontal(|ui| {
                            ui.label("Snapping:");

                            ui.checkbox(&mut self.snapping_config.grid_enabled, "Grid");

                            ui.label("Size:");
                            let mut grid_size_idx = match self.snapping_config.grid_size {
                                s if (s - 0.5).abs() < 0.01 => 0,
                                s if (s - 1.0).abs() < 0.01 => 1,
                                s if (s - 2.0).abs() < 0.01 => 2,
                                _ => 1,
                            };

                            if ui
                                .add(
                                    egui::Slider::new(&mut grid_size_idx, 0..=2)
                                        .show_value(false)
                                        .custom_formatter(|n, _| match n as usize {
                                            0 => "0.5".to_string(),
                                            1 => "1.0".to_string(),
                                            2 => "2.0".to_string(),
                                            _ => "1.0".to_string(),
                                        }),
                                )
                                .changed()
                            {
                                self.snapping_config.grid_size = match grid_size_idx {
                                    0 => 0.5,
                                    1 => 1.0,
                                    2 => 2.0,
                                    _ => 1.0,
                                };
                            }

                            ui.separator();
                            ui.checkbox(&mut self.snapping_config.angle_enabled, "Angle");
                            ui.label(format!("{}°", self.snapping_config.angle_increment));

                            ui.separator();

                            // Engine PBR Rendering toggle
                            let mut use_pbr = viewport.renderer().lock().map(|r| r.use_engine_rendering()).unwrap_or(false);
                            if ui.checkbox(&mut use_pbr, "Engine PBR").on_hover_text("Enable full PBR mesh rendering instead of cube placeholders").changed() {
                                if let Ok(mut renderer) = viewport.renderer().lock() {
                                    renderer.set_use_engine_rendering(use_pbr);
                                }
                            }
                        });

                        ui.separator();

                        // Render viewport (takes 70% width, full available height)
                        let runtime_state = self.runtime.state();
                        let is_playing = self.runtime.is_playing();
                        if runtime_state == RuntimeState::Editing && self.scene_state.is_none() {
                            self.scene_state =
                                Some(EditorSceneState::new(Self::create_default_world()));
                        }

                        let mut edited_world = false;
                        let world_to_render = if runtime_state == RuntimeState::Editing {
                            self.scene_state.as_mut().map(|state| state.world_mut())
                        } else {
                            self.runtime.sim_world_mut()
                        };

                        if let Some(world) = world_to_render {
                            if runtime_state == RuntimeState::Editing {
                                edited_world = true;
                            }
                            if let Err(e) = viewport.ui(
                                ui,
                                world,
                                &mut self.entity_manager,
                                &mut self.undo_stack,
                                Some(&mut self.prefab_manager),
                                is_playing,
                            ) {
                                self.console_logs.push(format!("Viewport error: {}", e));
                                warn!("Viewport error: {}", e);
                            }

                            // Forward captured game input to runtime
                            if let Some(gi) = viewport.take_game_input() {
                                self.runtime.inject_input(gi);
                            }
                        } else {
                            ui.label("No world available for rendering");
                        }

                        if edited_world {
                            if let Some(scene_state) = self.scene_state.as_mut() {
                                scene_state.sync_all();
                            }
                        }

                        // Sync selected entity from viewport to app state
                        // (handles both selection and deselection)
                        let vp_selected = viewport.selected_entity();
                        if vp_selected != self.selected_entity {
                            self.selected_entity = vp_selected;
                            self.selection_set.primary = vp_selected;
                        }

                        // Sync snapping settings from viewport toolbar to EditorApp
                        self.snapping_config.grid_enabled = viewport.toolbar().snap_enabled;
                        self.snapping_config.grid_size = viewport.toolbar().snap_size;
                        self.snapping_config.angle_enabled = viewport.toolbar().angle_snap_enabled;
                        self.snapping_config.angle_increment = viewport.toolbar().angle_snap_degrees;

                        // Sync play state from editor mode to viewport toolbar
                        viewport.toolbar_mut().play_state = match self.editor_mode {
                            EditorMode::Edit => crate::viewport::toolbar::PlayState::Editing,
                            EditorMode::Play => crate::viewport::toolbar::PlayState::Playing,
                            EditorMode::Paused => crate::viewport::toolbar::PlayState::Paused,
                        };

                        ui.add_space(10.0);
                    });

                    // Process play actions from viewport toolbar (outside viewport_frame borrow)
                    if let Some(action) = viewport.toolbar_mut().take_play_action() {
                        match action {
                            crate::viewport::toolbar::PlayAction::Play => self.request_play(),
                            crate::viewport::toolbar::PlayAction::Pause => self.request_pause(),
                            crate::viewport::toolbar::PlayAction::Stop => self.request_stop(),
                            crate::viewport::toolbar::PlayAction::Step => self.request_step(),
                        }
                    }

                    ui.separator();
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Auto-expand Console when simulation is running (so users see feedback)
                    let console_open = self.runtime.is_playing() || !self.console_logs.is_empty();

                    let scene_entity_count = self.active_world().map(|w| w.entities().len()).unwrap_or(0);
                    let scene_hier_header = format!("Scene Hierarchy ({} entities)", scene_entity_count);
                    ui.collapsing(scene_hier_header, |ui| self.show_scene_hierarchy(ui));
                    if self.show_inspector_panel {
                        let inspector_header = if let Some(entity_id) = self.selection_set.primary {
                            if let Ok(entity) = u32::try_from(entity_id) {
                                if let Some(world) = self.active_world() {
                                    let name = world.name(entity)
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| format!("Entity_{}", entity));
                                    format!("Inspector - {}", name)
                                } else {
                                    "Inspector".to_string()
                                }
                            } else {
                                "Inspector".to_string()
                            }
                        } else {
                            "Inspector (no selection)".to_string()
                        };
                        ui.collapsing(inspector_header, |ui| self.show_inspector(ui));
                    }

                    if self.show_console_panel {
                        // Console section with auto-expand when active
                        let console_header = format!("Console ({} messages)", self.console_logs.len());
                        egui::CollapsingHeader::new(console_header)
                            .default_open(console_open)
                            .show(ui, |ui| self.show_console(ui));
                    }

                    ui.collapsing("Scene Statistics", |ui| {
                        self.scene_stats_panel.show_inline(ui);
                    });

                    ui.collapsing("Performance Profiler", |ui| {
                        self.profiler_panel.show(ui);
                    });

                    let runtime_state = self.runtime.state();
                    let tick_count = self.runtime.stats().tick_count;
                    let profiler_header = match runtime_state {
                        RuntimeState::Editing => "Profiler [Editing]".to_string(),
                        RuntimeState::Playing => format!("Profiler [Playing - Tick {}]", tick_count),
                        RuntimeState::Paused => format!("Profiler [Paused - Tick {}]", tick_count),
                        RuntimeState::SteppingOneFrame => format!("Profiler [Step - Tick {}]", tick_count),
                    };
                    ui.collapsing(profiler_header, |ui| self.show_profiler(ui));
                    let graph_node_count = self.graph_panel.total_node_count();
                    let graph_header = format!("Behavior Graph Editor ({} nodes)", graph_node_count);
                    ui.collapsing(graph_header, |ui| {
                        self.show_behavior_graph_editor(ui)
                    });
                    ui.collapsing("Dialogue Graph Editor", |ui| {
                        self.show_dialogue_graph_editor(ui)
                    });
                    ui.collapsing("Quest Graph Editor", |ui| self.show_quest_graph_editor(ui));
                    ui.collapsing("Material Editor", |ui| self.show_material_editor(ui));
                    ui.collapsing("Material Inspector", |ui| {
                        self.material_inspector.show(ui, ctx)
                    });
                    ui.collapsing("Terrain Painter", |ui| self.show_terrain_painter(ui));
                    ui.collapsing("Navmesh Controls", |ui| self.show_navmesh_controls(ui));
                    ui.collapsing("Voxel Editor", |ui| self.show_voxel_editor(ui));
                    ui.collapsing("Asset Inspector", |ui| self.show_asset_inspector(ui));
                });
            });
    }

    fn show_scene_hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scene Hierarchy");

        // Collect entity data first to avoid borrow issues
        let entity_data: Vec<(Entity, String, Option<_>, Option<_>, Option<_>)> =
            if let Some(world) = self.active_world() {
                world
                    .entities()
                    .iter()
                    .map(|&entity| {
                        let name = world
                            .name(entity)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("Entity_{}", entity));
                        let pose = world.pose(entity);
                        let health = world.health(entity);
                        let team = world.team(entity);
                        (entity, name, pose, health, team)
                    })
                    .collect()
            } else {
                Vec::new()
            };

        if entity_data.is_empty() {
            if self.active_world().is_none() {
                ui.label("No scene loaded");
            } else {
                ui.label("No entities in scene");
            }
        } else {
            ui.label(format!("{} entities:", entity_data.len()));
            ui.separator();

            let mut new_selection: Option<u64> = None;
            let current_primary = self.selection_set.primary;

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for (entity, name, pose, health, team) in &entity_data {
                        let is_selected = current_primary == Some(u64::from(*entity));

                        let response = ui.selectable_label(is_selected, format!("{}", name));

                        if response.clicked() {
                            new_selection = Some(u64::from(*entity));
                        }

                        // Show entity info on hover
                        response.on_hover_ui(|ui| {
                            ui.label(format!("ID: {}", entity));
                            if let Some(pose) = pose {
                                ui.label(format!(
                                    "Position: ({}, {:.1}, {})",
                                    pose.pos.x, pose.height, pose.pos.y
                                ));
                                ui.label(format!("Scale: {:.2}", pose.scale));
                            }
                            if let Some(health) = health {
                                ui.label(format!("Health: {}", health.hp));
                            }
                            if let Some(team) = team {
                                ui.label(format!("Team: {}", team.id));
                            }
                        });
                    }
                });

            // Apply selection change after the scroll area
            if let Some(sel) = new_selection {
                self.selection_set.primary = Some(sel);
            }
        }
    }

    fn show_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");

        // Show selected entity's components
        if let Some(entity_id) = self.selection_set.primary {
            if let Ok(entity) = u32::try_from(entity_id) {
                if let Some(world) = self.active_world() {
                    let name = world
                        .name(entity)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Entity_{}", entity));

                    ui.label(format!("Selected: {} (ID: {})", name, entity));
                    ui.separator();

                    // Transform section
                    ui.collapsing("Transform", |ui| {
                        if let Some(pose) = world.pose(entity) {
                            ui.horizontal(|ui| {
                                ui.label("Position:");
                                ui.label(format!(
                                    "({}, {:.1}, {})",
                                    pose.pos.x, pose.height, pose.pos.y
                                ));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Rotation:");
                                ui.label(format!("{:.1}°", pose.rotation.to_degrees()));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Scale:");
                                ui.label(format!("{:.2}", pose.scale));
                            });
                        } else {
                            ui.label("No transform component");
                        }
                    });

                    // Health section
                    ui.collapsing("Health", |ui| {
                        if let Some(health) = world.health(entity) {
                            ui.horizontal(|ui| {
                                ui.label("HP:");
                                let hp_color = if health.hp > 50 {
                                    egui::Color32::GREEN
                                } else if health.hp > 20 {
                                    egui::Color32::YELLOW
                                } else {
                                    egui::Color32::RED
                                };
                                ui.colored_label(hp_color, format!("{}", health.hp));
                            });
                        } else {
                            ui.label("No health component");
                        }
                    });

                    // Team section
                    ui.collapsing("Team", |ui| {
                        if let Some(team) = world.team(entity) {
                            ui.horizontal(|ui| {
                                ui.label("Team ID:");
                                ui.label(format!("{}", team.id));
                            });
                        } else {
                            ui.label("No team component");
                        }
                    });

                    // Ammo section
                    ui.collapsing("Ammo", |ui| {
                        if let Some(ammo) = world.ammo(entity) {
                            ui.horizontal(|ui| {
                                ui.label("Rounds:");
                                ui.label(format!("{}", ammo.rounds));
                            });
                        } else {
                            ui.label("No ammo component");
                        }
                    });
                } else {
                    ui.label("No scene loaded");
                }
            } else {
                ui.label("Invalid entity ID");
            }
        } else {
            ui.label("No entity selected");
            ui.label("Select an entity in the Scene Hierarchy or viewport");
        }
    }

    fn show_console(&mut self, ui: &mut egui::Ui) {
        let action = self
            .console_panel
            .show_with_logs(ui, &mut self.console_logs);
        match action {
            panels::console_panel::ConsoleAction::SpawnEntity(entity_type) => {
                let id = self.entity_manager.create(entity_type.clone());
                self.console_logs
                    .push(format!("Spawned entity '{}' (id: {})", entity_type, id));
            }
            panels::console_panel::ConsoleAction::ListEntities => {
                let entities = self.entity_manager.entities();
                if entities.is_empty() {
                    self.console_logs.push("No entities in scene.".into());
                } else {
                    self.console_logs
                        .push(format!("Entities ({}):", entities.len()));
                    let mut sorted: Vec<_> = entities.iter().collect();
                    sorted.sort_by_key(|(id, _)| *id);
                    for (id, e) in &sorted {
                        self.console_logs.push(format!("  [{}] {}", id, e.name));
                    }
                }
            }
            panels::console_panel::ConsoleAction::Clear
            | panels::console_panel::ConsoleAction::None => {}
        }
    }

    fn show_profiler(&mut self, ui: &mut egui::Ui) {
        ui.heading("Profiler");
        if self.profiler_data.is_empty() {
            ui.label("No runtime telemetry yet – press Play to sample frame data.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    for data in self.profiler_data.iter().rev() {
                        ui.label(data);
                    }
                });
        }
    }

    fn selected_entity_handle(&self) -> Option<Entity> {
        self.selection_set
            .primary
            .and_then(|id| u32::try_from(id).ok())
    }

    #[allow(dead_code)] // Will be used when hierarchy panel wires entity labels
    fn resolve_entity_label(&self, entity: Entity) -> String {
        self.scene_state
            .as_ref()
            .and_then(|state| state.world().name(entity).map(|s| s.to_string()))
            .unwrap_or_else(|| format!("Entity_{}", entity))
    }

    fn load_behavior_graph_from_selection(&mut self) {
        let Some(entity) = self.selected_entity_handle() else {
            self.console_logs
                .push("Select an entity before loading its behavior graph.".into());
            return;
        };

        let Some(scene_state) = self.scene_state.as_ref() else {
            self.console_logs
                .push("No scene loaded – cannot pull behavior graphs.".into());
            return;
        };

        let entity_name = scene_state
            .world()
            .name(entity)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Entity_{}", entity));
        let current_graph = scene_state.world().behavior_graph(entity).cloned();

        match current_graph {
            Some(graph) => {
                self.behavior_graph_doc = BehaviorGraphDocument::from_runtime(&graph);
                self.behavior_graph_doc.rebuild_next_id();
                self.behavior_graph_binding =
                    Some(BehaviorGraphBinding::new(entity, entity_name.clone()));
                self.console_logs.push(format!(
                    "Loaded behavior graph from {} (#{}) into the editor.",
                    entity_name, entity
                ));
            }
            None => {
                self.behavior_graph_doc = BehaviorGraphDocument::new_default();
                self.behavior_graph_binding =
                    Some(BehaviorGraphBinding::new(entity, entity_name.clone()));
                self.console_logs.push(format!(
                    "{} had no behavior graph; starting from the default template.",
                    entity_name
                ));
            }
        }
    }

    fn apply_behavior_graph_to_selection(&mut self) {
        let Some(entity) = self.selected_entity_handle() else {
            self.console_logs
                .push("Select an entity before applying a behavior graph.".into());
            return;
        };

        let runtime_graph = match self.behavior_graph_doc.to_runtime() {
            Ok(graph) => graph,
            Err(err) => {
                self.console_logs
                    .push(format!("Behavior graph is invalid: {}", err));
                return;
            }
        };

        let Some(scene_state) = self.scene_state.as_mut() else {
            self.console_logs
                .push("No scene loaded – cannot apply behavior graphs.".into());
            return;
        };

        let entity_name = scene_state
            .world()
            .name(entity)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Entity_{}", entity));
        scene_state
            .world_mut()
            .set_behavior_graph(entity, runtime_graph);
        scene_state.sync_entity(entity);
        self.behavior_graph_binding = Some(BehaviorGraphBinding::new(entity, entity_name.clone()));
        self.console_logs.push(format!(
            "Applied behavior graph to {} (#{}) and synced the scene state.",
            entity_name, entity
        ));
    }

    fn spawn_prefab_from_drag(&mut self, prefab_path: PathBuf, spawn_pos: (i32, i32)) {
        let _span = span!(Level::INFO, "spawn_prefab", path = %prefab_path.display(), pos = ?(spawn_pos.0, spawn_pos.1)).entered();

        let Some(scene_state) = self.scene_state.as_mut() else {
            warn!("No scene loaded - cannot instantiate prefabs");
            self.console_logs
                .push("No scene loaded – cannot instantiate prefabs.".into());
            return;
        };

        let prefab_name = prefab_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");

        match self.prefab_manager.instantiate_prefab(
            &prefab_path,
            scene_state.world_mut(),
            spawn_pos,
        ) {
            Ok(root_entity) => {
                scene_state.sync_entity(root_entity);
                self.selected_entity = Some(u64::from(root_entity));
                info!(
                    "Instantiated prefab '{}' at ({}, {}) - root entity #{}",
                    prefab_name, spawn_pos.0, spawn_pos.1, root_entity
                );
                self.console_logs.push(format!(
                    "Instantiated prefab '{}' at ({}, {}). Root entity: #{}",
                    prefab_name, spawn_pos.0, spawn_pos.1, root_entity
                ));
                self.status = format!("Spawned prefab: {}", prefab_name);
            }
            Err(err) => {
                error!("Failed to instantiate prefab '{}': {}", prefab_name, err);
                self.console_logs.push(format!(
                    "Failed to instantiate prefab '{}': {}",
                    prefab_name, err
                ));
                self.status = format!("Failed to spawn prefab: {}", prefab_name);
            }
        }
    }

    // =========================================================================
    // Week 5 Day 3-4: Prefab Workflow Enhancements
    // =========================================================================

    /// Create a prefab from a selected entity
    fn create_prefab_from_entity(&mut self, entity: Entity) {
        let Some(scene_state) = self.scene_state.as_ref() else {
            self.log("No scene loaded".to_string());
            return;
        };

        let world = scene_state.world();
        let name = world
            .name(entity)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Entity_{}", entity));

        match self.prefab_manager.create_prefab(world, entity, &name) {
            Ok(path) => {
                self.log(format!("Created prefab: {}", path.display()));
                self.toast_success(format!("Created prefab: {}", name));
                self.status = format!("Created prefab: {}", name);
                self.hierarchy_panel.mark_as_prefab_instance(entity);
            }
            Err(e) => {
                self.log(format!("Failed to create prefab: {}", e));
                self.toast_error(format!("Failed to create prefab: {}", e));
            }
        }
    }

    /// Apply overrides from an entity instance back to its prefab source file
    fn apply_overrides_to_prefab(&mut self, entity: Entity) {
        let Some(scene_state) = self.scene_state.as_ref() else {
            self.log("No scene loaded".to_string());
            return;
        };

        match self
            .prefab_manager
            .apply_overrides_to_prefab(entity, scene_state.world())
        {
            Ok(()) => {
                self.log(format!("Applied overrides to prefab for entity {}", entity));
                self.toast_success("Applied overrides to prefab".to_string());
                self.status = "Applied overrides to prefab".to_string();
            }
            Err(e) => {
                self.log(format!("Failed to apply overrides: {}", e));
                self.toast_error(format!("Failed to apply overrides: {}", e));
            }
        }
    }

    /// Revert an entity to match its original prefab values
    fn revert_to_original_prefab(&mut self, entity: Entity) {
        let Some(scene_state) = self.scene_state.as_mut() else {
            self.log("No scene loaded".to_string());
            return;
        };

        match self
            .prefab_manager
            .revert_instance_to_prefab(entity, scene_state.world_mut())
        {
            Ok(()) => {
                scene_state.sync_entity(entity);
                self.log(format!(
                    "Reverted entity {} to original prefab values",
                    entity
                ));
                self.toast_success("Reverted to original prefab".to_string());
                self.status = "Reverted to original prefab".to_string();
            }
            Err(e) => {
                self.log(format!("Failed to revert: {}", e));
                self.toast_error(format!("Failed to revert: {}", e));
            }
        }
    }

    /// Break the connection between an entity and its prefab source
    fn break_prefab_connection(&mut self, entity: Entity) {
        match self.prefab_manager.break_prefab_connection(entity) {
            Ok(()) => {
                self.hierarchy_panel.unmark_as_prefab_instance(entity);
                self.log(format!("Broke prefab connection for entity {}", entity));
                self.toast_success(
                    "Broke prefab connection - entity is now standalone".to_string(),
                );
                self.status = "Prefab connection broken".to_string();
            }
            Err(e) => {
                self.log(format!("Failed to break prefab connection: {}", e));
                self.toast_error(format!("Failed to break connection: {}", e));
            }
        }
    }

    /// Enter prefab editing mode: load the prefab source into the editor for in-place editing
    fn enter_prefab_editing(&mut self, entity: Entity) {
        let source_path = match self.prefab_manager.find_instance(entity) {
            Some(inst) => inst.source.clone(),
            None => {
                self.toast_error("Entity is not a prefab instance".to_string());
                return;
            }
        };

        // Load the prefab data to verify it's valid
        match PrefabData::load_from_file(&source_path) {
            Ok(data) => {
                self.editing_prefab_path = Some(source_path.clone());
                self.log(format!(
                    "Editing prefab '{}' from {:?}",
                    data.name, source_path
                ));
                self.toast_success(format!("Editing prefab: {}", data.name));
                self.status = format!("Prefab Edit Mode: {}", data.name);
            }
            Err(e) => {
                self.log(format!("Failed to load prefab {:?}: {}", source_path, e));
                self.toast_error(format!("Failed to load prefab: {}", e));
            }
        }
    }

    /// Exit prefab editing mode and optionally save changes back to the prefab file
    #[allow(dead_code)] // Will be called from prefab editor toolbar
    fn exit_prefab_editing(&mut self, save: bool) {
        if let Some(path) = self.editing_prefab_path.take() {
            if save {
                // Collect instances and apply overrides
                let mut errors = Vec::new();
                if let Some(scene_state) = self.scene_state.as_ref() {
                    let world = scene_state.world();
                    let instances: Vec<Entity> = self
                        .prefab_manager
                        .find_instances_by_source(&path)
                        .iter()
                        .map(|inst| inst.root_entity)
                        .collect();

                    for entity in instances {
                        if let Err(e) = self.prefab_manager.apply_overrides_to_prefab(entity, world)
                        {
                            errors.push(format!("Failed to save prefab overrides: {}", e));
                        }
                    }
                }
                for err in errors {
                    self.log(err);
                }
                self.toast_success("Prefab saved".to_string());
                self.log("Exited prefab edit mode (saved)".to_string());
            } else {
                self.toast_success("Exited prefab edit mode (discarded)".to_string());
                self.log("Exited prefab edit mode (discarded)".to_string());
            }
            self.status = "Ready".to_string();
        }
    }

    /// Sync hierarchy panel's prefab instance tracking with prefab manager
    fn sync_hierarchy_prefab_instances(&mut self) {
        let prefab_entities: Vec<_> = self.prefab_manager.get_all_prefab_entities().collect();
        self.hierarchy_panel
            .sync_prefab_instances(prefab_entities.into_iter());
    }

    /// Process pending hierarchy panel actions
    fn process_hierarchy_actions(&mut self) {
        use crate::panels::hierarchy_panel::HierarchyAction;

        let actions = self.hierarchy_panel.take_pending_actions();
        for action in actions {
            match action {
                HierarchyAction::CreatePrefab(entity) => {
                    self.create_prefab_from_entity(entity);
                }
                HierarchyAction::DeleteEntity(entity) => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        // Use undo-able command instead of direct deletion
                        let delete_cmd = command::DeleteEntitiesCommand::new(vec![entity]);
                        if let Err(e) = self.undo_stack.execute(
                            delete_cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.log(format!("Delete failed: {}", e));
                        } else {
                            scene_state.sync_entity(entity);
                            self.log(format!("Deleted entity {}", entity));
                            if self.selected_entity == Some(u64::from(entity)) {
                                self.selected_entity = None;
                            }
                        }
                    }
                }
                HierarchyAction::DuplicateEntity(entity) => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        // Use clipboard to duplicate with offset
                        let clipboard = crate::clipboard::ClipboardData::from_entities(
                            scene_state.world(),
                            &[entity],
                        );
                        let offset = astraweave_core::IVec2 { x: 1, y: 1 };
                        match clipboard.spawn_entities(scene_state.world_mut(), offset) {
                            Ok(spawned) => {
                                for e in &spawned {
                                    scene_state.sync_entity(*e);
                                }
                                if let Some(&first) = spawned.first() {
                                    self.selected_entity = Some(u64::from(first));
                                }
                                self.log(format!("Duplicated entity {} -> {:?}", entity, spawned));
                            }
                            Err(e) => {
                                self.log(format!("Duplicate failed: {}", e));
                            }
                        }
                    }
                }
                HierarchyAction::FocusEntity(entity) => {
                    self.selected_entity = Some(u64::from(entity));
                    self.log(format!("Focused on entity {}", entity));
                }
                HierarchyAction::BreakPrefabConnection(entity) => {
                    self.break_prefab_connection(entity);
                }
                HierarchyAction::ApplyOverridesToPrefab(entity) => {
                    self.apply_overrides_to_prefab(entity);
                }
                HierarchyAction::RevertToOriginalPrefab(entity) => {
                    self.revert_to_original_prefab(entity);
                }
                HierarchyAction::SetParent(child, parent) => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let old_parent = scene_state.world().parent_of(child);
                        let cmd = command::SetParentCommand::new(child, parent, old_parent);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.log(format!("SetParent failed: {}", e));
                        } else {
                            scene_state.sync_entity(child);
                        }
                    }
                    if let Some(child_entity) = self.entity_manager.get_mut(u64::from(child)) {
                        child_entity.parent = Some(u64::from(parent));
                    }
                    self.log(format!("Set parent of {} to {}", child, parent));
                }
                HierarchyAction::Unparent(entity) => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let old_parent = scene_state.world().parent_of(entity);
                        let cmd = command::UnparentCommand::new(entity, old_parent);
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.log(format!("Unparent failed: {}", e));
                        } else {
                            scene_state.sync_entity(entity);
                        }
                    }
                    if let Some(e) = self.entity_manager.get_mut(u64::from(entity)) {
                        e.parent = None;
                    }
                    self.log(format!("Unparented entity {}", entity));
                }
                HierarchyAction::EditPrefab(entity) => {
                    self.enter_prefab_editing(entity);
                }
                HierarchyAction::Rename(entity, new_name) => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let old_name = scene_state
                            .world()
                            .name(entity)
                            .unwrap_or("Unknown")
                            .to_string();
                        let cmd =
                            command::RenameEntityCommand::new(entity, old_name, new_name.clone());
                        if let Err(e) = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.log(format!("Rename failed: {}", e));
                        } else {
                            scene_state.sync_entity(entity);
                        }
                    }
                    self.log(format!("Renamed entity {} to '{}'", entity, new_name));
                }
                HierarchyAction::DeleteWithChildren(entity) => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        // Collect entity + all descendants for cascading delete
                        let mut to_delete = vec![entity];
                        to_delete.extend(scene_state.world().descendants_of(entity));
                        let delete_cmd = command::DeleteEntitiesCommand::new(to_delete.clone());
                        if let Err(e) = self.undo_stack.execute(
                            delete_cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        ) {
                            self.log(format!("Cascading delete failed: {}", e));
                        } else {
                            for &e in &to_delete {
                                scene_state.sync_entity(e);
                            }
                            self.log(format!(
                                "Deleted entity {} and {} descendants",
                                entity,
                                to_delete.len() - 1
                            ));
                            if self.selected_entity == Some(u64::from(entity)) {
                                self.selected_entity = None;
                            }
                        }
                    }
                }
                HierarchyAction::MoveUp(entity) => {
                    self.hierarchy_panel.move_entity_up(entity);
                    self.log(format!("Moved entity {} up", entity));
                }
                HierarchyAction::MoveDown(entity) => {
                    self.hierarchy_panel.move_entity_down(entity);
                    self.log(format!("Moved entity {} down", entity));
                }
            }
        }
    }

    /// Process pending asset browser actions (button clicks, drag-drop, double-clicks)
    fn process_asset_browser_actions(&mut self) {
        // Collect pending actions from the dock tab viewer's asset browser
        let actions = self.dock_tab_viewer.take_asset_browser_actions();
        for action in actions {
            self.handle_asset_action(action);
        }
        // Handle drag-drop from the dock tab viewer's asset browser
        if let Some(dragged_path) = self.dock_tab_viewer.take_asset_browser_dragged_prefab() {
            self.handle_dragged_asset(dragged_path);
        }
        // Also check the standalone asset browser (backward compat)
        let standalone_actions = self.asset_browser.take_pending_actions();
        for action in standalone_actions {
            self.handle_asset_action(action);
        }
        if let Some(dragged_path) = self.asset_browser.take_dragged_prefab() {
            self.handle_dragged_asset(dragged_path);
        }
    }

    /// Process pending blend import panel actions (decomposition, pack gen, browse)
    fn process_blend_import_actions(&mut self) {
        use panels::blend_import_panel::BlendImportAction;

        let actions = self.dock_tab_viewer.take_blend_import_actions();
        if actions.is_empty() {
            return;
        }

        for action in actions {
            match action {
                BlendImportAction::StartDecomposition { blend_path } => {
                    let panel = self.dock_tab_viewer.blend_import_panel_mut();
                    let output_dir = panel.output_dir().to_path_buf();

                    // Resolve output_dir to absolute — the panel stores relative paths
                    // like "assets/imported/Namaqualand" which Blender can't resolve.
                    let output_dir = if output_dir.is_relative() {
                        std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join(&output_dir)
                    } else {
                        output_dir
                    };

                    // Check if already decomposed (fast path: manifest exists)
                    let stem = blend_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "imported".to_string());
                    let manifest = output_dir.join(&stem).join("manifest.json");
                    if manifest.exists() {
                        // Already decomposed — load cached manifest
                        panel.set_progress(0.5, "Loading cached decomposition...");
                        match self.load_cached_decomposition(&manifest) {
                            Ok(result) => {
                                // Collect mesh paths before moving assets into panel
                                let mesh_paths: Vec<String> = result
                                    .assets
                                    .iter()
                                    .map(|a| a.mesh_path.to_string_lossy().into_owned())
                                    .collect();

                                let p = self.dock_tab_viewer.blend_import_panel_mut();
                                p.set_decomposition_result(
                                    result.assets,
                                    result.hdri_paths,
                                    result.ground_texture_groups,
                                );

                                // Preload meshes into viewport
                                if let Some(viewport) = &self.viewport {
                                    let loaded = viewport.preload_gltf_meshes(&mesh_paths);
                                    if loaded > 0 {
                                        self.console_logs.push(format!(
                                            "[Blend Import] Preloaded {} cached meshes into viewport",
                                            loaded
                                        ));
                                    }
                                }

                                self.console_logs.push(format!(
                                    "[Blend Import] Loaded cached decomposition: {}",
                                    manifest.display()
                                ));
                                self.status = format!("Cached: {} decomposition loaded", stem);
                            }
                            Err(e) => {
                                let p = self.dock_tab_viewer.blend_import_panel_mut();
                                p.set_progress(0.0, &format!("Cache load failed: {e}"));
                                self.console_logs
                                    .push(format!("[Blend Import] Cache load error: {e}"));
                            }
                        }
                    } else {
                        // Spawn background decomposition thread
                        panel.set_progress(0.05, "Discovering Blender installation...");
                        self.console_logs.push(format!(
                            "[Blend Import] Decomposition started: {} -> {}",
                            blend_path.display(),
                            output_dir.display()
                        ));
                        self.console_logs.push(
                            "[Blend Import] Large files may take several minutes — use Cancel to abort.".into()
                        );
                        self.status = format!("Decomposing: {}", blend_path.display());

                        let (tx, rx) = std::sync::mpsc::channel();
                        self.decomp_receiver = Some(rx);

                        let blend_path_clone = blend_path.clone();
                        let output_dir_clone = output_dir.clone();
                        std::thread::spawn(move || {
                            let rt = match tokio::runtime::Runtime::new() {
                                Ok(rt) => rt,
                                Err(e) => {
                                    let _ = tx.send(DecompThreadMsg::Failed(format!(
                                        "Failed to create async runtime: {e}"
                                    )));
                                    return;
                                }
                            };
                            rt.block_on(async {
                                use astraweave_asset::blend_import::{BlendImporter, DecomposedAsset};

                                let _ = tx.send(DecompThreadMsg::Progress(
                                    0.1,
                                    "Discovering Blender installation...".into(),
                                ));

                                let mut importer = match BlendImporter::new().await {
                                    Ok(imp) => imp,
                                    Err(e) => {
                                        let _ = tx.send(DecompThreadMsg::Failed(
                                            format!("Blender not found: {e}"),
                                        ));
                                        return;
                                    }
                                };

                                let _ = tx.send(DecompThreadMsg::Progress(
                                    0.2,
                                    "Blender found — running scene decomposition...".into(),
                                ));

                                // Spawn a progress estimator that sends synthetic updates
                                // while the blocking decompose() call runs. Uses an
                                // asymptotic curve: progress = 0.2 + 0.7 * (1 - e^(-t/60))
                                // so it approaches 90% over ~3 minutes but never overshoots.
                                let tx_progress = tx.clone();
                                let progress_done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                                let progress_done_clone = progress_done.clone();
                                tokio::spawn(async move {
                                    let start = std::time::Instant::now();
                                    let stages = [
                                        (0.25, "Loading .blend file..."),
                                        (0.35, "Processing scene objects..."),
                                        (0.45, "Exporting meshes..."),
                                        (0.55, "Extracting textures..."),
                                        (0.65, "Processing materials..."),
                                        (0.72, "Generating manifest..."),
                                    ];
                                    let mut stage_idx = 0;
                                    loop {
                                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                        if progress_done_clone.load(std::sync::atomic::Ordering::Relaxed) {
                                            break;
                                        }
                                        let elapsed = start.elapsed().as_secs_f32();
                                        // Slow asymptotic curve that scales with scene complexity:
                                        // Uses t/300 (5-min time constant) so large scenes don't
                                        // plateau at 89% after 90s. Caps at 0.94 to leave room
                                        // for actual completion.
                                        let pct = (0.15 + 0.79 * (1.0 - (-elapsed / 300.0).exp())).min(0.94);
                                        let msg = if stage_idx < stages.len() && pct >= stages[stage_idx].0 {
                                            let m = stages[stage_idx].1;
                                            stage_idx += 1;
                                            m.to_string()
                                        } else {
                                            let mins = (elapsed / 60.0) as u32;
                                            let secs = (elapsed % 60.0) as u32;
                                            if mins > 0 {
                                                format!("Blender processing scene ({mins}m {secs}s elapsed — large scenes may take 5-15 minutes)...")
                                            } else {
                                                format!("Blender processing scene ({secs}s elapsed)...")
                                            }
                                        };
                                        if tx_progress.send(DecompThreadMsg::Progress(pct, msg)).is_err() {
                                            break;
                                        }
                                    }
                                });

                                let decomp_result = importer.decompose(&blend_path_clone, &output_dir_clone).await;
                                progress_done.store(true, std::sync::atomic::Ordering::Relaxed);
                                match decomp_result {
                                    Ok(result) => {
                                        if result.assets.is_empty() {
                                            // Read the raw JSON for diagnostics
                                            let result_json_path = output_dir_clone.join("decomposition_result.json");
                                            let raw_json_info = match std::fs::read_to_string(&result_json_path) {
                                                Ok(json) => {
                                                    // Parse to count raw asset entries
                                                    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&json) {
                                                        let raw_count = raw.get("assets")
                                                            .and_then(|a| a.as_array())
                                                            .map(|a| a.len())
                                                            .unwrap_or(0);
                                                        format!(
                                                            "Python reported total_objects={}, raw JSON has {} asset entries. \
                                                             If raw > 0, Rust deserialization dropped them (check stderr). \
                                                             JSON preview: {}",
                                                            result.total_objects,
                                                            raw_count,
                                                            &json[..json.len().min(2000)]
                                                        )
                                                    } else {
                                                        format!("decomposition_result.json exists but is not valid JSON: {}", &json[..json.len().min(500)])
                                                    }
                                                }
                                                Err(e) => format!("Could not read {}: {e}", result_json_path.display()),
                                            };
                                            tracing::warn!("[Blend Import] 0-asset diagnostics: {raw_json_info}");
                                            let _ = tx.send(DecompThreadMsg::Failed(
                                                format!(
                                                    "Blender processed the scene (total_objects={}) but 0 assets \
                                                     survived to the editor. {}",
                                                    result.total_objects,
                                                    if result.total_objects > 0 {
                                                        "Objects were found by Python but lost during Rust deserialization — check editor stderr for details."
                                                    } else {
                                                        "The scene genuinely contained 0 exportable mesh objects. \
                                                         Objects may have been excluded by name filters (Camera/Light) \
                                                         or have fewer than 3 vertices."
                                                    }
                                                ),
                                            ));
                                            return;
                                        }
                                        let assets = result.assets.iter().map(|a: &DecomposedAsset| {
                                            panels::blend_import_panel::DecomposedAssetEntry {
                                                name: a.name.clone(),
                                                category: a.category.clone(),
                                                mesh_path: PathBuf::from(&a.filename),
                                                vertex_count: a.vertex_count as u32,
                                                texture_count: a.textures.len(),
                                                dimensions: a.dimensions
                                                    .map(|d| [d[0] as f32, d[1] as f32, d[2] as f32])
                                                    .unwrap_or([0.0; 3]),
                                                include_in_pack: true,
                                            }
                                        }).collect();
                                        let hdri_paths: Vec<PathBuf> = result.hdris.iter()
                                            .map(|h| result.output_dir.join("hdri").join(&h.filename))
                                            .collect();
                                        let _ = tx.send(DecompThreadMsg::Done(DecompThreadResult {
                                            assets,
                                            hdri_paths,
                                            ground_texture_groups: Vec::new(),
                                            status_message: format!(
                                                "Decomposed {} assets in {:.1}s (Blender {})",
                                                result.assets.len(),
                                                result.duration.as_secs_f32(),
                                                result.blender_version,
                                            ),
                                        }));
                                    }
                                    Err(e) => {
                                        // Send Display message for the UI error box
                                        let display_msg = format!("{e}");
                                        // Send Debug representation to console for full diagnostics
                                        let debug_msg = format!("{e:?}");
                                        let _ = tx.send(DecompThreadMsg::Failed(display_msg));
                                        // Log Debug details — these include stderr, blender_output, etc.
                                        tracing::warn!("[Blend Import] Decomposition error: {debug_msg}");
                                    }
                                }
                            });
                        });
                    }
                }
                BlendImportAction::GenerateBiomePack {
                    output_dir,
                    pack_name,
                } => {
                    self.console_logs.push(format!(
                        "[Blend Import] Generating biome pack '{}' in {}",
                        pack_name,
                        output_dir.display()
                    ));

                    // Resolve to absolute path
                    let abs_output = if output_dir.is_relative() {
                        std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join(&output_dir)
                    } else {
                        output_dir.clone()
                    };
                    let manifest_path = abs_output.join("manifest.json");

                    match astraweave_terrain::BiomePack::from_manifest(&manifest_path) {
                        Ok(mut pack) => {
                            pack.name = pack_name.clone();
                            let safe_name: String = pack_name
                                .chars()
                                .map(|c| {
                                    if c.is_alphanumeric() || c == '_' || c == '-' {
                                        c
                                    } else {
                                        '_'
                                    }
                                })
                                .collect();
                            let pack_path = abs_output
                                .join(format!("{}.biomepack.json", safe_name.to_lowercase()));
                            match pack.save(&pack_path) {
                                Ok(()) => {
                                    self.console_logs.push(format!(
                                        "[Blend Import] Biome pack saved: {}",
                                        pack_path.display()
                                    ));
                                    let bip = self.dock_tab_viewer.blend_import_panel_mut();
                                    bip.set_pack_complete();
                                    bip.set_generated_pack_path(pack_path.clone());
                                    info!(
                                        "[Blend Import] Pack path set for zone creation: {}",
                                        pack_path.display()
                                    );
                                    self.status = format!(
                                        "Biome pack '{}' generated successfully",
                                        pack_name
                                    );
                                    // Refresh the biome options cache so dropdowns pick up the new pack
                                    terrain_integration::refresh_biome_options_cache();
                                }
                                Err(e) => {
                                    let msg = format!("Failed to save biome pack: {e}");
                                    self.console_logs.push(format!("[Blend Import] {}", msg));
                                    self.dock_tab_viewer.blend_import_panel_mut().set_error(msg);
                                }
                            }
                        }
                        Err(e) => {
                            let msg = format!("Failed to create biome pack from manifest: {e}");
                            self.console_logs.push(format!("[Blend Import] {}", msg));
                            self.dock_tab_viewer.blend_import_panel_mut().set_error(msg);
                        }
                    }
                }
                BlendImportAction::BrowseOutputDir { path } => {
                    // Resolve to absolute if relative
                    let abs_path = if path.is_relative() {
                        std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join(&path)
                    } else {
                        path.clone()
                    };
                    // Create the directory if it doesn't exist yet
                    if !abs_path.is_dir() {
                        let _ = std::fs::create_dir_all(&abs_path);
                    }
                    if abs_path.is_dir() {
                        self.asset_browser.navigate_to(abs_path);
                    }
                    self.console_logs
                        .push(format!("[Blend Import] Browse: {}", path.display()));
                    self.status = format!("Browsing: {}", path.display());
                }
                BlendImportAction::CancelDecomposition => {
                    self.decomp_receiver = None;
                    self.console_logs
                        .push("[Blend Import] Decomposition cancelled".into());
                    self.status = "Decomposition cancelled".into();
                }
                BlendImportAction::ClearSession => {
                    self.decomp_receiver = None;
                    self.console_logs
                        .push("[Blend Import] Session cleared".into());
                    self.status = "Blend import session cleared".into();
                }
                BlendImportAction::CreateReplicaZone { pack_path } => {
                    // Load the BiomePack to get scene footprint for default zone size.
                    // Try the path as-is first, then try relative to CWD.
                    let resolved_path = if pack_path.exists() {
                        pack_path.clone()
                    } else if pack_path.is_relative() {
                        let abs = std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."))
                            .join(&pack_path);
                        if abs.exists() {
                            abs
                        } else {
                            pack_path.clone()
                        }
                    } else {
                        pack_path.clone()
                    };
                    let pack_path_str = resolved_path.to_string_lossy().to_string();
                    self.console_logs.push(format!(
                        "[Blend Import] Creating replica zone from: {}",
                        resolved_path.display()
                    ));
                    match astraweave_terrain::BiomePack::load(&resolved_path) {
                        Ok(pack) => {
                            // Calculate default zone size from scene footprint
                            let footprint = pack.scene_footprint_area.unwrap_or(10000.0);
                            let half_side = (footprint.sqrt() / 2.0).max(32.0);

                            // Create a zone state centered at origin (user can reposition)
                            let zone = panels::blueprint_panel::ZoneState {
                                name: format!("{} (Replica)", pack.name),
                                vertices: vec![
                                    glam::Vec2::new(-half_side, -half_side),
                                    glam::Vec2::new(half_side, -half_side),
                                    glam::Vec2::new(half_side, half_side),
                                    glam::Vec2::new(-half_side, half_side),
                                ],
                                source: panels::blueprint_panel::ZoneSourceState::BlendScene {
                                    pack_path: pack_path_str,
                                    replica: true,
                                },
                                enabled: true,
                                priority: 10,
                                blend_margin: 8.0,
                                scene_scale_override: None,
                            };

                            self.dock_tab_viewer.add_blueprint_zone(zone);
                            self.console_logs.push(format!(
                                "[Blend Import] Created replica zone for '{}' ({:.0}m x {:.0}m)",
                                pack.name,
                                half_side * 2.0,
                                half_side * 2.0
                            ));
                            self.status = format!(
                                "Replica zone created — switch to Blueprint panel to generate"
                            );
                        }
                        Err(e) => {
                            let msg = format!("Failed to load biome pack for zone creation: {e}");
                            self.console_logs.push(format!("[Blend Import] {}", msg));
                            self.status = msg;
                        }
                    }
                }
            }
        }
    }

    /// Poll background blend decomposition thread for results.
    /// Drains all pending messages so progress updates don't lag behind.
    fn poll_blend_decomposition(&mut self) {
        let rx = match &self.decomp_receiver {
            Some(rx) => rx,
            None => return,
        };

        // Drain all pending messages — keep only the latest progress,
        // but process Done/Failed immediately.
        let mut latest_progress: Option<(f32, String)> = None;
        let mut final_msg: Option<DecompThreadMsg> = None;
        loop {
            match rx.try_recv() {
                Ok(DecompThreadMsg::Progress(pct, text)) => {
                    latest_progress = Some((pct, text));
                }
                Ok(msg @ DecompThreadMsg::Done(_)) | Ok(msg @ DecompThreadMsg::Failed(_)) => {
                    final_msg = Some(msg);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Thread died without sending — treat as error
                    self.decomp_receiver = None;
                    let panel = self.dock_tab_viewer.blend_import_panel_mut();
                    panel.reset_to_select("Select a .blend file to try again");
                    panel.set_error("Decomposition thread terminated unexpectedly".into());
                    self.console_logs.push(
                        "[Blend Import] ERROR: Background thread disconnected without sending a result.".into(),
                    );
                    return;
                }
            }
        }

        // Apply latest progress update
        if let Some((pct, text)) = latest_progress {
            let panel = self.dock_tab_viewer.blend_import_panel_mut();
            panel.set_progress(pct, &text);
        }

        // Process final message if received
        let msg = match final_msg {
            Some(m) => m,
            None => return,
        };

        match msg {
            DecompThreadMsg::Progress(..) => unreachable!(),
            DecompThreadMsg::Done(decomp) => {
                self.decomp_receiver = None;
                let msg = decomp.status_message.clone();

                // Collect mesh paths before moving assets into panel
                let mesh_paths: Vec<String> = decomp
                    .assets
                    .iter()
                    .map(|a| a.mesh_path.to_string_lossy().into_owned())
                    .collect();

                let panel = self.dock_tab_viewer.blend_import_panel_mut();
                panel.set_decomposition_result(
                    decomp.assets,
                    decomp.hdri_paths,
                    decomp.ground_texture_groups,
                );

                // Preload decomposed meshes into the viewport so they're
                // available when entities or scatter reference them.
                if let Some(viewport) = &self.viewport {
                    let loaded = viewport.preload_gltf_meshes(&mesh_paths);
                    if loaded > 0 {
                        self.console_logs.push(format!(
                            "[Blend Import] Preloaded {} meshes into viewport",
                            loaded
                        ));
                    }
                }

                self.console_logs.push(format!("[Blend Import] {}", msg));
                self.status = msg;
            }
            DecompThreadMsg::Failed(e) => {
                self.decomp_receiver = None;
                let panel = self.dock_tab_viewer.blend_import_panel_mut();
                panel.reset_to_select("Select a .blend file to try again");
                panel.set_error(e.clone());
                self.console_logs.push(format!("[Blend Import] ERROR: {e}"));
                self.status = "Decomposition failed".into();
            }
        }
    }

    /// Load a previously-cached decomposition manifest and convert to panel entries.
    fn load_cached_decomposition(
        &self,
        manifest_path: &std::path::Path,
    ) -> anyhow::Result<DecompThreadResult> {
        use astraweave_asset::blend_import::DecompositionResult;

        let data = std::fs::read_to_string(manifest_path)
            .with_context(|| format!("reading manifest {}", manifest_path.display()))?;
        let result: DecompositionResult =
            serde_json::from_str(&data).with_context(|| "parsing decomposition manifest")?;

        let assets = result
            .assets
            .iter()
            .map(|a| panels::blend_import_panel::DecomposedAssetEntry {
                name: a.name.clone(),
                category: a.category.clone(),
                mesh_path: PathBuf::from(&a.filename),
                vertex_count: a.vertex_count as u32,
                texture_count: a.textures.len(),
                dimensions: a
                    .dimensions
                    .map(|d| [d[0] as f32, d[1] as f32, d[2] as f32])
                    .unwrap_or([0.0; 3]),
                include_in_pack: true,
            })
            .collect();

        let hdri_paths: Vec<PathBuf> = result
            .hdris
            .iter()
            .map(|h| result.output_dir.join("hdri").join(&h.filename))
            .collect();

        Ok(DecompThreadResult {
            assets,
            hdri_paths,
            ground_texture_groups: Vec::new(),
            status_message: format!(
                "Loaded cached decomposition: {} assets",
                result.assets.len()
            ),
        })
    }

    /// Process pending blueprint panel actions (zone generation, save/load)
    fn process_blueprint_actions(&mut self) {
        use panels::blueprint_panel::BlueprintAction;

        let actions = self.dock_tab_viewer.take_blueprint_actions();
        if actions.is_empty() {
            return;
        }

        for action in actions {
            match action {
                BlueprintAction::GenerateZone { zone_index } => {
                    self.handle_generate_zone(zone_index);
                }
                BlueprintAction::GenerateAll => {
                    let zone_count = self.dock_tab_viewer.blueprint_zones().len();
                    let mut all_scatter = Vec::new();
                    let mut total_placements = 0usize;

                    // Determine the dominant biome from the first enabled zone
                    // and regenerate terrain with that biome if it differs from current.
                    if let Some(first_zone) = self.dock_tab_viewer.blueprint_zones().first() {
                        let target_biome = match &first_zone.source {
                            panels::blueprint_panel::ZoneSourceState::BiomePreset(name) => {
                                name.clone()
                            }
                            _ => "grassland".to_string(),
                        };
                        let seed = self.dock_tab_viewer.terrain_seed();
                        // Reconfigure terrain to match the zone biome
                        self.dock_tab_viewer
                            .trigger_terrain_generation(seed, &target_biome, 5);
                    }

                    for i in 0..zone_count {
                        let (scatter, patches, count) = self.generate_zone_results(i);
                        // Apply heightmap patches per zone (order matters for priority)
                        if !patches.is_empty() {
                            self.dock_tab_viewer.apply_zone_heightmap_patches(&patches);
                        }
                        all_scatter.extend(scatter);
                        total_placements += count;
                    }

                    // Apply ALL scatter at once so all zones render together
                    if !all_scatter.is_empty() {
                        if let Some(viewport) = &self.viewport {
                            viewport.set_scatter_placements(all_scatter);
                        }
                    }

                    self.console_logs.push(format!(
                        "[Blueprint] Generated all {} zones: {} total placements",
                        zone_count, total_placements
                    ));
                    self.status = format!(
                        "{} zones generated: {} objects placed",
                        zone_count, total_placements
                    );
                }
                BlueprintAction::ClearGeneration => {
                    // Clear zone registry — remove all previously generated zones
                    let registry = self.dock_tab_viewer.zone_registry_mut();
                    let ids: Vec<_> = registry.zones().iter().map(|z| z.id).collect();
                    for id in ids {
                        registry.remove_zone(id);
                    }
                    // Clear scatter from viewport
                    if let Some(viewport) = &self.viewport {
                        viewport.set_scatter_placements(Vec::new());
                    }
                    self.console_logs
                        .push("[Blueprint] Cleared generated content".into());
                    self.status = "Generation cleared".into();
                }
                BlueprintAction::SaveZones => {
                    self.handle_save_zones();
                }
                BlueprintAction::LoadZones => {
                    self.handle_load_zones();
                }
            }
        }

        // Sync zone overlay to viewport
        self.sync_zone_overlay();
    }

    /// Generate scatter results for a single zone without applying to viewport.
    /// Returns (scatter_placements, heightmap_patches, placement_count).
    fn generate_zone_results(
        &mut self,
        zone_index: usize,
    ) -> (
        Vec<terrain_integration::ScatterPlacement>,
        Vec<astraweave_terrain::HeightmapPatch>,
        usize,
    ) {
        use astraweave_terrain::{
            zone_scatter::ZoneScatterGenerator, BlueprintZone, PlacementMode, ZoneSource,
        };
        use panels::blueprint_panel::ZoneSourceState;

        let zones = self.dock_tab_viewer.blueprint_zones();
        let zone_state = match zones.get(zone_index) {
            Some(z) => z.clone(),
            None => {
                warn!("Blueprint: invalid zone index {}", zone_index);
                return (Vec::new(), Vec::new(), 0);
            }
        };

        if zone_state.vertices.len() < 3 {
            self.console_logs.push(format!(
                "[Blueprint] Zone '{}' has fewer than 3 vertices — skipping",
                zone_state.name
            ));
            return (Vec::new(), Vec::new(), 0);
        }

        // Convert panel ZoneState → terrain BlueprintZone
        let zone_id = self.dock_tab_viewer.zone_registry_mut().next_zone_id();
        let mut bz = BlueprintZone::new(zone_id, zone_state.name.clone());
        bz.vertices = zone_state
            .vertices
            .iter()
            .map(|v| glam::Vec2::new(v.x, v.y))
            .collect();
        bz.priority = zone_state.priority;
        bz.enabled = zone_state.enabled;
        bz.blend_margin = zone_state.blend_margin;

        bz.source = match &zone_state.source {
            ZoneSourceState::BiomePreset(name) => {
                let biome = match name.as_str() {
                    "Grassland" => astraweave_terrain::BiomeType::Grassland,
                    "Desert" => astraweave_terrain::BiomeType::Desert,
                    "Forest" => astraweave_terrain::BiomeType::Forest,
                    "Mountain" => astraweave_terrain::BiomeType::Mountain,
                    "Tundra" => astraweave_terrain::BiomeType::Tundra,
                    "Swamp" => astraweave_terrain::BiomeType::Swamp,
                    "Beach" => astraweave_terrain::BiomeType::Beach,
                    "River" => astraweave_terrain::BiomeType::River,
                    _ => astraweave_terrain::BiomeType::Grassland,
                };
                ZoneSource::BiomePreset(biome)
            }
            ZoneSourceState::BlendScene { pack_path, replica } => ZoneSource::BlendScene {
                pack_path: std::path::PathBuf::from(pack_path),
                placement_mode: if *replica {
                    PlacementMode::Replica
                } else {
                    PlacementMode::Inspired
                },
            },
        };

        bz.adaptive_scale_override = zone_state.scene_scale_override;

        // Register the zone
        self.dock_tab_viewer
            .zone_registry_mut()
            .add_zone(bz.clone());

        // Ensure terrain chunks exist — auto-generate if the user hasn't
        // clicked "Generate" in the terrain panel yet.
        // Use the zone's biome so terrain matches the zone configuration.
        let zone_biome = match &zone_state.source {
            ZoneSourceState::BiomePreset(name) => name.as_str(),
            _ => "grassland",
        };
        let zone_seed = self.dock_tab_viewer.terrain_seed();
        let auto_count = self
            .dock_tab_viewer
            .ensure_terrain_exists_with_biome(zone_biome, zone_seed);
        if auto_count > 0 {
            self.console_logs.push(format!(
                "[Blueprint] Auto-generated {} terrain chunks (biome: {}) for zone scatter",
                auto_count, zone_biome
            ));
        }

        // Run the scatter generator
        let terrain_chunks = self.dock_tab_viewer.collect_terrain_chunks();
        let chunk_refs: Vec<&astraweave_terrain::TerrainChunk> =
            terrain_chunks.iter().copied().collect();

        let generator = ZoneScatterGenerator::new(64.0, 65);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(42);

        match generator.generate_zone_scatter(&bz, &chunk_refs, seed) {
            Ok(result) => {
                let placement_count = result.placement_count();

                self.console_logs.push(format!(
                    "[Blueprint] Zone '{}' generated: {} placements",
                    zone_state.name, placement_count
                ));

                // Load BiomePack for dimension-aware mesh path resolution
                let loaded_pack = match &zone_state.source {
                    ZoneSourceState::BlendScene { pack_path, .. } => {
                        astraweave_terrain::BiomePack::load(&std::path::PathBuf::from(pack_path))
                            .ok()
                    }
                    _ => None,
                };
                let pack_ref = loaded_pack.as_ref();

                let scatter_placements: Vec<terrain_integration::ScatterPlacement> = result
                    .placements
                    .iter()
                    .map(|vi| {
                        terrain_integration::ScatterPlacement::from_zone_placement(vi, pack_ref)
                    })
                    .collect();

                (
                    scatter_placements,
                    result.heightmap_patches,
                    placement_count,
                )
            }
            Err(e) => {
                error!("Zone scatter generation failed: {}", e);
                self.console_logs.push(format!(
                    "[Blueprint] Generation failed for '{}': {}",
                    zone_state.name, e
                ));
                (Vec::new(), Vec::new(), 0)
            }
        }
    }

    /// Handle generation for a single zone by index in the blueprint panel.
    fn handle_generate_zone(&mut self, zone_index: usize) {
        let (scatter, patches, count) = self.generate_zone_results(zone_index);

        // Apply heightmap patches
        if !patches.is_empty() {
            self.dock_tab_viewer.apply_zone_heightmap_patches(&patches);
        }

        // Apply scatter to viewport
        if !scatter.is_empty() {
            if let Some(viewport) = &self.viewport {
                viewport.set_scatter_placements(scatter);
            }
        }

        if count > 0 {
            self.status = format!("Zone: {} objects placed", count);
        } else {
            self.status = "Zone: no placements generated".into();
        }
    }

    /// Save all blueprint panel zones to a .zones.json file.
    fn handle_save_zones(&mut self) {
        let path = std::path::PathBuf::from("assets/zones.json");
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let registry = self.dock_tab_viewer.zone_registry();
        match registry.save(&path) {
            Ok(()) => {
                self.console_logs.push(format!(
                    "[Blueprint] Saved {} zones to {}",
                    registry.len(),
                    path.display()
                ));
                self.status = format!("Zones saved to {}", path.display());
            }
            Err(e) => {
                error!("Failed to save zones: {}", e);
                self.console_logs
                    .push(format!("[Blueprint] Save failed: {}", e));
                self.status = "Zone save failed".into();
            }
        }
    }

    /// Load zones from a .zones.json file and sync into the blueprint panel.
    fn handle_load_zones(&mut self) {
        use astraweave_terrain::ZoneRegistry;
        use panels::blueprint_panel::{ZoneSourceState, ZoneState as PanelZoneState};

        let path = std::path::PathBuf::from("assets/zones.json");
        match ZoneRegistry::load(&path) {
            Ok(loaded) => {
                let count = loaded.len();

                // Convert terrain zones → panel zone states
                let panel_zones: Vec<PanelZoneState> = loaded
                    .zones()
                    .iter()
                    .map(|bz| {
                        let source = match &bz.source {
                            astraweave_terrain::ZoneSource::BiomePreset(bt) => {
                                ZoneSourceState::BiomePreset(format!("{:?}", bt))
                            }
                            astraweave_terrain::ZoneSource::BlendScene {
                                pack_path,
                                placement_mode,
                            } => ZoneSourceState::BlendScene {
                                pack_path: pack_path.display().to_string(),
                                replica: matches!(
                                    placement_mode,
                                    astraweave_terrain::PlacementMode::Replica
                                ),
                            },
                        };
                        PanelZoneState {
                            name: bz.name.clone(),
                            vertices: bz
                                .vertices
                                .iter()
                                .map(|v| glam::Vec2::new(v.x, v.y))
                                .collect(),
                            source,
                            priority: bz.priority,
                            enabled: bz.enabled,
                            blend_margin: bz.blend_margin,
                            scene_scale_override: bz.adaptive_scale_override,
                        }
                    })
                    .collect();

                // Replace registry and panel state
                *self.dock_tab_viewer.zone_registry_mut() = loaded;
                self.dock_tab_viewer.set_blueprint_zones(panel_zones);

                self.console_logs.push(format!(
                    "[Blueprint] Loaded {} zones from {}",
                    count,
                    path.display()
                ));
                self.status = format!("Loaded {} zones", count);

                // Sync overlay
                self.sync_zone_overlay();
            }
            Err(e) => {
                error!("Failed to load zones: {}", e);
                self.console_logs
                    .push(format!("[Blueprint] Load failed: {}", e));
                self.status = "Zone load failed".into();
            }
        }
    }

    /// Push zone polygon data to the viewport for 3D overlay rendering.
    fn sync_zone_overlay(&self) {
        use crate::viewport::BlueprintOverlay;
        use crate::viewport::ZoneOverlayData;
        use panels::blueprint_panel::ZoneSourceState;

        let zones = self.dock_tab_viewer.blueprint_zones();
        let overlay_data: Vec<ZoneOverlayData> = zones
            .iter()
            .enumerate()
            .map(|(_i, z)| {
                let color = match &z.source {
                    ZoneSourceState::BiomePreset(_) => [0.3, 0.8, 0.4],
                    ZoneSourceState::BlendScene { .. } => [0.3, 0.5, 0.9],
                };
                ZoneOverlayData {
                    vertices: z
                        .vertices
                        .iter()
                        .map(|v| glam::Vec2::new(v.x, v.y))
                        .collect(),
                    color,
                    selected: false, // Could track from panel if needed
                    editing: false,
                    y_height: 0.5, // Slightly above ground
                }
            })
            .collect();

        let lines = BlueprintOverlay::generate_lines(&overlay_data);
        if let Some(viewport) = &self.viewport {
            viewport.set_zone_overlay_lines(lines);
        }
    }

    /// Handle a dragged asset — import GLTF/GLB models directly, spawn prefabs for .prefab files
    fn handle_dragged_asset(&mut self, path: std::path::PathBuf) {
        // Compute spawn position from cursor ray → ground plane intersection
        let spawn_pos = self.last_cursor_ground_pos.unwrap_or((0, 0));

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "glb" | "gltf" | "obj" | "fbx" => {
                self.handle_asset_action(AssetAction::ImportModel { path });
                // Update the spawned entity position to the cursor ground position
                if let Some(entity_id) = self.selected_entity {
                    if let Some(em_entity) = self.entity_manager.get_mut(entity_id) {
                        em_entity.position =
                            glam::Vec3::new(spawn_pos.0 as f32, 0.0, spawn_pos.1 as f32);
                    }
                }
            }
            _ => {
                self.spawn_prefab_from_drag(path, spawn_pos);
            }
        }
    }

    /// Handle asset actions from the asset browser
    fn handle_asset_action(&mut self, action: AssetAction) {
        match action {
            AssetAction::ImportModel { path } => {
                let model_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("imported_model")
                    .to_string();

                if let Some(scene_state) = self.scene_state.as_mut() {
                    // Create a new entity with the model
                    let entity = scene_state.world_mut().spawn(
                        &model_name,
                        astraweave_core::IVec2 { x: 0, y: 0 },
                        astraweave_core::Team { id: 0 },
                        100,
                        0,
                    );
                    scene_state.sync_entity(entity);

                    let mesh_path_str = path.display().to_string();

                    // Get the editor entity and set its mesh
                    if let Some(editor_entity) = scene_state.get_editor_entity_mut(entity) {
                        editor_entity.set_mesh(mesh_path_str.clone());
                    }

                    // Sync mesh to the EntityManager so the viewport can find it
                    {
                        let em_id: u64 = entity.into();
                        if let Some(em_entity) = self.entity_manager.get_mut(em_id) {
                            em_entity.set_mesh(mesh_path_str);
                        } else {
                            let mut em_entity =
                                entity_manager::EditorEntity::new(em_id, model_name.clone());
                            em_entity.set_mesh(mesh_path_str);
                            if let Some(pose) = scene_state.world().pose(entity) {
                                em_entity.position = glam::Vec3::new(
                                    pose.pos.x as f32,
                                    pose.height,
                                    pose.pos.y as f32,
                                );
                            }
                            self.entity_manager.add(em_entity);
                        }
                    }

                    // Load the glTF model into the engine renderer
                    if let Some(viewport) = &self.viewport {
                        if let Err(e) = viewport.load_gltf_model(&model_name, &path) {
                            warn!("Failed to load glTF model into renderer: {}", e);
                            self.console_logs
                                .push(format!("glTF loading failed: {}", e));
                        } else {
                            debug!("Loaded glTF model '{}' into engine renderer", model_name);
                        }
                    }

                    self.selected_entity = Some(u64::from(entity));
                    info!("Imported model '{}' as entity #{}", model_name, entity);
                    self.console_logs.push(format!(
                        "Imported model '{}' as entity #{}",
                        model_name, entity
                    ));
                    self.status = format!("Imported: {}", model_name);
                } else {
                    warn!("No scene loaded - cannot import model");
                    self.console_logs
                        .push("No scene loaded – cannot import model.".into());
                }
            }

            AssetAction::ApplyTexture { path, texture_type } => {
                // Convert TextureType to MaterialSlot
                let slot = match texture_type {
                    TextureType::Albedo => MaterialSlot::Albedo,
                    TextureType::Normal => MaterialSlot::Normal,
                    TextureType::ORM => MaterialSlot::ORM,
                    TextureType::MRA => MaterialSlot::ORM, // Map MRA to ORM slot
                    TextureType::Roughness => MaterialSlot::Roughness,
                    TextureType::Metallic => MaterialSlot::Metallic,
                    TextureType::AO => MaterialSlot::AO,
                    TextureType::Emission => MaterialSlot::Emission,
                    TextureType::Height => MaterialSlot::Height,
                    TextureType::Unknown => MaterialSlot::Albedo, // Default to albedo
                };

                if let Some(selected_id) = self.selected_entity {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        if let Some(editor_entity) = scene_state
                            .get_editor_entity_mut(selected_id as astraweave_core::Entity)
                        {
                            editor_entity.set_texture(slot, path.clone());

                            // Sync texture to EntityManager
                            if let Some(em_entity) = self.entity_manager.get_mut(selected_id) {
                                em_entity.set_texture(slot, path.clone());
                            }

                            info!(
                                "Applied {:?} texture '{}' to entity #{}",
                                slot,
                                path.display(),
                                selected_id
                            );
                            self.console_logs.push(format!(
                                "Applied {:?} texture '{}' to entity #{}",
                                slot,
                                path.file_name().unwrap_or_default().to_string_lossy(),
                                selected_id
                            ));
                            self.status = format!(
                                "Applied texture: {}",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            );
                        }
                    }
                } else {
                    warn!("No entity selected - cannot apply texture");
                    self.console_logs
                        .push("Select an entity first to apply textures.".into());
                }
            }

            AssetAction::ApplyMaterial { path } => {
                let material_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("material")
                    .to_string();

                if let Some(selected_id) = self.selected_entity {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        if let Some(editor_entity) = scene_state
                            .get_editor_entity_mut(selected_id as astraweave_core::Entity)
                        {
                            // Create a new material with the name from the path
                            let mut material = entity_manager::EntityMaterial::new();
                            material.name = material_name.clone();
                            editor_entity.set_material(material.clone());

                            // Sync material to EntityManager
                            if let Some(em_entity) = self.entity_manager.get_mut(selected_id) {
                                em_entity.set_material(material);
                            }

                            info!(
                                "Applied material '{}' to entity #{}",
                                material_name, selected_id
                            );
                            self.console_logs.push(format!(
                                "Applied material '{}' to entity #{}",
                                material_name, selected_id
                            ));
                            self.status = format!("Applied material: {}", material_name);
                        }
                    }
                } else {
                    warn!("No entity selected - cannot apply material");
                    self.console_logs
                        .push("Select an entity first to apply materials.".into());
                }
            }

            AssetAction::LoadScene { path } => {
                let scene_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("scene")
                    .to_string();

                self.status = format!("Loading scene {}...", scene_name);
                self.log(format!("Loading scene: {}...", scene_name));

                match scene_serialization::load_scene(&path) {
                    Ok(loaded_world) => {
                        // Clear old scene state and prefab instances to prevent memory leaks
                        self.prefab_manager.clear_instances();
                        self.undo_stack.clear();

                        self.scene_state = Some(EditorSceneState::new(loaded_world));
                        self.current_scene_path = Some(path.clone());
                        self.is_dirty = false;

                        info!("Loaded scene: {}", scene_name);
                        self.toast_success(format!("Loaded scene: {}", scene_name));
                        self.log(format!("Loaded scene: {}", scene_name));
                        self.status = format!("Loaded: {}", scene_name);

                        // Add to recent files
                        self.recent_files.add_file(path);
                    }
                    Err(err) => {
                        error!("Failed to load scene '{}': {}", scene_name, err);
                        self.toast_error(format!("Failed to load scene: {}", err));
                        self.log(format!("Failed to load scene '{}': {}", scene_name, err));
                        self.status = "Error loading scene".into();
                    }
                }
            }

            AssetAction::SpawnPrefab { path } => {
                self.spawn_prefab_from_drag(path, (0, 0));
            }

            AssetAction::OpenExternal { path } => {
                // Use std::process::Command to open files with default application
                #[cfg(target_os = "windows")]
                {
                    if let Err(err) = std::process::Command::new("cmd")
                        .args(["/C", "start", "", &path.display().to_string()])
                        .spawn()
                    {
                        error!("Failed to open external: {}", err);
                        self.console_logs.push(format!("Failed to open: {}", err));
                    } else {
                        info!("Opened external: {}", path.display());
                    }
                }
                #[cfg(target_os = "macos")]
                {
                    if let Err(err) = std::process::Command::new("open").arg(&path).spawn() {
                        error!("Failed to open external: {}", err);
                        self.console_logs.push(format!("Failed to open: {}", err));
                    } else {
                        info!("Opened external: {}", path.display());
                    }
                }
                #[cfg(target_os = "linux")]
                {
                    if let Err(err) = std::process::Command::new("xdg-open").arg(&path).spawn() {
                        error!("Failed to open external: {}", err);
                        self.console_logs.push(format!("Failed to open: {}", err));
                    } else {
                        info!("Opened external: {}", path.display());
                    }
                }
            }

            AssetAction::LoadToViewport { path } => {
                // Load model directly to viewport for preview (no entity created)
                let model_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("preview_model")
                    .to_string();

                if let Some(viewport) = &self.viewport {
                    match viewport.load_gltf_model(&model_name, &path) {
                        Ok(()) => {
                            info!("Loaded '{}' to viewport for preview", model_name);
                            self.console_logs
                                .push(format!("Loaded '{}' to viewport", model_name));
                            self.status = format!("Viewing: {}", model_name);
                        }
                        Err(e) => {
                            warn!("Failed to load '{}' to viewport: {}", model_name, e);
                            self.console_logs
                                .push(format!("Failed to load '{}': {}", model_name, e));
                        }
                    }
                } else {
                    warn!("No viewport available for model preview");
                    self.console_logs
                        .push("Viewport not available for preview".into());
                }
            }

            AssetAction::InspectAsset { path } => {
                // Log for material inspector (future expansion)
                info!("Inspecting asset: {}", path.display());
                self.console_logs
                    .push(format!("Inspecting: {}", path.display()));
                self.status = format!(
                    "Inspecting: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }

            AssetAction::ImportBlendScene { path } => {
                info!("Import blend scene requested: {}", path.display());
                // Open the Blend Import panel and pre-fill the path
                if !self.dock_layout.has_panel(&PanelType::BlendImport) {
                    self.dock_layout.add_panel(PanelType::BlendImport);
                }
                self.dock_tab_viewer
                    .blend_import_panel_mut()
                    .set_blend_path(path.clone());
                self.console_logs
                    .push(format!("Opened blend import: {}", path.display()));
                self.status = format!(
                    "Importing: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }

            AssetAction::UseAsZoneSource { path } => {
                info!("Use as zone source: {}", path.display());
                // Open the Blueprint panel so the user can assign it
                if !self.dock_layout.has_panel(&PanelType::Blueprint) {
                    self.dock_layout.add_panel(PanelType::Blueprint);
                }
                self.console_logs.push(format!(
                    "Zone source set: {} — draw a zone in Blueprint mode",
                    path.display()
                ));
                self.status = format!(
                    "Zone source: {} — draw a zone in Blueprint panel",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
        }
    }

    fn show_behavior_graph_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Behavior Graph Editor");
        let selected_entity = self.selected_entity_handle();

        ui.horizontal(|ui| match (selected_entity, self.scene_state.as_ref()) {
            (Some(entity), Some(state)) => {
                let label = state.world().name(entity).unwrap_or("Unnamed");
                ui.label(format!("Selected entity: {} (#{})", label, entity));
            }
            _ => {
                ui.label("Select an entity to load/apply behavior graphs.");
            }
        });

        ui.horizontal(|ui| {
            let has_selection = selected_entity.is_some() && self.scene_state.is_some();
            if ui
                .add_enabled(has_selection, egui::Button::new("Load From Selection"))
                .clicked()
            {
                self.load_behavior_graph_from_selection();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Apply To Selection"))
                .clicked()
            {
                self.apply_behavior_graph_to_selection();
            }
            if ui
                .add_enabled(
                    self.behavior_graph_binding.is_some(),
                    egui::Button::new("Detach"),
                )
                .clicked()
            {
                self.behavior_graph_binding = None;
                self.console_logs
                    .push("Behavior graph document detached from entity binding.".into());
            }
        });

        if let Some(binding) = &self.behavior_graph_binding {
            ui.label(format!(
                "Document bound to {} (#{}) – changes can be applied directly.",
                binding.name, binding.entity
            ));
        } else {
            ui.label("Document is unbound. Load from an entity or file to bind.");
        }

        ui.separator();
        self.behavior_graph_ui
            .show(ui, &mut self.behavior_graph_doc, |entry| {
                self.console_logs.push(entry);
            });
    }

    fn show_dialogue_graph_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dialogue Graph Editor");
        ui.label("Node-based dialogue editor with live validation and editing");

        ui.horizontal(|ui| {
            if ui.button("Add Node").clicked() {
                let new_id = format!("node_{}", self.dialogue_graph.nodes.len());
                self.dialogue_graph
                    .nodes
                    .push(astraweave_dialogue::DialogueNode {
                        id: new_id,
                        text: "New dialogue text".into(),
                        responses: vec![astraweave_dialogue::DialogueResponse {
                            text: "Response".into(),
                            next_id: None,
                        }],
                    });
            }
            if ui.button("Validate Dialogue").clicked() {
                if let Err(e) = self.dialogue_graph.validate() {
                    self.console_logs
                        .push(format!("Dialogue validation error: {}", e));
                } else {
                    self.console_logs.push("Dialogue validated.".into());
                }
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, node) in self.dialogue_graph.nodes.iter_mut().enumerate() {
                ui.collapsing(format!("Node {}: {}", i, node.id), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        ui.text_edit_singleline(&mut node.id);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Text:");
                        ui.text_edit_multiline(&mut node.text);
                    });
                    ui.label("Responses:");
                    let mut to_remove = vec![];
                    for (j, resp) in node.responses.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", j));
                            ui.text_edit_singleline(&mut resp.text);
                            ui.label("Next ID:");
                            let next_id = resp.next_id.get_or_insert(String::new());
                            ui.text_edit_singleline(next_id);
                            if ui.button("Remove").clicked() {
                                to_remove.push(j);
                            }
                        });
                    }
                    // Remove in reverse order to avoid index invalidation
                    for &idx in to_remove.iter().rev() {
                        node.responses.remove(idx);
                    }
                    if ui.button("Add Response").clicked() {
                        node.responses.push(astraweave_dialogue::DialogueResponse {
                            text: "New response".into(),
                            next_id: None,
                        });
                    }
                });
            }
        });
    }

    fn show_quest_graph_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Quest Graph Editor");
        ui.label("Node-based quest editor");

        ui.horizontal(|ui| {
            if ui.button("Add Step").clicked() {
                self.quest_graph.steps.push(astraweave_quests::QuestStep {
                    description: "New quest step".into(),
                    completed: false,
                });
            }
            if ui.button("Validate Quest").clicked() {
                if let Err(e) = self.quest_graph.validate() {
                    self.console_logs
                        .push(format!("Quest validation error: {}", e));
                } else {
                    self.console_logs.push("Quest validated.".into());
                }
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, step) in self.quest_graph.steps.iter_mut().enumerate() {
                ui.collapsing(format!("Step {}: {}", i, step.description), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_singleline(&mut step.description);
                    });
                    ui.checkbox(&mut step.completed, "Completed");
                });
            }
        });
    }

    fn show_material_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Material Editor");
        ui.label("Live material editing - synced with 3D viewport");

        // Color sliders with live preview
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Base Color:");
        });

        if ui
            .add(egui::Slider::new(&mut self.mat_doc.base_color[0], 0.0..=1.0).text("R"))
            .changed()
        {
            changed = true;
        }
        if ui
            .add(egui::Slider::new(&mut self.mat_doc.base_color[1], 0.0..=1.0).text("G"))
            .changed()
        {
            changed = true;
        }
        if ui
            .add(egui::Slider::new(&mut self.mat_doc.base_color[2], 0.0..=1.0).text("B"))
            .changed()
        {
            changed = true;
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("PBR Properties:");
        });

        if ui
            .add(egui::Slider::new(&mut self.mat_doc.metallic, 0.0..=1.0).text("Metallic"))
            .changed()
        {
            changed = true;
        }
        if ui
            .add(egui::Slider::new(&mut self.mat_doc.roughness, 0.04..=1.0).text("Roughness"))
            .changed()
        {
            changed = true;
        }

        // Apply changes to 3D viewport in real-time
        if changed {
            if let Some(viewport) = &self.viewport {
                let base_color = [
                    self.mat_doc.base_color[0],
                    self.mat_doc.base_color[1],
                    self.mat_doc.base_color[2],
                    1.0, // Alpha
                ];
                if let Err(e) = viewport.set_material_params(
                    base_color,
                    self.mat_doc.metallic,
                    self.mat_doc.roughness,
                ) {
                    tracing::error!("Failed to apply material parameters to viewport: {e}");
                }
            }
        }

        // Color preview swatch
        ui.add_space(8.0);
        let preview_color = egui::Color32::from_rgb(
            (self.mat_doc.base_color[0] * 255.0) as u8,
            (self.mat_doc.base_color[1] * 255.0) as u8,
            (self.mat_doc.base_color[2] * 255.0) as u8,
        );
        ui.horizontal(|ui| {
            ui.label("Preview:");
            let (rect, _response) =
                ui.allocate_exact_size(egui::vec2(40.0, 20.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, preview_color);
            ui.label(format!(
                "M:{:.2} R:{:.2}",
                self.mat_doc.metallic, self.mat_doc.roughness
            ));
        });

        ui.add_space(8.0);

        // Manual apply button (in case auto-sync didn't work)
        ui.horizontal(|ui| {
            if ui
                .button("Apply to Viewport")
                .on_hover_text("Manually apply material to 3D viewport")
                .clicked()
            {
                if let Some(viewport) = &self.viewport {
                    let base_color = [
                        self.mat_doc.base_color[0],
                        self.mat_doc.base_color[1],
                        self.mat_doc.base_color[2],
                        1.0,
                    ];
                    match viewport.set_material_params(
                        base_color,
                        self.mat_doc.metallic,
                        self.mat_doc.roughness,
                    ) {
                        Ok(_) => {
                            self.console_logs
                                .push("Material applied to viewport".into());
                        }
                        Err(e) => {
                            self.console_logs.push(format!("Material error: {}", e));
                        }
                    }
                }
            }

            // Sync status indicator
            if self.viewport.is_some() {
                ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "Viewport synced");
            } else {
                ui.colored_label(egui::Color32::from_rgb(200, 150, 100), "No viewport");
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // Texture path
        let tex_ref = self.mat_doc.texture_path.get_or_insert(String::new());
        ui.horizontal(|ui| {
            ui.label("Texture:");
            ui.text_edit_singleline(tex_ref);
        });

        ui.add_space(8.0);
        if ui.button("Save & Reload Material").clicked() {
            let _ = fs::create_dir_all("assets");
            match serde_json::to_string_pretty(&self.mat_doc) {
                Ok(s) => {
                    let save_path = std::path::Path::new("assets/material_live.json");
                    if fs::write(save_path, s).is_ok() {
                        self.status = "Saved assets/material_live.json".into();
                        self.console_logs
                            .push("Material saved to assets/material_live.json".into());
                        // Trigger hot reload by reloading the material in the inspector
                        // The file watcher will also detect this change automatically
                        self.console_logs.push("Hot reload triggered".into());
                    } else {
                        self.status = "Failed to write material_live.json".into();
                        self.console_logs
                            .push("Failed to write material file".into());
                    }
                }
                Err(e) => {
                    self.status = format!("Serialize error: {e}");
                    self.console_logs
                        .push(format!("Material serialization error: {}", e));
                }
            }
        }
    }

    fn show_terrain_painter(&mut self, ui: &mut egui::Ui) {
        ui.heading("Terrain Painter");
        ui.label("Click cells to paint biomes");

        ui.horizontal(|ui| {
            ui.label("Selected Biome:");
            egui::ComboBox::from_label("")
                .selected_text(&self.selected_biome)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_biome, "grass".into(), "Grass");
                    ui.selectable_value(&mut self.selected_biome, "forest".into(), "Forest");
                    ui.selectable_value(&mut self.selected_biome, "mountain".into(), "Mountain");
                    ui.selectable_value(&mut self.selected_biome, "water".into(), "Water");
                });
        });

        ui.separator();

        egui::Grid::new("terrain_grid").show(ui, |ui| {
            for (y, row) in self.terrain_grid.iter_mut().enumerate() {
                for (x, cell) in row.iter_mut().enumerate() {
                    let color = match cell.as_str() {
                        "grass" => egui::Color32::GREEN,
                        "forest" => egui::Color32::DARK_GREEN,
                        "mountain" => egui::Color32::GRAY,
                        "water" => egui::Color32::BLUE,
                        _ => egui::Color32::WHITE,
                    };
                    let response = ui.add(
                        egui::Button::new("")
                            .fill(color)
                            .min_size(egui::Vec2::new(20.0, 20.0)),
                    );
                    if response.clicked() {
                        *cell = self.selected_biome.clone();
                    }
                    ui.label(format!("({}, {})", x, y));
                }
                ui.end_row();
            }
        });

        if ui.button("Save Terrain").clicked() {
            let _ = fs::create_dir_all("assets");
            match serde_json::to_string_pretty(&self.terrain_grid) {
                Ok(s) => {
                    if fs::write("assets/terrain_grid.json", s).is_ok() {
                        self.status = "Saved terrain grid".into();
                        self.console_logs
                            .push("Terrain grid saved to assets/terrain_grid.json".into());
                    } else {
                        self.status = "Failed to save terrain grid".into();
                        self.console_logs
                            .push("Failed to write terrain grid file".into());
                    }
                }
                Err(e) => {
                    self.status = format!("Serialize terrain error: {}", e);
                    self.console_logs
                        .push(format!("Terrain serialization error: {}", e));
                }
            }
        }

        if ui.button("Load Terrain").clicked() {
            match fs::read_to_string("assets/terrain_grid.json") {
                Ok(s) => match serde_json::from_str::<Vec<Vec<String>>>(&s) {
                    Ok(grid) => {
                        if grid.len() == 10 && grid.iter().all(|r| r.len() == 10) {
                            self.terrain_grid = grid;
                            self.status = "Loaded terrain grid".into();
                            self.console_logs
                                .push("Terrain grid loaded from assets/terrain_grid.json".into());
                        } else {
                            self.status = "Invalid terrain grid format".into();
                            self.console_logs
                                .push("Invalid terrain grid format (must be 10x10)".into());
                        }
                    }
                    Err(e) => {
                        self.status = format!("Deserialize terrain error: {}", e);
                        self.console_logs
                            .push(format!("Failed to parse terrain file: {}", e));
                    }
                },
                Err(e) => {
                    self.status = format!("Read terrain error: {}", e);
                    self.console_logs
                        .push(format!("Failed to read terrain file: {}", e));
                }
            }
        }

        if ui.button("Sync with Level").clicked() {
            // Convert grid to biome_paints
            self.level.biome_paints.clear();
            for (y, row) in self.terrain_grid.iter().enumerate() {
                for (x, cell) in row.iter().enumerate() {
                    if *cell == "grass" {
                        self.level.biome_paints.push(BiomePaint::GrassDense {
                            area: Circle {
                                cx: x as i32 * 10,
                                cz: y as i32 * 10,
                                radius: 5,
                            },
                        });
                    }
                    // Add others if needed
                }
            }
            self.status = "Synced terrain with level".into();
        }
    }

    fn show_voxel_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Voxel Editor");
        ui.label("Interactive terrain sculpting tools");

        let mut brush = *self.voxel_editor.brush();

        ui.group(|ui| {
            ui.label("Brush Settings");
            egui::ComboBox::from_label("Shape")
                .selected_text(format!("{:?}", brush.shape))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut brush.shape,
                        voxel_tools::BrushShape::Sphere,
                        "Sphere",
                    );
                    ui.selectable_value(&mut brush.shape, voxel_tools::BrushShape::Cube, "Cube");
                    ui.selectable_value(
                        &mut brush.shape,
                        voxel_tools::BrushShape::Cylinder,
                        "Cylinder",
                    );
                });

            egui::ComboBox::from_label("Mode")
                .selected_text(format!("{:?}", brush.mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut brush.mode, voxel_tools::BrushMode::Add, "Add");
                    ui.selectable_value(&mut brush.mode, voxel_tools::BrushMode::Remove, "Remove");
                    ui.selectable_value(&mut brush.mode, voxel_tools::BrushMode::Paint, "Paint");
                });

            ui.add(egui::Slider::new(&mut brush.radius, 0.1..=20.0).text("Radius"));
            ui.add(egui::Slider::new(&mut brush.strength, 0.0..=1.0).text("Strength"));
            ui.checkbox(&mut brush.smooth, "Smooth Edges");
        });

        self.voxel_editor.set_brush(brush);

        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.voxel_editor.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.log("Voxel undo requested (integration pending)");
            }
            if ui
                .add_enabled(self.voxel_editor.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.log("Voxel redo requested (integration pending)");
            }
        });

        if ui.button("Clear History").clicked() {
            self.voxel_editor.clear_history();
        }
    }

    fn show_navmesh_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Navmesh Controls");
        ui.label("Baking and visualization controls");

        ui.horizontal(|ui| {
            ui.label("Max Step:");
            ui.add(egui::DragValue::new(&mut self.nav_max_step).speed(0.1));
            ui.label("Max Slope Deg:");
            ui.add(egui::DragValue::new(&mut self.nav_max_slope_deg).speed(1.0));
        });

        if ui.button("Bake Navmesh").clicked() {
            // Generate triangles from level obstacles
            let mut tris = vec![];
            for obs in &self.level.obstacles {
                // Assume obstacle is a 1x1 square on XZ plane at Y=0
                let x = obs.pos[0] as i32;
                let z = obs.pos[2] as i32;
                // Generate two triangles for the square
                tris.push(astraweave_nav::Triangle {
                    a: glam::Vec3::new(x as f32, 0.0, z as f32),
                    b: glam::Vec3::new(x as f32 + 1.0, 0.0, z as f32),
                    c: glam::Vec3::new(x as f32, 0.0, z as f32 + 1.0),
                });
                tris.push(astraweave_nav::Triangle {
                    a: glam::Vec3::new(x as f32 + 1.0, 0.0, z as f32 + 1.0),
                    b: glam::Vec3::new(x as f32, 0.0, z as f32 + 1.0),
                    c: glam::Vec3::new(x as f32 + 1.0, 0.0, z as f32),
                });
            }
            if tris.is_empty() {
                // Fallback to dummy
                for x in 0..9 {
                    for z in 0..9 {
                        tris.push(astraweave_nav::Triangle {
                            a: glam::Vec3::new(x as f32, 0.0, z as f32),
                            b: glam::Vec3::new(x as f32 + 1.0, 0.0, z as f32),
                            c: glam::Vec3::new(x as f32, 0.0, z as f32 + 1.0),
                        });
                        tris.push(astraweave_nav::Triangle {
                            a: glam::Vec3::new(x as f32 + 1.0, 0.0, z as f32 + 1.0),
                            b: glam::Vec3::new(x as f32, 0.0, z as f32 + 1.0),
                            c: glam::Vec3::new(x as f32 + 1.0, 0.0, z as f32),
                        });
                    }
                }
            }
            self.nav_mesh =
                astraweave_nav::NavMesh::bake(&tris, self.nav_max_step, self.nav_max_slope_deg);
            let tri_count = self.nav_mesh.tris.len();
            self.console_logs.push(format!(
                "Navmesh baked: {} triangles, max_step={}, max_slope={}°",
                tri_count, self.nav_max_step, self.nav_max_slope_deg
            ));
            self.status = format!("Navmesh baked ({} triangles)", tri_count);
        }

        ui.label(format!("Triangles: {}", self.nav_mesh.tris.len()));
    }

    fn show_asset_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Asset Inspector");
        ui.label(format!("Total assets: {}", self.asset_db.assets.len()));
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (guid, meta) in &self.asset_db.assets {
                ui.collapsing(format!("{} ({})", meta.path, guid), |ui| {
                    ui.label(format!("Kind: {:?}", meta.kind));
                    ui.label(format!("Size: {} bytes", meta.size_bytes));
                    ui.label(format!("Hash: {}", &meta.hash[..16]));
                    ui.label(format!("Modified: {}", meta.last_modified));
                    if !meta.dependencies.is_empty() {
                        ui.label("Dependencies:");
                        for dep in &meta.dependencies {
                            ui.label(format!("  {}", dep));
                        }
                    }
                });
            }
        });
        if ui.button("Reload Assets").clicked() {
            self.asset_db = AssetDatabase::new();
            if let Ok(()) = self
                .asset_db
                .load_manifest(&PathBuf::from("assets/assets.json"))
            {
                self.status = "Reloaded assets from manifest".into();
                self.console_logs.push(format!(
                    "Assets reloaded from manifest: {} total",
                    self.asset_db.assets.len()
                ));
            } else {
                if let Err(e) = self.asset_db.scan_directory(&PathBuf::from("assets")) {
                    tracing::error!("Asset directory scan failed: {e}");
                }
                // Cache the manifest so subsequent startups are instant
                if let Err(e) = self
                    .asset_db
                    .save_manifest(&PathBuf::from("assets/assets.json"))
                {
                    tracing::error!("Asset manifest save failed: {e}");
                }
                self.status = "Rescanned assets directory (manifest cached)".into();
                self.console_logs.push(format!(
                    "Assets rescanned from directory: {} total (manifest saved)",
                    self.asset_db.assets.len()
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MaterialLiveDoc {
    base_color: [f32; 4],
    metallic: f32,
    roughness: f32,
    texture_path: Option<String>,
}

impl MenuActionHandler for EditorApp {
    fn on_new(&mut self) {
        if self.is_dirty {
            self.show_new_confirm_dialog = true;
        } else {
            self.create_new_scene();
        }
    }

    fn on_new_world_wizard(&mut self) {
        self.world_wizard.open();
    }

    fn on_show_tutorial(&mut self) {
        self.tutorial.start();
    }

    fn on_show_about(&mut self) {
        self.show_about_dialog = true;
    }

    fn on_import_blend_scene(&mut self) {
        // Ensure the Blend Import panel is open
        if !self.dock_layout.has_panel(&PanelType::BlendImport) {
            self.dock_layout.add_panel(PanelType::BlendImport);
        }
        // Trigger the native file browse dialog
        self.dock_tab_viewer
            .blend_import_panel_mut()
            .trigger_file_browse();
        self.status = "Import .blend scene — select a file".into();
    }

    fn on_toggle_blueprint_mode(&mut self) {
        if self.dock_layout.has_panel(&PanelType::Blueprint) {
            self.dock_layout.remove_panel(&PanelType::Blueprint);
            self.status = "Blueprint mode off".into();
        } else {
            self.dock_layout.add_panel(PanelType::Blueprint);
            self.status =
                "Blueprint mode — click canvas to place zone vertices, then Generate".into();
        }
    }

    fn is_blueprint_mode(&self) -> bool {
        self.dock_layout.has_panel(&PanelType::Blueprint)
    }

    fn on_open(&mut self) {
        // simple hardcoded example; integrate rfd/native dialog if desired
        let p = self.content_root.join("levels/forest_breach.level.toml");
        if let Ok(s) = fs::read_to_string(&p) {
            match toml::from_str::<LevelDoc>(&s) {
                Ok(ld) => {
                    self.level = ld;
                    self.status = format!("Opened {:?}", p);
                    self.console_logs.push(format!("Opened level: {:?}", p));
                }
                Err(e) => {
                    self.status = format!("Open failed: {e}");
                    self.console_logs
                        .push(format!("Failed to open level: {}", e));
                }
            }
        } else {
            self.console_logs.push(format!("File not found: {:?}", p));
            self.status = "File not found".into();
        }
    }

    fn on_save(&mut self) {
        let dir = self.content_root.join("levels");
        let _ = fs::create_dir_all(&dir);
        let p = dir.join(format!(
            "{}.level.toml",
            self.level.title.replace(' ', "_").to_lowercase()
        ));
        match toml::to_string_pretty(&self.level) {
            Ok(txt) => {
                if let Err(e) = fs::write(&p, txt) {
                    self.status = format!("Save failed: {e}");
                    self.console_logs.push(format!("Failed to save: {}", e));
                } else {
                    // Signal hot-reload to the runtime
                    let _ = fs::create_dir_all(&self.content_root);
                    let _ = fs::write(
                        self.content_root.join("reload.signal"),
                        Uuid::new_v4().to_string(),
                    );
                    self.status = format!("Saved {:?}", p);
                    self.console_logs.push(format!("Saved level: {:?}", p));
                }
            }
            Err(e) => {
                self.status = format!("Serialize failed: {e}");
                self.console_logs
                    .push(format!("Serialization failed: {}", e));
            }
        }
    }

    fn on_save_json(&mut self) {
        let dir = self.content_root.join("levels");
        let _ = fs::create_dir_all(&dir);
        let p = dir.join(format!(
            "{}.level.json",
            self.level.title.replace(' ', "_").to_lowercase()
        ));
        match serde_json::to_string_pretty(&self.level) {
            Ok(txt) => {
                if let Err(e) = fs::write(&p, txt) {
                    self.status = format!("Save JSON failed: {e}");
                    self.console_logs
                        .push(format!("Failed to save JSON: {}", e));
                } else {
                    self.status = format!("Saved JSON {:?}", p);
                    self.console_logs.push(format!("Saved JSON: {:?}", p));
                }
            }
            Err(e) => {
                self.status = format!("Serialize JSON failed: {e}");
                self.console_logs
                    .push(format!("JSON serialization failed: {}", e));
            }
        }
    }

    fn on_save_scene(&mut self) {
        if let Some(world) = self.edit_world() {
            let path = if let Some(p) = &self.current_scene_path {
                p.clone()
            } else {
                let dir = self.content_root.join("scenes");
                let _ = fs::create_dir_all(&dir);
                dir.join("untitled.scene.ron")
            };

            match scene_serialization::save_scene(world, &path) {
                Ok(()) => {
                    self.current_scene_path = Some(path.clone());
                    self.recent_files.add_file(path.clone());
                    self.is_dirty = false;
                    self.status = format!("Saved scene to {:?}", path);
                    self.toast_success(format!(
                        "Saved scene: {:?}",
                        path.file_name().unwrap_or_default()
                    ));
                    self.console_logs.push(format!("Scene saved: {:?}", path));
                    self.last_auto_save = std::time::Instant::now();
                }
                Err(e) => {
                    self.status = format!("Scene save failed: {}", e);
                    self.console_logs
                        .push(format!("Failed to save scene: {}", e));
                }
            }
        } else {
            self.console_logs.push("No world to save".into());
        }
    }

    fn on_load_scene(&mut self) {
        let path = self.content_root.join("scenes/untitled.scene.ron");
        self.request_open_scene(path);
    }

    fn on_exit(&mut self) {
        if self.is_dirty {
            self.show_quit_dialog = true;
        } else {
            self.pending_quit = true;
            self.remove_lock_file();
        }
    }

    fn on_undo(&mut self) {
        if let Some(scene_state) = self.scene_state.as_mut() {
            let world = scene_state.world_mut();
            if self.undo_stack.can_undo() {
                if let Err(e) = self.undo_stack.undo(world, Some(&mut self.entity_manager)) {
                    self.status = format!("Undo failed: {}", e);
                } else {
                    self.status = "Undo".to_string();
                }
            }
        }
    }

    fn on_redo(&mut self) {
        if let Some(scene_state) = self.scene_state.as_mut() {
            let world = scene_state.world_mut();
            if self.undo_stack.can_redo() {
                if let Err(e) = self.undo_stack.redo(world, Some(&mut self.entity_manager)) {
                    self.status = format!("Redo failed: {}", e);
                } else {
                    self.status = "Redo".to_string();
                }
            }
        }
    }

    fn on_delete(&mut self) {
        if let Some(scene_state) = self.scene_state.as_mut() {
            let world = scene_state.world_mut();
            if let Some(viewport) = &mut self.viewport {
                viewport.delete_selection(world, &mut self.undo_stack);
                self.status = "Deleted selection".to_string();
            }
        }
    }

    fn selection_count(&self) -> usize {
        self.selection_set.count()
    }

    fn on_apply_material(&mut self) {
        self.apply_material_to_selection();
    }

    fn on_group_selection(&mut self) {
        self.group_selection();
    }

    fn on_ungroup_selection(&mut self) {
        self.ungroup_selection();
    }

    fn on_align_selection(&mut self, dir: AlignDirection) {
        self.align_selection(dir);
    }

    // Recent Files
    fn get_recent_files(&self) -> Vec<PathBuf> {
        self.recent_files.get_files().to_vec()
    }

    fn on_open_recent(&mut self, path: PathBuf) {
        if self.is_dirty {
            self.pending_open_path = Some(path);
            self.show_open_confirm_dialog = true;
        } else {
            self.load_scene_from_path(&path);
        }
    }

    fn on_clear_recent(&mut self) {
        self.recent_files.clear();
    }

    // View
    fn is_view_hierarchy_open(&self) -> bool {
        self.dock_layout.has_panel(&PanelType::Hierarchy)
    }
    fn toggle_view_hierarchy(&mut self) {
        self.show_hierarchy_panel = !self.show_hierarchy_panel;
        self.dock_layout.toggle_panel(PanelType::Hierarchy);
        self.status = format!(
            "Hierarchy panel {}",
            if self.show_hierarchy_panel {
                "shown"
            } else {
                "hidden"
            }
        );
    }

    fn is_view_inspector_open(&self) -> bool {
        self.dock_layout.has_panel(&PanelType::Inspector)
    }
    fn toggle_view_inspector(&mut self) {
        self.show_inspector_panel = !self.show_inspector_panel;
        self.dock_layout.toggle_panel(PanelType::Inspector);
        self.status = format!(
            "Inspector panel {}",
            if self.show_inspector_panel {
                "shown"
            } else {
                "hidden"
            }
        );
    }

    fn is_view_console_open(&self) -> bool {
        self.dock_layout.has_panel(&PanelType::Console)
    }
    fn toggle_view_console(&mut self) {
        self.show_console_panel = !self.show_console_panel;
        self.dock_layout.toggle_panel(PanelType::Console);
        self.status = format!(
            "Console panel {}",
            if self.show_console_panel {
                "shown"
            } else {
                "hidden"
            }
        );
    }

    fn is_grid_visible(&self) -> bool {
        self.show_grid
    }
    fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
        self.status = format!(
            "Grid {}",
            if self.show_grid {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    fn viewport_layout(&self) -> crate::viewport::ViewportLayout {
        self.viewport_layout
    }

    fn set_viewport_layout(&mut self, layout: crate::viewport::ViewportLayout) {
        let needed = layout.viewport_count().saturating_sub(1); // extra viewports beyond primary
                                                                // Create any missing extra viewports
        while self.extra_viewports.len() < needed {
            if let Some(primary) = &self.viewport {
                match ViewportWidget::new_additional(primary) {
                    Ok(vp) => self.extra_viewports.push(vp),
                    Err(e) => {
                        self.log(&format!("Failed to create extra viewport: {}", e));
                        return;
                    }
                }
            } else {
                self.log("Cannot create extra viewport: primary viewport not initialised");
                return;
            }
        }

        self.viewport_layout = layout;
        self.status = format!("Viewport layout: {}", layout.label());
    }

    // Window
    fn is_docking_enabled(&self) -> bool {
        self.use_docking
    }
    fn toggle_docking(&mut self) {
        self.use_docking = !self.use_docking;
        self.status = format!(
            "Switched to {} layout mode",
            if self.use_docking {
                "Docking"
            } else {
                "Legacy"
            }
        );
    }

    fn on_apply_layout_preset(&mut self, preset_name: &str) {
        let preset = match preset_name {
            "Default" => LayoutPreset::Default,
            "Wide" => LayoutPreset::Wide,
            "Compact" => LayoutPreset::Compact,
            "Modeling" => LayoutPreset::Modeling,
            "Animation" => LayoutPreset::Animation,
            "Debug" => LayoutPreset::Debug,
            _ => LayoutPreset::Default,
        };
        self.dock_layout = DockLayout::from_preset(preset);
        self.status = format!("Applied {} layout", preset_name);
    }

    fn is_dock_panel_visible(&self, panel: PanelType) -> bool {
        self.dock_layout.has_panel(&panel)
    }

    fn toggle_dock_panel(&mut self, panel: PanelType) {
        if self.dock_layout.has_panel(&panel) {
            self.dock_layout.remove_panel(&panel);
            self.status = format!("Closed {} panel", panel.title());
        } else {
            self.dock_layout.add_panel(panel);
            self.status = format!("Opened {} panel", panel.title());
        }
    }

    // Settings
    fn on_open_settings(&mut self) {
        self.show_settings_dialog = true;
    }

    // Debug
    fn on_scan_for_models(&mut self) {
        let scan_dirs = [
            ("Local", PathBuf::from("assets/models")),
            ("Pine Forest", PathBuf::from("../pine_forest")),
            ("Downloads PF", PathBuf::from("../../Downloads/pine_forest")),
        ];
        let mut found_any = false;
        for (name, dir) in &scan_dirs {
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    let glb_files: Vec<_> = entries
                        .filter_map(|e| {
                            e.map_err(|err| tracing::debug!("Autosave dir entry error: {}", err))
                                .ok()
                        })
                        .filter(|e| {
                            matches!(
                                e.path().extension().and_then(|ext| ext.to_str()),
                                Some("glb" | "gltf")
                            )
                        })
                        .take(8)
                        .collect();
                    if !glb_files.is_empty() {
                        found_any = true;
                        self.console_logs
                            .push(format!("{} ({}):", name, glb_files.len()));
                        for entry in glb_files {
                            self.console_logs
                                .push(format!("  • {}", entry.file_name().to_string_lossy()));
                        }
                    }
                }
            }
        }
        if !found_any {
            self.console_logs
                .push("No glTF/glb models found in any scanned directory".into());
        }
    }

    fn on_load_test_model(&mut self, name: &str, path: PathBuf) {
        let target_path = if path.to_string_lossy() == "PINE_TREE_AUTO" {
            let possible_paths = [
                PathBuf::from("assets/models/pine_tree_01_1k.glb"),
                PathBuf::from("../pine_forest/pine_tree_01_1k.glb"),
                PathBuf::from("../../Downloads/pine_forest/pine_tree_01_1k.glb"),
            ];
            possible_paths.iter().find(|p| p.exists()).cloned()
        } else if path.exists() {
            Some(path.clone())
        } else {
            None
        };

        if let Some(target) = target_path {
            if let Some(viewport) = &self.viewport {
                match viewport.load_gltf_model(name, &target) {
                    Ok(()) => {
                        self.toast_success(format!("Model {} loaded!", name));
                        self.console_logs
                            .push(format!("Loaded model: {:?}", target));
                    }
                    Err(e) => {
                        self.console_logs.push(format!("Model load failed: {}", e));
                    }
                }
            }
        } else {
            self.console_logs
                .push(format!("Model not found: {:?}", path));
        }
    }

    fn on_toggle_engine_rendering(&mut self) {
        if let Some(viewport) = &self.viewport {
            if let Ok(mut renderer) = viewport.renderer().lock() {
                let current = renderer.use_engine_rendering();
                renderer.set_use_engine_rendering(!current);
                let state = if !current { "enabled" } else { "disabled" };
                self.console_logs
                    .push(format!("Engine rendering {}", state));
                self.status = format!("Engine rendering {}", state);
            }
        }
    }

    fn on_show_engine_info(&mut self) {
        if let Some(viewport) = &self.viewport {
            if let Ok(renderer) = viewport.renderer().lock() {
                let engine_active = renderer.use_engine_rendering();
                let adapter_init = renderer.engine_adapter_initialized();
                self.console_logs.push(format!(
                    "Engine Status:\n  - Engine Rendering: {}\n  - Adapter Initialized: {}",
                    engine_active, adapter_init
                ));
            }
        }
    }

    fn on_debug_material(&mut self, name: &str) {
        if let Some(viewport) = &self.viewport {
            let res = match name {
                "Red" => viewport.set_material_params([1.0, 0.2, 0.2, 1.0], 0.0, 0.5),
                "Green" => viewport.set_material_params([0.2, 0.8, 0.2, 1.0], 0.9, 0.3),
                "Blue" => viewport.set_material_params([0.2, 0.3, 0.9, 1.0], 0.1, 0.9),
                "White" => viewport.set_material_params([1.0, 1.0, 1.0, 1.0], 0.0, 0.5),
                _ => Ok(()),
            };
            if let Err(e) = res {
                self.console_logs.push(format!("Material error: {}", e));
            } else {
                self.console_logs.push(format!("{} material applied", name));
            }
        }
    }

    fn on_debug_time_set(&mut self, time: f32) {
        if let Some(viewport) = &self.viewport {
            if let Err(e) = viewport.set_time_of_day(time) {
                self.console_logs.push(format!("Lighting error: {}", e));
            } else {
                self.console_logs.push(format!("Time set to {}", time));
            }
        }
    }

    fn get_time_of_day(&self) -> f32 {
        self.viewport
            .as_ref()
            .and_then(|v| v.get_time_of_day().ok())
            .unwrap_or(0.0)
    }

    fn get_time_period(&self) -> String {
        self.viewport
            .as_ref()
            .and_then(|v| v.get_time_period().ok())
            .unwrap_or("Unknown")
            .to_string()
    }

    fn is_shadows_enabled(&self) -> bool {
        self.viewport
            .as_ref()
            .and_then(|v| v.shadows_enabled().ok())
            .unwrap_or(false)
    }

    fn set_shadows_enabled(&mut self, enabled: bool) {
        if let Some(viewport) = &self.viewport {
            if let Err(e) = viewport.set_shadows_enabled(enabled) {
                self.console_logs.push(format!("Shadow error: {}", e));
            } else {
                self.console_logs.push(format!(
                    "Shadows {}",
                    if enabled { "enabled" } else { "disabled" }
                ));
            }
        }
    }

    fn on_diff_assets(&mut self) {
        match std::process::Command::new("git")
            .args(["diff", "assets"])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stdout.is_empty() && stderr.is_empty() {
                    self.console_logs.push("No asset changes.".into());
                } else {
                    self.console_logs.push(format!("Asset diff:\n{}", stdout));
                    if !stderr.is_empty() {
                        self.console_logs.push(format!("Diff stderr: {}", stderr));
                    }
                }
            }
            Err(e) => self.console_logs.push(format!("Git diff failed: {}", e)),
        }
    }

    fn on_clear_console(&mut self) {
        self.console_logs.clear();
    }

    fn on_distribute_selection(&mut self, dir: DistributeDirection) {
        self.distribute_selection(dir);
    }

    fn on_select_all(&mut self) {
        self.select_all_entities();
    }

    fn on_deselect_all(&mut self) {
        self.selection_set.clear();
        self.selected_entity = None;
        self.status = "Deselected all".to_string();
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Tighten spacing for a compact professional look without crushing readability
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(4.0, 3.0);
        style.spacing.window_margin = egui::Margin::same(2);
        ctx.set_style(style);

        // Apply persisted theme on first frame
        self.theme_manager.apply_theme(ctx);

        // --- Startup splash screen (logo + cinematic video) ---
        if let Some(splash) = &mut self.splash {
            if splash.show(ctx) {
                return; // Splash still active
            }
            self.splash = None; // Splash finished, proceed to editor
        }

        // If pending_quit was set (e.g. from File > Exit with clean state), close immediately
        if self.pending_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Week 7 Day 5: Handle close requests with proper lock file cleanup
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.pending_quit {
                // User confirmed quit from dialog — allow the close to proceed
                self.remove_lock_file();
            } else if self.is_dirty {
                self.show_quit_dialog = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else {
                // Clean exit - remove lock file
                self.remove_lock_file();
                // Let the close proceed
            }
        }

        // Week 4: Handle drag-drop file imports
        self.handle_dropped_files(ctx);

        // Week 5 Day 3-4: Process hierarchy panel actions and sync prefab instances
        self.process_hierarchy_actions();
        self.sync_hierarchy_prefab_instances();

        // Drain entity panel mesh assignments (from archetype spawns)
        {
            let assignments = std::mem::take(&mut self.entity_panel.pending_mesh_assignments);
            for (entity_id, mesh_path) in assignments {
                let em_id: u64 = entity_id.into();
                if let Some(em_entity) = self.entity_manager.get_mut(em_id) {
                    em_entity.set_mesh(mesh_path);
                } else {
                    let name = self
                        .scene_state
                        .as_ref()
                        .and_then(|s| s.world().name(entity_id))
                        .unwrap_or("Entity")
                        .to_string();
                    let mut em_entity = entity_manager::EditorEntity::new(em_id, name);
                    em_entity.set_mesh(mesh_path);
                    if let Some(s) = self.scene_state.as_ref() {
                        if let Some(pose) = s.world().pose(entity_id) {
                            em_entity.position =
                                glam::Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32);
                        }
                    }
                    self.entity_manager.add(em_entity);
                }
            }
        }

        // Update cursor ground position for position-aware asset dropping
        if let Some(viewport) = &self.viewport {
            let hover_pos = ctx.input(|i| i.pointer.hover_pos());
            if let Some(pos) = hover_pos {
                if let Some((gx, gz)) = viewport.ground_position_at_screen_pos(pos) {
                    self.last_cursor_ground_pos = Some((gx.round() as i32, gz.round() as i32));
                }
            }
        }

        // Process asset browser actions (drag-drop, double-click, context actions)
        self.process_asset_browser_actions();

        // Process blend import panel actions (decomposition, pack gen, browse)
        self.process_blend_import_actions();

        // Poll background blend decomposition
        self.poll_blend_decomposition();

        // Process blueprint zone panel actions (generate, save, load)
        self.process_blueprint_actions();

        let now = std::time::Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.current_fps = if frame_time > 0.0 {
            1.0 / frame_time
        } else {
            60.0
        };

        let ui_setup_start = std::time::Instant::now();

        self.profiler_panel.push_frame_time(frame_time * 1000.0);

        // Hot-reload: lazily create file watcher (deferred from startup for fast window render)
        if self.file_watcher.is_none() && self.splash.is_none() {
            self.file_watcher = file_watcher::FileWatcher::new("assets").ok();
        }

        // Hot-reload: poll file watcher for asset changes
        if let Some(watcher) = &self.file_watcher {
            for event in watcher.drain_events() {
                let path_display = event.path().display().to_string();
                match &event {
                    file_watcher::ReloadEvent::Material(_) => {
                        self.console_logs
                            .push(format!("Hot-reload: Material changed: {}", path_display));
                    }
                    file_watcher::ReloadEvent::Texture(_) => {
                        self.console_logs
                            .push(format!("Hot-reload: Texture changed: {}", path_display));
                    }
                    file_watcher::ReloadEvent::Prefab(path) => {
                        // Auto-update all instances of this prefab
                        let root_entities: Vec<_> = self
                            .prefab_manager
                            .find_instances_by_source(path)
                            .iter()
                            .map(|inst| inst.root_entity)
                            .collect();
                        let count = root_entities.len();
                        if count > 0 {
                            if let Some(scene_state) = &mut self.scene_state {
                                for entity in &root_entities {
                                    if let Err(e) = self
                                        .prefab_manager
                                        .revert_instance_to_prefab(*entity, scene_state.world_mut())
                                    {
                                        tracing::error!(
                                            "Prefab hot-reload revert failed for entity {:?}: {e}",
                                            entity
                                        );
                                    }
                                }
                            }
                            self.console_logs.push(format!(
                                "Hot-reload: Updated {} prefab instance(s): {}",
                                count, path_display
                            ));
                            self.toast_manager.info(format!(
                                "Updated {} instances of {}",
                                count,
                                std::path::Path::new(&path_display)
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                            ));
                        } else {
                            self.console_logs.push(format!(
                                "Hot-reload: Prefab changed (no instances): {}",
                                path_display
                            ));
                        }
                    }
                    file_watcher::ReloadEvent::Model(_) => {
                        self.console_logs
                            .push(format!("Hot-reload: Model changed: {}", path_display));
                    }
                }
            }
        }

        let selected_count = self.selection_set.entities.len();
        let scene_entity_count = self
            .scene_state
            .as_ref()
            .map(|s| s.world().entities().len())
            .unwrap_or(0);

        // Enhanced scene statistics: pull real data from terrain + scatter renderers
        let (real_triangles, real_vertices, real_draw_calls, scatter_instances) =
            if let Some(viewport) = &self.viewport {
                if let Ok(renderer) = viewport.renderer().lock() {
                    let terrain_tris = renderer.terrain_triangles();
                    let terrain_indices = renderer.terrain_indices();
                    let scatter_tris = renderer.scatter_triangles();
                    let scatter_verts = renderer.scatter_vertices();
                    let scatter_dc = renderer.scatter_draw_calls() as usize;
                    let scatter_inst = renderer.scatter_instance_count() as usize;
                    (
                        terrain_tris + scatter_tris + scene_entity_count * 12,
                        terrain_indices + scatter_verts + scene_entity_count * 8,
                        scatter_dc + scene_entity_count + 2, // +2 for terrain+grid
                        scatter_inst,
                    )
                } else {
                    let est_tris = scene_entity_count * 500;
                    let est_verts = scene_entity_count * 300;
                    (est_tris, est_verts, scene_entity_count, 0)
                }
            } else {
                let est_tris = scene_entity_count * 500;
                let est_verts = scene_entity_count * 300;
                (est_tris, est_verts, scene_entity_count, 0)
            };

        let mesh_count = scene_entity_count + scatter_instances;
        let mesh_memory_kb = (real_vertices * 32 + real_triangles * 12) / 1024;

        // Texture estimates (could be replaced with actual tracking later)
        let texture_count = (scene_entity_count / 5).max(1) + 10; // +10 for terrain biome textures
        let avg_texture_size_kb = 256; // 512x512 RGBA compressed
        let texture_memory_kb = texture_count * avg_texture_size_kb;

        // Material and draw call estimates
        let material_count = (scene_entity_count / 3).max(1) + 2; // +2 for terrain+scatter shaders
        let unique_shader_count = 4; // PBR, unlit, terrain, scatter
        let estimated_state_changes = material_count + unique_shader_count;

        self.scene_stats_panel.update_stats(SceneStats {
            entity_count: scene_entity_count,
            selected_count,
            component_count: scene_entity_count * 3,
            prefab_count: self.prefab_manager.instance_count(),
            undo_stack_size: self.undo_stack.undo_count(),
            redo_stack_size: self.undo_stack.redo_count(),
            memory_estimate_kb: scene_entity_count * 2 + mesh_memory_kb + texture_memory_kb,
            scene_path: self
                .current_scene_path
                .as_ref()
                .map(|p| p.display().to_string()),
            is_dirty: self.is_dirty,
            mesh_count,
            total_triangles: real_triangles,
            total_vertices: real_vertices,
            mesh_memory_kb,
            texture_count,
            texture_memory_kb,
            max_texture_resolution: (2048, 2048), // Terrain biome textures
            material_count,
            unique_shader_count,
            estimated_draw_calls: real_draw_calls,
            estimated_state_changes,
            performance_warning: None, // Calculated by panel
        });

        // Phase 7: Dynamic window title with file name and dirty state
        let file_name = self
            .current_scene_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");
        let dirty_marker = if self.is_dirty { "*" } else { "" };
        let entity_count = self.entity_manager.count();
        let title = format!(
            "AstraWeave Editor - {}{} ({} entities)",
            file_name, dirty_marker, entity_count
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        // Week 7: Enhanced Auto-save with timestamped backups
        if self.auto_save_enabled
            && self.is_dirty
            && self.last_auto_save.elapsed().as_secs_f32() > self.auto_save_interval_secs
        {
            self.perform_auto_save();
        }

        // Render modal dialogs
        self.show_dialogs(ctx);

        // Check for close request
        ctx.input(|i| {
            if i.viewport().close_requested()
                && self.is_dirty
                && !self.show_quit_dialog
                && !self.pending_quit
            {
                self.show_quit_dialog = true;
            }
        });

        // Phase 2.1 & 2.2: Global hotkeys for undo/redo and scene save/load
        ctx.input(|i| {
            // Ctrl+Z: Undo
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let undo_error = self
                        .undo_stack
                        .undo(scene_state.world_mut(), Some(&mut self.entity_manager))
                        .err();

                    if let Some(e) = undo_error {
                        self.console_logs.push(format!("Undo failed: {}", e));
                    } else if let Some(desc) = self.undo_stack.redo_description() {
                        self.status = format!("Undid: {}", desc);
                        self.console_logs.push(format!("Undo: {}", desc));
                        self.is_dirty = true;
                    }
                }
            }

            // Ctrl+Y or Ctrl+Shift+Z: Redo
            if (i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
                || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let redo_error = self
                        .undo_stack
                        .redo(scene_state.world_mut(), Some(&mut self.entity_manager))
                        .err();

                    if let Some(e) = redo_error {
                        self.console_logs.push(format!("Redo failed: {}", e));
                    } else if let Some(desc) = self.undo_stack.undo_description() {
                        self.status = format!("Redid: {}", desc);
                        self.console_logs.push(format!("Redo: {}", desc));
                        self.is_dirty = true;
                    }
                }
            }

            // Ctrl+S: Save Scene
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                if let Some(world) = self.edit_world() {
                    let path = if let Some(p) = &self.current_scene_path {
                        p.clone()
                    } else {
                        let dir = self.content_root.join("scenes");
                        let _ = fs::create_dir_all(&dir);
                        dir.join("untitled.scene.ron")
                    };

                    match scene_serialization::save_scene(world, &path) {
                        Ok(()) => {
                            self.current_scene_path = Some(path.clone());
                            self.recent_files.add_file(path.clone());
                            self.status = format!("Saved scene to {:?}", path);
                            self.console_logs.push(format!("Scene saved: {:?}", path));
                            self.last_auto_save = std::time::Instant::now();
                            self.is_dirty = false;
                            self.toast_manager.success("Scene saved successfully");
                        }
                        Err(e) => {
                            self.status = format!("Scene save failed: {}", e);
                            self.console_logs
                                .push(format!("Failed to save scene: {}", e));
                            self.toast_manager.error(format!("Save failed: {}", e));
                        }
                    }
                } else {
                    self.console_logs.push("No world to save".into());
                }
            }

            // Ctrl+O: Load Scene (Week 7: with unsaved changes confirmation)
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                // Open file dialog to select scene
                let scenes_dir = self.content_root.join("scenes");
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Open Scene")
                    .set_directory(&scenes_dir)
                    .add_filter("Scene Files", &["ron"])
                    .add_filter("All Files", &["*"])
                    .pick_file()
                {
                    self.request_open_scene(path);
                }
            }

            // Ctrl+C: Copy selected entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) && !i.modifiers.shift {
                if let Some(world) = self.edit_world() {
                    let selected = self.hierarchy_panel.get_all_selected();
                    if !selected.is_empty() {
                        self.clipboard =
                            Some(clipboard::ClipboardData::from_entities(world, &selected));
                        self.status = format!("Copied {} entities", selected.len());
                        self.console_logs
                            .push(format!("Copied {} entities to clipboard", selected.len()));
                    } else {
                        self.console_logs
                            .push("No entities selected to copy".into());
                    }
                }
            }

            // Ctrl+V: Paste entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::V) {
                if let Some(clipboard) = &self.clipboard {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let clipboard_data = clipboard.clone();
                        let offset = IVec2 { x: 1, y: 1 };
                        let cmd =
                            command::SpawnEntitiesCommand::new(clipboard_data.clone(), offset);
                        let paste_result = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        );

                        match paste_result {
                            Ok(()) => {
                                let count = clipboard_data.entities.len();
                                self.status = format!("Pasted {} entities", count);
                                self.console_logs.push(format!("Pasted {} entities", count));
                            }
                            Err(e) => {
                                self.status = format!("Paste failed: {}", e);
                                self.console_logs.push(format!("Paste failed: {}", e));
                            }
                        }
                    }
                } else {
                    self.console_logs.push("Clipboard is empty".into());
                }
            }

            // Ctrl+D: Duplicate selected entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::D) {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let selected = self.hierarchy_panel.get_all_selected();
                    if !selected.is_empty() {
                        let offset = IVec2 { x: 1, y: 1 };
                        let cmd = command::DuplicateEntitiesCommand::new(selected.clone(), offset);
                        let duplicate_result = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        );

                        match duplicate_result {
                            Ok(()) => {
                                self.status = format!("Duplicated {} entities", selected.len());
                                self.console_logs
                                    .push(format!("Duplicated {} entities", selected.len()));
                            }
                            Err(e) => {
                                self.status = format!("Duplicate failed: {}", e);
                                self.console_logs.push(format!("Duplicate failed: {}", e));
                            }
                        }
                    } else {
                        self.console_logs
                            .push("No entities selected to duplicate".into());
                    }
                }
            }

            // F5: Play / Resume
            if i.key_pressed(egui::Key::F5) {
                self.request_play();
            }

            // F6: Pause/Unpause
            if i.key_pressed(egui::Key::F6) {
                if self.editor_mode.is_playing() {
                    self.request_pause();
                } else if self.editor_mode.is_paused() {
                    self.request_play();
                }
            }

            // F7: Stop (restore snapshot)
            if i.key_pressed(egui::Key::F7) {
                self.request_stop();
            }

            // F8: Step one frame
            if i.key_pressed(egui::Key::F8) {
                self.request_step();
            }

            // Delete: Delete selected entities
            if i.key_pressed(egui::Key::Delete) && self.editor_mode.can_edit() {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let selected = self.hierarchy_panel.get_all_selected();
                    if !selected.is_empty() {
                        let cmd = command::DeleteEntitiesCommand::new(selected.clone());
                        let delete_result = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        );

                        match delete_result {
                            Ok(()) => {
                                self.hierarchy_panel.set_selected(None);
                                self.selected_entity = None;
                                self.status = format!(" Deleted {} entities", selected.len());
                                self.console_logs
                                    .push(format!("Deleted {} entities", selected.len()));
                            }
                            Err(e) => {
                                self.status = format!("Delete failed: {}", e);
                                self.console_logs.push(format!("Delete failed: {}", e));
                            }
                        }
                    } else {
                        self.console_logs
                            .push("No entities selected to delete".into());
                    }
                }
            }

            // Ctrl+N: New Scene
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) && !i.modifiers.shift {
                if self.is_dirty {
                    self.show_new_confirm_dialog = true;
                } else {
                    self.create_new_scene();
                }
            }

            // Ctrl+Shift+N: New Entity
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::N) {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let world = scene_state.world_mut();
                    let entity_id = world.spawn(
                        "New Entity",
                        astraweave_core::IVec2 { x: 0, y: 0 },
                        astraweave_core::Team { id: 0 },
                        0,
                        0,
                    );
                    self.selected_entity = Some(u64::from(entity_id));
                    self.hierarchy_panel.set_selected(Some(entity_id));
                    self.is_dirty = true;
                    self.status = format!("Created entity {}", entity_id);
                    self.toast_success("New entity created");
                }
            }

            // Ctrl+Shift+S: Save As
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
                if let Some(world) = self.edit_world() {
                    let dir = self.content_root.join("scenes");
                    let _ = fs::create_dir_all(&dir);
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let path = dir.join(format!("scene_{}.scene.ron", timestamp));

                    match scene_serialization::save_scene(world, &path) {
                        Ok(()) => {
                            self.current_scene_path = Some(path.clone());
                            self.recent_files.add_file(path.clone());
                            self.status = format!("Saved scene as {:?}", path);
                            self.console_logs
                                .push(format!("Scene saved as: {:?}", path));
                        }
                        Err(e) => {
                            self.status = format!("Save As failed: {}", e);
                            self.console_logs.push(format!("Save As failed: {}", e));
                        }
                    }
                }
            }

            // Ctrl+A: Select All entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::A) && !i.modifiers.shift {
                if let Some(world) = self.edit_world() {
                    let all_entities = world.entities();
                    if !all_entities.is_empty() {
                        self.hierarchy_panel.set_selected_multiple(&all_entities);
                        self.status = format!("Selected {} entities", all_entities.len());
                    }
                }
            }

            // Ctrl+I: Import .blend Scene
            if i.modifiers.ctrl && i.key_pressed(egui::Key::I) && !i.modifiers.shift {
                self.on_import_blend_scene();
            }

            // Ctrl+B: Toggle Blueprint Mode
            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) && !i.modifiers.shift {
                self.on_toggle_blueprint_mode();
            }

            // Ctrl+G: Group selected entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::G) && !i.modifiers.shift {
                self.group_selection();
            }

            // Ctrl+Shift+G: Ungroup selected entity
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::G) {
                self.ungroup_selection();
            }

            // Escape: Deselect all (when not in gizmo mode)
            if i.key_pressed(egui::Key::Escape) && self.editor_mode.can_edit() {
                self.hierarchy_panel.set_selected(None);
                self.selected_entity = None;
                self.selection_set.primary = None;
                // Clear viewport selection to stay in sync
                if let Some(viewport) = &mut self.viewport {
                    viewport.clear_selection();
                }
                self.status = "Selection cleared".to_string();
            }

            // F: Focus camera on selected entity
            if i.key_pressed(egui::Key::F) && !i.modifiers.ctrl {
                if let Some(selected_id) = self.selected_entity {
                    if let Some(entity) = self.entity_manager.get(selected_id) {
                        if let Some(viewport) = &mut self.viewport {
                            let entity_pos = glam::Vec3::new(
                                entity.position.x,
                                entity.position.y,
                                entity.position.z,
                            );
                            viewport.camera_mut().frame_entity(entity_pos, 2.0);
                            self.status = format!("Focused on entity {}", selected_id);
                        }
                    }
                } else {
                    self.status = "No entity selected to focus".to_string();
                }
            }

            // Home: Reset camera to origin
            if i.key_pressed(egui::Key::Home) {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().reset_to_origin();
                    self.status = "Camera reset to origin".to_string();
                }
            }

            // Numpad 1: Front view
            if i.key_pressed(egui::Key::Num1) && i.modifiers.alt {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().set_view_front();
                    self.status = "Front view".to_string();
                }
            }

            // Numpad 3: Right view
            if i.key_pressed(egui::Key::Num3) && i.modifiers.alt {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().set_view_right();
                    self.status = "Right view".to_string();
                }
            }

            // Numpad 7: Top view
            if i.key_pressed(egui::Key::Num7) && i.modifiers.alt {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().set_view_top();
                    self.status = "Top view".to_string();
                }
            }

            // Numpad 0 / Alt+0: Perspective view
            if i.key_pressed(egui::Key::Num0) && i.modifiers.alt {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().set_view_perspective();
                    self.status = "Perspective view".to_string();
                }
            }

            // F1: Show keyboard shortcuts help
            if i.key_pressed(egui::Key::F1) {
                self.show_help_dialog = !self.show_help_dialog;
            }

            // G: Toggle grid visibility
            if i.key_pressed(egui::Key::G) && !i.modifiers.ctrl {
                self.show_grid = !self.show_grid;
                self.status = if self.show_grid {
                    "Grid enabled".to_string()
                } else {
                    "Grid disabled".to_string()
                };
            }

            // Escape: Close dialogs (Week 7: Added open confirm and quit dialogs)
            if i.key_pressed(egui::Key::Escape) {
                if self.show_quit_dialog {
                    self.show_quit_dialog = false;
                } else if self.show_new_confirm_dialog {
                    self.show_new_confirm_dialog = false;
                } else if self.show_open_confirm_dialog {
                    self.pending_open_path = None;
                    self.show_open_confirm_dialog = false;
                } else if self.show_settings_dialog {
                    self.save_preferences();
                    self.show_settings_dialog = false;
                } else if self.show_help_dialog {
                    self.show_help_dialog = false;
                }
            }

            // Ctrl+1..6: Layout presets
            let layout_keys = [
                (egui::Key::Num1, dock_layout::LayoutPreset::Default),
                (egui::Key::Num2, dock_layout::LayoutPreset::Wide),
                (egui::Key::Num3, dock_layout::LayoutPreset::Compact),
                (egui::Key::Num4, dock_layout::LayoutPreset::Modeling),
                (egui::Key::Num5, dock_layout::LayoutPreset::Animation),
                (egui::Key::Num6, dock_layout::LayoutPreset::Debug),
            ];
            for (key, preset) in layout_keys {
                if i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(key) {
                    self.dock_layout.apply_preset(preset);
                    self.status = format!("Layout: {:?}", preset);
                }
            }

            // Ctrl+D duplicate is handled by the clipboard-based DuplicateEntitiesCommand above.
            // (Legacy shallow-copy handler removed — it created ghost entities in EntityManager
            //  without corresponding World entities, corrupting selection state.)
        });

        // Phase 2.2 legacy autosave removed — handled by Week 7 enhanced autosave

        let stats = self.runtime.stats().clone();
        self.performance_panel.set_frame_time(frame_time * 1000.0);
        if !self.editor_mode.is_editing() {
            self.performance_panel.push_runtime_stats(&stats);

            if self.last_runtime_log.elapsed().as_millis() >= 500 {
                self.profiler_data.push(format!(
                    "Tick {:05} | {:>4} ents | {:>5.2} ms | {:>3.0} FPS",
                    stats.tick_count, stats.entity_count, stats.frame_time_ms, stats.fps
                ));
                if self.profiler_data.len() > 60 {
                    self.profiler_data.remove(0);
                }
                self.last_runtime_log = std::time::Instant::now();
            }
        }
        self.performance_panel.update();
        self.charts_panel.update();
        self.world_panel.update();
        self.animation_panel.update(frame_time);

        self.measured_ui_setup_ms = ui_setup_start.elapsed().as_secs_f32() * 1000.0;

        let render_start = std::time::Instant::now();

        self.show_top_panel(ctx);

        self.show_status_bar(ctx);

        // LEFT PANEL - Only show in legacy mode (pruned for docking)
        if !self.use_docking {
            self.show_legacy_left_panel(ctx);
        }

        // Render main content area - either docking layout or legacy panels
        if self.use_docking {
            self.show_docking_layout(ctx);
        } else {
            self.show_legacy_central_panel(ctx);
        }

        self.render_toasts(ctx);

        self.measured_render_ms = render_start.elapsed().as_secs_f32() * 1000.0;

        // --- Audio subsystem: process panel actions and tick engine ---
        let audio_start = std::time::Instant::now();
        let audio_actions = self.dock_tab_viewer.take_audio_actions();
        if !audio_actions.is_empty() {
            self.audio_bridge.process_actions(audio_actions);
        }
        self.audio_bridge.tick(frame_time);
        // Push live stats back into the audio panel
        self.dock_tab_viewer
            .set_audio_stats(self.audio_bridge.stats());
        let measured_audio_ms = audio_start.elapsed().as_secs_f32() * 1000.0;

        // Update listener position to match the camera
        if let Some(viewport) = &self.viewport {
            let cam = viewport.camera();
            self.audio_bridge
                .update_listener(cam.position(), cam.forward(), cam.up());
        }

        // --- Animation subsystem: sync panel playback state to bridge ---
        {
            let panel = &self.animation_panel;
            // If the panel's editor is actively playing, bridge that to the entity
            if panel.playback_state == panels::animation::PlaybackState::Playing {
                if let Some(entity_id) = panel.selected_entity {
                    self.animation_bridge
                        .assign_clip(u64::from(entity_id), panel.selected_clip_idx.unwrap_or(0));
                }
            }
        }
        self.animation_bridge.tick(frame_time);

        // --- Skinning: apply CPU skinning for entities with active skeleton animations ---
        {
            let entities_with_meshes: Vec<(u64, String)> = self
                .entity_manager
                .entities()
                .iter()
                .filter_map(|(&id, entity)| entity.mesh.as_ref().map(|path| (id, path.clone())))
                .collect();

            // Sync skeleton/animation data from loaded meshes → animation bridge
            if let Some(viewport) = &self.viewport {
                if let Ok(renderer) = viewport.renderer().lock() {
                    for (entity_id, mesh_path) in &entities_with_meshes {
                        if !self.animation_bridge.has_skeleton(*entity_id) {
                            if let Some(skel) = renderer.get_mesh_skeleton(mesh_path) {
                                self.animation_bridge
                                    .set_entity_skeleton(*entity_id, skel.clone());
                                let clips = renderer.get_mesh_animations(mesh_path);
                                if !clips.is_empty() {
                                    self.animation_bridge
                                        .set_entity_clips(*entity_id, clips.to_vec());
                                }
                            }
                        }
                    }
                }
            }

            // Apply CPU skinning for entities with active animations
            for (entity_id, mesh_path) in &entities_with_meshes {
                if let Some(joint_matrices) =
                    self.animation_bridge.compute_joint_matrices(*entity_id)
                {
                    if let Some(viewport) = &self.viewport {
                        if let Ok(mut renderer) = viewport.renderer().lock() {
                            renderer.apply_cpu_skinning(mesh_path, &joint_matrices);
                        }
                    }
                }
            }
        }

        // --- Movement scripts: tick entity movement in play mode ---
        if self.runtime.is_playing() {
            let mut scripted: Vec<(
                u64,
                movement_scripts::MovementScript,
                glam::Vec3,
                glam::Quat,
            )> = Vec::new();
            for entity in self.entity_manager.entities().values() {
                if let Some(script_val) = entity.components.get("MovementScript") {
                    if let Some(script) = movement_scripts::MovementScript::from_json(script_val) {
                        scripted.push((entity.id, script, entity.position, entity.rotation));
                    }
                }
            }
            if !scripted.is_empty() {
                let results = self.movement_system.tick_all(&scripted, frame_time);
                for (id, new_pos, new_rot) in results {
                    self.entity_manager
                        .update_transform(id, new_pos, new_rot, glam::Vec3::ONE);
                }
            }
        }

        let tick_start = std::time::Instant::now();
        if let Err(e) = self.runtime.tick(frame_time) {
            self.console_logs
                .push(format!("Runtime tick failed: {}", e));
        }
        self.measured_tick_ms = tick_start.elapsed().as_secs_f32() * 1000.0;

        // Push subsystem timings to profiler panel
        self.profiler_panel.push_subsystem_timings(
            crate::panels::profiler_panel::SubsystemTimings {
                render: self.measured_render_ms,
                physics: self.measured_tick_ms,
                audio: measured_audio_ms,
                ui: self.measured_ui_setup_ms,
                ..Default::default()
            },
        );

        // Safeguard: never let the OS cursor become invisible in the editor.
        // egui-winit translates CursorIcon::None → window.set_cursor_visible(false).
        ctx.output_mut(|o| {
            if o.cursor_icon == egui::CursorIcon::None {
                o.cursor_icon = egui::CursorIcon::Default;
            }
        });
    }
}

fn main() -> Result<()> {
    // Initialize observability
    if let Err(e) = astraweave_observability::init_observability(Default::default()) {
        eprintln!("Warning: Failed to initialize observability: {}", e);
    }

    // Create content directory if it doesn't exist
    let content_dir = PathBuf::from("content");
    let _ = fs::create_dir_all(&content_dir);
    let _ = fs::create_dir_all(content_dir.join("levels"));
    let _ = fs::create_dir_all(content_dir.join("encounters"));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_title("AstraWeave Level & Encounter Editor"),
        wgpu_options: egui_wgpu::WgpuConfiguration {
            wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(egui_wgpu::WgpuSetupCreateNew {
                device_descriptor: std::sync::Arc::new(|adapter| {
                    let base_limits = if adapter.get_info().backend == wgpu::Backend::Gl {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    };
                    // Enable wireframe rendering if the GPU supports it
                    let mut features = wgpu::Features::default();
                    if adapter
                        .features()
                        .contains(wgpu::Features::POLYGON_MODE_LINE)
                    {
                        features |= wgpu::Features::POLYGON_MODE_LINE;
                    }
                    wgpu::DeviceDescriptor {
                        label: Some("egui wgpu device"),
                        required_features: features,
                        required_limits: wgpu::Limits {
                            max_texture_dimension_2d: 8192,
                            max_bind_groups: 8,
                            ..base_limits
                        },
                        memory_hints: wgpu::MemoryHints::default(),
                        trace: wgpu::Trace::Off,
                    }
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "AstraWeave Level & Encounter Editor",
        options,
        Box::new(|cc| {
            // Use EditorApp::new() to initialize viewport with CreationContext
            match EditorApp::new(cc) {
                Ok(app) => Ok(Box::new(app) as Box<dyn eframe::App>),
                Err(e) => Err(format!("{:?}", e).into()),
            }
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))?;
    Ok(())
}
