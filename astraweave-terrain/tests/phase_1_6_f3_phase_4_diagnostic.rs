//! Phase 1.6-F.3-phase-4.A: diagnostic tests preceding the phase-4 fixes.
//!
//! (1) Verify biome_weights still match byte-for-byte at shared chunk
//!     edges (Shape A invariant: pre-erosion heightmap is byte-identical
//!     at shared edges via `TerrainNoise::sample_height`'s world-coord
//!     determinism; biome_weights are computed from pre-erosion heights
//!     per §2.5; therefore biome_weights at shared edges must be
//!     byte-identical). If this fails, something regressed in phase 2 or
//!     phase 3.
//! (2) Capture the post-phase-3 scale baseline per climate. Phase-4.C's
//!     droplet-count reduction targets returning peak compression to
//!     phase-2 levels (~-28% on Cold/Highland, ~-15% on Temperate).

use astraweave_terrain::{
    runevision_erosion::RunevisionConfig, smooth_shared_vertices, ChunkId, ClimateBias,
    TerrainChunk, WorldConfig, WorldGenerator,
};
use std::collections::HashMap;

fn make_generator(erosion: bool) -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = 12345;
    config.noise.erosion_enabled = erosion;
    WorldGenerator::new(config)
}

/// Phase 1.6-F.4.B.3.C: same as `make_generator` but enables the runevision
/// erosion filter and the derivative-weighted base layer it requires
/// (mirroring the Mountain/Tundra preset's configuration). Used by the
/// `phase_4_b_3_c_runevision_radius5_per_climate` diagnostic to measure
/// filter-ON Y statistics for direct comparison against the filter-OFF
/// baseline produced by `phase_4_b_1_scale_radius5_per_climate`.
fn make_generator_runevision(erosion: bool) -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = 12345;
    config.noise.erosion_enabled = erosion;
    config.noise.base_derivative_weighted = true;
    config.noise.runevision = Some(RunevisionConfig::default());
    WorldGenerator::new(config)
}

/// Phase-4.A invariant check: biome_weights populated by
/// `generate_chunk_with_climate` from pre-erosion heights must match
/// byte-for-byte at shared edges between adjacent chunks. Violation
/// means Shape A regressed — fix before proceeding to phase-4.B.
#[test]
fn biome_weights_at_shared_edges_match() {
    let gen = make_generator(true);
    let chunk_a = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("A");
    let chunk_b = gen
        .generate_chunk_with_climate(ChunkId::new(1, 0), ClimateBias::Temperate)
        .expect("B");

    let wa = chunk_a.biome_weights().expect("A biome_weights");
    let wb = chunk_b.biome_weights().expect("B biome_weights");

    let hm_a = chunk_a.heightmap();
    let dim = hm_a.resolution() as usize;

    for z in 0..dim {
        let a_right = wa[z * dim + (dim - 1)];
        let b_left = wb[z * dim];
        for slot in 0..8 {
            let diff = (a_right[slot] - b_left[slot]).abs();
            assert!(
                diff < 1e-5,
                "biome_weights Shape A violation at shared edge z={z} slot={slot}: \
                 {} vs {} (diff {diff})",
                a_right[slot],
                b_left[slot]
            );
        }
    }
}

/// Phase-4.A scale baseline: record pre-erosion and post-erosion p99 per
/// climate. Phase-4.C's droplet-count reduction aims to move post-erosion
/// p99 closer to pre-erosion (less compression). Baseline values feed
/// §10 closeout entry and the droplet-count tuning decision.
#[test]
fn phase_4_scale_baseline_per_climate() {
    let gen_on = make_generator(true);
    let gen_off = make_generator(false);

    let climates = [
        (ClimateBias::Temperate, "Temperate"),
        (ClimateBias::Cold, "Cold"),
        (ClimateBias::Arid, "Arid"),
        (ClimateBias::Tropical, "Tropical"),
        (ClimateBias::Wetland, "Wetland"),
        (ClimateBias::Highland, "Highland"),
    ];

    println!("======================================================");
    println!("F.3-phase-4.A: scale baseline (pre-fix, post-phase-3)");
    println!();
    println!("| Climate   | pre.p99 | post.p99 | Δp99 % |");
    println!("|-----------|--------:|---------:|-------:|");

    for (climate, name) in climates {
        let pre = gen_off
            .generate_chunk_with_climate(ChunkId::new(0, 0), climate)
            .expect("pre");
        let post = gen_on
            .generate_chunk_with_climate(ChunkId::new(0, 0), climate)
            .expect("post");

        let mut pre_ys: Vec<f32> = pre.heightmap().data().to_vec();
        let mut post_ys: Vec<f32> = post.heightmap().data().to_vec();
        pre_ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        post_ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pre_p99 = pre_ys[(pre_ys.len() * 99) / 100];
        let post_p99 = post_ys[(post_ys.len() * 99) / 100];
        let delta = if pre_p99.abs() > 1e-4 {
            (post_p99 - pre_p99) / pre_p99 * 100.0
        } else {
            0.0
        };
        println!("| {name:<9} | {pre_p99:7.2} | {post_p99:8.2} | {delta:+6.1} |");
    }
    println!("======================================================");
}

