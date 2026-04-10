//! Ready Asset System - Curated Asset Store with Production-Ready Prefabs
//!
//! Provides a metadata and validation system for "drop and play" assets:
//! - **Full Collider Setup**: Automatic collision mesh validation
//! - **Correct Materials**: Material assignments verified and complete
//! - **LOD System**: Multiple LOD levels with proper transitions
//! - **Occlusion/Shadow Flags**: Proper visibility culling configured
//! - **Baked Lightmap UVs**: UV2 present for lightmapping
//! - **Metadata Tags**: Searchable tags for asset discovery
//!
//! Assets meeting all criteria receive the "Ready" badge and appear
//! in the curated asset store for immediate use.

use std::collections::HashSet;
use std::path::PathBuf;

use egui::{Color32, RichText, Ui};
use tracing::info;

use crate::panels::Panel;

// ============================================================================
// PANEL ACTIONS - Events produced by the panel for external handling
// ============================================================================

/// Actions emitted by the ready asset store panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AssetStoreAction {
    /// Spawn the asset in the current scene
    SpawnAsset { asset_id: u64, asset_path: PathBuf },
    /// Request to preview the asset in 3D viewer
    PreviewAsset { asset_id: u64, asset_path: PathBuf },
    /// Filter by tag
    FilterByTag { tag: String },
    /// Request to refresh the asset list
    RefreshAssets,
    /// Request to validate an asset's checklist
    ValidateAsset { asset_id: u64, asset_path: PathBuf },
}

// ============================================================================
// ASSET READINESS LEVEL - How ready is this asset for production use
// ============================================================================

/// Readiness level for prefab assets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ReadinessLevel {
    #[default]
    NotReady,
    Basic,
    Standard,
    Production,
    Premium,
}

impl std::fmt::Display for ReadinessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ReadinessLevel {
    /// All readiness levels
    pub fn all() -> &'static [ReadinessLevel] {
        &[
            ReadinessLevel::NotReady,
            ReadinessLevel::Basic,
            ReadinessLevel::Standard,
            ReadinessLevel::Production,
            ReadinessLevel::Premium,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            ReadinessLevel::NotReady => "Not Ready",
            ReadinessLevel::Basic => "Basic",
            ReadinessLevel::Standard => "Standard",
            ReadinessLevel::Production => "Production",
            ReadinessLevel::Premium => "Premium",
        }
    }

    /// Icon/badge
    pub fn icon(&self) -> &'static str {
        match self {
            ReadinessLevel::NotReady => "[!]",
            ReadinessLevel::Basic => "[B]",
            ReadinessLevel::Standard => "[G]",
            ReadinessLevel::Production => "[Star]",
            ReadinessLevel::Premium => "[Gem]",
        }
    }

    /// Color for UI
    pub fn color(&self) -> Color32 {
        match self {
            ReadinessLevel::NotReady => Color32::from_rgb(180, 80, 80),
            ReadinessLevel::Basic => Color32::from_rgb(100, 150, 255),
            ReadinessLevel::Standard => Color32::from_rgb(100, 200, 100),
            ReadinessLevel::Production => Color32::from_rgb(255, 200, 100),
            ReadinessLevel::Premium => Color32::from_rgb(200, 150, 255),
        }
    }

    /// Minimum checklist items required
    pub fn min_requirements(&self) -> u32 {
        match self {
            ReadinessLevel::NotReady => 0,
            ReadinessLevel::Basic => 3,
            ReadinessLevel::Standard => 5,
            ReadinessLevel::Production => 7,
            ReadinessLevel::Premium => 9,
        }
    }

    /// Description of this level
    pub fn description(&self) -> &'static str {
        match self {
            ReadinessLevel::NotReady => "Asset needs work before use",
            ReadinessLevel::Basic => "Mesh and basic material only",
            ReadinessLevel::Standard => "Includes LODs and colliders",
            ReadinessLevel::Production => "Fully game-ready with all features",
            ReadinessLevel::Premium => "Hand-crafted with extras (variants, animations)",
        }
    }

    /// Calculate from checklist
    pub fn from_checklist(checklist: &AssetChecklist) -> Self {
        let passed = checklist.passed_count();
        if passed >= 9 {
            ReadinessLevel::Premium
        } else if passed >= 7 {
            ReadinessLevel::Production
        } else if passed >= 5 {
            ReadinessLevel::Standard
        } else if passed >= 3 {
            ReadinessLevel::Basic
        } else {
            ReadinessLevel::NotReady
        }
    }
}

// ============================================================================
// CHECKLIST ITEM - Individual quality check
// ============================================================================

