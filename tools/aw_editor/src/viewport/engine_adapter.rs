use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

use super::camera::OrbitCamera;
// Phase 1.B (`EditorTerrainSplat`) was superseded by
// `Renderer::terrain_forward` in 1.E.3. The wrapper type stays on disk
// as reference material; the adapter no longer imports it. See §9 of
// `docs/current/TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md` for the supersession
// deviation entry.
use super::types::{
    find_assets_dir, ScatterPlacement, TerrainFogParams, TerrainLightingParams, TerrainVertex,
    WaterStyle, MATERIAL_NAMES,
};

/// Render mode for the editor viewport.
///
/// Controls whether the viewport uses the full engine PBR pipeline or a
/// lightweight cube-based preview for fast iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Full engine PBR rendering (default): sky, shadows, PBR materials,
    /// water, weather particles, post-processing via `astraweave-render`.
    EnginePBR,
    /// Fast preview: cube placeholders per entity, simple gradient skybox,
    /// no PBR materials. Faster on weak GPUs or very large scenes.
    FastPreview,
}

impl Default for RenderMode {
    fn default() -> Self {
        Self::EnginePBR
    }
}

/// Editor rendering quality preset.
///
/// Controls shadow quality and post-processing complexity to balance
/// visual fidelity vs. frame time in the editor viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorQualityPreset {
    /// Full game-quality rendering: 2 CSM cascades at full resolution,
    /// all post-processing effects enabled. Use for final preview.
    GameQuality,
    /// Editor default: reduced shadow quality (smaller PCF, narrower cascades),
    /// only SSAO + Bloom + Tonemap post-processing. Good balance for editing.
    EditorDefault,
    /// Terrain-optimised: 2-cascade shadows, SSAO, bloom + tonemap.
    /// Applied automatically when terrain is loaded. Strikes a balance
    /// between visual fidelity (grounded shadows, AO) and performance.
    EditorTerrain,
    /// Minimal: shadows disabled, tonemap only. Maximum performance for
    /// large scenes or weak GPUs.
    Minimal,
}

impl Default for EditorQualityPreset {
    fn default() -> Self {
        Self::EditorDefault
    }
}

#[derive(Clone)]
struct ScatterPrimitiveLodAssets {
    full_mesh: astraweave_render::mesh::CpuMesh,
    simplified_mesh: astraweave_render::mesh::CpuMesh,
}

#[derive(Clone)]
struct ScatterLodAssets {
    primitives: Vec<ScatterPrimitiveLodAssets>,
    cross_billboard: astraweave_render::mesh::CpuMesh,
    impostor_card: astraweave_render::mesh::CpuMesh,
    aabb_min_y: f32,
    model_height: f32,
    model_half_width: f32,
}

const TERRAIN_CLUSTER_GRID: usize = 2;
const TERRAIN_MAX_VERTICES_PER_CLUSTER: usize = 5_000_000;

const TERRAIN_BIOME_TINTS: [[f32; 4]; 8] = [
    [0.80, 1.00, 0.70, 1.0],
    [1.10, 1.00, 0.75, 1.0],
    [0.60, 0.85, 0.50, 1.0],
    [0.85, 0.85, 0.80, 1.0],
    [1.05, 1.05, 1.10, 1.0],
    [0.70, 0.80, 0.55, 1.0],
    [1.10, 1.05, 0.85, 1.0],
    [0.75, 0.90, 0.85, 1.0],
];

const TERRAIN_MATERIAL_TINTS: [[f32; 4]; 22] = [
    [0.80, 1.00, 0.70, 1.0],
    [1.10, 1.00, 0.75, 1.0],
    [0.72, 0.88, 0.58, 1.0],
    [0.85, 0.85, 0.80, 1.0],
    [1.05, 1.05, 1.10, 1.0],
    [0.78, 0.68, 0.52, 1.0],
    [0.92, 0.80, 0.62, 1.0],
    [0.82, 0.84, 0.86, 1.0],
    [0.72, 0.74, 0.78, 1.0],
    [0.84, 0.74, 0.58, 1.0],
    [0.80, 0.80, 0.82, 1.0],
    [0.90, 0.86, 0.78, 1.0],
    [0.92, 0.92, 0.92, 1.0],
    [0.82, 0.80, 0.76, 1.0],
    [0.86, 0.96, 1.08, 1.0],
    [0.88, 0.70, 0.54, 1.0],
    [0.70, 0.88, 0.60, 1.0],
    [0.96, 0.94, 0.90, 1.0],
    [0.76, 0.84, 0.70, 1.0],
    [0.90, 0.62, 0.54, 1.0],
    [0.72, 0.56, 0.42, 1.0],
    [0.70, 0.92, 0.62, 1.0],
];

#[derive(Clone, Debug, Default)]
struct TerrainSurfaceSummary {
    biome_weights: [f32; 8],
    material_weights: [f32; 22],
}

impl TerrainSurfaceSummary {
    fn add_vertex(&mut self, vertex: &TerrainVertex) {
        let biome_weights = [
            vertex.biome_weights_0[0],
            vertex.biome_weights_0[1],
            vertex.biome_weights_0[2],
            vertex.biome_weights_0[3],
            vertex.biome_weights_1[0],
            vertex.biome_weights_1[1],
            vertex.biome_weights_1[2],
            vertex.biome_weights_1[3],
        ];
        for (idx, weight) in biome_weights.iter().enumerate() {
            self.biome_weights[idx] += weight.max(0.0);
        }

        for slot in 0..4 {
            let weight = vertex.material_weights[slot].max(0.0);
            if weight <= 0.0 {
                continue;
            }
            let material_id = vertex.material_ids[slot].round();
            if !material_id.is_finite() || material_id < 0.0 {
                continue;
            }
            let material_idx = material_id as usize;
            if material_idx < self.material_weights.len() {
                self.material_weights[material_idx] += weight;
            }
        }
    }

    fn merge(&mut self, other: &Self) {
        for (dst, src) in self
            .biome_weights
            .iter_mut()
            .zip(other.biome_weights.iter())
        {
            *dst += *src;
        }
        for (dst, src) in self
            .material_weights
            .iter_mut()
            .zip(other.material_weights.iter())
        {
            *dst += *src;
        }
    }

    fn dominant_material_index(&self) -> Option<usize> {
        self.material_weights
            .iter()
            .copied()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .and_then(|(index, weight)| (weight > 0.0001).then_some(index))
    }

    fn resolve_tint(&self) -> [f32; 4] {
        let biome_tint = weighted_palette_tint(&self.biome_weights, &TERRAIN_BIOME_TINTS);
        let material_tint = weighted_palette_tint(&self.material_weights, &TERRAIN_MATERIAL_TINTS);

        match (biome_tint, material_tint) {
            (Some(biome), Some(material)) => blend_tints(biome, material, 0.65),
            (Some(biome), None) => biome,
            (None, Some(material)) => material,
            (None, None) => [0.90, 0.90, 0.90, 1.0],
        }
    }
}

#[derive(Clone, Debug)]
struct TerrainChunkRenderData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    surface_summary: TerrainSurfaceSummary,
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
}

#[derive(Clone, Debug)]
struct TerrainClusterRecord {
    name: String,
    chunk_indices: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
struct TerrainChunkPlanningInfo {
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
    vertex_count: usize,
}

#[derive(Clone, Debug)]
struct TerrainMaterialSurfaceSet {
    albedo: (u32, u32, Vec<u8>),
    normal: (u32, u32, Vec<u8>),
    metallic_roughness: (u32, u32, Vec<u8>),
}

impl From<&TerrainChunkRenderData> for TerrainChunkPlanningInfo {
    fn from(chunk: &TerrainChunkRenderData) -> Self {
        Self {
            aabb_min: chunk.aabb_min,
            aabb_max: chunk.aabb_max,
            vertex_count: chunk.positions.len(),
        }
    }
}

/// Coarse world-space regular grid of terrain heights, rebuilt from the
/// full vertex set each time terrain chunks change.
///
/// Used by [`EngineRenderAdapter::upload_scatter_placements`] to re-sample
/// each placement's Y against the live terrain surface — preventing the
/// "floating / sunk scatter" artefact (Issue #6 from the 2026-04 editor
/// audit) that occurs when vegetation is generated against a stale height
/// or when the heightmap is edited after scatter generation.
///
/// # Sampling
///
/// Cells store the maximum height of any vertex within their XZ bounds,
/// so the grid represents an upper envelope rather than an average. This
/// keeps trees planted on top of micro-ridges instead of being sunk into
/// the mean of neighbouring valley vertices — a better fit for visual
/// grounding.
///
/// Missing cells (no vertex covered the cell) hold `f32::NAN`. Bilinear
/// sampling falls back to nearest-neighbour when any of the 4 samples
/// is `NaN` to handle sparse/edge coverage gracefully.
///
/// # Memory
///
/// Capped at `MAX_CELLS` (~4 M ≈ 16 MB). The grid is discarded in
/// [`EngineRenderAdapter::clear_terrain`] and rebuilt from scratch on the
/// next terrain upload — editing the heightmap already triggers a
/// chunk re-upload, which naturally refreshes the grid.
#[derive(Clone, Debug)]
struct TerrainHeightGrid {
    origin_x: f32,
    origin_z: f32,
    cell_size: f32,
    width: usize,
    height: usize,
    /// Row-major. `data[z * width + x]`. `NaN` means "no sample".
    data: Vec<f32>,
}

impl TerrainHeightGrid {
    const MIN_CELL_SIZE: f32 = 0.5;
    const MAX_CELL_SIZE: f32 = 8.0;
    const MAX_CELLS: usize = 4 * 1024 * 1024;

    /// Build a height grid from the full set of terrain vertices.
    ///
    /// Returns `None` when there are no vertices or the AABB degenerates
    /// to a point.
    fn build(chunks: &[TerrainChunkRenderData]) -> Option<Self> {
        // Compute world-space AABB over XZ and estimate a reasonable cell size
        // from the average vertex spacing so the grid matches heightmap
        // resolution without over-blurring.
        let mut min_x = f32::MAX;
        let mut min_z = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_z = f32::MIN;
        let mut total_verts = 0usize;
        for chunk in chunks {
            if chunk.positions.is_empty() {
                continue;
            }
            total_verts += chunk.positions.len();
            min_x = min_x.min(chunk.aabb_min[0]);
            min_z = min_z.min(chunk.aabb_min[2]);
            max_x = max_x.max(chunk.aabb_max[0]);
            max_z = max_z.max(chunk.aabb_max[2]);
        }
        if total_verts == 0 {
            return None;
        }
        let extent_x = (max_x - min_x).max(0.0);
        let extent_z = (max_z - min_z).max(0.0);
        if extent_x < 0.01 || extent_z < 0.01 {
            return None;
        }

        // Target cell size = sqrt(area / verts) — matches vertex density.
        // Clamp to [MIN_CELL_SIZE, MAX_CELL_SIZE] so coarse meshes still
        // get a usable grid and dense meshes don't blow the memory cap.
        let area = extent_x * extent_z;
        let density_cell = (area / total_verts as f32).sqrt();
        let mut cell_size =
            density_cell.clamp(Self::MIN_CELL_SIZE, Self::MAX_CELL_SIZE);

        let compute_dims = |cs: f32| -> (usize, usize) {
            let w = ((extent_x / cs).ceil() as usize + 1).max(2);
            let h = ((extent_z / cs).ceil() as usize + 1).max(2);
            (w, h)
        };
        let (mut width, mut height) = compute_dims(cell_size);

        // Enforce the memory cap by inflating cell_size if necessary.
        while width.saturating_mul(height) > Self::MAX_CELLS
            && cell_size < Self::MAX_CELL_SIZE
        {
            cell_size = (cell_size * 2.0).min(Self::MAX_CELL_SIZE);
            let (w, h) = compute_dims(cell_size);
            width = w;
            height = h;
        }

        let cell_count = width.saturating_mul(height);
        if cell_count == 0 || cell_count > Self::MAX_CELLS {
            return None;
        }

        let mut data = vec![f32::NAN; cell_count];
        let inv_cell = 1.0 / cell_size;
        for chunk in chunks {
            for pos in &chunk.positions {
                let fx = (pos[0] - min_x) * inv_cell;
                let fz = (pos[2] - min_z) * inv_cell;
                if !fx.is_finite() || !fz.is_finite() {
                    continue;
                }
                let ix = (fx as usize).min(width - 1);
                let iz = (fz as usize).min(height - 1);
                let idx = iz * width + ix;
                let y = pos[1];
                let cur = data[idx];
                data[idx] = if cur.is_nan() { y } else { cur.max(y) };
            }
        }

        Some(Self {
            origin_x: min_x,
            origin_z: min_z,
            cell_size,
            width,
            height,
            data,
        })
    }

    /// Bilinear sample at world position. Returns `None` outside the grid
    /// extent or when any of the 4 surrounding cells has no sample.
    #[inline]
    fn sample(&self, world_x: f32, world_z: f32) -> Option<f32> {
        if self.width < 2 || self.height < 2 {
            return None;
        }
        let inv_cell = 1.0 / self.cell_size;
        let fx = (world_x - self.origin_x) * inv_cell;
        let fz = (world_z - self.origin_z) * inv_cell;
        if !fx.is_finite() || !fz.is_finite() {
            return None;
        }
        if fx < 0.0 || fz < 0.0 {
            return None;
        }
        let max_x = (self.width - 1) as f32;
        let max_z = (self.height - 1) as f32;
        if fx > max_x || fz > max_z {
            return None;
        }
        let x0 = fx.floor() as usize;
        let z0 = fz.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let z1 = (z0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let tz = fz - z0 as f32;

        let h00 = self.data[z0 * self.width + x0];
        let h10 = self.data[z0 * self.width + x1];
        let h01 = self.data[z1 * self.width + x0];
        let h11 = self.data[z1 * self.width + x1];

        // If any corner is missing, fall back to nearest valid corner so
        // edge placements still get a reasonable height.
        if h00.is_nan() || h10.is_nan() || h01.is_nan() || h11.is_nan() {
            let mut best: Option<(f32, f32)> = None; // (dist², height)
            for (dx, dz, h) in [
                (tx, tz, h00),
                (1.0 - tx, tz, h10),
                (tx, 1.0 - tz, h01),
                (1.0 - tx, 1.0 - tz, h11),
            ] {
                if h.is_nan() {
                    continue;
                }
                let d2 = dx * dx + dz * dz;
                match best {
                    Some((bd, _)) if bd <= d2 => {}
                    _ => best = Some((d2, h)),
                }
            }
            return best.map(|(_, h)| h);
        }

        let hx0 = h00 * (1.0 - tx) + h10 * tx;
        let hx1 = h01 * (1.0 - tx) + h11 * tx;
        Some(hx0 * (1.0 - tz) + hx1 * tz)
    }
}

fn weighted_palette_tint<const N: usize>(
    weights: &[f32; N],
    palette: &[[f32; 4]; N],
) -> Option<[f32; 4]> {
    let total_weight: f32 = weights.iter().copied().sum();
    if total_weight <= 0.0001 {
        return None;
    }

    let mut tint = [0.0; 4];
    for (weight, color) in weights.iter().zip(palette.iter()) {
        tint[0] += color[0] * *weight;
        tint[1] += color[1] * *weight;
        tint[2] += color[2] * *weight;
    }

    tint[0] /= total_weight;
    tint[1] /= total_weight;
    tint[2] /= total_weight;
    tint[3] = 1.0;
    Some(tint)
}

fn blend_tints(a: [f32; 4], b: [f32; 4], b_weight: f32) -> [f32; 4] {
    let a_weight = 1.0 - b_weight;
    [
        a[0] * a_weight + b[0] * b_weight,
        a[1] * a_weight + b[1] * b_weight,
        a[2] * a_weight + b[2] * b_weight,
        1.0,
    ]
}

/// Derive a distance-fog colour from a procedural sky config so that
/// fogged terrain edges blend seamlessly into the horizon.
///
/// Uses `day_color_horizon` directly — that is the colour the sky shader
/// paints right at the horizon line, which is exactly where distant
/// terrain fades out. Keeping this a function (rather than inlining the
/// field read) makes the coupling explicit and gives a single place to
/// evolve the derivation later (e.g. blend toward `sunset_color_horizon`
/// based on time-of-day).
fn fog_color_from_sky(sky: &astraweave_render::SkyConfig) -> glam::Vec3 {
    sky.day_color_horizon
}

/// Infer the square-surface-grid dimension `N` from a terrain chunk's
/// total vertex count, including any edge skirts the editor appends after
/// the surface grid.
///
/// Editor chunks have one of two layouts:
/// * `N² + 4N` vertices when all 4 edge skirts are present (the default
///   produced by `terrain_integration::TerrainState::build_chunk_mesh`).
/// * `N²` vertices when no skirts are attached (e.g., test fixtures).
///
/// The "with skirts" shape is the inverse of `total = N² + 4N`, which
/// rearranges to `(N+2)² = total + 4`, so `N = sqrt(total + 4) - 2`.
///
/// Returns `None` if `vertex_count` doesn't match either layout exactly —
/// a defensive guard against malformed input that would otherwise produce
/// incorrect index filtering (the Phase 1 post-completion regression
/// originated from `floor(sqrt(N² + 4N))` = `N+2` overshooting and
/// producing a `surface_idx_count` that pulled skirt triangles into the
/// filtered set).
fn infer_surface_grid_dim(vertex_count: usize) -> Option<usize> {
    // With-skirts layout: (N+2)² = total + 4.
    let sqrt_disc = ((vertex_count + 4) as f64).sqrt().round() as usize;
    if sqrt_disc >= 4 {
        let n = sqrt_disc - 2;
        if n >= 2 && n.checked_mul(n).and_then(|nn| nn.checked_add(4 * n)) == Some(vertex_count) {
            return Some(n);
        }
    }
    // Plain-square layout: total = N².
    let n_plain = (vertex_count as f64).sqrt().round() as usize;
    if n_plain >= 2 && n_plain.checked_mul(n_plain) == Some(vertex_count) {
        return Some(n_plain);
    }
    None
}

/// Filter an editor-side terrain index buffer to triangles whose three
/// corners all reference valid surface vertices (indices `< surface_vert_count`).
///
/// Used by Phase 1's forward-path chunk upload: the editor's index buffer
/// contains both surface triangles (all 3 corners reference surface
/// vertices in `[0, surface_vert_count)`) and skirt triangles (2 corners
/// are surface vertices, 2 are skirt vertices at indices `≥ surface_vert_count`).
/// Skirt triangles must be dropped because the forward path's vertex
/// buffer is the surface prefix only; leaving them would index past the
/// end of the truncated buffer and produce degenerate triangles.
///
/// Processes the input in triangle-sized chunks. Silently drops a
/// trailing partial triangle (1-2 stray indices) if present. The caller
/// already ensured `indices.len() % 3 == 0` in practice, but the defensive
/// handling keeps this helper robust to future edits.
fn filter_surface_triangles(indices: &[u32], surface_vert_count: u32) -> Vec<u32> {
    let mut out = Vec::with_capacity(indices.len());
    for tri in indices.chunks_exact(3) {
        if tri[0] < surface_vert_count
            && tri[1] < surface_vert_count
            && tri[2] < surface_vert_count
        {
            out.extend_from_slice(tri);
        }
    }
    out
}

fn build_terrain_cluster_plan(
    chunks: &[TerrainChunkPlanningInfo],
    grid: usize,
    max_vertices_per_cluster: usize,
) -> Vec<Vec<usize>> {
    if chunks.is_empty() || grid == 0 {
        return Vec::new();
    }

    let mut global_aabb_min = [f32::MAX; 3];
    let mut global_aabb_max = [f32::MIN; 3];
    for chunk in chunks {
        for axis in 0..3 {
            global_aabb_min[axis] = global_aabb_min[axis].min(chunk.aabb_min[axis]);
            global_aabb_max[axis] = global_aabb_max[axis].max(chunk.aabb_max[axis]);
        }
    }

    let span_x = (global_aabb_max[0] - global_aabb_min[0]).max(1.0);
    let span_z = (global_aabb_max[2] - global_aabb_min[2]).max(1.0);
    let cell_w = span_x / grid as f32;
    let cell_d = span_z / grid as f32;

    let mut bins: Vec<Vec<usize>> = vec![Vec::new(); grid * grid];
    for (chunk_index, chunk) in chunks.iter().enumerate() {
        let center_x = (chunk.aabb_min[0] + chunk.aabb_max[0]) * 0.5;
        let center_z = (chunk.aabb_min[2] + chunk.aabb_max[2]) * 0.5;
        let gx = (((center_x - global_aabb_min[0]) / cell_w) as usize).min(grid - 1);
        let gz = (((center_z - global_aabb_min[2]) / cell_d) as usize).min(grid - 1);
        bins[gz * grid + gx].push(chunk_index);
    }

    let mut clusters = Vec::new();
    for bin in bins {
        if bin.is_empty() {
            continue;
        }

        let mut chunk_indices = Vec::new();
        let mut cluster_vertex_count = 0usize;
        for chunk_index in bin {
            let next_vertex_count = chunks[chunk_index].vertex_count;
            if !chunk_indices.is_empty()
                && cluster_vertex_count + next_vertex_count > max_vertices_per_cluster
            {
                clusters.push(std::mem::take(&mut chunk_indices));
                cluster_vertex_count = 0;
            }

            chunk_indices.push(chunk_index);
            cluster_vertex_count += next_vertex_count;
        }

        if !chunk_indices.is_empty() {
            clusters.push(chunk_indices);
        }
    }

    clusters
}

pub struct EngineRenderAdapter {
    renderer: astraweave_render::Renderer,
    initialized: bool,
    /// Tracks which clustered terrain model names are currently uploaded.
    terrain_model_names: Vec<String>,
    /// Converted source terrain chunks preserved in clustered upload order.
    terrain_chunks: Vec<TerrainChunkRenderData>,
    /// Maps logical source chunk indices to entries in `terrain_chunks`.
    /// Empty logical chunks retain identity as `None` so incremental updates
    /// do not misroute after clustered uploads.
    terrain_chunk_slot_map: Vec<Option<usize>>,
    /// Stable cluster ownership records used for targeted rebuilds after brush edits.
    terrain_clusters: Vec<TerrainClusterRecord>,
    /// Number of logical terrain chunks supplied by the editor.
    terrain_source_chunk_count: usize,
    /// Tracks scatter model names for cleanup.
    scatter_model_names: Vec<String>,
    /// Total terrain triangles across all uploaded chunks.
    terrain_total_triangles: usize,
    /// Total terrain indices across all uploaded chunks.
    terrain_total_indices: usize,
    /// Total scatter instances currently uploaded after LOD + density filtering.
    scatter_placement_count: usize,
    /// Number of scatter submodels currently uploaded to the renderer.
    scatter_draw_call_count: u32,
    /// Total scatter triangles represented by the currently uploaded models.
    scatter_total_triangles: usize,
    /// Total scatter vertices represented by the currently uploaded models.
    scatter_total_vertices: usize,
    /// Whether weather effects are currently active.
    weather_active: bool,
    /// Whether water rendering is enabled.
    water_enabled: bool,
    /// Current editor quality preset (shadows + post-processing).
    quality_preset: EditorQualityPreset,
    /// Cached entity count + selection for dirty-skip in feed_entities
    cached_entity_feed_count: usize,
    /// Cached selected entity set for feed_entities dirty check
    cached_entity_feed_selected: Vec<astraweave_core::Entity>,
    /// Cached mesh map length for feed_entities dirty check
    cached_entity_feed_mesh_count: usize,
    /// Persistent cache: glTF mesh path → loaded CpuMesh data.
    /// Survives across biome regenerations so the same .glb files are not
    /// re-parsed from disk every time scatter placements are recomputed.
    scatter_cpu_mesh_cache:
        std::collections::HashMap<String, Vec<astraweave_render::mesh::CpuMesh>>,
    /// Persistent cache: texture canonical path → decoded RGBA pixels.
    /// Prevents the same diffuse texture from being decoded multiple times
    /// across regenerations.
    scatter_texture_cache: std::collections::HashMap<std::path::PathBuf, (u32, u32, Vec<u8>)>,
    /// Persistent cache: mesh path -> derived LOD assets used by the editor's
    /// shared-policy vegetation uploader.
    scatter_lod_asset_cache: std::collections::HashMap<String, ScatterLodAssets>,
    /// Whether terrain surface maps have been uploaded to the renderer's
    /// global terrain material slots. This starts with a procedural fallback
    /// and is later replaced by authored biome maps when available.
    terrain_detail_texture_uploaded: bool,
    /// Cached decoded terrain surface triplets keyed by canonical material index.
    terrain_material_surfaces: std::collections::HashMap<usize, TerrainMaterialSurfaceSet>,
    /// Shared prototype models that own per-material terrain surface bind groups.
    terrain_material_prototypes: std::collections::HashMap<usize, String>,
    /// Current camera position for LOD selection and density scaling.
    /// Updated every frame via `update_camera()`.
    camera_position: glam::Vec3,
    /// Current camera yaw in radians. Used for impostor-card facing.
    camera_yaw: f32,
    /// Camera position at the time of the last scatter LOD refresh.
    /// When the camera moves enough from this point, scatter LOD levels are
    /// recomputed.
    scatter_lod_camera_pos: glam::Vec3,
    /// Camera yaw at the last scatter LOD refresh. Impostor cards use this to
    /// refresh facing without waiting for a large translation.
    scatter_lod_camera_yaw: f32,
    /// Stored scatter placements for camera-driven LOD refresh.
    /// Kept after initial upload so `refresh_scatter_lod()` can re-bucket
    /// instances without regenerating terrain.
    scatter_placements: Vec<ScatterPlacement>,
    /// Diffuse texture paths for scatter refresh (retained from last upload).
    scatter_diffuse_textures: std::collections::HashMap<String, std::path::PathBuf>,
    /// Billboard CpuMesh cache per mesh key (generated from model AABB).
    scatter_billboard_cache: std::collections::HashMap<String, astraweave_render::mesh::CpuMesh>,
    // ── Chunk streaming ──────────────────────────────────────────────
    /// Per-chunk model names for targeted unload. When a chunk leaves the
    /// active set, only its models are removed from the renderer.
    scatter_chunk_models: std::collections::HashMap<astraweave_terrain::ChunkId, Vec<String>>,
    /// Chunks currently uploaded to the GPU.
    active_scatter_chunks: std::collections::HashSet<astraweave_terrain::ChunkId>,
    /// Camera's current chunk (updated each frame). Used to detect chunk
    /// boundary crossings that trigger streaming load/unload.
    camera_chunk: astraweave_terrain::ChunkId,
    /// Terrain chunk size in world units (from config). Needed to convert
    /// world positions to ChunkId.
    scatter_chunk_size: f32,
    /// Monotonic wall-clock timestamp of the last scatter LOD refresh.
    /// Used to rate-limit rebucketing during continuous camera motion.
    /// `None` until the first refresh completes.
    scatter_last_refresh: Option<std::time::Instant>,
    /// Coarse world-space height grid rebuilt each time terrain chunks
    /// are uploaded. Used to re-sample scatter Y so vegetation stays on
    /// the surface after heightmap edits. `None` when no terrain is
    /// loaded or the grid build failed.
    terrain_height_grid: Option<TerrainHeightGrid>,
    /// Phase 5.3 T7 stage 3c.2: per-scatter-mesh impostor atlas registry.
    /// Populated lazily on the first LOD3 encounter (stage 3c.3 will wire
    /// the LOD3 path to this). `None` means the feature is unavailable in
    /// this build; `Some` means impostor-bake is compiled in and the
    /// registry's disk cache root is reachable.
    #[cfg(feature = "impostor-bake")]
    impostor_registry: Option<super::impostor_registry::ImpostorRegistry>,
    /// Phase 5.3 T7 stage 3c.2: keys of impostor passes currently installed
    /// on `self.renderer`. Used by `clear_scatter` / LOD refresh to retire
    /// passes whose underlying scatter is no longer loaded, mirroring the
    /// `scatter_model_names` retirement pattern for PBR models.
    #[cfg(feature = "impostor-bake")]
    installed_impostor_keys: std::collections::HashSet<String>,
}

impl EngineRenderAdapter {
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // Push a validation error scope so that any wgpu validation errors during
        // renderer creation are captured here instead of reaching the uncaptured
        // error handler (which panics by default in eframe). This gives us the
        // full, detailed error message for diagnosis.
        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let device_owned = (*device).clone();
        let queue_owned = (*queue).clone();

