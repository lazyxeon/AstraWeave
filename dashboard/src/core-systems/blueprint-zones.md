# Blueprint Zone Editor

> **Status**: Production Ready  
> **Tests**: 125+  
> **Crates**: `astraweave-terrain`, `astraweave-blend`, `aw_editor`

The Blueprint Zone system provides polygon-based spatial control for vegetation generation and heightmap injection. It bridges `.blend` scene imports with AstraWeave's terrain scatter pipeline, enabling both exact 1:1 reproduction (Replica mode) and procedural variation (Inspired mode) of imported environments.

## Overview

### Pipeline

```
.blend Scene → Decomposition → BiomePack → BlueprintZone → ZoneScatterGenerator
                    ↓                           ↓                    ↓
              heightmap_raster           ZoneRegistry          VegetationInstance
              (terrain mesh →            (polygon CRUD,        + HeightmapPatch
               heightmap + fixed          spatial queries,          ↓
               placements)                JSON persistence)   apply_heightmap_patches()
                                                                    ↓
                                                              TerrainChunk updates
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Polygon Zones** | Arbitrary polygons define generation boundaries |
| **Replica Mode** | 1:1 fixed placement from `.blend` scene data |
| **Inspired Mode** | Procedural scatter using extracted `ScatterConfig` |
| **Biome Presets** | Pure biome-driven scatter without `.blend` data |
| **Heightmap Injection** | Rasterized terrain meshes applied as height patches |
| **Boundary Blending** | Smoothstep falloff prevents hard cutoffs at zone edges |
| **Adaptive Scaling** | Auto-adjusts density/scale when zone ≠ source area |
| **Undo/Redo** | Full command stack in blueprint panel |
| **3D Overlay** | Wireframe zone projection in viewport |

---

## Quick Start

```rust
use astraweave_terrain::blueprint_zone::*;
use astraweave_terrain::zone_scatter::*;
use std::path::Path;

// 1. Define a zone
let zone = BlueprintZone {
    id: ZoneId(1),
    name: "Pine Forest".into(),
    vertices: vec![
        [0.0, 0.0], [200.0, 0.0],
        [200.0, 150.0], [0.0, 150.0],
    ],
    source: ZoneSource::BlendScene {
        pack_path: "assets/pine_forest.biomepack".into(),
        placement_mode: PlacementMode::Replica,
    },
    priority: 0,
    enabled: true,
};

// 2. Register it
let mut registry = ZoneRegistry::new();
registry.add_zone(zone.clone());

// 3. Generate scatter
let gen = ZoneScatterGenerator::new(256.0, 128);
let result = gen.generate_zone_scatter(&zone, &biome_pack)?;

println!("{} placements", result.placement_count());
println!("{} height modifications", result.modified_height_count());

// 4. Apply heightmap patches to terrain
let mut chunks = HashMap::new();
// ... populate with TerrainChunks ...
apply_heightmap_patches(&mut chunks, &[result]);

// 5. Persist zones
registry.save(Path::new("assets/zones.json"))?;
```

---

## Zone Data Model

### BlueprintZone

A `BlueprintZone` represents a polygon region with a vegetation/terrain source:

```rust
pub struct BlueprintZone {
    pub id: ZoneId,                    // Unique identifier
    pub name: String,                  // Display name
    pub vertices: Vec<[f32; 2]>,       // Polygon vertices (XZ plane)
    pub source: ZoneSource,            // What to generate
    pub priority: u32,                 // Overlap resolution (higher wins)
    pub enabled: bool,                 // Toggle generation
}
```

### ZoneSource

```rust
pub enum ZoneSource {
    BiomePreset(BiomeType),            // Built-in biome scatter
    BlendScene {
        pack_path: String,             // Path to .biomepack
        placement_mode: PlacementMode, // Replica or Inspired
    },
}
```

### PlacementMode

| Mode | Description |
|------|-------------|
| `Replica` | Exact positions from `.blend` scene, scaled by `AdaptiveScaleParams` |
| `Inspired` | Procedural scatter using extracted density/distribution rules |

### ZoneRegistry

```rust
let mut registry = ZoneRegistry::new();

// CRUD operations
registry.add_zone(zone);
registry.remove_zone(zone_id);
let zone = registry.get_zone(zone_id);
let zone_mut = registry.get_zone_mut(zone_id);

// Spatial queries
let zones = registry.zones_containing_point(x, z);
let overlaps = registry.zones_overlapping_rect(min_x, min_z, max_x, max_z);

// Persistence
registry.save(Path::new("zones.json"))?;
let loaded = ZoneRegistry::load(Path::new("zones.json"))?;
```

---

## Zone-Scoped Generation

### ZoneScatterGenerator

```rust
use astraweave_terrain::zone_scatter::*;

// Create generator with chunk size and heightmap resolution
let gen = ZoneScatterGenerator::new(256.0, 128);

// Generate for a single zone
let result: ZoneGenerationResult = gen.generate_zone_scatter(&zone, &biome_pack)?;
```

### ZoneGenerationResult

```rust
pub struct ZoneGenerationResult {
    pub placements: Vec<VegetationInstance>,     // What to place
    pub heightmap_patches: Vec<HeightmapPatch>,  // Height modifications
}

