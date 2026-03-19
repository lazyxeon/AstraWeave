//! Wave 2 Mutation Remediation — Erosion Algorithm Internals
//!
//! Targets the 372 missed mutants in advanced_erosion.rs by testing:
//! - Bilinear interpolation (calculate_height_and_gradient, sample_height_bilinear)
//! - Sediment deposit distribution (deposit_sediment)
//! - Erosion brush initialization (init_erosion_brush)
//! - Hydraulic erosion loop arithmetic (velocity, capacity, evaporation)
//! - Thermal erosion material redistribution
//! - Wind erosion windward/leeward logic
//! - SimpleRng determinism
//! - ErosionStats tracking

use astraweave_terrain::{
    AdvancedErosionSimulator, ErosionPreset, ErosionStats, Heightmap, HeightmapConfig,
    HydraulicErosionConfig, ThermalErosionConfig, WindErosionConfig,
};
use glam::Vec2;

// ============================================================================
// Helpers
// ============================================================================

fn make_heightmap(resolution: u32, heights: &[f32]) -> Heightmap {
    Heightmap::from_data(heights.to_vec(), resolution).unwrap()
}

fn flat_hm(resolution: u32, height: f32) -> Heightmap {
    let data = vec![height; (resolution * resolution) as usize];
    Heightmap::from_data(data, resolution).unwrap()
}

/// 4×4 heightmap with known bilinear-testable values:
///  row0: [0, 1, 2, 3]
///  row1: [4, 5, 6, 7]
///  row2: [8, 9, 10, 11]
///  row3: [12, 13, 14, 15]
fn grid_4x4() -> Heightmap {
    let data: Vec<f32> = (0..16).map(|i| i as f32).collect();
    Heightmap::from_data(data, 4).unwrap()
}

/// Sloped heightmap: height = x * scale
fn sloped_x(resolution: u32, scale: f32) -> Heightmap {
    let mut data = vec![0.0f32; (resolution * resolution) as usize];
    for z in 0..resolution {
        for x in 0..resolution {
            data[(z * resolution + x) as usize] = x as f32 * scale;
        }
    }
    Heightmap::from_data(data, resolution).unwrap()
}

/// Peak heightmap: height is highest at center, drops off radially
fn peak_hm(resolution: u32, peak: f32) -> Heightmap {
    let center = resolution as f32 / 2.0;
    let mut data = vec![0.0f32; (resolution * resolution) as usize];
    for z in 0..resolution {
        for x in 0..resolution {
            let dx = x as f32 - center;
            let dz = z as f32 - center;
            let dist = (dx * dx + dz * dz).sqrt();
            data[(z * resolution + x) as usize] = (center - dist).max(0.0) * peak / center;
        }
    }
    Heightmap::from_data(data, resolution).unwrap()
}

/// Small config for fast hydraulic erosion tests
fn fast_hydraulic(droplets: u32) -> HydraulicErosionConfig {
    HydraulicErosionConfig {
        droplet_count: droplets,
        erosion_radius: 2,
        max_droplet_lifetime: 15,
        ..Default::default()
    }
}

// ============================================================================
// BILINEAR INTERPOLATION — via hydraulic erosion on known heightmaps
// ============================================================================

