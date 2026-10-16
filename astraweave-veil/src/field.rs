//! Sparse world-space scar field.
//!
//! A death adds transient energy on a cell and its six face neighbors.
//! Each tick moves a fixed slice of that transient into the permanent floor
//! and drops the rest. The floor never falls. Cells live in a `BTreeMap`,
//! so iteration order is the key order.

use std::collections::BTreeMap;

use crate::fixed::Fixed64;

/// Edge of one cell, in whole metres.
pub const CELL_METRES: i32 = 4;
/// Integer tension injected at the death cell.
pub const DEATH_ENERGY: i64 = 48;
/// Permanent-looking tension that makes an agent wary, in whole units.
pub const WARY_TENSION: i64 = 12;
/// Tension that makes an agent leave, in whole units.
pub const AVOID_TENSION: i64 = 28;
/// Normalization peak for the GPU weight. Matches [`DEATH_ENERGY`].
pub const PEAK_TENSION: i64 = DEATH_ENERGY;
/// Scar window edge, in cells. 32³ RGBA16F is 256 KiB.
pub const GPU_DIM: u32 = 32;

/// Six face neighbors, then the home cell is written separately.
const FACES: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// One sparse cell. `effective = floor + transient`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScarCell {
    floor: Fixed64,
    transient: Fixed64,
}

impl Default for ScarCell {
    fn default() -> Self {
        Self {
            floor: Fixed64::ZERO,
            transient: Fixed64::ZERO,
        }
    }
}

impl ScarCell {
    pub fn floor(self) -> Fixed64 {
        self.floor
    }

    pub fn transient(self) -> Fixed64 {
        self.transient
    }

    pub fn effective(self) -> Fixed64 {
        self.floor.saturating_add(self.transient)
    }
}

/// Deterministic spatial memory of deaths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScarField {
    cell_metres: i32,
    decay_per_tick: Fixed64,
    retain_num: i64,
    retain_den: i64,
    wary_at: Fixed64,
    avoid_at: Fixed64,
    ticks: u64,
    cells: BTreeMap<[i32; 3], ScarCell>,
}

impl Default for ScarField {
    fn default() -> Self {
        Self::new()
    }
}

impl ScarField {
    pub fn new() -> Self {
        Self {
            cell_metres: CELL_METRES,
            // 1/4 unit per tick. A 48-unit surge lasts 192 ticks.
            decay_per_tick: Fixed64::from_ratio(1, 4),
            retain_num: 1,
            retain_den: 3,
            wary_at: Fixed64::from_int(WARY_TENSION),
            avoid_at: Fixed64::from_int(AVOID_TENSION),
            ticks: 0,
            cells: BTreeMap::new(),
        }
    }

    pub(crate) fn from_parts(
        cell_metres: i32,
        decay_per_tick: Fixed64,
        retain_num: i64,
        retain_den: i64,
        wary_at: Fixed64,
        avoid_at: Fixed64,
        ticks: u64,
    ) -> Self {
        Self {
            cell_metres,
            decay_per_tick,
            retain_num,
            retain_den,
            wary_at,
            avoid_at,
            ticks,
            cells: BTreeMap::new(),
        }
    }

    pub(crate) fn insert_loaded(&mut self, cell: [i32; 3], floor: Fixed64, transient: Fixed64) {
        if floor.raw() <= 0 && transient.raw() <= 0 {
            return;
        }
        self.cells.insert(cell, ScarCell { floor, transient });
    }

    pub fn cell_metres(&self) -> i32 {
        self.cell_metres
    }

    pub fn decay_per_tick(&self) -> Fixed64 {
        self.decay_per_tick
    }

    pub fn retain_num(&self) -> i64 {
        self.retain_num
    }

    pub fn retain_den(&self) -> i64 {
        self.retain_den
    }

    pub fn wary_at(&self) -> Fixed64 {
        self.wary_at
    }

    pub fn avoid_at(&self) -> Fixed64 {
        self.avoid_at
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn occupied_count(&self) -> usize {
        self.cells.len()
    }

    pub(crate) fn cells(&self) -> impl Iterator<Item = ([i32; 3], ScarCell)> + '_ {
        self.cells.iter().map(|(key, cell)| (*key, *cell))
    }

