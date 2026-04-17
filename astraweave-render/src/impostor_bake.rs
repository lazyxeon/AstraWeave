//! Phase 5.3 — Impostor atlas bake pipeline (Issue #8).
//!
//! The vegetation LOD3 system (see [`crate::vegetation_lod`]) draws each
//! distant tree instance as a camera-facing quad. Before this module existed
//! the quad had no texture binding and fell through to whatever the editor's
//! default material was — producing the "gray monolith" look from the
//! scatter-clip diagnosis.
//!
//! This module supplies the missing piece: a self-contained GPU renderer that
//! rasterises a source mesh into a specific rectangular region of a shared
//! RGBA8 atlas texture, once per (species, angle) pair. The atlas can then be
//! serialised to disk (PNG / KTX2) for ship, or kept in GPU memory for a
//! startup lazy-bake path.
//!
//! # Scope of this file (Phase 5.3 landing 1 of N)
//!
//! * [`fit_ortho_camera`] — pure-math camera fit (T5 of the Phase 5.3 plan).
//! * [`ImpostorBaker`] — offscreen render-target owner with a minimal
//!   unlit-with-alpha pipeline (T1 core).
//! * Unit + integration tests for both.
//!
//! Deferred (follow-up sessions):
//!
//! * T2 `aw_asset_cli impostor-bake` subcommand.
//! * T3 atlas loader (`load_impostor_atlas`) that round-trips the PNG + sidecar.
//! * T4 LOD3 sampling shader changes in the live draw path.
//! * T6 editor lazy-bake-on-missing fallback.
//! * T7 `engine_adapter.rs` wiring to switch LOD3 pipeline.
//!
//! # Design notes
//!
//! * **Self-contained.** The baker owns its atlas texture, depth buffer,
//!   pipeline, BGL, and uniform/sampler state. It does not require a running
//!   [`crate::Renderer`]; any caller that holds a `wgpu::Device` + `Queue` can
//!   drive it. This matches the pattern used by
//!   [`crate::terrain_material_manager::TerrainMaterialManager`].
//! * **Format.** Atlas is [`wgpu::TextureFormat::Rgba8UnormSrgb`] — matches the
//!   sampling convention in `vegetation_lod.rs` and keeps bake deterministic
//!   across backends (no HDR differences).
//! * **Pipeline.** Trivial unlit-with-alpha: vertex stage multiplies by
//!   `view_proj`, fragment samples the diffuse texture and emits `rgba`
//!   unmodified. Silhouette fidelity comes from the mesh's own alpha channel
//!   (for leaf cards etc.) plus depth-test; no lighting applied.
//! * **Viewport.** The bake function sets the viewport to the exact pixel
//!   rectangle of the target [`AtlasRegion`] — this keeps the atlas layout
//!   identical to whatever [`crate::vegetation_lod::ImpostorAtlasSpec::uniform`]
//!   computed for sampling.

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::path::Path;
use wgpu::util::DeviceExt;

use crate::vegetation_lod::{AtlasRegion, AtlasSpeciesEntry, ImpostorAtlasSpec};

/// Axis-aligned bounding box in local mesh space.
///
/// Used by [`fit_ortho_camera`] to choose the orthographic projection volume
/// and eye distance.
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// Build an AABB from a list of vertex positions. Returns `None` when the
    /// slice is empty — baker callers should guard against zero-vertex meshes.
    pub fn from_points(points: &[Vec3]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }
        let mut min = points[0];
        let mut max = points[0];
        for &p in &points[1..] {
            min = min.min(p);
            max = max.max(p);
        }
        Some(Self { min, max })
    }

    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    #[inline]
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

/// Fit an orthographic camera that frames `aabb` from the given Y-orbit angle.
///
/// Conventions (match [`crate::vegetation_lod::generate_impostor_card`]):
///
/// * `angle_rad = 0.0` places the camera on `+Z`, looking at the AABB centre.
/// * Positive angles rotate the camera **clockwise** around `+Y` when viewed
///   from above (i.e. `angle = π/2` puts the camera on `+X`).
/// * Up vector is always world `+Y`.
///
/// The returned projection uses an orthographic frustum sized to the AABB's
/// widest horizontal extent (`max(size.x, size.z)`) for width and `size.y` for
/// height, with a small padding factor to avoid clipping silhouettes that sit
/// exactly on the extreme bounds. Near/far are chosen to comfortably contain
/// the AABB along the camera's forward axis.
pub fn fit_ortho_camera(aabb: Aabb, angle_rad: f32) -> (Mat4, Mat4) {
    const PADDING: f32 = 1.05;

    let size = aabb.size();
    // At orbit angle `a` the world-to-view horizontal projection is
    // `x*cos(a) - z*sin(a)`. Across the AABB corners the maximum magnitude of
    // this expression is `0.5 * (|sx*cos(a)| + |sz*sin(a)|)`, which itself is
    // bounded above by `0.5 * sqrt(sx² + sz²)` (the `a·cos + b·sin` identity).
    // Using that conservative bound for both width and depth keeps every
    // AABB corner inside the ortho frustum at every angle without needing
    // angle-dependent resizing.
    let horiz_diag = (size.x * size.x + size.z * size.z).sqrt();
    let half_w = 0.5 * horiz_diag * PADDING;
    let half_h = 0.5 * size.y * PADDING;

    // Place the camera far enough back that the near plane clears the AABB.
    // Depth half-extent along the view axis follows the same bound as
    // `half_w` above (same math, orthogonal axis).
    let depth_half = horiz_diag.max(size.y) * 0.5 * PADDING;
    // Eye distance: clear the near plane with margin equal to depth_half so
    // that the full AABB lies between near = eye_distance - depth_half and
    // far = eye_distance + depth_half.
    let eye_distance = depth_half * 3.0;

    let center = aabb.center();
    let forward_in_world = Vec3::new(angle_rad.sin(), 0.0, angle_rad.cos());
    let eye = center + forward_in_world * eye_distance;

    let view = Mat4::look_at_rh(eye, center, Vec3::Y);
    let proj = Mat4::orthographic_rh(
        -half_w,
        half_w,
        -half_h,
        half_h,
        eye_distance - depth_half,
        eye_distance + depth_half,
    );
    (proj, view)
}

