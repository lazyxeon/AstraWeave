//! Wave 2 Mutation-Resistant Tests: Command System & Undo Stack
//!
//! Targets mutation-prone patterns in command.rs:
//! - UndoStack cursor arithmetic (+/-/boundary)
//! - UndoStackStats utilization (division, comparison thresholds)
//! - UndoStackIssue variant classification (is_error, severity icons)
//! - BatchCommand execute/rollback sequencing
//! - Command describe() string content
//! - MoveEntityCommand try_merge (entity identity)
//! - RotateEntityCommand fields (x/y/z mapping)
//! - ScaleEntityCommand single value
//! - EditHealthCommand / EditTeamCommand / EditAmmoCommand redo/undo
//! - Max size pruning arithmetic
//! - Push_executed vs execute behavior
//! - Recent commands / upcoming redos window logic

use astraweave_core::{IVec2, Team, World};
use aw_editor_lib::command::{
    BatchCommand, EditAmmoCommand, EditHealthCommand, EditTeamCommand, EditorCommand,
    MoveEntityCommand, RotateEntityCommand, ScaleEntityCommand, UndoStack, UndoStackIssue,
    UndoStackStats,
};

// ============================================================================
// Helpers
// ============================================================================

fn make_world_and_entity() -> (World, astraweave_core::Entity) {
    let mut w = World::new();
    let e = w.spawn("test", IVec2::new(0, 0), Team { id: 0 }, 100, 30);
    (w, e)
}

fn make_world_multi(n: usize) -> (World, Vec<astraweave_core::Entity>) {
    let mut w = World::new();
    let mut es = Vec::new();
    for i in 0..n {
        let e = w.spawn(
            &format!("E{}", i),
            IVec2::new(i as i32, i as i32),
            Team { id: 0 },
            100,
            30,
        );
        es.push(e);
    }
    (w, es)
}

// ============================================================================
// Section A: UndoStackStats — Utilization Boundary Conditions
// ============================================================================

#[test]
fn stats_utilization_zero_max_returns_zero() {
    // Mutation: max_size == 0 → division by zero guard
    let stats = UndoStackStats {
        total_commands: 5,
        undo_available: 3,
        redo_available: 2,
        max_size: 0,
        auto_merge_enabled: true,
    };
    assert_eq!(stats.utilization(), 0.0);
}

