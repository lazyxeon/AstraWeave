use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputContext {
    Gameplay,
    UI,
}

impl InputContext {
    /// Returns the name of this context.
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Gameplay => "Gameplay",
            Self::UI => "UI",
        }
    }

    /// Returns true if this is gameplay context.
    #[inline]
    pub fn is_gameplay(&self) -> bool {
        matches!(self, Self::Gameplay)
    }

    /// Returns true if this is UI context.
    #[inline]
    pub fn is_ui(&self) -> bool {
        matches!(self, Self::UI)
    }

    /// Returns all contexts.
    pub fn all() -> [InputContext; 2] {
        [Self::Gameplay, Self::UI]
    }
}

impl std::fmt::Display for InputContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Action {
    // Movement / Camera
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Crouch,
    Sprint,
    Interact,
    AttackLight,
    AttackHeavy,
    Ability1,
    Ability2,

    // UI toggles
    OpenInventory,
    OpenMap,
    OpenQuests,
    OpenCrafting,
    OpenMenu,

    // UI navigation (for controller)
    UiAccept,
    UiBack,
    UiUp,
    UiDown,
    UiLeft,
    UiRight,
}

impl Action {
    /// Returns the name of this action.
    pub fn name(&self) -> &'static str {
        match self {
            Self::MoveForward => "MoveForward",
            Self::MoveBackward => "MoveBackward",
            Self::MoveLeft => "MoveLeft",
            Self::MoveRight => "MoveRight",
            Self::Jump => "Jump",
            Self::Crouch => "Crouch",
            Self::Sprint => "Sprint",
            Self::Interact => "Interact",
            Self::AttackLight => "AttackLight",
            Self::AttackHeavy => "AttackHeavy",
            Self::Ability1 => "Ability1",
            Self::Ability2 => "Ability2",
            Self::OpenInventory => "OpenInventory",
            Self::OpenMap => "OpenMap",
            Self::OpenQuests => "OpenQuests",
            Self::OpenCrafting => "OpenCrafting",
            Self::OpenMenu => "OpenMenu",
            Self::UiAccept => "UiAccept",
            Self::UiBack => "UiBack",
            Self::UiUp => "UiUp",
            Self::UiDown => "UiDown",
            Self::UiLeft => "UiLeft",
            Self::UiRight => "UiRight",
        }
    }

    /// Returns true if this is a movement action.
    #[inline]
    pub fn is_movement(&self) -> bool {
        matches!(
            self,
            Self::MoveForward | Self::MoveBackward | Self::MoveLeft | Self::MoveRight
        )
    }

    /// Returns true if this is an attack action.
    #[inline]
    pub fn is_attack(&self) -> bool {
        matches!(self, Self::AttackLight | Self::AttackHeavy)
    }

    /// Returns true if this is an ability action.
    #[inline]
    pub fn is_ability(&self) -> bool {
        matches!(self, Self::Ability1 | Self::Ability2)
    }

    /// Returns true if this is a UI toggle action.
    #[inline]
    pub fn is_ui_toggle(&self) -> bool {
        matches!(
            self,
            Self::OpenInventory
                | Self::OpenMap
                | Self::OpenQuests
                | Self::OpenCrafting
                | Self::OpenMenu
        )
    }

    /// Returns true if this is a UI navigation action.
    #[inline]
    pub fn is_ui_nav(&self) -> bool {
        matches!(
            self,
            Self::UiAccept
                | Self::UiBack
                | Self::UiUp
                | Self::UiDown
                | Self::UiLeft
                | Self::UiRight
        )
    }

    /// Returns true if this is a gameplay action.
    #[inline]
    pub fn is_gameplay(&self) -> bool {
        !self.is_ui_nav()
    }

    /// Returns the context this action belongs to.
    #[inline]
    pub fn context(&self) -> InputContext {
        if self.is_ui_nav() {
            InputContext::UI
        } else {
            InputContext::Gameplay
        }
    }

    /// Returns all movement actions.
    pub fn movement_actions() -> [Action; 4] {
        [
            Self::MoveForward,
            Self::MoveBackward,
            Self::MoveLeft,
            Self::MoveRight,
        ]
    }

    /// Returns all attack actions.
    pub fn attack_actions() -> [Action; 2] {
        [Self::AttackLight, Self::AttackHeavy]
    }

    /// Returns all UI navigation actions.
    pub fn ui_nav_actions() -> [Action; 6] {
        [
            Self::UiAccept,
            Self::UiBack,
            Self::UiUp,
            Self::UiDown,
            Self::UiLeft,
            Self::UiRight,
        ]
    }

    /// Returns all actions.
    pub fn all() -> [Action; 23] {
        [
            Self::MoveForward,
            Self::MoveBackward,
            Self::MoveLeft,
            Self::MoveRight,
            Self::Jump,
            Self::Crouch,
            Self::Sprint,
            Self::Interact,
            Self::AttackLight,
            Self::AttackHeavy,
            Self::Ability1,
            Self::Ability2,
            Self::OpenInventory,
            Self::OpenMap,
            Self::OpenQuests,
            Self::OpenCrafting,
            Self::OpenMenu,
            Self::UiAccept,
            Self::UiBack,
            Self::UiUp,
            Self::UiDown,
            Self::UiLeft,
            Self::UiRight,
        ]
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Axis2 {
    pub x: f32,
    pub y: f32,
}

impl Axis2 {
    /// Creates a new axis with the given values.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Creates a zero axis.
    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Returns the length of the axis vector.
    #[inline]
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Returns the squared length of the axis vector.
    #[inline]
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Returns true if the axis is within the deadzone.
    #[inline]
    pub fn is_in_deadzone(&self, deadzone: f32) -> bool {
        self.length() < deadzone
    }

    /// Returns true if the axis is zero (or very close).
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.length_squared() < 1e-10
    }

    /// Returns a normalized version of the axis.
    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self::zero()
        }
    }

    /// Clamps the axis length to a maximum value.
    pub fn clamped(&self, max_length: f32) -> Self {
        let len = self.length();
        if len > max_length {
            let scale = max_length / len;
            Self {
                x: self.x * scale,
                y: self.y * scale,
            }
        } else {
            self.clone()
        }
    }

    /// Applies a deadzone, returning zero if within deadzone.
    pub fn with_deadzone(&self, deadzone: f32) -> Self {
        if self.is_in_deadzone(deadzone) {
            Self::zero()
        } else {
            self.clone()
        }
    }

    /// Returns the angle in radians from positive X axis.
    #[inline]
    pub fn angle(&self) -> f32 {
        self.y.atan2(self.x)
    }
}

