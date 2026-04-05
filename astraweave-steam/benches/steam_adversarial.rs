//! Adversarial Steam Integration Benchmarks
//!
//! Stress testing for Steamworks SDK integration, achievements, cloud saves, and stats.

#![allow(dead_code, unused_variables, clippy::type_complexity)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

// ============================================================================
// LOCAL TYPES (Mirror astraweave-steam API)
// ============================================================================

/// Steam user ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SteamId(u64);

/// Achievement definition
#[derive(Clone, Debug)]
struct Achievement {
    api_name: String,
    display_name: String,
    description: String,
    hidden: bool,
    unlocked: bool,
    unlock_time: Option<u64>,
    progress: Option<(u32, u32)>, // (current, max)
}

/// Player statistics
#[derive(Clone, Debug)]
struct PlayerStat {
    name: String,
    value: StatValue,
    min: Option<f64>,
    max: Option<f64>,
}

#[derive(Clone, Debug)]
enum StatValue {
    Int(i32),
    Float(f32),
    AvgRate { value: f32, duration: f32 },
}

/// Cloud save file metadata
#[derive(Clone, Debug)]
struct CloudFile {
    name: String,
    size: usize,
    timestamp: u64,
    data: Vec<u8>,
}

/// Leaderboard entry
#[derive(Clone, Debug)]
struct LeaderboardEntry {
    steam_id: SteamId,
    rank: u32,
    score: i32,
    details: Vec<i32>,
}

/// Leaderboard
#[derive(Clone, Debug)]
struct Leaderboard {
    name: String,
    sort_method: LeaderboardSort,
    display_type: LeaderboardDisplay,
    entries: Vec<LeaderboardEntry>,
}

#[derive(Clone, Debug)]
enum LeaderboardSort {
    Ascending,
    Descending,
}

#[derive(Clone, Debug)]
enum LeaderboardDisplay {
    Numeric,
    TimeSeconds,
    TimeMilliseconds,
}

/// Workshop item
#[derive(Clone, Debug)]
struct WorkshopItem {
    id: u64,
    title: String,
    description: String,
    tags: Vec<String>,
    file_size: usize,
    preview_url: String,
    votes_up: u32,
    votes_down: u32,
    subscribed: bool,
}

/// Simulated Steam client
#[derive(Default)]
struct MockSteamClient {
    user_id: Option<SteamId>,
    achievements: HashMap<String, Achievement>,
    stats: HashMap<String, PlayerStat>,
    cloud_files: HashMap<String, CloudFile>,
    leaderboards: HashMap<String, Leaderboard>,
    workshop_items: Vec<WorkshopItem>,
}

impl MockSteamClient {
    fn with_user(user_id: SteamId) -> Self {
        Self {
            user_id: Some(user_id),
            ..Default::default()
        }
    }

    fn is_logged_in(&self) -> bool {
        self.user_id.is_some()
    }
}

// ============================================================================
// CATEGORY 1: ACHIEVEMENT OPERATIONS
// ============================================================================

