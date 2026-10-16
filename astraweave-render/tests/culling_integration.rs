//! Integration tests for GPU-driven frustum culling
//! Validates CPU vs GPU culling parity and headless execution

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

use astraweave_render::culling::{cpu_frustum_cull, CullingPipeline, FrustumPlanes, InstanceAABB};
use glam::{Mat4, Vec3};

/// Helper to create a test wgpu device
fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    pollster::block_on(gpu_capability::request_headless(
        "culling_test_device",
        wgpu::Features::empty(),
        wgpu::Limits::downlevel_defaults(),
        wgpu::PowerPreference::LowPower,
        false,
    ))
    .map(|gpu| (gpu.device, gpu.queue))
}

#[test]
fn test_gpu_culling_pipeline_creation() {
    let Some((device, _queue)) = create_test_device() else {
        return;
    };
    let _pipeline = CullingPipeline::new(&device);
    // If we get here, pipeline creation succeeded
}

#[test]
fn test_cpu_vs_gpu_culling_parity() {
    let Some((device, queue)) = create_test_device() else {
        return;
    };
    let pipeline = CullingPipeline::new(&device);

    // Create perspective frustum
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
    let view_proj = proj * view;
    let frustum = FrustumPlanes::from_view_proj(&view_proj);

    // Create test instances
    let instances = vec![
        InstanceAABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::splat(0.5), 0), // Visible (at origin)
        InstanceAABB::new(Vec3::new(0.0, 0.0, -50.0), Vec3::splat(0.5), 1), // Visible (in front)
        InstanceAABB::new(Vec3::new(50.0, 0.0, 0.0), Vec3::splat(0.5), 2), // Not visible (too far right)
        InstanceAABB::new(Vec3::new(0.0, 50.0, 0.0), Vec3::splat(0.5), 3), // Not visible (too far up)
        InstanceAABB::new(Vec3::new(0.0, 0.0, 10.0), Vec3::splat(0.5), 4), // Not visible (behind camera)
    ];

    // CPU culling
    let cpu_visible = cpu_frustum_cull(&instances, &frustum);

    // GPU culling
    let resources = pipeline.create_culling_resources(&device, &instances, &frustum);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("culling_test_encoder"),
    });

    // Use execute_with_clear to ensure count buffer is zeroed
    pipeline.execute_with_clear(&mut encoder, &resources, instances.len() as u32);

    // Create staging buffer to readback results
    let staging_count = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_count"),
        size: std::mem::size_of::<u32>() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let staging_visible = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_visible"),
        size: (instances.len() * std::mem::size_of::<u32>()) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &resources.count_buffer,
        0,
        &staging_count,
        0,
        std::mem::size_of::<u32>() as u64,
    );
    encoder.copy_buffer_to_buffer(
        &resources.visible_buffer,
        0,
        &staging_visible,
        0,
        (instances.len() * std::mem::size_of::<u32>()) as u64,
    );

    queue.submit(Some(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);

    // Readback count
    let count_slice = staging_count.slice(..);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    count_slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).ok();
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv().unwrap().unwrap();

    let count_data = count_slice.get_mapped_range();
    let gpu_count =
        u32::from_ne_bytes([count_data[0], count_data[1], count_data[2], count_data[3]]);
    drop(count_data);
    staging_count.unmap();

    // Readback visible instances
    let visible_slice = staging_visible.slice(..);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    visible_slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).ok();
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv().unwrap().unwrap();

    let visible_data = visible_slice.get_mapped_range();
    let mut gpu_visible: Vec<u32> = Vec::new();
    for i in 0..gpu_count as usize {
        let offset = i * std::mem::size_of::<u32>();
        let index = u32::from_ne_bytes([
            visible_data[offset],
            visible_data[offset + 1],
            visible_data[offset + 2],
            visible_data[offset + 3],
        ]);
        gpu_visible.push(index);
    }
    drop(visible_data);
    staging_visible.unmap();

    // Sort both lists for comparison (GPU order is non-deterministic due to atomics)
    let mut cpu_sorted = cpu_visible.clone();
    let mut gpu_sorted = gpu_visible.clone();
    cpu_sorted.sort_unstable();
    gpu_sorted.sort_unstable();

    // Verify counts match
    assert_eq!(
        cpu_sorted.len(),
        gpu_sorted.len(),
        "CPU culled {} instances, GPU culled {}",
        cpu_sorted.len(),
        gpu_sorted.len()
    );

    // Verify same instances visible
    assert_eq!(
        cpu_sorted, gpu_sorted,
        "CPU and GPU culling produced different visible sets"
    );

    println!(
        "✓ CPU/GPU parity: {} visible instances (CPU: {:?}, GPU: {:?})",
        cpu_sorted.len(),
        cpu_sorted,
        gpu_sorted
    );
}

