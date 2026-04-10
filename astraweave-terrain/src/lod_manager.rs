//! LOD (Level of Detail) manager with hysteresis to prevent popping
//!
//! This module implements:
//! - 4 LOD levels (L0: full detail, L1: half, L2: quarter, L3: skybox)
//! - Hysteresis curve (10% margin to prevent flickering)
//! - Blend zones for smooth transitions
//! - Distance-based LOD selection

use crate::{meshing::ChunkMesh, ChunkId};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// LOD level for terrain chunks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LodLevel {
    /// L0: Full detail (all vertices)
    Full = 0,

    /// L1: Half resolution (every 2nd vertex)
    Half = 1,

    /// L2: Quarter resolution (every 4th vertex)
    Quarter = 2,

    /// L3: Skybox/impostor (minimal geometry)
    Skybox = 3,
}

impl LodLevel {
    /// Get vertex skip factor for this LOD level
    pub fn skip_factor(self) -> usize {
        match self {
            LodLevel::Full => 1,
            LodLevel::Half => 2,
            LodLevel::Quarter => 4,
            LodLevel::Skybox => 16,
        }
    }

    /// Get next lower detail level (or None if already lowest)
    pub fn lower(self) -> Option<LodLevel> {
        match self {
            LodLevel::Full => Some(LodLevel::Half),
            LodLevel::Half => Some(LodLevel::Quarter),
            LodLevel::Quarter => Some(LodLevel::Skybox),
            LodLevel::Skybox => None,
        }
    }

    /// Get next higher detail level (or None if already highest)
    pub fn higher(self) -> Option<LodLevel> {
        match self {
            LodLevel::Skybox => Some(LodLevel::Quarter),
            LodLevel::Quarter => Some(LodLevel::Half),
            LodLevel::Half => Some(LodLevel::Full),
            LodLevel::Full => None,
        }
    }
}

/// View parameters for screen-space error LOD selection.
///
/// When provided, LOD transitions use pixel-error metric instead of fixed
/// distance thresholds, producing resolution- and FOV-adaptive LOD that
/// allocates geometry budget where it matters most.
#[derive(Debug, Clone, Copy)]
pub struct ViewParams {
    /// Vertical field of view in radians (e.g. `Camera.fovy`).
    pub fov_y: f32,
    /// Viewport height in pixels (e.g. swapchain height).
    pub screen_height: f32,
}

impl ViewParams {
    pub fn new(fov_y: f32, screen_height: f32) -> Self {
        Self {
            fov_y,
            screen_height,
        }
    }
}

/// Compute the projected pixel error given a world-space geometric error,
/// distance to the observer, and view parameters.
///
/// Formula:
/// ```text
/// pixel_error = (geometric_error * screen_height) / (distance * 2.0 * tan(fov_y / 2.0))
/// ```
///
/// Returns `f32::MAX` when distance is effectively zero.
#[inline]
pub fn compute_pixel_error(geometric_error: f32, distance: f32, view: &ViewParams) -> f32 {
    let denom = distance * 2.0 * (view.fov_y * 0.5).tan();
    if denom < 1e-6 {
        return f32::MAX; // Camera inside chunk — max detail
    }
    (geometric_error * view.screen_height) / denom
}

/// LOD transition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LodConfig {
    /// Distance thresholds for each LOD level (in world units)
    /// [L0->L1, L1->L2, L2->L3]
    /// Used as fallback when `ViewParams` is not supplied.
    pub distance_thresholds: [f32; 3],

    /// Pixel-error thresholds for screen-space error selection.
    /// [L0->L1, L1->L2, L2->L3]
    /// When `ViewParams` is supplied, a chunk transitions to a lower LOD
    /// when its pixel error drops below the threshold.
    pub pixel_error_thresholds: [f32; 3],

    /// Per-LOD geometric error in world units, indexed by `LodLevel as usize`.
    /// This is the maximum surface deviation introduced at each LOD.
    /// Typically: [0.0, chunk_spacing, 2*chunk_spacing, 4*chunk_spacing].
    pub geometric_errors: [f32; 4],

    /// Hysteresis margin (0.0-1.0, typically 0.1 for 10%)
    pub hysteresis_margin: f32,

    /// Blend zone size (world units)
    pub blend_zone_size: f32,

    /// Enable blend zones (cross-fade between LODs)
    pub enable_blending: bool,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            // Legacy distance thresholds (fallback when ViewParams is not supplied)
            distance_thresholds: [256.0, 512.0, 1024.0],
            // Screen-space error thresholds in pixels:
            // L0→L1 at 2 px, L1→L2 at 4 px, L2→L3 at 8 px
            pixel_error_thresholds: [2.0, 4.0, 8.0],
            // Geometric error per LOD in world units:
            // L0 = 0 (perfect), L1 = 0.5m, L2 = 1.0m, L3 = 4.0m
            geometric_errors: [0.0, 0.5, 1.0, 4.0],
            hysteresis_margin: 0.1, // 10% margin
            blend_zone_size: 32.0,  // 32m blend zone
            enable_blending: true,
        }
    }
}

