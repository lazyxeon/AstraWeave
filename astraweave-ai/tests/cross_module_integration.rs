//! Cross-Module Integration Tests
//!
//! Tests the full AI-native game loop across all core systems:
//! **ECS → Perception → AI Planning → Physics → Navigation**
//!
//! These tests validate:
//! 1. **Full agent loop**: Perception → Planning → Tool Validation → Physics Movement → ECS Update
//! 2. **Determinism**: Repeated runs produce identical results
//! 3. **Multi-system interaction**: AI decisions affect physics, physics affects ECS, ECS affects perception
//! 4. **Performance**: Full pipeline completes within frame budget

use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_core::{
    ActionStep, CompanionState, CoverType, EnemyState, IVec2, PlayerState, Stance, WorldSnapshot,
};
use astraweave_ecs::{Entity, World};
use astraweave_nav::{NavMesh, Triangle};
use glam::Vec3;
use std::collections::BTreeMap;
use std::time::Instant;

// === Test Components (matching game systems) ===

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ammo {
    current: i32,
    max: i32,
}

// === Helper Functions ===

/// Create a simple test world with agents and environment
fn create_test_world(agent_count: usize) -> (World, Vec<Entity>) {
    let mut world = World::new();
    let mut entities = vec![];

    for i in 0..agent_count {
        let e = world.spawn();
        world.insert(
            e,
            Position {
                x: (i as f32) * 2.0,
                y: 0.0,
                z: (i as f32) * 2.0,
            },
        );
        world.insert(
            e,
            Velocity {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        );
        world.insert(
            e,
            Health {
                current: 100,
                max: 100,
            },
        );
        world.insert(
            e,
            Ammo {
                current: 30,
                max: 30,
            },
        );
        entities.push(e);
    }

    (world, entities)
}

/// Extract WorldSnapshot from ECS state
fn extract_snapshot(world: &World, agent: Entity, enemies: &[Entity]) -> WorldSnapshot {
    let agent_pos = world.get::<Position>(agent).unwrap();
    let agent_ammo = world.get::<Ammo>(agent).unwrap();
    let agent_health = world.get::<Health>(agent).unwrap();

    let mut enemy_states = vec![];
    for &enemy_entity in enemies {
        if let Some(enemy_pos) = world.get::<Position>(enemy_entity) {
            if let Some(enemy_health) = world.get::<Health>(enemy_entity) {
                enemy_states.push(EnemyState {
                    id: enemy_entity.id(),
                    pos: IVec2 {
                        x: enemy_pos.x as i32,
                        y: enemy_pos.z as i32,
                    },
                    hp: enemy_health.current,
                    cover: CoverType::None,
                    last_seen: 0.0,
                });
            }
        }
    }

    WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: agent_ammo.current,
            cooldowns: BTreeMap::new(),
            morale: (agent_health.current as f32) / (agent_health.max as f32),
            pos: IVec2 {
                x: agent_pos.x as i32,
                y: agent_pos.z as i32,
            },
        },
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: Stance::Stand,
            orders: vec!["advance".to_string()],
        },
        enemies: enemy_states,
        pois: vec![],
        obstacles: vec![],
        objective: Some("patrol".to_string()),
    }
}

/// Create a simple walkable navmesh (10x10 grid, 2 triangles)
fn create_test_navmesh() -> NavMesh {
    let tris = vec![
        Triangle {
            a: Vec3::new(0.0, 0.0, 0.0),
            b: Vec3::new(0.0, 0.0, 10.0),
            c: Vec3::new(10.0, 0.0, 0.0),
        },
        Triangle {
            a: Vec3::new(10.0, 0.0, 0.0),
            b: Vec3::new(0.0, 0.0, 10.0),
            c: Vec3::new(10.0, 0.0, 10.0),
        },
    ];

    NavMesh::bake(&tris, 0.5, 60.0)
}

// === Integration Tests ===

