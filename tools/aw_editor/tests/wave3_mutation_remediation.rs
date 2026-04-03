//! Wave 3 mutation-resistant remediation tests for aw_editor.
//!
//! Targets specific mutation survivors identified by cargo-mutants analysis.
//! Pins exact values, boundary operators, state-machine transitions, and
//! division-by-zero guards.

use astraweave_core::IVec2;
use aw_editor_lib::clipboard::{
    ClipboardData, ClipboardEntityData, ClipboardValidation, CLIPBOARD_SCHEMA_VERSION,
};
use aw_editor_lib::entity_manager::{EntityMaterial, MaterialSlot};
use aw_editor_lib::plugin::{PluginManagerStats, PluginState};
use aw_editor_lib::prefab::EntityOverrides;
use aw_editor_lib::{
    DockLayout, EditorEntity, EditorMode, EditorRuntime, EntityManager, LayoutPreset, PluginError,
    PluginEvent, PrefabHierarchySnapshot, RuntimeState, RuntimeStats, SelectionSet, UndoStack,
};
use glam::{Quat, Vec3, Vec4};

// ============================================================================
// REMEDIATION 1: entity_manager — Material slot classification
// ============================================================================

mod material_slot_exact {
    use super::*;

    #[test]
    fn is_color_slot_only_albedo_and_emission() {
        assert!(MaterialSlot::Albedo.is_color_slot());
        assert!(MaterialSlot::Emission.is_color_slot());
        // Everything else is NOT a color slot
        assert!(!MaterialSlot::Normal.is_color_slot());
        assert!(!MaterialSlot::Roughness.is_color_slot());
        assert!(!MaterialSlot::Metallic.is_color_slot());
        assert!(!MaterialSlot::AO.is_color_slot());
        assert!(!MaterialSlot::ORM.is_color_slot());
        assert!(!MaterialSlot::Height.is_color_slot());
    }

    #[test]
    fn is_data_slot_is_negation_of_color() {
        for &slot in MaterialSlot::all() {
            assert_eq!(
                slot.is_data_slot(),
                !slot.is_color_slot(),
                "is_data_slot should be !is_color_slot for {:?}",
                slot
            );
        }
    }

    #[test]
    fn all_returns_all_8_variants() {
        assert_eq!(MaterialSlot::all().len(), 8);
    }

    #[test]
    fn name_unique_per_variant() {
        let names: Vec<&str> = MaterialSlot::all().iter().map(|s| s.name()).collect();
        let mut deduped = names.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "All slot names must be unique");
    }
}

// ============================================================================
// REMEDIATION 2: entity_manager — EntityMaterial default values
// ============================================================================

mod entity_material_defaults {
    use super::*;

    #[test]
    fn default_values_exact() {
        let mat = EntityMaterial::new();
        assert_eq!(mat.metallic, 0.0, "Default metallic");
        assert_eq!(mat.roughness, 0.5, "Default roughness");
        assert_eq!(mat.emissive, Vec3::ZERO, "Default emissive");
        assert_eq!(mat.normal_strength, 1.0, "Default normal_strength");
        assert_eq!(mat.base_color, Vec4::ONE, "Default base_color");
        assert_eq!(mat.name, "Default", "Default name");
    }

    #[test]
    fn is_metallic_boundary_at_0_5() {
        let mut mat = EntityMaterial::new();

        mat.metallic = 0.5;
        assert!(
            !mat.is_metallic(),
            "metallic=0.5 should NOT be metallic (> 0.5 required)"
        );

        mat.metallic = 0.501;
        assert!(mat.is_metallic(), "metallic=0.501 should be metallic");

        mat.metallic = 0.0;
        assert!(!mat.is_metallic(), "metallic=0.0 should not be metallic");
    }

    #[test]
    fn is_rough_boundary_at_0_5() {
        let mut mat = EntityMaterial::new();

        mat.roughness = 0.5;
        assert!(
            !mat.is_rough(),
            "roughness=0.5 should NOT be rough (> 0.5 required)"
        );

        mat.roughness = 0.501;
        assert!(mat.is_rough(), "roughness=0.501 should be rough");
    }

    #[test]
    fn is_emissive_boundary() {
        let mut mat = EntityMaterial::new();

        mat.emissive = Vec3::ZERO;
        assert!(!mat.is_emissive(), "zero emissive → not emissive");

        // length_squared = 0.001 → not emissive (boundary: > 0.001)
        mat.emissive = Vec3::new(0.0316, 0.0, 0.0); // ~0.001 sq
        assert!(!mat.is_emissive(), "barely at boundary → not emissive");

        mat.emissive = Vec3::new(1.0, 0.0, 0.0);
        assert!(mat.is_emissive(), "bright emissive → is emissive");
    }

