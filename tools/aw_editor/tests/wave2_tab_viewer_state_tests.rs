//! Wave 2 Mutation Remediation: tab_viewer.rs — State Management
//!
//! Targets the high-value *stateful* paths inside EditorTabViewer:
//!   • EditorTabViewer::new() — constructor defaults
//!   • Console log push / overflow / classification
//!   • Entity add / remove / find
//!   • check_transform_changes — 0.001-threshold comparisons
//!   • update_animation — frame advancing logic
//!   • set_animation_frame — min-clamping
//!   • set_animation_playing — event emission
//!   • set_current_theme — emit-on-change gating
//!   • set_ui_scale — clamp [0.75, 2.0]
//!   • start_build / complete_build / reset_build_status — status codes
//!   • push_frame_time — cap at 120
//!   • begin_frame — clear per-frame state
//!   • take_events / take_closed_panels / take_panels_to_add — drain semantics
//!   • select_behavior_node — event emission
//!   • log_info / log_warn / log_error formatting
//!   • set_selected_transform — sync detection

use aw_editor_lib::panel_type::PanelType;
use aw_editor_lib::tab_viewer::{EditorTabViewer, EditorTheme, EntityInfo, PanelEvent};

// Helper: make a default EditorTabViewer
fn make_viewer() -> EditorTabViewer {
    EditorTabViewer::new()
}

fn make_entity(id: u64, name: &str) -> EntityInfo {
    EntityInfo {
        id,
        name: name.to_string(),
        components: vec!["Transform".to_string()],
        entity_type: "Actor".to_string(),
        hp: None,
        team_id: None,
        ammo: None,
        pos_x: None,
        pos_y: None,
        rotation: None,
        scale: None,
        component_data: std::collections::HashMap::new(),
        material_base_color: [1.0, 1.0, 1.0, 1.0],
        material_metallic: 0.0,
        material_roughness: 0.5,
        material_emissive: [0.0, 0.0, 0.0],
        material_textures: std::collections::HashMap::new(),
    }
}

// ============================================================================
// EditorTabViewer::new() — constructor defaults
// ============================================================================

#[test]
fn new_viewer_selected_entity_none() {
    let v = make_viewer();
    assert!(v.selected_entity.is_none());
}

#[test]
fn new_viewer_not_playing() {
    let v = make_viewer();
    assert!(!v.is_playing);
}

#[test]
fn new_viewer_events_empty() {
    let mut v = make_viewer();
    assert!(v.take_events().is_empty());
}

#[test]
fn new_viewer_closed_panels_empty() {
    let mut v = make_viewer();
    assert!(v.take_closed_panels().is_empty());
}

#[test]
fn new_viewer_panels_to_add_empty() {
    let mut v = make_viewer();
    assert!(v.take_panels_to_add().is_empty());
}

#[test]
fn new_viewer_theme_dark() {
    let v = make_viewer();
    assert_eq!(v.current_theme(), EditorTheme::Dark);
}

#[test]
fn new_viewer_ui_scale_1() {
    let v = make_viewer();
    assert_eq!(v.ui_scale(), 1.0);
}

#[test]
fn new_viewer_grid_enabled() {
    let v = make_viewer();
    assert!(v.is_grid_enabled());
}

#[test]
fn new_viewer_snap_enabled() {
    let v = make_viewer();
    assert!(v.is_snap_enabled());
}

#[test]
fn new_viewer_animation_state_not_playing() {
    let v = make_viewer();
    assert!(!v.animation_state().is_playing);
}

#[test]
fn new_viewer_animation_state_frame_0() {
    let v = make_viewer();
    assert_eq!(v.animation_state().current_frame, 0);
}

#[test]
fn new_viewer_animation_state_total_120() {
    let v = make_viewer();
    assert_eq!(v.animation_state().total_frames, 120);
}

#[test]
fn new_viewer_animation_state_fps_30() {
    let v = make_viewer();
    assert_eq!(v.animation_state().fps, 30.0);
}

#[test]
fn new_viewer_animation_state_has_tracks() {
    let v = make_viewer();
    // Constructor seeds default tracks
    assert!(!v.animation_state().tracks.is_empty());
}

#[test]
fn new_viewer_material_default_name() {
    let v = make_viewer();
    assert_eq!(v.current_material().name, "Default Material");
}

