//! Phase 1.6-F.4.B.3.D.5-diagnostic-3: cross-archetype terrain similarity
//! measurement.
//!
//! Investigation-only third diagnostic in the D.5 chain. Path B reduced max
//! elevations from 1214m to 698m but Andrew-gate revealed four archetypes
//! producing visually identical worlds. This diagnostic measures how much
//! of terrain shape comes from archetype-aware paths (climate → biome →
//! per-biome amplitude) vs archetype-agnostic paths (bootstrap noise).
//!
//! Three measurements:
//!   §1.1 — Cross-archetype heightmap similarity (CT vs Desert; CT vs ET)
//!          at 5 chunk positions. Post-erosion heights compared per-vertex.
//!   §1.2 — Per-source variance decomposition. Bootstrap-only height
//!          (`sample_height` with multiplier=1.0) vs current pipeline
//!          (per-biome blended modulation pre-erosion). Var ratio per
//!          archetype.
//!   §1.3 — Per-archetype blended amplitude distributions (1000 random
//!          positions per archetype, all 6 archetypes).
//!
//! `#[ignore]`-marked. Disposition (keep / retire) decided after audit
//! recommendation lands.
//!
//! See `docs/audits/f4b3d5_diagnostic_3_cross_archetype_2026-04-28.md`.

use astraweave_terrain::biome_lookup::BiomeId;
use astraweave_terrain::biome_param_blending::{
    blend_biome_parameters, BiomeParamBlendConfig,
};
use astraweave_terrain::biome_parameters::BiomeParameters;
use astraweave_terrain::climate::{ClimateConfig, ClimateMap};
use astraweave_terrain::world_archetypes::WorldArchetypeId;
use astraweave_terrain::{ChunkId, ClimateBias, TerrainNoise, WorldConfig, WorldGenerator};
use std::time::Instant;

const SAMPLE_CHUNKS: [(i32, i32); 5] = [(0, 0), (5, 5), (-3, 4), (0, -7), (8, 1)];

fn make_generator(archetype: WorldArchetypeId) -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = 12345;
    config.climate.archetype = archetype.default_archetype();
    WorldGenerator::new(config)
}

fn capture_chunk_heights(gen: &WorldGenerator, chunk_id: ChunkId) -> Vec<f32> {
    let chunk = gen
        .generate_chunk_with_climate(chunk_id, ClimateBias::Temperate)
        .expect("generate_chunk_with_climate");
    let hm = chunk.heightmap();
    let dim = hm.resolution() as usize;
    let mut heights = Vec::with_capacity(dim * dim);
    for z in 0..dim {
        for x in 0..dim {
            heights.push(hm.get_height(x as u32, z as u32));
        }
    }
    heights
}

fn pearson(a: &[f32], b: &[f32]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len() as f64;
    let mean_a = a.iter().map(|&v| v as f64).sum::<f64>() / n;
    let mean_b = b.iter().map(|&v| v as f64).sum::<f64>() / n;
    let mut num = 0.0;
    let mut da_sq = 0.0;
    let mut db_sq = 0.0;
    for i in 0..a.len() {
        let da = a[i] as f64 - mean_a;
        let db = b[i] as f64 - mean_b;
        num += da * db;
        da_sq += da * da;
        db_sq += db * db;
    }
    let denom = (da_sq * db_sq).sqrt();
    if denom < 1e-9 {
        0.0
    } else {
        num / denom
    }
}

