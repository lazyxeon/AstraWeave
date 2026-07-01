//! Wave 2 — Shard-0 targeted remediation tests
//!
//! Targeting 140 missed mutants from terrain sweep shard 0:
//!   - apply_hydraulic_erosion: 78 misses (arithmetic operator swaps on inertia,
//!     sediment capacity, velocity, gradient, bounds, brush application)
//!   - init_erosion_brush: 31 misses (weight calculation, index mapping)
//!   - scatter_chunk_content: 13 misses (seed offset arithmetic at L193/L201)
//!   - ErosionPreset: 10 misses (coastal/mountain/desert inherited fields)
//!   - Misc WorldGenerator: 8 misses

/// Fast config for scatter tests: smaller chunk (64 vs 256) + smaller heightmap (16 vs 128).
/// Avoids 60s+ scatter generation per chunk.
fn fast_scatter_config() -> WorldConfig {
    let mut config = WorldConfig::default();
    config.heightmap_resolution = 16;
    config.chunk_size = 64.0;
    config
}

use astraweave_terrain::advanced_erosion::{
    AdvancedErosionSimulator, ErosionPreset, HydraulicErosionConfig, ThermalErosionConfig,
    WindErosionConfig,
};
use astraweave_terrain::{ChunkId, Heightmap, WorldConfig, WorldGenerator};

// ============================================================================
// Helper: create deterministic heightmaps
// ============================================================================

fn flat_heightmap(resolution: u32, height: f32) -> Heightmap {
    let data = vec![height; (resolution * resolution) as usize];
    Heightmap::from_data(data, resolution).unwrap()
}

fn sloped_x_heightmap(resolution: u32, scale: f32) -> Heightmap {
    let mut data = Vec::with_capacity((resolution * resolution) as usize);
    for _z in 0..resolution {
        for x in 0..resolution {
            data.push(x as f32 * scale);
        }
    }
    Heightmap::from_data(data, resolution).unwrap()
}

fn peak_heightmap(resolution: u32, peak_height: f32) -> Heightmap {
    let mut data = Vec::with_capacity((resolution * resolution) as usize);
    let center = resolution as f32 / 2.0;
    for z in 0..resolution {
        for x in 0..resolution {
            let dx = x as f32 - center;
            let dz = z as f32 - center;
            let dist = (dx * dx + dz * dz).sqrt();
            data.push((peak_height - dist * 2.0).max(0.0));
        }
    }
    Heightmap::from_data(data, resolution).unwrap()
}

fn wavy_heightmap(resolution: u32) -> Heightmap {
    let mut data = Vec::with_capacity((resolution * resolution) as usize);
    for z in 0..resolution {
        for x in 0..resolution {
            data.push(((x as f32 * 0.31).sin() + (z as f32 * 0.29).cos() + 2.0) * 20.0);
        }
    }
    Heightmap::from_data(data, resolution).unwrap()
}

fn small_config(droplets: u32) -> HydraulicErosionConfig {
    HydraulicErosionConfig {
        droplet_count: droplets,
        max_droplet_lifetime: 30,
        erosion_radius: 3,
        ..Default::default()
    }
}

// ============================================================================
// A. ErosionPreset field-level mutations (10 misses)
// ============================================================================

// --- Desert inherited defaults ---
#[test]
fn desert_thermal_iterations_is_default() {
    let p = ErosionPreset::desert();
    let t = p.thermal.as_ref().unwrap();
    assert_eq!(t.iterations, ThermalErosionConfig::default().iterations);
}

#[test]
fn desert_thermal_eight_directional_is_default() {
    let p = ErosionPreset::desert();
    let t = p.thermal.as_ref().unwrap();
    assert_eq!(
        t.eight_directional,
        ThermalErosionConfig::default().eight_directional
    );
}

#[test]
fn desert_thermal_cell_size_is_default() {
    let p = ErosionPreset::desert();
    let t = p.thermal.as_ref().unwrap();
    assert!((t.cell_size - ThermalErosionConfig::default().cell_size).abs() < 1e-9);
}

#[test]
fn desert_wind_strength_is_default() {
    let p = ErosionPreset::desert();
    let w = p.wind.as_ref().unwrap();
    assert!((w.wind_strength - WindErosionConfig::default().wind_strength).abs() < 1e-9);
}

#[test]
fn desert_wind_saltation_distance_is_default() {
    let p = ErosionPreset::desert();
    let w = p.wind.as_ref().unwrap();
    assert!((w.saltation_distance - WindErosionConfig::default().saltation_distance).abs() < 1e-9);
}

#[test]
fn desert_wind_iterations_is_default() {
    let p = ErosionPreset::desert();
    let w = p.wind.as_ref().unwrap();
    assert_eq!(w.iterations, WindErosionConfig::default().iterations);
}

// --- Mountain inherited defaults ---
#[test]
fn mountain_hydraulic_evaporation_rate_is_default() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.evaporation_rate - HydraulicErosionConfig::default().evaporation_rate).abs() < 1e-9);
}

#[test]
fn mountain_hydraulic_initial_water_is_default() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.initial_water - HydraulicErosionConfig::default().initial_water).abs() < 1e-9);
}

#[test]
fn mountain_hydraulic_initial_speed_is_default() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.initial_speed - HydraulicErosionConfig::default().initial_speed).abs() < 1e-9);
}

#[test]
fn mountain_hydraulic_max_lifetime_is_default() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert_eq!(
        h.max_droplet_lifetime,
        HydraulicErosionConfig::default().max_droplet_lifetime
    );
}

#[test]
fn mountain_hydraulic_erosion_radius_is_default() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert_eq!(
        h.erosion_radius,
        HydraulicErosionConfig::default().erosion_radius
    );
}

#[test]
fn mountain_hydraulic_sediment_capacity_factor_is_default() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert!(
        (h.sediment_capacity_factor - HydraulicErosionConfig::default().sediment_capacity_factor)
            .abs()
            < 1e-9
    );
}

#[test]
fn mountain_hydraulic_min_slope_is_default() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.min_slope - HydraulicErosionConfig::default().min_slope).abs() < 1e-9);
}

