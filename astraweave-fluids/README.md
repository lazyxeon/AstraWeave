# astraweave-fluids

A GPU-accelerated, research-stage fluid simulation crate (PBD core solver) for the AstraWeave game engine. In-design: consumed only by examples/fluids_demo; no production game-loop crate depends on it yet.

## Features

### Core Simulation
- **Position-Based Dynamics (PBD)** solver with adaptive iterations
- **Temperature & Heat Transfer** with Boussinesq buoyancy
- **Multi-Phase Fluids** (water, oil, custom phases)
- **Surface Tension** (Akinci kernel)
- **Vorticity Confinement** for turbulent flow

### Rendering (Screen-Space Fluid Rendering)
- **Depth Smoothing** for smooth surfaces
- **Beer-Lambert Absorption** for volumetric lighting
- **Procedural Caustics** via Voronoi patterns — implemented in-crate (caustics.rs), not wired into any production renderer (zero consumers outside the crate)
- **Temporal Reprojection** for anti-aliasing
- **Whitewater & Foam** secondary particles — implemented in-crate (foam.rs), not wired into a production consumer

### Infrastructure
- **Dynamic Particle Management** (spawn/despawn at runtime)
- **Mesh-Based Emitters** (Point, Sphere, Box, Mesh shapes)
- **State Serialization** (save/load with bincode)
- **GPU Frustum Culling** for efficient rendering
- **LOD System** for distance-based optimization
- **Jump Flood Algorithm (JFA)** for SDF generation

### Production Optimization (NEW)
- **FluidOptimizationController** - Comprehensive auto-tuning system
- **GPU Vendor Detection** - Automatic NVIDIA/AMD/Intel presets
- **Real-Time Metrics** - Frame time tracking with quality adjustment
- **Adaptive Iterations** - Dynamic solver iterations based on error
- **Quality Tiers** - Ultra/High/Medium/Low/Potato presets
- **Budget-Aware Stepping** - Maintains target framerate automatically

## Quick Start

```rust
use astraweave_fluids::{FluidSystem, FluidRenderer, FluidLodManager};

// Create system with 100k particles
let mut fluid_system = FluidSystem::new(&device, 100_000);

// Configure simulation
fluid_system.smoothing_radius = 1.0;
fluid_system.viscosity = 10.0;
fluid_system.surface_tension = 0.02;

// Step simulation each frame
fluid_system.step(&device, &mut encoder, &queue, dt);

// Render
renderer.render(encoder, view, /* ... */);
```

## Optimization (Auto-Tuning)

Auto-tuning surface (`FluidOptimizationController`) exists in-crate; not yet consumed by a production game.

### Basic Usage

```rust
use astraweave_fluids::FluidOptimizationController;

// Create controller with GPU-specific presets
let mut controller = FluidOptimizationController::new();
controller.configure_for_gpu("NVIDIA GeForce RTX 4090");
controller.set_target_framerate(60.0);
controller.enable_lod([0.0, 5.0, 0.0]); // Fluid center position

// Game loop - automatic quality management
let result = controller.step_system(
    &mut fluid_system,
    &device,
    &mut encoder,
    &queue,
    dt,
    camera_position,
);

// Quality will auto-adjust to maintain 60 FPS
if result.suggests_quality_decrease() {
    // Controller handles this automatically if auto_tune is enabled
}
```

### RAII Frame Timing

```rust
// Automatic frame timing with RAII guard
{
    let guard = controller.begin_frame(camera_position);
    
    // Do simulation work
    fluid_system.step(&device, &mut encoder, &queue, dt);
    
    // guard.drop() automatically records frame time and updates LOD
}
```

### Budget-Aware Stepping

```rust
// Let the controller manage quality based on last frame time
let last_frame_ms = 12.5; // From previous frame
let result = controller.step_with_budget(
    &mut fluid_system,
    &device,
    &mut encoder,
    &queue,
    dt,
    last_frame_ms,
    camera_position,
);

// Check budget status
if controller.is_within_budget() {
    println!("Headroom: {:.1}%", controller.budget_headroom());
}
```

### Quality Tier Presets

| Tier | Particles | Iterations | Kernel | Target Use |
|------|-----------|------------|--------|------------|
| Ultra (0) | 100k+ | 8 | Full | High-end PCs |
| High (1) | 50k | 6 | Standard | Gaming PCs |
| Medium (2) | 25k | 4 | Simplified | Mid-range |
| Low (3) | 10k | 2 | Minimal | Low-end |
| Potato (4) | 5k | 1 | Basic | Integrated GPU |

### GPU Vendor Optimization

```rust
// Automatic preset selection based on GPU name
controller.configure_for_gpu("NVIDIA GeForce RTX 4090"); // Ultra
controller.configure_for_gpu("AMD Radeon RX 7900 XTX");  // High
controller.configure_for_gpu("Intel Arc A770");          // Medium
controller.configure_for_gpu("Intel UHD Graphics 770");  // Low
```

### Metrics & Debugging

```rust
let status = controller.status();
println!("Quality Tier: {}", status.quality_tier);
println!("Avg Frame Time: {:.2}ms", status.avg_frame_time_ms);
println!("Target: {:.2}ms", status.target_frame_time_ms);
println!("Within Budget: {}", status.within_budget);
println!("Frames Recorded: {}", status.frames_recorded);
println!("Auto-Tune: {}", status.auto_tune_enabled);
```

### Advanced Configuration

