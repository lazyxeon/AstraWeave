//! ECS integration for behavior trees

use crate::{BehaviorContext, BehaviorGraph, BehaviorStatus};
use astraweave_ecs::Query;

const SIMULATION_STAGE: &str = "simulation";

/// Component for entities with behavior graphs
pub struct CBehaviorGraph {
    pub graph: BehaviorGraph,
    pub context: BehaviorContext,
    pub status: BehaviorStatus,
    pub running_node: Option<String>, // for debugging
}
/// System to tick behavior graphs
pub fn behavior_tick_system(world: &mut astraweave_ecs::World) {
    // Collect entities with behavior graphs
    let mut to_update = vec![];
    {
        let q = Query::<CBehaviorGraph>::new(world);
        for (entity, bg) in q {
            let status = bg.graph.tick(&bg.context);
            to_update.push((entity, status));
        }
    }

    // Update statuses

    for (entity, status) in to_update {
        if let Some(bg) = world.get_mut::<CBehaviorGraph>(entity) {
            bg.status = status;
            bg.running_node = bg.graph.current_node_name();
        }
    }
}
/// Plugin to add behavior systems
pub struct BehaviorPlugin;

impl astraweave_ecs::Plugin for BehaviorPlugin {
    fn build(&self, app: &mut astraweave_ecs::App) {
        app.add_system_known_stage(
            SIMULATION_STAGE,
            behavior_tick_system as astraweave_ecs::SystemFn,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BehaviorGraph, BehaviorNode, BehaviorStatus};

    fn create_test_context() -> BehaviorContext {
        let mut context = BehaviorContext::new();
        context.register_action("succeed", || BehaviorStatus::Success);
        context.register_action("fail", || BehaviorStatus::Failure);
        context.register_action("run", || BehaviorStatus::Running);
        context.register_condition("always_true", || true);
        context.register_condition("always_false", || false);
        context
    }

    #[test]
    fn test_cbehavior_graph_component() {
        // Verify CBehaviorGraph component can be created
        let graph = BehaviorGraph::new(BehaviorNode::Action("test".to_string()));
        let context = create_test_context();

        let component = CBehaviorGraph {
            graph,
            context,
            status: BehaviorStatus::Running,
            running_node: None,
        };

        assert_eq!(component.status, BehaviorStatus::Running);
        assert!(component.running_node.is_none());
    }

    #[test]
    fn test_behavior_tick_system_updates_status() {
        // Verify behavior_tick_system updates entity status
        let mut world = astraweave_ecs::World::new();

        // Create a simple success action
        let graph = BehaviorGraph::new(BehaviorNode::Action("succeed".to_string()));
        let context = create_test_context();

        let entity = world.spawn();
        world.insert(
            entity,
            CBehaviorGraph {
                graph,
                context,
                status: BehaviorStatus::Running,
                running_node: None,
            },
        );

        // Run the system
        behavior_tick_system(&mut world);

        // Check status was updated
        let bg = world
            .get::<CBehaviorGraph>(entity)
            .expect("Component should exist");
        assert_eq!(bg.status, BehaviorStatus::Success);
    }

    #[test]
    fn test_behavior_tick_system_multiple_entities() {
        // Verify behavior_tick_system handles multiple entities
        let mut world = astraweave_ecs::World::new();
        let context = create_test_context();

        // Spawn 3 entities with behavior graphs
        let entity1 = world.spawn();
        world.insert(
            entity1,
            CBehaviorGraph {
                graph: BehaviorGraph::new(BehaviorNode::Action("succeed".to_string())),
                context: create_test_context(),
                status: BehaviorStatus::Running,
                running_node: None,
            },
        );

        let entity2 = world.spawn();
        world.insert(
            entity2,
            CBehaviorGraph {
                graph: BehaviorGraph::new(BehaviorNode::Action("fail".to_string())),
                context: create_test_context(),
                status: BehaviorStatus::Running,
                running_node: None,
            },
        );

        let entity3 = world.spawn();
        world.insert(
            entity3,
            CBehaviorGraph {
                graph: BehaviorGraph::new(BehaviorNode::Action("run".to_string())),
                context,
                status: BehaviorStatus::Running,
                running_node: None,
            },
        );

        // Run the system
        behavior_tick_system(&mut world);

        // All entities should have been processed
        assert!(world.get::<CBehaviorGraph>(entity1).is_some());
        assert!(world.get::<CBehaviorGraph>(entity2).is_some());
        assert!(world.get::<CBehaviorGraph>(entity3).is_some());

        // Verify statuses updated correctly
        let bg1 = world.get::<CBehaviorGraph>(entity1).unwrap();
        assert_eq!(bg1.status, BehaviorStatus::Success);

        let bg2 = world.get::<CBehaviorGraph>(entity2).unwrap();
        assert_eq!(bg2.status, BehaviorStatus::Failure);

        let bg3 = world.get::<CBehaviorGraph>(entity3).unwrap();
        assert_eq!(bg3.status, BehaviorStatus::Running);
    }

    #[test]
    fn test_behavior_tick_system_empty_world() {
        // Verify behavior_tick_system handles empty world gracefully
        let mut world = astraweave_ecs::World::new();

        // Run the system on empty world (should not panic)
        behavior_tick_system(&mut world);

        // No entities should exist
        let q = Query::<CBehaviorGraph>::new(&world);
        assert_eq!(q.count(), 0);
    }

    #[test]
    fn test_behavior_plugin_adds_system() {
        // Verify BehaviorPlugin adds the tick system
        let mut app = astraweave_ecs::App::new();
        let plugin = BehaviorPlugin;

        app = app.add_plugin(plugin);

        // Spawn an entity to verify system runs
        let entity = app.world.spawn();
        let graph = BehaviorGraph::new(BehaviorNode::Action("succeed".to_string()));
        let context = create_test_context();
        app.world.insert(
            entity,
            CBehaviorGraph {
                graph,
                context,
                status: BehaviorStatus::Running,
                running_node: None,
            },
        );

        // Run one tick (should execute the system)
        app = app.run_fixed(1);

        // Verify system ran by checking status changed
        let bg = app
            .world
            .get::<CBehaviorGraph>(entity)
            .expect("Component should exist");
        assert_eq!(bg.status, BehaviorStatus::Success);
    }

    #[test]
    fn test_behavior_running_node_tracking() {
        // Verify running_node field is updated with current node name
        let mut world = astraweave_ecs::World::new();

        let mut context = BehaviorContext::new();
        // Register actions that the sequence uses
        context.register_action("first", || BehaviorStatus::Success);
        context.register_action("second", || BehaviorStatus::Success);

        let graph = BehaviorGraph::new(BehaviorNode::Sequence(vec![
            BehaviorNode::Action("first".to_string()),
            BehaviorNode::Action("second".to_string()),
        ]));

        let entity = world.spawn();
        world.insert(
            entity,
            CBehaviorGraph {
                graph,
                context,
                status: BehaviorStatus::Running,
                running_node: None,
            },
        );

        // Run the system
        behavior_tick_system(&mut world);

        // Check running_node was updated
        let bg = world
            .get::<CBehaviorGraph>(entity)
            .expect("Component should exist");
        assert!(
            bg.running_node.is_some(),
            "running_node should be set after tick"
        );
    }

    #[test]
    fn test_cbehavior_graph_status_types() {
        // Verify CBehaviorGraph can hold all status types
        let graph = BehaviorGraph::new(BehaviorNode::Action("test".to_string()));
        let _context = create_test_context();

        // Test all status types
        let mut component = CBehaviorGraph {
            graph: graph.clone(),
            context: create_test_context(),
            status: BehaviorStatus::Success,
            running_node: None,
        };
        assert_eq!(component.status, BehaviorStatus::Success);

        component.status = BehaviorStatus::Failure;
        assert_eq!(component.status, BehaviorStatus::Failure);

        component.status = BehaviorStatus::Running;
        assert_eq!(component.status, BehaviorStatus::Running);
    }
}
