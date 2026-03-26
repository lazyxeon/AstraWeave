//! Dock Panel Integration
//!
//! This module provides the bridge between the egui_dock system and
//! actual panel implementations. It contains context structures that
//! hold mutable references to panels and world state for rendering.

use crate::panel_type::PanelType;
use crate::panels::{
    AdvancedWidgetsPanel, AnimationPanel, AssetBrowser, AudioPanel, BuildManagerPanel, ChartsPanel,
    CinematicsPanel, ConsolePanel, DialogueEditorPanel, EntityPanel, FoliagePanel, GraphPanel,
    HierarchyPanel, InputBindingsPanel, LightingPanel, LocalizationPanel, LodConfigPanel,
    MaterialEditorPanel, NavigationPanel, NetworkingPanel, Panel, ParticleSystemPanel, PcgPanel,
    PerformancePanel, PhysicsPanel, PostProcessPanel, ProfilerPanel, ProjectSettingsPanel,
    SceneStatsPanel, SplineEditorPanel, TerrainPanel, ThemeManagerPanel, TransformPanel,
    UiEditorPanel, WorldPanel,
};
use astraweave_core::World;
use egui::Ui;
use egui_dock::egui;
use egui_dock::tab_viewer::OnCloseResponse;
use egui_dock::TabViewer;

/// Context containing all panel instances for dock rendering
///
/// This struct holds mutable references to all panels so the docking
/// system can dispatch rendering to the actual implementations.
pub struct DockPanelContext<'a> {
    // Core panels
    pub hierarchy_panel: &'a mut HierarchyPanel,
    pub console_panel: &'a mut ConsolePanel,
    pub profiler_panel: &'a mut ProfilerPanel,
    pub scene_stats_panel: &'a mut SceneStatsPanel,

    // Asset panels
    pub asset_browser: &'a mut AssetBrowser,

    // Entity/Transform panels
    pub entity_panel: &'a mut EntityPanel,
    pub transform_panel: &'a mut TransformPanel,

    // World panel
    pub world_panel: &'a mut WorldPanel,

    // Graph editors
    pub graph_panel: &'a mut GraphPanel,
    pub animation_panel: &'a mut AnimationPanel,

    // Performance/Debug panels
    pub performance_panel: &'a mut PerformancePanel,
    pub charts_panel: &'a mut ChartsPanel,
    pub advanced_widgets_panel: &'a mut AdvancedWidgetsPanel,

    // Build/Theme
    pub build_manager_panel: &'a mut BuildManagerPanel,
    pub theme_manager: &'a mut ThemeManagerPanel,

    // Terrain panel
    pub terrain_panel: &'a mut TerrainPanel,

    // === New Phase 8 SOTA Panels ===
    pub ui_editor_panel: &'a mut UiEditorPanel,
    pub foliage_panel: &'a mut FoliagePanel,
    pub spline_editor_panel: &'a mut SplineEditorPanel,
    pub lod_config_panel: &'a mut LodConfigPanel,
    pub localization_panel: &'a mut LocalizationPanel,
    pub project_settings_panel: &'a mut ProjectSettingsPanel,
    pub audio_panel: &'a mut AudioPanel,
    pub cinematics_panel: &'a mut CinematicsPanel,
    pub dialogue_editor_panel: &'a mut DialogueEditorPanel,
    pub lighting_panel: &'a mut LightingPanel,
    pub navigation_panel: &'a mut NavigationPanel,
    pub networking_panel: &'a mut NetworkingPanel,
    pub particle_system_panel: &'a mut ParticleSystemPanel,
    pub pcg_panel: &'a mut PcgPanel,
    pub physics_panel: &'a mut PhysicsPanel,
    pub post_process_panel: &'a mut PostProcessPanel,
    pub input_bindings_panel: &'a mut InputBindingsPanel,
    pub material_editor_panel: &'a mut MaterialEditorPanel,

    // World state
    pub world: Option<&'a mut World>,
    pub console_logs: &'a mut Vec<String>,

    // Selection state
    pub selected_entity: Option<u64>,
    pub is_playing: bool,
}

