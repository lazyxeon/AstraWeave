//! Lumen Global Illumination Orchestrator.
//!
//! Coordinates the full Lumen-style GI pipeline:
//!
//! 1. **Surface Cache Update** — incrementally refresh SH probe grid
//! 2. **DFAO** — cone-traced long-range ambient occlusion from SDF
//! 3. **SSGI** — screen-space indirect diffuse (managed externally, composited here)
//! 4. **Final Gather** — composite all sources with temporal reprojection
//!
//! The orchestrator owns the surface cache, distance field, and final gather passes,
//! while SSGI and SSR are managed by their respective modules and fed in as inputs.

use glam::{Mat4, UVec3, Vec3};

use crate::distance_field::{DfaoConfig, DfaoPass, SdfConfig, SdfVolume};
use crate::final_gather::{FinalGatherConfig, FinalGatherPass};
use crate::surface_cache::{DirectionalLightGpu, SurfaceCacheConfig, SurfaceCachePass};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Master configuration for the Lumen GI system.
#[derive(Debug, Clone)]
pub struct LumenConfig {
    pub enabled: bool,
    pub surface_cache: SurfaceCacheConfig,
    pub sdf: SdfConfig,
    pub dfao: DfaoConfig,
    pub final_gather: FinalGatherConfig,
}

impl Default for LumenConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            surface_cache: SurfaceCacheConfig::default(),
            sdf: SdfConfig::default(),
            dfao: DfaoConfig::default(),
            final_gather: FinalGatherConfig::default(),
        }
    }
}

/// Quality presets for Lumen GI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LumenQuality {
    /// Low: 8×4×8 probes, 4×4×4 SDF, 8 DFAO steps
    Low,
    /// Medium: 16×8×16 probes, 32×16×32 SDF, 12 DFAO steps (default)
    Medium,
    /// High: 32×16×32 probes, 64×32×64 SDF, 16 DFAO steps
    High,
    /// Epic: 48×24×48 probes, 128×64×128 SDF, 24 DFAO steps
    Epic,
}

