#![allow(dead_code)]

use astraweave_terrain::{
    BiomeConfig, BiomeType, ChunkId, Heightmap, ScatterConfig, ScatterResult, TerrainChunk,
    VegetationInstance, VegetationScatter, WorldConfig, WorldGenerator,
};
use glam::Vec3;
use std::collections::HashMap;

/// Full noise preset for a biome — configures all three noise layers.
pub struct BiomeNoisePreset {
    // Base elevation (Perlin)
    pub base_scale: f64,
    pub base_amplitude: f32,
    pub base_octaves: usize,
    pub base_persistence: f64,
    pub base_lacunarity: f64,
    // Mountains (RidgedMulti)
    pub mountains_enabled: bool,
    pub mountains_scale: f64,
    pub mountains_amplitude: f32,
    pub mountains_octaves: usize,
    // Detail (Billow)
    pub detail_enabled: bool,
    pub detail_scale: f64,
    pub detail_amplitude: f32,
}

pub struct TerrainState {
    generator: Option<WorldGenerator>,
    config: WorldConfig,
    generated_chunks: HashMap<ChunkId, GeneratedChunk>,
    terrain_dirty: bool,
    last_seed: u64,
    last_biome: String,
}

pub struct GeneratedChunk {
    pub chunk: TerrainChunk,
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
    pub world_position: Vec3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub biome_id: u32,
    _padding: [u32; 3],
}

impl TerrainVertex {
    pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], biome_id: u32) -> Self {
        Self {
            position,
            normal,
            uv,
            biome_id,
            _padding: [0; 3],
        }
    }
}

impl Default for TerrainState {
    fn default() -> Self {
        Self {
            generator: None,
            config: WorldConfig::default(),
            generated_chunks: HashMap::new(),
            terrain_dirty: true,
            last_seed: 0,
            last_biome: String::new(),
        }
    }
}

impl TerrainState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configure(&mut self, seed: u64, primary_biome: &str) {
        if self.last_seed != seed || self.last_biome != primary_biome {
            self.terrain_dirty = true;
            self.last_seed = seed;
            self.last_biome = primary_biome.to_string();
        }

