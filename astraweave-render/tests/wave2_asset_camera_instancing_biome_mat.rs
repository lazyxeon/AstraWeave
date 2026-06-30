//! Wave 2 proactive remediation — asset_index, camera, instancing, biome_material.
//!
//! Targets the 4 highest-priority coverage gaps identified by the render crate
//! mutation audit:
//!   - asset_index.rs:  323 lines, ZERO external tests → covers all parse/lookup/validate
//!   - camera.rs:       418 lines, 0 external tests → dir math, view/proj matrices
//!   - instancing.rs:   480 lines, gaps in draw-call math → reduction %, get_batch_mut
//!   - biome_material.rs: 336 lines, 1 ext import → material_dir_for all biomes, state machine

use astraweave_render::asset_index::AssetIndex;
use astraweave_render::biome_material::{BiomeMaterialConfig, BiomeMaterialSystem};
use astraweave_camera::{CameraController, CameraMode, FreeFly as Camera};
use astraweave_render::hdri_catalog::DayPeriod;
use astraweave_render::instancing::{
    Instance, InstanceBatch, InstanceManager, InstancePatternBuilder, InstanceRaw,
};
use astraweave_terrain::biome::BiomeType;
use glam::{Mat4, Quat, Vec3};
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════
// asset_index.rs — comprehensive external test coverage
// ═══════════════════════════════════════════════════════════════════════

const FULL_TOML: &str = r#"
[index]
version = 2
generated = "2026-06-15"
asset_root = "game_assets"

[[material_set]]
biome = "forest"
dir = "materials/forest"
layers = 5
description = "Dense temperate forest"

[[material_set]]
biome = "desert"
dir = "materials/desert"
layers = 3
description = "Arid dunes"

[[material_set]]
biome = "tundra"
dir = "materials/tundra"
layers = 4

[[texture]]
name = "grass_01"
dir = "textures/grass"
maps = ["albedo", "normal", "mra"]
has_ktx2 = true
resolution = "2048x2048"

[[texture]]
name = "rock_cliff"
dir = "textures/rock"
maps = ["albedo", "normal"]
has_ktx2 = false
resolution = "1024x1024"

[[hdri]]
name = "forest_day"
file = "hdri/forest/day.hdr"
time = "day"
biomes = ["forest", "grassland"]

[[hdri]]
name = "forest_sunset"
file = "hdri/forest/sunset.hdr"
time = "sunset"
biomes = ["forest"]

[[hdri]]
name = "desert_day"
file = "hdri/desert/day.hdr"
time = "day"
biomes = ["desert"]

[[model]]
name = "pine_tree"
dir = "models/trees"
format = "glb"
source = "polyhaven"
license = "CC0"
note = "LOD0 only"

[[audio_pack]]
name = "ambient_forest"
dir = "audio/forest"
formats = ["ogg", "mp3"]
tracks = 12
source = "custom"
license = "MIT"
description = "Forest ambient loops"
"#;

const MINIMAL_TOML: &str = r#"
[index]
version = 1
generated = "2026-01-01"
asset_root = "assets"
"#;

// ── Parse & IndexMeta ──────────────────────────────────────────────────

#[test]
fn asset_index_parse_full_toml() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert_eq!(idx.index.version, 2);
    assert_eq!(idx.index.generated, "2026-06-15");
    assert_eq!(idx.index.asset_root, "game_assets");
}

#[test]
fn asset_index_material_sets_count() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert_eq!(idx.material_sets.len(), 3);
}

#[test]
fn asset_index_textures_count() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert_eq!(idx.textures.len(), 2);
}

#[test]
fn asset_index_hdris_count() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert_eq!(idx.hdris.len(), 3);
}

#[test]
fn asset_index_models_count() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert_eq!(idx.models.len(), 1);
}

#[test]
fn asset_index_audio_packs_count() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert_eq!(idx.audio_packs.len(), 1);
}

// ── Minimal TOML (empty collections) ───────────────────────────────────

#[test]
fn asset_index_parse_minimal() {
    let idx = AssetIndex::parse_str(MINIMAL_TOML).unwrap();
    assert_eq!(idx.index.version, 1);
    assert!(idx.material_sets.is_empty());
    assert!(idx.textures.is_empty());
    assert!(idx.hdris.is_empty());
    assert!(idx.models.is_empty());
    assert!(idx.audio_packs.is_empty());
}

// ── material_set() lookup (case-insensitive) ────────────────────────

#[test]
fn asset_index_material_set_lowercase() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let ms = idx.material_set("forest").unwrap();
    assert_eq!(ms.biome, "forest");
    assert_eq!(ms.dir, "materials/forest");
    assert_eq!(ms.layers, 5);
    assert_eq!(ms.description, "Dense temperate forest");
}

#[test]
fn asset_index_material_set_uppercase() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert!(idx.material_set("FOREST").is_some());
}

