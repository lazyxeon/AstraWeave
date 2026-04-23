//! Phase 1.6-F.3-phase-3.A: diagnostic investigation.
//!
//! Quantifies the two Andrew-gate failures phase 2 uncovered:
//!   (1) **Stitching divergence** at chunk boundaries under real erosion
//!       (phase 2 documented 15-40 world units; this codifies per-climate).
//!   (2) **Mountain scale compression** (erosion's effect on peaks/spread).
//!
//! No behavior change — pure measurement. Runs in release mode; in debug
//! these tests take several minutes each due to 50k+ droplet counts.
//!
//!   cargo test -p astraweave-terrain --release --test
//!     phase_1_6_f3_phase_3_diagnostic -- --nocapture
//!
//! Output feeds `docs/audits/terrain_erosion_seamless_diagnostic_2026-04-24.md`.

use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};

/// Helper: generate a `side`×`side` grid of chunks with a specific climate.
fn grid(
    gen: &WorldGenerator,
    climate: ClimateBias,
    side: i32,
) -> Vec<Vec<astraweave_terrain::TerrainChunk>> {
    (0..side)
        .map(|z| {
            (0..side)
                .map(|x| {
                    gen.generate_chunk_with_climate(ChunkId::new(x, z), climate)
                        .expect("chunk generation")
                })
                .collect()
        })
        .collect()
}

/// Compute X-axis and Z-axis edge divergence distributions across an
/// `n`×`n` chunk grid. Returns flat Vec of abs-diff values (all samples).
fn edge_divergences(
    chunks: &[Vec<astraweave_terrain::TerrainChunk>],
) -> Vec<f32> {
    let n = chunks.len();
    let dim = chunks[0][0].heightmap().resolution();
    let mut samples = Vec::new();

    // X-axis: right column of (x,z) vs left column of (x+1,z).
    for z in 0..n {
        for x in 0..(n - 1) {
            let a = chunks[z][x].heightmap();
            let b = chunks[z][x + 1].heightmap();
            for zi in 0..dim {
                samples.push((a.get_height(dim - 1, zi) - b.get_height(0, zi)).abs());
            }
        }
    }
    // Z-axis: bottom row of (x,z) vs top row of (x,z+1).
    for z in 0..(n - 1) {
        for x in 0..n {
            let a = chunks[z][x].heightmap();
            let b = chunks[z + 1][x].heightmap();
            for xi in 0..dim {
                samples.push((a.get_height(xi, dim - 1) - b.get_height(xi, 0)).abs());
            }
        }
    }
    samples
}

/// Summarize a distribution by mean / percentiles / max.
fn distribution_summary(mut vals: Vec<f32>) -> (f32, f32, f32, f32, f32, f32) {
    if vals.is_empty() {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = vals.iter().sum::<f32>() / vals.len() as f32;
    let p50 = vals[vals.len() / 2];
    let p95 = vals[(vals.len() * 95) / 100];
    let p99 = vals[(vals.len() * 99) / 100];
    let max = *vals.last().unwrap();
    let min = vals[0];
    (mean, p50, p95, p99, max, min)
}

fn make_generator(erosion: bool) -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = 12345;
    config.noise.erosion_enabled = erosion;
    WorldGenerator::new(config)
}

fn climate_name(c: ClimateBias) -> &'static str {
    match c {
        ClimateBias::Temperate => "Temperate",
        ClimateBias::Cold => "Cold",
        ClimateBias::Arid => "Arid",
        ClimateBias::Tropical => "Tropical",
        ClimateBias::Wetland => "Wetland",
        ClimateBias::Highland => "Highland",
    }
}

