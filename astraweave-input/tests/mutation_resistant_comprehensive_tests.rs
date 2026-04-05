//! Mutation-resistant comprehensive tests for astraweave-input.
//! Targets exact return values, boundary conditions, off-by-one errors,
//! negation swaps, and operator mutations for 90%+ kill rate.

use astraweave_input::*;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

// ========================================================================
// INPUT CONTEXT
// ========================================================================

#[test]
fn context_gameplay_name() {
    assert_eq!(InputContext::Gameplay.name(), "Gameplay");
}

#[test]
fn context_ui_name() {
    assert_eq!(InputContext::UI.name(), "UI");
}

#[test]
fn context_is_gameplay_true_for_gameplay() {
    assert!(InputContext::Gameplay.is_gameplay());
}

#[test]
fn context_is_gameplay_false_for_ui() {
    assert!(!InputContext::UI.is_gameplay());
}

#[test]
fn context_is_ui_true_for_ui() {
    assert!(InputContext::UI.is_ui());
}

#[test]
fn context_is_ui_false_for_gameplay() {
    assert!(!InputContext::Gameplay.is_ui());
}

#[test]
fn context_all_returns_both() {
    let all = InputContext::all();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0], InputContext::Gameplay);
    assert_eq!(all[1], InputContext::UI);
}

#[test]
fn context_display_gameplay() {
    assert_eq!(format!("{}", InputContext::Gameplay), "Gameplay");
}

#[test]
fn context_display_ui() {
    assert_eq!(format!("{}", InputContext::UI), "UI");
}

#[test]
fn context_clone_eq() {
    let c = InputContext::Gameplay;
    let c2 = c;
    assert_eq!(c, c2);
}

// ========================================================================
// ACTION ENUM
// ========================================================================

#[test]
fn action_all_returns_23() {
    assert_eq!(Action::all().len(), 23);
}

#[test]
fn action_movement_actions_exactly_4() {
    let m = Action::movement_actions();
    assert_eq!(m.len(), 4);
    assert!(m.contains(&Action::MoveForward));
    assert!(m.contains(&Action::MoveBackward));
    assert!(m.contains(&Action::MoveLeft));
    assert!(m.contains(&Action::MoveRight));
}

#[test]
fn action_attack_actions_exactly_2() {
    let a = Action::attack_actions();
    assert_eq!(a.len(), 2);
    assert!(a.contains(&Action::AttackLight));
    assert!(a.contains(&Action::AttackHeavy));
}

#[test]
fn action_ui_nav_actions_exactly_6() {
    let u = Action::ui_nav_actions();
    assert_eq!(u.len(), 6);
    assert!(u.contains(&Action::UiAccept));
    assert!(u.contains(&Action::UiBack));
    assert!(u.contains(&Action::UiUp));
    assert!(u.contains(&Action::UiDown));
    assert!(u.contains(&Action::UiLeft));
    assert!(u.contains(&Action::UiRight));
}

#[test]
fn action_is_movement_positive_cases() {
    assert!(Action::MoveForward.is_movement());
    assert!(Action::MoveBackward.is_movement());
    assert!(Action::MoveLeft.is_movement());
    assert!(Action::MoveRight.is_movement());
}

#[test]
fn action_is_movement_negative_cases() {
    assert!(!Action::Jump.is_movement());
    assert!(!Action::AttackLight.is_movement());
    assert!(!Action::UiAccept.is_movement());
    assert!(!Action::OpenInventory.is_movement());
}

#[test]
fn action_is_attack_positive() {
    assert!(Action::AttackLight.is_attack());
    assert!(Action::AttackHeavy.is_attack());
}

#[test]
fn action_is_attack_negative() {
    assert!(!Action::MoveForward.is_attack());
    assert!(!Action::Ability1.is_attack());
}

#[test]
fn action_is_ability_positive() {
    assert!(Action::Ability1.is_ability());
    assert!(Action::Ability2.is_ability());
}

#[test]
fn action_is_ability_negative() {
    assert!(!Action::AttackLight.is_ability());
    assert!(!Action::Jump.is_ability());
}

