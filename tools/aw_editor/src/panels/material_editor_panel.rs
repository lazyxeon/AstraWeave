//! Material Editor Panel for the editor UI
//!
//! Provides comprehensive PBR material editing:
//! - Albedo, normal, metallic, roughness, AO maps
//! - Material presets and templates
//! - Shader parameter configuration
//! - Material preview with lighting
//! - Material library management

use egui::{Color32, RichText, Ui, Vec2};
use tracing::info;

use crate::panels::Panel;

/// Material type/shader
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MaterialType {
    #[default]
    StandardPBR,
    Unlit,
    Subsurface,
    Glass,
    Water,
    Foliage,
    Hair,
    Cloth,
    Terrain,
    Decal,
}

impl std::fmt::Display for MaterialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl MaterialType {
    pub fn name(&self) -> &'static str {
        match self {
            MaterialType::StandardPBR => "Standard PBR",
            MaterialType::Unlit => "Unlit",
            MaterialType::Subsurface => "Subsurface",
            MaterialType::Glass => "Glass",
            MaterialType::Water => "Water",
            MaterialType::Foliage => "Foliage",
            MaterialType::Hair => "Hair",
            MaterialType::Cloth => "Cloth",
            MaterialType::Terrain => "Terrain",
            MaterialType::Decal => "Decal",
        }
    }

    pub fn all() -> &'static [MaterialType] {
        &[
            MaterialType::StandardPBR,
            MaterialType::Unlit,
            MaterialType::Subsurface,
            MaterialType::Glass,
            MaterialType::Water,
            MaterialType::Foliage,
            MaterialType::Hair,
            MaterialType::Cloth,
            MaterialType::Terrain,
            MaterialType::Decal,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            MaterialType::StandardPBR => "[Art]",
            MaterialType::Unlit => "[Lgt]",
            MaterialType::Subsurface => "🧴",
            MaterialType::Glass => "[Orb]",
            MaterialType::Water => "💧",
            MaterialType::Foliage => "[Leaf]",
            MaterialType::Hair => "💇",
            MaterialType::Cloth => "👕",
            MaterialType::Terrain => "🏔️",
            MaterialType::Decal => "[Tag]",
        }
    }
}

/// Blend mode for material
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BlendMode {
    #[default]
    Opaque,
    Masked,
    Translucent,
    Additive,
    Modulate,
}

impl std::fmt::Display for BlendMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl BlendMode {
    pub fn all() -> &'static [BlendMode] {
        &[
            BlendMode::Opaque,
            BlendMode::Masked,
            BlendMode::Translucent,
            BlendMode::Additive,
            BlendMode::Modulate,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BlendMode::Opaque => "Opaque",
            BlendMode::Masked => "Masked",
            BlendMode::Translucent => "Translucent",
            BlendMode::Additive => "Additive",
            BlendMode::Modulate => "Modulate",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BlendMode::Opaque => "⬛",
            BlendMode::Masked => "[Mask]",
            BlendMode::Translucent => "[Sq]",
            BlendMode::Additive => "+",
            BlendMode::Modulate => "[Shuf]",
        }
    }

    pub fn is_transparent(&self) -> bool {
        matches!(
            self,
            BlendMode::Translucent | BlendMode::Additive | BlendMode::Modulate
        )
    }
}

/// Texture channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextureChannel {
    Albedo,
    Normal,
    Metallic,
    Roughness,
    AO,
    Emissive,
    Height,
    Opacity,
}

impl std::fmt::Display for TextureChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl TextureChannel {
    pub fn all() -> &'static [TextureChannel] {
        &[
            TextureChannel::Albedo,
            TextureChannel::Normal,
            TextureChannel::Metallic,
            TextureChannel::Roughness,
            TextureChannel::AO,
            TextureChannel::Emissive,
            TextureChannel::Height,
            TextureChannel::Opacity,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TextureChannel::Albedo => "[Art]",
            TextureChannel::Normal => "🗺️",
            TextureChannel::Metallic => "[Fx]",
            TextureChannel::Roughness => "🔨",
            TextureChannel::AO => "🌑",
            TextureChannel::Emissive => "[Lgt]",
            TextureChannel::Height => "⛰️",
            TextureChannel::Opacity => "👻",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TextureChannel::Albedo => "Albedo",
            TextureChannel::Normal => "Normal",
            TextureChannel::Metallic => "Metallic",
            TextureChannel::Roughness => "Roughness",
            TextureChannel::AO => "Ambient Occlusion",
            TextureChannel::Emissive => "Emissive",
            TextureChannel::Height => "Height",
            TextureChannel::Opacity => "Opacity",
        }
    }
}

/// Texture slot configuration
#[derive(Debug, Clone)]
pub struct TextureSlot {
    pub channel: TextureChannel,
    pub texture_path: String,
    pub tiling: (f32, f32),
    pub offset: (f32, f32),
    pub intensity: f32,
    pub enabled: bool,
}

impl Default for TextureSlot {
    fn default() -> Self {
        Self {
            channel: TextureChannel::Albedo,
            texture_path: String::new(),
            tiling: (1.0, 1.0),
            offset: (0.0, 0.0),
            intensity: 1.0,
            enabled: true,
        }
    }
}

/// Material definition
#[derive(Debug, Clone)]
pub struct Material {
    pub id: u32,
    pub name: String,
    pub material_type: MaterialType,
    pub blend_mode: BlendMode,

    // PBR properties
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive_color: [f32; 3],
    pub emissive_intensity: f32,

    // Textures
    pub texture_slots: Vec<TextureSlot>,

    // Additional properties
    pub two_sided: bool,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
    pub alpha_cutoff: f32,

    // Subsurface (for skin, wax, etc.)
    pub subsurface_color: [f32; 3],
    pub subsurface_radius: f32,

    // Glass/translucent
    pub ior: f32,
    pub transmission: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Material".to_string(),
            material_type: MaterialType::StandardPBR,
            blend_mode: BlendMode::Opaque,

            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive_color: [0.0, 0.0, 0.0],
            emissive_intensity: 0.0,

            texture_slots: Vec::new(),

            two_sided: false,
            cast_shadows: true,
            receive_shadows: true,
            alpha_cutoff: 0.5,

            subsurface_color: [1.0, 0.2, 0.1],
            subsurface_radius: 1.0,

            ior: 1.5,
            transmission: 0.0,
        }
    }
}

