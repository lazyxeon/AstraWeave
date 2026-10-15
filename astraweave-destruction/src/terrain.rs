//! Integer terrain edits. The base column is flat until PR #215 lands.
//!
//! The column adapter that should call `surface_height` is not compiled in.
//! Issue #249 tracks that adapter. Collision and the hash read these
//! integers, never a display mesh.

use std::collections::{BTreeMap, BTreeSet};

use crate::command::{Brush, BrushOp, BrushShape};

pub const CHUNK: i32 = 32;

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ChunkLog {
    brushes: Vec<Brush>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Occupancy {
    height: BTreeMap<(i32, i32), i32>,
    density: BTreeMap<(i32, i32, i32), i8>,
}

#[derive(Clone, Debug)]
pub struct Terrain {
    base: FlatColumnBase,
    logs: BTreeMap<(i32, i32), ChunkLog>,
    cache: BTreeMap<(i32, i32), Occupancy>,
    loaded: BTreeSet<(i32, i32)>,
    remesh: Vec<(i32, i32)>,
}

impl Terrain {
    pub fn new(top: i32) -> Self {
        Self {
            base: FlatColumnBase { top },
            logs: BTreeMap::new(),
            cache: BTreeMap::new(),
            loaded: BTreeSet::new(),
            remesh: Vec::new(),
        }
    }

    pub fn load(&mut self, chunk: (i32, i32)) {
        self.loaded.insert(chunk);
        self.cache
            .insert(chunk, rebuild(&self.base, self.logs.get(&chunk)));
    }

    pub fn unload(&mut self, chunk: (i32, i32)) {
        self.loaded.remove(&chunk);
        self.cache.remove(&chunk);
    }

    pub fn is_loaded(&self, chunk: (i32, i32)) -> bool {
        self.loaded.contains(&chunk)
    }

    pub fn apply_brush(&mut self, brush: Brush) {
        let chunks = touched_chunks(brush);
        for chunk in chunks {
            self.logs.entry(chunk).or_default().brushes.push(brush);
            if self.loaded.contains(&chunk) {
                self.cache
                    .insert(chunk, rebuild(&self.base, self.logs.get(&chunk)));
                if !self.remesh.contains(&chunk) {
                    self.remesh.push(chunk);
                }
            }
        }
        self.remesh.sort_unstable();
    }

    pub fn take_remesh(&mut self, limit: usize) -> Vec<(i32, i32)> {
        let count = limit.min(self.remesh.len());
        let taken: Vec<(i32, i32)> = self.remesh.drain(..count).collect();
        taken
    }

    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        let chunk = chunk_of(x, z);
        let delta = self
            .cache
            .get(&chunk)
            .and_then(|occ| occ.height.get(&(x, z)))
            .copied()
            .unwrap_or(0);
        self.base.column_top(x, z).saturating_add(delta)
    }

    pub fn occupancy_bytes(&self, chunk: (i32, i32)) -> Vec<u8> {
        let Some(occ) = self.cache.get(&chunk) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        out.extend_from_slice(&chunk.0.to_le_bytes());
        out.extend_from_slice(&chunk.1.to_le_bytes());
        for ((x, z), delta) in &occ.height {
            out.extend_from_slice(&x.to_le_bytes());
            out.extend_from_slice(&z.to_le_bytes());
            out.extend_from_slice(&delta.to_le_bytes());
        }
        for ((x, y, z), density) in &occ.density {
            out.extend_from_slice(&x.to_le_bytes());
            out.extend_from_slice(&y.to_le_bytes());
            out.extend_from_slice(&z.to_le_bytes());
            out.extend_from_slice(&density.to_le_bytes());
        }
        out
    }

    pub fn loaded_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in &self.loaded {
            out.extend(self.occupancy_bytes(*chunk));
        }
        out
    }
}

fn chunk_of(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(CHUNK), z.div_euclid(CHUNK))
}

