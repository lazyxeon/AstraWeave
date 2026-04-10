//! Wave 2 mutation remediation tests — World + Console panels
//! Covers: WeatherType, TimePreset, TimeSettings, LightingSettings, WorldBounds,
//!         EnvironmentPreset, WorldEventType, WeatherSettings,
//!         LogLevel, LogEntry, ConsolePanel, SceneStatsPanel

use aw_editor_lib::panels::console_panel::{ConsolePanel, LogEntry, LogLevel};
use aw_editor_lib::panels::scene_stats_panel::{SceneStats, SceneStatsPanel};
use aw_editor_lib::panels::world_panel::{
    EnvironmentPreset, LightingSettings, TimePreset, TimeSettings, WeatherSettings, WeatherType,
    WorldBounds, WorldEvent, WorldEventType,
};

// ═══════════════════════════════════════════════════════════════════════════════════
// WEATHER TYPE
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn weather_type_all_count() {
    assert_eq!(WeatherType::all().len(), 11);
}

#[test]
fn weather_type_names() {
    assert_eq!(WeatherType::Clear.name(), "Clear");
    assert_eq!(WeatherType::Cloudy.name(), "Cloudy");
    assert_eq!(WeatherType::Overcast.name(), "Overcast");
    assert_eq!(WeatherType::LightRain.name(), "Light Rain");
    assert_eq!(WeatherType::HeavyRain.name(), "Heavy Rain");
    assert_eq!(WeatherType::Thunderstorm.name(), "Thunderstorm");
    assert_eq!(WeatherType::Snow.name(), "Snow");
    assert_eq!(WeatherType::Blizzard.name(), "Blizzard");
    assert_eq!(WeatherType::Fog.name(), "Fog");
    assert_eq!(WeatherType::Sandstorm.name(), "Sandstorm");
    assert_eq!(WeatherType::Hail.name(), "Hail");
}

#[test]
fn weather_type_icons_non_empty() {
    for w in WeatherType::all() {
        assert!(!w.icon().is_empty(), "{:?} icon empty", w);
    }
}

#[test]
fn weather_type_description_non_empty() {
    for w in WeatherType::all() {
        assert!(!w.description().is_empty(), "{:?} description empty", w);
    }
}

#[test]
fn weather_type_ambient_modifier_range() {
    for w in WeatherType::all() {
        let m = w.ambient_modifier();
        assert!(
            m > 0.0 && m <= 1.0,
            "{:?} ambient_modifier {} out of range",
            w,
            m
        );
    }
}

