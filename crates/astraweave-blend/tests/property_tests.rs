//! Property-based tests for astraweave-blend.
//!
//! These tests use proptest to automatically generate test cases and find edge cases
//! that would be difficult to discover with hand-written tests.

use proptest::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

use astraweave_blend::options::*;
use astraweave_blend::version::BlenderVersion;

// ============================================================================
// PROPTEST STRATEGIES
// ============================================================================

/// Strategy for generating valid Blender versions.
fn blender_version_strategy() -> impl Strategy<Value = BlenderVersion> {
    (2u32..6, 0u32..100, 0u32..20)
        .prop_map(|(major, minor, patch)| BlenderVersion::new(major, minor, patch))
}

/// Strategy for generating valid output formats.
fn output_format_strategy() -> impl Strategy<Value = OutputFormat> {
    prop_oneof![
        Just(OutputFormat::GlbBinary),
        Just(OutputFormat::GltfEmbedded),
        Just(OutputFormat::GltfSeparate),
    ]
}

/// Strategy for generating valid texture formats.
fn texture_format_strategy() -> impl Strategy<Value = TextureFormat> {
    prop_oneof![
        Just(TextureFormat::Png),
        Just(TextureFormat::Jpeg),
        Just(TextureFormat::WebP),
        Just(TextureFormat::Original),
    ]
}

/// Strategy for generating valid missing library actions.
fn missing_library_action_strategy() -> impl Strategy<Value = MissingLibraryAction> {
    prop_oneof![
        Just(MissingLibraryAction::Skip),
        Just(MissingLibraryAction::Warn),
        Just(MissingLibraryAction::Fail),
    ]
}

/// Strategy for generating valid file paths.
fn valid_path_strategy() -> impl Strategy<Value = PathBuf> {
    prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5).prop_map(|segments| {
        let mut path = PathBuf::new();
        for seg in segments {
            path.push(seg);
        }
        path
    })
}

/// Strategy for generating valid GltfExportOptions.
fn gltf_export_options_strategy() -> impl Strategy<Value = GltfExportOptions> {
    (
        any::<bool>(),                              // draco_compression
        0u8..11,                                    // draco_compression_level
        any::<bool>(),                              // export_extras
        any::<bool>(),                              // export_lights
        any::<bool>(),                              // export_cameras
        any::<bool>(),                              // y_up
        any::<bool>(),                              // selected_only
        any::<bool>(),                              // visible_only
        any::<bool>(),                              // active_collection_only
        any::<bool>(),                              // export_armatures
        any::<bool>(),                              // export_skins
        proptest::option::of("[a-zA-Z0-9 ]{0,50}"), // copyright
    )
        .prop_map(
            |(
                draco,
                level,
                extras,
                lights,
                cameras,
                y_up,
                selected,
                visible,
                active,
                armatures,
                skins,
                copyright,
            )| {
                GltfExportOptions {
                    draco_compression: draco,
                    draco_compression_level: level,
                    export_extras: extras,
                    export_lights: lights,
                    export_cameras: cameras,
                    y_up,
                    selected_only: selected,
                    visible_only: visible,
                    active_collection_only: active,
                    export_armatures: armatures,
                    export_skins: skins,
                    copyright,
                }
            },
        )
}

/// Strategy for generating valid TextureOptions.
fn texture_options_strategy() -> impl Strategy<Value = TextureOptions> {
    (
        texture_format_strategy(),
        proptest::option::of(128u32..8192), // max_resolution
        1u8..101,                           // jpeg_quality
        1u8..101,                           // webp_quality
        any::<bool>(),                      // unpack_embedded
        any::<bool>(),                      // generate_mipmaps
        any::<bool>(),                      // flip_y
    )
        .prop_map(
            |(format, max_res, jpeg_q, webp_q, unpack, mipmaps, flip)| TextureOptions {
                format,
                max_resolution: max_res,
                jpeg_quality: jpeg_q,
                webp_quality: webp_q,
                unpack_embedded: unpack,
                generate_mipmaps: mipmaps,
                flip_y: flip,
            },
        )
}

