use astract::advanced::{ColorPicker, RangeSlider, TreeNode, TreeView};
use egui::{Color32, Ui};
use tracing::debug;

/// Panel demonstrating advanced widgets with game engine use cases
pub struct AdvancedWidgetsPanel {
    // Color picker examples
    ambient_color: ColorPicker,
    directional_light_color: ColorPicker,
    fog_color: ColorPicker,

    // Tree view examples
    scene_hierarchy: TreeView,
    asset_browser: TreeView,

    // Range slider examples
    camera_distance: RangeSlider,
    player_level_range: RangeSlider,
    audio_frequency: RangeSlider,

    // Demo state
    initialized: bool,
}

impl Default for AdvancedWidgetsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedWidgetsPanel {
    pub fn new() -> Self {
        Self {
            // Color pickers with game engine defaults
            ambient_color: ColorPicker::new()
                .with_color(Color32::from_rgb(50, 50, 70))
                .show_alpha(false)
                .width(260.0),

            directional_light_color: ColorPicker::new()
                .with_color(Color32::from_rgb(255, 244, 214))
                .show_alpha(false)
                .width(260.0),

            fog_color: ColorPicker::new()
                .with_color(Color32::from_rgb(180, 180, 200))
                .show_alpha(true)
                .width(260.0),

            // Tree views (will be populated in init)
            scene_hierarchy: TreeView::new().with_indent(16.0),
            asset_browser: TreeView::new().with_indent(16.0),

            // Range sliders with game-relevant ranges
            camera_distance: RangeSlider::new(1.0, 100.0)
                .with_min(5.0)
                .with_max(25.0)
                .step(1.0)
                .suffix(" m")
                .width(280.0),

            player_level_range: RangeSlider::new(1.0, 100.0)
                .with_min(10.0)
                .with_max(50.0)
                .step(1.0)
                .prefix("Lv ")
                .width(280.0),

            audio_frequency: RangeSlider::new(20.0, 20000.0)
                .with_min(200.0)
                .with_max(8000.0)
                .step(100.0)
                .suffix(" Hz")
                .width(280.0),

            initialized: false,
        }
    }

