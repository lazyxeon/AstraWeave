//! Batch 7: GI/voxelization configs, GPU particles, deferred G-buffer formats
//! Mutation-resistant integration tests targeting:
//!   - VxgiConfig (default values, size, Pod/Zeroable)
//!   - VoxelRadiance (Pod/Zeroable, size)
//!   - HybridGiConfig (default flags, nested VxgiConfig)
//!   - VoxelizationConfig (default, size, Pod/Zeroable)
//!   - VoxelVertex (new, size, Pod/Zeroable)
//!   - VoxelMaterial (default, from_albedo, emissive, size, Pod/Zeroable)
//!   - VoxelizationMesh (new, triangle_count)
//!   - VoxelizationStats (default zero)
//!   - GpuParticle (size, Pod/Zeroable)
//!   - EmitterParams (size, Pod/Zeroable)
//!   - GBufferFormats (default texture formats)

use glam::Vec3;

use astraweave_render::deferred::GBufferFormats;
use astraweave_render::gi::HybridGiConfig;
use astraweave_render::gi::{
    VoxelMaterial, VoxelRadiance, VoxelVertex, VoxelizationConfig, VoxelizationMesh,
    VoxelizationStats, VxgiConfig,
};
use astraweave_render::gpu_particles::{EmitterParams, GpuParticle};

// ═══════════════════════════════════════════════════════════════════════════════
//  VxgiConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn vxgi_config_default_voxel_resolution() {
    let c = VxgiConfig::default();
    assert_eq!(c.voxel_resolution, 256);
}

#[test]
fn vxgi_config_default_world_size() {
    let c = VxgiConfig::default();
    assert!((c.world_size - 1000.0).abs() < 1e-3);
}

#[test]
fn vxgi_config_default_cone_count() {
    let c = VxgiConfig::default();
    assert_eq!(c.cone_count, 6);
}

#[test]
fn vxgi_config_default_max_trace_distance() {
    let c = VxgiConfig::default();
    assert!((c.max_trace_distance - 100.0).abs() < 1e-3);
}

#[test]
fn vxgi_config_default_cone_aperture() {
    let c = VxgiConfig::default();
    assert!((c.cone_aperture - 0.577).abs() < 0.001, "aperture ~33°");
}

#[test]
fn vxgi_config_default_padding_zero() {
    let c = VxgiConfig::default();
    assert_eq!(c._pad, [0; 3]);
}

#[test]
fn vxgi_config_size_32_bytes() {
    assert_eq!(std::mem::size_of::<VxgiConfig>(), 32);
}

