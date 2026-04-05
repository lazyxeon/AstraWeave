//! Mutation-resistant tests for aw_editor systems.
//!
//! These tests are designed to catch common mutations in:
//! - Editor mode transitions
//! - Layout preset handling
//! - Gizmo state management
//! - Entity material system
//! - Panel category logic
//! - Undo stack operations

use crate::dock_layout::LayoutPreset;
use crate::editor_mode::EditorMode;
use crate::entity_manager::{EntityMaterial, MaterialSlot};
use crate::gizmo::state::{AxisConstraint, GizmoMode};
use crate::panel_type::PanelCategory;
use glam::Vec3;
use std::path::PathBuf;

// ============================================================================
// Editor Mode Tests
// ============================================================================

mod editor_mode_tests {
    use super::*;

    #[test]
    fn test_default_mode_is_edit() {
        let mode = EditorMode::default();
        assert!(
            matches!(mode, EditorMode::Edit),
            "Default mode should be Edit"
        );
    }

    #[test]
    fn test_is_playing_returns_true_for_play() {
        let mode = EditorMode::Play;
        assert!(
            mode.is_playing(),
            "is_playing should return true for Play mode"
        );
    }

    #[test]
    fn test_is_playing_returns_false_for_edit() {
        let mode = EditorMode::Edit;
        assert!(
            !mode.is_playing(),
            "is_playing should return false for Edit mode"
        );
    }

    #[test]
    fn test_is_paused_returns_true_for_paused() {
        let mode = EditorMode::Paused;
        assert!(
            mode.is_paused(),
            "is_paused should return true for Paused mode"
        );
    }

    #[test]
    fn test_can_edit_returns_true_for_edit() {
        let mode = EditorMode::Edit;
        assert!(mode.can_edit(), "can_edit should return true for Edit mode");
    }

    #[test]
    fn test_can_edit_returns_false_for_play() {
        let mode = EditorMode::Play;
        assert!(
            !mode.can_edit(),
            "can_edit should return false for Play mode"
        );
    }

    #[test]
    fn test_valid_transition_edit_to_play() {
        let mode = EditorMode::Edit;
        assert!(
            mode.can_transition_to(EditorMode::Play),
            "Edit should transition to Play"
        );
    }

    #[test]
    fn test_invalid_transition_edit_to_paused() {
        let mode = EditorMode::Edit;
        assert!(
            !mode.can_transition_to(EditorMode::Paused),
            "Edit should NOT transition to Paused"
        );
    }

    #[test]
    fn test_valid_transitions_from_play() {
        let mode = EditorMode::Play;
        let transitions = mode.valid_transitions();
        assert!(
            transitions.contains(&EditorMode::Edit),
            "Play should transition to Edit"
        );
        assert!(
            transitions.contains(&EditorMode::Paused),
            "Play should transition to Paused"
        );
    }
}

// ============================================================================
// Layout Preset Tests
// ============================================================================

mod layout_preset_tests {
    use super::*;

    #[test]
    fn test_default_preset_is_default() {
        let preset = LayoutPreset::default();
        assert!(
            matches!(preset, LayoutPreset::Default),
            "Default preset should be Default"
        );
    }

    #[test]
    fn test_is_debug_layout_true_for_debug() {
        let preset = LayoutPreset::Debug;
        assert!(
            preset.is_debug_layout(),
            "is_debug_layout should return true for Debug"
        );
    }

    #[test]
    fn test_is_debug_layout_false_for_default() {
        let preset = LayoutPreset::Default;
        assert!(
            !preset.is_debug_layout(),
            "is_debug_layout should return false for Default"
        );
    }

    #[test]
    fn test_is_content_creation_true_for_modeling() {
        let preset = LayoutPreset::Modeling;
        assert!(
            preset.is_content_creation_layout(),
            "is_content_creation_layout should be true for Modeling"
        );
    }

    #[test]
    fn test_is_content_creation_true_for_animation() {
        let preset = LayoutPreset::Animation;
        assert!(
            preset.is_content_creation_layout(),
            "is_content_creation_layout should be true for Animation"
        );
    }

    #[test]
    fn test_is_content_creation_false_for_debug() {
        let preset = LayoutPreset::Debug;
        assert!(
            !preset.is_content_creation_layout(),
            "is_content_creation_layout should be false for Debug"
        );
    }

    #[test]
    fn test_expected_panel_count_varies_by_preset() {
        let default_count = LayoutPreset::Default.expected_panel_count();
        let wide_count = LayoutPreset::Wide.expected_panel_count();
        assert_ne!(
            default_count, wide_count,
            "Different presets should have different panel counts"
        );
    }
}

// ============================================================================
// Gizmo Mode Tests
// ============================================================================

mod gizmo_mode_tests {
    use super::*;

    #[test]
    fn test_default_mode_is_inactive() {
        let mode = GizmoMode::default();
        assert!(
            matches!(mode, GizmoMode::Inactive),
            "Default gizmo mode should be Inactive"
        );
    }

    #[test]
    fn test_is_active_false_for_inactive() {
        let mode = GizmoMode::Inactive;
        assert!(
            !mode.is_active(),
            "is_active should return false for Inactive"
        );
    }

    #[test]
    fn test_is_active_true_for_translate() {
        let mode = GizmoMode::Translate {
            constraint: AxisConstraint::None,
        };
        assert!(
            mode.is_active(),
            "is_active should return true for Translate"
        );
    }

    #[test]
    fn test_is_translate_true_for_translate() {
        let mode = GizmoMode::Translate {
            constraint: AxisConstraint::X,
        };
        assert!(
            mode.is_translate(),
            "is_translate should return true for Translate"
        );
    }

    #[test]
    fn test_is_translate_false_for_rotate() {
        let mode = GizmoMode::Rotate {
            constraint: AxisConstraint::X,
        };
        assert!(
            !mode.is_translate(),
            "is_translate should return false for Rotate"
        );
    }

    #[test]
    fn test_is_rotate_true_for_rotate() {
        let mode = GizmoMode::Rotate {
            constraint: AxisConstraint::Y,
        };
        assert!(mode.is_rotate(), "is_rotate should return true for Rotate");
    }

    #[test]
    fn test_is_scale_true_for_scale() {
        let mode = GizmoMode::Scale {
            constraint: AxisConstraint::Z,
            uniform: false,
        };
        assert!(mode.is_scale(), "is_scale should return true for Scale");
    }

    #[test]
    fn test_constraint_returns_some_for_active_modes() {
        let mode = GizmoMode::Translate {
            constraint: AxisConstraint::XY,
        };
        assert_eq!(
            mode.constraint(),
            Some(AxisConstraint::XY),
            "constraint should return Some for Translate"
        );
    }

    #[test]
    fn test_constraint_returns_none_for_inactive() {
        let mode = GizmoMode::Inactive;
        assert_eq!(
            mode.constraint(),
            None,
            "constraint should return None for Inactive"
        );
    }
}

// ============================================================================
// Axis Constraint Tests
// ============================================================================

mod axis_constraint_tests {
    use super::*;

    #[test]
    fn test_default_constraint_is_none() {
        let constraint = AxisConstraint::default();
        assert!(
            matches!(constraint, AxisConstraint::None),
            "Default constraint should be None"
        );
    }

    #[test]
    fn test_single_axis_constraints_are_distinct() {
        let x = AxisConstraint::X;
        let y = AxisConstraint::Y;
        let z = AxisConstraint::Z;
        assert_ne!(x, y, "X != Y");
        assert_ne!(y, z, "Y != Z");
        assert_ne!(x, z, "X != Z");
    }

    #[test]
    fn test_planar_constraints_are_distinct() {
        let xy = AxisConstraint::XY;
        let xz = AxisConstraint::XZ;
        let yz = AxisConstraint::YZ;
        assert_ne!(xy, xz, "XY != XZ");
        assert_ne!(xz, yz, "XZ != YZ");
        assert_ne!(xy, yz, "XY != YZ");
    }
}

// ============================================================================
// Entity Material Tests
// ============================================================================

mod entity_material_tests {
    use super::*;

    #[test]
    fn test_default_material_properties() {
        let mat = EntityMaterial::default();
        assert_eq!(mat.metallic, 0.0, "Default metallic should be 0.0");
        assert_eq!(mat.roughness, 0.5, "Default roughness should be 0.5");
        assert!(
            mat.emissive.length_squared() < 0.001,
            "Default should not be emissive"
        );
    }

    #[test]
    fn test_has_textures_false_for_new() {
        let mat = EntityMaterial::new();
        assert!(!mat.has_textures(), "New material should have no textures");
    }

    #[test]
    fn test_has_textures_true_after_set() {
        let mut mat = EntityMaterial::new();
        mat.set_texture(MaterialSlot::Albedo, PathBuf::from("texture.png"));
        assert!(
            mat.has_textures(),
            "Material should have textures after set"
        );
    }

    #[test]
    fn test_texture_count_increases() {
        let mut mat = EntityMaterial::new();
        assert_eq!(mat.texture_count(), 0, "Initial count should be 0");
        mat.set_texture(MaterialSlot::Albedo, PathBuf::from("a.png"));
        assert_eq!(
            mat.texture_count(),
            1,
            "Count should be 1 after first texture"
        );
        mat.set_texture(MaterialSlot::Normal, PathBuf::from("n.png"));
        assert_eq!(
            mat.texture_count(),
            2,
            "Count should be 2 after second texture"
        );
    }

    #[test]
    fn test_clear_texture_removes() {
        let mut mat = EntityMaterial::new();
        mat.set_texture(MaterialSlot::Albedo, PathBuf::from("a.png"));
        assert!(mat.get_texture(MaterialSlot::Albedo).is_some());
        mat.clear_texture(MaterialSlot::Albedo);
        assert!(
            mat.get_texture(MaterialSlot::Albedo).is_none(),
            "Texture should be removed"
        );
    }

    #[test]
    fn test_is_metallic_threshold() {
        let mut mat = EntityMaterial::new();
        mat.metallic = 0.5;
        assert!(
            !mat.is_metallic(),
            "0.5 should NOT be metallic (> 0.5 required)"
        );
        mat.metallic = 0.51;
        assert!(mat.is_metallic(), "0.51 should be metallic");
    }

    #[test]
    fn test_is_rough_threshold() {
        let mut mat = EntityMaterial::new();
        mat.roughness = 0.5;
        assert!(!mat.is_rough(), "0.5 should NOT be rough (> 0.5 required)");
        mat.roughness = 0.51;
        assert!(mat.is_rough(), "0.51 should be rough");
    }

    #[test]
    fn test_is_emissive_threshold() {
        let mut mat = EntityMaterial::new();
        mat.emissive = Vec3::ZERO;
        assert!(!mat.is_emissive(), "Zero emission should not be emissive");
        mat.emissive = Vec3::new(0.1, 0.0, 0.0);
        assert!(mat.is_emissive(), "Non-zero emission should be emissive");
    }
}

