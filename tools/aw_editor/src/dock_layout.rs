//! Docking Layout Manager
//!
//! Provides professional dockable panel layout using `egui_dock`.
//! Supports drag-and-drop panel repositioning, tab grouping,
//! split layouts, and layout persistence.
//!
//! # Architecture
//!
//! The layout system consists of:
//! - `DockLayout`: Main layout manager holding `egui_dock::DockState`
//! - `EditorTabViewer`: Implements `egui_dock::TabViewer` for panel rendering
//! - `LayoutPreset`: Predefined layouts (Default, Wide, Compact, etc.)
//!
//! # Example
//!
//! ```ignore
//! use aw_editor::dock_layout::{DockLayout, LayoutPreset};
//!
//! // Create default layout
//! let mut layout = DockLayout::new();
//!
//! // Or load from preset
//! let layout = DockLayout::from_preset(LayoutPreset::Wide);
//!
//! // Show in egui
//! layout.show(ctx, &mut app_state);
//! ```

use crate::panel_type::PanelType;
use egui_dock::egui;
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabIndex, TabViewer};
use serde::{Deserialize, Serialize};

/// Predefined layout configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LayoutPreset {
    /// Default layout: Left (Hierarchy/Assets), Center (Viewport), Right (Inspector), Bottom (Console)
    Default,
    /// Wide layout: Maximized viewport with minimal side panels
    Wide,
    /// Compact layout: All panels visible in smaller configuration
    Compact,
    /// Modeling layout: Large viewport with transform tools
    Modeling,
    /// Animation layout: Timeline at bottom, graph on side
    Animation,
    /// Debug layout: Console and profiler prominent
    Debug,
}

impl Default for LayoutPreset {
    fn default() -> Self {
        Self::Default
    }
}

impl LayoutPreset {
    /// Get all available layout presets
    pub fn all() -> &'static [LayoutPreset] {
        &[
            LayoutPreset::Default,
            LayoutPreset::Wide,
            LayoutPreset::Compact,
            LayoutPreset::Modeling,
            LayoutPreset::Animation,
            LayoutPreset::Debug,
        ]
    }

    /// Get human-readable name for this preset
    pub fn name(&self) -> &'static str {
        match self {
            LayoutPreset::Default => "Default",
            LayoutPreset::Wide => "Wide",
            LayoutPreset::Compact => "Compact",
            LayoutPreset::Modeling => "Modeling",
            LayoutPreset::Animation => "Animation",
            LayoutPreset::Debug => "Debug",
        }
    }

    /// Get description of this layout preset
    pub fn description(&self) -> &'static str {
        match self {
            LayoutPreset::Default => "Balanced layout for general editing",
            LayoutPreset::Wide => "Maximized viewport for scene viewing",
            LayoutPreset::Compact => "All panels visible in smaller configuration",
            LayoutPreset::Modeling => "Large viewport with transform tools",
            LayoutPreset::Animation => "Timeline at bottom, graph on side",
            LayoutPreset::Debug => "Console and profiler prominent",
        }
    }

    /// Get icon for this preset
    pub fn icon(&self) -> &'static str {
        match self {
            LayoutPreset::Default => "\u{1f3e0}",
            LayoutPreset::Wide => "\u{1f5a5}\u{fe0f}",
            LayoutPreset::Compact => "\u{1f4d0}",
            LayoutPreset::Modeling => "\u{1f527}",
            LayoutPreset::Animation => "\u{1f3ac}",
            LayoutPreset::Debug => "\u{1f50d}",
        }
    }

    /// Get keyboard shortcut hint for this preset
    pub fn shortcut_hint(&self) -> Option<&'static str> {
        match self {
            LayoutPreset::Default => Some("Ctrl+1"),
            LayoutPreset::Wide => Some("Ctrl+2"),
            LayoutPreset::Compact => Some("Ctrl+3"),
            LayoutPreset::Modeling => Some("Ctrl+4"),
            LayoutPreset::Animation => Some("Ctrl+5"),
            LayoutPreset::Debug => Some("Ctrl+6"),
        }
    }

    /// Get expected panel count for this preset
    pub fn expected_panel_count(&self) -> usize {
        match self {
            LayoutPreset::Default => 8, // Hierarchy, AssetBrowser, Viewport, Inspector, Transform, Console, Profiler, SceneStats
            LayoutPreset::Wide => 3,    // Hierarchy, Viewport, Inspector
            LayoutPreset::Compact => 10, // Many panels
            LayoutPreset::Modeling => 4, // Hierarchy, Viewport, Transform, Inspector
            LayoutPreset::Animation => 6, // Hierarchy, Viewport, BehaviorGraph, Inspector, Animation, Graph
            LayoutPreset::Debug => 6, // Hierarchy, Viewport, Performance, SceneStats, Console, Profiler
        }
    }

    /// Check if this is a debug/development layout
    pub fn is_debug_layout(&self) -> bool {
        matches!(self, LayoutPreset::Debug)
    }

    /// Check if this is a content creation layout
    pub fn is_content_creation_layout(&self) -> bool {
        matches!(self, LayoutPreset::Modeling | LayoutPreset::Animation)
    }
}