impl<'a> DockPanelContext<'a> {
    /// Render a panel based on its type
    pub fn render_panel(&mut self, ui: &mut Ui, panel_type: &PanelType) {
        match panel_type {
            PanelType::Hierarchy => {
                if let Some(world) = self.world.as_mut() {
                    self.hierarchy_panel.show_with_world(ui, world);
                } else {
                    ui.label("No world loaded");
                }
            }
            PanelType::Console => {
                self.console_panel.show_with_logs(ui, self.console_logs);
            }
            PanelType::Profiler => {
                self.profiler_panel.show(ui);
            }
            PanelType::SceneStats => {
                self.scene_stats_panel.show_inline(ui);
            }
            PanelType::AssetBrowser => {
                self.asset_browser.show(ui);
            }
            PanelType::EntityPanel => {
                self.entity_panel.show(ui);
            }
            PanelType::Transform => {
                self.transform_panel.show(ui);
            }
            PanelType::World => {
                self.world_panel.show(ui);
            }
            PanelType::Graph => {
                self.graph_panel.show(ui);
            }
            PanelType::Animation => {
                // Animation panel needs egui::Context, show placeholder
                ui.heading("Animation");
                ui.separator();
                ui.label("Animation timeline and keyframe editor");
                ui.label("(Full rendering requires egui::Context)");
            }
            PanelType::Performance => {
                self.performance_panel.show(ui);
            }
            PanelType::Charts => {
                self.charts_panel.show(ui);
            }
            PanelType::AdvancedWidgets => {
                self.advanced_widgets_panel.show(ui);
            }
            PanelType::BuildManager => {
                self.build_manager_panel.show(ui);
            }
            PanelType::ThemeManager => {
                self.theme_manager.show(ui);
            }
            PanelType::Inspector => {
                self.render_inspector(ui);
            }
            PanelType::Viewport => {
                self.render_viewport_placeholder(ui);
            }
            PanelType::BehaviorGraph => {
                // Behavior graph needs special context - show placeholder
                ui.heading("Behavior Graph");
                ui.separator();
                ui.label("Behavior graph editor");
                ui.label("(Requires additional context for full rendering)");
            }
            PanelType::MaterialEditor => {
                self.material_editor_panel.show(ui);
            }
            PanelType::Terrain => {
                self.terrain_panel.show(ui);
            }
            // === New Phase 8 SOTA Panels ===
            PanelType::UiEditor => {
                self.ui_editor_panel.show(ui);
            }
            PanelType::Foliage => {
                self.foliage_panel.show(ui);
            }
            PanelType::SplineEditor => {
                self.spline_editor_panel.show(ui);
            }
            PanelType::LodConfig => {
                self.lod_config_panel.show(ui);
            }
            PanelType::Localization => {
                self.localization_panel.show(ui);
            }
            PanelType::ProjectSettings => {
                self.project_settings_panel.show(ui);
            }
            PanelType::Audio => {
                self.audio_panel.show(ui);
            }
            PanelType::Cinematics => {
                self.cinematics_panel.show(ui);
            }
            PanelType::DialogueEditor => {
                self.dialogue_editor_panel.show(ui);
            }
            PanelType::Lighting => {
                self.lighting_panel.show(ui);
            }
            PanelType::Navigation => {
                self.navigation_panel.show(ui);
            }
            PanelType::Networking => {
                self.networking_panel.show(ui);
            }
            PanelType::ParticleSystem => {
                self.particle_system_panel.show(ui);
            }
            PanelType::Pcg => {
                self.pcg_panel.show(ui);
            }
            PanelType::Physics => {
                self.physics_panel.show(ui);
            }
            PanelType::PostProcess => {
                self.post_process_panel.show(ui);
            }
            PanelType::InputBindings => {
                self.input_bindings_panel.show(ui);
            }
            PanelType::FrameDebugger => {
                ui.heading("Frame Debugger");
                ui.separator();
                ui.label("Frame debugger is available in the docking layout.");
                ui.label("Switch to docking mode (Window > Use Docking Layout).");
            }
            PanelType::BlendImport => {
                ui.heading("Blend Import");
                ui.separator();
                ui.label("Import Blender .blend scenes, decompose into assets, and generate biome packs.");
            }
            PanelType::Blueprint => {
                ui.heading("Blueprint Zone Editor");
                ui.separator();
                ui.label("2D top-down zone editor for defining terrain generation zones.");
                ui.label("Switch to docking mode for the full interactive canvas.");
            }
        }
    }

