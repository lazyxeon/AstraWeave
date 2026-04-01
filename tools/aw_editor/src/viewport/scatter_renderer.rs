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

use anyhow::{Context as _, Result};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;
use wgpu::util::DeviceExt;

// ─── Background Mesh Loading ────────────────────────────────────────────────

/// Request sent to the background mesh-loading thread.
struct MeshLoadRequest {
    key: String,
    path: String,
}

/// Result returned from the background mesh-loading thread.
struct MeshLoadResult {
    key: String,
    mesh: Result<LoadedMeshData>,
}

/// Successfully loaded mesh data (GPU resources created on worker thread).
struct LoadedMeshData {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    vertex_count: u32,
    texture_bind_group: Option<wgpu::BindGroup>,
    has_texture: bool,
}

use super::camera::OrbitCamera;
use super::terrain_renderer::{TerrainFogParams, TerrainLightingParams};

// ─── GPU Data Structures ─────────────────────────────────────────────────────

/// Per-vertex data with UV coordinates for texture sampling.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ScatterVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
    uv: [f32; 2],
}

/// Per-instance data uploaded to the GPU each frame.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ScatterInstance {
    model_matrix: [[f32; 4]; 4], // 64 bytes
    tint: [f32; 4],              // 16 bytes  (RGBA, alpha = LOD fade)
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
    /// Per-mesh texture bind group (albedo texture + sampler). None = vertex-color only.
    texture_bind_group: Option<wgpu::BindGroup>,
    /// Whether this mesh has real texture data (vs vertex-color fallback)
    has_texture: bool,
}

// ─── Background Mesh Worker ─────────────────────────────────────────────────

/// Standalone mesh loading function executed on the background thread.
/// All I/O, GLTF parsing, and GPU buffer creation happen here — off the render thread.
fn load_mesh_on_worker(
    key: &str,
    path: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_bgl: &wgpu::BindGroupLayout,
) -> Result<LoadedMeshData> {
    const MAX_SCATTER_MESH_BYTES: u64 = 150 * 1024 * 1024;

    let file_size = std::fs::metadata(path)
        .with_context(|| format!("Cannot stat: {path}"))?
        .len();
    if file_size > MAX_SCATTER_MESH_BYTES {
        anyhow::bail!(
            "File too large for scatter ({:.1} MB, limit {:.0} MB)",
            file_size as f64 / (1024.0 * 1024.0),
            MAX_SCATTER_MESH_BYTES as f64 / (1024.0 * 1024.0),
        );
    }

    let base = Path::new(path).parent();
    let gltf_data =
        gltf::Gltf::open(path).with_context(|| format!("Failed to parse glTF: {path}"))?;
    let gltf::Gltf { document, blob } = gltf_data;
    let buffers = gltf::import_buffers(&document, base, blob)
        .with_context(|| format!("Failed to load glTF buffers: {path}"))?;

    let mesh = document.meshes().next().context("No meshes in glTF file")?;

    let mut all_vertices: Vec<ScatterVertex> = Vec::new();
    let mut all_indices: Vec<u32> = Vec::new();

    for primitive in mesh.primitives() {
        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

        let positions: Vec<[f32; 3]> = match reader.read_positions() {
            Some(p) => p.collect(),
            None => continue,
        };

        let normals: Vec<[f32; 3]> = reader
            .read_normals()
            .map(|n| n.collect())
            .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

        let uvs: Vec<[f32; 2]> = reader
            .read_tex_coords(0)
            .map(|tc| tc.into_f32().collect())
            .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

        let vertex_colors: Vec<[f32; 4]> = if let Some(colors) = reader.read_colors(0) {
            colors.into_rgba_f32().collect()
        } else {
            let pbr = primitive.material().pbr_metallic_roughness();
            vec![pbr.base_color_factor(); positions.len()]
        };

        let indices: Vec<u32> = match reader.read_indices() {
            Some(idx) => idx.into_u32().collect(),
            None => continue,
        };

        let base_vertex = all_vertices.len() as u32;
        for (((p, n), c), uv) in positions
            .iter()
            .zip(normals.iter())
            .zip(vertex_colors.iter())
            .zip(uvs.iter())
        {
            all_vertices.push(ScatterVertex {
                position: *p,
                normal: *n,
                color: *c,
                uv: *uv,
            });
        }
        for idx in &indices {
            all_indices.push(idx + base_vertex);
        }
    }

    anyhow::ensure!(!all_vertices.is_empty(), "No vertex data in any primitive");
    anyhow::ensure!(!all_indices.is_empty(), "No index data in any primitive");

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("Scatter VB: {key}")),
        contents: bytemuck::cast_slice(&all_vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("Scatter IB: {key}")),
        contents: bytemuck::cast_slice(&all_indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // Try to extract albedo texture
    let (texture_bind_group, has_texture) =
        try_extract_albedo(path, &document, &buffers, device, queue, texture_bgl)
            .unwrap_or((None, false));

    tracing::info!(
        "Scatter: loaded mesh '{key}': {} verts, {} tris, texture={}",
        all_vertices.len(),
        all_indices.len() / 3,
        if has_texture { "yes" } else { "vertex-color" },
    );

    Ok(LoadedMeshData {
        vertex_buffer,
        index_buffer,
        index_count: all_indices.len() as u32,
        vertex_count: all_vertices.len() as u32,
        texture_bind_group,
        has_texture,
    })
}

