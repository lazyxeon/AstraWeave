//! Heightmap generation and manipulation

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Configuration for heightmap generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightmapConfig {
    /// Resolution (width and height in vertices)
    pub resolution: u32,
    /// Minimum height value
    pub min_height: f32,
    /// Maximum height value
    pub max_height: f32,
    /// Scale factor for height values
    pub height_scale: f32,
}

impl Default for HeightmapConfig {
    fn default() -> Self {
        Self {
            resolution: 128,
            min_height: 0.0,
            max_height: 100.0,
            height_scale: 1.0,
        }
    }
}

/// A 2D heightmap storing terrain elevation data
#[derive(Debug, Clone)]
pub struct Heightmap {
    data: Vec<f32>,
    resolution: u32,
    min_height: f32,
    max_height: f32,
}

impl Heightmap {
    /// Create a new heightmap with the given configuration
    pub fn new(config: HeightmapConfig) -> anyhow::Result<Self> {
        let size = (config.resolution * config.resolution) as usize;
        Ok(Self {
            data: vec![0.0; size],
            resolution: config.resolution,
            min_height: 0.0, // Start with actual data range
            max_height: 0.0,
        })
    }

    /// Create a heightmap from existing data
    pub fn from_data(data: Vec<f32>, resolution: u32) -> anyhow::Result<Self> {
        if data.len() != (resolution * resolution) as usize {
            return Err(anyhow::anyhow!(
                "Data size {} doesn't match resolution {}x{}",
                data.len(),
                resolution,
                resolution
            ));
        }

        let min_height = data.iter().copied().fold(f32::INFINITY, f32::min);
        let max_height = data.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        Ok(Self {
            data,
            resolution,
            min_height,
            max_height,
        })
    }

    /// Get the resolution of the heightmap
    pub fn resolution(&self) -> u32 {
        self.resolution
    }

    /// Get the minimum height value
    pub fn min_height(&self) -> f32 {
        self.min_height
    }

    /// Get the maximum height value
    pub fn max_height(&self) -> f32 {
        self.max_height
    }

    /// Get the raw height data
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Get mutable access to raw height data for bulk operations
    ///
    /// Note: After modifying data through this method, call `recalculate_bounds()`
    /// to update min/max height values.
    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Recalculate min/max height bounds after bulk data modification
    pub fn recalculate_bounds(&mut self) {
        if self.data.is_empty() {
            self.min_height = 0.0;
            self.max_height = 0.0;
            return;
        }

        self.min_height = f32::MAX;
        self.max_height = f32::MIN;

        for &h in &self.data {
            self.min_height = self.min_height.min(h);
            self.max_height = self.max_height.max(h);
        }
    }

    /// Get height at a specific grid coordinate
    pub fn get_height(&self, x: u32, z: u32) -> f32 {
        if x >= self.resolution || z >= self.resolution {
            return 0.0;
        }
        let index = (z * self.resolution + x) as usize;
        self.data[index]
    }

    /// Get height at a specific index
    pub fn get_height_at_index(&self, index: usize) -> f32 {
        self.data.get(index).copied().unwrap_or(0.0)
    }

    /// Set height at a specific grid coordinate
    pub fn set_height(&mut self, x: u32, z: u32, height: f32) {
        if x >= self.resolution || z >= self.resolution {
            return;
        }
        let index = (z * self.resolution + x) as usize;
        self.data[index] = height;

        // Update min/max
        self.min_height = self.min_height.min(height);
        self.max_height = self.max_height.max(height);
    }

    /// Sample the heightmap with bilinear interpolation at fractional coordinates
    pub fn sample_bilinear(&self, u: f32, v: f32) -> f32 {
        let x = u.clamp(0.0, self.resolution as f32 - 1.001);
        let z = v.clamp(0.0, self.resolution as f32 - 1.001);

        let x0 = x.floor() as u32;
        let z0 = z.floor() as u32;
        let x1 = (x0 + 1).min(self.resolution - 1);
        let z1 = (z0 + 1).min(self.resolution - 1);

        let fx = x.fract();
        let fz = z.fract();

        let h00 = self.get_height(x0, z0);
        let h10 = self.get_height(x1, z0);
        let h01 = self.get_height(x0, z1);
        let h11 = self.get_height(x1, z1);

        let h0 = h00 * (1.0 - fx) + h10 * fx;
        let h1 = h01 * (1.0 - fx) + h11 * fx;

        h0 * (1.0 - fz) + h1 * fz
    }

