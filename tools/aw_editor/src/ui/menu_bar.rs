use crate::panel_type::PanelType;
use egui::Ui;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlignDirection {
    Left,
    Right,
    Top,
    Bottom,
    CenterX,
    CenterZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DistributeDirection {
    X,
    Z,
}

/// Handler for menu bar actions
pub trait MenuActionHandler {
    fn on_new(&mut self);
    fn on_new_world_wizard(&mut self);
    fn on_open(&mut self);
    fn on_save(&mut self);
    fn on_save_json(&mut self);
    fn on_save_scene(&mut self);
    fn on_load_scene(&mut self);
    fn on_exit(&mut self);

    fn on_undo(&mut self);
    fn on_redo(&mut self);
    fn on_delete(&mut self);

    fn selection_count(&self) -> usize;

    fn on_apply_material(&mut self);
    fn on_group_selection(&mut self);
    fn on_ungroup_selection(&mut self);
    fn on_align_selection(&mut self, dir: AlignDirection);
    fn on_distribute_selection(&mut self, dir: DistributeDirection);
    fn on_select_all(&mut self);
    fn on_deselect_all(&mut self);

    // Recent Files
    fn get_recent_files(&self) -> Vec<PathBuf>;
    fn on_open_recent(&mut self, path: PathBuf);
    fn on_clear_recent(&mut self);

    // View
    fn is_view_hierarchy_open(&self) -> bool;
    fn toggle_view_hierarchy(&mut self);
    fn is_view_inspector_open(&self) -> bool;
    fn toggle_view_inspector(&mut self);
    fn is_view_console_open(&self) -> bool;
    fn toggle_view_console(&mut self);
    fn is_grid_visible(&self) -> bool;
    fn toggle_grid(&mut self);

    // Viewport layout
    fn viewport_layout(&self) -> crate::viewport::ViewportLayout;
    fn set_viewport_layout(&mut self, layout: crate::viewport::ViewportLayout);

    // Window
    fn is_docking_enabled(&self) -> bool;
    fn toggle_docking(&mut self);
    fn on_apply_layout_preset(&mut self, preset_name: &str); // "Default", "Wide", "Compact", "Modeling", "Animation", "Debug"
    fn is_dock_panel_visible(&self, panel: PanelType) -> bool;
    fn toggle_dock_panel(&mut self, panel: PanelType);

    // Settings
    fn on_open_settings(&mut self);

    // Debug
    fn on_scan_for_models(&mut self);
    fn on_load_test_model(&mut self, name: &str, path: PathBuf);
    fn on_toggle_engine_rendering(&mut self);
    fn on_show_engine_info(&mut self);

    fn on_debug_material(&mut self, name: &str); // "Red", "Green", "Blue", "White"
    fn on_debug_time_set(&mut self, time: f32);
    fn get_time_of_day(&self) -> f32; // return -1.0 or similar if unavailable? Or Option<f32>
    fn get_time_period(&self) -> String;

    fn is_shadows_enabled(&self) -> bool;
    fn set_shadows_enabled(&mut self, enabled: bool);

    fn on_diff_assets(&mut self);
    fn on_clear_console(&mut self);

    // Help
    fn on_show_tutorial(&mut self);
    fn on_show_about(&mut self);

    // Blend & Blueprint quick-access
    fn on_import_blend_scene(&mut self);
    fn on_toggle_blueprint_mode(&mut self);
    fn is_blueprint_mode(&self) -> bool;
}

/// Main Menu Bar Component
pub struct MenuBar;

impl MenuBar {
    pub fn show(ui: &mut Ui, handler: &mut dyn MenuActionHandler) {
        // File menu — consolidates all file operations
        ui.menu_button("File", |ui| {
            if ui.button("New                  Ctrl+N").clicked() {
                handler.on_new();
                ui.close();
            }
            if ui.button("New World (Wizard)...").clicked() {
                handler.on_new_world_wizard();
                ui.close();
            }
            if ui.button("Open                Ctrl+O").clicked() {
                handler.on_open();
                ui.close();
            }
            ui.separator();
            if ui.button("Save                 Ctrl+S").clicked() {
                handler.on_save();
                ui.close();
            }
            if ui.button("Save Scene").clicked() {
                handler.on_save_scene();
                ui.close();
            }
            if ui.button("Export JSON").clicked() {
                handler.on_save_json();
                ui.close();
            }
            if ui.button("Load Scene").clicked() {
                handler.on_load_scene();
                ui.close();
            }
            ui.separator();
            if ui
                .button("\u{1f4e6} Import .blend Scene...     Ctrl+I")
                .clicked()
            {
                handler.on_import_blend_scene();
                ui.close();
            }
            ui.separator();
            ui.menu_button("Recent Files", |ui| {
                let files = handler.get_recent_files();
                if files.is_empty() {
                    ui.label("No recent files");
                } else {
                    for path in files {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        if ui.button(name).clicked() {
                            handler.on_open_recent(path);
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Recent Files").clicked() {
                        handler.on_clear_recent();
                        ui.close();
                    }
                }
            });
            ui.separator();
            if ui.button("Settings").clicked() {
                handler.on_open_settings();
                ui.close();
            }
            ui.separator();
            if ui.button("Exit").clicked() {
                handler.on_exit();
                ui.close();
            }
        });

        ui.menu_button("Edit", |ui| {
            let count = handler.selection_count();

            if ui.button("Undo                  Ctrl+Z").clicked() {
                handler.on_undo();
                ui.close();
            }
            if ui.button("Redo                  Ctrl+Y").clicked() {
                handler.on_redo();
                ui.close();
            }

            if ui
                .add_enabled(
                    count > 0,
                    egui::Button::new("Delete                    Del"),
                )
                .clicked()
            {
                handler.on_delete();
                ui.close();
            }

            ui.separator();

            ui.label(format!("{} selected", count));
            ui.separator();

            let has_multi = count > 1;

            if ui
                .add_enabled(has_multi, egui::Button::new("Apply Material to All"))
                .clicked()
            {
                handler.on_apply_material();
                ui.close();
            }

            ui.separator();

            if ui
                .add_enabled(has_multi, egui::Button::new("Group              Ctrl+G"))
                .clicked()
            {
                handler.on_group_selection();
                ui.close();
            }

            if ui.button("Ungroup     Ctrl+Shift+G").clicked() {
                handler.on_ungroup_selection();
                ui.close();
            }

            ui.separator();
            ui.label("Align Selection:");
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(has_multi, egui::Button::new("Left"))
                    .clicked()
                {
                    handler.on_align_selection(AlignDirection::Left);
                }
                if ui
                    .add_enabled(has_multi, egui::Button::new("Right"))
                    .clicked()
                {
                    handler.on_align_selection(AlignDirection::Right);
                }
            });

            ui.separator();

            let can_distribute = count >= 3;
            ui.label("Distribute:");
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_distribute, egui::Button::new("X"))
                    .on_hover_text("Distribute evenly along X")
                    .clicked()
                {
                    handler.on_distribute_selection(DistributeDirection::X);
                }
                if ui
                    .add_enabled(can_distribute, egui::Button::new("Z"))
                    .on_hover_text("Distribute evenly along Z")
                    .clicked()
                {
                    handler.on_distribute_selection(DistributeDirection::Z);
                }
            });

            ui.separator();

            if ui.button("Select All          Ctrl+A").clicked() {
                handler.on_select_all();
                ui.close();
            }
            if ui.button("Deselect All              Esc").clicked() {
                handler.on_deselect_all();
                ui.close();
            }
        });

        // View
        ui.menu_button("View", |ui| {
            let mut h = handler.is_view_hierarchy_open();
            if ui.checkbox(&mut h, "Hierarchy Panel").changed() {
                handler.toggle_view_hierarchy();
            }
            let mut i = handler.is_view_inspector_open();
            if ui.checkbox(&mut i, "Inspector Panel").changed() {
                handler.toggle_view_inspector();
            }
            let mut c = handler.is_view_console_open();
            if ui.checkbox(&mut c, "Console Panel").changed() {
                handler.toggle_view_console();
            }
            ui.separator();
            let mut g = handler.is_grid_visible();
            if ui.checkbox(&mut g, "Grid").changed() {
                handler.toggle_grid();
            }

            ui.separator();
            ui.label("Viewport Layout:");
            let current_layout = handler.viewport_layout();
            for layout in &[
                crate::viewport::ViewportLayout::Single,
                crate::viewport::ViewportLayout::SideBySide,
                crate::viewport::ViewportLayout::TopBottom,
                crate::viewport::ViewportLayout::Quad,
            ] {
                if ui
                    .selectable_label(current_layout == *layout, layout.label())
                    .clicked()
                {
                    handler.set_viewport_layout(*layout);
                    ui.close();
                }
            }
        });

        // Window
        ui.menu_button("Window", |ui| {
            let mut docking = handler.is_docking_enabled();
            if ui.checkbox(&mut docking, "Use Docking Layout").changed() {
                handler.toggle_docking();
            }

            ui.separator();
            ui.label("Layout Presets:");

            if ui.button("Default").clicked() {
                handler.on_apply_layout_preset("Default");
                ui.close();
            }
            if ui.button("Wide").clicked() {
                handler.on_apply_layout_preset("Wide");
                ui.close();
            }
            if ui.button("Compact").clicked() {
                handler.on_apply_layout_preset("Compact");
                ui.close();
            }
            if ui.button("Modeling").clicked() {
                handler.on_apply_layout_preset("Modeling");
                ui.close();
            }
            if ui.button("Animation").clicked() {
                handler.on_apply_layout_preset("Animation");
                ui.close();
            }
            if ui.button("Debug").clicked() {
                handler.on_apply_layout_preset("Debug");
                ui.close();
            }

            ui.separator();
            ui.label("Panels:");

            for &panel_type in PanelType::all() {
                let is_visible = handler.is_dock_panel_visible(panel_type);
                if ui
                    .selectable_label(is_visible, panel_type.title())
                    .clicked()
                {
                    handler.toggle_dock_panel(panel_type);
                }
            }
        });

        // Debug
        ui.menu_button("Debug", |ui| {
            ui.label("Viewport Tests:");

            {
                if ui.button("Load barrels.glb").clicked() {
                    handler.on_load_test_model(
                        "test_barrels",
                        PathBuf::from("assets/models/barrels.glb"),
                    );
                    ui.close();
                }
                if ui.button("Load bed.glb").clicked() {
                    handler.on_load_test_model("test_bed", PathBuf::from("assets/models/bed.glb"));
                    ui.close();
                }
                if ui.button("Load Pine Tree").clicked() {
                    handler.on_load_test_model("pine_tree", PathBuf::from("PINE_TREE_AUTO"));
                    ui.close();
                }

                if ui.button("Toggle Engine Rendering").clicked() {
                    handler.on_toggle_engine_rendering();
                    ui.close();
                }
            }

            ui.separator();
            ui.label("Diagnostics:");
            if ui.button("Show Engine Info").clicked() {
                handler.on_show_engine_info();
                ui.close();
            }

            ui.separator();
            ui.label("Material Testing:");
            if ui.button("Red Material").clicked() {
                handler.on_debug_material("Red");
                ui.close();
            }
            if ui.button("Green Metallic").clicked() {
                handler.on_debug_material("Green");
                ui.close();
            }
            if ui.button("Blue Rough").clicked() {
                handler.on_debug_material("Blue");
                ui.close();
            }
            if ui.button("White Default").clicked() {
                handler.on_debug_material("White");
                ui.close();
            }

            ui.separator();
            ui.label("Lighting / Time of Day:");
            ui.horizontal(|ui| {
                if ui.button("Dawn (6:00)").clicked() {
                    handler.on_debug_time_set(6.0);
                }
                if ui.button("Noon (12:00)").clicked() {
                    handler.on_debug_time_set(12.0);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Sunset (18:00)").clicked() {
                    handler.on_debug_time_set(18.0);
                }
                if ui.button("Midnight (0:00)").clicked() {
                    handler.on_debug_time_set(0.0);
                }
            });

            let time = handler.get_time_of_day();
            let hours = time.floor() as u32;
            let minutes = ((time - time.floor()) * 60.0) as u32;
            ui.label(format!(
                "Current: {:02}:{:02} ({})",
                hours,
                minutes,
                handler.get_time_period()
            ));

            ui.horizontal(|ui| {
                let shadows = handler.is_shadows_enabled();
                let label = if shadows {
                    "Shadows: ON"
                } else {
                    "Shadows: OFF"
                };
                if ui.button(label).clicked() {
                    handler.set_shadows_enabled(!shadows);
                }
            });

            ui.separator();
            ui.label("Model Discovery:");

            if ui.button("Scan For Models").clicked() {
                handler.on_scan_for_models();
                ui.close();
            }

            if ui.button("Diff Assets").clicked() {
                handler.on_diff_assets();
                ui.close();
            }

            if ui.button("Clear Console").clicked() {
                handler.on_clear_console();
                ui.close();
            }
        });

        // Help menu
        ui.menu_button("Help", |ui| {
            if ui.button("Tutorial").clicked() {
                handler.on_show_tutorial();
                ui.close();
            }
            ui.separator();
            if ui.button("About AstraWeave").clicked() {
                handler.on_show_about();
                ui.close();
            }
        });

        // ── Quick-access toolbar buttons (always visible in menu bar) ──
        ui.separator();
        // Import .blend button
        if ui
            .button(egui::RichText::new("\u{1f4e6} Import .blend").small())
            .on_hover_text("Import a Blender .blend scene file (Ctrl+I)")
            .clicked()
        {
            handler.on_import_blend_scene();
        }
        // Blueprint Mode toggle
        let bp_active = handler.is_blueprint_mode();
        let bp_label = if bp_active {
            egui::RichText::new("\u{1f5fa} Blueprint ON")
                .small()
                .color(egui::Color32::from_rgb(100, 200, 255))
        } else {
            egui::RichText::new("\u{1f5fa} Blueprint").small()
        };
        if ui
            .button(bp_label)
            .on_hover_text("Toggle Blueprint Zone Editor (Ctrl+B)")
            .clicked()
        {
            handler.on_toggle_blueprint_mode();
        }
    }
}

