use glam::Vec2;
use std::collections::HashSet;
use winit::event::{ElementState, KeyEvent, Touch, TouchPhase, WindowEvent};
use winit::keyboard::PhysicalKey;

use gilrs::{Axis, Button, Gilrs};

use crate::bindings::{BindingSet, GamepadButton};
use crate::{Action, Axis2, InputContext};

pub struct InputManager {
    pub context: InputContext,
    pub bindings: BindingSet,

    // pressed / just-pressed states
    pressed: HashSet<Action>,
    just_pressed: HashSet<Action>,

    // axes
    pub move_axis: Axis2,
    pub look_axis: Axis2,

    // mouse capture / sensitivity
    pub look_sensitivity: f32,

    // gamepad
    gilrs: Option<Gilrs>,

    // touch (virtual joystick)
    touch_active: bool,
    touch_id: Option<u64>,
    touch_origin: Option<Vec2>,
    touch_current: Option<Vec2>,
}

impl InputManager {
    pub fn new(context: InputContext, bindings: BindingSet) -> Self {
        let gilrs = Gilrs::new().ok();
        Self {
            context,
            bindings,
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            move_axis: Axis2::default(),
            look_axis: Axis2::default(),
            look_sensitivity: 0.12,
            gilrs,
            touch_active: false,
            touch_id: None,
            touch_origin: None,
            touch_current: None,
        }
    }

    pub fn set_context(&mut self, cx: InputContext) {
        self.context = cx;
    }

    #[inline]
    pub fn is_down(&self, a: Action) -> bool {
        self.pressed.contains(&a)
    }
    #[inline]
    pub fn just_pressed(&self, a: Action) -> bool {
        self.just_pressed.contains(&a)
    }

    pub fn clear_frame(&mut self) {
        self.just_pressed.clear();
    }

    /// Apply one physical key. Window events and synthetic tests share this.
    pub fn set_physical_key(&mut self, code: winit::keyboard::KeyCode, pressed: bool) {
        let actions: Vec<_> = self
            .bindings
            .actions
            .iter()
            .filter_map(|(action, binding)| {
                if binding.key == Some(code) {
                    Some(*action)
                } else {
                    None
                }
            })
            .collect();
        for action in actions {
            self.set_action(action, pressed);
        }
    }

