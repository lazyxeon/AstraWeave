//! Write contract for the field PR #253 owns.
//!
//! Weaving expands a template into [`FieldWrite`] stamps. It does not store
//! regions, channels, or a cell window. `LocalSim::tick` applies the stamps
//! and `LocalSim::field_digest` is the 32 bytes the weaving replay stores
//! once both branches share a base. Until then [`NullField`] accepts the
//! writes and returns a zero digest.

/// Matches `astraweave-localsim` (`POOL` is theirs; this cap is the write ring).
pub const MAX_FIELD_WRITES: usize = 64;
pub const CELL_MM: i32 = 1000;

pub const PHASE_DRY: u8 = 0;
pub const PHASE_WET: u8 = 1;
pub const PHASE_ICE: u8 = 2;
pub const PHASE_BURN: u8 = 3;
pub const PHASE_GAS: u8 = 4;
pub const PHASE_OIL: u8 = 5;

pub const FLAG_WET: u8 = 1 << 0;
pub const FLAG_ICE: u8 = 1 << 1;
pub const FLAG_BURN: u8 = 1 << 2;
pub const FLAG_SLIP: u8 = 1 << 3;
pub const FLAG_CONDUCTIVE: u8 = 1 << 4;

/// World gravity as milli-g. One g down is `-1000`, which is `-9810` mm/s².
pub const WORLD_GRAVITY_MILLI_G: i32 = -1000;

/// `milli_g` of `-1000` converts to `-9810` mm/s² (`-9.81 m/s²`).
pub fn gravity_mm_s2(milli_g: i32) -> i32 {
    milli_g.saturating_mul(981) / 100
}

/// Cell index from metres. Truncates toward zero after scaling to millimetres.
///
/// Returns `None` when `m` is not finite or `m * 1000` does not fit in `i32`.
/// The float-to-int cast saturates, so that range is rejected before the cast.
/// The replay path passes cell coordinates and does not call this.
pub fn meters_to_cell(m: f32) -> Option<i32> {
    if !m.is_finite() {
        return None;
    }
    let milli = m * 1000.0;
    if milli < i32::MIN as f32 || milli > i32::MAX as f32 {
        return None;
    }
    Some((milli as i32).div_euclid(CELL_MM))
}

/// `cell * CELL_MM` when the product fits in `i32`. Does not saturate.
pub fn cell_to_milli(cell: i32) -> Option<i32> {
    cell.checked_mul(CELL_MM)
}