        let renderer_result =
            astraweave_render::Renderer::new_from_device(device_owned, queue_owned, None, config)
                .await;

        // Pop the error scope to check for captured validation errors.
        // If a validation error occurred, it was silently captured instead of
        // panicking, so we can report the full details.
        if let Some(wgpu_err) = device.pop_error_scope().await {
            let msg = format!(
                "GPU validation error during engine renderer init:\n{wgpu_err}\n\nDebug:\n{wgpu_err:?}"
            );
            tracing::error!("{msg}");
            return Err(anyhow::anyhow!("{msg}"));
        }

        let renderer = match renderer_result {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("Engine renderer creation failed: {e:#}");
                tracing::error!("{msg}");
                return Err(anyhow::anyhow!("{msg}"));
            }
        };

        // Terrain Material System campaign — Phase 1.E.5.
        // The Phase 1.B `EditorTerrainSplat` field was removed here; its
        // role is now played by `Renderer::terrain_forward`, initialized
        // lazily in `upload_terrain_chunks` via `renderer.init_terrain_forward()`.

        let mut adapter = Self {
            renderer,
            initialized: true,
            terrain_model_names: Vec::new(),
            terrain_chunks: Vec::new(),
            terrain_chunk_slot_map: Vec::new(),
            terrain_clusters: Vec::new(),
            terrain_source_chunk_count: 0,
            scatter_model_names: Vec::new(),
            terrain_total_triangles: 0,
            terrain_total_indices: 0,
            scatter_placement_count: 0,
            scatter_draw_call_count: 0,
            scatter_total_triangles: 0,
            scatter_total_vertices: 0,
            weather_active: false,
            water_enabled: false,
            quality_preset: EditorQualityPreset::default(),
            cached_entity_feed_count: usize::MAX, // force first rebuild
            cached_entity_feed_selected: Vec::new(),
            cached_entity_feed_mesh_count: usize::MAX,
            scatter_cpu_mesh_cache: std::collections::HashMap::new(),
            scatter_texture_cache: std::collections::HashMap::new(),
            scatter_lod_asset_cache: std::collections::HashMap::new(),
            terrain_detail_texture_uploaded: false,
            terrain_material_surfaces: std::collections::HashMap::new(),
            terrain_material_prototypes: std::collections::HashMap::new(),
            camera_position: glam::Vec3::ZERO,
            camera_yaw: 0.0,
            scatter_lod_camera_pos: glam::Vec3::new(f32::MAX, f32::MAX, f32::MAX),
            scatter_lod_camera_yaw: f32::MAX,
            scatter_placements: Vec::new(),
            scatter_diffuse_textures: std::collections::HashMap::new(),
            scatter_billboard_cache: std::collections::HashMap::new(),
            scatter_chunk_models: std::collections::HashMap::new(),
            active_scatter_chunks: std::collections::HashSet::new(),
            camera_chunk: astraweave_terrain::ChunkId::new(0, 0),
            scatter_chunk_size: 256.0, // default; overridden on first scatter upload
            scatter_last_refresh: None,
            terrain_height_grid: None,
            #[cfg(feature = "impostor-bake")]
            impostor_registry: Some(super::impostor_registry::ImpostorRegistry::new(
                Self::default_impostor_cache_root(),
            )),
            #[cfg(feature = "impostor-bake")]
            installed_impostor_keys: std::collections::HashSet::new(),
        };
        adapter.apply_quality_preset(EditorQualityPreset::EditorDefault);
        Ok(adapter)
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn update_camera(&mut self, camera: &OrbitCamera) {
        // Pass the OrbitCamera's own view/proj matrices directly to the renderer.
        // This avoids yaw/pitch conversion issues between the orbit camera and
        // the engine camera's direction conventions.
        self.renderer.update_camera_matrices(
            camera.view_matrix(),
            camera.projection_matrix(),
            camera.position(),
            camera.near,
            camera.far,
            camera.fov.to_radians(),
            camera.aspect,
        );
        self.camera_position = camera.position();
        let camera_yaw = camera.yaw();
        self.camera_yaw = camera_yaw;

        // Stage 3c.3-b: keep every installed `ImpostorPass`'s camera UBO in
        // sync with the main camera every frame. Without this the billboards
        // would pick their atlas cell from the camera position captured at
        // refresh time and visibly snap when the camera rotates.
        #[cfg(feature = "impostor-bake")]
        {
            let view_proj = self.renderer.current_view_proj();
            let camera_pos = self.camera_position;
            self.renderer
                .update_all_impostor_cameras(view_proj, camera_pos);
        }

        // Detect chunk boundary crossings so the active placement filter can
        // be recomputed immediately.
        let new_chunk = astraweave_terrain::ChunkId::from_world_pos(
            self.camera_position,
            self.scatter_chunk_size,
        );

        let chunk_changed = new_chunk != self.camera_chunk;
        self.camera_chunk = new_chunk;

        if chunk_changed && !self.scatter_placements.is_empty() {
            self.stream_scatter_chunks();
        }

        // Rebucket vegetation only on meaningful translation. The prior
        // implementation refreshed on every 1 m of movement OR 0.01 rad
        // (~0.57°) of yaw change, which fired on essentially every camera
        // input and caused a full re-upload per frame during motion.
        //
        // Key insight: LOD3 impostor `face_yaw` is derived from
        // `(cam_pos - p.position)` per-instance — it does NOT depend on
        // camera yaw — so yaw-only refreshes were pure waste. We drop the
        // yaw trigger entirely, raise the translation threshold to 8 m
        // (roughly a quarter of the minimum LOD distance for trees), and
        // rate-limit refreshes to at most once per 250 ms during sustained
        // motion to keep frame times stable.
        const LOD_REFRESH_POSITION_THRESHOLD: f32 = 8.0;
        const LOD_REFRESH_MIN_INTERVAL: std::time::Duration =
            std::time::Duration::from_millis(250);
        let position_delta = self.camera_position - self.scatter_lod_camera_pos;
        let exceeded_distance = position_delta.length_squared()
            > LOD_REFRESH_POSITION_THRESHOLD * LOD_REFRESH_POSITION_THRESHOLD;
        let budget_elapsed = self
            .scatter_last_refresh
            .map(|t| t.elapsed() >= LOD_REFRESH_MIN_INTERVAL)
            .unwrap_or(true);
        let needs_scatter_refresh = exceeded_distance && budget_elapsed;

        if needs_scatter_refresh && !self.scatter_placements.is_empty() {
            self.refresh_scatter_lod();
        }
    }

    /// Render the scene into the given color target.
    ///
    /// `depth_view`: optional caller-provided depth attachment. When `Some(v)`, terrain + sky
    /// passes write depth into `v` (compatible Depth32Float texture required); enables external
    /// consumers (e.g., editor depth-pick at viewport/renderer.rs:read_depth_at_pixel) to read
    /// terrain depth post-render. When `None`, falls back to internal depth target.
    /// See Sub-phase 3 Mediator Brush Real-Fix per Round-5-Closure 569415a7a §12 Option (a).
    pub fn render_to_texture(
        &mut self,
        target: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let t0 = std::time::Instant::now();
        self.renderer
            .draw_into(target, depth_view, encoder)
            .context("Engine draw_into failed")?;
        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;

        // GPU per-pass timings (from the PREVIOUS frame — readback is async).
        // Log at info level once per second so users can see the GPU breakdown
        // without flooding the console.
        {
            use std::sync::atomic::{AtomicU64, Ordering};
            static LAST_GPU_LOG: AtomicU64 = AtomicU64::new(0);
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let prev = LAST_GPU_LOG.load(Ordering::Relaxed);
            if now_ms.saturating_sub(prev) >= 2000 {
                LAST_GPU_LOG.store(now_ms, Ordering::Relaxed);
                if let Some(prof) = self.renderer.gpu_profiler() {
                    let total = prof.total_gpu_ms();
                    if total > 0.0 {
                        let map = prof.results_map();
                        let breakdown: Vec<String> =
                            map.iter().map(|(k, v)| format!("{k}={v:.1}ms")).collect();
                        tracing::info!(
                            target: "aw_editor::viewport::perf",
                            "GPU frame: {total:.1}ms total | {} | CPU draw_into: {elapsed_ms:.1}ms | {}/{} models drawn",
                            breakdown.join(" "),
                            self.renderer.rendered_model_count(),
                            self.renderer.model_count(),
                        );
                    }
                }
            }
        }

        tracing::debug!(
            target: "aw_editor::viewport::perf",
            "draw_into CPU: {elapsed_ms:.2}ms, {} models in HashMap",
            self.renderer.model_count(),
        );
        Ok(())
    }

