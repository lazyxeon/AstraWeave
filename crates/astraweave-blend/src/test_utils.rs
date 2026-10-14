//! Test utilities, mock implementations, and property generators.
//!
//! This module provides comprehensive testing infrastructure including:
//! - Mock Blender installations for deterministic testing
//! - Property-based test generators for all major types
//! - Test fixtures and builders
//! - Adversarial input generators
//!
//! # Features
//!
//! Enable the `test-utils` feature to use these utilities:
//!
//! ```toml
//! [dev-dependencies]
//! astraweave-blend = { path = "..", features = ["test-utils"] }
//! ```

use crate::discovery::BlenderInstallation;
use crate::error::{BlendError, BlendResult};
use crate::version::BlenderVersion;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

// ============================================================================
// MOCK BLENDER INSTALLATION
// ============================================================================

/// Mock Blender installation for testing without real Blender binary.
#[derive(Debug, Clone)]
pub struct MockBlenderInstallation {
    /// Version to report.
    pub version: BlenderVersion,
    /// Path to mock executable.
    pub executable_path: PathBuf,
    /// Whether the mock should simulate failures.
    pub fail_mode: MockFailMode,
    /// Simulated conversion delay.
    pub conversion_delay: Duration,
    /// Custom error message for failures.
    pub error_message: Option<String>,
}

/// Failure modes for mock Blender.
#[derive(Debug, Clone, Default)]
pub enum MockFailMode {
    /// Normal successful operation.
    #[default]
    Success,
    /// Blender crashes during conversion.
    Crash,
    /// Blender hangs (timeout).
    Hang,
    /// Blender returns invalid output.
    InvalidOutput,
    /// Blender returns partial output.
    PartialOutput,
    /// Blender returns corrupt JSON.
    CorruptJson,
    /// Blender fails with specific exit code.
    ExitCode(i32),
    /// Blender produces zero-byte output.
    EmptyOutput,
    /// Random intermittent failures (50% chance).
    Intermittent,
    /// Fail after N conversions.
    FailAfter(u32),
}

impl Default for MockBlenderInstallation {
    fn default() -> Self {
        Self {
            version: BlenderVersion::new(4, 0, 0),
            executable_path: PathBuf::from("/mock/blender"),
            fail_mode: MockFailMode::Success,
            conversion_delay: Duration::from_millis(10),
            error_message: None,
        }
    }
}

impl MockBlenderInstallation {
    /// Creates a new mock with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a mock with specific version.
    pub fn with_version(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            version: BlenderVersion::new(major, minor, patch),
            ..Default::default()
        }
    }

    /// Sets the failure mode.
    pub fn with_fail_mode(mut self, mode: MockFailMode) -> Self {
        self.fail_mode = mode;
        self
    }

    /// Sets the simulated conversion delay.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.conversion_delay = delay;
        self
    }

    /// Converts to a real BlenderInstallation struct.
    #[allow(clippy::clone_on_copy)]
    pub fn to_installation(&self) -> BlenderInstallation {
        BlenderInstallation {
            executable_path: self.executable_path.clone(),
            version: self.version,
            discovery_method: crate::discovery::DiscoveryMethod::UserConfigured,
            install_dir: self
                .executable_path
                .parent()
                .unwrap_or(Path::new("/"))
                .to_path_buf(),
        }
    }
}

// ============================================================================
// TEST FIXTURES
// ============================================================================

/// Test fixture for creating temporary test environments.
pub struct TestFixture {
    /// Temporary directory root.
    pub temp_dir: TempDir,
    /// Path to mock source files.
    pub source_dir: PathBuf,
    /// Path to mock output files.
    pub output_dir: PathBuf,
    /// Path to cache directory.
    pub cache_dir: PathBuf,
    /// Mock Blender installation.
    pub mock_blender: MockBlenderInstallation,
    /// Created test files.
    files: Vec<PathBuf>,
}

