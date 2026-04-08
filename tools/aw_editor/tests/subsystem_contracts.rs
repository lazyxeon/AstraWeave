//! Contract tests for subsystem modules.
//!
//! These tests verify the contracts and invariants of the extracted subsystem
//! infrastructure (entity manager mutation tracking, generation counters, etc.)
//! without requiring a full EditorApp which lives in the binary crate.

use aw_editor_lib::command::{MoveEntityCommand, UndoStack};
use aw_editor_lib::entity_manager::{EditorEntity, EntityManager};

// ============================================================================
// EntityManager mutation_generation tracking
// ============================================================================

#[test]
fn entity_manager_mutation_generation_starts_at_zero() {
    let em = EntityManager::new();
    assert_eq!(em.mutation_generation(), 0);
}

#[test]
fn entity_manager_add_bumps_generation() {
    let mut em = EntityManager::new();
    let gen0 = em.mutation_generation();
    em.add(EditorEntity::new(1, "Entity1".to_string()));
    assert!(
        em.mutation_generation() > gen0,
        "add should bump generation"
    );
}

#[test]
fn entity_manager_remove_bumps_generation() {
    let mut em = EntityManager::new();
    em.add(EditorEntity::new(1, "Entity1".to_string()));
    let gen1 = em.mutation_generation();
    em.remove(1);
    assert!(
        em.mutation_generation() > gen1,
        "remove should bump generation"
    );
}

#[test]
fn entity_manager_get_mut_bumps_generation() {
    let mut em = EntityManager::new();
    em.add(EditorEntity::new(1, "Entity1".to_string()));
    let gen1 = em.mutation_generation();
    let _ = em.get_mut(1);
    assert!(
        em.mutation_generation() > gen1,
        "get_mut should bump generation"
    );
}

#[test]
fn entity_manager_get_does_not_bump_generation() {
    let mut em = EntityManager::new();
    em.add(EditorEntity::new(1, "Entity1".to_string()));
    let gen1 = em.mutation_generation();
    let _ = em.get(1);
    assert_eq!(
        em.mutation_generation(),
        gen1,
        "get (immutable) should not bump generation"
    );
}

#[test]
fn entity_manager_update_transform_bumps_generation() {
    let mut em = EntityManager::new();
    em.add(EditorEntity::new(1, "Entity1".to_string()));
    let gen1 = em.mutation_generation();
    em.update_transform(
        1,
        glam::Vec3::new(5.0, 0.0, 5.0),
        glam::Quat::IDENTITY,
        glam::Vec3::ONE,
    );
    // update_transform calls get_mut internally
    assert!(
        em.mutation_generation() > gen1,
        "update_transform should bump generation"
    );
}

#[test]
fn entity_manager_create_bumps_generation() {
    let mut em = EntityManager::new();
    let gen0 = em.mutation_generation();
    em.create("NewEntity".to_string());
    assert!(
        em.mutation_generation() > gen0,
        "create should bump generation"
    );
}

// ============================================================================
// Undo/redo command contracts
// ============================================================================

#[test]
fn undo_stack_execute_and_undo() {
    let mut stack = UndoStack::new(100);
    let mut world = astraweave_core::World::new();
    let entity = world.spawn(
        "Test",
        astraweave_core::IVec2 { x: 0, y: 0 },
        astraweave_core::Team { id: 0 },
        0,
        0,
    );

    let cmd = MoveEntityCommand::new(
        entity,
        astraweave_core::IVec2 { x: 0, y: 0 },
        astraweave_core::IVec2 { x: 5, y: 5 },
    );
    assert!(stack.execute(cmd, &mut world, None).is_ok());
    assert_eq!(stack.undo_count(), 1);
    assert_eq!(stack.redo_count(), 0);

    assert!(stack.undo(&mut world, None).is_ok());
    assert_eq!(stack.undo_count(), 0);
    assert_eq!(stack.redo_count(), 1);
}

#[test]
fn undo_stack_clear_resets_state() {
    let mut stack = UndoStack::new(100);
    let mut world = astraweave_core::World::new();
    let entity = world.spawn(
        "Test",
        astraweave_core::IVec2 { x: 0, y: 0 },
        astraweave_core::Team { id: 0 },
        0,
        0,
    );
    let cmd = MoveEntityCommand::new(
        entity,
        astraweave_core::IVec2 { x: 0, y: 0 },
        astraweave_core::IVec2 { x: 1, y: 1 },
    );
    let _ = stack.execute(cmd, &mut world, None);
    stack.clear();
    assert_eq!(stack.undo_count(), 0);
    assert_eq!(stack.redo_count(), 0);
}

// ============================================================================
// Scene stats panel contract
// ============================================================================

#[test]
fn scene_stats_struct_has_all_fields() {
    // Verify SceneStats can be constructed with all fields
    use aw_editor_lib::panels::SceneStats;
    let stats = SceneStats {
        entity_count: 10,
        selected_count: 2,
        component_count: 30,
        prefab_count: 1,
        undo_stack_size: 5,
        redo_stack_size: 0,
        memory_estimate_kb: 1024,
        scene_path: Some("/test/scene.ron".to_string()),
        is_dirty: true,
        mesh_count: 8,
        total_triangles: 5000,
        total_vertices: 3000,
        mesh_memory_kb: 512,
        texture_count: 4,
        texture_memory_kb: 256,
        max_texture_resolution: (2048, 2048),
        material_count: 3,
        unique_shader_count: 4,
        estimated_draw_calls: 12,
        estimated_state_changes: 7,
        performance_warning: None,
    };
    assert_eq!(stats.entity_count, 10);
    assert!(stats.is_dirty);
}

// ============================================================================
// GPU metrics contract
// ============================================================================

#[test]
fn gpu_metrics_default_is_zero() {
    use aw_editor_lib::panels::profiler_panel::GpuMetrics;
    let m = GpuMetrics::default();
    assert_eq!(m.draw_calls, 0);
    assert_eq!(m.triangles, 0);
    assert_eq!(m.gpu_time_ms, 0.0);
    assert_eq!(m.vram_used_mb, 0.0);
}
