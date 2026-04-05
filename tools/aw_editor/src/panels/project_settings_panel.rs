//! Project Settings Panel for the editor
//!
//! Provides centralized project configuration:
//! - Engine settings (rendering, physics, audio)
//! - Build settings (platforms, packaging)
//! - Input configuration
//! - Quality levels
//! - Tags and layers
//! - Version control integration

use egui::{Color32, RichText, Ui};

use crate::panels::Panel;

/// Target platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TargetPlatform {
    Windows,
    Linux,
    MacOS,
    Android,
    Ios,
    WebAssembly,
    PlayStation,
    Xbox,
    NintendoSwitch,
}

impl std::fmt::Display for TargetPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl TargetPlatform {
    pub fn name(&self) -> &'static str {
        match self {
            TargetPlatform::Windows => "Windows",
            TargetPlatform::Linux => "Linux",
            TargetPlatform::MacOS => "macOS",
            TargetPlatform::Android => "Android",
            TargetPlatform::Ios => "iOS",
            TargetPlatform::WebAssembly => "WebAssembly",
            TargetPlatform::PlayStation => "PlayStation",
            TargetPlatform::Xbox => "Xbox",
            TargetPlatform::NintendoSwitch => "Nintendo Switch",
        }
    }

    pub fn all() -> &'static [TargetPlatform] {
        &[
            TargetPlatform::Windows,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Android,
            TargetPlatform::Ios,
            TargetPlatform::WebAssembly,
            TargetPlatform::PlayStation,
            TargetPlatform::Xbox,
            TargetPlatform::NintendoSwitch,
        ]
    }

    pub fn is_desktop(&self) -> bool {
        matches!(
            self,
            TargetPlatform::Windows | TargetPlatform::Linux | TargetPlatform::MacOS
        )
    }

    pub fn is_mobile(&self) -> bool {
        matches!(self, TargetPlatform::Android | TargetPlatform::Ios)
    }

    pub fn is_console(&self) -> bool {
        matches!(
            self,
            TargetPlatform::PlayStation | TargetPlatform::Xbox | TargetPlatform::NintendoSwitch
        )
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TargetPlatform::Windows => "🪟",
            TargetPlatform::Linux => "🐧",
            TargetPlatform::MacOS => "🍎",
            TargetPlatform::Android => "[Bot]",
            TargetPlatform::Ios => "📱",
            TargetPlatform::WebAssembly => "[Net]",
            TargetPlatform::PlayStation => "[Gp]",
            TargetPlatform::Xbox => "[Gp]",
            TargetPlatform::NintendoSwitch => "[Gp]",
        }
    }
}

/// Quality level preset
#[derive(Debug, Clone)]
pub struct QualityLevel {
    pub name: String,
    pub shadow_resolution: u32,
    pub shadow_cascades: u32,
    pub texture_quality: TextureQuality,
    pub antialiasing: AntialiasingMode,
    pub vsync: bool,
    pub max_fps: u32,
    pub lod_bias: f32,
    pub particle_density: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextureQuality {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
}

impl std::fmt::Display for TextureQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TextureQuality {
    pub fn name(&self) -> &'static str {
        match self {
            TextureQuality::Low => "Low",
            TextureQuality::Medium => "Medium",
            TextureQuality::High => "High",
            TextureQuality::Ultra => "Ultra",
        }
    }

    pub fn all() -> &'static [TextureQuality] {
        &[
            TextureQuality::Low,
            TextureQuality::Medium,
            TextureQuality::High,
            TextureQuality::Ultra,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AntialiasingMode {
    None,
    Fxaa,
    #[default]
    Smaa,
    Taa,
    Msaa2x,
    Msaa4x,
    Msaa8x,
}

impl std::fmt::Display for AntialiasingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl AntialiasingMode {
    pub fn name(&self) -> &'static str {
        match self {
            AntialiasingMode::None => "None",
            AntialiasingMode::Fxaa => "FXAA",
            AntialiasingMode::Smaa => "SMAA",
            AntialiasingMode::Taa => "TAA",
            AntialiasingMode::Msaa2x => "MSAA 2x",
            AntialiasingMode::Msaa4x => "MSAA 4x",
            AntialiasingMode::Msaa8x => "MSAA 8x",
        }
    }

    pub fn all() -> &'static [AntialiasingMode] {
        &[
            AntialiasingMode::None,
            AntialiasingMode::Fxaa,
            AntialiasingMode::Smaa,
            AntialiasingMode::Taa,
            AntialiasingMode::Msaa2x,
            AntialiasingMode::Msaa4x,
            AntialiasingMode::Msaa8x,
        ]
    }

    pub fn is_msaa(&self) -> bool {
        matches!(
            self,
            AntialiasingMode::Msaa2x | AntialiasingMode::Msaa4x | AntialiasingMode::Msaa8x
        )
    }
}

impl Default for QualityLevel {
    fn default() -> Self {
        Self {
            name: "Medium".to_string(),
            shadow_resolution: 2048,
            shadow_cascades: 4,
            texture_quality: TextureQuality::High,
            antialiasing: AntialiasingMode::Smaa,
            vsync: true,
            max_fps: 60,
            lod_bias: 1.0,
            particle_density: 1.0,
        }
    }
}

/// Physics settings
#[derive(Debug, Clone)]
pub struct PhysicsSettings {
    pub gravity: [f32; 3],
    pub fixed_timestep: f32,
    pub max_substeps: u32,
    pub broadphase: BroadphaseType,
    pub default_friction: f32,
    pub default_restitution: f32,
    pub sleep_threshold: f32,
    pub enable_ccd: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BroadphaseType {
    #[default]
    Sap,
    DynamicAabb,
    Quadtree,
}

impl std::fmt::Display for BroadphaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl BroadphaseType {
    pub fn name(&self) -> &'static str {
        match self {
            BroadphaseType::Sap => "SAP",
            BroadphaseType::DynamicAabb => "Dynamic AABB",
            BroadphaseType::Quadtree => "Quadtree",
        }
    }

    pub fn all() -> &'static [BroadphaseType] {
        &[
            BroadphaseType::Sap,
            BroadphaseType::DynamicAabb,
            BroadphaseType::Quadtree,
        ]
    }
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            fixed_timestep: 1.0 / 60.0,
            max_substeps: 4,
            broadphase: BroadphaseType::Sap,
            default_friction: 0.5,
            default_restitution: 0.3,
            sleep_threshold: 0.1,
            enable_ccd: true,
        }
    }
}

/// Audio settings
#[derive(Debug, Clone)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub voice_volume: f32,
    pub ambient_volume: f32,
    pub max_simultaneous_sounds: u32,
    pub doppler_factor: f32,
    pub audio_backend: AudioBackend,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AudioBackend {
    #[default]
    Auto,
    Wasapi,
    CoreAudio,
    Alsa,
    PulseAudio,
}

impl std::fmt::Display for AudioBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl AudioBackend {
    pub fn name(&self) -> &'static str {
        match self {
            AudioBackend::Auto => "Auto",
            AudioBackend::Wasapi => "WASAPI",
            AudioBackend::CoreAudio => "Core Audio",
            AudioBackend::Alsa => "ALSA",
            AudioBackend::PulseAudio => "PulseAudio",
        }
    }

    pub fn all() -> &'static [AudioBackend] {
        &[
            AudioBackend::Auto,
            AudioBackend::Wasapi,
            AudioBackend::CoreAudio,
            AudioBackend::Alsa,
            AudioBackend::PulseAudio,
        ]
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.8,
            sfx_volume: 1.0,
            voice_volume: 1.0,
            ambient_volume: 0.7,
            max_simultaneous_sounds: 64,
            doppler_factor: 1.0,
            audio_backend: AudioBackend::Auto,
        }
    }
}

/// Rendering settings
#[derive(Debug, Clone)]
pub struct RenderingSettings {
    pub renderer_backend: RendererBackend,
    pub hdr_enabled: bool,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub tonemapping: TonemappingMode,
    pub ambient_occlusion: AoMode,
    pub global_illumination: GiMode,
    pub reflection_mode: ReflectionMode,
    pub shadow_mode: ShadowMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RendererBackend {
    #[default]
    Auto,
    Vulkan,
    DirectX12,
    Metal,
    OpenGL,
    WebGpu,
}

impl std::fmt::Display for RendererBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl RendererBackend {
    pub fn name(&self) -> &'static str {
        match self {
            RendererBackend::Auto => "Auto",
            RendererBackend::Vulkan => "Vulkan",
            RendererBackend::DirectX12 => "DirectX 12",
            RendererBackend::Metal => "Metal",
            RendererBackend::OpenGL => "OpenGL",
            RendererBackend::WebGpu => "WebGPU",
        }
    }