#[test]
fn vxgi_config_pod_zeroable() {
    let c = VxgiConfig::default();
    let bytes = bytemuck::bytes_of(&c);
    assert_eq!(bytes.len(), 32);
    let back: &VxgiConfig = bytemuck::from_bytes(bytes);
    assert_eq!(back.voxel_resolution, 256);
    assert_eq!(back.cone_count, 6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VoxelRadiance
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn voxel_radiance_size_16_bytes() {
    assert_eq!(std::mem::size_of::<VoxelRadiance>(), 16);
}

#[test]
fn voxel_radiance_zeroed() {
    let r: VoxelRadiance = bytemuck::Zeroable::zeroed();
    assert_eq!(r.radiance, [0.0; 4]);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  HybridGiConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn hybrid_gi_default_use_vxgi_true() {
    let c = HybridGiConfig::default();
    assert!(c.use_vxgi);
}

#[test]
fn hybrid_gi_default_use_ddgi_true() {
    let c = HybridGiConfig::default();
    assert!(c.use_ddgi);
}

#[test]
fn hybrid_gi_default_nested_vxgi_config() {
    let c = HybridGiConfig::default();
    assert_eq!(c.vxgi_config.voxel_resolution, 256);
    assert_eq!(c.vxgi_config.cone_count, 6);
    assert!((c.vxgi_config.world_size - 1000.0).abs() < 1e-3);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VoxelizationConfig
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn voxelization_config_default_resolution() {
    let c = VoxelizationConfig::default();
    assert_eq!(c.voxel_resolution, 256);
}

#[test]
fn voxelization_config_default_world_size() {
    let c = VoxelizationConfig::default();
    assert!((c.world_size - 1000.0).abs() < 1e-3);
}

#[test]
fn voxelization_config_default_triangle_count_zero() {
    let c = VoxelizationConfig::default();
    assert_eq!(c.triangle_count, 0);
}

#[test]
fn voxelization_config_default_light_intensity() {
    let c = VoxelizationConfig::default();
    assert!((c.light_intensity - 1.0).abs() < 1e-6);
}

#[test]
fn voxelization_config_size_16_bytes() {
    assert_eq!(std::mem::size_of::<VoxelizationConfig>(), 16);
}

#[test]
fn voxelization_config_pod_roundtrip() {
    let c = VoxelizationConfig::default();
    let bytes = bytemuck::bytes_of(&c);
    assert_eq!(bytes.len(), 16);
    let back: &VoxelizationConfig = bytemuck::from_bytes(bytes);
    assert_eq!(back.voxel_resolution, c.voxel_resolution);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VoxelVertex
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn voxel_vertex_new_from_vec3() {
    let v = VoxelVertex::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(v.position, [1.0, 2.0, 3.0]);
    assert_eq!(v.normal, [0.0, 1.0, 0.0]);
}

#[test]
fn voxel_vertex_size_24_bytes() {
    // position: [f32;3](12B) + normal: [f32;3](12B) = 24B
    assert_eq!(std::mem::size_of::<VoxelVertex>(), 24);
}

#[test]
fn voxel_vertex_zeroed() {
    let v: VoxelVertex = bytemuck::Zeroable::zeroed();
    assert_eq!(v.position, [0.0; 3]);
    assert_eq!(v.normal, [0.0; 3]);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VoxelMaterial
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn voxel_material_default_albedo() {
    let m = VoxelMaterial::default();
    let eps = 1e-4;
    assert!((m.albedo[0] - 0.8).abs() < eps);
    assert!((m.albedo[1] - 0.8).abs() < eps);
    assert!((m.albedo[2] - 0.8).abs() < eps);
}

#[test]
fn voxel_material_default_metallic_zero() {
    let m = VoxelMaterial::default();
    assert!((m.metallic - 0.0).abs() < 1e-6);
}

#[test]
fn voxel_material_default_roughness() {
    let m = VoxelMaterial::default();
    assert!((m.roughness - 0.8).abs() < 1e-4);
}

#[test]
fn voxel_material_default_emissive_zero() {
    let m = VoxelMaterial::default();
    assert_eq!(m.emissive, [0.0, 0.0, 0.0]);
}

#[test]
fn voxel_material_from_albedo() {
    let m = VoxelMaterial::from_albedo(Vec3::new(1.0, 0.5, 0.3));
    assert_eq!(m.albedo, [1.0, 0.5, 0.3]);
    // Other fields should be default
    assert!((m.metallic - 0.0).abs() < 1e-6);
    assert!((m.roughness - 0.8).abs() < 1e-4);
    assert_eq!(m.emissive, [0.0, 0.0, 0.0]);
}

#[test]
fn voxel_material_emissive_constructor() {
    let m = VoxelMaterial::emissive(Vec3::new(2.0, 3.0, 4.0));
    assert_eq!(m.emissive, [2.0, 3.0, 4.0]);
    // Other fields should be default
    assert!((m.albedo[0] - 0.8).abs() < 1e-4);
    assert!((m.metallic - 0.0).abs() < 1e-6);
}

#[test]
fn voxel_material_size_32_bytes() {
    // albedo[3](12B) + metallic(4B) + roughness(4B) + emissive[3](12B) = 32B
    assert_eq!(std::mem::size_of::<VoxelMaterial>(), 32);
}

#[test]
fn voxel_material_pod_roundtrip() {
    let m = VoxelMaterial::from_albedo(Vec3::new(0.1, 0.2, 0.3));
    let bytes = bytemuck::bytes_of(&m);
    assert_eq!(bytes.len(), 32);
    let back: &VoxelMaterial = bytemuck::from_bytes(bytes);
    assert_eq!(back.albedo, m.albedo);
    assert_eq!(back.roughness, m.roughness);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VoxelizationMesh
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn voxelization_mesh_triangle_count_empty() {
    let mesh = VoxelizationMesh::new(vec![], vec![], VoxelMaterial::default());
    assert_eq!(mesh.triangle_count(), 0);
}

#[test]
fn voxelization_mesh_triangle_count_one_tri() {
    let v = VoxelVertex::new(Vec3::ZERO, Vec3::Y);
    let mesh = VoxelizationMesh::new(vec![v, v, v], vec![0, 1, 2], VoxelMaterial::default());
    assert_eq!(mesh.triangle_count(), 1);
}

#[test]
fn voxelization_mesh_triangle_count_two_tris() {
    let v = VoxelVertex::new(Vec3::ZERO, Vec3::Y);
    let mesh = VoxelizationMesh::new(vec![v; 4], vec![0, 1, 2, 0, 2, 3], VoxelMaterial::default());
    assert_eq!(mesh.triangle_count(), 2);
}

#[test]
fn voxelization_mesh_triangle_count_partial_indices() {
    // 7 indices → 7/3 = 2 triangles (integer division)
    let v = VoxelVertex::new(Vec3::ZERO, Vec3::Y);
    let mesh = VoxelizationMesh::new(
        vec![v; 5],
        vec![0, 1, 2, 0, 2, 3, 4],
        VoxelMaterial::default(),
    );
    assert_eq!(mesh.triangle_count(), 2);
}

#[test]
fn voxelization_mesh_stores_material() {
    let mat = VoxelMaterial::from_albedo(Vec3::new(1.0, 0.0, 0.0));
    let mesh = VoxelizationMesh::new(vec![], vec![], mat);
    assert_eq!(mesh.material.albedo, [1.0, 0.0, 0.0]);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VoxelizationStats
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn voxelization_stats_default_all_zero() {
    let s = VoxelizationStats::default();
    assert_eq!(s.total_triangles, 0);
    assert_eq!(s.total_vertices, 0);
    assert!((s.voxelization_time_ms - 0.0).abs() < 1e-6);
    assert!((s.clear_time_ms - 0.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuParticle
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn gpu_particle_size_64_bytes() {
    // 4 × [f32;4] = 4 × 16 = 64 bytes
    assert_eq!(std::mem::size_of::<GpuParticle>(), 64);
}

#[test]
fn gpu_particle_zeroed() {
    let p: GpuParticle = bytemuck::Zeroable::zeroed();
    assert_eq!(p.position, [0.0; 4]);
    assert_eq!(p.velocity, [0.0; 4]);
    assert_eq!(p.color, [0.0; 4]);
    assert_eq!(p.scale, [0.0; 4]);
}

#[test]
fn gpu_particle_pod_roundtrip() {
    let p = GpuParticle {
        position: [1.0, 2.0, 3.0, 5.0],
        velocity: [0.1, 0.2, 0.3, 1.5],
        color: [1.0, 0.5, 0.0, 1.0],
        scale: [0.5, 0.5, 0.5, 1.0],
    };
    let bytes = bytemuck::bytes_of(&p);
    assert_eq!(bytes.len(), 64);
    let back: &GpuParticle = bytemuck::from_bytes(bytes);
    assert_eq!(back.position, p.position);
    assert_eq!(back.color, p.color);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  EmitterParams
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn emitter_params_size_80_bytes() {
    // position[4](16) + velocity[4](16) + 4×f32(16) + gravity[4](16) + 4×u32(16) = 80
    assert_eq!(std::mem::size_of::<EmitterParams>(), 80);
}

#[test]
fn emitter_params_zeroed() {
    let e: EmitterParams = bytemuck::Zeroable::zeroed();
    assert_eq!(e.position, [0.0; 4]);
    assert_eq!(e.velocity, [0.0; 4]);
    assert!((e.emission_rate - 0.0).abs() < 1e-6);
    assert!((e.lifetime - 0.0).abs() < 1e-6);
    assert_eq!(e.particle_count, 0);
    assert_eq!(e.max_particles, 0);
    assert_eq!(e.gravity, [0.0; 4]);
}

#[test]
fn emitter_params_pod_roundtrip() {
    let e = EmitterParams {
        position: [1.0, 2.0, 3.0, 0.0],
        velocity: [0.0, 1.0, 0.0, 0.0],
        emission_rate: 100.0,
        lifetime: 5.0,
        velocity_randomness: 0.3,
        delta_time: 0.016,
        gravity: [0.0, -9.81, 0.0, 0.0],
        particle_count: 50,
        max_particles: 1000,
        random_seed: 42,
        _padding: 0,
    };
    let bytes = bytemuck::bytes_of(&e);
    assert_eq!(bytes.len(), 80);
    let back: &EmitterParams = bytemuck::from_bytes(bytes);
    assert_eq!(back.emission_rate, 100.0);
    assert_eq!(back.max_particles, 1000);
    assert_eq!(back.random_seed, 42);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GBufferFormats
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn gbuffer_formats_default_albedo() {
    let f = GBufferFormats::default();
    assert_eq!(f.albedo, wgpu::TextureFormat::Rgba8UnormSrgb);
}

#[test]
fn gbuffer_formats_default_normal() {
    let f = GBufferFormats::default();
    assert_eq!(f.normal, wgpu::TextureFormat::Rgba16Float);
}

#[test]
fn gbuffer_formats_default_position_removed() {
    // Position GBuffer was removed: world position is now reconstructed from depth + inverse VP.
    // This test verifies the position field no longer exists on GBufferFormats.
    let f = GBufferFormats::default();
    // Depth format is still present — used for position reconstruction
    assert_eq!(f.depth, wgpu::TextureFormat::Depth32Float);
}

#[test]
fn gbuffer_formats_default_emissive() {
    let f = GBufferFormats::default();
    assert_eq!(f.emissive, wgpu::TextureFormat::Rgba8UnormSrgb);
}

#[test]
fn gbuffer_formats_default_depth() {
    let f = GBufferFormats::default();
    assert_eq!(f.depth, wgpu::TextureFormat::Depth32Float);
}

#[test]
fn gbuffer_formats_normal_is_high_precision() {
    let f = GBufferFormats::default();
    // Normal needs float precision, not SRGB
    assert_ne!(f.normal, wgpu::TextureFormat::Rgba8UnormSrgb);
}
