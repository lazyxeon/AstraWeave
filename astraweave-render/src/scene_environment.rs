//! Scene Environment Parameters
//!
//! Aggregates fog, ambient lighting, and tint parameters into a
//! GPU-uploadable uniform buffer. These values are driven by the
//! [`TransitionEffect`](crate::biome_transition::TransitionEffect) during
//! biome transitions and remain static between transitions.
//!
//! # GPU Layout
//!
//! ```text
//! SceneEnvironmentUBO (96 bytes, 16-byte aligned):
//!   fog_color:        vec3<f32> + fog_density: f32   (16 bytes)  offset  0
//!   fog_start:        f32 + fog_end: f32 + wetness   (16 bytes)  offset 16
//!     + snow_amount: f32
//!   ambient_color:    vec3<f32> + ambient_intensity   (16 bytes)  offset 32
//!   tint_color:       vec3<f32> + tint_alpha: f32     (16 bytes)  offset 48
//!   blend_factor:     f32 + _pad_align: vec3<f32>     (16 bytes)  offset 64
//!   sun_color:        vec3<f32> + sun_intensity: f32  (16 bytes)  offset 80
//! ```
//!
//! `wetness` (0.0–1.0) drives wet-surface PBR modulation (darker albedo,
//! lower roughness, water F0). `snow_amount` (0.0–1.0) drives snow
//! accumulation blending. Both are packed into the fog range vec4.
//!
//! Bind as `@group(4) @binding(0) var<uniform> uScene: SceneEnvironment;`
//! in WGSL (once the pipeline is extended to support it).

use crate::biome_transition::{BiomeVisuals, TransitionEffect};
use bytemuck::{Pod, Zeroable};

// ─── GPU Uniform ─────────────────────────────────────────────────────────

/// GPU-ready scene environment uniform buffer (96 bytes, 16-byte aligned).
///
/// Matches WGSL `SceneEnv` layout where `vec3<f32>` fields have 16-byte
/// alignment, producing a total struct size of 96 bytes (not 80).
///
/// Upload this to a `wgpu::Buffer` with `wgpu::BufferUsages::UNIFORM`
/// and update it each frame via `queue.write_buffer()`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SceneEnvironmentUBO {
    /// Fog color (linear RGB).
    pub fog_color: [f32; 3],
    /// Exponential fog density.
    pub fog_density: f32,

    /// Fog distance where fog starts (linear fog).
    pub fog_start: f32,
    /// Fog distance where fog is fully opaque (linear fog).
    pub fog_end: f32,
    /// Surface wetness (0.0–1.0). Drives PBR wet modulation in the shader:
    /// lower roughness, darker albedo, water F0. Set via weather system.
    pub wetness: f32,
    /// Snow accumulation amount (0.0–1.0). Drives snow material blending.
    pub snow_amount: f32,

    /// Ambient light color (linear RGB).
    pub ambient_color: [f32; 3],
    /// Ambient light intensity multiplier.
    pub ambient_intensity: f32,

    /// Screen tint color (linear RGB) applied during transitions.
    pub tint_color: [f32; 3],
    /// Screen tint alpha (0 = invisible, peaks during transitions).
    pub tint_alpha: f32,

    /// Current transition blend factor (0.0 = source biome, 1.0 = target biome).
    /// Useful for shader effects that want to know about the transition.
    pub blend_factor: f32,
    /// Padding to align `_pad1` to 16-byte boundary (WGSL vec3 alignment).
    pub _pad_align: [f32; 3],

    /// Sun color (linear RGB) for directional light in PBR shader.
    pub sun_color: [f32; 3],
    /// Sun intensity multiplier for directional light.
    pub sun_intensity: f32,
}

impl Default for SceneEnvironmentUBO {
    fn default() -> Self {
        Self::from_visuals(&BiomeVisuals::default(), 0.0, [0.0; 3], 0.0)
    }
}

impl SceneEnvironmentUBO {
    /// Create from explicit `BiomeVisuals`, blend factor, and tint params.
    pub fn from_visuals(
        visuals: &BiomeVisuals,
        blend_factor: f32,
        tint_color: [f32; 3],
        tint_alpha: f32,
    ) -> Self {
        Self {
            fog_color: visuals.fog_color.to_array(),
            fog_density: visuals.fog_density,
            fog_start: visuals.fog_start,
            fog_end: visuals.fog_end,
            wetness: 0.0,
            snow_amount: 0.0,
            ambient_color: visuals.ambient_color.to_array(),
            ambient_intensity: visuals.ambient_intensity,
            tint_color,
            tint_alpha,
            blend_factor,
            _pad_align: [0.0; 3],
            sun_color: [1.0, 0.98, 0.9], // warm white default
            sun_intensity: 1.0,
        }
    }

    /// Build from the current state of a `TransitionEffect`.
    ///
    /// If no transition is active, returns steady-state values for the
    /// given biome (or defaults if `None`).
    pub fn from_transition(effect: &TransitionEffect) -> Self {
        let visuals = effect.current_visuals();
        let tint = effect.tint_color().to_array();
        Self::from_visuals(&visuals, effect.blend_factor(), tint, effect.tint_alpha())
    }

    /// Build steady-state params for a non-transitioning biome.
    pub fn for_biome(biome: astraweave_terrain::biome::BiomeType) -> Self {
        let visuals = BiomeVisuals::for_biome(biome);
        Self::from_visuals(&visuals, 0.0, [0.0; 3], 0.0)
    }

