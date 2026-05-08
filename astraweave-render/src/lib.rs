#![deny(unsafe_code)]
//! # AstraWeave Render
//!
//! GPU rendering pipeline for AstraWeave, built on **wgpu 25**.
//!
//! This crate provides a complete rendering solution including:
//!
//! - **Core**: [`Renderer`], [`Camera`], [`CameraController`], [`Texture`], [`Vertex`], [`Mesh`]
//! - **Materials**: PBR material system with TOML-driven asset pipeline
//!   ([`MaterialManager`], [`MaterialGpuExtended`] with clearcoat, anisotropy, SSS)
//! - **Lighting**: Clustered forward rendering, MegaLights GPU culling, CSM shadows
//! - **Post-Processing**: Bloom, tonemapping, TAA, motion blur, DoF, SSAO, color grading
//! - **Animation**: GPU skinning ([`Skeleton`], [`AnimationClip`], [`JointPalette`])
//! - **Mesh Optimization**: Vertex compression (37.5% memory reduction), LOD generation, instancing
//! - **Environment**: Sky rendering, day/night cycle, weather system, water (Gerstner waves)
//! - **Advanced**: Deferred rendering ([`GBuffer`]), decals, GPU particles, biome materials
//! - **Streaming**: Texture streaming, GPU memory residency management
//!
//! # Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `postfx` | Post-processing effects (default) |
//! | `textures` | Texture loading (default) |
//! | `bloom` | HDR bloom pipeline |
//! | `ibl` | Image-based lighting |
//! | `megalights` | GPU-accelerated light culling |
//! | `deferred` | Deferred rendering path |
//! | `gpu-particles` | GPU particle system |
//! | `decals` | Projected decal system |
//! | `advanced-post` | TAA, motion blur, DoF, color grading |
//! | `ssao` | Screen-space ambient occlusion |

pub mod camera;
pub mod clustered;
pub mod clustered_forward; // Complete clustered forward rendering
pub mod clustered_megalights; // MegaLights: GPU-accelerated light culling (Phase 1)
pub mod compute_noise; // GPU compute noise generation (Perlin/fBM/Ridged/Billow/DomainWarped)
pub mod debug_quad;
pub mod depth;
pub mod environment;
pub mod error; // Typed error types for the rendering pipeline
pub mod gi; // Global Illumination (VXGI)
pub mod ibl; // image-based lighting manager
pub mod mesh; // cpu mesh structures + utils
pub mod mesh_registry;
pub mod post; // compile-only WGSL placeholders & tests
pub mod primitives;
pub mod renderer;
pub mod shadow_csm; // Cascaded Shadow Mapping (Phase 2)
pub mod shadow_point; // Point/spot light shadow maps with priority selection
pub mod shadow_quality; // Shadow quality: PCSS, Poisson PCF, cascade stabilization, normal-offset bias
pub mod terrain;
pub mod texture;
pub mod types; // clustered-lighting WGSL placeholders & tests // gpu upload & caching
               // See MATERIALS.md for canonical materials arrays and WGSL bindings
