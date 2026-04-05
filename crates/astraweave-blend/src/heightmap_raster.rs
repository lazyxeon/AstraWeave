//! Terrain mesh heightmap rasterization.
//!
//! Converts extracted terrain meshes from .blend scenes into rasterized
//! heightmap grids usable by the terrain system. Supports multiple terrain
//! tiles with seam stitching.

use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// A terrain mesh extracted from a .blend scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTerrainMesh {
    /// Object name from Blender.
    pub name: String,
    /// Vertex positions in world space `[x, y, z]`.
    pub vertices: Vec<[f32; 3]>,
    /// Triangle face indices (3 indices per face).
    pub faces: Vec<[u32; 3]>,
    /// Axis-aligned bounding box.
    pub bounds: TerrainBounds,
}

/// AABB for a terrain mesh.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TerrainBounds {
    /// Minimum corner [x, y, z].
    pub min: [f32; 3],
    /// Maximum corner [x, y, z].
    pub max: [f32; 3],
}

impl TerrainBounds {
    /// Compute bounds from a set of vertices. Returns None if empty.
    pub fn from_vertices(vertices: &[[f32; 3]]) -> Option<Self> {
        if vertices.is_empty() {
            return None;
        }
        let mut min = vertices[0];
        let mut max = vertices[0];
        for v in &vertices[1..] {
            for i in 0..3 {
                if v[i] < min[i] {
                    min[i] = v[i];
                }
                if v[i] > max[i] {
                    max[i] = v[i];
                }
            }
        }
        Some(Self { min, max })
    }

    /// Width in the X axis.
    pub fn width(&self) -> f32 {
        self.max[0] - self.min[0]
    }

    /// Depth in the Z axis.
    pub fn depth(&self) -> f32 {
        self.max[2] - self.min[2]
    }

    /// XZ footprint area.
    pub fn footprint_area(&self) -> f32 {
        self.width() * self.depth()
    }
}

/// A rasterized heightmap produced from terrain mesh data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterizedHeightmap {
    /// Height values in row-major order (z * resolution + x).
    pub data: Vec<f32>,
    /// Grid resolution (width and height in samples).
    pub resolution: u32,
    /// World-space XZ bounds that this heightmap covers.
    pub world_min: [f32; 2],
    /// World-space XZ maximum.
    pub world_max: [f32; 2],
    /// Minimum height value in the data.
    pub min_height: f32,
    /// Maximum height value in the data.
    pub max_height: f32,
}

impl RasterizedHeightmap {
    /// Sample the heightmap at a world XZ position using bilinear interpolation.
    pub fn sample(&self, world_x: f32, world_z: f32) -> f32 {
        let range_x = self.world_max[0] - self.world_min[0];
        let range_z = self.world_max[1] - self.world_min[1];
        if range_x <= 0.0 || range_z <= 0.0 {
            return self.min_height;
        }

        let u = ((world_x - self.world_min[0]) / range_x).clamp(0.0, 1.0);
        let v = ((world_z - self.world_min[1]) / range_z).clamp(0.0, 1.0);

        let fx = u * (self.resolution - 1) as f32;
        let fz = v * (self.resolution - 1) as f32;
        let ix = fx as u32;
        let iz = fz as u32;
        let fx = fx - ix as f32;
        let fz = fz - iz as f32;

        let ix1 = (ix + 1).min(self.resolution - 1);
        let iz1 = (iz + 1).min(self.resolution - 1);

        let v00 = self.data[(iz * self.resolution + ix) as usize];
        let v10 = self.data[(iz * self.resolution + ix1) as usize];
        let v01 = self.data[(iz1 * self.resolution + ix) as usize];
        let v11 = self.data[(iz1 * self.resolution + ix1) as usize];

        let top = v00 * (1.0 - fx) + v10 * fx;
        let bottom = v01 * (1.0 - fx) + v11 * fx;
        top * (1.0 - fz) + bottom * fz
    }

    /// XZ footprint area of the rasterized region.
    pub fn footprint_area(&self) -> f32 {
        let w = self.world_max[0] - self.world_min[0];
        let d = self.world_max[1] - self.world_min[1];
        w * d
    }
}

// ============================================================================
// Terrain mesh identification
// ============================================================================

/// Terrain-related name keywords (case-insensitive matching).
const TERRAIN_KEYWORDS: &[&str] = &["terrain", "ground", "landscape", "floor", "land", "earth"];

/// Check if an object name suggests it is a terrain mesh.
pub fn is_terrain_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    TERRAIN_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

