//! Mutation-Resistant Tests for Post-Process Panel Types
//!
//! This module provides comprehensive tests for post-processing effect types to ensure
//! mutation testing achieves 90%+ kill rates through:
//! - Exhaustive match arm coverage for all enum variants
//! - Boundary condition testing for all value methods
//! - Boolean return path testing for all predicate methods
//! - Comparison and ordering tests where applicable

// Note: Using post_process_panel::Tonemapper (has is_cinematic), not environment_preset_panel::Tonemapper
use aw_editor_lib::panels::post_process_panel::{
    AntiAliasing, AoMethod, DofMode, PostProcessTab, Tonemapper,
};

// ============================================================================
// Tonemapper Tests (6 variants)
// ============================================================================

mod tonemapper_tests {
    use super::*;

    // --- name() method tests - covers all 6 match arms ---

    #[test]
    fn tonemapper_none_name_returns_none() {
        assert_eq!(Tonemapper::None.name(), "None");
    }

    #[test]
    fn tonemapper_reinhard_name_returns_reinhard() {
        assert_eq!(Tonemapper::Reinhard.name(), "Reinhard");
    }

    #[test]
    fn tonemapper_aces_name_returns_aces() {
        assert_eq!(Tonemapper::ACES.name(), "ACES");
    }

    #[test]
    fn tonemapper_filmic_name_returns_filmic() {
        assert_eq!(Tonemapper::Filmic.name(), "Filmic");
    }

    #[test]
    fn tonemapper_agx_name_returns_agx() {
        assert_eq!(Tonemapper::AgX.name(), "AgX");
    }

    #[test]
    fn tonemapper_neutral_name_returns_neutral() {
        assert_eq!(Tonemapper::Neutral.name(), "Neutral");
    }

    // --- is_cinematic() boolean return path tests ---
    // TRUE paths: ACES, Filmic, AgX (3)
    // FALSE paths: None, Reinhard, Neutral (3)

    #[test]
    fn tonemapper_aces_is_cinematic_returns_true() {
        assert!(Tonemapper::ACES.is_cinematic());
    }

    #[test]
    fn tonemapper_filmic_is_cinematic_returns_true() {
        assert!(Tonemapper::Filmic.is_cinematic());
    }

    #[test]
    fn tonemapper_agx_is_cinematic_returns_true() {
        assert!(Tonemapper::AgX.is_cinematic());
    }

    #[test]
    fn tonemapper_none_is_cinematic_returns_false() {
        assert!(!Tonemapper::None.is_cinematic());
    }

    #[test]
    fn tonemapper_reinhard_is_cinematic_returns_false() {
        assert!(!Tonemapper::Reinhard.is_cinematic());
    }

    #[test]
    fn tonemapper_neutral_is_cinematic_returns_false() {
        assert!(!Tonemapper::Neutral.is_cinematic());
    }

    // --- is_cinematic() explicit boolean comparisons ---

    #[test]
    fn tonemapper_aces_is_cinematic_equals_true() {
        assert_eq!(Tonemapper::ACES.is_cinematic(), true);
    }

    #[test]
    fn tonemapper_filmic_is_cinematic_equals_true() {
        assert_eq!(Tonemapper::Filmic.is_cinematic(), true);
    }

    #[test]
    fn tonemapper_agx_is_cinematic_equals_true() {
        assert_eq!(Tonemapper::AgX.is_cinematic(), true);
    }

    #[test]
    fn tonemapper_none_is_cinematic_equals_false() {
        assert_eq!(Tonemapper::None.is_cinematic(), false);
    }

    #[test]
    fn tonemapper_reinhard_is_cinematic_equals_false() {
        assert_eq!(Tonemapper::Reinhard.is_cinematic(), false);
    }

    #[test]
    fn tonemapper_neutral_is_cinematic_equals_false() {
        assert_eq!(Tonemapper::Neutral.is_cinematic(), false);
    }

    // --- is_cinematic() negation tests ---

    #[test]
    fn tonemapper_aces_is_not_not_cinematic() {
        assert!(!(!Tonemapper::ACES.is_cinematic()));
    }

    #[test]
    fn tonemapper_none_is_not_cinematic_negation() {
        assert!(!(Tonemapper::None.is_cinematic()));
    }

    // --- all() method tests ---

    #[test]
    fn tonemapper_all_returns_six_variants() {
        assert_eq!(Tonemapper::all().len(), 6);
    }

    #[test]
    fn tonemapper_all_contains_none() {
        assert!(Tonemapper::all().contains(&Tonemapper::None));
    }

    #[test]
    fn tonemapper_all_contains_reinhard() {
        assert!(Tonemapper::all().contains(&Tonemapper::Reinhard));
    }

    #[test]
    fn tonemapper_all_contains_aces() {
        assert!(Tonemapper::all().contains(&Tonemapper::ACES));
    }

    #[test]
    fn tonemapper_all_contains_filmic() {
        assert!(Tonemapper::all().contains(&Tonemapper::Filmic));
    }

