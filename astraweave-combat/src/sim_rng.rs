//! Combat hit streams on the canonical [`astraweave_core::SimRng`].
//!
//! `main` at `85d09a2c` (PR #211) owns `mix_sim_seed` and `SimRng`. This
//! module does not copy them. `hit_rng` is the only extra fold: it builds a
//! `system_id` and then calls `SimRng::new`, which performs the one mix.
//!
//! The Q32.32 type is `astraweave_math::Fixed64` (PR #245), re-exported from
//! `fixed`. Moving the RNG did not change that arithmetic, so the golden
//! stage hash and the battle digest stay the same bytes.

pub use astraweave_core::{mix_sim_seed, SimRng, SIM_SYSTEM_COMBAT};

/// One stream per hit. Ordinal is the index in the sorted batch.
pub fn hit_rng(world_seed: u64, tick: u64, ordinal: u64, attack_id: u64) -> SimRng {
    let system = SIM_SYSTEM_COMBAT
        .wrapping_add(ordinal.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(attack_id.rotate_left(7));
    SimRng::new(world_seed, tick, system)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    #[test]
    fn reexport_matches_the_core_mix_vector() {
        assert_eq!(mix_sim_seed(1, 2, 3), 0x5991_344E_4904_5002);
    }

    #[test]
    fn streams_diverge_across_ordinal() {
        let mut a = hit_rng(1, 0, 0, 1);
        let mut b = hit_rng(1, 0, 1, 1);
        assert_ne!(a.next_u32(), b.next_u32());
    }
}
