//! Window edges turned into one [`TickInput`](crate::sim::TickInput).
//!
//! The fixed-tick sim never reads winit. Look deltas come from raw mouse
//! motion, and only while the cursor is captured. Held keys stay in the
//! input manager so a missed frame still moves on the next tick.

use astraweave_input::{Action, Binding, BindingSet, InputContext, InputManager};
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::CursorGrabMode;

use crate::motor::MotorInput;
use crate::sim::TickInput;

/// What the window should do after an input-state change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputTransition {
    Captured,
    Released,
    /// Escape while the cursor is already free.
    Quit,
}

pub struct PlayInput {
    input: InputManager,
    /// The player asked for a grabbed cursor.
    wants_capture: bool,
    /// The OS grab succeeded and look deltas are accepted.
    captured: bool,
    focused: bool,
    look_dx: f32,
    look_dy: f32,
    walk_held: bool,
}

impl PlayInput {
    pub fn new() -> Self {
        Self {
            input: demo_bindings(),
            wants_capture: true,
            captured: false,
            focused: false,
            look_dx: 0.0,
            look_dy: 0.0,
            walk_held: false,
        }
    }

    pub fn wants_capture(&self) -> bool {
        self.wants_capture
    }

    pub fn captured(&self) -> bool {
        self.captured
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn set_captured(&mut self, captured: bool) {
        self.captured = captured;
        if captured {
            self.wants_capture = true;
        }
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn is_down(&self, action: Action) -> bool {
        self.input.is_down(action)
    }

    pub fn just_pressed(&self, action: Action) -> bool {
        self.input.just_pressed(action)
    }

    pub fn clear_frame(&mut self) {
        self.input.clear_frame();
    }

    pub fn walk_held(&self) -> bool {
        self.walk_held
    }

    /// Grab modes to try, in order. Windows rejects `Locked` and can leave
    /// the cursor hidden, so it only tries `Confined`.
    pub fn grab_attempts() -> &'static [CursorGrabMode] {
        if cfg!(windows) {
            &[CursorGrabMode::Confined]
        } else {
            &[CursorGrabMode::Locked, CursorGrabMode::Confined]
        }
    }

    pub fn on_focus(&mut self, focused: bool) -> Option<InputTransition> {
        self.focused = focused;
        if focused {
            return None;
        }
        if self.captured || self.wants_capture {
            self.captured = false;
            self.wants_capture = false;
            self.look_dx = 0.0;
            self.look_dy = 0.0;
            return Some(InputTransition::Released);
        }
        None
    }

    /// Escape releases a captured cursor. A second press asks the window to quit.
    pub fn on_escape(&mut self) -> InputTransition {
        if self.captured || self.wants_capture {
            self.captured = false;
            self.wants_capture = false;
            self.look_dx = 0.0;
            self.look_dy = 0.0;
            InputTransition::Released
        } else {
            InputTransition::Quit
        }
    }

    /// A click asks to grab again. The window applies the OS grab.
    pub fn on_click_capture(&mut self) -> InputTransition {
        self.wants_capture = true;
        InputTransition::Captured
    }

    pub fn on_physical_key(&mut self, code: KeyCode, pressed: bool) {
        if matches!(code, KeyCode::AltLeft | KeyCode::AltRight) {
            self.walk_held = pressed;
        }
        self.input.set_physical_key(code, pressed);
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state,
                    ..
                },
            ..
        } = event
        {
            self.on_physical_key(*code, *state == ElementState::Pressed);
            return;
        }
        self.input.process_window_event(event);
    }

    /// Raw motion. Cursor-moved positions are not look input.
    pub fn on_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if self.captured && self.focused {
                self.look_dx += *dx as f32;
                self.look_dy += *dy as f32;
            }
        }
    }

    pub fn take_look(&mut self) -> (f32, f32) {
        let look = (self.look_dx, self.look_dy);
        self.look_dx = 0.0;
        self.look_dy = 0.0;
        look
    }

    /// One frame of held keys and the look accumulated since the last call.
    pub fn sample_tick(
        &mut self,
        lock_on: bool,
        dig: bool,
        place: bool,
        place_material: u16,
        attack: bool,
        respawn: bool,
    ) -> TickInput {
        let (move_x, move_y) = (
            axis(
                self.is_down(Action::MoveRight),
                self.is_down(Action::MoveLeft),
            ),
            axis(
                self.is_down(Action::MoveForward),
                self.is_down(Action::MoveBackward),
            ),
        );
        let (look_dx, look_dy) = self.take_look();
        TickInput {
            motor: MotorInput {
                move_x,
                move_y,
                jump: self.just_pressed(Action::Jump),
                sprint: self.is_down(Action::Sprint),
                crouch: self.is_down(Action::Crouch),
                roll: self.just_pressed(Action::Ability1),
                fly_toggle: self.just_pressed(Action::Ability2),
                walk: self.walk_held,
                feet_locked: false,
            },
            attack,
            block: self.is_down(Action::AttackHeavy),
            lock_on,
            dig,
            place,
            place_material,
            look_dx,
            look_dy,
            respawn,
        }
    }
}

