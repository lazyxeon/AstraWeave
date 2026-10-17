//! # AstraWeave Gameplay
//!
//! High-level gameplay systems and mechanics for AstraWeave.
//!
//! This crate provides production-ready game systems:
//!
//! - **Combat** — Raycast-based attack sweep with cone filtering, parry, and i-frames
//!   ([`combat_physics::perform_attack_sweep`]).
//! - **Crafting** — Recipe system with material requirements.
//! - **Quests** — Quest tracking with objectives and completion logic.
//! - **Dialogue** — Branching NPC conversation trees.
//! - **Stats** — Character statistics and damage calculation.
//! - **Items** — Inventory management and item definitions.
//! - **Biome** — Biome definitions, spawn rules, and biome transitions.
//! - **Veilweaver** — Game-specific mechanics (weaving, portals, telemetry, tutorial).
//! - **ECS Integration** — System registration for all gameplay subsystems.

pub mod biome;
pub mod biome_spawn;
pub mod combat;
pub mod combat_physics;
pub mod crafting;
pub mod cutscenes;
pub mod dialogue;
pub mod ecs;
pub mod harvesting;
pub mod items;
pub mod quests;
pub mod stats;
pub mod types;
pub mod veilweaver_slice;
pub mod veilweaver_tutorial;
pub mod water_movement;
pub mod weaving;

pub use biome::*;
pub use biome_spawn::*;
pub use combat::*;
pub use combat_physics::*;
pub use crafting::*;
pub use cutscenes::*;
pub use dialogue::*;
pub use ecs::*;
pub use harvesting::*;
pub use items::*;
pub use quests::*;
pub use stats::*;
pub use types::*;
pub use veilweaver_slice::*;
pub use veilweaver_tutorial::*;
pub use water_movement::*;
pub use weaving::*;

pub mod weave_portals;
pub mod weave_telemetry;
pub use weave_portals::*;
pub use weave_telemetry::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mutation_tests;

#[cfg(test)]
mod test_support;
