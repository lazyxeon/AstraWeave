//! Mesh optimization using meshopt
//!
//! Provides vertex cache optimization and overdraw reduction for meshes.

use anyhow::{Context, Result};
use bytemuck::cast_slice;
use meshopt::optimize_overdraw_in_place;
use meshopt::optimize_vertex_cache;
use meshopt::VertexDataAdapter;

/// Mesh optimization statistics
#[derive(Debug, Clone)]
pub struct MeshOptimizationStats {
    /// Original vertex count
    pub vertex_count: usize,
    /// Original index count
    pub index_count: usize,
    /// ACMR (Average Cache Miss Ratio) before optimization
    pub acmr_before: f32,
    /// ACMR after optimization
    pub acmr_after: f32,
    /// ACMR improvement percentage
    pub acmr_improvement_percent: f32,
    /// Overdraw ratio before optimization
    pub overdraw_before: f32,
    /// Overdraw ratio after optimization
    pub overdraw_after: f32,
    /// Overdraw improvement percentage
    pub overdraw_improvement_percent: f32,
    /// Optimization time in milliseconds
    pub time_ms: u64,
}

/// Mesh data for optimization
///
/// Stores vertices and indices in a format compatible with meshopt.
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Vertex positions (xyz format, tightly packed)
    pub positions: Vec<f32>,
    /// Triangle indices
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Create a new mesh from positions and indices
    pub fn new(positions: Vec<f32>, indices: Vec<u32>) -> Result<Self> {
        if positions.len() % 3 != 0 {
            anyhow::bail!(
                "Position count must be multiple of 3, got {}",
                positions.len()
            );
        }

        if indices.len() % 3 != 0 {
            anyhow::bail!(
                "Index count must be multiple of 3 (triangles), got {}",
                indices.len()
            );
        }

        let vertex_count = positions.len() / 3;
        for &idx in &indices {
            if idx as usize >= vertex_count {
                anyhow::bail!(
                    "Index {} out of bounds (vertex count: {})",
                    idx,
                    vertex_count
                );
            }
        }

        Ok(Self { positions, indices })
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Get triangle count
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Optimize mesh for vertex cache and overdraw
///
/// Applies two optimizations:
/// 1. **Vertex cache optimization**: Reorders indices for better GPU cache hit rate
/// 2. **Overdraw reduction**: Sorts triangles for early-Z rejection
///
/// ## Example
/// ```no_run
/// use astraweave_asset_pipeline::mesh::{Mesh, optimize_mesh};
///
/// # fn example() -> anyhow::Result<()> {
/// // Create a simple mesh (triangle)
/// let positions = vec![
///     0.0, 0.0, 0.0,  // Vertex 0
///     1.0, 0.0, 0.0,  // Vertex 1
///     0.5, 1.0, 0.0,  // Vertex 2
/// ];
/// let indices = vec![0, 1, 2];
///
/// let mesh = Mesh::new(positions, indices)?;
/// let (optimized, stats) = optimize_mesh(mesh)?;
///
/// println!("ACMR improved by {:.1}%", stats.acmr_improvement_percent);
/// println!("Overdraw reduced by {:.1}%", stats.overdraw_improvement_percent);
/// # Ok(())
/// # }
/// ```
pub fn optimize_mesh(mut mesh: Mesh) -> Result<(Mesh, MeshOptimizationStats)> {
    let start = std::time::Instant::now();

    let vertex_count = mesh.vertex_count();
    let index_count = mesh.indices.len();

    // Calculate ACMR before optimization
    let acmr_before = calculate_acmr(&mesh.indices, vertex_count);

    // Calculate overdraw before optimization (estimate)
    let overdraw_before = estimate_overdraw(&mesh);

    // Optimize vertex cache (reorder indices)
    optimize_vertex_cache_inplace(&mut mesh.indices, vertex_count)
        .context("Vertex cache optimization failed")?;

    // Optimize overdraw with meshopt (after vertex cache optimization)
    optimize_overdraw_inplace(&mut mesh.indices, &mesh.positions, vertex_count)
        .context("Overdraw optimization failed")?;

    // Calculate final metrics
    let acmr_after = calculate_acmr(&mesh.indices, vertex_count);
    let overdraw_after = estimate_overdraw(&mesh);

    let acmr_improvement_percent = 100.0 * (1.0 - acmr_after / acmr_before.max(0.001));
    let overdraw_improvement_percent = 100.0 * (1.0 - overdraw_after / overdraw_before.max(0.001));

    let elapsed = start.elapsed().as_millis() as u64;

    let stats = MeshOptimizationStats {
        vertex_count,
        index_count,
        acmr_before,
        acmr_after,
        acmr_improvement_percent,
        overdraw_before,
        overdraw_after,
        overdraw_improvement_percent,
        time_ms: elapsed,
    };

    tracing::info!(
        "Mesh optimized: {} vertices, {} indices, ACMR {:.2} → {:.2} ({:.1}% better), overdraw {:.2} → {:.2} ({:.1}% less)",
        vertex_count,
        index_count,
        acmr_before,
        acmr_after,
        acmr_improvement_percent,
        overdraw_before,
        overdraw_after,
        overdraw_improvement_percent
    );

    Ok((mesh, stats))
}

/// Optimize vertex cache in-place
fn optimize_vertex_cache_inplace(indices: &mut [u32], _vertex_count: usize) -> Result<()> {
    // meshopt reorders indices for better cache utilization
    // Note: meshopt 0.3 API simplified, now just takes indices
    let optimized = optimize_vertex_cache(indices, indices.len());
    indices.copy_from_slice(&optimized);
    Ok(())
}

/// Calculate ACMR (Average Cache Miss Ratio) for index buffer
///
/// ACMR measures cache efficiency:
/// - Lower is better (more cache hits)
/// - Optimal: ~0.5 for 32-entry cache
/// - Unoptimized: 1.5-3.0 typical
fn calculate_acmr(indices: &[u32], _vertex_count: usize) -> f32 {
    if indices.is_empty() {
        return 0.0;
    }

    // Simulate a 32-entry FIFO cache (typical GPU vertex cache)
    const CACHE_SIZE: usize = 32;
    let mut cache = [u32::MAX; CACHE_SIZE];
    let mut cache_pos = 0;
    let mut cache_misses = 0;

    for &index in indices {
        // Check if index is in cache
        if !cache.contains(&index) {
            // Cache miss
            cache_misses += 1;

            // Add to cache (FIFO)
            cache[cache_pos] = index;
            cache_pos = (cache_pos + 1) % CACHE_SIZE;
        }
    }

    // ACMR = misses per triangle
    cache_misses as f32 / (indices.len() / 3) as f32
}

/// Estimate overdraw ratio for mesh using meshopt's software rasterizer.
///
/// Returns the overdraw ratio (1.0 = no overdraw, >1.0 = pixels shaded multiple times).
fn estimate_overdraw(mesh: &Mesh) -> f32 {
    if mesh.indices.is_empty() || mesh.positions.is_empty() {
        return 1.0;
    }

    let vertex_stride = std::mem::size_of::<f32>() * 3;
    let positions_bytes: &[u8] = cast_slice(&mesh.positions);

    match VertexDataAdapter::new(positions_bytes, vertex_stride, 0) {
        Ok(vertices) => {
            let stats = meshopt::analyze_overdraw(&mesh.indices, &vertices);
            // overdraw is the ratio of pixels_shaded / pixels_covered
            // A value of 1.0 means perfect (no overdraw)
            stats.overdraw
        }
        Err(_) => 1.0,
    }
}

/// Optimize overdraw using meshopt
///
/// Must be called AFTER vertex cache optimization.
/// threshold: allows slight degradation of vertex cache efficiency to reduce overdraw
/// (1.05 = up to 5% degradation)
fn optimize_overdraw_inplace(
    indices: &mut [u32],
    positions: &[f32],
    _vertex_count: usize,
) -> Result<()> {
    // Create VertexDataAdapter for meshopt
    // positions are tightly packed f32 xyz, so stride is 3 * sizeof(f32) = 12 bytes
    let vertex_stride = std::mem::size_of::<f32>() * 3;
    let position_offset = 0; // positions start at byte 0

    // Convert positions to bytes for VertexDataAdapter
    let positions_bytes: &[u8] = cast_slice(positions);

    let vertices = VertexDataAdapter::new(positions_bytes, vertex_stride, position_offset)
        .context("Failed to create vertex data adapter")?;

    // threshold of 1.05 allows up to 5% vertex cache degradation to improve overdraw
    let threshold = 1.05;
    optimize_overdraw_in_place(indices, &vertices, threshold);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_creation() {
        let positions = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0];
        let indices = vec![0, 1, 2];

        let mesh = Mesh::new(positions, indices).expect("Mesh creation failed");
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_mesh_validation() {
        // Invalid position count (not multiple of 3)
        let result = Mesh::new(vec![0.0, 0.0], vec![0]);
        assert!(result.is_err());

        // Invalid index count (not multiple of 3)
        let result = Mesh::new(vec![0.0, 0.0, 0.0], vec![0, 1]);
        assert!(result.is_err());

        // Out of bounds index
        let result = Mesh::new(vec![0.0, 0.0, 0.0], vec![0, 1, 5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_acmr_calculation() {
        // Perfect cache reuse (triangle strip)
        let indices = vec![0, 1, 2, 1, 2, 3, 2, 3, 4];
        let acmr = calculate_acmr(&indices, 5);

        // Should be reasonable (triangle strip has good cache reuse)
        // Note: ACMR depends on cache size and order, < 2.0 is acceptable
        assert!(acmr < 2.0, "ACMR too high: {}", acmr);
    }

    #[test]
    fn test_mesh_optimization() {
        // Create a simple quad (2 triangles)
        let positions = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            1.0, 1.0, 0.0, // 2
            0.0, 1.0, 0.0, // 3
        ];

        // Intentionally bad order (no cache reuse)
        let indices = vec![0, 1, 2, 3, 0, 2];

        let mesh = Mesh::new(positions, indices).expect("Mesh creation failed");
        let (optimized, stats) = optimize_mesh(mesh).expect("Optimization failed");

        assert_eq!(optimized.vertex_count(), 4);
        assert_eq!(optimized.triangle_count(), 2);
        assert!(
            stats.acmr_after <= stats.acmr_before,
            "ACMR should improve or stay same"
        );
    }
}
