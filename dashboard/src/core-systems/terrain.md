# Terrain System

> **Status**: Production Ready  
> **Coverage**: 71.5%  
> **Crate**: `astraweave-terrain`

AstraWeave's terrain system provides procedural terrain generation using noise functions, voxel data, and biome classification with full LOD support and async streaming.

## Overview

### Core Features

| Feature | Description |
|---------|-------------|
| **Voxel Data** | Density-based terrain with material IDs |
| **Marching Cubes** | 256-configuration mesh generation |
| **Biome System** | Climate-based biome distribution |
| **Erosion Simulation** | Hydraulic, thermal, and wind erosion |
| **LOD Management** | Distance-based quality with hysteresis |
| **Async Streaming** | Background chunk loading |

### Performance

| Benchmark | Result |
|-----------|--------|
| World chunk generation | 15.06 ms |
| Marching cubes mesh | ~5 ms per chunk |
| SIMD heightmap | 2-3× faster than scalar |

---

## Quick Start

```rust
use astraweave_terrain::{
    ChunkManager, TerrainChunk, ChunkId,
    Heightmap, HeightmapConfig,
    Biome, BiomeType,
};

// Create heightmap generator
let heightmap = Heightmap::new(HeightmapConfig {
    seed: 12345,
    octaves: 6,
    frequency: 0.01,
    amplitude: 100.0,
});

// Generate chunk
let chunk_id = ChunkId::new(0, 0);
let chunk = TerrainChunk::generate(&heightmap, chunk_id);

// Get height at world position
let height = heightmap.sample(world_x, world_z);
```

---

## Architecture

### Module Overview

```
astraweave-terrain/
├── Generation
│   ├── noise_gen.rs          # Noise generation
│   ├── noise_simd.rs         # SIMD-optimized noise
│   ├── heightmap.rs          # Heightmap generation
│   └── structures.rs         # Structure placement
├── Voxel System
│   ├── voxel_data.rs         # Voxel grid & chunks
│   ├── chunk.rs              # Chunk management
│   └── terrain_modifier.rs   # Runtime modification
├── Meshing
│   ├── meshing.rs            # Mesh generation
│   ├── marching_cubes_tables.rs  # MC lookup tables
│   └── lod_blending.rs       # LOD mesh morphing
├── Biomes
│   ├── biome.rs              # Biome types
│   ├── biome_blending.rs     # Smooth transitions
│   ├── climate.rs            # Climate simulation
│   └── scatter.rs            # Vegetation placement
├── Erosion
│   ├── erosion.rs            # Basic erosion
│   └── advanced_erosion.rs   # Multi-type erosion
├── Streaming
│   ├── background_loader.rs  # Async loading
│   ├── lod_manager.rs        # LOD control
│   └── streaming_diagnostics.rs  # Debug info
└── Integration
    ├── partition_integration.rs  # World streaming
    ├── texture_splatting.rs      # Material blending
    └── terrain_persistence.rs    # Save/load
```

---

## Voxel System

### Voxel Data Structure

```rust
use astraweave_terrain::{Voxel, Density, MaterialId, VoxelChunk, CHUNK_SIZE};

// Voxel with density and material
let voxel = Voxel {
    density: Density::new(0.7),  // 0.0 = air, 1.0 = solid
    material: MaterialId::STONE,
};

// Create empty chunk
let mut chunk = VoxelChunk::new();

// Set voxel at local position
chunk.set(x, y, z, voxel);

// Get voxel
let v = chunk.get(x, y, z);
```

### Chunk Coordinates

```rust
use astraweave_terrain::{ChunkCoord, ChunkId, CHUNK_SIZE};

// World position to chunk
let chunk_coord = ChunkCoord::from_world_pos(world_pos);

// Chunk ID for manager
let chunk_id = ChunkId::new(chunk_coord.x, chunk_coord.z);

// Local position within chunk
let local_pos = chunk_coord.to_local(world_pos);
```

---

## Heightmap Generation

### Basic Heightmap

```rust
use astraweave_terrain::{Heightmap, HeightmapConfig};

let config = HeightmapConfig {
    seed: 42,
    octaves: 6,           // Detail levels
    frequency: 0.01,      // Base frequency
    amplitude: 100.0,     // Height range
    persistence: 0.5,     // Octave decay
    lacunarity: 2.0,      // Octave frequency multiplier
};

let heightmap = Heightmap::new(config);

// Sample height at position
let height = heightmap.sample(x, z);

// Get normal at position
let normal = heightmap.normal(x, z);
```

### SIMD-Optimized Generation