    pub fn all() -> &'static [RendererBackend] {
        &[
            RendererBackend::Auto,
            RendererBackend::Vulkan,
            RendererBackend::DirectX12,
            RendererBackend::Metal,
            RendererBackend::OpenGL,
            RendererBackend::WebGpu,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TonemappingMode {
    None,
    Reinhard,
    #[default]
    Aces,
    AgX,
    Filmic,
}

impl std::fmt::Display for TonemappingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TonemappingMode {
    pub fn name(&self) -> &'static str {
        match self {
            TonemappingMode::None => "None",
            TonemappingMode::Reinhard => "Reinhard",
            TonemappingMode::Aces => "ACES",
            TonemappingMode::AgX => "AgX",
            TonemappingMode::Filmic => "Filmic",
        }
    }

    pub fn all() -> &'static [TonemappingMode] {
        &[
            TonemappingMode::None,
            TonemappingMode::Reinhard,
            TonemappingMode::Aces,
            TonemappingMode::AgX,
            TonemappingMode::Filmic,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AoMode {
    None,
    Ssao,
    #[default]
    Hbao,
    Gtao,
}

impl std::fmt::Display for AoMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl AoMode {
    pub fn name(&self) -> &'static str {
        match self {
            AoMode::None => "None",
            AoMode::Ssao => "SSAO",
            AoMode::Hbao => "HBAO+",
            AoMode::Gtao => "GTAO",
        }
    }

    pub fn all() -> &'static [AoMode] {
        &[AoMode::None, AoMode::Ssao, AoMode::Hbao, AoMode::Gtao]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GiMode {
    #[default]
    None,
    ScreenSpace,
    Lumen,
    PathTraced,
}

impl std::fmt::Display for GiMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl GiMode {
    pub fn name(&self) -> &'static str {
        match self {
            GiMode::None => "None",
            GiMode::ScreenSpace => "Screen Space",
            GiMode::Lumen => "Lumen",
            GiMode::PathTraced => "Path Traced",
        }
    }

    pub fn all() -> &'static [GiMode] {
        &[
            GiMode::None,
            GiMode::ScreenSpace,
            GiMode::Lumen,
            GiMode::PathTraced,
        ]
    }

    pub fn is_raytraced(&self) -> bool {
        matches!(self, GiMode::Lumen | GiMode::PathTraced)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReflectionMode {
    None,
    #[default]
    ScreenSpace,
    Raytraced,
    Hybrid,
}

impl std::fmt::Display for ReflectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ReflectionMode {
    pub fn name(&self) -> &'static str {
        match self {
            ReflectionMode::None => "None",
            ReflectionMode::ScreenSpace => "Screen Space",
            ReflectionMode::Raytraced => "Raytraced",
            ReflectionMode::Hybrid => "Hybrid",
        }
    }

    pub fn all() -> &'static [ReflectionMode] {
        &[
            ReflectionMode::None,
            ReflectionMode::ScreenSpace,
            ReflectionMode::Raytraced,
            ReflectionMode::Hybrid,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShadowMode {
    None,
    HardShadows,
    #[default]
    SoftShadows,
    Raytraced,
}

impl std::fmt::Display for ShadowMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ShadowMode {
    pub fn name(&self) -> &'static str {
        match self {
            ShadowMode::None => "None",
            ShadowMode::HardShadows => "Hard Shadows",
            ShadowMode::SoftShadows => "Soft Shadows",
            ShadowMode::Raytraced => "Raytraced",
        }
    }

    pub fn all() -> &'static [ShadowMode] {
        &[
            ShadowMode::None,
            ShadowMode::HardShadows,
            ShadowMode::SoftShadows,
            ShadowMode::Raytraced,
        ]
    }
}

impl Default for RenderingSettings {
    fn default() -> Self {
        Self {
            renderer_backend: RendererBackend::Auto,
            hdr_enabled: true,
            bloom_enabled: true,
            bloom_intensity: 0.5,
            tonemapping: TonemappingMode::Aces,
            ambient_occlusion: AoMode::Hbao,
            global_illumination: GiMode::ScreenSpace,
            reflection_mode: ReflectionMode::ScreenSpace,
            shadow_mode: ShadowMode::SoftShadows,
        }
    }
}

/// Input action
#[derive(Debug, Clone)]
pub struct InputAction {
    pub name: String,
    pub primary_key: String,
    pub secondary_key: String,
    pub gamepad_button: String,
    pub dead_zone: f32,
}

impl Default for InputAction {
    fn default() -> Self {
        Self {
            name: String::new(),
            primary_key: String::new(),
            secondary_key: String::new(),
            gamepad_button: String::new(),
            dead_zone: 0.1,
        }
    }
}

/// Layer definition
#[derive(Debug, Clone)]
pub struct Layer {
    pub id: u32,
    pub name: String,
    pub collides_with: Vec<u32>,
}

/// Tag definition
#[derive(Debug, Clone)]
pub struct Tag {
    pub id: u32,
    pub name: String,
    pub color: Color32,
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub platform: TargetPlatform,
    pub enabled: bool,
    pub development_build: bool,
    pub compression: CompressionMode,
    pub icon_path: String,
    pub app_name: String,
    pub version: String,
    pub company: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompressionMode {
    None,
    #[default]
    Fast,
    Best,
}

impl std::fmt::Display for CompressionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl CompressionMode {
    pub fn name(&self) -> &'static str {
        match self {
            CompressionMode::None => "None",
            CompressionMode::Fast => "Fast",
            CompressionMode::Best => "Best",
        }
    }

    pub fn all() -> &'static [CompressionMode] {
        &[
            CompressionMode::None,
            CompressionMode::Fast,
            CompressionMode::Best,
        ]
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            platform: TargetPlatform::Windows,
            enabled: true,
            development_build: true,
            compression: CompressionMode::Fast,
            icon_path: String::new(),
            app_name: "My Game".to_string(),
            version: "1.0.0".to_string(),
            company: "My Company".to_string(),
        }
    }
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SettingsTab {
    #[default]
    Project,
    Gameplay,
    Rendering,
    Physics,
    Audio,
    Input,
    Quality,
    TagsLayers,
    Build,
}

/// Actions that can be performed by the project settings panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ProjectSettingsAction {
    // Project settings
    SetProjectName(String),
    SetProjectVersion(String),
    SetCompanyName(String),
    SetDefaultScene(String),

    // Rendering settings
    SetRendererBackend(RendererBackend),
    ToggleHDR(bool),
    ToggleBloom(bool),
    SetBloomIntensity(f32),
    SetTonemapping(TonemappingMode),
    SetAmbientOcclusion(AoMode),
    SetGlobalIllumination(GiMode),
    SetReflectionMode(ReflectionMode),
    SetShadowMode(ShadowMode),

    // Physics settings
    SetGravity([f32; 3]),
    SetFixedTimestep(f32),
    SetDefaultFriction(f32),
    SetDefaultRestitution(f32),

    // Audio settings
    SetMasterVolume(f32),
    SetMusicVolume(f32),
    SetSfxVolume(f32),
    SetVoiceVolume(f32),
    SetAmbientVolume(f32),
    SetMaxSimultaneousSounds(u32),
    SetAudioBackend(AudioBackend),

    // Quality levels
    AddQualityLevel,
    RemoveQualityLevel(usize),
    SelectQualityLevel(usize),
    SetQualityLevelName {
        index: usize,
        name: String,
    },
    ApplyQualityLevel(usize),

    // Input actions
    AddInputAction,
    RemoveInputAction(usize),
    SelectInputAction(usize),
    SetInputActionName {
        index: usize,
        name: String,
    },
    SetInputActionKey {
        index: usize,
        key: String,
    },

    // Tags and layers
    AddTag(String),
    RemoveTag(u32),
    AddLayer(String),
    RemoveLayer(u32),

    // Build
    AddBuildConfig,
    RemoveBuildConfig(usize),
    SelectBuildConfig(usize),
    SetBuildPlatform {
        index: usize,
        platform: TargetPlatform,
    },
    ToggleBuildDevelopment {
        index: usize,
        development: bool,
    },
    StartBuild(usize),

    // General
    SaveSettings,
    LoadSettings,
    ResetToDefaults,
    ExportSettings(String),
    ImportSettings(String),
}

impl std::fmt::Display for ProjectSettingsAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetProjectName(name) => write!(f, "Set project name to '{}'", name),
            Self::SetProjectVersion(v) => write!(f, "Set version to '{}'", v),
            Self::SetCompanyName(name) => write!(f, "Set company to '{}'", name),
            Self::SetDefaultScene(s) => write!(f, "Set default scene to '{}'", s),
            Self::SetRendererBackend(b) => write!(f, "Set renderer to {}", b),
            Self::ToggleHDR(on) => write!(f, "Turn HDR {}", if *on { "on" } else { "off" }),
            Self::ToggleBloom(on) => write!(f, "Turn bloom {}", if *on { "on" } else { "off" }),
            Self::SetBloomIntensity(i) => write!(f, "Set bloom intensity to {:.2}", i),
            Self::SetTonemapping(m) => write!(f, "Set tonemapping to {}", m),
            Self::SetAmbientOcclusion(m) => write!(f, "Set AO to {}", m),
            Self::SetGlobalIllumination(m) => write!(f, "Set GI to {}", m),
            Self::SetReflectionMode(m) => write!(f, "Set reflections to {}", m),
            Self::SetShadowMode(m) => write!(f, "Set shadows to {}", m),
            Self::SetGravity(g) => {
                write!(f, "Set gravity to ({:.1}, {:.1}, {:.1})", g[0], g[1], g[2])
            }
            Self::SetFixedTimestep(t) => write!(f, "Set fixed timestep to {:.4}s", t),
            Self::SetDefaultFriction(fr) => write!(f, "Set friction to {:.2}", fr),
            Self::SetDefaultRestitution(r) => write!(f, "Set restitution to {:.2}", r),
            Self::SetMasterVolume(v) => write!(f, "Set master volume to {:.0}%", v * 100.0),
            Self::SetMusicVolume(v) => write!(f, "Set music volume to {:.0}%", v * 100.0),
            Self::SetSfxVolume(v) => write!(f, "Set SFX volume to {:.0}%", v * 100.0),
            Self::SetVoiceVolume(v) => write!(f, "Set voice volume to {:.0}%", v * 100.0),
            Self::SetAmbientVolume(v) => write!(f, "Set ambient volume to {:.0}%", v * 100.0),
            Self::SetMaxSimultaneousSounds(n) => write!(f, "Set max sounds to {}", n),
            Self::SetAudioBackend(b) => write!(f, "Set audio backend to {}", b),
            Self::AddQualityLevel => write!(f, "Add quality level"),
            Self::RemoveQualityLevel(i) => write!(f, "Remove quality level {}", i),
            Self::SelectQualityLevel(i) => write!(f, "Select quality level {}", i),
            Self::SetQualityLevelName { index, name } => {
                write!(f, "Rename quality {} to '{}'", index, name)
            }
            Self::ApplyQualityLevel(i) => write!(f, "Apply quality level {}", i),
            Self::AddInputAction => write!(f, "Add input action"),
            Self::RemoveInputAction(i) => write!(f, "Remove input action {}", i),
            Self::SelectInputAction(i) => write!(f, "Select input action {}", i),
            Self::SetInputActionName { index, name } => {
                write!(f, "Rename action {} to '{}'", index, name)
            }
            Self::SetInputActionKey { index, key } => {
                write!(f, "Set action {} key to '{}'", index, key)
            }
            Self::AddTag(name) => write!(f, "Add tag '{}'", name),
            Self::RemoveTag(id) => write!(f, "Remove tag {}", id),
            Self::AddLayer(name) => write!(f, "Add layer '{}'", name),
            Self::RemoveLayer(id) => write!(f, "Remove layer {}", id),
            Self::AddBuildConfig => write!(f, "Add build config"),
            Self::RemoveBuildConfig(i) => write!(f, "Remove build config {}", i),
            Self::SelectBuildConfig(i) => write!(f, "Select build config {}", i),
            Self::SetBuildPlatform { platform, .. } => write!(f, "Set platform to {}", platform),
            Self::ToggleBuildDevelopment { development, .. } => {
                write!(
                    f,
                    "Set development mode {}",
                    if *development { "on" } else { "off" }
                )
            }
            Self::StartBuild(i) => write!(f, "Start build {}", i),
            Self::SaveSettings => write!(f, "Save settings"),
            Self::LoadSettings => write!(f, "Load settings"),
            Self::ResetToDefaults => write!(f, "Reset to defaults"),
            Self::ExportSettings(path) => write!(f, "Export settings to '{}'", path),
            Self::ImportSettings(path) => write!(f, "Import settings from '{}'", path),
        }
    }
}

