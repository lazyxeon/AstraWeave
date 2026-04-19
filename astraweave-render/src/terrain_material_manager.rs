//! Terrain splat-array material pipeline manager (Phase 2.2 — Issue #9 fix).
//!
//! Binds the 8-layer splat shader in `shaders/pbr_terrain.wgsl` (plus the
//! companion vertex stage in `shaders/pbr_terrain_vs.wgsl`) into a complete
//! render pipeline and owns the GPU resources required to draw multi-material
//! terrain chunks:
//!
//! * Four `texture_2d_array<f32>` shared across all chunks — `layer_albedo`,
//!   `layer_normal`, `layer_orm`, `layer_height`, each with up to 8 layers
//!   (configurable resolution via [`TerrainMaterialConfig`]).
//! * Two per-chunk `texture_2d<f32>` splat maps (`splat_0` = RGBA weights for
//!   layers 0..3, `splat_1` = RGBA weights for layers 4..7).
//! * A single `TerrainMaterialGpu` uniform buffer describing the active layer
//!   count, triplanar/height-blend settings, and per-layer material factors.
//!
//! This module is gated behind the `terrain-splat-arrays` Cargo feature and is
//! a no-op additive path: the existing single-texture terrain renderer used by
//! the game-path renderer is unaffected when the feature is off.
//!
//! # High-level flow (editor/game integration)
//!
//! 1. Call [`TerrainMaterialManager::new`] once per device to allocate the
//!    shared texture arrays, sampler, uniform buffer, and bind group layouts.
//! 2. Before the first draw, call [`TerrainMaterialManager::ensure_pipeline`]
//!    with the target color + depth formats to lazily build the render
//!    pipeline.
//! 3. Call [`TerrainMaterialManager::set_material`] to upload the 8 layer
//!    textures (albedo + normal + orm + height) for the current biome pack and
//!    write the `TerrainMaterialGpu` uniform.
//! 4. For each terrain chunk, call [`TerrainMaterialManager::set_chunk_splat`]
//!    to upload the two RGBA8 splat maps (typically rasterised from the
//!    per-vertex `biome_weights_0/1` in the editor).
//! 5. During the render pass, call [`TerrainMaterialManager::draw_chunk`] with
//!    the chunk's vertex/index buffers and the camera bind group.

#![cfg(feature = "terrain-splat-arrays")]

use std::collections::HashMap;

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::terrain_material::TerrainMaterialGpu;

/// Opaque identifier for a loaded terrain chunk splat pair.
///
/// The editor maps its logical chunk coordinates to `ChunkKey` values; the
/// manager does not interpret the key beyond hash-lookup.
pub type ChunkKey = u64;

/// Maximum number of terrain layers supported by [`pbr_terrain.wgsl`].
pub const MAX_TERRAIN_LAYERS: u32 = 8;

/// Number of components per layer texture (RGBA8).
const BYTES_PER_TEXEL: u32 = 4;

/// Configuration controlling the shared layer array resolutions.
///
/// Defaults chosen for editor use: 1024² albedo, 512² for the other channels,
/// giving roughly 56 MB of VRAM for the 8-layer arrays. Games that need higher
/// fidelity can construct a custom config with 2048² albedo.
#[derive(Debug, Clone, Copy)]
pub struct TerrainMaterialConfig {
    /// Albedo array resolution (per-layer square dim). Power-of-two.
    pub albedo_resolution: u32,
    /// Normal / ORM / height array resolution (per-layer square dim).
    pub aux_resolution: u32,
    /// Number of layers to allocate (1..=8). Must match shader expectations.
    pub layer_count: u32,
}

impl Default for TerrainMaterialConfig {
    fn default() -> Self {
        Self {
            albedo_resolution: 1024,
            aux_resolution: 512,
            layer_count: MAX_TERRAIN_LAYERS,
        }
    }
}

impl TerrainMaterialConfig {
    /// Validate the configuration; returns an error for out-of-range values.
    pub fn validate(&self) -> Result<()> {
        if !(1..=MAX_TERRAIN_LAYERS).contains(&self.layer_count) {
            anyhow::bail!(
                "TerrainMaterialConfig.layer_count must be in 1..={}, got {}",
                MAX_TERRAIN_LAYERS,
                self.layer_count
            );
        }
        if self.albedo_resolution == 0 || !self.albedo_resolution.is_power_of_two() {
            anyhow::bail!(
                "TerrainMaterialConfig.albedo_resolution must be power-of-two, got {}",
                self.albedo_resolution
            );
        }
        if self.aux_resolution == 0 || !self.aux_resolution.is_power_of_two() {
            anyhow::bail!(
                "TerrainMaterialConfig.aux_resolution must be power-of-two, got {}",
                self.aux_resolution
            );
        }
        Ok(())
    }

    /// Approximate GPU memory footprint of the shared texture arrays in bytes.
    pub fn approx_memory_bytes(&self) -> u64 {
        let albedo = (self.albedo_resolution as u64).pow(2) * BYTES_PER_TEXEL as u64;
        let aux = (self.aux_resolution as u64).pow(2) * BYTES_PER_TEXEL as u64;
        (albedo + aux * 3) * self.layer_count as u64
    }
}

/// Per-layer texture payload expected by [`TerrainMaterialManager::set_material`].
///
/// Each slice must be `resolution * resolution * 4` bytes (RGBA8). If any
/// channel is `None`, the manager uploads a solid-grey fallback for that layer.
#[derive(Default, Clone)]
pub struct LayerTextures<'a> {
    pub albedo: Option<&'a [u8]>,
    pub normal: Option<&'a [u8]>,
    pub orm: Option<&'a [u8]>,
    pub height: Option<&'a [u8]>,
}

