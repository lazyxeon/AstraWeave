//! Wave 2 mutation remediation tests — Distribution + Build Manager panels
//! Covers: BuildProfile (distribution), TargetPlatform, BuildOptions, AssetOptions,
//!         TextureFormat, AudioFormat, BuildStep, BuildProgress,
//!         BuildTarget (build_manager), BuildProfile (build_manager),
//!         BuildStatus, BuildConfig, BuildMessage

use aw_editor_lib::panels::build_manager::{
    BuildConfig, BuildManagerPanel, BuildMessage, BuildProfile as BmBuildProfile, BuildStatus,
    BuildTarget,
};
use aw_editor_lib::panels::distribution_panel::{
    AssetOptions, AudioFormat, BuildOptions, BuildProfile as DistBuildProfile, BuildProgress,
    BuildStep, TargetPlatform, TextureFormat,
};

// ═══════════════════════════════════════════════════════════════════════════════════
// DISTRIBUTION PANEL — BuildProfile
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn dist_build_profile_all_count() {
    assert_eq!(DistBuildProfile::all().len(), 4);
}

#[test]
fn dist_build_profile_names() {
    assert_eq!(DistBuildProfile::Debug.name(), "Debug");
    assert_eq!(DistBuildProfile::Release.name(), "Release");
    assert_eq!(
        DistBuildProfile::ReleaseOptimized.name(),
        "Release (Optimized)"
    );
    assert_eq!(DistBuildProfile::MinSize.name(), "Minimum Size");
}

#[test]
fn dist_build_profile_icons_non_empty() {
    for p in DistBuildProfile::all() {
        assert!(!p.icon().is_empty(), "{:?} icon empty", p);
    }
}

#[test]
fn dist_build_profile_description_non_empty() {
    for p in DistBuildProfile::all() {
        assert!(!p.description().is_empty(), "{:?} desc empty", p);
    }
}

#[test]
fn dist_build_profile_cargo_profile() {
    assert_eq!(DistBuildProfile::Debug.cargo_profile(), "debug");
    assert_eq!(DistBuildProfile::Release.cargo_profile(), "release");
    assert_eq!(
        DistBuildProfile::ReleaseOptimized.cargo_profile(),
        "release-lto"
    );
    assert_eq!(DistBuildProfile::MinSize.cargo_profile(), "release-small");
}

#[test]
fn dist_build_profile_is_release() {
    assert!(!DistBuildProfile::Debug.is_release());
    assert!(DistBuildProfile::Release.is_release());
    assert!(DistBuildProfile::ReleaseOptimized.is_release());
    assert!(DistBuildProfile::MinSize.is_release());
}

#[test]
fn dist_build_profile_default_is_release() {
    assert_eq!(DistBuildProfile::default(), DistBuildProfile::Release);
}

