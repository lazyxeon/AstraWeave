//! Environment Preset System - Unified Sky, Fog, Lighting, and Post-Processing Presets
//!
//! Provides one-click environment setup with coordinated:
//! - **Sky Settings**: Skybox, atmospheric scattering, sun position
//! - **Fog Configuration**: Density, color, falloff, height fog
//! - **Lighting Setup**: Sun color, ambient, indirect lighting
//! - **Post-Processing**: Tonemapping, exposure, color grading
//!
//! Presets can be applied instantly or blended for smooth transitions.

use egui::{Color32, RichText, Ui};
use tracing::info;

use crate::panels::Panel;

// ============================================================================
// PANEL ACTIONS - Events produced by the panel for external handling
// ============================================================================

/// Actions emitted by the environment preset panel
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EnvironmentAction {
    /// Apply current settings to the scene
    ApplySettings { settings: EnvironmentSettings },
    /// Save current settings as a named preset
    SavePreset {
        name: String,
        settings: EnvironmentSettings,
    },
    /// Load a preset by name
    LoadPreset { name: String },
    /// Apply a quick preset (time of day or weather)
    ApplyQuickPreset {
        time: TimeOfDay,
        weather: WeatherCondition,
    },
    /// Reset to default settings
    ResetToDefault,
    /// Start environment preview mode
    StartPreview,
    /// Stop environment preview mode
    StopPreview,
}

// ============================================================================
// TIME OF DAY - Master time control
// ============================================================================

/// Time of day presets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TimeOfDay {
    Dawn,
    EarlyMorning,
    Morning,
    #[default]
    Noon,
    Afternoon,
    GoldenHour,
    Sunset,
    Dusk,
    BlueHour,
    Night,
    Midnight,
    LateNight,
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl TimeOfDay {
    /// All time of day values
    pub fn all() -> &'static [TimeOfDay] {
        &[
            TimeOfDay::Dawn,
            TimeOfDay::EarlyMorning,
            TimeOfDay::Morning,
            TimeOfDay::Noon,
            TimeOfDay::Afternoon,
            TimeOfDay::GoldenHour,
            TimeOfDay::Sunset,
            TimeOfDay::Dusk,
            TimeOfDay::BlueHour,
            TimeOfDay::Night,
            TimeOfDay::Midnight,
            TimeOfDay::LateNight,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            TimeOfDay::Dawn => "Dawn",
            TimeOfDay::EarlyMorning => "Early Morning",
            TimeOfDay::Morning => "Morning",
            TimeOfDay::Noon => "Noon",
            TimeOfDay::Afternoon => "Afternoon",
            TimeOfDay::GoldenHour => "Golden Hour",
            TimeOfDay::Sunset => "Sunset",
            TimeOfDay::Dusk => "Dusk",
            TimeOfDay::BlueHour => "Blue Hour",
            TimeOfDay::Night => "Night",
            TimeOfDay::Midnight => "Midnight",
            TimeOfDay::LateNight => "Late Night",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            TimeOfDay::Dawn => "[Dawn]",
            TimeOfDay::EarlyMorning => "[Clr]",
            TimeOfDay::Morning => "[Sun]",
            TimeOfDay::Noon => "🌞",
            TimeOfDay::Afternoon => "⛅",
            TimeOfDay::GoldenHour => "🌇",
            TimeOfDay::Sunset => "🌆",
            TimeOfDay::Dusk => "[Ngt]",
            TimeOfDay::BlueHour => "🌌",
            TimeOfDay::Night => "[Moon]",
            TimeOfDay::Midnight => "🌑",
            TimeOfDay::LateNight => "[Fx]",
        }
    }

    /// Sun elevation angle in degrees (0 = horizon, 90 = zenith)
    pub fn sun_elevation(&self) -> f32 {
        match self {
            TimeOfDay::Dawn => 5.0,
            TimeOfDay::EarlyMorning => 15.0,
            TimeOfDay::Morning => 35.0,
            TimeOfDay::Noon => 75.0,
            TimeOfDay::Afternoon => 55.0,
            TimeOfDay::GoldenHour => 15.0,
            TimeOfDay::Sunset => 3.0,
            TimeOfDay::Dusk => -5.0,
            TimeOfDay::BlueHour => -10.0,
            TimeOfDay::Night => -30.0,
            TimeOfDay::Midnight => -45.0,
            TimeOfDay::LateNight => -35.0,
        }
    }

    /// Sun color temperature in Kelvin
    pub fn sun_temperature(&self) -> u32 {
        match self {
            TimeOfDay::Dawn => 2500,
            TimeOfDay::EarlyMorning => 3500,
            TimeOfDay::Morning => 5500,
            TimeOfDay::Noon => 6500,
            TimeOfDay::Afternoon => 5800,
            TimeOfDay::GoldenHour => 3000,
            TimeOfDay::Sunset => 2200,
            TimeOfDay::Dusk => 2800,
            TimeOfDay::BlueHour => 8000,
            TimeOfDay::Night => 10000,
            TimeOfDay::Midnight => 12000,
            TimeOfDay::LateNight => 10000,
        }
    }

    /// Whether sun is above horizon
    pub fn is_daytime(&self) -> bool {
        self.sun_elevation() > 0.0
    }

    /// Hour of day (0-24)
    pub fn hour(&self) -> f32 {
        match self {
            TimeOfDay::Dawn => 5.5,
            TimeOfDay::EarlyMorning => 7.0,
            TimeOfDay::Morning => 9.0,
            TimeOfDay::Noon => 12.0,
            TimeOfDay::Afternoon => 15.0,
            TimeOfDay::GoldenHour => 18.0,
            TimeOfDay::Sunset => 19.5,
            TimeOfDay::Dusk => 20.0,
            TimeOfDay::BlueHour => 20.5,
            TimeOfDay::Night => 22.0,
            TimeOfDay::Midnight => 0.0,
            TimeOfDay::LateNight => 3.0,
        }
    }
}