impl std::fmt::Display for LayoutPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Statistics about the current dock layout
#[derive(Debug, Clone, Default)]
pub struct LayoutStats {
    /// Total number of visible panels
    pub panel_count: usize,
    /// Number of tab groups (panels grouped together)
    pub tab_group_count: usize,
    /// List of all visible panel types
    pub visible_panels: Vec<PanelType>,
    /// Whether viewport is visible
    pub has_viewport: bool,
    /// Whether any debug panels are visible
    pub has_debug_panels: bool,
}

/// Docking layout manager
///
/// Manages the `egui_dock::DockState` and provides methods
/// for creating, saving, and restoring panel layouts.
pub struct DockLayout {
    /// The dock state containing all panel positions
    dock_state: DockState<PanelType>,

    /// Currently focused panel (if any)
    focused_panel: Option<PanelType>,

    /// Style configuration for the dock area
    style: Style,
}

impl Default for DockLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl DockLayout {
    /// Create a new dock layout with default configuration
    pub fn new() -> Self {
        let dock_state = Self::create_default_layout();

        Self {
            dock_state,
            focused_panel: None,
            style: Self::create_editor_style(),
        }
    }

    /// Create layout from a preset configuration
    pub fn from_preset(preset: LayoutPreset) -> Self {
        let dock_state = match preset {
            LayoutPreset::Default => Self::create_default_layout(),
            LayoutPreset::Wide => Self::create_wide_layout(),
            LayoutPreset::Compact => Self::create_compact_layout(),
            LayoutPreset::Modeling => Self::create_modeling_layout(),
            LayoutPreset::Animation => Self::create_animation_layout(),
            LayoutPreset::Debug => Self::create_debug_layout(),
        };

        Self {
            dock_state,
            focused_panel: None,
            style: Self::create_editor_style(),
        }
    }

    /// Get mutable reference to the dock state
    pub fn dock_state_mut(&mut self) -> &mut DockState<PanelType> {
        &mut self.dock_state
    }

    /// Get reference to the dock state
    pub fn dock_state(&self) -> &DockState<PanelType> {
        &self.dock_state
    }

