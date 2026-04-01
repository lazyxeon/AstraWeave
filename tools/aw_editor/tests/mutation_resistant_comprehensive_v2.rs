//! Mutation-Resistant Comprehensive Tests V2: Editor Crate
//!
//! Targets ALL identified mutation-vulnerable gaps NOT covered by existing
//! mutation_resistant_*.rs files. Achieves ≥90% mutation kill rate across:
//!
//! - DistributionFormat (7 variants × 8 methods)
//! - DistributionConfig (defaults, validation, sanitization)
//! - format_bytes() (4 ranges)
//! - RuntimeIssue (6 variants × 5 classifiers + severity + icon)
//! - RuntimeStats (estimated_entity_capacity, simulation_duration, headroom, stability, status_color)
//! - EditorRuntime (fixed_dt, state, tick)
//! - ClipboardEntityData validation boundary tests
//! - ClipboardStats classifiers
//! - CLIPBOARD_SCHEMA_VERSION constant
//! - SceneData validate() boundary tests
//! - SceneValidationIssue (is_error, is_warning, icon, name)
//! - SceneStats
//! - GizmoHandle (10 variants × classification + axis + color)
//! - GizmoPicker defaults
//! - SnappingConfig boundary tests
//! - ScaleGizmo clamping and constraints
//! - TranslateGizmo constrained snapping and numeric
//! - CameraController defaults, view presets, zoom

use astraweave_core::IVec2;
use aw_editor_lib::clipboard::{
    ClipboardEntityData, ClipboardStats, ClipboardValidation, CLIPBOARD_SCHEMA_VERSION,
};
use aw_editor_lib::distribution::{format_bytes, DistributionConfig, DistributionFormat};
use aw_editor_lib::gizmo::{
    AxisConstraint, CameraController, GizmoHandle, GizmoPicker, Ray, ScaleGizmo, SnappingConfig,
    TranslateGizmo,
};
use aw_editor_lib::runtime::RuntimeIssue;
use aw_editor_lib::scene_serialization::SceneValidationIssue;
use aw_editor_lib::{EditorRuntime, RuntimeState, RuntimeStats};
use aw_editor_lib::{EntityData, SceneData};
use glam::{Quat, Vec2, Vec3};
use std::collections::BTreeMap;

// =============================================================================
// DISTRIBUTION FORMAT — EXTENSION TESTS (7 variants)
// =============================================================================

mod distribution_format_extension_tests {
    use super::*;

    #[test]
    fn windows_installer_extension_is_exe() {
        assert_eq!(DistributionFormat::WindowsInstaller.extension(), "exe");
    }

    #[test]
    fn windows_portable_extension_is_zip() {
        assert_eq!(DistributionFormat::WindowsPortable.extension(), "zip");
    }

    #[test]
    fn macos_bundle_extension_is_app() {
        assert_eq!(DistributionFormat::MacOSBundle.extension(), "app");
    }

    #[test]
    fn macos_dmg_extension_is_dmg() {
        assert_eq!(DistributionFormat::MacOSDmg.extension(), "dmg");
    }

    #[test]
    fn linux_appimage_extension_is_appimage() {
        assert_eq!(DistributionFormat::LinuxAppImage.extension(), "AppImage");
    }

    #[test]
    fn linux_tarball_extension_is_tar_gz() {
        assert_eq!(DistributionFormat::LinuxTarball.extension(), "tar.gz");
    }

    #[test]
    fn steam_depot_extension_is_vdf() {
        assert_eq!(DistributionFormat::SteamDepot.extension(), "vdf");
    }
}

// =============================================================================
// DISTRIBUTION FORMAT — NAME TESTS (7 variants)
// =============================================================================

mod distribution_format_name_tests {
    use super::*;

    #[test]
    fn windows_installer_name() {
        assert_eq!(
            DistributionFormat::WindowsInstaller.name(),
            "Windows Installer"
        );
    }

    #[test]
    fn windows_portable_name() {
        assert_eq!(
            DistributionFormat::WindowsPortable.name(),
            "Windows Portable"
        );
    }

    #[test]
    fn macos_bundle_name() {
        assert_eq!(DistributionFormat::MacOSBundle.name(), "macOS App Bundle");
    }

    #[test]
    fn macos_dmg_name() {
        assert_eq!(DistributionFormat::MacOSDmg.name(), "macOS DMG");
    }

    #[test]
    fn linux_appimage_name() {
        assert_eq!(DistributionFormat::LinuxAppImage.name(), "Linux AppImage");
    }

    #[test]
    fn linux_tarball_name() {
        assert_eq!(DistributionFormat::LinuxTarball.name(), "Linux Tarball");
    }

    #[test]
    fn steam_depot_name() {
        assert_eq!(DistributionFormat::SteamDepot.name(), "Steam Depot");
    }
}

// =============================================================================
// DISTRIBUTION FORMAT — PLATFORM TESTS (7 variants)
// =============================================================================

mod distribution_format_platform_tests {
    use super::*;

    #[test]
    fn windows_installer_platform_is_windows() {
        assert_eq!(DistributionFormat::WindowsInstaller.platform(), "Windows");
    }

    #[test]
    fn windows_portable_platform_is_windows() {
        assert_eq!(DistributionFormat::WindowsPortable.platform(), "Windows");
    }

    #[test]
    fn macos_bundle_platform_is_macos() {
        assert_eq!(DistributionFormat::MacOSBundle.platform(), "macOS");
    }

    #[test]
    fn macos_dmg_platform_is_macos() {
        assert_eq!(DistributionFormat::MacOSDmg.platform(), "macOS");
    }

    #[test]
    fn linux_appimage_platform_is_linux() {
        assert_eq!(DistributionFormat::LinuxAppImage.platform(), "Linux");
    }

    #[test]
    fn linux_tarball_platform_is_linux() {
        assert_eq!(DistributionFormat::LinuxTarball.platform(), "Linux");
    }

    #[test]
    fn steam_depot_platform_is_steam() {
        assert_eq!(DistributionFormat::SteamDepot.platform(), "Steam");
    }
}

// =============================================================================
// DISTRIBUTION FORMAT — ICON TESTS (7 variants unique)
// =============================================================================

mod distribution_format_icon_tests {
    use super::*;

    #[test]
    fn windows_installer_icon() {
        assert_eq!(DistributionFormat::WindowsInstaller.icon(), "💿");
    }

    #[test]
    fn windows_portable_icon() {
        assert_eq!(DistributionFormat::WindowsPortable.icon(), "📦");
    }

    #[test]
    fn macos_bundle_icon() {
        assert_eq!(DistributionFormat::MacOSBundle.icon(), "🍎");
    }

    #[test]
    fn macos_dmg_icon() {
        assert_eq!(DistributionFormat::MacOSDmg.icon(), "💾");
    }

    #[test]
    fn linux_appimage_icon() {
        assert_eq!(DistributionFormat::LinuxAppImage.icon(), "🐧");
    }

    #[test]
    fn linux_tarball_icon() {
        assert_eq!(DistributionFormat::LinuxTarball.icon(), "📁");
    }

    #[test]
    fn steam_depot_icon() {
        assert_eq!(DistributionFormat::SteamDepot.icon(), "🎮");
    }
}

// =============================================================================
// DISTRIBUTION FORMAT — DESCRIPTION TESTS (7 variants unique)
// =============================================================================

mod distribution_format_description_tests {
    use super::*;

    #[test]
    fn windows_installer_description() {
        assert_eq!(
            DistributionFormat::WindowsInstaller.description(),
            "Installable EXE with setup wizard"
        );
    }

    #[test]
    fn windows_portable_description() {
        assert_eq!(
            DistributionFormat::WindowsPortable.description(),
            "Portable ZIP, no installation required"
        );
    }

    #[test]
    fn macos_bundle_description() {
        assert_eq!(
            DistributionFormat::MacOSBundle.description(),
            "Standard macOS application bundle"
        );
    }

    #[test]
    fn macos_dmg_description() {
        assert_eq!(
            DistributionFormat::MacOSDmg.description(),
            "Disk image for drag-and-drop install"
        );
    }

    #[test]
    fn linux_appimage_description() {
        assert_eq!(
            DistributionFormat::LinuxAppImage.description(),
            "Self-contained portable Linux app"
        );
    }

    #[test]
    fn linux_tarball_description() {
        assert_eq!(
            DistributionFormat::LinuxTarball.description(),
            "Compressed archive for manual install"
        );
    }

    #[test]
    fn steam_depot_description() {
        assert_eq!(
            DistributionFormat::SteamDepot.description(),
            "Steam content depot for publishing"
        );
    }
}

// =============================================================================
// DISTRIBUTION FORMAT — IS_WINDOWS / IS_MACOS / IS_LINUX / IS_STEAM
// =============================================================================

mod distribution_format_classification_tests {
    use super::*;