impl std::fmt::Display for Axis2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Axis2({:.2}, {:.2})", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== InputContext Tests =====

    #[test]
    fn test_input_context_name() {
        assert_eq!(InputContext::Gameplay.name(), "Gameplay");
        assert_eq!(InputContext::UI.name(), "UI");
    }

    #[test]
    fn test_input_context_is_gameplay() {
        assert!(InputContext::Gameplay.is_gameplay());
        assert!(!InputContext::UI.is_gameplay());
    }

    #[test]
    fn test_input_context_is_ui() {
        assert!(!InputContext::Gameplay.is_ui());
        assert!(InputContext::UI.is_ui());
    }

    #[test]
    fn test_input_context_all() {
        let all = InputContext::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&InputContext::Gameplay));
        assert!(all.contains(&InputContext::UI));
    }

    #[test]
    fn test_input_context_display() {
        assert_eq!(format!("{}", InputContext::Gameplay), "Gameplay");
        assert_eq!(format!("{}", InputContext::UI), "UI");
    }

    // ===== Action Tests =====

    #[test]
    fn test_action_name() {
        assert_eq!(Action::MoveForward.name(), "MoveForward");
        assert_eq!(Action::Jump.name(), "Jump");
        assert_eq!(Action::UiAccept.name(), "UiAccept");
    }

    #[test]
    fn test_action_is_movement() {
        assert!(Action::MoveForward.is_movement());
        assert!(Action::MoveBackward.is_movement());
        assert!(Action::MoveLeft.is_movement());
        assert!(Action::MoveRight.is_movement());
        assert!(!Action::Jump.is_movement());
        assert!(!Action::UiUp.is_movement());
    }

    #[test]
    fn test_action_is_attack() {
        assert!(Action::AttackLight.is_attack());
        assert!(Action::AttackHeavy.is_attack());
        assert!(!Action::Jump.is_attack());
        assert!(!Action::Ability1.is_attack());
    }

    #[test]
    fn test_action_is_ability() {
        assert!(Action::Ability1.is_ability());
        assert!(Action::Ability2.is_ability());
        assert!(!Action::AttackLight.is_ability());
    }

    #[test]
    fn test_action_is_ui_toggle() {
        assert!(Action::OpenInventory.is_ui_toggle());
        assert!(Action::OpenMap.is_ui_toggle());
        assert!(Action::OpenQuests.is_ui_toggle());
        assert!(Action::OpenCrafting.is_ui_toggle());
        assert!(Action::OpenMenu.is_ui_toggle());
        assert!(!Action::UiAccept.is_ui_toggle());
    }

    #[test]
    fn test_action_is_ui_nav() {
        assert!(Action::UiAccept.is_ui_nav());
        assert!(Action::UiBack.is_ui_nav());
        assert!(Action::UiUp.is_ui_nav());
        assert!(Action::UiDown.is_ui_nav());
        assert!(Action::UiLeft.is_ui_nav());
        assert!(Action::UiRight.is_ui_nav());
        assert!(!Action::Jump.is_ui_nav());
    }

    #[test]
    fn test_action_is_gameplay() {
        assert!(Action::Jump.is_gameplay());
        assert!(Action::MoveForward.is_gameplay());
        assert!(Action::OpenMenu.is_gameplay());
        assert!(!Action::UiAccept.is_gameplay());
    }

    #[test]
    fn test_action_context() {
        assert_eq!(Action::Jump.context(), InputContext::Gameplay);
        assert_eq!(Action::MoveForward.context(), InputContext::Gameplay);
        assert_eq!(Action::UiAccept.context(), InputContext::UI);
        assert_eq!(Action::UiUp.context(), InputContext::UI);
    }

    #[test]
    fn test_action_movement_actions() {
        let actions = Action::movement_actions();
        assert_eq!(actions.len(), 4);
        assert!(actions.contains(&Action::MoveForward));
    }

    #[test]
    fn test_action_attack_actions() {
        let actions = Action::attack_actions();
        assert_eq!(actions.len(), 2);
        assert!(actions.contains(&Action::AttackLight));
        assert!(actions.contains(&Action::AttackHeavy));
    }

    #[test]
    fn test_action_ui_nav_actions() {
        let actions = Action::ui_nav_actions();
        assert_eq!(actions.len(), 6);
        for a in &actions {
            assert!(a.is_ui_nav());
        }
    }

    #[test]
    fn test_action_all() {
        let all = Action::all();
        assert_eq!(all.len(), 23);
    }

    #[test]
    fn test_action_display() {
        assert_eq!(format!("{}", Action::Jump), "Jump");
        assert_eq!(format!("{}", Action::UiAccept), "UiAccept");
    }

    // ===== Axis2 Tests =====

    #[test]
    fn test_axis2_new() {
        let axis = Axis2::new(1.0, 2.0);
        assert_eq!(axis.x, 1.0);
        assert_eq!(axis.y, 2.0);
    }

    #[test]
    fn test_axis2_zero() {
        let axis = Axis2::zero();
        assert_eq!(axis.x, 0.0);
        assert_eq!(axis.y, 0.0);
    }

    #[test]
    fn test_axis2_length() {
        let axis = Axis2::new(3.0, 4.0);
        assert!((axis.length() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_axis2_length_squared() {
        let axis = Axis2::new(3.0, 4.0);
        assert!((axis.length_squared() - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_axis2_is_in_deadzone() {
        let axis = Axis2::new(0.05, 0.05);
        assert!(axis.is_in_deadzone(0.1));
        assert!(!axis.is_in_deadzone(0.05));
    }

    #[test]
    fn test_axis2_is_zero() {
        let zero = Axis2::zero();
        assert!(zero.is_zero());

        let nonzero = Axis2::new(0.1, 0.0);
        assert!(!nonzero.is_zero());
    }

    #[test]
    fn test_axis2_normalized() {
        let axis = Axis2::new(3.0, 4.0);
        let norm = axis.normalized();
        assert!((norm.length() - 1.0).abs() < 0.001);
        assert!((norm.x - 0.6).abs() < 0.001);
        assert!((norm.y - 0.8).abs() < 0.001);

        let zero = Axis2::zero();
        let zero_norm = zero.normalized();
        assert!(zero_norm.is_zero());
    }

    #[test]
    fn test_axis2_clamped() {
        let axis = Axis2::new(3.0, 4.0); // length 5
        let clamped = axis.clamped(2.0);
        assert!((clamped.length() - 2.0).abs() < 0.001);

        let short = Axis2::new(0.5, 0.0);
        let not_clamped = short.clamped(2.0);
        assert!((not_clamped.x - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_axis2_with_deadzone() {
        let axis = Axis2::new(0.05, 0.05);
        let with_dz = axis.with_deadzone(0.1);
        assert!(with_dz.is_zero());

        let outside = Axis2::new(0.5, 0.0);
        let no_change = outside.with_deadzone(0.1);
        assert!((no_change.x - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_axis2_angle() {
        let right = Axis2::new(1.0, 0.0);
        assert!(right.angle().abs() < 0.001);

        let up = Axis2::new(0.0, 1.0);
        assert!((up.angle() - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    }

    #[test]
    fn test_axis2_display() {
        let axis = Axis2::new(1.5, 2.5);
        let display = format!("{}", axis);
        assert!(display.contains("Axis2"));
        assert!(display.contains("1.50"));
        assert!(display.contains("2.50"));
    }

    #[test]
    fn test_axis2_default() {
        let axis = Axis2::default();
        assert!(axis.is_zero());
    }
}
