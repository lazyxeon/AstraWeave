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

use crate::impostor_lod3::{
    build_lod3_pipeline, Lod3InstanceRaw, Lod3Pipeline, Lod3Resources,
};
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
    QuadVertex { pos: [-0.5, 0.0, 0.0], uv: [0.0, 1.0] }, // bottom-left
    QuadVertex { pos: [ 0.5, 0.0, 0.0], uv: [1.0, 1.0] }, // bottom-right
    QuadVertex { pos: [ 0.5, 1.0, 0.0], uv: [1.0, 0.0] }, // top-right
    QuadVertex { pos: [-0.5, 1.0, 0.0], uv: [0.0, 0.0] }, // top-left
];

/// CCW winding (matches `front_face: Ccw` in [`build_lod3_pipeline`]).
const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

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
        let resources = Lod3Resources::upload(device, queue, pixels, width, height, spec, &pipeline)?;

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
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("impostor-pass-instance-buffer"),
                size: (new_cap * std::mem::size_of::<Lod3InstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_capacity = new_cap;
        }
        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }
        self.instance_count = instances.len() as u32;
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
