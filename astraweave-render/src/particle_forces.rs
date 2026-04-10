//! Enhanced Particle Simulation — Niagara-class force system.
//!
//! Extends the base `GpuParticleSystem` with:
//! - Force sources: gravity, drag, wind, curl-noise turbulence, point attractors
//! - Lifetime curves: 4-key color gradient and size curve
//! - Emission shapes: sphere, cone, box, ring
//!
//! Uses an external WGSL shader (`shaders/particles/simulate.wgsl`) for the
//! compute pass rather than inline source.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// Enhanced particle simulation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SimParams {
    pub delta_time: f32,
    pub particle_count: u32,
    pub max_particles: u32,
    pub random_seed: u32,
    pub gravity: [f32; 3],
    pub drag_coefficient: f32,
    pub wind: [f32; 3],
    pub turbulence_str: f32,
    pub turbulence_freq: f32,
    pub turbulence_speed: f32,
    pub time: f32,
    pub attractor_pos: [f32; 3],
    pub attractor_str: f32,
    pub color0: [f32; 4],
    pub color1: [f32; 4],
    pub color2: [f32; 4],
    pub color3: [f32; 4],
    pub size_keys: [f32; 4],
    pub _pad: [f32; 4],
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Force configuration for particle simulation.
#[derive(Debug, Clone)]
pub struct ParticleForces {
    pub gravity: Vec3,
    pub drag_coefficient: f32,
    pub wind: Vec3,
    pub turbulence_strength: f32,
    pub turbulence_frequency: f32,
    pub turbulence_speed: f32,
    pub attractor_pos: Vec3,
    pub attractor_strength: f32,
}

impl Default for ParticleForces {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            drag_coefficient: 0.5,
            wind: Vec3::ZERO,
            turbulence_strength: 0.0,
            turbulence_frequency: 1.0,
            turbulence_speed: 1.0,
            attractor_pos: Vec3::ZERO,
            attractor_strength: 0.0,
        }
    }
}

/// 4-key color gradient over particle lifetime.
#[derive(Debug, Clone, Copy)]
pub struct ColorGradient {
    pub color0: [f32; 4],
    pub color1: [f32; 4],
    pub color2: [f32; 4],
    pub color3: [f32; 4],
}

impl Default for ColorGradient {
    fn default() -> Self {
        Self {
            color0: [1.0, 1.0, 1.0, 1.0],  // born: white opaque
            color1: [1.0, 0.8, 0.4, 1.0],  // young: warm
            color2: [1.0, 0.3, 0.1, 0.7],  // middle: hot, fading
            color3: [0.3, 0.1, 0.05, 0.0], // death: dark, transparent
        }
    }
}

/// 4-key size curve over particle lifetime.
#[derive(Debug, Clone, Copy)]
pub struct SizeCurve {
    /// Size at t=0.0, 0.33, 0.66, 1.0
    pub keys: [f32; 4],
}

impl Default for SizeCurve {
    fn default() -> Self {
        Self {
            keys: [0.0, 1.0, 0.8, 0.0], // grow in, plateau, shrink out
        }
    }
}

/// Emission shape for spawning new particles.
#[derive(Debug, Clone, Copy, Default)]
pub enum EmissionShape {
    /// Point emission.
    #[default]
    Point,
    /// Sphere with given radius.
    Sphere { radius: f32 },
    /// Cone with half-angle (radians) and height.
    Cone { half_angle: f32, height: f32 },
    /// Axis-aligned box with half-extents.
    Box { half_extents: Vec3 },
    /// Ring with inner and outer radius (XZ plane).
    Ring { inner: f32, outer: f32 },
}

impl EmissionShape {
    /// Sample a random position within the emission shape (CPU-side for init).
    pub fn sample(&self, rng: &mut SimpleRng) -> Vec3 {
        match *self {
            EmissionShape::Point => Vec3::ZERO,
            EmissionShape::Sphere { radius } => {
                let u = rng.next_f32() * 2.0 - 1.0;
                let theta = rng.next_f32() * std::f32::consts::TAU;
                let r = radius * rng.next_f32().cbrt();
                let xy = (1.0 - u * u).sqrt();
                Vec3::new(r * xy * theta.cos(), r * u, r * xy * theta.sin())
            }
            EmissionShape::Cone { half_angle, height } => {
                let theta = rng.next_f32() * std::f32::consts::TAU;
                let t = rng.next_f32();
                let r = t * height * half_angle.tan();
                Vec3::new(r * theta.cos(), t * height, r * theta.sin())
            }
            EmissionShape::Box { half_extents } => {
                let x = (rng.next_f32() * 2.0 - 1.0) * half_extents.x;
                let y = (rng.next_f32() * 2.0 - 1.0) * half_extents.y;
                let z = (rng.next_f32() * 2.0 - 1.0) * half_extents.z;
                Vec3::new(x, y, z)
            }
            EmissionShape::Ring { inner, outer } => {
                let theta = rng.next_f32() * std::f32::consts::TAU;
                let r = inner + rng.next_f32() * (outer - inner);
                Vec3::new(r * theta.cos(), 0.0, r * theta.sin())
            }
        }
    }
}

