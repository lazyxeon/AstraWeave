use crate::types::InstanceRaw;
use glam::{vec3, Mat4, Vec3};
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WeatherKind {
    None,
    Rain,
    Snow,
    Sandstorm,
    WindTrails,
}

pub struct WeatherFx {
    kind: WeatherKind,
    /// Pre-allocated particle pool. Active particles are stored contiguously
    /// in `pool[0..active_count]`. Dead particles are swap-removed to avoid
    /// Vec shifts/reallocations during gameplay.
    pool: Vec<Particle>,
    /// Number of currently active particles in the pool.
    active_count: usize,
    buf: wgpu::Buffer,
    max: usize,
    /// Biome-driven tint multiplied into particle base colours.
    biome_tint: Vec3,
    /// Density multiplier (0.0-1.0) for biome-specific particle counts.
    density: f32,
    /// Wind strength for directional effects.
    wind_strength: f32,
    /// Wind direction (normalized XZ plane).
    wind_dir: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    color: [f32; 4],
    scale: Vec3,
}

impl WeatherFx {
    pub fn new(device: &wgpu::Device, max: usize) -> Self {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("weather inst"),
            size: (max * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            kind: WeatherKind::None,
            pool: Vec::with_capacity(max),
            active_count: 0,
            buf,
            max,
            biome_tint: Vec3::ONE, // neutral tint
            density: 1.0,
            wind_strength: 1.0,
            wind_dir: vec3(1.0, 0.0, 0.0),
        }
    }

    pub fn set_kind(&mut self, kind: WeatherKind) {
        self.kind = kind;
    }

    /// Apply biome-driven colour tint to all weather particles.
    ///
    /// The tint is multiplied into particle base colours (RGB). Pass
    /// `Vec3::ONE` for no tinting.
    pub fn set_biome_tint(&mut self, tint: Vec3) {
        self.biome_tint = tint;
    }

    /// Get current biome tint.
    pub fn biome_tint(&self) -> Vec3 {
        self.biome_tint
    }

    /// Set particle density multiplier.
    ///
    /// Value between 0.0 (no particles) and 1.0 (full density).
    /// Useful for biome-specific weather intensity.
    pub fn set_density(&mut self, density: f32) {
        self.density = density.clamp(0.0, 1.0);
    }

    /// Get current density.
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Set wind parameters for directional weather effects.
    pub fn set_wind(&mut self, strength: f32, direction: Vec3) {
        self.wind_strength = strength.max(0.0);
        let len = direction.length();
        self.wind_dir = if len > 0.001 {
            direction / len
        } else {
            vec3(1.0, 0.0, 0.0)
        };
    }

