//! Biome Showcase Demo
//!
//! This example demonstrates the comprehensive biome building and generation system
//! featuring sky/weather, vegetation, structures, and environmental effects.

use anyhow::Result;
use astraweave_camera::{CameraController, FreeFly as Camera};
use astraweave_render::*;
use astraweave_terrain::*;
use clap::Parser;
use glam::Vec3;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Parser, Clone)]
#[command(name = "biome_showcase")]
#[command(about = "AstraWeave Comprehensive Biome Showcase")]
struct Args {
    /// Random seed for world generation
    #[arg(short, long, default_value = "12345")]
    seed: u64,

    /// Size of terrain chunks
    #[arg(short, long, default_value = "256.0")]
    chunk_size: f32,

    /// Resolution of heightmaps (vertices per edge)
    #[arg(short, long, default_value = "128")]
    resolution: u32,

    /// Number of chunks to generate in each direction
    #[arg(short, long, default_value = "5")]
    grid_size: u32,

    /// Starting biome (grassland, desert, forest, mountain, tundra, swamp, beach, river)
    #[arg(short, long, default_value = "grassland")]
    biome: String,

    /// Enable structure generation
    #[arg(long)]
    structures: bool,

    /// Enable weather system
    #[arg(long)]
    weather: bool,

    /// Time of day (0.0-24.0)
    #[arg(long, default_value = "12.0")]
    time: f32,

    /// Weather type (clear, rain, snow, fog, storm, sandstorm)
    #[arg(long)]
    weather_type: Option<String>,

    /// Export statistics and configuration
    #[arg(short, long)]
    export: bool,

    /// Run in headless mode (no graphics)
    #[arg(long)]
    headless: bool,
}

/// Application state for the biome showcase
struct BiomeShowcase {
    /// World configuration
    world_config: WorldConfig,
    /// Terrain renderer
    terrain_renderer: TerrainRenderer,
    /// Sky renderer for environmental effects
    sky_renderer: SkyRenderer,
    /// Weather system
    weather_system: WeatherSystem,
    /// Weather particle system
    weather_particles: WeatherParticles,
    /// Camera for flying through the world
    camera: Camera,
    /// Camera controller for user input
    camera_controller: CameraController,
    /// Generated chunks
    generated_chunks: Vec<ChunkId>,
    /// Statistics
    stats: BiomeStats,
}

/// Statistics about the generated biome
#[derive(Debug, Default)]
struct BiomeStats {
    total_vertices: usize,
    total_triangles: usize,
    total_vegetation: usize,
    total_resources: usize,
    total_structures: usize,
    biome_distribution: std::collections::HashMap<BiomeType, usize>,
    structure_distribution: std::collections::HashMap<StructureType, usize>,
}

impl BiomeShowcase {
    /// Create a new biome showcase application
    fn new(args: &Args) -> Result<Self> {
        // Create world configuration
        let mut world_config = WorldConfig {
            seed: args.seed,
            chunk_size: args.chunk_size,
            heightmap_resolution: args.resolution,
            ..Default::default()
        };

        // Configure structures
        world_config.structures.density = if args.structures { 0.4 } else { 0.0 };
        world_config.structures.include_ancient = true;
        world_config.structures.include_defensive = true;

        // Filter to specific biome if requested
        if let Some(biome_type) = BiomeType::parse(&args.biome) {
            world_config.biomes.retain(|b| b.biome_type == biome_type);
            println!("Focusing on biome: {}", args.biome);
        }

        // Create terrain renderer
        let terrain_renderer = TerrainRenderer::new(world_config.clone());

        // Create sky renderer
        let sky_config = SkyConfig::default();
        let mut sky_renderer = SkyRenderer::new(sky_config);

        // Set initial time of day
        sky_renderer.time_of_day_mut().current_time = args.time;

        // Create weather system
        let mut weather_system = WeatherSystem::new();
        if let Some(weather_str) = &args.weather_type {
            if let Some(weather_type) = parse_weather_type(weather_str) {
                weather_system.set_weather(weather_type, 5.0);
                println!("Starting weather: {:?}", weather_type);
            }
        }

        // Create weather particles
        let weather_particles = WeatherParticles::new(1000, 200.0);

        // Create camera positioned above the center of the world
        let world_center = Vec3::new(
            args.chunk_size * args.grid_size as f32 * 0.5,
            100.0,
            args.chunk_size * args.grid_size as f32 * 0.5,
        );
        let camera = Camera {
            position: world_center,
            yaw: 0.0,
            pitch: -0.2, // Look down slightly
            fovy: 45.0_f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 1000.0,
        };
        let camera_controller = CameraController::new(50.0, 0.002);

        Ok(Self {
            world_config,
            terrain_renderer,
            sky_renderer,
            weather_system,
            weather_particles,
            camera,
            camera_controller,
            generated_chunks: Vec::new(),
            stats: BiomeStats::default(),
        })
    }