#[test]
fn asset_index_material_set_mixed_case() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert!(idx.material_set("FoReSt").is_some());
}

#[test]
fn asset_index_material_set_desert() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let ms = idx.material_set("desert").unwrap();
    assert_eq!(ms.layers, 3);
    assert_eq!(ms.description, "Arid dunes");
}

#[test]
fn asset_index_material_set_tundra_empty_desc() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let ms = idx.material_set("tundra").unwrap();
    assert_eq!(ms.layers, 4);
    assert_eq!(ms.description, ""); // serde default
}

#[test]
fn asset_index_material_set_not_found() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert!(idx.material_set("volcano").is_none());
}

// ── texture() lookup (case-insensitive) ────────────────────────────

#[test]
fn asset_index_texture_by_name() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let tex = idx.texture("grass_01").unwrap();
    assert_eq!(tex.dir, "textures/grass");
    assert!(tex.has_ktx2);
    assert_eq!(tex.resolution, "2048x2048");
    assert_eq!(tex.maps, vec!["albedo", "normal", "mra"]);
}

#[test]
fn asset_index_texture_case_insensitive() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert!(idx.texture("GRASS_01").is_some());
    assert!(idx.texture("Rock_Cliff").is_some());
}

#[test]
fn asset_index_texture_not_found() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert!(idx.texture("missing_tex").is_none());
}

#[test]
fn asset_index_texture_no_ktx2() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let tex = idx.texture("rock_cliff").unwrap();
    assert!(!tex.has_ktx2);
    assert_eq!(tex.maps.len(), 2);
}

// ── hdri() lookup (case-insensitive) ───────────────────────────────

#[test]
fn asset_index_hdri_by_name() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let h = idx.hdri("forest_day").unwrap();
    assert_eq!(h.file, "hdri/forest/day.hdr");
    assert_eq!(h.time, "day");
    assert_eq!(h.biomes, vec!["forest", "grassland"]);
}

#[test]
fn asset_index_hdri_case_insensitive() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert!(idx.hdri("FOREST_DAY").is_some());
    assert!(idx.hdri("Desert_Day").is_some());
}

#[test]
fn asset_index_hdri_not_found() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert!(idx.hdri("arctic_night").is_none());
}

// ── hdris_for() (biome + time filter) ──────────────────────────────

#[test]
fn asset_index_hdris_for_forest_day() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let matches = idx.hdris_for("forest", "day");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "forest_day");
}

#[test]
fn asset_index_hdris_for_forest_sunset() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let matches = idx.hdris_for("forest", "sunset");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "forest_sunset");
}

#[test]
fn asset_index_hdris_for_grassland_day() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    // "forest_day" has biomes = ["forest", "grassland"]
    let matches = idx.hdris_for("grassland", "day");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "forest_day");
}

#[test]
fn asset_index_hdris_for_desert_day() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let matches = idx.hdris_for("desert", "day");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "desert_day");
}

#[test]
fn asset_index_hdris_for_no_match() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let matches = idx.hdris_for("tundra", "night");
    assert!(matches.is_empty());
}

#[test]
fn asset_index_hdris_for_case_insensitive() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let matches = idx.hdris_for("FOREST", "DAY");
    assert_eq!(matches.len(), 1);
}

// ── material_set_map() ─────────────────────────────────────────────

#[test]
fn asset_index_material_set_map_keys() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let map = idx.material_set_map();
    assert_eq!(map.len(), 3);
    assert!(map.contains_key("forest"));
    assert!(map.contains_key("desert"));
    assert!(map.contains_key("tundra"));
}

#[test]
fn asset_index_material_set_map_values() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let map = idx.material_set_map();
    assert_eq!(map["forest"].layers, 5);
    assert_eq!(map["desert"].layers, 3);
}

#[test]
fn asset_index_material_set_map_empty() {
    let idx = AssetIndex::parse_str(MINIMAL_TOML).unwrap();
    let map = idx.material_set_map();
    assert!(map.is_empty());
}

// ── validate_paths() ───────────────────────────────────────────────

#[test]
fn asset_index_validate_paths_nonexistent_base() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let missing = idx.validate_paths("/nonexistent/base/path");
    // All material_sets + textures + hdris should be missing
    assert!(missing.len() >= 3 + 2 + 3); // 3 material sets, 2 textures, 3 hdris
}

#[test]
fn asset_index_validate_paths_minimal_no_missing() {
    let idx = AssetIndex::parse_str(MINIMAL_TOML).unwrap();
    let missing = idx.validate_paths("/any");
    assert!(missing.is_empty()); // no entries to validate
}

