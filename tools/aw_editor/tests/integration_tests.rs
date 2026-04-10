//! Comprehensive Integration Test Suite for aw_editor
//!
//! This test suite provides extensive coverage of all major editor subsystems:
//! 1. Entity Lifecycle (spawn, delete, undo/redo)
//! 2. Transform Operations (move, rotate, scale)
//! 3. Component Editing (health, team, ammo)
//! 4. Copy/Paste/Duplicate
//! 5. Undo/Redo Stack Behavior
//! 6. Play Mode Runtime (play, pause, stop, step, snapshot restore)
//! 7. Prefab System (creation, instantiation, overrides)
//! 8. Scene Serialization (save/load, component preservation)
//! 9. Complex Workflows (multi-step operations with undo)
//! 10. Edge Cases & Error Handling
//! 11. Performance & Scalability (many entities, many operations)

use astraweave_core::{IVec2, Team, World};
use aw_editor_lib::clipboard::ClipboardData;
use aw_editor_lib::command::{
    DeleteEntitiesCommand, DuplicateEntitiesCommand, EditAmmoCommand, EditHealthCommand,
    EditTeamCommand, EditorCommand, MoveEntityCommand, RotateEntityCommand, ScaleEntityCommand,
    SpawnEntitiesCommand, UndoStack,
};
use aw_editor_lib::prefab::{PrefabData, PrefabEntityData, PrefabManager};
use aw_editor_lib::runtime::{EditorRuntime, RuntimeState};
use aw_editor_lib::scene_serialization::SceneData;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tempfile::{tempdir, tempdir_in, TempDir};

// ============================================================================
// Test Utilities
// ============================================================================

