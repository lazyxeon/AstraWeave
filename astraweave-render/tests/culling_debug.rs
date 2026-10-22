//! Debug test to diagnose perspective frustum extraction

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::culling::{cpu_frustum_cull, FrustumPlanes, InstanceAABB};
use glam::{Mat4, Vec3};

#[test]
fn debug_perspective_frustum() {
    // Exact setup from failing test
    let view = Mat4::look_at_rh(
        Vec3::new(0.0, 0.0, 5.0), // Camera at +Z
        Vec3::ZERO,               // Looking at origin
        Vec3::Y,                  // Up is +Y
    );
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
    let view_proj = proj * view;

    println!("\n=== Camera Setup ===");
    println!("Position: (0, 0, 5)");
    println!("Target: (0, 0, 0)");
    println!("View direction: (0, 0, -1)");
    println!("FOV: 90°, Aspect: 1.0, Near: 0.1, Far: 100.0");

    println!("\n=== View Matrix ===");
    let view_cols = view.to_cols_array();
    println!("{:?}", &view_cols[0..4]);
    println!("{:?}", &view_cols[4..8]);
    println!("{:?}", &view_cols[8..12]);
    println!("{:?}", &view_cols[12..16]);

    println!("\n=== Projection Matrix ===");
    let proj_cols = proj.to_cols_array();
    println!("{:?}", &proj_cols[0..4]);
    println!("{:?}", &proj_cols[4..8]);
    println!("{:?}", &proj_cols[8..12]);
    println!("{:?}", &proj_cols[12..16]);

    println!("\n=== View-Projection Matrix ===");
    let vp_cols = view_proj.to_cols_array();
    println!("{:?}", &vp_cols[0..4]);
    println!("{:?}", &vp_cols[4..8]);
    println!("{:?}", &vp_cols[8..12]);
    println!("{:?}", &vp_cols[12..16]);

    let frustum = FrustumPlanes::from_view_proj(&view_proj);

    println!("\n=== Frustum Planes (n, d) ===");
    let plane_names = ["Left", "Right", "Bottom", "Top", "Near", "Far"];
    for (i, name) in plane_names.iter().enumerate() {
        let p = frustum.planes[i];
        let normal = Vec3::new(p[0], p[1], p[2]);
        println!(
            "{:7}: n=({:7.4}, {:7.4}, {:7.4}), d={:7.4}, |n|={:.6}",
            name,
            p[0],
            p[1],
            p[2],
            p[3],
            normal.length()
        );
    }

    println!("\n=== Test Instances ===");
    let instances = vec![
        InstanceAABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::splat(0.5), 0),
        InstanceAABB::new(Vec3::new(0.0, 0.0, -50.0), Vec3::splat(0.5), 1),
        InstanceAABB::new(Vec3::new(50.0, 0.0, 0.0), Vec3::splat(0.5), 2),
        InstanceAABB::new(Vec3::new(0.0, 50.0, 0.0), Vec3::splat(0.5), 3),
        InstanceAABB::new(Vec3::new(0.0, 0.0, 10.0), Vec3::splat(0.5), 4),
    ];

    let descriptions = [
        "At origin (should be visible)",
        "At (0, 0, -50) in front (should be visible)",
        "At (50, 0, 0) far right (should be culled)",
        "At (0, 50, 0) far up (should be culled)",
        "At (0, 0, 10) behind camera (should be culled)",
    ];

    for (idx, inst) in instances.iter().enumerate() {
        let center = Vec3::from_slice(&inst.center);
        let extent = Vec3::from_slice(&inst.extent);

        println!("\nInstance {}: {}", idx, descriptions[idx]);
        println!(
            "  Center: ({:.1}, {:.1}, {:.1}), Extent: {:.1}",
            center.x, center.y, center.z, extent.x
        );

        // Manually test each plane
        let mut all_pass = true;
        for (i, name) in plane_names.iter().enumerate() {
            let plane = frustum.planes[i];
            let normal = Vec3::new(plane[0], plane[1], plane[2]);
            let d = plane[3];

            let dist = normal.dot(center) + d;
            let radius = extent.x.abs() * normal.x.abs()
                + extent.y.abs() * normal.y.abs()
                + extent.z.abs() * normal.z.abs();

            let passes = dist >= -radius;
            if !passes {
                all_pass = false;
            }

            println!(
                "  {:7}: dist={:7.3}, radius={:7.3}, passes={}",
                name, dist, radius, passes
            );
        }

        let visible = frustum.test_aabb(center, extent);
        println!("  Result: {}", if visible { "VISIBLE" } else { "CULLED" });

        assert_eq!(visible, all_pass, "Manual test doesn't match test_aabb()");
    }

    println!("\n=== CPU Culling Result ===");
    let visible_indices = cpu_frustum_cull(&instances, &frustum);
    println!("Visible instances: {:?}", visible_indices);

    // Expected: [0, 1] (origin and instance in front)
    assert_eq!(visible_indices.len(), 2, "Expected 2 visible instances");
    assert!(visible_indices.contains(&0), "Instance 0 should be visible");
    assert!(visible_indices.contains(&1), "Instance 1 should be visible");
}

#[test]
fn debug_view_space_coordinates() {
    // Transform test points to view space to understand the coordinate system
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);

    let test_points = vec![
        ("Origin", Vec3::ZERO),
        ("In front (-50Z)", Vec3::new(0.0, 0.0, -50.0)),
        ("Behind (+10Z)", Vec3::new(0.0, 0.0, 10.0)),
        ("Camera pos", Vec3::new(0.0, 0.0, 5.0)),
    ];

    println!("\n=== View Space Coordinates ===");
    for (name, world_pos) in test_points {
        let view_pos = view.transform_point3(world_pos);
        println!(
            "{:20} World: {:7.2?}  ->  View: {:7.2?}",
            name, world_pos, view_pos
        );
    }

    // In RH view space:
    // - Camera looks down -Z
    // - Origin (0,0,0) at camera+5Z should transform to (0,0,-5) in view space
    // - Point at (0,0,-50) should transform to (0,0,+55) but we want negative Z in front

    println!("\n=== Expected Behavior ===");
    println!("RH convention: camera looks down -Z in view space");
    println!("Points in front of camera should have NEGATIVE Z in view space");
}
