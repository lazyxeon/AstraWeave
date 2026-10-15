//! Interface the local-sim grid owns. This crate writes [`SimModifier`] records.
//! It does not step liquids, gases, or heat conduction.
//!
//! Channel schema agreed with `feat/local-sim` (`SimModifier` / cell bytes):
//! temperature is their `heat` (`i16`, 0 = ambient), phase is derived from
//! material id, density is kg/m³, surface flags are derived from that material,
//! gravity is `GravityDelta` in milli-g. Cell size is 500 mm.
//!
//! `TestSimField` is the stand-in until `astraweave-localsim` exists. It stamps
//! modifiers onto a fixed window and runs one reaction per cell. It is not the
//! open-world solver.

use serde::{Deserialize, Serialize};

/// Millimetres along one cell edge. Matches the local-sim design.
pub const CELL_MM: i32 = 500;
/// Modifier cap. Extra pushes are rejected and do not enter the hash.
pub const MAX_MODIFIERS: usize = 32;
/// World gravity as milli-g. One g is 1000. Down is negative.
pub const WORLD_GRAVITY_MILLI_G: i32 = -1000;

pub const MAT_AIR: u16 = 0;
pub const MAT_WATER: u16 = 1;
pub const MAT_ICE: u16 = 2;
pub const MAT_STEAM: u16 = 3;
pub const MAT_OIL: u16 = 4;
pub const MAT_FIRE: u16 = 5;
pub const MAT_WOOD: u16 = 6;
pub const MAT_STONE: u16 = 7;
pub const MAT_SMOKE: u16 = 8;
pub const MAT_POISON: u16 = 9;

pub const PHASE_DRY: u8 = 0;
pub const PHASE_WET: u8 = 1;
pub const PHASE_ICE: u8 = 2;
pub const PHASE_BURN: u8 = 3;
pub const PHASE_GAS: u8 = 4;
pub const PHASE_OIL: u8 = 5;

pub const FLAG_WET: u8 = 1 << 0;
pub const FLAG_ICE: u8 = 1 << 1;
pub const FLAG_BURN: u8 = 1 << 2;
pub const FLAG_SLIPPERY: u8 = 1 << 3;
pub const FLAG_CONDUCTIVE: u8 = 1 << 4;