impl LodConfig {
    /// Get distance threshold for transitioning from `from` to `to` LOD
    pub fn get_threshold(&self, from: LodLevel, to: LodLevel, increasing_detail: bool) -> f32 {
        let base_threshold = match (from, to) {
            (LodLevel::Full, LodLevel::Half) | (LodLevel::Half, LodLevel::Full) => {
                self.distance_thresholds[0]
            }
            (LodLevel::Half, LodLevel::Quarter) | (LodLevel::Quarter, LodLevel::Half) => {
                self.distance_thresholds[1]
            }
            (LodLevel::Quarter, LodLevel::Skybox) | (LodLevel::Skybox, LodLevel::Quarter) => {
                self.distance_thresholds[2]
            }
            _ => return f32::MAX, // Invalid transition
        };

        // Apply hysteresis
        if increasing_detail {
            base_threshold * (1.0 - self.hysteresis_margin)
        } else {
            base_threshold * (1.0 + self.hysteresis_margin)
        }
    }

    /// Get pixel-error threshold with hysteresis for a LOD transition.
    ///
    /// For screen-space error: a higher pixel error means MORE detail needed,
    /// so "increasing detail" means the error exceeded the upper threshold.
    pub fn get_pixel_error_threshold(
        &self,
        from: LodLevel,
        to: LodLevel,
        increasing_detail: bool,
    ) -> f32 {
        let base = match (from, to) {
            (LodLevel::Full, LodLevel::Half) | (LodLevel::Half, LodLevel::Full) => {
                self.pixel_error_thresholds[0]
            }
            (LodLevel::Half, LodLevel::Quarter) | (LodLevel::Quarter, LodLevel::Half) => {
                self.pixel_error_thresholds[1]
            }
            (LodLevel::Quarter, LodLevel::Skybox) | (LodLevel::Skybox, LodLevel::Quarter) => {
                self.pixel_error_thresholds[2]
            }
            _ => return f32::MAX,
        };

        // Hysteresis on pixel error (inverted vs distance):
        // Increasing detail → error exceeded threshold → trigger at base * (1 + margin)
        // Decreasing detail → error dropped → trigger at base * (1 - margin)
        if increasing_detail {
            base * (1.0 + self.hysteresis_margin)
        } else {
            base * (1.0 - self.hysteresis_margin)
        }
    }
}

/// LOD state for a single chunk
#[derive(Debug, Clone)]
pub struct ChunkLodState {
    /// Current LOD level
    pub current_lod: LodLevel,

    /// Target LOD level (for blending)
    pub target_lod: Option<LodLevel>,

    /// Blend factor (0.0 = current, 1.0 = target)
    pub blend_factor: f32,

    /// Distance from camera
    pub distance: f32,
}

/// Cached LOD meshes for a chunk (Phase 2 optimization)
#[derive(Debug, Clone)]
pub struct ChunkLodCache {
    /// Full detail mesh (L0)
    pub l0_mesh: Option<Arc<ChunkMesh>>,

    /// Half resolution mesh (L1)
    pub l1_mesh: Option<Arc<ChunkMesh>>,

    /// Quarter resolution mesh (L2)
    pub l2_mesh: Option<Arc<ChunkMesh>>,

    /// Skybox/impostor mesh (L3) - typically not cached (minimal geometry)
    pub l3_mesh: Option<Arc<ChunkMesh>>,
}

impl Default for ChunkLodCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkLodCache {
    /// Create an empty cache
    pub fn new() -> Self {
        Self {
            l0_mesh: None,
            l1_mesh: None,
            l2_mesh: None,
            l3_mesh: None,
        }
    }