// ============================================================================
// WEATHER CONDITION - Atmospheric effects
// ============================================================================

/// Weather conditions
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WeatherCondition {
    #[default]
    Clear,
    PartlyCloudy,
    Overcast,
    LightRain,
    HeavyRain,
    Thunderstorm,
    LightSnow,
    HeavySnow,
    Blizzard,
    Foggy,
    DenseFog,
    Sandstorm,
    Haze,
    Windy,
}

impl std::fmt::Display for WeatherCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl WeatherCondition {
    /// All weather conditions
    pub fn all() -> &'static [WeatherCondition] {
        &[
            WeatherCondition::Clear,
            WeatherCondition::PartlyCloudy,
            WeatherCondition::Overcast,
            WeatherCondition::LightRain,
            WeatherCondition::HeavyRain,
            WeatherCondition::Thunderstorm,
            WeatherCondition::LightSnow,
            WeatherCondition::HeavySnow,
            WeatherCondition::Blizzard,
            WeatherCondition::Foggy,
            WeatherCondition::DenseFog,
            WeatherCondition::Sandstorm,
            WeatherCondition::Haze,
            WeatherCondition::Windy,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            WeatherCondition::Clear => "Clear",
            WeatherCondition::PartlyCloudy => "Partly Cloudy",
            WeatherCondition::Overcast => "Overcast",
            WeatherCondition::LightRain => "Light Rain",
            WeatherCondition::HeavyRain => "Heavy Rain",
            WeatherCondition::Thunderstorm => "Thunderstorm",
            WeatherCondition::LightSnow => "Light Snow",
            WeatherCondition::HeavySnow => "Heavy Snow",
            WeatherCondition::Blizzard => "Blizzard",
            WeatherCondition::Foggy => "Foggy",
            WeatherCondition::DenseFog => "Dense Fog",
            WeatherCondition::Sandstorm => "Sandstorm",
            WeatherCondition::Haze => "Haze",
            WeatherCondition::Windy => "Windy",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            WeatherCondition::Clear => "[Sun]",
            WeatherCondition::PartlyCloudy => "⛅",
            WeatherCondition::Overcast => "[Cld]",
            WeatherCondition::LightRain => "[Rain]",
            WeatherCondition::HeavyRain => "⛈️",
            WeatherCondition::Thunderstorm => "🌩️",
            WeatherCondition::LightSnow => "🌨️",
            WeatherCondition::HeavySnow => "❄️",
            WeatherCondition::Blizzard => "🌬️",
            WeatherCondition::Foggy => "[Fog]",
            WeatherCondition::DenseFog => "🌁",
            WeatherCondition::Sandstorm => "[Dash]",
            WeatherCondition::Haze => "[Haze]",
            WeatherCondition::Windy => "[Dash]",
        }
    }

    /// Cloud coverage (0.0 to 1.0)
    pub fn cloud_coverage(&self) -> f32 {
        match self {
            WeatherCondition::Clear => 0.0,
            WeatherCondition::PartlyCloudy => 0.3,
            WeatherCondition::Overcast => 0.9,
            WeatherCondition::LightRain => 0.7,
            WeatherCondition::HeavyRain => 0.95,
            WeatherCondition::Thunderstorm => 1.0,
            WeatherCondition::LightSnow => 0.6,
            WeatherCondition::HeavySnow => 0.85,
            WeatherCondition::Blizzard => 1.0,
            WeatherCondition::Foggy => 0.4,
            WeatherCondition::DenseFog => 0.5,
            WeatherCondition::Sandstorm => 0.3,
            WeatherCondition::Haze => 0.2,
            WeatherCondition::Windy => 0.2,
        }
    }

    /// Fog density multiplier
    pub fn fog_multiplier(&self) -> f32 {
        match self {
            WeatherCondition::Clear => 0.2,
            WeatherCondition::PartlyCloudy => 0.3,
            WeatherCondition::Overcast => 0.5,
            WeatherCondition::LightRain => 0.6,
            WeatherCondition::HeavyRain => 0.8,
            WeatherCondition::Thunderstorm => 0.9,
            WeatherCondition::LightSnow => 0.5,
            WeatherCondition::HeavySnow => 0.7,
            WeatherCondition::Blizzard => 0.95,
            WeatherCondition::Foggy => 1.0,
            WeatherCondition::DenseFog => 1.5,
            WeatherCondition::Sandstorm => 1.2,
            WeatherCondition::Haze => 0.7,
            WeatherCondition::Windy => 0.3,
        }
    }

    /// Sun intensity reduction (0.0 = full sun, 1.0 = no sun)
    pub fn sun_attenuation(&self) -> f32 {
        match self {
            WeatherCondition::Clear => 0.0,
            WeatherCondition::PartlyCloudy => 0.2,
            WeatherCondition::Overcast => 0.7,
            WeatherCondition::LightRain => 0.6,
            WeatherCondition::HeavyRain => 0.85,
            WeatherCondition::Thunderstorm => 0.95,
            WeatherCondition::LightSnow => 0.5,
            WeatherCondition::HeavySnow => 0.7,
            WeatherCondition::Blizzard => 0.9,
            WeatherCondition::Foggy => 0.4,
            WeatherCondition::DenseFog => 0.7,
            WeatherCondition::Sandstorm => 0.6,
            WeatherCondition::Haze => 0.3,
            WeatherCondition::Windy => 0.0,
        }
    }

    /// Has precipitation
    pub fn has_precipitation(&self) -> bool {
        matches!(
            self,
            WeatherCondition::LightRain
                | WeatherCondition::HeavyRain
                | WeatherCondition::Thunderstorm
                | WeatherCondition::LightSnow
                | WeatherCondition::HeavySnow
                | WeatherCondition::Blizzard
        )
    }
}

