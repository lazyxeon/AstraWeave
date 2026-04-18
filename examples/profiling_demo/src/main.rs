/*!
# AstraWeave Profiling Demo

Demonstrates Tracy profiling integration with AstraWeave engine.

This demo creates a realistic game scenario with 1,000 entities and instruments
all major systems (ECS, AI, Physics, Rendering) to capture performance data.

## Usage

1. **Download Tracy**: https://github.com/wolfpld/tracy/releases
2. **Start Tracy server**: Run `Tracy.exe` (Windows) or `tracy-profiler` (Linux/Mac)
3. **Build with profiling**:
   ```
   cargo build -p profiling_demo --release --features profiling
   ```
4. **Run demo**:
   ```
   cargo run -p profiling_demo --release --features profiling
   ```
5. **Capture data**: Tracy will automatically connect and show real-time profiling data

## What to Look For

- **Frame Time**: Should be < 16.67ms for 60 FPS
- **System Breakdown**:
  - ECS tick: Archetype iteration, component access
  - AI planning: GOAP/BT execution
  - Physics: Collision detection, rigid body simulation
  - Rendering: Mesh submission, draw calls
- **Hot Spots**: Functions consuming >5% frame time
- **Memory**: Allocation patterns, heap churn

## Key Metrics

The demo tracks:
- **Entity Count**: 1,000 entities (500 AI agents, 500 physics objects)
- **Component Churn**: Add/remove operations
- **AI Planning**: GOAP cache hit rate
- **Physics**: Collision pairs, contact generation
- **Render**: Draw calls, vertex count

Press ESC to exit.
*/

use anyhow::Result;
use astraweave_ecs::{App, Entity, Query2, Query2Mut, SystemStage, World};
#[cfg(feature = "parallel-schedule")]
use astraweave_ecs::parallel::{ParallelSchedule, SystemDescriptor};
use astraweave_physics::{SpatialHash, AABB};
use astraweave_profiling::{alloc_plot, frame_mark, message, plot, span, FrameAllocStats};
use glam::Vec3;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// Global allocator selection. Rust allows exactly one `#[global_allocator]` per
// binary, so the features are mutually exclusive at the install site:
//
//   - `alloc-counter` on → install `CountingAlloc`. With `fast-alloc` also on,
//     CountingAlloc internally delegates to MiMalloc (see astraweave-ecs).
//   - `alloc-counter` off, `fast-alloc` on → install MiMalloc directly via
//     astraweave-alloc's `setup_global_allocator!` macro.
//   - Neither on → the platform default allocator is used.
#[cfg(feature = "alloc-counter")]
#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

#[cfg(all(feature = "fast-alloc", not(feature = "alloc-counter")))]
astraweave_alloc::setup_global_allocator!();

// Global timing storage (thread-safe for system access)
lazy_static::lazy_static! {
    static ref SYSTEM_TIMINGS: Arc<Mutex<SystemTimings>> = Arc::new(Mutex::new(SystemTimings::default()));
}

// Command-line argument parsing
fn parse_args() -> (usize, usize) {
    let args: Vec<String> = std::env::args().collect();
    let mut entity_count = 1000; // Default: stress test
    let mut max_frames = 1000; // Default: 1000 frames (~16s @ 60 FPS)

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--entities" | "-e" => {
                if i + 1 < args.len() {
                    entity_count = args[i + 1].parse().unwrap_or(1000);
                    i += 1;
                }
            }
            "--frames" | "-f" => {
                if i + 1 < args.len() {
                    max_frames = args[i + 1].parse().unwrap_or(1000);
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("AstraWeave Profiling Demo - Tracy Performance Analysis");
                println!("\nUsage:");
                println!("  profiling_demo [OPTIONS]");
                println!("\nOptions:");
                println!("  --entities, -e <N>   Number of entities to spawn (default: 1000)");
                println!("  --frames, -f <N>     Number of frames to capture (default: 1000)");
                println!("  --help, -h           Show this help message");
                println!("\nExamples:");
                println!("  profiling_demo --entities 200   # Low load baseline");
                println!("  profiling_demo --entities 500   # Medium load (target capacity)");
                println!("  profiling_demo --entities 1000  # High load (stress test)");
                println!("  profiling_demo -e 500 -f 2000   # 500 entities, 2000 frames");
                println!("\nTracy Setup:");
                println!("  1. Download Tracy from: https://github.com/wolfpld/tracy/releases");
                println!("  2. Start Tracy server (Tracy.exe) BEFORE running this demo");
                println!("  3. Tracy will auto-connect on localhost:8086");
                println!("  4. Capture profiling data in real-time");
                println!("  5. Save trace: File > Save Trace > baseline_<entities>.tracy");
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}. Use --help for usage.", args[i]);
            }
        }
        i += 1;
    }

    (entity_count, max_frames)
}