    /// Get mesh for specific LOD level
    pub fn get_mesh(&self, lod: LodLevel) -> Option<Arc<ChunkMesh>> {
        match lod {
            LodLevel::Full => self.l0_mesh.clone(),
            LodLevel::Half => self.l1_mesh.clone(),
            LodLevel::Quarter => self.l2_mesh.clone(),
            LodLevel::Skybox => self.l3_mesh.clone(),
        }
    }

    /// Store mesh for specific LOD level
    pub fn set_mesh(&mut self, lod: LodLevel, mesh: Arc<ChunkMesh>) {
        match lod {
            LodLevel::Full => self.l0_mesh = Some(mesh),
            LodLevel::Half => self.l1_mesh = Some(mesh),
            LodLevel::Quarter => self.l2_mesh = Some(mesh),
            LodLevel::Skybox => self.l3_mesh = Some(mesh),
        }
    }

    /// Check if mesh exists for LOD level
    pub fn has_mesh(&self, lod: LodLevel) -> bool {
        self.get_mesh(lod).is_some()
    }

    /// Get total memory usage of cached meshes
    pub fn memory_usage(&self) -> usize {
        let mut total = 0;
        if let Some(m) = &self.l0_mesh {
            total += m.memory_usage();
        }
        if let Some(m) = &self.l1_mesh {
            total += m.memory_usage();
        }
        if let Some(m) = &self.l2_mesh {
            total += m.memory_usage();
        }
        if let Some(m) = &self.l3_mesh {
            total += m.memory_usage();
        }
        total
    }
}

/// LOD manager with hysteresis and mesh caching
pub struct LodManager {
    config: LodConfig,

    /// LOD state per chunk
    chunk_states: HashMap<ChunkId, ChunkLodState>,

    /// Mesh cache per chunk (Phase 2 optimization - eliminates regeneration)
    mesh_cache: HashMap<ChunkId, ChunkLodCache>,

    /// Chunk size (for distance calculations)
    chunk_size: f32,

    /// Cache statistics
    cache_hits: usize,
    cache_misses: usize,
}