/// Simple deterministic RNG (xorshift32) for particle emission.
pub struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }

    pub fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

// ---------------------------------------------------------------------------
// Enhanced Simulation Pass
// ---------------------------------------------------------------------------

/// GPU resources for the enhanced particle simulation compute pass.
pub struct ParticleSimPass {
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    params_buf: wgpu::Buffer,
    time: f32,
    frame: u32,
}

impl ParticleSimPass {
    /// Create the enhanced simulation pipeline.
    pub fn new(device: &wgpu::Device) -> Self {
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle_sim_bgl"),
            entries: &[
                // 0: particles_in (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: particles_out (read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: params (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle_sim_params"),
            contents: bytemuck::bytes_of(&SimParams::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle_simulate_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/particles/simulate.wgsl").into(),
            ),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle_sim_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("particle_sim_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("simulate_particles"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bgl,
            params_buf,
            time: 0.0,
            frame: 0,
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bgl
    }

    /// Update simulation parameters and dispatch.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        particles_in: &wgpu::Buffer,
        particles_out: &wgpu::Buffer,
        max_particles: u32,
        dt: f32,
        forces: &ParticleForces,
        gradient: &ColorGradient,
        size_curve: &SizeCurve,
    ) {
        self.time += dt;
        self.frame = self.frame.wrapping_add(1);

        let params = SimParams {
            delta_time: dt,
            particle_count: max_particles,
            max_particles,
            random_seed: self.frame,
            gravity: forces.gravity.to_array(),
            drag_coefficient: forces.drag_coefficient,
            wind: forces.wind.to_array(),
            turbulence_str: forces.turbulence_strength,
            turbulence_freq: forces.turbulence_frequency,
            turbulence_speed: forces.turbulence_speed,
            time: self.time,
            attractor_pos: forces.attractor_pos.to_array(),
            attractor_str: forces.attractor_strength,
            color0: gradient.color0,
            color1: gradient.color1,
            color2: gradient.color2,
            color3: gradient.color3,
            size_keys: size_curve.keys,
            _pad: [0.0; 4],
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_sim_bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particles_in.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particles_out.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buf.as_entire_binding(),
                },
            ],
        });

        let workgroups = max_particles.div_ceil(64);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("particle_simulate"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_params_size() {
        assert_eq!(std::mem::size_of::<SimParams>(), 172);
    }

    #[test]
    fn default_forces() {
        let f = ParticleForces::default();
        assert!(f.gravity.y < 0.0, "Gravity should point down");
        assert!(f.drag_coefficient > 0.0);
    }

    #[test]
    fn default_gradient() {
        let g = ColorGradient::default();
        assert_eq!(g.color0[3], 1.0, "Born particles should be opaque");
        assert_eq!(g.color3[3], 0.0, "Dead particles should be transparent");
    }

    #[test]
    fn default_size_curve() {
        let s = SizeCurve::default();
        assert_eq!(s.keys[0], 0.0, "Born size should be 0");
        assert!(s.keys[1] > s.keys[0], "Should grow");
        assert_eq!(s.keys[3], 0.0, "Death size should be 0");
    }

    #[test]
    fn emission_point() {
        let shape = EmissionShape::Point;
        let mut rng = SimpleRng::new(42);
        let pos = shape.sample(&mut rng);
        assert_eq!(pos, Vec3::ZERO);
    }

    #[test]
    fn emission_sphere() {
        let shape = EmissionShape::Sphere { radius: 5.0 };
        let mut rng = SimpleRng::new(42);
        for _ in 0..100 {
            let pos = shape.sample(&mut rng);
            assert!(pos.length() <= 5.01, "Should be within sphere: {pos}");
        }
    }

    #[test]
    fn emission_box() {
        let he = Vec3::new(2.0, 3.0, 4.0);
        let shape = EmissionShape::Box { half_extents: he };
        let mut rng = SimpleRng::new(42);
        for _ in 0..100 {
            let pos = shape.sample(&mut rng);
            assert!(pos.x.abs() <= he.x + 0.01);
            assert!(pos.y.abs() <= he.y + 0.01);
            assert!(pos.z.abs() <= he.z + 0.01);
        }
    }

    #[test]
    fn emission_ring() {
        let shape = EmissionShape::Ring {
            inner: 3.0,
            outer: 5.0,
        };
        let mut rng = SimpleRng::new(42);
        for _ in 0..100 {
            let pos = shape.sample(&mut rng);
            let r = (pos.x * pos.x + pos.z * pos.z).sqrt();
            assert!(r >= 2.99 && r <= 5.01, "Ring radius violated: {r}");
            assert!(pos.y.abs() < 0.01, "Ring should be flat");
        }
    }

    #[test]
    fn rng_deterministic() {
        let mut a = SimpleRng::new(123);
        let mut b = SimpleRng::new(123);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn rng_f32_range() {
        let mut rng = SimpleRng::new(999);
        for _ in 0..1000 {
            let v = rng.next_f32();
            assert!(v >= 0.0 && v <= 1.0, "Out of range: {v}");
        }
    }

    #[test]
    fn particle_sim_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let pass = ParticleSimPass::new(&device);
        assert!(pass.params_buf.size() > 0);
    }
}