    /// Set the maximum particle count, reallocating the GPU buffer if needed.
    pub fn set_max(&mut self, device: &wgpu::Device, new_max: usize) {
        if new_max == self.max {
            return;
        }
        self.max = new_max;
        // Reallocate GPU buffer to fit the new maximum
        self.buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("weather inst"),
            size: (new_max * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Trim existing particles if they exceed the new cap
        if self.active_count > new_max {
            self.active_count = new_max;
            self.pool.truncate(new_max);
        }
        // Reserve capacity for the new max
        self.pool.reserve(new_max.saturating_sub(self.pool.len()));
    }

    /// Get current wind strength.
    pub fn wind_strength(&self) -> f32 {
        self.wind_strength
    }

    /// Get current wind direction.
    pub fn wind_direction(&self) -> Vec3 {
        self.wind_dir
    }

    /// Number of active particles for instanced drawing.
    pub fn particle_count(&self) -> usize {
        self.active_count
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: f32, camera_pos: Vec3) {
        // Early-out: skip all work (including GPU upload) when weather is off and no particles remain
        if self.kind == WeatherKind::None && self.active_count == 0 {
            return;
        }

        let effective_max = ((self.max as f32) * self.density).max(1.0) as usize;
        match self.kind {
            WeatherKind::None => {
                self.pool.clear();
                self.active_count = 0;
            }
            WeatherKind::Rain => self.tick_rain(dt, effective_max, camera_pos),
            WeatherKind::Snow => self.tick_snow(dt, effective_max, camera_pos),
            WeatherKind::Sandstorm => self.tick_sandstorm(dt, effective_max, camera_pos),
            WeatherKind::WindTrails => self.tick_wind(dt, effective_max, camera_pos),
        }
        // Apply biome tint and upload
        let tint = self.biome_tint;
        let raws: Vec<InstanceRaw> = self.pool[..self.active_count]
            .iter()
            .map(|p| {
                let m = Mat4::from_scale_rotation_translation(p.scale, glam::Quat::IDENTITY, p.pos);
                // Mix particle colour with biome tint (RGB only, preserve alpha)
                let tinted = [
                    p.color[0] * tint.x,
                    p.color[1] * tint.y,
                    p.color[2] * tint.z,
                    p.color[3],
                ];
                InstanceRaw {
                    model: m.to_cols_array_2d(),
                    normal_matrix: [
                        m.inverse().transpose().x_axis.truncate().to_array(),
                        m.inverse().transpose().y_axis.truncate().to_array(),
                        m.inverse().transpose().z_axis.truncate().to_array(),
                    ],
                    color: tinted,
                    material_id: 0,
                    _padding: [0; 3],
                }
            })
            .collect();
        queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&raws));
    }

    fn tick_rain(&mut self, dt: f32, max: usize, cam: Vec3) {
        let mut rng = rand::rng();
        let wind_offset = self.wind_dir * (self.wind_strength * 5.0);
        while self.active_count < max {
            // Per-particle wind variation: ±30% jitter on wind + slight random drift
            let jitter_x = rng.random_range(-0.3..0.3);
            let jitter_z = rng.random_range(-0.3..0.3);
            let speed_var = rng.random_range(0.7..1.3);
            let p = Particle {
                pos: vec3(
                    cam.x + rng.random_range(-30.0..30.0),
                    cam.y + rng.random_range(8.0..25.0),
                    cam.z + rng.random_range(-30.0..30.0),
                ),
                vel: vec3(
                    (wind_offset.x + jitter_x) * speed_var,
                    -20.0 * speed_var,
                    (wind_offset.z + jitter_z) * speed_var,
                ),
                life: rng.random_range(0.5..1.5),
                // Bright translucent white-blue so rain is visible against dark backgrounds
                color: [0.85, 0.9, 1.0, 0.6],
                // Thin elongated streaks (stretched in Y for falling rain look)
                scale: vec3(0.015, 0.6, 0.015),
            };
            if self.active_count < self.pool.len() {
                self.pool[self.active_count] = p;
            } else {
                self.pool.push(p);
            }
            self.active_count += 1;
        }
        // Swap-remove dead particles (O(1) per removal, no shifts)
        let mut i = 0;
        while i < self.active_count {
            let p = &mut self.pool[i];
            p.life -= dt;
            // Subtle per-frame turbulence: small random drift
            p.vel.x += rng.random_range(-0.5..0.5) * dt;
            p.vel.z += rng.random_range(-0.5..0.5) * dt;
            p.pos += p.vel * dt;
            let dist_sq = (p.pos - cam).length_squared();
            if p.life <= 0.0 || dist_sq >= 3600.0 {
                self.active_count -= 1;
                self.pool.swap(i, self.active_count);
            } else {
                i += 1;
            }
        }
    }

    fn tick_snow(&mut self, dt: f32, max: usize, cam: Vec3) {
        let mut rng = rand::rng();
        let wind_offset = self.wind_dir * (self.wind_strength * 2.0);
        while self.active_count < max {
            let p = Particle {
                pos: vec3(
                    cam.x + rng.random_range(-35.0..35.0),
                    cam.y + rng.random_range(10.0..25.0),
                    cam.z + rng.random_range(-35.0..35.0),
                ),
                vel: vec3(
                    wind_offset.x + rng.random_range(-0.5..0.5),
                    rng.random_range(-2.5..-1.5),
                    wind_offset.z + rng.random_range(-0.5..0.5),
                ),
                life: rng.random_range(3.0..6.0),
                color: [1.0, 1.0, 1.0, 0.85],
                scale: vec3(0.08, 0.08, 0.08),
            };
            if self.active_count < self.pool.len() {
                self.pool[self.active_count] = p;
            } else {
                self.pool.push(p);
            }
            self.active_count += 1;
        }
        let mut i = 0;
        while i < self.active_count {
            let p = &mut self.pool[i];
            p.life -= dt;
            let sway = (p.life * 2.0).sin() * 0.3;
            p.vel.x += sway * dt;
            p.pos += p.vel * dt;
            let dist_sq = (p.pos - cam).length_squared();
            if p.life <= 0.0 || dist_sq >= 4900.0 {
                self.active_count -= 1;
                self.pool.swap(i, self.active_count);
            } else {
                i += 1;
            }
        }
    }

    fn tick_sandstorm(&mut self, dt: f32, max: usize, cam: Vec3) {
        let mut rng = rand::rng();
        let wind_speed = self.wind_strength * 15.0;
        while self.active_count < max {
            let p = Particle {
                pos: vec3(
                    cam.x + rng.random_range(-50.0..50.0),
                    cam.y + rng.random_range(-2.0..8.0),
                    cam.z + rng.random_range(-50.0..50.0),
                ),
                vel: vec3(
                    self.wind_dir.x * wind_speed + rng.random_range(-2.0..2.0),
                    rng.random_range(-1.0..2.0),
                    self.wind_dir.z * wind_speed + rng.random_range(-2.0..2.0),
                ),
                life: rng.random_range(0.8..2.5),
                color: [0.85, 0.75, 0.55, 0.7],
                scale: vec3(0.03, 0.03, 0.15),
            };
            if self.active_count < self.pool.len() {
                self.pool[self.active_count] = p;
            } else {
                self.pool.push(p);
            }
            self.active_count += 1;
        }
        let mut i = 0;
        while i < self.active_count {
            let p = &mut self.pool[i];
            p.life -= dt;
            p.vel.y += (rng.random_range(-1.0..1.0) as f32) * dt * 5.0;
            p.pos += p.vel * dt;
            let dist_sq = (p.pos - cam).length_squared();
            if p.life <= 0.0 || dist_sq >= 6400.0 {
                self.active_count -= 1;
                self.pool.swap(i, self.active_count);
            } else {
                i += 1;
            }
        }
    }

    fn tick_wind(&mut self, dt: f32, max: usize, cam: Vec3) {
        let mut rng = rand::rng();
        let wind_vel = self.wind_dir * (self.wind_strength * 8.0);
        while self.active_count < max {
            let p = Particle {
                pos: vec3(
                    cam.x + rng.random_range(-30.0..30.0),
                    cam.y + rng.random_range(-1.0..4.0),
                    cam.z + rng.random_range(-30.0..30.0),
                ),
                vel: vec3(wind_vel.x + 2.0, 0.0, wind_vel.z + 0.5),
                life: rng.random_range(1.0..3.0),
                color: [1.0, 1.0, 1.0, 0.3],
                scale: vec3(0.05, 0.05, 0.8),
            };
            if self.active_count < self.pool.len() {
                self.pool[self.active_count] = p;
            } else {
                self.pool.push(p);
            }
            self.active_count += 1;
        }
        let mut i = 0;
        while i < self.active_count {
            let p = &mut self.pool[i];
            p.life -= dt;
            p.pos += p.vel * dt;
            if p.life <= 0.0 {
                self.active_count -= 1;
                self.pool.swap(i, self.active_count);
            } else {
                i += 1;
            }
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buf
    }
    pub fn count(&self) -> u32 {
        self.active_count as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: true,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("test_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
            })
            .await
            .expect("Failed to create device")
    }

    #[test]
    fn test_weather_fx_new() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let fx = WeatherFx::new(&device, 1000);

            assert_eq!(fx.max, 1000);
            assert_eq!(fx.count(), 0, "Should start with no particles");
        });
    }

    #[test]
    fn test_weather_fx_set_kind() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::Rain);
            fx.set_kind(WeatherKind::WindTrails);
            fx.set_kind(WeatherKind::None);

            // Should not crash
        });
    }

    #[test]
    fn test_weather_fx_update_none() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::None);
            fx.update(&queue, 0.016, Vec3::ZERO); // One frame

            assert_eq!(fx.count(), 0, "None weather should have no particles");
        });
    }

    #[test]
    fn test_weather_fx_update_rain() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::Rain);
            fx.update(&queue, 0.016, Vec3::ZERO);

            assert!(fx.count() > 0, "Rain should spawn particles");
            assert!(fx.count() <= 100, "Should not exceed max particles");
        });
    }

    #[test]
    fn test_weather_fx_update_wind() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::WindTrails);
            fx.update(&queue, 0.016, Vec3::ZERO);

            assert!(fx.count() > 0, "Wind should spawn particles");
            assert!(fx.count() <= 100, "Should not exceed max particles");
        });
    }

    #[test]
    fn test_weather_fx_rain_spawns_up_to_max() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let max = 50;
            let mut fx = WeatherFx::new(&device, max);

            fx.set_kind(WeatherKind::Rain);

            // Update multiple times to fill particles
            for _ in 0..10 {
                fx.update(&queue, 0.016, Vec3::ZERO);
            }

            assert_eq!(fx.count(), max as u32, "Should fill to max capacity");
        });
    }

    #[test]
    fn test_weather_fx_particles_despawn() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::Rain);
            fx.update(&queue, 0.016, Vec3::ZERO);

            // Update with large dt to age out particles
            for _ in 0..100 {
                fx.update(&queue, 1.0, Vec3::ZERO); // 1 second per frame
            }

            // Rain continuously spawns, so should maintain particles
            // (verifies spawning and despawning cycle works without crashing)
            // Count may fluctuate but system should be stable
            assert!(fx.count() <= 100, "Should not exceed max");
        });
    }

    #[test]
    fn test_weather_fx_switch_kind_clears() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::Rain);
            fx.update(&queue, 0.016, Vec3::ZERO);
            assert!(fx.count() > 0, "Rain should spawn particles");

            fx.set_kind(WeatherKind::None);
            fx.update(&queue, 0.016, Vec3::ZERO);
            assert_eq!(fx.count(), 0, "None should clear all particles");
        });
    }

    #[test]
    fn test_weather_kind_debug() {
        let kinds = vec![
            WeatherKind::None,
            WeatherKind::Rain,
            WeatherKind::WindTrails,
        ];

        for kind in kinds {
            let debug_str = format!("{:?}", kind);
            assert!(!debug_str.is_empty(), "Debug should work");
        }
    }

    #[test]
    fn test_weather_fx_buffer_exists() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let fx = WeatherFx::new(&device, 100);

            let _buf = fx.buffer();
            // Should return buffer reference without panic
        });
    }

    #[test]
    fn test_biome_tint_defaults_to_one() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let fx = WeatherFx::new(&device, 100);

            assert_eq!(fx.biome_tint(), Vec3::ONE);
        });
    }

    #[test]
    fn test_biome_tint_set_and_get() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            let tint = vec3(0.8, 0.9, 0.7);
            fx.set_biome_tint(tint);
            assert_eq!(fx.biome_tint(), tint);
        });
    }

    #[test]
    fn test_biome_tint_applied_to_particles() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 50);

            // Set tint and spawn particles
            fx.set_biome_tint(vec3(0.5, 1.0, 0.5)); // greenish
            fx.set_kind(WeatherKind::Rain);
            fx.update(&queue, 0.016, Vec3::ZERO);

            // Particles should have spawned
            assert!(fx.count() > 0, "Should have rain particles");
            // The actual tinting happens during update() buffer upload
            // and affects the GPU data - we've verified the code path runs
        });
    }

    #[test]
    fn test_weather_fx_update_snow() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::Snow);
            fx.update(&queue, 0.016, Vec3::ZERO);

            assert!(fx.count() > 0, "Snow should spawn particles");
            assert!(fx.count() <= 100, "Should not exceed max particles");
        });
    }

    #[test]
    fn test_weather_fx_update_sandstorm() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::Sandstorm);
            fx.update(&queue, 0.016, Vec3::ZERO);

            assert!(fx.count() > 0, "Sandstorm should spawn particles");
            assert!(fx.count() <= 100, "Should not exceed max particles");
        });
    }

    #[test]
    fn test_density_defaults_to_one() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let fx = WeatherFx::new(&device, 100);
            assert!((fx.density() - 1.0).abs() < 0.001);
        });
    }

    #[test]
    fn test_density_set_and_get() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_density(0.5);
            assert!((fx.density() - 0.5).abs() < 0.001);
        });
    }

    #[test]
    fn test_density_clamped() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_density(2.0);
            assert!((fx.density() - 1.0).abs() < 0.001, "Should clamp to 1.0");

            fx.set_density(-0.5);
            assert!((fx.density() - 0.0).abs() < 0.001, "Should clamp to 0.0");
        });
    }

    #[test]
    fn test_density_affects_particle_count() {
        pollster::block_on(async {
            let (device, queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_kind(WeatherKind::Rain);
            fx.set_density(0.5);

            // Update several times to fill
            for _ in 0..10 {
                fx.update(&queue, 0.016, Vec3::ZERO);
            }

            // Should have roughly half the max
            assert!(fx.count() <= 50, "Should respect density limit");
        });
    }

    #[test]
    fn test_wind_defaults() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let fx = WeatherFx::new(&device, 100);
            assert!((fx.wind_strength() - 1.0).abs() < 0.001);
            assert!((fx.wind_direction().x - 1.0).abs() < 0.001);
        });
    }

    #[test]
    fn test_wind_set_and_get() {
        pollster::block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut fx = WeatherFx::new(&device, 100);

            fx.set_wind(2.5, vec3(0.0, 0.0, 1.0));
            assert!((fx.wind_strength() - 2.5).abs() < 0.001);
            assert!((fx.wind_direction().z - 1.0).abs() < 0.001);
        });
    }

    #[test]
    fn test_weather_kind_eq() {
        assert_eq!(WeatherKind::None, WeatherKind::None);
        assert_eq!(WeatherKind::Rain, WeatherKind::Rain);
        assert_eq!(WeatherKind::Snow, WeatherKind::Snow);
        assert_eq!(WeatherKind::Sandstorm, WeatherKind::Sandstorm);
        assert_eq!(WeatherKind::WindTrails, WeatherKind::WindTrails);
        assert_ne!(WeatherKind::Rain, WeatherKind::Snow);
    }
}
