//! Import Doctor Panel - Asset Import Wizard for the editor
//!
//! Provides comprehensive asset import validation and auto-fixing:
//! - **Convention Detection**: Unity, Unreal, Substance, Blender naming conventions
//! - **Texture Packing**: Auto-detect and convert ORM/RMA/MRA channel arrangements
//! - **One-Click Fixes**: "Treat as Normal Map", "Flip Green Channel", etc.
//! - **Tangent Generation**: Auto-generate missing tangents for meshes
//! - **Power-of-Two Warnings**: Detect non-power-of-two textures
//! - **Live Preview**: Show imported asset with default lighting
//!
//! The goal is to make any externally-created asset work correctly in AstraWeave
//! with minimal manual intervention.

use egui::{Color32, RichText, Ui, Vec2};
use std::path::PathBuf;
use tracing::info;

use crate::panels::Panel;

// ============================================================================
// PANEL ACTIONS - Events produced by the panel for external handling
// ============================================================================

/// Actions emitted by the import doctor panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ImportAction {
    /// Apply a specific quick fix
    ApplyQuickFix { fix: QuickFix, asset_path: PathBuf },
    /// Apply all auto-fixable issues
    ApplyAllFixes { asset_path: PathBuf },
    /// Import the asset with current settings
    ImportAsset {
        asset_path: PathBuf,
        settings: ImportSettings,
        applied_fixes: Vec<QuickFix>,
    },
    /// Clear the current selection
    ClearSelection,
    /// Request to scan an asset for issues
    ScanAsset { asset_path: PathBuf },
    /// Request 3D preview of asset
    PreviewAsset { asset_path: PathBuf },
}

// ============================================================================
// SOURCE ENGINE - What tool/engine created this asset
// ============================================================================

/// Detected source engine/tool for imported assets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SourceEngine {
    #[default]
    Unknown,
    Unity,
    Unreal,
    Blender,
    Maya,
    SubstancePainter,
    SubstanceDesigner,
    Quixel,
    ThreeDSMax,
    Cinema4D,
    ZBrush,
    Houdini,
    Photoshop,
    Custom,
}

impl std::fmt::Display for SourceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl SourceEngine {
    /// All source engines
    pub fn all() -> &'static [SourceEngine] {
        &[
            SourceEngine::Unknown,
            SourceEngine::Unity,
            SourceEngine::Unreal,
            SourceEngine::Blender,
            SourceEngine::Maya,
            SourceEngine::SubstancePainter,
            SourceEngine::SubstanceDesigner,
            SourceEngine::Quixel,
            SourceEngine::ThreeDSMax,
            SourceEngine::Cinema4D,
            SourceEngine::ZBrush,
            SourceEngine::Houdini,
            SourceEngine::Photoshop,
            SourceEngine::Custom,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            SourceEngine::Unknown => "Unknown",
            SourceEngine::Unity => "Unity",
            SourceEngine::Unreal => "Unreal Engine",
            SourceEngine::Blender => "Blender",
            SourceEngine::Maya => "Maya",
            SourceEngine::SubstancePainter => "Substance Painter",
            SourceEngine::SubstanceDesigner => "Substance Designer",
            SourceEngine::Quixel => "Quixel Mixer",
            SourceEngine::ThreeDSMax => "3DS Max",
            SourceEngine::Cinema4D => "Cinema 4D",
            SourceEngine::ZBrush => "ZBrush",
            SourceEngine::Houdini => "Houdini",
            SourceEngine::Photoshop => "Photoshop",
            SourceEngine::Custom => "Custom",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            SourceEngine::Unknown => "❓",
            SourceEngine::Unity => "[Gp]",
            SourceEngine::Unreal => "[Tgt]",
            SourceEngine::Blender => "🧊",
            SourceEngine::Maya => "[D]",
            SourceEngine::SubstancePainter => "[Art]",
            SourceEngine::SubstanceDesigner => "🔶",
            SourceEngine::Quixel => "[Net]",
            SourceEngine::ThreeDSMax => "[Sq]",
            SourceEngine::Cinema4D => "[Anim]",
            SourceEngine::ZBrush => "[Rock]",
            SourceEngine::Houdini => "💫",
            SourceEngine::Photoshop => "[Img]",
            SourceEngine::Custom => "⚙️",
        }
    }

    /// Whether this engine uses DirectX normal maps (green channel up)
    pub fn uses_directx_normals(&self) -> bool {
        matches!(
            self,
            SourceEngine::Unity
                | SourceEngine::Unreal
                | SourceEngine::ThreeDSMax
                | SourceEngine::Quixel
        )
    }

    /// Whether this engine uses OpenGL normal maps (green channel down)
    pub fn uses_opengl_normals(&self) -> bool {
        matches!(
            self,
            SourceEngine::Blender
                | SourceEngine::Maya
                | SourceEngine::SubstancePainter
                | SourceEngine::SubstanceDesigner
                | SourceEngine::Cinema4D
                | SourceEngine::Houdini
        )
    }

    /// Default texture packing format for this engine
    pub fn default_packing(&self) -> TexturePackingFormat {
        match self {
            SourceEngine::Unreal => TexturePackingFormat::ORM,
            SourceEngine::Unity => TexturePackingFormat::MRA,
            SourceEngine::SubstancePainter => TexturePackingFormat::ORM,
            SourceEngine::SubstanceDesigner => TexturePackingFormat::ORM,
            SourceEngine::Quixel => TexturePackingFormat::ORM,
            _ => TexturePackingFormat::Separate,
        }
    }

    /// Detect source engine from filename patterns
    pub fn from_filename(name: &str) -> Self {
        let lower = name.to_lowercase();

        if lower.contains("_unreal") || lower.contains("_ue4") || lower.contains("_ue5") {
            SourceEngine::Unreal
        } else if lower.contains("_unity") {
            SourceEngine::Unity
        } else if lower.contains("_blender") || lower.contains("_blend") {
            SourceEngine::Blender
        } else if lower.contains("_sp") || lower.contains("_substance") {
            SourceEngine::SubstancePainter
        } else if lower.contains("_quixel") || lower.contains("_megascans") {
            SourceEngine::Quixel
        } else if lower.contains("_maya") {
            SourceEngine::Maya
        } else if lower.contains("_max") || lower.contains("_3dsmax") {
            SourceEngine::ThreeDSMax
        } else if lower.contains("_c4d") || lower.contains("_cinema4d") {
            SourceEngine::Cinema4D
        } else if lower.contains("_houdini") {
            SourceEngine::Houdini
        } else if lower.contains("_zbrush") {
            SourceEngine::ZBrush
        } else {
            SourceEngine::Unknown
        }
    }
}