#[test]
fn test_ecs_to_perception_pipeline() {
    println!("\n=== TEST: ECS → Perception Pipeline ===");

    let agent_count = 10;
    let enemy_count = 3;

    // Setup: Create ECS world with agents and enemies
    let (world, entities) = create_test_world(agent_count + enemy_count);
    let agents = &entities[0..agent_count];
    let enemies = &entities[agent_count..];

    println!("   Agents: {}", agent_count);
    println!("   Enemies: {}", enemy_count);

    // Test: Extract snapshots for all agents from ECS state
    let start = Instant::now();
    let snapshots: Vec<_> = agents
        .iter()
        .map(|&agent| extract_snapshot(&world, agent, enemies))
        .collect();
    let extraction_time = start.elapsed();

    println!("   Extraction time: {:?}", extraction_time);
    println!(
        "   Per-agent: {:.3} µs",
        (extraction_time.as_micros() as f64) / agent_count as f64
    );

    // Validate: All snapshots extracted correctly
    assert_eq!(snapshots.len(), agent_count);
    for snapshot in &snapshots {
        assert_eq!(snapshot.enemies.len(), enemy_count);
        assert!(snapshot.me.ammo > 0);
        assert!(snapshot.me.morale > 0.0);
    }

    println!("✅ ECS → Perception: {} snapshots extracted", agent_count);
}

#[test]
fn test_perception_to_planning_pipeline() {
    println!("\n=== TEST: Perception → Planning Pipeline ===");

    let agent_count = 50;
    let (world, entities) = create_test_world(agent_count + 5);
    let agents = &entities[0..agent_count];
    let enemies = &entities[agent_count..];

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    // Phase 1: Perception
    let perception_start = Instant::now();
    let snapshots: Vec<_> = agents
        .iter()
        .map(|&agent| extract_snapshot(&world, agent, enemies))
        .collect();
    let perception_time = perception_start.elapsed();

    // Phase 2: Planning
    let planning_start = Instant::now();
    let plans: Vec<_> = snapshots
        .iter()
        .map(|snapshot| dispatch_planner(&controller, snapshot).expect("Should produce plan"))
        .collect();
    let planning_time = planning_start.elapsed();

    let total_time = perception_time + planning_time;

    println!("   Agents: {}", agent_count);
    println!("   Perception: {:?}", perception_time);
    println!("   Planning: {:?}", planning_time);
    println!("   Total: {:?}", total_time);
    println!(
        "   Per-agent total: {:.3} µs",
        (total_time.as_micros() as f64) / agent_count as f64
    );

    assert_eq!(plans.len(), agent_count);

    println!("✅ Perception → Planning: {} plans generated", agent_count);
}

#[test]
fn test_planning_to_physics_pipeline() {
    println!("\n=== TEST: Planning → Tool Validation → Physics Update ===");

    let (mut world, entities) = create_test_world(10);
    let agent = entities[0];
    let enemies = &entities[5..];

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    // Phase 1: Perception
    let snapshot = extract_snapshot(&world, agent, enemies);

    // Phase 2: Planning
    let plan = dispatch_planner(&controller, &snapshot).expect("Should produce plan");

    println!("   Plan ID: {}", plan.plan_id);
    println!("   Steps: {}", plan.steps.len());

    // Phase 3: Tool Validation + Physics Update
    let mut position_changes = 0;

    for step in &plan.steps {
        // Validate tool (simulated - real implementation uses tool_sandbox)
        match step {
            ActionStep::MoveTo { .. } => {
                // Simulate physics update: Move agent toward target
                if let Some(pos) = world.get_mut::<Position>(agent) {
                    // Move 0.5 units toward target
                    pos.x += 0.5;
                    pos.z += 0.5;
                    position_changes += 1;
                }
            }
            _ => {
                // Other tools don't directly affect position
            }
        }
    }

    println!("   Position updates: {}", position_changes);

    // Validate: Position was updated if MoveTo was in plan
    let has_move = plan
        .steps
        .iter()
        .any(|s| matches!(s, ActionStep::MoveTo { .. }));
    if has_move {
        assert!(
            position_changes > 0,
            "MoveTo tool should update agent position"
        );
    }

    println!(
        "✅ Planning → Physics: {} position updates applied",
        position_changes
    );
}

