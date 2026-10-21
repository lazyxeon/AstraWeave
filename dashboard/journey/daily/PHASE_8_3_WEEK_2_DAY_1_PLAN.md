# Phase 8.3 Week 2 Day 1: Player Profile System Implementation Plan

**Date**: November 1, 2025  
**Duration**: 4-6 hours estimated  
**Status**: ⏸️ NOT STARTED  

---

## Executive Summary

**Mission**: Create `PlayerProfile` system with settings, stats, unlocks, and TOML persistence

**Dependencies**: ✅ Week 1 Complete (ECS serialization proven @ 0.686 ms)

**Deliverables**:
- `PlayerProfile` struct with TOML serialization
- Settings integration (graphics, audio, controls)
- Progression tracking (unlocks, achievements, stats)
- Error handling (corrupted profiles → reset to default)

**Timeline**: 4-6 hours broken into 4 subtasks

---

## Task 1: Create PlayerProfile Struct (1 hour)

### Implementation

**File**: `crates/astraweave-persistence-player/Cargo.toml` (NEW CRATE)

```toml
[package]
name = "astraweave-persistence-player"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
anyhow = "1.0"
chrono = { version = "0.4", features = ["serde"] }
```

**File**: `crates/astraweave-persistence-player/src/lib.rs`

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub version: u32,                // For future migrations
    pub name: String,
    pub settings: GameSettings,
    pub stats: PlayerStats,
    pub unlocks: Unlocks,
    #[serde(default)]
    pub inventory: Inventory,        // Optional for now
    #[serde(default)]
    pub quest_progress: QuestProgress, // Optional for now
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
    pub controls: ControlSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub resolution: (u32, u32),      // Width × height
    pub quality: QualityPreset,
    pub vsync: bool,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QualityPreset {
    Low,
    Medium,
    High,
    Ultra,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub master_volume: f32,          // 0.0 - 1.0
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub voice_volume: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSettings {
    pub mouse_sensitivity: f32,      // 0.1 - 2.0
    pub invert_y: bool,
    pub key_bindings: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub playtime_seconds: u64,       // Total playtime
    pub enemies_defeated: u32,
    pub deaths: u32,
    pub achievements: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Unlocks {
    pub abilities: Vec<String>,
    pub items: Vec<String>,
    pub levels: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    // Placeholder for now (Week 3+)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestProgress {
    // Placeholder for now (Week 3+)
}

impl Default for PlayerProfile {
    fn default() -> Self {
        Self {
            version: 1,
            name: "Player".to_string(),
            settings: GameSettings::default(),
            stats: PlayerStats::default(),
            unlocks: Unlocks::default(),
            inventory: Inventory::default(),
            quest_progress: QuestProgress::default(),
        }
    }
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            graphics: GraphicsSettings::default(),
            audio: AudioSettings::default(),
            controls: ControlSettings::default(),
        }
    }
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            resolution: (1920, 1080),
            quality: QualityPreset::High,
            vsync: true,
            fullscreen: false,
        }
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.7,
            music_volume: 0.5,
            sfx_volume: 0.7,
            voice_volume: 0.8,
            muted: false,
        }
    }
}

impl Default for ControlSettings {
    fn default() -> Self {
        use std::collections::HashMap;
        let mut key_bindings = HashMap::new();
        key_bindings.insert("forward".to_string(), "W".to_string());
        key_bindings.insert("backward".to_string(), "S".to_string());
        key_bindings.insert("left".to_string(), "A".to_string());
        key_bindings.insert("right".to_string(), "D".to_string());
        key_bindings.insert("jump".to_string(), "Space".to_string());
        key_bindings.insert("interact".to_string(), "E".to_string());
        
        Self {
            mouse_sensitivity: 1.0,
            invert_y: false,
            key_bindings,
        }
    }
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            playtime_seconds: 0,
            enemies_defeated: 0,
            deaths: 0,
            achievements: Vec::new(),
        }
    }
}
```

**Validation**:
```powershell
cargo check -p astraweave-persistence-player
```

**Success Criteria**:
- ✅ All types compile
- ✅ Default implementations work
- ✅ TOML serialization support (serde derives)

---

## Task 2: Implement Save/Load (1 hour)

### Implementation

**File**: `crates/astraweave-persistence-player/src/lib.rs` (continue)

```rust
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

