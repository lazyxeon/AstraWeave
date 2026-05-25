use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};
use astraweave_cinematics as awc;
use astraweave_gameplay::cutscenes::*;
use astraweave_render::Renderer;
use glam::{vec3, Vec2, Vec3};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

struct CutsceneApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    camera: Camera,
    /// Retained for interactive camera control outside cinematic playback.
    /// During cinematic playback (C.7.B post-rewrite), `Renderer::tick_cinematics`
    /// is the sole camera driver; this controller is not invoked. The
    /// `process_keyboard`/`process_mouse_*` event handlers still feed it
    /// for future post-cinematic interactive phases.
    ctl: CameraController,
    /// Gameplay timeline (post-C.7.B: contains only `Cue::Title` and
    /// `Cue::Wait` cues). Camera cues moved to the renderer's
    /// `awc::Timeline` loaded at startup. Total duration matches the
    /// camera timeline (see the construction site for the
    /// duration-matching constraint).
    gameplay_timeline: Timeline,
    /// Tracks the gameplay timeline's state machine (Title events). The
    /// camera timeline is owned internally by the renderer
    /// (`cin_tl`/`cin_seq`).
    cs: CutsceneState,
    /// Buffered timeline + initial state for deferred renderer-side load.
    /// `resumed()` constructs the renderer; the awc::Timeline can only
    /// be loaded once the renderer exists, so it is staged here.
    pending_awc_timeline: Option<awc::Timeline>,
    t: f32,
    last: Instant,
}

impl CutsceneApp {
    fn new() -> Self {
        let camera = Camera {
            position: vec3(-3.0, 5.0, 10.0),
            yaw: -1.57,
            pitch: -0.4,
            fovy: 60f32.to_radians(),
            aspect: 16.0 / 9.0,
            znear: 0.1,
            zfar: 500.0,
        };
        let ctl = CameraController::new(10.0, 0.005);

        // C.7.B (Unified Camera campaign): the demo's camera state is
        // now driven by `Renderer::tick_cinematics` consuming an
        // `awc::Timeline` (the canonical cinematics path). The C.7.A
        // bridge — inline `apply_camera_key`-equivalent conversion at
        // the demo site — retires; the demo becomes the campaign's
        // first production caller of `tick_cinematics`, closing L.7.1.
        //
        // The two parallel state machines (camera via awc::Timeline,
        // Title via gameplay Timeline) run on matched-duration timelines
        // synchronized at construction time. Total duration: 6.0s,
        // matching pre-C.7.B behavior (1.5s Title + 0.5s Wait + 2.0s
        // first CameraTo + 2.0s second CameraTo = 6.0s).
        //
        // Helper for spherical-to-cartesian forward direction (canonical
        // per `FreeFly::dir` at `astraweave-camera/src/freefly.rs:55-62`).
        let forward = |yaw: f32, pitch: f32| -> Vec3 {
            Vec3::new(
                yaw.cos() * pitch.cos(),
                pitch.sin(),
                yaw.sin() * pitch.cos(),
            )
        };
        let pos1 = vec3(0.0, 6.0, 12.0);
        let pos2 = vec3(2.0, 4.0, 8.0);
        let look_at1 = pos1 + forward(-1.57, -0.35);
        let look_at2 = pos2 + forward(-1.40, -0.45);

        // awc::Timeline (camera state machine). Keyframes emit on
        // sequencer-step boundaries: at t=2.0 the camera snaps to pos1;
        // at t=4.0 it snaps to pos2. Matches pre-C.7.B's snap-on-cue-
        // start behavior (the pre-C.7.B CameraTo cue emitted its
        // destination every tick; apply_camera_key snapped on the first
        // tick and stayed).
        let mut awc_timeline = awc::Timeline::new("cutscene_demo", 6.0);
        awc_timeline.add_camera_track(vec![
            awc::CameraKey::new(
                awc::Time(2.0),
                (pos1.x, pos1.y, pos1.z),
                (look_at1.x, look_at1.y, look_at1.z),
                60.0,
            ),
            awc::CameraKey::new(
                awc::Time(4.0),
                (pos2.x, pos2.y, pos2.z),
                (look_at2.x, look_at2.y, look_at2.z),
                60.0,
            ),
        ]);

        // Gameplay Timeline (Title state machine). Contains only
        // `Cue::Title` and `Cue::Wait` cues post-C.7.B; the camera cues
        // moved to `awc_timeline` above. The `Cue::Wait { time: 4.5 }`
        // padding matches the awc::Timeline's remaining duration after
        // the Title cue completes (1.5s + 4.5s = 6.0s total), keeping
        // the two parallel state machines synchronized. If the
        // awc::Timeline's duration changes, this padding must match.
        let gameplay_timeline = Timeline {
            cues: vec![
                Cue::Title {
                    text: "Veilweaver".into(),
                    time: 1.5,
                },
                Cue::Wait { time: 4.5 },
            ],
        };
        let cs = CutsceneState::new();

        Self {
            window: None,
            renderer: None,
            camera,
            ctl,
            gameplay_timeline,
            cs,
            pending_awc_timeline: Some(awc_timeline),
            t: 0.0,
            last: Instant::now(),
        }
    }
}

