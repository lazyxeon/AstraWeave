//! Wave 2 Render Sweep — Environment & WeatherSystem Remediation Tests
//!
//! Proactive remediation tests targeting the 401 mutants in environment.rs.
//! Tests TimeOfDay (sun/moon position, light color, ambient, day/night/twilight),
//! WeatherSystem (transitions, intensities, terrain color modifier, light attenuation,
//! biome-appropriate weather), and WeatherParticles.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::environment::{TimeOfDay, WeatherParticles, WeatherSystem, WeatherType};

// ─── TimeOfDay: sun position ────────────────────────────────────────────────

#[test]
fn sun_at_noon_is_high() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let sun = tod.get_sun_position();
    // At noon, sun_angle = (12-6)*PI/12 = PI/2 → sin = 1.0 → sun_height = 1.0
    assert!(sun.y > 0.8, "Sun at noon should be high, got y={}", sun.y);
}

#[test]
fn sun_at_midnight_is_below_horizon() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let sun = tod.get_sun_position();
    // At 0:00, sun_angle = (0-6)*PI/12 = -PI/2 → sin = -1.0
    assert!(
        sun.y < -0.5,
        "Sun at midnight should be below horizon, got y={}",
        sun.y
    );
}

#[test]
fn sun_at_sunrise_near_horizon() {
    let tod = TimeOfDay::new(6.0, 1.0);
    let sun = tod.get_sun_position();
    // At 6am, sun_angle = 0 → sin = 0 → near horizon
    assert!(
        sun.y.abs() < 0.2,
        "Sun at 6am should be near horizon, got y={}",
        sun.y
    );
}

#[test]
fn sun_at_sunset_near_horizon() {
    let tod = TimeOfDay::new(18.0, 1.0);
    let sun = tod.get_sun_position();
    // At 6pm, sun_angle = (18-6)*PI/12 = PI → sin = 0 → near horizon
    assert!(
        sun.y.abs() < 0.2,
        "Sun at 6pm should be near horizon, got y={}",
        sun.y
    );
}

#[test]
fn sun_position_is_normalized() {
    for hour in [0.0f32, 3.0, 6.0, 9.0, 12.0, 15.0, 18.0, 21.0] {
        let tod = TimeOfDay::new(hour, 1.0);
        let sun = tod.get_sun_position();
        let len = sun.length();
        assert!(
            (len - 1.0).abs() < 0.05,
            "Sun position should be normalized at hour={hour}, length={len}"
        );
    }
}

// ─── TimeOfDay: moon position ───────────────────────────────────────────────

#[test]
fn moon_opposite_to_sun() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let sun = tod.get_sun_position();
    let moon = tod.get_moon_position();
    // Moon = -Sun
    assert!((moon.x + sun.x).abs() < 0.01, "Moon x should be -sun x");
    assert!((moon.y + sun.y).abs() < 0.01, "Moon y should be -sun y");
    assert!((moon.z + sun.z).abs() < 0.01, "Moon z should be -sun z");
}

#[test]
fn moon_high_at_midnight() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let moon = tod.get_moon_position();
    assert!(
        moon.y > 0.5,
        "Moon at midnight should be high, got y={}",
        moon.y
    );
}

// ─── TimeOfDay: light direction ─────────────────────────────────────────────

#[test]
fn light_direction_from_sun_during_day() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let light = tod.get_light_direction();
    let sun = tod.get_sun_position();
    // Light direction should be -sun (light comes FROM the sun)
    assert!(
        (light.x + sun.x).abs() < 0.01,
        "Light dir should be -sun.x during day"
    );
    assert!(
        (light.y + sun.y).abs() < 0.01,
        "Light dir should be -sun.y during day"
    );
}

// ─── TimeOfDay: light color ─────────────────────────────────────────────────

#[test]
fn light_color_warm_at_noon() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let color = tod.get_light_color();
    // Daytime: warm yellow/white
    assert!(
        color.x > 0.8,
        "Noon light color R should be bright, got {}",
        color.x
    );
    assert!(
        color.y > 0.7,
        "Noon light color G should be bright, got {}",
        color.y
    );
    assert!(
        color.z > 0.5,
        "Noon light color B should be present, got {}",
        color.z
    );
}

#[test]
fn light_color_dim_at_midnight() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let color = tod.get_light_color();
    // Night: cool blue moonlight, very dim
    assert!(
        color.x < 0.2,
        "Midnight light R should be dim, got {}",
        color.x
    );
    assert!(
        color.z > color.x,
        "Night light should be blue-shifted (B > R)"
    );
}

