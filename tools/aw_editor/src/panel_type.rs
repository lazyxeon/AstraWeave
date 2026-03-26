//! Panel Type Definitions for Dockable Editor Panels
//!
//! This module defines the `PanelType` enum that represents all available
//! editor panels. Each variant corresponds to a panel that can be docked,
//! undocked, tabbed, or hidden within the editor layout.
//!
//! # Architecture
//!
//! The panel system integrates with `egui_dock` to provide:
//! - Drag-and-drop panel repositioning
//! - Tab grouping (multiple panels in same area)
//! - Split layouts (horizontal/vertical)
//! - Layout persistence across sessions
//!
//! # Example
//!
//! ```
//! use aw_editor_lib::panel_type::PanelType;
//!
//! // Get display title for UI
//! let panel = PanelType::Hierarchy;
//! assert_eq!(panel.title(), "Hierarchy");
//!
//! // Check if panel is closable
//! assert!(panel.is_closable());
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Category for grouping related panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PanelCategory {
    /// Scene editing panels (Hierarchy, Inspector, Viewport)
    Scene,
    /// Asset management panels
    Assets,
    /// Debug and profiling panels
    Debug,
    /// Tool panels for specific workflows
    Tools,
    /// System and settings panels
    System,
    /// Content creation panels
    Content,
}

impl PanelCategory {
    /// Get all panel categories
    pub fn all() -> &'static [PanelCategory] {
        &[
            PanelCategory::Scene,
            PanelCategory::Assets,
            PanelCategory::Debug,
            PanelCategory::Tools,
            PanelCategory::System,
            PanelCategory::Content,
        ]
    }

    /// Get display name for this category
    pub fn name(&self) -> &'static str {
        match self {
            PanelCategory::Scene => "Scene",
            PanelCategory::Assets => "Assets",
            PanelCategory::Debug => "Debug",
            PanelCategory::Tools => "Tools",
            PanelCategory::System => "System",
            PanelCategory::Content => "Content Creation",
        }
    }

    /// Get icon for this category
    pub fn icon(&self) -> &'static str {
        match self {
            PanelCategory::Scene => "[Scn]",
            PanelCategory::Assets => "[Ast]",
            PanelCategory::Debug => "[Dbg]",
            PanelCategory::Tools => "[Tool]",
            PanelCategory::System => "[Sys]",
            PanelCategory::Content => "[Cnt]",
        }
    }
}

impl fmt::Display for PanelCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

/// Enumeration of all available editor panels
///
/// Each panel type represents a distinct functional area of the editor.
/// Panels can be docked in various configurations using `egui_dock`.
///
/// # Panel Categories
///
/// - **Scene Panels**: Hierarchy, Inspector, Viewport, World
/// - **Asset Panels**: AssetBrowser
/// - **Debug Panels**: Console, Profiler, Performance, SceneStats
/// - **Tool Panels**: Transform, Animation, Graph, Charts, AdvancedWidgets
/// - **System Panels**: BuildManager, ThemeManager
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PanelType {
    /// Scene hierarchy tree showing entity parent-child relationships
    Hierarchy,

    /// Property inspector for selected entities/assets
    Inspector,

    /// Asset browser for browsing and managing project assets
    AssetBrowser,

    /// 3D viewport for scene editing and visualization
    Viewport,

    /// Console output for logs, warnings, and errors
    Console,

    /// Performance profiler showing frame timing and system metrics
    Profiler,

    /// Build manager for project compilation and export
    BuildManager,

    /// Scene statistics (entity count, memory usage, etc.)
    SceneStats,

    /// Transform editing panel (position, rotation, scale)
    Transform,

    /// Performance monitoring graphs
    Performance,

    /// Data visualization charts
    Charts,

    /// Advanced widgets panel (color pickers, curves, etc.)
    AdvancedWidgets,

    /// Node graph editor for behavior/shader graphs
    Graph,

    /// Animation timeline and keyframe editor
    Animation,

    /// Theme manager for editor appearance customization
    ThemeManager,

    /// World settings and environment configuration
    World,

    /// Material editor with PBR preview
    MaterialEditor,

    /// Entity properties panel (alternative inspector view)
    EntityPanel,

    /// Behavior graph editor for AI logic
    BehaviorGraph,

    /// Terrain generation and editing panel
    Terrain,

    // === New SOTA Panels (Phase 8) ===
    /// UI Editor for runtime UI building
    UiEditor,

    /// Foliage painting and vegetation placement
    Foliage,

    /// Spline/path editor for roads, rails, rivers
    SplineEditor,

    /// LOD configuration and distance settings
    LodConfig,

    /// Localization and translation management
    Localization,

    /// Project-wide settings configuration
    ProjectSettings,

    /// Audio mixer and sound management
    Audio,

    /// Cinematics timeline and sequencer
    Cinematics,

    /// Dialogue editor for NPC conversations
    DialogueEditor,

    /// Lighting configuration and light placement
    Lighting,

    /// Navigation mesh and pathfinding
    Navigation,

    /// Network and multiplayer configuration
    Networking,

    /// Particle system editor
    ParticleSystem,

    /// Procedural content generation
    Pcg,

    /// Physics settings and simulation
    Physics,

    /// Post-processing effects stack
    PostProcess,

    /// Input bindings configuration
    InputBindings,

    /// GPU frame debugger — per-pass timing and draw call breakdown
    FrameDebugger,

    /// Blender .blend scene import, decomposition, and biome pack generation
    BlendImport,

    /// 2D top-down blueprint zone editor for terrain generation zones
    Blueprint,
}

