//! Batch 12 — Terrain renderer, debug quad, mesh registry, overlay, weather, clustered forward
//!
//! Targets: terrain.rs, debug_quad.rs, mesh_registry.rs, overlay.rs, effects.rs, clustered_forward.rs
//! All tests are CPU-only (no GPU device required).

use astraweave_render::clustered_forward::{ClusterConfig, GpuCluster, GpuLight};
use astraweave_render::debug_quad::{create_screen_quad, DebugQuadVertex};
use astraweave_render::effects::WeatherKind;
use astraweave_render::mesh_registry::{MeshHandle, MeshKey, MeshRegistry};
use astraweave_render::overlay::OverlayParams;
use astraweave_render::terrain::{
    generate_terrain_preview, TerrainRenderer, TerrainVertex, VegetationRenderInstance,
};
use astraweave_terrain::WorldConfig;
use glam::Vec3;

// ─── TerrainVertex (Pod/Zeroable) ───────────────────────────────────

#[test]
fn terrain_vertex_size_is_36_bytes() {
    // position: [f32;3]=12 + normal: [f32;3]=12 + uv: [f32;2]=8 + biome_id: u32=4 = 36
    assert_eq!(
        std::mem::size_of::<TerrainVertex>(),
        36,
        "TerrainVertex should be 36 bytes"
    );
}

#[test]
fn terrain_vertex_zeroed_is_all_zeros() {
    let v = TerrainVertex {
        position: [0.0; 3],
        normal: [0.0; 3],
        uv: [0.0; 2],
        biome_id: 0,
    };
    let bytes: &[u8] = bytemuck::bytes_of(&v);
    assert!(bytes.iter().all(|&b| b == 0));
}

#[test]
fn terrain_vertex_pod_roundtrip() {
    let v = TerrainVertex {
        position: [1.0, 2.0, 3.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.5, 0.5],
        biome_id: 3,
    };
    let bytes: &[u8] = bytemuck::bytes_of(&v);
    let v2: &TerrainVertex = bytemuck::from_bytes(bytes);
    assert_eq!(v2.position, [1.0, 2.0, 3.0]);
    assert_eq!(v2.normal, [0.0, 1.0, 0.0]);
    assert_eq!(v2.uv, [0.5, 0.5]);
    assert_eq!(v2.biome_id, 3);
}

// ─── TerrainRenderer ────────────────────────────────────────────────

#[test]
fn terrain_renderer_new_default_config() {
    let config = WorldConfig::default();
    let renderer = TerrainRenderer::new(config);
    // Canonical WorldConfig::default().chunk_size is 512.0 (astraweave-terrain/src/lib.rs).
    assert_eq!(renderer.world_generator().config().chunk_size, 512.0);
}

#[test]
fn terrain_renderer_no_loaded_meshes_initially() {
    let config = WorldConfig::default();
    let renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(99, 99);
    assert!(renderer.get_loaded_mesh(chunk_id).is_none());
}

#[test]
fn terrain_renderer_generate_chunk_mesh() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(0, 0);
    let mesh = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    assert_eq!(mesh.chunk_id, chunk_id);
    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.indices.is_empty());
}

#[test]
fn terrain_renderer_loaded_after_generate() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(0, 0);
    renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    assert!(renderer.get_loaded_mesh(chunk_id).is_some());
}

#[test]
fn terrain_renderer_chunk_vertices_have_valid_biome_ids() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(0, 0);
    let mesh = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    for v in &mesh.vertices {
        assert!(v.biome_id <= 7, "Biome ID {} exceeds max 7", v.biome_id);
    }
}

#[test]
fn terrain_renderer_chunk_indices_in_bounds() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let chunk_id = astraweave_terrain::ChunkId::new(0, 0);
    let mesh = renderer.get_or_generate_chunk_mesh(chunk_id).unwrap();
    let vcount = mesh.vertices.len() as u32;
    for &idx in &mesh.indices {
        assert!(idx < vcount, "Index {} >= vertex count {}", idx, vcount);
    }
}

#[test]
fn terrain_renderer_get_chunks_in_radius() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let center = Vec3::new(128.0, 0.0, 128.0);
    let chunks = renderer.get_chunks_in_radius(center, 1).unwrap();
    assert!(!chunks.is_empty());
    for cid in &chunks {
        assert!(renderer.get_loaded_mesh(*cid).is_some());
    }
}

// NOTE: generate_chunk_complete_returns_scatter removed — too slow for test (~60s+)

#[test]
fn terrain_renderer_world_generator_mut_accessible() {
    let config = WorldConfig::default();
    let mut renderer = TerrainRenderer::new(config);
    let _gen = renderer.world_generator_mut();
}

