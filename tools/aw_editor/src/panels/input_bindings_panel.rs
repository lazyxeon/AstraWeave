//! Input Bindings Panel for the editor UI
//!
//! Provides comprehensive input configuration:
//! - Keyboard/mouse/gamepad binding editor
//! - Action categories (Movement, Combat, UI, etc.)
//! - Preset management (default, FPS, third-person, etc.)
//! - Axis configuration (deadzone, sensitivity, invert)
//! - Input testing and visualization
//! - Conflict detection

#![allow(clippy::upper_case_acronyms)] // FPS, RTS are industry-standard acronyms

use egui::{Color32, RichText, Ui, Vec2};
use tracing::info;

use crate::panels::Panel;

// ============================================================================
// PANEL ACTIONS - Events produced by the panel for external handling
// ============================================================================

/// Actions emitted by the input bindings panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum InputBindingAction {
    /// Apply a new keybinding to an action
    SetBinding {
        action_index: usize,
        target: InputTarget,
        key: String, // Key name as string for flexibility
    },
    /// Clear all bindings for an action
    ClearBindings { action_index: usize },
    /// Apply a binding preset
    ApplyPreset { preset: BindingPreset },
    /// Save current bindings to file
    SaveBindings,
    /// Load bindings from file
    LoadBindings,
    /// Reset bindings to defaults
    ResetToDefaults,
    /// Update mouse sensitivity
    SetMouseSensitivity { sensitivity: f32 },
    /// Update gamepad rumble settings
    SetRumble { enabled: bool, intensity: f32 },
}

/// Input device type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputDevice {
    #[default]
    Keyboard,
    Mouse,
    Gamepad,
    All,
}

impl std::fmt::Display for InputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl InputDevice {
    pub fn all() -> &'static [InputDevice] {
        &[
            InputDevice::Keyboard,
            InputDevice::Mouse,
            InputDevice::Gamepad,
            InputDevice::All,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            InputDevice::Keyboard => "Keyboard",
            InputDevice::Mouse => "Mouse",
            InputDevice::Gamepad => "Gamepad",
            InputDevice::All => "All Devices",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            InputDevice::Keyboard => "⌨️",
            InputDevice::Mouse => "🖱️",
            InputDevice::Gamepad => "[Gp]",
            InputDevice::All => "[Wrn]",
        }
    }

    pub fn is_physical(&self) -> bool {
        !matches!(self, InputDevice::All)
    }
}

/// Action category for grouping
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ActionCategory {
    #[default]
    Movement,
    Combat,
    Interaction,
    UI,
    Camera,
    Vehicle,
    Debug,
}

impl std::fmt::Display for ActionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ActionCategory {
    pub fn all() -> &'static [ActionCategory] {
        &[
            ActionCategory::Movement,
            ActionCategory::Combat,
            ActionCategory::Interaction,
            ActionCategory::UI,
            ActionCategory::Camera,
            ActionCategory::Vehicle,
            ActionCategory::Debug,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ActionCategory::Movement => "Movement",
            ActionCategory::Combat => "Combat",
            ActionCategory::Interaction => "Interaction",
            ActionCategory::UI => "UI",
            ActionCategory::Camera => "Camera",
            ActionCategory::Vehicle => "Vehicle",
            ActionCategory::Debug => "Debug",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ActionCategory::Movement => "🚶",
            ActionCategory::Combat => "⚔️",
            ActionCategory::Interaction => "🤝",
            ActionCategory::UI => "[List]",
            ActionCategory::Camera => "📷",
            ActionCategory::Vehicle => "🚗",
            ActionCategory::Debug => "🐛",
        }
    }

    pub fn is_gameplay(&self) -> bool {
        matches!(
            self,
            ActionCategory::Movement
                | ActionCategory::Combat
                | ActionCategory::Interaction
                | ActionCategory::Vehicle
        )
    }
}

/// Binding preset
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BindingPreset {
    #[default]
    Default,
    FPS,
    ThirdPerson,
    RTS,
    Racing,
    LeftHanded,
    Custom,
}

impl std::fmt::Display for BindingPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl BindingPreset {
    pub fn all() -> &'static [BindingPreset] {
        &[
            BindingPreset::Default,
            BindingPreset::FPS,
            BindingPreset::ThirdPerson,
            BindingPreset::RTS,
            BindingPreset::Racing,
            BindingPreset::LeftHanded,
            BindingPreset::Custom,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BindingPreset::Default => "Default",
            BindingPreset::FPS => "FPS",
            BindingPreset::ThirdPerson => "Third Person",
            BindingPreset::RTS => "RTS",
            BindingPreset::Racing => "Racing",
            BindingPreset::LeftHanded => "Left Handed",
            BindingPreset::Custom => "Custom",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            BindingPreset::Default => "⚙️",
            BindingPreset::FPS => "[Wpn]",
            BindingPreset::ThirdPerson => "[Gp]",
            BindingPreset::RTS => "🗺️",
            BindingPreset::Racing => "🏎️",
            BindingPreset::LeftHanded => "🫲",
            BindingPreset::Custom => "✏️",
        }
    }

    pub fn is_built_in(&self) -> bool {
        !matches!(self, BindingPreset::Custom)
    }
}

/// Gamepad button representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GamepadButton {
    South, // A (Xbox), X (PlayStation)
    East,  // B (Xbox), Circle (PlayStation)
    West,  // X (Xbox), Square (PlayStation)
    North, // Y (Xbox), Triangle (PlayStation)
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

impl std::fmt::Display for GamepadButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl GamepadButton {
    pub fn all() -> &'static [GamepadButton] {
        &[
            GamepadButton::South,
            GamepadButton::East,
            GamepadButton::West,
            GamepadButton::North,
            GamepadButton::L1,
            GamepadButton::R1,
            GamepadButton::L2,
            GamepadButton::R2,
            GamepadButton::Select,
            GamepadButton::Start,
            GamepadButton::LStick,
            GamepadButton::RStick,
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadLeft,
            GamepadButton::DPadRight,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GamepadButton::South => "A / ✕",
            GamepadButton::East => "B / ○",
            GamepadButton::West => "X / □",
            GamepadButton::North => "Y / △",
            GamepadButton::L1 => "LB / L1",
            GamepadButton::R1 => "RB / R1",
            GamepadButton::L2 => "LT / L2",
            GamepadButton::R2 => "RT / R2",
            GamepadButton::Select => "Select",
            GamepadButton::Start => "Start",
            GamepadButton::LStick => "L3",
            GamepadButton::RStick => "R3",
            GamepadButton::DPadUp => "D-Up",
            GamepadButton::DPadDown => "D-Down",
            GamepadButton::DPadLeft => "D-Left",
            GamepadButton::DPadRight => "D-Right",
        }
    }

    pub fn is_face_button(&self) -> bool {
        matches!(
            self,
            GamepadButton::South | GamepadButton::East | GamepadButton::West | GamepadButton::North
        )
    }

    pub fn is_shoulder(&self) -> bool {
        matches!(
            self,
            GamepadButton::L1 | GamepadButton::R1 | GamepadButton::L2 | GamepadButton::R2
        )
    }

    pub fn is_dpad(&self) -> bool {
        matches!(
            self,
            GamepadButton::DPadUp
                | GamepadButton::DPadDown
                | GamepadButton::DPadLeft
                | GamepadButton::DPadRight
        )
    }
}

/// Common keyboard key representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum KeyboardKey {
    // Letters
    #[default]
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    // Numbers
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // Modifiers
    ShiftLeft,
    ShiftRight,
    CtrlLeft,
    CtrlRight,
    AltLeft,
    AltRight,
    // Special
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    // Arrow keys
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    // Other
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
}

impl std::fmt::Display for KeyboardKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "⌨️ {}", self.name())
    }
}