#[test]
fn asset_index_validate_paths_messages_contain_names() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    let missing = idx.validate_paths("/fake");
    // Check that error messages reference the biome/texture/hdri names
    let all_text = missing.join("\n");
    assert!(all_text.contains("forest"), "Should mention forest");
    assert!(
        all_text.contains("grass_01") || all_text.contains("grass"),
        "Should mention grass texture"
    );
    assert!(
        all_text.contains("forest_day") || all_text.contains("hdri"),
        "Should mention HDRI"
    );
}

// ── Invalid TOML parsing ───────────────────────────────────────────

#[test]
fn asset_index_parse_invalid_toml() {
    let result = AssetIndex::parse_str("not valid toml {{{}}}");
    assert!(result.is_err());
}

#[test]
fn asset_index_parse_missing_index_section() {
    let result =
        AssetIndex::parse_str("[[material_set]]\nbiome = \"x\"\ndir = \"y\"\nlayers = 1\n");
    assert!(result.is_err());
}

// ── Model and AudioPack fields ─────────────────────────────────────

#[test]
fn asset_index_model_fields() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert_eq!(idx.models[0].name, "pine_tree");
    assert_eq!(idx.models[0].dir, "models/trees");
    assert_eq!(idx.models[0].format, "glb");
    assert_eq!(idx.models[0].source, "polyhaven");
    assert_eq!(idx.models[0].license, "CC0");
    assert_eq!(idx.models[0].note, "LOD0 only");
}

#[test]
fn asset_index_audio_pack_fields() {
    let idx = AssetIndex::parse_str(FULL_TOML).unwrap();
    assert_eq!(idx.audio_packs[0].name, "ambient_forest");
    assert_eq!(idx.audio_packs[0].dir, "audio/forest");
    assert_eq!(idx.audio_packs[0].formats, vec!["ogg", "mp3"]);
    assert_eq!(idx.audio_packs[0].tracks, 12);
    assert_eq!(idx.audio_packs[0].source, "custom");
    assert_eq!(idx.audio_packs[0].license, "MIT");
    assert_eq!(idx.audio_packs[0].description, "Forest ambient loops");
}

// ── load() with nonexistent file ───────────────────────────────────

