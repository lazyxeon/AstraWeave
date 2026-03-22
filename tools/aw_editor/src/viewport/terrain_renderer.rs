#![allow(dead_code)]

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use image::GenericImageView;
use std::path::{Path, PathBuf};
use wgpu::util::DeviceExt;

use super::camera::OrbitCamera;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub biome_weights_0: [f32; 4],
    pub biome_weights_1: [f32; 4],
    pub splat_weights_0: [f32; 4],
    pub splat_weights_1: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    shading_mode: u32,
    fog_color: [f32; 3],
    fog_density: f32,
    fog_enabled: u32,
    weather_type: u32,
    time: f32,
    water_level: f32,
    // Lighting uniforms (new)
    sun_dir: [f32; 3],
    sun_intensity: f32,
    sun_color: [f32; 3],
    ambient_intensity: f32,
    ambient_color: [f32; 3],
    exposure: f32,
}

/// Fog and weather parameters passed to the terrain shader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainFogParams {
    pub fog_enabled: bool,
    pub fog_density: f32,
    pub fog_color: [f32; 3],
    pub weather_type: u32,
    /// Optional override for particle count (None = use default for weather type)
    pub particle_count_override: Option<u32>,
}

impl Default for TerrainFogParams {
    fn default() -> Self {
        Self {
            fog_enabled: false,
            fog_density: 0.01,
            particle_count_override: None,
            fog_color: [0.6, 0.6, 0.62],
            weather_type: 0,
        }
    }
}

/// Lighting parameters passed to the terrain shader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainLightingParams {
    pub sun_dir: [f32; 3],
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub exposure: f32,
}

impl Default for TerrainLightingParams {
    fn default() -> Self {
        Self {
            sun_dir: [0.5, 0.7, 0.35],
            sun_color: [1.0, 0.95, 0.85],
            sun_intensity: 2.0,
            ambient_color: [0.72, 0.70, 0.68],
            ambient_intensity: 0.7,
            exposure: 1.8,
        }
    }
}

// ─── PBR Texture Loading ─────────────────────────────────────────────────────

const BIOME_TEX_SIZE: u32 = 2048;
const BIOME_COUNT: u32 = 8;
const MATERIAL_LAYER_COUNT: u32 = 10;

/// Resolve the asset base directory by walking up from the executable until
/// we find a directory containing `assets/materials/grass.png`.
fn find_assets_dir() -> PathBuf {
    // Try working directory first
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("assets/materials/grass.png").exists() {
        return cwd.join("assets");
    }
    // Walk up from executable location
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            if d.join("assets/materials/grass.png").exists() {
                return d.join("assets");
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
    // Fallback
    PathBuf::from("assets")
}

/// Load a single texture from disk, resize to `target_size`, return RGBA8 bytes.
/// Returns magenta fallback on failure.
fn load_texture_layer(path: &Path, target_size: u32) -> Vec<u8> {
    let fallback = || -> Vec<u8> {
        eprintln!(
            "[terrain] WARN: missing texture {:?}, using magenta fallback",
            path
        );
        vec![255u8, 0, 255, 255].repeat((target_size * target_size) as usize)
    };

    let img = match image::open(path) {
        Ok(i) => i,
        Err(_) => return fallback(),
    };

    let (w, h) = img.dimensions();
    let resized = if w == target_size && h == target_size {
        img.to_rgba8()
    } else {
        image::imageops::resize(
            &img.to_rgba8(),
            target_size,
            target_size,
            image::imageops::FilterType::Lanczos3,
        )
    };

    resized.into_raw()
}

/// Load an array of textures (one per biome layer), concatenated into a single
/// byte buffer suitable for uploading as a texture_2d_array.
fn load_biome_texture_array(materials_dir: &Path, filenames: &[&str], target_size: u32) -> Vec<u8> {
    let layer_bytes = (target_size * target_size * 4) as usize;
    let mut data = Vec::with_capacity(layer_bytes * filenames.len());
    for filename in filenames {
        let path = materials_dir.join(filename);
        let layer = load_texture_layer(&path, target_size);
        assert_eq!(layer.len(), layer_bytes);
        data.extend_from_slice(&layer);
    }
    data
}

// Biome index → texture filename mappings
const BIOME_ALBEDO_FILES: [&str; 10] = [
    "grass.png",         // 0: Grassland
    "sand.png",          // 1: Desert
    "forest_floor.png",  // 2: Forest
    "mountain_rock.png", // 3: Mountain
    "snow.png",          // 4: Tundra
    "mud.png",           // 5: Swamp
    "sand.png",          // 6: Beach (reuse sand)
    "stone.png",         // 7: River
    "rock_slate.png",    // 8: Shared steep rock
    "dirt.png",          // 9: Shared dirt breakup
];

const BIOME_NORMAL_FILES: [&str; 10] = [
    "grass_n.png",
    "sand_n.png",
    "forest_floor_n.png",
    "mountain_rock_n.png",
    "snow_n.png",
    "mud_n.png",
    "sand_n.png",
    "stone_n.png",
    "rock_slate_n.png",
    "dirt_n.png",
];

const BIOME_MRA_FILES: [&str; 10] = [
    "grass_mra.png",
    "sand_mra.png",
    "forest_floor_mra.png",
    "mountain_rock_mra.png",
    "snow_mra.png",
    "mud_mra.png",
    "sand_mra.png",
    "stone_mra.png",
    "rock_slate_mra.png",
    "dirt_mra.png",
];

pub struct TerrainChunkGpu {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    center: Vec3,
    radius: f32,
}

pub struct TerrainRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    device: wgpu::Device,
    chunks: Vec<TerrainChunkGpu>,
    fog_params: TerrainFogParams,
    lighting_params: TerrainLightingParams,
    start_time: std::time::Instant,
    water_level: f32,
    _biome_texture: wgpu::Texture,
    _biome_tex_view: wgpu::TextureView,
    _biome_normal_texture: wgpu::Texture,
    _biome_normal_view: wgpu::TextureView,
    _biome_mra_texture: wgpu::Texture,
    _biome_mra_view: wgpu::TextureView,
    _biome_sampler: wgpu::Sampler,
}

