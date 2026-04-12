//! Post-Processing Panel for the editor UI
//!
//! Provides comprehensive post-processing configuration:
//! - Bloom, depth of field, motion blur
//! - Color grading and tonemapping
//! - Anti-aliasing settings
//! - Ambient occlusion
//! - Screen-space effects

#![allow(clippy::upper_case_acronyms)] // ACES, FXAA, SMAA, TAA, SSAO, HBAO, GTAO are standard graphics acronyms

use egui::{Color32, RichText, Ui};
use tracing::info;

use crate::panels::Panel;

/// Snapshot of post-process settings that need to be forwarded to the renderer.
#[derive(Debug, Clone)]
pub struct PostProcessPendingSettings {
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub bloom_soft_knee: f32,
    pub tonemap: Tonemapper,
}

/// Tonemapping algorithm
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Tonemapper {
    None,
    Reinhard,
    #[default]
    ACES,
    Filmic,
    AgX,
    Neutral,
}

impl std::fmt::Display for Tonemapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl Tonemapper {
    pub fn all() -> &'static [Tonemapper] {
        &[
            Tonemapper::None,
            Tonemapper::Reinhard,
            Tonemapper::ACES,
            Tonemapper::Filmic,
            Tonemapper::AgX,
            Tonemapper::Neutral,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Tonemapper::None => "None",
            Tonemapper::Reinhard => "Reinhard",
            Tonemapper::ACES => "ACES",
            Tonemapper::Filmic => "Filmic",
            Tonemapper::AgX => "AgX",
            Tonemapper::Neutral => "Neutral",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Tonemapper::None => "⚫",
            Tonemapper::Reinhard => "🌈",
            Tonemapper::ACES => "🎞️",
            Tonemapper::Filmic => "[Anim]",
            Tonemapper::AgX => "[Img]",
            Tonemapper::Neutral => "⚖️",
        }
    }

    pub fn is_cinematic(&self) -> bool {
        matches!(
            self,
            Tonemapper::ACES | Tonemapper::Filmic | Tonemapper::AgX
        )
    }
}

/// Anti-aliasing method
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AntiAliasing {
    None,
    FXAA,
    #[default]
    SMAA,
    TAA,
    MSAA2x,
    MSAA4x,
    MSAA8x,
}

impl std::fmt::Display for AntiAliasing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AntiAliasing {
    pub fn all() -> &'static [AntiAliasing] {
        &[
            AntiAliasing::None,
            AntiAliasing::FXAA,
            AntiAliasing::SMAA,
            AntiAliasing::TAA,
            AntiAliasing::MSAA2x,
            AntiAliasing::MSAA4x,
            AntiAliasing::MSAA8x,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            AntiAliasing::None => "None",
            AntiAliasing::FXAA => "FXAA",
            AntiAliasing::SMAA => "SMAA",
            AntiAliasing::TAA => "TAA",
            AntiAliasing::MSAA2x => "MSAA 2x",
            AntiAliasing::MSAA4x => "MSAA 4x",
            AntiAliasing::MSAA8x => "MSAA 8x",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AntiAliasing::None => "⚫",
            AntiAliasing::FXAA => "[Img]",
            AntiAliasing::SMAA => "[Img]",
            AntiAliasing::TAA => "[Time]",
            AntiAliasing::MSAA2x => "□",
            AntiAliasing::MSAA4x => "□",
            AntiAliasing::MSAA8x => "□",
        }
    }

    pub fn is_msaa(&self) -> bool {
        matches!(
            self,
            AntiAliasing::MSAA2x | AntiAliasing::MSAA4x | AntiAliasing::MSAA8x
        )
    }

    pub fn is_post_process(&self) -> bool {
        matches!(
            self,
            AntiAliasing::FXAA | AntiAliasing::SMAA | AntiAliasing::TAA
        )
    }
}

/// Depth of field mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DofMode {
    #[default]
    Disabled,
    Gaussian,
    Bokeh,
    CircleOfConfusion,
}

impl std::fmt::Display for DofMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl DofMode {
    pub fn all() -> &'static [DofMode] {
        &[
            DofMode::Disabled,
            DofMode::Gaussian,
            DofMode::Bokeh,
            DofMode::CircleOfConfusion,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            DofMode::Disabled => "Disabled",
            DofMode::Gaussian => "Gaussian",
            DofMode::Bokeh => "Bokeh",
            DofMode::CircleOfConfusion => "Circle of Confusion",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DofMode::Disabled => "⚫",
            DofMode::Gaussian => "[Fog]",
            DofMode::Bokeh => "[Fx]",
            DofMode::CircleOfConfusion => "○",
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, DofMode::Disabled)
    }
}

/// Bloom settings
#[derive(Debug, Clone, PartialEq)]
pub struct BloomSettings {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
    pub soft_threshold: f32,
    pub radius: f32,
    pub dirt_mask_enabled: bool,
    pub dirt_mask_intensity: f32,
    pub dirt_mask_path: String,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 0.5,
            threshold: 1.0,
            soft_threshold: 0.5,
            radius: 5.0,
            dirt_mask_enabled: false,
            dirt_mask_intensity: 1.0,
            dirt_mask_path: String::new(),
        }
    }
}

/// Depth of field settings
#[derive(Debug, Clone)]
pub struct DepthOfFieldSettings {
    pub mode: DofMode,
    pub focus_distance: f32,
    pub aperture: f32,
    pub focal_length: f32,
    pub blade_count: u32,
    pub blade_curvature: f32,
    pub max_blur: f32,
}

