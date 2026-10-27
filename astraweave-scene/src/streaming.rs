//! Streaming Manager for World Partition
//!
//! This module handles async loading and unloading of cells based on camera position.
//! It uses tokio for async operations and maintains an LRU cache of recently unloaded cells.

use crate::world_partition::{CellEntityBlueprint, CellState, GridCoord, LRUCache, WorldPartition};
use anyhow::Result;
use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use tokio::sync::RwLock;

struct LoadFailure {
    coord: GridCoord,
    error: String,
}

/// Events emitted by the streaming system
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum StreamingEvent {
    CellLoadStarted(GridCoord),
    CellLoaded(GridCoord),
    CellLoadFailed(GridCoord, String),
    CellUnloadStarted(GridCoord),
    CellUnloaded(GridCoord),
}

/// Streaming configuration
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum number of cells to keep loaded at once
    pub max_active_cells: usize,
    /// Number of cells to keep in LRU cache (avoid immediate reload)
    pub lru_cache_size: usize,
    /// Radius around camera to load cells (in world units)
    pub streaming_radius: f32,
    /// Maximum concurrent loading tasks
    pub max_concurrent_loads: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_active_cells: 25, // 5x5 grid around camera
            lru_cache_size: 5,
            streaming_radius: 500.0, // 500 meters
            max_concurrent_loads: 4,
        }
    }
}

/// Metrics for monitoring streaming performance
#[derive(Debug, Clone, Default)]
pub struct StreamingMetrics {
    pub active_cells: usize,
    pub loading_cells: usize,
    pub loaded_cells: usize,
    pub cached_cells: usize,
    pub memory_usage_bytes: usize,
    pub total_loads: u64,
    pub total_unloads: u64,
    pub failed_loads: u64,
}

/// World Partition Manager - handles streaming logic
pub struct WorldPartitionManager {
    partition: Arc<RwLock<WorldPartition>>,
    config: StreamingConfig,
    lru_cache: LRUCache,
    active_cells: HashSet<GridCoord>,
    loading_cells: HashSet<GridCoord>,
    metrics: StreamingMetrics,
    event_listeners: Vec<Box<dyn Fn(StreamingEvent) + Send + Sync>>,
    load_failure_tx: Sender<LoadFailure>,
    load_failures: Receiver<LoadFailure>,
}

impl WorldPartitionManager {
    pub fn new(partition: Arc<RwLock<WorldPartition>>, config: StreamingConfig) -> Self {
        let (load_failure_tx, load_failures) = mpsc::channel();
        Self {
            partition,
            lru_cache: LRUCache::new(config.lru_cache_size),
            config,
            active_cells: HashSet::new(),
            loading_cells: HashSet::new(),
            metrics: StreamingMetrics::default(),
            event_listeners: Vec::new(),
            load_failure_tx,
            load_failures,
        }
    }

    /// Add event listener
    pub fn add_event_listener<F>(&mut self, listener: F)
    where
        F: Fn(StreamingEvent) + Send + Sync + 'static,
    {
        self.event_listeners.push(Box::new(listener));
    }

    /// Emit event to all listeners
    fn emit_event(&self, event: StreamingEvent) {
        for listener in &self.event_listeners {
            listener(event.clone());
        }
    }

    /// Update streaming based on camera position
    pub async fn update(&mut self, camera_position: glam::Vec3) -> Result<()> {
        self.take_load_failures().await;
        let partition = self.partition.read().await;

        // Determine which cells should be active based on camera position
        let desired_cells =
            partition.cells_in_radius(camera_position, self.config.streaming_radius);

        drop(partition); // Release read lock

        // Cells to load (in desired but not active/loading)
        let to_load: Vec<GridCoord> = desired_cells
            .iter()
            .filter(|coord| {
                !self.active_cells.contains(coord) && !self.loading_cells.contains(coord)
            })
            .copied()
            .collect();

        // Cells to unload (in active but not desired)
        let to_unload: Vec<GridCoord> = self
            .active_cells
            .iter()
            .filter(|coord| !desired_cells.contains(coord))
            .copied()
            .collect();

        // Start loading new cells (respecting max concurrent loads)
        let available_slots = self
            .config
            .max_concurrent_loads
            .saturating_sub(self.loading_cells.len());
        for coord in to_load.into_iter().take(available_slots) {
            self.start_load_cell(coord).await?;
        }

        // Unload cells that are out of range
        for coord in to_unload {
            self.unload_cell(coord).await?;
        }

        // Update metrics
        self.update_metrics().await;

        Ok(())
    }