impl ApplicationHandler for CutsceneApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Cutscene Demo")
                .with_inner_size(PhysicalSize::new(1280, 720));
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());
            let mut renderer = pollster::block_on(Renderer::new(window.clone())).unwrap();

            // C.7.B: load + play the awc::Timeline staged in `new()`.
            // The programmatic `Renderer::load_timeline` API was added in
            // the same sub-phase as a counterpart to `load_timeline_json`
            // (the demo doesn't need disk serialization to bootstrap).
            if let Some(awc_timeline) = self.pending_awc_timeline.take() {
                renderer.load_timeline(awc_timeline);
                renderer.play_timeline();
            }

            self.renderer = Some(renderer);
            self.last = Instant::now();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let renderer = match self.renderer.as_mut() {
            Some(r) => r,
            None => return,
        };
        let _window = match self.window.as_ref() {
            Some(w) => w,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => {
                renderer.resize(s.width, s.height);
                self.camera.aspect = s.width as f32 / s.height.max(1) as f32;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(code),
                        ..
                    },
                ..
            } => {
                self.ctl
                    .process_keyboard(code, state == ElementState::Pressed);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    self.ctl
                        .process_mouse_button(MouseButton::Right, state == ElementState::Pressed);
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.ctl.process_mouse_move(
                &mut self.camera,
                Vec2::new(position.x as f32, position.y as f32),
            ),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let renderer = match self.renderer.as_mut() {
            Some(r) => r,
            None => return,
        };
        let window = match self.window.as_ref() {
            Some(w) => w,
            None => return,
        };

        let dt = (Instant::now() - self.last).as_secs_f32();
        self.last = Instant::now();
        self.t += dt;

        // C.7.B canonical cinematics path: camera state driven by the
        // renderer's `tick_cinematics` consuming the awc::Timeline
        // loaded in `resumed()`. The demo is the first production
        // caller of `tick_cinematics`, closing L.7.1 (the audit finding
        // that the canonical path had zero production callers). The
        // C.7.A bridge — inline `apply_camera_key`-equivalent conversion
        // at the demo site — retired with this rewrite.
        let _cinematics_events = renderer.tick_cinematics(dt, &mut self.camera);

        // Gameplay state machine: Title events polled from the parallel
        // gameplay Timeline. Post-C.7.B the gameplay Timeline contains
        // only `Cue::Title` and `Cue::Wait` cues; camera cues moved to
        // the awc::Timeline. The `unreachable!()` arm is the structural-
        // correctness guard: if a future edit accidentally adds a
        // `Cue::CameraTo` to the gameplay Timeline, it fires at runtime
        // surfacing the divergence (better than silent dual-write).
        match self.cs.tick(dt, &self.gameplay_timeline) {
            CutsceneTickEvent::Title(_text) => {
                // Title display is silent in this demo (pre-C.7.B the
                // Title fell through to the controller fallback;
                // post-C.7.B no UI text rendering is wired). Consuming
                // the event keeps the gameplay state machine advancing;
                // wiring the display surface is out of campaign scope.
            }
            CutsceneTickEvent::Camera(_) => {
                unreachable!(
                    "Gameplay Timeline should contain only Title/Wait cues post-C.7.B; \
                     camera cues live in the awc::Timeline loaded into the renderer."
                );
            }
            CutsceneTickEvent::Continue | CutsceneTickEvent::Done => {
                // No-op: camera is driven by `tick_cinematics`; no
                // controller fallback needed during cinematic playback.
            }
        }

        renderer.update_view(&self.camera.to_render_view());
        if let Err(e) = renderer.render() {
            eprintln!("{e:?}");
        }
        window.request_redraw();
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = CutsceneApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
