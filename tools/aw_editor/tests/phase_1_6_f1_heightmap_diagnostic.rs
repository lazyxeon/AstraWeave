//! Phase 1.6-F.1 diagnostic — measures runtime Y span for each biome-noise preset.
//!
//! Drives the exact editor call chain: configure → set_noise_params →
//! apply_biome_noise_preset → generate_terrain. This is critical — measuring
//! against `NoiseConfig::default()` without applying the preset is what
//! produced Phase 1.5-T's stale 125-unit measurement (see
//! `docs/audits/heightmap_generator_audit_2026-04-21.md` §6 "Correction to
//! Phase 1.5-T measurement").
//!
//! Because `TerrainPanel::noise_preset_for_biome` is a private associated
//! function and F.1's scope forbids API changes, this test inlines a
//! `BiomeNoisePreset` literal per preset. The literal MUST mirror the
//! corresponding arm in `tools/aw_editor/src/panels/terrain_panel.rs::
//! noise_preset_for_biome`. When a preset is tuned, the production literal
//! and this test literal must be updated together.
//!
//! Run with:
//!   cargo test -p aw_editor --test phase_1_6_f1_heightmap_diagnostic -- --nocapture
//!
//! This test is temporary. F.1.C deletes the file; the tuned preset values
//! are the permanent deliverable.

use aw_editor_lib::terrain_integration::{BiomeNoisePreset, TerrainState};

/// Number of histogram buckets across the observed global Y range.
const HIST_BUCKETS: usize = 20;

/// Editor's baseline slider values from `TerrainPanel::default()` (fields
/// `octaves`, `lacunarity`, `persistence`, `base_amplitude` at
/// `terrain_panel.rs:647-650`). `set_noise_params` applies these before the
/// preset override, matching `regenerate_terrain`'s call order.
const EDITOR_DEFAULT_OCTAVES: usize = 6;
const EDITOR_DEFAULT_LACUNARITY: f64 = 2.0;
const EDITOR_DEFAULT_PERSISTENCE: f64 = 0.5;
const EDITOR_DEFAULT_BASE_AMPLITUDE: f32 = 50.0;

/// Editor default chunk_radius (the primary diagnostic target).
const CHUNK_RADIUS: i32 = 5;

/// Diagnostic seed. Matches the parent campaign's canonical seed.
const SEED: u64 = 12345;

// ---------------------------------------------------------------------------
// Preset literals — mirror `terrain_panel.rs::noise_preset_for_biome`.
// Update these in lockstep with the production file during F.1 tuning.
// ---------------------------------------------------------------------------

fn grassland_preset() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.004,
        base_amplitude: 50.0,
        base_octaves: 5,
        base_persistence: 0.50,
        base_lacunarity: 2.0,
        mountains_enabled: true,
        mountains_scale: 0.0025,
        mountains_amplitude: 80.0,
        mountains_octaves: 6,
        detail_enabled: true,
        detail_scale: 0.02,
        detail_amplitude: 8.0,
        erosion_enabled: true,
        erosion_strength: 0.3,
    }
}

fn mountain_preset() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.003,
        base_amplitude: 55.0,
        base_octaves: 6,
        base_persistence: 0.55,
        base_lacunarity: 2.2,
        mountains_enabled: true,
        mountains_scale: 0.002,
        mountains_amplitude: 210.0,
        mountains_octaves: 8,
        detail_enabled: true,
        detail_scale: 0.03,
        detail_amplitude: 8.0,
        erosion_enabled: false,
        erosion_strength: 0.0,
    }
}

fn desert_preset() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.004,
        base_amplitude: 45.0,
        base_octaves: 5,
        base_persistence: 0.45,
        base_lacunarity: 2.2,
        mountains_enabled: true,
        mountains_scale: 0.0015,
        mountains_amplitude: 35.0,
        mountains_octaves: 4,
        detail_enabled: true,
        detail_scale: 0.06,
        detail_amplitude: 6.0,
        erosion_enabled: true,
        erosion_strength: 0.2,
    }
}

fn forest_preset() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.004,
        base_amplitude: 40.0,
        base_octaves: 5,
        base_persistence: 0.50,
        base_lacunarity: 2.0,
        mountains_enabled: true,
        mountains_scale: 0.003,
        mountains_amplitude: 40.0,
        mountains_octaves: 4,
        detail_enabled: true,
        detail_scale: 0.02,
        detail_amplitude: 6.0,
        erosion_enabled: true,
        erosion_strength: 0.3,
    }
}

fn tundra_preset() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.003,
        base_amplitude: 55.0,
        base_octaves: 5,
        base_persistence: 0.45,
        base_lacunarity: 2.0,
        mountains_enabled: true,
        mountains_scale: 0.002,
        mountains_amplitude: 150.0,
        mountains_octaves: 6,
        detail_enabled: true,
        detail_scale: 0.015,
        detail_amplitude: 5.0,
        erosion_enabled: true,
        erosion_strength: 0.3,
    }
}

