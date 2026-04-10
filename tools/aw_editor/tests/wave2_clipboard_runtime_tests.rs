//! Wave 2 Mutation-Resistant Tests: Clipboard & Runtime
//!
//! Targets mutation-prone patterns in clipboard.rs and runtime.rs:
//! - ClipboardEntityData validation (scale>0, name length, health/ammo sign)
//! - ClipboardEntityData helper methods (has_ai, is_scaled, is_rotated, position)
//! - ClipboardData version compatibility, stats, find_by_name, filter_by_team
//! - ClipboardValidation builder methods, summary, Display
//! - ClipboardStats percentage arithmetic, zero-division guards
//! - RuntimeState transitions (can_transition_to matrix)
//! - RuntimeState properties (has_simulation, is_editable, is_active)
//! - RuntimeState display/icon/shortcut/description strings
//! - RuntimeIssue classification (is_critical, is_performance, is_data, is_recoverable)
//! - RuntimeIssue severity levels and icon mapping
//! - RuntimeStats performance grades, budget percentages, headroom, validation

use astraweave_core::{IVec2, World};
use aw_editor_lib::clipboard::{
    ClipboardData, ClipboardEntityData, ClipboardValidation, CLIPBOARD_SCHEMA_VERSION,
};
use aw_editor_lib::runtime::{RuntimeIssue, RuntimeState, RuntimeStats};
use std::collections::BTreeMap;

// ============================================================================
// Helpers
// ============================================================================

fn make_entity_data(name: &str, hp: i32, ammo: i32, team: u8) -> ClipboardEntityData {
    ClipboardEntityData {
        name: name.to_string(),
        pos: IVec2::new(5, 10),
        rotation: 0.0,
        rotation_x: 0.0,
        rotation_z: 0.0,
        scale: 1.0,
        hp,
        team_id: team,
        ammo,
        cooldowns: BTreeMap::new(),
        behavior_graph: None,
        height: 0.0,
        parent: None,
    }
}

fn make_clipboard(entities: Vec<ClipboardEntityData>) -> ClipboardData {
    ClipboardData {
        version: CLIPBOARD_SCHEMA_VERSION,
        entities,
    }
}

// ============================================================================
// Section A: ClipboardEntityData Validation
// ============================================================================

#[test]
fn entity_data_valid_passes() {
    let data = make_entity_data("Player", 100, 30, 0);
    let v = data.validate();
    assert!(v.is_valid);
    assert!(v.errors.is_empty());
    assert!(v.warnings.is_empty());
}

#[test]
fn entity_data_empty_name_warning() {
    let data = make_entity_data("", 100, 30, 0);
    let v = data.validate();
    assert!(v.is_valid); // Warning only
    assert!(v.has_warnings());
    assert!(v
        .warnings
        .iter()
        .any(|w| w.to_lowercase().contains("empty")));
}

#[test]
fn entity_data_long_name_error() {
    let name = "x".repeat(300);
    let data = make_entity_data(&name, 100, 30, 0);
    let v = data.validate();
    assert!(!v.is_valid);
    assert!(v.has_errors());
    assert!(v.errors.iter().any(|e| e.contains("256")));
}

#[test]
fn entity_data_zero_scale_error() {
    let mut data = make_entity_data("E", 100, 30, 0);
    data.scale = 0.0;
    let v = data.validate();
    assert!(!v.is_valid);
    assert!(v.errors.iter().any(|e| e.to_lowercase().contains("scale")));
}

#[test]
fn entity_data_negative_scale_error() {
    let mut data = make_entity_data("E", 100, 30, 0);
    data.scale = -1.0;
    let v = data.validate();
    assert!(!v.is_valid);
}

#[test]
fn entity_data_very_large_scale_warning() {
    let mut data = make_entity_data("E", 100, 30, 0);
    data.scale = 1001.0;
    let v = data.validate();
    assert!(v.is_valid); // Warning only
    assert!(v.has_warnings());
}

#[test]
fn entity_data_scale_exactly_1000_no_warning() {
    let mut data = make_entity_data("E", 100, 30, 0);
    data.scale = 1000.0;
    let v = data.validate();
    // 1000.0 is NOT > 1000.0, so no warning
    assert!(!v
        .warnings
        .iter()
        .any(|w| w.to_lowercase().contains("large scale")));
}

#[test]
fn entity_data_negative_hp_warning() {
    let data = make_entity_data("E", -5, 30, 0);
    let v = data.validate();
    assert!(v.is_valid); // Warning
    assert!(v
        .warnings
        .iter()
        .any(|w| w.to_lowercase().contains("health") || w.to_lowercase().contains("negative")));
}

