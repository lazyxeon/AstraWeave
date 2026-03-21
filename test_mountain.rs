use astraweave_terrain::*;
fn main() {
    let mut config = WorldConfig::default();
    config.seed = 42;
    
    // Apply mountain preset manually (same as editor)
    config.noise.base_elevation.scale = 0.003;
    config.noise.base_elevation.amplitude = 55.0;
    config.noise.base_elevation.octaves = 6;
    config.noise.base_elevation.persistence = 0.55;
    config.noise.base_elevation.lacunarity = 2.2;
    config.noise.mountains.enabled = true;
    config.noise.mountains.scale = 0.002;
    config.noise.mountains.amplitude = 210.0;
    config.noise.mountains.octaves = 8;
    config.noise.detail.enabled = true;
    config.noise.detail.scale = 0.03;
    config.noise.detail.amplitude = 8.0;
    config.noise.erosion_enabled = false;
    
    // Set biomes to mountain-compatible
    config.biomes = vec![
        BiomeConfig::mountain(),
        BiomeConfig::tundra(),
        BiomeConfig::forest(),
        BiomeConfig::grassland(),
    ];
    
    let gen = WorldGenerator::new(config);
    
    match gen.generate_chunk(ChunkId::new(0, 0)) {
        Ok(chunk) => {
            let hm = chunk.heightmap();
            let data = hm.data();
            let min = data.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let avg: f32 = data.iter().sum::<f32>() / data.len() as f32;
            println!("MOUNTAIN CHUNK OK: {} points, min={:.1}, max={:.1}, avg={:.1}", data.len(), min, max, avg);
            let bm = chunk.biome_map();
            let mountain_count = bm.iter().filter(|b| **b == BiomeType::Mountain).count();
            println!("Biome map: {} total, {} mountain", bm.len(), mountain_count);
        }
        Err(e) => {
            println!("MOUNTAIN CHUNK FAILED: {}", e);
        }
    }
}
