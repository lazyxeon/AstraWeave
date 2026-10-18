//! Benchmarks for Transparency, Environment Rendering, and MSAA Subsystems
//!
//! This benchmark suite covers three critical rendering subsystems:
//! 1. Transparency & Depth Sorting: Back-to-front sorting, blend modes
//! 2. Environment Rendering: Time of day, sky, weather, particles
//! 3. MSAA: Multisample anti-aliasing configuration and state management
//!
//! Part of the AstraWeave AI-Native Game Engine benchmark infrastructure.

use criterion::{
    criterion_group, criterion_main, BenchmarkGroup, BenchmarkId, Criterion, SamplingMode,
    Throughput,
};
use glam::{vec3, Vec3};
use std::hint::black_box;
use std::time::{Duration, Instant};

// Shard 14 was cancelled at 60 minutes before this target finished
// (run 39100674332). Same flat sample budget so it cannot take 600s.
// Every function still runs. The caps stay.
fn ci_budget<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(1));
    group.measurement_time(Duration::from_secs(1));
    group
}

// ============================================================================
// SECTION 1: TRANSPARENCY & DEPTH SORTING BENCHMARKS
// ============================================================================

/// Blend mode for transparent rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    /// Standard alpha blending: src * alpha + dst * (1 - alpha)
    #[default]
    Alpha,
    /// Additive blending: src + dst
    Additive,
    /// Multiplicative blending: src * dst
    Multiplicative,
}

/// A transparent instance to be sorted for rendering
#[derive(Debug, Clone)]
pub struct TransparentInstance {
    pub instance_index: u32,
    pub world_position: Vec3,
    pub camera_distance: f32,
    pub blend_mode: BlendMode,
}

impl TransparentInstance {
    pub fn new(index: u32, position: Vec3, blend_mode: BlendMode) -> Self {
        Self {
            instance_index: index,
            world_position: position,
            camera_distance: 0.0,
            blend_mode,
        }
    }
}

/// Manages transparent instances with depth sorting
pub struct TransparencyManager {
    instances: Vec<TransparentInstance>,
    sorted_cache: Vec<TransparentInstance>,
    needs_sort: bool,
    camera_pos: Vec3,
}

impl TransparencyManager {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            sorted_cache: Vec::new(),
            needs_sort: true,
            camera_pos: Vec3::ZERO,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            sorted_cache: Vec::with_capacity(capacity),
            needs_sort: true,
            camera_pos: Vec3::ZERO,
        }
    }

    pub fn add_instance(&mut self, instance: TransparentInstance) {
        self.instances.push(instance);
        self.needs_sort = true;
    }

    pub fn update(&mut self, camera_pos: Vec3) {
        self.camera_pos = camera_pos;

        // Calculate camera distances
        for instance in &mut self.instances {
            instance.camera_distance = (instance.world_position - camera_pos).length();
        }

        // Clone to cache and sort back-to-front (descending by distance)
        self.sorted_cache.clone_from(&self.instances);
        self.sorted_cache
            .sort_by(|a, b| b.camera_distance.partial_cmp(&a.camera_distance).unwrap());

        self.needs_sort = false;
    }

    pub fn sorted_instances(&self) -> &[TransparentInstance] {
        &self.sorted_cache
    }

    pub fn instances_by_blend_mode(
        &self,
        mode: BlendMode,
    ) -> impl Iterator<Item = &TransparentInstance> {
        self.sorted_cache
            .iter()
            .filter(move |i| i.blend_mode == mode)
    }

    pub fn clear(&mut self) {
        self.instances.clear();
        self.sorted_cache.clear();
        self.needs_sort = true;
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

impl Default for TransparencyManager {
    fn default() -> Self {
        Self::new()
    }
}

fn bench_transparency_manager(c: &mut Criterion) {
    let mut group = ci_budget(c, "Transparency_Manager");

    // Benchmark creation
    group.bench_function("new", |b| {
        b.iter(|| {
            let manager = TransparencyManager::new();
            black_box(manager)
        })
    });

    group.bench_function("with_capacity_1000", |b| {
        b.iter(|| {
            let manager = TransparencyManager::with_capacity(1000);
            black_box(manager)
        })
    });

    // Benchmark add_instance for different counts
    for count in [100, 500, 1000, 5000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("add_instances", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let mut manager = TransparencyManager::with_capacity(count);
                    for i in 0..count {
                        let pos = vec3(
                            (i as f32 * 1.1) % 100.0,
                            (i as f32 * 0.7) % 50.0,
                            (i as f32 * 2.3) % 100.0,
                        );
                        let mode = match i % 3 {
                            0 => BlendMode::Alpha,
                            1 => BlendMode::Additive,
                            _ => BlendMode::Multiplicative,
                        };
                        manager.add_instance(TransparentInstance::new(i as u32, pos, mode));
                    }
                    black_box(manager)
                })
            },
        );
    }

    group.finish();
}