// ============================================================================
// MUTATION-RESISTANT TEST MODULE 1: Boundary Condition Tests
// ============================================================================

mod boundary_condition_tests {
    use super::*;

    // --- Layout Preset Panel Count Boundaries ---

    #[test]
    fn test_wide_layout_minimum_panels() {
        let preset = LayoutPreset::Wide;
        assert!(
            preset.expected_panel_count() >= 2,
            "Wide should have at least 2 panels"
        );
    }

    #[test]
    fn test_compact_layout_maximum_panels() {
        let preset = LayoutPreset::Compact;
        assert!(
            preset.expected_panel_count() >= 6,
            "Compact should have many panels"
        );
    }

    // --- Metallic/Roughness Boundaries ---

    #[test]
    fn test_metallic_exactly_half() {
        let mut mat = EntityMaterial::new();
        mat.metallic = 0.5;
        assert!(
            !mat.is_metallic(),
            "Exactly 0.5 should NOT be metallic (boundary)"
        );
    }

    #[test]
    fn test_metallic_just_above_half() {
        let mut mat = EntityMaterial::new();
        mat.metallic = 0.500001;
        assert!(mat.is_metallic(), "Just above 0.5 should be metallic");
    }

    #[test]
    fn test_metallic_just_below_half() {
        let mut mat = EntityMaterial::new();
        mat.metallic = 0.499999;
        assert!(!mat.is_metallic(), "Just below 0.5 should NOT be metallic");
    }

    #[test]
    fn test_roughness_exactly_half() {
        let mut mat = EntityMaterial::new();
        mat.roughness = 0.5;
        assert!(
            !mat.is_rough(),
            "Exactly 0.5 should NOT be rough (boundary)"
        );
    }

    #[test]
    fn test_roughness_just_above_half() {
        let mut mat = EntityMaterial::new();
        mat.roughness = 0.500001;
        assert!(mat.is_rough(), "Just above 0.5 should be rough");
    }

    // --- Emissive Threshold Boundaries ---

    #[test]
    fn test_emissive_zero() {
        let mut mat = EntityMaterial::new();
        mat.emissive = Vec3::ZERO;
        assert!(!mat.is_emissive(), "Zero emission should not be emissive");
    }

    #[test]
    fn test_emissive_just_above_threshold() {
        let mut mat = EntityMaterial::new();
        // length_squared needs to be > 0.001
        // sqrt(0.001) ≈ 0.0316
        mat.emissive = Vec3::new(0.033, 0.0, 0.0);
        assert!(mat.is_emissive(), "Above threshold should be emissive");
    }

    #[test]
    fn test_emissive_exactly_at_threshold() {
        let mut mat = EntityMaterial::new();
        // length_squared = 0.001 exactly
        // sqrt(0.001) ≈ 0.031622...
        mat.emissive = Vec3::new(0.031622, 0.0, 0.0);
        // 0.031622^2 ≈ 0.000999, which is < 0.001, so NOT emissive
        assert!(
            !mat.is_emissive(),
            "At threshold should not be emissive (exclusive)"
        );
    }

    // --- Normal Strength Boundaries ---

    #[test]
    fn test_normal_strength_default() {
        let mat = EntityMaterial::default();
        assert!(
            (mat.normal_strength - 1.0).abs() < f32::EPSILON,
            "Default normal strength should be 1.0"
        );
    }

    #[test]
    fn test_normal_strength_zero() {
        let mut mat = EntityMaterial::new();
        mat.normal_strength = 0.0;
        assert!(
            (mat.normal_strength).abs() < f32::EPSILON,
            "Zero normal strength"
        );
    }

    // --- Texture Count Boundaries ---

    #[test]
    fn test_texture_count_zero_initially() {
        let mat = EntityMaterial::new();
        assert_eq!(mat.texture_count(), 0, "Initial texture count should be 0");
    }

    #[test]
    fn test_texture_count_one_after_single_add() {
        let mut mat = EntityMaterial::new();
        mat.set_texture(MaterialSlot::Albedo, PathBuf::from("a.png"));
        assert_eq!(mat.texture_count(), 1, "One texture should give count 1");
    }

    #[test]
    fn test_texture_count_all_slots_filled() {
        let mut mat = EntityMaterial::new();
        for (i, slot) in MaterialSlot::all().iter().enumerate() {
            mat.set_texture(*slot, PathBuf::from(format!("{}.png", i)));
        }
        assert_eq!(
            mat.texture_count(),
            MaterialSlot::all().len(),
            "All slots should be filled"
        );
    }

    // --- Valid Transitions Count Boundaries ---

    #[test]
    fn test_edit_mode_has_one_transition() {
        let mode = EditorMode::Edit;
        assert_eq!(
            mode.valid_transitions().len(),
            1,
            "Edit should have exactly 1 valid transition"
        );
    }

    #[test]
    fn test_play_mode_has_two_transitions() {
        let mode = EditorMode::Play;
        assert_eq!(
            mode.valid_transitions().len(),
            2,
            "Play should have exactly 2 valid transitions"
        );
    }

    #[test]
    fn test_paused_mode_has_two_transitions() {
        let mode = EditorMode::Paused;
        assert_eq!(
            mode.valid_transitions().len(),
            2,
            "Paused should have exactly 2 valid transitions"
        );
    }
}

// ============================================================================
// MUTATION-RESISTANT TEST MODULE 2: Comparison Operator Tests
// ============================================================================

mod comparison_operator_tests {
    use super::*;

    // --- EditorMode Equality ---

    #[test]
    fn test_editor_mode_edit_equals_edit() {
        assert_eq!(EditorMode::Edit, EditorMode::Edit);
    }

    #[test]
    fn test_editor_mode_edit_not_equals_play() {
        assert_ne!(EditorMode::Edit, EditorMode::Play);
    }

    #[test]
    fn test_editor_mode_play_not_equals_paused() {
        assert_ne!(EditorMode::Play, EditorMode::Paused);
    }

    // --- LayoutPreset Equality ---

    #[test]
    fn test_layout_preset_equality() {
        assert_eq!(LayoutPreset::Default, LayoutPreset::Default);
        assert_ne!(LayoutPreset::Default, LayoutPreset::Wide);
        assert_ne!(LayoutPreset::Wide, LayoutPreset::Compact);
    }

    #[test]
    fn test_layout_preset_all_variants_distinct() {
        let presets = LayoutPreset::all();
        for i in 0..presets.len() {
            for j in 0..presets.len() {
                if i != j {
                    assert_ne!(
                        presets[i], presets[j],
                        "Different presets should be distinct"
                    );
                }
            }
        }
    }

    // --- GizmoMode Equality ---

    #[test]
    fn test_gizmo_mode_inactive_equals_inactive() {
        assert_eq!(GizmoMode::Inactive, GizmoMode::Inactive);
    }

    #[test]
    fn test_gizmo_mode_translate_not_equals_rotate() {
        let translate = GizmoMode::Translate {
            constraint: AxisConstraint::X,
        };
        let rotate = GizmoMode::Rotate {
            constraint: AxisConstraint::X,
        };
        assert_ne!(
            translate, rotate,
            "Translate and Rotate should be different"
        );
    }

    #[test]
    fn test_gizmo_mode_same_type_different_constraint() {
        let tx = GizmoMode::Translate {
            constraint: AxisConstraint::X,
        };
        let ty = GizmoMode::Translate {
            constraint: AxisConstraint::Y,
        };
        assert_ne!(tx, ty, "Same mode with different constraints should differ");
    }

    // --- AxisConstraint Equality ---

    #[test]
    fn test_axis_constraint_equality() {
        assert_eq!(AxisConstraint::None, AxisConstraint::None);
        assert_eq!(AxisConstraint::X, AxisConstraint::X);
        assert_ne!(AxisConstraint::X, AxisConstraint::Y);
    }

    #[test]
    fn test_axis_constraint_single_vs_planar() {
        assert_ne!(
            AxisConstraint::X,
            AxisConstraint::XY,
            "Single axis != planar"
        );
        assert_ne!(
            AxisConstraint::Y,
            AxisConstraint::XY,
            "Single axis != planar"
        );
    }

    // --- MaterialSlot Equality ---

    #[test]
    fn test_material_slot_equality() {
        assert_eq!(MaterialSlot::Albedo, MaterialSlot::Albedo);
        assert_ne!(MaterialSlot::Albedo, MaterialSlot::Normal);
        assert_ne!(MaterialSlot::Roughness, MaterialSlot::Metallic);
    }

    #[test]
    fn test_material_slot_all_distinct() {
        let slots = MaterialSlot::all();
        for i in 0..slots.len() {
            for j in 0..slots.len() {
                if i != j {
                    assert_ne!(slots[i], slots[j], "Different slots should be distinct");
                }
            }
        }
    }

    // --- PanelCategory Equality ---

    #[test]
    fn test_panel_category_equality() {
        assert_eq!(PanelCategory::Scene, PanelCategory::Scene);
        assert_ne!(PanelCategory::Scene, PanelCategory::Assets);
        assert_ne!(PanelCategory::Debug, PanelCategory::Tools);
    }

    // --- Transition Comparison ---

    #[test]
    fn test_can_transition_to_same_mode() {
        assert!(
            EditorMode::Edit.can_transition_to(EditorMode::Edit),
            "Same mode transition should be valid"
        );
        assert!(
            EditorMode::Play.can_transition_to(EditorMode::Play),
            "Same mode transition should be valid"
        );
    }

    #[test]
    fn test_asymmetric_transitions() {
        // Edit -> Paused is invalid, but Paused -> Edit is valid
        assert!(
            !EditorMode::Edit.can_transition_to(EditorMode::Paused),
            "Edit -> Paused invalid"
        );
        assert!(
            EditorMode::Paused.can_transition_to(EditorMode::Edit),
            "Paused -> Edit valid"
        );
    }
}

// ============================================================================
// MUTATION-RESISTANT TEST MODULE 3: Boolean Return Path Tests
// ============================================================================

mod boolean_return_path_tests {
    use super::*;

    // --- EditorMode Boolean Methods ---

    #[test]
    fn test_is_playing_true_path() {
        let result = EditorMode::Play.is_playing();
        assert!(result, "is_playing should be true for Play");
        assert_eq!(result, true, "Result should be exactly true");
    }

    #[test]
    fn test_is_playing_false_path() {
        let result = EditorMode::Edit.is_playing();
        assert!(!result, "is_playing should be false for Edit");
        assert_eq!(result, false, "Result should be exactly false");
    }

    #[test]
    fn test_is_paused_true_path() {
        let result = EditorMode::Paused.is_paused();
        assert!(result, "is_paused should be true for Paused");
        assert_eq!(result, true);
    }

    #[test]
    fn test_is_paused_false_path() {
        let result = EditorMode::Play.is_paused();
        assert!(!result, "is_paused should be false for Play");
        assert_eq!(result, false);
    }