    pub fn renderer(&self) -> &astraweave_render::Renderer {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut astraweave_render::Renderer {
        &mut self.renderer
    }

    /// Get the current quality preset.
    pub fn quality_preset(&self) -> EditorQualityPreset {
        self.quality_preset
    }

    /// Get GPU memory usage statistics from the budget tracker.
    /// Returns (total_used_bytes, total_budget_bytes, usage_percentage).
    pub fn gpu_memory_stats(&self) -> (u64, u64, f32) {
        let budget = self.renderer.gpu_memory_budget();
        let usage_pct = budget.usage_percentage();
        if usage_pct > 80.0 {
            tracing::warn!(target: "aw_editor::viewport", "GPU memory high: {:.1}% ({} MB used)", usage_pct, budget.total_usage() / (1024 * 1024));
        }
        (
            budget.total_usage(),
            2 * 1024 * 1024 * 1024, // 2GB default
            usage_pct,
        )
    }

    /// Get per-category GPU memory snapshot.
    /// Returns Vec of (category, current_bytes, hard_limit_bytes).
    pub fn gpu_memory_snapshot(&self) -> Vec<(astraweave_render::MemoryCategory, u64, u64)> {
        self.renderer.gpu_memory_budget().snapshot()
    }

    /// Apply an editor quality preset, configuring shadows and post-processing.
    ///
    /// - `GameQuality`: Full shadows + all post-processing (for final preview)
    /// - `EditorDefault`: Reduced shadows + SSAO/Bloom/Tonemap only (balanced)
    /// - `Minimal`: No shadows + tonemap only (maximum performance)
    pub fn apply_quality_preset(&mut self, preset: EditorQualityPreset) {
        tracing::info!(target: "aw_editor::viewport", "Quality preset changed to: {:?}", preset);
        self.quality_preset = preset;

        match preset {
            EditorQualityPreset::GameQuality => {
                // Full game-quality shadows + cloud shadows
                self.renderer.set_shadows_enabled(true);
                self.renderer.set_cloud_shadows_enabled(true);
                self.renderer.set_shadow_filter(2.0, 0.005, 1.5);
                self.renderer.set_cascade_extents(40.0, 120.0);
                self.renderer.set_cascade_lambda(0.75);
                self.renderer.set_max_draw_distance(0.0); // fog-based

                // Full post-processing chain
                let chain = astraweave_render::hdr_pipeline::PostProcessChain {
                    ssao_enabled: false,
                    ssr_enabled: false,
                    ssgi_enabled: false,
                    god_rays_enabled: false,
                    auto_exposure_enabled: false,
                    bloom_enabled: true,
                    taa_enabled: true,
                    dof_enabled: false, // DoF off by default even in game quality
                    motion_blur_enabled: false,
                    color_grading_enabled: true,
                    tonemap_operator: astraweave_render::hdr_pipeline::TonemapOperator::Aces,
                };
                self.renderer.set_post_process_chain(chain);
            }
            EditorQualityPreset::EditorDefault => {
                // Enable a single tight shadow cascade so trees and props
                // self-shadow and cast contact shadows on the terrain. Prior
                // behaviour disabled shadows entirely to claw back frame
                // time, which left the scene reading as ambient-only (every
                // surface equally lit regardless of facing direction).
                //
                // Tight 40 m / 120 m extents keep the relevant cascade
                // focused on the viewport vicinity; far geometry uses the
                // second cascade (low cost because most pixels sample the
                // first cascade at editor-camera distances).
                self.renderer.set_shadows_enabled(true);
                self.renderer.set_cloud_shadows_enabled(false);
                self.renderer.set_cascade_extents(40.0, 120.0);
                self.renderer.set_cascade_lambda(0.75);
                self.renderer.set_shadow_filter(1.5, 0.005, 1.5);
                self.renderer.set_max_draw_distance(1200.0);
                // Bloom disabled — the multi-pass compute pipeline
                // (downsample + upsample mip chain) adds ~1-3ms per frame
                // from write_buffer + compute dispatch overhead.
                //
                // SSAO enabled at default quality: restores base-of-trunk
                // darkening, crevice contact, and general shape definition
                // without noticeable cost at 1920×1080.
                let chain = astraweave_render::hdr_pipeline::PostProcessChain {
                    ssao_enabled: true,
                    ssr_enabled: false,
                    ssgi_enabled: false,
                    god_rays_enabled: false,
                    auto_exposure_enabled: false,
                    bloom_enabled: false,
                    taa_enabled: false,
                    dof_enabled: false,
                    motion_blur_enabled: false,
                    color_grading_enabled: true,
                    tonemap_operator: astraweave_render::hdr_pipeline::TonemapOperator::Aces,
                };
                self.renderer.set_post_process_chain(chain);
            }
            EditorQualityPreset::EditorTerrain => {
                // Terrain-optimised: shadows disabled to maintain interactive
                // framerates with 4M+ terrain triangles. Cloud shadows also
                // disabled — the 512px transmittance map produces visible
                // noise on large terrain surfaces.
                self.renderer.set_shadows_enabled(false);
                self.renderer.set_cloud_shadows_enabled(false);
                self.renderer.set_max_draw_distance(1200.0);

                // Bloom disabled for terrain editing — the multi-pass compute
                // pipeline adds constant overhead per frame.
                //
                // SSAO enabled: restores crevice/contact shading on the
                // terrain and around scattered props without the cost of
                // full shadow cascades.
                let chain = astraweave_render::hdr_pipeline::PostProcessChain {
                    ssao_enabled: true,
                    ssr_enabled: false,
                    ssgi_enabled: false,
                    god_rays_enabled: false,
                    auto_exposure_enabled: false,
                    bloom_enabled: false,
                    taa_enabled: false,
                    dof_enabled: false,
                    motion_blur_enabled: false,
                    color_grading_enabled: true,
                    tonemap_operator: astraweave_render::hdr_pipeline::TonemapOperator::Aces,
                };
                self.renderer.set_post_process_chain(chain);
            }
            EditorQualityPreset::Minimal => {
                // Everything disabled for maximum performance
                self.renderer.set_shadows_enabled(false);
                self.renderer.set_cloud_shadows_enabled(false);
                self.renderer.set_max_draw_distance(800.0);

                // Minimal post-processing: tonemap only
                let chain = astraweave_render::hdr_pipeline::PostProcessChain {
                    ssao_enabled: false,
                    ssr_enabled: false,
                    ssgi_enabled: false,
                    god_rays_enabled: false,
                    auto_exposure_enabled: false,
                    bloom_enabled: false,
                    taa_enabled: false,
                    dof_enabled: false,
                    motion_blur_enabled: false,
                    color_grading_enabled: false,
                    tonemap_operator: astraweave_render::hdr_pipeline::TonemapOperator::Aces,
                };
                self.renderer.set_post_process_chain(chain);
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.renderer.resize(width, height);
        }
    }

    pub fn load_gltf_model(&mut self, name: impl Into<String>, path: &Path) -> Result<()> {
        use astraweave_render::{mesh_gltf, Instance};

        let name = name.into();
        tracing::info!("Loading glTF model '{}' from: {}", name, path.display());

        let opts = mesh_gltf::GltfOptions::default();
        let cpu_meshes = mesh_gltf::load_gltf(path, &opts)
            .with_context(|| format!("Failed to load glTF: {}", path.display()))?;

        if cpu_meshes.is_empty() {
            anyhow::bail!("glTF file contains no meshes: {}", path.display());
        }

        tracing::info!(
            "Loaded {} mesh(es), first mesh has {} vertices, {} indices",
            cpu_meshes.len(),
            cpu_meshes[0].vertices.len(),
            cpu_meshes[0].indices.len()
        );

        let instance =
            Instance::from_pos_scale_color(glam::Vec3::ZERO, glam::Vec3::ONE, [1.0, 1.0, 1.0, 1.0]);
        self.renderer
            .add_composite_model(&name, cpu_meshes, &[instance]);
        tracing::info!("Model '{}' added to renderer", name);
        Ok(())
    }

    /// Feed World entities to the engine renderer as named models.
    ///
    /// Iterates all entities in the World, groups them by mesh path, and
    /// updates the engine's model list. Entities without a mesh use the
    /// engine's built-in cube primitive. Selected entities get an orange
    /// tint for highlighting.
    pub fn feed_entities(
        &mut self,
        world: &astraweave_core::World,
        entity_meshes: &std::collections::HashMap<astraweave_core::Entity, String>,
        selected_entities: &[astraweave_core::Entity],
    ) {
        // Skip rebuild when nothing changed (entity count, selection, mesh assignments)
        let entity_count = world.entities().len();
        if entity_count == self.cached_entity_feed_count
            && entity_meshes.len() == self.cached_entity_feed_mesh_count
            && selected_entities == self.cached_entity_feed_selected.as_slice()
        {
            return;
        }

        tracing::debug!(
            target: "aw_editor::viewport",
            "feed_entities: rebuilding — {} entities, {} mesh assignments, {} selected",
            entity_count,
            entity_meshes.len(),
            selected_entities.len(),
        );

        self.cached_entity_feed_count = entity_count;
        self.cached_entity_feed_mesh_count = entity_meshes.len();
        self.cached_entity_feed_selected = selected_entities.to_vec();

        use astraweave_render::Instance;
        use std::collections::HashMap;

        // Group instances by mesh path (None = default cube)
        let mut mesh_groups: HashMap<Option<String>, Vec<Instance>> = HashMap::new();

        for entity in world.entities() {
            if let Some(pose) = world.pose(entity) {
                let x = if pose.use_float_pos {
                    pose.float_x
                } else {
                    pose.pos.x as f32
                };
                let z = if pose.use_float_pos {
                    pose.float_z
                } else {
                    pose.pos.y as f32
                };
                let position = glam::Vec3::new(x, pose.height, z);
                let scale = glam::Vec3::new(pose.scale, pose.scale_y, pose.scale_z);
                let rotation = glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    pose.rotation_x,
                    pose.rotation,
                    pose.rotation_z,
                );
                let transform =
                    glam::Mat4::from_scale_rotation_translation(scale, rotation, position);

                let is_selected = selected_entities.contains(&entity);
                let color = if is_selected {
                    [1.0, 0.6, 0.2, 1.0] // Orange highlight
                } else if let Some(team) = world.team(entity) {
                    match team.id {
                        0 => [0.2, 0.8, 0.3, 1.0],
                        1 => [0.3, 0.6, 1.0, 1.0],
                        2 => [1.0, 0.3, 0.2, 1.0],
                        _ => [0.6, 0.6, 0.7, 1.0],
                    }
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                };

                let instance = Instance {
                    transform,
                    color,
                    material_id: 0,
                };

                let mesh_key = entity_meshes.get(&entity).cloned();
                mesh_groups.entry(mesh_key).or_default().push(instance);
            }
        }

        // Determine which entity model names are still active this frame
        let mut active_names: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(mesh_groups.len());

        // Add each group as a named model
        for (mesh_key, instances) in &mesh_groups {
            let model_name = match mesh_key {
                Some(path) => format!("entity_mesh_{}", path.replace(['/', '\\', '.'], "_")),
                None => "entity_default_cubes".to_string(),
            };
            active_names.insert(model_name.clone());

            // Fast path: model already exists → just update the instance buffer
            // (reuses the existing mesh GPU buffers, no disk I/O)
            if self.renderer.update_model_instances(&model_name, instances) {
                continue;
            }

            // Slow path: first time seeing this model → load mesh and create GPU resources
            let used_composite_model = match mesh_key {
                Some(path) => {
                    let opts = astraweave_render::mesh_gltf::GltfOptions::default();
                    match astraweave_render::mesh_gltf::load_gltf(Path::new(path), &opts) {
                        Ok(cpu_meshes) if !cpu_meshes.is_empty() => {
                            self.renderer
                                .add_composite_model(&model_name, cpu_meshes, instances);
                            true
                        }
                        _ => false,
                    }
                }
                None => false,
            };
            if !used_composite_model {
                let mesh = self.renderer.create_mesh_from_arrays(
                    &CUBE_POSITIONS,
                    &CUBE_NORMALS,
                    &CUBE_INDICES,
                );
                self.renderer.add_model(&model_name, mesh, instances);
            }
        }

        // Remove entity models that no longer have any instances
        let stale_names: Vec<String> = self
            .renderer
            .model_names_with_prefix("entity_")
            .into_iter()
            .filter(|n| !active_names.contains(n))
            .collect();
        for name in &stale_names {
            self.renderer.clear_model(name);
        }
    }

    /// Invalidate the feed_entities cache so the next call rebuilds all entity transforms.
    /// Call when entity transforms change (gizmo drag, undo, paste, etc.)
    pub fn invalidate_entity_cache(&mut self) {
        self.cached_entity_feed_count = usize::MAX;
    }

    pub fn has_model(&self, name: &str) -> bool {
        self.renderer.has_model(name)
    }

    pub fn clear_model(&mut self, name: &str) {
        self.renderer.clear_model(name);
    }

    /// Set material parameters for the current model
    pub fn set_material_params(&mut self, base_color: [f32; 4], metallic: f32, roughness: f32) {
        self.renderer
            .set_material_params(base_color, metallic, roughness);
        tracing::debug!(
            "Material params set: color={:?}, metallic={}, roughness={}",
            base_color,
            metallic,
            roughness
        );
    }

    /// Get model count
    pub fn model_count(&self) -> usize {
        self.renderer.model_count()
    }

    /// List all loaded model names
    pub fn model_names(&self) -> Vec<String> {
        self.renderer.model_names()
    }

    /// Get current time of day (0.0 - 24.0 game hours)
    pub fn get_time_of_day(&self) -> f32 {
        self.renderer.time_of_day().current_time
    }

    /// Set time of day (0.0 - 24.0 game hours)
    pub fn set_time_of_day(&mut self, hour: f32) {
        let time = self.renderer.time_of_day_mut();
        time.current_time = hour.clamp(0.0, 24.0);
        tracing::debug!("Time of day set to: {:.1}h", time.current_time);
    }

    /// Get time scale (1.0 = real time, 60.0 = 1 real minute = 1 game hour)
    pub fn get_time_scale(&self) -> f32 {
        self.renderer.time_of_day().time_scale
    }

    /// Set time scale
    pub fn set_time_scale(&mut self, scale: f32) {
        let time = self.renderer.time_of_day_mut();
        time.time_scale = scale.max(0.0);
        tracing::debug!("Time scale set to: {:.1}x", time.time_scale);
    }

    /// Check if it's currently daytime
    pub fn is_daytime(&self) -> bool {
        self.renderer.time_of_day().is_day()
    }

    /// Get current light direction
    pub fn get_light_direction(&self) -> glam::Vec3 {
        self.renderer.time_of_day().get_light_direction()
    }

    /// Get current light color
    pub fn get_light_color(&self) -> glam::Vec3 {
        self.renderer.time_of_day().get_light_color()
    }

    /// Get sun position
    pub fn get_sun_position(&self) -> glam::Vec3 {
        self.renderer.time_of_day().get_sun_position()
    }

    /// Get time-of-day period description
    pub fn get_time_period(&self) -> &'static str {
        let time = self.renderer.time_of_day();
        if time.is_night() {
            "Night"
        } else if time.is_twilight() {
            "Twilight"
        } else {
            "Day"
        }
    }

    /// Check if shadows are enabled
    pub fn shadows_enabled(&self) -> bool {
        self.renderer.shadows_enabled()
    }

    /// Enable or disable shadows
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.renderer.set_shadows_enabled(enabled);
        tracing::debug!("Shadows enabled: {}", enabled);
    }

    // ── Terrain chunk feeding ───────────────────────────────────────────

    /// Upload terrain chunks to the engine renderer as clustered models.
    ///
    /// The adapter preserves one converted record per source chunk, then groups
    /// those chunks into stable clustered models for culling. This keeps brush
    /// updates compatible with clustered rendering because the affected cluster
    /// can be rebuilt from its owned source chunks.
    pub fn upload_terrain_chunks(&mut self, chunks: &[(Vec<TerrainVertex>, Vec<u32>)]) {
        // Upload a procedural detail texture once so terrain is not rendered
        // with the default 1×1 white pixel (which makes it look flat/fuzzy).
        // If authored biome maps were already loaded for the current terrain,
        // preserve them instead of overwriting them with the fallback.
        if !self.terrain_detail_texture_uploaded {
            let (albedo_width, albedo_height, albedo) = Self::generate_terrain_detail_texture();
            let (normal_width, normal_height, normal) =
                Self::generate_default_terrain_normal_texture();
            let (mra_width, mra_height, mra) = Self::generate_default_terrain_mra_texture();
            self.set_terrain_surface_maps(
                (albedo_width, albedo_height, &albedo),
                Some((normal_width, normal_height, &normal)),
                Some((mra_width, mra_height, &mra)),
            );
        }

        // Clear previous terrain models
        for name in self.terrain_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }
        self.terrain_chunks.clear();
        self.terrain_chunk_slot_map.clear();
        self.terrain_clusters.clear();
        self.terrain_source_chunk_count = chunks.len();
        self.terrain_total_triangles = 0;
        self.terrain_total_indices = 0;

        // Terrain Material System campaign — Phase 1.E.4.b.
        // Lazy one-time init of the forward-lit splat path + upload of the
        // 8 placeholder biome material texture sets. Subsequent calls skip
        // this block because `renderer.terrain_forward()` is Some after
        // the first successful init. Phase 3 replaces placeholders with
        // real materials loaded from `assets/materials/{biome}/`.
        #[cfg(feature = "terrain-splat-arrays")]
        if self.renderer.terrain_forward().is_none() {
            use super::terrain_biome_placeholder as biome_ph;

            match self.renderer.init_terrain_forward() {
                Ok(()) => {
                    let albedos = biome_ph::generate_biome_placeholder_albedos();
                    let flat_normal = biome_ph::generate_flat_normal_map();
                    let neutral_orm = biome_ph::generate_neutral_orm_map();

                    let layers: Vec<astraweave_render::LayerTextures<'_>> = (0..8)
                        .map(|i| astraweave_render::LayerTextures {
                            albedo: Some(&albedos[i]),
                            normal: Some(&flat_normal),
                            orm: Some(&neutral_orm),
                            height: None,
                        })
                        .collect();

                    let mut gpu_material =
                        astraweave_render::TerrainMaterialGpu::default();
                    // Each biome is one layer (one albedo texture). Set the
                    // per-layer material factors so roughness/metallic come
                    // from the neutral ORM map unchanged.
                    gpu_material.active_layer_count = 8;
                    // Point each layer at its own array slice (0..7) for
                    // albedo/normal/orm. texture_indices = [a, n, o, h].
                    for (i, layer) in gpu_material.layers.iter_mut().enumerate() {
                        layer.texture_indices = [i as u32, i as u32, i as u32, i as u32];
                    }

                    if let Err(e) =
                        self.renderer.set_terrain_materials(&gpu_material, &layers)
                    {
                        tracing::warn!(
                            target: "aw_editor::viewport::terrain_forward",
                            "Phase 1.E.4 set_terrain_materials failed: {e:#}"
                        );
                    } else {
                        tracing::info!(
                            target: "aw_editor::viewport::terrain_forward",
                            "Phase 1 forward-lit terrain activated with 8 biome \
                             placeholder materials (1024² albedo, 512² normal/ORM)"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "aw_editor::viewport::terrain_forward",
                        "Phase 1.E.4 init_terrain_forward failed; legacy terrain path \
                         will remain active: {e:#}"
                    );
                }
            }
        }

        // Terrain Material System campaign — Phase 1.E.4.c.
        // Clear per-chunk forward state for the new terrain upload.
        #[cfg(feature = "terrain-splat-arrays")]
        if self.renderer.terrain_forward().is_some() {
            self.renderer.clear_terrain_chunks();
        }

        let total_verts: usize = chunks.iter().map(|(v, _)| v.len()).sum();
        let total_indices: usize = chunks.iter().map(|(_, i)| i.len()).sum();

        if total_verts == 0 || total_indices == 0 {
            self.renderer.reset_ground_plane();
            return;
        }

        for (vertices, indices) in chunks {
            if vertices.is_empty() || indices.is_empty() {
                self.terrain_chunk_slot_map.push(None);
                continue;
            }

            let chunk_index = self.terrain_chunks.len();
            self.terrain_chunks
                .push(Self::convert_terrain_chunk(vertices, indices));
            self.terrain_chunk_slot_map.push(Some(chunk_index));

            // Terrain Material System campaign — Phase 1.E.4.c.
            // Route the chunk through the forward-lit splat pipeline on the
            // engine side. Builds a `TerrainSplatVertex` buffer with
            // normalized per-chunk [0, 1] UVs (distinct from the legacy
            // world-scaled UVs at `engine_adapter.rs:1706-1710` — those
            // were a workaround for the single-texture legacy path), filters
            // the surface-triangle indices (drops skirt triangles appended
            // after the surface grid in `terrain_integration.rs:812+`), and
            // hands everything to `Renderer::upload_terrain_chunk` which
            // uploads the vertex/index buffers + splat textures and
            // registers the chunk with the forward pipeline. No-op when
            // `terrain_forward` isn't initialized (feature off or init
            // failed) — legacy cluster registration below picks up.
            #[cfg(feature = "terrain-splat-arrays")]
            if self.renderer.terrain_forward().is_some() {
                // Phase 1 post-completion fix: the original 1.E.4.c code used
                // `floor(sqrt(vertices.len()))` to infer the surface grid
                // dimension. For the editor's `N² + 4N` with-skirts layout,
                // that returns `N+2` and produced a `surface_idx_count` that
                // pulled skirt triangles into the filtered set, causing
                // long streaky degenerate triangles (skirt indices pointed
                // past the end of the truncated vertex buffer and read
                // out-of-bounds memory as vertex positions). Using the
                // closed-form `sqrt(total + 4) - 2` inverse of the skirts
                // formula gives the correct N.
                // Real-Fix.B per Round-6-Closure.A c7f3b50b3 §12 Option 1:
                // initial-upload path now routes through the shared
                // `upload_or_update_terrain_chunk_forward` helper (single
                // canonical implementation per CLAUDE.md v0.10.1 Edit 2).
                if let Err(e) =
                    self.upload_or_update_terrain_chunk_forward(chunk_index, vertices, indices)
                {
                    tracing::warn!(
                        target: "aw_editor::viewport::terrain_forward",
                        "Phase 1.E.4.c initial upload failed for chunk {chunk_index}: {e:#}"
                    );
                }
            }
        }

        if self.terrain_chunks.is_empty() {
            self.renderer.reset_ground_plane();
            return;
        }

        // Terrain Material System campaign — Phase 1.E.4.c.
        // When the forward-lit splat path is active AND has uploaded at
        // least one chunk, skip the legacy cluster-building + model
        // registration. The forward path renders terrain directly inside
        // `Renderer::draw_into` via its own pipeline; no cluster models
        // are needed. `terrain_clusters` and `terrain_model_names` stay
        // empty — downstream cluster rebuild paths (brush edits, etc.)
        // are inert until per-vertex material authoring reaches the
        // shader in Phase 2.
        //
        // When the feature is off, or `init_terrain_forward` failed, or
        // chunk upload failed for every chunk, the legacy path runs
        // unchanged — preserves the feature-off fallback and the
        // reversibility promise in plan §3.5.
        #[cfg(feature = "terrain-splat-arrays")]
        let forward_active = self
            .renderer
            .terrain_forward()
            .map(|tf| !tf.chunks.is_empty())
            .unwrap_or(false);
        #[cfg(not(feature = "terrain-splat-arrays"))]
        let forward_active = false;

        if !forward_active {
            let planning: Vec<_> = self
                .terrain_chunks
                .iter()
                .map(TerrainChunkPlanningInfo::from)
                .collect();
            self.terrain_clusters = build_terrain_cluster_plan(
                &planning,
                TERRAIN_CLUSTER_GRID,
                TERRAIN_MAX_VERTICES_PER_CLUSTER,
            )
            .into_iter()
            .enumerate()
            .map(|(cluster_index, chunk_indices)| TerrainClusterRecord {
                name: format!("terrain_cluster_{cluster_index}"),
                chunk_indices,
            })
            .collect();
            self.terrain_model_names = self
                .terrain_clusters
                .iter()
                .map(|cluster| cluster.name.clone())
                .collect();

            for cluster_index in 0..self.terrain_clusters.len() {
                self.rebuild_terrain_cluster(cluster_index);
            }
        }

        let mut global_aabb_min = [f32::MAX; 3];
        let mut global_aabb_max = [f32::MIN; 3];
        for chunk in &self.terrain_chunks {
            for axis in 0..3 {
                global_aabb_min[axis] = global_aabb_min[axis].min(chunk.aabb_min[axis]);
                global_aabb_max[axis] = global_aabb_max[axis].max(chunk.aabb_max[axis]);
            }
        }

        // Track stats for the scene stats panel
        self.terrain_total_indices = total_indices;
        self.terrain_total_triangles = total_indices / 3;

        // Rebuild the coarse height grid from the freshly-uploaded vertex
        // data. Used downstream by `upload_scatter_placements` to resample
        // per-instance Y against the live surface (Issue #6 grounding).
        self.terrain_height_grid = TerrainHeightGrid::build(&self.terrain_chunks);
        if let Some(grid) = &self.terrain_height_grid {
            tracing::debug!(
                target: "aw_editor::viewport",
                "Terrain height grid built: {}×{} cells, cell_size={:.2}m",
                grid.width, grid.height, grid.cell_size,
            );
        }

        tracing::info!(
            target: "aw_editor::viewport",
            "Terrain uploaded: {} chunks → {} GPU models, {} total tris, {} total verts",
            chunks.len(),
            self.terrain_model_names.len(),
            self.terrain_total_triangles,
            total_verts,
        );

        // Position ground fill plane below all terrain to block sky bleed-through.
        let global_min_y = global_aabb_min[1];
        let global_max_extent = global_aabb_max[0]
            .abs()
            .max(global_aabb_max[2].abs())
            .max(global_aabb_min[0].abs())
            .max(global_aabb_min[2].abs());

        if global_min_y < f32::MAX {
            let ground_y = global_min_y - 5.0;
            let extent = global_max_extent + 100.0;
            self.renderer.set_terrain_ground_plane(ground_y, extent);

            // ── Sky ────────────────────────────────────────────────────
            // Procedural sky config is the single source of truth for the
            // horizon colour. Fog is derived from it below to guarantee
            // they stay coherent — a mismatch creates a visible white
            // void at the terrain fade (editor audit Issue #5).
            let sky = astraweave_render::SkyConfig {
                day_color_top: glam::Vec3::new(0.25, 0.55, 1.0),
                day_color_horizon: glam::Vec3::new(0.75, 0.85, 1.0),
                sunset_color_top: glam::Vec3::new(0.8, 0.4, 0.2),
                sunset_color_horizon: glam::Vec3::new(1.0, 0.6, 0.3),
                night_color_top: glam::Vec3::new(0.0, 0.0, 0.1),
                night_color_horizon: glam::Vec3::new(0.1, 0.1, 0.2),
                cloud_coverage: 0.35,
                cloud_speed: 0.01,
                cloud_altitude: 800.0,
            };

            // ── Fog ────────────────────────────────────────────────────
            // Distance fog fades terrain edges smoothly into the sky.
            // The fog colour is taken DIRECTLY from `sky.day_color_horizon`
            // so the transition stays seamless even if the sky palette is
            // tweaked here in the future.
            let env = self.renderer.scene_environment_mut();
            env.visuals.fog_color = fog_color_from_sky(&sky);
            env.visuals.fog_start = 800.0; // crystal clear viewing to 800 units
            env.visuals.fog_end = 1800.0; // fully fogged at 1800
            env.visuals.fog_density = 0.0; // pure linear fog (no exponential)
                                           // Ambient fill so shadowed areas aren't pitch black
            env.visuals.ambient_color = glam::Vec3::new(0.45, 0.50, 0.55);
            env.visuals.ambient_intensity = 0.35;
            tracing::debug!(
                "Ground fill plane set at Y={ground_y:.1}, extent={extent:.0}, \
                 fog_start={:.0}, fog_density={:.5}",
                env.visuals.fog_start,
                env.visuals.fog_density,
            );

            // Activate the procedural sky renderer now that fog has been
            // aligned with its horizon colour.
            self.renderer.set_sky_config(sky);

            // ── Sun ────────────────────────────────────────────────────
            // A warm directional light at ~35° elevation for visible
            // terrain shadows and natural surface shading.
            let sun_dir = glam::Vec3::new(-0.5, -0.6, -0.4).normalize();
            self.renderer.set_light_direction_override(sun_dir, 1.5);

            // ── Shadow cascade tuning for terrain ──────────────────────
            // Wider cascade extents cover more terrain, and a higher
            // cascade lambda biases toward logarithmic splits for better
            // near-field shadow quality.
            self.renderer.set_cascade_extents(80.0, 250.0);
            self.renderer.set_cascade_lambda(0.7);
            self.renderer.set_shadow_filter(1.5, 0.0005, 1.0);

            // ── Quality preset ─────────────────────────────────────────
            // Auto-switch to EditorTerrain (shadows) when terrain
            // is loaded, unless the user has explicitly chosen GameQuality.
            if self.quality_preset != EditorQualityPreset::GameQuality {
                self.apply_quality_preset(EditorQualityPreset::EditorTerrain);
            }
        }

        tracing::debug!(
            "Uploaded {} terrain chunks ({} triangles) to engine renderer",
            self.terrain_source_chunk_count,
            self.terrain_total_triangles,
        );
    }