    #[test]
    fn tonemapper_all_contains_agx() {
        assert!(Tonemapper::all().contains(&Tonemapper::AgX));
    }

    #[test]
    fn tonemapper_all_contains_neutral() {
        assert!(Tonemapper::all().contains(&Tonemapper::Neutral));
    }

    // --- Counting tests ---

    #[test]
    fn tonemapper_cinematic_count_equals_three() {
        let count = Tonemapper::all()
            .iter()
            .filter(|t| t.is_cinematic())
            .count();
        assert_eq!(count, 3);
    }

    #[test]
    fn tonemapper_not_cinematic_count_equals_three() {
        let count = Tonemapper::all()
            .iter()
            .filter(|t| !t.is_cinematic())
            .count();
        assert_eq!(count, 3);
    }

    // --- Display trait tests ---

    #[test]
    fn tonemapper_none_display_contains_icon_and_name() {
        let display = format!("{}", Tonemapper::None);
        assert!(display.contains("⚫"));
        assert!(display.contains("None"));
    }

    #[test]
    fn tonemapper_aces_display_contains_icon_and_name() {
        let display = format!("{}", Tonemapper::ACES);
        assert!(display.contains("🎞️"));
        assert!(display.contains("ACES"));
    }

    #[test]
    fn tonemapper_filmic_display_contains_icon_and_name() {
        let display = format!("{}", Tonemapper::Filmic);
        assert!(display.contains("[Anim]"));
        assert!(display.contains("Filmic"));
    }

    // --- icon() method tests - covers all 6 match arms ---

    #[test]
    fn tonemapper_none_icon_returns_black_circle() {
        assert_eq!(Tonemapper::None.icon(), "⚫");
    }

    #[test]
    fn tonemapper_reinhard_icon_returns_rainbow() {
        assert_eq!(Tonemapper::Reinhard.icon(), "🌈");
    }

    #[test]
    fn tonemapper_aces_icon_returns_film_strip() {
        assert_eq!(Tonemapper::ACES.icon(), "🎞️");
    }

    #[test]
    fn tonemapper_filmic_icon_returns_clapper() {
        assert_eq!(Tonemapper::Filmic.icon(), "[Anim]");
    }

    #[test]
    fn tonemapper_agx_icon_returns_framed_picture() {
        assert_eq!(Tonemapper::AgX.icon(), "[Img]");
    }

    #[test]
    fn tonemapper_neutral_icon_returns_balance() {
        assert_eq!(Tonemapper::Neutral.icon(), "⚖️");
    }

    // --- Equality tests ---

    #[test]
    fn tonemapper_aces_equals_aces() {
        assert_eq!(Tonemapper::ACES, Tonemapper::ACES);
    }

    #[test]
    fn tonemapper_none_not_equal_to_aces() {
        assert_ne!(Tonemapper::None, Tonemapper::ACES);
    }

    // --- Boundary: all variants have names ---

    #[test]
    fn tonemapper_all_variants_have_non_empty_names() {
        for tm in Tonemapper::all() {
            assert!(!tm.name().is_empty());
        }
    }
}

// ============================================================================
// AntiAliasing Tests (7 variants)
// ============================================================================

mod antialiasing_tests {
    use super::*;

    // --- name() method tests - covers all 7 match arms ---

    #[test]
    fn aa_none_name_returns_none() {
        assert_eq!(AntiAliasing::None.name(), "None");
    }

    #[test]
    fn aa_fxaa_name_returns_fxaa() {
        assert_eq!(AntiAliasing::FXAA.name(), "FXAA");
    }

    #[test]
    fn aa_smaa_name_returns_smaa() {
        assert_eq!(AntiAliasing::SMAA.name(), "SMAA");
    }

    #[test]
    fn aa_taa_name_returns_taa() {
        assert_eq!(AntiAliasing::TAA.name(), "TAA");
    }

    #[test]
    fn aa_msaa2x_name_returns_msaa_2x() {
        assert_eq!(AntiAliasing::MSAA2x.name(), "MSAA 2x");
    }

    #[test]
    fn aa_msaa4x_name_returns_msaa_4x() {
        assert_eq!(AntiAliasing::MSAA4x.name(), "MSAA 4x");
    }

    #[test]
    fn aa_msaa8x_name_returns_msaa_8x() {
        assert_eq!(AntiAliasing::MSAA8x.name(), "MSAA 8x");
    }

    // --- is_msaa() boolean return path tests ---
    // TRUE paths: MSAA2x, MSAA4x, MSAA8x (3)
    // FALSE paths: None, FXAA, SMAA, TAA (4)

    #[test]
    fn aa_msaa2x_is_msaa_returns_true() {
        assert!(AntiAliasing::MSAA2x.is_msaa());
    }

    #[test]
    fn aa_msaa4x_is_msaa_returns_true() {
        assert!(AntiAliasing::MSAA4x.is_msaa());
    }

