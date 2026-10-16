//! libm stand-ins for the character controller.
//!
//! Rapier's `enhanced-determinism` feature already sends the solver through
//! `libm`. These wrappers cover the slope angle and the jump impulse, which
//! are computed in this crate before Rapier sees them.

pub fn sqrt(value: f32) -> f32 {
    libm::sqrtf(value)
}

pub fn acos(value: f32) -> f32 {
    libm::acosf(value)
}
