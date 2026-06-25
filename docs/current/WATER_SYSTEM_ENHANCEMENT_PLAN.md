# AstraWeave Advanced Water System Enhancement Plan

> **⚠ STALENESS BANNER (added W-series W.2 Phase 2, 2026-06-21).** This plan
> describes an **abandoned direction** — an Enshrouded-style hybrid **voxel water
> grid** with swimming/diving/fishing/water-wheels and "500k+ particle" SPH.
> F.3.S measured that CPU voxel water cannot carry general gameplay water at
> interactive cost on min-spec (decline A→C), and the W-series (Water Successor)
> **deleted** the voxel sim + SPH solver inventory in W.1 (commit `1a57fdd41`;
> recovery tag `w0-pre-deprecation`), re-scoping water to a layered **rendering**
> system. The plan below is **not pursued** — the current authority is
> `docs/campaigns/water-successor/W2_0_RECON.md` and `W2_DECISIONS.md`. **Read
> this as historical; do not cite its plan as current.**

**Version**: 1.0.0  
**Date**: January 22, 2026  
**Status**: 🎯 PLANNED  
**Reference**: Enshrouded "Wake of Water" Update (November 2025)

---

## Executive Summary

This document outlines a comprehensive plan to enhance AstraWeave's water system to match and exceed the capabilities demonstrated in Enshrouded's sophisticated "Wake of Water" update. Our approach leverages AstraWeave's existing GPU-based fluid simulation infrastructure while adding the missing gameplay, building, and interaction systems.

### Current State Assessment

**AstraWeave Strengths** (Already Implemented):
- ✅ **GPU Particle-Based Fluid Simulation** - 500k+ particles via PBD (Position-Based Dynamics)
- ✅ **Gerstner Wave Rendering** - 4-component ocean waves with foam, Fresnel reflections
- ✅ **Terrain Integration** - Automatic river/lake/waterfall detection from heightmaps
- ✅ **Buoyancy Physics** - Volume-based floating via Rapier3D integration
- ✅ **Screen-Space Fluid Rendering** - Smooth depth surfaces for realistic water
- ✅ **Waterfall/Mist Particles** - Secondary particle emission system
- ✅ **SDF Collision** - Signed distance field collision for dynamic objects

**Gaps vs Enshrouded** (To Be Implemented):
- ❌ **Volumetric Water Grid** - Voxel-based water volume simulation (flow, fill, split)
- ❌ **Player Water Mechanics** - Swimming, diving, oxygen, underwater movement
- ❌ **Water Dispensers/Drains** - Placeable water sources and sinks
- ❌ **Flow Control** - Gates, barriers, irrigation canals
- ❌ **Material Absorption** - Terrain-specific water absorption rates
- ❌ **Structure Interactions** - Flooding effects on crafting stations, torches
- ❌ **Water Wheels** - Mechanical power from water flow
- ❌ **Fishing System** - Water-based minigame
- ❌ **Underwater Rendering** - Caustics, god rays, fog when submerged

---

## Architecture Overview

### Hybrid Simulation Approach

We'll implement a **hybrid simulation** combining:

1. **Volumetric Water Grid** (New) - Voxel-based for building/terrain interaction
2. **GPU Particle Simulation** (Existing) - For visual detail and flowing water effects
3. **Surface Rendering** (Existing) - Gerstner waves for large water bodies