#[test]
fn asset_index_load_nonexistent_file() {
    let result = AssetIndex::load("/nonexistent/asset_index.toml");
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════
// camera.rs — external tests for Camera math + CameraController
// ═══════════════════════════════════════════════════════════════════════

fn make_camera() -> Camera {
    Camera {
        position: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        fovy: std::f32::consts::FRAC_PI_3, // 60°
        aspect: 16.0 / 9.0,
        znear: 0.1,
        zfar: 1000.0,
    }
}

// ── Camera::dir() ──────────────────────────────────────────────────

#[test]
fn camera_dir_yaw0_pitch0_is_positive_x() {
    let d = Camera::dir(0.0, 0.0);
    assert!((d.x - 1.0).abs() < 1e-5, "x should be ~1.0, got {}", d.x);
    assert!(d.y.abs() < 1e-5, "y should be ~0.0, got {}", d.y);
    assert!(d.z.abs() < 1e-5, "z should be ~0.0, got {}", d.z);
}

#[test]
fn camera_dir_yaw_pi_over_2() {
    let d = Camera::dir(std::f32::consts::FRAC_PI_2, 0.0);
    assert!(d.x.abs() < 1e-4, "x should be ~0.0, got {}", d.x);
    assert!(d.y.abs() < 1e-4, "y should be ~0.0, got {}", d.y);
    assert!((d.z - 1.0).abs() < 1e-4, "z should be ~1.0, got {}", d.z);
}

#[test]
fn camera_dir_pitch_up() {
    let d = Camera::dir(0.0, std::f32::consts::FRAC_PI_4);
    assert!(d.y > 0.5, "Positive pitch should look up, y={}", d.y);
    assert!((d.length() - 1.0).abs() < 1e-5, "Should be normalized");
}

#[test]
fn camera_dir_pitch_down() {
    let d = Camera::dir(0.0, -std::f32::consts::FRAC_PI_4);
    assert!(d.y < -0.5, "Negative pitch should look down, y={}", d.y);
}

#[test]
fn camera_dir_always_normalized() {
    for yaw_deg in (-180..=180).step_by(30) {
        for pitch_deg in (-80..=80).step_by(20) {
            let yaw = (yaw_deg as f32).to_radians();
            let pitch = (pitch_deg as f32).to_radians();
            let d = Camera::dir(yaw, pitch);
            assert!(
                (d.length() - 1.0).abs() < 1e-4,
                "Not normalized at yaw={yaw_deg} pitch={pitch_deg}: len={}",
                d.length()
            );
        }
    }
}

// ── Camera::view_matrix() ──────────────────────────────────────────

#[test]
fn camera_view_matrix_not_nan() {
    let cam = make_camera();
    let view = cam.view_matrix();
    let flat = view.to_cols_array();
    assert!(flat.iter().all(|v| !v.is_nan()), "View matrix has NaN");
}

#[test]
fn camera_view_matrix_determinant_nonzero() {
    let cam = make_camera();
    let det = cam.view_matrix().determinant();
    assert!(
        det.abs() > 1e-6,
        "View matrix should be invertible, det={det}"
    );
}

#[test]
fn camera_view_matrix_uses_positive_y_up() {
    // The engine uses +Y as the up vector (the prior -Y caused negative clip-space
    // w — a deliberate bug-fix in freefly.rs). Looking straight ahead (+X) with
    // up=+Y, world Y maps to positive view-space Y.
    let cam = make_camera();
    let view = cam.view_matrix();
    let world_up = view.transform_vector3(Vec3::Y);
    // With +Y up convention, world Y should map to positive view Y.
    assert!(world_up.y > 0.0, "Y should map to +view-Y, got {}", world_up.y);
}

// ── Camera::proj_matrix() ──────────────────────────────────────────

#[test]
fn camera_proj_matrix_not_nan() {
    let cam = make_camera();
    let proj = cam.proj_matrix();
    let flat = proj.to_cols_array();
    assert!(flat.iter().all(|v| !v.is_nan()), "Proj matrix has NaN");
}

#[test]
fn camera_proj_matrix_determinant_nonzero() {
    let cam = make_camera();
    let det = cam.proj_matrix().determinant();
    assert!(
        det.abs() > 1e-10,
        "Proj matrix should be invertible, det={det}"
    );
}

#[test]
fn camera_proj_matrix_aspect_clamped() {
    // Aspect ratio 0 shouldn't produce NaN (clamped to 0.01)
    let cam = Camera {
        aspect: 0.0,
        ..make_camera()
    };
    let proj = cam.proj_matrix();
    let flat = proj.to_cols_array();
    assert!(
        flat.iter().all(|v| !v.is_nan()),
        "Zero aspect should not produce NaN"
    );
}

// ── Camera::vp() ───────────────────────────────────────────────────

#[test]
fn camera_vp_is_proj_times_view() {
    let cam = make_camera();
    let vp = cam.vp();
    let expected = cam.proj_matrix() * cam.view_matrix();
    let vp_arr = vp.to_cols_array();
    let exp_arr = expected.to_cols_array();
    for i in 0..16 {
        assert!(
            (vp_arr[i] - exp_arr[i]).abs() < 1e-5,
            "vp[{i}] mismatch: {} vs {}",
            vp_arr[i],
            exp_arr[i]
        );
    }
}

// ── CameraController ───────────────────────────────────────────────

#[test]
fn camera_controller_new_defaults() {
    let cc = CameraController::new(10.0, 0.005);
    assert_eq!(cc.speed, 10.0);
    assert_eq!(cc.sensitivity, 0.005);
    assert!(matches!(cc.mode, CameraMode::FreeFly));
    assert!(!cc.is_dragging());
}

#[test]
fn camera_controller_begin_frame() {
    let mut cc = CameraController::new(5.0, 0.01);
    cc.begin_frame();
    // Should not panic; resets raw_used_this_frame
}

#[test]
fn camera_controller_orbit_distance_default() {
    let cc = CameraController::new(5.0, 0.01);
    assert!((cc.orbit_distance - 5.0).abs() < 1e-5);
}

#[test]
fn camera_controller_zoom_sensitivity_default() {
    let cc = CameraController::new(5.0, 0.01);
    assert!((cc.zoom_sensitivity - 0.1).abs() < 1e-5);
}

#[test]
fn camera_controller_mouse_smooth_default() {
    let cc = CameraController::new(5.0, 0.01);
    assert!((cc.mouse_smooth - 0.15).abs() < 1e-5);
}

#[test]
fn camera_controller_mouse_deadzone_default() {
    let cc = CameraController::new(5.0, 0.01);
    assert!((cc.mouse_deadzone - 0.25).abs() < 1e-5);
}

#[test]
fn camera_controller_orbit_target_default() {
    let cc = CameraController::new(5.0, 0.01);
    assert_eq!(cc.orbit_target, Vec3::ZERO);
}

#[test]
fn camera_controller_toggle_mode_to_orbit() {
    let mut cc = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    cc.toggle_mode(&mut cam);
    assert!(matches!(cc.mode, CameraMode::Orbit));
}

#[test]
fn camera_controller_toggle_mode_back_to_freefly() {
    let mut cc = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    cc.toggle_mode(&mut cam);
    cc.toggle_mode(&mut cam);
    assert!(matches!(cc.mode, CameraMode::FreeFly));
}

#[test]
fn camera_controller_set_orbit_target() {
    let mut cc = CameraController::new(5.0, 0.01);
    let mut cam = make_camera();
    cc.toggle_mode(&mut cam); // Enter orbit mode
    let target = Vec3::new(10.0, 5.0, 3.0);
    cc.set_orbit_target(target, &mut cam);
    assert_eq!(cc.orbit_target, target);
}

// ═══════════════════════════════════════════════════════════════════════
// instancing.rs — gap-filling tests
// ═══════════════════════════════════════════════════════════════════════

// ── InstanceRaw::from_transform with rotation ──────────────────────

#[test]
fn instance_raw_from_transform_rotation_90_y() {
    let raw = InstanceRaw::from_transform(
        Vec3::ZERO,
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        Vec3::ONE,
    );
    // After 90° Y rotation: X axis maps to -Z, Z axis maps to X
    // Column 0 (original X basis) should point in -Z direction
    assert!(raw.model[0][0].abs() < 1e-4, "col0[0] should be ~0");
    assert!(
        (raw.model[0][2] - (-1.0)).abs() < 1e-4,
        "col0[2] should be ~-1"
    );
}

#[test]
fn instance_raw_from_transform_combined() {
    let pos = Vec3::new(5.0, 10.0, 15.0);
    let scale = Vec3::new(2.0, 3.0, 4.0);
    let raw = InstanceRaw::from_transform(pos, Quat::IDENTITY, scale);
    // Translation in column 3
    assert_eq!(raw.model[3][0], 5.0);
    assert_eq!(raw.model[3][1], 10.0);
    assert_eq!(raw.model[3][2], 15.0);
    // Scale on diagonal
    assert!((raw.model[0][0] - 2.0).abs() < 1e-5);
    assert!((raw.model[1][1] - 3.0).abs() < 1e-5);
    assert!((raw.model[2][2] - 4.0).abs() < 1e-5);
}

#[test]
fn instance_raw_from_matrix_preserves_values() {
    let mat = Mat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 4.0, 5.0, 6.0, 1.0,
    ]);
    let raw = InstanceRaw::from_matrix(mat);
    assert_eq!(raw.model[0][0], 1.0);
    assert_eq!(raw.model[1][1], 2.0);
    assert_eq!(raw.model[2][2], 3.0);
    assert_eq!(raw.model[3][0], 4.0);
    assert_eq!(raw.model[3][1], 5.0);
    assert_eq!(raw.model[3][2], 6.0);
}