// ============================================================================
// TEXTURE PACKING FORMAT - How PBR channels are packed
// ============================================================================

/// Texture channel packing formats
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
#[non_exhaustive]
pub enum TexturePackingFormat {
    #[default]
    Separate,
    ORM, // Occlusion-Roughness-Metallic (Unreal, Substance)
    MRA, // Metallic-Roughness-AO (Unity)
    RMA, // Roughness-Metallic-AO (some custom workflows)
    ARM, // AO-Roughness-Metallic (alternative)
    MRO, // Metallic-Roughness-Occlusion (GLTF)
}

impl std::fmt::Display for TexturePackingFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TexturePackingFormat {
    /// All packing formats
    pub fn all() -> &'static [TexturePackingFormat] {
        &[
            TexturePackingFormat::Separate,
            TexturePackingFormat::ORM,
            TexturePackingFormat::MRA,
            TexturePackingFormat::RMA,
            TexturePackingFormat::ARM,
            TexturePackingFormat::MRO,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            TexturePackingFormat::Separate => "Separate",
            TexturePackingFormat::ORM => "ORM",
            TexturePackingFormat::MRA => "MRA",
            TexturePackingFormat::RMA => "RMA",
            TexturePackingFormat::ARM => "ARM",
            TexturePackingFormat::MRO => "MRO",
        }
    }

    /// Full description
    pub fn description(&self) -> &'static str {
        match self {
            TexturePackingFormat::Separate => "Separate textures for each channel",
            TexturePackingFormat::ORM => "R=Occlusion, G=Roughness, B=Metallic",
            TexturePackingFormat::MRA => "R=Metallic, G=Roughness, B=AO",
            TexturePackingFormat::RMA => "R=Roughness, G=Metallic, B=AO",
            TexturePackingFormat::ARM => "R=AO, G=Roughness, B=Metallic",
            TexturePackingFormat::MRO => "R=Metallic, G=Roughness, B=Occlusion (glTF)",
        }
    }

    /// Which channel contains occlusion/AO
    pub fn ao_channel(&self) -> Option<char> {
        match self {
            TexturePackingFormat::Separate => None,
            TexturePackingFormat::ORM => Some('R'),
            TexturePackingFormat::MRA => Some('B'),
            TexturePackingFormat::RMA => Some('B'),
            TexturePackingFormat::ARM => Some('R'),
            TexturePackingFormat::MRO => Some('B'),
        }
    }

    /// Which channel contains roughness
    pub fn roughness_channel(&self) -> Option<char> {
        match self {
            TexturePackingFormat::Separate => None,
            TexturePackingFormat::ORM => Some('G'),
            TexturePackingFormat::MRA => Some('G'),
            TexturePackingFormat::RMA => Some('R'),
            TexturePackingFormat::ARM => Some('G'),
            TexturePackingFormat::MRO => Some('G'),
        }
    }

    /// Which channel contains metallic
    pub fn metallic_channel(&self) -> Option<char> {
        match self {
            TexturePackingFormat::Separate => None,
            TexturePackingFormat::ORM => Some('B'),
            TexturePackingFormat::MRA => Some('R'),
            TexturePackingFormat::RMA => Some('G'),
            TexturePackingFormat::ARM => Some('B'),
            TexturePackingFormat::MRO => Some('R'),
        }
    }

    /// Detect from filename
    pub fn from_filename(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.contains("_orm") {
            TexturePackingFormat::ORM
        } else if lower.contains("_mra") {
            TexturePackingFormat::MRA
        } else if lower.contains("_rma") {
            TexturePackingFormat::RMA
        } else if lower.contains("_arm") {
            TexturePackingFormat::ARM
        } else if lower.contains("_mro") {
            TexturePackingFormat::MRO
        } else {
            TexturePackingFormat::Separate
        }
    }
}

// ============================================================================
// IMPORT ISSUE - Problems detected during import
// ============================================================================

/// Severity of import issues
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum IssueSeverity {
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl IssueSeverity {
    /// All severity levels
    pub fn all() -> &'static [IssueSeverity] {
        &[
            IssueSeverity::Info,
            IssueSeverity::Warning,
            IssueSeverity::Error,
            IssueSeverity::Critical,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            IssueSeverity::Info => "Info",
            IssueSeverity::Warning => "Warning",
            IssueSeverity::Error => "Error",
            IssueSeverity::Critical => "Critical",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            IssueSeverity::Info => "ℹ️",
            IssueSeverity::Warning => "[!]",
            IssueSeverity::Error => "[x]",
            IssueSeverity::Critical => "🚨",
        }
    }

    /// Color for UI
    pub fn color(&self) -> Color32 {
        match self {
            IssueSeverity::Info => Color32::from_rgb(100, 150, 255),
            IssueSeverity::Warning => Color32::from_rgb(255, 200, 100),
            IssueSeverity::Error => Color32::from_rgb(255, 100, 100),
            IssueSeverity::Critical => Color32::from_rgb(255, 50, 50),
        }
    }

    /// Whether this issue blocks import
    pub fn blocks_import(&self) -> bool {
        matches!(self, IssueSeverity::Critical)
    }
}

