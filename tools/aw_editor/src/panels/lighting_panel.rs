//! Lighting Panel for the editor UI
//!
//! Provides comprehensive lighting configuration:
//! - Light types (directional, point, spot, area)
//! - Global illumination settings
//! - Light probes and reflection probes
//! - Environment lighting and skybox
//! - Shadow configuration

#![allow(clippy::upper_case_acronyms)] // PCSS is industry-standard acronym

use egui::{Color32, RichText, Ui};
use tracing::info;

use crate::panels::Panel;

/// Light type enumeration
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LightType {
    #[default]
    Directional,
    Point,
    Spot,
    Area,
    Ambient,
}

impl std::fmt::Display for LightType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LightType {
    pub fn all() -> &'static [LightType] {
        &[
            LightType::Directional,
            LightType::Point,
            LightType::Spot,
            LightType::Area,
            LightType::Ambient,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LightType::Directional => "Directional",
            LightType::Point => "Point",
            LightType::Spot => "Spot",
            LightType::Area => "Area",
            LightType::Ambient => "Ambient",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LightType::Directional => "[Sun]",
            LightType::Point => "[Lgt]",
            LightType::Spot => "🔦",
            LightType::Area => "⬜",
            LightType::Ambient => "[Net]",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            LightType::Directional => "Sun-like light from infinitely far away",
            LightType::Point => "Omnidirectional light source",
            LightType::Spot => "Cone-shaped focused light",
            LightType::Area => "Light emitted from a surface",
            LightType::Ambient => "Global ambient illumination",
        }
    }

    pub fn is_directional(&self) -> bool {
        matches!(self, LightType::Directional)
    }

    pub fn has_range(&self) -> bool {
        matches!(self, LightType::Point | LightType::Spot)
    }
}

/// Shadow quality preset
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShadowQuality {
    Off,
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

impl std::fmt::Display for ShadowQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ShadowQuality {
    pub fn all() -> &'static [ShadowQuality] {
        &[
            ShadowQuality::Off,
            ShadowQuality::Low,
            ShadowQuality::Medium,
            ShadowQuality::High,
            ShadowQuality::Ultra,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ShadowQuality::Off => "Off",
            ShadowQuality::Low => "Low",
            ShadowQuality::Medium => "Medium",
            ShadowQuality::High => "High",
            ShadowQuality::Ultra => "Ultra",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ShadowQuality::Off => "⚫",
            ShadowQuality::Low => "🔅",
            ShadowQuality::Medium => "🔆",
            ShadowQuality::High => "[Gem]",
            ShadowQuality::Ultra => "[Fx]",
        }
    }

    pub fn resolution(&self) -> u32 {
        match self {
            ShadowQuality::Off => 0,
            ShadowQuality::Low => 512,
            ShadowQuality::Medium => 1024,
            ShadowQuality::High => 2048,
            ShadowQuality::Ultra => 4096,
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, ShadowQuality::Off)
    }
}

/// Shadow type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShadowType {
    None,
    #[default]
    Hard,
    Soft,
    PCSS,
}

impl std::fmt::Display for ShadowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ShadowType {
    pub fn all() -> &'static [ShadowType] {
        &[
            ShadowType::None,
            ShadowType::Hard,
            ShadowType::Soft,
            ShadowType::PCSS,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ShadowType::None => "None",
            ShadowType::Hard => "Hard",
            ShadowType::Soft => "Soft",
            ShadowType::PCSS => "PCSS",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ShadowType::None => "⚫",
            ShadowType::Hard => "🟥",
            ShadowType::Soft => "🟧",
            ShadowType::PCSS => "[G]",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ShadowType::None => "No shadows",
            ShadowType::Hard => "Sharp shadow edges",
            ShadowType::Soft => "Blurred shadow edges",
            ShadowType::PCSS => "Contact-hardening soft shadows",
        }
    }

    pub fn is_soft(&self) -> bool {
        matches!(self, ShadowType::Soft | ShadowType::PCSS)
    }
}

/// Light unit
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LightUnit {
    #[default]
    Unitless,
    Lumen,
    Candela,
    Lux,
    Nit,
}

impl std::fmt::Display for LightUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LightUnit {
    pub fn all() -> &'static [LightUnit] {
        &[
            LightUnit::Unitless,
            LightUnit::Lumen,
            LightUnit::Candela,
            LightUnit::Lux,
            LightUnit::Nit,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LightUnit::Unitless => "Unitless",
            LightUnit::Lumen => "Lumen",
            LightUnit::Candela => "Candela",
            LightUnit::Lux => "Lux",
            LightUnit::Nit => "Nit",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LightUnit::Unitless => "[Chart]",
            LightUnit::Lumen => "[Lgt]",
            LightUnit::Candela => "🕯️",
            LightUnit::Lux => "[Sun]",
            LightUnit::Nit => "📺",
        }
    }

    pub fn abbreviation(&self) -> &'static str {
        match self {
            LightUnit::Unitless => "",
            LightUnit::Lumen => "lm",
            LightUnit::Candela => "cd",
            LightUnit::Lux => "lx",
            LightUnit::Nit => "nt",
        }
    }

    pub fn is_physical(&self) -> bool {
        !matches!(self, LightUnit::Unitless)
    }
}

/// Light definition
#[derive(Debug, Clone)]
pub struct Light {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub light_type: LightType,

    // Transform
    pub position: [f32; 3],
    pub rotation: [f32; 3],

    // Properties
    pub color: [f32; 3],
    pub intensity: f32,
    pub unit: LightUnit,
    pub temperature: f32,
    pub use_temperature: bool,

    // Type-specific
    pub range: f32,
    pub spot_angle: f32,
    pub inner_spot_angle: f32,
    pub area_width: f32,
    pub area_height: f32,

    // Shadows
    pub cast_shadows: bool,
    pub shadow_type: ShadowType,
    pub shadow_resolution: u32,
    pub shadow_bias: f32,
    pub shadow_normal_bias: f32,
    pub shadow_near_plane: f32,

    // Advanced
    pub indirect_multiplier: f32,
    pub volumetric: bool,
    pub volumetric_strength: f32,
    pub cookie_path: String,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Light".to_string(),
            enabled: true,
            light_type: LightType::Point,

            position: [0.0, 5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],

            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            unit: LightUnit::Unitless,
            temperature: 6500.0,
            use_temperature: false,

            range: 10.0,
            spot_angle: 45.0,
            inner_spot_angle: 30.0,
            area_width: 1.0,
            area_height: 1.0,

            cast_shadows: true,
            shadow_type: ShadowType::Soft,
            shadow_resolution: 1024,
            shadow_bias: 0.05,
            shadow_normal_bias: 0.4,
            shadow_near_plane: 0.2,

            indirect_multiplier: 1.0,
            volumetric: false,
            volumetric_strength: 1.0,
            cookie_path: String::new(),
        }
    }
}

/// Global illumination mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GiMode {
    #[default]
    None,
    BakedLightmaps,
    RealtimeGI,
    Hybrid,
}

impl std::fmt::Display for GiMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl GiMode {
    pub fn all() -> &'static [GiMode] {
        &[
            GiMode::None,
            GiMode::BakedLightmaps,
            GiMode::RealtimeGI,
            GiMode::Hybrid,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            GiMode::None => "None",
            GiMode::BakedLightmaps => "Baked Lightmaps",
            GiMode::RealtimeGI => "Realtime GI",
            GiMode::Hybrid => "Hybrid",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            GiMode::None => "⚫",
            GiMode::BakedLightmaps => "🗺️",
            GiMode::RealtimeGI => "[Zap]",
            GiMode::Hybrid => "[Shuf]",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GiMode::None => "No global illumination",
            GiMode::BakedLightmaps => "Pre-computed lightmaps",
            GiMode::RealtimeGI => "Dynamic real-time GI",
            GiMode::Hybrid => "Baked + realtime combined",
        }
    }

    pub fn is_realtime(&self) -> bool {
        matches!(self, GiMode::RealtimeGI | GiMode::Hybrid)
    }

    pub fn requires_baking(&self) -> bool {
        matches!(self, GiMode::BakedLightmaps | GiMode::Hybrid)
    }
}

/// Global illumination settings
#[derive(Debug, Clone)]
pub struct GiSettings {
    pub mode: GiMode,

    // Lightmap settings
    pub lightmap_resolution: u32,
    pub indirect_intensity: f32,
    pub ambient_occlusion: bool,
    pub ao_intensity: f32,
    pub ao_max_distance: f32,

    // Realtime GI
    pub probe_spacing: f32,
    pub probe_update_rate: f32,
    pub cascade_count: u32,

    // Quality
    pub bounce_count: u32,
    pub samples_per_texel: u32,
}

impl Default for GiSettings {
    fn default() -> Self {
        Self {
            mode: GiMode::None,

            lightmap_resolution: 40,
            indirect_intensity: 1.0,
            ambient_occlusion: true,
            ao_intensity: 1.0,
            ao_max_distance: 1.0,

            probe_spacing: 1.0,
            probe_update_rate: 0.5,
            cascade_count: 4,

            bounce_count: 2,
            samples_per_texel: 16,
        }
    }
}

/// Light probe definition
#[derive(Debug, Clone)]
pub struct LightProbe {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub position: [f32; 3],
    pub blend_distance: f32,
    pub importance: f32,
    pub baked: bool,
}

impl Default for LightProbe {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Light Probe".to_string(),
            enabled: true,
            position: [0.0, 1.0, 0.0],
            blend_distance: 1.0,
            importance: 1.0,
            baked: false,
        }
    }
}

/// Reflection probe definition
#[derive(Debug, Clone)]
pub struct ReflectionProbe {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub position: [f32; 3],
    pub box_size: [f32; 3],
    pub resolution: u32,
    pub hdr: bool,
    pub realtime: bool,
    pub refresh_mode: RefreshMode,
    pub importance: f32,
    pub blend_distance: f32,
    pub box_projection: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RefreshMode {
    #[default]
    OnAwake,
    EveryFrame,
    ViaScript,
}

impl std::fmt::Display for RefreshMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl RefreshMode {
    /// Returns the human-readable name for this refresh mode
    pub fn name(&self) -> &'static str {
        match self {
            Self::OnAwake => "On Awake",
            Self::EveryFrame => "Every Frame",
            Self::ViaScript => "Via Script",
        }
    }

