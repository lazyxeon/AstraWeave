//! Voxel Editing Tools for AstraWeave Editor
//!
//! This module provides interactive voxel editing capabilities including
//! brushes for adding/removing voxels, undo/redo, and real-time preview.

use astraweave_terrain::{ChunkCoord, MaterialId, Voxel, VoxelGrid};
use glam::Vec3;
use std::collections::VecDeque;

/// Brush shape for voxel editing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrushShape {
    /// Spherical brush
    Sphere,
    /// Cubic brush
    Cube,
    /// Cylindrical brush
    Cylinder,
}

impl BrushShape {
    /// Get all brush shape variants
    pub fn all() -> &'static [Self] {
        &[Self::Sphere, Self::Cube, Self::Cylinder]
    }

    /// Get human-readable name
    pub fn name(&self) -> &str {
        match self {
            Self::Sphere => "Sphere",
            Self::Cube => "Cube",
            Self::Cylinder => "Cylinder",
        }
    }

    /// Get icon for this shape
    pub fn icon(&self) -> &str {
        match self {
            Self::Sphere => "(O)",
            Self::Cube => "[#]",
            Self::Cylinder => "(|)",
        }
    }

    /// Get keyboard shortcut
    pub fn shortcut(&self) -> &str {
        match self {
            Self::Sphere => "1",
            Self::Cube => "2",
            Self::Cylinder => "3",
        }
    }
}

impl std::fmt::Display for BrushShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

/// Brush mode (add or remove voxels)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrushMode {
    /// Add voxels (increase density)
    Add,
    /// Remove voxels (decrease density)
    Remove,
    /// Paint material without changing density
    Paint,
}

impl BrushMode {
    /// Get all brush mode variants
    pub fn all() -> &'static [Self] {
        &[Self::Add, Self::Remove, Self::Paint]
    }

    /// Get human-readable name
    pub fn name(&self) -> &str {
        match self {
            Self::Add => "Add",
            Self::Remove => "Remove",
            Self::Paint => "Paint",
        }
    }

    /// Get icon for this mode
    pub fn icon(&self) -> &str {
        match self {
            Self::Add => "+",
            Self::Remove => "-",
            Self::Paint => "[P]",
        }
    }

    /// Check if this mode modifies density
    pub fn modifies_density(&self) -> bool {
        matches!(self, Self::Add | Self::Remove)
    }

    /// Check if this mode modifies material
    pub fn modifies_material(&self) -> bool {
        matches!(self, Self::Add | Self::Paint)
    }
}

impl std::fmt::Display for BrushMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

/// Brush configuration
#[derive(Debug, Clone, Copy)]
pub struct BrushConfig {
    /// Brush shape
    pub shape: BrushShape,
    /// Brush mode
    pub mode: BrushMode,
    /// Brush radius in world units
    pub radius: f32,
    /// Brush strength (0.0 to 1.0)
    pub strength: f32,
    /// Material ID to apply
    pub material: MaterialId,
    /// Smooth edges
    pub smooth: bool,
}

impl Default for BrushConfig {
    fn default() -> Self {
        Self {
            shape: BrushShape::Sphere,
            mode: BrushMode::Add,
            radius: 5.0,
            strength: 1.0,
            material: 1,
            smooth: true,
        }
    }
}

impl BrushConfig {
    /// Get approximate volume affected by the brush
    pub fn approximate_volume(&self) -> f32 {
        match self.shape {
            BrushShape::Sphere => (4.0 / 3.0) * std::f32::consts::PI * self.radius.powi(3),
            BrushShape::Cube => (2.0 * self.radius).powi(3),
            BrushShape::Cylinder => {
                std::f32::consts::PI * self.radius.powi(2) * (2.0 * self.radius)
            }
        }
    }

    /// Get effective strength with falloff consideration
    pub fn effective_strength(&self) -> f32 {
        if self.smooth {
            self.strength * 0.5 // Average falloff effect
        } else {
            self.strength
        }
    }

    /// Check if the brush is configured for additive operations
    pub fn is_additive(&self) -> bool {
        matches!(self.mode, BrushMode::Add)
    }

    /// Check if the brush is configured for subtractive operations
    pub fn is_subtractive(&self) -> bool {
        matches!(self.mode, BrushMode::Remove)
    }

