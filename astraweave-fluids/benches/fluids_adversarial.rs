//! Adversarial Fluids Benchmarks
//!
//! Stress testing for GPU SPH fluid simulation, particles, and grid operations.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box as std_black_box;

// ============================================================================
// LOCAL TYPES (Mirror astraweave-fluids API)
// ============================================================================

#[derive(Clone, Copy, Debug)]
struct Particle {
    position: [f32; 3],
    velocity: [f32; 3],
    force: [f32; 3],
    density: f32,
    pressure: f32,
    mass: f32,
}

impl Particle {
    fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            velocity: [0.0, 0.0, 0.0],
            force: [0.0, 0.0, 0.0],
            density: 1000.0, // Water density
            pressure: 0.0,
            mass: 0.02, // 20 grams
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SimParams {
    smoothing_radius: f32,
    rest_density: f32,
    stiffness: f32,
    viscosity: f32,
    gravity: [f32; 3],
    time_step: f32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            smoothing_radius: 0.1,
            rest_density: 1000.0,
            stiffness: 1000.0,
            viscosity: 0.01,
            gravity: [0.0, -9.81, 0.0],
            time_step: 0.001,
        }
    }
}

struct SpatialGrid {
    cells: HashMap<(i32, i32, i32), Vec<usize>>,
    cell_size: f32,
}

impl SpatialGrid {
    fn new(cell_size: f32) -> Self {
        Self {
            cells: HashMap::new(),
            cell_size,
        }
    }

    fn cell_coord(&self, pos: &[f32; 3]) -> (i32, i32, i32) {
        (
            (pos[0] / self.cell_size).floor() as i32,
            (pos[1] / self.cell_size).floor() as i32,
            (pos[2] / self.cell_size).floor() as i32,
        )
    }

    fn insert(&mut self, particle_idx: usize, pos: &[f32; 3]) {
        let cell = self.cell_coord(pos);
        self.cells.entry(cell).or_default().push(particle_idx);
    }

    fn query_neighbors(&self, pos: &[f32; 3]) -> Vec<usize> {
        let center = self.cell_coord(pos);
        let mut neighbors = Vec::new();

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let cell = (center.0 + dx, center.1 + dy, center.2 + dz);
                    if let Some(indices) = self.cells.get(&cell) {
                        neighbors.extend_from_slice(indices);
                    }
                }
            }
        }

        neighbors
    }

    fn clear(&mut self) {
        self.cells.clear();
    }
}

fn generate_particles(count: usize, bounds: [f32; 3]) -> Vec<Particle> {
    (0..count)
        .map(|i| {
            let x = (i % 100) as f32 * bounds[0] / 100.0;
            let y = ((i / 100) % 100) as f32 * bounds[1] / 100.0;
            let z = (i / 10000) as f32 * bounds[2] / 100.0;
            Particle::new([x, y, z])
        })
        .collect()
}

// SPH Kernel functions
fn poly6_kernel(r_sq: f32, h: f32) -> f32 {
    let h_sq = h * h;
    if r_sq > h_sq {
        return 0.0;
    }
    let diff = h_sq - r_sq;
    let coeff = 315.0 / (64.0 * std::f32::consts::PI * h.powi(9));
    coeff * diff.powi(3)
}

fn spiky_gradient_kernel(r: f32, h: f32) -> f32 {
    if r > h || r < 0.0001 {
        return 0.0;
    }
    let diff = h - r;
    let coeff = -45.0 / (std::f32::consts::PI * h.powi(6));
    coeff * diff.powi(2) / r
}

fn viscosity_laplacian_kernel(r: f32, h: f32) -> f32 {
    if r > h {
        return 0.0;
    }
    let coeff = 45.0 / (std::f32::consts::PI * h.powi(6));
    coeff * (h - r)
}

// ============================================================================
// CATEGORY 1: PARTICLE OPERATIONS
// ============================================================================

