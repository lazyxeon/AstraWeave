//! HDR rendering pipeline orchestration.
//!
//! Manages the flow: geometry → HDR target → post-processing stack → LDR → swapchain.
//! Provides GPU-side uniforms for tonemapping and color grading, and coordinates the
//! post-processing pass order.

use crate::advanced_post::ColorGradingConfig;

/// Tonemapping operator selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TonemapOperator {
    /// ACES Filmic (default, good for games).
    Aces,
    /// AgX (neutral, avoids oversaturation in highlights).
    AgX,
    /// Reinhard (simple, classic).
    Reinhard,
    /// No tonemapping (linear passthrough).
    None,
}

impl Default for TonemapOperator {
    fn default() -> Self {
        Self::Aces
    }
}

impl TonemapOperator {
    /// Returns the integer index used in the GPU uniform.
    pub fn to_u32(self) -> u32 {
        match self {
            Self::Aces => 0,
            Self::AgX => 1,
            Self::Reinhard => 2,
            Self::None => 3,
        }
    }
}

/// GPU-side uniforms for the tonemapping + color grading composite pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TonemapUniforms {
    /// Exposure in EV (applied as 2^exposure).
    pub exposure: f32,
    /// Tonemap operator index (0=ACES, 1=AgX, 2=Reinhard, 3=None).
    pub tonemap_op: u32,
    /// Contrast adjustment (1.0 = neutral).
    pub contrast: f32,
    /// Saturation adjustment (1.0 = neutral).
    pub saturation: f32,
    /// Color temperature shift (-1 cool .. +1 warm).
    pub temperature: f32,
    /// Color tint shift (-1 green .. +1 magenta).
    pub tint: f32,
    /// Vignette intensity (0 = off).
    pub vignette_intensity: f32,
    /// Film grain intensity (0 = off).
    pub grain_intensity: f32,
}

impl Default for TonemapUniforms {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            tonemap_op: TonemapOperator::Aces.to_u32(),
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            vignette_intensity: 0.0,
            grain_intensity: 0.0,
        }
    }
}

impl TonemapUniforms {
    /// Create from a `ColorGradingConfig` and a tonemap operator.
    pub fn from_config(config: &ColorGradingConfig, operator: TonemapOperator) -> Self {
        Self {
            exposure: config.exposure,
            tonemap_op: operator.to_u32(),
            contrast: config.contrast,
            saturation: config.saturation,
            temperature: config.temperature,
            tint: config.tint,
            vignette_intensity: 0.0,
            grain_intensity: 0.0,
        }
    }
}

/// Describes the order in which post-processing effects are applied.
/// Each enabled effect reads from the previous output and writes to the next.
#[derive(Debug, Clone)]
pub struct PostProcessChain {
    pub bloom_enabled: bool,
    pub ssao_enabled: bool,
    pub ssr_enabled: bool,
    pub taa_enabled: bool,
    pub motion_blur_enabled: bool,
    pub dof_enabled: bool,
    pub color_grading_enabled: bool,
    pub tonemap_operator: TonemapOperator,
}

impl Default for PostProcessChain {
    fn default() -> Self {
        Self {
            bloom_enabled: false,
            ssao_enabled: false,
            ssr_enabled: false,
            taa_enabled: true,
            motion_blur_enabled: false,
            dof_enabled: false,
            color_grading_enabled: true,
            tonemap_operator: TonemapOperator::Aces,
        }
    }
}

impl PostProcessChain {
    /// Returns the ordered list of active post-processing passes.
    pub fn active_passes(&self) -> Vec<PostPass> {
        let mut passes = Vec::new();
        // Order matters: SSAO and SSR happen on HDR data, before tonemapping.
        if self.ssao_enabled {
            passes.push(PostPass::Ssao);
        }
        if self.ssr_enabled {
            passes.push(PostPass::Ssr);
        }
        // Bloom operates on HDR data (pre-tonemap).
        if self.bloom_enabled {
            passes.push(PostPass::Bloom);
        }
        // TAA before DOF/motion blur for stable input.
        if self.taa_enabled {
            passes.push(PostPass::Taa);
        }
        // DOF before motion blur.
        if self.dof_enabled {
            passes.push(PostPass::Dof);
        }
        if self.motion_blur_enabled {
            passes.push(PostPass::MotionBlur);
        }
        // Tonemapping + color grading is always last.
        passes.push(PostPass::Tonemap);
        passes
    }

