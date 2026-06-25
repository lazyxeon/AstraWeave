//! Mutation-resistant comprehensive tests for astraweave-fluids.
//!
//! Targets: FluidTimingStats (default, total_ms, breakdown), FluidProfiler (new, enable,
//! record_frame, average_stats, reset), FoamConfig (default, calm, stormy, rapids),
//! SnapshotParams (default), FluidSnapshot (with_capacity, bincode roundtrip),
//! OptimizationStats (default).
//!
//! (W.1: volume_grid coverage — MaterialType / WaterCell / WaterSimConfig /
//! CellFlags / WaterVolumeGrid — was removed with the voxel sim per the W.0
//! salvage list.)

#![allow(clippy::field_reassign_with_default)]

use astraweave_fluids::*;

// =========================================================================
// FluidTimingStats — Default, total_ms, breakdown
// =========================================================================

#[test]
fn timing_stats_default_all_zero() {
    let s = FluidTimingStats::default();
    assert_eq!(s.total_step_us, 0);
    assert_eq!(s.sdf_gen_us, 0);
    assert_eq!(s.predict_us, 0);
    assert_eq!(s.grid_build_us, 0);
    assert_eq!(s.constraint_solve_us, 0);
    assert_eq!(s.integrate_us, 0);
    assert_eq!(s.secondary_us, 0);
    assert_eq!(s.heat_diffuse_us, 0);
    assert_eq!(s.frame, 0);
}

#[test]
fn timing_stats_total_ms_zero() {
    let s = FluidTimingStats::default();
    assert_eq!(s.total_ms(), 0.0);
}

#[test]
fn timing_stats_total_ms_converts_correctly() {
    let mut s = FluidTimingStats::default();
    s.total_step_us = 1000;
    assert!((s.total_ms() - 1.0).abs() < 1e-6);
}

#[test]
fn timing_stats_total_ms_large_value() {
    let mut s = FluidTimingStats::default();
    s.total_step_us = 16_667; // ~16.667ms
    assert!((s.total_ms() - 16.667).abs() < 0.001);
}

#[test]
fn timing_stats_breakdown_keys() {
    let s = FluidTimingStats::default();
    let b = s.breakdown();
    assert!(b.contains_key("sdf_gen"));
    assert!(b.contains_key("predict"));
    assert!(b.contains_key("grid_build"));
    assert!(b.contains_key("constraint_solve"));
    assert!(b.contains_key("integrate"));
    assert!(b.contains_key("secondary"));
    assert!(b.contains_key("heat_diffuse"));
}

#[test]
fn timing_stats_breakdown_values_are_percentages() {
    // breakdown() returns percentages: (field_us / total_step_us.max(1)) * 100.0
    let mut s = FluidTimingStats::default();
    s.total_step_us = 1000;
    s.sdf_gen_us = 100; // 10%
    s.predict_us = 200; // 20%
    s.grid_build_us = 300; // 30%
    s.constraint_solve_us = 100; // 10%
    s.integrate_us = 150; // 15%
    s.secondary_us = 100; // 10%
    s.heat_diffuse_us = 50; // 5%
    let b = s.breakdown();
    assert!((b["sdf_gen"] - 10.0).abs() < 1e-3);
    assert!((b["predict"] - 20.0).abs() < 1e-3);
    assert!((b["grid_build"] - 30.0).abs() < 1e-3);
    assert!((b["constraint_solve"] - 10.0).abs() < 1e-3);
    assert!((b["integrate"] - 15.0).abs() < 1e-3);
    assert!((b["secondary"] - 10.0).abs() < 1e-3);
    assert!((b["heat_diffuse"] - 5.0).abs() < 1e-3);
}

#[test]
fn timing_stats_eq() {
    let a = FluidTimingStats::default();
    let b = FluidTimingStats::default();
    assert_eq!(a, b);
}

#[test]
fn timing_stats_ne() {
    let mut a = FluidTimingStats::default();
    a.frame = 1;
    let b = FluidTimingStats::default();
    assert_ne!(a, b);
}

#[test]
fn timing_stats_clone() {
    let mut s = FluidTimingStats::default();
    s.total_step_us = 999;
    let c = s.clone();
    assert_eq!(c.total_step_us, 999);
}

// =========================================================================
// FluidProfiler — new, enable, record_frame, average_stats, reset
// =========================================================================

#[test]
fn profiler_new_disabled() {
    let p = FluidProfiler::new();
    assert!(!p.is_enabled());
}

#[test]
fn profiler_set_enabled() {
    let mut p = FluidProfiler::new();
    p.set_enabled(true);
    assert!(p.is_enabled());
}

#[test]
fn profiler_set_enabled_false() {
    let mut p = FluidProfiler::new();
    p.set_enabled(true);
    p.set_enabled(false);
    assert!(!p.is_enabled());
}

