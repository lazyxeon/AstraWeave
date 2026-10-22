//! Wave 2 Mutation Remediation Tests — weather_system.rs
//!
//! Pins WeatherTransition defaults and state machine, BiomeWeatherMap
//! probability tables for all 8 biomes, BiomeWindProfile per-biome
//! parameters, and effective_strength / effective_direction formulas.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::effects::WeatherKind;
use astraweave_render::weather_system::{BiomeWeatherMap, BiomeWindProfile, WeatherTransition};
use astraweave_terrain::biome::BiomeType;
use glam::Vec3;

// ═══════════════════════════════════════════════════════════════════════
// WeatherTransition defaults & state machine
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn weather_transition_default_duration() {
    assert_eq!(WeatherTransition::default().duration(), 3.0);
}
#[test]
fn weather_transition_new_clamps_tiny_duration() {
    let wt = WeatherTransition::new(0.001);
    assert_eq!(wt.duration(), 0.01);
}
#[test]
fn weather_transition_starts_inactive() {
    let wt = WeatherTransition::new(2.0);
    assert!(!wt.is_active());
    assert_eq!(wt.current_kind(), WeatherKind::None);
}
#[test]
fn weather_transition_start_activates() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::None, WeatherKind::Snow);
    assert!(wt.is_active());
    assert_eq!(wt.from_kind(), WeatherKind::None);
    assert_eq!(wt.to_kind(), WeatherKind::Snow);
    assert!((wt.progress() - 0.0).abs() < 0.001);
}
#[test]
fn weather_transition_same_kind_noop() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::Rain, WeatherKind::Rain);
    assert!(!wt.is_active());
}
#[test]
fn weather_transition_update_progresses() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(1.0); // 50%
    assert!((wt.progress() - 0.5).abs() < 0.01);
    assert!(wt.is_active());
}
#[test]
fn weather_transition_completes() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(2.0);
    assert!(!wt.is_active());
    assert_eq!(wt.progress(), 1.0);
    assert_eq!(wt.from_kind(), WeatherKind::Rain);
}
#[test]
fn weather_transition_eased_progress_smoothstep() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::None, WeatherKind::Snow);
    wt.update(1.0); // 50%
    let t = 0.5_f32;
    let expected = t * t * (3.0 - 2.0 * t);
    assert!((wt.eased_progress() - expected).abs() < 0.01);
}
#[test]
fn weather_transition_multipliers_at_start() {
    let mut wt = WeatherTransition::new(1.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    let (fog, ambient) = wt.current_multipliers();
    // At t=0 eased=0 → fully from (None: fog=1.0, ambient=1.0)
    assert!((fog - 1.0).abs() < 0.01);
    assert!((ambient - 1.0).abs() < 0.01);
}
#[test]
fn weather_transition_multipliers_at_end() {
    let mut wt = WeatherTransition::new(1.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(1.0); // complete
    let (fog, ambient) = wt.current_multipliers();
    // None→Rain: fog=2.5, ambient=0.6
    assert!((fog - 2.5).abs() < 0.01);
    assert!((ambient - 0.6).abs() < 0.01);
}
#[test]
fn weather_transition_particle_density_rain() {
    let mut wt = WeatherTransition::new(1.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(1.0);
    assert!((wt.current_particle_density() - 1.0).abs() < 0.01);
}
#[test]
fn weather_transition_particle_density_none() {
    let mut wt = WeatherTransition::new(1.0);
    wt.start(WeatherKind::Rain, WeatherKind::None);
    wt.update(1.0);
    assert!((wt.current_particle_density() - 0.0).abs() < 0.01);
}
#[test]
fn weather_transition_outgoing_fade_active() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::Rain, WeatherKind::Snow);
    wt.update(1.0); // 50%
    let fade = wt.outgoing_particle_fade();
    assert!(fade > 0.0 && fade < 1.0);
}
#[test]
fn weather_transition_outgoing_fade_inactive() {
    let wt = WeatherTransition::new(2.0);
    assert_eq!(wt.outgoing_particle_fade(), 0.0);
}
#[test]
fn weather_transition_incoming_fade_inactive() {
    let wt = WeatherTransition::new(2.0);
    assert_eq!(wt.incoming_particle_fade(), 1.0);
}
#[test]
fn weather_transition_complete_snaps() {
    let mut wt = WeatherTransition::new(5.0);
    wt.start(WeatherKind::None, WeatherKind::Sandstorm);
    wt.update(0.1);
    wt.complete();
    assert!(!wt.is_active());
    assert_eq!(wt.progress(), 1.0);
    assert_eq!(wt.from_kind(), WeatherKind::Sandstorm);
}
#[test]
fn weather_transition_set_duration() {
    let mut wt = WeatherTransition::new(1.0);
    wt.set_duration(5.0);
    assert_eq!(wt.duration(), 5.0);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeWeatherMap — weight table counts per biome
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn weather_map_forest_weights_count() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Forest).len(), 4);
}
#[test]
fn weather_map_desert_weights_count() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Desert).len(), 4);
}
#[test]
fn weather_map_grassland_weights_count() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Grassland).len(), 4);
}
#[test]
fn weather_map_mountain_weights_count() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Mountain).len(), 4);
}
#[test]
fn weather_map_tundra_weights_count() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Tundra).len(), 4);
}
#[test]
fn weather_map_swamp_weights_count() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Swamp).len(), 4);
}
#[test]
fn weather_map_beach_weights_count() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Beach).len(), 4);
}
#[test]
fn weather_map_river_weights_count() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::River).len(), 4);
}

