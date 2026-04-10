//! Mutation-resistant comprehensive tests for aw_editor.
//!
//! Targets pure-logic modules: EditorMode, RuntimeState, RuntimeIssue,
//! GizmoMode, AxisConstraint, SnappingConfig, PanelType, PanelCategory,
//! and more — exact values, boundary conditions, state transitions.

use aw_editor_lib::editor_mode::EditorMode;
use aw_editor_lib::gizmo::snapping::SnappingConfig;
use aw_editor_lib::gizmo::state::{AxisConstraint, GizmoMode, GizmoState, TransformSnapshot};
use aw_editor_lib::runtime::{RuntimeIssue, RuntimeState};
use glam::{Vec2, Vec3};

// =========================================================================
// EditorMode — state predicates + transitions
// =========================================================================

#[test]
fn editor_mode_default_is_edit() {
    assert_eq!(EditorMode::default(), EditorMode::Edit);
}

#[test]
fn editor_mode_edit_predicates() {
    let m = EditorMode::Edit;
    assert!(m.is_editing());
    assert!(!m.is_playing());
    assert!(!m.is_paused());
    assert!(m.can_edit());
    assert!(m.allows_scene_changes());
    assert!(!m.is_simulating());
}

#[test]
fn editor_mode_play_predicates() {
    let m = EditorMode::Play;
    assert!(!m.is_editing());
    assert!(m.is_playing());
    assert!(!m.is_paused());
    assert!(!m.can_edit());
    assert!(!m.allows_scene_changes());
    assert!(m.is_simulating());
}

#[test]
fn editor_mode_paused_predicates() {
    let m = EditorMode::Paused;
    assert!(!m.is_editing());
    assert!(!m.is_playing());
    assert!(m.is_paused());
    assert!(!m.can_edit());
    assert!(!m.allows_scene_changes());
    assert!(!m.is_simulating());
}

#[test]
fn editor_mode_status_text_exact() {
    assert_eq!(EditorMode::Edit.status_text(), "Edit Mode");
    assert_eq!(EditorMode::Play.status_text(), "▶️ Playing");
    assert_eq!(EditorMode::Paused.status_text(), "⏸️ Paused");
}

#[test]
fn editor_mode_shortcut_hint_exact() {
    assert_eq!(EditorMode::Edit.shortcut_hint(), "Esc");
    assert_eq!(EditorMode::Play.shortcut_hint(), "F5");
    assert_eq!(EditorMode::Paused.shortcut_hint(), "F6");
}

#[test]
fn editor_mode_icon_exact() {
    assert_eq!(EditorMode::Edit.icon(), "🔧");
    assert_eq!(EditorMode::Play.icon(), "▶️");
    assert_eq!(EditorMode::Paused.icon(), "⏸️");
}

#[test]
fn editor_mode_all_has_three() {
    assert_eq!(EditorMode::all().len(), 3);
}

#[test]
fn editor_mode_transition_edit_to_play() {
    assert!(EditorMode::Edit.can_transition_to(EditorMode::Play));
}

#[test]
fn editor_mode_transition_edit_to_paused_invalid() {
    assert!(!EditorMode::Edit.can_transition_to(EditorMode::Paused));
}

#[test]
fn editor_mode_transition_play_to_edit() {
    assert!(EditorMode::Play.can_transition_to(EditorMode::Edit));
}

#[test]
fn editor_mode_transition_play_to_paused() {
    assert!(EditorMode::Play.can_transition_to(EditorMode::Paused));
}

#[test]
fn editor_mode_transition_paused_to_edit() {
    assert!(EditorMode::Paused.can_transition_to(EditorMode::Edit));
}

#[test]
fn editor_mode_transition_paused_to_play() {
    assert!(EditorMode::Paused.can_transition_to(EditorMode::Play));
}

#[test]
fn editor_mode_valid_transitions_edit() {
    let vt = EditorMode::Edit.valid_transitions();
    assert_eq!(vt, vec![EditorMode::Play]);
}

#[test]
fn editor_mode_valid_transitions_play() {
    let vt = EditorMode::Play.valid_transitions();
    assert!(vt.contains(&EditorMode::Edit));
    assert!(vt.contains(&EditorMode::Paused));
    assert_eq!(vt.len(), 2);
}

