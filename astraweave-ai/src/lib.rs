//! # AstraWeave AI
//!
//! AI orchestration and planning layer for AstraWeave.
//!
//! This crate implements the engine's AI-native architecture, providing:
//!
//! - **[`orchestrator`]** — The [`Orchestrator`] trait that abstracts AI planning
//!   (rule-based, behavior-tree, LLM, or hybrid).
//! - **[`core_loop`]** — The perception → reasoning → planning → action pipeline.
//! - **[`ecs_ai_plugin`]** — ECS integration via [`AiPlanningPlugin`] and
//!   [`build_app_with_ai()`].
//! - **[`tool_sandbox`]** — Runtime validation of AI-generated action plans.
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `llm_orchestrator` | Enables the LLM executor. A model may propose a plan. The sim validates it. |
//! | `veilweaver_slice` | Slice companion method select |
//!
//! # Performance
//!
//! - Behavior trees: 57–253 ns per tick (66,000 agents @ 60 FPS)
//! - Validated capacity: 12,700+ agents @ 60 FPS

pub mod core_loop;
#[cfg(feature = "decision-trace")]
pub mod decision_trace;
pub mod ecs_ai_plugin;
pub mod htn;
pub mod orchestrator;
pub mod scar_sense;
pub mod tool_sandbox;

/// Record a decision. With `decision-trace` off, the tokens are discarded.
#[macro_export]
macro_rules! decision_record {
    ($($tokens:tt)*) => {
        #[cfg(feature = "decision-trace")]
        {
            $($tokens)*
        }
    };
}

#[cfg(test)]
mod mutation_tests;

#[cfg(feature = "llm_orchestrator")]
pub mod async_task;

#[cfg(feature = "llm_orchestrator")]
pub mod llm_executor;

#[cfg(feature = "veilweaver_slice")]
pub mod veilweaver;

pub use astraweave_nav::bridge_and_ford;
pub use core_loop::*;
pub use ecs_ai_plugin::{build_app_with_ai, AiPlanningPlugin};
pub use htn::{SpatialTick, HTN_EXPANSIONS_PER_TICK};
pub use orchestrator::*;
pub use scar_sense::{sense, stance_at, ScarStance};
pub use tool_sandbox::*;

#[cfg(feature = "llm_orchestrator")]
pub use async_task::AsyncTask;

#[cfg(feature = "llm_orchestrator")]
pub use llm_executor::LlmExecutor;

#[cfg(feature = "veilweaver_slice")]
pub use veilweaver::VeilweaverCompanionOrchestrator;