fn hash_world(world: &World) -> u64 {
    let mut ids = world.entities();
    ids.sort_unstable();

    let mut hasher = DefaultHasher::new();
    (world.t.to_bits()).hash(&mut hasher);
    for id in ids {
        if let Some(pose) = world.pose(id) {
            pose.pos.x.hash(&mut hasher);
            pose.pos.y.hash(&mut hasher);
        }
        if let Some(team) = world.team(id) {
            team.id.hash(&mut hasher);
        }
        if let Some(ammo) = world.ammo(id) {
            ammo.rounds.hash(&mut hasher);
        }
        if let Some(health) = world.health(id) {
            health.hp.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn create_test_world() -> World {
    let mut world = World::new();
    world.spawn("Entity1", IVec2::new(0, 0), Team { id: 0 }, 100, 30);
    world.spawn("Entity2", IVec2::new(5, 5), Team { id: 1 }, 80, 20);
    world
}

fn sample_prefab() -> PrefabData {
    PrefabData {
        name: "TestPrefab".into(),
        entities: vec![PrefabEntityData {
            name: "Root".into(),
            pos_x: 0,
            pos_y: 0,
            team_id: 0,
            health: 100,
            max_health: 100,
            children_indices: Vec::new(),
            prefab_reference: None,
        }],
        root_entity_index: 0,
        version: "1.0".into(),
    }
}

fn content_scene_path(file_name: &str) -> (TempDir, std::path::PathBuf) {
    let workspace_root =
        std::env::current_dir().expect("tests should have access to current directory");
    let content_root = workspace_root.join("content");
    let tests_root = content_root.join("tests");
    std::fs::create_dir_all(&tests_root).expect("failed to create content/tests directory");
    let temp_dir =
        tempdir_in(&tests_root).expect("failed to create content-scoped temporary directory");
    let relative_dir = temp_dir
        .path()
        .strip_prefix(&content_root)
        .expect("temp dir should be inside content/")
        .to_path_buf();
    let relative_path = relative_dir.join(file_name);
    (temp_dir, relative_path)
}

// ============================================================================
// 1. Entity Lifecycle Tests
// ============================================================================

#[test]
fn test_spawn_entity_via_clipboard() {
    let mut world = World::new();
    let initial_count = world.entities().len();

    // Create an entity to copy
    let entity = world.spawn("TemplateEntity", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

    // Create clipboard data from entity
    let clipboard = ClipboardData::from_entities(&world, &[entity]);

    // Spawn using command
    let mut cmd = SpawnEntitiesCommand::new(clipboard, IVec2::new(10, 10));
    cmd.execute(&mut world, None).expect("spawn should succeed");

    assert_eq!(
        world.entities().len(),
        initial_count + 2,
        "should have original + spawned entity"
    );
}

#[test]
fn test_delete_entity_command() {
    let mut world = World::new();
    let entity = world.spawn("ToDelete", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

    let mut cmd = DeleteEntitiesCommand::new(vec![entity]);
    cmd.execute(&mut world, None)
        .expect("delete should succeed");

    // Entity should be destroyed (removed from world)
    // Note: destroy_entity removes the entity, so pose() returns None
    assert!(
        world.pose(entity).is_none(),
        "deleted entity should be removed from world"
    );
}

#[test]
fn test_delete_undo() {
    let mut world = World::new();
    let entity = world.spawn("ToDelete", IVec2::new(5, 5), Team { id: 0 }, 100, 30);

    let mut delete_cmd = DeleteEntitiesCommand::new(vec![entity]);
    delete_cmd.execute(&mut world, None).expect("delete");

    // Entity should be destroyed after delete
    assert!(
        world.pose(entity).is_none(),
        "entity destroyed after delete"
    );

    // Undo delete - creates a NEW entity with same data (not same ID)
    delete_cmd.undo(&mut world, None).expect("undo delete");

    // After undo, there should be an entity with the original position
    // Note: The restored entity may have a different ID
    let entities = world.entities();
    assert!(!entities.is_empty(), "undo should restore an entity");

    // Find the restored entity and check its position
    let has_restored_position = entities.iter().any(|&e| {
        world
            .pose(e)
            .map(|p| p.pos == IVec2::new(5, 5))
            .unwrap_or(false)
    });
    assert!(
        has_restored_position,
        "restored entity should have original position"
    );
}

// ============================================================================
// 2. Transform Operations Tests
// ============================================================================

#[test]
fn test_move_entity_command() {
    let mut world = create_test_world();
    let entity = world.entities()[0];

    let old_pos = world.pose(entity).unwrap().pos;
    let new_pos = IVec2::new(10, 15);

    let mut cmd = MoveEntityCommand::new(entity, old_pos, new_pos);
    cmd.execute(&mut world, None).expect("move should succeed");

    assert_eq!(
        world.pose(entity).unwrap().pos,
        new_pos,
        "entity should move to new position"
    );

    cmd.undo(&mut world, None).expect("undo should succeed");
    assert_eq!(
        world.pose(entity).unwrap().pos,
        old_pos,
        "undo should restore old position"
    );
}

#[test]
fn test_rotate_entity_command() {
    let mut world = create_test_world();
    let entity = world.entities()[0];

    let old_rotation = (0.0, 0.0, 0.0);
    let new_rotation = (0.5, 1.0, 0.25);

    let mut cmd = RotateEntityCommand::new(entity, old_rotation, new_rotation);
    cmd.execute(&mut world, None)
        .expect("rotate should succeed");

    let pose = world.pose(entity).unwrap();
    assert_eq!(pose.rotation_x, new_rotation.0);
    assert_eq!(pose.rotation, new_rotation.1);
    assert_eq!(pose.rotation_z, new_rotation.2);

    cmd.undo(&mut world, None).expect("undo should succeed");
    let pose = world.pose(entity).unwrap();
    assert_eq!(pose.rotation_x, old_rotation.0);
    assert_eq!(pose.rotation, old_rotation.1);
    assert_eq!(pose.rotation_z, old_rotation.2);
}

#[test]
fn test_scale_entity_command() {
    let mut world = create_test_world();
    let entity = world.entities()[0];

    let old_scale = 1.0;
    let new_scale = 2.5;

    let mut cmd = ScaleEntityCommand::new(entity, old_scale, new_scale);
    cmd.execute(&mut world, None).expect("scale should succeed");

    assert_eq!(world.pose(entity).unwrap().scale, new_scale);

    cmd.undo(&mut world, None).expect("undo should succeed");
    assert_eq!(world.pose(entity).unwrap().scale, old_scale);
}

// ============================================================================
// 3. Component Editing Tests
// ============================================================================

#[test]
fn test_edit_health_command() {
    let mut world = create_test_world();
    let entity = world.entities()[0];

    let old_health = world.health(entity).unwrap().hp;
    let new_health = 50;

    let mut cmd = EditHealthCommand::new(entity, old_health, new_health);
    cmd.execute(&mut world, None)
        .expect("edit health should succeed");

    assert_eq!(world.health(entity).unwrap().hp, new_health);

    cmd.undo(&mut world, None).expect("undo should succeed");
    assert_eq!(world.health(entity).unwrap().hp, old_health);
}

#[test]
fn test_edit_team_command() {
    let mut world = create_test_world();
    let entity = world.entities()[0];

    let old_team = world.team(entity).unwrap();
    let new_team = Team { id: 5 };

    let mut cmd = EditTeamCommand::new(entity, old_team, new_team);
    cmd.execute(&mut world, None)
        .expect("edit team should succeed");

    assert_eq!(world.team(entity).unwrap().id, 5);

    cmd.undo(&mut world, None).expect("undo should succeed");
    assert_eq!(world.team(entity).unwrap().id, old_team.id);
}

#[test]
fn test_edit_ammo_command() {
    let mut world = create_test_world();
    let entity = world.entities()[0];

    let old_ammo = world.ammo(entity).unwrap().rounds;
    let new_ammo = 100;

    let mut cmd = EditAmmoCommand::new(entity, old_ammo, new_ammo);
    cmd.execute(&mut world, None)
        .expect("edit ammo should succeed");

    assert_eq!(world.ammo(entity).unwrap().rounds, new_ammo);

    cmd.undo(&mut world, None).expect("undo should succeed");
    assert_eq!(world.ammo(entity).unwrap().rounds, old_ammo);
}

// ============================================================================
// 4. Copy/Paste/Duplicate Tests
// ============================================================================

#[test]
fn test_duplicate_entities_command() {
    let mut world = create_test_world();
    let entity = world.entities()[0];
    let initial_count = world.entities().len();

    let mut cmd = DuplicateEntitiesCommand::new(vec![entity], IVec2::new(10, 10));
    cmd.execute(&mut world, None)
        .expect("duplicate should succeed");

    assert_eq!(
        world.entities().len(),
        initial_count + 1,
        "duplicate should create new entity"
    );

    cmd.undo(&mut world, None).expect("undo should succeed");
    // After undo, duplicated entity should be removed
    assert_eq!(
        world.entities().len(),
        initial_count,
        "duplicated entity should be removed after undo"
    );
}

#[test]
fn test_clipboard_operations() {
    let world = create_test_world();
    let entities = world.entities();

    // Create clipboard from entities
    let clipboard = ClipboardData::from_entities(&world, &entities);
    assert_eq!(clipboard.entities.len(), 2);

    // Serialize and deserialize
    let json = clipboard.to_json().expect("serialize");
    let loaded = ClipboardData::from_json(&json).expect("deserialize");
    assert_eq!(loaded.entities.len(), clipboard.entities.len());
}

// ============================================================================
// 5. Undo/Redo Stack Behavior Tests
// ============================================================================

#[test]
fn test_undo_stack_basic_operations() {
    let mut world = create_test_world();
    let mut undo_stack = UndoStack::new(64);
    let entity = world.entities()[0];

    let old_pos = world.pose(entity).unwrap().pos;
    let new_pos = IVec2::new(20, 20);

    undo_stack
        .execute(
            MoveEntityCommand::new(entity, old_pos, new_pos),
            &mut world,
            None,
        )
        .expect("execute");

    assert!(undo_stack.can_undo());
    assert!(!undo_stack.can_redo());

    undo_stack.undo(&mut world, None).expect("undo");
    assert_eq!(world.pose(entity).unwrap().pos, old_pos);
    assert!(!undo_stack.can_undo());
    assert!(undo_stack.can_redo());

    undo_stack.redo(&mut world, None).expect("redo");
    assert_eq!(world.pose(entity).unwrap().pos, new_pos);
}

#[test]
fn test_undo_stack_multiple_operations() {
    let mut world = World::new();
    // Spawn a single entity at known position
    let entity = world.spawn("TestEntity", IVec2::new(0, 0), Team { id: 0 }, 100, 30);
    let mut undo_stack = UndoStack::new(64);
    undo_stack.set_auto_merge(false); // Disable merging to test individual operations

    let initial_pos = world.pose(entity).unwrap().pos;
    assert_eq!(
        initial_pos,
        IVec2::new(0, 0),
        "Initial position should be (0,0)"
    );

    // Perform multiple operations - each move to a new position
    let positions = [
        IVec2::new(10, 10),
        IVec2::new(20, 20),
        IVec2::new(30, 30),
        IVec2::new(40, 40),
        IVec2::new(50, 50),
    ];

    let mut prev_pos = initial_pos;
    for &new_pos in &positions {
        undo_stack
            .execute(
                MoveEntityCommand::new(entity, prev_pos, new_pos),
                &mut world,
                None,
            )
            .expect("execute");
        prev_pos = new_pos;
    }

    // Final position should be (50,50)
    assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(50, 50));

    // Undo all 5 operations
    for _ in 0..5 {
        undo_stack.undo(&mut world, None).expect("undo");
    }

    // Should be back to initial position
    assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(0, 0));
}

#[test]
fn test_undo_stack_branching() {
    let mut world = create_test_world();
    let mut undo_stack = UndoStack::new(64);
    let entity = world.entities()[0];

    // Execute, undo, then execute new command (creates branch)
    undo_stack
        .execute(
            MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(10, 10)),
            &mut world,
            None,
        )
        .expect("execute 1");

    undo_stack.undo(&mut world, None).expect("undo");

    undo_stack
        .execute(
            MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(20, 20)),
            &mut world,
            None,
        )
        .expect("execute 2");

    // Old redo history should be discarded
    assert!(!undo_stack.can_redo());
    assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(20, 20));
}

