//! Phase 1.X-F.4: paintable 2D archetype mask + falloff distance field for
//! the Regional Archetype Variation campaign's multi-archetype data path.
//!
//! Per campaign doc §2.4: each pixel of the 1024×1024 mask carries an
//! archetype ID (uint8) + a precomputed Euclidean distance-to-nearest-other-
//! archetype (uint8 normalized by `falloff_radius_pixels`). At Target B's
//! ~11264 WU world extent, this gives ~11 WU per pixel — sufficient for
//! archetype-region authoring; transitions handled by falloff at
//! sampling time, not by mask resolution.
//!
//! F.4.A scope (this commit):
//! - [`RegionalArchetypeMask`] type + unpainted constructor + paint helpers.
//! - [`RegionalArchetypeMaskMetadata`] for serde (RON) persistence.
//! - [`crate::world_archetypes::WorldArchetypeId`] mask-id bidirectional mapping.
//!
//! F.4.B adds save/load file I/O + Euclidean distance transform.
//! F.4.C adds [`RegionalArchetypeBlend`] sampler.
//! F.4.D adds blend math (`blend_bootstrap_params`).
//! F.4.E wires into `WorldGenerator`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// Phase 1.X-F.4.B: errors from [`RegionalArchetypeMask`] save/load operations.
#[derive(Debug, thiserror::Error)]
pub enum MaskIoError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("RON serde error: {0}")]
    Ron(String),
    #[error("size mismatch: expected {expected} bytes, got {actual} bytes")]
    Mismatch { expected: usize, actual: usize },
    #[error("unsupported mask format version {0} (this build supports version 1)")]
    UnsupportedVersion(u32),
}

impl From<ron::error::SpannedError> for MaskIoError {
    fn from(e: ron::error::SpannedError) -> Self {
        Self::Ron(e.to_string())
    }
}

impl From<ron::Error> for MaskIoError {
    fn from(e: ron::Error) -> Self {
        Self::Ron(e.to_string())
    }
}

/// Phase 1.X-F.4.A: paintable 2D archetype mask with precomputed
/// Euclidean distance falloff field. Stored as two uint8 grids:
/// `ids` (archetype identity per pixel; 0 = unpainted) and `falloff`
/// (distance-to-nearest-other-archetype per pixel, normalized by
/// `falloff_radius_pixels` to uint8; 0 = on a boundary; 255 = deep
/// interior).
///
/// Default 1024×1024 resolution at Target B's 11264 WU world extent
/// gives ~11 WU per pixel. Memory cost: 2 MB total (1 MB per channel).
/// Disk cost (uncompressed): ~10-100 KB compressed since archetype IDs
/// have low entropy.
///
/// IDs 0 reserved for unpainted (sample-time fallback to Continental
/// Temperate). IDs 1-6 map to the 6 D.5 catalog archetypes via
/// [`crate::world_archetypes::WorldArchetypeId::to_mask_id`]. IDs 7-255
/// reserved for future expansion.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionalArchetypeMask {
    /// Mask resolution per side (default `DEFAULT_RESOLUTION = 1024`).
    pub resolution: u32,
    /// World extent in WU per side (default `DEFAULT_WORLD_EXTENT_WU = 11264.0`
    /// at Target B's radius-10 chunk-512 configuration).
    pub world_extent_wu: f32,
    /// Falloff distance in pixels (default `DEFAULT_FALLOFF_RADIUS_PIXELS = 32`,
    /// = ~352 WU at default resolution + extent). Pixels closer than this to
    /// a boundary fall in the transition zone; pixels farther are deep interior.
    pub falloff_radius_pixels: u32,
    /// Archetype ID per pixel (uint8). 0 = unpainted; 1-6 = catalog archetypes;
    /// 7-255 = reserved. Length = `resolution × resolution`.
    pub ids: Vec<u8>,
    /// Euclidean distance-to-nearest-other-archetype per pixel (uint8 normalized).
    /// 0 = on a boundary; 255 = deep interior (distance ≥ falloff_radius_pixels).
    /// Computed by [`Self::recompute_falloff`] (F.4.B); F.4.A's paint helpers
    /// leave this at default (255) until distance transform runs.
    /// Length = `resolution × resolution`.
    pub falloff: Vec<u8>,
}

impl RegionalArchetypeMask {
    /// Default resolution: 1024×1024 (per campaign doc §2.4).
    pub const DEFAULT_RESOLUTION: u32 = 1024;
    /// Default world extent: 11264 WU per side (Target B, radius 10 × chunk 512 × 2.2).
    /// Slightly larger than the radius-10 world's 11264 WU per side to absorb
    /// rounding in mask-pixel conversion.
    pub const DEFAULT_WORLD_EXTENT_WU: f32 = 11264.0;
    /// Default falloff radius: 32 pixels = ~352 WU at default resolution + extent.
    pub const DEFAULT_FALLOFF_RADIUS_PIXELS: u32 = 32;

    /// Construct an unpainted mask: all pixels' archetype ID = 0 (unpainted),
    /// all falloff = 255 (deep interior; no transition zones since there are
    /// no archetypes painted).
    pub fn new_unpainted(resolution: u32, world_extent_wu: f32) -> Self {
        let n = (resolution as usize).saturating_mul(resolution as usize);
        Self {
            resolution,
            world_extent_wu,
            falloff_radius_pixels: Self::DEFAULT_FALLOFF_RADIUS_PIXELS,
            ids: vec![0u8; n],
            falloff: vec![255u8; n],
        }
    }

    /// Index a pixel by `(x, z)` in mask coordinates. Returns `None` for
    /// out-of-range coords.
    #[inline]
    pub fn pixel_index(&self, x: u32, z: u32) -> Option<usize> {
        if x < self.resolution && z < self.resolution {
            Some(z as usize * self.resolution as usize + x as usize)
        } else {
            None
        }
    }

    /// Read the archetype ID at a mask pixel. Returns 0 for out-of-range.
    #[inline]
    pub fn id_at(&self, x: u32, z: u32) -> u8 {
        self.pixel_index(x, z)
            .map(|idx| self.ids[idx])
            .unwrap_or(0)
    }

    /// Read the falloff value at a mask pixel. Returns 255 (deep interior)
    /// for out-of-range.
    #[inline]
    pub fn falloff_at(&self, x: u32, z: u32) -> u8 {
        self.pixel_index(x, z)
            .map(|idx| self.falloff[idx])
            .unwrap_or(255)
    }

