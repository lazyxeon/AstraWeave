//! Channels and activation types for the local field.
//!
//! [`FieldWrite`] and [`FieldEvent`] are the weaving crate's records.
//! This module adds the material row the solver reads and the solid query.

pub use astraweave_weaving::{
    FieldEvent, FieldWrite, Shape, WriteOp, FLAG_BURN, FLAG_CONDUCTIVE, FLAG_ICE, FLAG_SLIP,
    FLAG_WET, MAX_FIELD_WRITES, PHASE_BURN, PHASE_DRY, PHASE_GAS, PHASE_ICE, PHASE_OIL, PHASE_WET,
};

/// Regions allocated with the world. Not the number that tick.
pub const POOL_CAP: usize = 256;
/// Regions that run flow, gas, and heat on one tick.
pub const ACTIVE_BUDGET: usize = 32;
pub const REGION_EDGE: i32 = 16;
pub const REGION_CELLS: usize = 4096;
pub const SLEEP_CAP: usize = 1024;
pub const MAX_FIELD_EVENTS: usize = 128;
pub const MAX_HAZARDS: usize = 16;
pub const MAX_COLUMNS: usize = 4096;
pub const FLOW_ITERS: usize = 4;
pub const GAS_ITERS: usize = 4;
pub const HEAT_ITERS: usize = 4;
pub const VOLUME_MAX: u8 = 255;

pub use astraweave_core::SIM_SYSTEM_LOCAL_SIM;

/// Material ids weaving stamps. Phase is stored separately; these ids are the row key.
pub const MAT_AIR: u16 = 0;
pub const MAT_WATER: u16 = 1;
pub const MAT_ICE: u16 = 2;
pub const MAT_STEAM: u16 = 3;
pub const MAT_OIL: u16 = 4;
pub const MAT_FIRE: u16 = 5;
pub const MAT_WOOD: u16 = 6;
pub const MAT_METAL: u16 = 7;
pub const MAT_SMOKE: u16 = 8;
pub const MAT_POISON: u16 = 9;

/// Wood ignites at 300 deci-degrees. Freeze hold is -200, so ambient 0 stays liquid.
pub const WOOD_IGNITE: i16 = 300;
pub const FREEZE_HOLD: i16 = -200;

pub const TAG_FLAMMABLE: u16 = 1 << 0;
pub const TAG_CONDUCTIVE: u16 = 1 << 1;
pub const TAG_STRUCTURAL: u16 = 1 << 2;
pub const TAG_FREEZABLE: u16 = 1 << 3;
pub const TAG_WETTABLE: u16 = 1 << 4;