fn variance(values: &[f32]) -> f64 {
    let n = values.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let mean = values.iter().map(|&v| v as f64).sum::<f64>() / n;
    let var = values
        .iter()
        .map(|&v| {
            let d = v as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    var
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f32 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

// ============================================================
// §1.1 — Cross-archetype heightmap similarity
// ============================================================

fn measure_cross_archetype_pair(
    label_a: &str,
    archetype_a: WorldArchetypeId,
    label_b: &str,
    archetype_b: WorldArchetypeId,
) {
    println!();
    println!("=== §1.1: {} vs {} cross-archetype similarity ===", label_a, label_b);
    let gen_a = make_generator(archetype_a);
    let gen_b = make_generator(archetype_b);

    let mut all_a: Vec<f32> = Vec::new();
    let mut all_b: Vec<f32> = Vec::new();
    let mut all_deltas: Vec<f32> = Vec::new();

    println!();
    println!(
        "| Chunk         | Mean |Δ| | p50 |Δ| | p95 |Δ| | Max |Δ| | Pearson |"
    );
    println!(
        "| ------------- | -------- | -------- | -------- | -------- | ------- |"
    );
    for &(cx, cz) in &SAMPLE_CHUNKS {
        let chunk_id = ChunkId::new(cx, cz);
        let heights_a = capture_chunk_heights(&gen_a, chunk_id);
        let heights_b = capture_chunk_heights(&gen_b, chunk_id);

        let mut deltas: Vec<f32> = heights_a
            .iter()
            .zip(&heights_b)
            .map(|(a, b)| (a - b).abs())
            .collect();
        deltas.sort_by(|x, y| x.partial_cmp(y).unwrap());

        let mean_abs =
            deltas.iter().map(|&d| d as f64).sum::<f64>() / deltas.len() as f64;
        let p50 = percentile(&deltas, 0.50);
        let p95 = percentile(&deltas, 0.95);
        let max = *deltas.last().unwrap_or(&0.0);
        let r = pearson(&heights_a, &heights_b);

        println!(
            "| ({:>3},{:>3})    | {:8.2} | {:8.2} | {:8.2} | {:8.2} | {:7.3} |",
            cx, cz, mean_abs, p50, p95, max, r
        );

        all_a.extend(heights_a);
        all_b.extend(heights_b);
        all_deltas.extend(deltas);
    }

    all_deltas.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let mean_all = all_deltas.iter().map(|&d| d as f64).sum::<f64>()
        / all_deltas.len() as f64;
    let r_all = pearson(&all_a, &all_b);
    println!(
        "| AGGREGATE     | {:8.2} | {:8.2} | {:8.2} | {:8.2} | {:7.3} |",
        mean_all,
        percentile(&all_deltas, 0.50),
        percentile(&all_deltas, 0.95),
        all_deltas.last().unwrap_or(&0.0),
        r_all
    );

    println!();
    println!(
        "{} vs {} aggregate: mean |Δ|={:.2}m, max |Δ|={:.2}m, Pearson={:.4}",
        label_a,
        label_b,
        mean_all,
        all_deltas.last().unwrap_or(&0.0),
        r_all
    );
}

// ============================================================
// §1.2 — Per-source variance decomposition
// ============================================================

fn measure_variance_decomposition(archetype: WorldArchetypeId, label: &str) {
    println!();
    println!("=== §1.2: {} variance decomposition (chunk (0,0)) ===", label);
    let mut config = WorldConfig::default();
    config.seed = 12345;
    config.climate.archetype = archetype.default_archetype();

    // Reconstruct the underlying TerrainNoise for bootstrap-only sampling.
    let noise = TerrainNoise::new(&config.noise, config.seed);
    let climate = ClimateMap::new(&config.climate, config.seed + 1);
    let blend_cfg = BiomeParamBlendConfig::default();

    let chunk_size = config.chunk_size;
    let chunk_res = config.heightmap_resolution as usize;
    let step = chunk_size / (chunk_res as f32 - 1.0);
    let halo_origin_x = 0.0_f32; // chunk (0,0) origin
    let halo_origin_z = 0.0_f32;

    let mut bootstrap_only: Vec<f32> = Vec::with_capacity(chunk_res * chunk_res);
    let mut with_modulation: Vec<f32> = Vec::with_capacity(chunk_res * chunk_res);

    for z_idx in 0..chunk_res {
        for x_idx in 0..chunk_res {
            let wx_f32 = halo_origin_x + x_idx as f32 * step;
            let wz_f32 = halo_origin_z + z_idx as f32 * step;
            let wx = wx_f32 as f64;
            let wz = wz_f32 as f64;

            // Bootstrap-only height: per-biome multiplier = 1.0 (which is
            // exactly what `sample_height` does internally).
            let h_bootstrap = noise.sample_height(wx, wz);

            // Current pipeline height pre-erosion: same as
            // apply_per_biome_modulation_to_halo's per-vertex compute
            // (D.3b/D.4 path).
            let blended = blend_biome_parameters(wx_f32, wz_f32, h_bootstrap, &climate, &blend_cfg);
            let h_modulated = noise.sample_height_with_mountain_amplitude(
                wx,
                wz,
                blended.mountains_amplitude as f32,
            );

            bootstrap_only.push(h_bootstrap);
            with_modulation.push(h_modulated);
        }
    }

    let var_bootstrap = variance(&bootstrap_only);
    let var_modulated = variance(&with_modulation);
    let deltas: Vec<f32> = bootstrap_only
        .iter()
        .zip(&with_modulation)
        .map(|(a, b)| b - a)
        .collect();
    let var_delta = variance(&deltas);
    let mean_bootstrap =
        bootstrap_only.iter().map(|&v| v as f64).sum::<f64>() / bootstrap_only.len() as f64;
    let mean_modulated =
        with_modulation.iter().map(|&v| v as f64).sum::<f64>() / with_modulation.len() as f64;
    let mean_delta = deltas.iter().map(|&v| v as f64).sum::<f64>() / deltas.len() as f64;

    let mut sorted_b = bootstrap_only.clone();
    sorted_b.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let mut sorted_m = with_modulation.clone();
    sorted_m.sort_by(|x, y| x.partial_cmp(y).unwrap());

    println!();
    println!("Bootstrap-only (mountains_amplitude=1.0 everywhere, no climate path):");
    println!(
        "  mean={:.2}m  variance={:.2}m²  p50={:.2}m  p95={:.2}m  max={:.2}m",
        mean_bootstrap,
        var_bootstrap,
        percentile(&sorted_b, 0.50),
        percentile(&sorted_b, 0.95),
        sorted_b.last().unwrap_or(&0.0)
    );
    println!();
    println!("With per-biome modulation (current D.5-fix pipeline pre-erosion):");
    println!(
        "  mean={:.2}m  variance={:.2}m²  p50={:.2}m  p95={:.2}m  max={:.2}m",
        mean_modulated,
        var_modulated,
        percentile(&sorted_m, 0.50),
        percentile(&sorted_m, 0.95),
        sorted_m.last().unwrap_or(&0.0)
    );
    println!();
    println!("Delta (modulation - bootstrap):");
    println!(
        "  mean={:.2}m  variance={:.2}m²",
        mean_delta, var_delta
    );
    println!();
    println!(
        "Ratio var(delta) / var(bootstrap) = {:.4} ({:.2}% per-biome contribution)",
        var_delta / var_bootstrap.max(1e-9),
        100.0 * var_delta / var_bootstrap.max(1e-9)
    );
}

// ============================================================
// §1.3 — Per-archetype blended amplitude distributions
// ============================================================

fn det_random(seed: u32, idx: u32) -> f32 {
    let mut h = seed.wrapping_mul(0x9E37_79B9).wrapping_add(idx);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

fn measure_amplitude_distribution(archetype: WorldArchetypeId, label: &str) {
    let mut config = ClimateConfig::default();
    config.archetype = archetype.default_archetype();
    let climate = ClimateMap::new(&config, 12345);
    let blend_cfg = BiomeParamBlendConfig::default();

    let half = config.world_latitude_half_extent_wu as f64;
    let mut amplitudes = Vec::<f32>::with_capacity(1000);
    for i in 0..1000u32 {
        let x = (det_random(7, i * 2) * 2.0 - 1.0) as f32 * half as f32;
        let z = (det_random(7, i * 2 + 1) * 2.0 - 1.0) as f32 * half as f32;
        let elev_t = det_random(11, i);
        let elevation = -10.0 + elev_t * 520.0;
        let blended = blend_biome_parameters(x, z, elevation, &climate, &blend_cfg);
        amplitudes.push(blended.mountains_amplitude as f32);
    }
    amplitudes.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mean = amplitudes.iter().map(|&v| v as f64).sum::<f64>() / amplitudes.len() as f64;
    let var = variance(&amplitudes);
    let stddev = var.sqrt();

    let mut bins = [0u32; 5];
    for &amp in &amplitudes {
        let idx = ((amp / 0.5) as usize).min(4);
        bins[idx] += 1;
    }

    println!();
    println!("=== §1.3 {}: blended amplitude distribution (1000 samples) ===", label);
    println!(
        "  mean={:.3}  stddev={:.3}  p25={:.3}  p50={:.3}  p75={:.3}  p95={:.3}",
        mean,
        stddev,
        percentile(&amplitudes, 0.25),
        percentile(&amplitudes, 0.50),
        percentile(&amplitudes, 0.75),
        percentile(&amplitudes, 0.95)
    );
    println!("  Histogram:");
    for (i, &count) in bins.iter().enumerate() {
        let lo = i as f32 * 0.5;
        let hi = (i + 1) as f32 * 0.5;
        println!(
            "    [{:.1}, {:.1}): {:4} ({:5.1}%)",
            lo,
            hi,
            count,
            count as f32 / 10.0
        );
    }
}

// ============================================================
// Test entry point
// ============================================================

#[test]
#[ignore]
fn d5_diagnostic_3_cross_archetype_terrain_similarity() {
    let t0 = Instant::now();
    println!();
    println!("=== F.4.B.3.D.5-diagnostic-3 Cross-Archetype Terrain Similarity ===");
    println!("Seed: 12345");
    println!("Sample chunks: {:?}", SAMPLE_CHUNKS);

    // §1.1 — Cross-archetype heightmap similarity.
    measure_cross_archetype_pair(
        "Continental Temperate",
        WorldArchetypeId::ContinentalTemperate,
        "Desert",
        WorldArchetypeId::Desert,
    );
    measure_cross_archetype_pair(
        "Continental Temperate",
        WorldArchetypeId::ContinentalTemperate,
        "Equatorial Tropical",
        WorldArchetypeId::EquatorialTropical,
    );

    // §1.2 — Per-source variance decomposition.
    measure_variance_decomposition(WorldArchetypeId::ContinentalTemperate, "Continental Temperate");
    measure_variance_decomposition(WorldArchetypeId::Desert, "Desert");
    measure_variance_decomposition(WorldArchetypeId::EquatorialTropical, "Equatorial Tropical");

    // §1.3 — Per-archetype amplitude distributions (all 6).
    println!();
    println!("=== §1.3 Per-archetype blended amplitude distributions ===");
    measure_amplitude_distribution(WorldArchetypeId::ContinentalTemperate, "Continental Temperate");
    measure_amplitude_distribution(WorldArchetypeId::EquatorialTropical, "Equatorial Tropical");
    measure_amplitude_distribution(WorldArchetypeId::BorealSubarctic, "Boreal/Subarctic");
    measure_amplitude_distribution(WorldArchetypeId::Mediterranean, "Mediterranean");
    measure_amplitude_distribution(WorldArchetypeId::Desert, "Desert");
    measure_amplitude_distribution(WorldArchetypeId::Custom, "Custom (= Continental Temperate)");

    println!();
    println!("Total wall-clock: {:.2}s", t0.elapsed().as_secs_f32());

    // Touch BiomeParameters + BiomeId so unused-import warnings go away
    // (we use them indirectly via blend_biome_parameters but the linter
    // can't trace through trait method calls).
    let _ = BiomeParameters::for_biome(BiomeId::Alpine);
}
