//! Puddle accumulation system — Laplacian concavity-driven water pooling.
//!
//! # Architecture
//!
//! 1. **Compute pass** (`puddle_accumulation.wgsl`): Each frame, updates a
//!    per-chunk R32Float puddle depth map. Rain fills terrain concavities
//!    (detected via heightmap Laplacian); water drains when rain stops.
//!
//! 2. **PBR integration**: Puddle depth modifies material properties —
//!    roughness → 0 (mirror-like), metallic → 1 (Fresnel reflections).
//!    The puddle map can be sampled in the fragment shader.
//!
//! 3. **Configuration**: [`PuddleConfig`] controls fill/drain rates,
//!    concavity threshold, and max depth.

use bytemuck::{Pod, Zeroable};

// ── GPU Uniform ─────────────────────────────────────────────────────────────

/// GPU-side parameters for the puddle accumulation compute shader.
///
/// Must match `PuddleParams` in `puddle_accumulation.wgsl`. 32 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PuddleParamsGpu {
    /// Water fill rate per second during rain.
    pub fill_rate: f32,
    /// Drain/evaporation rate per second.
    pub drain_rate: f32,
    /// Frame delta time.
    pub dt: f32,
    /// Current rain intensity (0.0–1.0).
    pub rain_intensity: f32,
    /// Chunk width in texels.
    pub width: u32,
    /// Chunk height in texels.
    pub height: u32,
    /// Laplacian threshold for concavity detection.
    /// Higher values require deeper terrain depressions.
    pub concavity_threshold: f32,
    /// Maximum puddle depth.
    pub max_depth: f32,
}

impl Default for PuddleParamsGpu {
    fn default() -> Self {
        Self {
            fill_rate: 0.08,
            drain_rate: 0.02,
            dt: 1.0 / 60.0,
            rain_intensity: 0.0,
            width: 256,
            height: 256,
            concavity_threshold: 0.05,
            max_depth: 0.3,
        }
    }
}

// ── Puddle Material ─────────────────────────────────────────────────────────

/// Puddle material properties for PBR wet-surface blending.
///
/// The fragment shader blends these over the base terrain material
/// proportional to `puddle_depth / max_depth`.
#[derive(Debug, Clone, Copy)]
pub struct PuddleMaterial {
    /// Roughness when fully wet (puddle surface). Very smooth → mirror reflection.
    pub roughness: f32,
    /// Metallic when fully wet. 1.0 gives Fresnel-like water reflections.
    pub metallic: f32,
    /// Depth threshold below which no visible puddle appears (avoids noise).
    pub threshold: f32,
    /// Darkening factor for wet terrain albedo (water absorbs light).
    pub albedo_darken: f32,
}

impl Default for PuddleMaterial {
    fn default() -> Self {
        Self {
            roughness: 0.02,
            metallic: 1.0,
            threshold: 0.02,
            albedo_darken: 0.6,
        }
    }
}

// ── CPU Configuration ───────────────────────────────────────────────────────

/// Controls the compute-side puddle accumulation behavior.
#[derive(Debug, Clone, Copy)]
pub struct PuddleConfig {
    /// Rate at which water fills concavities during rain (depth/second).
    pub fill_rate: f32,
    /// Rate at which puddles drain/evaporate (depth/second).
    pub drain_rate: f32,
    /// Maximum puddle depth before capping.
    pub max_depth: f32,
    /// Heightmap Laplacian threshold for concavity detection.
    /// Larger values require deeper depressions.
    pub concavity_threshold: f32,
    /// Resolution of puddle depth map per chunk (square).
    pub map_resolution: u32,
}

impl Default for PuddleConfig {
    fn default() -> Self {
        Self {
            fill_rate: 0.08,
            drain_rate: 0.02,
            max_depth: 0.3,
            concavity_threshold: 0.05,
            map_resolution: 256,
        }
    }
}

impl PuddleConfig {
    /// Build GPU params from this config + runtime state.
    pub fn to_gpu_params(&self, dt: f32, rain_intensity: f32) -> PuddleParamsGpu {
        PuddleParamsGpu {
            fill_rate: self.fill_rate,
            drain_rate: self.drain_rate,
            dt,
            rain_intensity,
            width: self.map_resolution,
            height: self.map_resolution,
            concavity_threshold: self.concavity_threshold,
            max_depth: self.max_depth,
        }
    }