#[test]
fn editor_mode_next_mode() {
    assert_eq!(EditorMode::Edit.next_mode(), EditorMode::Play);
    assert_eq!(EditorMode::Play.next_mode(), EditorMode::Paused);
    assert_eq!(EditorMode::Paused.next_mode(), EditorMode::Play);
}

#[test]
fn editor_mode_action_verb_exact() {
    assert_eq!(EditorMode::Edit.action_verb(), "Stop");
    assert_eq!(EditorMode::Play.action_verb(), "Play");
    assert_eq!(EditorMode::Paused.action_verb(), "Pause");
}

#[test]
fn editor_mode_description_not_empty() {
    for m in EditorMode::all() {
        assert!(
            !m.description().is_empty(),
            "mode {:?} has empty description",
            m
        );
    }
}

#[test]
fn editor_mode_display_matches_status_text() {
    for m in EditorMode::all() {
        assert_eq!(format!("{}", m), m.status_text());
    }
}

#[test]
fn editor_mode_self_transition_allowed() {
    for m in EditorMode::all() {
        assert!(
            m.can_transition_to(m),
            "{:?} → {:?} should be allowed",
            m,
            m
        );
    }
}

// =========================================================================
// RuntimeState — state machine
// =========================================================================

#[test]
fn runtime_state_all_has_four() {
    assert_eq!(RuntimeState::all().len(), 4);
}

#[test]
fn runtime_state_editing_predicates() {
    let s = RuntimeState::Editing;
    assert!(s.is_editable());
    assert!(!s.has_simulation());
    assert!(!s.is_active());
}

#[test]
fn runtime_state_playing_predicates() {
    let s = RuntimeState::Playing;
    assert!(!s.is_editable());
    assert!(s.has_simulation());
    assert!(s.is_active());
}

#[test]
fn runtime_state_paused_predicates() {
    let s = RuntimeState::Paused;
    assert!(!s.is_editable());
    assert!(s.has_simulation());
    assert!(!s.is_active());
}

#[test]
fn runtime_state_stepping_predicates() {
    let s = RuntimeState::SteppingOneFrame;
    assert!(!s.is_editable());
    assert!(s.has_simulation());
    assert!(s.is_active());
}

#[test]
fn runtime_state_editing_to_playing() {
    assert!(RuntimeState::Editing.can_transition_to(RuntimeState::Playing));
}

#[test]
fn runtime_state_editing_to_paused_invalid() {
    assert!(!RuntimeState::Editing.can_transition_to(RuntimeState::Paused));
}

#[test]
fn runtime_state_playing_not_to_playing() {
    assert!(!RuntimeState::Playing.can_transition_to(RuntimeState::Playing));
}

#[test]
fn runtime_state_paused_not_to_paused() {
    assert!(!RuntimeState::Paused.can_transition_to(RuntimeState::Paused));
}

#[test]
fn runtime_state_stepping_to_all() {
    assert!(RuntimeState::SteppingOneFrame.can_transition_to(RuntimeState::Paused));
    assert!(RuntimeState::SteppingOneFrame.can_transition_to(RuntimeState::Editing));
    assert!(RuntimeState::SteppingOneFrame.can_transition_to(RuntimeState::Playing));
    assert!(RuntimeState::SteppingOneFrame.can_transition_to(RuntimeState::SteppingOneFrame));
}

#[test]
fn runtime_state_icon_non_empty() {
    for s in RuntimeState::all() {
        assert!(!s.icon().is_empty());
    }
}

#[test]
fn runtime_state_display_exact() {
    assert_eq!(format!("{}", RuntimeState::Editing), "Editing");
    assert_eq!(format!("{}", RuntimeState::Playing), "Playing");
    assert_eq!(format!("{}", RuntimeState::Paused), "Paused");
    assert_eq!(format!("{}", RuntimeState::SteppingOneFrame), "Stepping");
}

#[test]
fn runtime_state_description_non_empty() {
    for s in RuntimeState::all() {
        assert!(!s.description().is_empty());
    }
}

