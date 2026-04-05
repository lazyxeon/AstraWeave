//! Biome Transition Demo
//!
//! Demonstrates the complete biome detection → material + HDRI transition
//! pipeline running headless (no GPU required).
//!
//! Simulates a player walking across a procedural world and logs each biome
//! transition as it happens, showing:
//!
//! 1. **BiomeDetector** — watches player position via ClimateMap noise
//! 2. **BiomeMaterialSystem** — resolves HDRI + material dir per biome
//! 3. **AssetIndex** — validates available assets at startup
//! 4. **HdriCatalog** — maps biome × time-of-day → HDRI file
//! 5. **TransitionEffect** — smooth crossfade with fog/ambient interpolation
//! 6. **BiomeVisuals** — sky colours, water colours, fog, ambient per biome
//! 7. **BiomeAmbientMap** — biome → ambient audio track mapping
//!
//! Run:
//! ```sh
//! cargo run -p biome_transition_demo
//! ```

use anyhow::Result;
use astraweave_render::{
    asset_index::AssetIndex,
    biome_audio::BiomeAmbientMap,
    biome_detector::{BiomeDetector, BiomeDetectorConfig},
    biome_material::{BiomeMaterialConfig, BiomeMaterialSystem},
    biome_transition::{BiomeVisuals, EasingFunction, TransitionConfig, TransitionEffect},
    hdri_catalog::DayPeriod,
};
use astraweave_terrain::{
    biome::BiomeType,
    climate::{ClimateConfig, ClimateMap},
};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  AstraWeave — Biome Transition Pipeline Demo            ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // ── 1. Load & validate asset index ───────────────────────────────────
    let index_path = std::path::Path::new("assets/asset_index.toml");
    if index_path.exists() {
        let index = AssetIndex::load(index_path)?;
        println!("📦 Asset Index v{} loaded:", index.index.version);
        println!(
            "   {} material sets, {} textures, {} HDRIs, {} models, {} audio packs",
            index.material_sets.len(),
            index.textures.len(),
            index.hdris.len(),
            index.models.len(),
            index.audio_packs.len(),
        );

        let missing = index.validate_paths("assets");
        if missing.is_empty() {
            println!("   ✅ All referenced assets present on disk");
        } else {
            println!("   ⚠️  {} missing assets:", missing.len());
            for m in missing.iter().take(5) {
                println!("      - {}", m);
            }
        }
        println!();
    } else {
        println!("⚠️  No asset_index.toml found — skipping index validation\n");
    }

    // ── 2. Initialize climate + biome detector ───────────────────────────
    let seed = 42u64;
    let climate = ClimateMap::new(&ClimateConfig::default(), seed);

    let detector_cfg = BiomeDetectorConfig {
        sample_distance_threshold: 5.0,
        hysteresis_count: 2, // Quick transitions for demo
    };
    let mut detector = BiomeDetector::new(detector_cfg);

    // ── 3. Initialize biome material system ──────────────────────────────
    let mut bms = BiomeMaterialSystem::new(BiomeMaterialConfig::default());

    // ── 3b. Initialize transition effect ─────────────────────────────────
    let transition_cfg = TransitionConfig {
        duration: 1.5, // 1.5 seconds per transition
        easing: EasingFunction::SmootherStep,
        blend_fog: true,
        blend_ambient: true,
        apply_tint: true,
        tint_alpha: 0.15,
    };
    let mut transition_effect = TransitionEffect::new(transition_cfg);

    println!("🌍 Climate system seeded: {}", seed);
    println!("🎯 Biome detector: threshold=5.0 units, hysteresis=2 samples");
    println!(
        "🎨 Transition effect: {:?}, duration=1.5s",
        EasingFunction::SmootherStep
    );
    println!();

    // ── 4. Simulate walking across the world ─────────────────────────────
    println!("━━━━ Simulating player walking 0→4000 on X axis ━━━━");
    println!();

    let step = 10.0; // Move 10 world units per tick
    let total_steps = 400;
    let dt = 0.05; // 50ms per simulation step (simulating 20 FPS)
    let mut x: f64 = 0.0;
    let z: f64 = 0.0;

    for i in 0..total_steps {
        let height = climate.estimate_height(x, z);
        let (temp, moisture) = climate.sample_climate(x, z, height);

        // Update any active transition effect
        if transition_effect.is_active() {
            transition_effect.update(dt);

            // Log interpolation progress every ~10 steps while transitioning
            if i % 10 == 0 {
                let visuals = transition_effect.current_visuals();
                println!(
                    "   ⏳ Blend: {:.0}% | fog: [{:.2},{:.2},{:.2}] density={:.3} | ambient: [{:.2},{:.2},{:.2}]",
                    transition_effect.blend_factor() * 100.0,
                    visuals.fog_color[0], visuals.fog_color[1], visuals.fog_color[2],
                    visuals.fog_density,
                    visuals.ambient_color[0], visuals.ambient_color[1], visuals.ambient_color[2],
                );
                if transition_effect.tint_alpha() > 0.01 {
                    let tint = transition_effect.tint_color();
                    println!(
                        "   🎨 Tint: [{:.2},{:.2},{:.2}] alpha={:.2}",
                        tint[0],
                        tint[1],
                        tint[2],
                        transition_effect.tint_alpha(),
                    );
                }
            }
        }

        if let Some(transition) = detector.update(&climate, x, z, height) {
            println!(
                "🔄 Step {:>3} | x={:>7.1} | T={:.2} M={:.2} H={:.1}",
                i, x, temp, moisture, height
            );
            println!(
                "   Transition: {:?} → {:?}",
                transition.old_biome.map(|b| b.as_str()).unwrap_or("(none)"),
                transition.new_biome.as_str(),
            );

            // Resolve HDRI for new biome
            match bms.resolve_sky_mode(transition.new_biome) {
                Ok(sky_mode) => {
                    println!("   SkyMode: {:?}", sky_mode);
                }
                Err(e) => {
                    println!("   SkyMode: ⚠️  {}", e);
                }
            }

            // Show material directory
            let mat_dir = bms.material_dir_for(transition.new_biome);
            let has_mats = mat_dir.join("materials.toml").exists();
            println!(
                "   Materials: {} {}",
                mat_dir.display(),
                if has_mats { "✅" } else { "❌" }
            );

            // Start visual transition effect
            transition_effect.start(transition.old_biome, transition.new_biome);
            let from_visuals = transition
                .old_biome
                .map(BiomeVisuals::for_biome)
                .unwrap_or_else(|| BiomeVisuals::for_biome(BiomeType::Grassland));
            let to_visuals = BiomeVisuals::for_biome(transition.new_biome);
            println!(
                "   🎬 Starting transition: fog {:.3}→{:.3}, ambient {:.2}→{:.2}",
                from_visuals.fog_density,
                to_visuals.fog_density,
                from_visuals.ambient_intensity,
                to_visuals.ambient_intensity,
            );
            println!(
                "   🌅 Sky day: [{:.2},{:.2},{:.2}] → [{:.2},{:.2},{:.2}]",
                from_visuals.sky_day_top[0],
                from_visuals.sky_day_top[1],
                from_visuals.sky_day_top[2],
                to_visuals.sky_day_top[0],
                to_visuals.sky_day_top[1],
                to_visuals.sky_day_top[2],
            );
            println!(
                "   🌊 Water deep: [{:.2},{:.2},{:.2}] → [{:.2},{:.2},{:.2}]",
                from_visuals.water_deep[0],
                from_visuals.water_deep[1],
                from_visuals.water_deep[2],
                to_visuals.water_deep[0],
                to_visuals.water_deep[1],
                to_visuals.water_deep[2],
            );

            bms.mark_loaded(transition.new_biome, None);
            println!();
        }

        x += step;
    }

    // ── 5. Transition effect showcase ─────────────────────────────────────
    println!("━━━━ TransitionEffect easing comparison ━━━━");
    println!();

    let easings = [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ];

    for easing in &easings {
        let mut test_effect = TransitionEffect::new(TransitionConfig {
            duration: 1.0,
            easing: *easing,
            ..Default::default()
        });
        test_effect.start(Some(BiomeType::Grassland), BiomeType::Desert);

        // Sample at 0%, 25%, 50%, 75%, 100%
        let samples: Vec<f32> = [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .map(|&t| {
                test_effect.update(t);
                test_effect.blend_factor()
            })
            .collect();

        println!(
            "  {:>12?}: [.00→{:.2}] [.25→{:.2}] [.50→{:.2}] [.75→{:.2}] [1.0→{:.2}]",
            easing, samples[0], samples[1], samples[2], samples[3], samples[4],
        );
    }
    println!();

    // ── 6. Time-of-day sweep ─────────────────────────────────────────────
    println!("━━━━ Time-of-day HDRI resolution sweep ━━━━");
    println!();

    let current_biome = detector.current_biome().unwrap_or(BiomeType::Grassland);
    for period in DayPeriod::all() {
        bms.set_time_of_day(*period);
        match bms.resolve_sky_mode(current_biome) {
            Ok(sky_mode) => {
                println!("  {:?} {:>8?} → {:?}", current_biome, period, sky_mode,);
            }
            Err(e) => {
                println!("  {:?} {:>8?} → ⚠️  {}", current_biome, period, e);
            }
        }
    }
    println!();

    // ── 7. All-biome coverage check ──────────────────────────────────────
    println!("━━━━ All-biome HDRI + material coverage ━━━━");
    println!();

    let mut bms_check = BiomeMaterialSystem::new(BiomeMaterialConfig::default());
    for bt in BiomeType::all() {
        let mat_dir = bms_check.material_dir_for(*bt);
        let has_mats = mat_dir.join("materials.toml").exists();
        let has_hdri = bms_check.resolve_hdri_path(*bt).ok().flatten().is_some();

        println!(
            "  {:>10} | materials: {} | HDRI: {}",
            bt.as_str(),
            if has_mats { "✅" } else { "❌" },
            if has_hdri { "✅" } else { "❌" },
        );
    }
    println!();

    // ── 8. Biome Ambient Audio Mapping ─────────────────────────────────
    println!("━━━━ Biome Ambient Audio Mapping ━━━━");
    println!();

    let ambient_map = BiomeAmbientMap::default();
    println!(
        "  Default crossfade duration: {:.1}s",
        ambient_map.crossfade_sec()
    );
    println!();

    for bt in BiomeType::all() {
        if let Some(path) = ambient_map.get(*bt) {
            let exists = std::path::Path::new(path).exists();
            println!(
                "  {:>10} → {} {}",
                bt.as_str(),
                path,
                if exists { "✅" } else { "(placeholder)" }
            );
        } else {
            println!("  {:>10} → (none)", bt.as_str());
        }
    }
    println!();

    // ── 9. Summary ───────────────────────────────────────────────────────
    println!("━━━━ Summary ━━━━");
    println!("  Transitions detected: {}", detector.transition_count());
    println!(
        "  Final biome: {:?}",
        detector.current_biome().unwrap_or(BiomeType::Grassland)
    );

    // HDRI coverage gaps
    let gaps = bms_check.validate_hdri_coverage()?;
    if gaps.is_empty() {
        println!("  HDRI coverage: ✅ all biome×time combinations covered");
    } else {
        println!("  HDRI gaps: {} missing combinations", gaps.len());
        for (biome, period) in &gaps {
            println!("    - {} × {:?}", biome, period);
        }
    }

    // Material dir gaps
    let mat_gaps = bms_check.validate_material_dirs();
    if mat_gaps.is_empty() {
        println!("  Material dirs: ✅ all biomes have materials.toml");
    } else {
        println!("  Material gaps: {} biomes missing", mat_gaps.len());
        for bt in &mat_gaps {
            println!("    - {:?}", bt);
        }
    }

    println!();
    println!("✅ Demo complete — all pipeline components validated.");
    Ok(())
}
