//! Temporary copy of the #243 terrain seam. Drop this module on rebase onto
//! `feat/destruction` (PR #243, `444b485c`) and call `astraweave_destruction`
//! plus `astraweave_math::Fixed64` instead.
//!
//! `push_command` matches that commit's byte layout. `TerrainBase` is the
//! column hook. The deformed world is the seed column plus a replay of the
//! command log. This file does not implement fracture, bonds, or the
//! destruction world's other slots.

use serde::{Deserialize, Serialize};

use crate::column_gen::{biome_at, column_material, on_yaqar_road, surface_height};

/// Chunks whose display mesh may be rebuilt in one tick. Matches #243.
pub const MAX_REMESHES_PER_TICK: usize = 4;

/// Commands kept for late join. Matches #243.
pub const MAX_LOG_COMMANDS: usize = 65_536;

/// Cells a single brush may touch. Matches #243.
pub const MAX_BRUSH_CELLS: u32 = 4096;

const MAX_HEIGHT: usize = 256;
const MAX_DENSITY: usize = 1024;

/// Q32.32. `from_int(1)` is `2^32` raw. Subset of `astraweave_math::Fixed64`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fixed64(i64);

impl Fixed64 {
    pub const SCALE: i64 = 1 << 32;
    pub const ZERO: Self = Self(0);