// ── InstanceRaw::desc() ────────────────────────────────────────────

#[test]
fn instance_raw_desc_stride_64_bytes() {
    let desc = InstanceRaw::desc();
    assert_eq!(desc.array_stride, 64);
}

#[test]
fn instance_raw_desc_step_mode_instance() {
    let desc = InstanceRaw::desc();
    assert_eq!(desc.step_mode, wgpu::VertexStepMode::Instance);
}

#[test]
fn instance_raw_desc_has_4_attributes() {
    let desc = InstanceRaw::desc();
    assert_eq!(desc.attributes.len(), 4);
}

#[test]
fn instance_raw_desc_shader_locations() {
    let desc = InstanceRaw::desc();
    assert_eq!(desc.attributes[0].shader_location, 5);
    assert_eq!(desc.attributes[1].shader_location, 6);
    assert_eq!(desc.attributes[2].shader_location, 7);
    assert_eq!(desc.attributes[3].shader_location, 8);
}

#[test]
fn instance_raw_desc_offsets_sequential() {
    let desc = InstanceRaw::desc();
    assert_eq!(desc.attributes[0].offset, 0);
    assert_eq!(desc.attributes[1].offset, 16); // 4 * f32
    assert_eq!(desc.attributes[2].offset, 32); // 8 * f32
    assert_eq!(desc.attributes[3].offset, 48); // 12 * f32
}

#[test]
fn instance_raw_desc_all_float32x4() {
    let desc = InstanceRaw::desc();
    for attr in desc.attributes {
        assert_eq!(attr.format, wgpu::VertexFormat::Float32x4);
    }
}

// ── Instance ───────────────────────────────────────────────────────

#[test]
fn instance_identity_rotation_is_identity_quat() {
    let inst = Instance::identity();
    assert_eq!(inst.rotation, Quat::IDENTITY);
}

#[test]
fn instance_new_preserves_all_fields() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let rot = Quat::from_rotation_z(1.0);
    let sc = Vec3::new(0.5, 1.5, 2.5);
    let inst = Instance::new(pos, rot, sc);
    assert_eq!(inst.position, pos);
    assert!((inst.rotation.x - rot.x).abs() < 1e-6);
    assert_eq!(inst.scale, sc);
}

// ── InstanceBatch ──────────────────────────────────────────────────

#[test]
fn instance_batch_no_buffer_initially() {
    let batch = InstanceBatch::new(99);
    assert!(batch.buffer.is_none());
}

#[test]
fn instance_batch_mesh_id_preserved() {
    let batch = InstanceBatch::new(42);
    assert_eq!(batch.mesh_id, 42);
}