/// Strategy for generating valid AnimationOptions.
fn animation_options_strategy() -> impl Strategy<Value = AnimationOptions> {
    (
        any::<bool>(),                       // export_animations
        any::<bool>(),                       // export_shape_keys
        any::<bool>(),                       // merge_animations
        any::<bool>(),                       // optimize_animation_size
        any::<bool>(),                       // force_linear_interpolation
        any::<bool>(),                       // export_nla_strips
        proptest::option::of(1.0f32..120.0), // sampling_rate
        any::<bool>(),                       // force_sampling
    )
        .prop_map(
            |(exp_anim, exp_shape, merge, opt, force_lin, nla, rate, force_samp)| {
                AnimationOptions {
                    export_animations: exp_anim,
                    export_shape_keys: exp_shape,
                    merge_animations: merge,
                    optimize_animation_size: opt,
                    force_linear_interpolation: force_lin,
                    export_nla_strips: nla,
                    sampling_rate: rate,
                    force_sampling: force_samp,
                }
            },
        )
}

/// Strategy for generating valid MeshOptions.
fn mesh_options_strategy() -> impl Strategy<Value = MeshOptions> {
    (
        any::<bool>(),                        // apply_modifiers
        any::<bool>(),                        // triangulate
        any::<bool>(),                        // export_vertex_colors
        any::<bool>(),                        // export_uvs
        any::<bool>(),                        // export_normals
        any::<bool>(),                        // export_tangents
        any::<bool>(),                        // use_mesh_instancing
        proptest::option::of(0.0001f32..0.1), // merge_vertices_distance
        any::<bool>(),                        // export_loose_edges
        any::<bool>(),                        // export_loose_points
    )
        .prop_map(
            |(mods, tri, colors, uvs, normals, tangents, inst, merge, edges, points)| MeshOptions {
                apply_modifiers: mods,
                triangulate: tri,
                export_vertex_colors: colors,
                export_uvs: uvs,
                export_normals: normals,
                export_tangents: tangents,
                use_mesh_instancing: inst,
                merge_vertices_distance: merge,
                export_loose_edges: edges,
                export_loose_points: points,
            },
        )
}

/// Strategy for generating valid LinkedLibraryOptions.
fn linked_library_options_strategy() -> impl Strategy<Value = LinkedLibraryOptions> {
    (
        any::<bool>(),                                      // process_recursively
        1u32..50,                                           // max_recursion_depth
        prop::collection::vec(valid_path_strategy(), 0..5), // search_paths
        missing_library_action_strategy(),                  // missing_library_action
        any::<bool>(),                                      // detect_circular_references
    )
        .prop_map(
            |(recursive, depth, paths, action, detect)| LinkedLibraryOptions {
                process_recursively: recursive,
                max_recursion_depth: depth,
                search_paths: paths,
                missing_library_action: action,
                detect_circular_references: detect,
            },
        )
}

/// Strategy for generating valid ProcessOptions.
fn process_options_strategy() -> impl Strategy<Value = ProcessOptions> {
    (
        10u64..3600,   // timeout seconds
        any::<bool>(), // cancellable
        any::<bool>(), // capture_output
        0u32..32,      // threads
    )
        .prop_map(
            |(timeout_secs, cancellable, capture, threads)| ProcessOptions {
                timeout: Duration::from_secs(timeout_secs),
                cancellable,
                working_directory: None,
                extra_blender_args: Vec::new(),
                environment: Vec::new(),
                capture_output: capture,
                threads,
                parallel_workers: 0,
            },
        )
}

/// Strategy for generating valid CacheOptions.
fn cache_options_strategy() -> impl Strategy<Value = CacheOptions> {
    (
        any::<bool>(),                                 // enabled
        proptest::option::of(1024u64..10_000_000_000), // max_cache_size
        proptest::option::of(3600u64..86400 * 365),    // max_age seconds
        any::<bool>(),                                 // validate_on_access
    )
        .prop_map(|(enabled, max_size, max_age_secs, validate)| CacheOptions {
            enabled,
            cache_directory: None,
            max_cache_size: max_size,
            max_age: max_age_secs.map(Duration::from_secs),
            validate_on_access: validate,
        })
}

/// Strategy for generating complete ConversionOptions.
fn conversion_options_strategy() -> impl Strategy<Value = ConversionOptions> {
    (
        output_format_strategy(),
        gltf_export_options_strategy(),
        texture_options_strategy(),
        animation_options_strategy(),
        mesh_options_strategy(),
        linked_library_options_strategy(),
        process_options_strategy(),
        cache_options_strategy(),
    )
        .prop_map(
            |(format, gltf, textures, animation, mesh, linked, process, cache)| ConversionOptions {
                format,
                gltf,
                textures,
                animation,
                mesh,
                materials: MaterialOptions::default(),
                linked_libraries: linked,
                process,
                cache,
                decomposition: SceneDecompositionOptions::default(),
            },
        )
}