#[test]
fn dist_build_profile_display() {
    for p in DistBuildProfile::all() {
        let s = format!("{}", p);
        assert!(s.contains(p.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// DISTRIBUTION PANEL — TargetPlatform
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn target_platform_all_count() {
    assert_eq!(TargetPlatform::all().len(), 8);
}

#[test]
fn target_platform_names() {
    assert_eq!(TargetPlatform::Native.name(), "Native");
    assert_eq!(TargetPlatform::Windows64.name(), "Windows x64");
    assert_eq!(TargetPlatform::Windows32.name(), "Windows x86");
    assert_eq!(TargetPlatform::LinuxX64.name(), "Linux x64");
    assert_eq!(TargetPlatform::LinuxArm64.name(), "Linux ARM64");
}

#[test]
fn target_platform_icons_non_empty() {
    for p in TargetPlatform::all() {
        assert!(!p.icon().is_empty(), "{:?} icon empty", p);
    }
}

#[test]
fn target_platform_rust_target() {
    assert!(TargetPlatform::Native.rust_target().is_none());
    assert_eq!(
        TargetPlatform::Windows64.rust_target(),
        Some("x86_64-pc-windows-msvc")
    );
    assert_eq!(
        TargetPlatform::LinuxX64.rust_target(),
        Some("x86_64-unknown-linux-gnu")
    );
}

#[test]
fn target_platform_is_cross_compile() {
    assert!(!TargetPlatform::Native.is_cross_compile());
    assert!(TargetPlatform::Windows64.is_cross_compile());
    assert!(TargetPlatform::LinuxArm64.is_cross_compile());
}

#[test]
fn target_platform_is_64bit() {
    assert!(TargetPlatform::Native.is_64bit());
    assert!(TargetPlatform::Windows64.is_64bit());
    assert!(!TargetPlatform::Windows32.is_64bit());
    assert!(TargetPlatform::LinuxX64.is_64bit());
}

#[test]
fn target_platform_default_is_native() {
    assert_eq!(TargetPlatform::default(), TargetPlatform::Native);
}

#[test]
fn target_platform_display() {
    for p in TargetPlatform::all() {
        let s = format!("{}", p);
        assert!(s.contains(p.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// DISTRIBUTION PANEL — TextureFormat
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn texture_format_all_count() {
    assert_eq!(TextureFormat::all().len(), 5);
}

#[test]
fn texture_format_names_non_empty() {
    for f in TextureFormat::all() {
        assert!(!f.name().is_empty(), "{:?} name empty", f);
    }
}

#[test]
fn texture_format_is_compressed() {
    assert!(!TextureFormat::None.is_compressed());
    assert!(TextureFormat::BC7.is_compressed());
    assert!(TextureFormat::BC5.is_compressed());
    assert!(TextureFormat::ASTC.is_compressed());
    assert!(TextureFormat::ETC2.is_compressed());
}

#[test]
fn texture_format_default_is_bc7() {
    assert_eq!(TextureFormat::default(), TextureFormat::BC7);
}

#[test]
fn texture_format_display() {
    for f in TextureFormat::all() {
        let s = format!("{}", f);
        assert!(s.contains(f.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// DISTRIBUTION PANEL — AudioFormat
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn audio_format_all_count() {
    assert_eq!(AudioFormat::all().len(), 5);
}

#[test]
fn audio_format_is_compressed() {
    assert!(!AudioFormat::None.is_compressed());
    assert!(AudioFormat::Vorbis.is_compressed());
    assert!(AudioFormat::Opus.is_compressed());
    assert!(AudioFormat::MP3.is_compressed());
    assert!(AudioFormat::AAC.is_compressed());
}

#[test]
fn audio_format_default_is_vorbis() {
    assert_eq!(AudioFormat::default(), AudioFormat::Vorbis);
}

#[test]
fn audio_format_display() {
    for f in AudioFormat::all() {
        let s = format!("{}", f);
        assert!(s.contains(f.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// DISTRIBUTION PANEL — BuildStep
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn build_step_all_steps_count() {
    assert_eq!(BuildStep::all_steps().len(), 11);
}

#[test]
fn build_step_names_non_empty() {
    for s in BuildStep::all_steps() {
        assert!(!s.name().is_empty(), "{:?} name empty", s);
    }
}

#[test]
fn build_step_icons_non_empty() {
    for s in BuildStep::all_steps() {
        assert!(!s.icon().is_empty(), "{:?} icon empty", s);
    }
}

#[test]
fn build_step_is_terminal() {
    assert!(!BuildStep::Preparing.is_terminal());
    assert!(!BuildStep::CompilingCode.is_terminal());
    assert!(BuildStep::Complete.is_terminal());
    assert!(BuildStep::Failed.is_terminal());
}

#[test]
fn build_step_display() {
    for s in BuildStep::all_steps() {
        let d = format!("{}", s);
        assert!(d.contains(s.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// DISTRIBUTION PANEL — BuildOptions defaults
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn build_options_defaults() {
    let o = BuildOptions::default();
    assert!(o.strip_symbols);
    assert!(o.compress_assets);
    assert!(!o.embed_runtime);
    assert!(!o.sign_binary);
    assert!(!o.notarize_macos);
    assert!(o.create_installer);
    assert!(o.generate_checksums);
    assert!(!o.include_debug_symbols);
    assert!(o.run_tests_before_build);
    assert!(!o.clean_before_build);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// DISTRIBUTION PANEL — AssetOptions defaults
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn asset_options_defaults() {
    let o = AssetOptions::default();
    assert!(o.compress_textures);
    assert!(o.compress_audio);
    assert!(o.compress_meshes);
    assert!(o.pack_into_archives);
    assert!(!o.encrypt_assets);
    assert!(o.generate_manifests);
    assert_eq!(o.texture_format, TextureFormat::BC7);
    assert_eq!(o.audio_format, AudioFormat::Vorbis);
    assert_eq!(o.max_texture_size, 4096);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// DISTRIBUTION PANEL — BuildProgress
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn build_progress_defaults() {
    let p = BuildProgress::default();
    assert_eq!(p.current_step, BuildStep::Preparing);
    assert!((p.step_progress - 0.0).abs() < f32::EPSILON);
    assert!((p.overall_progress - 0.0).abs() < f32::EPSILON);
    assert!(p.status_message.is_empty());
    assert!(p.start_time.is_none());
    assert!(p.warnings.is_empty());
    assert!(p.errors.is_empty());
}

#[test]
fn build_progress_elapsed_secs_none() {
    let p = BuildProgress::default();
    assert!((p.elapsed_secs() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn build_progress_elapsed_secs_with_time() {
    let p = BuildProgress {
        start_time: Some(std::time::Instant::now()),
        ..Default::default()
    };
    // Should be very small but > 0
    assert!(p.elapsed_secs() >= 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// BUILD MANAGER — BuildTarget
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn bm_build_target_all_count() {
    assert_eq!(BuildTarget::all().len(), 4);
}

#[test]
fn bm_build_target_names_non_empty() {
    for t in BuildTarget::all() {
        assert!(!t.name().is_empty(), "{:?} name empty", t);
    }
}

#[test]
fn bm_build_target_icons_non_empty() {
    for t in BuildTarget::all() {
        assert!(!t.icon().is_empty(), "{:?} icon empty", t);
    }
}

#[test]
fn bm_build_target_cargo_target() {
    assert!(BuildTarget::Windows.cargo_target().is_none());
    assert_eq!(
        BuildTarget::Linux.cargo_target(),
        Some("x86_64-unknown-linux-gnu")
    );
    assert_eq!(
        BuildTarget::Web.cargo_target(),
        Some("wasm32-unknown-unknown")
    );
}

#[test]
fn bm_build_target_is_desktop() {
    assert!(BuildTarget::Windows.is_desktop());
    assert!(BuildTarget::Linux.is_desktop());
    assert!(BuildTarget::MacOS.is_desktop());
    assert!(!BuildTarget::Web.is_desktop());
}

#[test]
fn bm_build_target_is_crosscompile() {
    assert!(!BuildTarget::Windows.is_crosscompile());
    assert!(BuildTarget::Linux.is_crosscompile());
    assert!(BuildTarget::MacOS.is_crosscompile());
    assert!(BuildTarget::Web.is_crosscompile());
}

#[test]
fn bm_build_target_default_is_windows() {
    assert_eq!(BuildTarget::default(), BuildTarget::Windows);
}

#[test]
fn bm_build_target_display() {
    for t in BuildTarget::all() {
        let s = format!("{}", t);
        assert!(s.contains(t.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// BUILD MANAGER — BuildProfile
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn bm_build_profile_all_count() {
    assert_eq!(BmBuildProfile::all().len(), 2);
}

#[test]
fn bm_build_profile_names_non_empty() {
    for p in BmBuildProfile::all() {
        assert!(!p.name().is_empty(), "{:?} name empty", p);
    }
}

#[test]
fn bm_build_profile_cargo_flag() {
    assert!(BmBuildProfile::Debug.cargo_flag().is_none());
    assert_eq!(BmBuildProfile::Release.cargo_flag(), Some("--release"));
}

#[test]
fn bm_build_profile_is_optimized() {
    assert!(!BmBuildProfile::Debug.is_optimized());
    assert!(BmBuildProfile::Release.is_optimized());
}

#[test]
fn bm_build_profile_default_is_release() {
    assert_eq!(BmBuildProfile::default(), BmBuildProfile::Release);
}

#[test]
fn bm_build_profile_display() {
    for p in BmBuildProfile::all() {
        let s = format!("{}", p);
        assert!(s.contains(p.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// BUILD MANAGER — BuildStatus
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn build_status_default_is_idle() {
    assert!(matches!(BuildStatus::default(), BuildStatus::Idle));
}

#[test]
fn build_status_is_building() {
    let s = BuildStatus::Building {
        progress: 0.5,
        current_step: "Compiling".to_string(),
    };
    assert!(s.is_building());
    assert!(!s.is_success());
    assert!(!s.is_failed());
}

#[test]
fn build_status_is_success() {
    let s = BuildStatus::Success {
        output_path: "build/out".into(),
        duration_secs: 10.0,
    };
    assert!(!s.is_building());
    assert!(s.is_success());
    assert!(!s.is_failed());
}

#[test]
fn build_status_is_failed() {
    let s = BuildStatus::Failed {
        error_message: "link error".to_string(),
    };
    assert!(!s.is_building());
    assert!(!s.is_success());
    assert!(s.is_failed());
}

#[test]
fn build_status_icon() {
    assert_eq!(BuildStatus::Idle.icon(), "||");
}

#[test]
fn build_status_display() {
    let s = format!("{}", BuildStatus::Idle);
    assert!(s.contains("Idle"));
}

// ═══════════════════════════════════════════════════════════════════════════════════
// BUILD MANAGER — BuildConfig
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn build_config_defaults() {
    let c = BuildConfig::default();
    assert_eq!(c.target, BuildTarget::Windows);
    assert_eq!(c.profile, BmBuildProfile::Release);
    assert_eq!(c.project_name, "AstraWeaveGame");
    assert!(!c.include_debug_symbols);
    assert!(c.strip_unused_assets);
    assert!(c.compress_assets);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// BUILD MANAGER — BuildMessage
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn build_message_is_error() {
    let msg = BuildMessage::Failed {
        error: "oom".to_string(),
    };
    assert!(msg.is_error());
    let msg2 = BuildMessage::LogLine("ok".to_string());
    assert!(!msg2.is_error());
}

#[test]
fn build_message_is_terminal() {
    assert!(BuildMessage::Complete {
        output_path: "out".into(),
        duration_secs: 1.0
    }
    .is_terminal());
    assert!(BuildMessage::Failed {
        error: "err".to_string()
    }
    .is_terminal());
    assert!(!BuildMessage::LogLine("log".to_string()).is_terminal());
    assert!(!(BuildMessage::Progress {
        percent: 0.5,
        step: "x".to_string()
    })
    .is_terminal());
}

#[test]
fn build_message_icon_non_empty() {
    let msgs: Vec<BuildMessage> = vec![
        BuildMessage::Progress {
            percent: 0.0,
            step: "init".to_string(),
        },
        BuildMessage::LogLine("log".to_string()),
        BuildMessage::Complete {
            output_path: "out".into(),
            duration_secs: 1.0,
        },
        BuildMessage::Failed {
            error: "err".to_string(),
        },
    ];
    for m in &msgs {
        assert!(!m.icon().is_empty());
    }
}

#[test]
fn build_message_display() {
    let m = BuildMessage::LogLine("hello".to_string());
    let s = format!("{}", m);
    assert!(s.contains("hello"));
}