// ────────────────────────────────────────────────────────────────────────────
// GPU baker
// ────────────────────────────────────────────────────────────────────────────

/// CPU-mirror of the vertex layout consumed by the bake shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ImpostorVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl ImpostorVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ImpostorVertex>() as wgpu::BufferAddress,
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
}

/// View-projection uniform block (64 B).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ViewProjUbo {
    view_proj: [[f32; 4]; 4],
}

/// Configuration for a new [`ImpostorBaker`].
#[derive(Debug, Clone, Copy)]
pub struct ImpostorBakerConfig {
    pub atlas_width: u32,
    pub atlas_height: u32,
    /// Multisample count for the offscreen pass. `1` (no MSAA) is the safe
    /// default for `Limits::downlevel_defaults()` — software adapters on CI
    /// typically do not support 4× MSAA with colour + depth.
    pub sample_count: u32,
}

impl Default for ImpostorBakerConfig {
    fn default() -> Self {
        Self {
            atlas_width: 1024,
            atlas_height: 1024,
            sample_count: 1,
        }
    }
}

/// Minimal unlit-with-alpha GPU renderer that rasterises vegetation meshes
/// into an offscreen atlas texture.
pub struct ImpostorBaker {
    config: ImpostorBakerConfig,
    atlas: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    /// Bind group layout for (diffuse_tex, sampler). The view-proj UBO sits in
    /// group 0; diffuse in group 1.
    #[allow(dead_code)]
    vp_bgl: wgpu::BindGroupLayout,
    diffuse_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vp_buffer: wgpu::Buffer,
    vp_bind_group: wgpu::BindGroup,
}

