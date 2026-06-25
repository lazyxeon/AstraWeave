//! Water rendering system with animated Gerstner waves
//!
//! Provides a chunked, camera-distance LOD ocean surface with:
//! - 4 summed Gerstner wave components
//! - Fresnel-based reflections
//! - Depth-based color blending
//! - Animated foam on wave crests
//! - Discrete chunk-grid LOD (finer near the camera, coarser toward the horizon)
//! - Per-chunk skirts that hide LOD-boundary cracks
//! - Real water-level control via a shader uniform (no baked mesh Y)
//!
//! ## LOD / chunking model (W-series W.2a)
//!
//! The surface is a discrete grid of square chunks (`CHUNK_SIZE` units each)
//! centered on the camera. Each frame, every chunk in the `(2*GRID_RADIUS+1)²`
//! block around the camera picks an LOD by its distance, and chunks are drawn
//! **instanced per LOD** — one indexed-instanced draw call per LOD level,
//! independent of how many chunks chose that level. Wave displacement is a pure
//! function of world XZ, so chunk meshes are world-stable (no swimming) and
//! shared LOD-boundary vertices agree exactly; the only mismatch is curve-vs-chord
//! along a coarse neighbour's edge (≤ wave amplitude), which the per-chunk skirt
//! covers. This replaces the former single hardcoded `generate_water_plane(500,128)`
//! plane.

use glam::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;

// ── Chunked LOD configuration ────────────────────────────────────────────────

/// World-space edge length of one water chunk (units).
const CHUNK_SIZE: f32 = 64.0;
/// Chunk radius around the camera: a `(2*GRID_RADIUS+1)²` block is active.
const GRID_RADIUS: i32 = 8;
/// Maximum simultaneously-active chunks (instance-buffer capacity per LOD).
const MAX_CHUNKS: usize = ((2 * GRID_RADIUS + 1) * (2 * GRID_RADIUS + 1)) as usize; // 289
/// Per-LOD grid subdivision count (finest → coarsest).
const LOD_SUBDIVS: [u32; 4] = [32, 16, 8, 4];
/// Upper distance bound (world units, camera→chunk-center) for each LOD band.
/// The last entry is the catch-all for everything farther out.
const LOD_DISTANCES: [f32; 4] = [110.0, 220.0, 360.0, f32::INFINITY];
/// How far skirt vertices drop below the displaced surface edge (units).
/// Must exceed the maximum LOD-boundary height mismatch (≤ total wave
/// amplitude ≈ 1.65 units) by a wide margin so no crack outruns the skirt.
const SKIRT_DEPTH: f32 = 8.0;
/// Default water level (world Y). Matches the former baked plane Y so existing
/// consumers that never call [`WaterRenderer::set_water_level`] are unchanged.
const DEFAULT_WATER_LEVEL: f32 = 2.0;

// ── Weave-response deformation (W.2c) ────────────────────────────────────────

/// Maximum simultaneously-active weave-deformation instances (W.2c #4 ratification).
/// Built to 8 — trivially raisable later, but the shader loops to exactly this bound
/// so unused slots cost nothing beyond a `kind == None` early-continue.
pub const MAX_WEAVE_INSTANCES: usize = 8;

/// Peak world-space displacement a single weave can apply, in units. Equals
/// [`SKIRT_DEPTH`] so a `Part`/`Raise` at full intensity can never outrun the LOD
/// skirt and re-expose a seam (W.2c.2 skirt constraint). `intensity ∈ [0,1]` scales
/// against this; the shader bounds each instance's height contribution to ±this.
pub const WEAVE_MAX_DEFORM: f32 = SKIRT_DEPTH;

/// Which weave-response deformation a [`WeaveInstance`] applies. Encoded as `u32`
/// in the GPU instance; `None` (0) marks an inactive slot the shader skips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WeaveKind {
    /// Inactive slot — identity (no deformation).
    None = 0,
    /// Displace the surface *down*, exposing a corridor/well (additive negative height).
    Part = 1,
    /// Lift the surface *up* — a dome/ridge (additive positive height).
    Raise = 2,
    /// Lock/flatten the surface (suppress waves) + frozen material look. A mask, not
    /// a displacement.
    Freeze = 3,
}