fn bench_particle_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/particle_operations");

    // Test 1: Particle creation
    for count in [1000, 5000, 10000] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("particle_creation", count),
            &count,
            |bencher, &count| {
                bencher.iter(|| {
                    let particles = generate_particles(count, [10.0, 10.0, 10.0]);
                    std_black_box(particles.len())
                });
            },
        );
    }

    // Test 2: Position update
    group.bench_function("position_update_10000", |bencher| {
        let mut particles = generate_particles(10000, [10.0, 10.0, 10.0]);
        let dt = 0.001f32;

        bencher.iter(|| {
            for p in &mut particles {
                p.position[0] += p.velocity[0] * dt;
                p.position[1] += p.velocity[1] * dt;
                p.position[2] += p.velocity[2] * dt;
            }
            std_black_box(particles.len())
        });
    });

    // Test 3: Velocity update
    group.bench_function("velocity_update_10000", |bencher| {
        let mut particles = generate_particles(10000, [10.0, 10.0, 10.0]);
        let dt = 0.001f32;

        bencher.iter(|| {
            for p in &mut particles {
                // F = ma, so a = F/m, v += a*dt
                let inv_mass = 1.0 / p.mass;
                p.velocity[0] += p.force[0] * inv_mass * dt;
                p.velocity[1] += p.force[1] * inv_mass * dt;
                p.velocity[2] += p.force[2] * inv_mass * dt;
            }
            std_black_box(particles.len())
        });
    });

    // Test 4: Force accumulation
    group.bench_function("force_accumulation_10000", |bencher| {
        let mut particles = generate_particles(10000, [10.0, 10.0, 10.0]);
        let gravity = [0.0f32, -9.81, 0.0];

        bencher.iter(|| {
            for p in &mut particles {
                // Reset forces and add gravity
                p.force = [
                    gravity[0] * p.mass,
                    gravity[1] * p.mass,
                    gravity[2] * p.mass,
                ];
            }
            std_black_box(particles.len())
        });
    });

    // Test 5: Boundary collision
    #[allow(clippy::needless_range_loop)]
    group.bench_function("boundary_collision_10000", |bencher| {
        let mut particles = generate_particles(10000, [10.0, 10.0, 10.0]);
        let bounds = [10.0f32, 10.0, 10.0];
        let damping = 0.5f32;

        bencher.iter(|| {
            for p in &mut particles {
                for i in 0..3 {
                    if p.position[i] < 0.0 {
                        p.position[i] = 0.0;
                        p.velocity[i] *= -damping;
                    } else if p.position[i] > bounds[i] {
                        p.position[i] = bounds[i];
                        p.velocity[i] *= -damping;
                    }
                }
            }
            std_black_box(particles.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 2: SPATIAL HASHING
// ============================================================================

fn bench_spatial_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/spatial_hashing");

    // Test 1: Grid construction
    for count in [1000, 5000, 10000] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("grid_construction", count),
            &count,
            |bencher, &count| {
                let particles = generate_particles(count, [10.0, 10.0, 10.0]);

                bencher.iter(|| {
                    let mut grid = SpatialGrid::new(0.1);
                    for (i, p) in particles.iter().enumerate() {
                        grid.insert(i, &p.position);
                    }
                    std_black_box(grid.cells.len())
                });
            },
        );
    }

    // Test 2: Neighbor queries
    group.bench_function("neighbor_query_10000", |bencher| {
        let particles = generate_particles(10000, [10.0, 10.0, 10.0]);
        let mut grid = SpatialGrid::new(0.1);
        for (i, p) in particles.iter().enumerate() {
            grid.insert(i, &p.position);
        }

        bencher.iter(|| {
            let mut total_neighbors = 0;
            for p in &particles {
                let neighbors = grid.query_neighbors(&p.position);
                total_neighbors += neighbors.len();
            }
            std_black_box(total_neighbors)
        });
    });

    // Test 3: Grid rebuild
    group.bench_function("grid_rebuild_5000", |bencher| {
        let mut particles = generate_particles(5000, [10.0, 10.0, 10.0]);
        let mut grid = SpatialGrid::new(0.1);

        bencher.iter(|| {
            // Simulate particle movement
            for p in &mut particles {
                p.position[0] += 0.01;
                p.position[1] -= 0.005;
            }

            // Rebuild grid
            grid.clear();
            for (i, p) in particles.iter().enumerate() {
                grid.insert(i, &p.position);
            }

            std_black_box(grid.cells.len())
        });
    });

    // Test 4: Cell density distribution
    group.bench_function("cell_density_analysis", |bencher| {
        let particles = generate_particles(10000, [10.0, 10.0, 10.0]);
        let mut grid = SpatialGrid::new(0.1);
        for (i, p) in particles.iter().enumerate() {
            grid.insert(i, &p.position);
        }

        bencher.iter(|| {
            let densities: Vec<usize> = grid.cells.values().map(|v| v.len()).collect();
            let max_density = densities.iter().max().copied().unwrap_or(0);
            let avg_density = densities.iter().sum::<usize>() as f32 / densities.len() as f32;
            std_black_box((max_density, avg_density as usize))
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 3: SPH KERNELS
// ============================================================================

fn bench_sph_kernels(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/sph_kernels");

    // Test 1: Poly6 kernel evaluation
    group.bench_function("poly6_kernel_100000", |bencher| {
        let distances: Vec<f32> = (0..100000).map(|i| i as f32 * 0.001).collect();
        let h = 0.1f32;

        bencher.iter(|| {
            let results: Vec<f32> = distances.iter().map(|&r| poly6_kernel(r * r, h)).collect();
            std_black_box(results.iter().sum::<f32>())
        });
    });

    // Test 2: Spiky gradient kernel
    group.bench_function("spiky_gradient_100000", |bencher| {
        let distances: Vec<f32> = (0..100000).map(|i| i as f32 * 0.001).collect();
        let h = 0.1f32;

        bencher.iter(|| {
            let results: Vec<f32> = distances
                .iter()
                .map(|&r| spiky_gradient_kernel(r, h))
                .collect();
            std_black_box(results.iter().sum::<f32>())
        });
    });

    // Test 3: Viscosity laplacian kernel
    group.bench_function("viscosity_laplacian_100000", |bencher| {
        let distances: Vec<f32> = (0..100000).map(|i| i as f32 * 0.001).collect();
        let h = 0.1f32;

        bencher.iter(|| {
            let results: Vec<f32> = distances
                .iter()
                .map(|&r| viscosity_laplacian_kernel(r, h))
                .collect();
            std_black_box(results.iter().sum::<f32>())
        });
    });

    // Test 4: Combined kernel evaluation
    group.bench_function("combined_kernels_10000", |bencher| {
        let distances: Vec<f32> = (0..10000).map(|i| i as f32 * 0.01).collect();
        let h = 0.1f32;

        bencher.iter(|| {
            let results: Vec<(f32, f32, f32)> = distances
                .iter()
                .map(|&r| {
                    (
                        poly6_kernel(r * r, h),
                        spiky_gradient_kernel(r, h),
                        viscosity_laplacian_kernel(r, h),
                    )
                })
                .collect();

            let sums: (f32, f32, f32) = results.iter().fold((0.0, 0.0, 0.0), |acc, &(a, b, c)| {
                (acc.0 + a, acc.1 + b, acc.2 + c)
            });

            std_black_box(sums)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 4: DENSITY/PRESSURE CALCULATION
// ============================================================================

fn bench_density_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/density_pressure");

    // Test 1: Density computation
    group.bench_function("density_computation_5000", |bencher| {
        let mut particles = generate_particles(5000, [5.0, 5.0, 5.0]);
        let mut grid = SpatialGrid::new(0.1);
        for (i, p) in particles.iter().enumerate() {
            grid.insert(i, &p.position);
        }

        let params = SimParams::default();

        bencher.iter(|| {
            for i in 0..particles.len() {
                let neighbors = grid.query_neighbors(&particles[i].position);
                let mut density = 0.0f32;

                for &j in &neighbors {
                    let dx = particles[i].position[0] - particles[j].position[0];
                    let dy = particles[i].position[1] - particles[j].position[1];
                    let dz = particles[i].position[2] - particles[j].position[2];
                    let r_sq = dx * dx + dy * dy + dz * dz;

                    density += particles[j].mass * poly6_kernel(r_sq, params.smoothing_radius);
                }

                particles[i].density = density.max(0.001);
            }

            std_black_box(particles.iter().map(|p| p.density).sum::<f32>())
        });
    });

    // Test 2: Pressure computation
    group.bench_function("pressure_computation_5000", |bencher| {
        let mut particles = generate_particles(5000, [5.0, 5.0, 5.0]);
        let params = SimParams::default();

        // Set densities
        for p in &mut particles {
            p.density = params.rest_density * (0.8 + rand_simple(400) as f32 / 1000.0);
        }

        bencher.iter(|| {
            for p in &mut particles {
                // Tait equation of state
                p.pressure = params.stiffness * ((p.density / params.rest_density).powi(7) - 1.0);
            }

            std_black_box(particles.iter().map(|p| p.pressure).sum::<f32>())
        });
    });

    // Test 3: Pressure force computation
    group.bench_function("pressure_force_2000", |bencher| {
        let mut particles = generate_particles(2000, [3.0, 3.0, 3.0]);
        let mut grid = SpatialGrid::new(0.1);
        for (i, p) in particles.iter().enumerate() {
            grid.insert(i, &p.position);
        }

        let params = SimParams::default();

        // Initialize densities and pressures
        for p in &mut particles {
            p.density = params.rest_density;
            p.pressure = params.stiffness * 0.1;
        }

        bencher.iter(|| {
            let mut forces: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; particles.len()];

            for i in 0..particles.len() {
                let neighbors = grid.query_neighbors(&particles[i].position);

                for &j in &neighbors {
                    if i == j {
                        continue;
                    }

                    let dx = particles[i].position[0] - particles[j].position[0];
                    let dy = particles[i].position[1] - particles[j].position[1];
                    let dz = particles[i].position[2] - particles[j].position[2];
                    let r = (dx * dx + dy * dy + dz * dz).sqrt();

                    if r < 0.0001 {
                        continue;
                    }

                    let kernel = spiky_gradient_kernel(r, params.smoothing_radius);
                    let pressure_term = (particles[i].pressure + particles[j].pressure)
                        / (2.0 * particles[j].density);
                    let force_mag = -particles[j].mass * pressure_term * kernel;

                    forces[i][0] += force_mag * dx / r;
                    forces[i][1] += force_mag * dy / r;
                    forces[i][2] += force_mag * dz / r;
                }
            }

            std_black_box(forces.len())
        });
    });

    // Test 4: Viscosity force computation
    group.bench_function("viscosity_force_2000", |bencher| {
        let mut particles = generate_particles(2000, [3.0, 3.0, 3.0]);
        let mut grid = SpatialGrid::new(0.1);
        for (i, p) in particles.iter().enumerate() {
            grid.insert(i, &p.position);
        }

        let params = SimParams::default();

        // Initialize velocities
        for (i, p) in particles.iter_mut().enumerate() {
            p.velocity = [(i as f32 * 0.001).sin(), 0.0, 0.0];
            p.density = params.rest_density;
        }

        bencher.iter(|| {
            let mut forces: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; particles.len()];

            for i in 0..particles.len() {
                let neighbors = grid.query_neighbors(&particles[i].position);

                for &j in &neighbors {
                    if i == j {
                        continue;
                    }

                    let dx = particles[i].position[0] - particles[j].position[0];
                    let dy = particles[i].position[1] - particles[j].position[1];
                    let dz = particles[i].position[2] - particles[j].position[2];
                    let r = (dx * dx + dy * dy + dz * dz).sqrt();

                    let kernel = viscosity_laplacian_kernel(r, params.smoothing_radius);
                    let vel_diff = [
                        particles[j].velocity[0] - particles[i].velocity[0],
                        particles[j].velocity[1] - particles[i].velocity[1],
                        particles[j].velocity[2] - particles[i].velocity[2],
                    ];

                    let force_factor =
                        params.viscosity * particles[j].mass * kernel / particles[j].density;

                    forces[i][0] += force_factor * vel_diff[0];
                    forces[i][1] += force_factor * vel_diff[1];
                    forces[i][2] += force_factor * vel_diff[2];
                }
            }

            std_black_box(forces.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 5: FULL SIMULATION STEP
// ============================================================================

fn bench_simulation_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/simulation_step");

    // Test 1: Single step small system
    group.bench_function("single_step_1000", |bencher| {
        let mut particles = generate_particles(1000, [2.0, 2.0, 2.0]);
        let params = SimParams::default();

        bencher.iter(|| {
            // 1. Build spatial grid
            let mut grid = SpatialGrid::new(params.smoothing_radius);
            for (i, p) in particles.iter().enumerate() {
                grid.insert(i, &p.position);
            }

            // 2. Compute density
            for i in 0..particles.len() {
                let neighbors = grid.query_neighbors(&particles[i].position);
                let mut density = 0.0f32;

                for &j in &neighbors {
                    let dx = particles[i].position[0] - particles[j].position[0];
                    let dy = particles[i].position[1] - particles[j].position[1];
                    let dz = particles[i].position[2] - particles[j].position[2];
                    let r_sq = dx * dx + dy * dy + dz * dz;
                    density += particles[j].mass * poly6_kernel(r_sq, params.smoothing_radius);
                }

                particles[i].density = density.max(0.001);
            }

            // 3. Compute pressure
            for p in &mut particles {
                p.pressure = params.stiffness * ((p.density / params.rest_density).powi(7) - 1.0);
            }

            // 4. Reset forces + gravity
            for p in &mut particles {
                p.force = [
                    params.gravity[0] * p.mass,
                    params.gravity[1] * p.mass,
                    params.gravity[2] * p.mass,
                ];
            }

            // 5. Integration
            for p in &mut particles {
                let inv_mass = 1.0 / p.mass;
                p.velocity[0] += p.force[0] * inv_mass * params.time_step;
                p.velocity[1] += p.force[1] * inv_mass * params.time_step;
                p.velocity[2] += p.force[2] * inv_mass * params.time_step;

                p.position[0] += p.velocity[0] * params.time_step;
                p.position[1] += p.velocity[1] * params.time_step;
                p.position[2] += p.velocity[2] * params.time_step;
            }

            std_black_box(particles.len())
        });
    });

    // Test 2: Multiple steps
    group.bench_function("multi_step_500_x10", |bencher| {
        let mut particles = generate_particles(500, [2.0, 2.0, 2.0]);
        let params = SimParams::default();

        bencher.iter(|| {
            for _ in 0..10 {
                // Build grid
                let mut grid = SpatialGrid::new(params.smoothing_radius);
                for (i, p) in particles.iter().enumerate() {
                    grid.insert(i, &p.position);
                }

                // Update velocities with simple Euler
                for p in &mut particles {
                    p.velocity[1] += params.gravity[1] * params.time_step;
                }

                // Update positions
                for p in &mut particles {
                    p.position[0] += p.velocity[0] * params.time_step;
                    p.position[1] += p.velocity[1] * params.time_step;
                    p.position[2] += p.velocity[2] * params.time_step;

                    // Simple boundary
                    if p.position[1] < 0.0 {
                        p.position[1] = 0.0;
                        p.velocity[1] *= -0.5;
                    }
                }
            }

            std_black_box(particles.len())
        });
    });

    // Test 3: Adaptive time stepping
    group.bench_function("adaptive_timestep_1000", |bencher| {
        let particles = generate_particles(1000, [2.0, 2.0, 2.0]);
        let params = SimParams::default();

        bencher.iter(|| {
            // Find max velocity
            let max_vel = particles
                .iter()
                .map(|p| {
                    (p.velocity[0].powi(2) + p.velocity[1].powi(2) + p.velocity[2].powi(2)).sqrt()
                })
                .fold(0.0f32, f32::max);

            // CFL condition: dt = C * h / v_max
            let cfl = 0.4f32;
            let adaptive_dt = if max_vel > 0.0001 {
                (cfl * params.smoothing_radius / max_vel).min(params.time_step)
            } else {
                params.time_step
            };

            std_black_box(adaptive_dt)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 6: GPU DATA PREPARATION
// ============================================================================

fn bench_gpu_data_prep(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/gpu_data_prep");

    // Test 1: Position buffer packing
    group.bench_function("position_buffer_10000", |bencher| {
        let particles = generate_particles(10000, [10.0, 10.0, 10.0]);

        bencher.iter(|| {
            // Pack positions into contiguous buffer (vec4 aligned)
            let buffer: Vec<[f32; 4]> = particles
                .iter()
                .map(|p| [p.position[0], p.position[1], p.position[2], 1.0])
                .collect();

            std_black_box(buffer.len())
        });
    });

    // Test 2: Velocity buffer packing
    group.bench_function("velocity_buffer_10000", |bencher| {
        let particles = generate_particles(10000, [10.0, 10.0, 10.0]);

        bencher.iter(|| {
            let buffer: Vec<[f32; 4]> = particles
                .iter()
                .map(|p| [p.velocity[0], p.velocity[1], p.velocity[2], 0.0])
                .collect();

            std_black_box(buffer.len())
        });
    });

    // Test 3: Combined particle data packing
    group.bench_function("combined_buffer_10000", |bencher| {
        let particles = generate_particles(10000, [10.0, 10.0, 10.0]);

        bencher.iter(|| {
            // Pack all particle data into single buffer
            // Format: pos.xyz, density, vel.xyz, pressure
            let buffer: Vec<[f32; 8]> = particles
                .iter()
                .map(|p| {
                    [
                        p.position[0],
                        p.position[1],
                        p.position[2],
                        p.density,
                        p.velocity[0],
                        p.velocity[1],
                        p.velocity[2],
                        p.pressure,
                    ]
                })
                .collect();

            std_black_box(buffer.len())
        });
    });

    // Test 4: Cell index buffer generation
    group.bench_function("cell_index_buffer_10000", |bencher| {
        let particles = generate_particles(10000, [10.0, 10.0, 10.0]);
        let cell_size = 0.1f32;

        bencher.iter(|| {
            let cell_indices: Vec<u32> = particles
                .iter()
                .map(|p| {
                    let cx = (p.position[0] / cell_size).floor() as u32;
                    let cy = (p.position[1] / cell_size).floor() as u32;
                    let cz = (p.position[2] / cell_size).floor() as u32;

                    // Morton code for spatial coherence
                    (cx & 0x3FF) | ((cy & 0x3FF) << 10) | ((cz & 0x3FF) << 20)
                })
                .collect();

            std_black_box(cell_indices.len())
        });
    });

    // Test 5: Uniform buffer preparation
    group.bench_function("uniform_buffer_prep", |bencher| {
        let params = SimParams::default();

        bencher.iter(|| {
            // Pack simulation parameters for GPU
            let uniform_data: [f32; 16] = [
                params.smoothing_radius,
                params.rest_density,
                params.stiffness,
                params.viscosity,
                params.gravity[0],
                params.gravity[1],
                params.gravity[2],
                params.time_step,
                0.0,
                0.0,
                0.0,
                0.0, // Padding
                0.0,
                0.0,
                0.0,
                0.0, // More padding for alignment
            ];

            std_black_box(uniform_data)
        });
    });

    // Test 6: Sort preparation for GPU
    group.bench_function("sort_prep_10000", |bencher| {
        let particles = generate_particles(10000, [10.0, 10.0, 10.0]);
        let cell_size = 0.1f32;

        bencher.iter(|| {
            // Create (cell_index, particle_index) pairs for sorting
            let mut sort_keys: Vec<(u32, u32)> = particles
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let cx = (p.position[0] / cell_size).floor() as u32;
                    let cy = (p.position[1] / cell_size).floor() as u32;
                    let cz = (p.position[2] / cell_size).floor() as u32;
                    let cell = (cx & 0x3FF) | ((cy & 0x3FF) << 10) | ((cz & 0x3FF) << 20);
                    (cell, i as u32)
                })
                .collect();

            sort_keys.sort_by_key(|(cell, _)| *cell);

            std_black_box(sort_keys.len())
        });
    });

    group.finish();
}

// Simple deterministic random
fn rand_simple(max: u64) -> u64 {
    static mut SEED: u64 = 12345;
    unsafe {
        SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
        SEED % max
    }
}

// ============================================================================
// CATEGORY 7: OPTIMIZATION CONTROLLER
// ============================================================================

/// Mock FluidOptimizationController for CPU-side benchmarks
#[allow(dead_code)]
struct MockOptimizationController {
    quality_tier: u32,
    target_frame_time_ms: f32,
    frame_times: Vec<f32>,
    auto_tune: bool,
    lod_enabled: bool,
    adaptive_min: u32,
    adaptive_max: u32,
    adaptive_current: u32,
}

impl MockOptimizationController {
    fn new() -> Self {
        Self {
            quality_tier: 1, // High
            target_frame_time_ms: 16.67,
            frame_times: Vec::with_capacity(60),
            auto_tune: true,
            lod_enabled: false,
            adaptive_min: 2,
            adaptive_max: 8,
            adaptive_current: 4,
        }
    }

    fn record_frame(&mut self, frame_time_ms: f32) {
        if self.frame_times.len() >= 60 {
            self.frame_times.remove(0);
        }
        self.frame_times.push(frame_time_ms);

        if self.auto_tune {
            self.auto_adjust_quality();
        }
    }

    fn auto_adjust_quality(&mut self) {
        if self.frame_times.len() < 10 {
            return;
        }

        let avg = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;

        if avg > self.target_frame_time_ms * 1.2 && self.quality_tier < 4 {
            self.quality_tier += 1;
            self.adaptive_current = (self.adaptive_current - 1).max(self.adaptive_min);
        } else if avg < self.target_frame_time_ms * 0.7 && self.quality_tier > 0 {
            self.quality_tier -= 1;
            self.adaptive_current = (self.adaptive_current + 1).min(self.adaptive_max);
        }
    }

    fn is_within_budget(&self) -> bool {
        if self.frame_times.is_empty() {
            return true;
        }
        let avg = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        avg <= self.target_frame_time_ms
    }

    fn budget_headroom(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 100.0;
        }
        let avg = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        ((self.target_frame_time_ms - avg) / self.target_frame_time_ms) * 100.0
    }

    fn recommended_iterations(&self) -> u32 {
        match self.quality_tier {
            0 => 8,
            1 => 6,
            2 => 4,
            3 => 2,
            _ => 1,
        }
    }

    fn configure_for_gpu(&mut self, gpu_name: &str) {
        let name_lower = gpu_name.to_lowercase();
        if name_lower.contains("rtx 40") || name_lower.contains("rtx 30") {
            self.quality_tier = 0;
            self.adaptive_current = 8;
        } else if name_lower.contains("rtx") || name_lower.contains("radeon rx 7") {
            self.quality_tier = 1;
            self.adaptive_current = 6;
        } else if name_lower.contains("arc") || name_lower.contains("radeon rx 6") {
            self.quality_tier = 2;
            self.adaptive_current = 4;
        } else {
            self.quality_tier = 3;
            self.adaptive_current = 2;
        }
    }
}

fn bench_optimization_controller(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/optimization_controller");

    // Test 1: Controller creation
    group.bench_function("controller_creation", |bencher| {
        bencher.iter(|| {
            let controller = MockOptimizationController::new();
            std_black_box(controller.quality_tier)
        });
    });

    // Test 2: Frame recording throughput
    group.bench_function("frame_recording_1000", |bencher| {
        let mut controller = MockOptimizationController::new();

        bencher.iter(|| {
            for i in 0..1000 {
                let frame_time = 14.0 + (i % 10) as f32 * 0.5;
                controller.record_frame(frame_time);
            }
            std_black_box(controller.quality_tier)
        });
    });

    // Test 3: Auto-tune decision making
    group.bench_function("auto_tune_decision_10000", |bencher| {
        let mut controller = MockOptimizationController::new();

        // Pre-populate with frame times
        for i in 0..30 {
            controller.record_frame(12.0 + (i % 5) as f32);
        }

        bencher.iter(|| {
            for _ in 0..10000 {
                controller.auto_adjust_quality();
            }
            std_black_box(controller.quality_tier)
        });
    });

    // Test 4: Budget check performance
    group.bench_function("budget_check_10000", |bencher| {
        let mut controller = MockOptimizationController::new();
        for i in 0..60 {
            controller.record_frame(10.0 + (i % 10) as f32);
        }

        bencher.iter(|| {
            let mut within = 0;
            for _ in 0..10000 {
                if controller.is_within_budget() {
                    within += 1;
                }
            }
            std_black_box(within)
        });
    });

    // Test 5: Headroom calculation
    group.bench_function("headroom_calculation_10000", |bencher| {
        let mut controller = MockOptimizationController::new();
        for i in 0..60 {
            controller.record_frame(10.0 + (i % 10) as f32);
        }

        bencher.iter(|| {
            let mut total_headroom = 0.0f32;
            for _ in 0..10000 {
                total_headroom += controller.budget_headroom();
            }
            std_black_box(total_headroom)
        });
    });

    // Test 6: GPU configuration parsing
    group.bench_function("gpu_config_parsing_1000", |bencher| {
        let gpu_names = [
            "NVIDIA GeForce RTX 4090",
            "NVIDIA GeForce RTX 3080",
            "AMD Radeon RX 7900 XTX",
            "AMD Radeon RX 6800 XT",
            "Intel Arc A770",
            "Intel UHD Graphics 770",
        ];

        bencher.iter(|| {
            let mut controller = MockOptimizationController::new();
            for _ in 0..1000 {
                for name in &gpu_names {
                    controller.configure_for_gpu(name);
                }
            }
            std_black_box(controller.quality_tier)
        });
    });

    // Test 7: Iteration recommendation
    group.bench_function("iteration_recommendation_10000", |bencher| {
        let mut controller = MockOptimizationController::new();

        bencher.iter(|| {
            let mut total = 0u32;
            for tier in 0..=4 {
                controller.quality_tier = tier;
                for _ in 0..2000 {
                    total += controller.recommended_iterations();
                }
            }
            std_black_box(total)
        });
    });

    // Test 8: Full frame cycle (record + adjust + check)
    group.throughput(Throughput::Elements(1000));
    group.bench_function("full_frame_cycle_1000", |bencher| {
        let mut controller = MockOptimizationController::new();

        bencher.iter(|| {
            for i in 0..1000 {
                // Simulate variable frame times
                let frame_time = 12.0 + ((i * 7) % 20) as f32 * 0.5;
                controller.record_frame(frame_time);
                let _within = controller.is_within_budget();
                let _headroom = controller.budget_headroom();
                let _iters = controller.recommended_iterations();
            }
            std_black_box(controller.quality_tier)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 8: ADAPTIVE ITERATIONS
// ============================================================================

struct MockAdaptiveIterations {
    min: u32,
    max: u32,
    current: u32,
    density_errors: Vec<f32>,
    target_error: f32,
}

impl MockAdaptiveIterations {
    fn new(min: u32, max: u32) -> Self {
        Self {
            min,
            max,
            current: (min + max) / 2,
            density_errors: Vec::with_capacity(30),
            target_error: 0.01,
        }
    }

    fn update(&mut self, density_error: f32) {
        if self.density_errors.len() >= 30 {
            self.density_errors.remove(0);
        }
        self.density_errors.push(density_error);

        if self.density_errors.len() >= 5 {
            let avg_error =
                self.density_errors.iter().sum::<f32>() / self.density_errors.len() as f32;

            if avg_error > self.target_error * 1.5 && self.current < self.max {
                self.current += 1;
            } else if avg_error < self.target_error * 0.5 && self.current > self.min {
                self.current -= 1;
            }
        }
    }
}

fn bench_adaptive_iterations(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/adaptive_iterations");

    // Test 1: Adaptive iteration update
    group.bench_function("iteration_update_10000", |bencher| {
        let mut adaptive = MockAdaptiveIterations::new(2, 8);

        bencher.iter(|| {
            for i in 0..10000 {
                let error = 0.005 + (i % 100) as f32 * 0.0002;
                adaptive.update(error);
            }
            std_black_box(adaptive.current)
        });
    });

    // Test 2: Error tracking overhead
    group.bench_function("error_tracking_1000", |bencher| {
        let mut adaptive = MockAdaptiveIterations::new(1, 16);

        bencher.iter(|| {
            for i in 0..1000 {
                let error = 0.001 + (i % 50) as f32 * 0.001;
                if adaptive.density_errors.len() >= 30 {
                    adaptive.density_errors.remove(0);
                }
                adaptive.density_errors.push(error);
            }
            std_black_box(adaptive.density_errors.len())
        });
    });

    // Test 3: Decision boundary stress test
    group.bench_function("decision_boundary_stress", |bencher| {
        let mut adaptive = MockAdaptiveIterations::new(2, 8);

        // Fill with values near decision boundary
        for _ in 0..30 {
            adaptive.density_errors.push(0.015);
        }

        bencher.iter(|| {
            for _ in 0..10000 {
                // Oscillate around boundary
                adaptive.density_errors[0] = 0.014;
                adaptive.update(0.016);
                adaptive.density_errors[0] = 0.016;
                adaptive.update(0.014);
            }
            std_black_box(adaptive.current)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 9: LOD SYSTEM
// ============================================================================

#[allow(dead_code)]
struct MockLodManager {
    current_lod: u32,
    distances: [f32; 5],
    last_camera_pos: [f32; 3],
    last_fluid_center: [f32; 3],
}

#[allow(dead_code)]
impl MockLodManager {
    fn new() -> Self {
        Self {
            current_lod: 0,
            distances: [10.0, 25.0, 50.0, 100.0, 200.0],
            last_camera_pos: [0.0; 3],
            last_fluid_center: [0.0; 3],
        }
    }

    fn update(&mut self, camera_pos: [f32; 3], fluid_center: [f32; 3]) -> bool {
        self.last_camera_pos = camera_pos;
        self.last_fluid_center = fluid_center;

        let dx = camera_pos[0] - fluid_center[0];
        let dy = camera_pos[1] - fluid_center[1];
        let dz = camera_pos[2] - fluid_center[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();

        let new_lod = self.distances.iter().position(|&d| dist < d).unwrap_or(5) as u32;

        let changed = new_lod != self.current_lod;
        self.current_lod = new_lod;
        changed
    }

    fn should_simulate(&self) -> bool {
        self.current_lod < 4
    }

    fn particle_reduction_factor(&self) -> f32 {
        match self.current_lod {
            0 => 1.0,
            1 => 0.75,
            2 => 0.5,
            3 => 0.25,
            _ => 0.1,
        }
    }
}

fn bench_lod_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/lod_system");

    // Test 1: LOD update
    group.bench_function("lod_update_10000", |bencher| {
        let mut lod = MockLodManager::new();

        bencher.iter(|| {
            for i in 0..10000 {
                let dist = (i % 250) as f32;
                let camera = [dist, 5.0, 0.0];
                lod.update(camera, [0.0, 5.0, 0.0]);
            }
            std_black_box(lod.current_lod)
        });
    });

    // Test 2: Distance calculation overhead
    group.bench_function("distance_calculation_100000", |bencher| {
        bencher.iter(|| {
            let mut total_dist = 0.0f32;
            for i in 0..100000 {
                let x = (i % 100) as f32;
                let y = ((i / 100) % 100) as f32;
                let z = (i / 10000) as f32;
                let dist = (x * x + y * y + z * z).sqrt();
                total_dist += dist;
            }
            std_black_box(total_dist)
        });
    });

    // Test 3: LOD transition frequency
    group.bench_function("lod_transition_tracking", |bencher| {
        let mut lod = MockLodManager::new();

        bencher.iter(|| {
            let mut transitions = 0u32;
            for i in 0..10000 {
                // Simulate camera moving back and forth
                let dist = ((i as f32 * 0.1).sin() + 1.0) * 100.0;
                let camera = [dist, 5.0, 0.0];
                if lod.update(camera, [0.0, 5.0, 0.0]) {
                    transitions += 1;
                }
            }
            std_black_box(transitions)
        });
    });

    // Test 4: Particle reduction factor lookup
    group.bench_function("reduction_factor_lookup_10000", |bencher| {
        let mut lod = MockLodManager::new();

        bencher.iter(|| {
            let mut total_factor = 0.0f32;
            for i in 0..10000 {
                lod.current_lod = (i % 5) as u32;
                total_factor += lod.particle_reduction_factor();
            }
            std_black_box(total_factor)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 10: SIMD OPTIMIZATIONS
// ============================================================================

fn bench_simd_optimizations(c: &mut Criterion) {
    let mut group = c.benchmark_group("fluids_adversarial/simd_optimizations");

    // Test 1: Density accumulation 4x vs 8x unroll
    group.bench_function("density_4x_unroll_10000", |bencher| {
        let kernels: Vec<f32> = (0..10000)
            .map(|i| (i as f32 * 0.0001).sin().abs() * 1000.0)
            .collect();
        let masses: Vec<f32> = vec![0.02; 10000];

        bencher.iter(|| {
            let len = kernels.len();
            let chunks = len / 4;

            let (mut acc0, mut acc1, mut acc2, mut acc3) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for chunk in 0..chunks {
                let base = chunk * 4;
                acc0 += kernels[base] * masses[base];
                acc1 += kernels[base + 1] * masses[base + 1];
                acc2 += kernels[base + 2] * masses[base + 2];
                acc3 += kernels[base + 3] * masses[base + 3];
            }
            let mut density = acc0 + acc1 + acc2 + acc3;
            for i in (chunks * 4)..len {
                density += kernels[i] * masses[i];
            }
            std_black_box(density)
        });
    });

    group.bench_function("density_8x_unroll_10000", |bencher| {
        let kernels: Vec<f32> = (0..10000)
            .map(|i| (i as f32 * 0.0001).sin().abs() * 1000.0)
            .collect();
        let masses: Vec<f32> = vec![0.02; 10000];

        bencher.iter(|| {
            let len = kernels.len();
            let chunks = len / 8;

            let (mut acc0, mut acc1, mut acc2, mut acc3) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for chunk in 0..chunks {
                let base = chunk * 8;
                // First 4
                acc0 = kernels[base].mul_add(masses[base], acc0);
                acc1 = kernels[base + 1].mul_add(masses[base + 1], acc1);
                acc2 = kernels[base + 2].mul_add(masses[base + 2], acc2);
                acc3 = kernels[base + 3].mul_add(masses[base + 3], acc3);
                // Second 4
                acc0 = kernels[base + 4].mul_add(masses[base + 4], acc0);
                acc1 = kernels[base + 5].mul_add(masses[base + 5], acc1);
                acc2 = kernels[base + 6].mul_add(masses[base + 6], acc2);
                acc3 = kernels[base + 7].mul_add(masses[base + 7], acc3);
            }
            let mut density = acc0 + acc1 + acc2 + acc3;
            for i in (chunks * 8)..len {
                density = kernels[i].mul_add(masses[i], density);
            }
            std_black_box(density)
        });
    });

    // Test 2: Distance squared vs distance (sqrt avoidance)
    group.bench_function("distance_with_sqrt_100000", |bencher| {
        let positions: Vec<[f32; 3]> = (0..100000)
            .map(|i| {
                [
                    (i % 100) as f32 * 0.1,
                    ((i / 100) % 100) as f32 * 0.1,
                    (i / 10000) as f32 * 0.1,
                ]
            })
            .collect();
        let center = [5.0, 5.0, 0.5];

        bencher.iter(|| {
            let mut total = 0.0f32;
            for p in &positions {
                let dx = p[0] - center[0];
                let dy = p[1] - center[1];
                let dz = p[2] - center[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                total += dist;
            }
            std_black_box(total)
        });
    });

    group.bench_function("distance_squared_only_100000", |bencher| {
        let positions: Vec<[f32; 3]> = (0..100000)
            .map(|i| {
                [
                    (i % 100) as f32 * 0.1,
                    ((i / 100) % 100) as f32 * 0.1,
                    (i / 10000) as f32 * 0.1,
                ]
            })
            .collect();
        let center = [5.0, 5.0, 0.5];

        bencher.iter(|| {
            let mut total = 0.0f32;
            for p in &positions {
                let dx = p[0] - center[0];
                let dy = p[1] - center[1];
                let dz = p[2] - center[2];
                // FMA pattern
                let dist_sq = dx.mul_add(dx, dy.mul_add(dy, dz * dz));
                total += dist_sq;
            }
            std_black_box(total)
        });
    });

    // Test 3: Morton code generation
    group.bench_function("morton_code_generation_100000", |bencher| {
        let positions: Vec<(u32, u32, u32)> = (0..100000)
            .map(|i| {
                (
                    (i % 256) as u32,
                    ((i / 256) % 256) as u32,
                    (i / 65536) as u32,
                )
            })
            .collect();

        bencher.iter(|| {
            let mut total = 0u64;
            for (x, y, z) in &positions {
                // Inline Morton code calculation
                fn spread_bits_3(v: u32) -> u64 {
                    let mut x = v as u64 & 0x1FFFFF;
                    x = (x | (x << 32)) & 0x1F00000000FFFF;
                    x = (x | (x << 16)) & 0x1F0000FF0000FF;
                    x = (x | (x << 8)) & 0x100F00F00F00F00F;
                    x = (x | (x << 4)) & 0x10C30C30C30C30C3;
                    x = (x | (x << 2)) & 0x1249249249249249;
                    x
                }
                let code = spread_bits_3(*x) | (spread_bits_3(*y) << 1) | (spread_bits_3(*z) << 2);
                total = total.wrapping_add(code);
            }
            std_black_box(total)
        });
    });

    // Test 4: Kernel early-out benefit
    group.bench_function("kernel_without_early_out_10000", |bencher| {
        let positions: Vec<[f32; 3]> = (0..10000)
            .map(|i| {
                [
                    (i % 100) as f32 * 0.02,
                    ((i / 100) % 100) as f32 * 0.02,
                    0.0,
                ]
            })
            .collect();
        let center = [1.0, 1.0, 0.0];
        let h = 0.1f32;

        bencher.iter(|| {
            let mut total = 0.0f32;
            let h_inv = 1.0 / h;
            let norm = 8.0 / (std::f32::consts::PI * h * h * h);

            for p in &positions {
                let dx = p[0] - center[0];
                let dy = p[1] - center[1];
                let dz = p[2] - center[2];
                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                let q = r * h_inv;

                let val = if q >= 1.0 {
                    0.0
                } else if q >= 0.5 {
                    let t = 1.0 - q;
                    norm * 2.0 * t * t * t
                } else {
                    norm * (6.0 * q * q * (q - 1.0) + 1.0)
                };
                total += val;
            }
            std_black_box(total)
        });
    });

    group.bench_function("kernel_with_early_out_10000", |bencher| {
        let positions: Vec<[f32; 3]> = (0..10000)
            .map(|i| {
                [
                    (i % 100) as f32 * 0.02,
                    ((i / 100) % 100) as f32 * 0.02,
                    0.0,
                ]
            })
            .collect();
        let center = [1.0, 1.0, 0.0];
        let h = 0.1f32;

        bencher.iter(|| {
            let mut total = 0.0f32;
            let h_sq = h * h;
            let h_inv = 1.0 / h;
            let norm = 8.0 / (std::f32::consts::PI * h * h * h);

            for p in &positions {
                let dx = p[0] - center[0];
                let dy = p[1] - center[1];
                let dz = p[2] - center[2];
                let dist_sq = dx * dx + dy * dy + dz * dz;

                // Early out - skip sqrt if outside support
                if dist_sq >= h_sq {
                    continue;
                }

                let r = dist_sq.sqrt();
                let q = r * h_inv;

                let val = if q >= 0.5 {
                    let t = 1.0 - q;
                    norm * 2.0 * t * t * t
                } else {
                    norm * (6.0 * q * q * (q - 1.0) + 1.0)
                };
                total += val;
            }
            std_black_box(total)
        });
    });

    // Test 5: AOS vs SOA memory access pattern
    group.bench_function("aos_access_pattern_10000", |bencher| {
        let particles: Vec<[f32; 3]> = (0..10000)
            .map(|i| [i as f32 * 0.1, i as f32 * 0.2, i as f32 * 0.3])
            .collect();

        bencher.iter(|| {
            let mut sum_x = 0.0f32;
            let mut sum_y = 0.0f32;
            let mut sum_z = 0.0f32;
            for p in &particles {
                sum_x += p[0];
                sum_y += p[1];
                sum_z += p[2];
            }
            std_black_box((sum_x, sum_y, sum_z))
        });
    });

    group.bench_function("soa_access_pattern_10000", |bencher| {
        let n = 10000;
        let x: Vec<f32> = (0..n).map(|i| i as f32 * 0.1).collect();
        let y: Vec<f32> = (0..n).map(|i| i as f32 * 0.2).collect();
        let z: Vec<f32> = (0..n).map(|i| i as f32 * 0.3).collect();

        bencher.iter(|| {
            let sum_x: f32 = x.iter().sum();
            let sum_y: f32 = y.iter().sum();
            let sum_z: f32 = z.iter().sum();
            std_black_box((sum_x, sum_y, sum_z))
        });
    });

    // Test 6: Weighted centroid (XSPH common operation)
    group.bench_function("weighted_centroid_naive_1000", |bencher| {
        let positions: Vec<[f32; 3]> = (0..1000)
            .map(|i| {
                [
                    i as f32 * 0.01,
                    (i as f32 * 0.01).sin(),
                    (i as f32 * 0.01).cos(),
                ]
            })
            .collect();
        let weights: Vec<f32> = (0..1000).map(|i| 1.0 / (i as f32 + 1.0)).collect();

        bencher.iter(|| {
            let mut sum_x = 0.0f32;
            let mut sum_y = 0.0f32;
            let mut sum_z = 0.0f32;
            let mut sum_w = 0.0f32;

            for (p, &w) in positions.iter().zip(weights.iter()) {
                sum_x += w * p[0];
                sum_y += w * p[1];
                sum_z += w * p[2];
                sum_w += w;
            }

            let inv_w = 1.0 / sum_w;
            std_black_box([sum_x * inv_w, sum_y * inv_w, sum_z * inv_w])
        });
    });

    group.bench_function("weighted_centroid_fma_1000", |bencher| {
        let positions: Vec<[f32; 3]> = (0..1000)
            .map(|i| {
                [
                    i as f32 * 0.01,
                    (i as f32 * 0.01).sin(),
                    (i as f32 * 0.01).cos(),
                ]
            })
            .collect();
        let weights: Vec<f32> = (0..1000).map(|i| 1.0 / (i as f32 + 1.0)).collect();

        bencher.iter(|| {
            let mut sum_x = 0.0f32;
            let mut sum_y = 0.0f32;
            let mut sum_z = 0.0f32;
            let mut sum_w = 0.0f32;

            // 4x unroll with FMA
            let chunks = positions.len() / 4;
            for chunk in 0..chunks {
                let base = chunk * 4;
                sum_x = weights[base].mul_add(positions[base][0], sum_x);
                sum_y = weights[base].mul_add(positions[base][1], sum_y);
                sum_z = weights[base].mul_add(positions[base][2], sum_z);
                sum_w += weights[base];

                sum_x = weights[base + 1].mul_add(positions[base + 1][0], sum_x);
                sum_y = weights[base + 1].mul_add(positions[base + 1][1], sum_y);
                sum_z = weights[base + 1].mul_add(positions[base + 1][2], sum_z);
                sum_w += weights[base + 1];

                sum_x = weights[base + 2].mul_add(positions[base + 2][0], sum_x);
                sum_y = weights[base + 2].mul_add(positions[base + 2][1], sum_y);
                sum_z = weights[base + 2].mul_add(positions[base + 2][2], sum_z);
                sum_w += weights[base + 2];

                sum_x = weights[base + 3].mul_add(positions[base + 3][0], sum_x);
                sum_y = weights[base + 3].mul_add(positions[base + 3][1], sum_y);
                sum_z = weights[base + 3].mul_add(positions[base + 3][2], sum_z);
                sum_w += weights[base + 3];
            }

            for i in (chunks * 4)..positions.len() {
                sum_x = weights[i].mul_add(positions[i][0], sum_x);
                sum_y = weights[i].mul_add(positions[i][1], sum_y);
                sum_z = weights[i].mul_add(positions[i][2], sum_z);
                sum_w += weights[i];
            }

            let inv_w = 1.0 / sum_w;
            std_black_box([sum_x * inv_w, sum_y * inv_w, sum_z * inv_w])
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_particle_operations,
    bench_spatial_hashing,
    bench_sph_kernels,
    bench_density_pressure,
    bench_simulation_step,
    bench_gpu_data_prep,
    bench_optimization_controller,
    bench_adaptive_iterations,
    bench_lod_system,
    bench_simd_optimizations,
);

criterion_main!(benches);
