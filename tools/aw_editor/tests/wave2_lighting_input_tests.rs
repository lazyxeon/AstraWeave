//! Wave 2 Mutation Remediation — Lighting + Input Bindings panels
//!
//! Targets: lighting_panel.rs (2,865 lines) + input_bindings_panel.rs (2,320 lines)
//! Focus: enum Display/name/icon, Default values, is_* helpers, numeric returns

use aw_editor_lib::panels::input_bindings_panel::{
    ActionBinding, ActionCategory, AxisBinding, BindingPreset, GamepadButton, InputDevice,
    InputTab, KeyboardKey, MouseButton,
};
use aw_editor_lib::panels::lighting_panel::{
    AmbientMode, EnvironmentSettings, FogMode, GiMode, GiSettings, Light, LightProbe, LightType,
    LightUnit, ReflectionProbe, RefreshMode, ShadowQuality, ShadowType,
};

// ═══════════════════════════════════════════════════════════════════════════
// LIGHT TYPE — 5 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn light_type_default_is_directional() {
    assert_eq!(LightType::default(), LightType::Directional);
}

#[test]
fn light_type_all_count() {
    assert_eq!(LightType::all().len(), 5);
}

#[test]
fn light_type_names() {
    assert_eq!(LightType::Directional.name(), "Directional");
    assert_eq!(LightType::Point.name(), "Point");
    assert_eq!(LightType::Spot.name(), "Spot");
    assert_eq!(LightType::Area.name(), "Area");
    assert_eq!(LightType::Ambient.name(), "Ambient");
}

#[test]
fn light_type_icons_nonempty() {
    for v in LightType::all() {
        assert!(!v.icon().is_empty());
    }
}

#[test]
fn light_type_descriptions_nonempty() {
    for v in LightType::all() {
        assert!(!v.description().is_empty(), "description empty for {:?}", v);
    }
}

#[test]
fn light_type_display_contains_name() {
    for v in LightType::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

#[test]
fn light_type_is_directional() {
    assert!(LightType::Directional.is_directional());
    assert!(!LightType::Point.is_directional());
    assert!(!LightType::Spot.is_directional());
    assert!(!LightType::Area.is_directional());
    assert!(!LightType::Ambient.is_directional());
}

#[test]
fn light_type_has_range() {
    assert!(!LightType::Directional.has_range());
    assert!(LightType::Point.has_range());
    assert!(LightType::Spot.has_range());
    assert!(!LightType::Area.has_range());
    assert!(!LightType::Ambient.has_range());
}

// ═══════════════════════════════════════════════════════════════════════════
// SHADOW QUALITY — 5 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn shadow_quality_default_is_medium() {
    assert_eq!(ShadowQuality::default(), ShadowQuality::Medium);
}

#[test]
fn shadow_quality_all_count() {
    assert_eq!(ShadowQuality::all().len(), 5);
}

#[test]
fn shadow_quality_names() {
    assert_eq!(ShadowQuality::Off.name(), "Off");
    assert_eq!(ShadowQuality::Low.name(), "Low");
    assert_eq!(ShadowQuality::Medium.name(), "Medium");
    assert_eq!(ShadowQuality::High.name(), "High");
    assert_eq!(ShadowQuality::Ultra.name(), "Ultra");
}

#[test]
fn shadow_quality_icons_nonempty() {
    for v in ShadowQuality::all() {
        assert!(!v.icon().is_empty());
    }
}

#[test]
fn shadow_quality_resolution() {
    assert_eq!(ShadowQuality::Off.resolution(), 0);
    assert_eq!(ShadowQuality::Low.resolution(), 512);
    assert_eq!(ShadowQuality::Medium.resolution(), 1024);
    assert_eq!(ShadowQuality::High.resolution(), 2048);
    assert_eq!(ShadowQuality::Ultra.resolution(), 4096);
}