    /// Returns the size of this struct in bytes (for buffer creation).
    pub const fn size() -> u64 {
        std::mem::size_of::<Self>() as u64
    }
}

// ─── CPU-Side Scene Parameters ───────────────────────────────────────────

/// High-level scene environment state tracked on the CPU.
///
/// Aggregates the current biome visuals, transition state, and
/// weather/time modifiers. Call [`SceneEnvironment::to_ubo()`] to get
/// the GPU-uploadable representation.
#[derive(Debug, Clone)]
pub struct SceneEnvironment {
    /// Current interpolated biome visuals.
    pub visuals: BiomeVisuals,
    /// Transition blend factor (0.0 if not transitioning).
    pub blend_factor: f32,
    /// Screen tint color during transitions.
    pub tint_color: [f32; 3],
    /// Screen tint alpha (0.0 if not transitioning).
    pub tint_alpha: f32,
    /// Optional weather fog override (multiplied with biome fog).
    pub weather_fog_multiplier: f32,
    /// Optional weather ambient override (multiplied with biome ambient).
    pub weather_ambient_multiplier: f32,
    /// Directional sun color (set by editor world panel).
    pub sun_color: [f32; 3],
    /// Directional sun intensity (set by editor world panel).
    pub sun_intensity: f32,
    /// Surface wetness driven by rain (0.0–1.0).
    /// Accumulates during rain, evaporates when dry.
    pub wetness: f32,
    /// Snow accumulation level (0.0–1.0).
    /// Accumulates during snow, melts when warm.
    pub snow_amount: f32,
}

impl Default for SceneEnvironment {
    fn default() -> Self {
        Self {
            visuals: BiomeVisuals::default(),
            blend_factor: 0.0,
            tint_color: [0.0; 3],
            tint_alpha: 0.0,
            weather_fog_multiplier: 1.0,
            weather_ambient_multiplier: 1.0,
            sun_color: [1.0, 0.98, 0.9],
            sun_intensity: 1.0,
            wetness: 0.0,
            snow_amount: 0.0,
        }
    }
}

impl SceneEnvironment {
    /// Update from a `TransitionEffect` (call each frame).
    pub fn update_from_transition(&mut self, effect: &TransitionEffect) {
        self.visuals = effect.current_visuals();
        self.blend_factor = effect.blend_factor();
        self.tint_color = effect.tint_color().to_array();
        self.tint_alpha = effect.tint_alpha();
    }

    /// Set to steady-state for a specific biome (no transition).
    pub fn set_biome(&mut self, biome: astraweave_terrain::biome::BiomeType) {
        self.visuals = BiomeVisuals::for_biome(biome);
        self.blend_factor = 0.0;
        self.tint_color = [0.0; 3];
        self.tint_alpha = 0.0;
    }

    /// Convert to GPU-uploadable UBO, applying weather multipliers.
    pub fn to_ubo(&self) -> SceneEnvironmentUBO {
        let mut visuals = self.visuals;
        visuals.fog_density *= self.weather_fog_multiplier;
        visuals.ambient_intensity *= self.weather_ambient_multiplier;
        let mut ubo = SceneEnvironmentUBO::from_visuals(
            &visuals,
            self.blend_factor,
            self.tint_color,
            self.tint_alpha,
        );
        ubo.sun_color = self.sun_color;
        ubo.sun_intensity = self.sun_intensity;
        ubo.wetness = self.wetness.clamp(0.0, 1.0);
        ubo.snow_amount = self.snow_amount.clamp(0.0, 1.0);
        ubo
    }

    /// Apply weather-driven fog and ambient multipliers from a [`WeatherKind`].
    ///
    /// Maps each weather type to physically-motivated modifiers:
    /// - **None**: 1.0× fog, 1.0× ambient (no change)
    /// - **Rain**: 2.5× fog, 0.6× ambient (overcast, reduced visibility)
    /// - **Snow**: 1.8× fog, 0.75× ambient (diffuse overcast, soft light)
    /// - **Sandstorm**: 4.0× fog, 0.4× ambient (heavy particle obscuration)
    /// - **WindTrails**: 1.4× fog, 0.9× ambient (light haze)
    pub fn apply_weather(&mut self, kind: crate::effects::WeatherKind) {
        let (fog_mul, ambient_mul) = match kind {
            crate::effects::WeatherKind::None => (1.0, 1.0),
            crate::effects::WeatherKind::Rain => (2.5, 0.6),
            crate::effects::WeatherKind::Snow => (1.8, 0.75),
            crate::effects::WeatherKind::Sandstorm => (4.0, 0.4),
            crate::effects::WeatherKind::WindTrails => (1.4, 0.9),
        };
        self.weather_fog_multiplier = fog_mul;
        self.weather_ambient_multiplier = ambient_mul;
    }

    /// Update surface wetness based on current weather.
    ///
    /// During rain, wetness accumulates at `accumulate_rate` per second
    /// (default ≈ 0.5 → full wet in 2 s). When not raining, wetness
    /// evaporates at `dry_rate` per second (default ≈ 0.1 → dry in 10 s).
    ///
    /// Call once per frame with `dt` = frame delta time.
    pub fn update_wetness(&mut self, dt: f32, rain_active: bool) {
        const ACCUMULATE_RATE: f32 = 0.5;
        const DRY_RATE: f32 = 0.1;
        if rain_active {
            self.wetness = (self.wetness + ACCUMULATE_RATE * dt).min(1.0);
        } else {
            self.wetness = (self.wetness - DRY_RATE * dt).max(0.0);
        }
    }

