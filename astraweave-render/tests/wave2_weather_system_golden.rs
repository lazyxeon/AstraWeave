//! Wave 2 – Golden-value tests for weather_system.rs (162 mutants)
//!
//! Targets: WeatherTransition eased progress golden values,
//!          exact current_multipliers through lifecycle,
//!          BiomeWeatherMap::pick boundary thresholds per biome,
//!          BiomeWindProfile per-biome golden field values,
//!          effective_strength/direction golden outputs.
//!
//! Complements the 32+ inline unit tests with exact golden-value assertions
//! through the public API.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::effects::WeatherKind;
use astraweave_render::weather_system::{BiomeWeatherMap, BiomeWindProfile, WeatherTransition};
use astraweave_terrain::biome::BiomeType;

// ============================================================================
// WeatherTransition — eased_progress golden values
// ============================================================================

#[test]
fn eased_progress_at_zero() {
    let mut wt = WeatherTransition::new(4.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    // progress = 0.0 → smoothstep(0) = 0
    assert_eq!(wt.eased_progress(), 0.0);
}

#[test]
fn eased_progress_at_quarter() {
    let mut wt = WeatherTransition::new(4.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(1.0); // 25% linear
                    // smoothstep(0.25) = 0.25² * (3 - 2*0.25) = 0.0625 * 2.5 = 0.15625
    let ep = wt.eased_progress();
    assert!(
        (ep - 0.15625).abs() < 0.001,
        "eased_progress at 25% should be ~0.15625, got {}",
        ep
    );
}

#[test]
fn eased_progress_at_half() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(1.0); // 50%
    let ep = wt.eased_progress();
    assert!(
        (ep - 0.5).abs() < 0.001,
        "eased at 50% should be 0.5, got {}",
        ep
    );
}

#[test]
fn eased_progress_at_three_quarters() {
    let mut wt = WeatherTransition::new(4.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(3.0); // 75% linear
                    // smoothstep(0.75) = 0.75² * (3 - 1.5) = 0.5625 * 1.5 = 0.84375
    let ep = wt.eased_progress();
    assert!(
        (ep - 0.84375).abs() < 0.001,
        "eased at 75% should be ~0.84375, got {}",
        ep
    );
}

// ============================================================================
// WeatherTransition — multiplier lifecycle
// ============================================================================

#[test]
fn multipliers_none_to_rain_at_quarter() {
    let mut wt = WeatherTransition::new(4.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(1.0); // 25% linear → eased ≈ 0.15625
    let (fog, ambient) = wt.current_multipliers();
    // fog = lerp(1.0, 2.5, 0.15625) = 1.0 + 1.5*0.15625 = 1.234375
    assert!(
        (fog - 1.234).abs() < 0.01,
        "fog at 25% should be ~1.234, got {}",
        fog
    );
    // ambient = lerp(1.0, 0.6, 0.15625) = 1.0 - 0.4*0.15625 = 0.9375
    assert!(
        (ambient - 0.9375).abs() < 0.01,
        "ambient at 25% should be ~0.9375, got {}",
        ambient
    );
}

#[test]
fn multipliers_none_to_sandstorm_complete() {
    let mut wt = WeatherTransition::new(1.0);
    wt.start(WeatherKind::None, WeatherKind::Sandstorm);
    wt.update(1.0);
    let (fog, ambient) = wt.current_multipliers();
    // Sandstorm: fog=4.0, ambient=0.4
    assert!((fog - 4.0).abs() < 0.01, "sandstorm fog = {}", fog);
    assert!(
        (ambient - 0.4).abs() < 0.01,
        "sandstorm ambient = {}",
        ambient
    );
}

#[test]
fn multipliers_rain_to_snow_midpoint() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::Rain, WeatherKind::Snow);
    wt.update(1.0); // 50%
    let (fog, ambient) = wt.current_multipliers();
    // Rain fog=2.5, Snow fog=1.8 → midpoint = 2.15
    assert!((fog - 2.15).abs() < 0.05, "fog mid rain→snow: {}", fog);
    // Rain amb=0.6, Snow amb=0.75 → midpoint = 0.675
    assert!((ambient - 0.675).abs() < 0.05, "ambient mid: {}", ambient);
}

#[test]
fn particle_density_none_to_rain_midpoint() {
    let mut wt = WeatherTransition::new(2.0);
    wt.start(WeatherKind::None, WeatherKind::Rain);
    wt.update(1.0); // 50%
                    // None density=0, Rain density=1.0 → midpoint = 0.5
    let pd = wt.current_particle_density();
    assert!((pd - 0.5).abs() < 0.05, "particle density mid: {}", pd);
}

#[test]
fn particle_fade_outgoing_and_incoming_sum_to_one() {
    let mut wt = WeatherTransition::new(4.0);
    wt.start(WeatherKind::Rain, WeatherKind::Snow);
    for i in 1..=3 {
        wt.update(1.0);
        let out = wt.outgoing_particle_fade();
        let inc = wt.incoming_particle_fade();
        assert!(
            (out + inc - 1.0).abs() < 0.001,
            "Step {}: out+inc = {} + {} = {}",
            i,
            out,
            inc,
            out + inc
        );
    }
}

#[test]
fn outgoing_fade_zero_when_inactive() {
    let wt = WeatherTransition::new(1.0);
    assert_eq!(wt.outgoing_particle_fade(), 0.0);
}

#[test]
fn incoming_fade_one_when_inactive() {
    let wt = WeatherTransition::new(1.0);
    assert_eq!(wt.incoming_particle_fade(), 1.0);
}

// ============================================================================
// WeatherTransition — duration edge cases
// ============================================================================

#[test]
fn duration_negative_clamped() {
    let wt = WeatherTransition::new(-100.0);
    assert!(wt.duration() >= 0.01);
}

#[test]
fn duration_zero_clamped() {
    let wt = WeatherTransition::new(0.0);
    assert!(wt.duration() >= 0.01);
}

#[test]
fn set_duration_negative_clamped() {
    let mut wt = WeatherTransition::new(5.0);
    wt.set_duration(-10.0);
    assert!(wt.duration() >= 0.01);
}

#[test]
fn default_duration_3s() {
    let wt = WeatherTransition::default();
    assert_eq!(wt.duration(), 3.0);
}

// ============================================================================
// BiomeWeatherMap::pick — boundary thresholds per biome
// ============================================================================

// Forest: [None=0.45, Rain=0.35, WindTrails=0.15, Snow=0.05]
#[test]
fn pick_forest_none_at_zero() {
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.0),
        WeatherKind::None
    );
}

