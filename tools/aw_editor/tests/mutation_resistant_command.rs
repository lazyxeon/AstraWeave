//! Mutation-Resistant Tests: Command System
//!
//! Comprehensive tests for boundary conditions, comparison operators,
//! and boolean return paths to achieve ≥95% mutation kill rate.

use aw_editor_lib::command::{UndoStack, UndoStackIssue, UndoStackStats};

// =============================================================================
// UNDO STACK STATS - UTILIZATION() TESTS
// Formula: total_commands / max_size
// =============================================================================

mod utilization_tests {
    use super::*;

    #[test]
    fn utilization_0_of_100_is_exactly_0() {
        let stats = UndoStackStats {
            total_commands: 0,
            undo_available: 0,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn utilization_1_of_100_is_0_01() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 1,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 0.01).abs() < 1e-6);
    }

    #[test]
    fn utilization_50_of_100_is_0_5() {
        let stats = UndoStackStats {
            total_commands: 50,
            undo_available: 50,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn utilization_99_of_100_is_0_99() {
        let stats = UndoStackStats {
            total_commands: 99,
            undo_available: 99,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 0.99).abs() < 1e-6);
    }

    #[test]
    fn utilization_100_of_100_is_1_0() {
        let stats = UndoStackStats {
            total_commands: 100,
            undo_available: 100,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn utilization_1_of_1_is_1_0() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 1,
            redo_available: 0,
            max_size: 1,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn utilization_25_of_50_is_0_5() {
        let stats = UndoStackStats {
            total_commands: 25,
            undo_available: 25,
            redo_available: 0,
            max_size: 50,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn utilization_with_zero_max_size_returns_0() {
        let stats = UndoStackStats {
            total_commands: 10,
            undo_available: 10,
            redo_available: 0,
            max_size: 0,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn utilization_33_of_100_is_0_33() {
        let stats = UndoStackStats {
            total_commands: 33,
            undo_available: 33,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 0.33).abs() < 1e-6);
    }

    #[test]
    fn utilization_67_of_100_is_0_67() {
        let stats = UndoStackStats {
            total_commands: 67,
            undo_available: 67,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!((stats.utilization() - 0.67).abs() < 1e-6);
    }
}

// =============================================================================
// UNDO STACK STATS - IS_NEAR_CAPACITY() TESTS
// Threshold: utilization > 0.8
// =============================================================================

mod is_near_capacity_tests {
    use super::*;

    #[test]
    fn is_near_capacity_at_0_percent_is_false() {
        let stats = UndoStackStats {
            total_commands: 0,
            undo_available: 0,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.is_near_capacity());
    }

    #[test]
    fn is_near_capacity_at_50_percent_is_false() {
        let stats = UndoStackStats {
            total_commands: 50,
            undo_available: 50,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.is_near_capacity());
    }

    #[test]
    fn is_near_capacity_at_79_percent_is_false() {
        let stats = UndoStackStats {
            total_commands: 79,
            undo_available: 79,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.is_near_capacity());
    }

    #[test]
    fn is_near_capacity_at_80_percent_exactly_is_false() {
        // 80/100 = 0.8, which is NOT > 0.8
        let stats = UndoStackStats {
            total_commands: 80,
            undo_available: 80,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.is_near_capacity());
    }

    #[test]
    fn is_near_capacity_at_81_percent_is_true() {
        let stats = UndoStackStats {
            total_commands: 81,
            undo_available: 81,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.is_near_capacity());
    }

    #[test]
    fn is_near_capacity_at_90_percent_is_true() {
        let stats = UndoStackStats {
            total_commands: 90,
            undo_available: 90,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.is_near_capacity());
    }

    #[test]
    fn is_near_capacity_at_100_percent_is_true() {
        let stats = UndoStackStats {
            total_commands: 100,
            undo_available: 100,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.is_near_capacity());
    }

    #[test]
    fn is_near_capacity_with_smaller_max_size() {
        // 9/10 = 0.9 > 0.8
        let stats = UndoStackStats {
            total_commands: 9,
            undo_available: 9,
            redo_available: 0,
            max_size: 10,
            auto_merge_enabled: true,
        };
        assert!(stats.is_near_capacity());
    }

    #[test]
    fn is_near_capacity_with_zero_max_size_is_false() {
        let stats = UndoStackStats {
            total_commands: 5,
            undo_available: 5,
            redo_available: 0,
            max_size: 0,
            auto_merge_enabled: true,
        };
        assert!(!stats.is_near_capacity());
    }
}

// =============================================================================
// UNDO STACK STATS - IS_EMPTY() TESTS
// =============================================================================

mod is_empty_tests {
    use super::*;

    #[test]
    fn is_empty_with_0_commands_is_true() {
        let stats = UndoStackStats {
            total_commands: 0,
            undo_available: 0,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.is_empty());
    }

    #[test]
    fn is_empty_with_1_command_is_false() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 1,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.is_empty());
    }

    #[test]
    fn is_empty_with_100_commands_is_false() {
        let stats = UndoStackStats {
            total_commands: 100,
            undo_available: 100,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.is_empty());
    }
}

// =============================================================================
// UNDO STACK STATS - CAN_UNDO() / CAN_REDO() TESTS
// =============================================================================

mod can_undo_redo_tests {
    use super::*;

    #[test]
    fn can_undo_with_0_available_is_false() {
        let stats = UndoStackStats {
            total_commands: 0,
            undo_available: 0,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.can_undo());
    }

    #[test]
    fn can_undo_with_1_available_is_true() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 1,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.can_undo());
    }

    #[test]
    fn can_undo_with_100_available_is_true() {
        let stats = UndoStackStats {
            total_commands: 100,
            undo_available: 100,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.can_undo());
    }

    #[test]
    fn can_redo_with_0_available_is_false() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 1,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.can_redo());
    }

    #[test]
    fn can_redo_with_1_available_is_true() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 0,
            redo_available: 1,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.can_redo());
    }

    #[test]
    fn can_redo_with_50_available_is_true() {
        let stats = UndoStackStats {
            total_commands: 50,
            undo_available: 0,
            redo_available: 50,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.can_redo());
    }
}