/// Individual checklist items for asset readiness
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChecklistItem {
    HasMesh,
    HasMaterial,
    HasCollider,
    HasLODs,
    HasLightmapUVs,
    HasOcclusionFlags,
    HasShadowFlags,
    HasTags,
    HasThumbnail,
    HasVariants,
    HasAnimations,
    PowerOfTwoTextures,
    MaterialsComplete,
    ScaleCorrect,
}

impl std::fmt::Display for ChecklistItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ChecklistItem {
    /// All checklist items
    pub fn all() -> &'static [ChecklistItem] {
        &[
            ChecklistItem::HasMesh,
            ChecklistItem::HasMaterial,
            ChecklistItem::HasCollider,
            ChecklistItem::HasLODs,
            ChecklistItem::HasLightmapUVs,
            ChecklistItem::HasOcclusionFlags,
            ChecklistItem::HasShadowFlags,
            ChecklistItem::HasTags,
            ChecklistItem::HasThumbnail,
            ChecklistItem::HasVariants,
            ChecklistItem::HasAnimations,
            ChecklistItem::PowerOfTwoTextures,
            ChecklistItem::MaterialsComplete,
            ChecklistItem::ScaleCorrect,
        ]
    }

    /// Core items required for basic level
    pub fn core_items() -> &'static [ChecklistItem] {
        &[
            ChecklistItem::HasMesh,
            ChecklistItem::HasMaterial,
            ChecklistItem::HasCollider,
            ChecklistItem::HasLODs,
            ChecklistItem::HasLightmapUVs,
            ChecklistItem::HasOcclusionFlags,
            ChecklistItem::HasShadowFlags,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            ChecklistItem::HasMesh => "Valid Mesh",
            ChecklistItem::HasMaterial => "Material Assigned",
            ChecklistItem::HasCollider => "Collider Setup",
            ChecklistItem::HasLODs => "LOD Levels",
            ChecklistItem::HasLightmapUVs => "Lightmap UVs",
            ChecklistItem::HasOcclusionFlags => "Occlusion Flags",
            ChecklistItem::HasShadowFlags => "Shadow Flags",
            ChecklistItem::HasTags => "Metadata Tags",
            ChecklistItem::HasThumbnail => "Thumbnail",
            ChecklistItem::HasVariants => "Variants",
            ChecklistItem::HasAnimations => "Animations",
            ChecklistItem::PowerOfTwoTextures => "Power-of-Two Textures",
            ChecklistItem::MaterialsComplete => "Materials Complete",
            ChecklistItem::ScaleCorrect => "Correct Scale",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            ChecklistItem::HasMesh => "[Mask]",
            ChecklistItem::HasMaterial => "[Gem]",
            ChecklistItem::HasCollider => "[Hit]",
            ChecklistItem::HasLODs => "👁️",
            ChecklistItem::HasLightmapUVs => "🗺️",
            ChecklistItem::HasOcclusionFlags => "👻",
            ChecklistItem::HasShadowFlags => "🌑",
            ChecklistItem::HasTags => "[Tag]",
            ChecklistItem::HasThumbnail => "[Img]",
            ChecklistItem::HasVariants => "[Art]",
            ChecklistItem::HasAnimations => "[Anim]",
            ChecklistItem::PowerOfTwoTextures => "[Sq]",
            ChecklistItem::MaterialsComplete => "[ok]",
            ChecklistItem::ScaleCorrect => "📏",
        }
    }

    /// Whether this is a core requirement (vs optional extra)
    pub fn is_core(&self) -> bool {
        Self::core_items().contains(self)
    }

    /// Detailed description
    pub fn description(&self) -> &'static str {
        match self {
            ChecklistItem::HasMesh => "Asset contains valid renderable geometry",
            ChecklistItem::HasMaterial => "At least one material is properly assigned",
            ChecklistItem::HasCollider => "Physics collider is configured (convex or mesh)",
            ChecklistItem::HasLODs => "Multiple LOD levels for distance culling",
            ChecklistItem::HasLightmapUVs => "UV2 channel exists for lightmap baking",
            ChecklistItem::HasOcclusionFlags => "Occlusion culling flags are set",
            ChecklistItem::HasShadowFlags => "Shadow casting/receiving flags configured",
            ChecklistItem::HasTags => "Searchable metadata tags assigned",
            ChecklistItem::HasThumbnail => "Preview thumbnail image exists",
            ChecklistItem::HasVariants => "Material or mesh variants included",
            ChecklistItem::HasAnimations => "Animation clips are included",
            ChecklistItem::PowerOfTwoTextures => "All textures are power-of-two dimensions",
            ChecklistItem::MaterialsComplete => "All material slots have textures assigned",
            ChecklistItem::ScaleCorrect => {
                "Model is at correct real-world scale (1 unit = 1 meter)"
            }
        }
    }
}