fn swamp_preset() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.006,
        base_amplitude: 40.0,
        base_octaves: 4,
        base_persistence: 0.55,
        base_lacunarity: 1.8,
        mountains_enabled: true,
        mountains_scale: 0.003,
        mountains_amplitude: 45.0,
        mountains_octaves: 3,
        detail_enabled: true,
        detail_scale: 0.03,
        detail_amplitude: 2.0,
        erosion_enabled: true,
        erosion_strength: 0.3,
    }
}

fn beach_preset() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.008,
        base_amplitude: 32.0,
        base_octaves: 4,
        base_persistence: 0.40,
        base_lacunarity: 2.0,
        mountains_enabled: true,
        mountains_scale: 0.003,
        mountains_amplitude: 35.0,
        mountains_octaves: 3,
        detail_enabled: true,
        detail_scale: 0.05,
        detail_amplitude: 2.0,
        erosion_enabled: true,
        erosion_strength: 0.3,
    }
}

fn river_preset() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.004,
        base_amplitude: 35.0,
        base_octaves: 5,
        base_persistence: 0.45,
        base_lacunarity: 2.0,
        mountains_enabled: true,
        mountains_scale: 0.003,
        mountains_amplitude: 35.0,
        mountains_octaves: 4,
        detail_enabled: true,
        detail_scale: 0.025,
        detail_amplitude: 4.0,
        erosion_enabled: true,
        erosion_strength: 0.3,
    }
}

// ---------------------------------------------------------------------------
// Shared diagnostic helpers.
// ---------------------------------------------------------------------------

struct YStats {
    y_min: f32,
    y_max: f32,
    y_mean: f32,
    vertex_count: u64,
    histogram: [u64; HIST_BUCKETS],
    // Dominant-biome-per-vertex counts across all 8 biome slots.
    biome_counts: [u64; 8],
}

fn collect_y_statistics(state: &TerrainState) -> YStats {
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;
    let mut y_sum = 0.0f64;
    let mut n = 0u64;
    let mut biome_counts = [0u64; 8];

    for (_id, gc) in state.chunks() {
        for v in gc.vertices.iter() {
            let y = v.position[1];
            if y < y_min {
                y_min = y;
            }
            if y > y_max {
                y_max = y;
            }
            y_sum += y as f64;
            n += 1;

            let w = [
                v.biome_weights_0[0],
                v.biome_weights_0[1],
                v.biome_weights_0[2],
                v.biome_weights_0[3],
                v.biome_weights_1[0],
                v.biome_weights_1[1],
                v.biome_weights_1[2],
                v.biome_weights_1[3],
            ];
            let mut best_idx = 0usize;
            let mut best_val = w[0];
            for (i, &val) in w.iter().enumerate().skip(1) {
                if val > best_val {
                    best_idx = i;
                    best_val = val;
                }
            }
            biome_counts[best_idx] += 1;
        }
    }

    assert!(n > 0, "no vertices iterated");

    let y_mean = (y_sum / n as f64) as f32;
    let span = y_max - y_min;
    let bucket_width = if span > 0.0 {
        span / HIST_BUCKETS as f32
    } else {
        1.0
    };

    let mut histogram = [0u64; HIST_BUCKETS];
    for (_id, gc) in state.chunks() {
        for v in gc.vertices.iter() {
            let y = v.position[1];
            let t = ((y - y_min) / bucket_width).floor() as i64;
            let idx = (t.max(0) as usize).min(HIST_BUCKETS - 1);
            histogram[idx] += 1;
        }
    }

    YStats {
        y_min,
        y_max,
        y_mean,
        vertex_count: n,
        histogram,
        biome_counts,
    }
}

fn print_stats(label: &str, stats: &YStats) {
    let span = stats.y_max - stats.y_min;
    let bucket_width = if span > 0.0 {
        span / HIST_BUCKETS as f32
    } else {
        1.0
    };
    let n = stats.vertex_count as f64;

    println!("=========================================================");
    println!("Phase 1.6-F.1 diagnostic — preset: {label}");
    println!("  vertices = {}", stats.vertex_count);
    println!(
        "  Y min={:.3}  max={:.3}  span={:.3}  mean={:.3}",
        stats.y_min, stats.y_max, span, stats.y_mean
    );
    println!("  Histogram ({} buckets, {:.2} units each):", HIST_BUCKETS, bucket_width);
    for (i, c) in stats.histogram.iter().enumerate() {
        let low = stats.y_min + bucket_width * i as f32;
        let high = stats.y_min + bucket_width * (i + 1) as f32;
        let pct = 100.0 * (*c as f64) / n;
        println!(
            "    [{:2}] {:>7.2}..{:<7.2}  {:>8} ({:>5.2}%)",
            i, low, high, c, pct
        );
    }
    // Slot order matches elevation_biome module docs:
    //   [0] Grassland, [1] Desert, [2] Forest, [3] Mountain,
    //   [4] Tundra,    [5] Swamp,  [6] Beach,  [7] River
    let names = [
        "Grassland", "Desert", "Forest", "Mountain", "Tundra", "Swamp", "Beach", "River",
    ];
    println!("  Dominant-biome-per-vertex counts:");
    for i in 0..8 {
        let c = stats.biome_counts[i];
        let pct = 100.0 * c as f64 / n;
        println!(
            "    {:>10}: {:>8}  ({:>5.2}%)",
            names[i], c, pct
        );
    }
    println!("=========================================================");
}