#[test]
fn pick_forest_rain_at_046() {
    // Cumulative: 0.45 → 0.46 falls in Rain range [0.45, 0.80)
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.46),
        WeatherKind::Rain
    );
}

#[test]
fn pick_forest_wind_at_085() {
    // Rain ends at 0.80, WindTrails [0.80, 0.95)
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.85),
        WeatherKind::WindTrails
    );
}

#[test]
fn pick_forest_snow_at_096() {
    // WindTrails ends at 0.95, Snow [0.95, 1.0)
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Forest, 0.96),
        WeatherKind::Snow
    );
}

// Desert: [None=0.60, Sandstorm=0.25, WindTrails=0.12, Rain=0.03]
#[test]
fn pick_desert_none_at_050() {
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.50),
        WeatherKind::None
    );
}

#[test]
fn pick_desert_sandstorm_at_070() {
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.70),
        WeatherKind::Sandstorm
    );
}

#[test]
fn pick_desert_windtrails_at_090() {
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Desert, 0.90),
        WeatherKind::WindTrails
    );
}

// Tundra: [None=0.25, Snow=0.50, WindTrails=0.20, Rain=0.05]
#[test]
fn pick_tundra_snow_at_030() {
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Tundra, 0.30),
        WeatherKind::Snow
    );
}