For bulk generation (2-3× faster):

```rust
use astraweave_terrain::SimdHeightmapGenerator;

let generator = SimdHeightmapGenerator::new(config);

// Generate entire chunk heightmap
let heights: Vec<f32> = generator.generate_chunk(chunk_coord);
```

---

## Biome System

### Biome Types

```rust
use astraweave_terrain::{BiomeType, Biome, BiomeConfig};

pub enum BiomeType {
    Ocean,
    Beach,
    Plains,
    Forest,
    Taiga,
    Desert,
    Savanna,
    Jungle,
    Swamp,
    Mountain,
    SnowyMountain,
    Tundra,
}

// Biome configuration
let forest = BiomeConfig {
    biome_type: BiomeType::Forest,
    temperature_range: (0.3, 0.7),
    humidity_range: (0.5, 0.8),
    height_modifier: 1.0,
    vegetation_density: 0.8,
};
```

### Climate-Based Distribution

```rust
use astraweave_terrain::{ClimateMap, ClimateConfig};

let climate = ClimateMap::new(ClimateConfig {
    seed: 12345,
    temperature_scale: 0.001,
    humidity_scale: 0.002,
});

// Get biome at position
let biome = climate.get_biome_at(x, z);

// Get climate values
let (temp, humidity) = climate.sample(x, z);
```

### Biome Blending

```rust
use astraweave_terrain::{BiomeBlender, BiomeBlendConfig};

let blender = BiomeBlender::new(BiomeBlendConfig {
    blend_distance: 16.0,  // Transition width
    noise_strength: 0.2,   // Edge noise
});

// Get blended biome weights at position
let weights: Vec<BiomeWeight> = blender.get_weights(x, z);

// Apply to terrain materials
for weight in weights {
    material.blend(weight.biome, weight.strength);
}
```

---

## Mesh Generation

### Marching Cubes

```rust
use astraweave_terrain::meshing::{ChunkMesh, MeshVertex};

// Generate mesh from voxel chunk
let mesh = ChunkMesh::from_voxels(&chunk, &heightmap);

// Access mesh data
for vertex in mesh.vertices() {
    let pos = vertex.position;
    let normal = vertex.normal;
    let uv = vertex.uv;
}
```

### Dual Contouring

For sharper edges:

```rust
use astraweave_terrain::meshing::DualContouring;

let dc = DualContouring::new();
let mesh = dc.generate(&chunk, edge_sharpness: 0.8);
```

### LOD Mesh Generation

```rust
use astraweave_terrain::meshing::{LodMeshGenerator, LodConfig};

let generator = LodMeshGenerator::new(LodConfig {
    levels: vec![
        LodLevel { distance: 0.0, subdivisions: 8 },
        LodLevel { distance: 100.0, subdivisions: 4 },
        LodLevel { distance: 300.0, subdivisions: 2 },
        LodLevel { distance: 500.0, subdivisions: 1 },
    ],
});

// Generate LOD mesh for distance
let mesh = generator.generate(&chunk, camera_distance);
```

---

## Erosion Simulation

### Hydraulic Erosion

```rust
use astraweave_terrain::{
    AdvancedErosionSimulator,
    HydraulicErosionConfig,
    ErosionPreset,
};

let erosion = AdvancedErosionSimulator::new();

// Apply preset
let config = ErosionPreset::MountainRivers.into_config();
erosion.simulate_hydraulic(&mut heightmap, config, iterations: 50000);

// Or custom config
let custom = HydraulicErosionConfig {
    rain_amount: 0.01,
    erosion_rate: 0.3,
    deposition_rate: 0.3,
    evaporation_rate: 0.02,
    min_slope: 0.01,
};
```

### Thermal Erosion

```rust
use astraweave_terrain::ThermalErosionConfig;

let thermal_config = ThermalErosionConfig {
    talus_angle: 0.5,  // Max stable slope
    erosion_rate: 0.1,
};

erosion.simulate_thermal(&mut heightmap, thermal_config, iterations: 1000);
```

### Wind Erosion

```rust
use astraweave_terrain::WindErosionConfig;

let wind_config = WindErosionConfig {
    wind_direction: Vec3::new(1.0, 0.0, 0.5).normalize(),
    wind_strength: 0.5,
    particle_count: 10000,
};

erosion.simulate_wind(&mut heightmap, wind_config, iterations: 5000);
```

---

## Async Streaming

### Background Loader

