//! Mutation-resistant comprehensive tests for astraweave-persistence-player.

#![allow(clippy::field_reassign_with_default)]

use astraweave_persistence_player::*;

// ═══════════════════════════════════════════════════════════════════════════
// PlayerProfile Default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn player_profile_default_version() {
    let p = PlayerProfile::default();
    assert_eq!(p.version, 1);
}

#[test]
fn player_profile_default_name() {
    let p = PlayerProfile::default();
    assert_eq!(p.name, "Player");
}

#[test]
fn player_profile_default_stats_playtime() {
    let p = PlayerProfile::default();
    assert_eq!(p.stats.playtime_seconds, 0);
}

#[test]
fn player_profile_default_stats_enemies() {
    let p = PlayerProfile::default();
    assert_eq!(p.stats.enemies_defeated, 0);
}

#[test]
fn player_profile_default_stats_deaths() {
    let p = PlayerProfile::default();
    assert_eq!(p.stats.deaths, 0);
}

#[test]
fn player_profile_default_stats_achievements_empty() {
    let p = PlayerProfile::default();
    assert!(p.stats.achievements.is_empty());
}

#[test]
fn player_profile_default_unlocks_abilities_empty() {
    let p = PlayerProfile::default();
    assert!(p.unlocks.abilities.is_empty());
}

#[test]
fn player_profile_default_unlocks_items_empty() {
    let p = PlayerProfile::default();
    assert!(p.unlocks.items.is_empty());
}

#[test]
fn player_profile_default_unlocks_levels_empty() {
    let p = PlayerProfile::default();
    assert!(p.unlocks.levels.is_empty());
}

#[test]
fn player_profile_default_path() {
    let path = PlayerProfile::default_path();
    let p = path.to_string_lossy();
    assert!(p.contains("saves"), "path should contain 'saves': {p}");
    assert!(
        p.contains("player_profile.toml"),
        "path should contain 'player_profile.toml': {p}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// GraphicsSettings Default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn graphics_default_resolution() {
    let g = GraphicsSettings::default();
    assert_eq!(g.resolution, (1920, 1080));
}

#[test]
fn graphics_default_quality() {
    let g = GraphicsSettings::default();
    assert!(matches!(g.quality, QualityPreset::High));
}

#[test]
fn graphics_default_vsync() {
    let g = GraphicsSettings::default();
    assert!(g.vsync);
}

#[test]
fn graphics_default_fullscreen() {
    let g = GraphicsSettings::default();
    assert!(!g.fullscreen);
}

// ═══════════════════════════════════════════════════════════════════════════
// AudioSettings Default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn audio_default_master_volume() {
    let a = AudioSettings::default();
    assert!((a.master_volume - 0.7).abs() < f32::EPSILON);
}

#[test]
fn audio_default_music_volume() {
    let a = AudioSettings::default();
    assert!((a.music_volume - 0.5).abs() < f32::EPSILON);
}

#[test]
fn audio_default_sfx_volume() {
    let a = AudioSettings::default();
    assert!((a.sfx_volume - 0.7).abs() < f32::EPSILON);
}

#[test]
fn audio_default_voice_volume() {
    let a = AudioSettings::default();
    assert!((a.voice_volume - 0.8).abs() < f32::EPSILON);
}

#[test]
fn audio_default_muted() {
    let a = AudioSettings::default();
    assert!(!a.muted);
}

// ═══════════════════════════════════════════════════════════════════════════
// ControlSettings Default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn controls_default_mouse_sensitivity() {
    let c = ControlSettings::default();
    assert!((c.mouse_sensitivity - 1.0).abs() < f32::EPSILON);
}

#[test]
fn controls_default_invert_y() {
    let c = ControlSettings::default();
    assert!(!c.invert_y);
}

#[test]
fn controls_default_key_bindings_forward() {
    let c = ControlSettings::default();
    assert_eq!(c.key_bindings["forward"], "W");
}

#[test]
fn controls_default_key_bindings_backward() {
    let c = ControlSettings::default();
    assert_eq!(c.key_bindings["backward"], "S");
}

#[test]
fn controls_default_key_bindings_left() {
    let c = ControlSettings::default();
    assert_eq!(c.key_bindings["left"], "A");
}

#[test]
fn controls_default_key_bindings_right() {
    let c = ControlSettings::default();
    assert_eq!(c.key_bindings["right"], "D");
}

