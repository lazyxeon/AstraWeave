#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use astraweave_blend::options::{
    AnimationOptions, CacheOptions, ConversionOptions, GltfExportOptions, LinkedLibraryOptions,
    MaterialOptions, MeshOptions, OutputFormat, ProcessOptions, SceneDecompositionOptions,
    TextureFormat, TextureOptions,
};

use std::time::Duration;

#[derive(Debug, Arbitrary)]
struct FuzzOptionsInput {
    // Output format selection
    format_index: u8,
    texture_format_index: u8,

    // Texture options
    jpeg_quality: u8,
    webp_quality: u8,
    max_resolution: u32,
    generate_mipmaps: bool,
    flip_y: bool,

    // Animation options
    export_animations: bool,
    export_shape_keys: bool,
    merge_animations: bool,
    sampling_rate: f32,

    // Mesh options
    apply_modifiers: bool,
    triangulate: bool,
    export_normals: bool,
    export_tangents: bool,

    // Material options
    export_materials: bool,
    convert_to_pbr: bool,

    // gltf options
    draco_compression: bool,
    draco_level: u8,
    y_up: bool,

    // Process options
    timeout_secs: u64,

    // Cache options
    cache_enabled: bool,
    max_cache_size_mb: u64,
}

fuzz_target!(|input: FuzzOptionsInput| {
    // Test OutputFormat enumeration
    let format = match input.format_index % 3 {
        0 => OutputFormat::GlbBinary,
        1 => OutputFormat::GltfEmbedded,
        _ => OutputFormat::GltfSeparate,
    };

    let _ = format!("{:?}", format);
    let _ = format.extension();
    let _ = format.blender_format();

    // Test TextureFormat enumeration
    let tex_format = match input.texture_format_index % 4 {
        0 => TextureFormat::Png,
        1 => TextureFormat::Jpeg,
        2 => TextureFormat::WebP,
        _ => TextureFormat::Original,
    };

    let _ = format!("{:?}", tex_format);
    let _ = tex_format.extension();

    // Test TextureOptions with clamped values
    let tex_options = TextureOptions {
        format: tex_format,
        max_resolution: Some(input.max_resolution.clamp(1, 8192)),
        jpeg_quality: input.jpeg_quality.clamp(1, 100),
        webp_quality: input.webp_quality.clamp(1, 100),
        unpack_embedded: true,
        generate_mipmaps: input.generate_mipmaps,
        flip_y: input.flip_y,
    };

    // Should not panic
    let _ = tex_options.clone();
    let _ = format!("{:?}", tex_options);

    // Test AnimationOptions
    let anim_options = AnimationOptions {
        export_animations: input.export_animations,
        export_shape_keys: input.export_shape_keys,
        merge_animations: input.merge_animations,
        optimize_animation_size: true,
        force_linear_interpolation: false,
        export_nla_strips: true,
        sampling_rate: if input.sampling_rate.is_finite() && input.sampling_rate > 0.0 {
            Some(input.sampling_rate.clamp(1.0, 120.0))
        } else {
            None
        },
        force_sampling: false,
    };

    let _ = anim_options.clone();
    let _ = format!("{:?}", anim_options);

    // Test MeshOptions
    let mesh_options = MeshOptions {
        apply_modifiers: input.apply_modifiers,
        triangulate: input.triangulate,
        export_vertex_colors: true,
        export_uvs: true,
        export_normals: input.export_normals,
        export_tangents: input.export_tangents,
        use_mesh_instancing: true,
        merge_vertices_distance: None,
        export_loose_edges: false,
        export_loose_points: false,
    };

    let _ = mesh_options.clone();
    let _ = format!("{:?}", mesh_options);

    // Test MaterialOptions
    let mat_options = MaterialOptions {
        export_materials: input.export_materials,
        export_original_specular: false,
        export_environment_maps: false,
        convert_to_pbr: input.convert_to_pbr,
    };

    let _ = mat_options.clone();
    let _ = format!("{:?}", mat_options);

    // Test GltfExportOptions
    let gltf_options = GltfExportOptions {
        draco_compression: input.draco_compression,
        draco_compression_level: input.draco_level.clamp(0, 10),
        export_extras: true,
        export_lights: false,
        export_cameras: false,
        y_up: input.y_up,
        selected_only: false,
        visible_only: true,
        active_collection_only: false,
        export_armatures: true,
        export_skins: true,
        copyright: None,
    };

    let _ = gltf_options.clone();
    let _ = format!("{:?}", gltf_options);

    // Test ProcessOptions
    let process_options = ProcessOptions {
        timeout: Duration::from_secs(input.timeout_secs.clamp(1, 3600)),
        ..Default::default()
    };

    let _ = process_options.clone();
    let _ = format!("{:?}", process_options);

    // Test CacheOptions
    let cache_options = CacheOptions {
        enabled: input.cache_enabled,
        max_cache_size: Some(input.max_cache_size_mb.clamp(1, 10000) as u64 * 1024 * 1024),
        ..Default::default()
    };

    let _ = cache_options.clone();
    let _ = format!("{:?}", cache_options);

    // Test full ConversionOptions with builder
    let options = ConversionOptions {
        format,
        gltf: gltf_options,
        textures: tex_options,
        animation: anim_options,
        mesh: mesh_options,
        materials: mat_options,
        linked_libraries: LinkedLibraryOptions::default(),
        process: process_options,
        cache: cache_options,
        decomposition: SceneDecompositionOptions::default(),
    };

    let _ = options.clone();
    let _ = format!("{:?}", options);

    // Test preset constructors
    let _ = ConversionOptions::game_runtime();
    let _ = ConversionOptions::editor_preview();
    let _ = ConversionOptions::archival_quality();

    // Test Default implementation
    let _ = ConversionOptions::default();

    // Test serialization roundtrip - RON
    if let Ok(serialized) = ron::to_string(&options) {
        if let Ok(deserialized) = ron::from_str::<ConversionOptions>(&serialized) {
            // Verify key fields survived roundtrip
            assert_eq!(options.format, deserialized.format);
            assert_eq!(
                options.animation.export_animations,
                deserialized.animation.export_animations
            );
        }
    }

    // Test serialization roundtrip - JSON
    if let Ok(serialized) = serde_json::to_string(&options) {
        if let Ok(deserialized) = serde_json::from_str::<ConversionOptions>(&serialized) {
            assert_eq!(options.mesh.triangulate, deserialized.mesh.triangulate);
        }
    }

    // Test equality (reflexivity)
    assert_eq!(format, format);
    assert_eq!(tex_format, tex_format);
});