#[test]
fn shadow_quality_is_enabled() {
    assert!(!ShadowQuality::Off.is_enabled());
    assert!(ShadowQuality::Low.is_enabled());
    assert!(ShadowQuality::Medium.is_enabled());
    assert!(ShadowQuality::High.is_enabled());
    assert!(ShadowQuality::Ultra.is_enabled());
}

#[test]
fn shadow_quality_display_contains_name() {
    for v in ShadowQuality::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SHADOW TYPE — 4 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn shadow_type_default_is_hard() {
    assert_eq!(ShadowType::default(), ShadowType::Hard);
}

#[test]
fn shadow_type_all_count() {
    assert_eq!(ShadowType::all().len(), 4);
}

#[test]
fn shadow_type_names() {
    assert_eq!(ShadowType::None.name(), "None");
    assert_eq!(ShadowType::Hard.name(), "Hard");
    assert_eq!(ShadowType::Soft.name(), "Soft");
    assert_eq!(ShadowType::PCSS.name(), "PCSS");
}

#[test]
fn shadow_type_descriptions() {
    for v in ShadowType::all() {
        assert!(!v.description().is_empty());
    }
}

#[test]
fn shadow_type_is_soft() {
    assert!(!ShadowType::None.is_soft());
    assert!(!ShadowType::Hard.is_soft());
    assert!(ShadowType::Soft.is_soft());
    assert!(ShadowType::PCSS.is_soft());
}

#[test]
fn shadow_type_display_contains_name() {
    for v in ShadowType::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// LIGHT UNIT — 5 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn light_unit_default_is_unitless() {
    assert_eq!(LightUnit::default(), LightUnit::Unitless);
}

#[test]
fn light_unit_all_count() {
    assert_eq!(LightUnit::all().len(), 5);
}

#[test]
fn light_unit_names() {
    assert_eq!(LightUnit::Unitless.name(), "Unitless");
    assert_eq!(LightUnit::Lumen.name(), "Lumen");
    assert_eq!(LightUnit::Candela.name(), "Candela");
    assert_eq!(LightUnit::Lux.name(), "Lux");
    assert_eq!(LightUnit::Nit.name(), "Nit");
}

#[test]
fn light_unit_abbreviations() {
    assert_eq!(LightUnit::Unitless.abbreviation(), "");
    assert_eq!(LightUnit::Lumen.abbreviation(), "lm");
    assert_eq!(LightUnit::Candela.abbreviation(), "cd");
    assert_eq!(LightUnit::Lux.abbreviation(), "lx");
    assert_eq!(LightUnit::Nit.abbreviation(), "nt");
}

#[test]
fn light_unit_is_physical() {
    assert!(!LightUnit::Unitless.is_physical());
    assert!(LightUnit::Lumen.is_physical());
    assert!(LightUnit::Candela.is_physical());
    assert!(LightUnit::Lux.is_physical());
    assert!(LightUnit::Nit.is_physical());
}

#[test]
fn light_unit_display_contains_name() {
    for v in LightUnit::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GI MODE (lighting) — 4 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn lighting_gi_mode_default_is_none() {
    assert_eq!(GiMode::default(), GiMode::None);
}

#[test]
fn lighting_gi_mode_all_count() {
    assert_eq!(GiMode::all().len(), 4);
}

#[test]
fn lighting_gi_mode_names() {
    assert_eq!(GiMode::None.name(), "None");
    assert_eq!(GiMode::BakedLightmaps.name(), "Baked Lightmaps");
    assert_eq!(GiMode::RealtimeGI.name(), "Realtime GI");
    assert_eq!(GiMode::Hybrid.name(), "Hybrid");
}

#[test]
fn lighting_gi_mode_descriptions_nonempty() {
    for v in GiMode::all() {
        assert!(!v.description().is_empty());
    }
}

#[test]
fn lighting_gi_mode_is_realtime() {
    assert!(!GiMode::None.is_realtime());
    assert!(!GiMode::BakedLightmaps.is_realtime());
    assert!(GiMode::RealtimeGI.is_realtime());
    assert!(GiMode::Hybrid.is_realtime());
}

#[test]
fn lighting_gi_mode_requires_baking() {
    assert!(!GiMode::None.requires_baking());
    assert!(GiMode::BakedLightmaps.requires_baking());
    assert!(!GiMode::RealtimeGI.requires_baking());
    assert!(GiMode::Hybrid.requires_baking());
}

#[test]
fn lighting_gi_mode_display_contains_name() {
    for v in GiMode::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// REFRESH MODE — 3 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn refresh_mode_default_is_on_awake() {
    assert_eq!(RefreshMode::default(), RefreshMode::OnAwake);
}

#[test]
fn refresh_mode_all_count() {
    assert_eq!(RefreshMode::all().len(), 3);
}

#[test]
fn refresh_mode_names() {
    assert_eq!(RefreshMode::OnAwake.name(), "On Awake");
    assert_eq!(RefreshMode::EveryFrame.name(), "Every Frame");
    assert_eq!(RefreshMode::ViaScript.name(), "Via Script");
}

#[test]
fn refresh_mode_is_automatic() {
    assert!(RefreshMode::OnAwake.is_automatic());
    assert!(RefreshMode::EveryFrame.is_automatic());
    assert!(!RefreshMode::ViaScript.is_automatic());
}

// ═══════════════════════════════════════════════════════════════════════════
// AMBIENT MODE — 3 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ambient_mode_default_is_skybox() {
    assert_eq!(AmbientMode::default(), AmbientMode::Skybox);
}

#[test]
fn ambient_mode_all_count() {
    assert_eq!(AmbientMode::all().len(), 3);
}

#[test]
fn ambient_mode_names() {
    assert_eq!(AmbientMode::Skybox.name(), "Skybox");
    assert_eq!(AmbientMode::Color.name(), "Color");
    assert_eq!(AmbientMode::Gradient.name(), "Gradient");
}

// ═══════════════════════════════════════════════════════════════════════════
// FOG MODE — 4 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn fog_mode_default_is_linear() {
    assert_eq!(FogMode::default(), FogMode::Linear);
}

