//! Blend Import Panel — Blender .blend scene import, decomposition, and biome pack generation
//!
//! Provides a complete workflow for importing .blend files:
//! - **File Selection**: Browse for .blend files or drag-and-drop
//! - **Decomposition Preview**: Show extracted assets with categories
//! - **Texture Processing**: Configure HDR→LDR conversion, thumbnail generation
//! - **Biome Pack Generation**: Convert decomposed scenes into terrain scatter packs
//! - **Asset Output**: Browse and manage extracted assets

use egui::{Color32, RichText, Ui, Vec2};
use std::path::PathBuf;

use crate::panels::Panel;

// ============================================================================
// PANEL ACTIONS
// ============================================================================

/// Actions emitted by the Blend Import panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BlendImportAction {
    /// Start decomposition of the selected .blend file
    StartDecomposition { blend_path: PathBuf },
    /// Cancel an in-progress decomposition
    CancelDecomposition,
    /// Generate a BiomePack from already-decomposed assets
    GenerateBiomePack {
        output_dir: PathBuf,
        pack_name: String,
    },
    /// Open the decomposed output directory in the asset browser
    BrowseOutputDir { path: PathBuf },
    /// Clear the current import session
    ClearSession,
    /// Create a blueprint zone from the generated biome pack for 1:1 scene replica
    CreateReplicaZone { pack_path: PathBuf },
}

// ============================================================================
// IMPORT PHASE — Tracks where the user is in the workflow
// ============================================================================

/// Which step of the import workflow the panel is on
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImportPhase {
    /// Waiting for the user to select a .blend file
    #[default]
    SelectFile,
    /// Decomposition is in progress
    Decomposing,
    /// Decomposition complete — reviewing results
    ReviewAssets,
    /// Biome pack generation in progress
    GeneratingPack,
    /// Everything finished
    Complete,
}

impl ImportPhase {
    pub fn label(&self) -> &'static str {
        match self {
            ImportPhase::SelectFile => "Select File",
            ImportPhase::Decomposing => "Decomposing...",
            ImportPhase::ReviewAssets => "Review Assets",
            ImportPhase::GeneratingPack => "Generating Pack...",
            ImportPhase::Complete => "Complete",
        }
    }

    pub fn step_index(&self) -> usize {
        match self {
            ImportPhase::SelectFile => 0,
            ImportPhase::Decomposing => 1,
            ImportPhase::ReviewAssets => 2,
            ImportPhase::GeneratingPack => 3,
            ImportPhase::Complete => 4,
        }
    }
}

// ============================================================================
// DECOMPOSED ASSET ENTRY — Preview row for each extracted asset
// ============================================================================

/// Display-friendly record of one decomposed asset
#[derive(Debug, Clone)]
pub struct DecomposedAssetEntry {
    pub name: String,
    pub category: String,
    pub mesh_path: PathBuf,
    pub vertex_count: u32,
    pub texture_count: usize,
    pub dimensions: [f32; 3],
    /// Whether the user has selected this asset for biome pack inclusion
    pub include_in_pack: bool,
}

// ============================================================================
// TEXTURE PROCESSING OPTIONS — Mirrors TextureProcessingConfig for UI
// ============================================================================

/// UI-side texture processing settings
#[derive(Debug, Clone)]
pub struct TextureSettings {
    pub convert_hdr: bool,
    pub generate_thumbnails: bool,
    pub thumbnail_size: u32,
    pub max_resolution: u32,
    pub jpeg_quality: u8,
    pub output_png: bool,
}

impl Default for TextureSettings {
    fn default() -> Self {
        Self {
            convert_hdr: true,
            generate_thumbnails: true,
            thumbnail_size: 128,
            max_resolution: 4096,
            jpeg_quality: 90,
            output_png: true,
        }
    }
}

// ============================================================================
// SCATTER SETTINGS — BiomePack scatter parameters for UI editing
// ============================================================================

/// UI-side scatter configuration
#[derive(Debug, Clone)]
pub struct ScatterSettings {
    pub density: f32,
    pub use_poisson_disk: bool,
    pub min_distance: f32,
    pub max_slope: f32,
    pub size_variation: f32,
    pub random_rotation: bool,
}

impl Default for ScatterSettings {
    fn default() -> Self {
        Self {
            density: 1.0,
            use_poisson_disk: true,
            min_distance: 2.0,
            max_slope: 45.0,
            size_variation: 0.3,
            random_rotation: true,
        }
    }
}

// ============================================================================
// BIOME TYPE SELECTION — Maps to astraweave_terrain::BiomeType
// ============================================================================

/// Simplified biome type selector for the UI
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum BiomeTypeSelection {
    Grassland,
    Forest,
    #[default]
    Desert,
    Tundra,
    Swamp,
    Mountain,
    Volcanic,
    Coastal,
}