/// Fixed placement of a non-terrain object from a .blend scene.
///
/// Used in Replica mode to reproduce exact object positions/orientations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedPlacement {
    /// World-space position `[x, y, z]`.
    pub position: [f64; 3],
    /// Rotation as Euler XYZ radians.
    pub rotation: [f64; 3],
    /// Scale `[x, y, z]`.
    pub scale: [f64; 3],
    /// Mesh asset filename (relative path, e.g. `meshes/Tree_01.glb`).
    pub mesh_path: String,
    /// Asset category: `vegetation`, `rock`, `prop`, etc.
    pub category: String,
    /// Object name from Blender.
    pub name: String,
}

// ============================================================================
// Rasterization
// ============================================================================

/// Rasterize one or more terrain meshes into a heightmap grid.
///
/// For each grid cell, casts a ray downward (in -Y) and takes the highest
/// triangle intersection. Multiple terrain tiles are automatically stitched
/// by spatial overlap, with seam values averaged.
pub fn rasterize_terrain_meshes(
    meshes: &[ExtractedTerrainMesh],
    resolution: u32,
) -> anyhow::Result<RasterizedHeightmap> {
    if meshes.is_empty() {
        anyhow::bail!("No terrain meshes to rasterize");
    }
    if resolution < 2 {
        anyhow::bail!("Resolution must be at least 2");
    }

    // Compute combined world bounds across all meshes
    let mut global_min = [f32::MAX; 3];
    let mut global_max = [f32::MIN; 3];
    for mesh in meshes {
        for i in 0..3 {
            global_min[i] = global_min[i].min(mesh.bounds.min[i]);
            global_max[i] = global_max[i].max(mesh.bounds.max[i]);
        }
    }

    let range_x = global_max[0] - global_min[0];
    let range_z = global_max[2] - global_min[2];
    if range_x <= 0.0 || range_z <= 0.0 {
        anyhow::bail!("Terrain meshes have zero XZ extent");
    }

    let size = (resolution * resolution) as usize;
    let mut height_data = vec![f32::NEG_INFINITY; size];
    let mut hit_count = vec![0u32; size];

    let step_x = range_x / (resolution - 1) as f32;
    let step_z = range_z / (resolution - 1) as f32;

    // For each mesh, rasterize triangles into the grid
    for mesh in meshes {
        for face in &mesh.faces {
            let v0 = mesh.vertices[face[0] as usize];
            let v1 = mesh.vertices[face[1] as usize];
            let v2 = mesh.vertices[face[2] as usize];

            // Compute triangle XZ bounding box in grid coords
            let tri_min_x = v0[0].min(v1[0]).min(v2[0]);
            let tri_max_x = v0[0].max(v1[0]).max(v2[0]);
            let tri_min_z = v0[2].min(v1[2]).min(v2[2]);
            let tri_max_z = v0[2].max(v1[2]).max(v2[2]);

            let ix_min = ((tri_min_x - global_min[0]) / step_x).floor().max(0.0) as u32;
            let ix_max = ((tri_max_x - global_min[0]) / step_x)
                .ceil()
                .min(resolution as f32 - 1.0) as u32;
            let iz_min = ((tri_min_z - global_min[2]) / step_z).floor().max(0.0) as u32;
            let iz_max = ((tri_max_z - global_min[2]) / step_z)
                .ceil()
                .min(resolution as f32 - 1.0) as u32;

            for gz in iz_min..=iz_max {
                for gx in ix_min..=ix_max {
                    let world_x = global_min[0] + gx as f32 * step_x;
                    let world_z = global_min[2] + gz as f32 * step_z;

                    if let Some(height) = ray_triangle_height(world_x, world_z, &v0, &v1, &v2) {
                        let idx = (gz * resolution + gx) as usize;
                        if hit_count[idx] == 0 || height > height_data[idx] {
                            // For overlapping tiles: if this is a new hit on a cell
                            // already hit by another mesh, average the heights (seam blending)
                            if hit_count[idx] > 0 && height_data[idx] != f32::NEG_INFINITY {
                                // Average with previous value for seam blending
                                let prev = height_data[idx];
                                let prev_count = hit_count[idx] as f32;
                                height_data[idx] =
                                    (prev * prev_count + height) / (prev_count + 1.0);
                            } else {
                                height_data[idx] = height;
                            }
                            hit_count[idx] += 1;
                        }
                    }
                }
            }
        }
    }

    // Fill holes: cells that weren't hit by any triangle get interpolated
    // from nearest neighbors using a simple flood fill
    fill_holes(&mut height_data, &hit_count, resolution);

    let min_height = height_data
        .iter()
        .copied()
        .filter(|h| *h != f32::NEG_INFINITY)
        .fold(f32::INFINITY, f32::min);
    let max_height = height_data
        .iter()
        .copied()
        .filter(|h| *h != f32::NEG_INFINITY)
        .fold(f32::NEG_INFINITY, f32::max);

    // Replace any remaining NEG_INFINITY with min_height
    for h in &mut height_data {
        if *h == f32::NEG_INFINITY {
            *h = min_height;
        }
    }

    Ok(RasterizedHeightmap {
        data: height_data,
        resolution,
        world_min: [global_min[0], global_min[2]],
        world_max: [global_max[0], global_max[2]],
        min_height,
        max_height,
    })
}

