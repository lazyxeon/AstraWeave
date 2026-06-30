//! Shader Validation Tests
//!
//! Validates all WGSL shaders in the project compile correctly using naga.
//! This catches shader syntax errors, type mismatches, and unsupported features
//! before they cause runtime failures.

use naga::front::wgsl;
use std::path::PathBuf;

/// Get all WGSL shader files in the project
fn get_all_shaders() -> Vec<PathBuf> {
    let mut shaders = Vec::new();

    // Get workspace root (navigate up from astraweave-render/tests)
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = if current_dir.ends_with("astraweave-render") {
        current_dir.parent().unwrap().to_path_buf()
    } else {
        current_dir
    };

    // Change to workspace root for glob patterns
    std::env::set_current_dir(&workspace_root).unwrap();

    // Core rendering shaders
    for path in glob::glob("astraweave-render/shaders/**/*.wgsl")
        .unwrap()
        .flatten()
    {
        shaders.push(path);
    }
    for path in glob::glob("astraweave-render/src/shaders/**/*.wgsl")
        .unwrap()
        .flatten()
    {
        shaders.push(path);
    }

    // Bevy integration shaders
    for path in glob::glob("astraweave-render-bevy/shaders/**/*.wgsl")
        .unwrap()
        .flatten()
    {
        shaders.push(path);
    }
    for path in glob::glob("astraweave-render-bevy/src/shaders/**/*.wgsl")
        .unwrap()
        .flatten()
    {
        shaders.push(path);
    }

    // Editor viewport shaders
    for path in glob::glob("tools/aw_editor/src/viewport/shaders/**/*.wgsl")
        .unwrap()
        .flatten()
    {
        shaders.push(path);
    }

    // Example shaders
    for path in glob::glob("examples/**/src/**/*.wgsl").unwrap().flatten() {
        shaders.push(path);
    }

    shaders
}

/// Shaders that are NOT standalone-compilable by design and are skipped by
/// `test_all_shaders_compile`. Each is a real, in-use shader validated as a
/// concatenated unit at its GPU-pipeline build site (several also have dedicated
/// parse tests); naga cannot parse them in isolation. Keyed by forward-slash-
/// normalized path suffix, matched with `ends_with`. Keep this list minimal — the
/// `validated_count` floor below fails the test if it ever over-skips.
const SHADER_VALIDATION_SKIPS: &[(&str, &str)] = &[
    // modular fragments: rely on constants.wgsl (PI/TWO_PI/HALF_PI/INV_PI) and/or
    // brdf_common.wgsl prepended on the Rust side at runtime.
    ("shaders/brdf_common.wgsl", "modular-fragment: needs constants.wgsl (renderer.rs / terrain_material_manager.rs)"),
    ("shaders/shadow_sampling.wgsl", "modular-fragment: needs constants.wgsl TWO_PI (see shadow_sampling_shader_parses)"),
    ("shaders/vegetation_scatter.wgsl", "modular-fragment: needs constants.wgsl TWO_PI (vegetation_gpu.rs)"),
    ("shaders/ltc_area_lights.wgsl", "modular-fragment: needs constants.wgsl TWO_PI (ltc_area_lights.rs)"),
    ("shaders/gtao.wgsl", "modular-fragment: needs constants.wgsl PI (gtao.rs)"),
    ("shaders/ssgi.wgsl", "modular-fragment: needs constants.wgsl PI (ssgi.rs)"),
    ("shaders/pbr/disney_brdf.wgsl", "modular-fragment: needs constants.wgsl PI (see disney_brdf_wgsl_parses_naga)"),
    ("shaders/pbr/brdf_lut.wgsl", "modular-fragment: needs constants.wgsl PI (brdf_lut.rs)"),
    ("shaders/lumen/final_gather.wgsl", "modular-fragment: needs constants.wgsl INV_PI (final_gather.rs)"),
    ("shaders/lumen/surface_cache_update.wgsl", "modular-fragment: needs constants.wgsl INV_PI/PI (surface_cache.rs)"),
    ("shaders/atmosphere/sky_render.wgsl", "modular-fragment: needs constants.wgsl PI (atmosphere.rs)"),
    ("shaders/atmosphere/aerial_perspective.wgsl", "modular-fragment: needs constants.wgsl PI (atmosphere.rs)"),
    ("shaders/volumetrics/cloud_raymarching.wgsl", "modular-fragment: needs constants.wgsl PI (volumetric_clouds.rs)"),
    ("shaders/volumetrics/scatter.wgsl", "modular-fragment: needs constants.wgsl PI (volumetric_fog.rs)"),
    // subgroup shaders: valid WGSL using `enable subgroups;`, which the naga WGSL
    // frontend does not yet support (wgpu runtime does). Selected at runtime by
    // subgroup_ops.rs capability detection, with non-subgroup fallbacks.
    ("shaders/subgroup/auto_exposure_subgroup.wgsl", "subgroup-validator-limit: naga lacks `enable subgroups;`"),
    ("shaders/subgroup/bitonic_sort_subgroup.wgsl", "subgroup-validator-limit: naga lacks `enable subgroups;`"),
    ("shaders/subgroup/prefix_sum_subgroup.wgsl", "subgroup-validator-limit: naga lacks `enable subgroups;`"),
    // concatenation fragments: paired vertex/fragment halves validated together as
    // TERRAIN_SPLAT_SHADER (terrain_material_manager.rs).
    ("shaders/pbr_terrain.wgsl", "concatenation-fragment: fragment half of TERRAIN_SPLAT_SHADER (pairs with pbr_terrain_vs.wgsl)"),
    ("shaders/pbr_terrain_vs.wgsl", "concatenation-fragment: vertex half of TERRAIN_SPLAT_SHADER (needs VertexOutput from pbr_terrain.wgsl)"),
    // modular shader (clustered/vxgi modules prepended at pipeline build); and the
    // forward-lit terrain shader (see test_pbr_terrain_forward_validates_with_prefix).
    ("shaders/pbr.wgsl", "modular-shader: depends on clustered/vxgi modules prepended at pipeline build"),
    ("shaders/pbr_terrain_forward.wgsl", "concatenation-fragment: see test_pbr_terrain_forward_validates_with_prefix"),
];

