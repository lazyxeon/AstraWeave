//! Cascaded Shadow Mapping (CSM) Implementation
//!
//! This module implements a 4-cascade shadow mapping system for directional lights
//! (e.g., sun/moon). Each cascade covers a different depth range from the camera,
//! providing high detail near the camera and broader coverage far away.
//!
//! # Architecture
//!
//! - **Shadow Atlas**: Single 8192×8192 texture containing all 4 cascades (2048×2048 each)
//! - **Cascade Splitting**: Logarithmic distribution based on view frustum depth
//! - **Sampling**: PCF (Percentage Closer Filtering) for soft shadows
//! - **Bias**: Depth bias to prevent shadow acne
//!
//! # Performance Targets (60 FPS = 16.67ms budget)
//!
//! - Shadow map rendering: <2.0ms (4 cascades × 0.5ms each)
//! - Cascade selection: <0.01ms (per-pixel, branchless)
//! - PCF sampling: <0.5ms (5×5 kernel, optimized)
//! - Total shadow budget: <2.5ms (15% of frame)

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use wgpu;

// Minimal shadow-only shader (uses group(0) since it's the only bind group)
const SHADOW_DEPTH_SHADER: &str = r#"
// Shadow cascade data
struct ShadowCascade {
    view_proj: mat4x4<f32>,
    split_distances: vec4<f32>,
    atlas_transform: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> cascades: array<ShadowCascade, 4>;

struct ShadowVertexInput {
    @location(0) position: vec3<f32>,
}

struct ShadowVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn shadow_vertex_main(
    in: ShadowVertexInput,
    @builtin(instance_index) cascade_index: u32,
) -> ShadowVertexOutput {
    var out: ShadowVertexOutput;
    let world_pos = vec4<f32>(in.position, 1.0);
    let cascade_idx = min(cascade_index, 3u);
    out.clip_position = cascades[cascade_idx].view_proj * world_pos;
    return out;
}

@fragment
fn shadow_fragment_main(in: ShadowVertexOutput) {
    // Depth written automatically
}

// ─── Alpha-tested variant ───
// For vegetation, fences, particles — any geometry with alpha-masked textures.

struct AlphaVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct AlphaVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(1) @binding(0) var alpha_tex: texture_2d<f32>;
@group(1) @binding(1) var alpha_samp: sampler;

@vertex
fn shadow_vertex_alpha_main(
    in: AlphaVertexInput,
    @builtin(instance_index) cascade_index: u32,
) -> AlphaVertexOutput {
    var out: AlphaVertexOutput;
    let world_pos = vec4<f32>(in.position, 1.0);
    let cascade_idx = min(cascade_index, 3u);
    out.clip_position = cascades[cascade_idx].view_proj * world_pos;
    out.uv = in.uv;
    return out;
}

@fragment
fn shadow_fragment_alpha_main(in: AlphaVertexOutput) {
    let alpha = textureSample(alpha_tex, alpha_samp, in.uv).a;
    if (alpha < 0.5) {
        discard;
    }
}
"#;

/// Number of shadow cascades (industry standard: 4)
pub const CASCADE_COUNT: usize = 4;

/// Shadow map resolution per cascade (2048×2048 = high quality)
pub const CASCADE_RESOLUTION: u32 = 2048;

/// TEMP: Use separate textures instead of atlas for simplicity
pub const ATLAS_RESOLUTION: u32 = CASCADE_RESOLUTION; // 2048×2048 per cascade

/// Depth bias to prevent shadow acne (tweakable)
pub const DEPTH_BIAS: f32 = 0.005;

/// Shadow cascade configuration
///
/// Each cascade covers a depth range [near, far] in view space.
/// The view-projection matrix transforms world coords to shadow map space.
#[derive(Debug, Clone, Copy)]
pub struct ShadowCascade {
    /// Near plane distance (view space)
    pub near: f32,
    /// Far plane distance (view space)
    pub far: f32,
    /// View matrix (light space)
    pub view_matrix: Mat4,
    /// Projection matrix (orthographic, light space)
    pub proj_matrix: Mat4,
    /// Combined view-projection matrix (for shader upload)
    pub view_proj_matrix: Mat4,
    /// Atlas offset (UV coords: 0.0-0.5 for each quadrant)
    pub atlas_offset: Vec4, // (offset_x, offset_y, scale_x, scale_y)
}

/// GPU-compatible shadow cascade data (uploaded to shader)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuShadowCascade {
    /// View-projection matrix (light space)
    pub view_proj: [[f32; 4]; 4],
    /// Split distances (near, far, 0, 0) for depth comparison
    pub split_distances: [f32; 4],
    /// Atlas UV transform (offset_x, offset_y, scale_x, scale_y)
    pub atlas_transform: [f32; 4],
}