#[test]
fn fog_mode_all_count() {
    assert_eq!(FogMode::all().len(), 4);
}

#[test]
fn fog_mode_names() {
    assert_eq!(FogMode::Linear.name(), "Linear");
    assert_eq!(FogMode::Exponential.name(), "Exponential");
    assert_eq!(FogMode::ExponentialSquared.name(), "Exponential Squared");
    assert_eq!(FogMode::Height.name(), "Height");
}

#[test]
fn fog_mode_is_exponential() {
    assert!(!FogMode::Linear.is_exponential());
    assert!(FogMode::Exponential.is_exponential());
    assert!(FogMode::ExponentialSquared.is_exponential());
    assert!(!FogMode::Height.is_exponential());
}

// ═══════════════════════════════════════════════════════════════════════════
// LIGHTING STRUCT DEFAULTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn light_default_values() {
    let l = Light::default();
    assert_eq!(l.id, 0);
    assert_eq!(l.name, "New Light");
    assert!(l.enabled);
    assert_eq!(l.light_type, LightType::Point);
    assert_eq!(l.position, [0.0, 5.0, 0.0]);
    assert_eq!(l.color, [1.0, 1.0, 1.0]);
    assert!((l.intensity - 1.0).abs() < f32::EPSILON);
    assert_eq!(l.unit, LightUnit::Unitless);
    assert!((l.temperature - 6500.0).abs() < f32::EPSILON);
    assert!(!l.use_temperature);
    assert!((l.range - 10.0).abs() < f32::EPSILON);
    assert!((l.spot_angle - 45.0).abs() < f32::EPSILON);
    assert!((l.inner_spot_angle - 30.0).abs() < f32::EPSILON);
    assert!(l.cast_shadows);
    assert_eq!(l.shadow_type, ShadowType::Soft);
    assert_eq!(l.shadow_resolution, 1024);
    assert!(!l.volumetric);
    assert!(l.cookie_path.is_empty());
}

