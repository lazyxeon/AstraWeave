//! Phase 1.6-F.3-phase-1.B: halo scaffolding tests. Verifies that the
//! halo-expansion + center-crop MACHINERY produces chunk heightmaps
//! byte-identical (within float tolerance) to the legacy single-chunk path,
//! and that adjacent chunks' shared edges match in world-coordinate space.
//!
//! **Phase 2 update (2026-04-23):** F.3-phase-2.C wired
//! `AdvancedErosionSimulator::apply_preset` on the halo heightmap, which
//! produces behavioral divergence between the halo path and the legacy
//! single-chunk path (halos have per-halo-origin seeds → different droplet
//! trajectories → different post-erosion output). These phase 1 tests run
//! with `erosion_enabled = false` to continue validating the MACHINERY in
//! isolation; phase 2's behavioral continuity under real erosion is
//! covered by `phase_1_6_f3_phase_2_continuity.rs`.

use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};

fn make_generator(seed: u64) -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = seed;
    // Phase 2 note: erosion disabled here so the halo+crop machinery is
    // testable in isolation. The MACHINERY (halo sampling at per-vertex
    // world coords, center crop, byte-identity to single-chunk SIMD
    // generation) is unchanged by phase 2's AdvancedErosionSimulator
    // wiring. Phase 2's behavioral effects on adjacent-chunk continuity
    // are measured in `phase_1_6_f3_phase_2_continuity.rs`.
    config.noise.erosion_enabled = false;
    WorldGenerator::new(config)
}

#[test]
fn halo_cropped_heightmap_matches_single_chunk_generation() {
    // The halo path generates 3×3 chunks at per-vertex spacing matching
    // single-chunk generation, then crops the center. Because
    // `TerrainNoise::sample_height` is deterministic per world (x, z), the
    // cropped heights must match what `generate_chunk` produces for the same
    // chunk via the SIMD (or scalar) heightmap generator — up to float
    // precision differences in accumulator ordering.
    //
    // Tolerance: 0.01 world units. Runs with erosion_enabled=false so
    // phase 2's AdvancedErosion doesn't confound the machinery test.
    let gen = make_generator(12345);
    let legacy = gen
        .generate_chunk(ChunkId::new(0, 0))
        .expect("legacy generation should succeed");
    let new_path = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("new generation should succeed");

    let legacy_hm = legacy.heightmap();
    let new_hm = new_path.heightmap();
    assert_eq!(
        legacy_hm.resolution(),
        new_hm.resolution(),
        "heightmap resolutions must match"
    );

    let dim = legacy_hm.resolution();
    let mut max_diff = 0.0f32;
    for z in 0..dim {
        for x in 0..dim {
            let a = legacy_hm.get_height(x, z);
            let b = new_hm.get_height(x, z);
            let d = (a - b).abs();
            if d > max_diff {
                max_diff = d;
            }
        }
    }
    println!("halo vs legacy max height diff (no erosion): {max_diff:.6}");
    assert!(
        max_diff < 0.01,
        "halo-cropped heightmap machinery diverges from legacy single-chunk by {max_diff:.6}"
    );
}

#[test]
fn halo_preserves_adjacent_chunk_edge_continuity() {
    // Chunks (0,0) and (1,0) share a world-coordinate edge. With erosion
    // disabled, both sides sample the same noise field at the same world
    // coords, producing byte-identical edge vertices. This test validates
    // the halo+crop machinery preserves noise-field determinism.
    //
    // Phase 2 behavioral continuity under real erosion (with its inherent
    // per-halo-seed divergence) is tested in
    // `phase_1_6_f3_phase_2_continuity.rs` with appropriately wider
    // tolerances.
    let gen = make_generator(12345);
    let chunk_a = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("chunk (0,0) generation");
    let chunk_b = gen
        .generate_chunk_with_climate(ChunkId::new(1, 0), ClimateBias::Temperate)
        .expect("chunk (1,0) generation");

    let hm_a = chunk_a.heightmap();
    let hm_b = chunk_b.heightmap();
    let dim = hm_a.resolution();

    let mut max_diff = 0.0f32;
    for z in 0..dim {
        let a = hm_a.get_height(dim - 1, z);
        let b = hm_b.get_height(0, z);
        let d = (a - b).abs();
        if d > max_diff {
            max_diff = d;
        }
    }
    println!("adjacent chunk edge max height diff (no erosion): {max_diff:.4}");
    // With erosion disabled, noise determinism gives near-zero diff.
    assert!(
        max_diff < 0.01,
        "adjacent chunks' shared edge diverges by {max_diff:.4} (> 0.01 unit tolerance, \
         erosion disabled — machinery bug)"
    );
}

#[test]
fn halo_generation_deterministic_per_seed() {
    // Two generators with the same seed must produce identical chunks via the
    // halo path. Required for phase 2's halo-overlap convergence.
    let gen1 = make_generator(12345);
    let gen2 = make_generator(12345);

    let c1 = gen1
        .generate_chunk_with_climate(ChunkId::new(3, -2), ClimateBias::Highland)
        .expect("generation");
    let c2 = gen2
        .generate_chunk_with_climate(ChunkId::new(3, -2), ClimateBias::Highland)
        .expect("generation");

    let h1 = c1.heightmap();
    let h2 = c2.heightmap();
    let dim = h1.resolution();
    for z in 0..dim {
        for x in 0..dim {
            let a = h1.get_height(x, z);
            let b = h2.get_height(x, z);
            assert!(
                (a - b).abs() < 1e-6,
                "deterministic generation failed at ({x}, {z}): {a} vs {b}"
            );
        }
    }
}

#[test]
fn halo_generation_differs_across_seeds() {
    let gen1 = make_generator(12345);
    let gen2 = make_generator(67890);

    let c1 = gen1
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("generation");
    let c2 = gen2
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("generation");

    let h1 = c1.heightmap();
    let h2 = c2.heightmap();
    let dim = h1.resolution();
    let mut differ_count = 0;
    for z in 0..dim {
        for x in 0..dim {
            let a = h1.get_height(x, z);
            let b = h2.get_height(x, z);
            if (a - b).abs() > 0.1 {
                differ_count += 1;
            }
        }
    }
    // At least 90% of vertices should differ meaningfully between seeds.
    let total = (dim * dim) as u64;
    let ratio = differ_count as f64 / total as f64;
    println!("differ ratio across seeds: {:.2}%", ratio * 100.0);
    assert!(
        ratio > 0.9,
        "seeds produce suspiciously similar output: only {:.1}% of vertices differ",
        ratio * 100.0
    );
}