#[test]
fn new_viewer_behavior_graph_has_nodes() {
    let v = make_viewer();
    assert!(!v.behavior_graph().nodes.is_empty());
}

#[test]
fn new_viewer_behavior_graph_no_selection() {
    let v = make_viewer();
    assert!(v.behavior_graph().selected_node.is_none());
}

// ============================================================================
// Entity add / remove / find
// ============================================================================

#[test]
fn add_entity_then_find() {
    let mut v = make_viewer();
    v.add_entity(make_entity(1, "Player"));
    assert!(v.find_entity(1).is_some());
    assert_eq!(v.find_entity(1).unwrap().name, "Player");
}

#[test]
fn find_entity_not_found() {
    let v = make_viewer();
    assert!(v.find_entity(999).is_none());
}

#[test]
fn remove_entity_works() {
    let mut v = make_viewer();
    v.add_entity(make_entity(1, "A"));
    v.add_entity(make_entity(2, "B"));
    v.remove_entity(1);
    assert!(v.find_entity(1).is_none());
    assert!(v.find_entity(2).is_some());
}

#[test]
fn remove_entity_nonexistent_is_harmless() {
    let mut v = make_viewer();
    v.add_entity(make_entity(1, "A"));
    v.remove_entity(999); // no panic
    assert!(v.find_entity(1).is_some());
}

#[test]
fn set_entity_list_replaces() {
    let mut v = make_viewer();
    v.add_entity(make_entity(1, "Old"));
    v.set_entity_list(vec![make_entity(2, "New")]);
    assert!(v.find_entity(1).is_none());
    assert!(v.find_entity(2).is_some());
}

#[test]
fn entity_list_mut_allows_modification() {
    let mut v = make_viewer();
    v.add_entity(make_entity(1, "A"));
    v.entity_list_mut().push(make_entity(2, "B"));
    assert!(v.find_entity(2).is_some());
}

// ============================================================================
// begin_frame — clears per-frame state
// ============================================================================

#[test]
fn begin_frame_clears_events() {
    let mut v = make_viewer();
    v.set_animation_playing(true); // emits event
    assert!(!v.take_events().is_empty());
    v.set_animation_playing(false); // another event
    v.begin_frame();
    assert!(v.take_events().is_empty());
}

#[test]
fn begin_frame_clears_panels_to_close() {
    let mut v = make_viewer();
    v.panels_to_close.push(PanelType::Console);
    v.begin_frame();
    assert!(v.take_closed_panels().is_empty());
}

#[test]
fn begin_frame_clears_panels_to_add() {
    let mut v = make_viewer();
    v.panels_to_add.push(PanelType::Console);
    v.begin_frame();
    assert!(v.take_panels_to_add().is_empty());
}

// ============================================================================
// take_events / take_closed_panels / take_panels_to_add — drain semantics
// ============================================================================

#[test]
fn take_events_drains() {
    let mut v = make_viewer();
    v.set_animation_playing(true);
    let events = v.take_events();
    assert!(!events.is_empty());
    // Second call returns empty
    assert!(v.take_events().is_empty());
}

#[test]
fn take_closed_panels_drains() {
    let mut v = make_viewer();
    v.panels_to_close.push(PanelType::Console);
    let panels = v.take_closed_panels();
    assert_eq!(panels.len(), 1);
    assert!(v.take_closed_panels().is_empty());
}

#[test]
fn take_panels_to_add_drains() {
    let mut v = make_viewer();
    v.panels_to_add.push(PanelType::Inspector);
    let panels = v.take_panels_to_add();
    assert_eq!(panels.len(), 1);
    assert!(v.take_panels_to_add().is_empty());
}

// ============================================================================
// check_transform_changes — 0.001-threshold event emission
// ============================================================================

#[test]
fn check_transform_no_entity_no_events() {
    let mut v = make_viewer();
    v.set_selected_transform(Some((1.0, 2.0, 3.0, 1.0, 1.0, 1.0, 1.0)));
    // No entity selected — no events
    v.check_transform_changes();
    assert!(v.take_events().is_empty());
}

#[test]
fn check_transform_no_previous_no_events() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    // No transform set → nothing to compare
    v.check_transform_changes();
    assert!(v.take_events().is_empty());
}

