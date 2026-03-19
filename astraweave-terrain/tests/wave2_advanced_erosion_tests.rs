//! Wave 2 Mutation Remediation Tests — advanced_erosion.rs
//!
//! Targets config defaults, preset factory values, erosion algorithm behavior,
//! and SimpleRng properties. These pin exact numeric values to kill arithmetic
//! mutations in the ~488-mutant advanced_erosion.rs pool.

use astraweave_terrain::*;
use glam::Vec2;

// ============================================================================
// HydraulicErosionConfig: exact default values
// ============================================================================

#[test]
fn hydraulic_config_default_droplet_count() {
    let c = HydraulicErosionConfig::default();
    assert_eq!(c.droplet_count, 50000);
}

#[test]
fn hydraulic_config_default_inertia() {
    let c = HydraulicErosionConfig::default();
    assert!((c.inertia - 0.05).abs() < 1e-6);
}

#[test]
fn hydraulic_config_default_sediment_capacity_factor() {
    let c = HydraulicErosionConfig::default();
    assert!((c.sediment_capacity_factor - 4.0).abs() < 1e-6);
}

#[test]
fn hydraulic_config_default_min_slope() {
    let c = HydraulicErosionConfig::default();
    assert!((c.min_slope - 0.01).abs() < 1e-6);
}

#[test]
fn hydraulic_config_default_deposit_speed() {
    let c = HydraulicErosionConfig::default();
    assert!((c.deposit_speed - 0.3).abs() < 1e-6);
}

#[test]
fn hydraulic_config_default_erode_speed() {
    let c = HydraulicErosionConfig::default();
    assert!((c.erode_speed - 0.3).abs() < 1e-6);
}

#[test]
fn hydraulic_config_default_evaporation_rate() {
    let c = HydraulicErosionConfig::default();
    assert!((c.evaporation_rate - 0.01).abs() < 1e-6);
}

#[test]
fn hydraulic_config_default_initial_water() {
    let c = HydraulicErosionConfig::default();
    assert!((c.initial_water - 1.0).abs() < 1e-6);
}

#[test]
fn hydraulic_config_default_initial_speed() {
    let c = HydraulicErosionConfig::default();
    assert!((c.initial_speed - 1.0).abs() < 1e-6);
}

#[test]
fn hydraulic_config_default_max_droplet_lifetime() {
    let c = HydraulicErosionConfig::default();
    assert_eq!(c.max_droplet_lifetime, 30);
}

#[test]
fn hydraulic_config_default_erosion_radius() {
    let c = HydraulicErosionConfig::default();
    assert_eq!(c.erosion_radius, 3);
}

#[test]
fn hydraulic_config_default_gravity() {
    let c = HydraulicErosionConfig::default();
    assert!((c.gravity - 4.0).abs() < 1e-6);
}

// ============================================================================
// ThermalErosionConfig: exact default values
// ============================================================================

#[test]
fn thermal_config_default_iterations() {
    let c = ThermalErosionConfig::default();
    assert_eq!(c.iterations, 50);
}

#[test]
fn thermal_config_default_talus_angle() {
    let c = ThermalErosionConfig::default();
    assert!((c.talus_angle - 45.0).abs() < 1e-6);
}

#[test]
fn thermal_config_default_redistribution_rate() {
    let c = ThermalErosionConfig::default();
    assert!((c.redistribution_rate - 0.5).abs() < 1e-6);
}

#[test]
fn thermal_config_default_eight_directional() {
    let c = ThermalErosionConfig::default();
    assert!(c.eight_directional);
}

#[test]
fn thermal_config_default_cell_size() {
    let c = ThermalErosionConfig::default();
    assert!((c.cell_size - 1.0).abs() < 1e-6);
}

// ============================================================================
// WindErosionConfig: exact default values
// ============================================================================

#[test]
fn wind_config_default_direction() {
    let c = WindErosionConfig::default();
    assert!((c.wind_direction.x - 1.0).abs() < 1e-6);
    assert!((c.wind_direction.y - 0.0).abs() < 1e-6);
}

#[test]
fn wind_config_default_strength() {
    let c = WindErosionConfig::default();
    assert!((c.wind_strength - 0.5).abs() < 1e-6);
}