/// Simple position component
#[derive(Debug, Clone, Copy)]
struct Position(Vec3);

/// Simple velocity component
#[derive(Debug, Clone, Copy)]
struct Velocity(Vec3);

/// AI agent marker
#[derive(Debug, Clone, Copy)]
struct AIAgent {
    state: AgentState,
}

#[derive(Debug, Clone, Copy)]
enum AgentState {
    Idle,
    Moving,
    Attacking,
}

/// Physics body marker
#[derive(Debug, Clone, Copy)]
struct RigidBody {
    #[allow(dead_code)]
    mass: f32,
}

/// Renderable marker
#[derive(Debug, Clone, Copy)]
struct Renderable {
    mesh_id: u32,
}

/// System performance timings (manual measurement, no Tracy overhead)
#[derive(Debug, Clone, Copy, Default)]
struct SystemTimings {
    ai_perception_us: u64,
    ai_planning_us: u64,
    movement_us: u64,
    physics_us: u64,
    collision_detection_us: u64,
    cleanup_us: u64,
    rendering_us: u64,
    #[allow(dead_code)]
    total_us: u64,
}

/// Game state
struct GameState {
    app: App,
    #[cfg(feature = "parallel-schedule")]
    parallel_schedule: ParallelSchedule,
    #[allow(dead_code)]
    entity_count: usize,
    frame_count: u64,
    start_time: Instant,
    #[allow(dead_code)]
    timings: SystemTimings,
}

/// Build a `ParallelSchedule` with the same stage order and the same systems
/// as `App::new() + add_system(...)`, but with explicit `.reads::<T>()` /
/// `.writes::<T>()` access annotations derived from static inspection of each
/// system.
///
/// Annotation derivation (see audit report §3.1):
/// - `ai_perception_system`: `Query2::<Position, AIAgent>` read-only iteration
///   → reads Position, reads AIAgent.
/// - `ai_planning_system`: reads Position via Query2, then `get_mut::<AIAgent>`
///   to rotate state → reads Position, writes AIAgent.
/// - `movement_system`: `Query2Mut::<Position, Velocity>` gives `&mut Position`
///   and `&Velocity` → writes Position, reads Velocity.
/// - `physics_system`: `Query2::<Position, RigidBody>` then spatial-hash-only
///   work; no world mutation → reads Position, reads RigidBody.
/// - `cleanup_system`: no world access. Marked `.exclusive()` as the safe
///   default; it runs alone but this is free since it is a no-op and every
///   stage in `profiling_demo` has only one system anyway.
/// - `rendering_system`: `Query2::<Renderable, Position>` read-only
///   → reads Renderable, reads Position.
#[cfg(feature = "parallel-schedule")]
fn build_parallel_schedule() -> ParallelSchedule {
    let mut s = ParallelSchedule::new();
    s.add_stage(SystemStage::PRE_SIMULATION);
    s.add_stage(SystemStage::AI_PLANNING);
    s.add_stage(SystemStage::SIMULATION);
    s.add_stage(SystemStage::PHYSICS);
    s.add_stage(SystemStage::POST_SIMULATION);
    s.add_stage(SystemStage::PRESENTATION);

    s.add_system(
        SystemStage::PRE_SIMULATION,
        SystemDescriptor::new(ai_perception_system)
            .reads::<Position>()
            .reads::<AIAgent>()
            .named("ai_perception_system"),
    );
    s.add_system(
        SystemStage::AI_PLANNING,
        SystemDescriptor::new(ai_planning_system)
            .reads::<Position>()
            .writes::<AIAgent>()
            .named("ai_planning_system"),
    );
    s.add_system(
        SystemStage::SIMULATION,
        SystemDescriptor::new(movement_system)
            .writes::<Position>()
            .reads::<Velocity>()
            .named("movement_system"),
    );
    s.add_system(
        SystemStage::PHYSICS,
        SystemDescriptor::new(physics_system)
            .reads::<Position>()
            .reads::<RigidBody>()
            .named("physics_system"),
    );
    s.add_system(
        SystemStage::POST_SIMULATION,
        SystemDescriptor::new(cleanup_system)
            .exclusive()
            .named("cleanup_system"),
    );
    s.add_system(
        SystemStage::PRESENTATION,
        SystemDescriptor::new(rendering_system)
            .reads::<Renderable>()
            .reads::<Position>()
            .named("rendering_system"),
    );
    s
}

