# Greybox Asset Workflow for Veilweaver

**Version**: 1.0  
**Date**: November 8, 2025  
**Status**: ✅ APPROVED  
**Author**: AI Team  

---

## Executive Summary

This document defines the complete asset pipeline for creating greybox (placeholder) geometry for the Veilweaver vertical slice. The workflow uses **GLTF 2.0 (.glb format)** as the standard mesh format, leveraging AstraWeave's existing wgpu + gltf 1.4 infrastructure.

**Format Decision**: **GLTF 2.0 (.glb embedded binary format)** ✅

**Rationale**:
- ✅ **Native wgpu support**: AstraWeave already uses `gltf = "1.4"` with import features
- ✅ **900+ existing GLB files**: Proven format in production (assets/models/*.glb)
- ✅ **Open standard**: Royalty-free, maintained by Khronos Group
- ✅ **Blender native export**: Built-in GLTF 2.0 exporter (no plugins required)
- ✅ **Embedded textures**: .glb bundles mesh + textures in single file (easier management)
- ✅ **Animation support**: Full skeletal animation (future-proof for character work)

**Rejected Alternative: FBX**
- ❌ Proprietary format (Autodesk licensing)
- ❌ No native Rust support (requires fbx-sys bindings)
- ❌ Not used in existing AstraWeave assets (zero FBX files found)
- ❌ Blender FBX export quality issues (known problem in community)

---

## Workflow Overview

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐     ┌─────────────┐
│   Blender   │────>│ GLTF Export  │────>│ assets/models/  │────>│ AstraWeave  │
│  Modeling   │     │  (.glb)      │     │   greybox/      │     │  Renderer   │
└─────────────┘     └──────────────┘     └─────────────────┘     └─────────────┘
                            │                      │                      │
                            │                      │                      │
                            v                      v                      v
                    ┌──────────────┐     ┌─────────────────┐     ┌─────────────┐
                    │   Validate   │     │  Scene .ron     │     │  Load &     │
                    │  Dimensions  │     │  Descriptor     │     │  Display    │
                    └──────────────┘     └─────────────────┘     └─────────────┘
```

**Pipeline Stages**:
1. **Modeling**: Create geometry in Blender (or procedural generation)
2. **Export**: Export as GLTF 2.0 (.glb) with proper settings
3. **Validation**: Verify dimensions, materials, collision meshes
4. **Scene Descriptor**: Author .ron file linking mesh to zone data
5. **Runtime**: Load mesh in AstraWeave renderer (wgpu + gltf crate)

---

## Stage 1: Blender Modeling

### Software Requirements

**Blender 4.3+ (Recommended)**:
- Download: https://www.blender.org/download/
- GLTF 2.0 export: Built-in (no plugins)
- Alternatives: Blender 3.6+ also works (any version with GLTF 2.0 exporter)

**Procedural Generation (Fallback)**:
- If Blender unavailable, use Rust procedural mesh generation
- See `examples/procedural_mesh_demo/` (if exists) or create simple cubes/planes via code
- Trade-off: Faster setup, less artistic control

---

### Modeling Guidelines

#### Dimensions
- **Match specification**: Read zone spec (e.g., LOOMSPIRE_GREYBOX_SPEC.md: 50m diameter)
- **Blender units = meters**: 1 Blender unit = 1 meter in AstraWeave
- **Origin at world center**: Set mesh origin to (0, 0, 0) for predictable placement
- **Verify scale**: Use Blender's measurement tools (N panel → Transform → Dimensions)

#### Geometry Complexity
- **Greybox = simple**: Use cubes, cylinders, planes (10-100 triangles per object)
- **Low poly**: Target <1000 triangles per zone (greybox is for layout testing, not final art)
- **Avoid subdivision**: Use flat shading (no smooth normals needed)
- **Collision mesh**: Create separate simplified mesh tagged `_collision` suffix

#### Material Naming
Use standardized material names (AstraWeave will map to placeholder materials):
- `greybox_floor`: Ground planes, walkable surfaces
- `greybox_wall`: Walls, cliffs, vertical barriers
- `greybox_obstacle`: Cover elements, hazards, interactive objects
- `greybox_glass`: Transparent surfaces (optional, future use)

**Material Setup in Blender**:
1. Select mesh → Materials panel
2. Add new material (e.g., "greybox_floor")
3. Set diffuse color: 
   - Floor: Grey (0.5, 0.5, 0.5)
   - Wall: Dark grey (0.3, 0.3, 0.3)
   - Obstacle: Red tint (0.6, 0.3, 0.3)
4. No textures needed (greybox uses solid colors)

#### Collision Meshes
- **Purpose**: Simplified geometry for physics (Rapier3D collision detection)
- **Naming**: Original mesh `loomspire_sanctum`, collision mesh `loomspire_sanctum_collision`
- **Geometry**: Use convex hulls or simplified cubes (1/10th triangle count)
- **Hierarchy**: Place collision mesh as child of visible mesh in Blender scene tree
- **Tag in GLTF**: Collision meshes export as separate nodes, tagged by name suffix

---

### Example: Loomspire Sanctum Modeling Steps

**Specification** (from LOOMSPIRE_GREYBOX_SPEC.md):
- 50m diameter circular structure
- 3 tiers: Ground (50m), Mezzanine (30m), Observation (15m)
- Weaving chamber: 10m × 10m × 10m cube at center

**Blender Steps**:
1. **Ground Floor** (5 minutes):
   - Add → Mesh → Cylinder
   - Scale: X=25, Y=25, Z=2.5 (50m diameter × 5m height)
   - Position: (0, 0, 0)
   - Material: `greybox_floor`

2. **Mezzanine** (3 minutes):
   - Add → Mesh → Cylinder
   - Scale: X=15, Y=15, Z=1.5 (30m diameter × 3m height)
   - Position: (0, 0, 5) (Y offset +5m above ground)
   - Material: `greybox_floor`

3. **Observation Deck** (3 minutes):
   - Add → Mesh → Cylinder
   - Scale: X=7.5, Y=7.5, Z=1 (15m diameter × 2m height)
   - Position: (0, 0, 8) (Y offset +8m)
   - Material: `greybox_floor`

4. **Weaving Chamber** (2 minutes):
   - Add → Mesh → Cube
   - Scale: X=5, Y=5, Z=5 (10m × 10m × 10m)
   - Position: (0, 0, 1) (Y offset +1m above ground)
   - Material: `greybox_obstacle` (distinctive red tint)

5. **Stairs/Ramps** (5 minutes):
   - Add → Mesh → Cube
   - Scale: X=2, Y=5, Z=0.5 (4m wide × 10m long × 1m tall)
   - Rotate: 30° to create ramp
   - Duplicate for 3 connections (ground → mezzanine → observation)
   - Material: `greybox_floor`

6. **Collision Mesh** (5 minutes):
   - Select all visible meshes → Duplicate
   - Join duplicates (Ctrl+J)
   - Rename: `loomspire_sanctum_collision`
   - Simplify: Mesh → Clean Up → Decimate (50% ratio)
   - Hide in viewport (H key)

**Total Time**: ~23 minutes for Loomspire Sanctum

---

## Stage 2: GLTF 2.0 Export

### Export Settings (Blender)

**File → Export → glTF 2.0 (.glb/.gltf)**

#### Include Tab
- ✅ **Limit to**: Selected Objects (if exporting specific zone)
- ✅ **Custom Properties**: Enabled (preserves metadata)

#### Transform Tab
- ✅ **+Y Up**: Enabled (AstraWeave uses Y-up coordinate system)
- ❌ **+Z Up**: Disabled (Unity convention, not used in AstraWeave)

#### Geometry Tab
- ✅ **Apply Modifiers**: Enabled (bake subdivision, mirror, etc.)
- ✅ **UVs**: Enabled (required even if not textured yet)
- ✅ **Normals**: Enabled (flat shading normals)
- ❌ **Tangents**: Disabled (not needed for greybox)
- ❌ **Vertex Colors**: Disabled (materials handle colors)
- ✅ **Materials**: Export
- ✅ **Images**: Embedded (for .glb) OR Automatic (for .gltf)

#### Animation Tab
- ❌ **Animation**: Disabled (greybox is static)
- ❌ **Shape Keys**: Disabled (not used)
- ❌ **Skinning**: Disabled (no characters yet)

#### Compression Tab
- ❌ **Compress**: Disabled (greybox files are small <1 MB, no need)
- Future: Enable for final art (Draco compression, 50-80% size reduction)

#### Format
- **Format**: **glTF Binary (.glb)** ✅
- Rationale: Single file, embedded textures, easier to manage
- Alternative: glTF Separate (.gltf + .bin + .png) for version control (defer)

### Export Command (PowerShell)

If automating export via Blender Python:
```powershell
blender --background --python export_gltf.py -- `
  --input "loomspire_sanctum.blend" `
  --output "assets/models/greybox/loomspire_sanctum_greybox.glb"
```

**export_gltf.py** (automation script, optional):
```python
import bpy
import sys

# Parse args
args = sys.argv[sys.argv.index("--") + 1:]
input_path = args[args.index("--input") + 1]
output_path = args[args.index("--output") + 1]

# Load blend file
bpy.ops.wm.open_mainfile(filepath=input_path)

# Export GLTF
bpy.ops.export_scene.gltf(
    filepath=output_path,
    export_format='GLB',
    export_apply_modifiers=True,
    export_yup=True,
    export_normals=True,
    export_materials='EXPORT',
)

print(f"Exported: {output_path}")
```

---

## Stage 3: Validation

### Automated Validation Checklist

Run this checklist after EVERY export:

#### 1. File Existence (5 seconds)
```powershell
Test-Path "assets/models/greybox/loomspire_sanctum_greybox.glb"
# Expected: True
```

#### 2. File Size (10 seconds)
```powershell
(Get-Item "assets/models/greybox/loomspire_sanctum_greybox.glb").Length / 1MB
# Expected: 0.1 - 2.0 MB (greybox should be small)
# Warning: >5 MB suggests unnecessary textures or high poly count
```

#### 3. GLTF Structure (30 seconds)
Use `gltf-transform` CLI tool (if available):
```bash
gltf-transform inspect assets/models/greybox/loomspire_sanctum_greybox.glb
```

Expected output:
- Primitives: 5-20 (one per mesh object)
- Vertices: 100-1000 (low poly)
- Triangles: 100-1000 (greybox)
- Materials: 2-4 (greybox_floor, greybox_wall, greybox_obstacle)
- Textures: 0-4 (ideally 0 for pure greybox)

#### 4. Dimension Verification (manual, 1 minute)
Load in Blender or use gltf viewer:
- Ground floor: ~50m diameter visible
- Mezzanine: ~30m diameter at Y=+5m
- Weaving chamber: ~10m cube at center
- Total bounds: ~50m × 50m × 15m

#### 5. Material Names (30 seconds)
```powershell
# Extract material names from GLTF
cargo run -p gltf_inspector -- assets/models/greybox/loomspire_sanctum_greybox.glb --materials
```

Expected:
```
Materials:
- greybox_floor (used by: ground, mezzanine, observation)
- greybox_obstacle (used by: weaving_chamber)
```

#### 6. Collision Mesh (30 seconds)
Verify collision mesh exports as separate node:
```powershell
cargo run -p gltf_inspector -- assets/models/greybox/loomspire_sanctum_greybox.glb --nodes | Select-String "collision"
```

Expected:
```
Node: loomspire_sanctum_collision (mesh: 5, children: 0)
```

---

### Manual Validation (In-Engine)

Create `examples/greybox_viewer/` for quick validation:

```rust
// examples/greybox_viewer/src/main.rs
use astraweave_asset::gltf::load_gltf; // If loader exists
use astraweave_render::Renderer;
use winit::event_loop::EventLoop;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let gltf_path = args.get(1).expect("Usage: greybox_viewer <path.glb>");

    // Load GLTF
    let gltf_data = load_gltf(gltf_path).expect("Failed to load GLTF");
    
    // Init renderer
    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop);
    
    // Spawn mesh in scene
    renderer.add_mesh(gltf_data);
    
    // Position camera at spawn point (0, 2, -20)
    renderer.set_camera_pos((0.0, 2.0, -20.0), (0.0, 0.0, 1.0));
    
    // Render loop (1 frame validation or interactive)
    event_loop.run(move |event, _, control_flow| {
        // ... winit event handling
        renderer.render_frame();
    });
}
```

**Run validation**:
```powershell
cargo run -p greybox_viewer --release -- assets/models/greybox/loomspire_sanctum_greybox.glb
```

**Expected Result**:
- Window opens with 3D view
- Loomspire structure visible (50m diameter)
- Camera positioned at (0, 2, -20) facing forward
- Materials render as solid colors (grey, dark grey, red)
- No crashes, no missing textures

---

## Stage 4: Scene Descriptor (.ron)

### Zone Descriptor Schema

Every greybox zone requires a `.ron` file linking the mesh to gameplay data.

**Template**: `templates/zone_descriptor_template.ron`

```ron
(
    // Unique zone identifier (used in code)
    zone_id: "Z0_loomspire_sanctum",
    
    // Path to GLTF mesh (relative to assets/)
    mesh_path: "models/greybox/loomspire_sanctum_greybox.glb",
    
    // Player spawn points (position + facing direction)
    spawn_points: [
        (pos: (0.0, 0.0, -20.0), facing: (0.0, 0.0, 1.0)),
    ],
    
    // Event triggers (dialogue, combat, cinematics)
    triggers: [
        (
            name: "tutorial_start",
            bounds: ((-10.0, -10.0), (10.0, 10.0)), // AABB box
            action: "StartWeavingTutorial",
        ),
    ],
    
    // Weaving anchors (fate-weaving system)
    anchors: [
        (
            id: "loomspire_central_anchor",
            pos: (0.0, 1.0, 0.0),
            stability: 1.0,
            repair_cost: 10,
            max_stability: 1.0,
        ),
    ],
    
    // Navigation mesh (pathfinding, defer to Week 2)
    navigation_mesh: "navmeshes/loomspire_sanctum_navmesh.ron", // Placeholder
    
    // Dialogue nodes (linked to triggers)
    dialogue_nodes: [
        "intro_awakening",
        "tutorial_start",
    ],
    
    // Cinematics (play on zone enter)
    cinematic_triggers: [
        "loom_awakening",
    ],
)
```

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `zone_id` | String | ✅ | Unique identifier (e.g., "Z0_loomspire_sanctum") |
| `mesh_path` | String | ✅ | Path to .glb file (relative to assets/) |
| `spawn_points` | Vec\<SpawnPoint\> | ✅ | Player spawn locations (pos + facing) |
| `triggers` | Vec\<Trigger\> | ⚠️ | Event triggers (optional for Day 4, required for Day 6) |
| `anchors` | Vec\<Anchor\> | ⚠️ | Weaving anchors (optional, link to weaving system) |
| `navigation_mesh` | String | ⚠️ | Navmesh path (placeholder for Week 1, generate in Week 2) |
| `dialogue_nodes` | Vec\<String\> | ⚠️ | Dialogue node IDs from dialogue_intro.toml (Day 6 work) |
| `cinematic_triggers` | Vec\<String\> | ⚠️ | Cinematic IDs to play on zone enter (Day 7 work) |

---

### Example: Loomspire Sanctum .ron

**File**: `assets/cells/Z0_loomspire_sanctum.ron`

```ron
(
    zone_id: "Z0_loomspire_sanctum",
    mesh_path: "models/greybox/loomspire_sanctum_greybox.glb",
    
    spawn_points: [
        // Player spawns at southwest edge, facing center
        (pos: (0.0, 0.0, -20.0), facing: (0.0, 0.0, 1.0)),
    ],
    
    triggers: [
        // Tutorial trigger near center (10m × 10m box)
        (
            name: "tutorial_start",
            bounds: ((-5.0, -5.0), (5.0, 5.0)),
            action: "StartWeavingTutorial",
        ),
    ],
    
    anchors: [
        // Central anchor for tutorial (weaving chamber location)
        (
            id: "loomspire_central_anchor",
            pos: (0.0, 1.0, 0.0), // 1m above ground
            stability: 1.0, // Full stability at start
            repair_cost: 10, // 10 weave energy to repair
            max_stability: 1.0,
        ),
    ],
    
    navigation_mesh: "navmeshes/loomspire_sanctum_navmesh.ron", // TODO: Generate in Week 2
    
    dialogue_nodes: [
        "intro_awakening", // Cinematic A dialogue
        "tutorial_start",  // Tutorial instructions
    ],
    
    cinematic_triggers: [
        "loom_awakening", // Plays on first zone enter
    ],
)
```

---

## Stage 5: Runtime Loading

### Loading Pipeline (Rust)

**High-Level API** (ideal, may need implementation):
```rust
use astraweave_scene::ZoneDescriptor;
use astraweave_asset::gltf::load_gltf;
use astraweave_render::Renderer;

fn load_zone(zone_path: &str, renderer: &mut Renderer) -> Result<()> {
    // 1. Parse .ron descriptor
    let zone_desc = ZoneDescriptor::from_ron(zone_path)?;
    
    // 2. Load GLTF mesh
    let mesh_path = format!("assets/{}", zone_desc.mesh_path);
    let gltf_data = load_gltf(&mesh_path)?;
    
    // 3. Spawn mesh in renderer
    let mesh_id = renderer.add_mesh(gltf_data);
    
    // 4. Set player spawn
    let spawn = &zone_desc.spawn_points[0];
    renderer.set_camera_pos(spawn.pos, spawn.facing);
    
    // 5. Register triggers (Day 6 work)
    for trigger in &zone_desc.triggers {
        register_trigger(trigger); // TODO: Implement
    }
    
    // 6. Create anchors (Day 6 work)
    for anchor in &zone_desc.anchors {
        spawn_anchor(anchor); // TODO: Implement
    }
    
    Ok(())
}
```

**Validation Command**:
```powershell
cargo run -p veilweaver_slice_runtime --release -- --zone assets/cells/Z0_loomspire_sanctum.ron
```

**Expected Output**:
```
[INFO] Loading zone: Z0_loomspire_sanctum
[INFO] Parsing descriptor: assets/cells/Z0_loomspire_sanctum.ron
[INFO] Loading mesh: assets/models/greybox/loomspire_sanctum_greybox.glb
[INFO] Mesh loaded: 342 vertices, 189 triangles, 3 materials
[INFO] Player spawned at: (0.0, 0.0, -20.0)
[INFO] Registered 1 triggers
[INFO] Created 1 anchors
[INFO] Zone ready for playtest
```

---

## Material Mapping

### Greybox Material Definitions

AstraWeave renderer will map GLTF material names to placeholder shaders:

| GLTF Material Name | Shader | Base Color | Roughness | Metallic |
|--------------------|--------|------------|-----------|----------|
| `greybox_floor` | PBR | (0.5, 0.5, 0.5) | 0.8 | 0.0 |
| `greybox_wall` | PBR | (0.3, 0.3, 0.3) | 0.9 | 0.0 |
| `greybox_obstacle` | PBR | (0.6, 0.3, 0.3) | 0.7 | 0.0 |
| `greybox_glass` | PBR | (0.8, 0.8, 0.9) | 0.1 | 0.0 |

**Implementation** (in astraweave-render or astraweave-materials):
```rust
// astraweave-materials/src/greybox.rs
pub fn get_greybox_material(name: &str) -> MaterialPbr {
    match name {
        "greybox_floor" => MaterialPbr {
            base_color: [0.5, 0.5, 0.5, 1.0],
            roughness: 0.8,
            metallic: 0.0,
        },
        "greybox_wall" => MaterialPbr {
            base_color: [0.3, 0.3, 0.3, 1.0],
            roughness: 0.9,
            metallic: 0.0,
        },
        "greybox_obstacle" => MaterialPbr {
            base_color: [0.6, 0.3, 0.3, 1.0], // Red tint
            roughness: 0.7,
            metallic: 0.0,
        },
        "greybox_glass" => MaterialPbr {
            base_color: [0.8, 0.8, 0.9, 0.3], // Semi-transparent
            roughness: 0.1,
            metallic: 0.0,
        },
        _ => MaterialPbr::default(), // Fallback: white
    }
}
```

---

## Troubleshooting

### Issue: "GLTF file won't load"

**Symptoms**: Renderer crashes, "Failed to load GLTF" error

**Causes**:
1. Invalid GLTF structure (corrupt export)
2. Missing textures (if not embedded)
3. Unsupported GLTF feature (e.g., Draco compression)

**Solutions**:
1. Re-export from Blender with default settings
2. Verify .glb file is not corrupted (check file size >0 bytes)
3. Use `gltf-transform inspect` to validate structure
4. Disable Draco compression in export settings

---

### Issue: "Mesh is too small/large"

**Symptoms**: Mesh not visible, or fills entire screen

**Causes**:
1. Wrong scale in Blender (1 unit ≠ 1 meter)
2. Export scale factor wrong

**Solutions**:
1. In Blender: Scene Properties → Units → Unit Scale = 1.0
2. Export settings: Scale = 1.0 (default)
3. Verify dimensions in Blender (N panel → Dimensions shows meters)
4. Re-export with correct settings

---

### Issue: "Materials are white/missing"

**Symptoms**: Mesh loads but all surfaces are white

**Causes**:
1. Material names don't match greybox convention
2. Textures not embedded in .glb
3. Material mapping not implemented in renderer

**Solutions**:
1. Rename materials in Blender to `greybox_floor` etc.
2. Export with "Images: Embedded" setting
3. Implement `get_greybox_material()` function (see Material Mapping section)

---

### Issue: "Collision mesh doesn't work"

**Symptoms**: Player falls through floor, can't collide with walls

**Causes**:
1. Collision mesh not tagged with `_collision` suffix
2. Collision mesh not exported (hidden in Blender)
3. Physics system not configured to use collision mesh

**Solutions**:
1. Rename collision mesh to `<name>_collision` in Blender
2. Unhide collision mesh before export (Alt+H)
3. Verify collision node exists in GLTF (use gltf_inspector)
4. Configure Rapier3D to use collision mesh (Week 2 physics integration)

---

## Performance Considerations

### Target Metrics (Greybox Phase)

| Metric | Target | Rationale |
|--------|--------|-----------|
| **Triangles per zone** | <1000 | Greybox is simple geometry |
| **File size** | <2 MB | No textures, low poly |
| **Load time** | <100 ms | Near-instant zone streaming |
| **Draw calls** | <20 | One per material type |
| **Materials** | 2-4 | Greybox uses 3 base materials |

**Optimization**: Not needed for greybox (defer to final art phase)

**Future**: When transitioning to final art (Weeks 5-6):
- Use Draco compression (50-80% size reduction)
- Texture atlases (1 draw call per zone)
- LOD generation (3-5 levels, astraweave-render/src/lod_generator.rs)
- GPU instancing (100× draw call reduction, astraweave-render/src/instancing.rs)

---

## Procedural Generation Fallback

If Blender is unavailable, use Rust procedural mesh generation:

### Example: Procedural Cube Zone

```rust
// examples/procedural_greybox/src/main.rs
use astraweave_render::mesh::{Mesh, Vertex};

fn create_cube_zone(width: f32, height: f32, depth: f32) -> Mesh {
    let half_w = width / 2.0;
    let half_h = height / 2.0;
    let half_d = depth / 2.0;
    
    // 8 vertices (cube corners)
    let vertices = vec![
        Vertex { pos: [-half_w, -half_h, -half_d], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0] },
        Vertex { pos: [ half_w, -half_h, -half_d], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0] },
        // ... 6 more vertices
    ];
    
    // 12 triangles (2 per face × 6 faces)
    let indices = vec![
        0, 1, 2, 2, 3, 0, // Front face
        4, 5, 6, 6, 7, 4, // Back face
        // ... 4 more faces
    ];
    
    Mesh { vertices, indices, material: "greybox_floor".into() }
}
```

**Usage**:
```rust
let loomspire_ground = create_cube_zone(50.0, 5.0, 50.0); // 50m × 5m × 50m
renderer.add_mesh(loomspire_ground);
```

**Trade-offs**:
- ✅ No Blender dependency
- ✅ Faster iteration (code changes = instant rebuild)
- ❌ Less artistic control (no visual editor)
- ❌ More code to maintain

---

## Appendix A: Quick Reference

### GLTF Export Checklist (Blender)

- [ ] File → Export → glTF 2.0
- [ ] Format: **glTF Binary (.glb)**
- [ ] Transform: **+Y Up** ✅
- [ ] Geometry: **Apply Modifiers** ✅, **UVs** ✅, **Normals** ✅
- [ ] Materials: **Export** ✅
- [ ] Images: **Embedded** ✅ (for .glb)
- [ ] Animation: **Disabled** (greybox is static)
- [ ] Compression: **Disabled** (greybox is small)
- [ ] Save to: `assets/models/greybox/<zone_name>_greybox.glb`

---

### Validation Command Sequence

```powershell
# 1. Verify file exists
Test-Path assets/models/greybox/loomspire_sanctum_greybox.glb

# 2. Check file size
(Get-Item assets/models/greybox/loomspire_sanctum_greybox.glb).Length / 1MB

# 3. Inspect GLTF structure (if gltf-transform installed)
gltf-transform inspect assets/models/greybox/loomspire_sanctum_greybox.glb

# 4. Load in greybox viewer (if example exists)
cargo run -p greybox_viewer --release -- assets/models/greybox/loomspire_sanctum_greybox.glb

# 5. Load full zone (with .ron descriptor)
cargo run -p veilweaver_slice_runtime --release -- --zone assets/cells/Z0_loomspire_sanctum.ron
```

---

### Material Naming Quick Copy

```
greybox_floor       # Ground, walkable surfaces (grey 0.5, 0.5, 0.5)
greybox_wall        # Walls, barriers (dark grey 0.3, 0.3, 0.3)
greybox_obstacle    # Cover, hazards (red tint 0.6, 0.3, 0.3)
greybox_glass       # Transparent (blue-grey 0.8, 0.8, 0.9, alpha 0.3)
```

---

## Appendix B: File Structure

```
assets/
├── models/
│   └── greybox/                           # Greybox meshes
│       ├── loomspire_sanctum_greybox.glb  # Day 4 deliverable
│       ├── echo_grove_greybox.glb         # Day 4 deliverable
│       ├── fractured_cliffs_greybox.glb   # Day 5 deliverable
│       └── test_greybox.glb               # Day 3 validation mesh
├── cells/                                 # Zone descriptors
│   ├── Z0_loomspire_sanctum.ron          # Day 4-6 deliverable
│   ├── Z1_echo_grove.ron                 # Day 4-6 deliverable
│   └── Z2_fractured_cliffs.ron           # Day 5-6 deliverable
└── navmeshes/                            # Navigation meshes (Week 2)
    ├── loomspire_sanctum_navmesh.ron     # Deferred
    ├── echo_grove_navmesh.ron            # Deferred
    └── fractured_cliffs_navmesh.ron      # Deferred

docs/projects/veilweaver/
├── GREYBOX_ASSET_WORKFLOW.md             # This document
├── templates/
│   └── zone_descriptor_template.ron      # Day 3 deliverable
└── WEEK_1_DAYS_3_7_GREYBOX_PLAN.md       # Implementation plan

examples/
└── greybox_viewer/                       # Validation tool (optional)
    └── src/
        └── main.rs                        # Quick mesh viewer
```

---

## Appendix C: Rust Crates Reference

### GLTF Parsing

**Crate**: `gltf = "1.4"`  
**Features**: `["import"]` (enables binary loading)  
**Usage**:
```rust
use gltf::Gltf;

let gltf = Gltf::open("assets/models/greybox/loomspire.glb")?;
for mesh in gltf.meshes() {
    println!("Mesh: {:?}", mesh.name());
}
```

**Documentation**: https://docs.rs/gltf/1.4.0/gltf/

---

### Existing Usage in AstraWeave

| Crate | GLTF Feature | Purpose |
|-------|--------------|---------|
| `astraweave-render` | `gltf-assets = ["gltf", "assets"]` | Optional GLTF loading |
| `astraweave-asset` | `gltf = ["dep:gltf"]` | Asset pipeline |
| `tools/aw_asset_cli` | `gltf = "1"` | Asset CLI tools |
| `examples/unified_showcase` | `gltf = { version = "1.4", features = ["utils"] }` | Demo |
| `examples/nanite_demo` | `features = ["gltf"]` | Nanite mesh streaming |

---

## Appendix D: Timeline Estimates

| Task | Estimated Time | Actual Time (to be filled) |
|------|---------------|----------------------------|
| **Day 3: Asset Pipeline Setup** | 4-6 hours | |
| - Format research | 1 hour | |
| - .ron template creation | 1-2 hours | |
| - Material conventions | 30 minutes | |
| - Workflow documentation | 1-2 hours | |
| - Test mesh validation | 1 hour | |
| **Day 4: Loomspire + Echo Grove** | 6-8 hours | |
| - Loomspire modeling | 3-4 hours | |
| - Echo Grove modeling | 3-4 hours | |
| **Day 5: Fractured Cliffs + Validation** | 5-7 hours | |
| - Fractured Cliffs modeling | 3-4 hours | |
| - Validation & refinement | 2-3 hours | |
| **Day 6: Narrative Integration** | 4-6 hours | |
| **Day 7: Cinematics & Walkthrough** | 4-6 hours | |
| **TOTAL** | 23-33 hours | |

---

**Status**: ✅ APPROVED (Format decision: GLTF 2.0, workflow documented)  
**Next Step**: Create zone descriptor template (Day 3 Task 2)  
**Validation**: Test mesh creation and load validation (Day 3 Task 5)

