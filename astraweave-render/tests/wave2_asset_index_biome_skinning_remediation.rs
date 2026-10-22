//! Batch 10: AssetIndex TOML parsing + lookups, BiomeMaterialSystem state, SkinnedVertex
//! Mutation-resistant integration tests targeting:
//!   - AssetIndex::parse_str: round-trip from TOML string
//!   - material_set(), texture(), hdri() case-insensitive lookups
//!   - hdris_for() biome+time filtering
//!   - material_set_map() HashMap construction
//!   - BiomeMaterialConfig: default values
//!   - BiomeMaterialSystem: new, current_biome, time_of_day, set_time_of_day, needs_transition
//!   - material_dir_for, terrain_fallback_dir
//!   - SkinnedVertex: size, Pod/Zeroable

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::asset_index::AssetIndex;
use astraweave_render::biome_material::{BiomeMaterialConfig, BiomeMaterialSystem};
#[cfg(feature = "skinning-gpu")]
use astraweave_render::skinning_gpu::SkinnedVertex;
use astraweave_render::DayPeriod;
use astraweave_terrain::biome::BiomeType;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════════════════════════════

/// Minimal valid TOML for AssetIndex
fn sample_toml() -> &'static str {
    r#"
[index]
version = 1
generated = "2025-01-01"
asset_root = "assets"

[[material_set]]
biome = "Forest"
dir = "materials/forest"
layers = 4
description = "Dense forest"

[[material_set]]
biome = "Desert"
dir = "materials/desert"
layers = 3

[[texture]]
name = "rock_albedo"
dir = "textures/rock"
maps = ["albedo.png", "normal.png", "mra.png"]
has_ktx2 = true
resolution = "2048"

[[texture]]
name = "sand_albedo"
dir = "textures/sand"
maps = ["albedo.png"]

[[hdri]]
name = "forest_day"
file = "forest_day.hdr"
time = "day"
biomes = ["Forest"]

[[hdri]]
name = "forest_evening"
file = "forest_evening.hdr"
time = "evening"
biomes = ["Forest", "Swamp"]

[[hdri]]
name = "desert_day"
file = "desert_day.hdr"
time = "day"
biomes = ["Desert"]

[[model]]
name = "tree_oak"
dir = "models/oak"
format = "gltf"

[[audio_pack]]
name = "ambient_forest"
dir = "audio/forest"
formats = ["ogg", "wav"]
tracks = 5
"#
}

fn parse_sample() -> AssetIndex {
    AssetIndex::parse_str(sample_toml()).expect("sample TOML should parse")
}

// ═══════════════════════════════════════════════════════════════════════════════
//  AssetIndex::parse_str
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn asset_index_parse_version() {
    let idx = parse_sample();
    assert_eq!(idx.index.version, 1);
}

#[test]
fn asset_index_parse_generated() {
    let idx = parse_sample();
    assert_eq!(idx.index.generated, "2025-01-01");
}

#[test]
fn asset_index_parse_asset_root() {
    let idx = parse_sample();
    assert_eq!(idx.index.asset_root, "assets");
}

#[test]
fn asset_index_material_set_count() {
    let idx = parse_sample();
    assert_eq!(idx.material_sets.len(), 2);
}

#[test]
fn asset_index_texture_count() {
    let idx = parse_sample();
    assert_eq!(idx.textures.len(), 2);
}

#[test]
fn asset_index_hdri_count() {
    let idx = parse_sample();
    assert_eq!(idx.hdris.len(), 3);
}

#[test]
fn asset_index_model_count() {
    let idx = parse_sample();
    assert_eq!(idx.models.len(), 1);
}