#[test]
fn pick_tundra_wind_at_080() {
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Tundra, 0.80),
        WeatherKind::WindTrails
    );
}

// Swamp: [None=0.20, Rain=0.55, WindTrails=0.15, Snow=0.10]
#[test]
fn pick_swamp_rain_at_030() {
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Swamp, 0.30),
        WeatherKind::Rain
    );
}

#[test]
fn pick_swamp_wind_at_080() {
    assert_eq!(
        BiomeWeatherMap::pick(BiomeType::Swamp, 0.80),
        WeatherKind::WindTrails
    );
}

// ============================================================================
// BiomeWeatherMap::probability — exact fractions
// ============================================================================

#[test]
fn probability_forest_snow_golden() {
    let p = BiomeWeatherMap::probability(BiomeType::Forest, WeatherKind::Snow);
    assert!((p - 0.05).abs() < 0.01, "Forest snow prob: {}", p);
}

#[test]
fn probability_desert_sandstorm_golden() {
    let p = BiomeWeatherMap::probability(BiomeType::Desert, WeatherKind::Sandstorm);
    assert!((p - 0.25).abs() < 0.01, "Desert sandstorm prob: {}", p);
}

#[test]
fn probability_tundra_snow_golden() {
    let p = BiomeWeatherMap::probability(BiomeType::Tundra, WeatherKind::Snow);
    assert!((p - 0.50).abs() < 0.01, "Tundra snow prob: {}", p);
}

#[test]
fn probability_missing_kind_is_zero() {
    // Desert has no Snow entry (well, actually it doesn't — check!)
    // Actually Desert: None, Sandstorm, WindTrails, Rain — NO Snow
    let p = BiomeWeatherMap::probability(BiomeType::Desert, WeatherKind::Snow);
    assert_eq!(p, 0.0, "Desert has no snow");
}

// ============================================================================
// BiomeWeatherMap::most_likely
// ============================================================================

#[test]
fn most_likely_forest_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Forest),
        WeatherKind::None
    );
}

#[test]
fn most_likely_grassland_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Grassland),
        WeatherKind::None
    );
}

#[test]
fn most_likely_beach_none() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Beach),
        WeatherKind::None
    );
}

#[test]
fn most_likely_swamp_rain() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Swamp),
        WeatherKind::Rain
    );
}

#[test]
fn most_likely_tundra_snow() {
    assert_eq!(
        BiomeWeatherMap::most_likely(BiomeType::Tundra),
        WeatherKind::Snow
    );
}

// ============================================================================
// BiomeWindProfile — per-biome golden field values
// ============================================================================

#[test]
fn wind_forest_golden() {
    let w = BiomeWindProfile::for_biome(BiomeType::Forest);
    assert_eq!(w.base_strength, 0.4);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.15);
    assert_eq!(w.gust_variance, 0.6);
}

#[test]
fn wind_desert_golden() {
    let w = BiomeWindProfile::for_biome(BiomeType::Desert);
    assert_eq!(w.base_strength, 1.8);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.3);
    assert_eq!(w.gust_variance, 1.5);
}

#[test]
fn wind_mountain_golden() {
    let w = BiomeWindProfile::for_biome(BiomeType::Mountain);
    assert_eq!(w.base_strength, 2.5);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.4);
    assert_eq!(w.gust_variance, 2.0);
}

#[test]
fn wind_tundra_golden() {
    let w = BiomeWindProfile::for_biome(BiomeType::Tundra);
    assert_eq!(w.base_strength, 2.0);
    assert!(w.gusty);
    assert_eq!(w.gust_frequency, 0.25);
    assert_eq!(w.gust_variance, 1.2);
    // Tundra: wind from the pole → (0,0,1)
    assert_eq!(w.dominant_direction.x, 0.0);
    assert_eq!(w.dominant_direction.z, 1.0);
}

