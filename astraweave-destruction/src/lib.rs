//! Deterministic destruction.
//!
//! Sim state is `Fixed64` and integers. Display meshes and cosmetic particles
//! are outside this crate and are not hashed. Rapier poses are not read back.

#![forbid(unsafe_code)]

pub mod budget;
pub mod command;
pub mod debris;
pub mod fracture;
pub mod graph;
pub mod rng;
pub mod terrain;

use sha2::{Digest, Sha256};

use astraweave_math::Fixed64;

use budget::{DEMOLITION_TICKS, MAX_LOG_COMMANDS, MAX_REMESHES_PER_TICK};
use command::{
    command_ord, push_command, validate, Anchor, Brush, BrushOp, BrushShape, CommandError,
    CommandKind, DestructionCommand, Detach, Explosion, Impact,
};
use debris::DebrisPool;
use graph::{dist2, mul_div, Graph, PieceFlags};
use terrain::Terrain;

pub use budget::{
    DEMOLITION_TICKS as SHOWCASE_TICKS, MAX_BRUSH_CELLS, MAX_LOG_COMMANDS as LATE_JOIN_CAP,
    MAX_NODES_PER_TICK, MAX_SIM_DEBRIS, SLEEP_TICKS, STRESS_ITERS,
};
pub use command::{CommandError as DestructionCommandError, DestructionCommand as Command};
pub use fracture::{bake_bytes, bake_gate, BreakRule};
pub use graph::{allows_climb, PieceFlags as FractureFlags};
pub use terrain::FlatColumnBase;

#[derive(Clone, Debug)]
pub struct DestructionWorld {
    seed: u64,
    tick: u64,
    next_seq: u32,
    log: Vec<DestructionCommand>,
    pending: Vec<DestructionCommand>,
    applied_this_tick: Vec<DestructionCommand>,
    graph: Graph,
    terrain: Terrain,
    debris: DebrisPool,
    reload_chunk_on_tick: Option<u64>,
    late_join_closed: bool,
}

impl DestructionWorld {
    pub fn new(seed: u64, ground_top: i32) -> Self {
        let mut terrain = Terrain::new(ground_top);
        terrain.load((0, 0));
        Self {
            seed,
            tick: 0,
            next_seq: 1,
            log: Vec::new(),
            pending: Vec::new(),
            applied_this_tick: Vec::new(),
            graph: Graph::new(),
            terrain,
            debris: DebrisPool::new(),
            reload_chunk_on_tick: None,
            late_join_closed: false,
        }
    }

    pub fn aravath_gate(seed: u64) -> Self {
        let mut world = Self::new(seed, 0);
        world.reload_chunk_on_tick = Some(60);
        let half = [Fixed64::from_int(1); 3];
        world.graph.add_node(
            1,
            Fixed64::from_int(20),
            [Fixed64::from_int(-2), Fixed64::from_int(1), Fixed64::ZERO],
            half,
            PieceFlags::ANCHOR | PieceFlags::CLIMBABLE,
        );
        world.graph.add_node(
            1,
            Fixed64::from_int(20),
            [Fixed64::from_int(2), Fixed64::from_int(1), Fixed64::ZERO],
            half,
            PieceFlags::ANCHOR | PieceFlags::CLIMBABLE,
        );
        world.graph.add_node(
            1,
            Fixed64::from_int(80),
            [Fixed64::ZERO, Fixed64::from_int(6), Fixed64::ZERO],
            half,
            PieceFlags::ANTI_CLIMB,
        );
        world
            .graph
            .add_bond(1, 3, Fixed64::from_int(100), Fixed64::from_int(1));
        world
            .graph
            .add_bond(2, 3, Fixed64::from_int(100), Fixed64::from_int(1));
        world.push_script(impact(10, 1, Fixed64::from_int(-2), Fixed64::from_int(10)));
        world.push_script(explosion(20, 1));
        world.push_script(dig(40));
        world.push_script(fill(50));
        world
    }

    fn push_script(&mut self, mut cmd: DestructionCommand) {
        cmd.seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        self.pending.push(cmd);
    }

