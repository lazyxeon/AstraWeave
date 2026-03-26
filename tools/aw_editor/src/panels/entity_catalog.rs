//! Entity Catalog Panel — categorized, virtual-scrolled thumbnail grid for all GLB/GLTF models
//!
//! Scans the entire assets directory tree (KayKit collection, 3D asset packs,
//! and loose models) and presents a categorized, pack-filterable entity picker.
//! Click any thumbnail to spawn the corresponding model into the scene.

use egui::{Color32, ColorImage, ImageData, Sense, TextureHandle, Ui, Vec2};
use std::collections::{BTreeMap, HashMap, HashSet};
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

/// Check if a file extension is a supported 3D model format.
fn is_model_extension(ext: &str) -> bool {
    matches!(ext, "glb" | "gltf" | "fbx" | "obj")
}

/// Build the full entity catalog by scanning all asset directories.
pub fn build_catalog() -> Vec<CatalogEntry> {
    let mut entries = Vec::new();
    let assets_root = PathBuf::from("assets");

    // 1. KayKit character collection
    let kaykit_root = assets_root.join("The Complete KayKit Collection v4");
    if kaykit_root.is_dir() {
        scan_kaykit(&kaykit_root, &mut entries);
    }

    // 2. All 3D asset packs (GLB + GLTF + FBX format directories)
    let assets_3d = assets_root.join("3D assets");
    if assets_3d.is_dir() {
        scan_3d_assets(&assets_3d, &mut entries);
    }

    // 3. Loose models directory (GLB, GLTF, FBX, OBJ)
    let models_dir = assets_root.join("models");
    if models_dir.is_dir() {
        scan_models_dir(&models_dir, &mut entries);
    }

    // 4. Castles & Forts asset pack
    let castles = assets_root.join("castles_forts_asset_pack");
    if castles.is_dir() {
        scan_generic_pack(&castles, "Castles & Forts", EntityCategory::Building, &mut entries);
    }

    // 5. Road to Vostok survival props (FBX-heavy pack)
    let vostok = assets_root.join("Road to Vostok Assets Vol.1");
    if vostok.is_dir() {
        scan_vostok_pack(&vostok, &mut entries);
    }

    // 6. Loose terrain/material GLTF at assets root (PolyHaven terrain scans)
    if let Ok(read) = std::fs::read_dir(&assets_root) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if is_model_extension(ext) {
                        let stem = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let display = stem.replace('_', " ");
                        entries.push(CatalogEntry {
                            display_name: display,
                            path: path.display().to_string(),
                            category: EntityCategory::Nature,
                            pack: "Terrain Scans".to_string(),
                        });
                    }
                }
            }
        }
    }

    // 7. Auto-discover any remaining top-level directories with 3D models
    //    that we haven't already covered above.
    let known_dirs: std::collections::HashSet<&str> = [
        "3D assets",
        "The Complete KayKit Collection v4",
        "models",
        "castles_forts_asset_pack",
        "Road to Vostok Assets Vol.1",
        // Non-model dirs to skip:
        "2D assets", "UI assets", "audio", "materials", "textures", "Texture",
        "shaders", "cells", "cinematics", "hdri", "Icons", "imported", "navmesh",
        "navmeshes", "npc", "tests", "exemplars", "Archive", "Symphonie", "Other",
        "Goodies", "Mesh", "assets_src",
    ].iter().copied().collect();

    if let Ok(read) = std::fs::read_dir(&assets_root) {
        for entry in read.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if known_dirs.contains(dir_name.as_str()) {
                continue;
            }
            // Check if this directory has any model files
            let has_models = walkdir_has_models(&path);
            if has_models {
                let pack_name = dir_name.clone();
                let category = classify_pack(&pack_name);
                scan_generic_pack(&path, &pack_name, category, &mut entries);
            }
        }
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
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if is_model_extension(ext) {
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

        // Scan GLB, GLTF, and FBX format directories under Models/
        for subdir in &["GLB format", "GLTF format", "FBX format", "OBJ format"] {
            let model_dir = pack_path.join("Models").join(subdir);
            if !model_dir.is_dir() {
                continue;
            }
            let Ok(files) = std::fs::read_dir(&model_dir) else {
                continue;
            };

            for file in files.flatten() {
                let file_path = file.path();
                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                    if is_model_extension(ext) {
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

        // Also scan pack root and Models/ directly for loose model files
        for scan_dir in [&pack_path, &pack_path.join("Models")] {
            if !scan_dir.is_dir() {
                continue;
            }
            let Ok(files) = std::fs::read_dir(scan_dir) else {
                continue;
            };
            for file in files.flatten() {
                let file_path = file.path();
                if file_path.is_dir() {
                    continue;
                }
                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                    if is_model_extension(ext) {
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
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if is_model_extension(ext) {
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

/// Scan a Vostok-style asset pack (each subfolder = one FBX model with textures).
fn scan_vostok_pack(root: &Path, entries: &mut Vec<CatalogEntry>) {
    let Ok(dirs) = std::fs::read_dir(root) else {
        return;
    };

    for dir_entry in dirs.flatten() {
        let dir_path = dir_entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        let folder_name = dir_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Each subfolder contains FBX files
        let Ok(files) = std::fs::read_dir(&dir_path) else {
            continue;
        };
        for file in files.flatten() {
            let file_path = file.path();
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                if is_model_extension(ext) {
                    let stem = file_path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    // Strip "MS_" prefix common in this pack
                    let display = stem
                        .strip_prefix("MS_")
                        .unwrap_or(&stem)
                        .replace('_', " ");
                    let category = classify_vostok_entry(&folder_name);
                    entries.push(CatalogEntry {
                        display_name: display,
                        path: file_path.display().to_string(),
                        category,
                        pack: "Road to Vostok".to_string(),
                    });
                }
            }
        }
    }
}

/// Classify a Vostok asset by its folder name.
fn classify_vostok_entry(folder_name: &str) -> EntityCategory {
    let f = folder_name.to_lowercase();
    if f.contains("fence") || f.contains("barrier") || f.contains("pole") || f.contains("sign") {
        return EntityCategory::Infrastructure;
    }
    if f.contains("cabinet") || f.contains("fridge") || f.contains("sofa")
        || f.contains("table") || f.contains("chair") || f.contains("mattress")
        || f.contains("radiator") || f.contains("television") || f.contains("radio")
    {
        return EntityCategory::Furniture;
    }
    if f.contains("campfire") || f.contains("firewood") || f.contains("firepot")
        || f.contains("fireplace") || f.contains("candle")
    {
        return EntityCategory::Nature;
    }
    if f.contains("bus_stop") || f.contains("transformer") || f.contains("control_box") {
        return EntityCategory::Building;
    }
    EntityCategory::Prop
}

/// Scan a generic pack recursively for all model files.
fn scan_generic_pack(root: &Path, pack_name: &str, category: EntityCategory, entries: &mut Vec<CatalogEntry>) {
    scan_generic_recursive(root, pack_name, category, entries);
}

fn scan_generic_recursive(dir: &Path, pack_name: &str, category: EntityCategory, entries: &mut Vec<CatalogEntry>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_generic_recursive(&path, pack_name, category, entries);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if is_model_extension(ext) {
                let stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let display = stem.replace('_', " ");
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

/// Quick check if a directory tree contains any model files (stops at first match).
fn walkdir_has_models(dir: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if walkdir_has_models(&path) {
                return true;
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if is_model_extension(ext) {
                return true;
            }
        }
    }
    false
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
    /// Which categories are expanded in the collapsible view.
    expanded_categories: HashSet<EntityCategory>,
    /// Which pack sub-sections are expanded (category, pack) → open.
    expanded_packs: HashSet<(EntityCategory, String)>,
}

impl EntityCatalogState {
    pub fn new() -> Self {
        let entries = build_catalog();
        tracing::info!("Entity catalog: {} entries loaded", entries.len());

        // Collect unique sorted pack names
        let mut packs: Vec<String> = entries.iter().map(|e| e.pack.clone()).collect();
        packs.sort();
        packs.dedup();

        // Start with all categories expanded
        let expanded_categories: HashSet<EntityCategory> =
            EntityCategory::all().iter().copied().collect();

        Self {
            entries,
            packs,
            selected_category: None,
            selected_pack: None,
            search: String::new(),
            thumbnails: HashMap::new(),
            pending_spawns: Vec::new(),
            thumb_size: 72.0,
            expanded_categories,
            expanded_packs: HashSet::new(),
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

    /// Show the entity catalog UI with collapsible category sections.
    pub fn show(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let entries = &self.entries;

        // ── Search bar ──
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search);
            if ui.small_button("✕").clicked() {
                self.search.clear();
            }
        });

        // ── Pack filter dropdown ──
        ui.horizontal(|ui| {
            ui.label("Pack:");
            let selected_text = self
                .selected_pack
                .as_deref()
                .unwrap_or("All Packs");
            egui::ComboBox::from_id_salt("entity_pack_filter")
                .selected_text(selected_text)
                .width(160.0)
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

        // ── Quick expand/collapse ──
        ui.horizontal(|ui| {
            if ui.small_button("Expand All").clicked() {
                for &cat in EntityCategory::all() {
                    self.expanded_categories.insert(cat);
                }
            }
            if ui.small_button("Collapse All").clicked() {
                self.expanded_categories.clear();
                self.expanded_packs.clear();
            }
        });

        ui.add_space(2.0);

        // ── Filter entries ──
        let search_lower = self.search.to_lowercase();
        let filtered: Vec<&CatalogEntry> = entries
            .iter()
            .filter(|e| {
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

        ui.weak(format!("Showing {} of {} entities", filtered.len(), entries.len()));
        ui.add_space(2.0);

        // ── Group filtered entries by category → pack ──
        let mut by_category: BTreeMap<EntityCategory, BTreeMap<String, Vec<&CatalogEntry>>> =
            BTreeMap::new();
        for entry in &filtered {
            by_category
                .entry(entry.category)
                .or_default()
                .entry(entry.pack.clone())
                .or_default()
                .push(entry);
        }

        let thumb_size = self.thumb_size;
        let mut new_spawns: Vec<(String, String)> = Vec::new();

        // ── Render collapsible category sections ──
        for (category, packs_map) in &by_category {
            let cat_count: usize = packs_map.values().map(|v| v.len()).sum();
            let is_expanded = self.expanded_categories.contains(category);

            // Category header with colored indicator
            let header_text = format!("{} ({}) ", category.label(), cat_count);
            let header_response = ui.horizontal(|ui| {
                let arrow = if is_expanded { "v" } else { ">" };
                let btn = ui.button(
                    egui::RichText::new(format!("{} {}", arrow, header_text))
                        .strong()
                        .color(category.color()),
                );
                btn.clicked()
            });

            if header_response.inner {
                if is_expanded {
                    self.expanded_categories.remove(category);
                } else {
                    self.expanded_categories.insert(*category);
                }
            }

            if !self.expanded_categories.contains(category) {
                continue;
            }

            // If there's only one pack in this category, skip the sub-dropdown
            let single_pack = packs_map.len() == 1;

            for (pack_name, pack_entries) in packs_map {
                let pack_key = (*category, pack_name.clone());

                if !single_pack {
                    // Pack sub-section dropdown
                    let pack_expanded = self.expanded_packs.contains(&pack_key);
                    let pack_header = format!("  {} ({})", pack_name, pack_entries.len());
                    let pack_arrow = if pack_expanded { "v" } else { ">" };
                    if ui
                        .button(
                            egui::RichText::new(format!("  {} {}", pack_arrow, pack_header))
                                .small()
                                .color(Color32::from_gray(180)),
                        )
                        .clicked()
                    {
                        if pack_expanded {
                            self.expanded_packs.remove(&pack_key);
                        } else {
                            self.expanded_packs.insert(pack_key.clone());
                        }
                    }

                    if !self.expanded_packs.contains(&pack_key) {
                        continue;
                    }
                }

                // Render thumbnail grid for this group
                let available_width = ui.available_width();
                let cell_width = thumb_size + 8.0;
                let columns = ((available_width / cell_width).floor() as usize).max(1);
                let rows = (pack_entries.len() + columns - 1) / columns;

                for row_idx in 0..rows {
                    ui.horizontal(|ui| {
                        let start = row_idx * columns;
                        let end = (start + columns).min(pack_entries.len());
                        for i in start..end {
                            let entry = pack_entries[i];

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
            }

            ui.add_space(2.0);
        }

        self.pending_spawns.extend(new_spawns);
    }
}