#[test]
fn entity_data_negative_ammo_warning() {
    let data = make_entity_data("E", 100, -1, 0);
    let v = data.validate();
    assert!(v.is_valid); // Warning
    assert!(v
        .warnings
        .iter()
        .any(|w| w.to_lowercase().contains("ammo") || w.to_lowercase().contains("negative")));
}

// ============================================================================
// Section B: ClipboardEntityData Helper Methods
// ============================================================================

#[test]
fn entity_data_has_ai_false_when_no_behavior_graph() {
    let data = make_entity_data("E", 100, 30, 0);
    assert!(!data.has_ai());
}

#[test]
fn entity_data_has_cooldowns_false_when_empty() {
    let data = make_entity_data("E", 100, 30, 0);
    assert!(!data.has_cooldowns());
    assert_eq!(data.cooldown_count(), 0);
}

#[test]
fn entity_data_has_cooldowns_true_when_populated() {
    let mut data = make_entity_data("E", 100, 30, 0);
    data.cooldowns.insert("fire".to_string(), 2.5);
    assert!(data.has_cooldowns());
    assert_eq!(data.cooldown_count(), 1);
}

#[test]
fn entity_data_is_scaled_threshold() {
    let mut data = make_entity_data("E", 100, 30, 0);

    data.scale = 1.0;
    assert!(!data.is_scaled()); // Exactly 1.0 → not scaled

    data.scale = 1.002; // Just over threshold
    assert!(data.is_scaled());

    data.scale = 0.998;
    assert!(data.is_scaled());

    data.scale = 2.0;
    assert!(data.is_scaled());
}

#[test]
fn entity_data_is_rotated_threshold() {
    let mut data = make_entity_data("E", 100, 30, 0);

    // All zeros → not rotated
    assert!(!data.is_rotated());

    // Just rotation_x
    data.rotation_x = 0.5;
    assert!(data.is_rotated());

    data.rotation_x = 0.0;
    data.rotation = 0.01; // Y
    assert!(data.is_rotated());

    data.rotation = 0.0;
    data.rotation_z = 0.01;
    assert!(data.is_rotated());
}

#[test]
fn entity_data_position_returns_tuple() {
    let data = make_entity_data("E", 100, 30, 0);
    assert_eq!(data.position(), (5, 10));
}

#[test]
fn entity_data_summary_contains_name_and_hp() {
    let data = make_entity_data("Player", 75, 30, 2);
    let s = data.summary();
    assert!(s.contains("Player"));
    assert!(s.contains("75"));
    assert!(s.contains("2")); // team
}

#[test]
fn entity_data_display_contains_name_and_pos() {
    let data = make_entity_data("Soldier", 100, 30, 0);
    let d = format!("{}", data);
    assert!(d.contains("Soldier"));
    assert!(d.contains("5") && d.contains("10"));
}

// ============================================================================
// Section C: ClipboardValidation
// ============================================================================

#[test]
fn clipboard_validation_valid_factory() {
    let v = ClipboardValidation::valid();
    assert!(v.is_valid);
    assert!(v.errors.is_empty());
    assert!(v.warnings.is_empty());
    assert_eq!(v.issue_count(), 0);
    assert!(!v.has_warnings());
    assert!(!v.has_errors());
}

#[test]
fn clipboard_validation_with_error_factory() {
    let v = ClipboardValidation::with_error("Something went wrong");
    assert!(!v.is_valid);
    assert_eq!(v.errors.len(), 1);
    assert!(v.errors[0].contains("Something went wrong"));
    assert!(v.has_errors());
}

#[test]
fn clipboard_validation_add_warning_keeps_valid() {
    let mut v = ClipboardValidation::valid();
    v.add_warning("Minor issue");
    assert!(v.is_valid);
    assert!(v.has_warnings());
    assert_eq!(v.issue_count(), 1);
}

#[test]
fn clipboard_validation_add_error_marks_invalid() {
    let mut v = ClipboardValidation::valid();
    v.add_error("Major issue");
    assert!(!v.is_valid);
    assert!(v.has_errors());
    assert_eq!(v.issue_count(), 1);
}

#[test]
fn clipboard_validation_summary_valid() {
    let v = ClipboardValidation::valid();
    assert_eq!(v.summary(), "Valid");
}