    pub fn tick_index(&self) -> u64 {
        self.tick
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn terrain(&self) -> &Terrain {
        &self.terrain
    }

    pub fn debris(&self) -> &DebrisPool {
        &self.debris
    }

    pub fn log_len(&self) -> usize {
        self.log.len()
    }

    pub fn late_join_closed(&self) -> bool {
        self.late_join_closed
    }

    pub fn enqueue_preserving_seq(&mut self, cmd: DestructionCommand) -> Result<(), CommandError> {
        validate(&cmd)?;
        self.pending.push(cmd);
        Ok(())
    }

    pub fn enqueue(&mut self, mut cmd: DestructionCommand) -> Result<(), CommandError> {
        validate(&cmd)?;
        if self.log.len() + self.pending.len() >= MAX_LOG_COMMANDS {
            self.late_join_closed = true;
            return Err(CommandError::LogFull);
        }
        cmd.seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        self.pending.push(cmd);
        Ok(())
    }

    pub fn tick(&mut self) {
        let now = self.tick;
        self.applied_this_tick.clear();
        let mut due = Vec::new();
        self.pending.retain(|cmd| {
            if cmd.tick == now {
                due.push(*cmd);
                false
            } else {
                true
            }
        });
        due.sort_by_key(command_ord);
        for cmd in due {
            self.apply(cmd);
            self.log.push(cmd);
            self.applied_this_tick.push(cmd);
        }
        if self.graph.dirty {
            self.graph.solve();
            self.spawn_dynamic();
        }
        self.debris.step(&self.terrain);
        if self.reload_chunk_on_tick == Some(now) {
            self.terrain.unload((0, 0));
            self.terrain.load((0, 0));
        }
        let _ = self.terrain.take_remesh(MAX_REMESHES_PER_TICK);
        self.tick = self.tick.saturating_add(1);
    }

    pub fn state_hash(&self) -> [u8; 32] {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"AWDEST1");
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&(self.applied_this_tick.len() as u32).to_le_bytes());
        for cmd in &self.applied_this_tick {
            push_command(&mut bytes, cmd);
        }
        for node in &self.graph.nodes {
            bytes.extend_from_slice(&node.id.to_le_bytes());
            bytes.extend_from_slice(&node.island.to_le_bytes());
            bytes.extend_from_slice(&node.flags.bits().to_le_bytes());
            bytes.extend_from_slice(&node.mass.raw().to_le_bytes());
            for axis in node.pos {
                bytes.extend_from_slice(&axis.raw().to_le_bytes());
            }
        }
        for bond in &self.graph.bonds {
            bytes.extend_from_slice(&bond.id.to_le_bytes());
            bytes.extend_from_slice(&bond.damage.raw().to_le_bytes());
            bytes.push(u8::from(bond.intact));
        }
        for slot in self.debris.slots() {
            bytes.push(u8::from(slot.occupied));
            bytes.extend_from_slice(&slot.id.to_le_bytes());
            bytes.extend_from_slice(&slot.born_tick.to_le_bytes());
            bytes.extend_from_slice(&slot.sleep_held.to_le_bytes());
            bytes.push(slot.lod as u8);
            for axis in slot.pos {
                bytes.extend_from_slice(&axis.raw().to_le_bytes());
            }
            for axis in slot.vel {
                bytes.extend_from_slice(&axis.raw().to_le_bytes());
            }
        }
        bytes.extend(self.terrain.loaded_bytes());
        let digest = Sha256::digest(&bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }

    fn apply(&mut self, cmd: DestructionCommand) {
        match cmd.kind {
            CommandKind::Impact(impact) => self.apply_impact(cmd.entity_id, impact),
            CommandKind::Explosion(explosion) => self.apply_explosion(cmd.entity_id, explosion),
            CommandKind::Brush(brush) => self.terrain.apply_brush(brush),
            CommandKind::Detach(Detach { bond_id }) => {
                self.graph.detach(bond_id);
            }
            CommandKind::Anchor(Anchor { node_id, anchored }) => {
                self.graph.set_anchor(node_id, anchored);
            }
        }
    }

    fn apply_impact(&mut self, structure: u64, impact: Impact) {
        let Some(node_id) = self.graph.closest_node(impact.point, structure) else {
            return;
        };
        if self
            .graph
            .node(node_id)
            .is_some_and(|node| node.flags.contains(PieceFlags::ANTI_DESTROY))
        {
            return;
        }
        for bond_id in self.graph.bonds_of(node_id) {
            self.graph.add_damage(bond_id, impact.impulse);
        }
    }

    fn apply_explosion(&mut self, structure: u64, explosion: Explosion) {
        let radius2 = explosion.radius.mul(explosion.radius);
        let mut bond_ids = Vec::new();
        for node in &self.graph.nodes {
            if structure != 0 && node.structure != structure {
                continue;
            }
            if node.flags.contains(PieceFlags::ANTI_DESTROY) {
                continue;
            }
            let d2 = dist2(node.pos, explosion.center);
            if d2 > radius2 {
                continue;
            }
            let falloff = mul_div(explosion.impulse, radius2.saturating_sub(d2), radius2);
            for bond_id in self.graph.bonds_of(node.id) {
                bond_ids.push((bond_id, falloff));
            }
        }
        bond_ids.sort_by_key(|(bond_id, falloff)| (*bond_id, std::cmp::Reverse(falloff.raw())));
        let mut seen = 0u64;
        let mut have_seen = false;
        for (bond_id, falloff) in bond_ids {
            if have_seen && bond_id == seen {
                continue;
            }
            seen = bond_id;
            have_seen = true;
            self.graph.add_damage(bond_id, falloff);
        }
    }