impl BiomeTypeSelection {
    pub fn all() -> &'static [BiomeTypeSelection] {
        &[
            BiomeTypeSelection::Grassland,
            BiomeTypeSelection::Forest,
            BiomeTypeSelection::Desert,
            BiomeTypeSelection::Tundra,
            BiomeTypeSelection::Swamp,
            BiomeTypeSelection::Mountain,
            BiomeTypeSelection::Volcanic,
            BiomeTypeSelection::Coastal,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BiomeTypeSelection::Grassland => "Grassland",
            BiomeTypeSelection::Forest => "Forest",
            BiomeTypeSelection::Desert => "Desert",
            BiomeTypeSelection::Tundra => "Tundra",
            BiomeTypeSelection::Swamp => "Swamp",
            BiomeTypeSelection::Mountain => "Mountain",
            BiomeTypeSelection::Volcanic => "Volcanic",
            BiomeTypeSelection::Coastal => "Coastal",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BiomeTypeSelection::Grassland => "🌾",
            BiomeTypeSelection::Forest => "🌲",
            BiomeTypeSelection::Desert => "🏜️",
            BiomeTypeSelection::Tundra => "❄️",
            BiomeTypeSelection::Swamp => "🌿",
            BiomeTypeSelection::Mountain => "🏔️",
            BiomeTypeSelection::Volcanic => "🌋",
            BiomeTypeSelection::Coastal => "🏖️",
        }
    }
}

impl std::fmt::Display for BiomeTypeSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

// ============================================================================
// MAIN PANEL STATE
// ============================================================================

/// Blend Import panel state
pub struct BlendImportPanel {
    /// Current workflow phase
    phase: ImportPhase,
    /// Selected .blend file path
    blend_path: Option<PathBuf>,
    /// Output directory for decomposed assets
    output_dir: PathBuf,
    /// Blend file name (display)
    blend_filename: String,
    /// Blend file size in bytes
    blend_file_size: u64,
    /// Decomposed assets list
    assets: Vec<DecomposedAssetEntry>,
    /// HDRI files found during decomposition
    hdri_paths: Vec<PathBuf>,
    /// Ground texture groups found
    ground_texture_groups: Vec<(String, Vec<PathBuf>)>,
    /// Texture processing settings
    texture_settings: TextureSettings,
    /// Scatter settings for biome pack generation
    scatter_settings: ScatterSettings,
    /// Target biome type
    biome_type: BiomeTypeSelection,
    /// Biome pack name
    pack_name: String,
    /// Biome pack description
    pack_description: String,
    /// Status message displayed at the bottom
    status_message: String,
    /// Prominent error message (shown in red above content sections)
    error_message: Option<String>,
    /// Progress value (0.0 to 1.0) for progress bar
    progress: f32,
    /// Pending actions to be consumed by the editor
    actions: Vec<BlendImportAction>,
    /// Category filter for asset list
    category_filter: String,
    /// Whether to show only assets included in pack
    show_included_only: bool,
    /// Path to the generated biome pack file (set after successful pack generation)
    generated_pack_path: Option<PathBuf>,
    /// Collapsible section states
    section_file: bool,
    section_textures: bool,
    section_scatter: bool,
    section_assets: bool,
    section_pack: bool,
}

