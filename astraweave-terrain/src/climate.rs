//! Climate map generation for biome assignment

use crate::ChunkId;
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize};

/// Configuration for climate generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateConfig {
    /// Temperature noise settings
    pub temperature: ClimateLayer,
    /// Moisture noise settings
    pub moisture: ClimateLayer,
    /// Height influence on temperature (degrees per meter)
    pub temperature_height_gradient: f32,
    /// Latitude influence on temperature
    pub temperature_latitude_gradient: f32,
    /// Distance from water influence on moisture
    pub moisture_distance_falloff: f32,
}

impl Default for ClimateConfig {
    fn default() -> Self {
        Self {
            temperature: ClimateLayer {
                scale: 0.001,
                octaves: 3,
                persistence: 0.5,
                lacunarity: 2.0,
                amplitude: 1.0,
                offset: 0.5,
            },
            moisture: ClimateLayer {
                scale: 0.0015,
                octaves: 4,
                persistence: 0.6,
                lacunarity: 2.1,
                amplitude: 1.0,
                offset: 0.5,
            },
            temperature_height_gradient: -0.0065, // Standard atmospheric lapse rate
            temperature_latitude_gradient: 0.8,   // Stronger temperature variation by latitude
            moisture_distance_falloff: 0.001,     // Moisture decreases inland
        }
    }
}

/// Configuration for a single climate layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateLayer {
    /// Noise scale (frequency)
    pub scale: f64,
    /// Number of noise octaves
    pub octaves: usize,
    /// Persistence (amplitude falloff between octaves)
    pub persistence: f64,
    /// Lacunarity (frequency multiplier between octaves)
    pub lacunarity: f64,
    /// Overall amplitude
    pub amplitude: f32,
    /// Base offset value
    pub offset: f32,
}

/// Climate map generator that provides temperature and moisture data
#[derive(Debug)]
pub struct ClimateMap {
    temperature_noise: Perlin,
    moisture_noise: Perlin,
    config: ClimateConfig,
}

impl ClimateMap {
    /// Create a new climate map generator
    pub fn new(config: &ClimateConfig, seed: u64) -> Self {
        Self {
            temperature_noise: Perlin::new(seed as u32),
            moisture_noise: Perlin::new((seed + 1000) as u32),
            config: config.clone(),
        }
    }

    /// Sample temperature at a world position
    pub fn sample_temperature(&self, x: f64, z: f64, height: f32) -> f32 {
        // Base temperature from noise
        let mut temperature =
            self.sample_noise_fbm(&self.temperature_noise, &self.config.temperature, x, z);

        // Apply height gradient (cooler at higher elevations)
        temperature += height * self.config.temperature_height_gradient;

        // Apply latitude gradient (cooler towards poles)
        let latitude_factor = (z * 0.00001).sin(); // Very rough latitude simulation
        temperature += latitude_factor as f32 * self.config.temperature_latitude_gradient;

        // Clamp to reasonable range
        temperature.clamp(0.0, 1.0)
    }

    /// Sample moisture at a world position
    pub fn sample_moisture(&self, x: f64, z: f64, height: f32) -> f32 {
        // Base moisture from noise
        let mut moisture = self.sample_noise_fbm(&self.moisture_noise, &self.config.moisture, x, z);

        // Reduce moisture at higher elevations (rain shadow effect)
        let height_factor = (height * 0.01).clamp(0.0, 1.0);
        moisture *= 1.0 - height_factor * 0.3;

        // Distance from water effect (simplified - in real implementation would use actual water bodies)
        let water_distance = self.estimate_water_distance(x, z);
        let water_factor = (-water_distance * self.config.moisture_distance_falloff).exp();
        moisture = moisture * 0.7 + water_factor * 0.3;

        // Clamp to valid range
        moisture.clamp(0.0, 1.0)
    }

    /// Sample both temperature and moisture at a world position
    pub fn sample_climate(&self, x: f64, z: f64, height: f32) -> (f32, f32) {
        let temperature = self.sample_temperature(x, z, height);
        let moisture = self.sample_moisture(x, z, height);
        (temperature, moisture)
    }

    /// Sample climate data for an entire chunk
    pub fn sample_chunk(
        &self,
        chunk_id: ChunkId,
        chunk_size: f32,
        resolution: u32,
    ) -> anyhow::Result<Vec<(f32, f32)>> {
        let world_origin = chunk_id.to_world_pos(chunk_size);
        let step = chunk_size / (resolution - 1) as f32;
        let mut climate_data = Vec::with_capacity((resolution * resolution) as usize);

        for z in 0..resolution {
            for x in 0..resolution {
                let world_x = world_origin.x + x as f32 * step;
                let world_z = world_origin.z + z as f32 * step;

                // We need height data to calculate climate properly
                // For now, use a simple height estimation based on position
                let estimated_height = self.estimate_height(world_x as f64, world_z as f64);

                let climate = self.sample_climate(world_x as f64, world_z as f64, estimated_height);
                climate_data.push(climate);
            }
        }

        Ok(climate_data)
    }

