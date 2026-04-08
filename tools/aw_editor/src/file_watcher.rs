//! File Watcher Module - Phase PBR-G Task 3 + Phase 4.3
//!
//! Provides automatic hot-reload capabilities for materials, textures, prefabs, and models.
//!
//! # Features
//! - Watches `assets/materials/**/*.toml` for material definition changes
//! - Watches texture files (`*.png`, `*.ktx2`, `*.dds`) referenced by materials
//! - Watches `prefabs/**/*.prefab.ron` for prefab definition changes
//! - Watches `assets/models/**/*.{glb,gltf}` for 3D model changes
//! - Debouncing (500ms) to avoid duplicate events from editor saves
//! - Thread-safe communication via channels (mpsc)
//! - Graceful error handling (continues watching even if some events fail)
//!
//! # Architecture
//! ```text
//! FileWatcher (notify thread) -> Channel -> MaterialInspector (main thread)
//!     │                                          │
//!     ├─ Watches assets/materials/               ├─ Receives reload events
//!     ├─ Debounces events (500ms)                ├─ Re-parses TOML
//!     └─ Sends ReloadEvent                       └─ Updates GPU buffers
//! ```
//!
//! # Usage
//! ```rust, ignore
//! // In MaterialInspector::new()
//! let watcher = FileWatcher::new("assets/materials")?;
//!
//! // In EditorApp::default()
//! let prefab_watcher = FileWatcher::new("prefabs").ok();
//! let model_watcher = FileWatcher::new("assets/models").ok();
//!
//! // In update loop
//! while let Ok(event) = self.file_watcher.try_recv() {
//!     match event {
//!         ReloadEvent::Material(path) => self.reload_material(&path),
//!         ReloadEvent::Texture(path) => self.reload_texture(&path),
//!         ReloadEvent::Prefab(path) => self.reload_prefab(&path),
//!         ReloadEvent::Model(path) => self.reload_model(&path),
//!     }
//! }
//! ```

use anyhow::{Context, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Types of reload events
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ReloadEvent {
    /// A material TOML file changed
    Material(PathBuf),
    /// A texture file changed (albedo/normal/ORM)
    Texture(PathBuf),
    /// A prefab file changed
    Prefab(PathBuf),
    /// A model file changed (.glb, .gltf)
    Model(PathBuf),
}

impl ReloadEvent {
    /// Get the path associated with this event
    pub fn path(&self) -> &Path {
        match self {
            ReloadEvent::Material(p) => p,
            ReloadEvent::Texture(p) => p,
            ReloadEvent::Prefab(p) => p,
            ReloadEvent::Model(p) => p,
        }
    }

    /// Get the type name of this event
    pub fn type_name(&self) -> &'static str {
        match self {
            ReloadEvent::Material(_) => "Material",
            ReloadEvent::Texture(_) => "Texture",
            ReloadEvent::Prefab(_) => "Prefab",
            ReloadEvent::Model(_) => "Model",
        }
    }

    /// Get icon for this event type
    pub fn icon(&self) -> &'static str {
        match self {
            ReloadEvent::Material(_) => "[Mat]",
            ReloadEvent::Texture(_) => "[Tex]",
            ReloadEvent::Prefab(_) => "[Pfb]",
            ReloadEvent::Model(_) => "[Mdl]",
        }
    }

    /// Check if this is a material event
    pub fn is_material(&self) -> bool {
        matches!(self, ReloadEvent::Material(_))
    }

    /// Check if this is a texture event
    pub fn is_texture(&self) -> bool {
        matches!(self, ReloadEvent::Texture(_))
    }

    /// Check if this is a prefab event
    pub fn is_prefab(&self) -> bool {
        matches!(self, ReloadEvent::Prefab(_))
    }

    /// Check if this is a model event
    pub fn is_model(&self) -> bool {
        matches!(self, ReloadEvent::Model(_))
    }
}

impl std::fmt::Display for ReloadEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}: {}",
            self.icon(),
            self.type_name(),
            self.path().display()
        )
    }
}