```
┌─────────────────────────────────────────────────────────────────────┐
│                    AstraWeave Advanced Water System                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐       │
│  │  Volumetric     │  │  GPU Particle   │  │  Surface Wave   │       │
│  │  Water Grid     │◄─┼─►  Simulation    │◄─┼─►  Rendering     │       │
│  │  (Voxels)       │  │  (500k+ PBD)   │  │  (Gerstner)     │       │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘       │
│           │                    │                    │                 │
│  ┌────────▼────────────────────▼────────────────────▼────────┐       │
│  │                    Water State Manager                      │       │
│  │  • Volume tracking  • Flow simulation  • Level changes     │       │
│  └────────┬────────────────────┬────────────────────┬────────┘       │
│           │                    │                    │                 │
│  ┌────────▼────────┐  ┌────────▼────────┐  ┌────────▼────────┐       │
│  │  Gameplay       │  │  Building       │  │  Rendering      │       │
│  │  Integration    │  │  Integration    │  │  Integration    │       │
│  │  • Swimming     │  │  • Dispensers   │  │  • Underwater   │       │
│  │  • Oxygen       │  │  • Drains       │  │  • Caustics     │       │
│  │  • Fishing      │  │  • Water Wheels │  │  • Reflections  │       │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘       │
│                                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Volumetric Water Grid (2-3 weeks)

### 1.1 Water Volume Voxel System

Create a new module `astraweave-fluids/src/volume_grid.rs`:

```rust
/// Volumetric water simulation using cellular automaton
pub struct WaterVolumeGrid {
    /// Water level at each cell (0.0 = empty, 1.0 = full)
    levels: Vec<f32>,
    /// Flow velocity at each cell
    velocities: Vec<Vec3>,
    /// Material type affecting absorption
    materials: Vec<MaterialType>,
    /// Grid dimensions
    dimensions: UVec3,
    /// Cell size in world units
    cell_size: f32,
    /// World-space origin
    origin: Vec3,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MaterialType {
    Air,           // No absorption
    Stone,         // No absorption (constructed blocks)
    Soil,          // Low absorption (~0.01 per second)
    Mud,           // Full absorption (removes water)
    Rubble,        // Moderate absorption
    Shroud,        // Rapid dissipation
}
```

### 1.2 Flow Simulation Algorithm

Implement hydrostatic pressure simulation (similar to Enshrouded):

```rust
impl WaterVolumeGrid {
    /// Simulate one timestep of water flow
    pub fn simulate(&mut self, dt: f32) {
        // Phase 1: Compute pressure differentials
        self.compute_pressure();
        
        // Phase 2: Flow equalization (horizontal spreading)
        self.flow_horizontal(dt);
        
        // Phase 3: Gravity-driven vertical flow
        self.flow_vertical(dt);
        
        // Phase 4: Material absorption
        self.apply_absorption(dt);
        
        // Phase 5: Update particle system from volume changes
        self.sync_particles();
    }
    