    fn spawn_dynamic(&mut self) {
        let tick = self.tick;
        let nodes: Vec<_> = self
            .graph
            .nodes
            .iter()
            .filter(|node| node.flags.contains(PieceFlags::DYNAMIC))
            .cloned()
            .collect();
        for node in nodes {
            self.debris.spawn(&node, tick);
        }
    }
}

pub fn demolition_bytes(world_seed: u64) -> Vec<u8> {
    let mut world = DestructionWorld::aravath_gate(world_seed);
    let mut out = Vec::with_capacity(DEMOLITION_TICKS as usize * 32);
    for _ in 0..DEMOLITION_TICKS {
        world.tick();
        out.extend_from_slice(&world.state_hash());
    }
    out
}

pub fn demolition_digest(world_seed: u64) -> String {
    hex(Sha256::digest(demolition_bytes(world_seed)))
}

pub fn aravath_lines() -> [&'static str; 4] {
    [
        "The Aravath gate stands on packed ground.",
        "The Aravath gate loses its support.",
        "A crater opens in the Aravath yard.",
        "Debris on the Aravath ground has settled.",
    ]
}

fn impact(tick: u64, entity: u64, x: Fixed64, impulse: Fixed64) -> DestructionCommand {
    DestructionCommand {
        tick,
        seq: 0,
        entity_id: entity,
        kind: CommandKind::Impact(Impact {
            point: [x, Fixed64::from_int(1), Fixed64::ZERO],
            impulse,
        }),
    }
}

fn explosion(tick: u64, entity: u64) -> DestructionCommand {
    DestructionCommand {
        tick,
        seq: 0,
        entity_id: entity,
        kind: CommandKind::Explosion(Explosion {
            center: [Fixed64::ZERO, Fixed64::from_int(6), Fixed64::ZERO],
            radius: Fixed64::from_int(8),
            impulse: Fixed64::from_int(80),
        }),
    }
}

fn dig(tick: u64) -> DestructionCommand {
    DestructionCommand {
        tick,
        seq: 0,
        entity_id: 0,
        kind: CommandKind::Brush(Brush {
            shape: BrushShape::Sphere,
            op: BrushOp::Subtract,
            origin: [16, 2, 16],
            radius: 3,
            half_extents: [1, 1, 1],
            strength: 4,
        }),
    }
}

fn fill(tick: u64) -> DestructionCommand {
    DestructionCommand {
        tick,
        seq: 0,
        entity_id: 0,
        kind: CommandKind::Brush(Brush {
            shape: BrushShape::Box,
            op: BrushOp::Add,
            origin: [20, 1, 16],
            radius: 1,
            half_extents: [1, 1, 1],
            strength: 2,
        }),
    }
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use debris::DebrisLod;

    #[test]
    fn rejected_brush_does_not_enter_the_log() {
        let mut world = DestructionWorld::new(1, 0);
        let err = world.enqueue(DestructionCommand {
            tick: 1,
            seq: 0,
            entity_id: 0,
            kind: CommandKind::Brush(Brush {
                shape: BrushShape::Sphere,
                op: BrushOp::Subtract,
                origin: [0, 0, 0],
                radius: 0,
                half_extents: [0, 0, 0],
                strength: 1,
            }),
        });
        assert!(err.is_err());
        world.tick();
        assert_eq!(world.log_len(), 0);
    }

    #[test]
    fn insertion_order_does_not_change_the_hash() {
        let mut a = DestructionWorld::new(1, 0);
        let mut b = DestructionWorld::new(1, 0);
        let mut first = dig(3);
        let mut second = fill(3);
        first.seq = 1;
        second.seq = 2;
        a.enqueue_preserving_seq(first).unwrap();
        a.enqueue_preserving_seq(second).unwrap();
        b.enqueue_preserving_seq(second).unwrap();
        b.enqueue_preserving_seq(first).unwrap();
        for _ in 0..4 {
            a.tick();
            b.tick();
        }
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn showcase_collapses_digs_and_sleeps() {
        let mut world = DestructionWorld::aravath_gate(0xA2A0_0001);
        for _ in 0..30 {
            world.tick();
        }
        let lintel = world.graph().node(3).expect("lintel");
        assert!(lintel.flags.contains(PieceFlags::DYNAMIC));
        assert!(!allows_climb(lintel.flags));
        assert!(allows_climb(world.graph().node(1).expect("post").flags));
        for _ in 0..270 {
            world.tick();
        }
        assert!(world.terrain().surface_y(16, 16) < 0);
        assert!(world.terrain().surface_y(20, 16) > 0);
        assert!(world
            .debris()
            .slots()
            .iter()
            .any(|slot| slot.occupied && slot.lod == DebrisLod::Merged));
        assert_eq!(demolition_bytes(1), demolition_bytes(1));
    }
}
