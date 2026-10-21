# Configuration

This reference documents all configuration options for AstraWeave projects.

## Project Configuration

### Cargo.toml

Standard Rust project configuration with AstraWeave dependencies:

```toml
[package]
name = "my_game"
version = "0.1.0"
edition = "2021"

[dependencies]
astraweave-core = "0.1"
astraweave-ecs = "0.1"
astraweave-ai = "0.1"
astraweave-physics = "0.1"
astraweave-render = "0.1"
astraweave-audio = "0.1"
astraweave-input = "0.1"

[features]
default = ["graphics"]
graphics = ["astraweave-render/wgpu"]
headless = []
profiling = ["astraweave-core/tracy"]

[profile.release]
lto = true
codegen-units = 1
opt-level = 3
```

## Runtime Configuration

### Engine Configuration

Configure the engine at startup:

```rust
use astraweave_core::config::*;

let config = EngineConfig {
    tick_rate: TickRate::Fixed(60),
    
    window: WindowConfig {
        title: "My Game".into(),
        width: 1920,
        height: 1080,
        vsync: true,
        fullscreen: false,
    },
    
    graphics: GraphicsConfig {
        backend: GraphicsBackend::Auto,
        msaa_samples: 4,
        shadow_quality: ShadowQuality::High,
        max_lights: 128,
    },
    
    physics: PhysicsConfig {
        gravity: Vec3::new(0.0, -9.81, 0.0),
        substeps: 4,
        solver_iterations: 8,
    },
    
    audio: AudioConfig {
        sample_rate: 48000,
        channels: 2,
        buffer_size: 1024,
    },
    
    ai: AiConfig {
        tick_budget_ms: 8,
        max_concurrent_plans: 4,
        ollama_endpoint: "http://localhost:11434".into(),
        default_model: "hermes2-pro-mistral".into(),
    },
};

Engine::run(config, |world| {
    // Game setup
});
```

### Configuration File (TOML)

Load configuration from file:

```toml
# config.toml

[engine]
tick_rate = 60

[window]
title = "My Game"
width = 1920
height = 1080
vsync = true
fullscreen = false

[graphics]
backend = "vulkan"
msaa_samples = 4
shadow_quality = "high"
max_lights = 128
hdr = true
bloom = true

[physics]
gravity = [0.0, -9.81, 0.0]
substeps = 4
solver_iterations = 8
ccd_enabled = true

[audio]
sample_rate = 48000
channels = 2
buffer_size = 1024
master_volume = 1.0

[ai]
tick_budget_ms = 8
max_concurrent_plans = 4
ollama_endpoint = "http://localhost:11434"
default_model = "hermes2-pro-mistral"
fallback_enabled = true

[navigation]
cell_size = 0.3
cell_height = 0.2
agent_radius = 0.5
agent_height = 1.8
max_slope = 45.0

[networking]
tick_rate = 30
interpolation_delay_ms = 100
max_clients = 32
```

Load in code:

```rust
use astraweave_core::config::EngineConfig;

let config = EngineConfig::from_file("config.toml")?;
```

## System-Specific Configuration

### Graphics Configuration

```rust
let graphics = GraphicsConfig {
    backend: GraphicsBackend::Vulkan,
    
    msaa_samples: 4,
    anisotropic_filtering: 16,
    
    shadow_quality: ShadowQuality::High,
    shadow_cascade_count: 4,
    shadow_map_size: 2048,
    
    max_lights: 128,
    clustered_lighting: true,
    cluster_dimensions: [16, 9, 24],
    
    hdr: true,
    bloom: true,
    bloom_intensity: 0.5,
    
    ambient_occlusion: AmbientOcclusion::Ssao,
    
    vsync: VsyncMode::Enabled,
    frame_limit: None,
};
```

### Physics Configuration

```rust
let physics = PhysicsConfig {
    gravity: Vec3::new(0.0, -9.81, 0.0),
    
    substeps: 4,
    solver_iterations: 8,
    
    ccd_enabled: true,
    ccd_max_substeps: 4,
    
    broad_phase: BroadPhase::SweepAndPrune,
    
    contact_skin: 0.01,
    
    sleep_threshold_linear: 0.1,
    sleep_threshold_angular: 0.05,
    
    debug_render: false,
};
```