/// Vertex layout consumed by the terrain splat pipeline.
///
/// Matches `TerrainVSInput` in `pbr_terrain_vs.wgsl`.
/// Fields are in world space; normals are assumed unit-length.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TerrainSplatVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl TerrainSplatVertex {
    /// Vertex buffer layout as consumed by the splat pipeline.
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
        ],
    };
}

/// Concatenated vertex + fragment WGSL source for the splat pipeline.
const TERRAIN_SPLAT_SHADER: &str = concat!(
    include_str!("../shaders/pbr_terrain.wgsl"),
    include_str!("../shaders/pbr_terrain_vs.wgsl"),
);

/// Concatenated WGSL source for the Phase 1 forward-lit splat pipeline.
///
/// Composed as `constants.wgsl` + `brdf_common.wgsl` + `pbr_terrain_forward.wgsl`,
/// the same ordering the shader-validation test uses (see
/// `astraweave-render/tests/shader_validation.rs::test_pbr_terrain_forward_validates_with_prefix`).
/// The forward shader references `PI` from constants.wgsl and calls
/// `evaluate_brdf_lod` + `compute_material_lod` from brdf_common.wgsl.
const TERRAIN_FORWARD_SHADER: &str = concat!(
    include_str!("../shaders/constants.wgsl"),
    "\n",
    include_str!("../shaders/brdf_common.wgsl"),
    "\n",
    include_str!("../shaders/pbr_terrain_forward.wgsl"),
);

/// Per-chunk bind group storing the two splat maps (group 2 bindings 1 & 2).
struct ChunkSplat {
    _splat_0: wgpu::Texture,
    _splat_1: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    dims: (u32, u32),
}

/// CPU mirror of `CameraUniforms` in `pbr_terrain.wgsl` (80 bytes).
///
/// Matches the shader exactly:
/// * `view_proj`: mat4x4 (64 B)
/// * `camera_pos`: vec3 + padding (16 B)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CameraUniformsGpu {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _pad0: f32,
}

impl Default for CameraUniformsGpu {
    fn default() -> Self {
        Self {
            view_proj: [[0.0; 4]; 4],
            camera_pos: [0.0; 3],
            _pad0: 0.0,
        }
    }
}

/// CPU mirror of SHADER_SRC's `Camera` struct at
/// `astraweave-render/src/renderer.rs:48-54`. **Must remain byte-identical
/// to that shader struct** — see the Phase 1.E handoff §5 for why.
///
/// Layout (96 bytes, align 16):
/// * `view_proj`  — mat4x4<f32>           offset 0,  size 64
/// * `light_dir`  — vec3<f32>             offset 64, size 12
/// * `_pad0`      — f32                   offset 76, size 4  → 80
/// * `camera_pos` — vec3<f32>             offset 80, size 12
/// * `_pad1`      — f32                   offset 92, size 4  → 96
///
/// Used by the forward-lit splat terrain pipeline (Phase 1.E, Option D).
/// Distinct from `CameraUniformsGpu` above, which is 80 B and matches the
/// dormant deferred-pipeline shader's `CameraUniforms` struct.
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraForwardGpu {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 3],
    pub _pad0: f32,
    pub camera_pos: [f32; 3],
    pub _pad1: f32,
}

impl Default for CameraForwardGpu {
    fn default() -> Self {
        Self {
            view_proj: [[0.0; 4]; 4],
            light_dir: [0.0, -1.0, 0.0],
            _pad0: 0.0,
            camera_pos: [0.0; 3],
            _pad1: 0.0,
        }
    }
}

/// CPU mirror of SHADER_SRC's `SceneEnv` struct at
/// `astraweave-render/src/renderer.rs:86-100`. **Must remain byte-identical
/// to that shader struct.** This mirrors SHADER_SRC's full field set
/// (Option 1 per the Phase 1.E handoff §1.E.1.a): `tint_color`, `tint_alpha`,
/// `blend_factor` are included even though Phase 1's forward shader does not
/// currently read them, so that future shader revisions adding screen tint
/// consume the correct bytes without a UBO redefinition.
///
/// Layout (96 bytes, align 16):
/// * `fog_color`        — vec3<f32>   offset 0,  size 12
/// * `fog_density`      — f32         offset 12, size 4  → 16
/// * `fog_start`        — f32         offset 16, size 4
/// * `fog_end`          — f32         offset 20, size 4
/// * `_pad0`            — vec2<f32>   offset 24, size 8  → 32
/// * `ambient_color`    — vec3<f32>   offset 32, size 12
/// * `ambient_intensity`— f32         offset 44, size 4  → 48
/// * `tint_color`       — vec3<f32>   offset 48, size 12
/// * `tint_alpha`       — f32         offset 60, size 4  → 64
/// * `blend_factor`     — f32         offset 64, size 4
/// * `_pad1`            — [f32; 3]    offset 68, size 12 → 80
/// * `sun_color`        — vec3<f32>   offset 80, size 12
/// * `sun_intensity`    — f32         offset 92, size 4  → 96
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TerrainSceneEnvGpu {
    pub fog_color: [f32; 3],
    pub fog_density: f32,
    pub fog_start: f32,
    pub fog_end: f32,
    pub _pad0: [f32; 2],
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub tint_color: [f32; 3],
    pub tint_alpha: f32,
    pub blend_factor: f32,
    pub _pad1: [f32; 3],
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
}