impl PlayerProfile {
    /// Save profile to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_string = toml::to_string_pretty(self)
            .context("Failed to serialize PlayerProfile to TOML")?;
        
        // Create parent directory if needed
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .context("Failed to create saves directory")?;
        }
        
        fs::write(&path, toml_string)
            .context("Failed to write PlayerProfile to disk")?;
        
        Ok(())
    }
    
    /// Load profile from TOML file, or create default if missing/corrupted
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        
        // If file doesn't exist, return default
        if !path_ref.exists() {
            eprintln!("⚠️  Profile not found at {:?}, creating default", path_ref);
            return Ok(Self::default());
        }
        
        // Try to load file
        let contents = fs::read_to_string(path_ref)
            .context("Failed to read PlayerProfile file")?;
        
        // Try to deserialize
        match toml::from_str::<Self>(&contents) {
            Ok(profile) => Ok(profile),
            Err(e) => {
                eprintln!("⚠️  Corrupted profile at {:?}: {}", path_ref, e);
                eprintln!("⚠️  Resetting to default profile");
                Ok(Self::default())
            }
        }
    }
    
    /// Get default save path
    pub fn default_path() -> PathBuf {
        PathBuf::from("saves/player_profile.toml")
    }
    
    /// Quick save (to default path)
    pub fn quick_save(&self) -> Result<()> {
        self.save_to_file(Self::default_path())
    }
    
    /// Quick load (from default path)
    pub fn quick_load() -> Result<Self> {
        Self::load_from_file(Self::default_path())
    }
}
```

**Test**: `crates/astraweave-persistence-player/src/lib.rs` (continue)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_profile() {
        let profile = PlayerProfile::default();
        assert_eq!(profile.version, 1);
        assert_eq!(profile.name, "Player");
        assert_eq!(profile.settings.graphics.resolution, (1920, 1080));
        assert_eq!(profile.settings.audio.master_volume, 0.7);
    }
    
    #[test]
    fn test_roundtrip() {
        let profile = PlayerProfile::default();
        let path = "test_profile.toml";
        
        // Save
        profile.save_to_file(path).unwrap();
        
        // Load
        let loaded = PlayerProfile::load_from_file(path).unwrap();
        
        // Verify
        assert_eq!(profile.version, loaded.version);
        assert_eq!(profile.name, loaded.name);
        assert_eq!(profile.settings.graphics.resolution, loaded.settings.graphics.resolution);
        
        // Cleanup
        std::fs::remove_file(path).unwrap();
    }
    
    #[test]
    fn test_corrupted_profile() {
        let path = "test_corrupted.toml";
        
        // Write invalid TOML
        std::fs::write(path, "invalid toml :::").unwrap();
        
        // Load should return default (not crash)
        let profile = PlayerProfile::load_from_file(path).unwrap();
        assert_eq!(profile.version, 1);
        
        // Cleanup
        std::fs::remove_file(path).unwrap();
    }
}
```

**Validation**:
```powershell
cargo test -p astraweave-persistence-player
```

**Success Criteria**:
- ✅ Save creates TOML file
- ✅ Load reads TOML file
- ✅ Roundtrip preserves data
- ✅ Corrupted files reset to default

---

## Task 3: Settings Integration (1-2 hours)

### Implementation

**File**: `crates/astraweave-persistence-player/src/settings.rs` (NEW)

```rust
use crate::{GameSettings, GraphicsSettings, AudioSettings, ControlSettings, QualityPreset};

impl GameSettings {
    /// Apply settings to game systems
    pub fn apply(&self) {
        self.graphics.apply();
        self.audio.apply();
        self.controls.apply();
    }
}

impl GraphicsSettings {
    /// Apply graphics settings to renderer
    pub fn apply(&self) {
        // TODO: Integrate with Phase 8.2 renderer
        // For now, just log
        println!("📊 Graphics Settings Applied:");
        println!("   Resolution: {}×{}", self.resolution.0, self.resolution.1);
        println!("   Quality: {:?}", self.quality);
        println!("   VSync: {}", self.vsync);
        println!("   Fullscreen: {}", self.fullscreen);
    }
}

impl AudioSettings {
    /// Apply audio settings to audio system
    pub fn apply(&self) {
        // TODO: Integrate with Phase 8.4 audio mixer
        // For now, just log
        println!("🔊 Audio Settings Applied:");
        println!("   Master: {:.0}%", self.master_volume * 100.0);
        println!("   Music: {:.0}%", self.music_volume * 100.0);
        println!("   SFX: {:.0}%", self.sfx_volume * 100.0);
        println!("   Voice: {:.0}%", self.voice_volume * 100.0);
        println!("   Muted: {}", self.muted);
    }
}

impl ControlSettings {
    /// Apply control settings to input system
    pub fn apply(&self) {
        // TODO: Integrate with input system
        // For now, just log
        println!("🎮 Control Settings Applied:");
        println!("   Mouse Sensitivity: {:.2}", self.mouse_sensitivity);
        println!("   Invert Y: {}", self.invert_y);
        println!("   Key Bindings: {} actions", self.key_bindings.len());
    }
}
```

