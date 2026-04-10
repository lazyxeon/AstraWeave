//! Mutation-resistant tests for aw_editor
//!
//! These tests verify exact computed values to catch mutations during
//! `cargo mutants` testing. Each test asserts specific values that would
//! fail if source code arithmetic/logic is mutated.

use aw_editor_lib::command::{UndoStackIssue, UndoStackStats};
use aw_editor_lib::ui::{TaskCategory, ToastLevel};

// =============================================================================
// UNDO STACK STATS TESTS
// =============================================================================

mod undo_stack_stats_tests {
    use super::*;

    // -------------------------------------------------------------------------
    // utilization() tests - formula: total_commands / max_size
    // -------------------------------------------------------------------------

    #[test]
    fn utilization_0_of_100_is_0() {
        let stats = UndoStackStats {
            total_commands: 0,
            undo_available: 0,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        let util = stats.utilization();
        assert!(
            (util - 0.0).abs() < 1e-6,
            "0/100 should be 0.0, got {}",
            util
        );
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
        let util = stats.utilization();
        assert!(
            (util - 0.5).abs() < 1e-6,
            "50/100 should be 0.5, got {}",
            util
        );
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
        let util = stats.utilization();
        assert!(
            (util - 1.0).abs() < 1e-6,
            "100/100 should be 1.0, got {}",
            util
        );
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
        let util = stats.utilization();
        assert!(
            (util - 0.5).abs() < 1e-6,
            "25/50 should be 0.5, got {}",
            util
        );
    }

    #[test]
    fn utilization_with_zero_max_size_is_0() {
        let stats = UndoStackStats {
            total_commands: 10,
            undo_available: 10,
            redo_available: 0,
            max_size: 0,
            auto_merge_enabled: true,
        };
        let util = stats.utilization();
        assert!(
            (util - 0.0).abs() < 1e-6,
            "with max_size=0 should return 0.0, got {}",
            util
        );
    }

    // -------------------------------------------------------------------------
    // is_near_capacity() tests - true when utilization > 0.8
    // -------------------------------------------------------------------------

    #[test]
    fn is_near_capacity_at_79_percent_is_false() {
        let stats = UndoStackStats {
            total_commands: 79,
            undo_available: 79,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(!stats.is_near_capacity(), "79% should not be near capacity");
    }

    #[test]
    fn is_near_capacity_at_80_percent_is_false() {
        let stats = UndoStackStats {
            total_commands: 80,
            undo_available: 80,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        // Exactly 0.8 is NOT > 0.8, so false
        assert!(
            !stats.is_near_capacity(),
            "80% (exactly) should not be near capacity"
        );
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
        assert!(stats.is_near_capacity(), "81% should be near capacity");
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
        assert!(stats.is_near_capacity(), "100% should be near capacity");
    }

    // -------------------------------------------------------------------------
    // is_empty() tests
    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // can_undo() / can_redo() tests
    // -------------------------------------------------------------------------

    #[test]
    fn can_undo_with_0_is_false() {
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
    fn can_undo_with_1_is_true() {
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
    fn can_redo_with_0_is_false() {
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
    fn can_redo_with_1_is_true() {
        let stats = UndoStackStats {
            total_commands: 1,
            undo_available: 0,
            redo_available: 1,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert!(stats.can_redo());
    }

    // -------------------------------------------------------------------------
    // remaining_capacity() tests - formula: max_size - total_commands
    // -------------------------------------------------------------------------

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
    fn remaining_capacity_saturates_at_0() {
        // Edge case: total_commands > max_size (shouldn't happen, but test saturation)
        let stats = UndoStackStats {
            total_commands: 150,
            undo_available: 150,
            redo_available: 0,
            max_size: 100,
            auto_merge_enabled: true,
        };
        assert_eq!(
            stats.remaining_capacity(),
            0,
            "should saturate at 0, not underflow"
        );
    }
}

// =============================================================================
// UNDO STACK ISSUE TESTS
// =============================================================================

mod undo_stack_issue_tests {
    use super::*;

    // -------------------------------------------------------------------------
    // is_error() tests - only AtCapacity is an error
    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------
    // icon() tests
    // -------------------------------------------------------------------------

    #[test]
    fn near_capacity_icon_is_warning() {
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

    // -------------------------------------------------------------------------
    // Display formatting tests
    // -------------------------------------------------------------------------

    #[test]
    fn near_capacity_display_shows_percent() {
        let issue = UndoStackIssue::NearCapacity {
            utilization_percent: 85,
        };
        assert_eq!(format!("{}", issue), "Undo stack 85% full");
    }

    #[test]
    fn at_capacity_display() {
        let issue = UndoStackIssue::AtCapacity;
        assert_eq!(format!("{}", issue), "Undo stack at capacity");
    }
}

// =============================================================================
// TASK CATEGORY TESTS
// =============================================================================

mod task_category_tests {
    use super::*;

    #[test]
    fn all_categories_count_is_6() {
        assert_eq!(TaskCategory::all().len(), 6);
    }

    #[test]
    fn scene_loading_icon_is_folder() {
        assert_eq!(TaskCategory::SceneLoading.icon(), "[Open]");
    }

    #[test]
    fn asset_import_icon_is_inbox() {
        assert_eq!(TaskCategory::AssetImport.icon(), "📥");
    }

    #[test]
    fn build_icon_is_hammer() {
        assert_eq!(TaskCategory::Build.icon(), "🔨");
    }

    #[test]
    fn play_mode_icon_is_play() {
        assert_eq!(TaskCategory::PlayMode.icon(), ">");
    }

    #[test]
    fn export_icon_is_outbox() {
        assert_eq!(TaskCategory::Export.icon(), "📤");
    }

    #[test]
    fn other_icon_is_gear() {
        assert_eq!(TaskCategory::Other.icon(), "⚙️");
    }

    #[test]
    fn scene_loading_name_is_correct() {
        assert_eq!(TaskCategory::SceneLoading.name(), "Scene Loading");
    }

    #[test]
    fn build_name_is_correct() {
        assert_eq!(TaskCategory::Build.name(), "Build");
    }
}

// =============================================================================
// TOAST LEVEL TESTS
// =============================================================================

mod toast_level_tests {
    use super::*;

    #[test]
    fn all_levels_count_is_4() {
        assert_eq!(ToastLevel::all().len(), 4);
    }

    #[test]
    fn info_icon_is_info() {
        assert_eq!(ToastLevel::Info.icon(), "ℹ️");
    }

    #[test]
    fn success_icon_is_checkmark() {
        assert_eq!(ToastLevel::Success.icon(), "✅");
    }

    #[test]
    fn warning_icon_is_warning() {
        assert_eq!(ToastLevel::Warning.icon(), "⚠️");
    }

    #[test]
    fn error_icon_is_x() {
        assert_eq!(ToastLevel::Error.icon(), "❌");
    }

    #[test]
    fn info_name_is_info() {
        assert_eq!(ToastLevel::Info.name(), "Info");
    }

    #[test]
    fn success_name_is_success() {
        assert_eq!(ToastLevel::Success.name(), "Success");
    }

    #[test]
    fn warning_name_is_warning() {
        assert_eq!(ToastLevel::Warning.name(), "Warning");
    }

    #[test]
    fn error_name_is_error() {
        assert_eq!(ToastLevel::Error.name(), "Error");
    }

    // -------------------------------------------------------------------------
    // is_problem() tests
    // -------------------------------------------------------------------------

    #[test]
    fn info_is_not_problem() {
        assert!(!ToastLevel::Info.is_problem());
    }

    #[test]
    fn success_is_not_problem() {
        assert!(!ToastLevel::Success.is_problem());
    }

    #[test]
    fn warning_is_problem() {
        assert!(ToastLevel::Warning.is_problem());
    }

    #[test]
    fn error_is_problem() {
        assert!(ToastLevel::Error.is_problem());
    }

    // -------------------------------------------------------------------------
    // is_success() tests
    // -------------------------------------------------------------------------

    #[test]
    fn only_success_is_success() {
        assert!(!ToastLevel::Info.is_success());
        assert!(ToastLevel::Success.is_success());
        assert!(!ToastLevel::Warning.is_success());
        assert!(!ToastLevel::Error.is_success());
    }

    // -------------------------------------------------------------------------
    // severity() tests - 0=info, 1=success, 2=warning, 3=error
    // -------------------------------------------------------------------------

    #[test]
    fn info_severity_is_0() {
        assert_eq!(ToastLevel::Info.severity(), 0);
    }

    #[test]
    fn success_severity_is_1() {
        assert_eq!(ToastLevel::Success.severity(), 1);
    }

    #[test]
    fn warning_severity_is_2() {
        assert_eq!(ToastLevel::Warning.severity(), 2);
    }

    #[test]
    fn error_severity_is_3() {
        assert_eq!(ToastLevel::Error.severity(), 3);
    }
}