#[test]
fn runtime_state_shortcut_hint_non_empty() {
    for s in RuntimeState::all() {
        assert!(!s.shortcut_hint().is_empty());
    }
}

// =========================================================================
// RuntimeIssue — issue classification
// =========================================================================

#[test]
fn runtime_issue_missing_simulation_is_critical() {
    assert!(RuntimeIssue::MissingSimulation.is_critical());
    assert!(!RuntimeIssue::MissingSimulation.is_recoverable());
}

#[test]
fn runtime_issue_corrupted_is_critical() {
    let issue = RuntimeIssue::CorruptedSimulation { reason: "x".into() };
    assert!(issue.is_critical());
    assert!(issue.is_data_issue());
    assert!(!issue.is_recoverable());
}

#[test]
fn runtime_issue_frame_time_is_performance() {
    let issue = RuntimeIssue::FrameTimeExceeded {
        frame_time_ms: 50,
        threshold_ms: 33,
    };
    assert!(issue.is_performance_issue());
    assert!(!issue.is_critical());
    assert!(issue.is_recoverable());
}

#[test]
fn runtime_issue_low_fps_is_performance() {
    let issue = RuntimeIssue::LowFps {
        fps: 15,
        minimum_fps: 30,
    };
    assert!(issue.is_performance_issue());
    assert!(issue.is_recoverable());
}

#[test]
fn runtime_issue_entity_mismatch_is_data() {
    let issue = RuntimeIssue::EntityCountMismatch {
        expected: 100,
        actual: 95,
    };
    assert!(issue.is_data_issue());
    assert!(issue.is_recoverable());
    assert!(!issue.is_critical());
}

#[test]
fn runtime_issue_missing_snapshot_is_data() {
    assert!(RuntimeIssue::MissingEditSnapshot.is_data_issue());
    assert!(!RuntimeIssue::MissingEditSnapshot.is_recoverable());
}

#[test]
fn runtime_issue_all_variants_count() {
    assert_eq!(RuntimeIssue::all_variants().len(), 6);
}

// =========================================================================
// GizmoMode — operation types
// =========================================================================

#[test]
fn gizmo_mode_default_inactive() {
    assert_eq!(GizmoMode::default(), GizmoMode::Inactive);
}

#[test]
fn gizmo_mode_inactive_not_active() {
    assert!(!GizmoMode::Inactive.is_active());
    assert!(!GizmoMode::Inactive.is_translate());
    assert!(!GizmoMode::Inactive.is_rotate());
    assert!(!GizmoMode::Inactive.is_scale());
}

#[test]
fn gizmo_mode_translate_is_active() {
    let m = GizmoMode::Translate {
        constraint: AxisConstraint::None,
    };
    assert!(m.is_active());
    assert!(m.is_translate());
    assert!(!m.is_rotate());
    assert!(!m.is_scale());
}

#[test]
fn gizmo_mode_rotate_is_active() {
    let m = GizmoMode::Rotate {
        constraint: AxisConstraint::None,
    };
    assert!(m.is_active());
    assert!(m.is_rotate());
}

#[test]
fn gizmo_mode_scale_is_active() {
    let m = GizmoMode::Scale {
        constraint: AxisConstraint::None,
        uniform: false,
    };
    assert!(m.is_active());
    assert!(m.is_scale());
}

#[test]
fn gizmo_mode_name_exact() {
    assert_eq!(GizmoMode::Inactive.name(), "Inactive");
    assert_eq!(
        GizmoMode::Translate {
            constraint: AxisConstraint::X
        }
        .name(),
        "Translate"
    );
    assert_eq!(
        GizmoMode::Rotate {
            constraint: AxisConstraint::Y
        }
        .name(),
        "Rotate"
    );
    assert_eq!(
        GizmoMode::Scale {
            constraint: AxisConstraint::Z,
            uniform: true
        }
        .name(),
        "Scale"
    );
}

#[test]
fn gizmo_mode_icon_exact() {
    assert_eq!(GizmoMode::Inactive.icon(), "||");
    assert_eq!(
        GizmoMode::Translate {
            constraint: AxisConstraint::None
        }
        .icon(),
        "↔"
    );
    assert_eq!(
        GizmoMode::Rotate {
            constraint: AxisConstraint::None
        }
        .icon(),
        "↻"
    );
    assert_eq!(
        GizmoMode::Scale {
            constraint: AxisConstraint::None,
            uniform: false
        }
        .icon(),
        "⇲"
    );
}