    #[test]
    fn aa_msaa8x_is_msaa_returns_true() {
        assert!(AntiAliasing::MSAA8x.is_msaa());
    }

    #[test]
    fn aa_none_is_msaa_returns_false() {
        assert!(!AntiAliasing::None.is_msaa());
    }

    #[test]
    fn aa_fxaa_is_msaa_returns_false() {
        assert!(!AntiAliasing::FXAA.is_msaa());
    }

    #[test]
    fn aa_smaa_is_msaa_returns_false() {
        assert!(!AntiAliasing::SMAA.is_msaa());
    }

    #[test]
    fn aa_taa_is_msaa_returns_false() {
        assert!(!AntiAliasing::TAA.is_msaa());
    }

    // --- is_msaa() explicit boolean comparisons ---

    #[test]
    fn aa_msaa2x_is_msaa_equals_true() {
        assert_eq!(AntiAliasing::MSAA2x.is_msaa(), true);
    }

    #[test]
    fn aa_msaa4x_is_msaa_equals_true() {
        assert_eq!(AntiAliasing::MSAA4x.is_msaa(), true);
    }

    #[test]
    fn aa_msaa8x_is_msaa_equals_true() {
        assert_eq!(AntiAliasing::MSAA8x.is_msaa(), true);
    }

    #[test]
    fn aa_none_is_msaa_equals_false() {
        assert_eq!(AntiAliasing::None.is_msaa(), false);
    }

    #[test]
    fn aa_fxaa_is_msaa_equals_false() {
        assert_eq!(AntiAliasing::FXAA.is_msaa(), false);
    }

    // --- is_post_process() boolean return path tests ---
    // TRUE paths: FXAA, SMAA, TAA (3)
    // FALSE paths: None, MSAA2x, MSAA4x, MSAA8x (4)

    #[test]
    fn aa_fxaa_is_post_process_returns_true() {
        assert!(AntiAliasing::FXAA.is_post_process());
    }

    #[test]
    fn aa_smaa_is_post_process_returns_true() {
        assert!(AntiAliasing::SMAA.is_post_process());
    }

    #[test]
    fn aa_taa_is_post_process_returns_true() {
        assert!(AntiAliasing::TAA.is_post_process());
    }

    #[test]
    fn aa_none_is_post_process_returns_false() {
        assert!(!AntiAliasing::None.is_post_process());
    }

    #[test]
    fn aa_msaa2x_is_post_process_returns_false() {
        assert!(!AntiAliasing::MSAA2x.is_post_process());
    }

    #[test]
    fn aa_msaa4x_is_post_process_returns_false() {
        assert!(!AntiAliasing::MSAA4x.is_post_process());
    }

    #[test]
    fn aa_msaa8x_is_post_process_returns_false() {
        assert!(!AntiAliasing::MSAA8x.is_post_process());
    }

    // --- is_post_process() explicit boolean comparisons ---

    #[test]
    fn aa_fxaa_is_post_process_equals_true() {
        assert_eq!(AntiAliasing::FXAA.is_post_process(), true);
    }

    #[test]
    fn aa_smaa_is_post_process_equals_true() {
        assert_eq!(AntiAliasing::SMAA.is_post_process(), true);
    }

    #[test]
    fn aa_taa_is_post_process_equals_true() {
        assert_eq!(AntiAliasing::TAA.is_post_process(), true);
    }

    #[test]
    fn aa_none_is_post_process_equals_false() {
        assert_eq!(AntiAliasing::None.is_post_process(), false);
    }

    #[test]
    fn aa_msaa4x_is_post_process_equals_false() {
        assert_eq!(AntiAliasing::MSAA4x.is_post_process(), false);
    }

    // --- Mutual exclusivity tests: is_msaa() XOR is_post_process() OR Neither ---

    #[test]
    fn aa_none_is_neither_msaa_nor_post_process() {
        assert!(!AntiAliasing::None.is_msaa());
        assert!(!AntiAliasing::None.is_post_process());
    }

    #[test]
    fn aa_fxaa_is_post_process_but_not_msaa() {
        assert!(!AntiAliasing::FXAA.is_msaa());
        assert!(AntiAliasing::FXAA.is_post_process());
    }

    #[test]
    fn aa_msaa4x_is_msaa_but_not_post_process() {
        assert!(AntiAliasing::MSAA4x.is_msaa());
        assert!(!AntiAliasing::MSAA4x.is_post_process());
    }

    #[test]
    fn aa_no_variant_is_both_msaa_and_post_process() {
        for aa in AntiAliasing::all() {
            let both = aa.is_msaa() && aa.is_post_process();
            assert!(!both, "{:?} is both MSAA and post-process", aa);
        }
    }

    // --- all() method tests ---

    #[test]
    fn aa_all_returns_seven_variants() {
        assert_eq!(AntiAliasing::all().len(), 7);
    }

    #[test]
    fn aa_all_contains_none() {
        assert!(AntiAliasing::all().contains(&AntiAliasing::None));
    }