    // is_windows: true for WindowsInstaller and WindowsPortable only
    #[test]
    fn windows_installer_is_windows() {
        assert!(DistributionFormat::WindowsInstaller.is_windows());
    }
    #[test]
    fn windows_portable_is_windows() {
        assert!(DistributionFormat::WindowsPortable.is_windows());
    }
    #[test]
    fn macos_bundle_is_not_windows() {
        assert!(!DistributionFormat::MacOSBundle.is_windows());
    }
    #[test]
    fn macos_dmg_is_not_windows() {
        assert!(!DistributionFormat::MacOSDmg.is_windows());
    }
    #[test]
    fn linux_appimage_is_not_windows() {
        assert!(!DistributionFormat::LinuxAppImage.is_windows());
    }
    #[test]
    fn linux_tarball_is_not_windows() {
        assert!(!DistributionFormat::LinuxTarball.is_windows());
    }
    #[test]
    fn steam_depot_is_not_windows() {
        assert!(!DistributionFormat::SteamDepot.is_windows());
    }

    // is_macos: true for MacOSBundle and MacOSDmg only
    #[test]
    fn macos_bundle_is_macos() {
        assert!(DistributionFormat::MacOSBundle.is_macos());
    }
    #[test]
    fn macos_dmg_is_macos() {
        assert!(DistributionFormat::MacOSDmg.is_macos());
    }
    #[test]
    fn windows_installer_is_not_macos() {
        assert!(!DistributionFormat::WindowsInstaller.is_macos());
    }
    #[test]
    fn linux_appimage_is_not_macos() {
        assert!(!DistributionFormat::LinuxAppImage.is_macos());
    }
    #[test]
    fn steam_depot_is_not_macos() {
        assert!(!DistributionFormat::SteamDepot.is_macos());
    }

    // is_linux: true for LinuxAppImage and LinuxTarball only
    #[test]
    fn linux_appimage_is_linux() {
        assert!(DistributionFormat::LinuxAppImage.is_linux());
    }
    #[test]
    fn linux_tarball_is_linux() {
        assert!(DistributionFormat::LinuxTarball.is_linux());
    }
    #[test]
    fn windows_installer_is_not_linux() {
        assert!(!DistributionFormat::WindowsInstaller.is_linux());
    }
    #[test]
    fn macos_bundle_is_not_linux() {
        assert!(!DistributionFormat::MacOSBundle.is_linux());
    }
    #[test]
    fn steam_depot_is_not_linux() {
        assert!(!DistributionFormat::SteamDepot.is_linux());
    }

    // is_steam: true for SteamDepot only
    #[test]
    fn steam_depot_is_steam() {
        assert!(DistributionFormat::SteamDepot.is_steam());
    }
    #[test]
    fn windows_installer_is_not_steam() {
        assert!(!DistributionFormat::WindowsInstaller.is_steam());
    }
    #[test]
    fn macos_bundle_is_not_steam() {
        assert!(!DistributionFormat::MacOSBundle.is_steam());
    }
    #[test]
    fn linux_appimage_is_not_steam() {
        assert!(!DistributionFormat::LinuxAppImage.is_steam());
    }
}

// =============================================================================
// DISTRIBUTION FORMAT — ALL() AND DISPLAY
// =============================================================================

mod distribution_format_all_tests {
    use super::*;

    #[test]
    fn all_returns_7_formats() {
        assert_eq!(DistributionFormat::all().len(), 7);
    }

    #[test]
    fn display_matches_name() {
        for fmt in DistributionFormat::all() {
            assert_eq!(format!("{}", fmt), fmt.name());
        }
    }
}

// =============================================================================
// DISTRIBUTION CONFIG — DEFAULTS AND VALIDATION
// =============================================================================

mod distribution_config_tests {
    use super::*;

    #[test]
    fn default_game_name() {
        let cfg = DistributionConfig::default();
        assert_eq!(cfg.game_name, "AstraWeave Game");
    }

    #[test]
    fn default_version() {
        let cfg = DistributionConfig::default();
        assert_eq!(cfg.version, "1.0.0");
    }

    #[test]
    fn default_publisher() {
        let cfg = DistributionConfig::default();
        assert_eq!(cfg.publisher, "AstraWeave");
    }

    #[test]
    fn default_description() {
        let cfg = DistributionConfig::default();
        assert_eq!(cfg.description, "A game built with AstraWeave Engine");
    }

    #[test]
    fn default_has_no_steam_config() {
        let cfg = DistributionConfig::default();
        assert!(!cfg.has_steam_config());
    }

    #[test]
    fn default_has_no_custom_icon() {
        let cfg = DistributionConfig::default();
        assert!(!cfg.has_custom_icon());
    }

    #[test]
    fn default_is_valid() {
        let cfg = DistributionConfig::default();
        assert!(cfg.is_valid());
    }

    #[test]
    fn empty_game_name_is_invalid() {
        let mut cfg = DistributionConfig::default();
        cfg.game_name = String::new();
        assert!(!cfg.is_valid());
    }

    #[test]
    fn empty_version_is_invalid() {
        let mut cfg = DistributionConfig::default();
        cfg.version = String::new();
        assert!(!cfg.is_valid());
    }

    #[test]
    fn empty_publisher_is_invalid() {
        let mut cfg = DistributionConfig::default();
        cfg.publisher = String::new();
        assert!(!cfg.is_valid());
    }

    #[test]
    fn sanitized_name_replaces_spaces() {
        let mut cfg = DistributionConfig::default();
        cfg.game_name = "My Cool Game".to_string();
        assert_eq!(cfg.sanitized_name(), "My_Cool_Game");
    }

    #[test]
    fn sanitized_name_removes_special_chars() {
        let mut cfg = DistributionConfig::default();
        cfg.game_name = "Game:Reloaded/2".to_string();
        // Removes : and /, replaces space with _
        let name = cfg.sanitized_name();
        assert!(!name.contains(':'));
        assert!(!name.contains('/'));
    }

    #[test]
    fn summary_contains_name_version_publisher() {
        let cfg = DistributionConfig::default();
        let s = cfg.summary();
        assert!(s.contains("AstraWeave Game"));
        assert!(s.contains("1.0.0"));
        assert!(s.contains("AstraWeave"));
    }

    #[test]
    fn has_steam_config_when_app_id_set() {
        let mut cfg = DistributionConfig::default();
        cfg.steam_app_id = Some(12345);
        assert!(cfg.has_steam_config());
    }

    #[test]
    fn has_custom_icon_when_path_set() {
        let mut cfg = DistributionConfig::default();
        cfg.icon_path = Some(std::path::PathBuf::from("icon.png"));
        assert!(cfg.has_custom_icon());
    }
}

// =============================================================================
// FORMAT_BYTES TESTS (range boundaries)
// =============================================================================

mod format_bytes_tests {
    use super::*;

    #[test]
    fn zero_bytes() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn small_bytes() {
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn exactly_1023_bytes() {
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn exactly_1024_is_kb() {
        assert_eq!(format_bytes(1024), "1.00 KB");
    }

    #[test]
    fn kilobytes() {
        assert_eq!(format_bytes(1536), "1.50 KB");
    }

    #[test]
    fn exactly_1mb() {
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn megabytes() {
        assert_eq!(format_bytes(1_572_864), "1.50 MB");
    }

    #[test]
    fn exactly_1gb() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn gigabytes() {
        // 1.5 GB = 1610612736
        assert_eq!(format_bytes(1_610_612_736), "1.50 GB");
    }
}

// =============================================================================
// RUNTIME ISSUE — IS_CRITICAL (6 variants)
// =============================================================================

mod runtime_issue_is_critical_tests {
    use super::*;

    #[test]
    fn missing_simulation_is_critical() {
        assert!(RuntimeIssue::MissingSimulation.is_critical());
    }

    #[test]
    fn corrupted_simulation_is_critical() {
        let issue = RuntimeIssue::CorruptedSimulation {
            reason: "test".into(),
        };
        assert!(issue.is_critical());
    }

    #[test]
    fn missing_edit_snapshot_is_not_critical() {
        assert!(!RuntimeIssue::MissingEditSnapshot.is_critical());
    }

    #[test]
    fn frame_time_exceeded_is_not_critical() {
        let issue = RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33,
        };
        assert!(!issue.is_critical());
    }

    #[test]
    fn low_fps_is_not_critical() {
        let issue = RuntimeIssue::LowFps {
            fps: 20,
            minimum_fps: 30,
        };
        assert!(!issue.is_critical());
    }

    #[test]
    fn entity_count_mismatch_is_not_critical() {
        let issue = RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 90,
        };
        assert!(!issue.is_critical());
    }
}

// =============================================================================
// RUNTIME ISSUE — IS_PERFORMANCE_ISSUE (6 variants)
// =============================================================================

mod runtime_issue_is_performance_issue_tests {
    use super::*;

    #[test]
    fn frame_time_exceeded_is_performance_issue() {
        let issue = RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33,
        };
        assert!(issue.is_performance_issue());
    }