// ============================================================================
// ASSET CHECKLIST - Complete checklist for an asset
// ============================================================================

/// Complete checklist state for an asset
#[derive(Debug, Clone, Default)]
pub struct AssetChecklist {
    pub passed: HashSet<ChecklistItem>,
    pub failed: HashSet<ChecklistItem>,
    pub warnings: Vec<String>,
}

impl AssetChecklist {
    /// Create a new empty checklist
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark an item as passed
    pub fn pass(&mut self, item: ChecklistItem) {
        self.failed.remove(&item);
        self.passed.insert(item);
    }

    /// Mark an item as failed
    pub fn fail(&mut self, item: ChecklistItem) {
        self.passed.remove(&item);
        self.failed.insert(item);
    }

    /// Count passed items
    pub fn passed_count(&self) -> u32 {
        self.passed.len() as u32
    }

    /// Count failed items
    pub fn failed_count(&self) -> u32 {
        self.failed.len() as u32
    }

    /// Count core items passed
    pub fn core_passed_count(&self) -> u32 {
        ChecklistItem::core_items()
            .iter()
            .filter(|item| self.passed.contains(item))
            .count() as u32
    }

    /// Check if an item passed
    pub fn is_passed(&self, item: ChecklistItem) -> bool {
        self.passed.contains(&item)
    }

    /// Get readiness level
    pub fn readiness(&self) -> ReadinessLevel {
        ReadinessLevel::from_checklist(self)
    }

    /// Percentage complete
    pub fn completion_percentage(&self) -> f32 {
        let total = ChecklistItem::all().len() as f32;
        (self.passed.len() as f32 / total) * 100.0
    }
}

// ============================================================================
// ASSET CATEGORY - High-level asset organization
// ============================================================================

/// Asset category for the curated store
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AssetStoreCategory {
    #[default]
    All,
    Environment,
    Props,
    Characters,
    Vehicles,
    Weapons,
    Architecture,
    Nature,
    Industrial,
    SciFi,
    Fantasy,
    Effects,
    UI,
}

impl std::fmt::Display for AssetStoreCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AssetStoreCategory {
    /// All categories
    pub fn all() -> &'static [AssetStoreCategory] {
        &[
            AssetStoreCategory::All,
            AssetStoreCategory::Environment,
            AssetStoreCategory::Props,
            AssetStoreCategory::Characters,
            AssetStoreCategory::Vehicles,
            AssetStoreCategory::Weapons,
            AssetStoreCategory::Architecture,
            AssetStoreCategory::Nature,
            AssetStoreCategory::Industrial,
            AssetStoreCategory::SciFi,
            AssetStoreCategory::Fantasy,
            AssetStoreCategory::Effects,
            AssetStoreCategory::UI,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            AssetStoreCategory::All => "All",
            AssetStoreCategory::Environment => "Environment",
            AssetStoreCategory::Props => "Props",
            AssetStoreCategory::Characters => "Characters",
            AssetStoreCategory::Vehicles => "Vehicles",
            AssetStoreCategory::Weapons => "Weapons",
            AssetStoreCategory::Architecture => "Architecture",
            AssetStoreCategory::Nature => "Nature",
            AssetStoreCategory::Industrial => "Industrial",
            AssetStoreCategory::SciFi => "Sci-Fi",
            AssetStoreCategory::Fantasy => "Fantasy",
            AssetStoreCategory::Effects => "Effects",
            AssetStoreCategory::UI => "UI",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            AssetStoreCategory::All => "[Pkg]",
            AssetStoreCategory::Environment => "[Glb]",
            AssetStoreCategory::Props => "🪑",
            AssetStoreCategory::Characters => "👤",
            AssetStoreCategory::Vehicles => "🚗",
            AssetStoreCategory::Weapons => "⚔️",
            AssetStoreCategory::Architecture => "🏛️",
            AssetStoreCategory::Nature => "🌲",
            AssetStoreCategory::Industrial => "🏭",
            AssetStoreCategory::SciFi => "[Rkt]",
            AssetStoreCategory::Fantasy => "[Orb]",
            AssetStoreCategory::Effects => "[Fx]",
            AssetStoreCategory::UI => "[Img]",
        }
    }

    /// Related tags for this category
    pub fn related_tags(&self) -> &'static [&'static str] {
        match self {
            AssetStoreCategory::All => &[],
            AssetStoreCategory::Environment => &["terrain", "skybox", "landscape", "world"],
            AssetStoreCategory::Props => &["furniture", "decoration", "object", "item"],
            AssetStoreCategory::Characters => &["npc", "player", "humanoid", "creature"],
            AssetStoreCategory::Vehicles => &["car", "ship", "aircraft", "transport"],
            AssetStoreCategory::Weapons => &["sword", "gun", "melee", "ranged"],
            AssetStoreCategory::Architecture => &["building", "structure", "room", "interior"],
            AssetStoreCategory::Nature => &["tree", "rock", "plant", "foliage", "grass"],
            AssetStoreCategory::Industrial => &["factory", "machine", "pipe", "metal"],
            AssetStoreCategory::SciFi => &["futuristic", "space", "tech", "cyberpunk"],
            AssetStoreCategory::Fantasy => &["medieval", "magic", "dungeon", "castle"],
            AssetStoreCategory::Effects => &["particle", "vfx", "shader", "decal"],
            AssetStoreCategory::UI => &["button", "panel", "icon", "hud"],
        }
    }
}

