//! Wave 2 Mutation Remediation — Distribution & Polish Systems
//!
//! Targets mutation-prone patterns in:
//! - distribution.rs: DistributionFormat (7 variants × extension/name/platform/icon/description/is_*),
//!   format_bytes() threshold arithmetic, DistributionResult::formatted_duration() 60s boundary,
//!   DistributionConfig sanitized_name/is_valid, DistributionResult::summary()
//! - polish.rs: LoadingStyle (5 variants × name/icon/shows_progress/is_animated/supports_background),
//!   LoadingProgress::percentage() division, SplashSequence::total_duration(), SaveConfig defaults

use aw_editor_lib::distribution::{
    format_bytes, DistributionConfig, DistributionFormat, DistributionResult,
};
use aw_editor_lib::polish::{
    LoadingProgress, LoadingScreen, LoadingStyle, SaveConfig, SaveMetadata, SplashScreen,
    SplashSequence,
};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// DistributionFormat — 7 Variants × extension
// ============================================================================

#[test]
fn dist_ext_windows_installer() {
    assert_eq!(DistributionFormat::WindowsInstaller.extension(), "exe");
}

#[test]
fn dist_ext_windows_portable() {
    assert_eq!(DistributionFormat::WindowsPortable.extension(), "zip");
}

#[test]
fn dist_ext_macos_bundle() {
    assert_eq!(DistributionFormat::MacOSBundle.extension(), "app");
}

#[test]
fn dist_ext_macos_dmg() {
    assert_eq!(DistributionFormat::MacOSDmg.extension(), "dmg");
}

#[test]
fn dist_ext_linux_appimage() {
    assert_eq!(DistributionFormat::LinuxAppImage.extension(), "AppImage");
}

#[test]
fn dist_ext_linux_tarball() {
    assert_eq!(DistributionFormat::LinuxTarball.extension(), "tar.gz");
}

#[test]
fn dist_ext_steam_depot() {
    assert_eq!(DistributionFormat::SteamDepot.extension(), "vdf");
}

// ============================================================================
// DistributionFormat — 7 Variants × name
// ============================================================================

#[test]
fn dist_name_windows_installer() {
    assert_eq!(
        DistributionFormat::WindowsInstaller.name(),
        "Windows Installer"
    );
}

#[test]
fn dist_name_windows_portable() {
    assert_eq!(
        DistributionFormat::WindowsPortable.name(),
        "Windows Portable"
    );
}

#[test]
fn dist_name_macos_bundle() {
    assert_eq!(DistributionFormat::MacOSBundle.name(), "macOS App Bundle");
}

#[test]
fn dist_name_macos_dmg() {
    assert_eq!(DistributionFormat::MacOSDmg.name(), "macOS DMG");
}

#[test]
fn dist_name_linux_appimage() {
    assert_eq!(DistributionFormat::LinuxAppImage.name(), "Linux AppImage");
}

#[test]
fn dist_name_linux_tarball() {
    assert_eq!(DistributionFormat::LinuxTarball.name(), "Linux Tarball");
}

#[test]
fn dist_name_steam_depot() {
    assert_eq!(DistributionFormat::SteamDepot.name(), "Steam Depot");
}

// ============================================================================
// DistributionFormat — platform classification
// ============================================================================

#[test]
fn dist_platform_windows() {
    assert_eq!(DistributionFormat::WindowsInstaller.platform(), "Windows");
    assert_eq!(DistributionFormat::WindowsPortable.platform(), "Windows");
}

#[test]
fn dist_platform_macos() {
    assert_eq!(DistributionFormat::MacOSBundle.platform(), "macOS");
    assert_eq!(DistributionFormat::MacOSDmg.platform(), "macOS");
}

#[test]
fn dist_platform_linux() {
    assert_eq!(DistributionFormat::LinuxAppImage.platform(), "Linux");
    assert_eq!(DistributionFormat::LinuxTarball.platform(), "Linux");
}

#[test]
fn dist_platform_steam() {
    assert_eq!(DistributionFormat::SteamDepot.platform(), "Steam");
}

// ============================================================================
// DistributionFormat — icon per variant
// ============================================================================

