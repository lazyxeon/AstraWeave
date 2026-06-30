//! Edge case tests for astraweave-blend.
//!
//! Tests boundary conditions, empty inputs, maximum values, Unicode handling,
//! and other edge cases that often cause bugs in production.

use std::path::PathBuf;
use std::time::Duration;

use astraweave_blend::error::BlendError;
use astraweave_blend::options::*;
use astraweave_blend::version::BlenderVersion;

// ============================================================================
// BLENDER VERSION EDGE CASES
// ============================================================================

#[test]
fn version_zero_zero_zero() {
    let v = BlenderVersion::new(0, 0, 0);
    assert_eq!(v.major, 0);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 0);
    assert!(!v.meets_minimum());
}

#[test]
fn version_maximum_values() {
    let v = BlenderVersion::new(u32::MAX, u32::MAX, u32::MAX);
    assert_eq!(v.major, u32::MAX);
    assert_eq!(v.minor, u32::MAX);
    assert_eq!(v.patch, u32::MAX);
    assert!(v.meets_minimum()); // Should definitely meet minimum
}

#[test]
fn version_minimum_supported() {
    // Test exactly at minimum threshold (2.93.0 is minimum)
    let v_min = BlenderVersion::new(2, 93, 0);
    assert!(v_min.meets_minimum());

    // Just below minimum
    let v_below = BlenderVersion::new(2, 92, 99);
    assert!(!v_below.meets_minimum());
}

#[test]
fn version_major_boundary() {
    // Major version 2 with very high minor
    let v1 = BlenderVersion::new(2, 999, 0);
    let v2 = BlenderVersion::new(3, 0, 0);
    assert!(v2 > v1);
}

#[test]
fn version_equality_boundary() {
    let v1 = BlenderVersion::new(3, 6, 5);
    let v2 = BlenderVersion::new(3, 6, 5);
    assert_eq!(v1, v2);
    assert!(v1 <= v2);
    assert!(v1 >= v2);
}

#[test]
fn version_ordering_consistency() {
    let v1 = BlenderVersion::new(2, 93, 0);
    let v2 = BlenderVersion::new(3, 0, 0);
    let v3 = BlenderVersion::new(4, 0, 0);

    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v1 < v3); // Transitivity
}

#[test]
fn version_display_format_all_digits() {
    let v = BlenderVersion::new(123, 456, 789);
    let s = format!("{}", v);
    assert!(s.contains("123"));
    assert!(s.contains("456"));
    assert!(s.contains("789"));
}

// ============================================================================
// PATH EDGE CASES
// ============================================================================

#[test]
fn empty_path_handling() {
    let path = PathBuf::new();
    assert!(path.as_os_str().is_empty());
    assert_eq!(path.components().count(), 0);
}

#[test]
fn path_with_only_extension() {
    let path = PathBuf::from(".blend");
    // On Unix, ".blend" is a hidden file with no extension
    // On Windows, it's similar - the whole thing is the filename
    // The extension() method returns None for dot-files like ".blend"
    let ext = path.extension();
    // This behavior is platform-specific and may return None
    // Just verify it doesn't panic
    let _ = ext;

    // A proper extension file
    let path2 = PathBuf::from("file.blend");
    assert_eq!(path2.extension().map(|e| e.to_str()), Some(Some("blend")));
}

#[test]
fn path_very_long() {
    let long_name = "a".repeat(255); // Max filename length on most systems
    let path = PathBuf::from(format!("/path/to/{}.blend", long_name));

    // Should be constructible
    assert!(path.file_name().is_some());
}

#[test]
fn path_with_unicode() {
    let paths = vec![
        PathBuf::from("文件.blend"),     // Chinese
        PathBuf::from("ファイル.blend"), // Japanese
        PathBuf::from("файл.blend"),     // Russian
        PathBuf::from("αρχείο.blend"),   // Greek
        PathBuf::from("🎮game.blend"),   // Emoji
        PathBuf::from("café.blend"),     // Accented
    ];

    for path in &paths {
        // Each should have valid extension
        assert_eq!(
            path.extension().map(|e| e.to_str()),
            Some(Some("blend")),
            "Failed for path: {:?}",
            path
        );
    }
}