fn bench_depth_sorting(c: &mut Criterion) {
    let mut group = ci_budget(c, "Depth_Sorting");

    // Different instance counts for sorting
    for count in [100, 500, 1000, 2000, 5000, 10000] {
        // Pre-populate manager
        let mut manager = TransparencyManager::with_capacity(count);
        for i in 0..count {
            let pos = vec3(
                (i as f32 * 1.1) % 100.0,
                (i as f32 * 0.7) % 50.0,
                (i as f32 * 2.3) % 100.0,
            );
            let mode = match i % 3 {
                0 => BlendMode::Alpha,
                1 => BlendMode::Additive,
                _ => BlendMode::Multiplicative,
            };
            manager.add_instance(TransparentInstance::new(i as u32, pos, mode));
        }

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("update_and_sort", count),
            &count,
            |b, _| {
                b.iter(|| {
                    let camera_pos = vec3(50.0, 25.0, 50.0);
                    manager.update(camera_pos);
                    black_box(manager.sorted_instances().len())
                })
            },
        );
    }

    group.finish();
}

fn bench_blend_mode_filtering(c: &mut Criterion) {
    let mut group = ci_budget(c, "Blend_Mode_Filter");

    // Pre-populate with mixed blend modes
    let count = 3000;
    let mut manager = TransparencyManager::with_capacity(count);
    for i in 0..count {
        let pos = vec3(
            (i as f32 * 1.1) % 100.0,
            (i as f32 * 0.7) % 50.0,
            (i as f32 * 2.3) % 100.0,
        );
        let mode = match i % 3 {
            0 => BlendMode::Alpha,
            1 => BlendMode::Additive,
            _ => BlendMode::Multiplicative,
        };
        manager.add_instance(TransparentInstance::new(i as u32, pos, mode));
    }
    manager.update(vec3(50.0, 25.0, 50.0));

    group.bench_function("filter_alpha", |b| {
        b.iter(|| {
            let count = manager.instances_by_blend_mode(BlendMode::Alpha).count();
            black_box(count)
        })
    });

    group.bench_function("filter_additive", |b| {
        b.iter(|| {
            let count = manager.instances_by_blend_mode(BlendMode::Additive).count();
            black_box(count)
        })
    });

    group.bench_function("filter_all_modes", |b| {
        b.iter(|| {
            let alpha = manager.instances_by_blend_mode(BlendMode::Alpha).count();
            let additive = manager.instances_by_blend_mode(BlendMode::Additive).count();
            let mult = manager
                .instances_by_blend_mode(BlendMode::Multiplicative)
                .count();
            black_box((alpha, additive, mult))
        })
    });

    group.finish();
}

// ============================================================================
// SECTION 2: ENVIRONMENT RENDERING BENCHMARKS
// ============================================================================

/// Time of day system managing sun/moon positions
#[derive(Debug, Clone)]
pub struct TimeOfDay {
    pub current_time: f32, // 0.0 - 24.0
    pub time_scale: f32,
    pub day_length: f32,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self {
            current_time: 12.0,
            time_scale: 60.0,
            day_length: 1440.0,
        }
    }
}

impl TimeOfDay {
    pub fn new(start_time: f32, time_scale: f32) -> Self {
        Self {
            current_time: start_time,
            time_scale,
            day_length: 1440.0,
        }
    }

    pub fn update(&mut self, dt_seconds: f32) {
        let game_hours = (dt_seconds * self.time_scale) / 3600.0;
        self.current_time = (self.current_time + game_hours).rem_euclid(24.0);
    }

