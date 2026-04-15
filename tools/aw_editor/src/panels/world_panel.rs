// tools/aw_editor/src/panels/world_panel.rs - Comprehensive world/environment panel

use super::Panel;
use crate::level_doc::LevelDoc;
use crate::terrain_integration::{biome_display_name, cached_biome_options, TerrainState};
use egui::{Color32, RichText, Ui};
use std::collections::VecDeque;

// ═══════════════════════════════════════════════════════════════════════════════════
// WEATHER SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════════

/// Weather type with associated properties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[non_exhaustive]
pub enum WeatherType {
    #[default]
    Clear,
    Cloudy,
    Overcast,
    LightRain,
    HeavyRain,
    Thunderstorm,
    Snow,
    Blizzard,
    Fog,
    Sandstorm,
    Hail,
}

impl std::fmt::Display for WeatherType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl WeatherType {
    pub fn name(&self) -> &'static str {
        match self {
            WeatherType::Clear => "Clear",
            WeatherType::Cloudy => "Cloudy",
            WeatherType::Overcast => "Overcast",
            WeatherType::LightRain => "Light Rain",
            WeatherType::HeavyRain => "Heavy Rain",
            WeatherType::Thunderstorm => "Thunderstorm",
            WeatherType::Snow => "Snow",
            WeatherType::Blizzard => "Blizzard",
            WeatherType::Fog => "Fog",
            WeatherType::Sandstorm => "Sandstorm",
            WeatherType::Hail => "Hail",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            WeatherType::Clear => "[Sun]",
            WeatherType::Cloudy => "⛅",
            WeatherType::Overcast => "[Cld]",
            WeatherType::LightRain => "[Rain]",
            WeatherType::HeavyRain => "[Rain]",
            WeatherType::Thunderstorm => "⛈️",
            WeatherType::Snow => "🌨️",
            WeatherType::Blizzard => "❄️",
            WeatherType::Fog => "[Fog]",
            WeatherType::Sandstorm => "🏜️",
            WeatherType::Hail => "🌨️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            WeatherType::Clear => "Sunny skies with no clouds",
            WeatherType::Cloudy => "Partly cloudy with sun breaks",
            WeatherType::Overcast => "Full cloud cover, no sun",
            WeatherType::LightRain => "Light drizzle or showers",
            WeatherType::HeavyRain => "Heavy rainfall",
            WeatherType::Thunderstorm => "Heavy rain with lightning",
            WeatherType::Snow => "Light to moderate snowfall",
            WeatherType::Blizzard => "Heavy snow with strong winds",
            WeatherType::Fog => "Low visibility fog",
            WeatherType::Sandstorm => "Desert sandstorm",
            WeatherType::Hail => "Hail with potential damage",
        }
    }

    pub fn ambient_modifier(&self) -> f32 {
        match self {
            WeatherType::Clear => 1.0,
            WeatherType::Cloudy => 0.85,
            WeatherType::Overcast => 0.6,
            WeatherType::LightRain => 0.5,
            WeatherType::HeavyRain => 0.35,
            WeatherType::Thunderstorm => 0.25,
            WeatherType::Snow => 0.65,
            WeatherType::Blizzard => 0.4,
            WeatherType::Fog => 0.5,
            WeatherType::Sandstorm => 0.45,
            WeatherType::Hail => 0.35,
        }
    }

    pub fn all() -> &'static [WeatherType] {
        &[
            WeatherType::Clear,
            WeatherType::Cloudy,
            WeatherType::Overcast,
            WeatherType::LightRain,
            WeatherType::HeavyRain,
            WeatherType::Thunderstorm,
            WeatherType::Snow,
            WeatherType::Blizzard,
            WeatherType::Fog,
            WeatherType::Sandstorm,
            WeatherType::Hail,
        ]
    }
}

/// Weather settings
#[derive(Debug, Clone)]
pub struct WeatherSettings {
    pub current: WeatherType,
    pub intensity: f32,
    pub wind_speed: f32,
    pub wind_direction: f32,
    pub precipitation_density: f32,
    pub cloud_coverage: f32,
    pub cloud_speed: f32,
    pub lightning_frequency: f32,
    pub transition_time: f32,
    pub auto_weather: bool,
}