    #[test]
    fn aa_all_contains_fxaa() {
        assert!(AntiAliasing::all().contains(&AntiAliasing::FXAA));
    }

    #[test]
    fn aa_all_contains_smaa() {
        assert!(AntiAliasing::all().contains(&AntiAliasing::SMAA));
    }

    #[test]
    fn aa_all_contains_taa() {
        assert!(AntiAliasing::all().contains(&AntiAliasing::TAA));
    }

    #[test]
    fn aa_all_contains_msaa2x() {
        assert!(AntiAliasing::all().contains(&AntiAliasing::MSAA2x));
    }

    #[test]
    fn aa_all_contains_msaa4x() {
        assert!(AntiAliasing::all().contains(&AntiAliasing::MSAA4x));
    }

    #[test]
    fn aa_all_contains_msaa8x() {
        assert!(AntiAliasing::all().contains(&AntiAliasing::MSAA8x));
    }

    // --- Counting tests ---

    #[test]
    fn aa_msaa_count_equals_three() {
        let count = AntiAliasing::all().iter().filter(|aa| aa.is_msaa()).count();
        assert_eq!(count, 3);
    }

    #[test]
    fn aa_post_process_count_equals_three() {
        let count = AntiAliasing::all()
            .iter()
            .filter(|aa| aa.is_post_process())
            .count();
        assert_eq!(count, 3);
    }

    #[test]
    fn aa_neither_count_equals_one() {
        let count = AntiAliasing::all()
            .iter()
            .filter(|aa| !aa.is_msaa() && !aa.is_post_process())
            .count();
        assert_eq!(count, 1);
    }

    // --- Display trait tests ---

    #[test]
    fn aa_none_display_contains_name() {
        let display = format!("{}", AntiAliasing::None);
        assert!(display.contains("None"));
    }

    #[test]
    fn aa_fxaa_display_contains_name() {
        let display = format!("{}", AntiAliasing::FXAA);
        assert!(display.contains("FXAA"));
    }

    #[test]
    fn aa_msaa4x_display_contains_name() {
        let display = format!("{}", AntiAliasing::MSAA4x);
        assert!(display.contains("MSAA 4x"));
    }

    // --- Equality tests ---

    #[test]
    fn aa_msaa4x_equals_msaa4x() {
        assert_eq!(AntiAliasing::MSAA4x, AntiAliasing::MSAA4x);
    }

    #[test]
    fn aa_none_not_equal_to_fxaa() {
        assert_ne!(AntiAliasing::None, AntiAliasing::FXAA);
    }
}

// ============================================================================
// DofMode Tests (4 variants)
// ============================================================================

mod dof_mode_tests {
    use super::*;

    // --- name() method tests - covers all 4 match arms ---
    // Variants: Disabled, Gaussian, Bokeh, CircleOfConfusion

    #[test]
    fn dof_disabled_name_returns_disabled() {
        assert_eq!(DofMode::Disabled.name(), "Disabled");
    }

    #[test]
    fn dof_gaussian_name_returns_gaussian() {
        assert_eq!(DofMode::Gaussian.name(), "Gaussian");
    }

    #[test]
    fn dof_bokeh_name_returns_bokeh() {
        assert_eq!(DofMode::Bokeh.name(), "Bokeh");
    }

    #[test]
    fn dof_circle_of_confusion_name_returns_circle_of_confusion() {
        assert_eq!(DofMode::CircleOfConfusion.name(), "Circle of Confusion");
    }

    // --- icon() method tests - covers all 4 match arms ---

    #[test]
    fn dof_disabled_icon_returns_black_circle() {
        assert_eq!(DofMode::Disabled.icon(), "⚫");
    }

    #[test]
    fn dof_gaussian_icon_returns_fog() {
        assert_eq!(DofMode::Gaussian.icon(), "[Fog]");
    }

    #[test]
    fn dof_bokeh_icon_returns_sparkles() {
        assert_eq!(DofMode::Bokeh.icon(), "[Fx]");
    }

    #[test]
    fn dof_circle_of_confusion_icon_returns_circle() {
        assert_eq!(DofMode::CircleOfConfusion.icon(), "○");
    }

    // --- is_enabled() boolean return path tests ---
    // TRUE paths: Gaussian, Bokeh, CircleOfConfusion (3)
    // FALSE paths: Disabled (1)

    #[test]
    fn dof_gaussian_is_enabled_returns_true() {
        assert!(DofMode::Gaussian.is_enabled());
    }

    #[test]
    fn dof_bokeh_is_enabled_returns_true() {
        assert!(DofMode::Bokeh.is_enabled());
    }

    #[test]
    fn dof_circle_of_confusion_is_enabled_returns_true() {
        assert!(DofMode::CircleOfConfusion.is_enabled());
    }

    #[test]
    fn dof_disabled_is_enabled_returns_false() {
        assert!(!DofMode::Disabled.is_enabled());
    }

    // --- is_enabled() explicit boolean comparisons ---