/// Type of import issue
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IssueType {
    #[default]
    Unknown,
    NormalMapFormat,
    TexturePacking,
    MissingTangents,
    NonPowerOfTwo,
    MissingUVs,
    IncorrectScale,
    MissingCollider,
    IncorrectOrientation,
    MissingLODs,
    OversizedTexture,
    UnsupportedFormat,
    DuplicateMaterial,
    MissingTexture,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl IssueType {
    /// All issue types
    pub fn all() -> &'static [IssueType] {
        &[
            IssueType::Unknown,
            IssueType::NormalMapFormat,
            IssueType::TexturePacking,
            IssueType::MissingTangents,
            IssueType::NonPowerOfTwo,
            IssueType::MissingUVs,
            IssueType::IncorrectScale,
            IssueType::MissingCollider,
            IssueType::IncorrectOrientation,
            IssueType::MissingLODs,
            IssueType::OversizedTexture,
            IssueType::UnsupportedFormat,
            IssueType::DuplicateMaterial,
            IssueType::MissingTexture,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            IssueType::Unknown => "Unknown Issue",
            IssueType::NormalMapFormat => "Normal Map Format",
            IssueType::TexturePacking => "Texture Packing",
            IssueType::MissingTangents => "Missing Tangents",
            IssueType::NonPowerOfTwo => "Non-Power-of-Two",
            IssueType::MissingUVs => "Missing UVs",
            IssueType::IncorrectScale => "Incorrect Scale",
            IssueType::MissingCollider => "Missing Collider",
            IssueType::IncorrectOrientation => "Incorrect Orientation",
            IssueType::MissingLODs => "Missing LODs",
            IssueType::OversizedTexture => "Oversized Texture",
            IssueType::UnsupportedFormat => "Unsupported Format",
            IssueType::DuplicateMaterial => "Duplicate Material",
            IssueType::MissingTexture => "Missing Texture",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            IssueType::Unknown => "❓",
            IssueType::NormalMapFormat => "[B]",
            IssueType::TexturePacking => "[Pkg]",
            IssueType::MissingTangents => "[Sq]",
            IssueType::NonPowerOfTwo => "[!]",
            IssueType::MissingUVs => "🗺️",
            IssueType::IncorrectScale => "📏",
            IssueType::MissingCollider => "[Hit]",
            IssueType::IncorrectOrientation => "[Sync]",
            IssueType::MissingLODs => "👁️",
            IssueType::OversizedTexture => "[Sq]",
            IssueType::UnsupportedFormat => "🚫",
            IssueType::DuplicateMaterial => "🔁",
            IssueType::MissingTexture => "[Img]",
        }
    }

    /// Default severity for this issue type
    pub fn default_severity(&self) -> IssueSeverity {
        match self {
            IssueType::Unknown => IssueSeverity::Warning,
            IssueType::NormalMapFormat => IssueSeverity::Warning,
            IssueType::TexturePacking => IssueSeverity::Info,
            IssueType::MissingTangents => IssueSeverity::Warning,
            IssueType::NonPowerOfTwo => IssueSeverity::Warning,
            IssueType::MissingUVs => IssueSeverity::Error,
            IssueType::IncorrectScale => IssueSeverity::Warning,
            IssueType::MissingCollider => IssueSeverity::Info,
            IssueType::IncorrectOrientation => IssueSeverity::Warning,
            IssueType::MissingLODs => IssueSeverity::Info,
            IssueType::OversizedTexture => IssueSeverity::Warning,
            IssueType::UnsupportedFormat => IssueSeverity::Critical,
            IssueType::DuplicateMaterial => IssueSeverity::Info,
            IssueType::MissingTexture => IssueSeverity::Error,
        }
    }

    /// Whether this issue has an auto-fix available
    pub fn has_auto_fix(&self) -> bool {
        matches!(
            self,
            IssueType::NormalMapFormat
                | IssueType::TexturePacking
                | IssueType::MissingTangents
                | IssueType::NonPowerOfTwo
                | IssueType::IncorrectScale
                | IssueType::IncorrectOrientation
        )
    }

    /// Description of the auto-fix action
    pub fn fix_description(&self) -> Option<&'static str> {
        match self {
            IssueType::NormalMapFormat => Some("Flip green channel (DirectX ↔ OpenGL)"),
            IssueType::TexturePacking => Some("Repack texture channels"),
            IssueType::MissingTangents => Some("Generate tangents from UV data"),
            IssueType::NonPowerOfTwo => Some("Resize to nearest power-of-two"),
            IssueType::IncorrectScale => Some("Apply scale correction factor"),
            IssueType::IncorrectOrientation => Some("Apply rotation correction"),
            _ => None,
        }
    }

    /// Get the suggested quick fix for this issue type
    pub fn suggested_fix(&self) -> Option<QuickFix> {
        match self {
            IssueType::NormalMapFormat => Some(QuickFix::FlipGreenChannel),
            IssueType::TexturePacking => Some(QuickFix::ConvertToORM),
            IssueType::MissingTangents => Some(QuickFix::GenerateTangents),
            IssueType::NonPowerOfTwo => Some(QuickFix::ResizePowerOfTwo),
            IssueType::IncorrectScale => Some(QuickFix::FixScale),
            IssueType::IncorrectOrientation => Some(QuickFix::FixOrientation),
            IssueType::MissingLODs => Some(QuickFix::GenerateLODs),
            IssueType::MissingCollider => Some(QuickFix::GenerateCollider),
            _ => None,
        }
    }
}

// ============================================================================
// IMPORT ISSUE - A specific detected issue
// ============================================================================