#[test]
fn controls_default_key_bindings_jump() {
    let c = ControlSettings::default();
    assert_eq!(c.key_bindings["jump"], "Space");
}

#[test]
fn controls_default_key_bindings_interact() {
    let c = ControlSettings::default();
    assert_eq!(c.key_bindings["interact"], "E");
}

#[test]
fn controls_default_key_bindings_count() {
    let c = ControlSettings::default();
    assert_eq!(c.key_bindings.len(), 6);
}

// ═══════════════════════════════════════════════════════════════════════════
// QualityPreset variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn quality_preset_low_debug() {
    let q = QualityPreset::Low;
    let dbg = format!("{q:?}");
    assert!(dbg.contains("Low"));
}

#[test]
fn quality_preset_medium_debug() {
    let q = QualityPreset::Medium;
    let dbg = format!("{q:?}");
    assert!(dbg.contains("Medium"));
}

#[test]
fn quality_preset_high_debug() {
    let q = QualityPreset::High;
    let dbg = format!("{q:?}");
    assert!(dbg.contains("High"));
}

#[test]
fn quality_preset_ultra_debug() {
    let q = QualityPreset::Ultra;
    let dbg = format!("{q:?}");
    assert!(dbg.contains("Ultra"));
}

#[test]
fn quality_preset_copy() {
    let q = QualityPreset::High;
    let q2 = q;
    assert!(matches!(q2, QualityPreset::High));
    assert!(matches!(q, QualityPreset::High)); // original still usable (Copy)
}

// ═══════════════════════════════════════════════════════════════════════════
// Progression methods
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn unlock_ability() {
    let mut p = PlayerProfile::default();
    p.unlock_ability("fireball");
    assert_eq!(p.unlocks.abilities.len(), 1);
    assert_eq!(p.unlocks.abilities[0], "fireball");
}

#[test]
fn unlock_ability_multiple() {
    let mut p = PlayerProfile::default();
    p.unlock_ability("a");
    p.unlock_ability("b");
    assert_eq!(p.unlocks.abilities.len(), 2);
}

#[test]
fn unlock_item() {
    let mut p = PlayerProfile::default();
    p.unlock_item("sword");
    assert_eq!(p.unlocks.items.len(), 1);
    assert_eq!(p.unlocks.items[0], "sword");
}

#[test]
fn unlock_level() {
    let mut p = PlayerProfile::default();
    p.unlock_level("dungeon_1");
    assert_eq!(p.unlocks.levels.len(), 1);
    assert_eq!(p.unlocks.levels[0], "dungeon_1");
}

#[test]
fn grant_achievement() {
    let mut p = PlayerProfile::default();
    p.grant_achievement("first_blood");
    assert_eq!(p.stats.achievements.len(), 1);
    assert_eq!(p.stats.achievements[0], "first_blood");
}

#[test]
fn grant_multiple_achievements() {
    let mut p = PlayerProfile::default();
    p.grant_achievement("a");
    p.grant_achievement("b");
    p.grant_achievement("c");
    assert_eq!(p.stats.achievements.len(), 3);
}

#[test]
fn record_kill() {
    let mut p = PlayerProfile::default();
    assert_eq!(p.stats.enemies_defeated, 0);
    p.record_kill();
    assert_eq!(p.stats.enemies_defeated, 1);
    p.record_kill();
    assert_eq!(p.stats.enemies_defeated, 2);
}

#[test]
fn record_death() {
    let mut p = PlayerProfile::default();
    assert_eq!(p.stats.deaths, 0);
    p.record_death();
    assert_eq!(p.stats.deaths, 1);
}

#[test]
fn add_playtime() {
    let mut p = PlayerProfile::default();
    assert_eq!(p.stats.playtime_seconds, 0);
    p.add_playtime(100);
    assert_eq!(p.stats.playtime_seconds, 100);
    p.add_playtime(50);
    assert_eq!(p.stats.playtime_seconds, 150);
}

// ═══════════════════════════════════════════════════════════════════════════
// Save/Load roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_profile.toml");
    let mut p = PlayerProfile::default();
    p.name = "TestHero".into();
    p.stats.enemies_defeated = 42;
    p.settings.graphics.resolution = (2560, 1440);
    p.save_to_file(&path).unwrap();

    let loaded = PlayerProfile::load_from_file(&path).unwrap();
    assert_eq!(loaded.name, "TestHero");
    assert_eq!(loaded.stats.enemies_defeated, 42);
    assert_eq!(loaded.settings.graphics.resolution, (2560, 1440));
}