// ============================================================================
// SKY TYPE - Sky rendering mode
// ============================================================================

/// Sky rendering type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SkyType {
    #[default]
    Procedural,
    Hdri,
    SolidColor,
    Gradient,
}

impl std::fmt::Display for SkyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SkyType {
    /// All sky types
    pub fn all() -> &'static [SkyType] {
        &[
            SkyType::Procedural,
            SkyType::Hdri,
            SkyType::SolidColor,
            SkyType::Gradient,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            SkyType::Procedural => "Procedural",
            SkyType::Hdri => "HDRI",
            SkyType::SolidColor => "Solid Color",
            SkyType::Gradient => "Gradient",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            SkyType::Procedural => "🌈",
            SkyType::Hdri => "[Img]",
            SkyType::SolidColor => "[B]",
            SkyType::Gradient => "[Art]",
        }
    }

    /// Supports dynamic time of day
    pub fn supports_time_of_day(&self) -> bool {
        matches!(self, SkyType::Procedural)
    }
}

// ============================================================================
// FOG TYPE - Fog calculation mode
// ============================================================================

/// Fog calculation type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FogType {
    None,
    #[default]
    Linear,
    Exponential,
    ExponentialSquared,
    Height,
    Volumetric,
}

impl std::fmt::Display for FogType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl FogType {
    /// All fog types
    pub fn all() -> &'static [FogType] {
        &[
            FogType::None,
            FogType::Linear,
            FogType::Exponential,
            FogType::ExponentialSquared,
            FogType::Height,
            FogType::Volumetric,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            FogType::None => "None",
            FogType::Linear => "Linear",
            FogType::Exponential => "Exponential",
            FogType::ExponentialSquared => "Exp²",
            FogType::Height => "Height",
            FogType::Volumetric => "Volumetric",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            FogType::None => "🚫",
            FogType::Linear => "-",
            FogType::Exponential => "📈",
            FogType::ExponentialSquared => "📈",
            FogType::Height => "[Up]",
            FogType::Volumetric => "[Fog]",
        }
    }

    /// Performance cost (1-5)
    pub fn performance_cost(&self) -> u8 {
        match self {
            FogType::None => 0,
            FogType::Linear => 1,
            FogType::Exponential => 1,
            FogType::ExponentialSquared => 1,
            FogType::Height => 2,
            FogType::Volumetric => 4,
        }
    }
}

// ============================================================================
// TONEMAPPER - Color grading algorithm
// ============================================================================

/// Tonemapping algorithm
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Tonemapper {
    None,
    Reinhard,
    ReinhardExtended,
    #[default]
    Aces,
    AcesNarkowicz,
    Uncharted2,
    Khronos,
    AgX,
}