// ============================================================================
// READY ASSET - A curated asset entry
// ============================================================================

/// A curated, production-ready asset
#[derive(Debug, Clone)]
pub struct ReadyAsset {
    pub id: u64,
    pub name: String,
    pub path: PathBuf,
    pub category: AssetStoreCategory,
    pub tags: Vec<String>,
    pub checklist: AssetChecklist,
    pub thumbnail_path: Option<PathBuf>,

    // Mesh info
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub lod_count: u32,

    // Material info
    pub material_count: u32,
    pub texture_count: u32,

    // Metadata
    pub author: String,
    pub license: String,
    pub created_date: String,
    pub file_size_bytes: u64,
}

impl ReadyAsset {
    /// Get readiness level
    pub fn readiness(&self) -> ReadinessLevel {
        self.checklist.readiness()
    }

    /// Check if asset meets minimum readiness
    pub fn meets_minimum(&self, level: ReadinessLevel) -> bool {
        self.readiness() >= level
    }

    /// Format file size for display
    pub fn formatted_size(&self) -> String {
        if self.file_size_bytes < 1024 {
            format!("{} B", self.file_size_bytes)
        } else if self.file_size_bytes < 1024 * 1024 {
            format!("{:.1} KB", self.file_size_bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", self.file_size_bytes as f64 / (1024.0 * 1024.0))
        }
    }

    /// Check if asset matches search query
    pub fn matches_search(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.name.to_lowercase().contains(&query_lower)
            || self
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&query_lower))
    }

    /// Check if asset matches category filter
    pub fn matches_category(&self, category: AssetStoreCategory) -> bool {
        category == AssetStoreCategory::All || self.category == category
    }
}

// ============================================================================
// READY ASSET STORE PANEL
// ============================================================================

/// Curated asset store panel
#[derive(Debug)]
pub struct ReadyAssetStorePanel {
    pub assets: Vec<ReadyAsset>,
    pub selected_category: AssetStoreCategory,
    pub min_readiness: ReadinessLevel,
    pub search_query: String,
    pub selected_asset: Option<usize>,
    pub show_details: bool,
    pub sort_by: SortBy,
    pub grid_view: bool,
    /// Actions queued for external processing
    pending_actions: Vec<AssetStoreAction>,
}

/// Sort options for asset list
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SortBy {
    #[default]
    Name,
    Readiness,
    Category,
    Size,
    Date,
}

impl std::fmt::Display for SortBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl SortBy {
    pub fn all() -> &'static [SortBy] {
        &[
            SortBy::Name,
            SortBy::Readiness,
            SortBy::Category,
            SortBy::Size,
            SortBy::Date,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SortBy::Name => "Name",
            SortBy::Readiness => "Readiness",
            SortBy::Category => "Category",
            SortBy::Size => "Size",
            SortBy::Date => "Date",
        }
    }
}

impl Default for ReadyAssetStorePanel {
    fn default() -> Self {
        Self {
            assets: Vec::new(),
            selected_category: AssetStoreCategory::All,
            min_readiness: ReadinessLevel::Standard,
            search_query: String::new(),
            selected_asset: None,
            show_details: true,
            sort_by: SortBy::Name,
            grid_view: true,
            pending_actions: Vec::new(),
        }
    }
}

impl ReadyAssetStorePanel {
    /// Create a new ready asset store panel
    pub fn new() -> Self {
        Self::default()
    }