```rust
use astraweave_terrain::{
    BackgroundChunkLoader,
    StreamingConfig,
    StreamingStats,
};

let loader = BackgroundChunkLoader::new(StreamingConfig {
    load_radius: 8,           // Chunks to keep loaded
    unload_radius: 12,        // Distance to unload
    max_concurrent_loads: 4,  // Parallel chunk loading
    priority_distance: 2,     // High-priority radius
});

// Update from camera position
loader.update(camera_pos);

// Poll for completed chunks
while let Some(chunk) = loader.poll_completed() {
    chunk_manager.insert(chunk);
}

// Get statistics
let stats: StreamingStats = loader.stats();
println!("Chunks loaded: {}", stats.chunks_loaded);
println!("Load queue: {}", stats.queue_length);
```

### LOD Manager

```rust
use astraweave_terrain::{LodManager, LodConfig, LodStats};

let lod_manager = LodManager::new(LodConfig {
    hysteresis: 0.1,  // Prevent LOD flickering
    update_interval: 0.5,  // Seconds between LOD updates
});

// Update LOD states
lod_manager.update(camera_pos, delta_time);

// Get current LOD for chunk
let lod_level = lod_manager.get_lod(chunk_id);
```

### Streaming Diagnostics

```rust
use astraweave_terrain::StreamingDiagnostics;

let diagnostics = StreamingDiagnostics::new();

// Record frame data
diagnostics.record_frame(frame_time, chunks_loaded, chunks_visible);

// Detect hitches
if let Some(hitch) = diagnostics.detect_hitch() {
    warn!("Streaming hitch: {:?}", hitch);
}

// Generate report
let report = diagnostics.generate_report();
```

---

## Runtime Modification

### Terrain Modifier

```rust
use astraweave_terrain::{TerrainModifier, VoxelOp, VoxelOpType};

let mut modifier = TerrainModifier::new(&mut chunk_manager);

// Dig sphere
modifier.apply(VoxelOp {
    op_type: VoxelOpType::Subtract,
    center: world_pos,
    radius: 5.0,
    material: None,
});

// Add material
modifier.apply(VoxelOp {
    op_type: VoxelOpType::Add,
    center: world_pos,
    radius: 3.0,
    material: Some(MaterialId::STONE),
});

// Smooth area
modifier.apply(VoxelOp {
    op_type: VoxelOpType::Smooth,
    center: world_pos,
    radius: 4.0,
    material: None,
});

// Batch operations for efficiency
modifier.begin_batch();
for op in operations {
    modifier.apply(op);
}
modifier.end_batch();  // Regenerates affected meshes once
```

---

## Texture Splatting

### Material Blending

```rust
use astraweave_terrain::{
    SplatMapGenerator,
    SplatConfig,
    SplatRule,
    TerrainMaterial,
};

let generator = SplatMapGenerator::new(SplatConfig {
    resolution: 512,
    rules: vec![
        SplatRule::height_based(TerrainMaterial::Grass, 0.0, 50.0),
        SplatRule::height_based(TerrainMaterial::Rock, 50.0, 100.0),
        SplatRule::height_based(TerrainMaterial::Snow, 100.0, 200.0),
        SplatRule::slope_based(TerrainMaterial::Cliff, 0.7, 1.0),
    ],
});

let splat_map = generator.generate(&heightmap);
```

### Triplanar Mapping

```rust
use astraweave_terrain::TriplanarWeights;

// Calculate triplanar weights from normal
let weights = TriplanarWeights::from_normal(vertex_normal);

// Sample texture with triplanar projection
let color = 
    sample_xz(uv_xz) * weights.y +
    sample_xy(uv_xy) * weights.z +
    sample_zy(uv_zy) * weights.x;
```

---

## Persistence

### Save/Load Terrain

```rust
use astraweave_terrain::terrain_persistence::{save_terrain, load_terrain};

// Save modified terrain
save_terrain(&chunk_manager, "saves/terrain.bin")?;

// Load terrain
let loaded = load_terrain("saves/terrain.bin")?;
chunk_manager.restore(&loaded);
```

---

## AI Integration

### Terrain Solver

For AI-driven terrain queries:

```rust
use astraweave_terrain::{TerrainSolver, ResolvedLocation};

let solver = TerrainSolver::new(&chunk_manager);

// Find valid location for AI agent
let result: ResolvedLocation = solver.find_valid_location(
    search_center,
    radius: 10.0,
    constraints: &[
        Constraint::MinHeight(5.0),
        Constraint::MaxSlope(0.3),
        Constraint::RequireBiome(BiomeType::Forest),
    ],
)?;

println!("Found location: {:?}", result.position);
```

---

## Blueprint Zone System

