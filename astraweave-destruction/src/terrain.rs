//! Integer terrain edits on a fixed slab, plus a sparse voxel DAG.
//!
//! The base column is flat until PR #215 lands (issue #249). Brush replay
//! does not allocate: edits sit in fixed arrays. The DAG is rebuilt from
//! those arrays when the chunk deforms.

use crate::column_base::ColumnFns;
use crate::command::{Brush, BrushOp, BrushShape};
use crate::octree::{VoxelDag, CHUNK_EXTENT};

pub const CHUNK: i32 = CHUNK_EXTENT;

const MAX_CHUNKS: usize = 4;
const MAX_BRUSHES: usize = 16;
const MAX_HEIGHT: usize = 256;
const MAX_DENSITY: usize = 1024;

pub trait TerrainBase {
    fn column_top(&self, x: i32, z: i32) -> i32;
    fn column_material(&self, x: i32, y: i32, z: i32) -> u16;
}

/// Stand-in for #215's integer columns. Every column has the same top.
#[derive(Clone, Copy, Debug)]
pub struct FlatColumnBase {
    pub top: i32,
}

impl TerrainBase for FlatColumnBase {
    fn column_top(&self, _x: i32, _z: i32) -> i32 {
        self.top
    }

    fn column_material(&self, y: i32, _x: i32, _z: i32) -> u16 {
        if y <= self.top {
            1
        } else {
            0
        }
    }
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

struct ChunkSlot {
    coord: (i32, i32),
    used: bool,
    loaded: bool,
    brushes: [Brush; MAX_BRUSHES],
    brush_len: u8,
    occ: Occupancy,
    dag: VoxelDag,
}

impl ChunkSlot {
    fn empty() -> Self {
        Self {
            coord: (0, 0),
            used: false,
            loaded: false,
            brushes: [Brush {
                shape: BrushShape::Sphere,
                op: BrushOp::Subtract,
                origin: [0, 0, 0],
                radius: 1,
                half_extents: [1, 1, 1],
                strength: 1,
            }; MAX_BRUSHES],
            brush_len: 0,
            occ: Occupancy::empty(),
            dag: VoxelDag::new(),
        }
    }
}

pub struct Terrain {
    base: FlatColumnBase,
    /// When set, `surface_y` regenerates the seed column instead of the flat top.
    columns: Option<ColumnFns>,
    slots: Box<[ChunkSlot; MAX_CHUNKS]>,
    remesh: [(i32, i32); 8],
    remesh_len: usize,
}

impl Terrain {
    pub fn new(top: i32) -> Self {
        Self {
            base: FlatColumnBase { top },
            columns: None,
            slots: Box::new([
                ChunkSlot::empty(),
                ChunkSlot::empty(),
                ChunkSlot::empty(),
                ChunkSlot::empty(),
            ]),
            remesh: [(0, 0); 8],
            remesh_len: 0,
        }
    }

    pub fn base_top(&self) -> i32 {
        self.base.top
    }

    /// Use the caller's integer columns. `None` keeps [`FlatColumnBase`].
    pub fn set_column_fns(&mut self, fns: ColumnFns) {
        self.columns = Some(fns);
    }

    pub fn load(&mut self, chunk: (i32, i32)) {
        let base = self.base;
        let slot = self.slot_mut(chunk);
        slot.loaded = true;
        rebuild_slot(&base, slot);
    }

    pub fn unload(&mut self, chunk: (i32, i32)) {
        if let Some(slot) = self.find_mut(chunk) {
            slot.loaded = false;
            slot.occ = Occupancy::empty();
            slot.dag = VoxelDag::new();
        }
    }

    pub fn is_loaded(&self, chunk: (i32, i32)) -> bool {
        self.find(chunk).is_some_and(|slot| slot.loaded)
    }

    pub fn apply_brush(&mut self, brush: Brush) {
        let mut chunks = [(0i32, 0i32); 4];
        let count = touched_chunks(brush, &mut chunks);
        for index in 0..count {
            let chunk = chunks[index];
            {
                let base = self.base;
                let slot = self.slot_mut(chunk);
                if (slot.brush_len as usize) < MAX_BRUSHES {
                    slot.brushes[slot.brush_len as usize] = brush;
                    slot.brush_len += 1;
                }
                if slot.loaded {
                    rebuild_slot(&base, slot);
                }
            }
            self.push_remesh(chunk);
        }
    }

