//! Procedural grass blade rendering pass.
//!
//! Renders grass as procedural 3-vertex triangle blades rather than instanced
//! meshes. Each blade is a single triangle (bottom-left, bottom-right, tip)
//! with per-blade variation in height, width, facing direction, and color tint.
//!
//! Wind animation is driven by `GrassParams.time` + world-position-based phase.
//!
//! The pass reads from a `GrassInstance` storage buffer and draws N blades via
//! `draw(3, instance_count)` — no index buffer required.

use anyhow::Result;
use bytemuck::{Pod, Zeroable};

use crate::vegetation_gpu::VegetationInstanceGpu;

// ── Constants ───────────────────────────────────────────────────────────────

/// WGSL shader source (constants.wgsl + grass_blade.wgsl).
const GRASS_BLADE_SHADER: &str = concat!(
    include_str!("../shaders/constants.wgsl"),
    include_str!("../shaders/grass_blade.wgsl"),
);

/// Default interaction stamp texture size (1×1 when no interaction is active).
const INTERACTION_FALLBACK_SIZE: u32 = 1;

// ── GPU structs (must match WGSL layout) ────────────────────────────────────

/// Per-blade instance data (32 bytes, matches WGSL `GrassInstance`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GrassInstance {
    /// xyz = world position, w = blade height.
    pub pos_height: [f32; 4],
    /// xy = facing direction (normalised XZ), z = blade width, w = tint (0..1).
    pub dir_width: [f32; 4],
}

