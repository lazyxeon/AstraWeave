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
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use wgpu::util::DeviceExt;

use super::camera::OrbitCamera;
use super::terrain_renderer::{TerrainFogParams, TerrainLightingParams};

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
    // Lighting uniforms (matching terrain shader)
    sun_dir: [f32; 3],
    sun_intensity: f32,
    sun_color: [f32; 3],
    ambient_intensity: f32,
    ambient_color: [f32; 3],
    exposure: f32,
}

// ─── Cached Mesh ─────────────────────────────────────────────────────────────

/// A loaded mesh with GPU buffers ready for instanced rendering.
struct CachedMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    vertex_count: u32,
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
    lighting_params: TerrainLightingParams,
    start_time: std::time::Instant,

    // Stats
    last_instance_count: u32,
    last_draw_calls: u32,

    // Instance cache: avoid rebuilding every frame when camera is stationary
    cached_camera_pos: Vec3,
    cached_camera_yaw: f32,
    cached_camera_pitch: f32,
    cached_instances: Vec<ScatterInstance>,
    cached_draw_groups: Vec<DrawGroup>,
    cached_placement_count: usize,
    cache_valid: bool,

    // Diagnostic logging
    last_log_second: u32,
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
                    blend: Some(wgpu::BlendState::REPLACE),
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
                // Negative depth bias pushes scatter fragments slightly closer to the
                // camera than terrain, eliminating z-fighting at the terrain surface.
                bias: wgpu::DepthBiasState {
                    constant: -4,
                    slope_scale: -2.0,
                    clamp: 0.0,
                },
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
                cull_distance: 800.0,
                sun_dir: [0.5, 0.7, 0.35],
                sun_intensity: 2.0,
                sun_color: [1.0, 0.95, 0.85],
                ambient_intensity: 0.7,
                ambient_color: [0.72, 0.70, 0.68],
                exposure: 1.8,
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
            cull_distance: 700.0,
            fog_params: TerrainFogParams::default(),
            lighting_params: TerrainLightingParams::default(),
            start_time: std::time::Instant::now(),
            last_instance_count: 0,
            last_draw_calls: 0,
            cached_camera_pos: Vec3::ZERO,
            cached_camera_yaw: 0.0,
            cached_camera_pitch: 0.0,
            cached_instances: Vec::new(),
            cached_draw_groups: Vec::new(),
            cached_placement_count: 0,
            cache_valid: false,
            last_log_second: u32::MAX,
        })
    }

    // ─── Configuration ───────────────────────────────────────────────────────

    pub fn set_wind(&mut self, strength: f32, frequency: f32) {
        self.wind_strength = strength;
        self.wind_frequency = frequency;
    }

    pub fn set_cull_distance(&mut self, distance: f32) {
        let new_dist = distance.max(10.0);
        if (new_dist - self.cull_distance).abs() > 0.01 {
            self.cache_valid = false;
        }
        self.cull_distance = new_dist;
    }

    pub fn set_fog_params(&mut self, params: TerrainFogParams) {
        self.fog_params = params;
    }

    pub fn set_lighting_params(&mut self, params: TerrainLightingParams) {
        self.lighting_params = params;
    }

    pub fn last_instance_count(&self) -> u32 {
        self.last_instance_count
    }

    pub fn last_draw_calls(&self) -> u32 {
        self.last_draw_calls
    }

    /// Total triangles rendered last frame (instances × mesh triangles per draw group).
    pub fn last_total_triangles(&self) -> usize {
        self.cached_draw_groups
            .iter()
            .map(|g| {
                let mesh_tris = self
                    .mesh_cache
                    .get(&g.mesh_key)
                    .map_or(0, |m| m.index_count as usize / 3);
                mesh_tris * g.instance_count as usize
            })
            .sum()
    }

    /// Total vertices rendered last frame (instances × mesh vertices per draw group).
    pub fn last_total_vertices(&self) -> usize {
        self.cached_draw_groups
            .iter()
            .map(|g| {
                let mesh_verts = self
                    .mesh_cache
                    .get(&g.mesh_key)
                    .map_or(0, |m| m.vertex_count as usize);
                mesh_verts * g.instance_count as usize
            })
            .sum()
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
        let (document, buffers, images) =
            gltf::import(path).with_context(|| format!("Failed to import glTF: {path}"))?;

        let mesh = document.meshes().next().context("No meshes in glTF file")?;

        // Merge ALL primitives into one vertex/index buffer so every material
        // (e.g. leaves + bark) is rendered in a single draw call with correct colors.
        let mut all_vertices: Vec<ScatterVertex> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(p) => p.collect(),
                None => continue,
            };

            let normals: Vec<[f32; 3]> = if let Some(normals) = reader.read_normals() {
                normals.collect()
            } else {
                vec![[0.0, 1.0, 0.0]; positions.len()]
            };

            let vertex_colors: Vec<[f32; 4]> = if let Some(colors) = reader.read_colors(0) {
                tracing::info!("Scatter mesh '{key}': using explicit vertex colors");
                colors.into_rgba_f32().collect()
            } else {
                let material = primitive.material();
                let pbr = material.pbr_metallic_roughness();
                let base_color_factor = pbr.base_color_factor();

                // Try texture baking if available
                let uvs: Option<Vec<[f32; 2]>> =
                    reader.read_tex_coords(0).map(|tc| tc.into_f32().collect());

                if let (Some(tex_info), Some(ref uv_coords)) = (pbr.base_color_texture(), &uvs) {
                    let tex_index = tex_info.texture().source().index();
                    if tex_index < images.len() {
                        tracing::info!("Scatter mesh '{key}': UV texture baking (tex {tex_index}, factor=[{:.2},{:.2},{:.2},{:.2}])",
                            base_color_factor[0], base_color_factor[1], base_color_factor[2], base_color_factor[3]);
                        let img = &images[tex_index];
                        let w = img.width as usize;
                        let h = img.height as usize;
                        uv_coords
                            .iter()
                            .map(|uv| {
                                let u = (uv[0].fract() + 1.0).fract();
                                let v = (uv[1].fract() + 1.0).fract();
                                let px = ((u * w as f32) as usize).min(w.saturating_sub(1));
                                let py = ((v * h as f32) as usize).min(h.saturating_sub(1));
                                let bpp = match img.format {
                                    gltf::image::Format::R8G8B8A8 => 4,
                                    gltf::image::Format::R8G8B8 => 3,
                                    _ => 0,
                                };
                                if bpp >= 3 {
                                    let idx = (py * w + px) * bpp;
                                    if idx + 2 < img.pixels.len() {
                                        [
                                            (img.pixels[idx] as f32 / 255.0) * base_color_factor[0],
                                            (img.pixels[idx + 1] as f32 / 255.0)
                                                * base_color_factor[1],
                                            (img.pixels[idx + 2] as f32 / 255.0)
                                                * base_color_factor[2],
                                            if bpp == 4 && idx + 3 < img.pixels.len() {
                                                (img.pixels[idx + 3] as f32 / 255.0)
                                                    * base_color_factor[3]
                                            } else {
                                                base_color_factor[3]
                                            },
                                        ]
                                    } else {
                                        base_color_factor
                                    }
                                } else {
                                    base_color_factor
                                }
                            })
                            .collect()
                    } else {
                        tracing::info!("Scatter mesh '{key}': flat base_color_factor fallback [{:.2},{:.2},{:.2},{:.2}]",
                            base_color_factor[0], base_color_factor[1], base_color_factor[2], base_color_factor[3]);
                        vec![base_color_factor; positions.len()]
                    }
                } else {
                    tracing::info!("Scatter mesh '{key}': flat base_color_factor fallback [{:.2},{:.2},{:.2},{:.2}]",
                        base_color_factor[0], base_color_factor[1], base_color_factor[2], base_color_factor[3]);
                    vec![base_color_factor; positions.len()]
                }
            };

            let indices: Vec<u32> = match reader.read_indices() {
                Some(idx) => idx.into_u32().collect(),
                None => continue,
            };

            // Offset indices by current vertex count to merge into one buffer
            let base_vertex = all_vertices.len() as u32;

            for ((p, n), c) in positions
                .iter()
                .zip(normals.iter())
                .zip(vertex_colors.iter())
            {
                all_vertices.push(ScatterVertex {
                    position: *p,
                    normal: *n,
                    color: *c,
                });
            }

            for idx in &indices {
                all_indices.push(idx + base_vertex);
            }
        }

        anyhow::ensure!(!all_vertices.is_empty(), "No vertex data in any primitive");
        anyhow::ensure!(!all_indices.is_empty(), "No index data in any primitive");

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Scatter VB: {key}")),
                contents: bytemuck::cast_slice(&all_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Scatter IB: {key}")),
                contents: bytemuck::cast_slice(&all_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        tracing::info!(
            "Scatter: loaded mesh '{key}': {} verts, {} tris",
            all_vertices.len(),
            all_indices.len() / 3
        );

        self.mesh_cache.insert(
            key.to_string(),
            CachedMesh {
                vertex_buffer,
                index_buffer,
                index_count: all_indices.len() as u32,
                vertex_count: all_vertices.len() as u32,
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

        // Update uniforms — camera-relative VP to avoid f32 jitter far from origin
        let view_proj = camera.view_projection_matrix_relative();
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
                sun_dir: self.lighting_params.sun_dir,
                sun_intensity: self.lighting_params.sun_intensity,
                sun_color: self.lighting_params.sun_color,
                ambient_intensity: self.lighting_params.ambient_intensity,
                ambient_color: self.lighting_params.ambient_color,
                exposure: self.lighting_params.exposure,
            }),
        );

        // Lazy-load meshes (may invalidate cache if new meshes loaded)
        let paths_to_load: Vec<(String, String)> = placements
            .iter()
            .filter(|p| !self.mesh_cache.contains_key(&p.mesh_key))
            .map(|p| (p.mesh_key.clone(), p.mesh_path.clone()))
            .collect::<HashMap<_, _>>()
            .into_iter()
            .collect();

        for (key, path) in &paths_to_load {
            if let Err(e) = self.load_gltf_mesh(key, path) {
                tracing::warn!("Scatter: failed to load mesh '{key}': {e}");
            }
        }
        if !paths_to_load.is_empty() {
            self.cache_valid = false;
        }

        // Rebuild instance list with distance + frustum culling.
        // The frustum near-plane is now correctly extracted for wgpu's [0,1] depth
        // (using just row2 instead of the OpenGL row3+row2 formula), which was
        // previously the root cause of frame-to-frame culling instability.
        //
        // Cache: skip rebuild when camera hasn't moved and placements haven't changed.
        // This prevents per-frame allocation churn and eliminates flicker from
        // non-deterministic HashMap iteration order (now uses BTreeMap for stable ordering).
        let cam_moved = (camera_pos - self.cached_camera_pos).length_squared() > 0.01
            || (camera.yaw() - self.cached_camera_yaw).abs() > 0.001
            || (camera.pitch() - self.cached_camera_pitch).abs() > 0.001;
        let placements_changed = placements.len() != self.cached_placement_count;

        if cam_moved || placements_changed || !self.cache_valid {
            let frustum = camera.extract_frustum();
            // CPU cull at 10% beyond shader cull_distance so fade completes
            let cpu_cull = self.cull_distance * 1.10;
            let cull_dist_sq = cpu_cull * cpu_cull;

            // BTreeMap for deterministic draw group ordering — eliminates flicker
            let mut grouped: BTreeMap<String, Vec<ScatterInstance>> = BTreeMap::new();

            for placement in placements {
                if !self.mesh_cache.contains_key(&placement.mesh_key) {
                    continue;
                }
                let delta = placement.position - camera_pos;
                if delta.length_squared() > cull_dist_sq {
                    continue;
                }
                let cull_radius = placement.bounding_radius.max(3.0);
                if !frustum.contains_sphere(placement.position, cull_radius) {
                    continue;
                }

                let is_tree =
                    placement.mesh_key.contains("tree") || placement.mesh_key.contains("pine");
                let rotation = if is_tree {
                    // Trees stay upright (Y-axis only) for natural appearance
                    Quat::from_rotation_y(placement.rotation)
                } else {
                    // Rocks, bushes, grass etc. tilt to match terrain surface normal
                    let up_to_normal = Quat::from_rotation_arc(Vec3::Y, placement.terrain_normal);
                    up_to_normal * Quat::from_rotation_y(placement.rotation)
                };
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
                        tint: placement.tint,
                    });
            }

            // Flatten into cached arrays, respecting the MAX_SCATTER_INSTANCES cap.
            self.cached_instances.clear();
            self.cached_draw_groups.clear();
            let instance_cap = MAX_SCATTER_INSTANCES as usize;

            for (mesh_key, instances) in &grouped {
                let remaining = instance_cap.saturating_sub(self.cached_instances.len());
                if remaining == 0 {
                    break;
                }
                let count = instances.len().min(remaining);
                let first = self.cached_instances.len() as u32;
                self.cached_instances.extend_from_slice(&instances[..count]);
                self.cached_draw_groups.push(DrawGroup {
                    mesh_key: mesh_key.clone(),
                    first_instance: first,
                    instance_count: count as u32,
                });
            }

            self.cached_camera_pos = camera_pos;
            self.cached_camera_yaw = camera.yaw();
            self.cached_camera_pitch = camera.pitch();
            self.cached_placement_count = placements.len();
            self.cache_valid = true;
        }

        if self.cached_instances.is_empty() {
            self.last_instance_count = 0;
            self.last_draw_calls = 0;
            return Ok(());
        }

        let total = self.cached_instances.len();
        debug_assert!(total <= MAX_SCATTER_INSTANCES as usize);
        self.last_instance_count = total as u32;
        self.last_draw_calls = self.cached_draw_groups.len() as u32;

        // Diagnostic: log scatter stats once per second
        let elapsed_secs = time as u32;
        if elapsed_secs != self.last_log_second {
            self.last_log_second = elapsed_secs;
            tracing::info!(
                "Scatter: {} placements, {} meshes cached, {} instances, {} draw groups",
                placements.len(),
                self.mesh_cache.len(),
                total,
                self.cached_draw_groups.len(),
            );
        }

        // Upload instance data (already capped at MAX_SCATTER_INSTANCES)
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.cached_instances),
        );

        // ── Render pass (direct draw calls — more reliable than indirect) ─────

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

        // Issue one direct draw per mesh type (simpler and more portable than indirect draws)
        for group in &self.cached_draw_groups {
            if let Some(mesh) = self.mesh_cache.get(&group.mesh_key) {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
                pass.draw_indexed(
                    0..mesh.index_count,
                    0,
                    group.first_instance..group.first_instance + group.instance_count,
                );
            }
        }

        Ok(())
    }
}
