//! Voxel grid is the only ground. Meshes and trimeshes are derived.

use std::collections::{HashMap, HashSet};

use astraweave_physics::{Layers, PhysicsWorld};
use astraweave_render::mesh::{CpuMesh, MeshVertex as RenderVertex};
use astraweave_terrain::{ChunkCoord, ChunkMesh, DualContouring, MeshVertex, Voxel, VoxelGrid};
use glam::{Vec2, Vec3, Vec4};

use crate::island::{self, biome_at, column_material, Biome, MAT_WOOD};

pub struct ChunkPart {
    pub material: u16,
    pub mesh: CpuMesh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropKind {
    Oak,
    Pine,
    Palm,
    Rock,
    Grass,
    Bush,
}

#[derive(Clone, Copy, Debug)]
pub struct PropSpot {
    pub kind: PropKind,
    pub position: Vec3,
    pub yaw: f32,
    pub scale: f32,
}

pub struct ChunkUpload {
    pub coord: ChunkCoord,
    pub parts: Vec<ChunkPart>,
    pub props: Vec<PropSpot>,
    pub removed: bool,
}

impl ChunkUpload {
    pub fn vertex_count(&self) -> usize {
        self.parts.iter().map(|part| part.mesh.vertices.len()).sum()
    }
}

pub struct Ground {
    pub grid: VoxelGrid,
    pub edits: Vec<VoxelEdit>,
    mesher: DualContouring,
    bodies: HashMap<ChunkCoord, u64>,
    generated: HashSet<ChunkCoord>,
    scatter: HashMap<ChunkCoord, Vec<PropSpot>>,
}

#[derive(Clone, Debug)]
pub struct VoxelEdit {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub density: f32,
    pub material: u16,
}

impl Ground {
    pub fn from_grid(grid: VoxelGrid) -> Self {
        Self {
            grid,
            edits: Vec::new(),
            mesher: DualContouring::new(),
            bodies: HashMap::new(),
            generated: HashSet::new(),
            scatter: HashMap::new(),
        }
    }

    pub fn small_island() -> Self {
        Self::from_grid(island::small())
    }

