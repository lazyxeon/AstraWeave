//! Snow accumulation system — per-chunk compute + PBR snow blending configuration.
//!
//! # Architecture
//!
//! 1. **Compute pass** (`snow_accumulation.wgsl`): Each frame, updates a
//!    per-chunk R32Float accumulation heightmap based on snowfall state and
//!    terrain normals. Upward-facing surfaces gain depth; steep/melting lose it.
//!
//! 2. **PBR integration**: The global `snow_amount` in [`SceneEnvironmentUBO`]
//!    (`fog_range_pad.w`) drives a material blend in the fragment shader:
//!    albedo → snow white, roughness → 0.8, metallic → 0.0.
//!
//! 3. **Configuration**: [`SnowMaterial`] holds snow albedo, roughness, and
//!    blend thresholds. [`SnowAccumulationConfig`] controls compute-side rates.

use bytemuck::{Pod, Zeroable};

// ─── GPU Uniform ─────────────────────────────────────────────────────────

/// GPU-side parameters for the snow accumulation compute shader.
///
/// Must match `SnowParams` in `snow_accumulation.wgsl`. 48 bytes, 4-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SnowParamsGpu {
    /// Snow accumulation rate per second (world depth units).
    pub accumulate_rate: f32,
    /// Melt rate per second.
    pub melt_rate: f32,
    /// Frame delta time.
    pub dt: f32,
    /// 1.0 if snow is actively falling, 0.0 otherwise.
    pub snow_active: f32,
    /// Chunk width in texels.
    pub width: u32,
    /// Chunk height in texels.
    pub height: u32,
    /// Minimum slope cosine for accumulation (surfaces steeper than
    /// `acos(min_slope_cos)` shed snow). Typically ~0.5 (60°).
    pub min_slope_cos: f32,
    /// Maximum accumulation depth (caps the heightmap value).
    pub max_depth: f32,
    /// Current temperature in Celsius. Above `melt_threshold`, melt rate
    /// increases proportionally: `effective_melt = melt_rate * max(1.0, (temp - threshold) * melt_sensitivity + 1.0)`.
    /// Below threshold, melt rate uses the base value.
    pub temperature: f32,
    /// Temperature (°C) above which enhanced melting begins. Default: 0.0.
    pub melt_threshold: f32,
    /// How aggressively temperature above the threshold accelerates melting.
    /// Default: 0.5 (each 1°C above threshold adds 50% melt rate).
    pub melt_sensitivity: f32,
    /// Padding for 16-byte alignment.
    pub _pad: f32,
}

impl Default for SnowParamsGpu {
    fn default() -> Self {
        Self {
            accumulate_rate: 0.15,
            melt_rate: 0.03,
            dt: 1.0 / 60.0,
            snow_active: 0.0,
            width: 256,
            height: 256,
            min_slope_cos: 0.5, // ~60° slope limit
            max_depth: 1.0,
            temperature: -5.0,
            melt_threshold: 0.0,
            melt_sensitivity: 0.5,
            _pad: 0.0,
        }
    }
}

// ─── Snow Material Constants ─────────────────────────────────────────────

/// Snow material properties for PBR blending in the fragment shader.
///
/// These constants define how snow appears when blended over terrain/mesh materials.
/// The PBR shader uses `snow_amount` from SceneEnvironmentUBO to interpolate
/// between the base material and these snow properties.
#[derive(Debug, Clone, Copy)]
pub struct SnowMaterial {
    /// Snow albedo (linear RGB). Fresh snow is ~0.9 reflectance.
    pub albedo: [f32; 3],
    /// Snow roughness. Fresh snow is quite rough (~0.8), compacted ~0.6.
    pub roughness: f32,
    /// Threshold of `snow_amount * up_facing` below which no snow appears.
    /// Creates a sharp transition edge. Typically 0.1–0.2.
    pub threshold: f32,
    /// Sharpness of the blend transition. Higher = sharper edge.
    /// `snow_weight = saturate((raw - threshold) * sharpness)`.
    pub sharpness: f32,
}

impl Default for SnowMaterial {
    fn default() -> Self {
        Self {
            albedo: [0.95, 0.96, 0.98], // Slightly blue-white
            roughness: 0.8,
            threshold: 0.1,
            sharpness: 5.0,
        }
    }
}

impl SnowMaterial {
    /// Packed snow — slightly darker, smoother.
    pub fn packed() -> Self {
        Self {
            albedo: [0.85, 0.87, 0.90],
            roughness: 0.6,
            threshold: 0.15,
            sharpness: 4.0,
        }
    }

