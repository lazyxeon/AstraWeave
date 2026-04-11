// Advanced Post-Processing Effects
// TAA, Motion Blur, Depth of Field, Color Grading

use anyhow::Result;
use wgpu;

// Re-use the production TaaConfig from the taa module (single source of truth).
use crate::taa::TaaConfig;

/// Motion blur configuration
#[derive(Debug, Clone, Copy)]
pub struct MotionBlurConfig {
    /// Enable motion blur
    pub enabled: bool,
    /// Number of samples
    pub sample_count: u32,
    /// Blur strength
    pub strength: f32,
}

impl Default for MotionBlurConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sample_count: 8,
            strength: 1.0,
        }
    }
}

/// Depth of Field configuration
#[derive(Debug, Clone, Copy)]
pub struct DofConfig {
    /// Enable DOF
    pub enabled: bool,
    /// Focus distance
    pub focus_distance: f32,
    /// Focus range
    pub focus_range: f32,
    /// Bokeh size
    pub bokeh_size: f32,
}

impl Default for DofConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            focus_distance: 10.0,
            focus_range: 5.0,
            bokeh_size: 2.0,
        }
    }
}

/// Color grading configuration
#[derive(Debug, Clone)]
pub struct ColorGradingConfig {
    /// Enable color grading
    pub enabled: bool,
    /// Exposure adjustment
    pub exposure: f32,
    /// Contrast (1.0 = neutral)
    pub contrast: f32,
    /// Saturation (1.0 = neutral)
    pub saturation: f32,
    /// Color temperature (-1 to 1, negative = cooler, positive = warmer)
    pub temperature: f32,
    /// Tint (-1 to 1, negative = green, positive = magenta)
    pub tint: f32,
}

impl Default for ColorGradingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
        }
    }
}

/// Advanced post-processing system
pub struct AdvancedPostFx {
    // TAA resources (reserved for future full implementation)
    #[allow(dead_code)]
    taa_history_texture: wgpu::Texture,
    #[allow(dead_code)]
    taa_history_view: wgpu::TextureView,
    taa_pipeline: wgpu::RenderPipeline,
    taa_bind_group: wgpu::BindGroup,
    taa_config: TaaConfig,

    // Motion blur resources (reserved for future full implementation)
    #[allow(dead_code)]
    velocity_texture: wgpu::Texture,
    #[allow(dead_code)]
    velocity_view: wgpu::TextureView,
    #[allow(dead_code)]
    motion_blur_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    motion_blur_bind_group: Option<wgpu::BindGroup>,
    #[allow(dead_code)]
    motion_blur_config: MotionBlurConfig,

    // DOF resources
    #[allow(dead_code)]
    dof_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    dof_bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    dof_params_buf: wgpu::Buffer,
    #[allow(dead_code)]
    dof_bind_group: Option<wgpu::BindGroup>,
    #[allow(dead_code)]
    dof_config: DofConfig,

    // Color grading resources (reserved for future full implementation)
    #[allow(dead_code)]
    color_grading_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    color_grading_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    color_grading_bind_group: Option<wgpu::BindGroup>,
    #[allow(dead_code)]
    color_grading_config: ColorGradingConfig,

    // Common resources (used in new())
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    #[allow(dead_code)]
    bind_group_layout: wgpu::BindGroupLayout,

    frame_count: u32,
}

