# AstraWeave Comprehensive Repair Plan
## Post-ECS Refactor + PR #111-113 Feature Closure

**Status**: Ready for Implementation  
**Date**: October 3, 2025  
**Completion Target**: October 10, 2025 (7 work days / 56 hours)

---

## Critical Status Summary

### What's Working ✅
- **Nanite (PR #113)**: 100% complete - GPU culling, Hi-Z occlusion, visibility buffer, 17 tests passing

### What's Broken ⚠️
- **243 proc-macro errors** - Rust Analyzer cache corruption (P0 CRITICAL)
- **World Partition (PR #111)** - 75% complete, async I/O mocked (P1 HIGH) 
- **Voxel/Polygon Hybrid (PR #112)** - 70% complete, Marching Cubes stubbed (P1 HIGH)

### Key Documents
- **PR_111_112_113_GAP_ANALYSIS.md** - Comprehensive 850-line feature audit with implementation plans
- **This Document** - Tactical execution plan with code samples and validation commands

---

## Phase 0: Emergency Fixes (2 hours - START HERE)

**Goal**: Clear IDE errors and verify real compilation status

### Step 1: Clear Rust Analyzer Cache
```powershell
# Remove cached build data
Remove-Item -Recurse -Force $env:USERPROFILE\.cache\rust-analyzer -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force .vscode\.rust-analyzer -ErrorAction SilentlyContinue

# Clean cargo artifacts
cargo clean
```

### Step 2: Verify Real Errors
```powershell
# Check compilation (save output)
cargo check --workspace --all-features 2>&1 | Tee-Object build_output.txt

# Restart Rust Analyzer in VS Code
# Ctrl+Shift+P → "Rust Analyzer: Restart Server"
```

### Step 3: Assessment Decision
Review `build_output.txt`:
- **If clean** → Proceed to Phase 1 (feature gaps)
- **If ECS errors remain** → Fix ECS API alignment first
- **If other errors** → Document and triage

---

## Phase 1: World Partition Async I/O (16 hours)

**Problem**: `astraweave-scene/src/streaming.rs:180-250` mocks async loading

**Current Code** (streaming.rs:200):
```rust
// Simulate async loading (in real implementation, this would load assets)
self.finish_load_cell(coord_clone).await?;  // ❌ Fake immediate load
```

### Task 1.1: Create Cell Loader (4 hours)

Create **`astraweave-asset/src/cell_loader.rs`**:

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    pub coord: GridCoord,
    pub entities: Vec<EntityData>,
    pub assets: Vec<AssetRef>,
    pub static_meshes: Vec<StaticMeshInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    pub archetype: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub components: HashMap<String, ron::Value>,
}

/// Load cell data from RON file.
pub async fn load_cell_from_ron(cell_path: &Path) -> Result<CellData> {
    let contents = fs::read_to_string(cell_path).await
        .with_context(|| format!("Failed to read {}", cell_path.display()))?;
    let data: CellData = ron::from_str(&contents)?;
    Ok(data)
}
```

**Add to `astraweave-asset/Cargo.toml`**:
```toml
[dependencies]
tokio = { workspace = true, features = ["fs"] }
ron = { workspace = true }
```

### Task 1.2: Update Streaming Manager (6 hours)

**Replace mocked implementation** in `astraweave-scene/src/streaming.rs`:

```rust
use astraweave_asset::cell_loader;

impl WorldPartitionManager {
    async fn start_load_cell(&mut self, coord: GridCoord) -> Result<()> {
        self.emit_event(StreamingEvent::CellLoadStarted(coord));

        let cell_path = self.config.asset_root
            .join(format!("cells/cell_{}_{}_{}. ron", coord.x, coord.y, coord.z));

        let partition = Arc::clone(&self.partition);
        let event_sender = self.event_sender.clone();
        
        tokio::spawn(async move {
            match cell_loader::load_cell_from_ron(&cell_path).await {
                Ok(cell_data) => {
                    let mut part = partition.write().await;
                    if let Some(cell) = part.get_cell_mut(coord) {
                        cell.state = CellState::Loaded;
                        cell.data = Some(cell_data.clone());
                        cell.memory_bytes = estimate_memory_usage(&cell_data);
                    }
                    let _ = event_sender.send(StreamingEvent::CellLoadCompleted(coord, cell_data));
                }
                Err(e) => {
                    let _ = event_sender.send(StreamingEvent::CellLoadFailed(coord, e.to_string()));
                }
            }
        });

        Ok(())
    }
}

fn estimate_memory_usage(cell_data: &CellData) -> usize {
    cell_data.entities.len() * 1024 + cell_data.static_meshes.len() * 100 * 1024
}
```

### Task 1.3: Create Sample Assets (2 hours)

**`assets/cells/cell_0_0_0.ron`**:
```ron
(
    coord: (x: 0, y: 0, z: 0),
    entities: [
        (
            archetype: "tree_oak",
            position: [10.0, 0.0, 10.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.2, 1.0],
            components: {},
        ),
    ],
    assets: [
        (path: "meshes/tree_oak.obj", asset_type: Mesh),
    ],
    static_meshes: [],
)
```

### Task 1.4: Integration Tests (4 hours)

**`astraweave-scene/tests/streaming_integration.rs`**:

```rust
#[tokio::test]
async fn test_async_cell_loading() {
    let test_dir = tempfile::tempdir().unwrap();
    let cell_path = test_dir.path().join("cells/cell_0_0_0.ron");
    
    // Create test cell
    let test_cell = CellData {
        coord: GridCoord { x: 0, y: 0, z: 0 },
        entities: vec![/* ... */],
        assets: vec![],
        static_meshes: vec![],
    };
    
    cell_loader::save_cell_to_ron(&cell_path, &test_cell).await.unwrap();
    
    // Load via manager
    let mut config = StreamingConfig::default();
    config.asset_root = test_dir.path().to_path_buf();
    let mut manager = WorldPartitionManager::new(config);
    
    manager.force_load_cell(GridCoord::new(0, 0, 0)).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    let cell = manager.get_cell(GridCoord::new(0, 0, 0)).unwrap();
    assert_eq!(cell.state, CellState::Loaded);
}

#[tokio::test]
async fn test_memory_budget() {
    let config = StreamingConfig {
        max_memory_bytes: 10 * 1024 * 1024,
        ..Default::default()
    };
    let mut manager = WorldPartitionManager::new(config);
    
    // Request 400 cells
    for x in 0..20 {
        for z in 0..20 {
            let _ = manager.request_cell_load(GridCoord::new(x, 0, z)).await;
        }
    }
    
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    let stats = manager.get_metrics();
    assert!(stats.total_memory_bytes <= 10 * 1024 * 1024);
}
```

### Validation
```powershell
cargo test -p astraweave-scene --test streaming_integration
cargo run --example world_partition_demo --release -- --profile-memory
```

---

## Phase 2: Voxel Marching Cubes (12 hours)

**Problem**: `astraweave-terrain/src/meshing.rs:220-240` generates "simple quad"

### Task 2.1: Create MC Tables (2 hours)

Create **`astraweave-terrain/src/marching_cubes_tables.rs`**:

```rust
/// Edge flags for 256 cube configurations
pub const MC_EDGE_TABLE: [u16; 256] = [
    0x000, 0x109, 0x203, 0x30a, 0x406, 0x50f, 0x605, 0x70c,
    0x80c, 0x905, 0xa0f, 0xb06, 0xc0a, 0xd03, 0xe09, 0xf00,
    // ... (complete 256 entries from PR_111_112_113_GAP_ANALYSIS.md)
];

/// Triangle indices for 256 configurations (up to 5 triangles each)
pub const MC_TRI_TABLE: [[i8; 16]; 256] = [
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [0, 8, 3, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    // ... (complete 256×16 table from gap analysis)
];

pub const EDGE_VERTICES: [[usize; 2]; 12] = [
    [0, 1], [1, 2], [2, 3], [3, 0],
    [4, 5], [5, 6], [6, 7], [7, 4],
    [0, 4], [1, 5], [2, 6], [3, 7],
];

pub const CUBE_VERTICES: [[f32; 3]; 8] = [
    [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0],
];
```

> **Note**: Full tables (856 entries) available in `PR_111_112_113_GAP_ANALYSIS.md`

### Task 2.2: Implement MC Algorithm (6 hours)

**Update `astraweave-terrain/src/meshing.rs`**:

```rust
use crate::marching_cubes_tables::*;
use std::collections::HashMap;

impl DualContouring {
    pub fn generate_mesh_marching_cubes(&mut self, chunk: &VoxelChunk) -> ChunkMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut edge_cache: HashMap<(IVec3, usize), u32> = HashMap::new();
        
        for z in 0..31 {
            for y in 0..31 {
                for x in 0..31 {
                    self.process_cell_mc(
                        chunk,
                        IVec3::new(x, y, z),
                        &mut vertices,
                        &mut indices,
                        &mut edge_cache,
                    );
                }
            }
        }
        
        let normals = self.compute_normals(&vertices, &indices);
        ChunkMesh { vertices, indices, normals }
    }
    
    fn process_cell_mc(
        &self,
        chunk: &VoxelChunk,
        cell_pos: IVec3,
        vertices: &mut Vec<Vec3>,
        indices: &mut Vec<u32>,
        edge_cache: &mut HashMap<(IVec3, usize), u32>,
    ) {
        // 1. Sample 8 corners
        let mut cube_values = [0.0f32; 8];
        for i in 0..8 {
            let corner = cell_pos + IVec3::from_array([
                CUBE_VERTICES[i][0] as i32,
                CUBE_VERTICES[i][1] as i32,
                CUBE_VERTICES[i][2] as i32,
            ]);
            cube_values[i] = chunk.get_voxel(corner).density;
        }
        
        // 2. Compute config (0-255)
        let mut config = 0u8;
        for i in 0..8 {
            if cube_values[i] < 0.0 { config |= 1 << i; }
        }
        
        if config == 0 || config == 255 { return; }
        
        // 3. Get edge flags
        let edge_flags = MC_EDGE_TABLE[config as usize];
        
        // 4. Compute edge vertices (cached)
        let mut edge_vertices = [0u32; 12];
        for edge in 0..12 {
            if edge_flags & (1 << edge) != 0 {
                let cache_key = (cell_pos, edge);
                let vertex_index = edge_cache.get(&cache_key).copied().unwrap_or_else(|| {
                    let v0_idx = EDGE_VERTICES[edge][0];
                    let v1_idx = EDGE_VERTICES[edge][1];
                    let v0 = cell_pos.as_vec3() + Vec3::from_array(CUBE_VERTICES[v0_idx]);
                    let v1 = cell_pos.as_vec3() + Vec3::from_array(CUBE_VERTICES[v1_idx]);
                    let d0 = cube_values[v0_idx];
                    let d1 = cube_values[v1_idx];
                    let t = -d0 / (d1 - d0);
                    let vertex = v0.lerp(v1, t.clamp(0.0, 1.0));
                    let idx = vertices.len() as u32;
                    vertices.push(vertex);
                    edge_cache.insert(cache_key, idx);
                    idx
                });
                edge_vertices[edge] = vertex_index;
            }
        }
        
        // 5. Generate triangles
        let tri_config = MC_TRI_TABLE[config as usize];
        for i in (0..16).step_by(3) {
            if tri_config[i] == -1 { break; }
            indices.push(edge_vertices[tri_config[i] as usize]);
            indices.push(edge_vertices[tri_config[i + 1] as usize]);
            indices.push(edge_vertices[tri_config[i + 2] as usize]);
        }
    }
}
```

### Task 2.3: Add Parallel Meshing (2 hours)

```rust
use rayon::prelude::*;

impl DualContouring {
    pub fn generate_mesh_parallel(&mut self, chunk: &VoxelChunk) -> ChunkMesh {
        const SUB_SIZE: usize = 8;
        let divisions = 32 / SUB_SIZE;
        
        let sub_meshes: Vec<ChunkMesh> = (0..divisions)
            .into_par_iter()
            .flat_map(|sz| (0..divisions).into_par_iter()
                .flat_map(move |sy| (0..divisions).into_par_iter()
                    .map(move |sx| self.generate_sub_mesh(
                        chunk,
                        IVec3::new((sx * SUB_SIZE) as i32, (sy * SUB_SIZE) as i32, (sz * SUB_SIZE) as i32),
                        SUB_SIZE
                    ))
                )
            )
            .collect();
        
        self.merge_meshes(sub_meshes)
    }
}
```

### Task 2.4: Comprehensive Tests (2 hours)

**`astraweave-terrain/tests/marching_cubes_tests.rs`**:

```rust
#[test]
fn test_all_256_marching_cubes_configs() {
    for config in 0..256 {
        let mesh = generate_test_mesh_for_config(config as u8);
        assert_eq!(mesh.indices.len() % 3, 0);
        if !mesh.indices.is_empty() {
            assert!(is_watertight(&mesh), "Config {} not watertight", config);
        }
    }
}

#[test]
fn test_sphere_mesh_watertight() {
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let center = Vec3::new(16.0, 16.0, 16.0);
    let radius = 8.0;
    
    for z in 0..32 {
        for y in 0..32 {
            for x in 0..32 {
                let pos = Vec3::new(x as f32, y as f32, z as f32);
                let dist = (pos - center).length() - radius;
                chunk.set_voxel(IVec3::new(x, y, z), Voxel { density: dist });
            }
        }
    }
    
    let mut mesher = DualContouring::new();
    let mesh = mesher.generate_mesh_marching_cubes(&chunk);
    assert!(is_watertight(&mesh));
    assert!(mesh.vertices.len() > 100);
}

fn is_watertight(mesh: &ChunkMesh) -> bool {
    let mut edge_counts: HashMap<(u32, u32), usize> = HashMap::new();
    for tri in mesh.indices.chunks_exact(3) {
        for i in 0..3 {
            let v0 = tri[i];
            let v1 = tri[(i + 1) % 3];
            let edge = (v0.min(v1), v0.max(v1));
            *edge_counts.entry(edge).or_insert(0) += 1;
        }
    }
    edge_counts.values().all(|&count| count == 2)
}
```

### Validation
```powershell
cargo test -p astraweave-terrain --test marching_cubes_tests
cargo run --example hybrid_voxel_demo --release -- --test-deformation
```

---

## Phase 3: Polish & Examples (6 hours)

### Task 3.1: Fix unified_showcase (4 hours)

**Issue 1**: Add missing dependency to `astraweave-render/Cargo.toml`:
```toml
[dependencies]
toml = { workspace = true }
```

**Issue 2**: Fix module path in `astraweave-render/src/material.rs` line ~155:
```rust
// Before:
super::material_loader_impl::build_arrays(...)

// After:
crate::material_loader::material_loader_impl::build_arrays(...)
```

**Issue 3**: Restore `pattern_noise` in `unified_showcase/src/main.rs` lines 1108-1120:
```rust
fn pattern_noise(seed: u32, x: u32, y: u32) -> u8 {
    let mut value = seed.wrapping_mul(374_761_393).wrapping_add(x)
        .wrapping_mul(668_265_263).wrapping_add(y);
    value = value.wrapping_mul(value.wrapping_add(374_761_397));
    ((value >> 24) & 0xFF) as u8
}
```

**Issue 4**: Remove duplicate `upload_material_layers_from_library` at line 105

### Task 3.2: Update Documentation (2 hours)

Update `PR_111_112_113_GAP_ANALYSIS.md` status:
- World Partition: 75% → 100%
- Voxel/Polygon Hybrid: 70% → 100%
- Nanite: 100% (unchanged)

---

## Quality Gates

### Gate 1: Compilation ✅
```powershell
cargo build --workspace --all-features --release
```
**Success Criteria**: 0 errors, 0 warnings

### Gate 2: Testing ✅
```powershell
cargo test --workspace --all-features
```
**Success Criteria**: 100% pass rate

### Gate 3: Linting ✅
```powershell
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --workspace --check
```
**Success Criteria**: 0 warnings, all formatted

### Gate 4: Examples ✅
```powershell
cargo run --example world_partition_demo --release
cargo run --example hybrid_voxel_demo --release
cargo run --example nanite_demo --release
cargo run --example unified_showcase --release
```
**Success Criteria**: All run without crashes

---

## File Checklist

### Files to Create
- [ ] `astraweave-asset/src/cell_loader.rs`
- [ ] `astraweave-terrain/src/marching_cubes_tables.rs`
- [ ] `astraweave-scene/tests/streaming_integration.rs`
- [ ] `astraweave-terrain/tests/marching_cubes_tests.rs`
- [ ] `assets/cells/cell_0_0_0.ron` (+ 2 more)

### Files to Modify
- [ ] `astraweave-scene/src/streaming.rs` (lines 180-250)
- [ ] `astraweave-terrain/src/meshing.rs` (lines 220-300)
- [ ] `astraweave-render/src/material.rs` (line ~155)
- [ ] `astraweave-render/Cargo.toml`
- [ ] `examples/unified_showcase/src/main.rs` (lines 105, 1108-1120)

---

## Commands Quick Reference

### Emergency (Phase 0)
```powershell
Remove-Item -Recurse -Force $env:USERPROFILE\.cache\rust-analyzer -ErrorAction SilentlyContinue
cargo clean
cargo check --workspace --all-features 2>&1 | Tee-Object build_output.txt
```

### Build & Test
```powershell
cargo build --workspace --all-features --release
cargo test --workspace --all-features
cargo test -p astraweave-scene --test streaming_integration
cargo test -p astraweave-terrain --test marching_cubes_tests
```

### Quality
```powershell
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --workspace --check
cargo audit
```

### Examples
```powershell
cargo run --example world_partition_demo --release -- --profile-memory
cargo run --example hybrid_voxel_demo --release -- --test-deformation
cargo run --example unified_showcase --release
```

---

## Success Criteria Summary

### Critical (Must Have) ✅
- [ ] All proc-macro errors resolved
- [ ] World Partition async I/O complete (real tokio tasks)
- [ ] Voxel Marching Cubes complete (full 256-config tables)
- [ ] All tests passing
- [ ] Zero clippy warnings

### High Priority (Should Have) ✅
- [ ] Memory budget enforcement (<500MB)
- [ ] unified_showcase fixed
- [ ] Documentation updated

---

**Status**: ✅ Ready for Implementation  
**Estimated Completion**: October 10, 2025 (56 hours total)  
**Priority**: Execute Phase 0 immediately, then Phases 1-3 sequentially

**Next Action**: Run Phase 0 emergency fixes to clear IDE errors and assess real compilation state.