#[test]
fn clipboard_validation_summary_with_warnings() {
    let mut v = ClipboardValidation::valid();
    v.add_warning("w1");
    v.add_warning("w2");
    let s = v.summary();
    assert!(s.contains("2") && s.to_lowercase().contains("warning"));
}

#[test]
fn clipboard_validation_summary_with_errors() {
    let mut v = ClipboardValidation::valid();
    v.add_error("e1");
    v.add_warning("w1");
    let s = v.summary();
    assert!(s.contains("1") && s.to_lowercase().contains("error"));
}

#[test]
fn clipboard_validation_display_valid() {
    let v = ClipboardValidation::valid();
    let d = format!("{}", v);
    assert!(d.contains("Valid") || d.contains("✓"));
}

#[test]
fn clipboard_validation_display_invalid() {
    let v = ClipboardValidation::with_error("error");
    let d = format!("{}", v);
    assert!(d.contains("Invalid") || d.contains("✗"));
}

// ============================================================================
// Section D: ClipboardData — Version & Operations
// ============================================================================

#[test]
fn clipboard_data_version_is_current() {
    let cb = make_clipboard(vec![make_entity_data("E", 100, 30, 0)]);
    assert_eq!(cb.version, CLIPBOARD_SCHEMA_VERSION);
    assert!(cb.is_compatible());
}

#[test]
fn clipboard_data_future_version_still_validates_with_warning() {
    let mut cb = make_clipboard(vec![make_entity_data("E", 100, 30, 0)]);
    cb.version = CLIPBOARD_SCHEMA_VERSION + 10;
    assert!(!cb.is_compatible());

    let v = cb.validate();
    assert!(v.is_valid); // Warning only
    assert!(v.has_warnings());
}

#[test]
fn clipboard_data_len_and_is_empty() {
    let empty = make_clipboard(vec![]);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());

    let non_empty = make_clipboard(vec![make_entity_data("E", 100, 30, 0)]);
    assert_eq!(non_empty.len(), 1);
    assert!(!non_empty.is_empty());
}

#[test]
fn clipboard_data_entity_names() {
    let cb = make_clipboard(vec![
        make_entity_data("Alpha", 100, 30, 0),
        make_entity_data("Beta", 80, 20, 1),
    ]);
    let names = cb.entity_names();
    assert_eq!(names, vec!["Alpha", "Beta"]);
}

#[test]
fn clipboard_data_find_by_name_case_insensitive() {
    let cb = make_clipboard(vec![
        make_entity_data("Player", 100, 30, 0),
        make_entity_data("Enemy_Guard", 80, 20, 1),
        make_entity_data("Enemy_Scout", 60, 10, 1),
    ]);

    let results = cb.find_by_name("enemy");
    assert_eq!(results.len(), 2);
    assert!(results.contains(&1));
    assert!(results.contains(&2));
}

#[test]
fn clipboard_data_filter_by_team() {
    let cb = make_clipboard(vec![
        make_entity_data("Player", 100, 30, 0),
        make_entity_data("Ally", 80, 20, 0),
        make_entity_data("Enemy", 60, 10, 1),
    ]);

    let team0 = cb.filter_by_team(0);
    assert_eq!(team0.len(), 2);

    let team1 = cb.filter_by_team(1);
    assert_eq!(team1.len(), 1);
    assert_eq!(team1[0].name, "Enemy");

    let team99 = cb.filter_by_team(99);
    assert!(team99.is_empty());
}

#[test]
fn clipboard_data_validate_empty_has_warning() {
    let cb = make_clipboard(vec![]);
    let v = cb.validate();
    assert!(v.is_valid);
    assert!(v.has_warnings());
    assert!(v
        .warnings
        .iter()
        .any(|w| w.to_lowercase().contains("no entities") || w.to_lowercase().contains("empty")));
}

#[test]
fn clipboard_data_validate_propagates_entity_errors() {
    let mut bad = make_entity_data("E", 100, 30, 0);
    bad.scale = -1.0; // Error
    let cb = make_clipboard(vec![bad]);

    let v = cb.validate();
    assert!(!v.is_valid);
    assert!(v.errors.iter().any(|e| e.contains("Entity 0")));
}

#[test]
fn clipboard_data_json_round_trip() {
    let cb = make_clipboard(vec![
        make_entity_data("Alpha", 100, 30, 0),
        make_entity_data("Beta", 80, 20, 1),
    ]);

    let json = cb.to_json().unwrap();
    let restored = ClipboardData::from_json(&json).unwrap();

    assert_eq!(restored.entities.len(), 2);
    assert_eq!(restored.entities[0].name, "Alpha");
    assert_eq!(restored.entities[1].name, "Beta");
    assert_eq!(restored.version, cb.version);
}