impl AlignDirection {
    /// Returns all align direction variants.
    pub fn all() -> &'static [Self] {
        &[
            Self::Left,
            Self::Right,
            Self::Top,
            Self::Bottom,
            Self::CenterX,
            Self::CenterZ,
        ]
    }

    /// Returns the display name for this alignment.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Top => "Top",
            Self::Bottom => "Bottom",
            Self::CenterX => "Center X",
            Self::CenterZ => "Center Z",
        }
    }

    /// Returns the icon for this alignment.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Left => "[Lt]",
            Self::Right => "[Rt]",
            Self::Top => "[Up]",
            Self::Bottom => "[Dn]",
            Self::CenterX => "↔",
            Self::CenterZ => "↕",
        }
    }

    /// Returns true if this is a horizontal alignment.
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Left | Self::Right | Self::CenterX)
    }

    /// Returns true if this is a vertical alignment.
    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Top | Self::Bottom | Self::CenterZ)
    }

    /// Returns true if this is a center alignment.
    pub fn is_center(&self) -> bool {
        matches!(self, Self::CenterX | Self::CenterZ)
    }

    /// Returns the axis this alignment operates on ("X" or "Z").
    pub fn axis(&self) -> &'static str {
        match self {
            Self::Left | Self::Right | Self::CenterX => "X",
            Self::Top | Self::Bottom | Self::CenterZ => "Z",
        }
    }
}