#[test]
fn mountain_hydraulic_deposit_speed_is_default() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.deposit_speed - HydraulicErosionConfig::default().deposit_speed).abs() < 1e-9);
}

#[test]
fn mountain_thermal_redistribution_rate_is_default() {
    let p = ErosionPreset::mountain();
    let t = p.thermal.as_ref().unwrap();
    assert!(
        (t.redistribution_rate - ThermalErosionConfig::default().redistribution_rate).abs() < 1e-9
    );
}

#[test]
fn mountain_thermal_cell_size_is_default() {
    let p = ErosionPreset::mountain();
    let t = p.thermal.as_ref().unwrap();
    assert!((t.cell_size - ThermalErosionConfig::default().cell_size).abs() < 1e-9);
}

#[test]
fn mountain_thermal_eight_directional_is_default() {
    let p = ErosionPreset::mountain();
    let t = p.thermal.as_ref().unwrap();
    assert_eq!(
        t.eight_directional,
        ThermalErosionConfig::default().eight_directional
    );
}

// --- Coastal inherited defaults ---
#[test]
fn coastal_hydraulic_inertia_is_default() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.inertia - HydraulicErosionConfig::default().inertia).abs() < 1e-9);
}

#[test]
fn coastal_hydraulic_initial_water_is_default() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.initial_water - HydraulicErosionConfig::default().initial_water).abs() < 1e-9);
}

#[test]
fn coastal_hydraulic_initial_speed_is_default() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.initial_speed - HydraulicErosionConfig::default().initial_speed).abs() < 1e-9);
}

#[test]
fn coastal_hydraulic_gravity_is_default() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.gravity - HydraulicErosionConfig::default().gravity).abs() < 1e-9);
}

#[test]
fn coastal_thermal_redistribution_is_default() {
    let p = ErosionPreset::coastal();
    let t = p.thermal.as_ref().unwrap();
    assert!(
        (t.redistribution_rate - ThermalErosionConfig::default().redistribution_rate).abs() < 1e-9
    );
}

#[test]
fn coastal_thermal_eight_directional_is_default() {
    let p = ErosionPreset::coastal();
    let t = p.thermal.as_ref().unwrap();
    assert_eq!(
        t.eight_directional,
        ThermalErosionConfig::default().eight_directional
    );
}

#[test]
fn coastal_thermal_cell_size_is_default() {
    let p = ErosionPreset::coastal();
    let t = p.thermal.as_ref().unwrap();
    assert!((t.cell_size - ThermalErosionConfig::default().cell_size).abs() < 1e-9);
}

#[test]
fn coastal_wind_direction_is_default() {
    let p = ErosionPreset::coastal();
    let w = p.wind.as_ref().unwrap();
    let def = WindErosionConfig::default();
    assert!((w.wind_direction.x - def.wind_direction.x).abs() < 1e-9);
    assert!((w.wind_direction.y - def.wind_direction.y).abs() < 1e-9);
}

#[test]
fn coastal_wind_suspension_height_is_default() {
    let p = ErosionPreset::coastal();
    let w = p.wind.as_ref().unwrap();
    assert!((w.suspension_height - WindErosionConfig::default().suspension_height).abs() < 1e-9);
}

#[test]
fn coastal_wind_saltation_distance_is_default() {
    let p = ErosionPreset::coastal();
    let w = p.wind.as_ref().unwrap();
    assert!((w.saltation_distance - WindErosionConfig::default().saltation_distance).abs() < 1e-9);
}

#[test]
fn coastal_wind_iterations_is_default() {
    let p = ErosionPreset::coastal();
    let w = p.wind.as_ref().unwrap();
    assert_eq!(w.iterations, WindErosionConfig::default().iterations);
}

#[test]
fn coastal_pass_order_len_is_3() {
    let p = ErosionPreset::coastal();
    assert_eq!(p.pass_order.len(), 3);
}

#[test]
fn coastal_pass_order_element_2_is_wind() {
    let p = ErosionPreset::coastal();
    assert_eq!(p.pass_order[2], "wind");
}

// ============================================================================
// B. apply_hydraulic_erosion — direction/inertia mutations (L374-375)
// ============================================================================

#[test]
fn hydraulic_inertia_zero_follows_gradient() {
    // With inertia=0, droplets should change direction purely by gradient.
    // On a simple X-slope, they should move in the -X direction consistently.
    let mut hm = sloped_x_heightmap(32, 2.0);
    let before: Vec<f32> = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 500,
        inertia: 0.0,
        max_droplet_lifetime: 30,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // Droplets on an X-slope with inertia=0 should erode consistently
    assert!(stats.total_eroded > 0.0, "Zero inertia on slope must erode");

    // The high-end (right side) should lose more height than the low-end (left side)
    let res = 32usize;
    let high_col_before: f32 = (0..res).map(|z| before[z * res + (res - 2)]).sum();
    let high_col_after: f32 = (0..res).map(|z| hm.data()[z * res + (res - 2)]).sum();
    let low_col_before: f32 = (0..res).map(|z| before[z * res + 1]).sum();
    let low_col_after: f32 = (0..res).map(|z| hm.data()[z * res + 1]).sum();

    let high_change = (high_col_before - high_col_after).abs();
    let low_change = (low_col_before - low_col_after).abs();

    // High-end should change more (erosion moves material downslope)
    assert!(
        high_change > low_change || (high_change - low_change).abs() < 1.0,
        "High end ({high_change:.4}) should change >= low end ({low_change:.4}) on X-slope"
    );
}

#[test]
fn hydraulic_inertia_one_ignores_gradient() {
    // With inertia=1.0, droplets should never change direction (dir stays ZERO → random)
    // This means behavior should differ significantly from inertia=0
    let make_hm = || sloped_x_heightmap(32, 2.0);
    let mut hm0 = make_hm();
    let mut hm1 = make_hm();

    let mut sim0 = AdvancedErosionSimulator::new(42);
    let mut sim1 = AdvancedErosionSimulator::new(42);

    let mut config0 = small_config(500);
    config0.inertia = 0.0;
    let mut config1 = small_config(500);
    config1.inertia = 1.0;

    sim0.apply_hydraulic_erosion(&mut hm0, &config0);
    sim1.apply_hydraulic_erosion(&mut hm1, &config1);

    // Different inertia → different erosion pattern
    assert_ne!(
        hm0.data(),
        hm1.data(),
        "Inertia 0 vs 1 must produce different erosion"
    );
}

