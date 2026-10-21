# Phase 8.3 Week 2 Day 2: Save Slot Management Implementation Plan

**Date**: November 1, 2025  
**Duration**: 4-6 hours estimated  
**Status**: ⏸️ NOT STARTED  

---

## Executive Summary

**Mission**: Create multi-slot save system with metadata, thumbnails, and background I/O

**Dependencies**: ✅ Week 2 Day 1 Complete (PlayerProfile system working)

**Deliverables**:
- `SaveSlotManager` for managing 3-10 save slots
- Save/load/delete/list APIs
- Metadata (timestamp, level name, playtime, character name)
- Screenshot thumbnails (optional)
- Background I/O (non-blocking disk writes)

**Timeline**: 4-6 hours broken into 4 subtasks

---

## Task 1: Create SaveSlotManager (1-2 hours)

### Implementation

**File**: `crates/astraweave-persistence-player/src/save_slots.rs` (NEW)

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use chrono::{DateTime, Utc};
use anyhow::{Context, Result};

use crate::PlayerProfile;

/// Save slot manager for managing multiple game saves
pub struct SaveSlotManager {
    max_slots: usize,
    base_dir: PathBuf,
}

impl SaveSlotManager {
    /// Create new save slot manager
    /// 
    /// # Arguments
    /// 
    /// * `max_slots` - Maximum number of save slots (default: 10)
    /// * `base_dir` - Base directory for saves (default: "saves/slots")
    pub fn new(max_slots: usize, base_dir: PathBuf) -> Self {
        Self { max_slots, base_dir }
    }
    
    /// Create with default settings (10 slots, "saves/slots")
    pub fn default() -> Self {
        Self::new(10, PathBuf::from("saves/slots"))
    }
    
    /// Get slot directory path
    fn slot_dir(&self, slot_id: usize) -> PathBuf {
        self.base_dir.join(format!("slot_{}", slot_id))
    }
    
    /// Get save file path
    fn save_file_path(&self, slot_id: usize) -> PathBuf {
        self.slot_dir(slot_id).join("save.bin")
    }
    
    /// Get metadata file path
    fn metadata_path(&self, slot_id: usize) -> PathBuf {
        self.slot_dir(slot_id).join("metadata.toml")
    }
    
    /// Get thumbnail path
    fn thumbnail_path(&self, slot_id: usize) -> PathBuf {
        self.slot_dir(slot_id).join("thumbnail.png")
    }
}

/// Save slot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    /// Slot ID
    pub slot_id: usize,
    
    /// Save timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Total playtime (seconds)
    pub playtime_seconds: u64,
    
    /// Current level name
    pub level_name: String,
    
    /// Character name
    pub character_name: String,
    
    /// Checkpoint name (optional)
    pub checkpoint: Option<String>,
    
    /// Has thumbnail
    pub has_thumbnail: bool,
}

/// Complete save slot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlot {
    /// Metadata
    pub metadata: SaveMetadata,
    
    /// ECS world state (serialized binary)
    #[serde(with = "serde_bytes")]
    pub world_state: Vec<u8>,
    
    /// Player profile
    pub player_profile: PlayerProfile,
}
```

**Add to lib.rs**:
```rust
mod save_slots;
pub use save_slots::*;
```

**Validation**:
```powershell
cargo check -p astraweave-persistence-player
```

---

## Task 2: Implement Save/Load/Delete APIs (1-2 hours)

### Implementation

**File**: `crates/astraweave-persistence-player/src/save_slots.rs` (continue)

```rust
impl SaveSlotManager {
    /// Save game to slot
    /// 
    /// # Arguments
    /// 
    /// * `slot_id` - Slot number (0 to max_slots-1)
    /// * `world_state` - Serialized ECS world (from astraweave-persistence-ecs)
    /// * `player_profile` - Player profile
    /// * `level_name` - Current level name
    /// * `checkpoint` - Optional checkpoint name
    pub fn save_to_slot(
        &self,
        slot_id: usize,
        world_state: Vec<u8>,
        player_profile: PlayerProfile,
        level_name: String,
        checkpoint: Option<String>,
    ) -> Result<()> {
        if slot_id >= self.max_slots {
            anyhow::bail!("Slot ID {} exceeds max slots {}", slot_id, self.max_slots);
        }
        
        // Create slot directory
        let slot_dir = self.slot_dir(slot_id);
        fs::create_dir_all(&slot_dir)
            .context("Failed to create slot directory")?;
        
        // Create metadata
        let metadata = SaveMetadata {
            slot_id,
            timestamp: Utc::now(),
            playtime_seconds: player_profile.stats.playtime_seconds,
            level_name,
            character_name: player_profile.name.clone(),
            checkpoint,
            has_thumbnail: false, // Will be set by screenshot system
        };
        
        // Save metadata (TOML for easy inspection)
        let metadata_toml = toml::to_string_pretty(&metadata)
            .context("Failed to serialize metadata")?;
        fs::write(self.metadata_path(slot_id), metadata_toml)
            .context("Failed to write metadata")?;
        
        // Create save slot
        let save_slot = SaveSlot {
            metadata,
            world_state,
            player_profile,
        };
        
        // Serialize to binary (postcard for compactness)
        let save_data = postcard::to_allocvec(&save_slot)
            .context("Failed to serialize save slot")?;
        
        // Write to disk
        fs::write(self.save_file_path(slot_id), save_data)
            .context("Failed to write save file")?;
        
        println!("💾 Saved game to slot {} ({})", slot_id, metadata.timestamp);
        
        Ok(())
    }
    