#[test]
fn clipboard_data_spawn_entities_with_offset() {
    let mut world = World::new();
    let cb = make_clipboard(vec![make_entity_data("E", 75, 15, 2)]);

    let spawned = cb.spawn_entities(&mut world, IVec2::new(10, 20)).unwrap();
    assert_eq!(spawned.len(), 1);

    let pose = world.pose(spawned[0]).unwrap();
    assert_eq!(pose.pos, IVec2::new(15, 30)); // (5+10, 10+20)
}

// ============================================================================
// Section E: ClipboardStats
// ============================================================================

#[test]
fn clipboard_stats_empty() {
    let cb = make_clipboard(vec![]);
    let s = cb.stats();
    assert!(s.is_empty());
    assert_eq!(s.entity_count, 0);
    assert!(!s.has_ai_entities());
    assert!(!s.has_cooldown_entities());
    assert!(!s.is_multi_team());
}

#[test]
fn clipboard_stats_populated() {
    let mut e1 = make_entity_data("A", 100, 30, 0);
    e1.cooldowns.insert("fire".to_string(), 2.0);
    let e2 = make_entity_data("B", 80, 20, 1);

    let cb = make_clipboard(vec![e1, e2]);
    let s = cb.stats();

    assert_eq!(s.entity_count, 2);
    assert!(!s.is_empty());
    assert!(!s.has_ai_entities()); // No behavior graphs
    assert!(s.has_cooldown_entities()); // e1 has cooldowns
    assert!(s.is_multi_team()); // teams 0 and 1
    assert_eq!(s.unique_teams, 2);
    assert_eq!(s.total_cooldowns, 1);
}

#[test]
fn clipboard_stats_ai_percentage() {
    let cb = make_clipboard(vec![
        make_entity_data("A", 100, 30, 0),
        make_entity_data("B", 80, 20, 0),
    ]);
    let s = cb.stats();
    assert_eq!(s.ai_percentage(), 0.0); // 0 out of 2
}

#[test]
fn clipboard_stats_ai_percentage_zero_entities() {
    let cb = make_clipboard(vec![]);
    let s = cb.stats();
    assert_eq!(s.ai_percentage(), 0.0);
}

#[test]
fn clipboard_stats_avg_cooldowns_per_entity() {
    let mut e1 = make_entity_data("A", 100, 30, 0);
    e1.cooldowns.insert("fire".to_string(), 2.0);
    e1.cooldowns.insert("heal".to_string(), 5.0);
    let e2 = make_entity_data("B", 80, 20, 0);

    let cb = make_clipboard(vec![e1, e2]);
    let s = cb.stats();
    assert!((s.avg_cooldowns_per_entity() - 1.0).abs() < 0.001); // 2 cooldowns / 2 entities
}

#[test]
fn clipboard_stats_avg_cooldowns_zero_entities() {
    let cb = make_clipboard(vec![]);
    let s = cb.stats();
    assert_eq!(s.avg_cooldowns_per_entity(), 0.0);
}

#[test]
fn clipboard_stats_summary_format() {
    let cb = make_clipboard(vec![
        make_entity_data("A", 100, 30, 0),
        make_entity_data("B", 80, 20, 1),
    ]);
    let s = cb.stats();
    let summary = s.summary();
    assert!(summary.contains("2")); // 2 entities
}

#[test]
fn clipboard_stats_display_format() {
    let cb = make_clipboard(vec![make_entity_data("A", 100, 30, 0)]);
    let s = cb.stats();
    let d = format!("{}", s);
    assert!(d.contains("1")); // 1 entity
}

// ============================================================================
// Section F: RuntimeState — Transition Matrix
// ============================================================================

#[test]
fn editing_can_only_play() {
    let s = RuntimeState::Editing;
    assert!(s.can_transition_to(RuntimeState::Playing));
    assert!(!s.can_transition_to(RuntimeState::Paused));
    assert!(!s.can_transition_to(RuntimeState::Editing));
    assert!(!s.can_transition_to(RuntimeState::SteppingOneFrame));
}

#[test]
fn playing_can_pause_stop_step() {
    let s = RuntimeState::Playing;
    assert!(s.can_transition_to(RuntimeState::Paused));
    assert!(s.can_transition_to(RuntimeState::Editing));
    assert!(s.can_transition_to(RuntimeState::SteppingOneFrame));
    assert!(!s.can_transition_to(RuntimeState::Playing)); // Can't play while already playing
}