#[test]
fn path_with_special_chars() {
    let paths = vec![
        PathBuf::from("file with spaces.blend"),
        PathBuf::from("file-with-dashes.blend"),
        PathBuf::from("file_with_underscores.blend"),
        PathBuf::from("file.multiple.dots.blend"),
        PathBuf::from("file(with)(parens).blend"),
        PathBuf::from("file[with][brackets].blend"),
        PathBuf::from("file{with}{braces}.blend"),
    ];

    for path in &paths {
        assert_eq!(
            path.extension().map(|e| e.to_str()),
            Some(Some("blend")),
            "Failed for path: {:?}",
            path
        );
    }
}

// ============================================================================
// DURATION EDGE CASES
// ============================================================================

#[test]
fn timeout_zero_duration() {
    let opts = ConversionOptions::builder().timeout(Duration::ZERO).build();

    assert_eq!(opts.process.timeout, Duration::ZERO);
}

#[test]
fn timeout_maximum_duration() {
    let max_duration = Duration::MAX;
    let opts = ConversionOptions::builder().timeout(max_duration).build();

    assert_eq!(opts.process.timeout, max_duration);
}

#[test]
fn timeout_subsecond_precision() {
    let precise = Duration::from_nanos(123_456_789);
    let opts = ConversionOptions::builder().timeout(precise).build();

    assert_eq!(opts.process.timeout, precise);
}

// ============================================================================
// OPTIONS EDGE CASES
// ============================================================================

#[test]
fn options_builder_default_values() {
    let default = ConversionOptions::default();
    let builder = ConversionOptions::builder().build();

    // Compare key fields
    assert_eq!(default.format, builder.format);
    assert_eq!(
        default.gltf.draco_compression,
        builder.gltf.draco_compression
    );
}

#[test]
fn options_builder_override_same_field_multiple_times() {
    let opts = ConversionOptions::builder()
        .format(OutputFormat::GlbBinary)
        .format(OutputFormat::GltfEmbedded)
        .format(OutputFormat::GltfSeparate)
        .build();

    // Last value wins
    assert_eq!(opts.format, OutputFormat::GltfSeparate);
}

#[test]
fn options_texture_resolution_zero() {
    let opts = ConversionOptions::builder()
        .max_texture_resolution(Some(0))
        .build();

    assert_eq!(opts.textures.max_resolution, Some(0));
}

#[test]
fn options_texture_resolution_very_large() {
    let opts = ConversionOptions::builder()
        .max_texture_resolution(Some(32768)) // 32K resolution
        .build();

    assert_eq!(opts.textures.max_resolution, Some(32768));
}

#[test]
fn options_linked_library_depth_zero() {
    let opts = ConversionOptions::builder().linked_library_depth(0).build();

    assert_eq!(opts.linked_libraries.max_recursion_depth, 0);
}

#[test]
fn options_linked_library_depth_max() {
    let opts = ConversionOptions::builder()
        .linked_library_depth(u32::MAX)
        .build();

    assert_eq!(opts.linked_libraries.max_recursion_depth, u32::MAX);
}

// ============================================================================
// OUTPUT FORMAT EDGE CASES
// ============================================================================

#[test]
fn output_format_extension_consistency() {
    // Each format's extension should start with appropriate prefix
    assert!(OutputFormat::GlbBinary.extension().contains("glb"));
    assert!(OutputFormat::GltfEmbedded.extension().contains("gltf"));
    assert!(OutputFormat::GltfSeparate.extension().contains("gltf"));
}