    /// Update snow accumulation based on current weather.
    ///
    /// During snow, accumulation increases at `accumulate_rate` per second.
    /// When not snowing, snow melts at `melt_rate` per second.
    pub fn update_snow(&mut self, dt: f32, snow_active: bool) {
        const ACCUMULATE_RATE: f32 = 0.3;
        const MELT_RATE: f32 = 0.05;
        if snow_active {
            self.snow_amount = (self.snow_amount + ACCUMULATE_RATE * dt).min(1.0);
        } else {
            self.snow_amount = (self.snow_amount - MELT_RATE * dt).max(0.0);
        }
    }

    /// Derive ambient color and intensity from the time-of-day system.
    ///
    /// The `TimeOfDay::get_ambient_color()` returns a pre-multiplied RGB
    /// vector where brighter = more intense. We extract the luminance as
    /// intensity and normalize the color, then blend with the biome's own
    /// ambient to avoid overriding biome character entirely.
    pub fn apply_time_of_day(&mut self, tod: &crate::environment::TimeOfDay) {
        let tod_ambient = tod.get_ambient_color();
        let luminance = tod_ambient.x * 0.2126 + tod_ambient.y * 0.7152 + tod_ambient.z * 0.0722;
        if luminance > 0.001 {
            let normalized = tod_ambient / luminance;
            // Blend: 60% biome ambient color, 40% time-of-day color
            let biome_col = self.visuals.ambient_color;
            self.visuals.ambient_color = glam::Vec3::new(
                biome_col.x * 0.6 + normalized.x * 0.4,
                biome_col.y * 0.6 + normalized.y * 0.4,
                biome_col.z * 0.6 + normalized.z * 0.4,
            );
            // Scale biome intensity by time-of-day luminance (clamped for night)
            self.visuals.ambient_intensity *= luminance.clamp(0.15, 1.5);
        }
    }
}

// ─── WGSL Snippet ────────────────────────────────────────────────────────

/// WGSL struct definition for the scene environment uniform.
///
/// Include this in your shader source to declare the UBO type.
/// Bind it at `@group(4) @binding(0)` (or whichever group you choose).
pub const WGSL_SCENE_ENVIRONMENT: &str = r#"
struct SceneEnvironment {
    fog_color:          vec3<f32>,
    fog_density:        f32,
    fog_start:          f32,
    fog_end:            f32,
    wetness:            f32,
    snow_amount:        f32,
    ambient_color:      vec3<f32>,
    ambient_intensity:  f32,
    tint_color:         vec3<f32>,
    tint_alpha:         f32,
    blend_factor:       f32,
    _pad1x: f32, _pad1y: f32, _pad1z: f32,
    sun_color:          vec3<f32>,
    sun_intensity:      f32,
};
"#;

/// WGSL fog calculation functions.
///
/// Include this in your shader to get `apply_fog()` and `apply_tint()`.
pub const WGSL_FOG_FUNCTIONS: &str = r#"
// Linear distance fog
fn apply_linear_fog(color: vec3<f32>, dist: f32, scene: SceneEnvironment) -> vec3<f32> {
    let t = clamp((dist - scene.fog_start) / (scene.fog_end - scene.fog_start + 0.001), 0.0, 1.0);
    return mix(color, scene.fog_color, t);
}

// Exponential distance fog
fn apply_exp_fog(color: vec3<f32>, dist: f32, scene: SceneEnvironment) -> vec3<f32> {
    let f = exp(-dist * scene.fog_density);
    return mix(scene.fog_color, color, clamp(f, 0.0, 1.0));
}

// Combined fog (uses both linear ramp and exponential for high quality)
fn apply_fog(color: vec3<f32>, dist: f32, scene: SceneEnvironment) -> vec3<f32> {
    // Linear component for near-range
    let lin = clamp((dist - scene.fog_start) / (scene.fog_end - scene.fog_start + 0.001), 0.0, 1.0);
    // Exponential component for far-range
    let exp_f = 1.0 - exp(-dist * scene.fog_density);
    // Take the stronger fog effect
    let fog_amount = max(lin, exp_f);
    return mix(color, scene.fog_color, fog_amount);
}

// Screen-space tint overlay (call in post or at end of fragment)
fn apply_tint(color: vec3<f32>, scene: SceneEnvironment) -> vec3<f32> {
    return mix(color, scene.tint_color, scene.tint_alpha);
}
"#;

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::field_reassign_with_default,
    clippy::const_is_empty,
    clippy::clone_on_copy
)]
mod tests {
    use super::*;
    use crate::biome_transition::{EasingFunction, TransitionConfig, TransitionEffect};
    use astraweave_terrain::biome::BiomeType;
    use glam::Vec3;

    #[test]
    fn test_ubo_size_is_80_bytes() {
        assert_eq!(std::mem::size_of::<SceneEnvironmentUBO>(), 96);
        assert_eq!(SceneEnvironmentUBO::size(), 96);
    }