impl std::fmt::Display for AlignDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl DistributeDirection {
    /// Returns all distribute direction variants.
    pub fn all() -> &'static [Self] {
        &[Self::X, Self::Z]
    }

    /// Returns the display name for this distribution.
    pub fn name(&self) -> &'static str {
        match self {
            Self::X => "Distribute X",
            Self::Z => "Distribute Z",
        }
    }

    /// Returns the icon for this distribution.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::X => "↔",
            Self::Z => "↕",
        }
    }

    /// Returns the axis name.
    pub fn axis(&self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Z => "Z",
        }
    }

    /// Returns the keyboard shortcut hint.
    pub fn shortcut_hint(&self) -> &'static str {
        match self {
            Self::X => "Ctrl+Shift+X",
            Self::Z => "Ctrl+Shift+Z",
        }
    }
}

impl std::fmt::Display for DistributeDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Layout preset names available in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LayoutPreset {
    Default,
    Wide,
    Compact,
    Modeling,
    Animation,
    Debug,
}

impl LayoutPreset {
    /// Returns all layout preset variants.
    pub fn all() -> &'static [Self] {
        &[
            Self::Default,
            Self::Wide,
            Self::Compact,
            Self::Modeling,
            Self::Animation,
            Self::Debug,
        ]
    }

    /// Returns the display name for this preset.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Wide => "Wide",
            Self::Compact => "Compact",
            Self::Modeling => "Modeling",
            Self::Animation => "Animation",
            Self::Debug => "Debug",
        }
    }

    /// Returns the icon for this preset.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Default => "\u{1f3e0}",
            Self::Wide => "\u{1f5a5}\u{fe0f}",
            Self::Compact => "\u{1f4d0}",
            Self::Modeling => "\u{1f527}",
            Self::Animation => "\u{1f3ac}",
            Self::Debug => "\u{1f50d}",
        }
    }

    /// Returns a description of this preset.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Default => "Standard editor layout with balanced panels",
            Self::Wide => "Maximized viewport with side panels",
            Self::Compact => "Minimal panel layout for smaller screens",
            Self::Modeling => "Focused on 3D modeling with material and mesh tools",
            Self::Animation => "Optimized for animation workflow with timeline",
            Self::Debug => "Debugging layout with console and performance panels",
        }
    }

    /// Parse a preset from a string name.
    pub fn parse_name(name: &str) -> Option<Self> {
        match name {
            "Default" => Some(Self::Default),
            "Wide" => Some(Self::Wide),
            "Compact" => Some(Self::Compact),
            "Modeling" => Some(Self::Modeling),
            "Animation" => Some(Self::Animation),
            "Debug" => Some(Self::Debug),
            _ => None,
        }
    }
}