    /// Take all pending actions (drains the queue)
    pub fn take_actions(&mut self) -> Vec<AssetStoreAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an action for later processing
    fn queue_action(&mut self, action: AssetStoreAction) {
        self.pending_actions.push(action);
    }

    /// Get the currently selected asset, if any
    pub fn selected_asset(&self) -> Option<&ReadyAsset> {
        self.selected_asset.and_then(|idx| self.assets.get(idx))
    }

    /// Get filtered assets based on current settings
    pub fn filtered_assets(&self) -> Vec<&ReadyAsset> {
        self.assets
            .iter()
            .filter(|a| a.readiness() >= self.min_readiness)
            .filter(|a| a.matches_category(self.selected_category))
            .filter(|a| self.search_query.is_empty() || a.matches_search(&self.search_query))
            .collect()
    }

    fn render_filters(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("[Srch]");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search assets...")
                    .desired_width(150.0),
            );

            if !self.search_query.is_empty() && ui.small_button("✕").clicked() {
                self.search_query.clear();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Category:");
            egui::ComboBox::from_id_salt("category_filter")
                .selected_text(format!("{}", self.selected_category))
                .show_ui(ui, |ui| {
                    for cat in AssetStoreCategory::all() {
                        if ui
                            .selectable_label(self.selected_category == *cat, format!("{}", cat))
                            .clicked()
                        {
                            self.selected_category = *cat;
                        }
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Min Readiness:");
            egui::ComboBox::from_id_salt("readiness_filter")
                .selected_text(format!("{}", self.min_readiness))
                .show_ui(ui, |ui| {
                    for level in ReadinessLevel::all() {
                        if ui
                            .selectable_label(self.min_readiness == *level, format!("{}", level))
                            .clicked()
                        {
                            self.min_readiness = *level;
                        }
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Sort:");
            egui::ComboBox::from_id_salt("sort_by")
                .selected_text(format!("{}", self.sort_by))
                .show_ui(ui, |ui| {
                    for sort in SortBy::all() {
                        if ui
                            .selectable_label(self.sort_by == *sort, format!("{}", sort))
                            .clicked()
                        {
                            self.sort_by = *sort;
                        }
                    }
                });

            ui.separator();

            if ui.selectable_label(self.grid_view, "▦ Grid").clicked() {
                self.grid_view = true;
            }
            if ui.selectable_label(!self.grid_view, "☰ List").clicked() {
                self.grid_view = false;
            }
        });
    }

    fn render_asset_grid(&mut self, ui: &mut Ui) {
        // Collect data needed for rendering first to avoid borrowing issues
        let filtered_data: Vec<_> = self
            .filtered_assets()
            .iter()
            .enumerate()
            .map(|(idx, asset)| (idx, asset.name.clone(), asset.category, asset.readiness()))
            .collect();

        if filtered_data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No assets match your filters");
            });
            return;
        }

        let current_selected = self.selected_asset;
        let mut clicked_idx: Option<usize> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            let available_width = ui.available_width();
            let item_size = 120.0;
            let columns = ((available_width / item_size).floor() as usize).max(1);

            egui::Grid::new("asset_grid")
                .num_columns(columns)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (idx, name, category, readiness) in filtered_data.iter() {
                        let selected = current_selected == Some(*idx);

                        ui.vertical(|ui| {
                            // Thumbnail area
                            let (rect, response) = ui.allocate_exact_size(
                                egui::Vec2::new(100.0, 100.0),
                                egui::Sense::click(),
                            );

                            let bg_color = if selected {
                                Color32::from_rgb(60, 80, 100)
                            } else if response.hovered() {
                                Color32::from_rgb(50, 55, 60)
                            } else {
                                Color32::from_rgb(40, 45, 50)
                            };

                            ui.painter().rect_filled(rect, 6.0, bg_color);

                            // Category icon as placeholder
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                category.icon(),
                                egui::FontId::proportional(32.0),
                                Color32::WHITE,
                            );

                            // Readiness badge
                            let badge_pos = rect.right_top() + egui::Vec2::new(-20.0, 5.0);
                            ui.painter().text(
                                badge_pos,
                                egui::Align2::CENTER_CENTER,
                                readiness.icon(),
                                egui::FontId::proportional(14.0),
                                readiness.color(),
                            );

                            if response.clicked() {
                                clicked_idx = Some(*idx);
                            }

                            // Name
                            ui.add(
                                egui::Label::new(RichText::new(name).small().color(Color32::WHITE))
                                    .wrap_mode(egui::TextWrapMode::Truncate),
                            );
                        });

                        if (*idx + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

        if let Some(idx) = clicked_idx {
            self.selected_asset = Some(idx);
        }
    }

    fn render_asset_list(&mut self, ui: &mut Ui) {
        // Collect data needed for rendering first to avoid borrowing issues
        let filtered_data: Vec<_> = self
            .filtered_assets()
            .iter()
            .enumerate()
            .map(|(idx, asset)| {
                (
                    idx,
                    asset.name.clone(),
                    asset.category,
                    asset.readiness(),
                    asset.triangle_count,
                    asset.formatted_size(),
                )
            })
            .collect();

        if filtered_data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No assets match your filters");
            });
            return;
        }

