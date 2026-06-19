//! Fluid Emitter Module
//!
//! Provides mesh-based and point-based fluid emitters for runtime particle spawning.

/// Type alias for particle spawn data: (positions, velocities, colors)
type ParticleSpawnData = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 4]>);

/// A fluid emitter that spawns particles from mesh surfaces or points.
pub struct FluidEmitter {
    /// World-space transform for the emitter
    pub transform: [[f32; 4]; 4],
    /// Emission rate in particles per second
    pub rate: f32,
    /// Base velocity for emitted particles (local space)
    pub velocity: [f32; 3],
    /// Random velocity jitter magnitude
    pub jitter: f32,
    /// Particle color for emitted particles
    pub color: [f32; 4],
    /// Emitter shape
    pub shape: EmitterShape,
    /// Internal accumulator for emission timing
    accumulator: f32,
    /// Whether the emitter is currently active
    pub enabled: bool,
}

/// Shape of the fluid emitter source
#[non_exhaustive]
pub enum EmitterShape {
    /// Single point emitter
    Point,
    /// Spherical volume emitter
    Sphere { radius: f32 },
    /// Axis-aligned box volume emitter
    Box { half_extents: [f32; 3] },
    /// Mesh surface emitter (vertices + normals)
    Mesh {
        vertices: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
    },
}

impl Default for FluidEmitter {
    fn default() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            rate: 100.0,
            velocity: [0.0, -1.0, 0.0],
            jitter: 0.1,
            color: [0.2, 0.5, 0.8, 1.0],
            shape: EmitterShape::Point,
            accumulator: 0.0,
            enabled: true,
        }
    }
}

impl FluidEmitter {
    /// Create a new point emitter at the given position.
    pub fn point(position: [f32; 3], rate: f32) -> Self {
        let mut emitter = Self::default();
        emitter.transform[3] = [position[0], position[1], position[2], 1.0];
        emitter.rate = rate;
        emitter
    }

    /// Create a new spherical volume emitter.
    pub fn sphere(center: [f32; 3], radius: f32, rate: f32) -> Self {
        let mut emitter = Self::default();
        emitter.transform[3] = [center[0], center[1], center[2], 1.0];
        emitter.shape = EmitterShape::Sphere { radius };
        emitter.rate = rate;
        emitter
    }

    /// Create a new mesh surface emitter.
    pub fn from_mesh(vertices: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, rate: f32) -> Self {
        Self {
            shape: EmitterShape::Mesh { vertices, normals },
            rate,
            ..Default::default()
        }
    }

    /// Update the emitter and return particles to spawn.
    /// Returns (positions, velocities, colors) for spawning.
    pub fn tick(&mut self, dt: f32) -> ParticleSpawnData {
        if !self.enabled {
            return (Vec::new(), Vec::new(), Vec::new());
        }

        self.accumulator += dt * self.rate;
        let count = self.accumulator as usize;
        self.accumulator -= count as f32;

        let mut positions = Vec::with_capacity(count);
        let mut velocities = Vec::with_capacity(count);
        let mut colors = Vec::with_capacity(count);

        let origin = [
            self.transform[3][0],
            self.transform[3][1],
            self.transform[3][2],
        ];

        for i in 0..count {
            let (pos, normal) = self.sample_shape(i as u32);

            // Apply jitter
            let jitter_seed = (i as f32 * 12.9898).sin() * 43_758.547;
            let jx = (jitter_seed.fract() - 0.5) * self.jitter;
            let jy = ((jitter_seed * 2.0).fract() - 0.5) * self.jitter;
            let jz = ((jitter_seed * 3.0).fract() - 0.5) * self.jitter;

            positions.push([origin[0] + pos[0], origin[1] + pos[1], origin[2] + pos[2]]);

            velocities.push([
                self.velocity[0] * normal[0] + jx,
                self.velocity[1] * normal[1] + jy,
                self.velocity[2] * normal[2] + jz,
            ]);

            colors.push(self.color);
        }

        (positions, velocities, colors)
    }