pub mod animation;
pub mod asset_index;
pub mod atmosphere; // Bruneton physically-based atmosphere (Phase 8)
pub mod auto_exposure; // Auto-exposure via luminance histogram with temporal adaptation
pub mod biome_audio;
pub mod biome_detector;
pub mod biome_material;
pub mod biome_transition;
pub mod bloom; // Physically-based bloom with 13-tap downsample and tent upsample
pub mod brdf_lut; // BRDF integration LUT for split-sum IBL (Phase 9)
pub mod culling; // GPU-driven frustum culling (Phase 2 Task 3)
pub mod culling_node; // Culling node for render graph
pub mod disney_material; // Disney principled BRDF evaluation + WGSL source (Phase 9)
pub mod distance_field; // Signed Distance Field generation + Distance-Field AO (Lumen Phase 5)
pub mod final_gather; // Lumen final gather: multi-bounce diffuse indirect compositor
pub mod frame_graph; // Frame render graph: DAG-based pass pipeline builder
pub mod god_rays; // Screen-space god rays / crepuscular light shafts (Phase 6)
pub mod graph; // minimal render graph scaffolding (Phase 2)
pub mod graph_adapter; // runs a graph on Renderer frames
pub mod gtao; // Ground Truth Ambient Occlusion with visibility bitmask
pub mod hdr_pipeline; // HDR rendering pipeline orchestration (tonemap, color grading, post-FX chain)
pub mod hdri_catalog;
pub mod hiz_pyramid; // Shared Hi-Z min-depth pyramid for SSR/SSGI ray acceleration
pub mod lumen; // Lumen Global Illumination orchestrator (Phase 5)
pub mod material; // shared authored materials API + GPU arrays
pub mod material_extended; // Phase PBR-E: Advanced materials (clearcoat, anisotropy, SSS, sheen, transmission)
pub mod material_library; // Real-Fix.D 2026-05-08: canonical terrain material library (UI/renderer single source of truth)
#[cfg(feature = "textures")]
pub mod material_loader; // internal builder helpers
#[cfg(any(feature = "gltf-assets", feature = "assets"))]
pub mod mesh_gltf; // glTF loader
#[cfg(any(feature = "obj-assets", feature = "assets"))]
pub mod mesh_obj;
pub mod parallax; // Parallax Occlusion Mapping — steep ray-march + binary refinement
pub mod particle_forces; // Enhanced particle simulation: forces, curves, emission shapes (Phase 7)
pub mod particle_render; // Billboard particle render pipeline with blending (Phase 7)
pub mod particle_sort; // GPU bitonic sort for depth-ordered particle transparency (Phase 7)
pub mod pipeline_cache; // Disk-backed Vulkan/DX12 pipeline cache (eliminates cold-start stalls)
pub mod residency;
pub mod scene_environment;
pub mod shader_manager; // Shader hot-reload: hash-based change detection + pipeline invalidation
pub mod shader_permutation; // Compile-time permutation system for Disney BRDF lobes
pub mod ssgi; // Screen-Space Global Illumination with temporal denoise
pub mod ssr; // Screen-Space Reflections with Hi-Z ray marching
pub mod stochastic_tiling; // Hex-tile stochastic sampling to break terrain texture repetition
pub mod subgroup_ops; // Subgroup-optimized shader variants (auto-exposure, prefix sum, bitonic sort)
pub mod surface_cache; // World-space irradiance probe grid (Lumen Phase 5)
pub mod taa; // Temporal Anti-Aliasing with neighborhood clamping and RCAS sharpening
pub mod temporal_upscale; // Temporal upscaling (TAA-U) — render at reduced internal res, resolve to native
pub mod terrain_gpu_bridge; // Render-side impl of TerrainGpuAccelerator (GPU noise + erosion bridge)
pub mod terrain_material;
#[cfg(feature = "terrain-splat-arrays")]
pub mod terrain_material_manager; // Phase 2.2: 8-layer splat-array terrain pipeline (Issue #9 fix)
pub mod texture_streaming;
pub mod velocity; // Motion vector / velocity buffer for temporal effects (TAA, motion blur, TSR)
pub mod virtual_texture; // Sparse Virtual Texturing: tile-based page streaming with feedback + LRU cache
pub mod volumetric_clouds; // Perlin-Worley volumetric cloud raymarching (Phase 3)
pub mod volumetric_fog; // Froxel-based volumetric fog + light scattering (Phase 6)
pub mod weather_system; // Texture streaming with LRU cache and priority-based loading // Phase PBR-F: Terrain layering with splat maps and triplanar projection // asset streaming and residency management // OBJ fallback loader // Phase 2 Task 5: Skeletal animation with CPU/GPU skinning

#[cfg(feature = "skinning-gpu")]
pub mod skinning_gpu; // Phase 2 Task 5 Phase D: GPU skinning pipeline

pub mod instancing;
pub mod lod_generator; // Week 5 Action 19: LOD generation with quadric error metrics
pub mod ltc_area_lights; // LTC area lights: rectangular, disk, tube area lights (Heitz et al. 2016)
pub mod vertex_compression; // Week 5 Action 19: Vertex compression // Week 5 Action 19: GPU instancing for draw call reduction (octahedral normals, half-float UVs)

#[cfg(test)]
mod animation_extra_tests; // Phase 7: Additional animation tests

#[cfg(test)]
mod mutation_tests; // Phase 10B: Comprehensive mutation-killing tests

// Nanite virtualized geometry system
#[cfg(feature = "nanite")]
pub mod nanite_gpu_culling;
#[cfg(feature = "nanite")]
pub mod nanite_render;
#[cfg(feature = "nanite")]
pub mod nanite_visibility; // NEW: GPU-driven culling and visibility

pub use camera::{Camera, CameraController};
pub use environment::{
    SkyConfig, SkyRenderer, TimeOfDay, WeatherParticles, WeatherSystem, WeatherType,
};
pub use error::{RenderError, RenderResult};
pub use renderer::{ModelSurfaceMaps, Renderer};
pub use terrain::{TerrainMesh, TerrainRenderer, TerrainVertex, VegetationRenderInstance};
pub use texture::Texture;
pub use types::{Instance, Material, Mesh, SkinnedVertex, Vertex};