impl LodManager {
    /// Create a new LOD manager
    pub fn new(config: LodConfig, chunk_size: f32) -> Self {
        Self {
            config,
            chunk_states: HashMap::new(),
            mesh_cache: HashMap::new(),
            chunk_size,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Get mesh from cache (Phase 2 optimization)
    pub fn get_cached_mesh(&mut self, chunk_id: ChunkId, lod: LodLevel) -> Option<Arc<ChunkMesh>> {
        if let Some(cache) = self.mesh_cache.get(&chunk_id) {
            if let Some(mesh) = cache.get_mesh(lod) {
                self.cache_hits += 1;
                return Some(mesh);
            }
        }
        self.cache_misses += 1;
        None
    }

    /// Store mesh in cache (Phase 2 optimization)
    pub fn cache_mesh(&mut self, chunk_id: ChunkId, lod: LodLevel, mesh: Arc<ChunkMesh>) {
        let cache = self.mesh_cache.entry(chunk_id).or_default();
        cache.set_mesh(lod, mesh);
    }

    /// Get cache hit rate (for diagnostics)
    pub fn cache_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }

    /// Get total cache memory usage
    pub fn cache_memory_usage(&self) -> usize {
        self.mesh_cache.values().map(|c| c.memory_usage()).sum()
    }

    /// Evict cache for chunks beyond distance threshold
    pub fn evict_distant_cache(&mut self, camera_pos: Vec3, max_distance: f32) -> usize {
        let mut evicted = 0;
        self.mesh_cache.retain(|chunk_id, _| {
            let chunk_center = chunk_id.to_center_pos(self.chunk_size);
            let distance = (chunk_center - camera_pos).length();
            if distance > max_distance {
                evicted += 1;
                false
            } else {
                true
            }
        });
        evicted
    }

    /// Update LOD for a chunk based on camera position.
    ///
    /// When `view` is `Some`, uses screen-space pixel error for LOD selection
    /// (resolution- and FOV-adaptive). Falls back to fixed distance thresholds
    /// when `view` is `None`.
    pub fn update_chunk_lod(
        &mut self,
        chunk_id: ChunkId,
        camera_pos: Vec3,
        view: Option<&ViewParams>,
    ) -> bool {
        let chunk_center = chunk_id.to_center_pos(self.chunk_size);
        let distance = (chunk_center - camera_pos).length();

        // Get or create chunk state
        let state = self
            .chunk_states
            .entry(chunk_id)
            .or_insert_with(|| ChunkLodState {
                current_lod: LodLevel::Full,
                target_lod: None,
                blend_factor: 0.0,
                distance,
            });

        state.distance = distance;

        // Determine target LOD
        let target_lod = if let Some(vp) = view {
            // Screen-space error path: pick the coarsest LOD whose projected
            // geometric error is below the pixel-error budget.
            // Walk from coarsest (Skybox) to finest (Full).
            let err_skybox = compute_pixel_error(self.config.geometric_errors[3], distance, vp);
            let err_quarter = compute_pixel_error(self.config.geometric_errors[2], distance, vp);
            let err_half = compute_pixel_error(self.config.geometric_errors[1], distance, vp);

            if err_skybox <= self.config.pixel_error_thresholds[2] {
                LodLevel::Skybox
            } else if err_quarter <= self.config.pixel_error_thresholds[1] {
                LodLevel::Quarter
            } else if err_half <= self.config.pixel_error_thresholds[0] {
                LodLevel::Half
            } else {
                LodLevel::Full
            }
        } else {
            // Legacy distance-bucket fallback
            if distance < self.config.distance_thresholds[0] {
                LodLevel::Full
            } else if distance < self.config.distance_thresholds[1] {
                LodLevel::Half
            } else if distance < self.config.distance_thresholds[2] {
                LodLevel::Quarter
            } else {
                LodLevel::Skybox
            }
        };

        // Check if LOD should change (with hysteresis)
        if target_lod != state.current_lod {
            let increasing_detail = (target_lod as u8) < (state.current_lod as u8);

            let should_transition = if let Some(vp) = view {
                // Screen-space hysteresis: compare pixel error against threshold
                let candidate_lod = if increasing_detail {
                    target_lod
                } else {
                    state.current_lod
                };
                let geo_err = self.config.geometric_errors[candidate_lod as usize];
                let px_err = compute_pixel_error(geo_err, distance, vp);
                let threshold = self.config.get_pixel_error_threshold(
                    state.current_lod,
                    target_lod,
                    increasing_detail,
                );
                if increasing_detail {
                    px_err > threshold
                } else {
                    px_err < threshold
                }
            } else {
                // Legacy distance hysteresis
                let threshold =
                    self.config
                        .get_threshold(state.current_lod, target_lod, increasing_detail);
                if increasing_detail {
                    distance < threshold
                } else {
                    distance > threshold
                }
            };

            if should_transition {
                if self.config.enable_blending {
                    // Start blend transition
                    state.target_lod = Some(target_lod);
                    state.blend_factor = 0.0;
                } else {
                    // Instant transition
                    state.current_lod = target_lod;
                    state.target_lod = None;
                    state.blend_factor = 0.0;
                }
                return true; // LOD changed
            }
        }

        // Update blend factor if transitioning
        if let Some(target) = state.target_lod {
            if self.config.enable_blending {
                // Advance blend factor (lerp towards target)
                state.blend_factor += 0.1; // Adjust blend speed as needed

                if state.blend_factor >= 1.0 {
                    // Transition complete
                    state.current_lod = target;
                    state.target_lod = None;
                    state.blend_factor = 0.0;
                    return true; // LOD changed
                }
            } else {
                // Instant transition
                state.current_lod = target;
                state.target_lod = None;
                state.blend_factor = 0.0;
                return true;
            }
        }

        false // No LOD change
    }

    /// Update all loaded chunks.
    ///
    /// When `view` is `Some`, uses screen-space pixel error for LOD selection.
    pub fn update_all_chunks(
        &mut self,
        chunk_ids: &[ChunkId],
        camera_pos: Vec3,
        view: Option<&ViewParams>,
    ) -> usize {
        let mut changed_count = 0;

        for &chunk_id in chunk_ids {
            if self.update_chunk_lod(chunk_id, camera_pos, view) {
                changed_count += 1;
            }
        }

        // Remove states for unloaded chunks
        self.chunk_states.retain(|id, _| chunk_ids.contains(id));

        changed_count
    }

    /// Get LOD state for a chunk
    pub fn get_chunk_state(&self, chunk_id: ChunkId) -> Option<&ChunkLodState> {
        self.chunk_states.get(&chunk_id)
    }

    /// Get current LOD level for a chunk
    pub fn get_chunk_lod(&self, chunk_id: ChunkId) -> Option<LodLevel> {
        self.chunk_states.get(&chunk_id).map(|s| s.current_lod)
    }

    /// Check if chunk is transitioning between LODs
    pub fn is_transitioning(&self, chunk_id: ChunkId) -> bool {
        self.chunk_states
            .get(&chunk_id)
            .map(|s| s.target_lod.is_some())
            .unwrap_or(false)
    }

    /// Get blend factor for a transitioning chunk
    pub fn get_blend_factor(&self, chunk_id: ChunkId) -> f32 {
        self.chunk_states
            .get(&chunk_id)
            .map(|s| s.blend_factor)
            .unwrap_or(0.0)
    }

    /// Get statistics
    pub fn get_stats(&self) -> LodStats {
        let mut stats = LodStats::default();

        for state in self.chunk_states.values() {
            match state.current_lod {
                LodLevel::Full => stats.full_count += 1,
                LodLevel::Half => stats.half_count += 1,
                LodLevel::Quarter => stats.quarter_count += 1,
                LodLevel::Skybox => stats.skybox_count += 1,
            }

            if state.target_lod.is_some() {
                stats.transitioning_count += 1;
            }
        }

        stats.total_chunks = self.chunk_states.len();
        stats
    }
}

/// LOD statistics
#[derive(Debug, Clone, Default)]
pub struct LodStats {
    pub total_chunks: usize,
    pub full_count: usize,
    pub half_count: usize,
    pub quarter_count: usize,
    pub skybox_count: usize,
    pub transitioning_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_level_skip_factors() {
        assert_eq!(LodLevel::Full.skip_factor(), 1);
        assert_eq!(LodLevel::Half.skip_factor(), 2);
        assert_eq!(LodLevel::Quarter.skip_factor(), 4);
        assert_eq!(LodLevel::Skybox.skip_factor(), 16);
    }