impl std::fmt::Display for Tonemapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Tonemapper {
    /// All tonemappers
    pub fn all() -> &'static [Tonemapper] {
        &[
            Tonemapper::None,
            Tonemapper::Reinhard,
            Tonemapper::ReinhardExtended,
            Tonemapper::Aces,
            Tonemapper::AcesNarkowicz,
            Tonemapper::Uncharted2,
            Tonemapper::Khronos,
            Tonemapper::AgX,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            Tonemapper::None => "None",
            Tonemapper::Reinhard => "Reinhard",
            Tonemapper::ReinhardExtended => "Reinhard Extended",
            Tonemapper::Aces => "ACES",
            Tonemapper::AcesNarkowicz => "ACES (Narkowicz)",
            Tonemapper::Uncharted2 => "Uncharted 2",
            Tonemapper::Khronos => "Khronos PBR Neutral",
            Tonemapper::AgX => "AgX",
        }
    }

    /// Description
    pub fn description(&self) -> &'static str {
        match self {
            Tonemapper::None => "No tonemapping (raw HDR)",
            Tonemapper::Reinhard => "Simple global tonemapper",
            Tonemapper::ReinhardExtended => "Reinhard with white point",
            Tonemapper::Aces => "Academy standard filmic look",
            Tonemapper::AcesNarkowicz => "ACES approximation (fast)",
            Tonemapper::Uncharted2 => "Filmic look from Uncharted 2",
            Tonemapper::Khronos => "Khronos PBR neutral reference",
            Tonemapper::AgX => "Modern neutral look",
        }
    }

    /// Preserves colors better (good for stylized)
    pub fn is_color_preserving(&self) -> bool {
        matches!(self, Tonemapper::Reinhard | Tonemapper::ReinhardExtended)
    }

    /// Filmic look (good for realistic)
    pub fn is_filmic(&self) -> bool {
        matches!(
            self,
            Tonemapper::Aces | Tonemapper::AcesNarkowicz | Tonemapper::Uncharted2
        )
    }
}

// ============================================================================
// MOOD PRESET - High-level artistic mood
// ============================================================================

/// Artistic mood presets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MoodPreset {
    #[default]
    Neutral,
    Bright,
    Moody,
    Dramatic,
    Horror,
    Cinematic,
    Dreamy,
    Vintage,
    CyberPunk,
    Desert,
    Arctic,
    Tropical,
    Noir,
    Fantasy,
}

impl std::fmt::Display for MoodPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl MoodPreset {
    /// All mood presets
    pub fn all() -> &'static [MoodPreset] {
        &[
            MoodPreset::Neutral,
            MoodPreset::Bright,
            MoodPreset::Moody,
            MoodPreset::Dramatic,
            MoodPreset::Horror,
            MoodPreset::Cinematic,
            MoodPreset::Dreamy,
            MoodPreset::Vintage,
            MoodPreset::CyberPunk,
            MoodPreset::Desert,
            MoodPreset::Arctic,
            MoodPreset::Tropical,
            MoodPreset::Noir,
            MoodPreset::Fantasy,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            MoodPreset::Neutral => "Neutral",
            MoodPreset::Bright => "Bright",
            MoodPreset::Moody => "Moody",
            MoodPreset::Dramatic => "Dramatic",
            MoodPreset::Horror => "Horror",
            MoodPreset::Cinematic => "Cinematic",
            MoodPreset::Dreamy => "Dreamy",
            MoodPreset::Vintage => "Vintage",
            MoodPreset::CyberPunk => "Cyberpunk",
            MoodPreset::Desert => "Desert",
            MoodPreset::Arctic => "Arctic",
            MoodPreset::Tropical => "Tropical",
            MoodPreset::Noir => "Noir",
            MoodPreset::Fantasy => "Fantasy",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            MoodPreset::Neutral => "⚪",
            MoodPreset::Bright => "[Sun]",
            MoodPreset::Moody => "[Rain]",
            MoodPreset::Dramatic => "[Mask]",
            MoodPreset::Horror => "👻",
            MoodPreset::Cinematic => "[Anim]",
            MoodPreset::Dreamy => "💭",
            MoodPreset::Vintage => "📷",
            MoodPreset::CyberPunk => "[Bot]",
            MoodPreset::Desert => "🏜️",
            MoodPreset::Arctic => "❄️",
            MoodPreset::Tropical => "🌴",
            MoodPreset::Noir => "🎩",
            MoodPreset::Fantasy => "[Orb]",
        }
    }

    /// Recommended tonemapper
    pub fn recommended_tonemapper(&self) -> Tonemapper {
        match self {
            MoodPreset::Neutral => Tonemapper::Khronos,
            MoodPreset::Bright => Tonemapper::Reinhard,
            MoodPreset::Moody | MoodPreset::Noir => Tonemapper::Uncharted2,
            MoodPreset::Dramatic | MoodPreset::Cinematic => Tonemapper::Aces,
            MoodPreset::Horror => Tonemapper::Uncharted2,
            MoodPreset::Dreamy => Tonemapper::ReinhardExtended,
            MoodPreset::Vintage => Tonemapper::AgX,
            MoodPreset::CyberPunk => Tonemapper::Aces,
            MoodPreset::Desert | MoodPreset::Tropical => Tonemapper::Aces,
            MoodPreset::Arctic => Tonemapper::Reinhard,
            MoodPreset::Fantasy => Tonemapper::Reinhard,
        }
    }

    /// Contrast adjustment (-1.0 to 1.0)
    pub fn contrast(&self) -> f32 {
        match self {
            MoodPreset::Neutral => 0.0,
            MoodPreset::Bright => -0.1,
            MoodPreset::Moody => 0.2,
            MoodPreset::Dramatic => 0.4,
            MoodPreset::Horror => 0.3,
            MoodPreset::Cinematic => 0.15,
            MoodPreset::Dreamy => -0.2,
            MoodPreset::Vintage => 0.1,
            MoodPreset::CyberPunk => 0.25,
            MoodPreset::Desert => 0.1,
            MoodPreset::Arctic => 0.0,
            MoodPreset::Tropical => 0.1,
            MoodPreset::Noir => 0.5,
            MoodPreset::Fantasy => 0.05,
        }
    }

    /// Saturation adjustment (-1.0 to 1.0)
    pub fn saturation(&self) -> f32 {
        match self {
            MoodPreset::Neutral => 0.0,
            MoodPreset::Bright => 0.15,
            MoodPreset::Moody => -0.2,
            MoodPreset::Dramatic => 0.0,
            MoodPreset::Horror => -0.3,
            MoodPreset::Cinematic => 0.05,
            MoodPreset::Dreamy => 0.1,
            MoodPreset::Vintage => -0.15,
            MoodPreset::CyberPunk => 0.3,
            MoodPreset::Desert => 0.1,
            MoodPreset::Arctic => -0.2,
            MoodPreset::Tropical => 0.25,
            MoodPreset::Noir => -0.8,
            MoodPreset::Fantasy => 0.2,
        }
    }
}

