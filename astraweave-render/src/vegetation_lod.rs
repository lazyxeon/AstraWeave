//! Tree LOD chain with billboard/impostor support.
//!
//! Provides a multi-level LOD system for vegetation rendering:
//!
//! - **LOD0** (0–50 m): Full mesh
//! - **LOD1** (50–150 m): Simplified mesh (via `lod_generator.rs` quadric error metrics)
//! - **LOD2** (150–500 m): Cross-billboard — two quads at 90° with baked albedo
//! - **LOD3** (500 m+): Single impostor card — one camera-facing quad
//!
//! Billboard textures are packed into a shared atlas (8 angles × N species).
//! The cull compute pass writes LOD level per instance; draw is issued via
//! 4 separate draw-indirect calls, one per LOD tier, each binding the
//! appropriate mesh/shader.

use crate::lod_generator::{LODConfig, LODGenerator, SimplificationMesh};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

// ── LOD classification ──────────────────────────────────────────────────────

/// LOD level for a vegetation instance.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VegetationLod {
    /// Full-detail mesh (0–`lod0_max` metres from camera).
    FullMesh = 0,
    /// Simplified mesh via quadric-error collapse (`lod0_max`–`lod1_max`).
    Simplified = 1,
    /// Cross-billboard — two quads at 90° (`lod1_max`–`lod2_max`).
    CrossBillboard = 2,
    /// Single impostor card (`lod2_max`–`cull_distance`).
    ImpostorCard = 3,
}

/// LOD transition distances (metres from camera).
#[derive(Debug, Clone)]
pub struct TreeLodDistances {
    /// End of LOD0 (full mesh) band.
    pub lod0_max: f32,
    /// End of LOD1 (simplified mesh) band.
    pub lod1_max: f32,
    /// End of LOD2 (cross-billboard) band.
    pub lod2_max: f32,
    /// Beyond this distance, instances are culled entirely.
    pub cull_distance: f32,
}

impl Default for TreeLodDistances {
    fn default() -> Self {
        Self {
            lod0_max: 50.0,
            lod1_max: 150.0,
            lod2_max: 500.0,
            cull_distance: 1500.0,
        }
    }
}

/// Select the appropriate LOD level for a given camera distance.
///
/// Returns `None` when the instance should be culled.
pub fn select_lod(distance: f32, config: &TreeLodDistances) -> Option<VegetationLod> {
    if distance > config.cull_distance {
        None
    } else if distance > config.lod2_max {
        Some(VegetationLod::ImpostorCard)
    } else if distance > config.lod1_max {
        Some(VegetationLod::CrossBillboard)
    } else if distance > config.lod0_max {
        Some(VegetationLod::Simplified)
    } else {
        Some(VegetationLod::FullMesh)
    }
}

/// Build size-aware LOD distances from final world-space vegetation bounds.
///
/// The editor viewport historically classified vegetation from raw import size,
/// which caused aggressively scaled instances to remain in low-detail bands.
/// This helper derives stable distance bands from the size that actually reaches
/// the screen.
pub fn adaptive_lod_distances(
    world_height: f32,
    world_width: f32,
    cull_distance_override: Option<f32>,
) -> TreeLodDistances {
    let world_extent = world_height.max(world_width).max(0.25);

    let lod0_max = (world_extent * 6.0).clamp(35.0, 500.0);
    let lod1_max = (world_extent * 12.0).clamp(lod0_max + 20.0, 900.0);
    let default_lod2_max = (world_extent * 24.0).clamp(lod1_max + 20.0, 1800.0);
    let default_cull_distance = (world_extent * 42.0).clamp(default_lod2_max + 75.0, 3000.0);

    let cull_distance = cull_distance_override
        .filter(|distance| *distance > 0.0)
        .map(|distance| distance.max(default_lod2_max + 50.0))
        .unwrap_or(default_cull_distance);

    let lod2_max = default_lod2_max.min((cull_distance - 25.0).max(lod1_max + 10.0));

    TreeLodDistances {
        lod0_max,
        lod1_max,
        lod2_max,
        cull_distance,
    }
}

// ── GPU LOD distances uniform ───────────────────────────────────────────────

/// GPU-side LOD distances (16 bytes, matches WGSL `LodDistances`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LodDistancesGpu {
    /// x = lod0_max, y = lod1_max, z = lod2_max, w = cull_distance
    pub thresholds: [f32; 4],
}

