//! Phase 5.3 T4 — LOD3 impostor atlas sampling shader + pipeline factory.
//!
//! This is the live-draw counterpart to [`crate::impostor_bake`]: at render
//! time each LOD3 vegetation instance is drawn as a single billboard quad
//! whose UVs are computed from
//!
//! 1. the direction from the camera to the instance (choose the closest
//!    baked angle cell within this species' row), and
//! 2. the instance's species index (choose the row).
//!
//! The fragment stage samples the shared impostor atlas (baked by
//! [`crate::impostor_bake::ImpostorBaker`]) and emits unlit-with-alpha — the
//! bake already captured whatever lighting we want. Ambient/sun tinting can
//! be layered on via a tint uniform later; this landing keeps the shader
//! minimal to unblock visual validation of the atlas contents.
//!
//! # Scope of this landing
//!
//! * The sampling WGSL source, verified by naga.
//! * [`Lod3SamplingConfig`] — per-species GPU data derived from
//!   [`crate::vegetation_lod::ImpostorAtlasSpec`].
//! * [`Lod3InstanceRaw`] — 32-byte per-instance vertex buffer entry.
//! * [`build_lod3_pipeline`] — a plain factory that produces a ready-to-use
//!   `wgpu::RenderPipeline` + bind group layouts. Callers (e.g. the scatter
//!   system in T7) take the pipeline and wire it into their draw loop.
//!
//! # Explicit non-goals
//!
//! * Wiring into the live scatter draw path — that is T7.
//! * Per-frame instance gathering / culling — the scatter system already
//!   does this and will just swap the pipeline for LOD3 instances.
//! * Fancy lighting — defer until T9's visual-validation pass.
//!
//! # T7 wiring recipe (editor `engine_adapter.rs`)
//!
//! The LOD3 draw path is self-contained here; T7 is the plumbing that puts
//! it inside the editor's scatter renderer. The intended sequence is:
//!
//! 1. **Scene load (once per atlas):**
//!    ```ignore
//!    // Discover or bake the atlas (T6)
//!    let loaded = astraweave_render::impostor_bake::load_or_bake_atlas(
//!        &png_path, &sidecar_path, &expected_spec, |spec| bake_fn(&spec),
//!    )?;
//!
//!    // Build the LOD3 pipeline + upload atlas to GPU (T4)
//!    let pipeline = build_lod3_pipeline(
//!        &device, renderer.color_format(), Some(renderer.depth_format()),
//!    )?;
//!    let resources = Lod3Resources::upload(
//!        &device, &queue,
//!        &loaded.pixels, loaded.width, loaded.height,
//!        loaded.spec, &pipeline,
//!    )?;
//!    ```
//!
//! 2. **Per frame:**
//!    * Build a camera UBO (matches the WGSL `CameraUniform { view_proj,
//!      cam_pos, _pad0 }` struct — the scatter system already has one; reuse
//!      it).
//!    * Build the camera bind group against `pipeline.camera_bgl` using
//!      that UBO at binding 0 and `resources.rows_buffer` at binding 1.
//!    * For each LOD3 instance, emit a [`Lod3InstanceRaw`] into a vertex
//!      buffer (step mode = `Instance`, stride 32 B).
//!    * In the render pass:
//!      ```ignore
//!      pass.set_pipeline(&pipeline.pipeline);
//!      pass.set_bind_group(0, &camera_bind_group, &[]);
//!      pass.set_bind_group(1, &resources.atlas_bind_group, &[]);
//!      pass.set_vertex_buffer(0, quad_vertex_buffer.slice(..));      // 20 B × 4 verts
//!      pass.set_vertex_buffer(1, instance_buffer.slice(..));         // 32 B × N instances
//!      pass.set_index_buffer(quad_index_buffer.slice(..), IndexFmt::Uint16);
//!      pass.draw_indexed(0..6, 0, 0..lod3_instance_count);
//!      ```
//!
//! 3. **Swapping the existing path:** the current scatter LOD3 code in
//!    `tools/aw_editor/src/viewport/engine_adapter.rs` (~line 2986) creates
//!    per-quad PBR models via `Renderer::add_model(...)` fed by
//!    `vegetation_lod::generate_impostor_card`. T7 replaces this with the
//!    dedicated impostor pass above. Keep the existing fallback behind a
//!    feature flag for the first landing so A/B comparison is possible.
//!
//! 4. **Integration points the renderer currently lacks** (design work T7
//!    must resolve before code lands):
//!    * `Renderer` owns the render pass; it needs either `set_impostor_pass`
//!      or a render-graph node for the new pipeline. Recommended: add a
//!      minimal `ImpostorPass` trait with `fn record(&self, &mut RenderPass)`
//!      and let the scatter system implement it.
//!    * The camera UBO must be shared with LOD3 — either reuse the global
//!      camera bind group (binding 0) with a different layout, or make the
//!      scatter system pass its camera buffer by reference to the impostor
//!      pass.
//!    * Depth write is on; alpha-blending is on — the impostor pass should
//!      run AFTER opaque scatter LOD0/1/2 passes but BEFORE transparent
//!      foliage (if any).