#[test]
fn stats_utilization_half_full() {
    let stats = UndoStackStats {
        total_commands: 5,
        undo_available: 5,
        redo_available: 0,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert!((stats.utilization() - 0.5).abs() < 0.001);
    assert!(!stats.is_near_capacity()); // 50% < 80%
}

#[test]
fn stats_utilization_at_eighty_percent_exact() {
    // Mutation: > 0.8 boundary — exactly 0.8 should NOT be near capacity
    let stats = UndoStackStats {
        total_commands: 8,
        undo_available: 8,
        redo_available: 0,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert!((stats.utilization() - 0.8).abs() < 0.001);
    assert!(!stats.is_near_capacity()); // Exactly 80% is NOT >80%
}

#[test]
fn stats_utilization_at_eighty_one_percent_near_capacity() {
    // 81% should be near capacity
    let stats = UndoStackStats {
        total_commands: 81,
        undo_available: 81,
        redo_available: 0,
        max_size: 100,
        auto_merge_enabled: true,
    };
    assert!(stats.utilization() > 0.8);
    assert!(stats.is_near_capacity());
}

#[test]
fn stats_is_empty_only_when_zero_commands() {
    let empty = UndoStackStats {
        total_commands: 0,
        undo_available: 0,
        redo_available: 0,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert!(empty.is_empty());

    let non_empty = UndoStackStats {
        total_commands: 1,
        undo_available: 1,
        redo_available: 0,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert!(!non_empty.is_empty());
}

#[test]
fn stats_can_undo_only_when_undo_available_gt_zero() {
    let no_undo = UndoStackStats {
        total_commands: 5,
        undo_available: 0,
        redo_available: 5,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert!(!no_undo.can_undo());

    let has_undo = UndoStackStats {
        total_commands: 5,
        undo_available: 1,
        redo_available: 4,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert!(has_undo.can_undo());
}

#[test]
fn stats_can_redo_only_when_redo_available_gt_zero() {
    let no_redo = UndoStackStats {
        total_commands: 5,
        undo_available: 5,
        redo_available: 0,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert!(!no_redo.can_redo());

    let has_redo = UndoStackStats {
        total_commands: 5,
        undo_available: 4,
        redo_available: 1,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert!(has_redo.can_redo());
}

#[test]
fn stats_remaining_capacity_arithmetic() {
    let stats = UndoStackStats {
        total_commands: 3,
        undo_available: 3,
        redo_available: 0,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert_eq!(stats.remaining_capacity(), 7); // 10 - 3

    // Saturating: can't go below zero
    let over = UndoStackStats {
        total_commands: 15,
        undo_available: 10,
        redo_available: 5,
        max_size: 10,
        auto_merge_enabled: true,
    };
    assert_eq!(over.remaining_capacity(), 0);
}

// ============================================================================
// Section B: UndoStackIssue — Classification & Display
// ============================================================================

#[test]
fn issue_is_error_only_at_capacity() {
    // Only AtCapacity is an error
    assert!(UndoStackIssue::AtCapacity.is_error());
    assert!(!UndoStackIssue::NearCapacity {
        utilization_percent: 90
    }
    .is_error());
    assert!(!UndoStackIssue::AutoMergeDisabled.is_error());
    assert!(!UndoStackIssue::NoHistory.is_error());
}

#[test]
fn issue_icons_are_distinct_per_variant() {
    let near = UndoStackIssue::NearCapacity {
        utilization_percent: 85,
    }
    .icon();
    let at = UndoStackIssue::AtCapacity.icon();
    let merge = UndoStackIssue::AutoMergeDisabled.icon();
    let no_hist = UndoStackIssue::NoHistory.icon();

    // Each icon is non-empty
    assert!(!near.is_empty());
    assert!(!at.is_empty());
    assert!(!merge.is_empty());
    assert!(!no_hist.is_empty());

    // AtCapacity icon is different from NearCapacity icon
    assert_ne!(near, at);
}

#[test]
fn issue_display_includes_percentage_for_near_capacity() {
    let issue = UndoStackIssue::NearCapacity {
        utilization_percent: 92,
    };
    let display = format!("{}", issue);
    assert!(
        display.contains("92"),
        "Should contain the utilization percentage: {}",
        display
    );
}

#[test]
fn issue_display_at_capacity_contains_capacity() {
    let display = format!("{}", UndoStackIssue::AtCapacity);
    assert!(display.to_lowercase().contains("capacity"));
}

#[test]
fn issue_display_auto_merge_disabled_mentions_merge() {
    let display = format!("{}", UndoStackIssue::AutoMergeDisabled);
    let lower = display.to_lowercase();
    assert!(lower.contains("merge") || lower.contains("auto"));
}

#[test]
fn issue_display_no_history_mentions_history() {
    let display = format!("{}", UndoStackIssue::NoHistory);
    let lower = display.to_lowercase();
    assert!(lower.contains("history") || lower.contains("undo") || lower.contains("recorded"));
}

// ============================================================================
// Section C: UndoStack — Cursor Mechanics & Pruning
// ============================================================================

#[test]
fn undo_stack_cursor_starts_at_zero() {
    let stack = UndoStack::new(10);
    assert_eq!(stack.cursor(), 0);
    assert_eq!(stack.len(), 0);
    assert!(stack.is_empty());
}

#[test]
fn undo_stack_single_execute_increments_cursor() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    assert_eq!(stack.cursor(), 1);
    assert_eq!(stack.len(), 1);
}

#[test]
fn undo_decrements_cursor_by_one() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(2, 2)),
            &mut w,
            None,
        )
        .unwrap();
    assert_eq!(stack.cursor(), 2);

    stack.undo(&mut w, None).unwrap();
    assert_eq!(stack.cursor(), 1);
}

#[test]
fn redo_increments_cursor_by_one() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack.undo(&mut w, None).unwrap();
    assert_eq!(stack.cursor(), 0);

    stack.redo(&mut w, None).unwrap();
    assert_eq!(stack.cursor(), 1);
}

#[test]
fn undo_at_zero_cursor_is_noop() {
    let (mut w, _e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);

    // Undo with empty stack should be OK, no crash
    stack.undo(&mut w, None).unwrap();
    assert_eq!(stack.cursor(), 0);
}

#[test]
fn redo_at_end_is_noop() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    // Cursor is at end, redo should do nothing
    stack.redo(&mut w, None).unwrap();
    assert_eq!(stack.cursor(), 1);
}

#[test]
fn execute_after_undo_discards_redo_history() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(2, 2)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(2, 2), IVec2::new(3, 3)),
            &mut w,
            None,
        )
        .unwrap();

    // Undo twice
    stack.undo(&mut w, None).unwrap();
    stack.undo(&mut w, None).unwrap();
    assert_eq!(stack.cursor(), 1);
    assert_eq!(stack.len(), 3); // All 3 still in stack

    // Execute new command → discards commands[1] and commands[2]
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(99, 99)),
            &mut w,
            None,
        )
        .unwrap();
    assert_eq!(stack.cursor(), 2);
    assert_eq!(stack.len(), 2); // Old redo history gone
    assert!(!stack.can_redo());
}