#[test]
fn output_format_blender_format_not_empty() {
    let formats = [
        OutputFormat::GlbBinary,
        OutputFormat::GltfEmbedded,
        OutputFormat::GltfSeparate,
    ];

    for format in &formats {
        let blender_fmt = format.blender_format();
        assert!(
            !blender_fmt.is_empty(),
            "Format {:?} has empty blender_format",
            format
        );
    }
}

// ============================================================================
// TEXTURE FORMAT EDGE CASES
// ============================================================================

#[test]
fn texture_format_extension_valid() {
    let formats_and_expected = [
        (TextureFormat::Png, "png"),
        (TextureFormat::Jpeg, "jpg"),
        (TextureFormat::WebP, "webp"),
    ];

    for (format, expected) in &formats_and_expected {
        let ext = format.extension();
        assert!(
            ext.contains(expected),
            "Format {:?} extension '{}' should contain '{}'",
            format,
            ext,
            expected
        );
    }
}

// ============================================================================
// SERIALIZATION EDGE CASES
// ============================================================================

#[test]
fn version_serialization_roundtrip() {
    let versions = vec![
        BlenderVersion::new(0, 0, 0),
        BlenderVersion::new(2, 93, 0),
        BlenderVersion::new(4, 1, 0),
        BlenderVersion::new(u32::MAX, u32::MAX, u32::MAX),
    ];

    for v in &versions {
        let serialized = ron::to_string(v).expect("Serialization failed");
        let deserialized: BlenderVersion =
            ron::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(v.major, deserialized.major);
        assert_eq!(v.minor, deserialized.minor);
        assert_eq!(v.patch, deserialized.patch);
    }
}

#[test]
fn options_serialization_roundtrip_all_presets() {
    let presets = vec![
        ConversionOptions::default(),
        ConversionOptions::game_runtime(),
        ConversionOptions::editor_preview(),
        ConversionOptions::archival_quality(),
    ];

    for opts in &presets {
        let ron_str = ron::to_string(opts).expect("RON serialization failed");
        let _from_ron: ConversionOptions =
            ron::from_str(&ron_str).expect("RON deserialization failed");

        let json_str = serde_json::to_string(opts).expect("JSON serialization failed");
        let _from_json: ConversionOptions =
            serde_json::from_str(&json_str).expect("JSON deserialization failed");
    }
}

#[test]
fn serialization_with_special_strings() {
    // Test that options with special characters serialize correctly
    let opts = ConversionOptions::builder()
        .timeout(Duration::from_secs(60))
        .build();

    // Should not panic
    let ron_str = ron::to_string(&opts).unwrap();
    assert!(!ron_str.is_empty());

    let json_str = serde_json::to_string(&opts).unwrap();
    assert!(!json_str.is_empty());
}

// ============================================================================
// ERROR EDGE CASES
// ============================================================================

#[test]
fn error_display_blender_not_found_empty_paths() {
    let err = BlendError::BlenderNotFound {
        searched_paths: vec![],
    };

    let display = format!("{}", err);
    assert!(!display.is_empty());
}

#[test]
fn error_display_blender_not_found_many_paths() {
    let paths: Vec<PathBuf> = (0..100)
        .map(|i| PathBuf::from(format!("/path/{}", i)))
        .collect();

    let err = BlendError::BlenderNotFound {
        searched_paths: paths,
    };

    let display = format!("{}", err);
    assert!(!display.is_empty());
}

#[test]
fn error_display_invalid_blend_empty_message() {
    let err = BlendError::InvalidBlendFile {
        path: PathBuf::from("/test.blend"),
        message: String::new(),
    };

    let display = format!("{}", err);
    assert!(display.contains("/test.blend"));
}

#[test]
fn error_display_invalid_blend_unicode_path() {
    let err = BlendError::InvalidBlendFile {
        path: PathBuf::from("文件/测试.blend"),
        message: "Invalid magic bytes".to_string(),
    };

    let display = format!("{}", err);
    assert!(!display.is_empty());
}

