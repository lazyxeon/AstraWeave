//! Entity Catalog Panel — thumbnail grid for spawning KayKit characters/enemies/bosses
//!
//! Scans the KayKit asset collection and presents a categorized, thumbnail-based
//! entity picker. Click any thumbnail to spawn the corresponding GLB model into the scene.

use egui::{Color32, ColorImage, ImageData, Sense, TextureHandle, Ui, Vec2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Category for entity catalog entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityCategory {
    Hero,
    Enemy,
    Boss,
    NPC,
    Special,
    Prop,
}

impl EntityCategory {
    pub fn all() -> &'static [Self] {
        &[
            Self::Hero,
            Self::Enemy,
            Self::Boss,
            Self::NPC,
            Self::Special,
            Self::Prop,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hero => "Heroes",
            Self::Enemy => "Enemies",
            Self::Boss => "Bosses",
            Self::NPC => "NPCs",
            Self::Special => "Special",
            Self::Prop => "Props",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Self::Hero => Color32::from_rgb(80, 180, 255),
            Self::Enemy => Color32::from_rgb(255, 100, 100),
            Self::Boss => Color32::from_rgb(200, 80, 255),
            Self::NPC => Color32::from_rgb(100, 220, 150),
            Self::Special => Color32::from_rgb(255, 200, 80),
            Self::Prop => Color32::from_rgb(180, 180, 180),
        }
    }
}

/// A single entry in the catalog.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub display_name: String,
    pub path: String,
    pub category: EntityCategory,
    pub pack: String,
}

// ---------------------------------------------------------------------------
// Catalog builder (runs once at startup)
// ---------------------------------------------------------------------------

/// Build the full entity catalog by scanning the assets directory.
pub fn build_catalog() -> Vec<CatalogEntry> {
    let mut entries = Vec::new();

    let kaykit_root = PathBuf::from("assets").join("The Complete KayKit Collection v4");

    if kaykit_root.is_dir() {
        scan_kaykit(&kaykit_root, &mut entries);
    }

    // Also scan Kenney / other 3D asset packs that contain character models
    let assets_3d = PathBuf::from("assets").join("3D assets");
    if assets_3d.is_dir() {
        scan_3d_assets(&assets_3d, &mut entries);
    }

    entries.sort_by(|a, b| {
        a.category
            .label()
            .cmp(b.category.label())
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    entries
}

/// Classify a GLB file name into a category.
fn classify(name: &str, pack_hint: &str) -> EntityCategory {
    let n = name.to_lowercase();
    let p = pack_hint.to_lowercase();

    // Bosses
    if n.contains("golem")
        || n.contains("necromancer")
        || n.contains("werewolf")
        || n.contains("vampire")
        || n.contains("witch")
        || n.contains("combatmech")
        || n.contains("blackknight")
        || n.contains("frostgolem")
    {
        return EntityCategory::Boss;
    }

    // Enemies (skeleton pack or monster-ish)
    if p.contains("skeleton")
        || n.contains("skeleton")
        || n.contains("monster")
        || n.contains("animatronic")
        || n.contains("clanker")
    {
        return EntityCategory::Enemy;
    }

    // Heroes / player characters
    if p.contains("adventurer")
        || n.contains("barbarian")
        || n.contains("knight")
        || n.contains("mage")
        || n.contains("ranger")
        || n.contains("rogue")
        || n.contains("druid")
        || n.contains("engineer")
        || n.contains("paladin")
        || n.contains("ninja")
        || n.contains("superhero")
        || n.contains("survivalist")
        || n.contains("protagonist")
        || n.contains("hiker")
        || n.contains("tiefling")
        || n.contains("caveman")
        || n.contains("spaceranger")
    {
        return EntityCategory::Hero;
    }

    // NPCs
    if n.contains("helper")
        || n.contains("driver")
        || n.contains("clown")
        || n.contains("mannequin")
        || n.contains("dummy")
        || n.contains("actionfigure")
    {
        return EntityCategory::NPC;
    }

    // Props (orc equipment, vehicles, etc.)
    if n.contains("axe")
        || n.contains("club")
        || n.contains("backpack")
        || n.contains("wardrum")
        || n.contains("drinkinghorn")
        || n.contains("car")
        || n.contains("cone")
        || n.contains("roofrack")
    {
        return EntityCategory::Prop;
    }

    // Robots and special characters
    if n.contains("robot") || n.contains("monstercostume") || n.contains("orc") {
        return EntityCategory::Special;
    }

    // Default: if it's in a character folder, it's special
    EntityCategory::Special
}

fn scan_kaykit(root: &Path, entries: &mut Vec<CatalogEntry>) {
    let Ok(packs) = std::fs::read_dir(root) else {
        return;
    };

    for pack_entry in packs.flatten() {
        let pack_path = pack_entry.path();
        if !pack_path.is_dir() {
            continue;
        }
        let pack_name = pack_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Skip animation-only packs
        if pack_name.contains("Animation") && !pack_name.contains("Character") {
            // Still check for mannequin characters inside
        }

        // Recursively find all .glb files that are character models (not animation rigs)
        collect_glb_recursive(&pack_path, &pack_name, entries);
    }
}

fn collect_glb_recursive(dir: &Path, pack_name: &str, entries: &mut Vec<CatalogEntry>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip animation rig directories
            let dir_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if dir_name == "animations" || dir_name.starts_with("rig_") {
                continue;
            }
            collect_glb_recursive(&path, pack_name, entries);
        } else if let Some(ext) = path.extension() {
            if ext == "glb" {
                let stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // Skip animation rigs
                if stem.starts_with("Rig_") {
                    continue;
                }

                let display = stem.replace('_', " ");
                let category = classify(&stem, pack_name);
                let rel_path = path.display().to_string();

                entries.push(CatalogEntry {
                    display_name: display,
                    path: rel_path,
                    category,
                    pack: pack_name.to_string(),
                });
            }
        }
    }
}

