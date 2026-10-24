//! Phase 5.3 T7 (stage 1) — `ImpostorPass` draw-helper.
//!
//! This is the reusable plumbing that sits between the raw
//! [`crate::impostor_lod3`] pipeline/resources and whatever renderer (or
//! render-graph node, or editor adapter) actually owns the render pass.
//!
//! It bundles the six moving parts that every LOD3 draw needs:
//!
//! 1. the pipeline ([`Lod3Pipeline`]),
//! 2. the atlas-side GPU resources ([`Lod3Resources`]),
//! 3. a camera UBO + its bind group,
//! 4. a shared quad vertex + index buffer,
//! 5. an auto-resizing instance buffer,
//! 6. a recorded-instance count.
//!
//! Callers:
//!
//! * call [`ImpostorPass::new`] once per atlas (usually during scene load),
//! * call [`ImpostorPass::update_camera`] once per frame,
//! * call [`ImpostorPass::upload_instances`] whenever the LOD3 instance set
//!   changes (grows the internal buffer on demand),
//! * call [`ImpostorPass::record`] inside their active `wgpu::RenderPass`
//!   to record the draw.
//!
//! A [`Renderer::install_impostor_pass`][install] hook (stage 2) and the
//! editor `engine_adapter` rewrite (stage 3) build on top of this helper.
//! This stage 1 landing keeps the editor's current PBR-based LOD3 path
//! intact, so scatter visuals continue to render while the full
//! replacement is wired in.
//!
//! [install]: https://github.com/lazyxeon/AstraWeave

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

use crate::impostor_lod3::{build_lod3_pipeline, Lod3InstanceRaw, Lod3Pipeline, Lod3Resources};
use crate::vegetation_lod::ImpostorAtlasSpec;

// ────────────────────────────────────────────────────────────────────────────
// Camera UBO
// ────────────────────────────────────────────────────────────────────────────

/// CPU-side camera UBO layout. Mirrors the WGSL `Camera` struct in
/// [`crate::impostor_lod3::LOD3_SAMPLING_WGSL`].
///
/// * `view_proj` — row-major column-packed 4×4 projection × view matrix.
/// * `camera_pos` — world-space camera position; `w` is unused padding.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
struct CameraUboRaw {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
}

const CAMERA_UBO_BYTES: u64 = std::mem::size_of::<CameraUboRaw>() as u64;

// ────────────────────────────────────────────────────────────────────────────
// Quad geometry
// ────────────────────────────────────────────────────────────────────────────

/// 20-byte quad vertex (xyz + uv) matching [`crate::impostor_lod3::LOD3_QUAD_VERTEX_LAYOUT`].
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

/// Unit quad in local space with bottom-center at origin.
///
/// * x ∈ [-0.5, 0.5] — centered so billboard scaling is symmetric
/// * y ∈ [0.0, 1.0]  — grows upward from the ground plane
/// * UVs place (0,0) at top-left so `V=0` lines up with the top of an atlas
///   cell (row-major RGBA8 atlases have row 0 at the top).
const QUAD_VERTICES: [QuadVertex; 4] = [
    QuadVertex {
        pos: [-0.5, 0.0, 0.0],
        uv: [0.0, 1.0],
    }, // bottom-left
    QuadVertex {
        pos: [0.5, 0.0, 0.0],
        uv: [1.0, 1.0],
    }, // bottom-right
    QuadVertex {
        pos: [0.5, 1.0, 0.0],
        uv: [1.0, 0.0],
    }, // top-right
    QuadVertex {
        pos: [-0.5, 1.0, 0.0],
        uv: [0.0, 0.0],
    }, // top-left
];

/// CCW winding (matches `front_face: Ccw` in [`build_lod3_pipeline`]).
const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

const IMPOSTOR_CURRENT_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
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

const IMPOSTOR_PREV_POSITION: wgpu::VertexAttribute = wgpu::VertexAttribute {
    offset: 0,
    shader_location: 4,
    format: wgpu::VertexFormat::Float32x4,
};

const IMPOSTOR_PREV_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Lod3InstanceRaw>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &[IMPOSTOR_PREV_POSITION],
};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ImpostorVectorMats {
    curr_vp: [[f32; 4]; 4],
    prev_vp: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
}

const IMPOSTOR_VECTOR_WGSL: &str = r#"
struct Mats {
    curr_vp: mat4x4<f32>,
    prev_vp: mat4x4<f32>,
    view: mat4x4<f32>,
}
struct SpeciesRow {
    cell_0_uv: vec4<f32>,
    cell_step: vec4<f32>,
}
@group(0) @binding(0) var<uniform> mats: Mats;
@group(0) @binding(1) var<storage, read> rows: array<SpeciesRow>;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) facing: vec3<f32>,
    @location(2) @interpolate(linear) curr_ndc: vec2<f32>,
    @location(3) @interpolate(linear) prev_ndc: vec2<f32>,
}