// ─── generate_terrain_preview ───────────────────────────────────────

#[test]
fn terrain_preview_correct_size() {
    let config = WorldConfig::default();
    let preview = generate_terrain_preview(&config, Vec3::new(128.0, 0.0, 128.0), 16).unwrap();
    assert_eq!(preview.len(), 16 * 16);
}

#[test]
fn terrain_preview_values_are_finite() {
    let config = WorldConfig::default();
    let preview = generate_terrain_preview(&config, Vec3::ZERO, 8).unwrap();
    for &h in &preview {
        assert!(h.is_finite(), "Height {} is not finite", h);
    }
}

// ─── VegetationRenderInstance ───────────────────────────────────────

#[test]
fn vegetation_render_instance_size() {
    // transform: [f32;16]=64 + vegetation_type: u32=4 = 68
    assert_eq!(
        std::mem::size_of::<VegetationRenderInstance>(),
        68,
        "VegetationRenderInstance should be 68 bytes"
    );
}

// ─── DebugQuadVertex ────────────────────────────────────────────────

#[test]
fn debug_quad_vertex_size_is_20_bytes() {
    // position: [f32;3]=12 + uv: [f32;2]=8 = 20
    assert_eq!(
        std::mem::size_of::<DebugQuadVertex>(),
        20,
        "DebugQuadVertex should be 20 bytes"
    );
}

#[test]
fn debug_quad_vertex_zeroed() {
    let v = DebugQuadVertex {
        position: [0.0; 3],
        uv: [0.0; 2],
    };
    let bytes: &[u8] = bytemuck::bytes_of(&v);
    assert!(bytes.iter().all(|&b| b == 0));
}

#[test]
fn create_screen_quad_has_6_vertices() {
    let quad = create_screen_quad();
    assert_eq!(
        quad.len(),
        6,
        "Full-screen quad should be 2 triangles = 6 vertices"
    );
}