    fn rebuild_terrain_cluster(&mut self, cluster_index: usize) {
        let Some(cluster) = self.terrain_clusters.get(cluster_index).cloned() else {
            return;
        };

        let mut merged_positions = Vec::new();
        let mut merged_normals = Vec::new();
        let mut merged_tangents = Vec::new();
        let mut merged_uvs = Vec::new();
        let mut merged_indices = Vec::new();
        let mut cluster_aabb_min = [f32::MAX; 3];
        let mut cluster_aabb_max = [f32::MIN; 3];
        let mut surface_summary = TerrainSurfaceSummary::default();
        let mut vertex_offset = 0u32;

        for chunk_index in cluster.chunk_indices {
            let Some(chunk) = self.terrain_chunks.get(chunk_index) else {
                continue;
            };

            for &idx in &chunk.indices {
                merged_indices.push(idx + vertex_offset);
            }
            vertex_offset += chunk.positions.len() as u32;

            merged_positions.extend_from_slice(&chunk.positions);
            merged_normals.extend_from_slice(&chunk.normals);
            merged_tangents.extend_from_slice(&chunk.tangents);
            merged_uvs.extend_from_slice(&chunk.uvs);
            surface_summary.merge(&chunk.surface_summary);

            for axis in 0..3 {
                cluster_aabb_min[axis] = cluster_aabb_min[axis].min(chunk.aabb_min[axis]);
                cluster_aabb_max[axis] = cluster_aabb_max[axis].max(chunk.aabb_max[axis]);
            }
        }

        if merged_positions.is_empty() {
            self.renderer.clear_model(&cluster.name);
            return;
        }

        let prototype_name = surface_summary
            .dominant_material_index()
            .and_then(|material_index| self.ensure_terrain_material_prototype(material_index));
        let resolved_tint = surface_summary.resolve_tint();
        let instance = astraweave_render::Instance::from_pos_scale_color(
            glam::Vec3::ZERO,
            glam::Vec3::ONE,
            if prototype_name.is_some() {
                blend_tints([1.0, 1.0, 1.0, 1.0], resolved_tint, 0.20)
            } else {
                resolved_tint
            },
        );
        if let Some(prototype_name) = prototype_name.as_deref() {
            let mesh = self.renderer.create_mesh_from_full_arrays(
                &merged_positions,
                &merged_normals,
                &merged_tangents,
                &merged_uvs,
                &merged_indices,
            );
            if self.renderer.add_model_sharing_texture_with_bounds(
                &cluster.name,
                mesh,
                &[instance.clone()],
                prototype_name,
                cluster_aabb_min,
                cluster_aabb_max,
            ) {
                return;
            }
        }

        let mesh = self.renderer.create_mesh_from_full_arrays(
            &merged_positions,
            &merged_normals,
            &merged_tangents,
            &merged_uvs,
            &merged_indices,
        );
        self.renderer.add_model_with_bounds(
            &cluster.name,
            mesh,
            &[instance],
            cluster_aabb_min,
            cluster_aabb_max,
        );
    }

    fn rebuild_terrain_clusters_for_chunk(&mut self, chunk_index: usize) {
        let affected_clusters: Vec<_> = self
            .terrain_clusters
            .iter()
            .enumerate()
            .filter_map(|(cluster_index, cluster)| {
                cluster
                    .chunk_indices
                    .contains(&chunk_index)
                    .then_some(cluster_index)
            })
            .collect();

        for cluster_index in affected_clusters {
            self.rebuild_terrain_cluster(cluster_index);
        }
    }

    fn refresh_terrain_ground_plane(&mut self) {
        if self.terrain_chunks.is_empty() {
            self.renderer.reset_ground_plane();
            return;
        }

        let mut global_aabb_min = [f32::MAX; 3];
        let mut global_aabb_max = [f32::MIN; 3];
        for chunk in &self.terrain_chunks {
            for axis in 0..3 {
                global_aabb_min[axis] = global_aabb_min[axis].min(chunk.aabb_min[axis]);
                global_aabb_max[axis] = global_aabb_max[axis].max(chunk.aabb_max[axis]);
            }
        }

        let global_min_y = global_aabb_min[1];
        let global_max_extent = global_aabb_max[0]
            .abs()
            .max(global_aabb_max[2].abs())
            .max(global_aabb_min[0].abs())
            .max(global_aabb_min[2].abs());

        if global_min_y < f32::MAX {
            let ground_y = global_min_y - 5.0;
            let extent = global_max_extent + 100.0;
            self.renderer.set_terrain_ground_plane(ground_y, extent);
        }
    }

    /// Convert editor terrain vertices to clustered render data in a single pass.
    fn convert_terrain_chunk(
        vertices: &[TerrainVertex],
        indices: &[u32],
    ) -> TerrainChunkRenderData {
        let count = vertices.len();
        let mut positions = Vec::with_capacity(count);
        let mut normals = Vec::with_capacity(count);
        let mut tangents = Vec::with_capacity(count);
        let mut uvs = Vec::with_capacity(count);
        let mut surface_summary = TerrainSurfaceSummary::default();
        let mut aabb_min = [f32::MAX; 3];
        let mut aabb_max = [f32::MIN; 3];

        for v in vertices {
            positions.push(v.position);
            normals.push(v.normal);

            // World-space detail-UV tiling: the source `v.uv` maps 0..1 across
            // an entire terrain chunk (commonly 128 m+), which stretches the
            // global detail/biome texture into a near-uniform blur and is
            // the primary cause of the "flat green" look reported in the
            // visual audit (Issue #2). Overriding with position.xz * freq
            // gives a regular world-space tile regardless of chunk size.
            //
            // `DETAIL_UV_FREQ` tunes the visible pattern:
            //   * Too high (≥0.25, ≤4 m period): fine features read as a
            //     countable polka-dot grid at close range (audit clip 2, #6).
            //   * Too low (≤0.05, ≥20 m period): features become blurry
            //     blobs that don't add detail.
            //   * 0.125 (8 m period) is a pragmatic mid-point that reads
            //     as low-frequency variation at typical editor camera
            //     distances. The PROPER fix is the splat-array pipeline
            //     (Phase 2.2 of the editor fidelity plan) which substitutes
            //     authored per-material textures for this global tile.
            //
            // This is safe because:
            //   * Tangents are computed from the normal only (below), not UV.
            //   * The renderer samples a single global albedo/normal/MR via
            //     these UVs and does not use them as a chunk-relative index.
            //   * Per-vertex biome/material weights remain the sole source of
            //     tint variation via `TerrainSurfaceSummary::resolve_tint`.
            const DETAIL_UV_FREQ: f32 = 0.125; // 1 / 8 m
            uvs.push([
                v.position[0] * DETAIL_UV_FREQ,
                v.position[2] * DETAIL_UV_FREQ,
            ]);

            // Compute tangent from normal
            let n = glam::Vec3::from(v.normal);
            let up = if n.y.abs() > 0.99 {
                glam::Vec3::X
            } else {
                glam::Vec3::Y
            };
            let t = n.cross(up).normalize();
            tangents.push([t.x, t.y, t.z, 1.0]);

            surface_summary.add_vertex(v);

            // AABB
            for j in 0..3 {
                aabb_min[j] = aabb_min[j].min(v.position[j]);
                aabb_max[j] = aabb_max[j].max(v.position[j]);
            }
        }

        TerrainChunkRenderData {
            positions,
            normals,
            tangents,
            uvs,
            indices: indices.to_vec(),
            surface_summary,
            aabb_min,
            aabb_max,
        }
    }

    fn build_scatter_lod_assets(
        key: &str,
        cpu_meshes: &[astraweave_render::mesh::CpuMesh],
    ) -> Option<ScatterLodAssets> {
        let mut model_aabb_min = glam::Vec3::splat(f32::INFINITY);
        let mut model_aabb_max = glam::Vec3::splat(f32::NEG_INFINITY);
        for mesh in cpu_meshes {
            if let Some((min, max)) = mesh.aabb() {
                model_aabb_min = model_aabb_min.min(min);
                model_aabb_max = model_aabb_max.max(max);
            }
        }

        if !model_aabb_min.is_finite() || !model_aabb_max.is_finite() {
            return None;
        }

        let atlas_region = astraweave_render::vegetation_lod::AtlasRegion {
            u_min: 0.0,
            v_min: 0.0,
            u_max: 1.0,
            v_max: 1.0,
        };

        let aabb_min_y = model_aabb_min.y.min(0.0);
        let model_height = (model_aabb_max.y - model_aabb_min.y).max(0.1);
        let model_half_width =
            ((model_aabb_max.x - model_aabb_min.x).max(model_aabb_max.z - model_aabb_min.z) / 2.0)
                .max(0.05);

        let mut primitives = Vec::new();
        for mesh in cpu_meshes.iter().filter(|mesh| !mesh.vertices.is_empty()) {
            let simplification_target = match mesh.vertices.len() {
                0..=2_048 => 0.55,
                2_049..=8_192 => 0.40,
                8_193..=20_480 => 0.30,
                _ => 0.20,
            };
            let lod_chain = astraweave_render::vegetation_lod::VegetationLodChain::build(
                key,
                astraweave_render::lod_generator::SimplificationMesh::from_cpu_mesh(mesh),
                simplification_target,
                0.5,
                1.0,
                atlas_region,
            );
            let simplified_mesh = lod_chain
                .lod1_mesh
                .to_cpu_mesh(mesh.albedo_image.clone(), mesh.texture_source_hint.clone());
            primitives.push(ScatterPrimitiveLodAssets {
                full_mesh: mesh.clone(),
                simplified_mesh,
            });
        }

        if primitives.is_empty() {
            return None;
        }

        Some(ScatterLodAssets {
            primitives,
            cross_billboard: astraweave_render::vegetation_lod::generate_cross_billboard(0.5, 1.0)
                .to_cpu_mesh(),
            impostor_card: astraweave_render::vegetation_lod::generate_impostor_card(0.5, 1.0)
                .to_cpu_mesh(),
            aabb_min_y,
            model_height,
            model_half_width,
        })
    }

    fn scatter_density_for_distance(
        distances: &astraweave_render::vegetation_lod::TreeLodDistances,
        distance: f32,
    ) -> f32 {
        if distance <= distances.lod1_max {
            1.0
        } else if distance <= distances.lod2_max {
            0.5
        } else if distance <= distances.cull_distance * 0.85 {
            0.25
        } else {
            0.125
        }
    }

    fn scatter_alpha_sidecar_path(diffuse_path: &std::path::Path) -> Option<std::path::PathBuf> {
        let file_name = diffuse_path.file_name()?.to_str()?;
        let mut candidates = Vec::new();

        for needle in ["_diff", "_albedo", "_basecolor", "_color"] {
            if file_name.contains(needle) {
                candidates.push(file_name.replace(needle, "_alpha"));
                candidates.push(file_name.replace(needle, "_mask"));
            }
        }

        let parent = diffuse_path.parent()?;
        for candidate in candidates {
            let candidate_path = parent.join(candidate);
            if candidate_path.exists() {
                return Some(candidate_path);
            }
        }

        None
    }

    fn load_scatter_texture_from_path(
        path: &std::path::Path,
        max_texture_size: u32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let mut rgba = image::open(path).ok()?.to_rgba8();

        if let Some(alpha_path) = Self::scatter_alpha_sidecar_path(path) {
            if let Ok(alpha_image) = image::open(&alpha_path) {
                let mut alpha = alpha_image.to_luma8();
                if alpha.dimensions() != rgba.dimensions() {
                    alpha = image::imageops::resize(
                        &alpha,
                        rgba.width(),
                        rgba.height(),
                        image::imageops::FilterType::Triangle,
                    );
                }

                for (pixel, alpha_pixel) in rgba.pixels_mut().zip(alpha.pixels()) {
                    pixel[3] = alpha_pixel[0];
                }
            }
        }

        let (width, height) = rgba.dimensions();
        if width > max_texture_size || height > max_texture_size {
            let resized = image::imageops::resize(
                &rgba,
                width.min(max_texture_size),
                height.min(max_texture_size),
                image::imageops::FilterType::Triangle,
            );
            let (resized_width, resized_height) = resized.dimensions();
            return Some((resized_width, resized_height, resized.into_raw()));
        }

        Some((width, height, rgba.into_raw()))
    }

    pub fn set_terrain_surface_maps(
        &mut self,
        albedo: (u32, u32, &[u8]),
        normal: Option<(u32, u32, &[u8])>,
        metallic_roughness: Option<(u32, u32, &[u8])>,
    ) {
        self.renderer
            .set_albedo_from_rgba8(albedo.0, albedo.1, albedo.2);
        if let Some((width, height, data)) = normal {
            self.renderer.set_normal_from_rgba8(width, height, data);
        }
        if let Some((width, height, data)) = metallic_roughness {
            self.renderer
                .set_metallic_roughness_from_rgba8(width, height, data);
        }
        self.terrain_detail_texture_uploaded = true;
    }

    fn generate_default_terrain_normal_texture() -> (u32, u32, Vec<u8>) {
        const SIZE: u32 = 4;
        let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
        for _ in 0..(SIZE * SIZE) {
            data.extend_from_slice(&[128, 128, 255, 255]);
        }
        (SIZE, SIZE, data)
    }

    fn generate_default_terrain_mra_texture() -> (u32, u32, Vec<u8>) {
        const SIZE: u32 = 4;
        let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
        for _ in 0..(SIZE * SIZE) {
            data.extend_from_slice(&[0, 220, 255, 255]);
        }
        (SIZE, SIZE, data)
    }

    fn load_terrain_surface_texture(path: &std::path::Path) -> Option<(u32, u32, Vec<u8>)> {
        let rgba = image::open(path).ok()?.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some((width, height, rgba.into_raw()))
    }

    fn load_terrain_surface_or_fallback(
        primary_path: &std::path::Path,
        fallback_path: &std::path::Path,
    ) -> Option<(u32, u32, Vec<u8>)> {
        Self::load_terrain_surface_texture(primary_path)
            .or_else(|| Self::load_terrain_surface_texture(fallback_path))
    }

    fn ensure_terrain_material_surface_set(
        &mut self,
        material_index: usize,
    ) -> Option<&TerrainMaterialSurfaceSet> {
        if self.terrain_material_surfaces.contains_key(&material_index) {
            return self.terrain_material_surfaces.get(&material_index);
        }

        let material_name = MATERIAL_NAMES.get(material_index)?;
        let root = find_assets_dir().join("materials");
        let fallback_base = "default";

        let albedo = Self::load_terrain_surface_or_fallback(
            &root.join(format!("{material_name}.png")),
            &root.join(format!("{fallback_base}.png")),
        )?;
        let normal = Self::load_terrain_surface_or_fallback(
            &root.join(format!("{material_name}_n.png")),
            &root.join(format!("{fallback_base}_n.png")),
        )?;
        let metallic_roughness = Self::load_terrain_surface_or_fallback(
            &root.join(format!("{material_name}_mra.png")),
            &root.join(format!("{fallback_base}_mra.png")),
        )?;

        self.terrain_material_surfaces.insert(
            material_index,
            TerrainMaterialSurfaceSet {
                albedo,
                normal,
                metallic_roughness,
            },
        );

        self.terrain_material_surfaces.get(&material_index)
    }

    fn ensure_terrain_material_prototype(&mut self, material_index: usize) -> Option<String> {
        if let Some(name) = self.terrain_material_prototypes.get(&material_index) {
            return Some(name.clone());
        }

        let surface_set = self
            .ensure_terrain_material_surface_set(material_index)
            .cloned()?;
        let material_name = MATERIAL_NAMES.get(material_index)?;
        let prototype_name = format!("__aw_editor_terrain_surface_{material_name}");

        let prototype_mesh = self.renderer.create_mesh_from_full_arrays(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            &[[0.0, 1.0, 0.0]; 3],
            &[[1.0, 0.0, 0.0, 1.0]; 3],
            &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            &[0, 1, 2],
        );
        let surface_maps = astraweave_render::ModelSurfaceMaps {
            albedo: (
                surface_set.albedo.0,
                surface_set.albedo.1,
                &surface_set.albedo.2,
            ),
            normal: (
                surface_set.normal.0,
                surface_set.normal.1,
                &surface_set.normal.2,
            ),
            metallic_roughness: (
                surface_set.metallic_roughness.0,
                surface_set.metallic_roughness.1,
                &surface_set.metallic_roughness.2,
            ),
        };
        self.renderer.add_model_with_pbr_textures(
            &prototype_name,
            prototype_mesh,
            &[],
            &surface_maps,
        );
        self.terrain_material_prototypes
            .insert(material_index, prototype_name.clone());
        Some(prototype_name)
    }

