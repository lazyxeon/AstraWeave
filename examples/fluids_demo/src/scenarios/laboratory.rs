use super::FluidScenario;
use astraweave_fluids::renderer::CameraUniform;
use astraweave_fluids::{DynamicObject, FluidRenderer, FluidSystem};
use astraweave_physics::{Layers, PhysicsWorld};
use glam::Vec3;

pub struct LaboratoryScenario {
    name: String,
}

impl LaboratoryScenario {
    pub fn new() -> Self {
        Self {
            name: "Laboratory (Dam Break)".to_string(),
        }
    }
}

impl FluidScenario for LaboratoryScenario {
    fn name(&self) -> &str {
        &self.name
    }

    fn init(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        system: &mut FluidSystem,
        physics: &mut PhysicsWorld,
    ) {
        // Reset system for dam break
        system.smoothing_radius = 0.5;
        system.target_density = 10.0;
        system.viscosity = 40.0;
        system.surface_tension = 0.1;
        system.gravity = -9.81;

        // Setup dam break volume
        let particle_count = system.particle_count;
        let mut particles = Vec::with_capacity(particle_count as usize);

        // Arrange particles in a block on one side
        let spacing = 0.45;
        let width = 20;
        let height = 50;
        let _depth = 20;

        for i in 0..particle_count as usize {
            let x = (i % width) as f32 * spacing + 1.0;
            let y = ((i / width) % height) as f32 * spacing + 1.0;
            let z = (i / (width * height)) as f32 * spacing + 1.0;

            particles.push(astraweave_fluids::Particle {
                position: [x, y, z, 1.0],
                velocity: [0.0; 4],
                predicted_position: [x, y, z, 1.0],
                color: [0.3, 0.6, 1.0, 1.0], // Azure blue water
                lambda: 0.0,
                density: 0.0,
                phase: 0,           // 0 = water
                temperature: 293.0, // Room temperature in Kelvin
            });
        }

        system.reset_particles(queue, &particles);

        // Add some dynamic objects for buoyancy testing
        let box_id = physics.add_dynamic_box(
            Vec3::new(10.0, 15.0, 10.0),
            Vec3::new(1.0, 1.0, 1.0),
            10.0,
            Layers::DEFAULT,
        );
        physics.add_buoyancy(box_id, 8.0, 5.0); // Floats well (volume > mass/density ratio)

        let sphere_id = physics.add_dynamic_box(
            // Using box for simplicity in physics lib wrapper
            Vec3::new(5.0, 20.0, 5.0),
            Vec3::new(0.5, 0.5, 0.5),
            5.0,
            Layers::DEFAULT,
        );
        physics.add_buoyancy(sphere_id, 0.4, 2.0); // Sinks (volume < mass/density ratio)
    }

    fn update(
        &mut self,
        _dt: f32,
        system: &mut FluidSystem,
        physics: &mut PhysicsWorld,
        _camera_pos: glam::Vec3,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        // Dynamic water level based on scenario state
        physics.water_level = 4.0;
        physics.fluid_density = 1000.0;

        // Sync physics bodies to fluid system for collisions
        let mut dynamic_objects = Vec::new();

        // We can iterate over buoyancy bodies or all dynamic bodies
        for &id in physics.buoyancy_bodies.keys() {
            if let Some(transform) = physics.body_transform(id) {
                let inv_transform = transform.inverse();

                // For this demo, we'll assume they are boxes of size [1.0, 1.0, 1.0]
                // but we should ideally pull this from collider data
                dynamic_objects.push(DynamicObject {
                    transform: transform.to_cols_array_2d(),
                    inv_transform: inv_transform.to_cols_array_2d(),
                    half_extents: [1.0, 1.0, 1.0, 0.0], // 0.0 = Box
                });
            }
        }

        system.update_objects(queue, &dynamic_objects);
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        scene_view: &wgpu::TextureView,
        scene_depth_view: &wgpu::TextureView,
        _depth: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        system: &FluidSystem,
        renderer: &FluidRenderer,
        camera_uniform: CameraUniform,
        skybox: &wgpu::TextureView,
    ) {
        renderer.render(
            encoder,
            view,
            scene_view,
            scene_depth_view,
            skybox,
            system.get_particle_buffer(),
            system.particle_count,
            system.secondary_particle_buffer(),
            system.secondary_particle_count(),
            camera_uniform,
            queue,
            device,
        );
    }
}