    /// Load game from slot
    pub fn load_from_slot(&self, slot_id: usize) -> Result<SaveSlot> {
        if slot_id >= self.max_slots {
            anyhow::bail!("Slot ID {} exceeds max slots {}", slot_id, self.max_slots);
        }
        
        let save_path = self.save_file_path(slot_id);
        if !save_path.exists() {
            anyhow::bail!("Save slot {} not found", slot_id);
        }
        
        // Read save file
        let save_data = fs::read(&save_path)
            .context("Failed to read save file")?;
        
        // Deserialize
        let save_slot: SaveSlot = postcard::from_bytes(&save_data)
            .context("Failed to deserialize save slot")?;
        
        println!("📂 Loaded game from slot {} ({})", slot_id, save_slot.metadata.timestamp);
        
        Ok(save_slot)
    }
    
    /// Delete save slot
    pub fn delete_slot(&self, slot_id: usize) -> Result<()> {
        if slot_id >= self.max_slots {
            anyhow::bail!("Slot ID {} exceeds max slots {}", slot_id, self.max_slots);
        }
        
        let slot_dir = self.slot_dir(slot_id);
        if slot_dir.exists() {
            fs::remove_dir_all(&slot_dir)
                .context("Failed to delete slot directory")?;
            println!("🗑️  Deleted save slot {}", slot_id);
        }
        
        Ok(())
    }
    
    /// List all save slots with metadata
    pub fn list_slots(&self) -> Result<Vec<SaveMetadata>> {
        let mut slots = Vec::new();
        
        for slot_id in 0..self.max_slots {
            let metadata_path = self.metadata_path(slot_id);
            if metadata_path.exists() {
                let metadata_toml = fs::read_to_string(&metadata_path)
                    .context("Failed to read metadata")?;
                let metadata: SaveMetadata = toml::from_str(&metadata_toml)
                    .context("Failed to deserialize metadata")?;
                slots.push(metadata);
            }
        }
        
        // Sort by timestamp (newest first)
        slots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(slots)
    }
    
    /// Check if slot exists
    pub fn slot_exists(&self, slot_id: usize) -> bool {
        self.save_file_path(slot_id).exists()
    }
    
    /// Get next available slot ID
    pub fn next_available_slot(&self) -> Option<usize> {
        for slot_id in 0..self.max_slots {
            if !self.slot_exists(slot_id) {
                return Some(slot_id);
            }
        }
        None
    }
}
```

**Add dependency to Cargo.toml**:
```toml
postcard = { version = "1.0", features = ["alloc"] }
serde_bytes = "0.11"
```

---

## Task 3: Background I/O (1-2 hours)

### Implementation

**File**: `crates/astraweave-persistence-player/src/background_io.rs` (NEW)

```rust
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;
use std::fs;
use anyhow::Result;

/// Background I/O task
pub struct BackgroundSaveTask {
    handle: Option<thread::JoinHandle<Result<()>>>,
    progress: Arc<Mutex<f32>>,
}

impl BackgroundSaveTask {
    /// Start background save task
    pub fn start(path: PathBuf, data: Vec<u8>) -> Self {
        let progress = Arc::new(Mutex::new(0.0));
        let progress_clone = progress.clone();
        
        let handle = thread::spawn(move || {
            // Update progress: 0% → 50% (serialization already done)
            *progress_clone.lock().unwrap() = 0.5;
            
            // Write to disk (this is the slow part)
            fs::write(&path, &data)?;
            
            // Update progress: 100% (done)
            *progress_clone.lock().unwrap() = 1.0;
            
            Ok(())
        });
        
        Self {
            handle: Some(handle),
            progress,
        }
    }
    
