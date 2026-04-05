//! Terrain rendering integration for astraweave-render

use astraweave_terrain::{ScatterResult, TerrainChunk, WorldConfig, WorldGenerator};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// A simple terrain mesh for rendering
#[derive(Debug)]
pub struct TerrainMesh {
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
    pub chunk_id: astraweave_terrain::ChunkId,
}

/// Vertex format for terrain rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub biome_id: u32,
}

/// Terrain rendering system that integrates with WorldGenerator
pub struct TerrainRenderer {
    world_generator: WorldGenerator,
    loaded_meshes: std::collections::HashMap<astraweave_terrain::ChunkId, TerrainMesh>,
    chunk_size: f32,
}

impl TerrainRenderer {
    /// Create a new terrain renderer
    pub fn new(world_config: WorldConfig) -> Self {
        let chunk_size = world_config.chunk_size;
        let world_generator = WorldGenerator::new(world_config);

        Self {
            world_generator,
            loaded_meshes: std::collections::HashMap::new(),
            chunk_size,
        }
    }

    /// Generate or get a terrain mesh for the given chunk
    pub fn get_or_generate_chunk_mesh(
        &mut self,
        chunk_id: astraweave_terrain::ChunkId,
    ) -> anyhow::Result<&TerrainMesh> {
        if !self.loaded_meshes.contains_key(&chunk_id) {
            let chunk = self.world_generator.generate_chunk(chunk_id)?;
            let mesh = self.create_terrain_mesh(&chunk)?;
            self.loaded_meshes.insert(chunk_id, mesh);
        }

        self.loaded_meshes
            .get(&chunk_id)
            .ok_or_else(|| anyhow::anyhow!("BUG: chunk {chunk_id:?} missing after insert"))
    }

    /// Generate a complete chunk with vegetation and resources
    pub fn generate_chunk_complete(
        &mut self,
        chunk_id: astraweave_terrain::ChunkId,
    ) -> anyhow::Result<(TerrainMesh, ScatterResult)> {
        let (chunk, scatter_result) = self.world_generator.generate_chunk_with_scatter(chunk_id)?;
        let mesh = self.create_terrain_mesh(&chunk)?;
        Ok((mesh, scatter_result))
    }

    /// Create a terrain mesh from a terrain chunk
    fn create_terrain_mesh(&self, chunk: &TerrainChunk) -> anyhow::Result<TerrainMesh> {
        let heightmap = chunk.heightmap();
        let resolution = heightmap.resolution();
        let chunk_origin = chunk.id().to_world_pos(self.chunk_size);

        let mut vertices = Vec::new();
        let step = self.chunk_size / (resolution - 1) as f32;

        // Generate vertices
        for z in 0..resolution {
            for x in 0..resolution {
                let world_x = chunk_origin.x + x as f32 * step;
                let world_z = chunk_origin.z + z as f32 * step;
                let height = heightmap.get_height(x, z);

                let position = [world_x, height, world_z];

                // Calculate normal
                let normal = heightmap.calculate_normal(x, z, step);
                let normal_array = [normal.x, normal.y, normal.z];

                // UV coordinates
                let u = x as f32 / (resolution - 1) as f32;
                let v = z as f32 / (resolution - 1) as f32;
                let uv = [u, v];

                // Get biome at this position
                let biome_index = z as usize * resolution as usize + x as usize;
                let biome = chunk
                    .biome_map()
                    .get(biome_index)
                    .copied()
                    .unwrap_or(astraweave_terrain::BiomeType::Grassland);
                let biome_id = Self::biome_to_id(biome);

                vertices.push(TerrainVertex {
                    position,
                    normal: normal_array,
                    uv,
                    biome_id,
                });
            }
        }

        // Generate indices
        let indices = heightmap.generate_indices();

        Ok(TerrainMesh {
            vertices,
            indices,
            chunk_id: chunk.id(),
        })
    }

    /// Convert biome type to numeric ID for shading
    fn biome_to_id(biome: astraweave_terrain::BiomeType) -> u32 {
        match biome {
            astraweave_terrain::BiomeType::Grassland => 0,
            astraweave_terrain::BiomeType::Desert => 1,
            astraweave_terrain::BiomeType::Forest => 2,
            astraweave_terrain::BiomeType::Mountain => 3,
            astraweave_terrain::BiomeType::Tundra => 4,
            astraweave_terrain::BiomeType::Swamp => 5,
            astraweave_terrain::BiomeType::Beach => 6,
            astraweave_terrain::BiomeType::River => 7,
            // Fallback for future biome types
            _ => 0,
        }
    }

    /// Get chunks in a radius around a center position
    pub fn get_chunks_in_radius(
        &mut self,
        center: Vec3,
        radius: u32,
    ) -> anyhow::Result<Vec<astraweave_terrain::ChunkId>> {
        let chunks_needed =
            astraweave_terrain::ChunkId::get_chunks_in_radius(center, radius, self.chunk_size);

        // Generate all needed chunks first
        for chunk_id in &chunks_needed {
            if !self.loaded_meshes.contains_key(chunk_id) {
                self.get_or_generate_chunk_mesh(*chunk_id)?;
            }
        }

        Ok(chunks_needed)
    }

    /// Get a loaded mesh by chunk ID (must be loaded first)
    pub fn get_loaded_mesh(&self, chunk_id: astraweave_terrain::ChunkId) -> Option<&TerrainMesh> {
        self.loaded_meshes.get(&chunk_id)
    }

