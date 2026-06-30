//! Wave 2 remediation: gpu_memory, transparency, biome_audio, terrain_material, decals
//!
//! Targets pure CPU functions with exact golden values to kill arithmetic mutations.

use astraweave_render::biome_audio::{BiomeAmbientMap, DEFAULT_AMBIENT_CROSSFADE};
use astraweave_render::decals::{Decal, DecalBlendMode, GpuDecal};
use astraweave_render::gpu_memory::{BudgetEvent, CategoryBudget, GpuMemoryBudget, MemoryCategory};
use astraweave_render::terrain_material::{
    TerrainLayerGpu, TerrainMaterialDesc, TerrainMaterialGpu,
};
use astraweave_render::transparency::{BlendMode, TransparencyManager};

use astraweave_terrain::biome::BiomeType;
use glam::{Quat, Vec3};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════════════════
// MemoryCategory
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn memory_category_all_has_8_entries() {
    assert_eq!(MemoryCategory::all().len(), 8);
}

#[test]
fn memory_category_all_contains_expected() {
    let all = MemoryCategory::all();
    assert!(all.contains(&MemoryCategory::Geometry));
    assert!(all.contains(&MemoryCategory::Textures));
    assert!(all.contains(&MemoryCategory::RenderTargets));
    assert!(all.contains(&MemoryCategory::Uniforms));
    assert!(all.contains(&MemoryCategory::Staging));
    assert!(all.contains(&MemoryCategory::Shadows));
    assert!(all.contains(&MemoryCategory::Environment));
    assert!(all.contains(&MemoryCategory::Other));
}

// ═══════════════════════════════════════════════════════════════════════════
// CategoryBudget
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn category_budget_default_soft_limit() {
    let b = CategoryBudget::default();
    assert_eq!(b.soft_limit, 256 * 1024 * 1024); // 256 MB
}

#[test]
fn category_budget_default_hard_limit() {
    let b = CategoryBudget::default();
    assert_eq!(b.hard_limit, 512 * 1024 * 1024); // 512 MB
}

