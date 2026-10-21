# Performance Optimization

This guide covers profiling, benchmarking, and optimization techniques for AstraWeave games and the engine itself.

## Performance Philosophy

AstraWeave targets consistent 60+ FPS gameplay with AI-native features. Key principles:

1. **Measure First**: Always profile before optimizing
2. **Budget Time**: Allocate frame time across systems
3. **Batch Operations**: Minimize per-entity overhead
4. **Cache Strategically**: Trade memory for speed where appropriate
5. **Offload to Threads**: Parallelize independent work

## Frame Budget

At 60 FPS, each frame has ~16.67ms. Recommended allocation:

| System | Budget | Notes |
|--------|--------|-------|
| Game Logic | 2-3ms | ECS systems, gameplay |
| AI | 2-4ms | Perception, planning, behaviors |
| Physics | 2-3ms | Collision, dynamics |
| Rendering | 6-8ms | Draw calls, GPU submission |
| Audio | 0.5-1ms | Mixing, spatial |
| Buffer | 1-2ms | Headroom for spikes |

## Profiling Tools

### Tracy Integration

AstraWeave integrates with [Tracy](https://github.com/wolfpld/tracy) for real-time profiling:

```toml
[dependencies]
astraweave-profiling = { version = "0.1", features = ["tracy"] }
```

```rust
use astraweave_profiling::*;

fn my_system(query: Query<&MyComponent>) {
    profile_scope!("my_system");
    
    for component in query.iter() {
        profile_scope!("process_entity");
        // Work
    }
}
```

Run with Tracy:

```bash
cargo run --release --features tracy
```

### Built-in Profiler

Enable the debug overlay:

```rust
use astraweave_profiling::prelude::*;

app.add_plugin(ProfilingPlugin::default());
```

Press F3 in-game to toggle the performance overlay showing:
- Frame time graph
- System timing breakdown
- Memory usage
- Draw call count

### CPU Profiling

For detailed CPU analysis:

```bash
# Linux (perf)
perf record -g cargo run --release
perf report

# Windows (ETW)
cargo build --release
# Use Windows Performance Analyzer

# macOS (Instruments)
cargo instruments --release -t time
```

### GPU Profiling

```bash
# NVIDIA Nsight
cargo run --release
# Attach Nsight Graphics

# RenderDoc
cargo run --release --features renderdoc
# Press F12 to capture frame
```

## Optimization Techniques

### ECS Optimization

#### Query Optimization

```rust
// Bad: Iterating all entities
fn slow_system(query: Query<&Transform>) {
    for transform in query.iter() {
        // Processes all entities with Transform
    }
}

// Good: Filter to relevant entities
fn fast_system(
    query: Query<&Transform, (With<Enemy>, Without<Dead>)>,
) {
    for transform in query.iter() {
        // Only active enemies
    }
}
```

#### Parallel Iteration

```rust
use rayon::prelude::*;

fn parallel_system(query: Query<&mut Transform>) {
    query.par_iter_mut().for_each(|mut transform| {
        // Thread-safe processing
        transform.translation.y += 0.1;
    });
}
```

#### Change Detection

```rust
fn efficient_update(
    query: Query<&MyComponent, Changed<MyComponent>>,
) {
    for component in query.iter() {
        // Only processes recently changed entities
    }
}
```

#### Archetypes

Group components that are commonly accessed together:

```rust
// Good: Components often queried together
#[derive(Bundle)]
struct EnemyBundle {
    transform: Transform,
    health: Health,
    ai: AiAgent,
    collider: Collider,
}

// Avoid: Rarely used components on common entities
struct RarelyUsedData { /* ... */ }
```

### Memory Optimization

#### Component Size

```rust
// Bad: Large component
#[derive(Component)]
struct LargeComponent {
    data: [f32; 1000],  // 4KB per entity
    name: String,
}

// Good: Split into data and reference
#[derive(Component)]
struct SmallComponent {
    data_handle: Handle<LargeData>,  // 8 bytes
    flags: u8,
}
```

#### Object Pools

```rust
use astraweave_ecs::pool::*;

#[derive(Resource)]
struct BulletPool {
    pool: EntityPool<BulletBundle>,
}

impl BulletPool {
    fn spawn(&mut self, commands: &mut Commands) -> Entity {
        self.pool.get_or_spawn(commands, || BulletBundle::default())
    }
    
    fn despawn(&mut self, entity: Entity) {
        self.pool.return_entity(entity);
    }
}
```

#### Arena Allocation

For temporary allocations:

```rust
use bumpalo::Bump;

fn batch_process(entities: &[Entity]) {
    let arena = Bump::new();
    
    let temp_data: &mut [Vec3] = arena.alloc_slice_fill_default(entities.len());
    
    // Work with temp_data
    // Arena automatically freed at scope end
}
```

### Rendering Optimization

#### Batching

```rust
// Enable instanced rendering for repeated meshes
#[derive(Component)]
struct InstancedMesh {
    mesh: Handle<Mesh>,
    material: Handle<Material>,
    instances: Vec<Transform>,
}
```

#### Level of Detail

```rust
#[derive(Component)]
struct LodGroup {
    distances: [f32; 3],
    meshes: [Handle<Mesh>; 3],
}

fn lod_system(
    camera: Query<&Transform, With<Camera>>,
    mut lod_query: Query<(&Transform, &LodGroup, &mut Handle<Mesh>)>,
) {
    let camera_pos = camera.single().translation;
    
    for (transform, lod, mut mesh) in lod_query.iter_mut() {
        let distance = transform.translation.distance(camera_pos);
        
        let lod_level = if distance < lod.distances[0] { 0 }
            else if distance < lod.distances[1] { 1 }
            else { 2 };
        
        *mesh = lod.meshes[lod_level].clone();
    }
}
```

#### Culling

```rust
#[derive(Component)]
struct Visibility {
    pub visible: bool,
    pub render_layers: u32,
}

fn frustum_culling_system(
    camera: Query<(&Camera, &Transform)>,
    mut renderables: Query<(&Transform, &Aabb, &mut Visibility)>,
) {
    let (camera, cam_transform) = camera.single();
    let frustum = camera.compute_frustum(cam_transform);
    
    for (transform, aabb, mut visibility) in renderables.iter_mut() {
        let world_aabb = aabb.transformed(transform);
        visibility.visible = frustum.intersects_aabb(&world_aabb);
    }
}
```

### AI Optimization

#### Tick Budgeting

```rust
#[derive(Resource)]
pub struct AiBudget {
    pub max_ms_per_frame: f32,
    pub agents_processed: usize,
}

fn budgeted_ai_system(
    mut budget: ResMut<AiBudget>,
    mut agents: Query<&mut AiAgent>,
    time: Res<Time>,
) {
    let start = std::time::Instant::now();
    budget.agents_processed = 0;
    
    for mut agent in agents.iter_mut() {
        if start.elapsed().as_secs_f32() * 1000.0 > budget.max_ms_per_frame {
            break;
        }
        
        agent.tick();
        budget.agents_processed += 1;
    }
}
```

#### LOD for AI

```rust
#[derive(Component)]
pub struct AiLod {
    pub distance_from_player: f32,
    pub update_frequency: u32,
    pub frames_since_update: u32,
}

fn ai_lod_system(
    player: Query<&Transform, With<Player>>,
    mut ai_query: Query<(&Transform, &mut AiLod, &mut AiAgent)>,
) {
    let player_pos = player.single().translation;
    
    for (transform, mut lod, mut agent) in ai_query.iter_mut() {
        lod.distance_from_player = transform.translation.distance(player_pos);
        
        lod.update_frequency = match lod.distance_from_player {
            d if d < 20.0 => 1,   // Every frame
            d if d < 50.0 => 2,   // Every 2 frames
            d if d < 100.0 => 5,  // Every 5 frames
            _ => 10,              // Every 10 frames
        };
        
        lod.frames_since_update += 1;
        if lod.frames_since_update >= lod.update_frequency {
            lod.frames_since_update = 0;
            agent.tick();
        }
    }
}
```

#### LLM Caching

```rust
#[derive(Resource)]
pub struct LlmCache {
    cache: LruCache<u64, String>,
    hit_count: u64,
    miss_count: u64,
}

impl LlmCache {
    pub fn get_or_generate<F>(
        &mut self,
        prompt: &str,
        generator: F,
    ) -> &str
    where
        F: FnOnce() -> String,
    {
        let hash = hash_prompt(prompt);
        
        if !self.cache.contains(&hash) {
            self.miss_count += 1;
            let response = generator();
            self.cache.put(hash, response);
        } else {
            self.hit_count += 1;
        }
        
        self.cache.get(&hash).unwrap()
    }
}
```

### Physics Optimization

#### Broad Phase

```rust
#[derive(Resource)]
pub struct PhysicsConfig {
    pub broad_phase: BroadPhaseType,
    pub substeps: u32,
    pub velocity_iterations: u32,
}

pub enum BroadPhaseType {
    BruteForce,     // < 100 entities
    SpatialHash,    // 100-1000 entities
    BvhTree,        // > 1000 entities
}
```

#### Sleeping

```rust
#[derive(Component)]
pub struct RigidBody {
    pub sleeping: bool,
    pub sleep_threshold: f32,
    pub sleep_timer: f32,
}

fn sleep_system(mut bodies: Query<(&Velocity, &mut RigidBody)>) {
    for (velocity, mut body) in bodies.iter_mut() {
        if velocity.linear.length_squared() < body.sleep_threshold {
            body.sleep_timer += delta;
            if body.sleep_timer > 0.5 {
                body.sleeping = true;
            }
        } else {
            body.sleep_timer = 0.0;
            body.sleeping = false;
        }
    }
}
```

## Benchmarking

### Criterion Benchmarks

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_ecs_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_iteration");
    
    for entity_count in [1000, 10000, 100000] {
        let world = create_world_with_entities(entity_count);
        
        group.bench_with_input(
            BenchmarkId::new("query", entity_count),
            &world,
            |b, world| {
                b.iter(|| {
                    let mut count = 0;
                    for _ in world.query::<&Transform>().iter() {
                        count += 1;
                    }
                    count
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_ecs_iteration);
criterion_main!(benches);
```

### Performance Regression Testing

```yaml
# .github/workflows/bench.yml
name: Benchmarks
on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run benchmarks
        run: cargo bench --all -- --save-baseline new
      - name: Compare with main
        run: |
          git fetch origin main
          cargo bench --all -- --baseline main --save-baseline new
```

## Common Performance Issues

### Issue: Frame Spikes

**Symptoms**: Occasional stutters, inconsistent frame times

**Causes**:
- GC in scripting
- Asset loading on main thread
- Large allocations

**Solutions**:
```rust
// Pre-warm asset loading
fn warmup_system(asset_server: Res<AssetServer>) {
    asset_server.load::<Mesh>("meshes/common.gltf");
    asset_server.load::<Texture>("textures/atlas.png");
}

// Use streaming for large assets
let handle = asset_server.load_async::<LargeAsset>("path").await;
```

### Issue: High CPU Usage

**Symptoms**: High CPU, low GPU utilization

**Causes**:
- Inefficient queries
- Too many systems
- Excessive allocations

**Solutions**:
```rust
// Combine related systems
fn combined_system(
    mut query: Query<(&mut Transform, &Velocity, &mut Health)>,
) {
    for (mut transform, velocity, mut health) in query.iter_mut() {
        transform.translation += velocity.0;
        health.regen();
    }
}
```

### Issue: Memory Growth

**Symptoms**: Increasing memory over time

**Causes**:
- Entity leaks
- Cache growth
- Asset retention

**Solutions**:
```rust
// Periodic cleanup
fn cleanup_system(
    mut commands: Commands,
    dead_entities: Query<Entity, With<Dead>>,
    time: Res<Time>,
    mut cleanup_timer: ResMut<CleanupTimer>,
) {
    cleanup_timer.tick(time.delta());
    
    if cleanup_timer.just_finished() {
        for entity in dead_entities.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}
```

## Best Practices

```admonish tip title="Performance Tips"
1. **Profile in Release**: Debug builds are 10-50x slower
2. **Measure Realistically**: Test with actual content, not empty scenes
3. **Test on Target Hardware**: Don't only test on dev machines
4. **Budget Early**: Set performance targets before development
5. **Automate Testing**: Catch regressions in CI
```

```admonish warning title="Anti-Patterns"
- **Premature Optimization**: Don't optimize without profiling data
- **Micro-benchmarks**: Real-world performance may differ
- **Ignoring Memory**: CPU speed means nothing if you're thrashing cache
- **Single-threaded Thinking**: Utilize all cores
```

## Related Documentation

- [Configuration](../reference/configuration.md) - Performance-related settings
- [Building](building.md) - Release build optimization
- [Testing](testing.md) - Performance test strategies
- [Best Practices](../resources/best-practices.md) - General best practices