    /// Returns the icon for this refresh mode
    pub fn icon(&self) -> &'static str {
        match self {
            Self::OnAwake => "[Sync]",
            Self::EveryFrame => "[Zap]",
            Self::ViaScript => "📜",
        }
    }

    /// Returns all available refresh modes
    pub fn all() -> &'static [RefreshMode] {
        &[
            RefreshMode::OnAwake,
            RefreshMode::EveryFrame,
            RefreshMode::ViaScript,
        ]
    }

    /// Returns true if this mode is automatic (not script-controlled)
    pub fn is_automatic(&self) -> bool {
        !matches!(self, Self::ViaScript)
    }
}

impl Default for ReflectionProbe {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Reflection Probe".to_string(),
            enabled: true,
            position: [0.0, 2.0, 0.0],
            box_size: [10.0, 10.0, 10.0],
            resolution: 256,
            hdr: true,
            realtime: false,
            refresh_mode: RefreshMode::OnAwake,
            importance: 1.0,
            blend_distance: 1.0,
            box_projection: false,
        }
    }
}

/// Environment settings
#[derive(Debug, Clone)]
pub struct EnvironmentSettings {
    // Skybox
    pub skybox_enabled: bool,
    pub skybox_path: String,
    pub skybox_tint: [f32; 3],
    pub skybox_exposure: f32,
    pub skybox_rotation: f32,

    // Ambient
    pub ambient_mode: AmbientMode,
    pub ambient_color: [f32; 3],
    pub ambient_sky_color: [f32; 3],
    pub ambient_equator_color: [f32; 3],
    pub ambient_ground_color: [f32; 3],
    pub ambient_intensity: f32,

    // Fog
    pub fog_enabled: bool,
    pub fog_mode: FogMode,
    pub fog_color: [f32; 3],
    pub fog_density: f32,
    pub fog_start: f32,
    pub fog_end: f32,
    pub fog_height: f32,
    pub fog_height_density: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AmbientMode {
    #[default]
    Skybox,
    Color,
    Gradient,
}

impl std::fmt::Display for AmbientMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AmbientMode {
    /// Returns the human-readable name for this ambient mode
    pub fn name(&self) -> &'static str {
        match self {
            Self::Skybox => "Skybox",
            Self::Color => "Color",
            Self::Gradient => "Gradient",
        }
    }

    /// Returns the icon for this ambient mode
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Skybox => "[Dawn]",
            Self::Color => "[Art]",
            Self::Gradient => "🌈",
        }
    }

    /// Returns all available ambient modes
    pub fn all() -> &'static [AmbientMode] {
        &[
            AmbientMode::Skybox,
            AmbientMode::Color,
            AmbientMode::Gradient,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FogMode {
    #[default]
    Linear,
    Exponential,
    ExponentialSquared,
    Height,
}

impl std::fmt::Display for FogMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl FogMode {
    /// Returns the human-readable name for this fog mode
    pub fn name(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Exponential => "Exponential",
            Self::ExponentialSquared => "Exponential Squared",
            Self::Height => "Height",
        }
    }

    /// Returns the icon for this fog mode
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Linear => "📏",
            Self::Exponential => "📈",
            Self::ExponentialSquared => "[Chart]",
            Self::Height => "⛰️",
        }
    }

    /// Returns all available fog modes
    pub fn all() -> &'static [FogMode] {
        &[
            FogMode::Linear,
            FogMode::Exponential,
            FogMode::ExponentialSquared,
            FogMode::Height,
        ]
    }

    /// Returns true if this mode uses exponential falloff
    pub fn is_exponential(&self) -> bool {
        matches!(self, Self::Exponential | Self::ExponentialSquared)
    }
}

impl Default for EnvironmentSettings {
    fn default() -> Self {
        Self {
            skybox_enabled: true,
            skybox_path: String::new(),
            skybox_tint: [1.0, 1.0, 1.0],
            skybox_exposure: 1.0,
            skybox_rotation: 0.0,

            ambient_mode: AmbientMode::Skybox,
            ambient_color: [0.2, 0.2, 0.2],
            ambient_sky_color: [0.6, 0.7, 0.9],
            ambient_equator_color: [0.4, 0.4, 0.4],
            ambient_ground_color: [0.2, 0.15, 0.1],
            ambient_intensity: 1.0,

            fog_enabled: false,
            fog_mode: FogMode::Linear,
            fog_color: [0.5, 0.5, 0.5],
            fog_density: 0.01,
            fog_start: 0.0,
            fog_end: 300.0,
            fog_height: 0.0,
            fog_height_density: 0.5,
        }
    }
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LightingTab {
    #[default]
    Lights,
    Shadows,
    GI,
    Probes,
    Environment,
    Debug,
}

/// Actions that can be performed by the lighting panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LightingAction {
    // Light management
    AddLight(LightType),
    RemoveLight(u32),
    SelectLight(u32),
    DuplicateLight(u32),

    // Light properties
    SetLightEnabled { id: u32, enabled: bool },
    SetLightColor { id: u32, color: [f32; 3] },
    SetLightIntensity { id: u32, intensity: f32 },
    SetLightRange { id: u32, range: f32 },
    SetLightPosition { id: u32, position: [f32; 3] },
    SetLightRotation { id: u32, rotation: [f32; 3] },
    SetSpotAngle { id: u32, angle: f32 },
    SetAreaSize { id: u32, width: f32, height: f32 },
    ToggleShadows { id: u32, enabled: bool },
    SetShadowResolution { id: u32, resolution: u32 },
    SetShadowBias { id: u32, bias: f32 },

    // Global shadow settings
    SetShadowQuality(ShadowQuality),
    SetShadowDistance(f32),
    SetShadowCascades(u32),

    // GI settings
    SetGiMode(GiMode),
    SetIndirectIntensity(f32),
    ToggleAmbientOcclusion(bool),
    SetAoIntensity(f32),
    BakeLightmaps,
    ClearLightmaps,

    // Probes
    AddLightProbe([f32; 3]),
    AddReflectionProbe([f32; 3]),
    RemoveLightProbe(u32),
    RemoveReflectionProbe(u32),
    SelectProbe(u32),
    BakeProbes,
    BakeSelectedProbe(u32),

    // Environment
    SetSkyboxPath(String),
    SetSkyboxRotation(f32),
    SetAmbientColor([f32; 3]),
    SetAmbientIntensity(f32),
    SetFogEnabled(bool),
    SetFogColor([f32; 3]),
    SetFogDensity(f32),
    SetFogStart(f32),
    SetFogEnd(f32),

    // Debug
    ToggleDebugShadows(bool),
    ToggleDebugLightmaps(bool),
    ToggleDebugProbes(bool),
}

impl std::fmt::Display for LightingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddLight(lt) => write!(f, "Add {} light", lt.name()),
            Self::RemoveLight(id) => write!(f, "Remove light {}", id),
            Self::SelectLight(id) => write!(f, "Select light {}", id),
            Self::DuplicateLight(id) => write!(f, "Duplicate light {}", id),
            Self::SetLightEnabled { id, enabled } => {
                write!(
                    f,
                    "{} light {}",
                    if *enabled { "Enable" } else { "Disable" },
                    id
                )
            }
            Self::SetLightColor { id, .. } => write!(f, "Set light {} color", id),
            Self::SetLightIntensity { id, intensity } => {
                write!(f, "Set light {} intensity to {:.1}", id, intensity)
            }
            Self::SetLightRange { id, range } => {
                write!(f, "Set light {} range to {:.1}", id, range)
            }
            Self::SetLightPosition { id, .. } => write!(f, "Move light {}", id),
            Self::SetLightRotation { id, .. } => write!(f, "Rotate light {}", id),
            Self::SetSpotAngle { id, angle } => write!(f, "Set spot {} angle to {:.1}°", id, angle),
            Self::SetAreaSize { id, width, height } => {
                write!(f, "Set area {} size to {:.1}x{:.1}", id, width, height)
            }
            Self::ToggleShadows { id, enabled } => {
                write!(
                    f,
                    "{} shadows for light {}",
                    if *enabled { "Enable" } else { "Disable" },
                    id
                )
            }
            Self::SetShadowResolution { id, resolution } => {
                write!(f, "Set light {} shadow res to {}", id, resolution)
            }
            Self::SetShadowBias { id, bias } => {
                write!(f, "Set light {} shadow bias to {:.3}", id, bias)
            }
            Self::SetShadowQuality(q) => write!(f, "Set shadow quality to {}", q),
            Self::SetShadowDistance(d) => write!(f, "Set shadow distance to {:.1}", d),
            Self::SetShadowCascades(n) => write!(f, "Set shadow cascades to {}", n),
            Self::SetGiMode(mode) => write!(f, "Set GI mode to {}", mode),
            Self::SetIndirectIntensity(i) => write!(f, "Set indirect intensity to {:.2}", i),
            Self::ToggleAmbientOcclusion(on) => {
                write!(f, "Turn AO {}", if *on { "on" } else { "off" })
            }
            Self::SetAoIntensity(i) => write!(f, "Set AO intensity to {:.2}", i),
            Self::BakeLightmaps => write!(f, "Bake lightmaps"),
            Self::ClearLightmaps => write!(f, "Clear lightmaps"),
            Self::AddLightProbe(pos) => write!(
                f,
                "Add light probe at ({:.1}, {:.1}, {:.1})",
                pos[0], pos[1], pos[2]
            ),
            Self::AddReflectionProbe(pos) => write!(
                f,
                "Add reflection probe at ({:.1}, {:.1}, {:.1})",
                pos[0], pos[1], pos[2]
            ),
            Self::RemoveLightProbe(id) => write!(f, "Remove light probe {}", id),
            Self::RemoveReflectionProbe(id) => write!(f, "Remove reflection probe {}", id),
            Self::SelectProbe(id) => write!(f, "Select probe {}", id),
            Self::BakeProbes => write!(f, "Bake all probes"),
            Self::BakeSelectedProbe(id) => write!(f, "Bake probe {}", id),
            Self::SetSkyboxPath(path) => write!(f, "Set skybox to '{}'", path),
            Self::SetSkyboxRotation(r) => write!(f, "Set skybox rotation to {:.1}°", r),
            Self::SetAmbientColor(_) => write!(f, "Set ambient color"),
            Self::SetAmbientIntensity(i) => write!(f, "Set ambient intensity to {:.2}", i),
            Self::SetFogEnabled(on) => write!(f, "Turn fog {}", if *on { "on" } else { "off" }),
            Self::SetFogColor(_) => write!(f, "Set fog color"),
            Self::SetFogDensity(d) => write!(f, "Set fog density to {:.3}", d),
            Self::SetFogStart(s) => write!(f, "Set fog start to {:.1}", s),
            Self::SetFogEnd(e) => write!(f, "Set fog end to {:.1}", e),
            Self::ToggleDebugShadows(on) => {
                write!(f, "Turn shadow debug {}", if *on { "on" } else { "off" })
            }
            Self::ToggleDebugLightmaps(on) => {
                write!(f, "Turn lightmap debug {}", if *on { "on" } else { "off" })
            }
            Self::ToggleDebugProbes(on) => {
                write!(f, "Turn probe debug {}", if *on { "on" } else { "off" })
            }
        }
    }
}