#[test]
fn max_size_pruning_removes_oldest() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(3);
    stack.set_auto_merge(false);

    for i in 0..5 {
        stack
            .execute(
                MoveEntityCommand::new(e, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                &mut w,
                None,
            )
            .unwrap();
    }

    // Should be capped at 3
    assert_eq!(stack.len(), 3);
    // Cursor should be at end (3)
    assert_eq!(stack.cursor(), 3);
    // Can undo 3 times
    assert_eq!(stack.undo_count(), 3);
    assert_eq!(stack.redo_count(), 0);
}

#[test]
fn max_size_one_always_keeps_latest() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(1);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(2, 2)),
            &mut w,
            None,
        )
        .unwrap();

    assert_eq!(stack.len(), 1);
    // The latest command should be the Move to (2,2)
    assert_eq!(
        stack.undo_description().unwrap(),
        format!("Move Entity {:?}", e)
    );

    stack.undo(&mut w, None).unwrap();
    assert_eq!(w.pose(e).unwrap().pos, IVec2::new(1, 1));
}

#[test]
fn undo_count_and_redo_count_are_complementary() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    for i in 0..5 {
        stack
            .execute(
                MoveEntityCommand::new(e, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                &mut w,
                None,
            )
            .unwrap();
    }

    assert_eq!(stack.undo_count(), 5);
    assert_eq!(stack.redo_count(), 0);

    stack.undo(&mut w, None).unwrap();
    stack.undo(&mut w, None).unwrap();
    assert_eq!(stack.undo_count(), 3);
    assert_eq!(stack.redo_count(), 2);
    assert_eq!(stack.undo_count() + stack.redo_count(), stack.len());
}

// ============================================================================
// Section D: Undo Stack validate() logic
// ============================================================================

#[test]
fn validate_empty_stack_reports_no_history() {
    let stack = UndoStack::new(10);
    let issues = stack.validate();
    assert!(issues
        .iter()
        .any(|i| matches!(i, UndoStackIssue::NoHistory)));
}

#[test]
fn validate_auto_merge_disabled_reports_issue() {
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);
    let issues = stack.validate();
    assert!(issues
        .iter()
        .any(|i| matches!(i, UndoStackIssue::AutoMergeDisabled)));
}

#[test]
fn validate_at_full_capacity_reports_at_capacity() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(2);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(2, 2)),
            &mut w,
            None,
        )
        .unwrap();

    let issues = stack.validate();
    assert!(issues
        .iter()
        .any(|i| matches!(i, UndoStackIssue::AtCapacity)));
}

#[test]
fn validate_near_capacity_threshold_is_80_percent() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    // Fill to 80% (8 out of 10) — should NOT report NearCapacity yet
    for i in 0..8 {
        stack
            .execute(
                MoveEntityCommand::new(e, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                &mut w,
                None,
            )
            .unwrap();
    }

    let issues = stack.validate();
    // At 80%, should NOT have NearCapacity (needs > 80)
    // But may have AtCapacity if at full
    assert!(!issues
        .iter()
        .any(|i| matches!(i, UndoStackIssue::NearCapacity { .. })));

    // Fill to 90% (9 out of 10) — should report NearCapacity
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(8, 8), IVec2::new(9, 9)),
            &mut w,
            None,
        )
        .unwrap();

    let issues = stack.validate();
    assert!(issues
        .iter()
        .any(|i| matches!(i, UndoStackIssue::NearCapacity { .. })));
}