#[test]
fn dist_icon_windows_installer() {
    assert_eq!(DistributionFormat::WindowsInstaller.icon(), "💿");
}

#[test]
fn dist_icon_windows_portable() {
    assert_eq!(DistributionFormat::WindowsPortable.icon(), "[Pkg]");
}

#[test]
fn dist_icon_macos_bundle() {
    assert_eq!(DistributionFormat::MacOSBundle.icon(), "🍎");
}

#[test]
fn dist_icon_macos_dmg() {
    assert_eq!(DistributionFormat::MacOSDmg.icon(), "[Save]");
}

#[test]
fn dist_icon_linux_appimage() {
    assert_eq!(DistributionFormat::LinuxAppImage.icon(), "🐧");
}

#[test]
fn dist_icon_linux_tarball() {
    assert_eq!(DistributionFormat::LinuxTarball.icon(), "[Dir]");
}

#[test]
fn dist_icon_steam_depot() {
    assert_eq!(DistributionFormat::SteamDepot.icon(), "[Gp]");
}

// ============================================================================
// DistributionFormat — description per variant (unique, non-empty)
// ============================================================================

#[test]
fn dist_description_windows_installer() {
    assert!(DistributionFormat::WindowsInstaller
        .description()
        .contains("Installable"));
}

#[test]
fn dist_description_windows_portable() {
    assert!(DistributionFormat::WindowsPortable
        .description()
        .contains("Portable"));
}

#[test]
fn dist_description_macos_bundle() {
    assert!(DistributionFormat::MacOSBundle
        .description()
        .contains("macOS"));
}

#[test]
fn dist_description_macos_dmg() {
    assert!(DistributionFormat::MacOSDmg
        .description()
        .contains("Disk image"));
}

#[test]
fn dist_description_linux_appimage() {
    assert!(DistributionFormat::LinuxAppImage
        .description()
        .contains("portable"));
}

#[test]
fn dist_description_linux_tarball() {
    assert!(DistributionFormat::LinuxTarball
        .description()
        .contains("archive"));
}

#[test]
fn dist_description_steam_depot() {
    assert!(DistributionFormat::SteamDepot
        .description()
        .contains("Steam"));
}

// ============================================================================
// DistributionFormat — is_* classification
// ============================================================================

#[test]
fn dist_is_windows_true() {
    assert!(DistributionFormat::WindowsInstaller.is_windows());
    assert!(DistributionFormat::WindowsPortable.is_windows());
}

#[test]
fn dist_is_windows_false_for_non_windows() {
    for &f in &[
        DistributionFormat::MacOSBundle,
        DistributionFormat::MacOSDmg,
        DistributionFormat::LinuxAppImage,
        DistributionFormat::LinuxTarball,
        DistributionFormat::SteamDepot,
    ] {
        assert!(!f.is_windows(), "{:?} should not be windows", f);
    }
}

#[test]
fn dist_is_macos_true() {
    assert!(DistributionFormat::MacOSBundle.is_macos());
    assert!(DistributionFormat::MacOSDmg.is_macos());
}

#[test]
fn dist_is_macos_false_for_non_macos() {
    for &f in &[
        DistributionFormat::WindowsInstaller,
        DistributionFormat::WindowsPortable,
        DistributionFormat::LinuxAppImage,
        DistributionFormat::LinuxTarball,
        DistributionFormat::SteamDepot,
    ] {
        assert!(!f.is_macos(), "{:?} should not be macos", f);
    }
}

#[test]
fn dist_is_linux_true() {
    assert!(DistributionFormat::LinuxAppImage.is_linux());
    assert!(DistributionFormat::LinuxTarball.is_linux());
}

#[test]
fn dist_is_linux_false_for_non_linux() {
    for &f in &[
        DistributionFormat::WindowsInstaller,
        DistributionFormat::WindowsPortable,
        DistributionFormat::MacOSBundle,
        DistributionFormat::MacOSDmg,
        DistributionFormat::SteamDepot,
    ] {
        assert!(!f.is_linux(), "{:?} should not be linux", f);
    }
}

#[test]
fn dist_is_steam_true() {
    assert!(DistributionFormat::SteamDepot.is_steam());
}