impl From<&TreeLodDistances> for LodDistancesGpu {
    fn from(d: &TreeLodDistances) -> Self {
        Self {
            thresholds: [d.lod0_max, d.lod1_max, d.lod2_max, d.cull_distance],
        }
    }
}

// ── Billboard geometry generators ───────────────────────────────────────────

/// Vertex data for billboard geometry (position + normal + UV).
#[derive(Debug, Clone)]
pub struct BillboardMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl BillboardMesh {
    /// Convert this billboard mesh into a `CpuMesh` suitable for the renderer.
    ///
    /// Tangents are synthesized from normals (cross with Y-up fallback).
    /// No albedo image is embedded — the caller should assign an external
    /// texture via the scatter pipeline's texture discovery.
    pub fn to_cpu_mesh(&self) -> crate::mesh::CpuMesh {
        let vertices = self
            .positions
            .iter()
            .zip(self.normals.iter())
            .zip(self.uvs.iter())
            .map(|((pos, norm), uv)| {
                // Derive tangent from normal
                let n = *norm;
                let up = if n.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
                let t = n.cross(up).normalize();
                crate::mesh::MeshVertex {
                    position: pos.to_array(),
                    normal: norm.to_array(),
                    tangent: [t.x, t.y, t.z, 1.0],
                    uv: *uv,
                }
            })
            .collect();
        crate::mesh::CpuMesh {
            vertices,
            indices: self.indices.clone(),
            albedo_image: None,
            texture_source_hint: None,
        }
    }
}

/// Generate a cross-billboard: two quads at 90° forming an X shape.
///
/// Both quads are centred at the origin with their base at `y = 0`.
/// Quad A faces along the Z axis; Quad B faces along the X axis.
///
/// * `half_width` — half the width of each quad.
/// * `height` — total height of each quad.
///
/// Produces 8 vertices and 12 indices (4 triangles, double-sided with
/// duplicated winding so both faces are visible).
pub fn generate_cross_billboard(half_width: f32, height: f32) -> BillboardMesh {
    let hw = half_width;
    let h = height;

    // Quad A (faces Z): 4 vertices
    let positions = vec![
        // Quad A — facing +Z/−Z
        Vec3::new(-hw, 0.0, 0.0), // 0: bottom-left
        Vec3::new(hw, 0.0, 0.0),  // 1: bottom-right
        Vec3::new(hw, h, 0.0),    // 2: top-right
        Vec3::new(-hw, h, 0.0),   // 3: top-left
        // Quad B — facing +X/−X (rotated 90° around Y)
        Vec3::new(0.0, 0.0, -hw), // 4: bottom-left
        Vec3::new(0.0, 0.0, hw),  // 5: bottom-right
        Vec3::new(0.0, h, hw),    // 6: top-right
        Vec3::new(0.0, h, -hw),   // 7: top-left
    ];

    let normals = vec![
        Vec3::Z,
        Vec3::Z,
        Vec3::Z,
        Vec3::Z, // Quad A
        Vec3::X,
        Vec3::X,
        Vec3::X,
        Vec3::X, // Quad B
    ];

    let uvs = vec![
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0], // Quad A
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0], // Quad B
    ];

    // Two triangles per quad, front face (CCW) + back face (CW) for
    // double-sided visibility
    let indices = vec![
        // Quad A front (CCW viewed from +Z)
        0, 1, 2, 0, 2, 3, // Quad A back (CW from +Z = CCW from −Z)
        0, 2, 1, 0, 3, 2, // Quad B front (CCW viewed from +X)
        4, 5, 6, 4, 6, 7, // Quad B back
        4, 6, 5, 4, 7, 6,
    ];

    BillboardMesh {
        positions,
        normals,
        uvs,
        indices,
    }
}

/// Generate a single impostor card: one quad facing +Z.
///
/// The quad is centred horizontally at the origin with its base at `y = 0`.
/// At render time the vertex shader will billboard-align it to the camera.
///
/// * `half_width` — half the width of the card.
/// * `height` — total height of the card.
///
/// Produces 4 vertices and 6 indices (2 triangles, front-face only; the
/// vertex shader will rotate it to face the camera).
pub fn generate_impostor_card(half_width: f32, height: f32) -> BillboardMesh {
    let hw = half_width;
    let h = height;

    let positions = vec![
        Vec3::new(-hw, 0.0, 0.0), // 0: bottom-left
        Vec3::new(hw, 0.0, 0.0),  // 1: bottom-right
        Vec3::new(hw, h, 0.0),    // 2: top-right
        Vec3::new(-hw, h, 0.0),   // 3: top-left
    ];

    let normals = vec![Vec3::Z; 4];

    let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    let indices = vec![0, 1, 2, 0, 2, 3];

    BillboardMesh {
        positions,
        normals,
        uvs,
        indices,
    }
}

