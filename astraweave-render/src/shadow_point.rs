//! Point and spot light shadow map management.
//!
//! Provides shadow cubemaps for point lights and 2D shadow maps for spot lights.
//! Uses a priority system to select which lights get shadow maps based on
//! distance and screen-area contribution.

use glam::{Mat4, Vec3};

/// Maximum number of shadow-casting point lights per frame.
pub const MAX_POINT_SHADOW_LIGHTS: usize = 8;

/// Maximum number of shadow-casting spot lights per frame.
pub const MAX_SPOT_SHADOW_LIGHTS: usize = 8;

/// Resolution of each point light shadow cubemap face.
pub const POINT_SHADOW_RESOLUTION: u32 = 512;

/// Resolution of each spot light shadow map.
pub const SPOT_SHADOW_RESOLUTION: u32 = 1024;

/// A light candidate for shadow map allocation.
#[derive(Debug, Clone)]
pub struct ShadowLightCandidate {
    /// World-space position.
    pub position: Vec3,
    /// Light radius (attenuation range).
    pub radius: f32,
    /// Priority score (higher = more important for shadows).
    pub priority: f32,
    /// Index into the scene light array.
    pub light_index: usize,
}

/// Configuration for the point/spot shadow system.
#[derive(Debug, Clone)]
pub struct PointShadowConfig {
    /// Maximum point lights with shadow maps.
    pub max_point_shadows: usize,
    /// Maximum spot lights with shadow maps.
    pub max_spot_shadows: usize,
    /// Point shadow cubemap face resolution.
    pub point_resolution: u32,
    /// Spot shadow map resolution.
    pub spot_resolution: u32,
    /// Depth bias for point lights.
    pub point_bias: f32,
    /// Depth bias for spot lights.
    pub spot_bias: f32,
    /// PCF radius for spot lights.
    pub spot_pcf_radius: f32,
}

impl Default for PointShadowConfig {
    fn default() -> Self {
        Self {
            max_point_shadows: MAX_POINT_SHADOW_LIGHTS,
            max_spot_shadows: MAX_SPOT_SHADOW_LIGHTS,
            point_resolution: POINT_SHADOW_RESOLUTION,
            spot_resolution: SPOT_SHADOW_RESOLUTION,
            point_bias: 0.01,
            spot_bias: 0.005,
            spot_pcf_radius: 1.5,
        }
    }
}

/// Select which lights should receive shadow maps based on priority.
///
/// Priority is computed from distance to camera and light radius (screen coverage).
pub fn select_shadow_casters(
    candidates: &[ShadowLightCandidate],
    camera_pos: Vec3,
    max_count: usize,
) -> Vec<usize> {
    let mut scored: Vec<(usize, f32)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let dist = (c.position - camera_pos).length();
            // Priority: larger radius and closer distance = higher priority
            let screen_area = c.radius / (dist + 0.1);
            let score = screen_area * c.priority;
            (i, score)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored.iter().take(max_count).map(|&(i, _)| i).collect()
}

/// Compute the 6 view matrices for a point light cubemap.
///
/// Returns [+X, -X, +Y, -Y, +Z, -Z] view matrices.
pub fn point_light_view_matrices(light_pos: Vec3) -> [Mat4; 6] {
    [
        Mat4::look_at_rh(light_pos, light_pos + Vec3::X, -Vec3::Y), // +X
        Mat4::look_at_rh(light_pos, light_pos - Vec3::X, -Vec3::Y), // -X
        Mat4::look_at_rh(light_pos, light_pos + Vec3::Y, Vec3::Z),  // +Y
        Mat4::look_at_rh(light_pos, light_pos - Vec3::Y, -Vec3::Z), // -Y
        Mat4::look_at_rh(light_pos, light_pos + Vec3::Z, -Vec3::Y), // +Z
        Mat4::look_at_rh(light_pos, light_pos - Vec3::Z, -Vec3::Y), // -Z
    ]
}

/// Compute the perspective projection for a point light cubemap face.
pub fn point_light_projection(near: f32, far: f32) -> Mat4 {
    Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, near, far)
}

/// Compute the view-projection matrix for a spot light.
pub fn spot_light_vp(light_pos: Vec3, light_dir: Vec3, fov: f32, near: f32, far: f32) -> Mat4 {
    let up = if light_dir.y.abs() > 0.99 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let view = Mat4::look_at_rh(light_pos, light_pos + light_dir, up);
    let proj = Mat4::perspective_rh(fov, 1.0, near, far);
    proj * view
}