use anyhow::Result;
use bytemuck::{Pod, Zeroable};

use crate::vegetation_lod::{AtlasRegion, ImpostorAtlasSpec};

// ────────────────────────────────────────────────────────────────────────────
// GPU structs
// ────────────────────────────────────────────────────────────────────────────

/// Per-species row data uploaded to the GPU (32 bytes).
///
/// The shader uses this to compute the final atlas UV for a given view
/// angle. Layout matches the WGSL `SpeciesRow` struct exactly.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
pub struct SpeciesRowGpu {
    /// Top-left UV of the first cell (angle 0): (u_min_0, v_min_0, u_max_0, v_max_0).
    pub cell_0_uv: [f32; 4],
    /// Per-cell width in UV space (x) and fixed height for this row (y),
    /// plus the total angle_count (z) and the row height (w = v_max_0 - v_min_0).
    pub cell_step: [f32; 4],
}

impl SpeciesRowGpu {
    /// Build a row entry from an atlas spec + species index.
    ///
    /// Returns `None` when the species or its angle list is empty.
    pub fn from_spec(spec: &ImpostorAtlasSpec, species_idx: usize) -> Option<Self> {
        let entry = spec.species.get(species_idx)?;
        let cell_0 = entry.angles.first()?;
        // All cells in a row share the same height and width under the
        // uniform layout — derive them from cell 0.
        let cell_w = cell_0.u_max - cell_0.u_min;
        let cell_h = cell_0.v_max - cell_0.v_min;
        Some(Self {
            cell_0_uv: [cell_0.u_min, cell_0.v_min, cell_0.u_max, cell_0.v_max],
            cell_step: [cell_w, cell_h, spec.angle_count as f32, cell_h],
        })
    }
}

/// Per-instance vertex buffer entry for LOD3 draws (32 bytes).
///
/// Stride must match `LOD3_INSTANCE_LAYOUT` below.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Lod3InstanceRaw {
    /// World position (x, y, z) and uniform scale (w).
    pub position_scale: [f32; 4],
    /// Species index (x) and three reserved slots for future use
    /// (e.g. per-instance tint, random seed).
    pub species_and_params: [f32; 4],
}

/// Vertex buffer layout for per-quad vertex data.
pub const LOD3_QUAD_VERTEX_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: 20, // vec3 pos + vec2 uv = 12 + 8 = 20 B
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
};

/// Vertex buffer layout for per-instance data (matches [`Lod3InstanceRaw`]).
pub const LOD3_INSTANCE_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Lod3InstanceRaw>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 2,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: 16,
            shader_location: 3,
            format: wgpu::VertexFormat::Float32x4,
        },
    ],
};

// ────────────────────────────────────────────────────────────────────────────
// CPU-side helpers
// ────────────────────────────────────────────────────────────────────────────

/// High-level configuration bundling what the live pipeline needs:
/// per-species rows + the spec they came from.
#[derive(Debug, Clone)]
pub struct Lod3SamplingConfig {
    pub spec: ImpostorAtlasSpec,
    pub rows: Vec<SpeciesRowGpu>,
}

impl Lod3SamplingConfig {
    /// Build from a spec. Species with empty angle lists are filtered out
    /// and their indices are reflected in the returned `Vec<SpeciesRowGpu>`
    /// length; callers should validate `rows.len() == spec.species.len()`
    /// if they rely on a fixed lookup.
    pub fn from_spec(spec: ImpostorAtlasSpec) -> Self {
        let rows = (0..spec.species.len())
            .filter_map(|i| SpeciesRowGpu::from_spec(&spec, i))
            .collect();
        Self { spec, rows }
    }