impl Default for PlayInput {
    fn default() -> Self {
        Self::new()
    }
}

fn axis(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn demo_bindings() -> InputManager {
    let mut bindings = BindingSet::default();
    bindings.actions.insert(
        Action::Ability1,
        Binding {
            key: Some(KeyCode::KeyQ),
            ..Binding::default()
        },
    );
    bindings.actions.insert(
        Action::Ability2,
        Binding {
            key: Some(KeyCode::KeyF),
            ..Binding::default()
        },
    );
    InputManager::new(InputContext::Gameplay, bindings)
}

pub fn mouse_button_pressed(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{Greybox, FIXED_DT};

    fn press(play: &mut PlayInput, code: KeyCode) {
        play.on_physical_key(code, true);
    }

    #[test]
    fn windows_grab_does_not_try_locked_first() {
        let modes = PlayInput::grab_attempts();
        if cfg!(windows) {
            assert_eq!(modes, &[CursorGrabMode::Confined]);
        } else {
            assert_eq!(modes, &[CursorGrabMode::Locked, CursorGrabMode::Confined]);
            assert_ne!(modes[0], CursorGrabMode::Confined);
        }
    }

    #[test]
    fn synthetic_key_and_mouse_move_the_body_and_turn_the_camera() {
        let mut sim = Greybox::open(0);
        let mut play = PlayInput::new();
        play.set_focused(true);
        play.set_captured(true);
        let start = sim.player_pos();
        let yaw = sim.camera.yaw;
        press(&mut play, KeyCode::KeyW);
        play.on_device_event(&DeviceEvent::MouseMotion {
            delta: (80.0, -5.0),
        });
        let sampled = play.sample_tick(false, false, false, 1, false, false);
        assert!(sampled.motor.move_y > 0.0, "W did not feed move_y");
        assert!(sampled.look_dx > 0.0, "raw motion did not feed look");
        let mut latch = crate::sim::FrameLatch::default();
        latch.sample(sampled);
        let caught = latch.catch_up(&mut sim, FIXED_DT * 8.0);
        assert!(caught.steps >= 1);
        let moved = (sim.player_pos() - start).length();
        assert!(moved > 0.05, "player did not move, delta {moved}");
        assert!(
            (sim.camera.yaw - yaw).abs() > 0.01,
            "camera yaw {} stayed at {yaw}",
            sim.camera.yaw
        );
    }

    #[test]
    fn released_mouse_does_not_turn_the_camera_and_escape_releases() {
        let mut play = PlayInput::new();
        play.set_focused(true);
        play.set_captured(true);
        assert_eq!(play.on_escape(), InputTransition::Released);
        assert!(!play.captured());
        play.on_device_event(&DeviceEvent::MouseMotion { delta: (40.0, 0.0) });
        let sampled = play.sample_tick(false, false, false, 1, false, false);
        assert_eq!(sampled.look_dx, 0.0);
        assert_eq!(play.on_escape(), InputTransition::Quit);
        assert_eq!(play.on_click_capture(), InputTransition::Captured);
        assert!(play.wants_capture());
        play.set_captured(true);
        assert_eq!(play.on_focus(false), Some(InputTransition::Released));
        assert!(!play.captured());
        assert!(!play.focused());
    }
}
