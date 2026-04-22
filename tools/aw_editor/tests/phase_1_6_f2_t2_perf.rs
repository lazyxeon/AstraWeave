//! Phase 1.6-F.2-T-2.C temporary performance measurement.
//! Deleted at F.2-T-2.D closeout.

use astraweave_terrain::{DomainWarpConfig, NoiseType};
use aw_editor_lib::terrain_integration::{BiomeNoisePreset, TerrainState};

fn f2_t_2_grassland() -> BiomeNoisePreset {
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
        detail_amplitude: 4.0,
        erosion_enabled: true,
        erosion_strength: 0.3,
        base_noise_type: NoiseType::DomainWarped,
        base_domain_warp: Some(DomainWarpConfig {
            iterations: 1,
            warp_scale: 1.5,
            warp_strength: 15.0, // F.2-T-2.B.3
            warp_octaves: 3,
        }),
        continental_modulation: true,
    }
}

#[test]
fn phase_1_6_f2_t2_perf() {
    let mut state = TerrainState::new();
    state.configure(12345, "grassland");
    state.set_noise_params(6, 2.0, 0.5, 50.0);
    state.apply_biome_noise_preset(&f2_t_2_grassland());

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
    println!("===========================================");
    println!("Phase 1.6-F.2-T-2.C performance measurement");
    println!(
        "F.2-T-2 grassland: {} ms, Y=[{:.2}, {:.2}], span {:.2}",
        elapsed.as_millis(),
        y_min,
        y_max,
        y_max - y_min
    );
    println!("F.1 baseline (measured F.2.D): 554 ms");
    println!("F.2-T baseline (measured F.2-T.C): 881 ms (1.47x F.1)");
    println!("F.2-T-2 target: <= 1108 ms (<=2.00x F.1)");
    println!("===========================================");
}