#[test]
fn action_is_ui_toggle_positive() {
    assert!(Action::OpenInventory.is_ui_toggle());
    assert!(Action::OpenMap.is_ui_toggle());
    assert!(Action::OpenQuests.is_ui_toggle());
    assert!(Action::OpenCrafting.is_ui_toggle());
    assert!(Action::OpenMenu.is_ui_toggle());
}

#[test]
fn action_is_ui_toggle_negative() {
    assert!(!Action::UiAccept.is_ui_toggle());
    assert!(!Action::MoveForward.is_ui_toggle());
}

#[test]
fn action_is_ui_nav_positive() {
    assert!(Action::UiAccept.is_ui_nav());
    assert!(Action::UiBack.is_ui_nav());
    assert!(Action::UiUp.is_ui_nav());
    assert!(Action::UiDown.is_ui_nav());
    assert!(Action::UiLeft.is_ui_nav());
    assert!(Action::UiRight.is_ui_nav());
}

#[test]
fn action_is_ui_nav_negative() {
    assert!(!Action::Jump.is_ui_nav());
    assert!(!Action::OpenMenu.is_ui_nav());
}

#[test]
fn action_is_gameplay_true_for_non_ui_nav() {
    // is_gameplay = !is_ui_nav()
    assert!(Action::MoveForward.is_gameplay());
    assert!(Action::Jump.is_gameplay());
    assert!(Action::OpenInventory.is_gameplay());
}

#[test]
fn action_is_gameplay_false_for_ui_nav() {
    assert!(!Action::UiAccept.is_gameplay());
    assert!(!Action::UiBack.is_gameplay());
}

#[test]
fn action_context_gameplay_for_non_nav() {
    assert_eq!(Action::MoveForward.context(), InputContext::Gameplay);
    assert_eq!(Action::Jump.context(), InputContext::Gameplay);
    assert_eq!(Action::OpenMenu.context(), InputContext::Gameplay);
}

#[test]
fn action_context_ui_for_nav() {
    assert_eq!(Action::UiAccept.context(), InputContext::UI);
    assert_eq!(Action::UiBack.context(), InputContext::UI);
}

#[test]
fn action_names_non_empty() {
    for a in Action::all() {
        assert!(!a.name().is_empty(), "Action {:?} has empty name", a);
    }
}

#[test]
fn action_specific_names() {
    assert_eq!(Action::MoveForward.name(), "MoveForward");
    assert_eq!(Action::AttackLight.name(), "AttackLight");
    assert_eq!(Action::UiAccept.name(), "UiAccept");
}

// ========================================================================
// AXIS2
// ========================================================================

#[test]
fn axis2_zero_is_origin() {
    let a = Axis2::zero();
    assert_eq!(a.x, 0.0);
    assert_eq!(a.y, 0.0);
}

#[test]
fn axis2_default_is_zero() {
    let a = Axis2::default();
    assert_eq!(a.x, 0.0);
    assert_eq!(a.y, 0.0);
}

#[test]
fn axis2_new_stores_values() {
    let a = Axis2::new(1.5, -2.5);
    assert_eq!(a.x, 1.5);
    assert_eq!(a.y, -2.5);
}

#[test]
fn axis2_length_unit_x() {
    let a = Axis2::new(1.0, 0.0);
    assert!((a.length() - 1.0).abs() < 1e-6);
}

#[test]
fn axis2_length_3_4_5_triangle() {
    let a = Axis2::new(3.0, 4.0);
    assert!((a.length() - 5.0).abs() < 1e-6);
}

#[test]
fn axis2_length_squared_no_sqrt() {
    let a = Axis2::new(3.0, 4.0);
    assert!((a.length_squared() - 25.0).abs() < 1e-6);
}

#[test]
fn axis2_is_zero_at_origin() {
    assert!(Axis2::zero().is_zero());
}

#[test]
fn axis2_is_zero_tiny_value_below_threshold() {
    // threshold is length_squared < 1e-10
    let a = Axis2::new(1e-6, 0.0); // length_squared = 1e-12 < 1e-10
    assert!(a.is_zero());
}