impl TestFixture {
    /// Creates a new test fixture.
    pub fn new() -> BlendResult<Self> {
        let temp_dir = TempDir::new().map_err(|e| {
            BlendError::IoError(std::io::Error::other(format!(
                "Failed to create temp dir: {}",
                e
            )))
        })?;

        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("output");
        let cache_dir = temp_dir.path().join("cache");

        std::fs::create_dir_all(&source_dir)?;
        std::fs::create_dir_all(&output_dir)?;
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            temp_dir,
            source_dir,
            output_dir,
            cache_dir,
            mock_blender: MockBlenderInstallation::default(),
            files: Vec::new(),
        })
    }

    /// Creates a mock .blend file with specified content.
    pub fn create_mock_blend(&mut self, name: &str, content: &[u8]) -> BlendResult<PathBuf> {
        let path = self.source_dir.join(name);
        std::fs::write(&path, content)?;
        self.files.push(path.clone());
        Ok(path)
    }

    /// Creates a valid mock .blend file with Blender header.
    pub fn create_valid_blend(&mut self, name: &str) -> BlendResult<PathBuf> {
        self.create_mock_blend(name, VALID_BLEND_HEADER)
    }

    /// Creates a mock .blend file with specific size.
    pub fn create_blend_with_size(&mut self, name: &str, size: usize) -> BlendResult<PathBuf> {
        let mut content = VALID_BLEND_HEADER.to_vec();
        content.resize(size, 0);
        self.create_mock_blend(name, &content)
    }

    /// Creates a mock .glb output file.
    pub fn create_mock_output(&mut self, name: &str) -> BlendResult<PathBuf> {
        let path = self.output_dir.join(name);
        std::fs::write(&path, VALID_GLB_HEADER)?;
        self.files.push(path.clone());
        Ok(path)
    }

    /// Creates a directory structure for linked library testing.
    pub fn create_linked_library_structure(&mut self) -> BlendResult<LinkedLibraryFixture> {
        let main_blend = self.create_valid_blend("main.blend")?;
        let lib1 = self.create_valid_blend("lib/characters.blend")?;
        let lib2 = self.create_valid_blend("lib/props.blend")?;
        let lib3 = self.create_valid_blend("lib/materials.blend")?;

        Ok(LinkedLibraryFixture {
            main_file: main_blend,
            libraries: vec![lib1, lib2, lib3],
        })
    }

    /// Returns the root path of the fixture.
    pub fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Cleanup is automatic via TempDir Drop.
    pub fn cleanup(self) {
        // TempDir handles cleanup
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new().expect("Failed to create test fixture")
    }
}

/// Fixture for linked library testing.
pub struct LinkedLibraryFixture {
    /// Main .blend file.
    pub main_file: PathBuf,
    /// Linked library files.
    pub libraries: Vec<PathBuf>,
}

// ============================================================================
// VALID FILE HEADERS
// ============================================================================

/// Valid Blender file header (BLENDER magic bytes).
pub const VALID_BLEND_HEADER: &[u8] = b"BLENDER-v300";

/// Valid compressed Blender file header (gzip).
pub const VALID_COMPRESSED_BLEND_HEADER: &[u8] = &[0x1f, 0x8b, 0x08, 0x00];

/// Valid GLB file header (glTF binary magic).
pub const VALID_GLB_HEADER: &[u8] = &[0x67, 0x6c, 0x54, 0x46, 0x02, 0x00, 0x00, 0x00];

/// Valid glTF JSON header.
pub const VALID_GLTF_JSON: &str = r#"{"asset":{"version":"2.0"}}"#;

// ============================================================================
// ADVERSARIAL INPUTS
// ============================================================================

/// Collection of adversarial test inputs.
pub struct AdversarialInputs;