#[test]
fn dist_is_steam_false_for_non_steam() {
    for &f in &[
        DistributionFormat::WindowsInstaller,
        DistributionFormat::WindowsPortable,
        DistributionFormat::MacOSBundle,
        DistributionFormat::MacOSDmg,
        DistributionFormat::LinuxAppImage,
        DistributionFormat::LinuxTarball,
    ] {
        assert!(!f.is_steam(), "{:?} should not be steam", f);
    }
}

#[test]
fn dist_all_returns_7() {
    assert_eq!(DistributionFormat::all().len(), 7);
}

#[test]
fn dist_display_matches_name() {
    for &f in DistributionFormat::all() {
        assert_eq!(format!("{}", f), f.name());
    }
}

// ============================================================================
// format_bytes — threshold boundaries
// ============================================================================

#[test]
fn format_bytes_zero() {
    assert_eq!(format_bytes(0), "0 B");
}

#[test]
fn format_bytes_1_byte() {
    assert_eq!(format_bytes(1), "1 B");
}

#[test]
fn format_bytes_1023_bytes() {
    // Just below KB threshold (1024)
    assert_eq!(format_bytes(1023), "1023 B");
}

#[test]
fn format_bytes_exactly_1_kb() {
    assert_eq!(format_bytes(1024), "1.00 KB");
}

#[test]
fn format_bytes_1025_bytes() {
    // Just above KB
    let s = format_bytes(1025);
    assert!(s.contains("KB"));
}

#[test]
fn format_bytes_just_below_mb() {
    // 1MB - 1 = 1048575
    let s = format_bytes(1048575);
    assert!(s.contains("KB"));
}

#[test]
fn format_bytes_exactly_1_mb() {
    assert_eq!(format_bytes(1048576), "1.00 MB");
}

#[test]
fn format_bytes_1_5_mb() {
    assert_eq!(format_bytes(1_572_864), "1.50 MB");
}

#[test]
fn format_bytes_just_below_gb() {
    // 1GB - 1 = 1073741823
    let s = format_bytes(1073741823);
    assert!(s.contains("MB"));
}

#[test]
fn format_bytes_exactly_1_gb() {
    assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
}

#[test]
fn format_bytes_2_5_gb() {
    let s = format_bytes(2_684_354_560);
    assert!(s.contains("2.50 GB"));
}

// ============================================================================
// DistributionConfig — validation and helpers
// ============================================================================

#[test]
fn dist_config_default_is_valid() {
    assert!(DistributionConfig::default().is_valid());
}

#[test]
fn dist_config_empty_name_invalid() {
    let c = DistributionConfig {
        game_name: "".into(),
        ..Default::default()
    };
    assert!(!c.is_valid());
}

#[test]
fn dist_config_empty_version_invalid() {
    let c = DistributionConfig {
        version: "".into(),
        ..Default::default()
    };
    assert!(!c.is_valid());
}

#[test]
fn dist_config_empty_publisher_invalid() {
    let c = DistributionConfig {
        publisher: "".into(),
        ..Default::default()
    };
    assert!(!c.is_valid());
}

#[test]
fn dist_config_has_steam_config() {
    assert!(!DistributionConfig::default().has_steam_config());
    let c = DistributionConfig {
        steam_app_id: Some(480),
        ..Default::default()
    };
    assert!(c.has_steam_config());
}

#[test]
fn dist_config_has_custom_icon() {
    assert!(!DistributionConfig::default().has_custom_icon());
    let c = DistributionConfig {
        icon_path: Some(PathBuf::from("icon.png")),
        ..Default::default()
    };
    assert!(c.has_custom_icon());
}

#[test]
fn dist_config_sanitized_name_replaces_spaces() {
    let c = DistributionConfig {
        game_name: "My Cool Game".into(),
        ..Default::default()
    };
    assert_eq!(c.sanitized_name(), "My_Cool_Game");
}

#[test]
fn dist_config_sanitized_name_removes_special_chars() {
    let c = DistributionConfig {
        game_name: "Game: The <Sequel>".into(),
        ..Default::default()
    };
    let s = c.sanitized_name();
    assert!(!s.contains(':'));
    assert!(!s.contains('<'));
    assert!(!s.contains('>'));
}

