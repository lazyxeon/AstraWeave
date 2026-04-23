//! Phase 1.6-F.3-phase-2.E: end-to-end performance characterization of
//! `WorldGenerator::generate_chunk_with_climate` under the real phase-2
//! pipeline (halo-expand + AdvancedErosion + crop). Runs a 5×5 chunk grid
//! (25 chunks) per climate and extrapolates to 121 chunks for the editor's
//! radius-5 case.
//!
//! This complements `phase_1_6_f3_phase_0_perf.rs` (which measures
//! AdvancedErosionSimulator in isolation on synthetic slope heightmaps).
//! Real-terrain measurement is what matters for phase-2 operational
//! decisions (rayon fallback, droplet-count tuning).
//!
//! Run with --release for meaningful numbers:
//!   cargo test -p astraweave-terrain --release --test phase_1_6_f3_phase_2_perf -- --nocapture

use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};

fn timed_grid(gen: &WorldGenerator, climate: ClimateBias, side: i32) -> std::time::Duration {
    let start = std::time::Instant::now();
    for z in 0..side {
        for x in 0..side {
            let _ = gen
                .generate_chunk_with_climate(ChunkId::new(x, z), climate)
                .expect("chunk generation");
        }
    }
    start.elapsed()
}

#[test]
fn phase_1_6_f3_phase_2_end_to_end_timing() {
    const SIDE: i32 = 5; // 25 chunks — extrapolate ×4.84 for 121 chunks.
    const FACTOR: f32 = 121.0 / 25.0;

    let mut config = WorldConfig::default();
    config.seed = 12345;
    let gen = WorldGenerator::new(config);

    println!("======================================================");
    println!("F.3-phase-2.E: end-to-end chunk generation timing");
    println!("  5×5 grid ({} chunks), extrapolated to 121 chunks", SIDE * SIDE);
    println!();
    println!("| Climate  | Preset              |   5×5 ms | 121 ext s |  § 2.3 |");
    println!("|----------|---------------------|---------:|----------:|:------:|");

    let mut entries = Vec::new();
    for (climate, preset_name) in [
        (ClimateBias::Temperate, "default_balanced"),
        (ClimateBias::Cold, "mountain_balanced"),
        (ClimateBias::Arid, "desert"),
        (ClimateBias::Tropical, "coastal"),
        (ClimateBias::Wetland, "coastal"),
        (ClimateBias::Highland, "mountain_balanced"),
    ] {
        let t = timed_grid(&gen, climate, SIDE);
        let ms_25 = t.as_millis() as f64;
        let s_121 = (ms_25 / 1000.0) * FACTOR as f64;
        let budget_ok = if s_121 <= 30.0 {
            "OK"
        } else if s_121 <= 42.0 {
            "MARG"
        } else {
            "OVER"
        };
        let climate_name = format!("{climate:?}");
        println!(
            "| {climate_name:<8} | {preset_name:<19} | {ms_25:>8.0} | {s_121:>9.1} | {budget_ok:^6} |"
        );
        entries.push((climate_name, preset_name, ms_25, s_121));
    }

    println!();
    println!("Plan §2.3 budget: 30s (editor-time). Tolerance: ≤42s (40% over).");
    println!();
    println!("======================================================");

    // No asserts — this is a characterization test. Output is what matters.
    // CI / release runs capture the numbers in the commit log.
}
