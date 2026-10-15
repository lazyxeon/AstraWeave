use crate::World;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

/// Crafting item ids. Mixed into [`SimRng`] with the world seed and tick.
pub const SIM_SYSTEM_CRAFT: u64 = 0xC2AF_7001;
/// Harvest respawn amounts.
pub const SIM_SYSTEM_HARVEST: u64 = 0xC2AF_7002;
/// Weaving anchor particle spawns.
pub const SIM_SYSTEM_ANCHOR_PARTICLES: u64 = 0xAE11_7001;
/// Enemy patrol retargets. Weaving duplicates this constant; it does not depend on core.
pub const SIM_SYSTEM_ENEMY_PATROL: u64 = 0xAE11_7002;
/// Enemy wave spawn position and archetype. Weaving duplicates this constant.
pub const SIM_SYSTEM_SPAWNER: u64 = 0xAE11_7003;
/// Terrain solver distance rolls. The solver stores the mixed `u64`.
pub const SIM_SYSTEM_TERRAIN_SOLVER: u64 = 0x7E22_7001;
/// Combat hit streams. `astraweave-combat` folds ordinal and attack id into this.
pub const SIM_SYSTEM_COMBAT: u64 = 0xC0BA_7001;
/// Destruction fracture rolls. `astraweave-destruction` mixes this with the world seed.
pub const SIM_SYSTEM_DESTRUCTION: u64 = 0xD357_7001;

/// Mix `(world_seed, tick, system_id)` into one `StdRng` seed.
///
/// Integer-only. The same triple produces the same `u64` on every OS and
/// architecture. This is not `DefaultHasher` (that algorithm is not stable
/// across rustc versions).
pub fn mix_sim_seed(world_seed: u64, tick: u64, system_id: u64) -> u64 {
    let mut z = world_seed
        .wrapping_add(0x9E37_79B9_7F4A_7C15)
        .wrapping_mul(tick ^ 0xBF58_476D_1CE4_E5B9)
        .wrapping_add(system_id.rotate_left(17));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Simulation RNG. One stream per `(world_seed, tick, system_id)`.
///
/// Draws are `StdRng` (ChaCha12). Reseed from [`SimRng::new`] at the start of
/// a tick so the sequence does not depend on how many values the previous tick
/// consumed. Float draws that must be bit-identical across platforms should
/// use [`SimRng::unit_f32`], which is a dyadic rational, not libm.
pub struct SimRng {
    inner: StdRng,
    world_seed: u64,
    tick: u64,
    system_id: u64,
}

impl SimRng {
    pub fn new(world_seed: u64, tick: u64, system_id: u64) -> Self {
        let mixed = mix_sim_seed(world_seed, tick, system_id);
        Self {
            inner: StdRng::seed_from_u64(mixed),
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

    /// `f32` in `[0, 1)` from the top 24 bits. Every value is a multiple of
    /// `2^-24`, so the bit pattern does not depend on libm or on `rand`'s
    /// float distribution.
    pub fn unit_f32(&mut self) -> f32 {
        let u = self.next_u32() >> 8;
        (u as f32) * (1.0 / 16_777_216.0)
    }

    /// `f32` in `[-1, 1)`.
    pub fn signed_unit_f32(&mut self) -> f32 {
        self.unit_f32() * 2.0 - 1.0
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

pub struct SimConfig {
    pub dt: f32,
}

pub fn step(w: &mut World, cfg: &SimConfig) {
    w.tick(cfg.dt);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sim_config_creation() {
        let cfg = SimConfig { dt: 0.016 };
        assert_eq!(cfg.dt, 0.016);
    }

    #[test]
    fn test_sim_config_different_dt() {
        let cfg1 = SimConfig { dt: 0.016 };
        let cfg2 = SimConfig { dt: 0.033 };
        assert_eq!(cfg1.dt, 0.016);
        assert_eq!(cfg2.dt, 0.033);
    }

    #[test]
    fn test_step_doesnt_crash() {
        let mut world = World::new();
        let cfg = SimConfig { dt: 0.016 };
        step(&mut world, &cfg); // Should not crash
    }

    #[test]
    fn test_step_multiple_times() {
        let mut world = World::new();
        let cfg = SimConfig { dt: 0.016 };
        // Run 10 ticks without crashing
        for _ in 0..10 {
            step(&mut world, &cfg);
        }
    }

    #[test]
    fn test_step_with_different_dt_values() {
        let mut world = World::new();

        step(&mut world, &SimConfig { dt: 0.016 });
        step(&mut world, &SimConfig { dt: 0.033 });
        step(&mut world, &SimConfig { dt: 0.008 });
        // Should not crash with varying dt
    }

    #[test]
    fn sim_rng_same_triple_is_bit_identical() {
        let mut a = SimRng::new(0xA11C, 7, SIM_SYSTEM_CRAFT);
        let mut b = SimRng::new(0xA11C, 7, SIM_SYSTEM_CRAFT);
        for _ in 0..32 {
            assert_eq!(a.next_u32(), b.next_u32());
            assert_eq!(a.unit_f32().to_bits(), b.unit_f32().to_bits());
        }
    }

    #[test]
    fn sim_rng_streams_diverge_across_system_and_tick() {
        let craft = SimRng::new(1, 0, SIM_SYSTEM_CRAFT).next_u64();
        let harvest = SimRng::new(1, 0, SIM_SYSTEM_HARVEST).next_u64();
        let next_tick = SimRng::new(1, 1, SIM_SYSTEM_CRAFT).next_u64();
        assert_ne!(craft, harvest);
        assert_ne!(craft, next_tick);
    }

    #[test]
    fn mix_sim_seed_locked() {
        assert_eq!(mix_sim_seed(1, 2, 3), 0x5991_344E_4904_5002);
    }
}
