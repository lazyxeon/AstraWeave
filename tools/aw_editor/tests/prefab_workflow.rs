//! Prefab workflow tests
//!
//! NOTE: Many tests are commented out because they depend on PrefabSpawnCommand,
//! PrefabApplyOverridesCommand, and PrefabRevertOverridesCommand which are not yet
//! implemented in the command module. These should be re-enabled when the prefab
//! command infrastructure is complete.

use astraweave_core::{IVec2, World};
use aw_editor_lib::command::{PrefabRevertOverridesCommand, PrefabSpawnCommand};
use aw_editor_lib::prefab::{PrefabData, PrefabEntityData, PrefabHierarchySnapshot, PrefabManager};
use tempfile::tempdir;

fn sample_prefab() -> PrefabData {
    PrefabData {
        name: "TestPrefab".into(),
        entities: vec![PrefabEntityData {
            name: "CrateRoot".into(),
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

#[test]
fn prefab_spawn_command_instantiates_and_undoes() {
    let temp = tempdir().expect("temp dir");
    let prefab_path = temp.path().join("crate.prefab.ron");
    sample_prefab()
        .save_to_file(&prefab_path)
        .expect("write prefab");

    let manager = PrefabManager::shared(temp.path());
    let mut world = World::new();

    let mut cmd = PrefabSpawnCommand::new(prefab_path.clone(), manager.clone(), (2, 3));
    cmd.execute(&mut world, None)
        .expect("prefab spawn command executes");

    assert_eq!(world.entities().len(), 1);
    let entity = world.entities()[0];
    let pose = world.pose(entity).expect("spawned entity pose");
    assert_eq!(pose.pos, IVec2::new(2, 3));

    cmd.undo(&mut world, None).expect("undo removes prefab");
    let undone_pose = world.pose(entity).expect("entity still exists after undo");
    assert_eq!(
        undone_pose.pos,
        IVec2::new(-10000, -10000),
        "undo should soft-delete by moving entity far away"
    );
    assert!(
        undone_pose.scale < 0.01,
        "undo should set scale to 0 (invisible)"
    );
}

// TODO: Re-enable when PrefabApplyOverridesCommand is implemented
// #[test]
// fn prefab_apply_overrides_updates_prefab_file_and_is_undoable() { ... }

/// Test that prefab data can be saved and loaded correctly
#[test]
fn prefab_data_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let prefab_path = temp.path().join("test.prefab.ron");

    let original = sample_prefab();
    original.save_to_file(&prefab_path).expect("save prefab");

    let loaded = PrefabData::load_from_file(&prefab_path).expect("load prefab");
    assert_eq!(loaded.name, "TestPrefab");
    assert_eq!(loaded.entities.len(), 1);
    assert_eq!(loaded.entities[0].name, "CrateRoot");
}

/// Test that prefab manager can instantiate prefabs directly
#[test]
fn prefab_manager_instantiates_prefab() {
    let temp = tempdir().expect("temp dir");
    let prefab_path = temp.path().join("crate.prefab.ron");
    sample_prefab()
        .save_to_file(&prefab_path)
        .expect("write prefab");

    let manager = PrefabManager::shared(temp.path());
    let mut world = World::new();

    let root = {
        let mut mgr = manager.lock().expect("manager lock");
        mgr.instantiate_prefab(&prefab_path, &mut world, (4, -2))
            .expect("instantiate prefab")
    };

    let entities = world.entities();
    assert_eq!(entities.len(), 1, "instantiate creates a root entity");
    let pose = world.pose(root).expect("spawned pose");
    assert_eq!(pose.pos, IVec2::new(4, -2));

    {
        let mgr = manager.lock().expect("manager lock");
        assert!(
            mgr.find_instance(root).is_some(),
            "prefab manager tracks the spawned instance"
        );
    }
}

#[test]
fn prefab_creation_with_hierarchy_snapshot_persists_children() {
    let temp = tempdir().expect("temp dir");
    let manager = PrefabManager::shared(temp.path());
    let mut world = World::new();

    let root = world.spawn(
        "Root",
        IVec2::new(0, 0),
        astraweave_core::Team { id: 0 },
        100,
        0,
    );
    let child = world.spawn(
        "Child",
        IVec2::new(1, 0),
        astraweave_core::Team { id: 0 },
        100,
        0,
    );
    let grandchild = world.spawn(
        "Grandchild",
        IVec2::new(2, 0),
        astraweave_core::Team { id: 0 },
        100,
        0,
    );

    let hierarchy =
        PrefabHierarchySnapshot::from_iter([(root, vec![child]), (child, vec![grandchild])]);

    {
        let mgr = manager.lock().expect("manager lock");
        mgr.create_prefab_with_hierarchy(&world, root, "hierarchy", Some(&hierarchy))
            .expect("prefab creation succeeds");
    }

    let prefab_path = temp.path().join("hierarchy.prefab.ron");
    let saved = PrefabData::load_from_file(&prefab_path).expect("load saved prefab");

    assert_eq!(saved.entities.len(), 3);
    let root_data = &saved.entities[saved.root_entity_index];
    assert_eq!(root_data.children_indices, vec![1]);

    let child_data = &saved.entities[root_data.children_indices[0]];
    assert_eq!(child_data.children_indices, vec![2]);
}

#[test]
fn prefab_revert_overrides_restores_world_pose_and_supports_undo() {
    let temp = tempdir().expect("temp dir");
    let prefab_path = temp.path().join("crate.prefab.ron");
    sample_prefab()
        .save_to_file(&prefab_path)
        .expect("write prefab");

    let manager = PrefabManager::shared(temp.path());
    let mut world = World::new();

    let root = {
        let mut mgr = manager.lock().expect("manager lock");
        mgr.instantiate_prefab(&prefab_path, &mut world, (0, 0))
            .expect("instantiate prefab")
    };

    if let Some(pose) = world.pose_mut(root) {
        pose.pos = IVec2::new(5, 3);
    }

    let mutated_pose = world.pose(root).expect("mutated pose").pos;
    let snapshot = {
        let mgr = manager.lock().expect("manager lock");
        mgr.capture_snapshot(&world, root)
            .expect("snapshot available")
    };

    let mut cmd = PrefabRevertOverridesCommand::new(manager.clone(), root, snapshot);
    cmd.execute(&mut world, None).expect("revert overrides");
    let reverted = world.pose(root).expect("reverted pose");
    assert_eq!(reverted.pos, IVec2::new(0, 0));

    cmd.undo(&mut world, None).expect("undo revert");
    let restored = world.pose(root).expect("restored pose");
    assert_eq!(restored.pos, mutated_pose);
}

// TODO: Re-enable when spawn_prefab_with_undo is implemented
// #[test]
// fn spawn_prefab_helper_records_undo_entry() {
//     let temp = tempdir().expect("temp dir");
//     let prefab_path = temp.path().join("crate.prefab.ron");
//     sample_prefab()
//         .save_to_file(&prefab_path)
//         .expect("write prefab");
//
//     let manager = PrefabManager::shared(temp.path());
//     let mut world = World::new();
//     let mut undo_stack = UndoStack::new(8);
//
//     let root = spawn_prefab_with_undo(
//         manager.clone(),
//         prefab_path.clone(),
//         (7, 3),
//         &mut world,
//         &mut undo_stack,
//     )
//     .expect("spawn prefab with helper");
//
//     assert!(undo_stack.can_undo(), "helper pushes undo entry");
//     let pose = world.pose(root).expect("pose after spawn");
//     assert_eq!(pose.pos, IVec2::new(7, 3));
//
//     undo_stack.undo(&mut world).expect("undo drop");
//     let reverted = world.pose(root).expect("pose after undo");
//     assert_eq!(reverted.pos, IVec2::new(-10000, -10000));
// }

#[test]
fn prefab_manager_tracks_override_from_snapshot() {
    let temp = tempdir().expect("temp dir");
    let prefab_path = temp.path().join("crate.prefab.ron");
    sample_prefab()
        .save_to_file(&prefab_path)
        .expect("write prefab");

    let manager = PrefabManager::shared(temp.path());
    let mut world = World::new();

    let root = {
        let mut mgr = manager.lock().expect("manager lock");
        mgr.instantiate_prefab(&prefab_path, &mut world, (0, 0))
            .expect("instantiate prefab")
    };

    if let Some(pose) = world.pose_mut(root) {
        pose.pos = IVec2::new(-4, 8);
    }

    let pose = world.pose(root);
    let health = world.health(root);

    {
        let mut mgr = manager.lock().expect("manager lock");
        mgr.track_override_snapshot(root, pose, health);
        let instance = mgr.find_instance(root).expect("prefab instance");
        assert!(instance.has_overrides(root));
    }
}