    /// Phase 1.X-F.4.A test helper: paint a circular region with the given
    /// archetype ID. Builder-pattern; returns `Self` for chaining.
    ///
    /// Uses pixel-coordinate radius (NOT world units). Pixels outside the
    /// mask's bounds are silently skipped. Existing pixels overwritten.
    /// Falloff field NOT recomputed; call [`Self::recompute_falloff`]
    /// after all paint operations to update the distance field.
    pub fn with_painted_circle(
        mut self,
        center_x_px: u32,
        center_y_px: u32,
        radius_px: u32,
        archetype_id: u8,
    ) -> Self {
        let r2 = (radius_px as i64).pow(2);
        let cx = center_x_px as i64;
        let cy = center_y_px as i64;
        let res = self.resolution as i64;
        let min_x = (cx - radius_px as i64).max(0);
        let max_x = (cx + radius_px as i64).min(res - 1);
        let min_y = (cy - radius_px as i64).max(0);
        let max_y = (cy + radius_px as i64).min(res - 1);
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= r2 {
                    let idx = py as usize * self.resolution as usize + px as usize;
                    self.ids[idx] = archetype_id;
                }
            }
        }
        self
    }

    /// Phase 1.X-F.4.A test helper: paint a rectangular region with the given
    /// archetype ID. Builder-pattern; returns `Self` for chaining.
    ///
    /// Pixel coordinates inclusive on min, exclusive on max (standard
    /// half-open convention). Out-of-range coords clamped. Falloff field
    /// NOT recomputed; call [`Self::recompute_falloff`] after all paint
    /// operations.
    pub fn with_painted_rect(
        mut self,
        min_x_px: u32,
        min_y_px: u32,
        max_x_px: u32,
        max_y_px: u32,
        archetype_id: u8,
    ) -> Self {
        let res = self.resolution;
        let max_x = max_x_px.min(res);
        let max_y = max_y_px.min(res);
        let min_x = min_x_px.min(max_x);
        let min_y = min_y_px.min(max_y);
        for py in min_y..max_y {
            for px in min_x..max_x {
                let idx = py as usize * res as usize + px as usize;
                self.ids[idx] = archetype_id;
            }
        }
        self
    }

    /// Phase 1.X-F.4.B + F.4.F test helper: builder-pattern wrapper around
    /// [`Self::recompute_falloff`] returning `Self` for chaining.
    pub fn with_falloff_recomputed(mut self) -> Self {
        self.recompute_falloff();
        self
    }

    // =========================================================================
    // Phase 1.X-F.4.B: save/load file I/O
    // =========================================================================

    /// Save mask to disk as three files: `<base>.ron` (metadata),
    /// `<base>.id.bin` (raw uint8 ID grid), `<base>.falloff.bin` (raw uint8
    /// falloff grid). Files share the path stem `base_path`.
    ///
    /// At default 1024² resolution: ~1 MB per binary file + ~80 bytes RON.
    pub fn save_to_files(&self, base_path: &Path) -> Result<(), MaskIoError> {
        let metadata = RegionalArchetypeMaskMetadata::from_mask(self);

        let ron_path = path_with_extension(base_path, "ron");
        let id_path = path_with_extension(base_path, "id.bin");
        let falloff_path = path_with_extension(base_path, "falloff.bin");

        let ron_string = ron::ser::to_string_pretty(
            &metadata,
            ron::ser::PrettyConfig::default(),
        )
        .map_err(|e| MaskIoError::Ron(e.to_string()))?;
        fs::write(&ron_path, ron_string)?;
        fs::write(&id_path, &self.ids)?;
        fs::write(&falloff_path, &self.falloff)?;
        Ok(())
    }

    /// Load mask from the three companion files written by
    /// [`Self::save_to_files`]. Validates RON metadata's `format_version`
    /// (only 1 supported in this build) and binary file sizes against
    /// `metadata.resolution²`.
    pub fn load_from_files(base_path: &Path) -> Result<Self, MaskIoError> {
        let ron_path = path_with_extension(base_path, "ron");
        let id_path = path_with_extension(base_path, "id.bin");
        let falloff_path = path_with_extension(base_path, "falloff.bin");

        let ron_string = fs::read_to_string(&ron_path)?;
        let metadata: RegionalArchetypeMaskMetadata = ron::from_str(&ron_string)?;

        if metadata.format_version != 1 {
            return Err(MaskIoError::UnsupportedVersion(metadata.format_version));
        }

        let ids = fs::read(&id_path)?;
        let falloff = fs::read(&falloff_path)?;
        let expected =
            (metadata.resolution as usize).saturating_mul(metadata.resolution as usize);
        if ids.len() != expected {
            return Err(MaskIoError::Mismatch {
                expected,
                actual: ids.len(),
            });
        }
        if falloff.len() != expected {
            return Err(MaskIoError::Mismatch {
                expected,
                actual: falloff.len(),
            });
        }

        Ok(Self {
            resolution: metadata.resolution,
            world_extent_wu: metadata.world_extent_wu,
            falloff_radius_pixels: metadata.falloff_radius_pixels,
            ids,
            falloff,
        })
    }

    // =========================================================================
    // Phase 1.X-F.4.B: Euclidean distance transform (falloff field)
    // =========================================================================

    /// Compute the falloff distance field from the current `ids` field.
    ///
    /// For each pixel, computes the Euclidean distance to the nearest pixel
    /// with a different (non-zero) archetype ID, normalized by
    /// `falloff_radius_pixels` to uint8:
    ///
    /// - Pixels deep inside an archetype region (distance >
    ///   `falloff_radius_pixels`) → 255.
    /// - Pixels on archetype boundaries → 0.
    /// - Pixels in unpainted (ID = 0) regions → 255 (no transitions in
    ///   unpainted areas; sampler treats unpainted as Continental Temperate
    ///   solo per F.4.C invariant).
    /// - Pixels at distance d ∈ `[0, falloff_radius_pixels]` from a boundary
    ///   → `(255 * d / falloff_radius_pixels).round() as u8`.
    ///
    /// Algorithm: two-pass chamfer distance transform (Borgefors 1986).
    /// O(n) for n = resolution². Approximates Euclidean distance with
    /// max relative error ~5%; uint8 quantization absorbs the error.
    /// Sufficient for archetype-region authoring at the 32-pixel default
    /// falloff radius.
    pub fn recompute_falloff(&mut self) {
        let n = (self.resolution as usize).saturating_mul(self.resolution as usize);
        if n == 0 || self.ids.is_empty() {
            return;
        }
        let res = self.resolution as i32;
        // Sentinel for "unreached". 1e6 is large enough for any plausible
        // mask resolution; at 1024 res the max possible distance is ~1450.
        let inf: f32 = 1.0e6;

        // Initialize distance field. For each pixel, distance = 0 if the
        // pixel is on an archetype boundary (any 4-neighbor has a
        // different non-zero ID, OR the pixel itself is unpainted with a
        // painted neighbor). Otherwise distance = inf.
        let mut dist: Vec<f32> = vec![inf; n];
        for z in 0..res {
            for x in 0..res {
                let idx = (z * res + x) as usize;
                let id = self.ids[idx];
                let mut is_boundary = false;
                for &(dx, dz) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let nx = x + dx;
                    let nz = z + dz;
                    if nx >= 0 && nx < res && nz >= 0 && nz < res {
                        let nidx = (nz * res + nx) as usize;
                        let nid = self.ids[nidx];
                        // Boundary between two different archetype IDs (one or both can be 0).
                        if nid != id && (nid != 0 || id != 0) {
                            is_boundary = true;
                            break;
                        }
                    }
                }
                if is_boundary {
                    dist[idx] = 0.0;
                }
            }
        }

        // Forward pass (top-left → bottom-right). Standard 3×3 chamfer
        // distance with 1.0 / sqrt(2) ≈ 1.41421356 weights.
        const W_AXIS: f32 = 1.0;
        const W_DIAG: f32 = std::f32::consts::SQRT_2;
        for z in 0..res {
            for x in 0..res {
                let idx = (z * res + x) as usize;
                let mut d = dist[idx];
                // (-1, -1)
                if x > 0 && z > 0 {
                    d = d.min(dist[((z - 1) * res + (x - 1)) as usize] + W_DIAG);
                }
                // (0, -1)
                if z > 0 {
                    d = d.min(dist[((z - 1) * res + x) as usize] + W_AXIS);
                }
                // (1, -1)
                if x < res - 1 && z > 0 {
                    d = d.min(dist[((z - 1) * res + (x + 1)) as usize] + W_DIAG);
                }
                // (-1, 0)
                if x > 0 {
                    d = d.min(dist[(z * res + (x - 1)) as usize] + W_AXIS);
                }
                dist[idx] = d;
            }
        }

        // Backward pass (bottom-right → top-left).
        for z in (0..res).rev() {
            for x in (0..res).rev() {
                let idx = (z * res + x) as usize;
                let mut d = dist[idx];
                if x < res - 1 {
                    d = d.min(dist[(z * res + (x + 1)) as usize] + W_AXIS);
                }
                if z < res - 1 && x < res - 1 {
                    d = d.min(dist[((z + 1) * res + (x + 1)) as usize] + W_DIAG);
                }
                if z < res - 1 {
                    d = d.min(dist[((z + 1) * res + x) as usize] + W_AXIS);
                }
                if z < res - 1 && x > 0 {
                    d = d.min(dist[((z + 1) * res + (x - 1)) as usize] + W_DIAG);
                }
                dist[idx] = d;
            }
        }

        // Quantize distance to uint8, normalized by falloff_radius_pixels.
        // Unpainted (ID = 0) pixels stay at 255 regardless of distance —
        // sampler treats unpainted as Continental Temperate solo.
        let radius = self.falloff_radius_pixels.max(1) as f32;
        for idx in 0..n {
            let id = self.ids[idx];
            if id == 0 {
                self.falloff[idx] = 255;
                continue;
            }
            let d = dist[idx];
            let normalized = (d / radius).clamp(0.0, 1.0);
            self.falloff[idx] = (normalized * 255.0).round() as u8;
        }
    }
}