/// Material preset/template
#[derive(Debug, Clone)]
pub struct MaterialPreset {
    pub name: String,
    pub category: String,
    pub material_type: MaterialType,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

impl MaterialPreset {
    fn presets() -> Vec<MaterialPreset> {
        vec![
            MaterialPreset {
                name: "Polished Metal".to_string(),
                category: "Metals".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [0.9, 0.9, 0.9, 1.0],
                metallic: 1.0,
                roughness: 0.1,
            },
            MaterialPreset {
                name: "Brushed Steel".to_string(),
                category: "Metals".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [0.7, 0.7, 0.75, 1.0],
                metallic: 1.0,
                roughness: 0.4,
            },
            MaterialPreset {
                name: "Gold".to_string(),
                category: "Metals".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [1.0, 0.84, 0.0, 1.0],
                metallic: 1.0,
                roughness: 0.2,
            },
            MaterialPreset {
                name: "Rough Wood".to_string(),
                category: "Organic".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [0.55, 0.35, 0.2, 1.0],
                metallic: 0.0,
                roughness: 0.8,
            },
            MaterialPreset {
                name: "Polished Wood".to_string(),
                category: "Organic".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [0.6, 0.4, 0.25, 1.0],
                metallic: 0.0,
                roughness: 0.3,
            },
            MaterialPreset {
                name: "Concrete".to_string(),
                category: "Stone".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [0.5, 0.5, 0.5, 1.0],
                metallic: 0.0,
                roughness: 0.9,
            },
            MaterialPreset {
                name: "Marble".to_string(),
                category: "Stone".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [0.95, 0.95, 0.95, 1.0],
                metallic: 0.0,
                roughness: 0.2,
            },
            MaterialPreset {
                name: "Rubber".to_string(),
                category: "Synthetic".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [0.1, 0.1, 0.1, 1.0],
                metallic: 0.0,
                roughness: 0.95,
            },
            MaterialPreset {
                name: "Plastic".to_string(),
                category: "Synthetic".to_string(),
                material_type: MaterialType::StandardPBR,
                base_color: [0.8, 0.2, 0.2, 1.0],
                metallic: 0.0,
                roughness: 0.4,
            },
            MaterialPreset {
                name: "Glass".to_string(),
                category: "Transparent".to_string(),
                material_type: MaterialType::Glass,
                base_color: [1.0, 1.0, 1.0, 0.1],
                metallic: 0.0,
                roughness: 0.0,
            },
            MaterialPreset {
                name: "Skin".to_string(),
                category: "Organic".to_string(),
                material_type: MaterialType::Subsurface,
                base_color: [0.9, 0.7, 0.6, 1.0],
                metallic: 0.0,
                roughness: 0.5,
            },
        ]
    }
}

/// Preview lighting mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PreviewLighting {
    #[default]
    Studio,
    Outdoor,
    Indoor,
    Dramatic,
    Custom,
}

impl std::fmt::Display for PreviewLighting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl PreviewLighting {
    pub fn all() -> &'static [PreviewLighting] {
        &[
            PreviewLighting::Studio,
            PreviewLighting::Outdoor,
            PreviewLighting::Indoor,
            PreviewLighting::Dramatic,
            PreviewLighting::Custom,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PreviewLighting::Studio => "Studio",
            PreviewLighting::Outdoor => "Outdoor",
            PreviewLighting::Indoor => "Indoor",
            PreviewLighting::Dramatic => "Dramatic",
            PreviewLighting::Custom => "Custom",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PreviewLighting::Studio => "[Anim]",
            PreviewLighting::Outdoor => "[Sun]",
            PreviewLighting::Indoor => "[Lgt]",
            PreviewLighting::Dramatic => "[Mask]",
            PreviewLighting::Custom => "⚙️",
        }
    }
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MaterialTab {
    #[default]
    Properties,
    Textures,
    Advanced,
    Presets,
    Preview,
    Library,
}

impl std::fmt::Display for MaterialTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl MaterialTab {
    pub fn all() -> &'static [MaterialTab] {
        &[
            MaterialTab::Properties,
            MaterialTab::Textures,
            MaterialTab::Advanced,
            MaterialTab::Presets,
            MaterialTab::Preview,
            MaterialTab::Library,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            MaterialTab::Properties => "Properties",
            MaterialTab::Textures => "Textures",
            MaterialTab::Advanced => "Advanced",
            MaterialTab::Presets => "Presets",
            MaterialTab::Preview => "Preview",
            MaterialTab::Library => "Library",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            MaterialTab::Properties => "[Art]",
            MaterialTab::Textures => "[Img]",
            MaterialTab::Advanced => "⚙️",
            MaterialTab::Presets => "[List]",
            MaterialTab::Preview => "👁️",
            MaterialTab::Library => "📚",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// ACTION SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════════

/// Actions that can be triggered from the material editor panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MaterialEditorAction {
    // Tab navigation
    SetActiveTab(MaterialTab),

    // Material operations
    AddMaterial,
    RemoveMaterial(u32),
    SelectMaterial(u32),
    DuplicateMaterial(u32),
    SetMaterialName(u32, String),
    SetMaterialType(u32, MaterialType),

    // Properties
    SetBaseColor(u32, [f32; 4]),
    SetMetallic(u32, f32),
    SetRoughness(u32, f32),
    SetNormalStrength(u32, f32),
    SetAoStrength(u32, f32),
    SetBlendMode(u32, BlendMode),

    // Emissive
    SetEmissiveColor(u32, [f32; 3]),
    SetEmissiveIntensity(u32, f32),
    ToggleEmissive(u32, bool),

    // Textures
    SetTexture(u32, TextureChannel, String),
    RemoveTexture(u32, TextureChannel),
    SetTextureScale(u32, f32, f32),
    SetTextureOffset(u32, f32, f32),

    // Advanced
    ToggleTwoSided(u32, bool),
    ToggleCastShadow(u32, bool),
    ToggleReceiveShadow(u32, bool),
    SetRenderQueue(u32, i32),

    // Presets
    ApplyPreset(String),
    SaveAsPreset(u32, String),
    SetPresetFilter(String),

    // Preview
    SetPreviewLighting(PreviewLighting),
    SetPreviewRotation(f32),
    SetPreviewZoom(f32),

    // Library
    SetLibraryPath(String),
    RefreshLibrary,
    ImportMaterial(String),
    ExportMaterial(u32, String),
}

impl std::fmt::Display for MaterialEditorAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialEditorAction::SetActiveTab(tab) => write!(f, "Set tab: {}", tab),
            MaterialEditorAction::AddMaterial => write!(f, "Add material"),
            MaterialEditorAction::RemoveMaterial(id) => write!(f, "Remove material {}", id),
            MaterialEditorAction::SelectMaterial(id) => write!(f, "Select material {}", id),
            MaterialEditorAction::DuplicateMaterial(id) => write!(f, "Duplicate material {}", id),
            MaterialEditorAction::SetMaterialName(id, name) => {
                write!(f, "Set material {} name: {}", id, name)
            }
            MaterialEditorAction::SetMaterialType(id, t) => {
                write!(f, "Set material {} type: {}", id, t)
            }
            MaterialEditorAction::SetBaseColor(id, _) => {
                write!(f, "Set material {} base color", id)
            }
            MaterialEditorAction::SetMetallic(id, v) => {
                write!(f, "Set material {} metallic: {:.2}", id, v)
            }
            MaterialEditorAction::SetRoughness(id, v) => {
                write!(f, "Set material {} roughness: {:.2}", id, v)
            }
            MaterialEditorAction::SetNormalStrength(id, v) => {
                write!(f, "Set material {} normal: {:.2}", id, v)
            }
            MaterialEditorAction::SetAoStrength(id, v) => {
                write!(f, "Set material {} AO: {:.2}", id, v)
            }
            MaterialEditorAction::SetBlendMode(id, m) => {
                write!(f, "Set material {} blend: {}", id, m)
            }
            MaterialEditorAction::SetEmissiveColor(id, _) => {
                write!(f, "Set material {} emissive color", id)
            }
            MaterialEditorAction::SetEmissiveIntensity(id, v) => {
                write!(f, "Set material {} emissive: {:.2}", id, v)
            }
            MaterialEditorAction::ToggleEmissive(id, b) => {
                write!(f, "Toggle material {} emissive: {}", id, b)
            }
            MaterialEditorAction::SetTexture(id, ch, _) => {
                write!(f, "Set material {} texture {:?}", id, ch)
            }
            MaterialEditorAction::RemoveTexture(id, ch) => {
                write!(f, "Remove material {} texture {:?}", id, ch)
            }
            MaterialEditorAction::SetTextureScale(id, u, v) => {
                write!(f, "Set material {} tex scale: ({:.2}, {:.2})", id, u, v)
            }
            MaterialEditorAction::SetTextureOffset(id, u, v) => {
                write!(f, "Set material {} tex offset: ({:.2}, {:.2})", id, u, v)
            }
            MaterialEditorAction::ToggleTwoSided(id, b) => {
                write!(f, "Toggle material {} two-sided: {}", id, b)
            }
            MaterialEditorAction::ToggleCastShadow(id, b) => {
                write!(f, "Toggle material {} cast shadow: {}", id, b)
            }
            MaterialEditorAction::ToggleReceiveShadow(id, b) => {
                write!(f, "Toggle material {} receive shadow: {}", id, b)
            }
            MaterialEditorAction::SetRenderQueue(id, q) => {
                write!(f, "Set material {} queue: {}", id, q)
            }
            MaterialEditorAction::ApplyPreset(name) => write!(f, "Apply preset: {}", name),
            MaterialEditorAction::SaveAsPreset(id, name) => {
                write!(f, "Save material {} as preset: {}", id, name)
            }
            MaterialEditorAction::SetPresetFilter(filter) => {
                write!(f, "Filter presets: {}", filter)
            }
            MaterialEditorAction::SetPreviewLighting(l) => {
                write!(f, "Set preview lighting: {:?}", l)
            }
            MaterialEditorAction::SetPreviewRotation(r) => {
                write!(f, "Set preview rotation: {:.1}", r)
            }
            MaterialEditorAction::SetPreviewZoom(z) => write!(f, "Set preview zoom: {:.2}", z),
            MaterialEditorAction::SetLibraryPath(path) => write!(f, "Set library path: {}", path),
            MaterialEditorAction::RefreshLibrary => write!(f, "Refresh library"),
            MaterialEditorAction::ImportMaterial(path) => write!(f, "Import material: {}", path),
            MaterialEditorAction::ExportMaterial(id, path) => {
                write!(f, "Export material {} to: {}", id, path)
            }
        }
    }
}

