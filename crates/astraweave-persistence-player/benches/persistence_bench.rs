//! # Player Persistence Benchmark Suite
//!
//! Comprehensive benchmarks for the astraweave-persistence-player crate covering:
//! - Profile serialization/deserialization (TOML)
//! - Settings operations (graphics, audio, controls)
//! - Stats tracking operations
//! - Unlocks management
//! - Profile construction and default values
//!
//! Run with: `cargo bench -p astraweave-persistence-player`

#![allow(unused_imports, unused_mut, clippy::field_reassign_with_default)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;

use astraweave_persistence_player::{
    AudioSettings, ControlSettings, GameSettings, GraphicsSettings, PlayerProfile, PlayerStats,
    QualityPreset, Unlocks,
};

// ============================================================================
// CORRECTNESS ASSERTION HELPERS
// ============================================================================

/// Assert profile is valid
fn assert_profile_valid(profile: &PlayerProfile) {
    assert!(profile.version > 0, "Profile version should be positive");
    assert!(!profile.name.is_empty(), "Profile name should not be empty");
    assert!(
        profile.settings.audio.master_volume >= 0.0 && profile.settings.audio.master_volume <= 1.0,
        "Master volume should be in valid range"
    );
}

/// Assert TOML string is valid
fn assert_toml_valid(toml_str: &str) {
    assert!(!toml_str.is_empty(), "TOML string should not be empty");
    assert!(
        toml_str.contains("[settings.graphics]"),
        "TOML should contain graphics section"
    );
}

// ============================================================================
// TEST DATA GENERATORS
// ============================================================================

/// Create a default profile
fn create_default_profile() -> PlayerProfile {
    PlayerProfile::default()
}

/// Create a profile with custom stats
fn create_profile_with_stats(playtime: u64, enemies: u32, deaths: u32) -> PlayerProfile {
    let mut profile = PlayerProfile::default();
    profile.stats.playtime_seconds = playtime;
    profile.stats.enemies_defeated = enemies;
    profile.stats.deaths = deaths;
    profile
}

/// Create a profile with many achievements
fn create_profile_with_achievements(achievement_count: usize) -> PlayerProfile {
    let mut profile = PlayerProfile::default();
    for i in 0..achievement_count {
        profile
            .stats
            .achievements
            .push(format!("achievement_{}", i));
    }
    profile
}

/// Create a profile with many unlocks
fn create_profile_with_unlocks(ability_count: usize, item_count: usize) -> PlayerProfile {
    let mut profile = PlayerProfile::default();
    for i in 0..ability_count {
        profile.unlocks.abilities.push(format!("ability_{}", i));
    }
    for i in 0..item_count {
        profile.unlocks.items.push(format!("item_{}", i));
    }
    profile
}

/// Create a profile with many key bindings
fn create_profile_with_keybindings(binding_count: usize) -> PlayerProfile {
    let mut profile = PlayerProfile::default();
    for i in 0..binding_count {
        profile
            .settings
            .controls
            .key_bindings
            .insert(format!("action_{}", i), format!("Key{}", i));
    }
    profile
}

/// Create a fully populated profile (worst-case for serialization)
fn create_complex_profile() -> PlayerProfile {
    let mut profile = PlayerProfile::default();
    profile.name = "ComplexPlayer".to_string();
    profile.stats.playtime_seconds = 1_000_000;
    profile.stats.enemies_defeated = 50_000;
    profile.stats.deaths = 1_234;

    // Add many achievements
    for i in 0..100 {
        profile
            .stats
            .achievements
            .push(format!("achievement_{}", i));
    }

    // Add many unlocks
    for i in 0..50 {
        profile.unlocks.abilities.push(format!("ability_{}", i));
    }
    for i in 0..100 {
        profile.unlocks.items.push(format!("item_{}", i));
    }
    for i in 0..20 {
        profile.unlocks.levels.push(format!("level_{}", i));
    }

    // Add many key bindings
    for i in 0..50 {
        profile
            .settings
            .controls
            .key_bindings
            .insert(format!("action_{}", i), format!("Key{}", i));
    }

    profile
}

// ============================================================================
// PROFILE CONSTRUCTION BENCHMARKS
// ============================================================================

