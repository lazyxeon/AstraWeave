//! World Creation Wizard — a multi-step modal dialog that guides users through
//! creating a fully populated world in under 60 seconds.
//!
//! Steps:
//! 1. **Template** — Choose a world template (pre-configured biome + environment)
//! 2. **Terrain** — Fine-tune biome, seed, and world size
//! 3. **Environment** — Adjust atmosphere, lighting, and weather
//! 4. **Populate** — Configure scatter density and area
//! 5. **Review** — Summary and "Generate" button
//!
//! On completion the wizard emits a [`WorldWizardAction`] which the editor
//! translates into the existing [`FillerAction::GenerateFullScene`] pipeline.

use egui::{Color32, RichText, Ui, Vec2};
use tracing::info;

use super::gameplay_presets::{self, GameplayPreset};
use super::procedural_filler_panel::{BiomePreset, EnvironmentPreset, FillerAction};

// ============================================================================
// WORLD TEMPLATE — curated starting points
// ============================================================================

/// Pre-built world templates that map to a `BiomePreset` + `EnvironmentPreset` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldTemplate {
    LushForest,
    DesertWasteland,
    FrozenTundra,
    TropicalCoast,
    VolcanicWasteland,
    MysticWoods,
    Custom,
}

impl WorldTemplate {
    pub const ALL: &'static [WorldTemplate] = &[
        WorldTemplate::LushForest,
        WorldTemplate::DesertWasteland,
        WorldTemplate::FrozenTundra,
        WorldTemplate::TropicalCoast,
        WorldTemplate::VolcanicWasteland,
        WorldTemplate::MysticWoods,
        WorldTemplate::Custom,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::LushForest => "Lush Forest",
            Self::DesertWasteland => "Desert Wasteland",
            Self::FrozenTundra => "Frozen Tundra",
            Self::TropicalCoast => "Tropical Coast",
            Self::VolcanicWasteland => "Volcanic Wasteland",
            Self::MysticWoods => "Mystic Woods",
            Self::Custom => "Custom",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::LushForest => "Dense temperate forest with golden-hour lighting. Perfect for RPGs and exploration games.",
            Self::DesertWasteland => "Sweeping sand dunes under a blazing sun. Ideal for survival and open-world games.",
            Self::FrozenTundra => "Snow-covered arctic landscape with moonlit atmosphere. Great for stealth and horror.",
            Self::TropicalCoast => "Palm-lined beaches with crystal-clear skies. Best for adventure and sandbox games.",
            Self::VolcanicWasteland => "Scorched earth with fiery skies. Suited for war simulations and action games.",
            Self::MysticWoods => "Enchanted forest with fantasy lighting. Made for RPGs and story-driven games.",
            Self::Custom => "Start from scratch with full control over every setting.",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::LushForest => "🌲",
            Self::DesertWasteland => "🏜️",
            Self::FrozenTundra => "❄️",
            Self::TropicalCoast => "🌴",
            Self::VolcanicWasteland => "🌋",
            Self::MysticWoods => "[Orb]",
            Self::Custom => "⚙️",
        }
    }

    /// The primary biome for terrain generation.
    pub fn biome(&self) -> BiomePreset {
        match self {
            Self::LushForest => BiomePreset::TemperateForest,
            Self::DesertWasteland => BiomePreset::DesertDunes,
            Self::FrozenTundra => BiomePreset::ArcticTundra,
            Self::TropicalCoast => BiomePreset::MediterraneanCoast,
            Self::VolcanicWasteland => BiomePreset::VolcanicWasteland,
            Self::MysticWoods => BiomePreset::MysticWoods,
            Self::Custom => BiomePreset::Custom,
        }
    }

    /// The default environment/atmosphere for this template.
    pub fn environment(&self) -> EnvironmentPreset {
        match self {
            Self::LushForest => EnvironmentPreset::GoldenHour,
            Self::DesertWasteland => EnvironmentPreset::SunnyDay,
            Self::FrozenTundra => EnvironmentPreset::Moonlit,
            Self::TropicalCoast => EnvironmentPreset::SunnyDay,
            Self::VolcanicWasteland => EnvironmentPreset::Stormy,
            Self::MysticWoods => EnvironmentPreset::Fantasy,
            Self::Custom => EnvironmentPreset::SunnyDay,
        }
    }

    /// A palette of 4 representative colours used for the preview card.
    pub fn palette(&self) -> [Color32; 4] {
        match self {
            Self::LushForest => [
                Color32::from_rgb(34, 100, 40),  // forest green
                Color32::from_rgb(60, 140, 50),  // canopy
                Color32::from_rgb(200, 170, 80), // golden light
                Color32::from_rgb(90, 70, 50),   // bark
            ],
            Self::DesertWasteland => [
                Color32::from_rgb(210, 180, 120), // sand
                Color32::from_rgb(170, 130, 80),  // dune shadow
                Color32::from_rgb(240, 200, 100), // sun glare
                Color32::from_rgb(130, 90, 60),   // rock
            ],
            Self::FrozenTundra => [
                Color32::from_rgb(200, 220, 240), // ice
                Color32::from_rgb(160, 180, 210), // shadow
                Color32::from_rgb(40, 50, 80),    // night sky
                Color32::from_rgb(240, 245, 255), // snow
            ],
            Self::TropicalCoast => [
                Color32::from_rgb(40, 170, 200),  // ocean
                Color32::from_rgb(240, 220, 160), // sand
                Color32::from_rgb(20, 130, 60),   // palm
                Color32::from_rgb(80, 200, 230),  // sky
            ],
            Self::VolcanicWasteland => [
                Color32::from_rgb(60, 30, 20),  // charred earth
                Color32::from_rgb(200, 60, 20), // lava
                Color32::from_rgb(100, 50, 30), // basalt
                Color32::from_rgb(40, 20, 10),  // ash
            ],
            Self::MysticWoods => [
                Color32::from_rgb(60, 40, 100),   // twilight
                Color32::from_rgb(100, 60, 160),  // magic glow
                Color32::from_rgb(30, 80, 50),    // deep moss
                Color32::from_rgb(180, 140, 220), // fairy light
            ],
            Self::Custom => [
                Color32::from_rgb(80, 80, 80),
                Color32::from_rgb(120, 120, 120),
                Color32::from_rgb(160, 160, 160),
                Color32::from_rgb(100, 100, 100),
            ],
        }
    }

    /// Default area radius for this template.
    pub fn default_area_radius(&self) -> f32 {
        match self {
            Self::DesertWasteland | Self::FrozenTundra => 256.0,
            Self::Custom => 128.0,
            _ => 192.0,
        }
    }
}