fn camera_pos() -> vec3<f32> {
    let t = mats.view[3].xyz;
    return -vec3<f32>(
        dot(mats.view[0].xyz, t),
        dot(mats.view[1].xyz, t),
        dot(mats.view[2].xyz, t),
    );
}

@vertex
fn vs_main(
    @location(0) local_pos: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) position_scale: vec4<f32>,
    @location(3) species_and_params: vec4<f32>,
    @location(4) prev_position_scale: vec4<f32>,
) -> VsOut {
    let instance_pos = position_scale.xyz;
    let prev_pos = prev_position_scale.xyz;
    let scale = position_scale.w;
    let to_cam = camera_pos() - instance_pos;
    let horiz = vec3<f32>(to_cam.x, 0.0, to_cam.z);
    let horiz_len = max(length(horiz), 1.0e-4);
    let forward = horiz / horiz_len;
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), forward));
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let world_offset = right * (local_pos.x * scale) + up * (local_pos.y * scale);
    let world_pos = instance_pos + world_offset;
    let prev_world = prev_pos + world_offset;
    let species_idx = u32(species_and_params.x + 0.5);
    let row = rows[species_idx];
    let tau = 6.28318530718;
    let angle = atan2(forward.x, forward.z);
    let angle_count = row.cell_step.z;
    let step_rad = tau / angle_count;
    let angle_norm = angle - tau * floor(angle / tau);
    let cell_idx_f = floor(angle_norm / step_rad + 0.5);
    let cell_idx = u32(cell_idx_f - angle_count * floor(cell_idx_f / angle_count));
    let cell_u_min = row.cell_0_uv.x + f32(cell_idx) * row.cell_step.x;
    let cell_v_min = row.cell_0_uv.y;
    let uv = vec2<f32>(
        mix(cell_u_min, cell_u_min + row.cell_step.x, local_uv.x),
        mix(cell_v_min, cell_v_min + row.cell_step.y, local_uv.y),
    );
    let curr_clip = mats.curr_vp * vec4<f32>(world_pos, 1.0);
    let prev_clip = mats.prev_vp * vec4<f32>(prev_world, 1.0);
    var out: VsOut;
    out.clip = curr_clip;
    out.uv = uv;
    out.facing = forward;
    out.curr_ndc = curr_clip.xy / curr_clip.w;
    out.prev_ndc = prev_clip.xy / prev_clip.w;
    return out;
}

struct FsOut {
    @location(0) normal: vec4<f32>,
    @location(1) velocity: vec2<f32>,
}

@fragment
fn fs_main(in: VsOut) -> FsOut {
    let c = textureSample(atlas_tex, atlas_sampler, in.uv);
    if (c.a < 0.05) {
        discard;
    }
    let view_n = normalize((mats.view * vec4<f32>(in.facing, 0.0)).xyz);
    let curr_uv = vec2<f32>(in.curr_ndc.x, -in.curr_ndc.y) * 0.5 + vec2<f32>(0.5);
    let prev_uv = vec2<f32>(in.prev_ndc.x, -in.prev_ndc.y) * 0.5 + vec2<f32>(0.5);
    var out: FsOut;
    out.normal = vec4<f32>(view_n * 0.5 + vec3<f32>(0.5), 1.0);
    out.velocity = curr_uv - prev_uv;
    return out;
}
"#;

// ────────────────────────────────────────────────────────────────────────────
// ImpostorPass
// ────────────────────────────────────────────────────────────────────────────

/// End-to-end LOD3 impostor draw helper.
///
/// See module docs for the intended call sequence. This struct is
/// deliberately monolithic — the pieces it owns all live and die together
/// (one atlas ⇒ one pipeline ⇒ one camera UBO ⇒ one instance buffer).
pub struct ImpostorPass {
    pipeline: Lod3Pipeline,
    resources: Lod3Resources,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    quad_vbuf: wgpu::Buffer,
    quad_ibuf: wgpu::Buffer,

    /// Per-instance data buffer. Grown (re-allocated) on demand.
    instance_buffer: wgpu::Buffer,
    /// Current capacity measured in [`Lod3InstanceRaw`] entries.
    instance_capacity: usize,
    /// Number of instances recorded by the most recent
    /// [`ImpostorPass::upload_instances`] call.
    instance_count: u32,
    /// Instances from the previous upload, in that call's order.
    prev_cpu: Vec<Lod3InstanceRaw>,
    prev_buffer: wgpu::Buffer,
    vector_pipeline: wgpu::RenderPipeline,
    vector_uniform: wgpu::Buffer,
    vector_bind_group: wgpu::BindGroup,
}