/// Cast a vertical ray at (x, z) through a triangle and return the Y height
/// at the intersection, if the point is inside the triangle's XZ projection.
fn ray_triangle_height(x: f32, z: f32, v0: &[f32; 3], v1: &[f32; 3], v2: &[f32; 3]) -> Option<f32> {
    // Barycentric coordinate test in XZ plane
    let ax = v0[0];
    let az = v0[2];
    let bx = v1[0];
    let bz = v1[2];
    let cx = v2[0];
    let cz = v2[2];

    let denom = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz);
    if denom.abs() < f32::EPSILON {
        return None; // Degenerate triangle
    }

    let inv_denom = 1.0 / denom;
    let u = ((bz - cz) * (x - cx) + (cx - bx) * (z - cz)) * inv_denom;
    let v = ((cz - az) * (x - cx) + (ax - cx) * (z - cz)) * inv_denom;
    let w = 1.0 - u - v;

    // Check if point is inside triangle (with small epsilon for edge cases)
    const EPS: f32 = -1e-6;
    if u >= EPS && v >= EPS && w >= EPS {
        // Interpolate Y using barycentric coordinates
        let y = u * v0[1] + v * v1[1] + w * v2[1];
        Some(y)
    } else {
        None
    }
}

/// Fill holes in the heightmap where no triangle hit was registered.
/// Uses iterative neighbor averaging.
fn fill_holes(data: &mut [f32], hit_count: &[u32], resolution: u32) {
    let size = (resolution * resolution) as usize;
    let mut filled = vec![false; size];
    for i in 0..size {
        filled[i] = hit_count[i] > 0;
    }

    // Multiple passes until no more holes can be filled
    for _pass in 0..resolution {
        let mut any_filled = false;
        let prev_data = data.to_vec();

        for gz in 0..resolution {
            for gx in 0..resolution {
                let idx = (gz * resolution + gx) as usize;
                if filled[idx] {
                    continue;
                }

                // Average from filled neighbors
                let mut sum = 0.0f32;
                let mut count = 0u32;
                let offsets: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, -1), (0, 1)];
                for &(dx, dz) in offsets {
                    let nx = gx as i32 + dx;
                    let nz = gz as i32 + dz;
                    if nx >= 0 && nx < resolution as i32 && nz >= 0 && nz < resolution as i32 {
                        let nidx = (nz as u32 * resolution + nx as u32) as usize;
                        if filled[nidx] {
                            sum += prev_data[nidx];
                            count += 1;
                        }
                    }
                }

                if count > 0 {
                    data[idx] = sum / count as f32;
                    filled[idx] = true;
                    any_filled = true;
                }
            }
        }

        if !any_filled {
            break;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_plane_mesh(y: f32, size: f32) -> ExtractedTerrainMesh {
        // A simple flat quad at height y, from (0,y,0) to (size,y,size)
        ExtractedTerrainMesh {
            name: "Terrain".to_string(),
            vertices: vec![
                [0.0, y, 0.0],
                [size, y, 0.0],
                [size, y, size],
                [0.0, y, size],
            ],
            faces: vec![[0, 1, 2], [0, 2, 3]],
            bounds: TerrainBounds {
                min: [0.0, y, 0.0],
                max: [size, y, size],
            },
        }
    }

    fn sloped_plane_mesh() -> ExtractedTerrainMesh {
        // Plane sloping from y=0 at z=0 to y=10 at z=10
        ExtractedTerrainMesh {
            name: "SlopedTerrain".to_string(),
            vertices: vec![
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 10.0, 10.0],
                [0.0, 10.0, 10.0],
            ],
            faces: vec![[0, 1, 2], [0, 2, 3]],
            bounds: TerrainBounds {
                min: [0.0, 0.0, 0.0],
                max: [10.0, 10.0, 10.0],
            },
        }
    }

    #[test]
    fn test_rasterize_flat_plane() {
        let mesh = flat_plane_mesh(5.0, 10.0);
        let hm = rasterize_terrain_meshes(&[mesh], 8).unwrap();
        assert_eq!(hm.resolution, 8);
        assert_eq!(hm.data.len(), 64);

        // All heights should be ~5.0
        for h in &hm.data {
            assert!((*h - 5.0).abs() < 0.1, "Expected ~5.0, got {h}");
        }
        assert!((hm.min_height - 5.0).abs() < 0.1);
        assert!((hm.max_height - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_rasterize_sloped_plane() {
        let mesh = sloped_plane_mesh();
        let hm = rasterize_terrain_meshes(&[mesh], 16).unwrap();

        // At z=0 (row 0), height should be ~0
        let h_start = hm.data[0];
        assert!(
            h_start.abs() < 0.5,
            "Height at z=0 should be ~0, got {h_start}"
        );

        // At z=max (last row), height should be ~10
        let last_row = (hm.resolution - 1) * hm.resolution;
        let h_end = hm.data[last_row as usize];
        assert!(
            (h_end - 10.0).abs() < 0.5,
            "Height at z=max should be ~10, got {h_end}"
        );
    }

    #[test]
    fn test_rasterize_multi_tile() {
        // Two adjacent tiles
        let tile_a = ExtractedTerrainMesh {
            name: "Terrain_A".to_string(),
            vertices: vec![
                [0.0, 5.0, 0.0],
                [10.0, 5.0, 0.0],
                [10.0, 5.0, 10.0],
                [0.0, 5.0, 10.0],
            ],
            faces: vec![[0, 1, 2], [0, 2, 3]],
            bounds: TerrainBounds {
                min: [0.0, 5.0, 0.0],
                max: [10.0, 5.0, 10.0],
            },
        };
        let tile_b = ExtractedTerrainMesh {
            name: "Terrain_B".to_string(),
            vertices: vec![
                [10.0, 8.0, 0.0],
                [20.0, 8.0, 0.0],
                [20.0, 8.0, 10.0],
                [10.0, 8.0, 10.0],
            ],
            faces: vec![[0, 1, 2], [0, 2, 3]],
            bounds: TerrainBounds {
                min: [10.0, 8.0, 0.0],
                max: [20.0, 8.0, 10.0],
            },
        };

        let hm = rasterize_terrain_meshes(&[tile_a, tile_b], 8).unwrap();
        assert_eq!(hm.data.len(), 64);

        // Left side should be ~5, right side should be ~8
        // Sample at world x=2 (should be in tile A domain)
        let h_left = hm.sample(2.0, 5.0);
        assert!(
            (h_left - 5.0).abs() < 1.0,
            "Left tile height should be ~5, got {h_left}"
        );

        // Sample at world x=18 (should be in tile B domain)
        let h_right = hm.sample(18.0, 5.0);
        assert!(
            (h_right - 8.0).abs() < 1.0,
            "Right tile height should be ~8, got {h_right}"
        );
    }

    #[test]
    fn test_bilinear_sample() {
        let mesh = sloped_plane_mesh();
        let hm = rasterize_terrain_meshes(&[mesh], 8).unwrap();

        // Midpoint (5.0, 5.0) should give ~5.0 height
        let h = hm.sample(5.0, 5.0);
        assert!(
            (h - 5.0).abs() < 1.0,
            "Center of sloped plane should be ~5, got {h}"
        );
    }

    #[test]
    fn test_terrain_bounds() {
        let b =
            TerrainBounds::from_vertices(&[[0.0, 0.0, 0.0], [10.0, 5.0, 20.0], [-3.0, 2.0, 8.0]]);
        let b = b.unwrap();
        assert_eq!(b.min, [-3.0, 0.0, 0.0]);
        assert_eq!(b.max, [10.0, 5.0, 20.0]);
        assert!((b.width() - 13.0).abs() < 0.01);
        assert!((b.depth() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_terrain_name_detection() {
        assert!(is_terrain_name("Terrain_01"));
        assert!(is_terrain_name("main_ground"));
        assert!(is_terrain_name("Landscape"));
        assert!(is_terrain_name("FLOOR_mesh"));
        assert!(!is_terrain_name("Tree_01"));
        assert!(!is_terrain_name("Boulder"));
    }

    #[test]
    fn test_footprint_area() {
        let mesh = flat_plane_mesh(0.0, 10.0);
        assert!((mesh.bounds.footprint_area() - 100.0).abs() < 0.01);

        let hm = rasterize_terrain_meshes(&[mesh], 4).unwrap();
        assert!((hm.footprint_area() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_meshes_error() {
        let result = rasterize_terrain_meshes(&[], 8);
        assert!(result.is_err());
    }

    #[test]
    fn test_low_resolution_error() {
        let mesh = flat_plane_mesh(0.0, 10.0);
        let result = rasterize_terrain_meshes(&[mesh], 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_ray_triangle_height_inside() {
        let v0 = [0.0, 5.0, 0.0];
        let v1 = [10.0, 5.0, 0.0];
        let v2 = [5.0, 5.0, 10.0];

        let h = ray_triangle_height(5.0, 3.0, &v0, &v1, &v2);
        assert!(h.is_some());
        assert!((h.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_ray_triangle_height_outside() {
        let v0 = [0.0, 5.0, 0.0];
        let v1 = [10.0, 5.0, 0.0];
        let v2 = [5.0, 5.0, 10.0];

        let h = ray_triangle_height(-5.0, -5.0, &v0, &v1, &v2);
        assert!(h.is_none());
    }
}
