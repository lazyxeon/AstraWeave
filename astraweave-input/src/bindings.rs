use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::Action;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GamepadButton {
    South,
    East,
    West,
    North, // A/B/X/Y (Xbox)
    L1,
    R1,
    L2,
    R2,
    Select,
    Start,
    LStick,
    RStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

impl GamepadButton {
    /// Returns the name of this button.
    pub fn name(&self) -> &'static str {
        match self {
            Self::South => "South",
            Self::East => "East",
            Self::West => "West",
            Self::North => "North",
            Self::L1 => "L1",
            Self::R1 => "R1",
            Self::L2 => "L2",
            Self::R2 => "R2",
            Self::Select => "Select",
            Self::Start => "Start",
            Self::LStick => "LStick",
            Self::RStick => "RStick",
            Self::DPadUp => "DPadUp",
            Self::DPadDown => "DPadDown",
            Self::DPadLeft => "DPadLeft",
            Self::DPadRight => "DPadRight",
        }
    }

    /// Returns true if this is a face button (A/B/X/Y).
    #[inline]
    pub fn is_face(&self) -> bool {
        matches!(self, Self::South | Self::East | Self::West | Self::North)
    }

    /// Returns true if this is a shoulder button (L1/R1/L2/R2).
    #[inline]
    pub fn is_shoulder(&self) -> bool {
        matches!(self, Self::L1 | Self::R1 | Self::L2 | Self::R2)
    }

    /// Returns true if this is a trigger (L2/R2).
    #[inline]
    pub fn is_trigger(&self) -> bool {
        matches!(self, Self::L2 | Self::R2)
    }

    /// Returns true if this is a bumper (L1/R1).
    #[inline]
    pub fn is_bumper(&self) -> bool {
        matches!(self, Self::L1 | Self::R1)
    }

    /// Returns true if this is a D-pad button.
    #[inline]
    pub fn is_dpad(&self) -> bool {
        matches!(
            self,
            Self::DPadUp | Self::DPadDown | Self::DPadLeft | Self::DPadRight
        )
    }

    /// Returns true if this is a stick click.
    #[inline]
    pub fn is_stick(&self) -> bool {
        matches!(self, Self::LStick | Self::RStick)
    }

    /// Returns true if this is a system button (Select/Start).
    #[inline]
    pub fn is_system(&self) -> bool {
        matches!(self, Self::Select | Self::Start)
    }

    /// Returns all face buttons.
    pub fn face_buttons() -> [GamepadButton; 4] {
        [Self::South, Self::East, Self::West, Self::North]
    }

    /// Returns all shoulder buttons.
    pub fn shoulder_buttons() -> [GamepadButton; 4] {
        [Self::L1, Self::R1, Self::L2, Self::R2]
    }

    /// Returns all D-pad buttons.
    pub fn dpad_buttons() -> [GamepadButton; 4] {
        [
            Self::DPadUp,
            Self::DPadDown,
            Self::DPadLeft,
            Self::DPadRight,
        ]
    }

    /// Returns all buttons.
    pub fn all() -> [GamepadButton; 16] {
        [
            Self::South,
            Self::East,
            Self::West,
            Self::North,
            Self::L1,
            Self::R1,
            Self::L2,
            Self::R2,
            Self::Select,
            Self::Start,
            Self::LStick,
            Self::RStick,
            Self::DPadUp,
            Self::DPadDown,
            Self::DPadLeft,
            Self::DPadRight,
        ]
    }
}

impl std::fmt::Display for GamepadButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AxisKind {
    LeftX,
    LeftY,
    RightX,
    RightY,
    LT,
    RT,
}

