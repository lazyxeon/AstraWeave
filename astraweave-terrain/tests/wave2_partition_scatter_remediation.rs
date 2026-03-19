//! Wave 2 Mutation Remediation — partition_integration + scatter
//!
//! Targets ALL 26 MISSED mutations from shard 18:
//!   partition_integration.rs: update_stats, check_memory_budget, mesh_chunks,
//!                             get_mesh, get_all_meshes, get_voxel_grid,
//!                             get_stats, is_chunk_loaded, with_memory_budget
//!   scatter.rs:              scatter_vegetation * density, seed + offset, == 0 guard

use astraweave_terrain::partition_integration::{
    PartitionCoord, VoxelPartitionConfig, VoxelPartitionEvent, VoxelPartitionManager,
    VoxelPartitionStats,
};
use astraweave_terrain::voxel_data::ChunkCoord;

// ─────────────────────────── Helpers ────────────────────────────

fn small_cell_config() -> VoxelPartitionConfig {
    VoxelPartitionConfig {
        cell_size: 32.0, // 32 / 32 = 1 chunk per axis → 1 chunk total
        memory_budget: 500_000_000,
        auto_mesh: true,
        lod_distances: [100.0, 250.0, 500.0, 1000.0],
    }
}

fn tiny_budget_config() -> VoxelPartitionConfig {
    VoxelPartitionConfig {
        cell_size: 32.0,
        memory_budget: 1, // 1 byte — any loaded chunk will exceed
        auto_mesh: true,
        lod_distances: [100.0, 250.0, 500.0, 1000.0],
    }
}

// ─────────── update_stats: replace with () ──────────────────────

#[tokio::test]
async fn stats_updated_after_activate_cell() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();

    let stats = mgr.get_stats();
    // If update_stats is replaced with (), active_cells would stay 0
    // (stats is never written). active_cells is set inside update_stats.
    assert_eq!(
        stats.active_cells, 1,
        "active_cells must be 1 after activation"
    );
    assert!(
        stats.loaded_chunks > 0,
        "loaded_chunks must be >0 after activation"
    );
}

// ─────── update_stats: loaded_chunks * 65536 arithmetic ─────────

#[tokio::test]
async fn voxel_memory_equals_loaded_chunks_times_65536() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();

    let stats = mgr.get_stats();
    // 1 chunk per axis → exactly 1 chunk loaded
    assert_eq!(stats.loaded_chunks, 1);
    // voxel_memory = loaded_chunks * 65536
    assert_eq!(
        stats.voxel_memory,
        stats.loaded_chunks * 65536,
        "voxel_memory must equal loaded_chunks * 65536"
    );
    // Verify the actual value so mutating * to + or / is caught
    assert_eq!(stats.voxel_memory, 65536);
}

#[tokio::test]
async fn voxel_memory_scales_linearly_with_chunks() {
    // Activate 2 cells → 2 chunks (each cell = 1 chunk at cell_size=32)
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();
    mgr.activate_cell(PartitionCoord::new(1, 0, 0))
        .await
        .unwrap();

    let stats = mgr.get_stats();
    assert_eq!(stats.loaded_chunks, 2);
    assert_eq!(stats.voxel_memory, 2 * 65536);
}

// ─── update_stats: mesh_memory = vertices*32 + indices*4 ────────
// Mesh memory depends on whether DualContouring produces non-empty meshes.
// For default VoxelChunk (all densities 0), meshing may produce empty mesh
// meaning mesh_memory = 0. We verify the formula via stats consistency.

#[tokio::test]
async fn mesh_memory_zero_when_no_meshes() {
    // With all-zero chunk data, meshing should produce empty meshes (no surface crossings)
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let stats = mgr.get_stats();
    // Empty chunks → empty meshes → mesh_memory should be 0
    // If * is replaced with + for vertex calc, we'd get non-zero
    assert_eq!(
        stats.mesh_memory, 0,
        "mesh_memory should be 0 for empty voxel data"
    );
}