impl Default for DepthOfFieldSettings {
    fn default() -> Self {
        Self {
            mode: DofMode::Disabled,
            focus_distance: 10.0,
            aperture: 5.6,
            focal_length: 50.0,
            blade_count: 6,
            blade_curvature: 0.5,
            max_blur: 1.0,
        }
    }
}

/// Motion blur settings
#[derive(Debug, Clone)]
pub struct MotionBlurSettings {
    pub enabled: bool,
    pub intensity: f32,
    pub sample_count: u32,
    pub max_velocity: f32,
    pub camera_motion_blur: bool,
    pub object_motion_blur: bool,
}

impl Default for MotionBlurSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.5,
            sample_count: 8,
            max_velocity: 1000.0,
            camera_motion_blur: true,
            object_motion_blur: true,
        }
    }
}

/// Color grading settings
#[derive(Debug, Clone)]
pub struct ColorGradingSettings {
    pub enabled: bool,

    // White balance
    pub temperature: f32,
    pub tint: f32,

    // Tone
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub vibrance: f32,

    // Color adjustments
    pub hue_shift: f32,
    pub shadows: [f32; 3],
    pub midtones: [f32; 3],
    pub highlights: [f32; 3],

    // Curves
    pub gamma: f32,
    pub gain: f32,
    pub lift: f32,

    // LUT
    pub lut_enabled: bool,
    pub lut_path: String,
    pub lut_contribution: f32,
}

impl Default for ColorGradingSettings {
    fn default() -> Self {
        Self {
            enabled: true,

            temperature: 0.0,
            tint: 0.0,

            exposure: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            vibrance: 0.0,

            hue_shift: 0.0,
            shadows: [0.0, 0.0, 0.0],
            midtones: [0.0, 0.0, 0.0],
            highlights: [0.0, 0.0, 0.0],

            gamma: 1.0,
            gain: 1.0,
            lift: 0.0,

            lut_enabled: false,
            lut_path: String::new(),
            lut_contribution: 1.0,
        }
    }
}

/// Ambient occlusion settings
#[derive(Debug, Clone)]
pub struct AmbientOcclusionSettings {
    pub enabled: bool,
    pub method: AoMethod,
    pub intensity: f32,
    pub radius: f32,
    pub bias: f32,
    pub samples: u32,
    pub direct_lighting_strength: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AoMethod {
    #[default]
    SSAO,
    HBAO,
    GTAO,
}

impl std::fmt::Display for AoMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AoMethod {
    pub fn all() -> &'static [AoMethod] {
        &[AoMethod::SSAO, AoMethod::HBAO, AoMethod::GTAO]
    }

    pub fn name(&self) -> &'static str {
        match self {
            AoMethod::SSAO => "SSAO",
            AoMethod::HBAO => "HBAO+",
            AoMethod::GTAO => "GTAO",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AoMethod::SSAO => "🖤",
            AoMethod::HBAO => "🌑",
            AoMethod::GTAO => "🌚",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AoMethod::SSAO => "Screen-space ambient occlusion",
            AoMethod::HBAO => "Horizon-based ambient occlusion",
            AoMethod::GTAO => "Ground-truth ambient occlusion",
        }
    }
}

impl Default for AmbientOcclusionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            method: AoMethod::SSAO,
            intensity: 0.5,
            radius: 0.5,
            bias: 0.025,
            samples: 16,
            direct_lighting_strength: 0.0,
        }
    }
}

/// Screen-space reflections settings
#[derive(Debug, Clone)]
pub struct SsrSettings {
    pub enabled: bool,
    pub max_distance: f32,
    pub resolution: f32,
    pub thickness: f32,
    pub max_roughness: f32,
}

impl Default for SsrSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            max_distance: 100.0,
            resolution: 0.5,
            thickness: 0.1,
            max_roughness: 0.5,
        }
    }
}

/// Vignette settings
#[derive(Debug, Clone)]
pub struct VignetteSettings {
    pub enabled: bool,
    pub intensity: f32,
    pub smoothness: f32,
    pub roundness: f32,
    pub color: [f32; 3],
}

impl Default for VignetteSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.3,
            smoothness: 0.5,
            roundness: 1.0,
            color: [0.0, 0.0, 0.0],
        }
    }
}

/// Chromatic aberration settings
#[derive(Debug, Clone)]
pub struct ChromaticAberrationSettings {
    pub enabled: bool,
    pub intensity: f32,
}

impl Default for ChromaticAberrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.1,
        }
    }
}

/// Film grain settings
#[derive(Debug, Clone)]
pub struct FilmGrainSettings {
    pub enabled: bool,
    pub intensity: f32,
    pub response: f32,
}

impl Default for FilmGrainSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.3,
            response: 0.8,
        }
    }
}

/// Post-process profile
#[derive(Debug, Clone)]
pub struct PostProcessProfile {
    pub id: u32,
    pub name: String,

    pub tonemapper: Tonemapper,
    pub anti_aliasing: AntiAliasing,

    pub bloom: BloomSettings,
    pub dof: DepthOfFieldSettings,
    pub motion_blur: MotionBlurSettings,
    pub color_grading: ColorGradingSettings,
    pub ao: AmbientOcclusionSettings,
    pub ssr: SsrSettings,
    pub vignette: VignetteSettings,
    pub chromatic_aberration: ChromaticAberrationSettings,
    pub film_grain: FilmGrainSettings,
}

impl Default for PostProcessProfile {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Profile".to_string(),

            tonemapper: Tonemapper::ACES,
            anti_aliasing: AntiAliasing::SMAA,