    #[test]
    fn test_is_editing_true_path() {
        let result = EditorMode::Edit.is_editing();
        assert!(result, "is_editing should be true for Edit");
        assert_eq!(result, true);
    }

    #[test]
    fn test_is_editing_false_path() {
        let result = EditorMode::Play.is_editing();
        assert!(!result, "is_editing should be false for Play");
        assert_eq!(result, false);
    }

    #[test]
    fn test_can_edit_true_path() {
        let result = EditorMode::Edit.can_edit();
        assert!(result);
        assert_eq!(result, true);
    }

    #[test]
    fn test_can_edit_false_paths() {
        assert_eq!(EditorMode::Play.can_edit(), false);
        assert_eq!(EditorMode::Paused.can_edit(), false);
    }

    // --- GizmoMode Boolean Methods ---

    #[test]
    fn test_gizmo_is_active_true_paths() {
        assert!(GizmoMode::Translate {
            constraint: AxisConstraint::None
        }
        .is_active());
        assert!(GizmoMode::Rotate {
            constraint: AxisConstraint::None
        }
        .is_active());
        assert!(GizmoMode::Scale {
            constraint: AxisConstraint::None,
            uniform: false
        }
        .is_active());
    }

    #[test]
    fn test_gizmo_is_active_false_path() {
        assert!(!GizmoMode::Inactive.is_active());
        assert_eq!(GizmoMode::Inactive.is_active(), false);
    }

    #[test]
    fn test_gizmo_is_translate_true_path() {
        let mode = GizmoMode::Translate {
            constraint: AxisConstraint::X,
        };
        assert!(mode.is_translate());
        assert_eq!(mode.is_translate(), true);
    }

    #[test]
    fn test_gizmo_is_translate_false_paths() {
        assert_eq!(GizmoMode::Inactive.is_translate(), false);
        assert_eq!(
            GizmoMode::Rotate {
                constraint: AxisConstraint::X
            }
            .is_translate(),
            false
        );
        assert_eq!(
            GizmoMode::Scale {
                constraint: AxisConstraint::X,
                uniform: false
            }
            .is_translate(),
            false
        );
    }

    #[test]
    fn test_gizmo_is_rotate_true_path() {
        let mode = GizmoMode::Rotate {
            constraint: AxisConstraint::Y,
        };
        assert!(mode.is_rotate());
        assert_eq!(mode.is_rotate(), true);
    }

    #[test]
    fn test_gizmo_is_rotate_false_paths() {
        assert_eq!(GizmoMode::Inactive.is_rotate(), false);
        assert_eq!(
            GizmoMode::Translate {
                constraint: AxisConstraint::Y
            }
            .is_rotate(),
            false
        );
    }

    #[test]
    fn test_gizmo_is_scale_true_path() {
        let mode = GizmoMode::Scale {
            constraint: AxisConstraint::Z,
            uniform: true,
        };
        assert!(mode.is_scale());
        assert_eq!(mode.is_scale(), true);
    }

    #[test]
    fn test_gizmo_is_scale_false_paths() {
        assert_eq!(GizmoMode::Inactive.is_scale(), false);
        assert_eq!(
            GizmoMode::Translate {
                constraint: AxisConstraint::Z
            }
            .is_scale(),
            false
        );
    }

    // --- LayoutPreset Boolean Methods ---

    #[test]
    fn test_is_debug_layout_true_path() {
        let result = LayoutPreset::Debug.is_debug_layout();
        assert!(result);
        assert_eq!(result, true);
    }

    #[test]
    fn test_is_debug_layout_false_paths() {
        assert_eq!(LayoutPreset::Default.is_debug_layout(), false);
        assert_eq!(LayoutPreset::Wide.is_debug_layout(), false);
        assert_eq!(LayoutPreset::Compact.is_debug_layout(), false);
        assert_eq!(LayoutPreset::Modeling.is_debug_layout(), false);
        assert_eq!(LayoutPreset::Animation.is_debug_layout(), false);
    }

    #[test]
    fn test_is_content_creation_true_paths() {
        assert_eq!(LayoutPreset::Modeling.is_content_creation_layout(), true);
        assert_eq!(LayoutPreset::Animation.is_content_creation_layout(), true);
    }

    #[test]
    fn test_is_content_creation_false_paths() {
        assert_eq!(LayoutPreset::Default.is_content_creation_layout(), false);
        assert_eq!(LayoutPreset::Wide.is_content_creation_layout(), false);
        assert_eq!(LayoutPreset::Debug.is_content_creation_layout(), false);
    }

    // --- EntityMaterial Boolean Methods ---

    #[test]
    fn test_has_textures_true_path() {
        let mut mat = EntityMaterial::new();
        mat.set_texture(MaterialSlot::Albedo, PathBuf::from("a.png"));
        assert!(mat.has_textures());
        assert_eq!(mat.has_textures(), true);
    }

    #[test]
    fn test_has_textures_false_path() {
        let mat = EntityMaterial::new();
        assert!(!mat.has_textures());
        assert_eq!(mat.has_textures(), false);
    }

    #[test]
    fn test_is_metallic_true_path() {
        let mut mat = EntityMaterial::new();
        mat.metallic = 0.8;
        assert!(mat.is_metallic());
        assert_eq!(mat.is_metallic(), true);
    }

    #[test]
    fn test_is_metallic_false_path() {
        let mut mat = EntityMaterial::new();
        mat.metallic = 0.3;
        assert!(!mat.is_metallic());
        assert_eq!(mat.is_metallic(), false);
    }

    #[test]
    fn test_is_rough_true_path() {
        let mut mat = EntityMaterial::new();
        mat.roughness = 0.9;
        assert!(mat.is_rough());
        assert_eq!(mat.is_rough(), true);
    }

    #[test]
    fn test_is_rough_false_path() {
        let mut mat = EntityMaterial::new();
        mat.roughness = 0.2;
        assert!(!mat.is_rough());
        assert_eq!(mat.is_rough(), false);
    }

    #[test]
    fn test_is_emissive_true_path() {
        let mut mat = EntityMaterial::new();
        mat.emissive = Vec3::new(1.0, 0.0, 0.0);
        assert!(mat.is_emissive());
        assert_eq!(mat.is_emissive(), true);
    }

    #[test]
    fn test_is_emissive_false_path() {
        let mat = EntityMaterial::new();
        assert!(!mat.is_emissive());
        assert_eq!(mat.is_emissive(), false);
    }

    // --- Transition Boolean ---

    #[test]
    fn test_can_transition_to_true_paths() {
        assert_eq!(EditorMode::Edit.can_transition_to(EditorMode::Play), true);
        assert_eq!(EditorMode::Play.can_transition_to(EditorMode::Edit), true);
        assert_eq!(EditorMode::Play.can_transition_to(EditorMode::Paused), true);
        assert_eq!(EditorMode::Paused.can_transition_to(EditorMode::Play), true);
    }

    #[test]
    fn test_can_transition_to_false_path() {
        let result = EditorMode::Edit.can_transition_to(EditorMode::Paused);
        assert!(!result);
        assert_eq!(result, false);
    }

    // --- Option Return Paths ---

    #[test]
    fn test_constraint_some_path() {
        let mode = GizmoMode::Translate {
            constraint: AxisConstraint::XZ,
        };
        assert!(mode.constraint().is_some());
        assert_eq!(mode.constraint(), Some(AxisConstraint::XZ));
    }

    #[test]
    fn test_constraint_none_path() {
        let mode = GizmoMode::Inactive;
        assert!(mode.constraint().is_none());
        assert_eq!(mode.constraint(), None);
    }

    #[test]
    fn test_shortcut_some_path() {
        let mode = GizmoMode::Translate {
            constraint: AxisConstraint::None,
        };
        assert!(mode.shortcut().is_some());
        assert_eq!(mode.shortcut(), Some("G"));
    }

    #[test]
    fn test_shortcut_none_path() {
        let mode = GizmoMode::Inactive;
        assert!(mode.shortcut().is_none());
        assert_eq!(mode.shortcut(), None);
    }

    #[test]
    fn test_get_texture_some_path() {
        let mut mat = EntityMaterial::new();
        mat.set_texture(MaterialSlot::Normal, PathBuf::from("n.png"));
        assert!(mat.get_texture(MaterialSlot::Normal).is_some());
    }

    #[test]
    fn test_get_texture_none_path() {
        let mat = EntityMaterial::new();
        assert!(mat.get_texture(MaterialSlot::Normal).is_none());
    }

    #[test]
    fn test_shortcut_hint_some_paths() {
        assert!(LayoutPreset::Default.shortcut_hint().is_some());
        assert!(LayoutPreset::Wide.shortcut_hint().is_some());
    }
}

// ============================================================================
// Mutation-Resistant V2: Exact Value Tests
// ============================================================================

/// Tests that check EXACT return values for every match arm —
/// mutations that swap arms or replace with "xyzzy" / defaults will fail.
mod exact_value_tests {
    use super::*;
    use crate::dock_layout::{DockLayout, LayoutPreset, LayoutStats};
    use crate::editor_mode::EditorMode;

    // --- LayoutPreset::description() exact values ---
    #[test]
    fn test_layout_description_default() {
        assert_eq!(
            LayoutPreset::Default.description(),
            "Balanced layout for general editing"
        );
    }
    #[test]
    fn test_layout_description_wide() {
        assert_eq!(
            LayoutPreset::Wide.description(),
            "Maximized viewport for scene viewing"
        );
    }
    #[test]
    fn test_layout_description_compact() {
        assert_eq!(
            LayoutPreset::Compact.description(),
            "All panels visible in smaller configuration"
        );
    }
    #[test]
    fn test_layout_description_modeling() {
        assert_eq!(
            LayoutPreset::Modeling.description(),
            "Large viewport with transform tools"
        );
    }
    #[test]
    fn test_layout_description_animation() {
        assert_eq!(
            LayoutPreset::Animation.description(),
            "Timeline at bottom, graph on side"
        );
    }
    #[test]
    fn test_layout_description_debug() {
        assert_eq!(
            LayoutPreset::Debug.description(),
            "Console and profiler prominent"
        );
    }

    // --- LayoutPreset::icon() exact values ---
    #[test]
    fn test_layout_icon_default() {
        assert_eq!(LayoutPreset::Default.icon(), "\u{1f3e0}");
    }
    #[test]
    fn test_layout_icon_wide() {
        assert_eq!(LayoutPreset::Wide.icon(), "\u{1f5a5}\u{fe0f}");
    }
    #[test]
    fn test_layout_icon_compact() {
        assert_eq!(LayoutPreset::Compact.icon(), "\u{1f4d0}");
    }
    #[test]
    fn test_layout_icon_modeling() {
        assert_eq!(LayoutPreset::Modeling.icon(), "\u{1f527}");
    }
    #[test]
    fn test_layout_icon_animation() {
        assert_eq!(LayoutPreset::Animation.icon(), "\u{1f3ac}");
    }
    #[test]
    fn test_layout_icon_debug() {
        assert_eq!(LayoutPreset::Debug.icon(), "\u{1f50d}");
    }

