//! Phase 1.6-F.4.B.3.D.5-diagnostic-2: real-heightmap biome distribution
//! measurement.
//!
//! Investigation-only follow-up to D.5-diagnostic. Generates actual chunks
//! (not synthetic uniform-elevation samples) for Continental Temperate +
//! Equatorial Tropical archetypes at radius 5 / seed 12345, then aggregates
//! per-vertex (elevation, biome_id) pairs into class-fraction, elevation-
//! distribution, per-biome-elevation, and spatial-pattern statistics.
//!
//! Marked `#[ignore]` — measurement, not regression. Disposition (keep /
//! retire) decided at next session.
//!
//! See `docs/audits/f4b3d5_diagnostic_2_real_heightmap_2026-04-28.md` for
//! the audit produced from this test's output.

use astraweave_terrain::biome_lookup::BiomeId;
use astraweave_terrain::world_archetypes::WorldArchetypeId;
use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BiomeClass {
    Aquatic,
    MountainCharacter,
    Terrestrial,
    Other,
}

fn classify(b: BiomeId) -> BiomeClass {
    if b.is_aquatic() {
        BiomeClass::Aquatic
    } else if matches!(
        b,
        BiomeId::Alpine
            | BiomeId::SnowCap
            | BiomeId::MountainRocky
            | BiomeId::Scree
    ) {
        BiomeClass::MountainCharacter
    } else if b.is_terrestrial() {
        // Note: BiomeId::is_terrestrial includes Alpine; but the order
        // above intercepts it first, so this branch only sees the 10
        // non-Alpine terrestrial biomes.
        BiomeClass::Terrestrial
    } else {
        BiomeClass::Other
    }
}

fn class_char(c: BiomeClass) -> char {
    match c {
        BiomeClass::Aquatic => '~',
        BiomeClass::MountainCharacter => 'M',
        BiomeClass::Terrestrial => '.',
        BiomeClass::Other => '?',
    }
}