#[test]
fn save_load_preserves_unlocks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unlocks_test.toml");
    let mut p = PlayerProfile::default();
    p.unlock_ability("dash");
    p.unlock_item("shield");
    p.unlock_level("cave");
    p.save_to_file(&path).unwrap();

    let loaded = PlayerProfile::load_from_file(&path).unwrap();
    assert_eq!(loaded.unlocks.abilities, vec!["dash"]);
    assert_eq!(loaded.unlocks.items, vec!["shield"]);
    assert_eq!(loaded.unlocks.levels, vec!["cave"]);
}

#[test]
fn save_load_preserves_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings_test.toml");
    let mut p = PlayerProfile::default();
    p.settings.audio.master_volume = 0.3;
    p.settings.audio.muted = true;
    p.settings.controls.invert_y = true;
    p.settings.controls.mouse_sensitivity = 2.5;
    p.save_to_file(&path).unwrap();

    let loaded = PlayerProfile::load_from_file(&path).unwrap();
    assert!((loaded.settings.audio.master_volume - 0.3).abs() < f32::EPSILON);
    assert!(loaded.settings.audio.muted);
    assert!(loaded.settings.controls.invert_y);
    assert!((loaded.settings.controls.mouse_sensitivity - 2.5).abs() < f32::EPSILON);
}

#[test]
fn load_nonexistent_creates_default() {
    // load_from_file creates a default profile when file doesn't exist
    let result = PlayerProfile::load_from_file("/nonexistent/path/profile.toml");
    assert!(
        result.is_ok(),
        "load_from_file should create default on missing file"
    );
    let p = result.unwrap();
    assert_eq!(p.name, "Player");
    assert_eq!(p.version, 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// AutoSaver
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn autosaver_new() {
    let _as_ = AutoSaver::new();
    // Should not panic
}

#[test]
fn autosaver_default() {
    let _as_ = AutoSaver::default();
    // Default should be equivalent to new()
}

#[test]
fn autosaver_with_interval() {
    let _as_ = AutoSaver::with_interval(std::time::Duration::from_secs(60));
    // Should not panic
}

#[test]
fn autosaver_mark_dirty() {
    let mut saver = AutoSaver::new();
    saver.mark_dirty();
    // After marking dirty, update could save (but interval check prevents immediate save)
}

// ═══════════════════════════════════════════════════════════════════════════
// SaveSlotManager
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_slot_manager_with_defaults() {
    let _sm = SaveSlotManager::with_defaults();
    // Should not panic
}

#[test]
fn save_slot_manager_slot_exists_empty() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveSlotManager::new(10, dir.path().to_path_buf());
    assert!(!sm.slot_exists(0));
    assert!(!sm.slot_exists(9));
}

#[test]
fn save_slot_manager_save_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveSlotManager::new(10, dir.path().to_path_buf());
    let profile = PlayerProfile::default();
    sm.save_to_slot(
        0,
        vec![1, 2, 3],
        profile.clone(),
        "level_1".into(),
        Some("checkpoint_a".into()),
    )
    .unwrap();

    assert!(sm.slot_exists(0));
    let slot = sm.load_from_slot(0).unwrap();
    assert_eq!(slot.metadata.slot_id, 0);
    assert_eq!(slot.metadata.level_name, "level_1");
    assert_eq!(slot.metadata.checkpoint, Some("checkpoint_a".into()));
    assert_eq!(slot.world_state, vec![1, 2, 3]);
    assert_eq!(slot.player_profile.name, "Player");
}

#[test]
fn save_slot_manager_load_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveSlotManager::new(10, dir.path().to_path_buf());
    assert!(sm.load_from_slot(5).is_err());
}

#[test]
fn save_slot_manager_delete_slot() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveSlotManager::new(10, dir.path().to_path_buf());
    sm.save_to_slot(0, vec![], PlayerProfile::default(), "l".into(), None)
        .unwrap();
    assert!(sm.slot_exists(0));
    sm.delete_slot(0).unwrap();
    assert!(!sm.slot_exists(0));
}

