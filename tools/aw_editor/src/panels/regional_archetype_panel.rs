// Phase 1.X-F.5-paint: RegionalArchetypePanel — paintable archetype mask
// editor panel. Per campaign doc §2.8 (authoring affordances) and §8 (F.5
// specification).
//
// F.5-paint is the first half of a two-session split:
// - F.5-paint (this file): scaffold + brush UX + save/load.
// - F.5-overlay-and-gate: Climate Preview overlay + integration tests +
//   Andrew-gate.
//
// Architectural inheritance from F.4 (canonical, do not revisit):
// - `RegionalArchetypeMask` data type + persistence API.
// - `WorldGenerator.regional_archetype_mask` field as integration surface.
// - `WorldArchetypeId::to_mask_id` / `from_mask_id` for ID/enum bridging.

use astraweave_terrain::regional_archetype_mask::RegionalArchetypeMask;
use astraweave_terrain::world_archetypes::WorldArchetypeId;
use egui::Ui;

use super::Panel;

/// Phase 1.X-F.5-paint.A: paint mode for the panel's archetype palette.
/// Paint = write the selected archetype's mask ID; Erase = write 0
/// (unpainted; sample-time fallback to Continental Temperate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintMode {
    Paint,
    Erase,
}

/// Phase 1.X-F.5-paint.A: queued paint operation. Operations accumulate
/// during pointer events and apply at end-of-frame (single batched
/// `recompute_falloff` call after all operations land).
///
/// `archetype_id == 0` = erase; `1-6` = catalog archetype IDs.
#[derive(Debug, Clone, Copy)]
pub struct PaintOp {
    pub world_x: f32,
    pub world_z: f32,
    pub brush_size_pixels: u32,
    pub archetype_id: u8,
}

/// Phase 1.X-F.5-paint.A: editor panel for paintable archetype mask
/// authoring. Mirrors `TerrainPanel`'s structural pattern (egui-driven;
/// `Panel` trait via `name()` + `show(&mut Ui)`).
///
/// State:
/// - `brush_size_pixels` / `falloff_radius_pixels`: brush configuration.
/// - `selected_archetype` / `paint_mode`: palette selection.
/// - `paint_active`: pointer-capture flag (set during click+drag).
/// - `pending_paint_ops`: queue of paint operations applied at
///   end-of-frame. F.5-paint.A ships with empty queue handling; F.5-paint.B
///   adds the actual paint logic + queue processing.
///
/// **F.5-paint.A scope**: panel scaffold renders UI controls but doesn't
/// mutate the mask yet. Brush implementation lands in F.5-paint.B;
/// save/load wiring lands in F.5-paint.C.
pub struct RegionalArchetypePanel {
    /// Brush size in mask pixels. Default `DEFAULT_BRUSH_SIZE_PIXELS = 32`.
    pub brush_size_pixels: u32,
    /// Falloff distance in mask pixels (configures transition zone width).
    /// Default `DEFAULT_FALLOFF_RADIUS_PIXELS = 32` (matches
    /// `RegionalArchetypeMask::DEFAULT_FALLOFF_RADIUS_PIXELS`).
    pub falloff_radius_pixels: u32,
    /// Currently selected archetype for paint operations.
    /// Default: Continental Temperate.
    pub selected_archetype: WorldArchetypeId,
    /// Paint mode: Paint (write archetype ID) or Erase (write 0).
    /// Default: Paint.
    pub paint_mode: PaintMode,
    /// Whether the panel is active (capturing pointer events for paint
    /// operations). Set during click+drag; cleared on release.
    pub paint_active: bool,
    /// Pending paint operations queue. Operations accumulate during a
    /// frame's pointer events and apply at end-of-frame via
    /// [`Self::apply_pending_paint_ops`] (F.5-paint.B).
    pub pending_paint_ops: Vec<PaintOp>,
}

impl Default for RegionalArchetypePanel {
    fn default() -> Self {
        Self {
            brush_size_pixels: Self::DEFAULT_BRUSH_SIZE_PIXELS,
            falloff_radius_pixels: Self::DEFAULT_FALLOFF_RADIUS_PIXELS,
            selected_archetype: WorldArchetypeId::ContinentalTemperate,
            paint_mode: PaintMode::Paint,
            paint_active: false,
            pending_paint_ops: Vec::new(),
        }
    }
}