impl PanelType {
    /// Returns the display title for this panel
    ///
    /// This title is shown in tab headers and window titles.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Hierarchy => "Hierarchy",
            Self::Inspector => "Inspector",
            Self::AssetBrowser => "Asset Browser",
            Self::Viewport => "Viewport",
            Self::Console => "Console",
            Self::Profiler => "Profiler",
            Self::BuildManager => "Build Manager",
            Self::SceneStats => "Scene Stats",
            Self::Transform => "Transform",
            Self::Performance => "Performance",
            Self::Charts => "Charts",
            Self::AdvancedWidgets => "Advanced Widgets",
            Self::Graph => "Graph",
            Self::Animation => "Animation",
            Self::ThemeManager => "Theme Manager",
            Self::World => "World",
            Self::MaterialEditor => "Material Editor",
            Self::EntityPanel => "Entity",
            Self::BehaviorGraph => "Behavior Graph",
            Self::Terrain => "Terrain",
            // New SOTA panels
            Self::UiEditor => "UI Editor",
            Self::Foliage => "Foliage",
            Self::SplineEditor => "Spline Editor",
            Self::LodConfig => "LOD Config",
            Self::Localization => "Localization",
            Self::ProjectSettings => "Project Settings",
            Self::Audio => "Audio",
            Self::Cinematics => "Cinematics",
            Self::DialogueEditor => "Dialogue Editor",
            Self::Lighting => "Lighting",
            Self::Navigation => "Navigation",
            Self::Networking => "Networking",
            Self::ParticleSystem => "Particle System",
            Self::Pcg => "PCG",
            Self::Physics => "Physics",
            Self::PostProcess => "Post Process",
            Self::InputBindings => "Input Bindings",
            Self::FrameDebugger => "Frame Debugger",
            Self::BlendImport => "Blend Import",
            Self::Blueprint => "Blueprint",
        }
    }

    /// Returns a short icon representation for this panel
    ///
    /// Icons are used in compact tab views and toolbar buttons.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Hierarchy => "[H]",
            Self::Inspector => "[I]",
            Self::AssetBrowser => "[A]",
            Self::Viewport => "[V]",
            Self::Console => "[C]",
            Self::Profiler => "[P]",
            Self::BuildManager => "[B]",
            Self::SceneStats => "[SS]",
            Self::Transform => "[T]",
            Self::Performance => "[Pf]",
            Self::Charts => "[Ch]",
            Self::AdvancedWidgets => "[AW]",
            Self::Graph => "[G]",
            Self::Animation => "[An]",
            Self::ThemeManager => "[Th]",
            Self::World => "[W]",
            Self::MaterialEditor => "[M]",
            Self::EntityPanel => "[E]",
            Self::BehaviorGraph => "[BG]",
            Self::Terrain => "[Tr]",
            // New SOTA panels
            Self::UiEditor => "[UI]",
            Self::Foliage => "[F]",
            Self::SplineEditor => "[Sp]",
            Self::LodConfig => "[LD]",
            Self::Localization => "[L]",
            Self::ProjectSettings => "[PS]",
            Self::Audio => "[Au]",
            Self::Cinematics => "[Ci]",
            Self::DialogueEditor => "[D]",
            Self::Lighting => "[Lt]",
            Self::Navigation => "[N]",
            Self::Networking => "[Nw]",
            Self::ParticleSystem => "[Pt]",
            Self::Pcg => "[PC]",
            Self::Physics => "[Ph]",
            Self::PostProcess => "[PP]",
            Self::InputBindings => "[In]",
            Self::FrameDebugger => "[FD]",
            Self::BlendImport => "[BI]",
            Self::Blueprint => "[BP]",
        }
    }

    /// Returns whether this panel can be closed by the user
    ///
    /// Some panels (like Viewport) may be marked as not closable
    /// to ensure the editor always has a main work area.
    pub fn is_closable(&self) -> bool {
        match self {
            // Viewport should always be visible
            Self::Viewport => false,
            // All other panels can be closed
            _ => true,
        }
    }

    /// Returns whether this panel should have scroll bars
    pub fn has_scroll(&self) -> bool {
        match self {
            // Viewport handles its own scrolling
            Self::Viewport => false,
            // Graph/canvas panels have their own pan/zoom
            Self::Graph | Self::BehaviorGraph | Self::Blueprint => false,
            // All other panels should scroll
            _ => true,
        }
    }

    /// Returns the category this panel belongs to
    pub fn category(&self) -> PanelCategory {
        match self {
            // Scene panels
            Self::Hierarchy
            | Self::Inspector
            | Self::Viewport
            | Self::World
            | Self::EntityPanel
            | Self::Transform => PanelCategory::Scene,

            // Asset panels
            Self::AssetBrowser => PanelCategory::Assets,

            // Debug panels
            Self::Console
            | Self::Profiler
            | Self::Performance
            | Self::SceneStats
            | Self::FrameDebugger => PanelCategory::Debug,

            // Tool panels
            Self::Charts
            | Self::AdvancedWidgets
            | Self::Graph
            | Self::Animation
            | Self::BehaviorGraph
            | Self::SplineEditor
            | Self::Navigation
            | Self::Blueprint => PanelCategory::Tools,

            // System panels
            Self::BuildManager
            | Self::ThemeManager
            | Self::ProjectSettings
            | Self::Localization
            | Self::Networking
            | Self::InputBindings => PanelCategory::System,

            // Content creation panels
            Self::MaterialEditor
            | Self::Terrain
            | Self::UiEditor
            | Self::Foliage
            | Self::LodConfig
            | Self::Audio
            | Self::Cinematics
            | Self::DialogueEditor
            | Self::Lighting
            | Self::ParticleSystem
            | Self::Pcg
            | Self::Physics
            | Self::PostProcess
            | Self::BlendImport => PanelCategory::Content,
        }
    }

    /// Returns a description of what this panel does
    pub fn description(&self) -> &'static str {
        match self {
            Self::Hierarchy => "View and organize scene entities in a tree structure",
            Self::Inspector => "Edit properties of selected entities and assets",
            Self::AssetBrowser => "Browse, import, and manage project assets",
            Self::Viewport => "3D scene view for visual editing and placement",
            Self::Console => "View logs, warnings, and error messages",
            Self::Profiler => "Analyze frame timing and performance metrics",
            Self::BuildManager => "Configure and execute project builds",
            Self::SceneStats => "View scene statistics and memory usage",
            Self::Transform => "Edit position, rotation, and scale",
            Self::Performance => "Monitor real-time performance graphs",
            Self::Charts => "Data visualization and charting tools",
            Self::AdvancedWidgets => "Color pickers, curves, and specialized controls",
            Self::Graph => "Node-based graph editor for shaders and logic",
            Self::Animation => "Timeline and keyframe animation editor",
            Self::ThemeManager => "Customize editor appearance and layout",
            Self::World => "Configure world settings and environment",
            Self::MaterialEditor => "Create and edit PBR materials",
            Self::EntityPanel => "Entity component inspector and editor",
            Self::BehaviorGraph => "Visual AI behavior tree editor",
            Self::Terrain => "Terrain sculpting and painting tools",
            Self::UiEditor => "Design runtime UI layouts and widgets",
            Self::Foliage => "Paint and manage vegetation placement",
            Self::SplineEditor => "Create paths, roads, and spline meshes",
            Self::LodConfig => "Configure level-of-detail settings",
            Self::Localization => "Manage translations and localized text",
            Self::ProjectSettings => "Configure project-wide settings",
            Self::Audio => "Audio mixing and sound management",
            Self::Cinematics => "Sequence editor for cutscenes",
            Self::DialogueEditor => "Create NPC dialogue trees",
            Self::Lighting => "Configure lights and global illumination",
            Self::Navigation => "Edit navmesh and pathfinding settings",
            Self::Networking => "Multiplayer and network configuration",
            Self::ParticleSystem => "Create and edit particle effects",
            Self::Pcg => "Procedural content generation tools",
            Self::Physics => "Physics simulation settings",
            Self::PostProcess => "Post-processing effects configuration",
            Self::InputBindings => "Configure keyboard and controller bindings",
            Self::FrameDebugger => "GPU render pass timing and draw call breakdown",
            Self::BlendImport => "Import Blender .blend scenes, decompose into assets, and generate biome packs",
            Self::Blueprint => "2D top-down zone editor for defining terrain generation zones",
        }
    }

    /// Returns keyboard shortcut hint for opening this panel (if any)
    pub fn shortcut_hint(&self) -> Option<&'static str> {
        match self {
            Self::Hierarchy => Some("Ctrl+1"),
            Self::Inspector => Some("Ctrl+2"),
            Self::AssetBrowser => Some("Ctrl+3"),
            Self::Console => Some("Ctrl+`"),
            Self::Profiler => Some("Ctrl+P"),
            Self::Animation => Some("Ctrl+Shift+A"),
            Self::Graph => Some("Ctrl+G"),
            Self::ProjectSettings => Some("Ctrl+Shift+P"),
            _ => None,
        }
    }

    /// Check if this is a debug/development panel
    pub fn is_debug_panel(&self) -> bool {
        self.category() == PanelCategory::Debug
    }

    /// Check if this is a content creation panel
    pub fn is_content_panel(&self) -> bool {
        self.category() == PanelCategory::Content
    }

    /// Get all panels in a specific category
    pub fn in_category(category: PanelCategory) -> Vec<PanelType> {
        Self::all()
            .iter()
            .copied()
            .filter(|p| p.category() == category)
            .collect()
    }

    /// Returns a list of all available panel types
    pub fn all() -> &'static [PanelType] {
        &[
            Self::Hierarchy,
            Self::Inspector,
            Self::AssetBrowser,
            Self::Viewport,
            Self::Console,
            Self::Profiler,
            Self::BuildManager,
            Self::SceneStats,
            Self::Transform,
            Self::Performance,
            Self::Charts,
            Self::AdvancedWidgets,
            Self::Graph,
            Self::Animation,
            Self::ThemeManager,
            Self::World,
            Self::MaterialEditor,
            Self::EntityPanel,
            Self::BehaviorGraph,
            Self::Terrain,
            // New SOTA panels
            Self::UiEditor,
            Self::Foliage,
            Self::SplineEditor,
            Self::LodConfig,
            Self::Localization,
            Self::ProjectSettings,
            Self::Audio,
            Self::Cinematics,
            Self::DialogueEditor,
            Self::Lighting,
            Self::Navigation,
            Self::Networking,
            Self::ParticleSystem,
            Self::Pcg,
            Self::Physics,
            Self::PostProcess,
            Self::InputBindings,
            Self::FrameDebugger,
            Self::BlendImport,
            Self::Blueprint,
        ]
    }

    /// Returns the default panels for the left dock area
    pub fn default_left_panels() -> Vec<PanelType> {
        vec![Self::Hierarchy, Self::AssetBrowser]
    }

    /// Returns the default panels for the right dock area
    pub fn default_right_panels() -> Vec<PanelType> {
        vec![Self::Inspector, Self::Transform]
    }

    /// Returns the default panels for the bottom dock area
    pub fn default_bottom_panels() -> Vec<PanelType> {
        vec![Self::Console, Self::Profiler, Self::SceneStats]
    }

    /// Returns the default panel for the center area
    pub fn default_center_panel() -> PanelType {
        Self::Viewport
    }
}