    /// Initialize tree view data (only once)
    fn initialize_tree_views(&mut self) {
        if self.initialized {
            return;
        }

        // Scene Hierarchy: Typical game scene structure
        let _world = self
            .scene_hierarchy
            .add_node(TreeNode::new(1, "World").with_icon("[Glb]"));

        let environment = self
            .scene_hierarchy
            .add_node(TreeNode::new(2, "Environment").with_icon("🌄"));
        self.scene_hierarchy
            .add_child(environment, TreeNode::new(10, "Skybox").with_icon("🌌"));
        self.scene_hierarchy
            .add_child(environment, TreeNode::new(11, "Sun").with_icon("[Sun]"));
        self.scene_hierarchy
            .add_child(environment, TreeNode::new(12, "Fog").with_icon("[Fog]"));

        let entities = self
            .scene_hierarchy
            .add_node(TreeNode::new(3, "Entities").with_icon("[Gp]"));

        if let Some(player) = self
            .scene_hierarchy
            .add_child(entities, TreeNode::new(20, "Player").with_icon("👤"))
        {
            self.scene_hierarchy
                .add_child(player, TreeNode::new(21, "Camera").with_icon("📷"));
            self.scene_hierarchy
                .add_child(player, TreeNode::new(22, "Weapon").with_icon("[Wpn]"));
        }

        if let Some(enemies) = self
            .scene_hierarchy
            .add_child(entities, TreeNode::new(23, "Enemies").with_icon("👾"))
        {
            self.scene_hierarchy
                .add_child(enemies, TreeNode::new(24, "Enemy_1").with_icon("[Bot]"));
            self.scene_hierarchy
                .add_child(enemies, TreeNode::new(25, "Enemy_2").with_icon("[Bot]"));
            self.scene_hierarchy
                .add_child(enemies, TreeNode::new(26, "Enemy_3").with_icon("[Bot]"));
        }

        if let Some(npcs) = self
            .scene_hierarchy
            .add_child(entities, TreeNode::new(27, "NPCs").with_icon("🧑"))
        {
            self.scene_hierarchy
                .add_child(npcs, TreeNode::new(28, "Merchant").with_icon("🏪"));
            self.scene_hierarchy
                .add_child(npcs, TreeNode::new(29, "Guard").with_icon("[Shld]"));
        }

        // Asset Browser: Game asset organization
        let assets = self
            .asset_browser
            .add_node(TreeNode::new(100, "Assets").with_icon("[Pkg]"));

        if let Some(models) = self
            .asset_browser
            .add_child(assets, TreeNode::new(101, "Models").with_icon("🗿"))
        {
            self.asset_browser.add_child(
                models,
                TreeNode::new(110, "character.fbx").with_icon("[Doc]"),
            );
            self.asset_browser
                .add_child(models, TreeNode::new(111, "weapon.fbx").with_icon("[Doc]"));
            self.asset_browser.add_child(
                models,
                TreeNode::new(112, "environment.fbx").with_icon("[Doc]"),
            );
        }

        if let Some(textures) = self
            .asset_browser
            .add_child(assets, TreeNode::new(102, "Textures").with_icon("[Img]"))
        {
            self.asset_browser.add_child(
                textures,
                TreeNode::new(120, "albedo.png").with_icon("[Doc]"),
            );
            self.asset_browser.add_child(
                textures,
                TreeNode::new(121, "normal.png").with_icon("[Doc]"),
            );
            self.asset_browser.add_child(
                textures,
                TreeNode::new(122, "metallic.png").with_icon("[Doc]"),
            );
        }

        if let Some(audio) = self
            .asset_browser
            .add_child(assets, TreeNode::new(103, "Audio").with_icon("[Snd]"))
        {
            self.asset_browser
                .add_child(audio, TreeNode::new(130, "music.ogg").with_icon("[Mus]"));
            self.asset_browser
                .add_child(audio, TreeNode::new(131, "sfx_shot.wav").with_icon("[Snd]"));
            self.asset_browser
                .add_child(audio, TreeNode::new(132, "sfx_step.wav").with_icon("[Snd]"));
        }

        if let Some(scripts) = self
            .asset_browser
            .add_child(assets, TreeNode::new(104, "Scripts").with_icon("📜"))
        {
            self.asset_browser.add_child(
                scripts,
                TreeNode::new(140, "player_controller.rs").with_icon("🦀"),
            );
            self.asset_browser
                .add_child(scripts, TreeNode::new(141, "enemy_ai.rs").with_icon("🦀"));
        }

        self.initialized = true;
    }

    /// Update panel state
    pub fn update(&mut self) {
        if !self.initialized {
            debug!("advanced_widgets: initializing demo panel");
        }
        self.initialize_tree_views();
    }

    /// Show the advanced widgets panel
    pub fn show(&mut self, ui: &mut Ui) {
        ui.heading("Advanced Widgets");
        ui.add_space(8.0);

        // Color Pickers Section
        ui.collapsing("Color Pickers", |ui| {
            ui.label("Game engine lighting and atmosphere controls");
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.label("[Moon] Ambient Light Color");
                ui.add_space(4.0);
                if self.ambient_color.show(ui) {
                    let color = self.ambient_color.color();
                    ui.label(format!(
                        "RGB: ({}, {}, {})",
                        color.r(),
                        color.g(),
                        color.b()
                    ));
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("[Sun] Directional Light Color");
                ui.add_space(4.0);
                if self.directional_light_color.show(ui) {
                    let color = self.directional_light_color.color();
                    ui.label(format!(
                        "RGB: ({}, {}, {})",
                        color.r(),
                        color.g(),
                        color.b()
                    ));
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("[Fog] Fog Color (with Alpha)");
                ui.add_space(4.0);
                if self.fog_color.show(ui) {
                    let color = self.fog_color.color();
                    ui.label(format!(
                        "RGBA: ({}, {}, {}, {})",
                        color.r(),
                        color.g(),
                        color.b(),
                        color.a()
                    ));
                }
            });
        });

        ui.add_space(8.0);

        // Tree Views Section
        ui.collapsing("Tree Views", |ui| {
            ui.label("Hierarchical data visualization");
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.label("[Glb] Scene Hierarchy");
                ui.separator();
                ui.add_space(4.0);

                if let Some(clicked_id) = self.scene_hierarchy.show(ui) {
                    if let Some(node) = self.scene_hierarchy.get_node(clicked_id) {
                        ui.separator();
                        ui.colored_label(
                            Color32::from_rgb(100, 180, 255),
                            format!("Selected: {} (ID: {})", node.label, node.id),
                        );
                    }
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("Asset Browser");
                ui.separator();
                ui.add_space(4.0);

                if let Some(clicked_id) = self.asset_browser.show(ui) {
                    if let Some(node) = self.asset_browser.get_node(clicked_id) {
                        ui.separator();
                        ui.colored_label(
                            Color32::from_rgb(100, 180, 255),
                            format!("Selected: {} (ID: {})", node.label, node.id),
                        );
                    }
                }
            });
        });