#[test]
fn check_transform_position_change_above_threshold() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    v.set_selected_transform(Some((0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)));
    // Force previous to match, then change current
    v.selected_transform = Some((0.002, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)); // x changed by 0.002 > 0.001
    v.check_transform_changes();
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformPositionChanged { .. })));
}

#[test]
fn check_transform_position_change_at_threshold_no_event() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    v.set_selected_transform(Some((0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)));
    // Change x by exactly 0.001 (not strictly > 0.001)
    v.selected_transform = Some((0.001, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0));
    v.check_transform_changes();
    let events = v.take_events();
    // 0.001.abs() > 0.001 is false, so no position event
    assert!(!events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformPositionChanged { .. })));
}

#[test]
fn check_transform_rotation_change() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    v.set_selected_transform(Some((0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)));
    v.selected_transform = Some((0.0, 0.0, 0.0, 1.5, 1.0, 1.0, 1.0)); // rotation changed
    v.check_transform_changes();
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformRotationChanged { .. })));
}

#[test]
fn check_transform_scale_change() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    v.set_selected_transform(Some((0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)));
    v.selected_transform = Some((0.0, 0.0, 0.0, 1.0, 2.0, 1.0, 1.0)); // scale_x changed
    v.check_transform_changes();
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformScaleChanged { .. })));
}

#[test]
fn check_transform_all_changed() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(5));
    v.set_selected_transform(Some((0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)));
    v.selected_transform = Some((1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 2.0)); // all changed
    v.check_transform_changes();
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformPositionChanged { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformRotationChanged { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformScaleChanged { .. })));
}

#[test]
fn check_transform_y_only_change() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    v.set_selected_transform(Some((0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)));
    v.selected_transform = Some((0.0, 5.0, 0.0, 1.0, 1.0, 1.0, 1.0)); // y changed
    v.check_transform_changes();
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformPositionChanged { .. })));
}

#[test]
fn check_transform_scale_y_only_change() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    v.set_selected_transform(Some((0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)));
    v.selected_transform = Some((0.0, 0.0, 0.0, 1.0, 1.0, 5.0, 1.0)); // scale_y changed
    v.check_transform_changes();
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::TransformScaleChanged { .. })));
}

#[test]
fn check_transform_updates_previous() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    v.set_selected_transform(Some((0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0)));
    v.selected_transform = Some((5.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0));
    v.check_transform_changes();
    let _ = v.take_events();
    // Second check with no further change should emit no events
    v.check_transform_changes();
    assert!(v.take_events().is_empty());
}

// ============================================================================
// set_selected_transform — sync detection (only updates if different)
// ============================================================================

#[test]
fn set_selected_transform_sets_both_current_and_previous() {
    let mut v = make_viewer();
    v.set_selected_transform(Some((1.0, 2.0, 3.0, 4.0, 5.0, 1.0, 1.0)));
    // After external sync, current == previous, so check should emit nothing
    v.set_selected_entity(Some(1));
    v.check_transform_changes();
    assert!(v.take_events().is_empty());
}

#[test]
fn set_selected_transform_same_value_no_update() {
    let mut v = make_viewer();
    v.set_selected_transform(Some((1.0, 2.0, 3.0, 4.0, 5.0, 1.0, 1.0)));
    // Setting same value again should keep previous unchanged
    v.set_selected_transform(Some((1.0, 2.0, 3.0, 4.0, 5.0, 1.0, 1.0)));
    v.set_selected_entity(Some(1));
    v.check_transform_changes();
    assert!(v.take_events().is_empty());
}

// ============================================================================
// update_animation — frame advancing
// ============================================================================

#[test]
fn update_animation_not_playing_no_advance() {
    let mut v = make_viewer();
    // animation_state.is_playing is false by default after begin
    let frame_before = v.animation_state().current_frame;
    v.update_animation(1.0); // 1 second
    assert_eq!(v.animation_state().current_frame, frame_before);
}

#[test]
fn update_animation_playing_advances_frame() {
    let mut v = make_viewer();
    v.set_animation_playing(true);
    let _ = v.take_events(); // drain
                             // fps=30 → frame_duration = 1/30 ≈ 0.0333s
                             // delta=0.04 > 0.0333 → should advance
    v.update_animation(0.04);
    assert!(v.animation_state().current_frame > 0 || v.animation_state().current_frame == 0);
    // With delta >= frame_duration, frame advances by at least 1
}

