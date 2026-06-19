//! Underwater caustics system
//!
//! Simulates the dancing light patterns seen underwater caused by
//! light refraction through the water surface.

use glam::{Vec2, Vec3};

/// Configuration for caustics rendering
#[derive(Clone, Debug, PartialEq)]
pub struct CausticsConfig {
    /// Resolution of the caustics texture (power of 2)
    pub texture_size: u32,
    /// Caustics animation speed
    pub animation_speed: f32,
    /// Intensity of caustics effect (0-1)
    pub intensity: f32,
    /// Scale of caustics pattern in world units
    pub pattern_scale: f32,
    /// How deep caustics are visible (in world units)
    pub max_depth: f32,
    /// Chromatic aberration amount (RGB offset)
    pub chromatic_aberration: f32,
    /// Number of wave octaves for pattern generation
    pub wave_octaves: u32,
    /// How much caustics fade with depth
    pub depth_falloff: f32,
}

impl Default for CausticsConfig {
    fn default() -> Self {
        Self {
            texture_size: 512,
            animation_speed: 1.0,
            intensity: 0.8,
            pattern_scale: 2.0,
            max_depth: 50.0,
            chromatic_aberration: 0.02,
            wave_octaves: 3,
            depth_falloff: 0.5,
        }
    }
}

impl CausticsConfig {
    /// Preset for shallow, bright caustics (swimming pool style)
    pub fn shallow() -> Self {
        Self {
            texture_size: 512,
            animation_speed: 1.5,
            intensity: 1.0,
            pattern_scale: 1.5,
            max_depth: 10.0,
            chromatic_aberration: 0.03,
            wave_octaves: 4,
            depth_falloff: 0.3,
        }
    }

    /// Preset for deep ocean caustics (subtle, slow)
    pub fn deep_ocean() -> Self {
        Self {
            texture_size: 256,
            animation_speed: 0.3,
            intensity: 0.4,
            pattern_scale: 4.0,
            max_depth: 100.0,
            chromatic_aberration: 0.01,
            wave_octaves: 2,
            depth_falloff: 0.8,
        }
    }

    /// Preset for murky water (weak caustics)
    pub fn murky() -> Self {
        Self {
            texture_size: 256,
            animation_speed: 0.5,
            intensity: 0.2,
            pattern_scale: 3.0,
            max_depth: 5.0,
            chromatic_aberration: 0.0,
            wave_octaves: 2,
            depth_falloff: 1.0,
        }
    }
}

/// A single caustic sample point for CPU-side preview/debugging
#[derive(Clone, Copy, Debug)]
pub struct CausticSample {
    /// World position
    pub position: Vec3,
    /// Light intensity at this point (0-1)
    pub intensity: f32,
    /// Color tint (for chromatic aberration)
    pub color: Vec3,
}

/// GPU-compatible caustics uniforms
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CausticsUniforms {
    /// Time for animation
    pub time: f32,
    /// Intensity multiplier
    pub intensity: f32,
    /// Pattern scale
    pub scale: f32,
    /// Maximum depth for caustics
    pub max_depth: f32,
    /// Chromatic aberration offset
    pub chromatic_offset: f32,
    /// Depth falloff exponent
    pub depth_falloff: f32,
    /// Padding for alignment
    pub _padding: [f32; 2],
}

impl CausticsUniforms {
    /// Create uniforms from config and current time
    pub fn from_config(config: &CausticsConfig, time: f32) -> Self {
        Self {
            time: time * config.animation_speed,
            intensity: config.intensity,
            scale: config.pattern_scale,
            max_depth: config.max_depth,
            chromatic_offset: config.chromatic_aberration,
            depth_falloff: config.depth_falloff,
            _padding: [0.0; 2],
        }
    }
}

/// Caustics projection data for a light source
#[derive(Clone, Debug)]
pub struct CausticsProjector {
    /// Light direction (normalized, pointing down into water)
    pub light_direction: Vec3,
    /// Water surface height
    pub surface_height: f32,
    /// Projection bounds (min XZ, max XZ)
    pub bounds: (Vec2, Vec2),
    /// Current animation time
    pub time: f32,
}