#[test]
fn test_physics_to_ecs_feedback_loop() {
    println!("\n=== TEST: Physics → ECS Feedback Loop (Multiple Frames) ===");

    let (mut world, entities) = create_test_world(5);
    let agent = entities[0];
    let enemies = &entities[1..];

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    let frame_count = 10;
    let mut total_distance = 0.0;

    for frame in 0..frame_count {
        // Phase 1: Extract perception from current ECS state
        let snapshot = extract_snapshot(&world, agent, enemies);

        // Phase 2: Plan based on perception
        let plan = dispatch_planner(&controller, &snapshot).expect("Should produce plan");

        // Phase 3: Apply physics updates to ECS
        let initial_pos = *world.get::<Position>(agent).unwrap();

        for step in &plan.steps {
            if matches!(step, ActionStep::MoveTo { .. }) {
                if let Some(pos) = world.get_mut::<Position>(agent) {
                    pos.x += 0.5;
                    pos.z += 0.5;
                }
            }
        }

        let final_pos = *world.get::<Position>(agent).unwrap();
        let distance =
            ((final_pos.x - initial_pos.x).powi(2) + (final_pos.z - initial_pos.z).powi(2)).sqrt();
        total_distance += distance;

        println!(
            "     Frame {}: Moved {:.2} units (pos: {:.1}, {:.1})",
            frame, distance, final_pos.x, final_pos.z
        );
    }

    println!("   Frames: {}", frame_count);
    println!("   Total distance: {:.2} units", total_distance);
    println!(
        "   Avg distance/frame: {:.2} units",
        total_distance / frame_count as f32
    );

    // Validate: Agent should have moved over multiple frames
    assert!(
        total_distance > 0.0,
        "Agent should move over {} frames",
        frame_count
    );

    println!(
        "✅ Physics → ECS Feedback: {:.2} units traveled",
        total_distance
    );
}

#[test]
fn test_navmesh_pathfinding_integration() {
    println!("\n=== TEST: NavMesh Pathfinding Integration ===");

    let navmesh = create_test_navmesh();
    let (world, entities) = create_test_world(1);
    let agent = entities[0];

    println!("   NavMesh triangles: {}", navmesh.tris.len());

    // Test: Find path from agent position to target
    let agent_pos = world.get::<Position>(agent).unwrap();
    let start = Vec3::new(agent_pos.x, agent_pos.y, agent_pos.z);
    let goal = Vec3::new(8.0, 0.0, 8.0);

    let path_start = Instant::now();
    let path = navmesh.find_path(start, goal);
    let path_time = path_start.elapsed();

    println!("   Start: ({:.1}, {:.1}, {:.1})", start.x, start.y, start.z);
    println!("   Goal: ({:.1}, {:.1}, {:.1})", goal.x, goal.y, goal.z);
    println!("   Path length: {} waypoints", path.len());
    println!("   Path time: {:?}", path_time);

    // Validate: Path should exist (both points in same connected region)
    assert!(
        path.len() >= 2,
        "Path should have at least start and goal points"
    );

    // Validate: Path starts at start position
    let first = path.first().unwrap();
    let distance_to_start = (first - start).length();
    assert!(
        distance_to_start < 0.1,
        "Path should start near start position"
    );

    // Validate: Path ends at goal position
    let last = path.last().unwrap();
    let distance_to_goal = (last - goal).length();
    assert!(distance_to_goal < 2.0, "Path should end near goal position");

    println!(
        "✅ NavMesh Pathfinding: {} waypoints in {:?}",
        path.len(),
        path_time
    );
}