    /// Number of intermediate HDR targets needed (for ping-pong rendering).
    /// Returns 2 if any pre-tonemap pass is enabled, 1 otherwise.
    pub fn intermediate_targets_needed(&self) -> usize {
        let pre_tonemap_count = self
            .active_passes()
            .iter()
            .filter(|p| **p != PostPass::Tonemap)
            .count();
        if pre_tonemap_count > 0 {
            2
        } else {
            1
        }
    }
}

/// Individual post-processing pass identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostPass {
    Ssao,
    Ssr,
    Bloom,
    Taa,
    Dof,
    MotionBlur,
    Tonemap,
}

/// Manages HDR pipeline GPU resources: intermediate targets, tonemapping uniform buffer.
pub struct HdrPipeline {
    /// Tonemapping + color grading uniform buffer.
    tonemap_buf: wgpu::Buffer,
    /// Bind group layout for the tonemap pass.
    tonemap_bgl: wgpu::BindGroupLayout,
    /// Tonemapping render pipeline.
    tonemap_pipeline: wgpu::RenderPipeline,
    /// HDR intermediate textures for ping-pong rendering.
    hdr_textures: [wgpu::Texture; 2],
    hdr_views: [wgpu::TextureView; 2],
    /// Active post-processing chain configuration.
    chain: PostProcessChain,
    /// Current dimensions.
    width: u32,
    height: u32,
}

impl HdrPipeline {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let hdr_format = wgpu::TextureFormat::Rgba16Float;

        let hdr_textures = std::array::from_fn(|i| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("hdr_intermediate_{i}")),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: hdr_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        });
        let hdr_views = [
            hdr_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            hdr_textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];

        let tonemap_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tonemap_uniforms"),
            size: std::mem::size_of::<TonemapUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let tonemap_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tonemap_bgl"),
            entries: &[
                // HDR input texture
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
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Tonemap uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tonemap_shader"),
            source: wgpu::ShaderSource::Wgsl(TONEMAP_SHADER.into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tonemap_pipeline_layout"),
            bind_group_layouts: &[&tonemap_bgl],
            push_constant_ranges: &[],
        });

        let tonemap_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tonemap_pipeline"),
            layout: Some(&layout),
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
                    format: surface_format,
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

        Self {
            tonemap_buf,
            tonemap_bgl,
            tonemap_pipeline,
            hdr_textures,
            hdr_views,
            chain: PostProcessChain::default(),
            width,
            height,
        }
    }

    /// Update tonemapping uniforms. Call once per frame.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &TonemapUniforms) {
        queue.write_buffer(&self.tonemap_buf, 0, bytemuck::bytes_of(uniforms));
    }

    /// Execute the final tonemap pass: reads from HDR input, writes to LDR surface.
    pub fn tonemap_pass(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        hdr_input_view: &wgpu::TextureView,
        ldr_output_view: &wgpu::TextureView,
    ) {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("tonemap_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tonemap_bg"),
            layout: &self.tonemap_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.tonemap_buf.as_entire_binding(),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("tonemap_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: ldr_output_view,
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

        pass.set_pipeline(&self.tonemap_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Resize HDR intermediate targets.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        let hdr_format = wgpu::TextureFormat::Rgba16Float;
        self.hdr_textures = std::array::from_fn(|i| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("hdr_intermediate_{i}")),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: hdr_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        });
        self.hdr_views = [
            self.hdr_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            self.hdr_textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        self.width = width;
        self.height = height;
    }

    /// Get the HDR intermediate view for a given ping-pong index (0 or 1).
    pub fn hdr_view(&self, index: usize) -> &wgpu::TextureView {
        &self.hdr_views[index & 1]
    }

    /// Get the post-processing chain configuration.
    pub fn chain(&self) -> &PostProcessChain {
        &self.chain
    }

    /// Set the post-processing chain configuration.
    pub fn set_chain(&mut self, chain: PostProcessChain) {
        self.chain = chain;
    }

    /// Get current dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get the tonemap bind group layout (for external pipeline creation).
    pub fn tonemap_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.tonemap_bgl
    }
}

/// Production tonemapping + color grading shader with multiple operator support.
const TONEMAP_SHADER: &str = r#"
struct TonemapUniforms {
    exposure: f32,
    tonemap_op: u32,
    contrast: f32,
    saturation: f32,
    temperature: f32,
    tint: f32,
    vignette_intensity: f32,
    grain_intensity: f32,
};

@group(0) @binding(0) var hdr_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> u: TonemapUniforms;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    // Fullscreen triangle
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(positions[vid], 0.0, 1.0);
    out.uv = (positions[vid] + vec2<f32>(1.0, 1.0)) * 0.5;
    // Flip Y for proper UV mapping
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