impl std::fmt::Display for SettingsTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SettingsTab {
    pub fn name(&self) -> &'static str {
        match self {
            SettingsTab::Project => "Project",
            SettingsTab::Gameplay => "Gameplay",
            SettingsTab::Rendering => "Rendering",
            SettingsTab::Physics => "Physics",
            SettingsTab::Audio => "Audio",
            SettingsTab::Input => "Input",
            SettingsTab::Quality => "Quality",
            SettingsTab::TagsLayers => "Tags & Layers",
            SettingsTab::Build => "Build",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SettingsTab::Project => "[Dir]",
            SettingsTab::Gameplay => "[Sword]",
            SettingsTab::Rendering => "[Art]",
            SettingsTab::Physics => "⚙️",
            SettingsTab::Audio => "[Snd]",
            SettingsTab::Input => "[Gp]",
            SettingsTab::Quality => "[Fx]",
            SettingsTab::TagsLayers => "[Tag]",
            SettingsTab::Build => "[Pkg]",
        }
    }

    pub fn all() -> &'static [SettingsTab] {
        &[
            SettingsTab::Project,
            SettingsTab::Gameplay,
            SettingsTab::Rendering,
            SettingsTab::Physics,
            SettingsTab::Audio,
            SettingsTab::Input,
            SettingsTab::Quality,
            SettingsTab::TagsLayers,
            SettingsTab::Build,
        ]
    }
}

/// Main Project Settings Panel
pub struct ProjectSettingsPanel {
    active_tab: SettingsTab,

    // Project
    project_name: String,
    project_version: String,
    company_name: String,
    default_scene: String,

    // Gameplay genre preset
    gameplay_preset: super::gameplay_presets::GameplayPreset,

    // Settings
    rendering_settings: RenderingSettings,
    physics_settings: PhysicsSettings,
    audio_settings: AudioSettings,

    // Quality levels
    quality_levels: Vec<QualityLevel>,
    selected_quality: usize,

    // Input
    input_actions: Vec<InputAction>,
    selected_action: Option<usize>,

    // Tags and layers
    tags: Vec<Tag>,
    layers: Vec<Layer>,
    next_tag_id: u32,
    next_layer_id: u32,

    // Build
    build_configs: Vec<BuildConfig>,
    selected_build: usize,

    // Action queue for external processing
    pending_actions: Vec<ProjectSettingsAction>,
}

impl Default for ProjectSettingsPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: SettingsTab::Project,

            project_name: "AstraWeave Project".to_string(),
            project_version: "0.1.0".to_string(),
            company_name: "AstraWeave Studios".to_string(),
            default_scene: "scenes/main.scene".to_string(),

            gameplay_preset: super::gameplay_presets::GameplayPreset::Custom,

            rendering_settings: RenderingSettings::default(),
            physics_settings: PhysicsSettings::default(),
            audio_settings: AudioSettings::default(),

            quality_levels: Vec::new(),
            selected_quality: 0,

            input_actions: Vec::new(),
            selected_action: None,

            tags: Vec::new(),
            layers: Vec::new(),
            next_tag_id: 1,
            next_layer_id: 1,

            build_configs: Vec::new(),
            selected_build: 0,

            pending_actions: Vec::new(),
        };

        panel
    }
}