// ============================================================================
// BLENDER VERSION TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Version components should be stored correctly.
    #[test]
    fn version_components_stored_correctly(
        major in 0u32..100,
        minor in 0u32..100,
        patch in 0u32..100
    ) {
        let version = BlenderVersion::new(major, minor, patch);
        prop_assert_eq!(version.major, major);
        prop_assert_eq!(version.minor, minor);
        prop_assert_eq!(version.patch, patch);
    }

    /// Version comparison should be transitive.
    #[test]
    fn version_comparison_transitive(
        a in blender_version_strategy(),
        b in blender_version_strategy(),
        c in blender_version_strategy()
    ) {
        // If a <= b and b <= c, then a <= c
        if a <= b && b <= c {
            prop_assert!(a <= c);
        }
        // If a >= b and b >= c, then a >= c
        if a >= b && b >= c {
            prop_assert!(a >= c);
        }
    }

    /// Version comparison should be reflexive.
    #[test]
    fn version_comparison_reflexive(v in blender_version_strategy()) {
        prop_assert!(v == v);
        prop_assert!(v <= v);
        prop_assert!(v >= v);
    }

    /// Version comparison should be symmetric for equality.
    #[test]
    fn version_equality_symmetric(
        a in blender_version_strategy(),
        b in blender_version_strategy()
    ) {
        if a == b {
            prop_assert!(b == a);
        }
    }

    /// Version should round-trip through Display and parse.
    #[test]
    fn version_display_format(v in blender_version_strategy()) {
        let displayed = format!("{}", v);
        prop_assert!(displayed.contains(&v.major.to_string()));
        prop_assert!(displayed.contains(&v.minor.to_string()));
    }

    /// Minimum version check should be consistent.
    #[test]
    fn minimum_version_check_consistent(v in blender_version_strategy()) {
        let meets = v.meets_minimum();
        // If version meets minimum, all higher versions should too
        let higher = BlenderVersion::new(v.major + 1, v.minor, v.patch);
        if meets {
            prop_assert!(higher.meets_minimum());
        }
    }
}

// ============================================================================
// OUTPUT FORMAT TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Output format extension should be non-empty.
    #[test]
    fn output_format_extension_non_empty(format in output_format_strategy()) {
        let ext = format.extension();
        prop_assert!(!ext.is_empty());
    }

    /// Output format blender_format should be non-empty.
    #[test]
    fn output_format_blender_format_non_empty(format in output_format_strategy()) {
        let bf = format.blender_format();
        prop_assert!(!bf.is_empty());
    }

    /// GLB should have glb extension.
    #[test]
    fn glb_has_correct_extension(_unused in Just(())) {
        prop_assert_eq!(OutputFormat::GlbBinary.extension(), "glb");
    }

    /// GLTF formats should have gltf extension.
    #[test]
    fn gltf_has_correct_extension(_unused in Just(())) {
        prop_assert_eq!(OutputFormat::GltfEmbedded.extension(), "gltf");
        prop_assert_eq!(OutputFormat::GltfSeparate.extension(), "gltf");
    }
}

// ============================================================================
// TEXTURE FORMAT TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Texture format extension consistency.
    #[test]
    fn texture_format_extension_consistency(format in texture_format_strategy()) {
        let ext = format.extension();
        // Only Original format can have empty extension
        if format != TextureFormat::Original {
            prop_assert!(!ext.is_empty());
        }
    }
}