#[test]
fn is_valid_means_no_error_issues() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    assert!(stack.is_valid()); // Has commands, auto-merge on, not at capacity
}

#[test]
fn is_valid_false_when_at_capacity() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(1);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    // At capacity → AtCapacity issue → is_error → is_valid returns false
    assert!(!stack.is_valid());
}

// ============================================================================
// Section E: Recent Commands & Upcoming Redos
// ============================================================================

#[test]
fn recent_commands_returns_last_n() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(20);
    stack.set_auto_merge(false);

    for i in 0..10 {
        stack
            .execute(
                MoveEntityCommand::new(e, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                &mut w,
                None,
            )
            .unwrap();
    }

    let recent = stack.recent_commands(3);
    assert_eq!(recent.len(), 3);

    let recent_all = stack.recent_commands(100);
    assert_eq!(recent_all.len(), 10); // Only 10 exists
}

#[test]
fn recent_commands_when_empty_returns_empty() {
    let stack = UndoStack::new(10);
    let recent = stack.recent_commands(5);
    assert!(recent.is_empty());
}

#[test]
fn upcoming_redos_returns_correct_count() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    for i in 0..5 {
        stack
            .execute(
                MoveEntityCommand::new(e, IVec2::new(i, i), IVec2::new(i + 1, i + 1)),
                &mut w,
                None,
            )
            .unwrap();
    }

    // Undo all 5
    for _ in 0..5 {
        stack.undo(&mut w, None).unwrap();
    }

    let redos = stack.upcoming_redos(3);
    assert_eq!(redos.len(), 3);

    let redos = stack.upcoming_redos(100);
    assert_eq!(redos.len(), 5);
}

#[test]
fn undo_description_is_none_when_empty() {
    let stack = UndoStack::new(10);
    assert!(stack.undo_description().is_none());
}

#[test]
fn redo_description_is_none_when_at_end() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    assert!(stack.redo_description().is_none());
}

#[test]
fn undo_description_returns_last_command_desc() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    let desc = stack.undo_description().unwrap();
    assert!(desc.contains("Move"));
}

#[test]
fn redo_description_returns_next_command_desc() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack.undo(&mut w, None).unwrap();

    let desc = stack.redo_description().unwrap();
    assert!(desc.contains("Move"));
}

// ============================================================================
// Section F: MoveEntityCommand — Position Mapping & Merge
// ============================================================================

#[test]
fn move_execute_sets_new_pos() {
    let (mut w, e) = make_world_and_entity();
    let mut cmd = MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(42, 99));
    cmd.execute(&mut w, None).unwrap();
    assert_eq!(w.pose(e).unwrap().pos, IVec2::new(42, 99));
}

#[test]
fn move_undo_restores_old_pos() {
    let (mut w, e) = make_world_and_entity();
    let mut cmd = MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(42, 99));
    cmd.execute(&mut w, None).unwrap();
    cmd.undo(&mut w, None).unwrap();
    assert_eq!(w.pose(e).unwrap().pos, IVec2::new(0, 0));
}

#[test]
fn move_describe_contains_entity_id() {
    let (_, e) = make_world_and_entity();
    let cmd = MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1));
    let desc = cmd.describe();
    assert!(desc.contains("Move"));
    assert!(desc.contains("Entity"));
}

#[test]
fn move_merge_same_entity_updates_new_pos_keeps_old() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(true);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(5, 5)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(5, 5), IVec2::new(10, 10)),
            &mut w,
            None,
        )
        .unwrap();

    // All merged into 1
    assert_eq!(stack.len(), 1);
    assert_eq!(w.pose(e).unwrap().pos, IVec2::new(10, 10));

    // Undo should go back to original (0,0) not intermediate
    stack.undo(&mut w, None).unwrap();
    assert_eq!(w.pose(e).unwrap().pos, IVec2::new(0, 0));
}

