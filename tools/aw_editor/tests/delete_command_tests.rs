use astraweave_core::{IVec2, Team, World};
use aw_editor_lib::command::{DeleteEntitiesCommand, UndoStack};

#[test]
fn test_delete_entities_undo_redo() {
    let mut world = World::new();
    let entity = world.spawn("TestEnt", IVec2::new(10, 20), Team { id: 1 }, 100, 30);

    // Check existence
    assert!(world.pose(entity).is_some());
    assert_eq!(world.team(entity).unwrap().id, 1);

    let mut stack = UndoStack::new(10);

    // 1. Execute delete
    // Note: DeleteEntitiesCommand::new returns Box<dyn EditorCommand>
    let cmd = DeleteEntitiesCommand::new(vec![entity]);
    stack.execute(cmd, &mut world, None).expect("Delete failed");

    // Check deletion
    assert!(world.pose(entity).is_none(), "Entity should be destroyed");

    // 2. Undo
    stack.undo(&mut world, None).expect("Undo failed");

    // Check restoration
    assert!(world.pose(entity).is_some(), "Entity should be restored");
    assert_eq!(world.pose(entity).unwrap().pos.x, 10);
    assert_eq!(world.team(entity).unwrap().id, 1);

    // 3. Redo
    stack.redo(&mut world, None).expect("Redo failed");

    // Check deletion again
    assert!(
        world.pose(entity).is_none(),
        "Entity should be destroyed again"
    );
}

#[test]
fn test_delete_multiple_entities() {
    let mut world = World::new();
    let e1 = world.spawn("E1", IVec2::new(0, 0), Team { id: 1 }, 100, 30);
    let e2 = world.spawn("E2", IVec2::new(1, 1), Team { id: 2 }, 100, 30);

    let mut stack = UndoStack::new(10);

    stack
        .execute(DeleteEntitiesCommand::new(vec![e1, e2]), &mut world, None)
        .unwrap();

    assert!(world.pose(e1).is_none());
    assert!(world.pose(e2).is_none());

    stack.undo(&mut world, None).unwrap();

    assert!(world.pose(e1).is_some());
    assert!(world.pose(e2).is_some());
}