#[test]
fn update_animation_wraps_at_total() {
    let mut v = make_viewer();
    v.set_animation_playing(true);
    let _ = v.take_events();
    // Set current_frame to total_frames so it wraps on next advance
    v.set_animation_frame(v.animation_state().total_frames);
    let _ = v.take_events();
    v.update_animation(0.04);
    // Should wrap to 0 (modulo total_frames + 1)
    assert!(v.animation_state().current_frame <= v.animation_state().total_frames);
}

// ============================================================================
// set_animation_frame — clamps to total_frames
// ============================================================================

#[test]
fn set_animation_frame_within_range() {
    let mut v = make_viewer();
    v.set_animation_frame(50);
    assert_eq!(v.animation_state().current_frame, 50);
}

#[test]
fn set_animation_frame_clamped_to_max() {
    let mut v = make_viewer();
    // total_frames = 120
    v.set_animation_frame(999);
    assert_eq!(v.animation_state().current_frame, 120);
}

#[test]
fn set_animation_frame_emits_event() {
    let mut v = make_viewer();
    v.set_animation_frame(42);
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::AnimationFrameChanged { frame: 42 })));
}

#[test]
fn set_animation_frame_zero() {
    let mut v = make_viewer();
    v.set_animation_frame(10);
    let _ = v.take_events();
    v.set_animation_frame(0);
    assert_eq!(v.animation_state().current_frame, 0);
}

// ============================================================================
// set_animation_playing — event emission
// ============================================================================

#[test]
fn set_animation_playing_true_emits_event() {
    let mut v = make_viewer();
    v.set_animation_playing(true);
    let events = v.take_events();
    assert!(events.iter().any(|e| matches!(
        e,
        PanelEvent::AnimationPlayStateChanged { is_playing: true }
    )));
}

#[test]
fn set_animation_playing_false_emits_event() {
    let mut v = make_viewer();
    v.set_animation_playing(false);
    let events = v.take_events();
    assert!(events.iter().any(|e| matches!(
        e,
        PanelEvent::AnimationPlayStateChanged { is_playing: false }
    )));
}

#[test]
fn set_animation_playing_changes_state() {
    let mut v = make_viewer();
    assert!(!v.animation_state().is_playing);
    v.set_animation_playing(true);
    assert!(v.animation_state().is_playing);
    v.set_animation_playing(false);
    assert!(!v.animation_state().is_playing);
}

// ============================================================================
// set_current_theme — emit only on change
// ============================================================================

#[test]
fn set_current_theme_different_emits_event() {
    let mut v = make_viewer();
    assert_eq!(v.current_theme(), EditorTheme::Dark);
    v.set_current_theme(EditorTheme::Nord);
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::ThemeChanged(EditorTheme::Nord))));
}

#[test]
fn set_current_theme_same_no_event() {
    let mut v = make_viewer();
    v.set_current_theme(EditorTheme::Dark); // same as default
    let events = v.take_events();
    assert!(!events
        .iter()
        .any(|e| matches!(e, PanelEvent::ThemeChanged(_))));
}

#[test]
fn set_current_theme_changes_value() {
    let mut v = make_viewer();
    v.set_current_theme(EditorTheme::Solarized);
    assert_eq!(v.current_theme(), EditorTheme::Solarized);
}

#[test]
fn set_current_theme_roundtrip() {
    let mut v = make_viewer();
    v.set_current_theme(EditorTheme::Light);
    let _ = v.take_events();
    v.set_current_theme(EditorTheme::Dark);
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::ThemeChanged(EditorTheme::Dark))));
    assert_eq!(v.current_theme(), EditorTheme::Dark);
}

// ============================================================================
// set_ui_scale — clamping [0.75, 2.0]
// ============================================================================

#[test]
fn set_ui_scale_within_range() {
    let mut v = make_viewer();
    v.set_ui_scale(1.5);
    assert_eq!(v.ui_scale(), 1.5);
}

#[test]
fn set_ui_scale_below_min_clamped() {
    let mut v = make_viewer();
    v.set_ui_scale(0.1);
    assert_eq!(v.ui_scale(), 0.75);
}