// ============================================================================
// ENVIRONMENT SETTINGS - Complete environment configuration
// ============================================================================

/// Complete environment configuration
#[derive(Debug, Clone)]
pub struct EnvironmentSettings {
    // Time & Weather
    pub time_of_day: TimeOfDay,
    pub weather: WeatherCondition,

    // Sky
    pub sky_type: SkyType,
    pub sky_intensity: f32,

    // Fog
    pub fog_type: FogType,
    pub fog_density: f32,
    pub fog_color: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,

    // Lighting
    pub sun_intensity: f32,
    pub ambient_intensity: f32,

    // Post-processing
    pub tonemapper: Tonemapper,
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub vignette: f32,

    // Mood
    pub mood: MoodPreset,
}

impl Default for EnvironmentSettings {
    fn default() -> Self {
        Self {
            time_of_day: TimeOfDay::default(),
            weather: WeatherCondition::default(),
            sky_type: SkyType::default(),
            sky_intensity: 1.0,
            fog_type: FogType::default(),
            fog_density: 0.001,
            fog_color: [0.8, 0.85, 0.9],
            fog_start: 50.0,
            fog_end: 500.0,
            sun_intensity: 1.0,
            ambient_intensity: 0.3,
            tonemapper: Tonemapper::default(),
            exposure: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            vignette: 0.0,
            mood: MoodPreset::default(),
        }
    }
}

impl EnvironmentSettings {
    /// Create from mood preset
    pub fn from_mood(mood: MoodPreset) -> Self {
        Self {
            mood,
            tonemapper: mood.recommended_tonemapper(),
            contrast: mood.contrast(),
            saturation: mood.saturation(),
            ..Self::default()
        }
    }

    /// Apply time of day settings
    pub fn apply_time(&mut self, time: TimeOfDay) {
        self.time_of_day = time;
        // Adjust lighting based on time
        if time.is_daytime() {
            self.sun_intensity = 1.0 - (1.0 - time.sun_elevation() / 90.0).abs() * 0.3;
        } else {
            self.sun_intensity = 0.0;
            self.ambient_intensity = 0.1;
        }
    }

    /// Apply weather settings
    pub fn apply_weather(&mut self, weather: WeatherCondition) {
        self.weather = weather;
        self.fog_density *= weather.fog_multiplier();
        self.sun_intensity *= 1.0 - weather.sun_attenuation();
    }
}

// ============================================================================
// ENVIRONMENT PRESET PANEL
// ============================================================================

/// Environment presets panel
#[derive(Debug)]
pub struct EnvironmentPresetPanel {
    pub settings: EnvironmentSettings,
    pub show_advanced: bool,
    pub preview_enabled: bool,
    pub transition_duration: f32,
    pub saved_presets: Vec<(String, EnvironmentSettings)>,
    pub new_preset_name: String,
    /// Actions queued for external processing
    pending_actions: Vec<EnvironmentAction>,
}