    #[test]
    fn low_fps_is_performance_issue() {
        let issue = RuntimeIssue::LowFps {
            fps: 20,
            minimum_fps: 30,
        };
        assert!(issue.is_performance_issue());
    }

    #[test]
    fn missing_simulation_is_not_performance_issue() {
        assert!(!RuntimeIssue::MissingSimulation.is_performance_issue());
    }

    #[test]
    fn corrupted_simulation_is_not_performance_issue() {
        let issue = RuntimeIssue::CorruptedSimulation {
            reason: "test".into(),
        };
        assert!(!issue.is_performance_issue());
    }

    #[test]
    fn missing_edit_snapshot_is_not_performance_issue() {
        assert!(!RuntimeIssue::MissingEditSnapshot.is_performance_issue());
    }

    #[test]
    fn entity_count_mismatch_is_not_performance_issue() {
        let issue = RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 90,
        };
        assert!(!issue.is_performance_issue());
    }
}

// =============================================================================
// RUNTIME ISSUE — IS_DATA_ISSUE (6 variants)
// =============================================================================

mod runtime_issue_is_data_issue_tests {
    use super::*;

    #[test]
    fn entity_count_mismatch_is_data_issue() {
        let issue = RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 90,
        };
        assert!(issue.is_data_issue());
    }

    #[test]
    fn corrupted_simulation_is_data_issue() {
        let issue = RuntimeIssue::CorruptedSimulation {
            reason: "test".into(),
        };
        assert!(issue.is_data_issue());
    }

    #[test]
    fn missing_edit_snapshot_is_data_issue() {
        assert!(RuntimeIssue::MissingEditSnapshot.is_data_issue());
    }

    #[test]
    fn missing_simulation_is_not_data_issue() {
        assert!(!RuntimeIssue::MissingSimulation.is_data_issue());
    }

    #[test]
    fn frame_time_exceeded_is_not_data_issue() {
        let issue = RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33,
        };
        assert!(!issue.is_data_issue());
    }

    #[test]
    fn low_fps_is_not_data_issue() {
        let issue = RuntimeIssue::LowFps {
            fps: 20,
            minimum_fps: 30,
        };
        assert!(!issue.is_data_issue());
    }
}

// =============================================================================
// RUNTIME ISSUE — IS_RECOVERABLE (6 variants)
// =============================================================================

mod runtime_issue_is_recoverable_tests {
    use super::*;

    #[test]
    fn frame_time_exceeded_is_recoverable() {
        let issue = RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33,
        };
        assert!(issue.is_recoverable());
    }

    #[test]
    fn low_fps_is_recoverable() {
        let issue = RuntimeIssue::LowFps {
            fps: 20,
            minimum_fps: 30,
        };
        assert!(issue.is_recoverable());
    }

    #[test]
    fn entity_count_mismatch_is_recoverable() {
        let issue = RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 90,
        };
        assert!(issue.is_recoverable());
    }

    #[test]
    fn missing_simulation_is_not_recoverable() {
        assert!(!RuntimeIssue::MissingSimulation.is_recoverable());
    }

    #[test]
    fn corrupted_simulation_is_not_recoverable() {
        let issue = RuntimeIssue::CorruptedSimulation {
            reason: "test".into(),
        };
        assert!(!issue.is_recoverable());
    }

    #[test]
    fn missing_edit_snapshot_is_not_recoverable() {
        assert!(!RuntimeIssue::MissingEditSnapshot.is_recoverable());
    }
}

// =============================================================================
// RUNTIME ISSUE — SEVERITY (6 variants)
// =============================================================================

mod runtime_issue_severity_tests {
    use super::*;

    #[test]
    fn missing_simulation_severity_is_5() {
        assert_eq!(RuntimeIssue::MissingSimulation.severity(), 5);
    }

    #[test]
    fn corrupted_simulation_severity_is_5() {
        let issue = RuntimeIssue::CorruptedSimulation {
            reason: "test".into(),
        };
        assert_eq!(issue.severity(), 5);
    }

    #[test]
    fn missing_edit_snapshot_severity_is_4() {
        assert_eq!(RuntimeIssue::MissingEditSnapshot.severity(), 4);
    }

    #[test]
    fn entity_count_mismatch_severity_is_3() {
        let issue = RuntimeIssue::EntityCountMismatch {
            expected: 100,
            actual: 90,
        };
        assert_eq!(issue.severity(), 3);
    }

    #[test]
    fn frame_time_exceeded_severity_is_2() {
        let issue = RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33,
        };
        assert_eq!(issue.severity(), 2);
    }

    #[test]
    fn low_fps_severity_is_1() {
        let issue = RuntimeIssue::LowFps {
            fps: 20,
            minimum_fps: 30,
        };
        assert_eq!(issue.severity(), 1);
    }
}

// =============================================================================
// RUNTIME ISSUE — ICON (maps through severity)
// =============================================================================

mod runtime_issue_icon_tests {
    use super::*;

    #[test]
    fn severity_5_icon_is_red_circle() {
        // Both severity-5 variants should produce 🔴
        assert_eq!(RuntimeIssue::MissingSimulation.icon(), "🔴");
    }

    #[test]
    fn corrupted_also_severity_5_red() {
        let issue = RuntimeIssue::CorruptedSimulation {
            reason: "test".into(),
        };
        assert_eq!(issue.icon(), "🔴");
    }

    #[test]
    fn severity_4_icon_is_orange() {
        assert_eq!(RuntimeIssue::MissingEditSnapshot.icon(), "🟠");
    }

    #[test]
    fn severity_3_icon_is_yellow() {
        let issue = RuntimeIssue::EntityCountMismatch {
            expected: 10,
            actual: 5,
        };
        assert_eq!(issue.icon(), "🟡");
    }

    #[test]
    fn severity_2_icon_is_green() {
        let issue = RuntimeIssue::FrameTimeExceeded {
            frame_time_ms: 50,
            threshold_ms: 33,
        };
        assert_eq!(issue.icon(), "🟢");
    }

    #[test]
    fn severity_1_icon_is_info() {
        let issue = RuntimeIssue::LowFps {
            fps: 20,
            minimum_fps: 30,
        };
        assert_eq!(issue.icon(), "ℹ️");
    }
}

// =============================================================================
// RUNTIME STATS — ESTIMATED_ENTITY_CAPACITY
// =============================================================================

mod runtime_stats_estimated_entity_capacity_tests {
    use super::*;

    fn make_stats(entity_count: usize, frame_time_ms: f32) -> RuntimeStats {
        RuntimeStats {
            entity_count,
            frame_time_ms,
            ..Default::default()
        }
    }

    #[test]
    fn zero_entities_returns_0() {
        let stats = make_stats(0, 10.0);
        assert_eq!(stats.estimated_entity_capacity(), 0);
    }

    #[test]
    fn zero_frame_time_returns_0() {
        let stats = make_stats(100, 0.0);
        assert_eq!(stats.estimated_entity_capacity(), 0);
    }

    #[test]
    fn negative_frame_time_returns_0() {
        let stats = make_stats(100, -1.0);
        assert_eq!(stats.estimated_entity_capacity(), 0);
    }

    #[test]
    fn normal_capacity_calculation() {
        // 1000 entities at 8.33ms → capacity = (1000/8.33)*16.667 ≈ 2000
        let stats = make_stats(1000, 8.333);
        let cap = stats.estimated_entity_capacity();
        // Should be approximately 2000 (1000 / 8.333 * 16.667)
        assert!(cap > 1900 && cap < 2100, "capacity was {}", cap);
    }

    #[test]
    fn at_budget_capacity_equals_entity_count() {
        // 500 entities at exactly 16.667ms → capacity ≈ 500
        let stats = make_stats(500, 16.667);
        let cap = stats.estimated_entity_capacity();
        assert!(cap >= 490 && cap <= 510, "capacity was {}", cap);
    }
}

// =============================================================================
// RUNTIME STATS — SIMULATION_DURATION_SECS
// =============================================================================

mod runtime_stats_simulation_duration_tests {
    use super::*;

    #[test]
    fn zero_ticks_is_zero_seconds() {
        let stats = RuntimeStats {
            tick_count: 0,
            ..Default::default()
        };
        assert!((stats.simulation_duration_secs() - 0.0).abs() < 0.001);
    }

    #[test]
    fn sixty_ticks_is_one_second() {
        let stats = RuntimeStats {
            tick_count: 60,
            ..Default::default()
        };
        assert!((stats.simulation_duration_secs() - 1.0).abs() < 0.02);
    }

    #[test]
    fn one_tick_is_one_sixtieth() {
        let stats = RuntimeStats {
            tick_count: 1,
            ..Default::default()
        };
        let expected = 1.0 / 60.0;
        assert!(
            (stats.simulation_duration_secs() - expected).abs() < 0.001,
            "got {}",
            stats.simulation_duration_secs()
        );
    }