    /// Get reference to the dock style
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Get mutable reference to the dock style
    pub fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }

    /// Get the currently focused panel
    pub fn focused_panel(&self) -> Option<PanelType> {
        self.focused_panel
    }

    /// Set the focused panel
    pub fn set_focused_panel(&mut self, panel: Option<PanelType>) {
        self.focused_panel = panel;
    }

    /// Add a panel to the layout
    ///
    /// If the panel already exists, it will be focused.
    /// Otherwise, it will be added to the right side.
    pub fn add_panel(&mut self, panel: PanelType) {
        // Check if panel already exists
        if self.find_panel(&panel).is_some() {
            // Focus existing panel
            self.focused_panel = Some(panel);
            return;
        }

        // Add to main surface at root
        let surface = self.dock_state.main_surface_mut();
        surface.push_to_focused_leaf(panel);
    }

    /// Remove a panel from the layout
    ///
    /// Returns true if the panel was removed, false if it wasn't found
    /// or couldn't be removed (e.g., Viewport is not closable).
    pub fn remove_panel(&mut self, panel: &PanelType) -> bool {
        if !panel.is_closable() {
            return false;
        }

        if let Some((surface_idx, node_idx, tab_idx)) = self.find_panel(panel) {
            self.dock_state.remove_tab((surface_idx, node_idx, tab_idx));
            return true;
        }

        false
    }

    /// Find a panel in the layout
    ///
    /// Returns the surface, node, and tab indices if found.
    pub fn find_panel(&self, panel: &PanelType) -> Option<(SurfaceIndex, NodeIndex, TabIndex)> {
        self.dock_state.find_tab(panel)
    }

    /// Check if a panel is visible in the layout
    pub fn is_panel_visible(&self, panel: &PanelType) -> bool {
        self.find_panel(panel).is_some()
    }

    /// Alias for is_panel_visible - checks if panel exists in layout
    pub fn has_panel(&self, panel: &PanelType) -> bool {
        self.is_panel_visible(panel)
    }

    /// Toggle a panel's visibility
    pub fn toggle_panel(&mut self, panel: PanelType) {
        if self.is_panel_visible(&panel) {
            self.remove_panel(&panel);
        } else {
            self.add_panel(panel);
        }
    }

    /// Reset layout to default configuration
    pub fn reset_to_default(&mut self) {
        self.dock_state = Self::create_default_layout();
    }

    /// Apply a preset layout
    pub fn apply_preset(&mut self, preset: LayoutPreset) {
        self.dock_state = match preset {
            LayoutPreset::Default => Self::create_default_layout(),
            LayoutPreset::Wide => Self::create_wide_layout(),
            LayoutPreset::Compact => Self::create_compact_layout(),
            LayoutPreset::Modeling => Self::create_modeling_layout(),
            LayoutPreset::Animation => Self::create_animation_layout(),
            LayoutPreset::Debug => Self::create_debug_layout(),
        };
    }

    /// Serialize the layout to JSON for persistence
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        // Note: DockState serialization requires custom handling
        // For now, we'll serialize the panel list and positions
        let layout_data = LayoutData {
            panels: self.collect_visible_panels(),
        };
        serde_json::to_string_pretty(&layout_data)
    }

    /// Deserialize layout from JSON
    ///
    /// Restores visible panels from saved data.
    /// Panel positions use the default layout as a base since
    /// `DockState` tree structure is not serialized.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let layout_data: LayoutData = serde_json::from_str(json)?;
        let mut layout = Self::new();
        // Sync the panel set: ensure only saved panels are present
        let saved: std::collections::HashSet<PanelType> =
            layout_data.panels.iter().copied().collect();
        let current: Vec<PanelType> = layout.collect_visible_panels();
        // Remove panels not in saved set
        for panel in &current {
            if !saved.contains(panel) {
                layout.remove_panel(panel);
            }
        }
        // Add panels that were saved but missing from default
        let after_remove: std::collections::HashSet<PanelType> =
            layout.collect_visible_panels().into_iter().collect();
        for panel in &layout_data.panels {
            if !after_remove.contains(panel) {
                layout.add_panel(*panel);
            }
        }
        Ok(layout)
    }

    /// Collect list of all visible panels
    fn collect_visible_panels(&self) -> Vec<PanelType> {
        self.dock_state
            .iter_all_tabs()
            .map(|(_, tab)| *tab)
            .collect()
    }

    // === Layout Creation Methods ===

    /// Create the default layout
    ///
    /// Layout structure:
    /// ```text
    /// +----------------------------------------+
    /// | Hierarchy  |            |  Inspector   |
    /// | AssetBrowse|  Viewport  |  Transform   |
    /// |            |            |              |
    /// +------------+------------+--------------+
    /// |      Console | Profiler | Stats        |
    /// +----------------------------------------+
    /// ```
    fn create_default_layout() -> DockState<PanelType> {
        let mut dock_state = DockState::new(vec![PanelType::Viewport]);
        let surface = dock_state.main_surface_mut();

        // Split left panel (18% width for hierarchy/assets)
        let [_left_panel, _center] = surface.split_left(
            NodeIndex::root(),
            0.18,
            vec![PanelType::Hierarchy, PanelType::AssetBrowser],
        );

        // Split right panel (18% width for inspector/transform)
        let [_center, _right_panel] = surface.split_right(
            NodeIndex::root(),
            0.82,
            vec![PanelType::Inspector, PanelType::Transform],
        );

        // Split bottom panel (20% height for console/profiler)
        let [_top, _bottom] = surface.split_below(
            NodeIndex::root(),
            0.80,
            vec![
                PanelType::Console,
                PanelType::Profiler,
                PanelType::SceneStats,
            ],
        );

        dock_state
    }

    /// Create wide layout (maximized viewport)
    fn create_wide_layout() -> DockState<PanelType> {
        let mut dock_state = DockState::new(vec![PanelType::Viewport]);
        let surface = dock_state.main_surface_mut();

        // Left panel for hierarchy (12%)
        let [_left, _center] =
            surface.split_left(NodeIndex::root(), 0.12, vec![PanelType::Hierarchy]);

        // Minimal right panel (15%)
        let [_center, _right_panel] =
            surface.split_right(NodeIndex::root(), 0.85, vec![PanelType::Inspector]);

        dock_state
    }

    /// Create compact layout (all panels visible)
    fn create_compact_layout() -> DockState<PanelType> {
        let mut dock_state = DockState::new(vec![PanelType::Viewport]);
        let surface = dock_state.main_surface_mut();

        // Left panel for hierarchy/assets (20%)
        let [_left, _center] = surface.split_left(
            NodeIndex::root(),
            0.20,
            vec![PanelType::Hierarchy, PanelType::AssetBrowser],
        );

        // Right panel (25%)
        let [_center, _right_panel] = surface.split_right(
            NodeIndex::root(),
            0.75,
            vec![
                PanelType::Inspector,
                PanelType::Transform,
                PanelType::EntityPanel,
            ],
        );

        // Bottom panel (30%)
        let [_top, _bottom] = surface.split_below(
            NodeIndex::root(),
            0.70,
            vec![
                PanelType::Console,
                PanelType::Profiler,
                PanelType::SceneStats,
                PanelType::Performance,
            ],
        );

        dock_state
    }

    /// Create modeling layout
    fn create_modeling_layout() -> DockState<PanelType> {
        let mut dock_state = DockState::new(vec![PanelType::Viewport]);
        let surface = dock_state.main_surface_mut();

        // Left panel for hierarchy (15%)
        let [_left, _center] =
            surface.split_left(NodeIndex::root(), 0.15, vec![PanelType::Hierarchy]);

        // Right panel with transform tools (20%)
        let [_center, _right_panel] = surface.split_right(
            NodeIndex::root(),
            0.80,
            vec![PanelType::Transform, PanelType::Inspector],
        );

        dock_state
    }

    /// Create animation layout
    fn create_animation_layout() -> DockState<PanelType> {
        let mut dock_state = DockState::new(vec![PanelType::Viewport]);
        let surface = dock_state.main_surface_mut();

        // Left panel for hierarchy (15%)
        let [_left, _center] =
            surface.split_left(NodeIndex::root(), 0.15, vec![PanelType::Hierarchy]);

        // Right panel with animation graph (25%)
        let [_center, _right_panel] = surface.split_right(
            NodeIndex::root(),
            0.75,
            vec![PanelType::BehaviorGraph, PanelType::Inspector],
        );

        // Large bottom panel for timeline (35%)
        let [_top, _bottom] = surface.split_below(
            NodeIndex::root(),
            0.65,
            vec![PanelType::Animation, PanelType::Graph],
        );

        dock_state
    }

    /// Create debug layout
    fn create_debug_layout() -> DockState<PanelType> {
        let mut dock_state = DockState::new(vec![PanelType::Viewport]);
        let surface = dock_state.main_surface_mut();

        // Left panel for hierarchy (15%)
        let [_left, _center] =
            surface.split_left(NodeIndex::root(), 0.15, vec![PanelType::Hierarchy]);

        // Right panel with performance (25%)
        let [_center, _right_panel] = surface.split_right(
            NodeIndex::root(),
            0.75,
            vec![PanelType::Performance, PanelType::SceneStats],
        );

        // Large bottom panel for console/profiler (40%)
        let [_top, _bottom] = surface.split_below(
            NodeIndex::root(),
            0.60,
            vec![PanelType::Console, PanelType::Profiler],
        );

        dock_state
    }

    /// Create editor-specific dock style
    fn create_editor_style() -> Style {
        let mut style = Style::from_egui(&egui::Style::default());

        // Customize tab bar appearance
        style.tab_bar.height = 24.0;
        // Note: show_scroll_buttons_on_hover removed in egui_dock 0.17

        // Customize tab appearance
        style.tab.tab_body.inner_margin = egui::Margin::symmetric(8, 4);
        // Tab body uses default theme colors

        // Separator styling
        style.separator.width = 4.0;
        // Separator uses default theme colors for idle/hover states

        // Tab bar uses default theme background

        style
    }

    /// Show the dock layout using a TabViewer implementation
    ///
    /// This renders the entire docking area with all panels.
    /// The `tab_viewer` parameter controls how each panel is rendered.
    pub fn show<T: TabViewer<Tab = PanelType>>(&mut self, ctx: &egui::Context, tab_viewer: &mut T) {
        // Update style from current egui style for theme consistency
        self.style = Style::from_egui(ctx.style().as_ref());
        self.style.tab_bar.height = 24.0;
        self.style.tab.tab_body.inner_margin = egui::Margin::symmetric(8, 4);
        self.style.separator.width = 4.0;
        // Production theme uses default egui_dock colors derived from egui style

        DockArea::new(&mut self.dock_state)
            .style(self.style.clone())
            .draggable_tabs(true)
            .show(ctx, tab_viewer);
    }

    /// Show the dock layout inside a UI region
    ///
    /// Use this when embedding the dock in an existing panel.
    pub fn show_inside<T: TabViewer<Tab = PanelType>>(
        &mut self,
        ui: &mut egui::Ui,
        tab_viewer: &mut T,
    ) {
        // Update style from current egui style for theme consistency
        self.style = Style::from_egui(ui.style().as_ref());
        self.style.tab_bar.height = 24.0;
        self.style.tab.tab_body.inner_margin = egui::Margin::symmetric(8, 4);
        self.style.separator.width = 4.0;
        // Production theme uses default egui_dock colors derived from egui style

        DockArea::new(&mut self.dock_state)
            .style(self.style.clone())
            .draggable_tabs(true)
            .show_inside(ui, tab_viewer);
    }

    // === New Statistics and Query Methods ===

    /// Get statistics about the current layout
    pub fn stats(&self) -> LayoutStats {
        let visible_panels = self.collect_visible_panels();
        let panel_count = visible_panels.len();
        let has_viewport = visible_panels.contains(&PanelType::Viewport);
        let has_debug_panels = visible_panels.iter().any(|p| {
            matches!(
                p,
                PanelType::Console
                    | PanelType::Profiler
                    | PanelType::Performance
                    | PanelType::SceneStats
            )
        });

        // Count tab groups (approximate - one per unique position)
        let tab_group_count = self
            .dock_state
            .iter_all_tabs()
            .count()
            .saturating_sub(visible_panels.len().saturating_sub(1))
            .max(1);

        LayoutStats {
            panel_count,
            tab_group_count,
            visible_panels,
            has_viewport,
            has_debug_panels,
        }
    }

    /// Get count of visible panels
    pub fn panel_count(&self) -> usize {
        self.collect_visible_panels().len()
    }

    /// Get all currently visible panels
    pub fn visible_panels(&self) -> Vec<PanelType> {
        self.collect_visible_panels()
    }

    /// Check if any debug panels are visible
    pub fn has_debug_panels(&self) -> bool {
        self.collect_visible_panels().iter().any(|p| {
            matches!(
                p,
                PanelType::Console
                    | PanelType::Profiler
                    | PanelType::Performance
                    | PanelType::SceneStats
            )
        })
    }

    /// Check if layout has minimum required panels (at least viewport)
    pub fn is_valid(&self) -> bool {
        self.is_panel_visible(&PanelType::Viewport)
    }

    /// Get panels that are missing compared to expected preset
    pub fn missing_panels_for_preset(&self, preset: LayoutPreset) -> Vec<PanelType> {
        let current = self.collect_visible_panels();
        let expected = DockLayout::from_preset(preset).collect_visible_panels();
        expected
            .into_iter()
            .filter(|p| !current.contains(p))
            .collect()
    }

    /// Check if current layout matches a preset
    pub fn matches_preset(&self, preset: LayoutPreset) -> bool {
        let current: std::collections::HashSet<_> =
            self.collect_visible_panels().into_iter().collect();
        let expected: std::collections::HashSet<_> = DockLayout::from_preset(preset)
            .collect_visible_panels()
            .into_iter()
            .collect();
        current == expected
    }

    /// Find which preset best matches current layout
    pub fn closest_preset(&self) -> LayoutPreset {
        let current: std::collections::HashSet<_> =
            self.collect_visible_panels().into_iter().collect();

        LayoutPreset::all()
            .iter()
            .copied()
            .max_by_key(|&preset| {
                let preset_panels: std::collections::HashSet<_> = DockLayout::from_preset(preset)
                    .collect_visible_panels()
                    .into_iter()
                    .collect();
                current.intersection(&preset_panels).count()
            })
            .unwrap_or(LayoutPreset::Default)
    }
}