    /// Start loading a cell asynchronously
    async fn start_load_cell(&mut self, coord: GridCoord) -> Result<()> {
        // Check if in LRU cache (quick reload)
        if self.lru_cache.contains(coord) {
            self.lru_cache.remove(coord);
            self.active_cells.insert(coord);

            let mut partition = self.partition.write().await;
            if let Some(cell) = partition.get_cell_mut(coord) {
                cell.state = CellState::Loaded;
            }

            self.emit_event(StreamingEvent::CellLoaded(coord));
            self.metrics.total_loads += 1;
            return Ok(());
        }

        // Mark as loading
        self.loading_cells.insert(coord);

        {
            let mut partition = self.partition.write().await;
            let cell = partition.get_or_create_cell(coord);
            cell.state = CellState::Loading;
        }

        self.emit_event(StreamingEvent::CellLoadStarted(coord));

        // Spawn actual async loading task
        let partition_clone = Arc::clone(&self.partition);
        let failure_tx = self.load_failure_tx.clone();
        let coord_clone = coord;

        tokio::spawn(async move {
            // Construct cell file path
            let cell_path = std::path::PathBuf::from(format!(
                "assets/cells/{}_{}_{}.ron",
                coord_clone.x, coord_clone.y, coord_clone.z
            ));

            // Attempt to load cell data from RON file
            match Self::load_cell_data(&cell_path).await {
                Ok(cell_data) => {
                    // Load referenced assets
                    let assets_root = std::path::Path::new("assets");
                    for asset_ref in &cell_data.assets {
                        // Bytes are not stored. A failed read, including a
                        // texture over the cap, must not mark the cell loaded.
                        // The manager applies the failure on its next update.
                        if let Err(err) = Self::load_asset_data(asset_ref, assets_root).await {
                            let _ = failure_tx.send(LoadFailure {
                                coord: coord_clone,
                                error: format!("{err:#}"),
                            });
                            return Err(err);
                        }
                    }

                    let entity_blueprints: Vec<CellEntityBlueprint> = cell_data
                        .entities
                        .iter()
                        .map(|entity| CellEntityBlueprint {
                            name: entity.name.clone(),
                            position: entity.position,
                            rotation: entity.rotation,
                            scale: entity.scale,
                            components: entity.components.clone(),
                        })
                        .collect();
                    let asset_refs = cell_data.assets.clone();
                    let metadata = cell_data.metadata.clone();

                    // Update cell state
                    let mut partition = partition_clone.write().await;
                    if let Some(cell) = partition.get_cell_mut(coord_clone) {
                        cell.state = CellState::Loaded;
                        cell.entity_blueprints = entity_blueprints;
                        cell.metadata = metadata;
                        cell.assets.clear();
                        // Store entity/asset data in cell
                        // Note: Convert cell_data.entities to entity IDs via ECS integration
                        for asset in asset_refs {
                            cell.assets.push(crate::world_partition::AssetRef {
                                path: asset.path,
                                asset_type: match asset.kind {
                                    astraweave_asset::cell_loader::AssetKind::Mesh => {
                                        crate::world_partition::AssetType::Mesh
                                    }
                                    astraweave_asset::cell_loader::AssetKind::Texture => {
                                        crate::world_partition::AssetType::Texture
                                    }
                                    astraweave_asset::cell_loader::AssetKind::Material => {
                                        crate::world_partition::AssetType::Material
                                    }
                                    astraweave_asset::cell_loader::AssetKind::Audio => {
                                        crate::world_partition::AssetType::Audio
                                    }
                                    _ => crate::world_partition::AssetType::Other,
                                },
                            });
                        }
                    }

                    Ok::<(), anyhow::Error>(())
                }
                Err(e) => {
                    let _ = failure_tx.send(LoadFailure {
                        coord: coord_clone,
                        error: format!("{e:#}"),
                    });
                    Err(e)
                }
            }
        });

        // The spawned task will handle updating cell state asynchronously
        // Loading will complete in the background; check cell state later via partition.get_cell()
        Ok(())
    }

    /// Load cell data from RON file (helper for async task)
    async fn load_cell_data(
        cell_path: &std::path::Path,
    ) -> Result<astraweave_asset::cell_loader::CellData> {
        astraweave_asset::cell_loader::load_cell_from_ron(cell_path).await
    }