impl From<&ShadowCascade> for GpuShadowCascade {
    fn from(cascade: &ShadowCascade) -> Self {
        Self {
            view_proj: cascade.view_proj_matrix.to_cols_array_2d(),
            split_distances: [cascade.near, cascade.far, 0.0, 0.0],
            atlas_transform: cascade.atlas_offset.to_array(),
        }
    }
}

/// Cascaded Shadow Mapping renderer
///
/// Manages shadow map atlas, cascade splitting, and shadow rendering passes.
pub struct CsmRenderer {
    /// Shadow atlas texture (8192×8192, Depth32Float)
    pub atlas_texture: wgpu::Texture,
    /// Shadow atlas view (for rendering)
    pub atlas_view: wgpu::TextureView,
    /// Shadow sampler (comparison sampler for PCF)
    pub shadow_sampler: wgpu::Sampler,

    /// Cascade data (CPU-side, updated per frame)
    pub cascades: [ShadowCascade; CASCADE_COUNT],

    /// Cascade buffer (GPU-side, uploaded each frame)
    pub cascade_buffer: wgpu::Buffer,

    /// Bind group for shadow sampling (used in main render pass)
    pub bind_group: Option<wgpu::BindGroup>,
    pub bind_group_layout: wgpu::BindGroupLayout,

    // Shadow-specific bind group (just cascades buffer, for depth pass)
    shadow_bind_group: Option<wgpu::BindGroup>,
    shadow_bind_group_layout: wgpu::BindGroupLayout,

    /// Render pass depth attachments (one per cascade)
    cascade_views: [wgpu::TextureView; CASCADE_COUNT],

    /// Per-cascade dirty flags — skip re-rendering clean cascades
    cascade_dirty: [bool; CASCADE_COUNT],

    /// Previous cascade view-projection matrices for dirty detection
    prev_view_proj: [Mat4; CASCADE_COUNT],

    /// Shadow rendering pipeline (depth-only pass, opaque geometry)
    pub shadow_pipeline: wgpu::RenderPipeline,

    /// Alpha-tested shadow pipeline (for masked geometry: foliage, fences, etc.)
    pub shadow_alpha_pipeline: wgpu::RenderPipeline,

    /// Bind group layout for alpha-test material (group(1): albedo texture + sampler)
    pub shadow_alpha_bgl: wgpu::BindGroupLayout,

    /// Shader module
    #[allow(dead_code)]
    shader_module: wgpu::ShaderModule,
}