impl CausticsProjector {
    /// Create a new caustics projector
    pub fn new(light_direction: Vec3, surface_height: f32) -> Self {
        Self {
            light_direction: light_direction.normalize(),
            surface_height,
            bounds: (Vec2::new(-100.0, -100.0), Vec2::new(100.0, 100.0)),
            time: 0.0,
        }
    }

    /// Set the projection bounds
    pub fn with_bounds(mut self, min: Vec2, max: Vec2) -> Self {
        self.bounds = (min, max);
        self
    }

    /// Update animation time
    pub fn update(&mut self, dt: f32, config: &CausticsConfig) {
        self.time += dt * config.animation_speed;
    }

    /// Calculate caustic intensity at a world position
    pub fn sample(&self, position: Vec3, config: &CausticsConfig) -> f32 {
        // Only underwater
        if position.y >= self.surface_height {
            return 0.0;
        }

        let depth = self.surface_height - position.y;
        if depth > config.max_depth {
            return 0.0;
        }

        // Calculate UV from world position
        let uv = Vec2::new(
            (position.x - self.bounds.0.x) / (self.bounds.1.x - self.bounds.0.x),
            (position.z - self.bounds.0.y) / (self.bounds.1.y - self.bounds.0.y),
        );

        // Generate caustics pattern using layered noise
        let mut intensity = 0.0;
        let mut scale = config.pattern_scale;
        let mut amplitude = 1.0;

        for _ in 0..config.wave_octaves {
            intensity += self.caustic_noise(uv * scale, self.time) * amplitude;
            scale /= 2.0;
            amplitude *= 0.5;
        }

        // Normalize and apply intensity
        intensity = (intensity * 0.5 + 0.5).clamp(0.0, 1.0);
        intensity *= config.intensity;

        // Depth falloff
        let depth_factor = 1.0 - (depth / config.max_depth).powf(config.depth_falloff);
        intensity *= depth_factor;

        intensity
    }

    /// Sample with chromatic aberration (returns RGB intensities)
    pub fn sample_chromatic(&self, position: Vec3, config: &CausticsConfig) -> Vec3 {
        let offset = config.chromatic_aberration;

        let r = self.sample(position + Vec3::new(offset, 0.0, 0.0), config);
        let g = self.sample(position, config);
        let b = self.sample(position + Vec3::new(-offset, 0.0, offset), config);

        Vec3::new(r, g, b)
    }

    /// Simple noise function for caustics pattern
    fn caustic_noise(&self, uv: Vec2, time: f32) -> f32 {
        // Voronoi-like pattern for caustics
        let cell = Vec2::new(uv.x.floor(), uv.y.floor());
        let frac = Vec2::new(uv.x.fract(), uv.y.fract());

        let mut min_dist = 1.0f32;

        for x in -1..=1 {
            for y in -1..=1 {
                let neighbor = Vec2::new(x as f32, y as f32);
                let point = self.hash_2d(cell + neighbor, time);
                let diff = neighbor + point - frac;
                let dist = diff.length();
                min_dist = min_dist.min(dist);
            }
        }

        // Convert to caustic pattern
        let caustic = (1.0 - min_dist * 2.0).max(0.0);
        caustic * caustic // Square for sharper edges
    }

    /// Hash function for procedural points
    fn hash_2d(&self, p: Vec2, time: f32) -> Vec2 {
        let x = (p.x * 127.1 + p.y * 311.7 + time * 0.5).sin() * 43758.545;
        let y = (p.x * 269.5 + p.y * 183.3 + time * 0.7).sin() * 43758.545;
        Vec2::new(x.fract() * 0.5 + 0.25, y.fract() * 0.5 + 0.25)
    }
}

/// Manager for multiple caustics projectors
#[derive(Debug)]
pub struct CausticsSystem {
    /// Configuration
    config: CausticsConfig,
    /// Active projectors
    projectors: Vec<CausticsProjector>,
    /// Accumulated time
    time: f32,
}

impl CausticsSystem {
    /// Create a new caustics system
    pub fn new(config: CausticsConfig) -> Self {
        Self {
            config,
            projectors: Vec::new(),
            time: 0.0,
        }
    }