    /// Generate or drop y=0 chunks so the player stands in a finite loaded ring
    /// of an otherwise unbounded heightfield.
    pub fn stream_around(
        &mut self,
        physics: &mut PhysicsWorld,
        focus: Vec3,
        radius: i32,
    ) -> Vec<ChunkUpload> {
        let center = ChunkCoord::from_world_pos(Vec3::new(focus.x, 0.0, focus.z));
        let mut keep = HashSet::new();
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs().max(dz.abs()) > radius {
                    continue;
                }
                let coord = ChunkCoord::new(center.x + dx, 0, center.z + dz);
                keep.insert(coord);
                if self.generated.insert(coord) {
                    fill_height_chunk(&mut self.grid, coord);
                    self.reapply_edits(coord);
                    self.scatter.insert(coord, scatter_chunk(coord));
                }
            }
        }
        let mut stale: Vec<ChunkCoord> = self
            .generated
            .iter()
            .copied()
            .filter(|coord| coord.y == 0 && !keep.contains(coord))
            .collect();
        // HashSet iteration follows the process hasher. Removal order is part
        // of the physics step, so it has to be the same on every run.
        sort_coords(&mut stale);
        let mut removed = Vec::new();
        for coord in stale {
            self.generated.remove(&coord);
            self.scatter.remove(&coord);
            self.grid.remove_chunk(coord);
            if let Some(id) = self.bodies.remove(&coord) {
                physics.remove_body(id);
            }
            removed.push(ChunkUpload {
                coord,
                parts: Vec::new(),
                props: Vec::new(),
                removed: true,
            });
        }
        let mut uploads = self.sync_dirty(physics);
        uploads.extend(removed);
        uploads
    }

    fn reapply_edits(&mut self, coord: ChunkCoord) {
        let edits: Vec<VoxelEdit> = self
            .edits
            .iter()
            .filter(|edit| {
                ChunkCoord::from_world_pos(Vec3::new(edit.x as f32, edit.y as f32, edit.z as f32))
                    == coord
            })
            .cloned()
            .collect();
        for edit in edits {
            self.grid.set_voxel(
                Vec3::new(edit.x as f32, edit.y as f32, edit.z as f32),
                Voxel::new(edit.density, edit.material),
            );
        }
    }

    pub fn body_ids(&self) -> HashMap<ChunkCoord, u64> {
        self.bodies.clone()
    }

    /// Body ids in chunk-coordinate order. The map itself does not iterate stably.
    pub fn sorted_body_ids(&self) -> Vec<(i32, i32, i32, u64)> {
        let mut rows: Vec<(i32, i32, i32, u64)> = self
            .bodies
            .iter()
            .map(|(coord, id)| (coord.x, coord.y, coord.z, *id))
            .collect();
        rows.sort_unstable();
        rows
    }

    /// Remesh dirty chunks, replace their trimesh bodies, and return CPU meshes
    /// for the renderer. A clean grid returns an empty list and does not
    /// allocate new bodies.
    pub fn sync_dirty(&mut self, physics: &mut PhysicsWorld) -> Vec<ChunkUpload> {
        let dirty: Vec<ChunkCoord> = self.grid.dirty_chunks().to_vec();
        let mut uploads = Vec::with_capacity(dirty.len());
        for coord in dirty {
            let chunk = self.grid.get_chunk(coord).cloned();
            let Some(chunk) = chunk else {
                self.grid.mark_chunk_clean(coord);
                continue;
            };
            let mesh = self.mesher.generate_mesh(&chunk);
            self.grid.mark_chunk_clean(coord);
            self.replace_body(physics, coord, &mesh);
            let removed = mesh.is_empty();
            uploads.push(ChunkUpload {
                coord,
                parts: chunk_parts(&mesh),
                props: self.visible_props(coord),
                removed,
            });
        }
        uploads
    }

    /// Write air voxels. Caller runs [`Self::sync_dirty`] afterwards.
    pub fn dig_sphere(&mut self, center: Vec3, radius: f32) {
        let r = radius.ceil() as i32;
        let origin = IVecFloor::from_vec(center);
        for x in -r..=r {
            for y in -r..=r {
                for z in -r..=r {
                    let p = Vec3::new(
                        (origin.x + x) as f32,
                        (origin.y + y) as f32,
                        (origin.z + z) as f32,
                    );
                    if p.distance(center) <= radius {
                        self.grid.set_voxel(p, Voxel::new(0.0, 0));
                        self.remember(p, 0.0, 0);
                    }
                }
            }
        }
    }

    /// Write solid voxels that touch an existing solid neighbor, or sit on the floor.
    /// Returns false when every sample was rejected.
    pub fn place_sphere(&mut self, center: Vec3, radius: f32, material: u16) -> bool {
        let r = radius.ceil() as i32;
        let origin = IVecFloor::from_vec(center);
        let mut spots = Vec::new();
        for x in -r..=r {
            for y in -r..=r {
                for z in -r..=r {
                    let p = Vec3::new(
                        (origin.x + x) as f32,
                        (origin.y + y) as f32,
                        (origin.z + z) as f32,
                    );
                    if p.distance(center) <= radius {
                        spots.push((origin.y + y, p));
                    }
                }
            }
        }
        spots.sort_by_key(|(y, _)| *y);
        let mut placed = false;
        for (_, p) in spots {
            if self.supported(p) {
                self.grid.set_voxel(p, Voxel::new(1.0, material));
                self.remember(p, 1.0, material);
                placed = true;
            }
        }
        placed
    }

    pub fn replay(&mut self, edits: &[VoxelEdit]) {
        for edit in edits {
            let p = Vec3::new(edit.x as f32, edit.y as f32, edit.z as f32);
            self.grid
                .set_voxel(p, Voxel::new(edit.density, edit.material));
        }
        self.edits.extend(edits.iter().cloned());
    }

    fn supported(&self, p: Vec3) -> bool {
        let y = p.y.round() as i32;
        if y <= 0 {
            return true;
        }
        let neighbors = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
        neighbors.into_iter().any(|offset| {
            self.grid
                .get_voxel(p + offset)
                .is_some_and(|voxel| voxel.is_solid())
        })
    }

    fn remember(&mut self, p: Vec3, density: f32, material: u16) {
        self.edits.push(VoxelEdit {
            x: p.x.round() as i32,
            y: p.y.round() as i32,
            z: p.z.round() as i32,
            density,
            material,
        });
    }

    /// Replace the edit log and rebuild every loaded chunk from the heightfield
    /// plus that log. Used when a save is loaded.
    pub fn restore_edits(&mut self, edits: Vec<VoxelEdit>) {
        self.edits = edits;
        let mut coords: Vec<ChunkCoord> = self.generated.iter().copied().collect();
        sort_coords(&mut coords);
        for coord in coords {
            self.grid.remove_chunk(coord);
            fill_height_chunk(&mut self.grid, coord);
            self.reapply_edits(coord);
        }
    }

    fn visible_props(&self, coord: ChunkCoord) -> Vec<PropSpot> {
        self.scatter
            .get(&coord)
            .map(|spots| {
                spots
                    .iter()
                    .copied()
                    .filter(|spot| {
                        let below = spot.position - Vec3::Y * 0.6;
                        self.grid
                            .get_voxel(below)
                            .is_some_and(|voxel| voxel.is_solid())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn replace_body(&mut self, physics: &mut PhysicsWorld, coord: ChunkCoord, mesh: &ChunkMesh) {
        if let Some(old) = self.bodies.remove(&coord) {
            physics.remove_body(old);
        }
        if mesh.is_empty() || mesh.indices.len() < 3 {
            return;
        }
        let vertices: Vec<Vec3> = mesh.vertices.iter().map(|v| v.position).collect();
        let tris = triangle_list(&mesh.indices);
        if tris.is_empty() {
            return;
        }
        let id = physics.add_static_trimesh(&vertices, &tris, Layers::DEFAULT);
        self.bodies.insert(coord, id);
    }
}

struct IVecFloor {
    x: i32,
    y: i32,
    z: i32,
}

impl IVecFloor {
    fn from_vec(v: Vec3) -> Self {
        Self {
            x: v.x.floor() as i32,
            y: v.y.floor() as i32,
            z: v.z.floor() as i32,
        }
    }
}

fn fill_height_chunk(grid: &mut VoxelGrid, coord: ChunkCoord) {
    let origin = coord.to_world_pos();
    for x in 0..astraweave_terrain::CHUNK_SIZE {
        for z in 0..astraweave_terrain::CHUNK_SIZE {
            let wx = origin.x as i32 + x;
            let wz = origin.z as i32 + z;
            let h = island::surface_height(wx, wz).clamp(1, 24);
            let biome = biome_at(wx, wz);
            let road = island::on_yaqar_road(wx, wz);
            for y in 0..=h {
                let mut material = column_material(y, h, biome);
                if road && y >= h.saturating_sub(1) {
                    material = crate::island::MAT_DIRT;
                }
                grid.set_voxel(
                    Vec3::new(wx as f32, y as f32, wz as f32),
                    Voxel::new(1.0, material),
                );
            }
            if biome == Biome::Grass
                && !road
                && mix(wx, wz) % 53 == 0
                && x + 3 < astraweave_terrain::CHUNK_SIZE
            {
                for step in 0..3 {
                    grid.set_voxel(
                        Vec3::new((wx + step) as f32, (h + 1) as f32, wz as f32),
                        Voxel::new(1.0, MAT_WOOD),
                    );
                }
            }
        }
    }
}

fn scatter_chunk(coord: ChunkCoord) -> Vec<PropSpot> {
    let origin = coord.to_world_pos();
    let mut spots = Vec::new();
    let size = astraweave_terrain::CHUNK_SIZE;
    let mut x = 2;
    while x < size - 2 {
        let mut z = 2;
        while z < size - 2 {
            let wx = origin.x as i32 + x;
            let wz = origin.z as i32 + z;
            if island::on_yaqar_road(wx, wz) {
                z += 5;
                continue;
            }
            let h = island::surface_height(wx, wz);
            let n = mix(wx, wz);
            let yaw = (n & 255) as f32 / 255.0 * std::f32::consts::TAU;
            let position = Vec3::new(wx as f32 + 0.5, h as f32 + 0.55, wz as f32 + 0.5);
            match biome_at(wx, wz) {
                Biome::Grass => {
                    if n % 5 == 0 {
                        spots.push(PropSpot {
                            kind: PropKind::Grass,
                            position,
                            yaw,
                            scale: 1.6,
                        });
                    }
                    if n % 9 == 0 {
                        spots.push(PropSpot {
                            kind: PropKind::Bush,
                            position,
                            yaw,
                            scale: 2.2,
                        });
                    }
                    if n % 7 == 0 {
                        spots.push(PropSpot {
                            kind: if n % 2 == 0 {
                                PropKind::Oak
                            } else {
                                PropKind::Pine
                            },
                            position,
                            yaw,
                            scale: 3.2,
                        });
                    }
                }
                Biome::Sand => {
                    if n % 6 == 0 {
                        spots.push(PropSpot {
                            kind: PropKind::Palm,
                            position,
                            yaw,
                            scale: 3.0,
                        });
                    }
                    if n % 8 == 0 {
                        spots.push(PropSpot {
                            kind: PropKind::Rock,
                            position,
                            yaw,
                            scale: 1.8,
                        });
                    }
                }
                Biome::Rock => {
                    if n % 4 == 0 {
                        spots.push(PropSpot {
                            kind: PropKind::Rock,
                            position,
                            yaw,
                            scale: 2.4,
                        });
                    }
                    if n % 11 == 0 {
                        spots.push(PropSpot {
                            kind: PropKind::Pine,
                            position,
                            yaw,
                            scale: 2.6,
                        });
                    }
                }
            }
            z += 5;
        }
        x += 5;
    }
    spots
}

fn sort_coords(coords: &mut [ChunkCoord]) {
    coords.sort_by_key(|coord| (coord.x, coord.y, coord.z));
}

fn mix(x: i32, z: i32) -> u32 {
    let mut n = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((z as u32).wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n
}

fn triangle_list(indices: &[u32]) -> Vec<[u32; 3]> {
    indices
        .chunks_exact(3)
        .map(|t| [t[0], t[1], t[2]])
        .collect()
}

fn chunk_parts(mesh: &ChunkMesh) -> Vec<ChunkPart> {
    let mut faces: HashMap<u16, Vec<[u32; 3]>> = HashMap::new();
    for tri in mesh.indices.chunks_exact(3) {
        let Some(vertex) = mesh.vertices.get(tri[0] as usize) else {
            continue;
        };
        faces
            .entry(vertex.material)
            .or_default()
            .push([tri[0], tri[1], tri[2]]);
    }
    let mut materials: Vec<u16> = faces.keys().copied().collect();
    materials.sort_unstable();
    let mut parts = Vec::new();
    for material in materials {
        let tris = &faces[&material];
        let mut remap: HashMap<u32, u32> = HashMap::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for face in tris {
            for &index in face {
                let mapped = if let Some(existing) = remap.get(&index) {
                    *existing
                } else {
                    let id = vertices.len() as u32;
                    if let Some(vertex) = mesh.vertices.get(index as usize) {
                        vertices.push(render_vertex(vertex));
                    }
                    remap.insert(index, id);
                    id
                };
                indices.push(mapped);
            }
        }
        if vertices.is_empty() {
            continue;
        }
        parts.push(ChunkPart {
            material,
            mesh: CpuMesh {
                vertices,
                indices,
                albedo_image: None,
                texture_source_hint: None,
            },
        });
    }
    parts
}

fn render_vertex(v: &MeshVertex) -> RenderVertex {
    let n = if v.normal.length_squared() > 1e-8 {
        v.normal.normalize()
    } else {
        Vec3::Y
    };
    let helper = if n.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
    let tangent = n.cross(helper).normalize_or_zero();
    let (u, t) = if n.y.abs() >= n.x.abs() && n.y.abs() >= n.z.abs() {
        (v.position.x, v.position.z)
    } else if n.x.abs() >= n.z.abs() {
        (v.position.z, v.position.y)
    } else {
        (v.position.x, v.position.y)
    };
    RenderVertex::new(
        v.position,
        n,
        Vec4::new(tangent.x, tangent.y, tangent.z, 1.0),
        Vec2::new(u * 0.12, t * 0.12),
    )
}

pub fn material_tint(material: u16) -> [f32; 4] {
    match material {
        5 => [0.55, 0.36, 0.18, 1.0],
        4 => [0.76, 0.70, 0.45, 1.0],
        3 => [0.28, 0.46, 0.22, 1.0],
        2 => [0.42, 0.30, 0.16, 1.0],
        1 => [0.55, 0.55, 0.58, 1.0],
        _ => [0.7, 0.7, 0.72, 1.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_physics::PhysicsWorld;
    use astraweave_terrain::VoxelGrid;

    #[test]
    fn one_streamed_chunk_splits_materials_and_scatters_props() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::from_grid(VoxelGrid::new());
        let uploads = ground.stream_around(&mut physics, Vec3::new(16.0, 10.0, 16.0), 0);
        let materials: std::collections::HashSet<u16> = uploads
            .iter()
            .flat_map(|chunk| chunk.parts.iter().map(|part| part.material))
            .collect();
        assert!(
            materials.len() >= 2,
            "expected layered materials, got {materials:?}"
        );
        assert!(
            uploads.iter().any(|chunk| !chunk.props.is_empty()),
            "expected trees or rocks"
        );
        assert!(uploads.iter().any(|chunk| chunk.vertex_count() > 100));
    }

    #[test]
    fn the_road_toward_yaqar_is_dirt_without_changing_height() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::from_grid(VoxelGrid::new());
        let _ = ground.stream_around(&mut physics, Vec3::new(16.0, 10.0, 16.0), 0);
        let height = island::surface_height(20, 16);
        let road = ground.grid.get_voxel(Vec3::new(20.0, height as f32, 16.0));
        assert!(
            road.is_some_and(|voxel| voxel.is_solid() && voxel.material == crate::island::MAT_DIRT),
            "road surface should be dirt"
        );
        assert!(!island::on_yaqar_road(20, 24));
    }

    #[test]
    fn restore_edits_refills_a_dug_hole() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
        let mut ground = Ground::from_grid(VoxelGrid::new());
        let _ = ground.stream_around(&mut physics, Vec3::new(16.0, 10.0, 16.0), 0);
        let spot = Vec3::new(16.0, island::surface_height(16, 16) as f32, 16.0);
        assert!(ground
            .grid
            .get_voxel(spot)
            .is_some_and(|voxel| voxel.is_solid()));
        ground.dig_sphere(spot, 1.5);
        let _ = ground.sync_dirty(&mut physics);
        assert!(ground
            .grid
            .get_voxel(spot)
            .is_some_and(|voxel| !voxel.is_solid()));
        ground.restore_edits(Vec::new());
        let _ = ground.sync_dirty(&mut physics);
        assert!(
            ground
                .grid
                .get_voxel(spot)
                .is_some_and(|voxel| voxel.is_solid()),
            "load with an empty edit log should rebuild the column"
        );
        assert!(ground.edits.is_empty());
    }
}