/// F.4.B.1 scale diagnostic: measure Y statistics across a FULL
/// radius-5 grid (121 chunks) per climate. Single-chunk measurements
/// (chunk (0,0)) are local; scale decisions need global statistics.
/// No behavior change; pure measurement for
/// `docs/audits/terrain_scale_diagnostic_2026-04-24.md`.
#[test]
fn phase_4_b_1_scale_radius5_per_climate() {
    let gen_on = make_generator(true);
    let gen_off = make_generator(false);

    let climates = [
        (ClimateBias::Temperate, "Temperate"),
        (ClimateBias::Cold, "Cold"),
        (ClimateBias::Arid, "Arid"),
        (ClimateBias::Tropical, "Tropical"),
        (ClimateBias::Wetland, "Wetland"),
        (ClimateBias::Highland, "Highland"),
    ];

    println!("======================================================");
    println!("F.4.B.1: full-world scale per climate (radius 5, 121 chunks)");
    println!();
    println!(
        "| Climate   |     pre.max |  pre.p99 |  pre.p50 | post.max | post.p99 | post.p50 | Y span |"
    );
    println!(
        "|-----------|------------:|---------:|---------:|---------:|---------:|---------:|-------:|"
    );

    for (climate, name) in climates {
        let mut pre_all = Vec::new();
        let mut post_all = Vec::new();
        for x in -5..=5 {
            for z in -5..=5 {
                let chunk_id = ChunkId::new(x, z);
                let pre = gen_off
                    .generate_chunk_with_climate(chunk_id, climate)
                    .expect("pre");
                let post = gen_on
                    .generate_chunk_with_climate(chunk_id, climate)
                    .expect("post");
                pre_all.extend_from_slice(pre.heightmap().data());
                post_all.extend_from_slice(post.heightmap().data());
            }
        }
        pre_all.sort_by(|a, b| a.partial_cmp(b).unwrap());
        post_all.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pre_max = *pre_all.last().unwrap();
        let pre_p99 = pre_all[(pre_all.len() * 99) / 100];
        let pre_p50 = pre_all[pre_all.len() / 2];
        let post_max = *post_all.last().unwrap();
        let post_p99 = post_all[(post_all.len() * 99) / 100];
        let post_p50 = post_all[post_all.len() / 2];
        let post_min = *post_all.first().unwrap();
        let span = post_max - post_min;
        println!(
            "| {name:<9} | {pre_max:11.2} | {pre_p99:8.2} | {pre_p50:8.2} | {post_max:8.2} | {post_p99:8.2} | {post_p50:8.2} | {span:6.2} |"
        );
    }
    println!();
    println!("chunk_size = 256 WU, heightmap_resolution = 64, vertex spacing = 4 WU");
    println!("radius = 5 → 11×11 = 121 chunks → 2816 WU × 2816 WU world extent");
    println!("======================================================");
}

/// Phase 1.6-F.4.B.3.C: filter-ON Y-statistics measurement for radius-5 grid,
/// per climate. Mirrors `phase_4_b_1_scale_radius5_per_climate` but with
/// runevision filter enabled (and derivative-weighted base layer it requires).
/// Used at F.4.B.3.C closeout to capture per-climate Y-shift caused by the
/// combined (derivative-weighted base + runevision filter) configuration that
/// Mountain/Tundra presets opt into. Filter-OFF baseline comes from the
/// `phase_4_b_1_scale_radius5_per_climate` companion test.
#[test]
fn phase_4_b_3_c_runevision_radius5_per_climate() {
    let gen_on = make_generator_runevision(true);
    let gen_off = make_generator_runevision(false);

    let climates = [
        (ClimateBias::Temperate, "Temperate"),
        (ClimateBias::Cold, "Cold"),
        (ClimateBias::Arid, "Arid"),
        (ClimateBias::Tropical, "Tropical"),
        (ClimateBias::Wetland, "Wetland"),
        (ClimateBias::Highland, "Highland"),
    ];

    println!("======================================================");
    println!(
        "F.4.B.3.C: runevision-ON full-world scale per climate (radius 5, 121 chunks)"
    );
    println!();
    println!(
        "| Climate   |     pre.max |  pre.p99 |  pre.p50 | post.max | post.p99 | post.p50 | Y span |"
    );
    println!(
        "|-----------|------------:|---------:|---------:|---------:|---------:|---------:|-------:|"
    );

    for (climate, name) in climates {
        let mut pre_all = Vec::new();
        let mut post_all = Vec::new();
        for x in -5..=5 {
            for z in -5..=5 {
                let chunk_id = ChunkId::new(x, z);
                let pre = gen_off
                    .generate_chunk_with_climate(chunk_id, climate)
                    .expect("pre");
                let post = gen_on
                    .generate_chunk_with_climate(chunk_id, climate)
                    .expect("post");
                pre_all.extend_from_slice(pre.heightmap().data());
                post_all.extend_from_slice(post.heightmap().data());
            }
        }
        pre_all.sort_by(|a, b| a.partial_cmp(b).unwrap());
        post_all.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pre_max = *pre_all.last().unwrap();
        let pre_p99 = pre_all[(pre_all.len() * 99) / 100];
        let pre_p50 = pre_all[pre_all.len() / 2];
        let post_max = *post_all.last().unwrap();
        let post_p99 = post_all[(post_all.len() * 99) / 100];
        let post_p50 = post_all[post_all.len() / 2];
        let post_min = *post_all.first().unwrap();
        let span = post_max - post_min;
        println!(
            "| {name:<9} | {pre_max:11.2} | {pre_p99:8.2} | {pre_p50:8.2} | {post_max:8.2} | {post_p99:8.2} | {post_p50:8.2} | {span:6.2} |"
        );
    }
    println!();
    println!("config: base_derivative_weighted=true, runevision=Some(default)");
    println!("(Mountain/Tundra preset configuration; valley_alt=50, peak_alt=400, octaves=3)");
    println!("======================================================");
}

