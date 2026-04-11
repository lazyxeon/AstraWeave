//! Stochastic hex-tile texture sampling to break terrain texture repetition.
//!
//! Implements the Heitz & Neyret 2018 algorithm using a hexagonal grid with
//! per-cell random UV rotation and offset. The WGSL shader provides drop-in
//! replacements for `textureSample()` that eliminate visible tiling patterns
//! on large terrain surfaces.
//!
//! # Shader Integration
//!
//! Include the WGSL source via [`STOCHASTIC_TILING_WGSL`] in your shader
//! module (e.g. via naga `#import` or string concatenation):
//!
//! ```wgsl
//! // In your terrain fragment shader:
//! let color = sample_stochastic(albedo_tex, terrain_sampler, uv, 1.0);
//! ```
//!
//! For texture arrays (multi-layer terrain):
//! ```wgsl
//! let color = sample_stochastic_array(layer_albedo, samp, uv, layer_idx, 1.0);
//! ```

/// WGSL source for stochastic hex-tile sampling utilities.
///
/// Contains the following functions:
/// - `sample_stochastic(tex, samp, uv, scale)` — standard texture2d
/// - `sample_stochastic_array(tex, samp, uv, layer, scale)` — texture2d_array
/// - `hex_coords(uv)` — raw hex grid decomposition
/// - `rotate_uv(uv, angle)` — UV rotation helper
pub const STOCHASTIC_TILING_WGSL: &str = include_str!("../shaders/stochastic_tiling.wgsl");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_source_is_non_empty() {
        assert!(
            STOCHASTIC_TILING_WGSL.len() > 500,
            "stochastic tiling shader should have substantial content"
        );
    }

    #[test]
    fn shader_contains_key_functions() {
        assert!(STOCHASTIC_TILING_WGSL.contains("fn sample_stochastic("));
        assert!(STOCHASTIC_TILING_WGSL.contains("fn sample_stochastic_array("));
        assert!(STOCHASTIC_TILING_WGSL.contains("fn hex_coords("));
        assert!(STOCHASTIC_TILING_WGSL.contains("fn rotate_uv("));
    }
}
