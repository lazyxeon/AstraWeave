//! Biome Weather Demo
//!
//! Demonstrates the complete weather system with biome-specific parameters:
//!
//! - **5 Weather Types**: None, Rain, Snow, Sandstorm, WindTrails
//! - **8 Biomes**: Each with unique cloud, fog, and particle density settings
//! - **Scene Environment**: Shows fog/ambient multipliers per weather type
//!
//! Run:
//! ```sh
//! cargo run -p biome_weather_demo
//! ```

use anyhow::Result;
use astraweave_render::{
    biome_transition::BiomeVisuals, effects::WeatherKind, scene_environment::SceneEnvironment,
};
use astraweave_terrain::biome::BiomeType;

const BIOMES: [BiomeType; 8] = [
    BiomeType::Forest,
    BiomeType::Desert,
    BiomeType::Grassland,
    BiomeType::Mountain,
    BiomeType::Tundra,
    BiomeType::Swamp,
    BiomeType::Beach,
    BiomeType::River,
];

const WEATHER_KINDS: [WeatherKind; 5] = [
    WeatherKind::None,
    WeatherKind::Rain,
    WeatherKind::Snow,
    WeatherKind::Sandstorm,
    WeatherKind::WindTrails,
];

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  AstraWeave — Biome Weather System Demo                          ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    // ── 1. Show biome-specific weather parameters ────────────────────────
    println!("━━━━ BIOME-SPECIFIC WEATHER PARAMETERS ━━━━");
    println!();
    println!(
        "{:<12} {:>8} {:>8} {:>10} {:>12}",
        "Biome", "Clouds", "Speed", "Density", "Notes"
    );
    println!("{:-<12} {:->8} {:->8} {:->10} {:->12}", "", "", "", "", "");

    for biome in &BIOMES {
        let visuals = BiomeVisuals::for_biome(*biome);
        let notes = match biome {
            BiomeType::Forest => "canopy blocks",
            BiomeType::Desert => "sandstorms",
            BiomeType::Mountain => "high altitude",
            BiomeType::Tundra => "heavy snow",
            BiomeType::Swamp => "stagnant air",
            _ => "",
        };
        println!(
            "{:<12} {:>7.0}% {:>8.2} {:>10.1}× {:>12}",
            biome.as_str(),
            visuals.cloud_coverage * 100.0,
            visuals.cloud_speed,
            visuals.weather_particle_density,
            notes
        );
    }
    println!();

    // ── 2. Show weather type effects on scene environment ────────────────
    println!("━━━━ WEATHER TYPE → SCENE ENVIRONMENT ━━━━");
    println!();
    println!(
        "{:<12} {:>12} {:>12} {:>20}",
        "Weather", "Fog ×", "Ambient ×", "Description"
    );
    println!("{:-<12} {:->12} {:->12} {:->20}", "", "", "", "");

    for kind in &WEATHER_KINDS {
        let mut env = SceneEnvironment::default();
        env.apply_weather(*kind);

        let desc = match kind {
            WeatherKind::None => "Clear skies",
            WeatherKind::Rain => "Light overcast",
            WeatherKind::Snow => "Reduced visibility",
            WeatherKind::Sandstorm => "Severe obscuration",
            WeatherKind::WindTrails => "Light haze",
            _ => "Unknown weather",
        };

        println!(
            "{:<12} {:>11.1}× {:>11.2}× {:>20}",
            format!("{:?}", kind),
            env.weather_fog_multiplier,
            env.weather_ambient_multiplier,
            desc
        );
    }
    println!();

    // ── 3. Show biome × weather interactions ─────────────────────────────
    println!("━━━━ BIOME × WEATHER MATRIX ━━━━");
    println!();
    println!(
        "{:<12} {:>8} {:>8} {:>8} {:>8}",
        "Biome", "Rain", "Snow", "Sand", "Wind"
    );
    println!("{:-<12} {:->8} {:->8} {:->8} {:->8}", "", "", "", "", "");

    for biome in &BIOMES {
        let visuals = BiomeVisuals::for_biome(*biome);
        let density = visuals.weather_particle_density;

        // Simulate expected particle counts per frame (scaled by density)
        let rain_particles = (100.0 * density * 0.5).round() as u32;
        let snow_particles = (80.0 * density * 0.3).round() as u32;
        let sand_particles = (120.0 * density * 0.6).round() as u32;
        let wind_particles = (40.0 * density * 0.2).round() as u32;

        println!(
            "{:<12} {:>8} {:>8} {:>8} {:>8}",
            biome.as_str(),
            rain_particles,
            snow_particles,
            sand_particles,
            wind_particles
        );
    }
    println!();

    // ── 4. Demonstrate biome transition with weather parameters ──────────
    println!("━━━━ BIOME TRANSITION + WEATHER DEMO ━━━━");
    println!();

    let from = BiomeVisuals::for_biome(BiomeType::Forest);
    let to = BiomeVisuals::for_biome(BiomeType::Tundra);

    println!("Transitioning: Forest → Tundra");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let blended = BiomeVisuals::lerp(&from, &to, t);

        println!(
            "  t={:.2}: clouds={:.0}%, speed={:.3}, density={:.2}×",
            t,
            blended.cloud_coverage * 100.0,
            blended.cloud_speed,
            blended.weather_particle_density
        );
    }
    println!();

    // ── 5. Show effective fog densities (biome × weather) ────────────────
    println!("━━━━ EFFECTIVE FOG DENSITY (biome × weather) ━━━━");
    println!();
    println!(
        "{:<12} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Biome", "None", "Rain", "Snow", "Sand", "Wind"
    );
    println!(
        "{:-<12} {:->10} {:->10} {:->10} {:->10} {:->10}",
        "", "", "", "", "", ""
    );

    for biome in &BIOMES {
        let visuals = BiomeVisuals::for_biome(*biome);
        let base_fog = visuals.fog_density;

        // Weather multipliers: None=1.0, Rain=2.5, Snow=1.8, Sandstorm=4.0, Wind=1.4
        let fog_none = base_fog * 1.0;
        let fog_rain = base_fog * 2.5;
        let fog_snow = base_fog * 1.8;
        let fog_sand = base_fog * 4.0;
        let fog_wind = base_fog * 1.4;

        println!(
            "{:<12} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>10.4}",
            biome.as_str(),
            fog_none,
            fog_rain,
            fog_snow,
            fog_sand,
            fog_wind
        );
    }
    println!();

    // ── 6. Summary ───────────────────────────────────────────────────────
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Demo Complete!                                                  ║");
    println!("║                                                                  ║");
    println!("║  Features Demonstrated:                                          ║");
    println!("║  ✅ 5 weather types (None, Rain, Snow, Sandstorm, WindTrails)    ║");
    println!("║  ✅ 8 biomes with unique cloud/density parameters                ║");
    println!("║  ✅ Weather → scene environment (fog/ambient multipliers)        ║");
    println!("║  ✅ Particle density scaling per biome                           ║");
    println!("║  ✅ Effective fog = biome × weather multipliers                  ║");
    println!("║  ✅ Smooth biome transitions with weather parameter lerping      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    Ok(())
}
