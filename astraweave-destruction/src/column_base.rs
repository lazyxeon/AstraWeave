//! Integer column hook for the Aravath ground.
//!
//! The functions are supplied by the caller (`surface_height` and
//! `column_material` from the terrain crate). This crate does not depend on
//! that crate. `FlatColumnBase` stays the unit-test double.

/// Seed column sampled by function pointer. No heap, no floats.
#[derive(Clone, Copy, Debug)]
pub struct ColumnFns {
    pub top: fn(i32, i32) -> i32,
    pub material: fn(i32, i32, i32) -> u16,
}

impl ColumnFns {
    pub fn top(self, x: i32, z: i32) -> i32 {
        (self.top)(x, z)
    }

    pub fn material(self, x: i32, y: i32, z: i32) -> u16 {
        (self.material)(x, y, z)
    }
}