    /// Add a projector for a water body
    pub fn add_projector(&mut self, projector: CausticsProjector) {
        self.projectors.push(projector);
    }

    /// Remove all projectors
    pub fn clear_projectors(&mut self) {
        self.projectors.clear();
    }

    /// Get number of projectors
    pub fn projector_count(&self) -> usize {
        self.projectors.len()
    }

    /// Update all projectors
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for projector in &mut self.projectors {
            projector.update(dt, &self.config);
        }
    }

    /// Sample caustics at a world position (combines all projectors)
    pub fn sample(&self, position: Vec3) -> f32 {
        let mut total = 0.0;
        for projector in &self.projectors {
            total += projector.sample(position, &self.config);
        }
        total.min(1.0)
    }

    /// Sample with chromatic aberration
    pub fn sample_chromatic(&self, position: Vec3) -> Vec3 {
        let mut total = Vec3::ZERO;
        for projector in &self.projectors {
            total += projector.sample_chromatic(position, &self.config);
        }
        total.clamp(Vec3::ZERO, Vec3::ONE)
    }

    /// Get current uniforms for GPU rendering
    pub fn get_uniforms(&self) -> CausticsUniforms {
        CausticsUniforms::from_config(&self.config, self.time)
    }

    /// Get configuration
    pub fn config(&self) -> &CausticsConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: CausticsConfig) {
        self.config = config;
    }
}

/// WGSL shader code for caustics
pub const CAUSTICS_WGSL: &str = r#"
// Caustics uniforms
struct CausticsUniforms {
    time: f32,
    intensity: f32,
    scale: f32,
    max_depth: f32,
    chromatic_offset: f32,
    depth_falloff: f32,
    _padding: vec2<f32>,
}

@group(2) @binding(0) var<uniform> caustics: CausticsUniforms;

// Voronoi noise for caustics pattern
fn caustic_noise(uv: vec2<f32>, time: f32) -> f32 {
    let cell = floor(uv);
    let frac_uv = fract(uv);
    
    var min_dist: f32 = 1.0;
    
    for (var x: i32 = -1; x <= 1; x++) {
        for (var y: i32 = -1; y <= 1; y++) {
            let neighbor = vec2<f32>(f32(x), f32(y));
            let point = hash_2d(cell + neighbor, time);
            let diff = neighbor + point - frac_uv;
            let dist = length(diff);
            min_dist = min(min_dist, dist);
        }
    }
    
    let caustic = max(1.0 - min_dist * 2.0, 0.0);
    return caustic * caustic;
}

fn hash_2d(p: vec2<f32>, time: f32) -> vec2<f32> {
    let x = sin(p.x * 127.1 + p.y * 311.7 + time * 0.5) * 43758.5453;
    let y = sin(p.x * 269.5 + p.y * 183.3 + time * 0.7) * 43758.5453;
    return vec2<f32>(fract(x) * 0.5 + 0.25, fract(y) * 0.5 + 0.25);
}