    fn sample_shape(&self, seed: u32) -> ([f32; 3], [f32; 3]) {
        match &self.shape {
            EmitterShape::Point => ([0.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
            EmitterShape::Sphere { radius } => {
                // Random point on sphere surface
                let theta = (seed as f32 * 0.618033) * std::f32::consts::TAU;
                let phi = ((seed as f32 * 0.414213).fract() - 0.5) * std::f32::consts::PI;
                let x = phi.cos() * theta.cos() * radius;
                let y = phi.sin() * radius;
                let z = phi.cos() * theta.sin() * radius;
                let n = [theta.cos() * phi.cos(), phi.sin(), theta.sin() * phi.cos()];
                ([x, y, z], n)
            }
            EmitterShape::Box { half_extents } => {
                // Random point in box volume
                let fx = ((seed as f32 * 0.123456).fract() - 0.5) * 2.0;
                let fy = ((seed as f32 * 0.654321).fract() - 0.5) * 2.0;
                let fz = ((seed as f32 * 0.987654).fract() - 0.5) * 2.0;
                (
                    [
                        fx * half_extents[0],
                        fy * half_extents[1],
                        fz * half_extents[2],
                    ],
                    [0.0, -1.0, 0.0],
                )
            }
            EmitterShape::Mesh { vertices, normals } => {
                if vertices.is_empty() {
                    return ([0.0, 0.0, 0.0], [0.0, -1.0, 0.0]);
                }
                let idx = seed as usize % vertices.len();
                (
                    vertices[idx],
                    normals.get(idx).copied().unwrap_or([0.0, -1.0, 0.0]),
                )
            }
        }
    }
}

/// A drain that removes particles from a region
pub struct FluidDrain {
    /// Center position of the drain
    pub position: [f32; 3],
    /// Radius of the drain sphere
    pub radius: f32,
    /// Whether the drain is currently active
    pub enabled: bool,
}

impl FluidDrain {
    pub fn new(position: [f32; 3], radius: f32) -> Self {
        Self {
            position,
            radius,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================== FluidEmitter Default Tests ==================

    #[test]
    fn test_fluid_emitter_default() {
        let emitter = FluidEmitter::default();

        // Check identity transform
        assert_eq!(emitter.transform[0], [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(emitter.transform[1], [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(emitter.transform[2], [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(emitter.transform[3], [0.0, 0.0, 0.0, 1.0]);

        // Check default values
        assert_eq!(emitter.rate, 100.0);
        assert_eq!(emitter.velocity, [0.0, -1.0, 0.0]);
        assert_eq!(emitter.jitter, 0.1);
        assert_eq!(emitter.color, [0.2, 0.5, 0.8, 1.0]);
        assert!(matches!(emitter.shape, EmitterShape::Point));
        assert_eq!(emitter.accumulator, 0.0);
        assert!(emitter.enabled);
    }

    // ================== Point Emitter Tests ==================

    #[test]
    fn test_point_emitter_creation() {
        let position = [5.0, 10.0, 15.0];
        let rate = 50.0;
        let emitter = FluidEmitter::point(position, rate);

        // Check position is in transform
        assert_eq!(emitter.transform[3][0], 5.0);
        assert_eq!(emitter.transform[3][1], 10.0);
        assert_eq!(emitter.transform[3][2], 15.0);
        assert_eq!(emitter.transform[3][3], 1.0);

        // Check rate
        assert_eq!(emitter.rate, 50.0);

        // Check shape is Point
        assert!(matches!(emitter.shape, EmitterShape::Point));
    }

    #[test]
    fn test_point_emitter_at_origin() {
        let emitter = FluidEmitter::point([0.0, 0.0, 0.0], 100.0);
        assert_eq!(emitter.transform[3], [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_point_emitter_negative_position() {
        let emitter = FluidEmitter::point([-10.0, -20.0, -30.0], 25.0);
        assert_eq!(emitter.transform[3][0], -10.0);
        assert_eq!(emitter.transform[3][1], -20.0);
        assert_eq!(emitter.transform[3][2], -30.0);
    }

    // ================== Sphere Emitter Tests ==================

    #[test]
    fn test_sphere_emitter_creation() {
        let center = [1.0, 2.0, 3.0];
        let radius = 5.0;
        let rate = 200.0;
        let emitter = FluidEmitter::sphere(center, radius, rate);

        // Check center position
        assert_eq!(emitter.transform[3][0], 1.0);
        assert_eq!(emitter.transform[3][1], 2.0);
        assert_eq!(emitter.transform[3][2], 3.0);

        // Check rate
        assert_eq!(emitter.rate, 200.0);

        // Check shape
        match emitter.shape {
            EmitterShape::Sphere { radius: r } => assert_eq!(r, 5.0),
            _ => panic!("Expected Sphere shape"),
        }
    }

    #[test]
    fn test_sphere_emitter_small_radius() {
        let emitter = FluidEmitter::sphere([0.0, 0.0, 0.0], 0.001, 10.0);
        match emitter.shape {
            EmitterShape::Sphere { radius } => assert_eq!(radius, 0.001),
            _ => panic!("Expected Sphere shape"),
        }
    }

    #[test]
    fn test_sphere_emitter_large_radius() {
        let emitter = FluidEmitter::sphere([0.0, 0.0, 0.0], 1000.0, 1.0);
        match emitter.shape {
            EmitterShape::Sphere { radius } => assert_eq!(radius, 1000.0),
            _ => panic!("Expected Sphere shape"),
        }
    }

    // ================== Mesh Emitter Tests ==================

    #[test]
    fn test_mesh_emitter_creation() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let rate = 150.0;

        let emitter = FluidEmitter::from_mesh(vertices.clone(), normals.clone(), rate);

        assert_eq!(emitter.rate, 150.0);

        match &emitter.shape {
            EmitterShape::Mesh {
                vertices: v,
                normals: n,
            } => {
                assert_eq!(v.len(), 3);
                assert_eq!(n.len(), 3);
                assert_eq!(v[0], [0.0, 0.0, 0.0]);
                assert_eq!(n[0], [0.0, 0.0, 1.0]);
            }
            _ => panic!("Expected Mesh shape"),
        }
    }

    #[test]
    fn test_mesh_emitter_empty_mesh() {
        let emitter = FluidEmitter::from_mesh(vec![], vec![], 100.0);
        match &emitter.shape {
            EmitterShape::Mesh { vertices, normals } => {
                assert!(vertices.is_empty());
                assert!(normals.is_empty());
            }
            _ => panic!("Expected Mesh shape"),
        }
    }

    #[test]
    fn test_mesh_emitter_single_vertex() {
        let emitter = FluidEmitter::from_mesh(vec![[5.0, 5.0, 5.0]], vec![[0.0, 1.0, 0.0]], 50.0);
        match &emitter.shape {
            EmitterShape::Mesh { vertices, normals } => {
                assert_eq!(vertices.len(), 1);
                assert_eq!(normals.len(), 1);
            }
            _ => panic!("Expected Mesh shape"),
        }
    }

    // ================== Tick Tests ==================

    #[test]
    fn test_tick_disabled_emitter() {
        let mut emitter = FluidEmitter {
            enabled: false,
            ..Default::default()
        };

        let (positions, velocities, colors) = emitter.tick(1.0);

        assert!(positions.is_empty());
        assert!(velocities.is_empty());
        assert!(colors.is_empty());
    }

    #[test]
    fn test_tick_zero_dt() {
        let mut emitter = FluidEmitter {
            rate: 100.0,
            ..Default::default()
        };

        let (positions, velocities, colors) = emitter.tick(0.0);

        // No time passed, no particles should spawn
        assert!(positions.is_empty());
        assert!(velocities.is_empty());
        assert!(colors.is_empty());
    }

    #[test]
    fn test_tick_spawns_particles() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 100.0);

        // dt = 0.1 should spawn about 10 particles (100 * 0.1)
        let (positions, velocities, colors) = emitter.tick(0.1);

        assert_eq!(positions.len(), 10);
        assert_eq!(velocities.len(), 10);
        assert_eq!(colors.len(), 10);
    }

    #[test]
    fn test_tick_accumulates_fractional_particles() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 10.0);

        // First tick: 10 * 0.05 = 0.5 particles (0 spawned)
        let (positions1, _, _) = emitter.tick(0.05);
        assert_eq!(positions1.len(), 0);

        // Second tick: 0.5 + 0.5 = 1.0 particles (1 spawned)
        let (positions2, _, _) = emitter.tick(0.05);
        assert_eq!(positions2.len(), 1);
    }

    #[test]
    fn test_tick_particle_positions_at_origin() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 100.0);

        let (positions, _, _) = emitter.tick(0.1);

        // Point emitter at origin should spawn all particles at origin
        for pos in &positions {
            assert_eq!(pos[0], 0.0);
            assert_eq!(pos[1], 0.0);
            assert_eq!(pos[2], 0.0);
        }
    }

    #[test]
    fn test_tick_particle_positions_offset() {
        let mut emitter = FluidEmitter::point([10.0, 20.0, 30.0], 100.0);

        let (positions, _, _) = emitter.tick(0.1);

        // Point emitter should spawn particles at emitter position
        for pos in &positions {
            assert_eq!(pos[0], 10.0);
            assert_eq!(pos[1], 20.0);
            assert_eq!(pos[2], 30.0);
        }
    }

    #[test]
    fn test_tick_colors_match_emitter() {
        let mut emitter = FluidEmitter {
            color: [1.0, 0.0, 0.0, 1.0], // Red
            rate: 100.0,
            ..Default::default()
        };

        let (_, _, colors) = emitter.tick(0.1);

        for color in &colors {
            assert_eq!(*color, [1.0, 0.0, 0.0, 1.0]);
        }
    }

    #[test]
    fn test_tick_high_rate_emitter() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 10000.0);

        let (positions, velocities, colors) = emitter.tick(0.1);

        // 10000 * 0.1 = 1000 particles
        assert_eq!(positions.len(), 1000);
        assert_eq!(velocities.len(), 1000);
        assert_eq!(colors.len(), 1000);
    }

    #[test]
    fn test_tick_low_rate_emitter() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 1.0);

        // dt = 0.5 should spawn 0 particles (1 * 0.5 = 0.5, truncated to 0)
        let (positions, _, _) = emitter.tick(0.5);
        assert_eq!(positions.len(), 0);

        // dt = 0.5 again should spawn 1 particle (accumulator = 0.5 + 0.5 = 1.0)
        let (positions, _, _) = emitter.tick(0.5);
        assert_eq!(positions.len(), 1);
    }

    // ================== Sphere Tick Tests ==================

    #[test]
    fn test_tick_sphere_emitter() {
        let mut emitter = FluidEmitter::sphere([0.0, 0.0, 0.0], 1.0, 100.0);

        let (positions, _, _) = emitter.tick(0.1);

        assert_eq!(positions.len(), 10);

        // All positions should be within radius from origin
        for pos in &positions {
            let dist = (pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt();
            // Allow some tolerance for the sphere radius
            assert!(
                dist <= 1.1,
                "Position {:?} is outside sphere (dist = {})",
                pos,
                dist
            );
        }
    }

    // ================== Box Emitter Tests ==================

    #[test]
    fn test_emitter_shape_box() {
        let mut emitter = FluidEmitter {
            shape: EmitterShape::Box {
                half_extents: [2.0, 3.0, 4.0],
            },
            rate: 100.0,
            ..Default::default()
        };

        let (positions, _, _) = emitter.tick(0.1);

        assert_eq!(positions.len(), 10);

        // All positions should be within the box bounds
        for pos in &positions {
            assert!(pos[0].abs() <= 2.0, "X position {} outside bounds", pos[0]);
            assert!(pos[1].abs() <= 3.0, "Y position {} outside bounds", pos[1]);
            assert!(pos[2].abs() <= 4.0, "Z position {} outside bounds", pos[2]);
        }
    }

    // ================== Mesh Tick Tests ==================

    #[test]
    fn test_tick_mesh_emitter_empty() {
        let mut emitter = FluidEmitter::from_mesh(vec![], vec![], 100.0);

        let (positions, _, _) = emitter.tick(0.1);

        // Even with empty mesh, should still spawn particles at origin
        assert_eq!(positions.len(), 10);
        for pos in &positions {
            assert_eq!(*pos, [0.0, 0.0, 0.0]);
        }
    }

    #[test]
    fn test_tick_mesh_emitter_with_vertices() {
        let vertices = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let normals = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

        let mut emitter = FluidEmitter::from_mesh(vertices.clone(), normals.clone(), 100.0);

        let (positions, _, _) = emitter.tick(0.1);

        assert_eq!(positions.len(), 10);

        // Each position should match one of the mesh vertices
        for pos in &positions {
            let matches = vertices.contains(pos);
            assert!(matches, "Position {:?} doesn't match any vertex", pos);
        }
    }

    // ================== FluidDrain Tests ==================

    #[test]
    fn test_fluid_drain_new() {
        let drain = FluidDrain::new([5.0, 10.0, 15.0], 2.5);

        assert_eq!(drain.position, [5.0, 10.0, 15.0]);
        assert_eq!(drain.radius, 2.5);
        assert!(drain.enabled);
    }

    #[test]
    fn test_fluid_drain_at_origin() {
        let drain = FluidDrain::new([0.0, 0.0, 0.0], 1.0);

        assert_eq!(drain.position, [0.0, 0.0, 0.0]);
        assert_eq!(drain.radius, 1.0);
    }

    #[test]
    fn test_fluid_drain_negative_position() {
        let drain = FluidDrain::new([-10.0, -20.0, -30.0], 5.0);

        assert_eq!(drain.position, [-10.0, -20.0, -30.0]);
    }

    #[test]
    fn test_fluid_drain_large_radius() {
        let drain = FluidDrain::new([0.0, 0.0, 0.0], 1000.0);
        assert_eq!(drain.radius, 1000.0);
    }

    #[test]
    fn test_fluid_drain_small_radius() {
        let drain = FluidDrain::new([0.0, 0.0, 0.0], 0.001);
        assert_eq!(drain.radius, 0.001);
    }

    #[test]
    fn test_fluid_drain_disable() {
        let mut drain = FluidDrain::new([0.0, 0.0, 0.0], 1.0);
        assert!(drain.enabled);

        drain.enabled = false;
        assert!(!drain.enabled);
    }

    // ================== EmitterShape Tests ==================

    #[test]
    fn test_emitter_shape_point_matches() {
        let shape = EmitterShape::Point;
        assert!(matches!(shape, EmitterShape::Point));
    }

    #[test]
    fn test_emitter_shape_sphere_matches() {
        let shape = EmitterShape::Sphere { radius: 5.0 };
        match shape {
            EmitterShape::Sphere { radius } => assert_eq!(radius, 5.0),
            _ => panic!("Expected Sphere"),
        }
    }

    #[test]
    fn test_emitter_shape_box_matches() {
        let shape = EmitterShape::Box {
            half_extents: [1.0, 2.0, 3.0],
        };
        match shape {
            EmitterShape::Box { half_extents } => {
                assert_eq!(half_extents, [1.0, 2.0, 3.0]);
            }
            _ => panic!("Expected Box"),
        }
    }

    #[test]
    fn test_emitter_shape_mesh_matches() {
        let shape = EmitterShape::Mesh {
            vertices: vec![[0.0, 0.0, 0.0]],
            normals: vec![[0.0, 1.0, 0.0]],
        };
        match shape {
            EmitterShape::Mesh { vertices, normals } => {
                assert_eq!(vertices.len(), 1);
                assert_eq!(normals.len(), 1);
            }
            _ => panic!("Expected Mesh"),
        }
    }

    // ================== Transform Tests ==================

    #[test]
    fn test_emitter_custom_transform() {
        let mut emitter = FluidEmitter {
            transform: [
                [2.0, 0.0, 0.0, 0.0],
                [0.0, 2.0, 0.0, 0.0],
                [0.0, 0.0, 2.0, 0.0],
                [100.0, 200.0, 300.0, 1.0],
            ],
            rate: 100.0,
            ..Default::default()
        };

        let (positions, _, _) = emitter.tick(0.1);

        // Point emitter uses origin from transform[3]
        for pos in &positions {
            assert_eq!(pos[0], 100.0);
            assert_eq!(pos[1], 200.0);
            assert_eq!(pos[2], 300.0);
        }
    }

    // ================== Velocity/Jitter Tests ==================

    #[test]
    fn test_emitter_custom_velocity() {
        let mut emitter = FluidEmitter {
            velocity: [10.0, 20.0, 30.0],
            jitter: 0.0, // Disable jitter for predictable test
            rate: 10.0,
            ..Default::default()
        };

        let (_, velocities, _) = emitter.tick(0.1);

        // Point emitter normal is [0, -1, 0], velocity is multiplied by normal
        // v[0] = 10.0 * 0.0 = 0.0
        // v[1] = 20.0 * -1.0 = -20.0
        // v[2] = 30.0 * 0.0 = 0.0
        for vel in &velocities {
            assert_eq!(vel[0], 0.0);
            assert_eq!(vel[1], -20.0);
            assert_eq!(vel[2], 0.0);
        }
    }

    #[test]
    fn test_emitter_with_jitter() {
        let mut emitter = FluidEmitter {
            jitter: 1.0, // High jitter
            rate: 100.0,
            ..Default::default()
        };

        let (_, velocities, _) = emitter.tick(0.1);

        // With jitter, velocities should vary
        let mut has_variation = false;
        if velocities.len() > 1 {
            for i in 1..velocities.len() {
                if velocities[i] != velocities[0] {
                    has_variation = true;
                    break;
                }
            }
        }

        assert!(
            has_variation,
            "Velocities should have variation with jitter"
        );
    }

    // ================== Edge Cases ==================

    #[test]
    fn test_emitter_very_small_dt() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 100.0);

        let (positions, _, _) = emitter.tick(0.0001);

        // 100 * 0.0001 = 0.01 particles, should spawn 0
        assert_eq!(positions.len(), 0);
    }

    #[test]
    fn test_emitter_very_large_dt() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 100.0);