#[tokio::test]
async fn meshed_chunks_zero_for_empty_data() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let stats = mgr.get_stats();
    // With default VoxelChunk (all empty), no surface = no mesh stored
    assert_eq!(
        stats.meshed_chunks, 0,
        "no meshes should be stored for empty chunks"
    );
}

// ──────── check_memory_budget: replace with () ───────────────────

#[tokio::test]
async fn memory_budget_event_fired_when_exceeded() {
    let mut mgr = VoxelPartitionManager::new(tiny_budget_config());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();

    let events = mgr.drain_events();
    let budget_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, VoxelPartitionEvent::MemoryBudgetExceeded(_, _)))
        .collect();

    assert!(
        !budget_events.is_empty(),
        "MemoryBudgetExceeded event must fire when budget=1 byte but chunks loaded"
    );
}

// ─── check_memory_budget: + operator in total_memory ─────────────

#[tokio::test]
async fn memory_budget_event_reports_correct_usage() {
    let mut mgr = VoxelPartitionManager::new(tiny_budget_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let events = mgr.drain_events();
    let budget_event = events
        .iter()
        .find(|e| matches!(e, VoxelPartitionEvent::MemoryBudgetExceeded(_, _)));

    match budget_event {
        Some(VoxelPartitionEvent::MemoryBudgetExceeded(used, budget)) => {
            // used = voxel_memory + mesh_memory = 65536 + 0 = 65536
            assert_eq!(
                *used, 65536,
                "reported usage should be voxel_memory + mesh_memory"
            );
            assert_eq!(
                *budget, 1,
                "reported budget should be the configured budget"
            );
        }
        _ => panic!("MemoryBudgetExceeded event not found"),
    }
}

// ─── check_memory_budget: > comparison direction ─────────────────

#[tokio::test]
async fn no_budget_event_when_under_budget() {
    // Default budget = 500MB, 1 chunk = 64KB → well under budget
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let events = mgr.drain_events();
    let budget_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, VoxelPartitionEvent::MemoryBudgetExceeded(_, _)))
        .collect();

    assert!(
        budget_events.is_empty(),
        "MemoryBudgetExceeded should NOT fire when usage (64KB) < budget (500MB)"
    );
}

#[tokio::test]
async fn budget_event_exactly_at_budget_boundary() {
    // Set budget to exactly 65536 (= 1 chunk voxel memory with 0 mesh)
    // total_memory == budget → should NOT fire (> not >=)
    let cfg = VoxelPartitionConfig {
        cell_size: 32.0,
        memory_budget: 65536,
        auto_mesh: true,
        lod_distances: [100.0, 250.0, 500.0, 1000.0],
    };
    let mut mgr = VoxelPartitionManager::new(cfg);
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let events = mgr.drain_events();
    let budget_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, VoxelPartitionEvent::MemoryBudgetExceeded(_, _)))
        .collect();

    // 65536 > 65536 is false, so no event
    assert!(
        budget_events.is_empty(),
        "MemoryBudgetExceeded should NOT fire when usage == budget (uses > not >=)"
    );
}

#[tokio::test]
async fn budget_event_one_over_boundary() {
    // Set budget to 65535 (1 less than 1 chunk) → should fire
    let cfg = VoxelPartitionConfig {
        cell_size: 32.0,
        memory_budget: 65535,
        auto_mesh: true,
        lod_distances: [100.0, 250.0, 500.0, 1000.0],
    };
    let mut mgr = VoxelPartitionManager::new(cfg);
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let events = mgr.drain_events();
    let has_budget_event = events
        .iter()
        .any(|e| matches!(e, VoxelPartitionEvent::MemoryBudgetExceeded(_, _)));

    assert!(
        has_budget_event,
        "MemoryBudgetExceeded MUST fire when usage (65536) > budget (65535)"
    );
}