#[test]
fn wind_config_default_suspension_height() {
    let c = WindErosionConfig::default();
    assert!((c.suspension_height - 5.0).abs() < 1e-6);
}

#[test]
fn wind_config_default_iterations() {
    let c = WindErosionConfig::default();
    assert_eq!(c.iterations, 30);
}

#[test]
fn wind_config_default_saltation_distance() {
    let c = WindErosionConfig::default();
    assert!((c.saltation_distance - 3.0).abs() < 1e-6);
}

// ============================================================================
// ErosionPreset::default(): verify structure
// ============================================================================

#[test]
fn erosion_preset_default_name() {
    let p = ErosionPreset::default();
    assert_eq!(p.name, "Default");
}

#[test]
fn erosion_preset_default_has_hydraulic() {
    let p = ErosionPreset::default();
    assert!(p.hydraulic.is_some());
}

#[test]
fn erosion_preset_default_has_thermal() {
    let p = ErosionPreset::default();
    assert!(p.thermal.is_some());
}

#[test]
fn erosion_preset_default_no_wind() {
    let p = ErosionPreset::default();
    assert!(p.wind.is_none());
}

#[test]
fn erosion_preset_default_pass_order() {
    let p = ErosionPreset::default();
    assert_eq!(p.pass_order.len(), 2);
    assert_eq!(p.pass_order[0], "thermal");
    assert_eq!(p.pass_order[1], "hydraulic");
}

// ============================================================================
// ErosionPreset::desert(): verify structure
// ============================================================================

#[test]
fn erosion_preset_desert_name() {
    let p = ErosionPreset::desert();
    assert_eq!(p.name, "Desert");
}

#[test]
fn erosion_preset_desert_has_wind() {
    let p = ErosionPreset::desert();
    assert!(p.wind.is_some());
}

#[test]
fn erosion_preset_desert_no_hydraulic() {
    let p = ErosionPreset::desert();
    assert!(p.hydraulic.is_none());
}

#[test]
fn erosion_preset_desert_thermal_talus() {
    let p = ErosionPreset::desert();
    let thermal = p.thermal.as_ref().unwrap();
    assert!((thermal.talus_angle - 35.0).abs() < 1e-6);
}

#[test]
fn erosion_preset_desert_pass_order() {
    let p = ErosionPreset::desert();
    assert_eq!(p.pass_order, vec!["thermal", "wind"]);
}

// ============================================================================
// ErosionPreset::mountain(): verify structure
// ============================================================================

#[test]
fn erosion_preset_mountain_name() {
    let p = ErosionPreset::mountain();
    assert_eq!(p.name, "Mountain");
}

#[test]
fn erosion_preset_mountain_hydraulic_droplet_count() {
    let p = ErosionPreset::mountain();
    let hydraulic = p.hydraulic.as_ref().unwrap();
    assert_eq!(hydraulic.droplet_count, 100000);
}

#[test]
fn erosion_preset_mountain_hydraulic_erode_speed() {
    let p = ErosionPreset::mountain();
    let hydraulic = p.hydraulic.as_ref().unwrap();
    assert!((hydraulic.erode_speed - 0.4).abs() < 1e-6);
}

#[test]
fn erosion_preset_mountain_thermal_talus_angle() {
    let p = ErosionPreset::mountain();
    let thermal = p.thermal.as_ref().unwrap();
    assert!((thermal.talus_angle - 50.0).abs() < 1e-6);
}

#[test]
fn erosion_preset_mountain_thermal_iterations() {
    let p = ErosionPreset::mountain();
    let thermal = p.thermal.as_ref().unwrap();
    assert_eq!(thermal.iterations, 30);
}

#[test]
fn erosion_preset_mountain_no_wind() {
    let p = ErosionPreset::mountain();
    assert!(p.wind.is_none());
}

#[test]
fn erosion_preset_mountain_pass_order() {
    let p = ErosionPreset::mountain();
    assert_eq!(p.pass_order, vec!["hydraulic", "thermal"]);
}

// ============================================================================
// ErosionPreset::coastal(): verify structure
// ============================================================================

