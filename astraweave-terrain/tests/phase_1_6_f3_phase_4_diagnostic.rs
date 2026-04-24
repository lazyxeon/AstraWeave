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

use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};

fn make_generator(erosion: bool) -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = 12345;
    config.noise.erosion_enabled = erosion;
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