impl Default for TerrainSceneEnvGpu {
    fn default() -> Self {
        Self {
            fog_color: [0.5, 0.55, 0.6],
            fog_density: 0.0,
            fog_start: 500.0,
            fog_end: 2000.0,
            _pad0: [0.0; 2],
            ambient_color: [0.5, 0.55, 0.6],
            ambient_intensity: 0.35,
            tint_color: [1.0, 1.0, 1.0],
            tint_alpha: 0.0,
            blend_factor: 0.0,
            _pad1: [0.0; 3],
            sun_color: [1.0, 0.98, 0.92],
            sun_intensity: 1.5,
        }
    }
}

/// Shared texture arrays + sampler + uniform buffer living behind the pipeline.
struct SharedResources {
    layer_albedo: wgpu::TextureView,
    layer_normal: wgpu::TextureView,
    layer_orm: wgpu::TextureView,
    layer_height: wgpu::TextureView,
    _albedo_tex: wgpu::Texture,
    _normal_tex: wgpu::Texture,
    _orm_tex: wgpu::Texture,
    _height_tex: wgpu::Texture,
    sampler: wgpu::Sampler,
    material_uniform: wgpu::Buffer,
    camera_uniform: wgpu::Buffer,
}

/// Terrain splat-array material pipeline manager.
///
/// Owns the shared GPU resources and (lazily) the render pipeline; caches
/// per-chunk splat bind groups keyed by [`ChunkKey`]. The manager owns its
/// own camera uniform buffer + bind group so it can be driven standalone
/// without coupling to an external renderer's camera layout.
pub struct TerrainMaterialManager {
    config: TerrainMaterialConfig,
    shared: SharedResources,

    // Bind group layouts.
    camera_bgl: wgpu::BindGroupLayout,
    terrain_bgl: wgpu::BindGroupLayout,
    splat_bgl: wgpu::BindGroupLayout,

    /// Group 0 bind group (camera UBO; updated via `update_camera`).
    camera_bg: wgpu::BindGroup,
    /// Group 1 bind group (terrain uniform; single UBO; never changes per-draw).
    terrain_bg: wgpu::BindGroup,

    chunk_splats: HashMap<ChunkKey, ChunkSplat>,
    pipeline: Option<wgpu::RenderPipeline>,
    pipeline_formats: Option<(wgpu::TextureFormat, Option<wgpu::TextureFormat>)>,
    material_cache: TerrainMaterialGpu,
    camera_cache: CameraUniformsGpu,

    // Phase 1.E forward-pipeline state (Option D, Terrain Material System
    // Campaign). Additive to the fields above; the deferred pipeline
    // remains untouched. Built eagerly in `new`; `forward_pipeline` is
    // lazy and is populated on first call to `ensure_forward_pipeline`.
    forward_camera_bgl: wgpu::BindGroupLayout,
    forward_terrain_bgl: wgpu::BindGroupLayout,
    forward_splat_bgl: wgpu::BindGroupLayout,
    forward_camera_buffer: wgpu::Buffer,
    forward_scene_buffer: wgpu::Buffer,
    forward_camera_bg: wgpu::BindGroup,
    forward_terrain_bg: wgpu::BindGroup,
    forward_pipeline: Option<wgpu::RenderPipeline>,
    forward_pipeline_formats: Option<(wgpu::TextureFormat, Option<wgpu::TextureFormat>)>,
    forward_chunk_splats: HashMap<ChunkKey, ChunkSplatForward>,
}

/// Per-chunk splat bind group for the forward-lit pipeline.
///
/// Owns its own splat textures — kept separate from `ChunkSplat` above to
/// avoid coupling the forward and deferred paths. In Phase 1 only the
/// forward path is active; `chunk_splats` stays empty. A future
/// optimization could share underlying textures between the two paths.
struct ChunkSplatForward {
    _splat_0: wgpu::Texture,
    _splat_1: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)] // reserved for introspection/tests
    dims: (u32, u32),
}

