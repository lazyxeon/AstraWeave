//! Save slot management for multi-slot game saves

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
    /// * `max_slots` - Maximum number of save slots (e.g., 10)
    /// * `base_dir` - Base directory for saves (e.g., "saves/slots")
    pub fn new(max_slots: usize, base_dir: PathBuf) -> Self {
        Self {
            max_slots,
            base_dir,
        }
    }

    /// Create with default settings (10 slots, "saves/slots")
    pub fn with_defaults() -> Self {
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
    #[allow(dead_code)]
    fn thumbnail_path(&self, slot_id: usize) -> PathBuf {
        self.slot_dir(slot_id).join("thumbnail.png")
    }

    /// Save game to slot
    ///
    /// # Arguments
    ///
    /// * `slot_id` - Slot number (0 to max_slots-1)
    /// * `world_state` - Serialized ECS world (from astraweave-persistence-ecs)
    /// * `player_profile` - Player profile
    /// * `level_name` - Current level name
    /// * `checkpoint` - Optional checkpoint name
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use astraweave_persistence_player::{PlayerProfile, SaveSlotManager};
    /// # use std::path::PathBuf;
    /// let manager = SaveSlotManager::with_defaults();
    /// let profile = PlayerProfile::default();
    /// let world_state = vec![1, 2, 3, 4]; // From serialize_ecs_world()
    ///
    /// manager.save_to_slot(
    ///     0,
    ///     world_state,
    ///     profile,
    ///     "Tutorial Level".to_string(),
    ///     Some("Checkpoint 1".to_string()),
    /// ).unwrap();
    /// ```
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
        fs::create_dir_all(&slot_dir).context("Failed to create slot directory")?;

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
        let metadata_toml =
            toml::to_string_pretty(&metadata).context("Failed to serialize metadata")?;
        fs::write(self.metadata_path(slot_id), metadata_toml)
            .context("Failed to write metadata")?;

        // Create save slot
        let save_slot = SaveSlot {
            metadata: metadata.clone(),
            world_state,
            player_profile,
        };

        // Serialize to binary (postcard for compactness)
        let save_data =
            postcard::to_allocvec(&save_slot).context("Failed to serialize save slot")?;

        // Write to disk
        fs::write(self.save_file_path(slot_id), save_data).context("Failed to write save file")?;

        println!("💾 Saved game to slot {} ({})", slot_id, metadata.timestamp);

        Ok(())
    }

    /// Load game from slot
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use astraweave_persistence_player::SaveSlotManager;
    /// let manager = SaveSlotManager::with_defaults();
    /// let save_slot = manager.load_from_slot(0).unwrap();
    /// println!("Loaded: {}", save_slot.metadata.level_name);
    /// ```
    pub fn load_from_slot(&self, slot_id: usize) -> Result<SaveSlot> {
        if slot_id >= self.max_slots {
            anyhow::bail!("Slot ID {} exceeds max slots {}", slot_id, self.max_slots);
        }

        let save_path = self.save_file_path(slot_id);
        if !save_path.exists() {
            anyhow::bail!("Save slot {} not found", slot_id);
        }

        // Read save file
        let save_data = fs::read(&save_path).context("Failed to read save file")?;

        // Deserialize
        let save_slot: SaveSlot =
            postcard::from_bytes(&save_data).context("Failed to deserialize save slot")?;

        println!(
            "📂 Loaded game from slot {} ({})",
            slot_id, save_slot.metadata.timestamp
        );

        Ok(save_slot)
    }

    /// Delete save slot
    pub fn delete_slot(&self, slot_id: usize) -> Result<()> {
        if slot_id >= self.max_slots {
            anyhow::bail!("Slot ID {} exceeds max slots {}", slot_id, self.max_slots);
        }

        let slot_dir = self.slot_dir(slot_id);
        if slot_dir.exists() {
            fs::remove_dir_all(&slot_dir).context("Failed to delete slot directory")?;
            println!("🗑️  Deleted save slot {}", slot_id);
        }

        Ok(())
    }

    /// List all save slots with metadata
    ///
    /// Returns slots sorted by timestamp (newest first)
    pub fn list_slots(&self) -> Result<Vec<SaveMetadata>> {
        let mut slots = Vec::new();

        for slot_id in 0..self.max_slots {
            let metadata_path = self.metadata_path(slot_id);
            if metadata_path.exists() {
                let metadata_toml =
                    fs::read_to_string(&metadata_path).context("Failed to read metadata")?;
                let metadata: SaveMetadata =
                    toml::from_str(&metadata_toml).context("Failed to deserialize metadata")?;
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
        (0..self.max_slots).find(|&slot_id| !self.slot_exists(slot_id))
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Workspace `cargo test` shares a cwd. A relative save dir left behind by
    /// an earlier run makes `next_available_slot` look full.
    fn isolated_saves(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aw_save_slots_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = isolated_saves("roundtrip");
        let manager = SaveSlotManager::new(10, dir.clone());
        let profile = PlayerProfile::default();

        // Save to slot 0
        manager
            .save_to_slot(
                0,
                vec![1, 2, 3, 4], // Dummy world state
                profile.clone(),
                "Level 1".to_string(),
                None,
            )
            .unwrap();

        // Load from slot 0
        let loaded = manager.load_from_slot(0).unwrap();

        // Verify
        assert_eq!(loaded.metadata.slot_id, 0);
        assert_eq!(loaded.metadata.level_name, "Level 1");
        assert_eq!(loaded.world_state, vec![1, 2, 3, 4]);

        // Cleanup
        manager.delete_slot(0).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_list_slots() {
        let dir = isolated_saves("list");
        let manager = SaveSlotManager::new(10, dir.clone());
        let profile = PlayerProfile::default();

        // Save to slots 0, 1, 2
        for i in 0..3 {
            manager
                .save_to_slot(i, vec![], profile.clone(), format!("Level {}", i), None)
                .unwrap();
        }

        // List slots
        let slots = manager.list_slots().unwrap();
        assert_eq!(slots.len(), 3);

        // Cleanup
        for i in 0..3 {
            manager.delete_slot(i).unwrap();
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_next_available_slot() {
        let dir = isolated_saves("next");
        let manager = SaveSlotManager::new(3, dir.clone());
        let profile = PlayerProfile::default();

        // Initially, slot 0 should be available
        assert_eq!(manager.next_available_slot(), Some(0));

        // Save to slot 0
        manager
            .save_to_slot(0, vec![], profile.clone(), "Level 1".to_string(), None)
            .unwrap();

        // Now slot 1 should be available
        assert_eq!(manager.next_available_slot(), Some(1));

        // Fill all slots
        manager
            .save_to_slot(1, vec![], profile.clone(), "Level 2".to_string(), None)
            .unwrap();
        manager
            .save_to_slot(2, vec![], profile.clone(), "Level 3".to_string(), None)
            .unwrap();

        // No slots available
        assert_eq!(manager.next_available_slot(), None);

        // Cleanup
        for i in 0..3 {
            manager.delete_slot(i).unwrap();
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
