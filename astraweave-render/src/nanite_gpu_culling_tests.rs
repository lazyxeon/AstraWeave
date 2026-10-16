// Nanite GPU Culling Tests
// Comprehensive validation of GPU-driven visibility pipeline

use crate::nanite_gpu_culling::{CullStats, GpuCamera, NaniteCullingPipeline};
use crate::nanite_visibility::GpuMeshlet;
use glam::{Mat4, Vec3};

/// Create test GPU device for unit tests
async fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    crate::gpu_capability::request_headless(
        "test_device",
        wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
        wgpu::Limits::default(),
        wgpu::PowerPreference::default(),
        false,
    )
    .await
    .map(|gpu| (gpu.device, gpu.queue))
}

/// Create test meshlets for validation
fn create_test_meshlets(count: usize) -> Vec<GpuMeshlet> {
    (0..count)
        .map(|i| {
            let x = (i % 10) as f32 * 10.0;
            let y = ((i / 10) % 10) as f32 * 10.0;
            let z = (i / 100) as f32 * 10.0;

            GpuMeshlet {
                bounds_min: [x, y, z],
                vertex_offset: i as u32 * 64,
                bounds_max: [x + 5.0, y + 5.0, z + 5.0],
                vertex_count: 64,
                cone_apex: [x + 2.5, y + 2.5, z + 2.5],
                triangle_offset: i as u32 * 124 * 3,
                cone_axis: [0.0, 0.0, 1.0],
                triangle_count: 124,
                cone_cutoff: 0.5,
                lod_level: 0,
                lod_error: 0.1,
                material_id: (i % 8) as u32,
            }
        })
        .collect()
}

#[tokio::test]
async fn test_gpu_camera_frustum_extraction() {
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 16.0 / 9.0, 0.1, 100.0);
    let view_proj = proj * view;

    let camera = GpuCamera::from_matrix(view_proj, Vec3::new(0.0, 0.0, 10.0), 1920, 1080);

    // Verify frustum planes are valid
    for plane in &camera.frustum_planes {
        let normal = Vec3::from_array([plane[0], plane[1], plane[2]]);
        assert!(
            (normal.length() - 1.0).abs() < 0.01,
            "Frustum plane should be normalized"
        );
    }

    // Verify Hi-Z mip count
    assert_eq!(camera.hiz_mip_count, 11); // log2(1920) ≈ 10.9
}

#[tokio::test]
async fn test_cull_stats_alignment() {
    // Verify CullStats struct is properly aligned for GPU use
    assert_eq!(std::mem::size_of::<CullStats>(), 32); // 5 u32s + 3 padding
    assert_eq!(std::mem::align_of::<CullStats>(), 4);

    let stats = CullStats::default();
    let bytes = bytemuck::bytes_of(&stats);
    assert_eq!(bytes.len(), 32);
}

#[tokio::test]
async fn test_gpu_meshlet_size() {
    // Verify GpuMeshlet struct size matches shader expectations
    // 20 u32/f32 fields * 4 bytes = 80 bytes
    assert_eq!(std::mem::size_of::<GpuMeshlet>(), 80);

    let meshlet = create_test_meshlets(1)[0];
    let bytes = bytemuck::bytes_of(&meshlet);
    assert_eq!(bytes.len(), 80);
}

#[tokio::test]
async fn test_pipeline_creation() {
    let Some((device, _queue)) = create_test_device().await else {
        return;
    };

    let meshlets = create_test_meshlets(100);
    let vertices = vec![0u8; 1024]; // Placeholder vertex data
    let indices = vec![0u8; 512]; // Placeholder index data

    let pipeline = NaniteCullingPipeline::new(&device, 1920, 1080, &meshlets, &vertices, &indices);

    assert!(pipeline.is_ok(), "Pipeline creation should succeed");

    let pipeline = pipeline.unwrap();
    assert_eq!(pipeline.meshlet_count, 100);
    assert_eq!(pipeline.max_visible_meshlets, 100);
}

#[tokio::test]
async fn test_hiz_pyramid_creation() {
    let Some((device, _queue)) = create_test_device().await else {
        return;
    };

    let meshlets = create_test_meshlets(10);
    let vertices = vec![0u8; 1024];
    let indices = vec![0u8; 512];

    let pipeline =
        NaniteCullingPipeline::new(&device, 1024, 1024, &meshlets, &vertices, &indices).unwrap();

    // Verify Hi-Z texture has correct mip levels
    let expected_mips = (1024_f32).log2().ceil() as u32;
    assert_eq!(pipeline.hiz_views.len(), expected_mips as usize);

    // Verify bind groups for each mip transition
    assert_eq!(pipeline.hiz_bind_groups.len(), (expected_mips - 1) as usize);
}

#[tokio::test]
async fn test_visibility_buffer_format() {
    let Some((device, _queue)) = create_test_device().await else {
        return;
    };

    let meshlets = create_test_meshlets(10);
    let vertices = vec![0u8; 1024];
    let indices = vec![0u8; 512];

    let pipeline =
        NaniteCullingPipeline::new(&device, 1920, 1080, &meshlets, &vertices, &indices).unwrap();

    // Verify visibility buffer uses R32Uint format
    assert_eq!(
        pipeline.visibility_texture.format(),
        wgpu::TextureFormat::R32Uint
    );

    // Verify depth buffer uses R32Float format
    assert_eq!(
        pipeline.depth_texture.format(),
        wgpu::TextureFormat::R32Float
    );
}