    pub fn process_window_event(&mut self, ev: &WindowEvent) {
        match ev {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(code),
                        ..
                    },
                ..
            } => {
                self.set_physical_key(*code, *state == ElementState::Pressed);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let actions: Vec<_> = self
                    .bindings
                    .actions
                    .iter()
                    .filter_map(|(action, b)| {
                        if b.mouse == Some(*button) {
                            Some(*action)
                        } else {
                            None
                        }
                    })
                    .collect();

                for action in actions {
                    self.set_action(action, *state == ElementState::Pressed);
                }
            }
            WindowEvent::Touch(Touch {
                phase,
                id,
                location,
                ..
            }) => match phase {
                TouchPhase::Started => {
                    self.touch_active = true;
                    self.touch_id = Some(*id);
                    self.touch_origin = Some(glam::vec2(location.x as f32, location.y as f32));
                    self.touch_current = self.touch_origin;
                }
                TouchPhase::Moved => {
                    if self.touch_active && self.touch_id == Some(*id) {
                        self.touch_current = Some(glam::vec2(location.x as f32, location.y as f32));
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if self.touch_id == Some(*id) {
                        self.touch_active = false;
                        self.touch_id = None;
                        self.touch_origin = None;
                        self.touch_current = None;
                        self.move_axis = Axis2::default();
                    }
                }
            },
            _ => {}
        }
    }

    pub fn poll_gamepads(&mut self) {
        // Collect events first to avoid borrowing conflicts
        let mut events = Vec::new();
        if let Some(g) = self.gilrs.as_mut() {
            while let Some(ev) = g.next_event() {
                events.push(ev);
            }
        }

        // Process events after collecting them
        for ev in events {
            use gilrs::EventType::*;
            match ev.event {
                ButtonPressed(b, _) => self.handle_button(b, true),
                ButtonReleased(b, _) => self.handle_button(b, false),
                AxisChanged(a, v, _) => self.handle_axis(a, v),
                _ => {}
            }
        }

        // Virtual joystick from touch:
        if let (Some(o), Some(c)) = (self.touch_origin, self.touch_current) {
            let delta = (c - o) / 80.0; // pixels to normalized
            self.move_axis.x = delta.x.clamp(-1.0, 1.0);
            self.move_axis.y = (-delta.y).clamp(-1.0, 1.0);
        }
    }

    fn handle_button(&mut self, b: Button, down: bool) {
        use Button::*;
        let map = |b: Button| -> Option<crate::bindings::GamepadButton> {
            Some(match b {
                South => GamepadButton::South,
                East => GamepadButton::East,
                West => GamepadButton::West,
                North => GamepadButton::North,
                LeftTrigger => GamepadButton::L2,
                RightTrigger => GamepadButton::R2,
                LeftTrigger2 => GamepadButton::L1,
                RightTrigger2 => GamepadButton::R1,
                Select => GamepadButton::Select,
                Start => GamepadButton::Start,
                LeftThumb => GamepadButton::LStick,
                RightThumb => GamepadButton::RStick,
                DPadUp => GamepadButton::DPadUp,
                DPadDown => GamepadButton::DPadDown,
                DPadLeft => GamepadButton::DPadLeft,
                DPadRight => GamepadButton::DPadRight,
                _ => return None,
            })
        };
        if let Some(gb) = map(b) {
            let actions: Vec<_> = self
                .bindings
                .actions
                .iter()
                .filter_map(|(action, bind)| {
                    if bind.gamepad == Some(gb) {
                        Some(*action)
                    } else {
                        None
                    }
                })
                .collect();

            for action in actions {
                self.set_action(action, down);
            }
        }
    }

    fn handle_axis(&mut self, a: Axis, val: f32) {
        use Axis::*;
        let apply = |bind: &crate::bindings::AxisBinding, v: f32| -> f32 {
            let mut t = v;
            if bind.invert {
                t = -t;
            }
            if t.abs() < bind.deadzone {
                0.0
            } else {
                t
            }
        };
        let (mx, my) = &self.bindings.move_axes;
        let (lx, ly) = &self.bindings.look_axes;

        match a {
            LeftStickX => self.move_axis.x = apply(mx, val),
            LeftStickY => self.move_axis.y = apply(my, val),
            RightStickX => self.look_axis.x = apply(lx, val),
            RightStickY => self.look_axis.y = apply(ly, val),
            _ => {}
        }
    }

    fn set_action(&mut self, a: Action, down: bool) {
        if down {
            if !self.pressed.contains(&a) {
                self.just_pressed.insert(a);
            }
            self.pressed.insert(a);
        } else {
            self.pressed.remove(&a);
        }
    }

    // Test-only methods to expose internal state manipulation
    #[cfg(test)]
    pub(crate) fn test_set_action(&mut self, a: Action, down: bool) {
        self.set_action(a, down);
    }

    #[cfg(test)]
    pub(crate) fn test_handle_button(&mut self, b: gilrs::Button, down: bool) {
        self.handle_button(b, down);
    }

    #[cfg(test)]
    pub(crate) fn test_handle_axis(&mut self, a: gilrs::Axis, val: f32) {
        self.handle_axis(a, val);
    }

    #[cfg(test)]
    pub(crate) fn test_set_touch(&mut self, origin: Option<Vec2>, current: Option<Vec2>) {
        self.touch_origin = origin;
        self.touch_current = current;
        self.touch_active = origin.is_some();
    }
}

#[cfg(test)]
mod manager_internal_tests {
    use super::*;
    use crate::bindings::{BindingSet, GamepadButton};
    use crate::{Action, Binding};
    use gilrs::{Axis, Button};
    use glam::Vec2;

    fn default_manager() -> InputManager {
        let bindings = BindingSet::default();
        InputManager::new(InputContext::Gameplay, bindings)
    }