impl std::fmt::Display for LightingTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LightingTab {
    /// Returns the human-readable name for this tab
    pub fn name(&self) -> &'static str {
        match self {
            Self::Lights => "Lights",
            Self::Shadows => "Shadows",
            Self::GI => "GI",
            Self::Probes => "Probes",
            Self::Environment => "Environment",
            Self::Debug => "Debug",
        }
    }

    /// Returns the icon for this tab
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Lights => "[Lgt]",
            Self::Shadows => "[Shd]",
            Self::GI => "[GI]",
            Self::Probes => "[Orb]",
            Self::Environment => "[Glb]",
            Self::Debug => "[Bug]",
        }
    }

    /// Returns all available lighting tabs
    pub fn all() -> &'static [LightingTab] {
        &[
            LightingTab::Lights,
            LightingTab::Shadows,
            LightingTab::GI,
            LightingTab::Probes,
            LightingTab::Environment,
            LightingTab::Debug,
        ]
    }
}

/// Main Lighting Panel
pub struct LightingPanel {
    // Tab state
    active_tab: LightingTab,

    // Lights
    lights: Vec<Light>,
    selected_light: Option<u32>,
    current_light: Light,

    // Probes
    light_probes: Vec<LightProbe>,
    reflection_probes: Vec<ReflectionProbe>,
    selected_probe: Option<u32>,

    // Settings
    gi_settings: GiSettings,
    environment: EnvironmentSettings,
    shadow_quality: ShadowQuality,

    // Debug
    debug_shadows: bool,
    debug_lightmaps: bool,
    debug_probes: bool,

    // ID counters
    next_light_id: u32,
    next_probe_id: u32,

    // Action queue for external processing
    pending_actions: Vec<LightingAction>,
}

impl Default for LightingPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: LightingTab::Lights,

            lights: Vec::new(),
            selected_light: None,
            current_light: Light::default(),

            light_probes: Vec::new(),
            reflection_probes: Vec::new(),
            selected_probe: None,

            gi_settings: GiSettings::default(),
            environment: EnvironmentSettings::default(),
            shadow_quality: ShadowQuality::Medium,

            debug_shadows: false,
            debug_lightmaps: false,
            debug_probes: false,

            next_light_id: 1,
            next_probe_id: 1,

            pending_actions: Vec::new(),
        };

        panel
    }
}

impl LightingPanel {
    pub fn new() -> Self {
        Self::default()
    }

    // ==================== Action Queue Methods ====================

    /// Takes all pending actions, leaving the internal queue empty
    pub fn take_actions(&mut self) -> Vec<LightingAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for external processing
    pub fn queue_action(&mut self, action: LightingAction) {
        info!(target: "aw_editor::panels::lighting_panel", action = %action, "Lighting action queued");
        self.pending_actions.push(action);
    }

    /// Returns a reference to pending actions
    pub fn pending_actions(&self) -> &[LightingAction] {
        &self.pending_actions
    }

    fn create_sample_data(&mut self) {
        // Sun light
        let id = self.next_light_id();
        self.lights.push(Light {
            id,
            name: "Sun".to_string(),
            light_type: LightType::Directional,
            rotation: [-50.0, 30.0, 0.0],
            color: [1.0, 0.96, 0.84],
            intensity: 1.5,
            cast_shadows: true,
            shadow_type: ShadowType::Soft,
            shadow_resolution: 2048,
            ..Default::default()
        });
        self.next_light_id += 1;

        // Point light
        let id = self.next_light_id();
        self.lights.push(Light {
            id,
            name: "Torch Light".to_string(),
            light_type: LightType::Point,
            position: [5.0, 2.0, 0.0],
            color: [1.0, 0.7, 0.4],
            intensity: 2.0,
            range: 8.0,
            ..Default::default()
        });
        self.next_light_id += 1;

        // Spot light
        let id = self.next_light_id();
        self.lights.push(Light {
            id,
            name: "Spotlight".to_string(),
            light_type: LightType::Spot,
            position: [0.0, 5.0, -3.0],
            rotation: [45.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0],
            intensity: 3.0,
            range: 15.0,
            spot_angle: 45.0,
            inner_spot_angle: 30.0,
            ..Default::default()
        });
        self.next_light_id += 1;

        // Light probes
        for i in 0..4 {
            let id = self.next_probe_id();
            self.light_probes.push(LightProbe {
                id,
                name: format!("Light Probe {}", i + 1),
                position: [(i as f32 - 1.5) * 3.0, 1.0, 0.0],
                ..Default::default()
            });
            self.next_probe_id += 1;
        }

        // Reflection probe
        let id = self.next_probe_id();
        self.reflection_probes.push(ReflectionProbe {
            id,
            name: "Main Reflection Probe".to_string(),
            position: [0.0, 3.0, 0.0],
            box_size: [20.0, 10.0, 20.0],
            resolution: 512,
            ..Default::default()
        });
        self.next_probe_id += 1;

        if !self.lights.is_empty() {
            self.current_light = self.lights[0].clone();
            self.selected_light = Some(self.lights[0].id);
        }
    }

    fn next_light_id(&mut self) -> u32 {
        let id = self.next_light_id;
        self.next_light_id += 1;
        id
    }

    fn next_probe_id(&mut self) -> u32 {
        let id = self.next_probe_id;
        self.next_probe_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (LightingTab::Lights, "Lights"),
                (LightingTab::Shadows, "[Shd] Shadows"),
                (LightingTab::GI, "[Glb] GI"),
                (LightingTab::Probes, "Probes"),
                (LightingTab::Environment, "[Cld] Environment"),
                (LightingTab::Debug, "Debug"),
            ];

