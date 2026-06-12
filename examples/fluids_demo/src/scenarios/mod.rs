mod laboratory;
mod ocean;

pub use laboratory::LaboratoryScenario;
pub use ocean::OceanScenario;

use astraweave_fluids::renderer::CameraUniform;
use astraweave_fluids::{FluidRenderer, FluidSystem};
use astraweave_physics::PhysicsWorld;

pub trait FluidScenario {
    fn name(&self) -> &str;

    fn init(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        system: &mut FluidSystem,
        physics: &mut PhysicsWorld,
    );

    fn update(
        &mut self,
        dt: f32,
        system: &mut FluidSystem,
        physics: &mut PhysicsWorld,
        camera_pos: glam::Vec3,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );

    #[allow(clippy::too_many_arguments)]
    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        scene_view: &wgpu::TextureView,
        scene_depth_view: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        system: &FluidSystem,
        renderer: &FluidRenderer,
        camera_uniform: CameraUniform,
        skybox: &wgpu::TextureView,
    );
}

pub struct ScenarioManager {
    scenarios: Vec<Box<dyn FluidScenario>>,
    current_index: usize,
}

impl ScenarioManager {
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
            current_index: 0,
        }
    }

    pub fn add_scenario(&mut self, scenario: Box<dyn FluidScenario>) {
        self.scenarios.push(scenario);
    }

    pub fn current(&mut self) -> Option<&mut Box<dyn FluidScenario>> {
        self.scenarios.get_mut(self.current_index)
    }

    /// Index of the active scenario (0 = Laboratory). Used by the demo to
    /// gate particle spawning, which only the Laboratory scenario renders.
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn next(&mut self) {
        self.current_index = (self.current_index + 1) % self.scenarios.len();
    }
}