#[test]
fn wind_swamp_golden() {
    let w = BiomeWindProfile::for_biome(BiomeType::Swamp);
    assert_eq!(w.base_strength, 0.3);
    assert!(!w.gusty);
}

#[test]
fn wind_beach_golden() {
    let w = BiomeWindProfile::for_biome(BiomeType::Beach);
    assert_eq!(w.base_strength, 1.5);
    assert!(w.gusty);
    // Beach: onshore wind → (-1, 0, 0)
    assert_eq!(w.dominant_direction.x, -1.0);
    assert_eq!(w.dominant_direction.z, 0.0);
}

#[test]
fn wind_default_golden() {
    let w = BiomeWindProfile::default();
    assert_eq!(w.base_strength, 1.0);
    assert!(!w.gusty);
    assert_eq!(w.gust_frequency, 0.0);
    assert_eq!(w.gust_variance, 0.0);
}

// ============================================================================
// BiomeWindProfile — effective_strength golden values
// ============================================================================

#[test]
fn effective_strength_swamp_constant() {
    let w = BiomeWindProfile::for_biome(BiomeType::Swamp);
    for t in [0.0, 1.0, 5.0, 100.0] {
        assert_eq!(
            w.effective_strength(t),
            w.base_strength,
            "Swamp (non-gusty) should always be base_strength at t={}",
            t
        );
    }
}

#[test]
fn effective_strength_at_time_zero() {
    // At t=0, sin(0) = 0 → gust = 0 → returns base_strength
    let w = BiomeWindProfile::for_biome(BiomeType::Mountain);
    let s = w.effective_strength(0.0);
    assert_eq!(s, w.base_strength, "At t=0, sin(0)=0, no gust");
}

#[test]
fn effective_strength_bounded() {
    // Mountain: base=2.5, gust_var=2.0 → max = 2.5 + 2.0 = 4.5
    let w = BiomeWindProfile::for_biome(BiomeType::Mountain);
    for i in 0..200 {
        let s = w.effective_strength(i as f32 * 0.05);
        assert!(
            s >= w.base_strength,
            "strength {} < base {}",
            s,
            w.base_strength
        );
        assert!(
            s <= w.base_strength + w.gust_variance + 0.01,
            "strength {} > base+var {}",
            s,
            w.base_strength + w.gust_variance
        );
    }
}

// ============================================================================
// BiomeWindProfile — effective_direction
// ============================================================================

#[test]
fn effective_direction_swamp_is_normalized_dominant() {
    let w = BiomeWindProfile::for_biome(BiomeType::Swamp);
    let d = w.effective_direction(0.0);
    let expected = w.dominant_direction.normalize();
    assert!((d - expected).length() < 0.001);
}

#[test]
fn effective_direction_always_normalized() {
    for biome in BiomeType::all() {
        let w = BiomeWindProfile::for_biome(*biome);
        for i in 0..20 {
            let d = w.effective_direction(i as f32 * 0.5);
            let len = d.length();
            assert!(
                (len - 1.0).abs() < 0.01,
                "{:?} direction length {} at t={}",
                biome,
                len,
                i as f32 * 0.5
            );
        }
    }
}

// ============================================================================
// BiomeWeatherMap::weights — table sizes
// ============================================================================

#[test]
fn all_biomes_have_weights() {
    for biome in BiomeType::all() {
        let w = BiomeWeatherMap::weights(*biome);
        assert!(
            !w.is_empty(),
            "{:?} should have at least one weather weight",
            biome
        );
    }
}

#[test]
fn weights_all_positive() {
    for biome in BiomeType::all() {
        for entry in BiomeWeatherMap::weights(*biome) {
            assert!(
                entry.weight > 0.0,
                "{:?} has zero weight for {:?}",
                biome,
                entry.kind
            );
        }
    }
}

#[test]
fn forest_has_4_weather_entries() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Forest).len(), 4);
}

#[test]
fn desert_has_4_weather_entries() {
    assert_eq!(BiomeWeatherMap::weights(BiomeType::Desert).len(), 4);
}