#[test]
fn gizmo_mode_shortcut() {
    assert_eq!(GizmoMode::Inactive.shortcut(), None);
    assert_eq!(
        GizmoMode::Translate {
            constraint: AxisConstraint::None
        }
        .shortcut(),
        Some("G")
    );
    assert_eq!(
        GizmoMode::Rotate {
            constraint: AxisConstraint::None
        }
        .shortcut(),
        Some("R")
    );
    assert_eq!(
        GizmoMode::Scale {
            constraint: AxisConstraint::None,
            uniform: false
        }
        .shortcut(),
        Some("S")
    );
}

#[test]
fn gizmo_mode_constraint_inactive_none() {
    assert_eq!(GizmoMode::Inactive.constraint(), None);
}

#[test]
fn gizmo_mode_constraint_translate_x() {
    let m = GizmoMode::Translate {
        constraint: AxisConstraint::X,
    };
    assert_eq!(m.constraint(), Some(AxisConstraint::X));
}

#[test]
fn gizmo_mode_all_count() {
    assert_eq!(GizmoMode::all().len(), 4);
}

#[test]
fn gizmo_mode_display_inactive() {
    assert_eq!(format!("{}", GizmoMode::Inactive), "Inactive");
}

#[test]
fn gizmo_mode_display_translate_free() {
    let s = format!(
        "{}",
        GizmoMode::Translate {
            constraint: AxisConstraint::None
        }
    );
    assert!(s.contains("Translate"), "display={}", s);
    assert!(s.contains("Free"), "display={}", s);
}

// =========================================================================
// AxisConstraint — constraint types + math
// =========================================================================

#[test]
fn axis_constraint_default_is_none() {
    assert_eq!(AxisConstraint::default(), AxisConstraint::None);
}

#[test]
fn axis_constraint_all_count() {
    assert_eq!(AxisConstraint::all().len(), 7);
}

#[test]
fn axis_constraint_name_exact() {
    assert_eq!(AxisConstraint::None.name(), "Free");
    assert_eq!(AxisConstraint::X.name(), "X Axis");
    assert_eq!(AxisConstraint::Y.name(), "Y Axis");
    assert_eq!(AxisConstraint::Z.name(), "Z Axis");
    assert_eq!(AxisConstraint::XY.name(), "XY Plane");
    assert_eq!(AxisConstraint::XZ.name(), "XZ Plane");
    assert_eq!(AxisConstraint::YZ.name(), "YZ Plane");
}

#[test]
fn axis_constraint_key() {
    assert_eq!(AxisConstraint::None.key(), None);
    assert_eq!(AxisConstraint::X.key(), Some("X"));
    assert_eq!(AxisConstraint::Y.key(), Some("Y"));
    assert_eq!(AxisConstraint::Z.key(), Some("Z"));
    assert_eq!(AxisConstraint::XY.key(), None);
}

#[test]
fn axis_constraint_axis_vector_none_is_one() {
    assert_eq!(AxisConstraint::None.axis_vector(), Vec3::ONE);
}

#[test]
fn axis_constraint_axis_vector_x() {
    assert_eq!(AxisConstraint::X.axis_vector(), Vec3::X);
}

#[test]
fn axis_constraint_axis_vector_y() {
    assert_eq!(AxisConstraint::Y.axis_vector(), Vec3::Y);
}

#[test]
fn axis_constraint_axis_vector_z() {
    assert_eq!(AxisConstraint::Z.axis_vector(), Vec3::Z);
}

#[test]
fn axis_constraint_axis_vector_planar_xy() {
    assert_eq!(AxisConstraint::XY.axis_vector(), Vec3::new(1.0, 1.0, 0.0));
}

#[test]
fn axis_constraint_axis_vector_planar_xz() {
    assert_eq!(AxisConstraint::XZ.axis_vector(), Vec3::new(1.0, 0.0, 1.0));
}

