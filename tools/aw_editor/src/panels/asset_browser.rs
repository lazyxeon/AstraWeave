use egui::{ColorImage, ImageData, ScrollArea, TextureHandle, Ui};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// TEXTURE TYPE - Classification of texture maps for PBR workflow
// ============================================================================

/// Texture types for PBR (Physically Based Rendering) material workflow
/// Detected automatically from filename suffixes (e.g., _normal, _albedo, _orm)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
#[non_exhaustive]
pub enum TextureType {
    /// Base color / diffuse map
    Albedo,
    /// Normal map (tangent space)
    Normal,
    /// Combined Occlusion-Roughness-Metallic map (RGB channels)
    ORM,
    /// Combined Metallic-Roughness-AO map (RGB channels)
    MRA,
    /// Roughness map (grayscale)
    Roughness,
    /// Metallic map (grayscale)
    Metallic,
    /// Ambient Occlusion map
    AO,
    /// Emissive/glow map
    Emission,
    /// Height/displacement map
    Height,
    /// Unknown or unclassified texture
    Unknown,
}

impl std::fmt::Display for TextureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl TextureType {
    /// Get all texture types
    pub fn all() -> &'static [TextureType] {
        &[
            TextureType::Albedo,
            TextureType::Normal,
            TextureType::ORM,
            TextureType::MRA,
            TextureType::Roughness,
            TextureType::Metallic,
            TextureType::AO,
            TextureType::Emission,
            TextureType::Height,
            TextureType::Unknown,
        ]
    }

    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            TextureType::Albedo => "Albedo",
            TextureType::Normal => "Normal",
            TextureType::ORM => "ORM",
            TextureType::MRA => "MRA",
            TextureType::Roughness => "Roughness",
            TextureType::Metallic => "Metallic",
            TextureType::AO => "Ambient Occlusion",
            TextureType::Emission => "Emission",
            TextureType::Height => "Height",
            TextureType::Unknown => "Unknown",
        }
    }

    /// Check if this is a PBR component texture
    pub fn is_pbr_component(&self) -> bool {
        !matches!(self, TextureType::Unknown)
    }

    /// Check if this is a packed/combined texture
    pub fn is_packed(&self) -> bool {
        matches!(self, TextureType::ORM | TextureType::MRA)
    }

    /// Detect texture type from filename using common naming conventions
    /// Supports: _n, _normal, _nrm, _orm, _mra, _r, _rough, _roughness,
    /// _m, _metal, _metallic, _ao, _occlusion, _e, _emit, _emission,
    /// _h, _height, _disp, _displacement, _albedo, _diffuse, _basecolor, _color
    pub fn from_filename(name: &str) -> Self {
        let stem = Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check suffixes in order of specificity
        if stem.ends_with("_n")
            || stem.ends_with("_normal")
            || stem.ends_with("_nrm")
            || stem.ends_with("_nor")
        {
            TextureType::Normal
        } else if stem.ends_with("_orm") {
            TextureType::ORM
        } else if stem.ends_with("_mra") {
            TextureType::MRA
        } else if stem.ends_with("_r") || stem.ends_with("_rough") || stem.ends_with("_roughness") {
            TextureType::Roughness
        } else if stem.ends_with("_m")
            || stem.ends_with("_metal")
            || stem.ends_with("_metallic")
            || stem.ends_with("_metalness")
        {
            TextureType::Metallic
        } else if stem.ends_with("_ao") || stem.ends_with("_occlusion") {
            TextureType::AO
        } else if stem.ends_with("_e")
            || stem.ends_with("_emit")
            || stem.ends_with("_emission")
            || stem.ends_with("_emissive")
            || stem.ends_with("_glow")
        {
            TextureType::Emission
        } else if stem.ends_with("_h")
            || stem.ends_with("_height")
            || stem.ends_with("_disp")
            || stem.ends_with("_displacement")
            || stem.ends_with("_bump")
        {
            TextureType::Height
        } else if stem.ends_with("_albedo")
            || stem.ends_with("_diffuse")
            || stem.ends_with("_basecolor")
            || stem.ends_with("_base_color")
            || stem.ends_with("_color")
            || stem.ends_with("_col")
            || stem.ends_with("_d")
        {
            TextureType::Albedo
        } else {
            TextureType::Unknown
        }
    }

    /// Icon for display in the UI
    pub fn icon(&self) -> &'static str {
        match self {
            TextureType::Albedo => "🎨",
            TextureType::Normal => "🔵",
            TextureType::ORM => "🔶",
            TextureType::MRA => "🟣",
            TextureType::Roughness => "◽",
            TextureType::Metallic => "⬜",
            TextureType::AO => "⬛",
            TextureType::Emission => "✨",
            TextureType::Height => "📐",
            TextureType::Unknown => "❓",
        }
    }

    /// Display label for the texture type
    pub fn label(&self) -> &'static str {
        match self {
            TextureType::Albedo => "Albedo",
            TextureType::Normal => "Normal",
            TextureType::ORM => "ORM",
            TextureType::MRA => "MRA",
            TextureType::Roughness => "Rough",
            TextureType::Metallic => "Metal",
            TextureType::AO => "AO",
            TextureType::Emission => "Emit",
            TextureType::Height => "Height",
            TextureType::Unknown => "???",
        }
    }

    /// Color for UI badges
    pub fn color(&self) -> egui::Color32 {
        match self {
            TextureType::Albedo => egui::Color32::from_rgb(255, 180, 100),
            TextureType::Normal => egui::Color32::from_rgb(128, 128, 255),
            TextureType::ORM => egui::Color32::from_rgb(255, 165, 0),
            TextureType::MRA => egui::Color32::from_rgb(100, 149, 237),
            TextureType::Roughness => egui::Color32::from_rgb(180, 180, 180),
            TextureType::Metallic => egui::Color32::from_rgb(220, 220, 255),
            TextureType::AO => egui::Color32::from_rgb(80, 80, 80),
            TextureType::Emission => egui::Color32::from_rgb(255, 255, 100),
            TextureType::Height => egui::Color32::from_rgb(150, 100, 200),
            TextureType::Unknown => egui::Color32::from_rgb(128, 128, 128),
        }
    }
}

// ============================================================================
// ASSET CATEGORY - High-level organization for asset browser
// ============================================================================

/// Asset categories for organizing the asset browser
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AssetCategory {
    All,
    Models,
    Textures,
    Materials,
    Prefabs,
    Scenes,
    Audio,
    Configs,
}

impl std::fmt::Display for AssetCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AssetCategory {
    /// Get all asset categories
    pub fn all() -> &'static [AssetCategory] {
        &[
            AssetCategory::All,
            AssetCategory::Models,
            AssetCategory::Textures,
            AssetCategory::Materials,
            AssetCategory::Prefabs,
            AssetCategory::Scenes,
            AssetCategory::Audio,
            AssetCategory::Configs,
        ]
    }

    /// Get the display name
    pub fn name(&self) -> &'static str {
        self.label()
    }

    /// Check if an asset type matches this category
    pub fn matches(&self, asset_type: &AssetType) -> bool {
        match self {
            AssetCategory::All => true,
            AssetCategory::Models => *asset_type == AssetType::Model,
            AssetCategory::Textures => *asset_type == AssetType::Texture,
            AssetCategory::Materials => *asset_type == AssetType::Material,
            AssetCategory::Prefabs => *asset_type == AssetType::Prefab,
            AssetCategory::Scenes => *asset_type == AssetType::Scene,
            AssetCategory::Audio => *asset_type == AssetType::Audio,
            AssetCategory::Configs => *asset_type == AssetType::Config,
        }
    }

    /// Icon for category button
    pub fn icon(&self) -> &'static str {
        match self {
            AssetCategory::All => "📂",
            AssetCategory::Models => "🎲",
            AssetCategory::Textures => "🖼",
            AssetCategory::Materials => "💎",
            AssetCategory::Prefabs => "📦",
            AssetCategory::Scenes => "🌐",
            AssetCategory::Audio => "🎵",
            AssetCategory::Configs => "⚙",
        }
    }

    /// Display label
    pub fn label(&self) -> &'static str {
        match self {
            AssetCategory::All => "All",
            AssetCategory::Models => "Models",
            AssetCategory::Textures => "Textures",
            AssetCategory::Materials => "Materials",
            AssetCategory::Prefabs => "Prefabs",
            AssetCategory::Scenes => "Scenes",
            AssetCategory::Audio => "Audio",
            AssetCategory::Configs => "Configs",
        }
    }
}

// ============================================================================
// ASSET ACTION - Actions that can be performed on assets
// ============================================================================

/// Actions triggered from the asset browser for processing by the editor
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AssetAction {
    /// Import a 3D model into the scene as a new entity
    ImportModel { path: PathBuf },
    /// Load a 3D model directly to viewport for preview (no entity created)
    LoadToViewport { path: PathBuf },
    /// Apply a texture to the selected entity's material
    ApplyTexture {
        path: PathBuf,
        texture_type: TextureType,
    },
    /// Apply a material file to the selected entity
    ApplyMaterial { path: PathBuf },
    /// Load a scene file
    LoadScene { path: PathBuf },
    /// Spawn a prefab as a new entity
    SpawnPrefab { path: PathBuf },
    /// Open asset in external application
    OpenExternal { path: PathBuf },
    /// Inspect asset details (for material inspector panel)
    InspectAsset { path: PathBuf },
}

