//! First-run tutorial walkthrough — a guided 7-step overlay that teaches new
//! users the foundational development workflow.
//!
//! Steps:
//! 1. **Welcome** — Brief engine introduction
//! 2. **Create World** — Open the World Wizard
//! 3. **Viewport Controls** — Camera orbiting, panning, zooming
//! 4. **Place Objects** — Drag-and-drop from Asset Browser
//! 5. **Set the Mood** — Environment & weather presets
//! 6. **Add Life** — Gameplay presets & entities
//! 7. **Save & Next Steps** — Save scene, explore docs
//!
//! The tutorial renders as a floating tooltip-style overlay anchored to the
//! relevant editor region. It can be opened from Help → Tutorial or on first launch.

use egui::{Color32, RichText};

// ============================================================================
// TUTORIAL STEP
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TutorialStep {
    Welcome,
    CreateWorld,
    ViewportControls,
    PlaceObjects,
    SetMood,
    AddLife,
    SaveAndNext,
}

impl TutorialStep {
    pub const ALL: &'static [TutorialStep] = &[
        TutorialStep::Welcome,
        TutorialStep::CreateWorld,
        TutorialStep::ViewportControls,
        TutorialStep::PlaceObjects,
        TutorialStep::SetMood,
        TutorialStep::AddLife,
        TutorialStep::SaveAndNext,
    ];

    fn index(&self) -> usize {
        Self::ALL.iter().position(|s| s == self).unwrap_or(0)
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Welcome => "Welcome to AstraWeave!",
            Self::CreateWorld => "Create Your World",
            Self::ViewportControls => "Navigate the Viewport",
            Self::PlaceObjects => "Place Objects",
            Self::SetMood => "Set the Mood",
            Self::AddLife => "Add Life to Your World",
            Self::SaveAndNext => "Save & Next Steps",
        }
    }

    pub fn body(&self) -> &'static str {
        match self {
            Self::Welcome => {
                "\
AstraWeave is an AI-native game engine built for rapid world creation.\n\n\
This tutorial will walk you through building your first game world in \
under 10 minutes. You can restart this tutorial anytime from Help → Tutorial."
            }

            Self::CreateWorld => {
                "\
Open File → New World (Wizard)... to launch the World Creation Wizard.\n\n\
1. Pick a template (e.g. \"Lush Forest\")\n\
2. Optionally choose a gameplay genre\n\
3. Click through Terrain → Environment → Populate\n\
4. Hit \"Generate World\" — your terrain, lighting, and scatter \
   are created in one click!"
            }

            Self::ViewportControls => {
                "\
Navigate the 3D viewport with these controls:\n\n\
• Right-click + drag → Orbit camera\n\
• Middle-click + drag → Pan camera\n\
• Scroll wheel → Zoom in/out\n\
• W/A/S/D (while right-clicking) → Fly mode\n\
• F → Focus on selected entity\n\
• Gizmo handles → Move/Rotate/Scale objects"
            }

            Self::PlaceObjects => {
                "\
Open the Asset Browser panel (Window → Panels → Asset Browser).\n\n\
• Browse assets by category: Models, Textures, Audio, Prefabs\n\
• Drag any asset from the browser into the viewport\n\
• Objects drop at your cursor's 3D ground position\n\
• Use the Transform gizmo to fine-tune placement"
            }

            Self::SetMood => {
                "\
Open the World panel to configure the atmosphere:\n\n\
• Time of Day slider — dawn to midnight\n\
• Weather presets — Clear, Rain, Storm, Snow, Sandstorm\n\
• Particle intensity slider for precipitation\n\
• Fog density and color controls\n\
• Or use Environment Presets for one-click moods!"
            }

            Self::AddLife => {
                "\
In Project Settings → Gameplay, choose a genre preset:\n\n\
• Open-World RPG (inventory, quests, combat)\n\
• Top-Down RTS (unit selection, build queues)\n\
• Survival Builder (crafting, hunger, building)\n\
• Total War Sim (armies, diplomacy, formations)\n\n\
Each preset adds skeleton components and starter entities \
that you can flesh out for your game."
            }

            Self::SaveAndNext => {
                "\
Save your scene with Ctrl+S.\n\n\
What to explore next:\n\
• Material Editor — create custom PBR materials\n\
• Animation panel — timeline keyframe editing\n\
• Behavior Graph — visual AI logic editor\n\
• Cinematics — camera sequences and cutscenes\n\
• Build Manager — export to target platforms\n\n\
Happy building! 🎮"
            }
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Welcome => "[Star]",
            Self::CreateWorld => "[Globe]",
            Self::ViewportControls => "[Eye]",
            Self::PlaceObjects => "[Cube]",
            Self::SetMood => "[Sun]",
            Self::AddLife => "[Zap]",
            Self::SaveAndNext => "[Save]",
        }
    }
}