/// A runtime weave-deformation instance: a normalized analytical profile placed at a
/// world location. **Location lives only here.** The profile (selected by
/// [`kind`](Self::kind)) is a position-agnostic shape in *local* space; the shader
/// maps `world → local = rotate(world_xz − position, −orientation) / radius`,
/// evaluates the profile, and scales by [`intensity`](Self::intensity). A single
/// authored `Part` profile therefore serves a part-*anywhere* verb — only the
/// instance carries where / how-big / which-way / how-strong.
///
/// Representation-agnostic (W.2c #2): a future deformation-texture profile would add a
/// `profile_id` atlas index here without changing position/radius/orientation/
/// intensity or the shader-feeding interface.
#[derive(Debug, Clone, Copy)]
pub struct WeaveInstance {
    /// Which deformation this is (also selects the analytical profile in W.2c.2).
    pub kind: WeaveKind,
    /// World-space XZ center where the effect is placed (runtime input).
    pub position: Vec2,
    /// World-space footprint radius in units (runtime input): local `r = 1` maps here.
    pub radius: f32,
    /// Yaw orientation in radians (runtime input) — orients directional profiles.
    pub orientation: f32,
    /// Normalized magnitude in `[0, 1]` (runtime input). For `Part`/`Raise` it scales
    /// the height against [`WEAVE_MAX_DEFORM`]; for `Freeze` it is the mask strength.
    pub intensity: f32,
    /// Animation phase / age in seconds (runtime input) — drives shader-side ramps.
    pub phase: f32,
}

impl WeaveInstance {
    fn to_raw(self) -> WeaveInstanceRaw {
        WeaveInstanceRaw {
            position: self.position.into(),
            radius: self.radius.max(0.0),
            orientation: self.orientation,
            // Bound magnitude to [0,1]; with WEAVE_MAX_DEFORM == SKIRT_DEPTH in the
            // shader this keeps any single deformation within skirt tolerance.
            intensity: self.intensity.clamp(0.0, 1.0),
            phase: self.phase,
            kind: self.kind as u32,
            _pad: 0,
        }
    }
}

/// GPU-side weave instance (std140-compatible: 32 B = 2×vec4, 16-aligned so it packs
/// cleanly into the `WaterUniforms` uniform array). Public only because it appears in
/// the public `WaterUniforms` layout; its fields are private — construct weaves via
/// [`WeaveInstance`] and [`WaterRenderer::set_weave_instances`].
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WeaveInstanceRaw {
    position: [f32; 2], // 0..8
    radius: f32,        // 8..12
    orientation: f32,   // 12..16
    intensity: f32,     // 16..20
    phase: f32,         // 20..24
    kind: u32,          // 24..28
    _pad: u32,          // 28..32
}

impl WeaveInstanceRaw {
    const ZERO: Self = Self {
        position: [0.0, 0.0],
        radius: 0.0,
        orientation: 0.0,
        intensity: 0.0,
        phase: 0.0,
        kind: 0,
        _pad: 0,
    };
}