impl TerrainMaterialManager {
    /// Create a new manager and allocate shared layer arrays + sampler.
    ///
    /// The manager internally creates its own camera bind group layout (group 0)
    /// matching `CameraUniforms` in `pbr_terrain.wgsl`.
    pub fn new(device: &wgpu::Device, config: TerrainMaterialConfig) -> Result<Self> {
        config
            .validate()
            .context("invalid TerrainMaterialConfig")?;

        let shared = create_shared_resources(device, &config);

        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain-splat-camera-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let terrain_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain-splat-uniform-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let splat_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain-splat-arrays-bgl"),
            entries: &[
                // 0: sampler (shared)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 1: splat_map_0 (per chunk)
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
                // 2: splat_map_1 (per chunk)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 3-6: layer arrays (albedo, normal, orm, height)
                array_entry(3),
                array_entry(4),
                array_entry(5),
                array_entry(6),
            ],
        });

        let terrain_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain-splat-uniform-bg"),
            layout: &terrain_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shared.material_uniform.as_entire_binding(),
            }],
        });

        let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain-splat-camera-bg"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shared.camera_uniform.as_entire_binding(),
            }],
        });

        // ── Phase 1.E forward-pipeline bind group layouts ───────────────
        // Group 0 (camera): one uniform binding holding CameraForwardGpu (96 B).
        let forward_camera_bgl = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("terrain-forward-camera-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            },
        );

        // Group 1 (terrain): TerrainMaterialGpu UBO + TerrainSceneEnvGpu UBO +
        // sampler + 3 layer arrays (albedo, normal, orm). 6 bindings under
        // the default 8-per-group limit.
        let forward_terrain_bgl = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("terrain-forward-terrain-bgl"),
                entries: &[
                    // 0: TerrainMaterialGpu UBO
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
                    // 1: TerrainSceneEnvGpu UBO
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 2: sampler (filtering — needed for smooth biome blending
                    // across splat textures and mipmap transitions in layer arrays)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                    // 3-5: layer arrays (albedo, normal, orm). Height is
                    // omitted — Phase 1's forward shader doesn't use it.
                    array_entry(3),
                    array_entry(4),
                    array_entry(5),
                ],
            },
        );

        // Group 2 (per-chunk splat): 2 texture bindings (splat_0, splat_1).
        let forward_splat_bgl = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("terrain-forward-splat-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            },
        );

        // Forward-path UBO buffers (zero-initialized; written per frame by
        // update_forward_camera / update_forward_scene).
        let forward_camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain-forward-camera-ubo"),
            size: std::mem::size_of::<CameraForwardGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let forward_scene_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain-forward-scene-ubo"),
            size: std::mem::size_of::<TerrainSceneEnvGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let forward_camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain-forward-camera-bg"),
            layout: &forward_camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: forward_camera_buffer.as_entire_binding(),
            }],
        });

        let forward_terrain_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain-forward-terrain-bg"),
            layout: &forward_terrain_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shared.material_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: forward_scene_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shared.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&shared.layer_albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&shared.layer_normal),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&shared.layer_orm),
                },
            ],
        });

        Ok(Self {
            config,
            shared,
            camera_bgl,
            terrain_bgl,
            splat_bgl,
            camera_bg,
            terrain_bg,
            chunk_splats: HashMap::new(),
            pipeline: None,
            pipeline_formats: None,
            material_cache: TerrainMaterialGpu::default(),
            camera_cache: CameraUniformsGpu::default(),
            forward_camera_bgl,
            forward_terrain_bgl,
            forward_splat_bgl,
            forward_camera_buffer,
            forward_scene_buffer,
            forward_camera_bg,
            forward_terrain_bg,
            forward_pipeline: None,
            forward_pipeline_formats: None,
            forward_chunk_splats: HashMap::new(),
        })
    }

    /// Expose the manager's active config.
    pub fn config(&self) -> TerrainMaterialConfig {
        self.config
    }

    /// How many chunks currently have splat maps registered.
    pub fn chunk_splat_count(&self) -> usize {
        self.chunk_splats.len()
    }

    /// Drop all per-chunk splat bind groups (e.g. on biome reload).
    pub fn clear_chunks(&mut self) {
        self.chunk_splats.clear();
    }

    /// Update the camera uniform buffer. Must be called at least once before
    /// the first draw.
    ///
    /// Only `view_proj` and `camera_pos` are written — the shader's
    /// `CameraUniforms` struct contains nothing else. Additional fields
    /// (view/forward/right) are accepted for API symmetry but currently
    /// ignored; they will be wired when the shader grows to need them.
    pub fn update_camera(
        &mut self,
        queue: &wgpu::Queue,
        view_proj: glam::Mat4,
        _view: glam::Mat4,
        camera_pos: glam::Vec3,
        _camera_forward: glam::Vec3,
        _camera_right: glam::Vec3,
    ) {
        self.camera_cache = CameraUniformsGpu {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.into(),
            _pad0: 0.0,
        };
        queue.write_buffer(
            &self.shared.camera_uniform,
            0,
            bytemuck::bytes_of(&self.camera_cache),
        );
    }

    /// Upload the eight layer textures and write the `TerrainMaterialGpu`
    /// uniform buffer.
    ///
    /// `layers` must have length `config.layer_count`; surplus entries are
    /// ignored, missing entries are padded with a solid-grey fallback.
    pub fn set_material(
        &mut self,
        queue: &wgpu::Queue,
        gpu_material: &TerrainMaterialGpu,
        layers: &[LayerTextures<'_>],
    ) -> Result<()> {
        // Validate each layer payload matches the expected byte size.
        let albedo_bytes = (self.config.albedo_resolution as usize).pow(2) * BYTES_PER_TEXEL as usize;
        let aux_bytes = (self.config.aux_resolution as usize).pow(2) * BYTES_PER_TEXEL as usize;

        let layer_count = self.config.layer_count as usize;
        for (i, layer) in layers.iter().take(layer_count).enumerate() {
            if let Some(data) = layer.albedo {
                if data.len() != albedo_bytes {
                    anyhow::bail!(
                        "layer {} albedo size mismatch: expected {} bytes, got {}",
                        i,
                        albedo_bytes,
                        data.len()
                    );
                }
            }
            for (name, data) in [
                ("normal", layer.normal),
                ("orm", layer.orm),
                ("height", layer.height),
            ] {
                if let Some(data) = data {
                    if data.len() != aux_bytes {
                        anyhow::bail!(
                            "layer {} {} size mismatch: expected {} bytes, got {}",
                            i,
                            name,
                            aux_bytes,
                            data.len()
                        );
                    }
                }
            }
        }

        // Default grey / flat-normal / neutral-ORM payloads for missing channels.
        let grey_albedo = vec![128u8; albedo_bytes];
        let flat_normal = build_flat_normal(self.config.aux_resolution);
        let neutral_orm = build_neutral_orm(self.config.aux_resolution);
        let flat_height = vec![128u8; aux_bytes];

        for i in 0..layer_count {
            let payload = layers.get(i).cloned().unwrap_or_default();
            upload_layer_slice(
                queue,
                &self.shared._albedo_tex,
                i as u32,
                self.config.albedo_resolution,
                payload.albedo.unwrap_or(&grey_albedo),
            );
            upload_layer_slice(
                queue,
                &self.shared._normal_tex,
                i as u32,
                self.config.aux_resolution,
                payload.normal.unwrap_or(&flat_normal),
            );
            upload_layer_slice(
                queue,
                &self.shared._orm_tex,
                i as u32,
                self.config.aux_resolution,
                payload.orm.unwrap_or(&neutral_orm),
            );
            upload_layer_slice(
                queue,
                &self.shared._height_tex,
                i as u32,
                self.config.aux_resolution,
                payload.height.unwrap_or(&flat_height),
            );
        }

        // Write the uniform buffer (576 B).
        self.material_cache = *gpu_material;
        self.material_cache.active_layer_count = self.config.layer_count;
        queue.write_buffer(
            &self.shared.material_uniform,
            0,
            bytemuck::bytes_of(&self.material_cache),
        );

        Ok(())
    }

    /// Register or replace the two splat maps for a chunk.
    ///
    /// `splat_0` and `splat_1` must each be `dims.0 * dims.1 * 4` bytes (RGBA8).
    /// Channel mapping matches `pbr_terrain.wgsl`:
    /// * `splat_0`: R=layer0, G=layer1, B=layer2, A=layer3
    /// * `splat_1`: R=layer4, G=layer5, B=layer6, A=layer7
    pub fn set_chunk_splat(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk: ChunkKey,
        splat_0: &[u8],
        splat_1: &[u8],
        dims: (u32, u32),
    ) -> Result<()> {
        let (w, h) = dims;
        if w == 0 || h == 0 {
            anyhow::bail!("chunk splat dims must be non-zero, got {w}x{h}");
        }
        let expected = (w as usize) * (h as usize) * BYTES_PER_TEXEL as usize;
        if splat_0.len() != expected || splat_1.len() != expected {
            anyhow::bail!(
                "chunk splat payload mismatch: expected {} bytes, got {} / {}",
                expected,
                splat_0.len(),
                splat_1.len()
            );
        }

        let tex_desc = wgpu::TextureDescriptor {
            label: Some("terrain-chunk-splat"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let tex_0 = device.create_texture(&tex_desc);
        let tex_1 = device.create_texture(&tex_desc);
        upload_full_2d(queue, &tex_0, w, h, splat_0);
        upload_full_2d(queue, &tex_1, w, h, splat_1);

        let view_0 = tex_0.create_view(&wgpu::TextureViewDescriptor::default());
        let view_1 = tex_1.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain-splat-chunk-bg"),
            layout: &self.splat_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.shared.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_0),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view_1),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.shared.layer_albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.shared.layer_normal),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.shared.layer_orm),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.shared.layer_height),
                },
            ],
        });

        self.chunk_splats.insert(
            chunk,
            ChunkSplat {
                _splat_0: tex_0,
                _splat_1: tex_1,
                bind_group,
                dims,
            },
        );
        Ok(())
    }

    /// Remove a single chunk's splat pair (e.g. when the chunk is unloaded).
    pub fn remove_chunk_splat(&mut self, chunk: ChunkKey) -> bool {
        self.chunk_splats.remove(&chunk).is_some()
    }

    /// Return the dimensions of the splat maps registered for a chunk, if any.
    pub fn chunk_splat_dims(&self, chunk: ChunkKey) -> Option<(u32, u32)> {
        self.chunk_splats.get(&chunk).map(|s| s.dims)
    }

    /// Lazily create the render pipeline for the given target formats.
    ///
    /// Re-creates the pipeline when the requested formats differ from the
    /// previously cached pair.
    pub fn ensure_pipeline(
        &mut self,
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> &wgpu::RenderPipeline {
        if self.pipeline_formats == Some((color_format, depth_format)) {
            return self
                .pipeline
                .as_ref()
                .expect("pipeline populated when formats match");
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain-splat-shader"),
            source: wgpu::ShaderSource::Wgsl(TERRAIN_SPLAT_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain-splat-pipeline-layout"),
            bind_group_layouts: &[&self.camera_bgl, &self.terrain_bgl, &self.splat_bgl],
            push_constant_ranges: &[],
        });

        let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain-splat-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[TerrainSplatVertex::LAYOUT],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // @location(1) normal
                    Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // @location(2) orm
                    Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        self.pipeline = Some(pipeline);
        self.pipeline_formats = Some((color_format, depth_format));
        self.pipeline.as_ref().expect("pipeline just populated")
    }

    /// Lazily build the Phase 1 forward-lit splat pipeline (Option D).
    ///
    /// Unlike [`Self::ensure_pipeline`] (which builds the dormant deferred
    /// pipeline with three g-buffer color targets), this builds a pipeline
    /// that writes a single lit HDR color to `@location(0)`, compatible with
    /// the engine's forward `hdr_view` attachment (Rgba16Float).
    ///
    /// The pipeline is cached by `(color_format, depth_format)`; calling
    /// with the same formats is a no-op. Called from `Renderer::draw_into`
    /// once the renderer knows its HDR + depth formats.
    pub fn ensure_forward_pipeline(
        &mut self,
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> &wgpu::RenderPipeline {
        if self.forward_pipeline_formats == Some((color_format, depth_format))
            && self.forward_pipeline.is_some()
        {
            return self
                .forward_pipeline
                .as_ref()
                .expect("forward_pipeline populated when formats match");
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain-forward-shader"),
            source: wgpu::ShaderSource::Wgsl(TERRAIN_FORWARD_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain-forward-pipeline-layout"),
            bind_group_layouts: &[
                &self.forward_camera_bgl,
                &self.forward_terrain_bgl,
                &self.forward_splat_bgl,
            ],
            push_constant_ranges: &[],
        });

        let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain-forward-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[TerrainSplatVertex::LAYOUT],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    // REPLACE: terrain is opaque and writes over whatever
                    // geometry has already been drawn to this pixel.
                    // Depth test gates occlusion correctness.
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        self.forward_pipeline = Some(pipeline);
        self.forward_pipeline_formats = Some((color_format, depth_format));
        self.forward_pipeline
            .as_ref()
            .expect("forward_pipeline just populated")
    }

    /// Write the Phase 1 forward-path camera UBO (96 B, matches SHADER_SRC Camera).
    ///
    /// Call once per frame from `Renderer::draw_into`, before issuing any
    /// `draw_chunk_forward` calls. The UBO persists across calls — the shader
    /// reads the most recently written values.
    pub fn update_forward_camera(
        &self,
        queue: &wgpu::Queue,
        view_proj: glam::Mat4,
        light_dir: glam::Vec3,
        camera_pos: glam::Vec3,
    ) {
        let gpu = CameraForwardGpu {
            view_proj: view_proj.to_cols_array_2d(),
            light_dir: light_dir.to_array(),
            _pad0: 0.0,
            camera_pos: camera_pos.to_array(),
            _pad1: 0.0,
        };
        queue.write_buffer(
            &self.forward_camera_buffer,
            0,
            bytemuck::bytes_of(&gpu),
        );
    }

    /// Write the Phase 1 forward-path scene-env UBO (96 B, matches SHADER_SRC SceneEnv).
    ///
    /// Call once per frame from `Renderer::draw_into`. The caller is
    /// responsible for composing `TerrainSceneEnvGpu` from whatever the
    /// engine's live scene_env state is — see the corresponding helper in
    /// `Renderer` (added in 1.E.3).
    pub fn update_forward_scene(&self, queue: &wgpu::Queue, scene_env: &TerrainSceneEnvGpu) {
        queue.write_buffer(
            &self.forward_scene_buffer,
            0,
            bytemuck::bytes_of(scene_env),
        );
    }

    /// Upload per-chunk splat textures and build the forward-path bind group.
    ///
    /// Called by `Renderer::upload_terrain_chunk` (Phase 1.E.3). The two
    /// RGBA8 buffers are rasterized from the editor's per-vertex biome
    /// weights by `terrain_splat_builder::build_chunk_splat_maps`.
    ///
    /// `splat_0`: R=biome0 weight, G=biome1 weight, B=biome2, A=biome3.
    /// `splat_1`: R=biome4, G=biome5, B=biome6, A=biome7.
    pub fn set_chunk_splat_forward(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk: ChunkKey,
        splat_0: &[u8],
        splat_1: &[u8],
        dims: (u32, u32),
    ) -> Result<()> {
        let (w, h) = dims;
        if w == 0 || h == 0 {
            anyhow::bail!("forward chunk splat dims must be non-zero, got {w}x{h}");
        }
        let expected = (w as usize) * (h as usize) * BYTES_PER_TEXEL as usize;
        if splat_0.len() != expected || splat_1.len() != expected {
            anyhow::bail!(
                "forward chunk splat payload mismatch: expected {} bytes, got {} / {}",
                expected,
                splat_0.len(),
                splat_1.len()
            );
        }

        let tex_desc = wgpu::TextureDescriptor {
            label: Some("terrain-forward-chunk-splat"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let tex_0 = device.create_texture(&tex_desc);
        let tex_1 = device.create_texture(&tex_desc);
        upload_full_2d(queue, &tex_0, w, h, splat_0);
        upload_full_2d(queue, &tex_1, w, h, splat_1);

        let view_0 = tex_0.create_view(&wgpu::TextureViewDescriptor::default());
        let view_1 = tex_1.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain-forward-chunk-splat-bg"),
            layout: &self.forward_splat_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view_0),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_1),
                },
            ],
        });

        self.forward_chunk_splats.insert(
            chunk,
            ChunkSplatForward {
                _splat_0: tex_0,
                _splat_1: tex_1,
                bind_group,
                dims,
            },
        );
        Ok(())
    }

    /// Number of chunks currently registered in the forward path.
    pub fn forward_chunk_count(&self) -> usize {
        self.forward_chunk_splats.len()
    }

    /// Drop every per-chunk forward splat (e.g. on terrain reload).
    pub fn clear_forward_chunks(&mut self) {
        self.forward_chunk_splats.clear();
    }

    /// Issue a draw call for a chunk's forward-lit splat render.
    ///
    /// Prerequisites (caller must have satisfied):
    /// 1. [`Self::ensure_forward_pipeline`] was called with a compatible format.
    /// 2. [`Self::update_forward_camera`] and [`Self::update_forward_scene`]
    ///    were called earlier this frame (or sensible defaults are in the UBOs).
    /// 3. [`Self::set_chunk_splat_forward`] was called for this chunk key.
    ///
    /// Returns `false` when any prerequisite is missing; the caller can fall
    /// through to the legacy path without further state changes. Returns
    /// `true` when the draw was issued. The render pass itself must be
    /// configured for the `color_format` + `depth_format` passed to
    /// `ensure_forward_pipeline` — a mismatch is a wgpu validation error.
    pub fn draw_chunk_forward<'a>(
        &'a self,
        rpass: &mut wgpu::RenderPass<'a>,
        chunk: ChunkKey,
        vertex_buffer: &'a wgpu::Buffer,
        index_buffer: &'a wgpu::Buffer,
        index_count: u32,
    ) -> bool {
        let Some(pipeline) = self.forward_pipeline.as_ref() else {
            return false;
        };
        let Some(splat) = self.forward_chunk_splats.get(&chunk) else {
            return false;
        };
        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, &self.forward_camera_bg, &[]);
        rpass.set_bind_group(1, &self.forward_terrain_bg, &[]);
        rpass.set_bind_group(2, &splat.bind_group, &[]);
        rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
        rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..index_count, 0, 0..1);
        true
    }

    /// Issue a draw call for a chunk that has had its splats uploaded.
    ///
    /// Uses the manager's internal camera bind group (see
    /// [`Self::update_camera`]). Returns `false` when the chunk has no
    /// registered splat pair (caller should fall back to the legacy path).
    pub fn draw_chunk<'a>(
        &'a self,
        rpass: &mut wgpu::RenderPass<'a>,
        chunk: ChunkKey,
        vertex_buffer: &'a wgpu::Buffer,
        index_buffer: &'a wgpu::Buffer,
        index_count: u32,
    ) -> bool {
        let Some(pipeline) = self.pipeline.as_ref() else {
            return false;
        };
        let Some(splat) = self.chunk_splats.get(&chunk) else {
            return false;
        };
        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, &self.camera_bg, &[]);
        rpass.set_bind_group(1, &self.terrain_bg, &[]);
        rpass.set_bind_group(2, &splat.bind_group, &[]);
        rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
        rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..index_count, 0, 0..1);
        true
    }
}