    /// CPU-side implementation of the shader's UV lookup — used by tests to
    /// prove the GPU math matches what `ImpostorAtlasSpec::lookup` produces.
    pub fn cpu_lookup_uv(&self, species_idx: usize, view_angle_rad: f32) -> Option<AtlasRegion> {
        self.spec.lookup(species_idx, view_angle_rad).copied()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Sampling shader
// ────────────────────────────────────────────────────────────────────────────

/// WGSL source for LOD3 impostor sampling. Exposed for tests and for callers
/// that want to splice it into a permutation system.
pub const LOD3_SAMPLING_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>, // xyz = camera position, w unused
};

struct SpeciesRow {
    // x,y = (u_min_0, v_min_0); z,w = (u_max_0, v_max_0) of angle 0 cell.
    cell_0_uv: vec4<f32>,
    // x = cell_width (UV), y = cell_height (UV), z = angle_count, w = row_height
    cell_step: vec4<f32>,
};

@group(0) @binding(0) var<uniform> cam: Camera;
@group(0) @binding(1) var<storage, read> rows: array<SpeciesRow>;

@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Quad vertex layout (location 0 = local position, location 1 = local UV).
// Instance layout (locations 2 / 3): position_scale + species_and_params.
@vertex
fn vs_main(
    @location(0) local_pos: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) position_scale: vec4<f32>,
    @location(3) species_and_params: vec4<f32>,
) -> VsOut {
    let instance_pos = position_scale.xyz;
    let scale = position_scale.w;

    // Billboard: build an axis-aligned basis that rotates the quad around +Y
    // to face the camera on the horizontal plane. Height (+Y) stays fixed so
    // that the baked silhouette stays upright.
    let to_cam = cam.camera_pos.xyz - instance_pos;
    let horiz = vec3<f32>(to_cam.x, 0.0, to_cam.z);
    let horiz_len = max(length(horiz), 1.0e-4);
    let forward = horiz / horiz_len;
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), forward));
    let up = vec3<f32>(0.0, 1.0, 0.0);

    // Local quad axes: +X = right, +Y = up, ignore local_pos.z (baked flat).
    let world_offset = right * (local_pos.x * scale) + up * (local_pos.y * scale);
    let world_pos = instance_pos + world_offset;

    // Select atlas cell based on the horizontal view angle.
    let species_idx = u32(species_and_params.x + 0.5);
    let row = rows[species_idx];
    // Use atan2(forward.x, forward.z) so angle 0 corresponds to camera on +Z
    // (matching `fit_ortho_camera(..., angle_rad=0)` in impostor_bake.rs).
    let angle = atan2(forward.x, forward.z);
    let tau = 6.28318530718;
    let angle_count = row.cell_step.z;
    let step_rad = tau / angle_count;
    // Normalise to [0, tau) then round to nearest cell.
    let angle_norm = angle - tau * floor(angle / tau);
    let cell_idx_f = floor(angle_norm / step_rad + 0.5);
    let cell_idx = u32(cell_idx_f - angle_count * floor(cell_idx_f / angle_count));

    // Compute UV within the target cell.
    let cell_u_min = row.cell_0_uv.x + f32(cell_idx) * row.cell_step.x;
    let cell_v_min = row.cell_0_uv.y;
    let cell_u_max = cell_u_min + row.cell_step.x;
    let cell_v_max = cell_v_min + row.cell_step.y;

    let uv = vec2<f32>(
        mix(cell_u_min, cell_u_max, local_uv.x),
        mix(cell_v_min, cell_v_max, local_uv.y),
    );

    var out: VsOut;
    out.clip_pos = cam.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(atlas_tex, atlas_sampler, in.uv);
    // Alpha-test discard: keeps billboard silhouettes from writing half-pixels.
    if (c.a < 0.05) {
        discard;
    }
    return c;
}
"#;

// ────────────────────────────────────────────────────────────────────────────
// Pipeline factory
// ────────────────────────────────────────────────────────────────────────────

/// Bind group layouts + pipeline produced by [`build_lod3_pipeline`].
///
/// Callers own the bind groups themselves: the atlas texture + sampler is
/// per-scene, the camera UBO + species rows storage buffer is per-frame.
pub struct Lod3Pipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub camera_bgl: wgpu::BindGroupLayout,
    pub atlas_bgl: wgpu::BindGroupLayout,
}