impl BlendImportPanel {
    /// Create a new blend import panel
    pub fn new() -> Self {
        Self {
            phase: ImportPhase::SelectFile,
            blend_path: None,
            output_dir: PathBuf::from("assets/imported"),
            blend_filename: String::new(),
            blend_file_size: 0,
            assets: Vec::new(),
            hdri_paths: Vec::new(),
            ground_texture_groups: Vec::new(),
            texture_settings: TextureSettings::default(),
            scatter_settings: ScatterSettings::default(),
            biome_type: BiomeTypeSelection::default(),
            pack_name: String::new(),
            pack_description: String::new(),
            status_message: "Select a .blend file to begin".into(),
            error_message: None,
            progress: 0.0,
            actions: Vec::new(),
            category_filter: "All".into(),
            show_included_only: false,
            generated_pack_path: None,
            section_file: true,
            section_textures: true,
            section_scatter: true,
            section_assets: true,
            section_pack: true,
        }
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Drain pending actions
    pub fn take_actions(&mut self) -> Vec<BlendImportAction> {
        std::mem::take(&mut self.actions)
    }

    /// Set the decomposition result (called by editor after async decomposition completes)
    pub fn set_decomposition_result(
        &mut self,
        assets: Vec<DecomposedAssetEntry>,
        hdri_paths: Vec<PathBuf>,
        ground_texture_groups: Vec<(String, Vec<PathBuf>)>,
    ) {
        self.assets = assets;
        self.hdri_paths = hdri_paths;
        self.ground_texture_groups = ground_texture_groups;
        self.phase = ImportPhase::ReviewAssets;
        self.progress = 1.0;

        let asset_count = self.assets.len();
        let hdri_count = self.hdri_paths.len();
        self.status_message = format!(
            "Decomposition complete: {} assets, {} HDRIs extracted",
            asset_count, hdri_count
        );

        // Auto-generate pack name from blend filename
        if self.pack_name.is_empty() {
            self.pack_name = self.blend_filename.trim_end_matches(".blend").to_string();
        }
    }

    /// Set the progress during decomposition
    pub fn set_progress(&mut self, progress: f32, message: &str) {
        self.progress = progress.clamp(0.0, 1.0);
        self.status_message = message.to_string();
    }

    /// Mark biome pack generation as complete
    pub fn set_pack_complete(&mut self) {
        self.phase = ImportPhase::Complete;
        self.progress = 1.0;
        self.status_message = format!("Biome pack '{}' generated successfully", self.pack_name);
    }

    /// Store the path to the generated biome pack file (for zone creation flow).
    pub fn set_generated_pack_path(&mut self, path: PathBuf) {
        self.generated_pack_path = Some(path);
    }

    /// Select all assets for pack inclusion
    fn select_all_assets(&mut self) {
        for asset in &mut self.assets {
            asset.include_in_pack = true;
        }
    }

    /// Deselect all assets
    fn deselect_all_assets(&mut self) {
        for asset in &mut self.assets {
            asset.include_in_pack = false;
        }
    }

    /// Count how many assets are included in the pack
    fn included_count(&self) -> usize {
        self.assets.iter().filter(|a| a.include_in_pack).count()
    }

    /// Get unique categories from the asset list
    fn unique_categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .assets
            .iter()
            .map(|a| a.category.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        cats.insert(0, "All".to_string());
        cats
    }

    // ========================================================================
    // UI SECTIONS
    // ========================================================================

    fn show_progress_bar(&self, ui: &mut Ui) {
        let steps = ["Select", "Decompose", "Review", "Pack", "Done"];
        let current = self.phase.step_index();

        ui.horizontal(|ui| {
            for (i, step) in steps.iter().enumerate() {
                let color = if i < current {
                    Color32::from_rgb(80, 180, 80) // completed
                } else if i == current {
                    Color32::from_rgb(80, 140, 220) // active
                } else {
                    Color32::from_rgb(100, 100, 100) // pending
                };
                ui.colored_label(color, RichText::new(*step).strong());
                if i < steps.len() - 1 {
                    ui.colored_label(Color32::GRAY, ">");
                }
            }
        });
        ui.separator();
    }

    fn show_file_section(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new(RichText::new("📁  File Selection").strong().size(14.0))
            .default_open(self.section_file)
            .show(ui, |ui| {
                ui.add_space(4.0);

                if let Some(path) = &self.blend_path {
                    ui.horizontal(|ui| {
                        ui.label("File:");
                        ui.monospace(self.blend_filename.as_str());
                    });

                    if self.blend_file_size > 0 {
                        let size_mb = self.blend_file_size as f64 / (1024.0 * 1024.0);
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            ui.monospace(format!("{:.1} MB", size_mb));
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        let path_str = path.display().to_string();
                        // Truncate long paths for display
                        let display_path = if path_str.len() > 60 {
                            format!("...{}", &path_str[path_str.len() - 57..])
                        } else {
                            path_str
                        };
                        ui.monospace(display_path);
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Change File").clicked() {
                            self.open_file_dialog();
                        }
                        if ui.button("Clear").clicked() {
                            self.actions.push(BlendImportAction::ClearSession);
                            self.reset();
                        }
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("No .blend file selected")
                                .color(Color32::GRAY)
                                .size(13.0),
                        );
                        ui.add_space(8.0);
                        if ui
                            .button(RichText::new("  Browse for .blend file  ").size(14.0))
                            .clicked()
                        {
                            self.open_file_dialog();
                        }
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("or drag-and-drop a .blend file onto the editor")
                                .color(Color32::from_rgb(140, 140, 140))
                                .italics(),
                        );
                        ui.add_space(16.0);
                    });
                }

