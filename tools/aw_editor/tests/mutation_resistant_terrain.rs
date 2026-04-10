//! Mutation-Resistant Tests for Terrain Panel Types
//!
//! This module provides comprehensive tests for terrain-related types to ensure
//! mutation testing achieves 90%+ kill rates through:
//! - Exhaustive match arm coverage for all enum variants
//! - Boundary condition testing for all value methods
//! - Boolean return path testing for all predicate methods
//! - Comparison and ordering tests where applicable

use aw_editor_lib::panels::terrain_panel::{FluidQualityPreset, WaterBodyPreset};
use aw_editor_lib::panels::{BrushMode, ErosionPresetType};

// ============================================================================
// ErosionPresetType Tests (6 variants)
// ============================================================================

mod erosion_preset_type_tests {
    use super::*;

    // --- name() method tests - covers all 6 match arms ---

    #[test]
    fn erosion_preset_custom_name_returns_custom() {
        assert_eq!(ErosionPresetType::Custom.name(), "Custom");
    }

    #[test]
    fn erosion_preset_desert_name_returns_desert() {
        assert_eq!(ErosionPresetType::Desert.name(), "Desert");
    }

    #[test]
    fn erosion_preset_mountain_name_returns_mountain() {
        assert_eq!(ErosionPresetType::Mountain.name(), "Mountain");
    }

    #[test]
    fn erosion_preset_coastal_name_returns_coastal() {
        assert_eq!(ErosionPresetType::Coastal.name(), "Coastal");
    }

    #[test]
    fn erosion_preset_alpine_name_returns_alpine() {
        assert_eq!(ErosionPresetType::Alpine.name(), "Alpine");
    }

    #[test]
    fn erosion_preset_canyon_name_returns_canyon() {
        assert_eq!(ErosionPresetType::Canyon.name(), "Canyon");
    }

    // --- Variant identity tests (uniqueness) ---

    #[test]
    fn erosion_preset_custom_is_not_desert() {
        assert_ne!(
            ErosionPresetType::Custom.name(),
            ErosionPresetType::Desert.name()
        );
    }

    #[test]
    fn erosion_preset_custom_is_not_mountain() {
        assert_ne!(
            ErosionPresetType::Custom.name(),
            ErosionPresetType::Mountain.name()
        );
    }

    #[test]
    fn erosion_preset_desert_is_not_coastal() {
        assert_ne!(
            ErosionPresetType::Desert.name(),
            ErosionPresetType::Coastal.name()
        );
    }

    #[test]
    fn erosion_preset_mountain_is_not_alpine() {
        assert_ne!(
            ErosionPresetType::Mountain.name(),
            ErosionPresetType::Alpine.name()
        );
    }

    #[test]
    fn erosion_preset_coastal_is_not_canyon() {
        assert_ne!(
            ErosionPresetType::Coastal.name(),
            ErosionPresetType::Canyon.name()
        );
    }

    #[test]
    fn erosion_preset_alpine_is_not_custom() {
        assert_ne!(
            ErosionPresetType::Alpine.name(),
            ErosionPresetType::Custom.name()
        );
    }

    // --- all() method tests ---

    #[test]
    fn erosion_preset_all_returns_six_variants() {
        assert_eq!(ErosionPresetType::all().len(), 6);
    }

    #[test]
    fn erosion_preset_all_contains_custom() {
        assert!(ErosionPresetType::all().contains(&ErosionPresetType::Custom));
    }

    #[test]
    fn erosion_preset_all_contains_desert() {
        assert!(ErosionPresetType::all().contains(&ErosionPresetType::Desert));
    }

    #[test]
    fn erosion_preset_all_contains_mountain() {
        assert!(ErosionPresetType::all().contains(&ErosionPresetType::Mountain));
    }

    #[test]
    fn erosion_preset_all_contains_coastal() {
        assert!(ErosionPresetType::all().contains(&ErosionPresetType::Coastal));
    }

    #[test]
    fn erosion_preset_all_contains_alpine() {
        assert!(ErosionPresetType::all().contains(&ErosionPresetType::Alpine));
    }

    #[test]
    fn erosion_preset_all_contains_canyon() {
        assert!(ErosionPresetType::all().contains(&ErosionPresetType::Canyon));
    }