#[test]
fn test_visibility_id_packing() {
    // Test packing/unpacking of meshlet and triangle IDs
    let meshlet_id = 1234u32;
    let triangle_id = 98u32;

    let packed = (meshlet_id << 16) | (triangle_id & 0xFFFF);

    let unpacked_meshlet = packed >> 16;
    let unpacked_triangle = packed & 0xFFFF;

    assert_eq!(unpacked_meshlet, meshlet_id);
    assert_eq!(unpacked_triangle, triangle_id);
}

#[test]
fn test_visibility_id_bounds() {
    // Test maximum ID values
    let max_meshlet = 65535u32; // 16 bits
    let max_triangle = 65535u32; // 16 bits

    let packed = (max_meshlet << 16) | (max_triangle & 0xFFFF);

    let unpacked_meshlet = packed >> 16;
    let unpacked_triangle = packed & 0xFFFF;

    assert_eq!(unpacked_meshlet, max_meshlet);
    assert_eq!(unpacked_triangle, max_triangle);
}

#[tokio::test]
async fn test_camera_buffer_update() {
    let Some((device, queue)) = create_test_device().await else {
        return;
    };

    let meshlets = create_test_meshlets(10);
    let vertices = vec![0u8; 1024];
    let indices = vec![0u8; 512];

    let pipeline =
        NaniteCullingPipeline::new(&device, 1920, 1080, &meshlets, &vertices, &indices).unwrap();

    let view_proj = Mat4::IDENTITY;
    let camera = GpuCamera::from_matrix(view_proj, Vec3::ZERO, 1920, 1080);

    // This should not panic
    queue.write_buffer(&pipeline.camera_buffer, 0, bytemuck::bytes_of(&camera));
}

#[test]
fn test_edge_function() {
    // Test edge function for triangle rasterization
    // Returns positive value for points to the left of the edge (CCW winding)
    let edge_fn = |a: (f32, f32), b: (f32, f32), c: (f32, f32)| -> f32 {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    };

    // CCW triangle
    let v0 = (0.0, 0.0);
    let v1 = (1.0, 0.0);
    let v2 = (0.0, 1.0);

    // Point inside triangle
    let p_inside = (0.25, 0.25);
    assert!(edge_fn(v0, v1, p_inside) > 0.0);
    assert!(edge_fn(v1, v2, p_inside) > 0.0);
    assert!(edge_fn(v2, v0, p_inside) > 0.0);

    // Point outside triangle
    let p_outside = (2.0, 2.0);
    let inside_count = [
        edge_fn(v0, v1, p_outside) > 0.0,
        edge_fn(v1, v2, p_outside) > 0.0,
        edge_fn(v2, v0, p_outside) > 0.0,
    ]
    .iter()
    .filter(|&&x| x)
    .count();

    assert!(
        inside_count < 3,
        "Outside point should fail at least one edge test"
    );
}

#[tokio::test]
async fn test_meshlet_buffer_size() {
    let Some((device, _queue)) = create_test_device().await else {
        return;
    };

    let meshlet_count = 10000;
    let meshlets = create_test_meshlets(meshlet_count);
    let vertices = vec![0u8; meshlet_count * 64 * 32]; // 64 verts per meshlet, 32 bytes per vert
    let indices = vec![0u8; meshlet_count * 124 * 3 * 4]; // 124 tris, 3 indices, 4 bytes per index

    let pipeline = NaniteCullingPipeline::new(&device, 1920, 1080, &meshlets, &vertices, &indices);

    assert!(pipeline.is_ok(), "Should handle 10K meshlets");
}

#[tokio::test]
async fn test_maximum_meshlet_capacity() {
    let Some((device, _queue)) = create_test_device().await else {
        return;
    };

    // Test with realistic maximum count (100K meshlets ≈ 10M triangles)
    let meshlet_count = 100_000;
    let meshlets = create_test_meshlets(meshlet_count);

    // Use minimal vertex/index data to avoid OOM
    let vertices = vec![0u8; 1024];
    let indices = vec![0u8; 1024];

    let pipeline = NaniteCullingPipeline::new(&device, 1920, 1080, &meshlets, &vertices, &indices);

    assert!(
        pipeline.is_ok(),
        "Should handle 100K meshlets for 10M+ polygon test"
    );

    if let Ok(pipeline) = pipeline {
        // Verify visible meshlet buffer is large enough
        let expected_size = meshlet_count * std::mem::size_of::<u32>();
        assert!(pipeline.visible_meshlets_buffer.size() >= expected_size as u64);
    }
}

#[test]
fn test_gpu_camera_inv_view_proj() {
    let view_proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 16.0 / 9.0, 0.1, 100.0);
    let camera = GpuCamera::from_matrix(view_proj, Vec3::ZERO, 1920, 1080);

    let inv = Mat4::from_cols_array_2d(&camera.inv_view_proj);
    let original = Mat4::from_cols_array_2d(&camera.view_proj);

    // Verify inverse is correct (within floating point error)
    let identity = original * inv;
    let expected_identity = Mat4::IDENTITY;

    for i in 0..4 {
        for j in 0..4 {
            let diff = (identity.col(i)[j] - expected_identity.col(i)[j]).abs();
            assert!(
                diff < 0.01,
                "Inverse matrix verification failed at [{}, {}]",
                i,
                j
            );
        }
    }
}