fn bench_profile_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_profile_construction");

    // Default profile construction
    group.bench_function("construct_default", |b| {
        b.iter(|| {
            let profile = PlayerProfile::default();
            assert_profile_valid(&profile);
            black_box(profile)
        })
    });

    // Settings default construction
    group.bench_function("construct_settings_default", |b| {
        b.iter(|| {
            let settings = GameSettings::default();
            assert!(settings.audio.master_volume >= 0.0 && settings.audio.master_volume <= 1.0);
            black_box(settings)
        })
    });

    // Graphics settings construction
    group.bench_function("construct_graphics_default", |b| {
        b.iter(|| {
            let graphics = GraphicsSettings::default();
            assert_eq!(graphics.resolution, (1920, 1080));
            black_box(graphics)
        })
    });

    // Audio settings construction
    group.bench_function("construct_audio_default", |b| {
        b.iter(|| {
            let audio = AudioSettings::default();
            assert!(audio.master_volume >= 0.0);
            black_box(audio)
        })
    });

    // Control settings construction (with HashMap)
    group.bench_function("construct_controls_default", |b| {
        b.iter(|| {
            let controls = ControlSettings::default();
            assert!(!controls.key_bindings.is_empty());
            black_box(controls)
        })
    });

    // Stats construction
    group.bench_function("construct_stats_default", |b| {
        b.iter(|| {
            let stats = PlayerStats::default();
            assert_eq!(stats.playtime_seconds, 0);
            black_box(stats)
        })
    });

    // Unlocks construction
    group.bench_function("construct_unlocks_default", |b| {
        b.iter(|| {
            let unlocks = Unlocks::default();
            assert!(unlocks.abilities.is_empty());
            black_box(unlocks)
        })
    });

    group.finish();
}

// ============================================================================
// SERIALIZATION BENCHMARKS
// ============================================================================

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_profile_serialization");

    // Default profile serialization
    let default_profile = create_default_profile();
    group.bench_function("serialize_default", |b| {
        b.iter(|| {
            let toml_str = toml::to_string(black_box(&default_profile)).unwrap();
            assert_toml_valid(&toml_str);
            black_box(toml_str)
        })
    });

    // Pretty print serialization
    group.bench_function("serialize_pretty_default", |b| {
        b.iter(|| {
            let toml_str = toml::to_string_pretty(black_box(&default_profile)).unwrap();
            assert_toml_valid(&toml_str);
            black_box(toml_str)
        })
    });

    // Profile with stats serialization
    let stats_profile = create_profile_with_stats(100_000, 5_000, 100);
    group.bench_function("serialize_with_stats", |b| {
        b.iter(|| {
            let toml_str = toml::to_string(black_box(&stats_profile)).unwrap();
            assert_toml_valid(&toml_str);
            black_box(toml_str)
        })
    });

    // Profiles with varying achievement counts
    for achievement_count in [10, 50, 100, 500] {
        let profile = create_profile_with_achievements(achievement_count);
        group.throughput(Throughput::Elements(achievement_count as u64));
        group.bench_with_input(
            BenchmarkId::new("serialize_achievements", achievement_count),
            &profile,
            |b, profile| {
                b.iter(|| {
                    let toml_str = toml::to_string(black_box(profile)).unwrap();
                    assert_toml_valid(&toml_str);
                    black_box(toml_str)
                })
            },
        );
    }

    // Complex profile serialization
    let complex_profile = create_complex_profile();
    group.bench_function("serialize_complex", |b| {
        b.iter(|| {
            let toml_str = toml::to_string(black_box(&complex_profile)).unwrap();
            assert_toml_valid(&toml_str);
            black_box(toml_str)
        })
    });

    group.finish();
}

// ============================================================================
// DESERIALIZATION BENCHMARKS
// ============================================================================

fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_profile_deserialization");

    // Default profile deserialization
    let default_profile = create_default_profile();
    let default_toml = toml::to_string(&default_profile).unwrap();
    group.bench_function("deserialize_default", |b| {
        b.iter(|| {
            let profile: PlayerProfile = toml::from_str(black_box(&default_toml)).unwrap();
            assert_profile_valid(&profile);
            black_box(profile)
        })
    });

    // Profile with stats deserialization
    let stats_profile = create_profile_with_stats(100_000, 5_000, 100);
    let stats_toml = toml::to_string(&stats_profile).unwrap();
    group.bench_function("deserialize_with_stats", |b| {
        b.iter(|| {
            let profile: PlayerProfile = toml::from_str(black_box(&stats_toml)).unwrap();
            assert_profile_valid(&profile);
            black_box(profile)
        })
    });

    // Profiles with varying achievement counts
    for achievement_count in [10, 50, 100, 500] {
        let profile = create_profile_with_achievements(achievement_count);
        let toml_str = toml::to_string(&profile).unwrap();
        group.throughput(Throughput::Elements(achievement_count as u64));
        group.bench_with_input(
            BenchmarkId::new("deserialize_achievements", achievement_count),
            &toml_str,
            |b, toml_str| {
                b.iter(|| {
                    let profile: PlayerProfile = toml::from_str(black_box(toml_str)).unwrap();
                    assert_profile_valid(&profile);
                    black_box(profile)
                })
            },
        );
    }

    // Complex profile deserialization
    let complex_profile = create_complex_profile();
    let complex_toml = toml::to_string(&complex_profile).unwrap();
    group.bench_function("deserialize_complex", |b| {
        b.iter(|| {
            let profile: PlayerProfile = toml::from_str(black_box(&complex_toml)).unwrap();
            assert_profile_valid(&profile);
            black_box(profile)
        })
    });

    group.finish();
}