#[test]
fn instance_batch_instance_count_u32() {
    let mut batch = InstanceBatch::new(1);
    for _ in 0..10 {
        batch.add_instance(Instance::identity());
    }
    assert_eq!(batch.instance_count(), 10u32);
}

#[test]
fn instance_batch_clear_empties_instances() {
    let mut batch = InstanceBatch::new(1);
    batch.add_instance(Instance::identity());
    batch.add_instance(Instance::identity());
    batch.clear();
    assert_eq!(batch.instance_count(), 0);
    assert!(batch.instances.is_empty());
}

// ── InstanceManager — draw call reduction ──────────────────────────

#[test]
fn instance_manager_draw_call_reduction_zero_before_update() {
    // draw_call_reduction_percent returns 0 before update_buffers (no GPU)
    let mut mgr = InstanceManager::new();
    for _ in 0..100 {
        mgr.add_instance(1, Instance::identity());
    }
    // draw_calls_saved is 0 until calculate_draw_call_savings() called via update_buffers
    let reduction = mgr.draw_call_reduction_percent();
    assert_eq!(reduction, 0.0, "Before update, reduction should be 0");
}

#[test]
fn instance_manager_draw_call_saved_single_instance() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(1, Instance::identity());
    let reduction = mgr.draw_call_reduction_percent();
    // 1 instance -> 1 batch -> 0 saved -> 0%
    assert!((reduction - 0.0).abs() < 0.01);
}

#[test]
fn instance_manager_draw_call_saved_multiple_meshes() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(1, Instance::identity());
    mgr.add_instance(2, Instance::identity());
    mgr.add_instance(3, Instance::identity());
    // 3 instances, 3 batches -> 0 saved -> 0%
    let reduction = mgr.draw_call_reduction_percent();
    assert!((reduction - 0.0).abs() < 0.01);
}

#[test]
fn instance_manager_draw_calls_saved_zero_before_update() {
    // draw_calls_saved is 0 until update_buffers called (needs GPU)
    let mut mgr = InstanceManager::new();
    for _ in 0..50 {
        mgr.add_instance(1, Instance::identity());
    }
    for _ in 0..50 {
        mgr.add_instance(2, Instance::identity());
    }
    assert_eq!(mgr.draw_calls_saved(), 0);
    assert_eq!(mgr.total_instances(), 100);
    assert_eq!(mgr.batch_count(), 2);
}

// ── InstanceManager — add_instances bulk ───────────────────────────

#[test]
fn instance_manager_add_instances_bulk_count() {
    let mut mgr = InstanceManager::new();
    let instances = vec![Instance::identity(); 5];
    mgr.add_instances(42, instances);
    assert_eq!(mgr.total_instances(), 5);
    assert_eq!(mgr.batch_count(), 1);
    assert_eq!(mgr.get_batch(42).unwrap().instance_count(), 5);
}

// ── InstanceManager — get_batch_mut ────────────────────────────────

#[test]
fn instance_manager_get_batch_mut_exists() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(7, Instance::identity());
    let batch = mgr.get_batch_mut(7).unwrap();
    batch.add_instance(Instance::identity());
    assert_eq!(mgr.get_batch(7).unwrap().instance_count(), 2);
}

#[test]
fn instance_manager_get_batch_mut_not_found() {
    let mut mgr = InstanceManager::new();
    assert!(mgr.get_batch_mut(999).is_none());
}

// ── InstanceManager — batches() iterator ───────────────────────────

#[test]
fn instance_manager_batches_iterator() {
    let mut mgr = InstanceManager::new();
    mgr.add_instance(1, Instance::identity());
    mgr.add_instance(2, Instance::identity());
    mgr.add_instance(3, Instance::identity());
    let count = mgr.batches().count();
    assert_eq!(count, 3);
}

#[test]
fn instance_manager_batches_iterator_empty() {
    let mgr = InstanceManager::new();
    assert_eq!(mgr.batches().count(), 0);
}

// ── InstanceManager — clear ────────────────────────────────────────

#[test]
fn instance_manager_clear_resets_everything() {
    let mut mgr = InstanceManager::new();
    for _ in 0..10 {
        mgr.add_instance(1, Instance::identity());
    }
    mgr.clear();
    assert_eq!(mgr.total_instances(), 0);
    assert_eq!(mgr.batch_count(), 0);
    assert_eq!(mgr.draw_calls_saved(), 0);
    assert_eq!(mgr.draw_call_reduction_percent(), 0.0);
}

// ── InstancePatternBuilder — grid math ─────────────────────────────

#[test]
fn pattern_grid_0x0_empty() {
    let instances = InstancePatternBuilder::new().grid(0, 0, 1.0).build();
    assert!(instances.is_empty());
}

#[test]
fn pattern_grid_1x1_origin() {
    let instances = InstancePatternBuilder::new().grid(1, 1, 10.0).build();
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].position, Vec3::new(0.0, 0.0, 0.0));
}