#[test]
fn asset_index_audio_count() {
    let idx = parse_sample();
    assert_eq!(idx.audio_packs.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  material_set() lookup (case-insensitive)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn material_set_exact_case() {
    let idx = parse_sample();
    let ms = idx.material_set("Forest").expect("should find Forest");
    assert_eq!(ms.layers, 4);
    assert_eq!(ms.dir, "materials/forest");
}

#[test]
fn material_set_case_insensitive() {
    let idx = parse_sample();
    assert!(idx.material_set("forest").is_some());
    assert!(idx.material_set("FOREST").is_some());
    assert!(idx.material_set("fOREsT").is_some());
}

#[test]
fn material_set_not_found() {
    let idx = parse_sample();
    assert!(idx.material_set("Ocean").is_none());
}

#[test]
fn material_set_desert() {
    let idx = parse_sample();
    let ms = idx.material_set("Desert").unwrap();
    assert_eq!(ms.layers, 3);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  texture() lookup
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn texture_lookup_exact() {
    let idx = parse_sample();
    let t = idx.texture("rock_albedo").unwrap();
    assert_eq!(t.maps.len(), 3);
    assert!(t.has_ktx2);
}

#[test]
fn texture_lookup_case_insensitive() {
    let idx = parse_sample();
    assert!(idx.texture("Rock_Albedo").is_some());
    assert!(idx.texture("ROCK_ALBEDO").is_some());
}

#[test]
fn texture_not_found() {
    let idx = parse_sample();
    assert!(idx.texture("nonexistent").is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  hdri() lookup
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn hdri_lookup_exact() {
    let idx = parse_sample();
    let h = idx.hdri("forest_day").unwrap();
    assert_eq!(h.file, "forest_day.hdr");
    assert_eq!(h.time, "day");
}

#[test]
fn hdri_lookup_case_insensitive() {
    let idx = parse_sample();
    assert!(idx.hdri("FOREST_DAY").is_some());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  hdris_for() filtering by biome + time
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn hdris_for_forest_day() {
    let idx = parse_sample();
    let matches = idx.hdris_for("Forest", "day");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "forest_day");
}

#[test]
fn hdris_for_forest_evening() {
    let idx = parse_sample();
    let matches = idx.hdris_for("Forest", "evening");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "forest_evening");
}

#[test]
fn hdris_for_swamp_evening() {
    let idx = parse_sample();
    let matches = idx.hdris_for("Swamp", "evening");
    assert_eq!(matches.len(), 1); // forest_evening has biomes including Swamp
}

#[test]
fn hdris_for_desert_day() {
    let idx = parse_sample();
    let matches = idx.hdris_for("Desert", "day");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "desert_day");
}

#[test]
fn hdris_for_no_match() {
    let idx = parse_sample();
    let matches = idx.hdris_for("Tundra", "day");
    assert!(matches.is_empty());
}

#[test]
fn hdris_for_case_insensitive() {
    let idx = parse_sample();
    let matches = idx.hdris_for("forest", "DAY");
    assert_eq!(matches.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  material_set_map()
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn material_set_map_contains_all() {
    let idx = parse_sample();
    let map = idx.material_set_map();
    assert_eq!(map.len(), 2);
    assert!(map.contains_key("Forest"));
    assert!(map.contains_key("Desert"));
}

// ═══════════════════════════════════════════════════════════════════════════════
//  BiomeMaterialConfig default
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn biome_material_config_default_assets_root() {
    let c = BiomeMaterialConfig::default();
    assert_eq!(c.assets_root, PathBuf::from("assets"));
}

#[test]
fn biome_material_config_default_preload_adjacent_false() {
    let c = BiomeMaterialConfig::default();
    assert!(!c.preload_adjacent);
}

#[test]
fn biome_material_config_default_time_of_day() {
    let c = BiomeMaterialConfig::default();
    assert_eq!(c.time_of_day, DayPeriod::Day);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  BiomeMaterialSystem
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn biome_system_initial_no_biome() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(sys.current_biome().is_none());
}

#[test]
fn biome_system_initial_time_of_day() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert_eq!(sys.time_of_day(), DayPeriod::Day);
}

#[test]
fn biome_system_set_time_returns_true_on_change() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(sys.set_time_of_day(DayPeriod::Night));
}

#[test]
fn biome_system_set_time_returns_false_on_same() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(!sys.set_time_of_day(DayPeriod::Day)); // same as default
}

#[test]
fn biome_system_set_time_updates_value() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    sys.set_time_of_day(DayPeriod::Evening);
    assert_eq!(sys.time_of_day(), DayPeriod::Evening);
}

#[test]
fn biome_system_needs_transition_no_biome() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    assert!(sys.needs_transition(BiomeType::Forest));
}

#[test]
fn biome_system_needs_transition_after_load() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    sys.mark_loaded(BiomeType::Forest, None);
    assert!(!sys.needs_transition(BiomeType::Forest)); // same biome
    assert!(sys.needs_transition(BiomeType::Desert)); // different
}

#[test]
fn biome_system_current_biome_after_mark() {
    let mut sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    sys.mark_loaded(BiomeType::Desert, None);
    assert_eq!(sys.current_biome(), Some(BiomeType::Desert));
}

#[test]
fn biome_system_material_dir_for() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    let dir = sys.material_dir_for(BiomeType::Forest);
    assert!(dir.ends_with("materials/forest") || dir.ends_with("materials\\forest"));
}

#[test]
fn biome_system_terrain_fallback_dir() {
    let sys = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    let dir = sys.terrain_fallback_dir();
    assert!(dir.ends_with("materials/terrain") || dir.ends_with("materials\\terrain"));
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SkinnedVertex — requires "skinning-gpu" feature
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "skinning-gpu")]
#[test]
fn skinned_vertex_size() {
    // position[3](12) + normal[3](12) + uv[2](8) + tangent[4](16) + joints[4](16) + weights[4](16) = 80
    let sz = std::mem::size_of::<SkinnedVertex>();
    assert!(sz == 80 || sz == 72, "expected 72 or 80, got {sz}");
}

#[cfg(feature = "skinning-gpu")]
#[test]
fn skinned_vertex_zeroed() {
    let v: SkinnedVertex = bytemuck::Zeroable::zeroed();
    assert_eq!(v.position, [0.0; 3]);
    assert_eq!(v.normal, [0.0; 3]);
    assert_eq!(v.uv, [0.0; 2]);
    assert_eq!(v.joints, [0; 4]);
    assert_eq!(v.weights, [0.0; 4]);
}

#[cfg(feature = "skinning-gpu")]
#[test]
fn skinned_vertex_pod_roundtrip() {
    let v = SkinnedVertex {
        position: [1.0, 2.0, 3.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.5, 0.5],
        tangent: [1.0, 0.0, 0.0, 1.0],
        joints: [0, 1, 2, 3],
        weights: [0.5, 0.3, 0.15, 0.05],
    };
    let bytes = bytemuck::bytes_of(&v);
    let back: &SkinnedVertex = bytemuck::from_bytes(bytes);
    assert_eq!(back.position, v.position);
    assert_eq!(back.joints, v.joints);
    assert_eq!(back.weights, v.weights);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Edge cases for parse_str
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn parse_str_minimal() {
    let toml = r#"
[index]
version = 1
generated = "test"
asset_root = "."
"#;
    let idx = AssetIndex::parse_str(toml).unwrap();
    assert!(idx.material_sets.is_empty());
    assert!(idx.textures.is_empty());
    assert!(idx.hdris.is_empty());
}

#[test]
fn parse_str_invalid_returns_err() {
    let result = AssetIndex::parse_str("this is not valid toml :::");
    assert!(result.is_err());
}