fn bench_achievements(c: &mut Criterion) {
    let mut group = c.benchmark_group("steam_adversarial/achievements");

    // Test 1: Achievement definition loading
    group.bench_function("load_achievements_500", |bencher| {
        bencher.iter(|| {
            let achievements: Vec<Achievement> = (0..500)
                .map(|i| Achievement {
                    api_name: format!("ACH_{}", i),
                    display_name: format!("Achievement {}", i),
                    description: format!("Complete challenge {} to unlock this achievement", i),
                    hidden: i % 10 == 0,
                    unlocked: false,
                    unlock_time: None,
                    progress: if i % 5 == 0 { Some((0, 100)) } else { None },
                })
                .collect();

            black_box(achievements.len())
        });
    });

    // Test 2: Achievement unlock
    group.bench_function("unlock_achievements_1000", |bencher| {
        let mut client = MockSteamClient::with_user(SteamId(12345));

        for i in 0..200 {
            client.achievements.insert(
                format!("ACH_{}", i),
                Achievement {
                    api_name: format!("ACH_{}", i),
                    display_name: format!("Achievement {}", i),
                    description: String::new(),
                    hidden: false,
                    unlocked: false,
                    unlock_time: None,
                    progress: None,
                },
            );
        }

        bencher.iter(|| {
            let mut unlocked = 0;

            for i in 0..1000 {
                let api_name = format!("ACH_{}", i % 200);

                if let Some(ach) = client.achievements.get_mut(&api_name) {
                    if !ach.unlocked {
                        ach.unlocked = true;
                        ach.unlock_time = Some(i as u64);
                        unlocked += 1;
                    }
                }
            }

            black_box(unlocked)
        });
    });

    // Test 3: Achievement progress update
    group.bench_function("progress_update_10000", |bencher| {
        let mut client = MockSteamClient::with_user(SteamId(12345));

        for i in 0..100 {
            client.achievements.insert(
                format!("PROG_{}", i),
                Achievement {
                    api_name: format!("PROG_{}", i),
                    display_name: format!("Progress Achievement {}", i),
                    description: String::new(),
                    hidden: false,
                    unlocked: false,
                    unlock_time: None,
                    progress: Some((0, 100)),
                },
            );
        }

        bencher.iter(|| {
            let mut completed = 0;

            for i in 0..10000 {
                let api_name = format!("PROG_{}", i % 100);
                let increment = (i % 10 + 1) as u32;

                if let Some(ach) = client.achievements.get_mut(&api_name) {
                    if let Some((current, max)) = &mut ach.progress {
                        *current = (*current + increment).min(*max);

                        if *current >= *max && !ach.unlocked {
                            ach.unlocked = true;
                            ach.unlock_time = Some(i as u64);
                            completed += 1;
                        }
                    }
                }
            }

            black_box(completed)
        });
    });

    // Test 4: Achievement filtering and display
    group.bench_function("filter_achievements_5000", |bencher| {
        let achievements: Vec<Achievement> = (0..500)
            .map(|i| Achievement {
                api_name: format!("ACH_{}", i),
                display_name: format!("Achievement {}", i),
                description: format!("Description {}", i),
                hidden: i % 10 == 0,
                unlocked: i % 3 == 0,
                unlock_time: if i % 3 == 0 { Some(i as u64) } else { None },
                progress: if i % 5 == 0 {
                    Some((i as u32 % 100, 100))
                } else {
                    None
                },
            })
            .collect();

        bencher.iter(|| {
            for _ in 0..10 {
                // Filter unlocked
                let unlocked: Vec<&Achievement> =
                    achievements.iter().filter(|a| a.unlocked).collect();

                // Filter locked (non-hidden)
                let locked_visible: Vec<&Achievement> = achievements
                    .iter()
                    .filter(|a| !a.unlocked && !a.hidden)
                    .collect();

                // Filter in-progress
                let in_progress: Vec<&Achievement> = achievements
                    .iter()
                    .filter(|a| {
                        if let Some((current, max)) = a.progress {
                            current > 0 && current < max
                        } else {
                            false
                        }
                    })
                    .collect();

                black_box((unlocked.len(), locked_visible.len(), in_progress.len()));
            }
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 2: STATISTICS TRACKING
// ============================================================================

fn bench_statistics(c: &mut Criterion) {
    let mut group = c.benchmark_group("steam_adversarial/statistics");

    // Test 1: Stat initialization
    group.bench_function("init_stats_200", |bencher| {
        bencher.iter(|| {
            let stats: Vec<PlayerStat> = (0..200)
                .map(|i| {
                    let value = match i % 3 {
                        0 => StatValue::Int(0),
                        1 => StatValue::Float(0.0),
                        _ => StatValue::AvgRate {
                            value: 0.0,
                            duration: 0.0,
                        },
                    };

                    PlayerStat {
                        name: format!("stat_{}", i),
                        value,
                        min: Some(0.0),
                        max: Some(1_000_000.0),
                    }
                })
                .collect();

            black_box(stats.len())
        });
    });

    // Test 2: Stat updates (high frequency)
    group.bench_function("stat_updates_50000", |bencher| {
        let mut client = MockSteamClient::with_user(SteamId(12345));

        for i in 0..100 {
            client.stats.insert(
                format!("stat_{}", i),
                PlayerStat {
                    name: format!("stat_{}", i),
                    value: StatValue::Int(0),
                    min: Some(0.0),
                    max: Some(1_000_000.0),
                },
            );
        }

        bencher.iter(|| {
            for i in 0..50000 {
                let stat_name = format!("stat_{}", i % 100);

                if let Some(stat) = client.stats.get_mut(&stat_name) {
                    match &mut stat.value {
                        StatValue::Int(v) => *v += 1,
                        StatValue::Float(v) => *v += 0.1,
                        StatValue::AvgRate { value, duration } => {
                            *value += 1.0;
                            *duration += 0.016;
                        }
                    }
                }
            }

            black_box(client.stats.len())
        });
    });

    // Test 3: Stat aggregation
    group.bench_function("stat_aggregation_10000", |bencher| {
        let stats: Vec<PlayerStat> = (0..10000)
            .map(|i| PlayerStat {
                name: format!("stat_{}", i),
                value: StatValue::Int(i % 1000),
                min: Some(0.0),
                max: Some(1000.0),
            })
            .collect();

        bencher.iter(|| {
            // Calculate totals, averages, etc.
            let total: i64 = stats
                .iter()
                .map(|s| match &s.value {
                    StatValue::Int(v) => *v as i64,
                    StatValue::Float(v) => *v as i64,
                    StatValue::AvgRate { value, .. } => *value as i64,
                })
                .sum();

            let avg = total as f64 / stats.len() as f64;

            let max = stats
                .iter()
                .map(|s| match &s.value {
                    StatValue::Int(v) => *v as i64,
                    StatValue::Float(v) => *v as i64,
                    StatValue::AvgRate { value, .. } => *value as i64,
                })
                .max()
                .unwrap_or(0);

            black_box((total, avg, max))
        });
    });

    // Test 4: Stat validation
    group.bench_function("stat_validation_20000", |bencher| {
        let updates: Vec<(String, i32)> = (0..20000)
            .map(|i| (format!("stat_{}", i % 50), (i * 137) % 2000 - 500))
            .collect();

        let stat_limits: HashMap<String, (i32, i32)> = (0..50)
            .map(|i| (format!("stat_{}", i), (0, 1000)))
            .collect();

        bencher.iter(|| {
            let validated: Vec<(String, i32, bool)> = updates
                .iter()
                .map(|(name, value)| {
                    let valid = if let Some((min, max)) = stat_limits.get(name) {
                        *value >= *min && *value <= *max
                    } else {
                        false
                    };

                    let clamped = if let Some((min, max)) = stat_limits.get(name) {
                        (*value).clamp(*min, *max)
                    } else {
                        *value
                    };

                    (name.clone(), clamped, valid)
                })
                .collect();

            let invalid_count = validated.iter().filter(|(_, _, v)| !v).count();
            black_box(invalid_count)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 3: CLOUD SAVES
// ============================================================================

fn bench_cloud_saves(c: &mut Criterion) {
    let mut group = c.benchmark_group("steam_adversarial/cloud_saves");

    // Test 1: Save file creation
    for size in [1024, 16384, 65536, 262144] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("create_save", size),
            &size,
            |bencher, &size| {
                bencher.iter(|| {
                    let file = CloudFile {
                        name: "savegame.dat".to_string(),
                        size,
                        timestamp: 12345678,
                        data: vec![0u8; size],
                    };

                    black_box(file.data.len())
                });
            },
        );
    }

    // Test 2: Save file serialization
    group.bench_function("serialize_save_data_1000", |bencher| {
        #[derive(Clone)]
        struct SaveData {
            player_name: String,
            level: u32,
            experience: u64,
            position: (f32, f32, f32),
            inventory: Vec<(u32, u32)>, // (item_id, count)
            achievements: Vec<String>,
            playtime_seconds: u64,
        }

        let saves: Vec<SaveData> = (0..1000)
            .map(|i| SaveData {
                player_name: format!("Player_{}", i),
                level: (i % 100) as u32 + 1,
                experience: i as u64 * 1000,
                position: (i as f32, i as f32 * 0.5, i as f32 * 0.1),
                inventory: (0..50).map(|j| (j as u32, (i + j) as u32 % 100)).collect(),
                achievements: (0..20).map(|j| format!("ACH_{}_{}", i, j)).collect(),
                playtime_seconds: i as u64 * 3600,
            })
            .collect();

        bencher.iter(|| {
            // Simulate serialization (convert to bytes)
            let serialized: Vec<Vec<u8>> = saves
                .iter()
                .map(|save| {
                    let mut data = Vec::new();

                    // Simple binary format
                    data.extend(save.player_name.as_bytes());
                    data.push(0); // null terminator
                    data.extend(&save.level.to_le_bytes());
                    data.extend(&save.experience.to_le_bytes());
                    data.extend(&save.position.0.to_le_bytes());
                    data.extend(&save.position.1.to_le_bytes());
                    data.extend(&save.position.2.to_le_bytes());
                    data.extend(&(save.inventory.len() as u32).to_le_bytes());
                    for (item_id, count) in &save.inventory {
                        data.extend(&item_id.to_le_bytes());
                        data.extend(&count.to_le_bytes());
                    }

                    data
                })
                .collect();

            let total_size: usize = serialized.iter().map(|s| s.len()).sum();
            black_box(total_size)
        });
    });

    // Test 3: Cloud file management
    group.bench_function("file_management_5000", |bencher| {
        let quota = 100 * 1024 * 1024usize; // 100 MB quota

        bencher.iter(|| {
            let mut client = MockSteamClient::with_user(SteamId(12345));
            let mut total_size = 0usize;
            let mut deleted = 0;

            for i in 0..5000 {
                let file_size = 1024 + (i % 10000);

                // Check quota
                if total_size + file_size > quota {
                    // Delete oldest files - collect names first to avoid borrow conflict
                    let oldest_names: Vec<String> = {
                        let mut files: Vec<_> = client.cloud_files.iter().collect();
                        files.sort_by_key(|(_, f)| f.timestamp);
                        files.iter().map(|(name, _)| (*name).clone()).collect()
                    };

                    for name in oldest_names {
                        if total_size + file_size <= quota {
                            break;
                        }
                        if let Some(removed) = client.cloud_files.remove(&name) {
                            total_size -= removed.size;
                            deleted += 1;
                        }
                    }
                }

                // Add new file
                let file = CloudFile {
                    name: format!("save_{}.dat", i),
                    size: file_size,
                    timestamp: i as u64,
                    data: vec![0u8; file_size],
                };

                total_size += file_size;
                client.cloud_files.insert(file.name.clone(), file);
            }

            black_box((client.cloud_files.len(), deleted))
        });
    });

    // Test 4: Conflict resolution
    group.bench_function("conflict_resolution_1000", |bencher| {
        let local_files: Vec<CloudFile> = (0..100)
            .map(|i| CloudFile {
                name: format!("save_{}.dat", i),
                size: 1024,
                timestamp: 1000 + i as u64,
                data: vec![1u8; 1024],
            })
            .collect();

        let remote_files: Vec<CloudFile> = (0..100)
            .map(|i| CloudFile {
                name: format!("save_{}.dat", i),
                size: 1024,
                timestamp: if i % 2 == 0 {
                    1500 + i as u64
                } else {
                    500 + i as u64
                },
                data: vec![2u8; 1024],
            })
            .collect();

        bencher.iter(|| {
            for _ in 0..10 {
                let mut resolutions: Vec<(&str, &CloudFile)> = Vec::new();

                for (local, remote) in local_files.iter().zip(remote_files.iter()) {
                    // Keep newer version
                    if local.timestamp >= remote.timestamp {
                        resolutions.push(("local", local));
                    } else {
                        resolutions.push(("remote", remote));
                    }
                }

                let local_wins = resolutions.iter().filter(|(r, _)| *r == "local").count();
                black_box(local_wins);
            }
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 4: LEADERBOARDS
// ============================================================================

fn bench_leaderboards(c: &mut Criterion) {
    let mut group = c.benchmark_group("steam_adversarial/leaderboards");

    // Test 1: Leaderboard entry submission
    group.bench_function("submit_scores_10000", |bencher| {
        bencher.iter(|| {
            let mut leaderboard = Leaderboard {
                name: "high_scores".to_string(),
                sort_method: LeaderboardSort::Descending,
                display_type: LeaderboardDisplay::Numeric,
                entries: Vec::new(),
            };

            for i in 0..10000 {
                let entry = LeaderboardEntry {
                    steam_id: SteamId(i as u64),
                    rank: 0, // Will be calculated
                    score: (i * 137) % 1_000_000,
                    details: vec![i % 100, i % 50],
                };

                leaderboard.entries.push(entry);
            }

            // Sort and assign ranks
            leaderboard.entries.sort_by(|a, b| b.score.cmp(&a.score));
            for (i, entry) in leaderboard.entries.iter_mut().enumerate() {
                entry.rank = (i + 1) as u32;
            }

            black_box(leaderboard.entries.len())
        });
    });

    // Test 2: Leaderboard queries
    group.bench_function("query_leaderboard_5000", |bencher| {
        let mut leaderboard = Leaderboard {
            name: "high_scores".to_string(),
            sort_method: LeaderboardSort::Descending,
            display_type: LeaderboardDisplay::Numeric,
            entries: (0..10000)
                .map(|i| LeaderboardEntry {
                    steam_id: SteamId(i as u64),
                    rank: (i + 1) as u32,
                    score: 1_000_000 - i,
                    details: vec![],
                })
                .collect(),
        };

        leaderboard.entries.sort_by(|a, b| b.score.cmp(&a.score));

        bencher.iter(|| {
            for i in 0..5000 {
                // Query top N
                let top_10: Vec<&LeaderboardEntry> = leaderboard.entries.iter().take(10).collect();

                // Query around rank
                let rank: usize = (i % 9000) + 500;
                let around: Vec<&LeaderboardEntry> = leaderboard
                    .entries
                    .iter()
                    .skip(rank.saturating_sub(5))
                    .take(11)
                    .collect();

                // Query specific user
                let user_id = SteamId((i % 10000) as u64);
                let user_entry = leaderboard.entries.iter().find(|e| e.steam_id == user_id);

                black_box((top_10.len(), around.len(), user_entry.is_some()));
            }
        });
    });

    // Test 3: Score deduplication
    group.bench_function("score_deduplication_5000", |bencher| {
        let submissions: Vec<(SteamId, i32)> = (0..5000)
            .map(|i| {
                let user = SteamId((i % 1000) as u64);
                let score = (i * 137) % 1_000_000;
                (user, score)
            })
            .collect();

        bencher.iter(|| {
            let mut best_scores: HashMap<SteamId, i32> = HashMap::new();

            for (user, score) in &submissions {
                let entry = best_scores.entry(*user).or_insert(i32::MIN);
                *entry = (*entry).max(*score);
            }

            // Convert to sorted leaderboard
            let mut entries: Vec<_> = best_scores.into_iter().collect();
            entries.sort_by(|a, b| b.1.cmp(&a.1));

            black_box(entries.len())
        });
    });

    // Test 4: Time-based leaderboards
    group.bench_function("time_leaderboard_processing_2000", |bencher| {
        // Time in milliseconds
        let submissions: Vec<(SteamId, i32)> = (0..2000)
            .map(|i| {
                let user = SteamId((i % 500) as u64);
                let time_ms = 60_000 + (i * 137) % 120_000; // 1-3 minutes
                (user, time_ms)
            })
            .collect();

        bencher.iter(|| {
            let mut best_times: HashMap<SteamId, i32> = HashMap::new();

            // Keep best (lowest) time per user
            for (user, time) in &submissions {
                let entry = best_times.entry(*user).or_insert(i32::MAX);
                *entry = (*entry).min(*time);
            }

            // Sort ascending for time-based
            let mut entries: Vec<_> = best_times.into_iter().collect();
            entries.sort_by(|a, b| a.1.cmp(&b.1));

            // Format display times
            let formatted: Vec<(SteamId, String)> = entries
                .iter()
                .map(|(user, ms)| {
                    let minutes = ms / 60_000;
                    let seconds = (ms % 60_000) / 1000;
                    let millis = ms % 1000;
                    (*user, format!("{}:{:02}.{:03}", minutes, seconds, millis))
                })
                .collect();

            black_box(formatted.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 5: WORKSHOP INTEGRATION
// ============================================================================

fn bench_workshop(c: &mut Criterion) {
    let mut group = c.benchmark_group("steam_adversarial/workshop");

    // Test 1: Workshop item loading
    group.bench_function("load_workshop_items_1000", |bencher| {
        bencher.iter(|| {
            let items: Vec<WorkshopItem> = (0..1000)
                .map(|i| WorkshopItem {
                    id: 1000000 + i as u64,
                    title: format!("Mod {} - Amazing Content", i),
                    description: format!(
                        "This is a fantastic mod that adds new content to the game. {}",
                        "Lorem ipsum dolor sit amet. ".repeat(10)
                    ),
                    tags: vec![
                        ["weapons", "armor", "maps", "characters", "gameplay"][i % 5].to_string(),
                        ["featured", "popular", "new"][i % 3].to_string(),
                    ],
                    file_size: 1024 * 1024 + (i % 100) * 1024 * 100,
                    preview_url: format!("https://steamcdn.com/workshop/{}/preview.jpg", i),
                    votes_up: (i * 17) as u32 % 10000,
                    votes_down: (i * 7) as u32 % 1000,
                    subscribed: i % 5 == 0,
                })
                .collect();

            black_box(items.len())
        });
    });

    // Test 2: Workshop filtering and sorting
    group.bench_function("filter_sort_workshop_5000", |bencher| {
        let items: Vec<WorkshopItem> = (0..5000)
            .map(|i| WorkshopItem {
                id: i as u64,
                title: format!("Item {}", i),
                description: String::new(),
                tags: vec![
                    ["weapons", "armor", "maps", "characters", "gameplay"][i % 5].to_string(),
                ],
                file_size: 1024 * 1024 + (i % 100) * 1024 * 100,
                preview_url: String::new(),
                votes_up: (i * 17) as u32 % 10000,
                votes_down: (i * 7) as u32 % 1000,
                subscribed: i % 5 == 0,
            })
            .collect();

        bencher.iter(|| {
            // Filter by tag
            let weapons: Vec<&WorkshopItem> = items
                .iter()
                .filter(|item| item.tags.contains(&"weapons".to_string()))
                .collect();

            // Sort by rating
            let mut by_rating: Vec<&WorkshopItem> = items.iter().collect();
            by_rating.sort_by(|a, b| {
                let rating_a = a.votes_up as f32 / (a.votes_up + a.votes_down + 1) as f32;
                let rating_b = b.votes_up as f32 / (b.votes_up + b.votes_down + 1) as f32;
                rating_b.partial_cmp(&rating_a).unwrap()
            });

            // Get subscribed
            let subscribed: Vec<&WorkshopItem> =
                items.iter().filter(|item| item.subscribed).collect();

            black_box((weapons.len(), by_rating.len(), subscribed.len()))
        });
    });

    // Test 3: Subscription management
    group.bench_function("subscription_management_2000", |bencher| {
        bencher.iter(|| {
            let mut subscriptions: HashMap<u64, bool> = HashMap::new();

            for i in 0..2000 {
                let item_id = (i % 500) as u64;

                match i % 4 {
                    0 => {
                        // Subscribe
                        subscriptions.insert(item_id, true);
                    }
                    1 => {
                        // Unsubscribe
                        subscriptions.remove(&item_id);
                    }
                    2 => {
                        // Check status
                        let _is_subscribed = subscriptions.get(&item_id).copied().unwrap_or(false);
                    }
                    _ => {
                        // Toggle
                        let current = subscriptions.get(&item_id).copied().unwrap_or(false);
                        subscriptions.insert(item_id, !current);
                    }
                }
            }

            let sub_count = subscriptions.values().filter(|&&v| v).count();
            black_box(sub_count)
        });
    });

    // Test 4: Download queue management
    group.bench_function("download_queue_1000", |bencher| {
        let items: Vec<WorkshopItem> = (0..1000)
            .map(|i| WorkshopItem {
                id: i as u64,
                title: format!("Item {}", i),
                description: String::new(),
                tags: vec![],
                file_size: 1024 * 1024 + (i % 100) * 1024 * 1024,
                preview_url: String::new(),
                votes_up: 0,
                votes_down: 0,
                subscribed: true,
            })
            .collect();

        bencher.iter(|| {
            // Prioritize smaller files first
            let mut queue: Vec<&WorkshopItem> = items.iter().collect();
            queue.sort_by_key(|item| item.file_size);

            // Simulate download progress
            let mut downloaded = 0usize;
            let mut completed = 0;
            let bandwidth = 10 * 1024 * 1024usize; // 10 MB/s

            for item in &queue {
                downloaded += item.file_size;
                completed += 1;

                let _eta = (queue
                    .iter()
                    .skip(completed)
                    .map(|i| i.file_size)
                    .sum::<usize>())
                    / bandwidth;
            }

            black_box((completed, downloaded))
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 6: PLATFORM API SIMULATION
// ============================================================================

fn bench_platform_api(c: &mut Criterion) {
    let mut group = c.benchmark_group("steam_adversarial/platform_api");

    // Test 1: API call batching
    group.bench_function("api_call_batching_10000", |bencher| {
        #[derive(Clone)]
        struct ApiCall {
            method: String,
            params: HashMap<String, String>,
            timestamp: u64,
        }

        let calls: Vec<ApiCall> = (0..10000)
            .map(|i| {
                let mut params = HashMap::new();
                params.insert("user_id".to_string(), format!("{}", i % 1000));
                params.insert("data".to_string(), format!("value_{}", i));

                ApiCall {
                    method: ["GetStats", "SetStats", "GetAchievements", "GetLeaderboard"][i % 4]
                        .to_string(),
                    params,
                    timestamp: i as u64,
                }
            })
            .collect();

        bencher.iter(|| {
            // Batch by method and user
            let mut batches: HashMap<(String, String), Vec<&ApiCall>> = HashMap::new();

            for call in &calls {
                let user_id = call.params.get("user_id").cloned().unwrap_or_default();
                batches
                    .entry((call.method.clone(), user_id))
                    .or_default()
                    .push(call);
            }

            // Process batches
            let total_batches = batches.len();
            let max_batch_size = batches.values().map(|b| b.len()).max().unwrap_or(0);

            black_box((total_batches, max_batch_size))
        });
    });

    // Test 2: Rate limiting
    group.bench_function("rate_limiting_20000", |bencher| {
        let rate_limit = 100; // calls per second
        let window_ms = 1000;

        bencher.iter(|| {
            let mut call_times: Vec<u64> = Vec::new();
            let mut allowed = 0;
            let mut throttled = 0;

            for i in 0..20000 {
                let current_time = (i as u64 * 10) % 10000; // Simulated timestamps

                // Remove old calls outside window
                call_times.retain(|&t| current_time.saturating_sub(t) < window_ms);

                if call_times.len() < rate_limit {
                    call_times.push(current_time);
                    allowed += 1;
                } else {
                    throttled += 1;
                }
            }

            black_box((allowed, throttled))
        });
    });

    // Test 3: Callback queue processing
    group.bench_function("callback_processing_10000", |bencher| {
        #[derive(Clone)]
        struct Callback {
            callback_type: u32,
            data: Vec<u8>,
            timestamp: Instant,
        }

        bencher.iter(|| {
            let mut queue: Vec<Callback> = (0..10000)
                .map(|i| Callback {
                    callback_type: (i % 20) as u32,
                    data: vec![i as u8; 64],
                    timestamp: Instant::now(),
                })
                .collect();

            // Process by type priority
            queue.sort_by_key(|cb| cb.callback_type);

            let mut processed_by_type: HashMap<u32, usize> = HashMap::new();

            for callback in &queue {
                *processed_by_type.entry(callback.callback_type).or_insert(0) += 1;
            }

            black_box(processed_by_type.len())
        });
    });

    // Test 4: Session management
    group.bench_function("session_management_5000", |bencher| {
        #[derive(Clone)]
        struct Session {
            user_id: SteamId,
            app_id: u32,
            start_time: u64,
            last_activity: u64,
            state: String,
        }

        bencher.iter(|| {
            let mut sessions: HashMap<SteamId, Session> = HashMap::new();

            for i in 0..5000 {
                let user_id = SteamId((i % 500) as u64);

                match i % 5 {
                    0 => {
                        // Create session
                        sessions.insert(
                            user_id,
                            Session {
                                user_id,
                                app_id: 12345,
                                start_time: i as u64,
                                last_activity: i as u64,
                                state: "active".to_string(),
                            },
                        );
                    }
                    1 => {
                        // Update activity
                        if let Some(session) = sessions.get_mut(&user_id) {
                            session.last_activity = i as u64;
                        }
                    }
                    2 => {
                        // Check timeout (inactive > 300 seconds)
                        if let Some(session) = sessions.get(&user_id) {
                            let inactive = i as u64 - session.last_activity;
                            if inactive > 300 {
                                // Would mark for cleanup
                            }
                        }
                    }
                    3 => {
                        // End session
                        sessions.remove(&user_id);
                    }
                    _ => {
                        // Query session
                        let _exists = sessions.contains_key(&user_id);
                    }
                }
            }

            let active = sessions.values().filter(|s| s.state == "active").count();
            black_box(active)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_achievements,
    bench_statistics,
    bench_cloud_saves,
    bench_leaderboards,
    bench_workshop,
    bench_platform_api,
);

criterion_main!(benches);