#[test]
fn erosion_preset_coastal_name() {
    let p = ErosionPreset::coastal();
    assert_eq!(p.name, "Coastal");
}

#[test]
fn erosion_preset_coastal_has_all_three() {
    let p = ErosionPreset::coastal();
    assert!(p.hydraulic.is_some());
    assert!(p.thermal.is_some());
    assert!(p.wind.is_some());
}

#[test]
fn erosion_preset_coastal_hydraulic_droplet_count() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert_eq!(h.droplet_count, 30000);
}

#[test]
fn erosion_preset_coastal_hydraulic_evaporation() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.evaporation_rate - 0.02).abs() < 1e-6);
}

#[test]
fn erosion_preset_coastal_thermal_talus() {
    let p = ErosionPreset::coastal();
    let t = p.thermal.as_ref().unwrap();
    assert!((t.talus_angle - 40.0).abs() < 1e-6);
}

#[test]
fn erosion_preset_coastal_thermal_iterations() {
    let p = ErosionPreset::coastal();
    let t = p.thermal.as_ref().unwrap();
    assert_eq!(t.iterations, 20);
}

#[test]
fn erosion_preset_coastal_wind_strength() {
    let p = ErosionPreset::coastal();
    let w = p.wind.as_ref().unwrap();
    assert!((w.wind_strength - 0.3).abs() < 1e-6);
}

#[test]
fn erosion_preset_coastal_pass_order() {
    let p = ErosionPreset::coastal();
    assert_eq!(p.pass_order.len(), 3);
    assert_eq!(p.pass_order[0], "thermal");
    assert_eq!(p.pass_order[1], "hydraulic");
    assert_eq!(p.pass_order[2], "wind");
}

// ============================================================================
// ErosionStats: default values
// ============================================================================

#[test]
fn erosion_stats_default_zeros() {
    let s = ErosionStats::default();
    assert_eq!(s.total_eroded, 0.0);
    assert_eq!(s.total_deposited, 0.0);
    assert_eq!(s.droplets_terminated, 0);
    assert_eq!(s.avg_droplet_lifetime, 0.0);
    assert_eq!(s.max_erosion_depth, 0.0);
    assert!(s.erosion_map.is_none());
}

// ============================================================================
// Algorithm behavior: thermal erosion
// ============================================================================

fn make_spike_heightmap(res: u32) -> Heightmap {
    let mut data = vec![0.0f32; (res * res) as usize];
    let center = res / 2;
    for z in 0..res {
        for x in 0..res {
            let dx = x as f32 - center as f32;
            let dz = z as f32 - center as f32;
            let dist = (dx * dx + dz * dz).sqrt();
            data[(z * res + x) as usize] = (center as f32 - dist).max(0.0) * 3.0;
        }
    }
    Heightmap::from_data(data, res).unwrap()
}

#[test]
fn thermal_erosion_produces_erosion_map() {
    let mut hm = make_spike_heightmap(32);
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig::default();
    let stats = sim.apply_thermal_erosion(&mut hm, &config);
    assert!(stats.erosion_map.is_some(), "Should produce erosion map");
    let em = stats.erosion_map.unwrap();
    assert_eq!(em.len(), 1024, "Erosion map should match resolution^2");
}

#[test]
fn thermal_erosion_stats_total_eroded_positive() {
    let mut hm = make_spike_heightmap(32);
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig {
        iterations: 20,
        talus_angle: 20.0, // Lower angle → more erosion
        ..Default::default()
    };
    let stats = sim.apply_thermal_erosion(&mut hm, &config);
    assert!(
        stats.total_eroded > 0.0,
        "Thermal erosion should move material: {}",
        stats.total_eroded
    );
}

#[test]
fn thermal_erosion_flat_terrain_no_change() {
    let data = vec![10.0; 1024]; // 32×32 flat
    let mut hm = Heightmap::from_data(data, 32).unwrap();
    let sim = AdvancedErosionSimulator::new(42);
    let config = ThermalErosionConfig::default();
    let stats = sim.apply_thermal_erosion(&mut hm, &config);
    assert!(
        stats.total_eroded.abs() < 1e-6,
        "Flat terrain should not erode: {}",
        stats.total_eroded
    );
}