    #[test]
    fn test_lod_level_transitions() {
        assert_eq!(LodLevel::Full.lower(), Some(LodLevel::Half));
        assert_eq!(LodLevel::Half.lower(), Some(LodLevel::Quarter));
        assert_eq!(LodLevel::Quarter.lower(), Some(LodLevel::Skybox));
        assert_eq!(LodLevel::Skybox.lower(), None);

        assert_eq!(LodLevel::Skybox.higher(), Some(LodLevel::Quarter));
        assert_eq!(LodLevel::Quarter.higher(), Some(LodLevel::Half));
        assert_eq!(LodLevel::Half.higher(), Some(LodLevel::Full));
        assert_eq!(LodLevel::Full.higher(), None);
    }

    #[test]
    fn test_hysteresis_margins() {
        let config = LodConfig::default();

        // Increasing detail (moving closer)
        let threshold_in = config.get_threshold(LodLevel::Half, LodLevel::Full, true);
        assert!(threshold_in < config.distance_thresholds[0]);

        // Decreasing detail (moving away)
        let threshold_out = config.get_threshold(LodLevel::Full, LodLevel::Half, false);
        assert!(threshold_out > config.distance_thresholds[0]);

        // Hysteresis gap
        assert!(threshold_out > threshold_in);
    }

    #[test]
    fn test_lod_manager_basic() {
        let config = LodConfig {
            distance_thresholds: [256.0, 512.0, 1024.0],
            hysteresis_margin: 0.1,
            blend_zone_size: 32.0,
            enable_blending: false, // Disable blending for simpler test
            ..LodConfig::default()
        };
        let mut manager = LodManager::new(config, 256.0);
        let chunk_id = ChunkId::new(0, 0);

        // Start at chunk center (distance = 0) -> Full LOD
        let chunk_center = chunk_id.to_center_pos(256.0);
        manager.update_chunk_lod(chunk_id, chunk_center, None);
        assert_eq!(manager.get_chunk_lod(chunk_id), Some(LodLevel::Full));

        // Move to exact threshold distance + a bit more (300m from center)
        let far_pos = chunk_center + Vec3::new(300.0, 0.0, 0.0);
        manager.update_chunk_lod(chunk_id, far_pos, None);
        // Distance is 300, threshold for downgrade is 256 * 1.1 = 281.6
        assert_eq!(manager.get_chunk_lod(chunk_id), Some(LodLevel::Half));
    }