    /// Create GPU buffers for a terrain mesh
    pub fn create_gpu_buffers(
        device: &wgpu::Device,
        mesh: &TerrainMesh,
    ) -> (wgpu::Buffer, wgpu::Buffer) {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Terrain Vertex Buffer {:?}", mesh.chunk_id)),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Terrain Index Buffer {:?}", mesh.chunk_id)),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer)
    }

    /// Create simple cube instances for vegetation (placeholder until proper models)
    pub fn create_vegetation_instances(
        scatter_result: &ScatterResult,
    ) -> Vec<VegetationRenderInstance> {
        scatter_result
            .vegetation
            .iter()
            .map(|veg| {
                let transform_matrix = Mat4::from_scale_rotation_translation(
                    Vec3::splat(veg.scale),
                    glam::Quat::from_rotation_y(veg.rotation),
                    veg.position,
                );
                VegetationRenderInstance {
                    transform: transform_matrix.to_cols_array(),
                    vegetation_type: Self::vegetation_type_to_id(&veg.vegetation_type),
                }
            })
            .collect()
    }

    /// Convert vegetation type name to numeric ID
    fn vegetation_type_to_id(name: &str) -> u32 {
        match name {
            "grass_cluster" => 0,
            "oak_tree" => 1,
            "wildflowers" => 2,
            "cactus" => 3,
            "desert_shrub" => 4,
            "pine_tree" => 5,
            "birch_tree" => 6,
            "fern" => 7,
            "mushroom" => 8,
            "alpine_tree" => 9,
            "mountain_grass" => 10,
            "boulder" => 11,
            _ => 0, // Default to grass
        }
    }

    /// Get the world generator (for configuration access)
    pub fn world_generator(&self) -> &WorldGenerator {
        &self.world_generator
    }

    /// Get mutable access to world generator
    pub fn world_generator_mut(&mut self) -> &mut WorldGenerator {
        &mut self.world_generator
    }
}

/// Vegetation instance for rendering  
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VegetationRenderInstance {
    pub transform: [f32; 16], // Store as array instead of Mat4
    pub vegetation_type: u32,
}

/// Simple terrain preview generator for testing
pub fn generate_terrain_preview(
    world_config: &WorldConfig,
    center: Vec3,
    size: u32,
) -> anyhow::Result<Vec<f32>> {
    let generator = WorldGenerator::new(world_config.clone());
    let chunk_id = astraweave_terrain::ChunkId::from_world_pos(center, world_config.chunk_size);
    let chunk = generator.generate_chunk(chunk_id)?;

    let heightmap = chunk.heightmap();
    let mut preview = Vec::with_capacity((size * size) as usize);

    let step = world_config.chunk_size / size as f32;
    for z in 0..size {
        for x in 0..size {
            let u = x as f32 * step / world_config.chunk_size * (heightmap.resolution() - 1) as f32;
            let v = z as f32 * step / world_config.chunk_size * (heightmap.resolution() - 1) as f32;
            let height = heightmap.sample_bilinear(u, v);
            preview.push(height);
        }
    }

    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_terrain::{ChunkId, WorldConfig};

    #[test]
    fn test_terrain_renderer_creation() {
        let config = WorldConfig::default();
        let renderer = TerrainRenderer::new(config);
        assert_eq!(renderer.chunk_size, 256.0);
    }

    #[test]
    fn test_mesh_generation() -> anyhow::Result<()> {
        let config = WorldConfig::default();
        let mut renderer = TerrainRenderer::new(config);

        let chunk_id = ChunkId::new(0, 0);
        let mesh = renderer.get_or_generate_chunk_mesh(chunk_id)?;

        assert_eq!(mesh.chunk_id, chunk_id);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        Ok(())
    }

    #[test]
    fn test_chunk_radius_loading() -> anyhow::Result<()> {
        let config = WorldConfig::default();
        let mut renderer = TerrainRenderer::new(config);

        let center = Vec3::new(128.0, 0.0, 128.0);
        let chunk_ids = renderer.get_chunks_in_radius(center, 1)?;

        assert!(!chunk_ids.is_empty());

        // Check that meshes were actually loaded
        for chunk_id in chunk_ids {
            assert!(renderer.get_loaded_mesh(chunk_id).is_some());
        }

        Ok(())
    }

    #[test]
    fn test_terrain_preview() -> anyhow::Result<()> {
        let config = WorldConfig::default();
        let center = Vec3::new(128.0, 0.0, 128.0);

        let preview = generate_terrain_preview(&config, center, 32)?;
        assert_eq!(preview.len(), 32 * 32);

        Ok(())
    }

    #[test]
    fn test_biome_id_conversion() {
        assert_eq!(
            TerrainRenderer::biome_to_id(astraweave_terrain::BiomeType::Grassland),
            0
        );
        assert_eq!(
            TerrainRenderer::biome_to_id(astraweave_terrain::BiomeType::Desert),
            1
        );
        assert_eq!(
            TerrainRenderer::biome_to_id(astraweave_terrain::BiomeType::Forest),
            2
        );
    }

    #[test]
    fn test_vegetation_type_conversion() {
        assert_eq!(TerrainRenderer::vegetation_type_to_id("grass_cluster"), 0);
        assert_eq!(TerrainRenderer::vegetation_type_to_id("oak_tree"), 1);
        assert_eq!(TerrainRenderer::vegetation_type_to_id("unknown"), 0); // Default
    }
}