fn scan_3d_assets(root: &Path, entries: &mut Vec<CatalogEntry>) {
    // For 3D assets packs (Kenney Nature Kit already used for scatter),
    // only grab models that look like characters/creatures.
    let Ok(packs) = std::fs::read_dir(root) else {
        return;
    };

    for pack_entry in packs.flatten() {
        let pack_path = pack_entry.path();
        if !pack_path.is_dir() {
            continue;
        }
        let pack_name = pack_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Only pick up character-like models from these packs
        let search_dirs = [
            pack_path.join("Characters"),
            pack_path.join("characters"),
            pack_path.join("Enemies"),
            pack_path.join("enemies"),
        ];

        for search_dir in &search_dirs {
            if let Ok(files) = std::fs::read_dir(search_dir) {
                for file in files.flatten() {
                    let file_path = file.path();
                    if let Some(ext) = file_path.extension() {
                        if ext == "glb" || ext == "gltf" {
                            let stem = file_path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let display = format!("{} / {}", pack_name, stem.replace('_', " "));
                            let rel_path = file_path.display().to_string();
                            entries.push(CatalogEntry {
                                display_name: display,
                                path: rel_path,
                                category: classify(&stem, &pack_name),
                                pack: pack_name.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Thumbnail generation
// ---------------------------------------------------------------------------

/// Generate a colored placeholder thumbnail for a catalog entry.
fn generate_thumbnail(ctx: &egui::Context, entry: &CatalogEntry) -> TextureHandle {
    let size = 64usize;
    let mut pixels = vec![0u8; size * size * 4];
    let color = entry.category.color();
    let [r, g, b, _] = color.to_array();

    // Background: darker shade
    let bg_r = (r as u16 * 35 / 100) as u8;
    let bg_g = (g as u16 * 35 / 100) as u8;
    let bg_b = (b as u16 * 35 / 100) as u8;

    for pixel in pixels.chunks_exact_mut(4) {
        pixel[0] = bg_r;
        pixel[1] = bg_g;
        pixel[2] = bg_b;
        pixel[3] = 255;
    }

    // Draw a 2px bright border
    for y in 0..size {
        for x in 0..size {
            if x < 2 || x >= size - 2 || y < 2 || y >= size - 2 {
                let idx = (y * size + x) * 4;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = 255;
            }
        }
    }

    // Draw a simple silhouette icon in the center (head + body triangle)
    let cx = size / 2;
    let cy = size / 2;
    // Head (circle-ish, radius 8)
    for dy in -8i32..=8 {
        for dx in -8i32..=8 {
            if dx * dx + dy * dy <= 64 {
                let px = (cx as i32 + dx) as usize;
                let py = (cy as i32 + dy - 8) as usize;
                if px < size && py < size {
                    let idx = (py * size + px) * 4;
                    pixels[idx] = r;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = b;
                    pixels[idx + 3] = 255;
                }
            }
        }
    }
    // Body (triangle below head)
    for row in 0..16u32 {
        let half_width = (row + 4) as i32;
        let py = cy + row as usize + 2;
        if py >= size {
            break;
        }
        for dx in -half_width..=half_width {
            let px = (cx as i32 + dx) as usize;
            if px < size {
                let idx = (py * size + px) * 4;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = 255;
            }
        }
    }

    let color_image = ColorImage::from_rgba_unmultiplied([size, size], &pixels);

    ctx.load_texture(
        format!("entity_thumb_{}", entry.display_name),
        ImageData::Color(std::sync::Arc::new(color_image)),
        egui::TextureOptions::LINEAR,
    )
}

// ---------------------------------------------------------------------------
// Panel state & UI
// ---------------------------------------------------------------------------

/// State for the entity catalog panel.
pub struct EntityCatalogState {
    /// All catalog entries (None = not yet loaded)
    entries: Option<Vec<CatalogEntry>>,
    /// Currently selected category filter (None = show all)
    selected_category: Option<EntityCategory>,
    /// Search filter
    search: String,
    /// Thumbnail cache (display_name → TextureHandle)
    thumbnails: HashMap<String, TextureHandle>,
    /// Spawn events to emit this frame (name, path)
    pending_spawns: Vec<(String, String)>,
    /// Thumbnail size
    thumb_size: f32,
}

impl EntityCatalogState {
    pub fn new() -> Self {
        // Deferred: catalog is built lazily on first show() to avoid blocking startup
        Self {
            entries: None,
            selected_category: None,
            search: String::new(),
            thumbnails: HashMap::new(),
            pending_spawns: Vec::new(),
            thumb_size: 72.0,
        }
    }

    /// Ensure the catalog is loaded (lazy init on first access).
    fn ensure_loaded(&mut self) {
        if self.entries.is_none() {
            let entries = build_catalog();
            tracing::info!(
                "Entity catalog: {} entries found (lazy load)",
                entries.len()
            );
            self.entries = Some(entries);
        }
    }

    /// Take pending spawn events (consumed by tab_viewer / main loop).
    pub fn take_spawns(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.pending_spawns)
    }

    /// Number of catalog entries.
    pub fn entry_count(&self) -> usize {
        self.entries.as_ref().map_or(0, |e| e.len())
    }

    /// Show the entity catalog UI.
    pub fn show(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        // Lazy-load on first display
        self.ensure_loaded();
        let entries = self.entries.as_ref().unwrap();

        // Category tabs
        ui.horizontal_wrapped(|ui| {
            let all_selected = self.selected_category.is_none();
            if ui
                .selectable_label(all_selected, format!("All ({})", entries.len()))
                .clicked()
            {
                self.selected_category = None;
            }
            for &cat in EntityCategory::all() {
                let count = entries.iter().filter(|e| e.category == cat).count();
                if count == 0 {
                    continue;
                }
                let is_sel = self.selected_category == Some(cat);
                let label = format!("{} ({})", cat.label(), count);
                let resp = ui.selectable_label(is_sel, label);
                if resp.clicked() {
                    self.selected_category = Some(cat);
                }
            }
        });

        ui.add_space(2.0);

        // Search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search);
            if ui.small_button("X").clicked() {
                self.search.clear();
            }
        });

        ui.add_space(4.0);

        // Thumbnail grid
        let search_lower = self.search.to_lowercase();
        let filtered: Vec<&CatalogEntry> = entries
            .iter()
            .filter(|e| {
                if let Some(cat) = self.selected_category {
                    if e.category != cat {
                        return false;
                    }
                }
                if !search_lower.is_empty()
                    && !e.display_name.to_lowercase().contains(&search_lower)
                    && !e.pack.to_lowercase().contains(&search_lower)
                {
                    return false;
                }
                true
            })
            .collect();

        if filtered.is_empty() {
            ui.label("No entities match the current filter.");
            return;
        }

        let available_width = ui.available_width();
        let cell_width = self.thumb_size + 8.0;
        let columns = ((available_width / cell_width).floor() as usize).max(1);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                egui::Grid::new("entity_catalog_grid")
                    .spacing(Vec2::new(6.0, 6.0))
                    .show(ui, |ui| {
                        for (i, entry) in filtered.iter().enumerate() {
                            // Ensure thumbnail exists
                            if !self.thumbnails.contains_key(&entry.display_name) {
                                let tex = generate_thumbnail(ctx, entry);
                                self.thumbnails.insert(entry.display_name.clone(), tex);
                            }

                            let thumb = &self.thumbnails[&entry.display_name];

                            // Thumbnail button
                            ui.vertical(|ui| {
                                let img = egui::Image::new(thumb)
                                    .fit_to_exact_size(Vec2::splat(self.thumb_size));
                                let resp = ui.add(img.sense(Sense::click()));

                                if resp.clicked() {
                                    self.pending_spawns
                                        .push((entry.display_name.clone(), entry.path.clone()));
                                }

                                if resp.hovered() {
                                    resp.on_hover_text(format!(
                                        "{}\nPack: {}\nCategory: {}\nClick to spawn",
                                        entry.display_name,
                                        entry.pack,
                                        entry.category.label()
                                    ));
                                }

                                // Truncated label
                                let max_chars = (self.thumb_size / 6.0) as usize;
                                let label = if entry.display_name.len() > max_chars {
                                    format!(
                                        "{}...",
                                        &entry.display_name[..max_chars.saturating_sub(3)]
                                    )
                                } else {
                                    entry.display_name.clone()
                                };
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(label)
                                            .small()
                                            .color(entry.category.color()),
                                    )
                                    .truncate(),
                                );
                            });

                            if (i + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }
}