    // --- Display trait tests ---

    #[test]
    fn erosion_preset_custom_display_matches_name() {
        assert_eq!(format!("{}", ErosionPresetType::Custom), "Custom");
    }

    #[test]
    fn erosion_preset_desert_display_matches_name() {
        assert_eq!(format!("{}", ErosionPresetType::Desert), "Desert");
    }

    #[test]
    fn erosion_preset_mountain_display_matches_name() {
        assert_eq!(format!("{}", ErosionPresetType::Mountain), "Mountain");
    }

    #[test]
    fn erosion_preset_coastal_display_matches_name() {
        assert_eq!(format!("{}", ErosionPresetType::Coastal), "Coastal");
    }

    #[test]
    fn erosion_preset_alpine_display_matches_name() {
        assert_eq!(format!("{}", ErosionPresetType::Alpine), "Alpine");
    }

    #[test]
    fn erosion_preset_canyon_display_matches_name() {
        assert_eq!(format!("{}", ErosionPresetType::Canyon), "Canyon");
    }

    // --- Equality tests ---

    #[test]
    fn erosion_preset_custom_equals_custom() {
        assert_eq!(ErosionPresetType::Custom, ErosionPresetType::Custom);
    }

    #[test]
    fn erosion_preset_desert_equals_desert() {
        assert_eq!(ErosionPresetType::Desert, ErosionPresetType::Desert);
    }

    #[test]
    fn erosion_preset_mountain_equals_mountain() {
        assert_eq!(ErosionPresetType::Mountain, ErosionPresetType::Mountain);
    }

    #[test]
    fn erosion_preset_custom_not_equal_to_desert() {
        assert_ne!(ErosionPresetType::Custom, ErosionPresetType::Desert);
    }
}

// ============================================================================
// WaterBodyPreset Tests (7 variants)
// ============================================================================

mod water_body_preset_tests {
    use super::*;

    // --- name() method tests - covers all 7 match arms ---

    #[test]
    fn water_body_custom_name_returns_custom() {
        assert_eq!(WaterBodyPreset::Custom.name(), "Custom");
    }

    #[test]
    fn water_body_calm_lake_name_returns_calm_lake() {
        assert_eq!(WaterBodyPreset::CalmLake.name(), "Calm Lake");
    }

    #[test]
    fn water_body_mountain_stream_name_returns_mountain_stream() {
        assert_eq!(WaterBodyPreset::MountainStream.name(), "Mountain Stream");
    }

    #[test]
    fn water_body_raging_river_name_returns_raging_river() {
        assert_eq!(WaterBodyPreset::RagingRiver.name(), "Raging River");
    }

    #[test]
    fn water_body_ocean_name_returns_ocean() {
        assert_eq!(WaterBodyPreset::Ocean.name(), "Ocean");
    }

    #[test]
    fn water_body_waterfall_name_returns_waterfall() {
        assert_eq!(WaterBodyPreset::Waterfall.name(), "Waterfall");
    }

    #[test]
    fn water_body_swamp_wetland_name_returns_swamp_wetland() {
        assert_eq!(WaterBodyPreset::SwampWetland.name(), "Swamp/Wetland");
    }

    // --- is_flowing() boolean return path tests ---
    // TRUE paths: MountainStream, RagingRiver, Waterfall (3)
    // FALSE paths: Custom, CalmLake, Ocean, SwampWetland (4)

    #[test]
    fn water_body_mountain_stream_is_flowing_returns_true() {
        assert!(WaterBodyPreset::MountainStream.is_flowing());
    }

    #[test]
    fn water_body_raging_river_is_flowing_returns_true() {
        assert!(WaterBodyPreset::RagingRiver.is_flowing());
    }

    #[test]
    fn water_body_waterfall_is_flowing_returns_true() {
        assert!(WaterBodyPreset::Waterfall.is_flowing());
    }

    #[test]
    fn water_body_custom_is_flowing_returns_false() {
        assert!(!WaterBodyPreset::Custom.is_flowing());
    }

    #[test]
    fn water_body_calm_lake_is_flowing_returns_false() {
        assert!(!WaterBodyPreset::CalmLake.is_flowing());
    }

    #[test]
    fn water_body_ocean_is_flowing_returns_false() {
        assert!(!WaterBodyPreset::Ocean.is_flowing());
    }