/// Serializable layout data for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayoutData {
    panels: Vec<PanelType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_layout_creation() {
        let layout = DockLayout::new();

        // Should have viewport
        assert!(layout.is_panel_visible(&PanelType::Viewport));

        // Should have inspector (right panel in dock)
        assert!(layout.is_panel_visible(&PanelType::Inspector));

        // Should have console (bottom panel in dock)
        assert!(layout.is_panel_visible(&PanelType::Console));

        // Should have hierarchy (left panel in dock)
        assert!(layout.is_panel_visible(&PanelType::Hierarchy));
    }

    #[test]
    fn test_preset_layouts() {
        for preset in [
            LayoutPreset::Default,
            LayoutPreset::Wide,
            LayoutPreset::Compact,
            LayoutPreset::Modeling,
            LayoutPreset::Animation,
            LayoutPreset::Debug,
        ] {
            let layout = DockLayout::from_preset(preset);
            // All presets should have viewport
            assert!(layout.is_panel_visible(&PanelType::Viewport));
        }
    }

    #[test]
    fn test_add_panel() {
        let mut layout = DockLayout::new();

        // Add a panel that might not be visible
        layout.add_panel(PanelType::Charts);

        // Should now be visible
        assert!(layout.is_panel_visible(&PanelType::Charts));
    }

    #[test]
    fn test_toggle_panel() {
        let mut layout = DockLayout::new();

        // Console should be visible by default
        assert!(layout.is_panel_visible(&PanelType::Console));

        // Toggle off
        layout.toggle_panel(PanelType::Console);
        assert!(!layout.is_panel_visible(&PanelType::Console));

        // Toggle on
        layout.toggle_panel(PanelType::Console);
        assert!(layout.is_panel_visible(&PanelType::Console));
    }

    #[test]
    fn test_viewport_not_closable() {
        let mut layout = DockLayout::new();

        // Try to remove viewport
        let removed = layout.remove_panel(&PanelType::Viewport);

        // Should fail because viewport is not closable
        assert!(!removed);
        assert!(layout.is_panel_visible(&PanelType::Viewport));
    }

    #[test]
    fn test_reset_to_default() {
        let mut layout = DockLayout::from_preset(LayoutPreset::Wide);

        // Reset
        layout.reset_to_default();

        // Should have default dock panels (not legacy side panel ones)
        assert!(layout.is_panel_visible(&PanelType::Console));
        assert!(layout.is_panel_visible(&PanelType::Inspector));
        assert!(layout.is_panel_visible(&PanelType::Viewport));
    }

    // === New LayoutPreset tests ===

    #[test]
    fn test_layout_preset_all() {
        let all = LayoutPreset::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&LayoutPreset::Default));
        assert!(all.contains(&LayoutPreset::Wide));
        assert!(all.contains(&LayoutPreset::Compact));
        assert!(all.contains(&LayoutPreset::Modeling));
        assert!(all.contains(&LayoutPreset::Animation));
        assert!(all.contains(&LayoutPreset::Debug));
    }

    #[test]
    fn test_layout_preset_names() {
        assert_eq!(LayoutPreset::Default.name(), "Default");
        assert_eq!(LayoutPreset::Wide.name(), "Wide");
        assert_eq!(LayoutPreset::Compact.name(), "Compact");
        assert_eq!(LayoutPreset::Modeling.name(), "Modeling");
        assert_eq!(LayoutPreset::Animation.name(), "Animation");
        assert_eq!(LayoutPreset::Debug.name(), "Debug");
    }

    #[test]
    fn test_layout_preset_descriptions_not_empty() {
        for preset in LayoutPreset::all() {
            assert!(!preset.description().is_empty());
        }
    }

    #[test]
    fn test_layout_preset_icons_not_empty() {
        for preset in LayoutPreset::all() {
            assert!(!preset.icon().is_empty());
        }
    }

    #[test]
    fn test_layout_preset_shortcut_hints() {
        for preset in LayoutPreset::all() {
            // All presets should have shortcut hints
            assert!(preset.shortcut_hint().is_some());
        }
    }

    #[test]
    fn test_layout_preset_expected_panel_counts() {
        assert!(LayoutPreset::Default.expected_panel_count() >= 2);
        assert!(LayoutPreset::Wide.expected_panel_count() >= 1);
        assert!(LayoutPreset::Compact.expected_panel_count() >= 4);
    }

    #[test]
    fn test_layout_preset_is_debug_layout() {
        assert!(LayoutPreset::Debug.is_debug_layout());
        assert!(!LayoutPreset::Default.is_debug_layout());
        assert!(!LayoutPreset::Wide.is_debug_layout());
    }

    #[test]
    fn test_layout_preset_is_content_creation_layout() {
        assert!(LayoutPreset::Modeling.is_content_creation_layout());
        assert!(LayoutPreset::Animation.is_content_creation_layout());
        assert!(!LayoutPreset::Default.is_content_creation_layout());
        assert!(!LayoutPreset::Debug.is_content_creation_layout());
    }

    #[test]
    fn test_layout_preset_display() {
        assert_eq!(format!("{}", LayoutPreset::Default), "Default");
        assert_eq!(format!("{}", LayoutPreset::Debug), "Debug");
    }

    // === New DockLayout stats tests ===

    #[test]
    fn test_layout_stats_default() {
        let layout = DockLayout::new();
        let stats = layout.stats();

        assert!(stats.panel_count >= 2);
        assert!(stats.has_viewport);
    }

    #[test]
    fn test_layout_panel_count() {
        let layout = DockLayout::new();
        assert!(layout.panel_count() >= 2);
    }

    #[test]
    fn test_layout_visible_panels() {
        let layout = DockLayout::new();
        let panels = layout.visible_panels();

        assert!(panels.contains(&PanelType::Viewport));
    }

    #[test]
    fn test_layout_has_debug_panels() {
        let layout = DockLayout::from_preset(LayoutPreset::Debug);
        assert!(layout.has_debug_panels());
    }

    #[test]
    fn test_layout_is_valid() {
        let layout = DockLayout::new();
        assert!(layout.is_valid());
    }

    #[test]
    fn test_layout_focused_panel() {
        let mut layout = DockLayout::new();
        assert!(layout.focused_panel().is_none());

        layout.set_focused_panel(Some(PanelType::Inspector));
        assert_eq!(layout.focused_panel(), Some(PanelType::Inspector));

        layout.set_focused_panel(None);
        assert!(layout.focused_panel().is_none());
    }

    #[test]
    fn test_layout_matches_preset() {
        let layout = DockLayout::from_preset(LayoutPreset::Default);
        assert!(layout.matches_preset(LayoutPreset::Default));
    }

    #[test]
    fn test_layout_closest_preset() {
        let layout = DockLayout::from_preset(LayoutPreset::Wide);
        // Should match Wide or find closest
        let closest = layout.closest_preset();
        // At minimum, it should return a valid preset
        assert!(LayoutPreset::all().contains(&closest));
    }

    // === LayoutStats tests ===

    #[test]
    fn test_layout_stats_default_struct() {
        let stats = LayoutStats::default();
        assert_eq!(stats.panel_count, 0);
        assert!(!stats.has_viewport);
        assert!(!stats.has_debug_panels);
    }

    #[test]
    fn test_create_editor_style_via_constructor() {
        // Kills: replace DockLayout::create_editor_style -> Style with Default::default()
        // The constructor calls create_editor_style() and stores the result.
        // We verify the style has our custom values, not default.
        let layout = DockLayout::new();
        let style = layout.style();

        // create_editor_style sets tab_bar.height = 24.0
        assert!(
            (style.tab_bar.height - 24.0).abs() < f32::EPSILON,
            "tab_bar.height must be 24.0 (from create_editor_style), got {}",
            style.tab_bar.height
        );

        // create_editor_style sets separator.width = 4.0
        assert!(
            (style.separator.width - 4.0).abs() < f32::EPSILON,
            "separator.width must be 4.0 (from create_editor_style), got {}",
            style.separator.width
        );
    }
}
