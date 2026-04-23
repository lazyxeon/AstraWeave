//! Phase 1.6-F.3-phase-1.B: halo scaffolding tests. Verifies that the
//! halo-expansion + center-crop path produces chunk heightmaps byte-identical
//! (within float tolerance) to the legacy single-chunk path, and that
//! adjacent chunks' shared edges match in world-coordinate space.

use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};

fn make_generator(seed: u64) -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = seed;
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
    // Tolerance: 0.01 world units. If SIMD vs scalar paths ever introduce
    // larger divergence, this test should be revisited, not relaxed.
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
    // Simple CA erosion runs in both paths with the same inputs (same heights,
    // same strength); outputs match within fp precision.
    println!("halo vs legacy max height diff: {max_diff:.6}");
    assert!(
        max_diff < 0.01,
        "halo-cropped heightmap diverges from legacy single-chunk by {max_diff:.6}"
    );
}

#[test]
fn halo_preserves_adjacent_chunk_edge_continuity() {
    // Chunks (0,0) and (1,0) share a world-coordinate edge. Under both the
    // legacy single-chunk path and the new halo+crop path, the edge vertices
    // must have identical Y values (they sample the same world coordinates).
    //
    // This is trivially true for legacy (noise is deterministic) but the test
    // also verifies the halo path preserves it.
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

    // Chunk A's rightmost column (x = dim-1) maps to the SAME world coordinates
    // as Chunk B's leftmost column (x = 0). Simple CA erosion is per-chunk so
    // there may be small boundary differences; allow 1.0-unit tolerance.
    let mut max_diff = 0.0f32;
    for z in 0..dim {
        let a = hm_a.get_height(dim - 1, z);
        let b = hm_b.get_height(0, z);
        let d = (a - b).abs();
        if d > max_diff {
            max_diff = d;
        }
    }
    println!("adjacent chunk edge max height diff: {max_diff:.4}");
    // 1.0 world unit tolerance accommodates simple CA erosion boundary effects.
    // Phase 2's halo-based erosion will tighten this bound significantly.
    assert!(
        max_diff < 1.0,
        "adjacent chunks' shared edge diverges by {max_diff:.4} (> 1.0 unit tolerance)"
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