#[test]
fn axis_constraint_axis_vector_planar_yz() {
    assert_eq!(AxisConstraint::YZ.axis_vector(), Vec3::new(0.0, 1.0, 1.0));
}

#[test]
fn axis_constraint_is_planar() {
    assert!(!AxisConstraint::None.is_planar());
    assert!(!AxisConstraint::X.is_planar());
    assert!(AxisConstraint::XY.is_planar());
    assert!(AxisConstraint::XZ.is_planar());
    assert!(AxisConstraint::YZ.is_planar());
}

#[test]
fn axis_constraint_is_single_axis() {
    assert!(!AxisConstraint::None.is_single_axis());
    assert!(AxisConstraint::X.is_single_axis());
    assert!(AxisConstraint::Y.is_single_axis());
    assert!(AxisConstraint::Z.is_single_axis());
    assert!(!AxisConstraint::XY.is_single_axis());
}

#[test]
fn axis_constraint_cycle_none_to_x() {
    assert_eq!(
        AxisConstraint::None.cycle(AxisConstraint::X),
        AxisConstraint::X
    );
}

#[test]
fn axis_constraint_cycle_x_to_yz_plane() {
    assert_eq!(
        AxisConstraint::X.cycle(AxisConstraint::X),
        AxisConstraint::YZ
    );
}

#[test]
fn axis_constraint_cycle_y_to_xz_plane() {
    assert_eq!(
        AxisConstraint::Y.cycle(AxisConstraint::Y),
        AxisConstraint::XZ
    );
}

#[test]
fn axis_constraint_cycle_z_to_xy_plane() {
    assert_eq!(
        AxisConstraint::Z.cycle(AxisConstraint::Z),
        AxisConstraint::XY
    );
}

#[test]
fn axis_constraint_cycle_yz_back_to_none() {
    assert_eq!(
        AxisConstraint::YZ.cycle(AxisConstraint::X),
        AxisConstraint::None
    );
}

#[test]
fn axis_constraint_color_none_white() {
    assert_eq!(AxisConstraint::None.color(), [1.0, 1.0, 1.0]);
}

#[test]
fn axis_constraint_color_x_red() {
    let c = AxisConstraint::X.color();
    assert!(c[0] > c[1] && c[0] > c[2], "X should be red-ish: {:?}", c);
}

#[test]
fn axis_constraint_color_y_green() {
    let c = AxisConstraint::Y.color();
    assert!(c[1] > c[0] && c[1] > c[2], "Y should be green-ish: {:?}", c);
}

#[test]
fn axis_constraint_color_z_blue() {
    let c = AxisConstraint::Z.color();
    assert!(c[2] > c[0] && c[2] > c[1], "Z should be blue-ish: {:?}", c);
}

#[test]
fn axis_constraint_display_free() {
    assert_eq!(format!("{}", AxisConstraint::None), "Free");
}

#[test]
fn axis_constraint_display_x() {
    assert_eq!(format!("{}", AxisConstraint::X), "X");
}

// =========================================================================
// SnappingConfig — pure math
// =========================================================================

#[test]
fn snapping_config_defaults() {
    let c = SnappingConfig::default();
    assert_eq!(c.grid_size, 1.0);
    assert_eq!(c.angle_increment, 15.0);
    assert!(c.grid_enabled);
    assert!(c.angle_enabled);
}

#[test]
fn snapping_config_new_same_as_default() {
    assert_eq!(SnappingConfig::new(), SnappingConfig::default());
}

#[test]
fn snapping_config_with_grid_size() {
    let c = SnappingConfig::new().with_grid_size(0.5);
    assert_eq!(c.grid_size, 0.5);
}

#[test]
fn snapping_config_with_angle_increment() {
    let c = SnappingConfig::new().with_angle_increment(45.0);
    assert_eq!(c.angle_increment, 45.0);
}

#[test]
fn snapping_snap_position_rounds_to_grid() {
    let c = SnappingConfig::default();
    let snapped = c.snap_position(Vec3::new(1.7, 2.3, -0.4));
    assert_eq!(snapped, Vec3::new(2.0, 2.0, 0.0));
}