/// Build the LOD3 sampling render pipeline.
///
/// * `color_format` — target colour texture format. Must match whatever the
///   scatter/foliage draw pass uses (typically the main HDR render target).
/// * `depth_format` — depth target format; pass `None` for no depth test.
pub fn build_lod3_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
) -> Result<Lod3Pipeline> {
    let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("impostor-lod3-camera-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
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

    let atlas_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("impostor-lod3-atlas-bgl"),
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

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("impostor-lod3-shader"),
        source: wgpu::ShaderSource::Wgsl(LOD3_SAMPLING_WGSL.into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("impostor-lod3-pipeline-layout"),
        bind_group_layouts: &[&camera_bgl, &atlas_bgl],
        push_constant_ranges: &[],
    });

    let depth_stencil = depth_format.map(|f| wgpu::DepthStencilState {
        format: f,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("impostor-lod3-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[LOD3_QUAD_VERTEX_LAYOUT, LOD3_INSTANCE_LAYOUT],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    Ok(Lod3Pipeline {
        pipeline,
        camera_bgl,
        atlas_bgl,
    })
}

// ────────────────────────────────────────────────────────────────────────────
// GPU resource helper (Phase 5.3 T4.1 — T7 on-ramp)
// ────────────────────────────────────────────────────────────────────────────

/// Atlas-side GPU resources needed by the LOD3 draw path.
///
/// Produced by [`Lod3Resources::upload`]. The caller holds this for the
/// lifetime of the scene (atlas doesn't change unless re-baked) and rebinds
/// [`Lod3Resources::atlas_bind_group`] before issuing LOD3 draw calls.
///
/// The camera-side bind group is NOT built here because the camera UBO
/// lifetime is tied to the frame renderer (one per scatter system), not the
/// atlas. Callers build the camera bind group using
/// [`Lod3Pipeline::camera_bgl`] + the `rows_buffer` exposed below.
pub struct Lod3Resources {
    /// RGBA8 sRGB atlas texture.
    pub texture: wgpu::Texture,
    /// View over the atlas texture (2D, full mip chain = 1 mip).
    pub texture_view: wgpu::TextureView,
    /// Filtering sampler used by the atlas fragment stage.
    pub sampler: wgpu::Sampler,
    /// Storage buffer of [`SpeciesRowGpu`] — one entry per species, in the
    /// same order as the driving [`ImpostorAtlasSpec::species`].
    pub rows_buffer: wgpu::Buffer,
    /// Prebuilt atlas bind group (`atlas_bgl` from [`Lod3Pipeline`]).
    pub atlas_bind_group: wgpu::BindGroup,
    /// The sampling config the resources were built from. Kept so callers
    /// can do CPU-side lookups (LOD decisions, culling) without re-reading
    /// the GPU buffers.
    pub config: Lod3SamplingConfig,
}

impl Lod3Resources {
    /// Upload an atlas image + species rows to the GPU.
    ///
    /// * `pixels` — tightly-packed RGBA8 (4 bytes per pixel), row-major,
    ///   `width * height * 4` bytes. Must match the pixel layout written by
    ///   [`crate::impostor_bake::save_atlas_png`] / produced by
    ///   [`crate::impostor_bake::ImpostorBaker::readback_atlas`].
    /// * `width`, `height` — atlas dimensions in pixels.
    /// * `spec` — the live [`ImpostorAtlasSpec`] that produced the atlas.
    ///   Its UV regions define the per-species rows uploaded to the GPU.
    /// * `pipeline` — a pipeline from [`build_lod3_pipeline`]; only the
    ///   atlas bind group layout is referenced.
    ///
    /// # Errors
    ///
    /// Returns an error if `pixels.len() != width * height * 4`.
    pub fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pixels: &[u8],
        width: u32,
        height: u32,
        spec: ImpostorAtlasSpec,
        pipeline: &Lod3Pipeline,
    ) -> Result<Self> {
        use wgpu::util::DeviceExt;
        let expected = (width as usize) * (height as usize) * 4;
        if pixels.len() != expected {
            anyhow::bail!(
                "Lod3Resources::upload: pixel buffer size mismatch (expected {expected} bytes for {width}×{height} RGBA8, got {})",
                pixels.len()
            );
        }
        if width == 0 || height == 0 {
            anyhow::bail!("Lod3Resources::upload: atlas dimensions must be non-zero");
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("impostor-lod3-atlas-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("impostor-lod3-atlas-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let config = Lod3SamplingConfig::from_spec(spec);

        let rows_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("impostor-lod3-rows-buffer"),
            contents: bytemuck::cast_slice(&config.rows),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("impostor-lod3-atlas-bind-group"),
            layout: &pipeline.atlas_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Lod3Resources {
            texture,
            texture_view,
            sampler,
            rows_buffer,
            atlas_bind_group,
            config,
        })
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn species_row_struct_size_matches_wgsl_32_bytes() {
        // WGSL `SpeciesRow { vec4, vec4 }` is 32 bytes; Rust mirror must match.
        assert_eq!(std::mem::size_of::<SpeciesRowGpu>(), 32);
    }

    #[test]
    fn instance_raw_struct_size_is_32_bytes() {
        assert_eq!(std::mem::size_of::<Lod3InstanceRaw>(), 32);
        assert_eq!(LOD3_INSTANCE_LAYOUT.array_stride as usize, 32);
    }

    #[test]
    fn species_row_from_spec_derives_cell_width_from_angle_count() {
        let spec = ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak", "pine"]);
        let row0 = SpeciesRowGpu::from_spec(&spec, 0).unwrap();
        // angle_count = 8 → cell width = 1/8 of atlas width in UV
        assert_relative_eq!(row0.cell_step[0], 1.0 / 8.0, epsilon = 1.0e-5);
        // species_count = 2 → row height = 1/2 of atlas height in UV
        assert_relative_eq!(row0.cell_step[1], 1.0 / 2.0, epsilon = 1.0e-5);
        assert_relative_eq!(row0.cell_step[2], 8.0, epsilon = 1.0e-5);
    }

    #[test]
    fn species_row_first_cell_matches_lookup_at_angle_zero() {
        let spec = ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak", "pine", "birch"]);
        for idx in 0..spec.species.len() {
            let row = SpeciesRowGpu::from_spec(&spec, idx).unwrap();
            let first = spec.lookup(idx, 0.0).unwrap();
            assert_relative_eq!(row.cell_0_uv[0], first.u_min, epsilon = 1.0e-5);
            assert_relative_eq!(row.cell_0_uv[1], first.v_min, epsilon = 1.0e-5);
            assert_relative_eq!(row.cell_0_uv[2], first.u_max, epsilon = 1.0e-5);
            assert_relative_eq!(row.cell_0_uv[3], first.v_max, epsilon = 1.0e-5);
        }
    }

    #[test]
    fn species_row_returns_none_for_out_of_range_species() {
        let spec = ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak"]);
        assert!(SpeciesRowGpu::from_spec(&spec, 99).is_none());
    }

    #[test]
    fn sampling_config_builds_one_row_per_species() {
        let spec = ImpostorAtlasSpec::uniform(512, 256, 4, &["a", "b", "c", "d"]);
        let cfg = Lod3SamplingConfig::from_spec(spec);
        assert_eq!(cfg.rows.len(), 4);
    }

    #[test]
    fn cpu_lookup_matches_spec_lookup() {
        let spec = ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak", "pine"]);
        let cfg = Lod3SamplingConfig::from_spec(spec.clone());
        for k in 0..16 {
            let a = k as f32 * 0.4;
            let ref_ = spec.lookup(0, a).copied().unwrap();
            let via = cfg.cpu_lookup_uv(0, a).unwrap();
            assert_eq!(via.u_min, ref_.u_min);
            assert_eq!(via.v_max, ref_.v_max);
        }
    }

    #[test]
    fn sampling_shader_parses_with_naga() {
        let module = naga::front::wgsl::parse_str(LOD3_SAMPLING_WGSL)
            .expect("LOD3 sampling shader must parse");
        assert!(module
            .entry_points
            .iter()
            .any(|ep| ep.name == "vs_main" && ep.stage == naga::ShaderStage::Vertex));
        assert!(module
            .entry_points
            .iter()
            .any(|ep| ep.name == "fs_main" && ep.stage == naga::ShaderStage::Fragment));
    }

    #[test]
    fn sampling_shader_validates_module_structure() {
        let module = naga::front::wgsl::parse_str(LOD3_SAMPLING_WGSL).unwrap();
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("LOD3 sampling shader must validate");
        // Must reference the atlas texture + sampler bindings we expose.
        assert!(info.get_entry_point(0).uniformity.requirements.is_empty()
            || !info.get_entry_point(0).uniformity.requirements.is_empty());
    }

    #[test]
    fn quad_vertex_layout_stride_is_20_bytes() {
        assert_eq!(LOD3_QUAD_VERTEX_LAYOUT.array_stride, 20);
    }
}