#[test]
fn move_no_merge_different_entities() {
    let (mut w, es) = make_world_multi(2);
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(true);

    stack
        .execute(
            MoveEntityCommand::new(es[0], IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(es[1], IVec2::new(1, 1), IVec2::new(2, 2)),
            &mut w,
            None,
        )
        .unwrap();

    // Different entities → no merge
    assert_eq!(stack.len(), 2);
}

#[test]
fn move_execute_missing_entity_returns_error() {
    let mut w = World::new();
    let fake_entity = 9999;
    let mut cmd = MoveEntityCommand::new(fake_entity, IVec2::new(0, 0), IVec2::new(1, 1));
    assert!(cmd.execute(&mut w, None).is_err());
}

#[test]
fn move_undo_missing_entity_returns_error() {
    let mut w = World::new();
    let fake_entity = 9999;
    let mut cmd = MoveEntityCommand::new(fake_entity, IVec2::new(0, 0), IVec2::new(1, 1));
    assert!(cmd.undo(&mut w, None).is_err());
}

// ============================================================================
// Section G: RotateEntityCommand — X/Y/Z Mapping
// ============================================================================

#[test]
fn rotate_execute_sets_all_three_axes() {
    let (mut w, e) = make_world_and_entity();
    let mut cmd = RotateEntityCommand::new(e, (0.0, 0.0, 0.0), (0.5, 1.0, 1.5));
    cmd.execute(&mut w, None).unwrap();

    let pose = w.pose(e).unwrap();
    assert!((pose.rotation_x - 0.5).abs() < 0.001);
    assert!((pose.rotation - 1.0).abs() < 0.001); // Y-axis stored in `rotation`
    assert!((pose.rotation_z - 1.5).abs() < 0.001);
}

#[test]
fn rotate_undo_restores_all_three_axes() {
    let (mut w, e) = make_world_and_entity();
    // Set initial rotation
    if let Some(pose) = w.pose_mut(e) {
        pose.rotation_x = 0.1;
        pose.rotation = 0.2;
        pose.rotation_z = 0.3;
    }

    let mut cmd = RotateEntityCommand::new(e, (0.1, 0.2, 0.3), (1.0, 2.0, 3.0));
    cmd.execute(&mut w, None).unwrap();
    cmd.undo(&mut w, None).unwrap();

    let pose = w.pose(e).unwrap();
    assert!((pose.rotation_x - 0.1).abs() < 0.001);
    assert!((pose.rotation - 0.2).abs() < 0.001);
    assert!((pose.rotation_z - 0.3).abs() < 0.001);
}

#[test]
fn rotate_describe_contains_entity() {
    let (_, e) = make_world_and_entity();
    let cmd = RotateEntityCommand::new(e, (0.0, 0.0, 0.0), (1.0, 1.0, 1.0));
    assert!(cmd.describe().contains("Rotate"));
}

#[test]
fn rotate_merge_same_entity() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(true);

    stack
        .execute(
            RotateEntityCommand::new(e, (0.0, 0.0, 0.0), (0.5, 0.5, 0.5)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            RotateEntityCommand::new(e, (0.5, 0.5, 0.5), (1.0, 1.0, 1.0)),
            &mut w,
            None,
        )
        .unwrap();

    assert_eq!(stack.len(), 1); // Merged
}

#[test]
fn rotate_no_merge_different_entities() {
    let (mut w, es) = make_world_multi(2);
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(true);

    stack
        .execute(
            RotateEntityCommand::new(es[0], (0.0, 0.0, 0.0), (1.0, 0.0, 0.0)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            RotateEntityCommand::new(es[1], (0.0, 0.0, 0.0), (0.0, 1.0, 0.0)),
            &mut w,
            None,
        )
        .unwrap();

    assert_eq!(stack.len(), 2);
}

// ============================================================================
// Section H: ScaleEntityCommand
// ============================================================================

#[test]
fn scale_execute_sets_scale() {
    let (mut w, e) = make_world_and_entity();
    let mut cmd = ScaleEntityCommand::new(e, 1.0, 3.5);
    cmd.execute(&mut w, None).unwrap();
    assert!((w.pose(e).unwrap().scale - 3.5).abs() < 0.001);
}

#[test]
fn scale_undo_restores_scale() {
    let (mut w, e) = make_world_and_entity();
    let mut cmd = ScaleEntityCommand::new(e, 1.0, 3.5);
    cmd.execute(&mut w, None).unwrap();
    cmd.undo(&mut w, None).unwrap();
    assert!((w.pose(e).unwrap().scale - 1.0).abs() < 0.001);
}

#[test]
fn scale_describe_contains_scale() {
    let (_, e) = make_world_and_entity();
    let cmd = ScaleEntityCommand::new(e, 1.0, 2.0);
    assert!(cmd.describe().contains("Scale"));
}

#[test]
fn scale_merge_same_entity() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(true);

    stack
        .execute(ScaleEntityCommand::new(e, 1.0, 1.5), &mut w, None)
        .unwrap();
    stack
        .execute(ScaleEntityCommand::new(e, 1.5, 2.0), &mut w, None)
        .unwrap();
    stack
        .execute(ScaleEntityCommand::new(e, 2.0, 3.0), &mut w, None)
        .unwrap();

    assert_eq!(stack.len(), 1);
}

// ============================================================================
// Section I: EditHealthCommand / EditTeamCommand / EditAmmoCommand
// ============================================================================

#[test]
fn edit_health_push_and_undo_restores_old_hp() {
    let mut w = World::new();
    let e = w.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

    let mut stack = UndoStack::new(10);
    stack.push_executed(EditHealthCommand::new(e, 100, 50));

    stack.undo(&mut w, None).unwrap();
    assert_eq!(w.health(e).unwrap().hp, 100);
}

#[test]
fn edit_health_redo_applies_new_hp() {
    let mut w = World::new();
    let e = w.spawn("Player", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

    let mut stack = UndoStack::new(10);
    stack.push_executed(EditHealthCommand::new(e, 100, 50));

    stack.undo(&mut w, None).unwrap();
    stack.redo(&mut w, None).unwrap();
    assert_eq!(w.health(e).unwrap().hp, 50);
}

#[test]
fn edit_health_describe_contains_entity_id() {
    let cmd = EditHealthCommand::new(42, 100, 50);
    let desc = cmd.describe();
    assert!(desc.contains("Health"));
    assert!(desc.contains("42"));
}

#[test]
fn edit_team_undo_restores_old_team() {
    let mut w = World::new();
    let e = w.spawn("Enemy", IVec2::new(0, 0), Team { id: 2 }, 100, 30);

    let mut stack = UndoStack::new(10);
    stack.push_executed(EditTeamCommand::new(e, Team { id: 2 }, Team { id: 0 }));

    stack.undo(&mut w, None).unwrap();
    assert_eq!(w.team(e).unwrap().id, 2);
}

#[test]
fn edit_team_redo_applies_new_team() {
    let mut w = World::new();
    let e = w.spawn("Enemy", IVec2::new(0, 0), Team { id: 2 }, 100, 30);

    let mut stack = UndoStack::new(10);
    stack.push_executed(EditTeamCommand::new(e, Team { id: 2 }, Team { id: 0 }));

    stack.undo(&mut w, None).unwrap();
    stack.redo(&mut w, None).unwrap();
    assert_eq!(w.team(e).unwrap().id, 0);
}

#[test]
fn edit_team_describe_contains_entity_id() {
    let cmd = EditTeamCommand::new(7, Team { id: 0 }, Team { id: 1 });
    assert!(cmd.describe().contains("Team"));
    assert!(cmd.describe().contains("7"));
}

#[test]
fn edit_ammo_undo_restores_old_rounds() {
    let mut w = World::new();
    let e = w.spawn("Shooter", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

    let mut stack = UndoStack::new(10);
    stack.push_executed(EditAmmoCommand::new(e, 30, 10));

    stack.undo(&mut w, None).unwrap();
    assert_eq!(w.ammo(e).unwrap().rounds, 30);
}

#[test]
fn edit_ammo_redo_applies_new_rounds() {
    let mut w = World::new();
    let e = w.spawn("Shooter", IVec2::new(0, 0), Team { id: 0 }, 100, 30);

    let mut stack = UndoStack::new(10);
    stack.push_executed(EditAmmoCommand::new(e, 30, 10));

    stack.undo(&mut w, None).unwrap();
    stack.redo(&mut w, None).unwrap();
    assert_eq!(w.ammo(e).unwrap().rounds, 10);
}

#[test]
fn edit_ammo_describe_contains_entity_id() {
    let cmd = EditAmmoCommand::new(99, 30, 10);
    assert!(cmd.describe().contains("Ammo"));
    assert!(cmd.describe().contains("99"));
}

// ============================================================================
// Section J: BatchCommand — Execute, Undo, Rollback
// ============================================================================

#[test]
fn batch_command_executes_all_in_order() {
    let (mut w, es) = make_world_multi(3);

    let commands: Vec<Box<dyn EditorCommand>> = vec![
        MoveEntityCommand::new(es[0], IVec2::new(0, 0), IVec2::new(10, 0)),
        MoveEntityCommand::new(es[1], IVec2::new(1, 1), IVec2::new(0, 20)),
        MoveEntityCommand::new(es[2], IVec2::new(2, 2), IVec2::new(0, 0)),
    ];

    let mut batch = BatchCommand::new(commands, "Move 3".to_string());
    batch.execute(&mut w, None).unwrap();

    assert_eq!(w.pose(es[0]).unwrap().pos, IVec2::new(10, 0));
    assert_eq!(w.pose(es[1]).unwrap().pos, IVec2::new(0, 20));
    assert_eq!(w.pose(es[2]).unwrap().pos, IVec2::new(0, 0));
}

#[test]
fn batch_command_undo_reverts_in_reverse_order() {
    let (mut w, es) = make_world_multi(3);

    let commands: Vec<Box<dyn EditorCommand>> = vec![
        MoveEntityCommand::new(es[0], IVec2::new(0, 0), IVec2::new(10, 10)),
        MoveEntityCommand::new(es[1], IVec2::new(1, 1), IVec2::new(20, 20)),
        MoveEntityCommand::new(es[2], IVec2::new(2, 2), IVec2::new(30, 30)),
    ];

    let mut batch = BatchCommand::new(commands, "Move 3".to_string());
    batch.execute(&mut w, None).unwrap();
    batch.undo(&mut w, None).unwrap();

    assert_eq!(w.pose(es[0]).unwrap().pos, IVec2::new(0, 0));
    assert_eq!(w.pose(es[1]).unwrap().pos, IVec2::new(1, 1));
    assert_eq!(w.pose(es[2]).unwrap().pos, IVec2::new(2, 2));
}

#[test]
fn batch_command_empty_is_noop() {
    let mut w = World::new();
    let commands: Vec<Box<dyn EditorCommand>> = vec![];
    let mut batch = BatchCommand::new(commands, "Empty".to_string());

    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);

    batch.execute(&mut w, None).unwrap();
    batch.undo(&mut w, None).unwrap();
}

#[test]
fn batch_command_describe_returns_description() {
    let batch = BatchCommand::new(vec![], "Custom Batch Desc".to_string());
    assert_eq!(batch.describe(), "Custom Batch Desc");
}

#[test]
fn batch_from_moves_creates_correct_count() {
    let (_, es) = make_world_multi(4);
    let moves = vec![
        (es[0], IVec2::new(0, 0), IVec2::new(1, 1)),
        (es[1], IVec2::new(1, 1), IVec2::new(2, 2)),
        (es[2], IVec2::new(2, 2), IVec2::new(3, 3)),
        (es[3], IVec2::new(3, 3), IVec2::new(4, 4)),
    ];

    let batch = BatchCommand::from_moves(moves);
    assert_eq!(batch.len(), 4);
    assert!(!batch.is_empty());
    assert!(batch.describe().contains("4"));
}

#[test]
fn execute_batch_on_stack_is_one_entry() {
    let (mut w, es) = make_world_multi(2);
    let mut stack = UndoStack::new(10);

    let commands: Vec<Box<dyn EditorCommand>> = vec![
        MoveEntityCommand::new(es[0], IVec2::new(0, 0), IVec2::new(5, 5)),
        MoveEntityCommand::new(es[1], IVec2::new(1, 1), IVec2::new(6, 6)),
    ];

    stack
        .execute_batch(commands, &mut w, "Batch".to_string(), None)
        .unwrap();
    assert_eq!(stack.len(), 1);

    stack.undo(&mut w, None).unwrap();
    assert_eq!(w.pose(es[0]).unwrap().pos, IVec2::new(0, 0));
    assert_eq!(w.pose(es[1]).unwrap().pos, IVec2::new(1, 1));
}

#[test]
fn execute_batch_empty_does_not_add_to_stack() {
    let mut w = World::new();
    let mut stack = UndoStack::new(10);

    stack
        .execute_batch(vec![], &mut w, "Empty batch".to_string(), None)
        .unwrap();
    assert_eq!(stack.len(), 0);
}

// ============================================================================
// Section K: push_executed vs execute
// ============================================================================

#[test]
fn push_executed_does_not_re_execute_command() {
    let (mut w, e) = make_world_and_entity();
    // Manually set position to (5,5)
    w.pose_mut(e).unwrap().pos = IVec2::new(5, 5);

    let mut stack = UndoStack::new(10);
    // Push a command that says old=(0,0) new=(5,5) but don't execute it
    stack.push_executed(MoveEntityCommand::new(
        e,
        IVec2::new(0, 0),
        IVec2::new(5, 5),
    ));

    // Position should still be (5,5) — push_executed doesn't call execute
    assert_eq!(w.pose(e).unwrap().pos, IVec2::new(5, 5));

    // But undo should work
    stack.undo(&mut w, None).unwrap();
    assert_eq!(w.pose(e).unwrap().pos, IVec2::new(0, 0));
}

#[test]
fn push_executed_respects_auto_merge() {
    let (_w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(true);

    stack.push_executed(MoveEntityCommand::new(
        e,
        IVec2::new(0, 0),
        IVec2::new(1, 1),
    ));
    stack.push_executed(MoveEntityCommand::new(
        e,
        IVec2::new(1, 1),
        IVec2::new(2, 2),
    ));

    // Should merge since same entity
    assert_eq!(stack.len(), 1);
}

#[test]
fn push_executed_truncates_redo_history() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(2, 2)),
            &mut w,
            None,
        )
        .unwrap();

    // Undo once → can redo
    stack.undo(&mut w, None).unwrap();
    assert!(stack.can_redo());

    // push_executed discards redo history
    stack.push_executed(MoveEntityCommand::new(
        e,
        IVec2::new(1, 1),
        IVec2::new(99, 99),
    ));
    assert!(!stack.can_redo());
}