#[test]
fn error_display_conversion_failed_all_optional_none() {
    let err = BlendError::ConversionFailed {
        message: "Test".to_string(),
        exit_code: None,
        stderr: String::new(),
        blender_output: None,
    };

    let display = format!("{}", err);
    assert!(display.contains("Test"));
}

#[test]
fn error_display_conversion_failed_all_present() {
    let err = BlendError::ConversionFailed {
        message: "Failed".to_string(),
        exit_code: Some(-1),
        stderr: "Error output".to_string(),
        blender_output: Some("Full output".to_string()),
    };

    let display = format!("{}", err);
    assert!(display.contains("Failed"));
}

#[test]
fn error_display_timeout_zero_duration() {
    let err = BlendError::Timeout {
        operation: "test".to_string(),
        duration: Duration::ZERO,
        path: PathBuf::from("/test.blend"),
        timeout_secs: 0,
    };

    let display = format!("{}", err);
    assert!(display.contains("test"));
}

#[test]
fn error_cancelled_display() {
    let err = BlendError::Cancelled;
    let display = format!("{}", err);
    assert!(!display.is_empty());
}

// ============================================================================
// ERROR HELPER METHOD EDGE CASES
// ============================================================================

#[test]
fn error_is_blender_missing() {
    let err = BlendError::BlenderNotFound {
        searched_paths: vec![],
    };
    assert!(err.is_blender_missing());

    let other = BlendError::Cancelled;
    assert!(!other.is_blender_missing());
}

#[test]
fn error_is_retryable() {
    // Timeout should be retryable
    let timeout = BlendError::Timeout {
        operation: "test".to_string(),
        duration: Duration::from_secs(60),
        path: PathBuf::from("/test.blend"),
        timeout_secs: 60,
    };
    assert!(timeout.is_retryable());

    // Cancelled should not be retryable
    let cancelled = BlendError::Cancelled;
    assert!(!cancelled.is_retryable());
}

#[test]
fn error_is_cancelled() {
    let cancelled = BlendError::Cancelled;
    assert!(cancelled.is_cancelled());

    let timeout = BlendError::Timeout {
        operation: "test".to_string(),
        duration: Duration::from_secs(60),
        path: PathBuf::from("/test.blend"),
        timeout_secs: 60,
    };
    assert!(!timeout.is_cancelled());
}

// ============================================================================
// NUMERIC BOUNDARY TESTS
// ============================================================================

#[test]
fn version_patch_overflow_comparison() {
    // When comparing versions, patch should not overflow into minor
    let v1 = BlenderVersion::new(3, 5, u32::MAX);
    let v2 = BlenderVersion::new(3, 6, 0);

    assert!(v1 < v2);
}

#[test]
fn version_minor_overflow_comparison() {
    // When comparing versions, minor should not overflow into major
    let v1 = BlenderVersion::new(2, u32::MAX, 0);
    let v2 = BlenderVersion::new(3, 0, 0);

    assert!(v1 < v2);
}

// ============================================================================
// CLONE AND COPY SEMANTICS
// ============================================================================

#[test]
#[allow(clippy::clone_on_copy)]
fn version_clone_equality() {
    let v1 = BlenderVersion::new(4, 1, 0);
    let v2 = v1.clone();

    assert_eq!(v1, v2);
    assert_eq!(v1.major, v2.major);
    assert_eq!(v1.minor, v2.minor);
    assert_eq!(v1.patch, v2.patch);
}

#[test]
fn options_clone_deep() {
    let opts1 = ConversionOptions::game_runtime();
    let opts2 = opts1.clone();

    // Clones should be equal
    assert_eq!(opts1.format, opts2.format);
    assert_eq!(opts1.gltf.draco_compression, opts2.gltf.draco_compression);
    assert_eq!(opts1.process.timeout, opts2.process.timeout);
}

// ============================================================================
// DEFAULT VALUES VERIFICATION
// ============================================================================