impl Default for EnvironmentPresetPanel {
    fn default() -> Self {
        Self {
            settings: EnvironmentSettings::default(),
            show_advanced: false,
            preview_enabled: true,
            transition_duration: 2.0,
            saved_presets: Vec::new(),
            new_preset_name: String::new(),
            pending_actions: Vec::new(),
        }
    }
}

impl EnvironmentPresetPanel {
    /// Create new environment preset panel
    pub fn new() -> Self {
        Self::default()
    }

    /// Take all pending actions (drains the queue)
    pub fn take_actions(&mut self) -> Vec<EnvironmentAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for later processing
    fn queue_action(&mut self, action: EnvironmentAction) {
        self.pending_actions.push(action);
    }

    /// Get current environment settings
    pub fn current_settings(&self) -> &EnvironmentSettings {
        &self.settings
    }

    fn render_quick_presets(&mut self, ui: &mut Ui) {
        ui.heading("[Time] Quick Time Presets");

        ui.horizontal_wrapped(|ui| {
            for time in TimeOfDay::all() {
                let selected = self.settings.time_of_day == *time;
                if ui.selectable_label(selected, format!("{}", time)).clicked() {
                    info!(time = time.name(), "environment: time-of-day changed");
                    self.settings.apply_time(*time);
                }
            }
        });

        ui.add_space(8.0);
        ui.heading("[Clr] Weather");

        ui.horizontal_wrapped(|ui| {
            for weather in &[
                WeatherCondition::Clear,
                WeatherCondition::PartlyCloudy,
                WeatherCondition::Overcast,
                WeatherCondition::LightRain,
                WeatherCondition::Foggy,
            ] {
                let selected = self.settings.weather == *weather;
                if ui
                    .selectable_label(selected, format!("{}", weather))
                    .clicked()
                {
                    self.settings.apply_weather(*weather);
                }
            }
        });

        ui.add_space(8.0);
        ui.heading("Mood Presets");

        ui.horizontal_wrapped(|ui| {
            for mood in MoodPreset::all() {
                let selected = self.settings.mood == *mood;
                if ui.selectable_label(selected, format!("{}", mood)).clicked() {
                    self.settings = EnvironmentSettings::from_mood(*mood);
                }
            }
        });
    }

    fn render_sky_settings(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("[Cld] Sky")
            .default_open(true)
            .show(ui, |ui| {
                egui::ComboBox::from_label("Sky Type")
                    .selected_text(format!("{}", self.settings.sky_type))
                    .show_ui(ui, |ui| {
                        for sky in SkyType::all() {
                            if ui
                                .selectable_label(
                                    self.settings.sky_type == *sky,
                                    format!("{}", sky),
                                )
                                .clicked()
                            {
                                self.settings.sky_type = *sky;
                            }
                        }
                    });

                ui.add(
                    egui::Slider::new(&mut self.settings.sky_intensity, 0.0..=2.0)
                        .text("Intensity"),
                );
            });
    }

    fn render_fog_settings(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("[Fog] Fog")
            .default_open(true)
            .show(ui, |ui| {
                egui::ComboBox::from_label("Fog Type")
                    .selected_text(format!("{}", self.settings.fog_type))
                    .show_ui(ui, |ui| {
                        for fog in FogType::all() {
                            if ui
                                .selectable_label(
                                    self.settings.fog_type == *fog,
                                    format!("{}", fog),
                                )
                                .clicked()
                            {
                                self.settings.fog_type = *fog;
                            }
                        }
                    });

                if self.settings.fog_type != FogType::None {
                    ui.add(
                        egui::Slider::new(&mut self.settings.fog_density, 0.0001..=0.1)
                            .logarithmic(true)
                            .text("Density"),
                    );

                    ui.horizontal(|ui| {
                        ui.label("Start:");
                        ui.add(egui::DragValue::new(&mut self.settings.fog_start).speed(1.0));
                        ui.label("End:");
                        ui.add(egui::DragValue::new(&mut self.settings.fog_end).speed(1.0));
                    });
                }
            });
    }

    fn render_lighting_settings(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("Lighting")
            .default_open(true)
            .show(ui, |ui| {
                ui.add(
                    egui::Slider::new(&mut self.settings.sun_intensity, 0.0..=2.0)
                        .text("Sun Intensity"),
                );

                ui.add(
                    egui::Slider::new(&mut self.settings.ambient_intensity, 0.0..=1.0)
                        .text("Ambient"),
                );
            });
    }