    fn manager_with_gamepad_bindings() -> InputManager {
        let mut bindings = BindingSet::default();
        bindings.actions.insert(
            Action::AttackLight,
            Binding {
                gamepad: Some(GamepadButton::South),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::Jump,
            Binding {
                gamepad: Some(GamepadButton::East),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::Interact,
            Binding {
                gamepad: Some(GamepadButton::West),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::AttackHeavy,
            Binding {
                gamepad: Some(GamepadButton::North),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::Sprint,
            Binding {
                gamepad: Some(GamepadButton::L2),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::Crouch,
            Binding {
                gamepad: Some(GamepadButton::R2),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::Ability1,
            Binding {
                gamepad: Some(GamepadButton::L1),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::Ability2,
            Binding {
                gamepad: Some(GamepadButton::R1),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::OpenMenu,
            Binding {
                gamepad: Some(GamepadButton::Start),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::OpenInventory,
            Binding {
                gamepad: Some(GamepadButton::Select),
                ..Default::default()
            },
        );
        InputManager::new(InputContext::Gameplay, bindings)
    }

    // ========================================
    // set_action tests
    // ========================================

    #[test]
    fn test_set_action_press() {
        let mut mgr = default_manager();
        mgr.test_set_action(Action::Jump, true);
        assert!(mgr.is_down(Action::Jump));
        assert!(mgr.just_pressed(Action::Jump));
    }

    #[test]
    fn test_set_action_release() {
        let mut mgr = default_manager();
        mgr.test_set_action(Action::Jump, true);
        mgr.test_set_action(Action::Jump, false);
        assert!(!mgr.is_down(Action::Jump));
    }

    #[test]
    fn test_set_action_double_press_no_duplicate_just_pressed() {
        let mut mgr = default_manager();
        mgr.test_set_action(Action::AttackLight, true);
        mgr.clear_frame();
        mgr.test_set_action(Action::AttackLight, true); // Already pressed
        assert!(mgr.is_down(Action::AttackLight));
        assert!(!mgr.just_pressed(Action::AttackLight)); // Should NOT be just_pressed since already held
    }

    #[test]
    fn test_set_action_multiple_actions() {
        let mut mgr = default_manager();
        mgr.test_set_action(Action::Jump, true);
        mgr.test_set_action(Action::AttackLight, true);
        mgr.test_set_action(Action::Sprint, true);
        assert!(mgr.is_down(Action::Jump));
        assert!(mgr.is_down(Action::AttackLight));
        assert!(mgr.is_down(Action::Sprint));
    }

    #[test]
    fn test_set_action_release_not_pressed() {
        let mut mgr = default_manager();
        // Release action that was never pressed - should be no-op
        mgr.test_set_action(Action::Ability2, false);
        assert!(!mgr.is_down(Action::Ability2));
    }

    // ========================================
    // handle_button tests (gamepad)
    // ========================================

    #[test]
    fn test_handle_button_south() {
        let mut mgr = manager_with_gamepad_bindings();
        mgr.test_handle_button(Button::South, true);
        assert!(mgr.is_down(Action::AttackLight));
    }

    #[test]
    fn test_handle_button_east() {
        let mut mgr = manager_with_gamepad_bindings();
        mgr.test_handle_button(Button::East, true);
        assert!(mgr.is_down(Action::Jump));
    }

    #[test]
    fn test_handle_button_west() {
        let mut mgr = manager_with_gamepad_bindings();
        mgr.test_handle_button(Button::West, true);
        assert!(mgr.is_down(Action::Interact));
    }

    #[test]
    fn test_handle_button_north() {
        let mut mgr = manager_with_gamepad_bindings();
        mgr.test_handle_button(Button::North, true);
        assert!(mgr.is_down(Action::AttackHeavy));
    }

    #[test]
    fn test_handle_button_triggers() {
        let mut mgr = manager_with_gamepad_bindings();
        mgr.test_handle_button(Button::LeftTrigger, true);
        assert!(mgr.is_down(Action::Sprint));
        mgr.test_handle_button(Button::RightTrigger, true);
        assert!(mgr.is_down(Action::Crouch));
    }

    #[test]
    fn test_handle_button_bumpers() {
        let mut mgr = manager_with_gamepad_bindings();
        mgr.test_handle_button(Button::LeftTrigger2, true);
        assert!(mgr.is_down(Action::Ability1));
        mgr.test_handle_button(Button::RightTrigger2, true);
        assert!(mgr.is_down(Action::Ability2));
    }

    #[test]
    fn test_handle_button_start_select() {
        let mut mgr = manager_with_gamepad_bindings();
        mgr.test_handle_button(Button::Start, true);
        assert!(mgr.is_down(Action::OpenMenu));
        mgr.test_handle_button(Button::Select, true);
        assert!(mgr.is_down(Action::OpenInventory));
    }

    #[test]
    fn test_handle_button_release() {
        let mut mgr = manager_with_gamepad_bindings();
        mgr.test_handle_button(Button::South, true);
        assert!(mgr.is_down(Action::AttackLight));
        mgr.test_handle_button(Button::South, false);
        assert!(!mgr.is_down(Action::AttackLight));
    }

    #[test]
    fn test_handle_button_unknown_ignored() {
        let mut mgr = manager_with_gamepad_bindings();
        // C button is not mapped
        mgr.test_handle_button(Button::C, true);
        // No action should be triggered
        assert!(!mgr.is_down(Action::AttackLight));
        assert!(!mgr.is_down(Action::Jump));
    }

    // ========================================
    // handle_axis tests
    // ========================================

    #[test]
    fn test_handle_axis_left_stick_x() {
        let mut mgr = default_manager();
        mgr.test_handle_axis(Axis::LeftStickX, 0.75);
        assert!((mgr.move_axis.x - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_handle_axis_left_stick_y() {
        let mut mgr = default_manager();
        mgr.test_handle_axis(Axis::LeftStickY, -0.5);
        // Y axis is inverted by default, so -0.5 becomes 0.5
        assert!((mgr.move_axis.y - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_handle_axis_right_stick() {
        let mut mgr = default_manager();
        mgr.test_handle_axis(Axis::RightStickX, 0.3);
        mgr.test_handle_axis(Axis::RightStickY, -0.8);
        assert!((mgr.look_axis.x - 0.3).abs() < 0.01);
        // Y axis is inverted by default, so -0.8 becomes 0.8
        assert!((mgr.look_axis.y - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_handle_axis_deadzone() {
        let mut mgr = default_manager();
        // Default deadzone is 0.15, value below should be zeroed
        mgr.test_handle_axis(Axis::LeftStickX, 0.1);
        assert_eq!(mgr.move_axis.x, 0.0);
    }

    #[test]
    fn test_handle_axis_above_deadzone() {
        let mut mgr = default_manager();
        // Value above deadzone should pass through
        mgr.test_handle_axis(Axis::LeftStickX, 0.5);
        assert!((mgr.move_axis.x - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_handle_axis_invert() {
        let mut bindings = BindingSet::default();
        bindings.move_axes.0.invert = true;
        let mut mgr = InputManager::new(InputContext::Gameplay, bindings);
        mgr.test_handle_axis(Axis::LeftStickX, 0.5);
        assert!((mgr.move_axis.x - (-0.5)).abs() < 0.01);
    }

    #[test]
    fn test_handle_axis_unknown_ignored() {
        let mut mgr = default_manager();
        // Unknown axis should be ignored
        mgr.test_handle_axis(Axis::Unknown, 1.0);
        assert_eq!(mgr.move_axis.x, 0.0);
        assert_eq!(mgr.look_axis.x, 0.0);
    }

    // ========================================
    // Touch/virtual joystick tests
    // ========================================

    #[test]
    fn test_touch_virtual_joystick_basic() {
        let mut mgr = default_manager();
        mgr.test_set_touch(Some(Vec2::new(100.0, 100.0)), Some(Vec2::new(140.0, 100.0)));
        mgr.poll_gamepads();
        // Delta is 40px / 80 = 0.5
        assert!((mgr.move_axis.x - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_touch_virtual_joystick_negative() {
        let mut mgr = default_manager();
        mgr.test_set_touch(Some(Vec2::new(100.0, 100.0)), Some(Vec2::new(60.0, 100.0)));
        mgr.poll_gamepads();
        // Delta is -40px / 80 = -0.5
        assert!((mgr.move_axis.x - (-0.5)).abs() < 0.01);
    }

    #[test]
    fn test_touch_virtual_joystick_y_inverted() {
        let mut mgr = default_manager();
        mgr.test_set_touch(Some(Vec2::new(100.0, 100.0)), Some(Vec2::new(100.0, 60.0)));
        mgr.poll_gamepads();
        // Y is inverted: delta -40px screen becomes +0.5 game
        assert!((mgr.move_axis.y - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_touch_virtual_joystick_clamp() {
        let mut mgr = default_manager();
        mgr.test_set_touch(Some(Vec2::new(100.0, 100.0)), Some(Vec2::new(300.0, 100.0)));
        mgr.poll_gamepads();
        // Delta 200px / 80 = 2.5, clamped to 1.0
        assert!((mgr.move_axis.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_touch_inactive_no_update() {
        let mut mgr = default_manager();
        mgr.test_set_touch(None, None);
        mgr.poll_gamepads();
        assert_eq!(mgr.move_axis.x, 0.0);
        assert_eq!(mgr.move_axis.y, 0.0);
    }

    // ========================================
    // D-pad tests
    // ========================================

    #[test]
    fn test_handle_button_dpad_up() {
        let mut bindings = BindingSet::default();
        bindings.actions.insert(
            Action::UiUp,
            Binding {
                gamepad: Some(GamepadButton::DPadUp),
                ..Default::default()
            },
        );
        let mut mgr = InputManager::new(InputContext::UI, bindings);
        mgr.test_handle_button(Button::DPadUp, true);
        assert!(mgr.is_down(Action::UiUp));
    }

    #[test]
    fn test_handle_button_dpad_down() {
        let mut bindings = BindingSet::default();
        bindings.actions.insert(
            Action::UiDown,
            Binding {
                gamepad: Some(GamepadButton::DPadDown),
                ..Default::default()
            },
        );
        let mut mgr = InputManager::new(InputContext::UI, bindings);
        mgr.test_handle_button(Button::DPadDown, true);
        assert!(mgr.is_down(Action::UiDown));
    }

    #[test]
    fn test_handle_button_dpad_left() {
        let mut bindings = BindingSet::default();
        bindings.actions.insert(
            Action::UiLeft,
            Binding {
                gamepad: Some(GamepadButton::DPadLeft),
                ..Default::default()
            },
        );
        let mut mgr = InputManager::new(InputContext::UI, bindings);
        mgr.test_handle_button(Button::DPadLeft, true);
        assert!(mgr.is_down(Action::UiLeft));
    }

    #[test]
    fn test_handle_button_dpad_right() {
        let mut bindings = BindingSet::default();
        bindings.actions.insert(
            Action::UiRight,
            Binding {
                gamepad: Some(GamepadButton::DPadRight),
                ..Default::default()
            },
        );
        let mut mgr = InputManager::new(InputContext::UI, bindings);
        mgr.test_handle_button(Button::DPadRight, true);
        assert!(mgr.is_down(Action::UiRight));
    }

    #[test]
    fn test_handle_button_thumb_sticks() {
        let mut bindings = BindingSet::default();
        bindings.actions.insert(
            Action::Sprint,
            Binding {
                gamepad: Some(GamepadButton::LStick),
                ..Default::default()
            },
        );
        bindings.actions.insert(
            Action::Crouch,
            Binding {
                gamepad: Some(GamepadButton::RStick),
                ..Default::default()
            },
        );
        let mut mgr = InputManager::new(InputContext::Gameplay, bindings);
        mgr.test_handle_button(Button::LeftThumb, true);
        assert!(mgr.is_down(Action::Sprint));
        mgr.test_handle_button(Button::RightThumb, true);
        assert!(mgr.is_down(Action::Crouch));
    }
}