/// Helper: append an extension to a path stem. `base.ext` form.
fn path_with_extension(base: &Path, ext: &str) -> std::path::PathBuf {
    let mut p = base.to_path_buf();
    let new_name = match base.file_name() {
        Some(name) => format!("{}.{}", name.to_string_lossy(), ext),
        None => ext.to_string(),
    };
    p.set_file_name(new_name);
    p
}

// =============================================================================
// Phase 1.X-F.4.C: RegionalArchetypeBlend neighborhood-scan sampler
// =============================================================================

use crate::climate::ClimateSample;
use crate::spline_types::{BootstrapParams, BootstrapSplineSet};
use crate::world_archetypes::WorldArchetypeId;

/// Phase 1.X-F.4.D: blend per-archetype `BootstrapParams` via spline-output
/// blending per campaign doc §2.5. Each contributing archetype's
/// `BootstrapSplineSet` evaluates against the same climate sample
/// independently; weights from mask falloff combine spline outputs into
/// per-vertex blended `BootstrapParams`.
///
/// Bootstrap noise pipeline runs ONCE per vertex with the blended params
/// (per §2.5; not N times per vertex per archetype). This composes with
/// F.3.B's `sample_height_with_params(blended_params, multiplier, x, z)`
/// at the existing per-vertex call site.
///
/// **f32/f64 discipline (per F.3 deviation 2)**:
/// - `mountains_amplitude`, `continental_scale`, `base_elevation_amplitude`:
///   f32 weighted accumulation.
/// - `mountains_scale`: f64 weighted accumulation (`weight as f64` cast
///   before multiply) to preserve byte-identity for noise-coordinate
///   multipliers.
///
/// **Convex combination invariants** (verified by tests):
/// - Single-contributor case (`contributors.len() == 1`, weight = 1.0):
///   output equals that archetype's `BootstrapParams` directly.
/// - Identical-archetype case (e.g., `[(id, 0.5), (id, 0.5)]`): output
///   equals that archetype's params within f32/f64 epsilon.
/// - General convex case: blended params lie within convex hull of
///   contributing archetypes' params (no overshoot).
/// - Empty contributor list (degenerate; shouldn't occur per F.4.C
///   invariants): returns all-zero `BootstrapParams`. F.4.E guards
///   against passing empty lists to this function.
pub fn blend_bootstrap_params(
    contributors: &BlendContributors,
    archetype_splines: &dyn Fn(WorldArchetypeId) -> BootstrapSplineSet,
    sample: &ClimateSample,
) -> BootstrapParams {
    let mut blended = BootstrapParams {
        mountains_amplitude: 0.0,
        mountains_scale: 0.0,
        continental_scale: 0.0,
        base_elevation_amplitude: 0.0,
    };
    for (id, weight) in contributors.iter() {
        let archetype_params = archetype_splines(id).evaluate(sample);
        blended.mountains_amplitude += weight * archetype_params.mountains_amplitude;
        // F.3 deviation 2: mountains_scale is f64; preserve precision via
        // f64 weighted accumulation.
        blended.mountains_scale += (weight as f64) * archetype_params.mountains_scale;
        blended.continental_scale += weight * archetype_params.continental_scale;
        blended.base_elevation_amplitude +=
            weight * archetype_params.base_elevation_amplitude;
    }
    blended
}

/// Phase 1.X-F.4.C: small fixed-size container for up to 4
/// `(WorldArchetypeId, weight)` blend contributors. Avoids per-vertex Vec
/// allocation in the sampler. Returned by [`RegionalArchetypeBlend::sample_at`].
///
/// Iteration via `iter()` exposes `(WorldArchetypeId, f32)` tuples; weights
/// sum to 1.0 ± f32 epsilon per F.4.C invariant.
#[derive(Debug, Clone, Copy)]
pub struct BlendContributors {
    items: [(WorldArchetypeId, f32); 4],
    len: u8,
}

impl BlendContributors {
    /// Construct with a single contributor at weight 1.0. Used by sampler
    /// fast paths (unpainted mask, deep interior).
    pub fn single(id: WorldArchetypeId) -> Self {
        Self {
            items: [(id, 1.0); 4],
            len: 1,
        }
    }

    /// Number of active contributors (1-4).
    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate active `(archetype, weight)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (WorldArchetypeId, f32)> + '_ {
        self.items[..self.len as usize].iter().copied()
    }

    /// Push a contributor. Panics in debug if `len >= 4`; in release,
    /// silently drops the new contributor (sampler caps at 4 strongest).
    fn push(&mut self, id: WorldArchetypeId, weight: f32) {
        debug_assert!(self.len < 4, "BlendContributors capacity exceeded");
        if self.len < 4 {
            self.items[self.len as usize] = (id, weight);
            self.len += 1;
        }
    }

    /// Normalize weights to sum to 1.0. No-op if empty.
    fn normalize(&mut self) {
        let total: f32 = self.iter().map(|(_, w)| w).sum();
        if total > 0.0 {
            for i in 0..self.len as usize {
                self.items[i].1 /= total;
            }
        }
    }
}

/// Phase 1.X-F.4.C: per-vertex sampler over a [`RegionalArchetypeMask`].
/// Returns blended `(WorldArchetypeId, weight)` contributors at world
/// coordinates via three paths:
///
/// 1. **Unpainted fast path**: pixel ID = 0 → single
///    `(ContinentalTemperate, 1.0)` contributor.
/// 2. **Deep interior fast path**: pixel falloff ≥ 200 (78% of max) →
///    single `(WorldArchetypeId::from_mask_id(id).unwrap_or(ContinentalTemperate), 1.0)`.
/// 3. **Transition zone slow path**: scan a `falloff_radius_pixels`-radius
///    neighborhood; for each distinct archetype ID found, compute its
///    weight as `clamp(1.0 - distance_to_nearest_pixel_of_that_id /
///    falloff_radius_pixels, 0.0, 1.0)`. Normalize weights to sum to 1.0.
///    Return up to 4 strongest contributors (ties broken by archetype ID
///    for determinism); prune contributions < 0.05 weight.
///
/// Borrows the mask for its lifetime. Created once per chunk-generation
/// pass; sampled per vertex with no allocation (returns
/// [`BlendContributors`] on the stack).
pub struct RegionalArchetypeBlend<'a> {
    mask: &'a RegionalArchetypeMask,
}

impl<'a> RegionalArchetypeBlend<'a> {
    /// Threshold for "deep interior" fast path. Pixels with falloff above
    /// this are far enough from any boundary that single-contributor is
    /// safe; saves the neighborhood-scan cost.
    pub const DEEP_INTERIOR_THRESHOLD: u8 = 200;

