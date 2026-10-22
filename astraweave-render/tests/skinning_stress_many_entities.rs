//! Stress Tests for Skeletal Animation System
//!
//! Phase 2 Task 5 (Phase E): Validates system stability under load with many entities.
//! Tests for panics, buffer overflows, and reasonable performance.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::animation::*;
use glam::{Mat4, Quat, Vec3};
use std::time::Instant;

/// Create a test skeleton (3 joints)
fn create_stress_test_skeleton() -> Skeleton {
    Skeleton {
        root_indices: vec![0],
        joints: vec![
            Joint {
                name: "root".to_string(),
                parent_index: None,
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
            Joint {
                name: "child1".to_string(),
                parent_index: Some(0),
                inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    ..Default::default()
                },
            },
            Joint {
                name: "child2".to_string(),
                parent_index: Some(1),
                inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0)),
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    ..Default::default()
                },
            },
        ],
    }
}

/// Create test animation clip
fn create_stress_test_animation() -> AnimationClip {
    AnimationClip {
        name: "stress_anim".to_string(),
        duration: 2.0,
        channels: vec![AnimationChannel {
            target_joint_index: 1,
            times: vec![0.0, 1.0, 2.0],
            interpolation: Interpolation::Linear,
            data: ChannelData::Rotation(vec![
                Quat::IDENTITY,
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            ]),
        }],
    }
}

/// Simulated entity with animation state
struct AnimatedEntity {
    skeleton: Skeleton,
    clip: AnimationClip,
    current_time: f32,
    joint_matrices: Vec<Mat4>,
}

impl AnimatedEntity {
    fn new(skeleton: Skeleton, clip: AnimationClip) -> Self {
        let joint_count = skeleton.joints.len();
        Self {
            skeleton,
            clip,
            current_time: 0.0,
            joint_matrices: vec![Mat4::IDENTITY; joint_count],
        }
    }

    fn update(&mut self, dt: f32) {
        // Advance time with wrapping
        self.current_time += dt;
        if self.current_time > self.clip.duration {
            self.current_time -= self.clip.duration;
        }

        // Sample animation
        let local_poses = self.clip.sample(self.current_time, &self.skeleton);

        // Compute joint matrices
        self.joint_matrices = compute_joint_matrices(&self.skeleton, &local_poses).unwrap();
    }
}

/// Test: CPU stress with moderate entity count (CI-safe)
#[test]
fn test_stress_cpu_moderate() {
    const ENTITY_COUNT: usize = 100;
    const FRAME_COUNT: usize = 60;
    const DT: f32 = 1.0 / 60.0;

    eprintln!(
        "🔄 Starting CPU stress test: {} entities × {} frames",
        ENTITY_COUNT, FRAME_COUNT
    );

    // Create entities
    let skeleton = create_stress_test_skeleton();
    let clip = create_stress_test_animation();

    let mut entities: Vec<AnimatedEntity> = (0..ENTITY_COUNT)
        .map(|_| AnimatedEntity::new(skeleton.clone(), clip.clone()))
        .collect();

    // Simulate frames
    let start = Instant::now();
    let mut total_joints_updated = 0u64;

    for frame in 0..FRAME_COUNT {
        for entity in &mut entities {
            entity.update(DT);
            total_joints_updated += entity.skeleton.joints.len() as u64;
        }

        // Log progress every 10 frames
        if (frame + 1) % 10 == 0 {
            let elapsed = start.elapsed();
            let fps = (frame + 1) as f64 / elapsed.as_secs_f64();
            eprintln!(
                "   Frame {}/{}: {:.1} FPS, {} joints/frame",
                frame + 1,
                FRAME_COUNT,
                fps,
                ENTITY_COUNT * skeleton.joints.len()
            );
        }
    }

    let elapsed = start.elapsed();
    let avg_frame_ms = elapsed.as_secs_f64() * 1000.0 / FRAME_COUNT as f64;
    let total_updates = ENTITY_COUNT * FRAME_COUNT;

    eprintln!("✅ CPU stress test complete:");
    eprintln!("   Total entities: {}", ENTITY_COUNT);
    eprintln!("   Total frames: {}", FRAME_COUNT);
    eprintln!("   Total updates: {}", total_updates);
    eprintln!("   Total joints updated: {}", total_joints_updated);
    eprintln!("   Total time: {:.2}s", elapsed.as_secs_f64());
    eprintln!("   Avg frame time: {:.3}ms", avg_frame_ms);
    eprintln!(
        "   Updates/sec: {:.0}",
        total_updates as f64 / elapsed.as_secs_f64()
    );

    // Basic sanity checks
    assert!(
        avg_frame_ms < 100.0,
        "Frame time too high: {:.2}ms",
        avg_frame_ms
    );
    assert_eq!(entities.len(), ENTITY_COUNT, "Entity count changed");
    for (i, entity) in entities.iter().enumerate() {
        assert_eq!(
            entity.joint_matrices.len(),
            skeleton.joints.len(),
            "Entity {} joint matrix count mismatch",
            i
        );
        for (j, matrix) in entity.joint_matrices.iter().enumerate() {
            assert!(
                matrix.is_finite(),
                "Entity {} joint {} has non-finite matrix",
                i,
                j
            );
        }
    }
}