// ── Private helpers ─────────────────────────────────────────────────────────

fn array_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2Array,
            multisampled: false,
        },
        count: None,
    }
}

fn create_shared_resources(
    device: &wgpu::Device,
    config: &TerrainMaterialConfig,
) -> SharedResources {
    let albedo_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("terrain-layer-albedo-array"),
        size: wgpu::Extent3d {
            width: config.albedo_resolution,
            height: config.albedo_resolution,
            depth_or_array_layers: config.layer_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let normal_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("terrain-layer-normal-array"),
        size: wgpu::Extent3d {
            width: config.aux_resolution,
            height: config.aux_resolution,
            depth_or_array_layers: config.layer_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let orm_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("terrain-layer-orm-array"),
        size: wgpu::Extent3d {
            width: config.aux_resolution,
            height: config.aux_resolution,
            depth_or_array_layers: config.layer_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let height_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("terrain-layer-height-array"),
        size: wgpu::Extent3d {
            width: config.aux_resolution,
            height: config.aux_resolution,
            depth_or_array_layers: config.layer_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let view_desc = wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    };
    let layer_albedo = albedo_tex.create_view(&view_desc);
    let layer_normal = normal_tex.create_view(&view_desc);
    let layer_orm = orm_tex.create_view(&view_desc);
    let layer_height = height_tex.create_view(&view_desc);

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("terrain-splat-sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        anisotropy_clamp: 1,
        ..Default::default()
    });

    let material_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("terrain-splat-material-ubo"),
        contents: bytemuck::bytes_of(&TerrainMaterialGpu::default()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let camera_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("terrain-splat-camera-ubo"),
        contents: bytemuck::bytes_of(&CameraUniformsGpu::default()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    SharedResources {
        layer_albedo,
        layer_normal,
        layer_orm,
        layer_height,
        _albedo_tex: albedo_tex,
        _normal_tex: normal_tex,
        _orm_tex: orm_tex,
        _height_tex: height_tex,
        sampler,
        material_uniform,
        camera_uniform,
    }
}

fn upload_layer_slice(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    layer: u32,
    resolution: u32,
    data: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: 0,
                y: 0,
                z: layer,
            },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(resolution * BYTES_PER_TEXEL),
            rows_per_image: Some(resolution),
        },
        wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: 1,
        },
    );
}

