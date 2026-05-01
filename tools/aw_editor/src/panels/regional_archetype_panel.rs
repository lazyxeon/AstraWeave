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

    /// Phase 1.X-F.5-paint.B: queue a paint operation at world coordinates
    /// `(world_x, world_z)`. Operation is appended to `pending_paint_ops`
    /// and applied by [`Self::apply_pending_paint_ops`] at end-of-frame.
    ///
    /// Uses the panel's current `brush_size_pixels`, `selected_archetype`,
    /// and `paint_mode` state. In Erase mode, writes archetype ID 0
    /// (unpainted; sample-time fallback to Continental Temperate).
    pub fn queue_paint_op(&mut self, world_x: f32, world_z: f32) {
        let archetype_id = match self.paint_mode {
            PaintMode::Paint => self.selected_archetype.to_mask_id(),
            PaintMode::Erase => 0,
        };
        self.pending_paint_ops.push(PaintOp {
            world_x,
            world_z,
            brush_size_pixels: self.brush_size_pixels,
            archetype_id,
        });
    }

    /// Phase 1.X-F.5-paint.B: apply queued paint operations to the mask.
    /// Drains `pending_paint_ops`, applies each circular brush stamp to
    /// `mask.ids`, then calls `mask.recompute_falloff()` once at the end
    /// (single batched recompute per F.5-paint prompt §2.2 sync strategy).
    ///
    /// **Operation order discipline** (per F.5-paint prompt §3): operations
    /// applied in queue order; each op's `archetype_id` overwrites prior
    /// ops at the same pixel. This matches user expectation (paint-over
    /// semantics).
    ///
    /// **Falloff recompute trigger**: at end-of-batch only. Skipped if the
    /// queue was empty (no-op fast path). The recompute is sync per
    /// F.5-paint prompt §2.2 rationale (~50-100 ms at 1024²; acceptable
    /// for end-of-paint-stroke timing).
    pub fn apply_pending_paint_ops(&mut self, mask: &mut RegionalArchetypeMask) {
        if self.pending_paint_ops.is_empty() {
            return;
        }

        // Apply each op's circular brush stamp to mask.ids.
        let resolution = mask.resolution;
        let world_extent = mask.world_extent_wu;
        for op in self.pending_paint_ops.drain(..) {
            let (px, pz) = world_to_mask_pixel(op.world_x, op.world_z, resolution, world_extent);
            paint_circle(
                &mut mask.ids,
                resolution,
                px,
                pz,
                op.brush_size_pixels,
                op.archetype_id,
            );
        }

        // Single batched falloff recompute after all ops land.
        mask.recompute_falloff();
    }
}

/// Phase 1.X-F.5-paint.B: convert world `(world_x, world_z)` coordinates
/// to mask pixel `(px, pz)` coordinates given the mask's resolution and
/// world extent.
///
/// World origin (0, 0) maps to the mask's center pixel. World coordinates
/// outside `[-world_extent_wu/2, +world_extent_wu/2]` clamp to the mask's
/// edge pixels (silently; out-of-range paint operations don't panic).
///
/// Returns signed `i32` (not `u32`) so the caller can pass directly to
/// [`paint_circle`], which handles negative coordinates by clipping.
pub fn world_to_mask_pixel(
    world_x: f32,
    world_z: f32,
    resolution: u32,
    world_extent_wu: f32,
) -> (i32, i32) {
    let half_extent = world_extent_wu * 0.5;
    let scale = resolution as f32 / world_extent_wu;
    let px = ((world_x + half_extent) * scale) as i32;
    let pz = ((world_z + half_extent) * scale) as i32;
    (px, pz)
}

/// Phase 1.X-F.5-paint.B: paint a circular region of `archetype_id` into
/// `ids` at center pixel `(cx, cz)` with radius `radius` pixels. Pixels
/// outside the mask's bounds are silently skipped (no panic on
/// out-of-range coords).
///
/// Uses Euclidean-distance test: pixel `(px, pz)` is painted if
/// `(px-cx)² + (pz-cz)² ≤ radius²`. Matches
/// [`RegionalArchetypeMask::with_painted_circle`] semantics from F.4.A.
pub fn paint_circle(ids: &mut [u8], resolution: u32, cx: i32, cz: i32, radius: u32, archetype_id: u8) {
    let r = radius as i32;
    let r2 = r * r;
    let res = resolution as i32;
    let min_x = (cx - r).max(0);
    let max_x = (cx + r).min(res - 1);
    let min_y = (cz - r).max(0);
    let max_y = (cz + r).min(res - 1);
    for pz in min_y..=max_y {
        for px in min_x..=max_x {
            let dx = px - cx;
            let dz = pz - cz;
            if dx * dx + dz * dz <= r2 {
                let idx = (pz as usize) * (resolution as usize) + (px as usize);
                ids[idx] = archetype_id;
            }
        }
    }
}

