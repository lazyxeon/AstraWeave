//! # AstraWeave Core
//!
//! Central shared types and simulation vocabulary for the AstraWeave engine.
//!
//! This crate defines the canonical data types used across all engine subsystems:
//!
//! - **[`schema`]** — [`WorldSnapshot`], [`PlayerState`], [`CompanionState`], [`EnemyState`],
//!   [`PlanIntent`], [`ActionStep`], and other shared gameplay types.
//! - **[`tool_sandbox`] / [`tool_vocabulary`]** — [`ToolRegistry`] and [`ToolSpec`] for
//!   AI action validation (the "tool sandbox" pattern).
//! - **[`capture_replay`]** — Deterministic capture and replay infrastructure.
//! - **[`perception`]** — AI perception helpers (`astar_path`, `find_cover_positions`, `los_clear`).
//! - **[`validation`]** — Configuration validation traits.
//! - **[`metrics`]** — Telemetry and performance counters.
//!
//! # Architecture
//!
//! The core crate sits at the bottom of the dependency graph. Every other crate
//! (AI, physics, gameplay, rendering) depends on `astraweave-core` for shared
//! type definitions, ensuring a single source of truth for the AI-native loop:
//!
//! ```text
//! Perception → Reasoning → Planning → Action
//!     ↓            ↓           ↓          ↓
//! WorldSnapshot  AI Model   PlanIntent  Tool Validation
//! ```

pub mod capture_replay;
pub use capture_replay::*;
pub mod ecs_adapter;
pub mod ecs_bridge;
pub mod ecs_components;
pub mod ecs_events;
pub mod metrics;
pub mod perception;
pub mod schema;
pub mod sim;
pub mod tool_sandbox;
pub mod tool_vocabulary;
pub mod tools;
pub mod util;
pub mod validation;
pub mod world;

#[cfg(kani)]
mod schema_kani;

#[cfg(test)]
mod mutation_tests;

pub use perception::*;
pub use schema::*;
pub use sim::*;
// Note: tools::Poi and schema::Poi are different types - using qualified imports where needed
pub use ecs_components::*;
pub use tool_sandbox::*;
pub use tool_vocabulary::*;
pub use tools::{
    astar_path, find_cover_positions, glam_to_schema, los_clear, path_exists, schema_to_glam,
};
pub use util::{byte_prefix, byte_suffix, ceil_char_boundary, floor_char_boundary};
pub use validation::*;
pub use world::*;

/// Four core tools, named as `ActionStep`: `MoveTo`, `Throw`, `CoverFire`, `Revive`.
///
/// `LlmOrchestrator::new` uses this registry when the caller passes none.
/// `validate_plan` and `parse_llm_response` match these names exactly.
pub fn default_tool_registry() -> ToolRegistry {
    use std::collections::BTreeMap;
    ToolRegistry {
        tools: vec![
            ToolSpec {
                name: "MoveTo".into(),
                args: BTreeMap::from([("x".into(), "i32".into()), ("y".into(), "i32".into())]),
            },
            ToolSpec {
                name: "Throw".into(),
                args: BTreeMap::from([
                    ("item".into(), "enum[smoke,grenade]".into()),
                    ("x".into(), "i32".into()),
                    ("y".into(), "i32".into()),
                ]),
            },
            ToolSpec {
                name: "CoverFire".into(),
                args: BTreeMap::from([
                    ("target_id".into(), "u32".into()),
                    ("duration".into(), "f32".into()),
                ]),
            },
            ToolSpec {
                name: "Revive".into(),
                args: BTreeMap::from([("ally_id".into(), "u32".into())]),
            },
        ],
        constraints: Constraints {
            enforce_cooldowns: true,
            enforce_los: true,
            enforce_stamina: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tool_registry_has_four_tools() {
        let registry = default_tool_registry();
        assert_eq!(registry.tools.len(), 4);
    }

    #[test]
    fn test_default_tool_registry_move_to() {
        let registry = default_tool_registry();
        let move_to = registry.tools.iter().find(|t| t.name == "MoveTo");
        assert!(move_to.is_some());
        let tool = move_to.unwrap();
        assert_eq!(tool.args.get("x"), Some(&"i32".to_string()));
        assert_eq!(tool.args.get("y"), Some(&"i32".to_string()));
    }

    #[test]
    fn test_default_tool_registry_throw() {
        let registry = default_tool_registry();
        let throw = registry.tools.iter().find(|t| t.name == "Throw");
        assert!(throw.is_some());
        let tool = throw.unwrap();
        assert_eq!(
            tool.args.get("item"),
            Some(&"enum[smoke,grenade]".to_string())
        );
    }

    #[test]
    fn test_default_tool_registry_constraints() {
        let registry = default_tool_registry();
        assert!(registry.constraints.enforce_cooldowns);
        assert!(registry.constraints.enforce_los);
        assert!(registry.constraints.enforce_stamina);
    }

    #[test]
    fn test_default_tool_registry_cover_fire() {
        let registry = default_tool_registry();
        let cover_fire = registry.tools.iter().find(|t| t.name == "CoverFire");
        assert!(cover_fire.is_some());
    }

    #[test]
    fn test_default_tool_registry_revive() {
        let registry = default_tool_registry();
        let revive = registry.tools.iter().find(|t| t.name == "Revive");
        assert!(revive.is_some());
    }
}