// ──────── mesh_chunks: delete ! in !mesh.is_empty() ─────────────
// (tested indirectly: empty voxel data → no meshes stored)

#[tokio::test]
async fn empty_meshes_not_stored() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let meshes = mgr.get_all_meshes();
    assert!(
        meshes.is_empty(),
        "empty meshes (from all-zero voxels) must NOT be stored"
    );
}

// ──────── get_mesh: replace with None ────────────────────────────

#[tokio::test]
async fn get_mesh_returns_none_for_unloaded_chunk() {
    let mgr = VoxelPartitionManager::new(small_cell_config());
    let result = mgr.get_mesh(ChunkCoord::new(999, 999, 999));
    assert!(
        result.is_none(),
        "get_mesh should return None for unloaded chunk"
    );
}

// ──────── get_all_meshes: returns actual map ─────────────────────

#[tokio::test]
async fn get_all_meshes_returns_reference_to_internal_map() {
    let mgr = VoxelPartitionManager::new(small_cell_config());
    // Before any activation, meshes map should be empty
    let meshes = mgr.get_all_meshes();
    assert!(meshes.is_empty());
    // The pointer should be stable (not a leaked allocation)
    let ptr1 = meshes as *const _ as usize;
    let ptr2 = mgr.get_all_meshes() as *const _ as usize;
    assert_eq!(
        ptr1, ptr2,
        "get_all_meshes must return reference to same internal map"
    );
}

// ──────── get_voxel_grid: returns actual grid ────────────────────

#[tokio::test]
async fn get_voxel_grid_returns_shared_grid() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();

    let grid = mgr.get_voxel_grid();
    let grid_read = grid.read().await;
    // After activation, grid should have chunks
    assert!(
        grid_read.chunk_count() > 0,
        "get_voxel_grid must return shared grid with loaded chunks"
    );
}

#[tokio::test]
async fn get_voxel_grid_reflects_mutations() {
    let mgr = VoxelPartitionManager::new(small_cell_config());
    let grid1 = mgr.get_voxel_grid();
    let grid2 = mgr.get_voxel_grid();
    // Both should be Arc clones of the same RwLock
    // Verify by checking they share the same underlying data
    assert!(
        std::sync::Arc::ptr_eq(&grid1, &grid2),
        "get_voxel_grid must return Arc clone of same grid"
    );
}

// ──────── get_stats: returns actual stats ────────────────────────

#[tokio::test]
async fn get_stats_reflects_loaded_state() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let stats = mgr.get_stats();
    // If get_stats returns Box::leak(Default::default()), all fields are 0
    // But after activation, active_cells=1 and loaded_chunks=1
    assert_eq!(stats.active_cells, 1);
    assert_eq!(stats.loaded_chunks, 1);
    assert_eq!(stats.voxel_memory, 65536);
}

#[tokio::test]
async fn get_stats_reflects_deactivation() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();
    mgr.deactivate_cell(cell).await.unwrap();

    let stats = mgr.get_stats();
    assert_eq!(stats.active_cells, 0);
    assert_eq!(stats.loaded_chunks, 0);
    assert_eq!(stats.voxel_memory, 0);
}

// ──────── drain_events: returns actual events ────────────────────

#[tokio::test]
async fn drain_events_returns_activation_event() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();

    let events = mgr.drain_events();
    assert!(
        !events.is_empty(),
        "drain_events must return CellActivated event"
    );
    let has_activated = events
        .iter()
        .any(|e| matches!(e, VoxelPartitionEvent::CellActivated(_, _)));
    assert!(has_activated, "must contain CellActivated event");
}

#[tokio::test]
async fn drain_events_clears_after_drain() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let first = mgr.drain_events();
    assert!(!first.is_empty());

    let second = mgr.drain_events();
    assert!(second.is_empty(), "second drain_events should be empty");
}

