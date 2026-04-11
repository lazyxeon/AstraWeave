//! Rain occlusion compute pipeline.
//!
//! Post-process pass that kills weather particles occluded by scene geometry.
//! Runs after the GPU particle simulation pass on the resulting particle buffer.
//!
//! Uses the previous frame's depth buffer to screen-space test each particle:
//! if the particle's projected depth is behind the scene depth at that pixel,
//! the particle is killed (alpha set to 0, age maxed out).

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

/// Occlusion test parameters (128 bytes, 16-byte aligned).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct OcclusionParams {
    /// View-projection matrix for projecting particles to screen space.
    pub view_proj: [[f32; 4]; 4],
    /// Screen resolution: width, height.
    pub screen_size: [f32; 2],
    /// Number of active particles.
    pub particle_count: u32,
    /// Small depth bias to prevent self-occlusion (e.g. 0.0005).
    pub depth_bias: f32,
}

impl OcclusionParams {
    /// Build from camera state.
    pub fn new(
        view_proj: &Mat4,
        screen_width: u32,
        screen_height: u32,
        particle_count: u32,
        depth_bias: f32,
    ) -> Self {
        Self {
            view_proj: view_proj.to_cols_array_2d(),
            screen_size: [screen_width as f32, screen_height as f32],
            particle_count,
            depth_bias,
        }
    }
}

/// Compute shader source (loaded at compile time).
const OCCLUSION_SHADER_SRC: &str = include_str!("../shaders/particles/rain_occlusion.wgsl");

/// GPU pipeline for rain/weather particle occlusion testing.
pub struct RainOcclusionPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,
    depth_sampler: wgpu::Sampler,
}

impl RainOcclusionPipeline {
    /// Create the rain occlusion compute pipeline.
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rain_occlusion.wgsl"),
            source: wgpu::ShaderSource::Wgsl(OCCLUSION_SHADER_SRC.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RainOcclusion BGL"),
            entries: &[
                // 0: Particle buffer (storage, read-write)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: OcclusionParams uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: Scene depth texture (previous frame)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 3: Depth sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RainOcclusion PipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RainOcclusion Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("rain_occlusion"),
            compilation_options: Default::default(),
            cache: None,
        });

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RainOcclusion Params"),
            size: std::mem::size_of::<OcclusionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("RainOcclusion DepthSampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            pipeline,
            bind_group_layout,
            params_buffer,
            depth_sampler,
        })
    }

    /// Dispatch the occlusion test pass.
    ///
    /// * `particle_buffer` — the GPU particle storage buffer (read-write).
    ///   This is typically `GpuParticleSystem::particle_buffer()`.
    /// * `depth_view` — previous frame's depth buffer texture view.
    /// * `params` — occlusion test parameters.
    pub fn dispatch(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        particle_buffer: &wgpu::Buffer,
        depth_view: &wgpu::TextureView,
        params: &OcclusionParams,
    ) {
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(params));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RainOcclusion BG"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                },
            ],
        });

        let workgroups = params.particle_count.div_ceil(64);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("RainOcclusion Pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occlusion_params_size() {
        // mat4 (64) + vec2 + u32 + f32 = 64 + 8 + 4 + 4 = 80
        // But our struct has [[f32;4];4] (64) + [f32;2] (8) + u32 (4) + f32 (4) = 80
        assert_eq!(std::mem::size_of::<OcclusionParams>(), 80);
    }

    #[test]
    fn test_occlusion_params_new() {
        let vp = Mat4::IDENTITY;
        let params = OcclusionParams::new(&vp, 1920, 1080, 5000, 0.001);
        assert_eq!(params.screen_size, [1920.0, 1080.0]);
        assert_eq!(params.particle_count, 5000);
        assert_eq!(params.depth_bias, 0.001);
    }

    #[test]
    fn test_occlusion_shader_parses() {
        let module = naga::front::wgsl::parse_str(OCCLUSION_SHADER_SRC)
            .expect("rain occlusion WGSL should parse");
        assert!(
            module
                .entry_points
                .iter()
                .any(|e| e.name == "rain_occlusion"),
            "must have rain_occlusion entry point"
        );
    }

    #[test]
    fn test_occlusion_params_view_proj_identity() {
        let vp = Mat4::IDENTITY;
        let params = OcclusionParams::new(&vp, 800, 600, 100, 0.0);
        // Identity view_proj: diagonal should be 1.0
        assert_eq!(params.view_proj[0][0], 1.0);
        assert_eq!(params.view_proj[1][1], 1.0);
        assert_eq!(params.view_proj[2][2], 1.0);
        assert_eq!(params.view_proj[3][3], 1.0);
    }
}
