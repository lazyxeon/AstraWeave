//! Batch 6: GPU memory budget, transparency sorting, water structs, MSAA config
//! Mutation-resistant integration tests targeting:
//!   - GpuMemoryBudget (with_total_budget distribution, allocation/deallocation, pressure, snapshot)
//!   - MemoryCategory (all 8 variants, exhaustiveness)
//!   - CategoryBudget (default soft/hard limits)
//!   - TransparencyManager (sorting, blend mode filter, distance calculation)
//!   - BlendMode enum (3 variants)
//!   - create_blend_state (blend factor verification for each mode)
//!   - WaterUniforms (default values, size, Pod/Zeroable)
//!   - WaterVertex (size, desc layout)
//!   - MsaaMode (sample_count, is_enabled, multisample_state, default)

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use glam::Vec3;

use astraweave_render::gpu_memory::{BudgetEvent, CategoryBudget, GpuMemoryBudget, MemoryCategory};
use astraweave_render::msaa::MsaaMode;
use astraweave_render::transparency::{create_blend_state, BlendMode, TransparencyManager};
use astraweave_render::water::{WaterUniforms, WaterVertex};

// ═══════════════════════════════════════════════════════════════════════════════
//  MemoryCategory
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn memory_category_all_returns_8() {
    assert_eq!(MemoryCategory::all().len(), 8);
}

#[test]
fn memory_category_all_contains_all_variants() {
    let cats = MemoryCategory::all();
    assert!(cats.contains(&MemoryCategory::Geometry));
    assert!(cats.contains(&MemoryCategory::Textures));
    assert!(cats.contains(&MemoryCategory::RenderTargets));
    assert!(cats.contains(&MemoryCategory::Uniforms));
    assert!(cats.contains(&MemoryCategory::Staging));
    assert!(cats.contains(&MemoryCategory::Shadows));
    assert!(cats.contains(&MemoryCategory::Environment));
    assert!(cats.contains(&MemoryCategory::Other));
}