    pub fn apply_density_delta(&mut self, origin: [i32; 3], radius: i32, delta: i8) {
        if radius <= 0 {
            return;
        }
        let chunk = chunk_of(origin[0], origin[2]);
        let base = self.base;
        {
            let slot = self.slot_mut(chunk);
            if slot.loaded {
                let r2 = radius.saturating_mul(radius);
                for dz in -radius..=radius {
                    for dy in -radius..=radius {
                        for dx in -radius..=radius {
                            let d2 = dx.saturating_mul(dx)
                                + dy.saturating_mul(dy)
                                + dz.saturating_mul(dz);
                            if d2 > r2 {
                                continue;
                            }
                            let x = origin[0].saturating_add(dx);
                            let y = origin[1].saturating_add(dy);
                            let z = origin[2].saturating_add(dz);
                            let current = slot.occ.density_at(x, y, z).unwrap_or(0);
                            let next =
                                (i32::from(current) + i32::from(delta)).clamp(-128, 127) as i8;
                            slot.occ.set_density(x, y, z, next);
                        }
                    }
                }
                rebuild_dag(&base, slot);
            }
        }
        self.push_remesh(chunk);
    }

    pub fn take_remesh(&mut self, limit: usize) -> usize {
        let count = limit.min(self.remesh_len);
        self.remesh_len -= count;
        count
    }

    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        let chunk = chunk_of(x, z);
        let delta = self
            .find(chunk)
            .filter(|slot| slot.loaded)
            .map(|slot| slot.occ.height_delta(x, z))
            .unwrap_or(0);
        let top = match self.columns {
            Some(fns) => fns.top(x, z),
            None => self.base.column_top(x, z),
        };
        top.saturating_add(delta)
    }

    pub fn dag(&self, chunk: (i32, i32)) -> Option<&VoxelDag> {
        self.find(chunk)
            .filter(|slot| slot.loaded)
            .map(|slot| &slot.dag)
    }

    pub fn occupancy_bytes(&self, chunk: (i32, i32)) -> Vec<u8> {
        let Some(slot) = self.find(chunk).filter(|slot| slot.loaded) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        write_occupancy(slot, &mut out);
        out
    }

    pub fn write_loaded(&self, out: &mut Vec<u8>) {
        for slot in self.slots.iter() {
            if slot.used && slot.loaded {
                write_occupancy(slot, out);
            }
        }
    }

    fn slot_mut(&mut self, chunk: (i32, i32)) -> &mut ChunkSlot {
        if let Some(index) = self
            .slots
            .iter()
            .position(|slot| slot.used && slot.coord == chunk)
        {
            return &mut self.slots[index];
        }
        let index = self.slots.iter().position(|slot| !slot.used).unwrap_or(0);
        self.slots[index] = ChunkSlot::empty();
        self.slots[index].used = true;
        self.slots[index].coord = chunk;
        &mut self.slots[index]
    }

    fn find(&self, chunk: (i32, i32)) -> Option<&ChunkSlot> {
        self.slots
            .iter()
            .find(|slot| slot.used && slot.coord == chunk)
    }

    fn find_mut(&mut self, chunk: (i32, i32)) -> Option<&mut ChunkSlot> {
        self.slots
            .iter_mut()
            .find(|slot| slot.used && slot.coord == chunk)
    }

    fn push_remesh(&mut self, chunk: (i32, i32)) {
        if self.remesh[..self.remesh_len].contains(&chunk) {
            return;
        }
        if self.remesh_len < self.remesh.len() {
            self.remesh[self.remesh_len] = chunk;
            self.remesh_len += 1;
        }
    }
}

fn write_occupancy(slot: &ChunkSlot, out: &mut Vec<u8>) {
    out.extend_from_slice(&slot.coord.0.to_le_bytes());
    out.extend_from_slice(&slot.coord.1.to_le_bytes());
    for index in 0..slot.occ.height_len as usize {
        let cell = slot.occ.height[index];
        out.extend_from_slice(&cell.x.to_le_bytes());
        out.extend_from_slice(&cell.z.to_le_bytes());
        out.extend_from_slice(&cell.delta.to_le_bytes());
    }
    for index in 0..slot.occ.density_len as usize {
        let cell = slot.occ.density[index];
        out.extend_from_slice(&cell.x.to_le_bytes());
        out.extend_from_slice(&cell.y.to_le_bytes());
        out.extend_from_slice(&cell.z.to_le_bytes());
        out.extend_from_slice(&cell.density.to_le_bytes());
    }
}