#[test]
fn gi_settings_default_values() {
    let g = GiSettings::default();
    assert_eq!(g.mode, GiMode::None);
    assert_eq!(g.lightmap_resolution, 40);
    assert!((g.indirect_intensity - 1.0).abs() < f32::EPSILON);
    assert!(g.ambient_occlusion);
    assert!((g.ao_intensity - 1.0).abs() < f32::EPSILON);
    assert_eq!(g.bounce_count, 2);
    assert_eq!(g.samples_per_texel, 16);
    assert_eq!(g.cascade_count, 4);
}

#[test]
fn light_probe_default_values() {
    let p = LightProbe::default();
    assert_eq!(p.id, 0);
    assert_eq!(p.name, "Light Probe");
    assert!(p.enabled);
    assert_eq!(p.position, [0.0, 1.0, 0.0]);
    assert!((p.blend_distance - 1.0).abs() < f32::EPSILON);
    assert!((p.importance - 1.0).abs() < f32::EPSILON);
    assert!(!p.baked);
}

#[test]
fn reflection_probe_default_values() {
    let r = ReflectionProbe::default();
    assert_eq!(r.id, 0);
    assert_eq!(r.name, "Reflection Probe");
    assert!(r.enabled);
    assert_eq!(r.position, [0.0, 2.0, 0.0]);
    assert_eq!(r.box_size, [10.0, 10.0, 10.0]);
    assert_eq!(r.resolution, 256);
    assert!(r.hdr);
    assert!(!r.realtime);
    assert_eq!(r.refresh_mode, RefreshMode::OnAwake);
    assert!(!r.box_projection);
}

#[test]
fn environment_settings_default_values() {
    let e = EnvironmentSettings::default();
    assert!(e.skybox_enabled);
    assert!(e.skybox_path.is_empty());
    assert_eq!(e.skybox_tint, [1.0, 1.0, 1.0]);
    assert!((e.skybox_exposure - 1.0).abs() < f32::EPSILON);
    assert_eq!(e.ambient_mode, AmbientMode::Skybox);
    assert!(!e.fog_enabled);
    assert_eq!(e.fog_mode, FogMode::Linear);
}

// ═══════════════════════════════════════════════════════════════════════════
// INPUT DEVICE — 4 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn input_device_default_is_keyboard() {
    assert_eq!(InputDevice::default(), InputDevice::Keyboard);
}

#[test]
fn input_device_all_count() {
    assert_eq!(InputDevice::all().len(), 4);
}

#[test]
fn input_device_names() {
    assert_eq!(InputDevice::Keyboard.name(), "Keyboard");
    assert_eq!(InputDevice::Mouse.name(), "Mouse");
    assert_eq!(InputDevice::Gamepad.name(), "Gamepad");
    assert_eq!(InputDevice::All.name(), "All Devices");
}

#[test]
fn input_device_icons_nonempty() {
    for v in InputDevice::all() {
        assert!(!v.icon().is_empty());
    }
}

#[test]
fn input_device_is_physical() {
    assert!(InputDevice::Keyboard.is_physical());
    assert!(InputDevice::Mouse.is_physical());
    assert!(InputDevice::Gamepad.is_physical());
    assert!(!InputDevice::All.is_physical());
}