#[test]
fn category_budget_default_current_zero() {
    let b = CategoryBudget::default();
    assert_eq!(b.current, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// GpuMemoryBudget::new()
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn gpu_budget_new_total_usage_zero() {
    let b = GpuMemoryBudget::new();
    assert_eq!(b.total_usage(), 0);
}

#[test]
fn gpu_budget_new_usage_percentage_zero() {
    let b = GpuMemoryBudget::new();
    assert!((b.usage_percentage()).abs() < 1e-6);
}

#[test]
fn gpu_budget_new_snapshot_has_8_categories() {
    let b = GpuMemoryBudget::new();
    assert_eq!(b.snapshot().len(), 8);
}

#[test]
fn gpu_budget_new_all_categories_zero() {
    let b = GpuMemoryBudget::new();
    for &cat in MemoryCategory::all() {
        assert_eq!(b.get_usage(cat), 0, "category {:?} should start at 0", cat);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GpuMemoryBudget::with_total_budget
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn with_total_budget_distributes_evenly() {
    let total = 8 * 1024 * 1024 * 1024_u64; // 8 GB
    let b = GpuMemoryBudget::with_total_budget(total);
    let snap = b.snapshot();

    // Per-category hard limit = total / 8 (except Textures)
    let per_cat = total / 8;
    for (cat, _current, hard_limit) in &snap {
        if *cat != MemoryCategory::Textures {
            assert_eq!(*hard_limit, per_cat, "non-texture cat {:?} hard limit", cat);
        }
    }
}

#[test]
fn with_total_budget_textures_get_40_percent() {
    let total = 10_000_000_u64; // 10 MB
    let b = GpuMemoryBudget::with_total_budget(total);
    let snap = b.snapshot();
    let tex = snap
        .iter()
        .find(|(cat, _, _)| *cat == MemoryCategory::Textures)
        .unwrap();
    // Textures hard limit = total * 0.4
    let expected = (total as f64 * 0.4) as u64;
    assert_eq!(
        tex.2, expected,
        "textures hard limit should be 40% of total"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// try_allocate / deallocate / get_usage
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn try_allocate_increases_category_usage() {
    let b = GpuMemoryBudget::new();
    assert!(b.try_allocate(MemoryCategory::Geometry, 1000));
    assert_eq!(b.get_usage(MemoryCategory::Geometry), 1000);
}

#[test]
fn try_allocate_increases_total_usage() {
    let b = GpuMemoryBudget::new();
    assert!(b.try_allocate(MemoryCategory::Geometry, 500));
    assert!(b.try_allocate(MemoryCategory::Textures, 300));
    assert_eq!(b.total_usage(), 800);
}

#[test]
fn try_allocate_multiple_same_category() {
    let b = GpuMemoryBudget::new();
    assert!(b.try_allocate(MemoryCategory::Staging, 100));
    assert!(b.try_allocate(MemoryCategory::Staging, 200));
    assert_eq!(b.get_usage(MemoryCategory::Staging), 300);
}

#[test]
fn deallocate_decreases_usage() {
    let b = GpuMemoryBudget::new();
    assert!(b.try_allocate(MemoryCategory::Geometry, 1000));
    b.deallocate(MemoryCategory::Geometry, 400);
    assert_eq!(b.get_usage(MemoryCategory::Geometry), 600);
    assert_eq!(b.total_usage(), 600);
}

#[test]
fn deallocate_saturating_to_zero() {
    let b = GpuMemoryBudget::new();
    assert!(b.try_allocate(MemoryCategory::Other, 100));
    b.deallocate(MemoryCategory::Other, 999);
    assert_eq!(b.get_usage(MemoryCategory::Other), 0);
}

#[test]
fn hard_limit_blocks_allocation() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Shadows, 100, 200);
    assert!(b.try_allocate(MemoryCategory::Shadows, 200));
    assert!(!b.try_allocate(MemoryCategory::Shadows, 1));
}

#[test]
fn set_category_budget_changes_limits() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Uniforms, 50, 100);
    // Allocate beyond old soft limit but within new hard limit
    assert!(b.try_allocate(MemoryCategory::Uniforms, 100));
    assert!(!b.try_allocate(MemoryCategory::Uniforms, 1));
}

// ═══════════════════════════════════════════════════════════════════════════
// usage_percentage
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn usage_percentage_correct_value() {
    let b = GpuMemoryBudget::new();
    // Default total budget is 2 GB = 2_147_483_648
    assert!(b.try_allocate(MemoryCategory::Textures, 400 * 1024 * 1024));
    let pct = b.usage_percentage();
    let expected = 400.0 * 1024.0 * 1024.0 / (2.0 * 1024.0 * 1024.0 * 1024.0);
    assert!(
        (pct - expected as f32).abs() < 0.01,
        "pct={pct}, expected≈{expected}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// snapshot
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn snapshot_reflects_allocations() {
    let b = GpuMemoryBudget::new();
    b.try_allocate(MemoryCategory::Textures, 1234);
    b.try_allocate(MemoryCategory::Geometry, 5678);
    let snap = b.snapshot();
    let tex = snap.iter().find(|(c, _, _)| *c == MemoryCategory::Textures);
    let geo = snap.iter().find(|(c, _, _)| *c == MemoryCategory::Geometry);
    assert_eq!(tex.unwrap().1, 1234);
    assert_eq!(geo.unwrap().1, 5678);
}

// ═══════════════════════════════════════════════════════════════════════════
// Callbacks: soft limit, hard limit blocked, memory pressure
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn soft_limit_event_fires() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Textures, 100, 1000);
    let fired = Arc::new(AtomicBool::new(false));
    let f = fired.clone();
    b.on_event(Arc::new(move |e| {
        if matches!(e, BudgetEvent::SoftLimitExceeded { .. }) {
            f.store(true, Ordering::SeqCst);
        }
    }));
    b.try_allocate(MemoryCategory::Textures, 50); // below soft
    assert!(!fired.load(Ordering::SeqCst));
    b.try_allocate(MemoryCategory::Textures, 60); // above soft
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn hard_limit_blocked_event_fires() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Geometry, 50, 100);
    let fired = Arc::new(AtomicBool::new(false));
    let f = fired.clone();
    b.on_event(Arc::new(move |e| {
        if matches!(e, BudgetEvent::HardLimitBlocked { .. }) {
            f.store(true, Ordering::SeqCst);
        }
    }));
    b.try_allocate(MemoryCategory::Geometry, 100); // fills to hard limit
    assert!(
        !fired.load(Ordering::SeqCst),
        "should not fire at exactly hard limit"
    );
    b.try_allocate(MemoryCategory::Geometry, 1); // over hard limit → blocked
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn multiple_callbacks_all_fire() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Staging, 10, 100);
    let c1 = Arc::new(AtomicU32::new(0));
    let c2 = Arc::new(AtomicU32::new(0));
    let c1c = c1.clone();
    let c2c = c2.clone();
    b.on_event(Arc::new(move |_| {
        c1c.fetch_add(1, Ordering::SeqCst);
    }));
    b.on_event(Arc::new(move |_| {
        c2c.fetch_add(1, Ordering::SeqCst);
    }));
    b.try_allocate(MemoryCategory::Staging, 50); // triggers soft limit
    assert!(c1.load(Ordering::SeqCst) > 0);
    assert!(c2.load(Ordering::SeqCst) > 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// TransparencyManager
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn transparency_new_empty() {
    let mgr = TransparencyManager::new();
    assert_eq!(mgr.count(), 0);
}

#[test]
fn transparency_add_instance_count() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::new(1.0, 0.0, 0.0), BlendMode::Alpha);
    mgr.add_instance(1, Vec3::new(0.0, 1.0, 0.0), BlendMode::Additive);
    assert_eq!(mgr.count(), 2);
}