    /// Light dusting — very thin coverage with soft transition.
    pub fn dusting() -> Self {
        Self {
            albedo: [0.92, 0.93, 0.95],
            roughness: 0.75,
            threshold: 0.3,
            sharpness: 3.0,
        }
    }
}

// ─── CPU-side Configuration ──────────────────────────────────────────────

/// Controls the compute-side snow accumulation behavior.
#[derive(Debug, Clone, Copy)]
pub struct SnowAccumulationConfig {
    /// Rate at which snow accumulates on upward surfaces (depth/second).
    pub accumulate_rate: f32,
    /// Rate at which snow melts when not snowing (depth/second).
    pub melt_rate: f32,
    /// Maximum snow depth before capping.
    pub max_depth: f32,
    /// Minimum slope cosine (dot(N, UP)) to allow accumulation.
    /// 0.0 = all surfaces, 1.0 = perfectly flat only.
    pub min_slope_cos: f32,
    /// Resolution of accumulation heightmap per chunk (square).
    pub map_resolution: u32,
    /// Temperature threshold (°C) above which enhanced melting begins.
    pub melt_threshold: f32,
    /// How aggressively temperature above the threshold accelerates melting.
    /// Each 1°C above threshold multiplies melt rate by (1 + sensitivity).
    pub melt_sensitivity: f32,
}

impl Default for SnowAccumulationConfig {
    fn default() -> Self {
        Self {
            accumulate_rate: 0.15,
            melt_rate: 0.03,
            max_depth: 1.0,
            min_slope_cos: 0.5,
            map_resolution: 256,
            melt_threshold: 0.0,
            melt_sensitivity: 0.5,
        }
    }
}

impl SnowAccumulationConfig {
    /// Build GPU params from this config + runtime state.
    pub fn to_gpu_params(&self, dt: f32, snow_active: bool, temperature: f32) -> SnowParamsGpu {
        SnowParamsGpu {
            accumulate_rate: self.accumulate_rate,
            melt_rate: self.melt_rate,
            dt,
            snow_active: if snow_active { 1.0 } else { 0.0 },
            width: self.map_resolution,
            height: self.map_resolution,
            min_slope_cos: self.min_slope_cos,
            max_depth: self.max_depth,
            temperature,
            melt_threshold: self.melt_threshold,
            melt_sensitivity: self.melt_sensitivity,
            _pad: 0.0,
        }
    }

    /// Arctic environment — fast accumulation, slow melt.
    pub fn arctic() -> Self {
        Self {
            accumulate_rate: 0.25,
            melt_rate: 0.01,
            max_depth: 2.0,
            min_slope_cos: 0.3,
            map_resolution: 256,
            melt_threshold: -5.0,
            melt_sensitivity: 0.3,
        }
    }

    /// Temperate environment — moderate accumulation, faster melt.
    pub fn temperate() -> Self {
        Self {
            accumulate_rate: 0.10,
            melt_rate: 0.08,
            max_depth: 0.5,
            min_slope_cos: 0.6,
            map_resolution: 256,
            melt_threshold: 2.0,
            melt_sensitivity: 0.8,
        }
    }

    /// Compute dispatch workgroup count for the accumulation shader.
    /// Workgroup size is (8, 8), so divide resolution by 8, rounding up.
    pub fn dispatch_size(&self) -> (u32, u32) {
        let wg = 8u32;
        (
            self.map_resolution.div_ceil(wg),
            self.map_resolution.div_ceil(wg),
        )
    }
}

// ─── WGSL Source ─────────────────────────────────────────────────────────