#[test]
fn profiler_default_stats_zero() {
    let p = FluidProfiler::new();
    let s = p.stats();
    assert_eq!(s.total_step_us, 0);
}

#[test]
fn profiler_record_frame_updates_stats() {
    let mut p = FluidProfiler::new();
    p.set_enabled(true);
    let mut s = FluidTimingStats::default();
    s.total_step_us = 1000;
    s.predict_us = 500;
    p.record_frame(s);
    assert_eq!(p.stats().total_step_us, 1000);
    assert_eq!(p.stats().predict_us, 500);
}

#[test]
fn profiler_average_stats_single_frame() {
    let mut p = FluidProfiler::new();
    p.set_enabled(true);
    let mut s = FluidTimingStats::default();
    s.total_step_us = 2000;
    p.record_frame(s);
    let avg = p.average_stats();
    assert_eq!(avg.total_step_us, 2000);
}

#[test]
fn profiler_average_stats_two_frames() {
    let mut p = FluidProfiler::new();
    p.set_enabled(true);
    let mut s1 = FluidTimingStats::default();
    s1.total_step_us = 1000;
    p.record_frame(s1);
    let mut s2 = FluidTimingStats::default();
    s2.total_step_us = 3000;
    p.record_frame(s2);
    let avg = p.average_stats();
    assert_eq!(avg.total_step_us, 2000); // (1000 + 3000) / 2
}

#[test]
fn profiler_reset_clears_stats() {
    let mut p = FluidProfiler::new();
    p.set_enabled(true);
    let mut s = FluidTimingStats::default();
    s.total_step_us = 5000;
    p.record_frame(s);
    p.reset();
    assert_eq!(p.stats().total_step_us, 0);
}

// =========================================================================
// FoamConfig — Default, calm, stormy, rapids
// =========================================================================

#[test]
fn foam_config_default_max_particles() {
    let f = FoamConfig::default();
    assert_eq!(f.max_particles, 10000);
}

#[test]
fn foam_config_default_lifetime() {
    let f = FoamConfig::default();
    assert!((f.lifetime - 3.0).abs() < 1e-6);
}

#[test]
fn foam_config_default_spread_rate() {
    let f = FoamConfig::default();
    assert!((f.spread_rate - 0.5).abs() < 1e-6);
}

#[test]
fn foam_config_default_fade_rate() {
    let f = FoamConfig::default();
    assert!((f.fade_rate - 0.4).abs() < 1e-6);
}

#[test]
fn foam_config_default_whitecap_threshold() {
    let f = FoamConfig::default();
    assert!((f.whitecap_threshold - 0.6).abs() < 1e-6);
}

#[test]
fn foam_config_default_shore_intensity() {
    let f = FoamConfig::default();
    assert!((f.shore_intensity - 1.5).abs() < 1e-6);
}

#[test]
fn foam_config_default_wake_intensity() {
    let f = FoamConfig::default();
    assert!((f.wake_intensity - 1.0).abs() < 1e-6);
}

#[test]
fn foam_config_default_texture_scale() {
    let f = FoamConfig::default();
    assert!((f.texture_scale - 2.0).abs() < 1e-6);
}

#[test]
fn foam_config_default_color() {
    let f = FoamConfig::default();
    assert!((f.color.x - 0.95).abs() < 1e-6);
    assert!((f.color.y - 0.98).abs() < 1e-6);
    assert!((f.color.z - 1.0).abs() < 1e-6);
}

#[test]
fn foam_config_default_initial_opacity() {
    let f = FoamConfig::default();
    assert!((f.initial_opacity - 0.8).abs() < 1e-6);
}

#[test]
fn foam_config_calm_differs_from_default() {
    let calm = FoamConfig::calm();
    let def = FoamConfig::default();
    // calm should be gentler
    assert!(
        calm.whitecap_threshold > def.whitecap_threshold || calm.spread_rate != def.spread_rate
    );
}

#[test]
fn foam_config_stormy_differs_from_default() {
    let stormy = FoamConfig::stormy();
    let def = FoamConfig::default();
    assert_ne!(stormy, def);
}

#[test]
fn foam_config_rapids_differs_from_default() {
    let rapids = FoamConfig::rapids();
    let def = FoamConfig::default();
    assert_ne!(rapids, def);
}

#[test]
fn foam_config_clone() {
    let f = FoamConfig::default();
    let c = f.clone();
    assert_eq!(f, c);
}

// =========================================================================
// OptimizationStats — Default
// =========================================================================

#[test]
fn optimization_stats_default_quality_level() {
    let s = OptimizationStats::default();
    assert_eq!(s.quality_level, 0.0);
}

#[test]
fn optimization_stats_default_iterations() {
    let s = OptimizationStats::default();
    assert_eq!(s.iterations, 0);
}

#[test]
fn optimization_stats_default_resting_particles() {
    let s = OptimizationStats::default();
    assert_eq!(s.resting_particles, 0);
}