// ============================================================================
// 6. Play Mode Runtime Tests
// ============================================================================

#[test]
fn test_runtime_enter_play() {
    let world = create_test_world();
    let mut runtime = EditorRuntime::new();

    runtime.enter_play(&world).expect("enter play");

    assert_eq!(runtime.state(), RuntimeState::Playing);
    let sim_world = runtime.sim_world().expect("sim world should exist");
    assert_eq!(sim_world.entities().len(), world.entities().len());
}

#[test]
fn test_runtime_pause_resume() {
    let world = create_test_world();
    let mut runtime = EditorRuntime::new();

    runtime.enter_play(&world).expect("enter play");
    runtime.pause();
    assert_eq!(runtime.state(), RuntimeState::Paused);

    runtime.resume();
    assert_eq!(runtime.state(), RuntimeState::Playing);
}

#[test]
fn test_runtime_stop_restores_snapshot() {
    let world = create_test_world();
    let baseline_hash = hash_world(&world);
    let mut runtime = EditorRuntime::new();

    runtime.enter_play(&world).expect("enter play");

    // Modify sim world
    {
        let sim_world = runtime.sim_world_mut().expect("sim world");
        let entity = sim_world.entities()[0];
        if let Some(pose) = sim_world.pose_mut(entity) {
            pose.pos.x += 100;
        }
    }

    // Stop and verify restoration
    let restored = runtime.exit_play().expect("exit play");
    let restored_world = restored.expect("world restored");

    assert_eq!(hash_world(&restored_world), baseline_hash);
    assert!(runtime.sim_world().is_none());
}

