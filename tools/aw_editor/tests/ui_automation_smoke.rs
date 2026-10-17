//! Headless egui frames for the viewport toolbar.
//!
//! Clicks are synthetic `RawInput` pointer events at text rects painted on an
//! earlier frame. `egui_kittest` is not used: it forces AccessKit and zbus 5.

#[path = "common/egui_click.rs"]
mod egui_click;

use aw_editor_lib::viewport::camera::OrbitCamera;
use aw_editor_lib::viewport::toolbar::{ShadingMode, ViewportToolbar};
use egui_click::{painted_text, text_rect, UiFrame};

fn paint(ctx: &egui::Context, toolbar: &mut ViewportToolbar, camera: &mut OrbitCamera) {
    egui::CentralPanel::default().show(ctx, |ui| {
        toolbar.ui(ui, ui.max_rect(), camera);
    });
}

fn require_text(shapes: &[egui::epaint::ClippedShape], label: &str) -> egui::Rect {
    text_rect(shapes, |text| text == label)
        .unwrap_or_else(|| panic!("missing {label:?}. painted: {:?}", painted_text(shapes)))
}

/// A point on the painted glyphs. The center of a long label can land in the
/// gap beside the control; the left edge of the glyphs stays on the widget.
fn on_label(rect: egui::Rect) -> egui::Pos2 {
    rect.left_center() + egui::vec2(2.0, 0.0)
}

#[test]
fn toolbar_frame_keeps_the_default_grid_and_lit_shading() {
    let mut toolbar = ViewportToolbar::default();
    let mut camera = OrbitCamera::default();
    assert!(toolbar.show_grid);
    assert_eq!(toolbar.shading_mode, ShadingMode::Lit);
    assert!(!toolbar.snap_enabled);
    let mut frame = UiFrame::new(1280.0, 800.0);
    frame.paint(|ctx| paint(ctx, &mut toolbar, &mut camera));
    assert!(toolbar.show_grid);
    assert_eq!(toolbar.shading_mode, ShadingMode::Lit);
    assert!(!toolbar.snap_enabled);
}

#[test]
fn toolbar_frame_keeps_an_enabled_snap_size() {
    let mut toolbar = ViewportToolbar::default();
    let mut camera = OrbitCamera::default();
    toolbar.snap_enabled = true;
    toolbar.snap_size = 2.0;
    let mut frame = UiFrame::new(1280.0, 800.0);
    frame.paint(|ctx| paint(ctx, &mut toolbar, &mut camera));
    assert!(toolbar.snap_enabled);
    assert_eq!(toolbar.snap_size, 2.0);
}

#[test]
fn grid_checkbox_toggles_off_and_back_on() {
    let mut toolbar = ViewportToolbar::default();
    let mut camera = OrbitCamera::default();
    let mut frame = UiFrame::new(1280.0, 800.0);
    let shapes = frame.paint(|ctx| paint(ctx, &mut toolbar, &mut camera));
    let grid = on_label(require_text(&shapes, "Grid"));
    frame.click(grid, |ctx| paint(ctx, &mut toolbar, &mut camera));
    assert!(!toolbar.show_grid);
    let shapes = frame.paint(|ctx| paint(ctx, &mut toolbar, &mut camera));
    let grid = on_label(require_text(&shapes, "Grid"));
    frame.click(grid, |ctx| paint(ctx, &mut toolbar, &mut camera));
    assert!(toolbar.show_grid);
}

#[test]
fn shading_combo_selects_wireframe() {
    let mut toolbar = ViewportToolbar::default();
    let mut camera = OrbitCamera::default();
    assert!(toolbar.wireframe_supported);
    let mut frame = UiFrame::new(1280.0, 800.0);
    let shapes = frame.paint(|ctx| paint(ctx, &mut toolbar, &mut camera));
    let lit = on_label(require_text(&shapes, "Lit"));
    frame.click(lit, |ctx| paint(ctx, &mut toolbar, &mut camera));
    // The popup is a new area: its labels appear on the frame after it opens.
    let shapes = frame.paint(|ctx| paint(ctx, &mut toolbar, &mut camera));
    let wireframe = on_label(require_text(&shapes, "Wireframe"));
    frame.click(wireframe, |ctx| paint(ctx, &mut toolbar, &mut camera));
    assert_eq!(toolbar.shading_mode, ShadingMode::Wireframe);
}

#[test]
fn grid_snap_size_button_sets_two_meters() {
    let mut toolbar = ViewportToolbar::default();
    let mut camera = OrbitCamera::default();
    let mut frame = UiFrame::new(1280.0, 800.0);
    let shapes = frame.paint(|ctx| paint(ctx, &mut toolbar, &mut camera));
    let snap = on_label(require_text(&shapes, "Grid Snap"));
    let shapes = frame.click(snap, |ctx| paint(ctx, &mut toolbar, &mut camera));
    assert!(toolbar.snap_enabled);
    let two = on_label(require_text(&shapes, "2.0"));
    frame.click(two, |ctx| paint(ctx, &mut toolbar, &mut camera));
    assert_eq!(toolbar.snap_size, 2.0);
}