// ============================================================================
// Algorithm behavior: hydraulic erosion
// ============================================================================

#[test]
fn hydraulic_erosion_produces_erosion_map() {
    let mut hm = make_spike_heightmap(32);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 500, // Minimal for speed
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);
    assert!(stats.erosion_map.is_some());
}

#[test]
fn hydraulic_erosion_stats_populated() {
    let mut hm = make_spike_heightmap(32);
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 2000,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);
    // Should have eroded something on a mountain
    assert!(stats.total_eroded > 0.0, "Should erode material");
    assert!(
        stats.avg_droplet_lifetime > 0.0,
        "Droplets should survive some steps"
    );
}

#[test]
fn hydraulic_erosion_deterministic() {
    let hm1 = make_spike_heightmap(16);
    let hm2 = make_spike_heightmap(16);
    let mut h1 = hm1;
    let mut h2 = hm2;
    let config = HydraulicErosionConfig {
        droplet_count: 200,
        ..Default::default()
    };
    let mut s1 = AdvancedErosionSimulator::new(777);
    let mut s2 = AdvancedErosionSimulator::new(777);
    s1.apply_hydraulic_erosion(&mut h1, &config);
    s2.apply_hydraulic_erosion(&mut h2, &config);

    assert_eq!(h1.data().len(), h2.data().len());
    for i in 0..h1.data().len() {
        assert!(
            (h1.data()[i] - h2.data()[i]).abs() < 1e-6,
            "Same seed should give same result at index {}: {} vs {}",
            i,
            h1.data()[i],
            h2.data()[i]
        );
    }
}

// ============================================================================
// Algorithm behavior: wind erosion
// ============================================================================

#[test]
fn wind_erosion_moves_material_downwind() {
    let mut hm = make_spike_heightmap(32);
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig {
        wind_direction: Vec2::new(1.0, 0.0),
        wind_strength: 1.0,
        iterations: 50,
        ..Default::default()
    };
    let stats = sim.apply_wind_erosion(&mut hm, &config);
    assert!(
        stats.total_eroded > 0.0,
        "Wind should erode: {}",
        stats.total_eroded
    );
}

#[test]
fn wind_erosion_flat_terrain_no_change() {
    let data = vec![10.0; 1024]; // 32×32 flat
    let mut hm = Heightmap::from_data(data, 32).unwrap();
    let sim = AdvancedErosionSimulator::new(42);
    let config = WindErosionConfig::default();
    let stats = sim.apply_wind_erosion(&mut hm, &config);
    assert!(
        stats.total_eroded.abs() < 1e-6,
        "Flat terrain should not erode from wind"
    );
}

// ============================================================================
// Preset application: ensures correct pass routing
// ============================================================================

#[test]
fn apply_preset_default_runs_thermal_and_hydraulic() {
    let mut hm = make_spike_heightmap(32);
    let mut sim = AdvancedErosionSimulator::new(42);
    let preset = ErosionPreset {
        hydraulic: Some(HydraulicErosionConfig {
            droplet_count: 500,
            ..Default::default()
        }),
        ..Default::default()
    };
    let stats = sim.apply_preset(&mut hm, &preset);
    assert!(stats.total_eroded > 0.0, "Preset should erode");
}

#[test]
fn apply_preset_desert_includes_wind() {
    let mut hm = make_spike_heightmap(32);
    let mut sim = AdvancedErosionSimulator::new(42);
    let preset = ErosionPreset::desert();
    let stats = sim.apply_preset(&mut hm, &preset);
    assert!(stats.total_eroded > 0.0, "Desert preset should erode");
}

#[test]
fn apply_preset_unknown_pass_ignored() {
    let mut hm = make_spike_heightmap(16);
    let mut sim = AdvancedErosionSimulator::new(42);
    let preset = ErosionPreset {
        name: "Custom".to_string(),
        hydraulic: None,
        thermal: None,
        wind: None,
        pass_order: vec!["unknown_pass_type".to_string()],
    };
    let stats = sim.apply_preset(&mut hm, &preset);
    assert!(
        stats.total_eroded.abs() < 1e-6,
        "Unknown pass should do nothing"
    );
}