    /// Calculate the normal vector at a given grid coordinate
    pub fn calculate_normal(&self, x: u32, z: u32, scale: f32) -> Vec3 {
        let left = if x > 0 {
            self.get_height(x - 1, z)
        } else {
            self.get_height(x, z)
        };
        let right = if x < self.resolution - 1 {
            self.get_height(x + 1, z)
        } else {
            self.get_height(x, z)
        };
        let up = if z > 0 {
            self.get_height(x, z - 1)
        } else {
            self.get_height(x, z)
        };
        let down = if z < self.resolution - 1 {
            self.get_height(x, z + 1)
        } else {
            self.get_height(x, z)
        };

        let dx = (right - left) / (2.0 * scale);
        let dz = (down - up) / (2.0 * scale);

        Vec3::new(-dx, 1.0, -dz).normalize()
    }

    /// Apply simple hydraulic erosion to the heightmap
    pub fn apply_hydraulic_erosion(&mut self, strength: f32) -> anyhow::Result<()> {
        let iterations = 10;
        let dt = 1.2;
        let _density = 1.0;
        let evaporation = 0.05;
        let deposition = 0.3;
        let min_slope = 0.05;

        for _ in 0..iterations {
            let mut water = vec![0.0f32; self.data.len()];
            let mut velocity_x = vec![0.0f32; self.data.len()];
            let mut velocity_z = vec![0.0f32; self.data.len()];

            // Add water (rain)
            for w in &mut water {
                *w += strength * 0.01;
            }

            // Flow simulation
            for z in 1..(self.resolution - 1) {
                for x in 1..(self.resolution - 1) {
                    let idx = (z * self.resolution + x) as usize;

                    let height = self.data[idx];
                    let water_height = water[idx];
                    let total_height = height + water_height;

                    // Calculate height differences to neighbors
                    let left_height = self.data[idx - 1] + water[idx - 1];
                    let right_height = self.data[idx + 1] + water[idx + 1];
                    let up_height = self.data[idx - self.resolution as usize]
                        + water[idx - self.resolution as usize];
                    let down_height = self.data[idx + self.resolution as usize]
                        + water[idx + self.resolution as usize];

                    // Calculate velocity
                    velocity_x[idx] +=
                        dt * (left_height - total_height + right_height - total_height) / 2.0;
                    velocity_z[idx] +=
                        dt * (up_height - total_height + down_height - total_height) / 2.0;

                    // Apply velocity damping
                    velocity_x[idx] *= 0.99;
                    velocity_z[idx] *= 0.99;
                }
            }

            // Update water levels and apply erosion/deposition
            for z in 1..(self.resolution - 1) {
                for x in 1..(self.resolution - 1) {
                    let idx = (z * self.resolution + x) as usize;

                    let speed = (velocity_x[idx] * velocity_x[idx]
                        + velocity_z[idx] * velocity_z[idx])
                        .sqrt();

                    if speed > min_slope {
                        // Erosion
                        let erosion_amount = speed * deposition * strength * 0.1;
                        self.data[idx] -= erosion_amount;
                    } else {
                        // Deposition
                        let deposition_amount = speed * deposition * strength * 0.05;
                        self.data[idx] += deposition_amount;
                    }

                    // Evaporate water
                    water[idx] *= 1.0 - evaporation;
                }
            }
        }

        // Recalculate min/max heights
        self.min_height = self.data.iter().copied().fold(f32::INFINITY, f32::min);
        self.max_height = self.data.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        Ok(())
    }