    /// Sample fractal Brownian motion noise
    fn sample_noise_fbm(&self, noise: &Perlin, layer: &ClimateLayer, x: f64, z: f64) -> f32 {
        let mut value = 0.0;
        let mut amplitude = layer.amplitude;
        let mut frequency = layer.scale;

        for _ in 0..layer.octaves {
            value += noise.get([x * frequency, 0.0, z * frequency]) as f32 * amplitude;
            amplitude *= layer.persistence as f32;
            frequency *= layer.lacunarity;
        }

        value + layer.offset
    }

    /// Estimate height at a position (temporary until we have proper integration)
    pub fn estimate_height(&self, x: f64, z: f64) -> f32 {
        // Simple height estimation using noise — amplitude must roughly
        // match the maximum terrain amplitude used by noise presets (e.g.
        // mountain base_amplitude 120 + mountains_amplitude 150 ≈ ~270).
        let height_noise = self.sample_noise_fbm(
            &self.temperature_noise, // Reuse temperature noise for height
            &ClimateLayer {
                scale: 0.002,
                octaves: 4,
                persistence: 0.5,
                lacunarity: 2.0,
                amplitude: 150.0,
                offset: 10.0,
            },
            x,
            z,
        );
        height_noise.max(0.0)
    }

    /// Estimate distance to nearest water body (simplified)
    fn estimate_water_distance(&self, x: f64, z: f64) -> f32 {
        // Simplified water distance using noise to create "rivers" and "lakes"
        let water_noise = self.sample_noise_fbm(
            &self.moisture_noise,
            &ClimateLayer {
                scale: 0.003,
                octaves: 2,
                persistence: 0.4,
                lacunarity: 2.5,
                amplitude: 1.0,
                offset: 0.0,
            },
            x,
            z,
        );

        // If noise is below threshold, we're "near water"
        if water_noise.abs() < 0.1 {
            0.0 // At water
        } else {
            (water_noise.abs() - 0.1) * 1000.0 // Distance in arbitrary units
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &ClimateConfig {
        &self.config
    }
}

/// Utility functions for climate analysis
pub mod utils {
    use super::*;
    use crate::BiomeType;

    /// Classify biome based on temperature and moisture (Whittaker biome classification)
    pub fn classify_whittaker_biome(temperature: f32, moisture: f32) -> BiomeType {
        match (temperature, moisture) {
            (t, _m) if t < 0.2 => BiomeType::Tundra,
            (t, m) if t < 0.4 && m < 0.3 => BiomeType::Tundra,
            (t, m) if t < 0.6 && m < 0.2 => BiomeType::Desert,
            (t, m) if t > 0.7 && m < 0.4 => BiomeType::Desert,
            (_t, m) if m > 0.8 => BiomeType::Swamp,
            (t, m) if t > 0.6 && m > 0.6 => BiomeType::Forest,
            (t, m) if t > 0.4 && m > 0.4 => BiomeType::Forest,
            _ => BiomeType::Grassland,
        }
    }

    /// Generate a climate preview for visualization
    pub fn generate_climate_preview(
        climate: &ClimateMap,
        size: u32,
        scale: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let mut temperatures = Vec::with_capacity((size * size) as usize);
        let mut moistures = Vec::with_capacity((size * size) as usize);
        let step = scale / size as f32;

        for z in 0..size {
            for x in 0..size {
                let world_x = x as f32 * step;
                let world_z = z as f32 * step;
                let height = climate.estimate_height(world_x as f64, world_z as f64);

                let (temperature, moisture) =
                    climate.sample_climate(world_x as f64, world_z as f64, height);

                temperatures.push(temperature);
                moistures.push(moisture);
            }
        }

        (temperatures, moistures)
    }

    /// Create a biome classification map
    pub fn generate_biome_classification_map(
        climate: &ClimateMap,
        size: u32,
        scale: f32,
    ) -> Vec<BiomeType> {
        let mut biomes = Vec::with_capacity((size * size) as usize);
        let step = scale / size as f32;

        for z in 0..size {
            for x in 0..size {
                let world_x = x as f32 * step;
                let world_z = z as f32 * step;
                let height = climate.estimate_height(world_x as f64, world_z as f64);

                let (temperature, moisture) =
                    climate.sample_climate(world_x as f64, world_z as f64, height);

                let biome = classify_whittaker_biome(temperature, moisture);
                biomes.push(biome);
            }
        }

        biomes
    }

    /// Calculate climate statistics for a region
    pub fn calculate_climate_stats(
        climate: &ClimateMap,
        min_x: f64,
        max_x: f64,
        min_z: f64,
        max_z: f64,
        samples: u32,
    ) -> ClimateStats {
        let mut temperatures = Vec::new();
        let mut moistures = Vec::new();

        let step_x = (max_x - min_x) / samples as f64;
        let step_z = (max_z - min_z) / samples as f64;

        for i in 0..samples {
            for j in 0..samples {
                let x = min_x + i as f64 * step_x;
                let z = min_z + j as f64 * step_z;
                let height = climate.estimate_height(x, z);

                let (temperature, moisture) = climate.sample_climate(x, z, height);
                temperatures.push(temperature);
                moistures.push(moisture);
            }
        }

        ClimateStats {
            temperature_min: temperatures.iter().copied().fold(f32::INFINITY, f32::min),
            temperature_max: temperatures
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max),
            temperature_avg: temperatures.iter().sum::<f32>() / temperatures.len() as f32,
            moisture_min: moistures.iter().copied().fold(f32::INFINITY, f32::min),
            moisture_max: moistures.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            moisture_avg: moistures.iter().sum::<f32>() / moistures.len() as f32,
        }
    }

    /// Climate statistics for a region
    #[derive(Debug, Clone)]
    pub struct ClimateStats {
        pub temperature_min: f32,
        pub temperature_max: f32,
        pub temperature_avg: f32,
        pub moisture_min: f32,
        pub moisture_max: f32,
        pub moisture_avg: f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BiomeType;

    #[test]
    fn test_climate_config_default() {
        let config = ClimateConfig::default();
        assert!(config.temperature.scale > 0.0);
        assert!(config.moisture.scale > 0.0);
    }

    #[test]
    fn test_climate_map_creation() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let (temperature, moisture) = climate.sample_climate(100.0, 100.0, 10.0);
        assert!((0.0..=1.0).contains(&temperature));
        assert!((0.0..=1.0).contains(&moisture));
    }

    #[test]
    fn test_height_gradient() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let temp_low = climate.sample_temperature(100.0, 100.0, 0.0);
        let temp_high = climate.sample_temperature(100.0, 100.0, 100.0);

        // Higher elevation should be cooler
        assert!(temp_high < temp_low);
    }