The Blueprint Zone system enables polygon-based spatial control over vegetation generation and heightmap injection, bridging `.blend` scene imports with the terrain scatter pipeline.

### Overview

```
Editor Canvas → BlueprintZone → ZoneScatterGenerator → ZoneGenerationResult
     ↓               ↓                ↓                      ↓
Polygon drawing   ZoneRegistry    Replica/Inspired      placements + patches
                  (save/load)      mode dispatch         ↓
                                                    apply_heightmap_patches()
                                                         ↓
                                                    TerrainChunk updates
```

### Zone Data Model

```rust
use astraweave_terrain::blueprint_zone::*;

// Define a zone with polygon vertices
let zone = BlueprintZone {
    id: ZoneId(1),
    name: "Forest Clearing".into(),
    vertices: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
    source: ZoneSource::BlendScene {
        pack_path: "assets/pine_forest.biomepack".into(),
        placement_mode: PlacementMode::Replica,
    },
    priority: 0,
    enabled: true,
};

// ZoneRegistry — CRUD, spatial queries, persistence
let mut registry = ZoneRegistry::new();
registry.add_zone(zone);
let zones = registry.zones_containing_point(50.0, 50.0);
registry.save(&Path::new("zones.json"))?;
let loaded = ZoneRegistry::load(&Path::new("zones.json"))?;
```

### Placement Modes

| Mode | Behavior |
|------|----------|
| **Replica** | 1:1 reproduction — fixed positions from `.blend` scene, scaled by `AdaptiveScaleParams` |
| **Inspired** | Procedural scatter using `ScatterConfig` derived from `BiomePack`, respects zone polygon |
| **BiomePreset** | Pure biome-driven scatter (Grassland, Forest, Desert, etc.) without `.blend` data |

### Zone-Scoped Generation

```rust
use astraweave_terrain::zone_scatter::*;

let gen = ZoneScatterGenerator::new(256.0, 128); // chunk_size, heightmap_resolution
let result = gen.generate_zone_scatter(&zone, &biome_pack)?;

// result.placements — Vec<VegetationInstance> (position, rotation, scale, model_path)
// result.heightmap_patches — Vec<HeightmapPatch> (per-chunk height modifications)

// Apply heightmap patches to terrain chunks
let results = vec![result];
apply_heightmap_patches(&mut chunk_map, &results);
```

### Adaptive Scaling

When zone area differs from source scene footprint, density and scale adjust automatically:

```rust
let params = AdaptiveScaleParams::compute(reference_area, zone_area);
// density_multiplier = sqrt(zone_area / reference_area)
// scale_multiplier   = (zone_area / reference_area)^0.25
// position_scale     = sqrt(zone_area / reference_area)
```

### Boundary Blending

`apply_boundary_blending()` uses smoothstep falloff at zone edges:
- **Vegetation**: density fades via `BlendMask::sample(x, z)` → 0.0–1.0
- **Heightmap**: height delta scaled by mask value at each sample point
- **Manual**: `BrushMode::ZoneBlend` for blend-weight painting in-editor

### Heightmap Rasterization

Terrain meshes from `.blend` decomposition are rasterized into heightmaps:

```rust
use astraweave_blend::heightmap_raster::*;

let heightmap = rasterize_terrain_meshes(&terrain_meshes, resolution)?;
let height = heightmap.sample_bilinear(u, v); // Normalized [0,1] coords
let area = heightmap.footprint_area();         // World-space area in m²
```

### Editor Integration

The `BlueprintPanel` provides a 2D canvas for polygon zone editing:
- Tools: Select, DrawPolygon, MoveVertex, DeleteZone
- Zone inspector with name, source selection, placement mode toggle
- Undo/redo via command stack
- Save/Load zones as `.zones.json`

The `BlueprintOverlay` projects zone polygons as wireframe outlines into the 3D viewport, integrated with the physics renderer debug line pass.

### Test Coverage

| Test Suite | Tests |
|------------|-------|
| `blueprint_zone` unit tests | 24 |
| `zone_scatter` unit + integration | 27 |
| `heightmap_raster` unit + integration | 21 |
| `biome_pack` extended | 9 |
| `blueprint_panel` unit tests | 17 |
| `blueprint_overlay` unit tests | 7 |
| `blend_scanner` unit tests | 8 |
| **Total** | **113+** |

---

## See Also

- [API Reference](../api/terrain.md) - Detailed API documentation
- [Fluids System](./fluids.md) - Water integration
- [Physics System](./physics.md) - Collision terrain
- [Navigation](./navigation.md) - Navmesh from terrain
