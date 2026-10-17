//! Headless egui frames for the viewport toolbar.
//!
//! These used to click widgets by label through `egui_kittest`. That crate
//! forces `egui/accesskit`, which pulls `accesskit_unix` and zbus 5
//! (RUSTSEC-2026-0370, 0371, 0372). A plain `egui::Context` can paint the
//! toolbar and check that a frame does not change state on its own. It cannot
//! find a checkbox by its label and click it. The click cases that stay
//! untested until AccessKit is on zbus 6 are:
//!
//! - toggle `Grid` off and back on
//! - open the shading combo and choose Wireframe
//! - enable `Grid Snap` and press the `2.0` size button

use aw_editor_lib::viewport::camera::OrbitCamera;
use aw_editor_lib::viewport::toolbar::{ShadingMode, ViewportToolbar};

fn paint(toolbar: &mut ViewportToolbar) {
    let ctx = egui::Context::default();
    let mut camera = OrbitCamera::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            toolbar.ui(ui, ui.max_rect(), &mut camera);
        });
    });
}

#[test]
fn toolbar_frame_keeps_the_default_grid_and_lit_shading() {
    let mut toolbar = ViewportToolbar::default();
    assert!(toolbar.show_grid);
    assert_eq!(toolbar.shading_mode, ShadingMode::Lit);
    assert!(!toolbar.snap_enabled);
    paint(&mut toolbar);
    assert!(toolbar.show_grid);
    assert_eq!(toolbar.shading_mode, ShadingMode::Lit);
    assert!(!toolbar.snap_enabled);
}

#[test]
fn toolbar_frame_keeps_an_enabled_snap_size() {
    let mut toolbar = ViewportToolbar::default();
    toolbar.snap_enabled = true;
    toolbar.snap_size = 2.0;
    paint(&mut toolbar);
    assert!(toolbar.snap_enabled);
    assert_eq!(toolbar.snap_size, 2.0);
}