    /// Load asset data (helper for async task)
    async fn load_asset_data(
        asset_ref: &astraweave_asset::cell_loader::AssetRef,
        assets_root: &std::path::Path,
    ) -> Result<Vec<u8>> {
        astraweave_asset::cell_loader::load_asset(asset_ref, assets_root).await
    }

    /// Apply load failures the spawned task reported.
    ///
    /// The task cannot touch `loading_cells`. Leaving a failed coord there
    /// holds a `max_concurrent_loads` slot until the manager is rebuilt.
    async fn take_load_failures(&mut self) {
        while let Ok(failure) = self.load_failures.try_recv() {
            self.handle_load_failure(failure.coord, failure.error).await;
        }
    }

    /// Finish loading a cell (called after async load completes)
    #[allow(dead_code)]
    async fn finish_load_cell(&mut self, coord: GridCoord) -> Result<()> {
        self.loading_cells.remove(&coord);
        self.active_cells.insert(coord);

        let mut partition = self.partition.write().await;
        if let Some(cell) = partition.get_cell_mut(coord) {
            cell.state = CellState::Loaded;
        }

        self.emit_event(StreamingEvent::CellLoaded(coord));
        self.metrics.total_loads += 1;

        Ok(())
    }

    /// Handle load failure
    async fn handle_load_failure(&mut self, coord: GridCoord, error: String) {
        self.loading_cells.remove(&coord);

        let mut partition = self.partition.write().await;
        if let Some(cell) = partition.get_cell_mut(coord) {
            cell.state = CellState::Unloaded;
        }

        self.emit_event(StreamingEvent::CellLoadFailed(coord, error));
        self.metrics.failed_loads += 1;
    }

    /// Unload a cell
    async fn unload_cell(&mut self, coord: GridCoord) -> Result<()> {
        if !self.active_cells.contains(&coord) {
            return Ok(());
        }

        self.emit_event(StreamingEvent::CellUnloadStarted(coord));

        {
            let mut partition = self.partition.write().await;
            if let Some(cell) = partition.get_cell_mut(coord) {
                cell.state = CellState::Unloading;
            }
        }

        // Perform unload (in real implementation, release GPU resources, etc.)
        // For now, just mark as unloaded
        {
            let mut partition = self.partition.write().await;
            if let Some(cell) = partition.get_cell_mut(coord) {
                cell.state = CellState::Unloaded;
            }
        }

        self.active_cells.remove(&coord);
        self.lru_cache.touch(coord);

        self.emit_event(StreamingEvent::CellUnloaded(coord));
        self.metrics.total_unloads += 1;

        Ok(())
    }

    /// Update metrics
    async fn update_metrics(&mut self) {
        let partition = self.partition.read().await;

        self.metrics.active_cells = self.active_cells.len();
        self.metrics.loading_cells = self.loading_cells.len();
        self.metrics.loaded_cells = partition.loaded_cells().len();
        self.metrics.cached_cells = self.lru_cache.len();
        self.metrics.memory_usage_bytes = partition.memory_usage_estimate();
    }

    /// Get current metrics
    pub fn metrics(&self) -> &StreamingMetrics {
        &self.metrics
    }

    /// Force load a specific cell
    pub async fn force_load_cell(&mut self, coord: GridCoord) -> Result<()> {
        if self.active_cells.contains(&coord) {
            return Ok(());
        }
        self.start_load_cell(coord).await
    }

    /// Force unload a specific cell
    pub async fn force_unload_cell(&mut self, coord: GridCoord) -> Result<()> {
        self.unload_cell(coord).await
    }

    /// Get list of active cells
    pub fn active_cells(&self) -> Vec<GridCoord> {
        self.active_cells.iter().copied().collect()
    }

    /// Check if cell is active
    pub fn is_cell_active(&self, coord: GridCoord) -> bool {
        self.active_cells.contains(&coord)
    }

    /// Check if cell is loading
    pub fn is_cell_loading(&self, coord: GridCoord) -> bool {
        self.loading_cells.contains(&coord)
    }
}

