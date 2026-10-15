//! Fixed debris pool. Steady state does not allocate.

use astraweave_math::Fixed64;

use crate::budget::{MAX_SIM_DEBRIS, SLEEP_TICKS, TICK_HZ};
use crate::graph::{dist2, SupportNode};
use crate::terrain::Terrain;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DebrisLod {
    Rigid = 1,
    Proxy = 2,
    Merged = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DebrisSlot {
    pub occupied: bool,
    pub id: u64,
    pub source_node: u64,
    pub born_tick: u64,
    pub pos: [Fixed64; 3],
    pub vel: [Fixed64; 3],
    pub ang: [Fixed64; 3],
    pub half: [Fixed64; 3],
    pub sleep_held: u16,
    pub age: u16,
    pub lod: DebrisLod,
}

impl DebrisSlot {
    fn empty() -> Self {
        Self {
            occupied: false,
            id: 0,
            source_node: 0,
            born_tick: 0,
            pos: [Fixed64::ZERO; 3],
            vel: [Fixed64::ZERO; 3],
            ang: [Fixed64::ZERO; 3],
            half: [Fixed64::from_int(1); 3],
            sleep_held: 0,
            age: 0,
            lod: DebrisLod::Rigid,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DebrisPool {
    slots: [DebrisSlot; MAX_SIM_DEBRIS],
    next_id: u64,
}

impl DebrisPool {
    pub fn new() -> Self {
        Self {
            slots: [DebrisSlot::empty(); MAX_SIM_DEBRIS],
            next_id: 1,
        }
    }

    pub fn slots(&self) -> &[DebrisSlot; MAX_SIM_DEBRIS] {
        &self.slots
    }

    pub fn occupied_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.occupied).count()
    }

    pub fn spawn(&mut self, node: &SupportNode, tick: u64) {
        if self
            .slots
            .iter()
            .any(|slot| slot.occupied && slot.source_node == node.id)
        {
            return;
        }
        let index = self.free_index().unwrap_or_else(|| self.evict());
        let id = self.next_id;
        self.next_id += 1;
        self.slots[index] = DebrisSlot {
            occupied: true,
            id,
            source_node: node.id,
            born_tick: tick,
            pos: node.pos,
            vel: [Fixed64::ZERO; 3],
            ang: [Fixed64::ZERO; 3],
            half: node.half,
            sleep_held: 0,
            age: 0,
            lod: DebrisLod::Rigid,
        };
    }

    pub fn step(&mut self, terrain: &Terrain) {
        let gravity = Fixed64::from_ratio(-981, 100).mul_ratio(1, TICK_HZ);
        let sleep_sq = Fixed64::from_ratio(1, 50).mul(Fixed64::from_ratio(1, 50));
        for slot in &mut self.slots {
            if !slot.occupied || slot.lod == DebrisLod::Merged {
                continue;
            }
            slot.age = slot.age.saturating_add(1);
            if slot.lod == DebrisLod::Rigid {
                slot.vel[1] = slot.vel[1].saturating_add(gravity);
            }
            for axis in 0..3 {
                slot.pos[axis] =
                    slot.pos[axis].saturating_add(slot.vel[axis].mul_ratio(1, TICK_HZ));
            }
            let column_x = whole(slot.pos[0]);
            let column_z = whole(slot.pos[2]);
            let ground = Fixed64::from_int(i64::from(terrain.surface_y(column_x, column_z)))
                .saturating_add(slot.half[1]);
            if slot.pos[1] < ground {
                slot.pos[1] = ground;
                if slot.vel[1].raw() < 0 {
                    slot.vel[1] = Fixed64::ZERO;
                }
            }
            if slot.age > 120 && slot.lod == DebrisLod::Rigid {
                slot.lod = DebrisLod::Proxy;
                slot.ang = [Fixed64::ZERO; 3];
            }
            let speed = dist2(slot.vel, [Fixed64::ZERO; 3]);
            let spin = dist2(slot.ang, [Fixed64::ZERO; 3]);
            if speed <= sleep_sq && spin <= sleep_sq {
                slot.sleep_held = slot.sleep_held.saturating_add(1);
            } else {
                slot.sleep_held = 0;
            }
            if slot.sleep_held >= SLEEP_TICKS {
                slot.lod = DebrisLod::Merged;
                slot.vel = [Fixed64::ZERO; 3];
                slot.ang = [Fixed64::ZERO; 3];
            }
            if slot.age > 240 && slot.lod != DebrisLod::Merged {
                *slot = DebrisSlot::empty();
            }
        }
    }

    fn free_index(&self) -> Option<usize> {
        self.slots.iter().position(|slot| !slot.occupied)
    }

    fn evict(&mut self) -> usize {
        let mut choice = 0;
        for index in 1..MAX_SIM_DEBRIS {
            let slot = &self.slots[index];
            let best = &self.slots[choice];
            let older = slot.occupied
                && (!best.occupied
                    || slot.born_tick < best.born_tick
                    || (slot.born_tick == best.born_tick && slot.id < best.id));
            if older {
                choice = index;
            }
        }
        self.slots[choice] = DebrisSlot::empty();
        choice
    }
}

impl Default for DebrisPool {
    fn default() -> Self {
        Self::new()
    }
}

fn whole(value: Fixed64) -> i32 {
    let quot = value.raw().div_euclid(Fixed64::SCALE);
    quot.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::PieceFlags;

    fn node(id: u64, y: i64) -> SupportNode {
        SupportNode {
            id,
            structure: 1,
            mass: Fixed64::from_int(1),
            pos: [Fixed64::ZERO, Fixed64::from_int(y), Fixed64::ZERO],
            half: [Fixed64::from_int(1); 3],
            flags: PieceFlags::DYNAMIC,
            island: 1,
        }
    }

    #[test]
    fn a_falling_slot_sleeps_and_merges() {
        let mut pool = DebrisPool::new();
        let mut terrain = Terrain::new(0);
        terrain.load((0, 0));
        pool.spawn(&node(4, 6), 0);
        for _ in 0..180 {
            pool.step(&terrain);
        }
        let slot = pool
            .slots()
            .iter()
            .find(|slot| slot.occupied)
            .expect("slot");
        assert_eq!(slot.lod, DebrisLod::Merged);
        assert_eq!(slot.vel[1], Fixed64::ZERO);
    }

    #[test]
    fn eviction_drops_the_oldest_lowest_id() {
        let mut pool = DebrisPool::new();
        for id in 1..=MAX_SIM_DEBRIS as u64 {
            pool.spawn(&node(id, 1), id);
        }
        assert_eq!(pool.occupied_count(), MAX_SIM_DEBRIS);
        pool.spawn(&node(10_000, 1), 10_000);
        assert_eq!(pool.occupied_count(), MAX_SIM_DEBRIS);
        assert!(pool.slots().iter().all(|slot| slot.source_node != 1));
        assert!(pool.slots().iter().any(|slot| slot.source_node == 10_000));
    }
}