    /// Generate the world chunks
    fn generate_world(&mut self, grid_size: u32) -> Result<()> {
        println!("Generating {}x{} chunk world...", grid_size, grid_size);

        for x in 0..grid_size {
            for z in 0..grid_size {
                let chunk_id = ChunkId::new(x as i32, z as i32);

                // Generate chunk with all content
                let (mesh, scatter_result) =
                    self.terrain_renderer.generate_chunk_complete(chunk_id)?;

                // Update statistics
                self.stats.total_vertices += mesh.vertices.len();
                self.stats.total_triangles += mesh.indices.len() / 3;
                self.stats.total_vegetation += scatter_result.vegetation.len();
                self.stats.total_resources += scatter_result.resources.len();
                self.stats.total_structures += scatter_result.structures.len();

                // Track biome distribution
                for vertex in &mesh.vertices {
                    let biome_type = id_to_biome(vertex.biome_id);
                    *self.stats.biome_distribution.entry(biome_type).or_insert(0) += 1;
                }

                // Track structure distribution
                for structure in &scatter_result.structures {
                    *self
                        .stats
                        .structure_distribution
                        .entry(structure.structure_type)
                        .or_insert(0) += 1;
                }

                self.generated_chunks.push(chunk_id);

                println!(
                    "  Chunk ({}, {}): {} vertices, {} vegetation, {} resources, {} structures",
                    x,
                    z,
                    mesh.vertices.len(),
                    scatter_result.vegetation.len(),
                    scatter_result.resources.len(),
                    scatter_result.structures.len()
                );
            }
        }

        println!();
        self.print_statistics();
        Ok(())
    }

    /// Print generation statistics
    fn print_statistics(&self) {
        println!("World Generation Complete!");
        println!("=========================");
        println!("Total Statistics:");
        println!("  Vertices: {}", self.stats.total_vertices);
        println!("  Triangles: {}", self.stats.total_triangles);
        println!("  Vegetation: {}", self.stats.total_vegetation);
        println!("  Resources: {}", self.stats.total_resources);
        println!("  Structures: {}", self.stats.total_structures);

        println!();
        println!("Biome Distribution:");
        for (biome_type, count) in &self.stats.biome_distribution {
            let percentage = (*count as f32 / self.stats.total_vertices as f32) * 100.0;
            println!(
                "  {}: {} vertices ({:.1}%)",
                biome_type.as_str(),
                count,
                percentage
            );
        }

        if !self.stats.structure_distribution.is_empty() {
            println!();
            println!("Structure Distribution:");
            for (structure_type, count) in &self.stats.structure_distribution {
                println!("  {:?}: {}", structure_type, count);
            }
        }
    }

    /// Update the application state
    fn update(&mut self, delta_time: f32) {
        // Update camera
        self.camera_controller
            .update_camera(&mut self.camera, delta_time);

        // Update sky and time of day
        self.sky_renderer.update(delta_time);

        // Update weather system
        self.weather_system.update(delta_time);

        // Update weather particles
        self.weather_particles
            .update(delta_time, self.camera.position, &self.weather_system);
    }

    /// Handle keyboard input
    fn handle_keyboard(&mut self, key: KeyCode, state: ElementState) {
        match (key, state) {
            (KeyCode::Space, ElementState::Pressed) => {
                // Cycle through weather types
                let current = self.weather_system.current_weather();
                let next = match current {
                    WeatherType::Clear => WeatherType::Cloudy,
                    WeatherType::Cloudy => WeatherType::Rain,
                    WeatherType::Rain => WeatherType::Storm,
                    WeatherType::Storm => WeatherType::Snow,
                    WeatherType::Snow => WeatherType::Fog,
                    WeatherType::Fog => WeatherType::Sandstorm,
                    WeatherType::Sandstorm => WeatherType::Clear,
                    _ => WeatherType::Clear,
                };
                self.weather_system.set_weather(next, 3.0);
                println!("Weather changing to: {:?}", next);
            }
            (KeyCode::KeyT, ElementState::Pressed) => {
                // Advance time by 2 hours
                let time = &mut self.sky_renderer.time_of_day_mut().current_time;
                *time = (*time + 2.0) % 24.0;
                println!("Time: {:.1}:00", time);
            }
            _ => {
                self.camera_controller
                    .process_keyboard(key, state == ElementState::Pressed);
            }
        }
    }