    pub(crate) fn cell_of_metres(&self, metres: [i32; 3]) -> [i32; 3] {
        [
            metres[0].div_euclid(self.cell_metres),
            metres[1].div_euclid(self.cell_metres),
            metres[2].div_euclid(self.cell_metres),
        ]
    }

    pub(crate) fn effective_at_cell(&self, cell: [i32; 3]) -> Fixed64 {
        self.cells
            .get(&cell)
            .map(|cell| cell.effective())
            .unwrap_or(Fixed64::ZERO)
    }

    /// Standard death: [`DEATH_ENERGY`] on the cell, one quarter on each face.
    pub fn inject_death(&mut self, x: i32, y: i32, z: i32) {
        self.inject_energy(x, y, z, Fixed64::from_int(DEATH_ENERGY));
    }

    /// Add `energy` at the cell that contains these whole metres, and a
    /// quarter of it on each of the six face neighbors.
    pub fn inject_energy(&mut self, x: i32, y: i32, z: i32, energy: Fixed64) {
        if energy.raw() <= 0 {
            return;
        }
        let home = self.cell_of_metres([x, y, z]);
        self.add_transient(home, energy);
        let splash = energy.mul_ratio(1, 4);
        for face in FACES {
            self.add_transient(
                [home[0] + face[0], home[1] + face[1], home[2] + face[2]],
                splash,
            );
        }
    }

    fn add_transient(&mut self, cell: [i32; 3], energy: Fixed64) {
        if energy.raw() <= 0 {
            return;
        }
        let entry = self.cells.entry(cell).or_default();
        entry.transient = entry.transient.saturating_add(energy);
    }

    /// One simulation step. Not a wall-clock interval.
    pub fn tick(&mut self) {
        self.ticks = self.ticks.saturating_add(1);
        let decay = self.decay_per_tick;
        let num = self.retain_num;
        let den = self.retain_den;
        for cell in self.cells.values_mut() {
            if cell.transient.raw() <= 0 {
                continue;
            }
            let step = cell.transient.min(decay);
            let retained = step.mul_ratio(num, den);
            cell.floor = cell.floor.saturating_add(retained);
            cell.transient = cell.transient.saturating_sub(step);
        }
    }

    pub fn cell_at(&self, x: i32, y: i32, z: i32) -> Option<ScarCell> {
        let cell = self.cell_of_metres([x, y, z]);
        self.cells.get(&cell).copied()
    }