    pub fn get_sun_position(&self) -> Vec3 {
        let sun_angle = (self.current_time - 6.0) * std::f32::consts::PI / 12.0;
        let sun_height = sun_angle.sin();
        let sun_azimuth = (self.current_time - 12.0) * std::f32::consts::PI / 12.0;

        if sun_height.abs() < 0.01 {
            vec3(sun_azimuth.sin(), 0.0, sun_azimuth.cos()).normalize()
        } else {
            let horizontal_distance = (1.0 - sun_height.abs()).max(0.1);
            vec3(
                sun_azimuth.sin() * horizontal_distance,
                sun_height,
                sun_azimuth.cos() * horizontal_distance,
            )
            .normalize()
        }
    }

    pub fn get_moon_position(&self) -> Vec3 {
        -self.get_sun_position()
    }

    pub fn get_light_direction(&self) -> Vec3 {
        let sun_pos = self.get_sun_position();
        if sun_pos.y > 0.1 {
            -sun_pos
        } else {
            -self.get_moon_position()
        }
    }

    pub fn get_light_color(&self) -> Vec3 {
        let sun_height = self.get_sun_position().y;

        if sun_height > 0.2 {
            let intensity = (sun_height - 0.2) / 0.8;
            vec3(1.0, 0.95, 0.8) * (0.8 + 0.2 * intensity)
        } else if sun_height > -0.2 {
            let intensity = (sun_height + 0.2) / 0.4;
            vec3(1.0, 0.6, 0.3) * (0.3 + 0.5 * intensity)
        } else {
            vec3(0.3, 0.4, 0.8) * 0.15
        }
    }

    pub fn get_ambient_color(&self) -> Vec3 {
        let sun_height = self.get_sun_position().y;

        if sun_height > 0.0 {
            let intensity = sun_height.min(1.0);
            vec3(0.4, 0.6, 1.0) * (0.3 + 0.4 * intensity)
        } else {
            vec3(0.1, 0.15, 0.3) * 0.1
        }
    }

    pub fn is_day(&self) -> bool {
        self.get_sun_position().y > 0.0
    }

    pub fn is_night(&self) -> bool {
        self.get_sun_position().y < -0.1
    }

    pub fn is_twilight(&self) -> bool {
        let sun_height = self.get_sun_position().y;
        (-0.1..=0.1).contains(&sun_height)
    }
}

fn bench_time_of_day(c: &mut Criterion) {
    let mut group = ci_budget(c, "Time_Of_Day");

    group.bench_function("new", |b| {
        b.iter(|| {
            let tod = TimeOfDay::new(12.0, 60.0);
            black_box(tod)
        })
    });

    let tod = TimeOfDay::new(12.0, 60.0);

    group.bench_function("get_sun_position", |b| {
        b.iter(|| {
            let pos = tod.get_sun_position();
            black_box(pos)
        })
    });

    group.bench_function("get_moon_position", |b| {
        b.iter(|| {
            let pos = tod.get_moon_position();
            black_box(pos)
        })
    });

    group.bench_function("get_light_direction", |b| {
        b.iter(|| {
            let dir = tod.get_light_direction();
            black_box(dir)
        })
    });

    group.bench_function("get_light_color", |b| {
        b.iter(|| {
            let color = tod.get_light_color();
            black_box(color)
        })
    });

    group.bench_function("get_ambient_color", |b| {
        b.iter(|| {
            let color = tod.get_ambient_color();
            black_box(color)
        })
    });

    // Benchmark full state query (all lighting data)
    group.bench_function("full_lighting_query", |b| {
        b.iter(|| {
            let sun = tod.get_sun_position();
            let moon = tod.get_moon_position();
            let light_dir = tod.get_light_direction();
            let light_color = tod.get_light_color();
            let ambient = tod.get_ambient_color();
            let is_day = tod.is_day();
            black_box((sun, moon, light_dir, light_color, ambient, is_day))
        })
    });

    // Benchmark time cycle (24 hours)
    group.bench_function("time_cycle_24h", |b| {
        b.iter(|| {
            let mut tod = TimeOfDay::new(0.0, 1.0);
            for hour in 0..24 {
                tod.current_time = hour as f32;
                let _ = black_box(tod.get_light_color());
            }
        })
    });

    group.finish();
}

/// Weather types available
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherType {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Snow,
    Fog,
    Sandstorm,
}

/// Weather system managing transitions and effects
#[derive(Debug, Clone)]
pub struct WeatherSystem {
    current_weather: WeatherType,
    target_weather: WeatherType,
    transition_duration: f32,
    transition_progress: f32,
    rain_intensity: f32,
    snow_intensity: f32,
    fog_density: f32,
    wind_strength: f32,
    wind_direction: Vec3,
    last_update: Instant,
}

