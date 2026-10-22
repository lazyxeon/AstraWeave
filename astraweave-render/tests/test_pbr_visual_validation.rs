// Visual validation tests for Phase PBR-E advanced materials
// These tests generate sphere grid scenes for manual visual inspection

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::material_extended::*;
use glam::Vec3;

/// Helper to generate a grid of MaterialGpuExtended instances for parameter sweeps
/// Each material varies two parameters across X and Y axes for visual validation
#[derive(Clone, Copy)]
pub struct MaterialGrid {
    pub grid_size: usize, // Number of materials per axis (e.g., 10 = 10x10 = 100 materials)
    pub feature_type: FeatureType,
}

#[derive(Clone, Copy)]
pub enum FeatureType {
    Clearcoat,
    Anisotropy,
    Subsurface,
    Sheen,
    Transmission,
}

impl MaterialGrid {
    /// Generate a grid of materials with parameter sweeps
    /// Returns Vec<MaterialGpuExtended> where materials are ordered row-major (X varies fastest)
    pub fn generate(&self) -> Vec<MaterialGpuExtended> {
        let mut materials = Vec::with_capacity(self.grid_size * self.grid_size);

        for y in 0..self.grid_size {
            for x in 0..self.grid_size {
                let param_x = x as f32 / (self.grid_size - 1) as f32;
                let param_y = y as f32 / (self.grid_size - 1) as f32;

                let mat = match self.feature_type {
                    FeatureType::Clearcoat => self.generate_clearcoat(param_x, param_y),
                    FeatureType::Anisotropy => self.generate_anisotropy(param_x, param_y),
                    FeatureType::Subsurface => self.generate_subsurface(param_x, param_y),
                    FeatureType::Sheen => self.generate_sheen(param_x, param_y),
                    FeatureType::Transmission => self.generate_transmission(param_x, param_y),
                };

                materials.push(mat);
            }
        }

        materials
    }

    /// Clearcoat grid: X = strength (0→1), Y = roughness (0→1)
    fn generate_clearcoat(&self, strength: f32, roughness: f32) -> MaterialGpuExtended {
        let mut mat = MaterialGpuExtended::car_paint(
            Vec3::new(0.8, 0.0, 0.0), // Red base
            0.9,                      // High metallic
            0.3,                      // Base roughness
        );
        mat.clearcoat_strength = strength;
        mat.clearcoat_roughness = roughness;
        mat
    }

    /// Anisotropy grid: X = strength (-1→1), Y = rotation (0→2π)
    fn generate_anisotropy(&self, strength_norm: f32, rotation_norm: f32) -> MaterialGpuExtended {
        let strength = strength_norm * 2.0 - 1.0; // Map [0,1] → [-1,1]
        let rotation = rotation_norm * std::f32::consts::TAU; // Map [0,1] → [0,2π]

        MaterialGpuExtended::brushed_metal(
            Vec3::new(0.9, 0.9, 0.9), // Silver
            0.4,                      // Medium roughness
            strength,
            rotation,
        )
    }

    /// Subsurface grid: X = scale (0→1), Y = radius (0→5mm)
    fn generate_subsurface(&self, scale: f32, radius_norm: f32) -> MaterialGpuExtended {
        let radius = radius_norm * 5.0; // Map [0,1] → [0,5mm]

        MaterialGpuExtended::skin(
            Vec3::new(0.95, 0.8, 0.7), // Skin tone
            Vec3::new(0.9, 0.3, 0.3),  // Reddish subsurface
            radius,
            scale,
        )
    }

    /// Sheen grid: X = sheen intensity (0→1), Y = roughness (0→1)
    fn generate_sheen(&self, intensity: f32, roughness: f32) -> MaterialGpuExtended {
        let sheen_color = Vec3::ONE * intensity; // White sheen with variable intensity

        MaterialGpuExtended::velvet(
            Vec3::new(0.5, 0.0, 0.1), // Deep red base
            sheen_color,
            roughness,
        )
    }

    /// Transmission grid: X = transmission factor (0→1), Y = IOR (1.0→2.5)
    fn generate_transmission(&self, transmission: f32, ior_norm: f32) -> MaterialGpuExtended {
        let ior = 1.0 + ior_norm * 1.5; // Map [0,1] → [1.0,2.5]

        MaterialGpuExtended::glass(
            Vec3::ONE, // Clear tint
            0.05,      // Very smooth
            transmission,
            ior,
            Vec3::new(0.9, 1.0, 0.9), // Slight green tint
            10.0,                     // 10cm attenuation distance
        )
    }
}