    /// Generate a seamlessly tiling procedural terrain detail texture.
    ///
    /// The returned RGBA8 image is uploaded as the renderer's global albedo
    /// so terrain chunks (which have no per-model texture) render with
    /// visible surface detail instead of a flat solid color.
    ///
    /// The texture is neutral-bright (≈0.85-1.0 per channel) so that the
    /// biome tint instance color provides the actual hue while the noise
    /// provides organic variation and breaks up the flatness.
    fn generate_terrain_detail_texture() -> (u32, u32, Vec<u8>) {
        const SIZE: u32 = 512;
        // Number of noise grid cells per texture tile.  Higher values
        // create finer organic patterns that break up repetition.
        const GRID: i32 = 6;

        /// 2D gradient hash — returns a pseudo-random unit-length gradient.
        #[inline]
        fn grad2(mut ix: i32, mut iy: i32) -> (f32, f32) {
            ix = ix.wrapping_mul(1_597_334_677);
            iy = iy.wrapping_mul(2_654_435_761_u32 as i32);
            let h = (ix ^ iy).wrapping_mul(668_265_263) as u32;
            // 8 gradient directions (45° increments)
            let idx = h >> 29; // top 3 bits → 0..7
            const DIRS: [(f32, f32); 8] = [
                (1.0, 0.0),
                (0.707, 0.707),
                (0.0, 1.0),
                (-0.707, 0.707),
                (-1.0, 0.0),
                (-0.707, -0.707),
                (0.0, -1.0),
                (0.707, -0.707),
            ];
            DIRS[idx as usize]
        }

        /// Perlin-style gradient noise, seamlessly tiled at `period`.
        fn gradient_noise(x: f32, y: f32, period: i32) -> f32 {
            let ix = x.floor() as i32;
            let iy = y.floor() as i32;
            let fx = x - x.floor();
            let fy = y - y.floor();
            // Quintic smoothstep for C² continuity (no visible grid artifacts)
            let u = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
            let v = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);
            let wrap = |n: i32| ((n % period) + period) % period;
            let dot_grid = |cx: i32, cy: i32, dx: f32, dy: f32| -> f32 {
                let (gx, gy) = grad2(wrap(cx), wrap(cy));
                gx * dx + gy * dy
            };
            let n00 = dot_grid(ix, iy, fx, fy);
            let n10 = dot_grid(ix + 1, iy, fx - 1.0, fy);
            let n01 = dot_grid(ix, iy + 1, fx, fy - 1.0);
            let n11 = dot_grid(ix + 1, iy + 1, fx - 1.0, fy - 1.0);
            let nx0 = n00 + (n10 - n00) * u;
            let nx1 = n01 + (n11 - n01) * u;
            nx0 + (nx1 - nx0) * v
        }

        /// Fractal Brownian motion using gradient noise (tileable).
        /// Produces organic, non-repeating patterns.
        fn fbm(x: f32, y: f32, octaves: u32, base_grid: i32) -> f32 {
            let mut value = 0.0f32;
            let mut amp = 0.5f32;
            let mut freq = 1.0f32;
            let mut grid = base_grid;
            for _ in 0..octaves {
                value += gradient_noise(x * freq, y * freq, grid) * amp;
                amp *= 0.5;
                freq *= 2.0;
                grid *= 2; // period doubles with frequency to stay tileable
            }
            value
        }