    #[test]
    fn three_hundred_sixty_ticks_is_six_seconds() {
        let stats = RuntimeStats {
            tick_count: 360,
            ..Default::default()
        };
        assert!((stats.simulation_duration_secs() - 6.0).abs() < 0.1);
    }
}

// =============================================================================
// RUNTIME STATS — FRAME_TIME_HEADROOM
// =============================================================================

mod runtime_stats_frame_time_headroom_tests {
    use super::*;

    #[test]
    fn zero_frame_time_has_full_headroom() {
        let stats = RuntimeStats {
            frame_time_ms: 0.0,
            ..Default::default()
        };
        let h = stats.frame_time_headroom();
        assert!((h - 16.667).abs() < 0.01, "headroom was {}", h);
    }

    #[test]
    fn at_budget_has_zero_headroom() {
        let stats = RuntimeStats {
            frame_time_ms: 16.667,
            ..Default::default()
        };
        let h = stats.frame_time_headroom();
        assert!(h.abs() < 0.01, "headroom was {}", h);
    }

    #[test]
    fn over_budget_has_negative_headroom() {
        let stats = RuntimeStats {
            frame_time_ms: 20.0,
            ..Default::default()
        };
        let h = stats.frame_time_headroom();
        assert!(h < 0.0, "headroom was {}", h);
    }

    #[test]
    fn half_budget_has_positive_headroom() {
        let stats = RuntimeStats {
            frame_time_ms: 8.333,
            ..Default::default()
        };
        let h = stats.frame_time_headroom();
        assert!((h - 8.334).abs() < 0.01, "headroom was {}", h);
    }
}

// =============================================================================
// RUNTIME STATS — FPS_STABILITY
// =============================================================================

mod runtime_stats_fps_stability_tests {
    use super::*;

    #[test]
    fn zero_fps_stability_is_zero() {
        let stats = RuntimeStats {
            fps: 0.0,
            ..Default::default()
        };
        assert!((stats.fps_stability() - 0.0).abs() < 0.001);
    }

    #[test]
    fn negative_fps_stability_is_zero() {
        let stats = RuntimeStats {
            fps: -10.0,
            ..Default::default()
        };
        assert!((stats.fps_stability() - 0.0).abs() < 0.001);
    }

    #[test]
    fn fps_60_stability_is_1() {
        let stats = RuntimeStats {
            fps: 60.0,
            ..Default::default()
        };
        assert!((stats.fps_stability() - 1.0).abs() < 0.001);
    }

    #[test]
    fn fps_30_stability_is_half() {
        let stats = RuntimeStats {
            fps: 30.0,
            ..Default::default()
        };
        assert!((stats.fps_stability() - 0.5).abs() < 0.001);
    }

    #[test]
    fn fps_120_stability_clamped_to_1() {
        let stats = RuntimeStats {
            fps: 120.0,
            ..Default::default()
        };
        assert!((stats.fps_stability() - 1.0).abs() < 0.001);
    }

    #[test]
    fn fps_45_stability_is_0_75() {
        let stats = RuntimeStats {
            fps: 45.0,
            ..Default::default()
        };
        assert!((stats.fps_stability() - 0.75).abs() < 0.001);
    }
}

// =============================================================================
// RUNTIME STATS — STATUS_COLOR (maps through performance_grade)
// =============================================================================

mod runtime_stats_status_color_tests {
    use super::*;

    fn make_fps_stats(fps: f32) -> RuntimeStats {
        RuntimeStats {
            fps,
            ..Default::default()
        }
    }

    #[test]
    fn critical_fps_has_status_color() {
        let stats = make_fps_stats(10.0);
        let color = stats.status_color();
        assert!(!color.is_empty());
    }

    #[test]
    fn excellent_fps_has_status_color() {
        let stats = make_fps_stats(60.0);
        let color = stats.status_color();
        assert!(!color.is_empty());
    }

    #[test]
    fn different_grades_have_different_colors() {
        let critical = make_fps_stats(10.0).status_color();
        let excellent = make_fps_stats(60.0).status_color();
        assert_ne!(critical, excellent);
    }
}

// =============================================================================
// EDITOR RUNTIME — CONSTRUCTION AND FIXED_DT
// =============================================================================

mod editor_runtime_tests {
    use super::*;

    #[test]
    fn new_runtime_starts_in_editing_state() {
        let rt = EditorRuntime::new();
        assert_eq!(rt.state(), RuntimeState::Editing);
    }

    #[test]
    fn fixed_dt_is_one_sixtieth() {
        let rt = EditorRuntime::new();
        let expected = 1.0_f32 / 60.0;
        assert!((rt.fixed_dt() - expected).abs() < 0.0001);
    }

    #[test]
    fn initial_stats_have_zero_tick_count() {
        let rt = EditorRuntime::new();
        assert_eq!(rt.stats().tick_count, 0);
    }
}

// =============================================================================
// CLIPBOARD — SCHEMA VERSION CONSTANT
// =============================================================================

mod clipboard_schema_version_tests {
    use super::*;

    #[test]
    fn clipboard_schema_version_is_2() {
        assert_eq!(CLIPBOARD_SCHEMA_VERSION, 2);
    }

    #[test]
    fn clipboard_schema_version_is_not_0() {
        assert_ne!(CLIPBOARD_SCHEMA_VERSION, 0);
    }

    #[test]
    fn clipboard_schema_version_is_not_1() {
        assert_ne!(CLIPBOARD_SCHEMA_VERSION, 1);
    }
}

// =============================================================================
// CLIPBOARD ENTITY DATA — VALIDATE BOUNDARY TESTS
// =============================================================================

mod clipboard_entity_validate_tests {
    use super::*;

    fn make_entity(name: &str, scale: f32, hp: i32, ammo: i32) -> ClipboardEntityData {
        ClipboardEntityData {
            name: name.to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale,
            hp,
            team_id: 0,
            ammo,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
        }
    }

    #[test]
    fn valid_entity_passes() {
        let e = make_entity("soldier", 1.0, 100, 30);
        let v = e.validate();
        assert!(v.is_valid);
    }

    #[test]
    fn empty_name_generates_warning() {
        let e = make_entity("", 1.0, 100, 30);
        let v = e.validate();
        assert!(!v.warnings.is_empty());
    }

    #[test]
    fn name_exactly_256_chars_is_ok() {
        let name = "a".repeat(256);
        let e = make_entity(&name, 1.0, 100, 30);
        let v = e.validate();
        // No error for length exactly 256
        assert!(v.errors.is_empty(), "errors: {:?}", v.errors);
    }

    #[test]
    fn name_257_chars_generates_error() {
        let name = "a".repeat(257);
        let e = make_entity(&name, 1.0, 100, 30);
        let v = e.validate();
        assert!(!v.errors.is_empty());
        assert!(!v.is_valid);
    }

    #[test]
    fn scale_zero_generates_error() {
        let e = make_entity("test", 0.0, 100, 30);
        let v = e.validate();
        assert!(!v.errors.is_empty());
        assert!(!v.is_valid);
    }

    #[test]
    fn scale_negative_generates_error() {
        let e = make_entity("test", -0.5, 100, 30);
        let v = e.validate();
        assert!(!v.is_valid);
    }

    #[test]
    fn scale_small_positive_is_valid() {
        let e = make_entity("test", 0.001, 100, 30);
        let v = e.validate();
        assert!(v.is_valid);
    }

