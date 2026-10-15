//! Companion route across a span in the Aravath greybox.
//!
//! The window does not draw this body yet. Tick 30 detaches the deck. The
//! collapse notice is the union of pieces that became dynamic. That box, in
//! milli-units, is the nav mark and the causal cut. It is not a placeholder.

use astraweave_causality::{
    bridge_world, merchant_ford_input, CausalInput, CausalWorld, BRIDGE_STRUCTURE, CLASS_STRUCTURE,
    KIND_GEOMETRY_CUT,
};
use astraweave_destruction::command::{CommandKind, Detach, SpawnBond, SpawnNode};
use astraweave_destruction::{Command, DestructionWorld, FractureFlags, GeometryNotice};
use astraweave_math::Fixed64;
use astraweave_nav::{bridge_and_ford, nav_change_from_kind, NavChangeKind, NavScratch, NavTiles};
use glam::Vec3;

/// Player-facing name for this scene.
pub const SCENE_LABEL: &str = "Aravath";

const COLLAPSE_TICK: u64 = 30;
const FORD_TICK: u64 = 120;

pub struct BridgeAgent {
    tiles: NavTiles,
    scratch: NavScratch,
    destruction: DestructionWorld,
    causal: CausalWorld,
    /// Highest waypoint x, in milli-units, on the current route.
    pub route_x_milli: i32,
    /// Collapse notice converted to milli-units. Zeros until the deck drops.
    pub cut_milli: [i32; 6],
    pub setup_ok: bool,
}

impl BridgeAgent {
    #[must_use]
    pub fn new() -> Self {
        let tiles = bridge_and_ford();
        let mut destruction = DestructionWorld::new(0xA2A0_B81D, 0);
        let setup_ok = script_bridge(&mut destruction);
        let mut agent = Self {
            tiles,
            scratch: NavScratch::new(),
            destruction,
            causal: bridge_world(),
            route_x_milli: 0,
            cut_milli: [0; 6],
            setup_ok,
        };
        agent.refresh();
        agent
    }

    pub fn step(&mut self, tick: u64) {
        if tick == COLLAPSE_TICK && !self.detach_deck() {
            self.setup_ok = false;
        }
        self.destruction.tick();
        self.consume_collapse(tick);
        let _committed = self.tiles.commit_tick();
        self.refresh();
        if tick == FORD_TICK && self.route_x_milli > 16_000 && !self.admit_ford() {
            self.setup_ok = false;
        }
        if self.causal.tick().is_err() {
            self.setup_ok = false;
        }
    }

    #[must_use]
    pub fn stock(&self) -> i32 {
        self.causal.stock()
    }

    #[must_use]
    pub fn stance(&self) -> u8 {
        self.causal.stance()
    }

    #[must_use]
    pub fn quest_template(&self) -> u16 {
        self.causal.quest_template()
    }

    #[must_use]
    pub fn quest_complete(&self) -> u8 {
        self.causal.quest_complete()
    }

    fn detach_deck(&mut self) -> bool {
        self.enqueue(Command {
            tick: COLLAPSE_TICK,
            seq: 0,
            entity_id: 1,
            kind: CommandKind::Detach(Detach { bond_id: 1 }),
        }) && self.enqueue(Command {
            tick: COLLAPSE_TICK,
            seq: 0,
            entity_id: 1,
            kind: CommandKind::Detach(Detach { bond_id: 2 }),
        })
    }

    fn consume_collapse(&mut self, tick: u64) {
        let blank = GeometryNotice {
            tick: 0,
            x0: 0,
            y0: 0,
            z0: 0,
            x1: 0,
            y1: 0,
            z1: 0,
            reason: 0,
        };
        let mut notices = [blank; 8];
        let count = self.destruction.drain_geometry_notices(&mut notices);
        for notice in notices.iter().take(count) {
            if notice.reason != 2 {
                continue;
            }
            let min = [
                whole_to_milli(notice.x0),
                whole_to_milli(notice.y0),
                whole_to_milli(notice.z0),
            ];
            let max = [
                whole_to_milli(notice.x1),
                whole_to_milli(notice.y1),
                whole_to_milli(notice.z1),
            ];
            self.tiles.mark(nav_change_from_kind(
                NavChangeKind::Detach,
                tick,
                0,
                min,
                max,
            ));
            self.cut_milli = [min[0], min[1], min[2], max[0], max[1], max[2]];
            if self
                .causal
                .push_input(CausalInput {
                    tick,
                    seq: 0,
                    kind: KIND_GEOMETRY_CUT,
                    class: CLASS_STRUCTURE,
                    subject: BRIDGE_STRUCTURE,
                    region: self.cut_milli,
                    payload: [1, 0, 0, 0],
                })
                .is_err()
            {
                self.setup_ok = false;
            }
        }
    }