        let mut data = vec![0u8; (SIZE * SIZE * 4) as usize];
        for py in 0..SIZE {
            for px in 0..SIZE {
                // Map pixel coordinates to noise space [0, GRID)
                let nx = px as f32 / SIZE as f32 * GRID as f32;
                let ny = py as f32 / SIZE as f32 * GRID as f32;

                // Primary organic detail (large-scale)
                let n1 = fbm(nx, ny, 6, GRID);
                // Secondary detail at a rotated+offset frequency to break
                // any residual tiling alignment
                let rot_x = nx * 0.866 + ny * 0.5 + 3.7;
                let rot_y = -nx * 0.5 + ny * 0.866 + 7.1;
                let n2 = fbm(rot_x, rot_y, 4, GRID);

                // Blend: mostly primary with subtle secondary variation
                let n = n1 * 0.7 + n2 * 0.3;

                // Neutral-bright with moderate variation.
                // High values keep the biome tint dominant.
                let base = 0.80 + n * 0.20; // range ≈ [0.70, 1.0]
                let luminance = base.clamp(0.0, 1.0);

                // Warm/cool color variation to produce natural earthy shifts
                let r = (luminance * 1.03).clamp(0.0, 1.0);
                let g = (luminance * 1.00).clamp(0.0, 1.0);
                let b = (luminance * 0.92).clamp(0.0, 1.0);

                let idx = ((py * SIZE + px) * 4) as usize;
                data[idx] = (r * 255.0) as u8;
                data[idx + 1] = (g * 255.0) as u8;
                data[idx + 2] = (b * 255.0) as u8;
                data[idx + 3] = 255;
            }
        }
        (SIZE, SIZE, data)
    }

    /// Clear all terrain data from the engine renderer.
    pub fn clear_terrain(&mut self) {
        for name in self.terrain_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }
        for prototype_name in self.terrain_material_prototypes.values() {
            self.renderer.clear_model(prototype_name);
        }
        self.terrain_chunks.clear();
        self.terrain_chunk_slot_map.clear();
        self.terrain_clusters.clear();
        self.terrain_material_surfaces.clear();
        self.terrain_material_prototypes.clear();
        self.terrain_source_chunk_count = 0;
        self.terrain_total_triangles = 0;
        self.terrain_total_indices = 0;
        // Force re-upload of the terrain detail texture on next terrain load
        self.terrain_detail_texture_uploaded = false;
        // Invalidate the height grid; it will be rebuilt on next terrain upload.
        self.terrain_height_grid = None;
        // Restore default ground plane position
        self.renderer.reset_ground_plane();
    }

    /// Get the number of terrain chunks currently loaded in the engine.
    pub fn terrain_chunk_count(&self) -> usize {
        self.terrain_source_chunk_count
    }

    /// Total terrain triangles across all uploaded chunks.
    pub fn terrain_triangles(&self) -> usize {
        self.terrain_total_triangles
    }

    /// Total terrain indices across all uploaded chunks.
    pub fn terrain_indices(&self) -> usize {
        self.terrain_total_indices
    }

    /// Build splat-vertex array, filter surface triangles, build splat maps,
    /// and call `Renderer::upload_terrain_chunk` for the live `terrain_forward`
    /// path. Used by both initial upload (`upload_terrain_chunks`) and
    /// incremental update (`update_terrain_chunk`) to ensure both paths route
    /// through the same canonical terrain rendering abstraction
    /// (`Renderer::terrain_forward.chunks`).
    ///
    /// Real-Fix.B per Round-6-Closure.A `c7f3b50b3` §12 (Option 1): replaces
    /// dual-variant §7.7 trap (initial-upload routes to live; incremental-
    /// update routes to legacy dead path) with single canonical implementation
    /// per CLAUDE.md v0.10.1 Edit 2 (no-second-implementation).
    ///
    /// Errors are returned for caller-side `tracing::warn!` logging in the
    /// existing style; `?`-propagated within the helper for grid_dim inference
    /// failure, splat-map build failure, and upload failure.
    #[cfg(feature = "terrain-splat-arrays")]
    fn upload_or_update_terrain_chunk_forward(
        &mut self,
        chunk_index: usize,
        vertices: &[TerrainVertex],
        indices: &[u32],
    ) -> anyhow::Result<()> {
        // Phase 1.E.4.c surface grid inference: closed-form `sqrt(total + 4) - 2`
        // inverse of the editor's `N² + 4N` with-skirts vertex layout.
        let grid_dim = infer_surface_grid_dim(vertices.len())
            .filter(|n| *n >= 2)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "could not infer surface grid dim from vertex count {}",
                    vertices.len()
                )
            })?;
        let grid_dim_u32 = grid_dim as u32;
        let grid_verts = grid_dim * grid_dim;
        let surface_verts = &vertices[..grid_verts];

        let splat_maps = super::terrain_splat_builder::build_chunk_splat_maps(
            surface_verts,
            grid_dim_u32,
            grid_dim_u32,
        )?;

        // Build per-vertex [0, 1] UVs from grid position (row-major).
        let grid_span = (grid_dim - 1) as f32;
        let splat_vertices: Vec<astraweave_render::TerrainSplatVertex> = (0..grid_verts)
            .map(|i| {
                let v = &surface_verts[i];
                let gx = (i % grid_dim) as f32;
                let gy = (i / grid_dim) as f32;
                astraweave_render::TerrainSplatVertex {
                    position: v.position,
                    normal: v.normal,
                    uv: [gx / grid_span, gy / grid_span],
                }
            })
            .collect();

        // Triangle-by-triangle filter: drop skirt triangles (indices >= grid_verts).
        let surface_vert_count = grid_verts as u32;
        let surface_indices = filter_surface_triangles(indices, surface_vert_count);

        let chunk_key = chunk_index as u64;
        self.renderer
            .upload_terrain_chunk(
                chunk_key,
                &splat_vertices,
                &surface_indices,
                &splat_maps.splat_0,
                &splat_maps.splat_1,
                (splat_maps.width, splat_maps.height),
            )
            .map_err(|e| anyhow::anyhow!("upload_terrain_chunk failed for chunk {chunk_key}: {e:#}"))?;

        Ok(())
    }

    /// Incrementally update a single source terrain chunk on the GPU.
    ///
    /// The engine viewport clusters multiple source chunks into fewer render
    /// models, so an update rebuilds the owning clustered model(s) instead of
    /// attempting to replace a no-longer-existent one-chunk GPU model.
    pub fn update_terrain_chunk(&mut self, chunk_index: usize, vertices: &[TerrainVertex]) {
        if vertices.is_empty() {
            return;
        }

        let Some(Some(stored_chunk_index)) = self.terrain_chunk_slot_map.get(chunk_index).copied()
        else {
            tracing::warn!(
                "update_terrain_chunk: unknown logical source chunk index {chunk_index} for clustered terrain"
            );
            return;
        };

        let indices = match self.terrain_chunks.get(stored_chunk_index) {
            Some(chunk) => chunk.indices.clone(),
            None => {
                tracing::warn!(
                    "update_terrain_chunk: missing stored chunk {stored_chunk_index} for logical source chunk {chunk_index}"
                );
                return;
            }
        };

        self.terrain_chunks[stored_chunk_index] = Self::convert_terrain_chunk(vertices, &indices);

        // Real-Fix.B per Round-6-Closure.A c7f3b50b3 §12 Option 1: route
        // incremental update through the live terrain_forward path via the
        // shared helper. Resolves Mechanism C (mesh resource identity trap;
        // sibling §7.7 instance at mesh-data layer) — pre-fix, this function
        // routed only to the legacy dead cluster path (T9.D evidence:
        // self.models["terrain_cluster_*"] never read at render time);
        // post-fix, helper writes to Renderer::terrain_forward.chunks
        // HashMap which IS read at render time, so brush modifications
        // become visible.
        #[cfg(feature = "terrain-splat-arrays")]
        if let Err(e) =
            self.upload_or_update_terrain_chunk_forward(chunk_index, vertices, &indices)
        {
            tracing::warn!(
                target: "aw_editor::viewport::terrain_forward",
                "Real-Fix.B incremental update failed for chunk {chunk_index}: {e:#}"
            );
        }

        // Legacy cluster path PRESERVED during Real-Fix.B per audit §5.1
        // discipline. Round 6 T9.D evidence proves this path is dead code at
        // current configuration (terrain_cluster_models=0 across 238 samples)
        // — the call below is harmless but redundant. Cleanup-A session
        // (post-Andrew-gate-PASS) deletes legacy path for clean diff
        // narrative.
        self.rebuild_terrain_clusters_for_chunk(stored_chunk_index);
        self.refresh_terrain_ground_plane();
    }

    // ── Scatter / vegetation feeding ────────────────────────────────────

    /// Upload scatter placements as instanced models in the engine renderer.
    ///
    /// Implements LOD selection, per-species cull distance, distance-based
    /// density scaling, and billboard rendering for distant vegetation.
    /// Returns (loaded, total_groups, not_found, load_failed).
    pub fn upload_scatter_placements(
        &mut self,
        placements: &[ScatterPlacement],
        diffuse_textures: &std::collections::HashMap<String, std::path::PathBuf>,
    ) -> (u32, u32, u32, u32) {
        let upload_start = std::time::Instant::now();

        // Retain ALL placements + textures for camera-driven streaming.
        self.scatter_placements = placements.to_vec();
        self.scatter_diffuse_textures = diffuse_textures.clone();
        self.scatter_lod_camera_pos = self.camera_position;
        self.scatter_lod_camera_yaw = self.camera_yaw;

        // Re-sample Y against the current terrain height grid so scatter
        // stays planted on the surface even if the heightmap was edited
        // after the scatter instances were generated. Placements whose XZ
        // falls outside the grid (e.g. far edges, no terrain) keep their
        // original Y. See `TerrainHeightGrid` for sampling strategy.
        if let Some(grid) = &self.terrain_height_grid {
            let mut resampled = 0usize;
            for p in &mut self.scatter_placements {
                if let Some(y) = grid.sample(p.position.x, p.position.z) {
                    if (p.position.y - y).abs() > 0.001 {
                        resampled += 1;
                    }
                    p.position.y = y;
                }
            }
            if resampled > 0 {
                tracing::debug!(
                    target: "aw_editor::viewport",
                    "Scatter Y resampled against terrain grid: {}/{} placements adjusted",
                    resampled,
                    self.scatter_placements.len(),
                );
            }
        }

        if placements.is_empty() {
            for name in self.scatter_model_names.drain(..) {
                self.renderer.clear_model(&name);
            }
            self.scatter_chunk_models.clear();
            self.active_scatter_chunks.clear();
            self.scatter_placement_count = 0;
            self.scatter_draw_call_count = 0;
            self.scatter_total_triangles = 0;
            self.scatter_total_vertices = 0;
            return (0, 0, 0, 0);
        }

        let cam_pos = self.camera_position;
        let cam_yaw = self.camera_yaw;
        let cam_chunk =
            astraweave_terrain::ChunkId::from_world_pos(cam_pos, self.scatter_chunk_size);
        self.camera_chunk = cam_chunk;

        // ── Chunk-radius filter ──────────────────────────────────────
        const SCATTER_LOAD_RADIUS: i32 = 3;
        let has_chunk_tags = placements
            .iter()
            .any(|p| p.chunk_id != astraweave_terrain::ChunkId::new(0, 0));
        let active_placements: Vec<&ScatterPlacement> = if has_chunk_tags {
            placements
                .iter()
                .filter(|p| {
                    let dx = (p.chunk_id.x - cam_chunk.x).abs();
                    let dz = (p.chunk_id.z - cam_chunk.z).abs();
                    dx <= SCATTER_LOAD_RADIUS && dz <= SCATTER_LOAD_RADIUS
                })
                .collect()
        } else {
            placements.iter().collect()
        };

        if active_placements.is_empty() {
            for name in self.scatter_model_names.drain(..) {
                self.renderer.clear_model(&name);
            }
            self.scatter_chunk_models.clear();
            self.active_scatter_chunks.clear();
            self.scatter_placement_count = 0;
            self.scatter_draw_call_count = 0;
            self.scatter_total_triangles = 0;
            self.scatter_total_vertices = 0;
            tracing::info!("Scatter: 0 placements within chunk load radius {SCATTER_LOAD_RADIUS} of camera chunk ({}, {})",
                cam_chunk.x, cam_chunk.z);
            return (0, 0, 0, 0);
        }

        tracing::debug!(
            "Scatter streaming: {} / {} placements in radius {} of chunk ({}, {})",
            active_placements.len(),
            placements.len(),
            SCATTER_LOAD_RADIUS,
            cam_chunk.x,
            cam_chunk.z,
        );

        // Group by mesh_key (only active placements)
        let mut groups: std::collections::HashMap<String, Vec<&ScatterPlacement>> =
            std::collections::HashMap::new();
        for p in &active_placements {
            groups.entry(p.mesh_key.clone()).or_default().push(p);
        }

        let active_chunk_ids: std::collections::HashSet<astraweave_terrain::ChunkId> =
            active_placements.iter().map(|p| p.chunk_id).collect();
        self.active_scatter_chunks = active_chunk_ids.clone();
        self.scatter_chunk_models = active_chunk_ids
            .iter()
            .copied()
            .map(|chunk_id| (chunk_id, Vec::new()))
            .collect();

        let mut loaded_groups = 0u32;
        let mut skipped_not_found = 0u32;
        let mut skipped_load_fail = 0u32;
        let mut actual_draw_calls = 0u32;
        let mut actual_instance_count = 0usize;
        let mut actual_triangle_count = 0usize;
        let mut actual_vertex_count = 0usize;
        let mut used_model_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        // Stage 3c.3-b: live set of impostor-pass keys touched by this
        // refresh. After the group loop we call
        // `retire_stale_impostor_passes(&impostor_live_keys)` to drop passes
        // whose meshes fell out of the active chunk set.
        #[cfg(feature = "impostor-bake")]
        let mut impostor_live_keys: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for (key, items) in &groups {
            let mesh_path = &items[0].mesh_path;
            let mesh_path_obj = std::path::Path::new(mesh_path);
            if !mesh_path_obj.exists() {
                if skipped_not_found < 3 {
                    tracing::warn!(
                        target: "aw_editor::viewport",
                        "Scatter: skipping '{}' ({} instances) — mesh not found: {}",
                        key, items.len(), mesh_path
                    );
                }
                skipped_not_found += 1;
                continue;
            }

            // Load glTF mesh — persistent cache across biome regenerations.
            let group_start = std::time::Instant::now();
            let path = std::path::Path::new(mesh_path);
            let load_result =
                if let Some(cached) = self.scatter_cpu_mesh_cache.get(mesh_path).cloned() {
                    tracing::debug!("Scatter: mesh cache hit for '{key}'");
                    Ok(Ok(cached))
                } else {
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let opts = astraweave_render::mesh_gltf::GltfOptions::default();
                        astraweave_render::mesh_gltf::load_gltf(path, &opts)
                    }));
                    if let Ok(Ok(ref meshes)) = res {
                        if !meshes.is_empty() {
                            self.scatter_cpu_mesh_cache
                                .insert(mesh_path.to_string(), meshes.clone());
                        }
                    }
                    res
                };
            match load_result {
                Ok(Ok(cpu_meshes)) if !cpu_meshes.is_empty() => {
                    const MAX_SCATTER_VERTICES: usize = 80_000;
                    let total_verts: usize = cpu_meshes.iter().map(|m| m.vertices.len()).sum();
                    if total_verts > MAX_SCATTER_VERTICES {
                        tracing::warn!(
                            target: "aw_editor::viewport",
                            "Scatter: skipping '{}' ({} instances) — {} vertices exceeds budget of {} (file: {})",
                            key, items.len(), total_verts, MAX_SCATTER_VERTICES, mesh_path
                        );
                        skipped_load_fail += 1;
                        continue;
                    }

                    let lod_assets = if let Some(cached) =
                        self.scatter_lod_asset_cache.get(mesh_path).cloned()
                    {
                        cached
                    } else if let Some(built) = Self::build_scatter_lod_assets(key, &cpu_meshes) {
                        self.scatter_lod_asset_cache
                            .insert(mesh_path.to_string(), built.clone());
                        built
                    } else {
                        tracing::warn!(
                            "Scatter: skipping '{key}' — unable to derive LOD assets from {mesh_path}"
                        );
                        skipped_load_fail += 1;
                        continue;
                    };

                    let species_cull =
                        (items[0].cull_distance > 0.0).then_some(items[0].cull_distance);
                    let representative_scale = items
                        .iter()
                        .map(|placement| placement.scale)
                        .fold(0.0_f32, f32::max)
                        .max(1.0);
                    let lod_distances = astraweave_render::vegetation_lod::adaptive_lod_distances(
                        lod_assets.model_height * representative_scale,
                        lod_assets.model_half_width * 2.0 * representative_scale,
                        species_cull,
                    );

                    struct LodInstance {
                        position: glam::Vec3,
                        instance: astraweave_render::Instance,
                    }
                    let mut lod0_instances: Vec<LodInstance> = Vec::new();
                    let mut lod1_instances: Vec<LodInstance> = Vec::new();
                    let mut lod2_instances: Vec<LodInstance> = Vec::new();
                    let mut lod3_instances: Vec<LodInstance> = Vec::new();
                    // Stage 3c.3-b: raw instance buffer for the new ImpostorPass
                    // path. Populated in lockstep with `lod3_instances` inside
                    // the `ImpostorCard` match arm so both paths can coexist
                    // until stage 3d removes the legacy fallback.
                    #[cfg(feature = "impostor-bake")]
                    let mut lod3_raw_instances: Vec<
                        astraweave_render::impostor_lod3::Lod3InstanceRaw,
                    > = Vec::new();

                    for p in items.iter() {
                        let dist = p.position.distance(cam_pos);

                        let lod =
                            astraweave_render::vegetation_lod::select_lod(dist, &lod_distances);
                        let lod = match lod {
                            Some(l) => l,
                            None => continue, // culled
                        };

                        let density = Self::scatter_density_for_distance(&lod_distances, dist);
                        if density < 1.0 {
                            let hash_input = (p.position.x * 73856.093) as u32
                                ^ (p.position.y * 42191.271) as u32
                                ^ (p.position.z * 19349.663) as u32;
                            let h = astraweave_render::vegetation_gpu::pcg_hash(hash_input);
                            let r = astraweave_render::vegetation_gpu::hash_to_float(h);
                            if r > density {
                                continue;
                            }
                        }

                        let normal_quat = {
                            let n = p.terrain_normal;
                            if n.y < 0.996 && n.length_squared() > 0.5 {
                                glam::Quat::from_rotation_arc(glam::Vec3::Y, n)
                            } else {
                                glam::Quat::IDENTITY
                            }
                        };
                        let yaw_quat = glam::Quat::from_rotation_y(p.rotation);
                        let rotation = normal_quat * yaw_quat;

                        let pivot_offset = lod_assets.aabb_min_y * p.scale;
                        let mut pos = p.position;
                        pos.y -= pivot_offset;

                        match lod {
                            astraweave_render::vegetation_lod::VegetationLod::FullMesh => {
                                let transform = glam::Mat4::from_scale_rotation_translation(
                                    glam::Vec3::splat(p.scale),
                                    rotation,
                                    pos,
                                );
                                lod0_instances.push(LodInstance {
                                    position: p.position,
                                    instance: astraweave_render::Instance {
                                        transform,
                                        color: [p.tint[0], p.tint[1], p.tint[2], 1.0],
                                        material_id: 0,
                                    },
                                });
                            }
                            astraweave_render::vegetation_lod::VegetationLod::Simplified => {
                                let transform = glam::Mat4::from_scale_rotation_translation(
                                    glam::Vec3::splat(p.scale),
                                    rotation,
                                    pos,
                                );
                                lod1_instances.push(LodInstance {
                                    position: p.position,
                                    instance: astraweave_render::Instance {
                                        transform,
                                        color: [p.tint[0], p.tint[1], p.tint[2], 1.0],
                                        material_id: 0,
                                    },
                                });
                            }
                            astraweave_render::vegetation_lod::VegetationLod::CrossBillboard => {
                                let billboard_transform =
                                    glam::Mat4::from_scale_rotation_translation(
                                        glam::Vec3::new(
                                            lod_assets.model_half_width * 2.0 * p.scale,
                                            lod_assets.model_height * p.scale,
                                            lod_assets.model_half_width * 2.0 * p.scale,
                                        ),
                                        yaw_quat,
                                        pos,
                                    );
                                lod2_instances.push(LodInstance {
                                    position: p.position,
                                    instance: astraweave_render::Instance {
                                        transform: billboard_transform,
                                        color: [p.tint[0], p.tint[1], p.tint[2], 1.0],
                                        material_id: 0,
                                    },
                                });
                            }
                            astraweave_render::vegetation_lod::VegetationLod::ImpostorCard => {
                                let to_camera = cam_pos - p.position;
                                let face_yaw = if to_camera.x * to_camera.x
                                    + to_camera.z * to_camera.z
                                    > 1.0e-6
                                {
                                    to_camera.x.atan2(to_camera.z)
                                } else {
                                    cam_yaw
                                };
                                let impostor_transform =
                                    glam::Mat4::from_scale_rotation_translation(
                                        glam::Vec3::new(
                                            lod_assets.model_half_width * 2.0 * p.scale,
                                            lod_assets.model_height * p.scale,
                                            1.0,
                                        ),
                                        glam::Quat::from_rotation_y(face_yaw),
                                        pos,
                                    );
                                lod3_instances.push(LodInstance {
                                    position: p.position,
                                    instance: astraweave_render::Instance {
                                        transform: impostor_transform,
                                        color: [p.tint[0], p.tint[1], p.tint[2], 1.0],
                                        material_id: 0,
                                    },
                                });
                                // Stage 3c.3-b: mirror this placement into the
                                // `Lod3InstanceRaw` buffer used by the new
                                // `ImpostorPass` path. The shader uses a
                                // single scalar scale (unit quad, x ∈ [-0.5,
                                // 0.5] × y ∈ [0, 1]); pick the vertical extent
                                // so card height matches the baked atlas
                                // footprint (see `fit_ortho_camera`).
                                #[cfg(feature = "impostor-bake")]
                                {
                                    let card_scale = lod_assets.model_height * p.scale;
                                    lod3_raw_instances.push(
                                        astraweave_render::impostor_lod3::Lod3InstanceRaw {
                                            position_scale: [pos.x, pos.y, pos.z, card_scale],
                                            species_and_params: [0.0, 0.0, 0.0, 0.0],
                                        },
                                    );
                                }
                            }
                        }
                    }

                    const SCATTER_GRID: usize = 6;
                    let mut x_min = f32::MAX;
                    let mut x_max = f32::MIN;
                    let mut z_min = f32::MAX;
                    let mut z_max = f32::MIN;
                    for p in items.iter() {
                        x_min = x_min.min(p.position.x);
                        x_max = x_max.max(p.position.x);
                        z_min = z_min.min(p.position.z);
                        z_max = z_max.max(p.position.z);
                    }
                    let span_x = (x_max - x_min).max(1.0);
                    let span_z = (z_max - z_min).max(1.0);
                    let cell_w = span_x / SCATTER_GRID as f32;
                    let cell_d = span_z / SCATTER_GRID as f32;

                    let bin_instances = |instances: Vec<LodInstance>| {
                        let mut bins: Vec<Vec<(glam::Vec3, astraweave_render::Instance)>> = (0
                            ..SCATTER_GRID * SCATTER_GRID)
                            .map(|_| Vec::new())
                            .collect();
                        for li in instances {
                            let gx = ((li.position.x - x_min) / cell_w) as usize;
                            let gz = ((li.position.z - z_min) / cell_d) as usize;
                            let gx = gx.min(SCATTER_GRID - 1);
                            let gz = gz.min(SCATTER_GRID - 1);
                            bins[gz * SCATTER_GRID + gx].push((li.position, li.instance));
                        }
                        bins
                    };

                    let lod0_bins = bin_instances(lod0_instances);
                    let lod1_bins = bin_instances(lod1_instances);
                    let lod2_bins = bin_instances(lod2_instances);
                    let lod3_bins = bin_instances(lod3_instances);

                    const SCATTER_MAX_TEX_SIZE: u32 = 512;

                    let tex_dir: Option<std::path::PathBuf> = {
                        let mesh_dir = std::path::Path::new(mesh_path).parent();
                        mesh_dir
                            .and_then(|d| d.parent())
                            .map(|d| d.join("textures"))
                    };
                    let mesh_hint = std::path::Path::new(mesh_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();

                    let mut find_texture_by_hint =
                        |hint: &str,
                         tex_dir: &Option<std::path::PathBuf>|
                         -> Option<(u32, u32, Vec<u8>)> {
                            if let Some(p) = diffuse_textures.get(hint) {
                                if p.exists() {
                                    let canon = p.canonicalize().unwrap_or_else(|_| p.clone());
                                    if let Some(cached) = self.scatter_texture_cache.get(&canon) {
                                        return Some(cached.clone());
                                    }
                                    if let Some(loaded) = Self::load_scatter_texture_from_path(
                                        p,
                                        SCATTER_MAX_TEX_SIZE,
                                    ) {
                                        self.scatter_texture_cache.insert(canon, loaded.clone());
                                        return Some(loaded);
                                    }
                                }
                            }
                            let td = tex_dir.as_ref()?;
                            if !td.is_dir() {
                                return None;
                            }
                            let candidates = std::fs::read_dir(td).ok()?;
                            let search = format!("{}_diff.", hint).to_lowercase();
                            let mut patterns: Vec<String> = vec![search];
                            let mut s = hint.to_string();
                            while let Some(idx) = s.rfind('_') {
                                s.truncate(idx);
                                patterns.push(format!("{}_diff.", s).to_lowercase());
                            }
                            for entry in candidates {
                                if let Ok(e) = entry {
                                    let name = e.file_name().to_string_lossy().to_lowercase();
                                    for pat in &patterns {
                                        if name.starts_with(pat.as_str()) {
                                            let path = e.path();
                                            let canon = path
                                                .canonicalize()
                                                .unwrap_or_else(|_| path.clone());
                                            if let Some(cached) =
                                                self.scatter_texture_cache.get(&canon)
                                            {
                                                return Some(cached.clone());
                                            }
                                            if let Some(entry) =
                                                Self::load_scatter_texture_from_path(
                                                    &path,
                                                    SCATTER_MAX_TEX_SIZE,
                                                )
                                            {
                                                self.scatter_texture_cache
                                                    .insert(canon, entry.clone());
                                                tracing::debug!(
                                                    "Scatter: loaded texture {}×{} for hint '{hint}' from {}",
                                                    entry.0,
                                                    entry.1,
                                                    path.display(),
                                                );
                                                return Some(entry);
                                            }
                                        }
                                    }
                                }
                            }
                            None
                        };

                    let lod0_count: usize = lod0_bins.iter().map(|b| b.len()).sum();
                    let lod1_count: usize = lod1_bins.iter().map(|b| b.len()).sum();
                    let lod2_count: usize = lod2_bins.iter().map(|b| b.len()).sum();
                    let lod3_count: usize = lod3_bins.iter().map(|b| b.len()).sum();
                    let mut primitive_card_sources: Vec<Option<String>> =
                        vec![None; lod_assets.primitives.len()];

                    for (prim_idx, primitive) in lod_assets.primitives.iter().enumerate() {
                        let mut first_lod0_source: Option<String> = None;
                        if lod0_count > 0 {
                            let cpu_mesh = &primitive.full_mesh;
                            let prim_external_tex: Option<(u32, u32, Vec<u8>)> =
                                if let Some(hint) = cpu_mesh.texture_source_hint.as_deref() {
                                    find_texture_by_hint(hint, &tex_dir)
                                } else {
                                    find_texture_by_hint(&mesh_hint, &tex_dir)
                                };

                            for (qi, quad) in lod0_bins.iter().enumerate() {
                                if quad.is_empty() {
                                    continue;
                                }
                                let sub_name = if lod_assets.primitives.len() > 1 {
                                    format!("scatter_{key}_lod0_p{prim_idx}_q{qi}")
                                } else {
                                    format!("scatter_{key}_lod0_q{qi}")
                                };
                                let instances: Vec<astraweave_render::Instance> =
                                    quad.iter().map(|(_, inst)| inst.clone()).collect();

                                if !self.renderer.update_model_instances(&sub_name, &instances) {
                                    let mesh = self.renderer.create_mesh_from_cpu_mesh(cpu_mesh);
                                    if let Some(ref source) = first_lod0_source {
                                        if !self.renderer.add_model_sharing_texture(
                                            &sub_name,
                                            mesh.clone(),
                                            &instances,
                                            source,
                                        ) {
                                            self.renderer.add_model(&sub_name, mesh, &instances);
                                        }
                                    } else if let Some((w, h, pixels)) = prim_external_tex.as_ref()
                                    {
                                        self.renderer.add_model_with_texture(
                                            &sub_name, mesh, &instances, *w, *h, pixels,
                                        );
                                    } else if let Some(img) = cpu_mesh.albedo_image.as_ref() {
                                        let tw = img.width.min(SCATTER_MAX_TEX_SIZE);
                                        let th = img.height.min(SCATTER_MAX_TEX_SIZE);
                                        if tw < img.width || th < img.height {
                                            let src = image::RgbaImage::from_raw(
                                                img.width,
                                                img.height,
                                                img.pixels.clone(),
                                            );
                                            if let Some(src_img) = src {
                                                let resized = image::imageops::resize(
                                                    &src_img,
                                                    tw,
                                                    th,
                                                    image::imageops::FilterType::Triangle,
                                                );
                                                self.renderer.add_model_with_texture(
                                                    &sub_name,
                                                    mesh,
                                                    &instances,
                                                    tw,
                                                    th,
                                                    &resized.into_raw(),
                                                );
                                            } else {
                                                self.renderer
                                                    .add_model(&sub_name, mesh, &instances);
                                            }
                                        } else {
                                            self.renderer.add_model_with_texture(
                                                &sub_name,
                                                mesh,
                                                &instances,
                                                img.width,
                                                img.height,
                                                &img.pixels,
                                            );
                                        }
                                    } else {
                                        self.renderer.add_model(&sub_name, mesh, &instances);
                                    }
                                }

                                let mut g_min = [f32::MAX; 3];
                                let mut g_max = [f32::MIN; 3];
                                let br = items[0].bounding_radius;
                                for (pos, _) in quad.iter() {
                                    g_min[0] = g_min[0].min(pos.x - br);
                                    g_min[1] = g_min[1].min(pos.y - br);
                                    g_min[2] = g_min[2].min(pos.z - br);
                                    g_max[0] = g_max[0].max(pos.x + br);
                                    g_max[1] = g_max[1].max(pos.y + br);
                                    g_max[2] = g_max[2].max(pos.z + br);
                                }
                                self.renderer.set_model_bounds(&sub_name, g_min, g_max);
                                used_model_names.insert(sub_name.clone());
                                if first_lod0_source.is_none() {
                                    first_lod0_source = Some(sub_name.clone());
                                }
                                actual_draw_calls += 1;
                                actual_instance_count += instances.len();
                                actual_triangle_count +=
                                    (cpu_mesh.indices.len() / 3) * instances.len();
                                actual_vertex_count += cpu_mesh.vertices.len() * instances.len();
                            }
                        }

                        let mut first_lod1_source: Option<String> = None;
                        if lod1_count > 0 {
                            let cpu_mesh = &primitive.simplified_mesh;
                            let prim_external_tex: Option<(u32, u32, Vec<u8>)> =
                                if let Some(hint) = cpu_mesh.texture_source_hint.as_deref() {
                                    find_texture_by_hint(hint, &tex_dir)
                                } else {
                                    find_texture_by_hint(&mesh_hint, &tex_dir)
                                };

                            for (qi, quad) in lod1_bins.iter().enumerate() {
                                if quad.is_empty() {
                                    continue;
                                }
                                let sub_name = if lod_assets.primitives.len() > 1 {
                                    format!("scatter_{key}_lod1_p{prim_idx}_q{qi}")
                                } else {
                                    format!("scatter_{key}_lod1_q{qi}")
                                };
                                let instances: Vec<astraweave_render::Instance> =
                                    quad.iter().map(|(_, inst)| inst.clone()).collect();

                                if !self.renderer.update_model_instances(&sub_name, &instances) {
                                    let mesh = self.renderer.create_mesh_from_cpu_mesh(cpu_mesh);
                                    if let Some(ref source) = first_lod1_source {
                                        if !self.renderer.add_model_sharing_texture(
                                            &sub_name,
                                            mesh.clone(),
                                            &instances,
                                            source,
                                        ) {
                                            self.renderer.add_model(&sub_name, mesh, &instances);
                                        }
                                    } else if let Some((w, h, pixels)) = prim_external_tex.as_ref()
                                    {
                                        self.renderer.add_model_with_texture(
                                            &sub_name, mesh, &instances, *w, *h, pixels,
                                        );
                                    } else if let Some(img) = cpu_mesh.albedo_image.as_ref() {
                                        let tw = img.width.min(SCATTER_MAX_TEX_SIZE);
                                        let th = img.height.min(SCATTER_MAX_TEX_SIZE);
                                        if tw < img.width || th < img.height {
                                            let src = image::RgbaImage::from_raw(
                                                img.width,
                                                img.height,
                                                img.pixels.clone(),
                                            );
                                            if let Some(src_img) = src {
                                                let resized = image::imageops::resize(
                                                    &src_img,
                                                    tw,
                                                    th,
                                                    image::imageops::FilterType::Triangle,
                                                );
                                                self.renderer.add_model_with_texture(
                                                    &sub_name,
                                                    mesh,
                                                    &instances,
                                                    tw,
                                                    th,
                                                    &resized.into_raw(),
                                                );
                                            } else {
                                                self.renderer
                                                    .add_model(&sub_name, mesh, &instances);
                                            }
                                        } else {
                                            self.renderer.add_model_with_texture(
                                                &sub_name,
                                                mesh,
                                                &instances,
                                                img.width,
                                                img.height,
                                                &img.pixels,
                                            );
                                        }
                                    } else {
                                        self.renderer.add_model(&sub_name, mesh, &instances);
                                    }
                                }

                                let mut g_min = [f32::MAX; 3];
                                let mut g_max = [f32::MIN; 3];
                                let br = items[0].bounding_radius;
                                for (pos, _) in quad.iter() {
                                    g_min[0] = g_min[0].min(pos.x - br);
                                    g_min[1] = g_min[1].min(pos.y - br);
                                    g_min[2] = g_min[2].min(pos.z - br);
                                    g_max[0] = g_max[0].max(pos.x + br);
                                    g_max[1] = g_max[1].max(pos.y + br);
                                    g_max[2] = g_max[2].max(pos.z + br);
                                }
                                self.renderer.set_model_bounds(&sub_name, g_min, g_max);
                                used_model_names.insert(sub_name.clone());
                                if first_lod1_source.is_none() {
                                    first_lod1_source = Some(sub_name.clone());
                                }
                                actual_draw_calls += 1;
                                actual_instance_count += instances.len();
                                actual_triangle_count +=
                                    (cpu_mesh.indices.len() / 3) * instances.len();
                                actual_vertex_count += cpu_mesh.vertices.len() * instances.len();
                            }
                        }

                        primitive_card_sources[prim_idx] = first_lod1_source
                            .clone()
                            .or_else(|| first_lod0_source.clone());
                    }

                    if lod2_count > 0 {
                        let cpu_mesh = &lod_assets.cross_billboard;
                        for (prim_idx, primitive) in lod_assets.primitives.iter().enumerate() {
                            let mut first_lod2_source: Option<String> = None;
                            let primitive_card_source = primitive_card_sources[prim_idx].clone();
                            let prim_external_tex: Option<(u32, u32, Vec<u8>)> =
                                if primitive_card_source.is_none() {
                                    if let Some(hint) =
                                        primitive.full_mesh.texture_source_hint.as_deref()
                                    {
                                        find_texture_by_hint(hint, &tex_dir)
                                    } else {
                                        find_texture_by_hint(&mesh_hint, &tex_dir)
                                    }
                                } else {
                                    None
                                };

                            for (qi, quad) in lod2_bins.iter().enumerate() {
                                if quad.is_empty() {
                                    continue;
                                }
                                let sub_name = if lod_assets.primitives.len() > 1 {
                                    format!("scatter_{key}_lod2_p{prim_idx}_q{qi}")
                                } else {
                                    format!("scatter_{key}_lod2_q{qi}")
                                };
                                let instances: Vec<astraweave_render::Instance> =
                                    quad.iter().map(|(_, inst)| inst.clone()).collect();

                                if !self.renderer.update_model_instances(&sub_name, &instances) {
                                    let mesh = self.renderer.create_mesh_from_cpu_mesh(cpu_mesh);
                                    if let Some(ref source) = first_lod2_source {
                                        if !self.renderer.add_model_sharing_texture(
                                            &sub_name,
                                            mesh.clone(),
                                            &instances,
                                            source,
                                        ) {
                                            self.renderer.add_model(&sub_name, mesh, &instances);
                                        }
                                    } else if let Some(ref source) = primitive_card_source {
                                        if !self.renderer.add_model_sharing_texture(
                                            &sub_name,
                                            mesh.clone(),
                                            &instances,
                                            source,
                                        ) {
                                            self.renderer.add_model(&sub_name, mesh, &instances);
                                        }
                                    } else if let Some((w, h, pixels)) = prim_external_tex.as_ref()
                                    {
                                        self.renderer.add_model_with_texture(
                                            &sub_name, mesh, &instances, *w, *h, pixels,
                                        );
                                    } else if let Some(img) =
                                        primitive.full_mesh.albedo_image.as_ref()
                                    {
                                        let tw = img.width.min(SCATTER_MAX_TEX_SIZE);
                                        let th = img.height.min(SCATTER_MAX_TEX_SIZE);
                                        if tw < img.width || th < img.height {
                                            let src = image::RgbaImage::from_raw(
                                                img.width,
                                                img.height,
                                                img.pixels.clone(),
                                            );
                                            if let Some(src_img) = src {
                                                let resized = image::imageops::resize(
                                                    &src_img,
                                                    tw,
                                                    th,
                                                    image::imageops::FilterType::Triangle,
                                                );
                                                self.renderer.add_model_with_texture(
                                                    &sub_name,
                                                    mesh,
                                                    &instances,
                                                    tw,
                                                    th,
                                                    &resized.into_raw(),
                                                );
                                            } else {
                                                self.renderer
                                                    .add_model(&sub_name, mesh, &instances);
                                            }
                                        } else {
                                            self.renderer.add_model_with_texture(
                                                &sub_name,
                                                mesh,
                                                &instances,
                                                img.width,
                                                img.height,
                                                &img.pixels,
                                            );
                                        }
                                    } else {
                                        self.renderer.add_model(&sub_name, mesh, &instances);
                                    }
                                }

                                let mut g_min = [f32::MAX; 3];
                                let mut g_max = [f32::MIN; 3];
                                let br = items[0].bounding_radius;
                                for (pos, _) in quad.iter() {
                                    g_min[0] = g_min[0].min(pos.x - br);
                                    g_min[1] = g_min[1].min(pos.y - br);
                                    g_min[2] = g_min[2].min(pos.z - br);
                                    g_max[0] = g_max[0].max(pos.x + br);
                                    g_max[1] = g_max[1].max(pos.y + br);
                                    g_max[2] = g_max[2].max(pos.z + br);
                                }
                                self.renderer.set_model_bounds(&sub_name, g_min, g_max);
                                used_model_names.insert(sub_name.clone());
                                if first_lod2_source.is_none() {
                                    first_lod2_source = Some(sub_name.clone());
                                }
                                actual_draw_calls += 1;
                                actual_instance_count += instances.len();
                                actual_triangle_count +=
                                    (cpu_mesh.indices.len() / 3) * instances.len();
                                actual_vertex_count += cpu_mesh.vertices.len() * instances.len();
                            }
                        }
                    }

                    // Stage 3c.3-b: route LOD3 through the new `ImpostorPass`
                    // path (baked atlas + alpha-tested billboard). Stage 3d
                    // (April 2026) deleted the legacy PBR-quad fallback —
                    // `--no-default-features` callers silently skip LOD3.
                    #[cfg(feature = "impostor-bake")]
                    if !lod3_raw_instances.is_empty() {
                        let view_proj = self.renderer.current_view_proj();
                        let camera_pos = self.camera_position;
                        // Bake atlas size + angle count match the uniform spec
                        // the `aw-impostor-bake` CLI defaults to. Keep these
                        // in sync with `tools/aw-impostor-bake/src/bin/…`.
                        const ATLAS_W: u32 = 512;
                        const ATLAS_H: u32 = 512;
                        const ANGLES: u32 = 8;
                        for (prim_idx, primitive) in lod_assets.primitives.iter().enumerate() {
                            let species_label = if lod_assets.primitives.len() > 1 {
                                format!("{key}_p{prim_idx}")
                            } else {
                                key.clone()
                            };
                            match self.upload_impostor_pass_for_primitive(
                                &primitive.full_mesh,
                                &lod3_raw_instances,
                                &species_label,
                                view_proj,
                                camera_pos,
                                ATLAS_W,
                                ATLAS_H,
                                ANGLES,
                            ) {
                                Ok(installed_key) => {
                                    impostor_live_keys.insert(installed_key);
                                    // Instrumentation: 1 indirect-free draw
                                    // call per pass, 2 triangles (quad) and 4
                                    // vertices per instance.
                                    actual_draw_calls += 1;
                                    actual_instance_count += lod3_raw_instances.len();
                                    actual_triangle_count += 2 * lod3_raw_instances.len();
                                    actual_vertex_count += 4 * lod3_raw_instances.len();
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "ImpostorPass upload failed for '{key}' p{prim_idx}: {e:#}"
                                    );
                                }
                            }
                        }
                    }

                    #[cfg(not(feature = "impostor-bake"))]
                    {
                        // Stage 3d (April 2026): legacy PBR-quad LOD3 fallback
                        // deleted. Under `--no-default-features`, LOD3 simply
                        // does not render — LOD0/1/2 tiers still cover
                        // near/mid-range scatter. Re-enable LOD3 by building
                        // with `--features impostor-bake`.
                        let _ = (lod3_count, &lod3_bins);
                    }

                    loaded_groups += 1;
                    let visible_count = lod0_count + lod1_count + lod2_count + lod3_count;

                    tracing::info!(
                        target: "aw_editor::viewport::perf",
                        "Scatter group '{key}': {:.1}ms ({} LOD0 + {} LOD1 + {} LOD2 + {} LOD3, {} culled, {} bins)",
                        group_start.elapsed().as_secs_f64() * 1000.0,
                        lod0_count,
                        lod1_count,
                        lod2_count,
                        lod3_count,
                        items.len().saturating_sub(visible_count),
                        lod0_bins
                            .iter()
                            .chain(lod1_bins.iter())
                            .chain(lod2_bins.iter())
                            .chain(lod3_bins.iter())
                            .filter(|q| !q.is_empty())
                            .count(),
                    );

                    // Previously submitted an empty command buffer every 4 groups
                    // to "flush" the queue; this caused pipeline stalls with no
                    // benefit now that we use update_model_instances instead of
                    // re-creating buffers. Removed to eliminate a ~1-3 ms stall
                    // per refresh during continuous camera motion.
                }
                Ok(Ok(_)) => {
                    tracing::warn!("Scatter: '{key}' glTF has no meshes: {mesh_path}");
                    skipped_load_fail += 1;
                }
                Ok(Err(e)) => {
                    tracing::warn!("Scatter: skipping '{key}' — glTF load failed: {e}");
                    skipped_load_fail += 1;
                }
                Err(_) => {
                    tracing::warn!("Scatter: '{key}' glTF load panicked — skipping: {mesh_path}");
                    skipped_load_fail += 1;
                }
            }
        }

        let stale_models: Vec<String> = self
            .scatter_model_names
            .iter()
            .filter(|name| !used_model_names.contains(*name))
            .cloned()
            .collect();
        for name in stale_models {
            self.renderer.clear_model(&name);
        }
        let mut current_model_names: Vec<String> = used_model_names.into_iter().collect();
        current_model_names.sort();
        self.scatter_model_names = current_model_names;

        // Stage 3c.3-b: drop impostor passes whose meshes fell out of this
        // refresh's live set. Passes still in `impostor_live_keys` remain
        // installed and will be refreshed next time they're encountered.
        #[cfg(feature = "impostor-bake")]
        self.retire_stale_impostor_passes(&impostor_live_keys);

        self.scatter_placement_count = actual_instance_count;
        self.scatter_draw_call_count = actual_draw_calls;
        self.scatter_total_triangles = actual_triangle_count;
        self.scatter_total_vertices = actual_vertex_count;

        let total = groups.len() as u32;
        let upload_elapsed = upload_start.elapsed();
        tracing::info!(
            target: "aw_editor::viewport",
            "Scatter upload: {loaded_groups}/{total} groups, {skipped_not_found} not found, {skipped_load_fail} failed, {} active instances, {} draws, {:.1}ms ({} mesh cache, {} lod cache, {} tex cache)",
            actual_instance_count,
            actual_draw_calls,
            upload_elapsed.as_secs_f64() * 1000.0,
            self.scatter_cpu_mesh_cache.len(),
            self.scatter_lod_asset_cache.len(),
            self.scatter_texture_cache.len(),
        );

        (loaded_groups, total, skipped_not_found, skipped_load_fail)
    }

    /// Re-bucket scatter instances by LOD level after camera movement.
    ///
    /// This avoids a full re-upload (no mesh loading or texture discovery).
    /// Only updates instance buffers based on new camera position.
    fn refresh_scatter_lod(&mut self) {
        if self.scatter_placements.is_empty() {
            return;
        }
        let t0 = std::time::Instant::now();

        // Full re-upload using retained data. The mesh cache ensures no
        // disk I/O — only LOD classification + instance buffer updates.
        let placements = std::mem::take(&mut self.scatter_placements);
        let textures = std::mem::take(&mut self.scatter_diffuse_textures);

        self.upload_scatter_placements(&placements, &textures);

        // Restore retained data (upload_scatter_placements sets them again,
        // but defensive)
        if self.scatter_placements.is_empty() {
            self.scatter_placements = placements;
            self.scatter_diffuse_textures = textures;
        }

        // Record completion time for the refresh-budget rate limiter.
        self.scatter_last_refresh = Some(std::time::Instant::now());

        tracing::debug!(
            target: "aw_editor::viewport::perf",
            "Scatter LOD refresh: {:.1}ms, {} models",
            t0.elapsed().as_secs_f64() * 1000.0,
            self.scatter_model_names.len(),
        );
    }

    /// Stream scatter chunks based on camera position: load chunks entering
    /// the active radius, unload chunks leaving it.
    ///
    /// Uses a hysteresis gap (load at 3 chunks, unload at 5 chunks) to
    /// prevent thrashing at chunk boundaries.
    fn stream_scatter_chunks(&mut self) {
        if self.scatter_placements.is_empty() || self.scatter_chunk_size < 1.0 {
            return;
        }
        self.refresh_scatter_lod();
    }

    /// Remove all scatter models belonging to a single chunk from the renderer.
    #[allow(dead_code)]
    fn unload_chunk_scatter(&mut self, chunk_id: astraweave_terrain::ChunkId) {
        // Remove model names associated with this chunk
        if let Some(names) = self.scatter_chunk_models.remove(&chunk_id) {
            for name in &names {
                self.renderer.clear_model(name);
                // Also remove from scatter_model_names
                if let Some(idx) = self.scatter_model_names.iter().position(|n| n == name) {
                    self.scatter_model_names.swap_remove(idx);
                }
            }
        }
        self.active_scatter_chunks.remove(&chunk_id);
    }

    /// Clear all scatter data from the engine renderer.
    pub fn clear_scatter(&mut self) {
        for name in self.scatter_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }
        self.scatter_placement_count = 0;
        self.scatter_draw_call_count = 0;
        self.scatter_total_triangles = 0;
        self.scatter_total_vertices = 0;
        self.scatter_placements.clear();
        self.scatter_diffuse_textures.clear();
        self.scatter_chunk_models.clear();
        self.active_scatter_chunks.clear();
        #[cfg(feature = "impostor-bake")]
        self.retire_all_impostor_passes();
    }

    /// Canonical on-disk cache root for baked impostor atlases
    /// (`assets/cache/impostors/`). Relative to the process cwd; the
    /// [`super::impostor_registry::ImpostorRegistry`] creates per-hash
    /// subdirectories under this root on first bake.
    #[cfg(feature = "impostor-bake")]
    fn default_impostor_cache_root() -> std::path::PathBuf {
        std::path::PathBuf::from("assets").join("cache").join("impostors")
    }

    /// Retire every impostor pass currently installed on the renderer
    /// (stage 3c.2). Mirrors the `scatter_model_names` drain in
    /// [`Self::clear_scatter`]: keys tracked in
    /// `installed_impostor_keys` are dropped both from the renderer
    /// (releasing GPU resources) and from the tracking set.
    #[cfg(feature = "impostor-bake")]
    fn retire_all_impostor_passes(&mut self) {
        for key in self.installed_impostor_keys.drain() {
            self.renderer.remove_impostor_pass(&key);
        }
    }

    /// Retire any impostor passes whose keys are not in the given
    /// `live_keys` set (stage 3c.2). Called at the end of a scatter LOD
    /// refresh to clean up passes for meshes that are no longer present
    /// (e.g. after a biome regen or chunk unload). Keys still in
    /// `live_keys` remain installed so their GPU resources are reused.
    #[cfg(feature = "impostor-bake")]
    #[allow(dead_code)] // consumed in stage 3c.3
    fn retire_stale_impostor_passes(&mut self, live_keys: &std::collections::HashSet<String>) {
        let stale: Vec<String> = self
            .installed_impostor_keys
            .iter()
            .filter(|k| !live_keys.contains(k.as_str()))
            .cloned()
            .collect();
        for key in stale {
            self.renderer.remove_impostor_pass(&key);
            self.installed_impostor_keys.remove(&key);
        }
    }

    /// Number of impostor passes currently installed on the renderer
    /// (stage 3c.2 instrumentation). Returns 0 when the feature is off.
    #[cfg(feature = "impostor-bake")]
    pub fn installed_impostor_pass_count(&self) -> usize {
        self.installed_impostor_keys.len()
    }

    #[cfg(not(feature = "impostor-bake"))]
    pub fn installed_impostor_pass_count(&self) -> usize {
        0
    }

    /// Ensure an [`ImpostorPass`] is installed on the renderer for the given
    /// primitive's content hash, then upload its per-frame camera matrices
    /// and instance buffer (stage 3c.3-a).
    ///
    /// Flow on **first encounter** for a mesh hash:
    /// 1. `primitive_mesh_hash(full_mesh)` → content-addressed `MeshHash`.
    /// 2. `registry.ensure(hash, spec, bake_fn)` → baked RGBA8 pixels (lazy
    ///    disk + mem cache via `astraweave_render::impostor_bake::load_or_bake_atlas`).
    /// 3. `ImpostorPass::new(…, renderer.hdr_format(), Some(renderer.depth_format()))`
    ///    constructs the pipeline + atlas + instance buffer.
    /// 4. `renderer.install_impostor_pass(hash.as_str(), pass)` registers the
    ///    pass under the stable hash key and tracks it in
    ///    `installed_impostor_keys`.
    ///
    /// On **subsequent encounters** (pass already installed), steps 1-4 are
    /// skipped: we lookup the existing pass via `impostor_pass_mut` and only
    /// refresh camera + instances.
    ///
    /// The `species_name` argument is the single-species label baked into the
    /// atlas sidecar. Pass a stable identifier (typically the scatter group's
    /// mesh key) so disk-cached atlases remain recognisable.
    ///
    /// # Returns
    ///
    /// The key the pass was installed under (a clone of `hash.as_str()`). The
    /// caller should record it in its live-keys set so
    /// [`Self::retire_stale_impostor_passes`] can drop it when the mesh falls
    /// out of the active scatter set.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * the primitive has no vertices (hash + bake would fail),
    /// * the atlas fails to bake (propagated from `bake_primitive_pixels`),
    /// * `ImpostorPass::new` rejects the bake result (format mismatch, etc.).
    #[cfg(feature = "impostor-bake")]
    pub fn upload_impostor_pass_for_primitive(
        &mut self,
        primitive_full_mesh: &astraweave_render::mesh::CpuMesh,
        raw_instances: &[astraweave_render::impostor_lod3::Lod3InstanceRaw],
        species_name: &str,
        view_proj: glam::Mat4,
        camera_pos: glam::Vec3,
        atlas_width: u32,
        atlas_height: u32,
        angle_count: u32,
    ) -> Result<String> {
        use super::impostor_wiring::{bake_primitive_pixels, primitive_mesh_hash};
        use astraweave_render::impostor_pass::ImpostorPass;
        use astraweave_render::vegetation_lod::ImpostorAtlasSpec;

        let hash = primitive_mesh_hash(primitive_full_mesh);
        let key = hash.as_str().to_string();

        // Cheap Arc-backed clones so we can hold refs while `impostor_pass_mut`
        // takes a &mut borrow of the renderer.
        let device_clone = self.renderer.device().clone();
        let queue_clone = self.renderer.queue().clone();

        // Fast path: pass already installed — just refresh dynamic state.
        if self.renderer.has_impostor_pass(&key) {
            if let Some(pass) = self.renderer.impostor_pass_mut(&key) {
                pass.update_camera(&queue_clone, view_proj, camera_pos);
                pass.upload_instances(&device_clone, &queue_clone, raw_instances);
            }
            self.installed_impostor_keys.insert(key.clone());
            return Ok(key);
        }

        // Slow path: bake (or disk-load) the atlas, then build + install the pass.
        let registry = self
            .impostor_registry
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("impostor registry not initialised"))?;

        let spec = ImpostorAtlasSpec::uniform(atlas_width, atlas_height, angle_count, &[species_name]);

        // Clone cheap Arc-backed device/queue handles so the bake closure can
        // own them without conflicting with `registry`'s &mut borrow on self.
        let bake_device = device_clone.clone();
        let bake_queue = queue_clone.clone();
        let loaded = registry.ensure(&hash, &spec, move |s| {
            bake_primitive_pixels(&bake_device, &bake_queue, primitive_full_mesh, s, species_name)
        })?;

        let pass = ImpostorPass::new(
            &device_clone,
            &queue_clone,
            &loaded.pixels,
            loaded.width,
            loaded.height,
            loaded.spec.clone(),
            self.renderer.hdr_format(),
            Some(self.renderer.depth_format()),
        )
        .context("constructing ImpostorPass from baked atlas")?;

        self.renderer.install_impostor_pass(key.clone(), pass);
        self.installed_impostor_keys.insert(key.clone());

        // Upload initial camera + instances now that the pass is installed.
        if let Some(pass_mut) = self.renderer.impostor_pass_mut(&key) {
            pass_mut.update_camera(&queue_clone, view_proj, camera_pos);
            pass_mut.upload_instances(&device_clone, &queue_clone, raw_instances);
        }

        Ok(key)
    }

    /// Total scatter placements currently loaded.
    pub fn scatter_instance_count(&self) -> usize {
        self.scatter_placement_count
    }

    /// Number of unique scatter draw calls (one per mesh type).
    pub fn scatter_draw_calls(&self) -> u32 {
        self.scatter_draw_call_count
    }

    /// Total scatter triangles represented by the currently uploaded models.
    pub fn scatter_triangles(&self) -> usize {
        self.scatter_total_triangles
    }

    /// Total scatter vertices represented by the currently uploaded models.
    pub fn scatter_vertices(&self) -> usize {
        self.scatter_total_vertices
    }

    // ── Sky / weather / environment ─────────────────────────────────────

    /// Set the sky configuration on the engine renderer.
    pub fn set_sky_config(&mut self, cfg: astraweave_render::SkyConfig) {
        self.renderer.set_sky_config(cfg);
    }

    /// Load an HDRI file as the skybox and rebake IBL environment maps.
    pub fn load_hdri(&mut self, path: &std::path::Path) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();
        tracing::info!("Loading HDRI skybox from: {path_str}");
        self.renderer.ibl_mut().mode = astraweave_render::ibl::SkyMode::HdrPath {
            biome: "editor".to_string(),
            path: path_str,
        };
        self.renderer
            .bake_environment(astraweave_render::ibl::IblQuality::Medium)
            .context("Failed to bake HDRI environment")?;
        tracing::info!("HDRI skybox loaded and IBL baked successfully");
        Ok(())
    }

    /// Get the current sky configuration.
    pub fn sky_config(&self) -> astraweave_render::SkyConfig {
        self.renderer.sky_config()
    }

    /// Set weather type on the engine renderer.
    pub fn set_weather(&mut self, kind: astraweave_render::WeatherKind) {
        self.weather_active = kind != astraweave_render::WeatherKind::None;
        self.renderer.set_weather(kind);
    }

    /// Whether weather effects are currently active.
    pub fn weather_active(&self) -> bool {
        self.weather_active
    }

    /// Tick the weather particle system.
    pub fn tick_weather(&mut self, dt: f32) {
        self.renderer.tick_weather(dt);
    }

    /// Advance the environment (time-of-day, sky parameters).
    pub fn tick_environment(&mut self, dt: f32) {
        self.renderer.tick_environment(dt);
    }

    // ── Fog / lighting ──────────────────────────────────────────────────

    /// Apply fog parameters to the engine's scene environment.
    ///
    /// Only overwrites fog when the world panel's fog toggle is enabled.
    /// When disabled, terrain-scaled fog values (set by `upload_terrain_chunks`)
    /// are preserved.
    pub fn set_fog_params(&mut self, params: &TerrainFogParams) {
        let env = self.renderer.scene_environment_mut();
        // Always apply fog parameters — sliders work whether fog is "enabled" or not.
        // The "Enable Fog" checkbox gates weather-triggered fog; the start/end/density
        // sliders always control the scene atmosphere.
        env.visuals.fog_start = params.fog_start;
        env.visuals.fog_end = params.fog_end;
        env.visuals.fog_density = params.fog_density;
        env.visuals.fog_color = glam::Vec3::from(params.fog_color);
        // Apply particle count override from the UI slider
        if let Some(count) = params.particle_count_override {
            self.renderer.set_weather_max(count as usize);
        }
    }

    /// Apply lighting parameters to the engine's scene environment and camera UBO.
    ///
    /// Updates ambient lighting in the SceneEnvironment, and overrides the
    /// sun direction + intensity in the CameraUBO so the PBR shader uses
    /// the world panel's lighting settings instead of the internal TimeOfDay.
    pub fn set_lighting_params(&mut self, params: &TerrainLightingParams) {
        let env = self.renderer.scene_environment_mut();
        env.visuals.ambient_color = glam::Vec3::from(params.ambient_color);
        env.visuals.ambient_intensity = params.ambient_intensity;
        env.sun_color = params.sun_color;
        env.sun_intensity = params.sun_intensity;

        // Negate: sun_dir points TO the sun (positive Y = sun above),
        // but the shader convention for light_dir is direction FROM the
        // sun (negative Y = light traveling downward). The shader then
        // does L = normalize(-light_dir) to get the direction toward
        // the light source.
        let dir = (-glam::Vec3::from(params.sun_dir)).normalize();
        self.renderer
            .set_light_direction_override(dir, params.sun_intensity);
    }

    /// Update the post-processing chain (bloom/SSAO/SSR enable flags + tonemap).
    pub fn set_post_process_chain(
        &mut self,
        chain: astraweave_render::hdr_pipeline::PostProcessChain,
    ) {
        self.renderer.set_post_process_chain(chain);
    }

    /// Get the current post-processing chain.
    pub fn post_process_chain(&self) -> &astraweave_render::hdr_pipeline::PostProcessChain {
        self.renderer.post_process_chain()
    }

    /// Update bloom compute-pass parameters (intensity, threshold, etc.).
    pub fn set_bloom_config(&mut self, config: astraweave_render::bloom::BloomConfig) {
        self.renderer.set_bloom_config(config);
    }

    /// Set water configuration on the engine renderer.
    pub fn set_water_enabled(&mut self, enabled: bool, style: WaterStyle) {
        self.water_enabled = enabled;
        if enabled {
            let format = self.renderer.surface_format();
            let water = astraweave_render::WaterRenderer::new(
                self.renderer.device(),
                format,
                wgpu::TextureFormat::Depth32Float,
            );
            // Apply style-specific colors
            let (deep, shallow, foam) = match style {
                WaterStyle::Ocean => (
                    glam::Vec3::new(0.02, 0.08, 0.2),
                    glam::Vec3::new(0.1, 0.4, 0.5),
                    glam::Vec3::new(0.95, 0.98, 1.0),
                ),
                WaterStyle::River => (
                    glam::Vec3::new(0.01, 0.05, 0.04),
                    glam::Vec3::new(0.04, 0.10, 0.08),
                    glam::Vec3::new(0.9, 0.95, 0.9),
                ),
                WaterStyle::Lake => (
                    glam::Vec3::new(0.005, 0.04, 0.06),
                    glam::Vec3::new(0.02, 0.09, 0.12),
                    glam::Vec3::new(0.9, 0.95, 1.0),
                ),
                WaterStyle::Swamp => (
                    glam::Vec3::new(0.02, 0.03, 0.01),
                    glam::Vec3::new(0.05, 0.06, 0.03),
                    glam::Vec3::new(0.7, 0.75, 0.6),
                ),
            };
            let mut water = water;
            water.set_water_colors(deep, shallow, foam);
            self.renderer.set_water_renderer(water);
        } else {
            self.renderer.clear_water_renderer();
        }
    }

    /// Update water animation state each frame.
    pub fn update_water(&mut self, camera: &OrbitCamera, time: f32) {
        let engine_camera = camera.to_engine_camera();
        let vp = engine_camera.vp();
        let pos = camera.position();
        self.renderer.update_water(vp, pos, time);
    }
}