impl Default for WeatherSettings {
    fn default() -> Self {
        Self {
            current: WeatherType::Clear,
            intensity: 1.0,
            wind_speed: 5.0,
            wind_direction: 0.0,
            precipitation_density: 0.5,
            cloud_coverage: 0.3,
            cloud_speed: 0.1,
            lightning_frequency: 0.0,
            transition_time: 30.0,
            auto_weather: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// TIME OF DAY
// ═══════════════════════════════════════════════════════════════════════════════════

/// Time of day presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[non_exhaustive]
pub enum TimePreset {
    Midnight,
    Dawn,
    Sunrise,
    Morning,
    #[default]
    Noon,
    Afternoon,
    Sunset,
    Dusk,
    Night,
}

impl std::fmt::Display for TimePreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl TimePreset {
    pub fn all() -> &'static [TimePreset] {
        &[
            TimePreset::Midnight,
            TimePreset::Dawn,
            TimePreset::Sunrise,
            TimePreset::Morning,
            TimePreset::Noon,
            TimePreset::Afternoon,
            TimePreset::Sunset,
            TimePreset::Dusk,
            TimePreset::Night,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            TimePreset::Midnight => "Midnight",
            TimePreset::Dawn => "Dawn",
            TimePreset::Sunrise => "Sunrise",
            TimePreset::Morning => "Morning",
            TimePreset::Noon => "Noon",
            TimePreset::Afternoon => "Afternoon",
            TimePreset::Sunset => "Sunset",
            TimePreset::Dusk => "Dusk",
            TimePreset::Night => "Night",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TimePreset::Midnight => "[Moon]",
            TimePreset::Dawn => "[Dawn]",
            TimePreset::Sunrise => "🌄",
            TimePreset::Morning => "[Clr]",
            TimePreset::Noon => "[Sun]",
            TimePreset::Afternoon => "⛅",
            TimePreset::Sunset => "🌇",
            TimePreset::Dusk => "🌆",
            TimePreset::Night => "[Ngt]",
        }
    }

    pub fn hour(&self) -> f32 {
        match self {
            TimePreset::Midnight => 0.0,
            TimePreset::Dawn => 5.0,
            TimePreset::Sunrise => 6.5,
            TimePreset::Morning => 9.0,
            TimePreset::Noon => 12.0,
            TimePreset::Afternoon => 15.0,
            TimePreset::Sunset => 18.5,
            TimePreset::Dusk => 20.0,
            TimePreset::Night => 22.0,
        }
    }
}

/// Day/night cycle settings
#[derive(Debug, Clone)]
pub struct TimeSettings {
    pub current_hour: f32,
    pub day_length_minutes: f32,
    pub auto_cycle: bool,
    pub cycle_speed: f32,
    pub sun_angle: f32,
    pub moon_phase: u8,
}

impl Default for TimeSettings {
    fn default() -> Self {
        Self {
            current_hour: 12.0,
            day_length_minutes: 24.0,
            auto_cycle: false,
            cycle_speed: 1.0,
            sun_angle: 45.0,
            moon_phase: 0,
        }
    }
}

impl TimeSettings {
    pub fn is_daytime(&self) -> bool {
        self.current_hour >= 6.0 && self.current_hour < 18.0
    }

    pub fn sun_intensity(&self) -> f32 {
        if self.current_hour < 6.0 {
            0.0
        } else if self.current_hour < 9.0 {
            (self.current_hour - 6.0) / 3.0
        } else if self.current_hour < 15.0 {
            1.0
        } else if self.current_hour < 18.0 {
            1.0 - (self.current_hour - 15.0) / 3.0
        } else {
            0.0
        }
    }

    pub fn format_time(&self) -> String {
        let hours = self.current_hour.floor() as u32 % 24;
        let minutes = ((self.current_hour.fract() * 60.0).floor() as u32) % 60;
        format!("{:02}:{:02}", hours, minutes)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// LIGHTING SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════════

/// Global lighting settings
#[derive(Debug, Clone)]
pub struct LightingSettings {
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
    pub sun_elevation: f32,
    pub sun_azimuth: f32,
    pub shadow_intensity: f32,
    pub shadow_softness: f32,
    pub fog_enabled: bool,
    pub fog_color: [f32; 3],
    pub fog_density: f32,
    pub fog_start: f32,
    pub fog_end: f32,
    pub exposure: f32,
    pub gamma: f32,
}

impl LightingSettings {
    /// Compute a normalized sun direction vector from elevation and azimuth angles (degrees).
    pub fn sun_direction(&self) -> [f32; 3] {
        let elev = self.sun_elevation.to_radians();
        let azim = self.sun_azimuth.to_radians();
        let y = elev.sin();
        let xz = elev.cos();
        let x = xz * azim.cos();
        let z = xz * azim.sin();
        let len = (x * x + y * y + z * z).sqrt();
        if len < 0.0001 {
            return [0.0, 1.0, 0.0];
        }
        [x / len, y / len, z / len]
    }
}

impl Default for LightingSettings {
    fn default() -> Self {
        Self {
            ambient_color: [0.55, 0.60, 0.72],
            ambient_intensity: 0.7,
            sun_color: [1.0, 0.95, 0.85],
            sun_intensity: 1.5,
            sun_elevation: 45.0,
            sun_azimuth: 35.0,
            shadow_intensity: 0.7,
            shadow_softness: 0.5,
            fog_enabled: false,
            fog_color: [0.7, 0.75, 0.8],
            fog_density: 0.0,
            fog_start: 800.0,
            fog_end: 1800.0,
            exposure: 1.2,
            gamma: 2.2,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// WORLD BOUNDS
// ═══════════════════════════════════════════════════════════════════════════════════

/// World boundary settings
#[derive(Debug, Clone)]
pub struct WorldBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub enforce_bounds: bool,
    pub kill_plane_y: f32,
    pub ceiling_y: f32,
}

impl Default for WorldBounds {
    fn default() -> Self {
        Self {
            min: [-1000.0, -100.0, -1000.0],
            max: [1000.0, 500.0, 1000.0],
            enforce_bounds: true,
            kill_plane_y: -50.0,
            ceiling_y: 450.0,
        }
    }
}

impl WorldBounds {
    pub fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        ]
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// ENVIRONMENT PRESET
// ═══════════════════════════════════════════════════════════════════════════════════

/// Environment preset for quick setup
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[non_exhaustive]
pub enum EnvironmentPreset {
    #[default]
    Sunny,
    Overcast,
    Rainy,
    Stormy,
    Foggy,
    Sunset,
    Night,
    DarkNight,
    Arctic,
    Desert,
    Custom,
}

impl std::fmt::Display for EnvironmentPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl EnvironmentPreset {
    pub fn name(&self) -> &'static str {
        match self {
            EnvironmentPreset::Sunny => "Sunny Day",
            EnvironmentPreset::Overcast => "Overcast",
            EnvironmentPreset::Rainy => "Rainy",
            EnvironmentPreset::Stormy => "Stormy",
            EnvironmentPreset::Foggy => "Foggy",
            EnvironmentPreset::Sunset => "Sunset",
            EnvironmentPreset::Night => "Night",
            EnvironmentPreset::DarkNight => "Dark Night",
            EnvironmentPreset::Arctic => "Arctic",
            EnvironmentPreset::Desert => "Desert",
            EnvironmentPreset::Custom => "Custom",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            EnvironmentPreset::Sunny => "[Sun]",
            EnvironmentPreset::Overcast => "[Cld]",
            EnvironmentPreset::Rainy => "[Rain]",
            EnvironmentPreset::Stormy => "⛈️",
            EnvironmentPreset::Foggy => "[Fog]",
            EnvironmentPreset::Sunset => "[Dawn]",
            EnvironmentPreset::Night => "[Moon]",
            EnvironmentPreset::DarkNight => "🌑",
            EnvironmentPreset::Arctic => "❄️",
            EnvironmentPreset::Desert => "🏜️",
            EnvironmentPreset::Custom => "⚙️",
        }
    }

    pub fn all() -> &'static [EnvironmentPreset] {
        &[
            EnvironmentPreset::Sunny,
            EnvironmentPreset::Overcast,
            EnvironmentPreset::Rainy,
            EnvironmentPreset::Stormy,
            EnvironmentPreset::Foggy,
            EnvironmentPreset::Sunset,
            EnvironmentPreset::Night,
            EnvironmentPreset::DarkNight,
            EnvironmentPreset::Arctic,
            EnvironmentPreset::Desert,
            EnvironmentPreset::Custom,
        ]
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// WORLD EVENT LOG
// ═══════════════════════════════════════════════════════════════════════════════════

/// World event for history tracking
#[derive(Debug, Clone)]
pub struct WorldEvent {
    pub timestamp: std::time::Instant,
    pub event_type: WorldEventType,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WorldEventType {
    TerrainGenerated,
    WeatherChanged,
    TimeChanged,
    BiomeChanged,
    SettingsModified,
}

impl std::fmt::Display for WorldEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl WorldEvent {
    pub fn new(event_type: WorldEventType, message: impl Into<String>) -> Self {
        Self {
            timestamp: std::time::Instant::now(),
            event_type,
            message: message.into(),
        }
    }

    pub fn age_secs(&self) -> f32 {
        self.timestamp.elapsed().as_secs_f32()
    }
}

impl WorldEventType {
    pub fn all() -> &'static [WorldEventType] {
        &[
            WorldEventType::TerrainGenerated,
            WorldEventType::WeatherChanged,
            WorldEventType::TimeChanged,
            WorldEventType::BiomeChanged,
            WorldEventType::SettingsModified,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            WorldEventType::TerrainGenerated => "Terrain Generated",
            WorldEventType::WeatherChanged => "Weather Changed",
            WorldEventType::TimeChanged => "Time Changed",
            WorldEventType::BiomeChanged => "Biome Changed",
            WorldEventType::SettingsModified => "Settings Modified",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            WorldEventType::TerrainGenerated => "🏔️",
            WorldEventType::WeatherChanged => "[Clr]",
            WorldEventType::TimeChanged => "⏰",
            WorldEventType::BiomeChanged => "🌲",
            WorldEventType::SettingsModified => "⚙️",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// WORLD PANEL
// ═══════════════════════════════════════════════════════════════════════════════════

/// Result sent back from the background terrain generation thread (world panel)
struct WorldTerrainGenResult {
    terrain_state: TerrainState,
    chunk_count: usize,
    seed: u64,
    biome: String,
}

/// Comprehensive world/environment configuration panel
pub struct WorldPanel {
    // Terrain
    pub terrain_state: TerrainState,
    chunk_radius: i32,
    generation_status: Option<String>,
    auto_regenerate: bool,
    /// True while background terrain generation is in progress
    terrain_generating: bool,
    /// Receiver for completed background terrain generation
    terrain_gen_rx: Option<std::sync::mpsc::Receiver<WorldTerrainGenResult>>,

    // Weather
    weather: WeatherSettings,

    // Time
    time: TimeSettings,

    // Lighting
    lighting: LightingSettings,

    // World bounds
    bounds: WorldBounds,

    // Environment preset
    current_preset: EnvironmentPreset,

    // Event log
    events: VecDeque<WorldEvent>,
    max_events: usize,

    // UI state
    show_weather: bool,
    show_time: bool,
    show_lighting: bool,
    show_bounds: bool,
    show_events: bool,
}

impl WorldPanel {
    pub fn new() -> Self {
        Self {
            terrain_state: TerrainState::new(),
            chunk_radius: 4,
            generation_status: None,
            auto_regenerate: false,
            terrain_generating: false,
            terrain_gen_rx: None,
            weather: WeatherSettings::default(),
            time: TimeSettings::default(),
            lighting: LightingSettings::default(),
            bounds: WorldBounds::default(),
            current_preset: EnvironmentPreset::Sunny,
            events: VecDeque::with_capacity(50),
            max_events: 50,
            show_weather: true,
            show_time: true,
            show_lighting: true,
            show_bounds: false,
            show_events: true,
        }
    }

    fn add_event(&mut self, event_type: WorldEventType, message: impl Into<String>) {
        self.events.push_front(WorldEvent::new(event_type, message));
        if self.events.len() > self.max_events {
            self.events.pop_back();
        }
    }

    fn apply_preset(&mut self, preset: EnvironmentPreset) {
        self.current_preset = preset;

        match preset {
            EnvironmentPreset::Sunny => {
                self.weather.current = WeatherType::Clear;
                self.time.current_hour = 12.0;
                self.lighting.sun_intensity = 1.5;
                self.lighting.sun_elevation = 60.0;
                self.lighting.ambient_intensity = 0.7;
                self.lighting.exposure = 1.2;
                self.lighting.fog_enabled = false;
            }
            EnvironmentPreset::Overcast => {
                self.weather.current = WeatherType::Overcast;
                self.time.current_hour = 12.0;
                self.lighting.sun_intensity = 0.6;
                self.lighting.ambient_intensity = 0.9;
                self.lighting.exposure = 1.0;
            }
            EnvironmentPreset::Rainy => {
                self.weather.current = WeatherType::HeavyRain;
                self.time.current_hour = 14.0;
                self.lighting.sun_intensity = 0.4;
                self.lighting.ambient_intensity = 0.6;
                self.lighting.fog_enabled = true;
                self.lighting.fog_density = 0.02;
            }
            EnvironmentPreset::Stormy => {
                self.weather.current = WeatherType::Thunderstorm;
                self.weather.lightning_frequency = 0.3;
                self.time.current_hour = 15.0;
                self.lighting.sun_intensity = 0.3;
                self.lighting.ambient_intensity = 0.5;
            }
            EnvironmentPreset::Foggy => {
                self.weather.current = WeatherType::Fog;
                self.lighting.fog_enabled = true;
                self.lighting.fog_density = 0.05;
                self.lighting.fog_start = 20.0;
                self.lighting.fog_end = 150.0;
            }
            EnvironmentPreset::Sunset => {
                self.weather.current = WeatherType::Clear;
                self.time.current_hour = 18.5;
                self.lighting.sun_color = [1.0, 0.6, 0.3];
                self.lighting.sun_elevation = 12.0;
                self.lighting.sun_azimuth = 260.0;
                self.lighting.ambient_color = [0.5, 0.35, 0.3];
                self.lighting.exposure = 1.4;
            }
            EnvironmentPreset::Night => {
                self.weather.current = WeatherType::Clear;
                self.time.current_hour = 22.0;
                self.lighting.sun_intensity = 0.0;
                self.lighting.ambient_intensity = 0.15;
                self.lighting.ambient_color = [0.2, 0.25, 0.4];
            }
            EnvironmentPreset::DarkNight => {
                self.weather.current = WeatherType::Overcast;
                self.time.current_hour = 2.0;
                self.lighting.sun_intensity = 0.0;
                self.lighting.ambient_intensity = 0.05;
            }
            EnvironmentPreset::Arctic => {
                self.weather.current = WeatherType::Snow;
                self.time.current_hour = 11.0;
                self.lighting.sun_color = [0.9, 0.95, 1.0];
                self.lighting.ambient_color = [0.7, 0.8, 0.95];
                self.lighting.fog_enabled = true;
                self.lighting.fog_color = [0.9, 0.95, 1.0];
            }
            EnvironmentPreset::Desert => {
                self.weather.current = WeatherType::Clear;
                self.time.current_hour = 14.0;
                self.lighting.sun_intensity = 1.3;
                self.lighting.sun_color = [1.0, 0.9, 0.75];
                self.lighting.exposure = 0.8;
            }
            EnvironmentPreset::Custom => {}
        }

        self.add_event(
            WorldEventType::SettingsModified,
            format!("Applied preset: {}", preset.name()),
        );
    }

    pub fn show_with_level(&mut self, ui: &mut Ui, level: &mut LevelDoc) {
        ui.heading("[Glb] World Settings");
        ui.separator();

        // Quick summary bar
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    self.weather.current.icon(),
                    self.weather.current.name()
                ))
                .strong(),
            );
            ui.separator();
            ui.label(format!("🕐 {}", self.time.format_time()));
            ui.separator();
            ui.label(format!("🌡️ {:.0}% sun", self.time.sun_intensity() * 100.0));
        });

        ui.separator();

        // Display toggles
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_weather, "Weather");
            ui.checkbox(&mut self.show_time, "Time");
            ui.checkbox(&mut self.show_lighting, "Lighting");
            ui.checkbox(&mut self.show_bounds, "Bounds");
            ui.checkbox(&mut self.show_events, "Events");
        });

        ui.separator();

        // Environment presets
        ui.collapsing("Environment Presets", |ui| {
            ui.horizontal_wrapped(|ui| {
                for preset in EnvironmentPreset::all() {
                    let selected = self.current_preset == *preset;
                    let text = format!("{} {}", preset.icon(), preset.name());
                    if ui.selectable_label(selected, text).clicked() {
                        self.apply_preset(*preset);
                    }
                }
            });
        });

        let old_biome = level.biome.clone();
        let old_seed = level.seed;

        // Terrain section
        ui.collapsing("🏔️ Terrain", |ui| {
            ui.group(|ui| {
                ui.label("Biome");

                let options = cached_biome_options();
                egui::ComboBox::from_id_salt("biome_selector")
                    .selected_text(biome_display_name(&level.biome))
                    .show_ui(ui, |ui| {
                        for opt in &options {
                            if ui
                                .selectable_label(level.biome == opt.value, &opt.display)
                                .clicked()
                            {
                                level.biome = opt.value.clone();
                            }
                        }
                    });
            });

            ui.add_space(5.0);

            ui.group(|ui| {
                ui.label("Seed");
                ui.add(egui::Slider::new(&mut level.seed, 0..=99999).text("seed"));
                if ui.button("[Dice] Randomize").clicked() {
                    level.seed = rand::random::<u64>() % 100000;
                }
            });

            ui.add_space(5.0);

            ui.group(|ui| {
                ui.label("Generation");

                ui.horizontal(|ui| {
                    ui.label("Chunk Radius:");
                    ui.add(egui::Slider::new(&mut self.chunk_radius, 1..=12).text(""));
                });

                let chunks_to_generate = (self.chunk_radius * 2 + 1).pow(2);
                ui.label(format!("Will generate {} chunks", chunks_to_generate));

                ui.checkbox(&mut self.auto_regenerate, "Auto-regenerate on change");

                ui.add_space(5.0);

                // Poll for completed background generation
                if let Some(rx) = &self.terrain_gen_rx {
                    match rx.try_recv() {
                        Ok(result) => {
                            let msg = format!(
                                "Generated {} chunks (seed={}, biome={})",
                                result.chunk_count,
                                result.seed,
                                biome_display_name(&result.biome)
                            );
                            self.terrain_state = result.terrain_state;
                            self.generation_status = Some(msg.clone());
                            self.add_event(WorldEventType::TerrainGenerated, msg);
                            self.terrain_generating = false;
                            self.terrain_gen_rx = None;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            // Still generating
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            self.generation_status = Some("Generation failed".to_string());
                            self.terrain_generating = false;
                            self.terrain_gen_rx = None;
                        }
                    }
                }

                if self.terrain_generating {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Generating terrain...");
                    });
                } else {
                    let generate_clicked = ui.button("Generate Terrain").clicked();

                    self.terrain_state.configure(level.seed, &level.biome);

                    let should_generate = generate_clicked
                        || (self.auto_regenerate
                            && (old_biome != level.biome || old_seed != level.seed));

                    if should_generate {
                        // Run terrain generation on a background thread to avoid
                        // freezing the UI.
                        let mut state = TerrainState::new();
                        state.configure(level.seed, &level.biome);
                        let chunk_radius = self.chunk_radius;
                        let seed = level.seed;
                        let biome = level.biome.clone();

                        let (tx, rx) = std::sync::mpsc::channel();
                        self.terrain_generating = true;
                        self.terrain_gen_rx = Some(rx);
                        self.generation_status = Some("Generating...".to_string());

                        std::thread::spawn(move || match state.generate_terrain(chunk_radius) {
                            Ok(count) => {
                                let _ = tx.send(WorldTerrainGenResult {
                                    terrain_state: state,
                                    chunk_count: count,
                                    seed,
                                    biome,
                                });
                            }
                            Err(e) => {
                                tracing::error!("World panel terrain generation failed: {e}");
                            }
                        });
                    }
                }

                if let Some(status) = &self.generation_status {
                    ui.add_space(5.0);
                    ui.label(RichText::new(status).small());
                }

                if self.terrain_state.chunk_count() > 0 {
                    ui.label(format!(
                        "Active: {} chunks",
                        self.terrain_state.chunk_count()
                    ));
                }
            });
        });