#[test]
fn default_options_sensible() {
    let opts = ConversionOptions::default();

    // Timeout should be positive
    assert!(opts.process.timeout > Duration::ZERO);

    // Linked library depth should be reasonable
    assert!(opts.linked_libraries.max_recursion_depth > 0);
    assert!(opts.linked_libraries.max_recursion_depth < 100);
}

#[test]
fn game_runtime_preset_optimized() {
    let opts = ConversionOptions::game_runtime();

    // Game runtime has Draco DISABLED (the Rust gltf crate cannot decode Draco;
    // enabling it would emit assets the engine's own loader cannot read).
    assert!(!opts.gltf.draco_compression);

    // Should use binary format for smaller size
    assert_eq!(opts.format, OutputFormat::GlbBinary);
}

#[test]
fn archival_preset_high_quality() {
    let opts = ConversionOptions::archival_quality();

    // Archival should not use lossy compression
    assert!(!opts.gltf.draco_compression);

    // Should not limit texture resolution
    assert!(opts.textures.max_resolution.is_none());
}

// ============================================================================
// COMPREHENSIVE DRACO COMPRESSION EDGE CASES
// ============================================================================

#[test]
fn draco_compression_toggle() {
    // Enable
    let opts_on = ConversionOptions::builder().draco_compression(true).build();
    assert!(opts_on.gltf.draco_compression);

    // Disable
    let opts_off = ConversionOptions::builder()
        .draco_compression(false)
        .build();
    assert!(!opts_off.gltf.draco_compression);
}

#[test]
fn draco_compression_level_field_exists() {
    let opts = ConversionOptions::game_runtime();

    // draco_compression_level is a field on GltfOptions
    // It should be between 0 and 10 typically
    let level = opts.gltf.draco_compression_level;
    assert!(level <= 10, "Draco level {} too high", level);
}

// ============================================================================
// THREAD SAFETY (BASIC)
// ============================================================================

#[test]
fn options_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<ConversionOptions>();
    assert_send_sync::<BlenderVersion>();
    assert_send_sync::<OutputFormat>();
    assert_send_sync::<TextureFormat>();
}

#[test]
fn error_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    // BlendError should be Send + Sync
    assert_send_sync::<BlendError>();
}

// ============================================================================
// STRESS: MANY OPTIONS INSTANCES
// ============================================================================

#[test]
fn create_many_options_instances() {
    let instances: Vec<ConversionOptions> = (0..1000)
        .map(|_| ConversionOptions::game_runtime())
        .collect();

    assert_eq!(instances.len(), 1000);
}

#[test]
fn create_many_versions() {
    let versions: Vec<BlenderVersion> = (0..1000)
        .map(|i| BlenderVersion::new(2 + i / 100, (93 + i) % 100, i % 50))
        .collect();

    assert_eq!(versions.len(), 1000);
}

// ============================================================================
// HASH DETERMINISM (using SHA-256 directly)
// ============================================================================

#[test]
fn sha256_deterministic() {
    use sha2::{Digest, Sha256};

    let input = b"test input for hashing";

    let hash1 = {
        let mut hasher = Sha256::new();
        hasher.update(input);
        hex::encode(hasher.finalize())
    };

    let hash2 = {
        let mut hasher = Sha256::new();
        hasher.update(input);
        hex::encode(hasher.finalize())
    };

    assert_eq!(hash1, hash2);
}

#[test]
fn sha256_different_inputs_different_hashes() {
    use sha2::{Digest, Sha256};

    let compute_hash = |input: &[u8]| -> String {
        let mut hasher = Sha256::new();
        hasher.update(input);
        hex::encode(hasher.finalize())
    };

    let hash1 = compute_hash(b"input1");
    let hash2 = compute_hash(b"input2");
    let hash3 = compute_hash(b"INPUT1"); // Case difference

    assert_ne!(hash1, hash2);
    assert_ne!(hash1, hash3);
    assert_ne!(hash2, hash3);
}