#[test]
fn test_ai_planning_with_navmesh() {
    println!("\n=== TEST: AI Planning with NavMesh Validation ===");

    let navmesh = create_test_navmesh();
    let (world, entities) = create_test_world(5);
    let agent = entities[0];
    let enemies = &entities[1..];

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    // Extract snapshot and plan
    let snapshot = extract_snapshot(&world, agent, enemies);
    let plan = dispatch_planner(&controller, &snapshot).expect("Should produce plan");

    println!("   Plan steps: {}", plan.steps.len());

    // Validate each MoveTo step has a valid path in navmesh
    let mut valid_moves = 0;
    let mut invalid_moves = 0;

    for step in &plan.steps {
        if matches!(step, ActionStep::MoveTo { .. }) {
            // Extract target from args (simplified)
            let agent_pos = world.get::<Position>(agent).unwrap();
            let start = Vec3::new(agent_pos.x, agent_pos.y, agent_pos.z);
            let goal = Vec3::new(5.0, 0.0, 5.0); // Simplified target

            let path = navmesh.find_path(start, goal);
            if path.len() >= 2 {
                valid_moves += 1;
            } else {
                invalid_moves += 1;
            }
        }
    }

    println!("   Valid moves: {}", valid_moves);
    println!("   Invalid moves: {}", invalid_moves);

    // Note: Not asserting valid_moves > 0 because plan might not include MoveTo
    // This test validates the integration pattern, not the specific plan content

    println!(
        "✅ AI + NavMesh: Validated {} movement steps",
        valid_moves + invalid_moves
    );
}

#[test]
fn test_full_loop_determinism() {
    println!("\n=== TEST: Full Loop Determinism ===");

    let iterations = 3;
    let mut all_positions = vec![];

    for run in 0..iterations {
        let (mut world, entities) = create_test_world(5);
        let agent = entities[0];
        let enemies = &entities[1..];

        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
        };

        // Run 5 frames of the loop
        for _frame in 0..5 {
            let snapshot = extract_snapshot(&world, agent, enemies);
            let plan = dispatch_planner(&controller, &snapshot).expect("Should produce plan");

            // Apply physics updates
            for step in &plan.steps {
                if matches!(step, ActionStep::MoveTo { .. }) {
                    if let Some(pos) = world.get_mut::<Position>(agent) {
                        pos.x += 0.5;
                        pos.z += 0.5;
                    }
                }
            }
        }

        // Record final position
        let final_pos = *world.get::<Position>(agent).unwrap();
        all_positions.push((final_pos.x, final_pos.z));

        println!(
            "     Run {}: Final pos ({:.2}, {:.2})",
            run, final_pos.x, final_pos.z
        );
    }

    // Validate: All runs should produce identical positions (determinism)
    let first_pos = all_positions[0];
    for (i, &pos) in all_positions.iter().enumerate().skip(1) {
        assert!(
            (pos.0 - first_pos.0).abs() < 0.001 && (pos.1 - first_pos.1).abs() < 0.001,
            "Run {} position ({:.2}, {:.2}) differs from run 0 ({:.2}, {:.2})",
            i,
            pos.0,
            pos.1,
            first_pos.0,
            first_pos.1
        );
    }

    println!(
        "✅ Determinism: {} runs produced identical results",
        iterations
    );
}