#[test]
fn pattern_grid_spacing_correct() {
    let instances = InstancePatternBuilder::new().grid(1, 3, 7.5).build();
    assert_eq!(instances.len(), 3);
    assert_eq!(instances[0].position, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(instances[1].position, Vec3::new(7.5, 0.0, 0.0));
    assert_eq!(instances[2].position, Vec3::new(15.0, 0.0, 0.0));
}

#[test]
fn pattern_grid_all_y_zero() {
    let instances = InstancePatternBuilder::new().grid(5, 5, 2.0).build();
    for inst in &instances {
        assert_eq!(inst.position.y, 0.0);
    }
}

#[test]
fn pattern_grid_all_identity_rotation() {
    let instances = InstancePatternBuilder::new().grid(3, 3, 1.0).build();
    for inst in &instances {
        assert_eq!(inst.rotation, Quat::IDENTITY);
    }
}

#[test]
fn pattern_grid_all_unit_scale() {
    let instances = InstancePatternBuilder::new().grid(3, 3, 1.0).build();
    for inst in &instances {
        assert_eq!(inst.scale, Vec3::ONE);
    }
}

// ── InstancePatternBuilder — circle math ───────────────────────────

#[test]
fn pattern_circle_first_at_radius_x() {
    let instances = InstancePatternBuilder::new().circle(4, 10.0).build();
    // First instance: angle=0 → (cos(0)*10, 0, sin(0)*10) = (10, 0, 0)
    assert!((instances[0].position.x - 10.0).abs() < 1e-4);
    assert!(instances[0].position.z.abs() < 1e-4);
}

#[test]
fn pattern_circle_y_always_zero() {
    let instances = InstancePatternBuilder::new().circle(32, 5.0).build();
    for inst in &instances {
        assert_eq!(inst.position.y, 0.0);
    }
}

#[test]
fn pattern_circle_has_rotation() {
    let instances = InstancePatternBuilder::new().circle(4, 5.0).build();
    // Circle adds rotation: from_rotation_y(angle + PI)
    // First instance: angle=0 → rotation_y(PI)
    let expected = Quat::from_rotation_y(std::f32::consts::PI);
    let r0 = instances[0].rotation;
    assert!(
        (r0.x - expected.x).abs() < 1e-4
            && (r0.y - expected.y).abs() < 1e-4
            && (r0.z - expected.z).abs() < 1e-4
            && (r0.w - expected.w).abs() < 1e-4,
        "First circle instance should have rotation_y(PI)"
    );
}

#[test]
fn pattern_circle_unit_scale() {
    let instances = InstancePatternBuilder::new().circle(8, 5.0).build();
    for inst in &instances {
        assert_eq!(inst.scale, Vec3::ONE);
    }
}

// ── InstancePatternBuilder — variations ────────────────────────────

#[test]
fn pattern_with_position_jitter_bounds() {
    let instances = InstancePatternBuilder::new()
        .grid(1, 1, 0.0) // Single instance at origin
        .with_position_jitter(1.0)
        .build();
    assert_eq!(instances.len(), 1);
    // Jittered position should be within [-1, 1] for x and z
    assert!(instances[0].position.x.abs() <= 1.0);
    assert!(instances[0].position.z.abs() <= 1.0);
    assert_eq!(instances[0].position.y, 0.0); // Y unchanged
}

#[test]
fn pattern_with_scale_variation_range() {
    let instances = InstancePatternBuilder::new()
        .grid(5, 5, 1.0)
        .with_scale_variation(0.5, 2.0)
        .build();
    for inst in &instances {
        assert!(
            inst.scale.x >= 0.5 && inst.scale.x <= 2.0,
            "Scale out of range: {}",
            inst.scale.x
        );
        // Uniform scale (splat)
        assert_eq!(inst.scale.x, inst.scale.y);
        assert_eq!(inst.scale.x, inst.scale.z);
    }
}

#[test]
fn pattern_with_random_rotation_y_valid_quats() {
    let instances = InstancePatternBuilder::new()
        .grid(5, 5, 1.0)
        .with_random_rotation_y()
        .build();
    for inst in &instances {
        let q_len = (inst.rotation.x.powi(2)
            + inst.rotation.y.powi(2)
            + inst.rotation.z.powi(2)
            + inst.rotation.w.powi(2))
        .sqrt();
        assert!(
            (q_len - 1.0).abs() < 1e-4,
            "Quaternion not normalized: len={}",
            q_len
        );
    }
}

#[test]
fn pattern_chain_all_variations() {
    let instances = InstancePatternBuilder::new()
        .grid(3, 3, 2.0)
        .with_position_jitter(0.5)
        .with_scale_variation(0.8, 1.2)
        .with_random_rotation_y()
        .build();
    assert_eq!(instances.len(), 9);
    for inst in &instances {
        assert!(inst.scale.x >= 0.8 && inst.scale.x <= 1.2);
    }
}

// ── InstancePatternBuilder — chaining grid + circle ────────────────

#[test]
fn pattern_grid_then_circle_appends() {
    let instances = InstancePatternBuilder::new()
        .grid(2, 2, 1.0)
        .circle(6, 3.0)
        .build();
    assert_eq!(instances.len(), 4 + 6);
}

// ═══════════════════════════════════════════════════════════════════════
// biome_material.rs — external tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn biome_material_config_default_assets_root() {
    let config = BiomeMaterialConfig::default();
    assert_eq!(config.assets_root, PathBuf::from("assets"));
}