#[test]
fn optimization_stats_default_recommended_iterations() {
    let s = OptimizationStats::default();
    assert_eq!(s.recommended_iterations, 0);
}

#[test]
fn optimization_stats_default_under_budget() {
    let s = OptimizationStats::default();
    assert!(!s.under_budget);
}

#[test]
fn optimization_stats_clone() {
    let mut s = OptimizationStats::default();
    s.quality_level = 0.75;
    s.iterations = 4;
    s.under_budget = true;
    let c = s.clone();
    assert!((c.quality_level - 0.75).abs() < 1e-6);
    assert_eq!(c.iterations, 4);
    assert!(c.under_budget);
}

// =========================================================================
// SnapshotParams — default values
// =========================================================================

#[test]
fn snapshot_params_default_smoothing_radius() {
    let p = SnapshotParams::default();
    assert!((p.smoothing_radius - 1.0).abs() < 1e-6);
}

#[test]
fn snapshot_params_default_target_density() {
    let p = SnapshotParams::default();
    assert!((p.target_density - 12.0).abs() < 1e-6);
}

#[test]
fn snapshot_params_default_pressure_multiplier() {
    let p = SnapshotParams::default();
    assert!((p.pressure_multiplier - 300.0).abs() < 1e-6);
}

#[test]
fn snapshot_params_default_viscosity() {
    let p = SnapshotParams::default();
    assert!((p.viscosity - 10.0).abs() < 1e-6);
}

#[test]
fn snapshot_params_default_surface_tension() {
    let p = SnapshotParams::default();
    assert!((p.surface_tension - 0.02).abs() < 1e-6);
}

#[test]
fn snapshot_params_default_gravity() {
    let p = SnapshotParams::default();
    assert!((p.gravity - (-9.8)).abs() < 1e-6);
}

#[test]
fn snapshot_params_default_iterations() {
    let p = SnapshotParams::default();
    assert_eq!(p.iterations, 4);
}

#[test]
fn snapshot_params_default_cell_size() {
    let p = SnapshotParams::default();
    assert!((p.cell_size - 1.2).abs() < 1e-6);
}

#[test]
fn snapshot_params_default_grid_dims() {
    let p = SnapshotParams::default();
    assert_eq!(p.grid_width, 128);
    assert_eq!(p.grid_height, 128);
    assert_eq!(p.grid_depth, 128);
}

#[test]
fn snapshot_params_serde_roundtrip() {
    let p = SnapshotParams::default();
    let json = serde_json::to_string(&p).unwrap();
    let p2: SnapshotParams = serde_json::from_str(&json).unwrap();
    assert!((p.smoothing_radius - p2.smoothing_radius).abs() < 1e-15);
    assert!((p.gravity - p2.gravity).abs() < 1e-15);
    assert_eq!(p.iterations, p2.iterations);
}

// =========================================================================
// FluidSnapshot — with_capacity, bincode roundtrip
// =========================================================================

#[test]
fn fluid_snapshot_with_capacity_version() {
    let snap = FluidSnapshot::with_capacity(100);
    assert_eq!(snap.version, FluidSnapshot::VERSION);
}

#[test]
fn fluid_snapshot_with_capacity_empty_vecs() {
    let snap = FluidSnapshot::with_capacity(100);
    assert_eq!(snap.positions.len(), 0);
    assert_eq!(snap.velocities.len(), 0);
    assert_eq!(snap.colors.len(), 0);
}

#[test]
fn fluid_snapshot_with_capacity_frame_index_zero() {
    let snap = FluidSnapshot::with_capacity(100);
    assert_eq!(snap.frame_index, 0);
}

#[test]
fn fluid_snapshot_with_capacity_active_count_zero() {
    let snap = FluidSnapshot::with_capacity(100);
    assert_eq!(snap.active_count, 0);
}

#[test]
fn fluid_snapshot_bincode_roundtrip() {
    let mut snap = FluidSnapshot::with_capacity(2);
    snap.positions.push([1.0, 2.0, 3.0, 0.0]);
    snap.velocities.push([0.1, 0.2, 0.3, 0.0]);
    snap.colors.push([1.0, 1.0, 1.0, 1.0]);
    snap.frame_index = 42;
    snap.active_count = 1;

    let bytes = snap.to_bytes().unwrap();
    let restored = FluidSnapshot::from_bytes(&bytes).unwrap();
    assert_eq!(restored.version, snap.version);
    assert_eq!(restored.frame_index, 42);
    assert_eq!(restored.active_count, 1);
    assert_eq!(restored.positions.len(), 1);
    assert!((restored.positions[0][0] - 1.0).abs() < 1e-6);
}

#[test]
fn fluid_snapshot_version_constant() {
    assert_eq!(FluidSnapshot::VERSION, 1);
}
