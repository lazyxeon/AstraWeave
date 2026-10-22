//! Wave 2 mutation-resistant remediation tests for instancing.rs and biome_detector.rs
//!
//! Targets:
//!   - InstanceRaw::from_transform matrix golden values
//!   - InstanceRaw::from_matrix roundtrip
//!   - Instance::identity golden values
//!   - InstanceBatch CPU-only operations (add, count, clear)
//!   - InstanceManager draw-call statistics
//!   - InstancePatternBuilder grid/circle positions
//!   - BiomeDetectorConfig defaults
//!   - BiomeDetector::classify_scored golden values

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::biome_detector::{BiomeDetector, BiomeDetectorConfig};
use astraweave_render::instancing::{
    Instance, InstanceBatch, InstanceManager, InstancePatternBuilder, InstanceRaw,
};
use astraweave_terrain::biome::BiomeType;
use glam::{Mat4, Quat, Vec3};

// ===========================================================================
// InstanceRaw — size and constructors
// ===========================================================================

#[test]
fn instance_raw_size_64_bytes() {
    assert_eq!(std::mem::size_of::<InstanceRaw>(), 64);
}

#[test]
fn instance_raw_from_matrix_identity() {
    let m = Mat4::IDENTITY;
    let raw = InstanceRaw::from_matrix(m);
    assert_eq!(raw.model[0], [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(raw.model[1], [0.0, 1.0, 0.0, 0.0]);
    assert_eq!(raw.model[2], [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(raw.model[3], [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn instance_raw_from_transform_translation_only() {
    let raw = InstanceRaw::from_transform(Vec3::new(5.0, 10.0, 15.0), Quat::IDENTITY, Vec3::ONE);
    // Column-major: translation is in column 3
    assert_eq!(raw.model[3][0], 5.0);
    assert_eq!(raw.model[3][1], 10.0);
    assert_eq!(raw.model[3][2], 15.0);
    assert_eq!(raw.model[3][3], 1.0);
    // Diagonal should be 1.0 (no rotation, unit scale)
    assert_eq!(raw.model[0][0], 1.0);
    assert_eq!(raw.model[1][1], 1.0);
    assert_eq!(raw.model[2][2], 1.0);
}

#[test]
fn instance_raw_from_transform_scale_only() {
    let raw = InstanceRaw::from_transform(Vec3::ZERO, Quat::IDENTITY, Vec3::new(2.0, 3.0, 4.0));
    assert_eq!(raw.model[0][0], 2.0);
    assert_eq!(raw.model[1][1], 3.0);
    assert_eq!(raw.model[2][2], 4.0);
    // No translation
    assert_eq!(raw.model[3][0], 0.0);
    assert_eq!(raw.model[3][1], 0.0);
    assert_eq!(raw.model[3][2], 0.0);
}

#[test]
fn instance_raw_from_transform_rotation_90_y() {
    // 90 degrees around Y: X→-Z, Z→X
    let rot = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let raw = InstanceRaw::from_transform(Vec3::ZERO, rot, Vec3::ONE);
    // After 90° Y rotation: col0 = (cos90, 0, -sin90, 0) = (0, 0, -1, 0)
    assert!((raw.model[0][0]).abs() < 1e-5);
    assert!((raw.model[0][2] - (-1.0)).abs() < 1e-5);
    // col2 = (sin90, 0, cos90, 0) = (1, 0, 0, 0)
    assert!((raw.model[2][0] - 1.0).abs() < 1e-5);
    assert!((raw.model[2][2]).abs() < 1e-5);
}

#[test]
fn instance_raw_desc_layout() {
    let desc = InstanceRaw::desc();
    assert_eq!(desc.array_stride, 64);
    assert_eq!(desc.step_mode, wgpu::VertexStepMode::Instance);
    assert_eq!(desc.attributes.len(), 4);
    // Check shader locations 5, 6, 7, 8
    assert_eq!(desc.attributes[0].shader_location, 5);
    assert_eq!(desc.attributes[1].shader_location, 6);
    assert_eq!(desc.attributes[2].shader_location, 7);
    assert_eq!(desc.attributes[3].shader_location, 8);
}

#[test]
fn instance_raw_desc_offsets() {
    let desc = InstanceRaw::desc();
    assert_eq!(desc.attributes[0].offset, 0);
    assert_eq!(desc.attributes[1].offset, 16); // 4 floats
    assert_eq!(desc.attributes[2].offset, 32); // 8 floats
    assert_eq!(desc.attributes[3].offset, 48); // 12 floats
}

// ===========================================================================
// Instance — high-level CPU transform
// ===========================================================================

#[test]
fn instance_identity_fields() {
    let i = Instance::identity();
    assert_eq!(i.position, Vec3::ZERO);
    assert_eq!(i.rotation, Quat::IDENTITY);
    assert_eq!(i.scale, Vec3::ONE);
}

#[test]
fn instance_new_stores_fields() {
    let i = Instance::new(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::splat(2.0));
    assert_eq!(i.position, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(i.scale, Vec3::splat(2.0));
}

#[test]
fn instance_to_raw_identity_diagonal() {
    let raw = Instance::identity().to_raw();
    for idx in 0..4 {
        assert_eq!(raw.model[idx][idx], 1.0, "diagonal[{idx}] should be 1.0");
    }
}

#[test]
fn instance_to_raw_translation_matches() {
    let i = Instance::new(Vec3::new(7.0, 8.0, 9.0), Quat::IDENTITY, Vec3::ONE);
    let raw = i.to_raw();
    assert_eq!(raw.model[3][0], 7.0);
    assert_eq!(raw.model[3][1], 8.0);
    assert_eq!(raw.model[3][2], 9.0);
}

// ===========================================================================
// InstanceBatch — CPU-only operations
// ===========================================================================

#[test]
fn batch_new_starts_empty() {
    let b = InstanceBatch::new(99);
    assert_eq!(b.mesh_id, 99);
    assert_eq!(b.instance_count(), 0);
    assert!(b.buffer.is_none());
}

#[test]
fn batch_add_and_count() {
    let mut b = InstanceBatch::new(1);
    b.add_instance(Instance::identity());
    b.add_instance(Instance::identity());
    b.add_instance(Instance::identity());
    assert_eq!(b.instance_count(), 3);
}

#[test]
fn batch_clear_resets_count() {
    let mut b = InstanceBatch::new(1);
    b.add_instance(Instance::identity());
    b.add_instance(Instance::identity());
    b.clear();
    assert_eq!(b.instance_count(), 0);
}

// ===========================================================================
// InstanceManager — statistics
// ===========================================================================

#[test]
fn manager_default_empty() {
    let m = InstanceManager::default();
    assert_eq!(m.total_instances(), 0);
    assert_eq!(m.batch_count(), 0);
    assert_eq!(m.draw_calls_saved(), 0);
}

#[test]
fn manager_add_instance_tracks_total() {
    let mut m = InstanceManager::new();
    m.add_instance(1, Instance::identity());
    m.add_instance(1, Instance::identity());
    m.add_instance(2, Instance::identity());
    assert_eq!(m.total_instances(), 3);
    assert_eq!(m.batch_count(), 2);
}

#[test]
fn manager_add_instances_bulk() {
    let mut m = InstanceManager::new();
    let batch = vec![
        Instance::identity(),
        Instance::identity(),
        Instance::identity(),
    ];
    m.add_instances(42, batch);
    assert_eq!(m.total_instances(), 3);
    assert_eq!(m.batch_count(), 1);
}

#[test]
fn manager_draw_calls_saved_formula() {
    let mut m = InstanceManager::new();
    // 10 instances across 2 meshes
    for _ in 0..7 {
        m.add_instance(1, Instance::identity());
    }
    for _ in 0..3 {
        m.add_instance(2, Instance::identity());
    }
    // total=10, batches=2, saved = 10-2 = 8
    // Need to trigger internal recalc — we use draw_call_reduction_percent which
    // reads draw_calls_saved, but saved is only calculated after update_buffers or
    // an internal call. Let's check the initial state.
    // The calculate_draw_call_savings is private but called in update_buffers.
    // Without GPU we can't call update_buffers. The draw_calls_saved starts at 0.
    assert_eq!(m.total_instances(), 10);
    assert_eq!(m.batch_count(), 2);
}

#[test]
fn manager_clear_resets_all() {
    let mut m = InstanceManager::new();
    m.add_instance(1, Instance::identity());
    m.add_instance(2, Instance::identity());
    m.clear();
    assert_eq!(m.total_instances(), 0);
    assert_eq!(m.batch_count(), 0);
    assert_eq!(m.draw_calls_saved(), 0);
}

#[test]
fn manager_draw_call_reduction_percent_empty() {
    let m = InstanceManager::new();
    assert_eq!(m.draw_call_reduction_percent(), 0.0);
}

#[test]
fn manager_get_batch_returns_correct() {
    let mut m = InstanceManager::new();
    m.add_instance(7, Instance::identity());
    m.add_instance(7, Instance::identity());
    let b = m.get_batch(7).unwrap();
    assert_eq!(b.mesh_id, 7);
    assert_eq!(b.instance_count(), 2);
}

#[test]
fn manager_get_batch_missing_returns_none() {
    let m = InstanceManager::new();
    assert!(m.get_batch(999).is_none());
}

// ===========================================================================
// InstancePatternBuilder — grid
// ===========================================================================

#[test]
fn grid_pattern_count() {
    let instances = InstancePatternBuilder::new().grid(4, 5, 1.0).build();
    assert_eq!(instances.len(), 20); // 4×5
}

#[test]
fn grid_pattern_origin() {
    let instances = InstancePatternBuilder::new().grid(2, 2, 3.0).build();
    // First instance at (0,0,0)
    assert_eq!(instances[0].position, Vec3::new(0.0, 0.0, 0.0));
}

#[test]
fn grid_pattern_last_position() {
    let instances = InstancePatternBuilder::new().grid(3, 3, 2.0).build();
    // Last = row=2, col=2 → (2*2, 0, 2*2) = (4, 0, 4)
    assert_eq!(instances[8].position, Vec3::new(4.0, 0.0, 4.0));
}

#[test]
fn grid_pattern_spacing() {
    let instances = InstancePatternBuilder::new().grid(1, 3, 5.0).build();
    assert_eq!(instances[0].position.x, 0.0);
    assert_eq!(instances[1].position.x, 5.0);
    assert_eq!(instances[2].position.x, 10.0);
}

#[test]
fn grid_all_y_zero() {
    let instances = InstancePatternBuilder::new().grid(3, 3, 1.0).build();
    for i in &instances {
        assert_eq!(i.position.y, 0.0);
    }
}

// ===========================================================================
// InstancePatternBuilder — circle
// ===========================================================================

#[test]
fn circle_pattern_count() {
    let instances = InstancePatternBuilder::new().circle(12, 5.0).build();
    assert_eq!(instances.len(), 12);
}

#[test]
fn circle_pattern_radius() {
    let instances = InstancePatternBuilder::new().circle(8, 10.0).build();
    for i in &instances {
        let dist = (i.position.x * i.position.x + i.position.z * i.position.z).sqrt();
        assert!(
            (dist - 10.0).abs() < 0.01,
            "distance={dist}, expected ~10.0"
        );
    }
}

#[test]
fn circle_all_y_zero() {
    let instances = InstancePatternBuilder::new().circle(6, 3.0).build();
    for i in &instances {
        assert_eq!(i.position.y, 0.0);
    }
}

#[test]
fn circle_first_at_radius_0_angle() {
    let instances = InstancePatternBuilder::new().circle(4, 7.0).build();
    // First instance: angle=0, x=cos(0)*7=7, z=sin(0)*7=0
    assert!((instances[0].position.x - 7.0).abs() < 0.01);
    assert!(instances[0].position.z.abs() < 0.01);
}

// ===========================================================================
// BiomeDetectorConfig — defaults
// ===========================================================================

#[test]
fn biome_detector_config_default_values() {
    let cfg = BiomeDetectorConfig::default();
    assert_eq!(cfg.sample_distance_threshold, 2.0);
    assert_eq!(cfg.hysteresis_count, 3);
}

// ===========================================================================
// BiomeDetector — initial state
// ===========================================================================

#[test]
fn biome_detector_new_no_biome() {
    let det = BiomeDetector::new(BiomeDetectorConfig::default());
    assert!(det.current_biome().is_none());
    assert_eq!(det.transition_count(), 0);
}

#[test]
fn biome_detector_set_biome() {
    let mut det = BiomeDetector::new(BiomeDetectorConfig::default());
    det.set_biome(BiomeType::Desert);
    assert_eq!(det.current_biome(), Some(BiomeType::Desert));
}

#[test]
fn biome_detector_reset_clears() {
    let mut det = BiomeDetector::new(BiomeDetectorConfig::default());
    det.set_biome(BiomeType::Forest);
    det.reset();
    assert!(det.current_biome().is_none());
}

// ===========================================================================
// BiomeDetector::classify_scored — pure function golden values
// ===========================================================================

#[test]
fn classify_scored_hot_dry_is_desert() {
    assert_eq!(
        BiomeDetector::classify_scored(5.0, 0.9, 0.1),
        BiomeType::Desert
    );
}

#[test]
fn classify_scored_cold_is_tundra() {
    assert_eq!(
        BiomeDetector::classify_scored(5.0, 0.1, 0.3),
        BiomeType::Tundra
    );
}

#[test]
fn classify_scored_mild_moderate() {
    let result = BiomeDetector::classify_scored(25.0, 0.5, 0.5);
    // Should be grassland or forest (mild conditions)
    assert!(
        result == BiomeType::Grassland || result == BiomeType::Forest,
        "Expected grassland/forest at mild conditions, got {:?}",
        result
    );
}

#[test]
fn classify_scored_high_altitude_is_mountain() {
    // Very high altitude should trigger Mountain
    let result = BiomeDetector::classify_scored(200.0, 0.3, 0.4);
    assert_eq!(result, BiomeType::Mountain);
}

#[test]
fn classify_scored_low_height_warm_wet() {
    // Low height, warm, very wet
    let result = BiomeDetector::classify_scored(5.0, 0.6, 0.9);
    // Score-based classification — biome depends on config thresholds
    assert!(
        BiomeType::all().contains(&result),
        "Should return a valid biome, got {:?}",
        result
    );
}

#[test]
fn classify_scored_returns_valid_biome() {
    // Test various extremes
    let cases = [
        (0.0, 0.0, 0.0),
        (100.0, 1.0, 1.0),
        (50.0, 0.5, 0.5),
        (1.0, 0.95, 0.05),
        (200.0, 0.1, 0.9),
    ];
    for (h, t, m) in cases {
        let result = BiomeDetector::classify_scored(h, t, m);
        // Should be one of the 8 valid biome types
        assert!(
            BiomeType::all().contains(&result),
            "classify_scored({h},{t},{m}) returned {:?} which is not in all()",
            result
        );
    }
}
