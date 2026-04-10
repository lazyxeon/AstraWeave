//! LOD Configuration Panel for the editor
//!
//! Provides level-of-detail configuration:
//! - LOD group management
//! - Distance-based LOD switching
//! - Screen coverage settings
//! - Automatic LOD generation
//! - Per-asset LOD overrides
//! - Performance impact visualization

use egui::{Color32, RichText, Ui, Vec2};
use tracing::info;

use crate::panels::Panel;

/// LOD bias mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LodBiasMode {
    #[default]
    Auto,
    Quality,
    Balanced,
    Performance,
    Custom,
}

impl std::fmt::Display for LodBiasMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LodBiasMode {
    pub fn all() -> &'static [LodBiasMode] {
        &[
            LodBiasMode::Auto,
            LodBiasMode::Quality,
            LodBiasMode::Balanced,
            LodBiasMode::Performance,
            LodBiasMode::Custom,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LodBiasMode::Auto => "Auto",
            LodBiasMode::Quality => "Quality",
            LodBiasMode::Balanced => "Balanced",
            LodBiasMode::Performance => "Performance",
            LodBiasMode::Custom => "Custom",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LodBiasMode::Auto => "[Bot]",
            LodBiasMode::Quality => "[Gem]",
            LodBiasMode::Balanced => "⚖️",
            LodBiasMode::Performance => "[Zap]",
            LodBiasMode::Custom => "[Wrn]",
        }
    }
}

/// LOD level definition
#[derive(Debug, Clone)]
pub struct LodLevel {
    pub level: u32,
    pub mesh_path: String,
    pub distance: f32,
    pub screen_coverage: f32,
    pub triangle_count: u32,
    pub vertex_count: u32,
    pub reduction_percent: f32,
}

impl Default for LodLevel {
    fn default() -> Self {
        Self {
            level: 0,
            mesh_path: String::new(),
            distance: 0.0,
            screen_coverage: 1.0,
            triangle_count: 0,
            vertex_count: 0,
            reduction_percent: 0.0,
        }
    }
}

/// LOD group (collection of LOD levels for one asset)
#[derive(Debug, Clone)]
pub struct LodGroup {
    pub id: u32,
    pub name: String,
    pub asset_path: String,
    pub enabled: bool,
    pub levels: Vec<LodLevel>,

    // Transition settings
    pub fade_mode: FadeMode,
    pub fade_width: f32,
    pub cross_fade: bool,

    // Culling
    pub cull_distance: f32,
    pub shadow_lod_offset: i32,

    // Statistics
    pub base_triangles: u32,
    pub current_level: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FadeMode {
    #[default]
    None,
    CrossFade,
    SpeedTree,
    Dither,
}

impl std::fmt::Display for FadeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl FadeMode {
    pub fn all() -> &'static [FadeMode] {
        &[
            FadeMode::None,
            FadeMode::CrossFade,
            FadeMode::SpeedTree,
            FadeMode::Dither,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            FadeMode::None => "None",
            FadeMode::CrossFade => "Cross Fade",
            FadeMode::SpeedTree => "SpeedTree",
            FadeMode::Dither => "Dither",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            FadeMode::None => "⬛",
            FadeMode::CrossFade => "[Shuf]",
            FadeMode::SpeedTree => "[Tree]",
            FadeMode::Dither => "⊞",
        }
    }
}

impl Default for LodGroup {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New LOD Group".to_string(),
            asset_path: String::new(),
            enabled: true,
            levels: Vec::new(),

            fade_mode: FadeMode::CrossFade,
            fade_width: 0.1,
            cross_fade: true,

            cull_distance: 1000.0,
            shadow_lod_offset: 1,

            base_triangles: 0,
            current_level: 0,
        }
    }
}

/// Global LOD settings
#[derive(Debug, Clone)]
pub struct GlobalLodSettings {
    pub bias_mode: LodBiasMode,
    pub custom_bias: f32,
    pub maximum_lod_level: u32,
    pub lod_cross_fade_time: f32,

    // Quality presets
    pub quality_distances: [f32; 4],
    pub balanced_distances: [f32; 4],
    pub performance_distances: [f32; 4],

    // Screen coverage
    pub screen_coverage_enabled: bool,
    pub min_screen_coverage: f32,

    // Shadows
    pub shadow_lod_bias: i32,
    pub shadow_cull_distance: f32,
}

impl Default for GlobalLodSettings {
    fn default() -> Self {
        Self {
            bias_mode: LodBiasMode::Balanced,
            custom_bias: 1.0,
            maximum_lod_level: 4,
            lod_cross_fade_time: 0.5,

            quality_distances: [20.0, 50.0, 100.0, 200.0],
            balanced_distances: [15.0, 35.0, 70.0, 150.0],
            performance_distances: [10.0, 25.0, 50.0, 100.0],

            screen_coverage_enabled: false,
            min_screen_coverage: 0.01,

            shadow_lod_bias: 1,
            shadow_cull_distance: 500.0,
        }
    }
}

/// LOD generation settings
#[derive(Debug, Clone)]
pub struct LodGenerationSettings {
    pub auto_generate: bool,
    pub num_levels: u32,
    pub reduction_method: ReductionMethod,
    pub target_reductions: [f32; 4], // percent per level
    pub preserve_uvs: bool,
    pub preserve_normals: bool,
    pub preserve_borders: bool,
    pub weld_threshold: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReductionMethod {
    #[default]
    QuadricErrorMetric,
    EdgeCollapse,
    VertexClustering,
    Simplygon,
}

impl std::fmt::Display for ReductionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ReductionMethod {
    pub fn all() -> &'static [ReductionMethod] {
        &[
            ReductionMethod::QuadricErrorMetric,
            ReductionMethod::EdgeCollapse,
            ReductionMethod::VertexClustering,
            ReductionMethod::Simplygon,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ReductionMethod::QuadricErrorMetric => "Quadric Error Metric",
            ReductionMethod::EdgeCollapse => "Edge Collapse",
            ReductionMethod::VertexClustering => "Vertex Clustering",
            ReductionMethod::Simplygon => "Simplygon",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ReductionMethod::QuadricErrorMetric => "[Sq]",
            ReductionMethod::EdgeCollapse => "🔻",
            ReductionMethod::VertexClustering => "⚫",
            ReductionMethod::Simplygon => "[Wrn]",
        }
    }
}