#[test]
fn test_clearcoat_grid_generation() {
    let grid = MaterialGrid {
        grid_size: 10,
        feature_type: FeatureType::Clearcoat,
    };

    let materials = grid.generate();
    assert_eq!(
        materials.len(),
        100,
        "Should generate 10x10 = 100 materials"
    );

    // Validate corner cases
    // Bottom-left: strength=0, roughness=0
    assert!((materials[0].clearcoat_strength - 0.0).abs() < 1e-6);
    assert!((materials[0].clearcoat_roughness - 0.0).abs() < 1e-6);

    // Top-right: strength=1, roughness=1
    assert!((materials[99].clearcoat_strength - 1.0).abs() < 1e-6);
    assert!((materials[99].clearcoat_roughness - 1.0).abs() < 1e-6);

    // Check feature flag is set
    assert_eq!(
        materials[0].flags & MATERIAL_FLAG_CLEARCOAT,
        MATERIAL_FLAG_CLEARCOAT
    );
}

#[test]
fn test_anisotropy_grid_generation() {
    let grid = MaterialGrid {
        grid_size: 8,
        feature_type: FeatureType::Anisotropy,
    };

    let materials = grid.generate();
    assert_eq!(materials.len(), 64, "Should generate 8x8 = 64 materials");

    // Validate anisotropy range [-1, 1]
    // Bottom-left: strength=-1, rotation=0
    assert!((materials[0].anisotropy_strength - (-1.0)).abs() < 1e-5);
    assert!((materials[0].anisotropy_rotation - 0.0).abs() < 1e-5);

    // Top-right: strength=1, rotation=2π
    assert!((materials[63].anisotropy_strength - 1.0).abs() < 1e-5);
    assert!((materials[63].anisotropy_rotation - std::f32::consts::TAU).abs() < 1e-5);

    // Check feature flag is set
    assert_eq!(
        materials[0].flags & MATERIAL_FLAG_ANISOTROPY,
        MATERIAL_FLAG_ANISOTROPY
    );
}

#[test]
fn test_subsurface_grid_generation() {
    let grid = MaterialGrid {
        grid_size: 6,
        feature_type: FeatureType::Subsurface,
    };

    let materials = grid.generate();
    assert_eq!(materials.len(), 36, "Should generate 6x6 = 36 materials");

    // Validate subsurface parameter ranges
    // Bottom-left: scale=0, radius=0
    assert!((materials[0].subsurface_scale - 0.0).abs() < 1e-6);
    assert!((materials[0].subsurface_radius - 0.0).abs() < 1e-6);

    // Top-right: scale=1, radius=5mm
    assert!((materials[35].subsurface_scale - 1.0).abs() < 1e-6);
    assert!((materials[35].subsurface_radius - 5.0).abs() < 1e-5);

    // Check feature flag is set
    assert_eq!(
        materials[0].flags & MATERIAL_FLAG_SUBSURFACE,
        MATERIAL_FLAG_SUBSURFACE
    );
}

#[test]
fn test_sheen_grid_generation() {
    let grid = MaterialGrid {
        grid_size: 8,
        feature_type: FeatureType::Sheen,
    };

    let materials = grid.generate();
    assert_eq!(materials.len(), 64, "Should generate 8x8 = 64 materials");

    // Validate sheen parameters
    // Bottom-left: intensity=0, roughness=0
    let sheen_intensity_0 = materials[0]
        .sheen_color
        .iter()
        .fold(0.0f32, |a, &b| a.max(b));
    assert!(sheen_intensity_0 < 1e-5);
    assert!((materials[0].sheen_roughness - 0.0).abs() < 1e-6);

    // Top-right: intensity=1, roughness=1
    let sheen_intensity_1 = materials[63]
        .sheen_color
        .iter()
        .fold(0.0f32, |a, &b| a.max(b));
    assert!((sheen_intensity_1 - 1.0).abs() < 1e-5);
    assert!((materials[63].sheen_roughness - 1.0).abs() < 1e-6);

    // Check feature flag is set
    assert_eq!(
        materials[0].flags & MATERIAL_FLAG_SHEEN,
        MATERIAL_FLAG_SHEEN
    );
}