    #[test]
    fn test_chunk_sampling() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let chunk_id = ChunkId::new(0, 0);
        let climate_data = climate.sample_chunk(chunk_id, 256.0, 32).unwrap();

        assert_eq!(climate_data.len(), 32 * 32);
        for (temp, moisture) in climate_data {
            assert!((0.0..=1.0).contains(&temp));
            assert!((0.0..=1.0).contains(&moisture));
        }
    }

    #[test]
    fn test_deterministic_climate() {
        let config = ClimateConfig::default();
        let climate1 = ClimateMap::new(&config, 12345);
        let climate2 = ClimateMap::new(&config, 12345);

        let (temp1, moisture1) = climate1.sample_climate(100.0, 100.0, 10.0);
        let (temp2, moisture2) = climate2.sample_climate(100.0, 100.0, 10.0);

        assert_eq!(temp1, temp2);
        assert_eq!(moisture1, moisture2);
    }

    #[test]
    fn test_whittaker_classification() {
        assert_eq!(utils::classify_whittaker_biome(0.1, 0.5), BiomeType::Tundra);
        assert_eq!(utils::classify_whittaker_biome(0.8, 0.1), BiomeType::Desert);
        assert_eq!(utils::classify_whittaker_biome(0.7, 0.9), BiomeType::Swamp);
        assert_eq!(utils::classify_whittaker_biome(0.7, 0.7), BiomeType::Forest);
        assert_eq!(utils::classify_whittaker_biome(0.5, 0.5), BiomeType::Forest);
    }

    #[test]
    fn test_climate_preview() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let (temperatures, moistures) = utils::generate_climate_preview(&climate, 16, 256.0);

        assert_eq!(temperatures.len(), 16 * 16);
        assert_eq!(moistures.len(), 16 * 16);
    }

    #[test]
    fn test_biome_classification_map() {
        let config = ClimateConfig::default();
        let climate = ClimateMap::new(&config, 12345);

        let biomes = utils::generate_biome_classification_map(&climate, 16, 256.0);

        assert_eq!(biomes.len(), 16 * 16);
        assert!(biomes.iter().all(|b| BiomeType::all().contains(b)));
    }
}