/// Compile-time snow accumulation compute shader source.
pub const SNOW_ACCUMULATION_WGSL: &str = include_str!("../shaders/snow_accumulation.wgsl");

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snow_params_gpu_size() {
        assert_eq!(std::mem::size_of::<SnowParamsGpu>(), 48);
    }

    #[test]
    fn snow_params_gpu_default() {
        let p = SnowParamsGpu::default();
        assert_eq!(p.accumulate_rate, 0.15);
        assert_eq!(p.melt_rate, 0.03);
        assert_eq!(p.snow_active, 0.0);
        assert_eq!(p.width, 256);
        assert_eq!(p.height, 256);
        assert_eq!(p.min_slope_cos, 0.5);
        assert_eq!(p.max_depth, 1.0);
    }

    #[test]
    fn snow_params_gpu_bytemuck_roundtrip() {
        let p = SnowParamsGpu {
            accumulate_rate: 0.2,
            melt_rate: 0.05,
            dt: 0.016,
            snow_active: 1.0,
            width: 512,
            height: 512,
            min_slope_cos: 0.4,
            max_depth: 2.0,
            temperature: -5.0,
            melt_threshold: 0.0,
            melt_sensitivity: 0.5,
            _pad: 0.0,
        };
        let bytes = bytemuck::bytes_of(&p);
        assert_eq!(bytes.len(), 48);
        let back: &SnowParamsGpu = bytemuck::from_bytes(bytes);
        assert_eq!(back.width, 512);
        assert_eq!(back.snow_active, 1.0);
        assert_eq!(back.temperature, -5.0);
    }

    #[test]
    fn snow_material_default() {
        let m = SnowMaterial::default();
        assert!(m.albedo[0] > 0.9);
        assert!(m.roughness > 0.5);
        assert!(m.threshold > 0.0 && m.threshold < 0.5);
        assert!(m.sharpness > 1.0);
    }

    #[test]
    fn snow_material_packed_darker() {
        let fresh = SnowMaterial::default();
        let packed = SnowMaterial::packed();
        assert!(packed.albedo[0] < fresh.albedo[0]);
        assert!(packed.roughness < fresh.roughness);
    }

    #[test]
    fn snow_material_dusting_higher_threshold() {
        let fresh = SnowMaterial::default();
        let dust = SnowMaterial::dusting();
        assert!(dust.threshold > fresh.threshold);
    }

    #[test]
    fn snow_config_default() {
        let c = SnowAccumulationConfig::default();
        assert_eq!(c.accumulate_rate, 0.15);
        assert_eq!(c.melt_rate, 0.03);
        assert_eq!(c.map_resolution, 256);
    }

    #[test]
    fn snow_config_to_gpu_active() {
        let c = SnowAccumulationConfig::default();
        let g = c.to_gpu_params(0.016, true, -5.0);
        assert_eq!(g.snow_active, 1.0);
        assert!((g.dt - 0.016).abs() < 1e-6);
        assert_eq!(g.accumulate_rate, c.accumulate_rate);
    }

    #[test]
    fn snow_config_to_gpu_inactive() {
        let c = SnowAccumulationConfig::default();
        let g = c.to_gpu_params(0.016, false, -5.0);
        assert_eq!(g.snow_active, 0.0);
    }

    #[test]
    fn snow_config_arctic_fast_accumulate() {
        let a = SnowAccumulationConfig::arctic();
        let d = SnowAccumulationConfig::default();
        assert!(a.accumulate_rate > d.accumulate_rate);
        assert!(a.melt_rate < d.melt_rate);
        assert!(a.max_depth > d.max_depth);
    }

    #[test]
    fn snow_config_temperate_fast_melt() {
        let t = SnowAccumulationConfig::temperate();
        let d = SnowAccumulationConfig::default();
        assert!(t.melt_rate > d.melt_rate);
        assert!(t.accumulate_rate < d.accumulate_rate);
    }

    #[test]
    fn snow_config_dispatch_size() {
        let c = SnowAccumulationConfig::default(); // 256
        assert_eq!(c.dispatch_size(), (32, 32)); // 256/8

        let mut c2 = c;
        c2.map_resolution = 100;
        assert_eq!(c2.dispatch_size(), (13, 13)); // ceil(100/8) = 13
    }

    #[test]
    fn snow_wgsl_parses() {
        assert!(SNOW_ACCUMULATION_WGSL.contains("update_snow"));
        assert!(SNOW_ACCUMULATION_WGSL.contains("SnowParams"));
        assert!(SNOW_ACCUMULATION_WGSL.contains("accumulation"));
    }

    #[test]
    fn snow_wgsl_has_override_workgroup_size() {
        assert!(SNOW_ACCUMULATION_WGSL.contains("override WG_X: u32 = 8u;"));
        assert!(SNOW_ACCUMULATION_WGSL.contains("override WG_Y: u32 = 8u;"));
        assert!(SNOW_ACCUMULATION_WGSL.contains("@workgroup_size(WG_X, WG_Y)"));
    }

    #[test]
    fn snow_config_dispatch_size_exact_multiple() {
        let mut c = SnowAccumulationConfig::default();
        c.map_resolution = 64;
        assert_eq!(c.dispatch_size(), (8, 8));
    }

    #[test]
    fn snow_config_dispatch_size_one() {
        let mut c = SnowAccumulationConfig::default();
        c.map_resolution = 1;
        assert_eq!(c.dispatch_size(), (1, 1));
    }
}