// ============================================================================
// ROUNDTRIP BENCHMARKS
// ============================================================================

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_profile_roundtrip");

    // Default profile roundtrip
    let default_profile = create_default_profile();
    group.bench_function("roundtrip_default", |b| {
        b.iter(|| {
            let toml_str = toml::to_string(black_box(&default_profile)).unwrap();
            let restored: PlayerProfile = toml::from_str(&toml_str).unwrap();
            assert_eq!(default_profile.name, restored.name);
            black_box(restored)
        })
    });

    // Complex profile roundtrip
    let complex_profile = create_complex_profile();
    group.bench_function("roundtrip_complex", |b| {
        b.iter(|| {
            let toml_str = toml::to_string(black_box(&complex_profile)).unwrap();
            let restored: PlayerProfile = toml::from_str(&toml_str).unwrap();
            assert_eq!(complex_profile.name, restored.name);
            assert_eq!(
                complex_profile.stats.achievements.len(),
                restored.stats.achievements.len()
            );
            black_box(restored)
        })
    });

    group.finish();
}

// ============================================================================
// SETTINGS OPERATIONS BENCHMARKS
// ============================================================================

fn bench_settings_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_settings_operations");

    // Key binding lookup
    let profile = create_profile_with_keybindings(50);
    group.bench_function("keybinding_lookup", |b| {
        b.iter(|| {
            let binding = profile
                .settings
                .controls
                .key_bindings
                .get(black_box("action_25"));
            assert!(binding.is_some());
            black_box(binding)
        })
    });

    // Key binding insert
    let mut profile_mut = create_profile_with_keybindings(10);
    group.bench_function("keybinding_insert", |b| {
        b.iter(|| {
            let mut profile = profile_mut.clone();
            profile.settings.controls.key_bindings.insert(
                black_box("new_action".to_string()),
                black_box("KeyX".to_string()),
            );
            black_box(profile)
        })
    });

    // Settings clone
    let settings = GameSettings::default();
    group.bench_function("settings_clone", |b| {
        b.iter(|| {
            let cloned = black_box(&settings).clone();
            assert_eq!(cloned.graphics.resolution, settings.graphics.resolution);
            black_box(cloned)
        })
    });

    // Quality preset comparison
    group.bench_function("quality_preset_compare", |b| {
        let preset1 = QualityPreset::High;
        let preset2 = QualityPreset::Ultra;
        b.iter(|| {
            let same = std::mem::discriminant(&preset1) == std::mem::discriminant(&preset2);
            black_box(same)
        })
    });

    group.finish();
}

// ============================================================================
// STATS OPERATIONS BENCHMARKS
// ============================================================================

fn bench_stats_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_stats_operations");

    // Achievement lookup (Vec contains)
    let profile = create_profile_with_achievements(100);
    group.bench_function("achievement_contains_100", |b| {
        b.iter(|| {
            let has = profile
                .stats
                .achievements
                .contains(&black_box("achievement_50".to_string()));
            assert!(has);
            black_box(has)
        })
    });

    // Achievement add
    let mut profile_mut = create_profile_with_achievements(50);
    group.bench_function("achievement_add", |b| {
        b.iter(|| {
            let mut profile = profile_mut.clone();
            profile
                .stats
                .achievements
                .push(black_box("new_achievement".to_string()));
            black_box(profile)
        })
    });

    // Stats clone
    let stats = PlayerStats::default();
    group.bench_function("stats_clone", |b| {
        b.iter(|| {
            let cloned = black_box(&stats).clone();
            assert_eq!(cloned.playtime_seconds, stats.playtime_seconds);
            black_box(cloned)
        })
    });

    // Stats update (multiple fields)
    let mut profile_mut = create_default_profile();
    group.bench_function("stats_update_multiple", |b| {
        b.iter(|| {
            let mut profile = profile_mut.clone();
            profile.stats.playtime_seconds += 1;
            profile.stats.enemies_defeated += 1;
            profile.stats.deaths += 1;
            black_box(profile)
        })
    });

    group.finish();
}