                // Output directory
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Output Dir:");
                    let mut dir_str = self.output_dir.display().to_string();
                    if ui.text_edit_singleline(&mut dir_str).changed() {
                        self.output_dir = PathBuf::from(dir_str);
                    }
                });
            });
    }

    fn show_texture_settings(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new(RichText::new("🖼️  Texture Processing").strong().size(14.0))
            .default_open(self.section_textures)
            .show(ui, |ui| {
                ui.add_space(4.0);

                ui.checkbox(&mut self.texture_settings.convert_hdr, "Convert HDR to LDR");
                ui.checkbox(
                    &mut self.texture_settings.generate_thumbnails,
                    "Generate thumbnails",
                );

                ui.horizontal(|ui| {
                    ui.label("Thumbnail size:");
                    ui.add(
                        egui::DragValue::new(&mut self.texture_settings.thumbnail_size)
                            .range(32..=512)
                            .suffix("px"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Max resolution:");
                    ui.add(
                        egui::DragValue::new(&mut self.texture_settings.max_resolution)
                            .range(256..=8192)
                            .suffix("px"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Output format:");
                    ui.selectable_value(&mut self.texture_settings.output_png, true, "PNG");
                    ui.selectable_value(&mut self.texture_settings.output_png, false, "JPEG");
                });

                if !self.texture_settings.output_png {
                    ui.horizontal(|ui| {
                        ui.label("JPEG quality:");
                        ui.add(
                            egui::DragValue::new(&mut self.texture_settings.jpeg_quality)
                                .range(1..=100)
                                .suffix("%"),
                        );
                    });
                }
            });
    }

    fn show_scatter_settings(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new(RichText::new("🌿  Scatter Settings").strong().size(14.0))
            .default_open(self.section_scatter)
            .show(ui, |ui| {
                ui.add_space(4.0);

                // Biome type selector
                ui.horizontal(|ui| {
                    ui.label("Biome type:");
                    egui::ComboBox::from_id_salt("biome_type_combo")
                        .selected_text(self.biome_type.to_string())
                        .show_ui(ui, |ui| {
                            for bt in BiomeTypeSelection::all() {
                                ui.selectable_value(&mut self.biome_type, *bt, bt.to_string());
                            }
                        });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Density:");
                    ui.add(
                        egui::DragValue::new(&mut self.scatter_settings.density)
                            .range(0.01..=10.0)
                            .speed(0.05),
                    );
                });

                ui.checkbox(
                    &mut self.scatter_settings.use_poisson_disk,
                    "Poisson disk sampling",
                );

                if self.scatter_settings.use_poisson_disk {
                    ui.horizontal(|ui| {
                        ui.label("Min distance:");
                        ui.add(
                            egui::DragValue::new(&mut self.scatter_settings.min_distance)
                                .range(0.1..=50.0)
                                .speed(0.1)
                                .suffix("m"),
                        );
                    });
                }

                ui.horizontal(|ui| {
                    ui.label("Max slope:");
                    ui.add(
                        egui::DragValue::new(&mut self.scatter_settings.max_slope)
                            .range(0.0..=90.0)
                            .speed(0.5)
                            .suffix("°"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Size variation:");
                    ui.add(
                        egui::DragValue::new(&mut self.scatter_settings.size_variation)
                            .range(0.0..=1.0)
                            .speed(0.01),
                    );
                });

                ui.checkbox(
                    &mut self.scatter_settings.random_rotation,
                    "Random Y rotation",
                );
            });
    }

    fn show_asset_list(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new(
            RichText::new(format!("📦  Decomposed Assets ({})", self.assets.len()))
                .strong()
                .size(14.0),
        )
        .default_open(self.section_assets)
        .show(ui, |ui| {
            if self.assets.is_empty() {
                ui.colored_label(
                    Color32::GRAY,
                    "No assets yet — decompose a .blend file first",
                );
                return;
            }

            // Toolbar
            ui.horizontal(|ui| {
                // Category filter
                let categories = self.unique_categories();
                egui::ComboBox::from_id_salt("asset_category_filter")
                    .selected_text(&self.category_filter)
                    .show_ui(ui, |ui| {
                        for cat in &categories {
                            ui.selectable_value(&mut self.category_filter, cat.clone(), cat);
                        }
                    });

                ui.checkbox(&mut self.show_included_only, "Included only");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("None").clicked() {
                        self.deselect_all_assets();
                    }
                    if ui.small_button("All").clicked() {
                        self.select_all_assets();
                    }
                    ui.label(format!("{}/{}", self.included_count(), self.assets.len()));
                });
            });

            ui.separator();

            // Column headers
            ui.horizontal(|ui| {
                ui.label(RichText::new("Include").strong().size(11.0));
                ui.add_space(8.0);
                ui.label(RichText::new("Name").strong().size(11.0));
                ui.add_space(60.0);
                ui.label(RichText::new("Category").strong().size(11.0));
                ui.add_space(20.0);
                ui.label(RichText::new("Verts").strong().size(11.0));
                ui.add_space(12.0);
                ui.label(RichText::new("Textures").strong().size(11.0));
            });
            ui.separator();

            // Scrollable asset list
            let row_height = 22.0;
            let filter_cat = self.category_filter.clone();
            let show_included = self.show_included_only;

            // Collect indices of visible assets first
            let visible_indices: Vec<usize> = self
                .assets
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    let cat_match = filter_cat == "All" || a.category == filter_cat;
                    let inc_match = !show_included || a.include_in_pack;
                    cat_match && inc_match
                })
                .map(|(i, _)| i)
                .collect();

            egui::ScrollArea::vertical().max_height(300.0).show_rows(
                ui,
                row_height,
                visible_indices.len(),
                |ui, range| {
                    for &idx in &visible_indices[range] {
                        let asset = &self.assets[idx];
                        let mut include = asset.include_in_pack;

                        ui.horizontal(|ui| {
                            ui.checkbox(&mut include, "");
                            ui.add_space(4.0);

                            // Name (truncated)
                            let name = if asset.name.len() > 24 {
                                format!("{}...", &asset.name[..21])
                            } else {
                                asset.name.clone()
                            };
                            ui.label(RichText::new(name).monospace().size(11.0));

                            // Pad to align columns
                            let name_len = asset.name.len().min(24);
                            if name_len < 24 {
                                ui.add_space((24 - name_len) as f32 * 5.0);
                            }

                            // Category badge
                            let cat_color = category_color(&asset.category);
                            ui.colored_label(cat_color, RichText::new(&asset.category).size(11.0));

                            ui.add_space(12.0);
                            ui.monospace(
                                RichText::new(format_vertex_count(asset.vertex_count)).size(11.0),
                            );

                            ui.add_space(12.0);
                            ui.monospace(
                                RichText::new(format!("{}", asset.texture_count)).size(11.0),
                            );
                        });

                        // Apply checkbox change
                        self.assets[idx].include_in_pack = include;
                    }
                },
            );
        });
    }

    fn show_pack_section(&mut self, ui: &mut Ui) {
        egui::CollapsingHeader::new(RichText::new("📋  Biome Pack").strong().size(14.0))
            .default_open(self.section_pack)
            .show(ui, |ui| {
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Pack name:");
                    ui.text_edit_singleline(&mut self.pack_name);
                });

                ui.horizontal(|ui| {
                    ui.label("Description:");
                    ui.text_edit_singleline(&mut self.pack_description);
                });

                ui.add_space(4.0);

                // Summary
                let included = self.included_count();
                ui.label(format!(
                    "Assets: {}/{} selected",
                    included,
                    self.assets.len()
                ));
                ui.label(format!("HDRIs: {}", self.hdri_paths.len()));
                ui.label(format!(
                    "Ground texture groups: {}",
                    self.ground_texture_groups.len()
                ));
                ui.label(format!("Target biome: {}", self.biome_type));

                ui.add_space(8.0);

                // HDRI list (collapsed)
                if !self.hdri_paths.is_empty() {
                    egui::CollapsingHeader::new(format!("HDRIs ({})", self.hdri_paths.len())).show(
                        ui,
                        |ui| {
                            for p in &self.hdri_paths {
                                if let Some(name) = p.file_name() {
                                    ui.monospace(
                                        RichText::new(name.to_string_lossy().as_ref()).size(11.0),
                                    );
                                }
                            }
                        },
                    );
                }

                // Ground textures (collapsed)
                if !self.ground_texture_groups.is_empty() {
                    egui::CollapsingHeader::new(format!(
                        "Ground Textures ({})",
                        self.ground_texture_groups.len()
                    ))
                    .show(ui, |ui| {
                        for (name, paths) in &self.ground_texture_groups {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(name).strong().size(11.0));
                                ui.colored_label(
                                    Color32::GRAY,
                                    format!("{} channels", paths.len()),
                                );
                            });
                        }
                    });
                }
            });
    }

    fn show_action_buttons(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.add_space(4.0);

        match self.phase {
            ImportPhase::SelectFile => {
                let can_start = self.blend_path.is_some();
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(
                        RichText::new("  Start Decomposition  ").strong().size(14.0),
                    );
                    if ui.add_enabled(can_start, btn).clicked() {
                        if let Some(path) = self.blend_path.clone() {
                            self.phase = ImportPhase::Decomposing;
                            self.progress = 0.0;
                            self.error_message = None;
                            self.status_message = "Starting Blender decomposition...".into();
                            self.actions
                                .push(BlendImportAction::StartDecomposition { blend_path: path });
                        }
                    }
                });
            }
            ImportPhase::Decomposing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(&self.status_message);
                });
                let progress_bar = egui::ProgressBar::new(self.progress).show_percentage();
                ui.add(progress_bar);
                ui.add_space(4.0);
                if ui.button("Cancel").clicked() {
                    self.phase = ImportPhase::SelectFile;
                    self.progress = 0.0;
                    self.status_message = "Decomposition cancelled".into();
                    self.actions.push(BlendImportAction::CancelDecomposition);
                }
            }
            ImportPhase::ReviewAssets => {
                let has_included = self.included_count() > 0;
                let has_name = !self.pack_name.is_empty();

                ui.horizontal(|ui| {
                    let btn = egui::Button::new(
                        RichText::new("  Generate Biome Pack  ").strong().size(14.0),
                    );
                    if ui.add_enabled(has_included && has_name, btn).clicked() {
                        self.phase = ImportPhase::GeneratingPack;
                        self.progress = 0.0;
                        self.status_message = "Generating biome pack...".into();
                        self.actions.push(BlendImportAction::GenerateBiomePack {
                            output_dir: self.output_dir.clone(),
                            pack_name: self.pack_name.clone(),
                        });
                    }

                    if ui.button("Browse Output").clicked() {
                        self.actions.push(BlendImportAction::BrowseOutputDir {
                            path: self.output_dir.clone(),
                        });
                    }
                });

                if !has_name {
                    ui.colored_label(
                        Color32::from_rgb(220, 160, 60),
                        "Enter a pack name to continue",
                    );
                }
                if !has_included {
                    ui.colored_label(
                        Color32::from_rgb(220, 160, 60),
                        "Select at least one asset to include",
                    );
                }
            }
            ImportPhase::GeneratingPack => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(&self.status_message);
                });
                let progress_bar = egui::ProgressBar::new(self.progress).show_percentage();
                ui.add(progress_bar);
            }
            ImportPhase::Complete => {
                ui.colored_label(
                    Color32::from_rgb(80, 200, 80),
                    RichText::new(&self.status_message).strong().size(13.0),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Browse Output").clicked() {
                        self.actions.push(BlendImportAction::BrowseOutputDir {
                            path: self.output_dir.clone(),
                        });
                    }
                    if ui.button("Import Another").clicked() {
                        self.reset();
                    }
                });
                // Offer one-click zone creation from the generated pack
                if let Some(ref pack_path) = self.generated_pack_path {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(2.0);
                    ui.label(RichText::new("Scene Reconstruction").strong().size(12.0));
                    ui.label("Create a blueprint zone to place this scene in the viewport.");
                    if ui
                        .button(
                            RichText::new("Create Replica Zone")
                                .strong()
                                .color(Color32::from_rgb(60, 180, 220)),
                        )
                        .clicked()
                    {
                        self.actions.push(BlendImportAction::CreateReplicaZone {
                            pack_path: pack_path.clone(),
                        });
                        self.status_message =
                            "Creating replica zone — switch to Blueprint panel...".into();
                    }
                }
            }
        }
    }

    /// Open a native file dialog to select a .blend file
    fn open_file_dialog(&mut self) {
        self.trigger_file_browse();
    }

    /// Public entry point: open a native file dialog to select a .blend file.
    /// Called from menu bar "Import .blend Scene..." action.
    pub fn trigger_file_browse(&mut self) {
        let file = rfd::FileDialog::new()
            .set_title("Select Blender .blend File")
            .add_filter("Blender", &["blend"])
            .pick_file();

        if let Some(path) = file {
            self.set_blend_path(path);
        }
    }

    /// Set the currently selected blend file path
    pub fn set_blend_path(&mut self, path: PathBuf) {
        self.blend_filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.blend_file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        // Auto-set output dir based on blend filename
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "imported".to_string());
        self.output_dir = PathBuf::from("assets").join("imported").join(&stem);
        self.pack_name = stem;

        self.blend_path = Some(path);
        self.status_message = format!("Ready to decompose: {}", self.blend_filename);
    }

    /// Return the current texture settings for building a TextureProcessingConfig
    pub fn texture_settings(&self) -> &TextureSettings {
        &self.texture_settings
    }

    /// Return the current scatter settings
    pub fn scatter_settings(&self) -> &ScatterSettings {
        &self.scatter_settings
    }

    /// Return the current biome type selection
    pub fn biome_type(&self) -> BiomeTypeSelection {
        self.biome_type
    }

    /// Return the pack name
    pub fn pack_name(&self) -> &str {
        &self.pack_name
    }

    /// Return the pack description
    pub fn pack_description(&self) -> &str {
        &self.pack_description
    }

    /// Return the selected blend path
    pub fn blend_path(&self) -> Option<&PathBuf> {
        self.blend_path.as_ref()
    }

    /// Return the output directory
    pub fn output_dir(&self) -> &PathBuf {
        &self.output_dir
    }

    /// Return the current import phase
    pub fn phase(&self) -> ImportPhase {
        self.phase
    }

    /// Reset to file-selection phase (used after errors or cancellation)
    pub fn reset_to_select(&mut self, message: &str) {
        self.phase = ImportPhase::SelectFile;
        self.progress = 0.0;
        self.status_message = message.to_string();
    }

    /// Set a prominent error message (shown in red above content sections).
    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
    }

    /// Clear any displayed error message.
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Return the list of assets that are included in the pack
    pub fn included_assets(&self) -> Vec<&DecomposedAssetEntry> {
        self.assets.iter().filter(|a| a.include_in_pack).collect()
    }

    /// Reset the panel to initial state
    fn reset(&mut self) {
        self.phase = ImportPhase::SelectFile;
        self.blend_path = None;
        self.blend_filename.clear();
        self.blend_file_size = 0;
        self.assets.clear();
        self.hdri_paths.clear();
        self.ground_texture_groups.clear();
        self.pack_name.clear();
        self.pack_description.clear();
        self.progress = 0.0;
        self.status_message = "Select a .blend file to begin".into();
        self.error_message = None;
        self.category_filter = "All".into();
        self.show_included_only = false;
        self.generated_pack_path = None;
    }
}