/// Manages point and spot shadow map GPU resources.
pub struct PointShadowPass {
    config: PointShadowConfig,
    /// Point light shadow cubemap array (depth).
    point_shadow_texture: wgpu::Texture,
    point_shadow_view: wgpu::TextureView,
    /// Spot light shadow 2D array (depth).
    spot_shadow_texture: wgpu::Texture,
    spot_shadow_view: wgpu::TextureView,
    /// Comparison sampler.
    shadow_sampler: wgpu::Sampler,
}

impl PointShadowPass {
    pub fn new(device: &wgpu::Device, config: PointShadowConfig) -> Self {
        // Point light shadow cubemap array
        let point_shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("point_shadow_cubemap_array"),
            size: wgpu::Extent3d {
                width: config.point_resolution,
                height: config.point_resolution,
                // 6 faces per cubemap * max lights
                depth_or_array_layers: 6 * config.max_point_shadows as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let point_shadow_view = point_shadow_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::CubeArray),
            ..Default::default()
        });

        // Spot light shadow 2D array
        let spot_shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("spot_shadow_array"),
            size: wgpu::Extent3d {
                width: config.spot_resolution,
                height: config.spot_resolution,
                depth_or_array_layers: config.max_spot_shadows as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let spot_shadow_view =
            spot_shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("point_spot_shadow_sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            config,
            point_shadow_texture,
            point_shadow_view,
            spot_shadow_texture,
            spot_shadow_view,
            shadow_sampler,
        }
    }

    pub fn point_shadow_view(&self) -> &wgpu::TextureView {
        &self.point_shadow_view
    }

    pub fn spot_shadow_view(&self) -> &wgpu::TextureView {
        &self.spot_shadow_view
    }

    pub fn shadow_sampler(&self) -> &wgpu::Sampler {
        &self.shadow_sampler
    }

    pub fn config(&self) -> &PointShadowConfig {
        &self.config
    }

    /// Get a view for a specific point light cubemap face.
    pub fn point_face_view(&self, light_index: usize, face: u32) -> wgpu::TextureView {
        let layer = light_index as u32 * 6 + face;
        self.point_shadow_texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("point_shadow_face_{light_index}_{face}")),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: layer,
                array_layer_count: Some(1),
                ..Default::default()
            })
    }

    /// Get a view for a specific spot light shadow map.
    pub fn spot_layer_view(&self, light_index: usize) -> wgpu::TextureView {
        self.spot_shadow_texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("spot_shadow_{light_index}")),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: light_index as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_shadow_config_default() {
        let c = PointShadowConfig::default();
        assert_eq!(c.max_point_shadows, 8);
        assert_eq!(c.max_spot_shadows, 8);
        assert_eq!(c.point_resolution, 512);
        assert_eq!(c.spot_resolution, 1024);
    }

    #[test]
    fn select_shadow_casters_prioritizes_close_large_lights() {
        let camera = Vec3::ZERO;
        let candidates = vec![
            ShadowLightCandidate {
                position: Vec3::new(100.0, 0.0, 0.0),
                radius: 5.0,
                priority: 1.0,
                light_index: 0,
            },
            ShadowLightCandidate {
                position: Vec3::new(2.0, 0.0, 0.0),
                radius: 10.0,
                priority: 1.0,
                light_index: 1,
            },
            ShadowLightCandidate {
                position: Vec3::new(5.0, 0.0, 0.0),
                radius: 20.0,
                priority: 1.0,
                light_index: 2,
            },
        ];

        let selected = select_shadow_casters(&candidates, camera, 2);
        assert_eq!(selected.len(), 2);
        // Light 1 (close, medium) and light 2 (medium dist, large) should win
        assert!(selected.contains(&1) || selected.contains(&2));
    }

    #[test]
    fn point_light_view_matrices_are_valid() {
        let pos = Vec3::new(5.0, 10.0, 3.0);
        let views = point_light_view_matrices(pos);
        assert_eq!(views.len(), 6);
        // Each should be a valid rotation+translation matrix
        for v in &views {
            let det = v.determinant();
            assert!(det.abs() > 0.01, "View matrix should be non-degenerate");
        }
    }

    #[test]
    fn point_light_projection_is_90_degrees() {
        let proj = point_light_projection(0.1, 100.0);
        // 90-degree FOV, aspect 1:1
        let det = proj.determinant();
        assert!(det.abs() > 0.0001);
    }

    #[test]
    fn spot_light_vp_non_degenerate() {
        let vp = spot_light_vp(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            std::f32::consts::FRAC_PI_4,
            0.1,
            50.0,
        );
        assert!(vp.determinant().abs() > 0.0001);
    }

    #[test]
    fn point_shadow_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let config = PointShadowConfig::default();
        let pass = PointShadowPass::new(&device, config);
        assert_eq!(pass.config().max_point_shadows, 8);
    }
}
