//! Phase 1.6-F.3-phase-2.E: `WorldGenerator::generate_chunk_with_climate`
//! under the real phase-2 pipeline (halo-expand + AdvancedErosion + crop).
//!
//! One chunk per climate. A 5×5 grid times six climates only printed a
//! timing table, asserted nothing, and the Windows CI runner killed it at
//! the 240s slow-timeout. The pipeline is per chunk, so one chunk per
//! climate still runs halo expansion and the climate's erosion preset.

use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};

#[test]
fn phase_1_6_f3_phase_2_end_to_end_timing() {
    let config = WorldConfig {
        seed: 12345,
        ..WorldConfig::default()
    };
    let gen = WorldGenerator::new(config);

    println!("======================================================");
    println!("F.3-phase-2.E: one chunk per climate");
    println!();
    println!("| Climate  | Preset              | chunk ms | span |");
    println!("|----------|---------------------|---------:|-----:|");

    for (climate, preset_name) in [
        (ClimateBias::Temperate, "default_balanced"),
        (ClimateBias::Cold, "mountain_balanced"),
        (ClimateBias::Arid, "desert"),
        (ClimateBias::Tropical, "coastal"),
        (ClimateBias::Wetland, "coastal"),
        (ClimateBias::Highland, "mountain_balanced"),
    ] {
        let started = std::time::Instant::now();
        let chunk = gen
            .generate_chunk_with_climate(ChunkId::new(0, 0), climate)
            .expect("chunk generation");
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        let mut ys: Vec<f32> = chunk.heightmap().data().to_vec();
        let climate_name = format!("{climate:?}");
        assert!(
            !ys.is_empty(),
            "{climate_name} chunk heightmap must not be empty"
        );
        assert!(
            ys.iter().all(|y| y.is_finite()),
            "{climate_name} heights must be finite"
        );
        ys.sort_by(|a, b| a.total_cmp(b));
        let span = ys[ys.len() - 1] - ys[0];
        assert!(
            span > 0.0,
            "{climate_name} chunk must have relief, span {span}"
        );
        println!("| {climate_name:<8} | {preset_name:<19} | {ms:>8.0} | {span:>4.1} |");
    }

    println!("======================================================");
}
