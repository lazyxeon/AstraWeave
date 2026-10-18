// Advanced GOAP System Module
// Phase 1: Technical Foundation

#[cfg(feature = "planner_advanced")]
pub mod action;
#[cfg(feature = "planner_advanced")]
pub mod actions;
#[cfg(feature = "planner_advanced")]
pub mod adapter;
#[cfg(feature = "planner_advanced")]
pub mod config;
#[cfg(feature = "planner_advanced")]
pub mod debug_tools;
#[cfg(feature = "planner_advanced")]
pub mod goal;
#[cfg(feature = "planner_advanced")]
pub mod goal_authoring; // Phase 4
#[cfg(feature = "planner_advanced")]
pub mod goal_scheduler; // Phase 4
#[cfg(feature = "planner_advanced")]
pub mod goal_validator; // Phase 5
#[cfg(feature = "planner_advanced")]
pub mod history;
#[cfg(feature = "planner_advanced")]
pub mod learning;
#[cfg(feature = "planner_advanced")]
pub mod orchestrator;
#[cfg(feature = "planner_advanced")]
pub mod persistence;
#[cfg(feature = "planner_advanced")]
pub mod plan_analyzer; // Phase 5
#[cfg(feature = "planner_advanced")]
pub mod plan_stitcher; // Phase 4
#[cfg(feature = "planner_advanced")]
pub mod plan_visualizer; // Phase 5
#[cfg(feature = "planner_advanced")]
pub mod planner;
#[cfg(feature = "planner_advanced")]
pub mod shadow_mode;
#[cfg(feature = "planner_advanced")]
pub mod state;
#[cfg(feature = "planner_advanced")]
pub mod telemetry;
#[cfg(feature = "planner_advanced")]
pub mod tests; // Phase 5

#[cfg(feature = "planner_advanced")]
pub use action::{Action, SimpleAction};
#[cfg(feature = "planner_advanced")]
pub use actions::register_all_actions;
#[cfg(feature = "planner_advanced")]
pub use adapter::SnapshotAdapter;
#[cfg(feature = "planner_advanced")]
pub use config::{CostTuningConfig, GOAPConfig, LearningConfig, SmoothingConfig, SmoothingMethod};
#[cfg(feature = "planner_advanced")]
pub use debug_tools::{Explanation, PlanDebugger, ProgressReport, StateChange, StateDiff};
#[cfg(feature = "planner_advanced")]
pub use goal::{DecompositionStrategy, Goal};
#[cfg(feature = "planner_advanced")]
pub use goal_authoring::{GoalDefinition, GoalLibrary, StateValueDef};
#[cfg(feature = "planner_advanced")]
pub use goal_scheduler::GoalScheduler;
#[cfg(feature = "planner_advanced")]
pub use goal_validator::{GoalValidator, Severity, ValidationError, ValidationResult};
#[cfg(feature = "planner_advanced")]
pub use history::{ActionHistory, ActionStats};
#[cfg(feature = "planner_advanced")]
pub use learning::LearningManager;
#[cfg(feature = "planner_advanced")]
pub use orchestrator::GOAPOrchestrator;
#[cfg(feature = "planner_advanced")]
pub use persistence::{HistoryPersistence, PersistenceFormat};
#[cfg(feature = "planner_advanced")]
pub use plan_analyzer::{
    ComparisonReport, PlanAnalyzer, PlanMetrics, Suggestion, SuggestionPriority,
};
#[cfg(feature = "planner_advanced")]
pub use plan_stitcher::{Conflict, PlanStitcher, StitchError};
#[cfg(feature = "planner_advanced")]
pub use plan_visualizer::{PlanVisualizer, VisualizationFormat};
#[cfg(feature = "planner_advanced")]
pub use planner::AdvancedGOAP;
#[cfg(feature = "planner_advanced")]
pub use shadow_mode::{PlanComparison, ShadowModeRunner};
#[cfg(feature = "planner_advanced")]
pub use state::{OrderedFloat, StateValue, WorldState};
#[cfg(feature = "planner_advanced")]
pub use telemetry::{PlanExecutionTracker, TelemetryCollector};