impl AdvancedPostFx {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Result<Self> {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        // Create TAA history texture
        let taa_history_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TAA History"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let taa_history_view =
            taa_history_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create velocity texture for motion blur
        let velocity_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Velocity Buffer"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let velocity_view = velocity_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("PostFx Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PostFx BG Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create TAA pipeline
        let taa_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TAA Shader"),
            source: wgpu::ShaderSource::Wgsl(TAA_SHADER.into()),
        });

        let taa_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("TAA Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let taa_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("TAA Pipeline"),
            layout: Some(&taa_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &taa_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &taa_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create motion blur pipeline
        let motion_blur_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Motion Blur Shader"),
            source: wgpu::ShaderSource::Wgsl(MOTION_BLUR_SHADER.into()),
        });

        let motion_blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Motion Blur Pipeline"),
            layout: Some(&taa_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &motion_blur_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &motion_blur_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create DOF pipeline (with depth texture and params uniform)
        let dof_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("DoF BG Layout"),
            entries: &[
                // binding 0: color texture
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
                // binding 1: depth texture
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
                // binding 2: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 3: DoF params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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

        let dof_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DoF Params"),
            size: 32, // DofParams: 8 floats = 32 bytes
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let dof_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("DOF Shader"),
            source: wgpu::ShaderSource::Wgsl(DOF_SHADER.into()),
        });

        let dof_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("DoF Pipeline Layout"),
            bind_group_layouts: &[&dof_bgl],
            push_constant_ranges: &[],
        });

        let dof_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("DOF Pipeline"),
            layout: Some(&dof_pl),
            vertex: wgpu::VertexState {
                module: &dof_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &dof_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create color grading pipeline
        let color_grading_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Color Grading Shader"),
            source: wgpu::ShaderSource::Wgsl(COLOR_GRADING_SHADER.into()),
        });

        let color_grading_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Color Grading Buffer"),
            size: 32, // 5 floats + padding
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let color_grading_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Color Grading Pipeline"),
                layout: Some(&taa_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &color_grading_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &color_grading_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // Placeholder bind group (will be created per-frame)
        let taa_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TAA Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&taa_history_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Self {
            taa_history_texture,
            taa_history_view,
            taa_pipeline,
            taa_bind_group,
            taa_config: TaaConfig::default(),
            velocity_texture,
            velocity_view,
            motion_blur_pipeline,
            motion_blur_bind_group: None,
            motion_blur_config: MotionBlurConfig::default(),
            dof_pipeline,
            dof_bgl,
            dof_params_buf,
            dof_bind_group: None,
            dof_config: DofConfig::default(),
            color_grading_pipeline,
            color_grading_buffer,
            color_grading_bind_group: None,
            color_grading_config: ColorGradingConfig::default(),
            sampler,
            bind_group_layout,
            frame_count: 0,
        })
    }

    /// Get TAA jitter offset for current frame
    pub fn get_taa_jitter(&self) -> (f32, f32) {
        if !self.taa_config.enabled {
            return (0.0, 0.0);
        }

        // Halton sequence for jitter pattern
        let frame = (self.frame_count % 16) as f32;
        let jitter_x = (halton(frame, 2) - 0.5) * self.taa_config.jitter_scale;
        let jitter_y = (halton(frame, 3) - 0.5) * self.taa_config.jitter_scale;

        (jitter_x, jitter_y)
    }

    /// Apply TAA
    pub fn apply_taa(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        _input_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
    ) {
        if !self.taa_config.enabled {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("TAA Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.taa_pipeline);
        pass.set_bind_group(0, &self.taa_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Update frame counter
    pub fn next_frame(&mut self) {
        self.frame_count += 1;
    }
}

// Halton sequence for TAA jitter
fn halton(index: f32, base: u32) -> f32 {
    let mut result = 0.0;
    let mut f = 1.0;
    let mut i = index as u32;
    let b = base as f32;

    while i > 0 {
        f /= b;
        result += f * (i % base) as f32;
        i /= base;
    }

    result
}

// Shader code
const TAA_SHADER: &str = r#"
@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    let x = f32((vertex_index << 1u) & 2u) - 1.0;
    let y = f32(vertex_index & 2u) - 1.0;
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(color_tex, tex_sampler, input.uv);
    return color;
}
"#;

const MOTION_BLUR_SHADER: &str = r#"
@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    let x = f32((vertex_index << 1u) & 2u) - 1.0;
    let y = f32(vertex_index & 2u) - 1.0;
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple box blur as placeholder
    var color = vec4<f32>(0.0);
    let samples = 5;
    for (var i = 0; i < samples; i++) {
        let offset = f32(i - samples / 2) * 0.001;
        color += textureSample(color_tex, tex_sampler, input.uv + vec2<f32>(offset, 0.0));
    }
    return color / f32(samples);
}
"#;

const DOF_SHADER: &str = r#"
@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var depth_tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct DofParams {
    focus_distance: f32,
    focus_range: f32,
    bokeh_size: f32,
    _pad0: f32,
    near: f32,
    far: f32,
    inv_resolution: vec2<f32>,
}
@group(0) @binding(3) var<uniform> dof: DofParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    let x = f32((vertex_index << 1u) & 2u) - 1.0;
    let y = f32(vertex_index & 2u) - 1.0;
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return output;
}

// Linearize a reverse-Z depth value to view-space distance.
fn linearize_depth(d: f32) -> f32 {
    return dof.near * dof.far / (dof.far - d * (dof.far - dof.near));
}

// Compute circle of confusion (CoC) radius from linear depth.
fn compute_coc(linear_depth: f32) -> f32 {
    let diff = abs(linear_depth - dof.focus_distance);
    let coc = clamp(diff / dof.focus_range, 0.0, 1.0) * dof.bokeh_size;
    return coc;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let raw_depth = textureSample(depth_tex, tex_sampler, input.uv).r;
    let linear_depth = linearize_depth(raw_depth);
    let coc = compute_coc(linear_depth);

    // Early-out: fragments in focus need no blur.
    if (coc < 0.5) {
        return textureSample(color_tex, tex_sampler, input.uv);
    }

    // 9-tap Gaussian disk weighted by CoC.
    let offsets = array<vec2<f32>, 8>(
        vec2<f32>(-1.0,  0.0), vec2<f32>( 1.0,  0.0),
        vec2<f32>( 0.0, -1.0), vec2<f32>( 0.0,  1.0),
        vec2<f32>(-0.707, -0.707), vec2<f32>( 0.707, -0.707),
        vec2<f32>(-0.707,  0.707), vec2<f32>( 0.707,  0.707),
    );

    var accum = textureSample(color_tex, tex_sampler, input.uv);
    var total_weight = 1.0;

    for (var i = 0u; i < 8u; i = i + 1u) {
        let sample_uv = input.uv + offsets[i] * coc * dof.inv_resolution;
        let s = textureSample(color_tex, tex_sampler, sample_uv);
        let w = 0.75;
        accum += s * w;
        total_weight += w;
    }

    return accum / total_weight;
}
"#;