    #[test]
    fn test_lod_level_eq_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(LodLevel::Full);
        set.insert(LodLevel::Half);
        set.insert(LodLevel::Full); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&LodLevel::Full));
        assert!(set.contains(&LodLevel::Half));
    }

    #[test]
    fn test_lod_level_serialization() {
        let level = LodLevel::Quarter;
        let json = serde_json::to_string(&level).unwrap();
        let deserialized: LodLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, deserialized);
    }

    #[test]
    fn test_lod_config_default() {
        let config = LodConfig::default();
        assert_eq!(config.distance_thresholds[0], 256.0);
        assert_eq!(config.distance_thresholds[1], 512.0);
        assert_eq!(config.distance_thresholds[2], 1024.0);
        assert_eq!(config.hysteresis_margin, 0.1);
        assert_eq!(config.blend_zone_size, 32.0);
        assert!(config.enable_blending);
    }

    #[test]
    fn test_lod_config_serialization() {
        let config = LodConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LodConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.distance_thresholds, deserialized.distance_thresholds);
        assert_eq!(config.hysteresis_margin, deserialized.hysteresis_margin);
        assert_eq!(config.blend_zone_size, deserialized.blend_zone_size);
        assert_eq!(config.enable_blending, deserialized.enable_blending);
    }

    #[test]
    fn test_lod_config_invalid_transition() {
        let config = LodConfig::default();

        // Full to Skybox is not a valid single-step transition
        let threshold = config.get_threshold(LodLevel::Full, LodLevel::Skybox, true);
        assert_eq!(threshold, f32::MAX);
    }

    #[test]
    fn test_chunk_lod_cache_new() {
        let cache = ChunkLodCache::new();
        assert!(cache.l0_mesh.is_none());
        assert!(cache.l1_mesh.is_none());
        assert!(cache.l2_mesh.is_none());
        assert!(cache.l3_mesh.is_none());
    }

    #[test]
    fn test_chunk_lod_cache_has_mesh() {
        let cache = ChunkLodCache::new();
        assert!(!cache.has_mesh(LodLevel::Full));
        assert!(!cache.has_mesh(LodLevel::Half));
        assert!(!cache.has_mesh(LodLevel::Quarter));
        assert!(!cache.has_mesh(LodLevel::Skybox));
    }

    #[test]
    fn test_chunk_lod_state_clone() {
        let state = ChunkLodState {
            current_lod: LodLevel::Half,
            target_lod: Some(LodLevel::Full),
            blend_factor: 0.5,
            distance: 300.0,
        };

        let cloned = state.clone();
        assert_eq!(cloned.current_lod, LodLevel::Half);
        assert_eq!(cloned.target_lod, Some(LodLevel::Full));
        assert_eq!(cloned.blend_factor, 0.5);
        assert_eq!(cloned.distance, 300.0);
    }

    #[test]
    fn test_lod_manager_cache_operations() {
        let config = LodConfig::default();
        let mut manager = LodManager::new(config, 256.0);
        let chunk_id = ChunkId::new(1, 1);

        // Initially no cached mesh
        assert!(manager.get_cached_mesh(chunk_id, LodLevel::Full).is_none());

        // Cache miss should be recorded
        // Note: cache_misses would increment, but we can't check internal state directly
    }

    #[test]
    fn test_lod_manager_cache_hit_rate_zero() {
        let config = LodConfig::default();
        let manager = LodManager::new(config, 256.0);

        // No operations = 0% hit rate (0/0 case returns 0.0)
        assert_eq!(manager.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_lod_manager_cache_memory_empty() {
        let config = LodConfig::default();
        let manager = LodManager::new(config, 256.0);

        assert_eq!(manager.cache_memory_usage(), 0);
    }

    #[test]
    fn test_lod_manager_get_chunk_state_none() {
        let config = LodConfig::default();
        let manager = LodManager::new(config, 256.0);
        let chunk_id = ChunkId::new(999, 999);

        assert!(manager.get_chunk_state(chunk_id).is_none());
    }

    #[test]
    fn test_lod_manager_get_chunk_lod_none() {
        let config = LodConfig::default();
        let manager = LodManager::new(config, 256.0);
        let chunk_id = ChunkId::new(999, 999);

        assert!(manager.get_chunk_lod(chunk_id).is_none());
    }

    #[test]
    fn test_lod_manager_is_transitioning_false() {
        let config = LodConfig::default();
        let manager = LodManager::new(config, 256.0);
        let chunk_id = ChunkId::new(999, 999);

        assert!(!manager.is_transitioning(chunk_id));
    }

    #[test]
    fn test_lod_manager_get_blend_factor_default() {
        let config = LodConfig::default();
        let manager = LodManager::new(config, 256.0);
        let chunk_id = ChunkId::new(999, 999);

        assert_eq!(manager.get_blend_factor(chunk_id), 0.0);
    }

    #[test]
    fn test_lod_stats_default() {
        let stats = LodStats::default();
        assert_eq!(stats.total_chunks, 0);
        assert_eq!(stats.full_count, 0);
        assert_eq!(stats.half_count, 0);
        assert_eq!(stats.quarter_count, 0);
        assert_eq!(stats.skybox_count, 0);
        assert_eq!(stats.transitioning_count, 0);
    }

    #[test]
    fn test_lod_manager_get_stats_empty() {
        let config = LodConfig::default();
        let manager = LodManager::new(config, 256.0);

        let stats = manager.get_stats();
        assert_eq!(stats.total_chunks, 0);
        assert_eq!(stats.full_count, 0);
    }

    #[test]
    fn test_lod_manager_update_all_chunks_empty() {
        let config = LodConfig::default();
        let mut manager = LodManager::new(config, 256.0);

        let changed = manager.update_all_chunks(&[], Vec3::ZERO, None);
        assert_eq!(changed, 0);
    }

    #[test]
    fn test_lod_manager_update_all_chunks_multiple() {
        let config = LodConfig {
            distance_thresholds: [256.0, 512.0, 1024.0],
            hysteresis_margin: 0.1,
            blend_zone_size: 32.0,
            enable_blending: false,
            ..LodConfig::default()
        };
        let mut manager = LodManager::new(config, 256.0);

        let chunks = vec![ChunkId::new(0, 0), ChunkId::new(1, 0), ChunkId::new(2, 0)];

        // First update - all chunks get initialized
        manager.update_all_chunks(&chunks, Vec3::ZERO, None);

        let stats = manager.get_stats();
        assert_eq!(stats.total_chunks, 3);
    }

    #[test]
    fn test_lod_manager_evict_distant_cache() {
        let config = LodConfig::default();
        let mut manager = LodManager::new(config, 256.0);

        // Evicting with empty cache should return 0
        let evicted = manager.evict_distant_cache(Vec3::ZERO, 1000.0);
        assert_eq!(evicted, 0);
    }

    #[test]
    fn test_lod_config_threshold_half_to_quarter() {
        let config = LodConfig::default();

        let threshold_down = config.get_threshold(LodLevel::Half, LodLevel::Quarter, false);
        let threshold_up = config.get_threshold(LodLevel::Quarter, LodLevel::Half, true);

        // Should be different due to hysteresis
        assert!(threshold_down > threshold_up);
    }

    #[test]
    fn test_lod_config_threshold_quarter_to_skybox() {
        let config = LodConfig::default();

        let threshold_down = config.get_threshold(LodLevel::Quarter, LodLevel::Skybox, false);
        let threshold_up = config.get_threshold(LodLevel::Skybox, LodLevel::Quarter, true);

        assert!(threshold_down > threshold_up);
    }

    #[test]
    fn test_lod_manager_with_blending() {
        let config = LodConfig {
            distance_thresholds: [256.0, 512.0, 1024.0],
            hysteresis_margin: 0.1,
            blend_zone_size: 32.0,
            enable_blending: true,
            ..LodConfig::default()
        };
        let mut manager = LodManager::new(config, 256.0);
        let chunk_id = ChunkId::new(0, 0);

        // Initialize chunk at center
        let chunk_center = chunk_id.to_center_pos(256.0);
        manager.update_chunk_lod(chunk_id, chunk_center, None);

        // Verify it starts in Full LOD
        assert_eq!(manager.get_chunk_lod(chunk_id), Some(LodLevel::Full));
    }

    #[test]
    fn test_lod_stats_clone() {
        let stats = LodStats {
            total_chunks: 10,
            full_count: 4,
            half_count: 3,
            quarter_count: 2,
            skybox_count: 1,
            transitioning_count: 2,
        };

        let cloned = stats.clone();
        assert_eq!(stats.total_chunks, cloned.total_chunks);
        assert_eq!(stats.full_count, cloned.full_count);
    }
}