    /// Pruning threshold: contributions with weight below this are dropped
    /// before normalization.
    pub const PRUNE_WEIGHT_THRESHOLD: f32 = 0.05;

    pub fn new(mask: &'a RegionalArchetypeMask) -> Self {
        Self { mask }
    }

    /// Sample at world coordinates. Returns up to 4 contributors, weights
    /// summing to 1.0 ± f32 epsilon.
    pub fn sample_at(&self, world_x: f32, world_z: f32) -> BlendContributors {
        // Convert world coords to mask pixel coords.
        let half_extent = self.mask.world_extent_wu * 0.5;
        let res = self.mask.resolution as f32;
        let px_f = ((world_x + half_extent) / self.mask.world_extent_wu * res)
            .clamp(0.0, res - 1.0);
        let pz_f = ((world_z + half_extent) / self.mask.world_extent_wu * res)
            .clamp(0.0, res - 1.0);
        let px = px_f as u32;
        let pz = pz_f as u32;

        let center_id = self.mask.id_at(px, pz);

        // Unpainted fast path.
        if center_id == 0 {
            return BlendContributors::single(WorldArchetypeId::ContinentalTemperate);
        }

        let center_falloff = self.mask.falloff_at(px, pz);

        // Deep interior fast path.
        if center_falloff >= Self::DEEP_INTERIOR_THRESHOLD {
            let id = WorldArchetypeId::from_mask_id(center_id)
                .unwrap_or(WorldArchetypeId::ContinentalTemperate);
            return BlendContributors::single(id);
        }

        // Transition zone slow path: neighborhood scan.
        let radius = self.mask.falloff_radius_pixels as i32;
        let radius_f = radius as f32;
        let res_i = self.mask.resolution as i32;
        let cx = px as i32;
        let cz = pz as i32;

        // For each distinct archetype ID encountered, track the minimum
        // distance to any pixel of that ID.
        let mut min_distances: [(u8, f32); 8] = [(0, f32::INFINITY); 8];
        let mut n_distinct: usize = 0;

        let r2 = (radius_f * radius_f) as f32;
        for dz in -radius..=radius {
            let nz = cz + dz;
            if nz < 0 || nz >= res_i {
                continue;
            }
            for dx in -radius..=radius {
                let nx = cx + dx;
                if nx < 0 || nx >= res_i {
                    continue;
                }
                let dist_sq = (dx * dx + dz * dz) as f32;
                if dist_sq > r2 {
                    continue;
                }
                let nid = self.mask.ids[(nz * res_i + nx) as usize];
                if nid == 0 {
                    continue;
                }
                let dist = dist_sq.sqrt();
                // Find or insert this ID.
                let mut found = false;
                for slot in &mut min_distances[..n_distinct] {
                    if slot.0 == nid {
                        if dist < slot.1 {
                            slot.1 = dist;
                        }
                        found = true;
                        break;
                    }
                }
                if !found && n_distinct < 8 {
                    min_distances[n_distinct] = (nid, dist);
                    n_distinct += 1;
                }
            }
        }

        // Compute weights from min distances. Weight = clamp(1 - dist / radius, 0, 1).
        // Skip IDs that don't map to a known WorldArchetypeId (reserved/unassigned).
        let mut weighted: [(WorldArchetypeId, f32); 8] =
            [(WorldArchetypeId::ContinentalTemperate, 0.0); 8];
        let mut n_weighted: usize = 0;
        for i in 0..n_distinct {
            let (id, dist) = min_distances[i];
            let weight = (1.0 - dist / radius_f).clamp(0.0, 1.0);
            if weight < Self::PRUNE_WEIGHT_THRESHOLD {
                continue;
            }
            if let Some(archetype) = WorldArchetypeId::from_mask_id(id) {
                weighted[n_weighted] = (archetype, weight);
                n_weighted += 1;
            }
        }

        // Defensive fallback: if no contributors survived (e.g., all IDs
        // mapped to None, or all weights < threshold), use the center
        // pixel's archetype as a single contributor.
        if n_weighted == 0 {
            let id = WorldArchetypeId::from_mask_id(center_id)
                .unwrap_or(WorldArchetypeId::ContinentalTemperate);
            return BlendContributors::single(id);
        }

        // Sort by weight desc; ties broken by archetype ID for determinism.
        weighted[..n_weighted].sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.to_mask_id().cmp(&b.0.to_mask_id()))
        });

        // Take up to 4 strongest.
        let take = n_weighted.min(4);
        let mut contributors = BlendContributors {
            items: [(WorldArchetypeId::ContinentalTemperate, 0.0); 4],
            len: 0,
        };
        for i in 0..take {
            contributors.push(weighted[i].0, weighted[i].1);
        }
        contributors.normalize();
        contributors
    }
}

/// Phase 1.X-F.4.A: serde-serializable metadata for [`RegionalArchetypeMask`]
/// persistence. Pairs with raw uint8 binary files (`<base>.id.bin` and
/// `<base>.falloff.bin`) for full mask state.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RegionalArchetypeMaskMetadata {
    pub resolution: u32,
    pub world_extent_wu: f32,
    pub falloff_radius_pixels: u32,
    /// Format version. F.4.A ships version 1. Future format changes
    /// (multi-channel masks, different falloff encodings) bump this with
    /// explicit migration logic in [`RegionalArchetypeMask::load_from_files`].
    pub format_version: u32,
}

impl Default for RegionalArchetypeMaskMetadata {
    fn default() -> Self {
        Self {
            resolution: RegionalArchetypeMask::DEFAULT_RESOLUTION,
            world_extent_wu: RegionalArchetypeMask::DEFAULT_WORLD_EXTENT_WU,
            falloff_radius_pixels: RegionalArchetypeMask::DEFAULT_FALLOFF_RADIUS_PIXELS,
            format_version: 1,
        }
    }
}