```rust
// Full control over optimization behavior
controller.set_target_framerate(144.0);        // For high-refresh monitors
controller.set_quality_tier(1);                // Force High quality
controller.set_auto_tune(false);               // Disable auto-adjustment
controller.set_budget_margin(0.15);            // 15% safety margin
controller.configure_adaptive_iterations(2, 8); // 2-8 iterations
controller.enable_shader_lod();                 // Use simplified shaders at distance
controller.reset_metrics();                     // Clear frame history
```

## Modules

| Module | Description |
|--------|-------------|
| `lib.rs` | Core `FluidSystem`, `FluidOptimizationController`, `Particle` |
| `renderer.rs` | `FluidRenderer` with SSFR pipeline |
| `emitter.rs` | `FluidEmitter`, `FluidDrain` types |
| `lod.rs` | `FluidLodManager`, `ParticleStreamingManager` |
| `profiling.rs` | `FluidProfiler`, `FluidTimingStats` |
| `serialization.rs` | `FluidSnapshot` for state persistence |
| `sdf.rs` | `SdfSystem` with Jump Flood Algorithm |
| `optimization.rs` | `OptimizationPreset`, `AdaptiveIterations`, metrics |

## Shaders

| Shader | Purpose |
|--------|---------|
| `fluid.wgsl` | Core PBD solver kernels |
| `fluid_optimized.wgsl` | LOD-aware optimized solver |
| `ssfr_depth.wgsl` | Particle depth rendering |
| `ssfr_smooth.wgsl` | Bilateral depth smoothing |
| `ssfr_shade.wgsl` | Final shading with SSS |
| `ssfr_temporal.wgsl` | Temporal reprojection |
| `cull.wgsl` | GPU frustum culling |
| `sdf_gen.wgsl` | SDF generation via JFA |
| `secondary.wgsl` | Whitewater billboard rendering |

## Performance Tips

1. **Use FluidOptimizationController** - Automatic quality management for stable FPS
2. **Enable LOD** - Distance-based optimization reduces GPU load
3. **Set Target Framerate** - Controller will auto-adjust quality to hit target
4. **Configure for GPU** - Use vendor-specific presets for optimal settings
5. **Monitor Budget Headroom** - Aim for 20-30% headroom for consistent performance
6. **Use Adaptive Iterations** - Let solver iterations adjust based on density error

## Benchmarks

```powershell
# Run optimization benchmarks
cargo bench -p astraweave-fluids

# View HTML report
start target/criterion/report/index.html
```

## Demo

```powershell
cargo run -p fluids_demo --release
```

**Controls:**
- **SPACE** - Switch scenarios
- **F1** - Toggle debug panel
- **F2** - Toggle optimization overlay
- **WASD** - Orbit camera
- **Q/E** - Zoom in/out
- **Left Click** - Spawn particles
- **ESC** - Exit

## API Reference

### FluidOptimizationController

```rust
impl FluidOptimizationController {
    // Construction
    pub fn new() -> Self;
    
    // GPU Configuration
    pub fn configure_for_gpu(&mut self, gpu_name: &str);
    
    // Quality Control
    pub fn set_quality_tier(&mut self, tier: u32);
    pub fn quality_tier(&self) -> u32;
    pub fn set_auto_tune(&mut self, enabled: bool);
    
    // Framerate Targeting
    pub fn set_target_framerate(&mut self, fps: f32);
    pub fn set_budget_margin(&mut self, margin: f32);
    
    // LOD System
    pub fn enable_lod(&mut self, fluid_center: [f32; 3]);
    pub fn disable_lod(&mut self);
    pub fn enable_shader_lod(&mut self);
    
    // Adaptive Iterations
    pub fn configure_adaptive_iterations(&mut self, min: u32, max: u32);
    
    // Frame Recording
    pub fn record_frame(&mut self, frame_time_ms: f32);
    pub fn reset_metrics(&mut self);
    
    // Status & Metrics
    pub fn status(&self) -> ControllerStatus;
    pub fn is_within_budget(&self) -> bool;
    pub fn budget_headroom(&self) -> f32;
    pub fn recommended_iterations(&self) -> u32;
    
    // GPU Integration
    pub fn step_system(
        &mut self,
        system: &mut FluidSystem,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dt: f32,
        camera_pos: [f32; 3],
    ) -> OptimizedStepResult;
    
    pub fn step_with_budget(
        &mut self,
        system: &mut FluidSystem,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dt: f32,
        last_frame_time_ms: f32,
        camera_pos: [f32; 3],
    ) -> OptimizedStepResult;
    
    // Manual Workflow
    pub fn prepare_step(&mut self, system: &mut FluidSystem, camera_pos: [f32; 3]) -> u32;
    pub fn record_step_result(&mut self, frame_time_ms: f32, camera_pos: [f32; 3]);
    
    // RAII Frame Guard
    pub fn begin_frame(&mut self, camera_pos: [f32; 3]) -> FluidFrameGuard<'_>;
}
```

### OptimizedStepResult

```rust
pub struct OptimizedStepResult {
    pub frame_time_ms: f32,
    pub iterations_used: u32,
    pub density_error: f32,
    pub quality_tier: u32,
    pub within_budget: bool,
}

impl OptimizedStepResult {
    pub fn suggests_quality_increase(&self) -> bool;
    pub fn suggests_quality_decrease(&self) -> bool;
}
```

### FluidFrameGuard

```rust
pub struct FluidFrameGuard<'a> {
    // RAII guard that auto-records frame time on drop
}

impl<'a> FluidFrameGuard<'a> {
    pub fn elapsed_ms(&self) -> f32;
    pub fn camera_position(&self) -> [f32; 3];
    pub fn quality_tier(&self) -> u32;
    pub fn render_context(&self) -> FluidRenderContext;
    pub fn finish(self);
}
```

## License

MIT License - See LICENSE file for details.