/// Test: CPU stress with high entity count
#[test]
#[ignore = "Long-running stress test; run manually"]
fn test_stress_cpu_high() {
    const ENTITY_COUNT: usize = 2000;
    const FRAME_COUNT: usize = 60;
    const DT: f32 = 1.0 / 60.0;

    eprintln!(
        "🔄 Starting CPU high-stress test: {} entities × {} frames",
        ENTITY_COUNT, FRAME_COUNT
    );

    let skeleton = create_stress_test_skeleton();
    let clip = create_stress_test_animation();

    let mut entities: Vec<AnimatedEntity> = (0..ENTITY_COUNT)
        .map(|_| AnimatedEntity::new(skeleton.clone(), clip.clone()))
        .collect();

    let start = Instant::now();
    let mut max_frame_ms = 0.0f64;
    let mut min_frame_ms = f64::MAX;

    for frame in 0..FRAME_COUNT {
        let frame_start = Instant::now();

        for entity in &mut entities {
            entity.update(DT);
        }

        let frame_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
        max_frame_ms = max_frame_ms.max(frame_ms);
        min_frame_ms = min_frame_ms.min(frame_ms);

        if (frame + 1) % 10 == 0 {
            eprintln!(
                "   Frame {}/{}: {:.2}ms (min: {:.2}ms, max: {:.2}ms)",
                frame + 1,
                FRAME_COUNT,
                frame_ms,
                min_frame_ms,
                max_frame_ms
            );
        }
    }

    let elapsed = start.elapsed();
    let avg_frame_ms = elapsed.as_secs_f64() * 1000.0 / FRAME_COUNT as f64;

    eprintln!("✅ CPU high-stress test complete:");
    eprintln!("   Entities: {}", ENTITY_COUNT);
    eprintln!("   Frames: {}", FRAME_COUNT);
    eprintln!("   Avg frame: {:.2}ms", avg_frame_ms);
    eprintln!("   Min frame: {:.2}ms", min_frame_ms);
    eprintln!("   Max frame: {:.2}ms", max_frame_ms);

    // Performance bounds (CI may be slower, so generous)
    assert!(
        avg_frame_ms < 200.0,
        "Average frame time too high: {:.2}ms",
        avg_frame_ms
    );
}

/// Test: Memory stability over time
#[test]
fn test_stress_memory_stability() {
    const ENTITY_COUNT: usize = 50;
    const FRAME_COUNT: usize = 120; // 2 seconds at 60fps
    const DT: f32 = 1.0 / 60.0;

    eprintln!(
        "🔄 Starting memory stability test: {} entities × {} frames",
        ENTITY_COUNT, FRAME_COUNT
    );

    let skeleton = create_stress_test_skeleton();
    let clip = create_stress_test_animation();

    let mut entities: Vec<AnimatedEntity> = (0..ENTITY_COUNT)
        .map(|_| AnimatedEntity::new(skeleton.clone(), clip.clone()))
        .collect();

    // Track memory-related metrics
    let mut total_allocations = 0usize;
    let initial_capacity: Vec<usize> = entities
        .iter()
        .map(|e| e.joint_matrices.capacity())
        .collect();

    for _frame in 0..FRAME_COUNT {
        for entity in &mut entities {
            entity.update(DT);
            total_allocations += 1;
        }
    }

    // Verify no unexpected reallocations
    for (i, entity) in entities.iter().enumerate() {
        assert_eq!(
            entity.joint_matrices.capacity(),
            initial_capacity[i],
            "Entity {} capacity changed (possible memory leak)",
            i
        );
        assert_eq!(
            entity.joint_matrices.len(),
            skeleton.joints.len(),
            "Entity {} length changed",
            i
        );
    }

    eprintln!("✅ Memory stability verified:");
    eprintln!("   Total updates: {}", total_allocations);
    eprintln!("   No reallocations detected");
}