impl Default for LodGenerationSettings {
    fn default() -> Self {
        Self {
            auto_generate: true,
            num_levels: 4,
            reduction_method: ReductionMethod::QuadricErrorMetric,
            target_reductions: [50.0, 75.0, 90.0, 95.0],
            preserve_uvs: true,
            preserve_normals: true,
            preserve_borders: true,
            weld_threshold: 0.001,
        }
    }
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LodTab {
    #[default]
    Groups,
    Global,
    Generation,
    Statistics,
}

impl std::fmt::Display for LodTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LodTab {
    pub fn all() -> &'static [LodTab] {
        &[
            LodTab::Groups,
            LodTab::Global,
            LodTab::Generation,
            LodTab::Statistics,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LodTab::Groups => "Groups",
            LodTab::Global => "Global",
            LodTab::Generation => "Generation",
            LodTab::Statistics => "Statistics",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LodTab::Groups => "[Pkg]",
            LodTab::Global => "[Glb]",
            LodTab::Generation => "⚙️",
            LodTab::Statistics => "[Chart]",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// ACTION SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════════

/// Actions that can be triggered from the LOD configuration panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LodConfigAction {
    // Tab navigation
    SetActiveTab(LodTab),

    // LOD group operations
    AddGroup,
    RemoveGroup(u32),
    SelectGroup(u32),
    DuplicateGroup(u32),
    SetGroupName(u32, String),
    SetGroupAssetPath(u32, String),
    ToggleGroupEnabled(u32, bool),

    // LOD level operations
    AddLevel(u32),
    RemoveLevel(u32, u32),
    SetLevelDistance(u32, u32, f32),
    SetLevelScreenCoverage(u32, u32, f32),
    SetLevelMesh(u32, u32, String),

    // Transition settings
    SetFadeMode(u32, FadeMode),
    SetFadeWidth(u32, f32),
    ToggleCrossFade(u32, bool),
    SetCullDistance(u32, f32),
    SetShadowLodOffset(u32, i32),

    // Global settings
    SetBiasMode(LodBiasMode),
    SetGlobalBias(f32),
    SetMaxLodLevel(u32),
    ToggleAutoLod(bool),
    SetScreenSizeThreshold(f32),

    // Generation settings
    SetReductionMethod(ReductionMethod),
    SetTargetTriangles(u32, f32),
    GenerateLods(u32),
    GenerateAllLods,

    // Preview
    SetPreviewDistance(f32),
    ToggleLodColors(bool),

    // Statistics
    RefreshStatistics,
    ExportReport,
}

impl std::fmt::Display for LodConfigAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LodConfigAction::SetActiveTab(tab) => write!(f, "Set tab: {}", tab),
            LodConfigAction::AddGroup => write!(f, "Add LOD group"),
            LodConfigAction::RemoveGroup(id) => write!(f, "Remove group {}", id),
            LodConfigAction::SelectGroup(id) => write!(f, "Select group {}", id),
            LodConfigAction::DuplicateGroup(id) => write!(f, "Duplicate group {}", id),
            LodConfigAction::SetGroupName(id, name) => write!(f, "Set group {} name: {}", id, name),
            LodConfigAction::SetGroupAssetPath(id, path) => {
                write!(f, "Set group {} path: {}", id, path)
            }
            LodConfigAction::ToggleGroupEnabled(id, b) => {
                write!(f, "Toggle group {} enabled: {}", id, b)
            }
            LodConfigAction::AddLevel(gid) => write!(f, "Add level to group {}", gid),
            LodConfigAction::RemoveLevel(gid, lid) => {
                write!(f, "Remove level {} from group {}", lid, gid)
            }
            LodConfigAction::SetLevelDistance(gid, lid, d) => {
                write!(f, "Set group {} level {} distance: {:.1}", gid, lid, d)
            }
            LodConfigAction::SetLevelScreenCoverage(gid, lid, c) => {
                write!(f, "Set group {} level {} coverage: {:.2}", gid, lid, c)
            }
            LodConfigAction::SetLevelMesh(gid, lid, m) => {
                write!(f, "Set group {} level {} mesh: {}", gid, lid, m)
            }
            LodConfigAction::SetFadeMode(id, mode) => {
                write!(f, "Set group {} fade: {:?}", id, mode)
            }
            LodConfigAction::SetFadeWidth(id, w) => {
                write!(f, "Set group {} fade width: {:.2}", id, w)
            }
            LodConfigAction::ToggleCrossFade(id, b) => {
                write!(f, "Toggle group {} crossfade: {}", id, b)
            }
            LodConfigAction::SetCullDistance(id, d) => write!(f, "Set group {} cull: {:.1}", id, d),
            LodConfigAction::SetShadowLodOffset(id, o) => {
                write!(f, "Set group {} shadow offset: {}", id, o)
            }
            LodConfigAction::SetBiasMode(mode) => write!(f, "Set bias mode: {}", mode),
            LodConfigAction::SetGlobalBias(b) => write!(f, "Set global bias: {:.2}", b),
            LodConfigAction::SetMaxLodLevel(l) => write!(f, "Set max LOD: {}", l),
            LodConfigAction::ToggleAutoLod(b) => write!(f, "Toggle auto LOD: {}", b),
            LodConfigAction::SetScreenSizeThreshold(t) => {
                write!(f, "Set screen threshold: {:.2}", t)
            }
            LodConfigAction::SetReductionMethod(m) => write!(f, "Set reduction: {:?}", m),
            LodConfigAction::SetTargetTriangles(id, t) => {
                write!(f, "Set group {} target: {:.0}%", id, t)
            }
            LodConfigAction::GenerateLods(id) => write!(f, "Generate LODs for group {}", id),
            LodConfigAction::GenerateAllLods => write!(f, "Generate all LODs"),
            LodConfigAction::SetPreviewDistance(d) => write!(f, "Set preview distance: {:.1}", d),
            LodConfigAction::ToggleLodColors(b) => write!(f, "Toggle LOD colors: {}", b),
            LodConfigAction::RefreshStatistics => write!(f, "Refresh statistics"),
            LodConfigAction::ExportReport => write!(f, "Export report"),
        }
    }
}

/// Main LOD Configuration Panel
pub struct LodConfigPanel {
    active_tab: LodTab,

    // LOD groups
    lod_groups: Vec<LodGroup>,
    selected_group: Option<u32>,
    current_group: LodGroup,

    // Settings
    global_settings: GlobalLodSettings,
    generation_settings: LodGenerationSettings,

    // Preview
    preview_distance: f32,
    show_lod_colors: bool,

    // ID counter
    next_group_id: u32,

    // Action system
    actions: Vec<LodConfigAction>,
}

impl Default for LodConfigPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: LodTab::Groups,

            lod_groups: Vec::new(),
            selected_group: None,
            current_group: LodGroup::default(),

            global_settings: GlobalLodSettings::default(),
            generation_settings: LodGenerationSettings::default(),

            preview_distance: 0.0,
            show_lod_colors: true,

            next_group_id: 1,

            actions: Vec::new(),
        };

        panel
    }
}

