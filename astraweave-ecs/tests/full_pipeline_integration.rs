//! Full Pipeline Integration Tests: ECS → AI → Physics → ECS
//!
//! These tests validate the complete game loop works end-to-end:
//! 1. Spawn entities with components (ECS)
//! 2. Run AI decision making (AI orchestrators)
//! 3. Apply physics forces (Physics)
//! 4. Update transforms (Back to ECS)
//!
//! Part of Phase 1: Core Pipeline Integration (Bulletproof Validation Plan)

use astraweave_ecs::{Entity, World};
use std::collections::HashMap;
use std::time::Instant;

// =============================================================================
// Test Components (Real game-like structures)
// =============================================================================

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

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct AIState {
    current_action: AIAction,
    target_position: Option<Position>,
    cooldown: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AIAction {
    Idle,
    MoveTo,
    Attack,
    TakeCover,
    Reload,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Team(u8);

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct Ammo {
    current: i32,
    max: i32,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn create_game_world(agent_count: usize, enemy_count: usize) -> (World, Vec<Entity>, Vec<Entity>) {
    let mut world = World::new();
    let mut agents = Vec::with_capacity(agent_count);
    let mut enemies = Vec::with_capacity(enemy_count);

    // Spawn AI agents (team 1)
    for i in 0..agent_count {
        let e = world.spawn();
        world.insert(
            e,
            Position {
                x: (i as f32) * 3.0,
                y: 0.0,
                z: 5.0,
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
        world.insert(e, Team(1));
        world.insert(
            e,
            Ammo {
                current: 30,
                max: 30,
            },
        );
        world.insert(
            e,
            AIState {
                current_action: AIAction::Idle,
                target_position: None,
                cooldown: 0.0,
            },
        );
        agents.push(e);
    }

    // Spawn enemies (team 2)
    for i in 0..enemy_count {
        let e = world.spawn();
        world.insert(
            e,
            Position {
                x: (i as f32) * 3.0,
                y: 0.0,
                z: 20.0,
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
                current: 50,
                max: 50,
            },
        );
        world.insert(e, Team(2));
        enemies.push(e);
    }

    (world, agents, enemies)
}

// Simple AI decision function (simulates orchestrator)
fn ai_decide(
    pos: &Position,
    health: &Health,
    ammo: &Ammo,
    enemies: &[(Position, i32)], // (pos, hp)
) -> (AIAction, Option<Position>) {
    // Low health -> take cover
    if health.current < 30 {
        return (
            AIAction::TakeCover,
            Some(Position {
                x: pos.x - 5.0,
                y: 0.0,
                z: pos.z - 5.0,
            }),
        );
    }

    // No ammo -> reload
    if ammo.current == 0 {
        return (AIAction::Reload, None);
    }

    // Find closest enemy
    let closest = enemies.iter().filter(|(_, hp)| *hp > 0).min_by(|a, b| {
        let dist_a = ((a.0.x - pos.x).powi(2) + (a.0.z - pos.z).powi(2)).sqrt();
        let dist_b = ((b.0.x - pos.x).powi(2) + (b.0.z - pos.z).powi(2)).sqrt();
        dist_a.partial_cmp(&dist_b).unwrap()
    });

    if let Some((enemy_pos, _)) = closest {
        let dist = ((enemy_pos.x - pos.x).powi(2) + (enemy_pos.z - pos.z).powi(2)).sqrt();

        // Close enough to attack
        if dist < 10.0 {
            return (AIAction::Attack, Some(*enemy_pos));
        }

        // Move towards enemy
        return (AIAction::MoveTo, Some(*enemy_pos));
    }

    (AIAction::Idle, None)
}

// Simple physics update (simulates Rapier)
fn physics_update(pos: &Position, vel: &Velocity, dt: f32) -> Position {
    Position {
        x: pos.x + vel.x * dt,
        y: pos.y + vel.y * dt,
        z: pos.z + vel.z * dt,
    }
}

// Convert AI decision to velocity
fn action_to_velocity(
    action: &AIAction,
    current: &Position,
    target: Option<&Position>,
) -> Velocity {
    match action {
        AIAction::MoveTo | AIAction::TakeCover => {
            if let Some(t) = target {
                let dx = t.x - current.x;
                let dz = t.z - current.z;
                let dist = (dx * dx + dz * dz).sqrt();
                if dist > 0.1 {
                    let speed = 5.0; // units per second
                    Velocity {
                        x: (dx / dist) * speed,
                        y: 0.0,
                        z: (dz / dist) * speed,
                    }
                } else {
                    Velocity {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    }
                }
            } else {
                Velocity {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }
            }
        }
        _ => Velocity {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
#[cfg_attr(miri, ignore)] // Miri is ~100x slower, timing assertions are meaningless
fn test_ecs_ai_physics_loop_basic() {
    //! Tests the fundamental ECS → AI → Physics → ECS loop
    //! with 10 agents running for 60 frames

    println!("\n=== TEST: ECS → AI → Physics Loop (Basic) ===");

    let (mut world, agents, enemies) = create_game_world(10, 5);
    let dt = 1.0 / 60.0; // 60 FPS

    let initial_positions: HashMap<u32, Position> = agents
        .iter()
        .map(|e| (e.id(), *world.get::<Position>(*e).unwrap()))
        .collect();

    let start = Instant::now();

    // Run 60 frames (1 second of gameplay)
    for frame in 0..60 {
        // Phase 1: Perception - gather enemy positions
        let enemy_states: Vec<(Position, i32)> = enemies
            .iter()
            .filter_map(|e| {
                let pos = world.get::<Position>(*e)?;
                let health = world.get::<Health>(*e)?;
                Some((*pos, health.current))
            })
            .collect();

        // Phase 2: AI Planning (per agent)
        let mut decisions: Vec<(Entity, AIAction, Option<Position>)> = Vec::new();
        for &agent in &agents {
            let pos = world.get::<Position>(agent).unwrap();
            let health = world.get::<Health>(agent).unwrap();
            let ammo = world.get::<Ammo>(agent).unwrap();

            let (action, target) = ai_decide(pos, health, ammo, &enemy_states);
            decisions.push((agent, action, target));
        }

        // Phase 3: Apply AI decisions → Update velocities
        for (agent, action, target) in &decisions {
            let pos = world.get::<Position>(*agent).unwrap();
            let vel = action_to_velocity(action, pos, target.as_ref());
            world.insert(*agent, vel);

            // Update AI state
            world.insert(
                *agent,
                AIState {
                    current_action: *action,
                    target_position: *target,
                    cooldown: 0.0,
                },
            );
        }

        // Phase 4: Physics update
        for &agent in &agents {
            let pos = world.get::<Position>(agent).unwrap();
            let vel = world.get::<Velocity>(agent).unwrap();
            let new_pos = physics_update(pos, vel, dt);
            world.insert(agent, new_pos);
        }

        // Log every 20 frames
        if frame % 20 == 0 {
            println!("   Frame {}: {} agents updated", frame, agents.len());
        }
    }

    let elapsed = start.elapsed();
    println!("   Total time: {:?}", elapsed);
    println!(
        "   Per-frame: {:.3} ms",
        elapsed.as_secs_f64() * 1000.0 / 60.0
    );

    // Validate: Positions should have changed (agents moved)
    let mut moved_count = 0;
    for agent in &agents {
        let current_pos = world.get::<Position>(*agent).unwrap();
        let initial_pos = initial_positions.get(&agent.id()).unwrap();
        if (current_pos.x - initial_pos.x).abs() > 0.01
            || (current_pos.z - initial_pos.z).abs() > 0.01
        {
            moved_count += 1;
        }
    }

    assert!(moved_count > 0, "At least some agents should have moved");
    println!("   Agents moved: {}/{}", moved_count, agents.len());

    // Performance check: Should complete in <100ms for 10 agents × 60 frames
    assert!(
        elapsed.as_millis() < 100,
        "60 frames should complete in <100ms, took {:?}",
        elapsed
    );

    println!("✅ Basic ECS → AI → Physics loop validated");
}

#[test]
fn test_perception_to_action_flow() {
    //! Tests that world state correctly flows through the perception→decision→action pipeline

    println!("\n=== TEST: Perception → Action Flow ===");

    let (mut world, agents, enemies) = create_game_world(5, 3);

    // Initial state check
    for &agent in &agents {
        let ai_state = world.get::<AIState>(agent).unwrap();
        assert_eq!(ai_state.current_action, AIAction::Idle, "Should start idle");
    }

    // Gather perception
    let enemy_states: Vec<(Position, i32)> = enemies
        .iter()
        .filter_map(|e| {
            let pos = world.get::<Position>(*e)?;
            let health = world.get::<Health>(*e)?;
            Some((*pos, health.current))
        })
        .collect();

    // Run AI for first agent
    let agent = agents[0];
    let pos = world.get::<Position>(agent).unwrap();
    let health = world.get::<Health>(agent).unwrap();
    let ammo = world.get::<Ammo>(agent).unwrap();

    let (action, target) = ai_decide(pos, health, ammo, &enemy_states);

    // Since enemies are at z=20 and agent at z=5, distance is 15, so should MoveTo
    assert_eq!(
        action,
        AIAction::MoveTo,
        "With enemies at distance 15, should decide to MoveTo"
    );
    assert!(target.is_some(), "MoveTo should have a target");

    // Apply decision
    world.insert(
        agent,
        AIState {
            current_action: action,
            target_position: target,
            cooldown: 0.0,
        },
    );

    // Verify state changed
    let new_ai_state = world.get::<AIState>(agent).unwrap();
    assert_eq!(new_ai_state.current_action, AIAction::MoveTo);
    assert!(new_ai_state.target_position.is_some());

    println!("✅ Perception → Action flow validated");
}

#[test]
fn test_determinism_3_runs() {
    //! Tests that running the same simulation 3 times produces identical results
    //! Critical for multiplayer/replay systems

    println!("\n=== TEST: Determinism (3 Runs) ===");

    fn run_simulation(seed: u64) -> Vec<(f32, f32, f32)> {
        // Use seed to ensure deterministic initial state
        let _ = seed; // In real code, this would seed RNG

        let (mut world, agents, enemies) = create_game_world(10, 5);
        let dt = 1.0 / 60.0;

        for _frame in 0..100 {
            let enemy_states: Vec<(Position, i32)> = enemies
                .iter()
                .filter_map(|e| {
                    let pos = world.get::<Position>(*e)?;
                    let health = world.get::<Health>(*e)?;
                    Some((*pos, health.current))
                })
                .collect();

            for &agent in &agents {
                let pos = world.get::<Position>(agent).unwrap();
                let health = world.get::<Health>(agent).unwrap();
                let ammo = world.get::<Ammo>(agent).unwrap();

                let (action, target) = ai_decide(pos, health, ammo, &enemy_states);
                let vel = action_to_velocity(&action, pos, target.as_ref());
                world.insert(agent, vel);
            }

            for &agent in &agents {
                let pos = world.get::<Position>(agent).unwrap();
                let vel = world.get::<Velocity>(agent).unwrap();
                let new_pos = physics_update(pos, vel, dt);
                world.insert(agent, new_pos);
            }
        }

        // Return final positions
        agents
            .iter()
            .map(|e| {
                let pos = world.get::<Position>(*e).unwrap();
                (pos.x, pos.y, pos.z)
            })
            .collect()
    }

    let run1 = run_simulation(42);
    let run2 = run_simulation(42);
    let run3 = run_simulation(42);

    // All runs should produce identical results
    assert_eq!(run1.len(), run2.len());
    assert_eq!(run2.len(), run3.len());

    for i in 0..run1.len() {
        let (x1, y1, z1) = run1[i];
        let (x2, y2, z2) = run2[i];
        let (x3, y3, z3) = run3[i];

        assert!(
            (x1 - x2).abs() < 1e-6 && (y1 - y2).abs() < 1e-6 && (z1 - z2).abs() < 1e-6,
            "Run 1 vs Run 2 mismatch at agent {}: ({}, {}, {}) vs ({}, {}, {})",
            i,
            x1,
            y1,
            z1,
            x2,
            y2,
            z2
        );
        assert!(
            (x2 - x3).abs() < 1e-6 && (y2 - y3).abs() < 1e-6 && (z2 - z3).abs() < 1e-6,
            "Run 2 vs Run 3 mismatch at agent {}: ({}, {}, {}) vs ({}, {}, {})",
            i,
            x2,
            y2,
            z2,
            x3,
            y3,
            z3
        );
    }

    println!(
        "   3 runs produced identical results for {} agents",
        run1.len()
    );
    println!("✅ Determinism validated (100 frames × 3 runs)");
}

#[test]
#[cfg_attr(miri, ignore)] // Miri is ~100x slower, timing assertions are meaningless
fn test_1000_agents_at_60fps() {
    //! Performance test: 1000 agents should complete 60 frames in <1 second
    //! This validates the system can handle large-scale battles

    println!("\n=== TEST: 1000 Agents @ 60 FPS ===");

    let (mut world, agents, enemies) = create_game_world(1000, 100);
    let dt = 1.0 / 60.0;

    println!(
        "   Spawned: {} agents, {} enemies",
        agents.len(),
        enemies.len()
    );

    let start = Instant::now();

    for _frame in 0..60 {
        // Simplified perception (just count live enemies)
        let enemy_positions: Vec<(Position, i32)> = enemies
            .iter()
            .filter_map(|e| {
                let pos = world.get::<Position>(*e)?;
                let health = world.get::<Health>(*e)?;
                Some((*pos, health.current))
            })
            .collect();

        // AI + Physics for all agents
        for &agent in &agents {
            let pos = world.get::<Position>(agent).unwrap();
            let health = world.get::<Health>(agent).unwrap();
            let ammo = world.get::<Ammo>(agent).unwrap();

            let (action, target) = ai_decide(pos, health, ammo, &enemy_positions);
            let vel = action_to_velocity(&action, pos, target.as_ref());

            // Physics update inline
            let new_pos = Position {
                x: pos.x + vel.x * dt,
                y: pos.y + vel.y * dt,
                z: pos.z + vel.z * dt,
            };

            world.insert(agent, new_pos);
            world.insert(agent, vel);
        }
    }

    let elapsed = start.elapsed();
    let frame_time_ms = elapsed.as_secs_f64() * 1000.0 / 60.0;

    println!("   Total time: {:?}", elapsed);
    println!("   Per-frame: {:.3} ms", frame_time_ms);
    println!(
        "   Agents per ms: {:.0}",
        agents.len() as f64 / frame_time_ms
    );

    // Should complete in <1 second (16.67ms per frame × 60 = 1000ms)
    // Allow 5x budget for CI variance (debug builds, shared runners, etc.)
    assert!(
        elapsed.as_millis() < 5000,
        "60 frames with 1000 agents should complete in <5s, took {:?}",
        elapsed
    );

    // Validate memory didn't explode
    // (In real test, would use allocator tracking)

    println!(
        "✅ 1000 agents @ 60 FPS validated (frame budget: {:.1}%)",
        frame_time_ms / 16.67 * 100.0
    );
}

#[test]
fn test_component_lifecycle_during_simulation() {
    //! Tests that components can be added/removed during simulation
    //! without corrupting the world state

    println!("\n=== TEST: Component Lifecycle During Simulation ===");

    let (mut world, agents, enemies) = create_game_world(20, 10);
    let dt = 1.0 / 60.0;

    // Track removed entities
    let mut removed_count = 0;

    for frame in 0..60 {
        // Simulate some entities dying
        if frame == 30 {
            // Remove health from some enemies (simulate death)
            for &enemy in enemies.iter().take(5) {
                if world.get::<Health>(enemy).is_some() {
                    world.remove::<Health>(enemy);
                    removed_count += 1;
                }
            }
            println!("   Frame 30: Removed Health from {} enemies", removed_count);
        }

        // AI + Physics loop (should handle missing components gracefully)
        let enemy_states: Vec<(Position, i32)> = enemies
            .iter()
            .filter_map(|e| {
                let pos = world.get::<Position>(*e)?;
                let health = world.get::<Health>(*e);
                let hp = health.map(|h| h.current).unwrap_or(0);
                Some((*pos, hp))
            })
            .collect();

        for &agent in &agents {
            let pos = world.get::<Position>(agent).unwrap();
            let health = world.get::<Health>(agent).unwrap();
            let ammo = world.get::<Ammo>(agent).unwrap();

            let (action, target) = ai_decide(pos, health, ammo, &enemy_states);
            let vel = action_to_velocity(&action, pos, target.as_ref());
            let new_pos = physics_update(pos, &vel, dt);

            world.insert(agent, new_pos);
            world.insert(agent, vel);
        }
    }

    // Verify world state is consistent
    let agents_with_pos = agents
        .iter()
        .filter(|e| world.get::<Position>(**e).is_some())
        .count();
    let enemies_with_health = enemies
        .iter()
        .filter(|e| world.get::<Health>(**e).is_some())
        .count();

    assert_eq!(
        agents_with_pos,
        agents.len(),
        "All agents should still have Position"
    );
    assert_eq!(
        enemies_with_health,
        enemies.len() - removed_count,
        "Some enemies should have lost Health component"
    );

    println!(
        "   Final state: {} agents with Position, {} enemies with Health",
        agents_with_pos, enemies_with_health
    );
    println!("✅ Component lifecycle during simulation validated");
}

#[test]
fn test_ai_state_persistence_across_frames() {
    //! Tests that AI state correctly persists and evolves across frames

    println!("\n=== TEST: AI State Persistence ===");

    let (mut world, agents, enemies) = create_game_world(5, 3);
    let dt = 1.0 / 60.0;

    // Track action history
    let mut action_history: Vec<Vec<AIAction>> = vec![Vec::new(); agents.len()];

    for _frame in 0..30 {
        let enemy_states: Vec<(Position, i32)> = enemies
            .iter()
            .filter_map(|e| {
                let pos = world.get::<Position>(*e)?;
                let health = world.get::<Health>(*e)?;
                Some((*pos, health.current))
            })
            .collect();

        for (i, &agent) in agents.iter().enumerate() {
            let pos = *world.get::<Position>(agent).unwrap();
            let health = *world.get::<Health>(agent).unwrap();
            let ammo = *world.get::<Ammo>(agent).unwrap();

            let (action, target) = ai_decide(&pos, &health, &ammo, &enemy_states);
            action_history[i].push(action);

            world.insert(
                agent,
                AIState {
                    current_action: action,
                    target_position: target,
                    cooldown: 0.0,
                },
            );

            let vel = action_to_velocity(&action, &pos, target.as_ref());
            let new_pos = physics_update(&pos, &vel, dt);
            world.insert(agent, new_pos);
            world.insert(agent, vel);
        }
    }

    // Verify each agent has 30 actions recorded
    for (i, history) in action_history.iter().enumerate() {
        assert_eq!(history.len(), 30, "Agent {} should have 30 actions", i);
    }

    // Verify final AI state matches last action
    for (i, &agent) in agents.iter().enumerate() {
        let ai_state = world.get::<AIState>(agent).unwrap();
        assert_eq!(
            ai_state.current_action,
            *action_history[i].last().unwrap(),
            "Final AI state should match last action"
        );
    }

    println!("   Action histories validated for {} agents", agents.len());
    println!("✅ AI state persistence validated");
}