#[test]
fn light_color_components_non_negative() {
    for hour in (0..24).map(|h| h as f32) {
        let tod = TimeOfDay::new(hour, 1.0);
        let color = tod.get_light_color();
        assert!(
            color.x >= 0.0 && color.y >= 0.0 && color.z >= 0.0,
            "Light color at hour={hour} should be non-negative: {:?}",
            color
        );
    }
}

// ─── TimeOfDay: ambient color ───────────────────────────────────────────────

#[test]
fn ambient_color_brighter_in_day() {
    let day = TimeOfDay::new(12.0, 1.0);
    let night = TimeOfDay::new(0.0, 1.0);
    let day_ambient = day.get_ambient_color();
    let night_ambient = night.get_ambient_color();
    let day_lum = day_ambient.x + day_ambient.y + day_ambient.z;
    let night_lum = night_ambient.x + night_ambient.y + night_ambient.z;
    assert!(
        day_lum > night_lum,
        "Day ambient ({day_lum}) should be brighter than night ({night_lum})"
    );
}

// ─── TimeOfDay: day/night/twilight ──────────────────────────────────────────

#[test]
fn is_day_at_noon() {
    let tod = TimeOfDay::new(12.0, 1.0);
    assert!(tod.is_day(), "Noon should be day");
    assert!(!tod.is_night(), "Noon should not be night");
}

#[test]
fn is_night_at_midnight() {
    let tod = TimeOfDay::new(0.0, 1.0);
    assert!(tod.is_night(), "Midnight should be night");
    assert!(!tod.is_day(), "Midnight should not be day");
}

#[test]
fn twilight_near_sunrise_sunset() {
    // Sunrise ~6am, sunset ~18pm
    // Twilight happens when sun_height is in [-0.1, 0.1]
    // At exactly 6am, height=sin(0)=0, should be in range
    let tod = TimeOfDay::new(6.0, 1.0);
    let _sun = tod.get_sun_position();
    // Sun near horizon → twilight OR day boundary
    // At least should NOT be deep night
    assert!(!tod.is_night(), "6am should not be deep night");
}

// ─── TimeOfDay: defaults ────────────────────────────────────────────────────

#[test]
fn time_of_day_default_noon() {
    let tod = TimeOfDay::default();
    assert!(
        (tod.current_time - 12.0).abs() < 0.01,
        "Default should start at noon"
    );
    assert!(
        (tod.time_scale - 60.0).abs() < 0.01,
        "Default time_scale should be 60.0"
    );
    assert!(
        (tod.day_length - 1440.0).abs() < 0.01,
        "Default day_length should be 1440.0"
    );
}

// ─── WeatherSystem: creation & defaults ─────────────────────────────────────

#[test]
fn weather_system_starts_clear() {
    let ws = WeatherSystem::new();
    assert!(matches!(ws.current_weather(), WeatherType::Clear));
    assert!(matches!(ws.target_weather(), WeatherType::Clear));
}

#[test]
fn weather_system_initial_intensities() {
    let ws = WeatherSystem::new();
    assert!(
        (ws.get_rain_intensity() - 0.0).abs() < 0.01,
        "Should have no rain initially"
    );
    assert!(
        (ws.get_snow_intensity() - 0.0).abs() < 0.01,
        "Should have no snow initially"
    );
    assert!(
        (ws.get_fog_density() - 0.0).abs() < 0.01,
        "Should have no fog initially"
    );
    assert!(ws.get_wind_strength() > 0.0, "Should have some base wind");
}

// ─── WeatherSystem: set_weather instant ─────────────────────────────────────

#[test]
fn set_weather_instant_changes_immediately() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Rain, 0.0); // instant
    assert!(matches!(ws.current_weather(), WeatherType::Rain));
    assert!(
        ws.get_rain_intensity() > 0.5,
        "Instant rain should have high intensity"
    );
}

#[test]
fn set_weather_gradual_sets_target() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Snow, 10.0); // 10 second transition
    assert!(
        matches!(ws.current_weather(), WeatherType::Clear),
        "Current should still be Clear"
    );
    assert!(
        matches!(ws.target_weather(), WeatherType::Snow),
        "Target should be Snow"
    );
}

#[test]
fn set_weather_same_type_no_change() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Clear, 5.0); // Same as current
    assert!(matches!(ws.current_weather(), WeatherType::Clear));
    assert!(matches!(ws.target_weather(), WeatherType::Clear));
}

// ─── WeatherSystem: transition via update ───────────────────────────────────