#[test]
fn biome_material_config_default_time_of_day() {
    let config = BiomeMaterialConfig::default();
    assert_eq!(config.time_of_day, DayPeriod::Day);
}

#[test]
fn biome_material_config_default_no_preload() {
    let config = BiomeMaterialConfig::default();
    assert!(!config.preload_adjacent);
}

#[test]
fn biome_material_system_initial_state() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(sys.current_biome().is_none());
    assert_eq!(sys.time_of_day(), DayPeriod::Day);
}

#[test]
fn biome_material_material_dir_for_all_biomes() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    let expected = [
        (BiomeType::Forest, "assets/materials/forest"),
        (BiomeType::Desert, "assets/materials/desert"),
        (BiomeType::Grassland, "assets/materials/grassland"),
        (BiomeType::Mountain, "assets/materials/mountain"),
        (BiomeType::Tundra, "assets/materials/tundra"),
        (BiomeType::Swamp, "assets/materials/swamp"),
        (BiomeType::Beach, "assets/materials/beach"),
        (BiomeType::River, "assets/materials/river"),
    ];
    for (biome, path) in &expected {
        assert_eq!(
            sys.material_dir_for(*biome),
            PathBuf::from(path),
            "material_dir_for({:?}) mismatch",
            biome
        );
    }
}

#[test]
fn biome_material_terrain_fallback_dir() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert_eq!(
        sys.terrain_fallback_dir(),
        PathBuf::from("assets/materials/terrain")
    );
}

#[test]
fn biome_material_needs_transition_initially() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    for biome in BiomeType::all() {
        assert!(
            sys.needs_transition(*biome),
            "Should need transition for {:?}",
            biome
        );
    }
}

#[test]
fn biome_material_needs_transition_after_load() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    sys.mark_loaded(BiomeType::Forest, None);
    assert!(!sys.needs_transition(BiomeType::Forest));
    assert!(sys.needs_transition(BiomeType::Desert));
}

#[test]
fn biome_material_mark_loaded_updates_current_biome() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    sys.mark_loaded(
        BiomeType::Mountain,
        Some(PathBuf::from("hdri/mountain.hdr")),
    );
    assert_eq!(sys.current_biome(), Some(BiomeType::Mountain));
}

#[test]
fn biome_material_mark_loaded_replaces_previous() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    sys.mark_loaded(BiomeType::Forest, None);
    sys.mark_loaded(BiomeType::Desert, None);
    assert_eq!(sys.current_biome(), Some(BiomeType::Desert));
    assert!(!sys.needs_transition(BiomeType::Desert));
    assert!(sys.needs_transition(BiomeType::Forest));
}

#[test]
fn biome_material_set_time_same_returns_false() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(!sys.set_time_of_day(DayPeriod::Day)); // Same as default
}

#[test]
fn biome_material_set_time_different_returns_true() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(sys.set_time_of_day(DayPeriod::Night));
    assert_eq!(sys.time_of_day(), DayPeriod::Night);
}

#[test]
fn biome_material_set_time_updates_persists() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    sys.set_time_of_day(DayPeriod::Evening);
    assert_eq!(sys.time_of_day(), DayPeriod::Evening);
    // Setting to same should now return false
    assert!(!sys.set_time_of_day(DayPeriod::Evening));
}

#[test]
fn biome_material_custom_assets_root() {
    let config = BiomeMaterialConfig {
        assets_root: PathBuf::from("custom/path"),
        ..Default::default()
    };
    let sys = BiomeMaterialSystem::new(config);
    assert_eq!(
        sys.material_dir_for(BiomeType::Forest),
        PathBuf::from("custom/path/materials/forest")
    );
    assert_eq!(
        sys.terrain_fallback_dir(),
        PathBuf::from("custom/path/materials/terrain")
    );
}

#[test]
fn biome_material_validate_dirs_nonexistent_root() {
    let config = BiomeMaterialConfig {
        assets_root: PathBuf::from("/nonexistent/fantasy/path"),
        ..Default::default()
    };
    let sys = BiomeMaterialSystem::new(config);
    let missing = sys.validate_material_dirs();
    // All 8 biomes should be missing
    assert_eq!(
        missing.len(),
        8,
        "All biomes should be missing: {:?}",
        missing
    );
}