pub mod water; // Animated ocean with Gerstner waves
pub use water::WaterRenderer;

pub mod advanced_post;
pub mod decals; // Screen-space decal system
pub mod deferred; // Deferred rendering pipeline
pub mod effects; // NEW
pub mod gpu_erosion;
pub mod gpu_particles; // GPU compute-based particle system
pub mod grass_blade; // Procedural per-blade grass geometry rendering
#[cfg(feature = "impostor-bake")]
pub mod impostor_bake; // Phase 5.3: offline/lazy impostor atlas bake pipeline
#[cfg(feature = "impostor-bake")]
pub mod impostor_lod3; // Phase 5.3 T4: LOD3 live draw sampling pipeline
#[cfg(feature = "impostor-bake")]
pub mod impostor_pass; // Phase 5.3 T7 (stage 1): reusable LOD3 draw helper
pub mod material_bindless; // Bindless texture array material system
pub mod msaa; // MSAA anti-aliasing resources
pub mod oit; // Weighted Blended Order-Independent Transparency
pub mod overlay; // NEW (for cutscene fades/letterbox later)
pub mod puddle_accumulation; // Rain-driven puddle formation in terrain concavities
pub mod rain_occlusion; // GPU rain/weather particle occlusion via depth buffer
pub mod rain_splash; // Rain impact splash particle spawner
pub mod snow_accumulation; // Per-chunk snow accumulation compute + snow material blending
pub mod snow_footprint; // Entity footprint depression in accumulated snow
pub mod transparency; // Transparency depth sorting and render pass // Advanced post-processing (TAA, motion blur, DOF, color grading) // GPU compute SWE erosion
pub mod vegetation_gpu; // GPU-instanced vegetation scatter and frustum cull pipeline
pub mod vegetation_interaction; // Entity proximity grass bending stamp system
pub mod vegetation_lod; // Tree LOD chain with billboard/impostor support
pub mod weather_gpu; // GPU-accelerated weather particle emitter configurations

// GPU memory management
pub mod bind_group_cache; // Generation-tracked bind group cache
pub mod gpu_memory; // GPU memory budget tracking and enforcement
pub mod gpu_profiler; // GPU timestamp profiler for per-pass performance analysis
pub mod staging_ring; // Per-frame ring buffer for transient GPU allocations