/// Helper function to create a streaming manager with default config
pub fn create_streaming_manager(partition: Arc<RwLock<WorldPartition>>) -> WorldPartitionManager {
    WorldPartitionManager::new(partition, StreamingConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_partition::{CellState, GridConfig, GridCoord, WorldPartition};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_streaming_config_default_sane() {
        let cfg = StreamingConfig::default();
        assert!(cfg.max_active_cells > 0);
        assert!(cfg.lru_cache_size > 0);
        assert!(cfg.streaming_radius > 0.0);
        assert!(cfg.max_concurrent_loads > 0);
    }

    #[tokio::test]
    async fn test_force_load_cell_uses_lru_fast_path() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let cfg = StreamingConfig {
            streaming_radius: 0.1,
            max_concurrent_loads: 1,
            ..Default::default()
        };

        let mut mgr = WorldPartitionManager::new(Arc::clone(&partition), cfg);
        let coord = GridCoord::new(0, 0, 0);

        // Seed LRU to force the fast path (avoids spawning async file loads).
        mgr.lru_cache.touch(coord);

        {
            let mut p = partition.write().await;
            let cell = p.get_or_create_cell(coord);
            cell.state = CellState::Unloaded;
        }

        mgr.force_load_cell(coord).await.unwrap();
        assert!(mgr.is_cell_active(coord));
        assert!(!mgr.is_cell_loading(coord));
        assert_eq!(mgr.metrics.total_loads, 1);

        let p = partition.read().await;
        assert_eq!(p.get_cell(coord).unwrap().state, CellState::Loaded);
    }

    #[tokio::test]
    async fn test_update_unloads_active_cells_out_of_range_and_emits_events() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let cfg = StreamingConfig {
            streaming_radius: 0.1,
            max_concurrent_loads: 1,
            ..Default::default()
        };

        let mut mgr = WorldPartitionManager::new(Arc::clone(&partition), cfg);

        // Capture events.
        let events: Arc<Mutex<Vec<StreamingEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);
        mgr.add_event_listener(move |e| {
            events_clone.lock().unwrap().push(e);
        });

        let coord = GridCoord::new(0, 0, 0);
        // Make the cell active via LRU fast-path.
        mgr.lru_cache.touch(coord);
        mgr.force_load_cell(coord).await.unwrap();
        assert!(mgr.is_cell_active(coord));

        // Update with a camera position that yields no desired cells (radius too small vs cell center distance).
        // This should unload the active cell.
        mgr.update(glam::Vec3::ZERO).await.unwrap();
        assert!(!mgr.is_cell_active(coord));
        assert_eq!(mgr.metrics.total_unloads, 1);

        let captured = events.lock().unwrap().clone();
        assert!(captured
            .iter()
            .any(|e| matches!(e, StreamingEvent::CellUnloadStarted(c) if *c == coord)));
        assert!(captured
            .iter()
            .any(|e| matches!(e, StreamingEvent::CellUnloaded(c) if *c == coord)));
    }

    #[tokio::test]
    async fn test_update_metrics_tracks_counts() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mut mgr = create_streaming_manager(Arc::clone(&partition));

        let coord = GridCoord::new(0, 0, 0);
        {
            let mut p = partition.write().await;
            p.get_or_create_cell(coord).state = CellState::Loaded;
        }

        mgr.update_metrics().await;
        assert_eq!(mgr.metrics.loaded_cells, 1);
        assert_eq!(mgr.metrics.active_cells, 0);
        assert_eq!(mgr.metrics.cached_cells, 0);
    }

    // ── Additional streaming tests ──

    #[test]
    fn test_streaming_config_custom_values() {
        let cfg = StreamingConfig {
            max_active_cells: 100,
            lru_cache_size: 20,
            streaming_radius: 1000.0,
            max_concurrent_loads: 8,
        };
        assert_eq!(cfg.max_active_cells, 100);
        assert_eq!(cfg.lru_cache_size, 20);
        assert_eq!(cfg.streaming_radius, 1000.0);
        assert_eq!(cfg.max_concurrent_loads, 8);
    }

    #[test]
    fn test_streaming_config_clone() {
        let cfg = StreamingConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.max_active_cells, cloned.max_active_cells);
        assert_eq!(cfg.streaming_radius, cloned.streaming_radius);
    }

    #[test]
    fn test_streaming_config_debug() {
        let cfg = StreamingConfig::default();
        let debug = format!("{:?}", cfg);
        assert!(debug.contains("StreamingConfig"));
    }

    #[test]
    fn test_streaming_metrics_default_all_zero() {
        let metrics = StreamingMetrics::default();
        assert_eq!(metrics.active_cells, 0);
        assert_eq!(metrics.loading_cells, 0);
        assert_eq!(metrics.loaded_cells, 0);
        assert_eq!(metrics.cached_cells, 0);
        assert_eq!(metrics.memory_usage_bytes, 0);
        assert_eq!(metrics.total_loads, 0);
        assert_eq!(metrics.total_unloads, 0);
        assert_eq!(metrics.failed_loads, 0);
    }

    #[test]
    fn test_streaming_metrics_clone_and_debug() {
        let metrics = StreamingMetrics {
            active_cells: 5,
            loading_cells: 2,
            loaded_cells: 10,
            cached_cells: 3,
            memory_usage_bytes: 1024,
            total_loads: 100,
            total_unloads: 50,
            failed_loads: 5,
        };
        let cloned = metrics.clone();
        assert_eq!(cloned.active_cells, 5);
        assert_eq!(cloned.total_loads, 100);
        assert_eq!(cloned.failed_loads, 5);

        let debug = format!("{:?}", metrics);
        assert!(debug.contains("StreamingMetrics"));
    }

    #[test]
    fn test_streaming_event_debug_and_clone() {
        let events = vec![
            StreamingEvent::CellLoadStarted(GridCoord::new(1, 2, 3)),
            StreamingEvent::CellLoaded(GridCoord::new(4, 5, 6)),
            StreamingEvent::CellLoadFailed(GridCoord::new(7, 8, 9), "disk error".into()),
            StreamingEvent::CellUnloadStarted(GridCoord::new(10, 11, 12)),
            StreamingEvent::CellUnloaded(GridCoord::new(13, 14, 15)),
        ];
        for event in &events {
            let debug = format!("{:?}", event);
            assert!(!debug.is_empty());
            let cloned = event.clone();
            assert_eq!(format!("{:?}", cloned), debug);
        }
    }

    #[tokio::test]
    async fn test_new_manager_has_empty_state() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mgr = WorldPartitionManager::new(Arc::clone(&partition), StreamingConfig::default());

        assert!(mgr.active_cells().is_empty());
        assert!(!mgr.is_cell_active(GridCoord::new(0, 0, 0)));
        assert!(!mgr.is_cell_loading(GridCoord::new(0, 0, 0)));
        assert_eq!(mgr.metrics().active_cells, 0);
        assert_eq!(mgr.metrics().total_loads, 0);
    }

    #[tokio::test]
    async fn test_force_load_already_active_is_noop() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mut mgr = WorldPartitionManager::new(
            Arc::clone(&partition),
            StreamingConfig {
                streaming_radius: 0.1,
                ..Default::default()
            },
        );
        let coord = GridCoord::new(0, 0, 0);

        // Make active via LRU fast path
        mgr.lru_cache.touch(coord);
        mgr.force_load_cell(coord).await.unwrap();
        assert!(mgr.is_cell_active(coord));
        assert_eq!(mgr.metrics.total_loads, 1);

        // Force load again: already active, should return Ok without incrementing
        mgr.force_load_cell(coord).await.unwrap();
        assert_eq!(mgr.metrics.total_loads, 1); // same as before
    }

    #[tokio::test]
    async fn test_force_unload_inactive_cell_is_noop() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mut mgr =
            WorldPartitionManager::new(Arc::clone(&partition), StreamingConfig::default());
        let coord = GridCoord::new(5, 5, 5);

        // Not active, unload should be a no-op
        mgr.force_unload_cell(coord).await.unwrap();
        assert_eq!(mgr.metrics.total_unloads, 0);
    }

    #[tokio::test]
    async fn test_force_unload_moves_to_lru() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mut mgr = WorldPartitionManager::new(
            Arc::clone(&partition),
            StreamingConfig {
                streaming_radius: 0.1,
                ..Default::default()
            },
        );
        let coord = GridCoord::new(0, 0, 0);

        // Make active via LRU
        mgr.lru_cache.touch(coord);
        mgr.force_load_cell(coord).await.unwrap();
        assert!(mgr.is_cell_active(coord));

        // Force unload: should move to LRU
        mgr.force_unload_cell(coord).await.unwrap();
        assert!(!mgr.is_cell_active(coord));
        assert_eq!(mgr.metrics.total_unloads, 1);
    }

    #[tokio::test]
    async fn test_create_streaming_manager_helper() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mgr = create_streaming_manager(Arc::clone(&partition));
        assert!(mgr.active_cells().is_empty());
        // Should use default config
        assert_eq!(mgr.metrics().total_loads, 0);
    }

    #[tokio::test]
    async fn test_active_cells_returns_all_active() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mut mgr = WorldPartitionManager::new(
            Arc::clone(&partition),
            StreamingConfig {
                streaming_radius: 0.1,
                ..Default::default()
            },
        );
        let c1 = GridCoord::new(0, 0, 0);
        let c2 = GridCoord::new(1, 0, 0);

        mgr.lru_cache.touch(c1);
        mgr.lru_cache.touch(c2);
        mgr.force_load_cell(c1).await.unwrap();
        mgr.force_load_cell(c2).await.unwrap();

        let active = mgr.active_cells();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&c1));
        assert!(active.contains(&c2));
    }

    #[tokio::test]
    async fn test_event_listener_receives_load_events() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mut mgr = WorldPartitionManager::new(
            Arc::clone(&partition),
            StreamingConfig {
                streaming_radius: 0.1,
                ..Default::default()
            },
        );
        let coord = GridCoord::new(0, 0, 0);

        let events: Arc<Mutex<Vec<StreamingEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);
        mgr.add_event_listener(move |e| {
            events_clone.lock().unwrap().push(e);
        });

        mgr.lru_cache.touch(coord);
        mgr.force_load_cell(coord).await.unwrap();

        let captured = events.lock().unwrap().clone();
        // LRU fast-path emits CellLoaded (no CellLoadStarted)
        assert!(captured
            .iter()
            .any(|e| matches!(e, StreamingEvent::CellLoaded(c) if *c == coord)));
    }

    #[tokio::test]
    async fn test_multiple_event_listeners() {
        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mut mgr = WorldPartitionManager::new(
            Arc::clone(&partition),
            StreamingConfig {
                streaming_radius: 0.1,
                ..Default::default()
            },
        );

        let count1: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let count2: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let c1 = Arc::clone(&count1);
        let c2 = Arc::clone(&count2);

        mgr.add_event_listener(move |_| {
            *c1.lock().unwrap() += 1;
        });
        mgr.add_event_listener(move |_| {
            *c2.lock().unwrap() += 1;
        });

        let coord = GridCoord::new(0, 0, 0);
        mgr.lru_cache.touch(coord);
        mgr.force_load_cell(coord).await.unwrap();

        assert!(*count1.lock().unwrap() > 0);
        assert!(*count2.lock().unwrap() > 0);
    }

    #[tokio::test]
    async fn failed_cell_load_releases_the_streaming_slot() {
        let root = std::env::current_dir().expect("cwd");
        let cells = root.join("assets/cells");
        std::fs::create_dir_all(&cells).expect("cells dir");
        let cell_path = cells.join("9001_0_0.ron");
        let _cleanup = FileCleanup(cell_path.clone());
        std::fs::write(
            &cell_path,
            r#"(
                coord: [9001, 0, 0],
                entities: [],
                assets: [
                    (path: "missing_9001.gltf", kind: Mesh, guid: None),
                ],
                metadata: None,
            )"#,
        )
        .expect("cell ron");

        let partition = Arc::new(RwLock::new(WorldPartition::new(GridConfig::default())));
        let mut mgr = WorldPartitionManager::new(
            Arc::clone(&partition),
            StreamingConfig {
                streaming_radius: 0.0,
                max_concurrent_loads: 1,
                ..Default::default()
            },
        );
        let events: Arc<Mutex<Vec<StreamingEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&events);
        mgr.add_event_listener(move |event| {
            events_clone.lock().unwrap().push(event);
        });

        let coord = GridCoord::new(9001, 0, 0);
        mgr.force_load_cell(coord).await.unwrap();
        assert!(mgr.is_cell_loading(coord));

        let mut released = false;
        for _ in 0..50 {
            mgr.update(glam::Vec3::ZERO).await.unwrap();
            if mgr.metrics().failed_loads == 1 && !mgr.is_cell_loading(coord) {
                released = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(released, "failed load kept the streaming slot");
        {
            let partition = partition.read().await;
            assert_eq!(
                partition.get_cell(coord).unwrap().state,
                CellState::Unloaded
            );
        }
        let captured = events.lock().unwrap().clone();
        assert!(captured.iter().any(|event| matches!(
            event,
            StreamingEvent::CellLoadFailed(failed, _) if *failed == coord
        )));

        mgr.update(glam::Vec3::ZERO).await.unwrap();
        assert_eq!(mgr.metrics().failed_loads, 1);
        assert!(!mgr.is_cell_loading(coord));
    }

    struct FileCleanup(std::path::PathBuf);

    impl Drop for FileCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let mut dir = self.0.parent();
            for _ in 0..2 {
                let Some(path) = dir else {
                    break;
                };
                if std::fs::remove_dir(path).is_err() {
                    break;
                }
                dir = path.parent();
            }
        }
    }
}