impl ImpostorBaker {
    /// Atlas texture format — RGBA8 sRGB, deterministic across backends.
    pub const ATLAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    /// Depth format for the offscreen pass.
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    /// Construct a new baker, allocating an offscreen atlas + depth buffer.
    pub fn new(device: &wgpu::Device, config: ImpostorBakerConfig) -> Result<Self> {
        if config.atlas_width == 0 || config.atlas_height == 0 {
            anyhow::bail!("atlas dimensions must be non-zero");
        }
        if config.sample_count != 1 {
            // Keep the first landing simple; MSAA path can be added later.
            anyhow::bail!(
                "Phase 5.3 T1 only supports sample_count=1; got {}",
                config.sample_count
            );
        }

        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("impostor-atlas"),
            size: wgpu::Extent3d {
                width: config.atlas_width,
                height: config.atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::ATLAS_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("impostor-atlas-depth"),
            size: wgpu::Extent3d {
                width: config.atlas_width,
                height: config.atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("impostor-bake-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let vp_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("impostor-bake-vp-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<ViewProjUbo>() as u64
                    ),
                },
                count: None,
            }],
        });

        let diffuse_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("impostor-bake-diffuse-bgl"),
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

        let vp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("impostor-bake-vp-ubo"),
            size: std::mem::size_of::<ViewProjUbo>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vp_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("impostor-bake-vp-bg"),
            layout: &vp_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vp_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("impostor-bake-shader"),
            source: wgpu::ShaderSource::Wgsl(IMPOSTOR_BAKE_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("impostor-bake-pipeline-layout"),
            bind_group_layouts: &[&vp_bgl, &diffuse_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("impostor-bake-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[ImpostorVertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: Self::ATLAS_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // Keep both faces: vegetation meshes are often modeled with
                // single-sided leaf cards that rely on no culling.
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Self::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            config,
            atlas,
            atlas_view,
            depth_view,
            pipeline,
            vp_bgl,
            diffuse_bgl,
            sampler,
            vp_buffer,
            vp_bind_group,
        })
    }

    /// Access the underlying atlas texture (for sampling in the live draw path
    /// once T4 / T7 of Phase 5.3 land).
    pub fn atlas(&self) -> &wgpu::Texture {
        &self.atlas
    }

    /// Atlas dimensions in pixels.
    pub fn atlas_dimensions(&self) -> (u32, u32) {
        (self.config.atlas_width, self.config.atlas_height)
    }

    /// Bind-group layout used by the baker's diffuse binding. Exposed so
    /// downstream tooling can build compatible bind groups for source textures
    /// that live outside this module.
    pub fn diffuse_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.diffuse_bgl
    }

    /// Create a diffuse-texture bind group compatible with the baker pipeline.
    pub fn make_diffuse_bind_group(
        &self,
        device: &wgpu::Device,
        diffuse_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("impostor-bake-diffuse-bg"),
            layout: &self.diffuse_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(diffuse_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Clear the atlas to fully transparent black in one pass. Useful before a
    /// bulk rebake so that unoccupied cells do not retain stale pixels.
    pub fn clear(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("impostor-bake-clear"),
        });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("impostor-bake-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.atlas_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        queue.submit(Some(encoder.finish()));
    }

    /// Bake a single (view_proj, mesh, diffuse) triple into the atlas cell at
    /// `region`.
    ///
    /// * `vertex_buffer` / `index_buffer` must be laid out as
    ///   [`ImpostorVertex`] and `u32` respectively.
    /// * `diffuse_bg` must have been built from
    ///   [`Self::make_diffuse_bind_group`].
    /// * `region` is in normalised UV space `[0, 1]²`; the viewport rectangle
    ///   is derived by multiplying against atlas pixel dimensions.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_into_region(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        region: AtlasRegion,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
        index_count: u32,
        diffuse_bg: &wgpu::BindGroup,
    ) {
        let vp = ViewProjUbo {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.vp_buffer, 0, bytemuck::bytes_of(&vp));

        let (aw, ah) = (self.config.atlas_width as f32, self.config.atlas_height as f32);
        let x = (region.u_min * aw).round();
        let y = (region.v_min * ah).round();
        let w = ((region.u_max - region.u_min) * aw).round().max(1.0);
        let h = ((region.v_max - region.v_min) * ah).round().max(1.0);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("impostor-bake-draw"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("impostor-bake-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.atlas_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Load rather than clear: repeated calls accumulate
                        // into distinct atlas cells.
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_viewport(x, y, w, h, 0.0, 1.0);
            pass.set_scissor_rect(x as u32, y as u32, w as u32, h as u32);
            pass.set_bind_group(0, &self.vp_bind_group, &[]);
            pass.set_bind_group(1, diffuse_bg, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..index_count, 0, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }

    /// Synchronously copy the atlas texture into a CPU-visible buffer and
    /// return its RGBA8 pixels in row-major order (top-left origin).
    ///
    /// This is intended for offline bake / tests. It stalls the GPU, so do not
    /// call on a per-frame hot path.
    pub fn readback_atlas(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Vec<u8>> {
        let (w, h) = (self.config.atlas_width, self.config.atlas_height);
        // wgpu requires bytes-per-row to be a multiple of 256.
        let bytes_per_pixel: u32 = 4;
        let unpadded_bpr = w * bytes_per_pixel;
        let align = 256u32;
        let padded_bpr = ((unpadded_bpr + align - 1) / align) * align;
        let total = padded_bpr as u64 * h as u64;

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("impostor-atlas-readback"),
            size: total,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("impostor-atlas-readback-encoder"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.atlas,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bpr),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        device.poll(wgpu::PollType::Wait).ok();
        rx.recv()
            .context("readback channel dropped")?
            .context("failed to map atlas staging buffer")?;

        let data = slice.get_mapped_range();
        let mut out = Vec::with_capacity((unpadded_bpr * h) as usize);
        for row in 0..h {
            let src_offset = (row * padded_bpr) as usize;
            let src_end = src_offset + unpadded_bpr as usize;
            out.extend_from_slice(&data[src_offset..src_end]);
        }
        drop(data);
        staging.unmap();
        Ok(out)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// CPU-side mesh upload helpers
// ────────────────────────────────────────────────────────────────────────────

/// Upload a [`crate::lod_generator::SimplificationMesh`] into a vertex+index
/// buffer pair suitable for [`ImpostorBaker::draw_into_region`].
///
/// Returns the buffers and the index count.
pub fn upload_simplification_mesh(
    device: &wgpu::Device,
    mesh: &crate::lod_generator::SimplificationMesh,
) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let vertices: Vec<ImpostorVertex> = mesh
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| ImpostorVertex {
            position: [p.x, p.y, p.z],
            uv: mesh.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
        })
        .collect();

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("impostor-mesh-vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("impostor-mesh-indices"),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    (vertex_buffer, index_buffer, mesh.indices.len() as u32)
}

/// Minimal WGSL: vertex transforms pos via view_proj, fragment samples diffuse.
///
/// Lighting is intentionally omitted — impostor cards are baked unlit and the
/// live LOD3 sampling shader (Phase 5.3 T4) will apply ambient / sun tinting
/// at draw time.
const IMPOSTOR_BAKE_WGSL: &str = r#"
struct ViewProj {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> vp: ViewProj;

@group(1) @binding(0) var diffuse_tex: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
) -> VsOut {
    var out: VsOut;
    out.clip_pos = vp.view_proj * vec4<f32>(position, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(diffuse_tex, diffuse_sampler, in.uv);
    return c;
}
"#;

// ────────────────────────────────────────────────────────────────────────────
// Atlas sidecar + PNG I/O (Phase 5.3 T3)
// ────────────────────────────────────────────────────────────────────────────

/// Serde mirror of [`AtlasRegion`]. Lives in its own struct so we can derive
/// [`serde::Serialize`] / [`serde::Deserialize`] without forcing those traits
/// on the GPU-facing type in `vegetation_lod`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AtlasRegionSerde {
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

impl From<AtlasRegion> for AtlasRegionSerde {
    fn from(r: AtlasRegion) -> Self {
        Self {
            u_min: r.u_min,
            v_min: r.v_min,
            u_max: r.u_max,
            v_max: r.v_max,
        }
    }
}

impl From<AtlasRegionSerde> for AtlasRegion {
    fn from(r: AtlasRegionSerde) -> Self {
        Self {
            u_min: r.u_min,
            v_min: r.v_min,
            u_max: r.u_max,
            v_max: r.v_max,
        }
    }
}

/// Serde mirror of [`AtlasSpeciesEntry`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasSpeciesEntrySerde {
    pub name: String,
    pub angles: Vec<AtlasRegionSerde>,
}

/// Full on-disk layout for an impostor atlas.
///
/// This is the structure serialised to `*.atlas.toml` beside the baked
/// `*.atlas.png`. Ship paths load the PNG into a [`wgpu::Texture`] and the
/// sidecar into an [`ImpostorAtlasSpec`] used by LOD3 sampling.
///
/// # Format (TOML)
///
/// ```toml
/// atlas_width  = 1024
/// atlas_height = 512
/// angle_count  = 8
///
/// [[species]]
/// name = "oak"
/// angles = [
///   { u_min = 0.0, v_min = 0.0, u_max = 0.125, v_max = 0.5 },
///   # … 7 more
/// ]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImpostorAtlasSidecar {
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub angle_count: u32,
    pub species: Vec<AtlasSpeciesEntrySerde>,
}

impl From<&ImpostorAtlasSpec> for ImpostorAtlasSidecar {
    fn from(s: &ImpostorAtlasSpec) -> Self {
        Self {
            atlas_width: s.atlas_width,
            atlas_height: s.atlas_height,
            angle_count: s.angle_count,
            species: s
                .species
                .iter()
                .map(|e| AtlasSpeciesEntrySerde {
                    name: e.name.clone(),
                    angles: e.angles.iter().copied().map(Into::into).collect(),
                })
                .collect(),
        }
    }
}

impl From<ImpostorAtlasSidecar> for ImpostorAtlasSpec {
    fn from(s: ImpostorAtlasSidecar) -> Self {
        Self {
            atlas_width: s.atlas_width,
            atlas_height: s.atlas_height,
            angle_count: s.angle_count,
            species: s
                .species
                .into_iter()
                .map(|e| AtlasSpeciesEntry {
                    name: e.name,
                    angles: e.angles.into_iter().map(Into::into).collect(),
                })
                .collect(),
        }
    }
}

/// Serialise `spec` to a TOML sidecar at `path`. Parent directory must exist.
pub fn save_atlas_sidecar(path: &Path, spec: &ImpostorAtlasSpec) -> Result<()> {
    let sidecar = ImpostorAtlasSidecar::from(spec);
    let text = toml::to_string_pretty(&sidecar)
        .context("serialise ImpostorAtlasSidecar to TOML")?;
    std::fs::write(path, text)
        .with_context(|| format!("write atlas sidecar to {}", path.display()))?;
    Ok(())
}

/// Load a TOML sidecar from `path` into an [`ImpostorAtlasSpec`].
pub fn load_atlas_sidecar(path: &Path) -> Result<ImpostorAtlasSpec> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read atlas sidecar from {}", path.display()))?;
    let sidecar: ImpostorAtlasSidecar = toml::from_str(&text)
        .with_context(|| format!("parse atlas sidecar {}", path.display()))?;
    Ok(sidecar.into())
}

/// Save an RGBA8 pixel buffer (row-major, top-left origin) as a PNG at `path`.
///
/// `pixels.len()` must equal `width * height * 4`.
#[cfg(feature = "textures")]
pub fn save_atlas_png(path: &Path, pixels: &[u8], width: u32, height: u32) -> Result<()> {
    let expected = (width as usize) * (height as usize) * 4;
    if pixels.len() != expected {
        anyhow::bail!(
            "save_atlas_png: expected {} bytes for {}x{} RGBA8, got {}",
            expected,
            width,
            height,
            pixels.len()
        );
    }
    let img: image::RgbaImage =
        image::ImageBuffer::from_raw(width, height, pixels.to_vec()).ok_or_else(|| {
            anyhow::anyhow!(
                "save_atlas_png: failed to build RgbaImage from {}x{} buffer",
                width,
                height
            )
        })?;
    img.save(path)
        .with_context(|| format!("write atlas PNG to {}", path.display()))?;
    Ok(())
}

/// Load a PNG at `path` and return its RGBA8 pixels + dimensions.
///
/// Non-RGBA8 PNGs are converted to RGBA8 by the `image` crate.
#[cfg(feature = "textures")]
pub fn load_atlas_png(path: &Path) -> Result<(Vec<u8>, u32, u32)> {
    let img = image::open(path)
        .with_context(|| format!("read atlas PNG from {}", path.display()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    Ok((img.into_raw(), w, h))
}

// ────────────────────────────────────────────────────────────────────────────
// Lazy-bake orchestration (Phase 5.3 T6)
// ────────────────────────────────────────────────────────────────────────────

/// Outcome of a [`load_or_bake_atlas`] call.
///
/// Useful for log/telemetry — callers can distinguish a cheap ship-path load
/// from a full GPU bake that populated the disk cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasSource {
    /// Both PNG + sidecar existed and matched `expected` spec.
    LoadedFromDisk,
    /// Sidecar was missing, mismatched, or corrupt — the atlas was (re)baked
    /// and written to disk.
    Baked,
}

/// The payload returned by [`load_or_bake_atlas`]: RGBA8 pixel data plus the
/// live [`ImpostorAtlasSpec`] (either round-tripped from the sidecar or the
/// one the caller passed in for baking).
#[derive(Debug)]
pub struct LoadedAtlas {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub spec: ImpostorAtlasSpec,
    pub source: AtlasSource,
}

/// Returns `true` when `sidecar` is structurally equivalent to `expected` —
/// identical dimensions, angle count, and species ordering/names.
///
/// UV regions are NOT compared: the loader is the source of truth for those,
/// because the sidecar was written by a previous bake whose [`ImpostorAtlasSpec`]
/// produced them. A caller asking to re-use a disk atlas must only agree on
/// the schema, not the exact UV rectangles.
pub fn sidecar_matches_schema(sidecar: &ImpostorAtlasSpec, expected: &ImpostorAtlasSpec) -> bool {
    if sidecar.atlas_width != expected.atlas_width
        || sidecar.atlas_height != expected.atlas_height
        || sidecar.angle_count != expected.angle_count
        || sidecar.species.len() != expected.species.len()
    {
        return false;
    }
    sidecar
        .species
        .iter()
        .zip(expected.species.iter())
        .all(|(a, b)| a.name == b.name && a.angles.len() == b.angles.len())
}

/// Load an impostor atlas from disk if it exists and matches `expected`,
/// otherwise invoke `bake_fn` to produce fresh pixels and write both files
/// to disk.
///
/// This is the engine-side glue for Phase 5.3 — callers (editor, standalone
/// game) ship with or without `*.atlas.png` / `*.atlas.toml` beside their
/// vegetation assets, and the startup path silently produces them on first
/// run when missing. Shipped builds take the fast `LoadedFromDisk` branch.
///
/// # Arguments
///
/// * `png_path` — where the baked atlas PNG lives on disk.
/// * `sidecar_path` — where the TOML sidecar lives on disk.
/// * `expected` — the spec the caller is expecting (matches species count,
///   atlas dimensions, etc). Used both as a cache-validation key and as the
///   input to `bake_fn` on a miss.
/// * `bake_fn` — callback that performs the actual GPU bake. Receives the
///   `expected` spec and must return `(pixels, width, height)` in RGBA8.
///
/// # Cache-miss conditions
///
/// A fresh bake is performed when ANY of the following are true:
///
/// * PNG or sidecar file does not exist.
/// * Sidecar fails to parse as TOML.
/// * Sidecar schema differs from `expected` (see [`sidecar_matches_schema`]).
/// * PNG dimensions differ from the sidecar.
///
/// On a cache miss the function writes both files before returning — the
/// next run will hit `LoadedFromDisk`.
#[cfg(feature = "textures")]
pub fn load_or_bake_atlas<F>(
    png_path: &Path,
    sidecar_path: &Path,
    expected: &ImpostorAtlasSpec,
    bake_fn: F,
) -> Result<LoadedAtlas>
where
    F: FnOnce(&ImpostorAtlasSpec) -> Result<(Vec<u8>, u32, u32)>,
{
    // Fast path: both files present, parseable, matching.
    if png_path.exists() && sidecar_path.exists() {
        if let Ok(sidecar) = load_atlas_sidecar(sidecar_path) {
            if sidecar_matches_schema(&sidecar, expected) {
                if let Ok((pixels, w, h)) = load_atlas_png(png_path) {
                    if w == sidecar.atlas_width && h == sidecar.atlas_height {
                        return Ok(LoadedAtlas {
                            pixels,
                            width: w,
                            height: h,
                            spec: sidecar,
                            source: AtlasSource::LoadedFromDisk,
                        });
                    }
                }
            }
        }
    }

    // Slow path: bake, persist, return.
    let (pixels, w, h) = bake_fn(expected)
        .context("lazy-bake callback failed to produce atlas pixels")?;
    let expected_len = (w as usize) * (h as usize) * 4;
    if pixels.len() != expected_len {
        anyhow::bail!(
            "load_or_bake_atlas: bake_fn returned {} bytes but {}x{} RGBA8 needs {}",
            pixels.len(),
            w,
            h,
            expected_len
        );
    }

    if let Some(parent) = png_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create atlas dir {}", parent.display()))?;
        }
    }
    if let Some(parent) = sidecar_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create sidecar dir {}", parent.display()))?;
        }
    }
    save_atlas_png(png_path, &pixels, w, h)?;
    save_atlas_sidecar(sidecar_path, expected)?;

    Ok(LoadedAtlas {
        pixels,
        width: w,
        height: h,
        spec: expected.clone(),
        source: AtlasSource::Baked,
    })
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn aabb_from_points_tracks_min_max() {
        let pts = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-1.0, 5.0, 0.5),
            Vec3::new(2.5, -1.0, 4.0),
        ];
        let aabb = Aabb::from_points(&pts).unwrap();
        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, 0.5));
        assert_eq!(aabb.max, Vec3::new(2.5, 5.0, 4.0));
        assert_eq!(aabb.center(), Vec3::new(0.75, 2.0, 2.25));
    }

    #[test]
    fn aabb_from_empty_slice_returns_none() {
        assert!(Aabb::from_points(&[]).is_none());
    }

    #[test]
    fn fit_ortho_camera_angle_zero_places_camera_on_plus_z() {
        // Unit cube centered at origin.
        let aabb = Aabb {
            min: Vec3::splat(-0.5),
            max: Vec3::splat(0.5),
        };
        let (_proj, view) = fit_ortho_camera(aabb, 0.0);
        // The view matrix's inverse gives the camera's world transform.
        let cam_world = view.inverse();
        let eye = cam_world.transform_point3(Vec3::ZERO);
        // Eye should be on the +Z axis at a positive distance.
        assert!(
            eye.z > 0.5,
            "camera should be on +Z at angle=0, got {:?}",
            eye
        );
        assert_relative_eq!(eye.x, 0.0, epsilon = 1.0e-5);
        assert_relative_eq!(eye.y, 0.0, epsilon = 1.0e-5);
    }

    #[test]
    fn fit_ortho_camera_opposite_angles_produce_mirrored_views() {
        let aabb = Aabb {
            min: Vec3::new(-1.0, 0.0, -1.0),
            max: Vec3::new(1.0, 3.0, 1.0),
        };
        let (_p0, v0) = fit_ortho_camera(aabb, 0.0);
        let (_p1, v1) = fit_ortho_camera(aabb, std::f32::consts::PI);

        let eye0 = v0.inverse().transform_point3(Vec3::ZERO);
        let eye1 = v1.inverse().transform_point3(Vec3::ZERO);
        let center = aabb.center();
        // Eyes should be reflections of each other through the AABB centre on
        // the horizontal plane. Y should match (both sit on the centre plane).
        assert_relative_eq!(eye0.y, eye1.y, epsilon = 1.0e-4);
        assert_relative_eq!(eye0.x + eye1.x, 2.0 * center.x, epsilon = 1.0e-4);
        assert_relative_eq!(eye0.z + eye1.z, 2.0 * center.z, epsilon = 1.0e-4);
    }

    #[test]
    fn fit_ortho_camera_quarter_turn_places_camera_on_plus_x() {
        let aabb = Aabb {
            min: Vec3::splat(-1.0),
            max: Vec3::splat(1.0),
        };
        let (_proj, view) = fit_ortho_camera(aabb, std::f32::consts::FRAC_PI_2);
        let eye = view.inverse().transform_point3(Vec3::ZERO);
        assert!(eye.x > 0.5, "expected +X, got {:?}", eye);
        assert_relative_eq!(eye.z, 0.0, epsilon = 1.0e-4);
    }

    #[test]
    fn fit_ortho_camera_projection_contains_aabb_corners() {
        // Any AABB corner should project within NDC x/y ∈ [-1, 1] (with some
        // slack for the 5% padding).
        let aabb = Aabb {
            min: Vec3::new(-2.0, 0.0, -1.5),
            max: Vec3::new(1.5, 4.0, 2.0),
        };
        let corners = [
            Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z),
            Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z),
            Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z),
            Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z),
            Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z),
            Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z),
            Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z),
            Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z),
        ];
        for &angle in &[0.0_f32, 0.5, 1.2, std::f32::consts::PI, 4.2] {
            let (proj, view) = fit_ortho_camera(aabb, angle);
            let vp = proj * view;
            for c in &corners {
                let clip = vp * c.extend(1.0);
                let ndc = clip.truncate() / clip.w;
                assert!(
                    ndc.x.abs() <= 1.05 && ndc.y.abs() <= 1.05,
                    "corner {:?} at angle {} projected outside NDC: ndc={:?}",
                    c,
                    angle,
                    ndc
                );
            }
        }
    }

    #[test]
    fn impostor_vertex_layout_stride_matches_struct() {
        assert_eq!(
            ImpostorVertex::LAYOUT.array_stride as usize,
            std::mem::size_of::<ImpostorVertex>()
        );
        assert_eq!(std::mem::size_of::<ImpostorVertex>(), 20);
    }

    #[test]
    fn bake_shader_source_parses_with_naga() {
        // Syntax-level validation. Full uniform-layout validation only happens
        // at GPU pipeline creation time (see the gpu-tests integration test).
        let module = naga::front::wgsl::parse_str(IMPOSTOR_BAKE_WGSL)
            .expect("bake shader must parse");
        assert!(module
            .entry_points
            .iter()
            .any(|ep| ep.name == "vs_main" && ep.stage == naga::ShaderStage::Vertex));
        assert!(module
            .entry_points
            .iter()
            .any(|ep| ep.name == "fs_main" && ep.stage == naga::ShaderStage::Fragment));
    }

    // ── T3: sidecar + PNG I/O ───────────────────────────────────────────────

    fn sample_spec() -> ImpostorAtlasSpec {
        ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak", "pine", "birch"])
    }

    #[test]
    fn atlas_region_serde_roundtrip_preserves_uv() {
        let r = AtlasRegion {
            u_min: 0.125,
            v_min: 0.25,
            u_max: 0.375,
            v_max: 0.5,
        };
        let s: AtlasRegionSerde = r.into();
        let back: AtlasRegion = s.into();
        assert_eq!(back.u_min, r.u_min);
        assert_eq!(back.v_min, r.v_min);
        assert_eq!(back.u_max, r.u_max);
        assert_eq!(back.v_max, r.v_max);
    }

    #[test]
    fn sidecar_roundtrip_is_identity_for_uniform_spec() {
        let original = sample_spec();
        let sidecar = ImpostorAtlasSidecar::from(&original);
        let restored: ImpostorAtlasSpec = sidecar.into();

        assert_eq!(restored.atlas_width, original.atlas_width);
        assert_eq!(restored.atlas_height, original.atlas_height);
        assert_eq!(restored.angle_count, original.angle_count);
        assert_eq!(restored.species.len(), original.species.len());
        for (a, b) in restored.species.iter().zip(original.species.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.angles.len(), b.angles.len());
            for (ra, rb) in a.angles.iter().zip(b.angles.iter()) {
                assert_relative_eq!(ra.u_min, rb.u_min, epsilon = 1.0e-6);
                assert_relative_eq!(ra.v_min, rb.v_min, epsilon = 1.0e-6);
                assert_relative_eq!(ra.u_max, rb.u_max, epsilon = 1.0e-6);
                assert_relative_eq!(ra.v_max, rb.v_max, epsilon = 1.0e-6);
            }
        }
    }

    #[test]
    fn save_and_load_sidecar_roundtrips_through_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.atlas.toml");

        let original = sample_spec();
        save_atlas_sidecar(&path, &original).expect("save sidecar");
        let restored = load_atlas_sidecar(&path).expect("load sidecar");

        assert_eq!(restored.atlas_width, original.atlas_width);
        assert_eq!(restored.atlas_height, original.atlas_height);
        assert_eq!(restored.angle_count, original.angle_count);
        assert_eq!(restored.species.len(), 3);
        assert_eq!(restored.species[0].name, "oak");
        assert_eq!(restored.species[1].name, "pine");
        assert_eq!(restored.species[2].name, "birch");
        // Total cells per species = angle_count.
        assert_eq!(restored.species[0].angles.len(), 8);
    }

    #[test]
    fn sidecar_roundtrip_preserves_lookup_semantics() {
        // After sidecar round-trip, lookup() at arbitrary angles must return
        // identical UV regions — this is what keeps LOD3 sampling consistent
        // between live-bake and shipped-atlas paths.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("lookup.atlas.toml");
        let original = sample_spec();
        save_atlas_sidecar(&path, &original).unwrap();
        let restored = load_atlas_sidecar(&path).unwrap();

        for species_idx in 0..original.species.len() {
            for k in 0..16 {
                let angle = k as f32 * 0.3;
                let a = original.lookup(species_idx, angle).copied().unwrap();
                let b = restored.lookup(species_idx, angle).copied().unwrap();
                assert_relative_eq!(a.u_min, b.u_min, epsilon = 1.0e-6);
                assert_relative_eq!(a.v_min, b.v_min, epsilon = 1.0e-6);
                assert_relative_eq!(a.u_max, b.u_max, epsilon = 1.0e-6);
                assert_relative_eq!(a.v_max, b.v_max, epsilon = 1.0e-6);
            }
        }
    }

    #[test]
    fn load_sidecar_rejects_invalid_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("broken.atlas.toml");
        std::fs::write(&path, "not a valid = [sidecar").unwrap();
        assert!(load_atlas_sidecar(&path).is_err());
    }

    #[test]
    fn load_sidecar_missing_file_errors_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does_not_exist.atlas.toml");
        let err = load_atlas_sidecar(&path).unwrap_err();
        assert!(
            err.to_string().contains("read atlas sidecar"),
            "expected context about reading sidecar, got: {}",
            err
        );
    }

    #[cfg(feature = "textures")]
    #[test]
    fn save_atlas_png_rejects_wrong_buffer_size() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.png");
        let pixels = vec![0u8; 10]; // wrong size for 4x4 RGBA8
        let err = save_atlas_png(&path, &pixels, 4, 4).unwrap_err();
        assert!(err.to_string().contains("expected 64 bytes"));
    }

    #[cfg(feature = "textures")]
    #[test]
    fn atlas_png_roundtrips_pixels_exactly() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("roundtrip.png");

        // 4x4 checkerboard of magenta/green.
        let mut pixels = Vec::with_capacity(4 * 4 * 4);
        for y in 0..4 {
            for x in 0..4 {
                if (x + y) % 2 == 0 {
                    pixels.extend_from_slice(&[255, 0, 255, 255]);
                } else {
                    pixels.extend_from_slice(&[0, 255, 0, 255]);
                }
            }
        }

        save_atlas_png(&path, &pixels, 4, 4).expect("save png");
        let (loaded, w, h) = load_atlas_png(&path).expect("load png");
        assert_eq!(w, 4);
        assert_eq!(h, 4);
        assert_eq!(loaded, pixels, "PNG round-trip must be lossless");
    }

    // ── T6: lazy-bake fallback ──────────────────────────────────────────────

    fn fake_bake_pixels(spec: &ImpostorAtlasSpec) -> (Vec<u8>, u32, u32) {
        let (w, h) = (spec.atlas_width, spec.atlas_height);
        // Fill with a deterministic pattern so we can detect identity across
        // save→load cycles.
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                pixels.extend_from_slice(&[
                    (x & 0xFF) as u8,
                    (y & 0xFF) as u8,
                    ((x ^ y) & 0xFF) as u8,
                    255,
                ]);
            }
        }
        (pixels, w, h)
    }

    #[test]
    fn sidecar_schema_match_is_strict_on_dimensions() {
        let a = ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak", "pine"]);
        let b = ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak", "pine"]);
        assert!(sidecar_matches_schema(&a, &b));

        let c = ImpostorAtlasSpec::uniform(2048, 512, 8, &["oak", "pine"]); // wider
        assert!(!sidecar_matches_schema(&a, &c));

        let d = ImpostorAtlasSpec::uniform(1024, 512, 16, &["oak", "pine"]); // more angles
        assert!(!sidecar_matches_schema(&a, &d));
    }

    #[test]
    fn sidecar_schema_match_checks_species_names_in_order() {
        let a = ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak", "pine"]);
        let b = ImpostorAtlasSpec::uniform(1024, 512, 8, &["pine", "oak"]); // swapped
        assert!(!sidecar_matches_schema(&a, &b));

        let c = ImpostorAtlasSpec::uniform(1024, 512, 8, &["oak"]); // fewer
        assert!(!sidecar_matches_schema(&a, &c));
    }

    #[cfg(feature = "textures")]
    #[test]
    fn load_or_bake_produces_atlas_on_cold_start() {
        let tmp = tempfile::tempdir().unwrap();
        let png = tmp.path().join("tree.atlas.png");
        let side = tmp.path().join("tree.atlas.toml");
        let spec = ImpostorAtlasSpec::uniform(64, 32, 8, &["oak", "pine"]);

        let mut invoked = 0u32;
        let loaded = load_or_bake_atlas(&png, &side, &spec, |s| {
            invoked += 1;
            Ok(fake_bake_pixels(s))
        })
        .expect("bake should succeed on cold start");

        assert_eq!(invoked, 1, "bake callback invoked exactly once");
        assert_eq!(loaded.source, AtlasSource::Baked);
        assert_eq!(loaded.width, 64);
        assert_eq!(loaded.height, 32);
        assert_eq!(loaded.pixels.len(), 64 * 32 * 4);
        assert!(png.exists(), "PNG must be persisted");
        assert!(side.exists(), "sidecar must be persisted");
    }

    #[cfg(feature = "textures")]
    #[test]
    fn load_or_bake_reuses_disk_atlas_on_warm_start() {
        let tmp = tempfile::tempdir().unwrap();
        let png = tmp.path().join("tree.atlas.png");
        let side = tmp.path().join("tree.atlas.toml");
        let spec = ImpostorAtlasSpec::uniform(64, 32, 8, &["oak", "pine"]);

        // Cold start: populate disk.
        load_or_bake_atlas(&png, &side, &spec, |s| Ok(fake_bake_pixels(s)))
            .unwrap();

        // Warm start: callback must NOT be invoked.
        let mut invoked = 0u32;
        let loaded = load_or_bake_atlas(&png, &side, &spec, |_s| {
            invoked += 1;
            unreachable!("bake callback must not run when disk cache is valid");
        })
        .expect("warm start should succeed");

        assert_eq!(invoked, 0);
        assert_eq!(loaded.source, AtlasSource::LoadedFromDisk);
        assert_eq!(loaded.width, 64);
        assert_eq!(loaded.height, 32);
        // Pixel content must match what the cold bake wrote.
        let (expected, _, _) = fake_bake_pixels(&spec);
        assert_eq!(loaded.pixels, expected);
    }

    #[cfg(feature = "textures")]
    #[test]
    fn load_or_bake_rebakes_when_sidecar_schema_mismatches() {
        let tmp = tempfile::tempdir().unwrap();
        let png = tmp.path().join("tree.atlas.png");
        let side = tmp.path().join("tree.atlas.toml");

        let spec_small = ImpostorAtlasSpec::uniform(64, 32, 8, &["oak"]);
        let spec_big = ImpostorAtlasSpec::uniform(128, 64, 8, &["oak", "pine"]);

        // Write a cache keyed to the small spec.
        load_or_bake_atlas(&png, &side, &spec_small, |s| Ok(fake_bake_pixels(s))).unwrap();

        // Now request a different spec — should force a fresh bake.
        let mut invoked = 0u32;
        let loaded = load_or_bake_atlas(&png, &side, &spec_big, |s| {
            invoked += 1;
            Ok(fake_bake_pixels(s))
        })
        .expect("schema mismatch must trigger a rebake");

        assert_eq!(invoked, 1);
        assert_eq!(loaded.source, AtlasSource::Baked);
        assert_eq!(loaded.width, 128);
        assert_eq!(loaded.height, 64);
        // Cache should now reflect the big spec.
        let persisted = load_atlas_sidecar(&side).unwrap();
        assert!(sidecar_matches_schema(&persisted, &spec_big));
    }

    #[cfg(feature = "textures")]
    #[test]
    fn load_or_bake_rebakes_when_sidecar_missing_but_png_present() {
        let tmp = tempfile::tempdir().unwrap();
        let png = tmp.path().join("tree.atlas.png");
        let side = tmp.path().join("tree.atlas.toml");
        let spec = ImpostorAtlasSpec::uniform(64, 32, 8, &["oak"]);

        // Bake once so PNG exists, then delete sidecar.
        load_or_bake_atlas(&png, &side, &spec, |s| Ok(fake_bake_pixels(s))).unwrap();
        std::fs::remove_file(&side).unwrap();
        assert!(png.exists() && !side.exists());

        let mut invoked = 0u32;
        let loaded = load_or_bake_atlas(&png, &side, &spec, |s| {
            invoked += 1;
            Ok(fake_bake_pixels(s))
        })
        .unwrap();

        assert_eq!(invoked, 1, "missing sidecar must force a rebake");
        assert_eq!(loaded.source, AtlasSource::Baked);
        assert!(side.exists(), "sidecar must be regenerated");
    }

    #[cfg(feature = "textures")]
    #[test]
    fn load_or_bake_rebakes_when_sidecar_is_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        let png = tmp.path().join("tree.atlas.png");
        let side = tmp.path().join("tree.atlas.toml");
        let spec = ImpostorAtlasSpec::uniform(64, 32, 8, &["oak"]);

        // Bake once, then clobber the sidecar with garbage.
        load_or_bake_atlas(&png, &side, &spec, |s| Ok(fake_bake_pixels(s))).unwrap();
        std::fs::write(&side, "garbage = not = valid = toml").unwrap();

        let mut invoked = 0u32;
        let loaded = load_or_bake_atlas(&png, &side, &spec, |s| {
            invoked += 1;
            Ok(fake_bake_pixels(s))
        })
        .unwrap();

        assert_eq!(invoked, 1, "corrupt sidecar must force a rebake");
        assert_eq!(loaded.source, AtlasSource::Baked);
        // Post-bake sidecar should parse cleanly.
        assert!(load_atlas_sidecar(&side).is_ok());
    }

    #[cfg(feature = "textures")]
    #[test]
    fn load_or_bake_rejects_wrong_sized_pixels_from_callback() {
        let tmp = tempfile::tempdir().unwrap();
        let png = tmp.path().join("bad.atlas.png");
        let side = tmp.path().join("bad.atlas.toml");
        let spec = ImpostorAtlasSpec::uniform(64, 32, 8, &["oak"]);

        let err = load_or_bake_atlas(&png, &side, &spec, |_s| {
            // Dimensions say 32x32 but we return 64x32 worth of bytes.
            Ok((vec![0u8; 64 * 32 * 4], 32, 32))
        })
        .unwrap_err();

        assert!(
            err.to_string().contains("bake_fn returned"),
            "error should flag buffer/dim mismatch: {}",
            err
        );
        assert!(!png.exists(), "PNG must not be written when validation fails");
    }

    #[cfg(feature = "textures")]
    #[test]
    fn load_or_bake_propagates_bake_errors_without_writing_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let png = tmp.path().join("fail.atlas.png");
        let side = tmp.path().join("fail.atlas.toml");
        let spec = ImpostorAtlasSpec::uniform(64, 32, 8, &["oak"]);

        let err = load_or_bake_atlas(&png, &side, &spec, |_s| {
            anyhow::bail!("simulated GPU error")
        })
        .unwrap_err();

        assert!(err.to_string().contains("lazy-bake callback failed"));
        assert!(!png.exists());
        assert!(!side.exists());
    }
}
