//! Scatter Object Renderer
//!
//! GPU-instanced renderer for procedurally scattered vegetation, rocks, and props.
//! Uses indirect draw calls for efficient GPU-driven rendering with per-mesh-type
//! batching, frustum culling, LOD fade, and wind animation.
//!
//! # Architecture
//!
//! ```text
//! ScatterManager (CPU placement)
//!     ↓ VegetationInstance[]
//! ScatterRenderer
//!     ├─ GLTF mesh cache (per vegetation type)
//!     ├─ Instance buffer (transforms + tint per object)
//!     ├─ Indirect draw buffer (one DrawIndexedIndirect per mesh type)
//!     └─ Render pass (single pass, multi-draw-indirect)
//! ```
//!
//! # Performance Targets
//!
//! - 50,000 scatter instances @ 60 FPS
//! - Single instance buffer upload per frame
//! - One draw call per mesh type via multi-draw-indirect
//! - CPU frustum cull before GPU upload

#![allow(dead_code)]

use anyhow::{Context as _, Result};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

use super::camera::{Frustum, OrbitCamera};
use super::terrain_renderer::TerrainFogParams;

// ─── GPU Data Structures ─────────────────────────────────────────────────────

/// Per-vertex data (matches entity shader layout for GLTF mesh reuse).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ScatterVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
}

/// Per-instance data uploaded to the GPU each frame.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ScatterInstance {
    model_matrix: [[f32; 4]; 4], // 64 bytes
    tint: [f32; 4],              // 16 bytes  (RGBA, alpha = LOD fade)
}

/// Indirect draw arguments (wgpu `DrawIndexedIndirect`).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct DrawIndexedIndirectArgs {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

/// Uniforms matching the scatter.wgsl shader.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ScatterUniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    time: f32,
    fog_color: [f32; 3],
    fog_density: f32,
    fog_enabled: u32,
    wind_strength: f32,
    wind_frequency: f32,
    cull_distance: f32,
}

// ─── Cached Mesh ─────────────────────────────────────────────────────────────

/// A loaded mesh with GPU buffers ready for instanced rendering.
struct CachedMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    index_format: wgpu::IndexFormat,
}

// ─── CPU-side Scatter Instance ───────────────────────────────────────────────

pub use crate::terrain_integration::ScatterPlacement;

/// A draw group: one mesh type with a range of instances.
struct DrawGroup {
    mesh_key: String,
    first_instance: u32,
    instance_count: u32,
}

// ─── Scatter Renderer ────────────────────────────────────────────────────────

/// Maximum instances supported per frame.
const MAX_SCATTER_INSTANCES: u32 = 65_536;

/// Maximum indirect draw commands (= max unique mesh types in one frame).
const MAX_DRAW_COMMANDS: u32 = 64;

pub struct ScatterRenderer {
    device: Arc<wgpu::Device>,

    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,

    instance_buffer: wgpu::Buffer,
    indirect_buffer: wgpu::Buffer,

    mesh_cache: HashMap<String, CachedMesh>,

    // Wind / environment
    wind_strength: f32,
    wind_frequency: f32,
    cull_distance: f32,
    fog_params: TerrainFogParams,
    start_time: std::time::Instant,

    // Stats
    last_instance_count: u32,
    last_draw_calls: u32,
}