#[test]
fn dist_config_summary_format() {
    let c = DistributionConfig {
        game_name: "TestGame".into(),
        version: "2.0.0".into(),
        publisher: "TestPub".into(),
        ..Default::default()
    };
    let s = c.summary();
    assert!(s.contains("TestGame"));
    assert!(s.contains("2.0.0"));
    assert!(s.contains("TestPub"));
}

// ============================================================================
// DistributionResult — formatted_duration 60s boundary
// ============================================================================

#[test]
fn dist_result_duration_under_60s() {
    let r = DistributionResult {
        output_path: PathBuf::from("out.zip"),
        format: DistributionFormat::WindowsPortable,
        size_bytes: 1000,
        duration_secs: 30.5,
    };
    assert_eq!(r.formatted_duration(), "30.5s");
}

#[test]
fn dist_result_duration_exactly_60s() {
    let r = DistributionResult {
        output_path: PathBuf::from("out.zip"),
        format: DistributionFormat::WindowsPortable,
        size_bytes: 1000,
        duration_secs: 60.0,
    };
    // 60.0 is NOT < 60.0, so goes to else branch
    let d = r.formatted_duration();
    assert!(d.contains("m"));
}

#[test]
fn dist_result_duration_just_under_60s() {
    let r = DistributionResult {
        output_path: PathBuf::from("out.zip"),
        format: DistributionFormat::WindowsPortable,
        size_bytes: 1000,
        duration_secs: 59.9,
    };
    assert!(r.formatted_duration().contains("s"));
    assert!(!r.formatted_duration().contains("m"));
}

#[test]
fn dist_result_duration_125s() {
    let r = DistributionResult {
        output_path: PathBuf::from("out.zip"),
        format: DistributionFormat::WindowsPortable,
        size_bytes: 1000,
        duration_secs: 125.0,
    };
    let d = r.formatted_duration();
    assert!(d.contains("2m")); // 125/60 = 2.08, floor = 2
}

#[test]
fn dist_result_formatted_size() {
    let r = DistributionResult {
        output_path: PathBuf::from("out.zip"),
        format: DistributionFormat::WindowsPortable,
        size_bytes: 1_572_864,
        duration_secs: 1.0,
    };
    assert_eq!(r.formatted_size(), "1.50 MB");
}

#[test]
fn dist_result_file_name() {
    let r = DistributionResult {
        output_path: PathBuf::from("dist/game.zip"),
        format: DistributionFormat::WindowsPortable,
        size_bytes: 100,
        duration_secs: 1.0,
    };
    assert_eq!(r.file_name(), Some("game.zip"));
}

#[test]
fn dist_result_summary_contains_file_name() {
    let r = DistributionResult {
        output_path: PathBuf::from("game.zip"),
        format: DistributionFormat::WindowsPortable,
        size_bytes: 1024,
        duration_secs: 2.0,
    };
    let s = r.summary();
    assert!(s.contains("game.zip"));
    assert!(s.contains(DistributionFormat::WindowsPortable.icon()));
}

// ============================================================================
// LoadingStyle — 5 Variants × name
// ============================================================================

#[test]
fn loading_style_name_spinner() {
    assert_eq!(LoadingStyle::Spinner.name(), "Spinner");
}

#[test]
fn loading_style_name_progress_bar() {
    assert_eq!(LoadingStyle::ProgressBar.name(), "Progress Bar");
}

#[test]
fn loading_style_name_full_screen() {
    assert_eq!(LoadingStyle::FullScreen.name(), "Full Screen");
}

#[test]
fn loading_style_name_dots() {
    assert_eq!(LoadingStyle::Dots.name(), "Dots");
}

#[test]
fn loading_style_name_artwork_with_tips() {
    assert_eq!(LoadingStyle::ArtworkWithTips.name(), "Artwork With Tips");
}

// ============================================================================
// LoadingStyle — icon per variant
// ============================================================================

#[test]
fn loading_style_icon_spinner() {
    assert_eq!(LoadingStyle::Spinner.icon(), "[Sync]");
}

#[test]
fn loading_style_icon_progress_bar() {
    assert_eq!(LoadingStyle::ProgressBar.icon(), "█");
}

#[test]
fn loading_style_icon_full_screen() {
    assert_eq!(LoadingStyle::FullScreen.icon(), "[Img]");
}