/// Phase 1.6-F.3-phase-3.A.1: stitching divergence per climate.
/// Measures edge-diff distribution at shared chunk boundaries after real
/// erosion, for comparison against pre-erosion (which should be ~0 by
/// noise determinism) and phase 3's fix target (≤ 1 world unit).
#[test]
fn phase_1_6_f3_phase_3_stitching_per_climate() {
    const SIDE: i32 = 2;
    let climates = [
        ClimateBias::Temperate,
        ClimateBias::Cold,
        ClimateBias::Arid,
        ClimateBias::Tropical,
        ClimateBias::Wetland,
        ClimateBias::Highland,
    ];

    println!("======================================================");
    println!("F.3-phase-3.A.1: stitching divergence per climate");
    println!("  2×2 chunk grid, seed 12345, phase-2 per-halo seeding.");
    println!();
    println!("PRE-EROSION (noise field only, expected ~0):");
    println!("| Climate   |   mean |    p50 |    p95 |    p99 |    max |");
    println!("|-----------|-------:|-------:|-------:|-------:|-------:|");
    let gen_off = make_generator(false);
    for c in climates {
        let chunks = grid(&gen_off, c, SIDE);
        let (mean, p50, p95, p99, max, _min) =
            distribution_summary(edge_divergences(&chunks));
        println!(
            "| {:<9} | {mean:6.3} | {p50:6.3} | {p95:6.3} | {p99:6.3} | {max:6.3} |",
            climate_name(c)
        );
    }

    println!();
    println!("POST-EROSION (real AdvancedErosion, phase-2 per-halo seeding):");
    println!("| Climate   |   mean |    p50 |    p95 |    p99 |    max |");
    println!("|-----------|-------:|-------:|-------:|-------:|-------:|");
    let gen_on = make_generator(true);
    for c in climates {
        let chunks = grid(&gen_on, c, SIDE);
        let (mean, p50, p95, p99, max, _min) =
            distribution_summary(edge_divergences(&chunks));
        println!(
            "| {:<9} | {mean:6.3} | {p50:6.3} | {p95:6.3} | {p99:6.3} | {max:6.3} |",
            climate_name(c)
        );
    }
    println!("======================================================");
}

/// Phase 1.6-F.3-phase-3.A.2: scale compression per climate.
/// Measures how erosion changes peak height + horizontal spread.
/// Records pre-erosion and post-erosion Y distributions to decide whether
/// the "short and thin" Andrew-gate finding is erosion-driven or intrinsic
/// to F.2-T-4 noise.
#[test]
fn phase_1_6_f3_phase_3_scale_compression_per_climate() {
    let climates = [
        ClimateBias::Temperate,
        ClimateBias::Cold,
        ClimateBias::Arid,
        ClimateBias::Tropical,
        ClimateBias::Wetland,
        ClimateBias::Highland,
    ];

    println!("======================================================");
    println!("F.3-phase-3.A.2: scale compression per climate");
    println!("  Single chunk (0,0), seed 12345, pre-vs-post erosion.");
    println!();
    println!(
        "| Climate   | pre.max | pre.p99 | pre.p95 | post.max | post.p99 | post.p95 | Δp99 % |"
    );
    println!(
        "|-----------|--------:|--------:|--------:|---------:|---------:|---------:|-------:|"
    );

    let gen_off = make_generator(false);
    let gen_on = make_generator(true);

    for c in climates {
        let pre = gen_off
            .generate_chunk_with_climate(ChunkId::new(0, 0), c)
            .expect("pre chunk");
        let post = gen_on
            .generate_chunk_with_climate(ChunkId::new(0, 0), c)
            .expect("post chunk");
        let mut pre_ys: Vec<f32> = pre.heightmap().data().to_vec();
        let mut post_ys: Vec<f32> = post.heightmap().data().to_vec();
        pre_ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        post_ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pre_max = *pre_ys.last().unwrap();
        let pre_p99 = pre_ys[(pre_ys.len() * 99) / 100];
        let pre_p95 = pre_ys[(pre_ys.len() * 95) / 100];
        let post_max = *post_ys.last().unwrap();
        let post_p99 = post_ys[(post_ys.len() * 99) / 100];
        let post_p95 = post_ys[(post_ys.len() * 95) / 100];
        let delta_p99 = if pre_p99.abs() > 1e-4 {
            (post_p99 - pre_p99) / pre_p99 * 100.0
        } else {
            0.0
        };
        println!(
            "| {:<9} | {pre_max:7.2} | {pre_p99:7.2} | {pre_p95:7.2} | {post_max:8.2} | {post_p99:8.2} | {post_p95:8.2} | {delta_p99:+6.1} |",
            climate_name(c)
        );
    }
    println!();
    println!("Interpretation:");
    println!("  Δp99 = % change in 99th-percentile height (post - pre) / pre.");
    println!("  Highly negative: erosion reducing peak heights substantially.");
    println!("  Near zero: erosion barely affects peak heights.");
    println!("  If Andrew-gate's 'short and thin' is erosion-driven:");
    println!("    expect Δp99 < -20% on Temperate/Cold/Highland.");
    println!("  If source-noise-driven: expect Δp99 near zero across climates.");
    println!("======================================================");
}