impl KeyboardKey {
    /// Returns all keyboard keys
    pub fn all() -> &'static [KeyboardKey] {
        &[
            // Letters
            KeyboardKey::A,
            KeyboardKey::B,
            KeyboardKey::C,
            KeyboardKey::D,
            KeyboardKey::E,
            KeyboardKey::F,
            KeyboardKey::G,
            KeyboardKey::H,
            KeyboardKey::I,
            KeyboardKey::J,
            KeyboardKey::K,
            KeyboardKey::L,
            KeyboardKey::M,
            KeyboardKey::N,
            KeyboardKey::O,
            KeyboardKey::P,
            KeyboardKey::Q,
            KeyboardKey::R,
            KeyboardKey::S,
            KeyboardKey::T,
            KeyboardKey::U,
            KeyboardKey::V,
            KeyboardKey::W,
            KeyboardKey::X,
            KeyboardKey::Y,
            KeyboardKey::Z,
            // Numbers
            KeyboardKey::Key1,
            KeyboardKey::Key2,
            KeyboardKey::Key3,
            KeyboardKey::Key4,
            KeyboardKey::Key5,
            KeyboardKey::Key6,
            KeyboardKey::Key7,
            KeyboardKey::Key8,
            KeyboardKey::Key9,
            KeyboardKey::Key0,
            // Function keys
            KeyboardKey::F1,
            KeyboardKey::F2,
            KeyboardKey::F3,
            KeyboardKey::F4,
            KeyboardKey::F5,
            KeyboardKey::F6,
            KeyboardKey::F7,
            KeyboardKey::F8,
            KeyboardKey::F9,
            KeyboardKey::F10,
            KeyboardKey::F11,
            KeyboardKey::F12,
            // Modifiers
            KeyboardKey::ShiftLeft,
            KeyboardKey::ShiftRight,
            KeyboardKey::CtrlLeft,
            KeyboardKey::CtrlRight,
            KeyboardKey::AltLeft,
            KeyboardKey::AltRight,
            // Special
            KeyboardKey::Space,
            KeyboardKey::Enter,
            KeyboardKey::Escape,
            KeyboardKey::Tab,
            KeyboardKey::Backspace,
            // Arrow keys
            KeyboardKey::ArrowUp,
            KeyboardKey::ArrowDown,
            KeyboardKey::ArrowLeft,
            KeyboardKey::ArrowRight,
            // Other
            KeyboardKey::Insert,
            KeyboardKey::Delete,
            KeyboardKey::Home,
            KeyboardKey::End,
            KeyboardKey::PageUp,
            KeyboardKey::PageDown,
        ]
    }

    /// Returns the name of this key
    pub fn name(&self) -> &'static str {
        self.display_name()
    }

    /// Returns true if this is a letter key (A-Z)
    pub fn is_letter(&self) -> bool {
        matches!(
            self,
            KeyboardKey::A
                | KeyboardKey::B
                | KeyboardKey::C
                | KeyboardKey::D
                | KeyboardKey::E
                | KeyboardKey::F
                | KeyboardKey::G
                | KeyboardKey::H
                | KeyboardKey::I
                | KeyboardKey::J
                | KeyboardKey::K
                | KeyboardKey::L
                | KeyboardKey::M
                | KeyboardKey::N
                | KeyboardKey::O
                | KeyboardKey::P
                | KeyboardKey::Q
                | KeyboardKey::R
                | KeyboardKey::S
                | KeyboardKey::T
                | KeyboardKey::U
                | KeyboardKey::V
                | KeyboardKey::W
                | KeyboardKey::X
                | KeyboardKey::Y
                | KeyboardKey::Z
        )
    }

    /// Returns true if this is a number key (0-9)
    pub fn is_number(&self) -> bool {
        matches!(
            self,
            KeyboardKey::Key0
                | KeyboardKey::Key1
                | KeyboardKey::Key2
                | KeyboardKey::Key3
                | KeyboardKey::Key4
                | KeyboardKey::Key5
                | KeyboardKey::Key6
                | KeyboardKey::Key7
                | KeyboardKey::Key8
                | KeyboardKey::Key9
        )
    }

    /// Returns true if this is a function key (F1-F12)
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            KeyboardKey::F1
                | KeyboardKey::F2
                | KeyboardKey::F3
                | KeyboardKey::F4
                | KeyboardKey::F5
                | KeyboardKey::F6
                | KeyboardKey::F7
                | KeyboardKey::F8
                | KeyboardKey::F9
                | KeyboardKey::F10
                | KeyboardKey::F11
                | KeyboardKey::F12
        )
    }

    /// Returns true if this is a modifier key (Shift, Ctrl, Alt)
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            KeyboardKey::ShiftLeft
                | KeyboardKey::ShiftRight
                | KeyboardKey::CtrlLeft
                | KeyboardKey::CtrlRight
                | KeyboardKey::AltLeft
                | KeyboardKey::AltRight
        )
    }

    /// Returns true if this is an arrow key
    pub fn is_arrow(&self) -> bool {
        matches!(
            self,
            KeyboardKey::ArrowUp
                | KeyboardKey::ArrowDown
                | KeyboardKey::ArrowLeft
                | KeyboardKey::ArrowRight
        )
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            KeyboardKey::A => "A",
            KeyboardKey::B => "B",
            KeyboardKey::C => "C",
            KeyboardKey::D => "D",
            KeyboardKey::E => "E",
            KeyboardKey::F => "F",
            KeyboardKey::G => "G",
            KeyboardKey::H => "H",
            KeyboardKey::I => "I",
            KeyboardKey::J => "J",
            KeyboardKey::K => "K",
            KeyboardKey::L => "L",
            KeyboardKey::M => "M",
            KeyboardKey::N => "N",
            KeyboardKey::O => "O",
            KeyboardKey::P => "P",
            KeyboardKey::Q => "Q",
            KeyboardKey::R => "R",
            KeyboardKey::S => "S",
            KeyboardKey::T => "T",
            KeyboardKey::U => "U",
            KeyboardKey::V => "V",
            KeyboardKey::W => "W",
            KeyboardKey::X => "X",
            KeyboardKey::Y => "Y",
            KeyboardKey::Z => "Z",
            KeyboardKey::Key1 => "1",
            KeyboardKey::Key2 => "2",
            KeyboardKey::Key3 => "3",
            KeyboardKey::Key4 => "4",
            KeyboardKey::Key5 => "5",
            KeyboardKey::Key6 => "6",
            KeyboardKey::Key7 => "7",
            KeyboardKey::Key8 => "8",
            KeyboardKey::Key9 => "9",
            KeyboardKey::Key0 => "0",
            KeyboardKey::F1 => "F1",
            KeyboardKey::F2 => "F2",
            KeyboardKey::F3 => "F3",
            KeyboardKey::F4 => "F4",
            KeyboardKey::F5 => "F5",
            KeyboardKey::F6 => "F6",
            KeyboardKey::F7 => "F7",
            KeyboardKey::F8 => "F8",
            KeyboardKey::F9 => "F9",
            KeyboardKey::F10 => "F10",
            KeyboardKey::F11 => "F11",
            KeyboardKey::F12 => "F12",
            KeyboardKey::ShiftLeft => "L-Shift",
            KeyboardKey::ShiftRight => "R-Shift",
            KeyboardKey::CtrlLeft => "L-Ctrl",
            KeyboardKey::CtrlRight => "R-Ctrl",
            KeyboardKey::AltLeft => "L-Alt",
            KeyboardKey::AltRight => "R-Alt",
            KeyboardKey::Space => "Space",
            KeyboardKey::Enter => "Enter",
            KeyboardKey::Escape => "Esc",
            KeyboardKey::Tab => "Tab",
            KeyboardKey::Backspace => "Backspace",
            KeyboardKey::ArrowUp => "↑",
            KeyboardKey::ArrowDown => "↓",
            KeyboardKey::ArrowLeft => "←",
            KeyboardKey::ArrowRight => "→",
            KeyboardKey::Insert => "Ins",
            KeyboardKey::Delete => "Del",
            KeyboardKey::Home => "Home",
            KeyboardKey::End => "End",
            KeyboardKey::PageUp => "PgUp",
            KeyboardKey::PageDown => "PgDn",
        }
    }
}

/// Mouse button representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

impl std::fmt::Display for MouseButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "🖱️ {}", self.display_name())
    }
}

impl MouseButton {
    pub fn all() -> &'static [MouseButton] {
        &[
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Back,
            MouseButton::Forward,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            MouseButton::Left => "Left Click",
            MouseButton::Right => "Right Click",
            MouseButton::Middle => "Middle Click",
            MouseButton::Back => "Mouse 4",
            MouseButton::Forward => "Mouse 5",
        }
    }

    pub fn is_primary(&self) -> bool {
        matches!(self, MouseButton::Left | MouseButton::Right)
    }
}

