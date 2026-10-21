# Performance Budgets

This document defines frame time allocations for AstraWeave games targeting 60 FPS.

## Target Specification

| Metric | Value | Notes |
|--------|-------|-------|
| **Target FPS** | 60 | Standard for action games |
| **Frame Budget** | 16.67 ms | 1000ms ÷ 60 |
| **Headroom Target** | 20% | 3.33 ms reserved for spikes |
| **Available Budget** | 13.34 ms | 80% of frame |

---

## Budget Allocation

### Recommended Distribution

| System | Budget (ms) | Percentage | Notes |
|--------|-------------|------------|-------|
| **Game Logic** | 2.0 | 12% | ECS, AI, gameplay |
| **Physics** | 2.5 | 15% | Simulation, collision |
| **Rendering** | 6.0 | 36% | Draw calls, GPU sync |
| **Audio** | 0.5 | 3% | Spatial audio, mixing |
| **Networking** | 1.0 | 6% | Sync, prediction |
| **UI** | 0.5 | 3% | HUD, menus |
| **I/O** | 0.5 | 3% | Streaming, saves |
| **Headroom** | 3.67 | 22% | Spike absorption |
| **Total** | 16.67 | 100% | — |

### Visual Breakdown

```
Frame Budget: 16.67ms
├── Game Logic ████░░░░░░░░░░░░░░░░ 2.0ms (12%)
├── Physics    █████░░░░░░░░░░░░░░░ 2.5ms (15%)
├── Rendering  ████████████░░░░░░░░ 6.0ms (36%)
├── Audio      █░░░░░░░░░░░░░░░░░░░ 0.5ms (3%)
├── Networking ██░░░░░░░░░░░░░░░░░░ 1.0ms (6%)
├── UI         █░░░░░░░░░░░░░░░░░░░ 0.5ms (3%)
├── I/O        █░░░░░░░░░░░░░░░░░░░ 0.5ms (3%)
└── Headroom   ███████░░░░░░░░░░░░░ 3.67ms (22%)
```

---

## Game Logic Budget (2.0 ms)

### Sub-Allocation

| Component | Budget (µs) | Notes |
|-----------|-------------|-------|
| ECS core (1k entities) | 85 | 0.51% of frame |
| AI updates (500 agents) | 471 | GOAP + arbiter |
| Gameplay systems | 500 | Combat, quests, dialogue |
| Script execution | 300 | Rhai scripts |
| Event processing | 200 | Input, triggers |
| **Total** | 1,556 | 22% headroom within budget |

### Scaling Guidelines

| Entity Count | ECS Budget | AI Agents | Feasibility |
|--------------|------------|-----------|-------------|
| 1,000 | 85 µs | 500 | ✅ Comfortable |
| 5,000 | 529 µs | 1,000 | ✅ Within budget |
| 10,000 | 1 ms | 2,000 | ⚠️ Near limit |
| 20,000+ | >2 ms | — | ❌ Consider 30 FPS |

---

## Physics Budget (2.5 ms)

### Sub-Allocation

| Component | Budget (µs) | Notes |
|-----------|-------------|-------|
| Rigid body simulation | 500 | 100 bodies |
| Character controllers | 200 | Player + NPCs |
| Collision detection | 800 | Spatial hash accelerated |
| Raycasts | 300 | AI, weapons, cameras |
| Triggers/sensors | 200 | Zone detection |
| **Total** | 2,000 | 20% headroom |

### Collision Budget

With spatial hashing (99.96% check reduction):

| Objects | Naive | With Hash | Savings |
|---------|-------|-----------|---------|
| 100 | 4,950 checks | 2 checks | 99.96% |
| 1,000 | 499,500 checks | ~20 checks | 99.996% |
| 10,000 | 49.9M checks | ~200 checks | 99.9996% |

---

## Rendering Budget (6.0 ms)

### Sub-Allocation

| Stage | Budget (ms) | Notes |
|-------|-------------|-------|
| Culling | 0.5 | Frustum, occlusion |
| Shadow passes | 1.0 | CSM, spot shadows |
| G-Buffer | 1.5 | Deferred geometry |
| Lighting | 1.0 | Clustered, IBL |
| Post-processing | 1.0 | Bloom, SSAO, TAA |
| UI overlay | 0.5 | HUD, debug |
| GPU sync | 0.5 | Fence waiting |
| **Total** | 6.0 | — |

### Draw Call Budget

| Category | Calls | Notes |
|----------|-------|-------|
| Opaque geometry | 500 | With instancing |
| Transparent | 100 | Sorted, no batching |
| Shadows | 200 | Per cascade |
| UI | 50 | Batched |
| **Total** | 850 | Target maximum |

---

## Audio Budget (0.5 ms)

### Sub-Allocation

| Component | Budget (µs) | Notes |
|-----------|-------------|-------|
| Voice mixing | 200 | 4-bus mixer |
| Spatial positioning | 150 | 3D audio |
| DSP effects | 100 | Reverb, filters |
| Stream decode | 50 | Music, ambient |
| **Total** | 500 | — |

---

## Monitoring Budget Compliance

### Runtime Checks

```rust
use std::time::Instant;

struct FrameBudget {
    logic_budget: Duration,
    physics_budget: Duration,
    render_budget: Duration,
}

impl FrameBudget {
    fn check(&self, phase: &str, elapsed: Duration, budget: Duration) {
        if elapsed > budget {
            warn!(
                "{} exceeded budget: {:?} > {:?} ({}%)",
                phase,
                elapsed,
                budget,
                (elapsed.as_micros() * 100 / budget.as_micros())
            );
        }
    }
}
```

### Tracy Zones

```rust
#[cfg(feature = "profiling")]
{
    astraweave_profiling::zone!("GameLogic");
    // ... game logic ...
}
```

---

## 30 FPS Budget (Alternative)

For complex simulations or lower-end hardware:

| Metric | Value |
|--------|-------|
| Frame Budget | 33.33 ms |
| Headroom | 6.67 ms (20%) |
| Available | 26.66 ms |

**When to use 30 FPS**:
- Open worlds with >20k entities
- Complex physics simulations
- Lower-end hardware targets
- Cinematic experiences

---

## Budget Violations

### Severity Levels

| Violation | Duration | Response |
|-----------|----------|----------|
| **Minor** | 1-2 frames | Log, continue |
| **Moderate** | 3-10 frames | Reduce quality |
| **Severe** | >10 frames | Emergency LOD |

### Automatic Quality Scaling

```rust
// Reduce quality on budget overrun
if frame_time > Duration::from_millis(18) {
    quality.reduce_shadow_resolution();
}
if frame_time > Duration::from_millis(20) {
    quality.reduce_draw_distance();
}
if frame_time > Duration::from_millis(25) {
    quality.disable_post_processing();
}
```

---

## See Also

- [Benchmarks](./benchmarks.md) - Actual measured performance
- [Methodology](./methodology.md) - How we measure
- [Optimization Guide](./optimization.md) - Performance techniques
