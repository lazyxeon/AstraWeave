//! Deterministic causal world state.
//!
//! Games opt in by depending on this crate. The fixed tick admits recorded
//! inputs, fires a baked rule table, and never calls a model. A recorded line
//! may decorate a template the rules already emitted. It does not change stock,
//! stance, or quest state.
//!
//! The Aravath bridge slice stays out until destruction issue #279 publishes
//! real collapse bounds. This crate does not read a geometry notice.

#![forbid(unsafe_code)]

mod rules;
mod world;

pub use rules::*;
pub use world::*;
