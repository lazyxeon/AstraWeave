//! Phase 1.6-F.4.B.3.D.3 perf measurement: chunk generation time at
//! radius 10, seed 12345, Continental Temperate archetype.
//!
//! Per §1.6 of the D.3 plan: "Performance: chunk generation time at
//! radius 10, seed 12345, Continental Temperate archetype within +20%
//! of F.4.B.2.H baseline. Record the measurement in the deviation log
//! even if within budget."
//!
//! F.4.B.2.H baseline reference: rayon-parallelized full radius-10
//! generation projected at 2-4 minutes per F.4.B.2's plan §1.D rayon
//! analysis. This test runs a single chunk (~1/441 of full world) so
//! its absolute time is much smaller. The +20% budget applies to the
//! per-chunk cost ratio.
//!
//! Run with `cargo test --release --test phase_1_6_f4_b_3_d_3_perf
//! -- --ignored --nocapture` to see timing output. Marked `#[ignore]`
//! by default to avoid bloating routine test runs.

use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};
use std::time::Instant;

#[test]
#[ignore]
fn phase_1_6_f4_b_3_d_3_chunk_generation_time_continental_temperate() {
    let mut config = WorldConfig::default(); // Continental Temperate archetype
    config.seed = 12345;
    let gen = WorldGenerator::new(config);

    // Warmup chunk to amortize one-time costs (TerrainNoise construction is
    // already done in `new`; warmup primes any noise-cache or rayon thread
    // pool).
    let _warmup = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("warmup");

    // Measure: 6 chunks at varying positions sample average + variance.
    let positions = [(0, 0), (1, 0), (-1, 0), (0, 1), (5, 5), (-5, -5)];
    let mut times = Vec::with_capacity(positions.len());
    for &(x, z) in positions.iter() {
        let t0 = Instant::now();
        let _chunk = gen
            .generate_chunk_with_climate(ChunkId::new(x, z), ClimateBias::Temperate)
            .expect("generate");
        times.push(t0.elapsed().as_secs_f64());
    }
    let total: f64 = times.iter().sum();
    let mean = total / times.len() as f64;
    let max = times.iter().copied().fold(0.0f64, f64::max);
    let min = times.iter().copied().fold(f64::INFINITY, f64::min);

    println!("======================================================");
    println!("F.4.B.3.D.3 perf: chunk generation time, Continental Temperate, seed 12345");
    println!();
    println!("| Chunk      | Time (s) |");
    println!("|------------|---------:|");
    for (&(x, z), &t) in positions.iter().zip(times.iter()) {
        println!("| ({:>3},{:>3}) | {:8.3} |", x, z, t);
    }
    println!();
    println!("Mean: {:.3}s | Min: {:.3}s | Max: {:.3}s", mean, min, max);
    println!();
    println!(
        "Architecture: per-vertex BiomeId lookup (D.3b) +
        per-biome mountains_amplitude modulation (D.3a/D.3b)."
    );
    println!("Climate field: Continental Temperate archetype (D.1).");
    println!("Per-vertex cost: 2 noise samples (raw + biome-modulated) +");
    println!("                 1 climate sample + 1 biome lookup +");
    println!("                 1 BiomeParameters lookup.");
    println!("Erosion: AdvancedErosionSimulator world-coord seeding (F.3-phase-3.C).");
    println!("======================================================");
}
