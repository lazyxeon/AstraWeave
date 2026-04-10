//! Parallax Occlusion Mapping (POM) configuration and utilities.
//!
//! POM creates the illusion of surface depth by ray-marching through a heightmap
//! in tangent space. Steeper view angles use more steps (8–32) with binary
//! refinement for sub-texel accuracy.
//!
//! Integration:
//! - In `pbr.wgsl`: POM is controlled via `MaterialUbo._pad.y` (height_scale).
//!   Set to 0.0 to disable, 0.02–0.08 for typical stone/brick surfaces.
//!   Requires heightmap bound at group 3, bindings 6+7.
//! - In `pbr_terrain.wgsl`: `pom_offset_uv_terrain()` uses the existing
//!   `height_array` texture. Pass the layer's height scale from CPU.
//! - Standalone utility: `shaders/pbr/parallax.wgsl` provides both single-texture
//!   and texture-array variants.

/// WGSL source for the standalone POM utility module.
pub const PARALLAX_WGSL: &str = include_str!("../shaders/pbr/parallax.wgsl");

/// POM configuration parameters.
#[derive(Debug, Clone)]
pub struct PomConfig {
    /// Height scale (max displacement in UV space). 0.0 = disabled.
    /// Typical values: 0.02 (subtle) to 0.08 (pronounced).
    pub height_scale: f32,
    /// Minimum ray-march steps (used at perpendicular view).
    pub min_steps: u32,
    /// Maximum ray-march steps (used at grazing view).
    pub max_steps: u32,
    /// Binary refinement iterations after linear search.
    pub refinement_steps: u32,
}

impl Default for PomConfig {
    fn default() -> Self {
        Self {
            height_scale: 0.04,
            min_steps: 8,
            max_steps: 32,
            refinement_steps: 5,
        }
    }
}

impl PomConfig {
    /// POM disabled (height_scale = 0).
    pub fn disabled() -> Self {
        Self {
            height_scale: 0.0,
            ..Default::default()
        }
    }

    /// Preset for subtle displacement (cobblestones, fabric).
    pub fn subtle() -> Self {
        Self {
            height_scale: 0.02,
            min_steps: 8,
            max_steps: 24,
            refinement_steps: 4,
        }
    }

    /// Preset for moderate displacement (bricks, rough stone).
    pub fn moderate() -> Self {
        Self::default()
    }

    /// Preset for deep displacement (deep cracks, terrain close-up).
    pub fn deep() -> Self {
        Self {
            height_scale: 0.08,
            min_steps: 12,
            max_steps: 32,
            refinement_steps: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = PomConfig::default();
        assert!((cfg.height_scale - 0.04).abs() < 1e-5);
        assert_eq!(cfg.min_steps, 8);
        assert_eq!(cfg.max_steps, 32);
        assert_eq!(cfg.refinement_steps, 5);
    }

    #[test]
    fn disabled_config() {
        let cfg = PomConfig::disabled();
        assert_eq!(cfg.height_scale, 0.0);
    }

    #[test]
    fn presets() {
        let subtle = PomConfig::subtle();
        assert!((subtle.height_scale - 0.02).abs() < 1e-5);
        assert_eq!(subtle.max_steps, 24);

        let deep = PomConfig::deep();
        assert!((deep.height_scale - 0.08).abs() < 1e-5);
        assert_eq!(deep.min_steps, 12);
    }

    #[test]
    fn parse_parallax_wgsl() {
        let module = naga::front::wgsl::parse_str(PARALLAX_WGSL);
        assert!(
            module.is_ok(),
            "parallax.wgsl parse failed: {:?}",
            module.err()
        );
    }
}