    /// Tropical environment — fast fill, slow drain (humid).
    pub fn tropical() -> Self {
        Self {
            fill_rate: 0.15,
            drain_rate: 0.01,
            max_depth: 0.5,
            concavity_threshold: 0.03,
            map_resolution: 256,
        }
    }

    /// Arid environment — slow fill, fast drain.
    pub fn arid() -> Self {
        Self {
            fill_rate: 0.04,
            drain_rate: 0.08,
            max_depth: 0.15,
            concavity_threshold: 0.08,
            map_resolution: 256,
        }
    }

    /// Compute dispatch workgroup count.
    /// Workgroup size is (8, 8), so divide resolution by 8, rounding up.
    pub fn dispatch_size(&self) -> (u32, u32) {
        let wg = 8u32;
        (
            self.map_resolution.div_ceil(wg),
            self.map_resolution.div_ceil(wg),
        )
    }
}

// ── WGSL Source ─────────────────────────────────────────────────────────────

/// Compile-time puddle accumulation compute shader source.
pub const PUDDLE_ACCUMULATION_WGSL: &str = include_str!("../shaders/puddle_accumulation.wgsl");

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puddle_params_gpu_size() {
        assert_eq!(std::mem::size_of::<PuddleParamsGpu>(), 32);
    }

    #[test]
    fn puddle_params_gpu_default() {
        let p = PuddleParamsGpu::default();
        assert_eq!(p.fill_rate, 0.08);
        assert_eq!(p.drain_rate, 0.02);
        assert_eq!(p.rain_intensity, 0.0);
        assert_eq!(p.width, 256);
        assert_eq!(p.concavity_threshold, 0.05);
    }

    #[test]
    fn puddle_params_gpu_bytemuck_roundtrip() {
        let p = PuddleParamsGpu {
            fill_rate: 0.15,
            drain_rate: 0.03,
            dt: 0.016,
            rain_intensity: 0.8,
            width: 512,
            height: 512,
            concavity_threshold: 0.04,
            max_depth: 0.5,
        };
        let bytes = bytemuck::bytes_of(&p);
        assert_eq!(bytes.len(), 32);
        let back: &PuddleParamsGpu = bytemuck::from_bytes(bytes);
        assert_eq!(back.width, 512);
        assert!((back.rain_intensity - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn puddle_material_default() {
        let m = PuddleMaterial::default();
        assert!(m.roughness < 0.1, "puddle should be smooth");
        assert!((m.metallic - 1.0).abs() < f32::EPSILON);
        assert!(m.albedo_darken < 1.0);
    }

    #[test]
    fn puddle_config_to_gpu_params() {
        let cfg = PuddleConfig::default();
        let params = cfg.to_gpu_params(0.016, 0.7);
        assert!((params.dt - 0.016).abs() < f32::EPSILON);
        assert!((params.rain_intensity - 0.7).abs() < f32::EPSILON);
        assert_eq!(params.width, cfg.map_resolution);
    }

    #[test]
    fn puddle_config_tropical_fills_faster() {
        let default = PuddleConfig::default();
        let tropical = PuddleConfig::tropical();
        assert!(tropical.fill_rate > default.fill_rate);
        assert!(tropical.drain_rate < default.drain_rate);
    }

    #[test]
    fn puddle_config_arid_drains_faster() {
        let default = PuddleConfig::default();
        let arid = PuddleConfig::arid();
        assert!(arid.drain_rate > default.drain_rate);
        assert!(arid.fill_rate < default.fill_rate);
    }

    #[test]
    fn puddle_dispatch_size() {
        let cfg = PuddleConfig::default();
        let (x, y) = cfg.dispatch_size();
        assert_eq!(x, 32); // 256 / 8
        assert_eq!(y, 32);
    }

    #[test]
    fn puddle_wgsl_present() {
        assert!(PUDDLE_ACCUMULATION_WGSL.contains("update_puddles"));
        assert!(PUDDLE_ACCUMULATION_WGSL.contains("laplacian"));
        assert!(PUDDLE_ACCUMULATION_WGSL.contains("PuddleParams"));
    }
}