impl std::fmt::Display for LayoutPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // AlignDirection Tests
    // ========================================================================

    #[test]
    fn test_align_direction_all() {
        let all = AlignDirection::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&AlignDirection::Left));
        assert!(all.contains(&AlignDirection::Right));
        assert!(all.contains(&AlignDirection::Top));
        assert!(all.contains(&AlignDirection::Bottom));
        assert!(all.contains(&AlignDirection::CenterX));
        assert!(all.contains(&AlignDirection::CenterZ));
    }

    #[test]
    fn test_align_direction_name() {
        assert_eq!(AlignDirection::Left.name(), "Left");
        assert_eq!(AlignDirection::Right.name(), "Right");
        assert_eq!(AlignDirection::Top.name(), "Top");
        assert_eq!(AlignDirection::Bottom.name(), "Bottom");
        assert_eq!(AlignDirection::CenterX.name(), "Center X");
        assert_eq!(AlignDirection::CenterZ.name(), "Center Z");
    }

    #[test]
    fn test_align_direction_icon() {
        assert!(!AlignDirection::Left.icon().is_empty());
        assert!(!AlignDirection::Right.icon().is_empty());
        assert!(!AlignDirection::Top.icon().is_empty());
        assert!(!AlignDirection::Bottom.icon().is_empty());
        assert!(!AlignDirection::CenterX.icon().is_empty());
        assert!(!AlignDirection::CenterZ.icon().is_empty());
    }

    #[test]
    fn test_align_direction_is_horizontal() {
        assert!(AlignDirection::Left.is_horizontal());
        assert!(AlignDirection::Right.is_horizontal());
        assert!(AlignDirection::CenterX.is_horizontal());
        assert!(!AlignDirection::Top.is_horizontal());
        assert!(!AlignDirection::Bottom.is_horizontal());
        assert!(!AlignDirection::CenterZ.is_horizontal());
    }

    #[test]
    fn test_align_direction_is_vertical() {
        assert!(!AlignDirection::Left.is_vertical());
        assert!(!AlignDirection::Right.is_vertical());
        assert!(!AlignDirection::CenterX.is_vertical());
        assert!(AlignDirection::Top.is_vertical());
        assert!(AlignDirection::Bottom.is_vertical());
        assert!(AlignDirection::CenterZ.is_vertical());
    }

    #[test]
    fn test_align_direction_is_center() {
        assert!(!AlignDirection::Left.is_center());
        assert!(!AlignDirection::Right.is_center());
        assert!(AlignDirection::CenterX.is_center());
        assert!(!AlignDirection::Top.is_center());
        assert!(!AlignDirection::Bottom.is_center());
        assert!(AlignDirection::CenterZ.is_center());
    }

    #[test]
    fn test_align_direction_axis() {
        assert_eq!(AlignDirection::Left.axis(), "X");
        assert_eq!(AlignDirection::Right.axis(), "X");
        assert_eq!(AlignDirection::CenterX.axis(), "X");
        assert_eq!(AlignDirection::Top.axis(), "Z");
        assert_eq!(AlignDirection::Bottom.axis(), "Z");
        assert_eq!(AlignDirection::CenterZ.axis(), "Z");
    }

    #[test]
    fn test_align_direction_display() {
        assert_eq!(format!("{}", AlignDirection::Left), "Left");
        assert_eq!(format!("{}", AlignDirection::CenterX), "Center X");
    }

    #[test]
    fn test_align_direction_equality() {
        assert_eq!(AlignDirection::Left, AlignDirection::Left);
        assert_ne!(AlignDirection::Left, AlignDirection::Right);
    }

    #[test]
    fn test_align_direction_clone() {
        let dir = AlignDirection::Top;
        let cloned = dir;
        assert_eq!(dir, cloned);
    }

    // ========================================================================
    // DistributeDirection Tests
    // ========================================================================

    #[test]
    fn test_distribute_direction_all() {
        let all = DistributeDirection::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&DistributeDirection::X));
        assert!(all.contains(&DistributeDirection::Z));
    }

    #[test]
    fn test_distribute_direction_name() {
        assert_eq!(DistributeDirection::X.name(), "Distribute X");
        assert_eq!(DistributeDirection::Z.name(), "Distribute Z");
    }

    #[test]
    fn test_distribute_direction_icon() {
        assert!(!DistributeDirection::X.icon().is_empty());
        assert!(!DistributeDirection::Z.icon().is_empty());
    }

    #[test]
    fn test_distribute_direction_axis() {
        assert_eq!(DistributeDirection::X.axis(), "X");
        assert_eq!(DistributeDirection::Z.axis(), "Z");
    }

    #[test]
    fn test_distribute_direction_shortcut_hint() {
        assert!(!DistributeDirection::X.shortcut_hint().is_empty());
        assert!(!DistributeDirection::Z.shortcut_hint().is_empty());
    }

    #[test]
    fn test_distribute_direction_display() {
        assert_eq!(format!("{}", DistributeDirection::X), "Distribute X");
        assert_eq!(format!("{}", DistributeDirection::Z), "Distribute Z");
    }

    #[test]
    fn test_distribute_direction_equality() {
        assert_eq!(DistributeDirection::X, DistributeDirection::X);
        assert_ne!(DistributeDirection::X, DistributeDirection::Z);
    }

    // ========================================================================
    // LayoutPreset Tests
    // ========================================================================

    #[test]
    fn test_layout_preset_all() {
        let all = LayoutPreset::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&LayoutPreset::Default));
        assert!(all.contains(&LayoutPreset::Wide));
        assert!(all.contains(&LayoutPreset::Compact));
        assert!(all.contains(&LayoutPreset::Modeling));
        assert!(all.contains(&LayoutPreset::Animation));
        assert!(all.contains(&LayoutPreset::Debug));
    }

    #[test]
    fn test_layout_preset_name() {
        assert_eq!(LayoutPreset::Default.name(), "Default");
        assert_eq!(LayoutPreset::Wide.name(), "Wide");
        assert_eq!(LayoutPreset::Compact.name(), "Compact");
        assert_eq!(LayoutPreset::Modeling.name(), "Modeling");
        assert_eq!(LayoutPreset::Animation.name(), "Animation");
        assert_eq!(LayoutPreset::Debug.name(), "Debug");
    }

    #[test]
    fn test_layout_preset_icon() {
        for preset in LayoutPreset::all() {
            assert!(!preset.icon().is_empty());
        }
    }

    #[test]
    fn test_layout_preset_description() {
        for preset in LayoutPreset::all() {
            assert!(!preset.description().is_empty());
        }
    }

    #[test]
    fn test_layout_preset_from_str() {
        assert_eq!(
            LayoutPreset::parse_name("Default"),
            Some(LayoutPreset::Default)
        );
        assert_eq!(LayoutPreset::parse_name("Wide"), Some(LayoutPreset::Wide));
        assert_eq!(
            LayoutPreset::parse_name("Compact"),
            Some(LayoutPreset::Compact)
        );
        assert_eq!(
            LayoutPreset::parse_name("Modeling"),
            Some(LayoutPreset::Modeling)
        );
        assert_eq!(
            LayoutPreset::parse_name("Animation"),
            Some(LayoutPreset::Animation)
        );
        assert_eq!(LayoutPreset::parse_name("Debug"), Some(LayoutPreset::Debug));
        assert_eq!(LayoutPreset::parse_name("InvalidPreset"), None);
        assert_eq!(LayoutPreset::parse_name(""), None);
    }

    #[test]
    fn test_layout_preset_display() {
        assert_eq!(format!("{}", LayoutPreset::Default), "Default");
        assert_eq!(format!("{}", LayoutPreset::Animation), "Animation");
    }

    #[test]
    fn test_layout_preset_equality() {
        assert_eq!(LayoutPreset::Default, LayoutPreset::Default);
        assert_ne!(LayoutPreset::Default, LayoutPreset::Wide);
    }

    #[test]
    fn test_layout_preset_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(LayoutPreset::Default);
        set.insert(LayoutPreset::Wide);
        set.insert(LayoutPreset::Default); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_layout_preset_roundtrip() {
        for preset in LayoutPreset::all() {
            let name = preset.name();
            let parsed = LayoutPreset::parse_name(name);
            assert_eq!(parsed, Some(*preset));
        }
    }
}