/// Water uniforms for shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaterUniforms {
    pub view_proj: [[f32; 4]; 4],      // 0-64
    pub camera_pos: [f32; 3],          // 64-76
    pub time: f32,                     // 76-80
    pub water_color_deep: [f32; 3],    // 80-92
    pub _pad0: f32,                    // 92-96
    pub water_color_shallow: [f32; 3], // 96-108
    pub _pad1: f32,                    // 108-112
    pub foam_color: [f32; 3],          // 112-124
    pub foam_threshold: f32,           // 124-128
    pub rain_intensity: f32,           // 128-132
    pub ripple_scale: f32,             // 132-136
    pub ripple_strength: f32,          // 136-140
    pub water_level: f32,              // 140-144
    pub skirt_depth: f32,              // 144-148
    pub _pad2: f32,                    // 148-152
    pub _pad3: f32,                    // 152-156
    pub _pad4: f32,                    // 156-160
    // W.2b — refraction + depth-foam. inv_view_proj sits at offset 160 (16-aligned)
    // so the mat4 satisfies std140 alignment.
    pub inv_view_proj: [[f32; 4]; 4],  // 160-224 — reconstruct scene world pos from depth
    pub screen_size: [f32; 2],         // 224-232 — for frag → screen-UV
    pub refraction_strength: f32,      // 232-236 — normal-driven scene-color distortion
    pub foam_depth_band: f32,          // 236-240 — world-space shoreline foam width
    // W.2c — weave-response deformation. The `weave_instances` array is 16-aligned at
    // offset 256 (std140 array-of-struct stride 32); 8 × 32 B = 256 B → 512 B total.
    pub weave_count: u32,              // 240-244
    pub _pad5: u32,                    // 244-248
    pub _pad6: u32,                    // 248-252
    pub _pad7: u32,                    // 252-256
    pub weave_instances: [WeaveInstanceRaw; MAX_WEAVE_INSTANCES], // 256-512
}

impl Default for WaterUniforms {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 5.0, -10.0],
            time: 0.0,
            water_color_deep: [0.02, 0.08, 0.2], // Deep ocean blue
            _pad0: 0.0,
            water_color_shallow: [0.1, 0.4, 0.5], // Turquoise shallow
            _pad1: 0.0,
            foam_color: [0.95, 0.98, 1.0], // White foam
            foam_threshold: 0.6,
            rain_intensity: 0.0,
            ripple_scale: 4.0,
            ripple_strength: 0.15,
            water_level: DEFAULT_WATER_LEVEL,
            skirt_depth: SKIRT_DEPTH,
            _pad2: 0.0,
            _pad3: 0.0,
            _pad4: 0.0,
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            screen_size: [1920.0, 1080.0],
            refraction_strength: 0.02,
            foam_depth_band: 1.5,
            weave_count: 0,
            _pad5: 0,
            _pad6: 0,
            _pad7: 0,
            weave_instances: [WeaveInstanceRaw::ZERO; MAX_WEAVE_INSTANCES],
        }
    }
}

/// Water vertex (position + UV). Surface vertices carry local Y = 0; skirt
/// vertices carry a sentinel local Y = -1.0 (the shader drops them by
/// `skirt_depth` *after* wave displacement).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaterVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl WaterVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WaterVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Per-chunk instance data: the world-XZ center of the chunk. Added to each
/// tile vertex's local XZ in the vertex shader to place the chunk in the world.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkInstance {
    offset: [f32; 2],
}

impl ChunkInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ChunkInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

/// A pre-baked tile mesh for one LOD level (grid + skirt).
struct LodMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

/// Water rendering system
pub struct WaterRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    /// Sampler for the scene-color refraction tap (linear, clamp).
    sampler: wgpu::Sampler,
    /// 1×1 dummy scene textures keep the initial bind group valid before the
    /// renderer wires real scene-color/depth views (W.2b). Held alive (the bind
    /// group Arc-clones the views, but per the crate convention textures are kept
    /// explicitly) until the first `prepare_scene` swaps in real textures.
    _dummy_scene_color: wgpu::Texture,
    _dummy_depth: wgpu::Texture,
    /// Resource generation the current `bind_group` was built against
    /// (`u64::MAX` = built with dummy scene textures, not yet wired).
    scene_gen: u64,
    /// One pre-baked mesh per LOD level (`LOD_SUBDIVS`).
    lod_meshes: Vec<LodMesh>,
    /// Per-LOD instance buffer (chunk offsets selected for that LOD this frame).
    instance_buffers: Vec<wgpu::Buffer>,
    /// Per-LOD live instance count for the current frame.
    instance_counts: Vec<u32>,
    uniforms: WaterUniforms,
}