fn touched_chunks(brush: Brush) -> Vec<(i32, i32)> {
    let radius = i32::from(brush.radius.max(brush.half_extents[0]));
    let mut chunks = BTreeSet::new();
    let min_x = brush.origin[0] - radius;
    let max_x = brush.origin[0] + radius;
    let min_z = brush.origin[2] - radius;
    let max_z = brush.origin[2] + radius;
    let mut x = min_x;
    while x <= max_x {
        let mut z = min_z;
        while z <= max_z {
            chunks.insert(chunk_of(x, z));
            z += CHUNK;
        }
        chunks.insert(chunk_of(x, max_z));
        x += CHUNK;
    }
    chunks.insert(chunk_of(max_x, max_z));
    chunks.into_iter().collect()
}

fn rebuild(base: &FlatColumnBase, log: Option<&ChunkLog>) -> Occupancy {
    let mut occ = Occupancy::default();
    let Some(log) = log else {
        return occ;
    };
    for brush in &log.brushes {
        apply_one(base, &mut occ, *brush);
    }
    occ
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
    let r2 = radius.saturating_mul(radius);
    let strength = i32::from(brush.strength).abs().max(1);
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            if dx.saturating_mul(dx).saturating_add(dz.saturating_mul(dz)) > r2 {
                continue;
            }
            let x = brush.origin[0].saturating_add(dx);
            let z = brush.origin[2].saturating_add(dz);
            let fall = radius - dx.abs().max(dz.abs());
            let step = strength.saturating_mul(fall.max(0)) / radius.max(1);
            let entry = occ.height.entry((x, z)).or_insert(0);
            if brush.op == BrushOp::Subtract {
                *entry = entry.saturating_sub(step);
            } else {
                *entry = entry.saturating_add(step);
            }
        }
    }
    for dy in -radius..=radius {
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = dx.saturating_mul(dx) + dy.saturating_mul(dy) + dz.saturating_mul(dz);
                if d2 > r2 {
                    continue;
                }
                let x = brush.origin[0].saturating_add(dx);
                let y = brush.origin[1].saturating_add(dy);
                let z = brush.origin[2].saturating_add(dz);
                let value = if brush.op == BrushOp::Subtract {
                    0
                } else {
                    127
                };
                occ.density.insert((x, y, z), value);
            }
        }
    }
}

fn box_brush(occ: &mut Occupancy, brush: Brush) {
    let hx = i32::from(brush.half_extents[0]);
    let hy = i32::from(brush.half_extents[1]);
    let hz = i32::from(brush.half_extents[2]);
    let step = i32::from(brush.strength).abs().max(1);
    for dz in -hz..=hz {
        for dx in -hx..=hx {
            let x = brush.origin[0].saturating_add(dx);
            let z = brush.origin[2].saturating_add(dz);
            let entry = occ.height.entry((x, z)).or_insert(0);
            if brush.op == BrushOp::Subtract {
                *entry = entry.saturating_sub(step);
            } else {
                *entry = entry.saturating_add(step);
            }
        }
    }
    for dy in -hy..=hy {
        for dz in -hz..=hz {
            for dx in -hx..=hx {
                let x = brush.origin[0].saturating_add(dx);
                let y = brush.origin[1].saturating_add(dy);
                let z = brush.origin[2].saturating_add(dz);
                let value = if brush.op == BrushOp::Subtract {
                    0
                } else {
                    127
                };
                occ.density.insert((x, y, z), value);
            }
        }
    }
}

fn smooth(occ: &mut Occupancy) {
    let keys: Vec<(i32, i32, i32)> = occ.density.keys().copied().collect();
    let mut next = occ.density.clone();
    for (x, y, z) in keys {
        let here = i32::from(occ.density.get(&(x, y, z)).copied().unwrap_or(0));
        let left = i32::from(
            occ.density
                .get(&(x - 1, y, z))
                .copied()
                .unwrap_or(here as i8),
        );
        let right = i32::from(
            occ.density
                .get(&(x + 1, y, z))
                .copied()
                .unwrap_or(here as i8),
        );
        let value = (here + left + right) / 3;
        next.insert((x, y, z), value.clamp(0, 127) as i8);
    }
    occ.density = next;
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
}