impl Default for WeatherSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherSystem {
    pub fn new() -> Self {
        Self {
            current_weather: WeatherType::Clear,
            target_weather: WeatherType::Clear,
            transition_duration: 30.0,
            transition_progress: 1.0,
            rain_intensity: 0.0,
            snow_intensity: 0.0,
            fog_density: 0.0,
            wind_strength: 0.1,
            wind_direction: vec3(1.0, 0.0, 0.0),
            last_update: Instant::now(),
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.last_update = Instant::now();

        if self.current_weather != self.target_weather {
            self.transition_progress += delta_time / self.transition_duration;

            if self.transition_progress >= 1.0 {
                self.current_weather = self.target_weather;
                self.transition_progress = 1.0;
            }
        }

        self.update_weather_parameters();

        let wind_variation = (self.last_update.elapsed().as_secs_f32() * 0.5).sin() * 0.1;
        self.wind_direction = vec3(
            (1.0 + wind_variation).cos(),
            0.0,
            (1.0 + wind_variation).sin(),
        )
        .normalize();
    }

    pub fn set_weather(&mut self, weather: WeatherType, transition_duration: f32) {
        if weather != self.current_weather {
            self.target_weather = weather;
            if transition_duration <= 0.0 {
                self.current_weather = weather;
                self.transition_progress = 1.0;
                self.update_weather_parameters();
            } else {
                self.transition_duration = transition_duration;
                self.transition_progress = 0.0;
            }
        }
    }

    pub fn current_weather(&self) -> WeatherType {
        self.current_weather
    }

    pub fn get_rain_intensity(&self) -> f32 {
        self.rain_intensity
    }

    pub fn get_snow_intensity(&self) -> f32 {
        self.snow_intensity
    }

    pub fn get_fog_density(&self) -> f32 {
        self.fog_density
    }

    pub fn get_wind_strength(&self) -> f32 {
        self.wind_strength
    }

    pub fn get_wind_direction(&self) -> Vec3 {
        self.wind_direction
    }

    pub fn is_raining(&self) -> bool {
        matches!(self.current_weather, WeatherType::Rain | WeatherType::Storm)
            && self.rain_intensity > 0.1
    }

    pub fn is_snowing(&self) -> bool {
        self.current_weather == WeatherType::Snow && self.snow_intensity > 0.1
    }

    pub fn get_terrain_color_modifier(&self) -> Vec3 {
        match self.current_weather {
            WeatherType::Clear => vec3(1.0, 1.0, 1.0),
            WeatherType::Cloudy => vec3(0.8, 0.8, 0.9),
            WeatherType::Rain | WeatherType::Storm => {
                let wetness = self.rain_intensity * 0.7;
                vec3(
                    1.0 - wetness * 0.3,
                    1.0 - wetness * 0.2,
                    1.0 - wetness * 0.1,
                )
            }
            WeatherType::Snow => {
                let snow_cover = self.snow_intensity * 0.8;
                vec3(
                    1.0 + snow_cover * 0.5,
                    1.0 + snow_cover * 0.5,
                    1.0 + snow_cover * 0.6,
                )
            }
            WeatherType::Fog => vec3(0.9, 0.9, 1.0),
            WeatherType::Sandstorm => vec3(1.0, 0.8, 0.6),
        }
    }

    pub fn get_light_attenuation(&self) -> f32 {
        match self.current_weather {
            WeatherType::Clear => 1.0,
            WeatherType::Cloudy => 0.7,
            WeatherType::Rain => 0.5,
            WeatherType::Storm => 0.3,
            WeatherType::Snow => 0.6,
            WeatherType::Fog => 0.4,
            WeatherType::Sandstorm => 0.2,
        }
    }

    fn update_weather_parameters(&mut self) {
        let t = self.transition_progress;

        let (target_rain, target_snow, target_fog, target_wind) = match self.target_weather {
            WeatherType::Clear => (0.0, 0.0, 0.0, 0.1),
            WeatherType::Cloudy => (0.0, 0.0, 0.1, 0.2),
            WeatherType::Rain => (0.7, 0.0, 0.2, 0.4),
            WeatherType::Storm => (1.0, 0.0, 0.3, 0.8),
            WeatherType::Snow => (0.0, 0.8, 0.1, 0.3),
            WeatherType::Fog => (0.0, 0.0, 0.9, 0.1),
            WeatherType::Sandstorm => (0.0, 0.0, 0.4, 1.0),
        };

        let (current_rain, current_snow, current_fog, current_wind) = if t < 1.0 {
            match self.current_weather {
                WeatherType::Clear => (0.0, 0.0, 0.0, 0.1),
                WeatherType::Cloudy => (0.0, 0.0, 0.1, 0.2),
                WeatherType::Rain => (0.7, 0.0, 0.2, 0.4),
                WeatherType::Storm => (1.0, 0.0, 0.3, 0.8),
                WeatherType::Snow => (0.0, 0.8, 0.1, 0.3),
                WeatherType::Fog => (0.0, 0.0, 0.9, 0.1),
                WeatherType::Sandstorm => (0.0, 0.0, 0.4, 1.0),
            }
        } else {
            (target_rain, target_snow, target_fog, target_wind)
        };

        self.rain_intensity = current_rain + (target_rain - current_rain) * t;
        self.snow_intensity = current_snow + (target_snow - current_snow) * t;
        self.fog_density = current_fog + (target_fog - current_fog) * t;
        self.wind_strength = current_wind + (target_wind - current_wind) * t;
    }
}

fn bench_weather_system(c: &mut Criterion) {
    let mut group = ci_budget(c, "Weather_System");

    group.bench_function("new", |b| {
        b.iter(|| {
            let weather = WeatherSystem::new();
            black_box(weather)
        })
    });

    let mut weather = WeatherSystem::new();

    group.bench_function("update", |b| {
        b.iter(|| {
            weather.update(1.0 / 60.0);
            black_box(weather.current_weather())
        })
    });

    // Benchmark weather transition
    group.bench_function("set_weather_instant", |b| {
        b.iter(|| {
            let mut w = WeatherSystem::new();
            w.set_weather(WeatherType::Rain, 0.0);
            black_box(w.current_weather())
        })
    });

    group.bench_function("set_weather_transition", |b| {
        b.iter(|| {
            let mut w = WeatherSystem::new();
            w.set_weather(WeatherType::Rain, 30.0);
            black_box(w.current_weather())
        })
    });

    // Benchmark weather queries
    group.bench_function("get_all_intensities", |b| {
        let weather = WeatherSystem::new();
        b.iter(|| {
            let rain = weather.get_rain_intensity();
            let snow = weather.get_snow_intensity();
            let fog = weather.get_fog_density();
            let wind = weather.get_wind_strength();
            let wind_dir = weather.get_wind_direction();
            black_box((rain, snow, fog, wind, wind_dir))
        })
    });

    group.bench_function("get_terrain_modifier", |b| {
        let weather = WeatherSystem::new();
        b.iter(|| {
            let modifier = weather.get_terrain_color_modifier();
            black_box(modifier)
        })
    });

    group.bench_function("get_light_attenuation", |b| {
        let weather = WeatherSystem::new();
        b.iter(|| {
            let atten = weather.get_light_attenuation();
            black_box(atten)
        })
    });

    // Benchmark all weather type transitions
    group.bench_function("all_weather_types_query", |b| {
        b.iter(|| {
            let weather_types = [
                WeatherType::Clear,
                WeatherType::Cloudy,
                WeatherType::Rain,
                WeatherType::Storm,
                WeatherType::Snow,
                WeatherType::Fog,
                WeatherType::Sandstorm,
            ];

            for wt in weather_types.iter() {
                let mut w = WeatherSystem::new();
                w.set_weather(*wt, 0.0);
                let _ = black_box(w.get_light_attenuation());
            }
        })
    });

    group.finish();
}

/// Weather particle for precipitation
#[derive(Debug, Clone)]
pub struct WeatherParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
}

