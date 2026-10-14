//! Cross-Crate API Stability Tests - Tier 1
//!
//! Verify stable API contracts between foundational AstraWeave crates.
//! Focus: astraweave-ecs, astraweave-core, astraweave-behavior

// ============================================================================
// ECS Crate API Stability Tests
// ============================================================================

#[test]
fn test_ecs_world_api() {
    use astraweave_ecs::World;

    let mut world = World::new();

    let entity = world.spawn();
    assert!(world.is_alive(entity));

    #[derive(Clone, Copy)]
    struct TestComponent(i32);

    world.insert(entity, TestComponent(42));
    assert!(world.has::<TestComponent>(entity));

    let comp = world.get::<TestComponent>(entity).unwrap();
    assert_eq!(comp.0, 42);

    world.despawn(entity);
    assert!(!world.is_alive(entity));
}

#[test]
fn test_ecs_app_api() {
    use astraweave_ecs::{App, World};

    let mut app = App::new();

    fn test_system(_world: &mut World) {}

    app.schedule.add_stage("test");
    app.add_system("test", test_system)
        .expect("registered schedule stage");
    app.schedule.run(&mut app.world);
}

#[test]
fn test_ecs_query_api() {
    use astraweave_ecs::{Query, World};

    let mut world = World::new();

    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct Position(f32);

    for i in 0..5 {
        let e = world.spawn();
        world.insert(e, Position(i as f32));
    }

    let query = Query::<Position>::new(&world);
    assert_eq!(query.count(), 5);
}

#[test]
fn test_entity_api() {
    use astraweave_ecs::{Entity, World};

    let mut world = World::new();

    let entity = world.spawn();
    let _id: u32 = entity.id();
    let _gen: u32 = entity.generation();
    let _raw: u64 = entity.to_raw();

    let null = Entity::null();
    assert!(null.is_null());
    assert!(!entity.is_null());
}

// ============================================================================
// Core Crate API Stability Tests
// ============================================================================

#[test]
fn test_core_world_snapshot() {
    use astraweave_core::WorldSnapshot;

    let snapshot = WorldSnapshot::default();
    let _ = snapshot;
}

#[test]
fn test_core_plan_intent() {
    use astraweave_core::{ActionStep, PlanIntent};

    let intent = PlanIntent {
        plan_id: "test".to_string(),
        steps: vec![ActionStep::Reload],
    };
    assert_eq!(intent.plan_id, "test");
    assert_eq!(intent.steps.len(), 1);
}

#[test]
fn test_core_action_step_variants() {
    use astraweave_core::ActionStep;

    let _ = ActionStep::MoveTo {
        x: 0,
        y: 0,
        speed: None,
    };
    let _ = ActionStep::Attack { target_id: 1 };
    let _ = ActionStep::Reload;
    let _ = ActionStep::TakeCover { position: None };
}

#[test]
fn test_core_tool_registry() {
    use astraweave_core::default_tool_registry;

    let registry = default_tool_registry();
    assert!(!registry.tools.is_empty());
    assert!(registry.constraints.enforce_cooldowns);
}

// ============================================================================
// Behavior Crate API Stability Tests
// ============================================================================

#[test]
fn test_behavior_graph_api() {
    use astraweave_behavior::{BehaviorContext, BehaviorGraph, BehaviorNode, BehaviorStatus};

    // BehaviorNode uses tuple variants: Action(String), Condition(String), etc.
    let node = BehaviorNode::Action("test".to_string());
    let graph = BehaviorGraph::new(node);

    let mut ctx = BehaviorContext::default();
    ctx.register_action("test", || BehaviorStatus::Success);

    let status = graph.tick(&ctx);
    assert_eq!(status, BehaviorStatus::Success);
}

#[test]
fn test_behavior_node_variants() {
    use astraweave_behavior::BehaviorNode;

    // Tuple variants - verify API structure
    let _ = BehaviorNode::Action("action_name".to_string());
    let _ = BehaviorNode::Condition("condition_name".to_string());
    let _ = BehaviorNode::Sequence(vec![]);
    let _ = BehaviorNode::Selector(vec![]);
    let _ = BehaviorNode::Parallel(vec![], 1); // children, success_threshold
}

#[test]
fn test_behavior_status_api() {
    use astraweave_behavior::BehaviorStatus;

    assert_eq!(BehaviorStatus::Success, BehaviorStatus::Success);
    assert_ne!(BehaviorStatus::Success, BehaviorStatus::Failure);
}

#[test]
fn test_action_step_serialization() {
    use astraweave_core::ActionStep;

    let action = ActionStep::MoveTo {
        x: 10,
        y: 20,
        speed: None,
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("MoveTo"));
}