#[test]
fn test_culling_reduces_draw_count() {
    let Some((device, queue)) = create_test_device() else {
        return;
    };
    let pipeline = CullingPipeline::new(&device);

    // Create frustum looking down negative Z
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    let view_proj = proj * view;
    let frustum = FrustumPlanes::from_view_proj(&view_proj);

    // Create grid of instances - half in frustum, half outside
    let mut instances = Vec::new();
    let mut in_frustum_count = 0;
    for x in -5..5 {
        for y in -5..5 {
            for z in -5..5 {
                let pos = Vec3::new(x as f32 * 2.0, y as f32 * 2.0, z as f32 * 2.0 - 20.0);
                let idx = instances.len() as u32;
                instances.push(InstanceAABB::new(pos, Vec3::splat(0.5), idx));

                // Count how many should be visible (CPU test)
                if frustum.test_aabb(pos, Vec3::splat(0.5)) {
                    in_frustum_count += 1;
                }
            }
        }
    }

    let total_count = instances.len();

    // GPU culling
    let resources = pipeline.create_culling_resources(&device, &instances, &frustum);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("culling_test_encoder"),
    });

    pipeline.execute_with_clear(&mut encoder, &resources, instances.len() as u32);

    let staging_count = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_count"),
        size: std::mem::size_of::<u32>() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &resources.count_buffer,
        0,
        &staging_count,
        0,
        std::mem::size_of::<u32>() as u64,
    );
    queue.submit(Some(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);

    let count_slice = staging_count.slice(..);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    count_slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).ok();
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv().unwrap().unwrap();

    let count_data = count_slice.get_mapped_range();
    let visible_count =
        u32::from_ne_bytes([count_data[0], count_data[1], count_data[2], count_data[3]]) as usize;
    drop(count_data);
    staging_count.unmap();

    println!(
        "✓ Culling test: {}/{} instances visible ({}% reduction)",
        visible_count,
        total_count,
        100 - (visible_count * 100 / total_count.max(1))
    );

    // Verify significant culling occurred
    assert!(
        visible_count < total_count,
        "Expected some instances to be culled"
    );
    assert_eq!(
        visible_count, in_frustum_count,
        "GPU visible count should match CPU count"
    );
}

#[test]
fn test_all_instances_visible_when_inside_frustum() {
    let Some((device, queue)) = create_test_device() else {
        return;
    };
    let pipeline = CullingPipeline::new(&device);

    // Very wide frustum
    let view = Mat4::IDENTITY;
    let proj = Mat4::orthographic_rh(-100.0, 100.0, -100.0, 100.0, 0.1, 200.0);
    let view_proj = proj * view;
    let frustum = FrustumPlanes::from_view_proj(&view_proj);

    // All instances well within frustum
    let instances = vec![
        InstanceAABB::new(Vec3::new(0.0, 0.0, -10.0), Vec3::splat(0.5), 0),
        InstanceAABB::new(Vec3::new(5.0, 5.0, -10.0), Vec3::splat(0.5), 1),
        InstanceAABB::new(Vec3::new(-5.0, -5.0, -10.0), Vec3::splat(0.5), 2),
    ];

    let resources = pipeline.create_culling_resources(&device, &instances, &frustum);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("culling_test_encoder"),
    });

    pipeline.execute_with_clear(&mut encoder, &resources, instances.len() as u32);

    let staging_count = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_count"),
        size: std::mem::size_of::<u32>() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &resources.count_buffer,
        0,
        &staging_count,
        0,
        std::mem::size_of::<u32>() as u64,
    );
    queue.submit(Some(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);

    let count_slice = staging_count.slice(..);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    count_slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).ok();
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv().unwrap().unwrap();

    let count_data = count_slice.get_mapped_range();
    let visible_count =
        u32::from_ne_bytes([count_data[0], count_data[1], count_data[2], count_data[3]]);
    drop(count_data);
    staging_count.unmap();

    assert_eq!(
        visible_count as usize,
        instances.len(),
        "All instances should be visible"
    );
}

#[test]
fn test_empty_instance_list() {
    let Some((device, queue)) = create_test_device() else {
        return;
    };
    let pipeline = CullingPipeline::new(&device);

    let view_proj = Mat4::IDENTITY;
    let frustum = FrustumPlanes::from_view_proj(&view_proj);
    let instances: Vec<InstanceAABB> = vec![];

    // Should not crash with empty input
    let resources = pipeline.create_culling_resources(&device, &instances, &frustum);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("culling_test_encoder"),
    });

    pipeline.execute(&mut encoder, &resources.bind_group, 0);
    queue.submit(Some(encoder.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);
}