impl GameState {
    fn new(entity_count: usize) -> Result<Self> {
        span!("GameState::new");

        let mut app = App::new();

        // Register systems
        app.add_system(SystemStage::PRE_SIMULATION, ai_perception_system);
        app.add_system(SystemStage::AI_PLANNING, ai_planning_system);
        app.add_system(SystemStage::SIMULATION, movement_system);
        app.add_system(SystemStage::PHYSICS, physics_system);
        app.add_system(SystemStage::POST_SIMULATION, cleanup_system);
        app.add_system(SystemStage::PRESENTATION, rendering_system);

        // Spawn entities
        message!("Spawning {} entities", entity_count);
        {
            span!("entity_spawn");

            for i in 0..entity_count {
                let pos = Position(Vec3::new(
                    (i % 32) as f32 * 2.0,
                    ((i / 32) % 32) as f32 * 2.0,
                    (i / 1024) as f32 * 2.0,
                ));
                let vel = Velocity(Vec3::new(
                    (i as f32 * 0.1).sin() * 0.1,
                    (i as f32 * 0.1).cos() * 0.1,
                    0.0,
                ));

                let entity = app.world.spawn();
                app.world.insert(entity, pos);
                app.world.insert(entity, vel);

                if i % 2 == 0 {
                    // AI agent
                    app.world.insert(
                        entity,
                        AIAgent {
                            state: AgentState::Idle,
                        },
                    );
                    app.world.insert(entity, Renderable { mesh_id: 1 });
                } else {
                    // Physics object
                    app.world.insert(entity, RigidBody { mass: 1.0 });
                    app.world.insert(entity, Renderable { mesh_id: 2 });
                }
            }
        }

        message!("Entities spawned: {}", entity_count);

        Ok(Self {
            app,
            #[cfg(feature = "parallel-schedule")]
            parallel_schedule: build_parallel_schedule(),
            entity_count,
            frame_count: 0,
            start_time: Instant::now(),
            timings: SystemTimings::default(),
        })
    }