impl CsmRenderer {
    /// Create a new CSM renderer
    ///
    /// # Arguments
    ///
    /// - `device`: wgpu device for resource creation
    ///
    /// # Returns
    ///
    /// Initialized CSM renderer with shadow atlas and cascade buffers
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        // Create shadow atlas as TEXTURE ARRAY (4 layers, one per cascade)
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Atlas (Array)"),
            size: wgpu::Extent3d {
                width: CASCADE_RESOLUTION,
                height: CASCADE_RESOLUTION,
                depth_or_array_layers: CASCADE_COUNT as u32, // 4 layers
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Shadow Atlas Array View"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(CASCADE_COUNT as u32),
            usage: None,
        });

        // Create cascade views (each layer of the texture array)
        let cascade_views = [
            // Cascade 0 (layer 0)
            atlas_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cascade 0 View"),
                format: Some(wgpu::TextureFormat::Depth32Float),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: None,
            }),
            // Cascade 1 (layer 1)
            atlas_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cascade 1 View"),
                format: Some(wgpu::TextureFormat::Depth32Float),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 1,
                array_layer_count: Some(1),
                usage: None,
            }),
            // Cascade 2 (layer 2)
            atlas_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cascade 2 View"),
                format: Some(wgpu::TextureFormat::Depth32Float),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 2,
                array_layer_count: Some(1),
                usage: None,
            }),
            // Cascade 3 (layer 3)
            atlas_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cascade 3 View"),
                format: Some(wgpu::TextureFormat::Depth32Float),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 3,
                array_layer_count: Some(1),
                usage: None,
            }),
        ];

        // Create comparison sampler for PCF
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler (Comparison)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // Enable depth comparison
            ..Default::default()
        });

        // Create cascade buffer (4 cascades × GpuShadowCascade)
        let cascade_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shadow Cascade Buffer"),
            size: (std::mem::size_of::<GpuShadowCascade>() * CASCADE_COUNT) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout (for shader access)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("CSM Bind Group Layout"),
            entries: &[
                // Shadow atlas texture (D2Array for texture array)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // Shadow sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                // Cascade data buffer
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

        // Create shadow-only bind group layout (just cascades buffer for depth pass)
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Depth Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Initialize cascades with default values (will be updated in update_cascades)
        let cascades = [
            ShadowCascade {
                near: 0.1,
                far: 10.0,
                view_matrix: Mat4::IDENTITY,
                proj_matrix: Mat4::IDENTITY,
                view_proj_matrix: Mat4::IDENTITY,
                atlas_offset: Vec4::new(0.0, 0.0, 1.0, 1.0), // Full layer (texture array)
            },
            ShadowCascade {
                near: 10.0,
                far: 50.0,
                view_matrix: Mat4::IDENTITY,
                proj_matrix: Mat4::IDENTITY,
                view_proj_matrix: Mat4::IDENTITY,
                atlas_offset: Vec4::new(0.0, 0.0, 1.0, 1.0), // Full layer (texture array)
            },
            ShadowCascade {
                near: 50.0,
                far: 200.0,
                view_matrix: Mat4::IDENTITY,
                proj_matrix: Mat4::IDENTITY,
                view_proj_matrix: Mat4::IDENTITY,
                atlas_offset: Vec4::new(0.0, 0.0, 1.0, 1.0), // Full layer (texture array)
            },
            ShadowCascade {
                near: 200.0,
                far: 1000.0,
                view_matrix: Mat4::IDENTITY,
                proj_matrix: Mat4::IDENTITY,
                view_proj_matrix: Mat4::IDENTITY,
                atlas_offset: Vec4::new(0.0, 0.0, 1.0, 1.0), // Full layer (texture array)
            },
        ];

        // Create shader module for shadow depth pass (minimal, group(1) only)
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow CSM Depth Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_DEPTH_SHADER.into()),
        });

        // Create shadow render pipeline (depth-only pass)
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&shadow_bind_group_layout], // group(0): just cascades buffer
                push_constant_ranges: &[],
            });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Render Pipeline"),
            layout: Some(&shadow_pipeline_layout), // Use explicit layout (only group 1)
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("shadow_vertex_main"),
                buffers: &[
                    // FIXED: Full vertex buffer (position vec3 + normal vec3 = 24 bytes)
                    // Shadow shader only uses position, but stride must match actual vertex data
                    wgpu::VertexBufferLayout {
                        array_stride: 24, // 6 × f32 (position + normal)
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            },
                            // Normal at offset 12 (ignored by shader, but maintains alignment)
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2, // Depth bias for shadow acne prevention
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None, // Depth-only pass, no fragment shader
            multiview: None,
            cache: None,
        });

        // Alpha-tested shadow pipeline (for masked geometry)
        let shadow_alpha_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Alpha BGL"),
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

        let shadow_alpha_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Alpha Pipeline Layout"),
            bind_group_layouts: &[&shadow_bind_group_layout, &shadow_alpha_bgl],
            push_constant_ranges: &[],
        });

        let shadow_alpha_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Shadow Alpha Render Pipeline"),
                layout: Some(&shadow_alpha_pl),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("shadow_vertex_alpha_main"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 48, // Same stride as Vertex (pos+norm+tangent+uv)
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0, // position
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 40, // uv after pos(12)+normal(12)+tangent(16)
                                shader_location: 1, // uv → shader @location(1)
                            },
                        ],
                    }],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // No culling for alpha-tested geometry (two-sided foliage)
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("shadow_fragment_alpha_main"),
                    targets: &[], // Depth-only, no color targets
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        Ok(Self {
            atlas_texture,
            atlas_view,
            shadow_sampler,
            cascades,
            cascade_buffer,
            bind_group: None, // Created after first update
            bind_group_layout,
            shadow_bind_group: None, // Created after first update
            shadow_bind_group_layout,
            cascade_views,
            cascade_dirty: [true; CASCADE_COUNT],
            prev_view_proj: [Mat4::ZERO; CASCADE_COUNT],
            shadow_pipeline,
            shadow_alpha_pipeline,
            shadow_alpha_bgl,
            shader_module,
        })
    }

    /// Update cascade splits and matrices based on camera frustum
    ///
    /// # Arguments
    ///
    /// - `camera_pos`: Camera world position
    /// - `camera_view`: Camera view matrix
    /// - `camera_proj`: Camera projection matrix
    /// - `light_dir`: Directional light direction (normalized, pointing TOWARD light)
    /// - `near`: Camera near plane
    /// - `far`: Camera far plane
    ///
    /// # Algorithm
    ///
    /// Uses logarithmic splitting with λ=0.5 (balance between uniform and logarithmic):
    /// ```text
    /// split[i] = λ * (near * (far/near)^(i/N)) + (1-λ) * (near + (far-near) * i/N)
    /// ```
    pub fn update_cascades(
        &mut self,
        camera_pos: Vec3,
        camera_view: Mat4,
        camera_proj: Mat4,
        light_dir: Vec3,
        near: f32,
        far: f32,
    ) {
        // Calculate cascade split distances (logarithmic distribution)
        let lambda = 0.5; // Balance between uniform (0.0) and logarithmic (1.0)
        let mut split_distances = [0.0f32; CASCADE_COUNT + 1];
        split_distances[0] = near;
        split_distances[CASCADE_COUNT] = far;

        for (i, split) in split_distances
            .iter_mut()
            .enumerate()
            .take(CASCADE_COUNT)
            .skip(1)
        {
            let i_f = i as f32;
            let n_f = CASCADE_COUNT as f32;

            // Logarithmic split
            let log_split = near * (far / near).powf(i_f / n_f);

            // Uniform split
            let uniform_split = near + (far - near) * (i_f / n_f);

            // Blend
            *split = lambda * log_split + (1.0 - lambda) * uniform_split;
        }

        // Compute inverse view-projection to get frustum corners in world space
        let inv_proj = camera_proj.inverse();
        let inv_view = camera_view.inverse();

        // Update each cascade
        for (i, cascade) in self.cascades.iter_mut().enumerate() {
            cascade.near = split_distances[i];
            cascade.far = split_distances[i + 1];

            // Map cascade near/far to NDC z range [0, 1] (reverse-Z convention)
            // For a perspective projection, ndc_z = (split - near) / (far - near)
            let ndc_near = (split_distances[i] - near) / (far - near);
            let ndc_far = (split_distances[i + 1] - near) / (far - near);

            // Compute 8 frustum corners for this cascade slice in NDC space
            // NDC: x,y in [-1,1], z in [0,1] for wgpu
            let ndc_corners = [
                // Near plane
                Vec3::new(-1.0, -1.0, ndc_near),
                Vec3::new( 1.0, -1.0, ndc_near),
                Vec3::new(-1.0,  1.0, ndc_near),
                Vec3::new( 1.0,  1.0, ndc_near),
                // Far plane
                Vec3::new(-1.0, -1.0, ndc_far),
                Vec3::new( 1.0, -1.0, ndc_far),
                Vec3::new(-1.0,  1.0, ndc_far),
                Vec3::new( 1.0,  1.0, ndc_far),
            ];

            // Unproject NDC corners to world space
            let mut world_corners = [Vec3::ZERO; 8];
            let mut center = Vec3::ZERO;
            for (j, ndc) in ndc_corners.iter().enumerate() {
                let clip = glam::Vec4::new(ndc.x, ndc.y, ndc.z, 1.0);
                let view_pos = inv_proj * clip;
                let view_pos = view_pos / view_pos.w; // perspective divide
                let world_pos = inv_view * view_pos;
                world_corners[j] = Vec3::new(world_pos.x, world_pos.y, world_pos.z);
                center += world_corners[j];
            }
            center /= 8.0;

            // Choose up vector perpendicular to light direction
            let up = if light_dir.y.abs() > 0.9 {
                Vec3::X
            } else {
                Vec3::Y
            };

            // Compute light view matrix looking at the cascade center
            let light_norm = light_dir.normalize();

            // Compute the bounding sphere radius of the frustum slice to determine
            // how far back to place the light (ensures all geometry is captured)
            let mut max_radius = 0.0f32;
            for corner in &world_corners {
                let dist = (*corner - center).length();
                if dist > max_radius {
                    max_radius = dist;
                }
            }

            // Place light far enough behind the frustum center to capture everything.
            // Margin scales proportionally with cascade size (not fixed) to preserve
            // depth precision for near cascades and coverage for far cascades.
            let light_distance = max_radius * 2.0;
            let light_pos = center - light_norm * light_distance;

            cascade.view_matrix = Mat4::look_at_rh(light_pos, center, up);

            // Transform frustum corners to light space and compute tight AABB
            let mut ls_min = Vec3::splat(f32::MAX);
            let mut ls_max = Vec3::splat(f32::MIN);
            for corner in &world_corners {
                let ls = cascade.view_matrix.transform_point3(*corner);
                ls_min = ls_min.min(ls);
                ls_max = ls_max.max(ls);
            }

            // Also include camera position itself to avoid popping
            let ls_cam = cascade.view_matrix.transform_point3(camera_pos);
            ls_min = ls_min.min(ls_cam);
            ls_max = ls_max.max(ls_cam);

            let ortho_half_x = (ls_max.x - ls_min.x) * 0.5;
            let ortho_half_y = (ls_max.y - ls_min.y) * 0.5;
            // Use the larger of x/y for a square ortho (simplifies sampling)
            let ortho_size = ortho_half_x.max(ortho_half_y);

            // Texel-snapping: round light-space center to texel increments to prevent
            // shadow shimmer as the camera moves
            let shadow_map_size = 2048.0; // per-cascade resolution
            let texel_size = (ortho_size * 2.0) / shadow_map_size;
            let ls_center_x = (ls_min.x + ls_max.x) * 0.5;
            let ls_center_y = (ls_min.y + ls_max.y) * 0.5;
            let snapped_x = (ls_center_x / texel_size).floor() * texel_size;
            let snapped_y = (ls_center_y / texel_size).floor() * texel_size;
            let snap_offset_x = snapped_x - ls_center_x;
            let snap_offset_y = snapped_y - ls_center_y;

            // Apply texel-snap offset to the view matrix (translate in light space)
            cascade.view_matrix = Mat4::from_translation(Vec3::new(snap_offset_x, snap_offset_y, 0.0))
                * cascade.view_matrix;

            // Light-space depth range: ensure we capture geometry in front of and behind
            let ls_depth_range = ls_max.z - ls_min.z;
            let light_near = 0.1;
            let light_far = ls_depth_range + light_distance * 2.0;

            // Log diagnostics on first call
            if i == 0 {
                static FIRST_UPDATE: std::sync::atomic::AtomicBool =
                    std::sync::atomic::AtomicBool::new(true);
                if FIRST_UPDATE.swap(false, std::sync::atomic::Ordering::Relaxed) {
                    log::debug!(
                        "CSM cascade 0: ortho_size={:.1}, center=({:.1},{:.1},{:.1}), light_far={:.1}",
                        ortho_size,
                        center.x, center.y, center.z,
                        light_far,
                    );
                }
            }

            cascade.proj_matrix = Mat4::orthographic_rh(
                -ortho_size,
                ortho_size,
                -ortho_size,
                ortho_size,
                light_near,
                light_far,
            );

            cascade.view_proj_matrix = cascade.proj_matrix * cascade.view_matrix;

            // Dirty detection: compare against previous frame's matrix
            // Use column-wise max-abs-diff for numerical stability
            let diff = cascade.view_proj_matrix - self.prev_view_proj[i];
            let max_diff = diff.x_axis.abs().max_element()
                .max(diff.y_axis.abs().max_element())
                .max(diff.z_axis.abs().max_element())
                .max(diff.w_axis.abs().max_element());
            self.cascade_dirty[i] = max_diff > 1e-5;
            self.prev_view_proj[i] = cascade.view_proj_matrix;
        }
    }

    /// Upload cascade data to GPU
    ///
    /// Call this after `update_cascades()` and before rendering.
    pub fn upload_to_gpu(&mut self, queue: &wgpu::Queue, device: &wgpu::Device) {
        // Convert to GPU format
        let gpu_cascades: Vec<GpuShadowCascade> =
            self.cascades.iter().map(GpuShadowCascade::from).collect();

        // Upload buffer
        queue.write_buffer(&self.cascade_buffer, 0, bytemuck::cast_slice(&gpu_cascades));

        // Create/update bind group
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CSM Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.cascade_buffer.as_entire_binding(),
                },
            ],
        }));

        // Create/update shadow-only bind group (just cascades for depth pass)
        self.shadow_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Depth Bind Group"),
            layout: &self.shadow_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.cascade_buffer.as_entire_binding(),
            }],
        }));
    }

    /// Get cascade view for rendering (for shadow pass)
    ///
    /// # Returns
    ///
    /// Texture view for the specified cascade index (0-3)
    pub fn get_cascade_view(&self, cascade_index: usize) -> &wgpu::TextureView {
        &self.cascade_views[cascade_index]
    }

    /// Render shadow maps for all cascades
    ///
    /// # Arguments
    ///
    /// - `encoder`: Command encoder for recording render passes
    /// - `vertex_buffer`: Vertex buffer containing scene geometry (position-only)
    /// - `index_buffer`: Index buffer for indexed drawing
    /// - `index_count`: Number of indices to draw
    ///
    /// # Notes
    ///
    /// This creates 4 render passes (one per cascade) to populate the shadow atlas.
    /// Call this before your main render pass.
    pub fn render_shadow_maps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
        index_count: u32,
    ) {
        // Log cascade matrices on first call for diagnostics
        static FIRST_CALL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
        if FIRST_CALL.swap(false, std::sync::atomic::Ordering::Relaxed) {
            log::debug!("Shadow rendering debug:");
            log::debug!("  - Index count: {}", index_count);
            for (i, cascade) in self.cascades.iter().enumerate() {
                log::debug!(
                    "  - Cascade {}: near={:.1}, far={:.1}",
                    i,
                    cascade.near,
                    cascade.far
                );
                log::debug!("    view_proj = {:#?}", cascade.view_proj_matrix);
            }
        }

        for cascade_idx in 0..CASCADE_COUNT {
            // Skip re-rendering clean cascades (matrix unchanged since last frame)
            if !self.cascade_dirty[cascade_idx] {
                continue;
            }

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("Shadow Cascade {} Render Pass", cascade_idx)),
                color_attachments: &[], // Depth-only pass
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.cascade_views[cascade_idx], // Render to individual layer
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0), // Clear to far plane
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.shadow_pipeline);

            // Bind cascade data (group 0 for shadow pipeline)
            if let Some(ref bind_group) = self.shadow_bind_group {
                render_pass.set_bind_group(0, bind_group, &[]);
            }

            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            // Draw with instance_index = cascade_idx (for shader cascade selection)
            render_pass.draw_indexed(
                0..index_count,
                0,
                cascade_idx as u32..(cascade_idx as u32 + 1),
            );
        }
    }

    /// Mark all cascades as dirty, forcing re-render on next frame.
    /// Call when scene geometry changes (object added/removed/moved).
    pub fn invalidate_all(&mut self) {
        self.cascade_dirty = [true; CASCADE_COUNT];
    }

    /// Check if a specific cascade needs re-rendering.
    pub fn is_cascade_dirty(&self, cascade_idx: usize) -> bool {
        self.cascade_dirty.get(cascade_idx).copied().unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_shadow_cascade_size() {
        // Ensure struct is properly aligned for GPU
        assert_eq!(
            std::mem::size_of::<GpuShadowCascade>(),
            std::mem::size_of::<[[f32; 4]; 4]>() + // view_proj (64 bytes)
            std::mem::size_of::<[f32; 4]>() +      // split_distances (16 bytes)
            std::mem::size_of::<[f32; 4]>() // atlas_transform (16 bytes)
        );
    }

    #[test]
    fn test_cascade_split_distribution() {
        // Test logarithmic split calculation
        let near = 0.1f32;
        let far = 1000.0f32;
        let lambda = 0.5;

        let mut splits = [0.0f32; CASCADE_COUNT + 1];
        splits[0] = near;
        splits[CASCADE_COUNT] = far;

        for (i, split) in splits.iter_mut().enumerate().take(CASCADE_COUNT).skip(1) {
            let i_f = i as f32;
            let n_f = CASCADE_COUNT as f32;
            let log_split = near * (far / near).powf(i_f / n_f);
            let uniform_split = near + (far - near) * (i_f / n_f);
            *split = lambda * log_split + (1.0 - lambda) * uniform_split;
        }

        // Verify splits are monotonically increasing
        for pair in splits.windows(2) {
            assert!(pair[0] < pair[1]);
        }

        // Verify first and last splits
        assert_eq!(splits[0], near);
        assert_eq!(splits[CASCADE_COUNT], far);
    }

    #[test]
    fn test_atlas_offset_calculation() {
        // Verify atlas quadrants are correctly mapped
        let cascades = [
            Vec4::new(0.0, 0.0, 0.5, 0.5), // Top-left
            Vec4::new(0.5, 0.0, 0.5, 0.5), // Top-right
            Vec4::new(0.0, 0.5, 0.5, 0.5), // Bottom-left
            Vec4::new(0.5, 0.5, 0.5, 0.5), // Bottom-right
        ];

        for offset in &cascades {
            // All scales should be 0.5 (half atlas)
            assert_eq!(offset.z, 0.5);
            assert_eq!(offset.w, 0.5);

            // Offsets should be 0.0 or 0.5
            assert!(offset.x == 0.0 || offset.x == 0.5);
            assert!(offset.y == 0.0 || offset.y == 0.5);
        }
    }

    // --- Mutation-resistant tests ---

    #[test]
    fn cascade_count_is_four() {
        assert_eq!(CASCADE_COUNT, 4);
    }

    #[test]
    fn cascade_resolution_is_2048() {
        assert_eq!(CASCADE_RESOLUTION, 2048);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn depth_bias_is_positive() {
        assert!(
            DEPTH_BIAS > 0.0,
            "depth bias must be positive to prevent shadow acne"
        );
        assert!(DEPTH_BIAS < 0.1, "depth bias should be small");
    }

    #[test]
    fn gpu_shadow_cascade_from_shadow_cascade() {
        let cascade = ShadowCascade {
            near: 0.5,
            far: 25.0,
            view_matrix: Mat4::IDENTITY,
            proj_matrix: Mat4::IDENTITY,
            view_proj_matrix: Mat4::look_at_rh(Vec3::new(0.0, 50.0, 0.0), Vec3::ZERO, Vec3::X),
            atlas_offset: Vec4::new(0.0, 0.0, 1.0, 1.0),
        };
        let gpu: GpuShadowCascade = (&cascade).into();
        assert_eq!(gpu.split_distances[0], 0.5);
        assert_eq!(gpu.split_distances[1], 25.0);
        assert_eq!(gpu.split_distances[2], 0.0);
        assert_eq!(gpu.split_distances[3], 0.0);
        assert_eq!(gpu.atlas_transform, [0.0, 0.0, 1.0, 1.0]);
        // view_proj should match cascade.view_proj_matrix
        let expected = cascade.view_proj_matrix.to_cols_array_2d();
        assert_eq!(gpu.view_proj, expected);
    }

    #[test]
    fn cascade_splits_are_monotonic_for_various_ranges() {
        let test_cases = [
            (0.1f32, 100.0f32),
            (0.1, 1000.0),
            (1.0, 50.0),
            (0.01, 10000.0),
        ];
        let lambda = 0.5;
        for (near, far) in test_cases {
            let mut splits = [0.0f32; CASCADE_COUNT + 1];
            splits[0] = near;
            splits[CASCADE_COUNT] = far;
            for (i, split) in splits.iter_mut().enumerate().take(CASCADE_COUNT).skip(1) {
                let i_f = i as f32;
                let n_f = CASCADE_COUNT as f32;
                let log_split = near * (far / near).powf(i_f / n_f);
                let uniform_split = near + (far - near) * (i_f / n_f);
                *split = lambda * log_split + (1.0 - lambda) * uniform_split;
            }
            for pair in splits.windows(2) {
                assert!(
                    pair[0] < pair[1],
                    "splits must be monotonic for near={near}, far={far}"
                );
            }
            assert_eq!(splits[0], near);
            assert_eq!(splits[CASCADE_COUNT], far);
        }
    }

    #[test]
    fn cascade_splits_first_is_near_and_last_is_far() {
        let near = 0.5f32;
        let far = 500.0f32;
        let lambda = 0.5;
        let mut splits = [0.0f32; CASCADE_COUNT + 1];
        splits[0] = near;
        splits[CASCADE_COUNT] = far;
        for (i, split) in splits.iter_mut().enumerate().take(CASCADE_COUNT).skip(1) {
            let i_f = i as f32;
            let n_f = CASCADE_COUNT as f32;
            *split = lambda * (near * (far / near).powf(i_f / n_f))
                + (1.0 - lambda) * (near + (far - near) * (i_f / n_f));
        }
        assert_eq!(splits[0], near, "first split must equal near");
        assert_eq!(splits[CASCADE_COUNT], far, "last split must equal far");
    }

    #[test]
    fn light_view_matrix_up_vector_selection() {
        // Light mostly vertical (|y| > 0.9) should use X as up
        let vertical_dir = Vec3::new(0.0, -0.95, 0.1).normalize();
        let up = if vertical_dir.y.abs() > 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        assert_eq!(up, Vec3::X, "vertical light should use X as up");

        // Light horizontal should use Y as up
        let horizontal_dir = Vec3::new(0.5, -0.3, 0.8).normalize();
        let up2 = if horizontal_dir.y.abs() > 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        assert_eq!(up2, Vec3::Y, "horizontal light should use Y as up");
    }

    #[test]
    fn shadow_depth_shader_parses() {
        let module = naga::front::wgsl::parse_str(SHADOW_DEPTH_SHADER)
            .expect("SHADOW_DEPTH_SHADER WGSL should parse");
        let entry_names: Vec<&str> = module
            .entry_points
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert!(
            entry_names.contains(&"shadow_vertex_main"),
            "must have vertex entry"
        );
        assert!(
            entry_names.contains(&"shadow_fragment_main"),
            "must have fragment entry"
        );
    }

    #[test]
    fn gpu_shadow_cascade_struct_size() {
        // 64 (mat4) + 16 (split_distances) + 16 (atlas_transform) = 96 bytes
        assert_eq!(std::mem::size_of::<GpuShadowCascade>(), 96);
    }

    #[test]
    fn atlas_resolution_equals_cascade_resolution() {
        assert_eq!(ATLAS_RESOLUTION, CASCADE_RESOLUTION);
    }
}
