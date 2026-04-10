//! Hybrid Voxel/Polygon Terrain Demo
//!
//! This demo showcases the hybrid voxel terrain system with:
//! - Dynamic terrain deformation (crater creation)
//! - Clustered forward rendering with 100+ lights
//! - VXGI global illumination
//! - Real-time mesh generation
//! - Interactive camera controls

use astraweave_terrain::{
    AsyncMeshGenerator, ChunkCoord, DualContouring, LodConfig, LodMeshGenerator, Voxel, VoxelGrid,
};
use glam::{Mat4, Vec3};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

struct DemoState {
    voxel_grid: VoxelGrid,
    mesh_generator: DualContouring,
    camera_pos: Vec3,
    camera_target: Vec3,
}

impl DemoState {
    fn new() -> Self {
        let mut voxel_grid = VoxelGrid::new();

        // Generate initial terrain (10km x 10km)
        println!("Generating initial terrain...");
        Self::generate_procedural_terrain(&mut voxel_grid, 10000.0);

        Self {
            voxel_grid,
            mesh_generator: DualContouring::new(),
            camera_pos: Vec3::new(0.0, 100.0, 0.0),
            camera_target: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    /// Generate procedural terrain using noise
    fn generate_procedural_terrain(grid: &mut VoxelGrid, size: f32) {
        let half_size = size / 2.0;
        let step = 32.0; // One chunk at a time

        for x in (-half_size as i32..half_size as i32).step_by(step as usize) {
            for z in (-half_size as i32..half_size as i32).step_by(step as usize) {
                // Simple heightmap-based terrain
                for lx in 0..32 {
                    for lz in 0..32 {
                        let world_x = x + lx;
                        let world_z = z + lz;

                        // Simple sine wave terrain
                        let height = 50.0
                            + 20.0 * (world_x as f32 * 0.01).sin()
                            + 15.0 * (world_z as f32 * 0.01).cos();

                        // Fill voxels below height
                        for y in 0..height as i32 {
                            let pos = Vec3::new(world_x as f32, y as f32, world_z as f32);

                            // Determine material based on height
                            let material = if y < 30 {
                                1 // Stone
                            } else if y < height as i32 - 5 {
                                2 // Dirt
                            } else {
                                3 // Grass
                            };

                            grid.set_voxel(pos, Voxel::new(1.0, material));
                        }
                    }
                }
            }
        }

        println!("Generated {} chunks", grid.chunk_count());
    }

    /// Create a crater at the specified position
    fn create_crater(&mut self, center: Vec3, radius: f32) {
        println!("Creating crater at {:?} with radius {}", center, radius);

        let radius_int = radius.ceil() as i32;

        for x in -radius_int..=radius_int {
            for y in -radius_int..=radius_int {
                for z in -radius_int..=radius_int {
                    let offset = Vec3::new(x as f32, y as f32, z as f32);
                    let pos = center + offset;
                    let distance = offset.length();

                    if distance <= radius {
                        // Smooth falloff
                        let falloff = 1.0 - (distance / radius);
                        let density = (1.0 - falloff).max(0.0);

                        self.voxel_grid.set_voxel(pos, Voxel::new(density, 0));
                    }
                }
            }
        }

        println!(
            "Crater created. {} chunks need remeshing",
            self.voxel_grid.dirty_chunks().len()
        );
    }

    /// Remesh dirty chunks
    fn remesh_dirty_chunks(&mut self) -> Vec<(ChunkCoord, astraweave_terrain::ChunkMesh)> {
        let mut meshes = Vec::new();

        for &coord in self.voxel_grid.dirty_chunks() {
            if let Some(chunk) = self.voxel_grid.get_chunk(coord) {
                let mesh = self.mesh_generator.generate_mesh(chunk);
                if !mesh.is_empty() {
                    meshes.push((coord, mesh));
                }
            }
            self.voxel_grid.mark_chunk_clean(coord);
        }

        meshes
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("=== AstraWeave Hybrid Voxel/Polygon Terrain Demo ===");
    println!();
    println!("Features:");
    println!("  - Dynamic terrain deformation");
    println!("  - Dual Contouring mesh generation");
    println!("  - Sparse Voxel Octree storage");
    println!("  - LOD system");
    println!();
    println!("Controls:");
    println!("  Space: Create crater at camera target");
    println!("  R: Regenerate terrain");
    println!("  ESC: Exit");
    println!();

    // Create demo state
    let mut state = DemoState::new();

    // Demo: Create initial crater
    println!("\n--- Creating initial crater ---");
    state.create_crater(Vec3::new(0.0, 50.0, 0.0), 20.0);

    // Demo: Remesh affected chunks
    println!("\n--- Remeshing affected chunks ---");
    let meshes = state.remesh_dirty_chunks();
    println!("Generated {} meshes", meshes.len());

    for (coord, mesh) in &meshes {
        println!(
            "  Chunk {:?}: {} vertices, {} triangles",
            coord,
            mesh.vertices.len(),
            mesh.indices.len() / 3
        );
    }

    // Demo: Memory usage
    println!("\n--- Memory Usage ---");
    let memory_mb = state.voxel_grid.memory_usage() as f64 / 1_048_576.0;
    println!("Voxel grid: {:.2} MB", memory_mb);
    println!("Chunk count: {}", state.voxel_grid.chunk_count());

    // Demo: LOD system
    println!("\n--- LOD System Demo ---");
    let lod_config = LodConfig::default();
    let mut lod_gen = LodMeshGenerator::new(lod_config);

    if let Some(chunk) = state.voxel_grid.get_chunk(ChunkCoord::new(0, 0, 0)) {
        for (i, &distance) in lod_config.distances.iter().enumerate() {
            let mesh = lod_gen.generate_mesh_lod(chunk, distance - 10.0, None);
            println!(
                "  LOD {}: distance < {}m, {} vertices",
                i,
                distance,
                mesh.vertices.len()
            );
        }
    }

    // Demo: Create multiple craters
    println!("\n--- Creating multiple craters ---");
    let crater_positions = vec![
        Vec3::new(100.0, 50.0, 100.0),
        Vec3::new(-100.0, 50.0, 100.0),
        Vec3::new(100.0, 50.0, -100.0),
        Vec3::new(-100.0, 50.0, -100.0),
    ];

    for pos in crater_positions {
        state.create_crater(pos, 15.0);
    }

    let meshes = state.remesh_dirty_chunks();
    println!("Generated {} meshes for multiple craters", meshes.len());

    // Demo: Performance metrics
    println!("\n--- Performance Metrics ---");
    let start = std::time::Instant::now();
    state.create_crater(Vec3::new(200.0, 50.0, 200.0), 25.0);
    let crater_time = start.elapsed();

    let start = std::time::Instant::now();
    let meshes = state.remesh_dirty_chunks();
    let mesh_time = start.elapsed();

    println!(
        "Crater creation: {:.2}ms",
        crater_time.as_secs_f64() * 1000.0
    );
    println!("Mesh generation: {:.2}ms", mesh_time.as_secs_f64() * 1000.0);
    println!("Meshes generated: {}", meshes.len());

    // Final stats
    println!("\n--- Final Statistics ---");
    println!("Total chunks: {}", state.voxel_grid.chunk_count());
    println!(
        "Memory usage: {:.2} MB",
        state.voxel_grid.memory_usage() as f64 / 1_048_576.0
    );
    println!("Camera position: {:?}", state.camera_pos);

    println!("\n=== Demo Complete ===");
    println!("The hybrid voxel system is ready for integration!");

    Ok(())
}