// ACES Filmic tonemapping (Narkowicz approximation)
fn tonemap_aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// AgX tonemapping — sRGB → AgX log space via 3×3 inset matrix, log2 encoding,
// 6th-order polynomial contrast curve. Reference: Troy Sobotka, Blender AgX.
fn agx_default_contrast_approx(x: vec3<f32>) -> vec3<f32> {
    let x2 = x * x;
    let x4 = x2 * x2;
    return 15.5     * x4 * x2
         - 40.14    * x4 * x
         + 31.96    * x4
         -  6.868   * x2 * x
         +  0.4298  * x2
         +  0.1191  * x
         -  0.00232;
}

fn tonemap_agx(color: vec3<f32>) -> vec3<f32> {
    // 3×3 AgX inset matrix: maps sRGB linear primaries into the AgX log encoding
    // gamut. Without this, log encoding operates on raw sRGB values and produces
    // incorrect hue shifts, especially in saturated highlights.
    let agx_mat = mat3x3<f32>(
        0.842479062253094,  0.0423282422610123, 0.0423756549057051,
        0.0784335999999992, 0.878468636469772,  0.0784336,
        0.0792237451477643, 0.0791661274605434, 0.879142973793104
    );
    var val = agx_mat * max(color, vec3<f32>(1e-10));
    // Log2 encoding into [min_ev, max_ev] range, then normalize to [0,1]
    val = clamp(log2(val), vec3<f32>(-12.47393), vec3<f32>(4.026069));
    val = (val - vec3<f32>(-12.47393)) / (vec3<f32>(4.026069) - vec3<f32>(-12.47393));
    return agx_default_contrast_approx(val);
}

// Reinhard tonemapping (extended, per-channel)
fn tonemap_reinhard(x: vec3<f32>) -> vec3<f32> {
    return x / (vec3<f32>(1.0) + x);
}

// Color temperature shift (approximate Planckian locus)
fn apply_temperature(color: vec3<f32>, temp: f32) -> vec3<f32> {
    // Warm shifts toward red/yellow, cool shifts toward blue
    let warm = vec3<f32>(1.0 + temp * 0.1, 1.0, 1.0 - temp * 0.1);
    return color * warm;
}

// Color tint shift (green-magenta axis)
fn apply_tint(color: vec3<f32>, t: f32) -> vec3<f32> {
    let shift = vec3<f32>(1.0 + t * 0.05, 1.0 - abs(t) * 0.02, 1.0 - t * 0.05);
    return color * shift;
}

// sRGB gamma encoding
fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let cutoff = step(linear, vec3<f32>(0.0031308));
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055;
    return mix(high, low, cutoff);
}

