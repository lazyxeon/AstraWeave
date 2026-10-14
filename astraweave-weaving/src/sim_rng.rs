//! Seeded simulation RNG for weaving.
//!
//! The mix matches `astraweave_core::mix_sim_seed`. Weaving does not depend on
//! `astraweave-core`, so the integer mix is duplicated here and locked by the
//! same `(1, 2, 3) -> 0x5991_344E_4904_5002` vector.

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

/// Anchor particle spawns. Same constant as `astraweave_core::SIM_SYSTEM_ANCHOR_PARTICLES`.
pub const SIM_SYSTEM_ANCHOR_PARTICLES: u64 = 0xAE11_7001;
/// Enemy patrol retargets. Same constant as `astraweave_core::SIM_SYSTEM_ENEMY_PATROL`.
pub const SIM_SYSTEM_ENEMY_PATROL: u64 = 0xAE11_7002;
/// Wave spawn position and archetype. Same constant as `astraweave_core::SIM_SYSTEM_SPAWNER`.
pub const SIM_SYSTEM_SPAWNER: u64 = 0xAE11_7003;

/// Mix `(world_seed, tick, system_id)` into one `StdRng` seed.
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

    /// `f32` in `[0, 1)`, a multiple of `2^-24`. No libm.
    pub fn unit_f32(&mut self) -> f32 {
        let u = self.next_u32() >> 8;
        (u as f32) * (1.0 / 16_777_216.0)
    }

    /// `f32` in `[-1, 1)`.
    pub fn signed_unit_f32(&mut self) -> f32 {
        self.unit_f32() * 2.0 - 1.0
    }

    /// XZ offset inside a disk of `radius`. Rejection from the centered unit
    /// square, then scale. Accepted points are uniform in the disk. No `sin`,
    /// `cos`, or `sqrt`: the reject test is a multiply. The fallback after 16
    /// rejects is a point on +X, still inside the disk.
    pub fn disk_xz(&mut self, radius: f32) -> (f32, f32) {
        for _ in 0..16 {
            let x = self.signed_unit_f32();
            let z = self.signed_unit_f32();
            if x * x + z * z <= 1.0 {
                return (x * radius, z * radius);
            }
        }
        (self.unit_f32() * radius, 0.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_matches_core_contract() {
        assert_eq!(mix_sim_seed(1, 2, 3), 0x5991_344E_4904_5002);
    }

    #[test]
    fn same_triple_same_bits() {
        let mut a = SimRng::new(9, 4, SIM_SYSTEM_ANCHOR_PARTICLES);
        let mut b = SimRng::new(9, 4, SIM_SYSTEM_ANCHOR_PARTICLES);
        assert_eq!(a.unit_f32().to_bits(), b.unit_f32().to_bits());
        assert_eq!(a.next_u32(), b.next_u32());
    }
}