impl std::fmt::Display for AssetAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AssetAction {
    /// Returns all action type variant names
    pub fn all_variants() -> &'static [&'static str] {
        &[
            "ImportModel",
            "LoadToViewport",
            "ApplyTexture",
            "ApplyMaterial",
            "LoadScene",
            "SpawnPrefab",
            "OpenExternal",
            "InspectAsset",
        ]
    }

    /// Returns the name of this action type
    pub fn name(&self) -> &'static str {
        match self {
            AssetAction::ImportModel { .. } => "Import Model",
            AssetAction::LoadToViewport { .. } => "Load to Viewport",
            AssetAction::ApplyTexture { .. } => "Apply Texture",
            AssetAction::ApplyMaterial { .. } => "Apply Material",
            AssetAction::LoadScene { .. } => "Load Scene",
            AssetAction::SpawnPrefab { .. } => "Spawn Prefab",
            AssetAction::OpenExternal { .. } => "Open External",
            AssetAction::InspectAsset { .. } => "Inspect Asset",
        }
    }

    /// Returns the icon for this action type
    pub fn icon(&self) -> &'static str {
        match self {
            AssetAction::ImportModel { .. } => "📥",
            AssetAction::LoadToViewport { .. } => "👁",
            AssetAction::ApplyTexture { .. } => "🎨",
            AssetAction::ApplyMaterial { .. } => "💎",
            AssetAction::LoadScene { .. } => "🌐",
            AssetAction::SpawnPrefab { .. } => "📦",
            AssetAction::OpenExternal { .. } => "🔗",
            AssetAction::InspectAsset { .. } => "🔍",
        }
    }

    /// Returns the path associated with this action
    pub fn path(&self) -> &PathBuf {
        match self {
            AssetAction::ImportModel { path } => path,
            AssetAction::LoadToViewport { path } => path,
            AssetAction::ApplyTexture { path, .. } => path,
            AssetAction::ApplyMaterial { path } => path,
            AssetAction::LoadScene { path } => path,
            AssetAction::SpawnPrefab { path } => path,
            AssetAction::OpenExternal { path } => path,
            AssetAction::InspectAsset { path } => path,
        }
    }

    /// Returns true if this action modifies scene content
    pub fn is_modifying(&self) -> bool {
        matches!(
            self,
            AssetAction::ImportModel { .. }
                | AssetAction::ApplyTexture { .. }
                | AssetAction::ApplyMaterial { .. }
                | AssetAction::SpawnPrefab { .. }
        )
    }

    /// Returns true if this is a load/view action (no scene modification)
    pub fn is_viewing(&self) -> bool {
        matches!(
            self,
            AssetAction::LoadToViewport { .. }
                | AssetAction::OpenExternal { .. }
                | AssetAction::InspectAsset { .. }
        )
    }

    /// Returns true if this action loads a scene
    pub fn is_scene_action(&self) -> bool {
        matches!(self, AssetAction::LoadScene { .. })
    }
}