fn rebuild_slot(base: &FlatColumnBase, slot: &mut ChunkSlot) {
    slot.occ = Occupancy::empty();
    let len = slot.brush_len as usize;
    for index in 0..len {
        let brush = slot.brushes[index];
        apply_one(base, &mut slot.occ, brush);
    }
    rebuild_dag(base, slot);
}

fn rebuild_dag(base: &FlatColumnBase, slot: &mut ChunkSlot) {
    let origin = [slot.coord.0 * CHUNK, 0, slot.coord.1 * CHUNK];
    let occ_height = slot.occ.height_len;
    let occ_density = slot.occ.density_len;
    // Split borrows: sample through indices copied into the dag callback via raw reads.
    let height = slot.occ.height;
    let height_len = occ_height;
    let density = slot.occ.density;
    let density_len = occ_density;
    let _ = base;
    slot.dag.rebuild(origin, move |x, y, z| {
        for index in 0..density_len as usize {
            let cell = density[index];
            if cell.x == x && cell.y == y && cell.z == z {
                return Some(cell.density);
            }
        }
        let _ = (height, height_len);
        None
    });
}

fn apply_one(_base: &FlatColumnBase, occ: &mut Occupancy, brush: Brush) {
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
    for index in 0..len {
        let cell = occ.density[index];
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
    for index in 0..len {
        occ.density[index].density = next[index];
    }
}

fn chunk_of(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(CHUNK), z.div_euclid(CHUNK))
}

fn touched_chunks(brush: Brush, out: &mut [(i32, i32)]) -> usize {
    let radius = i32::from(brush.radius.max(brush.half_extents[0]));
    let min_x = brush.origin[0] - radius;
    let max_x = brush.origin[0] + radius;
    let min_z = brush.origin[2] - radius;
    let max_z = brush.origin[2] + radius;
    let mut count = 0usize;
    let mut x = min_x;
    while x <= max_x {
        let mut z = min_z;
        while z <= max_z {
            push_unique(out, &mut count, chunk_of(x, z));
            z += CHUNK;
        }
        push_unique(out, &mut count, chunk_of(x, max_z));
        x += CHUNK;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dig() -> Brush {
        Brush {
            shape: BrushShape::Sphere,
            op: BrushOp::Subtract,
            origin: [4, 2, 4],
            radius: 3,
            half_extents: [1, 1, 1],
            strength: 4,
        }
    }

    #[test]
    fn unload_then_load_matches_the_edited_occupancy() {
        let mut terrain = Terrain::new(4);
        terrain.load((0, 0));
        terrain.apply_brush(dig());
        let before = terrain.occupancy_bytes((0, 0));
        assert!(terrain.surface_y(4, 4) < 4);
        terrain.unload((0, 0));
        assert!(terrain.occupancy_bytes((0, 0)).is_empty());
        terrain.load((0, 0));
        assert_eq!(terrain.occupancy_bytes((0, 0)), before);
    }

    #[test]
    fn fill_raises_a_column() {
        let mut terrain = Terrain::new(4);
        terrain.load((0, 0));
        terrain.apply_brush(Brush {
            shape: BrushShape::Box,
            op: BrushOp::Add,
            origin: [2, 0, 2],
            radius: 1,
            half_extents: [1, 1, 1],
            strength: 3,
        });
        assert!(terrain.surface_y(2, 2) > 4);
    }

    #[test]
    fn dag_marks_a_dug_cell_empty() {
        let mut terrain = Terrain::new(4);
        terrain.load((0, 0));
        terrain.apply_brush(dig());
        let dag = terrain.dag((0, 0)).expect("dag");
        assert!(dag.ready());
        assert!(!dag.solid_at(4, 2, 4, |_, _, _| true));
    }

    #[test]
    fn seeded_columns_replay_the_same_occupancy() {
        fn top(x: i32, z: i32) -> i32 {
            6 + (x + z).rem_euclid(3)
        }
        fn material(_x: i32, y: i32, _z: i32) -> u16 {
            u16::from(y <= top(0, 0))
        }
        let mut terrain = Terrain::new(0);
        terrain.set_column_fns(ColumnFns { top, material });
        let seed = top(4, 4);
        terrain.load((0, 0));
        terrain.apply_brush(dig());
        assert!(terrain.surface_y(4, 4) < seed);
        let before = terrain.occupancy_bytes((0, 0));
        terrain.unload((0, 0));
        terrain.load((0, 0));
        assert_eq!(terrain.occupancy_bytes((0, 0)), before);
        assert_eq!(terrain.surface_y(0, 0), top(0, 0));
    }
}