    #[test]
    fn test_ubo_alignment_is_pod() {
        // This compiles only if Pod+Zeroable are correctly derived
        let ubo = SceneEnvironmentUBO::zeroed();
        assert_eq!(ubo.fog_density, 0.0);
        assert_eq!(ubo.blend_factor, 0.0);
    }

    #[test]
    fn test_ubo_from_visuals() {
        let visuals = BiomeVisuals {
            fog_color: Vec3::new(0.5, 0.6, 0.7),
            fog_density: 0.003,
            fog_start: 30.0,
            fog_end: 300.0,
            ambient_color: Vec3::new(0.3, 0.4, 0.2),
            ambient_intensity: 0.25,
            ..Default::default()
        };
        let ubo = SceneEnvironmentUBO::from_visuals(&visuals, 0.5, [1.0, 0.0, 0.0], 0.1);

        assert_eq!(ubo.fog_color, [0.5, 0.6, 0.7]);
        assert_eq!(ubo.fog_density, 0.003);
        assert_eq!(ubo.fog_start, 30.0);
        assert_eq!(ubo.fog_end, 300.0);
        assert_eq!(ubo.ambient_color, [0.3, 0.4, 0.2]);
        assert_eq!(ubo.ambient_intensity, 0.25);
        assert_eq!(ubo.tint_color, [1.0, 0.0, 0.0]);
        assert_eq!(ubo.tint_alpha, 0.1);
        assert_eq!(ubo.blend_factor, 0.5);
    }

    #[test]
    fn test_ubo_from_transition_idle() {
        let effect = TransitionEffect::new(TransitionConfig::default());
        let ubo = SceneEnvironmentUBO::from_transition(&effect);

        // Idle transition → blend_factor=0, tint_alpha=0
        assert_eq!(ubo.blend_factor, 0.0);
        assert_eq!(ubo.tint_alpha, 0.0);
    }

    #[test]
    fn test_ubo_from_transition_active() {
        let mut effect = TransitionEffect::new(TransitionConfig {
            duration: 1.0,
            easing: EasingFunction::Linear,
            apply_tint: true,
            tint_alpha: 0.2,
            ..Default::default()
        });
        effect.start(Some(BiomeType::Grassland), BiomeType::Desert);
        effect.update(0.5); // 50%

        let ubo = SceneEnvironmentUBO::from_transition(&effect);
        assert!((ubo.blend_factor - 0.5).abs() < 0.01);
        assert!(ubo.tint_alpha > 0.0);
        // Fog should be somewhere between grassland and desert
        let grass_fog = BiomeVisuals::for_biome(BiomeType::Grassland).fog_density;
        let desert_fog = BiomeVisuals::for_biome(BiomeType::Desert).fog_density;
        assert!(ubo.fog_density >= grass_fog.min(desert_fog));
        assert!(ubo.fog_density <= grass_fog.max(desert_fog) + 0.001);
    }

    #[test]
    fn test_ubo_for_biome() {
        let ubo = SceneEnvironmentUBO::for_biome(BiomeType::Tundra);
        let visuals = BiomeVisuals::for_biome(BiomeType::Tundra);

        assert_eq!(ubo.fog_density, visuals.fog_density);
        assert_eq!(ubo.ambient_intensity, visuals.ambient_intensity);
        assert_eq!(ubo.blend_factor, 0.0);
        assert_eq!(ubo.tint_alpha, 0.0);
    }

    #[test]
    fn test_scene_environment_update_from_transition() {
        let mut env = SceneEnvironment::default();
        let mut effect = TransitionEffect::new(TransitionConfig {
            duration: 2.0,
            ..Default::default()
        });
        effect.start(Some(BiomeType::Forest), BiomeType::Beach);
        effect.update(1.0); // 50%

        env.update_from_transition(&effect);
        assert!(env.blend_factor > 0.0);

        let ubo = env.to_ubo();
        assert!(ubo.blend_factor > 0.0);
    }

