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
        // F.1.2 (H-6a): was 10.0. With h=0.5 the kernel's self-contribution
        // alone is ~3.8 and the spawn lattice (0.45 spacing) measures ~3.85,
        // so a target of 10 keeps the density constraint maximally violated
        // forever — permanent attraction churn that held the dam as a roiling
        // droplet fog instead of a pool (capture evidence f0600 pre-fix).
        // 4.2 puts the rest state just above spawn packing: mild cohesion,
        // gravity wins, the basin can settle.
        system.target_density = 4.2;
        // F.1.2 (H-6a): was 40.0. `viscosity` scales the shader's vorticity-
        // confinement gain (x0.1), and F.1's settling measurements put gains
        // of this magnitude in a permanently-jittering regime: the dam
        // "exploded" into a gas of isolated spheres and no surface could ever
        // form (capture evidence f0120/f0400 pre-fix). 0.5 keeps a little
        // vortical energy without the popcorn machine.
        system.viscosity = 0.5;
        system.surface_tension = 0.1;
        system.gravity = -9.81;

        // Setup dam break volume
        let particle_count = system.particle_count;
        let mut particles = Vec::with_capacity(particle_count as usize);

        // F.1.2 (H-3): reserve a spawn pool. `spawn_particles` only draws
        // from the despawn free-list, and `reset_particles` marks every
        // particle active — so with the full count in the dam, the click-to-
        // spawn feature could NEVER work (the free-list stayed empty for the
        // demo's whole life). The last SPAWN_RESERVE particles are placed in
        // a far-corner block and despawned immediately below, populating the
        // free-list for interactive spawning.
        const SPAWN_RESERVE: usize = 2000;
        let dam_count = (particle_count as usize).saturating_sub(SPAWN_RESERVE);

        // Arrange particles in a block on one side.
        // F.1.2: was 20 wide x 50 tall — a 22-unit pillar whose collapse
        // scattered 20k particles across the whole 60x60 floor into a thin
        // non-fusing layer (capture evidence). A wide, low block pools into
        // a compact basin where the SSFR surface can actually form.
        let spacing = 0.45;
        let width = 32;
        let height = 18;
        let _depth = 32;

        let make_particle = |x: f32, y: f32, z: f32| astraweave_fluids::Particle {
            position: [x, y, z, 1.0],
            velocity: [0.0; 4],
            predicted_position: [x, y, z, 1.0],
            color: [0.3, 0.6, 1.0, 1.0], // Azure blue water
            lambda: 0.0,
            density: 0.0,
            phase: 0,           // 0 = water
            temperature: 293.0, // Room temperature in Kelvin
        };

        for i in 0..dam_count {
            let x = (i % width) as f32 * spacing + 1.0;
            let y = ((i / width) % height) as f32 * spacing + 1.0;
            let z = (i / (width * height)) as f32 * spacing + 1.0;
            particles.push(make_particle(x, y, z));
        }
        // Reserve block: tight grid in a far high corner, inside the world
        // box (|x|,|z| <= 29.5, y <= 59.5), away from the dam.
        for i in 0..(particle_count as usize - dam_count) {
            let x = 27.0 + (i % 13) as f32 * 0.2;
            let y = 55.0 + ((i / 13) % 13) as f32 * 0.2;
            let z = 27.0 + (i / 169) as f32 * 0.2;
            particles.push(make_particle(x, y, z));
        }

        system.reset_particles(queue, &particles);
        // Queue the reserve for despawn (processed by the next step()); the
        // freed slots become the interactive spawn pool.
        system.despawn_region(queue, [26.5, 54.5, 26.5], [29.6, 59.6, 29.6]);

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