/// Weather particle system
pub struct WeatherParticles {
    rain_particles: Vec<WeatherParticle>,
    snow_particles: Vec<WeatherParticle>,
    max_particles: usize,
    particle_area: f32,
}

impl WeatherParticles {
    pub fn new(max_particles: usize, area: f32) -> Self {
        Self {
            rain_particles: Vec::with_capacity(max_particles),
            snow_particles: Vec::with_capacity(max_particles),
            max_particles,
            particle_area: area,
        }
    }

    pub fn spawn_rain_particles(&mut self, count: usize, camera_pos: Vec3, wind: Vec3) {
        for i in 0..count.min(self.max_particles - self.rain_particles.len()) {
            let offset = Vec3::new(
                ((i as f32 * 17.3) % 1.0 - 0.5) * self.particle_area,
                (i as f32 * 7.1) % 50.0 + 20.0,
                ((i as f32 * 23.7) % 1.0 - 0.5) * self.particle_area,
            );

            self.rain_particles.push(WeatherParticle {
                position: camera_pos + offset,
                velocity: vec3(wind.x * 2.0, -15.0, wind.z * 2.0),
                life: 0.0,
                max_life: 3.0,
                size: 0.1,
            });
        }
    }

    pub fn spawn_snow_particles(&mut self, count: usize, camera_pos: Vec3, wind: Vec3) {
        for i in 0..count.min(self.max_particles - self.snow_particles.len()) {
            let offset = Vec3::new(
                ((i as f32 * 17.3) % 1.0 - 0.5) * self.particle_area,
                (i as f32 * 7.1) % 30.0 + 15.0,
                ((i as f32 * 23.7) % 1.0 - 0.5) * self.particle_area,
            );

            self.snow_particles.push(WeatherParticle {
                position: camera_pos + offset,
                velocity: vec3(wind.x, -2.0, wind.z),
                life: 0.0,
                max_life: 10.0,
                size: 0.2,
            });
        }
    }