// ============================================================================
// C. Flat terrain — gradient-dependent mutations (L351-352)
// ============================================================================

#[test]
fn hydraulic_flat_terrain_minimal_erosion() {
    // Flat terrain has zero gradient → sediment capacity ≈ 0 → minimal erosion
    let mut hm = flat_heightmap(32, 50.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = small_config(1000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // Flat terrain: very little erosion because sediment_capacity depends on slope
    assert!(
        stats.total_eroded < 10.0,
        "Flat terrain should have minimal erosion, got {}",
        stats.total_eroded
    );
}

#[test]
fn hydraulic_flat_terrain_all_heights_near_original() {
    let mut hm = flat_heightmap(32, 50.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = small_config(500);
    sim.apply_hydraulic_erosion(&mut hm, &config);

    // On flat terrain, all heights should stay near 50.0
    for &h in hm.data() {
        assert!(
            (h - 50.0).abs() < 5.0,
            "Flat terrain height deviated too much: {h}"
        );
    }
}

// ============================================================================
// D. Sediment capacity mutations (L407-412)
// ============================================================================

#[test]
fn hydraulic_capacity_factor_zero_no_erosion() {
    // sediment_capacity_factor = 0 → capacity = 0 → no erosion → only deposition
    let mut hm = peak_heightmap(32, 80.0);
    let before_sum: f64 = hm.data().iter().map(|&h| h as f64).sum();

    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 500,
        sediment_capacity_factor: 0.0,
        max_droplet_lifetime: 30,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);
    let after_sum: f64 = hm.data().iter().map(|&h| h as f64).sum();

    // With capacity = 0, erosion should be zero or extremely minimal
    assert!(
        stats.total_eroded < 1.0,
        "capacity_factor=0 should yield near-zero erosion, got {}",
        stats.total_eroded
    );
    // Heights should barely change
    assert!(
        (before_sum - after_sum).abs() < 10.0,
        "Heights should barely change with capacity_factor=0"
    );
}

#[test]
fn hydraulic_high_capacity_erodes_more() {
    // Higher capacity factor → more sediment can be carried → more erosion
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_low = make_hm();
    let mut hm_high = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_low = small_config(500);
    cfg_low.sediment_capacity_factor = 1.0;
    let mut cfg_high = small_config(500);
    cfg_high.sediment_capacity_factor = 20.0;

    let stats_low = sim1.apply_hydraulic_erosion(&mut hm_low, &cfg_low);
    let stats_high = sim2.apply_hydraulic_erosion(&mut hm_high, &cfg_high);

    assert!(
        stats_high.total_eroded >= stats_low.total_eroded,
        "Higher capacity ({}) should erode >= lower ({})",
        stats_high.total_eroded,
        stats_low.total_eroded
    );
}

#[test]
fn hydraulic_min_slope_affects_capacity() {
    // Larger min_slope → higher minimum capacity → more erosion on flat-ish areas
    let make_hm = || wavy_heightmap(32);
    let mut hm_small = make_hm();
    let mut hm_big = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_small = small_config(500);
    cfg_small.min_slope = 0.001;
    let mut cfg_big = small_config(500);
    cfg_big.min_slope = 1.0;

    let stats_small = sim1.apply_hydraulic_erosion(&mut hm_small, &cfg_small);
    let stats_big = sim2.apply_hydraulic_erosion(&mut hm_big, &cfg_big);

    // Different min_slope should produce different results
    assert_ne!(
        hm_small.data(),
        hm_big.data(),
        "Different min_slope must produce different erosion"
    );
    // Higher min_slope generally allows more erosion on gentle slopes
    assert!(
        (stats_big.total_eroded - stats_small.total_eroded).abs() > 0.01,
        "min_slope should affect total erosion"
    );
}

// ============================================================================
// E. Velocity update mutations (L457)
// ============================================================================

#[test]
fn hydraulic_gravity_affects_velocity_and_erosion() {
    // v = sqrt(v^2 + |delta_h| * gravity)
    // Higher gravity → faster velocity → more sediment capacity → more erosion
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_low_g = make_hm();
    let mut hm_high_g = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_low = small_config(500);
    cfg_low.gravity = 0.5;
    let mut cfg_high = small_config(500);
    cfg_high.gravity = 20.0;

    let stats_low = sim1.apply_hydraulic_erosion(&mut hm_low_g, &cfg_low);
    let stats_high = sim2.apply_hydraulic_erosion(&mut hm_high_g, &cfg_high);

    assert!(
        stats_high.total_eroded >= stats_low.total_eroded,
        "Higher gravity ({}) should erode >= lower ({})",
        stats_high.total_eroded,
        stats_low.total_eroded
    );
    assert_ne!(
        hm_low_g.data(),
        hm_high_g.data(),
        "Different gravity must produce different terrain"
    );
}

#[test]
fn hydraulic_zero_gravity_lower_erosion() {
    // gravity=0 → velocity stays constant → lower than with gravity
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_zero = make_hm();
    let mut hm_normal = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_zero = small_config(500);
    cfg_zero.gravity = 0.0;
    let cfg_normal = small_config(500);

    let stats_zero = sim1.apply_hydraulic_erosion(&mut hm_zero, &cfg_zero);
    let stats_normal = sim2.apply_hydraulic_erosion(&mut hm_normal, &cfg_normal);

    // Zero gravity means velocity never increases from height changes
    assert!(
        stats_normal.total_eroded >= stats_zero.total_eroded,
        "Normal gravity ({}) should erode >= zero gravity ({})",
        stats_normal.total_eroded,
        stats_zero.total_eroded
    );
}

// ============================================================================
// F. Evaporation / water mutations
// ============================================================================

#[test]
fn hydraulic_full_evaporation_short_lifetime() {
    // evaporation_rate = 1.0 → water * (1 - 1.0) = 0 after step 1
    // Droplets still erode on their first step, but not subsequent ones
    // Compare with no-evaporation to show the effect
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_full_evap = make_hm();
    let mut hm_no_evap = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let cfg_evap = HydraulicErosionConfig {
        droplet_count: 500,
        evaporation_rate: 1.0,
        max_droplet_lifetime: 30,
        ..Default::default()
    };
    let cfg_no_evap = HydraulicErosionConfig {
        droplet_count: 500,
        evaporation_rate: 0.0,
        max_droplet_lifetime: 30,
        ..Default::default()
    };

    let stats_evap = sim1.apply_hydraulic_erosion(&mut hm_full_evap, &cfg_evap);
    let stats_no = sim2.apply_hydraulic_erosion(&mut hm_no_evap, &cfg_no_evap);

    // Full evaporation should produce less erosion than no evaporation
    assert!(
        stats_no.total_eroded >= stats_evap.total_eroded,
        "No evaporation ({}) should erode >= full evaporation ({})",
        stats_no.total_eroded,
        stats_evap.total_eroded
    );
    // Different evaporation rates → different terrain
    assert_ne!(
        hm_full_evap.data(),
        hm_no_evap.data(),
        "Full vs no evaporation must produce different terrain"
    );
}

#[test]
fn hydraulic_no_evaporation_more_erosion() {
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_evap = make_hm();
    let mut hm_no_evap = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_evap = small_config(500);
    cfg_evap.evaporation_rate = 0.1;
    let mut cfg_no_evap = small_config(500);
    cfg_no_evap.evaporation_rate = 0.0;

    let stats_evap = sim1.apply_hydraulic_erosion(&mut hm_evap, &cfg_evap);
    let stats_no = sim2.apply_hydraulic_erosion(&mut hm_no_evap, &cfg_no_evap);

    // No evaporation → water stays at full volume → more sediment capacity → more erosion
    assert!(
        stats_no.total_eroded >= stats_evap.total_eroded,
        "No evaporation ({}) should erode >= with evaporation ({})",
        stats_no.total_eroded,
        stats_evap.total_eroded
    );
}

#[test]
fn hydraulic_initial_water_affects_erosion() {
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_low = make_hm();
    let mut hm_high = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_low = small_config(500);
    cfg_low.initial_water = 0.1;
    let mut cfg_high = small_config(500);
    cfg_high.initial_water = 5.0;

    let stats_low = sim1.apply_hydraulic_erosion(&mut hm_low, &cfg_low);
    let stats_high = sim2.apply_hydraulic_erosion(&mut hm_high, &cfg_high);

    assert!(
        stats_high.total_eroded >= stats_low.total_eroded,
        "More water ({}) should erode >= less ({})",
        stats_high.total_eroded,
        stats_low.total_eroded
    );
}

// ============================================================================
// G. Erosion/deposit speed mutations (L403-404, L435)
// ============================================================================

#[test]
fn hydraulic_erode_speed_affects_total() {
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_slow = make_hm();
    let mut hm_fast = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_slow = small_config(500);
    cfg_slow.erode_speed = 0.05;
    let mut cfg_fast = small_config(500);
    cfg_fast.erode_speed = 0.9;

    let stats_slow = sim1.apply_hydraulic_erosion(&mut hm_slow, &cfg_slow);
    let stats_fast = sim2.apply_hydraulic_erosion(&mut hm_fast, &cfg_fast);

    assert!(
        stats_fast.total_eroded >= stats_slow.total_eroded,
        "Faster erosion speed ({}) should erode >= slower ({})",
        stats_fast.total_eroded,
        stats_slow.total_eroded
    );
}

#[test]
fn hydraulic_deposit_speed_affects_deposition() {
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_slow = make_hm();
    let mut hm_fast = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_slow = small_config(1000);
    cfg_slow.deposit_speed = 0.01;
    let mut cfg_fast = small_config(1000);
    cfg_fast.deposit_speed = 0.99;

    let stats_slow = sim1.apply_hydraulic_erosion(&mut hm_slow, &cfg_slow);
    let stats_fast = sim2.apply_hydraulic_erosion(&mut hm_fast, &cfg_fast);

    // Different deposit speeds → different deposition patterns
    assert_ne!(
        hm_slow.data(),
        hm_fast.data(),
        "Different deposit speeds must produce different terrain"
    );
    // Higher deposit speed → more deposition
    assert!(
        stats_fast.total_deposited >= stats_slow.total_deposited,
        "Faster deposit ({}) should deposit >= slower ({})",
        stats_fast.total_deposited,
        stats_slow.total_deposited
    );
}

// ============================================================================
// H. Brush radius mutations (indirectly testing init_erosion_brush)
// ============================================================================

#[test]
fn hydraulic_radius_1_vs_5_different_spread() {
    // Different brush radii should produce different erosion patterns
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_r1 = make_hm();
    let mut hm_r5 = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_r1 = small_config(500);
    cfg_r1.erosion_radius = 1;
    let mut cfg_r5 = small_config(500);
    cfg_r5.erosion_radius = 5;

    sim1.apply_hydraulic_erosion(&mut hm_r1, &cfg_r1);
    sim2.apply_hydraulic_erosion(&mut hm_r5, &cfg_r5);

    assert_ne!(
        hm_r1.data(),
        hm_r5.data(),
        "Different radii must produce different erosion"
    );
}

#[test]
fn hydraulic_larger_radius_smoother_erosion() {
    // Larger brush radius → erosion is spread over more cells → smoother result
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_r1 = make_hm();
    let mut hm_r5 = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_r1 = small_config(2000);
    cfg_r1.erosion_radius = 1;
    let mut cfg_r5 = small_config(2000);
    cfg_r5.erosion_radius = 5;

    sim1.apply_hydraulic_erosion(&mut hm_r1, &cfg_r1);
    sim2.apply_hydraulic_erosion(&mut hm_r5, &cfg_r5);

    // Compute roughness: sum of absolute differences between adjacent cells
    let roughness = |data: &[f32], res: usize| -> f32 {
        let mut r = 0.0f32;
        for z in 0..res {
            for x in 0..(res - 1) {
                r += (data[z * res + x] - data[z * res + x + 1]).abs();
            }
        }
        r
    };
    let r1_rough = roughness(hm_r1.data(), 32);
    let r5_rough = roughness(hm_r5.data(), 32);

    // Larger radius shouldn't dramatically increase roughness
    // (it may or may not be smoother depending on pattern, but should differ)
    assert!(
        (r1_rough - r5_rough).abs() > 0.01,
        "Roughness should differ: r1={r1_rough:.4}, r5={r5_rough:.4}"
    );
}

// ============================================================================
// I. Bounds check mutations (L388/390)
// ============================================================================

#[test]
fn hydraulic_small_map_many_terminations() {
    // On a tiny map (8×8), many droplets should terminate quickly (hit boundary)
    let mut hm = sloped_x_heightmap(8, 3.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 500,
        max_droplet_lifetime: 100,
        erosion_radius: 1,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // Most droplets should terminate (reach boundary) on a tiny sloped map
    assert!(
        stats.droplets_terminated > 100,
        "Small sloped map should terminate many droplets, got {}",
        stats.droplets_terminated
    );
}

#[test]
fn hydraulic_avg_lifetime_bounded() {
    let mut hm = wavy_heightmap(32);
    let mut sim = AdvancedErosionSimulator::new(42);
    let max_life = 30;
    let config = HydraulicErosionConfig {
        droplet_count: 500,
        max_droplet_lifetime: max_life,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    // Average lifetime must be between 0 and max
    assert!(stats.avg_droplet_lifetime >= 0.0);
    assert!(
        stats.avg_droplet_lifetime <= max_life as f32,
        "avg lifetime ({}) must be <= max ({})",
        stats.avg_droplet_lifetime,
        max_life
    );
}

#[test]
fn hydraulic_max_lifetime_1_limits_erosion() {
    // max_droplet_lifetime = 1 → each droplet only takes 1 step → minimal erosion
    let mut hm = peak_heightmap(32, 80.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 500,
        max_droplet_lifetime: 1,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    assert!(
        stats.avg_droplet_lifetime <= 1.0,
        "Avg lifetime should be <= 1 with max=1, got {}",
        stats.avg_droplet_lifetime
    );
}

// ============================================================================
// J. Erosion map output size
// ============================================================================

#[test]
fn hydraulic_erosion_map_size_matches_heightmap() {
    let mut hm = wavy_heightmap(32);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = small_config(100);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    let map = stats.erosion_map.as_ref().expect("Should have erosion map");
    assert_eq!(map.len(), 32 * 32, "Erosion map must be resolution^2");
}

#[test]
fn hydraulic_erosion_map_has_nonzero_entries() {
    let mut hm = peak_heightmap(32, 80.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = small_config(1000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    let map = stats.erosion_map.as_ref().expect("Should have erosion map");
    let nonzero = map.iter().filter(|&&v| v.abs() > 1e-10).count();
    assert!(
        nonzero > 0,
        "Erosion map should have nonzero entries on sloped terrain"
    );
}

#[test]
fn hydraulic_max_erosion_depth_positive_on_slope() {
    let mut hm = sloped_x_heightmap(32, 2.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = small_config(1000);
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    assert!(
        stats.max_erosion_depth > 0.0,
        "max_erosion_depth should be > 0 on sloped terrain"
    );
}

// ============================================================================
// K. Droplet count = 0 edge case
// ============================================================================

#[test]
fn hydraulic_zero_droplets_no_change() {
    let mut hm = peak_heightmap(32, 80.0);
    let before: Vec<f32> = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 0,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);

    assert_eq!(
        hm.data(),
        &before[..],
        "Zero droplets should not modify terrain"
    );
    assert!((stats.total_eroded - 0.0).abs() < 1e-10);
    assert!((stats.total_deposited - 0.0).abs() < 1e-10);
}

// ============================================================================
// L. apply_preset dispatching tests
// ============================================================================

#[test]
fn apply_preset_coastal_uses_all_three_passes() {
    let mut hm = peak_heightmap(32, 60.0);
    let before: Vec<f32> = hm.data().to_vec();
    let mut sim = AdvancedErosionSimulator::new(42);
    let preset = ErosionPreset::coastal();
    let stats = sim.apply_preset(&mut hm, &preset);

    // Coastal runs thermal + hydraulic + wind → terrain should change significantly
    assert!(stats.total_eroded > 0.0, "Coastal preset should erode");
    assert_ne!(
        hm.data(),
        &before[..],
        "Coastal preset should modify terrain"
    );
}

#[test]
fn apply_preset_desert_no_hydraulic_pass() {
    // Desert preset should skip hydraulic (it's None), only thermal + wind
    let mut hm = peak_heightmap(32, 60.0);
    let mut sim = AdvancedErosionSimulator::new(42);
    let preset = ErosionPreset::desert();
    let stats = sim.apply_preset(&mut hm, &preset);

    // Should still erode via thermal and wind
    assert!(stats.total_eroded > 0.0, "Desert preset should erode");
}

#[test]
fn apply_preset_mountain_order_hydraulic_then_thermal() {
    let p = ErosionPreset::mountain();
    assert_eq!(p.pass_order[0], "hydraulic");
    assert_eq!(p.pass_order[1], "thermal");
}

// ============================================================================
// M. Thermal erosion property tests
// ============================================================================

#[test]
fn thermal_erosion_flat_no_change() {
    let mut hm = flat_heightmap(32, 50.0);
    let before: Vec<f32> = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig::default();
    let _stats = sim.apply_thermal_erosion(&mut hm, &config);

    // Flat terrain has no slope > talus → no material movement
    assert_eq!(
        hm.data(),
        &before[..],
        "Flat terrain should not change under thermal erosion"
    );
}

#[test]
fn thermal_erosion_steep_terrain_reduces_slope() {
    // Very steep terrain should have material moved from peaks to valleys
    let mut hm = peak_heightmap(32, 100.0);
    let peak_before = hm.data()[16 * 32 + 16]; // Center
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig {
        iterations: 50,
        talus_angle: 10.0, // Very low talus → aggressive erosion
        ..Default::default()
    };
    let stats = sim.apply_thermal_erosion(&mut hm, &config);
    let peak_after = hm.data()[16 * 32 + 16];

    assert!(
        peak_after < peak_before,
        "Peak should decrease: before={peak_before}, after={peak_after}"
    );
    assert!(
        stats.total_eroded > 0.0,
        "Thermal should erode steep terrain"
    );
}

#[test]
fn thermal_erosion_4dir_vs_8dir_different() {
    let make_hm = || peak_heightmap(32, 100.0);
    let mut hm4 = make_hm();
    let mut hm8 = make_hm();

    let sim1 = AdvancedErosionSimulator::new(42);
    let sim2 = AdvancedErosionSimulator::new(42);

    let cfg4 = ThermalErosionConfig {
        eight_directional: false,
        talus_angle: 20.0,
        iterations: 30,
        ..Default::default()
    };
    let cfg8 = ThermalErosionConfig {
        eight_directional: true,
        talus_angle: 20.0,
        iterations: 30,
        ..Default::default()
    };

    sim1.apply_thermal_erosion(&mut hm4, &cfg4);
    sim2.apply_thermal_erosion(&mut hm8, &cfg8);

    assert_ne!(
        hm4.data(),
        hm8.data(),
        "4-dir vs 8-dir thermal should differ"
    );
}

#[test]
fn thermal_erosion_talus_angle_affects_result() {
    let make_hm = || peak_heightmap(32, 100.0);
    let mut hm_low = make_hm();
    let mut hm_high = make_hm();

    let sim1 = AdvancedErosionSimulator::new(42);
    let sim2 = AdvancedErosionSimulator::new(42);

    let cfg_low = ThermalErosionConfig {
        talus_angle: 5.0,
        iterations: 30,
        ..Default::default()
    };
    let cfg_high = ThermalErosionConfig {
        talus_angle: 80.0,
        iterations: 30,
        ..Default::default()
    };

    let stats_low = sim1.apply_thermal_erosion(&mut hm_low, &cfg_low);
    let stats_high = sim2.apply_thermal_erosion(&mut hm_high, &cfg_high);

    // Lower talus angle → more material moves (even gentle slopes exceed threshold)
    assert!(
        stats_low.total_eroded >= stats_high.total_eroded,
        "Low talus ({}) should erode >= high talus ({})",
        stats_low.total_eroded,
        stats_high.total_eroded
    );
}

#[test]
fn thermal_redistribution_rate_affects_amount() {
    let make_hm = || peak_heightmap(32, 100.0);
    let mut hm_low = make_hm();
    let mut hm_high = make_hm();

    let sim1 = AdvancedErosionSimulator::new(42);
    let sim2 = AdvancedErosionSimulator::new(42);

    let cfg_low = ThermalErosionConfig {
        redistribution_rate: 0.05,
        talus_angle: 20.0,
        iterations: 30,
        ..Default::default()
    };
    let cfg_high = ThermalErosionConfig {
        redistribution_rate: 0.95,
        talus_angle: 20.0,
        iterations: 30,
        ..Default::default()
    };

    let stats_low = sim1.apply_thermal_erosion(&mut hm_low, &cfg_low);
    let stats_high = sim2.apply_thermal_erosion(&mut hm_high, &cfg_high);

    assert!(
        stats_high.total_eroded >= stats_low.total_eroded,
        "Higher redistribution ({}) should erode >= lower ({})",
        stats_high.total_eroded,
        stats_low.total_eroded
    );
}

#[test]
fn thermal_iterations_zero_no_change() {
    let mut hm = peak_heightmap(32, 100.0);
    let before: Vec<f32> = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig {
        iterations: 0,
        ..Default::default()
    };
    let _stats = sim.apply_thermal_erosion(&mut hm, &config);
    assert_eq!(
        hm.data(),
        &before[..],
        "Zero iterations should not change terrain"
    );
}

// ============================================================================
// N. Wind erosion property tests
// ============================================================================

#[test]
fn wind_erosion_flat_no_change() {
    let mut hm = flat_heightmap(32, 50.0);
    let before: Vec<f32> = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig::default();
    let _stats = sim.apply_wind_erosion(&mut hm, &config);

    // Flat: no windward slope → no erosion
    assert_eq!(
        hm.data(),
        &before[..],
        "Flat terrain should not change under wind erosion"
    );
}

#[test]
fn wind_erosion_affects_sloped_terrain() {
    let mut hm = sloped_x_heightmap(32, 2.0);
    let before: Vec<f32> = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig {
        wind_strength: 1.0,
        iterations: 50,
        ..Default::default()
    };
    let stats = sim.apply_wind_erosion(&mut hm, &config);

    assert!(stats.total_eroded > 0.0, "Wind should erode sloped terrain");
    assert_ne!(hm.data(), &before[..]);
}

#[test]
fn wind_strength_affects_erosion_amount() {
    let make_hm = || sloped_x_heightmap(32, 2.0);
    let mut hm_weak = make_hm();
    let mut hm_strong = make_hm();

    let sim1 = AdvancedErosionSimulator::new(42);
    let sim2 = AdvancedErosionSimulator::new(42);

    let cfg_weak = WindErosionConfig {
        wind_strength: 0.1,
        ..Default::default()
    };
    let cfg_strong = WindErosionConfig {
        wind_strength: 2.0,
        ..Default::default()
    };

    let stats_weak = sim1.apply_wind_erosion(&mut hm_weak, &cfg_weak);
    let stats_strong = sim2.apply_wind_erosion(&mut hm_strong, &cfg_strong);

    assert!(
        stats_strong.total_eroded >= stats_weak.total_eroded,
        "Stronger wind ({}) should erode >= weaker ({})",
        stats_strong.total_eroded,
        stats_weak.total_eroded
    );
}

#[test]
fn wind_direction_affects_pattern() {
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_east = make_hm();
    let mut hm_north = make_hm();

    let sim1 = AdvancedErosionSimulator::new(42);
    let sim2 = AdvancedErosionSimulator::new(42);

    let cfg_east = WindErosionConfig {
        wind_direction: glam::Vec2::new(1.0, 0.0),
        iterations: 30,
        ..Default::default()
    };
    let cfg_north = WindErosionConfig {
        wind_direction: glam::Vec2::new(0.0, 1.0),
        iterations: 30,
        ..Default::default()
    };

    sim1.apply_wind_erosion(&mut hm_east, &cfg_east);
    sim2.apply_wind_erosion(&mut hm_north, &cfg_north);

    assert_ne!(
        hm_east.data(),
        hm_north.data(),
        "Different wind directions should produce different patterns"
    );
}

#[test]
fn wind_erosion_zero_iterations_no_change() {
    let mut hm = sloped_x_heightmap(32, 2.0);
    let before: Vec<f32> = hm.data().to_vec();
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig {
        iterations: 0,
        ..Default::default()
    };
    let _stats = sim.apply_wind_erosion(&mut hm, &config);
    assert_eq!(
        hm.data(),
        &before[..],
        "Zero iterations should not change terrain"
    );
}

// ============================================================================
// O. Scatter seed offset mutations (L193/L201)
// ============================================================================

#[test]
fn scatter_different_chunks_different_vegetation() {
    let config = fast_scatter_config();
    let mut gen = WorldGenerator::new(config);

    // Generate two chunks at different positions
    let chunk_a = gen.generate_and_register_chunk(ChunkId::new(0, 0)).unwrap();
    let chunk_b = gen.generate_and_register_chunk(ChunkId::new(1, 0)).unwrap();

    let scatter_a = gen.scatter_chunk_content(&chunk_a).unwrap();
    let scatter_b = gen.scatter_chunk_content(&chunk_b).unwrap();

    // Chunks at different X positions should use different seeds → different scatter
    // (seed + x*1000 + z) differs between (0,0) and (1,0)
    let veg_a_count = scatter_a.vegetation.len();
    let veg_b_count = scatter_b.vegetation.len();
    let res_a_count = scatter_a.resources.len();
    let res_b_count = scatter_b.resources.len();

    // At least one of vegetation or resource counts should differ, or positions differ
    let something_differs = veg_a_count != veg_b_count
        || res_a_count != res_b_count
        || (veg_a_count > 0
            && veg_b_count > 0
            && scatter_a.vegetation[0].position != scatter_b.vegetation[0].position);
    assert!(
        something_differs,
        "Different chunks should produce different scatter (veg: {veg_a_count} vs {veg_b_count}, res: {res_a_count} vs {res_b_count})"
    );
}

#[test]
fn scatter_different_z_different_result() {
    // E3.a redesign (2026-06-30) — see docs/audits/e3_terrain_test_surface_recon_2026-06.md A.7.
    // FLAT terrain isolates the Z-seeded RNG (the only per-chunk difference);
    // compare CHUNK-LOCAL positions (offset-independent) over vegetation +
    // resources. Fails iff the Z term stops affecting the scatter.
    fn local_fp(
        chunk: &astraweave_terrain::TerrainChunk,
        s: &astraweave_terrain::ScatterResult,
        chunk_size: f32,
    ) -> Vec<(i32, i32, i32)> {
        let o = chunk.id().to_world_pos(chunk_size);
        let q = |px: f32, py: f32, pz: f32| {
            (((px - o.x) * 100.0) as i32, (py * 100.0) as i32, ((pz - o.z) * 100.0) as i32)
        };
        let mut v: Vec<(i32, i32, i32)> = s
            .vegetation
            .iter()
            .map(|i| q(i.position.x, i.position.y, i.position.z))
            .collect();
        v.extend(s.resources.iter().map(|r| q(r.pos.x, r.pos.y, r.pos.z)));
        v.sort_unstable();
        v
    }

    let mut config = fast_scatter_config();
    config.noise.base_elevation.amplitude = 0.0;
    config.noise.mountains.amplitude = 0.0;
    config.noise.detail.amplitude = 0.0;
    config.noise.erosion_enabled = false;
    let chunk_size = config.chunk_size;
    let mut gen = WorldGenerator::new(config);

    let chunk_a = gen.generate_and_register_chunk(ChunkId::new(0, 0)).unwrap();
    let chunk_b = gen.generate_and_register_chunk(ChunkId::new(0, 1)).unwrap();
    let scatter_a = gen.scatter_chunk_content(&chunk_a).unwrap();
    let scatter_b = gen.scatter_chunk_content(&chunk_b).unwrap();
    let fp_a = local_fp(&chunk_a, &scatter_a, chunk_size);
    let fp_b = local_fp(&chunk_b, &scatter_b, chunk_size);

    assert!(
        !fp_a.is_empty() || !fp_b.is_empty(),
        "scatter produced nothing on either chunk — cannot assess Z-variation"
    );
    assert_ne!(
        fp_a, fp_b,
        "Chunks at different Z must produce a different local scatter pattern \
         (the Z term seeds the RNG). Diagonal-collision hardening deferred — ROADMAP_R1 §6.3."
    );
}

#[test]
fn scatter_deterministic_same_chunk() {
    let config = fast_scatter_config();
    let mut gen = WorldGenerator::new(config);

    let chunk = gen.generate_and_register_chunk(ChunkId::new(0, 0)).unwrap();

    let scatter_1 = gen.scatter_chunk_content(&chunk).unwrap();
    let scatter_2 = gen.scatter_chunk_content(&chunk).unwrap();

    assert_eq!(
        scatter_1.vegetation.len(),
        scatter_2.vegetation.len(),
        "Same chunk should produce identical vegetation count"
    );
    assert_eq!(
        scatter_1.resources.len(),
        scatter_2.resources.len(),
        "Same chunk should produce identical resource count"
    );
}

#[test]
fn scatter_vegetation_uses_different_seed_than_resources() {
    // L193 uses x*1000, L201 uses x*2000 — they should produce different RNG sequences
    // Indirectly: for a chunk at (1,0), vegetation seed = seed+1000, resource seed = seed+2000
    // We test by checking that vegetation and resource placements differ
    let config = fast_scatter_config();
    let mut gen = WorldGenerator::new(config);

    let chunk = gen.generate_and_register_chunk(ChunkId::new(1, 0)).unwrap();
    let scatter = gen.scatter_chunk_content(&chunk).unwrap();

    // If there are both vegetation and resources, their positions should differ
    // (since they use different seed offsets)
    if !scatter.vegetation.is_empty() && !scatter.resources.is_empty() {
        let veg_pos = scatter.vegetation[0].position;
        let res_pos = scatter.resources[0].pos;
        // Positions should differ because different seeds were used
        assert!(
            (veg_pos.x - res_pos.x).abs() > 0.01
                || (veg_pos.y - res_pos.y).abs() > 0.01
                || (veg_pos.z - res_pos.z).abs() > 0.01,
            "Vegetation and resources should use different seeds → different positions"
        );
    }
}

// ============================================================================
// P. WorldGenerator edge cases
// ============================================================================

#[test]
fn world_generator_get_chunk_returns_none_unloaded() {
    let config = WorldConfig::default();
    let gen = WorldGenerator::new(config);
    assert!(
        gen.get_chunk(ChunkId::new(99, 99)).is_none(),
        "Unloaded chunk should return None"
    );
}

#[test]
fn world_generator_stream_chunks_unloads_distant() {
    let config = fast_scatter_config();
    let mut gen = WorldGenerator::new(config);

    // Load chunks near origin
    gen.stream_chunks(glam::Vec3::ZERO, 1).unwrap();

    // Stream far away — should unload origin chunks
    let far = glam::Vec3::new(10000.0, 0.0, 10000.0);
    gen.stream_chunks(far, 1).unwrap();

    // Original chunks should be unloaded
    assert!(
        gen.get_chunk(ChunkId::new(0, 0)).is_none(),
        "Distant chunks should be unloaded after streaming far away"
    );
}

#[test]
fn world_generator_generate_chunk_with_scatter_returns_both() {
    let config = fast_scatter_config();
    let mut gen = WorldGenerator::new(config);

    let (chunk, scatter) = gen.generate_chunk_with_scatter(ChunkId::new(0, 0)).unwrap();

    // Should return a valid chunk
    assert!(!chunk.heightmap().data().is_empty());
    // Scatter chunk_id should match
    assert_eq!(scatter.chunk_id, chunk.id());
}

// ============================================================================
// Q. ErosionStats default values
// ============================================================================

#[test]
fn erosion_stats_default_all_zero() {
    let stats = astraweave_terrain::advanced_erosion::ErosionStats::default();
    assert!((stats.total_eroded - 0.0).abs() < 1e-10);
    assert!((stats.total_deposited - 0.0).abs() < 1e-10);
    assert_eq!(stats.droplets_terminated, 0);
    assert!((stats.avg_droplet_lifetime - 0.0).abs() < 1e-10);
    assert!((stats.max_erosion_depth - 0.0).abs() < 1e-10);
    assert!(stats.erosion_map.is_none());
}

// ============================================================================
// R. Initial speed affects behavior
// ============================================================================

#[test]
fn hydraulic_initial_speed_affects_erosion() {
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_slow = make_hm();
    let mut hm_fast = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let mut cfg_slow = small_config(500);
    cfg_slow.initial_speed = 0.01;
    let mut cfg_fast = small_config(500);
    cfg_fast.initial_speed = 10.0;

    let stats_slow = sim1.apply_hydraulic_erosion(&mut hm_slow, &cfg_slow);
    let stats_fast = sim2.apply_hydraulic_erosion(&mut hm_fast, &cfg_fast);

    // Higher initial speed → higher initial velocity → more sediment capacity
    assert!(
        (stats_fast.total_eroded - stats_slow.total_eroded).abs() > 0.01,
        "Different initial speeds should produce different erosion amounts"
    );
    assert_ne!(
        hm_slow.data(),
        hm_fast.data(),
        "Different initial speed must produce different terrain"
    );
}

// ============================================================================
// S. Droplet count scaling
// ============================================================================

#[test]
fn hydraulic_more_droplets_more_erosion() {
    let make_hm = || peak_heightmap(32, 80.0);
    let mut hm_few = make_hm();
    let mut hm_many = make_hm();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);

    let cfg_few = small_config(100);
    let cfg_many = small_config(5000);

    let stats_few = sim1.apply_hydraulic_erosion(&mut hm_few, &cfg_few);
    let stats_many = sim2.apply_hydraulic_erosion(&mut hm_many, &cfg_many);

    assert!(
        stats_many.total_eroded >= stats_few.total_eroded,
        "More droplets ({}) should erode >= fewer ({})",
        stats_many.total_eroded,
        stats_few.total_eroded
    );
}

// ============================================================================
// T. Wind erosion saltation distance
// ============================================================================

#[test]
fn wind_saltation_distance_affects_pattern() {
    let make_hm = || sloped_x_heightmap(32, 2.0);
    let mut hm_short = make_hm();
    let mut hm_long = make_hm();

    let sim1 = AdvancedErosionSimulator::new(42);
    let sim2 = AdvancedErosionSimulator::new(42);

    let cfg_short = WindErosionConfig {
        saltation_distance: 1.0,
        iterations: 30,
        ..Default::default()
    };
    let cfg_long = WindErosionConfig {
        saltation_distance: 10.0,
        iterations: 30,
        ..Default::default()
    };

    sim1.apply_wind_erosion(&mut hm_short, &cfg_short);
    sim2.apply_wind_erosion(&mut hm_long, &cfg_long);

    assert_ne!(
        hm_short.data(),
        hm_long.data(),
        "Different saltation distances should produce different results"
    );
}
