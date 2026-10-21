// Light module - Directional, point, spot lights
// Day 3-4 implementation

//! Lighting systems extracted from Bevy

use glam::Vec3;

/// Directional light (sun/moon)
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    /// Light direction (normalized)
    pub direction: Vec3,

    /// Light color (linear RGB)
    pub color: Vec3,

    /// Illuminance in lux
    pub illuminance: f32,

    /// Cast shadows
    pub shadows_enabled: bool,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.5, -1.0, -0.3).normalize(),
            color: Vec3::ONE,
            illuminance: 100000.0, // Sunlight
            shadows_enabled: true,
        }
    }
}

/// Point light
#[derive(Debug, Clone)]
pub struct PointLight {
    /// World position
    pub position: Vec3,

    /// Light color (linear RGB)
    pub color: Vec3,

    /// Intensity in lumens
    pub intensity: f32,

    /// Maximum range
    pub range: f32,

    /// Light radius (for soft shadows)
    pub radius: f32,

    /// Cast shadows
    pub shadows_enabled: bool,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            color: Vec3::ONE,
            intensity: 800.0, // ~60W bulb
            range: 20.0,
            radius: 0.1,
            shadows_enabled: false,
        }
    }
}

/// Spot light
#[derive(Debug, Clone)]
pub struct SpotLight {
    /// World position
    pub position: Vec3,

    /// Direction (normalized)
    pub direction: Vec3,

    /// Light color (linear RGB)
    pub color: Vec3,

    /// Intensity in lumens
    pub intensity: f32,

    /// Maximum range
    pub range: f32,

    /// Inner cone angle (radians)
    pub inner_angle: f32,

    /// Outer cone angle (radians)
    pub outer_angle: f32,

    /// Cast shadows
    pub shadows_enabled: bool,
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            direction: Vec3::NEG_Y,
            color: Vec3::ONE,
            intensity: 1000.0,
            range: 20.0,
            inner_angle: 0.0,
            outer_angle: std::f32::consts::FRAC_PI_4, // 45 degrees
            shadows_enabled: false,
        }
    }
}

// Day 4: Add GPU data structures
// - GpuLights (uniform buffer)
// - ClusteredForwardBindGroup
// - Light culling