    pub fn update_rain(&mut self, delta_time: f32, camera_pos: Vec3) {
        for particle in &mut self.rain_particles {
            particle.life += delta_time;
            particle.position += particle.velocity * delta_time;
        }

        let area = self.particle_area;
        self.rain_particles
            .retain(|p| p.life < p.max_life && (p.position - camera_pos).length() < area * 0.6);
    }

    pub fn update_snow(&mut self, delta_time: f32, camera_pos: Vec3) {
        let drift_factor = 0.1;
        for particle in &mut self.snow_particles {
            particle.life += delta_time;
            particle.position += particle.velocity * delta_time;

            // Add drift simulation
            particle.velocity.x += ((particle.life * 3.7) % 1.0 - 0.5) * drift_factor;
            particle.velocity.z += ((particle.life * 5.3) % 1.0 - 0.5) * drift_factor;
        }

        let area = self.particle_area;
        self.snow_particles
            .retain(|p| p.life < p.max_life && (p.position - camera_pos).length() < area * 0.6);
    }

    pub fn rain_count(&self) -> usize {
        self.rain_particles.len()
    }

    pub fn snow_count(&self) -> usize {
        self.snow_particles.len()
    }

    pub fn clear(&mut self) {
        self.rain_particles.clear();
        self.snow_particles.clear();
    }
}

fn bench_weather_particles(c: &mut Criterion) {
    let mut group = ci_budget(c, "Weather_Particles");

    group.bench_function("new_1000", |b| {
        b.iter(|| {
            let particles = WeatherParticles::new(1000, 100.0);
            black_box(particles)
        })
    });

    group.bench_function("new_10000", |b| {
        b.iter(|| {
            let particles = WeatherParticles::new(10000, 100.0);
            black_box(particles)
        })
    });

    // Benchmark particle spawning
    for count in [100, 500, 1000, 5000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("spawn_rain", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let mut particles = WeatherParticles::new(count * 2, 100.0);
                    let camera_pos = vec3(0.0, 0.0, 0.0);
                    let wind = vec3(1.0, 0.0, 0.5);
                    particles.spawn_rain_particles(count, camera_pos, wind);
                    black_box(particles.rain_count())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("spawn_snow", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let mut particles = WeatherParticles::new(count * 2, 100.0);
                    let camera_pos = vec3(0.0, 0.0, 0.0);
                    let wind = vec3(0.5, 0.0, 0.3);
                    particles.spawn_snow_particles(count, camera_pos, wind);
                    black_box(particles.snow_count())
                })
            },
        );
    }

    // Benchmark particle updates
    for count in [100, 500, 1000, 5000] {
        group.throughput(Throughput::Elements(count as u64));

        // Pre-spawn particles
        let mut particles = WeatherParticles::new(count * 2, 100.0);
        let camera_pos = vec3(0.0, 0.0, 0.0);
        let wind = vec3(1.0, 0.0, 0.5);
        particles.spawn_rain_particles(count, camera_pos, wind);

        group.bench_with_input(BenchmarkId::new("update_rain", count), &count, |b, _| {
            b.iter(|| {
                particles.update_rain(1.0 / 60.0, camera_pos);
                black_box(particles.rain_count())
            })
        });

        // Reset for snow
        let mut snow_particles = WeatherParticles::new(count * 2, 100.0);
        snow_particles.spawn_snow_particles(count, camera_pos, wind);

        group.bench_with_input(BenchmarkId::new("update_snow", count), &count, |b, _| {
            b.iter(|| {
                snow_particles.update_snow(1.0 / 60.0, camera_pos);
                black_box(snow_particles.snow_count())
            })
        });
    }

    group.finish();
}