    #[test]
    fn has_textures_empty() {
        let mat = EntityMaterial::new();
        assert!(!mat.has_textures(), "Default has no textures");
        assert_eq!(mat.texture_count(), 0);
    }

    #[test]
    fn set_and_get_texture() {
        let mut mat = EntityMaterial::new();
        mat.set_texture(MaterialSlot::Albedo, "test.png".into());
        assert!(mat.has_textures());
        assert_eq!(mat.texture_count(), 1);
        assert!(mat.get_texture(MaterialSlot::Albedo).is_some());
        assert!(mat.get_texture(MaterialSlot::Normal).is_none());

        mat.clear_texture(MaterialSlot::Albedo);
        assert!(!mat.has_textures());
    }
}

// ============================================================================
// REMEDIATION 3: entity_manager — EditorEntity validate boundary checks
// ============================================================================

mod entity_validate_boundary {
    use super::*;

    #[test]
    fn default_entity_is_valid() {
        let e = EditorEntity::new(1, "Test".to_string());
        assert!(e.is_valid());
    }

    #[test]
    fn negative_scale_is_invalid() {
        let mut e = EditorEntity::new(1, "Test".to_string());
        e.scale = Vec3::new(-1.0, 1.0, 1.0);
        let v = e.validate();
        assert!(!v.is_valid, "Negative scale should be invalid");
    }

    #[test]
    fn zero_scale_is_invalid() {
        let mut e = EditorEntity::new(1, "Test".to_string());
        e.scale = Vec3::new(0.0, 1.0, 1.0);
        let v = e.validate();
        assert!(!v.is_valid, "Zero scale should be invalid");
    }

    #[test]
    fn name_too_long_is_invalid() {
        let long_name = "x".repeat(257);
        let e = EditorEntity::new(1, long_name);
        let v = e.validate();
        assert!(!v.is_valid, "Name > 256 chars should be invalid");
    }

    #[test]
    fn name_exactly_256_is_valid() {
        let name = "x".repeat(256);
        let e = EditorEntity::new(1, name);
        let v = e.validate();
        assert!(v.is_valid, "Name of exactly 256 chars should be valid");
    }

    #[test]
    fn nan_position_is_invalid() {
        let mut e = EditorEntity::new(1, "Test".to_string());
        e.position = Vec3::new(f32::NAN, 0.0, 0.0);
        let v = e.validate();
        assert!(!v.is_valid, "NaN position should be invalid");
    }

    #[test]
    fn unnormalized_rotation_boundary() {
        let mut e = EditorEntity::new(1, "Test".to_string());
        // (rot_length - 1.0).abs() > 0.01 → invalid
        e.rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 1.02);
        let v = e.validate();
        // length = 1.02, deviation = 0.02 > 0.01 → should flag
        assert!(
            !v.errors.is_empty() || !v.warnings.is_empty(),
            "Unnormalized rotation should flag"
        );
    }

    #[test]
    fn aabb_computation_exact() {
        let mut e = EditorEntity::new(1, "Test".to_string());
        e.position = Vec3::new(10.0, 20.0, 30.0);
        e.scale = Vec3::new(4.0, 6.0, 8.0);
        let (min, max) = e.aabb();
        // half_size = scale * 0.5 = (2, 3, 4)
        assert_eq!(min, Vec3::new(8.0, 17.0, 26.0));
        assert_eq!(max, Vec3::new(12.0, 23.0, 34.0));
    }

    #[test]
    fn transform_returns_correct_fields() {
        let mut e = EditorEntity::new(1, "Test".to_string());
        e.position = Vec3::new(1.0, 2.0, 3.0);
        e.rotation = Quat::IDENTITY;
        e.scale = Vec3::ONE;
        let (pos, rot, scale) = e.transform();
        assert_eq!(pos, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(rot, Quat::IDENTITY);
        assert_eq!(scale, Vec3::ONE);
    }
}

// ============================================================================
// REMEDIATION 4: entity_manager — EntityManager find_in_region boundary
// ============================================================================

mod entity_manager_region {
    use super::*;

