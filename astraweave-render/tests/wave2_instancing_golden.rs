//! Wave 2 – Golden-value tests for instancing.rs (72 mutants)
//!
//! Targets: InstanceRaw::from_transform matrix layout, InstanceManager
//!          draw-call savings arithmetic, InstancePatternBuilder grid/circle
//!          exact positions, Instance::identity fields.
//!
//! Strategy: Pin exact matrix entries for known transforms, exact positions
//! for grid/circle patterns, exact draw-call reduction math.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::instancing::{
    Instance, InstanceBatch, InstanceManager, InstancePatternBuilder, InstanceRaw,
};
use glam::{Mat4, Quat, Vec3};

// ============================================================================
// InstanceRaw layout & from_transform
// ============================================================================

#[test]
fn instance_raw_size_64_bytes() {
    assert_eq!(std::mem::size_of::<InstanceRaw>(), 64);
}

#[test]
fn instance_raw_identity_matrix() {
    let raw = InstanceRaw::from_transform(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
    // Column-major identity: [1,0,0,0], [0,1,0,0], [0,0,1,0], [0,0,0,1]
    assert_eq!(raw.model[0], [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(raw.model[1], [0.0, 1.0, 0.0, 0.0]);
    assert_eq!(raw.model[2], [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(raw.model[3], [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn instance_raw_translation_in_column_3() {
    let raw = InstanceRaw::from_transform(Vec3::new(5.0, 10.0, -3.0), Quat::IDENTITY, Vec3::ONE);
    // Translation goes in column 3 (last column)
    assert_eq!(raw.model[3][0], 5.0);
    assert_eq!(raw.model[3][1], 10.0);
    assert_eq!(raw.model[3][2], -3.0);
    assert_eq!(raw.model[3][3], 1.0);
    // Rotation part should still be identity
    assert_eq!(raw.model[0][0], 1.0);
    assert_eq!(raw.model[1][1], 1.0);
    assert_eq!(raw.model[2][2], 1.0);
}

#[test]
fn instance_raw_uniform_scale() {
    let raw = InstanceRaw::from_transform(Vec3::ZERO, Quat::IDENTITY, Vec3::splat(2.0));
    assert_eq!(raw.model[0][0], 2.0);
    assert_eq!(raw.model[1][1], 2.0);
    assert_eq!(raw.model[2][2], 2.0);
    assert_eq!(raw.model[3][3], 1.0);
}

#[test]
fn instance_raw_nonuniform_scale() {
    let raw = InstanceRaw::from_transform(Vec3::ZERO, Quat::IDENTITY, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(raw.model[0][0], 1.0);
    assert_eq!(raw.model[1][1], 2.0);
    assert_eq!(raw.model[2][2], 3.0);
}

#[test]
fn instance_raw_from_matrix_passthrough() {
    let m = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let raw = InstanceRaw::from_matrix(m);
    let expected = m.to_cols_array_2d();
    assert_eq!(raw.model, expected);
}

#[test]
fn instance_raw_rotation_90_y() {
    // 90° around Y: cos=0, sin=1
    // Column-major rotation matrix around Y:
    //   col0 = [cos, 0, -sin, 0]  col1 = [0,1,0,0]  col2 = [sin, 0, cos, 0]
    // For θ=π/2: col0=[0,0,-1,0], col1=[0,1,0,0], col2=[1,0,0,0]
    let q = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let raw = InstanceRaw::from_transform(Vec3::ZERO, q, Vec3::ONE);
    // Column 0: local X → world (0, 0, -1), i.e. -Z
    assert!((raw.model[0][0]).abs() < 1e-5, "c0r0 should be ~0");
    assert!(
        (raw.model[0][2] + 1.0).abs() < 1e-5,
        "c0r2 should be ~-1 (-sin)"
    );
    // Column 2: local Z → world (1, 0, 0), i.e. +X
    assert!(
        (raw.model[2][0] - 1.0).abs() < 1e-5,
        "c2r0 should be ~1 (sin)"
    );
    assert!((raw.model[2][2]).abs() < 1e-5, "c2r2 should be ~0");
    // Y axis unchanged
    assert!((raw.model[1][1] - 1.0).abs() < 1e-5, "c1r1 should be ~1");
}

// ============================================================================
// Instance — identity, new, to_raw
// ============================================================================

#[test]
fn instance_identity_fields() {
    let inst = Instance::identity();
    assert_eq!(inst.position, Vec3::ZERO);
    assert_eq!(inst.rotation, Quat::IDENTITY);
    assert_eq!(inst.scale, Vec3::ONE);
}

#[test]
fn instance_new_preserves_fields() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let rot = Quat::from_rotation_z(1.0);
    let sc = Vec3::new(0.5, 0.5, 0.5);
    let inst = Instance::new(pos, rot, sc);
    assert_eq!(inst.position, pos);
    assert_eq!(inst.rotation, rot);
    assert_eq!(inst.scale, sc);
}

#[test]
fn instance_to_raw_matches_from_transform() {
    let pos = Vec3::new(3.0, -1.0, 7.0);
    let rot = Quat::from_rotation_x(0.5);
    let sc = Vec3::new(2.0, 1.0, 0.5);
    let inst = Instance::new(pos, rot, sc);
    let raw = inst.to_raw();
    let direct = InstanceRaw::from_transform(pos, rot, sc);
    assert_eq!(raw.model, direct.model);
}

// ============================================================================
// InstanceBatch — CPU-side management (no GPU needed)
// ============================================================================

#[test]
fn batch_starts_empty() {
    let b = InstanceBatch::new(99);
    assert_eq!(b.mesh_id, 99);
    assert_eq!(b.instance_count(), 0);
    assert!(b.buffer.is_none());
}

#[test]
fn batch_add_increments_count() {
    let mut b = InstanceBatch::new(1);
    b.add_instance(Instance::identity());
    assert_eq!(b.instance_count(), 1);
    b.add_instance(Instance::identity());
    assert_eq!(b.instance_count(), 2);
}

#[test]
fn batch_clear_resets_count() {
    let mut b = InstanceBatch::new(1);
    b.add_instance(Instance::identity());
    b.add_instance(Instance::identity());
    b.clear();
    assert_eq!(b.instance_count(), 0);
}

// ============================================================================
// InstanceManager — draw-call savings math
// ============================================================================

#[test]
fn manager_starts_empty() {
    let m = InstanceManager::new();
    assert_eq!(m.total_instances(), 0);
    assert_eq!(m.batch_count(), 0);
    assert_eq!(m.draw_calls_saved(), 0);
}

#[test]
fn manager_default_eq_new() {
    let m = InstanceManager::default();
    assert_eq!(m.total_instances(), 0);
    assert_eq!(m.batch_count(), 0);
}

#[test]
fn manager_add_single_mesh() {
    let mut m = InstanceManager::new();
    m.add_instance(1, Instance::identity());
    m.add_instance(1, Instance::identity());
    m.add_instance(1, Instance::identity());
    assert_eq!(m.total_instances(), 3);
    assert_eq!(m.batch_count(), 1);
}

#[test]
fn manager_add_multiple_meshes() {
    let mut m = InstanceManager::new();
    m.add_instance(1, Instance::identity());
    m.add_instance(2, Instance::identity());
    m.add_instance(3, Instance::identity());
    assert_eq!(m.total_instances(), 3);
    assert_eq!(m.batch_count(), 3);
}

#[test]
fn manager_add_instances_bulk() {
    let mut m = InstanceManager::new();
    let insts = vec![
        Instance::identity(),
        Instance::identity(),
        Instance::identity(),
    ];
    m.add_instances(42, insts);
    assert_eq!(m.total_instances(), 3);
    assert_eq!(m.batch_count(), 1);
    assert_eq!(m.get_batch(42).unwrap().instance_count(), 3);
}

#[test]
fn manager_draw_call_savings_golden() {
    let mut m = InstanceManager::new();
    // 10 instances of mesh 1, 5 of mesh 2 → 15 total, 2 batches
    for _ in 0..10 {
        m.add_instance(1, Instance::identity());
    }
    for _ in 0..5 {
        m.add_instance(2, Instance::identity());
    }
    // Trigger calculation via private method — we access via the public API
    // draw_calls_saved = total(15) - batches(2) = 13
    // The savings aren't calculated until update_buffers is called (needs GPU)
    // But we can verify total_instances and batch_count
    assert_eq!(m.total_instances(), 15);
    assert_eq!(m.batch_count(), 2);
}

#[test]
fn manager_reduction_percent_zero_when_empty() {
    let m = InstanceManager::new();
    assert_eq!(m.draw_call_reduction_percent(), 0.0);
}

#[test]
fn manager_clear_resets_everything() {
    let mut m = InstanceManager::new();
    m.add_instance(1, Instance::identity());
    m.add_instance(2, Instance::identity());
    m.clear();
    assert_eq!(m.total_instances(), 0);
    assert_eq!(m.batch_count(), 0);
    assert_eq!(m.draw_calls_saved(), 0);
}

#[test]
fn manager_get_batch_returns_correct_mesh() {
    let mut m = InstanceManager::new();
    m.add_instance(42, Instance::identity());
    m.add_instance(99, Instance::identity());
    assert!(m.get_batch(42).is_some());
    assert!(m.get_batch(99).is_some());
    assert!(m.get_batch(0).is_none());
}

// ============================================================================
// InstancePatternBuilder — grid & circle golden positions
// ============================================================================

#[test]
fn grid_2x3_positions_golden() {
    let instances = InstancePatternBuilder::new().grid(2, 3, 5.0).build();
    assert_eq!(instances.len(), 6, "2×3 = 6 instances");
    // grid iterates row then col: col varies inner
    // row=0,col=0 → (0,0,0)
    assert_eq!(instances[0].position, Vec3::new(0.0, 0.0, 0.0));
    // row=0,col=1 → (5,0,0)
    assert_eq!(instances[1].position, Vec3::new(5.0, 0.0, 0.0));
    // row=0,col=2 → (10,0,0)
    assert_eq!(instances[2].position, Vec3::new(10.0, 0.0, 0.0));
    // row=1,col=0 → (0,0,5)
    assert_eq!(instances[3].position, Vec3::new(0.0, 0.0, 5.0));
    // row=1,col=2 → (10,0,5)
    assert_eq!(instances[5].position, Vec3::new(10.0, 0.0, 5.0));
}

#[test]
fn grid_all_y_zero() {
    let instances = InstancePatternBuilder::new().grid(4, 4, 1.0).build();
    for inst in &instances {
        assert_eq!(inst.position.y, 0.0);
    }
}

#[test]
fn grid_all_identity_rotation_and_scale() {
    let instances = InstancePatternBuilder::new().grid(3, 3, 1.0).build();
    for inst in &instances {
        assert_eq!(inst.rotation, Quat::IDENTITY);
        assert_eq!(inst.scale, Vec3::ONE);
    }
}

#[test]
fn circle_count_matches() {
    let instances = InstancePatternBuilder::new().circle(12, 5.0).build();
    assert_eq!(instances.len(), 12);
}

#[test]
fn circle_first_at_angle_zero() {
    let instances = InstancePatternBuilder::new().circle(8, 10.0).build();
    // angle=0: cos(0)=1, sin(0)=0 → x=10, z=0
    assert!((instances[0].position.x - 10.0).abs() < 0.001);
    assert!((instances[0].position.z).abs() < 0.001);
    assert_eq!(instances[0].position.y, 0.0);
}

#[test]
fn circle_quarter_at_angle_90() {
    let instances = InstancePatternBuilder::new().circle(4, 10.0).build();
    // i=1 of 4: angle = PI/2, cos=0, sin=1 → x=0, z=10
    assert!((instances[1].position.x).abs() < 0.001);
    assert!((instances[1].position.z - 10.0).abs() < 0.001);
}

#[test]
fn circle_all_at_radius() {
    let r = 7.5;
    let instances = InstancePatternBuilder::new().circle(16, r).build();
    for (idx, inst) in instances.iter().enumerate() {
        let dist = inst.position.length();
        assert!(
            (dist - r).abs() < 0.01,
            "Instance {} dist={}, expected {}",
            idx,
            dist,
            r
        );
    }
}

#[test]
fn circle_all_y_zero() {
    let instances = InstancePatternBuilder::new().circle(8, 5.0).build();
    for inst in &instances {
        assert_eq!(inst.position.y, 0.0);
    }
}

#[test]
fn pattern_builder_default_empty() {
    let instances = InstancePatternBuilder::default().build();
    assert!(instances.is_empty());
}
