//! Aravath greybox runtime.
//!
//! The voxel grid is the ground. Physics trimeshes and render meshes are
//! rebuilt from dirty chunks. Fate-weaving, the editor, and the LLM stack
//! are not dependencies of this crate. `bridge_agent` is the companion
//! route across the span. The window does not draw that body yet.

pub mod bridge_agent;
pub mod combat;
pub mod figure;
pub mod ground;
pub mod hud;
pub mod island;
pub mod locomotion;
pub mod motor;
pub mod player;
pub mod props;
pub mod save;
pub mod sim;

pub use bridge_agent::BridgeAgent;
pub use ground::{material_tint, ChunkUpload, Ground};
pub use player::Player;
pub use sim::Greybox;