#[test]
fn snapping_snap_position_half_grid() {
    let c = SnappingConfig::new().with_grid_size(0.5);
    let snapped = c.snap_position(Vec3::new(1.7, 2.3, -0.4));
    assert_eq!(snapped, Vec3::new(1.5, 2.5, -0.5));
}

#[test]
fn snapping_snap_position_disabled_no_change() {
    let c = SnappingConfig {
        grid_enabled: false,
        ..Default::default()
    };
    let pos = Vec3::new(1.7, 2.3, -0.4);
    assert_eq!(c.snap_position(pos), pos);
}

#[test]
fn snapping_snap_angle_rounds_to_15_deg() {
    let c = SnappingConfig::default();
    let angle = 23.0_f32.to_radians();
    let snapped = c.snap_angle(angle);
    // Rounds to 15° or 30° → closest to 23 is 30°
    assert!((snapped - 30.0_f32.to_radians()).abs() < 0.001);
}

#[test]
fn snapping_snap_angle_disabled_no_change() {
    let c = SnappingConfig {
        angle_enabled: false,
        ..Default::default()
    };
    let angle = 23.0_f32.to_radians();
    assert_eq!(c.snap_angle(angle), angle);
}

#[test]
fn snapping_snap_position_exact_on_grid_unchanged() {
    let c = SnappingConfig::default();
    let pos = Vec3::new(3.0, 5.0, -2.0);
    assert_eq!(c.snap_position(pos), pos);
}

// =========================================================================
// TransformSnapshot — defaults
// =========================================================================

#[test]
fn transform_snapshot_default_identity() {
    let t = TransformSnapshot::default();
    assert_eq!(t.position, Vec3::ZERO);
    assert_eq!(t.rotation, glam::Quat::IDENTITY);
    assert_eq!(t.scale, Vec3::ONE);
}

// =========================================================================
// GizmoState — state machine
// =========================================================================

#[test]
fn gizmo_state_default_inactive() {
    let gs = GizmoState::default();
    assert_eq!(gs.mode, GizmoMode::Inactive);
    assert!(!gs.confirmed);
    assert!(!gs.cancelled);
}

#[test]
fn gizmo_state_new_same_as_default() {
    let gs = GizmoState::new();
    assert_eq!(gs.mode, GizmoMode::Inactive);
}

#[test]
fn gizmo_state_start_translate() {
    let mut gs = GizmoState::new();
    gs.selected_entity = Some(1);
    gs.start_translate();
    assert!(gs.mode.is_translate());
    assert!(gs.is_active());
}

#[test]
fn gizmo_state_start_rotate() {
    let mut gs = GizmoState::new();
    gs.selected_entity = Some(1);
    gs.start_rotate();
    assert!(gs.mode.is_rotate());
}

#[test]
fn gizmo_state_start_scale() {
    let mut gs = GizmoState::new();
    gs.selected_entity = Some(1);
    gs.start_scale(false);
    assert!(gs.mode.is_scale());
}

#[test]
fn gizmo_state_cancel_resets_to_inactive() {
    let mut gs = GizmoState::new();
    gs.selected_entity = Some(1);
    gs.start_translate();
    gs.cancel_transform();
    assert!(gs.cancelled);
}

#[test]
fn gizmo_state_confirm() {
    let mut gs = GizmoState::new();
    gs.selected_entity = Some(1);
    gs.start_translate();
    gs.confirm_transform();
    assert!(gs.confirmed);
}

#[test]
fn gizmo_state_add_constraint() {
    let mut gs = GizmoState::new();
    gs.selected_entity = Some(1);
    gs.start_translate();
    gs.add_constraint(AxisConstraint::X);
    if let GizmoMode::Translate { constraint } = gs.mode {
        assert_eq!(constraint, AxisConstraint::X);
    } else {
        panic!("Expected Translate mode");
    }
}

#[test]
fn gizmo_state_mouse_delta_initial_zero() {
    let gs = GizmoState::new();
    assert_eq!(gs.mouse_delta(), Vec2::ZERO);
}

#[test]
fn gizmo_state_is_active_when_translating() {
    let mut gs = GizmoState::new();
    assert!(!gs.is_active());
    gs.selected_entity = Some(1);
    gs.start_translate();
    assert!(gs.is_active());
}
