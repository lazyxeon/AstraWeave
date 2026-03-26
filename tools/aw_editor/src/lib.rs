// Library exports for aw_editor (enables benchmarks and testing)
#![allow(dead_code)] // Suppress dead code warnings for benchmark-only exports

extern crate self as aw_editor_lib;

pub mod animation_bridge;
pub mod asset_pack;
pub mod audio_bridge;
pub mod behavior_graph;
pub mod blend_scanner;
pub mod clipboard;
pub mod command;
pub mod component_ui;
pub mod distribution;
pub mod dock_layout;
pub mod editor_mode;
pub mod editor_preferences;
pub mod entity_manager;
pub mod file_watcher;
pub mod game_project;
pub mod level_doc;
pub mod movement_scripts;
pub mod panel_type;
pub mod panels;
pub mod plugin;
pub mod polish;
pub mod prefab;
pub mod recent_files;
pub mod runtime;
pub mod scene_serialization;
pub mod scene_state;
pub mod tab_viewer;
pub mod terrain_integration;
pub mod ui;
pub mod viewport;

#[cfg(test)]
mod mutation_tests;
#[cfg(test)]
mod tests_asset_pack;
#[cfg(test)]
mod tests_dock_layout;
#[cfg(test)]
mod tests_game_project;
#[cfg(test)]
mod tests_polish;

// Re-export level document types
pub use level_doc::{
    BiomePaint, BossCfg, Circle, DirectorOp, FateThread, FortRegion, LevelDoc, NpcSpawn, Obstacle,
    Sky, Spawn, Trigger,
};

pub use command::{
    EditorCommand, MoveEntityCommand, RotateEntityCommand, ScaleEntityCommand, UndoStack,
};
pub use dock_layout::{DockLayout, LayoutPreset};
pub use editor_mode::EditorMode;
pub use entity_manager::{EditorEntity, EntityId, EntityManager, SelectionSet};
pub use panel_type::PanelType;
pub use plugin::{
    EditorPlugin, PluginContext, PluginError, PluginEvent, PluginManager, PluginMetadata,
};
pub use prefab::{
    PrefabData, PrefabEntitySnapshot, PrefabHierarchySnapshot, PrefabInstance,
    PrefabInstanceSnapshot, PrefabManager,
};
pub use runtime::{EditorRuntime, GameInput, RuntimeState, RuntimeStats};
pub use scene_serialization::{EntityData, SceneData};
pub use scene_state::{EditorSceneState, TransformableScene};
pub use tab_viewer::{EditorTabViewer, PanelEvent, SimpleTabViewer};
pub use ui::StatusBar;

pub mod gizmo {
    // Export all modules
    pub mod constraints;
    pub mod input;
    pub mod picking;
    pub mod rendering;
    pub mod rotate;
    pub mod scale;
    pub mod scene_viewport;
    pub mod snapping;
    pub mod state;
    pub mod translate;

    #[cfg(test)]
    pub mod tests_gizmo_math;

    // Re-export commonly used types for convenience
    pub use constraints::apply_constraint;
    pub use input::NumericInput;
    pub use picking::{GizmoHandle, GizmoPicker, Ray};
    pub use rendering::{GizmoRenderParams, GizmoRenderer};
    pub use rotate::RotateGizmo;
    pub use scale::ScaleGizmo;
    pub use scene_viewport::{CameraController, SceneViewport, Transform};
    pub use snapping::SnappingConfig;
    pub use state::{AxisConstraint, GizmoMode, GizmoState, TransformSnapshot};
    pub use translate::TranslateGizmo;
}

// Headless testing infrastructure
pub mod headless;
pub mod interaction;
pub mod telemetry;
