//! Phase 1.6-F.3-phase-1.A: verifies `generate_chunk_with_climate` populates
//! pre-erosion biome_weights on the chunk. For phase 1 (simple CA erosion)
//! the distinction between pre- and post-erosion heights is imperceptible,
//! so this test's main value is plumbing verification — it confirms the new
//! field is populated, has the correct length, and contains sum-to-one
//! weight vectors. Phase 2's real erosion will make the
//! pre-vs-post-erosion invariant meaningful.

use astraweave_terrain::{ChunkId, ClimateBias, WorldConfig, WorldGenerator};

fn make_generator() -> WorldGenerator {
    let mut config = WorldConfig::default();
    config.seed = 12345;
    WorldGenerator::new(config)
}

#[test]
fn biome_weights_populated_after_generate_chunk_with_climate() {
    let gen = make_generator();
    let chunk = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("chunk generation should succeed");

    let heights = chunk.heightmap();
    let weights = chunk
        .biome_weights()
        .expect("biome_weights should be populated after F.3-phase-1.A");

    let expected_count = (heights.resolution() as usize) * (heights.resolution() as usize);
    assert_eq!(
        weights.len(),
        expected_count,
        "biome_weights length should match heightmap vertex count"
    );
}

#[test]
fn biome_weights_are_normalized() {
    let gen = make_generator();
    let chunk = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("chunk generation should succeed");
    let weights = chunk
        .biome_weights()
        .expect("biome_weights should be populated");

    // Every vertex's weight vector should sum to ~1.0 (normalized biome distribution).
    for (i, w) in weights.iter().enumerate() {
        let sum: f32 = w.iter().sum();
        assert!(
            (sum - 1.0).abs() < 0.01,
            "biome_weights[{i}] sum is {sum:.3}, expected ~1.0; weights = {w:?}"
        );
    }
}

#[test]
fn biome_weights_reflect_elevation_bands() {
    // For a Temperate chunk with meaningful elevation variation, low-Y vertices
    // should have more Beach/Grassland weight than Mountain weight, and
    // high-Y vertices should have more Forest/Mountain weight than Beach.
    //
    // Slot layout from `elevation_biome.rs`:
    //   [0] Grassland, [1] Desert, [2] Forest, [3] Mountain,
    //   [4] Tundra, [5] Swamp, [6] Beach, [7] River
    let gen = make_generator();
    let chunk = gen
        .generate_chunk_with_climate(ChunkId::new(0, 0), ClimateBias::Temperate)
        .expect("chunk generation should succeed");
    let heights = chunk.heightmap();
    let weights = chunk
        .biome_weights()
        .expect("biome_weights should be populated");

    let dim = heights.resolution() as usize;
    let (mut lo_idx, mut hi_idx) = (0usize, 0usize);
    let (mut lo_y, mut hi_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for z in 0..dim {
        for x in 0..dim {
            let idx = z * dim + x;
            let y = heights.get_height(x as u32, z as u32);
            if y < lo_y {
                lo_y = y;
                lo_idx = idx;
            }
            if y > hi_y {
                hi_y = y;
                hi_idx = idx;
            }
        }
    }

    if (hi_y - lo_y) < 10.0 {
        println!(
            "chunk variance too low ({lo_y} to {hi_y}) for band differentiation — skipping"
        );
        return;
    }

    let lo_w = weights[lo_idx];
    let hi_w = weights[hi_idx];

    // Low-elevation vertex: Beach + Grassland dominance over Mountain.
    assert!(
        lo_w[6] + lo_w[0] > lo_w[3],
        "low-elevation vertex has mountain-dominant weights: {lo_w:?} at y={lo_y}"
    );
    // High-elevation vertex: Forest + Mountain dominance over Beach.
    assert!(
        hi_w[2] + hi_w[3] > hi_w[6],
        "high-elevation vertex has beach-dominant weights: {hi_w:?} at y={hi_y}"
    );
}

#[test]
fn legacy_generate_chunk_leaves_biome_weights_none() {
    // The non-climate generate_chunk method must not populate biome_weights
    // (preserves existing behavior for non-editor callers).
    let gen = make_generator();
    let chunk = gen
        .generate_chunk(ChunkId::new(0, 0))
        .expect("chunk generation should succeed");
    assert!(
        chunk.biome_weights().is_none(),
        "legacy generate_chunk should leave biome_weights as None"
    );
}
