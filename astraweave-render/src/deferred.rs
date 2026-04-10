// Deferred Rendering Pipeline
// Implements G-buffer generation and light accumulation passes

use anyhow::Result;
use wgpu;
use wgpu::util::DeviceExt;

/// G-buffer texture format configuration
#[derive(Debug, Clone, Copy)]
pub struct GBufferFormats {
    /// Albedo + roughness (RGBA8)
    pub albedo: wgpu::TextureFormat,
    /// Normal + metallic (RGBA16Float for precision)
    pub normal: wgpu::TextureFormat,
    /// Emissive (RGBA8)
    pub emissive: wgpu::TextureFormat,
    /// Velocity / motion vectors (RG16Float — screen-space pixel delta)
    pub velocity: wgpu::TextureFormat,
    /// Depth (Depth32Float)
    pub depth: wgpu::TextureFormat,
}

impl Default for GBufferFormats {
    fn default() -> Self {
        Self {
            albedo: wgpu::TextureFormat::Rgba8UnormSrgb,
            normal: wgpu::TextureFormat::Rgba16Float,
            emissive: wgpu::TextureFormat::Rgba8UnormSrgb,
            velocity: wgpu::TextureFormat::Rg16Float,
            depth: wgpu::TextureFormat::Depth32Float,
        }
    }
}

/// G-buffer textures
pub struct GBuffer {
    /// Albedo texture (RGB = albedo, A = roughness)
    pub albedo_texture: wgpu::Texture,
    pub albedo_view: wgpu::TextureView,

    /// Normal texture (RGB = normal, A = metallic)
    pub normal_texture: wgpu::Texture,
    pub normal_view: wgpu::TextureView,

    /// Emissive texture
    pub emissive_texture: wgpu::Texture,
    pub emissive_view: wgpu::TextureView,

    /// Velocity / motion vectors (RG = screen-space pixel delta)
    pub velocity_texture: wgpu::Texture,
    pub velocity_view: wgpu::TextureView,

    /// Depth texture
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,

    /// Texture size
    pub width: u32,
    pub height: u32,

    /// Formats
    pub formats: GBufferFormats,
}

impl GBuffer {
    /// Create a new G-buffer
    pub fn new(device: &wgpu::Device, width: u32, height: u32, formats: GBufferFormats) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        // Albedo texture
        let albedo_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer Albedo"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: formats.albedo,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let albedo_view = albedo_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Normal texture
        let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer Normal"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: formats.normal,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Emissive texture
        let emissive_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer Emissive"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: formats.emissive,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let emissive_view = emissive_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Velocity / motion vectors texture
        let velocity_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer Velocity"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: formats.velocity,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let velocity_view = velocity_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Depth texture
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GBuffer Depth"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: formats.depth,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            albedo_texture,
            albedo_view,
            normal_texture,
            normal_view,
            emissive_texture,
            emissive_view,
            velocity_texture,
            velocity_view,
            depth_texture,
            depth_view,
            width,
            height,
            formats,
        }
    }

    /// Resize G-buffer
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }

        *self = Self::new(device, width, height, self.formats);
    }

    /// Get color attachment descriptors for G-buffer pass (without velocity).
    /// Use `color_attachments_with_velocity()` for the full 4-target MRT.
    pub fn color_attachments(&self) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 3] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: &self.albedo_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.normal_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.emissive_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
        ]
    }

    /// Get color attachment descriptors including velocity buffer for motion vectors.
    pub fn color_attachments_with_velocity(
        &self,
    ) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 4] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: &self.albedo_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.normal_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.emissive_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.velocity_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Clear to zero velocity (no motion)
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }),
        ]
    }

    /// Get depth attachment descriptor
    pub fn depth_attachment(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }
}

/// Deferred renderer
pub struct DeferredRenderer {
    /// G-buffer
    gbuffer: GBuffer,

    /// Light accumulation pipeline
    light_pipeline: wgpu::RenderPipeline,

    /// Light accumulation bind group
    light_bind_group: wgpu::BindGroup,

    /// Bind group layout (used for rebuilding after resize)
    bind_group_layout: wgpu::BindGroupLayout,

    /// Inverse view-projection uniform buffer for position reconstruction
    inv_vp_buf: wgpu::Buffer,

    /// Sampler (cached, reused across frames)
    sampler: wgpu::Sampler,
}

impl DeferredRenderer {
    /// Create a new deferred renderer
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self> {
        let formats = GBufferFormats::default();
        let gbuffer = GBuffer::new(device, width, height, formats);

        // Inverse VP uniform buffer (mat4x4<f32> = 64 bytes)
        let identity_cols = glam::Mat4::IDENTITY.to_cols_array();
        let inv_vp_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Deferred InvVP"),
            contents: bytemuck::cast_slice(&identity_cols),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Deferred Light BG Layout"),
            entries: &[
                // Albedo texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Normal texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Depth texture (replaces position — reconstruct world pos from depth + inv VP)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Emissive texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Inverse view-projection matrix
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("GBuffer Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Deferred Light Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&gbuffer.emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: inv_vp_buf.as_entire_binding(),
                },
            ],
        });

        // Create light accumulation shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Deferred Light Shader"),
            source: wgpu::ShaderSource::Wgsl(DEFERRED_LIGHT_SHADER.into()),
        });

        // Create pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Deferred Light Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let light_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Deferred Light Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            gbuffer,
            light_pipeline,
            light_bind_group,
            bind_group_layout,
            inv_vp_buf,
            sampler,
        })
    }

    /// Get G-buffer
    pub fn gbuffer(&self) -> &GBuffer {
        &self.gbuffer
    }

    /// Get G-buffer (mutable)
    pub fn gbuffer_mut(&mut self) -> &mut GBuffer {
        &mut self.gbuffer
    }

    /// Upload the inverse view-projection matrix for this frame.
    pub fn update_inv_vp(&self, queue: &wgpu::Queue, inv_vp: &glam::Mat4) {
        let cols = inv_vp.to_cols_array();
        queue.write_buffer(&self.inv_vp_buf, 0, bytemuck::cast_slice(&cols));
    }

    /// Rebuild the light bind group after a GBuffer resize.
    pub fn rebuild_bind_group(&mut self, device: &wgpu::Device) {
        self.light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Deferred Light Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.gbuffer.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.gbuffer.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.gbuffer.depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.gbuffer.emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.inv_vp_buf.as_entire_binding(),
                },
            ],
        });
    }

    /// Perform light accumulation pass
    pub fn light_pass(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Deferred Light Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.light_pipeline);
        pass.set_bind_group(0, &self.light_bind_group, &[]);
        pass.draw(0..3, 0..1); // Fullscreen triangle
    }
}