        self.config.seed = seed;
        self.config.biomes = Self::biomes_for_primary(primary_biome);
    }

    /// Update the noise generation parameters (octaves, lacunarity, persistence, amplitude).
    pub fn set_noise_params(
        &mut self,
        octaves: usize,
        lacunarity: f64,
        persistence: f64,
        amplitude: f32,
    ) {
        self.config.noise.base_elevation.octaves = octaves;
        self.config.noise.base_elevation.lacunarity = lacunarity;
        self.config.noise.base_elevation.persistence = persistence;
        self.config.noise.base_elevation.amplitude = amplitude;
        self.terrain_dirty = true;
    }

    /// Apply a full biome noise preset that configures all three noise layers.
    pub fn apply_biome_noise_preset(&mut self, preset: &BiomeNoisePreset) {
        // Base elevation
        self.config.noise.base_elevation.scale = preset.base_scale;
        self.config.noise.base_elevation.amplitude = preset.base_amplitude;
        self.config.noise.base_elevation.octaves = preset.base_octaves;
        self.config.noise.base_elevation.persistence = preset.base_persistence;
        self.config.noise.base_elevation.lacunarity = preset.base_lacunarity;

        // Mountains
        self.config.noise.mountains.enabled = preset.mountains_enabled;
        self.config.noise.mountains.scale = preset.mountains_scale;
        self.config.noise.mountains.amplitude = preset.mountains_amplitude;
        self.config.noise.mountains.octaves = preset.mountains_octaves;

        // Detail
        self.config.noise.detail.enabled = preset.detail_enabled;
        self.config.noise.detail.scale = preset.detail_scale;
        self.config.noise.detail.amplitude = preset.detail_amplitude;

        self.terrain_dirty = true;
    }

    fn biomes_for_primary(primary: &str) -> Vec<BiomeConfig> {
        let primary_type = primary.parse::<BiomeType>().unwrap_or(BiomeType::Grassland);

        let mut biomes = vec![Self::biome_config_for_type(primary_type)];

        for bt in BiomeType::all() {
            if *bt != primary_type {
                biomes.push(Self::biome_config_for_type(*bt));
            }
        }
        biomes
    }

    fn biome_config_for_type(bt: BiomeType) -> BiomeConfig {
        match bt {
            BiomeType::Grassland => BiomeConfig::grassland(),
            BiomeType::Desert => BiomeConfig::desert(),
            BiomeType::Forest => BiomeConfig::forest(),
            BiomeType::Mountain => BiomeConfig::mountain(),
            BiomeType::Tundra => BiomeConfig::tundra(),
            BiomeType::Swamp => BiomeConfig::swamp(),
            BiomeType::Beach => BiomeConfig::beach(),
            BiomeType::River => BiomeConfig::river(),
            _ => BiomeConfig::grassland(), // Fallback for future biome types
        }
    }

    pub fn generate_terrain(&mut self, chunk_radius: i32) -> anyhow::Result<usize> {
        self.generator = Some(WorldGenerator::new(self.config.clone()));
        self.generated_chunks.clear();

        let generator = self
            .generator
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Generator not initialized"))?;

        let chunk_size = self.config.chunk_size;
        let mut count = 0;

        for x in -chunk_radius..=chunk_radius {
            for z in -chunk_radius..=chunk_radius {
                let chunk_id = ChunkId { x, z };

                // Use generate_chunk (heightmap only) instead of
                // generate_chunk_with_scatter which runs extremely expensive
                // O(n²) Poisson disk sampling for vegetation/resources that
                // the editor doesn't render anyway.
                let chunk = generator.generate_chunk(chunk_id)?;

                let world_pos = chunk_id.to_world_pos(chunk_size);
                let world_offset = Vec3::new(world_pos.x, 0.0, world_pos.z);

                let (vertices, indices) = Self::generate_heightmap_mesh(
                    chunk.heightmap(),
                    chunk.biome_map(),
                    chunk_size,
                    world_offset,
                );

                self.generated_chunks.insert(
                    chunk_id,
                    GeneratedChunk {
                        chunk,
                        vertices,
                        indices,
                        world_position: world_offset,
                    },
                );

                count += 1;
            }
        }

        self.terrain_dirty = false;
        Ok(count)
    }

    fn generate_heightmap_mesh(
        heightmap: &Heightmap,
        biome_map: &[BiomeType],
        chunk_size: f32,
        world_offset: Vec3,
    ) -> (Vec<TerrainVertex>, Vec<u32>) {
        let resolution = heightmap.resolution() as usize;
        let cell_size = chunk_size / (resolution - 1) as f32;

        let mut vertices = Vec::with_capacity(resolution * resolution);
        let mut indices = Vec::with_capacity((resolution - 1) * (resolution - 1) * 6);

        for z in 0..resolution {
            for x in 0..resolution {
                let height = heightmap.get_height(x as u32, z as u32);

                let world_x = world_offset.x + x as f32 * cell_size;
                let world_z = world_offset.z + z as f32 * cell_size;

                let normal = Self::calculate_normal(heightmap, x, z, cell_size);

                let biome_idx = z * resolution + x;
                let biome_id = biome_map
                    .get(biome_idx)
                    .map(|b| Self::biome_to_id(*b))
                    .unwrap_or(0);

                vertices.push(TerrainVertex::new(
                    [world_x, height, world_z],
                    [normal.x, normal.y, normal.z],
                    [x as f32 / resolution as f32, z as f32 / resolution as f32],
                    biome_id,
                ));
            }
        }

        for z in 0..(resolution - 1) {
            for x in 0..(resolution - 1) {
                let top_left = (z * resolution + x) as u32;
                let top_right = top_left + 1;
                let bottom_left = ((z + 1) * resolution + x) as u32;
                let bottom_right = bottom_left + 1;

                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        (vertices, indices)
    }

    fn calculate_normal(heightmap: &Heightmap, x: usize, z: usize, cell_size: f32) -> Vec3 {
        let resolution = heightmap.resolution() as usize;

        let h_center = heightmap.get_height(x as u32, z as u32);
        let h_left = if x > 0 {
            heightmap.get_height((x - 1) as u32, z as u32)
        } else {
            h_center
        };
        let h_right = if x < resolution - 1 {
            heightmap.get_height((x + 1) as u32, z as u32)
        } else {
            h_center
        };
        let h_up = if z > 0 {
            heightmap.get_height(x as u32, (z - 1) as u32)
        } else {
            h_center
        };
        let h_down = if z < resolution - 1 {
            heightmap.get_height(x as u32, (z + 1) as u32)
        } else {
            h_center
        };

        let dx = (h_right - h_left) / (2.0 * cell_size);
        let dz = (h_down - h_up) / (2.0 * cell_size);

        Vec3::new(-dx, 1.0, -dz).normalize()
    }

    fn biome_to_id(biome: BiomeType) -> u32 {
        match biome {
            BiomeType::Grassland => 0,
            BiomeType::Desert => 1,
            BiomeType::Forest => 2,
            BiomeType::Mountain => 3,
            BiomeType::Tundra => 4,
            BiomeType::Swamp => 5,
            BiomeType::Beach => 6,
            BiomeType::River => 7,
            _ => 0, // Fallback for future biome types
        }
    }

    fn id_to_biome(id: u32) -> BiomeType {
        match id {
            0 => BiomeType::Grassland,
            1 => BiomeType::Desert,
            2 => BiomeType::Forest,
            3 => BiomeType::Mountain,
            4 => BiomeType::Tundra,
            5 => BiomeType::Swamp,
            6 => BiomeType::Beach,
            7 => BiomeType::River,
            _ => BiomeType::Grassland,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.terrain_dirty
    }

    pub fn chunk_count(&self) -> usize {
        self.generated_chunks.len()
    }

    pub fn get_all_vertices(&self) -> Vec<TerrainVertex> {
        let mut all_vertices = Vec::new();
        for gen_chunk in self.generated_chunks.values() {
            all_vertices.extend_from_slice(&gen_chunk.vertices);
        }
        all_vertices
    }

    pub fn get_all_indices(&self, vertex_offset: u32) -> Vec<u32> {
        let mut all_indices = Vec::new();
        let mut current_offset = vertex_offset;

        for gen_chunk in self.generated_chunks.values() {
            for &idx in &gen_chunk.indices {
                all_indices.push(idx + current_offset);
            }
            current_offset += gen_chunk.vertices.len() as u32;
        }
        all_indices
    }

    pub fn get_height_at(&self, world_x: f32, world_z: f32) -> Option<f32> {
        let chunk_size = self.config.chunk_size;
        let chunk_x = (world_x / chunk_size).floor() as i32;
        let chunk_z = (world_z / chunk_size).floor() as i32;
        let chunk_id = ChunkId {
            x: chunk_x,
            z: chunk_z,
        };

        if let Some(gen_chunk) = self.generated_chunks.get(&chunk_id) {
            let world_pos = Vec3::new(world_x, 0.0, world_z);
            gen_chunk
                .chunk
                .get_height_at_world_pos(world_pos, chunk_size)
        } else {
            None
        }
    }

    pub fn seed(&self) -> u64 {
        self.config.seed
    }

    pub fn primary_biome(&self) -> &str {
        if let Some(first) = self.config.biomes.first() {
            first.biome_type.as_str()
        } else {
            "grassland"
        }
    }

    pub fn chunks(&self) -> impl Iterator<Item = (&ChunkId, &GeneratedChunk)> {
        self.generated_chunks.iter()
    }

    pub fn get_gpu_chunks(&self) -> Vec<(Vec<TerrainVertex>, Vec<u32>)> {
        self.generated_chunks
            .values()
            .map(|chunk| (chunk.vertices.clone(), chunk.indices.clone()))
            .collect()
    }

    /// Get total vertex count across all chunks
    pub fn total_vertex_count(&self) -> usize {
        self.generated_chunks
            .values()
            .map(|c| c.vertices.len())
            .sum()
    }

    /// Get total index/triangle count across all chunks
    pub fn total_index_count(&self) -> usize {
        self.generated_chunks
            .values()
            .map(|c| c.indices.len())
            .sum()
    }

    /// Get total triangle count
    pub fn total_triangle_count(&self) -> usize {
        self.total_index_count() / 3
    }

    /// Check if terrain has been generated
    pub fn has_terrain(&self) -> bool {
        !self.generated_chunks.is_empty()
    }

    /// Get chunk IDs as a list
    pub fn chunk_ids(&self) -> Vec<ChunkId> {
        self.generated_chunks.keys().cloned().collect()
    }

    /// Apply a sculpting brush at the given world-space position.
    ///
    /// `brush_mode`: 0=Raise, 1=Smooth, 2=Flatten, 3=Lower, 4=Erode
    /// Returns true if any terrain was modified.
    pub fn apply_brush(
        &mut self,
        world_x: f32,
        world_z: f32,
        radius: f32,
        strength: f32,
        brush_mode: u32,
    ) -> bool {
        let chunk_size = self.config.chunk_size;
        let mut modified = false;

        // Collect chunk IDs that might be affected
        let chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().cloned().collect();

        for chunk_id in chunk_ids {
            let chunk_origin_x = chunk_id.x as f32 * chunk_size;
            let chunk_origin_z = chunk_id.z as f32 * chunk_size;

            // Quick AABB check: does the brush circle overlap this chunk?
            let closest_x = world_x.clamp(chunk_origin_x, chunk_origin_x + chunk_size);
            let closest_z = world_z.clamp(chunk_origin_z, chunk_origin_z + chunk_size);
            let dx = world_x - closest_x;
            let dz = world_z - closest_z;
            if dx * dx + dz * dz > radius * radius {
                continue;
            }

            if let Some(gen_chunk) = self.generated_chunks.get_mut(&chunk_id) {
                let resolution = gen_chunk.chunk.heightmap().resolution();
                let cell_size = chunk_size / (resolution - 1) as f32;
                let mut chunk_modified = false;

                // Gather heights for smooth/flatten operations
                let avg_height = if brush_mode == 1 || brush_mode == 2 {
                    // Average height under brush
                    let mut sum = 0.0f32;
                    let mut count = 0u32;
                    for gz in 0..resolution {
                        for gx in 0..resolution {
                            let px = chunk_origin_x + gx as f32 * cell_size;
                            let pz = chunk_origin_z + gz as f32 * cell_size;
                            let d = ((px - world_x).powi(2) + (pz - world_z).powi(2)).sqrt();
                            if d <= radius {
                                sum += gen_chunk.chunk.heightmap().get_height(gx, gz);
                                count += 1;
                            }
                        }
                    }
                    if count > 0 { sum / count as f32 } else { 0.0 }
                } else {
                    0.0
                };

                for gz in 0..resolution {
                    for gx in 0..resolution {
                        let px = chunk_origin_x + gx as f32 * cell_size;
                        let pz = chunk_origin_z + gz as f32 * cell_size;
                        let dist = ((px - world_x).powi(2) + (pz - world_z).powi(2)).sqrt();
                        if dist > radius {
                            continue;
                        }

                        // Smooth falloff
                        let falloff = 1.0 - (dist / radius);
                        let falloff = falloff * falloff; // quadratic
                        let current_h = gen_chunk.chunk.heightmap().get_height(gx, gz);

                        let new_h = match brush_mode {
                            0 => current_h + strength * falloff * 5.0, // Raise (Sculpt)
                            1 => {
                                // Smooth toward average
                                let blend = strength * falloff * 0.3;
                                current_h * (1.0 - blend) + avg_height * blend
                            }
                            2 => {
                                // Flatten toward average
                                let blend = strength * falloff;
                                current_h * (1.0 - blend) + avg_height * blend
                            }
                            3 => current_h - strength * falloff * 5.0, // Lower (Paint as lower)
                            4 => {
                                // Erode (lower + smoothing)
                                let erode = current_h - strength * falloff * 3.0;
                                erode * (1.0 - falloff * 0.1) + avg_height * falloff * 0.1
                            }
                            _ => current_h,
                        };

                        gen_chunk.chunk.heightmap_mut().set_height(gx, gz, new_h);
                        chunk_modified = true;
                    }
                }

                if chunk_modified {
                    // Regenerate mesh for this chunk
                    let world_offset = Vec3::new(chunk_origin_x, 0.0, chunk_origin_z);
                    let (vertices, indices) = Self::generate_heightmap_mesh(
                        gen_chunk.chunk.heightmap(),
                        gen_chunk.chunk.biome_map(),
                        chunk_size,
                        world_offset,
                    );
                    gen_chunk.vertices = vertices;
                    gen_chunk.indices = indices;
                    modified = true;
                }
            }
        }

        modified
    }

    /// Paint a biome material at the given world-space position.
    ///
    /// `biome_id`: 0-7 corresponding to the shader biome IDs.
    /// Returns true if any terrain was modified.
    pub fn apply_brush_paint(
        &mut self,
        world_x: f32,
        world_z: f32,
        radius: f32,
        biome_id: u32,
    ) -> bool {
        let chunk_size = self.config.chunk_size;
        let target_biome = Self::id_to_biome(biome_id);
        let mut modified = false;

        let chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().cloned().collect();

        for chunk_id in chunk_ids {
            let chunk_origin_x = chunk_id.x as f32 * chunk_size;
            let chunk_origin_z = chunk_id.z as f32 * chunk_size;

            // Quick AABB check
            let closest_x = world_x.clamp(chunk_origin_x, chunk_origin_x + chunk_size);
            let closest_z = world_z.clamp(chunk_origin_z, chunk_origin_z + chunk_size);
            let dx = world_x - closest_x;
            let dz = world_z - closest_z;
            if dx * dx + dz * dz > radius * radius {
                continue;
            }

            if let Some(gen_chunk) = self.generated_chunks.get_mut(&chunk_id) {
                let resolution = gen_chunk.chunk.heightmap().resolution() as usize;
                let cell_size = chunk_size / (resolution - 1) as f32;
                let mut chunk_modified = false;

                let biome_map = gen_chunk.chunk.biome_map_mut();
                for gz in 0..resolution {
                    for gx in 0..resolution {
                        let px = chunk_origin_x + gx as f32 * cell_size;
                        let pz = chunk_origin_z + gz as f32 * cell_size;
                        let dist = ((px - world_x).powi(2) + (pz - world_z).powi(2)).sqrt();
                        if dist > radius {
                            continue;
                        }
                        let idx = gz * resolution + gx;
                        if idx < biome_map.len() {
                            biome_map[idx] = target_biome;
                            chunk_modified = true;
                        }
                    }
                }

                if chunk_modified {
                    let world_offset = Vec3::new(chunk_origin_x, 0.0, chunk_origin_z);
                    let (vertices, indices) = Self::generate_heightmap_mesh(
                        gen_chunk.chunk.heightmap(),
                        gen_chunk.chunk.biome_map(),
                        chunk_size,
                        world_offset,
                    );
                    gen_chunk.vertices = vertices;
                    gen_chunk.indices = indices;
                    modified = true;
                }
            }
        }

        modified
    }

    /// Get terrain statistics
    pub fn stats(&self) -> TerrainStats {
        TerrainStats {
            chunk_count: self.generated_chunks.len(),
            total_vertices: self.total_vertex_count(),
            total_indices: self.total_index_count(),
            total_triangles: self.total_triangle_count(),
            seed: self.last_seed,
            is_dirty: self.terrain_dirty,
        }
    }

    /// Generate scatter placements for all generated terrain chunks.
    ///
    /// Uses the existing VegetationScatter system from astraweave-terrain
    /// to place vegetation/rocks with Poisson disk sampling and biome-aware
    /// density rules. Returns a flat list of placements ready for the
    /// scatter renderer's GPU instancing pipeline.
    pub fn generate_scatter_placements(&self) -> Vec<ScatterPlacement> {
        let scatter_config = ScatterConfig::default();
        let scatter = VegetationScatter::new(scatter_config);
        let chunk_size = self.config.chunk_size;

        let mut placements = Vec::new();

        for (chunk_id, gen_chunk) in &self.generated_chunks {
            let chunk = &gen_chunk.chunk;

            // Sample biome at chunk center
            let chunk_center = chunk_id.to_center_pos(chunk_size);
            let center_biome = chunk
                .get_biome_at_world_pos(chunk_center, chunk_size)
                .unwrap_or(BiomeType::Grassland);

            let biome_config = self
                .config
                .biomes
                .iter()
                .find(|b| b.biome_type == center_biome)
                .cloned()
                .unwrap_or_else(BiomeConfig::grassland);

            let seed = self.config.seed + chunk_id.x as u64 * 1000 + chunk_id.z as u64;

            // Generate vegetation instances via the terrain scatter system
            let vegetation = match scatter.scatter_vegetation(chunk, chunk_size, &biome_config, seed)
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Scatter failed for chunk {:?}: {e}", chunk_id);
                    continue;
                }
            };

            for vi in vegetation {
                placements.push(ScatterPlacement::from_vegetation_instance(&vi));
            }
        }

        tracing::info!(
            "Generated {} scatter placements across {} chunks",
            placements.len(),
            self.generated_chunks.len()
        );

        placements
    }
}