/// Main Material Editor Panel
pub struct MaterialEditorPanel {
    // Tab state
    active_tab: MaterialTab,

    // Materials
    materials: Vec<Material>,
    selected_material: Option<u32>,
    current_material: Material,

    // Presets
    presets: Vec<MaterialPreset>,
    preset_filter: String,

    // Preview
    preview_lighting: PreviewLighting,
    preview_rotation: f32,
    preview_zoom: f32,

    // Library
    library_path: String,
    library_materials: Vec<String>,

    // ID counter
    next_id: u32,

    // Action system
    actions: Vec<MaterialEditorAction>,
}

impl Default for MaterialEditorPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: MaterialTab::Properties,

            materials: Vec::new(),
            selected_material: None,
            current_material: Material::default(),

            presets: MaterialPreset::presets(),
            preset_filter: String::new(),

            preview_lighting: PreviewLighting::Studio,
            preview_rotation: 0.0,
            preview_zoom: 1.0,

            library_path: "assets/materials".to_string(),
            library_materials: Vec::new(),

            next_id: 1,

            actions: Vec::new(),
        };

        panel
    }
}

impl MaterialEditorPanel {
    pub fn new() -> Self {
        Self::default()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Action System
    // ─────────────────────────────────────────────────────────────────────────────

    /// Queue an action for later processing
    pub fn queue_action(&mut self, action: MaterialEditorAction) {
        info!(target: "aw_editor::panels::material_editor_panel", action = %action, "Material editor action queued");
        self.actions.push(action);
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Get pending actions without consuming them
    pub fn pending_actions(&self) -> &[MaterialEditorAction] {
        &self.actions
    }

    /// Take all pending actions, clearing the queue
    pub fn take_actions(&mut self) -> Vec<MaterialEditorAction> {
        std::mem::take(&mut self.actions)
    }

    fn create_sample_data(&mut self) {
        // Create sample materials
        let id = self.next_id();
        self.materials.push(Material {
            id,
            name: "Ground Dirt".to_string(),
            material_type: MaterialType::StandardPBR,
            base_color: [0.4, 0.3, 0.2, 1.0],
            metallic: 0.0,
            roughness: 0.9,
            texture_slots: vec![
                TextureSlot {
                    channel: TextureChannel::Albedo,
                    texture_path: "textures/dirt_albedo.png".to_string(),
                    ..Default::default()
                },
                TextureSlot {
                    channel: TextureChannel::Normal,
                    texture_path: "textures/dirt_normal.png".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        self.next_id += 1;

        let id = self.next_id();
        self.materials.push(Material {
            id,
            name: "Metal Plate".to_string(),
            material_type: MaterialType::StandardPBR,
            base_color: [0.8, 0.8, 0.85, 1.0],
            metallic: 1.0,
            roughness: 0.3,
            ..Default::default()
        });
        self.next_id += 1;

        let id = self.next_id();
        self.materials.push(Material {
            id,
            name: "Glowing Crystal".to_string(),
            material_type: MaterialType::StandardPBR,
            base_color: [0.2, 0.8, 1.0, 0.8],
            blend_mode: BlendMode::Translucent,
            metallic: 0.0,
            roughness: 0.1,
            emissive_color: [0.2, 0.8, 1.0],
            emissive_intensity: 2.0,
            ..Default::default()
        });
        self.next_id += 1;

        // Sample library materials
        self.library_materials = vec![
            "ground/grass.mat".to_string(),
            "ground/dirt.mat".to_string(),
            "ground/stone.mat".to_string(),
            "metal/steel.mat".to_string(),
            "metal/copper.mat".to_string(),
            "wood/oak.mat".to_string(),
            "wood/pine.mat".to_string(),
            "fabric/cloth.mat".to_string(),
            "fabric/leather.mat".to_string(),
        ];

        self.current_material = self.materials[0].clone();
        self.selected_material = Some(self.materials[0].id);
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (MaterialTab::Properties, "Properties"),
                (MaterialTab::Textures, "Textures"),
                (MaterialTab::Advanced, "Advanced"),
                (MaterialTab::Presets, "Presets"),
                (MaterialTab::Preview, "Preview"),
                (MaterialTab::Library, "Library"),
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

        // Material info
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} {}",
                self.current_material.material_type.icon(),
                self.current_material.name
            ));
            ui.separator();
            ui.label(format!("Type: {:?}", self.current_material.material_type));
            ui.label(format!("| Blend: {:?}", self.current_material.blend_mode));
        });

        ui.separator();
    }

    fn show_properties_tab(&mut self, ui: &mut Ui) {
        ui.heading("Material Properties");
        ui.add_space(10.0);

        // Material selector
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("material_select")
                .selected_text(&self.current_material.name)
                .show_ui(ui, |ui| {
                    for mat in &self.materials.clone() {
                        if ui
                            .selectable_value(&mut self.selected_material, Some(mat.id), &mat.name)
                            .clicked()
                        {
                            self.current_material = mat.clone();
                        }
                    }
                });

            if ui.button("+ New").clicked() {
                let id = self.next_id();
                info!(target: "aw_editor::panels::material_editor_panel", material_id = id, "New material created");
                let new_mat = Material {
                    id,
                    name: format!("Material {}", id),
                    ..Default::default()
                };
                self.materials.push(new_mat.clone());
                self.current_material = new_mat;
                self.selected_material = Some(id);
            }
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Basic properties
                ui.group(|ui| {
                    ui.label(RichText::new("[Edit] Basic").strong());

                    egui::Grid::new("basic_props")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.current_material.name);
                            ui.end_row();

                            ui.label("Type:");
                            egui::ComboBox::from_id_salt("mat_type")
                                .selected_text(format!(
                                    "{} {:?}",
                                    self.current_material.material_type.icon(),
                                    self.current_material.material_type
                                ))
                                .show_ui(ui, |ui| {
                                    for t in MaterialType::all() {
                                        ui.selectable_value(
                                            &mut self.current_material.material_type,
                                            *t,
                                            format!("{} {:?}", t.icon(), t),
                                        );
                                    }
                                });
                            ui.end_row();

                            ui.label("Blend Mode:");
                            egui::ComboBox::from_id_salt("blend_mode")
                                .selected_text(format!("{:?}", self.current_material.blend_mode))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.current_material.blend_mode,
                                        BlendMode::Opaque,
                                        "Opaque",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_material.blend_mode,
                                        BlendMode::Masked,
                                        "Masked",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_material.blend_mode,
                                        BlendMode::Translucent,
                                        "Translucent",
                                    );
                                    ui.selectable_value(
                                        &mut self.current_material.blend_mode,
                                        BlendMode::Additive,
                                        "Additive",
                                    );
                                });
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // PBR properties
                ui.group(|ui| {
                    ui.label(RichText::new("PBR Properties").strong());

                    egui::Grid::new("pbr_props")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Base Color:");
                            let mut color = Color32::from_rgba_unmultiplied(
                                (self.current_material.base_color[0] * 255.0) as u8,
                                (self.current_material.base_color[1] * 255.0) as u8,
                                (self.current_material.base_color[2] * 255.0) as u8,
                                (self.current_material.base_color[3] * 255.0) as u8,
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                self.current_material.base_color = [
                                    color.r() as f32 / 255.0,
                                    color.g() as f32 / 255.0,
                                    color.b() as f32 / 255.0,
                                    color.a() as f32 / 255.0,
                                ];
                            }
                            ui.end_row();

                            ui.label("Metallic:");
                            ui.add(egui::Slider::new(
                                &mut self.current_material.metallic,
                                0.0..=1.0,
                            ));
                            ui.end_row();

                            ui.label("Roughness:");
                            ui.add(egui::Slider::new(
                                &mut self.current_material.roughness,
                                0.0..=1.0,
                            ));
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Emissive
                ui.group(|ui| {
                    ui.label(RichText::new("Emissive").strong());

                    egui::Grid::new("emissive_props")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Color:");
                            let mut color = Color32::from_rgb(
                                (self.current_material.emissive_color[0] * 255.0) as u8,
                                (self.current_material.emissive_color[1] * 255.0) as u8,
                                (self.current_material.emissive_color[2] * 255.0) as u8,
                            );
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                self.current_material.emissive_color = [
                                    color.r() as f32 / 255.0,
                                    color.g() as f32 / 255.0,
                                    color.b() as f32 / 255.0,
                                ];
                            }
                            ui.end_row();

                            ui.label("Intensity:");
                            ui.add(egui::Slider::new(
                                &mut self.current_material.emissive_intensity,
                                0.0..=10.0,
                            ));
                            ui.end_row();
                        });
                });
            });
    }

    fn show_textures_tab(&mut self, ui: &mut Ui) {
        ui.heading("Texture Slots");
        ui.add_space(10.0);

        // Add texture slot
        ui.horizontal(|ui| {
            if ui.button("+ Add Texture Slot").clicked() {
                info!(target: "aw_editor::panels::material_editor_panel", material = %self.current_material.name, "Texture slot added");
                self.current_material
                    .texture_slots
                    .push(TextureSlot::default());
            }
        });

        ui.add_space(10.0);

        if self.current_material.texture_slots.is_empty() {
            ui.label("No texture slots. Click '+ Add Texture Slot' to add one.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    let mut to_remove = None;

                    for (idx, slot) in self.current_material.texture_slots.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut slot.enabled, "");
                                ui.label(RichText::new(slot.channel.name()).strong());

                                if ui.button("[Del]").clicked() {
                                    info!(target: "aw_editor::panels::material_editor_panel", slot_index = idx, channel = %slot.channel.name(), "Texture slot removed");
                                    to_remove = Some(idx);
                                }
                            });

                            if slot.enabled {
                                egui::Grid::new(format!("texture_{}", idx))
                                    .num_columns(2)
                                    .spacing([10.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label("Channel:");
                                        egui::ComboBox::from_id_salt(format!("channel_{}", idx))
                                            .selected_text(slot.channel.name())
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut slot.channel,
                                                    TextureChannel::Albedo,
                                                    "Albedo",
                                                );
                                                ui.selectable_value(
                                                    &mut slot.channel,
                                                    TextureChannel::Normal,
                                                    "Normal",
                                                );
                                                ui.selectable_value(
                                                    &mut slot.channel,
                                                    TextureChannel::Metallic,
                                                    "Metallic",
                                                );
                                                ui.selectable_value(
                                                    &mut slot.channel,
                                                    TextureChannel::Roughness,
                                                    "Roughness",
                                                );
                                                ui.selectable_value(
                                                    &mut slot.channel,
                                                    TextureChannel::AO,
                                                    "Ambient Occlusion",
                                                );
                                                ui.selectable_value(
                                                    &mut slot.channel,
                                                    TextureChannel::Emissive,
                                                    "Emissive",
                                                );
                                                ui.selectable_value(
                                                    &mut slot.channel,
                                                    TextureChannel::Height,
                                                    "Height",
                                                );
                                                ui.selectable_value(
                                                    &mut slot.channel,
                                                    TextureChannel::Opacity,
                                                    "Opacity",
                                                );
                                            });
                                        ui.end_row();

                                        ui.label("Path:");
                                        ui.horizontal(|ui| {
                                            ui.text_edit_singleline(&mut slot.texture_path);
                                            if ui.button("[Open]").clicked() {
                                                // Open file dialog
                                            }
                                        });
                                        ui.end_row();

                                        ui.label("Tiling:");
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut slot.tiling.0)
                                                    .prefix("U:")
                                                    .speed(0.1),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut slot.tiling.1)
                                                    .prefix("V:")
                                                    .speed(0.1),
                                            );
                                        });
                                        ui.end_row();

                                        ui.label("Offset:");
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut slot.offset.0)
                                                    .prefix("U:")
                                                    .speed(0.01),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut slot.offset.1)
                                                    .prefix("V:")
                                                    .speed(0.01),
                                            );
                                        });
                                        ui.end_row();

                                        ui.label("Intensity:");
                                        ui.add(egui::Slider::new(&mut slot.intensity, 0.0..=2.0));
                                        ui.end_row();
                                    });
                            }
                        });
                    }

                    if let Some(idx) = to_remove {
                        self.current_material.texture_slots.remove(idx);
                    }
                });
        }
    }

    fn show_advanced_tab(&mut self, ui: &mut Ui) {
        ui.heading("Advanced Properties");
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Rendering options
                ui.group(|ui| {
                    ui.label(RichText::new("Rendering").strong());

                    ui.checkbox(&mut self.current_material.two_sided, "Two Sided");
                    ui.checkbox(&mut self.current_material.cast_shadows, "Cast Shadows");
                    ui.checkbox(
                        &mut self.current_material.receive_shadows,
                        "Receive Shadows",
                    );

                    if self.current_material.blend_mode == BlendMode::Masked {
                        ui.horizontal(|ui| {
                            ui.label("Alpha Cutoff:");
                            ui.add(egui::Slider::new(
                                &mut self.current_material.alpha_cutoff,
                                0.0..=1.0,
                            ));
                        });
                    }
                });

                ui.add_space(10.0);

                // Subsurface scattering (for skin, wax, etc.)
                if self.current_material.material_type == MaterialType::Subsurface {
                    ui.group(|ui| {
                        ui.label(RichText::new("🧴 Subsurface Scattering").strong());

                        egui::Grid::new("sss_props")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Color:");
                                let mut color = Color32::from_rgb(
                                    (self.current_material.subsurface_color[0] * 255.0) as u8,
                                    (self.current_material.subsurface_color[1] * 255.0) as u8,
                                    (self.current_material.subsurface_color[2] * 255.0) as u8,
                                );
                                if ui.color_edit_button_srgba(&mut color).changed() {
                                    self.current_material.subsurface_color = [
                                        color.r() as f32 / 255.0,
                                        color.g() as f32 / 255.0,
                                        color.b() as f32 / 255.0,
                                    ];
                                }
                                ui.end_row();

                                ui.label("Radius:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_material.subsurface_radius,
                                    0.0..=5.0,
                                ));
                                ui.end_row();
                            });
                    });

                    ui.add_space(10.0);
                }

                // Glass/Translucent properties
                if self.current_material.material_type == MaterialType::Glass
                    || self.current_material.blend_mode == BlendMode::Translucent
                {
                    ui.group(|ui| {
                        ui.label(RichText::new("Transmission").strong());

                        egui::Grid::new("trans_props")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("IOR:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_material.ior,
                                    1.0..=3.0,
                                ));
                                ui.end_row();

                                ui.label("Transmission:");
                                ui.add(egui::Slider::new(
                                    &mut self.current_material.transmission,
                                    0.0..=1.0,
                                ));
                                ui.end_row();
                            });
                    });
                }
            });
    }

    fn show_presets_tab(&mut self, ui: &mut Ui) {
        ui.heading("Material Presets");
        ui.add_space(10.0);

        // Filter
        ui.horizontal(|ui| {
            ui.label("[Srch]");
            ui.text_edit_singleline(&mut self.preset_filter);
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                let mut current_category = String::new();

                for preset in &self.presets {
                    // Filter
                    if !self.preset_filter.is_empty()
                        && !preset
                            .name
                            .to_lowercase()
                            .contains(&self.preset_filter.to_lowercase())
                    {
                        continue;
                    }

                    // Category header
                    if preset.category != current_category {
                        current_category = preset.category.clone();
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(&current_category)
                                .strong()
                                .color(Color32::from_rgb(150, 150, 200)),
                        );
                    }

                    ui.horizontal(|ui| {
                        // Color preview
                        let color = Color32::from_rgba_unmultiplied(
                            (preset.base_color[0] * 255.0) as u8,
                            (preset.base_color[1] * 255.0) as u8,
                            (preset.base_color[2] * 255.0) as u8,
                            255,
                        );
                        let color_rect = ui
                            .allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover())
                            .0;
                        ui.painter().rect_filled(color_rect, 3.0, color);

                        ui.label(format!("{} {}", preset.material_type.icon(), preset.name));

                        if ui.button("Apply").clicked() {
                            info!(target: "aw_editor::panels::material_editor_panel", preset = %preset.name, material = %self.current_material.name, "Material preset applied");
                            self.current_material.material_type = preset.material_type;
                            self.current_material.base_color = preset.base_color;
                            self.current_material.metallic = preset.metallic;
                            self.current_material.roughness = preset.roughness;
                        }
                    });
                }
            });
    }

    fn show_preview_tab(&mut self, ui: &mut Ui) {
        ui.heading("Material Preview");
        ui.add_space(10.0);

        // Preview controls
        ui.horizontal(|ui| {
            ui.label("Lighting:");
            egui::ComboBox::from_id_salt("preview_lighting")
                .selected_text(format!("{:?}", self.preview_lighting))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.preview_lighting,
                        PreviewLighting::Studio,
                        "Studio",
                    );
                    ui.selectable_value(
                        &mut self.preview_lighting,
                        PreviewLighting::Outdoor,
                        "Outdoor",
                    );
                    ui.selectable_value(
                        &mut self.preview_lighting,
                        PreviewLighting::Indoor,
                        "Indoor",
                    );
                    ui.selectable_value(
                        &mut self.preview_lighting,
                        PreviewLighting::Dramatic,
                        "Dramatic",
                    );
                });
        });

        ui.horizontal(|ui| {
            ui.label("Rotation:");
            ui.add(egui::Slider::new(&mut self.preview_rotation, 0.0..=360.0).suffix("°"));
        });

        ui.horizontal(|ui| {
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.preview_zoom, 0.5..=3.0));
        });

        ui.add_space(10.0);

        // Preview area (placeholder)
        let preview_size = Vec2::new(ui.available_width(), 200.0);
        let (rect, _response) = ui.allocate_exact_size(preview_size, egui::Sense::hover());

        let painter = ui.painter();
        painter.rect_filled(rect, 5.0, Color32::from_rgb(30, 30, 35));

        // Draw a simple sphere representation
        let center = rect.center();
        let radius = 60.0 * self.preview_zoom;

        let base_color = Color32::from_rgba_unmultiplied(
            (self.current_material.base_color[0] * 255.0) as u8,
            (self.current_material.base_color[1] * 255.0) as u8,
            (self.current_material.base_color[2] * 255.0) as u8,
            (self.current_material.base_color[3] * 255.0) as u8,
        );

        painter.circle_filled(center, radius, base_color);

        // Highlight (simulating metallic/roughness)
        let highlight_intensity = (1.0 - self.current_material.roughness) * 0.7;
        let highlight_color =
            Color32::from_rgba_unmultiplied(255, 255, 255, (highlight_intensity * 200.0) as u8);
        painter.circle_filled(
            egui::Pos2::new(center.x - radius * 0.3, center.y - radius * 0.3),
            radius * 0.3,
            highlight_color,
        );

        // Material info overlay
        painter.text(
            egui::Pos2::new(rect.min.x + 10.0, rect.max.y - 20.0),
            egui::Align2::LEFT_BOTTOM,
            format!(
                "M: {:.2}  R: {:.2}",
                self.current_material.metallic, self.current_material.roughness
            ),
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );
    }

    fn show_library_tab(&mut self, ui: &mut Ui) {
        ui.heading("Material Library");
        ui.add_space(10.0);

        // Library path
        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.text_edit_singleline(&mut self.library_path);
            if ui.button("Refresh").clicked() {
                // Refresh library
            }
        });

        ui.add_space(10.0);

        // Actions
        ui.horizontal(|ui| {
            if ui.button("Save Current").clicked() {
                info!(target: "aw_editor::panels::material_editor_panel", material = %self.current_material.name, "Material saved to library");
                // Save current material
            }
            if ui.button("Load").clicked() {
                info!(target: "aw_editor::panels::material_editor_panel", "Material load requested");
                // Load material
            }
            if ui.button("Duplicate").clicked() {
                info!(target: "aw_editor::panels::material_editor_panel", material = %self.current_material.name, "Material duplicated");
                // Duplicate material
            }
        });

        ui.add_space(10.0);

        // Library list
        ui.group(|ui| {
            ui.label(RichText::new("Available Materials").strong());

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for mat_path in &self.library_materials {
                        ui.horizontal(|ui| {
                            ui.label("[Doc]");
                            if ui.selectable_label(false, mat_path).clicked() {
                                // Load this material
                            }
                        });
                    }
                });
        });
    }

    // Getters for testing
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }

    pub fn preset_count(&self) -> usize {
        self.presets.len()
    }

    pub fn current_material_name(&self) -> &str {
        &self.current_material.name
    }

    pub fn texture_slot_count(&self) -> usize {
        self.current_material.texture_slots.len()
    }

    pub fn add_material(&mut self, name: &str) -> u32 {
        let id = self.next_id();
        self.materials.push(Material {
            id,
            name: name.to_string(),
            ..Default::default()
        });
        id
    }

    pub fn set_metallic(&mut self, value: f32) {
        self.current_material.metallic = value.clamp(0.0, 1.0);
    }

    pub fn set_roughness(&mut self, value: f32) {
        self.current_material.roughness = value.clamp(0.0, 1.0);
    }

    pub fn add_texture_slot(&mut self, channel: TextureChannel, path: &str) {
        self.current_material.texture_slots.push(TextureSlot {
            channel,
            texture_path: path.to_string(),
            ..Default::default()
        });
    }
}