impl LumenQuality {
    /// Create a `LumenConfig` from a quality preset.
    pub fn to_config(self) -> LumenConfig {
        match self {
            LumenQuality::Low => LumenConfig {
                surface_cache: SurfaceCacheConfig {
                    grid_dims: UVec3::new(8, 4, 8),
                    probe_spacing: 8.0,
                    update_fraction: 0.25,
                    ..Default::default()
                },
                sdf: SdfConfig {
                    dims: [16, 8, 16],
                    ..Default::default()
                },
                dfao: DfaoConfig {
                    num_steps: 8,
                    max_distance: 20.0,
                    ..Default::default()
                },
                final_gather: FinalGatherConfig {
                    ssgi_weight: 0.7,
                    probe_weight: 0.3,
                    ..Default::default()
                },
                ..Default::default()
            },
            LumenQuality::Medium => LumenConfig::default(),
            LumenQuality::High => LumenConfig {
                surface_cache: SurfaceCacheConfig {
                    grid_dims: UVec3::new(32, 16, 32),
                    probe_spacing: 2.0,
                    update_fraction: 0.0625,
                    ..Default::default()
                },
                sdf: SdfConfig {
                    dims: [64, 32, 64],
                    ..Default::default()
                },
                dfao: DfaoConfig {
                    num_steps: 16,
                    max_distance: 40.0,
                    ..Default::default()
                },
                ..Default::default()
            },
            LumenQuality::Epic => LumenConfig {
                surface_cache: SurfaceCacheConfig {
                    grid_dims: UVec3::new(48, 24, 48),
                    probe_spacing: 1.5,
                    update_fraction: 0.04,
                    ..Default::default()
                },
                sdf: SdfConfig {
                    dims: [128, 64, 128],
                    ..Default::default()
                },
                dfao: DfaoConfig {
                    num_steps: 24,
                    max_distance: 50.0,
                    ..Default::default()
                },
                final_gather: FinalGatherConfig {
                    ssgi_weight: 0.5,
                    probe_weight: 0.5,
                    dfao_weight: 0.9,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Lumen Orchestrator
// ---------------------------------------------------------------------------

/// The Lumen GI orchestrator. Owns and coordinates all GI sub-passes.
pub struct LumenGI {
    config: LumenConfig,
    surface_cache: SurfaceCachePass,
    dfao: DfaoPass,
    final_gather: FinalGatherPass,
    sdf_volume: SdfVolume,
    sdf_dirty: bool,
}

impl LumenGI {
    /// Create the full Lumen GI pipeline.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, config: LumenConfig) -> Self {
        let surface_cache = SurfaceCachePass::new(device, config.surface_cache.clone());
        let dfao = DfaoPass::new(device, width, height, config.sdf.clone());
        let final_gather = FinalGatherPass::new(device, width, height);
        let sdf_volume = SdfVolume::new(config.sdf.clone());

        Self {
            config,
            surface_cache,
            dfao,
            final_gather,
            sdf_volume,
            sdf_dirty: true, // force initial upload
        }
    }

    /// Create with a quality preset.
    pub fn with_quality(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        quality: LumenQuality,
    ) -> Self {
        Self::new(device, width, height, quality.to_config())
    }

    pub fn config(&self) -> &LumenConfig {
        &self.config
    }

    pub fn surface_cache(&self) -> &SurfaceCachePass {
        &self.surface_cache
    }

    pub fn dfao_pass(&self) -> &DfaoPass {
        &self.dfao
    }

    pub fn final_gather_pass(&self) -> &FinalGatherPass {
        &self.final_gather
    }

    /// Get the final indirect lighting output view.
    pub fn output_view(&self) -> &wgpu::TextureView {
        self.final_gather.output_view()
    }

    /// Get the DFAO output view.
    pub fn dfao_view(&self) -> &wgpu::TextureView {
        self.dfao.ao_view()
    }

    /// Update the SDF volume from scene geometry (bounding boxes).
    /// Call this when geometry changes (not every frame).
    pub fn update_sdf(&mut self, boxes: &[crate::distance_field::SdfBox]) {
        self.sdf_volume.bake_from_boxes(boxes);
        self.sdf_dirty = true;
    }

    /// Upload directional light data for the surface cache.
    pub fn upload_lights(&self, queue: &wgpu::Queue, lights: &[DirectionalLightGpu]) {
        self.surface_cache.upload_lights(queue, lights);
    }

    /// Prepare all sub-passes for this frame.
    pub fn prepare_frame(
        &mut self,
        queue: &wgpu::Queue,
        inv_view_proj: Mat4,
        view_pos: Vec3,
        near: f32,
        far: f32,
    ) {
        if !self.config.enabled {
            return;
        }

        // Upload SDF if dirty
        if self.sdf_dirty {
            self.dfao.upload_sdf(queue, &self.sdf_volume);
            self.sdf_dirty = false;
        }

        self.surface_cache.prepare_frame(queue);
        self.dfao
            .update_params(queue, inv_view_proj, view_pos, near, far);
        self.final_gather.update_params(
            queue,
            inv_view_proj,
            self.config.surface_cache.grid_origin.to_array(),
            self.config.surface_cache.probe_spacing,
            self.config.surface_cache.grid_dims,
            near,
            far,
        );
    }

    /// Execute the full Lumen GI pipeline for this frame.
    ///
    /// Pass order: Surface Cache → DFAO → Final Gather → Copy History
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        albedo_view: &wgpu::TextureView,
        ssgi_view: &wgpu::TextureView,
        velocity_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        // 1. Surface cache update (probe grid refresh)
        self.surface_cache
            .execute(device, encoder, depth_view, albedo_view, resource_gen);

        // 2. DFAO (distance-field ambient occlusion)
        self.dfao
            .execute(device, encoder, depth_view, normal_view, resource_gen);

        // 3. Final gather (composite SSGI + probes + DFAO)
        self.final_gather.execute(
            device,
            encoder,
            depth_view,
            normal_view,
            albedo_view,
            ssgi_view,
            self.dfao.ao_view(),
            velocity_view,
            self.surface_cache.probe_buffer(),
            resource_gen,
        );

        // 4. Copy output to history for next frame
        self.final_gather.copy_to_history(encoder);
    }

    /// Resize all sub-pass textures.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.dfao.resize(device, width, height);
        self.final_gather.resize(device, width, height);
    }

    /// Update DFAO configuration.
    pub fn set_dfao_config(&mut self, config: DfaoConfig) {
        self.dfao.set_config(config);
    }

    /// Update final gather configuration.
    pub fn set_final_gather_config(&mut self, config: FinalGatherConfig) {
        self.final_gather.set_config(config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let c = LumenConfig::default();
        assert!(c.enabled);
        assert_eq!(c.surface_cache.grid_dims, UVec3::new(16, 8, 16));
    }

    #[test]
    fn quality_presets() {
        let low = LumenQuality::Low.to_config();
        let med = LumenQuality::Medium.to_config();
        let high = LumenQuality::High.to_config();
        let epic = LumenQuality::Epic.to_config();

        assert!(low.surface_cache.total_probes() < med.surface_cache.total_probes());
        assert!(med.surface_cache.total_probes() < high.surface_cache.total_probes());
        assert!(high.surface_cache.total_probes() < epic.surface_cache.total_probes());

        assert!(low.dfao.num_steps < med.dfao.num_steps);
        assert!(med.dfao.num_steps < high.dfao.num_steps);
        assert!(high.dfao.num_steps < epic.dfao.num_steps);
    }

    #[test]
    fn quality_preset_probe_counts() {
        assert_eq!(
            LumenQuality::Low.to_config().surface_cache.total_probes(),
            256
        );
        assert_eq!(
            LumenQuality::Medium
                .to_config()
                .surface_cache
                .total_probes(),
            2048
        );
        assert_eq!(
            LumenQuality::High.to_config().surface_cache.total_probes(),
            16384
        );
        assert_eq!(
            LumenQuality::Epic.to_config().surface_cache.total_probes(),
            55296
        );
    }

    #[test]
    fn lumen_gi_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let config = LumenConfig {
            surface_cache: SurfaceCacheConfig {
                grid_dims: UVec3::new(4, 4, 4),
                ..Default::default()
            },
            sdf: SdfConfig {
                dims: [8, 8, 8],
                ..Default::default()
            },
            ..Default::default()
        };
        let lumen = LumenGI::new(&device, 640, 480, config);
        assert_eq!(lumen.surface_cache().total_probes(), 64);
    }

    #[test]
    fn lumen_gi_with_quality() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let lumen = LumenGI::with_quality(&device, 640, 480, LumenQuality::Low);
        assert_eq!(lumen.surface_cache().total_probes(), 256);
    }

    #[test]
    fn sdf_update() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let config = LumenConfig {
            surface_cache: SurfaceCacheConfig {
                grid_dims: UVec3::new(2, 2, 2),
                ..Default::default()
            },
            sdf: SdfConfig {
                dims: [4, 4, 4],
                ..Default::default()
            },
            ..Default::default()
        };
        let mut lumen = LumenGI::new(&device, 320, 240, config);

        let boxes = vec![crate::distance_field::SdfBox {
            center: Vec3::ZERO,
            half_extents: Vec3::ONE,
        }];
        lumen.update_sdf(&boxes);
        // sdf_dirty should be true after update
        assert!(lumen.sdf_dirty);
    }
}