#[test]
fn test_all_shaders_compile() {
    let shaders = get_all_shaders();

    assert!(
        !shaders.is_empty(),
        "No shaders found! Check glob patterns."
    );

    println!("📦 Found {} WGSL shader files", shaders.len());

    let mut failures = Vec::new();
    let mut warnings = Vec::new();
    let mut success_count = 0;
    // Counts shaders ACTUALLY parse+validated (not skips) — the non-vacuity guard.
    let mut validated_count = 0;

    for shader_path in &shaders {
        let relative_path = shader_path
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap_or(shader_path);

        let source = match std::fs::read_to_string(shader_path) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!(
                    "❌ {}: Failed to read file: {}",
                    relative_path.display(),
                    e
                ));
                continue;
            }
        };

        // Skip Bevy shaders that use preprocessor directives
        // These need Bevy's shader processor before naga validation
        if source.contains("#import") || source.contains("#define") {
            println!(
                "⏭️  {} (Bevy preprocessor shader - skipped)",
                relative_path.display()
            );
            success_count += 1; // Count as success (will be validated by Bevy)
            continue;
        }

        // Skip shaders that are not standalone-compilable by design (modular
        // fragments needing constants.wgsl/brdf_common.wgsl prepended, subgroup
        // shaders beyond naga's frontend, paired concatenation halves), matched
        // against the explicit, reason-annotated SHADER_VALIDATION_SKIPS manifest.
        let norm_path = relative_path.to_string_lossy().replace('\\', "/");
        if let Some((_, reason)) = SHADER_VALIDATION_SKIPS
            .iter()
            .find(|(suffix, _)| norm_path.ends_with(suffix))
        {
            println!("⏭️  {} ({})", relative_path.display(), reason);
            success_count += 1;
            continue;
        }

        // Parse shader with naga
        match wgsl::parse_str(&source) {
            Ok(module) => {
                // Validate module
                let mut validator = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                );

                match validator.validate(&module) {
                    Ok(_) => {
                        success_count += 1;
                        validated_count += 1;
                        println!("✅ {}", relative_path.display());
                    }
                    Err(e) => {
                        failures.push(format!(
                            "❌ {}: Validation error: {}",
                            relative_path.display(),
                            e
                        ));
                    }
                }
            }
            Err(e) => {
                // Check if it's a warning or fatal error
                let error_str = format!("{}", e);
                if error_str.contains("warning") {
                    warnings.push(format!("⚠️  {}: {}", relative_path.display(), e));
                    success_count += 1;
                } else {
                    failures.push(format!(
                        "❌ {}: Parse error: {}",
                        relative_path.display(),
                        e
                    ));
                }
            }
        }
    }

    println!("\n📊 Shader Validation Summary:");
    println!("   Total shaders: {}", shaders.len());
    println!("   ✅ Passed: {}", success_count);
    println!("   ⚠️  Warnings: {}", warnings.len());
    println!("   ❌ Failed: {}", failures.len());

    // Print warnings
    if !warnings.is_empty() {
        println!("\n⚠️  Warnings:");
        for warning in &warnings {
            println!("   {}", warning);
        }
    }

    // Print failures
    if !failures.is_empty() {
        println!("\n❌ Failures:");
        for failure in &failures {
            println!("   {}", failure);
        }

        panic!(
            "\n💥 {} shader(s) failed validation!\n\
             See errors above for details.",
            failures.len()
        );
    }

    // Non-vacuity guard: the skip manifest must never grow to skip everything.
    // The real standalone-shader count is ~68; assert a floor with margin so an
    // accidental over-skip (or a glob regression) fails loudly instead of
    // silently passing while validating nothing.
    assert!(
        validated_count >= 60,
        "shader validation went vacuous: only {} shaders were actually parse+validated \
         (expected >= 60 real standalone shaders). Check SHADER_VALIDATION_SKIPS / globs.",
        validated_count
    );

    println!(
        "\n🎉 Shader validation passed: {} parse+validated, {} skipped/warned, {} total.",
        validated_count,
        shaders.len() - validated_count,
        shaders.len()
    );
}