impl TerrainRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/terrain.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Terrain Bind Group Layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terrain Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Terrain Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TerrainVertex>() as u64,
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
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 48,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 64,
                            shader_location: 5,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 80,
                            shader_location: 6,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
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
                cull_mode: Some(wgpu::Face::Back),
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
            label: Some("Terrain Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                view_proj: [[0.0; 4]; 4],
                camera_pos: [0.0; 3],
                shading_mode: 0,
                fog_color: [0.6, 0.6, 0.62],
                fog_density: 0.01,
                fog_enabled: 0,
                weather_type: 0,
                time: 0.0,
                water_level: 0.0,
                sun_dir: [0.5, 0.7, 0.35],
                sun_intensity: 2.0,
                sun_color: [1.0, 0.95, 0.85],
                ambient_intensity: 0.7,
                ambient_color: [0.72, 0.70, 0.68],
                exposure: 1.8,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ─── Load PBR texture arrays from disk ─────────────────────────────────
        let assets_dir = find_assets_dir();
        let materials_dir = assets_dir.join("materials");
        eprintln!("[terrain] Loading PBR textures from {:?}", materials_dir);

        let albedo_data =
            load_biome_texture_array(&materials_dir, &BIOME_ALBEDO_FILES, BIOME_TEX_SIZE);
        let normal_data =
            load_biome_texture_array(&materials_dir, &BIOME_NORMAL_FILES, BIOME_TEX_SIZE);
        let mra_data = load_biome_texture_array(&materials_dir, &BIOME_MRA_FILES, BIOME_TEX_SIZE);
        eprintln!(
            "[terrain] Loaded {} albedo + {} normal + {} MRA bytes",
            albedo_data.len(),
            normal_data.len(),
            mra_data.len()
        );

        // ── Diagnostic: sample center pixels of grass layer (index 0) ──────────
        {
            let layer_bytes = (BIOME_TEX_SIZE * BIOME_TEX_SIZE * 4) as usize;
            let center = (BIOME_TEX_SIZE / 2 * BIOME_TEX_SIZE + BIOME_TEX_SIZE / 2) as usize * 4;
            if albedo_data.len() >= layer_bytes {
                let (r, g, b) = (
                    albedo_data[center],
                    albedo_data[center + 1],
                    albedo_data[center + 2],
                );
                eprintln!("[terrain] DIAG grass albedo center pixel: R={r} G={g} B={b}");
            }
            if mra_data.len() >= layer_bytes {
                let (m, rough, ao) = (mra_data[center], mra_data[center + 1], mra_data[center + 2]);
                eprintln!(
                    "[terrain] DIAG grass MRA center pixel: metallic={m} roughness={rough} ao={ao}"
                );
            }
        }

        let mip_count = (BIOME_TEX_SIZE as f32).log2() as u32 + 1; // 2048 → 11 levels

        // Helper: create a texture array, upload mip 0, generate CPU mipmaps
        let create_texture_array = |label: &str,
                                    data: Vec<u8>,
                                    format: wgpu::TextureFormat|
         -> (wgpu::Texture, wgpu::TextureView) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: BIOME_TEX_SIZE,
                    height: BIOME_TEX_SIZE,
                    depth_or_array_layers: MATERIAL_LAYER_COUNT,
                },
                mip_level_count: mip_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            // Upload mip level 0
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(BIOME_TEX_SIZE * 4),
                    rows_per_image: Some(BIOME_TEX_SIZE),
                },
                wgpu::Extent3d {
                    width: BIOME_TEX_SIZE,
                    height: BIOME_TEX_SIZE,
                    depth_or_array_layers: MATERIAL_LAYER_COUNT,
                },
            );

            // Generate and upload CPU mipmaps (box filter)
            {
                let size = BIOME_TEX_SIZE as usize;
                let mut prev_data = data;
                let mut prev_size = size;
                for mip in 1..mip_count {
                    let new_size = prev_size / 2;
                    if new_size == 0 {
                        break;
                    }
                    let new_layer_bytes = new_size * new_size * 4;
                    let mut mip_data = vec![0u8; new_layer_bytes * MATERIAL_LAYER_COUNT as usize];
                    for layer in 0..MATERIAL_LAYER_COUNT as usize {
                        let src_off = layer * prev_size * prev_size * 4;
                        let dst_off = layer * new_layer_bytes;
                        for y in 0..new_size {
                            for x in 0..new_size {
                                for c in 0..4usize {
                                    let s00 = prev_data
                                        [src_off + ((y * 2) * prev_size + x * 2) * 4 + c]
                                        as u32;
                                    let s10 = prev_data
                                        [src_off + ((y * 2) * prev_size + x * 2 + 1) * 4 + c]
                                        as u32;
                                    let s01 = prev_data
                                        [src_off + ((y * 2 + 1) * prev_size + x * 2) * 4 + c]
                                        as u32;
                                    let s11 = prev_data
                                        [src_off + ((y * 2 + 1) * prev_size + x * 2 + 1) * 4 + c]
                                        as u32;
                                    mip_data[dst_off + (y * new_size + x) * 4 + c] =
                                        ((s00 + s10 + s01 + s11 + 2) / 4) as u8;
                                }
                            }
                        }
                    }
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: mip,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &mip_data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(new_size as u32 * 4),
                            rows_per_image: Some(new_size as u32),
                        },
                        wgpu::Extent3d {
                            width: new_size as u32,
                            height: new_size as u32,
                            depth_or_array_layers: MATERIAL_LAYER_COUNT,
                        },
                    );
                    prev_data = mip_data;
                    prev_size = new_size;
                }
            }

            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });

            (texture, view)
        };

        let (biome_texture, biome_tex_view) = create_texture_array(
            "Biome Albedo Array",
            albedo_data,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let (biome_normal_texture, biome_normal_view) = create_texture_array(
            "Biome Normal Array",
            normal_data,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let (biome_mra_texture, biome_mra_view) =
            create_texture_array("Biome MRA Array", mra_data, wgpu::TextureFormat::Rgba8Unorm);

        let biome_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Biome Texture Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&biome_tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&biome_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&biome_normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&biome_mra_view),
                },
            ],
        });

        Ok(Self {
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            device: device.clone(),
            chunks: Vec::new(),
            fog_params: TerrainFogParams::default(),
            lighting_params: TerrainLightingParams::default(),
            start_time: std::time::Instant::now(),
            water_level: 0.0,
            _biome_texture: biome_texture,
            _biome_tex_view: biome_tex_view,
            _biome_normal_texture: biome_normal_texture,
            _biome_normal_view: biome_normal_view,
            _biome_mra_texture: biome_mra_texture,
            _biome_mra_view: biome_mra_view,
            _biome_sampler: biome_sampler,
        })
    }

    pub fn upload_chunks(&mut self, chunks: &[(Vec<TerrainVertex>, Vec<u32>)]) {
        self.chunks.clear();

        for (vertices, indices) in chunks {
            if vertices.is_empty() || indices.is_empty() {
                continue;
            }

            // Compute bounding sphere from vertex positions
            let mut min = Vec3::splat(f32::MAX);
            let mut max = Vec3::splat(f32::MIN);
            for v in vertices {
                let p = Vec3::from(v.position);
                min = min.min(p);
                max = max.max(p);
            }
            let center = (min + max) * 0.5;
            let radius = (max - min).length() * 0.5;

            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Terrain Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Terrain Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

            self.chunks.push(TerrainChunkGpu {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
                center,
                radius,
            });
        }
    }

    pub fn clear_chunks(&mut self) {
        self.chunks.clear();
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Total triangles across all uploaded terrain chunks.
    pub fn total_triangles(&self) -> usize {
        self.chunks.iter().map(|c| c.index_count as usize / 3).sum()
    }

    /// Total indices across all uploaded terrain chunks (vertex count approximation).
    pub fn total_indices(&self) -> usize {
        self.chunks.iter().map(|c| c.index_count as usize).sum()
    }

    pub fn set_fog_params(&mut self, params: TerrainFogParams) {
        self.fog_params = params;
    }

    pub fn set_lighting_params(&mut self, params: TerrainLightingParams) {
        self.lighting_params = params;
    }

    pub fn set_water_level(&mut self, level: f32) {
        self.water_level = level;
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        camera: &OrbitCamera,
        queue: &wgpu::Queue,
        shading_mode: u32,
    ) -> Result<()> {
        if self.chunks.is_empty() {
            return Ok(());
        }

        // Camera-relative VP to avoid f32 jitter far from origin
        let view_proj = camera.view_projection_matrix_relative();
        let camera_pos = camera.position();

        let elapsed = self.start_time.elapsed().as_secs_f32();

        // ── Diagnostic: dump key uniforms once per second ──
        {
            let sec = elapsed as u32;
            static LAST_SEC: std::sync::atomic::AtomicU32 =
                std::sync::atomic::AtomicU32::new(u32::MAX);
            let prev = LAST_SEC.swap(sec, std::sync::atomic::Ordering::Relaxed);
            if sec != prev {
                let fc = self.fog_params.fog_color;
                let ac = self.lighting_params.ambient_color;
                let sc = self.lighting_params.sun_color;
                let sd = self.lighting_params.sun_dir;
                eprintln!(
                    "[terrain] DIAG uniforms: fog=[{:.2},{:.2},{:.2}] ambient=[{:.2},{:.2},{:.2}] sun_color=[{:.2},{:.2},{:.2}] sun_dir=[{:.2},{:.2},{:.2}] exposure={:.2}",
                    fc[0], fc[1], fc[2], ac[0], ac[1], ac[2], sc[0], sc[1], sc[2], sd[0], sd[1], sd[2], self.lighting_params.exposure,
                );
            }
        }

        let uniforms = Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            shading_mode,
            fog_color: self.fog_params.fog_color,
            fog_density: self.fog_params.fog_density,
            fog_enabled: if self.fog_params.fog_enabled { 1 } else { 0 },
            weather_type: self.fog_params.weather_type,
            time: elapsed,
            water_level: self.water_level,
            sun_dir: self.lighting_params.sun_dir,
            sun_intensity: self.lighting_params.sun_intensity,
            sun_color: self.lighting_params.sun_color,
            ambient_intensity: self.lighting_params.ambient_intensity,
            ambient_color: self.lighting_params.ambient_color,
            exposure: self.lighting_params.exposure,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Terrain Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        let frustum = camera.extract_frustum();

        for chunk in &self.chunks {
            if !frustum.contains_sphere(chunk.center, chunk.radius) {
                continue;
            }
            render_pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
            render_pass.set_index_buffer(chunk.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..chunk.index_count, 0, 0..1);
        }

        Ok(())
    }
}