fn upload_full_2d(queue: &wgpu::Queue, texture: &wgpu::Texture, w: u32, h: u32, data: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * BYTES_PER_TEXEL),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
}

fn build_flat_normal(resolution: u32) -> Vec<u8> {
    // (0,0,1) encoded as (128, 128, 255) in unsigned 8-bit.
    let n = (resolution as usize).pow(2);
    let mut v = Vec::with_capacity(n * 4);
    for _ in 0..n {
        v.extend_from_slice(&[128, 128, 255, 255]);
    }
    v
}

fn build_neutral_orm(resolution: u32) -> Vec<u8> {
    // ORM: R=AO(1.0), G=Roughness(0.5), B=Metallic(0.0), A=1.0
    let n = (resolution as usize).pow(2);
    let mut v = Vec::with_capacity(n * 4);
    for _ in 0..n {
        v.extend_from_slice(&[255, 128, 0, 255]);
    }
    v
}

// ── Pure (CPU-only) unit tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Phase 1.E.1.a — byte-layout invariants for the forward-path UBO types.
    // A drift from SHADER_SRC's `Camera` / `SceneEnv` structs would produce
    // a shader that compiles but reads garbage from UBO memory. Catch any
    // such drift at test time rather than at render time. See the Phase
    // 1.E handoff §5 "Camera UBO byte layout".

    #[test]
    fn camera_forward_gpu_is_96_bytes_align_16() {
        assert_eq!(std::mem::size_of::<CameraForwardGpu>(), 96);
        assert_eq!(std::mem::align_of::<CameraForwardGpu>(), 16);
    }

    #[test]
    fn terrain_scene_env_gpu_is_96_bytes_align_16() {
        assert_eq!(std::mem::size_of::<TerrainSceneEnvGpu>(), 96);
        assert_eq!(std::mem::align_of::<TerrainSceneEnvGpu>(), 16);
    }

    #[test]
    fn camera_forward_gpu_field_offsets_match_shader_src() {
        use std::mem::offset_of;
        assert_eq!(offset_of!(CameraForwardGpu, view_proj), 0);
        assert_eq!(offset_of!(CameraForwardGpu, light_dir), 64);
        assert_eq!(offset_of!(CameraForwardGpu, _pad0), 76);
        assert_eq!(offset_of!(CameraForwardGpu, camera_pos), 80);
        assert_eq!(offset_of!(CameraForwardGpu, _pad1), 92);
    }

    #[test]
    fn terrain_scene_env_gpu_matches_engine_scene_env_ubo_size() {
        // Renderer::draw_into casts `SceneEnvironmentUBO` (the engine's
        // live scene env) into `TerrainSceneEnvGpu` via `bytemuck::cast`.
        // That cast compiles only when both types are the same size. If
        // this test fails, the cast in `renderer.rs:draw_into` will fail
        // to compile too — but failing here makes the reason obvious.
        assert_eq!(
            std::mem::size_of::<TerrainSceneEnvGpu>(),
            std::mem::size_of::<crate::scene_environment::SceneEnvironmentUBO>(),
            "TerrainSceneEnvGpu must match SceneEnvironmentUBO byte size so \
             Renderer::draw_into can bytemuck::cast between them",
        );
    }

    #[test]
    fn terrain_scene_env_gpu_field_offsets_match_shader_src() {
        use std::mem::offset_of;
        assert_eq!(offset_of!(TerrainSceneEnvGpu, fog_color), 0);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, fog_density), 12);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, fog_start), 16);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, fog_end), 20);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, _pad0), 24);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, ambient_color), 32);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, ambient_intensity), 44);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, tint_color), 48);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, tint_alpha), 60);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, blend_factor), 64);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, _pad1), 68);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, sun_color), 80);
        assert_eq!(offset_of!(TerrainSceneEnvGpu, sun_intensity), 92);
    }

    #[test]
    fn config_validates_power_of_two() {
        let mut cfg = TerrainMaterialConfig::default();
        cfg.validate().unwrap();

        cfg.albedo_resolution = 1000; // not power of two
        assert!(cfg.validate().is_err());

        cfg.albedo_resolution = 1024;
        cfg.aux_resolution = 0;
        assert!(cfg.validate().is_err());

        cfg.aux_resolution = 512;
        cfg.layer_count = 0;
        assert!(cfg.validate().is_err());

        cfg.layer_count = 9;
        assert!(cfg.validate().is_err());

        cfg.layer_count = MAX_TERRAIN_LAYERS;
        cfg.validate().unwrap();
    }

    #[test]
    fn memory_estimate_matches_formula() {
        let cfg = TerrainMaterialConfig {
            albedo_resolution: 1024,
            aux_resolution: 512,
            layer_count: 8,
        };
        // 1024² × 4 = 4 MiB per albedo layer
        // 512² × 4 = 1 MiB per aux layer × 3 channels = 3 MiB
        // (4 + 3) MiB × 8 = 56 MiB
        assert_eq!(cfg.approx_memory_bytes(), 56 * 1024 * 1024);
    }

    #[test]
    fn flat_normal_encodes_plus_z() {
        let data = build_flat_normal(4);
        assert_eq!(data.len(), 4 * 4 * 4);
        // Every texel should be (128, 128, 255, 255).
        for chunk in data.chunks_exact(4) {
            assert_eq!(chunk, &[128, 128, 255, 255]);
        }
    }

    #[test]
    fn neutral_orm_packs_ao_rough_metallic() {
        let data = build_neutral_orm(4);
        assert_eq!(data.len(), 4 * 4 * 4);
        for chunk in data.chunks_exact(4) {
            assert_eq!(chunk, &[255, 128, 0, 255]);
        }
    }

    #[test]
    fn terrain_splat_vertex_layout_matches_struct() {
        assert_eq!(
            TerrainSplatVertex::LAYOUT.array_stride,
            std::mem::size_of::<TerrainSplatVertex>() as wgpu::BufferAddress
        );
        assert_eq!(
            std::mem::size_of::<TerrainSplatVertex>(),
            std::mem::size_of::<[f32; 8]>()
        );
    }

    #[test]
    fn shader_source_includes_vertex_and_fragment_entrypoints() {
        assert!(TERRAIN_SPLAT_SHADER.contains("fn vs_main("));
        assert!(TERRAIN_SPLAT_SHADER.contains("fn fs_main("));
        // Sanity: the fragment stage must declare VertexOutput before vs_main
        // tries to return one; `VertexOutput` lives in pbr_terrain.wgsl which
        // is concatenated first.
        let fs_pos = TERRAIN_SPLAT_SHADER
            .find("struct VertexOutput")
            .expect("VertexOutput struct declaration present");
        let vs_pos = TERRAIN_SPLAT_SHADER
            .find("fn vs_main(")
            .expect("vs_main entrypoint present");
        assert!(
            fs_pos < vs_pos,
            "VertexOutput must be declared before vs_main"
        );
    }

    #[test]
    fn shader_source_parses_with_naga() {
        // Lock the shader module against future regressions: any WGSL syntax
        // error here will fail the fast-path test suite instead of only
        // surfacing when a wgpu device is available.
        let module = naga::front::wgsl::parse_str(TERRAIN_SPLAT_SHADER)
            .expect("terrain splat shader must parse");
        assert!(
            module.entry_points.iter().any(|ep| ep.name == "vs_main"),
            "vs_main entry point present"
        );
        assert!(
            module.entry_points.iter().any(|ep| ep.name == "fs_main"),
            "fs_main entry point present"
        );
    }
}