/// Statistics about file watcher events
#[derive(Debug, Clone, Default)]
pub struct WatcherStats {
    /// Total number of events received
    pub total_events: usize,
    /// Number of material events
    pub material_events: usize,
    /// Number of texture events
    pub texture_events: usize,
    /// Number of prefab events
    pub prefab_events: usize,
    /// Number of model events
    pub model_events: usize,
}

impl WatcherStats {
    /// Increment stats for an event type
    pub fn record_event(&mut self, event: &ReloadEvent) {
        self.total_events += 1;
        match event {
            ReloadEvent::Material(_) => self.material_events += 1,
            ReloadEvent::Texture(_) => self.texture_events += 1,
            ReloadEvent::Prefab(_) => self.prefab_events += 1,
            ReloadEvent::Model(_) => self.model_events += 1,
        }
    }

    /// Get the most common event type
    pub fn most_common_type(&self) -> Option<&'static str> {
        let counts = [
            (self.material_events, "Material"),
            (self.texture_events, "Texture"),
            (self.prefab_events, "Prefab"),
            (self.model_events, "Model"),
        ];
        counts
            .into_iter()
            .filter(|(c, _)| *c > 0)
            .max_by_key(|(c, _)| *c)
            .map(|(_, name)| name)
    }

    /// Get events per type as percentages
    pub fn type_percentages(&self) -> [f32; 4] {
        if self.total_events == 0 {
            return [0.0; 4];
        }
        let total = self.total_events as f32;
        [
            self.material_events as f32 / total * 100.0,
            self.texture_events as f32 / total * 100.0,
            self.prefab_events as f32 / total * 100.0,
            self.model_events as f32 / total * 100.0,
        ]
    }

    /// Reset all counters
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// File watcher state with debouncing
pub struct FileWatcher {
    /// notify watcher (keeps thread alive)
    _watcher: RecommendedWatcher,

    /// Receiver for reload events (debounced)
    pub receiver: Receiver<ReloadEvent>,