#[test]
fn test_runtime_step_frame() {
    let world = create_test_world();
    let mut runtime = EditorRuntime::new();

    runtime.enter_play(&world).expect("enter play");
    runtime.pause();

    runtime.step_frame().expect("step frame");
    assert_eq!(runtime.stats().tick_count, 1);
    assert_eq!(runtime.state(), RuntimeState::Paused);
}

#[test]
fn test_runtime_deterministic_replay() {
    let world = create_test_world();
    let mut runtime = EditorRuntime::new();

    // First run
    runtime.enter_play(&world).expect("enter play");
    for _ in 0..50 {
        runtime.tick(1.0 / 60.0).expect("tick");
    }
    let hash_a = hash_world(runtime.sim_world().expect("sim world"));

    // Second run
    runtime.exit_play().expect("exit play");
    runtime.enter_play(&world).expect("enter play again");
    for _ in 0..50 {
        runtime.tick(1.0 / 60.0).expect("tick");
    }
    let hash_b = hash_world(runtime.sim_world().expect("sim world"));

    assert_eq!(hash_a, hash_b, "simulation should be deterministic");
}

#[test]
fn test_runtime_stats_accuracy() {
    let world = create_test_world();
    let mut runtime = EditorRuntime::new();

    runtime.enter_play(&world).expect("enter play");
    runtime.tick(1.0 / 60.0).expect("tick");

    let stats = runtime.stats();
    assert_eq!(stats.entity_count, world.entities().len());
    assert_eq!(stats.tick_count, 1);
    assert!(stats.frame_time_ms >= 0.0);
}