#[test]
fn weather_type_ambient_modifier_clear_is_max() {
    assert!((WeatherType::Clear.ambient_modifier() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn weather_type_ambient_modifier_thunderstorm_low() {
    assert!(WeatherType::Thunderstorm.ambient_modifier() < 0.3);
}

#[test]
fn weather_type_display() {
    for w in WeatherType::all() {
        let s = format!("{}", w);
        assert!(s.contains(w.name()));
    }
}

#[test]
fn weather_type_default_is_clear() {
    assert_eq!(WeatherType::default(), WeatherType::Clear);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// TIME PRESET
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn time_preset_all_count() {
    assert_eq!(TimePreset::all().len(), 9);
}

#[test]
fn time_preset_names() {
    assert_eq!(TimePreset::Midnight.name(), "Midnight");
    assert_eq!(TimePreset::Dawn.name(), "Dawn");
    assert_eq!(TimePreset::Sunrise.name(), "Sunrise");
    assert_eq!(TimePreset::Morning.name(), "Morning");
    assert_eq!(TimePreset::Noon.name(), "Noon");
    assert_eq!(TimePreset::Afternoon.name(), "Afternoon");
    assert_eq!(TimePreset::Sunset.name(), "Sunset");
    assert_eq!(TimePreset::Dusk.name(), "Dusk");
    assert_eq!(TimePreset::Night.name(), "Night");
}

#[test]
fn time_preset_icons_non_empty() {
    for t in TimePreset::all() {
        assert!(!t.icon().is_empty(), "{:?} icon empty", t);
    }
}

#[test]
fn time_preset_hour_values() {
    assert!((TimePreset::Midnight.hour() - 0.0).abs() < f32::EPSILON);
    assert!((TimePreset::Dawn.hour() - 5.0).abs() < f32::EPSILON);
    assert!((TimePreset::Sunrise.hour() - 6.5).abs() < f32::EPSILON);
    assert!((TimePreset::Morning.hour() - 9.0).abs() < f32::EPSILON);
    assert!((TimePreset::Noon.hour() - 12.0).abs() < f32::EPSILON);
    assert!((TimePreset::Afternoon.hour() - 15.0).abs() < f32::EPSILON);
    assert!((TimePreset::Sunset.hour() - 18.5).abs() < f32::EPSILON);
    assert!((TimePreset::Dusk.hour() - 20.0).abs() < f32::EPSILON);
    assert!((TimePreset::Night.hour() - 22.0).abs() < f32::EPSILON);
}

#[test]
fn time_preset_hours_monotonic() {
    let hours: Vec<f32> = TimePreset::all().iter().map(|t| t.hour()).collect();
    for w in hours.windows(2) {
        assert!(w[0] < w[1], "Hours not monotonic: {:?}", hours);
    }
}

#[test]
fn time_preset_display() {
    for t in TimePreset::all() {
        let s = format!("{}", t);
        assert!(s.contains(t.name()));
    }
}

#[test]
fn time_preset_default_is_noon() {
    assert_eq!(TimePreset::default(), TimePreset::Noon);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// TIME SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn time_settings_defaults() {
    let s = TimeSettings::default();
    assert!((s.current_hour - 12.0).abs() < f32::EPSILON);
    assert!((s.day_length_minutes - 24.0).abs() < f32::EPSILON);
    assert!(!s.auto_cycle);
    assert!((s.cycle_speed - 1.0).abs() < f32::EPSILON);
    assert!((s.sun_angle - 45.0).abs() < f32::EPSILON);
    assert_eq!(s.moon_phase, 0);
}

#[test]
fn time_settings_is_daytime() {
    let mut s = TimeSettings::default();
    s.current_hour = 12.0;
    assert!(s.is_daytime());
    s.current_hour = 3.0;
    assert!(!s.is_daytime());
    s.current_hour = 6.0;
    assert!(s.is_daytime());
    s.current_hour = 18.0;
    assert!(!s.is_daytime());
    s.current_hour = 17.99;
    assert!(s.is_daytime());
}

#[test]
fn time_settings_sun_intensity() {
    let mut s = TimeSettings::default();
    // Midnight
    s.current_hour = 0.0;
    assert!((s.sun_intensity() - 0.0).abs() < f32::EPSILON);
    // Noon
    s.current_hour = 12.0;
    assert!((s.sun_intensity() - 1.0).abs() < f32::EPSILON);
    // Dawn start (6.0)
    s.current_hour = 6.0;
    assert!((s.sun_intensity() - 0.0).abs() < f32::EPSILON);
    // 7.5 (halfway between 6 and 9)
    s.current_hour = 7.5;
    assert!((s.sun_intensity() - 0.5).abs() < f32::EPSILON);
    // Late afternoon (17.0) — in sunset range
    s.current_hour = 17.0;
    assert!(s.sun_intensity() < 1.0);
    assert!(s.sun_intensity() > 0.0);
    // Night (19.0) — past sunset
    s.current_hour = 19.0;
    assert!((s.sun_intensity() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn time_settings_format_time() {
    let mut s = TimeSettings::default();
    s.current_hour = 12.0;
    assert_eq!(s.format_time(), "12:00");
    s.current_hour = 0.0;
    assert_eq!(s.format_time(), "00:00");
    s.current_hour = 23.5;
    assert_eq!(s.format_time(), "23:30");
}

// ═══════════════════════════════════════════════════════════════════════════════════
// WEATHER SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn weather_settings_defaults() {
    let s = WeatherSettings::default();
    assert_eq!(s.current, WeatherType::Clear);
    assert!((s.intensity - 1.0).abs() < f32::EPSILON);
    assert!((s.wind_speed - 5.0).abs() < f32::EPSILON);
    assert!((s.wind_direction - 0.0).abs() < f32::EPSILON);
    assert!((s.precipitation_density - 0.5).abs() < f32::EPSILON);
    assert!((s.cloud_coverage - 0.3).abs() < f32::EPSILON);
    assert!((s.transition_time - 30.0).abs() < f32::EPSILON);
    assert!(!s.auto_weather);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// LIGHTING SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn lighting_settings_defaults() {
    let s = LightingSettings::default();
    assert!((s.ambient_intensity - 0.7).abs() < f32::EPSILON);
    assert!((s.sun_intensity - 1.5).abs() < f32::EPSILON);
    assert!((s.shadow_intensity - 0.7).abs() < f32::EPSILON);
    assert!(!s.fog_enabled);
    assert!((s.exposure - 1.2).abs() < f32::EPSILON);
    assert!((s.gamma - 2.2).abs() < f32::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// WORLD BOUNDS
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn world_bounds_defaults() {
    let b = WorldBounds::default();
    assert_eq!(b.min, [-1000.0, -100.0, -1000.0]);
    assert_eq!(b.max, [1000.0, 500.0, 1000.0]);
    assert!(b.enforce_bounds);
    assert!((b.kill_plane_y - (-50.0)).abs() < f32::EPSILON);
    assert!((b.ceiling_y - 450.0).abs() < f32::EPSILON);
}

#[test]
fn world_bounds_size() {
    let b = WorldBounds::default();
    let size = b.size();
    assert!((size[0] - 2000.0).abs() < f32::EPSILON);
    assert!((size[1] - 600.0).abs() < f32::EPSILON);
    assert!((size[2] - 2000.0).abs() < f32::EPSILON);
}

#[test]
fn world_bounds_center() {
    let b = WorldBounds::default();
    let center = b.center();
    assert!((center[0] - 0.0).abs() < f32::EPSILON);
    assert!((center[1] - 200.0).abs() < f32::EPSILON);
    assert!((center[2] - 0.0).abs() < f32::EPSILON);
}

#[test]
fn world_bounds_custom_size() {
    let b = WorldBounds {
        min: [0.0, 0.0, 0.0],
        max: [10.0, 20.0, 30.0],
        ..Default::default()
    };
    assert_eq!(b.size(), [10.0, 20.0, 30.0]);
    assert_eq!(b.center(), [5.0, 10.0, 15.0]);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// ENVIRONMENT PRESET
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn environment_preset_all_count() {
    assert_eq!(EnvironmentPreset::all().len(), 11);
}

#[test]
fn environment_preset_names() {
    assert_eq!(EnvironmentPreset::Sunny.name(), "Sunny Day");
    assert_eq!(EnvironmentPreset::Overcast.name(), "Overcast");
    assert_eq!(EnvironmentPreset::Rainy.name(), "Rainy");
    assert_eq!(EnvironmentPreset::Stormy.name(), "Stormy");
    assert_eq!(EnvironmentPreset::Foggy.name(), "Foggy");
    assert_eq!(EnvironmentPreset::Sunset.name(), "Sunset");
    assert_eq!(EnvironmentPreset::Night.name(), "Night");
    assert_eq!(EnvironmentPreset::DarkNight.name(), "Dark Night");
    assert_eq!(EnvironmentPreset::Arctic.name(), "Arctic");
    assert_eq!(EnvironmentPreset::Desert.name(), "Desert");
    assert_eq!(EnvironmentPreset::Custom.name(), "Custom");
}

#[test]
fn environment_preset_icons_non_empty() {
    for preset in EnvironmentPreset::all() {
        assert!(!preset.icon().is_empty(), "{:?} icon empty", preset);
    }
}

#[test]
fn environment_preset_display() {
    for preset in EnvironmentPreset::all() {
        let s = format!("{}", preset);
        assert!(s.contains(preset.name()));
    }
}

#[test]
fn environment_preset_default_is_sunny() {
    assert_eq!(EnvironmentPreset::default(), EnvironmentPreset::Sunny);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// WORLD EVENT TYPE
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn world_event_type_all_count() {
    assert_eq!(WorldEventType::all().len(), 5);
}

#[test]
fn world_event_type_display() {
    for evt in WorldEventType::all() {
        let s = format!("{}", evt);
        assert!(!s.is_empty());
    }
}

#[test]
fn world_event_new() {
    let evt = WorldEvent::new(WorldEventType::WeatherChanged, "Rain started");
    assert_eq!(evt.event_type, WorldEventType::WeatherChanged);
    assert_eq!(evt.message, "Rain started");
    assert!(evt.age_secs() < 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// LOG LEVEL
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn log_level_all_levels_count() {
    assert_eq!(LogLevel::all_levels().len(), 5);
}

#[test]
fn log_level_names() {
    assert_eq!(LogLevel::All.name(), "All");
    assert_eq!(LogLevel::Debug.name(), "Debug");
    assert_eq!(LogLevel::Info.name(), "Info");
    assert_eq!(LogLevel::Warning.name(), "Warnings");
    assert_eq!(LogLevel::Error.name(), "Errors");
}

#[test]
fn log_level_icons_non_empty() {
    for level in LogLevel::all_levels() {
        assert!(!level.icon().is_empty(), "{:?} icon empty", level);
    }
}

#[test]
fn log_level_default_is_info() {
    assert_eq!(LogLevel::default(), LogLevel::Info);
}

#[test]
fn log_level_display() {
    for level in LogLevel::all_levels() {
        let s = format!("{}", level);
        assert!(s.contains(level.name()));
    }
}

#[test]
fn log_level_matches_all_always_true() {
    assert!(LogLevel::All.matches("anything"));
    assert!(LogLevel::All.matches("⚠️ warning"));
    assert!(LogLevel::All.matches("❌ error"));
}

#[test]
fn log_level_matches_warning() {
    assert!(LogLevel::Warning.matches("warning: something bad"));
    assert!(!LogLevel::Warning.matches("normal log"));
}

#[test]
fn log_level_matches_error() {
    assert!(LogLevel::Error.matches("error: critical failure"));
    assert!(!LogLevel::Error.matches("normal log"));
}

#[test]
fn log_level_matches_entry() {
    let entry = LogEntry::new("test", LogLevel::Warning);
    assert!(LogLevel::All.matches_entry(&entry));
    assert!(LogLevel::Warning.matches_entry(&entry));
    assert!(!LogLevel::Error.matches_entry(&entry));
}

// ═══════════════════════════════════════════════════════════════════════════════════
// LOG ENTRY
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn log_entry_new() {
    let entry = LogEntry::new("Test message", LogLevel::Info);
    assert_eq!(entry.message, "Test message");
    assert_eq!(entry.level, LogLevel::Info);
    assert!(entry.category.is_none());
    assert!(entry.source_file.is_none());
    assert!(entry.source_line.is_none());
    assert!(entry.stacktrace.is_none());
}

#[test]
fn log_entry_with_category() {
    let entry = LogEntry::new("msg", LogLevel::Debug).with_category("Render");
    assert_eq!(entry.category, Some("Render".to_string()));
}

#[test]
fn log_entry_with_source() {
    let entry = LogEntry::new("msg", LogLevel::Error).with_source("main.rs", 42);
    assert_eq!(entry.source_file, Some("main.rs".to_string()));
    assert_eq!(entry.source_line, Some(42));
}

#[test]
fn log_entry_with_stacktrace() {
    let entry = LogEntry::new("msg", LogLevel::Error).with_stacktrace("at main.rs:42");
    assert_eq!(entry.stacktrace, Some("at main.rs:42".to_string()));
}

#[test]
fn log_entry_format_timestamp() {
    let entry = LogEntry::new("msg", LogLevel::Info);
    let ts = entry.format_timestamp();
    // Should be HH:MM:SS.mmm format
    assert_eq!(ts.len(), 12);
    assert_eq!(&ts[2..3], ":");
    assert_eq!(&ts[5..6], ":");
    assert_eq!(&ts[8..9], ".");
}

// ═══════════════════════════════════════════════════════════════════════════════════
// CONSOLE PANEL
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn console_panel_new() {
    let panel = ConsolePanel::new();
    // Should not panic — basic construction test
    let _ = panel;
}

#[test]
fn console_panel_push_entries() {
    let mut panel = ConsolePanel::new();
    panel.push_entry(LogEntry::new("test1", LogLevel::Info));
    panel.push_entry(LogEntry::new("test2", LogLevel::Warning));
    // Push works without panic
}

// ═══════════════════════════════════════════════════════════════════════════════════
// SCENE STATS PANEL
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn scene_stats_defaults() {
    let s = SceneStats::default();
    assert_eq!(s.entity_count, 0);
    assert_eq!(s.selected_count, 0);
    assert_eq!(s.component_count, 0);
    assert_eq!(s.mesh_count, 0);
    assert_eq!(s.total_triangles, 0);
    assert_eq!(s.total_vertices, 0);
    assert_eq!(s.material_count, 0);
    assert!(s.scene_path.is_none());
    assert!(!s.is_dirty);
    assert!(s.performance_warning.is_none());
}

#[test]
fn scene_stats_panel_new() {
    let panel = SceneStatsPanel::new();
    let _ = panel;
}

#[test]
fn scene_stats_panel_no_warning_when_ok() {
    let panel = SceneStatsPanel::new();
    assert!(panel.generate_performance_warning().is_none());
}

#[test]
fn scene_stats_panel_warning_high_triangles() {
    let mut panel = SceneStatsPanel::new();
    panel.update_stats(SceneStats {
        total_triangles: 2_000_000,
        ..Default::default()
    });
    let warning = panel.generate_performance_warning();
    assert!(warning.is_some());
    assert!(warning.unwrap().contains("triangle"));
}

#[test]
fn scene_stats_panel_warning_high_draw_calls() {
    let mut panel = SceneStatsPanel::new();
    panel.update_stats(SceneStats {
        estimated_draw_calls: 600,
        ..Default::default()
    });
    let warning = panel.generate_performance_warning();
    assert!(warning.is_some());
    assert!(warning.unwrap().contains("draw call"));
}

#[test]
fn scene_stats_panel_warning_high_texture_memory() {
    let mut panel = SceneStatsPanel::new();
    panel.update_stats(SceneStats {
        texture_memory_kb: 600 * 1024,
        ..Default::default()
    });
    let warning = panel.generate_performance_warning();
    assert!(warning.is_some());
    assert!(warning.unwrap().contains("texture memory"));
}