        let current_selected = self.selected_asset;
        let mut clicked_idx: Option<usize> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, name, category, readiness, tris, size) in filtered_data.iter() {
                let selected = current_selected == Some(*idx);

                let response = ui.selectable_label(
                    selected,
                    format!(
                        "{} {} {} • {} tris • {}",
                        readiness.icon(),
                        category.icon(),
                        name,
                        tris,
                        size
                    ),
                );

                if response.clicked() {
                    clicked_idx = Some(*idx);
                }
            }
        });

        if let Some(idx) = clicked_idx {
            self.selected_asset = Some(idx);
        }
    }

    fn render_asset_details(&mut self, ui: &mut Ui) {
        let Some(idx) = self.selected_asset else {
            ui.centered_and_justified(|ui| {
                ui.label("Select an asset to view details");
            });
            return;
        };

        let asset = {
            let filtered = self.filtered_assets();
            let Some(a) = filtered.get(idx) else {
                return;
            };
            (*a).clone()
        };

        // Flags for deferred actions (cannot mutate self inside closures)
        let mut should_spawn = false;
        let mut should_preview = false;
        let asset_id = asset.id;
        let asset_path = asset.path.clone();

        ui.heading(format!("{} {}", asset.category.icon(), asset.name));

        ui.horizontal(|ui| {
            let readiness = asset.readiness();
            ui.colored_label(readiness.color(), format!("{}", readiness));
        });

        ui.separator();

        // Stats
        egui::Grid::new("asset_details_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Vertices:");
                ui.strong(format!("{}", asset.vertex_count));
                ui.end_row();

                ui.label("Triangles:");
                ui.strong(format!("{}", asset.triangle_count));
                ui.end_row();

                ui.label("LODs:");
                ui.strong(format!("{}", asset.lod_count));
                ui.end_row();

                ui.label("Materials:");
                ui.strong(format!("{}", asset.material_count));
                ui.end_row();

                ui.label("Textures:");
                ui.strong(format!("{}", asset.texture_count));
                ui.end_row();

                ui.label("Size:");
                ui.strong(asset.formatted_size());
                ui.end_row();
            });

        ui.separator();

        // Tags
        ui.label("Tags:");
        let mut clicked_tag = None;
        ui.horizontal_wrapped(|ui| {
            for tag in &asset.tags {
                if ui.small_button(format!("#{}", tag)).clicked() {
                    clicked_tag = Some(tag.clone());
                }
            }
        });
        if let Some(tag) = clicked_tag {
            self.queue_action(AssetStoreAction::FilterByTag { tag });
        }

        ui.separator();

        // Checklist
        egui::CollapsingHeader::new("Checklist")
            .default_open(false)
            .show(ui, |ui| {
                for item in ChecklistItem::core_items() {
                    let passed = asset.checklist.is_passed(*item);
                    let icon = if passed { "[ok]" } else { "[x]" };
                    ui.label(format!("{} {}", icon, item.name()));
                }
            });

        ui.separator();

        // Actions
        ui.horizontal(|ui| {
            if ui.button("Add to Scene").clicked() {
                should_spawn = true;
            }
            if ui.button("Preview").clicked() {
                should_preview = true;
            }
        });

        // Queue actions outside of nested UI context
        if should_spawn {
            info!(asset_id = asset_id, "asset_store: spawn asset");
            self.queue_action(AssetStoreAction::SpawnAsset {
                asset_id,
                asset_path: asset_path.clone(),
            });
        }
        if should_preview {
            info!(asset_id = asset_id, "asset_store: preview asset");
            self.queue_action(AssetStoreAction::PreviewAsset {
                asset_id,
                asset_path,
            });
        }
    }
}

impl Panel for ReadyAssetStorePanel {
    fn name(&self) -> &'static str {
        "Ready Assets"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.render_filters(ui);
        ui.separator();