impl AdversarialInputs {
    /// Path traversal attack patterns.
    pub fn path_traversal_patterns() -> Vec<&'static str> {
        vec![
            "../../../etc/passwd",
            "..\\..\\..\\Windows\\System32\\config\\SAM",
            "....//....//....//etc/passwd",
            "..%2f..%2f..%2fetc/passwd",
            "..%252f..%252f..%252fetc/passwd",
            "%2e%2e/%2e%2e/%2e%2e/etc/passwd",
            "..%c0%af..%c0%af..%c0%af/etc/passwd",
            "/var/log/../../etc/passwd",
            "file:///etc/passwd",
            "\\\\server\\share\\file.blend",
            "C:\\Windows\\System32\\config\\SAM",
            "/dev/null",
            "/dev/random",
            "/proc/self/environ",
            "CON",
            "PRN",
            "AUX",
            "NUL",
            "COM1",
            "LPT1",
        ]
    }

    /// Malformed file names.
    pub fn malformed_filenames() -> Vec<String> {
        vec![
            "".to_string(),
            " ".to_string(),
            ".".to_string(),
            "..".to_string(),
            "...".to_string(),
            "/".to_string(),
            "\\".to_string(),
            "file\x00name.blend".to_string(),
            "file\nname.blend".to_string(),
            "file\rname.blend".to_string(),
            "file\tname.blend".to_string(),
            "file\x1bname.blend".to_string(),
            "a".repeat(4096), // Very long name
            "file<>:\"|?*.blend".to_string(),
            "file\u{FEFF}.blend".to_string(),        // BOM
            "file\u{202E}dlb.exe.blend".to_string(), // Right-to-left override
            "🔥🔥🔥.blend".to_string(),              // Emoji
            "файл.blend".to_string(),                // Cyrillic
            "文件.blend".to_string(),                // Chinese
            "ファイル.blend".to_string(),            // Japanese
        ]
    }

    /// Corrupt file content patterns.
    pub fn corrupt_file_contents() -> Vec<&'static [u8]> {
        vec![
            &[],                                     // Empty
            &[0x00],                                 // Single null byte
            &[0xFF; 100],                            // All 0xFF bytes
            &[0x00; 100],                            // All null bytes
            b"BLENDER",                              // Truncated header
            b"BLENDER-",                             // Truncated header
            b"NOT_BLENDER_FILE",                     // Wrong magic
            b"\x1f\x8b",                             // Truncated gzip header
            b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00", // Invalid gzip
            b"PK\x03\x04",                           // ZIP header (wrong format)
            b"\x89PNG\r\n\x1a\n",                    // PNG header (wrong format)
            b"RIFF",                                 // RIFF header (wrong format)
            b"BM",                                   // BMP header (wrong format)
        ]
    }

    /// Very long string (for buffer overflow testing).
    pub fn very_long_string(len: usize) -> String {
        "A".repeat(len)
    }

    /// Very long path (for buffer overflow testing).
    pub fn very_long_path(segments: usize) -> PathBuf {
        let mut path = PathBuf::new();
        for i in 0..segments {
            path.push(format!("dir{}", i));
        }
        path.push("file.blend");
        path
    }

    /// Unicode edge cases.
    pub fn unicode_edge_cases() -> Vec<&'static str> {
        vec![
            "\u{0000}", // Null
            "\u{FEFF}", // BOM
            "\u{FFFF}", // Invalid character
            "\u{202E}", // Right-to-left override
            "\u{202D}", // Left-to-right override
            "\u{200B}", // Zero-width space
            "\u{200C}", // Zero-width non-joiner
            "\u{200D}", // Zero-width joiner
            "\u{2028}", // Line separator
            "\u{2029}", // Paragraph separator
            // Note: Surrogate code points cannot be represented in Rust strings
            "\u{10FFFF}", // Maximum code point
        ]
    }

    /// Malicious JSON payloads.
    pub fn malicious_json_payloads() -> Vec<String> {
        vec![
            r#"{"__proto__":{"admin":true}}"#.to_string(),
            r#"{"constructor":{"prototype":{"admin":true}}}"#.to_string(),
            r#"{"a":{"b":{"c":{"d":{"e":{"f":{"g":{"h":{"i":{"j":{}}}}}}}}}}}"#.to_string(), // Deep nesting
            "{".repeat(1000),                     // Unbalanced braces
            "[".repeat(1000),                     // Unbalanced brackets
            r#"{"key":"value"/**/}"#.to_string(), // Comments (invalid JSON)
            "null".to_string(),
            "undefined".to_string(),
            "NaN".to_string(),
            "Infinity".to_string(),
            "-Infinity".to_string(),
        ]
    }
}

// ============================================================================
// PROPERTY-BASED TEST GENERATORS
// ============================================================================