/// Phase 1.6-F.3-phase-4.B: after `smooth_shared_vertices` runs, every
/// shared-edge vertex across adjacent chunks should match exactly
/// (within floating-point precision). Measures divergence before and
/// after the smoothing pass.
#[test]
fn shared_edges_exactly_match_after_averaging() {
    let gen = make_generator(true);

    // Build a 3×3 grid so internal edges are shared; outer-ring edges
    // touch only one chunk and remain unchanged by `smooth_shared_vertices`.
    let mut chunks: HashMap<ChunkId, TerrainChunk> = HashMap::new();
    for z in 0..3 {
        for x in 0..3 {
            let id = ChunkId::new(x, z);
            chunks.insert(
                id,
                gen.generate_chunk_with_climate(id, ClimateBias::Temperate)
                    .expect("chunk"),
            );
        }
    }

    let dim = chunks
        .values()
        .next()
        .unwrap()
        .heightmap()
        .resolution();

    // Measure pre-smoothing divergence on one internal edge:
    // chunk (0,0) right col vs chunk (1,0) left col.
    let mut pre_max = 0.0f32;
    {
        let a = chunks[&ChunkId::new(0, 0)].heightmap();
        let b = chunks[&ChunkId::new(1, 0)].heightmap();
        for zi in 0..dim {
            pre_max = pre_max.max((a.get_height(dim - 1, zi) - b.get_height(0, zi)).abs());
        }
    }
    println!("pre-smoothing x-edge max diff: {pre_max:.6}");

    smooth_shared_vertices(&mut chunks);

    // Measure post-smoothing divergence on the SAME edge — expected
    // near-zero (floating-point noise only).
    let mut post_max = 0.0f32;
    {
        let a = chunks[&ChunkId::new(0, 0)].heightmap();
        let b = chunks[&ChunkId::new(1, 0)].heightmap();
        for zi in 0..dim {
            post_max = post_max.max((a.get_height(dim - 1, zi) - b.get_height(0, zi)).abs());
        }
    }
    println!("post-smoothing x-edge max diff: {post_max:.6}");

    // Must be < 1e-5 — any larger indicates the averaging is not setting
    // both chunks' shared-edge vertices to the same value.
    assert!(
        post_max < 1e-5,
        "shared-edge averaging failed: post-smoothing max diff {post_max}"
    );

    // Also verify a z-axis edge (chunk (0,0) bottom vs chunk (0,1) top).
    let mut post_z_max = 0.0f32;
    {
        let a = chunks[&ChunkId::new(0, 0)].heightmap();
        let b = chunks[&ChunkId::new(0, 1)].heightmap();
        for xi in 0..dim {
            post_z_max = post_z_max.max((a.get_height(xi, dim - 1) - b.get_height(xi, 0)).abs());
        }
    }
    assert!(
        post_z_max < 1e-5,
        "z-axis shared-edge averaging failed: post-smoothing max diff {post_z_max}"
    );

    // Corner vertex (0,0) in chunk (1,1) is shared by all 4 surrounding
    // chunks at their respective corners. All four should report the
    // same averaged height.
    let c_center = chunks[&ChunkId::new(1, 1)].heightmap().get_height(0, 0);
    let c_nw = chunks[&ChunkId::new(0, 0)]
        .heightmap()
        .get_height(dim - 1, dim - 1);
    let c_ne = chunks[&ChunkId::new(1, 0)]
        .heightmap()
        .get_height(0, dim - 1);
    let c_sw = chunks[&ChunkId::new(0, 1)]
        .heightmap()
        .get_height(dim - 1, 0);
    let max_corner_diff = [(c_center - c_nw), (c_center - c_ne), (c_center - c_sw)]
        .iter()
        .map(|d| d.abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_corner_diff < 1e-5,
        "4-way corner averaging failed: max corner diff {max_corner_diff}"
    );
}