    /// U-bend support: water can travel upward through pressure
    fn compute_pressure(&mut self) {
        // Hydrostatic pressure = density * g * h
        // Water will flow upward if pressure at bottom exceeds column height
    }
}
```

### 1.3 Enshrouded Parity Features

| Feature | Enshrouded Behavior | AstraWeave Implementation |
|---------|---------------------|---------------------------|
| Flow Rate | 36 blocks/second from dispenser | Configurable via `WaterDispenser::flow_rate` |
| Split Flow | Divides at forks | Pressure-based flow distribution |
| U-Bend | Travels upward through pressure | Hydrostatic pressure simulation |
| Viscosity | Gradual spreading | Damped flow with configurable viscosity |
| Persistence | Stable pools don't absorb | Only flowing water triggers absorption |

---

## Phase 2: Player Water Mechanics (1-2 weeks)

### 2.1 Swimming/Diving Controller

Add to `astraweave-gameplay/src/water_movement.rs`:

```rust
/// Player water interaction state
pub struct WaterPlayerState {
    /// Current submersion level (0.0 = dry, 1.0 = fully submerged)
    pub submersion: f32,
    /// Remaining oxygen (seconds)
    pub oxygen: f32,
    /// Maximum oxygen capacity
    pub max_oxygen: f32,
    /// Current movement mode
    pub mode: WaterMovementMode,
    /// Wet/Soaking debuff timer
    pub wet_timer: f32,
}

pub enum WaterMovementMode {
    Walking,        // Shallow water, normal movement
    Wading,         // Waist-deep, reduced speed
    Swimming,       // Surface swimming
    Diving,         // Underwater, oxygen consumed
}

impl WaterPlayerState {
    /// Update water state based on player position
    pub fn update(&mut self, water_grid: &WaterVolumeGrid, player_pos: Vec3, dt: f32) {
        self.submersion = water_grid.sample_submersion(player_pos, 1.8); // Player height
        
        self.mode = match self.submersion {
            s if s < 0.3 => WaterMovementMode::Walking,
            s if s < 0.6 => WaterMovementMode::Wading,
            s if s < 0.95 => WaterMovementMode::Swimming,
            _ => WaterMovementMode::Diving,
        };
        
        // Consume oxygen when diving
        if matches!(self.mode, WaterMovementMode::Diving) {
            self.oxygen = (self.oxygen - dt).max(0.0);
        } else {
            self.oxygen = (self.oxygen + dt * 3.0).min(self.max_oxygen);
        }
        
        // Wet debuff
        if self.submersion > 0.0 {
            self.wet_timer = 30.0; // 30 seconds to dry
        } else {
            self.wet_timer = (self.wet_timer - dt).max(0.0);
        }
    }
}
```

### 2.2 Enshrouded Skills Integration

Match Enshrouded's water-related skills:

```rust
/// Water-related player skills
pub struct WaterSkills {
    /// "Splash Dash" - Forward lunge while swimming
    pub splash_dash: bool,
    /// "Wet Dog/Wetter Dog" - Reduces stamina debuff from wet status
    pub wet_resistance_level: u8, // 0=none, 1=25%, 2=50%
}
```

### 2.3 Movement Modifiers

| Condition | Enshrouded Effect | AstraWeave Implementation |
|-----------|-------------------|---------------------------|
| Wading | Reduced speed | `movement_speed *= 0.6` |
| Swimming | Stamina drain | `stamina_drain *= 1.5` |
| Diving | Oxygen consumption | `oxygen -= dt` |
| Wet Debuff | -50% stamina regen | Debuff system integration |
| Soaking Wet | -50% stamina + regen | Extended wet timer |

---

## Phase 3: Water Building System (2 weeks)

### 3.1 Water Dispenser Component

```rust
/// Placeable water source that generates water volume
pub struct WaterDispenser {
    /// Flow rate in blocks per second (Enshrouded default: 36)
    pub flow_rate: f32,
    /// Whether the dispenser is active
    pub active: bool,
    /// Auto-shutoff when water reaches dispenser level
    pub auto_shutoff: bool,
    /// Direction of water emission (default: down)
    pub emission_direction: Vec3,
}

impl WaterDispenser {
    pub fn tick(&mut self, water_grid: &mut WaterVolumeGrid, position: IVec3, dt: f32) {
        if !self.active {
            return;
        }
        
        // Auto-shutoff: check if water level covers dispenser
        let water_level = water_grid.get_level(position);
        if self.auto_shutoff && water_level > 0.9 {
            return; // Stop producing water
        }
        
        // Emit water
        let target = position + self.emission_direction.as_ivec3();
        water_grid.add_water(target, self.flow_rate * dt);
    }
}
```

### 3.2 Water Drain Component

```rust
/// Removes water from the simulation at a specific level
pub struct WaterDrain {
    /// Maximum level water can reach (water above this drains)
    pub max_level: f32,
    /// Drain rate in blocks per second
    pub drain_rate: f32,
    /// Whether drain is active
    pub active: bool,
}
```

### 3.3 Water Gate Component

```rust
/// Controllable barrier for water flow
pub struct WaterGate {
    /// Current open percentage (0.0 = closed, 1.0 = fully open)
    pub openness: f32,
    /// Opening/closing speed
    pub speed: f32,
    /// Target state
    pub target_open: bool,
}

impl WaterGate {
    pub fn update(&mut self, dt: f32) {
        let target = if self.target_open { 1.0 } else { 0.0 };
        self.openness += (target - self.openness).signum() * self.speed * dt;
        self.openness = self.openness.clamp(0.0, 1.0);
    }
    