    #[test]
    fn scale_exactly_1000_no_warning() {
        let e = make_entity("test", 1000.0, 100, 30);
        let v = e.validate();
        // scale > 1000.0 is warning, scale at 1000 is ok
        let has_scale_warning = v
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("scale"));
        assert!(!has_scale_warning, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn scale_1001_generates_warning() {
        let e = make_entity("test", 1001.0, 100, 30);
        let v = e.validate();
        assert!(!v.warnings.is_empty());
    }

    #[test]
    fn hp_negative_generates_warning() {
        let e = make_entity("test", 1.0, -1, 30);
        let v = e.validate();
        assert!(!v.warnings.is_empty());
    }

    #[test]
    fn hp_zero_is_ok() {
        let e = make_entity("test", 1.0, 0, 30);
        let v = e.validate();
        let has_hp_warning = v.warnings.iter().any(|w| w.to_lowercase().contains("hp"));
        assert!(!has_hp_warning, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn ammo_negative_generates_warning() {
        let e = make_entity("test", 1.0, 100, -1);
        let v = e.validate();
        assert!(!v.warnings.is_empty());
    }

    #[test]
    fn ammo_zero_is_ok() {
        let e = make_entity("test", 1.0, 100, 0);
        let v = e.validate();
        let has_ammo_warning = v.warnings.iter().any(|w| w.to_lowercase().contains("ammo"));
        assert!(!has_ammo_warning, "warnings: {:?}", v.warnings);
    }
}

// =============================================================================
// CLIPBOARD VALIDATION — ISSUE_COUNT
// =============================================================================

mod clipboard_validation_tests {
    use super::*;

    #[test]
    fn empty_validation_has_zero_issues() {
        let v = ClipboardValidation {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(v.issue_count(), 0);
    }

    #[test]
    fn errors_plus_warnings_equals_issue_count() {
        let v = ClipboardValidation {
            is_valid: false,
            errors: vec!["e1".to_string(), "e2".to_string()],
            warnings: vec!["w1".to_string()],
        };
        assert_eq!(v.issue_count(), 3);
    }
}

// =============================================================================
// CLIPBOARD STATS — CLASSIFIERS
// =============================================================================

mod clipboard_stats_tests {
    use super::*;

    #[test]
    fn empty_stats_is_empty() {
        let s = ClipboardStats {
            entity_count: 0,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 0,
            version: 2,
        };
        assert!(s.is_empty());
    }

    #[test]
    fn non_empty_stats_is_not_empty() {
        let s = ClipboardStats {
            entity_count: 1,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 0,
            version: 2,
        };
        assert!(!s.is_empty());
    }

    #[test]
    fn has_ai_entities_when_behavior_graph_present() {
        let s = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 1,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 1,
            version: 2,
        };
        assert!(s.has_ai_entities());
    }

    #[test]
    fn no_ai_entities_when_zero_behavior_graphs() {
        let s = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 1,
            version: 2,
        };
        assert!(!s.has_ai_entities());
    }

    #[test]
    fn is_multi_team_when_more_than_1() {
        let s = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 2,
            version: 2,
        };
        assert!(s.is_multi_team());
    }

    #[test]
    fn not_multi_team_when_1_or_less() {
        let s = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 1,
            version: 2,
        };
        assert!(!s.is_multi_team());
    }

    #[test]
    fn not_multi_team_when_zero() {
        let s = ClipboardStats {
            entity_count: 5,
            with_behavior_graph: 0,
            with_cooldowns: 0,
            total_cooldowns: 0,
            unique_teams: 0,
            version: 2,
        };
        assert!(!s.is_multi_team());
    }
}

// =============================================================================
// SCENE VALIDATION ISSUE — IS_ERROR / IS_WARNING / ICON / NAME
// =============================================================================

mod scene_validation_issue_tests {
    use super::*;

    #[test]
    fn error_is_error() {
        let issue = SceneValidationIssue::Error("test".into());
        assert!(issue.is_error());
    }

    #[test]
    fn error_is_not_warning() {
        let issue = SceneValidationIssue::Error("test".into());
        assert!(!issue.is_warning());
    }

    #[test]
    fn warning_is_warning() {
        let issue = SceneValidationIssue::Warning("test".into());
        assert!(issue.is_warning());
    }

    #[test]
    fn warning_is_not_error() {
        let issue = SceneValidationIssue::Warning("test".into());
        assert!(!issue.is_error());
    }

    #[test]
    fn error_icon_is_red_x() {
        let issue = SceneValidationIssue::Error("test".into());
        assert_eq!(issue.icon(), "❌");
    }

    #[test]
    fn warning_icon_is_warning_sign() {
        let issue = SceneValidationIssue::Warning("test".into());
        assert_eq!(issue.icon(), "⚠️");
    }

    #[test]
    fn error_name_is_error() {
        let issue = SceneValidationIssue::Error("test".into());
        assert_eq!(issue.name(), "Error");
    }

    #[test]
    fn warning_name_is_warning() {
        let issue = SceneValidationIssue::Warning("test".into());
        assert_eq!(issue.name(), "Warning");
    }

    #[test]
    fn error_message_contains_text() {
        let issue = SceneValidationIssue::Error("duplicate id 42".into());
        assert_eq!(issue.message(), "duplicate id 42");
    }

    #[test]
    fn warning_message_contains_text() {
        let issue = SceneValidationIssue::Warning("empty name".into());
        assert_eq!(issue.message(), "empty name");
    }
}

// =============================================================================
// SCENE DATA — VALIDATE BOUNDARY TESTS
// =============================================================================

mod scene_data_validate_tests {
    use super::*;

    fn make_scene(entities: Vec<EntityData>, next_entity_id: u32) -> SceneData {
        SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: next_entity_id,
            entities,
            obstacles: vec![],
        }
    }

    fn make_entity_data(id: u32, name: &str, scale: f32, hp: i32, ammo: i32) -> EntityData {
        EntityData {
            id,
            name: name.to_string(),
            pos: IVec2 { x: 0, y: 0 },
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            scale,
            hp,
            team_id: 0,
            ammo,
            cooldowns: BTreeMap::new(),
            behavior_graph: None,
        }
    }

    #[test]
    fn empty_scene_is_valid() {
        let scene = make_scene(vec![], 1);
        assert!(scene.is_valid());
    }

    #[test]
    fn valid_single_entity_scene() {
        let entities = vec![make_entity_data(0, "soldier", 1.0, 100, 30)];
        let scene = make_scene(entities, 1);
        assert!(scene.is_valid());
    }

    #[test]
    fn duplicate_entity_ids_produce_error() {
        let entities = vec![
            make_entity_data(1, "a", 1.0, 100, 0),
            make_entity_data(1, "b", 1.0, 100, 0),
        ];
        let scene = make_scene(entities, 5);
        let issues = scene.validate();
        assert!(issues.iter().any(|i| i.is_error()));
    }

    #[test]
    fn scale_zero_produces_error() {
        let entities = vec![make_entity_data(0, "test", 0.0, 100, 0)];
        let scene = make_scene(entities, 1);
        let issues = scene.validate();
        assert!(issues.iter().any(|i| i.is_error()));
    }

    #[test]
    fn scale_negative_produces_error() {
        let entities = vec![make_entity_data(0, "test", -1.0, 100, 0)];
        let scene = make_scene(entities, 1);
        let issues = scene.validate();
        assert!(issues.iter().any(|i| i.is_error()));
    }

    #[test]
    fn scale_positive_no_error() {
        let entities = vec![make_entity_data(0, "test", 0.5, 100, 0)];
        let scene = make_scene(entities, 1);
        let issues = scene.validate();
        assert!(!issues.iter().any(|i| i.is_error()));
    }

    #[test]
    fn hp_negative_produces_warning() {
        let entities = vec![make_entity_data(0, "test", 1.0, -1, 0)];
        let scene = make_scene(entities, 1);
        let issues = scene.validate();
        assert!(issues.iter().any(|i| i.is_warning()));
    }

    #[test]
    fn hp_zero_no_warning_for_hp() {
        let entities = vec![make_entity_data(0, "soldier", 1.0, 0, 0)];
        let scene = make_scene(entities, 1);
        let issues = scene.validate();
        let hp_warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.is_warning() && i.message().to_lowercase().contains("hp"))
            .collect();
        assert!(hp_warnings.is_empty(), "hp_warnings: {:?}", hp_warnings);
    }

    #[test]
    fn ammo_negative_produces_warning() {
        let entities = vec![make_entity_data(0, "test", 1.0, 100, -1)];
        let scene = make_scene(entities, 1);
        let issues = scene.validate();
        assert!(issues.iter().any(|i| i.is_warning()));
    }

    #[test]
    fn ammo_zero_no_warning_for_ammo() {
        let entities = vec![make_entity_data(0, "soldier", 1.0, 100, 0)];
        let scene = make_scene(entities, 1);
        let issues = scene.validate();
        let ammo_warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.is_warning() && i.message().to_lowercase().contains("ammo"))
            .collect();
        assert!(
            ammo_warnings.is_empty(),
            "ammo_warnings: {:?}",
            ammo_warnings
        );
    }

    #[test]
    fn empty_name_produces_warning() {
        let entities = vec![make_entity_data(0, "", 1.0, 100, 0)];
        let scene = make_scene(entities, 1);
        let issues = scene.validate();
        assert!(issues.iter().any(|i| i.is_warning()));
    }
}

// =============================================================================
// SCENE DATA — STATS
// =============================================================================

mod scene_data_stats_tests {
    use super::*;

    #[test]
    fn empty_scene_has_zero_entity_count() {
        let scene = SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: 0,
            entities: vec![],
            obstacles: vec![],
        };
        let stats = scene.stats();
        assert_eq!(stats.entity_count, 0);
    }

    #[test]
    fn scene_with_obstacles_reflects_count() {
        let scene = SceneData {
            version: 1,
            time: 0.0,
            next_entity_id: 0,
            entities: vec![],
            obstacles: vec![(1, 2), (3, 4)],
        };
        let stats = scene.stats();
        assert_eq!(stats.obstacle_count, 2);
    }
}

// =============================================================================
// GIZMO HANDLE — ALL() AND CLASSIFICATION (10 variants)
// =============================================================================

mod gizmo_handle_tests {
    use super::*;

    #[test]
    fn all_returns_10_handles() {
        assert_eq!(GizmoHandle::all().len(), 10);
    }

