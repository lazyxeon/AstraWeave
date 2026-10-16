//! Column ring deformation. Command types, `Fixed64`, and `VoxelDag` are the
//! ones on main (`astraweave-destruction`, `astraweave-math`). This module
//! keeps the ring's occupancy replay, which the destruction crate does not
//! export: its `Terrain` slot set is four chunks, and the loaded ring is the
//! slot set.

pub use astraweave_destruction::budget::{MAX_LOG_COMMANDS, MAX_REMESHES_PER_TICK};
pub use astraweave_destruction::command::{
    push_command, validate, Brush, BrushOp, BrushShape, CommandError, CommandKind,
    DestructionCommand, Detach, Weave,
};
pub use astraweave_destruction::octree::{VoxelDag, CHUNK_EXTENT};
pub use astraweave_destruction::terrain::TerrainBase;

use crate::column_gen::{biome_at, column_material, on_yaqar_road, surface_height};

const MAX_HEIGHT: usize = 256;
const MAX_DENSITY: usize = 1024;

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