        // Weather section
        if self.show_weather {
            ui.collapsing("[Clr] Weather", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_id_salt("weather_type")
                        .selected_text(format!(
                            "{} {}",
                            self.weather.current.icon(),
                            self.weather.current.name()
                        ))
                        .show_ui(ui, |ui| {
                            for w in WeatherType::all() {
                                let text = format!("{} {}", w.icon(), w.name());
                                if ui
                                    .selectable_label(self.weather.current == *w, text)
                                    .clicked()
                                {
                                    let old = self.weather.current;
                                    self.weather.current = *w;
                                    if old != *w {
                                        self.add_event(
                                            WorldEventType::WeatherChanged,
                                            format!("Weather: {} → {}", old.name(), w.name()),
                                        );
                                    }
                                }
                            }
                        });
                });

                ui.label(
                    RichText::new(self.weather.current.description())
                        .small()
                        .weak(),
                );

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("Intensity:");
                    ui.add(
                        egui::Slider::new(&mut self.weather.intensity, 0.0..=1.0).show_value(true),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Wind Speed:");
                    ui.add(
                        egui::Slider::new(&mut self.weather.wind_speed, 0.0..=50.0).suffix(" m/s"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Wind Direction:");
                    ui.add(
                        egui::Slider::new(&mut self.weather.wind_direction, 0.0..=360.0)
                            .suffix("°"),
                    );
                });

                if matches!(
                    self.weather.current,
                    WeatherType::LightRain
                        | WeatherType::HeavyRain
                        | WeatherType::Snow
                        | WeatherType::Blizzard
                        | WeatherType::Hail
                ) {
                    ui.horizontal(|ui| {
                        ui.label("Precipitation:");
                        ui.add(egui::Slider::new(
                            &mut self.weather.precipitation_density,
                            0.0..=1.0,
                        ));
                    });
                }

                if self.weather.current == WeatherType::Thunderstorm {
                    ui.horizontal(|ui| {
                        ui.label("Lightning:");
                        ui.add(egui::Slider::new(
                            &mut self.weather.lightning_frequency,
                            0.0..=1.0,
                        ));
                    });
                }

                ui.horizontal(|ui| {
                    ui.label("Transition Time:");
                    ui.add(
                        egui::Slider::new(&mut self.weather.transition_time, 1.0..=120.0)
                            .suffix(" s"),
                    );
                });
            });
        }

        // Time section
        if self.show_time {
            ui.collapsing("🕐 Time of Day", |ui| {
                // Time presets
                ui.horizontal_wrapped(|ui| {
                    for preset in TimePreset::all() {
                        let text = format!("{} {}", preset.icon(), preset.name());
                        if ui.small_button(text).clicked() {
                            let old_hour = self.time.current_hour;
                            self.time.current_hour = preset.hour();
                            level.sky.time_of_day = preset.name().to_lowercase();
                            self.add_event(
                                WorldEventType::TimeChanged,
                                format!(
                                    "Time: {:02}:00 → {:02}:00",
                                    old_hour as u32,
                                    preset.hour() as u32
                                ),
                            );
                        }
                    }
                });

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("Hour:");
                    ui.add(
                        egui::Slider::new(&mut self.time.current_hour, 0.0..=23.99)
                            .custom_formatter(|v, _| {
                                let h = v.floor() as u32 % 24;
                                let m = ((v.fract() * 60.0).floor() as u32) % 60;
                                format!("{:02}:{:02}", h, m)
                            }),
                    );
                });

                let daytime_icon = if self.time.is_daytime() {
                    "[Sun]"
                } else {
                    "[Moon]"
                };
                ui.label(format!(
                    "{} {} (Sun: {:.0}%)",
                    daytime_icon,
                    if self.time.is_daytime() {
                        "Daytime"
                    } else {
                        "Nighttime"
                    },
                    self.time.sun_intensity() * 100.0
                ));

                ui.add_space(5.0);

                ui.checkbox(&mut self.time.auto_cycle, "Auto day/night cycle");
                if self.time.auto_cycle {
                    ui.horizontal(|ui| {
                        ui.label("Day Length:");
                        ui.add(
                            egui::Slider::new(&mut self.time.day_length_minutes, 1.0..=60.0)
                                .suffix(" min"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Cycle Speed:");
                        ui.add(
                            egui::Slider::new(&mut self.time.cycle_speed, 0.1..=10.0).suffix("x"),
                        );
                    });
                }
            });
        }

        // Lighting section
        if self.show_lighting {
            ui.collapsing("Lighting", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Sun Intensity:");
                    ui.add(egui::Slider::new(
                        &mut self.lighting.sun_intensity,
                        0.0..=3.0,
                    ));
                });

                ui.horizontal(|ui| {
                    ui.label("Sun Elevation:");
                    ui.add(
                        egui::Slider::new(&mut self.lighting.sun_elevation, 5.0..=90.0).suffix("°"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Sun Azimuth:");
                    ui.add(
                        egui::Slider::new(&mut self.lighting.sun_azimuth, 0.0..=360.0).suffix("°"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Ambient Intensity:");
                    ui.add(egui::Slider::new(
                        &mut self.lighting.ambient_intensity,
                        0.0..=2.0,
                    ));
                });

                ui.horizontal(|ui| {
                    ui.label("Shadow Intensity:");
                    ui.add(egui::Slider::new(
                        &mut self.lighting.shadow_intensity,
                        0.0..=1.0,
                    ));
                });

                ui.add_space(5.0);

                ui.checkbox(&mut self.lighting.fog_enabled, "Enable Fog");
                if self.lighting.fog_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Fog Start:");
                        ui.add(
                            egui::Slider::new(&mut self.lighting.fog_start, 10.0..=2000.0)
                                .suffix(" m"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Fog End:");
                        ui.add(
                            egui::Slider::new(&mut self.lighting.fog_end, 50.0..=5000.0)
                                .suffix(" m"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Fog Density:");
                        ui.add(
                            egui::Slider::new(&mut self.lighting.fog_density, 0.0..=0.05),
                        );
                    });
                }

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("Exposure:");
                    ui.add(egui::Slider::new(&mut self.lighting.exposure, 0.1..=3.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Gamma:");
                    ui.add(egui::Slider::new(&mut self.lighting.gamma, 1.0..=3.0));
                });
            });
        }

        // World bounds section
        if self.show_bounds {
            ui.collapsing("[Sq] World Bounds", |ui| {
                ui.checkbox(&mut self.bounds.enforce_bounds, "Enforce boundaries");

                ui.add_space(5.0);

                let size = self.bounds.size();
                ui.label(format!(
                    "World Size: {:.0} x {:.0} x {:.0} m",
                    size[0], size[1], size[2]
                ));

                ui.horizontal(|ui| {
                    ui.label("Kill Plane Y:");
                    ui.add(egui::DragValue::new(&mut self.bounds.kill_plane_y).speed(1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Ceiling Y:");
                    ui.add(egui::DragValue::new(&mut self.bounds.ceiling_y).speed(1.0));
                });
            });
        }

        // Event log section
        if self.show_events && !self.events.is_empty() {
            ui.collapsing(format!("Events ({})", self.events.len()), |ui| {
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        for event in self.events.iter().take(10) {
                            let age = event.age_secs();
                            let alpha = if age < 30.0 {
                                255
                            } else {
                                (255.0 * (1.0 - (age - 30.0) / 60.0).max(0.0)) as u8
                            };
                            let color = Color32::from_rgba_unmultiplied(200, 200, 200, alpha);

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(event.event_type.icon()).color(color));
                                ui.label(RichText::new(&event.message).color(color).small());
                                ui.label(RichText::new(format!("{:.0}s", age)).weak().small());
                            });
                        }
                    });
            });
        }
    }

    pub fn terrain_state(&self) -> &TerrainState {
        &self.terrain_state
    }

    pub fn terrain_state_mut(&mut self) -> &mut TerrainState {
        &mut self.terrain_state
    }

    pub fn weather(&self) -> &WeatherSettings {
        &self.weather
    }

    pub fn time(&self) -> &TimeSettings {
        &self.time
    }

    pub fn lighting(&self) -> &LightingSettings {
        &self.lighting
    }

    pub fn bounds(&self) -> &WorldBounds {
        &self.bounds
    }
}

