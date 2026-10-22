//! Wave 2 weather_system remediation — golden values for multiplier constants,
//! wind profile match arms, and boundary/edge-case arithmetic.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::effects::WeatherKind;
use astraweave_render::weather_system::{BiomeWeatherMap, BiomeWindProfile, WeatherTransition};
use astraweave_terrain::biome::BiomeType;
use glam::Vec3;

// ══════════════════════════════════════════════════════════════════════════════
// WeatherTransition — multiplier golden values per kind
// ══════════════════════════════════════════════════════════════════════════════

fn transition_to(kind: WeatherKind) -> (f32, f32, f32) {
    let mut wt = WeatherTransition::new(1.0);
    wt.start(WeatherKind::None, kind);
    wt.update(1.0); // complete
    let (fog, ambient) = wt.current_multipliers();
    let density = wt.current_particle_density();
    (fog, ambient, density)
}

#[test]
fn multipliers_rain() {
    let (fog, ambient, density) = transition_to(WeatherKind::Rain);
    assert!((fog - 2.5).abs() < 0.01, "rain fog={fog}");
    assert!((ambient - 0.6).abs() < 0.01, "rain ambient={ambient}");
    assert!((density - 1.0).abs() < 0.01, "rain density={density}");
}

#[test]
fn multipliers_snow() {
    let (fog, ambient, density) = transition_to(WeatherKind::Snow);
    assert!((fog - 1.8).abs() < 0.01, "snow fog={fog}");
    assert!((ambient - 0.75).abs() < 0.01, "snow ambient={ambient}");
    assert!((density - 1.0).abs() < 0.01, "snow density={density}");
}

#[test]
fn multipliers_sandstorm() {
    let (fog, ambient, density) = transition_to(WeatherKind::Sandstorm);
    assert!((fog - 4.0).abs() < 0.01, "sandstorm fog={fog}");
    assert!((ambient - 0.4).abs() < 0.01, "sandstorm ambient={ambient}");
    assert!((density - 1.0).abs() < 0.01, "sandstorm density={density}");
}

#[test]
fn multipliers_windtrails() {
    let (fog, ambient, density) = transition_to(WeatherKind::WindTrails);
    assert!((fog - 1.4).abs() < 0.01, "windtrails fog={fog}");
    assert!((ambient - 0.9).abs() < 0.01, "windtrails ambient={ambient}");
    assert!((density - 0.6).abs() < 0.01, "windtrails density={density}");
}

#[test]
fn multipliers_none() {
    let (fog, ambient, density) = transition_to(WeatherKind::None);
    // None→None = noop so it stays at None multipliers
    let wt = WeatherTransition::new(1.0);
    let (f, a) = wt.current_multipliers();
    assert!((f - 1.0).abs() < 0.01);
    assert!((a - 1.0).abs() < 0.01);
    // density for None is 0
    assert!((wt.current_particle_density() - 0.0).abs() < 0.01);
    let _ = (fog, ambient, density);
}

// ══════════════════════════════════════════════════════════════════════════════
// WeatherTransition — smoothstep golden values
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn eased_progress_at_boundaries() {
    let mut wt = WeatherTransition::new(4.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    // t=0 → eased=0
    assert!((wt.eased_progress()).abs() < 1e-4);
    // t=1.0 (progress=0.25) → smoothstep(0.25) = 0.25^2*(3-2*0.25) = 0.0625*2.5 = 0.15625
    wt.update(1.0);
    assert!(
        (wt.eased_progress() - 0.15625).abs() < 0.01,
        "eased at 0.25: {}",
        wt.eased_progress()
    );
    // t=2.0 (progress=0.5) → smoothstep(0.5) = 0.25*2 = 0.5
    wt.update(1.0);
    assert!(
        (wt.eased_progress() - 0.5).abs() < 0.01,
        "eased at 0.5: {}",
        wt.eased_progress()
    );
}

#[test]
fn outgoing_plus_incoming_equals_one_during_transition() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::Rain, WeatherKind::Snow);
    for i in 1..20 {
        wt.update(0.1);
        if wt.is_active() {
            let sum = wt.outgoing_particle_fade() + wt.incoming_particle_fade();
            assert!((sum - 1.0).abs() < 1e-4, "sum at step {i}: {sum}",);
        }
    }
}

#[test]
fn update_inactive_is_noop() {
    let mut wt = WeatherTransition::new(1.0);
    let progress_before = wt.progress();
    wt.update(100.0);
    assert_eq!(wt.progress(), progress_before);
}