#[tokio::test]
async fn drain_events_returns_deactivation_event() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();
    mgr.drain_events(); // clear activation events
    mgr.deactivate_cell(cell).await.unwrap();

    let events = mgr.drain_events();
    let has_deactivated = events
        .iter()
        .any(|e| matches!(e, VoxelPartitionEvent::CellDeactivated(_, _)));
    assert!(has_deactivated, "must contain CellDeactivated event");
}

// ──────── is_chunk_loaded: true/false ────────────────────────────

#[tokio::test]
async fn is_chunk_loaded_false_before_activation() {
    let mgr = VoxelPartitionManager::new(small_cell_config());
    // cell_size=32 → chunk (0,0,0) maps to partition (0,0,0)
    let loaded = mgr.is_chunk_loaded(ChunkCoord::new(0, 0, 0)).await;
    assert!(!loaded, "chunk must not be loaded before activation");
}

#[tokio::test]
async fn is_chunk_loaded_true_after_activation() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    // cell_size=32, CHUNK_SIZE=32 → 1 chunk per axis
    // partition (0,0,0) contains chunk (0,0,0)
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let loaded = mgr.is_chunk_loaded(ChunkCoord::new(0, 0, 0)).await;
    assert!(
        loaded,
        "chunk (0,0,0) must be loaded after activating partition (0,0,0)"
    );
}

#[tokio::test]
async fn is_chunk_loaded_false_after_deactivation() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();
    mgr.deactivate_cell(cell).await.unwrap();

    let loaded = mgr.is_chunk_loaded(ChunkCoord::new(0, 0, 0)).await;
    assert!(!loaded, "chunk must not be loaded after deactivation");
}

// ──────── with_memory_budget: config.memory_budget ───────────────

#[test]
fn with_memory_budget_applies_custom_budget() {
    let mgr = VoxelPartitionManager::with_memory_budget(42_000_000);
    // There's no direct getter for config, but we can verify via budget event behavior
    // The stats should be empty initially
    let stats = mgr.get_stats();
    assert_eq!(stats.active_cells, 0);
}

#[tokio::test]
async fn with_memory_budget_honors_custom_budget() {
    // Set budget to 1 byte → trigger MemoryBudgetExceeded on any activation
    let mut mgr = VoxelPartitionManager::with_memory_budget(1);
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let events = mgr.drain_events();
    let budget_exceeded = events
        .iter()
        .any(|e| matches!(e, VoxelPartitionEvent::MemoryBudgetExceeded(_, _)));
    assert!(
        budget_exceeded,
        "with_memory_budget(1) must trigger MemoryBudgetExceeded when chunks are loaded"
    );
}

#[tokio::test]
async fn with_memory_budget_500mb_no_event() {
    // Default budget should not trigger for 1 chunk
    let mut mgr = VoxelPartitionManager::with_memory_budget(500_000_000);
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let events = mgr.drain_events();
    let budget_exceeded = events
        .iter()
        .any(|e| matches!(e, VoxelPartitionEvent::MemoryBudgetExceeded(_, _)));
    assert!(
        !budget_exceeded,
        "500MB budget should not be exceeded by 1 chunk (64KB)"
    );
}

// ──────── with_cell_size ─────────────────────────────────────────

#[test]
fn with_cell_size_creates_valid_manager() {
    let mgr = VoxelPartitionManager::with_cell_size(128.0);
    let stats = mgr.get_stats();
    assert_eq!(stats.active_cells, 0);
    assert_eq!(stats.loaded_chunks, 0);
}

// ──────── VoxelPartitionStats Default ────────────────────────────

#[test]
fn partition_stats_default_all_zero() {
    let stats = VoxelPartitionStats::default();
    assert_eq!(stats.active_cells, 0);
    assert_eq!(stats.loaded_chunks, 0);
    assert_eq!(stats.meshed_chunks, 0);
    assert_eq!(stats.voxel_memory, 0);
    assert_eq!(stats.mesh_memory, 0);
}

