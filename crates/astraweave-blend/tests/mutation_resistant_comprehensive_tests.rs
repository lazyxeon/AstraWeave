//! Comprehensive mutation-resistant tests for astraweave-blend.
//!
//! Targets all key functions with exact return-value verification to kill
//! cargo-mutants mutations (replace return values, swap operators, etc.).

#![allow(
    clippy::nonminimal_bool,
    clippy::io_other_error,
    clippy::field_reassign_with_default
)]

use std::path::PathBuf;
use std::time::Duration;

use astraweave_blend::cache::{
    CacheEntry, CacheLookup, CacheManifest, CacheMissReason, CacheStats, ConversionCache,
};
use astraweave_blend::conversion::{ConversionJobBuilder, ConversionResult};
use astraweave_blend::discovery::{
    BlenderDiscovery, BlenderDiscoveryConfig, BlenderInstallation, DiscoveryMethod,
};
use astraweave_blend::error::BlendError;
use astraweave_blend::export_script::generate_export_script;
use astraweave_blend::options::*;
use astraweave_blend::progress::{ConversionProgress, ConversionStage, ProgressTracker};
use astraweave_blend::version::{
    BlenderVersion, MINIMUM_BLENDER_VERSION, RECOMMENDED_BLENDER_VERSION,
};

// ============================================================================
// Helper: create a mock BlenderInstallation for builder tests
// ============================================================================
fn mock_installation() -> BlenderInstallation {
    BlenderInstallation {
        executable_path: PathBuf::from("/usr/bin/blender"),
        version: BlenderVersion::new(4, 0, 0),
        discovery_method: DiscoveryMethod::SystemPath,
        install_dir: PathBuf::from("/usr/bin"),
    }
}

// ============================================================================
// MODULE 1: BlenderVersion — as_tuple, meets_minimum, is_recommended, validate
// ============================================================================
mod version_comprehensive_tests {
    use super::*;

    // ---- as_tuple exact values ----
    #[test]
    fn as_tuple_returns_exact_values() {
        let v = BlenderVersion::new(4, 2, 7);
        let (major, minor, patch) = v.as_tuple();
        assert_eq!(major, 4);
        assert_eq!(minor, 2);
        assert_eq!(patch, 7);
    }

    #[test]
    fn as_tuple_zero() {
        let v = BlenderVersion::new(0, 0, 0);
        assert_eq!(v.as_tuple(), (0, 0, 0));
    }

    #[test]
    fn as_tuple_large_values() {
        let v = BlenderVersion::new(100, 200, 300);
        assert_eq!(v.as_tuple(), (100, 200, 300));
    }

    #[test]
    fn as_tuple_minimum_version_matches_fields() {
        let v = MINIMUM_BLENDER_VERSION;
        let (major, minor, patch) = v.as_tuple();
        assert_eq!(major, v.major);
        assert_eq!(minor, v.minor);
        assert_eq!(patch, v.patch);
    }

    // ---- meets_minimum exact boundary ----
    #[test]
    fn meets_minimum_exactly_at_boundary() {
        let v = BlenderVersion::new(2, 93, 0);
        assert!(v.meets_minimum());
    }

    #[test]
    fn meets_minimum_one_below_minor() {
        let v = BlenderVersion::new(2, 92, 0);
        assert!(!v.meets_minimum());
    }

    #[test]
    fn meets_minimum_one_above_minor() {
        let v = BlenderVersion::new(2, 94, 0);
        assert!(v.meets_minimum());
    }

    #[test]
    fn meets_minimum_major_above() {
        let v = BlenderVersion::new(3, 0, 0);
        assert!(v.meets_minimum());
    }

    #[test]
    fn meets_minimum_major_below() {
        let v = BlenderVersion::new(1, 99, 99);
        assert!(!v.meets_minimum());
    }

    // ---- is_recommended exact boundary ----
    #[test]
    fn is_recommended_exactly_at_boundary() {
        let v = BlenderVersion::new(4, 0, 0);
        assert!(v.is_recommended());
    }

    #[test]
    fn is_recommended_below() {
        let v = BlenderVersion::new(3, 99, 99);
        assert!(!v.is_recommended());
    }

    #[test]
    fn is_recommended_above() {
        let v = BlenderVersion::new(4, 1, 0);
        assert!(v.is_recommended());
    }

    // ---- validate ----
    #[test]
    fn validate_ok_for_minimum() {
        let v = BlenderVersion::new(2, 93, 0);
        assert!(v.validate().is_ok());
    }

    #[test]
    fn validate_err_below_minimum() {
        let v = BlenderVersion::new(2, 80, 0);
        assert!(v.validate().is_err());
    }

    // ---- Display ----
    #[test]
    fn display_formatting() {
        let v = BlenderVersion::new(4, 2, 7);
        assert_eq!(v.to_string(), "4.2.7");
    }

    #[test]
    fn display_zero_versions() {
        let v = BlenderVersion::new(0, 0, 0);
        assert_eq!(v.to_string(), "0.0.0");
    }

    // ---- from_version_output edge cases ----
    #[test]
    fn from_version_output_with_trailing_text() {
        let v = BlenderVersion::from_version_output("Blender 3.6.5\n  build: xyz").unwrap();
        assert_eq!(v, BlenderVersion::new(3, 6, 5));
    }

    #[test]
    fn from_version_output_no_patch() {
        let v = BlenderVersion::from_version_output("Blender 4.1").unwrap();
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn from_version_output_invalid_returns_err() {
        assert!(BlenderVersion::from_version_output("Not Blender").is_err());
    }

    #[test]
    fn from_version_output_empty_returns_err() {
        assert!(BlenderVersion::from_version_output("").is_err());
    }

    // ---- FromStr ----
    #[test]
    fn from_str_three_parts() {
        let v: BlenderVersion = "3.6.2".parse().unwrap();
        assert_eq!(v, BlenderVersion::new(3, 6, 2));
    }

    #[test]
    fn from_str_two_parts() {
        let v: BlenderVersion = "4.0".parse().unwrap();
        assert_eq!(v, BlenderVersion::new(4, 0, 0));
    }

    #[test]
    fn from_str_one_part_fails() {
        let result: Result<BlenderVersion, _> = "4".parse();
        assert!(result.is_err());
    }

    #[test]
    fn from_str_non_numeric_fails() {
        let result: Result<BlenderVersion, _> = "a.b.c".parse();
        assert!(result.is_err());
    }

    // ---- Ord / PartialOrd ----
    #[test]
    fn ordering_major_takes_priority() {
        let v1 = BlenderVersion::new(2, 99, 99);
        let v2 = BlenderVersion::new(3, 0, 0);
        assert!(v1 < v2);
    }

    #[test]
    fn ordering_minor_within_same_major() {
        let v1 = BlenderVersion::new(3, 5, 0);
        let v2 = BlenderVersion::new(3, 6, 0);
        assert!(v1 < v2);
    }

    #[test]
    fn ordering_patch_within_same_minor() {
        let v1 = BlenderVersion::new(3, 6, 1);
        let v2 = BlenderVersion::new(3, 6, 2);
        assert!(v1 < v2);
    }

    #[test]
    fn ordering_equality() {
        let v1 = BlenderVersion::new(3, 6, 2);
        let v2 = BlenderVersion::new(3, 6, 2);
        assert!(v1 == v2);
        assert!(!(v1 < v2));
        assert!(!(v1 > v2));
    }

    // ---- Default ----
    #[test]
    fn default_equals_minimum() {
        let d = BlenderVersion::default();
        assert_eq!(d, MINIMUM_BLENDER_VERSION);
    }

    // ---- Constants ----
    #[test]
    fn minimum_version_is_2_93_0() {
        assert_eq!(MINIMUM_BLENDER_VERSION.major, 2);
        assert_eq!(MINIMUM_BLENDER_VERSION.minor, 93);
        assert_eq!(MINIMUM_BLENDER_VERSION.patch, 0);
    }

    #[test]
    fn recommended_version_is_4_0_0() {
        assert_eq!(RECOMMENDED_BLENDER_VERSION.major, 4);
        assert_eq!(RECOMMENDED_BLENDER_VERSION.minor, 0);
        assert_eq!(RECOMMENDED_BLENDER_VERSION.patch, 0);
    }
}

// ============================================================================
// MODULE 2: BlenderCapabilities — version threshold tests
// ============================================================================
mod capabilities_tests {
    use super::*;

    #[test]
    fn old_version_no_capabilities() {
        let caps = BlenderVersion::new(2, 80, 0).capabilities();
        assert!(!caps.draco_compression);
        assert!(!caps.gltf_materials_variants);
        assert!(!caps.geometry_nodes_export);
        assert!(!caps.usd_export);
        assert!(!caps.webp_textures);
        assert!(!caps.ktx2_textures);
    }

