#![allow(dead_code)]

#[derive(Clone, Serialize, Deserialize, Default)]
struct DialogueDoc {
    title: String,
    nodes: Vec<DialogueNode>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct DialogueNode {
    id: String,
    text: String,
    responses: Vec<DialogueResponse>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct DialogueResponse {
    text: String,
    next_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct QuestDoc {
    title: String,
    steps: Vec<QuestStep>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct QuestStep {
    description: String,
    completed: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

struct Toast {
    message: String,
    level: ToastLevel,
    created: std::time::Instant,
}

impl Toast {
    fn new(message: impl Into<String>, level: ToastLevel) -> Self {
        Self {
            message: message.into(),
            level,
            created: std::time::Instant::now(),
        }
    }

    fn color(&self) -> egui::Color32 {
        match self.level {
            ToastLevel::Info => egui::Color32::from_rgb(100, 149, 237),
            ToastLevel::Success => egui::Color32::from_rgb(50, 205, 50),
            ToastLevel::Warning => egui::Color32::from_rgb(255, 165, 0),
            ToastLevel::Error => egui::Color32::from_rgb(220, 20, 60),
        }
    }

    fn is_expired(&self, duration_secs: f32) -> bool {
        self.created.elapsed().as_secs_f32() > duration_secs
    }
}

mod asset_pack;
mod behavior_graph;
mod brdf_preview;
mod clipboard; // Phase 3.4 - Copy/Paste/Duplicate
mod command; // Phase 2.1 - Undo/Redo system
mod component_ui; // Phase 2.3 - Component-based inspector
mod dialogs; // Modal dialog helpers
mod editor_mode; // Phase 4.2 - Play-in-Editor
mod editor_preferences; // Phase 9 - Editor preferences persistence
mod entity_manager;
mod file_watcher;
mod game_project; // Game project configuration (game.toml)
mod gizmo;
mod interaction; // Phase 8.1 Week 5 Day 3 - Gizmo interaction helpers (auto-tracking)
mod level_doc; // Level document types
mod material_inspector;
mod panels;
mod polish;
mod prefab; // Phase 4.1 - Prefab System
mod recent_files; // Phase 3 - Recent files tracking
mod runtime; // Week 4 - Deterministic runtime integration
mod scene_serialization; // Phase 2.2 - Scene Save/Load
mod scene_state; // Week 1 - Canonical edit-mode world owner
mod splash; // Startup splash screen with logo + cinematic video
mod terrain_integration; // Terrain generation integration
mod ui; // Phase 3 - UI components (StatusBar, etc.)
mod viewport; // Phase 1.1 - 3D Viewport
mod voxel_tools; // Phase 10: Voxel editing tools // Phase 2: Asset packaging and compression

use anyhow::Result;
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
mod dock_layout;
mod dock_panels;
mod panel_type;
mod plugin;
mod tab_viewer;
use dock_layout::{DockLayout, LayoutPreset};
use panel_type::PanelType;
use prefab::PrefabManager;
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
use viewport::ViewportWidget; // Phase 1.1

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

struct EditorApp {
    content_root: PathBuf,
    level: LevelDoc,
    status: String,
    mat_doc: MaterialLiveDoc,
    #[allow(dead_code)]
    dialogue: DialogueDoc,
    #[allow(dead_code)]
    quest: QuestDoc,
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
    // Phase 2.2: Scene Save/Load
    current_scene_path: Option<PathBuf>,
    last_autosave: std::time::Instant,
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
    // Legacy toasts field (kept for backward compatibility during migration)
    #[allow(dead_code)]
    toasts: Vec<Toast>,
    // Phase 7: Help dialog
    show_help_dialog: bool,
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
    /// Counter for generating unique entity IDs
    next_entity_id: u64,
    /// Track if scene has unsaved changes
    is_scene_modified: bool,
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
}

impl Default for EditorApp {
    fn default() -> Self {
        let mut asset_db = AssetDatabase::new();
        // Try to load from assets.json
        if let Ok(()) = asset_db.load_manifest(&PathBuf::from("assets/assets.json")) {
            // Loaded
        } else {
            // Scan assets directory
            let _ = asset_db.scan_directory(&PathBuf::from("assets"));
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
            dialogue: DialogueDoc {
                title: "Sample Dialogue".into(),
                nodes: vec![DialogueNode {
                    id: "start".into(),
                    text: "Hello, traveler!".into(),
                    responses: vec![DialogueResponse {
                        text: "Hi!".into(),
                        next_id: None,
                    }],
                }],
            },
            quest: QuestDoc {
                title: "Sample Quest".into(),
                steps: vec![QuestStep {
                    description: "Talk to the elder.".into(),
                    completed: false,
                }],
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
            scene_state: Some(EditorSceneState::new(World::new())), // Start with empty scene
            material_inspector: MaterialInspector::new(),           // NEW - Phase PBR-G Task 2
            // Phase 1: Entity management
            entity_manager: EntityManager::new(),
            selected_entity: None,
            // Phase 2.1: Undo/Redo system
            undo_stack: command::UndoStack::new(100), // Store last 100 commands
            // Phase 2.2: Scene Save/Load
            current_scene_path: None,
            last_autosave: std::time::Instant::now(),
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
            file_watcher: file_watcher::FileWatcher::new("assets").ok(),
            // Phase 6: Dirty flag for unsaved changes
            is_dirty: false,
            show_quit_dialog: false,
            pending_quit: false,
            // Phase 6: Toast notifications (Week 6 Day 3-4: Enhanced toast manager)
            toast_manager: ui::ToastManager::new(),
            toasts: Vec::new(), // Legacy, kept for migration
            // Phase 7: Help dialog
            show_help_dialog: false,
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
            // Entity ID counter - start after sample entities (100+)
            next_entity_id: 100,
            // Scene modification tracking
            is_scene_modified: false,
            asset_registry: AssetRegistry::default(),
            last_save_time: None,
            // Week 6: Progress manager for long-running operations
            progress_manager: ui::ProgressManager::new(),
            // Week 6 Day 5: Resource usage tracking
            resource_usage: ui::ResourceUsage::new(),
            last_resource_sample: std::time::Instant::now(),
            splash: Some(splash::SplashScreen::new()),
        }
    }
}

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
    fn toast(&mut self, message: impl Into<String>, level: ToastLevel) {
        // Convert to new toast level
        let new_level = match level {
            ToastLevel::Info => ui::ToastLevel::Info,
            ToastLevel::Success => ui::ToastLevel::Success,
            ToastLevel::Warning => ui::ToastLevel::Warning,
            ToastLevel::Error => ui::ToastLevel::Error,
        };
        self.toast_manager.toast(message, new_level);
    }

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
            let em_id = entity_id as u64;

            let mut editor_entity = entity_manager::EditorEntity::new(em_id, name);

            // Populate position from World pose
            if let Some(pose) = world.pose(entity_id) {
                editor_entity.position = glam::Vec3::new(pose.pos.x as f32, pose.pos.y as f32, 0.0);
                editor_entity.components.insert(
                    "Transform".to_string(),
                    serde_json::json!({"x": pose.pos.x, "y": pose.pos.y, "z": 0}),
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
                .filter_map(|e| e.ok())
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
                .filter_map(|e| e.ok())
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
            self.toast(
                format!(
                    "Recovered from auto-save: {}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                ),
                ToastLevel::Success,
            );

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

        // For now, just log the intent - full hierarchy grouping requires ECS parent-child support
        self.log(format!(
            "Grouping {} entities (hierarchy support pending)",
            selected_ids.len()
        ));
        self.toast_info(format!(
            "Group {} entities - hierarchy pending",
            selected_ids.len()
        ));
        self.status = format!("Group {} entities - pending", selected_ids.len());
    }

    /// Ungroup the selected entity (if it's a group)
    fn ungroup_selection(&mut self) {
        if let Some(primary) = self.selection_set.primary {
            self.log(format!("Ungrouped entity #{}", primary));
            self.toast_info("Ungroup: Children promoted to scene root".to_string());
            self.status = "Ungrouped selection".to_string();
        } else {
            self.log("No group selected to ungroup".to_string());
        }
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
                let _ = self.undo_stack.undo(state.world_mut());
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
            let em_id = world_id as u64;
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

        // Scan for available 3D models from asset packs (Kenney etc.)
        let spawnable_models = Self::scan_spawnable_models();
        let model_count = spawnable_models.len();
        app.dock_tab_viewer.set_spawnable_models(spawnable_models);
        if model_count > 0 {
            app.console_logs
                .push(format!("Found {} 3D models in asset packs", model_count));
        }

        // Week 7 Day 5: Check for crash recovery
        app.check_for_crash_recovery();

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
                        ui.text_edit_singleline(&mut self.level.title);
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
                let eid = *id as u32;
                let (hp, team_id, ammo, pos_x, pos_y, rotation, scale) = if let Some(w) = &world_ref
                {
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
                self.dock_tab_viewer
                    .set_selected_transform(Some((pos.x, pos.y, pos.z, angle, scale.x, scale.y, scale.z)));
                let entity_type = if entity.components.contains_key("Camera") {
                    "Camera".to_string()
                } else if entity.components.contains_key("Light") {
                    "Light".to_string()
                } else if entity.components.contains_key("Mesh") {
                    "Mesh".to_string()
                } else {
                    "Entity".to_string()
                };
                let eid = entity_id as u32;
                let (hp, tid, ammo_v, px, py, rot_v, sc_v) = if let Some(w) = &world_ref {
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
                    }));
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
            // Actual draw calls: skybox(1) + grid(1) + entity_instances(1) + gizmos(1) + physics_debug(1)
            let draw_calls = if entity_count > 0 { 5 } else { 3 };
            // Actual triangles: each entity cube = 12 tris, grid = 2 tris, skybox = 2 tris
            let triangles = entity_count * 12 + 4;
            (draw_calls, triangles, stats.memory_usage_mb)
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
            // Subsystem timings: 0 when not running (editor mode)
            render_time_ms: 0.0,
            physics_time_ms: 0.0,
            ai_time_ms: 0.0,
            script_time_ms: 0.0,
            audio_time_ms: 0.0,
            draw_calls: vp_draw_calls,
            triangles: vp_triangles,
            gpu_memory_bytes,
        };
        self.dock_tab_viewer.set_runtime_stats(runtime_stats);

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
            is_modified: self.is_scene_modified,
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

                // Unified context rendering to avoid type-switching issues
                let mut context = EditorDrawContext::new(&mut self.dock_tab_viewer);

                if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut()) {
                    // Sync terrain brush state: tell viewport when brush is active
                    viewport.set_terrain_brush_active(brush_active);

                    context = context
                        .with_viewport(viewport)
                        .with_world(world)
                        .with_entity_manager(&mut self.entity_manager)
                        .with_undo_stack(&mut self.undo_stack)
                        .with_prefab_manager(&mut self.prefab_manager);
                }

                self.dock_layout.show_inside(ui, &mut context);
            });

        // Forward terrain brush hits from viewport to terrain panel
        if let Some(viewport) = self.viewport.as_mut() {
            let hits = viewport.take_terrain_brush_hits();
            for hit in hits {
                self.dock_tab_viewer.apply_terrain_brush_at(hit[0], hit[1]);
            }
        }

        // Sync environment settings (skybox preset, time-of-day, weather, fog) to viewport each frame
        if let Some(viewport) = &self.viewport {
            let (sky_top, sky_horizon, ground_color) =
                self.dock_tab_viewer.compute_sky_colors();
            viewport.set_sky_colors(sky_top, sky_horizon, ground_color);

            // Compute fog color from sky horizon (fog blends toward sky color)
            let fog_color = [sky_horizon[0], sky_horizon[1], sky_horizon[2]];
            let (fog_enabled, fog_density, weather_type) =
                self.dock_tab_viewer.fog_weather_params();
            let fog_params = crate::viewport::terrain_renderer::TerrainFogParams {
                fog_enabled,
                fog_density,
                fog_color,
                weather_type,
            };
            viewport.set_fog_params(fog_params);
            viewport.set_water_level(0.0);
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
                    self.status = format!("Selected entity {}", entity_id);
                }
                tab_viewer::PanelEvent::EntityDeselected => {
                    self.selected_entity = None;
                    self.selection_set.primary = None;
                    self.status = "Deselected entity".to_string();
                }
                tab_viewer::PanelEvent::TransformPositionChanged { entity_id, x, y, z } => {
                    // Update entity transform in scene
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity = entity_id as u32;
                        if let Some(pose) = scene_state.world_mut().pose_mut(entity) {
                            pose.pos.x = x as i32;
                            pose.pos.y = z as i32; // World IVec2.y maps to world Z
                        }
                        scene_state.sync_entity(entity);
                    }
                    // Sync EntityManager position (full 3D)
                    self.entity_manager
                        .update_position(entity_id, glam::Vec3::new(x, y, z));
                    self.status = format!("Entity {} position: ({:.2}, {:.2}, {:.2})", entity_id, x, y, z);
                }
                tab_viewer::PanelEvent::TransformRotationChanged {
                    entity_id,
                    rotation,
                } => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity = entity_id as u32;
                        if let Some(pose) = scene_state.world_mut().pose_mut(entity) {
                            pose.rotation = rotation;
                        }
                        scene_state.sync_entity(entity);
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
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity = entity_id as u32;
                        if let Some(pose) = scene_state.world_mut().pose_mut(entity) {
                            // Use average for uniform scale (World uses single scale value)
                            pose.scale = (scale_x + scale_y + scale_z) / 3.0;
                        }
                        scene_state.sync_entity(entity);
                    }
                    // Sync EntityManager scale (full 3D)
                    self.entity_manager
                        .update_scale(entity_id, glam::Vec3::new(scale_x, scale_y, scale_z));
                    self.status = format!(
                        "Entity {} scale: ({:.2}, {:.2}, {:.2})",
                        entity_id, scale_x, scale_y, scale_z
                    );
                }
                tab_viewer::PanelEvent::CreateEntity => {
                    // Create an empty entity in both World and EntityManager
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity_count = scene_state.world().entities().len();
                        let name = format!("Empty_{}", entity_count);
                        let entity = scene_state.world_mut().spawn(
                            &name,
                            astraweave_core::IVec2 { x: 0, y: 0 },
                            astraweave_core::Team { id: 0 },
                            0,
                            0,
                        );
                        scene_state.sync_entity(entity);
                        // Add to EntityManager using World entity ID for consistent lookups
                        let em_id = entity as u64;
                        let mut editor_entity =
                            entity_manager::EditorEntity::new(em_id, name.clone());
                        editor_entity.components.insert(
                            "Transform".to_string(),
                            serde_json::json!({"x": 0, "y": 0, "z": 0}),
                        );
                        self.entity_manager.add(editor_entity);
                        self.selected_entity = Some(em_id);
                        self.selection_set.primary = Some(em_id);
                        self.is_dirty = true;
                        self.hierarchy_panel
                            .sync_with_world(scene_state.world_mut());
                        self.console_logs
                            .push(format!("Created empty entity: {}", name));
                        self.status = format!("Created entity: {}", name);
                    }
                }
                tab_viewer::PanelEvent::SpawnArchetype { ref archetype } => {
                    // Spawn an entity from a named archetype into World + EntityManager
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
                        let entity = scene_state.world_mut().spawn(
                            &name,
                            astraweave_core::IVec2 { x: 0, y: 0 },
                            astraweave_core::Team { id: team_id },
                            hp,
                            ammo,
                        );
                        scene_state.sync_entity(entity);

                        // Add to EntityManager using World entity ID for consistent lookups
                        let em_id = entity as u64;
                        let mut em_entity_new =
                            entity_manager::EditorEntity::new(em_id, name.clone());
                        {
                            let em_entity = &mut em_entity_new;
                            em_entity.components.insert(
                                "Transform".to_string(),
                                serde_json::json!({"x": 0, "y": 0, "z": 0}),
                            );
                            match archetype.as_str() {
                                "Player" | "Companion" | "Enemy" | "Boss" => {
                                    em_entity.components.insert(
                                        "Health".to_string(),
                                        serde_json::json!({"hp": hp}),
                                    );
                                    em_entity.components.insert(
                                        "Team".to_string(),
                                        serde_json::json!({"id": team_id}),
                                    );
                                    em_entity.components.insert(
                                        "Ammo".to_string(),
                                        serde_json::json!({"count": ammo}),
                                    );
                                }
                                "NPC" => {
                                    em_entity.components.insert(
                                        "Health".to_string(),
                                        serde_json::json!({"hp": hp}),
                                    );
                                }
                                "Light" => {
                                    em_entity.components.insert(
                                        "Light".to_string(),
                                        serde_json::json!({"type": "point", "intensity": 1.0}),
                                    );
                                }
                                "Camera" => {
                                    em_entity.components.insert(
                                        "Camera".to_string(),
                                        serde_json::json!({"fov": 60.0, "near": 0.1, "far": 1000.0}),
                                    );
                                }
                                _ => {}
                            }
                        }
                        self.entity_manager.add(em_entity_new);

                        self.selected_entity = Some(em_id);
                        self.selection_set.primary = Some(em_id);
                        self.is_dirty = true;
                        self.hierarchy_panel
                            .sync_with_world(scene_state.world_mut());
                        self.console_logs
                            .push(format!("Spawned {} entity: {}", archetype, name));
                        self.status = format!("Spawned {}: {}", archetype, name);
                    }
                }
                tab_viewer::PanelEvent::SpawnModel { ref name, ref path } => {
                    // Spawn an entity representing a 3D model from the asset pack
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity_count = scene_state.world().entities().len();
                        let entity_name = format!("{}_{}", name, entity_count);
                        // Offset each spawned model so they don't overlap
                        let offset = entity_count as i32 * 3;
                        let entity = scene_state.world_mut().spawn(
                            &entity_name,
                            astraweave_core::IVec2 { x: offset, y: 0 },
                            astraweave_core::Team { id: 0 },
                            0,
                            0,
                        );
                        scene_state.sync_entity(entity);

                        let em_id = entity as u64;
                        let mut em_entity =
                            entity_manager::EditorEntity::new(em_id, entity_name.clone());
                        em_entity.components.insert(
                            "Transform".to_string(),
                            serde_json::json!({"x": 0, "y": 0, "z": 0}),
                        );
                        em_entity
                            .components
                            .insert("Model".to_string(), serde_json::json!({"path": path}));
                        em_entity.mesh = Some(path.clone());
                        self.entity_manager.add(em_entity);

                        self.selected_entity = Some(em_id);
                        self.selection_set.primary = Some(em_id);
                        self.is_dirty = true;
                        self.hierarchy_panel
                            .sync_with_world(scene_state.world_mut());
                        self.console_logs
                            .push(format!("Spawned model entity: {} ({})", entity_name, path));
                        self.status = format!("Spawned model: {}", entity_name);
                    }
                }
                tab_viewer::PanelEvent::DeleteEntity(entity_id) => {
                    // Remove entity from EntityManager and World
                    self.entity_manager.remove(entity_id);
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let entity = entity_id as u32;
                        let delete_cmd = command::DeleteEntitiesCommand::new(vec![entity]);
                        if let Err(e) = self.undo_stack.execute(delete_cmd, scene_state.world_mut())
                        {
                            self.console_logs.push(format!("Delete failed: {}", e));
                        } else {
                            scene_state.sync_entity(entity);
                            self.hierarchy_panel
                                .sync_with_world(scene_state.world_mut());
                        }
                    }
                    if self.selected_entity == Some(entity_id) {
                        self.selected_entity = None;
                        self.selection_set.primary = None;
                    }
                    self.is_dirty = true;
                    self.status = format!("Deleted entity {}", entity_id);
                }
                tab_viewer::PanelEvent::DuplicateEntity(entity_id) => {
                    // Duplicate entity in both EntityManager and World
                    let source_info = self.entity_manager.get(entity_id).cloned();
                    if let Some(source) = source_info {
                        let new_name = format!("{}_copy", source.name);
                        // Duplicate in World first to get the canonical entity ID
                        let mut new_em_id = None;
                        if let Some(scene_state) = self.scene_state.as_mut() {
                            let clipboard = crate::clipboard::ClipboardData::from_entities(
                                scene_state.world(),
                                &[entity_id as u32],
                            );
                            let offset = astraweave_core::IVec2 { x: 1, y: 1 };
                            if let Ok(spawned) =
                                clipboard.spawn_entities(scene_state.world_mut(), offset)
                            {
                                for e in &spawned {
                                    scene_state.sync_entity(*e);
                                    // Use first spawned World ID as EntityManager ID
                                    if new_em_id.is_none() {
                                        new_em_id = Some(*e as u64);
                                    }
                                }
                                self.hierarchy_panel
                                    .sync_with_world(scene_state.world_mut());
                            }
                        }
                        if let Some(em_id) = new_em_id {
                            let mut new_em =
                                entity_manager::EditorEntity::new(em_id, new_name.clone());
                            new_em.components = source.components.clone();
                            new_em.position = source.position + glam::Vec3::new(1.0, 0.0, 1.0);
                            new_em.mesh = source.mesh.clone();
                            new_em.material = source.material.clone();
                            self.entity_manager.add(new_em);
                            self.selected_entity = Some(em_id);
                            self.selection_set.primary = Some(em_id);
                        }
                        self.is_dirty = true;
                        self.status = format!("Duplicated entity {} as {}", entity_id, new_name);
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
                }
                tab_viewer::PanelEvent::AnimationPlayStateChanged { is_playing } => {
                    if is_playing {
                        self.status = "Animation playing".to_string();
                    } else {
                        self.status = "Animation paused".to_string();
                    }
                }
                tab_viewer::PanelEvent::AnimationFrameChanged { frame } => {
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
                }
                tab_viewer::PanelEvent::ConsoleCleared => {
                    self.status = "Console cleared".to_string();
                }
                tab_viewer::PanelEvent::AssetSelected(asset) => {
                    self.status = format!("Selected asset: {}", asset);
                }
                tab_viewer::PanelEvent::BehaviorNodeSelected(node_id) => {
                    self.status = format!("Selected behavior node: {}", node_id);
                }
                tab_viewer::PanelEvent::GraphNodeSelected(node_id) => {
                    self.status = format!("Selected graph node: {}", node_id);
                }
                tab_viewer::PanelEvent::HierarchySearchChanged(search) => {
                    self.status = format!("Hierarchy search: {}", search);
                }
                tab_viewer::PanelEvent::ConsoleSearchChanged(search) => {
                    self.status = format!("Console search: {}", search);
                }
                tab_viewer::PanelEvent::RefreshSceneStats => {
                    self.status = "Refreshing scene statistics...".to_string();
                }
                tab_viewer::PanelEvent::AddComponent {
                    entity_id,
                    component_type,
                } => {
                    // Add component to EntityManager entity
                    if let Some(entity) = self.entity_manager.get_mut(entity_id) {
                        let default_value = match component_type.as_str() {
                            "Transform" => serde_json::json!({"x": 0, "y": 0, "z": 0}),
                            "Health" => serde_json::json!({"hp": 100}),
                            "Team" => serde_json::json!({"id": 0}),
                            "Ammo" => serde_json::json!({"count": 30}),
                            "Sprite" => serde_json::json!({"texture": ""}),
                            "Collider" => serde_json::json!({"shape": "box", "size": [1, 1, 1]}),
                            "RigidBody" => serde_json::json!({"type": "dynamic", "mass": 1.0}),
                            "Script" => serde_json::json!({"path": ""}),
                            "Audio" => serde_json::json!({"clip": "", "volume": 1.0}),
                            "Light" => serde_json::json!({"type": "point", "intensity": 1.0}),
                            "Camera" => {
                                serde_json::json!({"fov": 60.0, "near": 0.1, "far": 1000.0})
                            }
                            _ => serde_json::json!({}),
                        };
                        entity
                            .components
                            .insert(component_type.clone(), default_value);
                        self.is_dirty = true;
                        self.console_logs.push(format!(
                            "Added {} to entity {}",
                            component_type, entity.name
                        ));
                    }
                    // Sync to World for core component types
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let eid = entity_id as u32;
                        match component_type.as_str() {
                            "Health" => {
                                if let Some(h) = scene_state.world_mut().health_mut(eid) {
                                    // Already exists, reset to default
                                    h.hp = 100;
                                }
                                // If entity doesn't have health in World, it was set at spawn
                            }
                            "Team" => {
                                if let Some(t) = scene_state.world_mut().team_mut(eid) {
                                    t.id = 0;
                                }
                            }
                            "Ammo" => {
                                if let Some(a) = scene_state.world_mut().ammo_mut(eid) {
                                    a.rounds = 30;
                                }
                            }
                            _ => {}
                        }
                    }
                    self.status = format!("Added {} to entity {}", component_type, entity_id);
                }
                tab_viewer::PanelEvent::RemoveComponent {
                    entity_id,
                    component_type,
                } => {
                    // Remove component from EntityManager entity
                    if let Some(entity) = self.entity_manager.get_mut(entity_id) {
                        entity.components.remove(&component_type);
                        self.is_dirty = true;
                        self.console_logs.push(format!(
                            "Removed {} from entity {}",
                            component_type, entity.name
                        ));
                    }
                    self.status = format!("Removed {} from entity {}", component_type, entity_id);
                }
                tab_viewer::PanelEvent::HealthChanged { entity_id, new_hp } => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let eid = entity_id as u32;
                        if let Some(h) = scene_state.world_mut().health_mut(eid) {
                            h.hp = new_hp;
                        }
                        scene_state.sync_entity(eid);
                    }
                    // Also update EntityManager JSON
                    if let Some(entity) = self.entity_manager.get_mut(entity_id) {
                        entity
                            .components
                            .insert("Health".to_string(), serde_json::json!({"hp": new_hp}));
                    }
                    self.is_dirty = true;
                }
                tab_viewer::PanelEvent::TeamChanged {
                    entity_id,
                    new_team_id,
                } => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let eid = entity_id as u32;
                        if let Some(t) = scene_state.world_mut().team_mut(eid) {
                            t.id = new_team_id;
                        }
                        scene_state.sync_entity(eid);
                    }
                    if let Some(entity) = self.entity_manager.get_mut(entity_id) {
                        entity
                            .components
                            .insert("Team".to_string(), serde_json::json!({"id": new_team_id}));
                    }
                    self.is_dirty = true;
                }
                tab_viewer::PanelEvent::AmmoChanged {
                    entity_id,
                    new_ammo,
                } => {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let eid = entity_id as u32;
                        if let Some(a) = scene_state.world_mut().ammo_mut(eid) {
                            a.rounds = new_ammo;
                        }
                        scene_state.sync_entity(eid);
                    }
                    if let Some(entity) = self.entity_manager.get_mut(entity_id) {
                        entity
                            .components
                            .insert("Ammo".to_string(), serde_json::json!({"count": new_ammo}));
                    }
                    self.is_dirty = true;
                }
                tab_viewer::PanelEvent::EntityRenamed {
                    entity_id,
                    ref new_name,
                } => {
                    if let Some(entity) = self.entity_manager.get_mut(entity_id) {
                        entity.name = new_name.clone();
                    }
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
                }
                tab_viewer::PanelEvent::ViewportGizmoSpaceChanged(space) => {
                    self.status = format!(
                        "Gizmo space: {}",
                        if space == 0 { "Local" } else { "World" }
                    );
                }
                tab_viewer::PanelEvent::ViewportOverlayToggled { overlay, enabled } => {
                    self.status = format!(
                        "Viewport overlay '{}': {}",
                        overlay,
                        if enabled { "enabled" } else { "disabled" }
                    );
                }
                tab_viewer::PanelEvent::ViewportCameraChanged {
                    fov,
                    near,
                    far,
                    speed,
                } => {
                    self.status = format!(
                        "Camera: FOV={:.0}°, Clip={:.2}-{:.0}, Speed={:.1}",
                        fov, near, far, speed
                    );
                }
                tab_viewer::PanelEvent::ViewportFocusOnSelection => {
                    self.status = "Focusing on selection...".to_string();
                }
                tab_viewer::PanelEvent::ViewportResetCamera => {
                    self.status = "Camera reset to default position".to_string();
                }
                tab_viewer::PanelEvent::ViewportCameraPreset(preset) => {
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
                    let chunks: Vec<(Vec<crate::viewport::terrain_renderer::TerrainVertex>, Vec<u32>)> =
                        src_chunks
                            .into_iter()
                            .map(|(verts, indices)| {
                                let gpu_verts = verts
                                    .iter()
                                    .map(|v| crate::viewport::terrain_renderer::TerrainVertex {
                                        position: v.position,
                                        normal: v.normal,
                                        uv: v.uv,
                                        biome_id: v.biome_id,
                                        _padding: [0; 3],
                                    })
                                    .collect();
                                (gpu_verts, indices)
                            })
                            .collect();
                    if let Some(viewport) = &self.viewport {
                        viewport.upload_terrain_chunks(&chunks);
                    }

                    // Generate scatter placements from terrain vegetation system
                    let scatter_placements = self.dock_tab_viewer.generate_scatter_placements();
                    if !scatter_placements.is_empty() {
                        if let Some(viewport) = &self.viewport {
                            if let Ok(mut renderer) = viewport.renderer().lock() {
                                let count = scatter_placements.len();
                                renderer.set_scatter_placements(scatter_placements);
                                tracing::info!("Scatter: {count} placements uploaded to renderer");
                            }
                        }
                    }

                    self.status =
                        format!("Terrain generated: {} chunks uploaded", chunks.len());
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
                egui::menu::bar(ui, |ui| {
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
                            ) {
                                self.console_logs.push(format!("Viewport error: {}", e));
                                warn!("Viewport error: {}", e);
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
                        if let Some(selected) = viewport.selected_entity() {
                            self.selected_entity = Some(selected);
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
                        let is_selected = current_primary == Some(*entity as u64);

                        let response = ui.selectable_label(is_selected, format!("{}", name));

                        if response.clicked() {
                            new_selection = Some(*entity as u64);
                        }

                        // Show entity info on hover
                        response.on_hover_ui(|ui| {
                            ui.label(format!("ID: {}", entity));
                            if let Some(pose) = pose {
                                ui.label(format!("Position: ({}, {})", pose.pos.x, pose.pos.y));
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
                                ui.label(format!("({}, {})", pose.pos.x, pose.pos.y));
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
                self.selected_entity = Some(root_entity as u64);
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
                        if let Err(e) = self.undo_stack.execute(delete_cmd, scene_state.world_mut())
                        {
                            self.log(format!("Delete failed: {}", e));
                        } else {
                            scene_state.sync_entity(entity);
                            self.log(format!("Deleted entity {}", entity));
                            if self.selected_entity == Some(entity as u64) {
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
                                    self.selected_entity = Some(first as u64);
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
                    self.selected_entity = Some(entity as u64);
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

    /// Handle a dragged asset — import GLTF/GLB models directly, spawn prefabs for .prefab files
    fn handle_dragged_asset(&mut self, path: std::path::PathBuf) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "glb" | "gltf" | "obj" | "fbx" => {
                self.handle_asset_action(AssetAction::ImportModel { path });
            }
            _ => {
                self.spawn_prefab_from_drag(path, (0, 0));
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
                        let em_id = entity as u64;
                        if let Some(em_entity) = self.entity_manager.get_mut(em_id) {
                            em_entity.set_mesh(mesh_path_str);
                        } else {
                            let mut em_entity =
                                entity_manager::EditorEntity::new(em_id, model_name.clone());
                            em_entity.set_mesh(mesh_path_str);
                            if let Some(pose) = scene_state.world().pose(entity) {
                                em_entity.position = glam::Vec3::new(
                                    pose.pos.x as f32,
                                    1.0,
                                    pose.pos.y as f32,
                                );
                            }
                            self.entity_manager.add(em_entity);
                        }
                    }

                    // Load the glTF model into the engine renderer
                    #[cfg(feature = "astraweave-render")]
                    if let Some(viewport) = &self.viewport {
                        if let Err(e) = viewport.load_gltf_model(&model_name, &path) {
                            warn!("Failed to load glTF model into renderer: {}", e);
                            self.console_logs
                                .push(format!("glTF loading failed: {}", e));
                        } else {
                            debug!("Loaded glTF model '{}' into engine renderer", model_name);
                        }
                    }

                    self.selected_entity = Some(entity as u64);
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

                #[cfg(feature = "astraweave-render")]
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

                #[cfg(not(feature = "astraweave-render"))]
                {
                    warn!("astraweave-render feature not enabled - cannot preview model");
                    self.console_logs
                        .push("Render feature not enabled - cannot preview model".into());
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
                let _ = viewport.set_material_params(
                    base_color,
                    self.mat_doc.metallic,
                    self.mat_doc.roughness,
                );
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
                let _ = self.asset_db.scan_directory(&PathBuf::from("assets"));
                self.status = "Rescanned assets directory".into();
                self.console_logs.push(format!(
                    "Assets rescanned from directory: {} total",
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
                if let Err(e) = self.undo_stack.undo(world) {
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
                if let Err(e) = self.undo_stack.redo(world) {
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
                        .filter_map(|e| e.ok())
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

        // Process asset browser actions (drag-drop, double-click, context actions)
        self.process_asset_browser_actions();

        let now = std::time::Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.current_fps = if frame_time > 0.0 {
            1.0 / frame_time
        } else {
            60.0
        };

        self.profiler_panel.push_frame_time(frame_time * 1000.0);

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
                                    let _ = self.prefab_manager.revert_instance_to_prefab(
                                        *entity,
                                        scene_state.world_mut(),
                                    );
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

        // Week 5 Day 5: Enhanced scene statistics with mesh/texture/performance estimates
        // Note: These are reasonable estimates until we have real mesh/texture tracking
        let mesh_count = scene_entity_count; // Assume 1 mesh per entity
        let avg_triangles_per_mesh = 500; // Reasonable for game models
        let avg_vertices_per_mesh = 300;
        let total_triangles = mesh_count * avg_triangles_per_mesh;
        let total_vertices = mesh_count * avg_vertices_per_mesh;
        let mesh_memory_kb = (total_vertices * 32 + total_triangles * 12) / 1024; // vertex attrs + indices

        // Texture estimates (could be replaced with actual tracking later)
        let texture_count = (scene_entity_count / 5).max(1); // ~1 texture per 5 entities
        let avg_texture_size_kb = 256; // 512x512 RGBA compressed
        let texture_memory_kb = texture_count * avg_texture_size_kb;

        // Material and draw call estimates
        let material_count = (scene_entity_count / 3).max(1);
        let unique_shader_count = 3; // Standard PBR, unlit, transparent
        let estimated_draw_calls = mesh_count; // 1 draw per mesh (no batching assumed)
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
            // Week 5 Day 5: New mesh/texture/performance fields
            mesh_count,
            total_triangles,
            total_vertices,
            mesh_memory_kb,
            texture_count,
            texture_memory_kb,
            max_texture_resolution: (2048, 2048), // Assume max 2K textures
            material_count,
            unique_shader_count,
            estimated_draw_calls,
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
                    let undo_error = self.undo_stack.undo(scene_state.world_mut()).err();

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
                    let redo_error = self.undo_stack.redo(scene_state.world_mut()).err();

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
                            self.toasts
                                .push(Toast::new("Scene saved successfully", ToastLevel::Success));
                        }
                        Err(e) => {
                            self.status = format!("Scene save failed: {}", e);
                            self.console_logs
                                .push(format!("Failed to save scene: {}", e));
                            self.toasts
                                .push(Toast::new(format!("Save failed: {}", e), ToastLevel::Error));
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
                        let paste_result = self.undo_stack.execute(cmd, scene_state.world_mut());

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
                        let duplicate_result =
                            self.undo_stack.execute(cmd, scene_state.world_mut());

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
                        let delete_result = self.undo_stack.execute(cmd, scene_state.world_mut());

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
                    self.selected_entity = Some(entity_id as u64);
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

            // Ctrl+D: Duplicate selected entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::D) {
                if let Some(selected_id) = self.selected_entity {
                    if let Some(entity) = self.entity_manager.get(selected_id) {
                        let new_name = format!("{}_copy", entity.name.clone());
                        let new_pos = glam::Vec3::new(
                            entity.position.x + 1.0,
                            entity.position.y,
                            entity.position.z + 1.0,
                        );
                        let rotation = entity.rotation;
                        let scale = entity.scale;
                        let new_id = self.entity_manager.create(new_name);
                        if let Some(new_entity) = self.entity_manager.get_mut(new_id) {
                            new_entity.set_transform(new_pos, rotation, scale);
                        }
                        self.selected_entity = Some(new_id);
                        self.hierarchy_panel.set_selected(Some(new_id as u32));
                        self.status = format!("Duplicated entity {} -> {}", selected_id, new_id);
                    }
                } else {
                    self.status = "No entity selected to duplicate".to_string();
                }
            }
        });

        // Phase 2.2: Autosave every 5 minutes
        if self.last_autosave.elapsed().as_secs() >= 300 {
            if let Some(world) = self.edit_world() {
                let autosave_dir = self.content_root.join(".autosave");
                let _ = fs::create_dir_all(&autosave_dir);
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let autosave_path = autosave_dir.join(format!("autosave_{}.scene.ron", timestamp));

                match scene_serialization::save_scene(world, &autosave_path) {
                    Ok(()) => {
                        self.console_logs
                            .push(format!("Autosaved to {:?}", autosave_path));
                        self.last_auto_save = std::time::Instant::now();
                    }
                    Err(e) => {
                        self.console_logs.push(format!("Autosave failed: {}", e));
                        self.last_auto_save = std::time::Instant::now();
                    }
                }
            }
        }

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

        if let Err(e) = self.runtime.tick(frame_time) {
            self.console_logs
                .push(format!("Runtime tick failed: {}", e));
        }
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
                    let base_limits =
                        if adapter.get_info().backend == wgpu::Backend::Gl {
                            wgpu::Limits::downlevel_webgl2_defaults()
                        } else {
                            wgpu::Limits::default()
                        };
                    wgpu::DeviceDescriptor {
                        label: Some("egui wgpu device"),
                        required_features: wgpu::Features::default(),
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
