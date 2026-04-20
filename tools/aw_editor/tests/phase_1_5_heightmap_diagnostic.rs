//! Phase 1.5-T.A tuning investigation — heightmap Y-range diagnostic.
//!
//! This test drives the exact terrain-generation path the editor uses
//! (seed `12345`, `terrain_primary_biome = "grassland"`, default chunk
//! radius `5`) and prints aggregate Y-range statistics so the elevation
//! band constants in `astraweave_terrain::elevation_biome` can be
//! retuned against real data rather than guessed from the shader.
//!
//! Run with: `cargo test -p aw_editor --test phase_1_5_heightmap_diagnostic -- --nocapture`.
//!
//! This is temporary diagnostic infrastructure. 1.5-T.E removes it once
//! the tuning pass closes; the findings live in
//! `docs/audits/phase_1_5_tuning_investigation_2026-04-20.md`.

use aw_editor_lib::terrain_integration::TerrainState;

/// Number of buckets in the global histogram.
const HIST_BUCKETS: usize = 20;

#[test]
fn heightmap_y_range_diagnostic_radius_5_grassland_seed_12345() {
    heightmap_diagnostic(5, "grassland");
}

#[test]
fn heightmap_y_range_diagnostic_radius_6_grassland_seed_12345() {
    heightmap_diagnostic(6, "grassland");
}

#[test]
fn heightmap_y_range_diagnostic_radius_5_tundra_seed_12345() {
    heightmap_diagnostic(5, "tundra");
}

fn heightmap_diagnostic(chunk_radius: i32, biome: &str) {
    let mut state = TerrainState::new();
    state.configure(12345, biome);

    let count = state
        .generate_terrain(chunk_radius)
        .expect("terrain generation should succeed");
    assert!(count > 0, "no chunks generated");

    // Global aggregates.
    let mut g_min = f32::INFINITY;
    let mut g_max = f32::NEG_INFINITY;
    let mut g_sum = 0.0f64;
    let mut g_count = 0u64;

    // Per-chunk stats for the first 5 chunks, for spot-checking.
    let mut chunk_stats = Vec::new();

    for (chunk_id, gen_chunk) in state.chunks() {
        let mut c_min = f32::INFINITY;
        let mut c_max = f32::NEG_INFINITY;
        let mut c_sum = 0.0f64;
        let mut c_count = 0u64;
        for v in gen_chunk.vertices.iter() {
            let y = v.position[1];
            c_min = c_min.min(y);
            c_max = c_max.max(y);
            c_sum += y as f64;
            c_count += 1;

            g_min = g_min.min(y);
            g_max = g_max.max(y);
            g_sum += y as f64;
            g_count += 1;
        }
        let c_mean = if c_count > 0 { c_sum / c_count as f64 } else { 0.0 };
        if chunk_stats.len() < 5 {
            chunk_stats.push((
                *chunk_id,
                c_min,
                c_max,
                c_mean as f32,
                c_count,
            ));
        }
    }

    assert!(g_count > 0, "no vertices iterated");
    let g_mean = (g_sum / g_count as f64) as f32;
    let span = g_max - g_min;

    // Second pass: 20-bucket histogram across the observed global Y range.
    let bucket_width = span / HIST_BUCKETS as f32;
    let mut histogram = [0u64; HIST_BUCKETS];
    for (_id, gen_chunk) in state.chunks() {
        for v in gen_chunk.vertices.iter() {
            let y = v.position[1];
            let t = if bucket_width > 0.0 {
                ((y - g_min) / bucket_width).floor() as i64
            } else {
                0
            };
            let idx = (t.max(0) as usize).min(HIST_BUCKETS - 1);
            histogram[idx] += 1;
        }
    }

    // Band-coverage summary against the current Phase 1.5 Temperate bands.
    // Slot order matches elevation_biome module docs:
    //   [0] Grassland, [1] Desert, [2] Forest, [3] Mountain,
    //   [4] Tundra,    [5] Swamp,  [6] Beach,  [7] River
    let mut beach_count = 0u64;     // slot 6
    let mut grassland_count = 0u64; // slot 0
    let mut forest_count = 0u64;    // slot 2
    let mut mountain_count = 0u64;  // slot 3
    let mut other_count = 0u64;
    for (_id, gen_chunk) in state.chunks() {
        for v in gen_chunk.vertices.iter() {
            // Dominant of the 8-slot combined array.
            let w: [f32; 8] = [
                v.biome_weights_0[0],
                v.biome_weights_0[1],
                v.biome_weights_0[2],
                v.biome_weights_0[3],
                v.biome_weights_1[0],
                v.biome_weights_1[1],
                v.biome_weights_1[2],
                v.biome_weights_1[3],
            ];
            let mut best = (0usize, w[0]);
            for (i, &val) in w.iter().enumerate().skip(1) {
                if val > best.1 {
                    best = (i, val);
                }
            }
            match best.0 {
                0 => grassland_count += 1,
                2 => forest_count += 1,
                3 => mountain_count += 1,
                6 => beach_count += 1,
                _ => other_count += 1,
            }
        }
    }

    println!("=========================================================");
    println!("Phase 1.5-T.A heightmap diagnostic");
    println!("  chunk_radius = {chunk_radius}  biome = {biome:?}");
    println!("  chunks = {count}  total_vertices = {g_count}");
    println!("");
    println!("  Global Y range:");
    println!("    min  = {:.3}", g_min);
    println!("    max  = {:.3}", g_max);
    println!("    span = {:.3}", span);
    println!("    mean = {:.3}", g_mean);
    println!("");
    println!(
        "  Y histogram ({} buckets across [{:.2}, {:.2}], {:.2} units each):",
        HIST_BUCKETS, g_min, g_max, bucket_width
    );
    for (i, c) in histogram.iter().enumerate() {
        let low = g_min + bucket_width * i as f32;
        let high = g_min + bucket_width * (i + 1) as f32;
        let pct = 100.0 * (*c as f64) / (g_count as f64);
        println!(
            "    [{:2}] {:>6.2}..{:<6.2}  {:>8} ({:>5.2}%)",
            i, low, high, c, pct
        );
    }
    println!("");
    println!("  First 5 per-chunk Y stats (chunk_id, min, max, mean, n):");
    for (id, mn, mx, mean, n) in &chunk_stats {
        println!(
            "    {:?}: min={:.2}  max={:.2}  mean={:.2}  n={}",
            id, mn, mx, mean, n
        );
    }
    println!("");
    println!("  Dominant-biome-per-vertex counts (from biome_weights_0/1):");
    let total = g_count as f64;
    let show = |label: &str, c: u64| {
        println!(
            "    {:>10}: {:>8}  ({:>5.2}%)",
            label,
            c,
            100.0 * c as f64 / total
        );
    };
    show("Beach", beach_count);
    show("Grassland", grassland_count);
    show("Forest", forest_count);
    show("Mountain", mountain_count);
    show("other", other_count);
    println!("=========================================================");
}
