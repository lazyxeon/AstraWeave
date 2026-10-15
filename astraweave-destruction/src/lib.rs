//! Deterministic destruction.
//!
//! Sim state is `Fixed64` and integers. Display meshes and cosmetic particles
//! are outside this crate and are not hashed. Rapier poses are not read back.

#![forbid(unsafe_code)]
#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

pub mod budget;
pub mod command;
pub mod debris;
pub mod fracture;
pub mod graph;
pub mod jobs;
pub mod octree;
pub mod rng;
pub mod terrain;

use sha2::{Digest, Sha256};

use astraweave_math::Fixed64;

use budget::{DEMOLITION_TICKS, MAX_LOG_COMMANDS, MAX_REMESHES_PER_TICK};
use command::{
    command_ord, push_command, validate, Anchor, Brush, BrushOp, BrushShape, CommandError,
    CommandKind, DestructionCommand, Detach, Explosion, Impact, SpawnBond, SpawnNode, Weave,
};
use debris::DebrisPool;
use graph::{dist2, mul_div, Graph, PieceFlags, SupportNode};
use jobs::QueryPool;
use terrain::Terrain;

pub use budget::{
    DEMOLITION_TICKS as SHOWCASE_TICKS, MAX_BRUSH_CELLS, MAX_LOG_COMMANDS as LATE_JOIN_CAP,
    MAX_NODES_PER_TICK, MAX_SIM_DEBRIS, SLEEP_TICKS, STRESS_ITERS,
};
pub use command::{CommandError as DestructionCommandError, DestructionCommand as Command};
pub use fracture::{bake_bytes, bake_gate, planar_children, BreakRule};
pub use graph::{allows_climb, PieceFlags as FractureFlags};
pub use terrain::FlatColumnBase;

/// A geometry change navigation can consume. Integer bounds, no floats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeometryNotice {
    pub tick: u64,
    pub x0: i32,
    pub y0: i32,
    pub z0: i32,
    pub x1: i32,
    pub y1: i32,
    pub z1: i32,
    /// 1 brush, 2 collapse, 3 weave.
    pub reason: u8,
}

const MAX_DUE: usize = 16;
const NOTICE_CAP: usize = 32;

pub struct DestructionWorld {
    seed: u64,
    ground_top: i32,
    tick: u64,
    next_seq: u32,
    log: Vec<DestructionCommand>,
    pending: Vec<DestructionCommand>,
    applied: [DestructionCommand; MAX_DUE],
    applied_len: usize,
    graph: Graph,
    terrain: Terrain,
    debris: DebrisPool,
    queries: QueryPool,
    reload_chunk_on_tick: Option<u64>,
    late_join_closed: bool,
    notices: [GeometryNotice; NOTICE_CAP],
    notice_len: usize,
    query_marks: [u8; 4],
    fractured: [u64; 16],
    fractured_len: usize,
    hash_buf: Vec<u8>,
    dag_children: Box<[[u16; 8]; 2048]>,
    dag_density: Box<[i8; 2048]>,
    dag_leaf: Box<[u8; 2048]>,
}

impl DestructionWorld {
    pub fn new(seed: u64, ground_top: i32) -> Self {
        let mut terrain = Terrain::new(ground_top);
        terrain.load((0, 0));
        Self {
            seed,
            ground_top,
            tick: 0,
            next_seq: 1,
            log: Vec::with_capacity(64),
            pending: Vec::with_capacity(64),
            applied: [DestructionCommand {
                tick: 0,
                seq: 0,
                entity_id: 0,
                kind: CommandKind::Anchor(Anchor {
                    node_id: 0,
                    anchored: false,
                }),
            }; MAX_DUE],
            applied_len: 0,
            graph: Graph::new(),
            terrain,
            debris: DebrisPool::new(),
            queries: QueryPool::new(),
            reload_chunk_on_tick: None,
            late_join_closed: false,
            notices: [GeometryNotice {
                tick: 0,
                x0: 0,
                y0: 0,
                z0: 0,
                x1: 0,
                y1: 0,
                z1: 0,
                reason: 0,
            }; NOTICE_CAP],
            notice_len: 0,
            query_marks: [0; 4],
            fractured: [0; 16],
            fractured_len: 0,
            hash_buf: Vec::with_capacity(65_536),
            dag_children: Box::new([[0; 8]; 2048]),
            dag_density: Box::new([0; 2048]),
            dag_leaf: Box::new([0; 2048]),
        }
    }