const COLOR_GRADING_SHADER: &str = r#"
@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    let x = f32((vertex_index << 1u) & 2u) - 1.0;
    let y = f32(vertex_index & 2u) - 1.0;
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(color_tex, tex_sampler, input.uv);
    
    // Exposure
    color = vec4<f32>(color.rgb * exp2(0.0), color.a);
    
    // Contrast
    color = vec4<f32>((color.rgb - 0.5) * 1.0 + 0.5, color.a);
    
    // Saturation
    let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    color = vec4<f32>(mix(vec3<f32>(luminance), color.rgb, 1.0), color.a);
    
    return color;
}
"#;

#[cfg(test)]
#[allow(clippy::manual_range_contains, clippy::const_is_empty)]
mod tests {
    use super::*;

    // --- Config defaults ---

    #[test]
    fn taa_config_defaults() {
        let c = TaaConfig::default();
        assert!(c.enabled);
        assert!((c.blend_factor - 0.95).abs() < f32::EPSILON);
        assert!((c.jitter_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn motion_blur_config_defaults() {
        let c = MotionBlurConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.sample_count, 8);
        assert!((c.strength - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dof_config_defaults() {
        let c = DofConfig::default();
        assert!(!c.enabled);
        assert!((c.focus_distance - 10.0).abs() < f32::EPSILON);
        assert!((c.focus_range - 5.0).abs() < f32::EPSILON);
        assert!((c.bokeh_size - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn color_grading_config_defaults() {
        let c = ColorGradingConfig::default();
        assert!(c.enabled);
        assert!((c.exposure - 0.0).abs() < f32::EPSILON);
        assert!((c.contrast - 1.0).abs() < f32::EPSILON);
        assert!((c.saturation - 1.0).abs() < f32::EPSILON);
        assert!((c.temperature - 0.0).abs() < f32::EPSILON);
        assert!((c.tint - 0.0).abs() < f32::EPSILON);
    }

    // --- Halton sequence ---

    #[test]
    fn halton_base2_known_values() {
        // H(1,2)=0.5, H(2,2)=0.25, H(3,2)=0.75, H(4,2)=0.125
        assert!((halton(1.0, 2) - 0.5).abs() < 1e-6);
        assert!((halton(2.0, 2) - 0.25).abs() < 1e-6);
        assert!((halton(3.0, 2) - 0.75).abs() < 1e-6);
        assert!((halton(4.0, 2) - 0.125).abs() < 1e-6);
    }

    #[test]
    fn halton_base3_known_values() {
        // H(1,3)=1/3, H(2,3)=2/3, H(3,3)=1/9
        assert!((halton(1.0, 3) - 1.0 / 3.0).abs() < 1e-6);
        assert!((halton(2.0, 3) - 2.0 / 3.0).abs() < 1e-6);
        assert!((halton(3.0, 3) - 1.0 / 9.0).abs() < 1e-6);
    }

    #[test]
    fn halton_zero_returns_zero() {
        assert_eq!(halton(0.0, 2), 0.0);
        assert_eq!(halton(0.0, 3), 0.0);
    }

    #[test]
    fn halton_values_in_unit_interval() {
        for i in 0..64 {
            let v2 = halton(i as f32, 2);
            let v3 = halton(i as f32, 3);
            assert!(
                v2 >= 0.0 && v2 < 1.0,
                "halton({}, 2) = {} out of range",
                i,
                v2
            );
            assert!(
                v3 >= 0.0 && v3 < 1.0,
                "halton({}, 3) = {} out of range",
                i,
                v3
            );
        }
    }

    // --- Shader constants ---

    #[test]
    fn shader_constants_non_empty() {
        assert!(!TAA_SHADER.is_empty());
        assert!(!MOTION_BLUR_SHADER.is_empty());
        assert!(!DOF_SHADER.is_empty());
        assert!(!COLOR_GRADING_SHADER.is_empty());
    }

    #[test]
    fn shader_constants_have_entry_points() {
        for (name, src) in [
            ("TAA", TAA_SHADER),
            ("MotionBlur", MOTION_BLUR_SHADER),
            ("DOF", DOF_SHADER),
            ("ColorGrading", COLOR_GRADING_SHADER),
        ] {
            assert!(src.contains("fn vs_main"), "{name} shader missing vs_main");
            assert!(src.contains("fn fs_main"), "{name} shader missing fs_main");
        }
    }
}