    /// FNV-1a 64 of the canonical little-endian encoding.
    pub fn state_hash(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        fn feed(hash: &mut u64, bytes: &[u8]) {
            for byte in bytes {
                *hash ^= u64::from(*byte);
                *hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        feed(&mut hash, &self.cell_metres.to_le_bytes());
        feed(&mut hash, &self.decay_per_tick.raw().to_le_bytes());
        feed(&mut hash, &self.retain_num.to_le_bytes());
        feed(&mut hash, &self.retain_den.to_le_bytes());
        feed(&mut hash, &self.wary_at.raw().to_le_bytes());
        feed(&mut hash, &self.avoid_at.raw().to_le_bytes());
        feed(&mut hash, &self.ticks.to_le_bytes());
        feed(&mut hash, &(self.cells.len() as u64).to_le_bytes());
        for (key, cell) in &self.cells {
            feed(&mut hash, &key[0].to_le_bytes());
            feed(&mut hash, &key[1].to_le_bytes());
            feed(&mut hash, &key[2].to_le_bytes());
            feed(&mut hash, &cell.floor.raw().to_le_bytes());
            feed(&mut hash, &cell.transient.raw().to_le_bytes());
        }
        hash
    }
}

/// Scripted deaths used by the cross-run hash. Two calls return equal fields.
#[cfg(test)]
pub(crate) fn reference_history() -> ScarField {
    let mut field = ScarField::new();
    field.inject_energy(12, 4, -8, Fixed64::from_int(48));
    for _ in 0..10 {
        field.tick();
    }
    field.inject_energy(-4, 4, 12, Fixed64::from_int(48));
    field.inject_energy(12, 4, -8, Fixed64::from_int(16));
    for _ in 0..30 {
        field.tick();
    }
    field
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_metres_use_euclid_cells() {
        let field = ScarField::new();
        assert_eq!(field.cell_of_metres([0, 0, 0]), [0, 0, 0]);
        assert_eq!(field.cell_of_metres([3, 0, 0]), [0, 0, 0]);
        assert_eq!(field.cell_of_metres([4, 0, 0]), [1, 0, 0]);
        assert_eq!(field.cell_of_metres([-1, 0, 0]), [-1, 0, 0]);
        assert_eq!(field.cell_of_metres([-4, 0, 0]), [-1, 0, 0]);
        assert_eq!(field.cell_of_metres([-5, 0, 0]), [-2, 0, 0]);
    }

    #[test]
    fn inject_order_does_not_change_the_hash() {
        let mut ab = ScarField::new();
        ab.inject_death(10, 2, -6);
        ab.inject_death(-3, 2, 9);
        let mut ba = ScarField::new();
        ba.inject_death(-3, 2, 9);
        ba.inject_death(10, 2, -6);
        assert_eq!(ab.state_hash(), ba.state_hash());
        assert_eq!(ab, ba);
    }

    #[test]
    fn transient_decays_onto_a_raised_floor() {
        let mut field = ScarField::new();
        field.inject_death(0, 0, 0);
        let peak = field.cell_at(0, 0, 0).unwrap().effective();
        assert!(peak >= Fixed64::from_int(AVOID_TENSION));
        for _ in 0..400 {
            field.tick();
        }
        let settled = field.cell_at(0, 0, 0).unwrap();
        assert_eq!(settled.transient(), Fixed64::ZERO);
        assert!(settled.floor().raw() > 0);
        assert!(settled.floor() < peak);
        assert!(settled.floor() >= Fixed64::from_int(WARY_TENSION));
        assert!(settled.effective() < Fixed64::from_int(AVOID_TENSION));
        let hash = field.state_hash();
        field.tick();
        // Another tick still advances the counter, so the hash changes, and
        // the floor does not.
        assert_ne!(field.state_hash(), hash);
        assert_eq!(field.cell_at(0, 0, 0).unwrap().floor(), settled.floor());
    }

    fn fold(hash: u64, bytes: &[u8]) -> u64 {
        let mut hash = hash;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    fn history_fingerprint(field: &ScarField) -> u64 {
        let mut hash = field.state_hash();
        for metres in [[12_i32, 4, -8], [-4, 4, 12], [400, 0, 400]] {
            let percept = field.perceive_metres(metres[0], metres[1], metres[2]);
            hash = fold(hash, &percept.tension.raw().to_le_bytes());
            hash = fold(hash, &[u8::from(percept.wary), u8::from(percept.avoid)]);
            hash = fold(hash, &percept.retreat[0].to_le_bytes());
            hash = fold(hash, &percept.retreat[1].to_le_bytes());
            hash = fold(hash, &percept.retreat[2].to_le_bytes());
        }
        hash
    }

    #[test]
    fn cross_run_reference_history_matches_the_golden_hash() {
        // Locked from a previous run. Covers the field and three percepts.
        // A second call in this process must match, and so must any later
        // process or machine: the inputs are integer and the hash is
        // little-endian FNV-1a.
        const GOLDEN: u64 = 0xb1cc29cc42401e83;
        let first = history_fingerprint(&reference_history());
        let second = history_fingerprint(&reference_history());
        assert_eq!(first, second);
        assert_eq!(first, GOLDEN, "cross-run scar hash drifted: {first:#018x}");
    }

    #[test]
    fn this_module_keeps_floats_and_clocks_out_of_the_sim() {
        let src = include_str!("field.rs");
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);
        assert!(!code.contains("f32"));
        assert!(!code.contains("f64"));
        assert!(!code.contains("HashMap"));
        assert!(!code.contains("SystemTime"));
        assert!(!code.contains("Instant"));
    }
}