fn pct(num: usize, denom: usize) -> f32 {
    if denom == 0 {
        0.0
    } else {
        num as f32 * 100.0 / denom as f32
    }
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f32 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn hist_bin(elev: f32) -> usize {
    if elev < 0.0 {
        0
    } else if elev < 50.0 {
        1
    } else if elev < 100.0 {
        2
    } else if elev < 200.0 {
        3
    } else if elev < 280.0 {
        4
    } else if elev < 350.0 {
        5
    } else if elev < 450.0 {
        6
    } else {
        7
    }
}

const HIST_LABELS: [&str; 8] = [
    "<0     ", "0-50   ", "50-100 ", "100-200", "200-280", "280-350", "350-450", "450+   ",
];

fn measure_archetype(
    archetype_id: WorldArchetypeId,
    label: &str,
    spatial_dump_chunk: ChunkId,
) {
    println!();
    println!("== {} ==", label);

    let mut config = WorldConfig::default();
    config.seed = 12345;
    config.climate.archetype = archetype_id.default_archetype();
    let dim_for_dump = config.heightmap_resolution as usize;
    let gen = WorldGenerator::new(config);

    let radius: i32 = 5;
    let chunks_per_side = (radius * 2 + 1) as usize; // 11
    let mut all_elev_biome: Vec<(f32, BiomeId)> = Vec::new();

    let mut spatial_dump_grid: Option<Vec<BiomeClass>> = None;

    let t0 = Instant::now();

    for x in -radius..=radius {
        for z in -radius..=radius {
            let chunk_id = ChunkId::new(x, z);
            // Use Temperate climate bias for both archetypes — climate-bias
            // drives erosion preset selection (a chunk-level concern), not
            // biome assignment. The archetype drives per-vertex biome
            // assignment via the climate field.
            let chunk = gen
                .generate_chunk_with_climate(chunk_id, ClimateBias::Temperate)
                .expect("generate_chunk_with_climate should succeed");

            let hm = chunk.heightmap();
            let dim = hm.resolution() as usize;
            let biome_ids = chunk
                .biome_ids()
                .expect("biome_ids must be populated by D.3b path");

            // Sanity: lengths match.
            assert_eq!(
                biome_ids.len(),
                dim * dim,
                "biome_ids length should equal dim^2"
            );

            for zi in 0..dim {
                for xi in 0..dim {
                    let h = hm.get_height(xi as u32, zi as u32);
                    let b = biome_ids[zi * dim + xi];
                    all_elev_biome.push((h, b));
                }
            }

            if chunk_id == spatial_dump_chunk {
                let mut grid = Vec::with_capacity(dim * dim);
                for &b in biome_ids {
                    grid.push(classify(b));
                }
                spatial_dump_grid = Some(grid);
            }
        }
    }

    let elapsed = t0.elapsed().as_secs_f32();
    let total = all_elev_biome.len();
    println!("Total vertices: {}", total);
    println!(
        "Generation time: {:.2}s ({} chunks of {}^2 each)",
        elapsed,
        chunks_per_side * chunks_per_side,
        dim_for_dump
    );

    // === A) Biome class fractions ===
    let mut counts = [0usize; 4];
    for &(_, b) in &all_elev_biome {
        let idx = match classify(b) {
            BiomeClass::Aquatic => 0,
            BiomeClass::MountainCharacter => 1,
            BiomeClass::Terrestrial => 2,
            BiomeClass::Other => 3,
        };
        counts[idx] += 1;
    }
    println!();
    println!("Biome class fractions:");
    println!(
        "  Aquatic:           {:6.2}% ({})",
        pct(counts[0], total),
        counts[0]
    );
    println!(
        "  MountainCharacter: {:6.2}% ({})",
        pct(counts[1], total),
        counts[1]
    );
    println!(
        "  Terrestrial:       {:6.2}% ({})",
        pct(counts[2], total),
        counts[2]
    );
    println!(
        "  Other:             {:6.2}% ({})",
        pct(counts[3], total),
        counts[3]
    );

    // === B) Elevation distribution conditional on MountainCharacter class ===
    let mc_elevs: Vec<f32> = all_elev_biome
        .iter()
        .filter(|(_, b)| classify(*b) == BiomeClass::MountainCharacter)
        .map(|(h, _)| *h)
        .collect();

    println!();
    println!(
        "MountainCharacter elevation distribution ({} verts, {:.2}% of total):",
        mc_elevs.len(),
        pct(mc_elevs.len(), total)
    );
    if !mc_elevs.is_empty() {
        let mut bins = [0usize; 8];
        for &e in &mc_elevs {
            bins[hist_bin(e)] += 1;
        }
        let mc_total = mc_elevs.len();
        println!("  Histogram (within MC class):");
        for i in 0..8 {
            println!(
                "    {} m: {:7} ({:5.2}%)",
                HIST_LABELS[i],
                bins[i],
                pct(bins[i], mc_total)
            );
        }
        let mut sorted = mc_elevs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "  Stats: min={:.1} p25={:.1} p50={:.1} p75={:.1} max={:.1}",
            sorted[0],
            percentile(&sorted, 0.25),
            percentile(&sorted, 0.50),
            percentile(&sorted, 0.75),
            sorted[sorted.len() - 1]
        );

        // Above/below 280m.
        let above = mc_elevs.iter().filter(|&&e| e >= 280.0).count();
        let below = mc_elevs.len() - above;
        println!(
            "  Above 280m: {:7} ({:5.2}% of MC, {:5.2}% of total)",
            above,
            pct(above, mc_elevs.len()),
            pct(above, total)
        );
        println!(
            "  Below 280m: {:7} ({:5.2}% of MC, {:5.2}% of total)",
            below,
            pct(below, mc_elevs.len()),
            pct(below, total)
        );
    } else {
        println!("  (no mountain-character vertices in this archetype)");
    }

    // === C) Per-biome elevation distribution within MountainCharacter ===
    println!();
    println!("Per-biome elevation distribution (within MountainCharacter):");
    for &mc_biome in &[
        BiomeId::Alpine,
        BiomeId::SnowCap,
        BiomeId::Scree,
        BiomeId::MountainRocky,
    ] {
        let elevs: Vec<f32> = all_elev_biome
            .iter()
            .filter(|(_, b)| *b == mc_biome)
            .map(|(h, _)| *h)
            .collect();
        if elevs.is_empty() {
            println!("  {:14?}: count=0", mc_biome);
            continue;
        }
        let mut sorted = elevs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "  {:14?}: count={:7} elev: min={:.1} p25={:.1} p50={:.1} p75={:.1} max={:.1}",
            mc_biome,
            elevs.len(),
            sorted[0],
            percentile(&sorted, 0.25),
            percentile(&sorted, 0.50),
            percentile(&sorted, 0.75),
            sorted[sorted.len() - 1]
        );
        // Threshold sanity per lookup_biome:
        //   Scree >= 220, Alpine >= 280, SnowCap >= 350.
        let threshold = match mc_biome {
            BiomeId::Scree => Some(220.0),
            BiomeId::Alpine => Some(280.0),
            BiomeId::SnowCap => Some(350.0),
            _ => None,
        };
        if let Some(t) = threshold {
            let below_threshold =
                elevs.iter().filter(|&&e| e < t).count();
            if below_threshold > 0 {
                println!(
                    "    !! WARNING: {} vertices BELOW threshold {} m ({:.2}% of {} count)",
                    below_threshold,
                    t,
                    pct(below_threshold, elevs.len()),
                    elevs.len(),
                );
            }
        }
    }

    // === D) Spatial pattern dump for selected chunk ===
    if let Some(grid) = spatial_dump_grid {
        println!();
        println!(
            "Spatial pattern dump (chunk {:?}, {}x{} grid: ~ Aquatic, M MountainCharacter, . Terrestrial):",
            spatial_dump_chunk, dim_for_dump, dim_for_dump
        );
        for zi in 0..dim_for_dump {
            let mut line = String::with_capacity(dim_for_dump);
            for xi in 0..dim_for_dump {
                line.push(class_char(grid[zi * dim_for_dump + xi]));
            }
            println!("  {}", line);
        }
    }

    // === E) Top-level overall elevation distribution (sanity context) ===
    println!();
    println!("Overall elevation distribution (all vertices, sanity context):");
    let all_elevs: Vec<f32> = all_elev_biome.iter().map(|(h, _)| *h).collect();
    let mut sorted = all_elevs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut bins = [0usize; 8];
    for &e in &all_elevs {
        bins[hist_bin(e)] += 1;
    }
    println!("  Histogram:");
    for i in 0..8 {
        println!(
            "    {} m: {:7} ({:5.2}%)",
            HIST_LABELS[i],
            bins[i],
            pct(bins[i], all_elevs.len())
        );
    }
    println!(
        "  Stats: min={:.1} p25={:.1} p50={:.1} p75={:.1} max={:.1}",
        sorted[0],
        percentile(&sorted, 0.25),
        percentile(&sorted, 0.50),
        percentile(&sorted, 0.75),
        sorted[sorted.len() - 1]
    );
}