impl RegionalArchetypePanel {
    /// Default brush size: 32 mask pixels (≈352 WU at 1024² mask + 11264
    /// WU world extent).
    pub const DEFAULT_BRUSH_SIZE_PIXELS: u32 = 32;
    /// Default falloff radius: 32 mask pixels (matches
    /// `RegionalArchetypeMask::DEFAULT_FALLOFF_RADIUS_PIXELS`).
    pub const DEFAULT_FALLOFF_RADIUS_PIXELS: u32 = 32;
    /// Brush size slider range.
    pub const BRUSH_SIZE_MIN: u32 = 8;
    pub const BRUSH_SIZE_MAX: u32 = 256;
    /// Falloff radius slider range.
    pub const FALLOFF_RADIUS_MIN: u32 = 8;
    pub const FALLOFF_RADIUS_MAX: u32 = 128;

    pub fn new() -> Self {
        Self::default()
    }
}

impl Panel for RegionalArchetypePanel {
    fn name(&self) -> &str {
        "Regional Archetypes"
    }

    fn show(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.show_brush_section(ui);
            ui.separator();
            self.show_palette_section(ui);
            ui.separator();
            self.show_persistence_section(ui);
            ui.separator();
            self.show_regenerate_section(ui);
        });
    }
}

impl RegionalArchetypePanel {
    /// Brush size + falloff distance sliders.
    fn show_brush_section(&mut self, ui: &mut Ui) {
        ui.heading("Brush");
        ui.add(
            egui::Slider::new(
                &mut self.brush_size_pixels,
                Self::BRUSH_SIZE_MIN..=Self::BRUSH_SIZE_MAX,
            )
            .text("Brush Size (pixels)"),
        );
        ui.add(
            egui::Slider::new(
                &mut self.falloff_radius_pixels,
                Self::FALLOFF_RADIUS_MIN..=Self::FALLOFF_RADIUS_MAX,
            )
            .text("Falloff Distance (pixels)"),
        );
    }

    /// Archetype palette dropdown + paint/erase toggle.
    fn show_palette_section(&mut self, ui: &mut Ui) {
        ui.heading("Archetype Palette");
        // Dropdown showing the 6 catalog archetypes with display colors.
        egui::ComboBox::from_label("Selected Archetype")
            .selected_text(self.selected_archetype.display_name())
            .show_ui(ui, |ui| {
                for &id in WorldArchetypeId::all() {
                    let color = archetype_display_color(id);
                    // Color swatch + display name in a horizontal layout.
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(16.0, 16.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 2.0, color);
                        ui.selectable_value(
                            &mut self.selected_archetype,
                            id,
                            id.display_name(),
                        );
                    });
                }
            });

        ui.horizontal(|ui| {
            ui.label("Paint Mode:");
            ui.radio_value(&mut self.paint_mode, PaintMode::Paint, "Paint");
            ui.radio_value(&mut self.paint_mode, PaintMode::Erase, "Erase");
        });
    }

    /// Save/Load/Clear mask buttons. F.5-paint.A scaffold; F.5-paint.C
    /// wires to the project sibling-directory save flow.
    fn show_persistence_section(&mut self, ui: &mut Ui) {
        ui.heading("Persistence");
        ui.horizontal(|ui| {
            // Buttons render but don't act yet at F.5-paint.A scope.
            // F.5-paint.C wires save/load.
            let _ = ui.button("Save Mask");
            let _ = ui.button("Load Mask");
            let _ = ui.button("Clear Mask");
        });
        ui.label(
            egui::RichText::new(
                "Save/Load wiring lands in F.5-paint.C. Currently buttons \
                 render but don't act.",
            )
            .small()
            .italics()
            .color(egui::Color32::GRAY),
        );
    }

    /// Regenerate Terrain button. Triggers `TerrainPanel`'s regenerate
    /// flow after mask changes (F.4.E's `WorldGenerator` integration
    /// reads from `regional_archetype_mask` field).
    fn show_regenerate_section(&mut self, ui: &mut Ui) {
        ui.heading("Apply Changes");
        let _ = ui.button("Regenerate Terrain");
        ui.label(
            egui::RichText::new(
                "Regenerate triggers re-running terrain generation with \
                 the current mask. F.5-paint.C wires the click handler.",
            )
            .small()
            .italics()
            .color(egui::Color32::GRAY),
        );
    }
}

// =============================================================================
// Phase 1.X-F.5-paint.A: archetype display color palette
// =============================================================================