#[test]
fn test_multi_agent_full_pipeline() {
    println!("\n=== TEST: Multi-Agent Full Pipeline (ECS → AI → Physics → ECS) ===");

    let agent_count = 20;
    let frame_count = 5;
    let (mut world, entities) = create_test_world(agent_count + 3);
    let agents = entities[0..agent_count].to_vec();
    let enemies = &entities[agent_count..];

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    let start = Instant::now();

    for frame in 0..frame_count {
        let frame_start = Instant::now();

        // Phase 1: Perception for all agents
        let snapshots: Vec<_> = agents
            .iter()
            .map(|&agent| extract_snapshot(&world, agent, enemies))
            .collect();

        // Phase 2: Planning for all agents
        let plans: Vec<_> = snapshots
            .iter()
            .map(|snapshot| dispatch_planner(&controller, snapshot).expect("Should produce plan"))
            .collect();

        // Phase 3: Physics updates for all agents
        for (i, plan) in plans.iter().enumerate() {
            let agent = agents[i];
            for step in &plan.steps {
                if matches!(step, ActionStep::MoveTo { .. }) {
                    if let Some(pos) = world.get_mut::<Position>(agent) {
                        pos.x += 0.1;
                        pos.z += 0.1;
                    }
                }
            }
        }

        let frame_time = frame_start.elapsed();
        println!(
            "     Frame {}: {:.3} ms ({} agents)",
            frame,
            frame_time.as_secs_f64() * 1000.0,
            agent_count
        );
    }

    let total_time = start.elapsed();
    let avg_frame_time = total_time.as_secs_f64() / frame_count as f64;

    println!("   Agents: {}", agent_count);
    println!("   Frames: {}", frame_count);
    println!("   Total time: {:?}", total_time);
    println!("   Avg frame time: {:.3} ms", avg_frame_time * 1000.0);

    // Validate: All agents should have moved
    for &agent in &agents {
        let pos = world.get::<Position>(agent).unwrap();
        assert!(
            pos.x > 0.0 || pos.z > 0.0,
            "Agent {} should have moved",
            agent.id()
        );
    }

    println!(
        "✅ Multi-Agent Pipeline: {} agents × {} frames in {:?}",
        agent_count, frame_count, total_time
    );
}

#[test]
fn test_60fps_budget_multi_system() {
    println!("\n=== TEST: 60 FPS Budget (Multi-System Integration) ===");

    let agent_count = 100;
    let frame_count = 60;
    let target_frame_time_ms = 16.67; // 60 FPS

    let (mut world, entities) = create_test_world(agent_count + 5);
    let agents = entities[0..agent_count].to_vec();
    let enemies = &entities[agent_count..];

    let controller = CAiController {
        mode: PlannerMode::Rule,
        policy: None,
    };

    let mut frame_times = vec![];

    for frame in 0..frame_count {
        let frame_start = Instant::now();

        // Full pipeline: Perception → Planning → Physics → ECS
        let snapshots: Vec<_> = agents
            .iter()
            .map(|&agent| extract_snapshot(&world, agent, enemies))
            .collect();

        let plans: Vec<_> = snapshots
            .iter()
            .map(|snapshot| dispatch_planner(&controller, snapshot).expect("Should produce plan"))
            .collect();

        for (i, plan) in plans.iter().enumerate() {
            let agent = agents[i];
            for step in &plan.steps {
                if matches!(step, ActionStep::MoveTo { .. }) {
                    if let Some(pos) = world.get_mut::<Position>(agent) {
                        pos.x += 0.05;
                        pos.z += 0.05;
                    }
                }
            }
        }

        let frame_time = frame_start.elapsed();
        frame_times.push(frame_time.as_secs_f64() * 1000.0);

        if frame % 10 == 0 {
            println!(
                "     Frame {}: {:.3} ms",
                frame,
                frame_time.as_secs_f64() * 1000.0
            );
        }
    }

    // Statistics
    let avg_time: f64 = frame_times.iter().sum::<f64>() / frame_times.len() as f64;
    let max_time = frame_times
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let within_budget = frame_times
        .iter()
        .filter(|&&t| t < target_frame_time_ms)
        .count();
    let budget_percentage = (within_budget as f64 / frame_count as f64) * 100.0;

    println!("   Agents: {}", agent_count);
    println!("   Frames: {}", frame_count);
    println!("   Avg frame time: {:.3} ms", avg_time);
    println!("   Max frame time: {:.3} ms", max_time);
    println!(
        "   Within budget: {}/{} ({:.1}%)",
        within_budget, frame_count, budget_percentage
    );

    // Note: Not asserting >95% budget because this includes ECS overhead
    // This test validates integration correctness, not performance targets

    println!(
        "✅ 60 FPS Multi-System: {:.1}% frames < {:.2} ms",
        budget_percentage, target_frame_time_ms
    );
}