    #[test]
    fn water_body_swamp_wetland_is_flowing_returns_false() {
        assert!(!WaterBodyPreset::SwampWetland.is_flowing());
    }

    // --- is_flowing() explicit boolean comparisons ---

    #[test]
    fn water_body_mountain_stream_is_flowing_equals_true() {
        assert_eq!(WaterBodyPreset::MountainStream.is_flowing(), true);
    }

    #[test]
    fn water_body_raging_river_is_flowing_equals_true() {
        assert_eq!(WaterBodyPreset::RagingRiver.is_flowing(), true);
    }

    #[test]
    fn water_body_waterfall_is_flowing_equals_true() {
        assert_eq!(WaterBodyPreset::Waterfall.is_flowing(), true);
    }

    #[test]
    fn water_body_custom_is_flowing_equals_false() {
        assert_eq!(WaterBodyPreset::Custom.is_flowing(), false);
    }

    #[test]
    fn water_body_calm_lake_is_flowing_equals_false() {
        assert_eq!(WaterBodyPreset::CalmLake.is_flowing(), false);
    }

    #[test]
    fn water_body_ocean_is_flowing_equals_false() {
        assert_eq!(WaterBodyPreset::Ocean.is_flowing(), false);
    }

    #[test]
    fn water_body_swamp_wetland_is_flowing_equals_false() {
        assert_eq!(WaterBodyPreset::SwampWetland.is_flowing(), false);
    }

    // --- is_flowing() negation tests ---

    #[test]
    fn water_body_mountain_stream_is_not_not_flowing() {
        assert!(!(!WaterBodyPreset::MountainStream.is_flowing()));
    }

    #[test]
    fn water_body_custom_is_not_flowing_negation() {
        assert!(!(WaterBodyPreset::Custom.is_flowing()));
    }

    // --- all() method tests ---

    #[test]
    fn water_body_all_returns_seven_variants() {
        assert_eq!(WaterBodyPreset::all().len(), 7);
    }

    #[test]
    fn water_body_all_contains_custom() {
        assert!(WaterBodyPreset::all().contains(&WaterBodyPreset::Custom));
    }

    #[test]
    fn water_body_all_contains_calm_lake() {
        assert!(WaterBodyPreset::all().contains(&WaterBodyPreset::CalmLake));
    }

    #[test]
    fn water_body_all_contains_mountain_stream() {
        assert!(WaterBodyPreset::all().contains(&WaterBodyPreset::MountainStream));
    }

    #[test]
    fn water_body_all_contains_raging_river() {
        assert!(WaterBodyPreset::all().contains(&WaterBodyPreset::RagingRiver));
    }

    #[test]
    fn water_body_all_contains_ocean() {
        assert!(WaterBodyPreset::all().contains(&WaterBodyPreset::Ocean));
    }

    #[test]
    fn water_body_all_contains_waterfall() {
        assert!(WaterBodyPreset::all().contains(&WaterBodyPreset::Waterfall));
    }

    #[test]
    fn water_body_all_contains_swamp_wetland() {
        assert!(WaterBodyPreset::all().contains(&WaterBodyPreset::SwampWetland));
    }

    // --- Display trait tests ---

    #[test]
    fn water_body_custom_display_matches_name() {
        assert_eq!(format!("{}", WaterBodyPreset::Custom), "Custom");
    }

    #[test]
    fn water_body_calm_lake_display_matches_name() {
        assert_eq!(format!("{}", WaterBodyPreset::CalmLake), "Calm Lake");
    }

    #[test]
    fn water_body_ocean_display_matches_name() {
        assert_eq!(format!("{}", WaterBodyPreset::Ocean), "Ocean");
    }

    // --- Counting tests ---

    #[test]
    fn water_body_flowing_count_equals_three() {
        let count = WaterBodyPreset::all()
            .iter()
            .filter(|p| p.is_flowing())
            .count();
        assert_eq!(count, 3);
    }

    #[test]
    fn water_body_not_flowing_count_equals_four() {
        let count = WaterBodyPreset::all()
            .iter()
            .filter(|p| !p.is_flowing())
            .count();
        assert_eq!(count, 4);
    }

    // --- Boundary: all variants have names ---