#[test]
fn memory_category_all_no_duplicates() {
    let cats = MemoryCategory::all();
    let unique: std::collections::HashSet<_> = cats.iter().collect();
    assert_eq!(unique.len(), cats.len(), "all() should have no duplicates");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  CategoryBudget
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn category_budget_default_soft_limit() {
    let b = CategoryBudget::default();
    assert_eq!(b.soft_limit, 256 * 1024 * 1024);
}

#[test]
fn category_budget_default_hard_limit() {
    let b = CategoryBudget::default();
    assert_eq!(b.hard_limit, 512 * 1024 * 1024);
}

#[test]
fn category_budget_default_current_zero() {
    let b = CategoryBudget::default();
    assert_eq!(b.current, 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuMemoryBudget — new / default
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn budget_new_total_usage_zero() {
    let b = GpuMemoryBudget::new();
    assert_eq!(b.total_usage(), 0);
}

#[test]
fn budget_new_usage_percentage_zero() {
    let b = GpuMemoryBudget::new();
    assert!((b.usage_percentage() - 0.0).abs() < 1e-6);
}

#[test]
fn budget_new_snapshot_has_all_categories() {
    let b = GpuMemoryBudget::new();
    let snap = b.snapshot();
    assert_eq!(snap.len(), 8, "snapshot should have 8 categories");
}

#[test]
fn budget_new_all_categories_current_zero() {
    let b = GpuMemoryBudget::new();
    for &cat in MemoryCategory::all() {
        assert_eq!(b.get_usage(cat), 0, "category {:?} should start at 0", cat);
    }
}

#[test]
fn budget_default_same_as_new() {
    let d = GpuMemoryBudget::default();
    let n = GpuMemoryBudget::new();
    assert_eq!(d.total_usage(), n.total_usage());
    assert!((d.usage_percentage() - n.usage_percentage()).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuMemoryBudget — with_total_budget distribution
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn budget_with_total_1gb_textures_get_30pct_soft_40pct_hard() {
    let total: u64 = 1024 * 1024 * 1024; // 1 GB
    let b = GpuMemoryBudget::with_total_budget(total);
    let snap = b.snapshot();
    let tex = snap
        .iter()
        .find(|(c, _, _)| *c == MemoryCategory::Textures)
        .unwrap();
    // Texture soft = total * 0.3, hard = total * 0.4
    let expected_hard = (total as f64 * 0.4) as u64;
    assert_eq!(
        tex.2, expected_hard,
        "textures hard limit should be 40% of total"
    );
}

#[test]
fn budget_with_total_other_categories_get_per_cat() {
    let total: u64 = 800_000_000; // 800 MB
    let per_cat = total / 8;
    let b = GpuMemoryBudget::with_total_budget(total);
    let snap = b.snapshot();
    // Non-texture categories should have hard_limit = per_category
    for (cat, _current, hard_limit) in &snap {
        if *cat != MemoryCategory::Textures {
            assert_eq!(
                *hard_limit, per_cat,
                "category {:?} hard limit should be total/8 = {}, got {}",
                cat, per_cat, hard_limit
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuMemoryBudget — allocate / deallocate
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn budget_allocate_increases_usage() {
    let b = GpuMemoryBudget::new();
    assert!(b.try_allocate(MemoryCategory::Geometry, 1000));
    assert_eq!(b.get_usage(MemoryCategory::Geometry), 1000);
    assert_eq!(b.total_usage(), 1000);
}

#[test]
fn budget_allocate_multiple_categories() {
    let b = GpuMemoryBudget::new();
    b.try_allocate(MemoryCategory::Geometry, 500);
    b.try_allocate(MemoryCategory::Textures, 700);
    assert_eq!(b.get_usage(MemoryCategory::Geometry), 500);
    assert_eq!(b.get_usage(MemoryCategory::Textures), 700);
    assert_eq!(b.total_usage(), 1200);
}

#[test]
fn budget_deallocate_decreases_usage() {
    let b = GpuMemoryBudget::new();
    b.try_allocate(MemoryCategory::Staging, 2000);
    b.deallocate(MemoryCategory::Staging, 800);
    assert_eq!(b.get_usage(MemoryCategory::Staging), 1200);
    assert_eq!(b.total_usage(), 1200);
}

#[test]
fn budget_deallocate_saturating_sub_no_underflow() {
    let b = GpuMemoryBudget::new();
    b.try_allocate(MemoryCategory::Shadows, 100);
    // Deallocate more than allocated — should saturate to 0
    b.deallocate(MemoryCategory::Shadows, 200);
    assert_eq!(b.get_usage(MemoryCategory::Shadows), 0);
}

#[test]
fn budget_hard_limit_blocks_allocation() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Geometry, 100, 200);
    assert!(b.try_allocate(MemoryCategory::Geometry, 200)); // at limit
    assert!(!b.try_allocate(MemoryCategory::Geometry, 1)); // over limit
}

#[test]
fn budget_hard_limit_exactly_at_boundary_passes() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Uniforms, 50, 100);
    assert!(b.try_allocate(MemoryCategory::Uniforms, 100)); // exactly at hard limit
    assert_eq!(b.get_usage(MemoryCategory::Uniforms), 100);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuMemoryBudget — usage_percentage
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn budget_usage_percentage_after_allocation() {
    let b = GpuMemoryBudget::with_total_budget(1000);
    b.set_category_budget(MemoryCategory::Geometry, 500, 500);
    b.try_allocate(MemoryCategory::Geometry, 250);
    let pct = b.usage_percentage();
    // 250 / 1000 = 0.25
    assert!((pct - 0.25).abs() < 0.01, "expected ~0.25, got {}", pct);
}

#[test]
fn budget_usage_percentage_multiple_categories() {
    let total: u64 = 10_000;
    let b = GpuMemoryBudget::with_total_budget(total);
    // Set generous limits so allocations succeed
    b.set_category_budget(MemoryCategory::Geometry, 5000, 5000);
    b.set_category_budget(MemoryCategory::Textures, 5000, 5000);
    b.try_allocate(MemoryCategory::Geometry, 3000);
    b.try_allocate(MemoryCategory::Textures, 2000);
    let pct = b.usage_percentage();
    // 5000 / 10000 = 0.5
    assert!((pct - 0.5).abs() < 0.01, "expected ~0.5, got {}", pct);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuMemoryBudget — callbacks
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn budget_soft_limit_fires_callback() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Textures, 100, 500);
    let fired = Arc::new(AtomicBool::new(false));
    let fired_clone = fired.clone();
    b.on_event(Arc::new(move |ev| {
        if matches!(ev, BudgetEvent::SoftLimitExceeded { .. }) {
            fired_clone.store(true, Ordering::SeqCst);
        }
    }));
    // Under soft → no callback
    b.try_allocate(MemoryCategory::Textures, 50);
    assert!(!fired.load(Ordering::SeqCst));
    // Over soft → callback
    b.try_allocate(MemoryCategory::Textures, 100);
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn budget_hard_limit_fires_blocked_callback() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Geometry, 50, 100);
    let blocked = Arc::new(AtomicBool::new(false));
    let blocked_clone = blocked.clone();
    b.on_event(Arc::new(move |ev| {
        if matches!(ev, BudgetEvent::HardLimitBlocked { .. }) {
            blocked_clone.store(true, Ordering::SeqCst);
        }
    }));
    b.try_allocate(MemoryCategory::Geometry, 100); // fill to hard limit
    b.try_allocate(MemoryCategory::Geometry, 1); // should fire blocked
    assert!(blocked.load(Ordering::SeqCst));
}

#[test]
fn budget_pressure_event_fires_above_threshold() {
    // Use with_total_budget(1000), pressure threshold = 0.85
    // Need to allocate >850/1000 to trigger
    let b = GpuMemoryBudget::with_total_budget(1000);
    // Set generous limits so we can allocate enough
    b.set_category_budget(MemoryCategory::Geometry, 900, 1000);
    let pressure_fired = Arc::new(AtomicBool::new(false));
    let pf = pressure_fired.clone();
    b.on_event(Arc::new(move |ev| {
        if matches!(ev, BudgetEvent::MemoryPressure { .. }) {
            pf.store(true, Ordering::SeqCst);
        }
    }));
    // At 800/1000 = 80% → below 85%
    b.try_allocate(MemoryCategory::Geometry, 800);
    assert!(!pressure_fired.load(Ordering::SeqCst));
    // At 900/1000 = 90% → above 85%
    b.try_allocate(MemoryCategory::Geometry, 100);
    assert!(pressure_fired.load(Ordering::SeqCst));
}

#[test]
fn budget_multiple_callbacks_all_fire() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Textures, 50, 200);
    let count = Arc::new(AtomicU32::new(0));
    let c1 = count.clone();
    let c2 = count.clone();
    b.on_event(Arc::new(move |_| {
        c1.fetch_add(1, Ordering::SeqCst);
    }));
    b.on_event(Arc::new(move |_| {
        c2.fetch_add(1, Ordering::SeqCst);
    }));
    // Trigger soft limit
    b.try_allocate(MemoryCategory::Textures, 100);
    // Both callbacks should have fired (at least soft limit event)
    assert!(
        count.load(Ordering::SeqCst) >= 2,
        "both callbacks should fire"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuMemoryBudget — snapshot
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn budget_snapshot_reflects_allocations() {
    let b = GpuMemoryBudget::new();
    b.try_allocate(MemoryCategory::Textures, 999);
    b.try_allocate(MemoryCategory::Geometry, 333);
    let snap = b.snapshot();
    let tex = snap
        .iter()
        .find(|(c, _, _)| *c == MemoryCategory::Textures)
        .unwrap();
    let geo = snap
        .iter()
        .find(|(c, _, _)| *c == MemoryCategory::Geometry)
        .unwrap();
    assert_eq!(tex.1, 999);
    assert_eq!(geo.1, 333);
}

#[test]
fn budget_snapshot_includes_hard_limits() {
    let b = GpuMemoryBudget::new();
    b.set_category_budget(MemoryCategory::Shadows, 100, 500);
    let snap = b.snapshot();
    let shad = snap
        .iter()
        .find(|(c, _, _)| *c == MemoryCategory::Shadows)
        .unwrap();
    assert_eq!(shad.2, 500, "hard limit should be 500");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  TransparencyManager — basics
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn transparency_new_count_zero() {
    let mgr = TransparencyManager::new();
    assert_eq!(mgr.count(), 0);
}

#[test]
fn transparency_default_same_as_new() {
    let mgr = TransparencyManager::default();
    assert_eq!(mgr.count(), 0);
}

#[test]
fn transparency_add_increments_count() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::new(1.0, 2.0, 3.0), BlendMode::Alpha);
    assert_eq!(mgr.count(), 1);
    mgr.add_instance(1, Vec3::new(4.0, 5.0, 6.0), BlendMode::Additive);
    assert_eq!(mgr.count(), 2);
}

#[test]
fn transparency_clear_resets_count() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::ZERO, BlendMode::Alpha);
    mgr.add_instance(1, Vec3::ONE, BlendMode::Multiplicative);
    mgr.clear();
    assert_eq!(mgr.count(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  TransparencyManager — sorting
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn transparency_sort_back_to_front() {
    let mut mgr = TransparencyManager::new();
    // Camera at origin, instances at varying distances along -Z
    mgr.add_instance(0, Vec3::new(0.0, 0.0, -2.0), BlendMode::Alpha); // 2 units
    mgr.add_instance(1, Vec3::new(0.0, 0.0, -10.0), BlendMode::Alpha); // 10 units (furthest)
    mgr.add_instance(2, Vec3::new(0.0, 0.0, -5.0), BlendMode::Alpha); // 5 units
    mgr.update(Vec3::ZERO);
    let sorted: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
    assert_eq!(
        sorted,
        vec![1, 2, 0],
        "back-to-front: furthest (10) first, closest (2) last"
    );
}

#[test]
fn transparency_sort_updates_with_camera_move() {
    let mut mgr = TransparencyManager::new();
    // Place instances along X axis
    mgr.add_instance(0, Vec3::new(0.0, 0.0, 0.0), BlendMode::Alpha); // at origin
    mgr.add_instance(1, Vec3::new(10.0, 0.0, 0.0), BlendMode::Alpha); // at x=10
                                                                      // Camera at x=-5 → instance 1 is further
    mgr.update(Vec3::new(-5.0, 0.0, 0.0));
    let sorted1: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
    assert_eq!(
        sorted1,
        vec![1, 0],
        "instance 1 further from camera at x=-5"
    );
    // Move camera to x=20 → instance 0 is now further
    mgr.update(Vec3::new(20.0, 0.0, 0.0));
    let sorted2: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
    assert_eq!(sorted2, vec![0, 1], "from x=20, instance 0 is further");
}

#[test]
fn transparency_sort_empty_manager() {
    let mut mgr = TransparencyManager::new();
    mgr.update(Vec3::ZERO);
    let sorted: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
    assert!(sorted.is_empty());
}

#[test]
fn transparency_sort_single_instance() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(42, Vec3::new(1.0, 2.0, 3.0), BlendMode::Alpha);
    mgr.update(Vec3::ZERO);
    let sorted: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
    assert_eq!(sorted, vec![42]);
}

#[test]
fn transparency_add_instance_calculates_distance() {
    let mut mgr = TransparencyManager::new();
    // Camera at origin (default), add instance at (3,4,0) → distance = 5
    mgr.add_instance(0, Vec3::new(3.0, 4.0, 0.0), BlendMode::Alpha);
    mgr.update(Vec3::ZERO);
    let inst = mgr.sorted_instances().next().unwrap();
    assert!(
        (inst.camera_distance - 5.0).abs() < 0.01,
        "distance from (0,0,0) to (3,4,0) = 5"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
//  TransparencyManager — blend mode filter
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn transparency_instances_by_blend_mode_alpha() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::new(0.0, 0.0, -1.0), BlendMode::Alpha);
    mgr.add_instance(1, Vec3::new(0.0, 0.0, -2.0), BlendMode::Additive);
    mgr.add_instance(2, Vec3::new(0.0, 0.0, -3.0), BlendMode::Alpha);
    mgr.update(Vec3::ZERO);
    let alpha_instances: Vec<u32> = mgr
        .instances_by_blend_mode(BlendMode::Alpha)
        .map(|i| i.instance_index)
        .collect();
    assert_eq!(alpha_instances.len(), 2);
    assert!(alpha_instances.contains(&0));
    assert!(alpha_instances.contains(&2));
    assert!(!alpha_instances.contains(&1));
}

#[test]
fn transparency_instances_by_blend_mode_additive() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::new(0.0, 0.0, -1.0), BlendMode::Additive);
    mgr.add_instance(1, Vec3::new(0.0, 0.0, -2.0), BlendMode::Alpha);
    mgr.update(Vec3::ZERO);
    let additive: Vec<u32> = mgr
        .instances_by_blend_mode(BlendMode::Additive)
        .map(|i| i.instance_index)
        .collect();
    assert_eq!(additive.len(), 1);
    assert_eq!(additive[0], 0);
}

#[test]
fn transparency_instances_by_blend_mode_multiplicative() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::ZERO, BlendMode::Multiplicative);
    mgr.update(Vec3::ZERO);
    let mult: Vec<u32> = mgr
        .instances_by_blend_mode(BlendMode::Multiplicative)
        .map(|i| i.instance_index)
        .collect();
    assert_eq!(mult.len(), 1);
}

