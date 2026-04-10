//! Mutation-resistant tests for Particle System Panel
//!
//! These tests target boundary conditions, comparison operators, and boolean return paths
//! to achieve high mutation testing kill rates (90%+).

use aw_editor_lib::panels::particle_system_panel::CurveType;
use aw_editor_lib::panels::{EmitterShape, ParticleBlendMode, ParticleRenderMode, SimulationSpace};
use std::collections::HashSet;

// ============================================================================
// EMITTER SHAPE TESTS
// ============================================================================

mod emitter_shape_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(EmitterShape::all().len(), 8);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = EmitterShape::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = EmitterShape::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_point_name() {
        assert_eq!(EmitterShape::Point.name(), "Point");
    }

    #[test]
    fn test_sphere_name() {
        assert_eq!(EmitterShape::Sphere.name(), "Sphere");
    }

    #[test]
    fn test_hemisphere_name() {
        assert_eq!(EmitterShape::Hemisphere.name(), "Hemisphere");
    }

    #[test]
    fn test_cone_name() {
        assert_eq!(EmitterShape::Cone.name(), "Cone");
    }

    #[test]
    fn test_box_name() {
        assert_eq!(EmitterShape::Box.name(), "Box");
    }

    #[test]
    fn test_circle_name() {
        assert_eq!(EmitterShape::Circle.name(), "Circle");
    }

    #[test]
    fn test_edge_name() {
        assert_eq!(EmitterShape::Edge.name(), "Edge");
    }

    #[test]
    fn test_mesh_name() {
        assert_eq!(EmitterShape::Mesh.name(), "Mesh");
    }

    // Test is_volumetric()
    #[test]
    fn test_sphere_is_volumetric() {
        assert!(EmitterShape::Sphere.is_volumetric());
    }

    #[test]
    fn test_hemisphere_is_volumetric() {
        assert!(EmitterShape::Hemisphere.is_volumetric());
    }

    #[test]
    fn test_cone_is_volumetric() {
        assert!(EmitterShape::Cone.is_volumetric());
    }

    #[test]
    fn test_box_is_volumetric() {
        assert!(EmitterShape::Box.is_volumetric());
    }

    #[test]
    fn test_point_is_not_volumetric() {
        assert!(!EmitterShape::Point.is_volumetric());
    }

    #[test]
    fn test_circle_is_not_volumetric() {
        assert!(!EmitterShape::Circle.is_volumetric());
    }

    #[test]
    fn test_edge_is_not_volumetric() {
        assert!(!EmitterShape::Edge.is_volumetric());
    }

    #[test]
    fn test_mesh_is_not_volumetric() {
        assert!(!EmitterShape::Mesh.is_volumetric());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", EmitterShape::Point);
        assert!(display.contains("•"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", EmitterShape::Sphere);
        assert!(display.contains("Sphere"));
    }

    // Test default
    #[test]
    fn test_default_is_point() {
        assert_eq!(EmitterShape::default(), EmitterShape::Point);
    }
}

// ============================================================================
// SIMULATION SPACE TESTS
// ============================================================================

mod simulation_space_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(SimulationSpace::all().len(), 2);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = SimulationSpace::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = SimulationSpace::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_local_name() {
        assert_eq!(SimulationSpace::Local.name(), "Local");
    }

    #[test]
    fn test_world_name() {
        assert_eq!(SimulationSpace::World.name(), "World");
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", SimulationSpace::Local);
        assert!(display.contains("📍"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", SimulationSpace::World);
        assert!(display.contains("World"));
    }

    // Test default
    #[test]
    fn test_default_is_local() {
        assert_eq!(SimulationSpace::default(), SimulationSpace::Local);
    }
}

// ============================================================================
// PARTICLE BLEND MODE TESTS
// ============================================================================

mod particle_blend_mode_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(ParticleBlendMode::all().len(), 4);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = ParticleBlendMode::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = ParticleBlendMode::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_alpha_name() {
        assert_eq!(ParticleBlendMode::Alpha.name(), "Alpha");
    }

    #[test]
    fn test_additive_name() {
        assert_eq!(ParticleBlendMode::Additive.name(), "Additive");
    }

    #[test]
    fn test_multiply_name() {
        assert_eq!(ParticleBlendMode::Multiply.name(), "Multiply");
    }

    #[test]
    fn test_premultiply_name() {
        assert_eq!(ParticleBlendMode::Premultiply.name(), "Premultiply");
    }

    // Test is_additive()
    #[test]
    fn test_additive_is_additive() {
        assert!(ParticleBlendMode::Additive.is_additive());
    }

    #[test]
    fn test_alpha_is_not_additive() {
        assert!(!ParticleBlendMode::Alpha.is_additive());
    }

    #[test]
    fn test_multiply_is_not_additive() {
        assert!(!ParticleBlendMode::Multiply.is_additive());
    }

    #[test]
    fn test_premultiply_is_not_additive() {
        assert!(!ParticleBlendMode::Premultiply.is_additive());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", ParticleBlendMode::Additive);
        assert!(display.contains("[Fx]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", ParticleBlendMode::Multiply);
        assert!(display.contains("Multiply"));
    }

    // Test default
    #[test]
    fn test_default_is_alpha() {
        assert_eq!(ParticleBlendMode::default(), ParticleBlendMode::Alpha);
    }
}

// ============================================================================
// PARTICLE RENDER MODE TESTS
// ============================================================================

mod particle_render_mode_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(ParticleRenderMode::all().len(), 6);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = ParticleRenderMode::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = ParticleRenderMode::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_billboard_name() {
        assert_eq!(ParticleRenderMode::Billboard.name(), "Billboard");
    }

    #[test]
    fn test_stretched_billboard_name() {
        assert_eq!(
            ParticleRenderMode::StretchedBillboard.name(),
            "Stretched Billboard"
        );
    }

    #[test]
    fn test_horizontal_billboard_name() {
        assert_eq!(
            ParticleRenderMode::HorizontalBillboard.name(),
            "Horizontal Billboard"
        );
    }

    #[test]
    fn test_vertical_billboard_name() {
        assert_eq!(
            ParticleRenderMode::VerticalBillboard.name(),
            "Vertical Billboard"
        );
    }

    #[test]
    fn test_mesh_name() {
        assert_eq!(ParticleRenderMode::Mesh.name(), "Mesh");
    }

    #[test]
    fn test_trail_name() {
        assert_eq!(ParticleRenderMode::Trail.name(), "Trail");
    }

    // Test is_billboard()
    #[test]
    fn test_billboard_is_billboard() {
        assert!(ParticleRenderMode::Billboard.is_billboard());
    }

    #[test]
    fn test_stretched_billboard_is_billboard() {
        assert!(ParticleRenderMode::StretchedBillboard.is_billboard());
    }

    #[test]
    fn test_horizontal_billboard_is_billboard() {
        assert!(ParticleRenderMode::HorizontalBillboard.is_billboard());
    }

    #[test]
    fn test_vertical_billboard_is_billboard() {
        assert!(ParticleRenderMode::VerticalBillboard.is_billboard());
    }

    #[test]
    fn test_mesh_is_not_billboard() {
        assert!(!ParticleRenderMode::Mesh.is_billboard());
    }

    #[test]
    fn test_trail_is_not_billboard() {
        assert!(!ParticleRenderMode::Trail.is_billboard());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", ParticleRenderMode::Billboard);
        assert!(display.contains("[Doc]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", ParticleRenderMode::Mesh);
        assert!(display.contains("Mesh"));
    }

    // Test default
    #[test]
    fn test_default_is_billboard() {
        assert_eq!(ParticleRenderMode::default(), ParticleRenderMode::Billboard);
    }
}

// ============================================================================
// CURVE TYPE TESTS
// ============================================================================

mod curve_type_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(CurveType::all().len(), 7);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = CurveType::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = CurveType::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_constant_name() {
        assert_eq!(CurveType::Constant.name(), "Constant");
    }

    #[test]
    fn test_linear_name() {
        assert_eq!(CurveType::Linear.name(), "Linear");
    }

    #[test]
    fn test_ease_in_name() {
        assert_eq!(CurveType::EaseIn.name(), "Ease In");
    }

    #[test]
    fn test_ease_out_name() {
        assert_eq!(CurveType::EaseOut.name(), "Ease Out");
    }

    #[test]
    fn test_ease_in_out_name() {
        assert_eq!(CurveType::EaseInOut.name(), "Ease In Out");
    }

    #[test]
    fn test_random_name() {
        assert_eq!(CurveType::Random.name(), "Random");
    }

    #[test]
    fn test_curve_name() {
        assert_eq!(CurveType::Curve.name(), "Curve");
    }

    // Test is_easing()
    #[test]
    fn test_ease_in_is_easing() {
        assert!(CurveType::EaseIn.is_easing());
    }

    #[test]
    fn test_ease_out_is_easing() {
        assert!(CurveType::EaseOut.is_easing());
    }

    #[test]
    fn test_ease_in_out_is_easing() {
        assert!(CurveType::EaseInOut.is_easing());
    }

    #[test]
    fn test_constant_is_not_easing() {
        assert!(!CurveType::Constant.is_easing());
    }

    #[test]
    fn test_linear_is_not_easing() {
        assert!(!CurveType::Linear.is_easing());
    }

    #[test]
    fn test_random_is_not_easing() {
        assert!(!CurveType::Random.is_easing());
    }

    #[test]
    fn test_curve_is_not_easing() {
        assert!(!CurveType::Curve.is_easing());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", CurveType::Linear);
        assert!(display.contains("📈"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", CurveType::Random);
        assert!(display.contains("Random"));
    }

    // Test default
    #[test]
    fn test_default_is_constant() {
        assert_eq!(CurveType::default(), CurveType::Constant);
    }
}