/// Phase 1.X-F.5-paint.B: project a screen-space pointer position to
/// world `(X, Z)` coordinates via simple Y=0 plane intersection.
///
/// Inputs:
/// - `pointer_screen_normalized`: pointer position normalized to `[-1, +1]`
///   in both axes (NDC-style; `(0, 0)` = screen center).
/// - `camera_world_pos`: camera position in world units (Y is up;
///   `(world_x, camera_y, world_z)`).
/// - `camera_pitch_rad`: camera pitch (rotation around X axis; positive =
///   looking down). Yaw set to 0 (looking down +Z); editor's actual yaw
///   composed by caller if needed.
/// - `camera_fov_y_rad`: camera vertical field of view in radians.
/// - `aspect`: viewport aspect ratio (width / height).
///
/// Returns `Some((world_x, world_z))` if the ray hits the Y=0 plane;
/// `None` if the ray is parallel to or pointing above the plane (e.g.,
/// camera looking at the sky).
///
/// **Simplification per F.5-paint prompt §2.2**: this projects against
/// the Y=0 reference plane, not actual terrain elevation. The writer
/// paints based on what they see at the cursor; for paint UX purposes,
/// hitting the reference plane is sufficient. F.5-overlay-and-gate may
/// refine to use actual heightmap elevation via raycast against the
/// terrain mesh if Andrew-gate surfaces a UX issue.
pub fn screen_to_world_xz_y0(
    pointer_screen_normalized: (f32, f32),
    camera_world_pos: (f32, f32, f32),
    camera_pitch_rad: f32,
    camera_fov_y_rad: f32,
    aspect: f32,
) -> Option<(f32, f32)> {
    // Ray direction in camera space: pointer NDC → view-space ray.
    let half_fov = camera_fov_y_rad * 0.5;
    let tan_half = half_fov.tan();
    let ndc_x = pointer_screen_normalized.0;
    let ndc_y = pointer_screen_normalized.1;
    // Camera-space ray (forward = -Z in camera space, but for this Y=0
    // intersection we treat camera-space forward as +Z and let the pitch
    // rotate it). Use right-handed convention: x_view = ndc_x * tan_half *
    // aspect; y_view = ndc_y * tan_half (positive ndc_y = up); z_view = 1.
    let dir_x = ndc_x * tan_half * aspect;
    let dir_y = ndc_y * tan_half;
    let dir_z = 1.0_f32;

    // Apply camera pitch (rotation around X axis): positive pitch =
    // looking down. The forward direction tilts toward -Y.
    let cos_p = camera_pitch_rad.cos();
    let sin_p = camera_pitch_rad.sin();
    // Rotated dir: y' = dir_y * cos_p - dir_z * sin_p
    //              z' = dir_y * sin_p + dir_z * cos_p
    let world_dir_y = dir_y * cos_p - dir_z * sin_p;
    let world_dir_z = dir_y * sin_p + dir_z * cos_p;
    let world_dir_x = dir_x;

    // Y=0 plane intersection: camera_y + t * world_dir_y = 0 → t = -camera_y / world_dir_y.
    // For valid intersection, t > 0 AND world_dir_y < 0 (ray pointing down).
    let camera_y = camera_world_pos.1;
    if world_dir_y.abs() < 1e-6 || world_dir_y >= 0.0 {
        return None;
    }
    let t = -camera_y / world_dir_y;
    if t <= 0.0 {
        return None;
    }
    let hit_x = camera_world_pos.0 + t * world_dir_x;
    let hit_z = camera_world_pos.2 + t * world_dir_z;
    Some((hit_x, hit_z))
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

    // =========================================================================
    // Phase 1.X-F.5-paint.B: brush + projection + apply tests
    // =========================================================================

    /// `paint_circle` writes pixels within `radius_pixels` of center;
    /// pixels outside don't.
    #[test]
    fn paint_circle_writes_pixels_inside_radius() {
        let mut ids = vec![0u8; 64 * 64];
        paint_circle(&mut ids, 64, 32, 32, 8, 1);

        // Center painted.
        assert_eq!(ids[32 * 64 + 32], 1);
        // Pixels within radius 8.
        assert_eq!(ids[32 * 64 + 40], 1); // dist = 8 (boundary)
        assert_eq!(ids[40 * 64 + 32], 1); // dist = 8
        // Pixels just outside (dist² = 81 > 64).
        assert_eq!(ids[32 * 64 + 41], 0); // dist = 9
        // Far pixels.
        assert_eq!(ids[0], 0);
        assert_eq!(ids[63 * 64 + 63], 0);
    }

    /// `paint_circle` clips out-of-range pixels without panic.
    #[test]
    fn paint_circle_respects_mask_bounds() {
        let mut ids = vec![0u8; 64 * 64];
        // Paint near the corner with a large radius — most of the brush
        // is outside the mask. Should not panic; only valid pixels
        // written.
        paint_circle(&mut ids, 64, 0, 0, 50, 5);
        // Center pixel (0, 0) painted.
        assert_eq!(ids[0], 5);
        // Distant pixel within mask but outside brush → 0.
        assert_eq!(ids[63 * 64 + 63], 0);
    }

    /// World position (0, 0) maps to mask center pixel; world position at
    /// world_extent_wu/2 maps to mask edge.
    #[test]
    fn world_to_mask_pixel_conversion() {
        // 64-pixel mask, 100 WU world extent.
        let (px, pz) = world_to_mask_pixel(0.0, 0.0, 64, 100.0);
        assert_eq!(px, 32);
        assert_eq!(pz, 32);

        // World edge: half-extent positive maps to last pixel.
        let (px, pz) = world_to_mask_pixel(50.0, 50.0, 64, 100.0);
        assert_eq!(px, 64); // out-of-range; signed i32; paint_circle clips.
        assert_eq!(pz, 64);

        // World edge: half-extent negative maps to pixel 0.
        let (px, pz) = world_to_mask_pixel(-50.0, -50.0, 64, 100.0);
        assert_eq!(px, 0);
        assert_eq!(pz, 0);
    }

    /// `apply_pending_paint_ops` updates mask IDs for queued ops and
    /// recomputes falloff once at end-of-batch.
    #[test]
    fn apply_pending_paint_ops_updates_mask_and_recomputes_falloff() {
        let mut mask = RegionalArchetypeMask::new_unpainted(64, 100.0);
        let mut panel = RegionalArchetypePanel::default();
        panel.brush_size_pixels = 8;
        // Queue 3 paint operations at different world positions.
        panel.queue_paint_op(0.0, 0.0); // → mask center (32, 32)
        panel.queue_paint_op(-25.0, -25.0); // → mask (16, 16)
        panel.queue_paint_op(25.0, 25.0); // → mask (48, 48)

        assert_eq!(panel.pending_paint_ops.len(), 3);
        panel.apply_pending_paint_ops(&mut mask);
        assert_eq!(panel.pending_paint_ops.len(), 0); // drained

        // All three painted pixels have the selected archetype's mask ID.
        let ct_id = WorldArchetypeId::ContinentalTemperate.to_mask_id();
        assert_eq!(mask.id_at(32, 32), ct_id);
        assert_eq!(mask.id_at(16, 16), ct_id);
        assert_eq!(mask.id_at(48, 48), ct_id);

        // Falloff recomputed: at least one pixel in the painted regions
        // should have non-default falloff (255 = default unpainted).
        // The paint creates archetype-vs-unpainted boundaries; nearby
        // unpainted pixels should retain 255 (per recompute_falloff
        // semantics: unpainted regions stay 255), but boundary pixels
        // of painted region should have falloff < 255.
        let painted_boundary = mask.falloff_at(40, 32); // edge of CT circle at (32, 32) radius 8
        assert!(
            painted_boundary < 255,
            "painted region's boundary pixel should have falloff < 255 \
             (recomputed); got {}",
            painted_boundary
        );
    }

    /// Empty `pending_paint_ops` queue → no mask mutation, no recompute
    /// call (no-op fast path).
    #[test]
    fn apply_pending_paint_ops_empty_queue_is_noop() {
        let mut mask = RegionalArchetypeMask::new_unpainted(64, 100.0);
        let original_ids = mask.ids.clone();
        let original_falloff = mask.falloff.clone();

        let mut panel = RegionalArchetypePanel::default();
        // Empty queue.
        panel.apply_pending_paint_ops(&mut mask);

        // Mask untouched.
        assert_eq!(mask.ids, original_ids);
        assert_eq!(mask.falloff, original_falloff);
    }

    /// Paint a circle of archetype 1; erase a smaller inner circle; the
    /// inner pixels become 0 (unpainted) while the outer ring stays 1.
    #[test]
    fn paint_then_erase_clears_pixels() {
        let mut mask = RegionalArchetypeMask::new_unpainted(64, 100.0);
        let mut panel = RegionalArchetypePanel::default();
        panel.selected_archetype = WorldArchetypeId::EquatorialTropical;
        panel.brush_size_pixels = 16;

        // Paint outer circle of ET.
        panel.queue_paint_op(0.0, 0.0);
        panel.apply_pending_paint_ops(&mut mask);
        let et_id = WorldArchetypeId::EquatorialTropical.to_mask_id();
        assert_eq!(mask.id_at(32, 32), et_id); // center painted

        // Switch to erase mode and erase inner small circle.
        panel.paint_mode = PaintMode::Erase;
        panel.brush_size_pixels = 4;
        panel.queue_paint_op(0.0, 0.0);
        panel.apply_pending_paint_ops(&mut mask);

        // Inner pixel erased.
        assert_eq!(mask.id_at(32, 32), 0);
        // Outer ring (radius ~10 from center) still ET.
        assert_eq!(mask.id_at(42, 32), et_id); // dist = 10, inside outer (16) outside inner (4)
    }

    /// Camera at (0, 1000, 0) with pitch -π/2 (looking straight down) and
    /// pointer at screen center projects to world origin.
    #[test]
    fn screen_to_world_xz_camera_overhead_returns_origin() {
        let world_xz = screen_to_world_xz_y0(
            (0.0, 0.0), // pointer at screen center
            (0.0, 1000.0, 0.0),
            std::f32::consts::FRAC_PI_2, // 90° down
            1.0,                          // FOV (irrelevant at center)
            16.0 / 9.0,                   // aspect
        );
        let (x, z) = world_xz.expect("ray hits Y=0 plane");
        assert!(x.abs() < 0.01, "world_x at screen center should be 0; got {}", x);
        assert!(z.abs() < 0.01, "world_z at screen center should be 0; got {}", z);
    }

    /// Camera looking up (pitch = 0) → ray parallel to Y=0 plane → no hit.
    #[test]
    fn screen_to_world_xz_camera_horizontal_misses_plane() {
        let world_xz = screen_to_world_xz_y0(
            (0.0, 0.0),
            (0.0, 100.0, 0.0),
            0.0, // looking horizontally (no pitch)
            1.0,
            16.0 / 9.0,
        );
        // Ray points along +Z with no Y component (pitch=0). Doesn't hit
        // Y=0 plane → None.
        assert!(world_xz.is_none(), "horizontal ray should miss Y=0 plane");
    }

    /// `queue_paint_op` populates `pending_paint_ops` with the panel's
    /// current state at queue time.
    #[test]
    fn queue_paint_op_captures_panel_state() {
        let mut panel = RegionalArchetypePanel::default();
        panel.selected_archetype = WorldArchetypeId::Desert;
        panel.brush_size_pixels = 64;
        panel.queue_paint_op(123.0, -456.0);

        assert_eq!(panel.pending_paint_ops.len(), 1);
        let op = &panel.pending_paint_ops[0];
        assert_eq!(op.world_x, 123.0);
        assert_eq!(op.world_z, -456.0);
        assert_eq!(op.brush_size_pixels, 64);
        assert_eq!(op.archetype_id, WorldArchetypeId::Desert.to_mask_id());
    }

    /// `queue_paint_op` in Erase mode writes archetype_id 0 regardless
    /// of `selected_archetype`.
    #[test]
    fn queue_paint_op_in_erase_mode_writes_zero() {
        let mut panel = RegionalArchetypePanel::default();
        panel.selected_archetype = WorldArchetypeId::Desert;
        panel.paint_mode = PaintMode::Erase;
        panel.queue_paint_op(0.0, 0.0);

        assert_eq!(panel.pending_paint_ops.len(), 1);
        let op = &panel.pending_paint_ops[0];
        assert_eq!(op.archetype_id, 0); // erase = 0 regardless of selected_archetype
    }
}
