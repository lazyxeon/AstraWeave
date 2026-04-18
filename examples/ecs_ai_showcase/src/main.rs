//! ECS AI Showcase - Demonstrates the AI-native ECS capabilities
//!
//! This example shows:
//! - AI Perception → Reasoning → Planning → Action loop
//! - Event-driven AI behaviors
//! - System stage ordering
//! - Query ergonomics with QueryMut
//! - Resource management

use anyhow::Result;
use astraweave_ecs::{App, Entity, Event, Events, World};
use glam::Vec3;
use std::collections::HashMap;

// Global allocator selection — same pattern as examples/profiling_demo.
// - `alloc-counter` on → CountingAlloc is the global allocator (wraps MiMalloc
//   when `fast-alloc` is also on, otherwise wraps System).
// - `alloc-counter` off, `fast-alloc` on → MiMalloc installed directly via
//   astraweave-alloc's `setup_global_allocator!`.
// - Neither on → platform default.
#[cfg(feature = "alloc-counter")]
#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

#[cfg(all(feature = "fast-alloc", not(feature = "alloc-counter")))]
astraweave_alloc::setup_global_allocator!();

// ============================================================================
// Components
// ============================================================================

#[derive(Clone, Copy, Debug)]
struct Position {
    pos: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct Velocity {
    vel: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum Team {
    Player,
    Enemy,
    Neutral,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Clone, Debug)]
struct AIAgent {
    perception_radius: f32,
    target: Option<Entity>,
    state: AIState,
}

#[derive(Clone, Debug, PartialEq)]
enum AIState {
    Idle,
    Patrolling,
    Chasing,
    Attacking,
    Fleeing,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct Player {
    name: String,
}

// ============================================================================
// Resources
// ============================================================================

#[derive(Clone, Debug)]
struct GameTime {
    tick: u64,
    delta_time: f32,
}

#[derive(Clone, Debug, Default)]
struct GameStats {
    enemies_defeated: u32,
    player_deaths: u32,
    total_damage_dealt: i32,
}

// ============================================================================
// Events
// ============================================================================

#[derive(Clone, Debug)]
struct DamageEvent {
    attacker: Entity,
    target: Entity,
    damage: i32,
}
impl Event for DamageEvent {}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct HealthChangedEvent {
    entity: Entity,
    old_health: i32,
    new_health: i32,
    source: Option<Entity>,
}
impl Event for HealthChangedEvent {}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct AIStateChangedEvent {
    entity: Entity,
    old_state: AIState,
    new_state: AIState,
}
impl Event for AIStateChangedEvent {}

// ============================================================================
// Systems - Perception Stage
// ============================================================================

/// AI Perception: Update AI agents' perception of the world
fn ai_perception_system(world: &mut World) {
    // Get all AI agents
    let ai_entities: Vec<Entity> = world.entities_with::<AIAgent>();

    // For each AI agent, find nearby enemies
    for agent_entity in ai_entities {
        let agent_pos = if let Some(pos) = world.get::<Position>(agent_entity) {
            pos.pos
        } else {
            continue;
        };

        let perception_radius = if let Some(ai) = world.get::<AIAgent>(agent_entity) {
            ai.perception_radius
        } else {
            continue;
        };

        // Find closest enemy
        let mut closest_enemy: Option<(Entity, f32)> = None;

        let all_entities: Vec<Entity> = world.entities_with::<Position>();
        for other_entity in all_entities {
            if other_entity == agent_entity {
                continue;
            }

            // Check if enemy team
            let is_enemy = matches!(
                (
                    world.get::<Team>(agent_entity),
                    world.get::<Team>(other_entity),
                ),
                (Some(Team::Enemy), Some(Team::Player)) | (Some(Team::Player), Some(Team::Enemy))
            );

            if !is_enemy {
                continue;
            }

            if let Some(other_pos) = world.get::<Position>(other_entity) {
                let distance = (other_pos.pos - agent_pos).length();

                if distance <= perception_radius {
                    if let Some((_, closest_dist)) = closest_enemy {
                        if distance < closest_dist {
                            closest_enemy = Some((other_entity, distance));
                        }
                    } else {
                        closest_enemy = Some((other_entity, distance));
                    }
                }
            }
        }

        // Update AI target
        if let Some(ai) = world.get_mut::<AIAgent>(agent_entity) {
            ai.target = closest_enemy.map(|(e, _)| e);
        }
    }
}

// ============================================================================
// Systems - AI Planning Stage
// ============================================================================

/// AI Planning: Decide actions based on perception
fn ai_planning_system(world: &mut World) {
    let ai_entities: Vec<Entity> = world.entities_with::<AIAgent>();

    for entity in ai_entities {
        let (current_state, target, health) = {
            let ai = if let Some(ai) = world.get::<AIAgent>(entity) {
                ai
            } else {
                continue;
            };

            let health = world
                .get::<Health>(entity)
                .map(|h| h.current)
                .unwrap_or(100);
            (ai.state.clone(), ai.target, health)
        };

        // State machine for AI decision making
        let new_state = match current_state {
            AIState::Idle => {
                if target.is_some() {
                    AIState::Chasing
                } else {
                    AIState::Patrolling
                }
            }
            AIState::Patrolling => {
                if target.is_some() {
                    AIState::Chasing
                } else {
                    AIState::Patrolling
                }
            }
            AIState::Chasing => {
                if target.is_none() {
                    AIState::Idle
                } else if health < 30 {
                    AIState::Fleeing
                } else {
                    // Check distance to target
                    if let Some(target_entity) = target {
                        if let (Some(my_pos), Some(target_pos)) = (
                            world.get::<Position>(entity),
                            world.get::<Position>(target_entity),
                        ) {
                            let distance = (target_pos.pos - my_pos.pos).length();
                            if distance < 2.0 {
                                AIState::Attacking
                            } else {
                                AIState::Chasing
                            }
                        } else {
                            AIState::Chasing
                        }
                    } else {
                        AIState::Idle
                    }
                }
            }
            AIState::Attacking => {
                if target.is_none() || health < 30 {
                    AIState::Fleeing
                } else {
                    AIState::Attacking
                }
            }
            AIState::Fleeing => {
                if health > 50 {
                    AIState::Idle
                } else {
                    AIState::Fleeing
                }
            }
        };

        // Update state and emit event if changed
        if new_state != current_state {
            if let Some(ai) = world.get_mut::<AIAgent>(entity) {
                ai.state = new_state.clone();
            }

            // Emit state changed event
            if let Some(events) = world.get_resource_mut::<Events>() {
                events.send(AIStateChangedEvent {
                    entity,
                    old_state: current_state,
                    new_state,
                });
            }
        }
    }
}

// ============================================================================
// Systems - Simulation Stage
// ============================================================================

/// Movement system: Apply velocity to position
fn movement_system(world: &mut World) {
    let delta_time = world
        .get_resource::<GameTime>()
        .map(|t| t.delta_time)
        .unwrap_or(1.0 / 60.0);

    let entities: Vec<Entity> = world.entities_with::<Position>();

    for entity in entities {
        // Copy velocity first to avoid borrow conflicts
        let vel = world.get::<Velocity>(entity).map(|v| v.vel);
        if let (Some(pos), Some(vel_val)) = (world.get_mut::<Position>(entity), vel) {
            pos.pos += vel_val * delta_time;
        }
    }
}

/// AI behavior execution based on state
fn ai_behavior_system(world: &mut World) {
    let ai_entities: Vec<Entity> = world.entities_with::<AIAgent>();

    for entity in ai_entities {
        let state = if let Some(ai) = world.get::<AIAgent>(entity) {
            ai.state.clone()
        } else {
            continue;
        };

        match state {
            AIState::Chasing => {
                // Move towards target
                if let Some(ai) = world.get::<AIAgent>(entity) {
                    if let Some(target) = ai.target {
                        if let (Some(my_pos), Some(target_pos)) =
                            (world.get::<Position>(entity), world.get::<Position>(target))
                        {
                            let direction = (target_pos.pos - my_pos.pos).normalize_or_zero();
                            let speed = 5.0;

                            if let Some(vel) = world.get_mut::<Velocity>(entity) {
                                vel.vel = direction * speed;
                            }
                        }
                    }
                }
            }
            AIState::Fleeing => {
                // Move away from target
                if let Some(ai) = world.get::<AIAgent>(entity) {
                    if let Some(target) = ai.target {
                        if let (Some(my_pos), Some(target_pos)) =
                            (world.get::<Position>(entity), world.get::<Position>(target))
                        {
                            let direction = (my_pos.pos - target_pos.pos).normalize_or_zero();
                            let speed = 7.0; // Flee faster

                            if let Some(vel) = world.get_mut::<Velocity>(entity) {
                                vel.vel = direction * speed;
                            }
                        }
                    }
                }
            }
            AIState::Attacking => {
                // Stop moving and attack
                if let Some(vel) = world.get_mut::<Velocity>(entity) {
                    vel.vel = Vec3::ZERO;
                }

                // Emit damage event
                if let Some(ai) = world.get::<AIAgent>(entity) {
                    if let Some(target) = ai.target {
                        if let Some(events) = world.get_resource_mut::<Events>() {
                            events.send(DamageEvent {
                                attacker: entity,
                                target,
                                damage: 10,
                            });
                        }
                    }
                }
            }
            AIState::Patrolling => {
                // Simple patrol: random movement
                if let Some(vel) = world.get_mut::<Velocity>(entity) {
                    vel.vel = Vec3::new(1.0, 0.0, 1.0).normalize() * 2.0;
                }
            }
            AIState::Idle => {
                if let Some(vel) = world.get_mut::<Velocity>(entity) {
                    vel.vel = Vec3::ZERO;
                }
            }
        }
    }
}

/// Combat system: Process damage events
fn combat_system(world: &mut World) {
    // Read damage events
    let damage_events: Vec<DamageEvent> = {
        if let Some(events) = world.get_resource::<Events>() {
            events.read::<DamageEvent>().cloned().collect()
        } else {
            Vec::new()
        }
    };

    let mut stats_update = (0, 0); // (damage_dealt, enemies_defeated)
    let mut health_changed_events = Vec::new();

    for event in damage_events {
        if let Some(health) = world.get_mut::<Health>(event.target) {
            health.current -= event.damage;
            stats_update.0 += event.damage;

            if health.current <= 0 {
                // Entity defeated
                stats_update.1 += 1;

                // Queue health changed event
                health_changed_events.push(HealthChangedEvent {
                    entity: event.target,
                    old_health: health.current + event.damage,
                    new_health: health.current,
                    source: Some(event.attacker),
                });
            }
        }
    }

    // Emit all health changed events
    if let Some(events) = world.get_resource_mut::<Events>() {
        for event in health_changed_events {
            events.send(event);
        }
    }

    // Update game stats
    if let Some(stats) = world.get_resource_mut::<GameStats>() {
        stats.total_damage_dealt += stats_update.0;
        stats.enemies_defeated += stats_update.1;
    }
}

// ============================================================================
// Systems - Post-Simulation Stage
// ============================================================================

/// Stats display system
fn stats_display_system(world: &mut World) {
    let tick = world
        .get_resource::<GameTime>()
        .map(|t| t.tick)
        .unwrap_or(0);

    // Print stats every 60 ticks (1 second at 60 FPS)
    if tick % 60 == 0 {
        if let Some(stats) = world.get_resource::<GameStats>() {
            println!("\n=== Game Stats (Tick {}) ===", tick);
            println!("Enemies Defeated: {}", stats.enemies_defeated);
            println!("Total Damage: {}", stats.total_damage_dealt);
            println!("Player Deaths: {}", stats.player_deaths);
        }

        // Count AI states
        let ai_entities: Vec<Entity> = world.entities_with::<AIAgent>();
        let mut state_counts: HashMap<String, u32> = HashMap::new();

        for entity in ai_entities {
            if let Some(ai) = world.get::<AIAgent>(entity) {
                let state_name = format!("{:?}", ai.state);
                *state_counts.entry(state_name).or_insert(0) += 1;
            }
        }

        println!("\n=== AI States ===");
        for (state, count) in state_counts {
            println!("{}: {}", state, count);
        }
    }
}

// ============================================================================
// Setup and Main
// ============================================================================

fn setup_world(app: &mut App, enemy_count: usize) {
    // Insert resources
    app.world.insert_resource(GameTime {
        tick: 0,
        delta_time: 1.0 / 60.0,
    });
    app.world.insert_resource(GameStats::default());
    app.world.insert_resource(Events::new());

    // Spawn player
    let player = app.world.spawn();
    app.world.insert(
        player,
        Player {
            name: "Hero".to_string(),
        },
    );
    app.world.insert(
        player,
        Position {
            pos: Vec3::new(0.0, 0.0, 0.0),
        },
    );
    app.world.insert(player, Velocity { vel: Vec3::ZERO });
    app.world.insert(
        player,
        Health {
            current: 100,
            max: 100,
        },
    );
    app.world.insert(player, Team::Player);

    // Spawn enemies
    for i in 0..enemy_count {
        let enemy = app.world.spawn();
        app.world.insert(
            enemy,
            Position {
                pos: Vec3::new(10.0 + i as f32 * 3.0, 0.0, 10.0),
            },
        );
        app.world.insert(enemy, Velocity { vel: Vec3::ZERO });
        app.world.insert(
            enemy,
            Health {
                current: 50,
                max: 50,
            },
        );
        app.world.insert(enemy, Team::Enemy);
        app.world.insert(
            enemy,
            AIAgent {
                perception_radius: 15.0,
                target: None,
                state: AIState::Idle,
            },
        );
    }

    println!("🎮 ECS AI Showcase initialized!");
    println!("   Player: 1");
    println!("   Enemies: {}", enemy_count);
    println!("   Running AI-native game loop: Perception → Planning → Simulation\n");
}

/// Parse `-e <count>` and `-f <ticks>` CLI args, matching profiling_demo.
/// Defaults (5 enemies, 300 ticks) preserve historical behaviour.
fn parse_args() -> (usize, u64) {
    let args: Vec<String> = std::env::args().collect();
    let mut enemy_count: usize = 5;
    let mut tick_count: u64 = 300;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--entities" | "-e" => {
                if i + 1 < args.len() {
                    enemy_count = args[i + 1].parse().unwrap_or(5);
                    i += 1;
                }
            }
            "--frames" | "-f" => {
                if i + 1 < args.len() {
                    tick_count = args[i + 1].parse().unwrap_or(300);
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("ECS AI Showcase");
                println!("Usage: ecs_ai_showcase [OPTIONS]");
                println!("  -e, --entities <N>  Number of enemies to spawn (default: 5)");
                println!("  -f, --frames   <N>  Number of ticks to run (default: 300)");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    (enemy_count, tick_count)
}

/// Deterministic state checksum for scheduler correctness diffing.
///
/// Hashes every entity's Position / Velocity / Health / AIAgent-state plus the
/// `GameStats` resource. The GameStats inclusion (per the task caveat) surfaces
/// any ordering sensitivity in `combat_system`'s damage-event processing: if
/// the sequential and parallel paths differ on event ordering, GameStats hash
/// will diverge while per-entity hashes (which are entity-ID-keyed) would not.
fn print_state_checksum(world: &World, frame: u64) {
    let mut pos_bits: u64 = 0;
    let mut vel_bits: u64 = 0;
    let mut health_bits: u64 = 0;
    let mut ai_bits: u64 = 0;

    let entities: Vec<Entity> = world.entities_with::<Position>();
    for e in entities {
        let eid = e.id() as u64;
        if let Some(p) = world.get::<Position>(e) {
            pos_bits = pos_bits.wrapping_add((p.pos.x.to_bits() as u64).wrapping_mul(eid.wrapping_add(1)));
            pos_bits = pos_bits.wrapping_add((p.pos.y.to_bits() as u64).wrapping_mul(eid.wrapping_add(2)));
            pos_bits = pos_bits.wrapping_add((p.pos.z.to_bits() as u64).wrapping_mul(eid.wrapping_add(3)));
        }
        if let Some(v) = world.get::<Velocity>(e) {
            vel_bits = vel_bits.wrapping_add((v.vel.x.to_bits() as u64).wrapping_mul(eid.wrapping_add(4)));
            vel_bits = vel_bits.wrapping_add((v.vel.y.to_bits() as u64).wrapping_mul(eid.wrapping_add(5)));
            vel_bits = vel_bits.wrapping_add((v.vel.z.to_bits() as u64).wrapping_mul(eid.wrapping_add(6)));
        }
        if let Some(h) = world.get::<Health>(e) {
            health_bits = health_bits.wrapping_add((h.current as u64).wrapping_mul(eid.wrapping_add(7)));
            health_bits = health_bits.wrapping_add((h.max as u64).wrapping_mul(eid.wrapping_add(8)));
        }
        if let Some(a) = world.get::<AIAgent>(e) {
            let disc: u64 = match a.state {
                AIState::Idle => 0,
                AIState::Patrolling => 1,
                AIState::Chasing => 2,
                AIState::Attacking => 3,
                AIState::Fleeing => 4,
            };
            ai_bits = ai_bits.wrapping_add(disc.wrapping_mul(eid.wrapping_add(9)));
            ai_bits = ai_bits.wrapping_add((a.target.map(|t| t.id() as u64).unwrap_or(u64::MAX))
                .wrapping_mul(eid.wrapping_add(10)));
        }
    }
    let mut stats_bits: u64 = 0;
    if let Some(s) = world.get_resource::<GameStats>() {
        stats_bits = (s.enemies_defeated as u64).wrapping_mul(97);
        stats_bits = stats_bits.wrapping_add((s.player_deaths as u64).wrapping_mul(193));
        stats_bits = stats_bits.wrapping_add((s.total_damage_dealt as u64).wrapping_mul(389));
    }

    println!(
        "[state-checksum] frame {}: pos={:016x} vel={:016x} health={:016x} ai={:016x} stats={:016x}",
        frame, pos_bits, vel_bits, health_bits, ai_bits, stats_bits
    );
}

fn main() -> Result<()> {
    let (enemy_count, tick_count) = parse_args();

    let mut app = App::new();

    // Register systems in AI-native order. The ECS schedule is deterministic
    // single-threaded; see docs/audits/parallel_schedule_removal_2026-04-18.md
    // for the rationale behind the single-threaded-ECS choice.
    app.add_system("perception", ai_perception_system);
    app.add_system("ai_planning", ai_planning_system);
    app.add_system("simulation", ai_behavior_system);
    app.add_system("simulation", movement_system);
    app.add_system("simulation", combat_system);
    app.add_system("post_simulation", stats_display_system);

    setup_world(&mut app, enemy_count);

    use astraweave_profiling::FrameAllocStats;
    use std::time::Instant;

    // Run simulation
    println!("🚀 Starting simulation ({} ticks)...\n", tick_count);
    let start = Instant::now();

    for _ in 0..tick_count {
        let alloc_stats = FrameAllocStats::begin_frame();

        // Update game time
        if let Some(time) = app.world.get_resource_mut::<GameTime>() {
            time.tick += 1;
        }

        // Run all systems via the sequential schedule.
        app.schedule.run(&mut app.world);

        // Update events
        if let Some(events) = app.world.get_resource_mut::<Events>() {
            events.update();
        }

        let tick = app.world.get_resource::<GameTime>().map(|t| t.tick).unwrap_or(0);
        let alloc_delta = alloc_stats.end_frame();

        // State checksum every 100 ticks for sequential↔parallel diffing.
        if tick % 100 == 0 && tick > 0 {
            print_state_checksum(&app.world, tick);
            #[cfg(feature = "alloc-counter")]
            println!(
                "[alloc-measure] frame {}: allocs={} bytes={} reallocs={} net={}",
                tick,
                alloc_delta.allocs,
                alloc_delta.bytes_allocated,
                alloc_delta.reallocs,
                alloc_delta.net_allocs
            );
            #[cfg(not(feature = "alloc-counter"))]
            let _ = alloc_delta;
        }
    }

    let elapsed = start.elapsed();
    let avg_fps = tick_count as f64 / elapsed.as_secs_f64();
    let avg_frame_ms = elapsed.as_millis() as f64 / tick_count as f64;

    println!("\n✅ Simulation complete!");
    println!("Configuration: {} enemies, {} ticks", enemy_count, tick_count);
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    println!("Average FPS: {:.2}", avg_fps);
    println!("Average frame time: {:.3}ms", avg_frame_ms);

    Ok(())
}
