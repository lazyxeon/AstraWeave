#![forbid(unsafe_code)]
//! Procedural Content Generation (PCG) with seed reproducibility
//!
//! This crate provides deterministic procedural generation for AstraWeave:
//! - Seed-based RNG with explicit seeding per layer
//! - Encounter placement with constraints
//! - Layout generation (rooms, paths)

pub mod encounters;
pub mod layout;
pub mod seed_rng;
pub mod wfc;

pub use encounters::{Encounter, EncounterConstraints, EncounterGenerator, EncounterKind};
pub use layout::{LayoutGenerator, Room};
pub use seed_rng::SeedRng;
pub use wfc::{Direction, TileId, TileSet, WfcError, WfcGrid};