pub const EVENT_IGNITED: u8 = 0;
pub const EVENT_FROZE: u8 = 1;
pub const EVENT_CLEARED_BURN: u8 = 2;
pub const EVENT_SHOCKED: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subject {
    pub id: u64,
    pub cell: [i32; 3],
    /// Players are activated before AI when the budget is full.
    pub player: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialView {
    pub id: u16,
    pub tags: u16,
    pub ignite: i16,
    pub freeze: i16,
    pub density0: u16,
    pub conductivity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GravityView {
    pub index: u16,
    pub axis: u8,
    /// 0 world-down, 1 zero, 2 +X, 3 -X, 4 +Y, 5 -Y, 6 +Z, 7 -Z, 8 point.
    pub strength_milli_g: i16,
    pub center: [i32; 3],
    pub radius: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HazardSample {
    pub cell: [i32; 3],
    pub phase: u8,
    pub temp: i16,
    pub flags: u8,
}

/// One cell the host may copy into a GPU accent after the digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualCell {
    pub cell: [i32; 3],
    pub volume: u8,
    pub gas: u8,
    pub temp: i16,
    pub phase: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct TickInput<'a> {
    pub subjects: &'a [Subject],
    pub writes: &'a [FieldWrite],
    pub materials: &'a [MaterialView],
    pub zones: &'a [GravityView],
}

pub trait SolidField {
    fn solid(&self, x: i32, y: i32, z: i32) -> bool;
    fn material_id(&self, x: i32, y: i32, z: i32) -> u16;
}

/// Floor at `floor_y`. `hole` is one open cell in that floor (a breach).
#[derive(Clone, Copy, Debug)]
pub struct FloorSolid {
    pub floor_y: i32,
    pub hole: Option<[i32; 3]>,
    /// When set, the whole floor plane is open (a wide breach).
    pub open_floor: bool,
}

impl SolidField for FloorSolid {
    fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        if y != self.floor_y {
            return false;
        }
        if self.open_floor {
            return false;
        }
        match self.hole {
            Some([hx, hy, hz]) => !(x == hx && y == hy && z == hz),
            None => true,
        }
    }

    fn material_id(&self, _x: i32, y: i32, _z: i32) -> u16 {
        if y == self.floor_y {
            MAT_WOOD
        } else {
            MAT_AIR
        }
    }
}

impl FloorSolid {
    pub const fn closed(floor_y: i32) -> Self {
        Self {
            floor_y,
            hole: None,
            open_floor: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct SleepCell {
    pub index: u16,
    pub temp: i16,
    pub phase: u8,
    pub volume: u8,
    pub gas: u8,
    pub flags: u8,
    pub density_delta: i8,
    pub gravity: u16,
    pub wind: [i8; 3],
    pub material: u16,
}

impl SleepCell {
    pub const ZERO: Self = Self {
        index: 0,
        temp: 0,
        phase: 0,
        volume: 0,
        gas: 0,
        flags: 0,
        density_delta: 0,
        gravity: 0,
        wind: [0, 0, 0],
        material: 0,
    };
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SlotMeta {
    pub subject: u64,
    pub origin: [i32; 3],
    pub player: bool,
    pub occupied: bool,
    pub present: bool,
    pub active: bool,
    pub sleeping: bool,
    pub dense_sleep: bool,
    pub lively: bool,
    pub sleep_len: u16,
}

impl SlotMeta {
    pub const EMPTY: Self = Self {
        subject: 0,
        origin: [0, 0, 0],
        player: false,
        occupied: false,
        present: false,
        active: false,
        sleeping: false,
        dense_sleep: false,
        lively: false,
        sleep_len: 0,
    };
}

#[inline]
pub fn cell_index(x: i32, y: i32, z: i32) -> usize {
    ((z * REGION_EDGE + y) * REGION_EDGE + x) as usize
}

#[inline]
pub fn decode_index(i: usize) -> (i32, i32, i32) {
    let edge = REGION_EDGE as usize;
    let x = (i % edge) as i32;
    let y = ((i / edge) % edge) as i32;
    let z = (i / (edge * edge)) as i32;
    (x, y, z)
}

pub fn write_hits(w: &FieldWrite, world: [i32; 3]) -> bool {
    let dx = world[0] - w.origin[0];
    let dy = world[1] - w.origin[1];
    let dz = world[2] - w.origin[2];
    match w.shape {
        Shape::Point => dx == 0 && dy == 0 && dz == 0,
        Shape::Sphere => {
            let r = i64::from(w.radius);
            let d = i64::from(dx) * i64::from(dx)
                + i64::from(dy) * i64::from(dy)
                + i64::from(dz) * i64::from(dz);
            d <= r * r
        }
        Shape::Column => dx == 0 && dz == 0 && dy >= 0 && dy <= i32::from(w.radius),
        Shape::Segment => segment_hits(w, dx, dy, dz),
    }
}

fn segment_hits(w: &FieldWrite, dx: i32, dy: i32, dz: i32) -> bool {
    let abx = i64::from(w.end[0] - w.origin[0]);
    let aby = i64::from(w.end[1] - w.origin[1]);
    let abz = i64::from(w.end[2] - w.origin[2]);
    let apx = i64::from(dx);
    let apy = i64::from(dy);
    let apz = i64::from(dz);
    let ab2 = abx * abx + aby * aby + abz * abz;
    let r = i64::from(w.radius);
    if ab2 == 0 {
        return apx.abs() <= r && apy.abs() <= r && apz.abs() <= r;
    }
    let dot = apx * abx + apy * aby + apz * abz;
    let t = dot.clamp(0, ab2);
    let ox = apx * ab2 - abx * t;
    let oy = apy * ab2 - aby * t;
    let oz = apz * ab2 - abz * t;
    let limit = r * ab2;
    ox.abs() <= limit && oy.abs() <= limit && oz.abs() <= limit
}

pub fn material_of(materials: &[MaterialView], id: u16) -> Option<MaterialView> {
    materials
        .binary_search_by_key(&id, |m| m.id)
        .ok()
        .map(|i| materials[i])
}

pub fn sat_i16(v: i32) -> i16 {
    v.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

pub fn sat_u8_from_i32(v: i32) -> u8 {
    v.clamp(0, i32::from(VOLUME_MAX)) as u8
}

pub fn sat_i8(v: i32) -> i8 {
    v.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8
}