            bloom: BloomSettings::default(),
            dof: DepthOfFieldSettings::default(),
            motion_blur: MotionBlurSettings::default(),
            color_grading: ColorGradingSettings::default(),
            ao: AmbientOcclusionSettings::default(),
            ssr: SsrSettings::default(),
            vignette: VignetteSettings::default(),
            chromatic_aberration: ChromaticAberrationSettings::default(),
            film_grain: FilmGrainSettings::default(),
        }
    }
}

/// Built-in presets
#[derive(Debug, Clone)]
pub struct PostProcessPreset {
    pub name: String,
    pub category: String,
    pub description: String,
}

impl PostProcessPreset {
    fn presets() -> Vec<PostProcessPreset> {
        vec![
            PostProcessPreset {
                name: "Cinematic".to_string(),
                category: "Film".to_string(),
                description: "Film-like color grading".to_string(),
            },
            PostProcessPreset {
                name: "Noir".to_string(),
                category: "Film".to_string(),
                description: "Black and white with high contrast".to_string(),
            },
            PostProcessPreset {
                name: "Vintage".to_string(),
                category: "Film".to_string(),
                description: "Warm, faded look".to_string(),
            },
            PostProcessPreset {
                name: "Horror".to_string(),
                category: "Game".to_string(),
                description: "Dark, desaturated, vignette".to_string(),
            },
            PostProcessPreset {
                name: "Sci-Fi".to_string(),
                category: "Game".to_string(),
                description: "Cool tones, bloom, chromatic".to_string(),
            },
            PostProcessPreset {
                name: "Fantasy".to_string(),
                category: "Game".to_string(),
                description: "Warm, magical glow".to_string(),
            },
            PostProcessPreset {
                name: "Realistic".to_string(),
                category: "Simulation".to_string(),
                description: "Natural colors, subtle effects".to_string(),
            },
            PostProcessPreset {
                name: "HDR Vivid".to_string(),
                category: "Simulation".to_string(),
                description: "Enhanced colors and contrast".to_string(),
            },
            PostProcessPreset {
                name: "Performance".to_string(),
                category: "Technical".to_string(),
                description: "Minimal effects for speed".to_string(),
            },
            PostProcessPreset {
                name: "Quality".to_string(),
                category: "Technical".to_string(),
                description: "Maximum visual fidelity".to_string(),
            },
        ]
    }
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PostProcessTab {
    #[default]
    Overview,
    Bloom,
    DepthOfField,
    MotionBlur,
    ColorGrading,
    Effects,
    Presets,
}

impl std::fmt::Display for PostProcessTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl PostProcessTab {
    pub fn all() -> &'static [PostProcessTab] {
        &[
            PostProcessTab::Overview,
            PostProcessTab::Bloom,
            PostProcessTab::DepthOfField,
            PostProcessTab::MotionBlur,
            PostProcessTab::ColorGrading,
            PostProcessTab::Effects,
            PostProcessTab::Presets,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PostProcessTab::Overview => "Overview",
            PostProcessTab::Bloom => "Bloom",
            PostProcessTab::DepthOfField => "Depth of Field",
            PostProcessTab::MotionBlur => "Motion Blur",
            PostProcessTab::ColorGrading => "Color Grading",
            PostProcessTab::Effects => "Effects",
            PostProcessTab::Presets => "Presets",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PostProcessTab::Overview => "[List]",
            PostProcessTab::Bloom => "[Fx]",
            PostProcessTab::DepthOfField => "📷",
            PostProcessTab::MotionBlur => "🎿",
            PostProcessTab::ColorGrading => "[Art]",
            PostProcessTab::Effects => "🌟",
            PostProcessTab::Presets => "[Save]",
        }
    }
}

/// Main Post-Processing Panel
pub struct PostProcessPanel {
    // Tab state
    active_tab: PostProcessTab,

    // Profiles
    profiles: Vec<PostProcessProfile>,
    selected_profile: Option<u32>,
    current_profile: PostProcessProfile,

    // Presets
    presets: Vec<PostProcessPreset>,
    preset_filter: String,

    // Preview
    preview_enabled: bool,
    split_view: bool,
    split_position: f32,

    // ID counter
    next_id: u32,

    // Dirty flag for renderer sync — set when any UI setting changes.
    settings_dirty: bool,
}

impl Default for PostProcessPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: PostProcessTab::Overview,

            profiles: Vec::new(),
            selected_profile: None,
            current_profile: PostProcessProfile::default(),

            presets: PostProcessPreset::presets(),
            preset_filter: String::new(),

            preview_enabled: true,
            split_view: false,
            split_position: 0.5,

            next_id: 1,

            settings_dirty: false,
        };

        panel
    }
}

impl PostProcessPanel {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_sample_data(&mut self) {
        // Default profile
        let id = self.next_id();
        self.profiles.push(PostProcessProfile {
            id,
            name: "Default".to_string(),
            ..Default::default()
        });
        self.next_id += 1;

        // Cinematic profile
        let id = self.next_id();
        self.profiles.push(PostProcessProfile {
            id,
            name: "Cinematic".to_string(),
            bloom: BloomSettings {
                enabled: true,
                intensity: 0.8,
                threshold: 0.8,
                ..Default::default()
            },
            color_grading: ColorGradingSettings {
                enabled: true,
                contrast: 0.1,
                saturation: -0.1,
                temperature: -5.0,
                ..Default::default()
            },
            vignette: VignetteSettings {
                enabled: true,
                intensity: 0.4,
                ..Default::default()
            },
            film_grain: FilmGrainSettings {
                enabled: true,
                intensity: 0.15,
                ..Default::default()
            },
            ..Default::default()
        });
        self.next_id += 1;