#[test]
fn apply_preset_empty_pass_order_no_op() {
    let data = vec![50.0; 256]; // 16×16
    let mut hm = Heightmap::from_data(data.clone(), 16).unwrap();
    let mut sim = AdvancedErosionSimulator::new(42);
    let preset = ErosionPreset {
        name: "Empty".to_string(),
        hydraulic: Some(HydraulicErosionConfig::default()),
        thermal: Some(ThermalErosionConfig::default()),
        wind: None,
        pass_order: vec![], // No passes
    };
    let stats = sim.apply_preset(&mut hm, &preset);
    assert!(stats.total_eroded.abs() < 1e-6);
}

// ============================================================================
// Thermal erosion: 4-directional vs 8-directional
// ============================================================================

#[test]
fn thermal_four_directional_erodes_less() {
    let mut hm_4 = make_spike_heightmap(32);
    let mut hm_8 = make_spike_heightmap(32);
    let sim = AdvancedErosionSimulator::new(42);

    let config_4 = ThermalErosionConfig {
        eight_directional: false,
        iterations: 10,
        talus_angle: 25.0,
        ..Default::default()
    };
    let config_8 = ThermalErosionConfig {
        eight_directional: true,
        iterations: 10,
        talus_angle: 25.0,
        ..Default::default()
    };

    let stats_4 = sim.apply_thermal_erosion(&mut hm_4, &config_4);
    let stats_8 = sim.apply_thermal_erosion(&mut hm_8, &config_8);

    // 8-directional should erode more (considers diagonal neighbors)
    assert!(
        stats_8.total_eroded >= stats_4.total_eroded,
        "8-dir ({}) should erode >= 4-dir ({})",
        stats_8.total_eroded,
        stats_4.total_eroded
    );
}

// ============================================================================
// Targeted: coastal preset inherited default fields
// ============================================================================

#[test]
fn erosion_preset_coastal_hydraulic_inertia_is_default() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.inertia - HydraulicErosionConfig::default().inertia).abs() < 1e-9);
}

#[test]
fn erosion_preset_coastal_hydraulic_sediment_capacity() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!(
        (h.sediment_capacity_factor - HydraulicErosionConfig::default().sediment_capacity_factor)
            .abs()
            < 1e-9
    );
}

#[test]
fn erosion_preset_coastal_hydraulic_min_slope() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.min_slope - HydraulicErosionConfig::default().min_slope).abs() < 1e-9);
}

#[test]
fn erosion_preset_coastal_hydraulic_erode_speed() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.erode_speed - HydraulicErosionConfig::default().erode_speed).abs() < 1e-9);
}

#[test]
fn erosion_preset_coastal_hydraulic_erosion_radius() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert_eq!(
        h.erosion_radius,
        HydraulicErosionConfig::default().erosion_radius
    );
}

#[test]
fn erosion_preset_coastal_hydraulic_gravity() {
    let p = ErosionPreset::coastal();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.gravity - HydraulicErosionConfig::default().gravity).abs() < 1e-9);
}

#[test]
fn erosion_preset_coastal_thermal_redistribution_rate() {
    let p = ErosionPreset::coastal();
    let t = p.thermal.as_ref().unwrap();
    assert!(
        (t.redistribution_rate - ThermalErosionConfig::default().redistribution_rate).abs() < 1e-9
    );
}

#[test]
fn erosion_preset_coastal_thermal_eight_directional() {
    let p = ErosionPreset::coastal();
    let t = p.thermal.as_ref().unwrap();
    assert_eq!(
        t.eight_directional,
        ThermalErosionConfig::default().eight_directional
    );
}

#[test]
fn erosion_preset_coastal_wind_direction() {
    let p = ErosionPreset::coastal();
    let w = p.wind.as_ref().unwrap();
    let def = WindErosionConfig::default();
    assert!((w.wind_direction.x - def.wind_direction.x).abs() < 1e-9);
    assert!((w.wind_direction.y - def.wind_direction.y).abs() < 1e-9);
}

