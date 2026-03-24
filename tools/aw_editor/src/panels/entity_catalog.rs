//! Entity Catalog Panel — categorized, virtual-scrolled thumbnail grid for all GLB/GLTF models
//!
//! Scans the entire assets directory tree (KayKit collection, 3D asset packs,
//! and loose models) and presents a categorized, pack-filterable entity picker.
//! Click any thumbnail to spawn the corresponding model into the scene.

use egui::{Color32, ColorImage, ImageData, Sense, TextureHandle, Ui, Vec2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Broad category for entity catalog entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntityCategory {
    Character,
    Building,
    Vehicle,
    Nature,
    Furniture,
    Food,
    Weapon,
    Infrastructure,
    Prop,
}

impl EntityCategory {
    pub fn all() -> &'static [Self] {
        &[
            Self::Character,
            Self::Building,
            Self::Vehicle,
            Self::Nature,
            Self::Furniture,
            Self::Food,
            Self::Weapon,
            Self::Infrastructure,
            Self::Prop,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Character => "Characters",
            Self::Building => "Buildings",
            Self::Vehicle => "Vehicles",
            Self::Nature => "Nature",
            Self::Furniture => "Furniture",
            Self::Food => "Food",
            Self::Weapon => "Weapons",
            Self::Infrastructure => "Infrastructure",
            Self::Prop => "Props",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Self::Character => Color32::from_rgb(80, 180, 255),
            Self::Building => Color32::from_rgb(255, 160, 80),
            Self::Vehicle => Color32::from_rgb(100, 220, 150),
            Self::Nature => Color32::from_rgb(80, 200, 80),
            Self::Furniture => Color32::from_rgb(200, 160, 100),
            Self::Food => Color32::from_rgb(255, 200, 80),
            Self::Weapon => Color32::from_rgb(220, 80, 80),
            Self::Infrastructure => Color32::from_rgb(160, 160, 200),
            Self::Prop => Color32::from_rgb(200, 160, 255),
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
// Catalog builder — scans all asset directories eagerly at startup
// ---------------------------------------------------------------------------

/// Build the full entity catalog by scanning all asset directories.
pub fn build_catalog() -> Vec<CatalogEntry> {
    let mut entries = Vec::new();

    // 1. KayKit character collection
    let kaykit_root = PathBuf::from("assets").join("The Complete KayKit Collection v4");
    if kaykit_root.is_dir() {
        scan_kaykit(&kaykit_root, &mut entries);
    }

    // 2. All 3D asset packs (GLB + GLTF format directories)
    let assets_3d = PathBuf::from("assets").join("3D assets");
    if assets_3d.is_dir() {
        scan_3d_assets(&assets_3d, &mut entries);
    }

    // 3. Loose models directory
    let models_dir = PathBuf::from("assets").join("models");
    if models_dir.is_dir() {
        scan_models_dir(&models_dir, &mut entries);
    }

    entries.sort_by(|a, b| {
        a.category
            .label()
            .cmp(b.category.label())
            .then_with(|| a.pack.cmp(&b.pack))
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    entries
}

// ---------------------------------------------------------------------------
// Classification helpers
// ---------------------------------------------------------------------------

/// Classify a KayKit model by filename.
fn classify_kaykit_entry(name: &str, _pack_hint: &str) -> EntityCategory {
    let n = name.to_lowercase();
    // Props within KayKit (weapons, equipments, vehicles)
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
    EntityCategory::Character
}

/// Classify a 3D asset pack by its name.
fn classify_pack(pack_name: &str) -> EntityCategory {
    let p = pack_name.to_lowercase();

    // Characters
    if p.contains("character") || p.starts_with("blocky") {
        return EntityCategory::Character;
    }

    // Vehicles (actual vehicles, not track pieces)
    if p.contains("car kit")
        || p.contains("toy car")
        || p.contains("watercraft")
        || p.contains("racing")
        || p.contains("train kit")
    {
        return EntityCategory::Vehicle;
    }

    // Nature / environmental
    if p.contains("nature") || p.contains("graveyard") {
        return EntityCategory::Nature;
    }

    // Food
    if p.contains("food") {
        return EntityCategory::Food;
    }

    // Weapons & combat
    if p.contains("weapon") || p.contains("blaster") {
        return EntityCategory::Weapon;
    }

    // Furniture & interiors
    if p.contains("furniture") {
        return EntityCategory::Furniture;
    }

    // Buildings / structures
    if p.contains("castle")
        || p.contains("building")
        || p.contains("dungeon")
        || p.contains("medieval")
        || p.contains("town")
        || p.contains("brick")
        || p.contains("space station")
        || p.contains("modular")
        || p.contains("pirate")
        || p.contains("tower defense")
        || p.contains("space kit")
    {
        return EntityCategory::Building;
    }

    // Infrastructure — roads, tracks, tiles, rails, conveyors, theme park pieces
    if p.contains("road")
        || p.contains("city kit")
        || p.contains("coaster")
        || p.contains("conveyor")
        || p.contains("hexagon")
        || p.contains("minigolf")
        || p.contains("marble")
        || p.contains("prototype")
        || p.contains("industrial")
        || p.contains("commercial")
        || p.contains("suburban")
    {
        return EntityCategory::Infrastructure;
    }

    // Themed prop sets (platformer, holiday, survival, arena, skate, etc.)
    EntityCategory::Prop
}

/// Classify a loose model file by its filename.
fn classify_loose_model(name: &str) -> EntityCategory {
    let n = name.to_lowercase();

    if n.contains("castle")
        || n.contains("tower")
        || n.contains("wall")
        || n.contains("gate")
        || n.contains("battlement")
        || n.contains("house")
        || n.contains("roof")
        || n.contains("stair")
        || n.contains("door")
        || n.contains("window")
        || n.contains("floor")
        || n.contains("pillar")
        || n.contains("column")
    {
        return EntityCategory::Building;
    }

    if n.contains("bridge") || n.contains("road") || n.contains("path") {
        return EntityCategory::Infrastructure;
    }

    if n.contains("tree")
        || n.contains("rock")
        || n.contains("grass")
        || n.contains("flower")
        || n.contains("cactus")
        || n.contains("mushroom")
        || n.contains("water")
        || n.contains("terrain")
    {
        return EntityCategory::Nature;
    }

    if n.contains("character")
        || n.contains("npc")
        || n.contains("person")
        || n.contains("skeleton")
    {
        return EntityCategory::Character;
    }

    if n.contains("bed")
        || n.contains("chair")
        || n.contains("table")
        || n.contains("shelf")
        || n.contains("cabinet")
        || n.contains("lamp")
        || n.contains("candle")
    {
        return EntityCategory::Furniture;
    }

    EntityCategory::Prop
}

// ---------------------------------------------------------------------------
// Directory scanners
// ---------------------------------------------------------------------------

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

        collect_glb_recursive(&pack_path, &pack_name, entries, classify_kaykit_entry);
    }
}

fn collect_glb_recursive(
    dir: &Path,
    pack_name: &str,
    entries: &mut Vec<CatalogEntry>,
    classifier: fn(&str, &str) -> EntityCategory,
) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if dir_name == "animations" || dir_name.starts_with("rig_") {
                continue;
            }
            collect_glb_recursive(&path, pack_name, entries, classifier);
        } else if let Some(ext) = path.extension() {
            if ext == "glb" || ext == "gltf" {
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
                let category = classifier(&stem, pack_name);

                entries.push(CatalogEntry {
                    display_name: display,
                    path: path.display().to_string(),
                    category,
                    pack: pack_name.to_string(),
                });
            }
        }
    }
}