        ui.add_space(8.0);

        // Range Sliders Section
        ui.collapsing("📏 Range Sliders", |ui| {
            ui.label("Dual-handle range selection controls");
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.label("📷 Camera Distance Range");
                ui.add_space(4.0);
                if self.camera_distance.show(ui) {
                    ui.label(format!(
                        "LOD switching: {} - {}",
                        self.camera_distance
                            .format_value(self.camera_distance.min_value()),
                        self.camera_distance
                            .format_value(self.camera_distance.max_value())
                    ));
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("Player Level Range Filter");
                ui.add_space(4.0);
                if self.player_level_range.show(ui) {
                    ui.label(format!(
                        "Matchmaking range: {} - {}",
                        self.player_level_range
                            .format_value(self.player_level_range.min_value()),
                        self.player_level_range
                            .format_value(self.player_level_range.max_value())
                    ));
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("Audio Frequency Filter");
                ui.add_space(4.0);
                if self.audio_frequency.show(ui) {
                    ui.label(format!(
                        "EQ range: {} - {}",
                        self.audio_frequency
                            .format_value(self.audio_frequency.min_value()),
                        self.audio_frequency
                            .format_value(self.audio_frequency.max_value())
                    ));
                }
            });
        });

        ui.add_space(8.0);

        // Stats
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(format!(
                "Scene Nodes: {}",
                self.scene_hierarchy.node_count()
            ));
            ui.separator();
            ui.label(format!("Assets: {}", self.asset_browser.node_count()));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Panel Creation Tests ===

    #[test]
    fn test_panel_creation() {
        let panel = AdvancedWidgetsPanel::new();
        assert!(!panel.initialized);
    }

    #[test]
    fn test_panel_default() {
        let panel = AdvancedWidgetsPanel::default();
        assert!(!panel.initialized);
    }