/// Action binding configuration
#[derive(Debug, Clone)]
pub struct ActionBinding {
    pub name: String,
    pub category: ActionCategory,
    pub description: String,
    pub keyboard_primary: Option<KeyboardKey>,
    pub keyboard_secondary: Option<KeyboardKey>,
    pub mouse_button: Option<MouseButton>,
    pub gamepad_button: Option<GamepadButton>,
    pub is_hold: bool,
    pub is_axis: bool,
}

impl Default for ActionBinding {
    fn default() -> Self {
        Self {
            name: "Unnamed".to_string(),
            category: ActionCategory::Movement,
            description: String::new(),
            keyboard_primary: None,
            keyboard_secondary: None,
            mouse_button: None,
            gamepad_button: None,
            is_hold: false,
            is_axis: false,
        }
    }
}

/// Axis binding configuration
#[derive(Debug, Clone)]
pub struct AxisBinding {
    pub name: String,
    pub description: String,
    pub sensitivity: f32,
    pub deadzone: f32,
    pub invert: bool,
    pub smoothing: f32,
}

impl Default for AxisBinding {
    fn default() -> Self {
        Self {
            name: "Unnamed Axis".to_string(),
            description: String::new(),
            sensitivity: 1.0,
            deadzone: 0.15,
            invert: false,
            smoothing: 0.0,
        }
    }
}

/// Binding conflict info
#[derive(Debug, Clone)]
pub struct BindingConflict {
    pub action1: String,
    pub action2: String,
    pub device: InputDevice,
    pub binding: String,
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputTab {
    #[default]
    Actions,
    Axes,
    Gamepad,
    Testing,
    Presets,
}

impl std::fmt::Display for InputTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl InputTab {
    pub fn all() -> &'static [InputTab] {
        &[
            InputTab::Actions,
            InputTab::Axes,
            InputTab::Gamepad,
            InputTab::Testing,
            InputTab::Presets,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            InputTab::Actions => "Actions",
            InputTab::Axes => "Axes",
            InputTab::Gamepad => "Gamepad",
            InputTab::Testing => "Testing",
            InputTab::Presets => "Presets",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            InputTab::Actions => "[Zap]",
            InputTab::Axes => "↔️",
            InputTab::Gamepad => "[Gp]",
            InputTab::Testing => "[Test]",
            InputTab::Presets => "[List]",
        }
    }
}

/// Main Input Bindings Panel
pub struct InputBindingsPanel {
    // Tab state
    active_tab: InputTab,

    // Filter state
    device_filter: InputDevice,
    category_filter: Option<ActionCategory>,
    search_text: String,

    // Binding data
    actions: Vec<ActionBinding>,
    axes: Vec<AxisBinding>,
    conflicts: Vec<BindingConflict>,

    // Preset
    current_preset: BindingPreset,

    // Editing state
    editing_action: Option<usize>,
    waiting_for_input: bool,
    input_target: Option<InputTarget>,

    // Gamepad settings
    gamepad_connected: bool,
    gamepad_name: String,
    rumble_enabled: bool,
    rumble_intensity: f32,

    // Mouse settings
    mouse_sensitivity: f32,
    mouse_smoothing: f32,
    mouse_invert_y: bool,
    mouse_raw_input: bool,

    // Testing state
    last_input: String,
    input_history: Vec<String>,
    test_axis_values: [f32; 4], // Left X, Left Y, Right X, Right Y

    // Pending actions for external consumption
    pending_actions: Vec<InputBindingAction>,
}

impl Default for InputBindingsPanel {
    fn default() -> Self {
        Self {
            active_tab: InputTab::Actions,

            device_filter: InputDevice::All,
            category_filter: None,
            search_text: String::new(),

            actions: Self::default_actions(),
            axes: Self::default_axes(),
            conflicts: Vec::new(),

            current_preset: BindingPreset::Default,

            editing_action: None,
            waiting_for_input: false,
            input_target: None,

            gamepad_connected: false,
            gamepad_name: "No gamepad detected".to_string(),
            rumble_enabled: true,
            rumble_intensity: 1.0,

            mouse_sensitivity: 1.0,
            mouse_smoothing: 0.0,
            mouse_invert_y: false,
            mouse_raw_input: true,

            last_input: String::new(),
            input_history: Vec::new(),
            test_axis_values: [0.0; 4],

            pending_actions: Vec::new(),
        }
    }
}

impl InputBindingsPanel {
    /// Drains and returns all pending actions
    pub fn take_actions(&mut self) -> Vec<InputBindingAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns true if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Queues an action for external handling
    fn queue_action(&mut self, action: InputBindingAction) {
        self.pending_actions.push(action);
    }

    /// Returns the rumble settings
    pub fn rumble_settings(&self) -> (bool, f32) {
        (self.rumble_enabled, self.rumble_intensity)
    }
}

/// What input we're waiting for
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputTarget {
    KeyboardPrimary(usize),
    KeyboardSecondary(usize),
    MouseButton(usize),
    GamepadButton(usize),
}

impl std::fmt::Display for InputTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl InputTarget {
    /// Returns all target type variant names
    pub fn all_variants() -> &'static [&'static str] {
        &[
            "KeyboardPrimary",
            "KeyboardSecondary",
            "MouseButton",
            "GamepadButton",
        ]
    }

    /// Returns the name of this input target type
    pub fn name(&self) -> &'static str {
        match self {
            InputTarget::KeyboardPrimary(_) => "Primary Key",
            InputTarget::KeyboardSecondary(_) => "Secondary Key",
            InputTarget::MouseButton(_) => "Mouse Button",
            InputTarget::GamepadButton(_) => "Gamepad Button",
        }
    }

    /// Returns the icon for this input target type
    pub fn icon(&self) -> &'static str {
        match self {
            InputTarget::KeyboardPrimary(_) => "⌨️",
            InputTarget::KeyboardSecondary(_) => "⌨️",
            InputTarget::MouseButton(_) => "🖱️",
            InputTarget::GamepadButton(_) => "[Gp]",
        }
    }

    /// Returns the action index this target refers to
    pub fn action_index(&self) -> usize {
        match self {
            InputTarget::KeyboardPrimary(i) => *i,
            InputTarget::KeyboardSecondary(i) => *i,
            InputTarget::MouseButton(i) => *i,
            InputTarget::GamepadButton(i) => *i,
        }
    }

    /// Returns true if this is a keyboard target
    pub fn is_keyboard(&self) -> bool {
        matches!(
            self,
            InputTarget::KeyboardPrimary(_) | InputTarget::KeyboardSecondary(_)
        )
    }

    /// Returns true if this is a mouse target
    pub fn is_mouse(&self) -> bool {
        matches!(self, InputTarget::MouseButton(_))
    }

    /// Returns true if this is a gamepad target
    pub fn is_gamepad(&self) -> bool {
        matches!(self, InputTarget::GamepadButton(_))
    }

    /// Returns true if this is a primary binding
    pub fn is_primary(&self) -> bool {
        matches!(self, InputTarget::KeyboardPrimary(_))
    }
}