// VegetationInstance fields:
// - position: Vec3
// - rotation: f32
// - scale: f32
// - vegetation_type: String
// - model_path: String
// - terrain_normal: Vec3
```

### Multi-Zone Generation

```rust
// Generate scatter for all zones with overlap priority resolution
let results = generate_multi_zone_scatter(&zones, &gen, &biome_packs)?;

// Apply all heightmap patches at once
apply_heightmap_patches(&mut chunk_map, &results);
```

---

## Adaptive Scaling

When a zone's area differs from the source scene's footprint, scaling parameters adjust automatically:

```rust
let params = AdaptiveScaleParams::compute(reference_area, zone_area);
```

| Parameter | Formula | Effect |
|-----------|---------|--------|
| `density_multiplier` | $\sqrt{\frac{\text{zone}}{\text{ref}}}$ | Adjusts placement density |
| `scale_multiplier` | $\left(\frac{\text{zone}}{\text{ref}}\right)^{0.25}$ | Adjusts object scale |
| `position_scale` | $\sqrt{\frac{\text{zone}}{\text{ref}}}$ | Adjusts position spread |

**Example**: A zone 4× larger than the source scene would have:
- `density_multiplier` = 2.0 (double the objects)
- `scale_multiplier` ≈ 1.41 (slightly larger objects)
- `position_scale` = 2.0 (spread positions wider)

---

## Boundary Blending

### Smoothstep Falloff

Zone edges use smoothstep interpolation to prevent hard vegetation/height cutoffs:

```rust
// BlendMask provides per-point blending weights
let mask = BlendMask::new(resolution, world_bounds);
let weight = mask.sample(x, z); // 0.0 at edge → 1.0 at center
```

### Manual Painting

The editor provides `BrushMode::ZoneBlend` for manually painting blend weights at zone boundaries, giving artists fine control over transition regions.

---

## Heightmap Rasterization

Terrain meshes extracted from `.blend` files are rasterized into heightmaps:

```rust
use astraweave_blend::heightmap_raster::*;

let heightmap = rasterize_terrain_meshes(&terrain_meshes, 128)?;

// Query rasterized data
let height = heightmap.sample_bilinear(0.5, 0.5); // Normalized UV coords
let area = heightmap.footprint_area();              // m² world-space

// Fixed placements (exact object positions from scene)
let placements: Vec<FixedPlacement> = extract_fixed_placements(&scene_objects);
```

The rasterizer uses ray-triangle intersection with:
- **Seam averaging** for multi-tile terrain boundaries
- **Hole filling** via neighbor interpolation for missing samples

---

## Editor Integration

### Blueprint Panel

The `BlueprintPanel` (panel type: `Blueprint`) provides:

- **2D Canvas**: Pan/zoom view for polygon zone drawing
- **Tools**: Select, DrawPolygon, MoveVertex, DeleteZone
- **Zone Inspector**: Name, source selection (biome preset or blend scene), placement mode toggle
- **Undo/Redo**: Full `BlueprintCommand` stack with Ctrl+Z/Ctrl+Shift+Z
- **Persistence**: Save/Load zones as `.zones.json` files

### Viewport Overlay

`BlueprintOverlay` projects zone polygons as colored wireframe outlines in the 3D viewport. Zone boundaries are rendered alongside component gizmos and brush cursors in the physics renderer debug line pass.

### Asset Browser

The `BlendAssetScanner` adds `.blend` file discovery to the asset browser with:
- Automatic directory scanning for `.blend` files
- Decomposition status detection (checking for `manifest.json`)
- Quick actions: Import Blend Scene, Use as Zone Source

### System Wiring

The editor's update loop dispatches `BlueprintAction` events:

| Action | Handler | Effect |
|--------|---------|--------|
| `GenerateZone` | `handle_generate_zone()` | Runs `ZoneScatterGenerator` for one zone |
| `GenerateAll` | Iterates all zones | Generates scatter for every enabled zone |
| `ClearGeneration` | Clears results | Removes generated placements |
| `SaveZones` | `handle_save_zones()` | Saves `ZoneRegistry` to JSON |
| `LoadZones` | `handle_load_zones()` | Loads registry + syncs panel state |

After each action, `sync_zone_overlay()` pushes updated zone data to the viewport.

---

## Test Coverage

| Suite | Count | Scope |
|-------|-------|-------|
| `blueprint_zone` unit | 24 | Zone model, registry, polygon math, persistence |
| `zone_scatter` unit | 16 | Generation modes, blending, heightmap patches |
| `zone_scatter_e2e` integration | 11 | Full pipeline, multi-zone, chunk spanning |
| `heightmap_raster` unit | 11 | Rasterization, terrain bounds, seam stitching |
| `heightmap_raster_e2e` integration | 10 | Multi-tile, bilinear sampling, edge cases |
| `biome_pack` unit | 9 | BiomePack extension fields, detection |
| `blueprint_panel` unit | 17 | Canvas, tools, undo/redo, actions |
| `blueprint_overlay` unit | 7 | Wireframe generation, color mapping |
| `blend_scanner` unit | 8 | Directory scanning, status detection |
| **Total** | **113+** | |

---

## See Also

- [Terrain System](./terrain.md) - Core terrain generation and streaming
- [Rendering](./rendering.md) - Renderer debug line pass for overlays
- [Physics](./physics.md) - DebugLine integration