### AI Configuration

```rust
let ai = AiConfig {
    tick_budget_ms: 8,
    
    max_concurrent_plans: 4,
    plan_queue_size: 16,
    
    ollama_endpoint: "http://localhost:11434".into(),
    default_model: "hermes2-pro-mistral".into(),
    
    temperature: 0.7,
    max_tokens: 256,
    request_timeout_ms: 100,
    
    plan_cache_enabled: true,
    plan_cache_duration_ms: 2000,
    
    fallback_enabled: true,
    
    context_window_size: 4096,
    
    tool_validation_strict: true,
};
```

### Audio Configuration

```rust
let audio = AudioConfig {
    sample_rate: 48000,
    channels: 2,
    buffer_size: 1024,
    
    master_volume: 1.0,
    music_volume: 0.8,
    sfx_volume: 1.0,
    voice_volume: 1.0,
    
    max_simultaneous_sounds: 32,
    
    spatial_audio: true,
    hrtf_enabled: true,
    
    reverb_enabled: true,
    
    distance_model: DistanceModel::InverseSquare,
    rolloff_factor: 1.0,
    reference_distance: 1.0,
    max_distance: 100.0,
};
```

### Navigation Configuration

```rust
let navigation = NavigationConfig {
    cell_size: 0.3,
    cell_height: 0.2,
    
    agent_radius: 0.5,
    agent_height: 1.8,
    agent_max_climb: 0.4,
    agent_max_slope: 45.0,
    
    region_min_size: 8,
    region_merge_size: 20,
    
    edge_max_len: 12.0,
    edge_max_error: 1.3,
    
    verts_per_poly: 6,
    detail_sample_distance: 6.0,
    detail_sample_max_error: 1.0,
    
    tile_size: 64,
    
    pathfinder: PathfinderConfig {
        max_iterations: 2048,
        heuristic_scale: 1.0,
    },
    
    crowd: CrowdConfig {
        max_agents: 128,
        avoidance_quality: AvoidanceQuality::High,
    },
};
```

### Input Configuration

```rust
let input = InputConfig {
    deadzone: 0.15,
    
    mouse_sensitivity: 1.0,
    mouse_invert_y: false,
    mouse_smoothing: true,
    
    gamepad_enabled: true,
    gamepad_vibration: true,
    
    action_repeat_delay_ms: 500,
    action_repeat_rate_ms: 50,
};
```

## Environment Variables

Override configuration with environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `ASTRAWEAVE_LOG` | Log level (trace, debug, info, warn, error) | `info` |
| `ASTRAWEAVE_GRAPHICS_BACKEND` | Graphics backend (vulkan, dx12, metal) | Auto |
| `ASTRAWEAVE_OLLAMA_ENDPOINT` | Ollama API endpoint | `http://localhost:11434` |
| `ASTRAWEAVE_OLLAMA_MODEL` | Default LLM model | `hermes2-pro-mistral` |
| `ASTRAWEAVE_TICK_RATE` | Simulation tick rate | `60` |
| `ASTRAWEAVE_HEADLESS` | Run without graphics | `false` |

Example:

```bash
ASTRAWEAVE_LOG=debug ASTRAWEAVE_HEADLESS=true cargo run -p my_game
```

## Platform-Specific Configuration

### Windows

```toml
[target.'cfg(windows)'.graphics]
backend = "dx12"
```

### Linux

```toml
[target.'cfg(target_os = "linux")'.graphics]
backend = "vulkan"
```

### macOS

```toml
[target.'cfg(target_os = "macos")'.graphics]
backend = "metal"
```

## Debug Configuration

```rust
#[cfg(debug_assertions)]
let debug_config = DebugConfig {
    show_fps: true,
    show_entity_count: true,
    physics_debug_draw: true,
    navmesh_debug_draw: false,
    ai_debug_overlay: true,
    memory_stats: true,
    profiler_enabled: true,
};
```

## See Also

- [Building from Source](../dev/building.md)
- [Performance Optimization](../dev/performance.md)
- [Platform Support](./platforms.md)
