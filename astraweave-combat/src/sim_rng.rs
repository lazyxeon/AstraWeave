//! Local `SimRng` seam.
//!
//! # Seam
//!
//! `astraweave_core::sim::SimRng` lives on `cursor/engine-ws-d2-stab` (PR #211)
//! and is not on `main`. This module copies the integer mix and the
//! `StdRng` (ChaCha12) wrapper so combat does not merge that branch.
//!
//! `mix_sim_seed(1, 2, 3)` is locked to `0x5991_344E_4904_5002`, the same
//! vector as core and as `astraweave-weaving` on that branch.
//!
//! When PR #211 lands, delete this file and call `astraweave_core::sim::SimRng`.
//! `hit_rng` must stay the only extra mix: `SimRng::new` already mixes
//! `(world_seed, tick, system_id)`. Do not mix a second time in the caller.
//!
//! This copy does not expose a float draw. Combat draws `u32` only.

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

/// Combat hit streams. Does not collide with the d2 system ids.
pub const SIM_SYSTEM_COMBAT: u64 = 0xC0BA_7001;

/// Mix `(world_seed, tick, system_id)` into one `StdRng` seed.
///
/// Integer-only. The same triple produces the same `u64` on every OS.
pub fn mix_sim_seed(world_seed: u64, tick: u64, system_id: u64) -> u64 {
    let mut z = world_seed
        .wrapping_add(0x9E37_79B9_7F4A_7C15)
        .wrapping_mul(tick ^ 0xBF58_476D_1CE4_E5B9)
        .wrapping_add(system_id.rotate_left(17));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Explicit sim stream. Draws are ChaCha12 via `StdRng`.
pub struct SimRng {
    inner: StdRng,
    world_seed: u64,
    tick: u64,
    system_id: u64,
}

impl SimRng {
    pub fn new(world_seed: u64, tick: u64, system_id: u64) -> Self {
        Self {
            inner: StdRng::seed_from_u64(mix_sim_seed(world_seed, tick, system_id)),
            world_seed,
            tick,
            system_id,
        }
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn system_id(&self) -> u64 {
        self.system_id
    }

    pub fn next_u32(&mut self) -> u32 {
        RngCore::next_u32(&mut self.inner)
    }
}

impl RngCore for SimRng {
    fn next_u32(&mut self) -> u32 {
        self.inner.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.inner.fill_bytes(dest)
    }
}

/// One stream per hit. Ordinal is the index in the sorted batch.
///
/// `SimRng::new` performs the only mix. `system_id` folds the combat id,
/// the ordinal, and the attack id so two hits in one tick do not share draws.
pub fn hit_rng(world_seed: u64, tick: u64, ordinal: u64, attack_id: u64) -> SimRng {
    let system = SIM_SYSTEM_COMBAT
        .wrapping_add(ordinal.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(attack_id.rotate_left(7));
    SimRng::new(world_seed, tick, system)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_matches_core_contract() {
        assert_eq!(mix_sim_seed(1, 2, 3), 0x5991_344E_4904_5002);
    }

    #[test]
    fn same_triple_same_bits() {
        let mut a = SimRng::new(9, 4, SIM_SYSTEM_COMBAT);
        let mut b = SimRng::new(9, 4, SIM_SYSTEM_COMBAT);
        assert_eq!(a.next_u32(), b.next_u32());
    }

    #[test]
    fn streams_diverge() {
        let a = SimRng::new(1, 0, SIM_SYSTEM_COMBAT).next_u32();
        let b = hit_rng(1, 0, 0, 1).next_u32();
        let c = hit_rng(1, 0, 1, 1).next_u32();
        assert_ne!(b, c);
        let _ = a;
    }
}