// ── Impostor atlas specification ────────────────────────────────────────────

/// Region occupied by one angle-capture of one species in the atlas.
#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    /// UV bounds in the atlas texture.
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

/// Per-species entry in the impostor atlas.
#[derive(Debug, Clone)]
pub struct AtlasSpeciesEntry {
    /// Species name (matches `VegetationType::name`).
    pub name: String,
    /// Regions for each capture angle (typically 8).
    pub angles: Vec<AtlasRegion>,
}

/// Specification for the shared impostor texture atlas.
///
/// Each species is rendered from `angle_count` equidistant views and packed
/// into a single texture. The atlas layout is a grid:
///
/// ```text
/// ┌────┬────┬────┬────┬────┬────┬────┬────┐
/// │ sp0│ sp0│ sp0│ sp0│ sp0│ sp0│ sp0│ sp0│  ← species 0, angles 0..7
/// ├────┼────┼────┼────┼────┼────┼────┼────┤
/// │ sp1│ sp1│ ...                          │  ← species 1
/// └────┴────┴─────────────────────────────┘
/// ```
#[derive(Debug, Clone)]
pub struct ImpostorAtlasSpec {
    /// Total atlas width in pixels.
    pub atlas_width: u32,
    /// Total atlas height in pixels.
    pub atlas_height: u32,
    /// Number of capture angles per species.
    pub angle_count: u32,
    /// Per-species layout entries (row order).
    pub species: Vec<AtlasSpeciesEntry>,
}

impl ImpostorAtlasSpec {
    /// Build an atlas specification with uniform cell sizes.
    ///
    /// Each cell is `atlas_width / angle_count` wide and
    /// `atlas_height / species_count` tall.
    pub fn uniform(
        atlas_width: u32,
        atlas_height: u32,
        angle_count: u32,
        species_names: &[&str],
    ) -> Self {
        let n_species = species_names.len().max(1);
        let cell_w = atlas_width as f32 / angle_count as f32;
        let cell_h = atlas_height as f32 / n_species as f32;

        let species = species_names
            .iter()
            .enumerate()
            .map(|(row, name)| {
                let angles = (0..angle_count)
                    .map(|col| {
                        let u_min = col as f32 * cell_w / atlas_width as f32;
                        let v_min = row as f32 * cell_h / atlas_height as f32;
                        let u_max = (col + 1) as f32 * cell_w / atlas_width as f32;
                        let v_max = (row + 1) as f32 * cell_h / atlas_height as f32;
                        AtlasRegion {
                            u_min,
                            v_min,
                            u_max,
                            v_max,
                        }
                    })
                    .collect();
                AtlasSpeciesEntry {
                    name: name.to_string(),
                    angles,
                }
            })
            .collect();

        Self {
            atlas_width,
            atlas_height,
            angle_count,
            species,
        }
    }

    /// For a given species index and viewing angle (radians), return the atlas
    /// UV region for the closest pre-rendered angle.
    pub fn lookup(&self, species_idx: usize, view_angle_rad: f32) -> Option<&AtlasRegion> {
        let entry = self.species.get(species_idx)?;
        if entry.angles.is_empty() {
            return None;
        }
        let angle_step = std::f32::consts::TAU / self.angle_count as f32;
        // Normalise to [0, TAU)
        let a = view_angle_rad.rem_euclid(std::f32::consts::TAU);
        let idx = ((a / angle_step + 0.5) as u32) % self.angle_count;
        entry.angles.get(idx as usize)
    }
}

// ── GPU atlas region (for shader upload) ────────────────────────────────────

/// Per-species atlas data uploaded to the GPU (16 bytes).
///
/// The shader computes the angle index from `atan2(camera_dir.z, camera_dir.x)`
/// and uses it to offset within the species' row.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct AtlasRegionGpu {
    /// UV of the first angle cell: u_min, v_min, u_max, v_max
    pub base_uv: [f32; 4],
}