    pub fn aravath_gate(seed: u64) -> Self {
        let mut world = Self::new(seed, 0);
        world.reload_chunk_on_tick = Some(60);
        let half = [Fixed64::from_int(1); 3];
        let post = PieceFlags::ANCHOR.bits() | PieceFlags::CLIMBABLE.bits();
        world.push_script(spawn_node(
            0,
            1,
            Fixed64::from_int(20),
            [Fixed64::from_int(-2), Fixed64::from_int(1), Fixed64::ZERO],
            half,
            post,
        ));
        world.push_script(spawn_node(
            0,
            1,
            Fixed64::from_int(20),
            [Fixed64::from_int(2), Fixed64::from_int(1), Fixed64::ZERO],
            half,
            post,
        ));
        world.push_script(spawn_node(
            0,
            1,
            Fixed64::from_int(80),
            [Fixed64::ZERO, Fixed64::from_int(6), Fixed64::ZERO],
            half,
            PieceFlags::ANTI_CLIMB.bits(),
        ));
        world.push_script(spawn_bond(
            0,
            1,
            3,
            Fixed64::from_int(100),
            Fixed64::from_int(1),
        ));
        world.push_script(spawn_bond(
            0,
            2,
            3,
            Fixed64::from_int(100),
            Fixed64::from_int(1),
        ));
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

    /// Weaving hook. Density and decay are recorded in the command log.
    pub fn enqueue_weave(
        &mut self,
        tick: u64,
        entity_id: u64,
        effect: Weave,
    ) -> Result<(), CommandError> {
        self.enqueue(DestructionCommand {
            tick,
            seq: 0,
            entity_id,
            kind: CommandKind::Weave(effect),
        })
    }

    /// Navigation hook. Drains geometry notices in the order they were recorded.
    pub fn drain_geometry_notices(&mut self, out: &mut [GeometryNotice]) -> usize {
        let count = self.notice_len.min(out.len());
        out[..count].copy_from_slice(&self.notices[..count]);
        self.notice_len = 0;
        count
    }

    /// Command log plus the seed and tick. Replaying it restores every structure.
    pub fn persist_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + (self.log.len() + self.pending.len()) * 64);
        out.extend_from_slice(b"AWDESTLOG");
        out.extend_from_slice(&self.seed.to_le_bytes());
        out.extend_from_slice(&self.ground_top.to_le_bytes());
        out.extend_from_slice(&self.tick.to_le_bytes());
        let count = (self.log.len() + self.pending.len()) as u32;
        out.extend_from_slice(&count.to_le_bytes());
        for cmd in self.log.iter().chain(self.pending.iter()) {
            push_command(&mut out, cmd);
        }
        out
    }

    pub fn restore(bytes: &[u8]) -> Result<Self, CommandError> {
        if bytes.len() < 9 + 8 + 4 + 8 + 4 || &bytes[..9] != b"AWDESTLOG" {
            return Err(CommandError::ZeroExtent);
        }
        let seed = u64::from_le_bytes(bytes[9..17].try_into().unwrap_or([0; 8]));
        let ground = i32::from_le_bytes(bytes[17..21].try_into().unwrap_or([0; 4]));
        let saved_tick = u64::from_le_bytes(bytes[21..29].try_into().unwrap_or([0; 8]));
        let count = u32::from_le_bytes(bytes[29..33].try_into().unwrap_or([0; 4])) as usize;
        let mut world = Self::new(seed, ground);
        let mut cursor = 33;
        for index in 0..count {
            if cursor >= bytes.len() {
                break;
            }
            let (cmd, next) = read_command(&bytes[cursor..], index as u32)?;
            cursor += next;
            world.enqueue_preserving_seq(cmd)?;
        }
        while world.tick < saved_tick {
            world.tick();
        }
        Ok(world)
    }

    pub fn tick(&mut self) {
        let now = self.tick;
        self.applied_len = 0;
        let mut due = [self.applied[0]; MAX_DUE];
        let mut due_len = 0;
        let mut index = 0;
        while index < self.pending.len() {
            if self.pending[index].tick == now {
                if due_len < MAX_DUE {
                    due[due_len] = self.pending[index];
                    due_len += 1;
                }
                self.pending.swap_remove(index);
            } else {
                index += 1;
            }
        }
        due[..due_len].sort_by_key(command_ord);
        for cmd in due.iter().take(due_len).copied() {
            self.apply(cmd);
            self.log.push(cmd);
            if self.applied_len < MAX_DUE {
                self.applied[self.applied_len] = cmd;
                self.applied_len += 1;
            }
        }
        let broken_before = self.graph.broken_ids().len();
        if self.graph.dirty {
            self.graph.solve();
            if self.graph.broken_ids().len() > broken_before {
                self.push_notice(0, 0, 0, 1, 1, 1, 2);
            }
            self.fracture_dynamic();
            self.spawn_dynamic();
        }
        self.debris.step(&self.terrain);
        if self.reload_chunk_on_tick == Some(now) {
            self.terrain.unload((0, 0));
            self.terrain.load((0, 0));
        }
        let _ = self.terrain.take_remesh(MAX_REMESHES_PER_TICK);
        self.refresh_queries();
        self.tick = self.tick.saturating_add(1);
    }

    pub fn state_hash(&mut self) -> [u8; 32] {
        self.hash_buf.clear();
        let bytes = &mut self.hash_buf;
        bytes.extend_from_slice(b"AWDEST1");
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&(self.applied_len as u32).to_le_bytes());
        for cmd in &self.applied[..self.applied_len] {
            push_command(bytes, cmd);
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
        self.terrain.write_loaded(bytes);
        bytes.extend_from_slice(&self.query_marks);
        bytes.extend_from_slice(&(self.notice_len as u32).to_le_bytes());
        for notice in &self.notices[..self.notice_len] {
            bytes.extend_from_slice(&notice.tick.to_le_bytes());
            bytes.extend_from_slice(&notice.x0.to_le_bytes());
            bytes.extend_from_slice(&notice.y0.to_le_bytes());
            bytes.extend_from_slice(&notice.z0.to_le_bytes());
            bytes.extend_from_slice(&notice.x1.to_le_bytes());
            bytes.extend_from_slice(&notice.y1.to_le_bytes());
            bytes.extend_from_slice(&notice.z1.to_le_bytes());
            bytes.push(notice.reason);
        }
        let digest = Sha256::digest(&self.hash_buf);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }

    fn apply(&mut self, cmd: DestructionCommand) {
        match cmd.kind {
            CommandKind::Impact(impact) => self.apply_impact(cmd.entity_id, impact),
            CommandKind::Explosion(explosion) => self.apply_explosion(cmd.entity_id, explosion),
            CommandKind::Brush(brush) => {
                let origin = brush.origin;
                let radius = i32::from(brush.radius.max(brush.half_extents[0]));
                self.terrain.apply_brush(brush);
                self.push_notice(
                    origin[0] - radius,
                    origin[1] - radius,
                    origin[2] - radius,
                    origin[0] + radius,
                    origin[1] + radius,
                    origin[2] + radius,
                    1,
                );
            }
            CommandKind::Detach(Detach { bond_id }) => {
                self.graph.detach(bond_id);
            }
            CommandKind::Anchor(Anchor { node_id, anchored }) => {
                self.graph.set_anchor(node_id, anchored);
            }
            CommandKind::SpawnNode(node) => {
                self.graph.add_node(
                    node.structure,
                    node.mass,
                    node.pos,
                    node.half,
                    PieceFlags::from_bits_truncate(node.flags),
                );
            }
            CommandKind::SpawnBond(bond) => {
                self.graph
                    .add_bond(bond.a, bond.b, bond.strength, bond.area);
            }
            CommandKind::Weave(weave) => self.apply_weave(weave),
        }
    }

    fn apply_weave(&mut self, weave: Weave) {
        let radius = i32::from(weave.radius);
        self.terrain
            .apply_density_delta(weave.origin, radius, weave.density_delta);
        let center = [
            Fixed64::from_int(i64::from(weave.origin[0])),
            Fixed64::from_int(i64::from(weave.origin[1])),
            Fixed64::from_int(i64::from(weave.origin[2])),
        ];
        let radius2 =
            Fixed64::from_int(i64::from(radius)).mul(Fixed64::from_int(i64::from(radius)));
        let mut ids = [0u64; 16];
        let mut count = 0usize;
        for node in &self.graph.nodes {
            if dist2(node.pos, center) > radius2 {
                continue;
            }
            for bond_id in self.graph.bonds_of(node.id) {
                if count < ids.len() {
                    ids[count] = bond_id;
                    count += 1;
                }
            }
        }
        ids[..count].sort_unstable();
        let mut previous = 0u64;
        let mut seen = false;
        for bond_id in &ids[..count] {
            if seen && *bond_id == previous {
                continue;
            }
            previous = *bond_id;
            seen = true;
            self.graph.add_damage(*bond_id, weave.decay);
        }
        self.push_notice(
            weave.origin[0] - radius,
            weave.origin[1] - radius,
            weave.origin[2] - radius,
            weave.origin[0] + radius,
            weave.origin[1] + radius,
            weave.origin[2] + radius,
            3,
        );
    }

    fn push_notice(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, reason: u8) {
        if self.notice_len == NOTICE_CAP {
            self.notices.copy_within(1.., 0);
            self.notice_len -= 1;
        }
        self.notices[self.notice_len] = GeometryNotice {
            tick: self.tick,
            x0,
            y0,
            z0,
            x1,
            y1,
            z1,
            reason,
        };
        self.notice_len += 1;
    }

    fn refresh_queries(&mut self) {
        let origin = [0, 0, 0];
        let points = [[16, 2, 16], [0, 0, 0], [20, 1, 16], [4, 2, 4]];
        let Some(dag) = self.terrain.dag((0, 0)) else {
            self.query_marks = [0; 4];
            return;
        };
        let len = dag.copy_nodes(
            self.dag_children.as_mut(),
            self.dag_density.as_mut(),
            self.dag_leaf.as_mut(),
        );
        let root = dag.root();
        let mut marks = [0u8; 4];
        self.queries.query_solid(
            self.dag_children.as_ref(),
            self.dag_density.as_ref(),
            self.dag_leaf.as_ref(),
            len,
            root,
            origin,
            &points,
            &mut marks,
        );
        self.query_marks = marks;
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
                if bond_ids.len() < 32 {
                    bond_ids.push((bond_id, falloff));
                }
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

    fn already_fractured(&self, id: u64) -> bool {
        self.fractured[..self.fractured_len].contains(&id)
    }

    fn mark_fractured(&mut self, id: u64) {
        if self.already_fractured(id) || self.fractured_len >= self.fractured.len() {
            return;
        }
        self.fractured[self.fractured_len] = id;
        self.fractured_len += 1;
    }

    /// One seeded planar cut the first time a node loses support.
    fn fracture_dynamic(&mut self) {
        let mut parents = [0u64; 8];
        let mut count = 0usize;
        for node in &self.graph.nodes {
            if node.flags.contains(PieceFlags::DYNAMIC)
                && !node.flags.contains(PieceFlags::ANTI_DESTROY)
                && !self.already_fractured(node.id)
                && count < parents.len()
            {
                parents[count] = node.id;
                count += 1;
            }
        }
        for index in 0..count {
            let id = parents[index];
            self.mark_fractured(id);
            let Some(parent) = self.graph.node(id).cloned() else {
                continue;
            };
            for child in planar_children(&parent) {
                let child_id = self.graph.add_node(
                    child.structure,
                    child.mass,
                    child.pos,
                    child.half,
                    child.flags,
                );
                self.mark_fractured(child_id);
            }
        }
    }

    fn spawn_dynamic(&mut self) {
        let tick = self.tick;
        let mut spawned: [SupportNode; 8] = core::array::from_fn(|_| SupportNode {
            id: 0,
            structure: 0,
            mass: Fixed64::ZERO,
            pos: [Fixed64::ZERO; 3],
            half: [Fixed64::ZERO; 3],
            flags: PieceFlags::empty(),
            island: 0,
        });
        let mut count = 0usize;
        for node in &self.graph.nodes {
            if node.flags.contains(PieceFlags::DYNAMIC) && count < spawned.len() {
                spawned[count] = node.clone();
                count += 1;
            }
        }
        for node in &spawned[..count] {
            self.debris.spawn(node, tick);
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

fn spawn_node(
    tick: u64,
    structure: u64,
    mass: Fixed64,
    pos: [Fixed64; 3],
    half: [Fixed64; 3],
    flags: u32,
) -> DestructionCommand {
    DestructionCommand {
        tick,
        seq: 0,
        entity_id: structure,
        kind: CommandKind::SpawnNode(SpawnNode {
            structure,
            mass,
            pos,
            half,
            flags,
        }),
    }
}

fn spawn_bond(tick: u64, a: u64, b: u64, strength: Fixed64, area: Fixed64) -> DestructionCommand {
    DestructionCommand {
        tick,
        seq: 0,
        entity_id: 1,
        kind: CommandKind::SpawnBond(SpawnBond {
            a,
            b,
            strength,
            area,
        }),
    }
}

fn read_command(bytes: &[u8], seq: u32) -> Result<(DestructionCommand, usize), CommandError> {
    if bytes.len() < 8 + 4 + 8 + 1 {
        return Err(CommandError::ZeroExtent);
    }
    let tick = u64::from_le_bytes(bytes[0..8].try_into().unwrap_or([0; 8]));
    let stored_seq = u32::from_le_bytes(bytes[8..12].try_into().unwrap_or([0; 4]));
    let entity_id = u64::from_le_bytes(bytes[12..20].try_into().unwrap_or([0; 8]));
    let _ = (stored_seq, seq);
    let tag = bytes[20];
    let mut cursor = 21;
    let kind = match tag {
        1 => {
            let (point, used) = read_fixed3(&bytes[cursor..])?;
            cursor += used;
            let (impulse, used) = read_fixed(&bytes[cursor..])?;
            cursor += used;
            CommandKind::Impact(Impact { point, impulse })
        }
        2 => {
            let (center, used) = read_fixed3(&bytes[cursor..])?;
            cursor += used;
            let (radius, used) = read_fixed(&bytes[cursor..])?;
            cursor += used;
            let (impulse, used) = read_fixed(&bytes[cursor..])?;
            cursor += used;
            CommandKind::Explosion(Explosion {
                center,
                radius,
                impulse,
            })
        }
        3 => {
            if bytes.len() < cursor + 2 + 12 + 2 + 6 + 2 {
                return Err(CommandError::ZeroExtent);
            }
            let shape = match bytes[cursor] {
                2 => BrushShape::Box,
                3 => BrushShape::Capsule,
                _ => BrushShape::Sphere,
            };
            let op = match bytes[cursor + 1] {
                2 => BrushOp::Add,
                3 => BrushOp::Smooth,
                _ => BrushOp::Subtract,
            };
            cursor += 2;
            let origin = [
                i32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap_or([0; 4])),
                i32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap_or([0; 4])),
                i32::from_le_bytes(bytes[cursor + 8..cursor + 12].try_into().unwrap_or([0; 4])),
            ];
            cursor += 12;
            let radius = u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap_or([0; 2]));
            cursor += 2;
            let half_extents = [
                u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap_or([0; 2])),
                u16::from_le_bytes(bytes[cursor + 2..cursor + 4].try_into().unwrap_or([0; 2])),
                u16::from_le_bytes(bytes[cursor + 4..cursor + 6].try_into().unwrap_or([0; 2])),
            ];
            cursor += 6;
            let strength =
                i16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap_or([0; 2]));
            cursor += 2;
            CommandKind::Brush(Brush {
                shape,
                op,
                origin,
                radius,
                half_extents,
                strength,
            })
        }
        6 => {
            if bytes.len() < cursor + 8 + 8 + 24 + 24 + 4 {
                return Err(CommandError::ZeroExtent);
            }
            let structure =
                u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap_or([0; 8]));
            cursor += 8;
            let (mass, used) = read_fixed(&bytes[cursor..])?;
            cursor += used;
            let (pos, used) = read_fixed3(&bytes[cursor..])?;
            cursor += used;
            let (half, used) = read_fixed3(&bytes[cursor..])?;
            cursor += used;
            let flags = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap_or([0; 4]));
            cursor += 4;
            CommandKind::SpawnNode(SpawnNode {
                structure,
                mass,
                pos,
                half,
                flags,
            })
        }
        7 => {
            if bytes.len() < cursor + 8 + 8 + 16 {
                return Err(CommandError::ZeroExtent);
            }
            let a = u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap_or([0; 8]));
            let b = u64::from_le_bytes(bytes[cursor + 8..cursor + 16].try_into().unwrap_or([0; 8]));
            cursor += 16;
            let (strength, used) = read_fixed(&bytes[cursor..])?;
            cursor += used;
            let (area, used) = read_fixed(&bytes[cursor..])?;
            cursor += used;
            CommandKind::SpawnBond(SpawnBond {
                a,
                b,
                strength,
                area,
            })
        }
        _ => return Err(CommandError::ZeroExtent),
    };
    Ok((
        DestructionCommand {
            tick,
            seq: stored_seq,
            entity_id,
            kind,
        },
        cursor,
    ))
}