    /// Render the inspector panel with entity details
    fn render_inspector(&mut self, ui: &mut Ui) {
        ui.heading("Inspector");
        ui.separator();

        if let Some(entity_id) = self.selected_entity {
            ui.label(format!("Selected Entity: {}", entity_id));
            ui.add_space(8.0);

            if let Some(world) = self.world.as_ref() {
                // Try to get entity details from world
                if let Ok(entity) = u32::try_from(entity_id) {
                    if let Some(name) = world.name(entity) {
                        ui.label(format!("Name: {}", name));
                    }
                    if let Some(pose) = world.pose(entity) {
                        ui.collapsing("Transform", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Position:");
                                ui.label(format!(
                                    "X: {}  H: {:.1}  Z: {}",
                                    pose.pos.x, pose.height, pose.pos.y
                                ));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Rotation:");
                                ui.label(format!("{:.1}°", pose.rotation.to_degrees()));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Scale:");
                                ui.label(format!("{:.2}", pose.scale));
                            });
                        });
                    }

                    // Show components
                    ui.collapsing("Components", |ui| {
                        ui.label("• Pose");
                        if world.health(entity).is_some() {
                            ui.label("• Health");
                        }
                        if world.team(entity).is_some() {
                            ui.label("• Team");
                        }
                    });
                }
            } else {
                ui.label("(No world context)");
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No entity selected");
            });
        }
    }

    /// Render viewport placeholder (actual viewport renders separately)
    fn render_viewport_placeholder(&self, ui: &mut Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading("3D Viewport");
                ui.add_space(10.0);
                ui.label("The 3D viewport renders in its own surface.");
                ui.add_space(10.0);
                if self.is_playing {
                    ui.colored_label(egui::Color32::GREEN, "⏵ Simulation Running");
                } else {
                    ui.label("Edit Mode");
                }
            });
        });
    }
}

/// A simpler context for when full panel access isn't available
pub struct MinimalPanelContext {
    pub selected_entity: Option<u64>,
    pub is_playing: bool,
}

impl MinimalPanelContext {
    pub fn new() -> Self {
        Self {
            selected_entity: None,
            is_playing: false,
        }
    }

    /// Render a panel with minimal context (placeholders)
    pub fn render_panel(&self, ui: &mut Ui, panel_type: &PanelType) {
        match panel_type {
            PanelType::Viewport => {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.heading("3D Viewport");
                        ui.add_space(10.0);
                        ui.label("The 3D viewport renders separately.");
                        if self.is_playing {
                            ui.colored_label(egui::Color32::GREEN, "⏵ Simulation Running");
                        }
                    });
                });
            }
            PanelType::Inspector => {
                ui.heading("Inspector");
                ui.separator();
                if let Some(entity_id) = self.selected_entity {
                    ui.label(format!("Selected Entity: {}", entity_id));
                    ui.collapsing("Transform", |ui| {
                        ui.label("Position: (0, 0)");
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No entity selected");
                    });
                }
            }
            PanelType::Hierarchy => {
                ui.heading("Hierarchy");
                ui.separator();
                ui.label("(Requires world context)");
            }
            PanelType::Console => {
                ui.heading("Console");
                ui.separator();
                ui.label("(Requires log context)");
            }
            _ => {
                ui.heading(panel_type.title());
                ui.separator();
                ui.label(format!("{} panel", panel_type.title()));
            }
        }
    }
}