/// Stable display color for each [`WorldArchetypeId`]. Used by the panel
/// palette dropdown's color swatches and (future, F.5-overlay-and-gate)
/// the Climate Preview overlay's archetype-ID overlay path.
///
/// **Stability invariant**: same archetype always returns same color.
/// Pairwise distinct: any two archetypes have L1 color distance > 50.
/// Verified by F.5-paint.A tests `archetype_display_colors_distinct`
/// and `archetype_display_colors_stable`.
pub fn archetype_display_color(id: WorldArchetypeId) -> egui::Color32 {
    match id {
        // Continental Temperate: forest green (NC/Appalachia analog).
        WorldArchetypeId::ContinentalTemperate => egui::Color32::from_rgb(85, 140, 60),
        // Equatorial Tropical: dark green (rainforest canopy).
        WorldArchetypeId::EquatorialTropical => egui::Color32::from_rgb(40, 110, 50),
        // Boreal/Subarctic: blue-white (snow + tundra).
        WorldArchetypeId::BorealSubarctic => egui::Color32::from_rgb(180, 200, 220),
        // Mediterranean: sandy brown (warm, dry).
        WorldArchetypeId::Mediterranean => egui::Color32::from_rgb(180, 150, 100),
        // Desert: orange (subtropical desert).
        WorldArchetypeId::Desert => egui::Color32::from_rgb(220, 170, 100),
        // Custom: neutral gray (advanced; user-tunable).
        WorldArchetypeId::Custom => egui::Color32::from_rgb(150, 150, 150),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default panel state per F.5-paint.A scaffold spec.
    #[test]
    fn panel_default_state() {
        let p = RegionalArchetypePanel::default();
        assert_eq!(p.brush_size_pixels, RegionalArchetypePanel::DEFAULT_BRUSH_SIZE_PIXELS);
        assert_eq!(p.falloff_radius_pixels, RegionalArchetypePanel::DEFAULT_FALLOFF_RADIUS_PIXELS);
        assert_eq!(p.selected_archetype, WorldArchetypeId::ContinentalTemperate);
        assert_eq!(p.paint_mode, PaintMode::Paint);
        assert!(!p.paint_active);
        assert!(p.pending_paint_ops.is_empty());
    }

    /// All 6 archetype display colors are pairwise distinct (L1 distance
    /// > 50). Catches accidental palette collisions.
    #[test]
    fn archetype_display_colors_distinct() {
        let archetypes = WorldArchetypeId::all();
        for (i, &a) in archetypes.iter().enumerate() {
            for &b in archetypes.iter().skip(i + 1) {
                let ca = archetype_display_color(a);
                let cb = archetype_display_color(b);
                let l1 = (ca.r() as i32 - cb.r() as i32).unsigned_abs()
                    + (ca.g() as i32 - cb.g() as i32).unsigned_abs()
                    + (ca.b() as i32 - cb.b() as i32).unsigned_abs();
                assert!(
                    l1 > 50,
                    "archetype colors {:?} and {:?} too similar (L1={}); \
                     palette collision",
                    a,
                    b,
                    l1
                );
            }
        }
    }

    /// Calling `archetype_display_color` twice for the same archetype
    /// returns identical Color32 values. Catches accidental
    /// non-determinism.
    #[test]
    fn archetype_display_colors_stable() {
        for &id in WorldArchetypeId::all() {
            let c1 = archetype_display_color(id);
            let c2 = archetype_display_color(id);
            assert_eq!(c1, c2, "{:?} display color not stable", id);
        }
    }

    /// `pending_paint_ops` starts empty on default construction.
    #[test]
    fn paint_op_queue_starts_empty() {
        let p = RegionalArchetypePanel::default();
        assert_eq!(p.pending_paint_ops.len(), 0);
    }

    /// Default paint mode is `Paint`, not `Erase`.
    #[test]
    fn paint_mode_paint_default() {
        let p = RegionalArchetypePanel::default();
        assert_eq!(p.paint_mode, PaintMode::Paint);
    }

    /// Panel implements the `Panel` trait with name "Regional Archetypes".
    #[test]
    fn panel_trait_name() {
        let p = RegionalArchetypePanel::default();
        assert_eq!(p.name(), "Regional Archetypes");
    }

    /// Constants match `RegionalArchetypeMask` defaults so paint UX
    /// produces masks that match runtime expectations.
    #[test]
    fn defaults_match_regional_archetype_mask_defaults() {
        assert_eq!(
            RegionalArchetypePanel::DEFAULT_FALLOFF_RADIUS_PIXELS,
            RegionalArchetypeMask::DEFAULT_FALLOFF_RADIUS_PIXELS
        );
    }
}