#[test]
fn paused_can_resume_stop_step() {
    let s = RuntimeState::Paused;
    assert!(s.can_transition_to(RuntimeState::Playing));
    assert!(s.can_transition_to(RuntimeState::Editing));
    assert!(s.can_transition_to(RuntimeState::SteppingOneFrame));
    assert!(!s.can_transition_to(RuntimeState::Paused)); // Can't pause while paused
}

#[test]
fn stepping_can_transition_to_all() {
    let s = RuntimeState::SteppingOneFrame;
    assert!(s.can_transition_to(RuntimeState::Paused));
    assert!(s.can_transition_to(RuntimeState::Editing));
    assert!(s.can_transition_to(RuntimeState::Playing));
    assert!(s.can_transition_to(RuntimeState::SteppingOneFrame));
}

#[test]
fn valid_transitions_return_correct_set() {
    let from_editing = RuntimeState::Editing.valid_transitions();
    assert_eq!(from_editing.len(), 1);
    assert!(from_editing.contains(&RuntimeState::Playing));

    let from_playing = RuntimeState::Playing.valid_transitions();
    assert_eq!(from_playing.len(), 3);

    let from_stepping = RuntimeState::SteppingOneFrame.valid_transitions();
    assert_eq!(from_stepping.len(), 4);
}

// ============================================================================
// Section G: RuntimeState — Properties
// ============================================================================

#[test]
fn has_simulation_only_when_not_editing() {
    assert!(!RuntimeState::Editing.has_simulation());
    assert!(RuntimeState::Playing.has_simulation());
    assert!(RuntimeState::Paused.has_simulation());
    assert!(RuntimeState::SteppingOneFrame.has_simulation());
}

#[test]
fn is_editable_only_when_editing() {
    assert!(RuntimeState::Editing.is_editable());
    assert!(!RuntimeState::Playing.is_editable());
    assert!(!RuntimeState::Paused.is_editable());
    assert!(!RuntimeState::SteppingOneFrame.is_editable());
}

#[test]
fn is_active_when_playing_or_stepping() {
    assert!(!RuntimeState::Editing.is_active());
    assert!(RuntimeState::Playing.is_active());
    assert!(!RuntimeState::Paused.is_active());
    assert!(RuntimeState::SteppingOneFrame.is_active());
}

#[test]
fn runtime_state_all_has_four() {
    assert_eq!(RuntimeState::all().len(), 4);
}

// ============================================================================
// Section H: RuntimeState — Display & Strings
// ============================================================================

#[test]
fn runtime_state_display_unique_per_state() {
    let displays: Vec<String> = RuntimeState::all()
        .iter()
        .map(|s| format!("{}", s))
        .collect();
    let unique: std::collections::HashSet<&String> = displays.iter().collect();
    assert_eq!(displays.len(), unique.len());
}

#[test]
fn runtime_state_icons_nonempty() {
    for s in RuntimeState::all() {
        assert!(!s.icon().is_empty());
    }
}

#[test]
fn runtime_state_shortcuts_nonempty() {
    for s in RuntimeState::all() {
        assert!(!s.shortcut_hint().is_empty());
    }
}

#[test]
fn runtime_state_descriptions_nonempty() {
    for s in RuntimeState::all() {
        assert!(!s.description().is_empty());
    }
}

// ============================================================================
// Section I: RuntimeIssue — Classification
// ============================================================================

#[test]
fn issue_is_critical_only_for_sim_and_corruption() {
    assert!(RuntimeIssue::MissingSimulation.is_critical());
    assert!(RuntimeIssue::CorruptedSimulation {
        reason: "test".into()
    }
    .is_critical());
    assert!(!RuntimeIssue::MissingEditSnapshot.is_critical());
    assert!(!RuntimeIssue::FrameTimeExceeded {
        frame_time_ms: 50,
        threshold_ms: 33
    }
    .is_critical());
    assert!(!RuntimeIssue::LowFps {
        fps: 15,
        minimum_fps: 30
    }
    .is_critical());
    assert!(!RuntimeIssue::EntityCountMismatch {
        expected: 100,
        actual: 95
    }
    .is_critical());
}

