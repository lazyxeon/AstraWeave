//! Phase 1.6-F.2-T.C temporary performance measurement.
//! Confirms F.2-T tuning keeps generation time within 2× F.1 baseline.
//! Deleted at F.2-T.D closeout.

use astraweave_terrain::{DomainWarpConfig, NoiseType};
use aw_editor_lib::terrain_integration::{BiomeNoisePreset, TerrainState};

fn f1_style_grassland() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.004,
        base_amplitude: 50.0,
        base_octaves: 5,
        base_persistence: 0.50,
        base_lacunarity: 2.0,
        mountains_enabled: true,
        mountains_scale: 0.0025,
        mountains_amplitude: 80.0,
        mountains_octaves: 6,
        detail_enabled: true,
        detail_scale: 0.02,
        detail_amplitude: 8.0,
        erosion_enabled: true,
        erosion_strength: 0.3,
        base_noise_type: NoiseType::Perlin,
        base_domain_warp: None,
        continental_modulation: false,
    }
}

fn f2_t_grassland() -> BiomeNoisePreset {
    BiomeNoisePreset {
        base_scale: 0.004,
        base_amplitude: 50.0,
        base_octaves: 5,
        base_persistence: 0.50,
        base_lacunarity: 2.0,
        mountains_enabled: true,
        mountains_scale: 0.0025,
        mountains_amplitude: 80.0,
        mountains_octaves: 6,
        detail_enabled: true,
        detail_scale: 0.02,
        detail_amplitude: 4.0, // F.2-T.B.2 reduction
        erosion_enabled: true,
        erosion_strength: 0.3,
        base_noise_type: NoiseType::DomainWarped,
        base_domain_warp: Some(DomainWarpConfig {
            iterations: 1,
            warp_scale: 1.5,
            warp_strength: 40.0,
            warp_octaves: 3,
        }),
        continental_modulation: true,
    }
}

fn run(label: &str, preset: BiomeNoisePreset) -> (std::time::Duration, f32, f32) {
    let mut state = TerrainState::new();
    state.configure(12345, "grassland");
    state.set_noise_params(6, 2.0, 0.5, 50.0);
    state.apply_biome_noise_preset(&preset);

    let start = std::time::Instant::now();
    state.generate_terrain(5).expect("generation");
    let elapsed = start.elapsed();

    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;
    for (_id, gc) in state.chunks() {
        for v in gc.vertices.iter() {
            if v.position[1] < y_min {
                y_min = v.position[1];
            }
            if v.position[1] > y_max {
                y_max = v.position[1];
            }
        }
    }
    println!(
        "{label}: {} ms, Y=[{:.2}, {:.2}], span {:.2}",
        elapsed.as_millis(),
        y_min,
        y_max,
        y_max - y_min
    );
    (elapsed, y_min, y_max)
}

#[test]
fn phase_1_6_f2_t_perf() {
    println!("===========================================");
    println!("Phase 1.6-F.2-T.C performance measurement");
    let (f1, _, _) = run("F.1-style", f1_style_grassland());
    let (f2t, _, _) = run("F.2-T landed", f2_t_grassland());
    let ratio = f2t.as_secs_f64() / f1.as_secs_f64();
    println!("F.2-T / F.1 ratio: {ratio:.2}x (gate: <= 2.00)");
    println!("===========================================");
}