    fn admit_ford(&mut self) -> bool {
        self.causal.push_input(merchant_ford_input()).is_ok()
    }

    fn enqueue(&mut self, cmd: Command) -> bool {
        self.destruction.enqueue(cmd).is_ok()
    }

    fn refresh(&mut self) {
        let _count = self.tiles.query_path(
            &mut self.scratch,
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 46.0),
        );
        let mut max_x = i32::MIN;
        for point in self
            .scratch
            .waypoints
            .iter()
            .take(self.scratch.waypoint_len as usize)
        {
            max_x = max_x.max((point.x * 1000.0) as i32);
        }
        self.route_x_milli = max_x;
    }
}

fn script_bridge(world: &mut DestructionWorld) -> bool {
    let post = [Fixed64::from_int(1); 3];
    let anchor = FractureFlags::ANCHOR.bits();
    let deck = [
        Fixed64::from_int(3),
        Fixed64::from_int(1),
        Fixed64::from_int(8),
    ];
    world
        .enqueue(spawn_node(
            0,
            [Fixed64::from_int(-6), Fixed64::ZERO, Fixed64::from_int(24)],
            post,
            anchor,
        ))
        .is_ok()
        && world
            .enqueue(spawn_node(
                0,
                [Fixed64::from_int(10), Fixed64::ZERO, Fixed64::from_int(24)],
                post,
                anchor,
            ))
            .is_ok()
        && world
            .enqueue(spawn_node(
                0,
                [Fixed64::from_int(2), Fixed64::ZERO, Fixed64::from_int(24)],
                deck,
                0,
            ))
            .is_ok()
        && world.enqueue(spawn_bond(0, 1, 3)).is_ok()
        && world.enqueue(spawn_bond(0, 2, 3)).is_ok()
}

fn spawn_node(tick: u64, pos: [Fixed64; 3], half: [Fixed64; 3], flags: u32) -> Command {
    Command {
        tick,
        seq: 0,
        entity_id: 1,
        kind: CommandKind::SpawnNode(SpawnNode {
            structure: 1,
            mass: Fixed64::from_int(20),
            pos,
            half,
            flags,
        }),
    }
}

fn spawn_bond(tick: u64, a: u64, b: u64) -> Command {
    Command {
        tick,
        seq: 0,
        entity_id: 1,
        kind: CommandKind::SpawnBond(SpawnBond {
            a,
            b,
            strength: Fixed64::from_int(100),
            area: Fixed64::from_int(1),
        }),
    }
}

fn whole_to_milli(whole: i32) -> i32 {
    whole.saturating_mul(1_000)
}

impl Default for BridgeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_sends_the_agent_to_the_ford() {
        assert_eq!(SCENE_LABEL, "Aravath");
        let mut agent = BridgeAgent::new();
        assert!(agent.setup_ok);
        assert!(
            agent.route_x_milli < 8_000,
            "bridge route x {}",
            agent.route_x_milli
        );
        for tick in 0..40 {
            agent.step(tick);
        }
        assert!(agent.setup_ok);
        assert!(
            agent.route_x_milli > 16_000,
            "ford route x {}",
            agent.route_x_milli
        );
    }

    #[test]
    fn collapse_notice_cuts_the_route_and_opens_the_quest() {
        let mut agent = BridgeAgent::new();
        for tick in 0..=FORD_TICK {
            agent.step(tick);
        }
        assert!(agent.setup_ok);
        assert_eq!(
            agent.cut_milli,
            [-1_000, -1_000, 16_000, 5_000, 1_000, 32_000]
        );
        let cut = agent
            .causal
            .events()
            .iter()
            .find(|event| event.kind == KIND_GEOMETRY_CUT)
            .expect("cut");
        assert_eq!(cut.region, agent.cut_milli);
        assert_eq!(cut.tick, COLLAPSE_TICK);
        assert!(agent.route_x_milli > 16_000);
        assert_eq!(agent.stock(), 0);
        assert_eq!(agent.stance(), 2);
        assert_eq!(agent.quest_template(), 1);
        assert_eq!(agent.quest_complete(), 1);
    }
}