impl Default for MinimalPanelContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Tab viewer that uses DockPanelContext for full panel rendering
///
/// This implements `egui_dock::TabViewer` and dispatches rendering
/// to actual panel implementations via `DockPanelContext`.
pub struct ContextualTabViewer<'a> {
    pub context: DockPanelContext<'a>,
    pub panels_to_close: Vec<PanelType>,
    pub panels_to_add: Vec<PanelType>,
    pub entity_selected: Option<u64>,
}

impl<'a> ContextualTabViewer<'a> {
    pub fn new(context: DockPanelContext<'a>) -> Self {
        Self {
            context,
            panels_to_close: Vec::new(),
            panels_to_add: Vec::new(),
            entity_selected: None,
        }
    }

    pub fn take_closed_panels(&mut self) -> Vec<PanelType> {
        std::mem::take(&mut self.panels_to_close)
    }

    pub fn take_panels_to_add(&mut self) -> Vec<PanelType> {
        std::mem::take(&mut self.panels_to_add)
    }
}

impl<'a> TabViewer for ContextualTabViewer<'a> {
    type Tab = PanelType;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        format!("{} {}", tab.icon(), tab.title()).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        // Play mode indicator
        if self.context.is_playing {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "> Play Mode");
            });
            ui.separator();
        }

        // Dispatch to actual panel rendering via context
        self.context.render_panel(ui, tab);
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        tab.is_closable()
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        if tab.has_scroll() {
            [true, true]
        } else {
            [false, false]
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(format!("dock_panel_{:?}", tab))
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        if tab.is_closable() {
            self.panels_to_close.push(*tab);
            OnCloseResponse::Close
        } else {
            OnCloseResponse::Ignore
        }
    }

    fn add_popup(
        &mut self,
        ui: &mut egui::Ui,
        _surface: egui_dock::SurfaceIndex,
        _node: egui_dock::NodeIndex,
    ) {
        ui.heading("Add Panel");
        ui.separator();

        // Group panels by category
        ui.collapsing("Core", |ui| {
            for panel in [
                PanelType::Viewport,
                PanelType::Inspector,
                PanelType::Hierarchy,
            ] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });

        ui.collapsing("Assets & World", |ui| {
            for panel in [
                PanelType::AssetBrowser,
                PanelType::World,
                PanelType::EntityPanel,
            ] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });

        ui.collapsing("Debug & Profiling", |ui| {
            for panel in [
                PanelType::Console,
                PanelType::Profiler,
                PanelType::SceneStats,
                PanelType::Performance,
            ] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });

        ui.collapsing("Editors", |ui| {
            for panel in [
                PanelType::Animation,
                PanelType::Graph,
                PanelType::BehaviorGraph,
                PanelType::Charts,
                PanelType::MaterialEditor,
                PanelType::UiEditor,
                PanelType::SplineEditor,
                PanelType::DialogueEditor,
            ] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });

        ui.collapsing("Environment & Rendering", |ui| {
            for panel in [
                PanelType::Terrain,
                PanelType::Foliage,
                PanelType::Lighting,
                PanelType::PostProcess,
                PanelType::ParticleSystem,
                PanelType::Cinematics,
            ] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });

        ui.collapsing("AI & Navigation", |ui| {
            for panel in [PanelType::Navigation, PanelType::Pcg] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });

        ui.collapsing("Audio & Physics", |ui| {
            for panel in [PanelType::Audio, PanelType::Physics] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });

        ui.collapsing("Configuration", |ui| {
            for panel in [
                PanelType::ProjectSettings,
                PanelType::LodConfig,
                PanelType::Localization,
                PanelType::InputBindings,
                PanelType::Networking,
            ] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });

        ui.collapsing("Build & Tools", |ui| {
            for panel in [
                PanelType::BuildManager,
                PanelType::ThemeManager,
                PanelType::AdvancedWidgets,
            ] {
                if ui
                    .button(format!("{} {}", panel.icon(), panel.title()))
                    .clicked()
                {
                    self.panels_to_add.push(panel);
                    ui.close();
                }
            }
        });
    }
}
