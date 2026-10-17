//! Headless pointer clicks for editor egui widgets.
//!
//! `egui_kittest` is not a dependency: it forces AccessKit, which pulls zbus 5.
//! A persistent `egui::Context` still hit-tests the previous frame's widget rects.
//! These helpers record painted text rects, then send `RawInput` press and release
//! events at those rects on later frames.

use egui::epaint::ClippedShape;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};

pub struct UiFrame {
    ctx: Context,
    screen: Rect,
    step: u64,
}

impl UiFrame {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            ctx: Context::default(),
            screen: Rect::from_min_size(Pos2::ZERO, egui::vec2(width, height)),
            step: 0,
        }
    }

    pub fn show(&mut self, events: Vec<Event>, mut ui: impl FnMut(&Context)) -> Vec<ClippedShape> {
        self.step += 1;
        let input = RawInput {
            screen_rect: Some(self.screen),
            time: Some(self.step as f64 / 60.0),
            events,
            ..Default::default()
        };
        self.ctx.run(input, |ctx| ui(ctx)).shapes
    }

    /// Paint a frame, and one more if the first only sized deferred areas.
    ///
    /// `egui::Area` (the viewport toolbar) emits no text shapes until the
    /// second pass has a known size.
    pub fn paint(&mut self, mut ui: impl FnMut(&Context)) -> Vec<ClippedShape> {
        let shapes = self.show(Vec::new(), &mut ui);
        if painted_text(&shapes).is_empty() {
            self.show(Vec::new(), ui)
        } else {
            shapes
        }
    }

    /// Hover, press, and release at `pos` across three frames.
    ///
    /// Hit testing uses the previous frame's rects, so the widget must already
    /// have been painted. The returned shapes are from the release frame.
    pub fn click(&mut self, pos: Pos2, mut ui: impl FnMut(&Context)) -> Vec<ClippedShape> {
        self.show(vec![Event::PointerMoved(pos)], &mut ui);
        self.show(
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
            ],
            &mut ui,
        );
        self.show(
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                },
            ],
            ui,
        )
    }
}

pub fn text_rect(shapes: &[ClippedShape], label: impl Fn(&str) -> bool) -> Option<Rect> {
    let mut found = Vec::new();
    for clipped in shapes {
        collect_text(&clipped.shape, &mut found);
    }
    found
        .into_iter()
        .find(|(text, _)| label(text))
        .map(|(_, rect)| rect)
}

pub fn painted_text(shapes: &[ClippedShape]) -> Vec<String> {
    let mut found = Vec::new();
    for clipped in shapes {
        collect_text(&clipped.shape, &mut found);
    }
    found.into_iter().map(|(text, _)| text).collect()
}

fn collect_text(shape: &Shape, out: &mut Vec<(String, Rect)>) {
    match shape {
        Shape::Text(text) => {
            out.push((text.galley.text().to_owned(), text.visual_bounding_rect()));
        }
        Shape::Vec(shapes) => {
            for shape in shapes {
                collect_text(shape, out);
            }
        }
        _ => {}
    }
}