impl LodConfigPanel {
    pub fn new() -> Self {
        Self::default()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Action System
    // ─────────────────────────────────────────────────────────────────────────────

    /// Queue an action for later processing
    pub fn queue_action(&mut self, action: LodConfigAction) {
        self.actions.push(action);
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Get pending actions without consuming them
    pub fn pending_actions(&self) -> &[LodConfigAction] {
        &self.actions
    }

    /// Take all pending actions, clearing the queue
    pub fn take_actions(&mut self) -> Vec<LodConfigAction> {
        std::mem::take(&mut self.actions)
    }

    fn create_sample_data(&mut self) {
        // Sample character LOD
        let id = self.next_group_id();
        let mut character_lod = LodGroup {
            id,
            name: "Player Character".to_string(),
            asset_path: "models/characters/player.fbx".to_string(),
            base_triangles: 25000,
            ..Default::default()
        };

        character_lod.levels = vec![
            LodLevel {
                level: 0,
                mesh_path: "player_lod0.mesh".to_string(),
                distance: 0.0,
                screen_coverage: 1.0,
                triangle_count: 25000,
                vertex_count: 15000,
                reduction_percent: 0.0,
            },
            LodLevel {
                level: 1,
                mesh_path: "player_lod1.mesh".to_string(),
                distance: 15.0,
                screen_coverage: 0.5,
                triangle_count: 12500,
                vertex_count: 7500,
                reduction_percent: 50.0,
            },
            LodLevel {
                level: 2,
                mesh_path: "player_lod2.mesh".to_string(),
                distance: 35.0,
                screen_coverage: 0.25,
                triangle_count: 6250,
                vertex_count: 3750,
                reduction_percent: 75.0,
            },
            LodLevel {
                level: 3,
                mesh_path: "player_lod3.mesh".to_string(),
                distance: 70.0,
                screen_coverage: 0.1,
                triangle_count: 2500,
                vertex_count: 1500,
                reduction_percent: 90.0,
            },
        ];

        self.lod_groups.push(character_lod.clone());
        self.current_group = character_lod;
        self.selected_group = Some(id);

        // Sample tree LOD
        let id = self.next_group_id();
        let mut tree_lod = LodGroup {
            id,
            name: "Pine Tree".to_string(),
            asset_path: "models/vegetation/pine_tree.fbx".to_string(),
            base_triangles: 8000,
            cull_distance: 500.0,
            ..Default::default()
        };

        tree_lod.levels = vec![
            LodLevel {
                level: 0,
                distance: 0.0,
                triangle_count: 8000,
                vertex_count: 5000,
                reduction_percent: 0.0,
                ..Default::default()
            },
            LodLevel {
                level: 1,
                distance: 25.0,
                triangle_count: 4000,
                vertex_count: 2500,
                reduction_percent: 50.0,
                ..Default::default()
            },
            LodLevel {
                level: 2,
                distance: 75.0,
                triangle_count: 1000,
                vertex_count: 600,
                reduction_percent: 87.5,
                ..Default::default()
            },
            LodLevel {
                level: 3,
                distance: 150.0,
                triangle_count: 100,
                vertex_count: 60,
                reduction_percent: 98.75,
                ..Default::default()
            },
        ];

        self.lod_groups.push(tree_lod);

        // Sample building LOD
        let id = self.next_group_id();
        let mut building_lod = LodGroup {
            id,
            name: "House Medium".to_string(),
            asset_path: "models/buildings/house_medium.fbx".to_string(),
            base_triangles: 15000,
            cull_distance: 800.0,
            ..Default::default()
        };

        building_lod.levels = vec![
            LodLevel {
                level: 0,
                distance: 0.0,
                triangle_count: 15000,
                vertex_count: 9000,
                reduction_percent: 0.0,
                ..Default::default()
            },
            LodLevel {
                level: 1,
                distance: 50.0,
                triangle_count: 7500,
                vertex_count: 4500,
                reduction_percent: 50.0,
                ..Default::default()
            },
            LodLevel {
                level: 2,
                distance: 150.0,
                triangle_count: 3000,
                vertex_count: 1800,
                reduction_percent: 80.0,
                ..Default::default()
            },
        ];

        self.lod_groups.push(building_lod);
    }

    fn next_group_id(&mut self) -> u32 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (LodTab::Groups, "Groups"),
                (LodTab::Global, "[Glb] Global"),
                (LodTab::Generation, "Generation"),
                (LodTab::Statistics, "Statistics"),
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

        ui.horizontal(|ui| {
            ui.label(format!("{} LOD groups", self.lod_groups.len()));
            ui.separator();
            ui.label(format!("🎚️ {:?} bias", self.global_settings.bias_mode));
        });

        ui.separator();
    }

    fn show_groups_tab(&mut self, ui: &mut Ui) {
        ui.heading("LOD Groups");
        ui.add_space(10.0);

        // Group list
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Groups").strong());
                if ui.button("+ New").clicked() {
                    let id = self.next_group_id();
                    info!(group_id = id, "LOD: added new group");
                    let new_group = LodGroup {
                        id,
                        name: format!("LOD Group {}", id),
                        ..Default::default()
                    };
                    self.lod_groups.push(new_group.clone());
                    self.current_group = new_group;
                    self.selected_group = Some(id);
                }
            });

            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    for group in &self.lod_groups.clone() {
                        let is_selected = self.selected_group == Some(group.id);
                        let label = format!(
                            "{} ({} levels, {} tris)",
                            group.name,
                            group.levels.len(),
                            group.base_triangles
                        );

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selected_group = Some(group.id);
                            self.current_group = group.clone();
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // Group properties
        ui.group(|ui| {
            ui.label(RichText::new("Properties").strong());

            egui::Grid::new("group_props")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.current_group.name);
                    ui.end_row();

                    ui.label("Asset:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.current_group.asset_path);
                        if ui.button("[Open]").clicked() {}
                    });
                    ui.end_row();

                    ui.label("Enabled:");
                    ui.checkbox(&mut self.current_group.enabled, "");
                    ui.end_row();

                    ui.label("Fade Mode:");
                    egui::ComboBox::from_id_salt("fade_mode")
                        .selected_text(format!("{:?}", self.current_group.fade_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.current_group.fade_mode,
                                FadeMode::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut self.current_group.fade_mode,
                                FadeMode::CrossFade,
                                "Cross Fade",
                            );
                            ui.selectable_value(
                                &mut self.current_group.fade_mode,
                                FadeMode::SpeedTree,
                                "SpeedTree",
                            );
                            ui.selectable_value(
                                &mut self.current_group.fade_mode,
                                FadeMode::Dither,
                                "Dither",
                            );
                        });
                    ui.end_row();

                    ui.label("Cull Distance:");
                    ui.add(
                        egui::DragValue::new(&mut self.current_group.cull_distance)
                            .speed(1.0)
                            .suffix("m"),
                    );
                    ui.end_row();

