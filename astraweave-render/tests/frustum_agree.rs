//! CPU frustum tests and the culling module's CPU path accept the same boxes.
//! Neither result is simulation state.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::culling::{cpu_frustum_cull, FrustumPlanes, InstanceAABB};
use glam::Mat4;

#[test]
fn cpu_frustum_path_matches_aabb_tests() {
    let view_proj = Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, 0.1, 10.0);
    let frustum = FrustumPlanes::from_view_proj(&view_proj);
    let instances = vec![
        InstanceAABB::new(glam::Vec3::new(0.0, 0.0, -1.0), glam::Vec3::splat(0.2), 0),
        InstanceAABB::new(glam::Vec3::new(50.0, 0.0, -1.0), glam::Vec3::splat(0.2), 1),
    ];
    let visible = cpu_frustum_cull(&instances, &frustum);
    for aabb in &instances {
        let center = glam::Vec3::from_array(aabb.center);
        let extent = glam::Vec3::from_array(aabb.extent);
        let inside = frustum.test_aabb(center, extent);
        assert_eq!(inside, visible.contains(&aabb.instance_index));
    }
    assert!(visible.contains(&0));
    assert!(!visible.contains(&1));
}