fn scan_3d_assets(root: &Path, entries: &mut Vec<CatalogEntry>) {
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

        let category = classify_pack(&pack_name);

        // Scan both GLB and GLTF format directories
        for subdir in &["GLB format", "GLTF format"] {
            let model_dir = pack_path.join("Models").join(subdir);
            if !model_dir.is_dir() {
                continue;
            }
            let Ok(files) = std::fs::read_dir(&model_dir) else {
                continue;
            };

            for file in files.flatten() {
                let file_path = file.path();
                if let Some(ext) = file_path.extension() {
                    if ext == "glb" || ext == "gltf" {
                        let stem = file_path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();

                        let display = stem.replace('_', " ");

                        entries.push(CatalogEntry {
                            display_name: display,
                            path: file_path.display().to_string(),
                            category,
                            pack: pack_name.clone(),
                        });
                    }
                }
            }
        }
    }
}

fn scan_models_dir(dir: &Path, entries: &mut Vec<CatalogEntry>) {
    scan_models_recursive(dir, "Models", entries);
}

fn scan_models_recursive(dir: &Path, pack_name: &str, entries: &mut Vec<CatalogEntry>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let subdir = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            scan_models_recursive(&path, &subdir, entries);
        } else if let Some(ext) = path.extension() {
            if ext == "glb" || ext == "gltf" {
                let stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // Skip animation libraries
                if stem.to_lowercase().contains("animationlibrary") {
                    continue;
                }

                let display = stem.replace('_', " ");
                let category = classify_loose_model(&stem);

                entries.push(CatalogEntry {
                    display_name: display,
                    path: path.display().to_string(),
                    category,
                    pack: pack_name.to_string(),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Thumbnail generation — category-specific icons with pack-based color variety
// ---------------------------------------------------------------------------

fn set_pixel(pixels: &mut [u8], size: usize, x: usize, y: usize, r: u8, g: u8, b: u8) {
    if x < size && y < size {
        let idx = (y * size + x) * 4;
        pixels[idx] = r;
        pixels[idx + 1] = g;
        pixels[idx + 2] = b;
        pixels[idx + 3] = 255;
    }
}

/// Person silhouette: head circle + body trapezoid.
fn draw_character_icon(pixels: &mut [u8], size: usize, r: u8, g: u8, b: u8) {
    let cx = size / 2;
    let cy = size / 2;
    // Head
    for dy in -6i32..=6 {
        for dx in -6i32..=6 {
            if dx * dx + dy * dy <= 36 {
                set_pixel(
                    pixels,
                    size,
                    (cx as i32 + dx) as usize,
                    (cy as i32 + dy - 10) as usize,
                    r,
                    g,
                    b,
                );
            }
        }
    }
    // Body
    for row in 0..14u32 {
        let half_w = (row / 2 + 3) as i32;
        let py = cy + row as usize - 2;
        for dx in -half_w..=half_w {
            set_pixel(pixels, size, (cx as i32 + dx) as usize, py, r, g, b);
        }
    }
}

/// House shape: rectangle body + triangle roof.
fn draw_building_icon(pixels: &mut [u8], size: usize, r: u8, g: u8, b: u8) {
    let cx = size / 2;
    let cy = size / 2;
    // Body rectangle
    for y in cy.saturating_sub(2)..cy + 14 {
        for x in cx.saturating_sub(10)..cx + 10 {
            set_pixel(pixels, size, x, y, r, g, b);
        }
    }
    // Roof triangle
    for row in 0..10u32 {
        let half_w = (10 - row as i32).max(0);
        let py = cy.saturating_sub(2 + row as usize);
        for dx in -half_w..=half_w {
            set_pixel(pixels, size, (cx as i32 + dx) as usize, py, r, g, b);
        }
    }
}

/// Car shape: body + cabin + two wheels.
fn draw_vehicle_icon(pixels: &mut [u8], size: usize, r: u8, g: u8, b: u8) {
    let cx = size / 2;
    let cy = size / 2;
    // Body
    for y in cy.saturating_sub(4)..cy + 4 {
        for x in cx.saturating_sub(14)..cx + 14 {
            set_pixel(pixels, size, x, y, r, g, b);
        }
    }
    // Cabin
    for y in cy.saturating_sub(10)..cy.saturating_sub(4) {
        for x in cx.saturating_sub(8)..cx + 6 {
            set_pixel(pixels, size, x, y, r, g, b);
        }
    }
    // Wheels
    for dy in -3i32..=3 {
        for dx in -3i32..=3 {
            if dx * dx + dy * dy <= 9 {
                set_pixel(
                    pixels,
                    size,
                    (cx as i32 - 9 + dx) as usize,
                    (cy as i32 + 6 + dy) as usize,
                    r,
                    g,
                    b,
                );
                set_pixel(
                    pixels,
                    size,
                    (cx as i32 + 9 + dx) as usize,
                    (cy as i32 + 6 + dy) as usize,
                    r,
                    g,
                    b,
                );
            }
        }
    }
}

/// Tree shape: triangular crown + narrow trunk.
fn draw_tree_icon(pixels: &mut [u8], size: usize, r: u8, g: u8, b: u8) {
    let cx = size / 2;
    let cy = size / 2;
    // Crown (triangle)
    for row in 0..18u32 {
        let half_w = (row / 2) as i32;
        let py = cy.saturating_sub(12) + row as usize;
        for dx in -half_w..=half_w {
            set_pixel(pixels, size, (cx as i32 + dx) as usize, py, r, g, b);
        }
    }
    // Trunk
    for y in cy + 6..cy + 14 {
        for x in cx.saturating_sub(2)..cx + 2 {
            set_pixel(pixels, size, x, y, r, g, b);
        }
    }
}

/// Diamond / gem shape for generic props.
fn draw_prop_icon(pixels: &mut [u8], size: usize, r: u8, g: u8, b: u8) {
    let cx = size / 2;
    let cy = size / 2;
    for dy in -10i32..=10 {
        let half_w = 10 - dy.abs();
        let py = (cy as i32 + dy) as usize;
        for dx in -half_w..=half_w {
            set_pixel(pixels, size, (cx as i32 + dx) as usize, py, r, g, b);
        }
    }
}

/// Generate a procedural thumbnail with category-specific icon and pack-varied color.
fn generate_thumbnail(ctx: &egui::Context, entry: &CatalogEntry) -> TextureHandle {
    let size = 64usize;
    let mut pixels = vec![0u8; size * size * 4];
    let base = entry.category.color();
    let [br, bg, bb, _] = base.to_array();

    // Pack-based hue variation so different packs look distinct
    let hash = entry
        .pack
        .bytes()
        .fold(0u32, |a, c| a.wrapping_mul(31).wrapping_add(c as u32));
    let shift = (hash % 40) as i16 - 20;
    let r = (br as i16 + shift).clamp(0, 255) as u8;
    let g = (bg as i16 - shift / 2).clamp(0, 255) as u8;
    let b = (bb as i16 + shift / 3).clamp(0, 255) as u8;

    // Dark background with subtle gradient
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let grad = (y as u16 * 12 / size as u16) as u8;
            pixels[idx] = 30 + grad;
            pixels[idx + 1] = 30 + grad;
            pixels[idx + 2] = 38 + grad;
            pixels[idx + 3] = 255;
        }
    }

    // 1px colored border
    for y in 0..size {
        for x in 0..size {
            if x == 0 || x == size - 1 || y == 0 || y == size - 1 {
                let idx = (y * size + x) * 4;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = 255;
            }
        }
    }

    // Category-specific icon
    match entry.category {
        EntityCategory::Character => draw_character_icon(&mut pixels, size, r, g, b),
        EntityCategory::Building => draw_building_icon(&mut pixels, size, r, g, b),
        EntityCategory::Vehicle => draw_vehicle_icon(&mut pixels, size, r, g, b),
        EntityCategory::Nature => draw_tree_icon(&mut pixels, size, r, g, b),
        EntityCategory::Infrastructure => draw_building_icon(&mut pixels, size, r, g, b),
        EntityCategory::Furniture | EntityCategory::Food | EntityCategory::Weapon
        | EntityCategory::Prop => draw_prop_icon(&mut pixels, size, r, g, b),
    }

    let color_image = ColorImage::from_rgba_unmultiplied([size, size], &pixels);

    // Use a hash of the path as the texture name (unique per entry)
    let name_hash = entry
        .path
        .bytes()
        .fold(0u32, |a, c| a.wrapping_mul(31).wrapping_add(c as u32));
    ctx.load_texture(
        format!("ent_{name_hash:08x}"),
        ImageData::Color(std::sync::Arc::new(color_image)),
        egui::TextureOptions::LINEAR,
    )
}

// ---------------------------------------------------------------------------
// Panel state & UI
// ---------------------------------------------------------------------------

/// State for the entity catalog panel.
pub struct EntityCatalogState {
    /// All catalog entries (eagerly loaded on construction).
    entries: Vec<CatalogEntry>,
    /// Unique pack names for the filter dropdown.
    packs: Vec<String>,
    /// Currently selected category filter (None = show all).
    selected_category: Option<EntityCategory>,
    /// Currently selected pack filter (None = all packs).
    selected_pack: Option<String>,
    /// Search filter text.
    search: String,
    /// Thumbnail cache keyed by model path.
    thumbnails: HashMap<String, TextureHandle>,
    /// Spawn events to emit this frame (name, path).
    pending_spawns: Vec<(String, String)>,
    /// Thumbnail display size.
    thumb_size: f32,
}

impl EntityCatalogState {
    pub fn new() -> Self {
        let entries = build_catalog();
        tracing::info!("Entity catalog: {} entries loaded", entries.len());

        // Collect unique sorted pack names
        let mut packs: Vec<String> = entries.iter().map(|e| e.pack.clone()).collect();
        packs.sort();
        packs.dedup();

        Self {
            entries,
            packs,
            selected_category: None,
            selected_pack: None,
            search: String::new(),
            thumbnails: HashMap::new(),
            pending_spawns: Vec::new(),
            thumb_size: 72.0,
        }
    }

    /// Take pending spawn events (consumed by tab_viewer / main loop).
    pub fn take_spawns(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.pending_spawns)
    }

    /// Number of catalog entries (always accurate since eagerly loaded).
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Show the entity catalog UI.
    pub fn show(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let entries = &self.entries;

        // ── Category filter tabs ──
        ui.horizontal_wrapped(|ui| {
            let all_sel = self.selected_category.is_none();
            if ui
                .selectable_label(all_sel, format!("All ({})", entries.len()))
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
                if ui
                    .selectable_label(is_sel, format!("{} ({})", cat.label(), count))
                    .clicked()
                {
                    self.selected_category = Some(cat);
                }
            }
        });

        ui.add_space(2.0);

        // ── Pack filter dropdown ──
        ui.horizontal(|ui| {
            ui.label("Pack:");
            let selected_text = self
                .selected_pack
                .as_deref()
                .unwrap_or("All Packs");
            egui::ComboBox::from_id_salt("entity_pack_filter")
                .selected_text(selected_text)
                .width(180.0)
                .show_ui(ui, |ui| {
                    ui.set_min_width(200.0);
                    if ui
                        .selectable_label(self.selected_pack.is_none(), "All Packs")
                        .clicked()
                    {
                        self.selected_pack = None;
                    }
                    for pack in &self.packs {
                        let selected = self.selected_pack.as_ref() == Some(pack);
                        if ui.selectable_label(selected, pack).clicked() {
                            self.selected_pack = Some(pack.clone());
                        }
                    }
                });
        });

        // ── Search bar ──
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search);
            if ui.small_button("✕").clicked() {
                self.search.clear();
            }
        });

        ui.add_space(4.0);

        // ── Filter entries ──
        let search_lower = self.search.to_lowercase();
        let filtered: Vec<&CatalogEntry> = entries
            .iter()
            .filter(|e| {
                if let Some(cat) = self.selected_category {
                    if e.category != cat {
                        return false;
                    }
                }
                if let Some(ref pack) = self.selected_pack {
                    if &e.pack != pack {
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

        ui.weak(format!("Showing {} entities", filtered.len()));
        ui.add_space(2.0);

        // ── Virtual-scrolled thumbnail grid ──
        let available_width = ui.available_width();
        let cell_width = self.thumb_size + 8.0;
        let columns = ((available_width / cell_width).floor() as usize).max(1);
        let total_rows = (filtered.len() + columns - 1) / columns;
        let row_height = self.thumb_size + 20.0;
        let thumb_size = self.thumb_size;
        let mut new_spawns: Vec<(String, String)> = Vec::new();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                for row_idx in row_range {
                    ui.horizontal(|ui| {
                        let start = row_idx * columns;
                        let end = (start + columns).min(filtered.len());
                        for i in start..end {
                            let entry = filtered[i];

                            // Lazy thumbnail generation
                            if !self.thumbnails.contains_key(&entry.path) {
                                let tex = generate_thumbnail(ctx, entry);
                                self.thumbnails.insert(entry.path.clone(), tex);
                            }
                            let thumb = &self.thumbnails[&entry.path];

                            ui.vertical(|ui| {
                                ui.set_max_width(thumb_size + 4.0);
                                let img = egui::Image::new(thumb)
                                    .fit_to_exact_size(Vec2::splat(thumb_size));
                                let resp = ui.add(img.sense(Sense::click()));

                                if resp.clicked() {
                                    new_spawns.push((
                                        entry.display_name.clone(),
                                        entry.path.clone(),
                                    ));
                                }

                                if resp.hovered() {
                                    resp.on_hover_text(format!(
                                        "{}\nPack: {}\nCategory: {}\nClick to spawn",
                                        entry.display_name,
                                        entry.pack,
                                        entry.category.label()
                                    ));
                                }

                                // Label with native truncation
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&entry.display_name)
                                            .small()
                                            .color(entry.category.color()),
                                    )
                                    .truncate(),
                                );
                            });
                        }
                    });
                }
            });

        self.pending_spawns.extend(new_spawns);
    }
}