    #[test]
    fn find_in_region_inclusive_boundaries() {
        let mut mgr = EntityManager::new();
        let id = mgr.create("at_boundary".to_string());
        mgr.update_position(id, Vec3::new(5.0, 5.0, 5.0));

        // Exact match at boundary should be found (>= min, <= max)
        let found = mgr.find_in_region(Vec3::new(5.0, 5.0, 5.0), Vec3::new(5.0, 5.0, 5.0));
        assert!(
            found.contains(&id),
            "Entity at exact boundary should be found"
        );
    }

    #[test]
    fn find_in_region_excludes_outside() {
        let mut mgr = EntityManager::new();
        let id = mgr.create("outside".to_string());
        mgr.update_position(id, Vec3::new(10.0, 10.0, 10.0));

        let found = mgr.find_in_region(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0));
        assert!(
            !found.contains(&id),
            "Entity outside region should not be found"
        );
    }

    #[test]
    fn find_by_name_case_insensitive() {
        let mut mgr = EntityManager::new();
        let _id = mgr.create("MyEntity".to_string());

        assert!(
            !mgr.find_by_name("myentity").is_empty(),
            "Should find case-insensitive"
        );
        assert!(
            !mgr.find_by_name("MYENTITY").is_empty(),
            "Should find uppercase"
        );
    }

    #[test]
    fn clear_resets_next_id() {
        let mut mgr = EntityManager::new();
        let _id1 = mgr.create("A".to_string());
        let _id2 = mgr.create("B".to_string());
        assert_eq!(mgr.count(), 2);

        mgr.clear();
        assert_eq!(mgr.count(), 0);

        // After clear, next_id should reset to 1
        let id3 = mgr.create("C".to_string());
        assert_eq!(id3, 1, "After clear, next_id should reset to 1");
    }

    #[test]
    fn success_rate_zero_entities_is_100() {
        let mgr = EntityManager::new();
        let validation = mgr.validate_all();
        assert_eq!(
            validation.success_rate(),
            100.0,
            "Zero entities → 100% success rate"
        );
    }
}

// ============================================================================
// REMEDIATION 5: entity_manager — SelectionSet boundary precision
// ============================================================================

mod selection_exact {
    use super::*;

    #[test]
    fn is_multi_select_boundary() {
        let mut sel = SelectionSet::default();
        assert!(!sel.is_multi_select(), "Empty → not multi-select");

        sel.add(1, true);
        assert!(!sel.is_multi_select(), "1 entity → not multi-select");

        sel.add(2, false);
        assert!(sel.is_multi_select(), "2 entities → multi-select");
    }

    #[test]
    fn remove_primary_reassigns() {
        let mut sel = SelectionSet::default();
        sel.add(1, true);
        sel.add(2, false);
        assert!(sel.is_primary(1));

        sel.remove(1);
        // After removing primary, primary should be reassigned or None
        assert!(!sel.is_primary(1));
        assert_eq!(sel.count(), 1);
    }

    #[test]
    fn toggle_adds_and_removes() {
        let mut sel = SelectionSet::default();
        sel.toggle(1);
        assert!(sel.is_selected(1));

        sel.toggle(1);
        assert!(!sel.is_selected(1));
    }

    #[test]
    fn select_only_clears_and_sets_primary() {
        let mut sel = SelectionSet::default();
        sel.add(1, true);
        sel.add(2, false);
        sel.add(3, false);

        sel.select_only(5);
        assert_eq!(sel.count(), 1);
        assert!(sel.is_selected(5));
        assert!(sel.is_primary(5));
        assert!(!sel.is_selected(1));
    }

    #[test]
    fn summary_matches_count() {
        let mut sel = SelectionSet::default();
        let s0 = sel.summary();
        assert!(
            s0.contains("0") || s0.to_lowercase().contains("no"),
            "Empty summary"
        );

        sel.add(1, true);
        let s1 = sel.summary();
        assert!(!s1.is_empty(), "Single selection summary");

        sel.add(2, false);
        let s2 = sel.summary();
        assert!(
            s2.contains("2"),
            "Multi-selection summary should mention count"
        );
    }
}

// ============================================================================
// REMEDIATION 6: command.rs — UndoStack cursor and boundary
// ============================================================================

mod undo_stack_exact {
    use super::*;