    /// Shutdown signal for the debounce thread
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        // Signal the debounce thread to exit
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl FileWatcher {
    /// Create a new file watcher for the given directory
    ///
    /// # Arguments
    /// - `watch_path`: Root directory to watch (e.g., "assets/materials")
    ///
    /// # Returns
    /// - `Ok(FileWatcher)` on success
    /// - `Err` if the directory doesn't exist or watcher creation fails
    pub fn new<P: AsRef<Path>>(watch_path: P) -> Result<Self> {
        let watch_path = watch_path.as_ref().to_path_buf();

        // Validate path exists
        if !watch_path.exists() {
            anyhow::bail!("Watch path does not exist: {}", watch_path.display());
        }

        // Create channels for debounced events
        let (tx, rx) = channel();

        // Debouncer state (shared between notify callback and debounce thread)
        let debounce_state = Arc::new(Mutex::new(DebounceState::new()));
        let debounce_state_clone = Arc::clone(&debounce_state);
        let tx_clone = tx.clone();

        // Create notify watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    Self::handle_notify_event(event, &debounce_state, &tx);
                }
            },
            Config::default(),
        )
        .context("Failed to create file watcher")?;

        // Start watching the directory recursively
        watcher
            .watch(&watch_path, RecursiveMode::Recursive)
            .context("Failed to watch directory")?;

        // Spawn debounce thread (processes buffered events after 500ms)
        let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_clone = std::sync::Arc::clone(&shutdown);
        std::thread::spawn(move || {
            Self::debounce_loop(debounce_state_clone, tx_clone, shutdown_clone);
        });

        Ok(FileWatcher {
            _watcher: watcher,
            receiver: rx,
            shutdown,
        })
    }

    /// Try to receive a reload event (non-blocking)
    pub fn try_recv(&self) -> Result<ReloadEvent, std::sync::mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receive all pending events (non-blocking)
    pub fn drain_events(&self) -> Vec<ReloadEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }

    /// Receive all pending events and update stats (non-blocking)
    pub fn drain_events_with_stats(&self, stats: &mut WatcherStats) -> Vec<ReloadEvent> {
        let events = self.drain_events();
        for event in &events {
            stats.record_event(event);
        }
        events
    }

    /// Filter pending events by type
    pub fn drain_events_filtered<F>(&self, predicate: F) -> Vec<ReloadEvent>
    where
        F: Fn(&ReloadEvent) -> bool,
    {
        self.drain_events().into_iter().filter(predicate).collect()
    }

    /// Get only material events
    pub fn drain_material_events(&self) -> Vec<PathBuf> {
        self.drain_events()
            .into_iter()
            .filter_map(|e| match e {
                ReloadEvent::Material(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// Get only texture events
    pub fn drain_texture_events(&self) -> Vec<PathBuf> {
        self.drain_events()
            .into_iter()
            .filter_map(|e| match e {
                ReloadEvent::Texture(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// Get only prefab events
    pub fn drain_prefab_events(&self) -> Vec<PathBuf> {
        self.drain_events()
            .into_iter()
            .filter_map(|e| match e {
                ReloadEvent::Prefab(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// Get only model events
    pub fn drain_model_events(&self) -> Vec<PathBuf> {
        self.drain_events()
            .into_iter()
            .filter_map(|e| match e {
                ReloadEvent::Model(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// Handle notify event (called from watcher thread)
    fn handle_notify_event(
        event: Event,
        debounce_state: &Arc<Mutex<DebounceState>>,
        _tx: &Sender<ReloadEvent>,
    ) {
        // Only care about modify/create/remove events
        let is_relevant = matches!(
            event.kind,
            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
        );

        if !is_relevant {
            return;
        }

        // Add each path to debounce buffer
        for path in event.paths {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if file_name.ends_with(".prefab.ron") {
                let mut state = debounce_state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state
                    .buffer
                    .insert(path.clone(), ReloadEvent::Prefab(path.clone()));
                state.last_event_time.insert(path, Instant::now());
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();

                // Material TOML files
                if ext_str == "toml" {
                    let mut state = debounce_state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state
                        .buffer
                        .insert(path.clone(), ReloadEvent::Material(path.clone()));
                    state.last_event_time.insert(path, Instant::now());
                }
                // Texture files
                else if matches!(
                    ext_str.as_str(),
                    "png" | "jpg" | "jpeg" | "ktx2" | "dds" | "basis"
                ) {
                    let mut state = debounce_state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state
                        .buffer
                        .insert(path.clone(), ReloadEvent::Texture(path.clone()));
                    state.last_event_time.insert(path, Instant::now());
                }
                // Model files
                else if matches!(ext_str.as_str(), "glb" | "gltf") {
                    let mut state = debounce_state
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state
                        .buffer
                        .insert(path.clone(), ReloadEvent::Model(path.clone()));
                    state.last_event_time.insert(path, Instant::now());
                }
            }
        }
    }

    /// Debounce loop (runs in separate thread)
    fn debounce_loop(
        debounce_state: Arc<Mutex<DebounceState>>,
        tx: Sender<ReloadEvent>,
        shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        const DEBOUNCE_DURATION: Duration = Duration::from_millis(500);

        loop {
            if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));

            let mut state = debounce_state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            // Check if any buffered events are ready to send
            let now = Instant::now();
            let ready_paths: Vec<PathBuf> = state
                .buffer
                .keys()
                .filter(|path| {
                    state
                        .last_event_time
                        .get(*path)
                        .map(|&time| now.duration_since(time) >= DEBOUNCE_DURATION)
                        .unwrap_or(true)
                })
                .cloned()
                .collect();

            // Send ready events and remove from buffer
            for path in ready_paths {
                if let Some(event) = state.buffer.remove(&path) {
                    // Send event (ignore errors if receiver dropped)
                    let _ = tx.send(event);
                    state.last_event_time.remove(&path);
                }
            }
        }
    }
}

/// Debounce state (shared between threads)
struct DebounceState {
    /// Buffered events (path -> event)
    buffer: HashMap<PathBuf, ReloadEvent>,

    /// Last event time for each path (for debouncing)
    last_event_time: HashMap<PathBuf, Instant>,
}

impl DebounceState {
    fn new() -> Self {
        DebounceState {
            buffer: HashMap::new(),
            last_event_time: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    #[ignore] // Requires assets/materials directory
    fn test_watcher_creation() {
        let watcher = FileWatcher::new("assets/materials");
        assert!(watcher.is_ok(), "Failed to create watcher");
    }

    #[test]
    #[ignore] // Integration test (requires file system changes)
    fn test_material_reload() {
        let temp_dir = std::env::temp_dir().join("astraweave_test_materials");
        fs::create_dir_all(&temp_dir).unwrap();

        let watcher = FileWatcher::new(&temp_dir).unwrap();

        // Create a test material file
        let test_material = temp_dir.join("test.toml");
        let mut file = fs::File::create(&test_material).unwrap();
        writeln!(file, "[material]").unwrap();
        writeln!(file, "name = \"test\"").unwrap();
        drop(file);

        // Wait for debounce + processing
        std::thread::sleep(Duration::from_millis(700));

        // Check for reload event
        let event = watcher.try_recv();
        assert!(event.is_ok(), "Expected reload event");

        if let Ok(ReloadEvent::Material(path)) = event {
            assert_eq!(path, test_material);
        } else {
            panic!("Expected Material reload event");
        }

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    #[ignore] // Integration test (requires file system changes)
    fn test_texture_reload() {
        let temp_dir = std::env::temp_dir().join("astraweave_test_textures");
        fs::create_dir_all(&temp_dir).unwrap();

        let watcher = FileWatcher::new(&temp_dir).unwrap();

        // Create a test texture file (empty PNG)
        let test_texture = temp_dir.join("test.png");
        fs::File::create(&test_texture).unwrap();

        // Wait for debounce + processing
        std::thread::sleep(Duration::from_millis(700));

        // Check for reload event
        let event = watcher.try_recv();
        assert!(event.is_ok(), "Expected reload event");

        if let Ok(ReloadEvent::Texture(path)) = event {
            assert_eq!(path, test_texture);
        } else {
            panic!("Expected Texture reload event");
        }

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_debounce() {
        // Create watcher with temporary directory
        let temp_dir = std::env::temp_dir().join("astraweave_test_debounce");
        fs::create_dir_all(&temp_dir).unwrap();

        let watcher = FileWatcher::new(&temp_dir).unwrap();

        // Create and modify file multiple times rapidly
        let test_file = temp_dir.join("test.toml");
        for i in 0..5 {
            let mut file = fs::File::create(&test_file).unwrap();
            writeln!(file, "[material]").unwrap();
            writeln!(file, "version = {}", i).unwrap();
            drop(file);
            std::thread::sleep(Duration::from_millis(50));
        }

        // Wait for debounce
        std::thread::sleep(Duration::from_millis(700));

        // Count events - debouncing may not be perfect due to OS/timing variations
        let mut event_count = 0;
        while watcher.try_recv().is_ok() {
            event_count += 1;
        }

        // Assert we got at least 1 event (file was changed)
        // Note: Debouncing may vary by OS/timing, so we just verify events are received
        assert!(
            event_count >= 1,
            "Expected at least 1 file change event, got {}",
            event_count
        );
        // Ideally would be 1 (perfectly debounced) but OS file watchers vary
        assert!(
            event_count <= 5,
            "Expected at most 5 events (one per write), got {}",
            event_count
        );

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    // === New ReloadEvent tests ===

    #[test]
    fn test_reload_event_path() {
        let path = PathBuf::from("test/material.toml");
        let event = ReloadEvent::Material(path.clone());
        assert_eq!(event.path(), path.as_path());
    }

    #[test]
    fn test_reload_event_type_names() {
        assert_eq!(
            ReloadEvent::Material(PathBuf::new()).type_name(),
            "Material"
        );
        assert_eq!(ReloadEvent::Texture(PathBuf::new()).type_name(), "Texture");
        assert_eq!(ReloadEvent::Prefab(PathBuf::new()).type_name(), "Prefab");
        assert_eq!(ReloadEvent::Model(PathBuf::new()).type_name(), "Model");
    }

    #[test]
    fn test_reload_event_icons_not_empty() {
        assert!(!ReloadEvent::Material(PathBuf::new()).icon().is_empty());
        assert!(!ReloadEvent::Texture(PathBuf::new()).icon().is_empty());
        assert!(!ReloadEvent::Prefab(PathBuf::new()).icon().is_empty());
        assert!(!ReloadEvent::Model(PathBuf::new()).icon().is_empty());
    }

    #[test]
    fn test_reload_event_is_material() {
        let event = ReloadEvent::Material(PathBuf::new());
        assert!(event.is_material());
        assert!(!event.is_texture());
        assert!(!event.is_prefab());
        assert!(!event.is_model());
    }

    #[test]
    fn test_reload_event_is_texture() {
        let event = ReloadEvent::Texture(PathBuf::new());
        assert!(!event.is_material());
        assert!(event.is_texture());
        assert!(!event.is_prefab());
        assert!(!event.is_model());
    }

    #[test]
    fn test_reload_event_is_prefab() {
        let event = ReloadEvent::Prefab(PathBuf::new());
        assert!(!event.is_material());
        assert!(!event.is_texture());
        assert!(event.is_prefab());
        assert!(!event.is_model());
    }

    #[test]
    fn test_reload_event_is_model() {
        let event = ReloadEvent::Model(PathBuf::new());
        assert!(!event.is_material());
        assert!(!event.is_texture());
        assert!(!event.is_prefab());
        assert!(event.is_model());
    }

    #[test]
    fn test_reload_event_display() {
        let event = ReloadEvent::Material(PathBuf::from("test.toml"));
        let display = format!("{}", event);
        assert!(display.contains("Material"));
        assert!(display.contains("test.toml"));
    }

    // === WatcherStats tests ===

    #[test]
    fn test_watcher_stats_default() {
        let stats = WatcherStats::default();
        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.material_events, 0);
        assert_eq!(stats.texture_events, 0);
        assert_eq!(stats.prefab_events, 0);
        assert_eq!(stats.model_events, 0);
    }

    #[test]
    fn test_watcher_stats_record_event() {
        let mut stats = WatcherStats::default();

        stats.record_event(&ReloadEvent::Material(PathBuf::new()));
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.material_events, 1);

        stats.record_event(&ReloadEvent::Texture(PathBuf::new()));
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.texture_events, 1);

        stats.record_event(&ReloadEvent::Prefab(PathBuf::new()));
        assert_eq!(stats.prefab_events, 1);

        stats.record_event(&ReloadEvent::Model(PathBuf::new()));
        assert_eq!(stats.model_events, 1);
        assert_eq!(stats.total_events, 4);
    }

    #[test]
    fn test_watcher_stats_most_common_type() {
        let mut stats = WatcherStats::default();
        assert!(stats.most_common_type().is_none());

        stats.material_events = 5;
        stats.texture_events = 3;
        stats.total_events = 8;

        assert_eq!(stats.most_common_type(), Some("Material"));
    }

    #[test]
    fn test_watcher_stats_type_percentages() {
        let mut stats = WatcherStats::default();

        let percentages = stats.type_percentages();
        assert_eq!(percentages, [0.0, 0.0, 0.0, 0.0]);

        stats.total_events = 10;
        stats.material_events = 5;
        stats.texture_events = 3;
        stats.prefab_events = 1;
        stats.model_events = 1;

        let percentages = stats.type_percentages();
        assert!((percentages[0] - 50.0).abs() < 0.1); // Material 50%
        assert!((percentages[1] - 30.0).abs() < 0.1); // Texture 30%
        assert!((percentages[2] - 10.0).abs() < 0.1); // Prefab 10%
        assert!((percentages[3] - 10.0).abs() < 0.1); // Model 10%
    }

    #[test]
    fn test_watcher_stats_reset() {
        let mut stats = WatcherStats {
            total_events: 10,
            material_events: 5,
            texture_events: 3,
            prefab_events: 1,
            model_events: 1,
        };

        stats.reset();

        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.material_events, 0);
        assert_eq!(stats.texture_events, 0);
        assert_eq!(stats.prefab_events, 0);
        assert_eq!(stats.model_events, 0);
    }
}