impl AxisKind {
    /// Returns the name of this axis.
    pub fn name(&self) -> &'static str {
        match self {
            Self::LeftX => "LeftX",
            Self::LeftY => "LeftY",
            Self::RightX => "RightX",
            Self::RightY => "RightY",
            Self::LT => "LT",
            Self::RT => "RT",
        }
    }

    /// Returns true if this is a left stick axis.
    #[inline]
    pub fn is_left_stick(&self) -> bool {
        matches!(self, Self::LeftX | Self::LeftY)
    }

    /// Returns true if this is a right stick axis.
    #[inline]
    pub fn is_right_stick(&self) -> bool {
        matches!(self, Self::RightX | Self::RightY)
    }

    /// Returns true if this is a stick axis (not a trigger).
    #[inline]
    pub fn is_stick(&self) -> bool {
        self.is_left_stick() || self.is_right_stick()
    }

    /// Returns true if this is a trigger axis.
    #[inline]
    pub fn is_trigger(&self) -> bool {
        matches!(self, Self::LT | Self::RT)
    }

    /// Returns true if this is an X-axis.
    #[inline]
    pub fn is_x_axis(&self) -> bool {
        matches!(self, Self::LeftX | Self::RightX)
    }

    /// Returns true if this is a Y-axis.
    #[inline]
    pub fn is_y_axis(&self) -> bool {
        matches!(self, Self::LeftY | Self::RightY)
    }

    /// Returns the paired axis (X pairs with Y, Y pairs with X).
    pub fn paired(&self) -> Option<AxisKind> {
        match self {
            Self::LeftX => Some(Self::LeftY),
            Self::LeftY => Some(Self::LeftX),
            Self::RightX => Some(Self::RightY),
            Self::RightY => Some(Self::RightX),
            Self::LT | Self::RT => None,
        }
    }

    /// Returns all stick axes.
    pub fn stick_axes() -> [AxisKind; 4] {
        [Self::LeftX, Self::LeftY, Self::RightX, Self::RightY]
    }

    /// Returns all trigger axes.
    pub fn trigger_axes() -> [AxisKind; 2] {
        [Self::LT, Self::RT]
    }

    /// Returns all axis kinds.
    pub fn all() -> [AxisKind; 6] {
        [
            Self::LeftX,
            Self::LeftY,
            Self::RightX,
            Self::RightY,
            Self::LT,
            Self::RT,
        ]
    }
}

impl std::fmt::Display for AxisKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Binding {
    pub key: Option<KeyCode>,
    pub mouse: Option<MouseButton>,
    pub gamepad: Option<GamepadButton>,
}

impl Binding {
    /// Creates a new empty binding.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a binding with a keyboard key.
    pub fn with_key(key: KeyCode) -> Self {
        Self {
            key: Some(key),
            mouse: None,
            gamepad: None,
        }
    }

    /// Creates a binding with a mouse button.
    pub fn with_mouse(button: MouseButton) -> Self {
        Self {
            key: None,
            mouse: Some(button),
            gamepad: None,
        }
    }

    /// Creates a binding with a gamepad button.
    pub fn with_gamepad(button: GamepadButton) -> Self {
        Self {
            key: None,
            mouse: None,
            gamepad: Some(button),
        }
    }

    /// Returns true if no input is bound.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.key.is_none() && self.mouse.is_none() && self.gamepad.is_none()
    }

    /// Returns true if a keyboard key is bound.
    #[inline]
    pub fn has_key(&self) -> bool {
        self.key.is_some()
    }

    /// Returns true if a mouse button is bound.
    #[inline]
    pub fn has_mouse(&self) -> bool {
        self.mouse.is_some()
    }

    /// Returns true if a gamepad button is bound.
    #[inline]
    pub fn has_gamepad(&self) -> bool {
        self.gamepad.is_some()
    }

    /// Returns the number of inputs bound.
    pub fn binding_count(&self) -> usize {
        let mut count = 0;
        if self.key.is_some() {
            count += 1;
        }
        if self.mouse.is_some() {
            count += 1;
        }
        if self.gamepad.is_some() {
            count += 1;
        }
        count
    }
}