#[test]
fn axis2_is_zero_above_threshold() {
    let a = Axis2::new(1e-4, 0.0); // length_squared = 1e-8 > 1e-10
    assert!(!a.is_zero());
}

#[test]
fn axis2_is_in_deadzone_strict_less_than() {
    // is_in_deadzone uses strict < : length < deadzone
    let a = Axis2::new(0.15, 0.0); // length = 0.15, deadzone = 0.15
    assert!(
        !a.is_in_deadzone(0.15),
        "exactly at deadzone is NOT in deadzone (strict <)"
    );
}

#[test]
fn axis2_is_in_deadzone_below() {
    let a = Axis2::new(0.14, 0.0);
    assert!(a.is_in_deadzone(0.15), "below deadzone IS in deadzone");
}

#[test]
fn axis2_is_in_deadzone_zero() {
    assert!(Axis2::zero().is_in_deadzone(0.15));
}

#[test]
fn axis2_is_in_deadzone_above() {
    let a = Axis2::new(0.5, 0.0);
    assert!(!a.is_in_deadzone(0.15));
}

#[test]
fn axis2_normalized_unit_length() {
    let a = Axis2::new(3.0, 4.0);
    let n = a.normalized();
    assert!((n.length() - 1.0).abs() < 1e-5);
    assert!((n.x - 0.6).abs() < 1e-5);
    assert!((n.y - 0.8).abs() < 1e-5);
}

#[test]
fn axis2_normalized_zero_returns_zero() {
    let n = Axis2::zero().normalized();
    assert!(n.is_zero());
}

#[test]
fn axis2_clamped_within_limit() {
    let a = Axis2::new(0.5, 0.0);
    let c = a.clamped(1.0);
    assert_eq!(c.x, 0.5);
    assert_eq!(c.y, 0.0);
}

#[test]
fn axis2_clamped_exceeds_limit() {
    let a = Axis2::new(3.0, 4.0); // length = 5.0
    let c = a.clamped(1.0); // scale to length 1.0
    assert!((c.length() - 1.0).abs() < 1e-5);
}

#[test]
fn axis2_clamped_exactly_at_limit() {
    let a = Axis2::new(1.0, 0.0);
    let c = a.clamped(1.0); // length == max, not > max
    assert_eq!(c.x, 1.0); // should not change
}

#[test]
fn axis2_with_deadzone_below_returns_zero() {
    let a = Axis2::new(0.1, 0.0);
    let d = a.with_deadzone(0.15);
    assert_eq!(d.x, 0.0);
    assert_eq!(d.y, 0.0);
}

#[test]
fn axis2_with_deadzone_above_passes_through() {
    let a = Axis2::new(0.5, 0.0);
    let d = a.with_deadzone(0.15);
    assert!(d.x > 0.0, "should pass through deadzone filter");
}

#[test]
fn axis2_angle_positive_x() {
    let a = Axis2::new(1.0, 0.0);
    assert!((a.angle() - 0.0).abs() < 1e-6);
}

