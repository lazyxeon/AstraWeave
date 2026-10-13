//! Aravath greybox runtime.
//!
//! The voxel grid is the ground. Physics trimeshes and render meshes are
//! rebuilt from dirty chunks. Fate-weaving, the editor, and the LLM stack
//! are not dependencies of this crate.

pub mod ground;
pub mod island;
pub mod player;
pub mod sim;

pub use ground::{material_tint, ChunkUpload, Ground};
pub use player::Player;
pub use sim::Greybox;