impl InputBindingsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    fn default_actions() -> Vec<ActionBinding> {
        vec![
            // Movement
            ActionBinding {
                name: "Move Forward".to_string(),
                category: ActionCategory::Movement,
                description: "Move the character forward".to_string(),
                keyboard_primary: Some(KeyboardKey::W),
                gamepad_button: None,
                is_hold: true,
                ..Default::default()
            },
            ActionBinding {
                name: "Move Backward".to_string(),
                category: ActionCategory::Movement,
                description: "Move the character backward".to_string(),
                keyboard_primary: Some(KeyboardKey::S),
                is_hold: true,
                ..Default::default()
            },
            ActionBinding {
                name: "Move Left".to_string(),
                category: ActionCategory::Movement,
                description: "Strafe left".to_string(),
                keyboard_primary: Some(KeyboardKey::A),
                is_hold: true,
                ..Default::default()
            },
            ActionBinding {
                name: "Move Right".to_string(),
                category: ActionCategory::Movement,
                description: "Strafe right".to_string(),
                keyboard_primary: Some(KeyboardKey::D),
                is_hold: true,
                ..Default::default()
            },
            ActionBinding {
                name: "Jump".to_string(),
                category: ActionCategory::Movement,
                description: "Jump / climb".to_string(),
                keyboard_primary: Some(KeyboardKey::Space),
                gamepad_button: Some(GamepadButton::South),
                ..Default::default()
            },
            ActionBinding {
                name: "Crouch".to_string(),
                category: ActionCategory::Movement,
                description: "Crouch / slide".to_string(),
                keyboard_primary: Some(KeyboardKey::CtrlLeft),
                gamepad_button: Some(GamepadButton::RStick),
                is_hold: true,
                ..Default::default()
            },
            ActionBinding {
                name: "Sprint".to_string(),
                category: ActionCategory::Movement,
                description: "Run faster".to_string(),
                keyboard_primary: Some(KeyboardKey::ShiftLeft),
                gamepad_button: Some(GamepadButton::LStick),
                is_hold: true,
                ..Default::default()
            },
            // Combat
            ActionBinding {
                name: "Attack".to_string(),
                category: ActionCategory::Combat,
                description: "Primary attack".to_string(),
                mouse_button: Some(MouseButton::Left),
                gamepad_button: Some(GamepadButton::R2),
                ..Default::default()
            },
            ActionBinding {
                name: "Aim".to_string(),
                category: ActionCategory::Combat,
                description: "Aim down sights / block".to_string(),
                mouse_button: Some(MouseButton::Right),
                gamepad_button: Some(GamepadButton::L2),
                is_hold: true,
                ..Default::default()
            },
            ActionBinding {
                name: "Reload".to_string(),
                category: ActionCategory::Combat,
                description: "Reload weapon".to_string(),
                keyboard_primary: Some(KeyboardKey::R),
                gamepad_button: Some(GamepadButton::West),
                ..Default::default()
            },
            ActionBinding {
                name: "Ability 1".to_string(),
                category: ActionCategory::Combat,
                description: "Use first ability".to_string(),
                keyboard_primary: Some(KeyboardKey::Q),
                gamepad_button: Some(GamepadButton::L1),
                ..Default::default()
            },
            ActionBinding {
                name: "Ability 2".to_string(),
                category: ActionCategory::Combat,
                description: "Use second ability".to_string(),
                keyboard_primary: Some(KeyboardKey::E),
                gamepad_button: Some(GamepadButton::R1),
                ..Default::default()
            },
            // Interaction
            ActionBinding {
                name: "Interact".to_string(),
                category: ActionCategory::Interaction,
                description: "Interact with objects/NPCs".to_string(),
                keyboard_primary: Some(KeyboardKey::F),
                gamepad_button: Some(GamepadButton::East),
                ..Default::default()
            },
            ActionBinding {
                name: "Use Item".to_string(),
                category: ActionCategory::Interaction,
                description: "Use equipped item".to_string(),
                keyboard_primary: Some(KeyboardKey::G),
                gamepad_button: Some(GamepadButton::DPadUp),
                ..Default::default()
            },
            // UI
            ActionBinding {
                name: "Inventory".to_string(),
                category: ActionCategory::UI,
                description: "Open inventory".to_string(),
                keyboard_primary: Some(KeyboardKey::I),
                keyboard_secondary: Some(KeyboardKey::Tab),
                gamepad_button: Some(GamepadButton::Select),
                ..Default::default()
            },
            ActionBinding {
                name: "Map".to_string(),
                category: ActionCategory::UI,
                description: "Open map".to_string(),
                keyboard_primary: Some(KeyboardKey::M),
                gamepad_button: Some(GamepadButton::DPadDown),
                ..Default::default()
            },
            ActionBinding {
                name: "Pause".to_string(),
                category: ActionCategory::UI,
                description: "Pause game".to_string(),
                keyboard_primary: Some(KeyboardKey::Escape),
                gamepad_button: Some(GamepadButton::Start),
                ..Default::default()
            },
        ]
    }

    fn default_axes() -> Vec<AxisBinding> {
        vec![
            AxisBinding {
                name: "Move X".to_string(),
                description: "Left stick horizontal (strafe)".to_string(),
                sensitivity: 1.0,
                deadzone: 0.15,
                invert: false,
                smoothing: 0.0,
            },
            AxisBinding {
                name: "Move Y".to_string(),
                description: "Left stick vertical (forward/back)".to_string(),
                sensitivity: 1.0,
                deadzone: 0.15,
                invert: false,
                smoothing: 0.0,
            },
            AxisBinding {
                name: "Look X".to_string(),
                description: "Right stick horizontal (turn)".to_string(),
                sensitivity: 2.0,
                deadzone: 0.1,
                invert: false,
                smoothing: 0.1,
            },
            AxisBinding {
                name: "Look Y".to_string(),
                description: "Right stick vertical (pitch)".to_string(),
                sensitivity: 2.0,
                deadzone: 0.1,
                invert: false,
                smoothing: 0.1,
            },
        ]
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (InputTab::Actions, "⌨️ Actions"),
                (InputTab::Axes, "🕹️ Axes"),
                (InputTab::Gamepad, "Gamepad"),
                (InputTab::Testing, "[Test] Testing"),
                (InputTab::Presets, "Presets"),
            ];

            for (tab, label) in tabs {
                let is_selected = self.active_tab == tab;
                let button = egui::Button::new(label).fill(if is_selected {
                    Color32::from_rgb(60, 100, 160)
                } else {
                    Color32::from_rgb(50, 50, 55)
                });

                if ui.add(button).clicked() {
                    self.active_tab = tab;
                }
            }
        });

        // Preset indicator
        ui.horizontal(|ui| {
            ui.label(format!("Preset: {:?}", self.current_preset));
            if !self.conflicts.is_empty() {
                ui.colored_label(
                    Color32::YELLOW,
                    format!("[!] {} conflicts", self.conflicts.len()),
                );
            }
            if self.gamepad_connected {
                ui.label("Connected");
            }
        });

        ui.separator();
    }

    fn show_actions_tab(&mut self, ui: &mut Ui) {
        ui.heading("⌨️ Action Bindings");
        ui.add_space(5.0);

        // Track deferred clear action (to avoid borrow conflicts)
        let mut clear_action_idx: Option<usize> = None;

        // Filters
        ui.horizontal(|ui| {
            ui.label("Filter:");

            // Device filter
            for device in InputDevice::all() {
                if ui
                    .selectable_label(
                        self.device_filter == *device,
                        format!("{} {:?}", device.icon(), device),
                    )
                    .clicked()
                {
                    self.device_filter = *device;
                }
            }

            ui.separator();

            // Search
            ui.label("[Srch]");
            ui.text_edit_singleline(&mut self.search_text);
        });

        // Category filter
        ui.horizontal(|ui| {
            ui.label("Category:");
            if ui
                .selectable_label(self.category_filter.is_none(), "All")
                .clicked()
            {
                self.category_filter = None;
            }
            for cat in ActionCategory::all() {
                if ui
                    .selectable_label(
                        self.category_filter == Some(*cat),
                        format!("{} {:?}", cat.icon(), cat),
                    )
                    .clicked()
                {
                    self.category_filter = Some(*cat);
                }
            }
        });

        ui.add_space(10.0);

        // Actions list
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                let filtered_indices: Vec<usize> = self
                    .actions
                    .iter()
                    .enumerate()
                    .filter(|(_, action)| {
                        // Category filter
                        if let Some(cat) = self.category_filter {
                            if action.category != cat {
                                return false;
                            }
                        }
                        // Search filter
                        if !self.search_text.is_empty()
                            && !action
                                .name
                                .to_lowercase()
                                .contains(&self.search_text.to_lowercase())
                        {
                            return false;
                        }
                        true
                    })
                    .map(|(i, _)| i)
                    .collect();

                for idx in filtered_indices {
                    let action = &self.actions[idx];
                    let is_editing = self.editing_action == Some(idx);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(action.category.icon());
                            ui.label(RichText::new(&action.name).strong());

                            if action.is_hold {
                                ui.label(RichText::new("[HOLD]").small().color(Color32::GRAY));
                            }
                        });

                        ui.horizontal(|ui| {
                            // Keyboard primary
                            let kb_text = action
                                .keyboard_primary
                                .map(|k| k.display_name())
                                .unwrap_or("---");
                            if ui.button(format!("⌨️ {}", kb_text)).clicked() {
                                self.editing_action = Some(idx);
                                self.waiting_for_input = true;
                                self.input_target = Some(InputTarget::KeyboardPrimary(idx));
                            }

                            // Keyboard secondary
                            let kb2_text = action
                                .keyboard_secondary
                                .map(|k| k.display_name())
                                .unwrap_or("---");
                            if ui.button(format!("⌨️ {}", kb2_text)).clicked() {
                                self.editing_action = Some(idx);
                                self.waiting_for_input = true;
                                self.input_target = Some(InputTarget::KeyboardSecondary(idx));
                            }

                            // Mouse
                            let mouse_text = action
                                .mouse_button
                                .map(|m| m.display_name())
                                .unwrap_or("---");
                            if ui.button(format!("🖱️ {}", mouse_text)).clicked() {
                                self.editing_action = Some(idx);
                                self.waiting_for_input = true;
                                self.input_target = Some(InputTarget::MouseButton(idx));
                            }

                            // Gamepad
                            let gp_text = action
                                .gamepad_button
                                .map(|g| g.display_name())
                                .unwrap_or("---");
                            if ui.button(format!("{}", gp_text)).clicked() {
                                self.editing_action = Some(idx);
                                self.waiting_for_input = true;
                                self.input_target = Some(InputTarget::GamepadButton(idx));
                            }

                            // Clear button
                            if ui.small_button("[Del]").clicked() {
                                info!(action_index = idx, "Input: clearing binding");
                                clear_action_idx = Some(idx);
                            }
                        });

                        if is_editing && self.waiting_for_input {
                            ui.colored_label(Color32::YELLOW, "... Press a key/button...");
                        }
                    });
                }
            });

        // Handle clear action outside borrow scope
        if let Some(idx) = clear_action_idx {
            if let Some(action) = self.actions.get_mut(idx) {
                action.keyboard_primary = None;
                action.keyboard_secondary = None;
                action.mouse_button = None;
                action.gamepad_button = None;
            }
            self.queue_action(InputBindingAction::ClearBindings { action_index: idx });
        }
    }

    fn show_axes_tab(&mut self, ui: &mut Ui) {
        ui.heading("🕹️ Axis Configuration");
        ui.add_space(10.0);

        // Track previous values for change detection
        let prev_sensitivity = self.mouse_sensitivity;

        // Mouse settings
        ui.group(|ui| {
            ui.label(RichText::new("🖱️ Mouse Settings").strong());

            ui.add(
                egui::Slider::new(&mut self.mouse_sensitivity, 0.1..=5.0)
                    .text("Sensitivity")
                    .logarithmic(true),
            );
            ui.add(egui::Slider::new(&mut self.mouse_smoothing, 0.0..=1.0).text("Smoothing"));
            ui.checkbox(&mut self.mouse_invert_y, "Invert Y axis");
            ui.checkbox(&mut self.mouse_raw_input, "Raw input (no acceleration)");
        });

        // Queue action if sensitivity changed
        if (self.mouse_sensitivity - prev_sensitivity).abs() > 0.001 {
            self.queue_action(InputBindingAction::SetMouseSensitivity {
                sensitivity: self.mouse_sensitivity,
            });
        }

        ui.add_space(10.0);

        // Gamepad axes
        ui.group(|ui| {
            ui.label(RichText::new("Gamepad Axes").strong());

            for axis in &mut self.axes {
                ui.collapsing(&axis.name, |ui| {
                    ui.label(&axis.description);

                    ui.add(egui::Slider::new(&mut axis.sensitivity, 0.1..=5.0).text("Sensitivity"));
                    ui.add(egui::Slider::new(&mut axis.deadzone, 0.0..=0.5).text("Deadzone"));
                    ui.add(egui::Slider::new(&mut axis.smoothing, 0.0..=1.0).text("Smoothing"));
                    ui.checkbox(&mut axis.invert, "Invert");
                });
            }
        });
    }

    fn show_gamepad_tab(&mut self, ui: &mut Ui) {
        ui.heading("Gamepad Settings");
        ui.add_space(10.0);

        // Track previous values for change detection
        let prev_rumble_enabled = self.rumble_enabled;
        let prev_rumble_intensity = self.rumble_intensity;

        // Connection status
        ui.group(|ui| {
            ui.label(RichText::new("Connection Status").strong());

            if self.gamepad_connected {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::GREEN, "[G] Connected");
                    ui.label(&self.gamepad_name);
                });
            } else {
                ui.colored_label(Color32::GRAY, "⚫ No gamepad detected");
                ui.label("Connect a gamepad to configure it.");
            }

            if ui.button("Refresh").clicked() {
                // Refresh gamepad detection
            }
        });

        ui.add_space(10.0);

        // Rumble settings
        ui.group(|ui| {
            ui.label(RichText::new("Vibration / Rumble").strong());

            ui.checkbox(&mut self.rumble_enabled, "Enable rumble");

            if self.rumble_enabled {
                ui.add(egui::Slider::new(&mut self.rumble_intensity, 0.0..=1.0).text("Intensity"));

                ui.horizontal(|ui| {
                    if ui.button("Test Light").clicked() {
                        // Test light rumble - would trigger rumble effect
                    }
                    if ui.button("Test Heavy").clicked() {
                        // Test heavy rumble - would trigger rumble effect
                    }
                });
            }
        });

        // Queue action if rumble settings changed
        if self.rumble_enabled != prev_rumble_enabled
            || (self.rumble_intensity - prev_rumble_intensity).abs() > 0.001
        {
            self.queue_action(InputBindingAction::SetRumble {
                enabled: self.rumble_enabled,
                intensity: self.rumble_intensity,
            });
        }

        ui.add_space(10.0);

        // Gamepad visualization
        ui.group(|ui| {
            ui.label(RichText::new("Gamepad Visualization").strong());

            self.draw_gamepad_diagram(ui);
        });
    }

    fn show_testing_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Test] Input Testing");
        ui.add_space(10.0);

        // Last input
        ui.group(|ui| {
            ui.label(RichText::new("Last Input").strong());

            if self.last_input.is_empty() {
                ui.label("Press any key/button to test...");
            } else {
                ui.label(
                    RichText::new(&self.last_input)
                        .size(24.0)
                        .color(Color32::from_rgb(100, 200, 255)),
                );
            }
        });

        ui.add_space(10.0);

        // Axis values
        ui.group(|ui| {
            ui.label(RichText::new("Axis Values").strong());

            egui::Grid::new("axis_values_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Left Stick X:");
                    ui.add(
                        egui::ProgressBar::new((self.test_axis_values[0] + 1.0) / 2.0)
                            .show_percentage(),
                    );
                    ui.end_row();

                    ui.label("Left Stick Y:");
                    ui.add(
                        egui::ProgressBar::new((self.test_axis_values[1] + 1.0) / 2.0)
                            .show_percentage(),
                    );
                    ui.end_row();

                    ui.label("Right Stick X:");
                    ui.add(
                        egui::ProgressBar::new((self.test_axis_values[2] + 1.0) / 2.0)
                            .show_percentage(),
                    );
                    ui.end_row();

                    ui.label("Right Stick Y:");
                    ui.add(
                        egui::ProgressBar::new((self.test_axis_values[3] + 1.0) / 2.0)
                            .show_percentage(),
                    );
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Input history
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Input History").strong());
                if ui.button("Clear").clicked() {
                    self.input_history.clear();
                }
            });

            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    for entry in self.input_history.iter().rev().take(20) {
                        ui.label(entry);
                    }
                    if self.input_history.is_empty() {
                        ui.label("No input recorded yet.");
                    }
                });
        });
    }

    fn show_presets_tab(&mut self, ui: &mut Ui) {
        ui.heading("Binding Presets");
        ui.add_space(10.0);

        // Track selected preset for action queuing
        let mut preset_changed: Option<BindingPreset> = None;
        let mut save_bindings = false;
        let mut load_bindings = false;
        let mut reset_defaults = false;

        // Current preset
        ui.group(|ui| {
            ui.label(RichText::new("Current Preset").strong());

            ui.horizontal_wrapped(|ui| {
                for preset in BindingPreset::all() {
                    if ui
                        .selectable_label(self.current_preset == *preset, format!("{:?}", preset))
                        .clicked()
                    {
                        preset_changed = Some(*preset);
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Preset description
        ui.group(|ui| {
            ui.label(RichText::new("Preset Details").strong());

            let description = match self.current_preset {
                BindingPreset::Default => "Standard WASD + Mouse controls",
                BindingPreset::FPS => "Optimized for first-person shooters",
                BindingPreset::ThirdPerson => "Optimized for third-person action games",
                BindingPreset::RTS => "Strategy game controls (QWERTY hotkeys)",
                BindingPreset::Racing => "Driving controls (arrows/triggers)",
                BindingPreset::LeftHanded => "Mirror layout for left-handed players",
                BindingPreset::Custom => "User-defined custom bindings",
            };

            ui.label(description);
        });

        ui.add_space(10.0);

        // Preset management
        ui.group(|ui| {
            ui.label(RichText::new("Management").strong());

            ui.horizontal(|ui| {
                if ui.button("Save as Custom").clicked() {
                    save_bindings = true;
                }
                if ui.button("Import").clicked() {
                    load_bindings = true;
                }
                if ui.button("📤 Export").clicked() {
                    save_bindings = true;
                }
                if ui.button("Reset to Default").clicked() {
                    reset_defaults = true;
                }
            });
        });

        // Handle deferred actions
        if let Some(preset) = preset_changed {
            info!(preset = ?preset, "Input: applied binding preset");
            self.current_preset = preset;
            self.apply_preset(preset);
            self.queue_action(InputBindingAction::ApplyPreset { preset });
        }
        if save_bindings {
            info!("Input: saving bindings");
            self.current_preset = BindingPreset::Custom;
            self.queue_action(InputBindingAction::SaveBindings);
        }
        if load_bindings {
            info!("Input: loading bindings");
            self.queue_action(InputBindingAction::LoadBindings);
        }
        if reset_defaults {
            self.current_preset = BindingPreset::Default;
            self.apply_preset(BindingPreset::Default);
            self.queue_action(InputBindingAction::ResetToDefaults);
        }

        ui.add_space(10.0);

        // Conflicts
        if !self.conflicts.is_empty() {
            ui.group(|ui| {
                ui.label(
                    RichText::new("Binding Conflicts")
                        .strong()
                        .color(Color32::YELLOW),
                );

                for conflict in &self.conflicts {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} ↔ {} ({} {})",
                            conflict.action1,
                            conflict.action2,
                            conflict.device.icon(),
                            conflict.binding
                        ));
                    });
                }
            });
        }
    }

    fn draw_gamepad_diagram(&self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width().min(300.0), 150.0),
            egui::Sense::hover(),
        );

        let center = rect.center();
        let painter = ui.painter();

        // Draw gamepad outline
        painter.rect_stroke(
            rect.shrink(10.0),
            20.0,
            egui::Stroke::new(2.0, Color32::GRAY),
            egui::StrokeKind::Outside,
        );

        // Left stick
        let left_stick_center = egui::Pos2::new(center.x - 60.0, center.y);
        painter.circle_stroke(
            left_stick_center,
            25.0,
            egui::Stroke::new(2.0, Color32::WHITE),
        );
        let stick_pos = egui::Pos2::new(
            left_stick_center.x + self.test_axis_values[0] * 20.0,
            left_stick_center.y + self.test_axis_values[1] * 20.0,
        );
        painter.circle_filled(stick_pos, 10.0, Color32::from_rgb(100, 150, 255));

        // Right stick
        let right_stick_center = egui::Pos2::new(center.x + 60.0, center.y + 20.0);
        painter.circle_stroke(
            right_stick_center,
            25.0,
            egui::Stroke::new(2.0, Color32::WHITE),
        );
        let stick_pos = egui::Pos2::new(
            right_stick_center.x + self.test_axis_values[2] * 20.0,
            right_stick_center.y + self.test_axis_values[3] * 20.0,
        );
        painter.circle_filled(stick_pos, 10.0, Color32::from_rgb(100, 150, 255));

        // D-Pad
        let dpad_center = egui::Pos2::new(center.x - 60.0, center.y + 40.0);
        painter.rect_filled(
            egui::Rect::from_center_size(dpad_center, Vec2::new(40.0, 15.0)),
            2.0,
            Color32::DARK_GRAY,
        );
        painter.rect_filled(
            egui::Rect::from_center_size(dpad_center, Vec2::new(15.0, 40.0)),
            2.0,
            Color32::DARK_GRAY,
        );

        // Face buttons
        let buttons_center = egui::Pos2::new(center.x + 60.0, center.y - 25.0);
        let button_positions = [
            (0.0, 15.0, "A", Color32::GREEN),   // South
            (15.0, 0.0, "B", Color32::RED),     // East
            (-15.0, 0.0, "X", Color32::BLUE),   // West
            (0.0, -15.0, "Y", Color32::YELLOW), // North
        ];

        for (dx, dy, label, color) in button_positions {
            let pos = egui::Pos2::new(buttons_center.x + dx, buttons_center.y + dy);
            painter.circle_filled(pos, 10.0, color);
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
    }

    fn apply_preset(&mut self, preset: BindingPreset) {
        match preset {
            BindingPreset::Default | BindingPreset::FPS | BindingPreset::ThirdPerson => {
                self.actions = Self::default_actions();
                self.axes = Self::default_axes();
            }
            BindingPreset::LeftHanded => {
                self.actions = Self::default_actions();
                // Swap WASD to IJKL, etc.
                for action in &mut self.actions {
                    action.keyboard_primary = match action.keyboard_primary {
                        Some(KeyboardKey::W) => Some(KeyboardKey::I),
                        Some(KeyboardKey::A) => Some(KeyboardKey::J),
                        Some(KeyboardKey::S) => Some(KeyboardKey::K),
                        Some(KeyboardKey::D) => Some(KeyboardKey::L),
                        other => other,
                    };
                }
            }
            _ => {}
        }
    }

    // Getters for testing
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    pub fn axis_count(&self) -> usize {
        self.axes.len()
    }

    pub fn current_preset(&self) -> BindingPreset {
        self.current_preset
    }

    pub fn device_filter(&self) -> InputDevice {
        self.device_filter
    }

    pub fn is_gamepad_connected(&self) -> bool {
        self.gamepad_connected
    }

    pub fn mouse_sensitivity(&self) -> f32 {
        self.mouse_sensitivity
    }

    pub fn set_preset(&mut self, preset: BindingPreset) {
        self.current_preset = preset;
        self.apply_preset(preset);
    }

    pub fn set_device_filter(&mut self, filter: InputDevice) {
        self.device_filter = filter;
    }

    pub fn set_gamepad_connected(&mut self, connected: bool) {
        self.gamepad_connected = connected;
    }

    pub fn set_mouse_sensitivity(&mut self, sensitivity: f32) {
        self.mouse_sensitivity = sensitivity;
    }

    pub fn record_input(&mut self, input: &str) {
        self.last_input = input.to_string();
        self.input_history.push(input.to_string());
    }

    pub fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }

    pub fn add_conflict(&mut self, conflict: BindingConflict) {
        self.conflicts.push(conflict);
    }
}