/// Extract albedo texture from GLTF (standalone version for worker thread).
fn try_extract_albedo(
    path: &str,
    document: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_bgl: &wgpu::BindGroupLayout,
) -> Result<(Option<wgpu::BindGroup>, bool)> {
    let material = document
        .materials()
        .next()
        .context("no materials in GLTF")?;
    let tex_info = material
        .pbr_metallic_roughness()
        .base_color_texture()
        .context("no base_color_texture in material")?;
    let image_source = document
        .images()
        .nth(tex_info.texture().source().index())
        .context("texture source image index out of bounds")?;

    let img_data = match image_source.source() {
        gltf::image::Source::View { view, mime_type } => {
            let buf = &buffers[view.buffer().index()];
            let start = view.offset();
            let end = start + view.length();
            let bytes = &buf[start..end];
            image::load_from_memory_with_format(
                bytes,
                match mime_type {
                    "image/png" => image::ImageFormat::Png,
                    "image/jpeg" => image::ImageFormat::Jpeg,
                    _ => anyhow::bail!("unsupported embedded image mime: {mime_type}"),
                },
            )?
        }
        gltf::image::Source::Uri { uri, .. } => {
            let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
            let img_path = base_dir.join(uri);
            image::open(&img_path)
                .with_context(|| format!("failed to load texture: {}", img_path.display()))?
        }
    };

    let rgba = img_data.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());

    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("Scatter Mesh Albedo"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &rgba,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Scatter Mesh Sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Scatter Mesh Texture BG"),
        layout: texture_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    Ok((Some(bind_group), true))
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

pub struct ScatterRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,

    instance_buffer: wgpu::Buffer,

    /// Layout for per-mesh texture bind groups (Arc for sharing with worker thread)
    texture_bind_group_layout: Arc<wgpu::BindGroupLayout>,
    /// Fallback bind group (1×1 white texture) for meshes without textures
    fallback_texture_bind_group: wgpu::BindGroup,

    mesh_cache: HashMap<String, CachedMesh>,

    // Negative cache: mesh keys that failed to load, with timestamp for retry after cooldown
    failed_meshes: HashMap<String, std::time::Instant>,

    // Keys already sent to the background loader (prevents duplicate requests)
    inflight_mesh_keys: std::collections::HashSet<String>,

    // Background mesh loading channels
    mesh_load_tx: std::sync::mpsc::Sender<MeshLoadRequest>,
    mesh_load_rx: std::sync::mpsc::Receiver<MeshLoadResult>,

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
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self> {
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

        // Bind group 1: per-mesh albedo texture (optional — vertex-color fallback when absent)
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Scatter Texture BGL"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Scatter Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Vertex buffer layout 0: per-vertex (position + normal + color + uv)
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
            wgpu::VertexAttribute {
                offset: 40, // position(12) + normal(12) + color(16) = 40
                shader_location: 8,
                format: wgpu::VertexFormat::Float32x2,
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

        // Create fallback 1×1 white texture for meshes without embedded textures
        let fallback_tex = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("Scatter Fallback 1x1 White"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[255u8, 255, 255, 255],
        );
        let fallback_tex_view = fallback_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let fallback_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Scatter Fallback Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });
        let fallback_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scatter Fallback Texture BG"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&fallback_sampler),
                },
            ],
        });

        // Wrap the BGL in Arc for sharing with the worker thread
        let texture_bind_group_layout = Arc::new(texture_bind_group_layout);

        // Spawn background mesh-loading thread
        let (req_tx, req_rx) = std::sync::mpsc::channel::<MeshLoadRequest>();
        let (res_tx, res_rx) = std::sync::mpsc::channel::<MeshLoadResult>();
        {
            let device = device.clone();
            let queue = queue.clone();
            let bgl = texture_bind_group_layout.clone();
            std::thread::Builder::new()
                .name("scatter-mesh-loader".into())
                .spawn(move || {
                    for req in req_rx {
                        let mesh = load_mesh_on_worker(&req.key, &req.path, &device, &queue, &bgl);
                        // If the main thread dropped the receiver, exit gracefully
                        if res_tx.send(MeshLoadResult { key: req.key, mesh }).is_err() {
                            break;
                        }
                    }
                })
                .context("Failed to spawn scatter mesh loader thread")?;
        }

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group,
            uniform_buffer,
            instance_buffer,
            texture_bind_group_layout,
            fallback_texture_bind_group,
            mesh_cache: HashMap::new(),
            failed_meshes: HashMap::new(),
            inflight_mesh_keys: std::collections::HashSet::new(),
            mesh_load_tx: req_tx,
            mesh_load_rx: res_rx,
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

    /// Enqueue a mesh for background loading. Non-blocking — returns immediately.
    /// The mesh will appear in the cache once the background thread completes loading.
    pub fn ensure_mesh_loaded(&mut self, key: &str, path: &str) -> Result<()> {
        if self.mesh_cache.contains_key(key)
            || self.inflight_mesh_keys.contains(key)
            || self.failed_meshes.contains_key(key)
        {
            return Ok(());
        }
        self.inflight_mesh_keys.insert(key.to_string());
        let _ = self.mesh_load_tx.send(MeshLoadRequest {
            key: key.to_string(),
            path: path.to_string(),
        });
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

        // ── Async mesh loading: enqueue new requests to background thread ────
        // Retry failed meshes after 30-second cooldown
        let retry_keys: Vec<String> = self
            .failed_meshes
            .iter()
            .filter(|(_, t)| t.elapsed().as_secs() >= 30)
            .map(|(k, _)| k.clone())
            .collect();
        for key in &retry_keys {
            tracing::info!("Scatter: retrying previously failed mesh '{key}' after cooldown");
            self.failed_meshes.remove(key);
            self.inflight_mesh_keys.remove(key);
        }

        for p in placements {
            if self.mesh_cache.contains_key(&p.mesh_key)
                || self.failed_meshes.contains_key(&p.mesh_key)
                || self.inflight_mesh_keys.contains(&p.mesh_key)
            {
                continue;
            }
            self.inflight_mesh_keys.insert(p.mesh_key.clone());
            let _ = self.mesh_load_tx.send(MeshLoadRequest {
                key: p.mesh_key.clone(),
                path: p.mesh_path.clone(),
            });
        }

        // ── Drain completed loads from background thread (non-blocking) ─────
        let mut any_loaded = false;
        while let Ok(result) = self.mesh_load_rx.try_recv() {
            self.inflight_mesh_keys.remove(&result.key);
            match result.mesh {
                Ok(data) => {
                    self.mesh_cache.insert(
                        result.key,
                        CachedMesh {
                            vertex_buffer: data.vertex_buffer,
                            index_buffer: data.index_buffer,
                            index_count: data.index_count,
                            vertex_count: data.vertex_count,
                            index_format: wgpu::IndexFormat::Uint32,
                            texture_bind_group: data.texture_bind_group,
                            has_texture: data.has_texture,
                        },
                    );
                    any_loaded = true;
                }
                Err(e) => {
                    tracing::error!(
                        "Scatter: background load failed for '{}': {e:#}. Will retry in 30s.",
                        result.key,
                    );
                    self.failed_meshes
                        .insert(result.key, std::time::Instant::now());
                }
            }
        }
        if any_loaded {
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
                "Scatter: {} placements, {} meshes cached, {} failed, {} instances, {} draw groups",
                placements.len(),
                self.mesh_cache.len(),
                self.failed_meshes.len(),
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
                // Bind per-mesh texture (or fallback white texture for vertex-color meshes)
                let tex_bg = mesh
                    .texture_bind_group
                    .as_ref()
                    .unwrap_or(&self.fallback_texture_bind_group);
                pass.set_bind_group(1, tex_bg, &[]);
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