        let (positions, _, _) = emitter.tick(10.0);

        // 100 * 10 = 1000 particles
        assert_eq!(positions.len(), 1000);
    }

    #[test]
    fn test_emitter_zero_rate() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 0.0);

        let (positions, _, _) = emitter.tick(1.0);

        assert!(positions.is_empty());
    }

    #[test]
    fn test_multiple_ticks_accumulation() {
        let mut emitter = FluidEmitter::point([0.0, 0.0, 0.0], 100.0);

        let mut total = 0;
        for _ in 0..100 {
            let (positions, _, _) = emitter.tick(0.001);
            total += positions.len();
        }

        // 100 ticks * 0.001 = 0.1 total time
        // 100 * 0.1 = 10 particles expected
        assert_eq!(total, 10);
    }

    // =========================================================================
    // Mutation-killing tests for jitter and velocity calculations
    // =========================================================================

    #[test]
    fn test_tick_jitter_and_velocity_exact() {
        // Non-zero jitter + custom velocity: verify exact jitter arithmetic
        let mut emitter = FluidEmitter {
            shape: EmitterShape::Point,
            velocity: [1.0, 2.0, 3.0],
            jitter: 0.5,
            rate: 20.0,
            enabled: true,
            ..Default::default()
        };

        // 20 * 0.1 = 2 particles
        let (_, velocities, _) = emitter.tick(0.1);
        assert!(
            velocities.len() >= 2,
            "need >= 2 particles, got {}",
            velocities.len()
        );

        // Reproduce the jitter formula for each particle.
        // Point emitter: sample_shape always returns pos=[0,0,0], normal=[0,-1,0].
        for (i, vel) in velocities.iter().enumerate() {
            let jitter_seed = (i as f32 * 12.9898).sin() * 43_758.547_f32;
            let jx = (jitter_seed.fract() - 0.5) * 0.5;
            let jy = ((jitter_seed * 2.0).fract() - 0.5) * 0.5;
            let jz = ((jitter_seed * 3.0).fract() - 0.5) * 0.5;

            // velocity * normal + jitter  (normal = [0, -1, 0])
            let expected_x = 1.0_f32 * 0.0 + jx;
            let expected_y = -2.0_f32 + jy; // velocity 2.0 * normal -1.0
            let expected_z = 3.0_f32 * 0.0 + jz;

            assert!(
                (vel[0] - expected_x).abs() < 1e-3,
                "particle {} vx: got {} expected {}",
                i,
                vel[0],
                expected_x
            );
            assert!(
                (vel[1] - expected_y).abs() < 1e-3,
                "particle {} vy: got {} expected {}",
                i,
                vel[1],
                expected_y
            );
            assert!(
                (vel[2] - expected_z).abs() < 1e-3,
                "particle {} vz: got {} expected {}",
                i,
                vel[2],
                expected_z
            );
        }
    }
}