        // Performance profile
        let id = self.next_id();
        self.profiles.push(PostProcessProfile {
            id,
            name: "Performance".to_string(),
            anti_aliasing: AntiAliasing::FXAA,
            bloom: BloomSettings {
                enabled: true,
                intensity: 0.3,
                ..Default::default()
            },
            ao: AmbientOcclusionSettings {
                enabled: false,
                ..Default::default()
            },
            ssr: SsrSettings {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        });
        self.next_id += 1;

        self.current_profile = self.profiles[0].clone();
        self.selected_profile = Some(self.profiles[0].id);
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (PostProcessTab::Overview, "Overview"),
                (PostProcessTab::Bloom, "Bloom"),
                (PostProcessTab::DepthOfField, "📷 DoF"),
                (PostProcessTab::MotionBlur, "[Dash] Motion"),
                (PostProcessTab::ColorGrading, "Color"),
                (PostProcessTab::Effects, "Effects"),
                (PostProcessTab::Presets, "Presets"),
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

        // Profile info
        ui.horizontal(|ui| {
            ui.label(format!("{}", self.current_profile.name));

            ui.separator();

            ui.checkbox(&mut self.preview_enabled, "Preview");
            ui.checkbox(&mut self.split_view, "Split View");

            if self.split_view {
                ui.add(egui::Slider::new(&mut self.split_position, 0.1..=0.9).show_value(false));
            }
        });

        ui.separator();
    }