// ============================================================================
// UNLOCKS OPERATIONS BENCHMARKS
// ============================================================================

fn bench_unlocks_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_unlocks_operations");

    // Ability unlock check
    let profile = create_profile_with_unlocks(50, 100);
    group.bench_function("ability_contains", |b| {
        b.iter(|| {
            let has = profile
                .unlocks
                .abilities
                .contains(&black_box("ability_25".to_string()));
            assert!(has);
            black_box(has)
        })
    });

    // Item unlock check
    group.bench_function("item_contains", |b| {
        b.iter(|| {
            let has = profile
                .unlocks
                .items
                .contains(&black_box("item_50".to_string()));
            assert!(has);
            black_box(has)
        })
    });

    // Add ability
    let mut profile_mut = create_profile_with_unlocks(10, 10);
    group.bench_function("ability_add", |b| {
        b.iter(|| {
            let mut profile = profile_mut.clone();
            profile
                .unlocks
                .abilities
                .push(black_box("new_ability".to_string()));
            black_box(profile)
        })
    });

    // Unlocks clone
    let unlocks = Unlocks::default();
    group.bench_function("unlocks_clone", |b| {
        b.iter(|| {
            let cloned = black_box(&unlocks).clone();
            black_box(cloned)
        })
    });

    group.finish();
}

// ============================================================================
// PROFILE CLONE BENCHMARKS
// ============================================================================

fn bench_profile_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_profile_clone");

    // Default profile clone
    let default_profile = create_default_profile();
    group.bench_function("clone_default", |b| {
        b.iter(|| {
            let cloned = black_box(&default_profile).clone();
            assert_profile_valid(&cloned);
            black_box(cloned)
        })
    });

    // Profile with varying sizes
    for (ability_count, item_count) in [(10, 10), (50, 100), (100, 200)] {
        let profile = create_profile_with_unlocks(ability_count, item_count);
        let total = ability_count + item_count;
        group.throughput(Throughput::Elements(total as u64));
        group.bench_with_input(
            BenchmarkId::new("clone_unlocks", total),
            &profile,
            |b, profile| {
                b.iter(|| {
                    let cloned = black_box(profile).clone();
                    assert_profile_valid(&cloned);
                    black_box(cloned)
                })
            },
        );
    }

    // Complex profile clone
    let complex_profile = create_complex_profile();
    group.bench_function("clone_complex", |b| {
        b.iter(|| {
            let cloned = black_box(&complex_profile).clone();
            assert_profile_valid(&cloned);
            black_box(cloned)
        })
    });

    group.finish();
}

// ============================================================================
// BATCH OPERATIONS BENCHMARKS
// ============================================================================

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("player_batch_operations");

    // Batch profile creation
    for count in [10, 50, 100] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_create_profiles", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let profiles: Vec<PlayerProfile> =
                        (0..count).map(|_| PlayerProfile::default()).collect();
                    assert_eq!(profiles.len(), count);
                    black_box(profiles)
                })
            },
        );
    }

    // Batch serialization
    for count in [10, 50, 100] {
        let profiles: Vec<PlayerProfile> = (0..count).map(|_| PlayerProfile::default()).collect();
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_serialize_profiles", count),
            &profiles,
            |b, profiles| {
                b.iter(|| {
                    let toml_strings: Vec<String> = profiles
                        .iter()
                        .map(|p| toml::to_string(p).unwrap())
                        .collect();
                    assert_eq!(toml_strings.len(), count);
                    black_box(toml_strings)
                })
            },
        );
    }

    // Batch deserialization
    for count in [10, 50, 100] {
        let profiles: Vec<PlayerProfile> = (0..count).map(|_| PlayerProfile::default()).collect();
        let toml_strings: Vec<String> = profiles
            .iter()
            .map(|p| toml::to_string(p).unwrap())
            .collect();
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_deserialize_profiles", count),
            &toml_strings,
            |b, toml_strings| {
                b.iter(|| {
                    let restored: Vec<PlayerProfile> = toml_strings
                        .iter()
                        .map(|s| toml::from_str(s).unwrap())
                        .collect();
                    assert_eq!(restored.len(), count);
                    black_box(restored)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// CRITERION GROUP REGISTRATION
// ============================================================================

criterion_group!(
    benches,
    bench_profile_construction,
    bench_serialization,
    bench_deserialization,
    bench_roundtrip,
    bench_settings_operations,
    bench_stats_operations,
    bench_unlocks_operations,
    bench_profile_clone,
    bench_batch_operations,
);

criterion_main!(benches);