/// Generators for property-based testing with proptest.
#[cfg(feature = "test-utils")]
pub mod generators {
    use super::*;
    use crate::cache::CacheEntry;
    use crate::conversion::ConversionResult;
    use crate::options::{
        AnimationOptions, CacheOptions, ConversionOptions, GltfExportOptions, LinkedLibraryOptions,
        MaterialOptions, MeshOptions, MissingLibraryAction, OutputFormat, ProcessOptions,
        SceneDecompositionOptions, TextureFormat, TextureOptions,
    };
    use rand::distributions::{Alphanumeric, DistString};
    use rand::prelude::*;
    use rand_chacha::ChaCha8Rng;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Seeded random number generator for reproducible tests.
    pub fn seeded_rng(seed: u64) -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(seed)
    }

    /// Generates random BlenderVersion.
    pub fn random_blender_version(rng: &mut impl Rng) -> BlenderVersion {
        BlenderVersion::new(
            rng.gen_range(2..5),   // Major: 2-4
            rng.gen_range(0..100), // Minor: 0-99
            rng.gen_range(0..20),  // Patch: 0-19
        )
    }

    /// Generates random ConversionOptions.
    pub fn random_conversion_options(rng: &mut impl Rng) -> ConversionOptions {
        ConversionOptions {
            format: random_output_format(rng),
            gltf: random_gltf_options(rng),
            textures: random_texture_options(rng),
            animation: random_animation_options(rng),
            mesh: random_mesh_options(rng),
            materials: MaterialOptions::default(),
            linked_libraries: random_linked_library_options(rng),
            process: random_process_options(rng),
            cache: random_cache_options(rng),
            decomposition: SceneDecompositionOptions::default(),
        }
    }

    /// Generates random OutputFormat.
    pub fn random_output_format(rng: &mut impl Rng) -> OutputFormat {
        match rng.gen_range(0..3) {
            0 => OutputFormat::GlbBinary,
            1 => OutputFormat::GltfEmbedded,
            _ => OutputFormat::GltfSeparate,
        }
    }

    /// Generates random GltfExportOptions.
    pub fn random_gltf_options(rng: &mut impl Rng) -> GltfExportOptions {
        GltfExportOptions {
            draco_compression: rng.gen_bool(0.5),
            draco_compression_level: rng.gen_range(0..11),
            export_extras: rng.gen_bool(0.5),
            export_lights: rng.gen_bool(0.3),
            export_cameras: rng.gen_bool(0.3),
            y_up: rng.gen_bool(0.9),
            selected_only: rng.gen_bool(0.2),
            visible_only: rng.gen_bool(0.8),
            active_collection_only: rng.gen_bool(0.1),
            export_armatures: rng.gen_bool(0.7),
            export_skins: rng.gen_bool(0.7),
            copyright: if rng.gen_bool(0.3) {
                Some(Alphanumeric.sample_string(rng, 20))
            } else {
                None
            },
        }
    }

    /// Generates random TextureOptions.
    pub fn random_texture_options(rng: &mut impl Rng) -> TextureOptions {
        TextureOptions {
            format: random_texture_format(rng),
            max_resolution: if rng.gen_bool(0.8) {
                Some(2_u32.pow(rng.gen_range(8..13))) // 256-4096
            } else {
                None
            },
            jpeg_quality: rng.gen_range(1..101),
            webp_quality: rng.gen_range(1..101),
            unpack_embedded: rng.gen_bool(0.9),
            generate_mipmaps: rng.gen_bool(0.3),
            flip_y: rng.gen_bool(0.2),
        }
    }

    /// Generates random TextureFormat.
    pub fn random_texture_format(rng: &mut impl Rng) -> TextureFormat {
        match rng.gen_range(0..4) {
            0 => TextureFormat::Png,
            1 => TextureFormat::Jpeg,
            2 => TextureFormat::WebP,
            _ => TextureFormat::Original,
        }
    }

    /// Generates random AnimationOptions.
    pub fn random_animation_options(rng: &mut impl Rng) -> AnimationOptions {
        AnimationOptions {
            export_animations: rng.gen_bool(0.8),
            export_shape_keys: rng.gen_bool(0.7),
            merge_animations: rng.gen_bool(0.2),
            optimize_animation_size: rng.gen_bool(0.4),
            force_linear_interpolation: rng.gen_bool(0.2),
            export_nla_strips: rng.gen_bool(0.5),
            sampling_rate: if rng.gen_bool(0.3) {
                Some(rng.gen_range(1.0..120.0))
            } else {
                None
            },
            force_sampling: rng.gen_bool(0.3),
        }
    }

    /// Generates random MeshOptions.
    pub fn random_mesh_options(rng: &mut impl Rng) -> MeshOptions {
        MeshOptions {
            apply_modifiers: rng.gen_bool(0.9),
            triangulate: rng.gen_bool(0.95),
            export_vertex_colors: rng.gen_bool(0.8),
            export_uvs: rng.gen_bool(0.95),
            export_normals: rng.gen_bool(0.95),
            export_tangents: rng.gen_bool(0.7),
            use_mesh_instancing: rng.gen_bool(0.5),
            merge_vertices_distance: if rng.gen_bool(0.2) {
                Some(rng.gen_range(0.0001..0.01))
            } else {
                None
            },
            export_loose_edges: rng.gen_bool(0.2),
            export_loose_points: rng.gen_bool(0.1),
        }
    }

    /// Generates random LinkedLibraryOptions.
    pub fn random_linked_library_options(rng: &mut impl Rng) -> LinkedLibraryOptions {
        LinkedLibraryOptions {
            process_recursively: rng.gen_bool(0.9),
            max_recursion_depth: rng.gen_range(1..20),
            search_paths: (0..rng.gen_range(0..3))
                .map(|_| PathBuf::from(Alphanumeric.sample_string(rng, 10)))
                .collect(),
            missing_library_action: random_missing_library_action(rng),
            detect_circular_references: rng.gen_bool(0.9),
        }
    }

    /// Generates random MissingLibraryAction.
    pub fn random_missing_library_action(rng: &mut impl Rng) -> MissingLibraryAction {
        match rng.gen_range(0..3) {
            0 => MissingLibraryAction::Skip,
            1 => MissingLibraryAction::Warn,
            _ => MissingLibraryAction::Fail,
        }
    }

    /// Generates random ProcessOptions.
    pub fn random_process_options(rng: &mut impl Rng) -> ProcessOptions {
        ProcessOptions {
            timeout: Duration::from_secs(rng.gen_range(10..3600)),
            cancellable: rng.gen_bool(0.9),
            working_directory: None,
            extra_blender_args: Vec::new(),
            environment: Vec::new(),
            capture_output: rng.gen_bool(0.9),
            threads: rng.gen_range(0..16),
            parallel_workers: rng.gen_range(0..8),
        }
    }

    /// Generates random CacheOptions.
    pub fn random_cache_options(rng: &mut impl Rng) -> CacheOptions {
        CacheOptions {
            enabled: rng.gen_bool(0.9),
            cache_directory: None,
            max_cache_size: if rng.gen_bool(0.5) {
                Some(rng.gen_range(1024 * 1024..1024 * 1024 * 1024))
            } else {
                None
            },
            max_age: if rng.gen_bool(0.5) {
                Some(Duration::from_secs(rng.gen_range(3600..86400 * 30)))
            } else {
                None
            },
            validate_on_access: rng.gen_bool(0.8),
        }
    }

    /// Generates random CacheEntry.
    pub fn random_cache_entry(rng: &mut impl Rng) -> CacheEntry {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        CacheEntry {
            source_hash: hex::encode(random_sha256(rng)),
            options_hash: hex::encode(random_sha256(rng)),
            blender_version: random_blender_version(rng),
            output_path: PathBuf::from(format!("{}.glb", Alphanumeric.sample_string(rng, 16))),
            source_path: PathBuf::from(format!("{}.blend", Alphanumeric.sample_string(rng, 16))),
            created_at: rng.gen_range(now - 86400 * 30..now),
            last_accessed: rng.gen_range(now - 86400 * 7..now),
            output_size: rng.gen_range(1024..1024 * 1024 * 100),
            conversion_duration_ms: rng.gen_range(100..60000),
            texture_files: (0..rng.gen_range(0..10))
                .map(|_| PathBuf::from(format!("{}.png", Alphanumeric.sample_string(rng, 8))))
                .collect(),
            linked_libraries: (0..rng.gen_range(0..5))
                .map(|_| PathBuf::from(format!("{}.blend", Alphanumeric.sample_string(rng, 8))))
                .collect(),
        }
    }

    /// Generates random SHA-256 hash bytes.
    pub fn random_sha256(rng: &mut impl Rng) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        bytes
    }

    /// Generates random valid path.
    pub fn random_valid_path(rng: &mut impl Rng, max_depth: usize) -> PathBuf {
        let depth = rng.gen_range(1..=max_depth);
        let mut path = PathBuf::new();
        for _ in 0..depth {
            let len = rng.gen_range(1..20);
            path.push(Alphanumeric.sample_string(rng, len));
        }
        path.push(format!("{}.blend", Alphanumeric.sample_string(rng, 8)));
        path
    }

    /// Generates random ConversionResult.
    pub fn random_conversion_result(rng: &mut impl Rng) -> ConversionResult {
        ConversionResult {
            output_path: PathBuf::from(format!("{}.glb", Alphanumeric.sample_string(rng, 16))),
            output_size: rng.gen_range(1024..1024 * 1024 * 100),
            duration: Duration::from_millis(rng.gen_range(100..60000)),
            from_cache: rng.gen_bool(0.3),
            blender_version: format!("{}", random_blender_version(rng)),
            texture_files: (0..rng.gen_range(0..10))
                .map(|_| PathBuf::from(format!("{}.png", Alphanumeric.sample_string(rng, 8))))
                .collect(),
            linked_libraries: (0..rng.gen_range(0..5))
                .map(|_| PathBuf::from(format!("{}.blend", Alphanumeric.sample_string(rng, 8))))
                .collect(),
            stdout: if rng.gen_bool(0.5) {
                Some(Alphanumeric.sample_string(rng, 100))
            } else {
                None
            },
            stderr: if rng.gen_bool(0.2) {
                Some(Alphanumeric.sample_string(rng, 50))
            } else {
                None
            },
        }
    }
}