/// Phase 1.6-F.3-phase-3.A.3: overlap-region analysis via output comparison.
/// Demonstrates that adjacent halos produce divergent output in the same
/// world-coordinate region because their RNG streams differ. Directly
/// evidences the root cause documented in phase 2 §10.
///
/// Method: for two adjacent chunks A=(0,0) and B=(1,0), their halos
/// overlap in world-space X=[0, 256] × Z=[-256, 256]. Within each halo,
/// the region mapping to chunk A's right edge (world X=256) is:
///   - In A's halo: local x index 2*63 = 126 (out of 190).
///   - In B's halo: local x index 1*63 = 63.
/// Same world coordinates; different halo-local positions; different RNG
/// seeds → different erosion output. Measured here so phase 3 can confirm
/// the fix drops this divergence.
#[test]
fn phase_1_6_f3_phase_3_overlap_divergence_characterization() {
    println!("======================================================");
    println!("F.3-phase-3.A.3: overlap-region erosion divergence");
    println!("  Target: chunk A=(0,0) right edge vs chunk B=(1,0) left edge.");
    println!("  Both edges sample the same world coordinates; erosion makes");
    println!("  them diverge because each halo uses a different seed.");
    println!();

    let gen = make_generator(true);
    let chunk_a = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("A");
    let chunk_b = gen
        .generate_chunk_with_climate(ChunkId::new(1, 0), ClimateBias::Temperate)
        .expect("B");
    let hm_a = chunk_a.heightmap();
    let hm_b = chunk_b.heightmap();
    let dim = hm_a.resolution();

    let mut diffs = Vec::new();
    for z in 0..dim {
        diffs.push((hm_a.get_height(dim - 1, z) - hm_b.get_height(0, z)).abs());
    }
    let (mean, p50, p95, p99, max, _min) = distribution_summary(diffs.clone());
    println!("Temperate Temperate overlap edge (chunk width):");
    println!(
        "  mean={mean:.3}  p50={p50:.3}  p95={p95:.3}  p99={p99:.3}  max={max:.3}"
    );
    println!();

    // Same experiment, with erosion disabled: expect ~0 everywhere.
    let gen_off = make_generator(false);
    let chunk_a_off = gen_off
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("A off");
    let chunk_b_off = gen_off
        .generate_chunk_with_climate(ChunkId::new(1, 0), ClimateBias::Temperate)
        .expect("B off");
    let hm_a_off = chunk_a_off.heightmap();
    let hm_b_off = chunk_b_off.heightmap();
    let mut diffs_off = Vec::new();
    for z in 0..dim {
        diffs_off.push((hm_a_off.get_height(dim - 1, z) - hm_b_off.get_height(0, z)).abs());
    }
    let (mean_off, _p50, _p95, _p99, max_off, _min) =
        distribution_summary(diffs_off);
    println!("Same edge with erosion disabled:");
    println!("  mean={mean_off:.6}  max={max_off:.6}");
    println!();

    println!("Root cause confirmed: erosion introduces {:.1}× divergence", max / max_off.max(1e-6));
    println!("vs noise-only baseline. Phase 3's world-coord seeding should");
    println!("reduce erosion-case max back to ~same order as noise-only.");
    println!("======================================================");
}