    fn show_overview_tab(&mut self, ui: &mut Ui) {
        ui.heading("Post-Processing Overview");
        ui.add_space(10.0);

        let prev_tonemapper = self.current_profile.tonemapper;

        // Profile selector
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("profile_select")
                .selected_text(&self.current_profile.name)
                .show_ui(ui, |ui| {
                    for profile in &self.profiles.clone() {
                        if ui
                            .selectable_value(
                                &mut self.selected_profile,
                                Some(profile.id),
                                &profile.name,
                            )
                            .clicked()
                        {
                            self.current_profile = profile.clone();
                        }
                    }
                });

            if ui.button("+ New").clicked() {
                let id = self.next_id();
                info!(profile_id = id, "PostProcess: created profile");
                let new_profile = PostProcessProfile {
                    id,
                    name: format!("Profile {}", id),
                    ..Default::default()
                };
                self.profiles.push(new_profile.clone());
                self.current_profile = new_profile;
                self.selected_profile = Some(id);
            }

            if ui.button("Duplicate").clicked() {
                let id = self.next_id();
                let mut dup = self.current_profile.clone();
                dup.id = id;
                dup.name = format!("{} (Copy)", dup.name);
                self.profiles.push(dup);
            }
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                // Global settings
                ui.group(|ui| {
                    ui.label(RichText::new("Global Settings").strong());

                    egui::Grid::new("global_settings")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.current_profile.name);
                            ui.end_row();

                            ui.label("Tonemapper:");
                            egui::ComboBox::from_id_salt("tonemapper")
                                .selected_text(format!("{:?}", self.current_profile.tonemapper))
                                .show_ui(ui, |ui| {
                                    for tm in Tonemapper::all() {
                                        ui.selectable_value(
                                            &mut self.current_profile.tonemapper,
                                            *tm,
                                            format!("{:?}", tm),
                                        );
                                    }
                                });
                            ui.end_row();

                            ui.label("Anti-Aliasing:");
                            egui::ComboBox::from_id_salt("aa")
                                .selected_text(format!("{:?}", self.current_profile.anti_aliasing))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.current_profile.anti_aliasing,
                                        AntiAliasing::None,
                                        "None",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_profile.anti_aliasing,
                                        AntiAliasing::FXAA,
                                        "FXAA",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_profile.anti_aliasing,
                                        AntiAliasing::SMAA,
                                        "SMAA",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_profile.anti_aliasing,
                                        AntiAliasing::TAA,
                                        "TAA",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_profile.anti_aliasing,
                                        AntiAliasing::MSAA2x,
                                        "MSAA 2x",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_profile.anti_aliasing,
                                        AntiAliasing::MSAA4x,
                                        "MSAA 4x",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_profile.anti_aliasing,
                                        AntiAliasing::MSAA8x,
                                        "MSAA 8x",
                                    );
                                });
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Effect status summary
                ui.group(|ui| {
                    ui.label(RichText::new("Effect Status").strong());

                    let effects = [
                        ("Bloom", self.current_profile.bloom.enabled),
                        (
                            "📷 Depth of Field",
                            self.current_profile.dof.mode != DofMode::Disabled,
                        ),
                        (
                            "[Dash] Motion Blur",
                            self.current_profile.motion_blur.enabled,
                        ),
                        ("Color Grading", self.current_profile.color_grading.enabled),
                        ("🌑 Ambient Occlusion", self.current_profile.ao.enabled),
                        ("🪞 SSR", self.current_profile.ssr.enabled),
                        ("⭕ Vignette", self.current_profile.vignette.enabled),
                        (
                            "🌈 Chromatic Aberration",
                            self.current_profile.chromatic_aberration.enabled,
                        ),
                        ("Film Grain", self.current_profile.film_grain.enabled),
                    ];

                    for (name, enabled) in effects {
                        ui.horizontal(|ui| {
                            let color = if enabled {
                                Color32::GREEN
                            } else {
                                Color32::GRAY
                            };
                            ui.label(RichText::new(if enabled { "●" } else { "○" }).color(color));
                            ui.label(name);
                        });
                    }
                });
            });
        // Mark dirty if tonemapper changed.
        if self.current_profile.tonemapper != prev_tonemapper {
            self.settings_dirty = true;
        }
    }

    fn show_bloom_tab(&mut self, ui: &mut Ui) {
        ui.heading("Bloom");
        ui.add_space(10.0);

        let before = self.current_profile.bloom.clone();
        let bloom_was_enabled = self.current_profile.bloom.enabled;
        ui.checkbox(&mut self.current_profile.bloom.enabled, "Enabled");
        if self.current_profile.bloom.enabled != bloom_was_enabled {
            info!(
                enabled = self.current_profile.bloom.enabled,
                "PostProcess: bloom toggled"
            );
        }

        if self.current_profile.bloom.enabled {
            ui.add_space(10.0);

            egui::Grid::new("bloom_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Intensity:");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.bloom.intensity,
                        0.0..=2.0,
                    ));
                    ui.end_row();

                    ui.label("Threshold:");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.bloom.threshold,
                        0.0..=5.0,
                    ));
                    ui.end_row();

                    ui.label("Soft Threshold:");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.bloom.soft_threshold,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    ui.label("Radius:");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.bloom.radius,
                        1.0..=10.0,
                    ));
                    ui.end_row();
                });

            ui.add_space(10.0);

            // Dirt mask
            ui.group(|ui| {
                ui.checkbox(
                    &mut self.current_profile.bloom.dirt_mask_enabled,
                    "Dirt Mask",
                );

                if self.current_profile.bloom.dirt_mask_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Intensity:");
                        ui.add(egui::Slider::new(
                            &mut self.current_profile.bloom.dirt_mask_intensity,
                            0.0..=5.0,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Texture:");
                        ui.text_edit_singleline(&mut self.current_profile.bloom.dirt_mask_path);
                        if ui.button("[Open]").clicked() {
                            // Open file dialog
                        }
                    });
                }
            });
        }
        // Mark dirty if any bloom setting changed.
        if self.current_profile.bloom != before {
            self.settings_dirty = true;
        }
    }

    fn show_dof_tab(&mut self, ui: &mut Ui) {
        ui.heading("📷 Depth of Field");
        ui.add_space(10.0);

        egui::ComboBox::from_id_salt("dof_mode")
            .selected_text(format!("{:?}", self.current_profile.dof.mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.current_profile.dof.mode,
                    DofMode::Disabled,
                    "Disabled",
                );
                ui.selectable_value(
                    &mut self.current_profile.dof.mode,
                    DofMode::Gaussian,
                    "Gaussian",
                );
                ui.selectable_value(&mut self.current_profile.dof.mode, DofMode::Bokeh, "Bokeh");
                ui.selectable_value(
                    &mut self.current_profile.dof.mode,
                    DofMode::CircleOfConfusion,
                    "Circle of Confusion",
                );
            });

        if self.current_profile.dof.mode != DofMode::Disabled {
            ui.add_space(10.0);

            egui::Grid::new("dof_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Focus Distance:");
                    ui.add(
                        egui::Slider::new(
                            &mut self.current_profile.dof.focus_distance,
                            0.1..=100.0,
                        )
                        .logarithmic(true),
                    );
                    ui.end_row();

                    ui.label("Aperture (f-stop):");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.dof.aperture,
                        1.0..=22.0,
                    ));
                    ui.end_row();

                    ui.label("Focal Length (mm):");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.dof.focal_length,
                        10.0..=200.0,
                    ));
                    ui.end_row();

                    ui.label("Max Blur:");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.dof.max_blur,
                        0.0..=3.0,
                    ));
                    ui.end_row();
                });

            if self.current_profile.dof.mode == DofMode::Bokeh {
                ui.add_space(10.0);
                ui.group(|ui| {
                    ui.label(RichText::new("Bokeh Shape").strong());

                    ui.horizontal(|ui| {
                        ui.label("Blade Count:");
                        ui.add(egui::Slider::new(
                            &mut self.current_profile.dof.blade_count,
                            3..=12,
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Blade Curvature:");
                        ui.add(egui::Slider::new(
                            &mut self.current_profile.dof.blade_curvature,
                            0.0..=1.0,
                        ));
                    });
                });
            }
        }
    }

    fn show_motion_blur_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Dash] Motion Blur");
        ui.add_space(10.0);

        let mb_was = self.current_profile.motion_blur.enabled;
        ui.checkbox(&mut self.current_profile.motion_blur.enabled, "Enabled");
        if self.current_profile.motion_blur.enabled != mb_was {
            info!(
                enabled = self.current_profile.motion_blur.enabled,
                "PostProcess: motion blur toggled"
            );
        }

        if self.current_profile.motion_blur.enabled {
            ui.add_space(10.0);

            egui::Grid::new("motion_blur_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Intensity:");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.motion_blur.intensity,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    ui.label("Sample Count:");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.motion_blur.sample_count,
                        4..=32,
                    ));
                    ui.end_row();

                    ui.label("Max Velocity:");
                    ui.add(egui::Slider::new(
                        &mut self.current_profile.motion_blur.max_velocity,
                        100.0..=2000.0,
                    ));
                    ui.end_row();
                });

            ui.add_space(10.0);

            ui.checkbox(
                &mut self.current_profile.motion_blur.camera_motion_blur,
                "Camera Motion Blur",
            );
            ui.checkbox(
                &mut self.current_profile.motion_blur.object_motion_blur,
                "Object Motion Blur",
            );
        }
    }

    fn show_color_grading_tab(&mut self, ui: &mut Ui) {
        ui.heading("Color Grading");
        ui.add_space(10.0);

        ui.checkbox(&mut self.current_profile.color_grading.enabled, "Enabled");

        if self.current_profile.color_grading.enabled {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    // White balance
                    ui.group(|ui| {
                        ui.label(RichText::new("[Sun] White Balance").strong());

                        egui::Grid::new("white_balance")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Temperature:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.temperature,
                                    -100.0..=100.0,
                                ));
                                ui.end_row();

                                ui.label("Tint:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.tint,
                                    -100.0..=100.0,
                                ));
                                ui.end_row();
                            });
                    });

                    ui.add_space(10.0);

                    // Tone
                    ui.group(|ui| {
                        ui.label(RichText::new("🎚️ Tone").strong());

                        egui::Grid::new("tone")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Exposure:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.exposure,
                                    -5.0..=5.0,
                                ));
                                ui.end_row();

                                ui.label("Contrast:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.contrast,
                                    -1.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Saturation:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.saturation,
                                    -1.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Vibrance:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.vibrance,
                                    -1.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Hue Shift:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.current_profile.color_grading.hue_shift,
                                        -180.0..=180.0,
                                    )
                                    .suffix("°"),
                                );
                                ui.end_row();
                            });
                    });

                    ui.add_space(10.0);

                    // Curves
                    ui.group(|ui| {
                        ui.label(RichText::new("📈 Curves").strong());

                        egui::Grid::new("curves")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Lift:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.lift,
                                    -1.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Gamma:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.gamma,
                                    0.1..=3.0,
                                ));
                                ui.end_row();

                                ui.label("Gain:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.gain,
                                    0.0..=3.0,
                                ));
                                ui.end_row();
                            });
                    });

                    ui.add_space(10.0);

                    // LUT
                    ui.group(|ui| {
                        ui.checkbox(&mut self.current_profile.color_grading.lut_enabled, "LUT");

                        if self.current_profile.color_grading.lut_enabled {
                            ui.horizontal(|ui| {
                                ui.label("Path:");
                                ui.text_edit_singleline(
                                    &mut self.current_profile.color_grading.lut_path,
                                );
                                if ui.button("[Open]").clicked() {
                                    // Open file dialog
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Contribution:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.color_grading.lut_contribution,
                                    0.0..=1.0,
                                ));
                            });
                        }
                    });
                });
        }
    }

    fn show_effects_tab(&mut self, ui: &mut Ui) {
        ui.heading("Screen Effects");
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Ambient Occlusion
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.current_profile.ao.enabled,
                        RichText::new("🌑 Ambient Occlusion").strong(),
                    );

                    if self.current_profile.ao.enabled {
                        egui::Grid::new("ao_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Method:");
                                egui::ComboBox::from_id_salt("ao_method")
                                    .selected_text(format!("{:?}", self.current_profile.ao.method))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.current_profile.ao.method,
                                            AoMethod::SSAO,
                                            "SSAO",
                                        );
                                        ui.selectable_value(
                                            &mut self.current_profile.ao.method,
                                            AoMethod::HBAO,
                                            "HBAO",
                                        );
                                        ui.selectable_value(
                                            &mut self.current_profile.ao.method,
                                            AoMethod::GTAO,
                                            "GTAO",
                                        );
                                    });
                                ui.end_row();

                                ui.label("Intensity:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.ao.intensity,
                                    0.0..=2.0,
                                ));
                                ui.end_row();

                                ui.label("Radius:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.ao.radius,
                                    0.1..=2.0,
                                ));
                                ui.end_row();

                                ui.label("Samples:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.ao.samples,
                                    4..=64,
                                ));
                                ui.end_row();
                            });
                    }
                });

                ui.add_space(10.0);

                // SSR
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.current_profile.ssr.enabled,
                        RichText::new("🪞 Screen-Space Reflections").strong(),
                    );

                    if self.current_profile.ssr.enabled {
                        egui::Grid::new("ssr_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Max Distance:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.ssr.max_distance,
                                    10.0..=500.0,
                                ));
                                ui.end_row();

                                ui.label("Resolution:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.ssr.resolution,
                                    0.25..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Max Roughness:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.ssr.max_roughness,
                                    0.0..=1.0,
                                ));
                                ui.end_row();
                            });
                    }
                });

                ui.add_space(10.0);

                // Vignette
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.current_profile.vignette.enabled,
                        RichText::new("⭕ Vignette").strong(),
                    );

                    if self.current_profile.vignette.enabled {
                        egui::Grid::new("vignette_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Intensity:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.vignette.intensity,
                                    0.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Smoothness:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.vignette.smoothness,
                                    0.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Roundness:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.vignette.roundness,
                                    0.0..=1.0,
                                ));
                                ui.end_row();
                            });
                    }
                });

                ui.add_space(10.0);

                // Chromatic Aberration
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.current_profile.chromatic_aberration.enabled,
                        RichText::new("🌈 Chromatic Aberration").strong(),
                    );

                    if self.current_profile.chromatic_aberration.enabled {
                        ui.horizontal(|ui| {
                            ui.label("Intensity:");
                            ui.add(egui::Slider::new(
                                &mut self.current_profile.chromatic_aberration.intensity,
                                0.0..=1.0,
                            ));
                        });
                    }
                });

                ui.add_space(10.0);

                // Film Grain
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.current_profile.film_grain.enabled,
                        RichText::new("Film Grain").strong(),
                    );

                    if self.current_profile.film_grain.enabled {
                        egui::Grid::new("grain_settings")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Intensity:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.film_grain.intensity,
                                    0.0..=1.0,
                                ));
                                ui.end_row();

                                ui.label("Response:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_profile.film_grain.response,
                                    0.0..=1.0,
                                ));
                                ui.end_row();
                            });
                    }
                });
            });
    }

    fn show_presets_tab(&mut self, ui: &mut Ui) {
        ui.heading("Presets");
        ui.add_space(10.0);

        // Filter
        ui.horizontal(|ui| {
            ui.label("[Srch]");
            ui.text_edit_singleline(&mut self.preset_filter);
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                let mut current_category = String::new();

                for preset in &self.presets {
                    if !self.preset_filter.is_empty()
                        && !preset
                            .name
                            .to_lowercase()
                            .contains(&self.preset_filter.to_lowercase())
                    {
                        continue;
                    }

                    if preset.category != current_category {
                        current_category = preset.category.clone();
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(&current_category)
                                .strong()
                                .color(Color32::from_rgb(150, 150, 200)),
                        );
                    }

                    ui.horizontal(|ui| {
                        ui.label(&preset.name);
                        ui.label(
                            RichText::new(&preset.description)
                                .small()
                                .color(Color32::GRAY),
                        );

                        if ui.button("Apply").clicked() {
                            // Apply preset
                        }
                    });
                }
            });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("Save as Preset").clicked() {
                // Save current profile as preset
            }
            if ui.button("📤 Export").clicked() {
                // Export profile
            }
            if ui.button("Import").clicked() {
                // Import profile
            }
        });
    }

    // Getters for testing
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    pub fn preset_count(&self) -> usize {
        self.presets.len()
    }

    pub fn current_profile_name(&self) -> &str {
        &self.current_profile.name
    }

    pub fn is_bloom_enabled(&self) -> bool {
        self.current_profile.bloom.enabled
    }

    pub fn add_profile(&mut self, name: &str) -> u32 {
        let id = self.next_id();
        self.profiles.push(PostProcessProfile {
            id,
            name: name.to_string(),
            ..Default::default()
        });
        id
    }

    pub fn set_bloom_intensity(&mut self, intensity: f32) {
        self.current_profile.bloom.intensity = intensity.clamp(0.0, 2.0);
        self.settings_dirty = true;
    }

    pub fn set_exposure(&mut self, exposure: f32) {
        self.current_profile.color_grading.exposure = exposure.clamp(-5.0, 5.0);
        self.settings_dirty = true;
    }

    /// Returns `true` if any post-process setting was changed since the last
    /// call to [`take_pending_settings`].
    pub fn is_dirty(&self) -> bool {
        self.settings_dirty
    }

    /// If settings have changed, returns the current bloom settings and clears
    /// the dirty flag.  The caller is responsible for forwarding to the renderer.
    pub fn take_pending_settings(&mut self) -> Option<PostProcessPendingSettings> {
        if !self.settings_dirty {
            return None;
        }
        self.settings_dirty = false;
        let bloom = &self.current_profile.bloom;
        Some(PostProcessPendingSettings {
            bloom_enabled: bloom.enabled,
            bloom_intensity: bloom.intensity,
            bloom_threshold: bloom.threshold,
            bloom_soft_knee: bloom.soft_threshold,
            tonemap: self.current_profile.tonemapper,
        })
    }
}

