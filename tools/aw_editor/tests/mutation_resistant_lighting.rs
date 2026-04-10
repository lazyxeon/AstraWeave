//! Mutation-resistant tests for Lighting Panel system
//!
//! These tests target boundary conditions, comparison operators, and boolean return paths
//! to achieve high mutation testing kill rates (90%+).

use aw_editor_lib::panels::{LightType, LightUnit, ShadowQuality, ShadowType};
use std::collections::HashSet;

// ============================================================================
// LIGHT TYPE TESTS
// ============================================================================

mod light_type_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(LightType::all().len(), 5);
    }

    // Test all variant name() uniqueness
    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = LightType::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test all variant icon() uniqueness
    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = LightType::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_directional_name() {
        assert_eq!(LightType::Directional.name(), "Directional");
    }

    #[test]
    fn test_point_name() {
        assert_eq!(LightType::Point.name(), "Point");
    }

    #[test]
    fn test_spot_name() {
        assert_eq!(LightType::Spot.name(), "Spot");
    }

    #[test]
    fn test_area_name() {
        assert_eq!(LightType::Area.name(), "Area");
    }

    #[test]
    fn test_ambient_name() {
        assert_eq!(LightType::Ambient.name(), "Ambient");
    }

    // Test is_directional()
    #[test]
    fn test_directional_is_directional() {
        assert!(LightType::Directional.is_directional());
    }

    #[test]
    fn test_point_is_not_directional() {
        assert!(!LightType::Point.is_directional());
    }

    #[test]
    fn test_spot_is_not_directional() {
        assert!(!LightType::Spot.is_directional());
    }

    #[test]
    fn test_area_is_not_directional() {
        assert!(!LightType::Area.is_directional());
    }

    #[test]
    fn test_ambient_is_not_directional() {
        assert!(!LightType::Ambient.is_directional());
    }

    // Test has_range()
    #[test]
    fn test_point_has_range() {
        assert!(LightType::Point.has_range());
    }

    #[test]
    fn test_spot_has_range() {
        assert!(LightType::Spot.has_range());
    }

    #[test]
    fn test_directional_has_no_range() {
        assert!(!LightType::Directional.has_range());
    }

    #[test]
    fn test_area_has_no_range() {
        assert!(!LightType::Area.has_range());
    }

    #[test]
    fn test_ambient_has_no_range() {
        assert!(!LightType::Ambient.has_range());
    }

    // Test description() uniqueness
    #[test]
    fn test_all_descriptions_unique() {
        let descs: Vec<&str> = LightType::all().iter().map(|c| c.description()).collect();
        let unique: HashSet<_> = descs.iter().collect();
        assert_eq!(descs.len(), unique.len());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", LightType::Directional);
        assert!(display.contains("[Sun]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", LightType::Point);
        assert!(display.contains("Point"));
    }

    // Test default
    #[test]
    fn test_default_is_directional() {
        assert_eq!(LightType::default(), LightType::Directional);
    }
}

// ============================================================================
// SHADOW QUALITY TESTS
// ============================================================================

mod shadow_quality_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(ShadowQuality::all().len(), 5);
    }

    // Test all variant name() uniqueness
    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = ShadowQuality::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test all variant icon() uniqueness
    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = ShadowQuality::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_off_name() {
        assert_eq!(ShadowQuality::Off.name(), "Off");
    }

    #[test]
    fn test_low_name() {
        assert_eq!(ShadowQuality::Low.name(), "Low");
    }

    #[test]
    fn test_medium_name() {
        assert_eq!(ShadowQuality::Medium.name(), "Medium");
    }

    #[test]
    fn test_high_name() {
        assert_eq!(ShadowQuality::High.name(), "High");
    }

    #[test]
    fn test_ultra_name() {
        assert_eq!(ShadowQuality::Ultra.name(), "Ultra");
    }

    // Test resolution()
    #[test]
    fn test_off_resolution() {
        assert_eq!(ShadowQuality::Off.resolution(), 0);
    }

    #[test]
    fn test_low_resolution() {
        assert_eq!(ShadowQuality::Low.resolution(), 512);
    }

    #[test]
    fn test_medium_resolution() {
        assert_eq!(ShadowQuality::Medium.resolution(), 1024);
    }

    #[test]
    fn test_high_resolution() {
        assert_eq!(ShadowQuality::High.resolution(), 2048);
    }

    #[test]
    fn test_ultra_resolution() {
        assert_eq!(ShadowQuality::Ultra.resolution(), 4096);
    }

    // Test is_enabled()
    #[test]
    fn test_off_is_not_enabled() {
        assert!(!ShadowQuality::Off.is_enabled());
    }

    #[test]
    fn test_low_is_enabled() {
        assert!(ShadowQuality::Low.is_enabled());
    }

    #[test]
    fn test_medium_is_enabled() {
        assert!(ShadowQuality::Medium.is_enabled());
    }

    #[test]
    fn test_high_is_enabled() {
        assert!(ShadowQuality::High.is_enabled());
    }

    #[test]
    fn test_ultra_is_enabled() {
        assert!(ShadowQuality::Ultra.is_enabled());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", ShadowQuality::High);
        assert!(display.contains("[Gem]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", ShadowQuality::Ultra);
        assert!(display.contains("Ultra"));
    }

    // Test default
    #[test]
    fn test_default_is_medium() {
        assert_eq!(ShadowQuality::default(), ShadowQuality::Medium);
    }
}

// ============================================================================
// SHADOW TYPE TESTS
// ============================================================================