    #[test]
    fn water_body_all_variants_have_non_empty_names() {
        for preset in WaterBodyPreset::all() {
            assert!(!preset.name().is_empty());
        }
    }
}

// ============================================================================
// FluidQualityPreset Tests (4 variants)
// ============================================================================

mod fluid_quality_preset_tests {
    use super::*;

    // --- name() method tests - covers all 4 match arms ---

    #[test]
    fn fluid_quality_performance_name_returns_performance() {
        assert_eq!(FluidQualityPreset::Performance.name(), "Performance");
    }

    #[test]
    fn fluid_quality_balanced_name_returns_balanced() {
        assert_eq!(FluidQualityPreset::Balanced.name(), "Balanced");
    }

    #[test]
    fn fluid_quality_quality_name_returns_quality() {
        assert_eq!(FluidQualityPreset::Quality.name(), "Quality");
    }

    #[test]
    fn fluid_quality_cinematic_name_returns_cinematic() {
        assert_eq!(FluidQualityPreset::Cinematic.name(), "Cinematic");
    }

    // --- Variant identity tests ---

    #[test]
    fn fluid_quality_performance_is_not_balanced() {
        assert_ne!(
            FluidQualityPreset::Performance.name(),
            FluidQualityPreset::Balanced.name()
        );
    }

    #[test]
    fn fluid_quality_balanced_is_not_quality() {
        assert_ne!(
            FluidQualityPreset::Balanced.name(),
            FluidQualityPreset::Quality.name()
        );
    }

    #[test]
    fn fluid_quality_quality_is_not_cinematic() {
        assert_ne!(
            FluidQualityPreset::Quality.name(),
            FluidQualityPreset::Cinematic.name()
        );
    }

    #[test]
    fn fluid_quality_cinematic_is_not_performance() {
        assert_ne!(
            FluidQualityPreset::Cinematic.name(),
            FluidQualityPreset::Performance.name()
        );
    }

    // --- all() method tests ---

    #[test]
    fn fluid_quality_all_returns_four_variants() {
        assert_eq!(FluidQualityPreset::all().len(), 4);
    }

    #[test]
    fn fluid_quality_all_contains_performance() {
        assert!(FluidQualityPreset::all().contains(&FluidQualityPreset::Performance));
    }

    #[test]
    fn fluid_quality_all_contains_balanced() {
        assert!(FluidQualityPreset::all().contains(&FluidQualityPreset::Balanced));
    }

    #[test]
    fn fluid_quality_all_contains_quality() {
        assert!(FluidQualityPreset::all().contains(&FluidQualityPreset::Quality));
    }

    #[test]
    fn fluid_quality_all_contains_cinematic() {
        assert!(FluidQualityPreset::all().contains(&FluidQualityPreset::Cinematic));
    }

    // --- Display trait tests ---

    #[test]
    fn fluid_quality_performance_display_matches_name() {
        assert_eq!(
            format!("{}", FluidQualityPreset::Performance),
            "Performance"
        );
    }

    #[test]
    fn fluid_quality_balanced_display_matches_name() {
        assert_eq!(format!("{}", FluidQualityPreset::Balanced), "Balanced");
    }

    #[test]
    fn fluid_quality_quality_display_matches_name() {
        assert_eq!(format!("{}", FluidQualityPreset::Quality), "Quality");
    }

    #[test]
    fn fluid_quality_cinematic_display_matches_name() {
        assert_eq!(format!("{}", FluidQualityPreset::Cinematic), "Cinematic");
    }

    // --- Equality tests ---

    #[test]
    fn fluid_quality_performance_equals_performance() {
        assert_eq!(
            FluidQualityPreset::Performance,
            FluidQualityPreset::Performance
        );
    }

    #[test]
    fn fluid_quality_balanced_equals_balanced() {
        assert_eq!(FluidQualityPreset::Balanced, FluidQualityPreset::Balanced);
    }

    #[test]
    fn fluid_quality_performance_not_equal_to_balanced() {
        assert_ne!(
            FluidQualityPreset::Performance,
            FluidQualityPreset::Balanced
        );
    }
}

// ============================================================================
// BrushMode Tests (5 variants)
// ============================================================================

mod brush_mode_tests {
    use super::*;

    // --- name() method tests - covers all 5 match arms ---

