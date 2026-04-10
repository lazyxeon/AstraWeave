//! Mutation-resistant tests for Animation Panel system
//!
//! These tests target boundary conditions, comparison operators, and boolean return paths
//! to achieve high mutation testing kill rates (90%+).

use aw_editor_lib::panels::animation_panel::{
    BlendTreeType, BlendingMode, ConditionType, ParameterType, ParameterValue, TangentMode,
};
use aw_editor_lib::panels::LoopMode;
use std::collections::HashSet;

// ============================================================================
// LOOP MODE TESTS
// ============================================================================

mod loop_mode_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(LoopMode::all().len(), 4);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = LoopMode::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = LoopMode::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    #[test]
    fn test_all_descriptions_unique() {
        let descs: Vec<&str> = LoopMode::all().iter().map(|c| c.description()).collect();
        let unique: HashSet<_> = descs.iter().collect();
        assert_eq!(descs.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_once_name() {
        assert_eq!(LoopMode::Once.name(), "Once");
    }

    #[test]
    fn test_loop_name() {
        assert_eq!(LoopMode::Loop.name(), "Loop");
    }

    #[test]
    fn test_ping_pong_name() {
        assert_eq!(LoopMode::PingPong.name(), "Ping-Pong");
    }

    #[test]
    fn test_clamp_forever_name() {
        assert_eq!(LoopMode::ClampForever.name(), "Clamp Forever");
    }

    // Test is_looping()
    #[test]
    fn test_loop_is_looping() {
        assert!(LoopMode::Loop.is_looping());
    }

    #[test]
    fn test_ping_pong_is_looping() {
        assert!(LoopMode::PingPong.is_looping());
    }

    #[test]
    fn test_once_is_not_looping() {
        assert!(!LoopMode::Once.is_looping());
    }

    #[test]
    fn test_clamp_forever_is_not_looping() {
        assert!(!LoopMode::ClampForever.is_looping());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", LoopMode::Loop);
        assert!(display.contains("🔁"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", LoopMode::PingPong);
        assert!(display.contains("Ping-Pong"));
    }

    // Test default
    #[test]
    fn test_default_is_loop() {
        assert_eq!(LoopMode::default(), LoopMode::Loop);
    }
}

// ============================================================================
// TANGENT MODE TESTS
// ============================================================================

mod tangent_mode_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(TangentMode::all().len(), 5);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = TangentMode::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = TangentMode::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_auto_name() {
        assert_eq!(TangentMode::Auto.name(), "Auto");
    }

    #[test]
    fn test_linear_name() {
        assert_eq!(TangentMode::Linear.name(), "Linear");
    }

    #[test]
    fn test_constant_name() {
        assert_eq!(TangentMode::Constant.name(), "Constant");
    }

    #[test]
    fn test_free_name() {
        assert_eq!(TangentMode::Free.name(), "Free");
    }

    #[test]
    fn test_broken_name() {
        assert_eq!(TangentMode::Broken.name(), "Broken");
    }

    // Test is_smooth()
    #[test]
    fn test_auto_is_smooth() {
        assert!(TangentMode::Auto.is_smooth());
    }

    #[test]
    fn test_free_is_smooth() {
        assert!(TangentMode::Free.is_smooth());
    }

    #[test]
    fn test_linear_is_not_smooth() {
        assert!(!TangentMode::Linear.is_smooth());
    }

    #[test]
    fn test_constant_is_not_smooth() {
        assert!(!TangentMode::Constant.is_smooth());
    }

    #[test]
    fn test_broken_is_not_smooth() {
        assert!(!TangentMode::Broken.is_smooth());
    }

    // Test is_editable()
    #[test]
    fn test_free_is_editable() {
        assert!(TangentMode::Free.is_editable());
    }

    #[test]
    fn test_broken_is_editable() {
        assert!(TangentMode::Broken.is_editable());
    }

    #[test]
    fn test_auto_is_not_editable() {
        assert!(!TangentMode::Auto.is_editable());
    }

    #[test]
    fn test_linear_is_not_editable() {
        assert!(!TangentMode::Linear.is_editable());
    }

    #[test]
    fn test_constant_is_not_editable() {
        assert!(!TangentMode::Constant.is_editable());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", TangentMode::Auto);
        assert!(display.contains("[Sync]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", TangentMode::Linear);
        assert!(display.contains("Linear"));
    }

    // Test default
    #[test]
    fn test_default_is_auto() {
        assert_eq!(TangentMode::default(), TangentMode::Auto);
    }
}

// ============================================================================
// CONDITION TYPE TESTS
// ============================================================================

mod condition_type_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(ConditionType::all().len(), 6);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = ConditionType::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_symbols_unique() {
        let symbols: Vec<&str> = ConditionType::all().iter().map(|c| c.symbol()).collect();
        let unique: HashSet<_> = symbols.iter().collect();
        assert_eq!(symbols.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_greater_name() {
        assert_eq!(ConditionType::Greater.name(), "Greater");
    }

    #[test]
    fn test_less_name() {
        assert_eq!(ConditionType::Less.name(), "Less");
    }

    #[test]
    fn test_equals_name() {
        assert_eq!(ConditionType::Equals.name(), "Equals");
    }

    #[test]
    fn test_not_equals_name() {
        assert_eq!(ConditionType::NotEquals.name(), "Not Equals");
    }

    #[test]
    fn test_true_name() {
        assert_eq!(ConditionType::True.name(), "True");
    }

    #[test]
    fn test_false_name() {
        assert_eq!(ConditionType::False.name(), "False");
    }

    // Test symbol()
    #[test]
    fn test_greater_symbol() {
        assert_eq!(ConditionType::Greater.symbol(), ">");
    }

    #[test]
    fn test_less_symbol() {
        assert_eq!(ConditionType::Less.symbol(), "<");
    }

    #[test]
    fn test_equals_symbol() {
        assert_eq!(ConditionType::Equals.symbol(), "=");
    }

    #[test]
    fn test_not_equals_symbol() {
        assert_eq!(ConditionType::NotEquals.symbol(), "≠");
    }

    // Test is_comparison()
    #[test]
    fn test_greater_is_comparison() {
        assert!(ConditionType::Greater.is_comparison());
    }

    #[test]
    fn test_less_is_comparison() {
        assert!(ConditionType::Less.is_comparison());
    }

    #[test]
    fn test_equals_is_comparison() {
        assert!(ConditionType::Equals.is_comparison());
    }

    #[test]
    fn test_not_equals_is_comparison() {
        assert!(ConditionType::NotEquals.is_comparison());
    }

    #[test]
    fn test_true_is_not_comparison() {
        assert!(!ConditionType::True.is_comparison());
    }

    #[test]
    fn test_false_is_not_comparison() {
        assert!(!ConditionType::False.is_comparison());
    }

    // Test is_boolean()
    #[test]
    fn test_true_is_boolean() {
        assert!(ConditionType::True.is_boolean());
    }

    #[test]
    fn test_false_is_boolean() {
        assert!(ConditionType::False.is_boolean());
    }

    #[test]
    fn test_greater_is_not_boolean() {
        assert!(!ConditionType::Greater.is_boolean());
    }

    #[test]
    fn test_less_is_not_boolean() {
        assert!(!ConditionType::Less.is_boolean());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_symbol() {
        let display = format!("{}", ConditionType::Greater);
        assert!(display.contains(">"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", ConditionType::Equals);
        assert!(display.contains("Equals"));
    }

    // Test default
    #[test]
    fn test_default_is_greater() {
        assert_eq!(ConditionType::default(), ConditionType::Greater);
    }
}

// ============================================================================
// PARAMETER TYPE TESTS
// ============================================================================

mod parameter_type_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(ParameterType::all().len(), 4);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = ParameterType::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = ParameterType::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_float_name() {
        assert_eq!(ParameterType::Float.name(), "Float");
    }

    #[test]
    fn test_int_name() {
        assert_eq!(ParameterType::Int.name(), "Int");
    }

    #[test]
    fn test_bool_name() {
        assert_eq!(ParameterType::Bool.name(), "Bool");
    }

    #[test]
    fn test_trigger_name() {
        assert_eq!(ParameterType::Trigger.name(), "Trigger");
    }

    // Test is_numeric()
    #[test]
    fn test_float_is_numeric() {
        assert!(ParameterType::Float.is_numeric());
    }

    #[test]
    fn test_int_is_numeric() {
        assert!(ParameterType::Int.is_numeric());
    }

    #[test]
    fn test_bool_is_not_numeric() {
        assert!(!ParameterType::Bool.is_numeric());
    }

    #[test]
    fn test_trigger_is_not_numeric() {
        assert!(!ParameterType::Trigger.is_numeric());
    }

    // Test is_instant()
    #[test]
    fn test_trigger_is_instant() {
        assert!(ParameterType::Trigger.is_instant());
    }

    #[test]
    fn test_float_is_not_instant() {
        assert!(!ParameterType::Float.is_instant());
    }

    #[test]
    fn test_int_is_not_instant() {
        assert!(!ParameterType::Int.is_instant());
    }

    #[test]
    fn test_bool_is_not_instant() {
        assert!(!ParameterType::Bool.is_instant());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", ParameterType::Float);
        assert!(display.contains("🔢"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", ParameterType::Bool);
        assert!(display.contains("Bool"));
    }

    // Test default
    #[test]
    fn test_default_is_float() {
        assert_eq!(ParameterType::default(), ParameterType::Float);
    }
}

// ============================================================================
// PARAMETER VALUE TESTS
// ============================================================================

mod parameter_value_tests {
    use super::*;

    #[test]
    fn test_all_variants_count() {
        assert_eq!(ParameterValue::all_variants().len(), 4);
    }

    // Test is_numeric()
    #[test]
    fn test_float_value_is_numeric() {
        assert!(ParameterValue::Float(1.0).is_numeric());
    }

    #[test]
    fn test_int_value_is_numeric() {
        assert!(ParameterValue::Int(1).is_numeric());
    }

    #[test]
    fn test_bool_value_is_not_numeric() {
        assert!(!ParameterValue::Bool(true).is_numeric());
    }

    #[test]
    fn test_trigger_value_is_not_numeric() {
        assert!(!ParameterValue::Trigger.is_numeric());
    }

    // Test is_bool()
    #[test]
    fn test_bool_true_is_bool() {
        assert!(ParameterValue::Bool(true).is_bool());
    }

    #[test]
    fn test_bool_false_is_bool() {
        assert!(ParameterValue::Bool(false).is_bool());
    }

    #[test]
    fn test_float_value_is_not_bool() {
        assert!(!ParameterValue::Float(0.0).is_bool());
    }

    #[test]
    fn test_int_value_is_not_bool() {
        assert!(!ParameterValue::Int(0).is_bool());
    }

    #[test]
    fn test_trigger_value_is_not_bool() {
        assert!(!ParameterValue::Trigger.is_bool());
    }

    // Test is_trigger()
    #[test]
    fn test_trigger_is_trigger() {
        assert!(ParameterValue::Trigger.is_trigger());
    }

    #[test]
    fn test_float_is_not_trigger() {
        assert!(!ParameterValue::Float(0.0).is_trigger());
    }

    #[test]
    fn test_int_is_not_trigger() {
        assert!(!ParameterValue::Int(0).is_trigger());
    }

    #[test]
    fn test_bool_is_not_trigger() {
        assert!(!ParameterValue::Bool(false).is_trigger());
    }

    // Test as_float()
    #[test]
    fn test_float_as_float_some() {
        assert_eq!(ParameterValue::Float(3.14).as_float(), Some(3.14));
    }

    #[test]
    fn test_int_as_float_none() {
        assert_eq!(ParameterValue::Int(42).as_float(), None);
    }

    #[test]
    fn test_bool_as_float_none() {
        assert_eq!(ParameterValue::Bool(true).as_float(), None);
    }

    #[test]
    fn test_trigger_as_float_none() {
        assert_eq!(ParameterValue::Trigger.as_float(), None);
    }

    // Test as_int()
    #[test]
    fn test_int_as_int_some() {
        assert_eq!(ParameterValue::Int(42).as_int(), Some(42));
    }

    #[test]
    fn test_float_as_int_none() {
        assert_eq!(ParameterValue::Float(3.14).as_int(), None);
    }

    #[test]
    fn test_bool_as_int_none() {
        assert_eq!(ParameterValue::Bool(true).as_int(), None);
    }

    #[test]
    fn test_trigger_as_int_none() {
        assert_eq!(ParameterValue::Trigger.as_int(), None);
    }

    // Test as_bool()
    #[test]
    fn test_bool_true_as_bool_some() {
        assert_eq!(ParameterValue::Bool(true).as_bool(), Some(true));
    }

    #[test]
    fn test_bool_false_as_bool_some() {
        assert_eq!(ParameterValue::Bool(false).as_bool(), Some(false));
    }

    #[test]
    fn test_float_as_bool_none() {
        assert_eq!(ParameterValue::Float(1.0).as_bool(), None);
    }

    #[test]
    fn test_int_as_bool_none() {
        assert_eq!(ParameterValue::Int(1).as_bool(), None);
    }

    #[test]
    fn test_trigger_as_bool_none() {
        assert_eq!(ParameterValue::Trigger.as_bool(), None);
    }

    // Test name()
    #[test]
    fn test_float_value_name() {
        assert_eq!(ParameterValue::Float(1.0).name(), "Float");
    }

    #[test]
    fn test_int_value_name() {
        assert_eq!(ParameterValue::Int(1).name(), "Int");
    }

    #[test]
    fn test_bool_value_name() {
        assert_eq!(ParameterValue::Bool(true).name(), "Bool");
    }

    #[test]
    fn test_trigger_value_name() {
        assert_eq!(ParameterValue::Trigger.name(), "Trigger");
    }

    // Test default
    #[test]
    fn test_default_is_float_zero() {
        assert_eq!(ParameterValue::default(), ParameterValue::Float(0.0));
    }
}

// ============================================================================
// BLEND TREE TYPE TESTS
// ============================================================================

mod blend_tree_type_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(BlendTreeType::all().len(), 5);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = BlendTreeType::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = BlendTreeType::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_simple_1d_name() {
        assert_eq!(BlendTreeType::Simple1D.name(), "Simple 1D");
    }

    #[test]
    fn test_simple_2d_name() {
        assert_eq!(BlendTreeType::Simple2D.name(), "Simple 2D");
    }

    #[test]
    fn test_freeform_directional_name() {
        assert_eq!(
            BlendTreeType::FreeformDirectional.name(),
            "Freeform Directional"
        );
    }

    #[test]
    fn test_freeform_cartesian_name() {
        assert_eq!(
            BlendTreeType::FreeformCartesian.name(),
            "Freeform Cartesian"
        );
    }

    #[test]
    fn test_direct_name() {
        assert_eq!(BlendTreeType::Direct.name(), "Direct");
    }

    // Test dimensions()
    #[test]
    fn test_simple_1d_dimensions() {
        assert_eq!(BlendTreeType::Simple1D.dimensions(), 1);
    }

    #[test]
    fn test_direct_dimensions() {
        assert_eq!(BlendTreeType::Direct.dimensions(), 1);
    }

    #[test]
    fn test_simple_2d_dimensions() {
        assert_eq!(BlendTreeType::Simple2D.dimensions(), 2);
    }

    #[test]
    fn test_freeform_directional_dimensions() {
        assert_eq!(BlendTreeType::FreeformDirectional.dimensions(), 2);
    }

    #[test]
    fn test_freeform_cartesian_dimensions() {
        assert_eq!(BlendTreeType::FreeformCartesian.dimensions(), 2);
    }

    // Test is_freeform()
    #[test]
    fn test_freeform_directional_is_freeform() {
        assert!(BlendTreeType::FreeformDirectional.is_freeform());
    }

    #[test]
    fn test_freeform_cartesian_is_freeform() {
        assert!(BlendTreeType::FreeformCartesian.is_freeform());
    }

    #[test]
    fn test_simple_1d_is_not_freeform() {
        assert!(!BlendTreeType::Simple1D.is_freeform());
    }

    #[test]
    fn test_simple_2d_is_not_freeform() {
        assert!(!BlendTreeType::Simple2D.is_freeform());
    }

    #[test]
    fn test_direct_is_not_freeform() {
        assert!(!BlendTreeType::Direct.is_freeform());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", BlendTreeType::Simple1D);
        assert!(display.contains("↔️"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", BlendTreeType::Simple2D);
        assert!(display.contains("Simple 2D"));
    }

    // Test default
    #[test]
    fn test_default_is_simple_1d() {
        assert_eq!(BlendTreeType::default(), BlendTreeType::Simple1D);
    }
}

// ============================================================================
// BLENDING MODE TESTS
// ============================================================================

mod blending_mode_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(BlendingMode::all().len(), 2);
    }

    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = BlendingMode::all().iter().map(|c| c.name()).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = BlendingMode::all().iter().map(|c| c.icon()).collect();
        let unique: HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_override_name() {
        assert_eq!(BlendingMode::Override.name(), "Override");
    }

    #[test]
    fn test_additive_name() {
        assert_eq!(BlendingMode::Additive.name(), "Additive");
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", BlendingMode::Override);
        assert!(display.contains("[Sync]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", BlendingMode::Additive);
        assert!(display.contains("Additive"));
    }

    // Test default
    #[test]
    fn test_default_is_override() {
        assert_eq!(BlendingMode::default(), BlendingMode::Override);
    }
}