#[test]
fn transparency_clear_resets() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::ZERO, BlendMode::Alpha);
    mgr.clear();
    assert_eq!(mgr.count(), 0);
}

#[test]
fn transparency_sorted_back_to_front() {
    let mut mgr = TransparencyManager::new();
    // Camera at origin, instances along -Z
    mgr.add_instance(10, Vec3::new(0.0, 0.0, -2.0), BlendMode::Alpha); // near
    mgr.add_instance(20, Vec3::new(0.0, 0.0, -10.0), BlendMode::Alpha); // far
    mgr.add_instance(30, Vec3::new(0.0, 0.0, -5.0), BlendMode::Alpha); // mid
    mgr.update(Vec3::ZERO);
    let sorted: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
    // Back-to-front: furthest first
    assert_eq!(sorted, vec![20, 30, 10], "should be back-to-front order");
}

#[test]
fn transparency_sorted_updates_on_camera_move() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(1, Vec3::new(10.0, 0.0, 0.0), BlendMode::Alpha);
    mgr.add_instance(2, Vec3::new(-10.0, 0.0, 0.0), BlendMode::Alpha);

    // Camera at origin: instance 1 and 2 equidistant
    mgr.update(Vec3::ZERO);

    // Move camera to (20, 0, 0): instance 2 is now far, instance 1 is near
    mgr.update(Vec3::new(20.0, 0.0, 0.0));
    let sorted: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
    assert_eq!(
        sorted[0], 2,
        "instance 2 should be furthest from camera(20,0,0)"
    );
    assert_eq!(sorted[1], 1, "instance 1 should be nearest");
}

#[test]
fn transparency_instances_by_blend_mode() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::ZERO, BlendMode::Alpha);
    mgr.add_instance(1, Vec3::ZERO, BlendMode::Additive);
    mgr.add_instance(2, Vec3::ZERO, BlendMode::Alpha);
    mgr.add_instance(3, Vec3::ZERO, BlendMode::Multiplicative);
    mgr.update(Vec3::ZERO);

    assert_eq!(mgr.instances_by_blend_mode(BlendMode::Alpha).count(), 2);
    assert_eq!(mgr.instances_by_blend_mode(BlendMode::Additive).count(), 1);
    assert_eq!(
        mgr.instances_by_blend_mode(BlendMode::Multiplicative)
            .count(),
        1
    );
}