impl RegionalArchetypeMaskMetadata {
    /// Construct metadata matching a given mask's parameters.
    pub fn from_mask(mask: &RegionalArchetypeMask) -> Self {
        Self {
            resolution: mask.resolution,
            world_extent_wu: mask.world_extent_wu,
            falloff_radius_pixels: mask.falloff_radius_pixels,
            format_version: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_archetypes::WorldArchetypeId;

    /// Unpainted constructor produces all-zero IDs and all-255 falloff.
    /// 1024×1024 = 1048576 bytes per channel.
    #[test]
    fn regional_mask_unpainted_default_state() {
        let mask = RegionalArchetypeMask::new_unpainted(
            RegionalArchetypeMask::DEFAULT_RESOLUTION,
            RegionalArchetypeMask::DEFAULT_WORLD_EXTENT_WU,
        );
        assert_eq!(mask.resolution, 1024);
        assert_eq!(mask.world_extent_wu, 11264.0);
        assert_eq!(mask.falloff_radius_pixels, 32);
        assert_eq!(mask.ids.len(), 1024 * 1024);
        assert_eq!(mask.falloff.len(), 1024 * 1024);
        assert!(mask.ids.iter().all(|&v| v == 0));
        assert!(mask.falloff.iter().all(|&v| v == 255));
    }

    /// `with_painted_circle` writes 1 to all pixels within radius 8 of
    /// (32, 32) in a 64×64 mask; leaves pixels outside as 0.
    #[test]
    fn regional_mask_painted_circle_writes_pixels() {
        let mask = RegionalArchetypeMask::new_unpainted(64, 100.0)
            .with_painted_circle(32, 32, 8, 1);

        // Center pixel painted.
        assert_eq!(mask.id_at(32, 32), 1);
        // Pixels within radius 8.
        assert_eq!(mask.id_at(40, 32), 1); // dist = 8
        assert_eq!(mask.id_at(32, 40), 1); // dist = 8
        // Pixels just outside radius 8 (dist² = 81 > 64).
        assert_eq!(mask.id_at(41, 32), 0); // dist = 9
        // Far pixel.
        assert_eq!(mask.id_at(0, 0), 0);
        assert_eq!(mask.id_at(63, 63), 0);
    }

    /// `with_painted_rect` writes archetype ID to pixels in `[min, max)`.
    #[test]
    fn regional_mask_painted_rect_writes_pixels() {
        let mask = RegionalArchetypeMask::new_unpainted(64, 100.0)
            .with_painted_rect(10, 10, 30, 30, 5);

        // Inside rect.
        assert_eq!(mask.id_at(10, 10), 5);
        assert_eq!(mask.id_at(15, 15), 5);
        assert_eq!(mask.id_at(29, 29), 5);
        // On exclusive max boundary.
        assert_eq!(mask.id_at(30, 30), 0);
        // Outside rect.
        assert_eq!(mask.id_at(9, 10), 0);
        assert_eq!(mask.id_at(10, 9), 0);
        assert_eq!(mask.id_at(40, 40), 0);
    }

    /// `RegionalArchetypeMaskMetadata` roundtrips through RON
    /// byte-identically.
    #[test]
    fn regional_mask_metadata_roundtrip_via_ron() {
        let original = RegionalArchetypeMaskMetadata {
            resolution: 512,
            world_extent_wu: 5632.0,
            falloff_radius_pixels: 16,
            format_version: 1,
        };
        let ron_string = ron::to_string(&original).expect("RON serialize");
        let roundtripped: RegionalArchetypeMaskMetadata =
            ron::from_str(&ron_string).expect("RON deserialize");
        assert_eq!(original, roundtripped);
    }

    /// Default `RegionalArchetypeMaskMetadata` ships format_version = 1
    /// per F.4.A invariant.
    #[test]
    fn regional_mask_metadata_format_version_is_one() {
        let meta = RegionalArchetypeMaskMetadata::default();
        assert_eq!(meta.format_version, 1);
    }

    /// Each of the 6 D.5 catalog archetypes roundtrips through
    /// `to_mask_id` → `from_mask_id` byte-identically.
    #[test]
    fn world_archetype_id_to_mask_id_roundtrip() {
        for &id in WorldArchetypeId::all() {
            let mask_id = id.to_mask_id();
            let recovered = WorldArchetypeId::from_mask_id(mask_id);
            assert_eq!(
                recovered,
                Some(id),
                "{:?} mask_id={} did not roundtrip",
                id,
                mask_id
            );
        }
    }

    /// `WorldArchetypeId::from_mask_id(0)` returns `None` (0 reserved for
    /// unpainted; sample-time fallback to Continental Temperate handled
    /// by sampler, not by enum lookup).
    #[test]
    fn world_archetype_id_unpainted_is_none() {
        assert!(WorldArchetypeId::from_mask_id(0).is_none());
    }

    /// `WorldArchetypeId::from_mask_id` for a reserved-but-unassigned ID
    /// returns `None`.
    #[test]
    fn world_archetype_id_unknown_is_none() {
        assert!(WorldArchetypeId::from_mask_id(200).is_none());
        assert!(WorldArchetypeId::from_mask_id(255).is_none());
    }

    // =========================================================================
    // Phase 1.X-F.4.B: save/load + Euclidean distance transform tests
    // =========================================================================

    /// Save → load roundtrip on an unpainted mask preserves all bytes.
    #[test]
    fn regional_mask_save_load_roundtrip_unpainted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("unpainted_test");
        let original = RegionalArchetypeMask::new_unpainted(64, 100.0);
        original.save_to_files(&base).expect("save");
        let loaded = RegionalArchetypeMask::load_from_files(&base).expect("load");
        assert_eq!(original, loaded);
    }

    /// Save → load roundtrip on a painted mask (with falloff recomputed)
    /// preserves all bytes.
    #[test]
    fn regional_mask_save_load_roundtrip_painted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("painted_test");
        let original = RegionalArchetypeMask::new_unpainted(64, 100.0)
            .with_painted_circle(32, 32, 16, 1)
            .with_painted_rect(0, 0, 16, 16, 2)
            .with_falloff_recomputed();
        original.save_to_files(&base).expect("save");
        let loaded = RegionalArchetypeMask::load_from_files(&base).expect("load");
        assert_eq!(original, loaded);
    }

    /// Loading a RON file with `format_version: 2` returns
    /// `MaskIoError::UnsupportedVersion(2)`.
    #[test]
    fn regional_mask_load_rejects_unsupported_version() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("unsupported_test");
        let ron_path = path_with_extension(&base, "ron");
        let id_path = path_with_extension(&base, "id.bin");
        let falloff_path = path_with_extension(&base, "falloff.bin");

        let bad_metadata = RegionalArchetypeMaskMetadata {
            resolution: 64,
            world_extent_wu: 100.0,
            falloff_radius_pixels: 8,
            format_version: 2,
        };
        let ron_string = ron::ser::to_string_pretty(
            &bad_metadata,
            ron::ser::PrettyConfig::default(),
        )
        .unwrap();
        std::fs::write(&ron_path, ron_string).unwrap();
        std::fs::write(&id_path, vec![0u8; 64 * 64]).unwrap();
        std::fs::write(&falloff_path, vec![255u8; 64 * 64]).unwrap();

