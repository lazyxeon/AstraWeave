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
//! | `llm_orchestrator` | Enables LLM executor and async task infrastructure |
//! | `veilweaver_slice` | Veilweaver-specific companion orchestrator |
//! | `planner_advanced` | GOAP planner with caching and visualization |
//!
//! # Performance
//!
//! - GOAP planning: 1.01 µs cache hit, 47.2 µs cache miss
//! - Behavior trees: 57–253 ns per tick (66,000 agents @ 60 FPS)
//! - Arbiter cycle: 313.7 ns (GOAP + LLM poll + metrics)
//! - Validated capacity: 12,700+ agents @ 60 FPS

pub mod core_loop;
pub mod ecs_ai_plugin;
pub mod orchestrator;
pub mod scar_sense;
pub mod tool_sandbox;

#[cfg(test)]
mod mutation_tests;

// Phase 7 Arbiter: Async infrastructure for GOAP+Hermes hybrid control
#[cfg(feature = "llm_orchestrator")]
pub mod async_task;

#[cfg(feature = "llm_orchestrator")]
pub mod llm_executor;

#[cfg(feature = "llm_orchestrator")]
pub mod ai_arbiter;

#[cfg(feature = "veilweaver_slice")]
pub mod veilweaver;

// Advanced GOAP module (Phase 1)
#[cfg(feature = "planner_advanced")]
pub mod goap;

pub use core_loop::*;
pub use ecs_ai_plugin::{build_app_with_ai, AiPlanningPlugin};
pub use orchestrator::*;
pub use scar_sense::{sense, stance_at, ScarStance};
pub use tool_sandbox::*;

#[cfg(feature = "llm_orchestrator")]
pub use async_task::AsyncTask;

#[cfg(feature = "llm_orchestrator")]
pub use llm_executor::LlmExecutor;

#[cfg(feature = "llm_orchestrator")]
pub use ai_arbiter::{AIArbiter, AIControlMode};

#[cfg(feature = "veilweaver_slice")]
pub use veilweaver::VeilweaverCompanionOrchestrator;