    fn render_post_process_settings(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new("Post-Processing")
            .default_open(true)
            .show(ui, |ui| {
                egui::ComboBox::from_label("Tonemapper")
                    .selected_text(format!("{}", self.settings.tonemapper))
                    .show_ui(ui, |ui| {
                        for tm in Tonemapper::all() {
                            if ui
                                .selectable_label(
                                    self.settings.tonemapper == *tm,
                                    format!("{}", tm),
                                )
                                .clicked()
                            {
                                self.settings.tonemapper = *tm;
                            }
                        }
                    });

                ui.add(egui::Slider::new(&mut self.settings.exposure, -3.0..=3.0).text("Exposure"));

                ui.add(egui::Slider::new(&mut self.settings.contrast, -1.0..=1.0).text("Contrast"));

                ui.add(
                    egui::Slider::new(&mut self.settings.saturation, -1.0..=1.0).text("Saturation"),
                );

                ui.add(egui::Slider::new(&mut self.settings.vignette, 0.0..=1.0).text("Vignette"));
            });
    }

    fn render_preset_management(&mut self, ui: &mut Ui) {
        ui.separator();

        // Flags for deferred actions
        let mut should_save_preset = false;
        let mut load_preset_name: Option<String> = None;
        let preset_name_to_save = self.new_preset_name.clone();

        ui.horizontal(|ui| {
            ui.label("Save Preset:");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_preset_name)
                    .hint_text("Preset name...")
                    .desired_width(120.0),
            );

            if ui.button("Save").clicked() && !self.new_preset_name.is_empty() {
                should_save_preset = true;
            }
        });

        if !self.saved_presets.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for (name, _settings) in self.saved_presets.iter() {
                    if ui.small_button(name).clicked() {
                        load_preset_name = Some(name.clone());
                    }
                }
            });
        }

        // Apply deferred actions
        if should_save_preset {
            let settings = self.settings.clone();
            self.saved_presets
                .push((preset_name_to_save.clone(), settings.clone()));
            self.queue_action(EnvironmentAction::SavePreset {
                name: preset_name_to_save,
                settings,
            });
            self.new_preset_name.clear();
        }

        if let Some(name) = load_preset_name {
            // Find and apply the preset locally
            if let Some((_, settings)) = self.saved_presets.iter().find(|(n, _)| n == &name) {
                self.settings = settings.clone();
            }
            self.queue_action(EnvironmentAction::LoadPreset { name });
        }
    }
}