    fn tick(&mut self) -> Result<()> {
        span!("GameState::tick");

        // Begin per-frame allocation measurement window. With `alloc-counter` off
        // this is a cheap snapshot of zeroed counters; the `end_frame` delta will
        // also be zero and `alloc_plot!` emits nothing when `profiling` is off.
        let alloc_stats = FrameAllocStats::begin_frame();

        let frame_start = Instant::now();

        // Update frame metrics
        self.frame_count += 1;
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let fps = self.frame_count as f64 / elapsed;

        plot!("FPS", fps);
        plot!("EntityCount", self.entity_count as f64);
        plot!("FrameNumber", self.frame_count as f64);

        // Run ECS systems. Under `parallel-schedule`, drive the rayon-scope
        // schedule with per-system access annotations; otherwise use the
        // sequential `Schedule` unchanged.
        {
            span!("schedule_run");
            #[cfg(feature = "parallel-schedule")]
            {
                self.parallel_schedule.run(&mut self.app.world);
            }
            #[cfg(not(feature = "parallel-schedule"))]
            {
                self.app.schedule.run(&mut self.app.world);
            }
        }

        // Deterministic world-state checksum for scheduler correctness checks.
        // Emitted every 100 frames alongside the timing breakdown. Printed to
        // stdout so both schedulers' outputs can be diffed at matching frame
        // numbers. Uses bit-casts to include exact f32 bit patterns; wrapping
        // add tolerates any entity count.
        if self.frame_count % 100 == 0 {
            let mut pos_bits: u64 = 0;
            let mut vel_bits: u64 = 0;
            let pq = Query2::<Position, Velocity>::new(&self.app.world);
            for (entity, pos, vel) in pq {
                let eid = entity.id() as u64;
                pos_bits = pos_bits.wrapping_add(
                    (pos.0.x.to_bits() as u64).wrapping_mul(eid.wrapping_add(1)),
                );
                pos_bits = pos_bits.wrapping_add(
                    (pos.0.y.to_bits() as u64).wrapping_mul(eid.wrapping_add(2)),
                );
                pos_bits = pos_bits.wrapping_add(
                    (pos.0.z.to_bits() as u64).wrapping_mul(eid.wrapping_add(3)),
                );
                vel_bits = vel_bits.wrapping_add(
                    (vel.0.x.to_bits() as u64).wrapping_mul(eid.wrapping_add(4)),
                );
                vel_bits = vel_bits.wrapping_add(
                    (vel.0.y.to_bits() as u64).wrapping_mul(eid.wrapping_add(5)),
                );
            }
            let mut agent_bits: u64 = 0;
            let aq = Query2::<Position, AIAgent>::new(&self.app.world);
            for (entity, _pos, agent) in aq {
                let disc = match agent.state {
                    AgentState::Idle => 0_u64,
                    AgentState::Moving => 1_u64,
                    AgentState::Attacking => 2_u64,
                };
                agent_bits = agent_bits.wrapping_add(disc.wrapping_mul(entity.id() as u64 + 1));
            }
            println!(
                "[state-checksum] frame {}: pos={:016x} vel={:016x} agent={:016x}",
                self.frame_count, pos_bits, vel_bits, agent_bits
            );
        }

        // Calculate total frame time
        let total_us = frame_start.elapsed().as_micros() as u64;

        // Print timing breakdown every 100 frames
        if self.frame_count % 100 == 0 {
            let timings = SYSTEM_TIMINGS.lock().unwrap();
            println!("\n=== Frame {} System Timings ===", self.frame_count);
            println!(
                "AI Perception:       {:>6} µs ({:>5.2}%)",
                timings.ai_perception_us,
                (timings.ai_perception_us as f64 / total_us as f64) * 100.0
            );
            println!(
                "AI Planning:         {:>6} µs ({:>5.2}%)",
                timings.ai_planning_us,
                (timings.ai_planning_us as f64 / total_us as f64) * 100.0
            );
            println!(
                "Movement:            {:>6} µs ({:>5.2}%)",
                timings.movement_us,
                (timings.movement_us as f64 / total_us as f64) * 100.0
            );
            println!(
                "Physics (total):     {:>6} µs ({:>5.2}%)",
                timings.physics_us,
                (timings.physics_us as f64 / total_us as f64) * 100.0
            );
            println!(
                "  - Collision Det:   {:>6} µs ({:>5.2}%)",
                timings.collision_detection_us,
                (timings.collision_detection_us as f64 / total_us as f64) * 100.0
            );
            println!(
                "Cleanup:             {:>6} µs ({:>5.2}%)",
                timings.cleanup_us,
                (timings.cleanup_us as f64 / total_us as f64) * 100.0
            );
            println!(
                "Rendering:           {:>6} µs ({:>5.2}%)",
                timings.rendering_us,
                (timings.rendering_us as f64 / total_us as f64) * 100.0
            );
            println!("---");
            println!(
                "Total Frame Time:    {:>6} µs ({:.3} ms)",
                total_us,
                total_us as f64 / 1000.0
            );
            println!("Target (Week 8):     {:>6} µs (2.700 ms)", 2700);
            println!("FPS:                 {:.2}", fps);
        }

        // Close the allocation measurement window and emit `frame.allocs` /
        // `frame.bytes` to Tracy. With `alloc-counter` off the delta is zero;
        // with `profiling` off the emit is a no-op.
        let alloc_delta = alloc_stats.end_frame();
        alloc_plot!(
            "frame",
            alloc_delta.allocs,
            alloc_delta.bytes_allocated
        );

        // When `alloc-counter` is enabled but Tracy is not running, print the
        // delta every 100 frames so terminal output still yields data.
        #[cfg(feature = "alloc-counter")]
        {
            if self.frame_count % 100 == 0 {
                println!(
                    "[alloc-measure] frame {}: allocs={} bytes={} reallocs={} net={}",
                    self.frame_count,
                    alloc_delta.allocs,
                    alloc_delta.bytes_allocated,
                    alloc_delta.reallocs,
                    alloc_delta.net_allocs
                );
            }
        }

        frame_mark!();

        Ok(())
    }
}

// System implementations (instrumented with profiling)

fn ai_perception_system(world: &mut World) {
    let timer_start = Instant::now();
    span!("ai_perception");

    let mut count = 0;
    // Simulate AI perception queries
    let query = Query2::<Position, AIAgent>::new(world);
    for (_entity, _pos, _agent) in query {
        count += 1;
        // Simulate perception work
        let _ = (count as f32).sin();
    }

    plot!("AI.Agents", count as f64);

    // Record timing
    if let Ok(mut timings) = SYSTEM_TIMINGS.lock() {
        timings.ai_perception_us = timer_start.elapsed().as_micros() as u64;
    }
}

