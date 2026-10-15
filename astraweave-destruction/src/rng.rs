//! One `SimRng` per destruction command. Extra ids fold into the system id.

use astraweave_core::{SimRng, SIM_SYSTEM_DESTRUCTION};

pub fn destruction_rng(world_seed: u64, tick: u64, entity_id: u64, seq: u64) -> SimRng {
    let system = SIM_SYSTEM_DESTRUCTION ^ entity_id.rotate_left(17) ^ seq.rotate_left(41);
    SimRng::new(world_seed, tick, system)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    #[test]
    fn the_same_tuple_draws_the_same_value() {
        let mut a = destruction_rng(3, 4, 5, 6);
        let mut b = destruction_rng(3, 4, 5, 6);
        assert_eq!(a.next_u64(), b.next_u64());
        assert_ne!(
            destruction_rng(3, 4, 5, 6).next_u64(),
            destruction_rng(3, 4, 5, 7).next_u64()
        );
    }
}