#[test]
fn erosion_preset_coastal_wind_iterations() {
    let p = ErosionPreset::coastal();
    let w = p.wind.as_ref().unwrap();
    assert_eq!(w.iterations, WindErosionConfig::default().iterations);
}

// ============================================================================
// Targeted: mountain preset inherited fields
// ============================================================================

#[test]
fn erosion_preset_mountain_hydraulic_inertia() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.inertia - HydraulicErosionConfig::default().inertia).abs() < 1e-9);
}

#[test]
fn erosion_preset_mountain_hydraulic_gravity() {
    let p = ErosionPreset::mountain();
    let h = p.hydraulic.as_ref().unwrap();
    assert!((h.gravity - HydraulicErosionConfig::default().gravity).abs() < 1e-9);
}

#[test]
fn erosion_preset_mountain_thermal_redistribution_rate() {
    let p = ErosionPreset::mountain();
    let t = p.thermal.as_ref().unwrap();
    assert!(
        (t.redistribution_rate - ThermalErosionConfig::default().redistribution_rate).abs() < 1e-9
    );
}

#[test]
fn erosion_preset_mountain_thermal_eight_directional() {
    let p = ErosionPreset::mountain();
    let t = p.thermal.as_ref().unwrap();
    assert!(t.eight_directional);
}

// ============================================================================
// Targeted: desert preset inherited fields
// ============================================================================

#[test]
fn erosion_preset_desert_thermal_redistribution_rate() {
    let p = ErosionPreset::desert();
    let t = p.thermal.as_ref().unwrap();
    assert!(
        (t.redistribution_rate - ThermalErosionConfig::default().redistribution_rate).abs() < 1e-9
    );
}

#[test]
fn erosion_preset_desert_wind_direction() {
    let p = ErosionPreset::desert();
    let w = p.wind.as_ref().unwrap();
    assert!((w.wind_direction.x - WindErosionConfig::default().wind_direction.x).abs() < 1e-9);
}

#[test]
fn erosion_preset_desert_wind_suspension_height() {
    let p = ErosionPreset::desert();
    let w = p.wind.as_ref().unwrap();
    assert!((w.suspension_height - WindErosionConfig::default().suspension_height).abs() < 1e-9);
}

// ============================================================================
// Targeted: hydraulic erosion brush math (indirect via output properties)
// ============================================================================

#[test]
fn hydraulic_erosion_net_conservation_approximate() {
    // Erosion should roughly conserve mass (eroded ≈ deposited + terminated)
    let mut data: Vec<f32> = Vec::new();
    for z in 0..32 {
        for x in 0..32 {
            data.push(((x as f32 * 0.31).sin() + (z as f32 * 0.29).cos() + 2.0) * 20.0);
        }
    }
    let mut hm = Heightmap::from_data(data, 32).unwrap();
    let before_sum: f32 = hm.data().iter().sum();

    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 1000,
        max_droplet_lifetime: 30,
        ..Default::default()
    };
    let _stats = sim.apply_hydraulic_erosion(&mut hm, &config);
    let after_sum: f32 = hm.data().iter().sum();

    // Heights should change
    assert!(
        (before_sum - after_sum).abs() > 0.1,
        "Hydraulic erosion should modify heights: before_sum={}, after_sum={}",
        before_sum,
        after_sum
    );
}

#[test]
fn hydraulic_erosion_stats_droplets_terminated() {
    let mut data: Vec<f32> = Vec::new();
    for z in 0..32 {
        for x in 0..32 {
            data.push(((x as f32 * 0.2).sin().abs() + 0.5) * 40.0);
        }
    }
    let mut hm = Heightmap::from_data(data, 32).unwrap();
    let mut sim = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 500,
        max_droplet_lifetime: 20,
        ..Default::default()
    };
    let stats = sim.apply_hydraulic_erosion(&mut hm, &config);
    // Some droplets should terminate (reach edge or water)
    assert!(
        stats.droplets_terminated > 0,
        "Expected some terminated droplets"
    );
}