// ============================================================================
// ASSET TYPE - Basic asset classification by file type
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AssetType {
    Model,
    Texture,
    Scene,
    Material,
    Audio,
    Config,
    Prefab,
    Directory,
    Unknown,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AssetType {
    /// Get all asset types
    pub fn all() -> &'static [AssetType] {
        &[
            AssetType::Model,
            AssetType::Texture,
            AssetType::Scene,
            AssetType::Material,
            AssetType::Audio,
            AssetType::Config,
            AssetType::Prefab,
            AssetType::Directory,
            AssetType::Unknown,
        ]
    }

    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            AssetType::Model => "Model",
            AssetType::Texture => "Texture",
            AssetType::Scene => "Scene",
            AssetType::Material => "Material",
            AssetType::Audio => "Audio",
            AssetType::Config => "Config",
            AssetType::Prefab => "Prefab",
            AssetType::Directory => "Directory",
            AssetType::Unknown => "Unknown",
        }
    }

    /// Check if this is a content asset (not config/directory)
    pub fn is_content(&self) -> bool {
        matches!(
            self,
            AssetType::Model
                | AssetType::Texture
                | AssetType::Scene
                | AssetType::Material
                | AssetType::Audio
                | AssetType::Prefab
        )
    }

    pub fn from_path(path: &Path) -> Self {
        if path.is_dir() {
            return AssetType::Directory;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if file_name.ends_with(".prefab.ron") {
            return AssetType::Prefab;
        }

        match path.extension().and_then(|e| e.to_str()) {
            Some("glb") | Some("gltf") | Some("obj") | Some("fbx") => AssetType::Model,
            Some("png") | Some("jpg") | Some("jpeg") | Some("ktx2") | Some("dds") => {
                AssetType::Texture
            }
            Some("ron") => AssetType::Scene,
            Some("toml") | Some("json") => AssetType::Config,
            Some("wav") | Some("ogg") | Some("mp3") => AssetType::Audio,
            _ => AssetType::Unknown,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AssetType::Model => "🎲",
            AssetType::Texture => "🖼",
            AssetType::Scene => "🌐",
            AssetType::Material => "💎",
            AssetType::Audio => "🎵",
            AssetType::Config => "⚙",
            AssetType::Prefab => "📦",
            AssetType::Directory => "📁",
            AssetType::Unknown => "📄",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            AssetType::Model => egui::Color32::from_rgb(100, 200, 255),
            AssetType::Texture => egui::Color32::from_rgb(255, 150, 100),
            AssetType::Scene => egui::Color32::from_rgb(100, 255, 100),
            AssetType::Material => egui::Color32::from_rgb(200, 100, 255),
            AssetType::Audio => egui::Color32::from_rgb(255, 255, 100),
            AssetType::Config => egui::Color32::from_rgb(200, 200, 200),
            AssetType::Prefab => egui::Color32::from_rgb(150, 200, 255),
            AssetType::Directory => egui::Color32::from_rgb(255, 200, 100),
            AssetType::Unknown => egui::Color32::from_rgb(150, 150, 150),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssetEntry {
    pub path: PathBuf,
    pub name: String,
    /// Human-readable display name (e.g., "Bridge Center Stone" from "bridge_center_stone.glb")
    pub display_name: String,
    pub asset_type: AssetType,
    pub texture_type: Option<TextureType>,
    pub size: u64,
}

/// Convert a file/directory name to a pretty display name.
/// "bridge_center_stone.glb" → "Bridge Center Stone"
/// "Nature Kit" → "Nature Kit" (already pretty)
fn prettify_name(name: &str) -> String {
    // Strip file extension
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);
    // Replace underscores/hyphens with spaces, then title-case each word
    stem.replace('_', " ")
        .replace('-', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl AssetEntry {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let asset_type = AssetType::from_path(&path);
        let display_name = prettify_name(&name);

        // Detect texture type if this is a texture
        let texture_type = if asset_type == AssetType::Texture {
            Some(TextureType::from_filename(&name))
        } else {
            None
        };

        let size = if path.is_file() {
            fs::metadata(&path).ok()?.len()
        } else {
            0
        };

        Some(AssetEntry {
            path,
            name,
            display_name,
            asset_type,
            texture_type,
            size,
        })
    }

    pub fn format_size(&self) -> String {
        if self.asset_type == AssetType::Directory {
            return String::new();
        }

        let size_kb = self.size as f64 / 1024.0;
        if size_kb < 1024.0 {
            format!("{:.1} KB", size_kb)
        } else {
            format!("{:.1} MB", size_kb / 1024.0)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ViewMode {
    List,
    Grid,
}

impl std::fmt::Display for ViewMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ViewMode {
    /// Get all view modes
    pub fn all() -> &'static [ViewMode] {
        &[ViewMode::List, ViewMode::Grid]
    }

    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            ViewMode::List => "List",
            ViewMode::Grid => "Grid",
        }
    }

    /// Get the icon
    pub fn icon(&self) -> &'static str {
        match self {
            ViewMode::List => "📋",
            ViewMode::Grid => "📰",
        }
    }
}

pub struct AssetBrowser {
    root_path: PathBuf,
    current_path: PathBuf,
    entries: Vec<AssetEntry>,
    selected_asset: Option<PathBuf>,
    show_hidden: bool,
    filter_type: Option<AssetType>,
    search_query: String,
    view_mode: ViewMode,
    thumbnail_cache: HashMap<PathBuf, TextureHandle>,
    thumbnail_lru: VecDeque<PathBuf>,
    max_cache_size: usize,
    thumbnail_size: f32,
    dragged_prefab: Option<PathBuf>,
    // New fields for enhanced organization
    category_filter: AssetCategory,
    texture_type_filter: Option<TextureType>,
    pending_actions: Vec<AssetAction>,
    show_texture_badges: bool,
}

impl AssetBrowser {
    pub fn new(root_path: PathBuf) -> Self {
        // Canonicalize the root path to ensure consistent path operations
        let canonical_root = root_path.canonicalize().unwrap_or(root_path);
        Self {
            root_path: canonical_root.clone(),
            current_path: canonical_root,
            entries: Vec::new(),
            selected_asset: None,
            show_hidden: false,
            filter_type: None,
            search_query: String::new(),
            view_mode: ViewMode::Grid, // Default to grid for better visual browsing
            thumbnail_cache: HashMap::new(),
            thumbnail_lru: VecDeque::new(),
            max_cache_size: 100,  // Limit cache to 100 textures
            thumbnail_size: 80.0, // Slightly larger for better visibility
            dragged_prefab: None,
            category_filter: AssetCategory::All,
            texture_type_filter: None,
            pending_actions: Vec::new(),
            show_texture_badges: true,
        }
        // Deferred: scan_current_directory() runs lazily on first show() to avoid blocking startup
    }

    /// Take any pending asset actions for processing by main editor
    pub fn take_pending_actions(&mut self) -> Vec<AssetAction> {
        std::mem::take(&mut self.pending_actions)
    }

    pub fn take_dragged_prefab(&mut self) -> Option<PathBuf> {
        self.dragged_prefab.take()
    }

    fn scan_current_directory(&mut self) {
        self.entries.clear();

        let Ok(read_dir) = fs::read_dir(&self.current_path) else {
            return;
        };

        let mut entries: Vec<AssetEntry> = read_dir
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path().canonicalize().unwrap_or_else(|_| entry.path());
                AssetEntry::from_path(path)
            })
            .filter(|entry| {
                // Hidden file filter
                if !self.show_hidden && entry.name.starts_with('.') {
                    return false;
                }

                // Always show directories for navigation
                if entry.asset_type == AssetType::Directory {
                    return true;
                }

                // Category filter
                if !self.category_filter.matches(&entry.asset_type) {
                    return false;
                }

                // Texture type filter (only applies to textures)
                if let Some(tex_filter) = &self.texture_type_filter {
                    if entry.asset_type == AssetType::Texture
                        && entry.texture_type.as_ref() != Some(tex_filter)
                    {
                        return false;
                    }
                }

                // Legacy filter_type support
                if let Some(filter) = &self.filter_type {
                    if &entry.asset_type != filter {
                        return false;
                    }
                }

                // Search query filter
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    if !entry.name.to_lowercase().contains(&query)
                        && !entry.display_name.to_lowercase().contains(&query)
                    {
                        return false;
                    }
                }

                true
            })
            .collect();

        entries.sort_by(|a, b| match (&a.asset_type, &b.asset_type) {
            (AssetType::Directory, AssetType::Directory) => a.name.cmp(&b.name),
            (AssetType::Directory, _) => std::cmp::Ordering::Less,
            (_, AssetType::Directory) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        self.entries = entries;
    }

    pub fn navigate_to(&mut self, path: PathBuf) {
        // Try canonicalizing first for reliable is_dir check
        let check_path = path.canonicalize().unwrap_or(path.clone());
        if check_path.is_dir() {
            self.current_path = check_path;
            self.scan_current_directory();
        }
    }

    pub fn navigate_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            let parent_buf = parent.to_path_buf();
            if parent_buf >= self.root_path {
                self.current_path = parent_buf;
                self.scan_current_directory();
            }
        }
    }

    pub fn selected_asset(&self) -> Option<&Path> {
        self.selected_asset.as_deref()
    }

    fn load_thumbnail(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        if let Some(texture) = self.thumbnail_cache.get(path) {
            // Update LRU: move to back
            if let Some(pos) = self.thumbnail_lru.iter().position(|p| p == path) {
                if let Some(p) = self.thumbnail_lru.remove(pos) {
                    self.thumbnail_lru.push_back(p);
                }
            }
            return Some(texture.clone());
        }

        let asset_type = AssetType::from_path(path);

        // Determine the image file to load as thumbnail
        let image_path = match asset_type {
            AssetType::Texture => {
                // Only attempt to load standard image formats
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                if !matches!(
                    ext.as_str(),
                    "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tga" | "tiff"
                ) {
                    return None;
                }
                path.to_path_buf()
            }
            AssetType::Directory => {
                // For directories, look for Preview.png or Sample.png (Kenney packs)
                let preview = path.join("Preview.png");
                if preview.exists() {
                    preview
                } else {
                    let sample = path.join("Sample.png");
                    if sample.exists() {
                        sample
                    } else {
                        return None;
                    }
                }
            }
            AssetType::Model | AssetType::Audio | AssetType::Material | AssetType::Prefab => {
                // Generate a colored placeholder thumbnail for non-image assets
                return self.generate_placeholder_thumbnail(ctx, path, asset_type);
            }
            _ => return None,
        };

        let image_data = match image::open(&image_path) {
            Ok(img) => img,
            Err(_) => return None,
        };
        let rgba = image_data.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.into_raw();

        let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);

        let texture = ctx.load_texture(
            path.display().to_string(),
            ImageData::Color(std::sync::Arc::new(color_image)),
            egui::TextureOptions::LINEAR,
        );

        // Evict oldest if cache full
        if self.thumbnail_cache.len() >= self.max_cache_size {
            if let Some(oldest) = self.thumbnail_lru.pop_front() {
                self.thumbnail_cache.remove(&oldest);
            }
        }

        self.thumbnail_cache
            .insert(path.to_path_buf(), texture.clone());
        self.thumbnail_lru.push_back(path.to_path_buf());

        Some(texture)
    }

    /// Generate a colored placeholder thumbnail for non-image assets (models, audio, etc.)
    fn generate_placeholder_thumbnail(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        asset_type: AssetType,
    ) -> Option<TextureHandle> {
        let size = 64usize;
        let mut pixels = vec![0u8; size * size * 4];
        let color = asset_type.color();
        let [r, g, b, _] = color.to_array();

        // Fill with a darker version of the type color for the background
        let bg_r = (r as u16 * 40 / 100) as u8;
        let bg_g = (g as u16 * 40 / 100) as u8;
        let bg_b = (b as u16 * 40 / 100) as u8;
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[0] = bg_r;
            pixel[1] = bg_g;
            pixel[2] = bg_b;
            pixel[3] = 255;
        }

        // Draw a centered icon-like shape based on asset type
        let center = size / 2;
        let radius = size / 4;
        match asset_type {
            AssetType::Model => {
                // Draw a diamond shape for 3D models
                for y in 0..size {
                    for x in 0..size {
                        let dx = (x as i32 - center as i32).unsigned_abs() as usize;
                        let dy = (y as i32 - center as i32).unsigned_abs() as usize;
                        if dx + dy <= radius {
                            let idx = (y * size + x) * 4;
                            pixels[idx] = r;
                            pixels[idx + 1] = g;
                            pixels[idx + 2] = b;
                            pixels[idx + 3] = 255;
                        }
                    }
                }
            }
            AssetType::Audio => {
                // Draw horizontal bars for audio
                for y in 0..size {
                    let bar_h = size / 8;
                    let in_bar = (y / bar_h) % 2 == 0 && y > size / 4 && y < size * 3 / 4;
                    if in_bar {
                        for x in size / 4..size * 3 / 4 {
                            let idx = (y * size + x) * 4;
                            pixels[idx] = r;
                            pixels[idx + 1] = g;
                            pixels[idx + 2] = b;
                            pixels[idx + 3] = 255;
                        }
                    }
                }
            }
            _ => {
                // Draw a simple circle for other types
                for y in 0..size {
                    for x in 0..size {
                        let dx = x as i32 - center as i32;
                        let dy = y as i32 - center as i32;
                        if (dx * dx + dy * dy) <= (radius * radius) as i32 {
                            let idx = (y * size + x) * 4;
                            pixels[idx] = r;
                            pixels[idx + 1] = g;
                            pixels[idx + 2] = b;
                            pixels[idx + 3] = 255;
                        }
                    }
                }
            }
        }

        let color_image = ColorImage::from_rgba_unmultiplied([size, size], &pixels);
        let texture = ctx.load_texture(
            path.display().to_string(),
            ImageData::Color(std::sync::Arc::new(color_image)),
            egui::TextureOptions::LINEAR,
        );

        if self.thumbnail_cache.len() >= self.max_cache_size {
            if let Some(oldest) = self.thumbnail_lru.pop_front() {
                self.thumbnail_cache.remove(&oldest);
            }
        }

        self.thumbnail_cache
            .insert(path.to_path_buf(), texture.clone());
        self.thumbnail_lru.push_back(path.to_path_buf());

        Some(texture)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        // Lazy-load entries on first display to avoid blocking startup
        if self.entries.is_empty() && self.search_query.is_empty() {
            self.scan_current_directory();
        }

        ui.heading("Asset Browser");
        ui.separator();

        // Navigation bar
        ui.horizontal(|ui| {
            if ui.button("[Up] Up").clicked() {
                self.navigate_up();
            }

            if ui.button("Root").clicked() {
                self.navigate_to(self.root_path.clone());
            }

            ui.separator();

            ui.label("[Srch]");
            if ui.text_edit_singleline(&mut self.search_query).changed() {
                self.scan_current_directory();
            }

            ui.separator();

            if ui
                .selectable_label(self.view_mode == ViewMode::List, "List")
                .clicked()
            {
                self.view_mode = ViewMode::List;
            }
            if ui
                .selectable_label(self.view_mode == ViewMode::Grid, "Grid")
                .clicked()
            {
                self.view_mode = ViewMode::Grid;
            }

            ui.separator();

            ui.checkbox(&mut self.show_texture_badges, "[Tag]");
            if ui
                .small_button("⚙️")
                .on_hover_text("Thumbnail size")
                .clicked()
            {
                // Toggle between size presets
                self.thumbnail_size = match self.thumbnail_size as i32 {
                    64 => 80.0,
                    80 => 100.0,
                    100 => 120.0,
                    _ => 64.0,
                };
            }
        });

        // Category filter bar
        ui.horizontal(|ui| {
            ui.label("Category:");

            let categories = [
                AssetCategory::All,
                AssetCategory::Models,
                AssetCategory::Textures,
                AssetCategory::Materials,
                AssetCategory::Prefabs,
                AssetCategory::Scenes,
                AssetCategory::Audio,
                AssetCategory::Configs,
            ];

            for cat in categories {
                let label = format!("{} {}", cat.icon(), cat.label());
                if ui
                    .selectable_label(self.category_filter == cat, label)
                    .clicked()
                {
                    self.category_filter = cat;
                    // Clear texture type filter when changing category
                    if cat != AssetCategory::Textures {
                        self.texture_type_filter = None;
                    }
                    self.scan_current_directory();
                }
            }
        });

        // Texture type sub-filter (only when Textures category selected)
        if self.category_filter == AssetCategory::Textures {
            ui.horizontal(|ui| {
                ui.label("Type:");

                if ui
                    .selectable_label(self.texture_type_filter.is_none(), "All")
                    .clicked()
                {
                    self.texture_type_filter = None;
                    self.scan_current_directory();
                }

                let tex_types = [
                    TextureType::Albedo,
                    TextureType::Normal,
                    TextureType::ORM,
                    TextureType::MRA,
                    TextureType::Roughness,
                    TextureType::Metallic,
                    TextureType::AO,
                    TextureType::Emission,
                    TextureType::Height,
                ];

                for tex_type in tex_types {
                    let label = format!("{} {}", tex_type.icon(), tex_type.label());
                    let is_selected = self.texture_type_filter == Some(tex_type);
                    if ui.selectable_label(is_selected, label).clicked() {
                        self.texture_type_filter = if is_selected { None } else { Some(tex_type) };
                        self.scan_current_directory();
                    }
                }
            });
        }

        // Current path display
        ui.label(format!(
            "{}",
            self.current_path
                .strip_prefix(&self.root_path)
                .unwrap_or(&self.current_path)
                .display()
        ));

        ui.separator();

        let mut path_to_navigate = None;

        match self.view_mode {
            ViewMode::List => {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().spacing.item_spacing.y = 2.0;

                        for entry in &self.entries {
                            let is_selected = self.selected_asset.as_ref() == Some(&entry.path);

                            let response = ui.selectable_label(
                                is_selected,
                                format!(
                                    "{} {} {}",
                                    entry.asset_type.icon(),
                                    entry.display_name,
                                    entry.format_size()
                                ),
                            );

                            if response.clicked() {
                                if entry.asset_type == AssetType::Directory {
                                    path_to_navigate = Some(entry.path.clone());
                                } else {
                                    self.selected_asset = Some(entry.path.clone());
                                }
                            }

                            if response.double_clicked() {
                                if entry.asset_type == AssetType::Directory {
                                    path_to_navigate = Some(entry.path.clone());
                                } else if entry.asset_type == AssetType::Model {
                                    // Double-click on model imports it to the scene
                                    self.pending_actions.push(AssetAction::ImportModel {
                                        path: entry.path.clone(),
                                    });
                                } else if entry.asset_type == AssetType::Texture {
                                    // Double-click on texture applies it to selected entity
                                    let tex_type =
                                        entry.texture_type.unwrap_or(TextureType::Albedo);
                                    self.pending_actions.push(AssetAction::ApplyTexture {
                                        path: entry.path.clone(),
                                        texture_type: tex_type,
                                    });
                                }
                            }

                            if response.hovered() {
                                response
                                    .clone()
                                    .on_hover_text(entry.path.display().to_string());
                            }

                            if matches!(entry.asset_type, AssetType::Prefab | AssetType::Model)
                                && response.drag_started()
                            {
                                self.dragged_prefab = Some(entry.path.clone());
                            }
                        }

                        if self.entries.is_empty() {
                            ui.colored_label(
                                egui::Color32::GRAY,
                                if self.search_query.is_empty() {
                                    "Empty directory"
                                } else {
                                    "No matching assets"
                                },
                            );
                        }
                    });
            }
            ViewMode::Grid => {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let item_spacing = 8.0;
                        let thumbnail_size = self.thumbnail_size;
                        let available_width = ui.available_width();
                        let items_per_row = ((available_width + item_spacing)
                            / (thumbnail_size + item_spacing))
                            .floor()
                            .max(1.0) as usize;

                        ui.style_mut().spacing.item_spacing =
                            egui::vec2(item_spacing, item_spacing);

                        for row_start in (0..self.entries.len()).step_by(items_per_row) {
                            ui.horizontal(|ui| {
                                for i in
                                    row_start..(row_start + items_per_row).min(self.entries.len())
                                {
                                    let entry = &self.entries[i];
                                    let is_selected =
                                        self.selected_asset.as_ref() == Some(&entry.path);
                                    let entry_path = entry.path.clone();
                                    let entry_display_name = entry.display_name.clone();
                                    let entry_asset_type = entry.asset_type;
                                    let entry_texture_type = entry.texture_type;
                                    let show_badges = self.show_texture_badges;

                                    // Load thumbnails for all supported asset types
                                    let ctx = ui.ctx().clone();
                                    let thumbnail = if matches!(
                                        entry_asset_type,
                                        AssetType::Texture
                                            | AssetType::Directory
                                            | AssetType::Model
                                            | AssetType::Audio
                                            | AssetType::Material
                                            | AssetType::Prefab
                                    ) {
                                        self.load_thumbnail(&ctx, &entry_path)
                                    } else {
                                        None
                                    };

                                    ui.vertical(|ui| {
                                        ui.set_width(thumbnail_size);

                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(thumbnail_size, thumbnail_size),
                                            egui::Sense::click(),
                                        );

                                        if ui.is_rect_visible(rect) {
                                            let bg_color = if is_selected {
                                                egui::Color32::from_rgb(60, 120, 180)
                                            } else if response.hovered() {
                                                egui::Color32::from_rgb(50, 50, 55)
                                            } else {
                                                egui::Color32::from_rgb(35, 35, 40)
                                            };

                                            ui.painter().rect_filled(rect, 4.0, bg_color);

                                            if let Some(texture) = thumbnail {
                                                ui.painter().image(
                                                    texture.id(),
                                                    rect.shrink(4.0),
                                                    egui::Rect::from_min_max(
                                                        egui::pos2(0.0, 0.0),
                                                        egui::pos2(1.0, 1.0),
                                                    ),
                                                    egui::Color32::WHITE,
                                                );
                                            } else {
                                                let icon_pos = rect.center();
                                                ui.painter().text(
                                                    icon_pos,
                                                    egui::Align2::CENTER_CENTER,
                                                    entry_asset_type.icon(),
                                                    egui::FontId::proportional(32.0),
                                                    entry_asset_type.color(),
                                                );
                                            }

                                            // Draw texture type badge in bottom-right corner
                                            if show_badges {
                                                if let Some(tex_type) = entry_texture_type {
                                                    let badge_size = 18.0;
                                                    let badge_rect = egui::Rect::from_min_size(
                                                        egui::pos2(
                                                            rect.max.x - badge_size - 4.0,
                                                            rect.max.y - badge_size - 4.0,
                                                        ),
                                                        egui::vec2(badge_size, badge_size),
                                                    );
                                                    ui.painter().rect_filled(
                                                        badge_rect,
                                                        3.0,
                                                        tex_type.color(),
                                                    );
                                                    ui.painter().text(
                                                        badge_rect.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        tex_type.icon(),
                                                        egui::FontId::proportional(10.0),
                                                        egui::Color32::WHITE,
                                                    );
                                                }
                                            }

                                            // Quick-add button for models (top-right corner, visible on hover)
                                            if entry_asset_type == AssetType::Model
                                                && response.hovered()
                                            {
                                                let btn_size = 22.0;
                                                let btn_rect = egui::Rect::from_min_size(
                                                    egui::pos2(
                                                        rect.max.x - btn_size - 2.0,
                                                        rect.min.y + 2.0,
                                                    ),
                                                    egui::vec2(btn_size, btn_size),
                                                );
                                                ui.painter().rect_filled(
                                                    btn_rect,
                                                    4.0,
                                                    egui::Color32::from_rgba_unmultiplied(
                                                        40, 160, 80, 220,
                                                    ),
                                                );
                                                ui.painter().text(
                                                    btn_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "+",
                                                    egui::FontId::proportional(16.0),
                                                    egui::Color32::WHITE,
                                                );
                                            }
                                        }

                                        if response.clicked() {
                                            if entry_asset_type == AssetType::Directory {
                                                path_to_navigate = Some(entry_path.clone());
                                            } else {
                                                self.selected_asset = Some(entry_path.clone());
                                            }
                                        }

                                        if response.double_clicked() {
                                            if entry_asset_type == AssetType::Directory {
                                                path_to_navigate = Some(entry_path.clone());
                                            } else if entry_asset_type == AssetType::Model {
                                                // Double-click on model imports it to the scene
                                                self.pending_actions.push(
                                                    AssetAction::ImportModel {
                                                        path: entry_path.clone(),
                                                    },
                                                );
                                            } else if entry_asset_type == AssetType::Texture {
                                                // Double-click on texture applies it to selected entity
                                                let tex_type = entry_texture_type
                                                    .unwrap_or(TextureType::Albedo);
                                                self.pending_actions.push(
                                                    AssetAction::ApplyTexture {
                                                        path: entry_path.clone(),
                                                        texture_type: tex_type,
                                                    },
                                                );
                                            }
                                        }

                                        if response.hovered() {
                                            response
                                                .clone()
                                                .on_hover_text(entry_path.display().to_string());
                                        }

                                        // Allow dragging models and prefabs to viewport
                                        if matches!(
                                            entry_asset_type,
                                            AssetType::Prefab | AssetType::Model
                                        ) && response.drag_started()
                                        {
                                            self.dragged_prefab = Some(entry_path.clone());
                                        }

                                        ui.add(
                                            egui::Label::new(&entry_display_name)
                                                .wrap_mode(egui::TextWrapMode::Truncate),
                                        );
                                    });
                                }
                            });
                        }

                        if self.entries.is_empty() {
                            ui.colored_label(
                                egui::Color32::GRAY,
                                if self.search_query.is_empty() {
                                    "Empty directory"
                                } else {
                                    "No matching assets"
                                },
                            );
                        }
                    });
            }
        }

        if let Some(path) = path_to_navigate {
            self.navigate_to(path);
        }

        // Selected asset details panel with action buttons
        if let Some(selected) = &self.selected_asset.clone() {
            ui.separator();

            let asset_type = AssetType::from_path(selected);
            let texture_type = if asset_type == AssetType::Texture {
                Some(TextureType::from_filename(
                    selected
                        .file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or(""),
                ))
            } else {
                None
            };

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(asset_type.icon());
                    ui.strong(
                        selected
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                    );
                    if let Some(tex_type) = texture_type {
                        ui.colored_label(tex_type.color(), format!("[{}]", tex_type.label()));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("[Open]");
                    ui.monospace(selected.parent().unwrap_or(selected).display().to_string());
                });

                ui.horizontal(|ui| {
                    // Context-appropriate action buttons
                    match asset_type {
                        AssetType::Model => {
                            if ui
                                .button(
                                    egui::RichText::new("📥 Add to Scene")
                                        .color(egui::Color32::from_rgb(100, 220, 140)),
                                )
                                .clicked()
                            {
                                self.pending_actions.push(AssetAction::ImportModel {
                                    path: selected.clone(),
                                });
                            }
                            if ui.button("Preview").clicked() {
                                self.pending_actions.push(AssetAction::LoadToViewport {
                                    path: selected.clone(),
                                });
                            }
                        }
                        AssetType::Texture => {
                            let tex_type = texture_type.unwrap_or(TextureType::Albedo);
                            if ui
                                .button(
                                    egui::RichText::new(format!(
                                        "🎨 Apply as {}",
                                        tex_type.label()
                                    ))
                                    .color(egui::Color32::from_rgb(100, 180, 220)),
                                )
                                .clicked()
                            {
                                self.pending_actions.push(AssetAction::ApplyTexture {
                                    path: selected.clone(),
                                    texture_type: tex_type,
                                });
                            }
                        }
                        AssetType::Material => {
                            if ui
                                .button(
                                    egui::RichText::new("💎 Apply Material")
                                        .color(egui::Color32::from_rgb(180, 140, 220)),
                                )
                                .clicked()
                            {
                                self.pending_actions.push(AssetAction::ApplyMaterial {
                                    path: selected.clone(),
                                });
                            }
                        }
                        AssetType::Scene => {
                            if ui.button("🌐 Load Scene").clicked() {
                                self.pending_actions.push(AssetAction::LoadScene {
                                    path: selected.clone(),
                                });
                            }
                        }
                        AssetType::Prefab => {
                            if ui.button("Spawn Prefab").clicked() {
                                self.pending_actions.push(AssetAction::SpawnPrefab {
                                    path: selected.clone(),
                                });
                            }
                        }
                        _ => {}
                    }

                    // Common actions
                    if ui.button("Open Folder").clicked() {
                        if let Some(parent) = selected.parent() {
                            self.pending_actions.push(AssetAction::OpenExternal {
                                path: parent.to_path_buf(),
                            });
                        }
                    }
                });
            });
        }
    }

    /// Show the selection actions panel (import/apply buttons) separately
    /// Call this OUTSIDE the collapsing/scroll area to ensure buttons are visible
    pub fn show_selection_actions(&mut self, ui: &mut Ui) {
        if let Some(selected) = &self.selected_asset.clone() {
            let asset_type = AssetType::from_path(selected);
            let texture_type = if asset_type == AssetType::Texture {
                Some(TextureType::from_filename(
                    selected
                        .file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or(""),
                ))
            } else {
                None
            };

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(asset_type.icon());
                    ui.strong(
                        selected
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                    );
                    if let Some(tex_type) = texture_type {
                        ui.colored_label(tex_type.color(), format!("[{}]", tex_type.label()));
                    }
                });

                ui.horizontal(|ui| {
                    // Context-appropriate action buttons
                    match asset_type {
                        AssetType::Model => {
                            if ui
                                .button(
                                    egui::RichText::new("📥 Add to Scene")
                                        .color(egui::Color32::from_rgb(100, 220, 140)),
                                )
                                .clicked()
                            {
                                self.pending_actions.push(AssetAction::ImportModel {
                                    path: selected.clone(),
                                });
                            }
                        }
                        AssetType::Texture => {
                            let tex_type = texture_type.unwrap_or(TextureType::Albedo);
                            if ui
                                .button(
                                    egui::RichText::new(format!(
                                        "🎨 Apply as {}",
                                        tex_type.label()
                                    ))
                                    .color(egui::Color32::from_rgb(100, 180, 220)),
                                )
                                .clicked()
                            {
                                self.pending_actions.push(AssetAction::ApplyTexture {
                                    path: selected.clone(),
                                    texture_type: tex_type,
                                });
                            }
                        }
                        AssetType::Material => {
                            if ui
                                .button(
                                    egui::RichText::new("💎 Apply Material")
                                        .color(egui::Color32::from_rgb(180, 140, 220)),
                                )
                                .clicked()
                            {
                                self.pending_actions.push(AssetAction::ApplyMaterial {
                                    path: selected.clone(),
                                });
                            }
                        }
                        AssetType::Scene => {
                            if ui.button("🌐 Load Scene").clicked() {
                                self.pending_actions.push(AssetAction::LoadScene {
                                    path: selected.clone(),
                                });
                            }
                        }
                        AssetType::Prefab => {
                            if ui.button("Spawn Prefab").clicked() {
                                self.pending_actions.push(AssetAction::SpawnPrefab {
                                    path: selected.clone(),
                                });
                            }
                        }
                        _ => {}
                    }
                });
            });
        }
    }

    pub fn get_asset_count(&self) -> usize {
        self.entries.len()
    }

    pub fn get_current_directory(&self) -> &Path {
        &self.current_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // ============================================================================
    // TEXTURE TYPE TESTS
    // ============================================================================

    #[test]
    fn test_texture_type_from_filename_normal() {
        assert_eq!(
            TextureType::from_filename("brick_normal.png"),
            TextureType::Normal
        );
        assert_eq!(
            TextureType::from_filename("wall_n.png"),
            TextureType::Normal
        );
        assert_eq!(
            TextureType::from_filename("floor_nrm.png"),
            TextureType::Normal
        );
        assert_eq!(
            TextureType::from_filename("tile_nor.png"),
            TextureType::Normal
        );
    }

    #[test]
    fn test_texture_type_from_filename_albedo() {
        assert_eq!(
            TextureType::from_filename("brick_albedo.png"),
            TextureType::Albedo
        );
        assert_eq!(
            TextureType::from_filename("wall_diffuse.png"),
            TextureType::Albedo
        );
        assert_eq!(
            TextureType::from_filename("floor_basecolor.png"),
            TextureType::Albedo
        );
        assert_eq!(
            TextureType::from_filename("tile_color.png"),
            TextureType::Albedo
        );
        assert_eq!(
            TextureType::from_filename("wood_d.png"),
            TextureType::Albedo
        );
    }

    #[test]
    fn test_texture_type_from_filename_orm_mra() {
        assert_eq!(
            TextureType::from_filename("brick_orm.png"),
            TextureType::ORM
        );
        assert_eq!(TextureType::from_filename("wall_mra.png"), TextureType::MRA);
    }

    #[test]
    fn test_texture_type_from_filename_roughness() {
        assert_eq!(
            TextureType::from_filename("brick_r.png"),
            TextureType::Roughness
        );
        assert_eq!(
            TextureType::from_filename("wall_rough.png"),
            TextureType::Roughness
        );
        assert_eq!(
            TextureType::from_filename("floor_roughness.png"),
            TextureType::Roughness
        );
    }

    #[test]
    fn test_texture_type_from_filename_metallic() {
        assert_eq!(
            TextureType::from_filename("brick_m.png"),
            TextureType::Metallic
        );
        assert_eq!(
            TextureType::from_filename("wall_metal.png"),
            TextureType::Metallic
        );
        assert_eq!(
            TextureType::from_filename("floor_metallic.png"),
            TextureType::Metallic
        );
        assert_eq!(
            TextureType::from_filename("tile_metalness.png"),
            TextureType::Metallic
        );
    }

    #[test]
    fn test_texture_type_from_filename_ao() {
        assert_eq!(TextureType::from_filename("brick_ao.png"), TextureType::AO);
        assert_eq!(
            TextureType::from_filename("wall_occlusion.png"),
            TextureType::AO
        );
    }

    #[test]
    fn test_texture_type_from_filename_emission() {
        assert_eq!(
            TextureType::from_filename("brick_e.png"),
            TextureType::Emission
        );
        assert_eq!(
            TextureType::from_filename("wall_emit.png"),
            TextureType::Emission
        );
        assert_eq!(
            TextureType::from_filename("floor_emission.png"),
            TextureType::Emission
        );
        assert_eq!(
            TextureType::from_filename("tile_emissive.png"),
            TextureType::Emission
        );
        assert_eq!(
            TextureType::from_filename("neon_glow.png"),
            TextureType::Emission
        );
    }

    #[test]
    fn test_texture_type_from_filename_height() {
        assert_eq!(
            TextureType::from_filename("brick_h.png"),
            TextureType::Height
        );
        assert_eq!(
            TextureType::from_filename("wall_height.png"),
            TextureType::Height
        );
        assert_eq!(
            TextureType::from_filename("floor_disp.png"),
            TextureType::Height
        );
        assert_eq!(
            TextureType::from_filename("tile_displacement.png"),
            TextureType::Height
        );
        assert_eq!(
            TextureType::from_filename("stone_bump.png"),
            TextureType::Height
        );
    }

    #[test]
    fn test_texture_type_from_filename_unknown() {
        assert_eq!(
            TextureType::from_filename("texture.png"),
            TextureType::Unknown
        );
        assert_eq!(
            TextureType::from_filename("random.jpg"),
            TextureType::Unknown
        );
    }

    #[test]
    fn test_texture_type_case_insensitive() {
        assert_eq!(
            TextureType::from_filename("Brick_NORMAL.PNG"),
            TextureType::Normal
        );
        assert_eq!(
            TextureType::from_filename("Wall_ALBEDO.jpg"),
            TextureType::Albedo
        );
    }

    #[test]
    fn test_texture_type_icons() {
        assert_eq!(TextureType::Albedo.icon(), "🎨");
        assert_eq!(TextureType::Normal.icon(), "🔵");
        assert_eq!(TextureType::ORM.icon(), "🔶");
        assert_eq!(TextureType::MRA.icon(), "🟣");
        assert_eq!(TextureType::Roughness.icon(), "◽");
        assert_eq!(TextureType::Metallic.icon(), "⬜");
        assert_eq!(TextureType::AO.icon(), "⬛");
        assert_eq!(TextureType::Emission.icon(), "✨");
        assert_eq!(TextureType::Height.icon(), "📐");
        assert_eq!(TextureType::Unknown.icon(), "❓");
    }

    #[test]
    fn test_texture_type_labels() {
        assert_eq!(TextureType::Albedo.label(), "Albedo");
        assert_eq!(TextureType::Normal.label(), "Normal");
        assert_eq!(TextureType::ORM.label(), "ORM");
        assert_eq!(TextureType::MRA.label(), "MRA");
        assert_eq!(TextureType::Roughness.label(), "Rough");
        assert_eq!(TextureType::Metallic.label(), "Metal");
        assert_eq!(TextureType::AO.label(), "AO");
        assert_eq!(TextureType::Emission.label(), "Emit");
        assert_eq!(TextureType::Height.label(), "Height");
        assert_eq!(TextureType::Unknown.label(), "???");
    }

    #[test]
    fn test_texture_type_colors_are_unique() {
        let colors = [
            TextureType::Albedo.color(),
            TextureType::Normal.color(),
            TextureType::ORM.color(),
            TextureType::MRA.color(),
            TextureType::AO.color(),
            TextureType::Emission.color(),
        ];
        // At least some should be different (not all can be unique with 10 types)
        assert!(colors.iter().any(|c| *c != colors[0]));
    }

    // ============================================================================
    // ASSET CATEGORY TESTS
    // ============================================================================

    #[test]
    fn test_asset_category_matches_all() {
        let cat = AssetCategory::All;
        assert!(cat.matches(&AssetType::Model));
        assert!(cat.matches(&AssetType::Texture));
        assert!(cat.matches(&AssetType::Scene));
        assert!(cat.matches(&AssetType::Material));
        assert!(cat.matches(&AssetType::Audio));
        assert!(cat.matches(&AssetType::Config));
        assert!(cat.matches(&AssetType::Prefab));
    }

    #[test]
    fn test_asset_category_matches_specific() {
        assert!(AssetCategory::Models.matches(&AssetType::Model));
        assert!(!AssetCategory::Models.matches(&AssetType::Texture));

        assert!(AssetCategory::Textures.matches(&AssetType::Texture));
        assert!(!AssetCategory::Textures.matches(&AssetType::Model));

        assert!(AssetCategory::Audio.matches(&AssetType::Audio));
        assert!(!AssetCategory::Audio.matches(&AssetType::Scene));
    }

    #[test]
    fn test_asset_category_icons() {
        assert_eq!(AssetCategory::All.icon(), "📂");
        assert_eq!(AssetCategory::Models.icon(), "🎲");
        assert_eq!(AssetCategory::Textures.icon(), "🖼");
        assert_eq!(AssetCategory::Materials.icon(), "💎");
        assert_eq!(AssetCategory::Prefabs.icon(), "📦");
        assert_eq!(AssetCategory::Scenes.icon(), "🌐");
        assert_eq!(AssetCategory::Audio.icon(), "🎵");
        assert_eq!(AssetCategory::Configs.icon(), "⚙");
    }

    #[test]
    fn test_asset_category_labels() {
        assert_eq!(AssetCategory::All.label(), "All");
        assert_eq!(AssetCategory::Models.label(), "Models");
        assert_eq!(AssetCategory::Textures.label(), "Textures");
        assert_eq!(AssetCategory::Materials.label(), "Materials");
        assert_eq!(AssetCategory::Prefabs.label(), "Prefabs");
        assert_eq!(AssetCategory::Scenes.label(), "Scenes");
        assert_eq!(AssetCategory::Audio.label(), "Audio");
        assert_eq!(AssetCategory::Configs.label(), "Configs");
    }

    // ============================================================================
    // ASSET TYPE TESTS
    // ============================================================================

    #[test]
    fn test_asset_type_from_path() {
        assert_eq!(
            AssetType::from_path(Path::new("test.glb")),
            AssetType::Model
        );
        assert_eq!(
            AssetType::from_path(Path::new("texture.png")),
            AssetType::Texture
        );
        assert_eq!(
            AssetType::from_path(Path::new("scene.ron")),
            AssetType::Scene
        );
        assert_eq!(
            AssetType::from_path(Path::new("config.toml")),
            AssetType::Config
        );
        assert_eq!(
            AssetType::from_path(Path::new("unknown.xyz")),
            AssetType::Unknown
        );
    }

    #[test]
    fn test_asset_type_from_path_models() {
        assert_eq!(
            AssetType::from_path(Path::new("model.glb")),
            AssetType::Model
        );
        assert_eq!(
            AssetType::from_path(Path::new("model.gltf")),
            AssetType::Model
        );
        assert_eq!(
            AssetType::from_path(Path::new("model.obj")),
            AssetType::Model
        );
        assert_eq!(
            AssetType::from_path(Path::new("model.fbx")),
            AssetType::Model
        );
    }

    #[test]
    fn test_asset_type_from_path_textures() {
        assert_eq!(
            AssetType::from_path(Path::new("tex.png")),
            AssetType::Texture
        );
        assert_eq!(
            AssetType::from_path(Path::new("tex.jpg")),
            AssetType::Texture
        );
        assert_eq!(
            AssetType::from_path(Path::new("tex.jpeg")),
            AssetType::Texture
        );
        assert_eq!(
            AssetType::from_path(Path::new("tex.ktx2")),
            AssetType::Texture
        );
        assert_eq!(
            AssetType::from_path(Path::new("tex.dds")),
            AssetType::Texture
        );
    }

    #[test]
    fn test_asset_type_from_path_audio() {
        assert_eq!(
            AssetType::from_path(Path::new("sound.wav")),
            AssetType::Audio
        );
        assert_eq!(
            AssetType::from_path(Path::new("music.ogg")),
            AssetType::Audio
        );
        assert_eq!(
            AssetType::from_path(Path::new("track.mp3")),
            AssetType::Audio
        );
    }

    #[test]
    fn test_asset_type_from_path_configs() {
        assert_eq!(
            AssetType::from_path(Path::new("config.toml")),
            AssetType::Config
        );
        assert_eq!(
            AssetType::from_path(Path::new("settings.json")),
            AssetType::Config
        );
    }

    #[test]
    fn test_asset_type_from_path_prefab() {
        assert_eq!(
            AssetType::from_path(Path::new("entity.prefab.ron")),
            AssetType::Prefab
        );
    }

    #[test]
    fn test_asset_type_icon() {
        assert_eq!(AssetType::Model.icon(), "🎲");
        assert_eq!(AssetType::Texture.icon(), "🖼");
        assert_eq!(AssetType::Scene.icon(), "🌐");
        assert_eq!(AssetType::Directory.icon(), "📁");
        assert_eq!(AssetType::Material.icon(), "💎");
        assert_eq!(AssetType::Audio.icon(), "🎵");
        assert_eq!(AssetType::Config.icon(), "⚙");
        assert_eq!(AssetType::Prefab.icon(), "📦");
        assert_eq!(AssetType::Unknown.icon(), "📄");
    }

    #[test]
    fn test_asset_type_color() {
        // Each type should have a color
        let _ = AssetType::Model.color();
        let _ = AssetType::Texture.color();
        let _ = AssetType::Scene.color();
        let _ = AssetType::Material.color();
        let _ = AssetType::Audio.color();
        let _ = AssetType::Config.color();
        let _ = AssetType::Prefab.color();
        let _ = AssetType::Directory.color();
        let _ = AssetType::Unknown.color();
    }

    // ============================================================================
    // ASSET ENTRY TESTS
    // ============================================================================

    #[test]
    fn test_asset_entry_format_size() {
        let entry = AssetEntry {
            path: PathBuf::from("test.glb"),
            name: "test.glb".to_string(),
            display_name: "Test".to_string(),
            asset_type: AssetType::Model,
            texture_type: None,
            size: 1024,
        };
        assert_eq!(entry.format_size(), "1.0 KB");
    }

    #[test]
    fn test_asset_entry_format_size_mb() {
        let entry = AssetEntry {
            path: PathBuf::from("large.glb"),
            name: "large.glb".to_string(),
            display_name: "Large".to_string(),
            asset_type: AssetType::Model,
            texture_type: None,
            size: 1024 * 1024,
        };
        assert_eq!(entry.format_size(), "1.0 MB");
    }

    #[test]
    fn test_asset_entry_format_size_directory() {
        let entry = AssetEntry {
            path: PathBuf::from("folder"),
            name: "folder".to_string(),
            display_name: "Folder".to_string(),
            asset_type: AssetType::Directory,
            texture_type: None,
            size: 0,
        };
        assert_eq!(entry.format_size(), "");
    }

    #[test]
    fn test_asset_entry_format_size_bytes() {
        let entry = AssetEntry {
            path: PathBuf::from("tiny.txt"),
            name: "tiny.txt".to_string(),
            display_name: "Tiny".to_string(),
            asset_type: AssetType::Unknown,
            texture_type: None,
            size: 512,
        };
        assert_eq!(entry.format_size(), "0.5 KB");
    }

    #[test]
    fn test_asset_entry_format_size_large() {
        let entry = AssetEntry {
            path: PathBuf::from("huge.glb"),
            name: "huge.glb".to_string(),
            display_name: "Huge".to_string(),
            asset_type: AssetType::Model,
            texture_type: None,
            size: 10 * 1024 * 1024,
        };
        assert_eq!(entry.format_size(), "10.0 MB");
    }

    #[test]
    fn test_asset_entry_texture_type_for_textures() {
        let entry = AssetEntry {
            path: PathBuf::from("brick_normal.png"),
            name: "brick_normal.png".to_string(),
            display_name: "Brick Normal".to_string(),
            asset_type: AssetType::Texture,
            texture_type: Some(TextureType::Normal),
            size: 1024,
        };
        assert_eq!(entry.texture_type, Some(TextureType::Normal));
    }

    #[test]
    fn test_asset_entry_no_texture_type_for_models() {
        let entry = AssetEntry {
            path: PathBuf::from("model.glb"),
            name: "model.glb".to_string(),
            display_name: "Model".to_string(),
            asset_type: AssetType::Model,
            texture_type: None,
            size: 1024,
        };
        assert!(entry.texture_type.is_none());
    }

    // ============================================================================
    // VIEW MODE TESTS
    // ============================================================================

    #[test]
    fn test_view_mode_list() {
        let mode = ViewMode::List;
        assert_eq!(mode, ViewMode::List);
    }

    #[test]
    fn test_view_mode_grid() {
        let mode = ViewMode::Grid;
        assert_eq!(mode, ViewMode::Grid);
    }

    #[test]
    fn test_view_mode_comparison() {
        assert_ne!(ViewMode::List, ViewMode::Grid);
    }

    // ============================================================================
    // ASSET BROWSER TESTS
    // ============================================================================

    #[test]
    fn test_asset_browser_creation() {
        let temp_dir = env::temp_dir();
        let canonical = temp_dir.canonicalize().unwrap_or(temp_dir.clone());
        let browser = AssetBrowser::new(temp_dir);
        assert_eq!(browser.root_path, canonical);
        assert_eq!(browser.current_path, canonical);
    }

    #[test]
    fn test_asset_browser_default_view_mode() {
        let temp_dir = env::temp_dir();
        let browser = AssetBrowser::new(temp_dir);
        assert_eq!(browser.view_mode, ViewMode::Grid);
    }

    #[test]
    fn test_asset_browser_default_category() {
        let temp_dir = env::temp_dir();
        let browser = AssetBrowser::new(temp_dir);
        assert_eq!(browser.category_filter, AssetCategory::All);
    }

    #[test]
    fn test_asset_browser_default_texture_filter() {
        let temp_dir = env::temp_dir();
        let browser = AssetBrowser::new(temp_dir);
        assert!(browser.texture_type_filter.is_none());
    }

    #[test]
    fn test_asset_browser_default_show_hidden() {
        let temp_dir = env::temp_dir();
        let browser = AssetBrowser::new(temp_dir);
        assert!(!browser.show_hidden);
    }

    #[test]
    fn test_asset_browser_default_texture_badges() {
        let temp_dir = env::temp_dir();
        let browser = AssetBrowser::new(temp_dir);
        assert!(browser.show_texture_badges);
    }

    #[test]
    fn test_asset_browser_navigation() {
        let temp_dir = env::temp_dir();
        let canonical = temp_dir.canonicalize().unwrap_or(temp_dir.clone());
        let mut browser = AssetBrowser::new(temp_dir);
        browser.navigate_up();
        browser.navigate_to(canonical.clone());
        assert_eq!(browser.current_path, canonical);
    }

    #[test]
    fn test_asset_browser_selected_asset_initially_none() {
        let temp_dir = env::temp_dir();
        let browser = AssetBrowser::new(temp_dir);
        assert!(browser.selected_asset().is_none());
    }

    #[test]
    fn test_asset_browser_filter() {
        let temp_dir = env::temp_dir();
        let mut browser = AssetBrowser::new(temp_dir);
        browser.filter_type = Some(AssetType::Model);
        assert_eq!(browser.filter_type, Some(AssetType::Model));
        browser.filter_type = None;
        assert_eq!(browser.filter_type, None);
    }

    #[test]
    fn test_asset_browser_search() {
        let temp_dir = env::temp_dir();
        let mut browser = AssetBrowser::new(temp_dir);
        browser.search_query = "test".to_string();
        assert_eq!(browser.search_query, "test");
    }

    #[test]
    fn test_asset_browser_get_current_directory() {
        let temp_dir = env::temp_dir();
        let canonical = temp_dir.canonicalize().unwrap_or(temp_dir.clone());
        let browser = AssetBrowser::new(temp_dir);
        assert_eq!(browser.get_current_directory(), canonical.as_path());
    }

    #[test]
    fn test_asset_browser_take_pending_actions() {
        let temp_dir = env::temp_dir();
        let mut browser = AssetBrowser::new(temp_dir);
        let actions = browser.take_pending_actions();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_asset_browser_take_dragged_prefab() {
        let temp_dir = env::temp_dir();
        let mut browser = AssetBrowser::new(temp_dir);
        let prefab = browser.take_dragged_prefab();
        assert!(prefab.is_none());
    }

    #[test]
    fn test_asset_browser_thumbnail_cache_size() {
        let temp_dir = env::temp_dir();
        let browser = AssetBrowser::new(temp_dir);
        assert_eq!(browser.max_cache_size, 100);
    }

    #[test]
    fn test_asset_browser_thumbnail_size() {
        let temp_dir = env::temp_dir();
        let browser = AssetBrowser::new(temp_dir);
        assert_eq!(browser.thumbnail_size, 80.0);
    }

    // ============================================================================
    // ASSET ACTION TESTS
    // ============================================================================

    #[test]
    fn test_asset_action_import_model() {
        let action = AssetAction::ImportModel {
            path: PathBuf::from("model.glb"),
        };
        match action {
            AssetAction::ImportModel { path } => assert_eq!(path, PathBuf::from("model.glb")),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_asset_action_apply_texture() {
        let action = AssetAction::ApplyTexture {
            path: PathBuf::from("tex.png"),
            texture_type: TextureType::Normal,
        };
        match action {
            AssetAction::ApplyTexture { path, texture_type } => {
                assert_eq!(path, PathBuf::from("tex.png"));
                assert_eq!(texture_type, TextureType::Normal);
            }
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_asset_action_load_scene() {
        let action = AssetAction::LoadScene {
            path: PathBuf::from("level.ron"),
        };
        match action {
            AssetAction::LoadScene { path } => assert_eq!(path, PathBuf::from("level.ron")),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_asset_action_spawn_prefab() {
        let action = AssetAction::SpawnPrefab {
            path: PathBuf::from("entity.prefab.ron"),
        };
        match action {
            AssetAction::SpawnPrefab { path } => {
                assert_eq!(path, PathBuf::from("entity.prefab.ron"))
            }
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_asset_action_apply_material() {
        let action = AssetAction::ApplyMaterial {
            path: PathBuf::from("metal.mat"),
        };
        match action {
            AssetAction::ApplyMaterial { path } => assert_eq!(path, PathBuf::from("metal.mat")),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_asset_action_open_external() {
        let action = AssetAction::OpenExternal {
            path: PathBuf::from("/assets"),
        };
        match action {
            AssetAction::OpenExternal { path } => assert_eq!(path, PathBuf::from("/assets")),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_asset_action_inspect_asset() {
        let action = AssetAction::InspectAsset {
            path: PathBuf::from("item.glb"),
        };
        match action {
            AssetAction::InspectAsset { path } => assert_eq!(path, PathBuf::from("item.glb")),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_asset_action_load_to_viewport() {
        let action = AssetAction::LoadToViewport {
            path: PathBuf::from("preview.glb"),
        };
        match action {
            AssetAction::LoadToViewport { path } => assert_eq!(path, PathBuf::from("preview.glb")),
            _ => panic!("Wrong action type"),
        }
    }

    // ============================================================
    // SESSION 6: ENUM DISPLAY & HELPER TESTS
    // ============================================================

    #[test]
    fn test_texture_type_display() {
        assert_eq!(format!("{}", TextureType::Albedo), "🎨 Albedo");
        assert_eq!(format!("{}", TextureType::Normal), "🔵 Normal");
        assert_eq!(format!("{}", TextureType::ORM), "🔶 ORM");
        assert_eq!(format!("{}", TextureType::MRA), "🟣 MRA");
        assert_eq!(format!("{}", TextureType::Unknown), "❓ Unknown");
    }

    #[test]
    fn test_texture_type_all() {
        let all = TextureType::all();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&TextureType::Albedo));
        assert!(all.contains(&TextureType::Unknown));
    }

    #[test]
    fn test_texture_type_name() {
        assert_eq!(TextureType::Albedo.name(), "Albedo");
        assert_eq!(TextureType::Normal.name(), "Normal");
        assert_eq!(TextureType::AO.name(), "Ambient Occlusion");
        assert_eq!(TextureType::Roughness.name(), "Roughness");
    }

    #[test]
    fn test_texture_type_is_pbr_component() {
        assert!(TextureType::Albedo.is_pbr_component());
        assert!(TextureType::Normal.is_pbr_component());
        assert!(TextureType::ORM.is_pbr_component());
        assert!(!TextureType::Unknown.is_pbr_component());
    }

    #[test]
    fn test_texture_type_is_packed() {
        assert!(!TextureType::Albedo.is_packed());
        assert!(!TextureType::Normal.is_packed());
        assert!(TextureType::ORM.is_packed());
        assert!(TextureType::MRA.is_packed());
        assert!(!TextureType::Roughness.is_packed());
    }

    #[test]
    fn test_asset_category_display() {
        assert_eq!(format!("{}", AssetCategory::All), "📂 All");
        assert_eq!(format!("{}", AssetCategory::Models), "🎲 Models");
        assert_eq!(format!("{}", AssetCategory::Textures), "🖼 Textures");
        assert_eq!(format!("{}", AssetCategory::Audio), "🎵 Audio");
    }

    #[test]
    fn test_asset_category_all() {
        let all = AssetCategory::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&AssetCategory::All));
        assert!(all.contains(&AssetCategory::Configs));
    }

    #[test]
    fn test_asset_category_name() {
        assert_eq!(AssetCategory::All.name(), "All");
        assert_eq!(AssetCategory::Models.name(), "Models");
        assert_eq!(AssetCategory::Textures.name(), "Textures");
    }

    #[test]
    fn test_asset_category_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AssetCategory::All);
        set.insert(AssetCategory::Models);
        assert!(set.contains(&AssetCategory::All));
        assert!(!set.contains(&AssetCategory::Audio));
    }

    #[test]
    fn test_asset_type_display() {
        assert_eq!(format!("{}", AssetType::Model), "🎲 Model");
        assert_eq!(format!("{}", AssetType::Texture), "🖼 Texture");
        assert_eq!(format!("{}", AssetType::Scene), "🌐 Scene");
        assert_eq!(format!("{}", AssetType::Directory), "📁 Directory");
        assert_eq!(format!("{}", AssetType::Unknown), "📄 Unknown");
    }

    #[test]
    fn test_asset_type_all() {
        let all = AssetType::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&AssetType::Model));
        assert!(all.contains(&AssetType::Unknown));
    }

    #[test]
    fn test_asset_type_name() {
        assert_eq!(AssetType::Model.name(), "Model");
        assert_eq!(AssetType::Texture.name(), "Texture");
        assert_eq!(AssetType::Scene.name(), "Scene");
        assert_eq!(AssetType::Directory.name(), "Directory");
    }

    #[test]
    fn test_asset_type_is_content() {
        assert!(AssetType::Model.is_content());
        assert!(AssetType::Texture.is_content());
        assert!(AssetType::Scene.is_content());
        assert!(AssetType::Audio.is_content());
        assert!(AssetType::Prefab.is_content());
        assert!(!AssetType::Config.is_content());
        assert!(!AssetType::Directory.is_content());
        assert!(!AssetType::Unknown.is_content());
    }

    #[test]
    fn test_asset_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AssetType::Model);
        set.insert(AssetType::Texture);
        assert!(set.contains(&AssetType::Model));
        assert!(!set.contains(&AssetType::Scene));
    }

    #[test]
    fn test_view_mode_display() {
        assert_eq!(format!("{}", ViewMode::List), "📋 List");
        assert_eq!(format!("{}", ViewMode::Grid), "📰 Grid");
    }

    #[test]
    fn test_view_mode_all() {
        let all = ViewMode::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&ViewMode::List));
        assert!(all.contains(&ViewMode::Grid));
    }

    #[test]
    fn test_view_mode_name() {
        assert_eq!(ViewMode::List.name(), "List");
        assert_eq!(ViewMode::Grid.name(), "Grid");
    }

    #[test]
    fn test_view_mode_icon() {
        assert_eq!(ViewMode::List.icon(), "📋");
        assert_eq!(ViewMode::Grid.icon(), "📰");
    }

    #[test]
    fn test_view_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ViewMode::List);
        assert!(set.contains(&ViewMode::List));
        assert!(!set.contains(&ViewMode::Grid));
    }

    // ========== AssetAction Tests ==========

    #[test]
    fn test_asset_action_display() {
        let action = AssetAction::ImportModel {
            path: PathBuf::from("test.obj"),
        };
        let display = format!("{}", action);
        assert!(display.contains(action.name()));
    }

    #[test]
    fn test_asset_action_all_variants() {
        let all = AssetAction::all_variants();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&"ImportModel"));
        assert!(all.contains(&"ApplyTexture"));
        assert!(all.contains(&"LoadScene"));
    }

    #[test]
    fn test_asset_action_names() {
        let import = AssetAction::ImportModel {
            path: PathBuf::from("model.obj"),
        };
        assert_eq!(import.name(), "Import Model");

        let load = AssetAction::LoadToViewport {
            path: PathBuf::from("model.obj"),
        };
        assert_eq!(load.name(), "Load to Viewport");

        let texture = AssetAction::ApplyTexture {
            path: PathBuf::from("texture.png"),
            texture_type: TextureType::Albedo,
        };
        assert_eq!(texture.name(), "Apply Texture");
    }

    #[test]
    fn test_asset_action_icons() {
        let import = AssetAction::ImportModel {
            path: PathBuf::from("model.obj"),
        };
        assert_eq!(import.icon(), "📥");

        let inspect = AssetAction::InspectAsset {
            path: PathBuf::from("asset.mat"),
        };
        assert_eq!(inspect.icon(), "🔍");
    }

    #[test]
    fn test_asset_action_path() {
        let path = PathBuf::from("test/model.obj");
        let action = AssetAction::ImportModel { path: path.clone() };
        assert_eq!(action.path(), &path);
    }

    #[test]
    fn test_asset_action_is_modifying() {
        let import = AssetAction::ImportModel {
            path: PathBuf::from("model.obj"),
        };
        assert!(import.is_modifying());

        let apply_texture = AssetAction::ApplyTexture {
            path: PathBuf::from("texture.png"),
            texture_type: TextureType::Normal,
        };
        assert!(apply_texture.is_modifying());

        let spawn = AssetAction::SpawnPrefab {
            path: PathBuf::from("prefab.ron"),
        };
        assert!(spawn.is_modifying());

        // Non-modifying
        let load = AssetAction::LoadToViewport {
            path: PathBuf::from("model.obj"),
        };
        assert!(!load.is_modifying());

        let inspect = AssetAction::InspectAsset {
            path: PathBuf::from("asset.mat"),
        };
        assert!(!inspect.is_modifying());
    }

    #[test]
    fn test_asset_action_is_viewing() {
        let load = AssetAction::LoadToViewport {
            path: PathBuf::from("model.obj"),
        };
        assert!(load.is_viewing());

        let external = AssetAction::OpenExternal {
            path: PathBuf::from("file.txt"),
        };
        assert!(external.is_viewing());

        let inspect = AssetAction::InspectAsset {
            path: PathBuf::from("asset.mat"),
        };
        assert!(inspect.is_viewing());

        // Non-viewing
        let import = AssetAction::ImportModel {
            path: PathBuf::from("model.obj"),
        };
        assert!(!import.is_viewing());
    }

    #[test]
    fn test_asset_action_is_scene_action() {
        let load_scene = AssetAction::LoadScene {
            path: PathBuf::from("level.scene"),
        };
        assert!(load_scene.is_scene_action());

        let import = AssetAction::ImportModel {
            path: PathBuf::from("model.obj"),
        };
        assert!(!import.is_scene_action());
    }

    #[test]
    fn test_asset_action_partial_eq() {
        let a1 = AssetAction::ImportModel {
            path: PathBuf::from("model.obj"),
        };
        let a2 = AssetAction::ImportModel {
            path: PathBuf::from("model.obj"),
        };
        let a3 = AssetAction::ImportModel {
            path: PathBuf::from("other.obj"),
        };
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    // ============================================================================
    // PRETTIFY NAME TESTS
    // ============================================================================

    #[test]
    fn test_prettify_name_snake_case() {
        assert_eq!(
            prettify_name("bridge_center_stone.glb"),
            "Bridge Center Stone"
        );
    }

    #[test]
    fn test_prettify_name_already_pretty() {
        assert_eq!(prettify_name("Nature Kit"), "Nature Kit");
    }

    #[test]
    fn test_prettify_name_hyphenated() {
        assert_eq!(prettify_name("car-race-kit.glb"), "Car Race Kit");
    }

    #[test]
    fn test_prettify_name_single_word() {
        assert_eq!(prettify_name("barrel.glb"), "Barrel");
    }

    #[test]
    fn test_prettify_name_directory() {
        assert_eq!(prettify_name("3D assets"), "3d Assets");
    }
}
