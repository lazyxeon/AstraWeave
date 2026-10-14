#![forbid(unsafe_code)]
//! Multi-agent coordination system for LLM-powered game entities
//!
//! This crate provides systems for managing interactions between multiple AI agents,
//! including NPCs, directors, dialogue systems, and quest generators. It handles
//! coordination, resource sharing, and emergent group behaviors.

pub mod agent;
pub use agent::*;

pub mod coordination;
pub use coordination::*;

pub mod sim_tick;
pub use sim_tick::*;

// pub mod social_graph;     // Source file does not exist on disk
// pub use social_graph::*;

pub mod world_events;
pub use world_events::*;

pub mod narrative_coherence;
pub use narrative_coherence::*;

// pub mod components;       // Source file does not exist on disk
// pub use components::*;

// pub mod systems;          // Source file does not exist on disk
// pub use systems::*;