    #[test]
    fn dof_gaussian_is_enabled_equals_true() {
        assert_eq!(DofMode::Gaussian.is_enabled(), true);
    }

    #[test]
    fn dof_bokeh_is_enabled_equals_true() {
        assert_eq!(DofMode::Bokeh.is_enabled(), true);
    }

    #[test]
    fn dof_circle_of_confusion_is_enabled_equals_true() {
        assert_eq!(DofMode::CircleOfConfusion.is_enabled(), true);
    }

    #[test]
    fn dof_disabled_is_enabled_equals_false() {
        assert_eq!(DofMode::Disabled.is_enabled(), false);
    }

    // --- is_enabled() negation tests ---

    #[test]
    fn dof_gaussian_is_not_not_enabled() {
        assert!(!(!DofMode::Gaussian.is_enabled()));
    }

    #[test]
    fn dof_disabled_is_not_enabled_negation() {
        assert!(!(DofMode::Disabled.is_enabled()));
    }

    // --- all() method tests ---

    #[test]
    fn dof_all_returns_four_variants() {
        assert_eq!(DofMode::all().len(), 4);
    }

    #[test]
    fn dof_all_contains_disabled() {
        assert!(DofMode::all().contains(&DofMode::Disabled));
    }

    #[test]
    fn dof_all_contains_gaussian() {
        assert!(DofMode::all().contains(&DofMode::Gaussian));
    }

    #[test]
    fn dof_all_contains_bokeh() {
        assert!(DofMode::all().contains(&DofMode::Bokeh));
    }

    #[test]
    fn dof_all_contains_circle_of_confusion() {
        assert!(DofMode::all().contains(&DofMode::CircleOfConfusion));
    }

    // --- Counting tests ---

    #[test]
    fn dof_enabled_count_equals_three() {
        let count = DofMode::all().iter().filter(|d| d.is_enabled()).count();
        assert_eq!(count, 3);
    }

    #[test]
    fn dof_disabled_count_equals_one() {
        let count = DofMode::all().iter().filter(|d| !d.is_enabled()).count();
        assert_eq!(count, 1);
    }

    // --- Display trait tests ---

    #[test]
    fn dof_disabled_display_contains_icon_and_name() {
        let display = format!("{}", DofMode::Disabled);
        assert!(display.contains("⚫"));
        assert!(display.contains("Disabled"));
    }

    #[test]
    fn dof_bokeh_display_contains_icon_and_name() {
        let display = format!("{}", DofMode::Bokeh);
        assert!(display.contains("[Fx]"));
        assert!(display.contains("Bokeh"));
    }

    // --- Equality tests ---

    #[test]
    fn dof_gaussian_equals_gaussian() {
        assert_eq!(DofMode::Gaussian, DofMode::Gaussian);
    }

    #[test]
    fn dof_disabled_not_equal_to_gaussian() {
        assert_ne!(DofMode::Disabled, DofMode::Gaussian);
    }

    // --- Default trait test ---

    #[test]
    fn dof_default_is_disabled() {
        assert_eq!(DofMode::default(), DofMode::Disabled);
    }

    // --- Boundary: all variants have names and icons ---

    #[test]
    fn dof_all_variants_have_non_empty_names() {
        for dof in DofMode::all() {
            assert!(!dof.name().is_empty());
        }
    }

    #[test]
    fn dof_all_variants_have_non_empty_icons() {
        for dof in DofMode::all() {
            assert!(!dof.icon().is_empty());
        }
    }

    // --- Hash uniqueness tests ---

    #[test]
    fn dof_all_variants_have_unique_hashes() {
        use std::collections::HashSet;
        let set: HashSet<DofMode> = DofMode::all().iter().copied().collect();
        assert_eq!(set.len(), 4);
    }
}

// ============================================================================
// AoMethod Tests (3 variants)
// ============================================================================

mod ao_method_tests {
    use super::*;

    // --- name() method tests - covers all 3 match arms ---

    #[test]
    fn ao_ssao_name_returns_ssao() {
        assert_eq!(AoMethod::SSAO.name(), "SSAO");
    }

    #[test]
    fn ao_hbao_name_returns_hbao_plus() {
        assert_eq!(AoMethod::HBAO.name(), "HBAO+");
    }

    #[test]
    fn ao_gtao_name_returns_gtao() {
        assert_eq!(AoMethod::GTAO.name(), "GTAO");
    }

    // --- icon() method tests - covers all 3 match arms ---

    #[test]
    fn ao_ssao_icon_returns_black_heart() {
        assert_eq!(AoMethod::SSAO.icon(), "🖤");
    }

    #[test]
    fn ao_hbao_icon_returns_new_moon() {
        assert_eq!(AoMethod::HBAO.icon(), "🌑");
    }

    #[test]
    fn ao_gtao_icon_returns_new_moon_face() {
        assert_eq!(AoMethod::GTAO.icon(), "🌚");
    }

    // --- description() method tests - covers all 3 match arms ---