// ─── Default cube geometry for entities without meshes ──────────────────────
#[rustfmt::skip]
const CUBE_POSITIONS: [[f32; 3]; 24] = [
    // Front face (+Z)
    [-0.5, -0.5,  0.5], [ 0.5, -0.5,  0.5], [ 0.5,  0.5,  0.5], [-0.5,  0.5,  0.5],
    // Back face (-Z)
    [ 0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5,  0.5, -0.5], [ 0.5,  0.5, -0.5],
    // Top face (+Y)
    [-0.5,  0.5,  0.5], [ 0.5,  0.5,  0.5], [ 0.5,  0.5, -0.5], [-0.5,  0.5, -0.5],
    // Bottom face (-Y)
    [-0.5, -0.5, -0.5], [ 0.5, -0.5, -0.5], [ 0.5, -0.5,  0.5], [-0.5, -0.5,  0.5],
    // Right face (+X)
    [ 0.5, -0.5,  0.5], [ 0.5, -0.5, -0.5], [ 0.5,  0.5, -0.5], [ 0.5,  0.5,  0.5],
    // Left face (-X)
    [-0.5, -0.5, -0.5], [-0.5, -0.5,  0.5], [-0.5,  0.5,  0.5], [-0.5,  0.5, -0.5],
];

#[rustfmt::skip]
const CUBE_NORMALS: [[f32; 3]; 24] = [
    // Front
    [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0],
    // Back
    [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0],
    // Top
    [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0],
    // Bottom
    [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0],
    // Right
    [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0],
    // Left
    [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0],
];