#[test]
fn update_completes_transition() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Rain, 1.0); // 1 second transition

    // Simulate enough time to complete the transition
    // update takes delta_time, transition_progress += delta_time / transition_duration
    ws.update(0.5); // progress = 0.5
    assert!(
        matches!(ws.current_weather(), WeatherType::Clear),
        "Should still be transitioning"
    );

    ws.update(0.6); // progress ≥ 1.0 → transition complete
    assert!(
        matches!(ws.current_weather(), WeatherType::Rain),
        "Should have completed transition to Rain"
    );
}

// ─── WeatherSystem: is_raining / is_snowing / is_foggy ─────────────────────

#[test]
fn is_raining_when_rain_active() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Rain, 0.0);
    assert!(ws.is_raining(), "Should be raining after instant rain");
}

#[test]
fn is_raining_in_storm() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Storm, 0.0);
    assert!(ws.is_raining(), "Storm should count as raining");
}

#[test]
fn not_raining_when_clear() {
    let ws = WeatherSystem::new();
    assert!(!ws.is_raining(), "Clear weather should not be raining");
}

#[test]
fn is_snowing_when_snow_active() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Snow, 0.0);
    assert!(ws.is_snowing(), "Should be snowing after instant snow");
}

#[test]
fn is_foggy_when_fog_active() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Fog, 0.0);
    assert!(ws.is_foggy(), "Should be foggy after instant fog");
}

#[test]
fn not_foggy_when_clear() {
    let ws = WeatherSystem::new();
    assert!(!ws.is_foggy(), "Clear weather should not be foggy");
}

// ─── WeatherSystem: terrain color modifier ──────────────────────────────────

#[test]
fn clear_weather_no_terrain_modifier() {
    let ws = WeatherSystem::new();
    let mod_color = ws.get_terrain_color_modifier();
    assert!(
        (mod_color.x - 1.0).abs() < 0.01,
        "Clear should have modifier 1.0"
    );
    assert!((mod_color.y - 1.0).abs() < 0.01);
    assert!((mod_color.z - 1.0).abs() < 0.01);
}

#[test]
fn rain_darkens_terrain() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Rain, 0.0);
    let mod_color = ws.get_terrain_color_modifier();
    // Rain: 1.0 - wetness * factor, so all < 1.0
    assert!(
        mod_color.x < 1.0,
        "Rain should darken terrain R (got {})",
        mod_color.x
    );
    assert!(
        mod_color.y < 1.0,
        "Rain should darken terrain G (got {})",
        mod_color.y
    );
}

#[test]
fn snow_brightens_terrain() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Snow, 0.0);
    let mod_color = ws.get_terrain_color_modifier();
    // Snow: 1.0 + snow_cover * factor, so all > 1.0
    assert!(
        mod_color.x > 1.0,
        "Snow should brighten terrain R (got {})",
        mod_color.x
    );
    assert!(
        mod_color.z > mod_color.x,
        "Snow should have extra blue (z > x)"
    );
}

#[test]
fn terrain_modifier_components_reasonable() {
    let weather_types = [
        WeatherType::Clear,
        WeatherType::Cloudy,
        WeatherType::Rain,
        WeatherType::Storm,
        WeatherType::Snow,
        WeatherType::Fog,
        WeatherType::Sandstorm,
    ];
    for &wt in &weather_types {
        let mut ws = WeatherSystem::new();
        ws.set_weather(wt, 0.0);
        let c = ws.get_terrain_color_modifier();
        // All components should be in reasonable range [0.1, 2.0]
        assert!(
            c.x > 0.1 && c.x < 2.0,
            "Terrain mod R out of range for {:?}: {}",
            wt,
            c.x
        );
        assert!(
            c.y > 0.1 && c.y < 2.0,
            "Terrain mod G out of range for {:?}: {}",
            wt,
            c.y
        );
        assert!(
            c.z > 0.1 && c.z < 2.0,
            "Terrain mod B out of range for {:?}: {}",
            wt,
            c.z
        );
    }
}

// ─── WeatherSystem: light attenuation ───────────────────────────────────────

#[test]
fn clear_full_light() {
    let ws = WeatherSystem::new();
    assert!(
        (ws.get_light_attenuation() - 1.0).abs() < 0.01,
        "Clear should have full light"
    );
}

#[test]
fn storm_heavy_attenuation() {
    let mut ws = WeatherSystem::new();
    ws.set_weather(WeatherType::Storm, 0.0);
    assert!(
        ws.get_light_attenuation() < 0.5,
        "Storm should heavily attenuate light"
    );
}