    // --- LayoutPreset::shortcut_hint() exact values ---
    #[test]
    fn test_layout_shortcut_default() {
        assert_eq!(LayoutPreset::Default.shortcut_hint(), Some("Ctrl+1"));
    }
    #[test]
    fn test_layout_shortcut_wide() {
        assert_eq!(LayoutPreset::Wide.shortcut_hint(), Some("Ctrl+2"));
    }
    #[test]
    fn test_layout_shortcut_compact() {
        assert_eq!(LayoutPreset::Compact.shortcut_hint(), Some("Ctrl+3"));
    }
    #[test]
    fn test_layout_shortcut_modeling() {
        assert_eq!(LayoutPreset::Modeling.shortcut_hint(), Some("Ctrl+4"));
    }
    #[test]
    fn test_layout_shortcut_animation() {
        assert_eq!(LayoutPreset::Animation.shortcut_hint(), Some("Ctrl+5"));
    }
    #[test]
    fn test_layout_shortcut_debug() {
        assert_eq!(LayoutPreset::Debug.shortcut_hint(), Some("Ctrl+6"));
    }

    // --- LayoutPreset::expected_panel_count() exact values ---
    #[test]
    fn test_layout_panel_count_exact_default() {
        assert_eq!(LayoutPreset::Default.expected_panel_count(), 8);
    }
    #[test]
    fn test_layout_panel_count_exact_wide() {
        assert_eq!(LayoutPreset::Wide.expected_panel_count(), 3);
    }
    #[test]
    fn test_layout_panel_count_exact_compact() {
        assert_eq!(LayoutPreset::Compact.expected_panel_count(), 10);
    }
    #[test]
    fn test_layout_panel_count_exact_modeling() {
        assert_eq!(LayoutPreset::Modeling.expected_panel_count(), 4);
    }
    #[test]
    fn test_layout_panel_count_exact_animation() {
        assert_eq!(LayoutPreset::Animation.expected_panel_count(), 6);
    }
    #[test]
    fn test_layout_panel_count_exact_debug() {
        assert_eq!(LayoutPreset::Debug.expected_panel_count(), 6);
    }

    // --- LayoutPreset::name() exact values ---
    #[test]
    fn test_layout_name_each_variant() {
        assert_eq!(LayoutPreset::Default.name(), "Default");
        assert_eq!(LayoutPreset::Wide.name(), "Wide");
        assert_eq!(LayoutPreset::Compact.name(), "Compact");
        assert_eq!(LayoutPreset::Modeling.name(), "Modeling");
        assert_eq!(LayoutPreset::Animation.name(), "Animation");
        assert_eq!(LayoutPreset::Debug.name(), "Debug");
    }

    // --- DockLayout::dock_state_mut() must return internal ref ---
    #[test]
    fn test_dock_state_mut_returns_internal() {
        use crate::panel_type::PanelType;
        let mut layout = DockLayout::new();
        let state = layout.dock_state_mut();
        let has_viewport = state.find_tab(&PanelType::Viewport).is_some();
        assert!(
            has_viewport,
            "dock_state_mut must return the internal state containing Viewport"
        );
    }

    // --- DockLayout::style_mut() must return internal ref ---
    #[test]
    fn test_style_mut_returns_internal() {
        let mut layout = DockLayout::new();
        // Modify style through style_mut
        layout.style_mut().tab_bar.height = 999.0;
        // Read back through style() – if style_mut returned a leaked default,
        // the internal style would still have 24.0
        assert!(
            (layout.style().tab_bar.height - 999.0).abs() < f32::EPSILON,
            "style_mut must return the actual internal style"
        );
    }

    // --- DockLayout::from_preset specific panel checks ---
    #[test]
    fn test_from_preset_wide_has_inspector_only() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Wide);
        assert!(layout.is_panel_visible(&PanelType::Viewport));
        assert!(layout.is_panel_visible(&PanelType::Inspector));
        assert!(!layout.is_panel_visible(&PanelType::Console));
    }

    #[test]
    fn test_from_preset_debug_has_console_profiler() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Debug);
        assert!(layout.is_panel_visible(&PanelType::Console));
        assert!(layout.is_panel_visible(&PanelType::Profiler));
        assert!(layout.is_panel_visible(&PanelType::Performance));
    }

    #[test]
    fn test_from_preset_modeling_has_transform() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Modeling);
        assert!(layout.is_panel_visible(&PanelType::Transform));
        assert!(layout.is_panel_visible(&PanelType::Inspector));
        assert!(!layout.is_panel_visible(&PanelType::Console));
    }

    #[test]
    fn test_from_preset_animation_has_graph() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Animation);
        assert!(layout.is_panel_visible(&PanelType::BehaviorGraph));
        assert!(layout.is_panel_visible(&PanelType::Animation));
        assert!(layout.is_panel_visible(&PanelType::Graph));
    }

    #[test]
    fn test_from_preset_compact_has_many_panels() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Compact);
        assert!(layout.is_panel_visible(&PanelType::Inspector));
        assert!(layout.is_panel_visible(&PanelType::Transform));
        assert!(layout.is_panel_visible(&PanelType::EntityPanel));
        assert!(layout.is_panel_visible(&PanelType::Console));
        assert!(layout.is_panel_visible(&PanelType::Profiler));
    }

    // --- DockLayout::remove_panel returns true for removable panel ---
    #[test]
    fn test_remove_panel_returns_true() {
        use crate::panel_type::PanelType;
        let mut layout = DockLayout::from_preset(LayoutPreset::Default);
        let removed = layout.remove_panel(&PanelType::Inspector);
        assert!(
            removed,
            "remove_panel should return true for a present closable panel"
        );
        assert!(!layout.is_panel_visible(&PanelType::Inspector));
    }

    #[test]
    fn test_remove_panel_returns_false_for_unclosable() {
        use crate::panel_type::PanelType;
        let mut layout = DockLayout::from_preset(LayoutPreset::Default);
        let removed = layout.remove_panel(&PanelType::Viewport);
        assert!(!removed, "Viewport is not closable, should return false");
    }

    // --- DockLayout::has_panel ---
    #[test]
    fn test_has_panel_true() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Default);
        assert!(layout.has_panel(&PanelType::Viewport));
        assert!(layout.has_panel(&PanelType::Inspector));
    }

    #[test]
    fn test_has_panel_false() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Wide);
        assert!(!layout.has_panel(&PanelType::Console));
    }

    // --- DockLayout::toggle_panel ---
    #[test]
    fn test_toggle_panel_removes_then_adds() {
        use crate::panel_type::PanelType;
        let mut layout = DockLayout::from_preset(LayoutPreset::Default);
        assert!(layout.has_panel(&PanelType::Inspector));
        layout.toggle_panel(PanelType::Inspector);
        assert!(
            !layout.has_panel(&PanelType::Inspector),
            "toggle should remove a visible panel"
        );
        layout.toggle_panel(PanelType::Inspector);
        assert!(
            layout.has_panel(&PanelType::Inspector),
            "toggle should add an absent panel"
        );
    }

    // --- DockLayout::apply_preset ---
    #[test]
    fn test_apply_preset_changes_layout() {
        use crate::panel_type::PanelType;
        let mut layout = DockLayout::from_preset(LayoutPreset::Wide);
        assert!(!layout.has_panel(&PanelType::Console));
        layout.apply_preset(LayoutPreset::Debug);
        assert!(
            layout.has_panel(&PanelType::Console),
            "apply_preset(Debug) should add Console panel"
        );
        assert!(
            layout.has_panel(&PanelType::Profiler),
            "apply_preset(Debug) should add Profiler panel"
        );
    }

    // --- DockLayout::to_json ---
    #[test]
    fn test_to_json_returns_valid_json() {
        let layout = DockLayout::from_preset(LayoutPreset::Default);
        let json_result = layout.to_json();
        assert!(json_result.is_ok(), "to_json should succeed");
        let json = json_result.unwrap();
        assert!(
            json.contains("Viewport"),
            "JSON should contain Viewport panel"
        );
        assert!(!json.is_empty());
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok(), "to_json output must be valid JSON");
    }

    // --- DockLayout::from_json ---
    #[test]
    fn test_from_json_rejects_invalid_json() {
        // If the mutant replaces the body with Ok(Default::default()),
        // invalid JSON would succeed instead of failing
        let result = DockLayout::from_json("NOT VALID JSON {{{");
        assert!(result.is_err(), "from_json must reject invalid JSON");
    }

    #[test]
    fn test_from_json_accepts_valid_json() {
        let layout = DockLayout::from_preset(LayoutPreset::Default);
        let json = layout.to_json().unwrap();
        let result = DockLayout::from_json(&json);
        assert!(
            result.is_ok(),
            "from_json must accept valid JSON from to_json"
        );
    }

    #[test]
    fn test_from_json_rejects_empty_string() {
        let result = DockLayout::from_json("");
        assert!(result.is_err(), "from_json must reject empty string");
    }

    #[test]
    fn test_from_json_rejects_wrong_structure() {
        // Valid JSON but wrong structure (missing "panels" field)
        let result = DockLayout::from_json(r#"{"foo": "bar"}"#);
        assert!(
            result.is_err(),
            "from_json must reject wrong JSON structure"
        );
    }

    // --- DockLayout::missing_panels_for_preset ---
    #[test]
    fn test_missing_panels_for_preset_default_is_empty() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Default);
        let missing = layout.missing_panels_for_preset(LayoutPreset::Default);
        assert!(
            missing.is_empty(),
            "Default layout should have no missing panels for Default preset"
        );
    }

    #[test]
    fn test_missing_panels_for_preset_finds_missing() {
        use crate::panel_type::PanelType;
        let layout = DockLayout::from_preset(LayoutPreset::Wide);
        let missing = layout.missing_panels_for_preset(LayoutPreset::Default);
        assert!(
            !missing.is_empty(),
            "Wide layout should be missing panels for Default preset"
        );
        // Wide has only Viewport + Inspector, Default has 6 panels, so at least 4 are missing
        assert!(missing.len() >= 4, "Should be missing at least 4 panels");
    }

    // --- EditorMode::status_color() exact RGB ---
    #[test]
    fn test_editor_mode_status_color_edit() {
        let c = EditorMode::Edit.status_color();
        assert_eq!(c, egui::Color32::from_rgb(100, 100, 100));
    }
    #[test]
    fn test_editor_mode_status_color_play() {
        let c = EditorMode::Play.status_color();
        assert_eq!(c, egui::Color32::from_rgb(100, 200, 100));
    }
    #[test]
    fn test_editor_mode_status_color_paused() {
        let c = EditorMode::Paused.status_color();
        assert_eq!(c, egui::Color32::from_rgb(255, 180, 50));
    }

    // --- EditorMode::description() exact values ---
    #[test]
    fn test_editor_mode_description_edit() {
        assert_eq!(
            EditorMode::Edit.description(),
            "Modify scene objects, properties, and layout"
        );
    }
    #[test]
    fn test_editor_mode_description_play() {
        assert_eq!(
            EditorMode::Play.description(),
            "Run the game simulation in real-time"
        );
    }
    #[test]
    fn test_editor_mode_description_paused() {
        assert_eq!(
            EditorMode::Paused.description(),
            "Simulation paused - can step frame by frame"
        );
    }

    // --- EditorMode::icon() exact values ---
    #[test]
    fn test_editor_mode_icon_edit() {
        assert_eq!(EditorMode::Edit.icon(), "\u{1f527}");
    }
    #[test]
    fn test_editor_mode_icon_play() {
        assert_eq!(EditorMode::Play.icon(), "\u{25b6}\u{fe0f}");
    }
    #[test]
    fn test_editor_mode_icon_paused() {
        assert_eq!(EditorMode::Paused.icon(), "\u{23f8}\u{fe0f}");
    }

    // --- EditorMode::status_text() exact values ---
    #[test]
    fn test_editor_mode_status_text_edit() {
        assert_eq!(EditorMode::Edit.status_text(), "Edit Mode");
    }
    #[test]
    fn test_editor_mode_status_text_play() {
        assert_eq!(EditorMode::Play.status_text(), "\u{25b6}\u{fe0f} Playing");
    }
    #[test]
    fn test_editor_mode_status_text_paused() {
        assert_eq!(EditorMode::Paused.status_text(), "\u{23f8}\u{fe0f} Paused");
    }

    // --- EditorMode::action_verb() exact values ---
    #[test]
    fn test_editor_mode_action_verb_each() {
        assert_eq!(EditorMode::Edit.action_verb(), "Stop");
        assert_eq!(EditorMode::Play.action_verb(), "Play");
        assert_eq!(EditorMode::Paused.action_verb(), "Pause");
    }

    // --- EditorMode::next_mode() exact transitions ---
    #[test]
    fn test_editor_mode_next_mode_each() {
        assert_eq!(EditorMode::Edit.next_mode(), EditorMode::Play);
        assert_eq!(EditorMode::Play.next_mode(), EditorMode::Paused);
        assert_eq!(EditorMode::Paused.next_mode(), EditorMode::Play);
    }

    // --- LayoutStats::default() fields ---
    #[test]
    fn test_layout_stats_default_all_fields() {
        let stats = LayoutStats::default();
        assert_eq!(stats.panel_count, 0);
        assert_eq!(stats.tab_group_count, 0);
        assert!(stats.visible_panels.is_empty());
        assert!(!stats.has_viewport);
        assert!(!stats.has_debug_panels);
    }
}