#[test]
fn test_shader_features_compatibility() {
    // Verify shaders don't use features unavailable on WebGL2
    // This is a placeholder for future platform-specific validation

    let shaders = get_all_shaders();
    let mut incompatible = Vec::new();

    for shader_path in &shaders {
        let source = std::fs::read_to_string(shader_path).unwrap();

        // Check for features that might not be available everywhere
        if source.contains("atomicAdd") || source.contains("atomicMax") {
            // Atomic operations - verify they're only in compute shaders
            if !source.contains("@compute") {
                incompatible.push(format!(
                    "{}: Uses atomic operations outside compute shader",
                    shader_path.display()
                ));
            }
        }

        // Check for excessive texture bindings (WebGL2 has lower limits)
        let binding_count = source.matches("@binding(").count();
        if binding_count > 16 {
            incompatible.push(format!(
                "{}: Has {} bindings (WebGL2 limit: 16)",
                shader_path.display(),
                binding_count
            ));
        }
    }

    if !incompatible.is_empty() {
        println!("⚠️  Potential compatibility issues:");
        for issue in &incompatible {
            println!("   {}", issue);
        }
    } else {
        println!("✅ No compatibility issues detected");
    }
}

#[test]
fn test_shader_entry_points() {
    // Verify all shaders have proper entry points
    let shaders = get_all_shaders();
    let mut missing_entry_points = Vec::new();

    for shader_path in &shaders {
        let source = std::fs::read_to_string(shader_path).unwrap();

        // Parse to get module
        if let Ok(module) = wgsl::parse_str(&source) {
            if module.entry_points.is_empty() {
                // Some shaders are libraries without entry points (e.g., pbr_lib.wgsl)
                // Only warn if it doesn't look like a library
                if !shader_path.to_string_lossy().contains("lib")
                    && !shader_path.to_string_lossy().contains("functions")
                    && !shader_path.to_string_lossy().contains("types")
                    && !shader_path.to_string_lossy().contains("bindings")
                    && !shader_path.to_string_lossy().contains("utils")
                {
                    missing_entry_points.push(shader_path.display().to_string());
                }
            }
        }
    }

    if !missing_entry_points.is_empty() {
        println!("⚠️  Shaders without entry points (may be libraries):");
        for shader in &missing_entry_points {
            println!("   {}", shader);
        }
    }

    // Informational only, not a failure.
    println!("Entry point check complete");
}

/// Validate the forward-lit terrain shader as its concatenation with
/// `constants.wgsl` + `brdf_common.wgsl`, matching how the pipeline
/// build-step composes the source at runtime. See Phase 1 of the
/// Terrain Material System Campaign.
#[test]
fn test_pbr_terrain_forward_validates_with_prefix() {
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = if current_dir.ends_with("astraweave-render") {
        current_dir.parent().unwrap().to_path_buf()
    } else {
        current_dir
    };

    let shaders_dir = workspace_root.join("astraweave-render").join("shaders");
    let constants = std::fs::read_to_string(shaders_dir.join("constants.wgsl"))
        .expect("read constants.wgsl");
    let brdf_common = std::fs::read_to_string(shaders_dir.join("brdf_common.wgsl"))
        .expect("read brdf_common.wgsl");
    let terrain_forward = std::fs::read_to_string(shaders_dir.join("pbr_terrain_forward.wgsl"))
        .expect("read pbr_terrain_forward.wgsl");

    let concatenated = format!("{}{}{}", constants, brdf_common, terrain_forward);

    let module = wgsl::parse_str(&concatenated)
        .unwrap_or_else(|e| panic!("forward-lit terrain shader failed to parse: {e}"));

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .unwrap_or_else(|e| panic!("forward-lit terrain shader failed to validate: {e}"));
}