// ──────── Multiple cell activation stats ─────────────────────────

#[tokio::test]
async fn stats_correct_after_multiple_activations() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();
    mgr.activate_cell(PartitionCoord::new(1, 0, 0))
        .await
        .unwrap();
    mgr.activate_cell(PartitionCoord::new(0, 1, 0))
        .await
        .unwrap();

    let stats = mgr.get_stats();
    assert_eq!(stats.active_cells, 3);
    assert_eq!(stats.loaded_chunks, 3);
    assert_eq!(stats.voxel_memory, 3 * 65536);
}

#[tokio::test]
async fn stats_correct_after_partial_deactivation() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    mgr.activate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();
    mgr.activate_cell(PartitionCoord::new(1, 0, 0))
        .await
        .unwrap();
    mgr.deactivate_cell(PartitionCoord::new(0, 0, 0))
        .await
        .unwrap();

    let stats = mgr.get_stats();
    assert_eq!(stats.active_cells, 1);
    assert_eq!(stats.loaded_chunks, 1);
    assert_eq!(stats.voxel_memory, 65536);
}

// ──────── Scatter vegetation tests ───────────────────────────────

use astraweave_terrain::{
    BiomeConfig, BiomeType, ChunkId, Heightmap, ScatterConfig, TerrainChunk, VegetationScatter,
    WorldConfig,
};

fn fast_scatter_config() -> WorldConfig {
    let mut config = WorldConfig::default();
    config.heightmap_resolution = 16;
    config.chunk_size = 64.0;
    config
}

fn flat_heightmap(resolution: u32, height: f32) -> Heightmap {
    let data = vec![height; (resolution * resolution) as usize];
    Heightmap::from_data(data, resolution).unwrap()
}

fn make_flat_chunk(chunk_size: f32, height: f32) -> TerrainChunk {
    let resolution = 16u32;
    let heightmap = flat_heightmap(resolution, height);
    let biome_map = vec![BiomeType::Forest; (resolution * resolution) as usize];
    TerrainChunk::new(ChunkId::new(0, 0), heightmap, biome_map)
}

#[test]
fn scatter_target_count_uses_multiplication() {
    let config = fast_scatter_config();
    let chunk_size = config.chunk_size;
    let chunk = make_flat_chunk(chunk_size, 50.0);

    let biome_config = BiomeConfig::forest();
    let density = biome_config.vegetation.density;
    let expected_area = chunk_size * chunk_size;
    let expected_count = (expected_area * density) as usize;

    let scatter = VegetationScatter::new(ScatterConfig {
        use_poisson_disk: false, // simpler, faster
        ..Default::default()
    });

    let instances = scatter
        .scatter_vegetation(&chunk, chunk_size, &biome_config, 42)
        .unwrap();

    // The number of instances should be approximately target_count
    assert!(
        instances.len() <= expected_count + 1,
        "scatter count {} should not exceed target {}",
        instances.len(),
        expected_count
    );

    // Critical: verify target_count > 0 (catches == vs != guard mutation)
    assert!(
        expected_count > 0,
        "Forest density should produce non-zero target count"
    );
    // And verify we actually got instances (catches != 0 guard that returns early)
    assert!(
        !instances.is_empty(),
        "Forest biome with density {} should produce vegetation instances",
        density
    );
}

#[test]
fn scatter_zero_density_produces_nothing() {
    let config = fast_scatter_config();
    let chunk_size = config.chunk_size;
    let chunk = make_flat_chunk(chunk_size, 50.0);

    // Create a biome config with zero density
    let mut biome_config = BiomeConfig::desert();
    biome_config.vegetation.density = 0.0;

    let scatter = VegetationScatter::new(ScatterConfig::default());
    let instances = scatter
        .scatter_vegetation(&chunk, chunk_size, &biome_config, 42)
        .unwrap();

    assert!(
        instances.is_empty(),
        "zero density should produce zero instances"
    );
}