    #[test]
    fn ao_ssao_description_returns_screen_space() {
        assert_eq!(
            AoMethod::SSAO.description(),
            "Screen-space ambient occlusion"
        );
    }

    #[test]
    fn ao_hbao_description_returns_horizon_based() {
        assert_eq!(
            AoMethod::HBAO.description(),
            "Horizon-based ambient occlusion"
        );
    }

    #[test]
    fn ao_gtao_description_returns_ground_truth() {
        assert_eq!(
            AoMethod::GTAO.description(),
            "Ground-truth ambient occlusion"
        );
    }

    // --- Variant identity tests ---

    #[test]
    fn ao_ssao_name_is_not_hbao_name() {
        assert_ne!(AoMethod::SSAO.name(), AoMethod::HBAO.name());
    }

    #[test]
    fn ao_hbao_name_is_not_gtao_name() {
        assert_ne!(AoMethod::HBAO.name(), AoMethod::GTAO.name());
    }

    #[test]
    fn ao_gtao_name_is_not_ssao_name() {
        assert_ne!(AoMethod::GTAO.name(), AoMethod::SSAO.name());
    }

    #[test]
    fn ao_ssao_icon_is_not_hbao_icon() {
        assert_ne!(AoMethod::SSAO.icon(), AoMethod::HBAO.icon());
    }

    #[test]
    fn ao_hbao_icon_is_not_gtao_icon() {
        assert_ne!(AoMethod::HBAO.icon(), AoMethod::GTAO.icon());
    }

    #[test]
    fn ao_gtao_icon_is_not_ssao_icon() {
        assert_ne!(AoMethod::GTAO.icon(), AoMethod::SSAO.icon());
    }

    #[test]
    fn ao_ssao_description_is_not_hbao_description() {
        assert_ne!(AoMethod::SSAO.description(), AoMethod::HBAO.description());
    }

    #[test]
    fn ao_hbao_description_is_not_gtao_description() {
        assert_ne!(AoMethod::HBAO.description(), AoMethod::GTAO.description());
    }

    // --- all() method tests ---

    #[test]
    fn ao_all_returns_three_variants() {
        assert_eq!(AoMethod::all().len(), 3);
    }

    #[test]
    fn ao_all_contains_ssao() {
        assert!(AoMethod::all().contains(&AoMethod::SSAO));
    }

    #[test]
    fn ao_all_contains_hbao() {
        assert!(AoMethod::all().contains(&AoMethod::HBAO));
    }

    #[test]
    fn ao_all_contains_gtao() {
        assert!(AoMethod::all().contains(&AoMethod::GTAO));
    }

    // --- Equality tests ---

    #[test]
    fn ao_ssao_equals_ssao() {
        assert_eq!(AoMethod::SSAO, AoMethod::SSAO);
    }

    #[test]
    fn ao_hbao_equals_hbao() {
        assert_eq!(AoMethod::HBAO, AoMethod::HBAO);
    }

    #[test]
    fn ao_gtao_equals_gtao() {
        assert_eq!(AoMethod::GTAO, AoMethod::GTAO);
    }

    #[test]
    fn ao_ssao_not_equal_to_hbao() {
        assert_ne!(AoMethod::SSAO, AoMethod::HBAO);
    }

    #[test]
    fn ao_hbao_not_equal_to_gtao() {
        assert_ne!(AoMethod::HBAO, AoMethod::GTAO);
    }

    // --- Hash uniqueness tests ---

    #[test]
    fn ao_all_variants_have_unique_hashes() {
        use std::collections::HashSet;
        let set: HashSet<AoMethod> = AoMethod::all().iter().copied().collect();
        assert_eq!(set.len(), 3);
    }

    // --- Boundary: all variants have non-empty properties ---

    #[test]
    fn ao_all_variants_have_non_empty_names() {
        for ao in AoMethod::all() {
            assert!(!ao.name().is_empty());
        }
    }

    #[test]
    fn ao_all_variants_have_non_empty_icons() {
        for ao in AoMethod::all() {
            assert!(!ao.icon().is_empty());
        }
    }

    #[test]
    fn ao_all_variants_have_non_empty_descriptions() {
        for ao in AoMethod::all() {
            assert!(!ao.description().is_empty());
        }
    }
}

// ============================================================================
// PostProcessTab Tests (7 variants)
// ============================================================================

mod post_process_tab_tests {
    use super::*;

    // --- name() method tests - covers all 7 match arms ---

    #[test]
    fn pptab_overview_name_returns_overview() {
        assert_eq!(PostProcessTab::Overview.name(), "Overview");
    }

    #[test]
    fn pptab_bloom_name_returns_bloom() {
        assert_eq!(PostProcessTab::Bloom.name(), "Bloom");
    }

    #[test]
    fn pptab_depth_of_field_name_returns_depth_of_field() {
        assert_eq!(PostProcessTab::DepthOfField.name(), "Depth of Field");
    }

    #[test]
    fn pptab_motion_blur_name_returns_motion_blur() {
        assert_eq!(PostProcessTab::MotionBlur.name(), "Motion Blur");
    }