#[test]
fn axis2_angle_positive_y() {
    let a = Axis2::new(0.0, 1.0);
    assert!((a.angle() - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
}

#[test]
fn axis2_display_format() {
    let a = Axis2::new(1.5, -2.5);
    let s = format!("{}", a);
    assert!(s.contains("1.5"), "display should contain x");
    assert!(
        s.contains("-2.5") || s.contains("2.5"),
        "display should contain y"
    );
}

// ========================================================================
// GAMEPAD BUTTON
// ========================================================================

#[test]
fn gamepad_all_returns_16() {
    assert_eq!(GamepadButton::all().len(), 16);
}

#[test]
fn gamepad_face_buttons_exactly_4() {
    let f = GamepadButton::face_buttons();
    assert_eq!(f.len(), 4);
    assert!(f.contains(&GamepadButton::South));
    assert!(f.contains(&GamepadButton::East));
    assert!(f.contains(&GamepadButton::West));
    assert!(f.contains(&GamepadButton::North));
}

#[test]
fn gamepad_shoulder_buttons_exactly_4() {
    let s = GamepadButton::shoulder_buttons();
    assert_eq!(s.len(), 4);
    assert!(s.contains(&GamepadButton::L1));
    assert!(s.contains(&GamepadButton::R1));
    assert!(s.contains(&GamepadButton::L2));
    assert!(s.contains(&GamepadButton::R2));
}

#[test]
fn gamepad_dpad_buttons_exactly_4() {
    let d = GamepadButton::dpad_buttons();
    assert_eq!(d.len(), 4);
}

#[test]
fn gamepad_is_face_positive() {
    assert!(GamepadButton::South.is_face());
    assert!(GamepadButton::East.is_face());
    assert!(GamepadButton::West.is_face());
    assert!(GamepadButton::North.is_face());
}

#[test]
fn gamepad_is_face_negative() {
    assert!(!GamepadButton::L1.is_face());
    assert!(!GamepadButton::DPadUp.is_face());
    assert!(!GamepadButton::Start.is_face());
}

#[test]
fn gamepad_is_shoulder_positive() {
    assert!(GamepadButton::L1.is_shoulder());
    assert!(GamepadButton::R1.is_shoulder());
    assert!(GamepadButton::L2.is_shoulder());
    assert!(GamepadButton::R2.is_shoulder());
}

#[test]
fn gamepad_is_shoulder_negative() {
    assert!(!GamepadButton::South.is_shoulder());
    assert!(!GamepadButton::DPadUp.is_shoulder());
}

#[test]
fn gamepad_is_trigger_positive() {
    assert!(GamepadButton::L2.is_trigger());
    assert!(GamepadButton::R2.is_trigger());
}

#[test]
fn gamepad_is_trigger_negative() {
    assert!(!GamepadButton::L1.is_trigger());
    assert!(!GamepadButton::R1.is_trigger()); // bumpers, not triggers
    assert!(!GamepadButton::South.is_trigger());
}

#[test]
fn gamepad_is_bumper_positive() {
    assert!(GamepadButton::L1.is_bumper());
    assert!(GamepadButton::R1.is_bumper());
}

#[test]
fn gamepad_is_bumper_negative() {
    assert!(!GamepadButton::L2.is_bumper());
    assert!(!GamepadButton::R2.is_bumper());
}

#[test]
fn gamepad_is_dpad_positive() {
    assert!(GamepadButton::DPadUp.is_dpad());
    assert!(GamepadButton::DPadDown.is_dpad());
    assert!(GamepadButton::DPadLeft.is_dpad());
    assert!(GamepadButton::DPadRight.is_dpad());
}

#[test]
fn gamepad_is_dpad_negative() {
    assert!(!GamepadButton::South.is_dpad());
    assert!(!GamepadButton::Start.is_dpad());
}

#[test]
fn gamepad_is_stick_positive() {
    assert!(GamepadButton::LStick.is_stick());
    assert!(GamepadButton::RStick.is_stick());
}

#[test]
fn gamepad_is_stick_negative() {
    assert!(!GamepadButton::L1.is_stick());
    assert!(!GamepadButton::South.is_stick());
}

#[test]
fn gamepad_is_system_positive() {
    assert!(GamepadButton::Select.is_system());
    assert!(GamepadButton::Start.is_system());
}

#[test]
fn gamepad_is_system_negative() {
    assert!(!GamepadButton::South.is_system());
    assert!(!GamepadButton::L1.is_system());
}

#[test]
fn gamepad_names_non_empty() {
    for b in GamepadButton::all() {
        assert!(!b.name().is_empty());
    }
}

// ========================================================================
// AXIS KIND
// ========================================================================

#[test]
fn axis_kind_all_returns_6() {
    assert_eq!(AxisKind::all().len(), 6);
}

#[test]
fn axis_kind_stick_axes_exactly_4() {
    let s = AxisKind::stick_axes();
    assert_eq!(s.len(), 4);
    assert!(s.contains(&AxisKind::LeftX));
    assert!(s.contains(&AxisKind::LeftY));
    assert!(s.contains(&AxisKind::RightX));
    assert!(s.contains(&AxisKind::RightY));
}

#[test]
fn axis_kind_trigger_axes_exactly_2() {
    let t = AxisKind::trigger_axes();
    assert_eq!(t.len(), 2);
    assert!(t.contains(&AxisKind::LT));
    assert!(t.contains(&AxisKind::RT));
}

#[test]
fn axis_kind_is_left_stick() {
    assert!(AxisKind::LeftX.is_left_stick());
    assert!(AxisKind::LeftY.is_left_stick());
    assert!(!AxisKind::RightX.is_left_stick());
    assert!(!AxisKind::LT.is_left_stick());
}

#[test]
fn axis_kind_is_right_stick() {
    assert!(AxisKind::RightX.is_right_stick());
    assert!(AxisKind::RightY.is_right_stick());
    assert!(!AxisKind::LeftX.is_right_stick());
    assert!(!AxisKind::RT.is_right_stick());
}

#[test]
fn axis_kind_is_stick() {
    assert!(AxisKind::LeftX.is_stick());
    assert!(AxisKind::RightY.is_stick());
    assert!(!AxisKind::LT.is_stick());
    assert!(!AxisKind::RT.is_stick());
}

#[test]
fn axis_kind_is_trigger() {
    assert!(AxisKind::LT.is_trigger());
    assert!(AxisKind::RT.is_trigger());
    assert!(!AxisKind::LeftX.is_trigger());
    assert!(!AxisKind::RightY.is_trigger());
}

#[test]
fn axis_kind_is_x_axis() {
    assert!(AxisKind::LeftX.is_x_axis());
    assert!(AxisKind::RightX.is_x_axis());
    assert!(!AxisKind::LeftY.is_x_axis());
    assert!(!AxisKind::LT.is_x_axis());
}

#[test]
fn axis_kind_is_y_axis() {
    assert!(AxisKind::LeftY.is_y_axis());
    assert!(AxisKind::RightY.is_y_axis());
    assert!(!AxisKind::LeftX.is_y_axis());
    assert!(!AxisKind::RT.is_y_axis());
}

#[test]
fn axis_kind_paired_xy() {
    assert_eq!(AxisKind::LeftX.paired(), Some(AxisKind::LeftY));
    assert_eq!(AxisKind::LeftY.paired(), Some(AxisKind::LeftX));
    assert_eq!(AxisKind::RightX.paired(), Some(AxisKind::RightY));
    assert_eq!(AxisKind::RightY.paired(), Some(AxisKind::RightX));
}

#[test]
fn axis_kind_paired_trigger_none() {
    assert_eq!(AxisKind::LT.paired(), None);
    assert_eq!(AxisKind::RT.paired(), None);
}

#[test]
fn axis_kind_names_non_empty() {
    for a in AxisKind::all() {
        assert!(!a.name().is_empty());
    }
}

// ========================================================================
// BINDING
// ========================================================================

#[test]
fn binding_new_is_empty() {
    let b = Binding::new();
    assert!(b.is_empty());
    assert_eq!(b.binding_count(), 0);
}

#[test]
fn binding_default_is_empty() {
    let b = Binding::default();
    assert!(b.is_empty());
}

#[test]
fn binding_with_key_sets_key() {
    let b = Binding::with_key(KeyCode::KeyW);
    assert!(b.has_key());
    assert!(!b.has_mouse());
    assert!(!b.has_gamepad());
    assert_eq!(b.binding_count(), 1);
    assert!(!b.is_empty());
}

#[test]
fn binding_with_mouse_sets_mouse() {
    let b = Binding::with_mouse(MouseButton::Left);
    assert!(!b.has_key());
    assert!(b.has_mouse());
    assert!(!b.has_gamepad());
    assert_eq!(b.binding_count(), 1);
}

#[test]
fn binding_with_gamepad_sets_gamepad() {
    let b = Binding::with_gamepad(GamepadButton::South);
    assert!(!b.has_key());
    assert!(!b.has_mouse());
    assert!(b.has_gamepad());
    assert_eq!(b.binding_count(), 1);
}

#[test]
fn binding_count_all_three() {
    let b = Binding {
        key: Some(KeyCode::KeyW),
        mouse: Some(MouseButton::Left),
        gamepad: Some(GamepadButton::South),
    };
    assert_eq!(b.binding_count(), 3);
    assert!(!b.is_empty());
}

#[test]
fn binding_clone_eq() {
    let b = Binding::with_key(KeyCode::Space);
    let b2 = b.clone();
    assert_eq!(b, b2);
}

// ========================================================================
// AXIS BINDING
// ========================================================================

#[test]
fn axis_binding_new_default_deadzone() {
    let ab = AxisBinding::new(AxisKind::LeftX);
    assert_eq!(ab.axis, AxisKind::LeftX);
    assert!(!ab.invert);
    assert!((ab.deadzone - 0.15).abs() < 1e-6, "default deadzone = 0.15");
}

#[test]
fn axis_binding_with_deadzone() {
    let ab = AxisBinding::with_deadzone(AxisKind::RightY, 0.25);
    assert_eq!(ab.axis, AxisKind::RightY);
    assert!((ab.deadzone - 0.25).abs() < 1e-6);
    assert!(!ab.invert);
}

#[test]
fn axis_binding_inverted() {
    let ab = AxisBinding::inverted(AxisKind::LeftY);
    assert!(ab.is_inverted());
    assert!(
        (ab.deadzone - 0.15).abs() < 1e-6,
        "inverted still uses 0.15 deadzone"
    );
}

#[test]
fn axis_binding_apply_below_deadzone_returns_zero() {
    let ab = AxisBinding::new(AxisKind::LeftX); // deadzone = 0.15
    assert_eq!(ab.apply(0.1), 0.0);
    assert_eq!(ab.apply(-0.1), 0.0);
}

#[test]
fn axis_binding_apply_exactly_at_deadzone_passes() {
    // apply uses strict < : abs_value < deadzone → 0.0
    // so exactly at deadzone passes through (not filtered)
    let ab = AxisBinding::new(AxisKind::LeftX); // deadzone = 0.15
    let result = ab.apply(0.15);
    assert!(result >= 0.0, "exactly at deadzone should pass (strict <)");
    // The remapped value: (0.15 - 0.15) / (1.0 - 0.15) = 0.0
    assert!(
        (result - 0.0).abs() < 1e-6,
        "output at deadzone boundary is ~0"
    );
}

#[test]
fn axis_binding_apply_above_deadzone_positive() {
    let ab = AxisBinding::new(AxisKind::LeftX);
    let result = ab.apply(0.5);
    assert!(
        result > 0.0,
        "above deadzone should produce positive output"
    );
}

#[test]
fn axis_binding_apply_full_positive() {
    let ab = AxisBinding::new(AxisKind::LeftX);
    let result = ab.apply(1.0);
    assert!(
        (result - 1.0).abs() < 1e-5,
        "full input should produce ~1.0 output"
    );
}

#[test]
fn axis_binding_apply_inverted_negates() {
    let ab = AxisBinding::inverted(AxisKind::LeftY);
    let normal = AxisBinding::new(AxisKind::LeftY);
    let result_inv = ab.apply(0.8);
    let result_norm = normal.apply(0.8);
    assert!(
        (result_inv + result_norm).abs() < 1e-5,
        "inverted should negate"
    );
}

#[test]
fn axis_binding_apply_negative_input() {
    let ab = AxisBinding::new(AxisKind::LeftX);
    let result = ab.apply(-0.8);
    assert!(result < 0.0, "negative input produces negative output");
}

// ========================================================================
// BINDING SET
// ========================================================================

#[test]
fn binding_set_new_is_empty() {
    let bs = BindingSet::new();
    assert_eq!(bs.action_count(), 0);
}

#[test]
fn binding_set_default_has_bindings() {
    let bs = BindingSet::default();
    assert!(bs.action_count() > 0, "default should have action bindings");
}

#[test]
fn binding_set_default_has_wasd() {
    let bs = BindingSet::default();
    let fwd = bs.get_binding(&Action::MoveForward).unwrap();
    assert_eq!(fwd.key, Some(KeyCode::KeyW));
    let back = bs.get_binding(&Action::MoveBackward).unwrap();
    assert_eq!(back.key, Some(KeyCode::KeyS));
    let left = bs.get_binding(&Action::MoveLeft).unwrap();
    assert_eq!(left.key, Some(KeyCode::KeyA));
    let right = bs.get_binding(&Action::MoveRight).unwrap();
    assert_eq!(right.key, Some(KeyCode::KeyD));
}

#[test]
fn binding_set_default_jump_space() {
    let bs = BindingSet::default();
    let j = bs.get_binding(&Action::Jump).unwrap();
    assert_eq!(j.key, Some(KeyCode::Space));
}

#[test]
fn binding_set_default_attack_mouse() {
    let bs = BindingSet::default();
    let light = bs.get_binding(&Action::AttackLight).unwrap();
    assert_eq!(light.mouse, Some(MouseButton::Left));
    let heavy = bs.get_binding(&Action::AttackHeavy).unwrap();
    assert_eq!(heavy.mouse, Some(MouseButton::Right));
}

#[test]
fn binding_set_default_ui_nav() {
    let bs = BindingSet::default();
    assert_eq!(
        bs.get_binding(&Action::UiAccept).unwrap().key,
        Some(KeyCode::Enter)
    );
    assert_eq!(
        bs.get_binding(&Action::UiBack).unwrap().key,
        Some(KeyCode::Escape)
    );
    assert_eq!(
        bs.get_binding(&Action::UiUp).unwrap().key,
        Some(KeyCode::ArrowUp)
    );
    assert_eq!(
        bs.get_binding(&Action::UiDown).unwrap().key,
        Some(KeyCode::ArrowDown)
    );
}

#[test]
fn binding_set_default_move_axes_deadzone() {
    let bs = BindingSet::default();
    assert!((bs.move_axes.0.deadzone - 0.15).abs() < 1e-6);
    assert!((bs.move_axes.1.deadzone - 0.15).abs() < 1e-6);
}

#[test]
fn binding_set_default_look_axes_deadzone() {
    let bs = BindingSet::default();
    assert!((bs.look_axes.0.deadzone - 0.12).abs() < 1e-6);
    assert!((bs.look_axes.1.deadzone - 0.12).abs() < 1e-6);
}

#[test]
fn binding_set_default_y_axes_inverted() {
    let bs = BindingSet::default();
    assert!(bs.move_axes.1.is_inverted(), "move Y inverted");
    assert!(bs.look_axes.1.is_inverted(), "look Y inverted");
    assert!(!bs.move_axes.0.is_inverted(), "move X NOT inverted");
    assert!(!bs.look_axes.0.is_inverted(), "look X NOT inverted");
}

#[test]
fn binding_set_set_and_get() {
    let mut bs = BindingSet::new();
    bs.set_binding(Action::Jump, Binding::with_key(KeyCode::Space));
    assert!(bs.has_binding(&Action::Jump));
    assert_eq!(bs.action_count(), 1);
    assert_eq!(
        bs.get_binding(&Action::Jump).unwrap().key,
        Some(KeyCode::Space)
    );
}

#[test]
fn binding_set_remove() {
    let mut bs = BindingSet::new();
    bs.set_binding(Action::Jump, Binding::with_key(KeyCode::Space));
    let removed = bs.remove_binding(&Action::Jump);
    assert!(removed.is_some());
    assert!(!bs.has_binding(&Action::Jump));
    assert_eq!(bs.action_count(), 0);
}

#[test]
fn binding_set_remove_nonexistent() {
    let mut bs = BindingSet::new();
    assert!(bs.remove_binding(&Action::Jump).is_none());
}

#[test]
fn binding_set_bound_actions() {
    let mut bs = BindingSet::new();
    bs.set_binding(Action::Jump, Binding::with_key(KeyCode::Space));
    bs.set_binding(Action::Crouch, Binding::with_key(KeyCode::ControlLeft));
    let bound = bs.bound_actions();
    assert_eq!(bound.len(), 2);
}

#[test]
fn binding_set_non_empty_binding_count() {
    let mut bs = BindingSet::new();
    bs.set_binding(Action::Jump, Binding::with_key(KeyCode::Space));
    bs.set_binding(Action::Crouch, Binding::new()); // empty binding
    assert_eq!(
        bs.non_empty_binding_count(),
        1,
        "empty bindings not counted"
    );
}

#[test]
fn binding_set_serde_roundtrip() {
    let bs = BindingSet::default();
    let json = serde_json::to_string(&bs).unwrap();
    let bs2: BindingSet = serde_json::from_str(&json).unwrap();
    assert_eq!(bs2.action_count(), bs.action_count());
}

// ========================================================================
// INPUT MANAGER
// ========================================================================

#[test]
fn input_manager_default_look_sensitivity() {
    let im = InputManager::new(InputContext::Gameplay, BindingSet::default());
    assert!(
        (im.look_sensitivity - 0.12).abs() < 1e-6,
        "default sensitivity = 0.12"
    );
}

#[test]
fn input_manager_context_switch() {
    let mut im = InputManager::new(InputContext::Gameplay, BindingSet::default());
    assert_eq!(im.context, InputContext::Gameplay);
    im.set_context(InputContext::UI);
    assert_eq!(im.context, InputContext::UI);
}

#[test]
fn input_manager_initial_axes_zero() {
    let im = InputManager::new(InputContext::Gameplay, BindingSet::default());
    assert!(im.move_axis.is_zero());
    assert!(im.look_axis.is_zero());
}

#[test]
fn input_manager_is_down_initially_false() {
    let im = InputManager::new(InputContext::Gameplay, BindingSet::default());
    assert!(!im.is_down(Action::Jump));
    assert!(!im.just_pressed(Action::MoveForward));
}

#[test]
fn input_manager_clear_frame() {
    let mut im = InputManager::new(InputContext::Gameplay, BindingSet::default());
    im.clear_frame();
    // After clear, just_pressed should be empty
    assert!(!im.just_pressed(Action::Jump));
}

// ========================================================================
// SAVE / LOAD
// ========================================================================

#[test]
fn save_load_roundtrip() {
    let bs = BindingSet::default();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bindings.json");
    let path_str = path.to_str().unwrap();
    save_bindings(path_str, &bs).unwrap();
    let loaded = load_bindings(path_str).unwrap();
    assert_eq!(loaded.action_count(), bs.action_count());
}

#[test]
fn load_nonexistent_returns_none() {
    let result = load_bindings("/tmp/nonexistent_astraweave_test_bindings_xyz.json");
    assert!(result.is_none());
}

// ========================================================================
// CATEGORY EXHAUSTIVENESS (catch missing match arms from mutations)
// ========================================================================

#[test]
fn every_action_classified() {
    // Every action should be in exactly one primary category or misc
    for a in Action::all() {
        let cats = [
            a.is_movement(),
            a.is_attack(),
            a.is_ability(),
            a.is_ui_toggle(),
            a.is_ui_nav(),
        ];
        let in_cat = cats.iter().filter(|&&c| c).count();
        let is_misc = matches!(
            a,
            Action::Jump | Action::Crouch | Action::Sprint | Action::Interact
        );
        if !is_misc {
            assert!(in_cat >= 1, "{:?} should be in at least one category", a);
        }
    }
}

#[test]
fn every_gamepad_button_classified() {
    for b in GamepadButton::all() {
        let cats = [
            b.is_face(),
            b.is_shoulder(),
            b.is_dpad(),
            b.is_stick(),
            b.is_system(),
        ];
        let count = cats.iter().filter(|&&c| c).count();
        assert_eq!(count, 1, "{:?} should be in exactly one category", b);
    }
}

#[test]
fn every_axis_kind_classified() {
    for a in AxisKind::all() {
        let is_stick = a.is_stick();
        let is_trigger = a.is_trigger();
        assert!(is_stick ^ is_trigger, "{:?} should be stick XOR trigger", a);
    }
}