#[test]
fn create_screen_quad_covers_ndc_range() {
    let quad = create_screen_quad();
    let min_x = quad
        .iter()
        .map(|v| v.position[0])
        .fold(f32::INFINITY, f32::min);
    let max_x = quad
        .iter()
        .map(|v| v.position[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = quad
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = quad
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!(min_x, -1.0, "Quad should reach NDC left edge");
    assert_eq!(max_x, 1.0, "Quad should reach NDC right edge");
    assert_eq!(min_y, -1.0, "Quad should reach NDC bottom edge");
    assert_eq!(max_y, 1.0, "Quad should reach NDC top edge");
}

#[test]
fn create_screen_quad_all_z_zero() {
    let quad = create_screen_quad();
    for v in &quad {
        assert_eq!(v.position[2], 0.0, "Quad Z should be 0");
    }
}

#[test]
fn create_screen_quad_uvs_in_0_1_range() {
    let quad = create_screen_quad();
    for v in &quad {
        assert!(
            (0.0..=1.0).contains(&v.uv[0]),
            "U out of [0,1]: {}",
            v.uv[0]
        );
        assert!(
            (0.0..=1.0).contains(&v.uv[1]),
            "V out of [0,1]: {}",
            v.uv[1]
        );
    }
}

// ─── MeshKey / MeshHandle / MeshRegistry ────────────────────────────

#[test]
fn mesh_key_equality() {
    let a = MeshKey("cube".to_string());
    let b = MeshKey("cube".to_string());
    let c = MeshKey("sphere".to_string());
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn mesh_handle_equality() {
    let a = MeshHandle(1);
    let b = MeshHandle(1);
    let c = MeshHandle(2);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn mesh_registry_new_is_empty() {
    let reg = MeshRegistry::new();
    assert!(reg.get(&MeshKey("anything".to_string())).is_none());
}

#[test]
fn mesh_registry_default_is_empty() {
    let reg = MeshRegistry::default();
    assert!(reg.get(&MeshKey("test".to_string())).is_none());
}

// ─── OverlayParams (Pod/Zeroable) ──────────────────────────────────

#[test]
fn overlay_params_size_is_16_bytes() {
    // fade: f32=4 + letterbox: f32=4 + _pad: [f32;2]=8 = 16
    assert_eq!(
        std::mem::size_of::<OverlayParams>(),
        16,
        "OverlayParams should be 16 bytes"
    );
}

#[test]
fn overlay_params_zeroed() {
    let p = OverlayParams {
        fade: 0.0,
        letterbox: 0.0,
        _pad: [0.0; 2],
    };
    let bytes: &[u8] = bytemuck::bytes_of(&p);
    assert!(bytes.iter().all(|&b| b == 0));
}

#[test]
fn overlay_params_pod_roundtrip() {
    let p = OverlayParams {
        fade: 0.5,
        letterbox: 0.1,
        _pad: [0.0; 2],
    };
    let bytes: &[u8] = bytemuck::bytes_of(&p);
    let p2: &OverlayParams = bytemuck::from_bytes(bytes);
    assert_eq!(p2.fade, 0.5);
    assert_eq!(p2.letterbox, 0.1);
}

// ─── WeatherKind ────────────────────────────────────────────────────

#[test]
fn weather_kind_variants_distinguishable() {
    let kinds = [
        WeatherKind::None,
        WeatherKind::Rain,
        WeatherKind::Snow,
        WeatherKind::Sandstorm,
        WeatherKind::WindTrails,
    ];
    for i in 0..kinds.len() {
        for j in (i + 1)..kinds.len() {
            assert_ne!(
                kinds[i], kinds[j],
                "Variants {:?} and {:?} should differ",
                kinds[i], kinds[j]
            );
        }
    }
}

#[test]
fn weather_kind_clone_eq() {
    let a = WeatherKind::Rain;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn weather_kind_none_is_none() {
    assert_eq!(WeatherKind::None, WeatherKind::None);
}

// ─── ClusteredForward: ClusterConfig ────────────────────────────────

#[test]
fn cluster_config_default_values() {
    let cfg = ClusterConfig::default();
    assert_eq!(cfg.cluster_x, 16);
    assert_eq!(cfg.cluster_y, 9);
    assert_eq!(cfg.cluster_z, 24);
    assert!((cfg.near - 0.1).abs() < f32::EPSILON);
    assert!((cfg.far - 100.0).abs() < f32::EPSILON);
}

#[test]
fn cluster_config_size_is_32_bytes() {
    assert_eq!(
        std::mem::size_of::<ClusterConfig>(),
        32,
        "ClusterConfig should be 32 bytes"
    );
}

#[test]
fn cluster_config_total_clusters() {
    let cfg = ClusterConfig::default();
    let total = cfg.cluster_x * cfg.cluster_y * cfg.cluster_z;
    assert_eq!(total, 16 * 9 * 24, "Total clusters should be 3456");
}

#[test]
fn cluster_config_pod_roundtrip() {
    let cfg = ClusterConfig::default();
    let bytes: &[u8] = bytemuck::bytes_of(&cfg);
    let cfg2: &ClusterConfig = bytemuck::from_bytes(bytes);
    assert_eq!(cfg2.cluster_x, 16);
    assert_eq!(cfg2.cluster_y, 9);
    assert_eq!(cfg2.cluster_z, 24);
}

// ─── ClusteredForward: GpuLight ─────────────────────────────────────

#[test]
fn gpu_light_cf_size_is_32_bytes() {
    assert_eq!(
        std::mem::size_of::<GpuLight>(),
        32,
        "GpuLight should be 32 bytes"
    );
}

#[test]
fn gpu_light_cf_new_stores_fields() {
    let light = GpuLight::new(
        Vec3::new(1.0, 2.0, 3.0),
        5.0,
        Vec3::new(0.5, 0.6, 0.7),
        10.0,
    );
    assert_eq!(light.position[0], 1.0);
    assert_eq!(light.position[1], 2.0);
    assert_eq!(light.position[2], 3.0);
    assert_eq!(light.position[3], 5.0, "w component = radius");
    assert_eq!(light.color[0], 0.5);
    assert_eq!(light.color[1], 0.6);
    assert_eq!(light.color[2], 0.7);
    assert_eq!(light.color[3], 10.0, "w component = intensity");
}

#[test]
fn gpu_light_cf_pod_roundtrip() {
    let light = GpuLight::new(Vec3::ONE, 1.0, Vec3::ZERO, 0.0);
    let bytes: &[u8] = bytemuck::bytes_of(&light);
    let l2: &GpuLight = bytemuck::from_bytes(bytes);
    assert_eq!(l2.position[0], 1.0);
    assert_eq!(l2.color[0], 0.0);
}

// ─── ClusteredForward: GpuCluster ───────────────────────────────────

#[test]
fn gpu_cluster_size_is_48_bytes() {
    assert_eq!(
        std::mem::size_of::<GpuCluster>(),
        48,
        "GpuCluster should be 48 bytes"
    );
}

#[test]
fn gpu_cluster_zeroed() {
    let c = GpuCluster {
        min_bounds: [0.0; 4],
        max_bounds: [0.0; 4],
        light_offset: 0,
        light_count: 0,
        _padding: [0; 2],
    };
    let bytes: &[u8] = bytemuck::bytes_of(&c);
    assert!(bytes.iter().all(|&b| b == 0));
}
