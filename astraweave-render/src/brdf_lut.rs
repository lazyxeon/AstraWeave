//! BRDF Integration LUT for split-sum IBL approximation.
//!
//! Precomputes a 2D lookup table parameterized by (NdotV, roughness) that stores
//! the integral of the GGX BRDF split into (scale, bias) terms:
//!   `specular = prefiltered_color * (F0 * scale + bias)`
//!
//! This LUT is resolution-independent and only needs computation once (or when
//! sample count changes).

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// Uniform parameters for BRDF LUT generation.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BrdfLutParams {
    pub lut_size: u32,
    pub num_samples: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the BRDF LUT.
#[derive(Debug, Clone, Copy)]
pub struct BrdfLutConfig {
    /// LUT resolution (square). Default: 256.
    pub size: u32,
    /// Number of importance samples per pixel. Default: 1024.
    pub num_samples: u32,
}

impl Default for BrdfLutConfig {
    fn default() -> Self {
        Self {
            size: 256,
            num_samples: 1024,
        }
    }
}

// ---------------------------------------------------------------------------
// BRDF LUT Pass
// ---------------------------------------------------------------------------

/// Manages GPU resources for BRDF LUT generation.
pub struct BrdfLutPass {
    config: BrdfLutConfig,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    params_buf: wgpu::Buffer,
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    lut_texture: wgpu::Texture,
    lut_view: wgpu::TextureView,
    generated: bool,
}

impl BrdfLutPass {
    pub fn new(device: &wgpu::Device, config: BrdfLutConfig) -> Self {
        let fmt = wgpu::TextureFormat::Rgba16Float;

        let lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("brdf_lut"),
            size: wgpu::Extent3d {
                width: config.size,
                height: config.size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let lut_view = lut_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let params = BrdfLutParams {
            lut_size: config.size,
            num_samples: config.num_samples,
            _pad0: 0,
            _pad1: 0,
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("brdf_lut_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brdf_lut_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: fmt,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("brdf_lut_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/pbr/brdf_lut.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("brdf_lut_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("brdf_lut_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("brdf_lut_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            pipeline,
            bgl,
            params_buf,
            lut_texture,
            lut_view,
            generated: false,
        }
    }

    /// Get the BRDF LUT texture view for binding in material shaders.
    pub fn lut_view(&self) -> &wgpu::TextureView {
        &self.lut_view
    }

    /// Whether the LUT has been generated.
    pub fn is_generated(&self) -> bool {
        self.generated
    }

    pub fn config(&self) -> &BrdfLutConfig {
        &self.config
    }

    /// Generate the BRDF LUT. Only needs to be called once.
    pub fn generate(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        if self.generated {
            return;
        }

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brdf_lut_bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.lut_view),
                },
            ],
        });

        let wg = self.config.size.div_ceil(8);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("brdf_lut_gen"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(wg, wg, 1);

        self.generated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brdf_lut_params_size() {
        assert_eq!(std::mem::size_of::<BrdfLutParams>(), 16);
    }

    #[test]
    fn default_config() {
        let c = BrdfLutConfig::default();
        assert_eq!(c.size, 256);
        assert_eq!(c.num_samples, 1024);
    }

    #[test]
    fn brdf_lut_pass_creation() {
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

        let config = BrdfLutConfig {
            size: 32,
            num_samples: 64,
        };
        let pass = BrdfLutPass::new(&device, config);
        assert!(!pass.is_generated());
        assert_eq!(pass.config().size, 32);
    }

    #[test]
    fn hammersley_cpu_validation() {
        // Validate the radical inverse Van der Corput sequence
        fn radical_inverse_vdc(mut bits: u32) -> f32 {
            bits = (bits << 16) | (bits >> 16);
            bits = ((bits & 0x55555555) << 1) | ((bits & 0xAAAAAAAA) >> 1);
            bits = ((bits & 0x33333333) << 2) | ((bits & 0xCCCCCCCC) >> 2);
            bits = ((bits & 0x0F0F0F0F) << 4) | ((bits & 0xF0F0F0F0) >> 4);
            bits = ((bits & 0x00FF00FF) << 8) | ((bits & 0xFF00FF00) >> 8);
            bits as f32 * 2.328_306_4e-10
        }

        // First few values should be well-distributed in [0, 1)
        let v0 = radical_inverse_vdc(0);
        let v1 = radical_inverse_vdc(1);
        let v2 = radical_inverse_vdc(2);
        assert!(v0 >= 0.0 && v0 < 1.0);
        assert!(v1 >= 0.0 && v1 < 1.0);
        assert!(v2 >= 0.0 && v2 < 1.0);
        // They should be different
        assert!((v0 - v1).abs() > 0.01);
        assert!((v1 - v2).abs() > 0.01);
    }

    #[test]
    fn importance_sample_ggx_cpu() {
        // Validate GGX importance sampling produces valid directions
        fn importance_sample_ggx(xi: [f32; 2], roughness: f32) -> [f32; 3] {
            let a = roughness * roughness;
            let phi = 2.0 * std::f32::consts::PI * xi[0];
            let cos_theta = ((1.0 - xi[1]) / (1.0 + (a * a - 1.0) * xi[1])).sqrt();
            let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
            [phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta]
        }

        let h = importance_sample_ggx([0.5, 0.5], 0.5);
        let len = (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-5, "Should be unit vector: {len}");
        assert!(h[2] >= 0.0, "z should be positive (hemisphere)");
    }
}