                    ui.label("Shadow LOD Offset:");
                    ui.add(
                        egui::DragValue::new(&mut self.current_group.shadow_lod_offset).speed(1),
                    );
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // LOD levels
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("LOD Levels").strong());
                if ui.button("+ Add Level").clicked() {
                    let level = self.current_group.levels.len() as u32;
                    info!(level = level, "LOD: added level");
                    let last_distance = self
                        .current_group
                        .levels
                        .last()
                        .map(|l| l.distance)
                        .unwrap_or(0.0);

                    self.current_group.levels.push(LodLevel {
                        level,
                        distance: last_distance + 25.0,
                        ..Default::default()
                    });
                }
            });

            // Level table
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    egui::Grid::new("lod_levels")
                        .num_columns(5)
                        .spacing([10.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Header
                            ui.label(RichText::new("LOD").strong());
                            ui.label(RichText::new("Distance").strong());
                            ui.label(RichText::new("Triangles").strong());
                            ui.label(RichText::new("Reduction").strong());
                            ui.label(RichText::new("Actions").strong());
                            ui.end_row();

                            for level in &mut self.current_group.levels {
                                let color = match level.level {
                                    0 => Color32::from_rgb(50, 200, 50),
                                    1 => Color32::from_rgb(200, 200, 50),
                                    2 => Color32::from_rgb(200, 150, 50),
                                    3 => Color32::from_rgb(200, 100, 50),
                                    _ => Color32::from_rgb(200, 50, 50),
                                };

                                ui.label(
                                    RichText::new(format!("LOD {}", level.level)).color(color),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut level.distance)
                                        .speed(1.0)
                                        .suffix("m"),
                                );
                                ui.label(format!("{}", level.triangle_count));
                                ui.label(format!("{:.1}%", level.reduction_percent));

                                if ui.button("[Del]").clicked() {
                                    // Remove level
                                }
                                ui.end_row();
                            }
                        });
                });
        });

        ui.add_space(10.0);

        // Preview
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Preview").strong());
                ui.checkbox(&mut self.show_lod_colors, "Show LOD Colors");
            });

            ui.horizontal(|ui| {
                ui.label("Distance:");
                ui.add(egui::Slider::new(&mut self.preview_distance, 0.0..=500.0).suffix("m"));
            });

            // Show current LOD at preview distance
            let current_lod = self
                .current_group
                .levels
                .iter()
                .filter(|l| l.distance <= self.preview_distance)
                .next_back()
                .map(|l| l.level)
                .unwrap_or(0);

            ui.label(format!(
                "Current LOD: {} at {:.0}m",
                current_lod, self.preview_distance
            ));

            // Visual bar showing LOD ranges
            let bar_height = 20.0;
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), bar_height),
                egui::Sense::hover(),
            );

            let painter = ui.painter();
            let total_distance = self.current_group.cull_distance;

            for (i, level) in self.current_group.levels.iter().enumerate() {
                let next_distance = self
                    .current_group
                    .levels
                    .get(i + 1)
                    .map(|l| l.distance)
                    .unwrap_or(total_distance);

                let start_x = rect.min.x + (level.distance / total_distance) * rect.width();
                let end_x = rect.min.x + (next_distance / total_distance) * rect.width();

                let color = match level.level {
                    0 => Color32::from_rgb(50, 200, 50),
                    1 => Color32::from_rgb(200, 200, 50),
                    2 => Color32::from_rgb(200, 150, 50),
                    3 => Color32::from_rgb(200, 100, 50),
                    _ => Color32::from_rgb(200, 50, 50),
                };

                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::Pos2::new(start_x, rect.min.y),
                        egui::Pos2::new(end_x, rect.max.y),
                    ),
                    0.0,
                    color,
                );
            }

            // Draw preview distance marker
            let marker_x = rect.min.x + (self.preview_distance / total_distance) * rect.width();
            painter.line_segment(
                [
                    egui::Pos2::new(marker_x, rect.min.y),
                    egui::Pos2::new(marker_x, rect.max.y),
                ],
                egui::Stroke::new(2.0, Color32::WHITE),
            );
        });
    }

    fn show_global_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Glb] Global LOD Settings");
        ui.add_space(10.0);

        // Quality preset
        ui.group(|ui| {
            ui.label(RichText::new("🎚️ Quality Preset").strong());

            egui::Grid::new("quality_preset")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Bias Mode:");
                    egui::ComboBox::from_id_salt("bias_mode")
                        .selected_text(format!("{:?}", self.global_settings.bias_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.global_settings.bias_mode,
                                LodBiasMode::Auto,
                                "Auto",
                            );
                            ui.selectable_value(
                                &mut self.global_settings.bias_mode,
                                LodBiasMode::Quality,
                                "Quality",
                            );
                            ui.selectable_value(
                                &mut self.global_settings.bias_mode,
                                LodBiasMode::Balanced,
                                "Balanced",
                            );
                            ui.selectable_value(
                                &mut self.global_settings.bias_mode,
                                LodBiasMode::Performance,
                                "Performance",
                            );
                            ui.selectable_value(
                                &mut self.global_settings.bias_mode,
                                LodBiasMode::Custom,
                                "Custom",
                            );
                        });
                    ui.end_row();

                    if self.global_settings.bias_mode == LodBiasMode::Custom {
                        ui.label("Custom Bias:");
                        ui.add(egui::Slider::new(
                            &mut self.global_settings.custom_bias,
                            0.25..=4.0,
                        ));
                        ui.end_row();
                    }

                    ui.label("Max LOD Level:");
                    ui.add(egui::Slider::new(
                        &mut self.global_settings.maximum_lod_level,
                        0..=8,
                    ));
                    ui.end_row();

                    ui.label("Cross Fade Time:");
                    ui.add(
                        egui::Slider::new(&mut self.global_settings.lod_cross_fade_time, 0.0..=2.0)
                            .suffix("s"),
                    );
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Distance presets
        ui.group(|ui| {
            ui.label(RichText::new("📏 Distance Presets").strong());

            ui.collapsing("Quality Distances", |ui| {
                for (i, dist) in self
                    .global_settings
                    .quality_distances
                    .iter_mut()
                    .enumerate()
                {
                    ui.horizontal(|ui| {
                        ui.label(format!("LOD {}:", i + 1));
                        ui.add(egui::DragValue::new(dist).speed(1.0).suffix("m"));
                    });
                }
            });

            ui.collapsing("Balanced Distances", |ui| {
                for (i, dist) in self
                    .global_settings
                    .balanced_distances
                    .iter_mut()
                    .enumerate()
                {
                    ui.horizontal(|ui| {
                        ui.label(format!("LOD {}:", i + 1));
                        ui.add(egui::DragValue::new(dist).speed(1.0).suffix("m"));
                    });
                }
            });

            ui.collapsing("Performance Distances", |ui| {
                for (i, dist) in self
                    .global_settings
                    .performance_distances
                    .iter_mut()
                    .enumerate()
                {
                    ui.horizontal(|ui| {
                        ui.label(format!("LOD {}:", i + 1));
                        ui.add(egui::DragValue::new(dist).speed(1.0).suffix("m"));
                    });
                }
            });
        });

        ui.add_space(10.0);

        // Screen coverage
        ui.group(|ui| {
            ui.label(RichText::new("[Sq] Screen Coverage").strong());

            ui.checkbox(
                &mut self.global_settings.screen_coverage_enabled,
                "Enable Screen Coverage LOD",
            );

            if self.global_settings.screen_coverage_enabled {
                ui.horizontal(|ui| {
                    ui.label("Min Coverage:");
                    ui.add(egui::Slider::new(
                        &mut self.global_settings.min_screen_coverage,
                        0.001..=0.1,
                    ));
                });
            }
        });

        ui.add_space(10.0);

        // Shadow settings
        ui.group(|ui| {
            ui.label(RichText::new("🌑 Shadow LOD").strong());

            egui::Grid::new("shadow_lod")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Shadow LOD Bias:");
                    ui.add(egui::Slider::new(
                        &mut self.global_settings.shadow_lod_bias,
                        -2..=4,
                    ));
                    ui.end_row();

                    ui.label("Shadow Cull Distance:");
                    ui.add(
                        egui::DragValue::new(&mut self.global_settings.shadow_cull_distance)
                            .speed(10.0)
                            .suffix("m"),
                    );
                    ui.end_row();
                });
        });
    }

    fn show_generation_tab(&mut self, ui: &mut Ui) {
        ui.heading("LOD Generation");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Generation Settings").strong());

            egui::Grid::new("generation_settings")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Auto Generate:");
                    ui.checkbox(&mut self.generation_settings.auto_generate, "");
                    ui.end_row();

                    ui.label("Number of Levels:");
                    ui.add(egui::Slider::new(
                        &mut self.generation_settings.num_levels,
                        1..=6,
                    ));
                    ui.end_row();

                    ui.label("Reduction Method:");
                    egui::ComboBox::from_id_salt("reduction_method")
                        .selected_text(format!("{:?}", self.generation_settings.reduction_method))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.generation_settings.reduction_method,
                                ReductionMethod::QuadricErrorMetric,
                                "Quadric Error Metric",
                            );
                            ui.selectable_value(
                                &mut self.generation_settings.reduction_method,
                                ReductionMethod::EdgeCollapse,
                                "Edge Collapse",
                            );
                            ui.selectable_value(
                                &mut self.generation_settings.reduction_method,
                                ReductionMethod::VertexClustering,
                                "Vertex Clustering",
                            );
                            ui.selectable_value(
                                &mut self.generation_settings.reduction_method,
                                ReductionMethod::Simplygon,
                                "Simplygon",
                            );
                        });
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Target reductions
        ui.group(|ui| {
            ui.label(RichText::new("📉 Target Reductions").strong());

            for (i, reduction) in self
                .generation_settings
                .target_reductions
                .iter_mut()
                .enumerate()
            {
                ui.horizontal(|ui| {
                    ui.label(format!("LOD {} Reduction:", i + 1));
                    ui.add(egui::Slider::new(reduction, 0.0..=99.0).suffix("%"));
                });
            }
        });

        ui.add_space(10.0);

        // Preservation settings
        ui.group(|ui| {
            ui.label(RichText::new("Preservation").strong());

            ui.checkbox(&mut self.generation_settings.preserve_uvs, "Preserve UVs");
            ui.checkbox(
                &mut self.generation_settings.preserve_normals,
                "Preserve Normals",
            );
            ui.checkbox(
                &mut self.generation_settings.preserve_borders,
                "Preserve Borders",
            );

            ui.horizontal(|ui| {
                ui.label("Weld Threshold:");
                ui.add(
                    egui::DragValue::new(&mut self.generation_settings.weld_threshold)
                        .speed(0.0001)
                        .min_decimals(4),
                );
            });
        });

        ui.add_space(10.0);

        // Actions
        ui.horizontal(|ui| {
            if ui.button("Generate LODs for Selected").clicked() {
                info!(group = ?self.selected_group, "LOD: generating for selected group");
                // Generate LODs for current group
            }
            if ui.button("Generate All LODs").clicked() {
                info!(
                    group_count = self.lod_groups.len(),
                    "LOD: generating for all groups"
                );
                // Generate LODs for all groups
            }
        });
    }

    fn show_statistics_tab(&mut self, ui: &mut Ui) {
        ui.heading("LOD Statistics");
        ui.add_space(10.0);

        // Overall stats
        let total_groups = self.lod_groups.len();
        let total_levels: usize = self.lod_groups.iter().map(|g| g.levels.len()).sum();
        let total_base_tris: u32 = self.lod_groups.iter().map(|g| g.base_triangles).sum();

        ui.group(|ui| {
            ui.label(RichText::new("📈 Overview").strong());

            egui::Grid::new("overview_stats")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Total LOD Groups:");
                    ui.label(RichText::new(format!("{}", total_groups)).strong());
                    ui.end_row();

                    ui.label("Total LOD Levels:");
                    ui.label(RichText::new(format!("{}", total_levels)).strong());
                    ui.end_row();

                    ui.label("Total Base Triangles:");
                    ui.label(RichText::new(format!("{}", total_base_tris)).strong());
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Per-group breakdown
        ui.group(|ui| {
            ui.label(RichText::new("Per-Group Statistics").strong());

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for group in &self.lod_groups {
                        ui.collapsing(&group.name, |ui| {
                            ui.label(format!("Base Triangles: {}", group.base_triangles));
                            ui.label(format!("Cull Distance: {}m", group.cull_distance));
                            ui.label(format!("LOD Levels: {}", group.levels.len()));

                            for level in &group.levels {
                                let savings = if group.base_triangles > 0 {
                                    100.0
                                        * (1.0
                                            - level.triangle_count as f32
                                                / group.base_triangles as f32)
                                } else {
                                    0.0
                                };

                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "  LOD {}: {} tris ({:.1}% saved)",
                                        level.level, level.triangle_count, savings
                                    ));
                                });
                            }
                        });
                    }
                });
        });

        ui.add_space(10.0);

        // Memory estimate
        ui.group(|ui| {
            ui.label(RichText::new("Memory Estimate").strong());

            // Rough estimate: 32 bytes per vertex
            let total_vertices: u32 = self
                .lod_groups
                .iter()
                .flat_map(|g| &g.levels)
                .map(|l| l.vertex_count)
                .sum();

            let estimated_mb = total_vertices as f32 * 32.0 / 1024.0 / 1024.0;

            ui.label(format!("Total Vertices: {}", total_vertices));
            ui.label(format!("Estimated Memory: {:.2} MB", estimated_mb));
        });
    }

    // Getters for testing
    pub fn group_count(&self) -> usize {
        self.lod_groups.len()
    }

    pub fn level_count(&self) -> usize {
        self.current_group.levels.len()
    }

    pub fn add_lod_group(&mut self, name: &str) -> u32 {
        let id = self.next_group_id();
        self.lod_groups.push(LodGroup {
            id,
            name: name.to_string(),
            ..Default::default()
        });
        id
    }

    pub fn set_bias_mode(&mut self, mode: LodBiasMode) {
        self.global_settings.bias_mode = mode;
    }
}