impl Default for WorldPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for WorldPanel {
    fn name(&self) -> &str {
        "World"
    }

    fn show(&mut self, _ui: &mut Ui) {}
}

// ═══════════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // WEATHER TYPE TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_weather_type_name() {
        assert_eq!(WeatherType::Clear.name(), "Clear");
        assert_eq!(WeatherType::Thunderstorm.name(), "Thunderstorm");
        assert_eq!(WeatherType::Blizzard.name(), "Blizzard");
    }

    #[test]
    fn test_weather_type_icon() {
        assert!(!WeatherType::Clear.icon().is_empty());
        assert!(!WeatherType::HeavyRain.icon().is_empty());
        assert!(!WeatherType::Snow.icon().is_empty());
    }

    #[test]
    fn test_weather_type_description() {
        assert!(!WeatherType::Clear.description().is_empty());
        assert!(!WeatherType::Fog.description().is_empty());
    }

    #[test]
    fn test_weather_type_ambient_modifier() {
        assert_eq!(WeatherType::Clear.ambient_modifier(), 1.0);
        assert!(WeatherType::Thunderstorm.ambient_modifier() < 1.0);
        assert!(WeatherType::Blizzard.ambient_modifier() < 0.5);
    }

    #[test]
    fn test_weather_type_all() {
        let all = WeatherType::all();
        assert_eq!(all.len(), 11);
        assert!(all.contains(&WeatherType::Clear));
        assert!(all.contains(&WeatherType::Thunderstorm));
    }

    #[test]
    fn test_weather_type_default() {
        assert_eq!(WeatherType::default(), WeatherType::Clear);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WEATHER SETTINGS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_weather_settings_default() {
        let settings = WeatherSettings::default();
        assert_eq!(settings.current, WeatherType::Clear);
        assert_eq!(settings.intensity, 1.0);
        assert!(settings.wind_speed >= 0.0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TIME PRESET TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_time_preset_name() {
        assert_eq!(TimePreset::Noon.name(), "Noon");
        assert_eq!(TimePreset::Midnight.name(), "Midnight");
        assert_eq!(TimePreset::Sunset.name(), "Sunset");
    }

    #[test]
    fn test_time_preset_icon() {
        assert!(!TimePreset::Dawn.icon().is_empty());
        assert!(!TimePreset::Night.icon().is_empty());
    }

    #[test]
    fn test_time_preset_hour() {
        assert_eq!(TimePreset::Midnight.hour(), 0.0);
        assert_eq!(TimePreset::Noon.hour(), 12.0);
        assert!(TimePreset::Sunset.hour() > 18.0);
    }

    #[test]
    fn test_time_preset_all() {
        let all = TimePreset::all();
        assert_eq!(all.len(), 9);
    }

    #[test]
    fn test_time_preset_default() {
        assert_eq!(TimePreset::default(), TimePreset::Noon);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TIME SETTINGS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_time_settings_default() {
        let settings = TimeSettings::default();
        assert_eq!(settings.current_hour, 12.0);
        assert!(!settings.auto_cycle);
    }

    #[test]
    fn test_time_settings_is_daytime() {
        let mut settings = TimeSettings {
            current_hour: 12.0,
            ..Default::default()
        };
        assert!(settings.is_daytime());

        settings.current_hour = 2.0;
        assert!(!settings.is_daytime());

        settings.current_hour = 22.0;
        assert!(!settings.is_daytime());
    }

    #[test]
    fn test_time_settings_sun_intensity() {
        let mut settings = TimeSettings {
            current_hour: 12.0,
            ..Default::default()
        };
        assert_eq!(settings.sun_intensity(), 1.0);

        settings.current_hour = 2.0;
        assert_eq!(settings.sun_intensity(), 0.0);

        settings.current_hour = 7.5;
        assert!(settings.sun_intensity() > 0.0 && settings.sun_intensity() < 1.0);
    }

    #[test]
    fn test_time_settings_format_time() {
        let mut settings = TimeSettings {
            current_hour: 12.0,
            ..Default::default()
        };
        assert_eq!(settings.format_time(), "12:00");

        settings.current_hour = 9.5;
        assert_eq!(settings.format_time(), "09:30");

        settings.current_hour = 0.0;
        assert_eq!(settings.format_time(), "00:00");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LIGHTING SETTINGS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_lighting_settings_default() {
        let settings = LightingSettings::default();
        assert_eq!(settings.sun_intensity, 1.5);
        assert!(!settings.fog_enabled);
        assert_eq!(settings.gamma, 2.2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WORLD BOUNDS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_world_bounds_default() {
        let bounds = WorldBounds::default();
        assert!(bounds.enforce_bounds);
        assert!(bounds.kill_plane_y < bounds.ceiling_y);
    }

    #[test]
    fn test_world_bounds_size() {
        let bounds = WorldBounds {
            min: [0.0, 0.0, 0.0],
            max: [100.0, 50.0, 200.0],
            ..Default::default()
        };

        let size = bounds.size();
        assert_eq!(size[0], 100.0);
        assert_eq!(size[1], 50.0);
        assert_eq!(size[2], 200.0);
    }

    #[test]
    fn test_world_bounds_center() {
        let bounds = WorldBounds {
            min: [0.0, 0.0, 0.0],
            max: [100.0, 100.0, 100.0],
            ..Default::default()
        };

        let center = bounds.center();
        assert_eq!(center[0], 50.0);
        assert_eq!(center[1], 50.0);
        assert_eq!(center[2], 50.0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENVIRONMENT PRESET TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_environment_preset_name() {
        assert_eq!(EnvironmentPreset::Sunny.name(), "Sunny Day");
        assert_eq!(EnvironmentPreset::Stormy.name(), "Stormy");
        assert_eq!(EnvironmentPreset::Arctic.name(), "Arctic");
    }

    #[test]
    fn test_environment_preset_icon() {
        assert!(!EnvironmentPreset::Sunny.icon().is_empty());
        assert!(!EnvironmentPreset::Night.icon().is_empty());
    }

    #[test]
    fn test_environment_preset_all() {
        let all = EnvironmentPreset::all();
        assert_eq!(all.len(), 11);
        assert!(all.contains(&EnvironmentPreset::Custom));
    }

    #[test]
    fn test_environment_preset_default() {
        assert_eq!(EnvironmentPreset::default(), EnvironmentPreset::Sunny);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WORLD EVENT TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_world_event_creation() {
        let event = WorldEvent::new(WorldEventType::TerrainGenerated, "Test event");
        assert_eq!(event.event_type, WorldEventType::TerrainGenerated);
        assert_eq!(event.message, "Test event");
    }

    #[test]
    fn test_world_event_age() {
        let event = WorldEvent::new(WorldEventType::WeatherChanged, "Test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(event.age_secs() >= 0.01);
    }

    #[test]
    fn test_world_event_type_icon() {
        assert!(!WorldEventType::TerrainGenerated.icon().is_empty());
        assert!(!WorldEventType::WeatherChanged.icon().is_empty());
        assert!(!WorldEventType::TimeChanged.icon().is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WORLD PANEL TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_world_panel_creation() {
        let panel = WorldPanel::new();
        assert_eq!(panel.chunk_radius, 4);
        assert!(!panel.auto_regenerate);
        assert_eq!(panel.current_preset, EnvironmentPreset::Sunny);
    }

    #[test]
    fn test_world_panel_default() {
        let panel: WorldPanel = Default::default();
        assert!(panel.show_weather);
        assert!(panel.show_time);
    }

    #[test]
    fn test_world_panel_name() {
        let panel = WorldPanel::new();
        assert_eq!(panel.name(), "World");
    }

    #[test]
    fn test_world_panel_terrain_state() {
        let panel = WorldPanel::new();
        let _ = panel.terrain_state();
    }

    #[test]
    fn test_world_panel_terrain_state_mut() {
        let mut panel = WorldPanel::new();
        let _ = panel.terrain_state_mut();
    }

    #[test]
    fn test_world_panel_weather() {
        let panel = WorldPanel::new();
        assert_eq!(panel.weather().current, WeatherType::Clear);
    }

    #[test]
    fn test_world_panel_time() {
        let panel = WorldPanel::new();
        assert_eq!(panel.time().current_hour, 12.0);
    }

    #[test]
    fn test_world_panel_lighting() {
        let panel = WorldPanel::new();
        assert_eq!(panel.lighting().sun_intensity, 1.5);
    }

    #[test]
    fn test_world_panel_bounds() {
        let panel = WorldPanel::new();
        assert!(panel.bounds().enforce_bounds);
    }

    #[test]
    fn test_world_panel_add_event() {
        let mut panel = WorldPanel::new();
        panel.add_event(WorldEventType::TerrainGenerated, "Test event");
        assert_eq!(panel.events.len(), 1);
    }

    #[test]
    fn test_world_panel_apply_preset_sunny() {
        let mut panel = WorldPanel::new();
        panel.apply_preset(EnvironmentPreset::Sunny);

        assert_eq!(panel.current_preset, EnvironmentPreset::Sunny);
        assert_eq!(panel.weather.current, WeatherType::Clear);
        assert_eq!(panel.time.current_hour, 12.0);
    }

    #[test]
    fn test_world_panel_apply_preset_rainy() {
        let mut panel = WorldPanel::new();
        panel.apply_preset(EnvironmentPreset::Rainy);

        assert_eq!(panel.current_preset, EnvironmentPreset::Rainy);
        assert_eq!(panel.weather.current, WeatherType::HeavyRain);
        assert!(panel.lighting.fog_enabled);
    }

    #[test]
    fn test_world_panel_apply_preset_night() {
        let mut panel = WorldPanel::new();
        panel.apply_preset(EnvironmentPreset::Night);

        assert_eq!(panel.current_preset, EnvironmentPreset::Night);
        assert_eq!(panel.time.current_hour, 22.0);
        assert_eq!(panel.lighting.sun_intensity, 0.0);
    }

    #[test]
    fn test_world_panel_event_limit() {
        let mut panel = WorldPanel::new();
        panel.max_events = 5;

        for i in 0..10 {
            panel.add_event(WorldEventType::SettingsModified, format!("Event {}", i));
        }

        assert_eq!(panel.events.len(), 5);
    }

    // =====================================================================
    // WeatherType Enum Tests
    // =====================================================================

    #[test]
    fn test_weather_type_display() {
        for weather in WeatherType::all() {
            let display = format!("{}", weather);
            assert!(
                display.contains(weather.name()),
                "Display should contain name"
            );
        }
    }

    #[test]
    fn test_weather_type_all_variants() {
        let variants = WeatherType::all();
        assert_eq!(variants.len(), 11, "Expected 11 weather type variants");
        assert!(variants.contains(&WeatherType::Clear));
        assert!(variants.contains(&WeatherType::Thunderstorm));
        assert!(variants.contains(&WeatherType::Snow));
    }

    #[test]
    fn test_weather_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for weather in WeatherType::all() {
            set.insert(*weather);
        }
        assert_eq!(set.len(), WeatherType::all().len());
    }

    // =====================================================================
    // TimePreset Enum Tests
    // =====================================================================

    #[test]
    fn test_time_preset_display() {
        for preset in TimePreset::all() {
            let display = format!("{}", preset);
            assert!(
                display.contains(preset.name()),
                "Display should contain name"
            );
        }
    }

    #[test]
    fn test_time_preset_all_variants() {
        let variants = TimePreset::all();
        assert_eq!(variants.len(), 9, "Expected 9 time preset variants");
        assert!(variants.contains(&TimePreset::Midnight));
        assert!(variants.contains(&TimePreset::Noon));
        assert!(variants.contains(&TimePreset::Sunset));
    }

    #[test]
    fn test_time_preset_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for preset in TimePreset::all() {
            set.insert(*preset);
        }
        assert_eq!(set.len(), TimePreset::all().len());
    }

    // =====================================================================
    // EnvironmentPreset Enum Tests
    // =====================================================================

    #[test]
    fn test_environment_preset_display() {
        for preset in EnvironmentPreset::all() {
            let display = format!("{}", preset);
            assert!(
                display.contains(preset.name()),
                "Display should contain name"
            );
        }
    }

    #[test]
    fn test_environment_preset_all_variants() {
        let variants = EnvironmentPreset::all();
        assert_eq!(
            variants.len(),
            11,
            "Expected 11 environment preset variants"
        );
        assert!(variants.contains(&EnvironmentPreset::Sunny));
        assert!(variants.contains(&EnvironmentPreset::Stormy));
        assert!(variants.contains(&EnvironmentPreset::Custom));
    }

    #[test]
    fn test_environment_preset_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for preset in EnvironmentPreset::all() {
            set.insert(*preset);
        }
        assert_eq!(set.len(), EnvironmentPreset::all().len());
    }

    // =====================================================================
    // WorldEventType Enum Tests
    // =====================================================================

    #[test]
    fn test_world_event_type_display() {
        for event in WorldEventType::all() {
            let display = format!("{}", event);
            assert!(
                display.contains(event.name()),
                "Display should contain name"
            );
        }
    }

    #[test]
    fn test_world_event_type_all_variants() {
        let variants = WorldEventType::all();
        assert_eq!(variants.len(), 5, "Expected 5 world event type variants");
        assert!(variants.contains(&WorldEventType::TerrainGenerated));
        assert!(variants.contains(&WorldEventType::WeatherChanged));
        assert!(variants.contains(&WorldEventType::SettingsModified));
    }

    #[test]
    fn test_world_event_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for event in WorldEventType::all() {
            set.insert(*event);
        }
        assert_eq!(set.len(), WorldEventType::all().len());
    }

    #[test]
    fn test_world_event_type_name() {
        assert_eq!(WorldEventType::TerrainGenerated.name(), "Terrain Generated");
        assert_eq!(WorldEventType::WeatherChanged.name(), "Weather Changed");
        assert_eq!(WorldEventType::SettingsModified.name(), "Settings Modified");
    }
}