    /// Export world data and statistics
    fn export_data(&self, filename_prefix: &str) -> Result<()> {
        // Export statistics
        let stats_file = format!("{}_stats.json", filename_prefix);
        let mut stats_data = serde_json::json!({
            "total_vertices": self.stats.total_vertices,
            "total_triangles": self.stats.total_triangles,
            "total_vegetation": self.stats.total_vegetation,
            "total_resources": self.stats.total_resources,
            "total_structures": self.stats.total_structures,
            "world_config": self.world_config
        });

        // Add biome distribution
        let mut biome_dist = serde_json::Map::new();
        for (biome, count) in &self.stats.biome_distribution {
            biome_dist.insert(
                biome.as_str().to_string(),
                serde_json::Value::Number((*count).into()),
            );
        }
        stats_data["biome_distribution"] = serde_json::Value::Object(biome_dist);

        // Add structure distribution
        let mut struct_dist = serde_json::Map::new();
        for (structure, count) in &self.stats.structure_distribution {
            struct_dist.insert(
                format!("{:?}", structure),
                serde_json::Value::Number((*count).into()),
            );
        }
        stats_data["structure_distribution"] = serde_json::Value::Object(struct_dist);

        std::fs::write(&stats_file, serde_json::to_string_pretty(&stats_data)?)?;
        println!("Statistics exported to: {}", stats_file);

        // Export current environmental state
        let env_file = format!("{}_environment.json", filename_prefix);
        let env_data = serde_json::json!({
            "time_of_day": self.sky_renderer.time_of_day().current_time,
            "weather": self.weather_system.current_weather(),
            "rain_intensity": self.weather_system.get_rain_intensity(),
            "snow_intensity": self.weather_system.get_snow_intensity(),
            "fog_density": self.weather_system.get_fog_density(),
            "wind_strength": self.weather_system.get_wind_strength(),
            "wind_direction": [
                self.weather_system.get_wind_direction().x,
                self.weather_system.get_wind_direction().y,
                self.weather_system.get_wind_direction().z
            ]
        });

        std::fs::write(&env_file, serde_json::to_string_pretty(&env_data)?)?;
        println!("Environment data exported to: {}", env_file);

        Ok(())
    }
}

/// Convert biome ID back to biome type for statistics
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

/// Parse weather type from string
fn parse_weather_type(s: &str) -> Option<WeatherType> {
    match s.to_lowercase().as_str() {
        "clear" => Some(WeatherType::Clear),
        "cloudy" => Some(WeatherType::Cloudy),
        "rain" => Some(WeatherType::Rain),
        "storm" => Some(WeatherType::Storm),
        "snow" => Some(WeatherType::Snow),
        "fog" => Some(WeatherType::Fog),
        "sandstorm" => Some(WeatherType::Sandstorm),
        _ => None,
    }
}

struct App {
    args: Args,
    showcase: Option<BiomeShowcase>,
    window: Option<Window>,
    last_frame: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("AstraWeave Biome Showcase")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
            let window = event_loop.create_window(window_attributes).unwrap();
            self.window = Some(window);

            // Initialize showcase if not already done
            if self.showcase.is_none() {
                match BiomeShowcase::new(&self.args) {
                    Ok(mut showcase) => {
                        if let Err(e) = showcase.generate_world(self.args.grid_size) {
                            eprintln!("Failed to generate world: {}", e);
                        }
                        if self.args.export {
                            if let Err(e) = showcase.export_data("biome_showcase") {
                                eprintln!("Failed to export data: {}", e);
                            }
                        }
                        self.showcase = Some(showcase);
                    }
                    Err(e) => eprintln!("Failed to create showcase: {}", e),
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    if key_code == KeyCode::Escape && event.state == ElementState::Pressed {
                        event_loop.exit();
                    } else if let Some(showcase) = &mut self.showcase {
                        showcase.handle_keyboard(key_code, event.state);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta_time = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                if let Some(showcase) = &mut self.showcase {
                    showcase.update(delta_time);
                }

                // Rendering code would go here

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("AstraWeave Biome Showcase");
    println!("=========================");
    println!("Configuration:");
    println!("  Seed: {}", args.seed);
    println!("  Chunk Size: {}", args.chunk_size);
    println!("  Resolution: {}", args.resolution);
    println!("  Grid Size: {}x{}", args.grid_size, args.grid_size);
    println!("  Structures: {}", args.structures);
    println!("  Weather: {}", args.weather);
    println!("  Time: {:.1}:00", args.time);
    println!("  Headless: {}", args.headless);
    println!();

    if args.headless {
        // Create the showcase application
        let mut showcase = BiomeShowcase::new(&args)?;

        // Generate the world
        showcase.generate_world(args.grid_size)?;

        // Export data if requested
        if args.export {
            showcase.export_data("biome_showcase")?;
        }
        println!("Running in headless mode. Generation complete!");
        return Ok(());
    }

    // Create graphics window and run the interactive showcase
    println!("Starting interactive showcase...");
    println!("Controls:");
    println!("  WASD: Move camera");
    println!("  Mouse: Look around");
    println!("  Space: Cycle weather");
    println!("  T: Advance time");
    println!("  Escape: Exit");
    println!();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        args,
        showcase: None,
        window: None,
        last_frame: Instant::now(),
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}
