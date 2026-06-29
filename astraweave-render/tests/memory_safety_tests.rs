use astraweave_render::mesh::{CpuMesh, MeshVertex};
use astraweave_render::mesh_registry::{MeshKey, MeshRegistry};
use glam::{Vec2, Vec3, Vec4};
use std::sync::Arc;

// Mock device/queue creation is painful in wgpu without a real backend or headless.
// We can try to use wgpu::Instance to create a software adapter if possible, or just use the public API if we can mock Device?
// wgpu::Device cannot be easily mocked.
// However, MeshRegistry requires Device/Queue for `fetch_or_upload`.
// We can skip the actual wgpu test if it's too hard to set up, but the prompt requires "Unit Test Verification".
// I'll assume we can create a headless device.

async fn create_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: true,
            compatible_surface: None,
        })
        .await
        .unwrap();

    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .unwrap()
}

#[tokio::test]
async fn test_mesh_registry_pruning() {
    let (device, queue) = create_device().await;
    let mut registry = MeshRegistry::new();

    let cpu_mesh = CpuMesh {
        vertices: vec![MeshVertex::new(Vec3::ZERO, Vec3::Y, Vec4::ONE, Vec2::ZERO)],
        indices: vec![0, 0, 0],
        ..Default::default()
    };
    let key = MeshKey("test_mesh".to_string());

    // 1. Upload mesh
    let handle = registry
        .fetch_or_upload(&device, &queue, key.clone(), &cpu_mesh)
        .unwrap();

    // 2. Get reference (Strong count = 2: one in registry, one here)
    let mesh_arc = registry.get_mesh(handle).expect("Mesh should exist");
    assert_eq!(Arc::strong_count(&mesh_arc), 2);

    // 3. Prune should NOT remove it because we hold a reference
    let pruned = registry.prune();
    assert_eq!(pruned, 0, "Should not prune mesh currently in use");
    assert!(registry.get_gpu(handle).is_some());

    // 4. Drop our reference (Strong count = 1: only registry)
    drop(mesh_arc);

    // 5. Prune should remove it
    let pruned = registry.prune();
    assert_eq!(pruned, 1, "Should prune unused mesh");
    assert!(registry.get_gpu(handle).is_none(), "Mesh should be gone");

    // Verify map is also cleaned up
    assert!(
        registry.get(&key).is_none(),
        "Key mapping should be removed"
    );
}