fn run_preset(biome: &str, preset: BiomeNoisePreset) -> YStats {
    let mut state = TerrainState::new();
    state.configure(SEED, biome);
    state.set_noise_params(
        EDITOR_DEFAULT_OCTAVES,
        EDITOR_DEFAULT_LACUNARITY,
        EDITOR_DEFAULT_PERSISTENCE,
        EDITOR_DEFAULT_BASE_AMPLITUDE,
    );
    state.apply_biome_noise_preset(&preset);
    let count = state
        .generate_terrain(CHUNK_RADIUS)
        .expect("terrain generation should succeed");
    assert!(count > 0, "no chunks generated for preset {biome}");
    collect_y_statistics(&state)
}

// ---------------------------------------------------------------------------
// Per-preset tests — F.1.A covers grassland; F.1.B extends to the rest.
// ---------------------------------------------------------------------------

#[test]
fn phase_1_6_f1_grassland_preset_y_span() {
    let stats = run_preset("grassland", grassland_preset());
    print_stats("grassland", &stats);
    let span = stats.y_max - stats.y_min;
    assert!(
        span >= 100.0,
        "Grassland preset produces Y span {span:.2} — expected >= 100 for Phase 1.5 bands to express"
    );
}

#[test]
fn phase_1_6_f1_mountain_preset_y_span() {
    let stats = run_preset("mountain", mountain_preset());
    print_stats("mountain", &stats);
    let span = stats.y_max - stats.y_min;
    assert!(
        span >= 150.0,
        "Mountain preset produces Y span {span:.2} — expected >= 150 (dramatic-preset floor)"
    );
}

#[test]
fn phase_1_6_f1_desert_preset_y_span() {
    let stats = run_preset("desert", desert_preset());
    print_stats("desert", &stats);
    let span = stats.y_max - stats.y_min;
    assert!(
        span >= 60.0,
        "Desert preset produces Y span {span:.2} — expected >= 60 (general-preset floor)"
    );
}

#[test]
fn phase_1_6_f1_forest_preset_y_span() {
    let stats = run_preset("forest", forest_preset());
    print_stats("forest", &stats);
    let span = stats.y_max - stats.y_min;
    assert!(
        span >= 60.0,
        "Forest preset produces Y span {span:.2} — expected >= 60 (general-preset floor)"
    );
}

#[test]
fn phase_1_6_f1_tundra_preset_y_span() {
    let stats = run_preset("tundra", tundra_preset());
    print_stats("tundra", &stats);
    let span = stats.y_max - stats.y_min;
    // Tundra maps to ClimateBias::Cold which uses Mountain-style erosion per
    // plan §2.2. Tundra terrain is typically mountainous — apply the dramatic
    // floor.
    assert!(
        span >= 150.0,
        "Tundra preset produces Y span {span:.2} — expected >= 150 (dramatic-preset floor)"
    );
}

#[test]
fn phase_1_6_f1_swamp_preset_y_span() {
    let stats = run_preset("swamp", swamp_preset());
    print_stats("swamp", &stats);
    let span = stats.y_max - stats.y_min;
    assert!(
        span >= 60.0,
        "Swamp preset produces Y span {span:.2} — expected >= 60 (general-preset floor)"
    );
}

#[test]
fn phase_1_6_f1_beach_preset_y_span() {
    let stats = run_preset("beach", beach_preset());
    print_stats("beach", &stats);
    let span = stats.y_max - stats.y_min;
    assert!(
        span >= 60.0,
        "Beach preset produces Y span {span:.2} — expected >= 60 (general-preset floor)"
    );
}

#[test]
fn phase_1_6_f1_river_preset_y_span() {
    let stats = run_preset("river", river_preset());
    print_stats("river", &stats);
    let span = stats.y_max - stats.y_min;
    assert!(
        span >= 60.0,
        "River preset produces Y span {span:.2} — expected >= 60 (general-preset floor)"
    );
}