impl ProjectSettingsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    // ==================== Action Queue Methods ====================

    /// Takes all pending actions, leaving the internal queue empty
    pub fn take_actions(&mut self) -> Vec<ProjectSettingsAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for external processing
    pub fn queue_action(&mut self, action: ProjectSettingsAction) {
        self.pending_actions.push(action);
    }

    /// Returns a reference to pending actions
    pub fn pending_actions(&self) -> &[ProjectSettingsAction] {
        &self.pending_actions
    }

    fn create_sample_data(&mut self) {
        // Quality levels
        self.quality_levels = vec![
            QualityLevel {
                name: "Low".to_string(),
                shadow_resolution: 512,
                shadow_cascades: 2,
                texture_quality: TextureQuality::Low,
                antialiasing: AntialiasingMode::None,
                vsync: false,
                max_fps: 30,
                lod_bias: 0.5,
                particle_density: 0.25,
            },
            QualityLevel {
                name: "Medium".to_string(),
                shadow_resolution: 1024,
                shadow_cascades: 3,
                texture_quality: TextureQuality::Medium,
                antialiasing: AntialiasingMode::Fxaa,
                vsync: true,
                max_fps: 60,
                lod_bias: 1.0,
                particle_density: 0.5,
            },
            QualityLevel {
                name: "High".to_string(),
                shadow_resolution: 2048,
                shadow_cascades: 4,
                texture_quality: TextureQuality::High,
                antialiasing: AntialiasingMode::Smaa,
                vsync: true,
                max_fps: 60,
                lod_bias: 1.5,
                particle_density: 1.0,
            },
            QualityLevel {
                name: "Ultra".to_string(),
                shadow_resolution: 4096,
                shadow_cascades: 4,
                texture_quality: TextureQuality::Ultra,
                antialiasing: AntialiasingMode::Taa,
                vsync: false,
                max_fps: 0, // Unlimited
                lod_bias: 2.0,
                particle_density: 1.5,
            },
        ];

        // Input actions
        self.input_actions = vec![
            InputAction {
                name: "MoveForward".to_string(),
                primary_key: "W".to_string(),
                secondary_key: "Up".to_string(),
                gamepad_button: "LeftStick-Y+".to_string(),
                dead_zone: 0.1,
            },
            InputAction {
                name: "MoveBackward".to_string(),
                primary_key: "S".to_string(),
                secondary_key: "Down".to_string(),
                gamepad_button: "LeftStick-Y-".to_string(),
                dead_zone: 0.1,
            },
            InputAction {
                name: "MoveLeft".to_string(),
                primary_key: "A".to_string(),
                secondary_key: "Left".to_string(),
                gamepad_button: "LeftStick-X-".to_string(),
                dead_zone: 0.1,
            },
            InputAction {
                name: "MoveRight".to_string(),
                primary_key: "D".to_string(),
                secondary_key: "Right".to_string(),
                gamepad_button: "LeftStick-X+".to_string(),
                dead_zone: 0.1,
            },
            InputAction {
                name: "Jump".to_string(),
                primary_key: "Space".to_string(),
                secondary_key: "".to_string(),
                gamepad_button: "A".to_string(),
                dead_zone: 0.0,
            },
            InputAction {
                name: "Attack".to_string(),
                primary_key: "Mouse1".to_string(),
                secondary_key: "".to_string(),
                gamepad_button: "RightTrigger".to_string(),
                dead_zone: 0.2,
            },
            InputAction {
                name: "Interact".to_string(),
                primary_key: "E".to_string(),
                secondary_key: "".to_string(),
                gamepad_button: "X".to_string(),
                dead_zone: 0.0,
            },
        ];

        // Tags
        self.tags = vec![
            Tag {
                id: self.next_tag_id(),
                name: "Player".to_string(),
                color: Color32::from_rgb(50, 200, 50),
            },
            Tag {
                id: self.next_tag_id(),
                name: "Enemy".to_string(),
                color: Color32::from_rgb(200, 50, 50),
            },
            Tag {
                id: self.next_tag_id(),
                name: "NPC".to_string(),
                color: Color32::from_rgb(50, 100, 200),
            },
            Tag {
                id: self.next_tag_id(),
                name: "Collectible".to_string(),
                color: Color32::from_rgb(200, 200, 50),
            },
            Tag {
                id: self.next_tag_id(),
                name: "Interactable".to_string(),
                color: Color32::from_rgb(150, 100, 200),
            },
        ];

        // Layers
        self.layers = vec![
            Layer {
                id: self.next_layer_id(),
                name: "Default".to_string(),
                collides_with: vec![1, 2, 3, 4, 5],
            },
            Layer {
                id: self.next_layer_id(),
                name: "Player".to_string(),
                collides_with: vec![1, 2, 3, 5],
            },
            Layer {
                id: self.next_layer_id(),
                name: "Enemy".to_string(),
                collides_with: vec![1, 2, 3, 5],
            },
            Layer {
                id: self.next_layer_id(),
                name: "Trigger".to_string(),
                collides_with: vec![2, 3],
            },
            Layer {
                id: self.next_layer_id(),
                name: "Projectile".to_string(),
                collides_with: vec![1, 2, 3],
            },
        ];

        // Build configs
        self.build_configs = vec![
            BuildConfig {
                platform: TargetPlatform::Windows,
                enabled: true,
                development_build: true,
                ..Default::default()
            },
            BuildConfig {
                platform: TargetPlatform::Linux,
                enabled: true,
                development_build: true,
                ..Default::default()
            },
            BuildConfig {
                platform: TargetPlatform::MacOS,
                enabled: false,
                ..Default::default()
            },
            BuildConfig {
                platform: TargetPlatform::WebAssembly,
                enabled: true,
                development_build: false,
                ..Default::default()
            },
        ];
    }

    fn next_tag_id(&mut self) -> u32 {
        let id = self.next_tag_id;
        self.next_tag_id += 1;
        id
    }

    fn next_layer_id(&mut self) -> u32 {
        let id = self.next_layer_id;
        self.next_layer_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (SettingsTab::Project, "Project"),
                (SettingsTab::Rendering, "Rendering"),
                (SettingsTab::Physics, "[Test] Physics"),
                (SettingsTab::Audio, "Audio"),
                (SettingsTab::Input, "Input"),
                (SettingsTab::Quality, "[Star] Quality"),
                (SettingsTab::TagsLayers, "[Tag] Tags/Layers"),
                (SettingsTab::Build, "Build"),
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

        ui.separator();
    }

    fn show_project_tab(&mut self, ui: &mut Ui) {
        ui.heading("Project Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Project Info").strong());

            egui::Grid::new("project_info")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Project Name:");
                    ui.text_edit_singleline(&mut self.project_name);
                    ui.end_row();

                    ui.label("Version:");
                    ui.text_edit_singleline(&mut self.project_version);
                    ui.end_row();

                    ui.label("Company:");
                    ui.text_edit_singleline(&mut self.company_name);
                    ui.end_row();

                    ui.label("Default Scene:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.default_scene);
                        if ui.button("[Open]").clicked() {}
                    });
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Engine Settings").strong());

            egui::Grid::new("engine_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Renderer Backend:");
                    egui::ComboBox::from_id_salt("renderer_backend")
                        .selected_text(format!("{:?}", self.rendering_settings.renderer_backend))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.rendering_settings.renderer_backend,
                                RendererBackend::Auto,
                                "Auto",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.renderer_backend,
                                RendererBackend::Vulkan,
                                "Vulkan",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.renderer_backend,
                                RendererBackend::DirectX12,
                                "DirectX 12",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.renderer_backend,
                                RendererBackend::Metal,
                                "Metal",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.renderer_backend,
                                RendererBackend::OpenGL,
                                "OpenGL",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.renderer_backend,
                                RendererBackend::WebGpu,
                                "WebGPU",
                            );
                        });
                    ui.end_row();

                    ui.label("Audio Backend:");
                    egui::ComboBox::from_id_salt("audio_backend")
                        .selected_text(format!("{:?}", self.audio_settings.audio_backend))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.audio_settings.audio_backend,
                                AudioBackend::Auto,
                                "Auto",
                            );
                            ui.selectable_value(
                                &mut self.audio_settings.audio_backend,
                                AudioBackend::Wasapi,
                                "WASAPI",
                            );
                            ui.selectable_value(
                                &mut self.audio_settings.audio_backend,
                                AudioBackend::CoreAudio,
                                "Core Audio",
                            );
                            ui.selectable_value(
                                &mut self.audio_settings.audio_backend,
                                AudioBackend::Alsa,
                                "ALSA",
                            );
                            ui.selectable_value(
                                &mut self.audio_settings.audio_backend,
                                AudioBackend::PulseAudio,
                                "PulseAudio",
                            );
                        });
                    ui.end_row();
                });
        });
    }

    fn show_gameplay_tab(&mut self, ui: &mut Ui) {
        ui.heading("Gameplay Template");
        ui.add_space(10.0);

        if let Some(new_preset) =
            super::gameplay_presets::show_preset_selector(ui, self.gameplay_preset)
        {
            self.gameplay_preset = new_preset;
        }

        // Show component schema details for the selected preset
        if self.gameplay_preset != super::gameplay_presets::GameplayPreset::Custom {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(RichText::new("Component Schemas").strong());
                ui.add_space(4.0);
                for schema in self.gameplay_preset.component_schemas() {
                    egui::CollapsingHeader::new(RichText::new(schema.name).strong().monospace())
                        .default_open(false)
                        .show(ui, |ui| {
                            for (field_name, field_type) in schema.fields {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(*field_name)
                                            .monospace()
                                            .color(egui::Color32::from_rgb(140, 190, 255)),
                                    );
                                    ui.label(
                                        RichText::new(format!(": {}", field_type))
                                            .monospace()
                                            .color(egui::Color32::from_rgb(160, 160, 175)),
                                    );
                                });
                            }
                        });
                }
            });

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(RichText::new("Starter Entities").strong());
                ui.add_space(4.0);
                for entity in self.gameplay_preset.starter_entities() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(entity.name)
                                .strong()
                                .color(egui::Color32::from_rgb(200, 200, 210)),
                        );
                        ui.label(
                            RichText::new(format!("({})", entity.archetype))
                                .color(egui::Color32::from_rgb(140, 140, 155)),
                        );
                    });
                    ui.label(
                        RichText::new(entity.description)
                            .small()
                            .color(egui::Color32::from_rgb(120, 120, 130)),
                    );
                    ui.add_space(2.0);
                }
            });
        }
    }

    fn show_rendering_tab(&mut self, ui: &mut Ui) {
        ui.heading("Rendering Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("🔆 HDR & Post-Processing").strong());

            egui::Grid::new("hdr_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("HDR:");
                    ui.checkbox(&mut self.rendering_settings.hdr_enabled, "Enabled");
                    ui.end_row();

                    ui.label("Bloom:");
                    ui.checkbox(&mut self.rendering_settings.bloom_enabled, "Enabled");
                    ui.end_row();

                    if self.rendering_settings.bloom_enabled {
                        ui.label("Bloom Intensity:");
                        ui.add(egui::Slider::new(
                            &mut self.rendering_settings.bloom_intensity,
                            0.0..=2.0,
                        ));
                        ui.end_row();
                    }

                    ui.label("Tonemapping:");
                    egui::ComboBox::from_id_salt("tonemapping")
                        .selected_text(format!("{:?}", self.rendering_settings.tonemapping))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.rendering_settings.tonemapping,
                                TonemappingMode::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.tonemapping,
                                TonemappingMode::Reinhard,
                                "Reinhard",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.tonemapping,
                                TonemappingMode::Aces,
                                "ACES",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.tonemapping,
                                TonemappingMode::AgX,
                                "AgX",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.tonemapping,
                                TonemappingMode::Filmic,
                                "Filmic",
                            );
                        });
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Lighting").strong());

            egui::Grid::new("lighting_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Ambient Occlusion:");
                    egui::ComboBox::from_id_salt("ao_mode")
                        .selected_text(format!("{:?}", self.rendering_settings.ambient_occlusion))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.rendering_settings.ambient_occlusion,
                                AoMode::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.ambient_occlusion,
                                AoMode::Ssao,
                                "SSAO",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.ambient_occlusion,
                                AoMode::Hbao,
                                "HBAO+",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.ambient_occlusion,
                                AoMode::Gtao,
                                "GTAO",
                            );
                        });
                    ui.end_row();

                    ui.label("Global Illumination:");
                    egui::ComboBox::from_id_salt("gi_mode")
                        .selected_text(format!("{:?}", self.rendering_settings.global_illumination))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.rendering_settings.global_illumination,
                                GiMode::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.global_illumination,
                                GiMode::ScreenSpace,
                                "Screen Space",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.global_illumination,
                                GiMode::Lumen,
                                "Lumen",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.global_illumination,
                                GiMode::PathTraced,
                                "Path Traced",
                            );
                        });
                    ui.end_row();

                    ui.label("Reflections:");
                    egui::ComboBox::from_id_salt("reflection_mode")
                        .selected_text(format!("{:?}", self.rendering_settings.reflection_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.rendering_settings.reflection_mode,
                                ReflectionMode::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.reflection_mode,
                                ReflectionMode::ScreenSpace,
                                "Screen Space",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.reflection_mode,
                                ReflectionMode::Raytraced,
                                "Ray Traced",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.reflection_mode,
                                ReflectionMode::Hybrid,
                                "Hybrid",
                            );
                        });
                    ui.end_row();

                    ui.label("Shadows:");
                    egui::ComboBox::from_id_salt("shadow_mode")
                        .selected_text(format!("{:?}", self.rendering_settings.shadow_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.rendering_settings.shadow_mode,
                                ShadowMode::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.shadow_mode,
                                ShadowMode::HardShadows,
                                "Hard",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.shadow_mode,
                                ShadowMode::SoftShadows,
                                "Soft",
                            );
                            ui.selectable_value(
                                &mut self.rendering_settings.shadow_mode,
                                ShadowMode::Raytraced,
                                "Ray Traced",
                            );
                        });
                    ui.end_row();
                });
        });
    }

    fn show_physics_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Test] Physics Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("[Glb] World").strong());

            egui::Grid::new("physics_world")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Gravity X:");
                    ui.add(egui::DragValue::new(&mut self.physics_settings.gravity[0]).speed(0.1));
                    ui.end_row();

                    ui.label("Gravity Y:");
                    ui.add(egui::DragValue::new(&mut self.physics_settings.gravity[1]).speed(0.1));
                    ui.end_row();

                    ui.label("Gravity Z:");
                    ui.add(egui::DragValue::new(&mut self.physics_settings.gravity[2]).speed(0.1));
                    ui.end_row();

                    ui.label("Fixed Timestep:");
                    ui.add(
                        egui::DragValue::new(&mut self.physics_settings.fixed_timestep)
                            .speed(0.001)
                            .suffix("s"),
                    );
                    ui.end_row();

                    ui.label("Max Substeps:");
                    ui.add(egui::Slider::new(
                        &mut self.physics_settings.max_substeps,
                        1..=16,
                    ));
                    ui.end_row();

                    ui.label("Broadphase:");
                    egui::ComboBox::from_id_salt("broadphase")
                        .selected_text(format!("{:?}", self.physics_settings.broadphase))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.physics_settings.broadphase,
                                BroadphaseType::Sap,
                                "SAP",
                            );
                            ui.selectable_value(
                                &mut self.physics_settings.broadphase,
                                BroadphaseType::DynamicAabb,
                                "Dynamic AABB",
                            );
                            ui.selectable_value(
                                &mut self.physics_settings.broadphase,
                                BroadphaseType::Quadtree,
                                "Quadtree",
                            );
                        });
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("📏 Defaults").strong());

            egui::Grid::new("physics_defaults")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Default Friction:");
                    ui.add(egui::Slider::new(
                        &mut self.physics_settings.default_friction,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    ui.label("Default Restitution:");
                    ui.add(egui::Slider::new(
                        &mut self.physics_settings.default_restitution,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    ui.label("Sleep Threshold:");
                    ui.add(
                        egui::DragValue::new(&mut self.physics_settings.sleep_threshold)
                            .speed(0.01),
                    );
                    ui.end_row();

                    ui.label("CCD:");
                    ui.checkbox(
                        &mut self.physics_settings.enable_ccd,
                        "Enable Continuous Collision Detection",
                    );
                    ui.end_row();
                });
        });
    }

    fn show_audio_tab(&mut self, ui: &mut Ui) {
        ui.heading("Audio Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("🎚️ Volume").strong());

            egui::Grid::new("audio_volumes")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Master Volume:");
                    ui.add(egui::Slider::new(
                        &mut self.audio_settings.master_volume,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    ui.label("Music Volume:");
                    ui.add(egui::Slider::new(
                        &mut self.audio_settings.music_volume,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    ui.label("SFX Volume:");
                    ui.add(egui::Slider::new(
                        &mut self.audio_settings.sfx_volume,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    ui.label("Voice Volume:");
                    ui.add(egui::Slider::new(
                        &mut self.audio_settings.voice_volume,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    ui.label("Ambient Volume:");
                    ui.add(egui::Slider::new(
                        &mut self.audio_settings.ambient_volume,
                        0.0..=1.0,
                    ));
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Settings").strong());

            egui::Grid::new("audio_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Max Simultaneous:");
                    ui.add(egui::Slider::new(
                        &mut self.audio_settings.max_simultaneous_sounds,
                        16..=256,
                    ));
                    ui.end_row();

                    ui.label("Doppler Factor:");
                    ui.add(egui::Slider::new(
                        &mut self.audio_settings.doppler_factor,
                        0.0..=2.0,
                    ));
                    ui.end_row();
                });
        });
    }

    fn show_input_tab(&mut self, ui: &mut Ui) {
        ui.heading("Input Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Actions").strong());
                if ui.button("+ Add Action").clicked() {
                    self.input_actions.push(InputAction {
                        name: format!("NewAction{}", self.input_actions.len()),
                        ..Default::default()
                    });
                }
            });

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    egui::Grid::new("input_actions")
                        .num_columns(5)
                        .spacing([10.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Header
                            ui.label(RichText::new("Action").strong());
                            ui.label(RichText::new("Primary Key").strong());
                            ui.label(RichText::new("Secondary").strong());
                            ui.label(RichText::new("Gamepad").strong());
                            ui.label(RichText::new("Dead Zone").strong());
                            ui.end_row();

                            for action in &mut self.input_actions {
                                ui.text_edit_singleline(&mut action.name);
                                ui.text_edit_singleline(&mut action.primary_key);
                                ui.text_edit_singleline(&mut action.secondary_key);
                                ui.text_edit_singleline(&mut action.gamepad_button);
                                ui.add(
                                    egui::DragValue::new(&mut action.dead_zone)
                                        .speed(0.01)
                                        .range(0.0..=1.0),
                                );
                                ui.end_row();
                            }
                        });
                });
        });
    }

    fn show_quality_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Star] Quality Levels");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            // Quality level selector
            for (i, level) in self.quality_levels.iter().enumerate() {
                let is_selected = self.selected_quality == i;
                let button = egui::Button::new(&level.name).fill(if is_selected {
                    Color32::from_rgb(60, 100, 160)
                } else {
                    Color32::from_rgb(50, 50, 55)
                });

                if ui.add(button).clicked() {
                    self.selected_quality = i;
                }
            }

            if ui.button("+ Add Level").clicked() {
                self.quality_levels.push(QualityLevel {
                    name: format!("Custom {}", self.quality_levels.len()),
                    ..Default::default()
                });
            }
        });

        ui.add_space(10.0);

        if let Some(level) = self.quality_levels.get_mut(self.selected_quality) {
            ui.group(|ui| {
                egui::Grid::new("quality_settings")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut level.name);
                        ui.end_row();

                        ui.label("Shadow Resolution:");
                        egui::ComboBox::from_id_salt("shadow_res")
                            .selected_text(format!("{}", level.shadow_resolution))
                            .show_ui(ui, |ui| {
                                for res in [256, 512, 1024, 2048, 4096, 8192] {
                                    ui.selectable_value(
                                        &mut level.shadow_resolution,
                                        res,
                                        format!("{}", res),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Shadow Cascades:");
                        ui.add(egui::Slider::new(&mut level.shadow_cascades, 1..=6));
                        ui.end_row();

                        ui.label("Texture Quality:");
                        egui::ComboBox::from_id_salt("tex_quality")
                            .selected_text(format!("{:?}", level.texture_quality))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut level.texture_quality,
                                    TextureQuality::Low,
                                    "Low",
                                );
                                ui.selectable_value(
                                    &mut level.texture_quality,
                                    TextureQuality::Medium,
                                    "Medium",
                                );
                                ui.selectable_value(
                                    &mut level.texture_quality,
                                    TextureQuality::High,
                                    "High",
                                );
                                ui.selectable_value(
                                    &mut level.texture_quality,
                                    TextureQuality::Ultra,
                                    "Ultra",
                                );
                            });
                        ui.end_row();

                        ui.label("Antialiasing:");
                        egui::ComboBox::from_id_salt("aa_mode")
                            .selected_text(format!("{:?}", level.antialiasing))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut level.antialiasing,
                                    AntialiasingMode::None,
                                    "None",
                                );
                                ui.selectable_value(
                                    &mut level.antialiasing,
                                    AntialiasingMode::Fxaa,
                                    "FXAA",
                                );
                                ui.selectable_value(
                                    &mut level.antialiasing,
                                    AntialiasingMode::Smaa,
                                    "SMAA",
                                );
                                ui.selectable_value(
                                    &mut level.antialiasing,
                                    AntialiasingMode::Taa,
                                    "TAA",
                                );
                                ui.selectable_value(
                                    &mut level.antialiasing,
                                    AntialiasingMode::Msaa2x,
                                    "MSAA 2x",
                                );
                                ui.selectable_value(
                                    &mut level.antialiasing,
                                    AntialiasingMode::Msaa4x,
                                    "MSAA 4x",
                                );
                                ui.selectable_value(
                                    &mut level.antialiasing,
                                    AntialiasingMode::Msaa8x,
                                    "MSAA 8x",
                                );
                            });
                        ui.end_row();

                        ui.label("VSync:");
                        ui.checkbox(&mut level.vsync, "");
                        ui.end_row();

                        ui.label("Max FPS:");
                        ui.add(
                            egui::DragValue::new(&mut level.max_fps)
                                .speed(1)
                                .suffix(" (0=unlimited)"),
                        );
                        ui.end_row();

                        ui.label("LOD Bias:");
                        ui.add(egui::Slider::new(&mut level.lod_bias, 0.5..=2.0));
                        ui.end_row();

                        ui.label("Particle Density:");
                        ui.add(egui::Slider::new(&mut level.particle_density, 0.0..=2.0));
                        ui.end_row();
                    });
            });
        }
    }

    fn show_tags_layers_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Tag] Tags & Layers");
        ui.add_space(10.0);

        ui.columns(2, |cols| {
            // Tags
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("[Tag] Tags").strong());
                    if ui.button("+ Add").clicked() {
                        let id = self.next_tag_id();
                        self.tags.push(Tag {
                            id,
                            name: format!("Tag{}", id),
                            color: Color32::GRAY,
                        });
                    }
                });

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for tag in &mut self.tags {
                            ui.horizontal(|ui| {
                                ui.color_edit_button_srgba(&mut tag.color);
                                ui.text_edit_singleline(&mut tag.name);
                                if ui.button("[Del]").clicked() {
                                    // Remove tag
                                }
                            });
                        }
                    });
            });

            // Layers
            cols[1].group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Layers").strong());
                    if ui.button("+ Add").clicked() {
                        let id = self.next_layer_id();
                        self.layers.push(Layer {
                            id,
                            name: format!("Layer{}", id),
                            collides_with: Vec::new(),
                        });
                    }
                });

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for layer in &mut self.layers {
                            ui.horizontal(|ui| {
                                ui.label(format!("[{}]", layer.id));
                                ui.text_edit_singleline(&mut layer.name);
                                if ui.button("[Del]").clicked() {
                                    // Remove layer
                                }
                            });
                        }
                    });
            });
        });
    }

    fn show_build_tab(&mut self, ui: &mut Ui) {
        ui.heading("Build Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Target Platforms").strong());

            for config in &mut self.build_configs {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut config.enabled, "");
                    ui.label(format!("{} {:?}", config.platform.icon(), config.platform));
                    ui.checkbox(&mut config.development_build, "Development");
                });
            }
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("[Edit] Application Info").strong());

            if let Some(config) = self.build_configs.get_mut(self.selected_build) {
                egui::Grid::new("build_info")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("App Name:");
                        ui.text_edit_singleline(&mut config.app_name);
                        ui.end_row();

                        ui.label("Version:");
                        ui.text_edit_singleline(&mut config.version);
                        ui.end_row();

                        ui.label("Company:");
                        ui.text_edit_singleline(&mut config.company);
                        ui.end_row();

                        ui.label("Icon:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut config.icon_path);
                            if ui.button("[Open]").clicked() {}
                        });
                        ui.end_row();

                        ui.label("Compression:");
                        egui::ComboBox::from_id_salt("compression")
                            .selected_text(format!("{:?}", config.compression))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut config.compression,
                                    CompressionMode::None,
                                    "None",
                                );
                                ui.selectable_value(
                                    &mut config.compression,
                                    CompressionMode::Fast,
                                    "Fast",
                                );
                                ui.selectable_value(
                                    &mut config.compression,
                                    CompressionMode::Best,
                                    "Best",
                                );
                            });
                        ui.end_row();
                    });
            }
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("🔨 Build").clicked() {
                self.pending_actions
                    .push(ProjectSettingsAction::StartBuild(self.selected_build));
            }
            if ui.button("🔨 Build and Run").clicked() {
                // Start a build; the runner will detect that it should also launch
                self.pending_actions
                    .push(ProjectSettingsAction::StartBuild(self.selected_build));
            }
            if ui.button("Open Build Folder").clicked() {
                let output_dir = std::path::PathBuf::from("target/build_output");
                if output_dir.exists() {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("explorer")
                            .arg(&output_dir)
                            .spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg(&output_dir).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open")
                            .arg(&output_dir)
                            .spawn();
                    }
                }
            }
        });
    }

    // Getters for testing
    pub fn quality_level_count(&self) -> usize {
        self.quality_levels.len()
    }

    pub fn input_action_count(&self) -> usize {
        self.input_actions.len()
    }

    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn set_project_name(&mut self, name: &str) {
        self.project_name = name.to_string();
    }
}