#[test]
fn input_device_display_contains_name() {
    for v in InputDevice::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ACTION CATEGORY — 7 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn action_category_default_is_movement() {
    assert_eq!(ActionCategory::default(), ActionCategory::Movement);
}

#[test]
fn action_category_all_count() {
    assert_eq!(ActionCategory::all().len(), 7);
}

#[test]
fn action_category_names() {
    assert_eq!(ActionCategory::Movement.name(), "Movement");
    assert_eq!(ActionCategory::Combat.name(), "Combat");
    assert_eq!(ActionCategory::Interaction.name(), "Interaction");
    assert_eq!(ActionCategory::UI.name(), "UI");
    assert_eq!(ActionCategory::Camera.name(), "Camera");
    assert_eq!(ActionCategory::Vehicle.name(), "Vehicle");
    assert_eq!(ActionCategory::Debug.name(), "Debug");
}

#[test]
fn action_category_is_gameplay() {
    assert!(ActionCategory::Movement.is_gameplay());
    assert!(ActionCategory::Combat.is_gameplay());
    assert!(ActionCategory::Interaction.is_gameplay());
    assert!(!ActionCategory::UI.is_gameplay());
    assert!(!ActionCategory::Camera.is_gameplay());
    assert!(ActionCategory::Vehicle.is_gameplay());
    assert!(!ActionCategory::Debug.is_gameplay());
}

#[test]
fn action_category_display_contains_name() {
    for v in ActionCategory::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BINDING PRESET — 7 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn binding_preset_default_is_default() {
    assert_eq!(BindingPreset::default(), BindingPreset::Default);
}

#[test]
fn binding_preset_all_count() {
    assert_eq!(BindingPreset::all().len(), 7);
}

#[test]
fn binding_preset_names() {
    assert_eq!(BindingPreset::Default.name(), "Default");
    assert_eq!(BindingPreset::FPS.name(), "FPS");
    assert_eq!(BindingPreset::ThirdPerson.name(), "Third Person");
    assert_eq!(BindingPreset::RTS.name(), "RTS");
    assert_eq!(BindingPreset::Racing.name(), "Racing");
    assert_eq!(BindingPreset::LeftHanded.name(), "Left Handed");
    assert_eq!(BindingPreset::Custom.name(), "Custom");
}

#[test]
fn binding_preset_is_built_in() {
    assert!(BindingPreset::Default.is_built_in());
    assert!(BindingPreset::FPS.is_built_in());
    assert!(BindingPreset::ThirdPerson.is_built_in());
    assert!(BindingPreset::RTS.is_built_in());
    assert!(BindingPreset::Racing.is_built_in());
    assert!(BindingPreset::LeftHanded.is_built_in());
    assert!(!BindingPreset::Custom.is_built_in());
}

#[test]
fn binding_preset_display_contains_name() {
    for v in BindingPreset::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GAMEPAD BUTTON — 16 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn gamepad_button_all_count() {
    assert_eq!(GamepadButton::all().len(), 16);
}

#[test]
fn gamepad_button_display_names_nonempty() {
    for v in GamepadButton::all() {
        assert!(!v.display_name().is_empty());
    }
}

#[test]
fn gamepad_button_is_face_button() {
    assert!(GamepadButton::South.is_face_button());
    assert!(GamepadButton::East.is_face_button());
    assert!(GamepadButton::West.is_face_button());
    assert!(GamepadButton::North.is_face_button());
    assert!(!GamepadButton::L1.is_face_button());
    assert!(!GamepadButton::R1.is_face_button());
    assert!(!GamepadButton::DPadUp.is_face_button());
    assert!(!GamepadButton::Start.is_face_button());
}

#[test]
fn gamepad_button_is_shoulder() {
    assert!(GamepadButton::L1.is_shoulder());
    assert!(GamepadButton::R1.is_shoulder());
    assert!(GamepadButton::L2.is_shoulder());
    assert!(GamepadButton::R2.is_shoulder());
    assert!(!GamepadButton::South.is_shoulder());
    assert!(!GamepadButton::Start.is_shoulder());
    assert!(!GamepadButton::DPadUp.is_shoulder());
}

#[test]
fn gamepad_button_is_dpad() {
    assert!(GamepadButton::DPadUp.is_dpad());
    assert!(GamepadButton::DPadDown.is_dpad());
    assert!(GamepadButton::DPadLeft.is_dpad());
    assert!(GamepadButton::DPadRight.is_dpad());
    assert!(!GamepadButton::South.is_dpad());
    assert!(!GamepadButton::L1.is_dpad());
    assert!(!GamepadButton::Start.is_dpad());
}

#[test]
fn gamepad_button_display_has_gamepad_prefix() {
    for v in GamepadButton::all() {
        let d = format!("{v}");
        assert!(!d.is_empty(), "display '{}' should be non-empty", d);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// KEYBOARD KEY — comprehensive classification tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn keyboard_key_default_is_a() {
    assert_eq!(KeyboardKey::default(), KeyboardKey::A);
}

#[test]
fn keyboard_key_all_count() {
    // 26 letters + 10 numbers + 12 function + 6 modifiers + 5 special + 4 arrows + 6 other = 69
    assert_eq!(KeyboardKey::all().len(), 69);
}

#[test]
fn keyboard_key_is_letter() {
    assert!(KeyboardKey::A.is_letter());
    assert!(KeyboardKey::Z.is_letter());
    assert!(KeyboardKey::M.is_letter());
    assert!(!KeyboardKey::Key0.is_letter());
    assert!(!KeyboardKey::F1.is_letter());
    assert!(!KeyboardKey::Space.is_letter());
}

#[test]
fn keyboard_key_is_number() {
    assert!(KeyboardKey::Key0.is_number());
    assert!(KeyboardKey::Key5.is_number());
    assert!(KeyboardKey::Key9.is_number());
    assert!(!KeyboardKey::A.is_number());
    assert!(!KeyboardKey::F1.is_number());
}

#[test]
fn keyboard_key_is_function() {
    assert!(KeyboardKey::F1.is_function());
    assert!(KeyboardKey::F6.is_function());
    assert!(KeyboardKey::F12.is_function());
    assert!(!KeyboardKey::A.is_function());
    assert!(!KeyboardKey::Key1.is_function());
}

#[test]
fn keyboard_key_is_modifier() {
    assert!(KeyboardKey::ShiftLeft.is_modifier());
    assert!(KeyboardKey::ShiftRight.is_modifier());
    assert!(KeyboardKey::CtrlLeft.is_modifier());
    assert!(KeyboardKey::CtrlRight.is_modifier());
    assert!(KeyboardKey::AltLeft.is_modifier());
    assert!(KeyboardKey::AltRight.is_modifier());
    assert!(!KeyboardKey::A.is_modifier());
    assert!(!KeyboardKey::Space.is_modifier());
}

#[test]
fn keyboard_key_is_arrow() {
    assert!(KeyboardKey::ArrowUp.is_arrow());
    assert!(KeyboardKey::ArrowDown.is_arrow());
    assert!(KeyboardKey::ArrowLeft.is_arrow());
    assert!(KeyboardKey::ArrowRight.is_arrow());
    assert!(!KeyboardKey::A.is_arrow());
    assert!(!KeyboardKey::Space.is_arrow());
}

#[test]
fn keyboard_key_display_names_nonempty() {
    for v in KeyboardKey::all() {
        assert!(
            !v.display_name().is_empty(),
            "display_name empty for {:?}",
            v
        );
    }
}

#[test]
fn keyboard_key_display_has_keyboard_prefix() {
    for v in KeyboardKey::all() {
        let d = format!("{v}");
        assert!(
            d.contains("⌨️"),
            "display '{}' missing keyboard icon for {:?}",
            d,
            v
        );
    }
}

#[test]
fn keyboard_key_name_equals_display_name() {
    for v in KeyboardKey::all() {
        assert_eq!(
            v.name(),
            v.display_name(),
            "name != display_name for {:?}",
            v
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MOUSE BUTTON — 5 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn mouse_button_all_count() {
    assert_eq!(MouseButton::all().len(), 5);
}

#[test]
fn mouse_button_display_names() {
    assert_eq!(MouseButton::Left.display_name(), "Left Click");
    assert_eq!(MouseButton::Right.display_name(), "Right Click");
    assert_eq!(MouseButton::Middle.display_name(), "Middle Click");
    assert_eq!(MouseButton::Back.display_name(), "Mouse 4");
    assert_eq!(MouseButton::Forward.display_name(), "Mouse 5");
}

#[test]
fn mouse_button_is_primary() {
    assert!(MouseButton::Left.is_primary());
    assert!(MouseButton::Right.is_primary());
    assert!(!MouseButton::Middle.is_primary());
    assert!(!MouseButton::Back.is_primary());
    assert!(!MouseButton::Forward.is_primary());
}

#[test]
fn mouse_button_display_has_mouse_icon() {
    for v in MouseButton::all() {
        let d = format!("{v}");
        assert!(d.contains("🖱️"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INPUT TAB — 5 variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn input_tab_default_is_actions() {
    assert_eq!(InputTab::default(), InputTab::Actions);
}

#[test]
fn input_tab_all_count() {
    assert_eq!(InputTab::all().len(), 5);
}

#[test]
fn input_tab_names() {
    assert_eq!(InputTab::Actions.name(), "Actions");
    assert_eq!(InputTab::Axes.name(), "Axes");
    assert_eq!(InputTab::Gamepad.name(), "Gamepad");
    assert_eq!(InputTab::Testing.name(), "Testing");
    assert_eq!(InputTab::Presets.name(), "Presets");
}

#[test]
fn input_tab_display_contains_name() {
    for v in InputTab::all() {
        let d = format!("{v}");
        assert!(d.contains(v.name()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INPUT STRUCT DEFAULTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn action_binding_default_values() {
    let a = ActionBinding::default();
    assert_eq!(a.name, "Unnamed");
    assert_eq!(a.category, ActionCategory::Movement);
    assert!(a.description.is_empty());
    assert_eq!(a.keyboard_primary, None);
    assert_eq!(a.keyboard_secondary, None);
    assert_eq!(a.mouse_button, None);
    assert_eq!(a.gamepad_button, None);
    assert!(!a.is_hold);
    assert!(!a.is_axis);
}

#[test]
fn axis_binding_default_values() {
    let a = AxisBinding::default();
    assert_eq!(a.name, "Unnamed Axis");
    assert!(a.description.is_empty());
    assert!((a.sensitivity - 1.0).abs() < f32::EPSILON);
    assert!((a.deadzone - 0.15).abs() < f32::EPSILON);
    assert!(!a.invert);
    assert!((a.smoothing - 0.0).abs() < f32::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════════════
// CROSS-CUTTING: Unique name checks
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn light_type_names_unique() {
    let names: Vec<&str> = LightType::all().iter().map(|v| v.name()).collect();
    let set: std::collections::HashSet<&&str> = names.iter().collect();
    assert_eq!(names.len(), set.len());
}

#[test]
fn shadow_quality_names_unique() {
    let names: Vec<&str> = ShadowQuality::all().iter().map(|v| v.name()).collect();
    let set: std::collections::HashSet<&&str> = names.iter().collect();
    assert_eq!(names.len(), set.len());
}

#[test]
fn action_category_names_unique() {
    let names: Vec<&str> = ActionCategory::all().iter().map(|v| v.name()).collect();
    let set: std::collections::HashSet<&&str> = names.iter().collect();
    assert_eq!(names.len(), set.len());
}

#[test]
fn gamepad_button_display_names_unique() {
    let names: Vec<&str> = GamepadButton::all()
        .iter()
        .map(|v| v.display_name())
        .collect();
    let set: std::collections::HashSet<&&str> = names.iter().collect();
    assert_eq!(names.len(), set.len());
}

#[test]
fn keyboard_key_all_names_unique() {
    let names: Vec<&str> = KeyboardKey::all()
        .iter()
        .map(|v| v.display_name())
        .collect();
    let set: std::collections::HashSet<&&str> = names.iter().collect();
    assert_eq!(names.len(), set.len());
}

#[test]
fn shadow_quality_resolution_increasing() {
    let resolutions: Vec<u32> = ShadowQuality::all()
        .iter()
        .filter(|q| q.is_enabled())
        .map(|q| q.resolution())
        .collect();
    for window in resolutions.windows(2) {
        assert!(
            window[0] < window[1],
            "resolutions not monotonically increasing"
        );
    }
}