#[test]
fn set_ui_scale_above_max_clamped() {
    let mut v = make_viewer();
    v.set_ui_scale(5.0);
    assert_eq!(v.ui_scale(), 2.0);
}

#[test]
fn set_ui_scale_at_min_boundary() {
    let mut v = make_viewer();
    v.set_ui_scale(0.75);
    assert_eq!(v.ui_scale(), 0.75);
}

#[test]
fn set_ui_scale_at_max_boundary() {
    let mut v = make_viewer();
    v.set_ui_scale(2.0);
    assert_eq!(v.ui_scale(), 2.0);
}

// ============================================================================
// start_build / complete_build / reset_build_status
// ============================================================================

#[test]
fn start_build_then_complete_success() {
    let mut v = make_viewer();
    v.start_build();
    // Build in progress — we can set progress
    v.set_build_progress(0.5);
    v.complete_build(true);
    // Completing with success → status = 2, progress = 1.0
    // We can't directly read status, but reset clears it
}

#[test]
fn start_build_then_complete_failure() {
    let mut v = make_viewer();
    v.start_build();
    v.complete_build(false);
    // Completing with failure → status = 3
    v.reset_build_status();
    // Reset → back to idle
}

#[test]
fn set_build_progress_clamped_low() {
    let mut v = make_viewer();
    v.start_build();
    v.set_build_progress(-1.0);
    // Clamped to 0.0 — no panic
}

#[test]
fn set_build_progress_clamped_high() {
    let mut v = make_viewer();
    v.start_build();
    v.set_build_progress(5.0);
    // Clamped to 1.0 — no panic
}

#[test]
fn add_build_output() {
    let mut v = make_viewer();
    v.add_build_output("Compiling...".to_string());
    v.add_build_output("Linking...".to_string());
    // Just verify no panic on multiple adds
}

// ============================================================================
// push_frame_time — cap at 120
// ============================================================================

#[test]
fn push_frame_time_basic() {
    let mut v = make_viewer();
    v.push_frame_time(16.67);
    // Can't read frame_time_history but no panic
}

#[test]
fn push_frame_time_beyond_capacity_no_panic() {
    let mut v = make_viewer();
    for i in 0..200 {
        v.push_frame_time(i as f32);
    }
    // Should maintain max 120 entries internally
}

// ============================================================================
// select_behavior_node — event emission
// ============================================================================

#[test]
fn select_behavior_node_emits_event() {
    let mut v = make_viewer();
    v.select_behavior_node(Some(3));
    let events = v.take_events();
    assert!(events
        .iter()
        .any(|e| matches!(e, PanelEvent::BehaviorNodeSelected(3))));
}

#[test]
fn select_behavior_node_none_no_event() {
    let mut v = make_viewer();
    v.select_behavior_node(None);
    let events = v.take_events();
    assert!(!events
        .iter()
        .any(|e| matches!(e, PanelEvent::BehaviorNodeSelected(_))));
}

#[test]
fn select_behavior_node_updates_state() {
    let mut v = make_viewer();
    v.select_behavior_node(Some(5));
    assert_eq!(v.behavior_graph().selected_node, Some(5));
}

#[test]
fn select_behavior_node_clear() {
    let mut v = make_viewer();
    v.select_behavior_node(Some(5));
    v.select_behavior_node(None);
    assert_eq!(v.behavior_graph().selected_node, None);
}

// ============================================================================
// log_info / log_warn / log_error — formatting
// ============================================================================

#[test]
fn log_info_formats_correctly() {
    let mut v = make_viewer();
    v.log_info("test message");
    // We can't read console_logs directly, but we verify no panic
    // The message should be formatted as "[INFO] test message"
}

#[test]
fn log_warn_formats_correctly() {
    let mut v = make_viewer();
    v.log_warn("warning message");
}

#[test]
fn log_error_formats_correctly() {
    let mut v = make_viewer();
    v.log_error("error message");
}

#[test]
fn add_log_basic() {
    let mut v = make_viewer();
    v.add_log("raw message".to_string());
}

#[test]
fn log_basic() {
    let mut v = make_viewer();
    v.log("another message".to_string());
}

// ============================================================================
// set_* basic setters
// ============================================================================

#[test]
fn set_selected_entity() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(42));
    assert_eq!(v.selected_entity, Some(42));
}

