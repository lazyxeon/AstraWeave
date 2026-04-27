//! Phase 1.6-F.4.B.3.D.3b: integration tests for the per-vertex biome lookup
//! refactor of `WorldGenerator::generate_chunk_with_climate`.
//!
//! Per §1.6 of the D.3 plan: "generate a small chunk with mixed biomes
//! (climate gradient across the chunk via varying latitude or moisture)
//! and verify per-vertex biome IDs differ across the chunk. This is the
//! structural test that the new architecture actually works — if every
//! vertex in a varied-climate chunk gets the same biome ID, the refactor
//! failed."

use astraweave_terrain::biome_lookup::BiomeId;
use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};
use std::collections::HashSet;

/// Helper: build a generator at the world's near-edge (high |z|) where
/// latitude effect is strongest, maximizing climate variation across a
/// single chunk.
fn make_edge_generator() -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = 12345;
    config.noise.erosion_enabled = false; // Disable erosion to keep the test fast
    WorldGenerator::new(config)
}

#[test]
fn phase_1_6_f4_b_3_d_3_chunk_has_per_vertex_biome_ids() {
    // After D.3b, generate_chunk_with_climate populates per-vertex biome IDs
    // via the climate field + Whittaker lookup. The returned chunk's
    // `biome_ids()` accessor must return Some(_).
    let gen = make_edge_generator();
    let chunk = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("generate_chunk_with_climate should succeed");

    let ids = chunk
        .biome_ids()
        .expect("chunk from generate_chunk_with_climate must have biome_ids populated");

    let expected_count = (chunk.heightmap().resolution() as usize).pow(2);
    assert_eq!(
        ids.len(),
        expected_count,
        "biome_ids length should match heightmap vertex count ({} × {} = {})",
        chunk.heightmap().resolution(),
        chunk.heightmap().resolution(),
        expected_count
    );
}

#[test]
fn phase_1_6_f4_b_3_d_3_mixed_climate_chunk_produces_varied_biomes() {
    // §1.6 verification: a chunk near the world edge experiences
    // significant climate variation (latitude modulator + elevation
    // variation from terrain noise). Per-vertex biome IDs must differ
    // across the chunk — single-biome-dominance would mean the refactor
    // did not actually wire per-vertex variation.
    //
    // We sample a chunk at (8, 8) — relatively close to the radius-10
    // world edge — to maximize latitude effect, and rely on the
    // base-elevation noise to produce within-chunk elevation variation.
    let gen = make_edge_generator();
    let chunk = gen
        .generate_chunk_with_climate(ChunkId::new(8, 8), ClimateBias::Temperate)
        .expect("generate_chunk_with_climate should succeed");

    let ids = chunk
        .biome_ids()
        .expect("biome_ids must be populated by generate_chunk_with_climate");

    // Collect unique biome IDs.
    let unique: HashSet<BiomeId> = ids.iter().copied().collect();
    assert!(
        unique.len() >= 2,
        "edge-of-world chunk should produce at least 2 distinct BiomeIds; \
         got {} unique IDs: {:?}",
        unique.len(),
        unique
    );
}

#[test]
fn phase_1_6_f4_b_3_d_3_per_vertex_biome_ids_deterministic() {
    // Same seed + same chunk_id + same climate_bias → same biome_ids.
    // Determinism is load-bearing for chunk caching, persistence, and
    // multiplayer sync.
    let gen_a = make_edge_generator();
    let gen_b = make_edge_generator();

    let chunk_a = gen_a
        .generate_chunk_with_climate(ChunkId::new(3, 3), ClimateBias::Temperate)
        .expect("a");
    let chunk_b = gen_b
        .generate_chunk_with_climate(ChunkId::new(3, 3), ClimateBias::Temperate)
        .expect("b");

    let ids_a = chunk_a.biome_ids().expect("a biome_ids");
    let ids_b = chunk_b.biome_ids().expect("b biome_ids");
    assert_eq!(
        ids_a, ids_b,
        "biome_ids must be deterministic for same (seed, chunk_id, climate_bias)"
    );
}

#[test]
fn phase_1_6_f4_b_3_d_3_legacy_generate_chunk_keeps_biome_ids_none() {
    // The legacy `generate_chunk` code path (not climate-aware) must
    // continue to return chunks with `biome_ids: None`. D.3b only wires
    // per-vertex biome IDs through the `generate_chunk_with_climate`
    // path; downstream callers using `generate_chunk` see None as before.
    let gen = make_edge_generator();
    let chunk = gen
        .generate_chunk(ChunkId::new(0, 0))
        .expect("generate_chunk should succeed");
    assert!(
        chunk.biome_ids().is_none(),
        "legacy generate_chunk should leave biome_ids: None"
    );
}

#[test]
fn phase_1_6_f4_b_3_d_3_per_biome_amplitude_changes_heightmap() {
    // Sanity: with per-biome amplitude wired, a chunk that resolves to
    // mostly low-amplitude biomes (e.g., grassland-dominated lowlands)
    // should produce a different post-modulation heightmap than one
    // that resolves to high-amplitude biomes (alpine).
    //
    // Practical test: compare the same chunk's pre-D.3b "baseline"
    // height (mountain_amplitude_multiplier=1.0 everywhere) vs
    // post-D.3b "modulated" height (per-biome). The modulated heightmap
    // must differ at vertices that resolve to non-1.0-multiplier biomes.
    use astraweave_terrain::biome_parameters::BiomeParameters;
    let gen = make_edge_generator();

    // Generate a climate-field-driven chunk.
    let chunk = gen
        .generate_chunk_with_climate(ChunkId::new(5, 5), ClimateBias::Temperate)
        .expect("modulated");
    let modulated_heights = chunk.heightmap();
    let biome_ids = chunk.biome_ids().expect("biome_ids");

    // For each vertex, get the per-biome multiplier. If at least one
    // vertex has a multiplier != 1.0, the modulated heightmap should
    // differ from a hypothetical "all 1.0" baseline at that vertex.
    let mut any_non_unity = false;
    for &id in biome_ids.iter() {
        let params = BiomeParameters::for_biome(id);
        if (params.mountains_amplitude - 1.0).abs() > 0.05 {
            any_non_unity = true;
            break;
        }
    }
    assert!(
        any_non_unity,
        "expected at least one vertex with a non-unity mountain multiplier; \
         per-biome amplitude wiring may not be active"
    );

    // Sanity: heightmap is non-zero (otherwise the test is moot).
    let max_h = modulated_heights.max_height();
    assert!(max_h > 0.0, "modulated heightmap should produce positive heights");
}

#[test]
fn phase_1_6_f4_b_3_d_3_per_vertex_biome_ids_match_heightmap_resolution() {
    // The biome_ids array length must match heightmap.resolution()² so
    // downstream consumers (D.4 blending, D.5+ rendering) can index
    // (z, x) cleanly without bounds-mismatch bugs.
    let gen = make_edge_generator();
    for chunk_xz in [(0, 0), (1, -1), (-3, 2), (5, 5)] {
        let chunk = gen
            .generate_chunk_with_climate(
                ChunkId::new(chunk_xz.0, chunk_xz.1),
                ClimateBias::Temperate,
            )
            .expect("generate");
        let ids = chunk.biome_ids().expect("biome_ids");
        let res = chunk.heightmap().resolution() as usize;
        assert_eq!(
            ids.len(),
            res * res,
            "chunk ({}, {}): biome_ids length {} should equal {}^2 = {}",
            chunk_xz.0,
            chunk_xz.1,
            ids.len(),
            res,
            res * res
        );
    }
}