fn ai_planning_system(world: &mut World) {
    let timer_start = Instant::now();
    span!("ai_planning");

    let mut planning_count = 0;
    let mut cache_hits = 0;

    // Simulate AI planning with cache - need to collect entities first to avoid borrow conflicts
    let entities: Vec<Entity> = {
        let query = Query2::<Position, AIAgent>::new(world);
        query.map(|(entity, _, _)| entity).collect()
    };

    for entity in entities {
        if let Some(_agent) = world.get_mut::<AIAgent>(entity) {
            planning_count += 1;

            // Simulate GOAP planning with cache hits
            if planning_count % 10 == 0 {
                // Cache miss - expensive planning
                span!("goap_planning");
                for _ in 0..100 {
                    let _ = (planning_count as f32).sqrt();
                }
            } else {
                // Cache hit - cheap lookup
                cache_hits += 1;
            }

            // State transitions
            let agent = world.get_mut::<AIAgent>(entity).unwrap();
            agent.state = match agent.state {
                AgentState::Idle => AgentState::Moving,
                AgentState::Moving => AgentState::Attacking,
                AgentState::Attacking => AgentState::Idle,
            };
        }
    }

    let _cache_hit_rate = if planning_count > 0 {
        (cache_hits as f64 / planning_count as f64) * 100.0
    } else {
        0.0
    };

    plot!("AI.PlanningOperations", planning_count as f64);
    plot!("AI.CacheHitRate", _cache_hit_rate);

    // Record timing
    if let Ok(mut timings) = SYSTEM_TIMINGS.lock() {
        timings.ai_planning_us = timer_start.elapsed().as_micros() as u64;
    }
}

fn movement_system(world: &mut World) {
    let timer_start = Instant::now();
    span!("movement");

    // OPTIMIZED: Use Query2Mut to update positions directly without writeback
    // This eliminates the 770µs O(log n) writeback bottleneck
    let query = Query2Mut::<Position, Velocity>::new(world);

    // Direct mutation - no collect, no writeback, O(1) per entity
    // Note: We use a manual counter because enumerate() would require additional iterator machinery
    // and we need the final count for profiling metrics
    #[allow(clippy::explicit_counter_loop)]
    // `moved_count` is unused when the `profiling` feature is off because `plot!`
    // expands to a no-op. It is intentionally bound without an underscore prefix
    // so the non-underscored reference below compiles when `profiling` is on.
    #[allow(unused_variables)]
    let moved_count = {
        let mut count = 0_usize;
        for (_entity, pos, vel) in query {
            count += 1;

            // Update position with velocity
            pos.0.x += vel.0.x * 1.0;
            pos.0.y += vel.0.y * 1.0;
            pos.0.z += vel.0.z * 1.0;

            // Apply bounds wrapping inline
            if pos.0.x.abs() > 64.0 {
                pos.0.x = -pos.0.x.signum() * 64.0;
            }
            if pos.0.y.abs() > 64.0 {
                pos.0.y = -pos.0.y.signum() * 64.0;
            }
        }
        count
    };

    plot!("Movement.Updates", moved_count as f64);

    // Record timing
    if let Ok(mut timings) = SYSTEM_TIMINGS.lock() {
        timings.movement_us = timer_start.elapsed().as_micros() as u64;
    }
}

