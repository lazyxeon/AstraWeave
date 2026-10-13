//! Voxel grid is the only ground. Meshes and trimeshes are derived.

use std::collections::{HashMap, HashSet};

use astraweave_physics::{Layers, PhysicsWorld};
use astraweave_render::mesh::{CpuMesh, MeshVertex as RenderVertex};
use astraweave_terrain::{ChunkCoord, ChunkMesh, DualContouring, MeshVertex, Voxel, VoxelGrid};
use glam::{Vec2, Vec3, Vec4};

use crate::island;

pub struct ChunkUpload {
    pub coord: ChunkCoord,
    pub mesh: CpuMesh,
    /// Dominant solid material, used as the chunk's instance tint.
    pub material: u16,
    pub removed: bool,
}

pub struct Ground {
    pub grid: VoxelGrid,
    pub edits: Vec<VoxelEdit>,
    mesher: DualContouring,
    bodies: HashMap<ChunkCoord, u64>,
    generated: HashSet<ChunkCoord>,
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
                }
            }
        }
        let stale: Vec<ChunkCoord> = self
            .generated
            .iter()
            .copied()
            .filter(|coord| coord.y == 0 && !keep.contains(coord))
            .collect();
        let mut removed = Vec::new();
        for coord in stale {
            self.generated.remove(&coord);
            self.grid.remove_chunk(coord);
            if let Some(id) = self.bodies.remove(&coord) {
                physics.remove_body(id);
            }
            removed.push(ChunkUpload {
                coord,
                mesh: chunk_mesh_to_cpu(&ChunkMesh::empty(coord)),
                material: 0,
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
            let material = dominant_material(&mesh);
            uploads.push(ChunkUpload {
                coord,
                mesh: chunk_mesh_to_cpu(&mesh),
                material,
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
            for y in 0..=h {
                let material = if y < h { 1 } else { 3 };
                grid.set_voxel(
                    Vec3::new(wx as f32, y as f32, wz as f32),
                    Voxel::new(1.0, material),
                );
            }
        }
    }
}

fn triangle_list(indices: &[u32]) -> Vec<[u32; 3]> {
    indices
        .chunks_exact(3)
        .map(|t| [t[0], t[1], t[2]])
        .collect()
}

fn dominant_material(mesh: &ChunkMesh) -> u16 {
    mesh.vertices
        .iter()
        .map(|v| v.material)
        .find(|m| *m != 0)
        .unwrap_or(0)
}

pub fn chunk_mesh_to_cpu(mesh: &ChunkMesh) -> CpuMesh {
    let vertices = mesh.vertices.iter().map(render_vertex).collect();
    CpuMesh {
        vertices,
        indices: mesh.indices.clone(),
        albedo_image: None,
        texture_source_hint: None,
    }
}

fn render_vertex(v: &MeshVertex) -> RenderVertex {
    let n = if v.normal.length_squared() > 1e-8 {
        v.normal.normalize()
    } else {
        Vec3::Y
    };
    let helper = if n.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
    let tangent = n.cross(helper).normalize_or_zero();
    RenderVertex::new(
        v.position,
        n,
        Vec4::new(tangent.x, tangent.y, tangent.z, 1.0),
        Vec2::new(v.position.x * 0.05, v.position.z * 0.05),
    )
}

pub fn material_tint(material: u16) -> [f32; 4] {
    match material {
        3 => [0.35, 0.55, 0.28, 1.0],
        2 => [0.45, 0.32, 0.18, 1.0],
        1 => [0.55, 0.55, 0.58, 1.0],
        _ => [0.7, 0.7, 0.72, 1.0],
    }
}
