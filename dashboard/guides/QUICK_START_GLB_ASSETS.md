# Quick Reference: Using GLB Assets in AstraWeave

## 1. Download a GLB Model

**Recommended Source**: Poly Pizza (https://poly.pizza)

```powershell
# Example: Download a tree model
Invoke-WebRequest -Uri "https://poly.pizza/cdn/tree.glb" -OutFile "assets/models/tree.glb"
```

## 2. Update Code (3 locations)

### Location 1: Module Declaration (line ~25)
```rust
mod procedural_textures;
mod gltf_loader;  // ← ADD THIS
```

### Location 2: Tree Loading (line ~1476)
```rust
// REPLACE THIS:
let (tree_template_vertices, tree_template_indices) = create_tree(8.0, 0.6, 10.0, 4.0);

// WITH THIS:
let (tree_template_vertices, tree_template_indices) = match gltf_loader::load_gltf("assets/models/tree.glb") {
    Ok(loaded_mesh) => {
        log::info!("✅ Loaded tree: {} vertices", loaded_mesh.vertices.len());
        let vertices: Vec<Vertex> = loaded_mesh.vertices.iter().map(|v| Vertex {
            position: v.position,
            normal: v.normal,
            uv: v.uv,
        }).collect();
        (vertices, loaded_mesh.indices)
    }
    Err(e) => {
        log::warn!("⚠️ Failed to load tree.glb: {}", e);
        create_tree(8.0, 0.6, 10.0, 4.0)
    }
};
```

### Location 3: Building Loading (OPTIONAL - line ~1528)
```rust
// Same pattern for buildings:
let (building_template_vertices, building_template_indices) = match gltf_loader::load_gltf("assets/models/house.glb") {
    Ok(loaded_mesh) => {
        // Convert vertices...
    }
    Err(_) => create_building(10.0, 8.0, 10.0)
};
```

## 3. Run and Verify

```powershell
cargo run -p unified_showcase --release
```

**Expected Output**:
```
[INFO  unified_showcase::gltf_loader] Attempting to load GLTF: assets/models/tree.glb
[INFO  unified_showcase::gltf_loader] GLTF loaded successfully, parsing meshes...
[INFO  unified_showcase::gltf_loader] Found mesh: 'Tree_01'
[INFO  unified_showcase::gltf_loader] Loaded GLTF mesh 'Tree_01': 1234 vertices, 2000 triangles
[INFO  unified_showcase] ✅ Loaded tree: 1234 vertices
```

## Asset Directory Structure

```
assets/
└── models/
    ├── README.md (full documentation)
    ├── tree.glb
    ├── house.glb
    ├── character.glb
    └── cow.glb
```

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| "Failed to load GLTF file" | File doesn't exist | Check path is correct |
| "No meshes found" | Empty GLTF file | Open in Blender, verify mesh exists |
| Model invisible | Scale mismatch | Multiply positions by scale factor |
| Purple texture | Missing materials | Model doesn't have textures embedded |

## Advanced: Batch Replace All Meshes

```rust
// Define asset paths
let tree_path = "assets/models/nature/pine_tree.glb";
let building_path = "assets/models/buildings/medieval_house.glb";
let npc_path = "assets/models/characters/villager.glb";
let animal_path = "assets/models/animals/cow.glb";

// Load all at startup
let tree_mesh = gltf_loader::load_gltf(tree_path).unwrap_or_else(|_| { /* procedural fallback */ });
let building_mesh = gltf_loader::load_gltf(building_path).unwrap_or_else(|_| { /* fallback */ });
// etc...
```

## Performance Notes

- **Loading**: ~10-50ms per GLB file (one-time at startup)
- **Memory**: 1MB GLB ≈ 10,000 vertices ≈ 480KB GPU memory
- **Instancing**: 1 GLB → 1,000 instances (no extra memory)

## Next Steps

1. Download 5-10 models from Poly Pizza, Quaternius, or Sketchfab
2. Place in `assets/models/`
3. Update file paths in `main_bevy_v2.rs`
4. Rebuild and run
5. Enjoy real 3D assets! 🎉

---

**See**: `assets/models/README.md` for full documentation
**See**: `docs/journey/daily/GLB_ASSET_LOADING_COMPLETE.md` for implementation details