impl Panel for MaterialEditorPanel {
    fn name(&self) -> &'static str {
        "Material Editor"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            MaterialTab::Properties => self.show_properties_tab(ui),
            MaterialTab::Textures => self.show_textures_tab(ui),
            MaterialTab::Advanced => self.show_advanced_tab(ui),
            MaterialTab::Presets => self.show_presets_tab(ui),
            MaterialTab::Preview => self.show_preview_tab(ui),
            MaterialTab::Library => self.show_library_tab(ui),
        }
    }

    fn update(&mut self) {
        // Could auto-update preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_editor_panel_creation() {
        let panel = MaterialEditorPanel::new();
        assert!(!panel.current_material_name().is_empty());
    }

    #[test]
    fn test_default_sample_data() {
        let mut panel = MaterialEditorPanel::new();
        panel.create_sample_data();
        assert!(panel.material_count() >= 3);
        assert!(panel.preset_count() >= 10);
    }

    #[test]
    fn test_add_material() {
        let mut panel = MaterialEditorPanel::new();
        let initial_count = panel.material_count();

        let id = panel.add_material("Test Material");
        assert!(id > 0);
        assert_eq!(panel.material_count(), initial_count + 1);
    }

    #[test]
    fn test_set_metallic() {
        let mut panel = MaterialEditorPanel::new();
        panel.set_metallic(0.75);
        assert!((panel.current_material.metallic - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_set_roughness() {
        let mut panel = MaterialEditorPanel::new();
        panel.set_roughness(0.3);
        assert!((panel.current_material.roughness - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_add_texture_slot() {
        let mut panel = MaterialEditorPanel::new();
        let initial_count = panel.texture_slot_count();

        panel.add_texture_slot(TextureChannel::Normal, "textures/normal.png");
        assert_eq!(panel.texture_slot_count(), initial_count + 1);
    }

    #[test]
    fn test_material_type_icons() {
        assert_eq!(MaterialType::StandardPBR.icon(), "[Art]");
        assert_eq!(MaterialType::Glass.icon(), "[Orb]");
        assert_eq!(MaterialType::Water.icon(), "💧");
    }

    #[test]
    fn test_texture_channel_names() {
        assert_eq!(TextureChannel::Albedo.name(), "Albedo");
        assert_eq!(TextureChannel::Normal.name(), "Normal");
        assert_eq!(TextureChannel::Roughness.name(), "Roughness");
    }

    #[test]
    fn test_metallic_clamping() {
        let mut panel = MaterialEditorPanel::new();
        panel.set_metallic(1.5);
        assert!((panel.current_material.metallic - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = MaterialEditorPanel::new();
        assert_eq!(panel.name(), "Material Editor");
    }

    // ============================================================
    // ROUND 10 ENUM TESTS
    // ============================================================

    // MaterialType tests (7 tests)
    #[test]
    fn test_material_type_display() {
        assert_eq!(
            format!("{}", MaterialType::StandardPBR),
            "[Art] Standard PBR"
        );
        assert_eq!(format!("{}", MaterialType::Unlit), "[Lgt] Unlit");
        assert_eq!(format!("{}", MaterialType::Subsurface), "🧴 Subsurface");
        assert_eq!(format!("{}", MaterialType::Glass), "[Orb] Glass");
        assert_eq!(format!("{}", MaterialType::Water), "💧 Water");
        assert_eq!(format!("{}", MaterialType::Foliage), "[Leaf] Foliage");
        assert_eq!(format!("{}", MaterialType::Hair), "💇 Hair");
        assert_eq!(format!("{}", MaterialType::Cloth), "👕 Cloth");
        assert_eq!(format!("{}", MaterialType::Terrain), "🏔️ Terrain");
        assert_eq!(format!("{}", MaterialType::Decal), "[Tag] Decal");
    }

    #[test]
    fn test_material_type_all() {
        let all = MaterialType::all();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&MaterialType::StandardPBR));
        assert!(all.contains(&MaterialType::Decal));
    }

    #[test]
    fn test_material_type_name() {
        assert_eq!(MaterialType::StandardPBR.name(), "Standard PBR");
        assert_eq!(MaterialType::Unlit.name(), "Unlit");
        assert_eq!(MaterialType::Glass.name(), "Glass");
    }

    #[test]
    fn test_material_type_icon() {
        assert_eq!(MaterialType::StandardPBR.icon(), "[Art]");
        assert_eq!(MaterialType::Water.icon(), "💧");
        assert_eq!(MaterialType::Decal.icon(), "[Tag]");
    }

    #[test]
    fn test_material_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for mat in MaterialType::all() {
            assert!(set.insert(*mat));
        }
        assert_eq!(set.len(), 10);
    }

    #[test]
    fn test_material_type_default() {
        assert_eq!(MaterialType::default(), MaterialType::StandardPBR);
    }

    #[test]
    fn test_material_type_coverage() {
        for mat in MaterialType::all() {
            assert!(!mat.name().is_empty());
            assert!(!mat.icon().is_empty());
        }
    }

    // BlendMode tests (8 tests)
    #[test]
    fn test_blend_mode_display() {
        assert_eq!(format!("{}", BlendMode::Opaque), "⬛ Opaque");
        assert_eq!(format!("{}", BlendMode::Masked), "[Mask] Masked");
        assert_eq!(format!("{}", BlendMode::Translucent), "[Sq] Translucent");
        assert_eq!(format!("{}", BlendMode::Additive), "+ Additive");
        assert_eq!(format!("{}", BlendMode::Modulate), "[Shuf] Modulate");
    }

    #[test]
    fn test_blend_mode_all() {
        let all = BlendMode::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&BlendMode::Opaque));
        assert!(all.contains(&BlendMode::Modulate));
    }

    #[test]
    fn test_blend_mode_name() {
        assert_eq!(BlendMode::Opaque.name(), "Opaque");
        assert_eq!(BlendMode::Masked.name(), "Masked");
        assert_eq!(BlendMode::Translucent.name(), "Translucent");
        assert_eq!(BlendMode::Additive.name(), "Additive");
        assert_eq!(BlendMode::Modulate.name(), "Modulate");
    }

    #[test]
    fn test_blend_mode_icon() {
        assert_eq!(BlendMode::Opaque.icon(), "⬛");
        assert_eq!(BlendMode::Masked.icon(), "[Mask]");
        assert_eq!(BlendMode::Translucent.icon(), "[Sq]");
        assert_eq!(BlendMode::Additive.icon(), "+");
        assert_eq!(BlendMode::Modulate.icon(), "[Shuf]");
    }

    #[test]
    fn test_blend_mode_is_transparent() {
        assert!(!BlendMode::Opaque.is_transparent());
        assert!(!BlendMode::Masked.is_transparent());
        assert!(BlendMode::Translucent.is_transparent());
        assert!(BlendMode::Additive.is_transparent());
        assert!(BlendMode::Modulate.is_transparent());
    }

    #[test]
    fn test_blend_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for blend in BlendMode::all() {
            assert!(set.insert(*blend));
        }
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_blend_mode_default() {
        assert_eq!(BlendMode::default(), BlendMode::Opaque);
    }

    #[test]
    fn test_blend_mode_transparency_types() {
        let transparent: Vec<_> = BlendMode::all()
            .iter()
            .filter(|b| b.is_transparent())
            .collect();
        assert_eq!(transparent.len(), 3);
    }

    // TextureChannel tests (7 tests)
    #[test]
    fn test_texture_channel_display() {
        assert_eq!(format!("{}", TextureChannel::Albedo), "[Art] Albedo");
        assert_eq!(format!("{}", TextureChannel::Normal), "🗺️ Normal");
        assert_eq!(format!("{}", TextureChannel::Metallic), "[Fx] Metallic");
        assert_eq!(format!("{}", TextureChannel::Roughness), "🔨 Roughness");
        assert_eq!(format!("{}", TextureChannel::AO), "🌑 Ambient Occlusion");
        assert_eq!(format!("{}", TextureChannel::Emissive), "[Lgt] Emissive");
        assert_eq!(format!("{}", TextureChannel::Height), "⛰️ Height");
        assert_eq!(format!("{}", TextureChannel::Opacity), "👻 Opacity");
    }

    #[test]
    fn test_texture_channel_all() {
        let all = TextureChannel::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&TextureChannel::Albedo));
        assert!(all.contains(&TextureChannel::Opacity));
    }

    #[test]
    fn test_texture_channel_name() {
        assert_eq!(TextureChannel::Albedo.name(), "Albedo");
        assert_eq!(TextureChannel::Normal.name(), "Normal");
        assert_eq!(TextureChannel::AO.name(), "Ambient Occlusion");
    }

    #[test]
    fn test_texture_channel_icon() {
        assert_eq!(TextureChannel::Albedo.icon(), "[Art]");
        assert_eq!(TextureChannel::Normal.icon(), "🗺️");
        assert_eq!(TextureChannel::Metallic.icon(), "[Fx]");
        assert_eq!(TextureChannel::Emissive.icon(), "[Lgt]");
    }

    #[test]
    fn test_texture_channel_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for channel in TextureChannel::all() {
            assert!(set.insert(*channel));
        }
        assert_eq!(set.len(), 8);
    }

    #[test]
    fn test_texture_channel_coverage() {
        for channel in TextureChannel::all() {
            assert!(!channel.name().is_empty());
            assert!(!channel.icon().is_empty());
        }
    }

    #[test]
    fn test_texture_channel_pbr_channels() {
        let pbr_channels = [
            TextureChannel::Albedo,
            TextureChannel::Normal,
            TextureChannel::Metallic,
            TextureChannel::Roughness,
            TextureChannel::AO,
        ];
        for channel in &pbr_channels {
            assert!(TextureChannel::all().contains(channel));
        }
    }

    // PreviewLighting tests (7 tests)
    #[test]
    fn test_preview_lighting_display() {
        assert_eq!(format!("{}", PreviewLighting::Studio), "[Anim] Studio");
        assert_eq!(format!("{}", PreviewLighting::Outdoor), "[Sun] Outdoor");
        assert_eq!(format!("{}", PreviewLighting::Indoor), "[Lgt] Indoor");
        assert_eq!(format!("{}", PreviewLighting::Dramatic), "[Mask] Dramatic");
        assert_eq!(format!("{}", PreviewLighting::Custom), "⚙️ Custom");
    }

    #[test]
    fn test_preview_lighting_all() {
        let all = PreviewLighting::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&PreviewLighting::Studio));
        assert!(all.contains(&PreviewLighting::Custom));
    }

    #[test]
    fn test_preview_lighting_name() {
        assert_eq!(PreviewLighting::Studio.name(), "Studio");
        assert_eq!(PreviewLighting::Outdoor.name(), "Outdoor");
        assert_eq!(PreviewLighting::Indoor.name(), "Indoor");
        assert_eq!(PreviewLighting::Dramatic.name(), "Dramatic");
        assert_eq!(PreviewLighting::Custom.name(), "Custom");
    }

    #[test]
    fn test_preview_lighting_icon() {
        assert_eq!(PreviewLighting::Studio.icon(), "[Anim]");
        assert_eq!(PreviewLighting::Outdoor.icon(), "[Sun]");
        assert_eq!(PreviewLighting::Indoor.icon(), "[Lgt]");
        assert_eq!(PreviewLighting::Dramatic.icon(), "[Mask]");
        assert_eq!(PreviewLighting::Custom.icon(), "⚙️");
    }

    #[test]
    fn test_preview_lighting_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for lighting in PreviewLighting::all() {
            assert!(set.insert(*lighting));
        }
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_preview_lighting_default() {
        assert_eq!(PreviewLighting::default(), PreviewLighting::Studio);
    }

    #[test]
    fn test_preview_lighting_coverage() {
        for lighting in PreviewLighting::all() {
            assert!(!lighting.name().is_empty());
            assert!(!lighting.icon().is_empty());
        }
    }

    // MaterialTab tests (6 tests)
    #[test]
    fn test_material_tab_display() {
        assert_eq!(format!("{}", MaterialTab::Properties), "[Art] Properties");
        assert_eq!(format!("{}", MaterialTab::Textures), "[Img] Textures");
        assert_eq!(format!("{}", MaterialTab::Advanced), "⚙️ Advanced");
        assert_eq!(format!("{}", MaterialTab::Presets), "[List] Presets");
        assert_eq!(format!("{}", MaterialTab::Preview), "👁️ Preview");
        assert_eq!(format!("{}", MaterialTab::Library), "📚 Library");
    }

    #[test]
    fn test_material_tab_all() {
        let all = MaterialTab::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&MaterialTab::Properties));
        assert!(all.contains(&MaterialTab::Library));
    }

    #[test]
    fn test_material_tab_name() {
        assert_eq!(MaterialTab::Properties.name(), "Properties");
        assert_eq!(MaterialTab::Textures.name(), "Textures");
        assert_eq!(MaterialTab::Advanced.name(), "Advanced");
        assert_eq!(MaterialTab::Presets.name(), "Presets");
        assert_eq!(MaterialTab::Preview.name(), "Preview");
        assert_eq!(MaterialTab::Library.name(), "Library");
    }

    #[test]
    fn test_material_tab_icon() {
        assert_eq!(MaterialTab::Properties.icon(), "[Art]");
        assert_eq!(MaterialTab::Textures.icon(), "[Img]");
        assert_eq!(MaterialTab::Advanced.icon(), "⚙️");
        assert_eq!(MaterialTab::Presets.icon(), "[List]");
        assert_eq!(MaterialTab::Preview.icon(), "👁️");
        assert_eq!(MaterialTab::Library.icon(), "📚");
    }

    #[test]
    fn test_material_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in MaterialTab::all() {
            assert!(set.insert(*tab));
        }
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn test_material_tab_default() {
        assert_eq!(MaterialTab::default(), MaterialTab::Properties);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // MaterialEditorAction Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_material_editor_action_display() {
        let action = MaterialEditorAction::SetActiveTab(MaterialTab::Properties);
        let display = format!("{}", action);
        assert!(display.contains("tab"));
    }

    #[test]
    fn test_material_editor_action_display_all_variants() {
        let actions = vec![
            MaterialEditorAction::SetActiveTab(MaterialTab::Textures),
            MaterialEditorAction::AddMaterial,
            MaterialEditorAction::RemoveMaterial(0),
            MaterialEditorAction::SelectMaterial(1),
            MaterialEditorAction::SetBaseColor(0, [1.0, 0.5, 0.0, 1.0]),
            MaterialEditorAction::SetMetallic(0, 0.8),
            MaterialEditorAction::ApplyPreset("Metal".to_string()),
            MaterialEditorAction::RefreshLibrary,
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
    fn test_material_editor_action_equality() {
        let action1 = MaterialEditorAction::SelectMaterial(5);
        let action2 = MaterialEditorAction::SelectMaterial(5);
        let action3 = MaterialEditorAction::SelectMaterial(10);

        assert_eq!(action1, action2);
        assert_ne!(action1, action3);
    }

    #[test]
    fn test_material_editor_action_clone() {
        let action = MaterialEditorAction::SetMaterialName(0, "TestMaterial".to_string());
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_material_editor_panel_pending_actions_empty_by_default() {
        let panel = MaterialEditorPanel::new();
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_material_editor_panel_queue_action() {
        let mut panel = MaterialEditorPanel::new();
        panel.queue_action(MaterialEditorAction::AddMaterial);
        assert!(panel.has_pending_actions());
        assert_eq!(panel.pending_actions().len(), 1);
    }

    #[test]
    fn test_material_editor_panel_take_actions() {
        let mut panel = MaterialEditorPanel::new();
        panel.queue_action(MaterialEditorAction::AddMaterial);
        panel.queue_action(MaterialEditorAction::SetActiveTab(MaterialTab::Textures));

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_material_editor_panel_action_order_preserved() {
        let mut panel = MaterialEditorPanel::new();
        panel.queue_action(MaterialEditorAction::AddMaterial);
        panel.queue_action(MaterialEditorAction::SelectMaterial(0));
        panel.queue_action(MaterialEditorAction::RemoveMaterial(0));

        let actions = panel.take_actions();
        assert!(matches!(actions[0], MaterialEditorAction::AddMaterial));
        assert!(matches!(
            actions[1],
            MaterialEditorAction::SelectMaterial(_)
        ));
        assert!(matches!(
            actions[2],
            MaterialEditorAction::RemoveMaterial(_)
        ));
    }

    #[test]
    fn test_material_editor_action_material_operations() {
        let actions = vec![
            MaterialEditorAction::AddMaterial,
            MaterialEditorAction::RemoveMaterial(0),
            MaterialEditorAction::SelectMaterial(1),
            MaterialEditorAction::DuplicateMaterial(2),
            MaterialEditorAction::SetMaterialName(0, "NewMaterial".to_string()),
            MaterialEditorAction::SetMaterialType(0, MaterialType::StandardPBR),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_material_editor_action_property_operations() {
        let actions = vec![
            MaterialEditorAction::SetBaseColor(0, [1.0, 0.0, 0.0, 1.0]),
            MaterialEditorAction::SetMetallic(0, 0.9),
            MaterialEditorAction::SetRoughness(0, 0.3),
            MaterialEditorAction::SetNormalStrength(0, 1.5),
            MaterialEditorAction::SetAoStrength(0, 1.0),
            MaterialEditorAction::SetBlendMode(0, BlendMode::Translucent),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("base color"));
        assert!(displays[1].contains("0.9"));
        assert!(displays[5].contains("Translucent"));
    }

    #[test]
    fn test_material_editor_action_emissive_operations() {
        let actions = vec![
            MaterialEditorAction::SetEmissiveColor(0, [0.0, 1.0, 1.0]),
            MaterialEditorAction::SetEmissiveIntensity(0, 5.0),
            MaterialEditorAction::ToggleEmissive(0, true),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("emissive"));
        assert!(displays[2].contains("emissive"));
    }

    #[test]
    fn test_material_editor_action_texture_operations() {
        let actions = vec![
            MaterialEditorAction::SetTexture(0, TextureChannel::Albedo, "diffuse.png".to_string()),
            MaterialEditorAction::RemoveTexture(0, TextureChannel::Normal),
            MaterialEditorAction::SetTextureScale(0, 2.0, 2.0),
            MaterialEditorAction::SetTextureOffset(0, 0.5, 0.0),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Albedo"));
        assert!(displays[1].contains("Normal"));
    }

    #[test]
    fn test_material_editor_action_advanced_operations() {
        let actions = vec![
            MaterialEditorAction::ToggleTwoSided(0, true),
            MaterialEditorAction::ToggleCastShadow(0, false),
            MaterialEditorAction::ToggleReceiveShadow(0, true),
            MaterialEditorAction::SetRenderQueue(0, 2500),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("two-sided"));
        assert!(displays[3].contains("2500"));
    }

    #[test]
    fn test_material_editor_action_preset_and_library() {
        let actions = vec![
            MaterialEditorAction::ApplyPreset("Plastic".to_string()),
            MaterialEditorAction::SaveAsPreset(0, "CustomMetal".to_string()),
            MaterialEditorAction::RefreshLibrary,
            MaterialEditorAction::ImportMaterial("material_123.mat".to_string()),
            MaterialEditorAction::SetPreviewLighting(PreviewLighting::Studio),
            MaterialEditorAction::SetPreviewRotation(45.0),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Plastic"));
        assert!(displays[4].contains("preview"));
    }
}