// ============================================================================
// 7. Prefab System Tests
// ============================================================================

#[test]
fn test_prefab_instantiation() {
    let temp = tempdir().expect("temp dir");
    let prefab_path = temp.path().join("test.prefab.ron");
    sample_prefab()
        .save_to_file(&prefab_path)
        .expect("save prefab");

    let mut manager = PrefabManager::new(temp.path());
    let mut world = World::new();

    let root = manager
        .instantiate_prefab(&prefab_path, &mut world, (5, 10))
        .expect("instantiate");

    assert_eq!(world.pose(root).unwrap().pos, IVec2::new(5, 10));
    assert!(manager.find_instance(root).is_some());
}

#[test]
fn test_prefab_save_load() {
    let temp = tempdir().expect("temp dir");
    let prefab_path = temp.path().join("save_load.prefab.ron");

    let original = sample_prefab();
    original.save_to_file(&prefab_path).expect("save");

    let loaded = PrefabData::load_from_file(&prefab_path).expect("load");

    assert_eq!(original.name, loaded.name);
    assert_eq!(original.entities.len(), loaded.entities.len());
}

#[test]
fn test_prefab_from_entity() {
    let mut world = World::new();
    let entity = world.spawn("TestEntity", IVec2::new(10, 20), Team { id: 2 }, 75, 15);

    let prefab =
        PrefabData::from_entity(&world, entity, "EntityPrefab".to_string()).expect("create prefab");

    assert_eq!(prefab.name, "EntityPrefab");
    assert_eq!(prefab.entities.len(), 1);
    assert_eq!(prefab.entities[0].pos_x, 10);
    assert_eq!(prefab.entities[0].pos_y, 20);
}

// ============================================================================
// 8. Scene Serialization Tests
// ============================================================================

// NOTE: These tests use tempdir() which creates paths outside the content/ directory.
// The scene serialization has security constraints that restrict paths to content/.
// These tests need to be refactored to use content/ relative paths or mocked paths.