/// Statistics for terrain state
#[derive(Debug, Clone)]
pub struct TerrainStats {
    /// Number of chunks generated
    pub chunk_count: usize,
    /// Total vertex count
    pub total_vertices: usize,
    /// Total index count
    pub total_indices: usize,
    /// Total triangle count
    pub total_triangles: usize,
    /// Seed used for generation
    pub seed: u64,
    /// Whether terrain needs regeneration
    pub is_dirty: bool,
}

impl TerrainStats {
    /// Check if any terrain has been generated
    pub fn has_terrain(&self) -> bool {
        self.chunk_count > 0
    }

    /// Get average vertices per chunk
    pub fn avg_vertices_per_chunk(&self) -> f32 {
        if self.chunk_count == 0 {
            0.0
        } else {
            self.total_vertices as f32 / self.chunk_count as f32
        }
    }

    /// Get average triangles per chunk
    pub fn avg_triangles_per_chunk(&self) -> f32 {
        if self.chunk_count == 0 {
            0.0
        } else {
            self.total_triangles as f32 / self.chunk_count as f32
        }
    }
}

impl TerrainVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

/// CPU-side scatter placement (bridges terrain VegetationInstance to GPU renderer).
#[derive(Debug, Clone)]
pub struct ScatterPlacement {
    pub position: Vec3,
    pub rotation: f32,
    pub scale: f32,
    pub mesh_key: String,
    pub mesh_path: String,
    pub bounding_radius: f32,
}