#[test]
fn loading_style_icon_dots() {
    assert_eq!(LoadingStyle::Dots.icon(), "•••");
}

#[test]
fn loading_style_icon_artwork_with_tips() {
    assert_eq!(LoadingStyle::ArtworkWithTips.icon(), "[Art]");
}

// ============================================================================
// LoadingStyle — shows_progress
// ============================================================================

#[test]
fn loading_style_shows_progress_true() {
    assert!(LoadingStyle::ProgressBar.shows_progress());
    assert!(LoadingStyle::FullScreen.shows_progress());
    assert!(LoadingStyle::ArtworkWithTips.shows_progress());
}

#[test]
fn loading_style_shows_progress_false() {
    assert!(!LoadingStyle::Spinner.shows_progress());
    assert!(!LoadingStyle::Dots.shows_progress());
}

// ============================================================================
// LoadingStyle — is_animated
// ============================================================================

#[test]
fn loading_style_is_animated_true() {
    assert!(LoadingStyle::Spinner.is_animated());
    assert!(LoadingStyle::Dots.is_animated());
}

#[test]
fn loading_style_is_animated_false() {
    assert!(!LoadingStyle::ProgressBar.is_animated());
    assert!(!LoadingStyle::FullScreen.is_animated());
    assert!(!LoadingStyle::ArtworkWithTips.is_animated());
}

// ============================================================================
// LoadingStyle — supports_background
// ============================================================================

#[test]
fn loading_style_supports_background_true() {
    assert!(LoadingStyle::FullScreen.supports_background());
    assert!(LoadingStyle::ArtworkWithTips.supports_background());
}

#[test]
fn loading_style_supports_background_false() {
    assert!(!LoadingStyle::Spinner.supports_background());
    assert!(!LoadingStyle::ProgressBar.supports_background());
    assert!(!LoadingStyle::Dots.supports_background());
}

#[test]
fn loading_style_all_returns_5() {
    assert_eq!(LoadingStyle::all().len(), 5);
}

#[test]
fn loading_style_default_is_progress_bar() {
    assert_eq!(LoadingStyle::default(), LoadingStyle::ProgressBar);
}

#[test]
fn loading_style_display_matches_name() {
    for &s in LoadingStyle::all() {
        assert_eq!(format!("{}", s), s.name());
    }
}

// ============================================================================
// LoadingProgress — percentage() division + boundary
// ============================================================================

#[test]
fn progress_percentage_zero_tasks_returns_1() {
    let p = LoadingProgress::new(0);
    assert!((p.percentage() - 1.0).abs() < 0.001);
}

#[test]
fn progress_percentage_none_completed() {
    let p = LoadingProgress::new(10);
    assert!((p.percentage() - 0.0).abs() < 0.001);
}

#[test]
fn progress_percentage_half_completed() {
    let mut p = LoadingProgress::new(10);
    for i in 0..5 {
        p.advance(format!("task {}", i));
    }
    assert!((p.percentage() - 0.5).abs() < 0.001);
}

#[test]
fn progress_percentage_all_completed() {
    let mut p = LoadingProgress::new(4);
    for i in 0..4 {
        p.advance(format!("task {}", i));
    }
    assert!((p.percentage() - 1.0).abs() < 0.001);
}

#[test]
fn progress_percentage_one_of_three() {
    let mut p = LoadingProgress::new(3);
    p.advance("first");
    // 1/3 = 0.333...
    assert!((p.percentage() - 0.333).abs() < 0.01);
}

#[test]
fn progress_is_complete_false_initially() {
    let p = LoadingProgress::new(5);
    assert!(!p.is_complete());
}

#[test]
fn progress_is_complete_true_when_all_done() {
    let mut p = LoadingProgress::new(2);
    p.advance("a");
    assert!(!p.is_complete());
    p.advance("b");
    assert!(p.is_complete());
}

#[test]
fn progress_is_complete_true_when_exceeded() {
    let mut p = LoadingProgress::new(1);
    p.advance("a");
    p.advance("b"); // exceeded
    assert!(p.is_complete());
}