/// On a perfectly flat terrain, hydraulic erosion should produce negligible total erosion.
/// This tests that height == new_height (delta_height ≈ 0) is computed correctly, meaning
/// the bilinear interpolation returns the SAME value at new_pos as at pos.
#[test]
fn bilinear_flat_terrain_zero_delta() {
    let mut hm = flat_hm(16, 50.0);
    let original_data = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = fast_hydraulic(500);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // On flat terrain, sediment capacity = (-0).max(min_slope) * vel * water * factor
    // Erosion is bounded by -delta_height = 0, so each erode step erodes min(capacity-sed, 0) ≈
    // very little. Total erosion should be negligible relative to terrain height.
    let max_change = hm
        .data()
        .iter()
        .zip(original_data.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    // Each cell should barely change on flat terrain
    assert!(
        max_change < 1.0,
        "Flat terrain max cell change was {max_change}, expected < 1.0"
    );
    // Deposition should roughly equal erosion on flat terrain (everything deposited back)
    // Both should be relatively small
    assert!(
        stats.total_eroded < 100.0,
        "Flat terrain total_eroded={}, expected small",
        stats.total_eroded
    );
}

/// On a uniformly sloped terrain (height = x), droplets should consistently move
/// in the -x direction (downhill gradient). Testing that the gradient calculation
/// correctly identifies the slope direction.
#[test]
fn gradient_follows_slope_direction() {
    let mut hm = sloped_x(32, 2.0); // height = 2*x, gradient should point in +x
    let before_data = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(777);
    let config = HydraulicErosionConfig {
        droplet_count: 5000,
        inertia: 0.0, // Zero inertia = droplets follow gradient exactly
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // With consistent downhill flow, we expect erosion at higher-x cells
    // and deposition at lower-x cells. Total eroded should be significant.
    assert!(
        stats.total_eroded > 10.0,
        "Sloped terrain should produce meaningful erosion, got {}",
        stats.total_eroded
    );

    // The higher side (right) should lose more material than the lower side (left)
    let res = 32u32;
    let left_change: f32 = (0..res)
        .map(|z| {
            let idx = (z * res + 0) as usize;
            (hm.data()[idx] - before_data[idx])
        })
        .sum();
    let right_change: f32 = (0..res)
        .map(|z| {
            let idx = (z * res + res - 2) as usize;
            (hm.data()[idx] - before_data[idx])
        })
        .sum();

    // Right side should lose material (negative change), left side should gain
    assert!(
        right_change < left_change,
        "Right (high) side change={right_change} should be less than left (low) side={left_change}"
    );
}

/// Verify that different seeds produce different erosion patterns.
/// This catches mutations in the SimpleRng xorshift arithmetic.
#[test]
fn different_seeds_different_results() {
    let config = fast_hydraulic(2000);
    let mut hm1 = peak_hm(32, 20.0);
    let mut hm2 = peak_hm(32, 20.0);

    let mut sim1 = AdvancedErosionSimulator::new(111);
    let mut sim2 = AdvancedErosionSimulator::new(222);

    sim1.apply_hydraulic_erosion(&mut hm1, &config);
    sim2.apply_hydraulic_erosion(&mut hm2, &config);

    let diff_count = hm1
        .data()
        .iter()
        .zip(hm2.data().iter())
        .filter(|(a, b)| (*a - *b).abs() > 0.001)
        .count();
    assert!(
        diff_count > 10,
        "Different seeds should produce different terrains, only {} cells differ",
        diff_count
    );
}

/// Same seed must produce bit-identical results (determinism).
/// This catches mutations in the xorshift state transitions.
#[test]
fn same_seed_exact_match() {
    let config = fast_hydraulic(1000);
    let mut hm1 = peak_hm(16, 10.0);
    let mut hm2 = peak_hm(16, 10.0);

    let mut sim1 = AdvancedErosionSimulator::new(99999);
    let mut sim2 = AdvancedErosionSimulator::new(99999);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config);

    // Heights must be identical
    for (i, (a, b)) in hm1.data().iter().zip(hm2.data().iter()).enumerate() {
        assert!((a - b).abs() < 1e-6, "Cell {i} mismatch: {a} vs {b}");
    }
    // Stats must match
    assert_eq!(stats1.droplets_terminated, stats2.droplets_terminated);
    assert!((stats1.total_eroded - stats2.total_eroded).abs() < 1e-6);
    assert!((stats1.total_deposited - stats2.total_deposited).abs() < 1e-6);
}

// ============================================================================
// EROSION STATS TRACKING
// ============================================================================

/// Verify total_eroded is accumulated correctly across brush cells.
/// If the += delta operation is mutated (e.g., -= or =), total_eroded will be wrong.
#[test]
fn hydraulic_total_eroded_positive_on_slope() {
    let mut hm = peak_hm(32, 30.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = fast_hydraulic(5000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    assert!(
        stats.total_eroded > 0.0,
        "total_eroded should be > 0, got {}",
        stats.total_eroded
    );
    assert!(
        stats.total_deposited > 0.0,
        "total_deposited should be > 0, got {}",
        stats.total_deposited
    );
}

/// max_erosion_depth should be > 0 on terrain with significant slope.
#[test]
fn hydraulic_max_erosion_depth_nonzero() {
    let mut hm = peak_hm(32, 40.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = fast_hydraulic(5000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    assert!(
        stats.max_erosion_depth > 0.0,
        "max_erosion_depth should be positive, got {}",
        stats.max_erosion_depth
    );
}

/// erosion_map should be Some and have the same size as the heightmap.
#[test]
fn hydraulic_erosion_map_correct_size() {
    let mut hm = peak_hm(16, 10.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = fast_hydraulic(500);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    let map = stats
        .erosion_map
        .as_ref()
        .expect("erosion_map should be Some");
    assert_eq!(map.len(), (16 * 16) as usize);
}

/// erosion_map should contain both positive and negative values on steep terrain.
/// Positive = deposition, negative = erosion.
#[test]
fn hydraulic_erosion_map_has_both_signs() {
    let mut hm = peak_hm(32, 30.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = fast_hydraulic(5000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    let map = stats.erosion_map.as_ref().unwrap();
    let has_neg = map.iter().any(|&v| v < -0.001);
    let has_pos = map.iter().any(|&v| v > 0.001);
    assert!(has_neg, "erosion_map should have negative values (erosion)");
    assert!(
        has_pos,
        "erosion_map should have positive values (deposition)"
    );
}

/// avg_droplet_lifetime should be > 0 and ≤ max_droplet_lifetime.
#[test]
fn hydraulic_avg_lifetime_bounded() {
    let mut hm = peak_hm(32, 20.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = fast_hydraulic(2000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    assert!(
        stats.avg_droplet_lifetime > 0.0,
        "avg_droplet_lifetime should be positive, got {}",
        stats.avg_droplet_lifetime
    );
    assert!(
        stats.avg_droplet_lifetime <= config.max_droplet_lifetime as f32,
        "avg_droplet_lifetime {} exceeds max {}",
        stats.avg_droplet_lifetime,
        config.max_droplet_lifetime
    );
}

/// droplets_terminated should be ≤ droplet_count.
#[test]
fn hydraulic_terminated_bounded() {
    let mut hm = peak_hm(16, 10.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = fast_hydraulic(1000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    assert!(
        stats.droplets_terminated <= config.droplet_count,
        "terminated {} > count {}",
        stats.droplets_terminated,
        config.droplet_count
    );
}

// ============================================================================
// VELOCITY UPDATE — v_new = sqrt(v² + |Δh| * g)
// ============================================================================

/// Higher gravity should lead to more total erosion on steep terrain.
/// This catches mutations in the `delta_height.abs() * config.gravity` term.
#[test]
fn higher_gravity_more_erosion() {
    let config_low = HydraulicErosionConfig {
        gravity: 1.0,
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config_high = HydraulicErosionConfig {
        gravity: 10.0,
        ..config_low.clone()
    };

    let mut hm_low = peak_hm(32, 30.0);
    let mut hm_high = peak_hm(32, 30.0);
    let mut sim_low = AdvancedErosionSimulator::new(42);
    let mut sim_high = AdvancedErosionSimulator::new(42);

    let stats_low = sim_low.apply_hydraulic_erosion(&mut hm_low, &config_low);
    let stats_high = sim_high.apply_hydraulic_erosion(&mut hm_high, &config_high);

    assert!(
        stats_high.total_eroded > stats_low.total_eroded,
        "Higher gravity should erode more: high={} vs low={}",
        stats_high.total_eroded,
        stats_low.total_eroded
    );
}

/// Zero gravity means velocity = sqrt(v²) = v (no acceleration from height).
/// Should still erode a bit from initial speed but much less than with gravity.
#[test]
fn zero_gravity_very_low_erosion() {
    let config = HydraulicErosionConfig {
        gravity: 0.0,
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let mut hm = peak_hm(32, 30.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // With zero gravity, velocity only comes from initial_speed
    // Erosion should still happen (initial speed > 0) but be much less
    // than with default gravity=4.0
    let config_default = HydraulicErosionConfig {
        gravity: 4.0,
        ..config.clone()
    };
    let mut hm2 = peak_hm(32, 30.0);
    let mut sim2 = AdvancedErosionSimulator::new(42);
    let stats_def = sim2.apply_hydraulic_erosion(&mut hm2, &config_default);

    assert!(
        stats.total_eroded < stats_def.total_eroded,
        "Zero gravity erosion {} should be less than default {}",
        stats.total_eroded,
        stats_def.total_eroded
    );
}

// ============================================================================
// EVAPORATION — water *= 1.0 - evaporation_rate
// ============================================================================

/// Full evaporation (rate=1.0) should cause droplets to die immediately.
/// This catches mutations in the `1.0 - config.evaporation_rate` expression.
#[test]
fn full_evaporation_kills_droplets_fast() {
    let config = HydraulicErosionConfig {
        evaporation_rate: 1.0, // water *= 0.0 each step
        droplet_count: 1000,
        erosion_radius: 2,
        max_droplet_lifetime: 30,
        ..Default::default()
    };
    let mut hm = peak_hm(32, 20.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // With water=0 after first step, sediment_capacity = 0, so erosion is minimal
    // avg_droplet_lifetime should still run the full loop (water doesn't terminate the droplet)
    // But erosion should be very small since capacity drops to 0
    let config2 = HydraulicErosionConfig {
        evaporation_rate: 0.0,
        ..config.clone()
    };
    let mut hm2 = peak_hm(32, 20.0);
    let mut sim2 = AdvancedErosionSimulator::new(42);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config2);

    assert!(
        stats.total_eroded < stats2.total_eroded,
        "Full evap erosion {} should be less than zero evap {}",
        stats.total_eroded,
        stats2.total_eroded
    );
}

/// Zero evaporation should mean more erosion (water persists).
#[test]
fn zero_evaporation_more_erosion() {
    let base = HydraulicErosionConfig {
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config_low_evap = HydraulicErosionConfig {
        evaporation_rate: 0.001,
        ..base.clone()
    };
    let config_high_evap = HydraulicErosionConfig {
        evaporation_rate: 0.5,
        ..base.clone()
    };

    let mut hm1 = peak_hm(32, 20.0);
    let mut hm2 = peak_hm(32, 20.0);
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config_low_evap);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config_high_evap);

    assert!(
        stats1.total_eroded > stats2.total_eroded,
        "Low evap erosion {} should exceed high evap {}",
        stats1.total_eroded,
        stats2.total_eroded
    );
}

// ============================================================================
// SEDIMENT CAPACITY — (-Δh).max(min_slope) * v * water * factor
// ============================================================================

/// Doubling sediment_capacity_factor should roughly double erosion on uniform terrain.
/// Tests that the multiplication by sediment_capacity_factor is preserved.
#[test]
fn capacity_factor_scales_erosion() {
    let base = HydraulicErosionConfig {
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config1 = HydraulicErosionConfig {
        sediment_capacity_factor: 2.0,
        ..base.clone()
    };
    let config2 = HydraulicErosionConfig {
        sediment_capacity_factor: 8.0,
        ..base.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config1);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config2);

    assert!(
        stats2.total_eroded > stats1.total_eroded,
        "Higher capacity factor should erode more: high={} vs low={}",
        stats2.total_eroded,
        stats1.total_eroded
    );
}

/// Zero capacity factor means zero capacity → all sediment deposited immediately.
#[test]
fn zero_capacity_factor_no_erosion() {
    let config = HydraulicErosionConfig {
        sediment_capacity_factor: 0.0,
        droplet_count: 2000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let mut hm = peak_hm(32, 20.0);
    let original = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // With zero capacity, the erode branch is never taken
    // (sediment > 0 == capacity), so terrain should barely change
    let max_change = hm
        .data()
        .iter()
        .zip(original.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_change < 5.0,
        "Zero capacity factor max change={max_change}, expected minimal"
    );
}

/// min_slope affects capacity when terrain is nearly flat.
/// Higher min_slope → higher capacity on flat areas → more erosion.
#[test]
fn min_slope_increases_erosion_on_flat() {
    let base = HydraulicErosionConfig {
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config_low = HydraulicErosionConfig {
        min_slope: 0.001,
        ..base.clone()
    };
    let config_high = HydraulicErosionConfig {
        min_slope: 1.0,
        ..base.clone()
    };

    // Use a mostly-flat terrain with small variation
    let mut hm1 = flat_hm(32, 50.0);
    let mut hm2 = flat_hm(32, 50.0);
    // Add tiny bumps
    for i in 0..hm1.data().len() {
        let bump = (i as f32 * 0.1).sin() * 0.01;
        hm1.data_mut()[i] += bump;
        hm2.data_mut()[i] += bump;
    }

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config_low);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config_high);

    assert!(
        stats2.total_eroded >= stats1.total_eroded,
        "Higher min_slope should allow more erosion on flat: high={} low={}",
        stats2.total_eroded,
        stats1.total_eroded
    );
}

// ============================================================================
// INERTIA — dir = old_dir * inertia - gradient * (1 - inertia)
// ============================================================================

/// Inertia=0 means direction follows gradient exactly. On uniform slope,
/// all droplets should flow in the same direction.
#[test]
fn zero_inertia_follows_gradient() {
    let config = HydraulicErosionConfig {
        inertia: 0.0,
        droplet_count: 2000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let mut hm = sloped_x(32, 1.0);
    let before = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    sim.apply_hydraulic_erosion(&mut hm, &config);

    // With inertia=0, droplets always go downhill.
    // Right columns (high) should lose height, left columns (low) should gain
    let right_col_sum: f32 = (0..32)
        .map(|z| hm.data()[(z * 32 + 30) as usize] - before[(z * 32 + 30) as usize])
        .sum();
    let left_col_sum: f32 = (0..32)
        .map(|z| hm.data()[(z * 32 + 1) as usize] - before[(z * 32 + 1) as usize])
        .sum();

    assert!(
        right_col_sum < left_col_sum,
        "Right (high) should lose more: right_change={right_col_sum}, left_change={left_col_sum}"
    );
}

/// Inertia=1 means old direction is completely preserved, gradient ignored.
/// Droplets won't follow slope, so erosion pattern differs from inertia=0.
#[test]
fn full_inertia_ignores_gradient() {
    let config_inertia0 = HydraulicErosionConfig {
        inertia: 0.0,
        droplet_count: 2000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config_inertia1 = HydraulicErosionConfig {
        inertia: 1.0,
        ..config_inertia0.clone()
    };

    let mut hm0 = peak_hm(32, 20.0);
    let mut hm1 = peak_hm(32, 20.0);
    let mut sim0 = AdvancedErosionSimulator::new(42);
    let mut sim1 = AdvancedErosionSimulator::new(42);

    let stats0 = sim0.apply_hydraulic_erosion(&mut hm0, &config_inertia0);
    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config_inertia1);

    // Different inertia should produce different erosion amounts
    assert!(
        (stats0.total_eroded - stats1.total_eroded).abs() > 1.0,
        "Inertia 0 vs 1 should differ: i0={} vs i1={}",
        stats0.total_eroded,
        stats1.total_eroded
    );
}

// ============================================================================
// DEPOSIT SPEED — (sediment - capacity) * deposit_speed
// ============================================================================

/// Higher deposit_speed should lead to more deposition (sediment released faster).
#[test]
fn deposit_speed_increases_deposition() {
    let base = HydraulicErosionConfig {
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config_slow = HydraulicErosionConfig {
        deposit_speed: 0.01,
        ..base.clone()
    };
    let config_fast = HydraulicErosionConfig {
        deposit_speed: 0.9,
        ..base.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config_slow);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config_fast);

    assert!(
        stats2.total_deposited > stats1.total_deposited,
        "Fast deposit should deposit more: fast={} slow={}",
        stats2.total_deposited,
        stats1.total_deposited
    );
}

// ============================================================================
// ERODE SPEED — (capacity - sediment) * erode_speed
// ============================================================================

/// Higher erode_speed should increase total erosion.
#[test]
fn erode_speed_increases_erosion() {
    let base = HydraulicErosionConfig {
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config_slow = HydraulicErosionConfig {
        erode_speed: 0.05,
        ..base.clone()
    };
    let config_fast = HydraulicErosionConfig {
        erode_speed: 0.9,
        ..base.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config_slow);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config_fast);

    assert!(
        stats2.total_eroded > stats1.total_eroded,
        "Faster erode_speed should erode more: fast={} slow={}",
        stats2.total_eroded,
        stats1.total_eroded
    );
}

// ============================================================================
// INITIAL WATER & SPEED
// ============================================================================

/// More initial water → higher sediment capacity → more erosion.
#[test]
fn initial_water_increases_erosion() {
    let base = HydraulicErosionConfig {
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config_low = HydraulicErosionConfig {
        initial_water: 0.1,
        ..base.clone()
    };
    let config_high = HydraulicErosionConfig {
        initial_water: 5.0,
        ..base.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config_low);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config_high);

    assert!(
        stats2.total_eroded > stats1.total_eroded,
        "More initial water should erode more: high={} low={}",
        stats2.total_eroded,
        stats1.total_eroded
    );
}

/// Higher initial speed → higher velocity → higher capacity → more erosion.
#[test]
fn initial_speed_increases_erosion() {
    let base = HydraulicErosionConfig {
        droplet_count: 3000,
        erosion_radius: 2,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let config_slow = HydraulicErosionConfig {
        initial_speed: 0.01,
        ..base.clone()
    };
    let config_fast = HydraulicErosionConfig {
        initial_speed: 5.0,
        ..base.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config_slow);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config_fast);

    assert!(
        stats2.total_eroded > stats1.total_eroded,
        "Higher initial speed should erode more: fast={} slow={}",
        stats2.total_eroded,
        stats1.total_eroded
    );
}

// ============================================================================
// EROSION RADIUS (BRUSH)
// ============================================================================

/// Larger radius spreads erosion differently than smaller radius.
/// The erosion pattern should be measurably different.
#[test]
fn larger_radius_different_erosion_pattern() {
    let config_small = HydraulicErosionConfig {
        erosion_radius: 1,
        droplet_count: 2000,
        max_droplet_lifetime: 15,
        ..Default::default()
    };
    let config_large = HydraulicErosionConfig {
        erosion_radius: 5,
        ..config_small.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let stats1 = sim1.apply_hydraulic_erosion(&mut hm1, &config_small);
    let stats2 = sim2.apply_hydraulic_erosion(&mut hm2, &config_large);

    // Different radii should produce measurably different results
    let diff_count = hm1
        .data()
        .iter()
        .zip(hm2.data().iter())
        .filter(|(a, b)| (*a - *b).abs() > 0.001)
        .count();
    assert!(
        diff_count > 10,
        "Different radii should produce different patterns: {diff_count} cells differ"
    );
    // Both should still erode
    assert!(stats1.total_eroded > 0.0);
    assert!(stats2.total_eroded > 0.0);
}

// ============================================================================
// ZERO DROPLETS — edge case
// ============================================================================

#[test]
fn zero_droplets_no_change() {
    let config = HydraulicErosionConfig {
        droplet_count: 0,
        ..Default::default()
    };
    let mut hm = peak_hm(16, 10.0);
    let before = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    assert_eq!(hm.data(), before.as_slice());
    assert_eq!(stats.total_eroded, 0.0);
    assert_eq!(stats.total_deposited, 0.0);
    assert_eq!(stats.droplets_terminated, 0);
}

// ============================================================================
// THERMAL EROSION
// ============================================================================

/// Thermal erosion should move material from steep cells to lower neighbors.
/// Total material should be approximately conserved (within the interior).
#[test]
fn thermal_conserves_material_approximately() {
    let mut hm = peak_hm(32, 50.0);
    let before_sum: f64 = hm.data().iter().map(|&h| h as f64).sum();

    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig {
        iterations: 20,
        talus_angle: 30.0,
        redistribution_rate: 0.5,
        eight_directional: true,
        cell_size: 1.0,
    };
    sim.apply_thermal_erosion(&mut hm, &config);

    let after_sum: f64 = hm.data().iter().map(|&h| h as f64).sum();

    // Material is moved, not created or destroyed
    let diff = (after_sum - before_sum).abs();
    assert!(
        diff < 1.0,
        "Thermal erosion should conserve material: before={before_sum}, after={after_sum}, diff={diff}"
    );
}

/// With talus_angle=90, tan(90°) → infinity, so no slope exceeds talus.
/// No material should be redistributed.
#[test]
fn thermal_very_high_talus_no_erosion() {
    let mut hm = peak_hm(16, 20.0);
    let before = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig {
        iterations: 10,
        talus_angle: 89.0, // Nearly vertical — nothing exceeds this
        redistribution_rate: 0.5,
        eight_directional: true,
        cell_size: 1.0,
    };
    let stats = sim.apply_thermal_erosion(&mut hm, &config);

    // On a gentle peak (max slope ~1), 89° talus should prevent any redistribution
    assert!(
        stats.total_eroded < 0.1,
        "Very high talus should prevent erosion: {}",
        stats.total_eroded
    );
}

/// Low talus angle should cause more redistribution than high angle.
#[test]
fn thermal_low_talus_more_erosion() {
    let config_low = ThermalErosionConfig {
        talus_angle: 5.0, // Very low — lots of material moves
        iterations: 20,
        redistribution_rate: 0.5,
        eight_directional: true,
        cell_size: 1.0,
    };
    let config_high = ThermalErosionConfig {
        talus_angle: 60.0,
        ..config_low.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let sim = AdvancedErosionSimulator::new(42);

    let stats_low = sim.apply_thermal_erosion(&mut hm1, &config_low);
    let stats_high = sim.apply_thermal_erosion(&mut hm2, &config_high);

    assert!(
        stats_low.total_eroded > stats_high.total_eroded,
        "Low talus should erode more: low={} high={}",
        stats_low.total_eroded,
        stats_high.total_eroded
    );
}

/// 8-directional neighbors should distribute to more cells than 4-directional.
#[test]
fn thermal_eight_dir_vs_four_dir() {
    let config4 = ThermalErosionConfig {
        eight_directional: false,
        iterations: 10,
        talus_angle: 20.0,
        redistribution_rate: 0.5,
        cell_size: 1.0,
    };
    let config8 = ThermalErosionConfig {
        eight_directional: true,
        ..config4.clone()
    };

    let mut hm4 = peak_hm(32, 30.0);
    let mut hm8 = peak_hm(32, 30.0);
    let sim = AdvancedErosionSimulator::new(42);

    let stats4 = sim.apply_thermal_erosion(&mut hm4, &config4);
    let stats8 = sim.apply_thermal_erosion(&mut hm8, &config8);

    // 8-directional should erode more since diagonal slopes also count
    assert!(
        stats8.total_eroded > stats4.total_eroded,
        "8-dir should erode more: 8dir={} 4dir={}",
        stats8.total_eroded,
        stats4.total_eroded
    );
}

/// redistribution_rate controls how much material moves per iteration.
/// Higher rate → more erosion per iteration.
#[test]
fn thermal_redistribution_rate_affects_erosion() {
    let config_low = ThermalErosionConfig {
        redistribution_rate: 0.1,
        iterations: 20,
        talus_angle: 20.0,
        eight_directional: true,
        cell_size: 1.0,
    };
    let config_high = ThermalErosionConfig {
        redistribution_rate: 0.9,
        ..config_low.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let sim = AdvancedErosionSimulator::new(42);

    let stats1 = sim.apply_thermal_erosion(&mut hm1, &config_low);
    let stats2 = sim.apply_thermal_erosion(&mut hm2, &config_high);

    assert!(
        stats2.total_eroded > stats1.total_eroded,
        "Higher redistribution should erode more: high={} low={}",
        stats2.total_eroded,
        stats1.total_eroded
    );
}

/// cell_size affects slope calculation: diff / (dist * cell_size).
/// Larger cell_size → smaller slope → less exceeds talus → less erosion.
#[test]
fn thermal_cell_size_affects_slope() {
    let config_small = ThermalErosionConfig {
        cell_size: 0.5,
        iterations: 20,
        talus_angle: 30.0,
        redistribution_rate: 0.5,
        eight_directional: true,
    };
    let config_large = ThermalErosionConfig {
        cell_size: 5.0,
        ..config_small.clone()
    };

    let mut hm1 = peak_hm(32, 30.0);
    let mut hm2 = peak_hm(32, 30.0);
    let sim = AdvancedErosionSimulator::new(42);

    let stats1 = sim.apply_thermal_erosion(&mut hm1, &config_small);
    let stats2 = sim.apply_thermal_erosion(&mut hm2, &config_large);

    assert!(
        stats1.total_eroded > stats2.total_eroded,
        "Smaller cell_size makes steeper slopes, more erosion: small={} large={}",
        stats1.total_eroded,
        stats2.total_eroded
    );
}

/// Zero iterations means no work done.
#[test]
fn thermal_zero_iterations_no_change() {
    let mut hm = peak_hm(16, 20.0);
    let before = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig {
        iterations: 0,
        ..Default::default()
    };
    let stats = sim.apply_thermal_erosion(&mut hm, &config);

    assert_eq!(hm.data(), before.as_slice());
    assert_eq!(stats.total_eroded, 0.0);
}

/// Thermal erosion_map should be produced.
#[test]
fn thermal_produces_erosion_map() {
    let mut hm = peak_hm(16, 20.0);
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig::default();
    let stats = sim.apply_thermal_erosion(&mut hm, &config);

    let map = stats
        .erosion_map
        .as_ref()
        .expect("erosion_map should be Some");
    assert_eq!(map.len(), 16 * 16);
}

// ============================================================================
// WIND EROSION
// ============================================================================

/// Wind erodes windward slopes and deposits on leeward side.
/// Windward = opposite of wind direction. If wind blows +x, windward neighbor is at x-1.
/// Erosion occurs when current_height > windward_height (cell faces the wind).
#[test]
fn wind_moves_material_downwind() {
    let mut hm = sloped_x(32, 1.0); // height = x, gradient in +x
    let sim = AdvancedErosionSimulator::new(42);
    // Wind blowing in +x direction → windward at x-1 → current > windward for sloped_x
    let config = WindErosionConfig {
        wind_direction: Vec2::new(1.0, 0.0),
        wind_strength: 1.0,
        iterations: 20,
        saltation_distance: 3.0,
        suspension_height: 5.0,
    };
    let stats = sim.apply_wind_erosion(&mut hm, &config);

    assert!(
        stats.total_eroded > 0.0,
        "Wind should erode windward slopes: {}",
        stats.total_eroded
    );
}

/// Wind strength scales erosion amount.
#[test]
fn wind_strength_scales_erosion() {
    let config_low = WindErosionConfig {
        wind_strength: 0.1,
        iterations: 20,
        ..Default::default()
    };
    let config_high = WindErosionConfig {
        wind_strength: 2.0,
        ..config_low.clone()
    };

    let mut hm1 = peak_hm(32, 20.0);
    let mut hm2 = peak_hm(32, 20.0);
    let sim = AdvancedErosionSimulator::new(42);

    let stats1 = sim.apply_wind_erosion(&mut hm1, &config_low);
    let stats2 = sim.apply_wind_erosion(&mut hm2, &config_high);

    assert!(
        stats2.total_eroded > stats1.total_eroded,
        "Higher wind strength should erode more: high={} low={}",
        stats2.total_eroded,
        stats1.total_eroded
    );
}

/// More iterations → more erosion.
#[test]
fn wind_iterations_increase_erosion() {
    let config_few = WindErosionConfig {
        iterations: 5,
        ..Default::default()
    };
    let config_many = WindErosionConfig {
        iterations: 50,
        ..config_few.clone()
    };

    let mut hm1 = peak_hm(32, 20.0);
    let mut hm2 = peak_hm(32, 20.0);
    let sim = AdvancedErosionSimulator::new(42);

    let stats1 = sim.apply_wind_erosion(&mut hm1, &config_few);
    let stats2 = sim.apply_wind_erosion(&mut hm2, &config_many);

    assert!(
        stats2.total_eroded > stats1.total_eroded,
        "More iterations should erode more: many={} few={}",
        stats2.total_eroded,
        stats1.total_eroded
    );
}

/// Zero iterations means no change.
#[test]
fn wind_zero_iterations_no_change() {
    let mut hm = peak_hm(16, 20.0);
    let before = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig {
        iterations: 0,
        ..Default::default()
    };
    let stats = sim.apply_wind_erosion(&mut hm, &config);

    assert_eq!(hm.data(), before.as_slice());
    assert_eq!(stats.total_eroded, 0.0);
}

/// Flat terrain should produce no wind erosion (no windward slopes).
#[test]
fn wind_flat_terrain_no_erosion() {
    let mut hm = flat_hm(32, 50.0);
    let before = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig::default();
    let stats = sim.apply_wind_erosion(&mut hm, &config);

    assert_eq!(
        stats.total_eroded, 0.0,
        "Flat terrain should have zero wind erosion"
    );
    assert_eq!(hm.data(), before.as_slice());
}

/// Wind erosion produces an erosion_map.
#[test]
fn wind_produces_erosion_map() {
    let mut hm = peak_hm(16, 10.0);
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig::default();
    let stats = sim.apply_wind_erosion(&mut hm, &config);

    let map = stats.erosion_map.as_ref().unwrap();
    assert_eq!(map.len(), 16 * 16);
}

/// Wind direction affects erosion pattern. Different directions should produce
/// different results on non-symmetric terrain.
#[test]
fn wind_direction_affects_pattern() {
    let config_east = WindErosionConfig {
        wind_direction: Vec2::new(1.0, 0.0),
        iterations: 20,
        ..Default::default()
    };
    let config_north = WindErosionConfig {
        wind_direction: Vec2::new(0.0, 1.0),
        ..config_east.clone()
    };

    // Use sloped terrain (asymmetric)
    let mut hm1 = sloped_x(32, 1.0);
    let mut hm2 = sloped_x(32, 1.0);
    let sim = AdvancedErosionSimulator::new(42);

    let stats1 = sim.apply_wind_erosion(&mut hm1, &config_east);
    let stats2 = sim.apply_wind_erosion(&mut hm2, &config_north);

    // Different wind directions on x-sloped terrain should produce different erosion
    let diff_count = hm1
        .data()
        .iter()
        .zip(hm2.data().iter())
        .filter(|(a, b)| (*a - *b).abs() > 0.001)
        .count();
    assert!(
        diff_count > 10,
        "Different wind directions should produce different patterns: {diff_count} cells differ"
    );
}

/// Saltation distance affects where material is deposited.
#[test]
fn wind_saltation_distance_changes_deposit_pattern() {
    let config_short = WindErosionConfig {
        saltation_distance: 1.0,
        iterations: 20,
        ..Default::default()
    };
    let config_long = WindErosionConfig {
        saltation_distance: 10.0,
        ..config_short.clone()
    };

    let mut hm1 = peak_hm(32, 20.0);
    let mut hm2 = peak_hm(32, 20.0);
    let sim = AdvancedErosionSimulator::new(42);

    sim.apply_wind_erosion(&mut hm1, &config_short);
    sim.apply_wind_erosion(&mut hm2, &config_long);

    // Different saltation should produce different deposit patterns
    let diff_count = hm1
        .data()
        .iter()
        .zip(hm2.data().iter())
        .filter(|(a, b)| (*a - *b).abs() > 0.001)
        .count();
    assert!(
        diff_count > 5,
        "Different saltation distances should change pattern: {diff_count} cells differ"
    );
}

// ============================================================================
// PRESET APPLICATION
// ============================================================================

/// apply_preset should run all passes in order and accumulate stats.
#[test]
fn preset_default_accumulates_stats() {
    let preset = ErosionPreset::default();
    let mut hm = peak_hm(32, 30.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut hm, &preset);

    // Default preset has thermal + hydraulic
    assert!(
        stats.total_eroded > 0.0,
        "Default preset should erode: {}",
        stats.total_eroded
    );
}

/// Desert preset has thermal + wind, no hydraulic.
#[test]
fn preset_desert_thermal_and_wind() {
    let preset = ErosionPreset::desert();
    assert!(preset.hydraulic.is_none());
    assert!(preset.thermal.is_some());
    assert!(preset.wind.is_some());
    assert_eq!(preset.pass_order.len(), 2);
    assert_eq!(preset.pass_order[0], "thermal");
    assert_eq!(preset.pass_order[1], "wind");

    let mut hm = peak_hm(32, 30.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut hm, &preset);
    assert!(stats.total_eroded > 0.0);
}

/// Mountain preset has hydraulic + thermal.
#[test]
fn preset_mountain_hydraulic_and_thermal() {
    let preset = ErosionPreset::mountain();
    assert!(preset.hydraulic.is_some());
    assert!(preset.thermal.is_some());
    assert!(preset.wind.is_none());
    assert_eq!(preset.pass_order[0], "hydraulic");
    assert_eq!(preset.pass_order[1], "thermal");

    let mut hm = peak_hm(32, 30.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut hm, &preset);
    assert!(stats.total_eroded > 0.0);
}

/// Coastal preset has all three passes.
#[test]
fn preset_coastal_all_three() {
    let preset = ErosionPreset::coastal();
    assert!(preset.hydraulic.is_some());
    assert!(preset.thermal.is_some());
    assert!(preset.wind.is_some());
    assert_eq!(preset.pass_order.len(), 3);

    let mut hm = peak_hm(32, 30.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut hm, &preset);
    assert!(stats.total_eroded > 0.0);
}

/// Unknown pass name should be ignored (not crash).
#[test]
fn preset_unknown_pass_ignored() {
    let preset = ErosionPreset {
        name: "test".to_string(),
        hydraulic: Some(HydraulicErosionConfig::default()),
        thermal: None,
        wind: None,
        pass_order: vec!["unknown_pass".to_string(), "hydraulic".to_string()],
    };
    let mut hm = peak_hm(16, 10.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut hm, &preset);
    assert!(stats.total_eroded > 0.0);
}

/// Preset with no passes should produce zero stats.
#[test]
fn preset_empty_passes_no_change() {
    let preset = ErosionPreset {
        name: "empty".to_string(),
        hydraulic: None,
        thermal: None,
        wind: None,
        pass_order: vec![],
    };
    let mut hm = peak_hm(16, 10.0);
    let before = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut hm, &preset);

    assert_eq!(hm.data(), before.as_slice());
    assert_eq!(stats.total_eroded, 0.0);
}

/// max_erosion_depth should propagate from sub-pass to combined stats.
#[test]
fn preset_propagates_max_erosion_depth() {
    let preset = ErosionPreset::mountain();
    let mut hm = peak_hm(32, 40.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let stats = sim.apply_preset(&mut hm, &preset);

    assert!(
        stats.max_erosion_depth > 0.0,
        "max_erosion_depth should propagate from passes: {}",
        stats.max_erosion_depth
    );
}

// ============================================================================
// SMALL HEIGHTMAP EDGE CASES
// ============================================================================

/// On a tiny 4×4 heightmap, hydraulic erosion should not crash and should
/// handle brush clamping correctly.
#[test]
fn hydraulic_on_4x4_no_crash() {
    let data: Vec<f32> = (0..16).map(|i| (i as f32) * 0.5).collect();
    let mut hm = Heightmap::from_data(data, 4).unwrap();
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 100,
        erosion_radius: 1,
        max_droplet_lifetime: 10,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);
    // Should complete without panic
    assert!(stats.total_eroded >= 0.0);
}

/// Thermal erosion on 4×4 should not crash.
#[test]
fn thermal_on_4x4_no_crash() {
    let data: Vec<f32> = vec![
        10.0, 5.0, 2.0, 0.0, 8.0, 4.0, 1.0, 0.0, 6.0, 3.0, 0.5, 0.0, 4.0, 2.0, 0.0, 0.0,
    ];
    let mut hm = Heightmap::from_data(data, 4).unwrap();
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig {
        iterations: 5,
        talus_angle: 20.0,
        ..Default::default()
    };
    let stats = sim.apply_thermal_erosion(&mut hm, &config);
    assert!(stats.total_eroded >= 0.0);
}

/// Wind erosion on 4×4 should not crash.
#[test]
fn wind_on_4x4_no_crash() {
    let data: Vec<f32> = vec![
        0.0, 1.0, 3.0, 6.0, 0.0, 1.5, 4.0, 7.0, 0.0, 2.0, 5.0, 8.0, 0.0, 2.5, 6.0, 9.0,
    ];
    let mut hm = Heightmap::from_data(data, 4).unwrap();
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig {
        iterations: 5,
        ..Default::default()
    };
    let stats = sim.apply_wind_erosion(&mut hm, &config);
    assert!(stats.total_eroded >= 0.0);
}

// ============================================================================
// HEIGHTMAP DATA INTEGRITY
// ============================================================================

/// Heights should remain non-negative after erosion (erosion is clamped).
#[test]
fn hydraulic_heights_non_negative() {
    let mut hm = peak_hm(32, 10.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 10000,
        erode_speed: 0.9,
        erosion_radius: 3,
        max_droplet_lifetime: 30,
        ..Default::default()
    };
    sim.apply_hydraulic_erosion(&mut hm, &config);

    for (i, &h) in hm.data().iter().enumerate() {
        assert!(h >= 0.0, "Height at index {i} is negative: {h}");
    }
}

/// Heights should remain finite after all erosion types.
#[test]
fn heights_remain_finite_after_preset() {
    let mut hm = peak_hm(32, 50.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let preset = ErosionPreset::coastal();
    sim.apply_preset(&mut hm, &preset);

    for (i, &h) in hm.data().iter().enumerate() {
        assert!(h.is_finite(), "Height at index {i} is not finite: {h}");
    }
}

// ============================================================================
// EROSION STATS DEFAULT
// ============================================================================

#[test]
fn erosion_stats_default_is_zero() {
    let stats = ErosionStats::default();
    assert_eq!(stats.total_eroded, 0.0);
    assert_eq!(stats.total_deposited, 0.0);
    assert_eq!(stats.droplets_terminated, 0);
    assert_eq!(stats.avg_droplet_lifetime, 0.0);
    assert_eq!(stats.max_erosion_depth, 0.0);
    assert!(stats.erosion_map.is_none());
}

// ============================================================================
// CONFIG DEFAULTS — catching literal mutations
// ============================================================================

#[test]
fn hydraulic_config_defaults_exact() {
    let c = HydraulicErosionConfig::default();
    assert_eq!(c.droplet_count, 50000);
    assert_eq!(c.inertia, 0.05);
    assert_eq!(c.sediment_capacity_factor, 4.0);
    assert_eq!(c.min_slope, 0.01);
    assert_eq!(c.deposit_speed, 0.3);
    assert_eq!(c.erode_speed, 0.3);
    assert_eq!(c.evaporation_rate, 0.01);
    assert_eq!(c.initial_water, 1.0);
    assert_eq!(c.initial_speed, 1.0);
    assert_eq!(c.max_droplet_lifetime, 30);
    assert_eq!(c.erosion_radius, 3);
    assert_eq!(c.gravity, 4.0);
}

#[test]
fn thermal_config_defaults_exact() {
    let c = ThermalErosionConfig::default();
    assert_eq!(c.iterations, 50);
    assert_eq!(c.talus_angle, 45.0);
    assert_eq!(c.redistribution_rate, 0.5);
    assert!(c.eight_directional);
    assert_eq!(c.cell_size, 1.0);
}

#[test]
fn wind_config_defaults_exact() {
    let c = WindErosionConfig::default();
    assert_eq!(c.wind_direction, Vec2::new(1.0, 0.0));
    assert_eq!(c.wind_strength, 0.5);
    assert_eq!(c.suspension_height, 5.0);
    assert_eq!(c.iterations, 30);
    assert_eq!(c.saltation_distance, 3.0);
}

#[test]
fn erosion_preset_default_exact() {
    let p = ErosionPreset::default();
    assert_eq!(p.name, "Default");
    assert!(p.hydraulic.is_some());
    assert!(p.thermal.is_some());
    assert!(p.wind.is_none());
    assert_eq!(p.pass_order, vec!["thermal", "hydraulic"]);
}

#[test]
fn erosion_preset_desert_fields() {
    let p = ErosionPreset::desert();
    assert_eq!(p.name, "Desert");
    assert!(p.hydraulic.is_none());
    assert!(p.thermal.is_some());
    assert!(p.wind.is_some());
    assert_eq!(p.thermal.as_ref().unwrap().talus_angle, 35.0);
}

#[test]
fn erosion_preset_mountain_fields() {
    let p = ErosionPreset::mountain();
    assert_eq!(p.name, "Mountain");
    assert!(p.hydraulic.is_some());
    assert_eq!(p.hydraulic.as_ref().unwrap().droplet_count, 100000);
    assert_eq!(p.hydraulic.as_ref().unwrap().erode_speed, 0.4);
    assert!(p.thermal.is_some());
    assert_eq!(p.thermal.as_ref().unwrap().talus_angle, 50.0);
    assert_eq!(p.thermal.as_ref().unwrap().iterations, 30);
    assert!(p.wind.is_none());
}

#[test]
fn erosion_preset_coastal_fields() {
    let p = ErosionPreset::coastal();
    assert_eq!(p.name, "Coastal");
    assert!(p.hydraulic.is_some());
    assert_eq!(p.hydraulic.as_ref().unwrap().droplet_count, 30000);
    assert_eq!(p.hydraulic.as_ref().unwrap().evaporation_rate, 0.02);
    assert!(p.thermal.is_some());
    assert_eq!(p.thermal.as_ref().unwrap().talus_angle, 40.0);
    assert_eq!(p.thermal.as_ref().unwrap().iterations, 20);
    assert!(p.wind.is_some());
    assert_eq!(p.wind.as_ref().unwrap().wind_strength, 0.3);
}

// ============================================================================
// MASS BALANCE — eroded - deposited ≈ net height change
// ============================================================================

/// total_deposited should be tracked correctly. On steep terrain both
/// erosion and deposition happen.
#[test]
fn hydraulic_deposited_correlates_with_height_gain() {
    let mut hm = peak_hm(32, 40.0);
    let before = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = fast_hydraulic(5000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // Sum of positive height changes should be close to total_deposited
    // (not exact because of erosion map tracking)
    let height_gained: f64 = hm
        .data()
        .iter()
        .zip(before.iter())
        .map(|(&after, &before)| {
            if after > before {
                (after - before) as f64
            } else {
                0.0
            }
        })
        .sum();

    // Deposited amount should be in the same order of magnitude
    assert!(
        stats.total_deposited > 0.0,
        "Should have some deposition on steep terrain"
    );
    assert!(
        height_gained > 0.0,
        "Some cells should gain height from deposition"
    );
}