impl Panel for PostProcessPanel {
    fn name(&self) -> &'static str {
        "Post-Processing"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            PostProcessTab::Overview => self.show_overview_tab(ui),
            PostProcessTab::Bloom => self.show_bloom_tab(ui),
            PostProcessTab::DepthOfField => self.show_dof_tab(ui),
            PostProcessTab::MotionBlur => self.show_motion_blur_tab(ui),
            PostProcessTab::ColorGrading => self.show_color_grading_tab(ui),
            PostProcessTab::Effects => self.show_effects_tab(ui),
            PostProcessTab::Presets => self.show_presets_tab(ui),
        }
    }

    fn update(&mut self) {
        // Update preview if needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_process_panel_creation() {
        let panel = PostProcessPanel::new();
        assert!(!panel.current_profile_name().is_empty());
    }

    #[test]
    fn test_default_sample_data() {
        let mut panel = PostProcessPanel::new();
        panel.create_sample_data();
        assert!(panel.profile_count() >= 3);
        assert!(panel.preset_count() >= 10);
    }

    #[test]
    fn test_add_profile() {
        let mut panel = PostProcessPanel::new();
        let initial_count = panel.profile_count();

        let id = panel.add_profile("Test Profile");
        assert!(id > 0);
        assert_eq!(panel.profile_count(), initial_count + 1);
    }

    #[test]
    fn test_bloom_enabled() {
        let panel = PostProcessPanel::new();
        assert!(panel.is_bloom_enabled());
    }

    #[test]
    fn test_set_bloom_intensity() {
        let mut panel = PostProcessPanel::new();
        panel.set_bloom_intensity(1.5);
        assert!((panel.current_profile.bloom.intensity - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_bloom_intensity_clamping() {
        let mut panel = PostProcessPanel::new();
        panel.set_bloom_intensity(5.0);
        assert!((panel.current_profile.bloom.intensity - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_set_exposure() {
        let mut panel = PostProcessPanel::new();
        panel.set_exposure(-2.0);
        assert!((panel.current_profile.color_grading.exposure - -2.0).abs() < 0.001);
    }

    #[test]
    fn test_tonemapper_list() {
        let tonemappers = Tonemapper::all();
        assert!(tonemappers.len() >= 6);
    }

    #[test]
    fn test_default_settings() {
        let bloom = BloomSettings::default();
        assert!(bloom.enabled);
        assert!((bloom.intensity - 0.5).abs() < 0.001);

        let dof = DepthOfFieldSettings::default();
        assert_eq!(dof.mode, DofMode::Disabled);
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = PostProcessPanel::new();
        assert_eq!(panel.name(), "Post-Processing");
    }

    // === Tonemapper Display and Hash Tests ===

    #[test]
    fn test_tonemapper_display() {
        for tm in Tonemapper::all() {
            let display = format!("{}", tm);
            assert!(display.contains(tm.name()));
        }
    }

    #[test]
    fn test_tonemapper_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tm in Tonemapper::all() {
            set.insert(*tm);
        }
        assert_eq!(set.len(), Tonemapper::all().len());
    }

    #[test]
    fn test_tonemapper_all() {
        let all = Tonemapper::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&Tonemapper::None));
        assert!(all.contains(&Tonemapper::ACES));
        assert!(all.contains(&Tonemapper::Filmic));
    }

    #[test]
    fn test_tonemapper_is_cinematic() {
        assert!(!Tonemapper::None.is_cinematic());
        assert!(!Tonemapper::Reinhard.is_cinematic());
        assert!(Tonemapper::ACES.is_cinematic());
        assert!(Tonemapper::Filmic.is_cinematic());
        assert!(Tonemapper::AgX.is_cinematic());
        assert!(!Tonemapper::Neutral.is_cinematic());
    }

    // === AntiAliasing Display and Hash Tests ===

    #[test]
    fn test_anti_aliasing_display() {
        for aa in AntiAliasing::all() {
            let display = format!("{}", aa);
            assert!(display.contains(aa.name()));
        }
    }

    #[test]
    fn test_anti_aliasing_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for aa in AntiAliasing::all() {
            set.insert(*aa);
        }
        assert_eq!(set.len(), AntiAliasing::all().len());
    }

    #[test]
    fn test_anti_aliasing_all() {
        let all = AntiAliasing::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&AntiAliasing::None));
        assert!(all.contains(&AntiAliasing::FXAA));
        assert!(all.contains(&AntiAliasing::MSAA8x));
    }

    #[test]
    fn test_anti_aliasing_is_msaa() {
        assert!(!AntiAliasing::None.is_msaa());
        assert!(!AntiAliasing::FXAA.is_msaa());
        assert!(AntiAliasing::MSAA2x.is_msaa());
        assert!(AntiAliasing::MSAA4x.is_msaa());
        assert!(AntiAliasing::MSAA8x.is_msaa());
    }

    #[test]
    fn test_anti_aliasing_is_post_process() {
        assert!(!AntiAliasing::None.is_post_process());
        assert!(AntiAliasing::FXAA.is_post_process());
        assert!(AntiAliasing::SMAA.is_post_process());
        assert!(AntiAliasing::TAA.is_post_process());
        assert!(!AntiAliasing::MSAA4x.is_post_process());
    }

    // === DofMode Display and Hash Tests ===

    #[test]
    fn test_dof_mode_display() {
        for mode in DofMode::all() {
            let display = format!("{}", mode);
            assert!(display.contains(mode.name()));
        }
    }

    #[test]
    fn test_dof_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in DofMode::all() {
            set.insert(*mode);
        }
        assert_eq!(set.len(), DofMode::all().len());
    }

    #[test]
    fn test_dof_mode_all() {
        let all = DofMode::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&DofMode::Disabled));
        assert!(all.contains(&DofMode::Gaussian));
        assert!(all.contains(&DofMode::Bokeh));
    }

    #[test]
    fn test_dof_mode_is_enabled() {
        assert!(!DofMode::Disabled.is_enabled());
        assert!(DofMode::Gaussian.is_enabled());
        assert!(DofMode::Bokeh.is_enabled());
        assert!(DofMode::CircleOfConfusion.is_enabled());
    }

    // === AoMethod Display and Hash Tests ===

    #[test]
    fn test_ao_method_display() {
        for method in AoMethod::all() {
            let display = format!("{}", method);
            assert!(display.contains(method.name()));
        }
    }

    #[test]
    fn test_ao_method_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for method in AoMethod::all() {
            set.insert(*method);
        }
        assert_eq!(set.len(), AoMethod::all().len());
    }

    #[test]
    fn test_ao_method_all() {
        let all = AoMethod::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&AoMethod::SSAO));
        assert!(all.contains(&AoMethod::HBAO));
        assert!(all.contains(&AoMethod::GTAO));
    }

    #[test]
    fn test_ao_method_description() {
        for method in AoMethod::all() {
            let desc = method.description();
            assert!(!desc.is_empty());
        }
    }

    // === PostProcessTab Display and Hash Tests ===

    #[test]
    fn test_post_process_tab_display() {
        for tab in PostProcessTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()));
        }
    }

    #[test]
    fn test_post_process_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in PostProcessTab::all() {
            set.insert(*tab);
        }
        assert_eq!(set.len(), PostProcessTab::all().len());
    }

    #[test]
    fn test_post_process_tab_all() {
        let all = PostProcessTab::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&PostProcessTab::Overview));
        assert!(all.contains(&PostProcessTab::Bloom));
        assert!(all.contains(&PostProcessTab::Presets));
    }
}