@fragment
fn fs_main(input: VSOut) -> @location(0) vec4<f32> {
    var color = textureSampleLevel(hdr_tex, samp, input.uv, 0.0).rgb;

    // 1. Exposure
    color = color * exp2(u.exposure);

    // 2. Color temperature & tint (in linear HDR space)
    color = apply_temperature(color, u.temperature);
    color = apply_tint(color, u.tint);

    // 3. Tonemapping
    if (u.tonemap_op == 0u) {
        color = tonemap_aces(color);
    } else if (u.tonemap_op == 1u) {
        color = tonemap_agx(color);
    } else if (u.tonemap_op == 2u) {
        color = tonemap_reinhard(color);
    }
    // op == 3: no tonemapping (linear passthrough, clamp to 0..1)
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    // 4. Contrast (applied in gamma-ish space for perceptual correctness)
    color = (color - 0.5) * u.contrast + 0.5;
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    // 5. Saturation
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = mix(vec3<f32>(luma), color, u.saturation);

    // 6. Vignette
    if (u.vignette_intensity > 0.0) {
        let center = input.uv - vec2<f32>(0.5);
        let dist = length(center) * 1.414; // normalize to corners
        let vig = 1.0 - u.vignette_intensity * dist * dist;
        color = color * max(vig, 0.0);
    }

    // 7. Film grain (simple noise)
    if (u.grain_intensity > 0.0) {
        let noise = fract(sin(dot(input.uv * 1000.0, vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5;
        color = color + vec3<f32>(noise * u.grain_intensity);
    }

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tonemap_operator_indices() {
        assert_eq!(TonemapOperator::Aces.to_u32(), 0);
        assert_eq!(TonemapOperator::AgX.to_u32(), 1);
        assert_eq!(TonemapOperator::Reinhard.to_u32(), 2);
        assert_eq!(TonemapOperator::None.to_u32(), 3);
    }

    #[test]
    fn tonemap_uniforms_default() {
        let u = TonemapUniforms::default();
        assert_eq!(u.exposure, 0.0);
        assert_eq!(u.tonemap_op, 0); // ACES
        assert_eq!(u.contrast, 1.0);
        assert_eq!(u.saturation, 1.0);
        assert_eq!(u.temperature, 0.0);
        assert_eq!(u.tint, 0.0);
        assert_eq!(u.vignette_intensity, 0.0);
        assert_eq!(u.grain_intensity, 0.0);
    }

    #[test]
    fn tonemap_uniforms_pod_size() {
        assert_eq!(std::mem::size_of::<TonemapUniforms>(), 32);
    }

    #[test]
    fn tonemap_uniforms_from_config() {
        let config = ColorGradingConfig {
            enabled: true,
            exposure: 1.5,
            contrast: 1.2,
            saturation: 0.8,
            temperature: -0.3,
            tint: 0.1,
        };
        let u = TonemapUniforms::from_config(&config, TonemapOperator::AgX);
        assert_eq!(u.exposure, 1.5);
        assert_eq!(u.tonemap_op, 1); // AgX
        assert_eq!(u.contrast, 1.2);
        assert_eq!(u.saturation, 0.8);
    }

    #[test]
    fn post_process_chain_default() {
        let chain = PostProcessChain::default();
        assert!(!chain.bloom_enabled);
        assert!(chain.taa_enabled);
        assert!(chain.color_grading_enabled);
        assert_eq!(chain.tonemap_operator, TonemapOperator::Aces);
    }

    #[test]
    fn post_process_chain_active_passes() {
        let chain = PostProcessChain::default();
        let passes = chain.active_passes();
        // With defaults: TAA + Tonemap
        assert!(passes.contains(&PostPass::Taa));
        assert!(passes.contains(&PostPass::Tonemap));
        assert!(!passes.contains(&PostPass::Bloom));
    }

    #[test]
    fn post_process_chain_all_enabled() {
        let chain = PostProcessChain {
            bloom_enabled: true,
            ssao_enabled: true,
            ssr_enabled: true,
            taa_enabled: true,
            motion_blur_enabled: true,
            dof_enabled: true,
            color_grading_enabled: true,
            tonemap_operator: TonemapOperator::Aces,
        };
        let passes = chain.active_passes();
        assert_eq!(passes.len(), 7);
        // Order: SSAO → SSR → Bloom → TAA → DoF → MotionBlur → Tonemap
        assert_eq!(passes[0], PostPass::Ssao);
        assert_eq!(passes[1], PostPass::Ssr);
        assert_eq!(passes[2], PostPass::Bloom);
        assert_eq!(passes[3], PostPass::Taa);
        assert_eq!(passes[4], PostPass::Dof);
        assert_eq!(passes[5], PostPass::MotionBlur);
        assert_eq!(passes[6], PostPass::Tonemap);
    }

    #[test]
    fn intermediate_targets_needed() {
        let chain = PostProcessChain::default();
        // TAA is enabled, so we need 2 targets for ping-pong
        assert_eq!(chain.intermediate_targets_needed(), 2);

        let minimal = PostProcessChain {
            bloom_enabled: false,
            ssao_enabled: false,
            ssr_enabled: false,
            taa_enabled: false,
            motion_blur_enabled: false,
            dof_enabled: false,
            color_grading_enabled: false,
            tonemap_operator: TonemapOperator::Aces,
        };
        // Only tonemap, no pre-tonemap passes
        assert_eq!(minimal.intermediate_targets_needed(), 1);
    }

    #[test]
    fn tonemap_shader_parses() {
        let module =
            naga::front::wgsl::parse_str(TONEMAP_SHADER).expect("tonemap WGSL should parse");
        let names: Vec<&str> = module
            .entry_points
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(names.contains(&"vs_main"));
        assert!(names.contains(&"fs_main"));
    }

    #[test]
    fn hdr_pipeline_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let pipeline = HdrPipeline::new(&device, 1920, 1080, wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(pipeline.dimensions(), (1920, 1080));
    }

    #[test]
    fn hdr_pipeline_resize() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let mut pipeline = HdrPipeline::new(&device, 800, 600, wgpu::TextureFormat::Bgra8UnormSrgb);
        pipeline.resize(&device, 1920, 1080);
        assert_eq!(pipeline.dimensions(), (1920, 1080));
    }
}