#[test]
fn test_scene_save_load() {
    let (_content_dir, scene_path) = content_scene_path("test_scene.ron");

    let world = create_test_world();
    let scene_data = SceneData::from_world(&world);

    scene_data.save_to_file(&scene_path).expect("save scene");

    let loaded_data = SceneData::load_from_file(&scene_path).expect("load scene");
    let loaded_world = loaded_data.to_world();

    assert_eq!(
        world.entities().len(),
        loaded_world.entities().len(),
        "entity count should match"
    );
    assert_eq!(hash_world(&world), hash_world(&loaded_world));
}

#[test]
fn test_scene_preserves_components() {
    let (_content_dir, scene_path) = content_scene_path("components.ron");

    let mut world = World::new();
    let entity = world.spawn("TestEntity", IVec2::new(5, 10), Team { id: 2 }, 75, 15);

    // Set rotation and scale
    if let Some(pose) = world.pose_mut(entity) {
        pose.rotation = 1.5;
        pose.rotation_x = 0.5;
        pose.rotation_z = 0.25;
        pose.scale = 2.0;
    }

    let scene_data = SceneData::from_world(&world);
    scene_data.save_to_file(&scene_path).expect("save");

    let loaded_data = SceneData::load_from_file(&scene_path).expect("load");
    let loaded_world = loaded_data.to_world();

    let loaded_entity = loaded_world.entities()[0];
    let loaded_pose = loaded_world.pose(loaded_entity).unwrap();

    assert_eq!(loaded_pose.pos, IVec2::new(5, 10));
    assert_eq!(loaded_pose.rotation, 1.5);
    assert_eq!(loaded_pose.rotation_x, 0.5);
    assert_eq!(loaded_pose.rotation_z, 0.25);
    assert_eq!(loaded_pose.scale, 2.0);
    assert_eq!(loaded_world.team(loaded_entity).unwrap().id, 2);
    assert_eq!(loaded_world.health(loaded_entity).unwrap().hp, 75);
    assert_eq!(loaded_world.ammo(loaded_entity).unwrap().rounds, 15);
}

// ============================================================================
// 9. Complex Workflow Tests
// ============================================================================

#[test]
fn test_complex_workflow_edit_save_load() {
    let (_content_dir, scene_path) = content_scene_path("workflow.ron");

    let mut world = World::new();
    let mut undo_stack = UndoStack::new(64);
    let entity = world.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

    // Move entity
    undo_stack
        .execute(
            MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(10, 20)),
            &mut world,
            None,
        )
        .expect("move");

    // Edit health
    undo_stack
        .execute(EditHealthCommand::new(entity, 100, 50), &mut world, None)
        .expect("edit health");

    // Save scene
    let scene_data = SceneData::from_world(&world);
    scene_data.save_to_file(&scene_path).expect("save");

    // Load scene
    let loaded_data = SceneData::load_from_file(&scene_path).expect("load");
    let loaded_world = loaded_data.to_world();

    let loaded_entity = loaded_world.entities()[0];
    assert_eq!(
        loaded_world.pose(loaded_entity).unwrap().pos,
        IVec2::new(10, 20)
    );
    assert_eq!(loaded_world.health(loaded_entity).unwrap().hp, 50);
}

#[test]
fn test_complex_undo_redo_sequence() {
    let mut world = create_test_world();
    let mut undo_stack = UndoStack::new(64);
    let entity = world.entities()[0];

    // Perform multiple edits
    undo_stack
        .execute(
            MoveEntityCommand::new(entity, IVec2::new(0, 0), IVec2::new(5, 5)),
            &mut world,
            None,
        )
        .expect("move");

    undo_stack
        .execute(EditHealthCommand::new(entity, 100, 75), &mut world, None)
        .expect("health");

    undo_stack
        .execute(
            EditTeamCommand::new(entity, Team { id: 0 }, Team { id: 3 }),
            &mut world,
            None,
        )
        .expect("team");

    // Undo all
    undo_stack.undo(&mut world, None).expect("undo team");
    undo_stack.undo(&mut world, None).expect("undo health");
    undo_stack.undo(&mut world, None).expect("undo move");

    assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(0, 0));
    assert_eq!(world.health(entity).unwrap().hp, 100);
    assert_eq!(world.team(entity).unwrap().id, 0);

    // Redo all
    undo_stack.redo(&mut world, None).expect("redo move");
    undo_stack.redo(&mut world, None).expect("redo health");
    undo_stack.redo(&mut world, None).expect("redo team");

    assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(5, 5));
    assert_eq!(world.health(entity).unwrap().hp, 75);
    assert_eq!(world.team(entity).unwrap().id, 3);
}