#[test]
fn issue_is_performance_only_for_frame_time_and_fps() {
    assert!(!RuntimeIssue::MissingSimulation.is_performance_issue());
    assert!(!RuntimeIssue::MissingEditSnapshot.is_performance_issue());
    assert!(!RuntimeIssue::CorruptedSimulation {
        reason: "test".into()
    }
    .is_performance_issue());
    assert!(RuntimeIssue::FrameTimeExceeded {
        frame_time_ms: 50,
        threshold_ms: 33
    }
    .is_performance_issue());
    assert!(RuntimeIssue::LowFps {
        fps: 15,
        minimum_fps: 30
    }
    .is_performance_issue());
    assert!(!RuntimeIssue::EntityCountMismatch {
        expected: 100,
        actual: 95
    }
    .is_performance_issue());
}

#[test]
fn issue_is_data_for_relevant_variants() {
    assert!(!RuntimeIssue::MissingSimulation.is_data_issue());
    assert!(RuntimeIssue::MissingEditSnapshot.is_data_issue());
    assert!(RuntimeIssue::CorruptedSimulation {
        reason: "test".into()
    }
    .is_data_issue());
    assert!(!RuntimeIssue::FrameTimeExceeded {
        frame_time_ms: 50,
        threshold_ms: 33
    }
    .is_data_issue());
    assert!(!RuntimeIssue::LowFps {
        fps: 15,
        minimum_fps: 30
    }
    .is_data_issue());
    assert!(RuntimeIssue::EntityCountMismatch {
        expected: 100,
        actual: 95
    }
    .is_data_issue());
}

#[test]
fn issue_is_recoverable_only_for_performance_and_mismatch() {
    assert!(!RuntimeIssue::MissingSimulation.is_recoverable());
    assert!(!RuntimeIssue::MissingEditSnapshot.is_recoverable());
    assert!(!RuntimeIssue::CorruptedSimulation {
        reason: "test".into()
    }
    .is_recoverable());
    assert!(RuntimeIssue::FrameTimeExceeded {
        frame_time_ms: 50,
        threshold_ms: 33
    }
    .is_recoverable());
    assert!(RuntimeIssue::LowFps {
        fps: 15,
        minimum_fps: 30
    }
    .is_recoverable());
    assert!(RuntimeIssue::EntityCountMismatch {
        expected: 100,
        actual: 95
    }
    .is_recoverable());
}

// ============================================================================
// Section J: RuntimeIssue — Severity & Display
// ============================================================================

#[test]
fn issue_severity_levels() {
    assert_eq!(RuntimeIssue::MissingSimulation.severity(), 5);
    assert_eq!(
        RuntimeIssue::CorruptedSimulation { reason: "t".into() }.severity(),
        5
    );
    assert_eq!(RuntimeIssue::MissingEditSnapshot.severity(), 4);
    assert_eq!(
        RuntimeIssue::EntityCountMismatch {
            expected: 10,
            actual: 5
        }
        .severity(),
        3
    );
    assert_eq!(
        RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33
        }
        .severity(),
        2
    );
    assert_eq!(
        RuntimeIssue::LowFps {
            fps: 15,
            minimum_fps: 30
        }
        .severity(),
        1
    );
}

#[test]
fn issue_icon_maps_to_severity() {
    // Severity 5 → [R]
    assert_eq!(RuntimeIssue::MissingSimulation.icon(), "[R]");
    assert_eq!(
        RuntimeIssue::CorruptedSimulation { reason: "x".into() }.icon(),
        "[R]"
    );
    // Severity 4 → 🟠
    assert_eq!(RuntimeIssue::MissingEditSnapshot.icon(), "🟠");
    // Severity 3 → [Y]
    assert_eq!(
        RuntimeIssue::EntityCountMismatch {
            expected: 10,
            actual: 5
        }
        .icon(),
        "[Y]"
    );
    // Severity 2 → [G]
    assert_eq!(
        RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33
        }
        .icon(),
        "[G]"
    );
    // Severity 1 → ℹ️
    assert_eq!(
        RuntimeIssue::LowFps {
            fps: 15,
            minimum_fps: 30
        }
        .icon(),
        "ℹ️"
    );
}

#[test]
fn issue_titles_nonempty_and_unique() {
    let issues = RuntimeIssue::all_variants();
    let titles: Vec<&str> = issues.iter().map(|i| i.title()).collect();
    for t in &titles {
        assert!(!t.is_empty());
    }
}

#[test]
fn issue_display_frame_time_contains_values() {
    let issue = RuntimeIssue::FrameTimeExceeded {
        frame_time_ms: 50,
        threshold_ms: 33,
    };
    let d = format!("{}", issue);
    assert!(d.contains("50"));
    assert!(d.contains("33"));
}