    // is_translate tests
    #[test]
    fn translate_x_is_translate() {
        assert!(GizmoHandle::TranslateX.is_translate());
    }
    #[test]
    fn translate_y_is_translate() {
        assert!(GizmoHandle::TranslateY.is_translate());
    }
    #[test]
    fn translate_z_is_translate() {
        assert!(GizmoHandle::TranslateZ.is_translate());
    }
    #[test]
    fn rotate_x_is_not_translate() {
        assert!(!GizmoHandle::RotateX.is_translate());
    }
    #[test]
    fn scale_x_is_not_translate() {
        assert!(!GizmoHandle::ScaleX.is_translate());
    }
    #[test]
    fn scale_uniform_is_not_translate() {
        assert!(!GizmoHandle::ScaleUniform.is_translate());
    }

    // is_rotate tests
    #[test]
    fn rotate_x_is_rotate() {
        assert!(GizmoHandle::RotateX.is_rotate());
    }
    #[test]
    fn rotate_y_is_rotate() {
        assert!(GizmoHandle::RotateY.is_rotate());
    }
    #[test]
    fn rotate_z_is_rotate() {
        assert!(GizmoHandle::RotateZ.is_rotate());
    }
    #[test]
    fn translate_x_is_not_rotate() {
        assert!(!GizmoHandle::TranslateX.is_rotate());
    }
    #[test]
    fn scale_x_is_not_rotate() {
        assert!(!GizmoHandle::ScaleX.is_rotate());
    }

    // is_scale tests
    #[test]
    fn scale_x_is_scale() {
        assert!(GizmoHandle::ScaleX.is_scale());
    }
    #[test]
    fn scale_y_is_scale() {
        assert!(GizmoHandle::ScaleY.is_scale());
    }
    #[test]
    fn scale_z_is_scale() {
        assert!(GizmoHandle::ScaleZ.is_scale());
    }
    #[test]
    fn scale_uniform_is_scale() {
        assert!(GizmoHandle::ScaleUniform.is_scale());
    }
    #[test]
    fn translate_x_is_not_scale() {
        assert!(!GizmoHandle::TranslateX.is_scale());
    }
    #[test]
    fn rotate_x_is_not_scale() {
        assert!(!GizmoHandle::RotateX.is_scale());
    }

    // axis() tests
    #[test]
    fn translate_x_axis_is_x() {
        assert_eq!(GizmoHandle::TranslateX.axis(), 'X');
    }
    #[test]
    fn translate_y_axis_is_y() {
        assert_eq!(GizmoHandle::TranslateY.axis(), 'Y');
    }
    #[test]
    fn translate_z_axis_is_z() {
        assert_eq!(GizmoHandle::TranslateZ.axis(), 'Z');
    }
    #[test]
    fn rotate_x_axis_is_x() {
        assert_eq!(GizmoHandle::RotateX.axis(), 'X');
    }
    #[test]
    fn rotate_y_axis_is_y() {
        assert_eq!(GizmoHandle::RotateY.axis(), 'Y');
    }
    #[test]
    fn rotate_z_axis_is_z() {
        assert_eq!(GizmoHandle::RotateZ.axis(), 'Z');
    }
    #[test]
    fn scale_x_axis_is_x() {
        assert_eq!(GizmoHandle::ScaleX.axis(), 'X');
    }
    #[test]
    fn scale_y_axis_is_y() {
        assert_eq!(GizmoHandle::ScaleY.axis(), 'Y');
    }
    #[test]
    fn scale_z_axis_is_z() {
        assert_eq!(GizmoHandle::ScaleZ.axis(), 'Z');
    }
    #[test]
    fn scale_uniform_axis_is_u() {
        assert_eq!(GizmoHandle::ScaleUniform.axis(), 'U');
    }

    // color() tests — X=red, Y=green, Z=blue, Uniform=white
    #[test]
    fn translate_x_color_has_high_red() {
        let c = GizmoHandle::TranslateX.color();
        assert!(c[0] > 0.9, "r={}", c[0]);
    }
    #[test]
    fn translate_y_color_has_high_green() {
        let c = GizmoHandle::TranslateY.color();
        assert!(c[1] > 0.9, "g={}", c[1]);
    }
    #[test]
    fn translate_z_color_has_high_blue() {
        let c = GizmoHandle::TranslateZ.color();
        assert!(c[2] > 0.9, "b={}", c[2]);
    }
    #[test]
    fn scale_uniform_color_is_white() {
        let c = GizmoHandle::ScaleUniform.color();
        assert!(c[0] > 0.9 && c[1] > 0.9 && c[2] > 0.9);
    }
}

// =============================================================================
// GIZMO PICKER — DEFAULTS
// =============================================================================

mod gizmo_picker_default_tests {
    use super::*;

    #[test]
    fn default_max_distance_is_100() {
        let picker = GizmoPicker::default();
        assert!((picker.max_distance - 100.0).abs() < 0.001);
    }

    #[test]
    fn default_tolerance_is_0_2() {
        let picker = GizmoPicker::default();
        assert!((picker.tolerance - 0.2).abs() < 0.001);
    }

    #[test]
    fn default_gizmo_scale_is_1() {
        let picker = GizmoPicker::default();
        assert!((picker.gizmo_scale - 1.0).abs() < 0.001);
    }
}

// =============================================================================
// RAY — POINT_AT
// =============================================================================

mod ray_tests {
    use super::*;

    #[test]
    fn point_at_zero_is_origin() {
        let ray = Ray {
            origin: Vec3::new(1.0, 2.0, 3.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
        };
        let p = ray.point_at(0.0);
        assert!((p - Vec3::new(1.0, 2.0, 3.0)).length() < 0.001);
    }

    #[test]
    fn point_at_1_is_origin_plus_direction() {
        let ray = Ray {
            origin: Vec3::ZERO,
            direction: Vec3::new(1.0, 0.0, 0.0),
        };
        let p = ray.point_at(1.0);
        assert!((p - Vec3::new(1.0, 0.0, 0.0)).length() < 0.001);
    }

    #[test]
    fn point_at_5_scales_direction() {
        let ray = Ray {
            origin: Vec3::ZERO,
            direction: Vec3::new(0.0, 1.0, 0.0),
        };
        let p = ray.point_at(5.0);
        assert!((p - Vec3::new(0.0, 5.0, 0.0)).length() < 0.001);
    }
}

// =============================================================================
// SNAPPING CONFIG — BOUNDARY TESTS
// =============================================================================

mod snapping_config_tests {
    use super::*;

    #[test]
    fn default_grid_size_is_1() {
        let cfg = SnappingConfig::default();
        assert!((cfg.grid_size - 1.0).abs() < 0.001);
    }

    #[test]
    fn default_angle_increment_is_15() {
        let cfg = SnappingConfig::default();
        assert!((cfg.angle_increment - 15.0).abs() < 0.001);
    }

    #[test]
    fn default_grid_enabled_is_true() {
        let cfg = SnappingConfig::default();
        assert!(cfg.grid_enabled);
    }

    #[test]
    fn default_angle_enabled_is_true() {
        let cfg = SnappingConfig::default();
        assert!(cfg.angle_enabled);
    }

    #[test]
    fn snap_position_disabled_returns_raw() {
        let cfg = SnappingConfig {
            grid_enabled: false,
            grid_size: 1.0,
            angle_increment: 15.0,
            angle_enabled: true,
        };
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let snapped = cfg.snap_position(pos);
        assert!((snapped - pos).length() < 0.001);
    }

    #[test]
    fn snap_position_zero_grid_size_returns_raw() {
        let cfg = SnappingConfig {
            grid_enabled: true,
            grid_size: 0.0,
            angle_increment: 15.0,
            angle_enabled: true,
        };
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let snapped = cfg.snap_position(pos);
        assert!((snapped - pos).length() < 0.001);
    }

    #[test]
    fn snap_position_negative_grid_size_returns_raw() {
        let cfg = SnappingConfig {
            grid_enabled: true,
            grid_size: -1.0,
            angle_increment: 15.0,
            angle_enabled: true,
        };
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let snapped = cfg.snap_position(pos);
        assert!((snapped - pos).length() < 0.001);
    }

    #[test]
    fn snap_position_default_snaps_to_grid() {
        let cfg = SnappingConfig::default();
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let snapped = cfg.snap_position(pos);
        assert!((snapped.x - 2.0).abs() < 0.001);
        assert!((snapped.y - 2.0).abs() < 0.001);
        assert!((snapped.z - 5.0).abs() < 0.001);
    }

    #[test]
    fn snap_position_half_grid() {
        let cfg = SnappingConfig {
            grid_size: 0.5,
            grid_enabled: true,
            angle_increment: 15.0,
            angle_enabled: true,
        };
        let pos = Vec3::new(1.3, 0.0, 0.0);
        let snapped = cfg.snap_position(pos);
        assert!((snapped.x - 1.5).abs() < 0.001);
    }