impl Default for BlendImportPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for BlendImportPanel {
    fn name(&self) -> &str {
        "Blend Import"
    }

    fn show(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);

        // Header
        ui.heading(
            RichText::new("[BI] Blend Import")
                .strong()
                .color(Color32::from_rgb(180, 140, 220)),
        );
        ui.label(
            RichText::new("Import .blend scenes, extract assets, generate biome packs")
                .color(Color32::GRAY)
                .size(11.0),
        );
        ui.add_space(4.0);

        // Progress breadcrumb
        self.show_progress_bar(ui);

        // Action buttons first (above content so progress/cancel are always visible)
        self.show_action_buttons(ui);
        ui.add_space(2.0);

        // Prominent error display (always visible above content sections)
        if let Some(ref err) = self.error_message {
            ui.add_space(4.0);
            egui::Frame::NONE
                .fill(Color32::from_rgb(60, 20, 20))
                .corner_radius(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.colored_label(
                        Color32::from_rgb(255, 100, 100),
                        RichText::new(format!("\u{26A0} {err}")).strong().size(12.0),
                    );
                    ui.add_space(4.0);
                    ui.colored_label(
                        Color32::from_rgb(180, 140, 140),
                        RichText::new("Check the Console panel for detailed output.").size(10.0),
                    );
                });
            ui.add_space(4.0);
        }

        // Phase-gated content sections
        match self.phase {
            ImportPhase::SelectFile | ImportPhase::Decomposing => {
                self.show_file_section(ui);
                ui.add_space(2.0);
                self.show_texture_settings(ui);
                ui.add_space(2.0);
                self.show_scatter_settings(ui);
            }
            ImportPhase::ReviewAssets => {
                self.show_asset_list(ui);
                ui.add_space(2.0);
                self.show_pack_section(ui);
            }
            ImportPhase::GeneratingPack => {
                self.show_pack_section(ui);
            }
            ImportPhase::Complete => {
                self.show_asset_list(ui);
                ui.add_space(2.0);
                self.show_pack_section(ui);
            }
        }

        // Status bar
        ui.add_space(4.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&self.status_message)
                    .size(11.0)
                    .color(Color32::from_rgb(160, 160, 160)),
            );
        });
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Color for asset category badges
fn category_color(category: &str) -> Color32 {
    match category.to_lowercase().as_str() {
        "vegetation" => Color32::from_rgb(80, 180, 80),
        "rock" | "rocks" => Color32::from_rgb(160, 140, 120),
        "terrain" => Color32::from_rgb(140, 100, 60),
        "structure" => Color32::from_rgb(100, 140, 200),
        "prop" | "props" => Color32::from_rgb(200, 160, 80),
        _ => Color32::from_rgb(160, 160, 160),
    }
}