// ============================================================================
// CONVERSION OPTIONS TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// ConversionOptions should always create valid defaults.
    #[test]
    fn conversion_options_defaults_valid(_unused in Just(())) {
        let opts = ConversionOptions::default();
        // Default timeout should be reasonable (at least 1 second)
        prop_assert!(opts.process.timeout.as_secs() >= 1);
    }

    /// JPEG quality should be in valid range.
    #[test]
    fn texture_jpeg_quality_valid(opts in texture_options_strategy()) {
        prop_assert!(opts.jpeg_quality >= 1 && opts.jpeg_quality <= 100);
    }

    /// WebP quality should be in valid range.
    #[test]
    fn texture_webp_quality_valid(opts in texture_options_strategy()) {
        prop_assert!(opts.webp_quality >= 1 && opts.webp_quality <= 100);
    }

    /// Draco compression level should be valid.
    #[test]
    fn gltf_draco_level_valid(opts in gltf_export_options_strategy()) {
        prop_assert!(opts.draco_compression_level <= 10);
    }

    /// Linked library recursion depth should be positive.
    #[test]
    fn linked_library_depth_positive(opts in linked_library_options_strategy()) {
        prop_assert!(opts.max_recursion_depth >= 1);
    }

    /// Process timeout should be at least 10 seconds.
    #[test]
    fn process_timeout_reasonable(opts in process_options_strategy()) {
        prop_assert!(opts.timeout.as_secs() >= 10);
    }

    /// Cache max size should be reasonable if set.
    #[test]
    fn cache_size_reasonable(opts in cache_options_strategy()) {
        if let Some(size) = opts.max_cache_size {
            prop_assert!(size >= 1024);  // At least 1KB
        }
    }

    /// Generated ConversionOptions should have valid nested options.
    #[test]
    fn conversion_options_nested_valid(opts in conversion_options_strategy()) {
        // All nested options should be valid
        prop_assert!(opts.gltf.draco_compression_level <= 10);
        prop_assert!(opts.textures.jpeg_quality >= 1 && opts.textures.jpeg_quality <= 100);
        prop_assert!(opts.linked_libraries.max_recursion_depth >= 1);
        prop_assert!(opts.process.timeout.as_secs() >= 10);
    }
}

// ============================================================================
// PRESET TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    /// Game runtime preset should have sensible settings.
    #[test]
    fn game_runtime_preset_sensible(_unused in Just(())) {
        let opts = ConversionOptions::game_runtime();
        // Game runtime should prefer binary format
        prop_assert_eq!(opts.format, OutputFormat::GlbBinary);
        // Draco is DISABLED (the Rust gltf crate cannot decode Draco)
        prop_assert!(!opts.gltf.draco_compression);
        // Should triangulate meshes
        prop_assert!(opts.mesh.triangulate);
        // Should have reasonable texture limits
        prop_assert!(opts.textures.max_resolution.is_some());
    }

    /// Editor preview preset should be fast.
    #[test]
    fn editor_preview_preset_fast(_unused in Just(())) {
        let opts = ConversionOptions::editor_preview();
        // Editor preview should skip heavy compression
        prop_assert!(!opts.gltf.draco_compression);
        // Should have lower texture resolution
        let max_res = opts.textures.max_resolution.unwrap_or(4096);
        prop_assert!(max_res <= 1024);
    }

    /// Archival preset should preserve quality.
    #[test]
    fn archival_preset_quality(_unused in Just(())) {
        let opts = ConversionOptions::archival_quality();
        // Archival should use separate format for flexibility
        prop_assert_eq!(opts.format, OutputFormat::GltfSeparate);
        // Should not compress (preserve precision)
        prop_assert!(!opts.gltf.draco_compression);
    }
}