#[test]
fn transparency_camera_distance_calculated() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::new(3.0, 4.0, 0.0), BlendMode::Alpha);
    mgr.update(Vec3::ZERO);
    let inst = mgr.sorted_instances().next().unwrap();
    // distance should be 5.0 (3-4-5 triangle)
    assert!(
        (inst.camera_distance - 5.0).abs() < 1e-4,
        "dist={}",
        inst.camera_distance
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// create_blend_state golden values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn blend_state_alpha_src_factor() {
    let state = astraweave_render::transparency::create_blend_state(BlendMode::Alpha);
    assert_eq!(state.color.src_factor, wgpu::BlendFactor::SrcAlpha);
    assert_eq!(state.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
    assert_eq!(state.color.operation, wgpu::BlendOperation::Add);
}

#[test]
fn blend_state_additive() {
    let state = astraweave_render::transparency::create_blend_state(BlendMode::Additive);
    assert_eq!(state.color.src_factor, wgpu::BlendFactor::SrcAlpha);
    assert_eq!(state.color.dst_factor, wgpu::BlendFactor::One);
}

#[test]
fn blend_state_multiplicative() {
    let state = astraweave_render::transparency::create_blend_state(BlendMode::Multiplicative);
    assert_eq!(state.color.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(state.color.dst_factor, wgpu::BlendFactor::Src);
}

// ═══════════════════════════════════════════════════════════════════════════
// BiomeAmbientMap
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn biome_audio_default_crossfade_constant() {
    assert!((DEFAULT_AMBIENT_CROSSFADE - 3.0).abs() < 1e-6);
}

#[test]
fn biome_audio_new_has_8_tracks() {
    let map = BiomeAmbientMap::new();
    assert_eq!(map.len(), 8);
    assert!(!map.is_empty());
}

#[test]
fn biome_audio_empty_has_none() {
    let map = BiomeAmbientMap::empty();
    assert_eq!(map.len(), 0);
    assert!(map.is_empty());
}

#[test]
fn biome_audio_default_paths_pattern() {
    let map = BiomeAmbientMap::new();
    for biome in BiomeType::all() {
        let path = map.get(*biome).expect(&format!("missing {:?}", biome));
        assert!(
            path.starts_with("assets/audio/ambient/"),
            "bad path: {path}"
        );
        assert!(path.ends_with(".ogg"), "bad ext: {path}");
    }
}

#[test]
fn biome_audio_forest_path_golden() {
    let map = BiomeAmbientMap::new();
    assert_eq!(
        map.get(BiomeType::Forest).unwrap(),
        "assets/audio/ambient/forest.ogg"
    );
}

#[test]
fn biome_audio_desert_path_golden() {
    let map = BiomeAmbientMap::new();
    assert_eq!(
        map.get(BiomeType::Desert).unwrap(),
        "assets/audio/ambient/desert.ogg"
    );
}

#[test]
fn biome_audio_set_overrides() {
    let mut map = BiomeAmbientMap::new();
    map.set(BiomeType::Swamp, "custom/swamp_night.ogg");
    assert_eq!(map.get(BiomeType::Swamp).unwrap(), "custom/swamp_night.ogg");
}

#[test]
fn biome_audio_remove_makes_none() {
    let mut map = BiomeAmbientMap::new();
    map.remove(BiomeType::Beach);
    assert!(map.get(BiomeType::Beach).is_none());
    assert_eq!(map.len(), 7);
}

#[test]
fn biome_audio_crossfade_default() {
    let map = BiomeAmbientMap::new();
    assert!((map.crossfade_sec() - 3.0).abs() < 1e-6);
}

#[test]
fn biome_audio_set_crossfade() {
    let mut map = BiomeAmbientMap::new();
    map.set_crossfade_sec(5.0);
    assert!((map.crossfade_sec() - 5.0).abs() < 1e-6);
}

#[test]
fn biome_audio_crossfade_clamps_negative() {
    let mut map = BiomeAmbientMap::new();
    map.set_crossfade_sec(-10.0);
    assert!(map.crossfade_sec() > 0.0, "should clamp to min 0.01");
    assert!((map.crossfade_sec() - 0.01).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════
// TerrainLayerGpu
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn terrain_layer_gpu_size_64() {
    assert_eq!(std::mem::size_of::<TerrainLayerGpu>(), 64);
}

#[test]
fn terrain_layer_gpu_align_16() {
    assert_eq!(std::mem::align_of::<TerrainLayerGpu>(), 16);
}

#[test]
fn terrain_layer_gpu_default_values() {
    let l = TerrainLayerGpu::default();
    assert_eq!(l.texture_indices, [0, 0, 0, 0]);
    assert_eq!(l.uv_scale, [1.0, 1.0]);
    assert_eq!(l.height_range, [0.0, 100.0]);
    assert!((l.blend_sharpness - 0.5).abs() < 1e-6);
    assert!((l.triplanar_power - 4.0).abs() < 1e-6);
    assert_eq!(l.material_factors, [0.0, 0.5]); // metallic=0, roughness=0.5
}

// ═══════════════════════════════════════════════════════════════════════════
// TerrainMaterialGpu
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn terrain_material_gpu_size_2112() {
    assert_eq!(std::mem::size_of::<TerrainMaterialGpu>(), 2112);
}

#[test]
fn terrain_material_gpu_default_values() {
    let m = TerrainMaterialGpu::default();
    assert_eq!(m.splat_map_index_0, 0);
    assert!((m.splat_uv_scale - 1.0).abs() < 1e-6);
    assert_eq!(m.triplanar_enabled, 1);
    assert_eq!(m.normal_blend_method, 1); // RNM
    assert!((m.triplanar_slope_threshold - 45.0).abs() < 1e-6);
    assert_eq!(m.height_blend_enabled, 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// TerrainMaterialDesc defaults & factories
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn terrain_material_desc_default() {
    let d = TerrainMaterialDesc::default();
    assert!(d.name.is_empty());
    assert!(d.biome.is_empty());
    assert!(d.splat_map.is_none());
    assert!((d.splat_uv_scale - 1.0).abs() < 1e-6);
    assert!(d.triplanar_enabled);
    assert!((d.triplanar_slope_threshold - 45.0).abs() < 1e-6);
    assert_eq!(d.normal_blend_method, "rnm");
    assert!(d.height_blend_enabled);
    assert!(d.layers.is_empty());
}

#[test]
fn grassland_factory_golden() {
    let g = TerrainMaterialDesc::grassland();
    assert_eq!(g.name, "grassland_terrain");
    assert_eq!(g.biome, "grassland");
    assert_eq!(g.layers.len(), 4);
    assert_eq!(g.layers[0].name, "grass");
    assert_eq!(g.layers[1].name, "dirt");
    assert_eq!(g.layers[2].name, "rock");
    assert_eq!(g.layers[3].name, "sparse_grass");
    assert!((g.splat_uv_scale - 0.5).abs() < 1e-6);
    assert!((g.triplanar_slope_threshold - 35.0).abs() < 1e-6);
    // First layer grass: uv_scale=[8,8], roughness=0.9, blend_sharpness=0.6
    assert_eq!(g.layers[0].uv_scale, [8.0, 8.0]);
    assert!((g.layers[0].roughness - 0.9).abs() < 1e-6);
    assert!((g.layers[0].blend_sharpness - 0.6).abs() < 1e-6);
}

#[test]
fn desert_factory_golden() {
    let d = TerrainMaterialDesc::desert();
    assert_eq!(d.name, "desert_terrain");
    assert_eq!(d.biome, "desert");
    assert_eq!(d.layers.len(), 4);
    assert_eq!(d.layers[0].name, "sand");
    assert_eq!(d.layers[1].name, "red_sand");
    assert_eq!(d.layers[2].name, "desert_rock");
    assert_eq!(d.layers[3].name, "cracked_ground");
    assert!((d.splat_uv_scale - 0.4).abs() < 1e-6);
    assert!((d.triplanar_slope_threshold - 40.0).abs() < 1e-6);
    assert_eq!(d.layers[0].uv_scale, [12.0, 12.0]);
    assert!((d.layers[0].roughness - 0.95).abs() < 1e-6);
}

#[test]
fn forest_factory_golden() {
    let f = TerrainMaterialDesc::forest();
    assert_eq!(f.name, "forest_terrain");
    assert_eq!(f.biome, "forest");
    assert_eq!(f.layers.len(), 4);
    assert_eq!(f.layers[0].name, "moss");
    assert_eq!(f.layers[3].name, "leaf_litter");
    assert!((f.splat_uv_scale - 0.6).abs() < 1e-6);
    assert!((f.triplanar_slope_threshold - 30.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════
// normal_blend_to_gpu
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn normal_blend_linear_is_0() {
    let d = TerrainMaterialDesc {
        normal_blend_method: "linear".into(),
        ..Default::default()
    };
    assert_eq!(d.normal_blend_to_gpu(), 0);
}

#[test]
fn normal_blend_rnm_is_1() {
    let d = TerrainMaterialDesc {
        normal_blend_method: "rnm".into(),
        ..Default::default()
    };
    assert_eq!(d.normal_blend_to_gpu(), 1);
}

#[test]
fn normal_blend_udn_is_2() {
    let d = TerrainMaterialDesc {
        normal_blend_method: "udn".into(),
        ..Default::default()
    };
    assert_eq!(d.normal_blend_to_gpu(), 2);
}

#[test]
fn normal_blend_case_insensitive() {
    let d = TerrainMaterialDesc {
        normal_blend_method: "LINEAR".into(),
        ..Default::default()
    };
    assert_eq!(d.normal_blend_to_gpu(), 0);
    let d2 = TerrainMaterialDesc {
        normal_blend_method: "UDN".into(),
        ..Default::default()
    };
    assert_eq!(d2.normal_blend_to_gpu(), 2);
}

#[test]
fn normal_blend_invalid_fallback_rnm() {
    let d = TerrainMaterialDesc {
        normal_blend_method: "garbage".into(),
        ..Default::default()
    };
    assert_eq!(d.normal_blend_to_gpu(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// to_gpu conversion
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn to_gpu_grassland_properties() {
    let desc = TerrainMaterialDesc::grassland();
    let resolver = |_: &std::path::PathBuf| -> u32 { 42 };
    let gpu = desc.to_gpu(&resolver);
    assert!((gpu.splat_uv_scale - 0.5).abs() < 1e-6);
    assert_eq!(gpu.triplanar_enabled, 1);
    assert_eq!(gpu.normal_blend_method, 1); // rnm
    assert!((gpu.triplanar_slope_threshold - 35.0).abs() < 1e-6);
    assert_eq!(gpu.height_blend_enabled, 1);
}

#[test]
fn to_gpu_layer_transfer() {
    let desc = TerrainMaterialDesc::grassland();
    let counter = std::sync::atomic::AtomicU32::new(0);
    let resolver = |_: &std::path::PathBuf| -> u32 { counter.fetch_add(1, Ordering::SeqCst) };
    let gpu = desc.to_gpu(&resolver);
    // First layer (grass) has albedo, normal, orm, height → 4 texture indices
    // They should be sequential: 0, 1, 2, 3 (then splat=4, or was it called first?)
    // Actually splat_map is resolved first: index 0, then layers
    assert_eq!(gpu.layers[0].uv_scale, [8.0, 8.0]);
    assert!((gpu.layers[0].blend_sharpness - 0.6).abs() < 1e-6);
    assert!((gpu.layers[0].triplanar_power - 3.0).abs() < 1e-6);
    assert_eq!(gpu.layers[0].material_factors, [0.0, 0.9]); // metallic=0, roughness=0.9
}

#[test]
fn to_gpu_triplanar_disabled() {
    let desc = TerrainMaterialDesc {
        triplanar_enabled: false,
        ..Default::default()
    };
    let resolver = |_: &std::path::PathBuf| -> u32 { 0 };
    let gpu = desc.to_gpu(&resolver);
    assert_eq!(gpu.triplanar_enabled, 0);
}

#[test]
fn to_gpu_height_blend_disabled() {
    let desc = TerrainMaterialDesc {
        height_blend_enabled: false,
        ..Default::default()
    };
    let resolver = |_: &std::path::PathBuf| -> u32 { 0 };
    let gpu = desc.to_gpu(&resolver);
    assert_eq!(gpu.height_blend_enabled, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Decal
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn gpu_decal_size() {
    assert_eq!(std::mem::size_of::<GpuDecal>(), 112);
}

#[test]
fn decal_new_defaults() {
    let d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    assert_eq!(d.albedo_tint, [1.0, 1.0, 1.0, 1.0]);
    assert!((d.normal_strength - 1.0).abs() < 1e-6);
    assert!((d.roughness - 0.5).abs() < 1e-6);
    assert!((d.metallic).abs() < 1e-6);
    assert_eq!(d.blend_mode, DecalBlendMode::AlphaBlend);
    assert!((d.fade_duration).abs() < 1e-6);
    assert!((d.fade_time).abs() < 1e-6);
}

#[test]
fn decal_update_permanent_stays_alive() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    // fade_duration = 0 means permanent
    assert!(d.update(100.0), "permanent decal should always return true");
}

#[test]
fn decal_update_fade_progression() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    d.fade_duration = 2.0;
    assert!(d.update(0.5)); // 0.5s into 2.0s fade → alive
                            // alpha should be 1.0 - (0.5/2.0) = 0.75
    assert!(
        (d.albedo_tint[3] - 0.75).abs() < 1e-4,
        "alpha={}",
        d.albedo_tint[3]
    );
}

#[test]
fn decal_update_fade_complete() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    d.fade_duration = 1.0;
    assert!(d.update(0.5)); // alive at 0.5s
    assert!(!d.update(0.6)); // dead at 1.1s ≥ 1.0s
}

#[test]
fn decal_update_fade_alpha_at_half() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    d.fade_duration = 4.0;
    d.update(2.0); // exactly halfway
                   // alpha = 1.0 - (2.0/4.0) = 0.5
    assert!((d.albedo_tint[3] - 0.5).abs() < 1e-4);
}

#[test]
fn decal_to_gpu_params() {
    let mut d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    d.normal_strength = 0.8;
    d.roughness = 0.3;
    d.metallic = 0.1;
    d.blend_mode = DecalBlendMode::Additive;
    let gpu = d.to_gpu();
    assert!((gpu.params[0] - 0.8).abs() < 1e-6, "normal_strength");
    assert!((gpu.params[1] - 0.3).abs() < 1e-6, "roughness");
    assert!((gpu.params[2] - 0.1).abs() < 1e-6, "metallic");
    assert!(
        (gpu.params[3] - 1.0).abs() < 1e-6,
        "blend_mode (Additive=1)"
    );
}

#[test]
fn decal_to_gpu_atlas_uv() {
    let d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.25, 0.5], [0.125, 0.125]),
    );
    let gpu = d.to_gpu();
    assert!((gpu.atlas_uv[0] - 0.25).abs() < 1e-6);
    assert!((gpu.atlas_uv[1] - 0.5).abs() < 1e-6);
    assert!((gpu.atlas_uv[2] - 0.125).abs() < 1e-6);
    assert!((gpu.atlas_uv[3] - 0.125).abs() < 1e-6);
}

#[test]
fn decal_to_gpu_identity_transform_inv() {
    let d = Decal::new(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        ([0.0, 0.0], [1.0, 1.0]),
    );
    let gpu = d.to_gpu();
    // Identity transform → inverse should also be ≈ identity
    // Check diagonal elements [col][row] — column-major
    assert!((gpu.inv_projection[0][0] - 1.0).abs() < 1e-4);
    assert!((gpu.inv_projection[1][1] - 1.0).abs() < 1e-4);
    assert!((gpu.inv_projection[2][2] - 1.0).abs() < 1e-4);
    assert!((gpu.inv_projection[3][3] - 1.0).abs() < 1e-4);
}

#[test]
fn decal_blend_mode_enum_values() {
    assert_eq!(DecalBlendMode::Multiply as u32, 0);
    assert_eq!(DecalBlendMode::Additive as u32, 1);
    assert_eq!(DecalBlendMode::AlphaBlend as u32, 2);
    assert_eq!(DecalBlendMode::Stain as u32, 3);
}