/// A detected import issue with details
#[derive(Debug, Clone)]
pub struct ImportIssue {
    pub issue_type: IssueType,
    pub severity: IssueSeverity,
    pub message: String,
    pub file_path: Option<PathBuf>,
    pub can_auto_fix: bool,
    pub fix_applied: bool,
}

impl ImportIssue {
    /// Create a new import issue
    pub fn new(issue_type: IssueType, message: impl Into<String>) -> Self {
        Self {
            issue_type,
            severity: issue_type.default_severity(),
            message: message.into(),
            file_path: None,
            can_auto_fix: issue_type.has_auto_fix(),
            fix_applied: false,
        }
    }

    /// Set the file path for this issue
    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Set severity override
    pub fn with_severity(mut self, severity: IssueSeverity) -> Self {
        self.severity = severity;
        self
    }
}

// ============================================================================
// QUICK FIX - One-click conversion actions
// ============================================================================

/// Quick-fix actions for common import issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum QuickFix {
    TreatAsNormalMap,
    FlipGreenChannel,
    ConvertToORM,
    ConvertToMRA,
    GenerateTangents,
    ResizePowerOfTwo,
    GenerateLODs,
    GenerateCollider,
    FixScale,
    FixOrientation,
    MarkAsSRGB,
    MarkAsLinear,
}

impl std::fmt::Display for QuickFix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl QuickFix {
    /// All quick fixes
    pub fn all() -> &'static [QuickFix] {
        &[
            QuickFix::TreatAsNormalMap,
            QuickFix::FlipGreenChannel,
            QuickFix::ConvertToORM,
            QuickFix::ConvertToMRA,
            QuickFix::GenerateTangents,
            QuickFix::ResizePowerOfTwo,
            QuickFix::GenerateLODs,
            QuickFix::GenerateCollider,
            QuickFix::FixScale,
            QuickFix::FixOrientation,
            QuickFix::MarkAsSRGB,
            QuickFix::MarkAsLinear,
        ]
    }

    /// Display name
    pub fn name(&self) -> &'static str {
        match self {
            QuickFix::TreatAsNormalMap => "Treat as Normal Map",
            QuickFix::FlipGreenChannel => "Flip Green Channel",
            QuickFix::ConvertToORM => "Convert to ORM",
            QuickFix::ConvertToMRA => "Convert to MRA",
            QuickFix::GenerateTangents => "Generate Tangents",
            QuickFix::ResizePowerOfTwo => "Resize to Power-of-Two",
            QuickFix::GenerateLODs => "Generate LODs",
            QuickFix::GenerateCollider => "Generate Collider",
            QuickFix::FixScale => "Fix Scale",
            QuickFix::FixOrientation => "Fix Orientation",
            QuickFix::MarkAsSRGB => "Mark as sRGB",
            QuickFix::MarkAsLinear => "Mark as Linear",
        }
    }

    /// Icon for UI
    pub fn icon(&self) -> &'static str {
        match self {
            QuickFix::TreatAsNormalMap => "[B]",
            QuickFix::FlipGreenChannel => "↕️",
            QuickFix::ConvertToORM => "🔶",
            QuickFix::ConvertToMRA => "[D]",
            QuickFix::GenerateTangents => "[Sq]",
            QuickFix::ResizePowerOfTwo => "📏",
            QuickFix::GenerateLODs => "👁️",
            QuickFix::GenerateCollider => "[Hit]",
            QuickFix::FixScale => "📏",
            QuickFix::FixOrientation => "[Sync]",
            QuickFix::MarkAsSRGB => "[Art]",
            QuickFix::MarkAsLinear => "⬜",
        }
    }

    /// Detailed description
    pub fn description(&self) -> &'static str {
        match self {
            QuickFix::TreatAsNormalMap => {
                "Mark texture as normal map (linear colorspace, correct sampling)"
            }
            QuickFix::FlipGreenChannel => {
                "Flip green channel for DirectX ↔ OpenGL normal map conversion"
            }
            QuickFix::ConvertToORM => "Repack channels to Occlusion-Roughness-Metallic format",
            QuickFix::ConvertToMRA => "Repack channels to Metallic-Roughness-AO format",
            QuickFix::GenerateTangents => "Calculate tangent vectors from UV coordinates",
            QuickFix::ResizePowerOfTwo => "Resize texture to nearest power-of-two dimensions",
            QuickFix::GenerateLODs => "Create LOD meshes with simplified geometry",
            QuickFix::GenerateCollider => "Generate convex hull or mesh collider",
            QuickFix::FixScale => "Apply scale correction (e.g., cm → m, inches → m)",
            QuickFix::FixOrientation => "Rotate to correct up-axis (Y-up vs Z-up)",
            QuickFix::MarkAsSRGB => "Set texture color space to sRGB (for albedo/emissive)",
            QuickFix::MarkAsLinear => {
                "Set texture color space to Linear (for normal/roughness/metal)"
            }
        }
    }

    /// Whether this fix is destructive (modifies original file)
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            QuickFix::FlipGreenChannel
                | QuickFix::ConvertToORM
                | QuickFix::ConvertToMRA
                | QuickFix::ResizePowerOfTwo
        )
    }
}

// ============================================================================
// IMPORT SETTINGS - Configuration for import operations
// ============================================================================

/// Import settings for the doctor wizard
#[derive(Debug, Clone, PartialEq)]
pub struct ImportSettings {
    pub auto_detect_source: bool,
    pub source_override: Option<SourceEngine>,
    pub packing_format: TexturePackingFormat,
    pub flip_normal_green: bool,
    pub generate_tangents: bool,
    pub resize_non_pot: bool,
    pub generate_lods: bool,
    pub lod_levels: u32,
    pub generate_colliders: bool,
    pub fix_scale: bool,
    pub scale_factor: f32,
    pub fix_orientation: bool,
    pub target_up_axis: UpAxis,
    pub show_preview: bool,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self {
            auto_detect_source: true,
            source_override: None,
            packing_format: TexturePackingFormat::ORM,
            flip_normal_green: false,
            generate_tangents: true,
            resize_non_pot: false,
            generate_lods: true,
            lod_levels: 3,
            generate_colliders: false,
            fix_scale: true,
            scale_factor: 1.0,
            fix_orientation: true,
            target_up_axis: UpAxis::Y,
            show_preview: true,
        }
    }
}