    #[test]
    fn pptab_color_grading_name_returns_color_grading() {
        assert_eq!(PostProcessTab::ColorGrading.name(), "Color Grading");
    }

    #[test]
    fn pptab_effects_name_returns_effects() {
        assert_eq!(PostProcessTab::Effects.name(), "Effects");
    }

    #[test]
    fn pptab_presets_name_returns_presets() {
        assert_eq!(PostProcessTab::Presets.name(), "Presets");
    }

    // --- icon() method tests - covers all 7 match arms ---

    #[test]
    fn pptab_overview_icon_returns_clipboard() {
        assert_eq!(PostProcessTab::Overview.icon(), "[List]");
    }

    #[test]
    fn pptab_bloom_icon_returns_sparkles() {
        assert_eq!(PostProcessTab::Bloom.icon(), "[Fx]");
    }

    #[test]
    fn pptab_depth_of_field_icon_returns_camera() {
        assert_eq!(PostProcessTab::DepthOfField.icon(), "📷");
    }

    #[test]
    fn pptab_motion_blur_icon_returns_ski() {
        assert_eq!(PostProcessTab::MotionBlur.icon(), "🎿");
    }

    #[test]
    fn pptab_color_grading_icon_returns_palette() {
        assert_eq!(PostProcessTab::ColorGrading.icon(), "[Art]");
    }

    #[test]
    fn pptab_effects_icon_returns_star() {
        assert_eq!(PostProcessTab::Effects.icon(), "🌟");
    }

    #[test]
    fn pptab_presets_icon_returns_floppy() {
        assert_eq!(PostProcessTab::Presets.icon(), "[Save]");
    }

    // --- Variant identity tests ---

    #[test]
    fn pptab_overview_is_not_bloom() {
        assert_ne!(
            PostProcessTab::Overview.name(),
            PostProcessTab::Bloom.name()
        );
    }

    #[test]
    fn pptab_bloom_is_not_depth_of_field() {
        assert_ne!(
            PostProcessTab::Bloom.name(),
            PostProcessTab::DepthOfField.name()
        );
    }

    #[test]
    fn pptab_depth_of_field_is_not_motion_blur() {
        assert_ne!(
            PostProcessTab::DepthOfField.name(),
            PostProcessTab::MotionBlur.name()
        );
    }

    #[test]
    fn pptab_motion_blur_is_not_color_grading() {
        assert_ne!(
            PostProcessTab::MotionBlur.name(),
            PostProcessTab::ColorGrading.name()
        );
    }

    #[test]
    fn pptab_color_grading_is_not_effects() {
        assert_ne!(
            PostProcessTab::ColorGrading.name(),
            PostProcessTab::Effects.name()
        );
    }

    #[test]
    fn pptab_effects_is_not_presets() {
        assert_ne!(
            PostProcessTab::Effects.name(),
            PostProcessTab::Presets.name()
        );
    }

    // --- Icon uniqueness tests ---

    #[test]
    fn pptab_overview_icon_is_not_bloom_icon() {
        assert_ne!(
            PostProcessTab::Overview.icon(),
            PostProcessTab::Bloom.icon()
        );
    }

    #[test]
    fn pptab_bloom_icon_is_not_depth_of_field_icon() {
        assert_ne!(
            PostProcessTab::Bloom.icon(),
            PostProcessTab::DepthOfField.icon()
        );
    }

    #[test]
    fn pptab_depth_of_field_icon_is_not_motion_blur_icon() {
        assert_ne!(
            PostProcessTab::DepthOfField.icon(),
            PostProcessTab::MotionBlur.icon()
        );
    }

    // --- all() method tests ---

    #[test]
    fn pptab_all_returns_seven_variants() {
        assert_eq!(PostProcessTab::all().len(), 7);
    }

    #[test]
    fn pptab_all_contains_overview() {
        assert!(PostProcessTab::all().contains(&PostProcessTab::Overview));
    }

    #[test]
    fn pptab_all_contains_bloom() {
        assert!(PostProcessTab::all().contains(&PostProcessTab::Bloom));
    }

    #[test]
    fn pptab_all_contains_depth_of_field() {
        assert!(PostProcessTab::all().contains(&PostProcessTab::DepthOfField));
    }

    #[test]
    fn pptab_all_contains_motion_blur() {
        assert!(PostProcessTab::all().contains(&PostProcessTab::MotionBlur));
    }

    #[test]
    fn pptab_all_contains_color_grading() {
        assert!(PostProcessTab::all().contains(&PostProcessTab::ColorGrading));
    }

    #[test]
    fn pptab_all_contains_effects() {
        assert!(PostProcessTab::all().contains(&PostProcessTab::Effects));
    }

    #[test]
    fn pptab_all_contains_presets() {
        assert!(PostProcessTab::all().contains(&PostProcessTab::Presets));
    }

    // --- Display trait tests ---

