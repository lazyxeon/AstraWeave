use astraweave_scene::gpu_resource_manager::{AssetId, CellGpuResources};
use astraweave_scene::world_partition::GridCoord;

async fn create_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let mut misses = Vec::new();
    for fallback in [true, false] {
        let requested = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: fallback,
                compatible_surface: None,
            })
            .await;
        let adapter = match requested {
            Ok(adapter) => adapter,
            Err(err) => {
                misses.push(format!("force_fallback={fallback}: {err}"));
                continue;
            }
        };
        match adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
        {
            Ok(device) => return Some(device),
            Err(err) => misses.push(format!("force_fallback={fallback} device: {err}")),
        }
    }
    let reason = format!(
        "test_memory_accounting_overwrite found no adapter. {}",
        misses.join("; ")
    );
    if std::env::var_os("AW_GPU_SKIP_IF_UNAVAILABLE").is_some() {
        astraweave_render::renderer::record_gpu_skip_for_tests(&reason);
        None
    } else {
        panic!("{reason}");
    }
}

#[tokio::test]
async fn test_memory_accounting_overwrite() {
    let Some((device, queue)) = create_device().await else {
        return;
    };
    let coord = GridCoord { x: 0, y: 0, z: 0 }; // Assuming constructor or public fields
    let mut resources = CellGpuResources::new(coord);
    let asset_id: AssetId = 1;

    // 1. Upload Vertex Buffer (100 bytes)
    let vertices = vec![0u8; 100];
    resources
        .upload_vertex_buffer(&device, &queue, asset_id, &vertices)
        .unwrap();

    assert_eq!(
        resources.memory_usage, 100,
        "Initial vertex upload size mismatch"
    );

    // 2. Overwrite Vertex Buffer (100 bytes)
    // Should result in 100 bytes total (subtract old 100, add new 100)
    resources
        .upload_vertex_buffer(&device, &queue, asset_id, &vertices)
        .unwrap();

    assert_eq!(
        resources.memory_usage, 100,
        "Overwrite vertex buffer should maintain correct size"
    );

    // 3. Overwrite with larger buffer (150 bytes)
    let larger_vertices = vec![0u8; 150];
    resources
        .upload_vertex_buffer(&device, &queue, asset_id, &larger_vertices)
        .unwrap();
    assert_eq!(
        resources.memory_usage, 150,
        "Overwrite with larger buffer failed"
    );

    // 4. Upload Texture (4x4 RGBA8 = 64 bytes)
    let tex_width = 4;
    let tex_height = 4;
    let tex_data = vec![0u8; (tex_width * tex_height * 4) as usize];
    let tex_id: AssetId = 2;

    resources
        .upload_texture(&device, &queue, tex_id, tex_width, tex_height, &tex_data)
        .unwrap();

    // Total = 150 (vertex) + 64 (texture) = 214
    assert_eq!(
        resources.memory_usage,
        150 + 64,
        "Texture upload addition failed"
    );

    // 5. Overwrite Texture (same size)
    resources
        .upload_texture(&device, &queue, tex_id, tex_width, tex_height, &tex_data)
        .unwrap();
    assert_eq!(resources.memory_usage, 150 + 64, "Texture overwrite failed");

    // 6. Unload All
    resources.unload_all();
    assert_eq!(
        resources.memory_usage, 0,
        "Unload all should reset memory usage"
    );
    assert!(
        resources.texture_sizes.is_empty(),
        "Texture sizes should be cleared"
    );
}