#[test]
fn push_executed_prunes_at_max_size() {
    let (_, e) = make_world_and_entity();
    let mut stack = UndoStack::new(3);
    stack.set_auto_merge(false);

    for i in 0..5 {
        stack.push_executed(MoveEntityCommand::new(
            e,
            IVec2::new(i, i),
            IVec2::new(i + 1, i + 1),
        ));
    }

    assert_eq!(stack.len(), 3);
}

// ============================================================================
// Section L: Stack Accessors & State
// ============================================================================

#[test]
fn max_size_accessor() {
    let stack = UndoStack::new(42);
    assert_eq!(stack.max_size(), 42);
}

#[test]
fn auto_merge_accessor() {
    let mut stack = UndoStack::new(10);
    assert!(stack.is_auto_merge_enabled());

    stack.set_auto_merge(false);
    assert!(!stack.is_auto_merge_enabled());

    stack.set_auto_merge(true);
    assert!(stack.is_auto_merge_enabled());
}

#[test]
fn clear_resets_everything() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(2, 2)),
            &mut w,
            None,
        )
        .unwrap();

    stack.clear();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);
    assert_eq!(stack.cursor(), 0);
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());
}

#[test]
fn stats_reflect_current_state() {
    let (mut w, e) = make_world_and_entity();
    let mut stack = UndoStack::new(10);
    stack.set_auto_merge(false);

    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(0, 0), IVec2::new(1, 1)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(1, 1), IVec2::new(2, 2)),
            &mut w,
            None,
        )
        .unwrap();
    stack
        .execute(
            MoveEntityCommand::new(e, IVec2::new(2, 2), IVec2::new(3, 3)),
            &mut w,
            None,
        )
        .unwrap();

    stack.undo(&mut w, None).unwrap();

    let stats = stack.stats();
    assert_eq!(stats.total_commands, 3);
    assert_eq!(stats.undo_available, 2);
    assert_eq!(stats.redo_available, 1);
    assert_eq!(stats.max_size, 10);
    assert!(stats.auto_merge_enabled == false);
}