/// Format vertex count with K/M suffixes
fn format_vertex_count(count: u32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        format!("{}", count)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = BlendImportPanel::new();
        assert_eq!(panel.phase(), ImportPhase::SelectFile);
        assert!(panel.blend_path().is_none());
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_set_blend_path() {
        let mut panel = BlendImportPanel::new();
        panel.set_blend_path(PathBuf::from("test/Namaqualand.blend"));
        assert!(panel.blend_path().is_some());
        assert_eq!(panel.pack_name(), "Namaqualand");
        assert_eq!(
            panel.blend_path.as_ref().unwrap().to_str().unwrap(),
            "test/Namaqualand.blend"
        );
    }

    #[test]
    fn test_decomposition_result() {
        let mut panel = BlendImportPanel::new();
        panel.set_blend_path(PathBuf::from("test.blend"));

        let assets = vec![
            DecomposedAssetEntry {
                name: "QuiverTree_01".into(),
                category: "vegetation".into(),
                mesh_path: PathBuf::from("meshes/QuiverTree_01.glb"),
                vertex_count: 5000,
                texture_count: 3,
                dimensions: [2.0, 5.0, 2.0],
                include_in_pack: true,
            },
            DecomposedAssetEntry {
                name: "Boulder_Large".into(),
                category: "rock".into(),
                mesh_path: PathBuf::from("meshes/Boulder_Large.glb"),
                vertex_count: 1200,
                texture_count: 2,
                dimensions: [3.0, 2.0, 3.0],
                include_in_pack: true,
            },
        ];

        panel.set_decomposition_result(
            assets,
            vec![PathBuf::from("hdri/sunset.hdr")],
            vec![("cliff".into(), vec![PathBuf::from("tex/cliff_diffuse.png")])],
        );

        assert_eq!(panel.phase(), ImportPhase::ReviewAssets);
        assert_eq!(panel.included_count(), 2);
    }

    #[test]
    fn test_select_deselect_all() {
        let mut panel = BlendImportPanel::new();
        let assets = vec![
            DecomposedAssetEntry {
                name: "A".into(),
                category: "rock".into(),
                mesh_path: PathBuf::from("a.glb"),
                vertex_count: 100,
                texture_count: 1,
                dimensions: [1.0, 1.0, 1.0],
                include_in_pack: false,
            },
            DecomposedAssetEntry {
                name: "B".into(),
                category: "vegetation".into(),
                mesh_path: PathBuf::from("b.glb"),
                vertex_count: 200,
                texture_count: 2,
                dimensions: [1.0, 2.0, 1.0],
                include_in_pack: false,
            },
        ];
        panel.assets = assets;

        assert_eq!(panel.included_count(), 0);
        panel.select_all_assets();
        assert_eq!(panel.included_count(), 2);
        panel.deselect_all_assets();
        assert_eq!(panel.included_count(), 0);
    }

    #[test]
    fn test_unique_categories() {
        let mut panel = BlendImportPanel::new();
        panel.assets = vec![
            DecomposedAssetEntry {
                name: "A".into(),
                category: "rock".into(),
                mesh_path: PathBuf::from("a.glb"),
                vertex_count: 100,
                texture_count: 0,
                dimensions: [1.0; 3],
                include_in_pack: true,
            },
            DecomposedAssetEntry {
                name: "B".into(),
                category: "vegetation".into(),
                mesh_path: PathBuf::from("b.glb"),
                vertex_count: 100,
                texture_count: 0,
                dimensions: [1.0; 3],
                include_in_pack: true,
            },
            DecomposedAssetEntry {
                name: "C".into(),
                category: "rock".into(),
                mesh_path: PathBuf::from("c.glb"),
                vertex_count: 100,
                texture_count: 0,
                dimensions: [1.0; 3],
                include_in_pack: true,
            },
        ];

        let cats = panel.unique_categories();
        assert_eq!(cats.len(), 3); // "All", "rock", "vegetation"
        assert_eq!(cats[0], "All");
        assert!(cats.contains(&"rock".to_string()));
        assert!(cats.contains(&"vegetation".to_string()));
    }

    #[test]
    fn test_import_phase_progression() {
        assert_eq!(ImportPhase::SelectFile.step_index(), 0);
        assert_eq!(ImportPhase::Decomposing.step_index(), 1);
        assert_eq!(ImportPhase::ReviewAssets.step_index(), 2);
        assert_eq!(ImportPhase::GeneratingPack.step_index(), 3);
        assert_eq!(ImportPhase::Complete.step_index(), 4);
    }

    #[test]
    fn test_actions_produced() {
        let mut panel = BlendImportPanel::new();
        panel.set_blend_path(PathBuf::from("test.blend"));

        // Simulate clicking start decomposition
        panel.phase = ImportPhase::Decomposing;
        panel.actions.push(BlendImportAction::StartDecomposition {
            blend_path: PathBuf::from("test.blend"),
        });

        assert!(panel.has_pending_actions());
        let actions = panel.take_actions();
        assert_eq!(actions.len(), 1);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_format_vertex_count() {
        assert_eq!(format_vertex_count(500), "500");
        assert_eq!(format_vertex_count(1500), "1.5K");
        assert_eq!(format_vertex_count(1_500_000), "1.5M");
    }

    #[test]
    fn test_category_colors() {
        // Just ensure no panics
        let _ = category_color("vegetation");
        let _ = category_color("rock");
        let _ = category_color("terrain");
        let _ = category_color("structure");
        let _ = category_color("unknown");
    }

    #[test]
    fn test_biome_type_selection() {
        assert_eq!(BiomeTypeSelection::all().len(), 8);
        for bt in BiomeTypeSelection::all() {
            assert!(!bt.name().is_empty());
            assert!(!bt.icon().is_empty());
            let display = format!("{}", bt);
            assert!(display.contains(bt.name()));
        }
    }

    #[test]
    fn test_reset() {
        let mut panel = BlendImportPanel::new();
        panel.set_blend_path(PathBuf::from("test.blend"));
        panel.assets.push(DecomposedAssetEntry {
            name: "A".into(),
            category: "rock".into(),
            mesh_path: PathBuf::from("a.glb"),
            vertex_count: 100,
            texture_count: 0,
            dimensions: [1.0; 3],
            include_in_pack: true,
        });
        panel.pack_name = "TestPack".into();
        panel.phase = ImportPhase::Complete;

        panel.reset();

        assert_eq!(panel.phase(), ImportPhase::SelectFile);
        assert!(panel.blend_path().is_none());
        assert!(panel.assets.is_empty());
        assert!(panel.pack_name().is_empty());
    }

    #[test]
    fn test_pack_complete() {
        let mut panel = BlendImportPanel::new();
        panel.pack_name = "Namaqualand".into();
        panel.set_pack_complete();
        assert_eq!(panel.phase(), ImportPhase::Complete);
        assert!(panel.status_message.contains("Namaqualand"));
    }

    #[test]
    fn test_texture_settings_default() {
        let ts = TextureSettings::default();
        assert!(ts.convert_hdr);
        assert!(ts.generate_thumbnails);
        assert_eq!(ts.thumbnail_size, 128);
        assert_eq!(ts.max_resolution, 4096);
        assert_eq!(ts.jpeg_quality, 90);
        assert!(ts.output_png);
    }

    #[test]
    fn test_scatter_settings_default() {
        let ss = ScatterSettings::default();
        assert!((ss.density - 1.0).abs() < f32::EPSILON);
        assert!(ss.use_poisson_disk);
        assert!((ss.min_distance - 2.0).abs() < f32::EPSILON);
        assert!((ss.max_slope - 45.0).abs() < f32::EPSILON);
        assert!((ss.size_variation - 0.3).abs() < f32::EPSILON);
        assert!(ss.random_rotation);
    }
}