// ============================================================================
// STATISTICAL HELPERS
// ============================================================================

/// Statistical counter for test metrics.
#[derive(Debug, Default)]
pub struct TestStats {
    /// Total test count.
    pub total: AtomicU64,
    /// Passed test count.
    pub passed: AtomicU64,
    /// Failed test count.
    pub failed: AtomicU64,
    /// Skipped test count.
    pub skipped: AtomicU64,
}

impl TestStats {
    /// Creates new empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a passed test.
    pub fn record_pass(&self) {
        self.total.fetch_add(1, Ordering::SeqCst);
        self.passed.fetch_add(1, Ordering::SeqCst);
    }

    /// Records a failed test.
    pub fn record_fail(&self) {
        self.total.fetch_add(1, Ordering::SeqCst);
        self.failed.fetch_add(1, Ordering::SeqCst);
    }

    /// Records a skipped test.
    pub fn record_skip(&self) {
        self.total.fetch_add(1, Ordering::SeqCst);
        self.skipped.fetch_add(1, Ordering::SeqCst);
    }

    /// Returns summary string.
    pub fn summary(&self) -> String {
        format!(
            "Total: {}, Passed: {}, Failed: {}, Skipped: {}",
            self.total.load(Ordering::SeqCst),
            self.passed.load(Ordering::SeqCst),
            self.failed.load(Ordering::SeqCst),
            self.skipped.load(Ordering::SeqCst)
        )
    }