/// Tests for interaction.rs mutation killing
mod interaction_exact_tests {
    use crate::interaction::{
        GizmoCancelMetadata, GizmoCommitMetadata, GizmoMeasurement, GizmoOperationKind,
    };
    use astraweave_core::IVec2;
    use glam::{Quat, Vec3};

    // --- GizmoOperationKind::icon() exact values ---
    #[test]
    fn test_gizmo_op_icon_translate() {
        assert_eq!(GizmoOperationKind::Translate.icon(), "\u{2194}");
    }
    #[test]
    fn test_gizmo_op_icon_rotate() {
        assert_eq!(GizmoOperationKind::Rotate.icon(), "\u{27f3}");
    }
    #[test]
    fn test_gizmo_op_icon_scale() {
        assert_eq!(GizmoOperationKind::Scale.icon(), "\u{2922}");
    }

    // --- GizmoOperationKind::name() exact values ---
    #[test]
    fn test_gizmo_op_name_each() {
        assert_eq!(GizmoOperationKind::Translate.name(), "Translate");
        assert_eq!(GizmoOperationKind::Rotate.name(), "Rotate");
        assert_eq!(GizmoOperationKind::Scale.name(), "Scale");
    }

    // --- GizmoOperationKind::shortcut() exact values ---
    #[test]
    fn test_gizmo_op_shortcut_each() {
        assert_eq!(GizmoOperationKind::Translate.shortcut(), "G");
        assert_eq!(GizmoOperationKind::Rotate.shortcut(), "R");
        assert_eq!(GizmoOperationKind::Scale.shortcut(), "S");
    }

    // --- GizmoMeasurement::kind() exact mapping ---
    #[test]
    fn test_measurement_kind_translate() {
        let m = GizmoMeasurement::Translate {
            from: IVec2 { x: 0, y: 0 },
            to: IVec2 { x: 1, y: 1 },
        };
        assert_eq!(m.kind(), GizmoOperationKind::Translate);
    }
    #[test]
    fn test_measurement_kind_rotate() {
        let m = GizmoMeasurement::Rotate {
            from: (0.0, 0.0, 0.0),
            to: (1.0, 0.0, 0.0),
        };
        assert_eq!(m.kind(), GizmoOperationKind::Rotate);
    }
    #[test]
    fn test_measurement_kind_scale() {
        let m = GizmoMeasurement::Scale { from: 1.0, to: 2.0 };
        assert_eq!(m.kind(), GizmoOperationKind::Scale);
    }

    // --- GizmoMeasurement::magnitude() precise arithmetic ---
    #[test]
    fn test_magnitude_translate_3_4_5() {
        let m = GizmoMeasurement::Translate {
            from: IVec2 { x: 0, y: 0 },
            to: IVec2 { x: 3, y: 4 },
        };
        assert!((m.magnitude() - 5.0).abs() < 1e-6);
    }
    #[test]
    fn test_magnitude_translate_negative() {
        let m = GizmoMeasurement::Translate {
            from: IVec2 { x: 5, y: 5 },
            to: IVec2 { x: 2, y: 1 },
        };
        assert!((m.magnitude() - 5.0).abs() < 1e-6);
    }
    #[test]
    fn test_magnitude_rotate_unit() {
        let m = GizmoMeasurement::Rotate {
            from: (0.0, 0.0, 0.0),
            to: (1.0, 0.0, 0.0),
        };
        assert!((m.magnitude() - 1.0).abs() < 1e-6);
    }
    #[test]
    fn test_magnitude_scale_abs() {
        let m = GizmoMeasurement::Scale { from: 3.0, to: 1.0 };
        assert!((m.magnitude() - 2.0).abs() < 1e-6);
    }
    #[test]
    fn test_magnitude_zero() {
        let m = GizmoMeasurement::Translate {
            from: IVec2 { x: 5, y: 5 },
            to: IVec2 { x: 5, y: 5 },
        };
        assert!((m.magnitude()).abs() < 1e-6);
    }

    // --- GizmoMeasurement::is_significant() threshold ---
    #[test]
    fn test_is_significant_above_threshold() {
        let m = GizmoMeasurement::Scale {
            from: 1.0,
            to: 1.02,
        };
        assert!(m.is_significant());
    }
    #[test]
    fn test_is_significant_at_threshold() {
        let m = GizmoMeasurement::Scale {
            from: 1.0,
            to: 1.01,
        };
        assert!(!m.is_significant());
    }
    #[test]
    fn test_is_significant_below_threshold() {
        let m = GizmoMeasurement::Scale {
            from: 1.0,
            to: 1.005,
        };
        assert!(!m.is_significant());
    }

    // --- GizmoMeasurement::summary() format ---
    #[test]
    fn test_summary_translate_contains_coords() {
        let m = GizmoMeasurement::Translate {
            from: IVec2 { x: 10, y: 20 },
            to: IVec2 { x: 30, y: 40 },
        };
        let s = m.summary();
        assert!(s.contains("10"), "summary should contain from.x");
        assert!(s.contains("20"), "summary should contain from.y");
        assert!(s.contains("30"), "summary should contain to.x");
        assert!(s.contains("40"), "summary should contain to.y");
    }
    #[test]
    fn test_summary_scale_contains_values() {
        let m = GizmoMeasurement::Scale { from: 1.5, to: 2.5 };
        let s = m.summary();
        assert!(s.contains("1.50"), "should contain from scale");
        assert!(s.contains("2.50"), "should contain to scale");
    }

    // --- GizmoCommitMetadata::is_constrained ---
    #[test]
    fn test_commit_is_constrained_true() {
        let meta = GizmoCommitMetadata {
            entity: 1,
            operation: GizmoOperationKind::Translate,
            measurement: GizmoMeasurement::Translate {
                from: IVec2 { x: 0, y: 0 },
                to: IVec2 { x: 1, y: 0 },
            },
            constraint: Some("X".to_string()),
        };
        assert!(meta.is_constrained());
    }
    #[test]
    fn test_commit_is_constrained_false() {
        let meta = GizmoCommitMetadata {
            entity: 1,
            operation: GizmoOperationKind::Translate,
            measurement: GizmoMeasurement::Translate {
                from: IVec2 { x: 0, y: 0 },
                to: IVec2 { x: 1, y: 0 },
            },
            constraint: None,
        };
        assert!(!meta.is_constrained());
    }

    // --- GizmoCommitMetadata::summary() format ---
    #[test]
    fn test_commit_summary_contains_constraint() {
        let meta = GizmoCommitMetadata {
            entity: 42,
            operation: GizmoOperationKind::Translate,
            measurement: GizmoMeasurement::Translate {
                from: IVec2 { x: 0, y: 0 },
                to: IVec2 { x: 5, y: 0 },
            },
            constraint: Some("X".to_string()),
        };
        let s = meta.summary();
        assert!(s.contains("X"), "summary should mention constraint");
    }
    #[test]
    fn test_commit_summary_none_constraint() {
        let meta = GizmoCommitMetadata {
            entity: 1,
            operation: GizmoOperationKind::Scale,
            measurement: GizmoMeasurement::Scale { from: 1.0, to: 2.0 },
            constraint: None,
        };
        let s = meta.summary();
        assert!(s.contains("None"), "should show None for no constraint");
    }