#[test]
fn progress_advance_updates_current_task() {
    let mut p = LoadingProgress::new(5);
    p.advance("Loading textures");
    assert_eq!(p.current_task, "Loading textures");
    assert_eq!(p.completed_tasks, 1);
}

#[test]
fn progress_elapsed_is_non_negative() {
    let p = LoadingProgress::new(1);
    assert!(p.elapsed() >= Duration::ZERO);
}

// ============================================================================
// SplashSequence — total_duration sum
// ============================================================================

#[test]
fn splash_sequence_empty_duration_zero() {
    let seq = SplashSequence::new();
    assert_eq!(seq.total_duration(), Duration::ZERO);
}

#[test]
fn splash_sequence_single_screen_duration() {
    let seq = SplashSequence::new().add_screen(SplashScreen {
        duration: Some(Duration::from_secs(3)),
        fade_in: Duration::from_millis(500),
        fade_out: Duration::from_millis(500),
        ..Default::default()
    });
    // 3000 + 500 + 500 = 4000ms
    assert_eq!(seq.total_duration(), Duration::from_millis(4000));
}

#[test]
fn splash_sequence_multiple_screens_sum() {
    let seq = SplashSequence::new()
        .with_engine_logo()
        .with_publisher_logo("pub.png");
    assert!(seq.total_duration() > Duration::ZERO);
    assert_eq!(seq.screens.len(), 2);
}

#[test]
fn splash_sequence_none_duration_treated_as_zero() {
    let seq = SplashSequence::new().add_screen(SplashScreen {
        duration: None,
        fade_in: Duration::from_millis(200),
        fade_out: Duration::from_millis(300),
        ..Default::default()
    });
    // None => Duration::ZERO + 200 + 300 = 500ms
    assert_eq!(seq.total_duration(), Duration::from_millis(500));
}

// ============================================================================
// SplashScreen — defaults
// ============================================================================

#[test]
fn splash_screen_default_skippable() {
    assert!(SplashScreen::default().skippable);
}

#[test]
fn splash_screen_default_has_duration() {
    assert!(SplashScreen::default().duration.is_some());
}

// ============================================================================
// LoadingScreen — builder
// ============================================================================

#[test]
fn loading_screen_add_tip() {
    let ls = LoadingScreen::default().add_tip("Tip 1").add_tip("Tip 2");
    assert_eq!(ls.tips.len(), 2);
    assert_eq!(ls.tips[0], "Tip 1");
}

#[test]
fn loading_screen_with_tips() {
    let ls = LoadingScreen::default().with_tips(vec!["A".into(), "B".into(), "C".into()]);
    assert_eq!(ls.tips.len(), 3);
}

#[test]
fn loading_screen_default_values() {
    let ls = LoadingScreen::default();
    assert!(ls.show_percentage);
    assert!(ls.show_task_description);
    assert_eq!(ls.style, LoadingStyle::ProgressBar);
    assert!(ls.tips.is_empty());
}

// ============================================================================
// SaveConfig — defaults
// ============================================================================

#[test]
fn save_config_default_extension() {
    assert_eq!(SaveConfig::default().extension, "sav");
}

#[test]
fn save_config_default_directory() {
    assert_eq!(SaveConfig::default().directory, "saves");
}

#[test]
fn save_config_default_max_autosaves() {
    assert_eq!(SaveConfig::default().max_autosaves, 3);
}

#[test]
fn save_config_default_compress() {
    assert!(SaveConfig::default().compress);
}

#[test]
fn save_config_default_include_screenshot() {
    assert!(SaveConfig::default().include_screenshot);
}

#[test]
fn save_config_default_autosave_interval() {
    let interval = SaveConfig::default().autosave_interval;
    assert!(interval.is_some());
    assert_eq!(interval.unwrap(), Duration::from_secs(300));
}

// ============================================================================
// SaveMetadata — constructor
// ============================================================================

#[test]
fn save_metadata_new() {
    let m = SaveMetadata::new("Quick Save", "1.0.0");
    assert_eq!(m.name, "Quick Save");
    assert_eq!(m.version, "1.0.0");
    assert!(!m.is_autosave);
    assert!(m.timestamp > 0);
    assert_eq!(m.playtime_seconds, 0);
    assert_eq!(m.location, "Unknown");
    assert!(m.screenshot.is_none());
}