impl WaterRenderer {
    /// Create a new water renderer
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        // Load shader
        let shader_source = include_str!("shaders/water.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Uniform buffer
        let uniforms = WaterUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water_uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group layout — W.2b: 4 entries (uniform + scene-color + scene-depth
        // + sampler) so the fragment shader can refract the scene behind the water
        // and compute the depth-delta shoreline foam.
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water_bind_group_layout"),
            entries: &[
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
                // @1 scene color (opaque HDR snapshot) — sampled for refraction.
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
                // @2 scene depth — read via textureLoad for the foam delta.
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
                // @3 sampler for the scene-color tap.
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("water_scene_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 1×1 dummy scene textures so the initial bind group is valid before the
        // renderer wires real scene-color/depth views via `prepare_scene`.
        let dummy_scene_color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("water_dummy_scene_color"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy_scene_color_view =
            dummy_scene_color.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("water_dummy_depth"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy_depth_view = dummy_depth.create_view(&wgpu::TextureViewDescriptor::default());

        // Initial bind group references the dummy scene textures.
        let bind_group = Self::build_bind_group(
            device,
            &bind_group_layout,
            &uniform_buffer,
            &dummy_scene_color_view,
            &dummy_depth_view,
            &sampler,
        );

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Render pipeline with alpha blending.
        // Two vertex buffers: per-vertex tile geometry + per-instance chunk offset.
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[WaterVertex::desc(), ChunkInstance::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // Top surface (normal +Y) is CCW-front when viewed from above and
                // skirt walls are wound outward-front, so back-face culling keeps
                // the correct faces. The submerged/underside two-sided case is a
                // deferred effects-phase concern (W-series Gemini triage §E).
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: false, // Transparent, don't write depth
                depth_compare: wgpu::CompareFunction::LessEqual, // Normal depth testing
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pre-bake one tile mesh per LOD level, plus an instance buffer per LOD.
        let mut lod_meshes = Vec::with_capacity(LOD_SUBDIVS.len());
        let mut instance_buffers = Vec::with_capacity(LOD_SUBDIVS.len());
        let mut instance_counts = Vec::with_capacity(LOD_SUBDIVS.len());
        for (lod, &subdiv) in LOD_SUBDIVS.iter().enumerate() {
            let (vertices, indices) = Self::generate_tile(CHUNK_SIZE, subdiv);
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("water_tile_vb_lod{lod}")),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("water_tile_ib_lod{lod}")),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            lod_meshes.push(LodMesh {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
            });
            instance_buffers.push(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("water_instances_lod{lod}")),
                size: (MAX_CHUNKS * std::mem::size_of::<ChunkInstance>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            instance_counts.push(0u32);
        }

        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            sampler,
            _dummy_scene_color: dummy_scene_color,
            _dummy_depth: dummy_depth,
            scene_gen: u64::MAX,
            lod_meshes,
            instance_buffers,
            instance_counts,
            uniforms,
        }
    }

    /// Build the 4-entry water bind group (uniform + scene-color + scene-depth + sampler).
    fn build_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform_buffer: &wgpu::Buffer,
        scene_color_view: &wgpu::TextureView,
        scene_depth_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(scene_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(scene_depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// Generate a subdivided water plane (surface grid only, local Y = 0,
    /// centered at the origin). Reusable mesh primitive; [`Self::generate_tile`]
    /// wraps it with a skirt.
    fn generate_water_plane(size: f32, subdivisions: u32) -> (Vec<WaterVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let half_size = size / 2.0;
        let step = size / subdivisions as f32;

        // Generate vertices
        for z in 0..=subdivisions {
            for x in 0..=subdivisions {
                let pos_x = -half_size + x as f32 * step;
                let pos_z = -half_size + z as f32 * step;
                let u = x as f32 / subdivisions as f32;
                let v = z as f32 / subdivisions as f32;

                vertices.push(WaterVertex {
                    position: [pos_x, 0.0, pos_z], // local Y = 0; world Y from uniform
                    uv: [u, v],
                });
            }
        }

        // Generate indices
        for z in 0..subdivisions {
            for x in 0..subdivisions {
                let top_left = z * (subdivisions + 1) + x;
                let top_right = top_left + 1;
                let bottom_left = (z + 1) * (subdivisions + 1) + x;
                let bottom_right = bottom_left + 1;

                // First triangle
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                // Second triangle
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        (vertices, indices)
    }

    /// Append one outward-facing skirt wall segment between two adjacent top-edge
    /// vertices. Bottom twins reuse the top XZ with sentinel local Y = -1.0. The
    /// fixed winding `(a_top, b_top, a_bot)` + `(b_top, b_bot, a_bot)` yields an
    /// outward horizontal normal when callers traverse each edge in the direction
    /// documented in [`Self::generate_tile`].
    fn push_wall(
        vertices: &mut Vec<WaterVertex>,
        indices: &mut Vec<u32>,
        a_top: u32,
        b_top: u32,
    ) {
        let a = vertices[a_top as usize];
        let b = vertices[b_top as usize];
        let a_bot = vertices.len() as u32;
        vertices.push(WaterVertex {
            position: [a.position[0], -1.0, a.position[2]],
            uv: a.uv,
        });
        let b_bot = vertices.len() as u32;
        vertices.push(WaterVertex {
            position: [b.position[0], -1.0, b.position[2]],
            uv: b.uv,
        });
        indices.extend_from_slice(&[a_top, b_top, a_bot, b_top, b_bot, a_bot]);
    }

    /// Generate a chunk tile = surface grid + a perimeter skirt that hides
    /// LOD-boundary cracks. Edges are traversed so the fixed skirt winding faces
    /// outward (away from the chunk center): TOP +x, BOTTOM -x, LEFT -z, RIGHT +z.
    fn generate_tile(size: f32, subdivisions: u32) -> (Vec<WaterVertex>, Vec<u32>) {
        let (mut vertices, mut indices) = Self::generate_water_plane(size, subdivisions);
        let n = subdivisions;
        let stride = n + 1;

        // TOP edge (z = 0 row), traverse x: 0 → n
        for x in 0..n {
            Self::push_wall(&mut vertices, &mut indices, x, x + 1);
        }
        // BOTTOM edge (z = n row), traverse x: n → 0
        let zbase = n * stride;
        for x in (1..=n).rev() {
            Self::push_wall(&mut vertices, &mut indices, zbase + x, zbase + x - 1);
        }
        // LEFT edge (x = 0 col), traverse z: n → 0
        for z in (1..=n).rev() {
            Self::push_wall(&mut vertices, &mut indices, z * stride, (z - 1) * stride);
        }
        // RIGHT edge (x = n col), traverse z: 0 → n
        for z in 0..n {
            Self::push_wall(&mut vertices, &mut indices, z * stride + n, (z + 1) * stride + n);
        }

        (vertices, indices)
    }

    /// Whether any LOD band has chunks to draw this frame. False before the first
    /// [`Self::update`] (e.g. the editor dormant-water case where `update_water`
    /// is never called), letting the renderer skip the scene-color snapshot + pass.
    pub fn has_visible_chunks(&self) -> bool {
        self.instance_counts.iter().any(|&c| c > 0)
    }

    /// Select an LOD band (index into `LOD_SUBDIVS`) for a camera→chunk distance.
    fn lod_for_distance(dist: f32) -> usize {
        LOD_DISTANCES
            .iter()
            .position(|&d| dist < d)
            .unwrap_or(LOD_SUBDIVS.len() - 1)
    }

    /// Update water state for animation and recompute the per-LOD chunk set from
    /// the camera position. Call once per frame before [`Self::render`].
    pub fn update(&mut self, queue: &wgpu::Queue, view_proj: Mat4, camera_pos: Vec3, time: f32) {
        self.uniforms.view_proj = view_proj.to_cols_array_2d();
        // Inverse view-proj reconstructs the world position of the opaque scene
        // behind the water from its sampled depth (W.2b depth-delta foam).
        self.uniforms.inv_view_proj = view_proj.inverse().to_cols_array_2d();
        self.uniforms.camera_pos = camera_pos.into();
        self.uniforms.time = time;
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));

        // Assign the active chunk block around the camera to LOD bands.
        let cam_cx = (camera_pos.x / CHUNK_SIZE).floor() as i32;
        let cam_cz = (camera_pos.z / CHUNK_SIZE).floor() as i32;
        let mut per_lod: Vec<Vec<ChunkInstance>> =
            (0..LOD_SUBDIVS.len()).map(|_| Vec::new()).collect();

        for dz in -GRID_RADIUS..=GRID_RADIUS {
            for dx in -GRID_RADIUS..=GRID_RADIUS {
                let cx = cam_cx + dx;
                let cz = cam_cz + dz;
                let center_x = (cx as f32 + 0.5) * CHUNK_SIZE;
                let center_z = (cz as f32 + 0.5) * CHUNK_SIZE;
                let dx_w = center_x - camera_pos.x;
                let dz_w = center_z - camera_pos.z;
                let dist = (dx_w * dx_w + dz_w * dz_w).sqrt();
                let lod = Self::lod_for_distance(dist);
                per_lod[lod].push(ChunkInstance {
                    offset: [center_x, center_z],
                });
            }
        }

        for (lod, chunks) in per_lod.iter().enumerate() {
            self.instance_counts[lod] = chunks.len() as u32;
            if !chunks.is_empty() {
                queue.write_buffer(
                    &self.instance_buffers[lod],
                    0,
                    bytemuck::cast_slice(chunks),
                );
            }
        }
    }

    /// Wire the scene-color snapshot + scene-depth views into the water bind group
    /// and upload the screen size, immediately before the water pass (W.2b). The
    /// bind group is only rebuilt when `resource_gen` changes (resize); the uniform
    /// upload is cheap and unconditional. Must be called each frame before
    /// [`Self::render`] in the split water pass.
    pub fn prepare_scene(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene_color_view: &wgpu::TextureView,
        scene_depth_view: &wgpu::TextureView,
        screen_size: [f32; 2],
        resource_gen: u64,
    ) {
        if self.scene_gen != resource_gen {
            self.bind_group = Self::build_bind_group(
                device,
                &self.bind_group_layout,
                &self.uniform_buffer,
                scene_color_view,
                scene_depth_view,
                &self.sampler,
            );
            self.scene_gen = resource_gen;
        }
        // Patch only screen_size — the rest of the block was uploaded by `update()`
        // this frame, so re-uploading all 240 B here would be redundant.
        self.uniforms.screen_size = screen_size;
        let off = std::mem::offset_of!(WaterUniforms, screen_size) as u64;
        queue.write_buffer(&self.uniform_buffer, off, bytemuck::bytes_of(&self.uniforms.screen_size));
    }

    /// Set water level (world Y). Takes effect on the next [`Self::update`] or
    /// [`Self::write_uniforms`] upload.
    pub fn set_water_level(&mut self, level: f32) {
        self.uniforms.water_level = level;
    }

    /// Current water level (world Y).
    pub fn water_level(&self) -> f32 {
        self.uniforms.water_level
    }

    /// Upload the current uniform block immediately. Used by callers (e.g. the
    /// editor water-level knob) that must see a setter take effect without
    /// waiting for the next per-frame [`Self::update`].
    pub fn write_uniforms(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));
    }

    /// Set biome-driven water colors (deep, shallow, foam).
    ///
    /// Colours are applied on the next [`Self::update`] call which
    /// uploads the uniform buffer to the GPU.
    pub fn set_water_colors(&mut self, deep: Vec3, shallow: Vec3, foam: Vec3) {
        self.uniforms.water_color_deep = deep.into();
        self.uniforms.water_color_shallow = shallow.into();
        self.uniforms.foam_color = foam.into();
    }

    /// Set rain intensity for ripple effects on the water surface.
    ///
    /// `intensity`: 0.0 = no rain, 1.0 = heavy rain.
    /// Applied on the next [`Self::update`] call.
    pub fn set_rain_intensity(&mut self, intensity: f32) {
        self.uniforms.rain_intensity = intensity.clamp(0.0, 1.0);
    }

    /// Set the active weave-deformation instances (W.2c). At most
    /// [`MAX_WEAVE_INSTANCES`] are honored; any beyond the ceiling are dropped. An
    /// **empty slice clears all weaves** — the surface is then identical to the
    /// no-weave surface (the deformation is additive-zero / identity at zero
    /// instances). Instances ride in [`WaterUniforms`] and upload on the next
    /// [`Self::update`].
    pub fn set_weave_instances(&mut self, instances: &[WeaveInstance]) {
        let n = instances.len().min(MAX_WEAVE_INSTANCES);
        for (slot, inst) in self.uniforms.weave_instances.iter_mut().zip(&instances[..n]) {
            *slot = inst.to_raw();
        }
        // Clear unused slots so a previously-set instance can't linger active.
        for slot in self.uniforms.weave_instances.iter_mut().skip(n) {
            *slot = WeaveInstanceRaw::ZERO;
        }
        self.uniforms.weave_count = n as u32;
    }

    /// Remove all active weave deformations (the surface returns to identity).
    pub fn clear_weave_instances(&mut self) {
        self.uniforms.weave_instances = [WeaveInstanceRaw::ZERO; MAX_WEAVE_INSTANCES];
        self.uniforms.weave_count = 0;
    }

    /// Number of active weave instances this frame (diagnostics / tests).
    pub fn weave_count(&self) -> u32 {
        self.uniforms.weave_count
    }

    /// Get current water colors (deep, shallow, foam).
    pub fn water_colors(&self) -> (Vec3, Vec3, Vec3) {
        (
            Vec3::from(self.uniforms.water_color_deep),
            Vec3::from(self.uniforms.water_color_shallow),
            Vec3::from(self.uniforms.foam_color),
        )
    }

    /// Render the water surface (one instanced draw per active LOD band).
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        for (lod, mesh) in self.lod_meshes.iter().enumerate() {
            let count = self.instance_counts[lod];
            if count == 0 {
                continue;
            }
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffers[lod].slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.index_count, 0, 0..count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_plane_generation() {
        let (vertices, indices) = WaterRenderer::generate_water_plane(10.0, 4);
        assert_eq!(vertices.len(), 25); // (4+1)^2
        assert_eq!(indices.len(), 96); // 4*4*6
    }

    #[test]
    fn test_tile_has_skirt() {
        // Tile = grid + perimeter skirt. Skirt adds 4*subdiv segments, each with
        // 2 new bottom verts and 2 triangles (6 indices).
        let subdiv = 4u32;
        let (grid_v, grid_i) = WaterRenderer::generate_water_plane(10.0, subdiv);
        let (tile_v, tile_i) = WaterRenderer::generate_tile(10.0, subdiv);
        let segments = 4 * subdiv as usize;
        assert_eq!(tile_v.len(), grid_v.len() + segments * 2);
        assert_eq!(tile_i.len(), grid_i.len() + segments * 6);

        // Skirt vertices use the sentinel local Y = -1.0; surface vertices are 0.
        assert!(tile_v.iter().any(|v| v.position[1] < -0.5));
        assert!(grid_v.iter().all(|v| v.position[1] == 0.0));
    }

    #[test]
    fn test_lod_for_distance() {
        assert_eq!(WaterRenderer::lod_for_distance(0.0), 0);
        assert_eq!(WaterRenderer::lod_for_distance(150.0), 1);
        assert_eq!(WaterRenderer::lod_for_distance(300.0), 2);
        assert_eq!(WaterRenderer::lod_for_distance(10_000.0), 3);
    }

    #[test]
    fn test_uniforms_size() {
        // Ensure uniform struct is properly aligned for GPU (16-byte multiple).
        // W.2b grew it to 240 B (inv_view_proj mat4 + screen_size + refraction
        // params); W.2c appended the weave array → 512 B. inv_view_proj stays at
        // offset 160; the weave array is 16-aligned at offset 256 (8 × 32 B).
        assert_eq!(std::mem::size_of::<WaterUniforms>(), 512);
        assert_eq!(std::mem::offset_of!(WaterUniforms, inv_view_proj), 160);
        assert_eq!(std::mem::offset_of!(WaterUniforms, weave_count), 240);
        assert_eq!(std::mem::offset_of!(WaterUniforms, weave_instances), 256);
        // GPU instance must be 32 B (2×vec4) so the std140 array stride matches WGSL.
        assert_eq!(std::mem::size_of::<WeaveInstanceRaw>(), 32);
    }

    #[test]
    fn test_water_vertex_desc() {
        let desc = WaterVertex::desc();
        assert_eq!(desc.array_stride, std::mem::size_of::<WaterVertex>() as u64);
        assert_eq!(desc.attributes.len(), 2);
    }

    #[test]
    fn test_chunk_instance_desc() {
        let desc = ChunkInstance::desc();
        assert_eq!(desc.array_stride, std::mem::size_of::<ChunkInstance>() as u64);
        assert_eq!(desc.step_mode, wgpu::VertexStepMode::Instance);
        assert_eq!(desc.attributes[0].shader_location, 2);
    }

    #[test]
    fn test_water_renderer_new_and_update() {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await;
            if let Ok(adapter) = adapter {
                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor::default())
                    .await
                    .unwrap();
                let mut renderer = WaterRenderer::new(
                    &device,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureFormat::Depth32Float,
                );

                // One pre-baked mesh per LOD band.
                assert_eq!(renderer.lod_meshes.len(), LOD_SUBDIVS.len());

                // W-FU-2 regression guard: before the first update() the renderer
                // is dormant (no chunks) and run_water_pass must skip it. This is
                // exactly the state the editor was stuck in until update_water was
                // wired — a fresh WaterRenderer with has_visible_chunks() == false.
                assert!(
                    !renderer.has_visible_chunks(),
                    "fresh WaterRenderer must report no visible chunks (dormant)"
                );

                let view_proj = Mat4::IDENTITY;
                let camera_pos = Vec3::new(1.0, 2.0, 3.0);
                let time = 10.0;

                renderer.update(&queue, view_proj, camera_pos, time);

                // After one update() (what update_water drives every frame) the
                // chunk set is populated, so has_visible_chunks() flips true and
                // the post-opaque water pass runs instead of early-returning.
                assert!(
                    renderer.has_visible_chunks(),
                    "after update() the water renderer must have visible chunks"
                );

                assert_eq!(renderer.uniforms.time, 10.0);
                assert_eq!(renderer.uniforms.camera_pos, [1.0, 2.0, 3.0]);

                // The full (2*GRID_RADIUS+1)^2 block is assigned to some LOD.
                let total: u32 = renderer.instance_counts.iter().sum();
                assert_eq!(total as usize, MAX_CHUNKS);

                // Real water-level control.
                renderer.set_water_level(5.0);
                assert_eq!(renderer.water_level(), 5.0);
                assert_eq!(renderer.uniforms.water_level, 5.0);

                // W.2c weave instance list: zero by default (surface = identity).
                assert_eq!(renderer.weave_count(), 0);
                let part = WeaveInstance {
                    kind: WeaveKind::Part,
                    position: Vec2::new(10.0, -4.0),
                    radius: 20.0,
                    orientation: 0.5,
                    intensity: 2.0, // out-of-range — must clamp to 1.0 in to_raw
                    phase: 0.0,
                };
                renderer.set_weave_instances(&[
                    part,
                    WeaveInstance { kind: WeaveKind::Raise, ..part },
                    WeaveInstance { kind: WeaveKind::Freeze, ..part },
                ]);
                assert_eq!(renderer.weave_count(), 3);
                // intensity clamped to [0,1] (skirt-tolerance guard).
                assert_eq!(renderer.uniforms.weave_instances[0].intensity, 1.0);
                assert_eq!(renderer.uniforms.weave_instances[0].kind, WeaveKind::Part as u32);

                // Ceiling: more than MAX_WEAVE_INSTANCES are dropped.
                let many = vec![part; MAX_WEAVE_INSTANCES + 5];
                renderer.set_weave_instances(&many);
                assert_eq!(renderer.weave_count() as usize, MAX_WEAVE_INSTANCES);

                // Clear → back to identity.
                renderer.clear_weave_instances();
                assert_eq!(renderer.weave_count(), 0);
                assert_eq!(renderer.uniforms.weave_instances[0].kind, WeaveKind::None as u32);
            }
        });
    }
}