    /// Flow multiplier for water passing through
    pub fn flow_multiplier(&self) -> f32 {
        self.openness
    }
}
```

### 3.4 Water Wheel (Power Generation)

```rust
/// Generates mechanical power from water flow
pub struct WaterWheel {
    /// Current rotation speed (RPM)
    pub rotation_speed: f32,
    /// Power output (arbitrary units matching Enshrouded's "force")
    pub power_output: f32,
    /// Required minimum flow rate to turn
    pub min_flow_rate: f32,
}

impl WaterWheel {
    pub fn update(&mut self, water_grid: &WaterVolumeGrid, position: IVec3, dt: f32) {
        let flow = water_grid.get_flow_rate(position);
        
        if flow >= self.min_flow_rate {
            self.rotation_speed = flow * 10.0; // RPM proportional to flow
            self.power_output = self.rotation_speed * 0.1;
        } else {
            self.rotation_speed *= 0.9; // Slow down
            self.power_output = 0.0;
        }
    }
}
```

---

## Phase 4: Structure Interactions (1 week)

### 4.1 Flooding System

```rust
/// Checks if a structure is flooded and applies effects
pub fn check_flooding(
    water_grid: &WaterVolumeGrid,
    position: IVec3,
    structure_type: StructureType,
) -> FloodingResult {
    let water_level = water_grid.get_level(position);
    
    match structure_type {
        StructureType::CraftingStation => {
            if water_level > 0.5 {
                FloodingResult::Disabled("Crafting station flooded")
            } else {
                FloodingResult::Functional
            }
        }
        StructureType::Torch | StructureType::Fireplace => {
            if water_level > 0.1 {
                FloodingResult::Flickering // Visual effect, reduced light
            } else {
                FloodingResult::Functional
            }
        }
        StructureType::Bed => {
            if water_level > 0.3 {
                FloodingResult::Unassigned // Bed becomes unusable
            } else {
                FloodingResult::Functional
            }
        }
        _ => FloodingResult::Functional,
    }
}
```

### 4.2 Flame Altar Water Removal (Enshrouded Feature)

```rust
/// Emergency water removal within a player base
pub fn remove_all_water_in_base(
    water_grid: &mut WaterVolumeGrid,
    base_bounds: &Aabb,
) -> u32 {
    let mut removed_count = 0;
    
    for pos in base_bounds.iter_cells() {
        let level = water_grid.get_level(pos);
        if level > 0.0 {
            water_grid.set_level(pos, 0.0);
            removed_count += 1;
        }
    }
    
    removed_count
}
```

---

## Phase 5: Advanced Rendering (1-2 weeks)

### 5.1 Underwater Rendering Effects

Add to `astraweave-render/src/underwater.rs`:

```rust
/// Underwater post-processing effects
pub struct UnderwaterRenderer {
    /// Caustics texture (animated light patterns)
    caustics_texture: wgpu::Texture,
    /// Underwater fog color (depth-based)
    fog_color: Vec3,
    /// Fog density
    fog_density: f32,
    /// God ray intensity
    god_ray_intensity: f32,
}

impl UnderwaterRenderer {
    /// Render underwater effects when camera is submerged
    pub fn render(&self, render_pass: &mut wgpu::RenderPass, camera_depth: f32) {
        if camera_depth <= 0.0 {
            return; // Camera above water
        }
        
        // Apply fog based on depth
        let fog_factor = 1.0 - (-camera_depth * self.fog_density).exp();
        
        // Render caustics on surfaces
        // Render god rays
    }
}
```

### 5.2 Water Audio System

Match Enshrouded's sophisticated water audio:

```rust
/// Water-related audio management
pub struct WaterAudioSystem {
    /// Ambient water sounds (flowing, still pools)
    ambient_sources: Vec<SpatialAudioSource>,
    /// Splash sounds pool
    splash_sounds: SoundPool,
    /// Underwater muffling effect
    underwater_filter: AudioFilter,
}
```

---

## Phase 6: Fishing System (Optional, 1 week)

### 6.1 Fishing Minigame

```rust
/// Fishing activity state
pub struct FishingState {
    /// Current fishing stage
    pub stage: FishingStage,
    /// Time in current stage
    pub stage_timer: f32,
    /// Currently hooked fish (if any)
    pub hooked_fish: Option<FishType>,
    /// Player's "fishing endurance" for the battle
    pub endurance: f32,
}

pub enum FishingStage {
    Casting,
    Waiting { bite_timer: f32 },
    Hooking { window: f32 },
    Reeling { fish: FishType, tension: f32 },
    Caught { fish: FishType },
    Failed,
}
```

---

## Implementation Timeline

### Week 1-2: Volumetric Water Grid
- [ ] Implement `WaterVolumeGrid` struct
- [ ] Flow simulation with hydrostatic pressure
- [ ] Material absorption system
- [ ] Sync with existing particle system
- [ ] Unit tests for flow behavior

### Week 3: Player Water Mechanics
- [ ] `WaterPlayerState` component
- [ ] Swimming/diving movement
- [ ] Oxygen system with UI
- [ ] Wet debuff integration
- [ ] Movement speed modifiers

### Week 4-5: Building Integration
- [ ] Water Dispenser component
- [ ] Water Drain component
- [ ] Water Gate component
- [ ] Water Wheel power system
- [ ] Editor UI for water tools

### Week 6: Structure Interactions
- [ ] Flooding detection system
- [ ] Structure effect application
- [ ] Flame Altar water removal
- [ ] Persistence for water in bases

### Week 7-8: Rendering & Polish
- [ ] Underwater camera effects
- [ ] Caustics rendering
- [ ] Water audio system
- [ ] Performance optimization

### Week 9 (Optional): Fishing
- [ ] Fishing minigame implementation
- [ ] Fish types and locations
- [ ] Fishing rod items
- [ ] Bait system

---

## Performance Considerations

### Volumetric Grid Optimization

```rust
/// Configuration for water simulation performance
pub struct WaterSimulationConfig {
    /// Maximum grid resolution (cells per axis)
    pub max_resolution: u32, // Default: 256
    /// Simulation tick rate (Hz)
    pub tick_rate: f32, // Default: 30.0
    /// Maximum simulation radius from player
    pub simulation_radius: f32, // Default: 100.0
    /// Enable GPU acceleration for large grids
    pub gpu_accelerated: bool,
}
```

### Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Flow simulation (256³) | <5ms | Per tick @ 30Hz |
| Rendering integration | <1ms | Particle sync per frame |
| Player query | <0.1ms | Submersion check |
| Structure flooding | <0.5ms | Per tick for loaded structures |

---

## Comparison: AstraWeave vs Enshrouded (Post-Implementation)

| Feature | Enshrouded | AstraWeave (Planned) | AstraWeave Advantage |
|---------|------------|---------------------|----------------------|
| Flow Simulation | Voxel-based | Hybrid (Voxel + GPU Particles) | ✅ Higher visual fidelity |
| Particle Count | Unknown | 500k+ | ✅ Massive scale |
| Flow Rate | 36 blocks/sec | Configurable | ✅ More flexibility |
| Material Types | 6+ | 6+ (matching) | ✅ Parity |
| Swimming | Basic | Full physics | ✅ More realistic |
| Underwater Rendering | Reflections | Caustics + God Rays + Fog | ✅ Superior visuals |
| Performance | Optimized | GPU-accelerated | ✅ Better scaling |
| Determinism | Unknown | 100% deterministic | ✅ Multiplayer-ready |

---

## Files to Create/Modify

### New Files

1. `astraweave-fluids/src/volume_grid.rs` - Volumetric water simulation
2. `astraweave-fluids/src/flow.rs` - Flow physics and hydrostatics
3. `astraweave-fluids/src/building.rs` - Dispenser/drain/gate/wheel
4. `astraweave-gameplay/src/water_movement.rs` - Player water mechanics
5. `astraweave-gameplay/src/fishing.rs` - Fishing minigame
6. `astraweave-render/src/underwater.rs` - Underwater rendering effects
7. `astraweave-fluids/shaders/volume_flow.wgsl` - GPU flow compute shader

### Modified Files

1. `astraweave-fluids/src/lib.rs` - Export new modules
2. `astraweave-fluids/src/terrain_integration.rs` - Connect to volume grid
3. `astraweave-render/src/water.rs` - Integrate with volume grid
4. `astraweave-gameplay/src/lib.rs` - Export water gameplay
5. `astraweave-physics/src/lib.rs` - Enhanced buoyancy from volume grid

---

## Success Criteria

### Minimum Viable Product (Week 5)
- [ ] Water flows and fills terrain depressions
- [ ] Player can swim and dive with oxygen
- [ ] Water dispensers create water
- [ ] Drains remove water at set levels
- [ ] Basic underwater fog effect

### Full Feature Parity (Week 8)
- [ ] All Enshrouded water features matched
- [ ] Material absorption working
- [ ] Structure flooding effects
- [ ] Water wheels generate power
- [ ] Underwater caustics and god rays

### Exceeds Enshrouded (Week 9+)
- [ ] 500k+ particle visual enhancement
- [ ] GPU-accelerated flow for massive grids
- [ ] Advanced fishing with AI-generated fish behavior
- [ ] Multiplayer-synchronized water state

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance with large grids | High | GPU compute shaders, LOD for distant water |
| Multiplayer sync complexity | Medium | Deterministic simulation, delta compression |
| Visual quality vs performance | Medium | Configurable quality presets |
| Memory usage for volume grid | Medium | Sparse storage, streaming |

---

## Conclusion

This plan provides a comprehensive roadmap to match and exceed Enshrouded's water system. AstraWeave's existing GPU fluid simulation gives us a significant visual quality advantage, while the new volumetric grid will enable the gameplay features players expect.

**Estimated Total Effort**: 6-9 weeks for full implementation

**Key Differentiator**: AstraWeave will have the visual fidelity of a AAA engine's water (500k+ particles, caustics, god rays) combined with the gameplay depth of Enshrouded's building-focused water mechanics.

---

**Author**: GitHub Copilot  
**Project**: AstraWeave AI-Native Gaming Engine  
**Last Updated**: January 22, 2026