// ── LOD chain builder ───────────────────────────────────────────────────────

/// Complete LOD chain for a single vegetation species.
///
/// LOD0 and LOD1 are conventional triangle meshes; LOD2 and LOD3 are
/// procedurally generated billboard geometry.
pub struct VegetationLodChain {
    /// Species identifier.
    pub species_name: String,

    /// LOD0: original full-detail mesh.
    pub lod0_mesh: SimplificationMesh,

    /// LOD1: simplified mesh (generated from LOD0).
    pub lod1_mesh: SimplificationMesh,

    /// LOD2: cross-billboard geometry.
    pub lod2_cross: BillboardMesh,

    /// LOD3: single impostor card.
    pub lod3_impostor: BillboardMesh,

    /// Atlas region for this species (first angle; shader offsets for others).
    pub atlas_region: AtlasRegion,
}

impl VegetationLodChain {
    /// Generate a full LOD chain from a source mesh.
    ///
    /// * `species_name` — unique name for this tree species.
    /// * `full_mesh` — LOD0 source geometry.
    /// * `simplification_target` — fraction of vertices to keep for LOD1
    ///   (e.g. 0.50 = keep 50%).
    /// * `half_width`, `height` — dimensions for billboard geometry.
    /// * `atlas_region` — region allocated in the impostor atlas.
    pub fn build(
        species_name: &str,
        full_mesh: SimplificationMesh,
        simplification_target: f32,
        half_width: f32,
        height: f32,
        atlas_region: AtlasRegion,
    ) -> Self {
        let target_verts =
            (full_mesh.vertex_count() as f32 * simplification_target.clamp(0.05, 1.0)) as usize;

        let generator = LODGenerator::new(LODConfig {
            reduction_targets: vec![simplification_target],
            max_error: 0.05,
            preserve_boundaries: true,
        });

        let lod1_mesh = generator.simplify(&full_mesh, target_verts.max(4));
        let lod2_cross = generate_cross_billboard(half_width, height);
        let lod3_impostor = generate_impostor_card(half_width, height);

        Self {
            species_name: species_name.to_string(),
            lod0_mesh: full_mesh,
            lod1_mesh,
            lod2_cross,
            lod3_impostor,
            atlas_region,
        }
    }
}

// ── Per-instance LOD-sorted output ──────────────────────────────────────────