#[test]
fn test_transmission_grid_generation() {
    let grid = MaterialGrid {
        grid_size: 10,
        feature_type: FeatureType::Transmission,
    };

    let materials = grid.generate();
    assert_eq!(
        materials.len(),
        100,
        "Should generate 10x10 = 100 materials"
    );

    // Validate transmission parameters
    // Bottom-left: transmission=0, ior=1.0
    assert!((materials[0].transmission_factor - 0.0).abs() < 1e-6);
    assert!((materials[0].ior - 1.0).abs() < 1e-5);

    // Top-right: transmission=1, ior=2.5
    assert!((materials[99].transmission_factor - 1.0).abs() < 1e-6);
    assert!((materials[99].ior - 2.5).abs() < 1e-5);

    // Check feature flag is set
    assert_eq!(
        materials[0].flags & MATERIAL_FLAG_TRANSMISSION,
        MATERIAL_FLAG_TRANSMISSION
    );
}

#[test]
fn test_multi_feature_grid_generation() {
    // Test generating grids for all 5 features
    let features = [
        FeatureType::Clearcoat,
        FeatureType::Anisotropy,
        FeatureType::Subsurface,
        FeatureType::Sheen,
        FeatureType::Transmission,
    ];

    for feature in &features {
        let grid = MaterialGrid {
            grid_size: 5,
            feature_type: *feature,
        };

        let materials = grid.generate();
        assert_eq!(materials.len(), 25, "Should generate 5x5 = 25 materials");

        // Verify all materials have correct size
        for mat in &materials {
            assert_eq!(
                std::mem::size_of_val(mat),
                256,
                "Material should be 256 bytes"
            );
        }
    }
}

#[test]
fn test_grid_material_ordering() {
    // Verify materials are ordered row-major (X varies fastest)
    let grid = MaterialGrid {
        grid_size: 4,
        feature_type: FeatureType::Clearcoat,
    };

    let materials = grid.generate();

    // First row (y=0): strength should increase, roughness=0
    for (x, mat) in materials.iter().take(4).enumerate() {
        let expected_strength = x as f32 / 3.0;
        assert!((mat.clearcoat_strength - expected_strength).abs() < 1e-5);
        assert!((mat.clearcoat_roughness - 0.0).abs() < 1e-6);
    }

    // Last row (y=3): strength should increase, roughness=1
    for (x, mat) in materials.iter().skip(12).take(4).enumerate() {
        let expected_strength = x as f32 / 3.0;
        assert!((mat.clearcoat_strength - expected_strength).abs() < 1e-5);
        assert!((mat.clearcoat_roughness - 1.0).abs() < 1e-6);
    }
}

/// Helper function for examples/unified_showcase integration
/// Returns material grid + sphere positions for rendering
pub fn generate_validation_scene(
    feature: FeatureType,
    grid_size: usize,
) -> (Vec<MaterialGpuExtended>, Vec<Vec3>) {
    let grid = MaterialGrid {
        grid_size,
        feature_type: feature,
    };
    let materials = grid.generate();

    // Generate sphere positions in a grid layout
    let spacing = 2.5; // World units between spheres
    let mut positions = Vec::with_capacity(grid_size * grid_size);

    let offset = (grid_size as f32 - 1.0) * spacing * 0.5; // Center the grid

    for y in 0..grid_size {
        for x in 0..grid_size {
            let pos = Vec3::new(
                x as f32 * spacing - offset,
                0.0,
                y as f32 * spacing - offset,
            );
            positions.push(pos);
        }
    }

    (materials, positions)
}

#[test]
fn test_validation_scene_generation() {
    let (materials, positions) = generate_validation_scene(FeatureType::Clearcoat, 8);

    assert_eq!(materials.len(), 64);
    assert_eq!(positions.len(), 64);

    // Verify grid is centered around origin
    let spacing = 2.5;
    let offset = (8.0 - 1.0) * spacing * 0.5;

    // First position should be at (-offset, 0, -offset)
    assert!((positions[0].x - (-offset)).abs() < 1e-5);
    assert!((positions[0].y - 0.0).abs() < 1e-6);
    assert!((positions[0].z - (-offset)).abs() < 1e-5);

    // Last position should be at (offset, 0, offset)
    assert!((positions[63].x - offset).abs() < 1e-5);
    assert!((positions[63].y - 0.0).abs() < 1e-6);
    assert!((positions[63].z - offset).abs() < 1e-5);
}