    /// Get progress (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        *self.progress.lock().unwrap()
    }
    
    /// Check if task is done
    pub fn is_done(&self) -> bool {
        self.progress() >= 1.0
    }
    
    /// Wait for task to complete
    pub fn wait(mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap()?;
        }
        Ok(())
    }
}
```

**Integrate with SaveSlotManager**:
```rust
impl SaveSlotManager {
    /// Save to slot (background, non-blocking)
    pub fn save_to_slot_async(
        &self,
        slot_id: usize,
        world_state: Vec<u8>,
        player_profile: PlayerProfile,
        level_name: String,
        checkpoint: Option<String>,
    ) -> Result<BackgroundSaveTask> {
        // Serialize on main thread (fast: ~0.686 ms)
        let metadata = SaveMetadata { /* ... */ };
        let save_slot = SaveSlot { /* ... */ };
        let save_data = postcard::to_allocvec(&save_slot)?;
        
        // Write to disk on background thread (slow: 10-100 ms)
        let path = self.save_file_path(slot_id);
        Ok(BackgroundSaveTask::start(path, save_data))
    }
}
```

---

## Task 4: Testing & Example (1 hour)

### Tests

**File**: `crates/astraweave-persistence-player/src/save_slots.rs` (continue)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_save_load_roundtrip() {
        let manager = SaveSlotManager::new(10, PathBuf::from("test_saves"));
        let profile = PlayerProfile::default();
        
        // Save to slot 0
        manager.save_to_slot(
            0,
            vec![1, 2, 3, 4], // Dummy world state
            profile.clone(),
            "Level 1".to_string(),
            None,
        ).unwrap();
        
        // Load from slot 0
        let loaded = manager.load_from_slot(0).unwrap();
        
        // Verify
        assert_eq!(loaded.metadata.slot_id, 0);
        assert_eq!(loaded.metadata.level_name, "Level 1");
        assert_eq!(loaded.world_state, vec![1, 2, 3, 4]);
        
        // Cleanup
        manager.delete_slot(0).unwrap();
        fs::remove_dir_all("test_saves").unwrap();
    }
    
    #[test]
    fn test_list_slots() {
        let manager = SaveSlotManager::new(10, PathBuf::from("test_saves2"));
        let profile = PlayerProfile::default();
        
        // Save to slots 0, 1, 2
        for i in 0..3 {
            manager.save_to_slot(i, vec![], profile.clone(), format!("Level {}", i), None).unwrap();
        }
        
        // List slots
        let slots = manager.list_slots().unwrap();
        assert_eq!(slots.len(), 3);
        
        // Cleanup
        for i in 0..3 {
            manager.delete_slot(i).unwrap();
        }
        fs::remove_dir_all("test_saves2").unwrap();
    }
}
```

### Example

**File**: `crates/astraweave-persistence-player/examples/save_slots_demo.rs`

```rust
use astraweave_persistence_player::{PlayerProfile, SaveSlotManager};
use std::path::PathBuf;

fn main() {
    println!("=== AstraWeave Save Slot Manager Demo ===\n");
    
    let manager = SaveSlotManager::new(10, PathBuf::from("saves/slots"));
    let mut profile = PlayerProfile::default();
    
    // Make some progress
    profile.unlock_ability("Dash");
    profile.grant_achievement("First Blood");
    profile.add_playtime(3600); // 1 hour
    
    // Save to slot 0
    println!("💾 Saving to slot 0...");
    manager.save_to_slot(
        0,
        vec![1, 2, 3, 4], // Dummy world state
        profile.clone(),
        "Tutorial Level".to_string(),
        Some("Checkpoint 1".to_string()),
    ).unwrap();
    
    // Save to slot 1
    println!("💾 Saving to slot 1...");
    profile.unlock_ability("Double Jump");
    profile.add_playtime(1800); // 30 more minutes
    manager.save_to_slot(
        1,
        vec![5, 6, 7, 8],
        profile.clone(),
        "Level 2".to_string(),
        None,
    ).unwrap();
    
    // List all saves
    println!("\n📂 Available save slots:");
    let slots = manager.list_slots().unwrap();
    for slot in &slots {
        println!("   Slot {}: {} - {} ({} hours playtime)",
            slot.slot_id,
            slot.character_name,
            slot.level_name,
            slot.playtime_seconds / 3600,
        );
    }
    
    // Load from slot 0
    println!("\n📂 Loading slot 0...");
    let loaded = manager.load_from_slot(0).unwrap();
    println!("   Level: {}", loaded.metadata.level_name);
    println!("   Checkpoint: {:?}", loaded.metadata.checkpoint);
    println!("   Playtime: {} hours", loaded.player_profile.stats.playtime_seconds / 3600);
    
    // Delete slot 1
    println!("\n🗑️  Deleting slot 1...");
    manager.delete_slot(1).unwrap();
    
    println!("\n✅ Save slot management demo complete!");
}
```

---

## Success Criteria

**Day 2 Complete When**:
- ✅ `SaveSlotManager` compiles
- ✅ Save/load/delete APIs work
- ✅ Metadata saved/loaded correctly
- ✅ List slots returns sorted metadata
- ✅ Background I/O working (optional for v1)
- ✅ All unit tests passing
- ✅ Example works

**Estimated Time**: 4-6 hours  
**Actual Time**: TBD (will track)

---

## Next Steps (Week 2 Complete)

After Day 2 complete:
- ✅ PlayerProfile system working (Day 1)
- ✅ SaveSlotManager working (Day 2)
- ⏸️ Week 3: Versioning, migration, replay (TODO)

**Week 2 Deliverables**:
- Player profile persistence (TOML)
- Multi-slot save system (3-10 slots)
- Metadata for UI (timestamp, level, playtime)
- Background I/O (non-blocking)