// ============================================================================
// BUILDER PATTERN TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Builder should correctly set format.
    #[test]
    fn builder_sets_format(format in output_format_strategy()) {
        let opts = ConversionOptions::builder().format(format).build();
        prop_assert_eq!(opts.format, format);
    }

    /// Builder should correctly set draco compression.
    #[test]
    fn builder_sets_draco(enabled in any::<bool>()) {
        let opts = ConversionOptions::builder().draco_compression(enabled).build();
        prop_assert_eq!(opts.gltf.draco_compression, enabled);
    }

    /// Builder should correctly set texture format.
    #[test]
    fn builder_sets_texture_format(format in texture_format_strategy()) {
        let opts = ConversionOptions::builder().texture_format(format).build();
        prop_assert_eq!(opts.textures.format, format);
    }

    /// Builder should correctly set max texture resolution.
    #[test]
    fn builder_sets_max_resolution(res in proptest::option::of(128u32..8192)) {
        let opts = ConversionOptions::builder().max_texture_resolution(res).build();
        prop_assert_eq!(opts.textures.max_resolution, res);
    }

    /// Builder should correctly set timeout.
    #[test]
    fn builder_sets_timeout(secs in 1u64..3600) {
        let timeout = Duration::from_secs(secs);
        let opts = ConversionOptions::builder().timeout(timeout).build();
        prop_assert_eq!(opts.process.timeout, timeout);
    }

    /// Builder should correctly set animation export.
    #[test]
    fn builder_sets_animations(enabled in any::<bool>()) {
        let opts = ConversionOptions::builder().export_animations(enabled).build();
        prop_assert_eq!(opts.animation.export_animations, enabled);
    }

    /// Builder should correctly set apply modifiers.
    #[test]
    fn builder_sets_modifiers(enabled in any::<bool>()) {
        let opts = ConversionOptions::builder().apply_modifiers(enabled).build();
        prop_assert_eq!(opts.mesh.apply_modifiers, enabled);
    }

    /// Builder should correctly set linked library depth.
    #[test]
    fn builder_sets_library_depth(depth in 1u32..100) {
        let opts = ConversionOptions::builder().linked_library_depth(depth).build();
        prop_assert_eq!(opts.linked_libraries.max_recursion_depth, depth);
    }

    /// Builder should correctly set cache enabled.
    #[test]
    fn builder_sets_cache(enabled in any::<bool>()) {
        let opts = ConversionOptions::builder().cache_enabled(enabled).build();
        prop_assert_eq!(opts.cache.enabled, enabled);
    }

    /// Builder chaining should work correctly.
    #[test]
    fn builder_chaining_works(
        format in output_format_strategy(),
        draco in any::<bool>(),
        tex_format in texture_format_strategy(),
        timeout_secs in 10u64..300,
    ) {
        let opts = ConversionOptions::builder()
            .format(format)
            .draco_compression(draco)
            .texture_format(tex_format)
            .timeout(Duration::from_secs(timeout_secs))
            .build();

        prop_assert_eq!(opts.format, format);
        prop_assert_eq!(opts.gltf.draco_compression, draco);
        prop_assert_eq!(opts.textures.format, tex_format);
        prop_assert_eq!(opts.process.timeout, Duration::from_secs(timeout_secs));
    }
}

// ============================================================================
// SERIALIZATION INVARIANTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Version should serialize to valid RON.
    #[test]
    fn version_serializes_to_ron(v in blender_version_strategy()) {
        let ron_str = ron::to_string(&v);
        prop_assert!(ron_str.is_ok(), "Failed to serialize version: {:?}", ron_str.err());
    }

    /// Output format should serialize to valid RON.
    #[test]
    fn output_format_serializes_to_ron(format in output_format_strategy()) {
        let ron_str = ron::to_string(&format);
        prop_assert!(ron_str.is_ok(), "Failed to serialize format: {:?}", ron_str.err());
    }

    /// Texture format should serialize to valid RON.
    #[test]
    fn texture_format_serializes_to_ron(format in texture_format_strategy()) {
        let ron_str = ron::to_string(&format);
        prop_assert!(ron_str.is_ok(), "Failed to serialize format: {:?}", ron_str.err());
    }
}

// ============================================================================
// EDGE CASES
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Version 0.0.0 should be valid but old.
    #[test]
    fn version_zero_is_old(_unused in Just(())) {
        let v = BlenderVersion::new(0, 0, 0);
        prop_assert!(!v.meets_minimum());
    }

    /// Very high version should meet minimum.
    #[test]
    fn high_version_meets_minimum(_unused in Just(())) {
        let v = BlenderVersion::new(99, 99, 99);
        prop_assert!(v.meets_minimum());
    }

    /// Sampling rate edge cases.
    #[test]
    fn sampling_rate_edge_cases(_unused in Just(())) {
        // Minimum sampling rate
        let opts_min = AnimationOptions {
            sampling_rate: Some(0.001),
            ..Default::default()
        };
        prop_assert!(opts_min.sampling_rate.unwrap() > 0.0);

        // Maximum reasonable sampling rate
        let opts_max = AnimationOptions {
            sampling_rate: Some(10000.0),
            ..Default::default()
        };
        prop_assert!(opts_max.sampling_rate.unwrap() > 0.0);
    }

    /// Timeout edge cases.
    #[test]
    fn timeout_edge_cases(_unused in Just(())) {
        // Very short timeout (but non-zero)
        let opts_short = ProcessOptions {
            timeout: Duration::from_millis(1),
            ..Default::default()
        };
        prop_assert!(!opts_short.timeout.is_zero());

        // Very long timeout
        let opts_long = ProcessOptions {
            timeout: Duration::from_secs(86400 * 365),  // 1 year
            ..Default::default()
        };
        prop_assert!(opts_long.timeout.as_secs() > 0);
    }
}