/// Up-axis orientation
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UpAxis {
    #[default]
    Y,
    Z,
}

impl std::fmt::Display for UpAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl UpAxis {
    pub fn name(&self) -> &'static str {
        match self {
            UpAxis::Y => "Y-Up",
            UpAxis::Z => "Z-Up",
        }
    }
}

// ============================================================================
// IMPORT DOCTOR PANEL
// ============================================================================

/// Import Doctor/Wizard panel for asset validation and fixing
#[derive(Debug)]
pub struct ImportDoctorPanel {
    pub settings: ImportSettings,
    pub detected_source: SourceEngine,
    pub detected_packing: TexturePackingFormat,
    pub issues: Vec<ImportIssue>,
    pub selected_files: Vec<PathBuf>,
    pub preview_ready: bool,
    pub scanning: bool,
    pub scan_progress: f32,
    pub last_scan_time_ms: u64,
    pub quick_fixes_applied: Vec<QuickFix>,
    // Pending actions to be consumed by the editor
    pub pending_actions: Vec<ImportAction>,
}

impl Default for ImportDoctorPanel {
    fn default() -> Self {
        Self {
            settings: ImportSettings::default(),
            detected_source: SourceEngine::Unknown,
            detected_packing: TexturePackingFormat::Separate,
            issues: Vec::new(),
            selected_files: Vec::new(),
            preview_ready: false,
            scanning: false,
            scan_progress: 0.0,
            last_scan_time_ms: 0,
            quick_fixes_applied: Vec::new(),
            pending_actions: Vec::new(),
        }
    }
}

impl ImportDoctorPanel {
    /// Create a new import doctor panel
    pub fn new() -> Self {
        Self::default()
    }

    /// Take all pending actions (clears the queue)
    pub fn take_actions(&mut self) -> Vec<ImportAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queue an import action
    fn queue_action(&mut self, action: ImportAction) {
        self.pending_actions.push(action);
    }

    /// Get the current asset path (first selected file)
    fn current_asset_path(&self) -> Option<PathBuf> {
        self.selected_files.first().cloned()
    }