pub use advanced_post::{AdvancedPostFx, ColorGradingConfig, DofConfig, MotionBlurConfig};
pub use asset_index::{AssetIndex, HdriRef as AssetHdriRef, MaterialSetEntry, TextureEntry};
pub use atmosphere::{AtmosphereConfig, AtmospherePass};
pub use bind_group_cache::{CachedBindGroup, CachedBindGroupSet, Generation};
pub use biome_detector::{BiomeDetector, BiomeDetectorConfig, BiomeTransition};
pub use biome_material::{BiomeMaterialConfig, BiomeMaterialSystem};
pub use biome_transition::{BiomeVisuals, EasingFunction, TransitionConfig, TransitionEffect};
pub use brdf_lut::{BrdfLutConfig, BrdfLutPass};
pub use compute_noise::{GpuNoiseConfig, GpuNoisePipeline, GpuNoiseType};
pub use culling::{
    batch_visible_instances, build_indirect_commands_cpu, cpu_frustum_cull,
    dispatch_indexed_indirect_draws, dispatch_multi_draw_indexed_indirect, BatchId,
    CullingPipeline, CullingResources, DrawBatch, DrawIndexedIndirectCommand, DrawIndirectCommand,
    FrustumPlanes, IndirectDrawPipeline, IndirectDrawResources, InstanceAABB,
};
pub use culling_node::{CullingNode, IndirectCullingNode};
pub use decals::{Decal, DecalAtlas, DecalBlendMode, DecalSystem, GpuDecal, DECAL_SHADER};
pub use deferred::{DeferredRenderer, GBuffer, GBufferFormats};
pub use disney_material::{evaluate_disney_brdf, BrdfResult, BRDF_LUT_WGSL, DISNEY_BRDF_WGSL};
pub use distance_field::{DfaoConfig, DfaoParams, DfaoPass, SdfBox, SdfConfig, SdfVolume};
pub use effects::{WeatherFx, WeatherKind};
pub use final_gather::{FinalGatherConfig, FinalGatherParams, FinalGatherPass};
pub use god_rays::{sun_to_screen, GodRayConfig, GodRayParams, GodRayPass};
pub use gpu_erosion::{ErosionPreset, GpuErosionConfig, GpuErosionPipeline};
pub use gpu_memory::{GpuMemoryBudget, MemoryCategory};
pub use gpu_particles::{EmitterParams, GpuParticle, GpuParticleSystem};
pub use gpu_profiler::{GpuProfiler, PassTiming};
pub use hdri_catalog::{DayPeriod, HdriCatalog, HdriEntry};
pub use ibl::{IblManager, IblQuality, IblResources, SkyMode};
pub use ltc_area_lights::{AreaLight, AreaLightManager, AreaLightType, GpuAreaLight};
pub use lumen::{LumenConfig, LumenGI, LumenQuality};
pub use material::{
    ArrayLayout, MaterialGpu, MaterialGpuArrays, MaterialLayerDesc, MaterialLoadStats,
    MaterialManager, MaterialPackDesc,
};
pub use material_bindless::{BindlessMaterialConfig, BindlessMaterialSystem, GpuMaterialEntry};
pub use material_extended::{
    MaterialDefinitionExtended, MaterialGpuExtended, MATERIAL_FLAG_ANISOTROPY,
    MATERIAL_FLAG_CLEARCOAT, MATERIAL_FLAG_SHEEN, MATERIAL_FLAG_SUBSURFACE,
    MATERIAL_FLAG_TRANSMISSION,
};
pub use mesh::{CpuMesh, MeshVertex, MeshVertexLayout};
pub use mesh_registry::{MeshHandle, MeshKey, MeshRegistry};
pub use msaa::{create_msaa_depth_texture, MsaaMode, MsaaRenderTarget};
pub use oit::{OitBuffers, WboitRenderer, ACCUM_FORMAT, REVEALAGE_FORMAT};
pub use parallax::PomConfig;
pub use particle_forces::{
    ColorGradient, EmissionShape, ParticleForces, ParticleSimPass, SimParams, SizeCurve,
};
pub use particle_render::{
    ParticleBlendMode, ParticleCameraUniforms, ParticleRenderConfig, ParticleRenderPass,
};
pub use particle_sort::{ParticleSortPass, SortEntry};
pub use residency::ResidencyManager;
pub use scene_environment::{
    SceneEnvironment, SceneEnvironmentUBO, WGSL_FOG_FUNCTIONS, WGSL_SCENE_ENVIRONMENT,
};
pub use shader_manager::{ShaderKey, ShaderManager};
pub use staging_ring::{StagingRing, SubAllocation};
pub use subgroup_ops::SubgroupCapabilities;
pub use surface_cache::{
    DirectionalLightGpu, ProbeSH, SurfaceCacheConfig, SurfaceCacheParams, SurfaceCachePass,
};
pub use taa::TaaConfig;
pub use temporal_upscale::{TemporalUpscalePass, UpscaleConfig, UpscaleQuality};
// Note: `material_library::Material` is intentionally NOT re-exported at the
// crate root to avoid colliding with `types::Material`. Access it via
// `astraweave_render::material_library::Material` if a typed handle is needed.
pub use material_library::{
    MaterialLibrary, MATERIAL_DISPLAY_NAMES, MATERIAL_NAMES, MAX_TERRAIN_LAYERS, NUM_SPLAT_MAPS,
};
pub use terrain_material::{
    TerrainLayerDesc, TerrainLayerGpu, TerrainMaterialDesc, TerrainMaterialGpu,
};
#[cfg(feature = "terrain-splat-arrays")]
pub use terrain_material_manager::{
    CameraForwardGpu, ChunkKey, LayerTextures, TerrainMaterialConfig, TerrainMaterialManager,
    TerrainSceneEnvGpu, TerrainSplatVertex,
};
pub use texture_streaming::{TextureStreamingManager, TextureStreamingStats};
pub use transparency::{create_blend_state, BlendMode, TransparencyManager, TransparentInstance};
pub use virtual_texture::{PageCache, PageRequest, VirtualTextureConfig, VirtualTextureFeedback};
pub use volumetric_clouds::{CloudConfig, CloudQuality, VolumetricCloudsPass};
pub use volumetric_fog::{VolumetricFogConfig, VolumetricFogPass, VolumetricQuality};

// Phase 2 Task 5: Skeletal Animation exports
pub use animation::{
    compute_joint_matrices, skin_vertex_cpu, AnimationChannel, AnimationClip, AnimationState,
    ChannelData, Interpolation, Joint, JointMatrixGPU, JointPalette, Skeleton, Transform,
    MAX_JOINTS,
};

#[cfg(feature = "skinning-gpu")]
pub use skinning_gpu::{JointPaletteHandle, JointPaletteManager, SKINNING_GPU_SHADER};

// Comprehensive renderer tests (Phase 1: Foundation)
#[cfg(test)]
mod renderer_tests;