impl Panel for EnvironmentPresetPanel {
    fn name(&self) -> &'static str {
        "Environment Presets"
    }

    fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.preview_enabled, "Live Preview");
            ui.separator();
            ui.add(
                egui::Slider::new(&mut self.transition_duration, 0.0..=10.0).text("Transition (s)"),
            );
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.render_quick_presets(ui);
            ui.separator();

            ui.horizontal(|ui| {
                ui.heading("Settings");
                ui.toggle_value(&mut self.show_advanced, "Advanced");
            });

            self.render_sky_settings(ui);
            self.render_fog_settings(ui);
            self.render_lighting_settings(ui);
            self.render_post_process_settings(ui);
            self.render_preset_management(ui);
        });

        ui.separator();

        // Flags for deferred actions (cannot mutate self inside closures)
        let mut should_apply = false;
        let mut should_reset = false;

        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("Apply").color(Color32::from_rgb(100, 200, 100)))
                .clicked()
            {
                should_apply = true;
            }

            if ui.button("Reset").clicked() {
                should_reset = true;
            }
        });

        // Apply actions outside UI context
        if should_apply {
            info!("environment: settings applied");
            self.queue_action(EnvironmentAction::ApplySettings {
                settings: self.settings.clone(),
            });
        }
        if should_reset {
            info!("environment: reset to defaults");
            self.settings = EnvironmentSettings::default();
            self.queue_action(EnvironmentAction::ResetToDefault);
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_of_day_display() {
        assert!(format!("{}", TimeOfDay::Noon).contains("Noon"));
        assert!(format!("{}", TimeOfDay::Sunset).contains("Sunset"));
    }

    #[test]
    fn test_time_of_day_all() {
        let times = TimeOfDay::all();
        assert_eq!(times.len(), 12);
    }

    #[test]
    fn test_time_of_day_daytime() {
        assert!(TimeOfDay::Noon.is_daytime());
        assert!(TimeOfDay::Morning.is_daytime());
        assert!(!TimeOfDay::Night.is_daytime());
        assert!(!TimeOfDay::Midnight.is_daytime());
    }

    #[test]
    fn test_weather_display() {
        assert!(format!("{}", WeatherCondition::Clear).contains("Clear"));
        assert!(format!("{}", WeatherCondition::Thunderstorm).contains("Thunderstorm"));
    }

    #[test]
    fn test_weather_all() {
        let conditions = WeatherCondition::all();
        assert_eq!(conditions.len(), 14);
    }

    #[test]
    fn test_weather_precipitation() {
        assert!(!WeatherCondition::Clear.has_precipitation());
        assert!(WeatherCondition::LightRain.has_precipitation());
        assert!(WeatherCondition::HeavySnow.has_precipitation());
    }

    #[test]
    fn test_sky_type_display() {
        assert!(format!("{}", SkyType::Procedural).contains("Procedural"));
        assert!(format!("{}", SkyType::Hdri).contains("HDRI"));
    }

    #[test]
    fn test_sky_type_time_of_day_support() {
        assert!(SkyType::Procedural.supports_time_of_day());
        assert!(!SkyType::Hdri.supports_time_of_day());
    }

    #[test]
    fn test_fog_type_display() {
        assert!(format!("{}", FogType::Linear).contains("Linear"));
        assert!(format!("{}", FogType::Volumetric).contains("Volumetric"));
    }

    #[test]
    fn test_fog_type_performance() {
        assert!(FogType::None.performance_cost() < FogType::Volumetric.performance_cost());
        assert!(FogType::Linear.performance_cost() < FogType::Height.performance_cost());
    }

    #[test]
    fn test_tonemapper_display() {
        assert_eq!(Tonemapper::Aces.name(), "ACES");
        assert_eq!(Tonemapper::Reinhard.name(), "Reinhard");
    }

    #[test]
    fn test_tonemapper_filmic() {
        assert!(Tonemapper::Aces.is_filmic());
        assert!(Tonemapper::Uncharted2.is_filmic());
        assert!(!Tonemapper::Reinhard.is_filmic());
    }

    #[test]
    fn test_mood_preset_display() {
        assert!(format!("{}", MoodPreset::Horror).contains("Horror"));
        assert!(format!("{}", MoodPreset::CyberPunk).contains("Cyberpunk"));
    }

    #[test]
    fn test_mood_preset_all() {
        let moods = MoodPreset::all();
        assert_eq!(moods.len(), 14);
    }

    #[test]
    fn test_mood_preset_tonemapper() {
        assert_eq!(
            MoodPreset::Cinematic.recommended_tonemapper(),
            Tonemapper::Aces
        );
        assert_eq!(
            MoodPreset::Neutral.recommended_tonemapper(),
            Tonemapper::Khronos
        );
    }

    #[test]
    fn test_environment_settings_default() {
        let settings = EnvironmentSettings::default();
        assert_eq!(settings.time_of_day, TimeOfDay::Noon);
        assert_eq!(settings.weather, WeatherCondition::Clear);
    }

    #[test]
    fn test_environment_settings_from_mood() {
        let settings = EnvironmentSettings::from_mood(MoodPreset::Horror);
        assert_eq!(settings.mood, MoodPreset::Horror);
        assert!(settings.contrast > 0.0); // Horror has high contrast
        assert!(settings.saturation < 0.0); // Horror is desaturated
    }

    #[test]
    fn test_environment_preset_panel_default() {
        let panel = EnvironmentPresetPanel::new();
        assert!(panel.preview_enabled);
        assert_eq!(panel.transition_duration, 2.0);
    }

    // ==========================================================================
    // Action System Tests
    // ==========================================================================

    #[test]
    fn test_action_queue_initially_empty() {
        let panel = EnvironmentPresetPanel::new();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_take_actions_drains_queue() {
        let mut panel = EnvironmentPresetPanel::new();
        panel
            .pending_actions
            .push(EnvironmentAction::ResetToDefault);
        panel.pending_actions.push(EnvironmentAction::StartPreview);

        assert!(panel.has_pending_actions());
        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_environment_action_variants() {
        let apply_action = EnvironmentAction::ApplySettings {
            settings: EnvironmentSettings::default(),
        };
        assert!(matches!(
            apply_action,
            EnvironmentAction::ApplySettings { .. }
        ));

        let save_action = EnvironmentAction::SavePreset {
            name: "My Sunset".into(),
            settings: EnvironmentSettings::default(),
        };
        assert!(matches!(save_action, EnvironmentAction::SavePreset { .. }));

        let load_action = EnvironmentAction::LoadPreset {
            name: "My Sunset".into(),
        };
        assert!(matches!(load_action, EnvironmentAction::LoadPreset { .. }));

        let quick_action = EnvironmentAction::ApplyQuickPreset {
            time: TimeOfDay::Sunset,
            weather: WeatherCondition::Clear,
        };
        assert!(matches!(
            quick_action,
            EnvironmentAction::ApplyQuickPreset { .. }
        ));
    }

    #[test]
    fn test_current_settings_accessor() {
        let mut panel = EnvironmentPresetPanel::new();
        panel.settings.time_of_day = TimeOfDay::Midnight;
        panel.settings.fog_density = 0.8;

        let settings = panel.current_settings();
        assert_eq!(settings.time_of_day, TimeOfDay::Midnight);
        assert!((settings.fog_density - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_saved_presets_management() {
        let mut panel = EnvironmentPresetPanel::new();
        assert!(panel.saved_presets.is_empty());

        // Simulate saving a preset
        let settings = EnvironmentSettings::default();
        panel
            .saved_presets
            .push(("Test Preset".into(), settings.clone()));

        assert_eq!(panel.saved_presets.len(), 1);
        assert_eq!(panel.saved_presets[0].0, "Test Preset");
    }
}