/// `milli_g` is world milli-g plus a `GravityDelta`. Result is mm/s².
/// `-1000` milli-g (one g down) converts to `-9810`, which is `-9.81 m/s²`.
pub fn gravity_mm_s2(milli_g: i32) -> i32 {
    milli_g.saturating_mul(981) / 100
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ModifierKind {
    HeatSource = 1,
    Wind = 2,
    GravityDelta = 3,
    MaterialStamp = 4,
    FillStamp = 5,
    DensityScale = 6,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimModifier {
    pub id: u32,
    pub kind: ModifierKind,
    /// Cell coordinates.
    pub origin: [i32; 3],
    pub radius: u16,
    pub a: i16,
    pub b: i16,
    pub c: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldSample {
    pub temperature: i16,
    pub phase: u8,
    pub density: u16,
    pub flags: u8,
    /// Milli-g added to [`WORLD_GRAVITY_MILLI_G`].
    pub gravity_delta: i16,
    pub material: u16,
}

impl Default for FieldSample {
    fn default() -> Self {
        Self {
            temperature: 0,
            phase: PHASE_DRY,
            density: 0,
            flags: 0,
            gravity_delta: 0,
            material: MAT_AIR,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierReject {
    Full,
}

/// What weaving is allowed to do to the grid. The sim applies the records.
pub trait SimField {
    fn begin_tick(&mut self);
    fn try_push(&mut self, modifier: SimModifier) -> Result<(), ModifierReject>;
    fn modifier_count(&self) -> usize;
    fn modifier(&self, index: usize) -> Option<SimModifier>;
    fn sample(&self, cell: [i32; 3]) -> FieldSample;
}

#[derive(Clone, Copy)]
struct Cell {
    temperature: i16,
    material: u16,
    density: u16,
    gravity_delta: i16,
    fill: u8,
    reacted: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            temperature: 0,
            material: MAT_STONE,
            density: 2600,
            gravity_delta: 0,
            fill: 255,
            reacted: 0,
        }
    }
}

const WINDOW: i32 = 16;
const WINDOW_ORIGIN: i32 = -8;

/// Fixed 16³ window. Allocated with the value. `stamp_and_react` does not allocate.
#[derive(Clone)]
pub struct TestSimField {
    cells: Box<[Cell]>,
    modifiers: [SimModifier; MAX_MODIFIERS],
    modifier_count: u8,
    wind: [i16; 3],
    next_id: u32,
}

impl TestSimField {
    pub fn new() -> Self {
        Self {
            cells: vec![Cell::default(); (WINDOW * WINDOW * WINDOW) as usize].into_boxed_slice(),
            modifiers: [SimModifier {
                id: 0,
                kind: ModifierKind::HeatSource,
                origin: [0, 0, 0],
                radius: 0,
                a: 0,
                b: 0,
                c: 0,
            }; MAX_MODIFIERS],
            modifier_count: 0,
            wind: [0, 0, 0],
            next_id: 1,
        }
    }

    pub fn wind(&self) -> [i16; 3] {
        self.wind
    }

    pub fn assign_id(&mut self, modifier: &mut SimModifier) {
        if modifier.id == 0 {
            modifier.id = self.next_id;
            self.next_id = self.next_id.saturating_add(1);
        }
    }

    /// One stamp in modifier order, then one reaction per cell in index order.
    pub fn stamp_and_react(&mut self) {
        for cell in self.cells.iter_mut() {
            cell.temperature = 0;
            cell.gravity_delta = 0;
            cell.reacted = 0;
        }
        let count = self.modifier_count as usize;
        let staged = self.modifiers;
        for modifier in staged.into_iter().take(count) {
            self.stamp(modifier);
        }
        let len = self.cells.len();
        for index in 0..len {
            if self.cells[index].reacted != 0 {
                continue;
            }
            let cell = self.cells[index];
            if cell.material == MAT_WOOD && cell.temperature >= 300 {
                self.cells[index].material = MAT_FIRE;
                self.cells[index].reacted = 1;
            } else if cell.material == MAT_WATER && cell.temperature <= -100 {
                self.cells[index].material = MAT_ICE;
                self.cells[index].reacted = 1;
            } else if cell.material == MAT_OIL && cell.temperature >= 300 {
                self.cells[index].material = MAT_FIRE;
                self.cells[index].reacted = 1;
            }
        }
    }

    fn stamp(&mut self, modifier: SimModifier) {
        match modifier.kind {
            ModifierKind::Wind => {
                self.wind = [modifier.a, modifier.b, modifier.c];
                return;
            }
            ModifierKind::HeatSource
            | ModifierKind::GravityDelta
            | ModifierKind::MaterialStamp
            | ModifierKind::FillStamp
            | ModifierKind::DensityScale => {}
        }
        let radius = modifier.radius as i32;
        let [ox, oy, oz] = modifier.origin;
        for z in (oz - radius)..=(oz + radius) {
            for y in (oy - radius)..=(oy + radius) {
                for x in (ox - radius)..=(ox + radius) {
                    let dx = (x - ox).abs();
                    let dy = (y - oy).abs();
                    let dz = (z - oz).abs();
                    if dx.max(dy).max(dz) > radius {
                        continue;
                    }
                    let Some(index) = index_of(x, y, z) else {
                        continue;
                    };
                    let cell = &mut self.cells[index];
                    match modifier.kind {
                        ModifierKind::HeatSource => {
                            cell.temperature = cell.temperature.saturating_add(modifier.a);
                        }
                        ModifierKind::GravityDelta => {
                            cell.gravity_delta = modifier.a;
                        }
                        ModifierKind::MaterialStamp => {
                            cell.material = modifier.a.max(0) as u16;
                        }
                        ModifierKind::FillStamp => {
                            cell.material = modifier.a.max(0) as u16;
                            cell.fill = modifier.b.clamp(0, 255) as u8;
                        }
                        ModifierKind::DensityScale => {
                            cell.density = modifier.a.max(0) as u16;
                        }
                        ModifierKind::Wind => {}
                    }
                }
            }
        }
    }
}

impl Default for TestSimField {
    fn default() -> Self {
        Self::new()
    }
}

fn index_of(x: i32, y: i32, z: i32) -> Option<usize> {
    let lx = x - WINDOW_ORIGIN;
    let ly = y - WINDOW_ORIGIN;
    let lz = z - WINDOW_ORIGIN;
    if (0..WINDOW).contains(&lx) && (0..WINDOW).contains(&ly) && (0..WINDOW).contains(&lz) {
        Some(((lz * WINDOW + ly) * WINDOW + lx) as usize)
    } else {
        None
    }
}

pub fn phase_of(material: u16) -> u8 {
    match material {
        MAT_WATER => PHASE_WET,
        MAT_ICE => PHASE_ICE,
        MAT_FIRE => PHASE_BURN,
        MAT_STEAM | MAT_SMOKE | MAT_POISON => PHASE_GAS,
        MAT_OIL => PHASE_OIL,
        _ => PHASE_DRY,
    }
}

pub fn flags_of(material: u16) -> u8 {
    match material {
        MAT_WATER => FLAG_WET | FLAG_CONDUCTIVE,
        MAT_ICE => FLAG_ICE | FLAG_SLIPPERY,
        MAT_FIRE => FLAG_BURN,
        MAT_OIL => FLAG_SLIPPERY,
        _ => 0,
    }
}

impl SimField for TestSimField {
    fn begin_tick(&mut self) {
        self.modifier_count = 0;
        self.wind = [0, 0, 0];
    }

    fn try_push(&mut self, modifier: SimModifier) -> Result<(), ModifierReject> {
        let index = self.modifier_count as usize;
        if index >= MAX_MODIFIERS {
            return Err(ModifierReject::Full);
        }
        self.modifiers[index] = modifier;
        self.modifier_count = self.modifier_count.saturating_add(1);
        Ok(())
    }

    fn modifier_count(&self) -> usize {
        self.modifier_count as usize
    }

    fn modifier(&self, index: usize) -> Option<SimModifier> {
        if index < self.modifier_count as usize {
            Some(self.modifiers[index])
        } else {
            None
        }
    }

    fn sample(&self, cell: [i32; 3]) -> FieldSample {
        let Some(index) = index_of(cell[0], cell[1], cell[2]) else {
            return FieldSample::default();
        };
        let cell = self.cells[index];
        FieldSample {
            temperature: cell.temperature,
            phase: phase_of(cell.material),
            density: cell.density,
            flags: flags_of(cell.material),
            gravity_delta: cell.gravity_delta,
            material: cell.material,
        }
    }
}

/// Nearest cell index from a `f32` metre position. Truncates toward zero after
/// scaling to millimetres. The replay path does not call this.
pub fn meters_to_cell(m: f32) -> i32 {
    let milli = (m * 1000.0) as i32;
    milli.div_euclid(CELL_MM)
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
    fn heat_then_wood_becomes_fire_once() {
        let mut field = TestSimField::new();
        field
            .try_push(SimModifier {
                id: 1,
                kind: ModifierKind::MaterialStamp,
                origin: [0, 0, 0],
                radius: 0,
                a: MAT_WOOD as i16,
                b: 0,
                c: 0,
            })
            .unwrap();
        field
            .try_push(SimModifier {
                id: 2,
                kind: ModifierKind::HeatSource,
                origin: [0, 0, 0],
                radius: 0,
                a: 400,
                b: 0,
                c: 0,
            })
            .unwrap();
        field.stamp_and_react();
        let sample = field.sample([0, 0, 0]);
        assert_eq!(sample.phase, PHASE_BURN);
        assert_eq!(sample.flags & FLAG_BURN, FLAG_BURN);
        field.stamp_and_react();
        assert_eq!(field.sample([0, 0, 0]).material, MAT_FIRE);
    }

    #[test]
    fn full_modifier_list_rejects() {
        let mut field = TestSimField::new();
        for id in 0..MAX_MODIFIERS {
            field
                .try_push(SimModifier {
                    id: id as u32 + 1,
                    kind: ModifierKind::Wind,
                    origin: [0, 0, 0],
                    radius: 0,
                    a: 0,
                    b: 0,
                    c: 1,
                })
                .unwrap();
        }
        let rejected = field.try_push(SimModifier {
            id: 99,
            kind: ModifierKind::Wind,
            origin: [0, 0, 0],
            radius: 0,
            a: 0,
            b: 0,
            c: 1,
        });
        assert_eq!(rejected, Err(ModifierReject::Full));
    }
}