#[test]
fn transparency_instances_by_blend_mode_none_match() {
    let mut mgr = TransparencyManager::new();
    mgr.add_instance(0, Vec3::ZERO, BlendMode::Alpha);
    mgr.update(Vec3::ZERO);
    let mult: Vec<u32> = mgr
        .instances_by_blend_mode(BlendMode::Multiplicative)
        .map(|i| i.instance_index)
        .collect();
    assert!(mult.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  BlendMode enum
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn blend_mode_equality() {
    assert_eq!(BlendMode::Alpha, BlendMode::Alpha);
    assert_eq!(BlendMode::Additive, BlendMode::Additive);
    assert_eq!(BlendMode::Multiplicative, BlendMode::Multiplicative);
    assert_ne!(BlendMode::Alpha, BlendMode::Additive);
    assert_ne!(BlendMode::Alpha, BlendMode::Multiplicative);
    assert_ne!(BlendMode::Additive, BlendMode::Multiplicative);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  create_blend_state — blend factor verification
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn blend_state_alpha_color_factors() {
    let bs = create_blend_state(BlendMode::Alpha);
    assert_eq!(bs.color.src_factor, wgpu::BlendFactor::SrcAlpha);
    assert_eq!(bs.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
    assert_eq!(bs.color.operation, wgpu::BlendOperation::Add);
}

#[test]
fn blend_state_alpha_alpha_factors() {
    let bs = create_blend_state(BlendMode::Alpha);
    assert_eq!(bs.alpha.src_factor, wgpu::BlendFactor::One);
    assert_eq!(bs.alpha.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
    assert_eq!(bs.alpha.operation, wgpu::BlendOperation::Add);
}

#[test]
fn blend_state_additive_color_factors() {
    let bs = create_blend_state(BlendMode::Additive);
    assert_eq!(bs.color.src_factor, wgpu::BlendFactor::SrcAlpha);
    assert_eq!(bs.color.dst_factor, wgpu::BlendFactor::One);
    assert_eq!(bs.color.operation, wgpu::BlendOperation::Add);
}

#[test]
fn blend_state_additive_alpha_factors() {
    let bs = create_blend_state(BlendMode::Additive);
    assert_eq!(bs.alpha.src_factor, wgpu::BlendFactor::One);
    assert_eq!(bs.alpha.dst_factor, wgpu::BlendFactor::One);
}

#[test]
fn blend_state_multiplicative_color_factors() {
    let bs = create_blend_state(BlendMode::Multiplicative);
    assert_eq!(bs.color.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(bs.color.dst_factor, wgpu::BlendFactor::Src);
    assert_eq!(bs.color.operation, wgpu::BlendOperation::Add);
}

#[test]
fn blend_state_multiplicative_alpha_factors() {
    let bs = create_blend_state(BlendMode::Multiplicative);
    assert_eq!(bs.alpha.src_factor, wgpu::BlendFactor::Zero);
    assert_eq!(bs.alpha.dst_factor, wgpu::BlendFactor::One);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  WaterUniforms — defaults and layout
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn water_uniforms_size_512_bytes() {
    assert_eq!(std::mem::size_of::<WaterUniforms>(), 512);
}

#[test]
fn water_uniforms_default_camera_pos() {
    let u = WaterUniforms::default();
    assert_eq!(u.camera_pos, [0.0, 5.0, -10.0]);
}

#[test]
fn water_uniforms_default_time_zero() {
    let u = WaterUniforms::default();
    assert!((u.time - 0.0).abs() < 1e-6);
}

#[test]
fn water_uniforms_default_deep_color() {
    let u = WaterUniforms::default();
    let eps = 1e-4;
    assert!((u.water_color_deep[0] - 0.02).abs() < eps);
    assert!((u.water_color_deep[1] - 0.08).abs() < eps);
    assert!((u.water_color_deep[2] - 0.2).abs() < eps);
}

#[test]
fn water_uniforms_default_shallow_color() {
    let u = WaterUniforms::default();
    let eps = 1e-4;
    assert!((u.water_color_shallow[0] - 0.1).abs() < eps);
    assert!((u.water_color_shallow[1] - 0.4).abs() < eps);
    assert!((u.water_color_shallow[2] - 0.5).abs() < eps);
}

#[test]
fn water_uniforms_default_foam_color() {
    let u = WaterUniforms::default();
    let eps = 1e-4;
    assert!((u.foam_color[0] - 0.95).abs() < eps);
    assert!((u.foam_color[1] - 0.98).abs() < eps);
    assert!((u.foam_color[2] - 1.0).abs() < eps);
}

#[test]
fn water_uniforms_default_foam_threshold() {
    let u = WaterUniforms::default();
    assert!((u.foam_threshold - 0.6).abs() < 1e-4);
}

#[test]
fn water_uniforms_bytemuck_roundtrip() {
    let u = WaterUniforms::default();
    let bytes = bytemuck::bytes_of(&u);
    assert_eq!(bytes.len(), 512);
    let back: &WaterUniforms = bytemuck::from_bytes(bytes);
    assert_eq!(back.camera_pos, u.camera_pos);
    assert!((back.foam_threshold - u.foam_threshold).abs() < 1e-6);
}

#[test]
fn water_uniforms_padding_zeroed() {
    let u = WaterUniforms::default();
    assert!((u._pad0 - 0.0).abs() < 1e-6);
    assert!((u._pad1 - 0.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  WaterVertex
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn water_vertex_size() {
    // position: [f32; 3] (12B) + uv: [f32; 2] (8B) = 20B
    assert_eq!(std::mem::size_of::<WaterVertex>(), 20);
}

#[test]
fn water_vertex_desc_stride() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.array_stride, 20);
}

#[test]
fn water_vertex_desc_two_attributes() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.attributes.len(), 2);
}

#[test]
fn water_vertex_desc_position_attribute() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.attributes[0].offset, 0);
    assert_eq!(desc.attributes[0].shader_location, 0);
    assert_eq!(desc.attributes[0].format, wgpu::VertexFormat::Float32x3);
}

#[test]
fn water_vertex_desc_uv_attribute() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.attributes[1].offset, 12);
    assert_eq!(desc.attributes[1].shader_location, 1);
    assert_eq!(desc.attributes[1].format, wgpu::VertexFormat::Float32x2);
}

#[test]
fn water_vertex_step_mode() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.step_mode, wgpu::VertexStepMode::Vertex);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MsaaMode — integration test supplement
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn msaa_default_is_x4() {
    assert_eq!(MsaaMode::default(), MsaaMode::X4);
}

#[test]
fn msaa_sample_counts_all_variants() {
    assert_eq!(MsaaMode::Off.sample_count(), 1);
    assert_eq!(MsaaMode::X2.sample_count(), 2);
    assert_eq!(MsaaMode::X4.sample_count(), 4);
    assert_eq!(MsaaMode::X8.sample_count(), 8);
}

#[test]
fn msaa_is_enabled_off_false() {
    assert!(!MsaaMode::Off.is_enabled());
}

#[test]
fn msaa_is_enabled_x2_x4_x8_true() {
    assert!(MsaaMode::X2.is_enabled());
    assert!(MsaaMode::X4.is_enabled());
    assert!(MsaaMode::X8.is_enabled());
}

#[test]
fn msaa_multisample_state_x4() {
    let state = MsaaMode::X4.multisample_state();
    assert_eq!(state.count, 4);
    assert_eq!(state.mask, !0);
    assert!(!state.alpha_to_coverage_enabled);
}

#[test]
fn msaa_multisample_state_sample_count_matches() {
    for mode in [MsaaMode::Off, MsaaMode::X2, MsaaMode::X4, MsaaMode::X8] {
        let state = mode.multisample_state();
        assert_eq!(
            state.count,
            mode.sample_count(),
            "multisample_state.count should match sample_count() for {:?}",
            mode
        );
    }
}