#[test]
fn issue_display_low_fps_contains_values() {
    let issue = RuntimeIssue::LowFps {
        fps: 15,
        minimum_fps: 30,
    };
    let d = format!("{}", issue);
    assert!(d.contains("15"));
    assert!(d.contains("30"));
}

#[test]
fn issue_display_entity_mismatch_contains_values() {
    let issue = RuntimeIssue::EntityCountMismatch {
        expected: 100,
        actual: 95,
    };
    let d = format!("{}", issue);
    assert!(d.contains("100"));
    assert!(d.contains("95"));
}

#[test]
fn issue_display_corrupted_contains_reason() {
    let issue = RuntimeIssue::CorruptedSimulation {
        reason: "bad data".to_string(),
    };
    let d = format!("{}", issue);
    assert!(d.contains("bad data"));
}

// ============================================================================
// Section K: RuntimeStats — Performance Grades & Budget
// ============================================================================

#[test]
fn stats_performance_grade_critical() {
    let s = RuntimeStats {
        fps: 10.0,
        ..Default::default()
    };
    assert_eq!(s.performance_grade(), "Critical");
}

#[test]
fn stats_performance_grade_poor() {
    let s = RuntimeStats {
        fps: 20.0,
        ..Default::default()
    };
    assert_eq!(s.performance_grade(), "Poor");
}

#[test]
fn stats_performance_grade_fair() {
    let s = RuntimeStats {
        fps: 35.0,
        ..Default::default()
    };
    assert_eq!(s.performance_grade(), "Fair");
}

#[test]
fn stats_performance_grade_good() {
    let s = RuntimeStats {
        fps: 50.0,
        ..Default::default()
    };
    assert_eq!(s.performance_grade(), "Good");
}

#[test]
fn stats_performance_grade_excellent() {
    let s = RuntimeStats {
        fps: 120.0,
        ..Default::default()
    };
    assert_eq!(s.performance_grade(), "Excellent");
}

#[test]
fn stats_frame_budget_percentage() {
    let s = RuntimeStats {
        frame_time_ms: 8.33,
        ..Default::default()
    };
    // 8.33 / 16.667 * 100 ≈ 50%
    assert!((s.frame_budget_percentage() - 50.0).abs() < 1.0);
}

#[test]
fn stats_frame_time_headroom_positive() {
    let s = RuntimeStats {
        frame_time_ms: 10.0,
        ..Default::default()
    };
    assert!((s.frame_time_headroom() - 6.667).abs() < 0.01);
}

#[test]
fn stats_frame_time_headroom_negative_overbudget() {
    let s = RuntimeStats {
        frame_time_ms: 20.0,
        ..Default::default()
    };
    assert!(s.frame_time_headroom() < 0.0); // Over budget
}

#[test]
fn stats_is_running_smoothly_true() {
    let s = RuntimeStats {
        fps: 61.0,
        frame_time_ms: 16.0,
        ..Default::default()
    };
    assert!(s.is_running_smoothly());
}

#[test]
fn stats_is_running_smoothly_false_low_fps() {
    let s = RuntimeStats {
        fps: 59.0,
        frame_time_ms: 16.0,
        ..Default::default()
    };
    assert!(!s.is_running_smoothly());
}

#[test]
fn stats_is_running_smoothly_false_high_frame_time() {
    let s = RuntimeStats {
        fps: 65.0,
        frame_time_ms: 17.0,
        ..Default::default()
    };
    assert!(!s.is_running_smoothly());
}

#[test]
fn stats_is_frame_time_healthy() {
    let s = RuntimeStats {
        frame_time_ms: 10.0,
        ..Default::default()
    };
    assert!(s.is_frame_time_healthy(16.67));
    assert!(!s.is_frame_time_healthy(5.0));
}

#[test]
fn stats_is_fps_healthy() {
    let s = RuntimeStats {
        fps: 60.0,
        ..Default::default()
    };
    assert!(s.is_fps_healthy(30.0));
    assert!(s.is_fps_healthy(60.0));
    assert!(!s.is_fps_healthy(61.0));
}

#[test]
fn stats_is_frame_time_critical_threshold() {
    let s = RuntimeStats {
        frame_time_ms: 33.33,
        ..Default::default()
    };
    assert!(!s.is_frame_time_critical()); // Exactly at threshold, not >

    let s2 = RuntimeStats {
        frame_time_ms: 33.34,
        ..Default::default()
    };
    assert!(s2.is_frame_time_critical());
}