// ============================================================================
// WIZARD STEP
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Template,
    Terrain,
    Environment,
    Populate,
    Review,
}

impl WizardStep {
    const ALL: &'static [WizardStep] = &[
        WizardStep::Template,
        WizardStep::Terrain,
        WizardStep::Environment,
        WizardStep::Populate,
        WizardStep::Review,
    ];

    fn index(&self) -> usize {
        Self::ALL.iter().position(|s| s == self).unwrap_or(0)
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Template => "Template",
            Self::Terrain => "Terrain",
            Self::Environment => "Environment",
            Self::Populate => "Populate",
            Self::Review => "Review",
        }
    }
}

// ============================================================================
// WIZARD ACTION — output
// ============================================================================

/// Actions produced when the wizard completes or is cancelled.
#[derive(Debug, Clone)]
pub enum WorldWizardAction {
    /// User clicked "Generate World" on the review page.
    Generate {
        template: WorldTemplate,
        gameplay: GameplayPreset,
        filler_action: FillerAction,
    },
    /// User closed the wizard without generating.
    Cancelled,
}

// ============================================================================
// WIZARD STATE
// ============================================================================

pub struct WorldWizard {
    pub open: bool,
    step: WizardStep,

    // Template page
    selected_template: WorldTemplate,

    // Gameplay genre
    gameplay_preset: GameplayPreset,

    // Terrain page
    biome: BiomePreset,
    seed: u64,
    seed_text: String,
    area_radius: f32,

    // Environment page
    environment: EnvironmentPreset,

    // Populate page — scatter density multiplier 0.0 .. 2.0
    scatter_density: f32,
    populate_enabled: bool,
}