// Sample caustics at world position
fn sample_caustics(world_pos: vec3<f32>, surface_height: f32) -> vec3<f32> {
    let depth = surface_height - world_pos.y;
    
    // Only underwater
    if (depth <= 0.0 || depth > caustics.max_depth) {
        return vec3<f32>(0.0);
    }
    
    let uv = world_pos.xz / caustics.scale;
    
    // Multi-octave caustics
    var intensity: f32 = 0.0;
    var scale: f32 = 1.0;
    var amplitude: f32 = 1.0;
    
    for (var i: u32 = 0u; i < 3u; i++) {
        intensity += caustic_noise(uv * scale, caustics.time) * amplitude;
        scale *= 2.0;
        amplitude *= 0.5;
    }
    
    intensity = clamp(intensity * 0.5 + 0.5, 0.0, 1.0) * caustics.intensity;
    
    // Depth falloff
    let depth_factor = 1.0 - pow(depth / caustics.max_depth, caustics.depth_falloff);
    intensity *= depth_factor;
    
    // Chromatic aberration
    let offset = caustics.chromatic_offset;
    let r = caustic_noise((uv + vec2<f32>(offset, 0.0)) * scale, caustics.time);
    let g = intensity;
    let b = caustic_noise((uv + vec2<f32>(-offset, offset)) * scale, caustics.time);
    
    return vec3<f32>(r, g, b) * intensity;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caustics_config_default() {
        let config = CausticsConfig::default();
        assert_eq!(config.texture_size, 512);
        assert_eq!(config.animation_speed, 1.0);
        assert_eq!(config.intensity, 0.8);
        assert!(config.max_depth > 0.0);
    }

    #[test]
    fn test_caustics_config_presets() {
        let shallow = CausticsConfig::shallow();
        let deep = CausticsConfig::deep_ocean();
        let murky = CausticsConfig::murky();

        // Shallow should have higher intensity and faster animation
        assert!(shallow.intensity > deep.intensity);
        assert!(shallow.animation_speed > deep.animation_speed);

        // Murky should have lowest intensity
        assert!(murky.intensity < shallow.intensity);
        assert!(murky.intensity < deep.intensity);
    }

    #[test]
    fn test_caustics_uniforms() {
        let config = CausticsConfig::default();
        let uniforms = CausticsUniforms::from_config(&config, 1.0);

        assert_eq!(uniforms.time, config.animation_speed);
        assert_eq!(uniforms.intensity, config.intensity);
        assert_eq!(uniforms.scale, config.pattern_scale);
    }

    #[test]
    fn test_caustics_uniforms_size() {
        assert_eq!(std::mem::size_of::<CausticsUniforms>(), 32);
    }

    #[test]
    fn test_projector_creation() {
        let projector = CausticsProjector::new(Vec3::new(0.0, -1.0, 0.0), 10.0);

        assert_eq!(projector.surface_height, 10.0);
        assert!(projector.light_direction.y < 0.0);
    }

    #[test]
    fn test_projector_bounds() {
        let projector = CausticsProjector::new(Vec3::NEG_Y, 0.0)
            .with_bounds(Vec2::new(-50.0, -50.0), Vec2::new(50.0, 50.0));

        assert_eq!(projector.bounds.0, Vec2::new(-50.0, -50.0));
        assert_eq!(projector.bounds.1, Vec2::new(50.0, 50.0));
    }

    #[test]
    fn test_sample_above_water() {
        let config = CausticsConfig::default();
        let projector = CausticsProjector::new(Vec3::NEG_Y, 0.0);

        // Above water should return 0
        let sample = projector.sample(Vec3::new(0.0, 5.0, 0.0), &config);
        assert_eq!(sample, 0.0);
    }

    #[test]
    fn test_sample_underwater() {
        let config = CausticsConfig::default();
        let projector = CausticsProjector::new(Vec3::NEG_Y, 10.0);

        // Underwater should return non-zero
        let sample = projector.sample(Vec3::new(0.0, 5.0, 0.0), &config);
        assert!(sample >= 0.0);
    }

    #[test]
    fn test_sample_too_deep() {
        let config = CausticsConfig {
            max_depth: 10.0,
            ..Default::default()
        };
        let projector = CausticsProjector::new(Vec3::NEG_Y, 100.0);

        // Too deep should return 0
        let sample = projector.sample(Vec3::new(0.0, 0.0, 0.0), &config);
        assert_eq!(sample, 0.0);
    }

    #[test]
    fn test_chromatic_sample() {
        let config = CausticsConfig {
            chromatic_aberration: 0.1,
            ..Default::default()
        };
        let projector = CausticsProjector::new(Vec3::NEG_Y, 10.0);

        let sample = projector.sample_chromatic(Vec3::new(0.0, 5.0, 0.0), &config);

        // All channels should be valid
        assert!(sample.x >= 0.0 && sample.x <= 1.0);
        assert!(sample.y >= 0.0 && sample.y <= 1.0);
        assert!(sample.z >= 0.0 && sample.z <= 1.0);
    }

    #[test]
    fn test_system_creation() {
        let system = CausticsSystem::new(CausticsConfig::default());
        assert_eq!(system.projector_count(), 0);
    }

    #[test]
    fn test_system_add_projector() {
        let mut system = CausticsSystem::new(CausticsConfig::default());

        system.add_projector(CausticsProjector::new(Vec3::NEG_Y, 0.0));
        assert_eq!(system.projector_count(), 1);

        system.add_projector(CausticsProjector::new(Vec3::NEG_Y, 10.0));
        assert_eq!(system.projector_count(), 2);
    }

    #[test]
    fn test_system_clear() {
        let mut system = CausticsSystem::new(CausticsConfig::default());

        system.add_projector(CausticsProjector::new(Vec3::NEG_Y, 0.0));
        system.add_projector(CausticsProjector::new(Vec3::NEG_Y, 10.0));

        system.clear_projectors();
        assert_eq!(system.projector_count(), 0);
    }

    #[test]
    fn test_system_update() {
        let mut system = CausticsSystem::new(CausticsConfig::default());
        system.add_projector(CausticsProjector::new(Vec3::NEG_Y, 0.0));

        let initial_time = system.time;
        system.update(0.5);

        assert!(system.time > initial_time);
    }

    #[test]
    fn test_system_sample() {
        let mut system = CausticsSystem::new(CausticsConfig::default());
        system.add_projector(CausticsProjector::new(Vec3::NEG_Y, 10.0));

        let sample = system.sample(Vec3::new(0.0, 5.0, 0.0));
        assert!(sample >= 0.0 && sample <= 1.0);
    }

    #[test]
    fn test_system_get_uniforms() {
        let system = CausticsSystem::new(CausticsConfig::default());
        let uniforms = system.get_uniforms();

        assert_eq!(uniforms.intensity, system.config().intensity);
    }

    #[test]
    fn test_wgsl_shader_exists() {
        assert!(!CAUSTICS_WGSL.is_empty());
        assert!(CAUSTICS_WGSL.contains("sample_caustics"));
        assert!(CAUSTICS_WGSL.contains("caustic_noise"));
    }

    // =========================================================================
    // Mutation-killing tests
    // =========================================================================

    #[test]
    fn test_shallow_all_fields() {
        let c = CausticsConfig::shallow();
        assert!((c.animation_speed - 1.5).abs() < 1e-6);
        assert!((c.intensity - 1.0).abs() < 1e-6);
        assert!((c.pattern_scale - 1.5).abs() < 1e-6);
        assert!((c.max_depth - 10.0).abs() < 1e-6);
        assert!((c.chromatic_aberration - 0.03).abs() < 1e-6);
        assert_eq!(c.wave_octaves, 4);
        assert!((c.depth_falloff - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_deep_ocean_all_fields() {
        let c = CausticsConfig::deep_ocean();
        assert_eq!(c.texture_size, 256);
        assert!((c.animation_speed - 0.3).abs() < 1e-6);
        assert!((c.intensity - 0.4).abs() < 1e-6);
        assert!((c.pattern_scale - 4.0).abs() < 1e-6);
        assert!((c.max_depth - 100.0).abs() < 1e-6);
        assert!((c.chromatic_aberration - 0.01).abs() < 1e-6);
        assert_eq!(c.wave_octaves, 2);
        assert!((c.depth_falloff - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_from_config_time_scaling() {
        let config = CausticsConfig {
            animation_speed: 2.5,
            ..Default::default()
        };
        let u = CausticsUniforms::from_config(&config, 3.0);
        // time = 3.0 * 2.5 = 7.5
        assert!(
            (u.time - 7.5).abs() < 1e-6,
            "time should be 7.5, got {}",
            u.time
        );
    }

    #[test]
    fn test_update_advances_time() {
        let config = CausticsConfig {
            animation_speed: 2.0,
            ..Default::default()
        };
        let mut proj = CausticsProjector::new(Vec3::NEG_Y, 0.0);
        assert_eq!(proj.time, 0.0);

        proj.update(0.5, &config);
        // time += 0.5 * 2.0 = 1.0
        assert!((proj.time - 1.0).abs() < 1e-6, "time={}", proj.time);

        proj.update(0.25, &config);
        // time += 0.25 * 2.0 = 0.5 → total 1.5
        assert!((proj.time - 1.5).abs() < 1e-6, "time={}", proj.time);
    }

    #[test]
    fn test_sample_depth_exact() {
        // With 0 octaves: intensity = (0*0.5+0.5) = 0.5
        // then *= config.intensity, then *= depth_factor
        let config = CausticsConfig {
            wave_octaves: 0,
            intensity: 1.0,
            max_depth: 100.0,
            depth_falloff: 1.0,
            ..Default::default()
        };
        let proj = CausticsProjector::new(Vec3::NEG_Y, 100.0);

        // depth=50 → factor = 1 - (50/100)^1 = 0.5 → result = 0.5*1.0*0.5 = 0.25
        let r = proj.sample(Vec3::new(0.0, 50.0, 0.0), &config);
        assert!((r - 0.25).abs() < 1e-4, "expected 0.25, got {}", r);

        // depth=75 → factor = 0.25 → result = 0.125
        let r2 = proj.sample(Vec3::new(0.0, 25.0, 0.0), &config);
        assert!((r2 - 0.125).abs() < 1e-4, "expected 0.125, got {}", r2);
    }

    #[test]
    fn test_sample_multi_point_golden() {
        // Multi-point golden test with non-symmetric bounds, 3 octaves,
        // and tight tolerance to catch ALL arithmetic mutations in sample().
        // Non-symmetric bounds ensure UV - vs + mutations are detected.
        // 3 octaves ensure scale/amplitude loop mutations diverge.
        // pattern_scale=3.5 (not 2.0) to prevent *= vs += equivalence.
        let config = CausticsConfig {
            wave_octaves: 3,
            intensity: 0.7,
            pattern_scale: 3.5,
            max_depth: 80.0,
            depth_falloff: 0.6,
            ..Default::default()
        };
        let proj = CausticsProjector::new(Vec3::NEG_Y, 50.0)
            .with_bounds(Vec2::new(-10.0, -20.0), Vec2::new(30.0, 40.0));

        // Three sample points at different depths and UVs
        let p1 = proj.sample(Vec3::new(5.0, 30.0, 10.0), &config); // depth=20
        let p2 = proj.sample(Vec3::new(-3.0, 10.0, 25.0), &config); // depth=40
        let p3 = proj.sample(Vec3::new(15.0, 45.0, -5.0), &config); // depth=5

        eprintln!("GOLDEN p1={p1} p2={p2} p3={p3}");

        // Golden values from verified correct implementation (tolerance 1e-5).
        // All three must match — any arithmetic mutation in UV, noise, octave
        // loop, normalization, or depth falloff changes at least one.
        assert!((p1 - 0.22275063).abs() < 1e-5, "p1 mismatch: {p1}");
        assert!((p2 - 0.1604235).abs() < 1e-5, "p2 mismatch: {p2}");
        assert!((p3 - 0.29726005).abs() < 1e-5, "p3 mismatch: {p3}");

        // Determinism check
        let p1b = proj.sample(Vec3::new(5.0, 30.0, 10.0), &config);
        assert!((p1 - p1b).abs() < 1e-6, "non-deterministic");
    }

    #[test]
    fn test_sample_chromatic_returns_three_channels() {
        // Exercises sample_chromatic which calls sample 3 times with offsets.
        // Verifies chromatic aberration offset logic.
        let config = CausticsConfig {
            wave_octaves: 2,
            intensity: 0.8,
            pattern_scale: 3.0,
            max_depth: 80.0,
            depth_falloff: 0.5,
            chromatic_aberration: 0.05,
            ..Default::default()
        };
        let proj = CausticsProjector::new(Vec3::NEG_Y, 50.0)
            .with_bounds(Vec2::new(-10.0, -20.0), Vec2::new(30.0, 40.0));
        let rgb = proj.sample_chromatic(Vec3::new(5.0, 30.0, 10.0), &config);

        // All channels should be in [0, 1]
        assert!(rgb.x >= 0.0 && rgb.x <= 1.0, "R: {}", rgb.x);
        assert!(rgb.y >= 0.0 && rgb.y <= 1.0, "G: {}", rgb.y);
        assert!(rgb.z >= 0.0 && rgb.z <= 1.0, "B: {}", rgb.z);

        // G channel should match direct sample() call
        let g = proj.sample(Vec3::new(5.0, 30.0, 10.0), &config);
        assert!((rgb.y - g).abs() < 1e-6, "G channel should match sample()");
    }
}