impl std::fmt::Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if let Some(key) = &self.key {
            parts.push(format!("Key({:?})", key));
        }
        if let Some(mouse) = &self.mouse {
            parts.push(format!("Mouse({:?})", mouse));
        }
        if let Some(gamepad) = &self.gamepad {
            parts.push(format!("Gamepad({})", gamepad));
        }
        if parts.is_empty() {
            write!(f, "Binding(none)")
        } else {
            write!(f, "Binding({})", parts.join(", "))
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AxisBinding {
    pub axis: AxisKind,
    pub invert: bool,
    pub deadzone: f32,
}

impl AxisBinding {
    /// Creates a new axis binding with default deadzone.
    pub fn new(axis: AxisKind) -> Self {
        Self {
            axis,
            invert: false,
            deadzone: 0.15,
        }
    }

    /// Creates a new axis binding with the specified deadzone.
    pub fn with_deadzone(axis: AxisKind, deadzone: f32) -> Self {
        Self {
            axis,
            invert: false,
            deadzone,
        }
    }

    /// Creates an inverted axis binding.
    pub fn inverted(axis: AxisKind) -> Self {
        Self {
            axis,
            invert: true,
            deadzone: 0.15,
        }
    }

    /// Returns true if the axis is inverted.
    #[inline]
    pub fn is_inverted(&self) -> bool {
        self.invert
    }

    /// Applies the binding to a raw axis value.
    pub fn apply(&self, value: f32) -> f32 {
        let abs_value = value.abs();
        if abs_value < self.deadzone {
            return 0.0;
        }
        let normalized = (abs_value - self.deadzone) / (1.0 - self.deadzone);
        let result = normalized * value.signum();
        if self.invert {
            -result
        } else {
            result
        }
    }
}

impl std::fmt::Display for AxisBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AxisBinding({}", self.axis)?;
        if self.invert {
            write!(f, ", inverted")?;
        }
        write!(f, ", deadzone={})", self.deadzone)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BindingSet {
    pub actions: HashMap<Action, Binding>,
    pub move_axes: (AxisBinding, AxisBinding), // (LeftX, LeftY)
    pub look_axes: (AxisBinding, AxisBinding), // (RightX, RightY)
}

impl BindingSet {
    /// Creates a new empty binding set.
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
            move_axes: (
                AxisBinding::new(AxisKind::LeftX),
                AxisBinding::new(AxisKind::LeftY),
            ),
            look_axes: (
                AxisBinding::new(AxisKind::RightX),
                AxisBinding::new(AxisKind::RightY),
            ),
        }
    }

    /// Returns the number of action bindings.
    #[inline]
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Returns true if there's a binding for the given action.
    #[inline]
    pub fn has_binding(&self, action: &Action) -> bool {
        self.actions.contains_key(action)
    }

    /// Returns the binding for an action, if any.
    pub fn get_binding(&self, action: &Action) -> Option<&Binding> {
        self.actions.get(action)
    }

    /// Sets or updates a binding for an action.
    pub fn set_binding(&mut self, action: Action, binding: Binding) {
        self.actions.insert(action, binding);
    }

    /// Removes a binding for an action.
    pub fn remove_binding(&mut self, action: &Action) -> Option<Binding> {
        self.actions.remove(action)
    }

    /// Returns all bound actions.
    pub fn bound_actions(&self) -> Vec<&Action> {
        self.actions.keys().collect()
    }

    /// Returns the number of non-empty bindings.
    pub fn non_empty_binding_count(&self) -> usize {
        self.actions.values().filter(|b| !b.is_empty()).count()
    }
}

impl std::fmt::Display for BindingSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BindingSet({} actions, move={}/{}, look={}/{})",
            self.actions.len(),
            self.move_axes.0.axis,
            self.move_axes.1.axis,
            self.look_axes.0.axis,
            self.look_axes.1.axis
        )
    }
}