// ============================================================================
// 10. Edge Cases & Error Handling Tests
// ============================================================================

#[test]
fn test_empty_world_operations() {
    let world = World::new();
    let mut runtime = EditorRuntime::new();

    runtime
        .enter_play(&world)
        .expect("enter play with empty world");
    assert_eq!(runtime.sim_world().expect("sim world").entities().len(), 0);

    let restored = runtime.exit_play().expect("exit play");
    assert_eq!(restored.expect("world").entities().len(), 0);
}

#[test]
fn test_invalid_entity_operations() {
    let mut world = World::new();
    let invalid_entity = 9999;

    let mut cmd = MoveEntityCommand::new(invalid_entity, IVec2::new(0, 0), IVec2::new(1, 1));
    let result = cmd.execute(&mut world, None);

    assert!(result.is_err(), "operation on invalid entity should fail");
}

#[test]
fn test_undo_stack_max_size() {
    let mut world = create_test_world();
    let mut undo_stack = UndoStack::new(5); // Small limit
    let entity = world.entities()[0];

    // Execute more commands than limit
    for i in 0..10 {
        undo_stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                &mut world,
                None,
            )
            .expect("execute");
    }

    // Stack should be capped at max_size
    assert!(undo_stack.len() <= 5);
}

// ============================================================================
// 11. Performance & Scalability Tests
// ============================================================================

#[test]
fn test_many_entities() {
    let mut world = World::new();

    // Spawn many entities
    for i in 0..100 {
        world.spawn(
            &format!("Entity_{}", i),
            IVec2::new(i, i),
            Team { id: (i % 4) as u8 },
            100,
            30,
        );
    }

    assert_eq!(world.entities().len(), 100);

    // Save and load
    let (_content_dir, scene_path) = content_scene_path("many_entities.ron");

    let scene_data = SceneData::from_world(&world);
    scene_data.save_to_file(&scene_path).expect("save");

    let loaded_data = SceneData::load_from_file(&scene_path).expect("load");
    let loaded_world = loaded_data.to_world();

    assert_eq!(loaded_world.entities().len(), 100);
    assert_eq!(hash_world(&world), hash_world(&loaded_world));
}

#[test]
fn test_many_undo_operations() {
    let mut world = create_test_world();
    let mut undo_stack = UndoStack::new(200);
    let entity = world.entities()[0];

    // Perform many operations
    for i in 0..100 {
        undo_stack
            .execute(
                MoveEntityCommand::new(entity, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                &mut world,
                None,
            )
            .expect("execute");
    }

    // Undo all
    for _ in 0..100 {
        undo_stack.undo(&mut world, None).expect("undo");
    }

    assert_eq!(world.pose(entity).unwrap().pos, IVec2::new(0, 0));
}

#[test]
fn test_runtime_with_many_entities() {
    let mut world = World::new();

    for i in 0..50 {
        world.spawn(
            &format!("Agent_{}", i),
            IVec2::new(i * 2, i * 2),
            Team { id: (i % 2) as u8 },
            100,
            20,
        );
    }

    let mut runtime = EditorRuntime::new();
    runtime.enter_play(&world).expect("enter play");

    for _ in 0..10 {
        runtime.tick(1.0 / 60.0).expect("tick");
    }

    let stats = runtime.stats();
    assert_eq!(stats.entity_count, 50);
    assert_eq!(stats.tick_count, 10);
}