fn read_fixed(bytes: &[u8]) -> Result<(Fixed64, usize), CommandError> {
    if bytes.len() < 8 {
        return Err(CommandError::ZeroExtent);
    }
    let raw = i64::from_le_bytes(bytes[..8].try_into().unwrap_or([0; 8]));
    Ok((Fixed64::from_raw(raw), 8))
}

fn read_fixed3(bytes: &[u8]) -> Result<([Fixed64; 3], usize), CommandError> {
    let (x, _) = read_fixed(bytes)?;
    let (y, _) = read_fixed(&bytes[8..])?;
    let (z, _) = read_fixed(&bytes[16..])?;
    Ok(([x, y, z], 24))
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
        assert!(world.notice_len > 0);
    }

    #[test]
    fn command_log_restores_the_gate_and_the_crater() {
        let mut world = DestructionWorld::aravath_gate(0xA2A0_0001);
        for _ in 0..80 {
            world.tick();
        }
        let bytes = world.persist_bytes();
        let mut restored = DestructionWorld::restore(&bytes).expect("restore");
        assert_eq!(restored.state_hash(), world.state_hash());
        assert!(restored.terrain().surface_y(16, 16) < 0);
        assert!(restored
            .graph()
            .node(3)
            .expect("lintel")
            .flags
            .contains(PieceFlags::DYNAMIC));
    }

    #[test]
    fn weave_changes_density_and_decays_a_bond() {
        let mut world = DestructionWorld::aravath_gate(2);
        world.tick();
        world
            .enqueue_weave(
                1,
                1,
                Weave {
                    origin: [0, 6, 0],
                    radius: 2,
                    density_delta: -40,
                    decay: Fixed64::from_int(90),
                },
            )
            .unwrap();
        world.tick();
        let damage = world
            .graph()
            .bonds
            .iter()
            .map(|bond| bond.damage.raw())
            .max()
            .unwrap_or(0);
        assert!(damage > 0);
        let mut notices = [GeometryNotice {
            tick: 0,
            x0: 0,
            y0: 0,
            z0: 0,
            x1: 0,
            y1: 0,
            z1: 0,
            reason: 0,
        }; 8];
        let count = world.drain_geometry_notices(&mut notices);
        assert!(count > 0);
        assert!(notices[..count].iter().any(|notice| notice.reason == 3));
    }

    #[test]
    fn parallel_queries_match_the_dag_walk() {
        let mut world = DestructionWorld::new(1, 4);
        world.tick();
        world.enqueue(dig(1)).unwrap();
        world.tick();
        let solid = world
            .terrain()
            .dag((0, 0))
            .expect("dag")
            .solid_at(16, 2, 16, |_, _, _| true);
        assert!(!solid);
        assert_eq!(world.query_marks[0], 1);
    }
}