/// Classify a batch of vegetation instances into LOD buckets.
///
/// Returns four lists of instance indices, one per LOD level. Instances
/// beyond `cull_distance` are dropped.
pub fn classify_instances_by_lod(
    instances: &[super::vegetation_gpu::VegetationInstanceGpu],
    camera_pos: Vec3,
    distances: &TreeLodDistances,
) -> [Vec<u32>; 4] {
    let mut buckets: [Vec<u32>; 4] = Default::default();

    for (i, inst) in instances.iter().enumerate() {
        let pos = Vec3::new(inst.pos_scale[0], inst.pos_scale[1], inst.pos_scale[2]);
        let dist = pos.distance(camera_pos);
        if let Some(lod) = select_lod(dist, distances) {
            buckets[lod as usize].push(i as u32);
        }
    }

    buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── select_lod ──────────────────────────────────────────────────────────

    #[test]
    fn test_select_lod_full_mesh() {
        let d = TreeLodDistances::default();
        assert_eq!(select_lod(0.0, &d), Some(VegetationLod::FullMesh));
        assert_eq!(select_lod(25.0, &d), Some(VegetationLod::FullMesh));
        assert_eq!(select_lod(49.9, &d), Some(VegetationLod::FullMesh));
    }

    #[test]
    fn test_select_lod_simplified() {
        let d = TreeLodDistances::default();
        assert_eq!(select_lod(50.0, &d), Some(VegetationLod::FullMesh));
        assert_eq!(select_lod(50.1, &d), Some(VegetationLod::Simplified));
        assert_eq!(select_lod(100.0, &d), Some(VegetationLod::Simplified));
        assert_eq!(select_lod(149.9, &d), Some(VegetationLod::Simplified));
    }

    #[test]
    fn test_adaptive_lod_distances_scale_with_world_size() {
        let small = adaptive_lod_distances(2.0, 1.0, None);
        let large = adaptive_lod_distances(24.0, 8.0, None);

        assert!(large.lod0_max > small.lod0_max);
        assert!(large.lod1_max > small.lod1_max);
        assert!(large.lod2_max > small.lod2_max);
        assert!(large.cull_distance > small.cull_distance);
    }

    #[test]
    fn test_adaptive_lod_distances_respect_override() {
        let distances = adaptive_lod_distances(12.0, 4.0, Some(900.0));

        assert_eq!(distances.cull_distance, 900.0);
        assert!(distances.lod2_max < distances.cull_distance);
    }

    #[test]
    fn test_select_lod_cross_billboard() {
        let d = TreeLodDistances::default();
        assert_eq!(select_lod(200.0, &d), Some(VegetationLod::CrossBillboard));
        assert_eq!(select_lod(499.0, &d), Some(VegetationLod::CrossBillboard));
    }

    #[test]
    fn test_select_lod_impostor() {
        let d = TreeLodDistances::default();
        assert_eq!(select_lod(600.0, &d), Some(VegetationLod::ImpostorCard));
        assert_eq!(select_lod(1499.0, &d), Some(VegetationLod::ImpostorCard));
    }

    #[test]
    fn test_select_lod_culled() {
        let d = TreeLodDistances::default();
        assert_eq!(select_lod(1500.1, &d), None);
        assert_eq!(select_lod(5000.0, &d), None);
    }

    #[test]
    fn test_select_lod_boundary_at_lod0_max() {
        let d = TreeLodDistances::default();
        // Exactly at boundary → stays in lower LOD (≤ comparison)
        assert_eq!(select_lod(50.0, &d), Some(VegetationLod::FullMesh));
    }

    // ── cross-billboard ─────────────────────────────────────────────────────

    #[test]
    fn test_cross_billboard_geometry() {
        let bb = generate_cross_billboard(2.0, 10.0);
        assert_eq!(bb.positions.len(), 8, "cross billboard: 8 vertices");
        assert_eq!(
            bb.indices.len(),
            24,
            "cross billboard: 24 indices (4 tris × 2 quads, double-sided)"
        );
        // Bottom vertices at y=0, top at y=height
        assert_eq!(bb.positions[0].y, 0.0);
        assert_eq!(bb.positions[2].y, 10.0);
    }

    #[test]
    fn test_cross_billboard_orthogonality() {
        let bb = generate_cross_billboard(3.0, 8.0);
        // Quad A is in XY plane (z=0), Quad B is in YZ plane (x=0)
        for i in 0..4 {
            assert_eq!(bb.positions[i].z, 0.0, "Quad A should lie in z=0 plane");
            assert_eq!(bb.positions[i + 4].x, 0.0, "Quad B should lie in x=0 plane");
        }
    }

    // ── impostor card ───────────────────────────────────────────────────────

    #[test]
    fn test_impostor_card_geometry() {
        let imp = generate_impostor_card(2.0, 10.0);
        assert_eq!(imp.positions.len(), 4, "impostor card: 4 vertices");
        assert_eq!(imp.indices.len(), 6, "impostor card: 6 indices");
        assert_eq!(imp.positions[0].y, 0.0);
        assert_eq!(imp.positions[2].y, 10.0);
    }

    // ── atlas spec ──────────────────────────────────────────────────────────

    #[test]
    fn test_atlas_spec_uniform() {
        let spec = ImpostorAtlasSpec::uniform(2048, 1024, 8, &["oak", "pine", "birch"]);
        assert_eq!(spec.species.len(), 3);
        assert_eq!(spec.species[0].angles.len(), 8);
        // First cell should start at (0.0, 0.0)
        let r = &spec.species[0].angles[0];
        assert!((r.u_min - 0.0).abs() < 1e-6);
        assert!((r.v_min - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_atlas_spec_coverage() {
        let spec = ImpostorAtlasSpec::uniform(2048, 1024, 8, &["oak", "pine"]);
        // Last species, last angle, should reach (1.0, 1.0)
        let last = &spec.species[1].angles[7];
        assert!((last.u_max - 1.0).abs() < 1e-6);
        assert!((last.v_max - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_atlas_lookup_nearest_angle() {
        let spec = ImpostorAtlasSpec::uniform(2048, 512, 8, &["tree"]);
        // angle_step = TAU/8 = ~0.785 rad
        // Looking at 0.0 rad → closest = angle 0
        let r0 = spec.lookup(0, 0.0).unwrap();
        assert!((r0.u_min - 0.0).abs() < 1e-6);

        // Looking at PI/2 (1.571 rad) → closest = angle 2
        let r2 = spec.lookup(0, std::f32::consts::FRAC_PI_2).unwrap();
        let expected_u = 2.0 / 8.0;
        assert!(
            (r2.u_min - expected_u).abs() < 1e-5,
            "expected u_min={expected_u}, got {}",
            r2.u_min,
        );
    }

    #[test]
    fn test_atlas_lookup_out_of_range() {
        let spec = ImpostorAtlasSpec::uniform(2048, 512, 8, &["tree"]);
        assert!(spec.lookup(999, 0.0).is_none());
    }

    // ── classify instances ──────────────────────────────────────────────────

    #[test]
    fn test_classify_instances_by_lod() {
        use crate::vegetation_gpu::VegetationInstanceGpu;

        let camera = Vec3::ZERO;
        let dists = TreeLodDistances::default();

        let instances = vec![
            // 10m away → LOD0
            VegetationInstanceGpu {
                pos_scale: [10.0, 0.0, 0.0, 1.0],
                rot_type_normal: [0.0; 4],
            },
            // 100m away → LOD1
            VegetationInstanceGpu {
                pos_scale: [100.0, 0.0, 0.0, 1.0],
                rot_type_normal: [0.0; 4],
            },
            // 300m away → LOD2
            VegetationInstanceGpu {
                pos_scale: [300.0, 0.0, 0.0, 1.0],
                rot_type_normal: [0.0; 4],
            },
            // 800m away → LOD3
            VegetationInstanceGpu {
                pos_scale: [800.0, 0.0, 0.0, 1.0],
                rot_type_normal: [0.0; 4],
            },
            // 2000m away → culled
            VegetationInstanceGpu {
                pos_scale: [2000.0, 0.0, 0.0, 1.0],
                rot_type_normal: [0.0; 4],
            },
        ];

        let buckets = classify_instances_by_lod(&instances, camera, &dists);

        assert_eq!(buckets[0].len(), 1, "LOD0 bucket"); // 10m
        assert_eq!(buckets[1].len(), 1, "LOD1 bucket"); // 100m
        assert_eq!(buckets[2].len(), 1, "LOD2 bucket"); // 300m
        assert_eq!(buckets[3].len(), 1, "LOD3 bucket"); // 800m
                                                        // 2000m instance should be culled — not in any bucket
        let total: usize = buckets.iter().map(|b| b.len()).sum();
        assert_eq!(total, 4, "culled instance should not appear");
    }

    // ── LOD chain builder ───────────────────────────────────────────────────

    #[test]
    fn test_lod_chain_build() {
        // Minimal triangle mesh for LOD0
        let mesh = SimplificationMesh::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0),
                Vec3::new(0.5, 0.0, 1.0),
            ],
            vec![Vec3::Y; 4],
            vec![[0.0, 0.0]; 4],
            vec![0, 1, 2, 0, 2, 3, 1, 3, 2],
        );

        let region = AtlasRegion {
            u_min: 0.0,
            v_min: 0.0,
            u_max: 0.125,
            v_max: 0.5,
        };

        let chain = VegetationLodChain::build("test_oak", mesh, 0.5, 2.0, 8.0, region);

        assert_eq!(chain.species_name, "test_oak");
        assert!(!chain.lod2_cross.positions.is_empty());
        assert!(!chain.lod3_impostor.positions.is_empty());
        assert_eq!(chain.lod2_cross.positions.len(), 8);
        assert_eq!(chain.lod3_impostor.positions.len(), 4);
    }

    // ── LodDistancesGpu ─────────────────────────────────────────────────────

    #[test]
    fn test_lod_distances_gpu_size() {
        assert_eq!(std::mem::size_of::<LodDistancesGpu>(), 16);
    }

    #[test]
    fn test_lod_distances_gpu_from() {
        let d = TreeLodDistances {
            lod0_max: 60.0,
            lod1_max: 200.0,
            lod2_max: 600.0,
            cull_distance: 2000.0,
        };
        let gpu: LodDistancesGpu = (&d).into();
        assert_eq!(gpu.thresholds, [60.0, 200.0, 600.0, 2000.0]);
    }

    #[test]
    fn test_atlas_region_gpu_size() {
        assert_eq!(std::mem::size_of::<AtlasRegionGpu>(), 16);
    }
}