#[test]
fn transition_rate_inversely_proportional_to_duration() {
    // Duration=2s, update by 1s → 50% progress
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(1.0);
    assert!((wt.progress() - 0.5).abs() < 0.01);
    // Duration=4s, update by 1s → 25% progress
    let mut wt2 = WeatherTransition::new(4.0);
    wt2.start(WeatherKind::None, WeatherKind::Rain);
    wt2.update(1.0);
    assert!((wt2.progress() - 0.25).abs() < 0.01);
}

#[test]
fn duration_min_clamp() {
    let mut wt = WeatherTransition::new(0.0);
    assert!(wt.duration() >= 0.01);
    wt.set_duration(-100.0);
    assert!(wt.duration() >= 0.01);
}

// ══════════════════════════════════════════════════════════════════════════════
// BiomeWeatherMap — per-biome probability golden values
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn forest_pick_boundaries() {
    // Forest: None=0.45, Rain=0.35, WindTrails=0.15, Snow=0.05
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.0),
        WeatherKind::None
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.44),
        WeatherKind::None
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.46),
        WeatherKind::Rain
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.79),
        WeatherKind::Rain
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.81),
        WeatherKind::WindTrails
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.96),
        WeatherKind::Snow
    );
}

#[test]
fn desert_pick_boundaries() {
    // Desert: None=0.60, Sandstorm=0.25, WindTrails=0.12, Rain=0.03
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.0),
        WeatherKind::None
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.59),
        WeatherKind::None
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.61),
        WeatherKind::Sandstorm
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.84),
        WeatherKind::Sandstorm
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.86),
        WeatherKind::WindTrails
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.98),
        WeatherKind::Rain
    );
}

#[test]
fn mountain_pick_boundaries() {
    // Mountain: None=0.30, Snow=0.30, WindTrails=0.25, Rain=0.15
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Mountain, 0.0),
        WeatherKind::None
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Mountain, 0.29),
        WeatherKind::None
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Mountain, 0.31),
        WeatherKind::Snow
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Mountain, 0.59),
        WeatherKind::Snow
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Mountain, 0.61),
        WeatherKind::WindTrails
    );
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Mountain, 0.86),
        WeatherKind::Rain
    );
}

#[test]
fn grassland_most_likely_is_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Grassland),
        WeatherKind::None
    );
}

#[test]
fn beach_most_likely_is_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Beach),
        WeatherKind::None
    );
}

#[test]
fn river_most_likely_is_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::River),
        WeatherKind::None
    );
}

#[test]
fn forest_most_likely_is_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Forest),
        WeatherKind::None
    );
}

#[test]
fn probability_desert_sandstorm_25_percent() {
    let p = BiomeWeatherMap::probability(BiomeType::Desert, WeatherKind::Sandstorm);
    assert!((p - 0.25).abs() < 0.01, "got {p}");
}

#[test]
fn probability_tundra_snow_50_percent() {
    let p = BiomeWeatherMap::probability(BiomeType::Tundra, WeatherKind::Snow);
    assert!((p - 0.50).abs() < 0.01, "got {p}");
}

#[test]
fn probability_swamp_rain_55_percent() {
    let p = BiomeWeatherMap::probability(BiomeType::Swamp, WeatherKind::Rain);
    assert!((p - 0.55).abs() < 0.01, "got {p}");
}

#[test]
fn probability_nonexistent_weather_returns_zero() {
    // Desert has no Snow entry
    let p = BiomeWeatherMap::probability(BiomeType::Desert, WeatherKind::Snow);
    assert!((p).abs() < 0.01, "desert snow prob should be ~0, got {p}");
}

// ══════════════════════════════════════════════════════════════════════════════
// BiomeWindProfile — golden values per biome
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn wind_forest_base_strength() {
    let w = BiomeWindProfile::for_biome(BiomeType::Forest);
    assert!((w.base_strength - 0.4).abs() < 0.01);
    assert!(w.gusty);
    assert!((w.gust_frequency - 0.15).abs() < 0.01);
    assert!((w.gust_variance - 0.6).abs() < 0.01);
}

#[test]
fn wind_desert_base_strength() {
    let w = BiomeWindProfile::for_biome(BiomeType::Desert);
    assert!((w.base_strength - 1.8).abs() < 0.01);
    assert!((w.gust_frequency - 0.3).abs() < 0.01);
    assert!((w.gust_variance - 1.5).abs() < 0.01);
}