    #[test]
    fn pptab_overview_display_contains_icon_and_name() {
        let display = format!("{}", PostProcessTab::Overview);
        assert!(display.contains("[List]"));
        assert!(display.contains("Overview"));
    }

    #[test]
    fn pptab_bloom_display_contains_icon_and_name() {
        let display = format!("{}", PostProcessTab::Bloom);
        assert!(display.contains("[Fx]"));
        assert!(display.contains("Bloom"));
    }

    // --- Default trait test ---

    #[test]
    fn pptab_default_is_overview() {
        assert_eq!(PostProcessTab::default(), PostProcessTab::Overview);
    }

    // --- Equality tests ---

    #[test]
    fn pptab_overview_equals_overview() {
        assert_eq!(PostProcessTab::Overview, PostProcessTab::Overview);
    }

    #[test]
    fn pptab_bloom_equals_bloom() {
        assert_eq!(PostProcessTab::Bloom, PostProcessTab::Bloom);
    }

    #[test]
    fn pptab_overview_not_equal_to_bloom() {
        assert_ne!(PostProcessTab::Overview, PostProcessTab::Bloom);
    }

    // --- Hash uniqueness tests ---

    #[test]
    fn pptab_all_variants_have_unique_hashes() {
        use std::collections::HashSet;
        let set: HashSet<PostProcessTab> = PostProcessTab::all().iter().copied().collect();
        assert_eq!(set.len(), 7);
    }

    // --- Boundary: all variants have non-empty properties ---

    #[test]
    fn pptab_all_variants_have_non_empty_names() {
        for tab in PostProcessTab::all() {
            assert!(!tab.name().is_empty());
        }
    }

    #[test]
    fn pptab_all_variants_have_non_empty_icons() {
        for tab in PostProcessTab::all() {
            assert!(!tab.icon().is_empty());
        }
    }
}

// ============================================================================
// Cross-Type Integration Tests
// ============================================================================

mod cross_type_integration_tests {
    use super::*;

    #[test]
    fn post_process_types_all_methods_return_non_empty_slices() {
        assert!(!Tonemapper::all().is_empty());
        assert!(!AntiAliasing::all().is_empty());
        assert!(!DofMode::all().is_empty());
        assert!(!AoMethod::all().is_empty());
        assert!(!PostProcessTab::all().is_empty());
    }

    #[test]
    fn post_process_types_all_methods_return_correct_counts() {
        assert_eq!(Tonemapper::all().len(), 6);
        assert_eq!(AntiAliasing::all().len(), 7);
        assert_eq!(DofMode::all().len(), 4);
        assert_eq!(AoMethod::all().len(), 3);
        assert_eq!(PostProcessTab::all().len(), 7);
    }

    #[test]
    fn tonemapper_cinematic_and_not_cinematic_partitions_all_variants() {
        let cinematic_count = Tonemapper::all()
            .iter()
            .filter(|t| t.is_cinematic())
            .count();
        let not_cinematic_count = Tonemapper::all()
            .iter()
            .filter(|t| !t.is_cinematic())
            .count();
        assert_eq!(
            cinematic_count + not_cinematic_count,
            Tonemapper::all().len()
        );
    }

    #[test]
    fn dof_enabled_and_disabled_partitions_all_variants() {
        let enabled_count = DofMode::all().iter().filter(|d| d.is_enabled()).count();
        let disabled_count = DofMode::all().iter().filter(|d| !d.is_enabled()).count();
        assert_eq!(enabled_count + disabled_count, DofMode::all().len());
    }

    #[test]
    fn antialiasing_msaa_plus_post_plus_neither_partitions_all_variants() {
        let msaa_count = AntiAliasing::all().iter().filter(|aa| aa.is_msaa()).count();
        let post_count = AntiAliasing::all()
            .iter()
            .filter(|aa| aa.is_post_process())
            .count();
        let neither_count = AntiAliasing::all()
            .iter()
            .filter(|aa| !aa.is_msaa() && !aa.is_post_process())
            .count();
        assert_eq!(
            msaa_count + post_count + neither_count,
            AntiAliasing::all().len()
        );
    }

    #[test]
    fn tonemapper_names_are_all_unique() {
        let names: Vec<_> = Tonemapper::all().iter().map(|t| t.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn antialiasing_names_are_all_unique() {
        let names: Vec<_> = AntiAliasing::all().iter().map(|aa| aa.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn dof_mode_names_are_all_unique() {
        let names: Vec<_> = DofMode::all().iter().map(|d| d.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn ao_method_names_are_all_unique() {
        let names: Vec<_> = AoMethod::all().iter().map(|ao| ao.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn post_process_tab_names_are_all_unique() {
        let names: Vec<_> = PostProcessTab::all().iter().map(|t| t.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn post_process_tab_icons_are_all_unique() {
        let icons: Vec<_> = PostProcessTab::all().iter().map(|t| t.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    #[test]
    fn ao_method_icons_are_all_unique() {
        let icons: Vec<_> = AoMethod::all().iter().map(|ao| ao.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }
}