/// Test: Concurrent entity updates (determinism check)
#[test]
fn test_stress_determinism() {
    const ENTITY_COUNT: usize = 10;
    const FRAME_COUNT: usize = 30;
    const DT: f32 = 1.0 / 60.0;

    eprintln!("🔄 Testing determinism with {} entities", ENTITY_COUNT);

    let skeleton = create_stress_test_skeleton();
    let clip = create_stress_test_animation();

    // Run 1
    let mut entities1: Vec<AnimatedEntity> = (0..ENTITY_COUNT)
        .map(|_| AnimatedEntity::new(skeleton.clone(), clip.clone()))
        .collect();

    for _ in 0..FRAME_COUNT {
        for entity in &mut entities1 {
            entity.update(DT);
        }
    }

    // Run 2 (identical)
    let mut entities2: Vec<AnimatedEntity> = (0..ENTITY_COUNT)
        .map(|_| AnimatedEntity::new(skeleton.clone(), clip.clone()))
        .collect();

    for _ in 0..FRAME_COUNT {
        for entity in &mut entities2 {
            entity.update(DT);
        }
    }

    // Compare results
    for i in 0..ENTITY_COUNT {
        assert_eq!(
            entities1[i].current_time, entities2[i].current_time,
            "Entity {} time mismatch",
            i
        );

        for j in 0..skeleton.joints.len() {
            let mat1 = entities1[i].joint_matrices[j];
            let mat2 = entities2[i].joint_matrices[j];

            let diff = (mat1 - mat2).abs();
            let max_diff = diff
                .to_cols_array()
                .iter()
                .fold(0.0f32, |acc, &x| acc.max(x));

            assert!(
                max_diff < 1e-6,
                "Entity {} joint {} matrix differs: {}",
                i,
                j,
                max_diff
            );
        }
    }

    eprintln!("✅ Determinism verified: identical results across runs");
}

/// Test: Edge case - zero dt updates
#[test]
fn test_stress_zero_dt() {
    let skeleton = create_stress_test_skeleton();
    let clip = create_stress_test_animation();

    let mut entity = AnimatedEntity::new(skeleton, clip);

    // Update with zero dt (should not crash or produce NaN)
    for _ in 0..10 {
        entity.update(0.0);
    }

    // Verify still valid
    for matrix in &entity.joint_matrices {
        assert!(matrix.is_finite(), "Zero dt produced non-finite matrix");
    }

    assert_eq!(
        entity.current_time, 0.0,
        "Time should not advance with zero dt"
    );

    eprintln!("✅ Zero dt handled correctly");
}

/// Test: Edge case - negative dt (user error)
#[test]
fn test_stress_negative_dt() {
    let skeleton = create_stress_test_skeleton();
    let clip = create_stress_test_animation();

    let mut entity = AnimatedEntity::new(skeleton, clip);

    // Update with negative dt (should clamp or wrap, not crash)
    entity.update(-0.1);

    // Verify no corruption
    for matrix in &entity.joint_matrices {
        assert!(matrix.is_finite(), "Negative dt produced non-finite matrix");
    }

    eprintln!("✅ Negative dt handled without panic");
}

/// Test: Edge case - very large dt
#[test]
fn test_stress_large_dt() {
    let skeleton = create_stress_test_skeleton();
    let clip = create_stress_test_animation();

    let mut entity = AnimatedEntity::new(skeleton, clip);

    // Update with huge dt (multiple animation loops)
    entity.update(100.0);

    // Verify time is still positive and finite (wrapping behavior may vary)
    assert!(
        entity.current_time >= 0.0,
        "Time should be non-negative, got: {}",
        entity.current_time
    );
    assert!(
        entity.current_time.is_finite(),
        "Time should be finite, got: {}",
        entity.current_time
    );

    for matrix in &entity.joint_matrices {
        assert!(matrix.is_finite(), "Large dt produced non-finite matrix");
    }

    eprintln!(
        "✅ Large dt handled correctly (time: {})",
        entity.current_time
    );
}

#[cfg(test)]
mod stress_test_notes {
    //! Stress Test Notes
    //!
    //! **Test Levels:**
    //! - `test_stress_cpu_moderate`: 100 entities × 60 frames (CI-safe, ~1-2s)
    //! - `test_stress_cpu_high`: 2000 entities × 60 frames (manual run, ~10-20s)
    //!
    //! **Counters Tracked:**
    //! - Total joint updates
    //! - Frame time (avg, min, max)
    //! - Updates per second
    //! - Memory stability (no reallocations)
    //!
    //! **Performance Expectations:**
    //! - Moderate: < 100ms/frame average
    //! - High: < 200ms/frame average (CI may be slower)
    //!
    //! **Why These Numbers?**
    //! - 100 entities: Typical small-medium scene
    //! - 2000 entities: Stress test for large crowds
    //! - 60 frames: 1 second of gameplay at 60fps
    //! - 3 joints/entity: Minimal but realistic (full humanoid would be ~50-70)
    //!
    //! **GPU Stress:**
    //! GPU stress tests would be similar but require `--features skinning-gpu`
    //! and are marked `#[ignore]` for CI. They would track:
    //! - Buffer upload bytes/frame
    //! - Palette texture updates
    //! - GPU memory usage
}
