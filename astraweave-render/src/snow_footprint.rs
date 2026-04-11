//! Snow footprint stamp system — entity deformation of accumulated snow.
//!
//! Stamps entity foot positions into a snow depression map that is
//! subtracted from the snow accumulation during PBR evaluation.
//! Footprints slowly recover (snow settling) via a per-frame recovery pass.
//!
//! Uses the same world-space top-down texture approach as `vegetation_interaction`.

use bytemuck::{Pod, Zeroable};

// ── Constants ───────────────────────────────────────────────────────────────

const SNOW_FOOTPRINT_WGSL: &str = include_str!("../shaders/snow_footprint.wgsl");

const DEFAULT_TEX_SIZE: u32 = 128;

/// Maximum entity stamps per frame (matches entity buffer allocation).
pub const MAX_ENTITIES: usize = 64;

// ── GPU structs ─────────────────────────────────────────────────────────────

/// Footprint stamp parameters (48 bytes, matches WGSL `FootprintParams`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FootprintParams {
    pub camera_x: f32,
    pub camera_z: f32,
    pub stamp_radius: f32,
    pub stamp_depth: f32,
    pub recovery_rate: f32,
    pub dt: f32,
    pub entity_count: u32,
    pub tex_size: f32,
    pub world_extent: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl Default for FootprintParams {
    fn default() -> Self {
        Self {
            camera_x: 0.0,
            camera_z: 0.0,
            stamp_radius: 0.3,
            stamp_depth: 0.15,
            recovery_rate: 0.02,
            dt: 1.0 / 60.0,
            entity_count: 0,
            tex_size: DEFAULT_TEX_SIZE as f32,
            world_extent: 16.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

/// Entity foot position (8 bytes, matches WGSL `EntityFoot`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct EntityFoot {
    pub x: f32,
    pub z: f32,
}

// ── Configuration ───────────────────────────────────────────────────────────

/// CPU-side snow footprint configuration.
#[derive(Debug, Clone, Copy)]
pub struct SnowFootprintConfig {
    /// Foot contact radius in world units.
    pub stamp_radius: f32,
    /// Maximum depression depth.
    pub stamp_depth: f32,
    /// Rate at which footprints fill back in (depth/second).
    pub recovery_rate: f32,
    /// World-space extent per axis from camera center.
    pub world_extent: f32,
    /// Footprint texture resolution (square).
    pub tex_resolution: u32,
}

impl Default for SnowFootprintConfig {
    fn default() -> Self {
        Self {
            stamp_radius: 0.3,
            stamp_depth: 0.15,
            recovery_rate: 0.02,
            world_extent: 16.0,
            tex_resolution: DEFAULT_TEX_SIZE,
        }
    }
}

impl SnowFootprintConfig {
    /// Build GPU params from config + runtime state.
    pub fn to_gpu_params(
        &self,
        camera_xz: [f32; 2],
        entity_count: u32,
        dt: f32,
    ) -> FootprintParams {
        FootprintParams {
            camera_x: camera_xz[0],
            camera_z: camera_xz[1],
            stamp_radius: self.stamp_radius,
            stamp_depth: self.stamp_depth,
            recovery_rate: self.recovery_rate,
            dt,
            entity_count,
            tex_size: self.tex_resolution as f32,
            world_extent: self.world_extent,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }

    /// Deep snow preset — larger footprints, slower recovery.
    pub fn deep_snow() -> Self {
        Self {
            stamp_radius: 0.35,
            stamp_depth: 0.25,
            recovery_rate: 0.005,
            world_extent: 20.0,
            tex_resolution: 256,
        }
    }

    /// Light snow preset — shallow prints, fast recovery.
    pub fn light_snow() -> Self {
        Self {
            stamp_radius: 0.25,
            stamp_depth: 0.08,
            recovery_rate: 0.05,
            world_extent: 12.0,
            tex_resolution: 128,
        }
    }

    /// Compute dispatch workgroup count.
    pub fn dispatch_size(&self) -> (u32, u32) {
        let wg = 8u32;
        (
            self.tex_resolution.div_ceil(wg),
            self.tex_resolution.div_ceil(wg),
        )
    }
}

// ── WGSL Source ─────────────────────────────────────────────────────────────

/// Compile-time snow footprint shader source.
pub const SNOW_FOOTPRINT_SHADER: &str = SNOW_FOOTPRINT_WGSL;

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footprint_params_size() {
        assert_eq!(std::mem::size_of::<FootprintParams>(), 48);
    }

    #[test]
    fn entity_foot_size() {
        assert_eq!(std::mem::size_of::<EntityFoot>(), 8);
    }

    #[test]
    fn footprint_params_default() {
        let p = FootprintParams::default();
        assert!(p.stamp_radius > 0.0);
        assert!(p.stamp_depth > 0.0);
        assert!(p.recovery_rate > 0.0);
        assert_eq!(p.entity_count, 0);
    }

    #[test]
    fn footprint_params_bytemuck_roundtrip() {
        let p = FootprintParams {
            camera_x: 10.0,
            camera_z: 20.0,
            stamp_radius: 0.3,
            stamp_depth: 0.15,
            recovery_rate: 0.02,
            dt: 0.016,
            entity_count: 5,
            tex_size: 128.0,
            world_extent: 16.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let bytes = bytemuck::bytes_of(&p);
        assert_eq!(bytes.len(), 48);
        let back: &FootprintParams = bytemuck::from_bytes(bytes);
        assert!((back.camera_x - 10.0).abs() < f32::EPSILON);
        assert_eq!(back.entity_count, 5);
    }

    #[test]
    fn footprint_config_to_gpu_params() {
        let cfg = SnowFootprintConfig::default();
        let params = cfg.to_gpu_params([5.0, 10.0], 3, 0.016);
        assert!((params.camera_x - 5.0).abs() < f32::EPSILON);
        assert_eq!(params.entity_count, 3);
    }

    #[test]
    fn footprint_config_presets() {
        let deep = SnowFootprintConfig::deep_snow();
        let light = SnowFootprintConfig::light_snow();
        assert!(deep.stamp_depth > light.stamp_depth);
        assert!(deep.recovery_rate < light.recovery_rate);
    }

    #[test]
    fn footprint_dispatch_size() {
        let cfg = SnowFootprintConfig::default();
        let (x, y) = cfg.dispatch_size();
        assert_eq!(x, 16); // 128 / 8
        assert_eq!(y, 16);
    }

    #[test]
    fn footprint_wgsl_present() {
        assert!(SNOW_FOOTPRINT_WGSL.contains("recover"));
        assert!(SNOW_FOOTPRINT_WGSL.contains("stamp_footprint"));
        assert!(SNOW_FOOTPRINT_WGSL.contains("FootprintParams"));
    }
}