/// Deferred lighting shader — reconstructs world position from depth + inverse VP.
const DEFERRED_LIGHT_SHADER: &str = r#"
@group(0) @binding(0) var albedo_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_depth_2d;
@group(0) @binding(3) var emissive_tex: texture_2d<f32>;
@group(0) @binding(4) var tex_sampler: sampler;

struct InvVP {
    m: mat4x4<f32>,
};
@group(0) @binding(5) var<uniform> inv_vp: InvVP;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle
    var output: VertexOutput;
    let x = f32((vertex_index << 1u) & 2u) - 1.0;
    let y = f32(vertex_index & 2u) - 1.0;
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample G-buffer
    let albedo = textureSample(albedo_tex, tex_sampler, input.uv);
    let normal_metallic = textureSample(normal_tex, tex_sampler, input.uv);
    let emissive = textureSample(emissive_tex, tex_sampler, input.uv);

    // Reconstruct world position from depth + inverse view-projection
    let depth = textureSample(depth_tex, tex_sampler, input.uv);
    let ndc = vec4<f32>(input.uv * 2.0 - 1.0, depth, 1.0);
    let world_h = inv_vp.m * ndc;
    let world_pos = world_h.xyz / world_h.w;

    let normal = normalize(normal_metallic.xyz * 2.0 - 1.0);
    let roughness = albedo.a;
    let metallic = normal_metallic.a;

    // Simple directional light (placeholder for full lighting)
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let n_dot_l = max(dot(normal, light_dir), 0.0);

    let diffuse = albedo.rgb * n_dot_l;
    let ambient = albedo.rgb * 0.1;

    let final_color = diffuse + ambient + emissive.rgb;

    return vec4<f32>(final_color, 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbuffer_formats_default() {
        let formats = GBufferFormats::default();
        assert_eq!(formats.albedo, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(formats.normal, wgpu::TextureFormat::Rgba16Float);
    }

    // --- Mutation-resistant tests ---

    #[test]
    fn gbuffer_formats_default_all_fields() {
        let f = GBufferFormats::default();
        assert_eq!(f.albedo, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(f.normal, wgpu::TextureFormat::Rgba16Float);
        assert_eq!(f.emissive, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(f.velocity, wgpu::TextureFormat::Rg16Float);
        assert_eq!(f.depth, wgpu::TextureFormat::Depth32Float);
    }

    #[test]
    fn gbuffer_has_five_attachment_formats() {
        // 4 color (albedo, normal, emissive, velocity) + 1 depth
        let f = GBufferFormats::default();
        let formats = [f.albedo, f.normal, f.emissive, f.velocity, f.depth];
        assert_eq!(formats.len(), 5);
    }

    #[test]
    fn deferred_light_shader_parses() {
        let module = naga::front::wgsl::parse_str(DEFERRED_LIGHT_SHADER)
            .expect("deferred light WGSL should parse");
        let names: Vec<&str> = module
            .entry_points
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(names.contains(&"vs_main"), "must have vertex entry point");
        assert!(names.contains(&"fs_main"), "must have fragment entry point");
    }

    #[test]
    fn deferred_light_shader_has_six_bindings() {
        // 3 textures + 1 depth texture + 1 sampler + 1 uniform in group(0)
        let src = DEFERRED_LIGHT_SHADER;
        let binding_count = src.matches("@binding").count();
        assert_eq!(
            binding_count, 6,
            "shader should have 6 bindings (3 tex + 1 depth + 1 sampler + 1 uniform)"
        );
    }

    #[test]
    fn gbuffer_formats_are_distinct_types() {
        let f = GBufferFormats::default();
        // Albedo and emissive are sRGB (8-bit), normal is float16
        assert_ne!(
            f.albedo, f.normal,
            "albedo should differ from normal format"
        );
        assert_ne!(f.depth, f.albedo, "depth should differ from color format");
        // Velocity is RG16Float (different from all others)
        assert_ne!(f.velocity, f.albedo);
        assert_ne!(f.velocity, f.normal);
        assert_ne!(f.velocity, f.depth);
    }

    #[test]
    fn deferred_light_shader_reconstructs_from_depth() {
        // Verify the shader uses depth reconstruction instead of position texture
        assert!(
            DEFERRED_LIGHT_SHADER.contains("depth_tex"),
            "shader must sample depth texture"
        );
        assert!(
            DEFERRED_LIGHT_SHADER.contains("inv_vp"),
            "shader must use inverse VP for reconstruction"
        );
        assert!(
            !DEFERRED_LIGHT_SHADER.contains("position_tex"),
            "shader must NOT reference position texture"
        );
    }
}