    #[test]
    fn new_undo_stack_state() {
        let stack = UndoStack::new(100);
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.cursor(), 0);
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 0);
        assert_eq!(stack.max_size(), 100);
        assert!(stack.is_auto_merge_enabled(), "auto_merge defaults to true");
    }

    #[test]
    fn stats_utilization_zero_max_returns_zero() {
        let stack = UndoStack::new(0);
        let stats = stack.stats();
        assert_eq!(stats.utilization(), 0.0, "zero max_size → 0.0 utilization");
    }

    #[test]
    fn validate_issues_near_capacity() {
        let stack = UndoStack::new(1);
        let issues = stack.validate();
        // Empty stack with max_size=1: might report NoHistory
        // Just verify it doesn't panic and returns something
        assert!(
            !issues.iter().any(|i| i.is_error()),
            "Empty stack should not have error issues"
        );
    }

    #[test]
    fn clear_resets_cursor() {
        let mut stack = UndoStack::new(100);
        stack.clear();
        assert_eq!(stack.cursor(), 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn set_auto_merge() {
        let mut stack = UndoStack::new(100);
        assert!(stack.is_auto_merge_enabled());
        stack.set_auto_merge(false);
        assert!(!stack.is_auto_merge_enabled());
        stack.set_auto_merge(true);
        assert!(stack.is_auto_merge_enabled());
    }
}

// ============================================================================
// REMEDIATION 7: editor_mode — State machine transitions
// ============================================================================

mod editor_mode_transitions {
    use super::*;

    #[test]
    fn edit_can_only_go_to_play() {
        assert!(EditorMode::Edit.can_transition_to(EditorMode::Play));
        assert!(EditorMode::Edit.can_transition_to(EditorMode::Edit)); // self
        assert!(!EditorMode::Edit.can_transition_to(EditorMode::Paused));
    }

    #[test]
    fn play_can_go_to_all() {
        assert!(EditorMode::Play.can_transition_to(EditorMode::Edit));
        assert!(EditorMode::Play.can_transition_to(EditorMode::Paused));
        assert!(EditorMode::Play.can_transition_to(EditorMode::Play)); // self
    }

    #[test]
    fn paused_can_go_to_edit_or_play() {
        assert!(EditorMode::Paused.can_transition_to(EditorMode::Edit));
        assert!(EditorMode::Paused.can_transition_to(EditorMode::Play));
        // Paused → Paused: depends on implementation
    }

    #[test]
    fn is_playing_only_play() {
        assert!(EditorMode::Play.is_playing());
        assert!(!EditorMode::Edit.is_playing());
        assert!(!EditorMode::Paused.is_playing());
    }

    #[test]
    fn is_editing_only_edit() {
        assert!(EditorMode::Edit.is_editing());
        assert!(!EditorMode::Play.is_editing());
        assert!(!EditorMode::Paused.is_editing());
    }

    #[test]
    fn can_edit_only_edit() {
        assert!(EditorMode::Edit.can_edit());
        assert!(!EditorMode::Play.can_edit());
        assert!(!EditorMode::Paused.can_edit());
    }

    #[test]
    fn default_is_edit() {
        assert_eq!(EditorMode::default(), EditorMode::Edit);
    }

    #[test]
    fn all_returns_3_variants() {
        assert_eq!(EditorMode::all().len(), 3);
    }

    #[test]
    fn next_mode_cycle() {
        assert_eq!(EditorMode::Edit.next_mode(), EditorMode::Play);
        assert_eq!(EditorMode::Play.next_mode(), EditorMode::Paused);
        assert_eq!(EditorMode::Paused.next_mode(), EditorMode::Play);
    }

    #[test]
    fn valid_transitions_edit() {
        let t = EditorMode::Edit.valid_transitions();
        assert!(
            t.contains(&EditorMode::Play),
            "Edit should list Play as valid"
        );
        assert!(
            !t.contains(&EditorMode::Paused),
            "Edit should not list Paused"
        );
    }

    #[test]
    fn shortcut_hints_unique() {
        let hints: Vec<&str> = EditorMode::all()
            .iter()
            .map(|m| m.shortcut_hint())
            .collect();
        let mut deduped = hints.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(hints.len(), deduped.len());
    }

    #[test]
    fn is_simulating_only_play() {
        assert!(EditorMode::Play.is_simulating());
        assert!(!EditorMode::Edit.is_simulating());
        assert!(!EditorMode::Paused.is_simulating());
    }

    #[test]
    fn allows_scene_changes_only_edit() {
        assert!(EditorMode::Edit.allows_scene_changes());
        assert!(!EditorMode::Play.allows_scene_changes());
        assert!(!EditorMode::Paused.allows_scene_changes());
    }
}

// ============================================================================
// REMEDIATION 8: runtime — RuntimeState transitions
// ============================================================================

mod runtime_state_transitions {
    use super::*;

    #[test]
    fn editing_can_only_go_to_playing() {
        assert!(RuntimeState::Editing.can_transition_to(RuntimeState::Playing));
        assert!(!RuntimeState::Editing.can_transition_to(RuntimeState::Paused));
        assert!(!RuntimeState::Editing.can_transition_to(RuntimeState::SteppingOneFrame));
    }

    #[test]
    fn playing_can_go_to_paused_and_editing() {
        assert!(RuntimeState::Playing.can_transition_to(RuntimeState::Paused));
        assert!(RuntimeState::Playing.can_transition_to(RuntimeState::Editing));
        assert!(RuntimeState::Playing.can_transition_to(RuntimeState::SteppingOneFrame));
        assert!(!RuntimeState::Playing.can_transition_to(RuntimeState::Playing));
    }

    #[test]
    fn paused_transitions() {
        assert!(RuntimeState::Paused.can_transition_to(RuntimeState::Playing));
        assert!(RuntimeState::Paused.can_transition_to(RuntimeState::Editing));
        assert!(RuntimeState::Paused.can_transition_to(RuntimeState::SteppingOneFrame));
        assert!(!RuntimeState::Paused.can_transition_to(RuntimeState::Paused));
    }

    #[test]
    fn has_simulation_exact() {
        assert!(!RuntimeState::Editing.has_simulation());
        assert!(RuntimeState::Playing.has_simulation());
        assert!(RuntimeState::Paused.has_simulation());
        assert!(RuntimeState::SteppingOneFrame.has_simulation());
    }

    #[test]
    fn is_editable_only_editing() {
        assert!(RuntimeState::Editing.is_editable());
        assert!(!RuntimeState::Playing.is_editable());
        assert!(!RuntimeState::Paused.is_editable());
        assert!(!RuntimeState::SteppingOneFrame.is_editable());
    }

    #[test]
    fn is_active_exact() {
        assert!(!RuntimeState::Editing.is_active());
        assert!(RuntimeState::Playing.is_active());
        assert!(!RuntimeState::Paused.is_active());
        assert!(RuntimeState::SteppingOneFrame.is_active());
    }

    #[test]
    fn all_returns_4_variants() {
        assert_eq!(RuntimeState::all().len(), 4);
    }
}

// ============================================================================
// REMEDIATION 9: runtime — RuntimeStats performance_grade boundaries
// ============================================================================

mod runtime_stats_exact {
    use super::*;

    #[test]
    fn performance_grade_boundaries() {
        let mut stats = RuntimeStats::default();

        stats.fps = 60.0;
        assert_eq!(stats.performance_grade(), "Excellent");

        stats.fps = 59.0;
        assert_eq!(stats.performance_grade(), "Good");

        stats.fps = 45.0;
        assert_eq!(stats.performance_grade(), "Good");

        stats.fps = 44.0;
        assert_eq!(stats.performance_grade(), "Fair");

        stats.fps = 30.0;
        assert_eq!(stats.performance_grade(), "Fair");

        stats.fps = 29.0;
        assert_eq!(stats.performance_grade(), "Poor");

        stats.fps = 15.0;
        assert_eq!(stats.performance_grade(), "Poor");

        stats.fps = 14.0;
        assert_eq!(stats.performance_grade(), "Critical");

        stats.fps = 0.0;
        assert_eq!(stats.performance_grade(), "Critical");
    }

    #[test]
    fn is_frame_time_healthy_boundary() {
        let mut stats = RuntimeStats::default();

        stats.frame_time_ms = 16.67;
        assert!(stats.is_frame_time_healthy(16.67));

        stats.frame_time_ms = 16.68;
        assert!(!stats.is_frame_time_healthy(16.67));
    }

    #[test]
    fn is_fps_healthy_boundary() {
        let mut stats = RuntimeStats::default();

        stats.fps = 60.0;
        assert!(stats.is_fps_healthy(60.0));

        stats.fps = 59.9;
        assert!(!stats.is_fps_healthy(60.0));
    }

    #[test]
    fn frame_budget_percentage_exact() {
        let mut stats = RuntimeStats::default();
        stats.frame_time_ms = 16.667;
        let pct = stats.frame_budget_percentage();
        assert!(
            (pct - 100.0).abs() < 0.1,
            "16.667ms should be ~100%, got {}",
            pct
        );
    }

    #[test]
    fn is_running_smoothly_exact() {
        let mut stats = RuntimeStats::default();
        stats.fps = 60.0;
        stats.frame_time_ms = 16.67;
        assert!(stats.is_running_smoothly());

        stats.fps = 59.0;
        assert!(
            !stats.is_running_smoothly(),
            "fps < 60 should not be smooth"
        );

        stats.fps = 60.0;
        stats.frame_time_ms = 17.0;
        assert!(
            !stats.is_running_smoothly(),
            "frame_time > 16.67 should not be smooth"
        );
    }
}

// ============================================================================
// REMEDIATION 10: runtime — EditorRuntime new and fixed_dt
// ============================================================================

mod editor_runtime_exact {
    use super::*;

    #[test]
    fn new_runtime_state() {
        let rt = EditorRuntime::new();
        assert_eq!(rt.state(), RuntimeState::Editing);
        assert_eq!(rt.tick_count(), 0);
        assert!(!rt.is_playing());
        assert!(!rt.is_paused());
        assert!(!rt.has_edit_snapshot());
    }

    #[test]
    fn fixed_dt_is_60hz() {
        let rt = EditorRuntime::new();
        let expected = 1.0 / 60.0;
        assert!(
            (rt.fixed_dt() - expected).abs() < 1e-6,
            "fixed_dt should be 1/60"
        );
    }
}

// ============================================================================
// REMEDIATION 11: clipboard — Schema version and validation
// ============================================================================

mod clipboard_exact {
    use super::*;

    #[test]
    fn schema_version_is_2() {
        assert_eq!(CLIPBOARD_SCHEMA_VERSION, 2);
    }

    #[test]
    fn entity_validate_empty_name_warns() {
        let entity = ClipboardEntityData {
            name: String::new(),
            pos: IVec2::new(0, 0),
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 0,
            cooldowns: Default::default(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        let v = entity.validate();
        assert!(v.has_warnings(), "Empty name should warn");
    }

    #[test]
    fn entity_validate_name_too_long() {
        let entity = ClipboardEntityData {
            name: "x".repeat(257),
            pos: IVec2::new(0, 0),
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 0,
            cooldowns: Default::default(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        let v = entity.validate();
        assert!(v.has_errors(), "Name > 256 should be error");
    }

    #[test]
    fn entity_validate_zero_scale_error() {
        let entity = ClipboardEntityData {
            name: "test".to_string(),
            pos: IVec2::new(0, 0),
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 0.0,
            hp: 100,
            team_id: 0,
            ammo: 0,
            cooldowns: Default::default(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        let v = entity.validate();
        assert!(v.has_errors(), "scale <= 0 should be error");
    }

    #[test]
    fn entity_validate_negative_hp_warns() {
        let entity = ClipboardEntityData {
            name: "test".to_string(),
            pos: IVec2::new(0, 0),
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: -1,
            team_id: 0,
            ammo: 0,
            cooldowns: Default::default(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        let v = entity.validate();
        assert!(v.has_warnings(), "hp < 0 should warn");
    }

    #[test]
    fn is_scaled_boundary() {
        let mut entity = ClipboardEntityData {
            name: "test".to_string(),
            pos: IVec2::new(0, 0),
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 0,
            cooldowns: Default::default(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert!(!entity.is_scaled(), "scale=1.0 should not be scaled");

        entity.scale = 1.002;
        assert!(
            entity.is_scaled(),
            "scale=1.002 should be scaled (> 0.001 from 1.0)"
        );

        entity.scale = 1.0005;
        assert!(
            !entity.is_scaled(),
            "scale=1.0005 should not be scaled (within 0.001)"
        );
    }

    #[test]
    fn is_rotated_boundary() {
        let mut entity = ClipboardEntityData {
            name: "test".to_string(),
            pos: IVec2::new(0, 0),
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale: 1.0,
            hp: 100,
            team_id: 0,
            ammo: 0,
            cooldowns: Default::default(),
            behavior_graph: None,
            height: 0.0,
            parent: None,
        };
        assert!(!entity.is_rotated(), "zero rotation → not rotated");

        entity.rotation = 0.002;
        assert!(entity.is_rotated(), "rotation=0.002 → rotated");

        entity.rotation = 0.0005;
        assert!(!entity.is_rotated(), "rotation=0.0005 → within tolerance");
    }

    #[test]
    fn clipboard_data_is_compatible() {
        let data = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities: vec![],
        };
        assert!(data.is_compatible());

        let future = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION + 1,
            entities: vec![],
        };
        assert!(
            !future.is_compatible(),
            "Future version should not be compatible"
        );

        let old = ClipboardData {
            version: 1,
            entities: vec![],
        };
        assert!(old.is_compatible(), "Old version should be compatible");
    }

    #[test]
    fn clipboard_validation_add_error_sets_invalid() {
        let mut v = ClipboardValidation::valid();
        assert!(v.is_valid);
        v.add_error("boom");
        assert!(!v.is_valid);
        assert!(v.has_errors());
    }

    #[test]
    fn clipboard_stats_div_by_zero_guards() {
        let data = ClipboardData {
            version: CLIPBOARD_SCHEMA_VERSION,
            entities: vec![],
        };
        let stats = data.stats();
        assert_eq!(stats.ai_percentage(), 0.0);
        assert_eq!(stats.avg_cooldowns_per_entity(), 0.0);
    }
}

// ============================================================================
// REMEDIATION 12: plugin — PluginEvent classification
// ============================================================================

mod plugin_event_exact {
    use super::*;

    #[test]
    fn is_lifecycle_only_loaded_unloading() {
        assert!(PluginEvent::Loaded.is_lifecycle());
        assert!(PluginEvent::Unloading.is_lifecycle());
        assert!(!PluginEvent::Update.is_lifecycle());
        assert!(!PluginEvent::SceneLoaded.is_lifecycle());
        assert!(!PluginEvent::EntitySelected.is_lifecycle());
        assert!(!PluginEvent::PlayModeEnter.is_lifecycle());
    }

    #[test]
    fn is_scene_event_only_scene_loaded_saved() {
        assert!(PluginEvent::SceneLoaded.is_scene_event());
        assert!(PluginEvent::SceneSaved.is_scene_event());
        assert!(!PluginEvent::Loaded.is_scene_event());
        assert!(!PluginEvent::Update.is_scene_event());
    }

    #[test]
    fn is_entity_event_only_entity_types() {
        assert!(PluginEvent::EntitySelected.is_entity_event());
        assert!(PluginEvent::EntityCreated.is_entity_event());
        assert!(!PluginEvent::SceneLoaded.is_entity_event());
        assert!(!PluginEvent::Update.is_entity_event());
    }

    #[test]
    fn is_play_mode_event() {
        assert!(PluginEvent::PlayModeEnter.is_play_mode_event());
        assert!(PluginEvent::PlayModeExit.is_play_mode_event());
        assert!(!PluginEvent::Update.is_play_mode_event());
        assert!(!PluginEvent::Loaded.is_play_mode_event());
    }

    #[test]
    fn is_frequent_only_update() {
        assert!(PluginEvent::Update.is_frequent());
        assert!(!PluginEvent::Loaded.is_frequent());
        assert!(!PluginEvent::SceneLoaded.is_frequent());
    }

    #[test]
    fn default_is_loaded() {
        assert_eq!(PluginEvent::default(), PluginEvent::Loaded);
    }
}

// ============================================================================
// REMEDIATION 13: plugin — PluginState and PluginError classification
// ============================================================================

mod plugin_state_exact {
    use super::*;

    #[test]
    fn is_operational_only_active() {
        assert!(PluginState::Active.is_operational());
        assert!(!PluginState::Loaded.is_operational());
        assert!(!PluginState::Error.is_operational());
        assert!(!PluginState::Disabled.is_operational());
    }

    #[test]
    fn all_returns_4() {
        assert_eq!(PluginState::all().len(), 4);
    }

    #[test]
    fn plugin_error_is_fatal() {
        assert!(PluginError::InitFailed("".into()).is_fatal());
        assert!(PluginError::IncompatibleVersion {
            required: "".into(),
            actual: "".into()
        }
        .is_fatal());
        assert!(PluginError::MissingDependency("".into()).is_fatal());
        assert!(!PluginError::ConfigError("".into()).is_fatal());
        assert!(!PluginError::Other("".into()).is_fatal());
    }

    #[test]
    fn health_percentage_zero_total_is_100() {
        let stats = PluginManagerStats::default();
        assert_eq!(
            stats.health_percentage(),
            100.0,
            "Zero plugins → 100% health"
        );
    }

    #[test]
    fn all_active_requires_nonzero_total() {
        let stats = PluginManagerStats::default();
        assert!(
            !stats.all_active(),
            "Zero plugins → not all_active (need total>0)"
        );
    }
}

// ============================================================================
// REMEDIATION 14: dock_layout — LayoutPreset exact panel counts
// ============================================================================

mod layout_preset_exact {
    use super::*;

    #[test]
    fn expected_panel_count_per_preset() {
        assert_eq!(LayoutPreset::Default.expected_panel_count(), 6);
        assert_eq!(LayoutPreset::Wide.expected_panel_count(), 2);
        assert_eq!(LayoutPreset::Compact.expected_panel_count(), 8);
        assert_eq!(LayoutPreset::Modeling.expected_panel_count(), 3);
        assert_eq!(LayoutPreset::Animation.expected_panel_count(), 5);
        assert_eq!(LayoutPreset::Debug.expected_panel_count(), 5);
    }

    #[test]
    fn all_returns_6_presets() {
        assert_eq!(LayoutPreset::all().len(), 6);
    }

    #[test]
    fn is_debug_layout_only_debug() {
        assert!(LayoutPreset::Debug.is_debug_layout());
        assert!(!LayoutPreset::Default.is_debug_layout());
        assert!(!LayoutPreset::Wide.is_debug_layout());
    }

    #[test]
    fn is_content_creation_layout() {
        assert!(LayoutPreset::Modeling.is_content_creation_layout());
        assert!(LayoutPreset::Animation.is_content_creation_layout());
        assert!(!LayoutPreset::Default.is_content_creation_layout());
        assert!(!LayoutPreset::Debug.is_content_creation_layout());
    }

    #[test]
    fn shortcut_hints_all_present() {
        for preset in LayoutPreset::all() {
            assert!(
                preset.shortcut_hint().is_some(),
                "{:?} should have shortcut",
                preset
            );
        }
    }

    #[test]
    fn names_unique() {
        let names: Vec<&str> = LayoutPreset::all().iter().map(|p| p.name()).collect();
        let mut deduped = names.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len());
    }
}

// ============================================================================
// REMEDIATION 15: prefab — EntityOverrides counting
// ============================================================================

mod prefab_overrides_exact {
    use super::*;

    #[test]
    fn override_count_zero_default() {
        let ov = EntityOverrides::default();
        assert_eq!(ov.override_count(), 0);
        assert!(!ov.has_any_override());
        assert!(!ov.has_pose_override());
        assert!(!ov.has_health_override());
    }

    #[test]
    fn override_count_max_4() {
        let ov = EntityOverrides {
            pos_x: Some(1),
            pos_y: Some(2),
            health: Some(100),
            max_health: Some(200),
        };
        assert_eq!(ov.override_count(), 4);
        assert!(ov.has_any_override());
        assert!(ov.has_pose_override());
        assert!(ov.has_health_override());
    }

    #[test]
    fn has_pose_override_checks_both() {
        let ov = EntityOverrides {
            pos_x: Some(1),
            ..Default::default()
        };
        assert!(ov.has_pose_override(), "pos_x alone → has pose override");

        let ov = EntityOverrides {
            pos_y: Some(1),
            ..Default::default()
        };
        assert!(ov.has_pose_override(), "pos_y alone → has pose override");
    }

    #[test]
    fn has_health_override_checks_both() {
        let ov = EntityOverrides {
            health: Some(50),
            ..Default::default()
        };
        assert!(ov.has_health_override());

        let ov = EntityOverrides {
            max_health: Some(200),
            ..Default::default()
        };
        assert!(ov.has_health_override());
    }

    #[test]
    fn hierarchy_snapshot_default_empty() {
        let h = PrefabHierarchySnapshot::new();
        // Just verify it creates without panic
        let children = h.children_of(999_u32);
        assert!(children.is_empty(), "No children for nonexistent parent");
    }
}

// ============================================================================
// REMEDIATION 16: dock_layout — DockLayout validity
// ============================================================================

mod dock_layout_exact {
    use super::*;

    #[test]
    fn new_layout_is_valid() {
        let layout = DockLayout::new();
        assert!(
            layout.is_valid(),
            "Default layout should be valid (has viewport)"
        );
    }

    #[test]
    fn panel_count_nonzero_default() {
        let layout = DockLayout::new();
        assert!(
            layout.panel_count() > 0,
            "Default layout should have panels"
        );
    }

    #[test]
    fn from_preset_matches_preset() {
        for &preset in LayoutPreset::all() {
            let layout = DockLayout::from_preset(preset);
            assert!(
                layout.matches_preset(preset),
                "{:?} preset layout should match its own preset",
                preset
            );
        }
    }
}
