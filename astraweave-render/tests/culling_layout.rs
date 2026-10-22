//! Test to verify InstanceAABB struct layout matches GPU expectations

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::culling::InstanceAABB;
use bytemuck::cast_slice;
use glam::Vec3;

#[test]
fn test_instance_aabb_layout() {
    let inst = InstanceAABB::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.5, 0.6, 0.7), 42);

    // Verify size
    assert_eq!(
        std::mem::size_of::<InstanceAABB>(),
        32,
        "InstanceAABB should be 32 bytes (std140 padding)"
    );

    // Cast to bytes and verify layout
    let bytes: &[u8] = bytemuck::bytes_of(&inst);
    assert_eq!(bytes.len(), 32);

    // Verify fields at correct offsets
    let f32_slice: &[f32] = cast_slice(bytes);

    println!("\n=== InstanceAABB Binary Layout ===");
    println!("Offset  0- 3 (center.x): {}", f32_slice[0]);
    println!("Offset  4- 7 (center.y): {}", f32_slice[1]);
    println!("Offset  8-11 (center.z): {}", f32_slice[2]);
    println!("Offset 12-15 (_pad0):    {} (should be 0)", f32_slice[3]);
    println!("Offset 16-19 (extent.x): {}", f32_slice[4]);
    println!("Offset 20-23 (extent.y): {}", f32_slice[5]);
    println!("Offset 24-27 (extent.z): {}", f32_slice[6]);
    println!(
        "Offset 28-31 (instance_index): {} (should be 42)",
        f32_slice[7]
    );

    assert_eq!(f32_slice[0], 1.0, "center.x");
    assert_eq!(f32_slice[1], 2.0, "center.y");
    assert_eq!(f32_slice[2], 3.0, "center.z");
    assert_eq!(f32_slice[3], 0.0, "_pad0 should be 0");
    assert_eq!(f32_slice[4], 0.5, "extent.x");
    assert_eq!(f32_slice[5], 0.6, "extent.y");
    assert_eq!(f32_slice[6], 0.7, "extent.z");

    // instance_index is u32, need to reinterpret
    let u32_slice: &[u32] = cast_slice(&bytes[28..32]);
    assert_eq!(u32_slice[0], 42, "instance_index");
}

#[test]
fn test_multiple_instances_layout() {
    let instances = vec![
        InstanceAABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::splat(0.5), 0),
        InstanceAABB::new(Vec3::new(0.0, 0.0, -50.0), Vec3::splat(0.5), 1),
    ];

    let bytes: &[u8] = cast_slice(&instances);
    println!("\n=== Two-Instance Buffer ===");
    println!(
        "Total size: {} bytes ({} instances * 32 bytes)",
        bytes.len(),
        instances.len()
    );

    // Parse first instance
    let inst0_bytes = &bytes[0..32];
    let inst0_floats: &[f32] = cast_slice(inst0_bytes);
    println!("\nInstance 0:");
    println!(
        "  center: ({}, {}, {})",
        inst0_floats[0], inst0_floats[1], inst0_floats[2]
    );
    println!("  _pad0: {}", inst0_floats[3]);
    println!(
        "  extent: ({}, {}, {})",
        inst0_floats[4], inst0_floats[5], inst0_floats[6]
    );
    let inst0_index: &[u32] = cast_slice(&inst0_bytes[28..32]);
    println!("  instance_index: {}", inst0_index[0]);

    // Parse second instance
    let inst1_bytes = &bytes[32..64];
    let inst1_floats: &[f32] = cast_slice(inst1_bytes);
    println!("\nInstance 1:");
    println!(
        "  center: ({}, {}, {})",
        inst1_floats[0], inst1_floats[1], inst1_floats[2]
    );
    println!("  _pad0: {}", inst1_floats[3]);
    println!(
        "  extent: ({}, {}, {})",
        inst1_floats[4], inst1_floats[5], inst1_floats[6]
    );
    let inst1_index: &[u32] = cast_slice(&inst1_bytes[28..32]);
    println!("  instance_index: {}", inst1_index[0]);

    // Verify instance 1 data
    assert_eq!(inst1_floats[0], 0.0, "inst1 center.x");
    assert_eq!(inst1_floats[1], 0.0, "inst1 center.y");
    assert_eq!(inst1_floats[2], -50.0, "inst1 center.z");
    assert_eq!(inst1_index[0], 1, "inst1 instance_index");
}