impl Default for WorldWizard {
    fn default() -> Self {
        Self {
            open: false,
            step: WizardStep::Template,
            selected_template: WorldTemplate::LushForest,
            gameplay_preset: GameplayPreset::Custom,
            biome: WorldTemplate::LushForest.biome(),
            seed: 42,
            seed_text: "42".into(),
            area_radius: WorldTemplate::LushForest.default_area_radius(),
            environment: WorldTemplate::LushForest.environment(),
            scatter_density: 1.0,
            populate_enabled: true,
        }
    }
}

impl WorldWizard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the wizard, resetting to the first step.
    pub fn open(&mut self) {
        self.open = true;
        self.step = WizardStep::Template;
    }

    /// Render the wizard modal. Returns an action when the user completes or cancels.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<WorldWizardAction> {
        if !self.open {
            return None;
        }

        let mut action: Option<WorldWizardAction> = None;

        // Semi-transparent overlay
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("world_wizard_overlay"))
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let painter = ui.painter();
                painter.rect_filled(screen, 0.0, Color32::from_black_alpha(160));
            });

        let mut still_open = true;

        egui::Window::new("New World Wizard")
            .collapsible(false)
            .resizable(false)
            .default_width(680.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut still_open)
            .show(ctx, |ui| {
                // ── Step indicator bar ──
                self.render_step_bar(ui);
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // ── Page content (scrollable) ──
                egui::ScrollArea::vertical()
                    .max_height(480.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.step {
                        WizardStep::Template => self.page_template(ui),
                        WizardStep::Terrain => self.page_terrain(ui),
                        WizardStep::Environment => self.page_environment(ui),
                        WizardStep::Populate => self.page_populate(ui),
                        WizardStep::Review => self.page_review(ui),
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);

                // ── Navigation buttons ──
                ui.horizontal(|ui| {
                    // Cancel
                    if ui.button("Cancel").clicked() {
                        action = Some(WorldWizardAction::Cancelled);
                        self.open = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let step_idx = self.step.index();

                        if step_idx == WizardStep::ALL.len() - 1 {
                            // Final step — Generate
                            if ui
                                .button(
                                    RichText::new("Generate World")
                                        .strong()
                                        .color(Color32::from_rgb(80, 200, 120)),
                                )
                                .clicked()
                            {
                                info!(template = ?self.selected_template, "world_wizard: world created");
                                action = Some(WorldWizardAction::Generate {
                                    template: self.selected_template,
                                    gameplay: self.gameplay_preset,
                                    filler_action: self.build_filler_action(),
                                });
                                self.open = false;
                            }
                        } else if ui.button("Next →").clicked() {
                            info!(from = self.step.label(), to = WizardStep::ALL[step_idx + 1].label(), "world_wizard: step completed");
                            self.step = WizardStep::ALL[step_idx + 1];
                        }

                        if step_idx > 0 {
                            if ui.button("← Back").clicked() {
                                self.step = WizardStep::ALL[step_idx - 1];
                            }
                        }
                    });
                });
            });

        if !still_open {
            self.open = false;
            action = Some(WorldWizardAction::Cancelled);
        }

        action
    }

    // ────────────────────────────────────────────────────────────────────
    // Step indicator
    // ────────────────────────────────────────────────────────────────────

    fn render_step_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for (i, step) in WizardStep::ALL.iter().enumerate() {
                let is_current = *step == self.step;
                let is_past = step.index() < self.step.index();

                let color = if is_current {
                    Color32::from_rgb(80, 160, 255)
                } else if is_past {
                    Color32::from_rgb(80, 200, 120)
                } else {
                    Color32::from_rgb(120, 120, 130)
                };

                let label = format!("{}. {}", i + 1, step.label());
                let text = if is_current {
                    RichText::new(label).strong().color(color)
                } else {
                    RichText::new(label).color(color)
                };

                ui.label(text);
                if i < WizardStep::ALL.len() - 1 {
                    ui.label(RichText::new("→").color(Color32::from_rgb(80, 80, 90)));
                }
            }
        });
    }

    // ────────────────────────────────────────────────────────────────────
    // Page 1: Template selection
    // ────────────────────────────────────────────────────────────────────

    fn page_template(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Choose a World Template").size(18.0).strong());
        ui.label("Select a starting point. You can fine-tune everything on the next pages.");
        ui.add_space(8.0);

        let card_size = Vec2::new(200.0, 140.0);

        // Lay out template cards in a wrapping grid
        ui.horizontal_wrapped(|ui| {
            for template in WorldTemplate::ALL {
                let is_selected = *template == self.selected_template;
                let response = self.render_template_card(ui, *template, card_size, is_selected);
                if response.clicked() {
                    self.selected_template = *template;
                    // Auto-fill downstream settings from template
                    self.biome = template.biome();
                    self.environment = template.environment();
                    self.area_radius = template.default_area_radius();
                }
            }
        });

        // Selected template description
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    self.selected_template.icon(),
                    self.selected_template.name()
                ))
                .strong()
                .size(14.0),
            );
            ui.label(self.selected_template.description());
        });

        // Gameplay genre preset
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        if let Some(new_preset) = gameplay_presets::show_preset_selector(ui, self.gameplay_preset) {
            self.gameplay_preset = new_preset;
        }
    }

    fn render_template_card(
        &self,
        ui: &mut Ui,
        template: WorldTemplate,
        size: Vec2,
        selected: bool,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let painter = ui.painter_at(rect);

        // Background
        let bg = if selected {
            Color32::from_rgb(35, 50, 75)
        } else if response.hovered() {
            Color32::from_rgb(40, 40, 50)
        } else {
            Color32::from_rgb(28, 28, 36)
        };
        let stroke = if selected {
            egui::Stroke::new(2.0, Color32::from_rgb(80, 160, 255))
        } else {
            egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 60))
        };
        painter.rect(rect, 6.0, bg, stroke, egui::StrokeKind::Outside);

        // Colour palette preview (4 horizontal bands in top half)
        let palette = template.palette();
        let band_h = rect.height() * 0.4 / 4.0;
        for (i, color) in palette.iter().enumerate() {
            let band = egui::Rect::from_min_size(
                rect.min + egui::vec2(4.0, 4.0 + i as f32 * band_h),
                egui::vec2(rect.width() - 8.0, band_h),
            );
            painter.rect_filled(band, 2.0, *color);
        }

        // Icon + name
        let text_y = rect.min.y + rect.height() * 0.50;
        let icon_pos = egui::pos2(rect.min.x + 8.0, text_y);
        painter.text(
            icon_pos,
            egui::Align2::LEFT_TOP,
            template.icon(),
            egui::FontId::proportional(18.0),
            Color32::WHITE,
        );

        let name_pos = egui::pos2(rect.min.x + 8.0, text_y + 22.0);
        painter.text(
            name_pos,
            egui::Align2::LEFT_TOP,
            template.name(),
            egui::FontId::proportional(13.0),
            if selected {
                Color32::from_rgb(180, 210, 255)
            } else {
                Color32::from_rgb(200, 200, 210)
            },
        );

        // Brief one-liner
        let desc_pos = egui::pos2(rect.min.x + 8.0, text_y + 40.0);
        let short_desc = match template {
            WorldTemplate::LushForest => "Forest + Golden Hour",
            WorldTemplate::DesertWasteland => "Dunes + Blazing Sun",
            WorldTemplate::FrozenTundra => "Arctic + Moonlit",
            WorldTemplate::TropicalCoast => "Beach + Clear Sky",
            WorldTemplate::VolcanicWasteland => "Lava + Storm",
            WorldTemplate::MysticWoods => "Enchanted + Fantasy",
            WorldTemplate::Custom => "Blank Canvas",
        };
        painter.text(
            desc_pos,
            egui::Align2::LEFT_TOP,
            short_desc,
            egui::FontId::proportional(10.0),
            Color32::from_rgb(140, 140, 155),
        );

        response
    }

    // ────────────────────────────────────────────────────────────────────
    // Page 2: Terrain settings
    // ────────────────────────────────────────────────────────────────────

    fn page_terrain(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Terrain Settings").size(18.0).strong());
        ui.label("Fine-tune the terrain generation parameters.");
        ui.add_space(8.0);

        egui::Grid::new("terrain_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                // Biome
                ui.label("Biome:");
                egui::ComboBox::from_id_salt("wizard_biome")
                    .selected_text(format!("{} {}", self.biome.icon(), self.biome.name()))
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for preset in BiomePreset::all() {
                            let label = format!("{} {}", preset.icon(), preset.name());
                            ui.selectable_value(&mut self.biome, *preset, label);
                        }
                    });
                ui.end_row();

                // Seed
                ui.label("Seed:");
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.seed_text)
                            .desired_width(140.0)
                            .hint_text("Numeric seed"),
                    );
                    if resp.changed() {
                        if let Ok(v) = self.seed_text.parse::<u64>() {
                            self.seed = v;
                        }
                    }
                    if ui.button("Randomize").clicked() {
                        self.seed = rand_seed();
                        self.seed_text = self.seed.to_string();
                    }
                });
                ui.end_row();

                // Area radius
                ui.label("World Radius:");
                ui.add(
                    egui::Slider::new(&mut self.area_radius, 64.0..=512.0)
                        .suffix(" m")
                        .logarithmic(true),
                );
                ui.end_row();
            });

        // Biome info box
        ui.add_space(8.0);
        ui.group(|ui| {
            let pc = self.biome.primary_color();
            let color = Color32::from_rgb(
                (pc[0] * 255.0) as u8,
                (pc[1] * 255.0) as u8,
                (pc[2] * 255.0) as u8,
            );
            ui.horizontal(|ui| {
                let (swatch, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                ui.painter_at(swatch).rect_filled(swatch, 3.0, color);
                ui.label(
                    RichText::new(format!("{} {}", self.biome.icon(), self.biome.name())).strong(),
                );
            });
            ui.label(format!(
                "Tree density: {:.1}  |  Rock density: {:.1}  |  Water: {}",
                self.biome.tree_density(),
                self.biome.rock_density(),
                if self.biome.has_water() { "Yes" } else { "No" }
            ));
        });
    }

    // ────────────────────────────────────────────────────────────────────
    // Page 3: Environment / atmosphere
    // ────────────────────────────────────────────────────────────────────

    fn page_environment(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new("Environment & Atmosphere")
                .size(18.0)
                .strong(),
        );
        ui.label("Set the mood and lighting for your world.");
        ui.add_space(8.0);

        egui::Grid::new("env_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Atmosphere:");
                egui::ComboBox::from_id_salt("wizard_env")
                    .selected_text(format!(
                        "{} {}",
                        self.environment.icon(),
                        self.environment.name()
                    ))
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for preset in EnvironmentPreset::all() {
                            let label = format!("{} {}", preset.icon(), preset.name());
                            ui.selectable_value(&mut self.environment, *preset, label);
                        }
                    });
                ui.end_row();
            });

        // Visual preview — grid of all environment presets as selectable chips
        ui.add_space(8.0);
        ui.label("Quick-Select:");
        ui.horizontal_wrapped(|ui| {
            for preset in EnvironmentPreset::all() {
                let is_selected = *preset == self.environment;
                let text = format!("{} {}", preset.icon(), preset.name());
                let rt = if is_selected {
                    RichText::new(text)
                        .strong()
                        .color(Color32::from_rgb(80, 200, 120))
                } else {
                    RichText::new(text).color(Color32::from_rgb(180, 180, 190))
                };
                if ui.selectable_label(is_selected, rt).clicked() {
                    self.environment = *preset;
                }
            }
        });
    }

    // ────────────────────────────────────────────────────────────────────
    // Page 4: Populate / scatter
    // ────────────────────────────────────────────────────────────────────

    fn page_populate(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Populate World").size(18.0).strong());
        ui.label("Control how densely the world is filled with vegetation, rocks, and props.");
        ui.add_space(8.0);

        ui.checkbox(&mut self.populate_enabled, "Enable auto-populate");

        if self.populate_enabled {
            ui.add_space(4.0);
            egui::Grid::new("populate_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Scatter Density:");
                    ui.add(
                        egui::Slider::new(&mut self.scatter_density, 0.1..=3.0)
                            .text("x")
                            .fixed_decimals(1),
                    );
                    ui.end_row();
                });

            ui.add_space(4.0);
            ui.label(
                RichText::new(density_description(self.scatter_density))
                    .color(Color32::from_rgb(160, 160, 175)),
            );
        } else {
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Terrain will be generated but left empty — you place objects manually.",
                )
                .color(Color32::from_rgb(160, 160, 175)),
            );
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // Page 5: Review & Generate
    // ────────────────────────────────────────────────────────────────────

    fn page_review(&self, ui: &mut Ui) {
        ui.label(RichText::new("Review & Generate").size(18.0).strong());
        ui.label("Confirm your settings and generate the world.");
        ui.add_space(8.0);

        egui::Grid::new("review_grid")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(RichText::new("Template:").strong());
                ui.label(format!(
                    "{} {}",
                    self.selected_template.icon(),
                    self.selected_template.name()
                ));
                ui.end_row();

                ui.label(RichText::new("Gameplay:").strong());
                ui.label(format!(
                    "{} {}",
                    self.gameplay_preset.icon(),
                    self.gameplay_preset.name()
                ));
                ui.end_row();

                ui.label(RichText::new("Biome:").strong());
                ui.label(format!("{} {}", self.biome.icon(), self.biome.name()));
                ui.end_row();

                ui.label(RichText::new("Seed:").strong());
                ui.label(self.seed.to_string());
                ui.end_row();

                ui.label(RichText::new("World Radius:").strong());
                ui.label(format!("{:.0} m", self.area_radius));
                ui.end_row();

                ui.label(RichText::new("Atmosphere:").strong());
                ui.label(format!(
                    "{} {}",
                    self.environment.icon(),
                    self.environment.name()
                ));
                ui.end_row();

                ui.label(RichText::new("Auto-Populate:").strong());
                if self.populate_enabled {
                    ui.label(format!("Yes (density {:.1}x)", self.scatter_density));
                } else {
                    ui.label("No — manual placement");
                }
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.label(
            RichText::new(
                "Click \"Generate World\" to create your world. This may take a few seconds.",
            )
            .color(Color32::from_rgb(140, 190, 255)),
        );
    }

    // ────────────────────────────────────────────────────────────────────
    // Build the FillerAction for the generate step
    // ────────────────────────────────────────────────────────────────────

    fn build_filler_action(&self) -> FillerAction {
        FillerAction::GenerateFullScene {
            seed: self.seed,
            biome: self.biome,
            environment: self.environment,
            area_radius: self.area_radius,
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn density_description(d: f32) -> &'static str {
    if d < 0.4 {
        "Sparse — scattered objects, lots of open space."
    } else if d < 0.9 {
        "Light — gentle coverage, comfortable exploration."
    } else if d < 1.4 {
        "Normal — balanced density for most game types."
    } else if d < 2.2 {
        "Dense — thick vegetation and frequent obstacles."
    } else {
        "Overgrown — maximum foliage, jungle-like coverage."
    }
}

/// Simple deterministic pseudo-random seed from system time.
fn rand_seed() -> u64 {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    // Combine nanos and secs for reasonable entropy
    dur.as_nanos() as u64 ^ dur.as_secs().wrapping_mul(6364136223846793005)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_mappings_are_consistent() {
        for t in WorldTemplate::ALL {
            // Every template must have a valid biome and environment
            let _ = t.biome();
            let _ = t.environment();
            assert!(!t.name().is_empty());
            assert!(!t.description().is_empty());
            assert!(t.default_area_radius() >= 64.0);
        }
    }

    #[test]
    fn wizard_default_is_valid() {
        let wiz = WorldWizard::new();
        assert!(!wiz.open);
        assert_eq!(wiz.step, WizardStep::Template);
        assert_eq!(wiz.selected_template, WorldTemplate::LushForest);
    }

    #[test]
    fn build_filler_action_produces_full_scene() {
        let wiz = WorldWizard::new();
        let action = wiz.build_filler_action();
        match action {
            FillerAction::GenerateFullScene {
                seed,
                biome,
                environment,
                area_radius,
            } => {
                assert_eq!(seed, 42);
                assert_eq!(biome, BiomePreset::TemperateForest);
                assert_eq!(environment, EnvironmentPreset::GoldenHour);
                assert!(area_radius > 0.0);
            }
            _ => panic!("Expected GenerateFullScene"),
        }
    }

    #[test]
    fn rand_seed_produces_varying_values() {
        let s1 = rand_seed();
        // Busy-wait a tiny bit to advance the clock
        let mut _x = 0u64;
        for i in 0..10000 {
            _x = _x.wrapping_add(i);
        }
        let s2 = rand_seed();
        // They should differ (extremely unlikely to collide)
        assert_ne!(s1, s2);
    }
}