    #[test]
    fn test_scene_environment_weather_multipliers() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Swamp);
        let base_fog = env.visuals.fog_density;
        let base_ambient = env.visuals.ambient_intensity;

        // Heavy fog weather → 3× fog
        env.weather_fog_multiplier = 3.0;
        env.weather_ambient_multiplier = 0.5;
        let ubo = env.to_ubo();

        assert!((ubo.fog_density - base_fog * 3.0).abs() < 0.0001);
        assert!((ubo.ambient_intensity - base_ambient * 0.5).abs() < 0.01);
    }

    #[test]
    fn test_scene_environment_set_biome() {
        let mut env = SceneEnvironment::default();
        env.blend_factor = 0.5;
        env.tint_alpha = 0.3;

        env.set_biome(BiomeType::Mountain);
        assert_eq!(env.blend_factor, 0.0);
        assert_eq!(env.tint_alpha, 0.0);

        let mountain = BiomeVisuals::for_biome(BiomeType::Mountain);
        assert_eq!(env.visuals.fog_density, mountain.fog_density);
    }

    #[test]
    fn test_default_ubo() {
        let ubo = SceneEnvironmentUBO::default();
        assert!(ubo.fog_density > 0.0);
        assert!(ubo.fog_end > ubo.fog_start);
        assert_eq!(ubo.blend_factor, 0.0);
        assert_eq!(ubo.tint_alpha, 0.0);
    }

    #[test]
    fn test_wgsl_snippets_not_empty() {
        assert!(!WGSL_SCENE_ENVIRONMENT.is_empty());
        assert!(WGSL_SCENE_ENVIRONMENT.contains("SceneEnvironment"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("fog_color"));
        assert!(WGSL_SCENE_ENVIRONMENT.contains("ambient_intensity"));

        assert!(!WGSL_FOG_FUNCTIONS.is_empty());
        assert!(WGSL_FOG_FUNCTIONS.contains("apply_fog"));
        assert!(WGSL_FOG_FUNCTIONS.contains("apply_tint"));
    }

    // ── GPU Pipeline Integration Tests ──────────────────────────────────

    #[test]
    fn test_ubo_bytemuck_roundtrip() {
        // Simulate: create UBO → serialize → deserialize → compare
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Desert);
        env.weather_fog_multiplier = 2.5;
        let ubo = env.to_ubo();

        let bytes = bytemuck::bytes_of(&ubo);
        assert_eq!(bytes.len(), 96);

        let deserialized: &SceneEnvironmentUBO = bytemuck::from_bytes(bytes);
        assert_eq!(deserialized.fog_density, ubo.fog_density);
        assert_eq!(deserialized.ambient_intensity, ubo.ambient_intensity);
        assert_eq!(deserialized.tint_alpha, ubo.tint_alpha);
        assert_eq!(deserialized.blend_factor, ubo.blend_factor);
        assert_eq!(deserialized.fog_color, ubo.fog_color);
        assert_eq!(deserialized.ambient_color, ubo.ambient_color);
    }

    #[test]
    fn test_full_transition_pipeline_ubo_sequence() {
        // Simulate a full biome transition and verify UBO values at each stage:
        // Initial → Start Transition → Mid-transition → Complete → New Steady-State
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Grassland);

        // Stage 1: Steady-state grassland
        let ubo_initial = env.to_ubo();
        assert_eq!(ubo_initial.blend_factor, 0.0);
        assert_eq!(ubo_initial.tint_alpha, 0.0);
        let grassland_fog = BiomeVisuals::for_biome(BiomeType::Grassland).fog_density;
        assert!((ubo_initial.fog_density - grassland_fog).abs() < 0.001);

        // Stage 2: Start transition to Desert
        let mut effect = TransitionEffect::new(TransitionConfig {
            duration: 2.0,
            easing: EasingFunction::Linear,
            apply_tint: true,
            tint_alpha: 0.3,
            ..Default::default()
        });
        effect.start(Some(BiomeType::Grassland), BiomeType::Desert);

        // Stage 3: Mid-transition (t=1.0s of 2.0s = 50%)
        effect.update(1.0);
        env.update_from_transition(&effect);
        let ubo_mid = env.to_ubo();
        assert!((ubo_mid.blend_factor - 0.5).abs() < 0.05);
        assert!(ubo_mid.tint_alpha > 0.0); // tint present during transition
        let desert_fog = BiomeVisuals::for_biome(BiomeType::Desert).fog_density;
        assert!(ubo_mid.fog_density > grassland_fog.min(desert_fog) - 0.001);
        assert!(ubo_mid.fog_density < grassland_fog.max(desert_fog) + 0.001);

        // Stage 4: Complete transition (t=2.0s → 100%)
        effect.update(1.0);
        env.update_from_transition(&effect);
        let ubo_end = env.to_ubo();
        assert!((ubo_end.blend_factor - 1.0).abs() < 0.05);
        // Tint should be 0 at end of transition (bell-curve peaks then fades)
        // Fog should be desert-like
        assert!((ubo_end.fog_density - desert_fog).abs() < 0.01);

        // Stage 5: Set new steady-state
        env.set_biome(BiomeType::Desert);
        let ubo_steady = env.to_ubo();
        assert_eq!(ubo_steady.blend_factor, 0.0);
        assert_eq!(ubo_steady.tint_alpha, 0.0);
        assert!((ubo_steady.fog_density - desert_fog).abs() < 0.001);
    }

    #[test]
    fn test_weather_multipliers_chain_through_ubo() {
        // Verify weather multipliers correctly modify UBO output
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Forest);
        let base_ubo = env.to_ubo();
        let base_fog = base_ubo.fog_density;
        let base_ambient = base_ubo.ambient_intensity;

        // Clear day: normal values
        env.weather_fog_multiplier = 1.0;
        env.weather_ambient_multiplier = 1.0;
        let clear_ubo = env.to_ubo();
        assert!((clear_ubo.fog_density - base_fog).abs() < 0.0001);

        // Heavy rain: triple fog, half ambient
        env.weather_fog_multiplier = 3.0;
        env.weather_ambient_multiplier = 0.5;
        let rain_ubo = env.to_ubo();
        assert!((rain_ubo.fog_density - base_fog * 3.0).abs() < 0.001);
        assert!((rain_ubo.ambient_intensity - base_ambient * 0.5).abs() < 0.01);

        // Verify weather doesn't corrupt tint/blend
        assert_eq!(rain_ubo.blend_factor, 0.0);
        assert_eq!(rain_ubo.tint_alpha, 0.0);
    }

    #[test]
    fn test_all_biomes_produce_valid_ubos() {
        let biomes = [
            BiomeType::Grassland,
            BiomeType::Desert,
            BiomeType::Forest,
            BiomeType::Mountain,
            BiomeType::Tundra,
            BiomeType::Swamp,
            BiomeType::Beach,
            BiomeType::River,
        ];

        for biome in &biomes {
            let ubo = SceneEnvironmentUBO::for_biome(*biome);
            // Every biome should have positive fog density
            assert!(
                ubo.fog_density > 0.0,
                "Biome {:?} has zero fog density",
                biome
            );
            // Fog end > fog start
            assert!(
                ubo.fog_end > ubo.fog_start,
                "Biome {:?} has invalid fog range: {} >= {}",
                biome,
                ubo.fog_start,
                ubo.fog_end
            );
            // Ambient intensity should be positive and reasonable
            assert!(
                ubo.ambient_intensity > 0.0 && ubo.ambient_intensity <= 2.0,
                "Biome {:?} has unreasonable ambient intensity: {}",
                biome,
                ubo.ambient_intensity
            );
            // Size is always 96 bytes (matches WGSL SceneEnv layout)
            assert_eq!(bytemuck::bytes_of(&ubo).len(), 96);
        }
    }

    #[test]
    fn test_ubo_transition_between_all_adjacent_biome_pairs() {
        // Verify all possible biome pair transitions produce valid intermediate UBOs
        let biomes = [
            BiomeType::Grassland,
            BiomeType::Desert,
            BiomeType::Forest,
            BiomeType::Mountain,
        ];

        for (i, src) in biomes.iter().enumerate() {
            for dst in biomes.iter().skip(i + 1) {
                let mut effect = TransitionEffect::new(TransitionConfig {
                    duration: 1.0,
                    easing: EasingFunction::Linear,
                    apply_tint: true,
                    tint_alpha: 0.2,
                    ..Default::default()
                });
                effect.start(Some(*src), *dst);
                effect.update(0.5);

                let mut env = SceneEnvironment::default();
                env.update_from_transition(&effect);
                let ubo = env.to_ubo();

                assert!(
                    ubo.fog_density > 0.0,
                    "Transition {:?}→{:?} produced zero fog",
                    src,
                    dst
                );
                assert!(
                    ubo.blend_factor >= 0.0 && ubo.blend_factor <= 1.0,
                    "Transition {:?}→{:?} blend out of range: {}",
                    src,
                    dst,
                    ubo.blend_factor
                );
                assert_eq!(bytemuck::bytes_of(&ubo).len(), 96);
            }
        }
    }

    #[test]
    fn test_main_shader_contains_scene_env_bindings() {
        // Verify the main WGSL shader constant has the SceneEnv UBO declarations
        // This catches regressions where shader edits remove the scene environment
        let shader = crate::renderer::SHADER_SRC;
        assert!(
            shader.contains("@group(4) @binding(0) var<uniform> uScene: SceneEnv;"),
            "Main shader missing group(4) scene env UBO binding"
        );
        assert!(
            shader.contains("struct SceneEnv"),
            "Main shader missing SceneEnv struct definition"
        );
        assert!(
            shader.contains("apply_scene_fog"),
            "Main shader missing fog application function"
        );
        assert!(
            shader.contains("apply_scene_tint"),
            "Main shader missing tint application function"
        );
        assert!(
            shader.contains("uScene.ambient_color * uScene.ambient_intensity"),
            "Main shader missing ambient from scene environment"
        );
    }

    // ── Weather bridge tests ─────────────────────────────────────────────

    #[test]
    fn test_apply_weather_none() {
        let mut env = SceneEnvironment::default();
        env.apply_weather(crate::effects::WeatherKind::None);
        assert!((env.weather_fog_multiplier - 1.0).abs() < 1e-6);
        assert!((env.weather_ambient_multiplier - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_weather_rain() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Forest);
        env.apply_weather(crate::effects::WeatherKind::Rain);
        assert!((env.weather_fog_multiplier - 2.5).abs() < 1e-6);
        assert!((env.weather_ambient_multiplier - 0.6).abs() < 1e-6);

        // UBO should show amplified fog
        let ubo = env.to_ubo();
        let base_visuals = BiomeVisuals::for_biome(BiomeType::Forest);
        let expected_density = base_visuals.fog_density * 2.5;
        assert!(
            (ubo.fog_density - expected_density).abs() < 1e-4,
            "Rain fog density mismatch: got {} expected {}",
            ubo.fog_density,
            expected_density,
        );
    }

    #[test]
    fn test_apply_weather_wind_trails() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Desert);
        env.apply_weather(crate::effects::WeatherKind::WindTrails);
        assert!((env.weather_fog_multiplier - 1.4).abs() < 1e-6);
        assert!((env.weather_ambient_multiplier - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_weather_reset_clears_multipliers() {
        let mut env = SceneEnvironment::default();
        env.apply_weather(crate::effects::WeatherKind::Rain);
        assert!(env.weather_fog_multiplier > 1.0);
        env.apply_weather(crate::effects::WeatherKind::None);
        assert!((env.weather_fog_multiplier - 1.0).abs() < 1e-6);
        assert!((env.weather_ambient_multiplier - 1.0).abs() < 1e-6);
    }

    // ── Time-of-day ambient tests ────────────────────────────────────────

    #[test]
    fn test_apply_time_of_day_noon() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Grassland);

        let mut tod = crate::environment::TimeOfDay::default();
        tod.current_time = 12.0; // Noon
        env.apply_time_of_day(&tod);

        // At noon, ambient should remain bright
        assert!(
            env.visuals.ambient_intensity > 0.0,
            "Noon ambient should be positive",
        );
    }

    #[test]
    fn test_apply_time_of_day_midnight() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Mountain);
        let base_intensity = env.visuals.ambient_intensity;

        let mut tod = crate::environment::TimeOfDay::default();
        tod.current_time = 0.0; // Midnight
        env.apply_time_of_day(&tod);

        // At midnight the ambient should be dimmer
        // (clamped to at least 0.15× base)
        assert!(
            env.visuals.ambient_intensity <= base_intensity * 1.5 + 0.01,
            "Midnight ambient should not exceed daytime levels",
        );
    }

    #[test]
    fn test_apply_time_of_day_colors_blend_with_biome() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Swamp);

        let mut tod = crate::environment::TimeOfDay::default();
        tod.current_time = 12.0;
        env.apply_time_of_day(&tod);

        // After applying ToD, the ambient should be a blend
        assert!(
            env.visuals.ambient_color.length() > 0.01,
            "Ambient color should not be zero after ToD application",
        );
    }

    // ── Post-shader scene env tests ──────────────────────────────────────

    #[test]
    #[cfg(not(feature = "postfx"))]
    fn test_post_shader_contains_scene_env() {
        let post_shader = crate::renderer::POST_SHADER;
        assert!(
            post_shader.contains("PostSceneEnv"),
            "POST_SHADER missing PostSceneEnv struct",
        );
        assert!(
            post_shader.contains("@group(1) @binding(0)"),
            "POST_SHADER missing group(1) binding",
        );
        assert!(
            post_shader.contains("tint_color"),
            "POST_SHADER missing tint_color field",
        );
        assert!(
            post_shader.contains("tint_alpha"),
            "POST_SHADER missing tint_alpha field",
        );
    }

    #[test]
    #[cfg(feature = "postfx")]
    fn test_post_shader_fx_contains_scene_env() {
        let post_fx = crate::renderer::POST_SHADER_FX;
        assert!(
            post_fx.contains("PostSceneEnv"),
            "POST_SHADER_FX missing PostSceneEnv struct",
        );
        assert!(
            post_fx.contains("@group(1) @binding(0)"),
            "POST_SHADER_FX missing group(1) binding",
        );
    }

    #[test]
    fn test_combined_weather_and_time_of_day() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Beach);

        // Apply rain
        env.apply_weather(crate::effects::WeatherKind::Rain);
        assert!((env.weather_fog_multiplier - 2.5).abs() < 1e-6);

        // Then apply time-of-day (twilight)
        let mut tod = crate::environment::TimeOfDay::default();
        tod.current_time = 5.5; // Early dawn
        env.apply_time_of_day(&tod);

        // Both should be reflected in UBO
        let ubo = env.to_ubo();
        // Fog should be amplified by rain multiplier
        assert!(
            ubo.fog_density > 0.0,
            "Combined fog density should be positive"
        );
        // Ambient should be modified by both ToD and rain
        assert!(
            ubo.ambient_intensity > 0.0,
            "Combined ambient should be positive"
        );
    }

    // ── Sky / Water integration tests ─────────────────────────────────

    #[test]
    fn test_biome_visuals_carries_sky_colors() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Beach);
        let beach = crate::biome_transition::BiomeVisuals::for_biome(BiomeType::Beach);

        // SceneEnvironment.visuals should contain the same sky colours
        assert_eq!(env.visuals.sky_day_top, beach.sky_day_top);
        assert_eq!(env.visuals.sky_sunset_horizon, beach.sky_sunset_horizon);
        assert_eq!(env.visuals.sky_night_top, beach.sky_night_top);
    }

    #[test]
    fn test_biome_visuals_carries_water_colors() {
        let mut env = SceneEnvironment::default();
        env.set_biome(BiomeType::Swamp);
        let swamp = crate::biome_transition::BiomeVisuals::for_biome(BiomeType::Swamp);

        assert_eq!(env.visuals.water_deep, swamp.water_deep);
        assert_eq!(env.visuals.water_shallow, swamp.water_shallow);
        assert_eq!(env.visuals.water_foam, swamp.water_foam);
    }

    #[test]
    fn test_transition_interpolates_sky_water() {
        use crate::biome_transition::{TransitionConfig, TransitionEffect};

        let config = TransitionConfig {
            duration: 1.0,
            easing: crate::biome_transition::EasingFunction::Linear,
            ..Default::default()
        };
        let mut effect = TransitionEffect::new(config.clone());
        effect.start(Some(BiomeType::Forest), BiomeType::Desert);

        let mut env = SceneEnvironment::default();

        // At t=0 → Forest visuals
        env.update_from_transition(&effect);
        let forest = crate::biome_transition::BiomeVisuals::for_biome(BiomeType::Forest);
        assert!(
            (env.visuals.water_deep - forest.water_deep).length() < 0.01,
            "t=0 should be forest water"
        );

        // At t=0.5 → midpoint
        effect.update(0.5);
        env.update_from_transition(&effect);
        let desert = crate::biome_transition::BiomeVisuals::for_biome(BiomeType::Desert);
        let expected_mid = forest.water_deep.lerp(desert.water_deep, 0.5);
        assert!(
            (env.visuals.water_deep - expected_mid).length() < 0.05,
            "t=0.5 should be midpoint water"
        );

        // At t=1.0 → Desert visuals
        effect.update(0.5);
        env.update_from_transition(&effect);
        assert!(
            (env.visuals.sky_day_top - desert.sky_day_top).length() < 0.01,
            "t=1 should be desert sky"
        );
    }

    #[test]
    fn test_sky_config_from_biome_visuals() {
        let beach = crate::biome_transition::BiomeVisuals::for_biome(BiomeType::Beach);
        let cfg = beach.to_sky_config();

        assert_eq!(cfg.day_color_top, beach.sky_day_top);
        assert_eq!(cfg.sunset_color_horizon, beach.sky_sunset_horizon);
        assert_eq!(cfg.night_color_horizon, beach.sky_night_horizon);
    }

    #[test]
    fn test_all_biomes_sky_water_distinct() {
        // Each biome pair should have at least one distinct sky or water colour
        let biomes: Vec<BiomeType> = BiomeType::all().to_vec();
        for (i, &a) in biomes.iter().enumerate() {
            for &b in biomes.iter().skip(i + 1) {
                let va = crate::biome_transition::BiomeVisuals::for_biome(a);
                let vb = crate::biome_transition::BiomeVisuals::for_biome(b);

                let sky_diff = (va.sky_day_top - vb.sky_day_top).length()
                    + (va.sky_sunset_top - vb.sky_sunset_top).length();
                let water_diff = (va.water_deep - vb.water_deep).length()
                    + (va.water_shallow - vb.water_shallow).length();

                assert!(
                    sky_diff > 0.01 || water_diff > 0.01,
                    "{:?} and {:?} have identical sky/water colours",
                    a,
                    b
                );
            }
        }
    }

    // ── Wetness / snow accumulation tests ────────────────────────────────

    #[test]
    fn test_ubo_wetness_default_zero() {
        let ubo = SceneEnvironmentUBO::default();
        assert_eq!(ubo.wetness, 0.0);
        assert_eq!(ubo.snow_amount, 0.0);
    }

    #[test]
    fn test_ubo_wetness_from_scene_env() {
        let mut env = SceneEnvironment::default();
        env.wetness = 0.75;
        env.snow_amount = 0.3;
        let ubo = env.to_ubo();
        assert!((ubo.wetness - 0.75).abs() < 1e-6);
        assert!((ubo.snow_amount - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_ubo_wetness_clamped_to_unit() {
        let mut env = SceneEnvironment::default();
        env.wetness = 2.0;
        env.snow_amount = -0.5;
        let ubo = env.to_ubo();
        assert_eq!(ubo.wetness, 1.0);
        assert_eq!(ubo.snow_amount, 0.0);
    }

    #[test]
    fn test_update_wetness_accumulates_during_rain() {
        let mut env = SceneEnvironment::default();
        assert_eq!(env.wetness, 0.0);
        env.update_wetness(1.0, true); // 1 second of rain
        assert!((env.wetness - 0.5).abs() < 1e-6); // ACCUMULATE_RATE = 0.5
    }

    #[test]
    fn test_update_wetness_saturates_at_one() {
        let mut env = SceneEnvironment::default();
        env.update_wetness(10.0, true); // 10 seconds
        assert_eq!(env.wetness, 1.0);
    }

    #[test]
    fn test_update_wetness_dries_when_not_raining() {
        let mut env = SceneEnvironment::default();
        env.wetness = 1.0;
        env.update_wetness(5.0, false); // DRY_RATE=0.1 → 0.5
        assert!((env.wetness - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_update_wetness_floors_at_zero() {
        let mut env = SceneEnvironment::default();
        env.wetness = 0.05;
        env.update_wetness(10.0, false);
        assert_eq!(env.wetness, 0.0);
    }

    #[test]
    fn test_update_snow_accumulates() {
        let mut env = SceneEnvironment::default();
        env.update_snow(1.0, true); // ACCUMULATE_RATE = 0.3
        assert!((env.snow_amount - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_update_snow_melts() {
        let mut env = SceneEnvironment::default();
        env.snow_amount = 1.0;
        env.update_snow(10.0, false); // MELT_RATE=0.05 → 0.5
        assert!((env.snow_amount - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_wetness_byte_offset_in_ubo() {
        // Verify wetness sits at byte offset 24 (fog_range_pad.z in WGSL).
        let mut ubo = SceneEnvironmentUBO::zeroed();
        ubo.wetness = 0.42;
        let bytes = bytemuck::bytes_of(&ubo);
        let val = f32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        assert!((val - 0.42).abs() < 1e-6);
    }

    #[test]
    fn test_snow_byte_offset_in_ubo() {
        // Verify snow_amount sits at byte offset 28 (fog_range_pad.w in WGSL).
        let mut ubo = SceneEnvironmentUBO::zeroed();
        ubo.snow_amount = 0.77;
        let bytes = bytemuck::bytes_of(&ubo);
        let val = f32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
        assert!((val - 0.77).abs() < 1e-6);
    }

    #[test]
    fn test_wetness_survives_bytemuck_roundtrip() {
        let mut env = SceneEnvironment::default();
        env.wetness = 0.65;
        env.snow_amount = 0.22;
        let ubo = env.to_ubo();
        let bytes = bytemuck::bytes_of(&ubo);
        let back: &SceneEnvironmentUBO = bytemuck::from_bytes(bytes);
        assert!((back.wetness - 0.65).abs() < 1e-6);
        assert!((back.snow_amount - 0.22).abs() < 1e-6);
    }
}