fn physics_system(world: &mut World) {
    let timer_start = Instant::now();
    span!("physics");

    let mut _collision_checks = 0;
    let mut _collisions = 0;
    let collision_start = Instant::now();

    // Spatial hash collision detection (optimized)
    {
        span!("collision_detection");

        // Collect entities with positions
        let entities_data: Vec<(Entity, Vec3)> = {
            let query = Query2::<Position, RigidBody>::new(world);
            query.map(|(entity, pos, _)| (entity, pos.0)).collect()
        };

        if !entities_data.is_empty() {
            // Build entity index map for O(1) lookups (fix for O(n²) regression)
            let entity_map: std::collections::HashMap<u64, (usize, Vec3)> = entities_data
                .iter()
                .enumerate()
                .map(|(i, (e, pos))| (e.id() as u64, (i, *pos)))
                .collect();

            // Build spatial hash grid (use entity ID as u32)
            let mut grid = SpatialHash::new(2.0); // Cell size = 2× collision radius

            for (entity, pos) in &entities_data {
                let aabb = AABB::from_sphere(*pos, 0.5); // Collision radius = 0.5
                grid.insert(entity.id(), aabb);
            }

            // Query for collisions using spatial hash
            for (i, (_entity, pos)) in entities_data.iter().enumerate() {
                // Query radius must match collision distance (1.0), not object radius (0.5)!
                let query_aabb = AABB::from_sphere(*pos, 1.0); // collision_distance = 1.0
                let candidates = grid.query(query_aabb);

                for &candidate_id in &candidates {
                    // O(1) lookup instead of O(n) find!
                    if let Some(&(j, candidate_pos)) = entity_map.get(&(candidate_id as u64)) {
                        // Only check each pair once (i < j)
                        if i < j {
                            _collision_checks += 1;

                            let dist = pos.distance(candidate_pos);
                            if dist < 1.0 {
                                _collisions += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    let collision_elapsed = collision_start.elapsed().as_micros() as u64;

    plot!("Physics.CollisionChecks", _collision_checks as f64);
    plot!("Physics.Collisions", _collisions as f64);

    // Record timing
    if let Ok(mut timings) = SYSTEM_TIMINGS.lock() {
        timings.physics_us = timer_start.elapsed().as_micros() as u64;
        timings.collision_detection_us = collision_elapsed;
    }
}

fn cleanup_system(_world: &mut World) {
    let timer_start = Instant::now();
    span!("cleanup");
    // Simulate cleanup work

    // Record timing
    if let Ok(mut timings) = SYSTEM_TIMINGS.lock() {
        timings.cleanup_us = timer_start.elapsed().as_micros() as u64;
    }
}

fn rendering_system(world: &mut World) {
    let timer_start = Instant::now();
    span!("rendering");

    let mut _draw_calls = 0;
    let mut _vertex_count = 0;

    // Simulate rendering
    {
        span!("render_submit");

        let query = Query2::<Renderable, Position>::new(world);
        for (_, renderable, _pos) in query {
            _draw_calls += 1;
            // Simulate vertex count per mesh
            _vertex_count += if renderable.mesh_id == 1 { 36 } else { 64 };
        }
    }

    plot!("Render.DrawCalls", _draw_calls as f64);
    plot!("Render.VertexCount", _vertex_count as f64);

    // Record timing
    if let Ok(mut timings) = SYSTEM_TIMINGS.lock() {
        timings.rendering_us = timer_start.elapsed().as_micros() as u64;
    }
}

fn main() -> Result<()> {
    // Parse command-line arguments
    let (entity_count, max_frames) = parse_args();

    println!("=== AstraWeave Profiling Demo ===");
    println!(
        "Tracy profiling enabled: {}",
        astraweave_profiling::Profiler::is_enabled()
    );
    println!("Configuration:");
    println!("  Entities: {}", entity_count);
    println!(
        "  Frames: {} (~{:.1}s @ 60 FPS)",
        max_frames,
        max_frames as f64 / 60.0
    );
    println!("\nControls:");
    println!("  Run for {} frames then exit", max_frames);
    println!("\nStart Tracy server before running for best results.");
    println!("Tracy will auto-connect and capture profiling data.\n");

    message!(
        "=== Profiling Demo Start === Entities: {}, Frames: {}",
        entity_count,
        max_frames
    );

    // Create game state
    let mut game = GameState::new(entity_count)?;

    message!("Game initialized with {} entities", entity_count);

    // Run profiling loop
    let start = Instant::now();

    for frame in 0..max_frames {
        if frame % 100 == 0 {
            println!("Frame {}/{}", frame, max_frames);
            message!("Milestone: Frame {}", frame);
        }

        game.tick()?;
    }

    let elapsed = start.elapsed();
    let avg_fps = max_frames as f64 / elapsed.as_secs_f64();
    let avg_frame_ms = elapsed.as_millis() as f64 / max_frames as f64;

    println!("\n=== Profiling Complete ===");
    println!(
        "Configuration: {} entities, {} frames",
        entity_count, max_frames
    );
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    println!("Average FPS: {:.2}", avg_fps);
    println!("Average frame time: {:.3}ms", avg_frame_ms);
    println!("\nCheck Tracy for detailed profiling data!");
    println!(
        "Save trace: File > Save Trace > baseline_{}.tracy",
        entity_count
    );

    message!(
        "=== Profiling Demo Complete === Entities: {}, FPS: {:.2}, Frame: {:.3}ms",
        entity_count,
        avg_fps,
        avg_frame_ms
    );

    Ok(())
}