#[test]
#[ignore]
fn d5_diagnostic_2_real_heightmap_biome_distribution() {
    println!();
    println!("=== F.4.B.3.D.5-diagnostic-2 Real-Heightmap Biome Distribution ===");
    println!("Seed: 12345");
    println!("Radius: 5 (121 chunks per archetype)");
    println!("Archetypes: Continental Temperate, Equatorial Tropical");

    measure_archetype(
        WorldArchetypeId::ContinentalTemperate,
        "Continental Temperate",
        ChunkId::new(5, 5),
    );
    measure_archetype(
        WorldArchetypeId::EquatorialTropical,
        "Equatorial Tropical",
        ChunkId::new(0, 0),
    );

    println!();
    println!("=== Comparison Table ===");
    println!(
        "| Archetype | Synthetic MC% (D.5-diag) | Real MC% | Above 280m | Below 280m | Pattern |"
    );
    println!(
        "| --------- | ------------------------ | -------- | ---------- | ---------- | ------- |"
    );
    println!(
        "| Continental Temperate | 41.8% | (see CT block above) | (see CT) | (see CT) | (see dump) |"
    );
    println!(
        "| Equatorial Tropical | 41.7% | (see ET block above) | (see ET) | (see ET) | (see dump) |"
    );
    println!();
    println!(
        "Map measurements to §2 decision matrix in F.4.B.3.D.5-diagnostic-2 audit."
    );
}