/// Three cell coordinates scaled to millimetres, or `None` if any product overflows.
pub fn milli_corner(x: Option<i32>, y: Option<i32>, z: Option<i32>) -> Option<[i32; 3]> {
    Some([cell_to_milli(x?)?, cell_to_milli(y?)?, cell_to_milli(z?)?])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Point,
    Sphere,
    Segment,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteOp {
    AddTemp { delta: i16, stack: bool },
    SetPhase { phase: u8 },
    AddDensity { delta: i8, stack: bool },
    SetFlags { set: u8, clear: u8 },
    SetGravity { index: u16 },
    AddVolume { delta: i16 },
    SetWind { x: i8, y: i8, z: i8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldWrite {
    pub seq: u32,
    pub shape: Shape,
    pub origin: [i32; 3],
    pub end: [i32; 3],
    pub radius: u8,
    pub op: WriteOp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldEvent {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub kind: u8,
    pub material_id: u16,
    pub temp: i16,
}

/// What weaving is allowed to do to the grid #253 owns.
///
/// `astraweave_localsim::LocalSim` implements this once that crate is on the
/// same branch: `apply_writes` feeds `TickInput::writes`, and `field_digest`
/// is `LocalSim::field_digest`.
pub trait SimField {
    fn apply_writes(&mut self, writes: &[FieldWrite]);
    fn field_digest(&self) -> [u8; 32];
}

/// Accepts stamps and returns a zero digest. No cells.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullField;

impl SimField for NullField {
    fn apply_writes(&mut self, _writes: &[FieldWrite]) {}

    fn field_digest(&self) -> [u8; 32] {
        [0; 32]
    }
}

pub fn encode_write(out: &mut Vec<u8>, write: &FieldWrite) {
    out.extend_from_slice(&write.seq.to_le_bytes());
    out.push(shape_tag(write.shape));
    for axis in write.origin {
        out.extend_from_slice(&axis.to_le_bytes());
    }
    for axis in write.end {
        out.extend_from_slice(&axis.to_le_bytes());
    }
    out.push(write.radius);
    encode_op(out, &write.op);
}

fn shape_tag(shape: Shape) -> u8 {
    match shape {
        Shape::Point => 0,
        Shape::Sphere => 1,
        Shape::Segment => 2,
        Shape::Column => 3,
    }
}

fn encode_op(out: &mut Vec<u8>, op: &WriteOp) {
    match *op {
        WriteOp::AddTemp { delta, stack } => {
            out.push(1);
            out.extend_from_slice(&delta.to_le_bytes());
            out.push(u8::from(stack));
        }
        WriteOp::SetPhase { phase } => {
            out.push(2);
            out.push(phase);
        }
        WriteOp::AddDensity { delta, stack } => {
            out.push(3);
            out.push(delta as u8);
            out.push(u8::from(stack));
        }
        WriteOp::SetFlags { set, clear } => {
            out.push(4);
            out.push(set);
            out.push(clear);
        }
        WriteOp::SetGravity { index } => {
            out.push(5);
            out.extend_from_slice(&index.to_le_bytes());
        }
        WriteOp::AddVolume { delta } => {
            out.push(6);
            out.extend_from_slice(&delta.to_le_bytes());
        }
        WriteOp::SetWind { x, y, z } => {
            out.push(7);
            out.push(x as u8);
            out.push(y as u8);
            out.push(z as u8);
        }
    }
}

/// Records the writes from the last `apply_writes`. Not a grid. Test-only.
#[cfg(test)]
#[derive(Clone)]
pub struct TestSimField {
    writes: [FieldWrite; MAX_FIELD_WRITES],
    count: u8,
}

#[cfg(test)]
impl TestSimField {
    pub fn new() -> Self {
        Self {
            writes: [FieldWrite {
                seq: 0,
                shape: Shape::Point,
                origin: [0, 0, 0],
                end: [0, 0, 0],
                radius: 0,
                op: WriteOp::AddTemp {
                    delta: 0,
                    stack: false,
                },
            }; MAX_FIELD_WRITES],
            count: 0,
        }
    }

    pub fn count(&self) -> usize {
        self.count as usize
    }

    pub fn write(&self, index: usize) -> Option<FieldWrite> {
        if index < self.count as usize {
            Some(self.writes[index])
        } else {
            None
        }
    }
}

#[cfg(test)]
impl Default for TestSimField {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl SimField for TestSimField {
    fn apply_writes(&mut self, writes: &[FieldWrite]) {
        self.count = 0;
        for write in writes.iter().take(MAX_FIELD_WRITES) {
            let index = self.count as usize;
            self.writes[index] = *write;
            self.count = self.count.saturating_add(1);
        }
    }

    fn field_digest(&self) -> [u8; 32] {
        let mut raw = Vec::new();
        for index in 0..self.count as usize {
            encode_write(&mut raw, &self.writes[index]);
        }
        let mut digest = [0u8; 32];
        let n = raw.len().min(32);
        digest[..n].copy_from_slice(&raw[..n]);
        digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_conversion_matches_9810() {
        assert_eq!(gravity_mm_s2(WORLD_GRAVITY_MILLI_G), -9810);
        assert_eq!(gravity_mm_s2(0), 0);
    }

    #[test]
    fn null_field_digest_is_zero_and_stores_nothing() {
        let mut field = NullField;
        field.apply_writes(&[FieldWrite {
            seq: 1,
            shape: Shape::Point,
            origin: [0, 0, 0],
            end: [0, 0, 0],
            radius: 0,
            op: WriteOp::AddTemp {
                delta: 400,
                stack: false,
            },
        }]);
        assert_eq!(field.field_digest(), [0; 32]);
    }

    #[test]
    fn test_field_records_writes_without_cells() {
        let mut field = TestSimField::new();
        field.apply_writes(&[FieldWrite {
            seq: 3,
            shape: Shape::Sphere,
            origin: [1, 0, 0],
            end: [1, 0, 0],
            radius: 2,
            op: WriteOp::SetPhase { phase: PHASE_ICE },
        }]);
        assert_eq!(field.count(), 1);
        assert!(matches!(
            field.write(0).unwrap().op,
            WriteOp::SetPhase { phase: PHASE_ICE }
        ));
    }
}