impl fmt::Display for PanelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.icon(), self.title())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_type_title() {
        assert_eq!(PanelType::Hierarchy.title(), "Hierarchy");
        assert_eq!(PanelType::Viewport.title(), "Viewport");
        assert_eq!(PanelType::AssetBrowser.title(), "Asset Browser");
    }

    #[test]
    fn test_panel_type_icon() {
        assert_eq!(PanelType::Hierarchy.icon(), "[H]");
        assert_eq!(PanelType::Viewport.icon(), "[V]");
        assert_eq!(PanelType::Console.icon(), "[C]");
    }

    #[test]
    fn test_panel_type_closable() {
        assert!(!PanelType::Viewport.is_closable());
        assert!(PanelType::Console.is_closable());
        assert!(PanelType::Inspector.is_closable());
    }

    #[test]
    fn test_panel_type_all() {
        let all = PanelType::all();
        assert!(all.len() >= 16, "Should have at least 16 panel types");
        assert!(all.contains(&PanelType::Viewport));
        assert!(all.contains(&PanelType::Hierarchy));
    }

    #[test]
    fn test_panel_type_display() {
        let panel = PanelType::Hierarchy;
        let display = format!("{}", panel);
        assert!(display.contains("Hierarchy"));
        assert!(display.contains("[H]"));
    }

    #[test]
    fn test_panel_type_serialization() {
        let panel = PanelType::Inspector;
        let json = serde_json::to_string(&panel).unwrap();
        let deserialized: PanelType = serde_json::from_str(&json).unwrap();
        assert_eq!(panel, deserialized);
    }

    #[test]
    fn test_default_layout() {
        let left = PanelType::default_left_panels();
        let right = PanelType::default_right_panels();
        let bottom = PanelType::default_bottom_panels();
        let center = PanelType::default_center_panel();

        assert!(left.contains(&PanelType::Hierarchy));
        assert!(right.contains(&PanelType::Inspector));
        assert!(bottom.contains(&PanelType::Console));
        assert_eq!(center, PanelType::Viewport);
    }

    // ====================================================================
    // PanelCategory Tests
    // ====================================================================

    #[test]
    fn test_panel_category_all() {
        let categories = PanelCategory::all();
        assert!(categories.len() >= 5);
    }

    #[test]
    fn test_panel_category_name_not_empty() {
        for cat in PanelCategory::all() {
            assert!(!cat.name().is_empty());
        }
    }

    #[test]
    fn test_panel_category_icon_not_empty() {
        for cat in PanelCategory::all() {
            assert!(!cat.icon().is_empty());
        }
    }

    #[test]
    fn test_panel_category_display() {
        let display = format!("{}", PanelCategory::Scene);
        assert!(display.contains("Scene"));
    }

    // ====================================================================
    // PanelType Category Tests
    // ====================================================================

    #[test]
    fn test_panel_type_category_scene() {
        assert_eq!(PanelType::Hierarchy.category(), PanelCategory::Scene);
        assert_eq!(PanelType::Viewport.category(), PanelCategory::Scene);
        assert_eq!(PanelType::Inspector.category(), PanelCategory::Scene);
    }

    #[test]
    fn test_panel_type_category_debug() {
        assert_eq!(PanelType::Console.category(), PanelCategory::Debug);
        assert_eq!(PanelType::Profiler.category(), PanelCategory::Debug);
        assert_eq!(PanelType::Performance.category(), PanelCategory::Debug);
    }

    #[test]
    fn test_panel_type_category_content() {
        assert_eq!(PanelType::MaterialEditor.category(), PanelCategory::Content);
        assert_eq!(PanelType::Terrain.category(), PanelCategory::Content);
    }

    #[test]
    fn test_panel_type_description_not_empty() {
        for panel in PanelType::all() {
            assert!(
                !panel.description().is_empty(),
                "{:?} has empty description",
                panel
            );
        }
    }

    #[test]
    fn test_panel_type_shortcut_hint() {
        assert!(PanelType::Hierarchy.shortcut_hint().is_some());
        assert!(PanelType::Console.shortcut_hint().is_some());
        // Most panels don't have shortcuts
        assert!(PanelType::Terrain.shortcut_hint().is_none());
    }

    #[test]
    fn test_panel_type_is_debug_panel() {
        assert!(PanelType::Console.is_debug_panel());
        assert!(PanelType::Profiler.is_debug_panel());
        assert!(!PanelType::Hierarchy.is_debug_panel());
        assert!(!PanelType::MaterialEditor.is_debug_panel());
    }

    #[test]
    fn test_panel_type_is_content_panel() {
        assert!(PanelType::MaterialEditor.is_content_panel());
        assert!(PanelType::Terrain.is_content_panel());
        assert!(PanelType::ParticleSystem.is_content_panel());
        assert!(!PanelType::Console.is_content_panel());
    }

    #[test]
    fn test_panel_type_in_category() {
        let debug_panels = PanelType::in_category(PanelCategory::Debug);
        assert!(debug_panels.contains(&PanelType::Console));
        assert!(debug_panels.contains(&PanelType::Profiler));
        assert!(!debug_panels.contains(&PanelType::Hierarchy));
    }

    #[test]
    fn test_all_panels_have_category() {
        for panel in PanelType::all() {
            // Just verify category() doesn't panic
            let _cat = panel.category();
        }
    }
}