impl Panel for InputBindingsPanel {
    fn name(&self) -> &'static str {
        "Input Bindings"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            InputTab::Actions => self.show_actions_tab(ui),
            InputTab::Axes => self.show_axes_tab(ui),
            InputTab::Gamepad => self.show_gamepad_tab(ui),
            InputTab::Testing => self.show_testing_tab(ui),
            InputTab::Presets => self.show_presets_tab(ui),
        }
    }

    fn update(&mut self) {
        // Cancel waiting for input after timeout
        // Poll for gamepad state
        // Update axis visualization
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_bindings_panel_creation() {
        let panel = InputBindingsPanel::new();
        assert_eq!(panel.current_preset(), BindingPreset::Default);
        assert_eq!(panel.device_filter(), InputDevice::All);
    }

    #[test]
    fn test_default_actions() {
        let panel = InputBindingsPanel::new();
        assert!(panel.action_count() > 10); // Should have many default actions
    }

    #[test]
    fn test_default_axes() {
        let panel = InputBindingsPanel::new();
        assert_eq!(panel.axis_count(), 4); // Move X/Y, Look X/Y
    }

    #[test]
    fn test_preset_switching() {
        let mut panel = InputBindingsPanel::new();
        panel.set_preset(BindingPreset::LeftHanded);
        assert_eq!(panel.current_preset(), BindingPreset::LeftHanded);
    }

    #[test]
    fn test_device_filter() {
        let mut panel = InputBindingsPanel::new();
        panel.set_device_filter(InputDevice::Gamepad);
        assert_eq!(panel.device_filter(), InputDevice::Gamepad);
    }

    #[test]
    fn test_gamepad_connection() {
        let mut panel = InputBindingsPanel::new();
        assert!(!panel.is_gamepad_connected());

        panel.set_gamepad_connected(true);
        assert!(panel.is_gamepad_connected());
    }

    #[test]
    fn test_mouse_sensitivity() {
        let mut panel = InputBindingsPanel::new();
        assert_eq!(panel.mouse_sensitivity(), 1.0);

        panel.set_mouse_sensitivity(2.5);
        assert_eq!(panel.mouse_sensitivity(), 2.5);
    }

    #[test]
    fn test_input_recording() {
        let mut panel = InputBindingsPanel::new();
        panel.record_input("Space pressed");
        assert_eq!(panel.last_input, "Space pressed");
        assert_eq!(panel.input_history.len(), 1);
    }

    #[test]
    fn test_conflict_tracking() {
        let mut panel = InputBindingsPanel::new();
        assert_eq!(panel.conflict_count(), 0);

        panel.add_conflict(BindingConflict {
            action1: "Jump".to_string(),
            action2: "Crouch".to_string(),
            device: InputDevice::Keyboard,
            binding: "Space".to_string(),
        });
        assert_eq!(panel.conflict_count(), 1);
    }

    #[test]
    fn test_gamepad_button_names() {
        assert_eq!(GamepadButton::South.display_name(), "A / ✕");
        assert_eq!(GamepadButton::L1.display_name(), "LB / L1");
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = InputBindingsPanel::new();
        assert_eq!(panel.name(), "Input Bindings");
    }

    // === InputDevice Display and Hash Tests ===

    #[test]
    fn test_input_device_display() {
        for device in InputDevice::all() {
            let display = format!("{}", device);
            assert!(display.contains(device.name()));
        }
    }

    #[test]
    fn test_input_device_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for device in InputDevice::all() {
            set.insert(*device);
        }
        assert_eq!(set.len(), InputDevice::all().len());
    }

    #[test]
    fn test_input_device_all() {
        let all = InputDevice::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&InputDevice::Keyboard));
        assert!(all.contains(&InputDevice::Mouse));
        assert!(all.contains(&InputDevice::Gamepad));
        assert!(all.contains(&InputDevice::All));
    }

    #[test]
    fn test_input_device_is_physical() {
        assert!(InputDevice::Keyboard.is_physical());
        assert!(InputDevice::Mouse.is_physical());
        assert!(InputDevice::Gamepad.is_physical());
        assert!(!InputDevice::All.is_physical());
    }

    // === ActionCategory Display and Hash Tests ===

    #[test]
    fn test_action_category_display() {
        for cat in ActionCategory::all() {
            let display = format!("{}", cat);
            assert!(display.contains(cat.name()));
        }
    }

    #[test]
    fn test_action_category_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for cat in ActionCategory::all() {
            set.insert(*cat);
        }
        assert_eq!(set.len(), ActionCategory::all().len());
    }

    #[test]
    fn test_action_category_all() {
        let all = ActionCategory::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&ActionCategory::Movement));
        assert!(all.contains(&ActionCategory::Combat));
        assert!(all.contains(&ActionCategory::Debug));
    }

    #[test]
    fn test_action_category_is_gameplay() {
        assert!(ActionCategory::Movement.is_gameplay());
        assert!(ActionCategory::Combat.is_gameplay());
        assert!(ActionCategory::Vehicle.is_gameplay());
        assert!(!ActionCategory::UI.is_gameplay());
        assert!(!ActionCategory::Camera.is_gameplay());
        assert!(!ActionCategory::Debug.is_gameplay());
    }

    // === BindingPreset Display and Hash Tests ===

    #[test]
    fn test_binding_preset_display() {
        for preset in BindingPreset::all() {
            let display = format!("{}", preset);
            assert!(display.contains(preset.name()));
        }
    }

    #[test]
    fn test_binding_preset_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for preset in BindingPreset::all() {
            set.insert(*preset);
        }
        assert_eq!(set.len(), BindingPreset::all().len());
    }

    #[test]
    fn test_binding_preset_all() {
        let all = BindingPreset::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&BindingPreset::Default));
        assert!(all.contains(&BindingPreset::FPS));
        assert!(all.contains(&BindingPreset::Custom));
    }

    #[test]
    fn test_binding_preset_is_built_in() {
        assert!(BindingPreset::Default.is_built_in());
        assert!(BindingPreset::FPS.is_built_in());
        assert!(BindingPreset::LeftHanded.is_built_in());
        assert!(!BindingPreset::Custom.is_built_in());
    }

    // === GamepadButton Display and Hash Tests ===

    #[test]
    fn test_gamepad_button_display() {
        for btn in GamepadButton::all() {
            let display = format!("{}", btn);
            assert!(display.contains(btn.display_name()));
        }
    }

    #[test]
    fn test_gamepad_button_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for btn in GamepadButton::all() {
            set.insert(*btn);
        }
        assert_eq!(set.len(), GamepadButton::all().len());
    }

    #[test]
    fn test_gamepad_button_all() {
        let all = GamepadButton::all();
        assert_eq!(all.len(), 16);
        assert!(all.contains(&GamepadButton::South));
        assert!(all.contains(&GamepadButton::Start));
    }

    #[test]
    fn test_gamepad_button_is_face_button() {
        assert!(GamepadButton::South.is_face_button());
        assert!(GamepadButton::East.is_face_button());
        assert!(GamepadButton::West.is_face_button());
        assert!(GamepadButton::North.is_face_button());
        assert!(!GamepadButton::L1.is_face_button());
        assert!(!GamepadButton::DPadUp.is_face_button());
    }

    #[test]
    fn test_gamepad_button_is_shoulder() {
        assert!(GamepadButton::L1.is_shoulder());
        assert!(GamepadButton::R1.is_shoulder());
        assert!(GamepadButton::L2.is_shoulder());
        assert!(GamepadButton::R2.is_shoulder());
        assert!(!GamepadButton::South.is_shoulder());
    }

    #[test]
    fn test_gamepad_button_is_dpad() {
        assert!(GamepadButton::DPadUp.is_dpad());
        assert!(GamepadButton::DPadDown.is_dpad());
        assert!(GamepadButton::DPadLeft.is_dpad());
        assert!(GamepadButton::DPadRight.is_dpad());
        assert!(!GamepadButton::Start.is_dpad());
    }

    // === MouseButton Display and Hash Tests ===

    #[test]
    fn test_mouse_button_display() {
        for btn in MouseButton::all() {
            let display = format!("{}", btn);
            assert!(display.contains(btn.display_name()));
        }
    }

    #[test]
    fn test_mouse_button_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for btn in MouseButton::all() {
            set.insert(*btn);
        }
        assert_eq!(set.len(), MouseButton::all().len());
    }

    #[test]
    fn test_mouse_button_all() {
        let all = MouseButton::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&MouseButton::Left));
        assert!(all.contains(&MouseButton::Right));
        assert!(all.contains(&MouseButton::Forward));
    }

    #[test]
    fn test_mouse_button_is_primary() {
        assert!(MouseButton::Left.is_primary());
        assert!(MouseButton::Right.is_primary());
        assert!(!MouseButton::Middle.is_primary());
        assert!(!MouseButton::Back.is_primary());
    }

    // === InputTab Display and Hash Tests ===

    #[test]
    fn test_input_tab_display() {
        for tab in InputTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()));
        }
    }

    #[test]
    fn test_input_tab_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in InputTab::all() {
            set.insert(*tab);
        }
        assert_eq!(set.len(), InputTab::all().len());
    }

    #[test]
    fn test_input_tab_all() {
        let all = InputTab::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&InputTab::Actions));
        assert!(all.contains(&InputTab::Axes));
        assert!(all.contains(&InputTab::Gamepad));
        assert!(all.contains(&InputTab::Testing));
        assert!(all.contains(&InputTab::Presets));
    }

    // === KeyboardKey Display and Helper Tests ===

    #[test]
    fn test_keyboard_key_display() {
        for key in KeyboardKey::all() {
            let display = format!("{}", key);
            assert!(display.contains(key.name()));
        }
    }

    #[test]
    fn test_keyboard_key_all() {
        let all = KeyboardKey::all();
        assert_eq!(all.len(), 69); // 26 letters + 10 numbers + 12 F-keys + 6 modifiers + 5 special + 4 arrows + 6 other
    }

    #[test]
    fn test_keyboard_key_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for key in KeyboardKey::all() {
            set.insert(*key);
        }
        assert_eq!(set.len(), KeyboardKey::all().len());
    }

    #[test]
    fn test_keyboard_key_is_letter() {
        assert!(KeyboardKey::A.is_letter());
        assert!(KeyboardKey::Z.is_letter());
        assert!(KeyboardKey::M.is_letter());
        assert!(!KeyboardKey::Key1.is_letter());
        assert!(!KeyboardKey::F1.is_letter());
        assert!(!KeyboardKey::Space.is_letter());
    }

    #[test]
    fn test_keyboard_key_is_number() {
        assert!(KeyboardKey::Key0.is_number());
        assert!(KeyboardKey::Key5.is_number());
        assert!(KeyboardKey::Key9.is_number());
        assert!(!KeyboardKey::A.is_number());
        assert!(!KeyboardKey::F1.is_number());
    }

    #[test]
    fn test_keyboard_key_is_function() {
        assert!(KeyboardKey::F1.is_function());
        assert!(KeyboardKey::F12.is_function());
        assert!(KeyboardKey::F6.is_function());
        assert!(!KeyboardKey::A.is_function());
        assert!(!KeyboardKey::Key1.is_function());
    }

    #[test]
    fn test_keyboard_key_is_modifier() {
        assert!(KeyboardKey::ShiftLeft.is_modifier());
        assert!(KeyboardKey::CtrlRight.is_modifier());
        assert!(KeyboardKey::AltLeft.is_modifier());
        assert!(!KeyboardKey::A.is_modifier());
        assert!(!KeyboardKey::Space.is_modifier());
    }

    #[test]
    fn test_keyboard_key_is_arrow() {
        assert!(KeyboardKey::ArrowUp.is_arrow());
        assert!(KeyboardKey::ArrowDown.is_arrow());
        assert!(KeyboardKey::ArrowLeft.is_arrow());
        assert!(KeyboardKey::ArrowRight.is_arrow());
        assert!(!KeyboardKey::W.is_arrow());
    }

    #[test]
    fn test_keyboard_key_name() {
        assert_eq!(KeyboardKey::Space.name(), "Space");
        assert_eq!(KeyboardKey::Enter.name(), "Enter");
        assert_eq!(KeyboardKey::ArrowUp.name(), "↑");
    }

    // === InputTarget Display and Helper Tests ===

    #[test]
    fn test_input_target_display() {
        let target = InputTarget::KeyboardPrimary(0);
        let display = format!("{}", target);
        assert!(display.contains(target.name()));
    }

    #[test]
    fn test_input_target_all_variants() {
        let all = InputTarget::all_variants();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&"KeyboardPrimary"));
        assert!(all.contains(&"KeyboardSecondary"));
        assert!(all.contains(&"MouseButton"));
        assert!(all.contains(&"GamepadButton"));
    }

    #[test]
    fn test_input_target_names() {
        let kb_primary = InputTarget::KeyboardPrimary(0);
        assert_eq!(kb_primary.name(), "Primary Key");

        let kb_secondary = InputTarget::KeyboardSecondary(1);
        assert_eq!(kb_secondary.name(), "Secondary Key");

        let mouse = InputTarget::MouseButton(2);
        assert_eq!(mouse.name(), "Mouse Button");

        let gamepad = InputTarget::GamepadButton(3);
        assert_eq!(gamepad.name(), "Gamepad Button");
    }

    #[test]
    fn test_input_target_icons() {
        let kb = InputTarget::KeyboardPrimary(0);
        assert_eq!(kb.icon(), "⌨️");

        let mouse = InputTarget::MouseButton(0);
        assert_eq!(mouse.icon(), "🖱️");

        let gamepad = InputTarget::GamepadButton(0);
        assert_eq!(gamepad.icon(), "[Gp]");
    }

    #[test]
    fn test_input_target_action_index() {
        assert_eq!(InputTarget::KeyboardPrimary(5).action_index(), 5);
        assert_eq!(InputTarget::KeyboardSecondary(10).action_index(), 10);
        assert_eq!(InputTarget::MouseButton(3).action_index(), 3);
        assert_eq!(InputTarget::GamepadButton(7).action_index(), 7);
    }

    #[test]
    fn test_input_target_is_keyboard() {
        assert!(InputTarget::KeyboardPrimary(0).is_keyboard());
        assert!(InputTarget::KeyboardSecondary(0).is_keyboard());
        assert!(!InputTarget::MouseButton(0).is_keyboard());
        assert!(!InputTarget::GamepadButton(0).is_keyboard());
    }

    #[test]
    fn test_input_target_is_mouse() {
        assert!(InputTarget::MouseButton(0).is_mouse());
        assert!(!InputTarget::KeyboardPrimary(0).is_mouse());
        assert!(!InputTarget::GamepadButton(0).is_mouse());
    }

    #[test]
    fn test_input_target_is_gamepad() {
        assert!(InputTarget::GamepadButton(0).is_gamepad());
        assert!(!InputTarget::KeyboardPrimary(0).is_gamepad());
        assert!(!InputTarget::MouseButton(0).is_gamepad());
    }

    #[test]
    fn test_input_target_is_primary() {
        assert!(InputTarget::KeyboardPrimary(0).is_primary());
        assert!(!InputTarget::KeyboardSecondary(0).is_primary());
        assert!(!InputTarget::MouseButton(0).is_primary());
    }

    #[test]
    fn test_input_target_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(InputTarget::KeyboardPrimary(0));
        set.insert(InputTarget::KeyboardSecondary(0));
        set.insert(InputTarget::MouseButton(0));
        set.insert(InputTarget::GamepadButton(0));
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_input_target_partial_eq() {
        let t1 = InputTarget::KeyboardPrimary(5);
        let t2 = InputTarget::KeyboardPrimary(5);
        let t3 = InputTarget::KeyboardPrimary(10);
        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
    }

    // === Action System Tests ===

    #[test]
    fn test_action_queue_initially_empty() {
        let panel = InputBindingsPanel::new();
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_take_actions_drains_queue() {
        let mut panel = InputBindingsPanel::new();
        // Simulate an action by directly pushing (testing the mechanism)
        panel.pending_actions.push(InputBindingAction::SaveBindings);
        panel.pending_actions.push(InputBindingAction::LoadBindings);

        assert!(panel.has_pending_actions());
        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
    }

    #[test]
    fn test_clear_bindings_action() {
        let action = InputBindingAction::ClearBindings { action_index: 5 };
        assert_eq!(
            action,
            InputBindingAction::ClearBindings { action_index: 5 }
        );
    }

    #[test]
    fn test_apply_preset_action() {
        let action = InputBindingAction::ApplyPreset {
            preset: BindingPreset::FPS,
        };
        assert_eq!(
            action,
            InputBindingAction::ApplyPreset {
                preset: BindingPreset::FPS
            }
        );
    }

    #[test]
    fn test_set_mouse_sensitivity_action() {
        let action = InputBindingAction::SetMouseSensitivity { sensitivity: 2.5 };
        if let InputBindingAction::SetMouseSensitivity { sensitivity } = action {
            assert!((sensitivity - 2.5).abs() < f32::EPSILON);
        } else {
            panic!("Wrong action type");
        }
    }

    #[test]
    fn test_set_rumble_action() {
        let action = InputBindingAction::SetRumble {
            enabled: true,
            intensity: 0.75,
        };
        if let InputBindingAction::SetRumble { enabled, intensity } = action {
            assert!(enabled);
            assert!((intensity - 0.75).abs() < f32::EPSILON);
        } else {
            panic!("Wrong action type");
        }
    }

    #[test]
    fn test_set_binding_action() {
        let action = InputBindingAction::SetBinding {
            action_index: 3,
            target: InputTarget::KeyboardPrimary(3),
            key: "Space".to_string(),
        };
        if let InputBindingAction::SetBinding {
            action_index,
            target,
            key,
        } = action
        {
            assert_eq!(action_index, 3);
            assert_eq!(target, InputTarget::KeyboardPrimary(3));
            assert_eq!(key, "Space");
        } else {
            panic!("Wrong action type");
        }
    }

    #[test]
    fn test_rumble_settings_accessor() {
        let panel = InputBindingsPanel::new();
        let (enabled, intensity) = panel.rumble_settings();
        assert!(enabled); // Default is enabled
        assert!((intensity - 1.0).abs() < f32::EPSILON); // Default is 1.0
    }
}