    #[test]
    fn brush_mode_sculpt_name_returns_sculpt() {
        assert_eq!(BrushMode::Sculpt.name(), "Sculpt");
    }

    #[test]
    fn brush_mode_smooth_name_returns_smooth() {
        assert_eq!(BrushMode::Smooth.name(), "Smooth");
    }

    #[test]
    fn brush_mode_flatten_name_returns_flatten() {
        assert_eq!(BrushMode::Flatten.name(), "Flatten");
    }

    #[test]
    fn brush_mode_paint_name_returns_paint() {
        assert_eq!(BrushMode::Paint.name(), "Paint");
    }

    #[test]
    fn brush_mode_erode_name_returns_erode() {
        assert_eq!(BrushMode::Erode.name(), "Erode");
    }

    // --- icon() method tests - covers all 5 match arms ---

    #[test]
    fn brush_mode_sculpt_icon_returns_mountain_emoji() {
        assert_eq!(BrushMode::Sculpt.icon(), "🏔️");
    }

    #[test]
    fn brush_mode_smooth_icon_returns_wave_emoji() {
        assert_eq!(BrushMode::Smooth.icon(), "〰️");
    }

    #[test]
    fn brush_mode_flatten_icon_returns_minus_emoji() {
        assert_eq!(BrushMode::Flatten.icon(), "-");
    }

    #[test]
    fn brush_mode_paint_icon_returns_paintbrush_emoji() {
        assert_eq!(BrushMode::Paint.icon(), "🖌️");
    }

    #[test]
    fn brush_mode_erode_icon_returns_water_emoji() {
        assert_eq!(BrushMode::Erode.icon(), "💧");
    }

    // --- Variant identity tests ---

    #[test]
    fn brush_mode_sculpt_is_not_smooth() {
        assert_ne!(BrushMode::Sculpt.name(), BrushMode::Smooth.name());
    }

    #[test]
    fn brush_mode_smooth_is_not_flatten() {
        assert_ne!(BrushMode::Smooth.name(), BrushMode::Flatten.name());
    }

    #[test]
    fn brush_mode_flatten_is_not_paint() {
        assert_ne!(BrushMode::Flatten.name(), BrushMode::Paint.name());
    }

    #[test]
    fn brush_mode_paint_is_not_erode() {
        assert_ne!(BrushMode::Paint.name(), BrushMode::Erode.name());
    }

    #[test]
    fn brush_mode_erode_is_not_sculpt() {
        assert_ne!(BrushMode::Erode.name(), BrushMode::Sculpt.name());
    }

    // --- Icon uniqueness tests ---

    #[test]
    fn brush_mode_sculpt_icon_is_not_smooth_icon() {
        assert_ne!(BrushMode::Sculpt.icon(), BrushMode::Smooth.icon());
    }

    #[test]
    fn brush_mode_smooth_icon_is_not_flatten_icon() {
        assert_ne!(BrushMode::Smooth.icon(), BrushMode::Flatten.icon());
    }

    #[test]
    fn brush_mode_flatten_icon_is_not_paint_icon() {
        assert_ne!(BrushMode::Flatten.icon(), BrushMode::Paint.icon());
    }

    #[test]
    fn brush_mode_paint_icon_is_not_erode_icon() {
        assert_ne!(BrushMode::Paint.icon(), BrushMode::Erode.icon());
    }

    // --- all() method tests ---

    #[test]
    fn brush_mode_all_returns_eight_variants() {
        assert_eq!(BrushMode::all().len(), 8);
    }

    #[test]
    fn brush_mode_all_contains_sculpt() {
        assert!(BrushMode::all().contains(&BrushMode::Sculpt));
    }

    #[test]
    fn brush_mode_all_contains_smooth() {
        assert!(BrushMode::all().contains(&BrushMode::Smooth));
    }

    #[test]
    fn brush_mode_all_contains_flatten() {
        assert!(BrushMode::all().contains(&BrushMode::Flatten));
    }

    #[test]
    fn brush_mode_all_contains_paint() {
        assert!(BrushMode::all().contains(&BrushMode::Paint));
    }

    #[test]
    fn brush_mode_all_contains_erode() {
        assert!(BrushMode::all().contains(&BrushMode::Erode));
    }

    // --- Display trait tests ---