// =============================================================================
// UNDO STACK STATS - REMAINING_CAPACITY() TESTS
// Formula: max_size.saturating_sub(total_commands)
// =============================================================================

mod remaining_capacity_tests {
    use super::*;

    #[test]
    fn remaining_capacity_100_minus_0_is_100() {
        let stats = UndoStackStats {
            total_commands: 0,
            undo_available: 0,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert_eq!(stats.remaining_capacity(), 100);
    }

    #[test]
    fn remaining_capacity_100_minus_1_is_99() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 1,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert_eq!(stats.remaining_capacity(), 99);
    }

    #[test]
    fn remaining_capacity_100_minus_50_is_50() {
        let stats = UndoStackStats {
            total_commands: 50,
            undo_available: 50,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert_eq!(stats.remaining_capacity(), 50);
    }

    #[test]
    fn remaining_capacity_100_minus_99_is_1() {
        let stats = UndoStackStats {
            total_commands: 99,
            undo_available: 99,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert_eq!(stats.remaining_capacity(), 1);
    }

    #[test]
    fn remaining_capacity_100_minus_100_is_0() {
        let stats = UndoStackStats {
            total_commands: 100,
            undo_available: 100,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert_eq!(stats.remaining_capacity(), 0);
    }

    #[test]
    fn remaining_capacity_saturates_at_0_when_over_max() {
        let stats = UndoStackStats {
            total_commands: 150,
            undo_available: 150,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert_eq!(stats.remaining_capacity(), 0);
    }

    #[test]
    fn remaining_capacity_1_minus_0_is_1() {
        let stats = UndoStackStats {
            total_commands: 0,
            undo_available: 0,
            redo_available: 0,
            max_size: 1,
            auto_merge_enabled: true,
        };
        assert_eq!(stats.remaining_capacity(), 1);
    }

    #[test]
    fn remaining_capacity_1_minus_1_is_0() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 1,
            redo_available: 0,
            max_size: 1,
            auto_merge_enabled: true,
        };
        assert_eq!(stats.remaining_capacity(), 0);
    }
}

// =============================================================================
// UNDO STACK ISSUE - IS_ERROR() TESTS
// =============================================================================

mod issue_is_error_tests {
    use super::*;

    #[test]
    fn near_capacity_is_not_error() {
        let issue = UndoStackIssue::NearCapacity {
            utilization_percent: 85,
        };
        assert!(!issue.is_error());
    }

    #[test]
    fn at_capacity_is_error() {
        let issue = UndoStackIssue::AtCapacity;
        assert!(issue.is_error());
    }

    #[test]
    fn auto_merge_disabled_is_not_error() {
        let issue = UndoStackIssue::AutoMergeDisabled;
        assert!(!issue.is_error());
    }

    #[test]
    fn no_history_is_not_error() {
        let issue = UndoStackIssue::NoHistory;
        assert!(!issue.is_error());
    }
}

// =============================================================================
// UNDO STACK ISSUE - ICON() TESTS
// =============================================================================

mod issue_icon_tests {
    use super::*;

    #[test]
    fn near_capacity_icon_is_warning_emoji() {
        let issue = UndoStackIssue::NearCapacity {
            utilization_percent: 85,
        };
        assert_eq!(issue.icon(), "[!]");
    }

    #[test]
    fn at_capacity_icon_is_red_circle() {
        let issue = UndoStackIssue::AtCapacity;
        assert_eq!(issue.icon(), "[!!]");
    }

    #[test]
    fn auto_merge_disabled_icon_is_info() {
        let issue = UndoStackIssue::AutoMergeDisabled;
        assert_eq!(issue.icon(), "[i]");
    }

    #[test]
    fn no_history_icon_is_memo() {
        let issue = UndoStackIssue::NoHistory;
        assert_eq!(issue.icon(), "[--]");
    }
}

// =============================================================================
// UNDO STACK ISSUE - DISPLAY TESTS
// =============================================================================

mod issue_display_tests {
    use super::*;

    #[test]
    fn near_capacity_display_includes_percent() {
        let issue = UndoStackIssue::NearCapacity {
            utilization_percent: 85,
        };
        let display = format!("{}", issue);
        assert!(display.contains("85"));
        assert!(display.contains("%"));
    }

    #[test]
    fn at_capacity_display_mentions_capacity() {
        let issue = UndoStackIssue::AtCapacity;
        let display = format!("{}", issue);
        assert!(display.to_lowercase().contains("capacity"));
    }

    #[test]
    fn auto_merge_disabled_display_mentions_merge() {
        let issue = UndoStackIssue::AutoMergeDisabled;
        let display = format!("{}", issue);
        assert!(display.to_lowercase().contains("merge"));
    }

    #[test]
    fn no_history_display_mentions_history() {
        let issue = UndoStackIssue::NoHistory;
        let display = format!("{}", issue);
        assert!(display.to_lowercase().contains("history"));
    }
}

// =============================================================================
// UNDO STACK - BASIC OPERATIONS
// =============================================================================

mod undo_stack_basic_tests {
    use super::*;

    #[test]
    fn new_stack_has_zero_len() {
        let stack = UndoStack::new(100);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn new_stack_is_empty() {
        let stack = UndoStack::new(100);
        assert!(stack.is_empty());
    }

    #[test]
    fn new_stack_cursor_is_zero() {
        let stack = UndoStack::new(100);
        assert_eq!(stack.cursor(), 0);
    }

    #[test]
    fn new_stack_cannot_undo() {
        let stack = UndoStack::new(100);
        assert!(!stack.can_undo());
    }

    #[test]
    fn new_stack_cannot_redo() {
        let stack = UndoStack::new(100);
        assert!(!stack.can_redo());
    }

    #[test]
    fn new_stack_undo_count_is_zero() {
        let stack = UndoStack::new(100);
        assert_eq!(stack.undo_count(), 0);
    }

    #[test]
    fn new_stack_redo_count_is_zero() {
        let stack = UndoStack::new(100);
        assert_eq!(stack.redo_count(), 0);
    }

    #[test]
    fn new_stack_max_size_matches_constructor() {
        let stack = UndoStack::new(50);
        assert_eq!(stack.max_size(), 50);
    }

    #[test]
    fn new_stack_auto_merge_enabled_by_default() {
        let stack = UndoStack::new(100);
        assert!(stack.is_auto_merge_enabled());
    }

    #[test]
    fn new_stack_undo_description_is_none() {
        let stack = UndoStack::new(100);
        assert!(stack.undo_description().is_none());
    }

    #[test]
    fn new_stack_redo_description_is_none() {
        let stack = UndoStack::new(100);
        assert!(stack.redo_description().is_none());
    }
}

// =============================================================================
// UNDO STACK - STATS INTEGRATION
// =============================================================================

mod undo_stack_stats_integration_tests {
    use super::*;

    #[test]
    fn stats_total_commands_matches_len() {
        let stack = UndoStack::new(100);
        let stats = stack.stats();
        assert_eq!(stats.total_commands, stack.len());
    }

    #[test]
    fn stats_undo_available_matches_undo_count() {
        let stack = UndoStack::new(100);
        let stats = stack.stats();
        assert_eq!(stats.undo_available, stack.undo_count());
    }

    #[test]
    fn stats_redo_available_matches_redo_count() {
        let stack = UndoStack::new(100);
        let stats = stack.stats();
        assert_eq!(stats.redo_available, stack.redo_count());
    }

    #[test]
    fn stats_max_size_matches_stack() {
        let stack = UndoStack::new(75);
        let stats = stack.stats();
        assert_eq!(stats.max_size, 75);
    }

    #[test]
    fn stats_auto_merge_enabled_matches_stack() {
        let stack = UndoStack::new(100);
        let stats = stack.stats();
        assert_eq!(stats.auto_merge_enabled, stack.is_auto_merge_enabled());
    }
}

// =============================================================================
// UNDO STACK - VALIDATE TESTS
// =============================================================================

mod undo_stack_validate_tests {
    use super::*;

    #[test]
    fn empty_stack_validation_includes_no_history() {
        let stack = UndoStack::new(100);
        let issues = stack.validate();
        assert!(issues.contains(&UndoStackIssue::NoHistory));
    }

    #[test]
    fn new_stack_is_valid_returns_true() {
        let stack = UndoStack::new(100);
        // Empty stack only has NoHistory which is not an error
        assert!(stack.is_valid());
    }
}

// =============================================================================
// UNDO STACK - RECENT COMMANDS TESTS
// =============================================================================

mod recent_commands_tests {
    use super::*;

    #[test]
    fn recent_commands_on_empty_stack_returns_empty() {
        let stack = UndoStack::new(100);
        let recent = stack.recent_commands(5);
        assert!(recent.is_empty());
    }

    #[test]
    fn upcoming_redos_on_empty_stack_returns_empty() {
        let stack = UndoStack::new(100);
        let upcoming = stack.upcoming_redos(5);
        assert!(upcoming.is_empty());
    }
}