// Pin exact weight values per biome
#[test]
fn weather_map_forest_exact_weights() {
    let w = BiomeWeatherMap::weights(BiomeType::Forest);
    assert_eq!(w[0].kind, WeatherKind::None);
    assert_eq!(w[0].weight, 0.45);
    assert_eq!(w[1].kind, WeatherKind::Rain);
    assert_eq!(w[1].weight, 0.35);
    assert_eq!(w[2].kind, WeatherKind::WindTrails);
    assert_eq!(w[2].weight, 0.15);
    assert_eq!(w[3].kind, WeatherKind::Snow);
    assert_eq!(w[3].weight, 0.05);
}
#[test]
fn weather_map_desert_exact_weights() {
    let w = BiomeWeatherMap::weights(BiomeType::Desert);
    assert_eq!(w[0].kind, WeatherKind::None);
    assert_eq!(w[0].weight, 0.60);
    assert_eq!(w[1].kind, WeatherKind::Sandstorm);
    assert_eq!(w[1].weight, 0.25);
    assert_eq!(w[2].kind, WeatherKind::WindTrails);
    assert_eq!(w[2].weight, 0.12);
    assert_eq!(w[3].kind, WeatherKind::Rain);
    assert_eq!(w[3].weight, 0.03);
}
#[test]
fn weather_map_mountain_exact_weights() {
    let w = BiomeWeatherMap::weights(BiomeType::Mountain);
    assert_eq!(w[0].kind, WeatherKind::None);
    assert_eq!(w[0].weight, 0.30);
    assert_eq!(w[1].kind, WeatherKind::Snow);
    assert_eq!(w[1].weight, 0.30);
    assert_eq!(w[2].kind, WeatherKind::WindTrails);
    assert_eq!(w[2].weight, 0.25);
    assert_eq!(w[3].kind, WeatherKind::Rain);
    assert_eq!(w[3].weight, 0.15);
}
#[test]
fn weather_map_tundra_exact_weights() {
    let w = BiomeWeatherMap::weights(BiomeType::Tundra);
    assert_eq!(w[0].kind, WeatherKind::None);
    assert_eq!(w[0].weight, 0.25);
    assert_eq!(w[1].kind, WeatherKind::Snow);
    assert_eq!(w[1].weight, 0.50);
    assert_eq!(w[2].kind, WeatherKind::WindTrails);
    assert_eq!(w[2].weight, 0.20);
    assert_eq!(w[3].kind, WeatherKind::Rain);
    assert_eq!(w[3].weight, 0.05);
}
#[test]
fn weather_map_swamp_exact_weights() {
    let w = BiomeWeatherMap::weights(BiomeType::Swamp);
    assert_eq!(w[0].kind, WeatherKind::None);
    assert_eq!(w[0].weight, 0.20);
    assert_eq!(w[1].kind, WeatherKind::Rain);
    assert_eq!(w[1].weight, 0.55);
    assert_eq!(w[2].kind, WeatherKind::WindTrails);
    assert_eq!(w[2].weight, 0.15);
    assert_eq!(w[3].kind, WeatherKind::Snow);
    assert_eq!(w[3].weight, 0.10);
}
#[test]
fn weather_map_beach_exact_weights() {
    let w = BiomeWeatherMap::weights(BiomeType::Beach);
    assert_eq!(w[0].kind, WeatherKind::None);
    assert_eq!(w[0].weight, 0.55);
    assert_eq!(w[1].kind, WeatherKind::Rain);
    assert_eq!(w[1].weight, 0.20);
    assert_eq!(w[2].kind, WeatherKind::WindTrails);
    assert_eq!(w[2].weight, 0.20);
    assert_eq!(w[3].kind, WeatherKind::Snow);
    assert_eq!(w[3].weight, 0.05);
}
#[test]
fn weather_map_river_exact_weights() {
    let w = BiomeWeatherMap::weights(BiomeType::River);
    assert_eq!(w[0].kind, WeatherKind::None);
    assert_eq!(w[0].weight, 0.45);
    assert_eq!(w[1].kind, WeatherKind::Rain);
    assert_eq!(w[1].weight, 0.30);
    assert_eq!(w[2].kind, WeatherKind::WindTrails);
    assert_eq!(w[2].weight, 0.20);
    assert_eq!(w[3].kind, WeatherKind::Snow);
    assert_eq!(w[3].weight, 0.05);
}
#[test]
fn weather_map_grassland_exact_weights() {
    let w = BiomeWeatherMap::weights(BiomeType::Grassland);
    assert_eq!(w[0].kind, WeatherKind::None);
    assert_eq!(w[0].weight, 0.50);
    assert_eq!(w[1].kind, WeatherKind::Rain);
    assert_eq!(w[1].weight, 0.25);
    assert_eq!(w[2].kind, WeatherKind::WindTrails);
    assert_eq!(w[2].weight, 0.20);
    assert_eq!(w[3].kind, WeatherKind::Snow);
    assert_eq!(w[3].weight, 0.05);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeWeatherMap::pick — boundary sampling
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn weather_pick_tundra_low_roll_is_none() {
    // roll=0.0 → first bucket (None, weight=0.25)
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Tundra, 0.0),
        WeatherKind::None
    );
}
#[test]
fn weather_pick_tundra_mid_roll_is_snow() {
    // After None(0.25), next is Snow(0.50) → roll at 0.3 should be Snow
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Tundra, 0.3),
        WeatherKind::Snow
    );
}
#[test]
fn weather_pick_desert_sandstorm_range() {
    // None=0.60, Sandstorm=0.25, so Sandstorm range is [0.60, 0.85)
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.7),
        WeatherKind::Sandstorm
    );
}
#[test]
fn weather_pick_swamp_rain_dominant() {
    // Swamp: None=0.20, Rain=0.55 → Rain is at [0.20, 0.75)
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Swamp, 0.5),
        WeatherKind::Rain
    );
}
#[test]
fn weather_pick_forest_last_bucket() {
    // Forest: None=0.45, Rain=0.35, WindTrails=0.15, Snow=0.05
    // Snow range: [0.95, 1.0)
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.96),
        WeatherKind::Snow
    );
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeWeatherMap::most_likely
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn weather_most_likely_swamp_is_rain() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Swamp),
        WeatherKind::Rain
    );
}
#[test]
fn weather_most_likely_tundra_is_snow() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Tundra),
        WeatherKind::Snow
    );
}
#[test]
fn weather_most_likely_desert_is_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Desert),
        WeatherKind::None
    );
}
#[test]
fn weather_most_likely_beach_is_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Beach),
        WeatherKind::None
    );
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeWeatherMap::probability
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn weather_probability_tundra_snow() {
    assert!(
        (BiomeWeatherMap::probability(BiomeType::Tundra, WeatherKind::Snow) - 0.50).abs() < 0.01
    );
}
#[test]
fn weather_probability_desert_sandstorm() {
    assert!(
        (BiomeWeatherMap::probability(BiomeType::Desert, WeatherKind::Sandstorm) - 0.25).abs()
            < 0.01
    );
}
#[test]
fn weather_probability_forest_snow_small() {
    assert!(
        (BiomeWeatherMap::probability(BiomeType::Forest, WeatherKind::Snow) - 0.05).abs() < 0.01
    );
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeWindProfile defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn wind_default_base_strength() {
    assert_eq!(BiomeWindProfile::default().base_strength, 1.0);
}
#[test]
fn wind_default_gusty() {
    assert!(!BiomeWindProfile::default().gusty);
}
#[test]
fn wind_default_gust_frequency() {
    assert_eq!(BiomeWindProfile::default().gust_frequency, 0.0);
}
#[test]
fn wind_default_gust_variance() {
    assert_eq!(BiomeWindProfile::default().gust_variance, 0.0);
}
#[test]
fn wind_default_direction() {
    let d = BiomeWindProfile::default().dominant_direction;
    assert_eq!(d, Vec3::new(1.0, 0.0, 0.0));
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeWindProfile::for_biome — all 8 biomes exact values
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn wind_forest() {
    let w = BiomeWindProfile::for_biome(BiomeType::Forest);
    assert_eq!(w.base_strength, 0.4);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.15);
    assert_eq!(w.gust_variance, 0.6);
    assert_eq!(w.dominant_direction, Vec3::new(0.8, 0.0, 0.6));
}
#[test]
fn wind_desert() {
    let w = BiomeWindProfile::for_biome(BiomeType::Desert);
    assert_eq!(w.base_strength, 1.8);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.3);
    assert_eq!(w.gust_variance, 1.5);
    assert_eq!(w.dominant_direction, Vec3::new(1.0, 0.0, 0.2));
}
#[test]
fn wind_grassland() {
    let w = BiomeWindProfile::for_biome(BiomeType::Grassland);
    assert_eq!(w.base_strength, 1.2);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.2);
    assert_eq!(w.gust_variance, 0.8);
    assert_eq!(w.dominant_direction, Vec3::new(0.9, 0.0, 0.4));
}
#[test]
fn wind_mountain() {
    let w = BiomeWindProfile::for_biome(BiomeType::Mountain);
    assert_eq!(w.base_strength, 2.5);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.4);
    assert_eq!(w.gust_variance, 2.0);
    assert_eq!(w.dominant_direction, Vec3::new(0.6, 0.0, 0.8));
}
#[test]
fn wind_tundra() {
    let w = BiomeWindProfile::for_biome(BiomeType::Tundra);
    assert_eq!(w.base_strength, 2.0);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.25);
    assert_eq!(w.gust_variance, 1.2);
    assert_eq!(w.dominant_direction, Vec3::new(0.0, 0.0, 1.0));
}
#[test]
fn wind_swamp() {
    let w = BiomeWindProfile::for_biome(BiomeType::Swamp);
    assert_eq!(w.base_strength, 0.3);
    assert!(!w.gusty);
    assert_eq!(w.gust_frequency, 0.05);
    assert_eq!(w.gust_variance, 0.2);
    assert_eq!(w.dominant_direction, Vec3::new(0.5, 0.0, 0.5));
}
#[test]
fn wind_beach() {
    let w = BiomeWindProfile::for_biome(BiomeType::Beach);
    assert_eq!(w.base_strength, 1.5);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.2);
    assert_eq!(w.gust_variance, 0.9);
    assert_eq!(w.dominant_direction, Vec3::new(-1.0, 0.0, 0.0));
}
#[test]
fn wind_river() {
    let w = BiomeWindProfile::for_biome(BiomeType::River);
    assert_eq!(w.base_strength, 0.8);
    assert!(!w.gusty);
    assert_eq!(w.gust_frequency, 0.1);
    assert_eq!(w.gust_variance, 0.4);
    assert_eq!(w.dominant_direction, Vec3::new(0.7, 0.0, 0.7));
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeWindProfile::effective_strength — formulas
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn wind_effective_strength_non_gusty_returns_base() {
    let w = BiomeWindProfile::for_biome(BiomeType::Swamp); // gusty=false
    assert_eq!(w.effective_strength(100.0), w.base_strength);
}
#[test]
fn wind_effective_strength_gusty_at_zero_time() {
    let w = BiomeWindProfile::for_biome(BiomeType::Forest);
    // phase=0*0.15*TAU=0, sin(0)=0, gust=max(0,0)=0
    assert_eq!(w.effective_strength(0.0), w.base_strength);
}
#[test]
fn wind_effective_strength_gusty_at_peak() {
    let w = BiomeWindProfile::for_biome(BiomeType::Mountain);
    // Peak when sin(phase)=1, so phase=PI/2 → time*0.4*TAU=PI/2
    // → time = PI/2 / (0.4*TAU) = 0.25/0.4 = 0.625
    let peak_time = std::f32::consts::FRAC_PI_2 / (w.gust_frequency * std::f32::consts::TAU);
    let strength = w.effective_strength(peak_time);
    let expected = w.base_strength + w.gust_variance; // 2.5+2.0=4.5
    assert!((strength - expected).abs() < 0.01);
}
#[test]
fn wind_effective_strength_gusty_negative_sin_clamps() {
    let w = BiomeWindProfile::for_biome(BiomeType::Desert);
    // At sin(phase)<0, max(0,sin)=0, so effective = base_strength
    // phase=3*PI/2 → time = 3*PI/2 / (0.3*TAU)
    let neg_time = 3.0 * std::f32::consts::FRAC_PI_2 / (w.gust_frequency * std::f32::consts::TAU);
    let strength = w.effective_strength(neg_time);
    assert!((strength - w.base_strength).abs() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeWindProfile::effective_direction
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn wind_effective_direction_non_gusty_is_normalized_dominant() {
    let w = BiomeWindProfile::for_biome(BiomeType::River); // gusty=false
    let dir = w.effective_direction(42.0);
    let expected = w.dominant_direction.normalize();
    assert!((dir - expected).length() < 0.01);
}
#[test]
fn wind_effective_direction_gusty_at_zero_no_rotation() {
    let w = BiomeWindProfile::for_biome(BiomeType::Forest);
    let dir = w.effective_direction(0.0);
    let expected = w.dominant_direction.normalize();
    assert!((dir - expected).length() < 0.01);
}
#[test]
fn wind_effective_direction_is_unit_length() {
    for biome in [
        BiomeType::Forest,
        BiomeType::Desert,
        BiomeType::Mountain,
        BiomeType::Tundra,
    ] {
        let wp = BiomeWindProfile::for_biome(biome);
        for t in [0.0_f32, 1.0, 2.5, 10.0] {
            let dir = wp.effective_direction(t);
            assert!(
                (dir.length() - 1.0).abs() < 0.01,
                "dir not unit for {biome:?} at t={t}"
            );
        }
    }
}