    #[test]
    fn version_2_93_draco_only() {
        let caps = BlenderVersion::new(2, 93, 0).capabilities();
        assert!(caps.draco_compression);
        assert!(!caps.gltf_materials_variants);
        assert!(!caps.geometry_nodes_export);
        assert!(!caps.usd_export);
        assert!(!caps.webp_textures);
        assert!(!caps.ktx2_textures);
    }

    #[test]
    fn version_3_0_adds_usd() {
        let caps = BlenderVersion::new(3, 0, 0).capabilities();
        assert!(caps.draco_compression);
        assert!(caps.usd_export);
        assert!(!caps.gltf_materials_variants);
        assert!(!caps.geometry_nodes_export);
        assert!(!caps.webp_textures);
        assert!(!caps.ktx2_textures);
    }

    #[test]
    fn version_3_3_adds_material_variants() {
        let caps = BlenderVersion::new(3, 3, 0).capabilities();
        assert!(caps.draco_compression);
        assert!(caps.usd_export);
        assert!(caps.gltf_materials_variants);
        assert!(!caps.geometry_nodes_export);
        assert!(!caps.webp_textures);
        assert!(!caps.ktx2_textures);
    }

    #[test]
    fn version_3_4_adds_webp() {
        let caps = BlenderVersion::new(3, 4, 0).capabilities();
        assert!(caps.webp_textures);
        assert!(!caps.geometry_nodes_export); // Needs 3.5
        assert!(!caps.ktx2_textures); // Needs 4.0
    }

    #[test]
    fn version_3_5_adds_geometry_nodes() {
        let caps = BlenderVersion::new(3, 5, 0).capabilities();
        assert!(caps.geometry_nodes_export);
        assert!(caps.webp_textures);
        assert!(!caps.ktx2_textures);
    }

    #[test]
    fn version_4_0_all_capabilities() {
        let caps = BlenderVersion::new(4, 0, 0).capabilities();
        assert!(caps.draco_compression);
        assert!(caps.gltf_materials_variants);
        assert!(caps.geometry_nodes_export);
        assert!(caps.usd_export);
        assert!(caps.webp_textures);
        assert!(caps.ktx2_textures);
    }

    // ---- describe() ----
    #[test]
    fn describe_returns_six_entries() {
        let caps = BlenderVersion::new(4, 0, 0).capabilities();
        let desc = caps.describe();
        assert_eq!(desc.len(), 6);
    }

    #[test]
    fn describe_all_true_for_4_0() {
        let caps = BlenderVersion::new(4, 0, 0).capabilities();
        for (name, enabled) in caps.describe() {
            assert!(enabled, "Expected {} to be true for 4.0", name);
        }
    }

    #[test]
    fn describe_all_false_for_old() {
        let caps = BlenderVersion::new(2, 80, 0).capabilities();
        for (name, enabled) in caps.describe() {
            assert!(!enabled, "Expected {} to be false for 2.80", name);
        }
    }

    #[test]
    fn describe_names_are_correct() {
        let caps = BlenderVersion::new(4, 0, 0).capabilities();
        let desc = caps.describe();
        let names: Vec<&str> = desc.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"Draco compression"));
        assert!(names.contains(&"Material variants"));
        assert!(names.contains(&"Geometry nodes export"));
        assert!(names.contains(&"USD export"));
        assert!(names.contains(&"WebP textures"));
        assert!(names.contains(&"KTX2 textures"));
    }

    // ---- threshold boundary tests (>= edge) ----
    #[test]
    fn draco_threshold_just_below() {
        let caps = BlenderVersion::new(2, 92, 99).capabilities();
        assert!(!caps.draco_compression);
    }

    #[test]
    fn usd_threshold_just_below() {
        let caps = BlenderVersion::new(2, 99, 99).capabilities();
        assert!(!caps.usd_export);
    }

    #[test]
    fn ktx2_threshold_just_below() {
        let caps = BlenderVersion::new(3, 99, 99).capabilities();
        assert!(!caps.ktx2_textures);
    }
}

// ============================================================================
// MODULE 3: ConversionStage — is_terminal, base_progress, description, Display
// ============================================================================
mod conversion_stage_tests {
    use super::*;

    // ---- is_terminal ----
    #[test]
    fn completed_is_terminal() {
        assert!(ConversionStage::Completed.is_terminal());
    }

    #[test]
    fn failed_is_terminal() {
        assert!(ConversionStage::Failed.is_terminal());
    }

    #[test]
    fn cancelled_is_terminal() {
        assert!(ConversionStage::Cancelled.is_terminal());
    }

    #[test]
    fn initializing_is_not_terminal() {
        assert!(!ConversionStage::Initializing.is_terminal());
    }

    #[test]
    fn loading_is_not_terminal() {
        assert!(!ConversionStage::LoadingBlendFile.is_terminal());
    }

    #[test]
    fn exporting_meshes_is_not_terminal() {
        assert!(!ConversionStage::ExportingMeshes.is_terminal());
    }

    #[test]
    fn finalizing_is_not_terminal() {
        assert!(!ConversionStage::Finalizing.is_terminal());
    }

    // ---- base_progress exact values ----
    #[test]
    fn base_progress_initializing() {
        assert_eq!(ConversionStage::Initializing.base_progress(), 0.0);
    }