**Validation**:
```powershell
cargo check -p astraweave-persistence-player
```

**Success Criteria**:
- ✅ Settings can be applied (even if just logging for now)
- ✅ Foundation for Phase 8.1/8.2/8.4 integration

---

## Task 4: Progression Tracking (1-2 hours)

### Implementation

**File**: `crates/astraweave-persistence-player/src/progression.rs` (NEW)

```rust
use crate::{PlayerProfile, PlayerStats, Unlocks};

impl PlayerProfile {
    /// Unlock an ability
    pub fn unlock_ability(&mut self, ability: &str) {
        if !self.unlocks.abilities.contains(&ability.to_string()) {
            self.unlocks.abilities.push(ability.to_string());
            println!("✨ Ability Unlocked: {}", ability);
        }
    }
    
    /// Unlock an item
    pub fn unlock_item(&mut self, item: &str) {
        if !self.unlocks.items.contains(&item.to_string()) {
            self.unlocks.items.push(item.to_string());
            println!("✨ Item Unlocked: {}", item);
        }
    }
    
    /// Unlock a level
    pub fn unlock_level(&mut self, level: &str) {
        if !self.unlocks.levels.contains(&level.to_string()) {
            self.unlocks.levels.push(level.to_string());
            println!("✨ Level Unlocked: {}", level);
        }
    }
    
    /// Grant achievement
    pub fn grant_achievement(&mut self, achievement: &str) {
        if !self.stats.achievements.contains(&achievement.to_string()) {
            self.stats.achievements.push(achievement.to_string());
            println!("🏆 Achievement Unlocked: {}", achievement);
        }
    }
    
    /// Increment kills
    pub fn record_kill(&mut self) {
        self.stats.enemies_defeated += 1;
    }
    
    /// Increment deaths
    pub fn record_death(&mut self) {
        self.stats.deaths += 1;
    }
    
    /// Add playtime (in seconds)
    pub fn add_playtime(&mut self, seconds: u64) {
        self.stats.playtime_seconds += seconds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unlock_ability() {
        let mut profile = PlayerProfile::default();
        
        profile.unlock_ability("Dash");
        assert_eq!(profile.unlocks.abilities.len(), 1);
        assert!(profile.unlocks.abilities.contains(&"Dash".to_string()));
        
        // Duplicate unlock should not add twice
        profile.unlock_ability("Dash");
        assert_eq!(profile.unlocks.abilities.len(), 1);
    }
    
    #[test]
    fn test_grant_achievement() {
        let mut profile = PlayerProfile::default();
        
        profile.grant_achievement("First Blood");
        assert_eq!(profile.stats.achievements.len(), 1);
        
        // Duplicate achievement should not add twice
        profile.grant_achievement("First Blood");
        assert_eq!(profile.stats.achievements.len(), 1);
    }
    
    #[test]
    fn test_stats_tracking() {
        let mut profile = PlayerProfile::default();
        
        profile.record_kill();
        profile.record_kill();
        profile.record_death();
        profile.add_playtime(3600); // 1 hour
        
        assert_eq!(profile.stats.enemies_defeated, 2);
        assert_eq!(profile.stats.deaths, 1);
        assert_eq!(profile.stats.playtime_seconds, 3600);
    }
}
```

**Autosave Integration**: `crates/astraweave-persistence-player/src/autosave.rs` (NEW)