impl ScatterRenderer {
    pub fn new(device: Arc<wgpu::Device>) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Scatter Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scatter.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scatter BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Scatter Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Vertex buffer layout 0: per-vertex (position + normal + color)
        let vertex_attrs = [
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
            },
        ];

        // Vertex buffer layout 1: per-instance (model_matrix + tint)
        let instance_attrs = [
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 4,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: 32,
                shader_location: 5,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: 48,
                shader_location: 6,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: 64,
                shader_location: 7,
                format: wgpu::VertexFormat::Float32x4,
            },
        ];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Scatter Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<ScatterVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &vertex_attrs,
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<ScatterInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &instance_attrs,
                    },
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Two-sided for vegetation
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scatter Uniform Buffer"),
            contents: bytemuck::bytes_of(&ScatterUniforms {
                view_proj: [[0.0; 4]; 4],
                camera_pos: [0.0; 3],
                time: 0.0,
                fog_color: [0.6, 0.6, 0.62],
                fog_density: 0.01,
                fog_enabled: 0,
                wind_strength: 0.0,
                wind_frequency: 1.0,
                cull_distance: 200.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scatter Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Scatter Instance Buffer"),
            size: (MAX_SCATTER_INSTANCES as u64) * std::mem::size_of::<ScatterInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Scatter Indirect Draw Buffer"),
            size: (MAX_DRAW_COMMANDS as u64)
                * std::mem::size_of::<DrawIndexedIndirectArgs>() as u64,
            usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            device,
            pipeline,
            bind_group,
            uniform_buffer,
            instance_buffer,
            indirect_buffer,
            mesh_cache: HashMap::new(),
            wind_strength: 0.5,
            wind_frequency: 1.2,
            cull_distance: 200.0,
            fog_params: TerrainFogParams::default(),
            start_time: std::time::Instant::now(),
            last_instance_count: 0,
            last_draw_calls: 0,
        })
    }

    // ─── Configuration ───────────────────────────────────────────────────────

    pub fn set_wind(&mut self, strength: f32, frequency: f32) {
        self.wind_strength = strength;
        self.wind_frequency = frequency;
    }

    pub fn set_cull_distance(&mut self, distance: f32) {
        self.cull_distance = distance.max(10.0);
    }

    pub fn set_fog_params(&mut self, params: TerrainFogParams) {
        self.fog_params = params;
    }

    pub fn last_instance_count(&self) -> u32 {
        self.last_instance_count
    }

    pub fn last_draw_calls(&self) -> u32 {
        self.last_draw_calls
    }

    // ─── Mesh Management ─────────────────────────────────────────────────────

    /// Load a GLTF/GLB mesh and cache it by key. No-op if already cached.
    pub fn ensure_mesh_loaded(&mut self, key: &str, path: &str) -> Result<()> {
        if self.mesh_cache.contains_key(key) {
            return Ok(());
        }
        self.load_gltf_mesh(key, path)
    }

    fn load_gltf_mesh(&mut self, key: &str, path: &str) -> Result<()> {
        let (document, buffers, _images) =
            gltf::import(path).with_context(|| format!("Failed to import glTF: {path}"))?;

        let mesh = document
            .meshes()
            .next()
            .context("No meshes in glTF file")?;
        let primitive = mesh
            .primitives()
            .next()
            .context("No primitives in mesh")?;

        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        let positions: Vec<[f32; 3]> = reader
            .read_positions()
            .context("No position data in mesh")?
            .collect();

        let normals: Vec<[f32; 3]> = if let Some(normals) = reader.read_normals() {
            normals.collect()
        } else {
            vec![[0.0, 1.0, 0.0]; positions.len()]
        };

        let vertex_colors: Vec<[f32; 4]> = if let Some(colors) = reader.read_colors(0) {
            colors.into_rgba_f32().collect()
        } else {
            let base_color = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_factor();
            vec![base_color; positions.len()]
        };

        let indices: Vec<u32> = reader
            .read_indices()
            .context("No index data in mesh")?
            .into_u32()
            .collect();

        let vertices: Vec<ScatterVertex> = positions
            .iter()
            .zip(normals.iter())
            .zip(vertex_colors.iter())
            .map(|((p, n), c)| ScatterVertex {
                position: *p,
                normal: *n,
                color: *c,
            })
            .collect();

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Scatter VB: {key}")),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Scatter IB: {key}")),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        tracing::info!(
            "Scatter: loaded mesh '{key}': {} verts, {} tris",
            vertices.len(),
            indices.len() / 3
        );

        self.mesh_cache.insert(
            key.to_string(),
            CachedMesh {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
                index_format: wgpu::IndexFormat::Uint32,
            },
        );

        Ok(())
    }

    // ─── Render ──────────────────────────────────────────────────────────────

    /// Render scatter instances.
    ///
    /// Performs CPU frustum culling, groups instances by mesh type,
    /// builds the indirect draw buffer, and issues a single render pass
    /// with one draw_indexed_indirect call per mesh type.
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        camera: &OrbitCamera,
        placements: &[ScatterPlacement],
        queue: &wgpu::Queue,
    ) -> Result<()> {
        if placements.is_empty() {
            self.last_instance_count = 0;
            self.last_draw_calls = 0;
            return Ok(());
        }

        // Update uniforms
        let view_proj = camera.view_projection_matrix();
        let camera_pos = camera.position();
        let time = self.start_time.elapsed().as_secs_f32();

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&ScatterUniforms {
                view_proj: view_proj.to_cols_array_2d(),
                camera_pos: camera_pos.to_array(),
                time,
                fog_color: self.fog_params.fog_color,
                fog_density: self.fog_params.fog_density,
                fog_enabled: if self.fog_params.fog_enabled { 1 } else { 0 },
                wind_strength: self.wind_strength,
                wind_frequency: self.wind_frequency,
                cull_distance: self.cull_distance,
            }),
        );

        let frustum = camera.extract_frustum();
        let cull_dist_sq = self.cull_distance * self.cull_distance;

        // Lazy-load meshes
        let paths_to_load: Vec<(String, String)> = placements
            .iter()
            .filter(|p| !self.mesh_cache.contains_key(&p.mesh_key))
            .map(|p| (p.mesh_key.clone(), p.mesh_path.clone()))
            .collect::<HashMap<_, _>>()
            .into_iter()
            .collect();

        for (key, path) in paths_to_load {
            if let Err(e) = self.load_gltf_mesh(&key, &path) {
                tracing::warn!("Scatter: failed to load mesh '{key}': {e}");
            }
        }

        // CPU frustum + distance cull and group by mesh key
        let mut grouped: HashMap<String, Vec<ScatterInstance>> = HashMap::new();

        for placement in placements {
            // Distance cull
            let delta = placement.position - camera_pos;
            if delta.length_squared() > cull_dist_sq {
                continue;
            }

            // Frustum cull
            if !frustum.contains_sphere(placement.position, placement.bounding_radius) {
                continue;
            }

            // Skip meshes we couldn't load
            if !self.mesh_cache.contains_key(&placement.mesh_key) {
                continue;
            }

            let rotation = Quat::from_rotation_y(placement.rotation);
            let transform = Mat4::from_scale_rotation_translation(
                Vec3::splat(placement.scale),
                rotation,
                placement.position,
            );

            grouped
                .entry(placement.mesh_key.clone())
                .or_default()
                .push(ScatterInstance {
                    model_matrix: transform.to_cols_array_2d(),
                    tint: [1.0, 1.0, 1.0, 1.0],
                });
        }

        if grouped.is_empty() {
            self.last_instance_count = 0;
            self.last_draw_calls = 0;
            return Ok(());
        }

        // Flatten instances and build draw groups
        let mut all_instances: Vec<ScatterInstance> = Vec::new();
        let mut draw_groups: Vec<DrawGroup> = Vec::new();

        for (mesh_key, instances) in &grouped {
            let first = all_instances.len() as u32;
            let count = instances.len() as u32;
            all_instances.extend_from_slice(instances);
            draw_groups.push(DrawGroup {
                mesh_key: mesh_key.clone(),
                first_instance: first,
                instance_count: count,
            });
        }

        let total = all_instances.len().min(MAX_SCATTER_INSTANCES as usize);
        self.last_instance_count = total as u32;
        self.last_draw_calls = draw_groups.len() as u32;

        // Upload instance data
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&all_instances[..total]),
        );

        // Build and upload indirect draw args
        let mut indirect_args: Vec<DrawIndexedIndirectArgs> = Vec::new();
        for group in &draw_groups {
            if let Some(mesh) = self.mesh_cache.get(&group.mesh_key) {
                indirect_args.push(DrawIndexedIndirectArgs {
                    index_count: mesh.index_count,
                    instance_count: group.instance_count,
                    first_index: 0,
                    base_vertex: 0,
                    first_instance: group.first_instance,
                });
            }
        }

        if indirect_args.is_empty() {
            return Ok(());
        }

        queue.write_buffer(
            &self.indirect_buffer,
            0,
            bytemuck::cast_slice(&indirect_args),
        );

        // ── Render pass ──────────────────────────────────────────────────────

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Scatter Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

        // Issue one indirect draw per mesh type
        let stride = std::mem::size_of::<DrawIndexedIndirectArgs>() as u64;
        for (i, group) in draw_groups.iter().enumerate() {
            if let Some(mesh) = self.mesh_cache.get(&group.mesh_key) {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
                pass.draw_indexed_indirect(&self.indirect_buffer, i as u64 * stride);
            }
        }

        Ok(())
    }
}