impl Default for BindingSet {
    fn default() -> Self {
        use Action::*;
        let mut actions = HashMap::new();
        // Keyboard defaults
        actions.insert(
            MoveForward,
            Binding {
                key: Some(KeyCode::KeyW),
                ..Default::default()
            },
        );
        actions.insert(
            MoveBackward,
            Binding {
                key: Some(KeyCode::KeyS),
                ..Default::default()
            },
        );
        actions.insert(
            MoveLeft,
            Binding {
                key: Some(KeyCode::KeyA),
                ..Default::default()
            },
        );
        actions.insert(
            MoveRight,
            Binding {
                key: Some(KeyCode::KeyD),
                ..Default::default()
            },
        );
        actions.insert(
            Jump,
            Binding {
                key: Some(KeyCode::Space),
                ..Default::default()
            },
        );
        actions.insert(
            Crouch,
            Binding {
                key: Some(KeyCode::ControlLeft),
                ..Default::default()
            },
        );
        actions.insert(
            Sprint,
            Binding {
                key: Some(KeyCode::ShiftLeft),
                ..Default::default()
            },
        );
        actions.insert(
            Interact,
            Binding {
                key: Some(KeyCode::KeyE),
                ..Default::default()
            },
        );
        actions.insert(
            AttackLight,
            Binding {
                mouse: Some(MouseButton::Left),
                ..Default::default()
            },
        );
        actions.insert(
            AttackHeavy,
            Binding {
                mouse: Some(MouseButton::Right),
                ..Default::default()
            },
        );

        actions.insert(
            OpenInventory,
            Binding {
                key: Some(KeyCode::KeyI),
                ..Default::default()
            },
        );
        actions.insert(
            OpenMap,
            Binding {
                key: Some(KeyCode::KeyM),
                ..Default::default()
            },
        );
        actions.insert(
            OpenQuests,
            Binding {
                key: Some(KeyCode::KeyJ),
                ..Default::default()
            },
        );
        actions.insert(
            OpenCrafting,
            Binding {
                key: Some(KeyCode::KeyC),
                ..Default::default()
            },
        );
        actions.insert(
            OpenMenu,
            Binding {
                key: Some(KeyCode::Escape),
                ..Default::default()
            },
        );

        // UI nav defaults
        actions.insert(
            UiAccept,
            Binding {
                key: Some(KeyCode::Enter),
                ..Default::default()
            },
        );
        actions.insert(
            UiBack,
            Binding {
                key: Some(KeyCode::Escape),
                ..Default::default()
            },
        );
        actions.insert(
            UiUp,
            Binding {
                key: Some(KeyCode::ArrowUp),
                ..Default::default()
            },
        );
        actions.insert(
            UiDown,
            Binding {
                key: Some(KeyCode::ArrowDown),
                ..Default::default()
            },
        );
        actions.insert(
            UiLeft,
            Binding {
                key: Some(KeyCode::ArrowLeft),
                ..Default::default()
            },
        );
        actions.insert(
            UiRight,
            Binding {
                key: Some(KeyCode::ArrowRight),
                ..Default::default()
            },
        );

        Self {
            actions,
            move_axes: (
                AxisBinding {
                    axis: AxisKind::LeftX,
                    invert: false,
                    deadzone: 0.15,
                },
                AxisBinding {
                    axis: AxisKind::LeftY,
                    invert: true,
                    deadzone: 0.15,
                },
            ),
            look_axes: (
                AxisBinding {
                    axis: AxisKind::RightX,
                    invert: false,
                    deadzone: 0.12,
                },
                AxisBinding {
                    axis: AxisKind::RightY,
                    invert: true,
                    deadzone: 0.12,
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== GamepadButton Tests ==========

    #[test]
    fn test_gamepad_button_name() {
        assert_eq!(GamepadButton::South.name(), "South");
        assert_eq!(GamepadButton::North.name(), "North");
        assert_eq!(GamepadButton::L1.name(), "L1");
        assert_eq!(GamepadButton::DPadUp.name(), "DPadUp");
    }

    #[test]
    fn test_gamepad_button_is_face() {
        assert!(GamepadButton::South.is_face());
        assert!(GamepadButton::East.is_face());
        assert!(GamepadButton::West.is_face());
        assert!(GamepadButton::North.is_face());
        assert!(!GamepadButton::L1.is_face());
        assert!(!GamepadButton::DPadUp.is_face());
    }

    #[test]
    fn test_gamepad_button_is_shoulder() {
        assert!(GamepadButton::L1.is_shoulder());
        assert!(GamepadButton::R1.is_shoulder());
        assert!(GamepadButton::L2.is_shoulder());
        assert!(GamepadButton::R2.is_shoulder());
        assert!(!GamepadButton::South.is_shoulder());
        assert!(!GamepadButton::DPadUp.is_shoulder());
    }

    #[test]
    fn test_gamepad_button_is_trigger() {
        assert!(GamepadButton::L2.is_trigger());
        assert!(GamepadButton::R2.is_trigger());
        assert!(!GamepadButton::L1.is_trigger());
        assert!(!GamepadButton::R1.is_trigger());
    }

    #[test]
    fn test_gamepad_button_is_bumper() {
        assert!(GamepadButton::L1.is_bumper());
        assert!(GamepadButton::R1.is_bumper());
        assert!(!GamepadButton::L2.is_bumper());
        assert!(!GamepadButton::R2.is_bumper());
    }

    #[test]
    fn test_gamepad_button_is_dpad() {
        assert!(GamepadButton::DPadUp.is_dpad());
        assert!(GamepadButton::DPadDown.is_dpad());
        assert!(GamepadButton::DPadLeft.is_dpad());
        assert!(GamepadButton::DPadRight.is_dpad());
        assert!(!GamepadButton::South.is_dpad());
    }

    #[test]
    fn test_gamepad_button_is_stick() {
        assert!(GamepadButton::LStick.is_stick());
        assert!(GamepadButton::RStick.is_stick());
        assert!(!GamepadButton::South.is_stick());
        assert!(!GamepadButton::Start.is_stick());
    }

    #[test]
    fn test_gamepad_button_is_system() {
        assert!(GamepadButton::Select.is_system());
        assert!(GamepadButton::Start.is_system());
        assert!(!GamepadButton::South.is_system());
        assert!(!GamepadButton::LStick.is_system());
    }

    #[test]
    fn test_gamepad_button_face_buttons() {
        let faces = GamepadButton::face_buttons();
        assert_eq!(faces.len(), 4);
        assert!(faces.iter().all(|b| b.is_face()));
    }

    #[test]
    fn test_gamepad_button_shoulder_buttons() {
        let shoulders = GamepadButton::shoulder_buttons();
        assert_eq!(shoulders.len(), 4);
        assert!(shoulders.iter().all(|b| b.is_shoulder()));
    }

    #[test]
    fn test_gamepad_button_dpad_buttons() {
        let dpads = GamepadButton::dpad_buttons();
        assert_eq!(dpads.len(), 4);
        assert!(dpads.iter().all(|b| b.is_dpad()));
    }

    #[test]
    fn test_gamepad_button_all() {
        let all = GamepadButton::all();
        assert_eq!(all.len(), 16);
    }

    #[test]
    fn test_gamepad_button_display() {
        assert_eq!(format!("{}", GamepadButton::South), "South");
        assert_eq!(format!("{}", GamepadButton::DPadUp), "DPadUp");
    }

    // ========== AxisKind Tests ==========

    #[test]
    fn test_axis_kind_name() {
        assert_eq!(AxisKind::LeftX.name(), "LeftX");
        assert_eq!(AxisKind::RightY.name(), "RightY");
        assert_eq!(AxisKind::LT.name(), "LT");
    }

    #[test]
    fn test_axis_kind_is_left_stick() {
        assert!(AxisKind::LeftX.is_left_stick());
        assert!(AxisKind::LeftY.is_left_stick());
        assert!(!AxisKind::RightX.is_left_stick());
        assert!(!AxisKind::LT.is_left_stick());
    }

    #[test]
    fn test_axis_kind_is_right_stick() {
        assert!(AxisKind::RightX.is_right_stick());
        assert!(AxisKind::RightY.is_right_stick());
        assert!(!AxisKind::LeftX.is_right_stick());
        assert!(!AxisKind::RT.is_right_stick());
    }

    #[test]
    fn test_axis_kind_is_stick() {
        assert!(AxisKind::LeftX.is_stick());
        assert!(AxisKind::LeftY.is_stick());
        assert!(AxisKind::RightX.is_stick());
        assert!(AxisKind::RightY.is_stick());
        assert!(!AxisKind::LT.is_stick());
        assert!(!AxisKind::RT.is_stick());
    }

    #[test]
    fn test_axis_kind_is_trigger() {
        assert!(AxisKind::LT.is_trigger());
        assert!(AxisKind::RT.is_trigger());
        assert!(!AxisKind::LeftX.is_trigger());
    }

    #[test]
    fn test_axis_kind_is_x_axis() {
        assert!(AxisKind::LeftX.is_x_axis());
        assert!(AxisKind::RightX.is_x_axis());
        assert!(!AxisKind::LeftY.is_x_axis());
        assert!(!AxisKind::LT.is_x_axis());
    }

    #[test]
    fn test_axis_kind_is_y_axis() {
        assert!(AxisKind::LeftY.is_y_axis());
        assert!(AxisKind::RightY.is_y_axis());
        assert!(!AxisKind::LeftX.is_y_axis());
        assert!(!AxisKind::RT.is_y_axis());
    }

    #[test]
    fn test_axis_kind_paired() {
        assert_eq!(AxisKind::LeftX.paired(), Some(AxisKind::LeftY));
        assert_eq!(AxisKind::LeftY.paired(), Some(AxisKind::LeftX));
        assert_eq!(AxisKind::RightX.paired(), Some(AxisKind::RightY));
        assert_eq!(AxisKind::RightY.paired(), Some(AxisKind::RightX));
        assert_eq!(AxisKind::LT.paired(), None);
        assert_eq!(AxisKind::RT.paired(), None);
    }

    #[test]
    fn test_axis_kind_stick_axes() {
        let sticks = AxisKind::stick_axes();
        assert_eq!(sticks.len(), 4);
        assert!(sticks.iter().all(|a| a.is_stick()));
    }

    #[test]
    fn test_axis_kind_trigger_axes() {
        let triggers = AxisKind::trigger_axes();
        assert_eq!(triggers.len(), 2);
        assert!(triggers.iter().all(|a| a.is_trigger()));
    }

    #[test]
    fn test_axis_kind_all() {
        let all = AxisKind::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_axis_kind_display() {
        assert_eq!(format!("{}", AxisKind::LeftX), "LeftX");
        assert_eq!(format!("{}", AxisKind::RT), "RT");
    }

    // ========== Binding Tests ==========

    #[test]
    fn test_binding_new() {
        let binding = Binding::new();
        assert!(binding.is_empty());
        assert!(!binding.has_key());
        assert!(!binding.has_mouse());
        assert!(!binding.has_gamepad());
    }

    #[test]
    fn test_binding_with_key() {
        let binding = Binding::with_key(KeyCode::Space);
        assert!(binding.has_key());
        assert!(!binding.has_mouse());
        assert!(!binding.has_gamepad());
        assert!(!binding.is_empty());
        assert_eq!(binding.binding_count(), 1);
    }

    #[test]
    fn test_binding_with_mouse() {
        let binding = Binding::with_mouse(MouseButton::Left);
        assert!(!binding.has_key());
        assert!(binding.has_mouse());
        assert!(!binding.has_gamepad());
        assert_eq!(binding.binding_count(), 1);
    }

    #[test]
    fn test_binding_with_gamepad() {
        let binding = Binding::with_gamepad(GamepadButton::South);
        assert!(!binding.has_key());
        assert!(!binding.has_mouse());
        assert!(binding.has_gamepad());
        assert_eq!(binding.binding_count(), 1);
    }

    #[test]
    fn test_binding_count_multiple() {
        let binding = Binding {
            key: Some(KeyCode::Space),
            mouse: Some(MouseButton::Left),
            gamepad: Some(GamepadButton::South),
        };
        assert_eq!(binding.binding_count(), 3);
    }

    #[test]
    fn test_binding_display_empty() {
        let binding = Binding::new();
        assert_eq!(format!("{}", binding), "Binding(none)");
    }

    #[test]
    fn test_binding_display_key() {
        let binding = Binding::with_key(KeyCode::Space);
        assert!(format!("{}", binding).contains("Key("));
    }

    #[test]
    fn test_binding_display_gamepad() {
        let binding = Binding::with_gamepad(GamepadButton::South);
        assert!(format!("{}", binding).contains("Gamepad(South)"));
    }

    // ========== AxisBinding Tests ==========

    #[test]
    fn test_axis_binding_new() {
        let binding = AxisBinding::new(AxisKind::LeftX);
        assert_eq!(binding.axis, AxisKind::LeftX);
        assert!(!binding.invert);
        assert_eq!(binding.deadzone, 0.15);
    }

    #[test]
    fn test_axis_binding_with_deadzone() {
        let binding = AxisBinding::with_deadzone(AxisKind::RightY, 0.25);
        assert_eq!(binding.axis, AxisKind::RightY);
        assert!(!binding.invert);
        assert_eq!(binding.deadzone, 0.25);
    }

    #[test]
    fn test_axis_binding_inverted() {
        let binding = AxisBinding::inverted(AxisKind::LeftY);
        assert_eq!(binding.axis, AxisKind::LeftY);
        assert!(binding.invert);
        assert!(binding.is_inverted());
    }

    #[test]
    fn test_axis_binding_apply_zero_in_deadzone() {
        let binding = AxisBinding::with_deadzone(AxisKind::LeftX, 0.2);
        assert_eq!(binding.apply(0.1), 0.0);
        assert_eq!(binding.apply(-0.1), 0.0);
        assert_eq!(binding.apply(0.19), 0.0);
    }

    #[test]
    fn test_axis_binding_apply_outside_deadzone() {
        let binding = AxisBinding::with_deadzone(AxisKind::LeftX, 0.2);
        let result = binding.apply(0.6);
        assert!(result > 0.0);
        assert!(result <= 1.0);
    }

    #[test]
    fn test_axis_binding_apply_inverted() {
        let binding = AxisBinding::inverted(AxisKind::LeftY);
        let result = binding.apply(0.5);
        assert!(result < 0.0); // Should be inverted
    }

    #[test]
    fn test_axis_binding_display() {
        let binding = AxisBinding::new(AxisKind::LeftX);
        let display = format!("{}", binding);
        assert!(display.contains("AxisBinding(LeftX"));
        assert!(display.contains("deadzone="));
    }

    #[test]
    fn test_axis_binding_display_inverted() {
        let binding = AxisBinding::inverted(AxisKind::LeftY);
        let display = format!("{}", binding);
        assert!(display.contains("inverted"));
    }

    // ========== BindingSet Tests ==========

    #[test]
    fn test_binding_set_new() {
        let set = BindingSet::new();
        assert_eq!(set.action_count(), 0);
        assert_eq!(set.move_axes.0.axis, AxisKind::LeftX);
        assert_eq!(set.move_axes.1.axis, AxisKind::LeftY);
        assert_eq!(set.look_axes.0.axis, AxisKind::RightX);
        assert_eq!(set.look_axes.1.axis, AxisKind::RightY);
    }

    #[test]
    fn test_binding_set_default_has_bindings() {
        let set = BindingSet::default();
        assert!(set.action_count() > 0);
        assert!(set.has_binding(&Action::MoveForward));
        assert!(set.has_binding(&Action::Jump));
    }

    #[test]
    fn test_binding_set_get_binding() {
        let set = BindingSet::default();
        let binding = set.get_binding(&Action::MoveForward);
        assert!(binding.is_some());
        let binding = binding.unwrap();
        assert!(binding.has_key());
        assert_eq!(binding.key, Some(KeyCode::KeyW));
    }

    #[test]
    fn test_binding_set_set_binding() {
        let mut set = BindingSet::new();
        set.set_binding(Action::Jump, Binding::with_key(KeyCode::Space));
        assert!(set.has_binding(&Action::Jump));
        assert_eq!(set.action_count(), 1);
    }

    #[test]
    fn test_binding_set_remove_binding() {
        let mut set = BindingSet::default();
        assert!(set.has_binding(&Action::MoveForward));
        let removed = set.remove_binding(&Action::MoveForward);
        assert!(removed.is_some());
        assert!(!set.has_binding(&Action::MoveForward));
    }

    #[test]
    fn test_binding_set_bound_actions() {
        let set = BindingSet::default();
        let actions = set.bound_actions();
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_binding_set_non_empty_binding_count() {
        let mut set = BindingSet::new();
        set.set_binding(Action::Jump, Binding::with_key(KeyCode::Space));
        set.set_binding(Action::AttackLight, Binding::new()); // empty
        assert_eq!(set.action_count(), 2);
        assert_eq!(set.non_empty_binding_count(), 1);
    }

    #[test]
    fn test_binding_set_display() {
        let set = BindingSet::default();
        let display = format!("{}", set);
        assert!(display.contains("BindingSet("));
        assert!(display.contains("actions"));
    }

    /// Kills `bindings.rs:409` `non_empty_binding_count → 1`
    /// Default BindingSet has many non-empty bindings (key-bound actions).
    #[test]
    fn non_empty_binding_count_default_is_not_one() {
        let set = BindingSet::default();
        let count = set.non_empty_binding_count();
        assert!(
            count > 1,
            "default set should have >1 non-empty bindings, got {}",
            count
        );
    }

    /// Kills `bindings.rs:409` `delete !` in non_empty_binding_count filter
    /// An empty BindingSet should have 0 non-empty bindings.
    #[test]
    fn non_empty_binding_count_empty_set_is_zero() {
        let set = BindingSet::new(); // empty, no actions
        assert_eq!(set.non_empty_binding_count(), 0);
    }
}