```rust
use crate::PlayerProfile;
use std::time::{Duration, Instant};

pub struct AutoSaver {
    last_save: Instant,
    interval: Duration,
    dirty: bool,
}

impl AutoSaver {
    /// Create new autosaver (default: save every 30 seconds)
    pub fn new() -> Self {
        Self {
            last_save: Instant::now(),
            interval: Duration::from_secs(30),
            dirty: false,
        }
    }
    
    /// Mark profile as dirty (needs save)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// Update autosaver (call every frame)
    pub fn update(&mut self, profile: &PlayerProfile) {
        if !self.dirty {
            return;
        }
        
        if self.last_save.elapsed() >= self.interval {
            if let Err(e) = profile.quick_save() {
                eprintln!("⚠️  Autosave failed: {}", e);
            } else {
                println!("💾 Autosaved profile");
            }
            
            self.last_save = Instant::now();
            self.dirty = false;
        }
    }
}

impl Default for AutoSaver {
    fn default() -> Self {
        Self::new()
    }
}
```

**Validation**:
```powershell
cargo test -p astraweave-persistence-player
```

**Success Criteria**:
- ✅ Unlocks tracked (abilities, items, levels)
- ✅ Achievements granted (no duplicates)
- ✅ Stats tracked (kills, deaths, playtime)
- ✅ Autosave system works (30 sec interval)

---

## Validation & Testing

### Unit Tests

Run all tests:
```powershell
cargo test -p astraweave-persistence-player
```

**Expected Results**:
- ✅ `test_default_profile` - Default profile created
- ✅ `test_roundtrip` - Save → Load preserves data
- ✅ `test_corrupted_profile` - Corrupted files reset to default
- ✅ `test_unlock_ability` - Unlocks work (no duplicates)
- ✅ `test_grant_achievement` - Achievements work
- ✅ `test_stats_tracking` - Stats increment correctly

### Manual Testing

Create example:

**File**: `crates/astraweave-persistence-player/examples/profile_demo.rs`

```rust
use astraweave_persistence_player::PlayerProfile;

fn main() {
    // Load or create profile
    let mut profile = PlayerProfile::quick_load().unwrap();
    
    // Display current state
    println!("Player: {}", profile.name);
    println!("Playtime: {} seconds", profile.stats.playtime_seconds);
    println!("Kills: {}", profile.stats.enemies_defeated);
    println!("Deaths: {}", profile.stats.deaths);
    println!("Achievements: {}", profile.stats.achievements.len());
    
    // Make some changes
    profile.unlock_ability("Dash");
    profile.unlock_ability("Double Jump");
    profile.grant_achievement("First Blood");
    profile.record_kill();
    profile.add_playtime(120);
    
    // Save
    profile.quick_save().unwrap();
    
    println!("\n✅ Profile updated and saved!");
}
```

**Run**:
```powershell
cargo run -p astraweave-persistence-player --example profile_demo
```

**Verify**:
- Check `saves/player_profile.toml` exists
- Inspect file (should be human-readable TOML)
- Run again → should load existing profile
- Verify stats increment correctly

---

## Success Criteria

**Day 1 Complete When**:
- ✅ `PlayerProfile` struct compiles
- ✅ Save/load works (TOML format)
- ✅ Roundtrip test passes
- ✅ Corrupted profiles reset to default
- ✅ Settings can be applied (logging for now)
- ✅ Progression tracking works (unlocks, achievements, stats)
- ✅ Autosave system implemented (30 sec interval)
- ✅ All unit tests passing (6+)
- ✅ Manual example works

**Estimated Time**: 4-6 hours  
**Actual Time**: TBD (will track)

---

## Next Steps (Day 2)

After Day 1 complete:
- ✅ PlayerProfile system working
- ⏸️ Save Slot Management (Day 2 task)
  - Multiple save slots (3-10)
  - Metadata (timestamp, level, playtime)
  - Screenshot thumbnails
  - Save/load/delete APIs

**Files Created**:
- `crates/astraweave-persistence-player/Cargo.toml`
- `crates/astraweave-persistence-player/src/lib.rs`
- `crates/astraweave-persistence-player/src/settings.rs`
- `crates/astraweave-persistence-player/src/progression.rs`
- `crates/astraweave-persistence-player/src/autosave.rs`
- `crates/astraweave-persistence-player/examples/profile_demo.rs`

**Dependencies**:
- serde 1.0
- toml 0.8
- anyhow 1.0
- chrono 0.4

**Coverage**: 100% API surface (save/load/apply/unlocks/stats/autosave)