impl Panel for ProjectSettingsPanel {
    fn name(&self) -> &'static str {
        "Project Settings"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            SettingsTab::Project => self.show_project_tab(ui),
            SettingsTab::Gameplay => self.show_gameplay_tab(ui),
            SettingsTab::Rendering => self.show_rendering_tab(ui),
            SettingsTab::Physics => self.show_physics_tab(ui),
            SettingsTab::Audio => self.show_audio_tab(ui),
            SettingsTab::Input => self.show_input_tab(ui),
            SettingsTab::Quality => self.show_quality_tab(ui),
            SettingsTab::TagsLayers => self.show_tags_layers_tab(ui),
            SettingsTab::Build => self.show_build_tab(ui),
        }
    }

    fn update(&mut self) {
        // No specific update logic needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // TARGET PLATFORM TESTS
    // ============================================================

    #[test]
    fn test_target_platform_all_variants() {
        let variants = [
            TargetPlatform::Windows,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Android,
            TargetPlatform::Ios,
            TargetPlatform::WebAssembly,
            TargetPlatform::PlayStation,
            TargetPlatform::Xbox,
            TargetPlatform::NintendoSwitch,
        ];
        assert_eq!(variants.len(), 9);
    }

    #[test]
    fn test_platform_icons() {
        assert_eq!(TargetPlatform::Windows.icon(), "🪟");
        assert_eq!(TargetPlatform::Linux.icon(), "🐧");
    }

    #[test]
    fn test_platform_icon_all() {
        assert_eq!(TargetPlatform::Windows.icon(), "🪟");
        assert_eq!(TargetPlatform::Linux.icon(), "🐧");
        assert_eq!(TargetPlatform::MacOS.icon(), "🍎");
        assert_eq!(TargetPlatform::Android.icon(), "[Bot]");
        assert_eq!(TargetPlatform::Ios.icon(), "📱");
        assert_eq!(TargetPlatform::WebAssembly.icon(), "[Net]");
        assert_eq!(TargetPlatform::PlayStation.icon(), "[Gp]");
        assert_eq!(TargetPlatform::Xbox.icon(), "[Gp]");
        assert_eq!(TargetPlatform::NintendoSwitch.icon(), "[Gp]");
    }

    #[test]
    fn test_target_platform_clone() {
        let platform = TargetPlatform::Linux;
        let cloned = platform;
        assert_eq!(cloned, TargetPlatform::Linux);
    }

    // ============================================================
    // TEXTURE QUALITY TESTS
    // ============================================================

    #[test]
    fn test_texture_quality_default() {
        let quality = TextureQuality::default();
        assert_eq!(quality, TextureQuality::High);
    }

    #[test]
    fn test_texture_quality_all_variants() {
        let variants = [
            TextureQuality::Low,
            TextureQuality::Medium,
            TextureQuality::High,
            TextureQuality::Ultra,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_texture_quality_clone() {
        let quality = TextureQuality::Ultra;
        let cloned = quality;
        assert_eq!(cloned, TextureQuality::Ultra);
    }

    // ============================================================
    // ANTIALIASING MODE TESTS
    // ============================================================

    #[test]
    fn test_antialiasing_mode_default() {
        let aa = AntialiasingMode::default();
        assert_eq!(aa, AntialiasingMode::Smaa);
    }

    #[test]
    fn test_antialiasing_mode_all_variants() {
        let variants = [
            AntialiasingMode::None,
            AntialiasingMode::Fxaa,
            AntialiasingMode::Smaa,
            AntialiasingMode::Taa,
            AntialiasingMode::Msaa2x,
            AntialiasingMode::Msaa4x,
            AntialiasingMode::Msaa8x,
        ];
        assert_eq!(variants.len(), 7);
    }

    #[test]
    fn test_antialiasing_mode_clone() {
        let aa = AntialiasingMode::Taa;
        let cloned = aa;
        assert_eq!(cloned, AntialiasingMode::Taa);
    }

    // ============================================================
    // QUALITY LEVEL TESTS
    // ============================================================

    #[test]
    fn test_quality_level_default() {
        let level = QualityLevel::default();
        assert_eq!(level.name, "Medium");
        assert_eq!(level.shadow_resolution, 2048);
        assert_eq!(level.shadow_cascades, 4);
        assert_eq!(level.texture_quality, TextureQuality::High);
        assert_eq!(level.antialiasing, AntialiasingMode::Smaa);
        assert!(level.vsync);
        assert_eq!(level.max_fps, 60);
        assert_eq!(level.lod_bias, 1.0);
        assert_eq!(level.particle_density, 1.0);
    }

    #[test]
    fn test_quality_level_custom() {
        let level = QualityLevel {
            name: "Custom".to_string(),
            shadow_resolution: 4096,
            shadow_cascades: 6,
            texture_quality: TextureQuality::Ultra,
            antialiasing: AntialiasingMode::Msaa8x,
            vsync: false,
            max_fps: 144,
            lod_bias: 2.0,
            particle_density: 1.5,
        };
        assert_eq!(level.name, "Custom");
        assert_eq!(level.shadow_resolution, 4096);
        assert_eq!(level.max_fps, 144);
    }

    #[test]
    fn test_quality_level_clone() {
        let level = QualityLevel::default();
        let cloned = level.clone();
        assert_eq!(cloned.name, "Medium");
        assert_eq!(cloned.shadow_resolution, 2048);
    }

    // ============================================================
    // BROADPHASE TYPE TESTS
    // ============================================================

    #[test]
    fn test_broadphase_type_default() {
        let bp = BroadphaseType::default();
        assert_eq!(bp, BroadphaseType::Sap);
    }

    #[test]
    fn test_broadphase_type_all_variants() {
        let variants = [
            BroadphaseType::Sap,
            BroadphaseType::DynamicAabb,
            BroadphaseType::Quadtree,
        ];
        assert_eq!(variants.len(), 3);
    }

    // ============================================================
    // PHYSICS SETTINGS TESTS
    // ============================================================

    #[test]
    fn test_default_physics() {
        let settings = PhysicsSettings::default();
        assert_eq!(settings.gravity[1], -9.81);
    }

    #[test]
    fn test_physics_settings_default_full() {
        let settings = PhysicsSettings::default();
        assert_eq!(settings.gravity, [0.0, -9.81, 0.0]);
        assert!((settings.fixed_timestep - 1.0 / 60.0).abs() < 0.0001);
        assert_eq!(settings.max_substeps, 4);
        assert_eq!(settings.broadphase, BroadphaseType::Sap);
        assert_eq!(settings.default_friction, 0.5);
        assert_eq!(settings.default_restitution, 0.3);
        assert_eq!(settings.sleep_threshold, 0.1);
        assert!(settings.enable_ccd);
    }

    #[test]
    fn test_physics_settings_custom() {
        let settings = PhysicsSettings {
            gravity: [0.0, -15.0, 0.0],
            fixed_timestep: 1.0 / 120.0,
            max_substeps: 8,
            broadphase: BroadphaseType::DynamicAabb,
            default_friction: 0.8,
            default_restitution: 0.6,
            sleep_threshold: 0.05,
            enable_ccd: false,
        };
        assert_eq!(settings.gravity[1], -15.0);
        assert_eq!(settings.max_substeps, 8);
        assert!(!settings.enable_ccd);
    }

    #[test]
    fn test_physics_settings_clone() {
        let settings = PhysicsSettings::default();
        let cloned = settings.clone();
        assert_eq!(cloned.gravity, [0.0, -9.81, 0.0]);
    }

    // ============================================================
    // AUDIO BACKEND TESTS
    // ============================================================

    #[test]
    fn test_audio_backend_default() {
        let backend = AudioBackend::default();
        assert_eq!(backend, AudioBackend::Auto);
    }

    #[test]
    fn test_audio_backend_all_variants() {
        let variants = [
            AudioBackend::Auto,
            AudioBackend::Wasapi,
            AudioBackend::CoreAudio,
            AudioBackend::Alsa,
            AudioBackend::PulseAudio,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // AUDIO SETTINGS TESTS
    // ============================================================

    #[test]
    fn test_audio_settings_default() {
        let settings = AudioSettings::default();
        assert_eq!(settings.master_volume, 1.0);
        assert_eq!(settings.music_volume, 0.8);
        assert_eq!(settings.sfx_volume, 1.0);
        assert_eq!(settings.voice_volume, 1.0);
        assert_eq!(settings.ambient_volume, 0.7);
        assert_eq!(settings.max_simultaneous_sounds, 64);
        assert_eq!(settings.doppler_factor, 1.0);
        assert_eq!(settings.audio_backend, AudioBackend::Auto);
    }

    #[test]
    fn test_audio_settings_custom() {
        let settings = AudioSettings {
            master_volume: 0.5,
            music_volume: 0.3,
            sfx_volume: 0.8,
            voice_volume: 1.0,
            ambient_volume: 0.4,
            max_simultaneous_sounds: 128,
            doppler_factor: 0.5,
            audio_backend: AudioBackend::Wasapi,
        };
        assert_eq!(settings.master_volume, 0.5);
        assert_eq!(settings.max_simultaneous_sounds, 128);
        assert_eq!(settings.audio_backend, AudioBackend::Wasapi);
    }

    #[test]
    fn test_audio_settings_clone() {
        let settings = AudioSettings::default();
        let cloned = settings.clone();
        assert_eq!(cloned.master_volume, 1.0);
    }

    // ============================================================
    // RENDERER BACKEND TESTS
    // ============================================================

    #[test]
    fn test_renderer_backend_default() {
        let backend = RendererBackend::default();
        assert_eq!(backend, RendererBackend::Auto);
    }

    #[test]
    fn test_renderer_backend_all_variants() {
        let variants = [
            RendererBackend::Auto,
            RendererBackend::Vulkan,
            RendererBackend::DirectX12,
            RendererBackend::Metal,
            RendererBackend::OpenGL,
            RendererBackend::WebGpu,
        ];
        assert_eq!(variants.len(), 6);
    }

    // ============================================================
    // TONEMAPPING MODE TESTS
    // ============================================================

    #[test]
    fn test_tonemapping_mode_default() {
        let mode = TonemappingMode::default();
        assert_eq!(mode, TonemappingMode::Aces);
    }

    #[test]
    fn test_tonemapping_mode_all_variants() {
        let variants = [
            TonemappingMode::None,
            TonemappingMode::Reinhard,
            TonemappingMode::Aces,
            TonemappingMode::AgX,
            TonemappingMode::Filmic,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ============================================================
    // AO MODE TESTS
    // ============================================================

    #[test]
    fn test_ao_mode_default() {
        let mode = AoMode::default();
        assert_eq!(mode, AoMode::Hbao);
    }

    #[test]
    fn test_ao_mode_all_variants() {
        let variants = [AoMode::None, AoMode::Ssao, AoMode::Hbao, AoMode::Gtao];
        assert_eq!(variants.len(), 4);
    }

    // ============================================================
    // GI MODE TESTS
    // ============================================================

    #[test]
    fn test_gi_mode_default() {
        let mode = GiMode::default();
        assert_eq!(mode, GiMode::None);
    }

    #[test]
    fn test_gi_mode_all_variants() {
        let variants = [
            GiMode::None,
            GiMode::ScreenSpace,
            GiMode::Lumen,
            GiMode::PathTraced,
        ];
        assert_eq!(variants.len(), 4);
    }

    // ============================================================
    // REFLECTION MODE TESTS
    // ============================================================

    #[test]
    fn test_reflection_mode_default() {
        let mode = ReflectionMode::default();
        assert_eq!(mode, ReflectionMode::ScreenSpace);
    }

    #[test]
    fn test_reflection_mode_all_variants() {
        let variants = [
            ReflectionMode::None,
            ReflectionMode::ScreenSpace,
            ReflectionMode::Raytraced,
            ReflectionMode::Hybrid,
        ];
        assert_eq!(variants.len(), 4);
    }

    // ============================================================
    // SHADOW MODE TESTS
    // ============================================================

    #[test]
    fn test_shadow_mode_default() {
        let mode = ShadowMode::default();
        assert_eq!(mode, ShadowMode::SoftShadows);
    }

    #[test]
    fn test_shadow_mode_all_variants() {
        let variants = [
            ShadowMode::None,
            ShadowMode::HardShadows,
            ShadowMode::SoftShadows,
            ShadowMode::Raytraced,
        ];
        assert_eq!(variants.len(), 4);
    }

    // ============================================================
    // RENDERING SETTINGS TESTS
    // ============================================================

    #[test]
    fn test_rendering_settings_default() {
        let settings = RenderingSettings::default();
        assert_eq!(settings.renderer_backend, RendererBackend::Auto);
        assert!(settings.hdr_enabled);
        assert!(settings.bloom_enabled);
        assert_eq!(settings.bloom_intensity, 0.5);
        assert_eq!(settings.tonemapping, TonemappingMode::Aces);
        assert_eq!(settings.ambient_occlusion, AoMode::Hbao);
        assert_eq!(settings.global_illumination, GiMode::ScreenSpace);
        assert_eq!(settings.reflection_mode, ReflectionMode::ScreenSpace);
        assert_eq!(settings.shadow_mode, ShadowMode::SoftShadows);
    }

    #[test]
    fn test_rendering_settings_clone() {
        let settings = RenderingSettings::default();
        let cloned = settings.clone();
        assert_eq!(cloned.renderer_backend, RendererBackend::Auto);
        assert!(cloned.hdr_enabled);
    }

    // ============================================================
    // INPUT ACTION TESTS
    // ============================================================

    #[test]
    fn test_input_action_default() {
        let action = InputAction::default();
        assert!(action.name.is_empty());
        assert!(action.primary_key.is_empty());
        assert!(action.secondary_key.is_empty());
        assert!(action.gamepad_button.is_empty());
        assert_eq!(action.dead_zone, 0.1);
    }

    #[test]
    fn test_input_action_custom() {
        let action = InputAction {
            name: "Jump".to_string(),
            primary_key: "Space".to_string(),
            secondary_key: "W".to_string(),
            gamepad_button: "A".to_string(),
            dead_zone: 0.0,
        };
        assert_eq!(action.name, "Jump");
        assert_eq!(action.primary_key, "Space");
        assert_eq!(action.dead_zone, 0.0);
    }

    #[test]
    fn test_input_action_clone() {
        let action = InputAction {
            name: "Attack".to_string(),
            ..Default::default()
        };
        let cloned = action.clone();
        assert_eq!(cloned.name, "Attack");
    }

    // ============================================================
    // LAYER TESTS
    // ============================================================

    #[test]
    fn test_layer_creation() {
        let layer = Layer {
            id: 1,
            name: "Player".to_string(),
            collides_with: vec![1, 2, 3],
        };
        assert_eq!(layer.id, 1);
        assert_eq!(layer.name, "Player");
        assert_eq!(layer.collides_with.len(), 3);
    }

    #[test]
    fn test_layer_clone() {
        let layer = Layer {
            id: 5,
            name: "Environment".to_string(),
            collides_with: vec![1, 2],
        };
        let cloned = layer.clone();
        assert_eq!(cloned.id, 5);
        assert_eq!(cloned.name, "Environment");
    }

    // ============================================================
    // TAG TESTS
    // ============================================================

    #[test]
    fn test_tag_creation() {
        let tag = Tag {
            id: 1,
            name: "Enemy".to_string(),
            color: Color32::RED,
        };
        assert_eq!(tag.id, 1);
        assert_eq!(tag.name, "Enemy");
        assert_eq!(tag.color, Color32::RED);
    }

    #[test]
    fn test_tag_clone() {
        let tag = Tag {
            id: 3,
            name: "Collectible".to_string(),
            color: Color32::YELLOW,
        };
        let cloned = tag.clone();
        assert_eq!(cloned.id, 3);
        assert_eq!(cloned.name, "Collectible");
    }

    // ============================================================
    // COMPRESSION MODE TESTS
    // ============================================================

    #[test]
    fn test_compression_mode_default() {
        let mode = CompressionMode::default();
        assert_eq!(mode, CompressionMode::Fast);
    }

    #[test]
    fn test_compression_mode_all_variants() {
        let variants = [
            CompressionMode::None,
            CompressionMode::Fast,
            CompressionMode::Best,
        ];
        assert_eq!(variants.len(), 3);
    }

    // ============================================================
    // BUILD CONFIG TESTS
    // ============================================================

    #[test]
    fn test_build_config_default() {
        let config = BuildConfig::default();
        assert_eq!(config.platform, TargetPlatform::Windows);
        assert!(config.enabled);
        assert!(config.development_build);
        assert_eq!(config.compression, CompressionMode::Fast);
        assert!(config.icon_path.is_empty());
        assert_eq!(config.app_name, "My Game");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.company, "My Company");
    }

    #[test]
    fn test_build_config_custom() {
        let config = BuildConfig {
            platform: TargetPlatform::Linux,
            enabled: false,
            development_build: false,
            compression: CompressionMode::Best,
            icon_path: "icons/game.png".to_string(),
            app_name: "My Awesome Game".to_string(),
            version: "2.0.0".to_string(),
            company: "Awesome Studios".to_string(),
        };
        assert_eq!(config.platform, TargetPlatform::Linux);
        assert!(!config.enabled);
        assert_eq!(config.compression, CompressionMode::Best);
    }

    #[test]
    fn test_build_config_clone() {
        let config = BuildConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.platform, TargetPlatform::Windows);
        assert_eq!(cloned.app_name, "My Game");
    }

    // ============================================================
    // SETTINGS TAB TESTS
    // ============================================================

    #[test]
    fn test_settings_tab_default() {
        let tab = SettingsTab::default();
        assert_eq!(tab, SettingsTab::Project);
    }

    #[test]
    fn test_settings_tab_all_variants() {
        let variants = [
            SettingsTab::Project,
            SettingsTab::Rendering,
            SettingsTab::Physics,
            SettingsTab::Audio,
            SettingsTab::Input,
            SettingsTab::Quality,
            SettingsTab::TagsLayers,
            SettingsTab::Build,
        ];
        assert_eq!(variants.len(), 8);
    }

    #[test]
    fn test_settings_tab_clone() {
        let tab = SettingsTab::Physics;
        let cloned = tab;
        assert_eq!(cloned, SettingsTab::Physics);
    }

    // ============================================================
    // PROJECT SETTINGS PANEL TESTS
    // ============================================================

    #[test]
    fn test_project_settings_creation() {
        let mut panel = ProjectSettingsPanel::new();
        panel.create_sample_data();
        assert!(panel.quality_level_count() >= 4);
    }

    #[test]
    fn test_project_settings_default() {
        let mut panel = ProjectSettingsPanel::default();
        panel.create_sample_data();
        assert!(panel.quality_level_count() >= 4);
        assert!(panel.input_action_count() >= 7);
        assert!(panel.tag_count() >= 5);
        assert!(panel.layer_count() >= 5);
    }

    #[test]
    fn test_input_actions() {
        let mut panel = ProjectSettingsPanel::new();
        panel.create_sample_data();
        assert!(panel.input_action_count() >= 7);
    }

    #[test]
    fn test_tags() {
        let mut panel = ProjectSettingsPanel::new();
        panel.create_sample_data();
        assert!(panel.tag_count() >= 5);
    }

    #[test]
    fn test_layers() {
        let mut panel = ProjectSettingsPanel::new();
        panel.create_sample_data();
        assert!(panel.layer_count() >= 5);
    }

    #[test]
    fn test_set_project_name() {
        let mut panel = ProjectSettingsPanel::new();
        panel.set_project_name("Test Project");
        assert_eq!(panel.project_name, "Test Project");
    }

    #[test]
    fn test_panel_trait() {
        let panel = ProjectSettingsPanel::new();
        assert_eq!(panel.name(), "Project Settings");
    }

    #[test]
    fn test_set_empty_project_name() {
        let mut panel = ProjectSettingsPanel::new();
        panel.set_project_name("");
        assert_eq!(panel.project_name, "");
    }

    #[test]
    fn test_set_long_project_name() {
        let mut panel = ProjectSettingsPanel::new();
        let long_name = "A Very Long Project Name That Goes On And On For Testing Purposes";
        panel.set_project_name(long_name);
        assert_eq!(panel.project_name, long_name);
    }

    // ============================================================
    // SAMPLE DATA TESTS
    // ============================================================

    #[test]
    fn test_sample_quality_levels() {
        let mut panel = ProjectSettingsPanel::new();
        panel.create_sample_data();
        // Should have Low, Medium, High, Ultra
        assert_eq!(panel.quality_level_count(), 4);
    }

    #[test]
    fn test_sample_input_actions() {
        let mut panel = ProjectSettingsPanel::new();
        panel.create_sample_data();
        // Should have movement, jump, attack, interact
        assert!(panel.input_action_count() >= 7);
    }

    #[test]
    fn test_sample_tags() {
        let mut panel = ProjectSettingsPanel::new();
        panel.create_sample_data();
        // Should have Player, Enemy, NPC, Collectible, Interactable
        assert_eq!(panel.tag_count(), 5);
    }

    #[test]
    fn test_sample_layers() {
        let mut panel = ProjectSettingsPanel::new();
        panel.create_sample_data();
        // Should have Default, Player, Enemy, Trigger, Projectile
        assert_eq!(panel.layer_count(), 5);
    }

    // ============================================================
    // DISPLAY TRAIT TESTS
    // ============================================================

    #[test]
    fn test_target_platform_display() {
        for platform in TargetPlatform::all() {
            let display = format!("{}", platform);
            assert!(
                display.contains(platform.name()),
                "Display should contain name"
            );
        }
    }

    #[test]
    fn test_target_platform_name() {
        assert_eq!(TargetPlatform::Windows.name(), "Windows");
        assert_eq!(TargetPlatform::MacOS.name(), "macOS");
        assert_eq!(TargetPlatform::NintendoSwitch.name(), "Nintendo Switch");
    }

    #[test]
    fn test_target_platform_all() {
        let all = TargetPlatform::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&TargetPlatform::Windows));
        assert!(all.contains(&TargetPlatform::WebAssembly));
    }

    #[test]
    fn test_target_platform_is_desktop() {
        assert!(TargetPlatform::Windows.is_desktop());
        assert!(TargetPlatform::Linux.is_desktop());
        assert!(TargetPlatform::MacOS.is_desktop());
        assert!(!TargetPlatform::Android.is_desktop());
        assert!(!TargetPlatform::PlayStation.is_desktop());
    }

    #[test]
    fn test_target_platform_is_mobile() {
        assert!(TargetPlatform::Android.is_mobile());
        assert!(TargetPlatform::Ios.is_mobile());
        assert!(!TargetPlatform::Windows.is_mobile());
    }

    #[test]
    fn test_target_platform_is_console() {
        assert!(TargetPlatform::PlayStation.is_console());
        assert!(TargetPlatform::Xbox.is_console());
        assert!(TargetPlatform::NintendoSwitch.is_console());
        assert!(!TargetPlatform::Windows.is_console());
    }

    #[test]
    fn test_target_platform_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for platform in TargetPlatform::all() {
            set.insert(*platform);
        }
        assert_eq!(set.len(), TargetPlatform::all().len());
    }

    #[test]
    fn test_texture_quality_display() {
        for quality in TextureQuality::all() {
            let display = format!("{}", quality);
            assert!(display.contains(quality.name()));
        }
    }

    #[test]
    fn test_texture_quality_all() {
        let all = TextureQuality::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_texture_quality_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for quality in TextureQuality::all() {
            set.insert(*quality);
        }
        assert_eq!(set.len(), TextureQuality::all().len());
    }

    #[test]
    fn test_antialiasing_mode_display() {
        for mode in AntialiasingMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_antialiasing_mode_all() {
        let all = AntialiasingMode::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_antialiasing_mode_is_msaa() {
        assert!(AntialiasingMode::Msaa2x.is_msaa());
        assert!(AntialiasingMode::Msaa4x.is_msaa());
        assert!(AntialiasingMode::Msaa8x.is_msaa());
        assert!(!AntialiasingMode::Fxaa.is_msaa());
        assert!(!AntialiasingMode::Taa.is_msaa());
    }

    #[test]
    fn test_antialiasing_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in AntialiasingMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), AntialiasingMode::all().len());
    }

    #[test]
    fn test_broadphase_type_display() {
        for bp in BroadphaseType::all() {
            let display = format!("{}", bp);
            assert!(display.contains(bp.name()));
        }
    }

    #[test]
    fn test_broadphase_type_all() {
        let all = BroadphaseType::all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_broadphase_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for bp in BroadphaseType::all() {
            set.insert(*bp);
        }
        assert_eq!(set.len(), BroadphaseType::all().len());
    }

    #[test]
    fn test_audio_backend_display() {
        for backend in AudioBackend::all() {
            let display = format!("{}", backend);
            assert!(display.contains(backend.name()));
        }
    }

    #[test]
    fn test_audio_backend_all() {
        let all = AudioBackend::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_audio_backend_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for backend in AudioBackend::all() {
            set.insert(*backend);
        }
        assert_eq!(set.len(), AudioBackend::all().len());
    }

    #[test]
    fn test_renderer_backend_display() {
        for backend in RendererBackend::all() {
            let display = format!("{}", backend);
            assert!(display.contains(backend.name()));
        }
    }

    #[test]
    fn test_renderer_backend_all() {
        let all = RendererBackend::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_renderer_backend_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for backend in RendererBackend::all() {
            set.insert(*backend);
        }
        assert_eq!(set.len(), RendererBackend::all().len());
    }

    #[test]
    fn test_tonemapping_mode_display() {
        for mode in TonemappingMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_tonemapping_mode_all() {
        let all = TonemappingMode::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_tonemapping_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in TonemappingMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), TonemappingMode::all().len());
    }

    #[test]
    fn test_ao_mode_display() {
        for mode in AoMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_ao_mode_all() {
        let all = AoMode::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_ao_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in AoMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), AoMode::all().len());
    }

    #[test]
    fn test_gi_mode_display() {
        for mode in GiMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_gi_mode_all() {
        let all = GiMode::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_gi_mode_is_raytraced() {
        assert!(GiMode::Lumen.is_raytraced());
        assert!(GiMode::PathTraced.is_raytraced());
        assert!(!GiMode::None.is_raytraced());
        assert!(!GiMode::ScreenSpace.is_raytraced());
    }

    #[test]
    fn test_gi_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in GiMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), GiMode::all().len());
    }

    #[test]
    fn test_reflection_mode_display() {
        for mode in ReflectionMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_reflection_mode_all() {
        let all = ReflectionMode::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_reflection_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in ReflectionMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), ReflectionMode::all().len());
    }

    #[test]
    fn test_shadow_mode_display() {
        for mode in ShadowMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_shadow_mode_all() {
        let all = ShadowMode::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_shadow_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in ShadowMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), ShadowMode::all().len());
    }

    #[test]
    fn test_compression_mode_display() {
        for mode in CompressionMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_compression_mode_all() {
        let all = CompressionMode::all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_compression_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in CompressionMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), CompressionMode::all().len());
    }

    #[test]
    fn test_settings_tab_display() {
        for tab in SettingsTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()), "Display should contain name");
        }
    }

    #[test]
    fn test_settings_tab_name() {
        assert_eq!(SettingsTab::Project.name(), "Project");
        assert_eq!(SettingsTab::TagsLayers.name(), "Tags & Layers");
    }

    #[test]
    fn test_settings_tab_icon() {
        assert_eq!(SettingsTab::Project.icon(), "[Dir]");
        assert_eq!(SettingsTab::Rendering.icon(), "[Art]");
        assert_eq!(SettingsTab::Build.icon(), "[Pkg]");
    }

    #[test]
    fn test_settings_tab_all() {
        let all = SettingsTab::all();
        assert_eq!(all.len(), 9);
    }

    #[test]
    fn test_settings_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in SettingsTab::all() {
            set.insert(*tab);
        }
        assert_eq!(set.len(), SettingsTab::all().len());
    }

    // =====================================================================
    // ProjectSettingsAction Tests
    // =====================================================================

    #[test]
    fn test_project_settings_action_display_project() {
        let action = ProjectSettingsAction::SetProjectName("MyGame".to_string());
        assert!(format!("{}", action).contains("MyGame"));

        let action = ProjectSettingsAction::SetProjectVersion("1.0.0".to_string());
        assert!(format!("{}", action).contains("1.0.0"));
    }

    #[test]
    fn test_project_settings_action_display_rendering() {
        let action = ProjectSettingsAction::ToggleHDR(true);
        assert!(format!("{}", action).contains("on"));

        let action = ProjectSettingsAction::ToggleBloom(false);
        assert!(format!("{}", action).contains("off"));

        let action = ProjectSettingsAction::SetBloomIntensity(0.75);
        assert!(format!("{}", action).contains("0.75"));
    }

    #[test]
    fn test_project_settings_action_display_audio() {
        let action = ProjectSettingsAction::SetMasterVolume(0.8);
        assert!(format!("{}", action).contains("80"));

        let action = ProjectSettingsAction::SetMaxSimultaneousSounds(64);
        assert!(format!("{}", action).contains("64"));
    }

    #[test]
    fn test_project_settings_action_display_quality() {
        let action = ProjectSettingsAction::AddQualityLevel;
        assert!(format!("{}", action).contains("quality"));

        let action = ProjectSettingsAction::SelectQualityLevel(2);
        assert!(format!("{}", action).contains("2"));
    }

    #[test]
    fn test_project_settings_action_display_input() {
        let action = ProjectSettingsAction::AddInputAction;
        assert!(format!("{}", action).contains("input"));

        let action = ProjectSettingsAction::SetInputActionKey {
            index: 0,
            key: "Space".to_string(),
        };
        assert!(format!("{}", action).contains("Space"));
    }

    #[test]
    fn test_project_settings_action_display_tags_layers() {
        let action = ProjectSettingsAction::AddTag("Player".to_string());
        assert!(format!("{}", action).contains("Player"));

        let action = ProjectSettingsAction::AddLayer("Ground".to_string());
        assert!(format!("{}", action).contains("Ground"));
    }

    #[test]
    fn test_project_settings_action_display_build() {
        let action = ProjectSettingsAction::StartBuild(0);
        assert!(format!("{}", action).contains("build"));

        let action = ProjectSettingsAction::SetBuildPlatform {
            index: 0,
            platform: TargetPlatform::Windows,
        };
        assert!(format!("{}", action).contains("Windows"));
    }

    #[test]
    fn test_project_settings_action_display_general() {
        let action = ProjectSettingsAction::SaveSettings;
        assert!(format!("{}", action).contains("Save"));

        let action = ProjectSettingsAction::LoadSettings;
        assert!(format!("{}", action).contains("Load"));

        let action = ProjectSettingsAction::ResetToDefaults;
        assert!(format!("{}", action).contains("Reset"));
    }

    #[test]
    fn test_project_settings_panel_action_queue() {
        let mut panel = ProjectSettingsPanel::new();
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());

        panel.queue_action(ProjectSettingsAction::SaveSettings);
        assert!(panel.has_pending_actions());
        assert_eq!(panel.pending_actions().len(), 1);
    }

    #[test]
    fn test_project_settings_panel_take_actions() {
        let mut panel = ProjectSettingsPanel::new();
        panel.queue_action(ProjectSettingsAction::SetProjectName("Test".to_string()));
        panel.queue_action(ProjectSettingsAction::ToggleHDR(true));

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_project_settings_panel_action_order() {
        let mut panel = ProjectSettingsPanel::new();
        panel.queue_action(ProjectSettingsAction::SetMasterVolume(0.5));
        panel.queue_action(ProjectSettingsAction::SetMusicVolume(0.7));
        panel.queue_action(ProjectSettingsAction::SetSfxVolume(1.0));

        let actions = panel.take_actions();
        assert!(matches!(
            actions[0],
            ProjectSettingsAction::SetMasterVolume(_)
        ));
        assert!(matches!(
            actions[1],
            ProjectSettingsAction::SetMusicVolume(_)
        ));
        assert!(matches!(actions[2], ProjectSettingsAction::SetSfxVolume(_)));
    }

    #[test]
    fn test_project_settings_action_physics() {
        let action = ProjectSettingsAction::SetGravity([0.0, -9.81, 0.0]);
        let display = format!("{}", action);
        assert!(display.contains("-9.8"));

        let action = ProjectSettingsAction::SetFixedTimestep(0.0166);
        assert!(format!("{}", action).contains("0.016"));
    }

    #[test]
    fn test_project_settings_action_equality() {
        let a1 = ProjectSettingsAction::SetProjectName("A".to_string());
        let a2 = ProjectSettingsAction::SetProjectName("A".to_string());
        let a3 = ProjectSettingsAction::SetProjectName("B".to_string());

        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn test_project_settings_action_clone() {
        let action = ProjectSettingsAction::ExportSettings("/path/to/settings".to_string());
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }
}