#[rustfmt::skip]
const CUBE_INDICES: [u32; 36] = [
    0,  1,  2,  2,  3,  0,   // Front
    4,  5,  6,  6,  7,  4,   // Back
    8,  9,  10, 10, 11, 8,   // Top
    12, 13, 14, 14, 15, 12,  // Bottom
    16, 17, 18, 18, 19, 16,  // Right
    20, 21, 22, 22, 23, 20,  // Left
];

#[cfg(test)]
mod tests {
    use super::*;

    // Phase 1 post-completion fix — triangle streak regression.
    // The editor's terrain chunks have `N² + 4N` vertices (N² surface +
    // 4N edge skirts) and the surface triangle indices come first,
    // followed by skirt triangle indices that reference vertices
    // `≥ N²`. The 1.E.4.c prefix-take filter mis-computed N from
    // `floor(sqrt(total))` which overshoots to `N+2`, causing 1542
    // skirt indices to leak into the forward upload and reference
    // out-of-bounds vertices (the streaks).

    #[test]
    fn infer_surface_grid_dim_handles_with_skirts() {
        // N=129, typical editor chunk resolution.
        let vertex_count = 129 * 129 + 4 * 129; // 17157
        assert_eq!(infer_surface_grid_dim(vertex_count), Some(129));

        // Smaller cases to exercise the algorithm.
        assert_eq!(infer_surface_grid_dim(2 * 2 + 4 * 2), Some(2));       // 12
        assert_eq!(infer_surface_grid_dim(3 * 3 + 4 * 3), Some(3));       // 21
        assert_eq!(infer_surface_grid_dim(4 * 4 + 4 * 4), Some(4));       // 32
        assert_eq!(infer_surface_grid_dim(17 * 17 + 4 * 17), Some(17));   // 357
        assert_eq!(infer_surface_grid_dim(65 * 65 + 4 * 65), Some(65));   // 4485
        assert_eq!(infer_surface_grid_dim(257 * 257 + 4 * 257), Some(257)); // 67077
    }

    #[test]
    fn infer_surface_grid_dim_handles_plain_squares() {
        // Test-fixture chunks without skirts.
        assert_eq!(infer_surface_grid_dim(4), Some(2));
        assert_eq!(infer_surface_grid_dim(9), Some(3));
        assert_eq!(infer_surface_grid_dim(16), Some(4));
        assert_eq!(infer_surface_grid_dim(129 * 129), Some(129));
    }

    #[test]
    fn infer_surface_grid_dim_rejects_nonmatching_shapes() {
        // Neither N² nor N² + 4N.
        assert_eq!(infer_surface_grid_dim(0), None);
        assert_eq!(infer_surface_grid_dim(1), None);
        assert_eq!(infer_surface_grid_dim(3), None);
        assert_eq!(infer_surface_grid_dim(5), None);
        assert_eq!(infer_surface_grid_dim(17155), None); // close to 129² + 4·129 but off
        assert_eq!(infer_surface_grid_dim(17158), None); // one past it
    }

    #[test]
    fn infer_surface_grid_dim_prefers_skirts_over_plain_when_ambiguous() {
        // No real collisions exist between N² and M² + 4M for integer N, M
        // in practical sizes, but the algorithm tries the with-skirts form
        // first — verify the precedence doesn't accidentally shadow a
        // plain-square match. Using N=10: 100 = 10² (plain); check the
        // with-skirts discriminant 104, sqrt≈10.2, rounded to 10, N=8,
        // 8² + 32 = 96 ≠ 100, so the skirts form correctly rejects.
        assert_eq!(infer_surface_grid_dim(100), Some(10));
    }

    #[test]
    fn filter_surface_triangles_drops_skirt_triangles() {
        // Mock a tiny editor index buffer: 2 surface triangles then 2
        // skirt triangles. Surface vertex count = 4.
        let indices: Vec<u32> = vec![
            // Surface triangles — all indices < 4.
            0, 1, 2,
            1, 3, 2,
            // Skirt triangles — each has a corner at index 4 or 5.
            0, 4, 1,
            1, 4, 5,
        ];
        let out = filter_surface_triangles(&indices, 4);
        assert_eq!(out, vec![0, 1, 2, 1, 3, 2]);
    }

    #[test]
    fn filter_surface_triangles_handles_interleaved() {
        // Skirts interleaved with surface triangles — the filter must
        // drop the right ones regardless of ordering.
        let indices: Vec<u32> = vec![
            0, 1, 2,  // surface
            0, 4, 1,  // skirt
            2, 3, 0,  // surface
            1, 5, 4,  // skirt
        ];
        let out = filter_surface_triangles(&indices, 4);
        assert_eq!(out, vec![0, 1, 2, 2, 3, 0]);
    }

    #[test]
    fn filter_surface_triangles_preserves_triangle_count_for_editor_layout() {
        // Replicate the editor's layout: N=5 produces 5²=25 surface
        // vertices, (5-1)²·2=32 surface triangles (96 indices), and
        // 4·(5-1)·2 = 32 skirt triangles (96 indices). The filter
        // should keep exactly 96 surface indices and drop 96 skirt
        // indices.
        let mut indices = Vec::new();
        // 32 surface triangles with all indices < 25.
        for i in 0..32u32 {
            indices.extend_from_slice(&[0, 1, 2]);
            let _ = i;
        }
        // 32 skirt triangles with at least one index ≥ 25.
        for i in 0..32u32 {
            indices.extend_from_slice(&[0, 1, 25 + (i % 20)]);
        }
        let out = filter_surface_triangles(&indices, 25);
        assert_eq!(out.len(), 32 * 3);
        assert!(out.chunks_exact(3).all(|t| {
            t[0] < 25 && t[1] < 25 && t[2] < 25
        }));
    }

    #[test]
    fn filter_surface_triangles_handles_empty_and_trailing_stray() {
        // Empty input.
        assert!(filter_surface_triangles(&[], 4).is_empty());

        // Trailing 1-2 stray indices (not a full triangle) are silently
        // dropped by `chunks_exact(3)`.
        let indices = vec![0u32, 1, 2, 0, 1];
        let out = filter_surface_triangles(&indices, 4);
        assert_eq!(out, vec![0, 1, 2]);
    }

    #[cfg(feature = "impostor-bake")]
    #[test]
    fn default_impostor_cache_root_is_assets_cache_impostors() {
        let root = EngineRenderAdapter::default_impostor_cache_root();
        let components: Vec<&str> = root
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => s.to_str(),
                _ => None,
            })
            .collect();
        assert_eq!(components, vec!["assets", "cache", "impostors"]);
    }

    fn terrain_vertex(
        position: [f32; 3],
        biome_weights_0: [f32; 4],
        biome_weights_1: [f32; 4],
        material_ids: [f32; 4],
        material_weights: [f32; 4],
    ) -> TerrainVertex {
        TerrainVertex {
            position,
            normal: [0.0, 1.0, 0.0],
            uv: [position[0], position[2]],
            biome_weights_0,
            biome_weights_1,
            material_ids,
            material_weights,
        }
    }

    fn assert_color_approx_eq(actual: [f32; 4], expected: [f32; 4]) {
        for channel in 0..4 {
            assert!(
                (actual[channel] - expected[channel]).abs() < 0.0001,
                "channel {channel} mismatch: actual={:?}, expected={:?}",
                actual,
                expected,
            );
        }
    }

    #[test]
    fn terrain_cluster_plan_covers_all_chunks_once() {
        let chunks = [
            TerrainChunkPlanningInfo {
                aabb_min: [0.0, 0.0, 0.0],
                aabb_max: [10.0, 1.0, 10.0],
                vertex_count: 100,
            },
            TerrainChunkPlanningInfo {
                aabb_min: [12.0, 0.0, 0.0],
                aabb_max: [22.0, 1.0, 10.0],
                vertex_count: 100,
            },
            TerrainChunkPlanningInfo {
                aabb_min: [0.0, 0.0, 12.0],
                aabb_max: [10.0, 1.0, 22.0],
                vertex_count: 100,
            },
            TerrainChunkPlanningInfo {
                aabb_min: [12.0, 0.0, 12.0],
                aabb_max: [22.0, 1.0, 22.0],
                vertex_count: 100,
            },
        ];

        let mut planned_chunks: Vec<_> = build_terrain_cluster_plan(&chunks, 2, 10_000)
            .into_iter()
            .flatten()
            .collect();
        planned_chunks.sort_unstable();

        assert_eq!(planned_chunks, vec![0, 1, 2, 3]);
    }

    #[test]
    fn terrain_cluster_plan_splits_large_bins_by_vertex_budget() {
        let chunks = [
            TerrainChunkPlanningInfo {
                aabb_min: [0.0, 0.0, 0.0],
                aabb_max: [4.0, 1.0, 4.0],
                vertex_count: 4,
            },
            TerrainChunkPlanningInfo {
                aabb_min: [1.0, 0.0, 1.0],
                aabb_max: [5.0, 1.0, 5.0],
                vertex_count: 4,
            },
            TerrainChunkPlanningInfo {
                aabb_min: [2.0, 0.0, 2.0],
                aabb_max: [6.0, 1.0, 6.0],
                vertex_count: 4,
            },
        ];

        let plan = build_terrain_cluster_plan(&chunks, 1, 5);
        assert_eq!(plan, vec![vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn terrain_surface_summary_blends_material_tint_over_biome_fallback() {
        let mut summary = TerrainSurfaceSummary::default();
        summary.add_vertex(&terrain_vertex(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ));

        let expected = blend_tints(TERRAIN_BIOME_TINTS[0], TERRAIN_MATERIAL_TINTS[1], 0.65);
        assert_color_approx_eq(summary.resolve_tint(), expected);
    }

    #[test]
    fn terrain_surface_summary_tracks_dominant_material_index() {
        let mut summary = TerrainSurfaceSummary::default();
        summary.add_vertex(&terrain_vertex(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [3.0, 5.0, 0.0, 0.0],
            [0.25, 0.75, 0.0, 0.0],
        ));

        assert_eq!(summary.dominant_material_index(), Some(5));
    }

    #[test]
    fn convert_terrain_chunk_accumulates_surface_summary_across_vertices() {
        let vertices = vec![
            terrain_vertex(
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
            ),
            terrain_vertex(
                [4.0, 2.0, 8.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
            ),
        ];

        let chunk = EngineRenderAdapter::convert_terrain_chunk(&vertices, &[0, 1, 0]);
        let mut expected_biomes = [0.0; 8];
        expected_biomes[0] = 1.0;
        expected_biomes[1] = 1.0;
        let expected_tint = weighted_palette_tint(&expected_biomes, &TERRAIN_BIOME_TINTS)
            .expect("expected biome tint");

        assert_eq!(chunk.indices, vec![0, 1, 0]);
        assert_eq!(chunk.aabb_min, [0.0, 0.0, 0.0]);
        assert_eq!(chunk.aabb_max, [4.0, 2.0, 8.0]);
        assert_color_approx_eq(chunk.surface_summary.resolve_tint(), expected_tint);
    }

    // ── TerrainHeightGrid (Phase 5.1 scatter Y grounding) ──────────────

    /// Build a synthetic single-chunk render data with a flat-ish ramp so
    /// the grid has a predictable shape for sampling assertions.
    fn make_chunk(positions: Vec<[f32; 3]>) -> TerrainChunkRenderData {
        let mut min = [f32::MAX, f32::MAX, f32::MAX];
        let mut max = [f32::MIN, f32::MIN, f32::MIN];
        for p in &positions {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        let count = positions.len();
        TerrainChunkRenderData {
            positions,
            normals: vec![[0.0, 1.0, 0.0]; count],
            tangents: vec![[1.0, 0.0, 0.0, 1.0]; count],
            uvs: vec![[0.0, 0.0]; count],
            indices: Vec::new(),
            surface_summary: TerrainSurfaceSummary::default(),
            aabb_min: min,
            aabb_max: max,
        }
    }

    #[test]
    fn height_grid_build_returns_none_for_empty_input() {
        assert!(TerrainHeightGrid::build(&[]).is_none());
        let empty = make_chunk(Vec::new());
        assert!(TerrainHeightGrid::build(std::slice::from_ref(&empty)).is_none());
    }

    #[test]
    fn height_grid_build_returns_none_for_degenerate_extent() {
        // All vertices at the same XZ — zero extent on both axes.
        let chunk = make_chunk(vec![
            [5.0, 1.0, 5.0],
            [5.0, 2.0, 5.0],
            [5.0, 3.0, 5.0],
        ]);
        assert!(TerrainHeightGrid::build(std::slice::from_ref(&chunk)).is_none());
    }

    #[test]
    fn height_grid_samples_flat_plane_exactly() {
        // A 10×10 flat plane at Y=7 with a 2 m vertex spacing.
        let mut positions = Vec::new();
        for gz in 0..6 {
            for gx in 0..6 {
                positions.push([gx as f32 * 2.0, 7.0, gz as f32 * 2.0]);
            }
        }
        let chunk = make_chunk(positions);
        let grid = TerrainHeightGrid::build(std::slice::from_ref(&chunk))
            .expect("grid should build from flat plane");

        // Interior sample → exactly 7.0.
        let y = grid.sample(5.0, 5.0).expect("interior sample");
        assert!((y - 7.0).abs() < 1e-4, "expected 7.0, got {y}");

        // Corner sample (still inside extent).
        let y = grid.sample(0.0, 0.0).expect("corner sample");
        assert!((y - 7.0).abs() < 1e-4, "expected 7.0, got {y}");

        // Outside extent → None.
        assert!(grid.sample(-1.0, 5.0).is_none());
        assert!(grid.sample(5.0, 100.0).is_none());
    }

    #[test]
    fn height_grid_bilinear_interpolates_ramp() {
        // Build a ramp where Y = X. With cell size ≈ 1 m, bilinear sampling
        // at mid-cell should give a value between neighbouring vertices.
        let mut positions = Vec::new();
        for gz in 0..4 {
            for gx in 0..8 {
                let x = gx as f32;
                positions.push([x, x, gz as f32]);
            }
        }
        let chunk = make_chunk(positions);
        let grid = TerrainHeightGrid::build(std::slice::from_ref(&chunk))
            .expect("grid should build from ramp");

        // Monotonicity: a larger X sample must not return a smaller height
        // than a smaller X sample on a non-decreasing ramp. This is the
        // property scatter grounding actually needs (flat + ascending
        // surfaces both get a height at or above the true vertex Y).
        let y_low = grid.sample(1.5, 1.0).expect("low sample");
        let y_mid = grid.sample(3.5, 1.0).expect("mid sample");
        let y_high = grid.sample(5.5, 1.0).expect("high sample");
        assert!(y_low <= y_mid + 1e-3, "non-monotonic: {y_low} > {y_mid}");
        assert!(y_mid <= y_high + 1e-3, "non-monotonic: {y_mid} > {y_high}");
        // All samples should be within the ramp range [0, 7] extended by
        // at most one cell for max-Y rasterisation.
        for y in [y_low, y_mid, y_high] {
            assert!((0.0..=8.0).contains(&y), "sample {y} outside expected range");
        }
    }

    #[test]
    fn height_grid_clamps_cell_count_under_cap() {
        // Realistic-ish 2 km × 2 km terrain at ~4 m vertex spacing
        // (~250 k vertices) — the grid must stay under MAX_CELLS regardless
        // of vertex density estimate.
        let mut positions = Vec::new();
        let step = 4.0_f32;
        let extent_cells = 500; // 2000 m / 4 m
        for gz in 0..extent_cells {
            for gx in 0..extent_cells {
                positions.push([gx as f32 * step, 1.0, gz as f32 * step]);
            }
        }
        let chunk = make_chunk(positions);
        let grid = TerrainHeightGrid::build(std::slice::from_ref(&chunk))
            .expect("grid should build from dense terrain");
        let cells = grid.width * grid.height;
        assert!(
            cells <= TerrainHeightGrid::MAX_CELLS,
            "cell count {cells} exceeds cap {}",
            TerrainHeightGrid::MAX_CELLS,
        );
        assert!(
            grid.cell_size >= TerrainHeightGrid::MIN_CELL_SIZE - 1e-4,
            "cell size {} below min",
            grid.cell_size,
        );
        assert!(
            grid.cell_size <= TerrainHeightGrid::MAX_CELL_SIZE + 1e-4,
            "cell size {} above max",
            grid.cell_size,
        );
    }

    #[test]
    fn height_grid_stores_max_y_per_cell() {
        // Two vertices in the same cell with different heights — the
        // grid must record the higher one so scatter sits on ridges.
        let positions = vec![
            [0.0, 1.0, 0.0],
            [0.1, 9.0, 0.1], // same cell as (0,0) at 1 m resolution
            [5.0, 5.0, 0.0],
            [0.0, 5.0, 5.0],
            [5.0, 5.0, 5.0],
        ];
        let chunk = make_chunk(positions);
        let grid = TerrainHeightGrid::build(std::slice::from_ref(&chunk))
            .expect("grid should build");
        let y = grid.sample(0.05, 0.05).expect("sample near origin");
        assert!(y >= 8.9, "expected max-Y ~9.0, got {y}");
    }

    #[test]
    fn fog_color_from_sky_matches_day_horizon() {
        let sky = astraweave_render::SkyConfig {
            day_color_top: glam::Vec3::new(0.1, 0.2, 0.3),
            day_color_horizon: glam::Vec3::new(0.4, 0.5, 0.6),
            sunset_color_top: glam::Vec3::ZERO,
            sunset_color_horizon: glam::Vec3::ZERO,
            night_color_top: glam::Vec3::ZERO,
            night_color_horizon: glam::Vec3::ZERO,
            cloud_coverage: 0.0,
            cloud_speed: 0.0,
            cloud_altitude: 0.0,
        };
        assert_eq!(fog_color_from_sky(&sky), sky.day_color_horizon);
    }
}