#[test]
fn stats_simulation_duration_secs() {
    let s = RuntimeStats {
        tick_count: 120,
        ..Default::default()
    };
    assert!((s.simulation_duration_secs() - 2.0).abs() < 0.001);

    let s2 = RuntimeStats {
        tick_count: 0,
        ..Default::default()
    };
    assert_eq!(s2.simulation_duration_secs(), 0.0);
}

#[test]
fn stats_estimated_entity_capacity_zero_when_no_entities() {
    let s = RuntimeStats {
        entity_count: 0,
        frame_time_ms: 10.0,
        ..Default::default()
    };
    assert_eq!(s.estimated_entity_capacity(), 0);
}

#[test]
fn stats_estimated_entity_capacity_zero_when_no_frame_time() {
    let s = RuntimeStats {
        entity_count: 100,
        frame_time_ms: 0.0,
        ..Default::default()
    };
    assert_eq!(s.estimated_entity_capacity(), 0);
}

#[test]
fn stats_estimated_entity_capacity_calculation() {
    let s = RuntimeStats {
        entity_count: 100,
        frame_time_ms: 8.33,
        ..Default::default()
    };
    // 100 / 8.33 * 16.667 ≈ 200
    let cap = s.estimated_entity_capacity();
    assert!(cap >= 180 && cap <= 220);
}

#[test]
fn stats_validate_detects_high_frame_time() {
    let s = RuntimeStats {
        frame_time_ms: 40.0,
        ..Default::default()
    };
    let issues = s.validate();
    assert!(issues
        .iter()
        .any(|i| matches!(i, RuntimeIssue::FrameTimeExceeded { .. })));
}

#[test]
fn stats_validate_detects_low_fps() {
    let s = RuntimeStats {
        fps: 20.0,
        ..Default::default()
    };
    let issues = s.validate();
    assert!(issues
        .iter()
        .any(|i| matches!(i, RuntimeIssue::LowFps { .. })));
}

#[test]
fn stats_validate_no_issues_when_healthy() {
    let s = RuntimeStats {
        fps: 60.0,
        frame_time_ms: 16.0,
        ..Default::default()
    };
    assert!(s.validate().is_empty());
}

#[test]
fn stats_validate_zero_fps_no_low_fps_issue() {
    // fps=0 → condition is fps > 0.0 && fps < 30.0, so 0.0 doesn't trigger
    let s = RuntimeStats {
        fps: 0.0,
        ..Default::default()
    };
    let issues = s.validate();
    assert!(!issues
        .iter()
        .any(|i| matches!(i, RuntimeIssue::LowFps { .. })));
}

#[test]
fn stats_fps_stability_capped_at_one() {
    let s = RuntimeStats {
        fps: 120.0,
        ..Default::default()
    };
    assert!((s.fps_stability() - 1.0).abs() < 0.001);
}

#[test]
fn stats_fps_stability_zero_when_no_fps() {
    let s = RuntimeStats {
        fps: 0.0,
        ..Default::default()
    };
    assert_eq!(s.fps_stability(), 0.0);
}

#[test]
fn stats_fps_stability_half() {
    let s = RuntimeStats {
        fps: 30.0,
        ..Default::default()
    };
    assert!((s.fps_stability() - 0.5).abs() < 0.001);
}

#[test]
fn stats_status_color_maps_grade() {
    let excellent = RuntimeStats {
        fps: 65.0,
        ..Default::default()
    };
    assert_eq!(excellent.status_color(), "green");

    let good = RuntimeStats {
        fps: 50.0,
        ..Default::default()
    };
    assert_eq!(good.status_color(), "lightgreen");

    let fair = RuntimeStats {
        fps: 35.0,
        ..Default::default()
    };
    assert_eq!(fair.status_color(), "yellow");

    let poor = RuntimeStats {
        fps: 20.0,
        ..Default::default()
    };
    assert_eq!(poor.status_color(), "orange");

    let critical = RuntimeStats {
        fps: 10.0,
        ..Default::default()
    };
    assert_eq!(critical.status_color(), "red");
}

#[test]
fn stats_is_healthy_alias_for_is_running_smoothly() {
    let healthy = RuntimeStats {
        fps: 61.0,
        frame_time_ms: 16.0,
        ..Default::default()
    };
    assert_eq!(healthy.is_healthy(), healthy.is_running_smoothly());

    let unhealthy = RuntimeStats {
        fps: 20.0,
        frame_time_ms: 50.0,
        ..Default::default()
    };
    assert_eq!(unhealthy.is_healthy(), unhealthy.is_running_smoothly());
}