    /// Count issues by severity
    pub fn issue_count(&self, severity: IssueSeverity) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .count()
    }

    /// Count total fixable issues
    pub fn fixable_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.can_auto_fix && !i.fix_applied)
            .count()
    }

    /// Whether import can proceed
    pub fn can_import(&self) -> bool {
        !self.issues.iter().any(|i| i.severity.blocks_import())
    }

    fn render_detection_panel(&mut self, ui: &mut Ui) {
        ui.heading("Detection");

        ui.horizontal(|ui| {
            ui.label("Detected Source:");
            ui.strong(format!("{}", self.detected_source));
        });

        ui.horizontal(|ui| {
            ui.label("Detected Packing:");
            ui.strong(format!("{}", self.detected_packing));
        });

        ui.separator();

        ui.checkbox(&mut self.settings.auto_detect_source, "Auto-detect source");

        if !self.settings.auto_detect_source {
            egui::ComboBox::from_id_salt("source_override")
                .selected_text(
                    self.settings
                        .source_override
                        .map(|s| format!("{}", s))
                        .unwrap_or_else(|| "Select...".to_string()),
                )
                .show_ui(ui, |ui| {
                    for engine in SourceEngine::all() {
                        if ui
                            .selectable_label(
                                self.settings.source_override == Some(*engine),
                                format!("{}", engine),
                            )
                            .clicked()
                        {
                            self.settings.source_override = Some(*engine);
                        }
                    }
                });
        }
    }

    fn render_issues_panel(&mut self, ui: &mut Ui) {
        ui.heading("Issues");

        // Summary
        ui.horizontal(|ui| {
            let critical = self.issue_count(IssueSeverity::Critical);
            let errors = self.issue_count(IssueSeverity::Error);
            let warnings = self.issue_count(IssueSeverity::Warning);
            let info = self.issue_count(IssueSeverity::Info);

            if critical > 0 {
                ui.colored_label(IssueSeverity::Critical.color(), format!("{} 🚨", critical));
            }
            if errors > 0 {
                ui.colored_label(IssueSeverity::Error.color(), format!("{} [x]", errors));
            }
            if warnings > 0 {
                ui.colored_label(IssueSeverity::Warning.color(), format!("{} [!]", warnings));
            }
            if info > 0 {
                ui.colored_label(IssueSeverity::Info.color(), format!("{} ℹ️", info));
            }

            if self.issues.is_empty() {
                ui.colored_label(Color32::from_rgb(100, 200, 100), "No issues detected");
            }
        });

        ui.separator();

        // Collect indices of issues to mark as fixed
        let mut issues_to_fix: Vec<(usize, QuickFix)> = Vec::new();
        let asset_path = self.current_asset_path();

        // Issue list
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (idx, issue) in self.issues.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.colored_label(issue.severity.color(), issue.severity.icon());
                        ui.label(&issue.message);

                        if issue.can_auto_fix && !issue.fix_applied {
                            if ui.small_button("Fix").clicked() {
                                if let Some(fix) = issue.issue_type.suggested_fix() {
                                    info!(fix = ?fix, issue = %issue.message, "Import Doctor: applying fix");
                                    issues_to_fix.push((idx, fix));
                                }
                            }
                        } else if issue.fix_applied {
                            ui.label("Fixed");
                        }
                    });
                }
            });

        // Apply fixes and queue actions
        for (idx, fix) in issues_to_fix {
            if let Some(issue) = self.issues.get_mut(idx) {
                issue.fix_applied = true;
            }
            self.quick_fixes_applied.push(fix);
            if let Some(path) = &asset_path {
                self.queue_action(ImportAction::ApplyQuickFix {
                    fix,
                    asset_path: path.clone(),
                });
            }
        }
    }

    fn render_quick_fixes(&mut self, ui: &mut Ui) {
        ui.heading("Quick Fixes");

        egui::Grid::new("quick_fixes_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                // Normal map fixes
                if ui
                    .button(format!("{}", QuickFix::TreatAsNormalMap))
                    .clicked()
                {
                    self.quick_fixes_applied.push(QuickFix::TreatAsNormalMap);
                }
                ui.label(QuickFix::TreatAsNormalMap.description());
                ui.end_row();

                if ui
                    .button(format!("{}", QuickFix::FlipGreenChannel))
                    .clicked()
                {
                    self.quick_fixes_applied.push(QuickFix::FlipGreenChannel);
                }
                ui.label(QuickFix::FlipGreenChannel.description());
                ui.end_row();

                // Packing conversions
                if ui.button(format!("{}", QuickFix::ConvertToORM)).clicked() {
                    self.quick_fixes_applied.push(QuickFix::ConvertToORM);
                }
                ui.label(QuickFix::ConvertToORM.description());
                ui.end_row();

                if ui.button(format!("{}", QuickFix::ConvertToMRA)).clicked() {
                    self.quick_fixes_applied.push(QuickFix::ConvertToMRA);
                }
                ui.label(QuickFix::ConvertToMRA.description());
                ui.end_row();

                // Mesh fixes
                if ui
                    .button(format!("{}", QuickFix::GenerateTangents))
                    .clicked()
                {
                    self.quick_fixes_applied.push(QuickFix::GenerateTangents);
                }
                ui.label(QuickFix::GenerateTangents.description());
                ui.end_row();

                if ui.button(format!("{}", QuickFix::GenerateLODs)).clicked() {
                    self.quick_fixes_applied.push(QuickFix::GenerateLODs);
                }
                ui.label(QuickFix::GenerateLODs.description());
                ui.end_row();
            });
    }

    fn render_settings(&mut self, ui: &mut Ui) {
        ui.heading("Import Settings");

        egui::CollapsingHeader::new("Textures")
            .default_open(true)
            .show(ui, |ui| {
                ui.checkbox(
                    &mut self.settings.flip_normal_green,
                    "Flip normal green channel",
                );
                ui.checkbox(&mut self.settings.resize_non_pot, "Resize non-power-of-two");

                ui.horizontal(|ui| {
                    ui.label("Target Packing:");
                    egui::ComboBox::from_id_salt("packing_format")
                        .selected_text(format!("{}", self.settings.packing_format))
                        .show_ui(ui, |ui| {
                            for format in TexturePackingFormat::all() {
                                if ui
                                    .selectable_label(
                                        self.settings.packing_format == *format,
                                        format!("{}", format),
                                    )
                                    .clicked()
                                {
                                    self.settings.packing_format = *format;
                                }
                            }
                        });
                });
            });

        egui::CollapsingHeader::new("Meshes")
            .default_open(true)
            .show(ui, |ui| {
                ui.checkbox(&mut self.settings.generate_tangents, "Generate tangents");
                ui.checkbox(&mut self.settings.generate_lods, "Generate LODs");

                if self.settings.generate_lods {
                    ui.horizontal(|ui| {
                        ui.label("LOD Levels:");
                        ui.add(egui::DragValue::new(&mut self.settings.lod_levels).range(1..=5));
                    });
                }

                ui.checkbox(&mut self.settings.generate_colliders, "Generate colliders");
            });

        egui::CollapsingHeader::new("Transform")
            .default_open(false)
            .show(ui, |ui| {
                ui.checkbox(&mut self.settings.fix_scale, "Apply scale correction");

                if self.settings.fix_scale {
                    ui.horizontal(|ui| {
                        ui.label("Scale Factor:");
                        ui.add(
                            egui::DragValue::new(&mut self.settings.scale_factor)
                                .range(0.001..=1000.0),
                        );
                    });
                }

                ui.checkbox(&mut self.settings.fix_orientation, "Fix orientation");

                if self.settings.fix_orientation {
                    ui.horizontal(|ui| {
                        ui.label("Target Up:");
                        ui.selectable_value(&mut self.settings.target_up_axis, UpAxis::Y, "Y-Up");
                        ui.selectable_value(&mut self.settings.target_up_axis, UpAxis::Z, "Z-Up");
                    });
                }
            });

        ui.checkbox(
            &mut self.settings.show_preview,
            "Show preview with lighting",
        );
    }

    fn render_preview(&mut self, ui: &mut Ui) {
        ui.heading("Preview");

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(200.0, 150.0), egui::Sense::click());

        if self.preview_ready {
            // Preview is active - show placeholder with lighting
            ui.painter()
                .rect_filled(rect, 8.0, Color32::from_rgb(40, 45, 50));

            // Draw a simple mesh icon to indicate 3D preview
            let center = rect.center();
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "[Mask]",
                egui::FontId::proportional(48.0),
                Color32::from_rgb(150, 150, 160),
            );
            ui.painter().text(
                center + egui::vec2(0.0, 35.0),
                egui::Align2::CENTER_CENTER,
                "Preview Active",
                egui::FontId::default(),
                Color32::GRAY,
            );
        } else {
            ui.painter()
                .rect_filled(rect, 8.0, Color32::from_rgb(30, 30, 35));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Click to preview asset",
                egui::FontId::default(),
                Color32::DARK_GRAY,
            );

            // Handle click to request preview
            if response.clicked() {
                if let Some(path) = self.current_asset_path() {
                    self.queue_action(ImportAction::PreviewAsset { asset_path: path });
                    self.preview_ready = true;
                }
            }
        }
    }

    fn render_action_buttons(&mut self, ui: &mut Ui) {
        ui.separator();

        let can_import = self.can_import();
        let asset_path = self.current_asset_path();
        let mut should_import = false;
        let mut should_fix_all = false;
        let mut should_clear = false;

        ui.horizontal(|ui| {
            if can_import {
                if ui.button(RichText::new("Import").strong()).clicked() {
                    should_import = true;
                }
            } else {
                ui.add_enabled(
                    false,
                    egui::Button::new(RichText::new("Cannot Import").color(Color32::RED)),
                );
            }

            if ui.button("Fix All").clicked() {
                should_fix_all = true;
            }

            if ui.button("Clear").clicked() {
                should_clear = true;
            }
        });

        // Process actions after UI rendering to avoid borrow issues
        if should_import {
            if let Some(path) = &asset_path {
                info!(path = %path.display(), "Import Doctor: importing asset");
                self.queue_action(ImportAction::ImportAsset {
                    asset_path: path.clone(),
                    settings: self.settings.clone(),
                    applied_fixes: self.quick_fixes_applied.clone(),
                });
            }
        }

        if should_fix_all {
            info!(
                fixable = self.fixable_count(),
                "Import Doctor: applying all fixes"
            );
            // Mark all fixable issues as fixed
            for issue in &mut self.issues {
                if issue.can_auto_fix && !issue.fix_applied {
                    issue.fix_applied = true;
                    if let Some(fix) = issue.issue_type.suggested_fix() {
                        self.quick_fixes_applied.push(fix);
                    }
                }
            }
            if let Some(path) = &asset_path {
                self.queue_action(ImportAction::ApplyAllFixes {
                    asset_path: path.clone(),
                });
            }
        }

        if should_clear {
            self.selected_files.clear();
            self.issues.clear();
            self.quick_fixes_applied.clear();
            self.preview_ready = false;
            self.detected_source = SourceEngine::Unknown;
            self.detected_packing = TexturePackingFormat::Separate;
            self.queue_action(ImportAction::ClearSelection);
        }

        // Status
        if !can_import {
            ui.colored_label(
                Color32::from_rgb(255, 100, 100),
                "Critical issues must be resolved before import",
            );
        } else if !self.issues.is_empty() {
            let fixable = self.fixable_count();
            if fixable > 0 {
                ui.colored_label(
                    Color32::from_rgb(255, 200, 100),
                    format!("{} fixable issues remaining", fixable),
                );
            }
        } else {
            ui.colored_label(Color32::from_rgb(100, 200, 100), "Asset ready for import");
        }
    }
}