    #[inline]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> i64 {
        self.0
    }

    #[inline]
    pub const fn from_int(v: i64) -> Self {
        Self(v.saturating_mul(Self::SCALE))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestructionCommand {
    pub tick: u64,
    pub seq: u32,
    pub entity_id: u64,
    pub kind: CommandKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum CommandKind {
    Impact(Impact),
    Explosion(Explosion),
    Brush(Brush),
    Detach(Detach),
    Anchor(Anchor),
    SpawnNode(SpawnNode),
    SpawnBond(SpawnBond),
    Weave(Weave),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Impact {
    pub point: [Fixed64; 3],
    pub impulse: Fixed64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Explosion {
    pub center: [Fixed64; 3],
    pub radius: Fixed64,
    pub impulse: Fixed64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrushShape {
    Sphere,
    Box,
    Capsule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrushOp {
    Subtract,
    Add,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brush {
    pub shape: BrushShape,
    pub op: BrushOp,
    pub origin: [i32; 3],
    pub radius: u16,
    pub half_extents: [u16; 3],
    pub strength: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detach {
    pub bond_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    pub node_id: u64,
    pub anchored: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnNode {
    pub structure: u64,
    pub mass: Fixed64,
    pub pos: [Fixed64; 3],
    pub half: [Fixed64; 3],
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnBond {
    pub a: u64,
    pub b: u64,
    pub strength: Fixed64,
    pub area: Fixed64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weave {
    pub origin: [i32; 3],
    pub radius: u16,
    pub density_delta: i8,
    pub decay: Fixed64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    ZeroExtent,
    BrushTooLarge,
    LogFull,
}

pub fn validate(cmd: &DestructionCommand) -> Result<(), CommandError> {
    match cmd.kind {
        CommandKind::Brush(brush) => validate_brush(brush),
        CommandKind::Explosion(explosion) if explosion.radius.raw() <= 0 => {
            Err(CommandError::ZeroExtent)
        }
        CommandKind::Weave(weave) if weave.radius == 0 => Err(CommandError::ZeroExtent),
        _ => Ok(()),
    }
}

fn validate_brush(brush: Brush) -> Result<(), CommandError> {
    let cells = match brush.shape {
        BrushShape::Sphere | BrushShape::Capsule => {
            let d = u32::from(brush.radius).saturating_mul(2).saturating_add(1);
            d.saturating_mul(d).saturating_mul(d)
        }
        BrushShape::Box => {
            let x = u32::from(brush.half_extents[0])
                .saturating_mul(2)
                .saturating_add(1);
            let y = u32::from(brush.half_extents[1])
                .saturating_mul(2)
                .saturating_add(1);
            let z = u32::from(brush.half_extents[2])
                .saturating_mul(2)
                .saturating_add(1);
            x.saturating_mul(y).saturating_mul(z)
        }
    };
    if cells == 0
        || (brush.radius == 0 && matches!(brush.shape, BrushShape::Sphere | BrushShape::Capsule))
    {
        return Err(CommandError::ZeroExtent);
    }
    if cells > MAX_BRUSH_CELLS {
        return Err(CommandError::BrushTooLarge);
    }
    Ok(())
}

pub fn push_command(out: &mut Vec<u8>, cmd: &DestructionCommand) {
    out.extend_from_slice(&cmd.tick.to_le_bytes());
    out.extend_from_slice(&cmd.seq.to_le_bytes());
    out.extend_from_slice(&cmd.entity_id.to_le_bytes());
    match cmd.kind {
        CommandKind::Impact(impact) => {
            out.push(1);
            push_fixed3(out, impact.point);
            push_fixed(out, impact.impulse);
        }
        CommandKind::Explosion(explosion) => {
            out.push(2);
            push_fixed3(out, explosion.center);
            push_fixed(out, explosion.radius);
            push_fixed(out, explosion.impulse);
        }
        CommandKind::Brush(brush) => {
            out.push(3);
            out.push(match brush.shape {
                BrushShape::Sphere => 1,
                BrushShape::Box => 2,
                BrushShape::Capsule => 3,
            });
            out.push(match brush.op {
                BrushOp::Subtract => 1,
                BrushOp::Add => 2,
                BrushOp::Smooth => 3,
            });
            for value in brush.origin {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&brush.radius.to_le_bytes());
            for value in brush.half_extents {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&brush.strength.to_le_bytes());
        }
        CommandKind::Detach(detach) => {
            out.push(4);
            out.extend_from_slice(&detach.bond_id.to_le_bytes());
        }
        CommandKind::Anchor(anchor) => {
            out.push(5);
            out.extend_from_slice(&anchor.node_id.to_le_bytes());
            out.push(u8::from(anchor.anchored));
        }
        CommandKind::SpawnNode(node) => {
            out.push(6);
            out.extend_from_slice(&node.structure.to_le_bytes());
            push_fixed(out, node.mass);
            push_fixed3(out, node.pos);
            push_fixed3(out, node.half);
            out.extend_from_slice(&node.flags.to_le_bytes());
        }
        CommandKind::SpawnBond(bond) => {
            out.push(7);
            out.extend_from_slice(&bond.a.to_le_bytes());
            out.extend_from_slice(&bond.b.to_le_bytes());
            push_fixed(out, bond.strength);
            push_fixed(out, bond.area);
        }
        CommandKind::Weave(weave) => {
            out.push(8);
            for value in weave.origin {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&weave.radius.to_le_bytes());
            out.extend_from_slice(&weave.density_delta.to_le_bytes());
            push_fixed(out, weave.decay);
        }
    }
}

fn push_fixed(out: &mut Vec<u8>, value: Fixed64) {
    out.extend_from_slice(&value.raw().to_le_bytes());
}

fn push_fixed3(out: &mut Vec<u8>, value: [Fixed64; 3]) {
    for axis in value {
        push_fixed(out, axis);
    }
}

pub trait TerrainBase {
    fn column_top(&self, x: i32, z: i32) -> i32;
    fn column_material(&self, x: i32, y: i32, z: i32) -> u16;
}

/// Seed columns. The same `(x, z)` regenerates the same top and material.
#[derive(Clone, Copy, Debug, Default)]
pub struct SeedColumnBase;

impl TerrainBase for SeedColumnBase {
    fn column_top(&self, x: i32, z: i32) -> i32 {
        surface_height(x, z)
    }

    fn column_material(&self, x: i32, y: i32, z: i32) -> u16 {
        let top = surface_height(x, z);
        column_material(y, top, biome_at(x, z), on_yaqar_road(x, z))
    }
}

pub const CHUNK_EXTENT: i32 = 32;

#[derive(Clone, Copy)]
struct Node {
    children: [u16; 8],
    density: i8,
    leaf: bool,
}

const MAX_NODES: usize = 2048;

/// Content-addressed octree for one edited chunk. Node 0 is unused. Node 1
/// is the shared empty leaf.
#[derive(Clone)]
pub struct VoxelDag {
    nodes: [Node; MAX_NODES],
    keys: [u64; MAX_NODES],
    len: u16,
    root: u16,
    origin: [i32; 3],
    ready: bool,
}

impl VoxelDag {
    pub fn new() -> Self {
        let mut dag = Self {
            nodes: [Node {
                children: [0; 8],
                density: 0,
                leaf: true,
            }; MAX_NODES],
            keys: [0; MAX_NODES],
            len: 2,
            root: 1,
            origin: [0, 0, 0],
            ready: false,
        };
        dag.nodes[1].leaf = true;
        dag.nodes[1].density = 0;
        dag.keys[1] = leaf_key(0);
        dag
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn root(&self) -> u16 {
        self.root
    }

    pub fn node_len(&self) -> u16 {
        self.len
    }

    pub fn rebuild<F>(&mut self, origin: [i32; 3], mut sample: F)
    where
        F: FnMut(i32, i32, i32) -> Option<i8>,
    {
        self.len = 2;
        self.origin = origin;
        self.root = self.build(origin[0], origin[1], origin[2], CHUNK_EXTENT, &mut sample);
        self.ready = true;
    }

    pub fn solid_at<F>(&self, x: i32, y: i32, z: i32, mut base_solid: F) -> bool
    where
        F: FnMut(i32, i32, i32) -> bool,
    {
        if !self.ready {
            return base_solid(x, y, z);
        }
        let local_x = x - self.origin[0];
        let local_y = y - self.origin[1];
        let local_z = z - self.origin[2];
        if !(0..CHUNK_EXTENT).contains(&local_x)
            || !(0..CHUNK_EXTENT).contains(&local_y)
            || !(0..CHUNK_EXTENT).contains(&local_z)
        {
            return base_solid(x, y, z);
        }
        match self.density_at(local_x, local_y, local_z) {
            Some(density) => density > 0,
            None => base_solid(x, y, z),
        }
    }

    fn density_at(&self, mut x: i32, mut y: i32, mut z: i32) -> Option<i8> {
        let mut index = self.root;
        let mut size = CHUNK_EXTENT;
        loop {
            if index == 0 {
                return None;
            }
            let node = self.nodes[index as usize];
            if node.leaf {
                return Some(node.density);
            }
            size /= 2;
            if size == 0 {
                return Some(node.density);
            }
            let oct = octant(x, y, z, size);
            x %= size;
            y %= size;
            z %= size;
            let child = node.children[oct];
            if child == 0 {
                return None;
            }
            index = child;
        }
    }

    fn build<F>(&mut self, x: i32, y: i32, z: i32, size: i32, sample: &mut F) -> u16
    where
        F: FnMut(i32, i32, i32) -> Option<i8>,
    {
        if size <= 1 {
            return match sample(x, y, z) {
                Some(density) => self.intern_leaf(density),
                None => 0,
            };
        }
        let half = size / 2;
        let mut children = [0u16; 8];
        let mut any = false;
        for (oct, child_out) in children.iter_mut().enumerate() {
            let (ox, oy, oz) = oct_offset(oct, half);
            let child = self.build(x + ox, y + oy, z + oz, half, sample);
            *child_out = child;
            if child != 0 {
                any = true;
            }
        }
        if !any {
            return 0;
        }
        self.intern_branch(children)
    }

    fn intern_leaf(&mut self, density: i8) -> u16 {
        let key = leaf_key(density);
        if let Some(found) = self.find(key) {
            return found;
        }
        self.push(
            key,
            Node {
                children: [0; 8],
                density,
                leaf: true,
            },
        )
    }

    fn intern_branch(&mut self, children: [u16; 8]) -> u16 {
        let key = branch_key(children);
        if let Some(found) = self.find(key) {
            return found;
        }
        self.push(
            key,
            Node {
                children,
                density: 0,
                leaf: false,
            },
        )
    }

    fn find(&self, key: u64) -> Option<u16> {
        let mut index = 1u16;
        while index < self.len {
            if self.keys[index as usize] == key {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn push(&mut self, key: u64, node: Node) -> u16 {
        if self.len as usize >= MAX_NODES {
            return 1;
        }
        let index = self.len;
        self.nodes[index as usize] = node;
        self.keys[index as usize] = key;
        self.len += 1;
        index
    }
}

impl Default for VoxelDag {
    fn default() -> Self {
        Self::new()
    }
}

fn leaf_key(density: i8) -> u64 {
    0x1000_0000_0000_0000 | u64::from(density as u8)
}

fn branch_key(children: [u16; 8]) -> u64 {
    let mut key = 0x2000_0000_0000_0000u64;
    for (shift, child) in children.iter().enumerate() {
        key ^= u64::from(*child)
            .wrapping_mul(0x9E37_79B9)
            .rotate_left((shift as u32) * 7);
    }
    key
}

fn octant(x: i32, y: i32, z: i32, half: i32) -> usize {
    let mut oct = 0;
    if x >= half {
        oct |= 1;
    }
    if y >= half {
        oct |= 2;
    }
    if z >= half {
        oct |= 4;
    }
    oct
}

fn oct_offset(oct: usize, half: i32) -> (i32, i32, i32) {
    (
        if oct & 1 == 1 { half } else { 0 },
        if oct & 2 == 2 { half } else { 0 },
        if oct & 4 == 4 { half } else { 0 },
    )
}

#[derive(Clone, Copy)]
struct HeightCell {
    x: i32,
    z: i32,
    delta: i32,
}

#[derive(Clone, Copy)]
struct DensityCell {
    x: i32,
    y: i32,
    z: i32,
    density: i8,
}

#[derive(Clone, Copy)]
struct Occupancy {
    height: [HeightCell; MAX_HEIGHT],
    height_len: u16,
    density: [DensityCell; MAX_DENSITY],
    density_len: u16,
}

impl Occupancy {
    fn empty() -> Self {
        Self {
            height: [HeightCell {
                x: 0,
                z: 0,
                delta: 0,
            }; MAX_HEIGHT],
            height_len: 0,
            density: [DensityCell {
                x: 0,
                y: 0,
                z: 0,
                density: 0,
            }; MAX_DENSITY],
            density_len: 0,
        }
    }

    fn height_delta(&self, x: i32, z: i32) -> i32 {
        for index in 0..self.height_len as usize {
            let cell = self.height[index];
            if cell.x == x && cell.z == z {
                return cell.delta;
            }
        }
        0
    }

    fn add_height(&mut self, x: i32, z: i32, step: i32, subtract: bool) {
        let mut at = self.height_len as usize;
        for index in 0..self.height_len as usize {
            let cell = self.height[index];
            if cell.x == x && cell.z == z {
                if subtract {
                    self.height[index].delta = cell.delta.saturating_sub(step);
                } else {
                    self.height[index].delta = cell.delta.saturating_add(step);
                }
                return;
            }
            if (x, z) < (cell.x, cell.z) {
                at = index;
                break;
            }
        }
        if self.height_len as usize >= MAX_HEIGHT {
            return;
        }
        let len = self.height_len as usize;
        for index in (at..len).rev() {
            self.height[index + 1] = self.height[index];
        }
        let delta = if subtract {
            step.saturating_neg()
        } else {
            step
        };
        self.height[at] = HeightCell { x, z, delta };
        self.height_len += 1;
    }

    fn density_at(&self, x: i32, y: i32, z: i32) -> Option<i8> {
        for index in 0..self.density_len as usize {
            let cell = self.density[index];
            if cell.x == x && cell.y == y && cell.z == z {
                return Some(cell.density);
            }
        }
        None
    }

    fn set_density(&mut self, x: i32, y: i32, z: i32, value: i8) {
        let mut at = self.density_len as usize;
        for index in 0..self.density_len as usize {
            let cell = self.density[index];
            if cell.x == x && cell.y == y && cell.z == z {
                self.density[index].density = value;
                return;
            }
            if (x, y, z) < (cell.x, cell.y, cell.z) {
                at = index;
                break;
            }
        }
        if self.density_len as usize >= MAX_DENSITY {
            return;
        }
        let len = self.density_len as usize;
        for index in (at..len).rev() {
            self.density[index + 1] = self.density[index];
        }
        self.density[at] = DensityCell {
            x,
            y,
            z,
            density: value,
        };
        self.density_len += 1;
    }
}

pub(crate) struct SeamSlot {
    coord: (i32, i32),
    occ: Occupancy,
    dag: VoxelDag,
}

impl SeamSlot {
    pub(crate) fn empty() -> Self {
        Self {
            coord: (0, 0),
            occ: Occupancy::empty(),
            dag: VoxelDag::new(),
        }
    }

    pub(crate) fn prepare(&mut self, coord: (i32, i32)) {
        self.coord = coord;
        self.occ = Occupancy::empty();
        self.dag = VoxelDag::new();
    }

    pub(crate) fn replay(&mut self, coord: (i32, i32), log: &[DestructionCommand]) {
        self.coord = coord;
        self.occ = Occupancy::empty();
        for cmd in log {
            match cmd.kind {
                CommandKind::Brush(brush) if brush_touches(brush, coord) => {
                    apply_one(&mut self.occ, brush);
                }
                CommandKind::Weave(weave)
                    if chunk_of(weave.origin[0], weave.origin[2]) == coord =>
                {
                    apply_weave(&mut self.occ, weave);
                }
                _ => {}
            }
        }
        let density = self.occ.density;
        let density_len = self.occ.density_len;
        let origin = [coord.0 * CHUNK_EXTENT, 0, coord.1 * CHUNK_EXTENT];
        self.dag.rebuild(origin, move |x, y, z| {
            for cell in &density[..density_len as usize] {
                if cell.x == x && cell.y == y && cell.z == z {
                    return Some(cell.density);
                }
            }
            None
        });
    }

    pub(crate) fn height_delta(&self, x: i32, z: i32) -> i32 {
        self.occ.height_delta(x, z)
    }

    pub(crate) fn dag(&self) -> &VoxelDag {
        &self.dag
    }

    pub(crate) fn write_occupancy(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.coord.0.to_le_bytes());
        out.extend_from_slice(&self.coord.1.to_le_bytes());
        out.extend_from_slice(&self.occ.height_len.to_le_bytes());
        for index in 0..self.occ.height_len as usize {
            let cell = self.occ.height[index];
            out.extend_from_slice(&cell.x.to_le_bytes());
            out.extend_from_slice(&cell.z.to_le_bytes());
            out.extend_from_slice(&cell.delta.to_le_bytes());
        }
        out.extend_from_slice(&self.occ.density_len.to_le_bytes());
        for index in 0..self.occ.density_len as usize {
            let cell = self.occ.density[index];
            out.extend_from_slice(&cell.x.to_le_bytes());
            out.extend_from_slice(&cell.y.to_le_bytes());
            out.extend_from_slice(&cell.z.to_le_bytes());
            out.extend_from_slice(&cell.density.to_le_bytes());
        }
        out.extend_from_slice(&self.dag.root().to_le_bytes());
        out.extend_from_slice(&self.dag.node_len().to_le_bytes());
    }
}

pub(crate) fn chunk_of(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(CHUNK_EXTENT), z.div_euclid(CHUNK_EXTENT))
}

pub(crate) fn command_touches(cmd: &DestructionCommand, chunk: (i32, i32)) -> bool {
    match cmd.kind {
        CommandKind::Brush(brush) => brush_touches(brush, chunk),
        CommandKind::Weave(weave) => chunk_of(weave.origin[0], weave.origin[2]) == chunk,
        _ => false,
    }
}

fn brush_touches(brush: Brush, chunk: (i32, i32)) -> bool {
    let mut touched = [(0i32, 0i32); 8];
    let count = touched_chunks(brush, &mut touched);
    touched[..count].contains(&chunk)
}

fn touched_chunks(brush: Brush, out: &mut [(i32, i32)]) -> usize {
    let reach = i32::from(brush.radius)
        .max(i32::from(brush.half_extents[0]))
        .max(i32::from(brush.half_extents[2]));
    let min_x = brush.origin[0] - reach;
    let max_x = brush.origin[0] + reach;
    let min_z = brush.origin[2] - reach;
    let max_z = brush.origin[2] + reach;
    let mut count = 0usize;
    let mut x = min_x;
    while x <= max_x {
        let mut z = min_z;
        while z <= max_z {
            push_unique(out, &mut count, chunk_of(x, z));
            z += CHUNK_EXTENT;
        }
        push_unique(out, &mut count, chunk_of(x, max_z));
        x += CHUNK_EXTENT;
    }
    push_unique(out, &mut count, chunk_of(max_x, max_z));
    count
}

fn push_unique(out: &mut [(i32, i32)], count: &mut usize, chunk: (i32, i32)) {
    if out[..*count].contains(&chunk) || *count >= out.len() {
        return;
    }
    out[*count] = chunk;
    *count += 1;
}

fn apply_one(occ: &mut Occupancy, brush: Brush) {
    match brush.op {
        BrushOp::Smooth => smooth(occ),
        BrushOp::Add | BrushOp::Subtract => match brush.shape {
            BrushShape::Sphere => sphere(occ, brush),
            BrushShape::Box => box_brush(occ, brush),
            BrushShape::Capsule => {
                sphere(occ, brush);
                box_brush(occ, brush);
            }
        },
    }
}

fn sphere(occ: &mut Occupancy, brush: Brush) {
    let radius = i32::from(brush.radius);
    if radius <= 0 {
        return;
    }
    let r2 = radius.saturating_mul(radius);
    let strength = i32::from(brush.strength).abs().max(1);
    let subtract = brush.op == BrushOp::Subtract;
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            if dx.saturating_mul(dx).saturating_add(dz.saturating_mul(dz)) > r2 {
                continue;
            }
            let x = brush.origin[0].saturating_add(dx);
            let z = brush.origin[2].saturating_add(dz);
            let fall = radius - dx.abs().max(dz.abs());
            let step = strength.saturating_mul(fall.max(0)) / radius.max(1);
            occ.add_height(x, z, step, subtract);
        }
    }
    let value = if subtract { 0 } else { 127 };
    for dy in -radius..=radius {
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx.saturating_mul(dx) + dy.saturating_mul(dy) + dz.saturating_mul(dz);
                if d2 > r2 {
                    continue;
                }
                occ.set_density(
                    brush.origin[0].saturating_add(dx),
                    brush.origin[1].saturating_add(dy),
                    brush.origin[2].saturating_add(dz),
                    value,
                );
            }
        }
    }
}

fn box_brush(occ: &mut Occupancy, brush: Brush) {
    let hx = i32::from(brush.half_extents[0]);
    let hy = i32::from(brush.half_extents[1]);
    let hz = i32::from(brush.half_extents[2]);
    let step = i32::from(brush.strength).abs().max(1);
    let subtract = brush.op == BrushOp::Subtract;
    for dz in -hz..=hz {
        for dx in -hx..=hx {
            occ.add_height(
                brush.origin[0].saturating_add(dx),
                brush.origin[2].saturating_add(dz),
                step,
                subtract,
            );
        }
    }
    let value = if subtract { 0 } else { 127 };
    for dy in -hy..=hy {
        for dz in -hz..=hz {
            for dx in -hx..=hx {
                occ.set_density(
                    brush.origin[0].saturating_add(dx),
                    brush.origin[1].saturating_add(dy),
                    brush.origin[2].saturating_add(dz),
                    value,
                );
            }
        }
    }
}

fn smooth(occ: &mut Occupancy) {
    let mut next = [0i8; MAX_DENSITY];
    let len = occ.density_len as usize;
    for (index, cell) in occ.density[..len].iter().copied().enumerate() {
        let here = i32::from(cell.density);
        let left = i32::from(
            occ.density_at(cell.x - 1, cell.y, cell.z)
                .unwrap_or(cell.density),
        );
        let right = i32::from(
            occ.density_at(cell.x + 1, cell.y, cell.z)
                .unwrap_or(cell.density),
        );
        next[index] = ((here + left + right) / 3).clamp(0, 127) as i8;
    }
    for (cell, averaged) in occ.density[..len].iter_mut().zip(next) {
        cell.density = averaged;
    }
}

fn apply_weave(occ: &mut Occupancy, weave: Weave) {
    let radius = i32::from(weave.radius);
    if radius <= 0 {
        return;
    }
    let r2 = radius.saturating_mul(radius);
    for dz in -radius..=radius {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx.saturating_mul(dx) + dy.saturating_mul(dy) + dz.saturating_mul(dz);
                if d2 > r2 {
                    continue;
                }
                let x = weave.origin[0].saturating_add(dx);
                let y = weave.origin[1].saturating_add(dy);
                let z = weave.origin[2].saturating_add(dz);
                let current = occ.density_at(x, y, z).unwrap_or(0);
                let next =
                    (i32::from(current) + i32::from(weave.density_delta)).clamp(-128, 127) as i8;
                occ.set_density(x, y, z, next);
            }
        }
    }
}

/// One-column height edit. A box of half-extent 0 changes that column by `strength`.
pub fn column_step_brush(x: i32, y: i32, z: i32, subtract: bool) -> Brush {
    Brush {
        shape: BrushShape::Box,
        op: if subtract {
            BrushOp::Subtract
        } else {
            BrushOp::Add
        },
        origin: [x, y, z],
        radius: 0,
        half_extents: [0, 0, 0],
        strength: 1,
    }
}