        ui.columns(2, |columns| {
            // Left: Asset list/grid
            if self.grid_view {
                self.render_asset_grid(&mut columns[0]);
            } else {
                self.render_asset_list(&mut columns[0]);
            }

            // Right: Details
            if self.show_details {
                self.render_asset_details(&mut columns[1]);
            }
        });
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readiness_level_display() {
        assert!(format!("{}", ReadinessLevel::Production).contains("Production"));
        assert!(format!("{}", ReadinessLevel::Premium).contains("Premium"));
    }

    #[test]
    fn test_readiness_level_all() {
        let levels = ReadinessLevel::all();
        assert_eq!(levels.len(), 5);
    }

    #[test]
    fn test_readiness_level_ordering() {
        assert!(ReadinessLevel::Premium > ReadinessLevel::Production);
        assert!(ReadinessLevel::Production > ReadinessLevel::Standard);
        assert!(ReadinessLevel::Standard > ReadinessLevel::Basic);
        assert!(ReadinessLevel::Basic > ReadinessLevel::NotReady);
    }

    #[test]
    fn test_readiness_from_checklist() {
        let mut checklist = AssetChecklist::new();

        // No items = NotReady
        assert_eq!(
            ReadinessLevel::from_checklist(&checklist),
            ReadinessLevel::NotReady
        );

        // 3 items = Basic
        checklist.pass(ChecklistItem::HasMesh);
        checklist.pass(ChecklistItem::HasMaterial);
        checklist.pass(ChecklistItem::HasCollider);
        assert_eq!(
            ReadinessLevel::from_checklist(&checklist),
            ReadinessLevel::Basic
        );

        // 5 items = Standard
        checklist.pass(ChecklistItem::HasLODs);
        checklist.pass(ChecklistItem::HasLightmapUVs);
        assert_eq!(
            ReadinessLevel::from_checklist(&checklist),
            ReadinessLevel::Standard
        );

        // 7 items = Production
        checklist.pass(ChecklistItem::HasOcclusionFlags);
        checklist.pass(ChecklistItem::HasShadowFlags);
        assert_eq!(
            ReadinessLevel::from_checklist(&checklist),
            ReadinessLevel::Production
        );

        // 9+ items = Premium
        checklist.pass(ChecklistItem::HasTags);
        checklist.pass(ChecklistItem::HasThumbnail);
        assert_eq!(
            ReadinessLevel::from_checklist(&checklist),
            ReadinessLevel::Premium
        );
    }

    #[test]
    fn test_checklist_item_display() {
        assert!(format!("{}", ChecklistItem::HasMesh).contains("Mesh"));
        assert!(format!("{}", ChecklistItem::HasCollider).contains("Collider"));
    }

    #[test]
    fn test_checklist_item_core() {
        assert!(ChecklistItem::HasMesh.is_core());
        assert!(ChecklistItem::HasCollider.is_core());
        assert!(!ChecklistItem::HasVariants.is_core());
        assert!(!ChecklistItem::HasAnimations.is_core());
    }

    #[test]
    fn test_asset_checklist() {
        let mut checklist = AssetChecklist::new();

        checklist.pass(ChecklistItem::HasMesh);
        checklist.pass(ChecklistItem::HasMaterial);
        checklist.fail(ChecklistItem::HasCollider);

        assert_eq!(checklist.passed_count(), 2);
        assert_eq!(checklist.failed_count(), 1);
        assert!(checklist.is_passed(ChecklistItem::HasMesh));
        assert!(!checklist.is_passed(ChecklistItem::HasCollider));
    }

    #[test]
    fn test_asset_store_category_display() {
        assert!(format!("{}", AssetStoreCategory::Nature).contains("Nature"));
        assert!(format!("{}", AssetStoreCategory::Weapons).contains("Weapons"));
    }

    #[test]
    fn test_asset_store_category_all() {
        let cats = AssetStoreCategory::all();
        assert_eq!(cats.len(), 13);
    }

    #[test]
    fn test_ready_asset_formatted_size() {
        let mut asset = ReadyAsset {
            id: 1,
            name: "Test".to_string(),
            path: PathBuf::new(),
            category: AssetStoreCategory::Props,
            tags: vec![],
            checklist: AssetChecklist::new(),
            thumbnail_path: None,
            vertex_count: 0,
            triangle_count: 0,
            lod_count: 0,
            material_count: 0,
            texture_count: 0,
            author: String::new(),
            license: String::new(),
            created_date: String::new(),
            file_size_bytes: 500,
        };

        assert!(asset.formatted_size().contains("B"));

        asset.file_size_bytes = 2048;
        assert!(asset.formatted_size().contains("KB"));

        asset.file_size_bytes = 2 * 1024 * 1024;
        assert!(asset.formatted_size().contains("MB"));
    }