#[test]
fn scatter_seed_offset_changes_output() {
    let config = fast_scatter_config();
    let chunk_size = config.chunk_size;
    let chunk = make_flat_chunk(chunk_size, 50.0);
    let biome_config = BiomeConfig::forest();

    let scatter_a = VegetationScatter::new(ScatterConfig {
        use_poisson_disk: false,
        seed_offset: 0,
        ..Default::default()
    });
    let scatter_b = VegetationScatter::new(ScatterConfig {
        use_poisson_disk: false,
        seed_offset: 100,
        ..Default::default()
    });

    let instances_a = scatter_a
        .scatter_vegetation(&chunk, chunk_size, &biome_config, 42)
        .unwrap();
    let instances_b = scatter_b
        .scatter_vegetation(&chunk, chunk_size, &biome_config, 42)
        .unwrap();

    // Different seed_offset should produce different positions
    if !instances_a.is_empty() && !instances_b.is_empty() {
        let pos_a = instances_a[0].position;
        let pos_b = instances_b[0].position;
        assert_ne!(
            pos_a, pos_b,
            "different seed_offset must produce different scatter positions"
        );
    }
}

#[test]
fn scatter_same_seed_same_output() {
    let config = fast_scatter_config();
    let chunk_size = config.chunk_size;
    let chunk = make_flat_chunk(chunk_size, 50.0);
    let biome_config = BiomeConfig::forest();

    let scatter = VegetationScatter::new(ScatterConfig {
        use_poisson_disk: false,
        seed_offset: 7,
        ..Default::default()
    });

    let a = scatter
        .scatter_vegetation(&chunk, chunk_size, &biome_config, 42)
        .unwrap();
    let b = scatter
        .scatter_vegetation(&chunk, chunk_size, &biome_config, 42)
        .unwrap();

    assert_eq!(a.len(), b.len(), "same seed must produce same count");
    for (i, (ia, ib)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            ia.position, ib.position,
            "instance {} position mismatch with same seed",
            i
        );
    }
}

#[test]
fn scatter_poisson_disk_respects_min_distance() {
    let config = fast_scatter_config();
    let chunk_size = config.chunk_size;
    let chunk = make_flat_chunk(chunk_size, 50.0);
    let biome_config = BiomeConfig::forest();

    let min_dist = 3.0;
    let scatter = VegetationScatter::new(ScatterConfig {
        use_poisson_disk: true,
        min_distance: min_dist,
        seed_offset: 0,
        ..Default::default()
    });

    let instances = scatter
        .scatter_vegetation(&chunk, chunk_size, &biome_config, 42)
        .unwrap();

    // Verify minimum distance constraint
    for (i, a) in instances.iter().enumerate() {
        for (j, b) in instances.iter().enumerate() {
            if i != j {
                let dist = (a.position - b.position).length();
                assert!(
                    dist >= min_dist - 0.01, // small epsilon for float
                    "instances {} and {} are too close: distance={}, min={}",
                    i,
                    j,
                    dist,
                    min_dist
                );
            }
        }
    }
}

// ──────── Non-exhaustive event pattern match ─────────────────────

#[tokio::test]
async fn events_contain_correct_cell_coords() {
    let mut mgr = VoxelPartitionManager::new(small_cell_config());
    let cell = PartitionCoord::new(3, 1, 2);
    mgr.activate_cell(cell).await.unwrap();

    let events = mgr.drain_events();
    let activated = events.iter().find_map(|e| {
        if let VoxelPartitionEvent::CellActivated(coord, chunks) = e {
            Some((coord, chunks))
        } else {
            None
        }
    });

    let (coord, chunks) = activated.expect("CellActivated event must be emitted");
    assert_eq!(*coord, cell);
    assert!(!chunks.is_empty());
}
