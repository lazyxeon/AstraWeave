//! Shadow quality improvements: cascade stabilization, PCSS configuration,
//! normal-offset bias, and cascade blending.
//!
//! Works alongside the existing `shadow_csm.rs` to enhance shadow quality
//! without replacing the core cascade infrastructure.

use glam::{Mat4, Vec3, Vec4};

/// GPU-side uniform for production shadow sampling.
/// Matches `ShadowParams` in `shadow_sampling.wgsl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowParamsGpu {
    /// Per-cascade view-projection matrices.
    pub cascade_vp: [[[f32; 4]; 4]; 4],
    /// Cascade split distances (view-space depth). x/y/z/w = cascade 0/1/2/3 far.
    pub splits: [f32; 4],
    /// x: PCF radius (texels), y: depth bias, z: normal offset scale, w: PCSS light size.
    pub shadow_config: [f32; 4],
    /// x: cascade blend range (world units), y: PCSS blocker search radius, z/w: unused.
    pub shadow_config2: [f32; 4],
}

impl Default for ShadowParamsGpu {
    fn default() -> Self {
        Self {
            cascade_vp: [Mat4::IDENTITY.to_cols_array_2d(); 4],
            splits: [10.0, 30.0, 80.0, 200.0],
            shadow_config: [2.0, 0.005, 1.0, 0.0], // pcf=2, bias=0.005, normal_offset=1, pcss=off
            shadow_config2: [5.0, 8.0, 0.0, 0.0],  // blend=5 units, blocker_search=8 texels
        }
    }
}

/// Shadow quality configuration.
#[derive(Debug, Clone)]
pub struct ShadowQualityConfig {
    /// PCF filter radius in texels (1.0 = sharp, 3.0 = soft).
    pub pcf_radius: f32,
    /// Depth bias to prevent shadow acne.
    pub depth_bias: f32,
    /// Normal-offset bias scale (prevents acne without peter-panning).
    pub normal_offset_scale: f32,
    /// PCSS light source size (0 = disabled, >0 = contact-hardening shadows).
    pub pcss_light_size: f32,
    /// PCSS blocker search radius in texels.
    pub pcss_blocker_search_radius: f32,
    /// Cascade blend range in view-space depth units.
    pub cascade_blend_range: f32,
    /// Enable cascade stabilization (texel-snapping).
    pub stabilize_cascades: bool,
    /// Logarithmic split factor (0 = uniform, 1 = fully logarithmic).
    pub log_split_factor: f32,
}

impl Default for ShadowQualityConfig {
    fn default() -> Self {
        Self {
            pcf_radius: 2.0,
            depth_bias: 0.005,
            normal_offset_scale: 1.0,
            pcss_light_size: 2.0, // Enable PCSS by default
            pcss_blocker_search_radius: 8.0,
            cascade_blend_range: 5.0,
            stabilize_cascades: true,
            log_split_factor: 0.75,
        }
    }
}

/// Compute logarithmic cascade split distances.
///
/// Uses a blend of uniform and logarithmic distribution for optimal
/// shadow map utilization across the view frustum.
pub fn compute_cascade_splits(
    near: f32,
    far: f32,
    cascade_count: usize,
    log_factor: f32,
) -> Vec<f32> {
    let mut splits = Vec::with_capacity(cascade_count);
    for i in 1..=cascade_count {
        let t = i as f32 / cascade_count as f32;
        // Logarithmic split
        let log_split = near * (far / near).powf(t);
        // Uniform split
        let uniform_split = near + (far - near) * t;
        // Blend
        let split = log_factor * log_split + (1.0 - log_factor) * uniform_split;
        splits.push(split);
    }
    splits
}

/// Stabilize a cascade's orthographic projection to prevent shadow swimming.
///
/// Snaps the light-space bounds to texel boundaries so that as the camera
/// moves, the shadow map texels don't shift sub-pixel, eliminating shimmer.
pub fn stabilize_cascade_matrix(view_proj: Mat4, shadow_map_size: f32) -> Mat4 {
    // Extract the translation components from the VP matrix
    let mut stabilized = view_proj;

    // Calculate world-space units per texel
    let vp_origin = view_proj * Vec4::new(0.0, 0.0, 0.0, 1.0);
    let texel_size_x = 2.0 / shadow_map_size;
    let texel_size_y = 2.0 / shadow_map_size;

    // Snap the origin to texel boundaries
    let offset_x = vp_origin.x % texel_size_x;
    let offset_y = vp_origin.y % texel_size_y;

    // Adjust the matrix to remove sub-texel offset
    // This modifies the translation column of the VP matrix
    let mut cols = stabilized.to_cols_array_2d();
    cols[3][0] -= offset_x;
    cols[3][1] -= offset_y;
    stabilized = Mat4::from_cols_array_2d(&cols);

    stabilized
}

/// Compute the normal-offset position for shadow sampling.
///
/// Pushes the sample position along the surface normal to eliminate
/// shadow acne. The offset is proportional to the angle between the
/// surface normal and light direction.
pub fn compute_normal_offset(
    world_pos: Vec3,
    normal: Vec3,
    light_dir: Vec3,
    shadow_map_size: f32,
    cascade_world_size: f32,
    offset_scale: f32,
) -> Vec3 {
    let cos_angle = normal.dot(light_dir).abs();
    let texel_world_size = cascade_world_size / shadow_map_size;
    let offset = (1.0 - cos_angle) * offset_scale * texel_world_size;
    world_pos + normal * offset
}