    /// Generate vertex positions for rendering
    pub fn generate_vertices(&self, chunk_size: f32, offset: Vec3) -> Vec<Vec3> {
        let mut vertices = Vec::with_capacity((self.resolution * self.resolution) as usize);
        let step = chunk_size / (self.resolution - 1) as f32;

        for z in 0..self.resolution {
            for x in 0..self.resolution {
                let world_x = offset.x + x as f32 * step;
                let world_z = offset.z + z as f32 * step;
                let height = self.get_height(x, z);

                vertices.push(Vec3::new(world_x, height, world_z));
            }
        }

        vertices
    }

    /// Generate triangle indices for rendering.
    ///
    /// Winding order is counter-clockwise when viewed from above (+Y),
    /// matching wgpu's default `FrontFace::Ccw` with `cull_mode: Back`.
    pub fn generate_indices(&self) -> Vec<u32> {
        let mut indices = Vec::new();

        for z in 0..(self.resolution - 1) {
            for x in 0..(self.resolution - 1) {
                let base = z * self.resolution + x;

                // First triangle (CCW from above: base → base+res → base+1)
                indices.push(base);
                indices.push(base + self.resolution);
                indices.push(base + 1);

                // Second triangle (CCW from above: base+1 → base+res → base+res+1)
                indices.push(base + 1);
                indices.push(base + self.resolution);
                indices.push(base + self.resolution + 1);
            }
        }

        indices
    }

    /// Apply a smoothing filter to the heightmap
    pub fn smooth(&mut self, iterations: u32) {
        for _ in 0..iterations {
            let mut new_data = self.data.clone();

            for z in 1..(self.resolution - 1) {
                for x in 1..(self.resolution - 1) {
                    let idx = (z * self.resolution + x) as usize;

                    let sum = self.data[idx - 1]
                        + self.data[idx + 1]
                        + self.data[idx - self.resolution as usize]
                        + self.data[idx + self.resolution as usize]
                        + self.data[idx] * 4.0;

                    new_data[idx] = sum / 8.0;
                }
            }

            self.data = new_data;
        }

        // Recalculate min/max heights
        self.min_height = self.data.iter().copied().fold(f32::INFINITY, f32::min);
        self.max_height = self.data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heightmap_creation() {
        let config = HeightmapConfig::default();
        let heightmap = Heightmap::new(config).unwrap();

        assert_eq!(heightmap.resolution(), 128);
        assert_eq!(heightmap.data().len(), 128 * 128);
    }

    #[test]
    fn test_heightmap_get_set() {
        let config = HeightmapConfig::default();
        let mut heightmap = Heightmap::new(config).unwrap();

        heightmap.set_height(0, 0, 10.0);
        assert_eq!(heightmap.get_height(0, 0), 10.0);
        assert_eq!(heightmap.max_height(), 10.0);
    }

    #[test]
    fn test_bilinear_sampling() {
        let config = HeightmapConfig {
            resolution: 3,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(config).unwrap();

        heightmap.set_height(0, 0, 0.0);
        heightmap.set_height(1, 0, 10.0);
        heightmap.set_height(0, 1, 0.0);
        heightmap.set_height(1, 1, 10.0);

        let interpolated = heightmap.sample_bilinear(0.5, 0.5);
        assert_eq!(interpolated, 5.0);
    }

    #[test]
    fn test_normal_calculation() {
        let config = HeightmapConfig {
            resolution: 3,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(config).unwrap();

        heightmap.set_height(1, 1, 10.0);
        let normal = heightmap.calculate_normal(1, 1, 1.0);

        // Should point upward since surrounding heights are 0
        assert!(normal.y > 0.0);
    }

    #[test]
    fn test_vertex_generation() {
        let config = HeightmapConfig {
            resolution: 3,
            ..Default::default()
        };
        let heightmap = Heightmap::new(config).unwrap();

        let vertices = heightmap.generate_vertices(256.0, Vec3::ZERO);
        assert_eq!(vertices.len(), 9); // 3x3 grid
    }

    #[test]
    fn test_index_generation() {
        let config = HeightmapConfig {
            resolution: 3,
            ..Default::default()
        };
        let heightmap = Heightmap::new(config).unwrap();

        let indices = heightmap.generate_indices();
        assert_eq!(indices.len(), 24); // 4 quads * 2 triangles * 3 vertices
    }
}