impl Panel for ImportDoctorPanel {
    fn name(&self) -> &'static str {
        "Import Doctor"
    }

    fn show(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.render_detection_panel(ui);
            ui.separator();

            self.render_issues_panel(ui);
            ui.separator();

            self.render_quick_fixes(ui);
            ui.separator();

            self.render_settings(ui);
            ui.separator();

            if self.settings.show_preview {
                self.render_preview(ui);
                ui.separator();
            }

            self.render_action_buttons(ui);
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
    fn test_source_engine_display() {
        assert!(format!("{}", SourceEngine::Unity).contains("Unity"));
        assert!(format!("{}", SourceEngine::Blender).contains("Blender"));
    }

    #[test]
    fn test_source_engine_all() {
        let engines = SourceEngine::all();
        assert_eq!(engines.len(), 14);
    }

    #[test]
    fn test_source_engine_normals() {
        assert!(SourceEngine::Unity.uses_directx_normals());
        assert!(SourceEngine::Unreal.uses_directx_normals());
        assert!(SourceEngine::Blender.uses_opengl_normals());
        assert!(SourceEngine::SubstancePainter.uses_opengl_normals());
    }

    #[test]
    fn test_source_engine_from_filename() {
        assert_eq!(
            SourceEngine::from_filename("rock_unreal_export.fbx"),
            SourceEngine::Unreal
        );
        assert_eq!(
            SourceEngine::from_filename("character_blender.glb"),
            SourceEngine::Blender
        );
        assert_eq!(
            SourceEngine::from_filename("texture_sp_export.png"),
            SourceEngine::SubstancePainter
        );
        assert_eq!(
            SourceEngine::from_filename("random_asset.fbx"),
            SourceEngine::Unknown
        );
    }

    #[test]
    fn test_packing_format_display() {
        assert_eq!(TexturePackingFormat::ORM.name(), "ORM");
        assert_eq!(TexturePackingFormat::MRA.name(), "MRA");
    }

    #[test]
    fn test_packing_format_all() {
        let formats = TexturePackingFormat::all();
        assert_eq!(formats.len(), 6);
    }

    #[test]
    fn test_packing_format_channels() {
        assert_eq!(TexturePackingFormat::ORM.ao_channel(), Some('R'));
        assert_eq!(TexturePackingFormat::ORM.roughness_channel(), Some('G'));
        assert_eq!(TexturePackingFormat::ORM.metallic_channel(), Some('B'));

        assert_eq!(TexturePackingFormat::MRA.metallic_channel(), Some('R'));
        assert_eq!(TexturePackingFormat::MRA.roughness_channel(), Some('G'));
        assert_eq!(TexturePackingFormat::MRA.ao_channel(), Some('B'));
    }

    #[test]
    fn test_packing_from_filename() {
        assert_eq!(
            TexturePackingFormat::from_filename("rock_orm.png"),
            TexturePackingFormat::ORM
        );
        assert_eq!(
            TexturePackingFormat::from_filename("rock_mra.png"),
            TexturePackingFormat::MRA
        );
        assert_eq!(
            TexturePackingFormat::from_filename("rock_diffuse.png"),
            TexturePackingFormat::Separate
        );
    }

    #[test]
    fn test_issue_severity_display() {
        assert!(format!("{}", IssueSeverity::Critical).contains("Critical"));
        assert!(format!("{}", IssueSeverity::Warning).contains("Warning"));
    }

    #[test]
    fn test_issue_severity_ordering() {
        assert!(IssueSeverity::Critical > IssueSeverity::Error);
        assert!(IssueSeverity::Error > IssueSeverity::Warning);
        assert!(IssueSeverity::Warning > IssueSeverity::Info);
    }

    #[test]
    fn test_issue_severity_blocks_import() {
        assert!(IssueSeverity::Critical.blocks_import());
        assert!(!IssueSeverity::Error.blocks_import());
        assert!(!IssueSeverity::Warning.blocks_import());
    }

    #[test]
    fn test_issue_type_display() {
        assert!(format!("{}", IssueType::NormalMapFormat).contains("Normal Map"));
        assert!(format!("{}", IssueType::MissingTangents).contains("Tangents"));
    }

    #[test]
    fn test_issue_type_auto_fix() {
        assert!(IssueType::NormalMapFormat.has_auto_fix());
        assert!(IssueType::MissingTangents.has_auto_fix());
        assert!(!IssueType::UnsupportedFormat.has_auto_fix());
    }

    #[test]
    fn test_quick_fix_display() {
        assert!(format!("{}", QuickFix::FlipGreenChannel).contains("Flip Green"));
        assert!(format!("{}", QuickFix::GenerateTangents).contains("Tangents"));
    }

    #[test]
    fn test_quick_fix_destructive() {
        assert!(QuickFix::FlipGreenChannel.is_destructive());
        assert!(QuickFix::ConvertToORM.is_destructive());
        assert!(!QuickFix::TreatAsNormalMap.is_destructive());
        assert!(!QuickFix::GenerateTangents.is_destructive());
    }

    #[test]
    fn test_import_issue_creation() {
        let issue = ImportIssue::new(IssueType::MissingTangents, "Mesh has no tangent data");
        assert_eq!(issue.severity, IssueSeverity::Warning);
        assert!(issue.can_auto_fix);
        assert!(!issue.fix_applied);
    }

    #[test]
    fn test_panel_default() {
        let panel = ImportDoctorPanel::new();
        assert!(panel.settings.auto_detect_source);
        assert!(panel.can_import());
        assert!(panel.issues.is_empty());
    }

    #[test]
    fn test_panel_issue_count() {
        let mut panel = ImportDoctorPanel::new();
        panel
            .issues
            .push(ImportIssue::new(IssueType::MissingTangents, ""));
        panel
            .issues
            .push(ImportIssue::new(IssueType::NonPowerOfTwo, ""));
        panel
            .issues
            .push(ImportIssue::new(IssueType::MissingTexture, ""));

        assert_eq!(panel.issue_count(IssueSeverity::Warning), 2);
        assert_eq!(panel.issue_count(IssueSeverity::Error), 1);
    }

    #[test]
    fn test_panel_can_import_with_critical() {
        let mut panel = ImportDoctorPanel::new();
        panel.issues.push(
            ImportIssue::new(IssueType::UnsupportedFormat, "Cannot read file")
                .with_severity(IssueSeverity::Critical),
        );

        assert!(!panel.can_import());
    }

    // ==========================================================================
    // Action System Tests
    // ==========================================================================

    #[test]
    fn test_action_queue_initially_empty() {
        let panel = ImportDoctorPanel::new();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_take_actions_drains_queue() {
        let mut panel = ImportDoctorPanel::new();
        panel.pending_actions.push(ImportAction::ClearSelection);
        panel.pending_actions.push(ImportAction::ScanAsset {
            asset_path: PathBuf::from("test.fbx"),
        });

        assert!(panel.has_pending_actions());
        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_import_action_variants() {
        use std::path::PathBuf;

        let fix_action = ImportAction::ApplyQuickFix {
            fix: QuickFix::FlipGreenChannel,
            asset_path: PathBuf::from("test.fbx"),
        };
        assert!(matches!(fix_action, ImportAction::ApplyQuickFix { .. }));

        let import_action = ImportAction::ImportAsset {
            asset_path: PathBuf::from("test.fbx"),
            settings: ImportSettings::default(),
            applied_fixes: vec![QuickFix::GenerateTangents],
        };
        assert!(matches!(import_action, ImportAction::ImportAsset { .. }));
    }

    #[test]
    fn test_issue_type_suggested_fix_mapping() {
        assert_eq!(
            IssueType::NormalMapFormat.suggested_fix(),
            Some(QuickFix::FlipGreenChannel)
        );
        assert_eq!(
            IssueType::MissingTangents.suggested_fix(),
            Some(QuickFix::GenerateTangents)
        );
        assert_eq!(
            IssueType::NonPowerOfTwo.suggested_fix(),
            Some(QuickFix::ResizePowerOfTwo)
        );
        assert_eq!(
            IssueType::IncorrectScale.suggested_fix(),
            Some(QuickFix::FixScale)
        );
        assert_eq!(
            IssueType::IncorrectOrientation.suggested_fix(),
            Some(QuickFix::FixOrientation)
        );
        // Issues without auto-fix should return None
        assert_eq!(IssueType::UnsupportedFormat.suggested_fix(), None);
        assert_eq!(IssueType::MissingTexture.suggested_fix(), None);
    }

    #[test]
    fn test_current_asset_path() {
        let mut panel = ImportDoctorPanel::new();
        assert!(panel.current_asset_path().is_none());

        panel.selected_files.push(PathBuf::from("assets/model.fbx"));
        assert_eq!(
            panel.current_asset_path(),
            Some(PathBuf::from("assets/model.fbx"))
        );
    }
}