    /// Returns pass rate as percentage.
    pub fn pass_rate(&self) -> f64 {
        let total = self.total.load(Ordering::SeqCst);
        let passed = self.passed.load(Ordering::SeqCst);
        if total == 0 {
            100.0
        } else {
            (passed as f64 / total as f64) * 100.0
        }
    }
}

// ============================================================================
// ASSERTION HELPERS
// ============================================================================

/// Asserts that a Result is an error of specific type.
#[macro_export]
macro_rules! assert_blend_error {
    ($result:expr, $pattern:pat) => {
        match $result {
            Err($pattern) => {}
            Err(e) => panic!(
                "Expected error pattern {}, got: {:?}",
                stringify!($pattern),
                e
            ),
            Ok(v) => panic!("Expected error, got Ok: {:?}", v),
        }
    };
}

/// Asserts that duration is within expected range.
#[macro_export]
macro_rules! assert_duration_range {
    ($duration:expr, $min:expr, $max:expr) => {
        let dur = $duration;
        let min_dur = $min;
        let max_dur = $max;
        assert!(
            dur >= min_dur && dur <= max_dur,
            "Duration {:?} not in range [{:?}, {:?}]",
            dur,
            min_dur,
            max_dur
        );
    };
}

/// Asserts that a path exists.
#[macro_export]
macro_rules! assert_path_exists {
    ($path:expr) => {
        let p = $path;
        assert!(p.exists(), "Path does not exist: {:?}", p);
    };
}

/// Asserts that a path does not exist.
#[macro_export]
macro_rules! assert_path_not_exists {
    ($path:expr) => {
        let p = $path;
        assert!(!p.exists(), "Path should not exist: {:?}", p);
    };
}

// ============================================================================
// TIMING UTILITIES
// ============================================================================

/// Times the execution of a closure.
pub fn time_execution<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = std::time::Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Async version of time_execution.
pub async fn time_execution_async<F, T>(f: F) -> (T, Duration)
where
    F: std::future::Future<Output = T>,
{
    let start = std::time::Instant::now();
    let result = f.await;
    let duration = start.elapsed();
    (result, duration)
}

/// Benchmark helper that runs a function N times and returns statistics.
pub fn benchmark_fn<F>(name: &str, iterations: usize, f: F) -> BenchmarkResult
where
    F: Fn(),
{
    let mut durations = Vec::with_capacity(iterations);

    // Warmup
    for _ in 0..std::cmp::min(10, iterations / 10) {
        f();
    }

    // Measure
    for _ in 0..iterations {
        let start = std::time::Instant::now();
        f();
        durations.push(start.elapsed());
    }

    BenchmarkResult::from_durations(name, &durations)
}

/// Benchmark results with statistics.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Benchmark name.
    pub name: String,
    /// Number of iterations.
    pub iterations: usize,
    /// Minimum duration.
    pub min: Duration,
    /// Maximum duration.
    pub max: Duration,
    /// Mean duration.
    pub mean: Duration,
    /// Median duration.
    pub median: Duration,
    /// Standard deviation in nanoseconds.
    pub std_dev_nanos: f64,
    /// Total time.
    pub total: Duration,
}