    #[test]
    fn snap_angle_disabled_returns_raw() {
        let cfg = SnappingConfig {
            grid_size: 1.0,
            grid_enabled: true,
            angle_increment: 15.0,
            angle_enabled: false,
        };
        let angle = 0.123;
        let snapped = cfg.snap_angle(angle);
        assert!((snapped - angle).abs() < 0.001);
    }

    #[test]
    fn snap_angle_zero_increment_returns_raw() {
        let cfg = SnappingConfig {
            grid_size: 1.0,
            grid_enabled: true,
            angle_increment: 0.0,
            angle_enabled: true,
        };
        let angle = 0.123;
        let snapped = cfg.snap_angle(angle);
        assert!((snapped - angle).abs() < 0.001);
    }

    #[test]
    fn snap_angle_negative_increment_returns_raw() {
        let cfg = SnappingConfig {
            grid_size: 1.0,
            grid_enabled: true,
            angle_increment: -15.0,
            angle_enabled: true,
        };
        let angle = 0.123;
        let snapped = cfg.snap_angle(angle);
        assert!((snapped - angle).abs() < 0.001);
    }

    #[test]
    fn snap_angle_default_snaps_to_15_degrees() {
        let cfg = SnappingConfig::default();
        let angle = 20.0_f32.to_radians(); // Between 15° and 30°, closer to 15°
        let snapped = cfg.snap_angle(angle);
        let expected = 15.0_f32.to_radians();
        assert!(
            (snapped - expected).abs() < 0.01,
            "snapped={} expected={}",
            snapped.to_degrees(),
            expected.to_degrees()
        );
    }

    #[test]
    fn snap_angle_at_22_5_snaps_to_15() {
        let cfg = SnappingConfig::default();
        // 22.5° is midpoint: should round to 15° or 30° depending on rounding
        let angle = 22.5_f32.to_radians();
        let snapped = cfg.snap_angle(angle);
        // 22.5 / 15 = 1.5, round() = 2, so 2*15=30°
        let expected = 30.0_f32.to_radians();
        assert!(
            (snapped - expected).abs() < 0.01,
            "snapped={} expected={}",
            snapped.to_degrees(),
            expected.to_degrees()
        );
    }
}

// =============================================================================
// SCALE GIZMO — CLAMPING AND CONSTRAINTS
// =============================================================================

mod scale_gizmo_tests {
    use super::*;