mod shadow_type_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(ShadowType::all().len(), 4);
    }

    // Test all variant name() uniqueness
    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = ShadowType::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test all variant icon() uniqueness
    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = ShadowType::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test all variant description() uniqueness
    #[test]
    fn test_all_descriptions_unique() {
        let descs: Vec<&str> = ShadowType::all().iter().map(|c| c.description()).collect();
        let unique: HashSet<_> = descs.iter().collect();
        assert_eq!(descs.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_none_name() {
        assert_eq!(ShadowType::None.name(), "None");
    }

    #[test]
    fn test_hard_name() {
        assert_eq!(ShadowType::Hard.name(), "Hard");
    }

    #[test]
    fn test_soft_name() {
        assert_eq!(ShadowType::Soft.name(), "Soft");
    }

    #[test]
    fn test_pcss_name() {
        assert_eq!(ShadowType::PCSS.name(), "PCSS");
    }

    // Test is_soft()
    #[test]
    fn test_none_is_not_soft() {
        assert!(!ShadowType::None.is_soft());
    }

    #[test]
    fn test_hard_is_not_soft() {
        assert!(!ShadowType::Hard.is_soft());
    }

    #[test]
    fn test_soft_is_soft() {
        assert!(ShadowType::Soft.is_soft());
    }

    #[test]
    fn test_pcss_is_soft() {
        assert!(ShadowType::PCSS.is_soft());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", ShadowType::PCSS);
        assert!(display.contains("[G]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", ShadowType::Hard);
        assert!(display.contains("Hard"));
    }

    // Test default
    #[test]
    fn test_default_is_hard() {
        assert_eq!(ShadowType::default(), ShadowType::Hard);
    }
}

// ============================================================================
// LIGHT UNIT TESTS
// ============================================================================

mod light_unit_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(LightUnit::all().len(), 5);
    }

    // Test all variant name() uniqueness
    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = LightUnit::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test all variant icon() uniqueness
    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = LightUnit::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_unitless_name() {
        assert_eq!(LightUnit::Unitless.name(), "Unitless");
    }

    #[test]
    fn test_lumen_name() {
        assert_eq!(LightUnit::Lumen.name(), "Lumen");
    }

    #[test]
    fn test_candela_name() {
        assert_eq!(LightUnit::Candela.name(), "Candela");
    }

    #[test]
    fn test_lux_name() {
        assert_eq!(LightUnit::Lux.name(), "Lux");
    }

    #[test]
    fn test_nit_name() {
        assert_eq!(LightUnit::Nit.name(), "Nit");
    }

    // Test abbreviation()
    #[test]
    fn test_unitless_abbreviation() {
        assert_eq!(LightUnit::Unitless.abbreviation(), "");
    }

    #[test]
    fn test_lumen_abbreviation() {
        assert_eq!(LightUnit::Lumen.abbreviation(), "lm");
    }

    #[test]
    fn test_candela_abbreviation() {
        assert_eq!(LightUnit::Candela.abbreviation(), "cd");
    }

    #[test]
    fn test_lux_abbreviation() {
        assert_eq!(LightUnit::Lux.abbreviation(), "lx");
    }

    #[test]
    fn test_nit_abbreviation() {
        assert_eq!(LightUnit::Nit.abbreviation(), "nt");
    }

    // Test is_physical()
    #[test]
    fn test_unitless_is_not_physical() {
        assert!(!LightUnit::Unitless.is_physical());
    }

    #[test]
    fn test_lumen_is_physical() {
        assert!(LightUnit::Lumen.is_physical());
    }

    #[test]
    fn test_candela_is_physical() {
        assert!(LightUnit::Candela.is_physical());
    }

    #[test]
    fn test_lux_is_physical() {
        assert!(LightUnit::Lux.is_physical());
    }

    #[test]
    fn test_nit_is_physical() {
        assert!(LightUnit::Nit.is_physical());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", LightUnit::Lumen);
        assert!(display.contains("[Lgt]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", LightUnit::Candela);
        assert!(display.contains("Candela"));
    }

    // Test default
    #[test]
    fn test_default_is_unitless() {
        assert_eq!(LightUnit::default(), LightUnit::Unitless);
    }
}

// ============================================================================
// LIGHT TESTS
// ============================================================================

mod light_tests {
    use aw_editor_lib::panels::Light;

    #[test]
    fn test_default_enabled() {
        let light = Light::default();
        assert!(light.enabled);
    }

    #[test]
    fn test_default_cast_shadows() {
        let light = Light::default();
        assert!(light.cast_shadows);
    }

    #[test]
    fn test_default_intensity() {
        let light = Light::default();
        assert_eq!(light.intensity, 1.0);
    }

    #[test]
    fn test_default_temperature() {
        let light = Light::default();
        assert_eq!(light.temperature, 6500.0);
    }

    #[test]
    fn test_default_range() {
        let light = Light::default();
        assert_eq!(light.range, 10.0);
    }

    #[test]
    fn test_default_spot_angle() {
        let light = Light::default();
        assert_eq!(light.spot_angle, 45.0);
    }

    #[test]
    fn test_default_inner_spot_angle() {
        let light = Light::default();
        assert_eq!(light.inner_spot_angle, 30.0);
    }

    #[test]
    fn test_default_shadow_bias() {
        let light = Light::default();
        assert_eq!(light.shadow_bias, 0.05);
    }

    #[test]
    fn test_default_color() {
        let light = Light::default();
        assert_eq!(light.color, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_default_use_temperature() {
        let light = Light::default();
        assert!(!light.use_temperature);
    }
}