        let err = RegionalArchetypeMask::load_from_files(&base)
            .expect_err("expected UnsupportedVersion");
        assert!(matches!(err, MaskIoError::UnsupportedVersion(2)));
    }

    /// Loading a mask whose binary file size doesn't match the RON's
    /// declared resolution returns `MaskIoError::Mismatch`.
    #[test]
    fn regional_mask_load_rejects_size_mismatch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("mismatch_test");
        let ron_path = path_with_extension(&base, "ron");
        let id_path = path_with_extension(&base, "id.bin");
        let falloff_path = path_with_extension(&base, "falloff.bin");

        let metadata = RegionalArchetypeMaskMetadata {
            resolution: 1024,
            world_extent_wu: 11264.0,
            falloff_radius_pixels: 32,
            format_version: 1,
        };
        let ron_string = ron::ser::to_string_pretty(
            &metadata,
            ron::ser::PrettyConfig::default(),
        )
        .unwrap();
        std::fs::write(&ron_path, ron_string).unwrap();
        // Write only 64×64 bytes instead of 1024² claimed by metadata.
        std::fs::write(&id_path, vec![0u8; 64 * 64]).unwrap();
        std::fs::write(&falloff_path, vec![255u8; 64 * 64]).unwrap();

        let err = RegionalArchetypeMask::load_from_files(&base)
            .expect_err("expected Mismatch");
        assert!(matches!(err, MaskIoError::Mismatch { expected: 1048576, actual: 4096 }));
    }

    /// Unpainted mask (all IDs = 0) → recompute_falloff → all 255 (no
    /// boundaries; sampler treats as Continental Temperate solo).
    #[test]
    fn distance_transform_unpainted_is_all_max() {
        let mut mask = RegionalArchetypeMask::new_unpainted(32, 100.0);
        mask.recompute_falloff();
        assert!(mask.falloff.iter().all(|&v| v == 255));
    }

    /// Painted-circle interior at distance > falloff_radius from boundary
    /// has falloff = 255 (deep interior).
    #[test]
    fn distance_transform_single_archetype_interior_is_max() {
        let mut mask = RegionalArchetypeMask::new_unpainted(256, 100.0)
            .with_painted_circle(128, 128, 80, 1);
        // falloff_radius_pixels default = 32; interior pixels at distance
        // > 32 from any boundary should hit 255.
        mask.recompute_falloff();
        // Center pixel is ~80 pixels from the nearest boundary → 255.
        assert_eq!(mask.falloff_at(128, 128), 255);
    }

    /// Painted boundary pixels (distance 0 from a different ID) have
    /// falloff = 0.
    #[test]
    fn distance_transform_boundary_is_zero() {
        let mut mask = RegionalArchetypeMask::new_unpainted(64, 100.0)
            .with_painted_rect(0, 0, 32, 64, 1)
            .with_painted_rect(32, 0, 64, 64, 2);
        mask.recompute_falloff();
        // The boundary between the two rects is at x=31/x=32. Pixels at
        // x=31 are painted ID=1 with a different (ID=2) neighbor at
        // x=32 → boundary → falloff = 0.
        assert_eq!(mask.falloff_at(31, 32), 0);
        assert_eq!(mask.falloff_at(32, 32), 0);
    }

    /// Pixels at distance ≈ falloff_radius/2 from a boundary have falloff
    /// ≈ 128 (middle of the [0, 255] range).
    #[test]
    fn distance_transform_within_falloff_radius_is_intermediate() {
        let mut mask = RegionalArchetypeMask::new_unpainted(128, 100.0);
        mask.falloff_radius_pixels = 32;
        // Paint two adjacent rects sharing a boundary at x=64.
        let mask = mask
            .with_painted_rect(0, 0, 64, 128, 1)
            .with_painted_rect(64, 0, 128, 128, 2);
        let mut mask = mask;
        mask.recompute_falloff();
        // At x=48 (16 pixels left of boundary x=64), distance ≈ 16.
        // Normalized: 16/32 = 0.5. Quantized: ~128 ± chamfer error.
        let v = mask.falloff_at(48, 64);
        assert!(
            (110..=146).contains(&v),
            "falloff at distance ~16/32 should be ~128; got {}",
            v
        );
    }

    /// Two adjacent archetype rects' shared boundary has falloff = 0;
    /// deep interior of each has falloff = 255 (when far enough).
    #[test]
    fn distance_transform_two_archetypes_meet_at_boundary() {
        let mut mask = RegionalArchetypeMask::new_unpainted(256, 100.0);
        mask.falloff_radius_pixels = 16;
        let mut mask = mask
            .with_painted_rect(0, 0, 128, 256, 1)
            .with_painted_rect(128, 0, 256, 256, 2);
        mask.recompute_falloff();
        // Boundary at x=127/x=128.
        assert_eq!(mask.falloff_at(127, 128), 0);
        assert_eq!(mask.falloff_at(128, 128), 0);
        // Deep interior of left rect: x=64 is 64 pixels from boundary;
        // 64 > falloff_radius_pixels=16 → falloff = 255.
        assert_eq!(mask.falloff_at(64, 128), 255);
        // Deep interior of right rect: x=192 is 64 pixels from boundary.
        assert_eq!(mask.falloff_at(192, 128), 255);
    }

    // =========================================================================
    // Phase 1.X-F.4.C: RegionalArchetypeBlend sampler tests
    // =========================================================================

    /// Sample at any world position in an unpainted mask returns single
    /// `(ContinentalTemperate, 1.0)` contributor.
    #[test]
    fn blend_unpainted_sample_returns_continental_temperate() {
        let mask = RegionalArchetypeMask::new_unpainted(64, 100.0);
        let blend = RegionalArchetypeBlend::new(&mask);
        for &(x, z) in &[(0.0_f32, 0.0_f32), (-40.0, 30.0), (49.0, -49.0)] {
            let c = blend.sample_at(x, z);
            assert_eq!(c.len(), 1);
            let v: Vec<_> = c.iter().collect();
            assert_eq!(v[0].0, WorldArchetypeId::ContinentalTemperate);
            assert!((v[0].1 - 1.0).abs() < 1e-6);
        }
    }

    /// Sample at the deep interior of a painted region returns a single
    /// contributor matching that region's archetype.
    #[test]
    fn blend_deep_interior_returns_single_contributor() {
        let mut mask = RegionalArchetypeMask::new_unpainted(256, 100.0)
            .with_painted_rect(64, 64, 192, 192, WorldArchetypeId::EquatorialTropical.to_mask_id());
        mask.recompute_falloff();
        let blend = RegionalArchetypeBlend::new(&mask);
        // Center of the rect at world (0, 0) → mask pixel (128, 128) → deep interior.
        let c = blend.sample_at(0.0, 0.0);
        assert_eq!(c.len(), 1);
        let v: Vec<_> = c.iter().collect();
        assert_eq!(v[0].0, WorldArchetypeId::EquatorialTropical);
    }

    /// Sample in the transition zone between two painted archetypes returns
    /// 2 contributors with non-trivial weights.
    #[test]
    fn blend_transition_zone_returns_multiple_contributors() {
        let mut mask = RegionalArchetypeMask::new_unpainted(256, 100.0);
        mask.falloff_radius_pixels = 32;
        let mut mask = mask
            .with_painted_rect(0, 0, 128, 256, WorldArchetypeId::BorealSubarctic.to_mask_id())
            .with_painted_rect(128, 0, 256, 256, WorldArchetypeId::Desert.to_mask_id());
        mask.recompute_falloff();
        let blend = RegionalArchetypeBlend::new(&mask);
        // World x=0 maps to mask px=128 (boundary). Falloff is small there
        // (transition zone). Sample at world x=0 z=0.
        let c = blend.sample_at(0.0, 0.0);
        assert!(c.len() >= 2, "expected >=2 contributors at transition; got {}", c.len());
        let total: f32 = c.iter().map(|(_, w)| w).sum();
        assert!((total - 1.0).abs() < 1e-5);
    }

    /// Across 100 sample positions in a 3-archetype painted mask, all
    /// returned weight lists sum to 1.0 ± f32 epsilon.
    #[test]
    fn blend_weights_sum_to_one() {
        let mut mask = RegionalArchetypeMask::new_unpainted(128, 100.0)
            .with_painted_rect(0, 0, 64, 128, WorldArchetypeId::ContinentalTemperate.to_mask_id())
            .with_painted_rect(64, 0, 96, 128, WorldArchetypeId::EquatorialTropical.to_mask_id())
            .with_painted_rect(96, 0, 128, 128, WorldArchetypeId::BorealSubarctic.to_mask_id());
        mask.recompute_falloff();
        let blend = RegionalArchetypeBlend::new(&mask);
        for i in 0..10 {
            for j in 0..10 {
                let x = -49.0 + i as f32 * 9.8;
                let z = -49.0 + j as f32 * 9.8;
                let c = blend.sample_at(x, z);
                let total: f32 = c.iter().map(|(_, w)| w).sum();
                assert!(
                    (total - 1.0).abs() < 1e-5,
                    "weights sum drift at ({}, {}): {} (n={})",
                    x,
                    z,
                    total,
                    c.len()
                );
            }
        }
    }

    /// Returned contributor list never exceeds 4 entries.
    #[test]
    fn blend_returns_at_most_four_contributors() {
        // Construct a 5-archetype world meeting at a near-corner.
        let mut mask = RegionalArchetypeMask::new_unpainted(64, 100.0);
        mask.falloff_radius_pixels = 32;
        let mut mask = mask
            .with_painted_rect(0, 0, 32, 32, 1)
            .with_painted_rect(32, 0, 64, 32, 2)
            .with_painted_rect(0, 32, 32, 64, 3)
            .with_painted_rect(32, 32, 48, 64, 4)
            .with_painted_rect(48, 32, 64, 64, 5);
        mask.recompute_falloff();
        let blend = RegionalArchetypeBlend::new(&mask);
        let c = blend.sample_at(0.0, 0.0);
        assert!(c.len() <= 4);
    }

    /// Sample twice at same position; byte-identical results.
    #[test]
    fn blend_determinism_same_inputs_same_outputs() {
        let mut mask = RegionalArchetypeMask::new_unpainted(128, 100.0)
            .with_painted_rect(0, 0, 64, 128, 1)
            .with_painted_rect(64, 0, 128, 128, 2);
        mask.recompute_falloff();
        let blend = RegionalArchetypeBlend::new(&mask);
        let a = blend.sample_at(-12.5, 30.0);
        let b = blend.sample_at(-12.5, 30.0);
        assert_eq!(a.len(), b.len());
        for (av, bv) in a.iter().zip(b.iter()) {
            assert_eq!(av.0, bv.0);
            assert_eq!(av.1.to_bits(), bv.1.to_bits());
        }
    }

    /// `sample_at(0.0, 0.0)` in unpainted mask returns single CT contributor.
    #[test]
    fn blend_at_world_origin_with_continental_temperate_unpainted_returns_ct() {
        let mask = RegionalArchetypeMask::new_unpainted(64, 100.0);
        let blend = RegionalArchetypeBlend::new(&mask);
        let c = blend.sample_at(0.0, 0.0);
        assert_eq!(c.len(), 1);
        let v: Vec<_> = c.iter().collect();
        assert_eq!(v[0].0, WorldArchetypeId::ContinentalTemperate);
    }

    /// Same world coordinate sampled (corresponding to chunk shared edges)
    /// produces identical contributor lists. F.4 inherits F.3-phase-3's
    /// world-coord determinism contract.
    #[test]
    fn blend_chunk_edge_continuity() {
        let mut mask = RegionalArchetypeMask::new_unpainted(128, 100.0)
            .with_painted_rect(32, 32, 96, 96, WorldArchetypeId::EquatorialTropical.to_mask_id());
        mask.recompute_falloff();
        let blend = RegionalArchetypeBlend::new(&mask);
        // Two chunks sharing edge at world x=0; both sample at the same x=0.
        let chunk_a_edge = blend.sample_at(0.0, 5.0);
        let chunk_b_edge = blend.sample_at(0.0, 5.0);
        assert_eq!(chunk_a_edge.len(), chunk_b_edge.len());
        for (a, b) in chunk_a_edge.iter().zip(chunk_b_edge.iter()) {
            assert_eq!(a.0, b.0);
            assert_eq!(a.1.to_bits(), b.1.to_bits());
        }
    }

    /// `BlendContributors::single` constructs a 1-contributor instance with
    /// weight 1.0.
    #[test]
    fn blend_contributors_single_helper() {
        let c = BlendContributors::single(WorldArchetypeId::Mediterranean);
        assert_eq!(c.len(), 1);
        let v: Vec<_> = c.iter().collect();
        assert_eq!(v[0].0, WorldArchetypeId::Mediterranean);
        assert!((v[0].1 - 1.0).abs() < 1e-6);
    }

    // =========================================================================
    // Phase 1.X-F.4.D: blend_bootstrap_params multi-archetype blend math tests
    // =========================================================================

    use crate::spline_types::{
        bootstrap_splines_continental_temperate, BootstrapSplineSet,
        ClimateInputDim, ParamSpline, Spline1D,
    };

    /// Build a synthetic `BootstrapSplineSet` with the given mountain
    /// amplitude, baseline values for other params. Used by F.4.D blend
    /// math tests to verify per-archetype output is preserved through
    /// blending.
    fn synthetic_splines(mountains_amplitude: f32) -> BootstrapSplineSet {
        BootstrapSplineSet {
            mountains_amplitude: ParamSpline {
                climate_input: ClimateInputDim::Pv,
                spline: Spline1D::from_control_points(vec![(0.0, mountains_amplitude)])
                    .unwrap(),
            },
            mountains_scale: 0.002, // f64 baseline
            continental_scale: ParamSpline {
                climate_input: ClimateInputDim::Continentalness,
                spline: Spline1D::from_control_points(vec![(0.0, 0.0003)]).unwrap(),
            },
            base_elevation_amplitude: ParamSpline {
                climate_input: ClimateInputDim::Pv,
                spline: Spline1D::from_control_points(vec![(0.0, 150.0)]).unwrap(),
            },
        }
    }

    fn median_climate_sample() -> ClimateSample {
        ClimateSample {
            temperature_c: 12.0,
            moisture_mm: 800.0,
            continentalness: 0.5,
            erosion: 0.0,
            weirdness: 1.0, // pv = 0
        }
    }

    /// Single-contributor blend produces output byte-identical to
    /// `BootstrapSplineSet::evaluate` directly.
    #[test]
    fn blend_bootstrap_params_single_contributor_byte_identical_to_archetype_evaluate() {
        let splines = bootstrap_splines_continental_temperate();
        let sample = median_climate_sample();
        let direct = splines.evaluate(&sample);

        let contributors = BlendContributors::single(WorldArchetypeId::ContinentalTemperate);
        let lookup = |_id: WorldArchetypeId| bootstrap_splines_continental_temperate();
        let blended = blend_bootstrap_params(&contributors, &lookup, &sample);

        assert_eq!(direct.mountains_amplitude.to_bits(), blended.mountains_amplitude.to_bits());
        assert_eq!(direct.mountains_scale.to_bits(), blended.mountains_scale.to_bits());
        assert_eq!(direct.continental_scale.to_bits(), blended.continental_scale.to_bits());
        assert_eq!(
            direct.base_elevation_amplitude.to_bits(),
            blended.base_elevation_amplitude.to_bits()
        );
    }

    /// Same archetype with weights 0.5+0.5 produces output equal to single-
    /// contributor case within f32/f64 epsilon.
    #[test]
    fn blend_bootstrap_params_identical_archetype_two_halves() {
        let mut contributors = BlendContributors {
            items: [(WorldArchetypeId::ContinentalTemperate, 0.0); 4],
            len: 0,
        };
        contributors.push(WorldArchetypeId::ContinentalTemperate, 0.5);
        contributors.push(WorldArchetypeId::ContinentalTemperate, 0.5);
        let lookup = |_id: WorldArchetypeId| bootstrap_splines_continental_temperate();
        let sample = median_climate_sample();
        let blended = blend_bootstrap_params(&contributors, &lookup, &sample);
        let direct = bootstrap_splines_continental_temperate().evaluate(&sample);

        assert!((blended.mountains_amplitude - direct.mountains_amplitude).abs() < 1e-3);
        assert!((blended.mountains_scale - direct.mountains_scale).abs() < 1e-9);
        assert!((blended.continental_scale - direct.continental_scale).abs() < 1e-7);
        assert!(
            (blended.base_elevation_amplitude - direct.base_elevation_amplitude).abs() < 1e-3
        );
    }

    /// Two distinct archetypes (synthetic 480 + 800 amplitudes) at 50/50
    /// weights → blended amplitude = 640 (within epsilon).
    #[test]
    fn blend_bootstrap_params_two_distinct_archetypes_50_50() {
        let mut contributors = BlendContributors {
            items: [(WorldArchetypeId::ContinentalTemperate, 0.0); 4],
            len: 0,
        };
        contributors.push(WorldArchetypeId::ContinentalTemperate, 0.5);
        contributors.push(WorldArchetypeId::BorealSubarctic, 0.5);
        let lookup = |id: WorldArchetypeId| match id {
            WorldArchetypeId::ContinentalTemperate => synthetic_splines(480.0),
            WorldArchetypeId::BorealSubarctic => synthetic_splines(800.0),
            _ => synthetic_splines(0.0),
        };
        let blended = blend_bootstrap_params(&contributors, &lookup, &median_climate_sample());
        assert!(
            (blended.mountains_amplitude - 640.0).abs() < 1e-3,
            "blended amplitude should be 640.0; got {}",
            blended.mountains_amplitude
        );
    }

    /// Four archetypes at 0.25 each → blended amplitude = mean.
    #[test]
    fn blend_bootstrap_params_four_archetypes_quarter_each() {
        let mut contributors = BlendContributors {
            items: [(WorldArchetypeId::ContinentalTemperate, 0.0); 4],
            len: 0,
        };
        contributors.push(WorldArchetypeId::ContinentalTemperate, 0.25);
        contributors.push(WorldArchetypeId::EquatorialTropical, 0.25);
        contributors.push(WorldArchetypeId::BorealSubarctic, 0.25);
        contributors.push(WorldArchetypeId::Desert, 0.25);
        let lookup = |id: WorldArchetypeId| match id {
            WorldArchetypeId::ContinentalTemperate => synthetic_splines(480.0),
            WorldArchetypeId::EquatorialTropical => synthetic_splines(350.0),
            WorldArchetypeId::BorealSubarctic => synthetic_splines(800.0),
            WorldArchetypeId::Desert => synthetic_splines(200.0),
            _ => synthetic_splines(0.0),
        };
        let blended = blend_bootstrap_params(&contributors, &lookup, &median_climate_sample());
        let expected_mean = (480.0 + 350.0 + 800.0 + 200.0) / 4.0; // = 457.5
        assert!(
            (blended.mountains_amplitude - expected_mean).abs() < 1e-3,
            "blended amplitude should be {}; got {}",
            expected_mean,
            blended.mountains_amplitude
        );
    }

    /// Convex combination invariant: blended `mountains_amplitude` is
    /// between min and max of contributing archetypes' amplitudes across
    /// 50 random contributor sets.
    #[test]
    fn blend_bootstrap_params_convex_hull_invariant() {
        // Deterministic pseudo-random via incrementing seed-like values.
        let amplitudes = [200.0_f32, 350.0, 480.0, 600.0, 800.0];
        for trial in 0..50 {
            let n = 1 + (trial % 4); // 1..=4 contributors
            let mut contributors = BlendContributors {
                items: [(WorldArchetypeId::ContinentalTemperate, 0.0); 4],
                len: 0,
            };
            let archetypes = [
                WorldArchetypeId::ContinentalTemperate,
                WorldArchetypeId::EquatorialTropical,
                WorldArchetypeId::BorealSubarctic,
                WorldArchetypeId::Mediterranean,
                WorldArchetypeId::Desert,
            ];
            let mut pseudo_weights = [0.0f32; 4];
            let mut total = 0.0f32;
            for i in 0..n {
                let pw = ((trial * 7 + i * 13) % 100) as f32 / 100.0 + 0.1;
                pseudo_weights[i] = pw;
                total += pw;
            }
            for i in 0..n {
                pseudo_weights[i] /= total;
                contributors.push(archetypes[i], pseudo_weights[i]);
            }

            let lookup = |id: WorldArchetypeId| {
                let amp = match id {
                    WorldArchetypeId::ContinentalTemperate => amplitudes[0],
                    WorldArchetypeId::EquatorialTropical => amplitudes[1],
                    WorldArchetypeId::BorealSubarctic => amplitudes[2],
                    WorldArchetypeId::Mediterranean => amplitudes[3],
                    WorldArchetypeId::Desert => amplitudes[4],
                    _ => 0.0,
                };
                synthetic_splines(amp)
            };
            let blended =
                blend_bootstrap_params(&contributors, &lookup, &median_climate_sample());

            // Compute min/max amplitudes among contributors.
            let mut min_amp = f32::INFINITY;
            let mut max_amp = f32::NEG_INFINITY;
            for i in 0..n {
                let amp = match archetypes[i] {
                    WorldArchetypeId::ContinentalTemperate => amplitudes[0],
                    WorldArchetypeId::EquatorialTropical => amplitudes[1],
                    WorldArchetypeId::BorealSubarctic => amplitudes[2],
                    WorldArchetypeId::Mediterranean => amplitudes[3],
                    WorldArchetypeId::Desert => amplitudes[4],
                    _ => 0.0,
                };
                if amp < min_amp {
                    min_amp = amp;
                }
                if amp > max_amp {
                    max_amp = amp;
                }
            }
            assert!(
                blended.mountains_amplitude >= min_amp - 1e-3
                    && blended.mountains_amplitude <= max_amp + 1e-3,
                "trial {}: blended amplitude {} not in convex hull [{}, {}]",
                trial,
                blended.mountains_amplitude,
                min_amp,
                max_amp
            );
        }
    }

    /// `mountains_scale` blend preserves f64 precision (per F.3 deviation 2).
    /// Synthetic splines with mountains_scale 0.001 + 0.003 at 50/50 weights
    /// → blended mountains_scale = 0.002 within f64 epsilon.
    #[test]
    fn blend_bootstrap_params_mountains_scale_f64_precision() {
        let mut contributors = BlendContributors {
            items: [(WorldArchetypeId::ContinentalTemperate, 0.0); 4],
            len: 0,
        };
        contributors.push(WorldArchetypeId::ContinentalTemperate, 0.5);
        contributors.push(WorldArchetypeId::BorealSubarctic, 0.5);
        // Synthetic splines with different mountains_scale values
        // (each archetype's BootstrapSplineSet stores it as direct f64).
        let lookup = |id: WorldArchetypeId| {
            let mut s = synthetic_splines(480.0);
            s.mountains_scale = match id {
                WorldArchetypeId::ContinentalTemperate => 0.001,
                WorldArchetypeId::BorealSubarctic => 0.003,
                _ => 0.002,
            };
            s
        };
        let blended = blend_bootstrap_params(&contributors, &lookup, &median_climate_sample());
        // Expected: 0.5 * 0.001 + 0.5 * 0.003 = 0.002
        assert!(
            (blended.mountains_scale - 0.002).abs() < 1e-15,
            "blended mountains_scale should be 0.002 (f64 exact); got {}",
            blended.mountains_scale
        );
    }

    /// Empty contributor list returns all-zero `BootstrapParams` (degenerate
    /// case; F.4.E guards against this reaching the noise pipeline).
    #[test]
    fn blend_bootstrap_params_zero_contributors_returns_zero() {
        let contributors = BlendContributors {
            items: [(WorldArchetypeId::ContinentalTemperate, 0.0); 4],
            len: 0,
        };
        let lookup = |_id: WorldArchetypeId| bootstrap_splines_continental_temperate();
        let blended = blend_bootstrap_params(&contributors, &lookup, &median_climate_sample());
        assert_eq!(blended.mountains_amplitude, 0.0);
        assert_eq!(blended.mountains_scale, 0.0);
        assert_eq!(blended.continental_scale, 0.0);
        assert_eq!(blended.base_elevation_amplitude, 0.0);
    }
}