            for (tab, label) in tabs {
                let is_selected = self.active_tab == tab;
                let button = egui::Button::new(label).fill(if is_selected {
                    Color32::from_rgb(60, 100, 160)
                } else {
                    Color32::from_rgb(50, 50, 55)
                });

                if ui.add(button).clicked() {
                    self.active_tab = tab;
                }
            }
        });

        // Light count info
        ui.horizontal(|ui| {
            ui.label(format!("{} lights", self.lights.len()));
            ui.separator();
            ui.label(format!(
                "{} probes",
                self.light_probes.len() + self.reflection_probes.len()
            ));
        });

        ui.separator();
    }

    fn show_lights_tab(&mut self, ui: &mut Ui) {
        ui.heading("Scene Lights");
        ui.add_space(10.0);

        // Light list and add button
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("light_select")
                .selected_text(format!(
                    "{} {}",
                    self.current_light.light_type.icon(),
                    &self.current_light.name
                ))
                .show_ui(ui, |ui| {
                    for light in &self.lights.clone() {
                        let label = format!("{} {}", light.light_type.icon(), &light.name);
                        if ui
                            .selectable_value(&mut self.selected_light, Some(light.id), label)
                            .clicked()
                        {
                            self.current_light = light.clone();
                        }
                    }
                });

            if ui.button("+ Add Light").clicked() {
                let id = self.next_light_id();
                info!(target: "aw_editor::panels::lighting_panel", light_id = id, "New light added");
                let new_light = Light {
                    id,
                    name: format!("Light {}", id),
                    ..Default::default()
                };
                self.lights.push(new_light.clone());
                self.current_light = new_light;
                self.selected_light = Some(id);
            }

            if ui.button("[Del]").clicked() && self.lights.len() > 1 {
                if let Some(sel_id) = self.selected_light {
                    info!(target: "aw_editor::panels::lighting_panel", light_id = sel_id, "Light removed");
                    self.lights.retain(|l| l.id != sel_id);
                    if !self.lights.is_empty() {
                        self.current_light = self.lights[0].clone();
                        self.selected_light = Some(self.lights[0].id);
                    }
                }
            }
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                // Basic settings
                ui.group(|ui| {
                    ui.label(RichText::new("[Edit] Basic").strong());

                    egui::Grid::new("light_basic")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.current_light.name);
                            ui.end_row();

                            ui.label("Enabled:");
                            ui.checkbox(&mut self.current_light.enabled, "");
                            ui.end_row();

                            ui.label("Type:");
                            egui::ComboBox::from_id_salt("light_type")
                                .selected_text(format!(
                                    "{} {:?}",
                                    self.current_light.light_type.icon(),
                                    self.current_light.light_type
                                ))
                                .show_ui(ui, |ui| {
                                    for lt in LightType::all() {
                                        ui.selectable_value(
                                            &mut self.current_light.light_type,
                                            *lt,
                                            format!("{} {:?}", lt.icon(), lt),
                                        );
                                    }
                                });
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Color and intensity
                ui.group(|ui| {
                    ui.label(RichText::new("Color & Intensity").strong());

                    egui::Grid::new("light_color")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Color:");
                            let mut color = Color32::from_rgb(
                                (self.current_light.color[0] * 255.0) as u8,
                                (self.current_light.color[1] * 255.0) as u8,
                                (self.current_light.color[2] * 255.0) as u8,
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                self.current_light.color = [
                                    color.r() as f32 / 255.0,
                                    color.g() as f32 / 255.0,
                                    color.b() as f32 / 255.0,
                                ];
                            }
                            ui.end_row();

                            ui.label("Intensity:");
                            ui.add(egui::Slider::new(
                                &mut self.current_light.intensity,
                                0.0..=10.0,
                            ));
                            ui.end_row();

                            ui.label("Use Temperature:");
                            ui.checkbox(&mut self.current_light.use_temperature, "");
                            ui.end_row();

                            if self.current_light.use_temperature {
                                ui.label("Temperature (K):");
                                ui.add(egui::Slider::new(
                                    &mut self.current_light.temperature,
                                    1000.0..=20000.0,
                                ));
                                ui.end_row();
                            }
                        });
                });

                ui.add_space(10.0);

                // Type-specific settings
                ui.group(|ui| match self.current_light.light_type {
                    LightType::Point => {
                        ui.label(RichText::new("Point Light").strong());
                        ui.horizontal(|ui| {
                            ui.label("Range:");
                            ui.add(egui::Slider::new(
                                &mut self.current_light.range,
                                0.1..=100.0,
                            ));
                        });
                    }
                    LightType::Spot => {
                        ui.label(RichText::new("🔦 Spot Light").strong());

                        egui::Grid::new("spot_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Range:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_light.range,
                                    0.1..=100.0,
                                ));
                                ui.end_row();

                                ui.label("Spot Angle:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.current_light.spot_angle,
                                        1.0..=179.0,
                                    )
                                    .suffix("°"),
                                );
                                ui.end_row();

                                ui.label("Inner Angle:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.current_light.inner_spot_angle,
                                        0.0..=self.current_light.spot_angle,
                                    )
                                    .suffix("°"),
                                );
                                ui.end_row();
                            });
                    }
                    LightType::Area => {
                        ui.label(RichText::new("⬜ Area Light").strong());

                        egui::Grid::new("area_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Width:");
                                ui.add(
                                    egui::DragValue::new(&mut self.current_light.area_width)
                                        .speed(0.1)
                                        .range(0.1..=100.0),
                                );
                                ui.end_row();

                                ui.label("Height:");
                                ui.add(
                                    egui::DragValue::new(&mut self.current_light.area_height)
                                        .speed(0.1)
                                        .range(0.1..=100.0),
                                );
                                ui.end_row();
                            });
                    }
                    LightType::Directional => {
                        ui.label(RichText::new("[Sun] Directional Light").strong());
                        ui.label("Affects entire scene from rotation direction");
                    }
                    LightType::Ambient => {
                        ui.label(RichText::new("Ambient Light").strong());
                        ui.label("Uniform lighting throughout scene");
                    }
                });
            });
    }

    fn show_shadows_tab(&mut self, ui: &mut Ui) {
        ui.heading("🌑 Shadow Settings");
        ui.add_space(10.0);

        // Global shadow quality
        ui.group(|ui| {
            ui.label(RichText::new("Global Quality").strong());

            ui.horizontal(|ui| {
                ui.label("Shadow Quality:");
                egui::ComboBox::from_id_salt("shadow_quality")
                    .selected_text(format!("{:?}", self.shadow_quality))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.shadow_quality, ShadowQuality::Off, "Off");
                        ui.selectable_value(&mut self.shadow_quality, ShadowQuality::Low, "Low");
                        ui.selectable_value(
                            &mut self.shadow_quality,
                            ShadowQuality::Medium,
                            "Medium",
                        );
                        ui.selectable_value(&mut self.shadow_quality, ShadowQuality::High, "High");
                        ui.selectable_value(
                            &mut self.shadow_quality,
                            ShadowQuality::Ultra,
                            "Ultra",
                        );
                    });
            });
        });

        ui.add_space(10.0);

        // Current light shadow settings
        ui.group(|ui| {
            ui.label(RichText::new(format!("{} Shadows", self.current_light.name)).strong());

            egui::Grid::new("shadow_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Cast Shadows:");
                    ui.checkbox(&mut self.current_light.cast_shadows, "");
                    ui.end_row();

                    if self.current_light.cast_shadows {
                        ui.label("Shadow Type:");
                        egui::ComboBox::from_id_salt("shadow_type")
                            .selected_text(format!("{:?}", self.current_light.shadow_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.current_light.shadow_type,
                                    ShadowType::None,
                                    "None",
                                );
                                ui.selectable_value(
                                    &mut self.current_light.shadow_type,
                                    ShadowType::Hard,
                                    "Hard",
                                );
                                ui.selectable_value(
                                    &mut self.current_light.shadow_type,
                                    ShadowType::Soft,
                                    "Soft",
                                );
                                ui.selectable_value(
                                    &mut self.current_light.shadow_type,
                                    ShadowType::PCSS,
                                    "PCSS",
                                );
                            });
                        ui.end_row();

                        ui.label("Resolution:");
                        egui::ComboBox::from_id_salt("shadow_res")
                            .selected_text(format!("{}", self.current_light.shadow_resolution))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.current_light.shadow_resolution,
                                    256,
                                    "256",
                                );
                                ui.selectable_value(
                                    &mut self.current_light.shadow_resolution,
                                    512,
                                    "512",
                                );
                                ui.selectable_value(
                                    &mut self.current_light.shadow_resolution,
                                    1024,
                                    "1024",
                                );
                                ui.selectable_value(
                                    &mut self.current_light.shadow_resolution,
                                    2048,
                                    "2048",
                                );
                                ui.selectable_value(
                                    &mut self.current_light.shadow_resolution,
                                    4096,
                                    "4096",
                                );
                            });
                        ui.end_row();

                        ui.label("Bias:");
                        ui.add(egui::Slider::new(
                            &mut self.current_light.shadow_bias,
                            0.0..=0.5,
                        ));
                        ui.end_row();

                        ui.label("Normal Bias:");
                        ui.add(egui::Slider::new(
                            &mut self.current_light.shadow_normal_bias,
                            0.0..=3.0,
                        ));
                        ui.end_row();

                        ui.label("Near Plane:");
                        ui.add(
                            egui::Slider::new(
                                &mut self.current_light.shadow_near_plane,
                                0.01..=10.0,
                            )
                            .logarithmic(true),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    fn show_gi_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Glb] Global Illumination");
        ui.add_space(10.0);

        // GI Mode
        ui.group(|ui| {
            ui.label(RichText::new("Mode").strong());

            egui::ComboBox::from_id_salt("gi_mode")
                .selected_text(format!("{:?}", self.gi_settings.mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.gi_settings.mode, GiMode::None, "None");
                    ui.selectable_value(
                        &mut self.gi_settings.mode,
                        GiMode::BakedLightmaps,
                        "Baked Lightmaps",
                    );
                    ui.selectable_value(
                        &mut self.gi_settings.mode,
                        GiMode::RealtimeGI,
                        "Realtime GI",
                    );
                    ui.selectable_value(&mut self.gi_settings.mode, GiMode::Hybrid, "Hybrid");
                });
        });

        if self.gi_settings.mode == GiMode::None {
            ui.label("Global Illumination is disabled.");
            return;
        }

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                // Lightmap settings
                if matches!(
                    self.gi_settings.mode,
                    GiMode::BakedLightmaps | GiMode::Hybrid
                ) {
                    ui.group(|ui| {
                        ui.label(RichText::new("🗺️ Lightmaps").strong());

                        egui::Grid::new("lightmap_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Resolution:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.lightmap_resolution,
                                    10..=100,
                                ));
                                ui.end_row();

                                ui.label("Indirect Intensity:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.indirect_intensity,
                                    0.0..=5.0,
                                ));
                                ui.end_row();

                                ui.label("Bounce Count:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.bounce_count,
                                    1..=8,
                                ));
                                ui.end_row();

                                ui.label("Samples Per Texel:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.samples_per_texel,
                                    4..=64,
                                ));
                                ui.end_row();
                            });

                        ui.add_space(5.0);

                        if ui.button("🔨 Bake Lightmaps").clicked() {
                            info!(target: "aw_editor::panels::lighting_panel", "Lightmap bake started");
                            // Start lightmap baking
                        }
                    });
                }

                // Realtime GI settings
                if matches!(self.gi_settings.mode, GiMode::RealtimeGI | GiMode::Hybrid) {
                    ui.add_space(10.0);
                    ui.group(|ui| {
                        ui.label(RichText::new("Realtime GI").strong());

                        egui::Grid::new("realtime_gi")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Probe Spacing:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.probe_spacing,
                                    0.5..=5.0,
                                ));
                                ui.end_row();

                                ui.label("Update Rate:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.probe_update_rate,
                                    0.1..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Cascade Count:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.cascade_count,
                                    1..=8,
                                ));
                                ui.end_row();
                            });
                    });
                }

                // AO settings
                ui.add_space(10.0);
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.gi_settings.ambient_occlusion,
                        RichText::new("🌑 Ambient Occlusion").strong(),
                    );

                    if self.gi_settings.ambient_occlusion {
                        egui::Grid::new("ao_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Intensity:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.ao_intensity,
                                    0.0..=2.0,
                                ));
                                ui.end_row();

                                ui.label("Max Distance:");
                                ui.add(egui::Slider::new(
                                    &mut self.gi_settings.ao_max_distance,
                                    0.1..=10.0,
                                ));
                                ui.end_row();
                            });
                    }
                });
            });
    }

    fn show_probes_tab(&mut self, ui: &mut Ui) {
        ui.heading("Light & Reflection Probes");
        ui.add_space(10.0);

        // Light probes
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Light Probes").strong());
                if ui.button("+ Add").clicked() {
                    let id = self.next_probe_id();
                    info!(target: "aw_editor::panels::lighting_panel", probe_id = id, "Light probe added");
                    self.light_probes.push(LightProbe {
                        id,
                        name: format!("Light Probe {}", id),
                        ..Default::default()
                    });
                }
            });

            if self.light_probes.is_empty() {
                ui.label("No light probes. Click '+ Add' to create one.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .id_salt("light_probes_scroll")
                    .show(ui, |ui| {
                        for probe in &mut self.light_probes {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut probe.enabled, "");
                                ui.label(&probe.name);
                                if probe.baked {
                                    ui.label(
                                        RichText::new("(baked)").small().color(Color32::GREEN),
                                    );
                                }
                            });
                        }
                    });

                if ui.button("🔨 Bake All").clicked() {
                    info!(target: "aw_editor::panels::lighting_panel", count = self.light_probes.len(), "All light probes bake started");
                    for probe in &mut self.light_probes {
                        probe.baked = true;
                    }
                }
            }
        });

        ui.add_space(10.0);

        // Reflection probes
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🪞 Reflection Probes").strong());
                if ui.button("+ Add").clicked() {
                    let id = self.next_probe_id();
                    info!(target: "aw_editor::panels::lighting_panel", probe_id = id, "Reflection probe added");
                    self.reflection_probes.push(ReflectionProbe {
                        id,
                        name: format!("Reflection Probe {}", id),
                        ..Default::default()
                    });
                }
            });

            if self.reflection_probes.is_empty() {
                ui.label("No reflection probes. Click '+ Add' to create one.");
            } else {
                for probe in &mut self.reflection_probes {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut probe.enabled, "");
                            ui.label(RichText::new(&probe.name).strong());
                        });

                        egui::Grid::new(format!("refl_probe_{}", probe.id))
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Resolution:");
                                egui::ComboBox::from_id_salt(format!("refl_res_{}", probe.id))
                                    .selected_text(format!("{}", probe.resolution))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut probe.resolution, 64, "64");
                                        ui.selectable_value(&mut probe.resolution, 128, "128");
                                        ui.selectable_value(&mut probe.resolution, 256, "256");
                                        ui.selectable_value(&mut probe.resolution, 512, "512");
                                        ui.selectable_value(&mut probe.resolution, 1024, "1024");
                                    });
                                ui.end_row();

                                ui.label("HDR:");
                                ui.checkbox(&mut probe.hdr, "");
                                ui.end_row();

                                ui.label("Box Projection:");
                                ui.checkbox(&mut probe.box_projection, "");
                                ui.end_row();
                            });
                    });
                }
            }
        });
    }

    fn show_environment_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Cld] Environment");
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Skybox
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.environment.skybox_enabled,
                        RichText::new("🌌 Skybox").strong(),
                    );

                    if self.environment.skybox_enabled {
                        egui::Grid::new("skybox_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Texture:");
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(&mut self.environment.skybox_path);
                                    if ui.button("[Open]").clicked() {
                                        // Open file dialog
                                    }
                                });
                                ui.end_row();

                                ui.label("Exposure:");
                                ui.add(egui::Slider::new(
                                    &mut self.environment.skybox_exposure,
                                    0.0..=8.0,
                                ));
                                ui.end_row();

                                ui.label("Rotation:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.environment.skybox_rotation,
                                        0.0..=360.0,
                                    )
                                    .suffix("°"),
                                );
                                ui.end_row();
                            });
                    }
                });

                ui.add_space(10.0);

                // Ambient
                ui.group(|ui| {
                    ui.label(RichText::new("Ambient Light").strong());

                    ui.horizontal(|ui| {
                        ui.label("Mode:");
                        egui::ComboBox::from_id_salt("ambient_mode")
                            .selected_text(format!("{:?}", self.environment.ambient_mode))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.environment.ambient_mode,
                                    AmbientMode::Skybox,
                                    "Skybox",
                                );
                                ui.selectable_value(
                                    &mut self.environment.ambient_mode,
                                    AmbientMode::Color,
                                    "Color",
                                );
                                ui.selectable_value(
                                    &mut self.environment.ambient_mode,
                                    AmbientMode::Gradient,
                                    "Gradient",
                                );
                            });
                    });

                    match self.environment.ambient_mode {
                        AmbientMode::Skybox => {
                            ui.horizontal(|ui| {
                                ui.label("Intensity:");
                                ui.add(egui::Slider::new(
                                    &mut self.environment.ambient_intensity,
                                    0.0..=8.0,
                                ));
                            });
                        }
                        AmbientMode::Color => {
                            let mut color = Color32::from_rgb(
                                (self.environment.ambient_color[0] * 255.0) as u8,
                                (self.environment.ambient_color[1] * 255.0) as u8,
                                (self.environment.ambient_color[2] * 255.0) as u8,
                            );
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                if ui.color_edit_button_srgba(&mut color).changed() {
                                    self.environment.ambient_color = [
                                        color.r() as f32 / 255.0,
                                        color.g() as f32 / 255.0,
                                        color.b() as f32 / 255.0,
                                    ];
                                }
                            });
                        }
                        AmbientMode::Gradient => {
                            // Sky, equator, ground colors
                            ui.label("Sky Color:");
                            ui.label("Equator Color:");
                            ui.label("Ground Color:");
                        }
                    }
                });

                ui.add_space(10.0);

                // Fog
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.environment.fog_enabled,
                        RichText::new("[Fog] Fog").strong(),
                    );

                    if self.environment.fog_enabled {
                        egui::Grid::new("fog_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Mode:");
                                egui::ComboBox::from_id_salt("fog_mode")
                                    .selected_text(format!("{:?}", self.environment.fog_mode))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.environment.fog_mode,
                                            FogMode::Linear,
                                            "Linear",
                                        );
                                        ui.selectable_value(
                                            &mut self.environment.fog_mode,
                                            FogMode::Exponential,
                                            "Exponential",
                                        );
                                        ui.selectable_value(
                                            &mut self.environment.fog_mode,
                                            FogMode::ExponentialSquared,
                                            "Exponential Squared",
                                        );
                                        ui.selectable_value(
                                            &mut self.environment.fog_mode,
                                            FogMode::Height,
                                            "Height",
                                        );
                                    });
                                ui.end_row();

                                ui.label("Color:");
                                let mut color = Color32::from_rgb(
                                    (self.environment.fog_color[0] * 255.0) as u8,
                                    (self.environment.fog_color[1] * 255.0) as u8,
                                    (self.environment.fog_color[2] * 255.0) as u8,
                                );
                                if ui.color_edit_button_srgba(&mut color).changed() {
                                    self.environment.fog_color = [
                                        color.r() as f32 / 255.0,
                                        color.g() as f32 / 255.0,
                                        color.b() as f32 / 255.0,
                                    ];
                                }
                                ui.end_row();

                                match self.environment.fog_mode {
                                    FogMode::Linear => {
                                        ui.label("Start:");
                                        ui.add(
                                            egui::DragValue::new(&mut self.environment.fog_start)
                                                .speed(1.0),
                                        );
                                        ui.end_row();

                                        ui.label("End:");
                                        ui.add(
                                            egui::DragValue::new(&mut self.environment.fog_end)
                                                .speed(1.0),
                                        );
                                        ui.end_row();
                                    }
                                    FogMode::Exponential | FogMode::ExponentialSquared => {
                                        ui.label("Density:");
                                        ui.add(egui::Slider::new(
                                            &mut self.environment.fog_density,
                                            0.0..=0.1,
                                        ));
                                        ui.end_row();
                                    }
                                    FogMode::Height => {
                                        ui.label("Height:");
                                        ui.add(
                                            egui::DragValue::new(&mut self.environment.fog_height)
                                                .speed(0.5),
                                        );
                                        ui.end_row();

                                        ui.label("Height Density:");
                                        ui.add(egui::Slider::new(
                                            &mut self.environment.fog_height_density,
                                            0.0..=1.0,
                                        ));
                                        ui.end_row();
                                    }
                                }
                            });
                    }
                });
            });
    }

    fn show_debug_tab(&mut self, ui: &mut Ui) {
        ui.heading("Debug Visualization");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Debug Modes").strong());

            ui.checkbox(&mut self.debug_shadows, "Show Shadow Maps");
            ui.checkbox(&mut self.debug_lightmaps, "Show Lightmaps");
            ui.checkbox(&mut self.debug_probes, "Show Probe Volumes");
        });

        ui.add_space(10.0);

        // Light statistics
        ui.group(|ui| {
            ui.label(RichText::new("📈 Statistics").strong());

            ui.label(format!("Total Lights: {}", self.lights.len()));
            ui.label(format!(
                "Shadow-casting: {}",
                self.lights.iter().filter(|l| l.cast_shadows).count()
            ));
            ui.label(format!("Light Probes: {}", self.light_probes.len()));
            ui.label(format!(
                "Reflection Probes: {}",
                self.reflection_probes.len()
            ));
        });
    }

    // Getters for testing
    pub fn light_count(&self) -> usize {
        self.lights.len()
    }

    pub fn light_probe_count(&self) -> usize {
        self.light_probes.len()
    }

    pub fn reflection_probe_count(&self) -> usize {
        self.reflection_probes.len()
    }

    pub fn current_light_name(&self) -> &str {
        &self.current_light.name
    }

    pub fn add_light(&mut self, name: &str, light_type: LightType) -> u32 {
        let id = self.next_light_id();
        self.lights.push(Light {
            id,
            name: name.to_string(),
            light_type,
            ..Default::default()
        });
        id
    }

    pub fn set_light_intensity(&mut self, intensity: f32) {
        self.current_light.intensity = intensity.max(0.0);
    }
}