#[test]
fn light_attenuation_all_types_in_range() {
    let weather_types = [
        WeatherType::Clear,
        WeatherType::Cloudy,
        WeatherType::Rain,
        WeatherType::Storm,
        WeatherType::Snow,
        WeatherType::Fog,
        WeatherType::Sandstorm,
    ];
    for &wt in &weather_types {
        let mut ws = WeatherSystem::new();
        ws.set_weather(wt, 0.0);
        let att = ws.get_light_attenuation();
        assert!(
            att > 0.0 && att <= 1.0,
            "{:?} attenuation {} out of [0,1] range",
            wt,
            att
        );
    }
}

#[test]
fn light_attenuation_ordered_by_severity() {
    // Clear > Cloudy > Snow > Rain > Fog > Storm > Sandstorm
    let cases = [
        (WeatherType::Clear, 1.0),
        (WeatherType::Cloudy, 0.7),
        (WeatherType::Snow, 0.6),
        (WeatherType::Rain, 0.5),
        (WeatherType::Fog, 0.4),
        (WeatherType::Storm, 0.3),
        (WeatherType::Sandstorm, 0.2),
    ];
    for &(wt, expected) in &cases {
        let mut ws = WeatherSystem::new();
        ws.set_weather(wt, 0.0);
        let att = ws.get_light_attenuation();
        assert!(
            (att - expected).abs() < 0.05,
            "{:?} expected {expected}, got {att}",
            wt
        );
    }
}

// ─── WeatherSystem: biome-appropriate weather ───────────────────────────────

#[test]
fn desert_allows_sandstorm() {
    use astraweave_terrain::BiomeType;
    let weathers = WeatherSystem::get_biome_appropriate_weather(BiomeType::Desert);
    assert!(
        weathers.iter().any(|w| matches!(w, WeatherType::Sandstorm)),
        "Desert should allow sandstorm"
    );
    assert!(
        !weathers.iter().any(|w| matches!(w, WeatherType::Snow)),
        "Desert should not allow snow"
    );
}

#[test]
fn tundra_allows_snow() {
    use astraweave_terrain::BiomeType;
    let weathers = WeatherSystem::get_biome_appropriate_weather(BiomeType::Tundra);
    assert!(
        weathers.iter().any(|w| matches!(w, WeatherType::Snow)),
        "Tundra should allow snow"
    );
}

#[test]
fn forest_allows_rain_and_fog() {
    use astraweave_terrain::BiomeType;
    let weathers = WeatherSystem::get_biome_appropriate_weather(BiomeType::Forest);
    assert!(
        weathers.iter().any(|w| matches!(w, WeatherType::Rain)),
        "Forest should allow rain"
    );
    assert!(
        weathers.iter().any(|w| matches!(w, WeatherType::Fog)),
        "Forest should allow fog"
    );
}

#[test]
fn swamp_allows_fog() {
    use astraweave_terrain::BiomeType;
    let weathers = WeatherSystem::get_biome_appropriate_weather(BiomeType::Swamp);
    assert!(
        weathers.iter().any(|w| matches!(w, WeatherType::Fog)),
        "Swamp should allow fog"
    );
}

#[test]
fn all_biomes_have_weather_options() {
    use astraweave_terrain::BiomeType;
    for &biome in BiomeType::all() {
        let weathers = WeatherSystem::get_biome_appropriate_weather(biome);
        assert!(
            !weathers.is_empty(),
            "Every biome should have at least one weather type"
        );
        assert!(
            weathers.len() >= 2,
            "{:?} should have at least 2 weather options, got {}",
            biome,
            weathers.len()
        );
    }
}

// ─── WeatherSystem: wind ────────────────────────────────────────────────────

#[test]
fn wind_direction_normalized() {
    let ws = WeatherSystem::new();
    let dir = ws.get_wind_direction();
    let len = dir.length();
    assert!(
        (len - 1.0).abs() < 0.01,
        "Wind direction should be normalized, got length={len}"
    );
}

#[test]
fn wind_strength_positive() {
    let ws = WeatherSystem::new();
    assert!(
        ws.get_wind_strength() > 0.0,
        "Base wind strength should be positive"
    );
}

// ─── WeatherParticles ───────────────────────────────────────────────────────

#[test]
fn weather_particles_creation() {
    let particles = WeatherParticles::new(1000, 50.0);
    assert!(
        particles.rain_particles().is_empty(),
        "Should start with no rain particles"
    );
    assert!(
        particles.snow_particles().is_empty(),
        "Should start with no snow particles"
    );
}