impl Panel for LodConfigPanel {
    fn name(&self) -> &'static str {
        "LOD Configuration"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            LodTab::Groups => self.show_groups_tab(ui),
            LodTab::Global => self.show_global_tab(ui),
            LodTab::Generation => self.show_generation_tab(ui),
            LodTab::Statistics => self.show_statistics_tab(ui),
        }
    }

    fn update(&mut self) {
        // Sync current group back to list
        if let Some(group_id) = self.selected_group {
            if let Some(group) = self.lod_groups.iter_mut().find(|g| g.id == group_id) {
                *group = self.current_group.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // LOD BIAS MODE TESTS
    // ============================================================================

    #[test]
    fn test_lod_bias_mode_default() {
        let mode: LodBiasMode = Default::default();
        assert_eq!(mode, LodBiasMode::Auto);
    }

    #[test]
    fn test_lod_bias_mode_variants() {
        assert_eq!(LodBiasMode::Auto, LodBiasMode::Auto);
        assert_eq!(LodBiasMode::Quality, LodBiasMode::Quality);
        assert_eq!(LodBiasMode::Balanced, LodBiasMode::Balanced);
        assert_eq!(LodBiasMode::Performance, LodBiasMode::Performance);
        assert_eq!(LodBiasMode::Custom, LodBiasMode::Custom);
    }

    #[test]
    fn test_lod_bias_mode_clone() {
        let mode = LodBiasMode::Quality;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    // ============================================================
    // Session 5: Enum Enhancement Tests
    // ============================================================

    // LodBiasMode tests (7 tests)
    #[test]
    fn test_lod_bias_mode_display() {
        assert!(format!("{}", LodBiasMode::Auto).contains("Auto"));
        assert!(format!("{}", LodBiasMode::Quality).contains("Quality"));
        assert!(format!("{}", LodBiasMode::Balanced).contains("Balanced"));
        assert!(format!("{}", LodBiasMode::Performance).contains("Performance"));
    }

    #[test]
    fn test_lod_bias_mode_name() {
        assert_eq!(LodBiasMode::Auto.name(), "Auto");
        assert_eq!(LodBiasMode::Quality.name(), "Quality");
        assert_eq!(LodBiasMode::Balanced.name(), "Balanced");
        assert_eq!(LodBiasMode::Custom.name(), "Custom");
    }

    #[test]
    fn test_lod_bias_mode_icon_present() {
        assert!(!LodBiasMode::Auto.icon().is_empty());
        assert!(!LodBiasMode::Quality.icon().is_empty());
        assert!(!LodBiasMode::Balanced.icon().is_empty());
        assert!(!LodBiasMode::Performance.icon().is_empty());
    }

    #[test]
    fn test_lod_bias_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in LodBiasMode::all() {
            assert!(set.insert(*mode));
        }
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_lod_bias_mode_default_value() {
        assert_eq!(LodBiasMode::default(), LodBiasMode::Auto);
    }

    #[test]
    fn test_lod_bias_mode_all_have_names() {
        for mode in LodBiasMode::all() {
            assert!(!mode.name().is_empty());
        }
    }

    #[test]
    fn test_lod_bias_mode_all_have_icons() {
        for mode in LodBiasMode::all() {
            assert!(!mode.icon().is_empty());
        }
    }

    // FadeMode tests (7 tests)
    #[test]
    fn test_fade_mode_display() {
        assert!(format!("{}", FadeMode::None).contains("None"));
        assert!(format!("{}", FadeMode::CrossFade).contains("Cross Fade"));
        assert!(format!("{}", FadeMode::SpeedTree).contains("SpeedTree"));
        assert!(format!("{}", FadeMode::Dither).contains("Dither"));
    }

    #[test]
    fn test_fade_mode_name() {
        assert_eq!(FadeMode::None.name(), "None");
        assert_eq!(FadeMode::CrossFade.name(), "Cross Fade");
        assert_eq!(FadeMode::SpeedTree.name(), "SpeedTree");
        assert_eq!(FadeMode::Dither.name(), "Dither");
    }

    #[test]
    fn test_fade_mode_icon_present() {
        assert!(!FadeMode::None.icon().is_empty());
        assert!(!FadeMode::CrossFade.icon().is_empty());
        assert!(!FadeMode::SpeedTree.icon().is_empty());
        assert!(!FadeMode::Dither.icon().is_empty());
    }

    #[test]
    fn test_fade_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mode in FadeMode::all() {
            assert!(set.insert(*mode));
        }
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_fade_mode_default_value() {
        assert_eq!(FadeMode::default(), FadeMode::None);
    }

    #[test]
    fn test_fade_mode_all_have_names() {
        for mode in FadeMode::all() {
            assert!(!mode.name().is_empty());
        }
    }

    #[test]
    fn test_fade_mode_all_have_icons() {
        for mode in FadeMode::all() {
            assert!(!mode.icon().is_empty());
        }
    }

    // ReductionMethod tests (7 tests)
    #[test]
    fn test_reduction_method_display() {
        assert!(format!("{}", ReductionMethod::QuadricErrorMetric).contains("Quadric Error Metric"));
        assert!(format!("{}", ReductionMethod::EdgeCollapse).contains("Edge Collapse"));
        assert!(format!("{}", ReductionMethod::VertexClustering).contains("Vertex Clustering"));
        assert!(format!("{}", ReductionMethod::Simplygon).contains("Simplygon"));
    }

    #[test]
    fn test_reduction_method_name() {
        assert_eq!(
            ReductionMethod::QuadricErrorMetric.name(),
            "Quadric Error Metric"
        );
        assert_eq!(ReductionMethod::EdgeCollapse.name(), "Edge Collapse");
        assert_eq!(
            ReductionMethod::VertexClustering.name(),
            "Vertex Clustering"
        );
        assert_eq!(ReductionMethod::Simplygon.name(), "Simplygon");
    }

    #[test]
    fn test_reduction_method_icon_present() {
        assert!(!ReductionMethod::QuadricErrorMetric.icon().is_empty());
        assert!(!ReductionMethod::EdgeCollapse.icon().is_empty());
        assert!(!ReductionMethod::VertexClustering.icon().is_empty());
        assert!(!ReductionMethod::Simplygon.icon().is_empty());
    }

    #[test]
    fn test_reduction_method_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for method in ReductionMethod::all() {
            assert!(set.insert(*method));
        }
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_reduction_method_default_value() {
        assert_eq!(
            ReductionMethod::default(),
            ReductionMethod::QuadricErrorMetric
        );
    }

    #[test]
    fn test_reduction_method_all_have_names() {
        for method in ReductionMethod::all() {
            assert!(!method.name().is_empty());
        }
    }

    #[test]
    fn test_reduction_method_all_have_icons() {
        for method in ReductionMethod::all() {
            assert!(!method.icon().is_empty());
        }
    }

    // LodTab tests (7 tests)
    #[test]
    fn test_lod_tab_display() {
        assert!(format!("{}", LodTab::Groups).contains("Groups"));
        assert!(format!("{}", LodTab::Global).contains("Global"));
        assert!(format!("{}", LodTab::Generation).contains("Generation"));
        assert!(format!("{}", LodTab::Statistics).contains("Statistics"));
    }

    #[test]
    fn test_lod_tab_name() {
        assert_eq!(LodTab::Groups.name(), "Groups");
        assert_eq!(LodTab::Global.name(), "Global");
        assert_eq!(LodTab::Generation.name(), "Generation");
        assert_eq!(LodTab::Statistics.name(), "Statistics");
    }

    #[test]
    fn test_lod_tab_icon_present() {
        assert!(!LodTab::Groups.icon().is_empty());
        assert!(!LodTab::Global.icon().is_empty());
        assert!(!LodTab::Generation.icon().is_empty());
        assert!(!LodTab::Statistics.icon().is_empty());
    }

    #[test]
    fn test_lod_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in LodTab::all() {
            assert!(set.insert(*tab));
        }
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_lod_tab_default_value() {
        assert_eq!(LodTab::default(), LodTab::Groups);
    }

    #[test]
    fn test_lod_tab_all_have_names() {
        for tab in LodTab::all() {
            assert!(!tab.name().is_empty());
        }
    }

    #[test]
    fn test_lod_tab_all_have_icons() {
        for tab in LodTab::all() {
            assert!(!tab.icon().is_empty());
        }
    }

    // ============================================================================
    // LOD LEVEL TESTS
    // ============================================================================

    #[test]
    fn test_lod_level_default() {
        let level = LodLevel::default();
        assert_eq!(level.level, 0);
        assert!(level.mesh_path.is_empty());
        assert_eq!(level.distance, 0.0);
        assert_eq!(level.screen_coverage, 1.0);
        assert_eq!(level.triangle_count, 0);
        assert_eq!(level.vertex_count, 0);
        assert_eq!(level.reduction_percent, 0.0);
    }

    #[test]
    fn test_lod_level_custom() {
        let level = LodLevel {
            level: 2,
            mesh_path: "model_lod2.mesh".to_string(),
            distance: 50.0,
            screen_coverage: 0.25,
            triangle_count: 1000,
            vertex_count: 600,
            reduction_percent: 75.0,
        };
        assert_eq!(level.level, 2);
        assert_eq!(level.mesh_path, "model_lod2.mesh");
        assert_eq!(level.distance, 50.0);
    }

    #[test]
    fn test_lod_level_clone() {
        let level = LodLevel {
            level: 1,
            mesh_path: "test.mesh".to_string(),
            distance: 25.0,
            screen_coverage: 0.5,
            triangle_count: 5000,
            vertex_count: 3000,
            reduction_percent: 50.0,
        };
        let cloned = level.clone();
        assert_eq!(cloned.level, 1);
        assert_eq!(cloned.mesh_path, "test.mesh");
    }

    // ============================================================================
    // FADE MODE TESTS
    // ============================================================================

    #[test]
    fn test_fade_mode_default() {
        let mode: FadeMode = Default::default();
        assert_eq!(mode, FadeMode::None);
    }

    #[test]
    fn test_fade_mode_variants() {
        assert_eq!(FadeMode::None, FadeMode::None);
        assert_eq!(FadeMode::CrossFade, FadeMode::CrossFade);
        assert_eq!(FadeMode::SpeedTree, FadeMode::SpeedTree);
        assert_eq!(FadeMode::Dither, FadeMode::Dither);
    }

    #[test]
    fn test_fade_mode_copy() {
        let mode = FadeMode::CrossFade;
        let copied = mode;
        assert_eq!(mode, copied);
    }

    // ============================================================================
    // LOD GROUP TESTS
    // ============================================================================

    #[test]
    fn test_lod_group_default() {
        let group = LodGroup::default();
        assert_eq!(group.id, 0);
        assert_eq!(group.name, "New LOD Group");
        assert!(group.asset_path.is_empty());
        assert!(group.enabled);
        assert!(group.levels.is_empty());
        assert_eq!(group.fade_mode, FadeMode::CrossFade);
        assert_eq!(group.fade_width, 0.1);
        assert!(group.cross_fade);
        assert_eq!(group.cull_distance, 1000.0);
        assert_eq!(group.shadow_lod_offset, 1);
        assert_eq!(group.base_triangles, 0);
        assert_eq!(group.current_level, 0);
    }

    #[test]
    fn test_lod_group_custom() {
        let group = LodGroup {
            id: 5,
            name: "Hero Character".to_string(),
            asset_path: "hero.fbx".to_string(),
            enabled: false,
            levels: vec![LodLevel::default()],
            fade_mode: FadeMode::Dither,
            fade_width: 0.2,
            cross_fade: false,
            cull_distance: 500.0,
            shadow_lod_offset: 2,
            base_triangles: 10000,
            current_level: 1,
        };
        assert_eq!(group.id, 5);
        assert_eq!(group.name, "Hero Character");
        assert!(!group.enabled);
        assert_eq!(group.levels.len(), 1);
    }

    #[test]
    fn test_lod_group_clone() {
        let group = LodGroup {
            id: 3,
            name: "Tree".to_string(),
            ..Default::default()
        };
        let cloned = group.clone();
        assert_eq!(cloned.id, 3);
        assert_eq!(cloned.name, "Tree");
    }

    // ============================================================================
    // GLOBAL LOD SETTINGS TESTS
    // ============================================================================

    #[test]
    fn test_global_lod_settings_default() {
        let settings = GlobalLodSettings::default();
        assert_eq!(settings.bias_mode, LodBiasMode::Balanced);
        assert_eq!(settings.custom_bias, 1.0);
        assert_eq!(settings.maximum_lod_level, 4);
        assert_eq!(settings.lod_cross_fade_time, 0.5);
        assert!(!settings.screen_coverage_enabled);
        assert_eq!(settings.min_screen_coverage, 0.01);
        assert_eq!(settings.shadow_lod_bias, 1);
        assert_eq!(settings.shadow_cull_distance, 500.0);
    }

    #[test]
    fn test_global_lod_settings_quality_distances() {
        let settings = GlobalLodSettings::default();
        assert_eq!(settings.quality_distances, [20.0, 50.0, 100.0, 200.0]);
    }

    #[test]
    fn test_global_lod_settings_balanced_distances() {
        let settings = GlobalLodSettings::default();
        assert_eq!(settings.balanced_distances, [15.0, 35.0, 70.0, 150.0]);
    }

    #[test]
    fn test_global_lod_settings_performance_distances() {
        let settings = GlobalLodSettings::default();
        assert_eq!(settings.performance_distances, [10.0, 25.0, 50.0, 100.0]);
    }

    #[test]
    fn test_global_lod_settings_clone() {
        let settings = GlobalLodSettings::default();
        let cloned = settings.clone();
        assert_eq!(cloned.bias_mode, LodBiasMode::Balanced);
    }

    // ============================================================================
    // REDUCTION METHOD TESTS
    // ============================================================================

    #[test]
    fn test_reduction_method_default() {
        let method: ReductionMethod = Default::default();
        assert_eq!(method, ReductionMethod::QuadricErrorMetric);
    }

    #[test]
    fn test_reduction_method_variants() {
        assert_eq!(
            ReductionMethod::QuadricErrorMetric,
            ReductionMethod::QuadricErrorMetric
        );
        assert_eq!(ReductionMethod::EdgeCollapse, ReductionMethod::EdgeCollapse);
        assert_eq!(
            ReductionMethod::VertexClustering,
            ReductionMethod::VertexClustering
        );
        assert_eq!(ReductionMethod::Simplygon, ReductionMethod::Simplygon);
    }

    // ============================================================================
    // LOD GENERATION SETTINGS TESTS
    // ============================================================================

    #[test]
    fn test_lod_generation_settings_default() {
        let settings = LodGenerationSettings::default();
        assert!(settings.auto_generate);
        assert_eq!(settings.num_levels, 4);
        assert_eq!(
            settings.reduction_method,
            ReductionMethod::QuadricErrorMetric
        );
        assert_eq!(settings.target_reductions, [50.0, 75.0, 90.0, 95.0]);
        assert!(settings.preserve_uvs);
        assert!(settings.preserve_normals);
        assert!(settings.preserve_borders);
        assert_eq!(settings.weld_threshold, 0.001);
    }

    #[test]
    fn test_lod_generation_settings_clone() {
        let settings = LodGenerationSettings::default();
        let cloned = settings.clone();
        assert!(cloned.auto_generate);
        assert_eq!(cloned.num_levels, 4);
    }

    // ============================================================================
    // LOD TAB TESTS
    // ============================================================================

    #[test]
    fn test_lod_tab_default() {
        let tab: LodTab = Default::default();
        assert_eq!(tab, LodTab::Groups);
    }

    #[test]
    fn test_lod_tab_variants() {
        assert_eq!(LodTab::Groups, LodTab::Groups);
        assert_eq!(LodTab::Global, LodTab::Global);
        assert_eq!(LodTab::Generation, LodTab::Generation);
        assert_eq!(LodTab::Statistics, LodTab::Statistics);
    }

    #[test]
    fn test_lod_tab_copy() {
        let tab = LodTab::Generation;
        let copied = tab;
        assert_eq!(tab, copied);
    }

    // ============================================================================
    // LOD CONFIG PANEL TESTS
    // ============================================================================

    #[test]
    fn test_lod_panel_creation() {
        let mut panel = LodConfigPanel::new();
        panel.create_sample_data();
        assert!(panel.group_count() >= 3);
    }

    #[test]
    fn test_lod_panel_default() {
        let panel = LodConfigPanel::default();
        assert_eq!(panel.active_tab, LodTab::Groups);
        assert!(panel.show_lod_colors);
        assert_eq!(panel.preview_distance, 0.0);
    }

    #[test]
    fn test_default_levels() {
        let mut panel = LodConfigPanel::new();
        panel.create_sample_data();
        assert!(panel.level_count() >= 3);
    }

    #[test]
    fn test_add_lod_group() {
        let mut panel = LodConfigPanel::new();
        let initial = panel.group_count();
        let id = panel.add_lod_group("Test Group");
        assert!(id > 0);
        assert_eq!(panel.group_count(), initial + 1);
    }

    #[test]
    fn test_add_multiple_lod_groups() {
        let mut panel = LodConfigPanel::new();
        let initial = panel.group_count();
        let id1 = panel.add_lod_group("Group 1");
        let id2 = panel.add_lod_group("Group 2");
        assert_ne!(id1, id2);
        assert_eq!(panel.group_count(), initial + 2);
    }

    #[test]
    fn test_bias_mode() {
        let mut panel = LodConfigPanel::new();
        panel.set_bias_mode(LodBiasMode::Performance);
        assert_eq!(panel.global_settings.bias_mode, LodBiasMode::Performance);
    }

    #[test]
    fn test_bias_mode_quality() {
        let mut panel = LodConfigPanel::new();
        panel.set_bias_mode(LodBiasMode::Quality);
        assert_eq!(panel.global_settings.bias_mode, LodBiasMode::Quality);
    }

    #[test]
    fn test_default_settings() {
        let settings = GlobalLodSettings::default();
        assert_eq!(settings.bias_mode, LodBiasMode::Balanced);
        assert!(settings.maximum_lod_level >= 4);
    }

    #[test]
    fn test_generation_settings() {
        let settings = LodGenerationSettings::default();
        assert!(settings.auto_generate);
        assert_eq!(settings.num_levels, 4);
    }

    #[test]
    fn test_panel_trait() {
        let panel = LodConfigPanel::new();
        assert_eq!(panel.name(), "LOD Configuration");
    }

    #[test]
    fn test_sample_data_character() {
        let mut panel = LodConfigPanel::new();
        panel.create_sample_data();
        let character = panel
            .lod_groups
            .iter()
            .find(|g| g.name == "Player Character");
        assert!(character.is_some());
        let char_group = character.unwrap();
        assert_eq!(char_group.base_triangles, 25000);
        assert_eq!(char_group.levels.len(), 4);
    }

    #[test]
    fn test_sample_data_tree() {
        let mut panel = LodConfigPanel::new();
        panel.create_sample_data();
        let tree = panel.lod_groups.iter().find(|g| g.name == "Pine Tree");
        assert!(tree.is_some());
        let tree_group = tree.unwrap();
        assert_eq!(tree_group.base_triangles, 8000);
        assert_eq!(tree_group.cull_distance, 500.0);
    }

    #[test]
    fn test_sample_data_building() {
        let mut panel = LodConfigPanel::new();
        panel.create_sample_data();
        let building = panel.lod_groups.iter().find(|g| g.name == "House Medium");
        assert!(building.is_some());
        let build_group = building.unwrap();
        assert_eq!(build_group.base_triangles, 15000);
    }

    #[test]
    fn test_selected_group_initially_set() {
        let mut panel = LodConfigPanel::new();
        panel.create_sample_data();
        assert!(panel.selected_group.is_some());
    }

    #[test]
    fn test_next_group_id_increments() {
        let mut panel = LodConfigPanel::new();
        let id1 = panel.add_lod_group("A");
        let id2 = panel.add_lod_group("B");
        assert!(id2 > id1);
    }

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    #[test]
    fn test_panel_with_settings() {
        let mut panel = LodConfigPanel::new();
        panel.set_bias_mode(LodBiasMode::Custom);
        panel.global_settings.custom_bias = 2.5;
        assert_eq!(panel.global_settings.bias_mode, LodBiasMode::Custom);
        assert_eq!(panel.global_settings.custom_bias, 2.5);
    }

    #[test]
    fn test_lod_level_chain() {
        let mut panel = LodConfigPanel::new();
        panel.create_sample_data();
        let char_group = panel
            .lod_groups
            .iter()
            .find(|g| g.name == "Player Character")
            .unwrap();

        // Verify LOD levels have increasing distances
        for i in 1..char_group.levels.len() {
            assert!(char_group.levels[i].distance >= char_group.levels[i - 1].distance);
        }
    }

    #[test]
    fn test_lod_level_triangle_reduction() {
        let mut panel = LodConfigPanel::new();
        panel.create_sample_data();
        let char_group = panel
            .lod_groups
            .iter()
            .find(|g| g.name == "Player Character")
            .unwrap();

        // Verify triangle counts decrease
        for i in 1..char_group.levels.len() {
            assert!(char_group.levels[i].triangle_count <= char_group.levels[i - 1].triangle_count);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // LodConfigAction Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_lod_config_action_display() {
        let action = LodConfigAction::SetActiveTab(LodTab::Groups);
        let display = format!("{}", action);
        assert!(display.contains("tab"));
    }

    #[test]
    fn test_lod_config_action_display_all_variants() {
        let actions = vec![
            LodConfigAction::SetActiveTab(LodTab::Global),
            LodConfigAction::AddGroup,
            LodConfigAction::RemoveGroup(0),
            LodConfigAction::SelectGroup(1),
            LodConfigAction::AddLevel(0),
            LodConfigAction::SetGlobalBias(0.5),
            LodConfigAction::GenerateLods(0),
            LodConfigAction::SetPreviewDistance(50.0),
        ];

        for action in actions {
            let display = format!("{}", action);
            assert!(
                !display.is_empty(),
                "Display should not be empty for {:?}",
                action
            );
        }
    }

    #[test]
    fn test_lod_config_action_equality() {
        let action1 = LodConfigAction::SelectGroup(5);
        let action2 = LodConfigAction::SelectGroup(5);
        let action3 = LodConfigAction::SelectGroup(10);

        assert_eq!(action1, action2);
        assert_ne!(action1, action3);
    }

    #[test]
    fn test_lod_config_action_clone() {
        let action = LodConfigAction::SetGroupName(0, "TestGroup".to_string());
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_lod_config_panel_pending_actions_empty_by_default() {
        let panel = LodConfigPanel::new();
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_lod_config_panel_queue_action() {
        let mut panel = LodConfigPanel::new();
        panel.queue_action(LodConfigAction::AddGroup);
        assert!(panel.has_pending_actions());
        assert_eq!(panel.pending_actions().len(), 1);
    }

    #[test]
    fn test_lod_config_panel_take_actions() {
        let mut panel = LodConfigPanel::new();
        panel.queue_action(LodConfigAction::AddGroup);
        panel.queue_action(LodConfigAction::SetActiveTab(LodTab::Global));

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_lod_config_panel_action_order_preserved() {
        let mut panel = LodConfigPanel::new();
        panel.queue_action(LodConfigAction::AddGroup);
        panel.queue_action(LodConfigAction::SelectGroup(0));
        panel.queue_action(LodConfigAction::RemoveGroup(0));

        let actions = panel.take_actions();
        assert!(matches!(actions[0], LodConfigAction::AddGroup));
        assert!(matches!(actions[1], LodConfigAction::SelectGroup(_)));
        assert!(matches!(actions[2], LodConfigAction::RemoveGroup(_)));
    }

    #[test]
    fn test_lod_config_action_group_operations() {
        let actions = vec![
            LodConfigAction::AddGroup,
            LodConfigAction::RemoveGroup(0),
            LodConfigAction::SelectGroup(1),
            LodConfigAction::DuplicateGroup(2),
            LodConfigAction::SetGroupName(0, "NewGroup".to_string()),
            LodConfigAction::SetGroupAssetPath(0, "/assets/mesh.fbx".to_string()),
            LodConfigAction::ToggleGroupEnabled(0, true),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_lod_config_action_level_operations() {
        let actions = vec![
            LodConfigAction::AddLevel(0),
            LodConfigAction::RemoveLevel(0, 1),
            LodConfigAction::SetLevelDistance(0, 1, 50.0),
            LodConfigAction::SetLevelScreenCoverage(0, 1, 0.25),
            LodConfigAction::SetLevelMesh(0, 1, "mesh_lod1".to_string()),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_lod_config_action_transition_settings() {
        let actions = vec![
            LodConfigAction::SetFadeMode(0, FadeMode::CrossFade),
            LodConfigAction::SetFadeWidth(0, 5.0),
            LodConfigAction::ToggleCrossFade(0, true),
            LodConfigAction::SetCullDistance(0, 1000.0),
            LodConfigAction::SetShadowLodOffset(0, 2),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("CrossFade"));
        assert!(displays[3].contains("cull"));
    }

    #[test]
    fn test_lod_config_action_global_settings() {
        let actions = vec![
            LodConfigAction::SetBiasMode(LodBiasMode::Quality),
            LodConfigAction::SetGlobalBias(1.5),
            LodConfigAction::SetMaxLodLevel(4),
            LodConfigAction::ToggleAutoLod(true),
            LodConfigAction::SetScreenSizeThreshold(0.05),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Quality"));
        assert!(displays[1].contains("1.5"));
    }

    #[test]
    fn test_lod_config_action_generation() {
        let actions = vec![
            LodConfigAction::SetReductionMethod(ReductionMethod::QuadricErrorMetric),
            LodConfigAction::SetTargetTriangles(0, 50.0),
            LodConfigAction::GenerateLods(0),
            LodConfigAction::GenerateAllLods,
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Quadric"));
        assert!(displays[2].contains("Generate"));
    }

    #[test]
    fn test_lod_config_action_preview_and_stats() {
        let actions = vec![
            LodConfigAction::SetPreviewDistance(100.0),
            LodConfigAction::ToggleLodColors(true),
            LodConfigAction::RefreshStatistics,
            LodConfigAction::ExportReport,
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }
}