impl Panel for LightingPanel {
    fn name(&self) -> &'static str {
        "Lighting"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            LightingTab::Lights => self.show_lights_tab(ui),
            LightingTab::Shadows => self.show_shadows_tab(ui),
            LightingTab::GI => self.show_gi_tab(ui),
            LightingTab::Probes => self.show_probes_tab(ui),
            LightingTab::Environment => self.show_environment_tab(ui),
            LightingTab::Debug => self.show_debug_tab(ui),
        }
    }

    fn update(&mut self) {
        // Sync current light back to list
        if let Some(sel_id) = self.selected_light {
            if let Some(light) = self.lights.iter_mut().find(|l| l.id == sel_id) {
                *light = self.current_light.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // LIGHT TYPE TESTS
    // ============================================================

    #[test]
    fn test_light_type_default() {
        let lt = LightType::default();
        assert_eq!(lt, LightType::Directional);
    }

    #[test]
    fn test_light_type_all() {
        let all = LightType::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_light_type_icon_directional() {
        assert_eq!(LightType::Directional.icon(), "[Sun]");
    }

    #[test]
    fn test_light_type_icon_point() {
        assert_eq!(LightType::Point.icon(), "[Lgt]");
    }

    #[test]
    fn test_light_type_icon_spot() {
        assert_eq!(LightType::Spot.icon(), "🔦");
    }

    #[test]
    fn test_light_type_icon_area() {
        assert_eq!(LightType::Area.icon(), "⬜");
    }

    #[test]
    fn test_light_type_icon_ambient() {
        assert_eq!(LightType::Ambient.icon(), "[Net]");
    }

    #[test]
    fn test_light_type_all_have_icons() {
        for lt in LightType::all() {
            assert!(!lt.icon().is_empty());
        }
    }

    // ============================================================
    // SHADOW QUALITY TESTS
    // ============================================================

    #[test]
    fn test_shadow_quality_default() {
        let sq = ShadowQuality::default();
        assert_eq!(sq, ShadowQuality::Medium);
    }

    #[test]
    fn test_shadow_quality_all_variants() {
        let variants = [
            ShadowQuality::Off,
            ShadowQuality::Low,
            ShadowQuality::Medium,
            ShadowQuality::High,
            ShadowQuality::Ultra,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // SHADOW TYPE TESTS
    // ============================================================

    #[test]
    fn test_shadow_type_default() {
        let st = ShadowType::default();
        assert_eq!(st, ShadowType::Hard);
    }

    #[test]
    fn test_shadow_type_all_variants() {
        let variants = [
            ShadowType::None,
            ShadowType::Hard,
            ShadowType::Soft,
            ShadowType::PCSS,
        ];
        assert_eq!(variants.len(), 4);
    }

    // ============================================================
    // LIGHT UNIT TESTS
    // ============================================================

    #[test]
    fn test_light_unit_default() {
        let lu = LightUnit::default();
        assert_eq!(lu, LightUnit::Unitless);
    }

    #[test]
    fn test_light_unit_all_variants() {
        let variants = [
            LightUnit::Unitless,
            LightUnit::Lumen,
            LightUnit::Candela,
            LightUnit::Lux,
            LightUnit::Nit,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // GI MODE TESTS
    // ============================================================

    #[test]
    fn test_gi_mode_default() {
        let gm = GiMode::default();
        assert_eq!(gm, GiMode::None);
    }

    #[test]
    fn test_gi_mode_all_variants() {
        let variants = [
            GiMode::None,
            GiMode::BakedLightmaps,
            GiMode::RealtimeGI,
            GiMode::Hybrid,
        ];
        assert_eq!(variants.len(), 4);
    }

    // ============================================================
    // REFRESH MODE TESTS
    // ============================================================

    #[test]
    fn test_refresh_mode_default() {
        let rm = RefreshMode::default();
        assert_eq!(rm, RefreshMode::OnAwake);
    }

    #[test]
    fn test_refresh_mode_all_variants() {
        let variants = [
            RefreshMode::OnAwake,
            RefreshMode::EveryFrame,
            RefreshMode::ViaScript,
        ];
        assert_eq!(variants.len(), 3);
    }

    // ============================================================
    // AMBIENT MODE TESTS
    // ============================================================

    #[test]
    fn test_ambient_mode_default() {
        let am = AmbientMode::default();
        assert_eq!(am, AmbientMode::Skybox);
    }

    #[test]
    fn test_ambient_mode_all_variants() {
        let variants = [
            AmbientMode::Skybox,
            AmbientMode::Color,
            AmbientMode::Gradient,
        ];
        assert_eq!(variants.len(), 3);
    }

    // ============================================================
    // FOG MODE TESTS
    // ============================================================

    #[test]
    fn test_fog_mode_default() {
        let fm = FogMode::default();
        assert_eq!(fm, FogMode::Linear);
    }

    #[test]
    fn test_fog_mode_all_variants() {
        let variants = [
            FogMode::Linear,
            FogMode::Exponential,
            FogMode::ExponentialSquared,
            FogMode::Height,
        ];
        assert_eq!(variants.len(), 4);
    }

    // ============================================================
    // LIGHTING TAB TESTS
    // ============================================================

    #[test]
    fn test_lighting_tab_default() {
        let tab = LightingTab::default();
        assert_eq!(tab, LightingTab::Lights);
    }

    #[test]
    fn test_lighting_tab_all_variants() {
        let variants = [
            LightingTab::Lights,
            LightingTab::Shadows,
            LightingTab::GI,
            LightingTab::Probes,
            LightingTab::Environment,
            LightingTab::Debug,
        ];
        assert_eq!(variants.len(), 6);
    }

    // ============================================================
    // LIGHT TESTS
    // ============================================================

    #[test]
    fn test_light_default() {
        let light = Light::default();
        assert_eq!(light.id, 0);
        assert_eq!(light.name, "New Light");
        assert!(light.enabled);
        assert_eq!(light.light_type, LightType::Point);
    }

    #[test]
    fn test_light_default_transform() {
        let light = Light::default();
        assert!((light.position[1] - 5.0).abs() < 0.001);
        assert!((light.rotation[0] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_light_default_color() {
        let light = Light::default();
        assert!((light.color[0] - 1.0).abs() < 0.001);
        assert!((light.color[1] - 1.0).abs() < 0.001);
        assert!((light.color[2] - 1.0).abs() < 0.001);
        assert!((light.intensity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_light_default_temperature() {
        let light = Light::default();
        assert!((light.temperature - 6500.0).abs() < 0.001);
        assert!(!light.use_temperature);
    }

    #[test]
    fn test_light_default_shadows() {
        let light = Light::default();
        assert!(light.cast_shadows);
        assert_eq!(light.shadow_type, ShadowType::Soft);
        assert_eq!(light.shadow_resolution, 1024);
    }

    #[test]
    fn test_light_default_volumetric() {
        let light = Light::default();
        assert!(!light.volumetric);
        assert!((light.volumetric_strength - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_light_clone() {
        let light = Light::default();
        let cloned = light.clone();
        assert_eq!(cloned.name, "New Light");
        assert!(cloned.enabled);
    }

    // ============================================================
    // GI SETTINGS TESTS
    // ============================================================

    #[test]
    fn test_gi_settings_default() {
        let gi = GiSettings::default();
        assert_eq!(gi.mode, GiMode::None);
        assert!(gi.ambient_occlusion);
    }

    #[test]
    fn test_gi_settings_default_lightmap() {
        let gi = GiSettings::default();
        assert_eq!(gi.lightmap_resolution, 40);
        assert!((gi.indirect_intensity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gi_settings_default_ao() {
        let gi = GiSettings::default();
        assert!(gi.ambient_occlusion);
        assert!((gi.ao_intensity - 1.0).abs() < 0.001);
        assert!((gi.ao_max_distance - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gi_settings_default_realtime() {
        let gi = GiSettings::default();
        assert!((gi.probe_spacing - 1.0).abs() < 0.001);
        assert!((gi.probe_update_rate - 0.5).abs() < 0.001);
        assert_eq!(gi.cascade_count, 4);
    }

    #[test]
    fn test_gi_settings_default_quality() {
        let gi = GiSettings::default();
        assert_eq!(gi.bounce_count, 2);
        assert_eq!(gi.samples_per_texel, 16);
    }

    #[test]
    fn test_gi_settings_clone() {
        let gi = GiSettings::default();
        let cloned = gi.clone();
        assert_eq!(cloned.mode, GiMode::None);
    }

    // ============================================================
    // LIGHT PROBE TESTS
    // ============================================================

    #[test]
    fn test_light_probe_default() {
        let lp = LightProbe::default();
        assert_eq!(lp.id, 0);
        assert_eq!(lp.name, "Light Probe");
        assert!(lp.enabled);
    }

    #[test]
    fn test_light_probe_default_position() {
        let lp = LightProbe::default();
        assert!((lp.position[1] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_light_probe_default_properties() {
        let lp = LightProbe::default();
        assert!((lp.blend_distance - 1.0).abs() < 0.001);
        assert!((lp.importance - 1.0).abs() < 0.001);
        assert!(!lp.baked);
    }

    #[test]
    fn test_light_probe_clone() {
        let lp = LightProbe::default();
        let cloned = lp.clone();
        assert_eq!(cloned.name, "Light Probe");
    }

    // ============================================================
    // REFLECTION PROBE TESTS
    // ============================================================

    #[test]
    fn test_reflection_probe_default() {
        let rp = ReflectionProbe::default();
        assert_eq!(rp.id, 0);
        assert_eq!(rp.name, "Reflection Probe");
        assert!(rp.enabled);
    }

    #[test]
    fn test_reflection_probe_default_box() {
        let rp = ReflectionProbe::default();
        assert!((rp.box_size[0] - 10.0).abs() < 0.001);
        assert!((rp.box_size[1] - 10.0).abs() < 0.001);
        assert!((rp.box_size[2] - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_reflection_probe_default_resolution() {
        let rp = ReflectionProbe::default();
        assert_eq!(rp.resolution, 256);
        assert!(rp.hdr);
    }

    #[test]
    fn test_reflection_probe_default_refresh() {
        let rp = ReflectionProbe::default();
        assert!(!rp.realtime);
        assert_eq!(rp.refresh_mode, RefreshMode::OnAwake);
    }

    #[test]
    fn test_reflection_probe_clone() {
        let rp = ReflectionProbe::default();
        let cloned = rp.clone();
        assert_eq!(cloned.name, "Reflection Probe");
    }

    // ============================================================
    // ENVIRONMENT SETTINGS TESTS
    // ============================================================

    #[test]
    fn test_environment_default() {
        let env = EnvironmentSettings::default();
        assert!(env.skybox_enabled);
        assert!(!env.fog_enabled);
    }

    #[test]
    fn test_environment_default_skybox() {
        let env = EnvironmentSettings::default();
        assert!(env.skybox_enabled);
        assert!((env.skybox_exposure - 1.0).abs() < 0.001);
        assert!((env.skybox_rotation - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_environment_default_ambient() {
        let env = EnvironmentSettings::default();
        assert_eq!(env.ambient_mode, AmbientMode::Skybox);
        assert!((env.ambient_intensity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_environment_default_fog() {
        let env = EnvironmentSettings::default();
        assert!(!env.fog_enabled);
        assert_eq!(env.fog_mode, FogMode::Linear);
        assert!((env.fog_density - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_environment_default_fog_range() {
        let env = EnvironmentSettings::default();
        assert!((env.fog_start - 0.0).abs() < 0.001);
        assert!((env.fog_end - 300.0).abs() < 0.001);
    }

    #[test]
    fn test_environment_clone() {
        let env = EnvironmentSettings::default();
        let cloned = env.clone();
        assert!(cloned.skybox_enabled);
    }

    // ============================================================
    // LIGHTING PANEL TESTS
    // ============================================================

    #[test]
    fn test_lighting_panel_creation() {
        let panel = LightingPanel::new();
        assert!(!panel.current_light_name().is_empty());
    }

    #[test]
    fn test_default_sample_data() {
        let mut panel = LightingPanel::new();
        panel.create_sample_data();
        assert!(panel.light_count() >= 3);
        assert!(panel.light_probe_count() >= 4);
        assert!(panel.reflection_probe_count() >= 1);
    }

    #[test]
    fn test_add_light() {
        let mut panel = LightingPanel::new();
        let initial_count = panel.light_count();

        let id = panel.add_light("Test Light", LightType::Point);
        assert!(id > 0);
        assert_eq!(panel.light_count(), initial_count + 1);
    }

    #[test]
    fn test_add_multiple_lights() {
        let mut panel = LightingPanel::new();
        let initial = panel.light_count();
        let id1 = panel.add_light("Light A", LightType::Point);
        let id2 = panel.add_light("Light B", LightType::Spot);
        let id3 = panel.add_light("Light C", LightType::Directional);
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_eq!(panel.light_count(), initial + 3);
    }

    #[test]
    fn test_set_light_intensity() {
        let mut panel = LightingPanel::new();
        panel.set_light_intensity(5.0);
        assert!((panel.current_light.intensity - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_light_intensity_clamping() {
        let mut panel = LightingPanel::new();
        panel.set_light_intensity(-1.0);
        assert!((panel.current_light.intensity - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_light_intensity_high_value() {
        let mut panel = LightingPanel::new();
        panel.set_light_intensity(1000.0);
        assert!((panel.current_light.intensity - 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = LightingPanel::new();
        assert_eq!(panel.name(), "Lighting");
    }

    // ============================================================
    // INTEGRATION TESTS
    // ============================================================

    #[test]
    fn test_light_type_coverage() {
        let all = LightType::all();
        assert!(all.contains(&LightType::Directional));
        assert!(all.contains(&LightType::Point));
        assert!(all.contains(&LightType::Spot));
        assert!(all.contains(&LightType::Area));
        assert!(all.contains(&LightType::Ambient));
    }

    #[test]
    fn test_default_light_is_enabled() {
        let light = Light::default();
        assert!(light.enabled);
    }

    #[test]
    fn test_color_values_valid() {
        let light = Light::default();
        for c in light.color {
            assert!((0.0..=1.0).contains(&c));
        }
    }

    #[test]
    fn test_shadow_bias_reasonable() {
        let light = Light::default();
        assert!(light.shadow_bias >= 0.0);
        assert!(light.shadow_bias <= 1.0);
    }

    #[test]
    fn test_spot_angles_valid() {
        let light = Light::default();
        assert!(light.inner_spot_angle <= light.spot_angle);
        assert!(light.spot_angle <= 180.0);
    }

    #[test]
    fn test_fog_range_valid() {
        let env = EnvironmentSettings::default();
        assert!(env.fog_start <= env.fog_end);
    }

    // ============================================================
    // SESSION 6: ENUM DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_light_type_display() {
        // Verify Display trait outputs icon + name
        for light_type in LightType::all() {
            let display = format!("{}", light_type);
            assert!(display.contains(light_type.name()));
            assert!(display.contains(light_type.icon()));
        }
    }

    #[test]
    fn test_light_type_name() {
        assert_eq!(LightType::Directional.name(), "Directional");
        assert_eq!(LightType::Point.name(), "Point");
        assert_eq!(LightType::Spot.name(), "Spot");
        assert_eq!(LightType::Area.name(), "Area");
        assert_eq!(LightType::Ambient.name(), "Ambient");
    }

    #[test]
    fn test_light_type_is_directional() {
        assert!(LightType::Directional.is_directional());
        assert!(!LightType::Point.is_directional());
        assert!(!LightType::Spot.is_directional());
    }

    #[test]
    fn test_light_type_has_range() {
        assert!(!LightType::Directional.has_range());
        assert!(LightType::Point.has_range());
        assert!(LightType::Spot.has_range());
        assert!(!LightType::Area.has_range());
        assert!(!LightType::Ambient.has_range());
    }

    #[test]
    fn test_light_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(LightType::Directional);
        set.insert(LightType::Point);
        assert!(set.contains(&LightType::Directional));
        assert!(!set.contains(&LightType::Spot));
    }

    #[test]
    fn test_shadow_quality_display() {
        // Verify Display trait outputs icon + name
        for quality in ShadowQuality::all() {
            let display = format!("{}", quality);
            assert!(display.contains(quality.name()));
            assert!(display.contains(quality.icon()));
        }
    }

    #[test]
    fn test_shadow_quality_all() {
        let all = ShadowQuality::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&ShadowQuality::Off));
        assert!(all.contains(&ShadowQuality::Ultra));
    }

    #[test]
    fn test_shadow_quality_resolution() {
        assert_eq!(ShadowQuality::Off.resolution(), 0);
        assert_eq!(ShadowQuality::Low.resolution(), 512);
        assert_eq!(ShadowQuality::Medium.resolution(), 1024);
        assert_eq!(ShadowQuality::High.resolution(), 2048);
        assert_eq!(ShadowQuality::Ultra.resolution(), 4096);
    }

    #[test]
    fn test_shadow_quality_is_enabled() {
        assert!(!ShadowQuality::Off.is_enabled());
        assert!(ShadowQuality::Low.is_enabled());
        assert!(ShadowQuality::Ultra.is_enabled());
    }

    #[test]
    fn test_shadow_type_display() {
        // Verify Display trait outputs icon + name
        for shadow_type in ShadowType::all() {
            let display = format!("{}", shadow_type);
            assert!(display.contains(shadow_type.name()));
            assert!(display.contains(shadow_type.icon()));
        }
    }

    #[test]
    fn test_shadow_type_is_soft() {
        assert!(!ShadowType::None.is_soft());
        assert!(!ShadowType::Hard.is_soft());
        assert!(ShadowType::Soft.is_soft());
        assert!(ShadowType::PCSS.is_soft());
    }

    #[test]
    fn test_light_unit_display() {
        // Verify Display trait outputs icon + name
        for unit in LightUnit::all() {
            let display = format!("{}", unit);
            assert!(display.contains(unit.name()));
            assert!(display.contains(unit.icon()));
        }
    }

    #[test]
    fn test_light_unit_abbreviation() {
        assert_eq!(LightUnit::Unitless.abbreviation(), "");
        assert_eq!(LightUnit::Lumen.abbreviation(), "lm");
        assert_eq!(LightUnit::Candela.abbreviation(), "cd");
        assert_eq!(LightUnit::Lux.abbreviation(), "lx");
        assert_eq!(LightUnit::Nit.abbreviation(), "nt");
    }

    #[test]
    fn test_light_unit_is_physical() {
        assert!(!LightUnit::Unitless.is_physical());
        assert!(LightUnit::Lumen.is_physical());
        assert!(LightUnit::Candela.is_physical());
        assert!(LightUnit::Lux.is_physical());
        assert!(LightUnit::Nit.is_physical());
    }

    #[test]
    fn test_gi_mode_display() {
        // Verify Display trait outputs icon + name
        for mode in GiMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
            assert!(display.contains(mode.icon()));
        }
    }

    #[test]
    fn test_gi_mode_is_realtime() {
        assert!(!GiMode::None.is_realtime());
        assert!(!GiMode::BakedLightmaps.is_realtime());
        assert!(GiMode::RealtimeGI.is_realtime());
        assert!(GiMode::Hybrid.is_realtime());
    }

    #[test]
    fn test_gi_mode_requires_baking() {
        assert!(!GiMode::None.requires_baking());
        assert!(GiMode::BakedLightmaps.requires_baking());
        assert!(!GiMode::RealtimeGI.requires_baking());
        assert!(GiMode::Hybrid.requires_baking());
    }

    #[test]
    fn test_gi_mode_all() {
        let all = GiMode::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&GiMode::None));
        assert!(all.contains(&GiMode::Hybrid));
    }

    // ============================================================================
    // ENHANCED ENUM TESTS (RefreshMode, AmbientMode, FogMode, LightingTab)
    // ============================================================================

    #[test]
    fn test_refresh_mode_display() {
        for mode in RefreshMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
            assert!(display.contains(mode.icon()));
        }
    }

    #[test]
    fn test_refresh_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in RefreshMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_refresh_mode_all() {
        let all = RefreshMode::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&RefreshMode::OnAwake));
        assert!(all.contains(&RefreshMode::EveryFrame));
        assert!(all.contains(&RefreshMode::ViaScript));
    }

    #[test]
    fn test_refresh_mode_name() {
        assert_eq!(RefreshMode::OnAwake.name(), "On Awake");
        assert_eq!(RefreshMode::EveryFrame.name(), "Every Frame");
        assert_eq!(RefreshMode::ViaScript.name(), "Via Script");
    }

    #[test]
    fn test_refresh_mode_is_automatic() {
        assert!(RefreshMode::OnAwake.is_automatic());
        assert!(RefreshMode::EveryFrame.is_automatic());
        assert!(!RefreshMode::ViaScript.is_automatic());
    }

    #[test]
    fn test_ambient_mode_display() {
        for mode in AmbientMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
            assert!(display.contains(mode.icon()));
        }
    }

    #[test]
    fn test_ambient_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in AmbientMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_ambient_mode_all() {
        let all = AmbientMode::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&AmbientMode::Skybox));
        assert!(all.contains(&AmbientMode::Color));
        assert!(all.contains(&AmbientMode::Gradient));
    }

    #[test]
    fn test_ambient_mode_name() {
        assert_eq!(AmbientMode::Skybox.name(), "Skybox");
        assert_eq!(AmbientMode::Color.name(), "Color");
        assert_eq!(AmbientMode::Gradient.name(), "Gradient");
    }

    #[test]
    fn test_fog_mode_display() {
        for mode in FogMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
            assert!(display.contains(mode.icon()));
        }
    }

    #[test]
    fn test_fog_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in FogMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_fog_mode_all() {
        let all = FogMode::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&FogMode::Linear));
        assert!(all.contains(&FogMode::Exponential));
        assert!(all.contains(&FogMode::ExponentialSquared));
        assert!(all.contains(&FogMode::Height));
    }

    #[test]
    fn test_fog_mode_name() {
        assert_eq!(FogMode::Linear.name(), "Linear");
        assert_eq!(FogMode::Exponential.name(), "Exponential");
        assert_eq!(FogMode::ExponentialSquared.name(), "Exponential Squared");
        assert_eq!(FogMode::Height.name(), "Height");
    }

    #[test]
    fn test_fog_mode_is_exponential() {
        assert!(!FogMode::Linear.is_exponential());
        assert!(FogMode::Exponential.is_exponential());
        assert!(FogMode::ExponentialSquared.is_exponential());
        assert!(!FogMode::Height.is_exponential());
    }

    #[test]
    fn test_lighting_tab_display() {
        for tab in LightingTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()));
            assert!(display.contains(tab.icon()));
        }
    }

    #[test]
    fn test_lighting_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in LightingTab::all() {
            set.insert(*tab);
        }
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn test_lighting_tab_all() {
        let all = LightingTab::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&LightingTab::Lights));
        assert!(all.contains(&LightingTab::Shadows));
        assert!(all.contains(&LightingTab::GI));
        assert!(all.contains(&LightingTab::Probes));
        assert!(all.contains(&LightingTab::Environment));
        assert!(all.contains(&LightingTab::Debug));
    }

    #[test]
    fn test_lighting_tab_name() {
        assert_eq!(LightingTab::Lights.name(), "Lights");
        assert_eq!(LightingTab::Shadows.name(), "Shadows");
        assert_eq!(LightingTab::GI.name(), "GI");
        assert_eq!(LightingTab::Probes.name(), "Probes");
        assert_eq!(LightingTab::Environment.name(), "Environment");
        assert_eq!(LightingTab::Debug.name(), "Debug");
    }

    #[test]
    fn test_lighting_tab_icon() {
        assert_eq!(LightingTab::Lights.icon(), "[Lgt]");
        assert_eq!(LightingTab::Shadows.icon(), "[Shd]");
        assert_eq!(LightingTab::GI.icon(), "[GI]");
        assert_eq!(LightingTab::Probes.icon(), "[Orb]");
        assert_eq!(LightingTab::Environment.icon(), "[Glb]");
        assert_eq!(LightingTab::Debug.icon(), "[Bug]");
    }

    // =====================================================================
    // LightingAction Tests
    // =====================================================================

    #[test]
    fn test_lighting_action_display_light_management() {
        let action = LightingAction::AddLight(LightType::Point);
        assert!(format!("{}", action).contains("Point"));

        let action = LightingAction::RemoveLight(42);
        assert!(format!("{}", action).contains("42"));

        let action = LightingAction::SelectLight(5);
        assert!(format!("{}", action).contains("5"));
    }

    #[test]
    fn test_lighting_action_display_light_properties() {
        let action = LightingAction::SetLightEnabled {
            id: 1,
            enabled: true,
        };
        assert!(format!("{}", action).contains("Enable"));

        let action = LightingAction::SetLightEnabled {
            id: 1,
            enabled: false,
        };
        assert!(format!("{}", action).contains("Disable"));

        let action = LightingAction::SetLightIntensity {
            id: 1,
            intensity: 2.5,
        };
        assert!(format!("{}", action).contains("2.5"));
    }

    #[test]
    fn test_lighting_action_display_shadows() {
        let action = LightingAction::ToggleShadows {
            id: 1,
            enabled: true,
        };
        assert!(format!("{}", action).contains("Enable"));

        let action = LightingAction::SetShadowResolution {
            id: 1,
            resolution: 2048,
        };
        assert!(format!("{}", action).contains("2048"));

        let action = LightingAction::SetShadowQuality(ShadowQuality::High);
        assert!(format!("{}", action).contains("High"));
    }

    #[test]
    fn test_lighting_action_display_gi() {
        let action = LightingAction::SetGiMode(GiMode::RealtimeGI);
        assert!(format!("{}", action).contains("Realtime"));

        let action = LightingAction::BakeLightmaps;
        assert!(format!("{}", action).contains("Bake"));

        let action = LightingAction::ToggleAmbientOcclusion(true);
        assert!(format!("{}", action).contains("on"));
    }

    #[test]
    fn test_lighting_action_display_probes() {
        let action = LightingAction::AddLightProbe([1.0, 2.0, 3.0]);
        assert!(format!("{}", action).contains("1.0"));

        let action = LightingAction::BakeProbes;
        assert!(format!("{}", action).contains("Bake"));

        let action = LightingAction::RemoveReflectionProbe(5);
        assert!(format!("{}", action).contains("5"));
    }

    #[test]
    fn test_lighting_action_display_environment() {
        let action = LightingAction::SetSkyboxPath("sky/cloudy.hdr".to_string());
        assert!(format!("{}", action).contains("cloudy"));

        let action = LightingAction::SetFogEnabled(true);
        assert!(format!("{}", action).contains("on"));

        let action = LightingAction::SetFogDensity(0.05);
        assert!(format!("{}", action).contains("0.05"));
    }

    #[test]
    fn test_lighting_action_display_debug() {
        let action = LightingAction::ToggleDebugShadows(true);
        assert!(format!("{}", action).contains("shadow"));

        let action = LightingAction::ToggleDebugLightmaps(false);
        assert!(format!("{}", action).contains("off"));

        let action = LightingAction::ToggleDebugProbes(true);
        assert!(format!("{}", action).contains("probe"));
    }

    #[test]
    fn test_lighting_panel_action_queue() {
        let mut panel = LightingPanel::new();
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());

        panel.queue_action(LightingAction::AddLight(LightType::Spot));
        assert!(panel.has_pending_actions());
        assert_eq!(panel.pending_actions().len(), 1);
    }

    #[test]
    fn test_lighting_panel_take_actions() {
        let mut panel = LightingPanel::new();
        panel.queue_action(LightingAction::BakeLightmaps);
        panel.queue_action(LightingAction::BakeProbes);

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_lighting_panel_action_order() {
        let mut panel = LightingPanel::new();
        panel.queue_action(LightingAction::AddLight(LightType::Point));
        panel.queue_action(LightingAction::SetLightIntensity {
            id: 1,
            intensity: 5.0,
        });
        panel.queue_action(LightingAction::ToggleShadows {
            id: 1,
            enabled: true,
        });

        let actions = panel.take_actions();
        assert!(matches!(actions[0], LightingAction::AddLight(_)));
        assert!(matches!(
            actions[1],
            LightingAction::SetLightIntensity { .. }
        ));
        assert!(matches!(actions[2], LightingAction::ToggleShadows { .. }));
    }

    #[test]
    fn test_lighting_action_spot_properties() {
        let action = LightingAction::SetSpotAngle { id: 1, angle: 45.0 };
        assert!(format!("{}", action).contains("45.0"));

        let action = LightingAction::SetAreaSize {
            id: 2,
            width: 10.0,
            height: 5.0,
        };
        let display = format!("{}", action);
        assert!(display.contains("10.0"));
        assert!(display.contains("5.0"));
    }

    #[test]
    fn test_lighting_action_position_rotation() {
        let action = LightingAction::SetLightPosition {
            id: 1,
            position: [1.0, 2.0, 3.0],
        };
        assert!(format!("{}", action).contains("Move"));

        let action = LightingAction::SetLightRotation {
            id: 1,
            rotation: [0.0, 90.0, 0.0],
        };
        assert!(format!("{}", action).contains("Rotate"));
    }

    #[test]
    fn test_lighting_action_equality() {
        let a1 = LightingAction::AddLight(LightType::Point);
        let a2 = LightingAction::AddLight(LightType::Point);
        let a3 = LightingAction::AddLight(LightType::Spot);

        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn test_lighting_action_clone() {
        let action = LightingAction::SetSkyboxPath("/path/to/sky.hdr".to_string());
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }
}