    #[test]
    fn brush_mode_sculpt_display_contains_icon_and_name() {
        let display = format!("{}", BrushMode::Sculpt);
        assert!(display.contains("🏔️"));
        assert!(display.contains("Sculpt"));
    }

    #[test]
    fn brush_mode_smooth_display_contains_icon_and_name() {
        let display = format!("{}", BrushMode::Smooth);
        assert!(display.contains("〰️"));
        assert!(display.contains("Smooth"));
    }

    #[test]
    fn brush_mode_flatten_display_contains_icon_and_name() {
        let display = format!("{}", BrushMode::Flatten);
        assert!(display.contains("-"));
        assert!(display.contains("Flatten"));
    }

    #[test]
    fn brush_mode_paint_display_contains_icon_and_name() {
        let display = format!("{}", BrushMode::Paint);
        assert!(display.contains("🖌️"));
        assert!(display.contains("Paint"));
    }

    #[test]
    fn brush_mode_erode_display_contains_icon_and_name() {
        let display = format!("{}", BrushMode::Erode);
        assert!(display.contains("💧"));
        assert!(display.contains("Erode"));
    }

    // --- Equality tests ---

    #[test]
    fn brush_mode_sculpt_equals_sculpt() {
        assert_eq!(BrushMode::Sculpt, BrushMode::Sculpt);
    }

    #[test]
    fn brush_mode_smooth_equals_smooth() {
        assert_eq!(BrushMode::Smooth, BrushMode::Smooth);
    }

    #[test]
    fn brush_mode_sculpt_not_equal_to_smooth() {
        assert_ne!(BrushMode::Sculpt, BrushMode::Smooth);
    }

    // --- Hash tests ---

    #[test]
    fn brush_mode_all_variants_have_unique_hashes() {
        use std::collections::HashSet;
        let set: HashSet<BrushMode> = BrushMode::all().iter().copied().collect();
        assert_eq!(set.len(), 8);
    }

    // --- Boundary: all variants have non-empty names and icons ---

    #[test]
    fn brush_mode_all_variants_have_non_empty_names() {
        for mode in BrushMode::all() {
            assert!(!mode.name().is_empty());
        }
    }

    #[test]
    fn brush_mode_all_variants_have_non_empty_icons() {
        for mode in BrushMode::all() {
            assert!(!mode.icon().is_empty());
        }
    }
}

// ============================================================================
// Cross-Type Integration Tests
// ============================================================================

mod cross_type_integration_tests {
    use super::*;

    #[test]
    fn terrain_types_all_methods_return_non_empty_slices() {
        assert!(!ErosionPresetType::all().is_empty());
        assert!(!WaterBodyPreset::all().is_empty());
        assert!(!FluidQualityPreset::all().is_empty());
        assert!(!BrushMode::all().is_empty());
    }

    #[test]
    fn terrain_types_all_methods_return_correct_counts() {
        assert_eq!(ErosionPresetType::all().len(), 6);
        assert_eq!(WaterBodyPreset::all().len(), 7);
        assert_eq!(FluidQualityPreset::all().len(), 4);
        assert_eq!(BrushMode::all().len(), 8);
    }

    #[test]
    fn water_body_flowing_and_not_flowing_partitions_all_variants() {
        let flowing_count = WaterBodyPreset::all()
            .iter()
            .filter(|p| p.is_flowing())
            .count();
        let not_flowing_count = WaterBodyPreset::all()
            .iter()
            .filter(|p| !p.is_flowing())
            .count();
        assert_eq!(
            flowing_count + not_flowing_count,
            WaterBodyPreset::all().len()
        );
    }

    #[test]
    fn erosion_preset_names_are_all_unique() {
        let names: Vec<_> = ErosionPresetType::all().iter().map(|p| p.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn water_body_names_are_all_unique() {
        let names: Vec<_> = WaterBodyPreset::all().iter().map(|p| p.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn fluid_quality_names_are_all_unique() {
        let names: Vec<_> = FluidQualityPreset::all().iter().map(|p| p.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn brush_mode_names_are_all_unique() {
        let names: Vec<_> = BrushMode::all().iter().map(|p| p.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn brush_mode_icons_are_all_unique() {
        let icons: Vec<_> = BrushMode::all().iter().map(|p| p.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }
}