/// Grass rendering parameters (32 bytes, matches WGSL `GrassParams`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GrassParams {
    pub time: f32,
    pub wind_strength: f32,
    pub wind_dir_x: f32,
    pub wind_dir_z: f32,
    pub interaction_radius: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl Default for GrassParams {
    fn default() -> Self {
        Self {
            time: 0.0,
            wind_strength: 1.0,
            wind_dir_x: std::f32::consts::FRAC_1_SQRT_2,
            wind_dir_z: std::f32::consts::FRAC_1_SQRT_2,
            interaction_radius: 8.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

/// Camera uniforms for grass blade shader (matches WGSL `CameraUniforms`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GrassCameraUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4],
    pub camera_pos: [f32; 4],
}

// ── Pipeline ────────────────────────────────────────────────────────────────

/// Manages the procedural grass blade render pipeline.
pub struct GrassBladePass {
    render_pipeline: wgpu::RenderPipeline,
    camera_bgl: wgpu::BindGroupLayout,
    interaction_bgl: wgpu::BindGroupLayout,

    // Uniform buffers
    camera_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,

    // Blade instance storage buffer
    blade_buffer: wgpu::Buffer,
    max_blades: u32,

    // Fallback interaction texture (1×1 black) + sampler
    interaction_fallback_view: wgpu::TextureView,
    interaction_sampler: wgpu::Sampler,
}

impl GrassBladePass {
    /// Create the grass blade rendering pass.
    ///
    /// `color_format`: the render target format (typically `Rgba16Float` or `Bgra8UnormSrgb`).
    /// `depth_format`: the depth buffer format (typically `Depth32Float`).
    /// `max_blades`: maximum number of grass blade instances.
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        max_blades: u32,
    ) -> Result<Self> {
        // ── Shader ──────────────────────────────────────────────────────────
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grass_blade.wgsl"),
            source: wgpu::ShaderSource::Wgsl(GRASS_BLADE_SHADER.into()),
        });

        // ── Bind group layouts ──────────────────────────────────────────────

        // Group 0: Camera + Params + Blade storage
        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GrassBlade Camera BGL"),
            entries: &[
                // 0: CameraUniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: GrassParams
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: GrassInstance storage buffer (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Group 1: Interaction stamp texture + sampler
        let interaction_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GrassBlade Interaction BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // ── Pipeline layout ─────────────────────────────────────────────────
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GrassBlade PipelineLayout"),
            bind_group_layouts: &[&camera_bgl, &interaction_bgl],
            push_constant_ranges: &[],
        });

        // ── Render pipeline ─────────────────────────────────────────────────
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GrassBlade Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // No vertex buffers — procedural from instance storage
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Double-sided grass blades
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: None, // Opaque grass
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        // ── Buffers ─────────────────────────────────────────────────────────
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrassBlade Camera UBO"),
            size: std::mem::size_of::<GrassCameraUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrassBlade Params UBO"),
            size: std::mem::size_of::<GrassParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let blade_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrassBlade Instances"),
            size: max_blades as u64 * std::mem::size_of::<GrassInstance>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Interaction fallback (1×1 black texture) ────────────────────────
        let fallback_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GrassBlade InteractionFallback"),
            size: wgpu::Extent3d {
                width: INTERACTION_FALLBACK_SIZE,
                height: INTERACTION_FALLBACK_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let interaction_fallback_view =
            fallback_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let interaction_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("GrassBlade InteractionSampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            render_pipeline,
            camera_bgl,
            interaction_bgl,
            camera_buffer,
            params_buffer,
            blade_buffer,
            max_blades,
            interaction_fallback_view,
            interaction_sampler,
        })
    }

    /// Convert vegetation instances (from scatter pipeline) to grass blade instances.
    ///
    /// Filters instances by `type_index` matching any of `grass_type_ids`, then
    /// generates per-blade data with deterministic variation from the instance
    /// position hash.
    pub fn convert_vegetation_to_blades(
        instances: &[VegetationInstanceGpu],
        grass_type_ids: &[u32],
        default_blade_height: f32,
        default_blade_width: f32,
    ) -> Vec<GrassInstance> {
        let mut blades = Vec::with_capacity(instances.len());

        for inst in instances {
            let type_id = inst.rot_type_normal[1] as u32;
            if !grass_type_ids.contains(&type_id) {
                continue;
            }

            let pos = [inst.pos_scale[0], inst.pos_scale[1], inst.pos_scale[2]];
            let scale = inst.pos_scale[3];
            let rotation = inst.rot_type_normal[0];

            // Facing direction from rotation angle.
            let (sin_r, cos_r) = rotation.sin_cos();
            let facing_x = cos_r;
            let facing_z = sin_r;

            // Tint variation from position hash.
            let h = fract_hash(pos[0] * 127.1 + pos[2] * 311.7);

            blades.push(GrassInstance {
                pos_height: [pos[0], pos[1], pos[2], default_blade_height * scale],
                dir_width: [facing_x, facing_z, default_blade_width * scale, h],
            });
        }

        blades
    }

    /// Upload blade instances and render parameters, then draw.
    ///
    /// Call this during the render pass that writes to the color + depth targets.
    /// `interaction_view`: optional interaction stamp texture. Pass `None` to
    /// use the fallback (no interaction bending).
    #[allow(clippy::too_many_arguments)]
    pub fn draw<'a>(
        &'a self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera: &GrassCameraUniforms,
        params: &GrassParams,
        blades: &[GrassInstance],
        interaction_view: Option<&'a wgpu::TextureView>,
    ) {
        if blades.is_empty() {
            return;
        }

        let blade_count = (blades.len() as u32).min(self.max_blades);

        // Upload camera + params
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(camera));
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(params));

        // Upload blade instances (clamped to capacity)
        let upload_slice = &blades[..blade_count as usize];
        queue.write_buffer(&self.blade_buffer, 0, bytemuck::cast_slice(upload_slice));

        // Build bind groups
        let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GrassBlade Camera BG"),
            layout: &self.camera_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.blade_buffer.as_entire_binding(),
                },
            ],
        });

        let int_view = interaction_view.unwrap_or(&self.interaction_fallback_view);
        let interaction_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GrassBlade Interaction BG"),
            layout: &self.interaction_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(int_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.interaction_sampler),
                },
            ],
        });

        // Draw: 3 vertices per blade, instanced
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &camera_bg, &[]);
        render_pass.set_bind_group(1, &interaction_bg, &[]);
        render_pass.draw(0..3, 0..blade_count);
    }

    /// Get the blade storage buffer (for GPU-driven population).
    pub fn blade_buffer(&self) -> &wgpu::Buffer {
        &self.blade_buffer
    }

    /// Maximum blade capacity.
    pub fn max_blades(&self) -> u32 {
        self.max_blades
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn fract_hash(x: f32) -> f32 {
    (x.sin() * 43_758.547).fract().abs()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grass_instance_size() {
        assert_eq!(std::mem::size_of::<GrassInstance>(), 32);
    }

    #[test]
    fn test_grass_params_size() {
        assert_eq!(std::mem::size_of::<GrassParams>(), 32);
    }

    #[test]
    fn test_grass_camera_uniforms_size() {
        assert_eq!(std::mem::size_of::<GrassCameraUniforms>(), 96);
    }

    #[test]
    fn test_convert_vegetation_to_blades_filters_by_type() {
        let veg = vec![
            VegetationInstanceGpu {
                pos_scale: [10.0, 5.0, 20.0, 1.0],
                rot_type_normal: [0.0, 0.0, 0.0, 1.0], // type 0 = grass
            },
            VegetationInstanceGpu {
                pos_scale: [30.0, 8.0, 40.0, 1.5],
                rot_type_normal: [1.0, 1.0, 0.0, 1.0], // type 1 = tree
            },
            VegetationInstanceGpu {
                pos_scale: [50.0, 3.0, 60.0, 0.8],
                rot_type_normal: [2.0, 0.0, 0.0, 1.0], // type 0 = grass
            },
        ];

        let blades = GrassBladePass::convert_vegetation_to_blades(&veg, &[0], 0.5, 0.05);

        assert_eq!(blades.len(), 2, "should filter to type 0 only");
        assert!((blades[0].pos_height[0] - 10.0).abs() < 0.001);
        assert!((blades[1].pos_height[0] - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_convert_vegetation_applies_scale() {
        let veg = vec![VegetationInstanceGpu {
            pos_scale: [0.0, 0.0, 0.0, 2.0],
            rot_type_normal: [0.0, 0.0, 0.0, 1.0],
        }];

        let blades = GrassBladePass::convert_vegetation_to_blades(&veg, &[0], 0.5, 0.05);

        assert_eq!(blades.len(), 1);
        // Height should be 0.5 * 2.0 = 1.0
        assert!((blades[0].pos_height[3] - 1.0).abs() < 0.001);
        // Width should be 0.05 * 2.0 = 0.1
        assert!((blades[0].dir_width[2] - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_convert_empty_input() {
        let blades = GrassBladePass::convert_vegetation_to_blades(&[], &[0], 0.5, 0.05);
        assert!(blades.is_empty());
    }

    #[test]
    fn test_convert_no_matching_types() {
        let veg = vec![VegetationInstanceGpu {
            pos_scale: [0.0, 0.0, 0.0, 1.0],
            rot_type_normal: [0.0, 5.0, 0.0, 1.0], // type 5
        }];

        let blades = GrassBladePass::convert_vegetation_to_blades(&veg, &[0, 1], 0.5, 0.05);
        assert!(blades.is_empty());
    }

    #[test]
    fn test_fract_hash_deterministic() {
        let a = fract_hash(1.234);
        let b = fract_hash(1.234);
        assert!((a - b).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fract_hash_range() {
        for i in 0..1000 {
            let h = fract_hash(i as f32 * 0.1);
            assert!(h >= 0.0 && h <= 1.0, "hash out of range: {h}");
        }
    }

    #[test]
    fn test_grass_blade_wgsl_present() {
        assert!(
            GRASS_BLADE_SHADER.contains("vs_main"),
            "shader must contain vs_main entry point"
        );
        assert!(
            GRASS_BLADE_SHADER.contains("fs_main"),
            "shader must contain fs_main entry point"
        );
        assert!(
            GRASS_BLADE_SHADER.contains("GrassInstance"),
            "shader must contain GrassInstance struct"
        );
        assert!(
            GRASS_BLADE_SHADER.contains("blade_hash"),
            "shader must contain blade_hash function"
        );
    }

    #[test]
    fn test_grass_params_default() {
        let p = GrassParams::default();
        assert!(p.wind_strength > 0.0);
        assert!(p.interaction_radius > 0.0);
        assert_eq!(p.time, 0.0);
    }
}
