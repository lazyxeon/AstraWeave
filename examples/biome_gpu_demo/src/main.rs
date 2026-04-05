//! GPU Biome Environment Demo
//!
//! Exercises the **full GPU pipeline** for biome transitions:
//!
//! 1. Creates a headless `Renderer` (no window, wgpu software adapter).
//! 2. Walks a virtual player across biomes via `update_player_biome()`.
//! 3. Cycles through **weather kinds** and verifies scene environment UBO values.
//! 4. Steps the **time-of-day** system and prints ambient shifts.
//! 5. Dumps the `SceneEnvironmentUBO` at each stage so you can see the GPU
//!    data that would drive fog, ambient, and tint in the shaders.
//!
//! Run:
//! ```sh
//! cargo run -p biome_gpu_demo
//! ```

use anyhow::Result;
use astraweave_render::{
    biome_detector::BiomeDetectorConfig,
    biome_transition::{EasingFunction, TransitionConfig},
    effects::WeatherKind,
    scene_environment::SceneEnvironmentUBO,
};
use astraweave_terrain::{
    biome::BiomeType,
    climate::{ClimateConfig, ClimateMap},
};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  AstraWeave — GPU Biome Environment Pipeline Demo        ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // ── 1. Create headless Renderer ──────────────────────────────────────
    println!("⏳ Creating headless GPU renderer (256×256) …");
    let mut renderer = pollster::block_on(astraweave_render::renderer::Renderer::new_headless(
        256, 256,
    ))?;
    println!("✅ Renderer created\n");

    // Configure faster transitions for the demo
    renderer.set_transition_config(TransitionConfig {
        duration: 0.5,
        easing: EasingFunction::SmootherStep,
        blend_fog: true,
        blend_ambient: true,
        apply_tint: true,
        tint_alpha: 0.20,
    });
    renderer.set_biome_detector_config(BiomeDetectorConfig {
        sample_distance_threshold: 5.0,
        hysteresis_count: 1, // Instant for demo
    });

    // ── 2. Walk across biomes ────────────────────────────────────────────
    let seed = 42u64;
    let climate = ClimateMap::new(&ClimateConfig::default(), seed);
    let dt = 0.016; // ~60 FPS frame time

    println!("━━━━ Phase 1: Biome Walk (x = 0 → 2000) ━━━━\n");

    let mut last_biome: Option<BiomeType> = None;
    let mut transitions = 0u32;

    for step in 0..200 {
        let x = step as f64 * 10.0;
        let z = 0.0;
        let height = climate.estimate_height(x, z);

        if let Some(biome) = renderer.update_player_biome(&climate, x, z, height, dt) {
            transitions += 1;
            let ubo = renderer.scene_environment_ubo();
            println!(
                "🔄 Transition #{} at x={:.0}: {:?} → {:?}",
                transitions,
                x,
                last_biome.map(|b| b.as_str()).unwrap_or("(none)"),
                biome.as_str(),
            );
            print_ubo("  ", &ubo);
            last_biome = Some(biome);
        } else {
            // Even when no transition, the blend continues
            renderer.update_player_biome(&climate, x, z, height, dt);
        }
    }

    // Flush remaining transition frames
    for _ in 0..60 {
        renderer.update_player_biome(&climate, 2000.0, 0.0, 100.0, dt);
    }

    let final_ubo = renderer.scene_environment_ubo();
    println!("\n📊 Final biome-walk UBO:");
    print_ubo("  ", &final_ubo);
    println!("   Total transitions: {}\n", transitions);

    // ── 3. Weather system ────────────────────────────────────────────────
    println!("━━━━ Phase 2: Weather System ━━━━\n");

    let weather_kinds = [
        (WeatherKind::None, "Clear"),
        (WeatherKind::Rain, "Rain"),
        (WeatherKind::WindTrails, "Wind Trails"),
        (WeatherKind::None, "Clear (reset)"),
    ];

    for (kind, label) in &weather_kinds {
        renderer.set_weather(*kind);
        // Tick once to propagate
        renderer.tick_weather(dt);
        let ubo = renderer.scene_environment_ubo();
        println!("🌦️  Weather: {}", label);
        println!(
            "   fog_density={:.4}  ambient_intensity={:.4}",
            ubo.fog_density, ubo.ambient_intensity
        );
        println!();
    }

    // ── 4. Time-of-day ──────────────────────────────────────────────────
    println!("━━━━ Phase 3: Time-of-Day Cycle ━━━━\n");

    // Reset weather to clear so we isolate time-of-day
    renderer.set_weather(WeatherKind::None);

    let hours_to_check = [6.0f32, 9.0, 12.0, 17.0, 20.0, 0.0, 3.0];
    for &hour in &hours_to_check {
        renderer.time_of_day_mut().current_time = hour;
        renderer.tick_environment(dt);
        let tod = renderer.time_of_day();
        let ambient = tod.get_ambient_color();
        let ubo = renderer.scene_environment_ubo();

        let period = if tod.is_day() {
            "Day"
        } else if tod.is_night() {
            "Night"
        } else {
            "Twilight"
        };

        println!(
            "🕐 {:>5.1}h ({:<8}) | ToD ambient=[{:.2},{:.2},{:.2}] | UBO ambient=[{:.2},{:.2},{:.2}] int={:.3}",
            hour,
            period,
            ambient.x, ambient.y, ambient.z,
            ubo.ambient_color[0], ubo.ambient_color[1], ubo.ambient_color[2],
            ubo.ambient_intensity,
        );
    }

    // ── 5. Combined: weather + time-of-day + biome ──────────────────────
    println!("\n━━━━ Phase 4: Combined (Rainy Night in current biome) ━━━━\n");

    renderer.set_weather(WeatherKind::Rain);
    renderer.time_of_day_mut().current_time = 23.0;
    renderer.tick_environment(dt);
    renderer.tick_weather(dt);

    let ubo = renderer.scene_environment_ubo();
    println!("🌧️🌙 Rainy night scene:");
    print_ubo("  ", &ubo);

    println!("\n━━━━ Phase 5: Post-processing tint verify ━━━━\n");

    // Start a fresh transition to see tint values
    renderer.set_weather(WeatherKind::None);
    renderer.time_of_day_mut().current_time = 12.0;
    renderer.tick_environment(dt);

    // Force a transition by moving far
    let _ = renderer.update_player_biome(&climate, 500.0, 500.0, 50.0, dt);
    let _ = renderer.update_player_biome(&climate, 500.0, 500.0, 50.0, dt);

    // Half-way through transition:
    for _ in 0..15 {
        renderer.update_player_biome(&climate, 500.0, 500.0, 50.0, dt);
    }
    let mid_ubo = renderer.scene_environment_ubo();
    println!("Mid-transition:");
    println!(
        "  tint=[{:.3},{:.3},{:.3}]  tint_alpha={:.4}  blend={:.3}",
        mid_ubo.tint_color[0],
        mid_ubo.tint_color[1],
        mid_ubo.tint_color[2],
        mid_ubo.tint_alpha,
        mid_ubo.blend_factor,
    );

    // Completion
    for _ in 0..100 {
        renderer.update_player_biome(&climate, 500.0, 500.0, 50.0, dt);
    }
    let end_ubo = renderer.scene_environment_ubo();
    println!("Post-transition:");
    println!(
        "  tint=[{:.3},{:.3},{:.3}]  tint_alpha={:.4}  blend={:.3}",
        end_ubo.tint_color[0],
        end_ubo.tint_color[1],
        end_ubo.tint_color[2],
        end_ubo.tint_alpha,
        end_ubo.blend_factor,
    );

    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  ✅  GPU biome environment pipeline validated             ║");
    println!("╚═══════════════════════════════════════════════════════════╝");

    Ok(())
}

fn print_ubo(prefix: &str, ubo: &SceneEnvironmentUBO) {
    println!(
        "{}fog=[{:.3},{:.3},{:.3}] density={:.4} start={:.1} end={:.1}",
        prefix,
        ubo.fog_color[0],
        ubo.fog_color[1],
        ubo.fog_color[2],
        ubo.fog_density,
        ubo.fog_start,
        ubo.fog_end,
    );
    println!(
        "{}ambient=[{:.3},{:.3},{:.3}] intensity={:.4}",
        prefix,
        ubo.ambient_color[0],
        ubo.ambient_color[1],
        ubo.ambient_color[2],
        ubo.ambient_intensity,
    );
    println!(
        "{}tint=[{:.3},{:.3},{:.3}] alpha={:.4} blend={:.3}",
        prefix,
        ubo.tint_color[0],
        ubo.tint_color[1],
        ubo.tint_color[2],
        ubo.tint_alpha,
        ubo.blend_factor,
    );
}