impl BenchmarkResult {
    /// Creates BenchmarkResult from duration samples.
    pub fn from_durations(name: &str, durations: &[Duration]) -> Self {
        let mut sorted: Vec<_> = durations.to_vec();
        sorted.sort();

        let total: Duration = durations.iter().sum();
        let mean_nanos = total.as_nanos() as f64 / durations.len() as f64;

        let variance: f64 = durations
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean_nanos;
                diff * diff
            })
            .sum::<f64>()
            / durations.len() as f64;

        Self {
            name: name.to_string(),
            iterations: durations.len(),
            min: sorted.first().copied().unwrap_or_default(),
            max: sorted.last().copied().unwrap_or_default(),
            mean: Duration::from_nanos(mean_nanos as u64),
            median: sorted.get(sorted.len() / 2).copied().unwrap_or_default(),
            std_dev_nanos: variance.sqrt(),
            total,
        }
    }

    /// Prints benchmark result in human-readable format.
    pub fn print(&self) {
        println!("Benchmark: {}", self.name);
        println!("  Iterations: {}", self.iterations);
        println!("  Min:        {:?}", self.min);
        println!("  Max:        {:?}", self.max);
        println!("  Mean:       {:?}", self.mean);
        println!("  Median:     {:?}", self.median);
        println!("  Std Dev:    {:.2} ns", self.std_dev_nanos);
        println!("  Total:      {:?}", self.total);
    }
}

// ============================================================================
// MEMORY TRACKING
// ============================================================================

/// Tracks memory allocation during a test.
#[derive(Debug, Default)]
pub struct MemoryTracker {
    /// Starting memory (approximate).
    start_memory: usize,
    /// Peak memory (approximate).
    peak_memory: usize,
}

impl MemoryTracker {
    /// Creates a new memory tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records current approximate memory usage.
    pub fn record_start(&mut self) {
        // Note: This is a rough approximation
        // For accurate memory tracking, use a profiler
        self.start_memory = Self::current_memory();
    }

    /// Updates peak memory.
    pub fn record_peak(&mut self) {
        let current = Self::current_memory();
        if current > self.peak_memory {
            self.peak_memory = current;
        }
    }

    /// Returns approximate memory delta.
    pub fn delta(&self) -> usize {
        self.peak_memory.saturating_sub(self.start_memory)
    }

    /// Gets current approximate memory usage.
    fn current_memory() -> usize {
        // This is a rough approximation
        // Uses /proc/self/statm on Linux, rough estimate on other platforms
        #[cfg(target_os = "linux")]
        {
            if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
                if let Some(size) = statm.split_whitespace().next() {
                    if let Ok(pages) = size.parse::<usize>() {
                        return pages * 4096; // Typical page size
                    }
                }
            }
        }
        0 // Unknown on other platforms
    }
}

// ============================================================================
// CONCURRENT TEST HELPERS
// ============================================================================

/// Runs a function concurrently from multiple tasks.
pub async fn run_concurrent<F, Fut, T>(count: usize, f: F) -> Vec<T>
where
    F: Fn(usize) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = T> + Send,
    T: Send + 'static,
{
    let f = Arc::new(f);
    let mut handles = Vec::with_capacity(count);

    for i in 0..count {
        let f = f.clone();
        handles.push(tokio::spawn(async move { f(i).await }));
    }

    let mut results = Vec::with_capacity(count);
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }
    results
}