    // --- GizmoCancelMetadata snapshot accessors ---
    #[test]
    fn test_cancel_original_position() {
        use crate::gizmo::state::TransformSnapshot;
        let meta = GizmoCancelMetadata {
            entity: 1,
            operation: GizmoOperationKind::Translate,
            snapshot: TransformSnapshot {
                position: Vec3::new(10.0, 20.0, 30.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        };
        assert_eq!(meta.original_position(), Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(meta.original_rotation(), Quat::IDENTITY);
        assert_eq!(meta.original_scale(), Vec3::ONE);
    }
    #[test]
    fn test_cancel_summary_contains_operation() {
        use crate::gizmo::state::TransformSnapshot;
        let meta = GizmoCancelMetadata {
            entity: 1,
            operation: GizmoOperationKind::Rotate,
            snapshot: TransformSnapshot {
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        };
        let s = meta.summary();
        assert!(s.contains("Rotate"), "should contain operation name");
        assert!(s.contains("cancelled"), "should contain 'cancelled'");
    }
}

/// Tests for toast.rs mutation killing
mod toast_exact_tests {
    use crate::ui::toast::{Toast, ToastAction, ToastLevel, ToastManager};
    use std::time::Duration;

    // --- ToastLevel::color() exact RGB ---
    #[test]
    fn test_toast_color_info() {
        let c = ToastLevel::Info.color();
        assert_eq!(c, egui::Color32::from_rgb(60, 120, 200));
    }
    #[test]
    fn test_toast_color_success() {
        let c = ToastLevel::Success.color();
        assert_eq!(c, egui::Color32::from_rgb(40, 160, 80));
    }
    #[test]
    fn test_toast_color_warning() {
        let c = ToastLevel::Warning.color();
        assert_eq!(c, egui::Color32::from_rgb(200, 140, 40));
    }
    #[test]
    fn test_toast_color_error() {
        let c = ToastLevel::Error.color();
        assert_eq!(c, egui::Color32::from_rgb(200, 60, 60));
    }

    // --- ToastLevel::icon() exact values ---
    #[test]
    fn test_toast_icon_info() {
        assert_eq!(ToastLevel::Info.icon(), "\u{2139}\u{fe0f}");
    }
    #[test]
    fn test_toast_icon_success() {
        assert_eq!(ToastLevel::Success.icon(), "\u{2705}");
    }
    #[test]
    fn test_toast_icon_warning() {
        assert_eq!(ToastLevel::Warning.icon(), "\u{26a0}\u{fe0f}");
    }
    #[test]
    fn test_toast_icon_error() {
        assert_eq!(ToastLevel::Error.icon(), "\u{274c}");
    }

    // --- ToastLevel::name() exact values ---
    #[test]
    fn test_toast_name_each() {
        assert_eq!(ToastLevel::Info.name(), "Info");
        assert_eq!(ToastLevel::Success.name(), "Success");
        assert_eq!(ToastLevel::Warning.name(), "Warning");
        assert_eq!(ToastLevel::Error.name(), "Error");
    }

    // --- ToastLevel::severity() exact values ---
    #[test]
    fn test_toast_severity_each() {
        assert_eq!(ToastLevel::Info.severity(), 0);
        assert_eq!(ToastLevel::Success.severity(), 1);
        assert_eq!(ToastLevel::Warning.severity(), 2);
        assert_eq!(ToastLevel::Error.severity(), 3);
    }

    // --- ToastLevel::is_problem() ---
    #[test]
    fn test_toast_is_problem_true() {
        assert!(ToastLevel::Warning.is_problem());
        assert!(ToastLevel::Error.is_problem());
    }
    #[test]
    fn test_toast_is_problem_false() {
        assert!(!ToastLevel::Info.is_problem());
        assert!(!ToastLevel::Success.is_problem());
    }

    // --- ToastLevel::is_success() ---
    #[test]
    fn test_toast_is_success_true() {
        assert!(ToastLevel::Success.is_success());
    }
    #[test]
    fn test_toast_is_success_false() {
        assert!(!ToastLevel::Info.is_success());
        assert!(!ToastLevel::Warning.is_success());
        assert!(!ToastLevel::Error.is_success());
    }

    // --- ToastAction::label() exact values ---
    #[test]
    fn test_toast_action_label_undo() {
        assert_eq!(ToastAction::Undo.label(), "Undo");
    }
    #[test]
    fn test_toast_action_label_details() {
        assert_eq!(
            ToastAction::ViewDetails("info".to_string()).label(),
            "Details"
        );
    }
    #[test]
    fn test_toast_action_label_retry() {
        assert_eq!(ToastAction::Retry.label(), "Retry");
    }
    #[test]
    fn test_toast_action_label_open() {
        assert_eq!(ToastAction::Open("/path".to_string()).label(), "Open");
    }
    #[test]
    fn test_toast_action_label_custom() {
        let action = ToastAction::Custom {
            label: "MyAction".to_string(),
            action_id: "id1".to_string(),
        };
        assert_eq!(action.label(), "MyAction");
    }

    // --- ToastAction::icon() exact values ---
    #[test]
    fn test_toast_action_icon_each() {
        assert_eq!(ToastAction::Undo.icon(), "\u{21a9}\u{fe0f}");
        assert_eq!(ToastAction::ViewDetails("x".into()).icon(), "\u{1f50d}");
        assert_eq!(ToastAction::Retry.icon(), "\u{1f504}");
        assert_eq!(ToastAction::Open("x".into()).icon(), "\u{1f4c2}");
        assert_eq!(
            ToastAction::Custom {
                label: "x".into(),
                action_id: "y".into()
            }
            .icon(),
            "\u{26a1}"
        );
    }

    // --- ToastAction::is_mutating() ---
    #[test]
    fn test_toast_action_is_mutating_true() {
        assert!(ToastAction::Undo.is_mutating());
        assert!(ToastAction::Retry.is_mutating());
    }
    #[test]
    fn test_toast_action_is_mutating_false() {
        assert!(!ToastAction::ViewDetails("x".into()).is_mutating());
        assert!(!ToastAction::Open("x".into()).is_mutating());
        assert!(!ToastAction::Custom {
            label: "x".into(),
            action_id: "y".into()
        }
        .is_mutating());
    }

    // --- Toast::should_remove() with dismissed ---
    #[test]
    fn test_toast_should_remove_if_dismissed() {
        let mut toast = Toast::new("test", ToastLevel::Info);
        toast.dismissed = true;
        assert!(toast.should_remove());
    }
    #[test]
    fn test_toast_should_not_remove_when_hovered() {
        let mut toast = Toast::new("test", ToastLevel::Info);
        toast.hovered = true;
        toast.duration = Duration::from_millis(0);
        std::thread::sleep(Duration::from_millis(10));
        assert!(!toast.should_remove());
    }
    #[test]
    fn test_toast_not_removed_fresh() {
        let toast = Toast::new("test", ToastLevel::Info);
        assert!(!toast.should_remove());
    }

    // --- Toast builder methods ---
    #[test]
    fn test_toast_with_duration() {
        let toast = Toast::new("msg", ToastLevel::Info).with_duration(Duration::from_secs(10));
        assert_eq!(toast.duration, Duration::from_secs(10));
    }
    #[test]
    fn test_toast_with_action() {
        let toast = Toast::new("msg", ToastLevel::Info).with_action(ToastAction::Undo);
        assert_eq!(toast.actions.len(), 1);
    }
    #[test]
    fn test_toast_with_group() {
        let toast = Toast::new("msg", ToastLevel::Info).with_group("my_group");
        assert_eq!(toast.group_key, Some("my_group".to_string()));
    }

    // --- ToastManager ---
    #[test]
    fn test_toast_manager_new_defaults() {
        let mgr = ToastManager::new();
        assert_eq!(mgr.active_count(), 0);
        assert!(!mgr.has_toasts());
        assert_eq!(mgr.count(), 0);
    }
    #[test]
    fn test_toast_manager_add_and_count() {
        let mut mgr = ToastManager::new();
        mgr.toast("hello", ToastLevel::Info);
        assert_eq!(mgr.active_count(), 1);
        assert!(mgr.has_toasts());
    }
    #[test]
    fn test_toast_manager_group_dedup() {
        let mut mgr = ToastManager::new();
        mgr.add(Toast::new("first", ToastLevel::Info).with_group("g1"));
        mgr.add(Toast::new("second", ToastLevel::Info).with_group("g1"));
        assert_eq!(mgr.active_count(), 1);
    }
    #[test]
    fn test_toast_manager_clear() {
        let mut mgr = ToastManager::new();
        mgr.toast("a", ToastLevel::Info);
        mgr.toast("b", ToastLevel::Warning);
        mgr.clear();
        assert_eq!(mgr.active_count(), 0);
    }
    #[test]
    fn test_toast_manager_convenience_methods() {
        let mut mgr = ToastManager::new();
        mgr.success("ok");
        mgr.error("fail");
        mgr.info("note");
        mgr.warning("warn");
        assert_eq!(mgr.active_count(), 4);
    }
}

/// Tests for scene_state.rs mutation killing
mod scene_state_exact_tests {
    use crate::scene_state::{EditorSceneState, SceneStateIssue, SceneStateStats};
    use astraweave_core::{IVec2, Team, World};
    use glam::Vec3;

    /// Helper: build a world with one entity at a given grid position.
    fn world_with_entity(name: &str, pos: IVec2) -> World {
        let mut w = World::new();
        w.spawn(name, pos, Team { id: 0 }, 100, 10);
        w
    }

    // --- SceneStateIssue error vs warning ---
    #[test]
    fn test_issue_error_is_error_true() {
        let issue = SceneStateIssue::error(1, "bad");
        assert!(issue.is_error);
    }
    #[test]
    fn test_issue_warning_is_error_false() {
        let issue = SceneStateIssue::warning(1, "meh");
        assert!(!issue.is_error);
    }

    // --- SceneStateStats cache_coverage div-by-zero ---
    #[test]
    fn test_stats_coverage_empty_world() {
        let world = World::default();
        let state = EditorSceneState::new(world);
        let stats = state.stats();
        assert_eq!(stats.cache_coverage, 1.0);
    }

    #[test]
    fn test_stats_with_entity() {
        let w = world_with_entity("TestEntity", IVec2 { x: 5, y: 5 });
        let state = EditorSceneState::new(w);
        let stats = state.stats();
        assert_eq!(stats.entity_count, 1);
        assert_eq!(stats.cached_entity_count, 1);
        assert_eq!(stats.cache_coverage, 1.0);
    }

    // --- find_entities_near via world-based spawning ---
    #[test]
    fn test_find_near_includes_close_entity() {
        let w = world_with_entity("Near", IVec2 { x: 3, y: 4 });
        let state = EditorSceneState::new(w);
        let found = state.find_entities_near(Vec3::ZERO, 6.0);
        assert_eq!(found.len(), 1, "entity within radius should be found");
    }
    #[test]
    fn test_find_near_excludes_far_entity() {
        let w = world_with_entity("Far", IVec2 { x: 100, y: 100 });
        let state = EditorSceneState::new(w);
        let found = state.find_entities_near(Vec3::ZERO, 1.0);
        assert_eq!(found.len(), 0, "entity outside radius should be excluded");
    }

    // Kill mutation: replace - with + in find_entities_near
    // Using non-zero center so (pos - center) != (pos + center)
    #[test]
    fn test_find_near_with_nonzero_center() {
        // Entity at (5, 1, 0) — center at (4, 1, 0) — distance = 1.0
        let w = world_with_entity("Close", IVec2 { x: 5, y: 0 });
        let state = EditorSceneState::new(w);
        // pos = (5.0, 1.0, 0.0), center = (4.0, 1.0, 0.0)
        // Original: (5-4, 1-1, 0-0) = (1,0,0), length_sq = 1 <= 4 → found
        // Mutated:  (5+4, 1+1, 0+0) = (9,2,0), length_sq = 85 > 4 → NOT found
        let found = state.find_entities_near(Vec3::new(4.0, 1.0, 0.0), 2.0);
        assert_eq!(
            found.len(),
            1,
            "entity near non-zero center should be found (kills -/+ mutation)"
        );
    }

    // --- validate with valid entities ---
    #[test]
    fn test_validate_valid_world() {
        let w = world_with_entity("Valid", IVec2 { x: 0, y: 0 });
        let state = EditorSceneState::new(w);
        let issues = state.validate();
        assert!(issues.is_empty() || issues.iter().all(|i| !i.is_error));
    }

    // --- validate with zero scale ---
    #[test]
    fn test_validate_zero_scale_is_error() {
        let mut w = World::new();
        let entity = w.spawn("ZeroScale", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 10);
        if let Some(pose) = w.pose_mut(entity) {
            pose.scale = 0.0;
        }
        let state = EditorSceneState::new(w);
        let issues = state.validate();
        assert!(
            issues.iter().any(|i| i.is_error),
            "scale=0 should be an error"
        );
    }

    // --- find_entities_by_name ---
    #[test]
    fn test_find_by_name_case_insensitive() {
        let w = world_with_entity("MyPlayer", IVec2 { x: 0, y: 0 });
        let state = EditorSceneState::new(w);
        let found = state.find_entities_by_name("myplayer");
        assert_eq!(found.len(), 1);
        let found2 = state.find_entities_by_name("MYPLAYER");
        assert_eq!(found2.len(), 1);
    }
    #[test]
    fn test_find_by_name_no_match() {
        let w = world_with_entity("Alpha", IVec2 { x: 0, y: 0 });
        let state = EditorSceneState::new(w);
        let found = state.find_entities_by_name("Beta");
        assert!(found.is_empty());
    }

    // --- entity_count ---
    #[test]
    fn test_entity_count_matches_world() {
        let mut w = World::new();
        w.spawn("A", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 10);
        w.spawn("B", IVec2 { x: 1, y: 1 }, Team { id: 1 }, 100, 10);
        let state = EditorSceneState::new(w);
        assert_eq!(state.entity_count(), 2);
    }

    // --- is_valid ---
    #[test]
    fn test_is_valid_empty() {
        let state = EditorSceneState::new(World::default());
        assert!(state.is_valid());
    }

    // --- clear_cache ---
    #[test]
    fn test_clear_cache_resets() {
        let w = world_with_entity("ToClear", IVec2 { x: 0, y: 0 });
        let mut state = EditorSceneState::new(w);
        assert!(state.all_cached_entities().count() > 0);
        state.clear_cache();
        assert_eq!(state.all_cached_entities().count(), 0);
    }

    // --- transform_for returns correct position ---
    #[test]
    fn test_transform_for_existing() {
        let w = world_with_entity("HasPose", IVec2 { x: 7, y: 3 });
        let state = EditorSceneState::new(w);
        let entities: Vec<_> = state.all_cached_entities().map(|(e, _)| e).collect();
        assert!(!entities.is_empty());
        let t = state.transform_for(entities[0]);
        assert!(t.is_some());
        let t = t.unwrap();
        assert!((t.position.x - 7.0).abs() < 0.01);
        assert!((t.position.z - 3.0).abs() < 0.01);
        assert!((t.position.y - 0.0).abs() < 0.01);
    }
}

/// Tests for behavior_graph/document.rs mutation killing
mod behavior_graph_exact_tests {
    use crate::behavior_graph::document::{
        BehaviorGraphNodeKind, DecoratorKind, DecoratorNode, NodePosition,
    };

    // --- DecoratorKind::name() exact ---
    #[test]
    fn test_decorator_name_each() {
        assert_eq!(DecoratorKind::Inverter.name(), "Inverter");
        assert_eq!(DecoratorKind::Succeeder.name(), "Succeeder");
        assert_eq!(DecoratorKind::Failer.name(), "Failer");
        assert_eq!(DecoratorKind::Repeat(3).name(), "Repeat");
        assert_eq!(DecoratorKind::Retry(5).name(), "Retry");
    }

    // --- DecoratorKind::icon() exact ---
    #[test]
    fn test_decorator_icon_inverter() {
        assert_eq!(DecoratorKind::Inverter.icon(), "\u{1f504}");
    }
    #[test]
    fn test_decorator_icon_succeeder() {
        assert_eq!(DecoratorKind::Succeeder.icon(), "\u{2705}");
    }
    #[test]
    fn test_decorator_icon_failer() {
        assert_eq!(DecoratorKind::Failer.icon(), "\u{274c}");
    }
    #[test]
    fn test_decorator_icon_repeat() {
        assert_eq!(DecoratorKind::Repeat(1).icon(), "\u{1f501}");
    }
    #[test]
    fn test_decorator_icon_retry() {
        assert_eq!(DecoratorKind::Retry(1).icon(), "\u{1f502}");
    }

    // --- DecoratorKind::modifies_result() ---
    #[test]
    fn test_decorator_modifies_result_true() {
        assert!(DecoratorKind::Inverter.modifies_result());
        assert!(DecoratorKind::Succeeder.modifies_result());
        assert!(DecoratorKind::Failer.modifies_result());
    }
    #[test]
    fn test_decorator_modifies_result_false() {
        assert!(!DecoratorKind::Repeat(1).modifies_result());
        assert!(!DecoratorKind::Retry(1).modifies_result());
    }

    // --- DecoratorKind::is_looping() ---
    #[test]
    fn test_decorator_is_looping_true() {
        assert!(DecoratorKind::Repeat(1).is_looping());
        assert!(DecoratorKind::Retry(1).is_looping());
    }
    #[test]
    fn test_decorator_is_looping_false() {
        assert!(!DecoratorKind::Inverter.is_looping());
        assert!(!DecoratorKind::Succeeder.is_looping());
        assert!(!DecoratorKind::Failer.is_looping());
    }

    // --- DecoratorKind::loop_count() ---
    #[test]
    fn test_decorator_loop_count() {
        assert_eq!(DecoratorKind::Repeat(7).loop_count(), Some(7));
        assert_eq!(DecoratorKind::Retry(3).loop_count(), Some(3));
        assert_eq!(DecoratorKind::Inverter.loop_count(), None);
    }

    // --- BehaviorGraphNodeKind::display_name() exact ---
    #[test]
    fn test_node_kind_display_name_action() {
        let kind = BehaviorGraphNodeKind::Action {
            name: "attack".into(),
        };
        assert_eq!(kind.display_name(), "Action");
    }
    #[test]
    fn test_node_kind_display_name_condition() {
        let kind = BehaviorGraphNodeKind::Condition {
            name: "has_target".into(),
        };
        assert_eq!(kind.display_name(), "Condition");
    }
    #[test]
    fn test_node_kind_display_name_sequence() {
        let kind = BehaviorGraphNodeKind::Sequence { children: vec![] };
        assert_eq!(kind.display_name(), "Sequence");
    }
    #[test]
    fn test_node_kind_display_name_selector() {
        let kind = BehaviorGraphNodeKind::Selector { children: vec![] };
        assert_eq!(kind.display_name(), "Selector");
    }
    #[test]
    fn test_node_kind_display_name_parallel() {
        let kind = BehaviorGraphNodeKind::Parallel {
            children: vec![],
            success_threshold: 1,
        };
        assert_eq!(kind.display_name(), "Parallel");
    }
    #[test]
    fn test_node_kind_display_name_decorator() {
        let kind = BehaviorGraphNodeKind::Decorator(DecoratorNode::new(DecoratorKind::Inverter));
        assert_eq!(kind.display_name(), "Decorator");
    }

    // --- BehaviorGraphNodeKind::icon() exact ---
    #[test]
    fn test_node_kind_icon_each() {
        assert_eq!(
            BehaviorGraphNodeKind::Action { name: "x".into() }.icon(),
            "\u{26a1}"
        );
        assert_eq!(
            BehaviorGraphNodeKind::Condition { name: "x".into() }.icon(),
            "\u{2753}"
        );
        assert_eq!(
            BehaviorGraphNodeKind::Sequence { children: vec![] }.icon(),
            "\u{27a1}\u{fe0f}"
        );
        assert_eq!(
            BehaviorGraphNodeKind::Selector { children: vec![] }.icon(),
            "\u{1f500}"
        );
        assert_eq!(
            BehaviorGraphNodeKind::Parallel {
                children: vec![],
                success_threshold: 1
            }
            .icon(),
            "\u{23f8}"
        );
        assert_eq!(
            BehaviorGraphNodeKind::Decorator(DecoratorNode::new(DecoratorKind::Inverter)).icon(),
            "\u{1f381}"
        );
    }

    // --- BehaviorGraphNodeKind::child_count() ---
    #[test]
    fn test_node_kind_child_count_leaf() {
        let kind = BehaviorGraphNodeKind::Action { name: "x".into() };
        assert_eq!(kind.child_count(), 0);
    }
    #[test]
    fn test_node_kind_child_count_sequence() {
        let kind = BehaviorGraphNodeKind::Sequence {
            children: vec![1, 2, 3],
        };
        assert_eq!(kind.child_count(), 3);
    }
    #[test]
    fn test_node_kind_child_count_decorator_with_child() {
        let mut node = DecoratorNode::new(DecoratorKind::Inverter);
        node.child = Some(42);
        let kind = BehaviorGraphNodeKind::Decorator(node);
        assert_eq!(kind.child_count(), 1);
    }
    #[test]
    fn test_node_kind_child_count_decorator_without_child() {
        let kind = BehaviorGraphNodeKind::Decorator(DecoratorNode::new(DecoratorKind::Inverter));
        assert_eq!(kind.child_count(), 0);
    }

    // --- NodePosition default ---
    #[test]
    fn test_node_position_default() {
        let pos = NodePosition::default();
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 0.0);
    }

    // --- DecoratorKind::default ---
    #[test]
    fn test_decorator_kind_default() {
        assert_eq!(DecoratorKind::default(), DecoratorKind::Inverter);
    }

    // --- DecoratorKind Display ---
    #[test]
    fn test_decorator_display_repeat() {
        assert_eq!(format!("{}", DecoratorKind::Repeat(5)), "Repeat (5)");
    }
    #[test]
    fn test_decorator_display_retry() {
        assert_eq!(format!("{}", DecoratorKind::Retry(3)), "Retry (3)");
    }
    #[test]
    fn test_decorator_display_inverter() {
        assert_eq!(format!("{}", DecoratorKind::Inverter), "Inverter");
    }
}

// ==========================================================================
// v4 Mutation Kill Tests — dock_layout, scene_state additional coverage
// ==========================================================================

/// Additional tests for dock_layout.rs mutations (v4 misses)
mod dock_layout_v4_tests {
    use crate::dock_layout::{DockLayout, LayoutPreset};

    // --- has_debug_panels must return false for non-debug layouts ---
    #[test]
    fn test_has_debug_panels_false_for_wide() {
        let layout = DockLayout::from_preset(LayoutPreset::Wide);
        // Wide only has Viewport + Inspector — no debug panels
        assert!(
            !layout.has_debug_panels(),
            "Wide layout should have no debug panels"
        );
    }

    #[test]
    fn test_has_debug_panels_true_for_debug() {
        let layout = DockLayout::from_preset(LayoutPreset::Debug);
        assert!(
            layout.has_debug_panels(),
            "Debug layout should have debug panels"
        );
    }

    // --- matches_preset must return false for non-matching presets ---
    #[test]
    fn test_matches_preset_false_for_mismatch() {
        let layout = DockLayout::from_preset(LayoutPreset::Wide);
        // Wide != Default
        assert!(
            !layout.matches_preset(LayoutPreset::Default),
            "Wide layout should not match Default preset"
        );
    }

    #[test]
    fn test_matches_preset_true_for_same() {
        let layout = DockLayout::from_preset(LayoutPreset::Default);
        assert!(
            layout.matches_preset(LayoutPreset::Default),
            "Default layout should match Default preset"
        );
    }

    // --- closest_preset must return the correct preset ---
    #[test]
    fn test_closest_preset_debug_returns_debug() {
        let layout = DockLayout::from_preset(LayoutPreset::Debug);
        assert_eq!(
            layout.closest_preset(),
            LayoutPreset::Debug,
            "Debug layout's closest preset must be Debug, not Default"
        );
    }

    #[test]
    fn test_closest_preset_not_always_default() {
        // Kill mutation: closest_preset -> Default::default() (which is Default)
        // Debug layout's closest preset must NOT be Default
        let layout = DockLayout::from_preset(LayoutPreset::Debug);
        assert_ne!(
            layout.closest_preset(),
            LayoutPreset::Default,
            "Debug layout must not return Default as closest preset"
        );
    }
}

/// Additional scene_state.rs tests for v4 mutations
mod scene_state_v4_tests {
    use crate::gizmo::state::TransformSnapshot;
    use crate::scene_state::{EditorSceneState, TransformableScene};
    use astraweave_core::{IVec2, Team, World};
    use glam::{Quat, Vec3};

    /// Helper: build world + EditorSceneState with one entity
    fn state_with_entity(
        name: &str,
        x: i32,
        y: i32,
    ) -> (EditorSceneState, astraweave_core::Entity) {
        let mut w = World::new();
        let e = w.spawn(name, IVec2 { x, y }, Team { id: 0 }, 100, 10);
        let state = EditorSceneState::new(w);
        (state, e)
    }

    // --- world() must return the internal world, not a leaked default ---
    #[test]
    fn test_world_returns_actual_world() {
        let (state, entity) = state_with_entity("probe", 5, 10);
        // If world() returned a leaked Default::default(), it wouldn't have our entity
        let world = state.world();
        assert!(
            world.pose(entity).is_some(),
            "world() must return the actual internal world containing our entity"
        );
    }

    // --- world_mut() must return the actual mutable world ---
    #[test]
    fn test_world_mut_returns_actual_world() {
        let (mut state, entity) = state_with_entity("probe", 5, 10);
        // Mutate through world_mut, then read back through world
        let new_entity =
            state
                .world_mut()
                .spawn("new_entity", IVec2 { x: 0, y: 0 }, Team { id: 1 }, 50, 5);
        assert!(
            state.world().pose(new_entity).is_some(),
            "world_mut() must return the actual internal world"
        );
    }

    // --- get_editor_entity_mut must return Some for existing entity ---
    #[test]
    fn test_get_editor_entity_mut_returns_some() {
        let (mut state, entity) = state_with_entity("hero", 3, 4);
        let result = state.get_editor_entity_mut(entity);
        assert!(
            result.is_some(),
            "get_editor_entity_mut must return Some for existing entity"
        );
        let editor_entity = result.unwrap();
        assert_eq!(editor_entity.name, "hero");
    }

    // --- apply_transform updates the world pose ---
    #[test]
    fn test_apply_transform_updates_world() {
        use crate::gizmo::scene_viewport::Transform;
        let (mut state, entity) = state_with_entity("target", 0, 0);
        let new_transform = Transform {
            position: Vec3::new(10.0, 1.0, 20.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        state.apply_transform(entity, &new_transform);
        let pose = state.world().pose(entity).unwrap();
        assert_eq!(pose.pos.x, 10);
        assert_eq!(pose.pos.y, 20);
    }

    // --- TransformableScene::world() must return actual world ---
    #[test]
    fn test_transformable_scene_world() {
        let (state, entity) = state_with_entity("ts_probe", 7, 8);
        let world = TransformableScene::world(&state);
        assert!(
            world.pose(entity).is_some(),
            "TransformableScene::world() must return the actual world"
        );
    }

    // --- TransformableScene::world_mut() must return actual mutable world ---
    #[test]
    fn test_transformable_scene_world_mut() {
        let (mut state, _) = state_with_entity("ts_probe", 7, 8);
        let new_e = TransformableScene::world_mut(&mut state).spawn(
            "added",
            IVec2 { x: 1, y: 2 },
            Team { id: 0 },
            50,
            5,
        );
        assert!(
            TransformableScene::world(&state).pose(new_e).is_some(),
            "TransformableScene::world_mut() must modify the actual world"
        );
    }

    // --- TransformableScene::sync_all must update cache ---
    #[test]
    fn test_transformable_scene_sync_all_updates_cache() {
        let (mut state, entity) = state_with_entity("sync_test", 5, 5);
        // Clear cache first (via EditorSceneState method)
        state.clear_cache();
        assert_eq!(state.stats().cached_entity_count, 0);
        // sync_all should rebuild cache
        TransformableScene::sync_all(&mut state);
        assert!(
            state.stats().cached_entity_count > 0,
            "sync_all must rebuild the cache"
        );
    }

    // --- TransformableScene::snapshot_for ---
    #[test]
    fn test_snapshot_for_existing_entity() {
        let (state, entity) = state_with_entity("snap", 3, 7);
        let snap = TransformableScene::snapshot_for(&state, entity);
        assert!(
            snap.is_some(),
            "snapshot_for must return Some for existing entity"
        );
        let snap = snap.unwrap();
        // Position should reflect entity at (3, 7) → (3.0, 0.0, 7.0)
        assert!((snap.position.x - 3.0).abs() < 0.01);
        assert!((snap.position.y - 0.0).abs() < 0.01);
        assert!((snap.position.z - 7.0).abs() < 0.01);
    }

    #[test]
    fn test_snapshot_for_nonexistent_entity() {
        let (state, _) = state_with_entity("snap", 3, 7);
        let snap = TransformableScene::snapshot_for(&state, 99999);
        assert!(
            snap.is_none(),
            "snapshot_for must return None for nonexistent entity"
        );
    }

    #[test]
    fn test_snapshot_for_has_correct_values_not_default() {
        // Kill mutation: snapshot_for -> Some(Default::default())
        let (state, entity) = state_with_entity("offset", 10, 20);
        let snap = TransformableScene::snapshot_for(&state, entity).unwrap();
        // Default position is Vec3::ZERO, but our entity is at (10, 1, 20)
        assert!(
            snap.position.x > 1.0,
            "snapshot_for must return actual position, not Default"
        );
        assert!(
            snap.position.z > 1.0,
            "snapshot_for z must reflect entity y coordinate"
        );
    }

    // --- TransformableScene::apply_snapshot ---
    #[test]
    fn test_apply_snapshot_modifies_world() {
        let (mut state, entity) = state_with_entity("snap_apply", 0, 0);
        let snapshot = TransformSnapshot {
            position: Vec3::new(15.0, 1.0, 25.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        TransformableScene::apply_snapshot(&mut state, entity, &snapshot);
        let pose = state.world().pose(entity).unwrap();
        assert_eq!(pose.pos.x, 15, "apply_snapshot must update world x");
        assert_eq!(pose.pos.y, 25, "apply_snapshot must update world y");
    }

    // --- stats: division (not multiplication) for cache_coverage ---
    #[test]
    fn test_stats_coverage_is_division_not_multiplication() {
        // With 2 entities and 2 cached, coverage = 2/2 = 1.0
        // If mutated to multiplication: 2*2 = 4.0 — clearly wrong
        let mut w = World::new();
        w.spawn("a", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 10);
        w.spawn("b", IVec2 { x: 1, y: 1 }, Team { id: 0 }, 100, 10);
        let state = EditorSceneState::new(w);
        let stats = state.stats();
        assert_eq!(stats.entity_count, 2);
        assert!(
            stats.cache_coverage <= 1.0,
            "cache_coverage must be <= 1.0 (division), got {}",
            stats.cache_coverage
        );
        assert!(
            (stats.cache_coverage - 1.0).abs() < 0.01,
            "With all entities cached, coverage should be 1.0, got {}",
            stats.cache_coverage
        );
    }

    // --- validate: rotation checks ---
    #[test]
    fn test_validate_valid_entity_no_rotation_warnings() {
        // Entity with valid (finite, normalized) rotation → no rotation warnings
        let (state, _) = state_with_entity("valid_rot", 1, 1);
        let issues = state.validate();
        let rotation_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("Rotation") || i.message.contains("rotation"))
            .collect();
        assert!(
            rotation_issues.is_empty(),
            "Valid entity should have no rotation issues, got: {:?}",
            rotation_issues
        );
    }

    // --- validate: delete ! in scale check (line 269 region) ---
    #[test]
    fn test_validate_with_all_entities_cached_no_uncached_warning() {
        // After sync, all entities should be cached — no "not in cache" warnings
        let (state, _) = state_with_entity("cached", 5, 5);
        let issues = state.validate();
        let uncached_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("not in cache"))
            .collect();
        assert!(
            uncached_issues.is_empty(),
            "After sync, no 'not in cache' warnings should exist, got: {:?}",
            uncached_issues
        );
    }

    // --- validate: scale checks ---
    #[test]
    fn test_validate_normal_scale_no_error() {
        // Entity with default scale (1.0) should produce no scale errors
        let (state, _) = state_with_entity("good_scale", 1, 1);
        let issues = state.validate();
        let scale_errors: Vec<_> = issues
            .iter()
            .filter(|i| i.message.contains("scale") || i.message.contains("Scale"))
            .collect();
        assert!(
            scale_errors.is_empty(),
            "Entity with normal scale should have no scale errors, got: {:?}",
            scale_errors
        );
    }
}