impl ScatterPlacement {
    pub fn from_vegetation_instance(vi: &VegetationInstance) -> Self {
        Self {
            position: vi.position,
            rotation: vi.rotation,
            scale: vi.scale,
            mesh_key: vi.vegetation_type.clone(),
            mesh_path: vi.model_path.clone(),
            bounding_radius: vi.scale * 1.5, // conservative estimate
        }
    }
}

pub fn biome_display_name(biome_str: &str) -> &'static str {
    match biome_str {
        "grassland" => "Grassland",
        "desert" => "Desert",
        "forest" => "Forest",
        "mountain" => "Mountain",
        "tundra" => "Tundra",
        "swamp" => "Swamp",
        "beach" => "Beach",
        "river" => "River",
        "temperate_forest" => "Forest",
        _ => "Unknown",
    }
}

pub fn all_biome_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("grassland", "Grassland"),
        ("desert", "Desert"),
        ("forest", "Forest"),
        ("mountain", "Mountain"),
        ("tundra", "Tundra"),
        ("swamp", "Swamp"),
        ("beach", "Beach"),
        ("river", "River"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_state_creation() {
        let state = TerrainState::new();
        assert_eq!(state.chunk_count(), 0);
        assert!(!state.has_terrain());
    }

    #[test]
    fn test_terrain_state_has_terrain() {
        let state = TerrainState::new();
        assert!(!state.has_terrain());
    }

    #[test]
    fn test_terrain_state_total_vertex_count() {
        let state = TerrainState::new();
        assert_eq!(state.total_vertex_count(), 0);
    }

    #[test]
    fn test_terrain_state_total_index_count() {
        let state = TerrainState::new();
        assert_eq!(state.total_index_count(), 0);
    }

    #[test]
    fn test_terrain_state_total_triangle_count() {
        let state = TerrainState::new();
        assert_eq!(state.total_triangle_count(), 0);
    }

    #[test]
    fn test_terrain_state_chunk_ids() {
        let state = TerrainState::new();
        assert!(state.chunk_ids().is_empty());
    }

    #[test]
    fn test_terrain_state_stats() {
        let state = TerrainState::new();
        let stats = state.stats();
        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.total_vertices, 0);
        // New terrain states start as dirty (needing generation)
        assert!(stats.is_dirty);
    }

    // ====================================================================
    // TerrainStats Tests
    // ====================================================================

    #[test]
    fn test_terrain_stats_has_terrain() {
        let no_terrain = TerrainStats {
            chunk_count: 0,
            total_vertices: 0,
            total_indices: 0,
            total_triangles: 0,
            seed: 0,
            is_dirty: false,
        };
        assert!(!no_terrain.has_terrain());

        let with_terrain = TerrainStats {
            chunk_count: 4,
            total_vertices: 1000,
            total_indices: 3000,
            total_triangles: 1000,
            seed: 12345,
            is_dirty: false,
        };
        assert!(with_terrain.has_terrain());
    }

    #[test]
    fn test_terrain_stats_avg_vertices_per_chunk() {
        let stats = TerrainStats {
            chunk_count: 4,
            total_vertices: 1000,
            total_indices: 3000,
            total_triangles: 1000,
            seed: 0,
            is_dirty: false,
        };
        assert!((stats.avg_vertices_per_chunk() - 250.0).abs() < 0.1);
    }

    #[test]
    fn test_terrain_stats_avg_vertices_per_chunk_empty() {
        let stats = TerrainStats {
            chunk_count: 0,
            total_vertices: 0,
            total_indices: 0,
            total_triangles: 0,
            seed: 0,
            is_dirty: false,
        };
        assert!((stats.avg_vertices_per_chunk() - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_terrain_stats_avg_triangles_per_chunk() {
        let stats = TerrainStats {
            chunk_count: 5,
            total_vertices: 1000,
            total_indices: 3000,
            total_triangles: 500,
            seed: 0,
            is_dirty: false,
        };
        assert!((stats.avg_triangles_per_chunk() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_terrain_stats_avg_triangles_per_chunk_empty() {
        let stats = TerrainStats {
            chunk_count: 0,
            total_vertices: 0,
            total_indices: 0,
            total_triangles: 0,
            seed: 0,
            is_dirty: false,
        };
        assert!((stats.avg_triangles_per_chunk() - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_biome_display_name() {
        assert_eq!(biome_display_name("grassland"), "Grassland");
        assert_eq!(biome_display_name("forest"), "Forest");
        assert_eq!(biome_display_name("unknown"), "Unknown");
    }

    #[test]
    fn test_all_biome_options() {
        let options = all_biome_options();
        assert_eq!(options.len(), 8);
        assert_eq!(options[0], ("grassland", "Grassland"));
    }
}