/// Stress tests a function with many concurrent calls.
pub async fn stress_test<F, Fut>(
    name: &str,
    concurrent_tasks: usize,
    iterations_per_task: usize,
    f: F,
) -> StressTestResult
where
    F: Fn(usize, usize) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = bool> + Send,
{
    let f = Arc::new(f);
    let success_count = Arc::new(AtomicU64::new(0));
    let failure_count = Arc::new(AtomicU64::new(0));
    let start = std::time::Instant::now();

    let mut handles = Vec::with_capacity(concurrent_tasks);

    for task_id in 0..concurrent_tasks {
        let f = f.clone();
        let success = success_count.clone();
        let failure = failure_count.clone();

        handles.push(tokio::spawn(async move {
            for iter in 0..iterations_per_task {
                if f(task_id, iter).await {
                    success.fetch_add(1, Ordering::SeqCst);
                } else {
                    failure.fetch_add(1, Ordering::SeqCst);
                }
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    let successes = success_count.load(Ordering::SeqCst);
    let failures = failure_count.load(Ordering::SeqCst);

    StressTestResult {
        name: name.to_string(),
        total_operations: concurrent_tasks * iterations_per_task,
        successes: successes as usize,
        failures: failures as usize,
        duration,
        ops_per_second: (concurrent_tasks * iterations_per_task) as f64 / duration.as_secs_f64(),
    }
}

/// Result of a stress test.
#[derive(Debug, Clone)]
pub struct StressTestResult {
    /// Test name.
    pub name: String,
    /// Total operations attempted.
    pub total_operations: usize,
    /// Successful operations.
    pub successes: usize,
    /// Failed operations.
    pub failures: usize,
    /// Total duration.
    pub duration: Duration,
    /// Operations per second.
    pub ops_per_second: f64,
}

impl StressTestResult {
    /// Returns success rate as percentage.
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            100.0
        } else {
            (self.successes as f64 / self.total_operations as f64) * 100.0
        }
    }

    /// Prints results.
    pub fn print(&self) {
        println!("Stress Test: {}", self.name);
        println!("  Total Operations: {}", self.total_operations);
        println!("  Successes:        {}", self.successes);
        println!("  Failures:         {}", self.failures);
        println!("  Success Rate:     {:.2}%", self.success_rate());
        println!("  Duration:         {:?}", self.duration);
        println!("  Ops/Second:       {:.2}", self.ops_per_second);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_blender_installation() {
        let mock = MockBlenderInstallation::with_version(4, 0, 0);
        let installation = mock.to_installation();
        assert_eq!(installation.version.major, 4);
        assert_eq!(installation.version.minor, 0);
        assert_eq!(installation.version.patch, 0);
    }

    #[test]
    fn test_test_fixture_creation() {
        let fixture = TestFixture::new().unwrap();
        assert!(fixture.source_dir.exists());
        assert!(fixture.output_dir.exists());
        assert!(fixture.cache_dir.exists());
    }

    #[test]
    fn test_adversarial_inputs() {
        let patterns = AdversarialInputs::path_traversal_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns.contains(&"../../../etc/passwd"));

        let filenames = AdversarialInputs::malformed_filenames();
        assert!(!filenames.is_empty());

        let corrupt = AdversarialInputs::corrupt_file_contents();
        assert!(!corrupt.is_empty());
    }

    #[test]
    fn test_benchmark_result() {
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(15),
        ];
        let result = BenchmarkResult::from_durations("test", &durations);
        assert_eq!(result.iterations, 3);
        assert_eq!(result.min, Duration::from_millis(10));
        assert_eq!(result.max, Duration::from_millis(20));
    }

    #[test]
    fn test_test_stats() {
        let stats = TestStats::new();
        stats.record_pass();
        stats.record_pass();
        stats.record_fail();
        stats.record_skip();

        assert_eq!(stats.total.load(Ordering::SeqCst), 4);
        assert_eq!(stats.passed.load(Ordering::SeqCst), 2);
        assert_eq!(stats.failed.load(Ordering::SeqCst), 1);
        assert_eq!(stats.skipped.load(Ordering::SeqCst), 1);
        assert!((stats.pass_rate() - 50.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_stress_test_helper() {
        let result = stress_test("simple", 4, 10, |_task, _iter| async { true }).await;

        assert_eq!(result.total_operations, 40);
        assert_eq!(result.successes, 40);
        assert_eq!(result.failures, 0);
        assert!((result.success_rate() - 100.0).abs() < 0.01);
    }

    #[cfg(feature = "test-utils")]
    mod generator_tests {
        use super::super::generators::*;

        #[test]
        fn test_random_blender_version() {
            let mut rng = seeded_rng(42);
            let version = random_blender_version(&mut rng);
            assert!(version.major >= 2 && version.major <= 4);
        }

        #[test]
        fn test_random_conversion_options() {
            let mut rng = seeded_rng(42);
            let options = random_conversion_options(&mut rng);
            // Options should be valid - just verify it doesn't panic
            let _ = options.format.extension();
        }

        #[test]
        fn test_random_cache_entry() {
            let mut rng = seeded_rng(42);
            let entry = random_cache_entry(&mut rng);
            assert!(!entry.source_hash.is_empty());
            assert!(!entry.options_hash.is_empty());
        }

        #[test]
        fn test_seeded_rng_reproducibility() {
            let mut rng1 = seeded_rng(42);
            let mut rng2 = seeded_rng(42);

            let v1 = random_blender_version(&mut rng1);
            let v2 = random_blender_version(&mut rng2);

            assert_eq!(v1.major, v2.major);
            assert_eq!(v1.minor, v2.minor);
            assert_eq!(v1.patch, v2.patch);
        }
    }
}