#[test]
fn wind_mountain_base_strength() {
    let w = BiomeWindProfile::for_biome(BiomeType::Mountain);
    assert!((w.base_strength - 2.5).abs() < 0.01);
    assert!((w.gust_frequency - 0.4).abs() < 0.01);
    assert!((w.gust_variance - 2.0).abs() < 0.01);
}

#[test]
fn wind_tundra_base_strength() {
    let w = BiomeWindProfile::for_biome(BiomeType::Tundra);
    assert!((w.base_strength - 2.0).abs() < 0.01);
    assert!((w.gust_frequency - 0.25).abs() < 0.01);
    assert!((w.gust_variance - 1.2).abs() < 0.01);
}

#[test]
fn wind_swamp_values() {
    let w = BiomeWindProfile::for_biome(BiomeType::Swamp);
    assert!((w.base_strength - 0.3).abs() < 0.01);
    assert!(!w.gusty);
    assert!((w.gust_frequency - 0.05).abs() < 0.01);
    assert!((w.gust_variance - 0.2).abs() < 0.01);
}

#[test]
fn wind_beach_values() {
    let w = BiomeWindProfile::for_biome(BiomeType::Beach);
    assert!((w.base_strength - 1.5).abs() < 0.01);
    assert!(w.gusty);
    assert!((w.gust_frequency - 0.2).abs() < 0.01);
    assert!((w.gust_variance - 0.9).abs() < 0.01);
    // Onshore wind
    assert!((w.dominant_direction.x - (-1.0)).abs() < 0.01);
}

#[test]
fn wind_river_values() {
    let w = BiomeWindProfile::for_biome(BiomeType::River);
    assert!((w.base_strength - 0.8).abs() < 0.01);
    assert!(!w.gusty);
}

#[test]
fn wind_grassland_values() {
    let w = BiomeWindProfile::for_biome(BiomeType::Grassland);
    assert!((w.base_strength - 1.2).abs() < 0.01);
    assert!(w.gusty);
    assert!((w.gust_frequency - 0.2).abs() < 0.01);
}

// ══════════════════════════════════════════════════════════════════════════════
// effective_strength/direction — golden math
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn effective_strength_gust_peak() {
    let w = BiomeWindProfile::for_biome(BiomeType::Mountain);
    // Find the peak gust (sin = 1.0 at quarter period)
    // period = 1/freq = 1/0.4 = 2.5s. Quarter = 0.625s
    // phase = 0.625 * 0.4 * TAU = 0.625 * 2.5133 ≈ 1.5708 ≈ PI/2 → sin = 1.0
    let peak = w.effective_strength(0.625);
    let expected = 2.5 + 1.0 * 2.0; // base + sin(peak)*variance
    assert!(
        (peak - expected).abs() < 0.1,
        "peak={peak}, expected={expected}"
    );
}

#[test]
fn effective_strength_gust_trough() {
    let w = BiomeWindProfile::for_biome(BiomeType::Mountain);
    // At sin = -1 → clamped to 0, so effective = base only
    // 3/4 period = 1.875s → phase = PI*3/2 → sin = -1 → max(0, -1) = 0
    let trough = w.effective_strength(1.875);
    assert!(
        (trough - w.base_strength).abs() < 0.1,
        "trough={trough}, base={}",
        w.base_strength
    );
}

#[test]
fn effective_direction_non_gusty_is_normalized_dominant() {
    let w = BiomeWindProfile::for_biome(BiomeType::Swamp);
    let dir = w.effective_direction(0.0);
    assert!((dir.length() - 1.0).abs() < 0.01);
    // dominant is (0.5, 0, 0.5), normalized = (0.707, 0, 0.707)
    let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
    assert!((dir.x - inv_sqrt2).abs() < 0.01, "x={}", dir.x);
    assert!(dir.y.abs() < 0.01);
    assert!((dir.z - inv_sqrt2).abs() < 0.01, "z={}", dir.z);
}

#[test]
fn effective_direction_default_at_zero_length() {
    // Create default, zero dominant direction — should return Vec3::X
    let profile = BiomeWindProfile {
        dominant_direction: Vec3::ZERO,
        ..Default::default()
    };
    let dir = profile.effective_direction(0.0);
    assert!((dir - Vec3::X).length() < 0.01);
}

#[test]
fn wind_default_values() {
    let d = BiomeWindProfile::default();
    assert!((d.base_strength - 1.0).abs() < 0.01);
    assert!(!d.gusty);
    assert!((d.gust_frequency).abs() < 0.01);
    assert!((d.gust_variance).abs() < 0.01);
    assert!((d.dominant_direction.x - 1.0).abs() < 0.01);
}