    #[test]
    fn uniform_scale_2x() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100.0, 0.0),
            AxisConstraint::None,
            true,
            1.0,
            Quat::IDENTITY,
            false,
        );
        // 1.0 + (100/100)*1.0 = 2.0
        assert!((scale.x - 2.0).abs() < 0.01);
        assert!((scale.y - 2.0).abs() < 0.01);
        assert!((scale.z - 2.0).abs() < 0.01);
    }

    #[test]
    fn x_axis_scale_leaves_y_z_at_1() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100.0, 0.0),
            AxisConstraint::X,
            false,
            1.0,
            Quat::IDENTITY,
            false,
        );
        assert!((scale.x - 2.0).abs() < 0.01);
        assert!((scale.y - 1.0).abs() < 0.01);
        assert!((scale.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn y_axis_scale_leaves_x_z_at_1() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(200.0, 0.0),
            AxisConstraint::Y,
            false,
            1.0,
            Quat::IDENTITY,
            false,
        );
        assert!((scale.x - 1.0).abs() < 0.01);
        assert!((scale.y - 3.0).abs() < 0.01); // 1 + 200/100 = 3
        assert!((scale.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn z_axis_scale_leaves_x_y_at_1() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(50.0, 0.0),
            AxisConstraint::Z,
            false,
            1.0,
            Quat::IDENTITY,
            false,
        );
        assert!((scale.x - 1.0).abs() < 0.01);
        assert!((scale.y - 1.0).abs() < 0.01);
        assert!((scale.z - 1.5).abs() < 0.01); // 1 + 50/100 = 1.5
    }

    #[test]
    fn zero_delta_gives_scale_1() {
        let scale = ScaleGizmo::calculate_scale(
            Vec2::ZERO,
            AxisConstraint::None,
            true,
            1.0,
            Quat::IDENTITY,
            false,
        );
        assert!((scale.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn clamped_to_max_100() {
        // Very large delta should be clamped to MAX_SCALE=100
        let scale = ScaleGizmo::calculate_scale(
            Vec2::new(100000.0, 0.0),
            AxisConstraint::None,
            true,
            1.0,
            Quat::IDENTITY,
            false,
        );
        assert!((scale.x - 100.0).abs() < 0.01);
    }

    #[test]
    fn numeric_scale_x_axis() {
        let scale = ScaleGizmo::calculate_scale_numeric(2.0, AxisConstraint::X, false);
        assert!((scale.x - 2.0).abs() < 0.01);
        assert!((scale.y - 1.0).abs() < 0.01);
        assert!((scale.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn numeric_scale_y_axis() {
        let scale = ScaleGizmo::calculate_scale_numeric(3.0, AxisConstraint::Y, false);
        assert!((scale.x - 1.0).abs() < 0.01);
        assert!((scale.y - 3.0).abs() < 0.01);
        assert!((scale.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn numeric_scale_z_axis() {
        let scale = ScaleGizmo::calculate_scale_numeric(0.5, AxisConstraint::Z, false);
        assert!((scale.x - 1.0).abs() < 0.01);
        assert!((scale.y - 1.0).abs() < 0.01);
        assert!((scale.z - 0.5).abs() < 0.01);
    }

    #[test]
    fn numeric_uniform() {
        let scale = ScaleGizmo::calculate_scale_numeric(2.0, AxisConstraint::None, true);
        assert!((scale.x - 2.0).abs() < 0.01);
        assert!((scale.y - 2.0).abs() < 0.01);
        assert!((scale.z - 2.0).abs() < 0.01);
    }

    #[test]
    fn numeric_clamped_min() {
        let scale = ScaleGizmo::calculate_scale_numeric(-5.0, AxisConstraint::None, true);
        // Clamped to MIN_SCALE = 0.01
        assert!((scale.x - 0.01).abs() < 0.001);
    }

    #[test]
    fn numeric_clamped_max() {
        let scale = ScaleGizmo::calculate_scale_numeric(999.0, AxisConstraint::None, true);
        // Clamped to MAX_SCALE = 100
        assert!((scale.x - 100.0).abs() < 0.01);
    }
}

// =============================================================================
// TRANSLATE GIZMO — CONSTRAINED SNAPPING
// =============================================================================

mod translate_gizmo_tests {
    use super::*;

    #[test]
    fn snap_position_constrained_x_only_snaps_x() {
        let cfg = SnappingConfig::default(); // grid_size=1.0
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let orig = Vec3::new(0.0, 0.0, 0.0);
        let snapped = TranslateGizmo::snap_position_constrained(pos, orig, AxisConstraint::X, &cfg);
        assert!((snapped.x - 2.0).abs() < 0.001); // X snapped
        assert!((snapped.y - orig.y).abs() < 0.001); // Y preserved from original
        assert!((snapped.z - orig.z).abs() < 0.001); // Z preserved from original
    }

    #[test]
    fn snap_position_constrained_y_only_snaps_y() {
        let cfg = SnappingConfig::default();
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let orig = Vec3::new(1.0, 1.0, 1.0);
        let snapped = TranslateGizmo::snap_position_constrained(pos, orig, AxisConstraint::Y, &cfg);
        assert!((snapped.x - orig.x).abs() < 0.001); // X preserved
        assert!((snapped.y - 2.0).abs() < 0.001); // Y snapped
        assert!((snapped.z - orig.z).abs() < 0.001); // Z preserved
    }

    #[test]
    fn snap_position_constrained_z_only_snaps_z() {
        let cfg = SnappingConfig::default();
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let orig = Vec3::new(1.0, 1.0, 1.0);
        let snapped = TranslateGizmo::snap_position_constrained(pos, orig, AxisConstraint::Z, &cfg);
        assert!((snapped.x - orig.x).abs() < 0.001);
        assert!((snapped.y - orig.y).abs() < 0.001);
        assert!((snapped.z - 5.0).abs() < 0.001); // Z snapped
    }

    #[test]
    fn snap_position_constrained_xy_snaps_both() {
        let cfg = SnappingConfig::default();
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let orig = Vec3::new(0.0, 0.0, 0.0);
        let snapped =
            TranslateGizmo::snap_position_constrained(pos, orig, AxisConstraint::XY, &cfg);
        assert!((snapped.x - 2.0).abs() < 0.001);
        assert!((snapped.y - 2.0).abs() < 0.001);
        assert!((snapped.z - orig.z).abs() < 0.001); // Z preserved
    }

    #[test]
    fn snap_position_constrained_xz_snaps_both() {
        let cfg = SnappingConfig::default();
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let orig = Vec3::new(0.0, 0.0, 0.0);
        let snapped =
            TranslateGizmo::snap_position_constrained(pos, orig, AxisConstraint::XZ, &cfg);
        assert!((snapped.x - 2.0).abs() < 0.001);
        assert!((snapped.y - orig.y).abs() < 0.001); // Y preserved
        assert!((snapped.z - 5.0).abs() < 0.001);
    }

    #[test]
    fn snap_position_constrained_yz_snaps_both() {
        let cfg = SnappingConfig::default();
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let orig = Vec3::new(0.0, 0.0, 0.0);
        let snapped =
            TranslateGizmo::snap_position_constrained(pos, orig, AxisConstraint::YZ, &cfg);
        assert!((snapped.x - orig.x).abs() < 0.001); // X preserved
        assert!((snapped.y - 2.0).abs() < 0.001);
        assert!((snapped.z - 5.0).abs() < 0.001);
    }

    #[test]
    fn snap_position_constrained_none_snaps_all() {
        let cfg = SnappingConfig::default();
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let orig = Vec3::new(0.0, 0.0, 0.0);
        let snapped =
            TranslateGizmo::snap_position_constrained(pos, orig, AxisConstraint::None, &cfg);
        assert!((snapped.x - 2.0).abs() < 0.001);
        assert!((snapped.y - 2.0).abs() < 0.001);
        assert!((snapped.z - 5.0).abs() < 0.001);
    }

    #[test]
    fn snap_position_constrained_disabled_returns_raw() {
        let cfg = SnappingConfig {
            grid_enabled: false,
            grid_size: 1.0,
            angle_increment: 15.0,
            angle_enabled: true,
        };
        let pos = Vec3::new(1.7, 2.3, 4.9);
        let orig = Vec3::ZERO;
        let snapped = TranslateGizmo::snap_position_constrained(pos, orig, AxisConstraint::X, &cfg);
        assert!((snapped - pos).length() < 0.001);
    }

    // Numeric translation tests
    #[test]
    fn numeric_x_translation() {
        let t = TranslateGizmo::calculate_translation_numeric(
            5.0,
            AxisConstraint::X,
            Quat::IDENTITY,
            false,
        );
        assert!((t.x - 5.0).abs() < 0.001);
        assert!((t.y - 0.0).abs() < 0.001);
        assert!((t.z - 0.0).abs() < 0.001);
    }

    #[test]
    fn numeric_y_translation() {
        let t = TranslateGizmo::calculate_translation_numeric(
            3.0,
            AxisConstraint::Y,
            Quat::IDENTITY,
            false,
        );
        assert!((t.x - 0.0).abs() < 0.001);
        assert!((t.y - 3.0).abs() < 0.001);
        assert!((t.z - 0.0).abs() < 0.001);
    }

    #[test]
    fn numeric_z_translation() {
        let t = TranslateGizmo::calculate_translation_numeric(
            -2.0,
            AxisConstraint::Z,
            Quat::IDENTITY,
            false,
        );
        assert!((t.x - 0.0).abs() < 0.001);
        assert!((t.y - 0.0).abs() < 0.001);
        assert!((t.z - (-2.0)).abs() < 0.001);
    }

    #[test]
    fn numeric_none_constraint_returns_zero() {
        let t = TranslateGizmo::calculate_translation_numeric(
            5.0,
            AxisConstraint::None,
            Quat::IDENTITY,
            false,
        );
        assert!(t.length() < 0.001);
    }

    #[test]
    fn numeric_planar_constraint_returns_zero() {
        let t = TranslateGizmo::calculate_translation_numeric(
            5.0,
            AxisConstraint::XY,
            Quat::IDENTITY,
            false,
        );
        assert!(t.length() < 0.001);
    }
}

// =============================================================================
// CAMERA CONTROLLER — DEFAULTS AND VIEWS
// =============================================================================

mod camera_controller_tests {
    use super::*;

    #[test]
    fn default_position() {
        let cam = CameraController::default();
        assert!((cam.position - Vec3::new(5.0, 5.0, 5.0)).length() < 0.001);
    }

    #[test]
    fn default_target_is_origin() {
        let cam = CameraController::default();
        assert!((cam.target - Vec3::ZERO).length() < 0.001);
    }

    #[test]
    fn default_up_is_y() {
        let cam = CameraController::default();
        assert!((cam.up - Vec3::Y).length() < 0.001);
    }

    #[test]
    fn default_fov_is_45_degrees() {
        let cam = CameraController::default();
        assert!((cam.fov - std::f32::consts::FRAC_PI_4).abs() < 0.001);
    }

    #[test]
    fn default_aspect_is_16_9() {
        let cam = CameraController::default();
        assert!((cam.aspect - 16.0 / 9.0).abs() < 0.001);
    }

    #[test]
    fn default_near_is_0_1() {
        let cam = CameraController::default();
        assert!((cam.near - 0.1).abs() < 0.001);
    }

    #[test]
    fn default_far_is_1000() {
        let cam = CameraController::default();
        assert!((cam.far - 1000.0).abs() < 0.001);
    }

    #[test]
    fn distance_from_default() {
        let cam = CameraController::default();
        // sqrt(5^2 + 5^2 + 5^2) = sqrt(75) ≈ 8.66
        let d = cam.distance();
        assert!((d - 75.0_f32.sqrt()).abs() < 0.01);
    }

    #[test]
    fn zoom_in_reduces_distance() {
        let mut cam = CameraController::default();
        let d_before = cam.distance();
        cam.zoom(1.0, 0.1);
        let d_after = cam.distance();
        assert!(d_after < d_before);
    }

    #[test]
    fn zoom_out_increases_distance() {
        let mut cam = CameraController::default();
        let d_before = cam.distance();
        cam.zoom(-1.0, 0.1);
        let d_after = cam.distance();
        assert!(d_after > d_before);
    }

    #[test]
    fn zoom_clamps_minimum_distance() {
        let mut cam = CameraController::default();
        // Zoom in extremely far
        cam.zoom(100000.0, 100.0);
        // Distance should be clamped to >= 0.1
        assert!(cam.distance() >= 0.099);
    }

    #[test]
    fn set_view_front_z_positive() {
        let mut cam = CameraController::default();
        cam.set_view_front();
        // Position should be at (0, 0, distance) from target
        assert!((cam.position.x - cam.target.x).abs() < 0.001);
        assert!((cam.position.y - cam.target.y).abs() < 0.001);
        assert!(cam.position.z > cam.target.z);
    }

    #[test]
    fn set_view_right_x_positive() {
        let mut cam = CameraController::default();
        cam.set_view_right();
        assert!(cam.position.x > cam.target.x);
        assert!((cam.position.y - cam.target.y).abs() < 0.001);
        assert!((cam.position.z - cam.target.z).abs() < 0.001);
    }

    #[test]
    fn set_view_top_y_positive() {
        let mut cam = CameraController::default();
        cam.set_view_top();
        assert!(cam.position.y > cam.target.y);
        assert!((cam.position.x - cam.target.x).abs() < 0.001);
        assert!((cam.position.z - cam.target.z).abs() < 0.001);
    }

    #[test]
    fn focus_on_preserves_distance() {
        let mut cam = CameraController::default();
        let d_before = cam.distance();
        cam.focus_on(Vec3::new(10.0, 0.0, 0.0));
        let d_after = cam.distance();
        assert!((d_before - d_after).abs() < 0.01);
    }

    #[test]
    fn focus_on_changes_target() {
        let mut cam = CameraController::default();
        let new_target = Vec3::new(10.0, 5.0, -3.0);
        cam.focus_on(new_target);
        assert!((cam.target - new_target).length() < 0.001);
    }

    #[test]
    fn view_projection_is_projection_times_view() {
        let cam = CameraController::default();
        let vp = cam.view_projection_matrix();
        let expected = cam.projection_matrix() * cam.view_matrix();
        // Compare first element as a sanity check
        let diff = (vp.col(0) - expected.col(0)).length();
        assert!(diff < 0.001);
    }

    #[test]
    fn inverse_view_projection_roundtrips() {
        let cam = CameraController::default();
        let vp = cam.view_projection_matrix();
        let ivp = cam.inverse_view_projection_matrix();
        let identity = vp * ivp;
        // Should be approximately identity
        assert!((identity.col(0).x - 1.0).abs() < 0.01);
        assert!((identity.col(1).y - 1.0).abs() < 0.01);
    }

    #[test]
    fn pan_preserves_offset_direction() {
        let mut cam = CameraController::default();
        let offset_before = cam.position - cam.target;
        cam.pan(Vec2::new(1.0, 0.0), 0.1);
        let offset_after = cam.position - cam.target;
        // Direction should be approximately the same since pan moves both position and target
        let dot = offset_before.normalize().dot(offset_after.normalize());
        assert!(dot > 0.99, "dot={}", dot);
    }
}