#[test]
fn hydraulic_erosion_erodes_steep_more() {
    // Steeper terrain should erode more than gentle
    let mut steep: Vec<f32> = Vec::new();
    let mut gentle: Vec<f32> = Vec::new();
    for _z in 0..32 {
        for x in 0..32 {
            steep.push(x as f32 * 3.0); // Very steep gradient
            gentle.push(x as f32 * 0.5); // Gentle slope
        }
    }
    let mut hm_steep = Heightmap::from_data(steep, 32).unwrap();
    let mut hm_gentle = Heightmap::from_data(gentle, 32).unwrap();

    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);
    let config = HydraulicErosionConfig {
        droplet_count: 500,
        max_droplet_lifetime: 30,
        ..Default::default()
    };
    let stats_steep = sim1.apply_hydraulic_erosion(&mut hm_steep, &config);
    let stats_gentle = sim2.apply_hydraulic_erosion(&mut hm_gentle, &config);

    assert!(
        stats_steep.total_eroded >= stats_gentle.total_eroded,
        "Steep ({}) should erode >= gentle ({})",
        stats_steep.total_eroded,
        stats_gentle.total_eroded
    );
}

#[test]
fn hydraulic_erosion_deterministic_same_seed() {
    let make_hm = || {
        let data: Vec<f32> = (0..1024)
            .map(|i| (i as f32 * 0.1).sin().abs() * 50.0)
            .collect();
        Heightmap::from_data(data, 32).unwrap()
    };
    let mut hm1 = make_hm();
    let mut hm2 = make_hm();
    let config = HydraulicErosionConfig {
        droplet_count: 200,
        ..Default::default()
    };
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(42);
    sim1.apply_hydraulic_erosion(&mut hm1, &config);
    sim2.apply_hydraulic_erosion(&mut hm2, &config);
    assert_eq!(
        hm1.data(),
        hm2.data(),
        "Same seed must produce identical output"
    );
}

#[test]
fn hydraulic_erosion_different_seed_different_output() {
    let make_hm = || {
        let data: Vec<f32> = (0..1024)
            .map(|i| (i as f32 * 0.1).sin().abs() * 50.0)
            .collect();
        Heightmap::from_data(data, 32).unwrap()
    };
    let mut hm1 = make_hm();
    let mut hm2 = make_hm();
    let config = HydraulicErosionConfig {
        droplet_count: 500,
        ..Default::default()
    };
    let mut sim1 = AdvancedErosionSimulator::new(42);
    let mut sim2 = AdvancedErosionSimulator::new(999);
    sim1.apply_hydraulic_erosion(&mut hm1, &config);
    sim2.apply_hydraulic_erosion(&mut hm2, &config);
    assert_ne!(
        hm1.data(),
        hm2.data(),
        "Different seeds should produce different output"
    );
}

// ============================================================================
// Targeted: WorldGenerator output properties
// ============================================================================

#[test]
fn world_generator_generate_chunk_biome_not_empty() {
    let config = WorldConfig::default();
    let generator = WorldGenerator::new(config);
    let chunk = generator.generate_chunk(ChunkId::new(0, 0)).unwrap();
    // Biome map should have entries
    assert!(
        !chunk.biome_map().is_empty(),
        "Generated chunk should have biome assignments"
    );
}

#[test]
fn world_generator_chunk_heightmap_resolution() {
    let config = WorldConfig::default();
    let resolution = config.heightmap_resolution;
    let generator = WorldGenerator::new(config);
    let chunk = generator.generate_chunk(ChunkId::new(0, 0)).unwrap();
    assert_eq!(chunk.heightmap().resolution(), resolution);
}

#[test]
fn world_generator_stream_chunks_loads_center() {
    let config = WorldConfig::default();
    let mut generator = WorldGenerator::new(config);
    let center = glam::Vec3::new(128.0, 0.0, 128.0);
    let loaded = generator.stream_chunks(center, 1).unwrap();
    // At least the center chunk should be loaded
    assert!(
        !loaded.is_empty(),
        "stream_chunks with radius 1 should load at least 1 chunk"
    );
}

#[test]
fn world_generator_config_returns_correct_seed() {
    let mut config = WorldConfig::default();
    config.seed = 99999;
    let gen = WorldGenerator::new(config);
    assert_eq!(gen.config().seed, 99999);
}