impl ImpostorPass {
    /// Create a new impostor pass.
    ///
    /// * `pixels`, `width`, `height`, `spec` — atlas contents and layout,
    ///   typically produced by [`crate::impostor_bake::load_or_bake_atlas`]
    ///   or the `aw-impostor-bake` CLI.
    /// * `color_format`, `depth_format` — must match the final render
    ///   target(s) the caller will invoke [`ImpostorPass::record`] against.
    ///
    /// The instance buffer starts with a small default capacity; the first
    /// [`ImpostorPass::upload_instances`] call grows it as needed.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pixels: &[u8],
        width: u32,
        height: u32,
        spec: ImpostorAtlasSpec,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Result<Self> {
        use wgpu::util::DeviceExt;

        let pipeline = build_lod3_pipeline(device, color_format, depth_format)?;
        let resources =
            Lod3Resources::upload(device, queue, pixels, width, height, spec, &pipeline)?;

        // Camera UBO — populated with identity matrices until update_camera is called.
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("impostor-pass-camera-ubo"),
            size: CAMERA_UBO_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let identity = CameraUboRaw {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0; 4],
        };
        queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&identity));

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("impostor-pass-camera-bg"),
            layout: &pipeline.camera_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: resources.rows_buffer.as_entire_binding(),
                },
            ],
        });

        let quad_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("impostor-pass-quad-vbuf"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("impostor-pass-quad-ibuf"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Start with a reasonable baseline so the first upload typically
        // doesn't force a reallocation.
        let instance_capacity = 64;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("impostor-pass-instance-buffer"),
            size: (instance_capacity * std::mem::size_of::<Lod3InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let prev_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("impostor-pass-prev-instances"),
            size: (instance_capacity * std::mem::size_of::<Lod3InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vector_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("impostor scene vectors"),
            source: wgpu::ShaderSource::Wgsl(IMPOSTOR_VECTOR_WGSL.into()),
        });
        let vector_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("impostor scene vectors"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
        let vector_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("impostor scene vector matrices"),
            size: std::mem::size_of::<ImpostorVectorMats>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vector_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("impostor scene vectors"),
            layout: &vector_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vector_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: resources.rows_buffer.as_entire_binding(),
                },
            ],
        });
        let vector_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("impostor scene vectors"),
            bind_group_layouts: &[&vector_bgl, &pipeline.atlas_bgl],
            push_constant_ranges: &[],
        });
        let vector_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("impostor scene vectors"),
            layout: Some(&vector_layout),
            vertex: wgpu::VertexState {
                module: &vector_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    crate::impostor_lod3::LOD3_QUAD_VERTEX_LAYOUT,
                    IMPOSTOR_CURRENT_LAYOUT,
                    IMPOSTOR_PREV_LAYOUT,
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &vector_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rg16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: -2,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Ok(Self {
            pipeline,
            resources,
            camera_buffer,
            camera_bind_group,
            quad_vbuf,
            quad_ibuf,
            instance_buffer,
            instance_capacity,
            instance_count: 0,
            prev_cpu: Vec::new(),
            prev_buffer,
            vector_pipeline,
            vector_uniform,
            vector_bind_group,
        })
    }

    /// Update the camera UBO for the next recorded draw.
    pub fn update_camera(&self, queue: &wgpu::Queue, view_proj: Mat4, camera_pos: Vec3) {
        let raw = CameraUboRaw {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z, 0.0],
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&raw));
    }

    /// Upload LOD3 instances for the next recorded draw. Resizes the
    /// instance buffer if `instances.len()` exceeds the current capacity.
    ///
    /// Passing an empty slice is legal; subsequent [`ImpostorPass::record`]
    /// calls will issue zero draws.
    pub fn upload_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[Lod3InstanceRaw],
    ) {
        if instances.len() > self.instance_capacity {
            // Grow geometrically to amortise future resizes.
            let mut new_cap = self.instance_capacity.max(1);
            while new_cap < instances.len() {
                new_cap *= 2;
            }
            let bytes = (new_cap * std::mem::size_of::<Lod3InstanceRaw>()) as u64;
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("impostor-pass-instance-buffer"),
                size: bytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.prev_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("impostor-pass-prev-instances"),
                size: bytes,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity = new_cap;
        }
        let prev_src: Vec<Lod3InstanceRaw> = if self.prev_cpu.is_empty() {
            instances.to_vec()
        } else if self.prev_cpu.len() >= instances.len() {
            self.prev_cpu[..instances.len()].to_vec()
        } else {
            let mut padded = self.prev_cpu.clone();
            padded.extend(instances[self.prev_cpu.len()..].iter().copied());
            padded
        };
        if !prev_src.is_empty() {
            queue.write_buffer(&self.prev_buffer, 0, bytemuck::cast_slice(&prev_src));
        }
        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }
        self.prev_cpu = instances.to_vec();
        self.instance_count = instances.len() as u32;
    }

    /// Draw billboard normals into the scene-vector targets.
    ///
    /// The facing direction matches the color-pass billboard. Alpha below
    /// 0.05 is discarded, same as the atlas silhouette. Previous positions
    /// are the last `upload_instances` set.
    /// Write the current instance set into the previous buffer.
    ///
    /// The first enabled scene-vector frame uses the current view-projection
    /// as the previous one, and the mesh slots were cleared. Impostor uploads
    /// keep a one-frame delta while the pass is off. This drops that delta
    /// so the first frame back on stores zero object motion, matching meshes.
    pub fn align_history_to_current(&self, queue: &wgpu::Queue) {
        if self.prev_cpu.is_empty() {
            return;
        }
        queue.write_buffer(&self.prev_buffer, 0, bytemuck::cast_slice(&self.prev_cpu));
    }

    pub fn prepare_scene_vectors(
        &self,
        queue: &wgpu::Queue,
        curr_vp: Mat4,
        prev_vp: Mat4,
        view: Mat4,
    ) {
        if self.instance_count == 0 {
            return;
        }
        let mats = ImpostorVectorMats {
            curr_vp: curr_vp.to_cols_array_2d(),
            prev_vp: prev_vp.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
        };
        queue.write_buffer(&self.vector_uniform, 0, bytemuck::bytes_of(&mats));
    }

    pub fn record_scene_vectors<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.vector_pipeline);
        pass.set_bind_group(0, &self.vector_bind_group, &[]);
        pass.set_bind_group(1, &self.resources.atlas_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vbuf.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_vertex_buffer(2, self.prev_buffer.slice(..));
        pass.set_index_buffer(self.quad_ibuf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..self.instance_count);
    }

    /// Number of instances recorded by the most recent
    /// [`ImpostorPass::upload_instances`] call. `0` means [`record`] is a no-op.
    ///
    /// [`record`]: Self::record
    pub fn instance_count(&self) -> u32 {
        self.instance_count
    }

    /// Read-only access to the pipeline this pass drives. Exposed so
    /// callers can inspect `color_format`-matching bind group layouts when
    /// integrating into a larger render graph.
    pub fn pipeline(&self) -> &Lod3Pipeline {
        &self.pipeline
    }

    /// Record the LOD3 draw into an active render pass.
    ///
    /// No-op when [`ImpostorPass::instance_count`] is zero.
    pub fn record<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.resources.atlas_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vbuf.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.quad_ibuf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..self.instance_count);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_ubo_layout_is_80_bytes() {
        // mat4x4<f32> (64) + vec4<f32> (16) = 80 B. Must match WGSL `Camera`.
        assert_eq!(std::mem::size_of::<CameraUboRaw>(), 80);
        assert_eq!(CAMERA_UBO_BYTES, 80);
    }

    #[test]
    fn quad_vertex_struct_size_matches_layout() {
        use crate::impostor_lod3::LOD3_QUAD_VERTEX_LAYOUT;
        assert_eq!(std::mem::size_of::<QuadVertex>(), 20);
        assert_eq!(LOD3_QUAD_VERTEX_LAYOUT.array_stride as usize, 20);
    }

    #[test]
    fn quad_indices_are_ccw() {
        // Triangle 1: (0,1,2). Triangle 2: (0,2,3). Both wind CCW when
        // looking from +Z toward origin — matches pipeline `front_face: Ccw`.
        assert_eq!(QUAD_INDICES, [0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn quad_vertices_cover_unit_uv_range() {
        // U: 0→1 covers full width. V: 0→1 covers full height.
        let us: Vec<f32> = QUAD_VERTICES.iter().map(|v| v.uv[0]).collect();
        let vs: Vec<f32> = QUAD_VERTICES.iter().map(|v| v.uv[1]).collect();
        assert_eq!(us.iter().cloned().fold(f32::INFINITY, f32::min), 0.0);
        assert_eq!(us.iter().cloned().fold(f32::NEG_INFINITY, f32::max), 1.0);
        assert_eq!(vs.iter().cloned().fold(f32::INFINITY, f32::min), 0.0);
        assert_eq!(vs.iter().cloned().fold(f32::NEG_INFINITY, f32::max), 1.0);
    }
}