// ============================================================================
// SECTION 3: MSAA BENCHMARKS
// ============================================================================

/// MSAA sample count configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MsaaMode {
    Off,
    X2,
    #[default]
    X4,
    X8,
}

impl MsaaMode {
    pub fn sample_count(self) -> u32 {
        match self {
            MsaaMode::Off => 1,
            MsaaMode::X2 => 2,
            MsaaMode::X4 => 4,
            MsaaMode::X8 => 8,
        }
    }

    pub fn is_enabled(self) -> bool {
        self != MsaaMode::Off
    }

    pub fn memory_multiplier(self) -> f32 {
        self.sample_count() as f32
    }
}

/// MSAA render target manager (simplified for benchmarking)
pub struct MsaaRenderTarget {
    mode: MsaaMode,
    width: u32,
    height: u32,
    needs_recreate: bool,
}

impl MsaaRenderTarget {
    pub fn new() -> Self {
        Self {
            mode: MsaaMode::default(),
            width: 0,
            height: 0,
            needs_recreate: true,
        }
    }

    pub fn set_mode(&mut self, mode: MsaaMode) {
        if mode != self.mode {
            self.mode = mode;
            self.needs_recreate = true;
        }
    }

    pub fn mode(&self) -> MsaaMode {
        self.mode
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.needs_recreate = true;
        }
    }

    pub fn needs_recreate(&self) -> bool {
        self.needs_recreate
    }

    pub fn mark_recreated(&mut self) {
        self.needs_recreate = false;
    }

    pub fn calculate_memory_usage(&self) -> usize {
        if !self.mode.is_enabled() {
            return 0;
        }
        let bytes_per_pixel = 4 * 4; // RGBA32Float
        let samples = self.mode.sample_count() as usize;
        self.width as usize * self.height as usize * bytes_per_pixel * samples
    }
}

impl Default for MsaaRenderTarget {
    fn default() -> Self {
        Self::new()
    }
}

fn bench_msaa(c: &mut Criterion) {
    let mut group = ci_budget(c, "MSAA");

    group.bench_function("mode_sample_count", |b| {
        b.iter(|| {
            let counts = [
                MsaaMode::Off.sample_count(),
                MsaaMode::X2.sample_count(),
                MsaaMode::X4.sample_count(),
                MsaaMode::X8.sample_count(),
            ];
            black_box(counts)
        })
    });

    group.bench_function("mode_is_enabled", |b| {
        b.iter(|| {
            let enabled = [
                MsaaMode::Off.is_enabled(),
                MsaaMode::X2.is_enabled(),
                MsaaMode::X4.is_enabled(),
                MsaaMode::X8.is_enabled(),
            ];
            black_box(enabled)
        })
    });

    group.bench_function("render_target_new", |b| {
        b.iter(|| {
            let target = MsaaRenderTarget::new();
            black_box(target)
        })
    });

    group.bench_function("render_target_set_mode", |b| {
        b.iter(|| {
            let mut target = MsaaRenderTarget::new();
            target.set_mode(MsaaMode::X2);
            target.set_mode(MsaaMode::X4);
            target.set_mode(MsaaMode::X8);
            target.set_mode(MsaaMode::Off);
            black_box(target.mode())
        })
    });

    // Resolution benchmarks
    let resolutions = [
        (1280, 720, "720p"),
        (1920, 1080, "1080p"),
        (2560, 1440, "1440p"),
        (3840, 2160, "4K"),
    ];

    for (width, height, name) in resolutions.iter() {
        group.bench_function(format!("resize_{}", name), |b| {
            b.iter(|| {
                let mut target = MsaaRenderTarget::new();
                target.resize(*width, *height);
                black_box(target.needs_recreate())
            })
        });

        group.bench_function(format!("memory_calc_{}", name), |b| {
            let mut target = MsaaRenderTarget::new();
            target.set_mode(MsaaMode::X4);
            target.resize(*width, *height);

            b.iter(|| {
                let memory = target.calculate_memory_usage();
                black_box(memory)
            })
        });
    }

    group.finish();
}