    #[test]
    fn test_ready_asset_matches_search() {
        let asset = ReadyAsset {
            id: 1,
            name: "Pine Tree".to_string(),
            path: PathBuf::new(),
            category: AssetStoreCategory::Nature,
            tags: vec!["tree".to_string(), "forest".to_string()],
            checklist: AssetChecklist::new(),
            thumbnail_path: None,
            vertex_count: 0,
            triangle_count: 0,
            lod_count: 0,
            material_count: 0,
            texture_count: 0,
            author: String::new(),
            license: String::new(),
            created_date: String::new(),
            file_size_bytes: 0,
        };

        assert!(asset.matches_search("pine"));
        assert!(asset.matches_search("tree"));
        assert!(asset.matches_search("forest"));
        assert!(!asset.matches_search("rock"));
    }

    #[test]
    fn test_ready_asset_matches_category() {
        let asset = ReadyAsset {
            id: 1,
            name: "Test".to_string(),
            path: PathBuf::new(),
            category: AssetStoreCategory::Nature,
            tags: vec![],
            checklist: AssetChecklist::new(),
            thumbnail_path: None,
            vertex_count: 0,
            triangle_count: 0,
            lod_count: 0,
            material_count: 0,
            texture_count: 0,
            author: String::new(),
            license: String::new(),
            created_date: String::new(),
            file_size_bytes: 0,
        };

        assert!(asset.matches_category(AssetStoreCategory::All));
        assert!(asset.matches_category(AssetStoreCategory::Nature));
        assert!(!asset.matches_category(AssetStoreCategory::Weapons));
    }

    #[test]
    fn test_sort_by_display() {
        assert_eq!(SortBy::Name.name(), "Name");
        assert_eq!(SortBy::Readiness.name(), "Readiness");
    }

    #[test]
    fn test_panel_default() {
        let panel = ReadyAssetStorePanel::new();
        assert_eq!(panel.selected_category, AssetStoreCategory::All);
        assert_eq!(panel.min_readiness, ReadinessLevel::Standard);
        assert!(panel.grid_view);
    }

    // ==========================================================================
    // Action System Tests
    // ==========================================================================

    #[test]
    fn test_action_queue_initially_empty() {
        let panel = ReadyAssetStorePanel::new();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_take_actions_drains_queue() {
        let mut panel = ReadyAssetStorePanel::new();
        panel.pending_actions.push(AssetStoreAction::RefreshAssets);
        panel
            .pending_actions
            .push(AssetStoreAction::FilterByTag { tag: "tree".into() });

        assert!(panel.has_pending_actions());
        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_asset_store_action_variants() {
        use std::path::PathBuf;

        let spawn_action = AssetStoreAction::SpawnAsset {
            asset_id: 42,
            asset_path: PathBuf::from("assets/tree.prefab"),
        };
        assert!(matches!(spawn_action, AssetStoreAction::SpawnAsset { .. }));

        let preview_action = AssetStoreAction::PreviewAsset {
            asset_id: 42,
            asset_path: PathBuf::from("assets/tree.prefab"),
        };
        assert!(matches!(
            preview_action,
            AssetStoreAction::PreviewAsset { .. }
        ));

        let filter_action = AssetStoreAction::FilterByTag {
            tag: "nature".into(),
        };
        assert!(matches!(
            filter_action,
            AssetStoreAction::FilterByTag { .. }
        ));

        let validate_action = AssetStoreAction::ValidateAsset {
            asset_id: 1,
            asset_path: PathBuf::from("assets/rock.prefab"),
        };
        assert!(matches!(
            validate_action,
            AssetStoreAction::ValidateAsset { .. }
        ));
    }

    #[test]
    fn test_selected_asset_accessor() {
        let mut panel = ReadyAssetStorePanel::new();
        assert!(panel.selected_asset().is_none());

        // Add an asset and select it
        panel.assets.push(ReadyAsset {
            id: 1,
            name: "Test Asset".into(),
            path: PathBuf::from("test.prefab"),
            category: AssetStoreCategory::Nature,
            tags: vec!["test".into()],
            checklist: AssetChecklist::default(),
            thumbnail_path: None,
            vertex_count: 100,
            triangle_count: 50,
            lod_count: 3,
            material_count: 1,
            texture_count: 2,
            author: String::new(),
            license: String::new(),
            created_date: String::new(),
            file_size_bytes: 1024,
        });
        panel.selected_asset = Some(0);

        let selected = panel.selected_asset();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name, "Test Asset");
    }
}