#[test]
fn save_slot_manager_next_available_slot() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveSlotManager::new(3, dir.path().to_path_buf());
    assert_eq!(sm.next_available_slot(), Some(0));

    sm.save_to_slot(0, vec![], PlayerProfile::default(), "".into(), None)
        .unwrap();
    assert_eq!(sm.next_available_slot(), Some(1));

    sm.save_to_slot(1, vec![], PlayerProfile::default(), "".into(), None)
        .unwrap();
    assert_eq!(sm.next_available_slot(), Some(2));

    sm.save_to_slot(2, vec![], PlayerProfile::default(), "".into(), None)
        .unwrap();
    assert_eq!(sm.next_available_slot(), None, "all slots full → None");
}

#[test]
fn save_slot_manager_list_slots() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveSlotManager::new(10, dir.path().to_path_buf());
    sm.save_to_slot(0, vec![], PlayerProfile::default(), "a".into(), None)
        .unwrap();
    sm.save_to_slot(3, vec![], PlayerProfile::default(), "b".into(), None)
        .unwrap();
    let list = sm.list_slots().unwrap();
    assert_eq!(list.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// Clone & Debug traits
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn player_profile_clone_preserves_all() {
    let mut p = PlayerProfile::default();
    p.name = "Hero".into();
    p.stats.enemies_defeated = 10;
    p.settings.audio.muted = true;
    let p2 = p.clone();
    assert_eq!(p2.name, "Hero");
    assert_eq!(p2.stats.enemies_defeated, 10);
    assert!(p2.settings.audio.muted);
}

#[test]
fn player_profile_debug() {
    let p = PlayerProfile::default();
    let dbg = format!("{p:?}");
    assert!(dbg.contains("PlayerProfile"));
}

#[test]
fn game_settings_clone() {
    let gs = GameSettings::default();
    let gs2 = gs.clone();
    assert_eq!(gs2.graphics.resolution, (1920, 1080));
}

// ═══════════════════════════════════════════════════════════════════════════
// Serialization roundtrips
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn quality_preset_json_roundtrip() {
    let q = QualityPreset::Ultra;
    let json = serde_json::to_string(&q).unwrap();
    let back: QualityPreset = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, QualityPreset::Ultra));
}

#[test]
fn audio_settings_json_roundtrip() {
    let a = AudioSettings::default();
    let json = serde_json::to_string(&a).unwrap();
    let back: AudioSettings = serde_json::from_str(&json).unwrap();
    assert!((back.master_volume - 0.7).abs() < f32::EPSILON);
    assert!(!back.muted);
}

#[test]
fn control_settings_json_roundtrip() {
    let c = ControlSettings::default();
    let json = serde_json::to_string(&c).unwrap();
    let back: ControlSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back.key_bindings["forward"], "W");
    assert!(!back.invert_y);
}

#[test]
fn graphics_settings_json_roundtrip() {
    let g = GraphicsSettings::default();
    let json = serde_json::to_string(&g).unwrap();
    let back: GraphicsSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back.resolution, (1920, 1080));
    assert!(back.vsync);
}

#[test]
fn player_stats_json_roundtrip() {
    let mut ps = PlayerStats::default();
    ps.enemies_defeated = 55;
    ps.deaths = 3;
    let json = serde_json::to_string(&ps).unwrap();
    let back: PlayerStats = serde_json::from_str(&json).unwrap();
    assert_eq!(back.enemies_defeated, 55);
    assert_eq!(back.deaths, 3);
}

// ═══════════════════════════════════════════════════════════════════════════
// Combined progression
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn combined_progression_save_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("combined.toml");
    let mut p = PlayerProfile::default();
    p.name = "CombinedHero".into();
    p.record_kill();
    p.record_kill();
    p.record_death();
    p.add_playtime(3600);
    p.unlock_ability("dash");
    p.unlock_item("potion");
    p.unlock_level("world_2");
    p.grant_achievement("speedrun");
    p.save_to_file(&path).unwrap();

    let loaded = PlayerProfile::load_from_file(&path).unwrap();
    assert_eq!(loaded.name, "CombinedHero");
    assert_eq!(loaded.stats.enemies_defeated, 2);
    assert_eq!(loaded.stats.deaths, 1);
    assert_eq!(loaded.stats.playtime_seconds, 3600);
    assert_eq!(loaded.unlocks.abilities, vec!["dash"]);
    assert_eq!(loaded.unlocks.items, vec!["potion"]);
    assert_eq!(loaded.unlocks.levels, vec!["world_2"]);
    assert_eq!(loaded.stats.achievements, vec!["speedrun"]);
}