    #[test]
    fn base_progress_loading() {
        assert!((ConversionStage::LoadingBlendFile.base_progress() - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_linked_libs() {
        assert!(
            (ConversionStage::ProcessingLinkedLibraries.base_progress() - 0.10).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn base_progress_modifiers() {
        assert!((ConversionStage::ApplyingModifiers.base_progress() - 0.20).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_meshes() {
        assert!((ConversionStage::ExportingMeshes.base_progress() - 0.30).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_materials() {
        assert!((ConversionStage::ExportingMaterials.base_progress() - 0.50).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_textures() {
        assert!((ConversionStage::ExportingTextures.base_progress() - 0.60).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_animations() {
        assert!((ConversionStage::ExportingAnimations.base_progress() - 0.70).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_armatures() {
        assert!((ConversionStage::ExportingArmatures.base_progress() - 0.80).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_writing() {
        assert!((ConversionStage::WritingOutput.base_progress() - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_compressing() {
        assert!((ConversionStage::Compressing.base_progress() - 0.90).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_caching() {
        assert!((ConversionStage::CachingResults.base_progress() - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_finalizing() {
        assert!((ConversionStage::Finalizing.base_progress() - 0.98).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_completed() {
        assert!((ConversionStage::Completed.base_progress() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn base_progress_failed_is_zero() {
        assert_eq!(ConversionStage::Failed.base_progress(), 0.0);
    }

    #[test]
    fn base_progress_cancelled_is_zero() {
        assert_eq!(ConversionStage::Cancelled.base_progress(), 0.0);
    }

    // ---- base_progress monotonically increases through pipeline ----
    #[test]
    fn base_progress_monotonic_increase() {
        let stages = [
            ConversionStage::Initializing,
            ConversionStage::LoadingBlendFile,
            ConversionStage::ProcessingLinkedLibraries,
            ConversionStage::ApplyingModifiers,
            ConversionStage::ExportingMeshes,
            ConversionStage::ExportingMaterials,
            ConversionStage::ExportingTextures,
            ConversionStage::ExportingAnimations,
            ConversionStage::ExportingArmatures,
            ConversionStage::WritingOutput,
            ConversionStage::Compressing,
            ConversionStage::CachingResults,
            ConversionStage::Finalizing,
            ConversionStage::Completed,
        ];
        for w in stages.windows(2) {
            assert!(
                w[0].base_progress() <= w[1].base_progress(),
                "{:?} ({}) should be <= {:?} ({})",
                w[0],
                w[0].base_progress(),
                w[1],
                w[1].base_progress()
            );
        }
    }

    // ---- description exact strings ----
    #[test]
    fn description_initializing() {
        assert_eq!(
            ConversionStage::Initializing.description(),
            "Initializing conversion"
        );
    }

    #[test]
    fn description_loading() {
        assert_eq!(
            ConversionStage::LoadingBlendFile.description(),
            "Loading .blend file"
        );
    }

    #[test]
    fn description_completed() {
        assert_eq!(ConversionStage::Completed.description(), "Completed");
    }

    #[test]
    fn description_failed() {
        assert_eq!(ConversionStage::Failed.description(), "Failed");
    }

    #[test]
    fn description_cancelled() {
        assert_eq!(ConversionStage::Cancelled.description(), "Cancelled");
    }

    // ---- Display delegates to description ----
    #[test]
    fn display_matches_description() {
        let stages = [
            ConversionStage::Initializing,
            ConversionStage::LoadingBlendFile,
            ConversionStage::ExportingMeshes,
            ConversionStage::Completed,
            ConversionStage::Failed,
            ConversionStage::Cancelled,
        ];
        for stage in &stages {
            assert_eq!(format!("{}", stage), stage.description());
        }
    }
}

// ============================================================================
// MODULE 4: ProgressTracker — stages, items, cancellation, completion
// ============================================================================
mod progress_tracker_tests {
    use super::*;

    #[test]
    fn new_tracker_starts_initializing() {
        let tracker = ProgressTracker::new();
        let p = tracker.current();
        assert_eq!(p.stage, ConversionStage::Initializing);
        assert_eq!(p.overall_progress, 0.0);
    }

    #[test]
    fn set_stage_updates_overall_progress() {
        let tracker = ProgressTracker::new();
        tracker.set_stage(ConversionStage::ExportingMeshes);
        let p = tracker.current();
        assert_eq!(p.stage, ConversionStage::ExportingMeshes);
        assert!((p.overall_progress - 0.30).abs() < 0.01);
    }

    #[test]
    fn set_stage_progress_interpolates() {
        let tracker = ProgressTracker::new();
        tracker.set_stage(ConversionStage::ExportingMeshes);
        tracker.set_stage_progress(0.5);
        let p = tracker.current();
        // base 0.30, next 0.50, weight 0.20 → 0.30 + 0.20*0.5 = 0.40
        assert!((p.overall_progress - 0.40).abs() < 0.01);
    }

    #[test]
    fn stage_progress_clamped_to_one() {
        let tracker = ProgressTracker::new();
        tracker.set_stage(ConversionStage::Initializing);
        tracker.set_stage_progress(2.0);
        let p = tracker.current();
        assert!(p.stage_progress <= 1.0);
    }

    #[test]
    fn stage_progress_clamped_to_zero() {
        let tracker = ProgressTracker::new();
        tracker.set_stage(ConversionStage::Initializing);
        tracker.set_stage_progress(-1.0);
        let p = tracker.current();
        assert!(p.stage_progress >= 0.0);
    }

    #[test]
    fn set_message_updates() {
        let tracker = ProgressTracker::new();
        tracker.set_message("Processing textures...");
        let p = tracker.current();
        assert_eq!(p.message, "Processing textures...");
    }

    #[test]
    fn set_total_items() {
        let tracker = ProgressTracker::new();
        tracker.set_total_items(100);
        let p = tracker.current();
        assert_eq!(p.items_total, Some(100));
    }

    #[test]
    fn increment_items_counts() {
        let tracker = ProgressTracker::new();
        tracker.set_stage(ConversionStage::ExportingMeshes);
        tracker.set_total_items(4);
        tracker.increment_items();
        tracker.increment_items();
        let p = tracker.current();
        assert_eq!(p.items_processed, 2);
        assert!((p.stage_progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn set_bytes_written() {
        let tracker = ProgressTracker::new();
        tracker.set_bytes_written(1024);
        let p = tracker.current();
        assert_eq!(p.bytes_written, 1024);
    }

    #[test]
    fn complete_sets_1_0() {
        let tracker = ProgressTracker::new();
        tracker.complete();
        let p = tracker.current();
        assert_eq!(p.stage, ConversionStage::Completed);
        assert_eq!(p.overall_progress, 1.0);
        assert_eq!(p.stage_progress, 1.0);
    }

    #[test]
    fn fail_sets_failed_stage() {
        let tracker = ProgressTracker::new();
        tracker.fail("something broke");
        let p = tracker.current();
        assert_eq!(p.stage, ConversionStage::Failed);
        assert_eq!(p.message, "something broke");
    }

    #[test]
    fn cancellation_propagates() {
        let tracker = ProgressTracker::new();
        let token = tracker.cancellation_token();
        assert!(!tracker.is_cancelled());
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(tracker.is_cancelled());
        assert!(token.is_cancelled());
    }

    #[test]
    fn mark_cancelled_sets_state() {
        let tracker = ProgressTracker::new();
        tracker.mark_cancelled();
        let p = tracker.current();
        assert_eq!(p.stage, ConversionStage::Cancelled);
        assert!(p.cancelled);
        assert!(tracker.is_cancelled());
    }

    #[test]
    fn subscribe_gets_updates() {
        let tracker = ProgressTracker::new();
        let receiver = tracker.subscribe();
        tracker.set_stage(ConversionStage::Compressing);
        let p = receiver.current();
        assert_eq!(p.stage, ConversionStage::Compressing);
    }

    #[test]
    fn receiver_is_complete_true_for_terminal() {
        let tracker = ProgressTracker::new();
        let receiver = tracker.subscribe();
        tracker.complete();
        assert!(receiver.is_complete());
    }

    #[test]
    fn receiver_is_complete_false_for_active() {
        let tracker = ProgressTracker::new();
        let receiver = tracker.subscribe();
        tracker.set_stage(ConversionStage::ExportingMeshes);
        assert!(!receiver.is_complete());
    }

    #[test]
    fn default_progress_initial_values() {
        let p = ConversionProgress::default();
        assert_eq!(p.stage, ConversionStage::Initializing);
        assert_eq!(p.stage_progress, 0.0);
        assert_eq!(p.overall_progress, 0.0);
        assert_eq!(p.items_processed, 0);
        assert_eq!(p.items_total, None);
        assert_eq!(p.bytes_written, 0);
        assert!(!p.cancelled);
        assert!(p.estimated_remaining.is_none());
    }
}

// ============================================================================
// MODULE 5: BlendError — classification methods
// ============================================================================
mod error_classification_tests {
    use super::*;

    // ---- is_blender_missing ----
    #[test]
    fn blender_not_found_is_missing() {
        let e = BlendError::BlenderNotFound {
            searched_paths: vec![],
        };
        assert!(e.is_blender_missing());
    }

    #[test]
    fn blender_executable_not_found_is_missing() {
        let e = BlendError::BlenderExecutableNotFound {
            path: PathBuf::from("/bin/blender"),
            reason: "not found".into(),
        };
        assert!(e.is_blender_missing());
    }

    #[test]
    fn cancelled_is_not_blender_missing() {
        assert!(!BlendError::Cancelled.is_blender_missing());
    }

    #[test]
    fn timeout_is_not_blender_missing() {
        let e = BlendError::Timeout {
            operation: "test".into(),
            duration: Duration::from_secs(1),
            path: PathBuf::from("x"),
            timeout_secs: 1,
        };
        assert!(!e.is_blender_missing());
    }

    // ---- is_retryable ----
    #[test]
    fn timeout_is_retryable() {
        let e = BlendError::Timeout {
            operation: "conv".into(),
            duration: Duration::from_secs(120),
            path: PathBuf::from("file.blend"),
            timeout_secs: 120,
        };
        assert!(e.is_retryable());
    }

    #[test]
    fn process_spawn_is_retryable() {
        let e = BlendError::ProcessSpawnError {
            source: std::io::Error::new(std::io::ErrorKind::Other, "test"),
        };
        assert!(e.is_retryable());
    }

    #[test]
    fn io_error_is_retryable() {
        let e = BlendError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        assert!(e.is_retryable());
    }

    #[test]
    fn cancelled_is_not_retryable() {
        assert!(!BlendError::Cancelled.is_retryable());
    }

    #[test]
    fn version_too_old_is_not_retryable() {
        let e = BlendError::BlenderVersionTooOld {
            found: "2.80".into(),
            required: "2.93".into(),
        };
        assert!(!e.is_retryable());
    }

    // ---- is_cancelled ----
    #[test]
    fn cancelled_is_cancelled() {
        assert!(BlendError::Cancelled.is_cancelled());
    }

    #[test]
    fn not_found_is_not_cancelled() {
        let e = BlendError::BlenderNotFound {
            searched_paths: vec![],
        };
        assert!(!e.is_cancelled());
    }

    // ---- is_cache_error ----
    #[test]
    fn cache_corrupted_is_cache_error() {
        let e = BlendError::CacheCorrupted {
            path: PathBuf::from("/cache"),
            message: "bad".into(),
        };
        assert!(e.is_cache_error());
    }

    #[test]
    fn cache_load_error_is_cache_error() {
        let e = BlendError::CacheLoadError {
            path: PathBuf::from("/cache"),
            reason: "fail".into(),
        };
        assert!(e.is_cache_error());
    }

    #[test]
    fn cache_save_error_is_cache_error() {
        let e = BlendError::CacheSaveError {
            path: PathBuf::from("/cache"),
            source: std::io::Error::new(std::io::ErrorKind::Other, "test"),
        };
        assert!(e.is_cache_error());
    }

    #[test]
    fn cancelled_is_not_cache_error() {
        assert!(!BlendError::Cancelled.is_cache_error());
    }

    // ---- is_configuration_error ----
    #[test]
    fn configuration_error_is_config() {
        let e = BlendError::ConfigurationError {
            message: "bad".into(),
        };
        assert!(e.is_configuration_error());
    }

    #[test]
    fn invalid_option_is_config() {
        let e = BlendError::InvalidOption {
            reason: "bad".into(),
        };
        assert!(e.is_configuration_error());
    }

    #[test]
    fn configured_path_not_found_is_config() {
        let e = BlendError::ConfiguredPathNotFound {
            path: PathBuf::from("/x"),
        };
        assert!(e.is_configuration_error());
    }

    #[test]
    fn configured_path_not_executable_is_config() {
        let e = BlendError::ConfiguredPathNotExecutable {
            path: PathBuf::from("/x"),
        };
        assert!(e.is_configuration_error());
    }

    #[test]
    fn cancelled_is_not_config_error() {
        assert!(!BlendError::Cancelled.is_configuration_error());
    }

    // ---- suggestion ----
    #[test]
    fn blender_not_found_has_suggestion() {
        let e = BlendError::BlenderNotFound {
            searched_paths: vec![],
        };
        assert!(e.suggestion().is_some());
        assert!(e.suggestion().unwrap().contains("blender.org"));
    }

    #[test]
    fn version_too_old_has_suggestion() {
        let e = BlendError::BlenderVersionTooOld {
            found: "2.80".into(),
            required: "2.93".into(),
        };
        assert!(e.suggestion().is_some());
        assert!(e.suggestion().unwrap().contains("2.93"));
    }

    #[test]
    fn timeout_has_suggestion() {
        let e = BlendError::Timeout {
            operation: "x".into(),
            duration: Duration::from_secs(1),
            path: PathBuf::from("x"),
            timeout_secs: 1,
        };
        assert!(e.suggestion().is_some());
    }

    #[test]
    fn cache_corrupted_has_suggestion() {
        let e = BlendError::CacheCorrupted {
            path: PathBuf::from("/cache"),
            message: "bad".into(),
        };
        assert!(e.suggestion().is_some());
    }

    #[test]
    fn linked_library_not_found_has_suggestion() {
        let e = BlendError::LinkedLibraryNotFound {
            library_path: PathBuf::from("lib.blend"),
            source_blend: PathBuf::from("main.blend"),
        };
        assert!(e.suggestion().is_some());
    }

    #[test]
    fn circular_library_has_suggestion() {
        let e = BlendError::CircularLibraryReference {
            cycle: vec![PathBuf::from("a.blend"), PathBuf::from("b.blend")],
        };
        assert!(e.suggestion().is_some());
    }

    #[test]
    fn invalid_blend_file_has_suggestion() {
        let e = BlendError::InvalidBlendFile {
            path: PathBuf::from("x"),
            message: "bad".into(),
        };
        assert!(e.suggestion().is_some());
    }

    #[test]
    fn execution_failed_has_suggestion() {
        let e = BlendError::BlenderExecutionFailed {
            path: PathBuf::from("/blender"),
            reason: "error".into(),
        };
        assert!(e.suggestion().is_some());
    }

    #[test]
    fn cancelled_has_no_suggestion() {
        assert!(BlendError::Cancelled.suggestion().is_none());
    }

    #[test]
    fn internal_has_no_suggestion() {
        let e = BlendError::Internal {
            message: "bad".into(),
        };
        assert!(e.suggestion().is_none());
    }

    // ---- Display messages contain key info ----
    #[test]
    fn display_blender_not_found() {
        let e = BlendError::BlenderNotFound {
            searched_paths: vec![PathBuf::from("/usr/bin")],
        };
        let msg = e.to_string();
        assert!(msg.contains("Blender not found"));
        assert!(msg.contains("blender.org"));
    }

    #[test]
    fn display_version_too_old() {
        let e = BlendError::BlenderVersionTooOld {
            found: "2.80.0".into(),
            required: "2.93.0".into(),
        };
        let msg = e.to_string();
        assert!(msg.contains("2.80.0"));
        assert!(msg.contains("2.93.0"));
    }

    #[test]
    fn display_timeout() {
        let e = BlendError::Timeout {
            operation: "conversion".into(),
            duration: Duration::from_secs(120),
            path: PathBuf::from("model.blend"),
            timeout_secs: 120,
        };
        let msg = e.to_string();
        assert!(msg.contains("conversion"));
        assert!(msg.contains("model.blend"));
    }

    #[test]
    fn display_cancelled() {
        assert_eq!(
            BlendError::Cancelled.to_string(),
            "Conversion cancelled by user"
        );
    }
}

// ============================================================================
// MODULE 6: CacheMissReason — Display
// ============================================================================
mod cache_miss_reason_tests {
    use super::*;

    #[test]
    fn not_cached_display() {
        assert_eq!(CacheMissReason::NotCached.to_string(), "not cached");
    }

    #[test]
    fn source_modified_display() {
        assert_eq!(
            CacheMissReason::SourceModified.to_string(),
            "source file modified"
        );
    }

    #[test]
    fn options_changed_display() {
        assert_eq!(
            CacheMissReason::OptionsChanged.to_string(),
            "conversion options changed"
        );
    }

    #[test]
    fn blender_version_changed_display() {
        assert_eq!(
            CacheMissReason::BlenderVersionChanged.to_string(),
            "Blender version changed"
        );
    }

    #[test]
    fn output_missing_display() {
        assert_eq!(
            CacheMissReason::OutputMissing.to_string(),
            "cached output missing"
        );
    }

    #[test]
    fn expired_display() {
        assert_eq!(CacheMissReason::Expired.to_string(), "cache entry expired");
    }

    #[test]
    fn validation_failed_display() {
        let r = CacheMissReason::ValidationFailed("hash mismatch".into());
        assert_eq!(r.to_string(), "validation failed: hash mismatch");
    }
}

// ============================================================================
// MODULE 7: CacheManifest, CacheEntry, CacheStats
// ============================================================================
mod cache_data_structure_tests {
    use super::*;

    #[test]
    fn manifest_new_version_is_current() {
        let m = CacheManifest::new();
        assert_eq!(m.version, CacheManifest::CURRENT_VERSION);
    }

    #[test]
    fn manifest_new_empty() {
        let m = CacheManifest::new();
        assert!(m.entries.is_empty());
        assert_eq!(m.total_size, 0);
    }

    #[test]
    fn manifest_recalculate_size() {
        let mut m = CacheManifest::new();
        let entry_a = CacheEntry::new(
            "a".into(),
            "o".into(),
            BlenderVersion::new(4, 0, 0),
            PathBuf::from("a.glb"),
            PathBuf::from("a.blend"),
            100,
            10,
        );
        let entry_b = CacheEntry::new(
            "b".into(),
            "o".into(),
            BlenderVersion::new(4, 0, 0),
            PathBuf::from("b.glb"),
            PathBuf::from("b.blend"),
            200,
            20,
        );
        m.entries.insert("a".into(), entry_a);
        m.entries.insert("b".into(), entry_b);
        m.recalculate_size();
        assert_eq!(m.total_size, 300);
    }

    #[test]
    fn manifest_recalculate_empty() {
        let mut m = CacheManifest::new();
        m.total_size = 999;
        m.recalculate_size();
        assert_eq!(m.total_size, 0);
    }

    #[test]
    fn cache_entry_new_timestamps_recent() {
        let entry = CacheEntry::new(
            "hash".into(),
            "opts".into(),
            BlenderVersion::new(4, 0, 0),
            PathBuf::from("out.glb"),
            PathBuf::from("src.blend"),
            500,
            100,
        );
        assert!(entry.age() < Duration::from_secs(2));
        assert!(entry.time_since_access() < Duration::from_secs(2));
    }

    #[test]
    fn cache_entry_touch_updates_accessed() {
        let mut entry = CacheEntry::new(
            "hash".into(),
            "opts".into(),
            BlenderVersion::new(4, 0, 0),
            PathBuf::from("out.glb"),
            PathBuf::from("src.blend"),
            500,
            100,
        );
        let before = entry.last_accessed;
        entry.touch();
        // Should be >= before (might be same second)
        assert!(entry.last_accessed >= before);
    }

    #[test]
    fn cache_entry_fields_stored() {
        let entry = CacheEntry::new(
            "hash123".into(),
            "opts456".into(),
            BlenderVersion::new(3, 6, 0),
            PathBuf::from("out.glb"),
            PathBuf::from("src.blend"),
            1024,
            500,
        );
        assert_eq!(entry.source_hash, "hash123");
        assert_eq!(entry.options_hash, "opts456");
        assert_eq!(entry.blender_version, BlenderVersion::new(3, 6, 0));
        assert_eq!(entry.output_size, 1024);
        assert_eq!(entry.conversion_duration_ms, 500);
        assert!(entry.texture_files.is_empty());
        assert!(entry.linked_libraries.is_empty());
    }

    // ---- CacheStats ----
    #[test]
    fn stats_utilization_half() {
        let stats = CacheStats {
            entry_count: 5,
            total_size: 500,
            max_size: Some(1000),
            enabled: true,
        };
        assert_eq!(stats.utilization(), Some(0.5));
    }

    #[test]
    fn stats_utilization_full() {
        let stats = CacheStats {
            entry_count: 5,
            total_size: 1000,
            max_size: Some(1000),
            enabled: true,
        };
        assert_eq!(stats.utilization(), Some(1.0));
    }

    #[test]
    fn stats_utilization_none_when_unlimited() {
        let stats = CacheStats {
            entry_count: 10,
            total_size: 5000,
            max_size: None,
            enabled: true,
        };
        assert_eq!(stats.utilization(), None);
    }

    #[test]
    fn stats_utilization_zero_max() {
        let stats = CacheStats {
            entry_count: 0,
            total_size: 0,
            max_size: Some(0),
            enabled: true,
        };
        assert_eq!(stats.utilization(), Some(0.0));
    }

    #[test]
    fn stats_utilization_over_full() {
        let stats = CacheStats {
            entry_count: 10,
            total_size: 2000,
            max_size: Some(1000),
            enabled: true,
        };
        let util = stats.utilization().unwrap();
        assert!((util - 2.0).abs() < f64::EPSILON);
    }
}

// ============================================================================
// MODULE 8: ConversionCache — filesystem operations
// ============================================================================
mod cache_filesystem_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn cache_creation_creates_directory() {
        let temp = TempDir::new().unwrap();
        let cache = ConversionCache::new(temp.path()).unwrap();
        assert!(cache.cache_dir().exists());
    }

    #[test]
    fn cache_stats_empty_on_new() {
        let temp = TempDir::new().unwrap();
        let cache = ConversionCache::new(temp.path()).unwrap();
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_size, 0);
        assert!(stats.enabled);
    }

    #[test]
    fn cache_hash_file_real_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.blend");
        std::fs::write(&file_path, b"BLENDER_FILE_CONTENT").unwrap();
        let hash = ConversionCache::hash_file(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn cache_hash_file_deterministic() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.blend");
        std::fs::write(&file_path, b"same content").unwrap();
        let hash1 = ConversionCache::hash_file(&file_path).unwrap();
        let hash2 = ConversionCache::hash_file(&file_path).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn cache_hash_file_different_content() {
        let temp = TempDir::new().unwrap();
        let f1 = temp.path().join("a.blend");
        let f2 = temp.path().join("b.blend");
        std::fs::write(&f1, b"content_a").unwrap();
        std::fs::write(&f2, b"content_b").unwrap();
        let h1 = ConversionCache::hash_file(&f1).unwrap();
        let h2 = ConversionCache::hash_file(&f2).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn cache_hash_file_nonexistent() {
        let result = ConversionCache::hash_file(&PathBuf::from("/nonexistent/file.blend"));
        assert!(result.is_err());
    }

    #[test]
    fn cache_invalidate_nonexistent() {
        let temp = TempDir::new().unwrap();
        let mut cache = ConversionCache::new(temp.path()).unwrap();
        let removed = cache
            .invalidate(&PathBuf::from("nonexistent.blend"))
            .unwrap();
        assert!(!removed);
    }

    #[test]
    fn cache_clear_empty() {
        let temp = TempDir::new().unwrap();
        let mut cache = ConversionCache::new(temp.path()).unwrap();
        assert!(cache.clear().is_ok());
        assert_eq!(cache.stats().entry_count, 0);
    }

    #[test]
    fn cache_for_project() {
        let temp = TempDir::new().unwrap();
        let cache = ConversionCache::for_project(temp.path()).unwrap();
        assert!(cache.cache_dir().to_string_lossy().contains(".astraweave"));
    }

    #[test]
    fn cache_with_max_size_builder() {
        let temp = TempDir::new().unwrap();
        let cache = ConversionCache::new(temp.path())
            .unwrap()
            .with_max_size(Some(1024));
        // The builder should return the cache (it's a builder pattern)
        let _ = cache;
    }

    #[test]
    fn cache_with_max_age_builder() {
        let temp = TempDir::new().unwrap();
        let cache = ConversionCache::new(temp.path())
            .unwrap()
            .with_max_age(Some(Duration::from_secs(3600)));
        let _ = cache;
    }

    #[test]
    fn cache_set_enabled_false() {
        let temp = TempDir::new().unwrap();
        let mut cache = ConversionCache::new(temp.path()).unwrap();
        cache.set_enabled(false);
        assert!(!cache.stats().enabled);
    }

    #[test]
    fn cache_disabled_lookup_returns_miss() {
        let temp = TempDir::new().unwrap();
        let mut cache = ConversionCache::new(temp.path()).unwrap();
        cache.set_enabled(false);

        let file_path = temp.path().join("test.blend");
        std::fs::write(&file_path, b"content").unwrap();

        let result = cache.lookup(
            &file_path,
            &ConversionOptions::default(),
            &BlenderVersion::new(4, 0, 0),
        );
        match result {
            Ok(CacheLookup::Miss { reason }) => {
                assert_eq!(reason, CacheMissReason::NotCached);
            }
            _ => panic!("Expected cache miss when disabled"),
        }
    }
}

// ============================================================================
// MODULE 9: ConversionOptions — presets, builder, defaults
// ============================================================================
mod options_tests {
    use super::*;

    // ---- Preset factories ----
    #[test]
    fn game_runtime_draco_disabled() {
        let opts = ConversionOptions::game_runtime();
        // Draco disabled: the Rust gltf crate cannot decode Draco.
        assert!(!opts.gltf.draco_compression);
    }

    #[test]
    fn game_runtime_triangulate() {
        let opts = ConversionOptions::game_runtime();
        assert!(opts.mesh.triangulate);
    }

    #[test]
    fn game_runtime_max_tex_2048() {
        let opts = ConversionOptions::game_runtime();
        assert_eq!(opts.textures.max_resolution, Some(2048));
    }

    #[test]
    fn editor_preview_draco_disabled() {
        let opts = ConversionOptions::editor_preview();
        assert!(!opts.gltf.draco_compression);
    }

    #[test]
    fn editor_preview_max_tex_512() {
        let opts = ConversionOptions::editor_preview();
        assert_eq!(opts.textures.max_resolution, Some(512));
    }

    #[test]
    fn editor_preview_shorter_timeout() {
        let opts = ConversionOptions::editor_preview();
        assert_eq!(opts.process.timeout, Duration::from_secs(30));
    }

    #[test]
    fn editor_preview_jpeg_format() {
        let opts = ConversionOptions::editor_preview();
        assert_eq!(opts.textures.format, TextureFormat::Jpeg);
    }

    #[test]
    fn archival_no_draco() {
        let opts = ConversionOptions::archival_quality();
        assert!(!opts.gltf.draco_compression);
    }

    #[test]
    fn archival_gltf_separate() {
        let opts = ConversionOptions::archival_quality();
        assert_eq!(opts.format, OutputFormat::GltfSeparate);
    }

    #[test]
    fn archival_no_max_resolution() {
        let opts = ConversionOptions::archival_quality();
        assert_eq!(opts.textures.max_resolution, None);
    }

    #[test]
    fn archival_animations_enabled() {
        let opts = ConversionOptions::archival_quality();
        assert!(opts.animation.export_animations);
        assert!(opts.animation.export_shape_keys);
    }

    #[test]
    fn archival_no_optimize_animation() {
        let opts = ConversionOptions::archival_quality();
        assert!(!opts.animation.optimize_animation_size);
    }

    #[test]
    fn archival_extras_lights_cameras() {
        let opts = ConversionOptions::archival_quality();
        assert!(opts.gltf.export_extras);
        assert!(opts.gltf.export_lights);
        assert!(opts.gltf.export_cameras);
    }

    // ---- Builder ----
    #[test]
    fn builder_format() {
        let opts = ConversionOptions::builder()
            .format(OutputFormat::GltfSeparate)
            .build();
        assert_eq!(opts.format, OutputFormat::GltfSeparate);
    }

    #[test]
    fn builder_draco() {
        let opts = ConversionOptions::builder()
            .draco_compression(false)
            .build();
        assert!(!opts.gltf.draco_compression);
    }

    #[test]
    fn builder_texture_format() {
        let opts = ConversionOptions::builder()
            .texture_format(TextureFormat::WebP)
            .build();
        assert_eq!(opts.textures.format, TextureFormat::WebP);
    }

    #[test]
    fn builder_max_texture_resolution() {
        let opts = ConversionOptions::builder()
            .max_texture_resolution(Some(1024))
            .build();
        assert_eq!(opts.textures.max_resolution, Some(1024));
    }

    #[test]
    fn builder_timeout() {
        let opts = ConversionOptions::builder()
            .timeout(Duration::from_secs(60))
            .build();
        assert_eq!(opts.process.timeout, Duration::from_secs(60));
    }

    #[test]
    fn builder_animations() {
        let opts = ConversionOptions::builder()
            .export_animations(false)
            .build();
        assert!(!opts.animation.export_animations);
    }

    #[test]
    fn builder_modifiers() {
        let opts = ConversionOptions::builder().apply_modifiers(false).build();
        assert!(!opts.mesh.apply_modifiers);
    }

    #[test]
    fn builder_linked_library_depth() {
        let opts = ConversionOptions::builder().linked_library_depth(5).build();
        assert_eq!(opts.linked_libraries.max_recursion_depth, 5);
    }

    #[test]
    fn builder_cache() {
        let opts = ConversionOptions::builder().cache_enabled(false).build();
        assert!(!opts.cache.enabled);
    }

    #[test]
    fn builder_chain_multiple_options() {
        let opts = ConversionOptions::builder()
            .format(OutputFormat::GltfEmbedded)
            .draco_compression(true)
            .texture_format(TextureFormat::Jpeg)
            .max_texture_resolution(Some(256))
            .timeout(Duration::from_secs(10))
            .export_animations(false)
            .apply_modifiers(false)
            .linked_library_depth(3)
            .cache_enabled(true)
            .build();
        assert_eq!(opts.format, OutputFormat::GltfEmbedded);
        assert!(opts.gltf.draco_compression);
        assert_eq!(opts.textures.format, TextureFormat::Jpeg);
        assert_eq!(opts.textures.max_resolution, Some(256));
        assert_eq!(opts.process.timeout, Duration::from_secs(10));
        assert!(!opts.animation.export_animations);
        assert!(!opts.mesh.apply_modifiers);
        assert_eq!(opts.linked_libraries.max_recursion_depth, 3);
        assert!(opts.cache.enabled);
    }

    // ---- OutputFormat ----
    #[test]
    fn glb_extension() {
        assert_eq!(OutputFormat::GlbBinary.extension(), "glb");
    }

    #[test]
    fn gltf_embedded_extension() {
        assert_eq!(OutputFormat::GltfEmbedded.extension(), "gltf");
    }

    #[test]
    fn gltf_separate_extension() {
        assert_eq!(OutputFormat::GltfSeparate.extension(), "gltf");
    }

    #[test]
    fn glb_blender_format() {
        assert_eq!(OutputFormat::GlbBinary.blender_format(), "GLB");
    }

    #[test]
    fn gltf_embedded_blender_format() {
        assert_eq!(OutputFormat::GltfEmbedded.blender_format(), "GLTF_EMBEDDED");
    }

    #[test]
    fn gltf_separate_blender_format() {
        assert_eq!(OutputFormat::GltfSeparate.blender_format(), "GLTF_SEPARATE");
    }

    // ---- TextureFormat extension ----
    #[test]
    fn png_extension() {
        assert_eq!(TextureFormat::Png.extension(), "png");
    }

    #[test]
    fn jpeg_extension() {
        assert_eq!(TextureFormat::Jpeg.extension(), "jpg");
    }

    #[test]
    fn webp_extension() {
        assert_eq!(TextureFormat::WebP.extension(), "webp");
    }

    #[test]
    fn original_extension_empty() {
        assert_eq!(TextureFormat::Original.extension(), "");
    }

    // ---- Default values ----
    #[test]
    fn default_output_format_is_glb() {
        assert_eq!(OutputFormat::default(), OutputFormat::GlbBinary);
    }

    #[test]
    fn default_texture_format_is_png() {
        assert_eq!(TextureFormat::default(), TextureFormat::Png);
    }

    #[test]
    fn default_missing_lib_action_is_warn() {
        assert_eq!(MissingLibraryAction::default(), MissingLibraryAction::Warn);
    }

    #[test]
    fn default_gltf_options_draco() {
        let opts = GltfExportOptions::default();
        // Draco disabled by default: the Rust gltf crate cannot decode Draco.
        assert!(!opts.draco_compression);
        assert_eq!(opts.draco_compression_level, 6);
    }

    #[test]
    fn default_texture_options_max_4096() {
        let opts = TextureOptions::default();
        assert_eq!(opts.max_resolution, Some(4096));
        assert_eq!(opts.jpeg_quality, 90);
    }

    #[test]
    fn default_process_timeout_300s() {
        let opts = ProcessOptions::default();
        assert_eq!(opts.timeout, Duration::from_secs(300));
    }

    #[test]
    fn default_cache_enabled() {
        let opts = CacheOptions::default();
        assert!(opts.enabled);
        assert!(opts.validate_on_access);
    }

    #[test]
    fn default_linked_libs_recursive() {
        let opts = LinkedLibraryOptions::default();
        assert!(opts.process_recursively);
        assert_eq!(opts.max_recursion_depth, 10);
        assert!(opts.detect_circular_references);
    }
}

// ============================================================================
// MODULE 10: DiscoveryMethod — Display
// ============================================================================
mod discovery_method_tests {
    use super::*;

    #[test]
    fn user_configured_display() {
        assert_eq!(
            format!("{}", DiscoveryMethod::UserConfigured),
            "user configured"
        );
    }

    #[test]
    fn system_path_display() {
        assert_eq!(format!("{}", DiscoveryMethod::SystemPath), "system PATH");
    }

    #[test]
    fn common_directory_display() {
        assert_eq!(
            format!("{}", DiscoveryMethod::CommonDirectory),
            "common installation directory"
        );
    }

    #[test]
    fn windows_registry_display() {
        assert_eq!(
            format!("{}", DiscoveryMethod::WindowsRegistry),
            "Windows Registry"
        );
    }

    #[test]
    fn macos_spotlight_display() {
        assert_eq!(
            format!("{}", DiscoveryMethod::MacOsSpotlight),
            "macOS Spotlight"
        );
    }

    #[test]
    fn custom_search_path_display() {
        assert_eq!(
            format!("{}", DiscoveryMethod::CustomSearchPath),
            "custom search path"
        );
    }
}

// ============================================================================
// MODULE 11: BlenderInstallation — is_valid, capabilities
// ============================================================================
mod installation_tests {
    use super::*;

    #[test]
    fn installation_is_valid_when_meets_minimum() {
        let inst = BlenderInstallation {
            executable_path: PathBuf::from("/usr/bin/blender"),
            version: BlenderVersion::new(4, 0, 0),
            discovery_method: DiscoveryMethod::SystemPath,
            install_dir: PathBuf::from("/usr/bin"),
        };
        assert!(inst.is_valid());
    }

    #[test]
    fn installation_not_valid_when_old() {
        let inst = BlenderInstallation {
            executable_path: PathBuf::from("/usr/bin/blender"),
            version: BlenderVersion::new(2, 80, 0),
            discovery_method: DiscoveryMethod::SystemPath,
            install_dir: PathBuf::from("/usr/bin"),
        };
        assert!(!inst.is_valid());
    }

    #[test]
    fn installation_capabilities_delegates() {
        let inst = BlenderInstallation {
            executable_path: PathBuf::from("/usr/bin/blender"),
            version: BlenderVersion::new(4, 0, 0),
            discovery_method: DiscoveryMethod::SystemPath,
            install_dir: PathBuf::from("/usr/bin"),
        };
        let caps = inst.capabilities();
        assert!(caps.draco_compression);
        assert!(caps.ktx2_textures);
    }
}

// ============================================================================
// MODULE 12: BlenderDiscovery — config, user path management
// ============================================================================
mod discovery_config_tests {
    use super::*;

    #[test]
    fn new_default_config() {
        let d = BlenderDiscovery::new();
        let cfg = d.config();
        assert!(cfg.user_path.is_none());
        assert!(cfg.search_path);
        assert!(cfg.search_common_dirs);
        assert!(cfg.use_platform_discovery);
        assert!(cfg.custom_search_paths.is_empty());
    }

    #[test]
    fn with_config_custom() {
        let config = BlenderDiscoveryConfig {
            user_path: Some(PathBuf::from("/custom/blender")),
            search_path: false,
            search_common_dirs: false,
            use_platform_discovery: false,
            custom_search_paths: vec![PathBuf::from("/opt")],
        };
        let d = BlenderDiscovery::with_config(config);
        let cfg = d.config();
        assert_eq!(cfg.user_path, Some(PathBuf::from("/custom/blender")));
        assert!(!cfg.search_path);
        assert!(!cfg.search_common_dirs);
        assert!(!cfg.use_platform_discovery);
        assert_eq!(cfg.custom_search_paths.len(), 1);
    }

    #[test]
    fn set_user_path() {
        let mut d = BlenderDiscovery::new();
        d.set_user_path("/my/blender");
        assert_eq!(d.config().user_path, Some(PathBuf::from("/my/blender")));
    }

    #[test]
    fn clear_user_path() {
        let mut d = BlenderDiscovery::new();
        d.set_user_path("/my/blender");
        d.clear_user_path();
        assert!(d.config().user_path.is_none());
    }
}

// ============================================================================
// MODULE 13: ConversionJobBuilder — error paths
// ============================================================================
mod builder_tests {
    use super::*;

    #[test]
    fn builder_no_source_errors() {
        let result = ConversionJobBuilder::new()
            .installation(mock_installation())
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_no_installation_errors() {
        let result = ConversionJobBuilder::new()
            .source("/test/model.blend")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_all_required_ok() {
        let result = ConversionJobBuilder::new()
            .source("/test/model.blend")
            .output("/output/model.glb")
            .installation(mock_installation())
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn builder_auto_output_path_builds_ok() {
        // When no output specified, builder should auto-generate from source + format extension
        let result = ConversionJobBuilder::new()
            .source("/test/model.blend")
            .options(
                ConversionOptions::builder()
                    .format(OutputFormat::GltfSeparate)
                    .build(),
            )
            .installation(mock_installation())
            .build();
        assert!(result.is_ok());
    }
}

// ============================================================================
// MODULE: Cache mutation kill tests — targets cache.rs misses
// ============================================================================
mod cache_mutation_kill_tests {
    use super::*;
    use astraweave_blend::cache::CacheEntry;
    use std::fs;
    use tempfile::TempDir;

    fn make_test_version() -> BlenderVersion {
        BlenderVersion::new(4, 0, 0)
    }

    /// Helper: set up a cache with a stored entry for round-trip tests.
    /// Returns (cache, temp_dir, source_path, output_path, version, options).
    fn setup_cache_with_entry() -> (
        ConversionCache,
        TempDir,
        PathBuf,
        PathBuf,
        BlenderVersion,
        ConversionOptions,
    ) {
        let temp = TempDir::new().unwrap();

        let source = temp.path().join("model.blend");
        fs::write(&source, b"BLENDER_SOURCE_DATA_FOR_TESTING").unwrap();

        let output = temp.path().join("model.glb");
        fs::write(&output, b"GLTF_BINARY_OUTPUT_DATA").unwrap();

        let cache_dir = temp.path().join("cache");
        let mut cache = ConversionCache::new(&cache_dir).unwrap();
        let version = make_test_version();
        let options = ConversionOptions::default();

        cache
            .store(&source, &output, &options, &version, 100, vec![], vec![])
            .unwrap();

        (cache, temp, source, output, version, options)
    }

    // --- CacheEntry::touch ---
    #[test]
    fn touch_updates_last_accessed() {
        let mut entry = CacheEntry::new(
            "hash".into(),
            "opts".into(),
            make_test_version(),
            PathBuf::from("out.glb"),
            PathBuf::from("src.blend"),
            100,
            50,
        );
        // Force an old timestamp
        entry.last_accessed = 1000;
        entry.touch();
        // After touch, last_accessed must be updated to something much larger
        assert!(entry.last_accessed > 1000);
    }

    // --- CacheEntry::age ---
    #[test]
    fn age_nonzero_for_old_entry() {
        let mut entry = CacheEntry::new(
            "hash".into(),
            "opts".into(),
            make_test_version(),
            PathBuf::from("out.glb"),
            PathBuf::from("src.blend"),
            100,
            50,
        );
        // Set created_at to the past
        entry.created_at = 0;
        let age = entry.age();
        assert!(age > Duration::from_secs(1_000_000));
    }

    // --- CacheEntry::time_since_access ---
    #[test]
    fn time_since_access_nonzero_for_old_entry() {
        let mut entry = CacheEntry::new(
            "hash".into(),
            "opts".into(),
            make_test_version(),
            PathBuf::from("out.glb"),
            PathBuf::from("src.blend"),
            100,
            50,
        );
        entry.last_accessed = 0;
        let tsa = entry.time_since_access();
        assert!(tsa > Duration::from_secs(1_000_000));
    }

    // --- ConversionCache::lookup disabled ---
    #[test]
    fn lookup_disabled_returns_miss() {
        let (mut cache, _temp, source, _output, version, options) = setup_cache_with_entry();
        cache.set_enabled(false);

        let result = cache.lookup(&source, &options, &version).unwrap();
        assert!(
            matches!(result, CacheLookup::Miss { .. }),
            "disabled cache must return miss"
        );
    }

    // --- lookup version mismatch ---
    #[test]
    fn lookup_version_mismatch_returns_miss() {
        let (mut cache, _temp, source, _output, _version, options) = setup_cache_with_entry();
        let different_version = BlenderVersion::new(4, 1, 0);

        let result = cache.lookup(&source, &options, &different_version).unwrap();
        match result {
            CacheLookup::Miss { reason } => {
                assert_eq!(reason, CacheMissReason::BlenderVersionChanged);
            }
            CacheLookup::Hit { .. } => panic!("expected miss for version mismatch"),
            _ => panic!("unexpected variant"),
        }
    }

    // --- lookup options mismatch ---
    #[test]
    fn lookup_options_mismatch_returns_miss() {
        let (mut cache, _temp, source, _output, version, _options) = setup_cache_with_entry();
        let different_options = ConversionOptions::game_runtime();

        let result = cache.lookup(&source, &different_options, &version).unwrap();
        match result {
            CacheLookup::Miss { reason } => {
                assert_eq!(reason, CacheMissReason::OptionsChanged);
            }
            CacheLookup::Hit { .. } => panic!("expected miss for options mismatch"),
            _ => panic!("unexpected variant"),
        }
    }

    // --- lookup source modified ---
    #[test]
    fn lookup_source_modified_returns_miss() {
        let (mut cache, _temp, source, _output, version, options) = setup_cache_with_entry();

        // Modify the source file so hash changes
        fs::write(&source, b"COMPLETELY_DIFFERENT_SOURCE_CONTENT").unwrap();

        let result = cache.lookup(&source, &options, &version).unwrap();
        match result {
            CacheLookup::Miss { reason } => {
                assert_eq!(reason, CacheMissReason::SourceModified);
            }
            CacheLookup::Hit { .. } => panic!("expected miss for source modification"),
            _ => panic!("unexpected variant"),
        }
    }

    // --- lookup output missing ---
    #[test]
    fn lookup_output_missing_returns_miss() {
        let (mut cache, temp, source, _output, version, options) = setup_cache_with_entry();

        // Delete all files in cache dir that match the cached output pattern
        let cache_dir = temp.path().join("cache");
        for entry in fs::read_dir(&cache_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().map_or(false, |e| e == "glb") {
                fs::remove_file(&path).unwrap();
            }
        }

        let result = cache.lookup(&source, &options, &version).unwrap();
        match result {
            CacheLookup::Miss { reason } => {
                assert_eq!(reason, CacheMissReason::OutputMissing);
            }
            CacheLookup::Hit { .. } => panic!("expected miss when output file deleted"),
            _ => panic!("unexpected variant"),
        }
    }

    // --- lookup expired (max_age) ---
    #[test]
    fn lookup_expired_returns_miss() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("model.blend");
        fs::write(&src, b"SOURCE_CONTENT_EXPIRE").unwrap();
        let out = temp.path().join("model.glb");
        fs::write(&out, b"OUTPUT_CONTENT_EXPIRE").unwrap();
        let cache_dir = temp.path().join("cache");
        let v = make_test_version();
        let o = ConversionOptions::default();

        // Store an entry first (no max_age yet)
        let mut cache = ConversionCache::new(&cache_dir).unwrap();
        cache.store(&src, &out, &o, &v, 50, vec![], vec![]).unwrap();

        // Read the manifest file, backdate created_at to epoch 0, rewrite it
        let manifest_path = cache_dir.join("cache_manifest.ron");
        let manifest_text = fs::read_to_string(&manifest_path).unwrap();
        // Replace the created_at timestamp (which is a recent epoch seconds value)
        // with 0 so the entry appears very old.
        let backdated = manifest_text.replacen("created_at:", "created_at: 0, // was:", 1);
        // Actually, RON has field: value format, so we need to replace the entire line.
        // Let's use a regex to replace created_at: <number> with created_at: 0
        let re = regex::Regex::new(r"created_at:\s*\d+").unwrap();
        let backdated = re.replace_all(&manifest_text, "created_at: 0");
        fs::write(&manifest_path, backdated.as_ref()).unwrap();

        // Re-open cache with max_age = 1 second
        let mut cache2 = ConversionCache::new(&cache_dir)
            .unwrap()
            .with_max_age(Some(Duration::from_secs(1)));

        let result = cache2.lookup(&src, &o, &v).unwrap();
        match result {
            CacheLookup::Miss { reason } => {
                assert_eq!(reason, CacheMissReason::Expired);
            }
            CacheLookup::Hit { .. } => panic!("expected miss for expired entry"),
            _ => panic!("unexpected variant"),
        }
    }

    // --- enforce_size_limit via store with max_size ---
    #[test]
    fn store_evicts_lru_when_over_max_size() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");

        // Create cache with tiny max_size — any real file exceeds it
        let mut cache = ConversionCache::new(&cache_dir)
            .unwrap()
            .with_max_size(Some(1)); // 1 byte limit

        let src = temp.path().join("a.blend");
        fs::write(&src, b"BLEND_A").unwrap();
        let out = temp.path().join("a.glb");
        fs::write(&out, b"OUTPUT_A_DATA_WHICH_IS_MORE_THAN_1_BYTE").unwrap();

        let v = make_test_version();
        let o = ConversionOptions::default();

        cache
            .store(&src, &out, &o, &v, 100, vec![], vec![])
            .unwrap();

        // After storing, enforced size limit should have evicted entries
        // So entry count should be 0 (evicted) since total_size > 1
        assert_eq!(cache.stats().entry_count, 0);
    }

    // --- normalize_path returns meaningful string ---
    #[test]
    fn normalize_path_via_round_trip() {
        // normalize_path is private, but it's exercised through lookup/store.
        // If it returns "" or "xyzzy", lookup after store would fail to find the entry.
        let (mut cache, _temp, source, _output, version, options) = setup_cache_with_entry();

        // A successful hit proves normalize_path returns a consistent, non-empty, non-"xyzzy" key.
        let result = cache.lookup(&source, &options, &version).unwrap();
        assert!(
            matches!(result, CacheLookup::Hit { .. }),
            "round-trip must hit — proves normalize_path works correctly"
        );
    }

    // --- store disabled returns early ---
    #[test]
    fn store_disabled_returns_input_path() {
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        let mut cache = ConversionCache::new(&cache_dir).unwrap();
        cache.set_enabled(false);

        let src = temp.path().join("s.blend");
        fs::write(&src, b"SRC").unwrap();
        let out = temp.path().join("s.glb");
        fs::write(&out, b"OUT").unwrap();

        let result = cache
            .store(
                &src,
                &out,
                &ConversionOptions::default(),
                &make_test_version(),
                10,
                vec![],
                vec![],
            )
            .unwrap();

        // When disabled, store returns the output_path unchanged
        assert_eq!(result, out);
        assert_eq!(cache.stats().entry_count, 0);
    }

    #[test]
    fn builder_with_custom_options() {
        let opts = ConversionOptions::game_runtime();
        let result = ConversionJobBuilder::new()
            .source("/test/model.blend")
            .output("/out/model.glb")
            .options(opts)
            .installation(mock_installation())
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn builder_default_is_new() {
        let b1 = ConversionJobBuilder::new();
        let b2 = ConversionJobBuilder::default();
        // Both should have None sources
        assert!(b1.build().is_err());
        assert!(b2.build().is_err());
    }
}

// ============================================================================
// MODULE 14: export_script — generate_export_script content validation
// ============================================================================
mod export_script_tests {
    use super::*;

    #[test]
    fn script_contains_blend_path() {
        let script = generate_export_script(
            &PathBuf::from("/test/model.blend"),
            &PathBuf::from("/out/model.glb"),
            &ConversionOptions::default(),
            "hash123",
        );
        assert!(script.contains("/test/model.blend") || script.contains("model.blend"));
    }

    #[test]
    fn script_contains_output_path() {
        let script = generate_export_script(
            &PathBuf::from("/test/model.blend"),
            &PathBuf::from("/out/model.glb"),
            &ConversionOptions::default(),
            "hash123",
        );
        assert!(script.contains("model.glb"));
    }

    #[test]
    fn script_contains_hash() {
        let script = generate_export_script(
            &PathBuf::from("/test/model.blend"),
            &PathBuf::from("/out/model.glb"),
            &ConversionOptions::default(),
            "unique_hash_value",
        );
        assert!(script.contains("unique_hash_value"));
    }

    #[test]
    fn script_draco_true_when_enabled() {
        let mut opts = ConversionOptions::default();
        opts.gltf.draco_compression = true;
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.glb"),
            &opts,
            "h",
        );
        assert!(script.contains("DRACO_COMPRESSION = True"));
    }

    #[test]
    fn script_draco_false_when_disabled() {
        let mut opts = ConversionOptions::default();
        opts.gltf.draco_compression = false;
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.glb"),
            &opts,
            "h",
        );
        assert!(script.contains("DRACO_COMPRESSION = False"));
    }

    #[test]
    fn script_has_main_function() {
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.glb"),
            &ConversionOptions::default(),
            "h",
        );
        assert!(script.contains("def main()"));
    }

    #[test]
    fn script_has_export_gltf() {
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.glb"),
            &ConversionOptions::default(),
            "h",
        );
        assert!(script.contains("def export_gltf()"));
    }

    #[test]
    fn script_has_bpy_import() {
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.glb"),
            &ConversionOptions::default(),
            "h",
        );
        assert!(script.contains("import bpy"));
    }

    #[test]
    fn script_with_linked_libraries() {
        let mut opts = ConversionOptions::default();
        opts.linked_libraries.process_recursively = true;
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.glb"),
            &opts,
            "h",
        );
        assert!(script.contains("process_linked_libraries"));
        assert!(script.contains("_processed_libraries"));
    }

    #[test]
    fn script_without_linked_libraries() {
        let mut opts = ConversionOptions::default();
        opts.linked_libraries.process_recursively = false;
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.glb"),
            &opts,
            "h",
        );
        assert!(!script.contains("def process_linked_libraries"));
    }

    #[test]
    fn script_glb_format_string() {
        let mut opts = ConversionOptions::default();
        opts.format = OutputFormat::GlbBinary;
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.glb"),
            &opts,
            "h",
        );
        assert!(script.contains("OUTPUT_FORMAT = \"GLB\""));
    }

    #[test]
    fn script_gltf_separate_format_string() {
        let mut opts = ConversionOptions::default();
        opts.format = OutputFormat::GltfSeparate;
        let script = generate_export_script(
            &PathBuf::from("/x.blend"),
            &PathBuf::from("/x.gltf"),
            &opts,
            "h",
        );
        assert!(script.contains("OUTPUT_FORMAT = \"GLTF_SEPARATE\""));
    }
}

// ============================================================================
// MODULE 15: ConversionProgress — default state
// ============================================================================
mod conversion_progress_tests {
    use super::*;

    #[test]
    fn default_stage_is_initializing() {
        let p = ConversionProgress::default();
        assert_eq!(p.stage, ConversionStage::Initializing);
    }

    #[test]
    fn default_message_is_initializing() {
        let p = ConversionProgress::default();
        assert_eq!(p.message, "Initializing...");
    }

    #[test]
    fn default_elapsed_is_zero() {
        let p = ConversionProgress::default();
        assert_eq!(p.elapsed, Duration::ZERO);
    }
}

// ============================================================================
// MODULE 16: ConversionResult — field verification
// ============================================================================
mod conversion_result_tests {
    use super::*;

    #[test]
    fn result_from_cache_flag() {
        let result = ConversionResult {
            output_path: PathBuf::from("/out/model.glb"),
            output_size: 5000,
            duration: Duration::from_millis(50),
            from_cache: true,
            blender_version: "4.0.0".to_string(),
            texture_files: vec![],
            linked_libraries: vec![],
            stdout: None,
            stderr: None,
        };
        assert!(result.from_cache);
        assert_eq!(result.output_size, 5000);
        assert_eq!(result.blender_version, "4.0.0");
    }

    #[test]
    fn result_not_from_cache() {
        let result = ConversionResult {
            output_path: PathBuf::from("/out/model.glb"),
            output_size: 10000,
            duration: Duration::from_secs(5),
            from_cache: false,
            blender_version: "3.6.0".to_string(),
            texture_files: vec![PathBuf::from("tex.png")],
            linked_libraries: vec![PathBuf::from("lib.blend")],
            stdout: Some("Export complete".into()),
            stderr: Some("".into()),
        };
        assert!(!result.from_cache);
        assert_eq!(result.texture_files.len(), 1);
        assert_eq!(result.linked_libraries.len(), 1);
        assert!(result.stdout.is_some());
    }
}

// ============================================================================
// MODULE 17: BlendImporterConfig — defaults
// ============================================================================
mod importer_config_tests {
    #[test]
    fn default_cache_enabled() {
        let cfg = astraweave_blend::importer::BlendImporterConfig::default();
        assert!(cfg.cache_enabled);
    }

    #[test]
    fn default_project_root_none() {
        let cfg = astraweave_blend::importer::BlendImporterConfig::default();
        assert!(cfg.project_root.is_none());
    }
}