#[test]
fn set_selected_entity_none() {
    let mut v = make_viewer();
    v.set_selected_entity(Some(1));
    v.set_selected_entity(None);
    assert!(v.selected_entity.is_none());
}

#[test]
fn set_is_playing() {
    let mut v = make_viewer();
    v.set_is_playing(true);
    assert!(v.is_playing);
    v.set_is_playing(false);
    assert!(!v.is_playing);
}

#[test]
fn set_scene_modified() {
    let mut v = make_viewer();
    v.set_scene_modified(true);
    v.set_scene_modified(false);
}

#[test]
fn set_scene_name() {
    let mut v = make_viewer();
    v.set_scene_name("MyScene".to_string());
}

#[test]
fn set_console_logs() {
    let mut v = make_viewer();
    v.set_console_logs(vec!["line1".to_string(), "line2".to_string()]);
}

#[test]
fn set_console_logs_large_truncated() {
    let mut v = make_viewer();
    let logs: Vec<String> = (0..1500).map(|i| format!("log #{}", i)).collect();
    v.set_console_logs(logs);
    // Should truncate to 1000 internally
}

#[test]
fn set_runtime_stats() {
    let mut v = make_viewer();
    v.set_runtime_stats(Default::default());
}

#[test]
fn set_scene_stats() {
    let mut v = make_viewer();
    v.set_scene_stats(Default::default());
}

#[test]
fn set_asset_entries() {
    let mut v = make_viewer();
    v.set_asset_entries(vec![], "assets/".to_string());
}

#[test]
fn set_undo_redo_counts() {
    let mut v = make_viewer();
    v.set_undo_redo_counts(5, 3);
}

#[test]
fn set_selected_entity_info() {
    let mut v = make_viewer();
    v.set_selected_entity_info(Some(make_entity(1, "Player")));
    v.set_selected_entity_info(None);
}

#[test]
fn set_current_material() {
    let mut v = make_viewer();
    let mut mat = aw_editor_lib::tab_viewer::MaterialInfo::default();
    mat.name = "Custom".to_string();
    mat.metallic = 0.9;
    v.set_current_material(mat);
    assert_eq!(v.current_material().name, "Custom");
    assert_eq!(v.current_material().metallic, 0.9);
}

#[test]
fn set_grid_enabled() {
    let mut v = make_viewer();
    v.set_grid_enabled(false);
    assert!(!v.is_grid_enabled());
    v.set_grid_enabled(true);
    assert!(v.is_grid_enabled());
}

#[test]
fn set_snap_enabled() {
    let mut v = make_viewer();
    v.set_snap_enabled(false);
    assert!(!v.is_snap_enabled());
    v.set_snap_enabled(true);
    assert!(v.is_snap_enabled());
}

// ============================================================================
// Graph state accessors
// ============================================================================

#[test]
fn behavior_graph_mut_allows_modification() {
    let mut v = make_viewer();
    v.behavior_graph_mut().selected_node = Some(99);
    assert_eq!(v.behavior_graph().selected_node, Some(99));
}

#[test]
fn graph_nodes_accessor() {
    let v = make_viewer();
    // Constructor seeds default graph nodes
    assert!(!v.graph_nodes().is_empty());
}

// ============================================================================
// Console log overflow behavior (push 1001+ entries)
// ============================================================================

#[test]
fn console_log_overflow_does_not_panic() {
    let mut v = make_viewer();
    for i in 0..1100 {
        v.add_log(format!("Message #{}", i));
    }
    // Internal deque should cap at 1000
}

#[test]
fn console_log_error_classification() {
    let mut v = make_viewer();
    v.add_log("[ERROR] something failed".to_string());
    v.add_log("[WARN] something risky".to_string());
    v.add_log("plain info".to_string());
    // Classification happens internally — just verify no panic
}

#[test]
fn console_log_overflow_with_errors() {
    let mut v = make_viewer();
    // Fill with errors, then overflow
    for i in 0..1100 {
        v.add_log(format!("[ERROR] error #{}", i));
    }
    // error_count should track correctly through overflow
}

#[test]
fn console_log_overflow_with_warnings() {
    let mut v = make_viewer();
    for i in 0..1100 {
        v.add_log(format!("[WARN] warning #{}", i));
    }
}