/// Build GPU params from config and cascade matrices.
pub fn build_shadow_params(
    config: &ShadowQualityConfig,
    cascade_vps: &[Mat4; 4],
    splits: &[f32; 4],
    shadow_map_size: f32,
) -> ShadowParamsGpu {
    let mut vps = [Mat4::IDENTITY.to_cols_array_2d(); 4];
    for i in 0..4 {
        let vp = if config.stabilize_cascades {
            stabilize_cascade_matrix(cascade_vps[i], shadow_map_size)
        } else {
            cascade_vps[i]
        };
        vps[i] = vp.to_cols_array_2d();
    }

    ShadowParamsGpu {
        cascade_vp: vps,
        splits: *splits,
        shadow_config: [
            config.pcf_radius,
            config.depth_bias,
            config.normal_offset_scale,
            config.pcss_light_size,
        ],
        shadow_config2: [
            config.cascade_blend_range,
            config.pcss_blocker_search_radius,
            0.0,
            0.0,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_params_gpu_size() {
        // 4 * 64 (matrices) + padding + 4*4 (splits) + 4*4 (config) + 4*4 (config2)
        let size = std::mem::size_of::<ShadowParamsGpu>();
        assert!(
            size >= 288,
            "ShadowParamsGpu should be at least 288 bytes, got {}",
            size
        );
        assert!(
            size <= 320,
            "ShadowParamsGpu should be at most 320 bytes, got {}",
            size
        );
    }

    #[test]
    fn shadow_params_default() {
        let p = ShadowParamsGpu::default();
        assert_eq!(p.splits, [10.0, 30.0, 80.0, 200.0]);
        assert_eq!(p.shadow_config[0], 2.0); // pcf radius
        assert_eq!(p.shadow_config[1], 0.005); // depth bias
    }

    #[test]
    fn shadow_quality_config_default() {
        let c = ShadowQualityConfig::default();
        assert!(c.stabilize_cascades);
        assert!(c.pcss_light_size > 0.0);
        assert_eq!(c.cascade_blend_range, 5.0);
    }

    #[test]
    fn cascade_splits_logarithmic() {
        let splits = compute_cascade_splits(0.1, 200.0, 4, 0.75);
        assert_eq!(splits.len(), 4);
        // Splits should be monotonically increasing
        for i in 1..splits.len() {
            assert!(splits[i] > splits[i - 1], "splits should increase");
        }
        // Last split should equal far
        assert!((splits[3] - 200.0).abs() < 0.01);
        // First split should be between near and far
        assert!(splits[0] > 0.1);
        assert!(splits[0] < 200.0);
    }

    #[test]
    fn cascade_splits_uniform() {
        let splits = compute_cascade_splits(0.1, 100.0, 4, 0.0);
        // With log_factor = 0, splits should be evenly spaced
        let expected_step = (100.0 - 0.1) / 4.0;
        for i in 0..4 {
            let expected = 0.1 + expected_step * (i as f32 + 1.0);
            assert!(
                (splits[i] - expected).abs() < 0.01,
                "split {} should be ~{}, got {}",
                i,
                expected,
                splits[i]
            );
        }
    }

    #[test]
    fn stabilize_cascade_is_idempotent() {
        let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let s1 = stabilize_cascade_matrix(vp, 2048.0);
        let s2 = stabilize_cascade_matrix(s1, 2048.0);
        // Applying stabilization twice should give same result (or very close)
        let diff = (s1 - s2).abs_diff_eq(Mat4::ZERO, 1e-5);
        assert!(diff, "Stabilization should be near-idempotent");
    }

    #[test]
    fn normal_offset_increases_at_grazing_angles() {
        let pos = Vec3::ZERO;
        let normal = Vec3::Y;
        let light_straight = Vec3::Y; // Light straight down = cos=1 = no offset
        let light_grazing = Vec3::new(1.0, 0.1, 0.0).normalize(); // Near-grazing

        let offset_straight =
            compute_normal_offset(pos, normal, light_straight, 2048.0, 100.0, 1.0);
        let offset_grazing = compute_normal_offset(pos, normal, light_grazing, 2048.0, 100.0, 1.0);

        let dist_straight = (offset_straight - pos).length();
        let dist_grazing = (offset_grazing - pos).length();
        assert!(
            dist_grazing > dist_straight,
            "Grazing angle should have larger offset"
        );
    }

    #[test]
    fn build_shadow_params_with_stabilization() {
        let config = ShadowQualityConfig::default();
        let vps = [
            Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0),
            Mat4::orthographic_rh(-20.0, 20.0, -20.0, 20.0, 0.1, 100.0),
            Mat4::orthographic_rh(-40.0, 40.0, -40.0, 40.0, 0.1, 100.0),
            Mat4::orthographic_rh(-80.0, 80.0, -80.0, 80.0, 0.1, 100.0),
        ];
        let splits = [10.0, 30.0, 80.0, 200.0];
        let params = build_shadow_params(&config, &vps, &splits, 2048.0);

        assert_eq!(params.splits, splits);
        assert_eq!(params.shadow_config[0], config.pcf_radius);
        assert_eq!(params.shadow_config[3], config.pcss_light_size);
    }

    #[test]
    fn shadow_sampling_shader_parses() {
        let src = concat!(
            include_str!("../shaders/constants.wgsl"),
            include_str!("../shaders/shadow_sampling.wgsl")
        );
        // The shader contains only functions (no entry points), so naga will parse
        // but won't find entry points. We just validate it parses without errors.
        // Use naga's frontend directly to check syntax.
        let result = naga::front::wgsl::parse_str(src);
        assert!(
            result.is_ok(),
            "shadow_sampling.wgsl should parse: {:?}",
            result.err()
        );
    }
}