    #[test]
    fn test_panel_initialization() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        assert!(panel.initialized);
        assert!(panel.scene_hierarchy.node_count() > 0);
        assert!(panel.asset_browser.node_count() > 0);
    }

    #[test]
    fn test_panel_double_initialization() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();
        let count1 = panel.scene_hierarchy.node_count();

        panel.initialize_tree_views(); // Should not re-initialize
        let count2 = panel.scene_hierarchy.node_count();

        assert_eq!(count1, count2);
    }

    #[test]
    fn test_panel_update_initializes() {
        let mut panel = AdvancedWidgetsPanel::new();
        assert!(!panel.initialized);

        panel.update();

        assert!(panel.initialized);
    }

    // === Color Picker Tests ===

    #[test]
    fn test_ambient_color_default() {
        let panel = AdvancedWidgetsPanel::new();
        let ambient = panel.ambient_color.color();
        assert_eq!(ambient.r(), 50);
        assert_eq!(ambient.g(), 50);
        assert_eq!(ambient.b(), 70);
    }

    #[test]
    fn test_directional_light_color_default() {
        let panel = AdvancedWidgetsPanel::new();
        let light = panel.directional_light_color.color();
        assert_eq!(light.r(), 255);
        assert_eq!(light.g(), 244);
        assert_eq!(light.b(), 214);
    }

    #[test]
    fn test_fog_color_default() {
        let panel = AdvancedWidgetsPanel::new();
        let fog = panel.fog_color.color();
        assert_eq!(fog.r(), 180);
        assert_eq!(fog.g(), 180);
        assert_eq!(fog.b(), 200);
    }

    #[test]
    fn test_ambient_color_is_dark() {
        let panel = AdvancedWidgetsPanel::new();
        let c = panel.ambient_color.color();
        // Ambient should be darker than directional light
        let brightness = (c.r() as u32 + c.g() as u32 + c.b() as u32) / 3;
        assert!(brightness < 100);
    }

    #[test]
    fn test_directional_light_is_bright() {
        let panel = AdvancedWidgetsPanel::new();
        let c = panel.directional_light_color.color();
        // Sun light should be bright
        let brightness = (c.r() as u32 + c.g() as u32 + c.b() as u32) / 3;
        assert!(brightness > 200);
    }

    // === Range Slider Tests ===

    #[test]
    fn test_camera_distance_defaults() {
        let panel = AdvancedWidgetsPanel::new();
        assert_eq!(panel.camera_distance.min_value(), 5.0);
        assert_eq!(panel.camera_distance.max_value(), 25.0);
    }

    #[test]
    fn test_camera_distance_bounds() {
        let panel = AdvancedWidgetsPanel::new();
        // Range is 1.0 to 100.0, current is 5.0 to 25.0
        assert!(panel.camera_distance.min_value() >= 1.0);
        assert!(panel.camera_distance.max_value() <= 100.0);
    }

    #[test]
    fn test_player_level_range_defaults() {
        let panel = AdvancedWidgetsPanel::new();
        assert_eq!(panel.player_level_range.min_value(), 10.0);
        assert_eq!(panel.player_level_range.max_value(), 50.0);
    }

    #[test]
    fn test_player_level_range_bounds() {
        let panel = AdvancedWidgetsPanel::new();
        // Range is 1.0 to 100.0
        assert!(panel.player_level_range.min_value() >= 1.0);
        assert!(panel.player_level_range.max_value() <= 100.0);
    }

    #[test]
    fn test_audio_frequency_defaults() {
        let panel = AdvancedWidgetsPanel::new();
        assert_eq!(panel.audio_frequency.min_value(), 200.0);
        assert_eq!(panel.audio_frequency.max_value(), 8000.0);
    }

    #[test]
    fn test_audio_frequency_bounds() {
        let panel = AdvancedWidgetsPanel::new();
        // Range is 20.0 to 20000.0 (human hearing range)
        assert!(panel.audio_frequency.min_value() >= 20.0);
        assert!(panel.audio_frequency.max_value() <= 20000.0);
    }

    #[test]
    fn test_range_slider_min_less_than_max() {
        let panel = AdvancedWidgetsPanel::new();
        assert!(panel.camera_distance.min_value() < panel.camera_distance.max_value());
        assert!(panel.player_level_range.min_value() < panel.player_level_range.max_value());
        assert!(panel.audio_frequency.min_value() < panel.audio_frequency.max_value());
    }

    // === Scene Hierarchy Tests ===

    #[test]
    fn test_scene_hierarchy_structure() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        assert!(panel.scene_hierarchy.node_count() >= 3);
    }

    #[test]
    fn test_scene_hierarchy_world_node() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let world = panel.scene_hierarchy.get_node(1);
        assert!(world.is_some());
        assert_eq!(world.unwrap().label, "World");
    }

    #[test]
    fn test_scene_hierarchy_environment_node() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let env = panel.scene_hierarchy.get_node(2);
        assert!(env.is_some());
        assert_eq!(env.unwrap().label, "Environment");
    }

    #[test]
    fn test_scene_hierarchy_entities_node() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let entities = panel.scene_hierarchy.get_node(3);
        assert!(entities.is_some());
        assert_eq!(entities.unwrap().label, "Entities");
    }

    #[test]
    fn test_scene_hierarchy_player_node() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let player = panel.scene_hierarchy.get_node(20);
        assert!(player.is_some());
        assert_eq!(player.unwrap().label, "Player");
    }

    #[test]
    fn test_scene_hierarchy_camera_node() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let camera = panel.scene_hierarchy.get_node(21);
        assert!(camera.is_some());
        assert_eq!(camera.unwrap().label, "Camera");
    }

    #[test]
    fn test_scene_hierarchy_enemies_exist() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        // Check enemies parent
        let enemies = panel.scene_hierarchy.get_node(23);
        assert!(enemies.is_some());

        // Check individual enemies
        assert!(panel.scene_hierarchy.get_node(24).is_some()); // Enemy_1
        assert!(panel.scene_hierarchy.get_node(25).is_some()); // Enemy_2
        assert!(panel.scene_hierarchy.get_node(26).is_some()); // Enemy_3
    }

    #[test]
    fn test_scene_hierarchy_npcs_exist() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let npcs = panel.scene_hierarchy.get_node(27);
        assert!(npcs.is_some());

        assert!(panel.scene_hierarchy.get_node(28).is_some()); // Merchant
        assert!(panel.scene_hierarchy.get_node(29).is_some()); // Guard
    }

    // === Asset Browser Tests ===

    #[test]
    fn test_asset_browser_structure() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        assert!(panel.asset_browser.node_count() > 0);
    }

    #[test]
    fn test_asset_browser_root() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let assets = panel.asset_browser.get_node(100);
        assert!(assets.is_some());
        assert_eq!(assets.unwrap().label, "Assets");
    }

    #[test]
    fn test_asset_browser_models_category() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let models = panel.asset_browser.get_node(101);
        assert!(models.is_some());
        assert_eq!(models.unwrap().label, "Models");

        // Check model files
        assert!(panel.asset_browser.get_node(110).is_some()); // character.fbx
        assert!(panel.asset_browser.get_node(111).is_some()); // weapon.fbx
    }

    #[test]
    fn test_asset_browser_textures_category() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let textures = panel.asset_browser.get_node(102);
        assert!(textures.is_some());
        assert_eq!(textures.unwrap().label, "Textures");

        // Check texture files
        assert!(panel.asset_browser.get_node(120).is_some()); // albedo.png
        assert!(panel.asset_browser.get_node(121).is_some()); // normal.png
    }

    #[test]
    fn test_asset_browser_audio_category() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let audio = panel.asset_browser.get_node(103);
        assert!(audio.is_some());
        assert_eq!(audio.unwrap().label, "Audio");

        // Check audio files
        assert!(panel.asset_browser.get_node(130).is_some()); // music.ogg
        assert!(panel.asset_browser.get_node(131).is_some()); // sfx_shot.wav
    }

    #[test]
    fn test_asset_browser_scripts_category() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.initialize_tree_views();

        let scripts = panel.asset_browser.get_node(104);
        assert!(scripts.is_some());
        assert_eq!(scripts.unwrap().label, "Scripts");

        // Check script files
        assert!(panel.asset_browser.get_node(140).is_some()); // player_controller.rs
        assert!(panel.asset_browser.get_node(141).is_some()); // enemy_ai.rs
    }

    // === Integration Tests ===

    #[test]
    fn test_full_initialization() {
        let mut panel = AdvancedWidgetsPanel::new();

        // Before init
        assert!(!panel.initialized);
        assert_eq!(panel.scene_hierarchy.node_count(), 0);
        assert_eq!(panel.asset_browser.node_count(), 0);

        // After init
        panel.update();

        assert!(panel.initialized);
        assert!(panel.scene_hierarchy.node_count() > 10);
        assert!(panel.asset_browser.node_count() > 10);
    }

    #[test]
    fn test_node_count_summary() {
        let mut panel = AdvancedWidgetsPanel::new();
        panel.update();

        // Scene should have more nodes than just root
        let scene_count = panel.scene_hierarchy.node_count();
        let asset_count = panel.asset_browser.node_count();

        assert!(scene_count >= 15, "Scene should have at least 15 nodes");
        assert!(
            asset_count >= 10,
            "Asset browser should have at least 10 nodes"
        );
    }
}