// ============================================================================
// TUTORIAL ACTION
// ============================================================================

/// Actions emitted by the tutorial overlay.
#[derive(Debug, Clone, PartialEq)]
pub enum TutorialAction {
    /// User requested to open the World Wizard (step 2 shortcut).
    OpenWorldWizard,
    /// Tutorial completed or dismissed.
    Completed,
    /// Tutorial skipped entirely.
    Skipped,
}

// ============================================================================
// TUTORIAL STATE
// ============================================================================

pub struct Tutorial {
    pub active: bool,
    step: TutorialStep,
}

impl Default for Tutorial {
    fn default() -> Self {
        Self {
            active: false,
            step: TutorialStep::Welcome,
        }
    }
}

impl Tutorial {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start the tutorial from the beginning.
    pub fn start(&mut self) {
        self.active = true;
        self.step = TutorialStep::Welcome;
    }

    /// Render the tutorial overlay. Returns an action when the user interacts.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<TutorialAction> {
        if !self.active {
            return None;
        }

        let mut action: Option<TutorialAction> = None;

        let step_idx = self.step.index();
        let total = TutorialStep::ALL.len();

        egui::Window::new("Tutorial")
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // Step indicator
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Step {} of {}", step_idx + 1, total))
                            .color(Color32::from_rgb(80, 160, 255))
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button(
                                RichText::new("Skip Tutorial")
                                    .color(Color32::from_rgb(160, 160, 170)),
                            )
                            .clicked()
                        {
                            action = Some(TutorialAction::Skipped);
                            self.active = false;
                        }
                    });
                });

                // Progress bar
                let progress = (step_idx + 1) as f32 / total as f32;
                let bar_w = ui.available_width();
                let (bar_rect, _) =
                    ui.allocate_exact_size(egui::vec2(bar_w, 4.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(bar_rect, 2.0, Color32::from_rgb(40, 40, 50));
                let fill_rect =
                    egui::Rect::from_min_size(bar_rect.min, egui::vec2(bar_w * progress, 4.0));
                ui.painter()
                    .rect_filled(fill_rect, 2.0, Color32::from_rgb(80, 160, 255));

                ui.add_space(12.0);

                // Title
                ui.label(
                    RichText::new(format!("{} {}", self.step.icon(), self.step.title()))
                        .size(18.0)
                        .strong(),
                );
                ui.add_space(6.0);

                // Body text
                ui.label(RichText::new(self.step.body()).color(Color32::from_rgb(200, 200, 215)));

                // Action shortcut button on step 2
                if self.step == TutorialStep::CreateWorld {
                    ui.add_space(8.0);
                    if ui
                        .button(
                            RichText::new("Open World Wizard Now")
                                .strong()
                                .color(Color32::from_rgb(80, 200, 120)),
                        )
                        .clicked()
                    {
                        action = Some(TutorialAction::OpenWorldWizard);
                    }
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(4.0);

                // Navigation buttons
                ui.horizontal(|ui| {
                    if step_idx > 0 {
                        if ui.button("← Back").clicked() {
                            self.step = TutorialStep::ALL[step_idx - 1];
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if step_idx == total - 1 {
                            // Final step
                            if ui
                                .button(
                                    RichText::new("Finish")
                                        .strong()
                                        .color(Color32::from_rgb(80, 200, 120)),
                                )
                                .clicked()
                            {
                                action = Some(TutorialAction::Completed);
                                self.active = false;
                            }
                        } else if ui.button("Next →").clicked() {
                            self.step = TutorialStep::ALL[step_idx + 1];
                        }
                    });
                });
            });

        action
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_steps_have_content() {
        for step in TutorialStep::ALL {
            assert!(!step.title().is_empty());
            assert!(!step.body().is_empty());
            assert!(!step.icon().is_empty());
        }
    }

    #[test]
    fn tutorial_default_is_inactive() {
        let t = Tutorial::new();
        assert!(!t.active);
        assert_eq!(t.step, TutorialStep::Welcome);
    }

    #[test]
    fn start_activates_tutorial() {
        let mut t = Tutorial::new();
        t.start();
        assert!(t.active);
        assert_eq!(t.step, TutorialStep::Welcome);
    }

    #[test]
    fn step_indices_are_sequential() {
        for (i, step) in TutorialStep::ALL.iter().enumerate() {
            assert_eq!(step.index(), i);
        }
    }
}