// ============================================================================
// SECTION 4: COMBINED SCENARIO BENCHMARKS
// ============================================================================

fn bench_full_environment_frame(c: &mut Criterion) {
    let mut group = ci_budget(c, "Full_Environment_Frame");

    // Simulate a complete frame update for environment systems
    group.bench_function("typical_frame", |b| {
        let mut tod = TimeOfDay::new(12.0, 60.0);
        let mut weather = WeatherSystem::new();
        let mut particles = WeatherParticles::new(5000, 100.0);
        let camera_pos = vec3(0.0, 50.0, 0.0);
        let delta_time = 1.0 / 60.0;

        // Pre-populate particles
        particles.spawn_rain_particles(1000, camera_pos, vec3(1.0, 0.0, 0.5));

        b.iter(|| {
            // Update time of day
            tod.update(1.0 / 60.0);

            // Update weather
            weather.update(delta_time);

            // Update particles
            particles.update_rain(delta_time, camera_pos);

            // Query all lighting
            let sun = tod.get_sun_position();
            let light_color = tod.get_light_color();
            let ambient = tod.get_ambient_color();
            let terrain_mod = weather.get_terrain_color_modifier();
            let light_atten = weather.get_light_attenuation();

            black_box((sun, light_color, ambient, terrain_mod, light_atten))
        })
    });

    // Heavy weather frame (storm with particles)
    group.bench_function("storm_frame_5000_particles", |b| {
        let mut weather = WeatherSystem::new();
        weather.set_weather(WeatherType::Storm, 0.0);
        let mut particles = WeatherParticles::new(10000, 100.0);
        let camera_pos = vec3(0.0, 50.0, 0.0);
        let delta_time = 1.0 / 60.0;

        // Pre-populate particles
        particles.spawn_rain_particles(5000, camera_pos, weather.get_wind_direction());

        b.iter(|| {
            weather.update(delta_time);
            particles.update_rain(delta_time, camera_pos);

            // Spawn more to maintain count
            let wind = weather.get_wind_direction();
            if particles.rain_count() < 4500 {
                particles.spawn_rain_particles(500, camera_pos, wind);
            }

            black_box(particles.rain_count())
        })
    });

    // Combined transparency + environment
    group.bench_function("transparency_with_weather_1000", |b| {
        let mut transparency = TransparencyManager::with_capacity(1000);
        let weather = WeatherSystem::new();
        let camera_pos = vec3(50.0, 25.0, 50.0);

        // Pre-populate transparency
        for i in 0..1000 {
            let pos = vec3(
                (i as f32 * 1.1) % 100.0,
                (i as f32 * 0.7) % 50.0,
                (i as f32 * 2.3) % 100.0,
            );
            let mode = match i % 3 {
                0 => BlendMode::Alpha,
                1 => BlendMode::Additive,
                _ => BlendMode::Multiplicative,
            };
            transparency.add_instance(TransparentInstance::new(i as u32, pos, mode));
        }

        b.iter(|| {
            // Update transparency
            transparency.update(camera_pos);

            // Apply weather modifications
            let terrain_mod = weather.get_terrain_color_modifier();
            let light_atten = weather.get_light_attenuation();

            // Get sorted instances
            let sorted_count = transparency.sorted_instances().len();

            black_box((sorted_count, terrain_mod, light_atten))
        })
    });

    group.finish();
}

// ============================================================================
// CRITERION CONFIGURATION
// ============================================================================

criterion_group!(
    transparency_benches,
    bench_transparency_manager,
    bench_depth_sorting,
    bench_blend_mode_filtering,
);

criterion_group!(
    environment_benches,
    bench_time_of_day,
    bench_weather_system,
    bench_weather_particles,
);

criterion_group!(msaa_benches, bench_msaa,);

criterion_group!(combined_benches, bench_full_environment_frame,);

criterion_main!(
    transparency_benches,
    environment_benches,
    msaa_benches,
    combined_benches,
);