    /// Generate a summary of the brush configuration
    pub fn summary(&self) -> String {
        format!(
            "{} {} r={:.1} s={:.0}%{}",
            self.shape.icon(),
            self.mode.name(),
            self.radius,
            self.strength * 100.0,
            if self.smooth { " (smooth)" } else { "" }
        )
    }
}

/// A single edit operation for undo/redo
#[derive(Debug, Clone)]
struct EditOperation {
    /// Position of the edit
    position: Vec3,
    /// Affected voxels before the edit
    before: Vec<(Vec3, Voxel)>,
    /// Affected voxels after the edit
    after: Vec<(Vec3, Voxel)>,
}

/// Voxel editor with brush tools and undo/redo
pub struct VoxelEditor {
    /// Current brush configuration
    brush: BrushConfig,
    /// Undo stack
    undo_stack: VecDeque<EditOperation>,
    /// Redo stack
    redo_stack: VecDeque<EditOperation>,
    /// Maximum undo history
    max_undo_history: usize,
    /// Preview mode (don't commit changes)
    preview_mode: bool,
}

impl VoxelEditor {
    /// Create a new voxel editor
    pub fn new() -> Self {
        Self {
            brush: BrushConfig::default(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_undo_history: 100,
            preview_mode: false,
        }
    }

    /// Set brush configuration
    pub fn set_brush(&mut self, brush: BrushConfig) {
        self.brush = brush;
    }

    /// Get current brush configuration
    pub fn brush(&self) -> &BrushConfig {
        &self.brush
    }

    /// Apply brush at world position
    pub fn apply_brush(&mut self, grid: &mut VoxelGrid, position: Vec3) -> Vec<ChunkCoord> {
        let mut operation = EditOperation {
            position,
            before: Vec::new(),
            after: Vec::new(),
        };

        let mut affected_chunks = Vec::new();

        // Get voxels in brush radius
        let voxels = self.get_voxels_in_brush(position);

        for voxel_pos in voxels {
            // Store before state
            if let Some(voxel) = grid.get_voxel(voxel_pos) {
                operation.before.push((voxel_pos, voxel));
            } else {
                operation.before.push((voxel_pos, Voxel::default()));
            }

            // Apply brush effect
            let new_voxel = self.apply_brush_to_voxel(
                grid.get_voxel(voxel_pos).unwrap_or_default(),
                voxel_pos,
                position,
            );

            // Set new voxel
            grid.set_voxel(voxel_pos, new_voxel);
            operation.after.push((voxel_pos, new_voxel));

            // Track affected chunk
            let chunk_coord = ChunkCoord::from_world_pos(voxel_pos);
            if !affected_chunks.contains(&chunk_coord) {
                affected_chunks.push(chunk_coord);
            }
        }

        // Add to undo stack if not in preview mode
        if !self.preview_mode {
            self.undo_stack.push_back(operation);
            if self.undo_stack.len() > self.max_undo_history {
                self.undo_stack.pop_front();
            }
            self.redo_stack.clear();
        }

        affected_chunks
    }

    /// Get all voxel positions within brush radius
    fn get_voxels_in_brush(&self, center: Vec3) -> Vec<Vec3> {
        let mut voxels = Vec::new();
        let radius_int = self.brush.radius.ceil() as i32;

        for x in -radius_int..=radius_int {
            for y in -radius_int..=radius_int {
                for z in -radius_int..=radius_int {
                    let offset = Vec3::new(x as f32, y as f32, z as f32);
                    let pos = center + offset;

                    if self.is_in_brush(pos, center) {
                        voxels.push(pos);
                    }
                }
            }
        }

        voxels
    }

    /// Check if position is within brush shape
    fn is_in_brush(&self, pos: Vec3, center: Vec3) -> bool {
        let offset = pos - center;

        match self.brush.shape {
            BrushShape::Sphere => offset.length() <= self.brush.radius,
            BrushShape::Cube => {
                offset.x.abs() <= self.brush.radius
                    && offset.y.abs() <= self.brush.radius
                    && offset.z.abs() <= self.brush.radius
            }
            BrushShape::Cylinder => {
                let horizontal_dist = (offset.x * offset.x + offset.z * offset.z).sqrt();
                horizontal_dist <= self.brush.radius && offset.y.abs() <= self.brush.radius
            }
        }
    }

    /// Apply brush effect to a single voxel
    fn apply_brush_to_voxel(&self, mut voxel: Voxel, pos: Vec3, center: Vec3) -> Voxel {
        let distance = (pos - center).length();
        let falloff = if self.brush.smooth {
            (1.0 - distance / self.brush.radius).max(0.0)
        } else {
            1.0
        };

        let strength = self.brush.strength * falloff;

        match self.brush.mode {
            BrushMode::Add => {
                voxel.density = (voxel.density + strength).min(1.0);
                if voxel.density > 0.5 {
                    voxel.material = self.brush.material;
                }
            }
            BrushMode::Remove => {
                voxel.density = (voxel.density - strength).max(0.0);
            }
            BrushMode::Paint => {
                if voxel.density > 0.5 {
                    voxel.material = self.brush.material;
                }
            }
        }

        voxel
    }

    /// Undo last operation
    pub fn undo(&mut self, grid: &mut VoxelGrid) -> Option<Vec<ChunkCoord>> {
        if let Some(operation) = self.undo_stack.pop_back() {
            let mut affected_chunks = Vec::new();

            // Restore before state
            for (pos, voxel) in &operation.before {
                grid.set_voxel(*pos, *voxel);
                let chunk_coord = ChunkCoord::from_world_pos(*pos);
                if !affected_chunks.contains(&chunk_coord) {
                    affected_chunks.push(chunk_coord);
                }
            }

            // Add to redo stack
            self.redo_stack.push_back(operation);

            Some(affected_chunks)
        } else {
            None
        }
    }

    /// Redo last undone operation
    pub fn redo(&mut self, grid: &mut VoxelGrid) -> Option<Vec<ChunkCoord>> {
        if let Some(operation) = self.redo_stack.pop_back() {
            let mut affected_chunks = Vec::new();

            // Restore after state
            for (pos, voxel) in &operation.after {
                grid.set_voxel(*pos, *voxel);
                let chunk_coord = ChunkCoord::from_world_pos(*pos);
                if !affected_chunks.contains(&chunk_coord) {
                    affected_chunks.push(chunk_coord);
                }
            }

            // Add back to undo stack
            self.undo_stack.push_back(operation);

            Some(affected_chunks)
        } else {
            None
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear undo/redo history
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Enable/disable preview mode
    pub fn set_preview_mode(&mut self, enabled: bool) {
        self.preview_mode = enabled;
    }

    /// Check if in preview mode
    pub fn is_preview_mode(&self) -> bool {
        self.preview_mode
    }

    /// Get the number of undo operations available
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of redo operations available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Get maximum undo history size
    pub fn max_history(&self) -> usize {
        self.max_undo_history
    }

    /// Check if undo history is at capacity
    pub fn is_history_full(&self) -> bool {
        self.undo_stack.len() >= self.max_undo_history
    }

    /// Get total number of voxel edits in history
    pub fn total_edits_in_history(&self) -> usize {
        self.undo_stack
            .iter()
            .map(|op| op.after.len())
            .sum::<usize>()
            + self
                .redo_stack
                .iter()
                .map(|op| op.after.len())
                .sum::<usize>()
    }

    /// Get editor statistics
    pub fn stats(&self) -> VoxelEditorStats {
        VoxelEditorStats {
            undo_available: self.undo_stack.len(),
            redo_available: self.redo_stack.len(),
            max_history: self.max_undo_history,
            total_edits: self.total_edits_in_history(),
            preview_mode: self.preview_mode,
        }
    }
}

/// Statistics for the voxel editor
#[derive(Debug, Clone)]
pub struct VoxelEditorStats {
    /// Number of undo operations available
    pub undo_available: usize,
    /// Number of redo operations available
    pub redo_available: usize,
    /// Maximum history size
    pub max_history: usize,
    /// Total edits in history
    pub total_edits: usize,
    /// Whether in preview mode
    pub preview_mode: bool,
}

impl VoxelEditorStats {
    /// Check if any history is available
    pub fn has_history(&self) -> bool {
        self.undo_available > 0 || self.redo_available > 0
    }

    /// Get history usage percentage
    pub fn history_usage_percent(&self) -> f32 {
        if self.max_history == 0 {
            0.0
        } else {
            (self.undo_available as f32 / self.max_history as f32) * 100.0
        }
    }
}

impl Default for VoxelEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Voxel raycasting for editor interaction
pub struct VoxelRaycaster {
    /// Maximum ray distance
    max_distance: f32,
    /// Step size for ray marching
    step_size: f32,
}

impl VoxelRaycaster {
    /// Create a new voxel raycaster
    pub fn new(max_distance: f32) -> Self {
        Self {
            max_distance,
            step_size: 0.5,
        }
    }

    /// Cast a ray and find the first solid voxel
    pub fn raycast(&self, grid: &VoxelGrid, origin: Vec3, direction: Vec3) -> Option<Vec3> {
        let direction = direction.normalize();
        let mut current_pos = origin;
        let mut distance = 0.0;

        while distance < self.max_distance {
            if let Some(voxel) = grid.get_voxel(current_pos) {
                if voxel.is_solid() {
                    return Some(current_pos);
                }
            }

            current_pos += direction * self.step_size;
            distance += self.step_size;
        }

        None
    }

    /// Cast a ray and find the first empty voxel adjacent to a solid voxel
    pub fn raycast_surface(&self, grid: &VoxelGrid, origin: Vec3, direction: Vec3) -> Option<Vec3> {
        let direction = direction.normalize();
        let mut current_pos = origin;
        let mut distance = 0.0;
        let mut last_pos = origin;

        while distance < self.max_distance {
            if let Some(voxel) = grid.get_voxel(current_pos) {
                if voxel.is_solid() {
                    // Return the last empty position before hitting solid
                    return Some(last_pos);
                }
            }

            last_pos = current_pos;
            current_pos += direction * self.step_size;
            distance += self.step_size;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brush_config_default() {
        let brush = BrushConfig::default();
        assert_eq!(brush.shape, BrushShape::Sphere);
        assert_eq!(brush.mode, BrushMode::Add);
        assert_eq!(brush.radius, 5.0);
    }

    #[test]
    fn test_voxel_editor_creation() {
        let editor = VoxelEditor::new();
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn test_brush_sphere_shape() {
        let editor = VoxelEditor::new();
        let center = Vec3::new(0.0, 0.0, 0.0);

        assert!(editor.is_in_brush(Vec3::new(0.0, 0.0, 0.0), center));
        assert!(editor.is_in_brush(Vec3::new(3.0, 0.0, 0.0), center));
        assert!(!editor.is_in_brush(Vec3::new(10.0, 0.0, 0.0), center));
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = VoxelEditor::new();
        let mut grid = VoxelGrid::new();

        let pos = Vec3::new(10.0, 10.0, 10.0);
        editor.apply_brush(&mut grid, pos);

        assert!(editor.can_undo());
        assert!(!editor.can_redo());

        editor.undo(&mut grid);
        assert!(!editor.can_undo());
        assert!(editor.can_redo());

        editor.redo(&mut grid);
        assert!(editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn test_raycaster() {
        let raycaster = VoxelRaycaster::new(100.0);
        let mut grid = VoxelGrid::new();

        // Add a solid voxel
        grid.set_voxel(Vec3::new(10.0, 0.0, 0.0), Voxel::new(1.0, 1));

        let origin = Vec3::new(0.0, 0.0, 0.0);
        let direction = Vec3::new(1.0, 0.0, 0.0);

        let hit = raycaster.raycast(&grid, origin, direction);
        assert!(hit.is_some());
    }

    // ====================================================================
    // BrushShape New Methods Tests
    // ====================================================================

    #[test]
    fn test_brush_shape_all() {
        let all = BrushShape::all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_brush_shape_name() {
        assert_eq!(BrushShape::Sphere.name(), "Sphere");
        assert_eq!(BrushShape::Cube.name(), "Cube");
        assert_eq!(BrushShape::Cylinder.name(), "Cylinder");
    }

    #[test]
    fn test_brush_shape_icon_not_empty() {
        for shape in BrushShape::all() {
            assert!(!shape.icon().is_empty());
        }
    }

    #[test]
    fn test_brush_shape_shortcut() {
        assert_eq!(BrushShape::Sphere.shortcut(), "1");
        assert_eq!(BrushShape::Cube.shortcut(), "2");
        assert_eq!(BrushShape::Cylinder.shortcut(), "3");
    }

    #[test]
    fn test_brush_shape_display() {
        assert_eq!(format!("{}", BrushShape::Sphere), "(O) Sphere");
        assert_eq!(format!("{}", BrushShape::Cube), "[#] Cube");
        assert_eq!(format!("{}", BrushShape::Cylinder), "(|) Cylinder");
    }

    // ====================================================================
    // BrushMode New Methods Tests
    // ====================================================================

    #[test]
    fn test_brush_mode_all() {
        let all = BrushMode::all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_brush_mode_name() {
        assert_eq!(BrushMode::Add.name(), "Add");
        assert_eq!(BrushMode::Remove.name(), "Remove");
        assert_eq!(BrushMode::Paint.name(), "Paint");
    }

    #[test]
    fn test_brush_mode_icon_not_empty() {
        for mode in BrushMode::all() {
            assert!(!mode.icon().is_empty());
        }
    }

    #[test]
    fn test_brush_mode_modifies_density() {
        assert!(BrushMode::Add.modifies_density());
        assert!(BrushMode::Remove.modifies_density());
        assert!(!BrushMode::Paint.modifies_density());
    }

    #[test]
    fn test_brush_mode_modifies_material() {
        assert!(BrushMode::Add.modifies_material());
        assert!(!BrushMode::Remove.modifies_material());
        assert!(BrushMode::Paint.modifies_material());
    }

    #[test]
    fn test_brush_mode_display() {
        assert_eq!(format!("{}", BrushMode::Add), "+ Add");
        assert_eq!(format!("{}", BrushMode::Remove), "- Remove");
        assert_eq!(format!("{}", BrushMode::Paint), "[P] Paint");
    }

    // ====================================================================
    // BrushConfig New Methods Tests
    // ====================================================================

    #[test]
    fn test_brush_config_approximate_volume() {
        let sphere = BrushConfig {
            shape: BrushShape::Sphere,
            radius: 5.0,
            ..Default::default()
        };
        assert!(sphere.approximate_volume() > 0.0);

        let cube = BrushConfig {
            shape: BrushShape::Cube,
            radius: 5.0,
            ..Default::default()
        };
        assert!(cube.approximate_volume() > sphere.approximate_volume());
    }

    #[test]
    fn test_brush_config_effective_strength() {
        let smooth = BrushConfig {
            smooth: true,
            strength: 1.0,
            ..Default::default()
        };
        assert!(smooth.effective_strength() < 1.0);

        let hard = BrushConfig {
            smooth: false,
            strength: 1.0,
            ..Default::default()
        };
        assert!((hard.effective_strength() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_brush_config_is_additive() {
        let add = BrushConfig {
            mode: BrushMode::Add,
            ..Default::default()
        };
        assert!(add.is_additive());
        assert!(!add.is_subtractive());
    }

    #[test]
    fn test_brush_config_is_subtractive() {
        let remove = BrushConfig {
            mode: BrushMode::Remove,
            ..Default::default()
        };
        assert!(remove.is_subtractive());
        assert!(!remove.is_additive());
    }

    #[test]
    fn test_brush_config_summary() {
        let config = BrushConfig::default();
        let summary = config.summary();
        assert!(summary.contains("Add"));
        assert!(summary.contains("5.0"));
    }

    // ====================================================================
    // VoxelEditor Stats Tests
    // ====================================================================

    #[test]
    fn test_voxel_editor_undo_redo_count() {
        let editor = VoxelEditor::new();
        assert_eq!(editor.undo_count(), 0);
        assert_eq!(editor.redo_count(), 0);
    }

    #[test]
    fn test_voxel_editor_max_history() {
        let editor = VoxelEditor::new();
        assert_eq!(editor.max_history(), 100);
    }

    #[test]
    fn test_voxel_editor_is_history_full() {
        let editor = VoxelEditor::new();
        assert!(!editor.is_history_full());
    }

    #[test]
    fn test_voxel_editor_stats() {
        let editor = VoxelEditor::new();
        let stats = editor.stats();
        assert_eq!(stats.undo_available, 0);
        assert_eq!(stats.redo_available, 0);
        assert!(!stats.preview_mode);
    }

    #[test]
    fn test_voxel_editor_stats_has_history() {
        let stats = VoxelEditorStats {
            undo_available: 0,
            redo_available: 0,
            max_history: 100,
            total_edits: 0,
            preview_mode: false,
        };
        assert!(!stats.has_history());

        let stats_with_history = VoxelEditorStats {
            undo_available: 5,
            redo_available: 0,
            max_history: 100,
            total_edits: 10,
            preview_mode: false,
        };
        assert!(stats_with_history.has_history());
    }

    #[test]
    fn test_voxel_editor_stats_history_usage_percent() {
        let stats = VoxelEditorStats {
            undo_available: 50,
            redo_available: 0,
            max_history: 100,
            total_edits: 50,
            preview_mode: false,
        };
        assert!((stats.history_usage_percent() - 50.0).abs() < 0.1);
    }
}
