#![forbid(unsafe_code)]
//! Deterministic fate-scar field.
//!
//! A death writes transient energy into a sparse world-space grid. The energy
//! decays toward a raised permanent floor. The grid is Q32.32 fixed-point,
//! stored in a `BTreeMap`, and stepped by tick count. Float conversion happens
//! only in [`gpu_f32`], when baking a volume for the existing volumetric fog
//! pass.
//!
//! Agents read [`ScarField::perceive_metres`]. The Aravath demo asks
//! `astraweave-ai` to turn that percept into a stance.

mod field;
mod fixed;
mod gpu;
mod percept;
mod save_blob;

pub use field::{
    ScarCell, ScarField, AVOID_TENSION, CELL_METRES, DEATH_ENERGY, GPU_DIM, PEAK_TENSION,
    WARY_TENSION,
};
pub use fixed::Fixed64;
pub use gpu::{gpu_f32, GpuScarVolume};
pub use percept::ScarPercept;
pub use save_blob::{insert_into_meta, read_from_meta, META_KEY};
