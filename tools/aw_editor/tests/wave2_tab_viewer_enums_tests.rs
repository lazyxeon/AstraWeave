//! Wave 2 Mutation Remediation: tab_viewer.rs — Enums
//!
//! Targets: PanelEvent (35 variants × Display/category/is_*/entity_id),
//!          EditorTheme (4 variants × name/is_dark/is_light/Display),
//!          BehaviorNodeType (6 variants × name/icon/is_composite/is_leaf/can_have_children/Display)
//!
//! These tests kill mutants on every branch and return-value route in the
//! three richest enums living inside tab_viewer.rs.

use aw_editor_lib::panel_type::PanelType;
use aw_editor_lib::tab_viewer::{
    AnimationState, BehaviorNodeType, EditorTheme, MaterialInfo, PanelEvent,
};

// ============================================================================
// PanelEvent — Display (each variant must produce the exact expected string)
// ============================================================================

#[test]
fn panel_event_display_panel_closed() {
    let e = PanelEvent::PanelClosed(PanelType::Inspector);
    let s = e.to_string();
    assert!(s.contains("Panel Closed"), "got: {s}");
    assert!(s.contains(&PanelType::Inspector.to_string()));
}

#[test]
fn panel_event_display_panel_focused() {
    let e = PanelEvent::PanelFocused(PanelType::Console);
    let s = e.to_string();
    assert!(s.contains("Panel Focused"));
}

#[test]
fn panel_event_display_add_panel() {
    let e = PanelEvent::AddPanel(PanelType::Hierarchy);
    let s = e.to_string();
    assert!(s.contains("Add Panel"));
}

#[test]
fn panel_event_display_entity_selected() {
    let e = PanelEvent::EntitySelected(42);
    assert_eq!(e.to_string(), "Entity Selected: 42");
}

#[test]
fn panel_event_display_entity_deselected() {
    let e = PanelEvent::EntityDeselected;
    assert_eq!(e.to_string(), "Entity Deselected");
}

#[test]
fn panel_event_display_transform_position_changed() {
    let e = PanelEvent::TransformPositionChanged {
        entity_id: 7,
        x: 1.0,
        y: 2.0,
        z: 0.0,
    };
    let s = e.to_string();
    assert!(s.contains("Transform Position Changed: 7"));
}

#[test]
fn panel_event_display_transform_rotation_changed() {
    let e = PanelEvent::TransformRotationChanged {
        entity_id: 8,
        rotation: 90.0,
    };
    let s = e.to_string();
    assert!(s.contains("Transform Rotation Changed: 8"));
}

#[test]
fn panel_event_display_transform_scale_changed() {
    let e = PanelEvent::TransformScaleChanged {
        entity_id: 9,
        scale_x: 2.0,
        scale_y: 3.0,
        scale_z: 1.0,
    };
    let s = e.to_string();
    assert!(s.contains("Transform Scale Changed: 9"));
}

#[test]
fn panel_event_display_create_entity() {
    assert_eq!(PanelEvent::CreateEntity.to_string(), "Create Entity");
}

#[test]
fn panel_event_display_delete_entity() {
    assert_eq!(
        PanelEvent::DeleteEntity(55).to_string(),
        "Delete Entity: 55"
    );
}

#[test]
fn panel_event_display_duplicate_entity() {
    assert_eq!(
        PanelEvent::DuplicateEntity(101).to_string(),
        "Duplicate Entity: 101"
    );
}

#[test]
fn panel_event_display_material_changed() {
    let e = PanelEvent::MaterialChanged {
        name: "steel".into(),
        property: "roughness".into(),
        value: 0.5,
    };
    assert_eq!(e.to_string(), "Material Changed: steel.roughness");
}

#[test]
fn panel_event_display_animation_play_state_playing() {
    let e = PanelEvent::AnimationPlayStateChanged { is_playing: true };
    assert_eq!(e.to_string(), "Animation Playing");
}

#[test]
fn panel_event_display_animation_play_state_paused() {
    let e = PanelEvent::AnimationPlayStateChanged { is_playing: false };
    assert_eq!(e.to_string(), "Animation Paused");
}

#[test]
fn panel_event_display_animation_frame_changed() {
    let e = PanelEvent::AnimationFrameChanged { frame: 45 };
    assert_eq!(e.to_string(), "Animation Frame: 45");
}

#[test]
fn panel_event_display_animation_keyframe_added() {
    let e = PanelEvent::AnimationKeyframeAdded {
        track_index: 2,
        frame: 10,
        value: 3.14,
    };
    assert_eq!(e.to_string(), "Keyframe Added: Track 2 @ 10");
}

#[test]
fn panel_event_display_theme_changed() {
    let e = PanelEvent::ThemeChanged(EditorTheme::Nord);
    assert!(e.to_string().contains("Theme Changed"));
    assert!(e.to_string().contains("Nord"));
}

#[test]
fn panel_event_display_build_requested() {
    let e = PanelEvent::BuildRequested {
        target: "windows".into(),
        profile: "release".into(),
    };
    let s = e.to_string();
    assert!(s.contains("Build Requested"));
    assert!(s.contains("windows"));
    assert!(s.contains("release"));
}

#[test]
fn panel_event_display_console_cleared() {
    assert_eq!(PanelEvent::ConsoleCleared.to_string(), "Console Cleared");
}

#[test]
fn panel_event_display_asset_selected() {
    let e = PanelEvent::AssetSelected("textures/wood.png".into());
    assert_eq!(e.to_string(), "Asset Selected: textures/wood.png");
}

#[test]
fn panel_event_display_behavior_node_selected() {
    let e = PanelEvent::BehaviorNodeSelected(3);
    assert_eq!(e.to_string(), "Behavior Node Selected: 3");
}

#[test]
fn panel_event_display_graph_node_selected() {
    let e = PanelEvent::GraphNodeSelected(7);
    assert_eq!(e.to_string(), "Graph Node Selected: 7");
}

#[test]
fn panel_event_display_hierarchy_search_changed() {
    let e = PanelEvent::HierarchySearchChanged("player".into());
    assert_eq!(e.to_string(), "Hierarchy Search: player");
}

#[test]
fn panel_event_display_console_search_changed() {
    let e = PanelEvent::ConsoleSearchChanged("error".into());
    assert_eq!(e.to_string(), "Console Search: error");
}

#[test]
fn panel_event_display_refresh_scene_stats() {
    assert_eq!(
        PanelEvent::RefreshSceneStats.to_string(),
        "Refresh Scene Stats"
    );
}

#[test]
fn panel_event_display_add_component() {
    let e = PanelEvent::AddComponent {
        entity_id: 10,
        component_type: "Rigidbody".into(),
    };
    assert_eq!(e.to_string(), "Add Component: Rigidbody");
}

#[test]
fn panel_event_display_remove_component() {
    let e = PanelEvent::RemoveComponent {
        entity_id: 10,
        component_type: "Collider".into(),
    };
    assert_eq!(e.to_string(), "Remove Component: Collider");
}

#[test]
fn panel_event_display_viewport_view_mode_changed() {
    let e = PanelEvent::ViewportViewModeChanged(2);
    assert_eq!(e.to_string(), "Viewport View Mode: 2");
}

#[test]
fn panel_event_display_viewport_gizmo_mode_changed() {
    let e = PanelEvent::ViewportGizmoModeChanged(1);
    assert_eq!(e.to_string(), "Viewport Gizmo Mode: 1");
}

#[test]
fn panel_event_display_viewport_gizmo_space_changed() {
    let e = PanelEvent::ViewportGizmoSpaceChanged(0);
    assert_eq!(e.to_string(), "Viewport Gizmo Space: 0");
}

#[test]
fn panel_event_display_viewport_overlay_toggled() {
    let e = PanelEvent::ViewportOverlayToggled {
        overlay: "grid".into(),
        enabled: true,
    };
    let s = e.to_string();
    assert!(s.contains("grid"));
    assert!(s.contains("true"));
}

#[test]
fn panel_event_display_viewport_camera_changed() {
    let e = PanelEvent::ViewportCameraChanged {
        fov: 60.0,
        near: 0.1,
        far: 1000.0,
        speed: 10.0,
    };
    assert_eq!(e.to_string(), "Viewport Camera Changed");
}

#[test]
fn panel_event_display_viewport_focus_on_selection() {
    assert_eq!(
        PanelEvent::ViewportFocusOnSelection.to_string(),
        "Viewport Focus On Selection"
    );
}

#[test]
fn panel_event_display_viewport_reset_camera() {
    assert_eq!(
        PanelEvent::ViewportResetCamera.to_string(),
        "Viewport Reset Camera"
    );
}

#[test]
fn panel_event_display_viewport_camera_preset() {
    let e = PanelEvent::ViewportCameraPreset("front".into());
    assert_eq!(e.to_string(), "Viewport Camera Preset: front");
}

#[test]
fn panel_event_display_reset_layout() {
    assert_eq!(PanelEvent::ResetLayout.to_string(), "Reset Layout");
}

// ============================================================================
// PanelEvent — category()  (every variant → exact category string)
// ============================================================================

#[test]
fn panel_event_category_panel_variants() {
    assert_eq!(
        PanelEvent::PanelClosed(PanelType::Viewport).category(),
        "Panel"
    );
    assert_eq!(
        PanelEvent::PanelFocused(PanelType::Viewport).category(),
        "Panel"
    );
    assert_eq!(
        PanelEvent::AddPanel(PanelType::Viewport).category(),
        "Panel"
    );
    assert_eq!(PanelEvent::ResetLayout.category(), "Panel");
}

#[test]
fn panel_event_category_entity_variants() {
    assert_eq!(PanelEvent::EntitySelected(1).category(), "Entity");
    assert_eq!(PanelEvent::EntityDeselected.category(), "Entity");
    assert_eq!(PanelEvent::CreateEntity.category(), "Entity");
    assert_eq!(PanelEvent::DeleteEntity(1).category(), "Entity");
    assert_eq!(PanelEvent::DuplicateEntity(1).category(), "Entity");
}

#[test]
fn panel_event_category_transform_variants() {
    assert_eq!(
        PanelEvent::TransformPositionChanged {
            entity_id: 1,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
        .category(),
        "Transform"
    );
    assert_eq!(
        PanelEvent::TransformRotationChanged {
            entity_id: 1,
            rotation: 0.0
        }
        .category(),
        "Transform"
    );
    assert_eq!(
        PanelEvent::TransformScaleChanged {
            entity_id: 1,
            scale_x: 1.0,
            scale_y: 1.0,
            scale_z: 1.0,
        }
        .category(),
        "Transform"
    );
}

#[test]
fn panel_event_category_material() {
    assert_eq!(
        PanelEvent::MaterialChanged {
            name: "a".into(),
            property: "b".into(),
            value: 0.0
        }
        .category(),
        "Material"
    );
}

#[test]
fn panel_event_category_animation_variants() {
    assert_eq!(
        PanelEvent::AnimationPlayStateChanged { is_playing: true }.category(),
        "Animation"
    );
    assert_eq!(
        PanelEvent::AnimationFrameChanged { frame: 0 }.category(),
        "Animation"
    );
    assert_eq!(
        PanelEvent::AnimationKeyframeAdded {
            track_index: 0,
            frame: 0,
            value: 0.0
        }
        .category(),
        "Animation"
    );
}

#[test]
fn panel_event_category_theme() {
    assert_eq!(
        PanelEvent::ThemeChanged(EditorTheme::Dark).category(),
        "Theme"
    );
}

#[test]
fn panel_event_category_build() {
    assert_eq!(
        PanelEvent::BuildRequested {
            target: "w".into(),
            profile: "r".into()
        }
        .category(),
        "Build"
    );
}

#[test]
fn panel_event_category_console_variants() {
    assert_eq!(PanelEvent::ConsoleCleared.category(), "Console");
    assert_eq!(
        PanelEvent::ConsoleSearchChanged("x".into()).category(),
        "Console"
    );
}

#[test]
fn panel_event_category_asset() {
    assert_eq!(PanelEvent::AssetSelected("x".into()).category(), "Asset");
}

#[test]
fn panel_event_category_graph_variants() {
    assert_eq!(PanelEvent::BehaviorNodeSelected(0).category(), "Graph");
    assert_eq!(PanelEvent::GraphNodeSelected(0).category(), "Graph");
}

#[test]
fn panel_event_category_hierarchy() {
    assert_eq!(
        PanelEvent::HierarchySearchChanged("x".into()).category(),
        "Hierarchy"
    );
}

#[test]
fn panel_event_category_scene() {
    assert_eq!(PanelEvent::RefreshSceneStats.category(), "Scene");
}

#[test]
fn panel_event_category_component_variants() {
    assert_eq!(
        PanelEvent::AddComponent {
            entity_id: 1,
            component_type: "c".into()
        }
        .category(),
        "Component"
    );
    assert_eq!(
        PanelEvent::RemoveComponent {
            entity_id: 1,
            component_type: "c".into()
        }
        .category(),
        "Component"
    );
}

#[test]
fn panel_event_category_viewport_variants() {
    assert_eq!(
        PanelEvent::ViewportViewModeChanged(0).category(),
        "Viewport"
    );
    assert_eq!(
        PanelEvent::ViewportGizmoModeChanged(0).category(),
        "Viewport"
    );
    assert_eq!(
        PanelEvent::ViewportGizmoSpaceChanged(0).category(),
        "Viewport"
    );
    assert_eq!(
        PanelEvent::ViewportOverlayToggled {
            overlay: "g".into(),
            enabled: true
        }
        .category(),
        "Viewport"
    );
    assert_eq!(
        PanelEvent::ViewportCameraChanged {
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
            speed: 10.0
        }
        .category(),
        "Viewport"
    );
    assert_eq!(PanelEvent::ViewportFocusOnSelection.category(), "Viewport");
    assert_eq!(PanelEvent::ViewportResetCamera.category(), "Viewport");
    assert_eq!(
        PanelEvent::ViewportCameraPreset("f".into()).category(),
        "Viewport"
    );
}

// ============================================================================
// PanelEvent — is_panel_event()
// ============================================================================

#[test]
fn panel_event_is_panel_event_positive() {
    assert!(PanelEvent::PanelClosed(PanelType::Console).is_panel_event());
    assert!(PanelEvent::PanelFocused(PanelType::Console).is_panel_event());
    assert!(PanelEvent::AddPanel(PanelType::Console).is_panel_event());
    assert!(PanelEvent::ResetLayout.is_panel_event());
}

#[test]
fn panel_event_is_panel_event_negative() {
    assert!(!PanelEvent::EntitySelected(1).is_panel_event());
    assert!(!PanelEvent::CreateEntity.is_panel_event());
    assert!(!PanelEvent::ConsoleCleared.is_panel_event());
    assert!(!PanelEvent::ViewportResetCamera.is_panel_event());
    assert!(!PanelEvent::ThemeChanged(EditorTheme::Dark).is_panel_event());
}

// ============================================================================
// PanelEvent — is_entity_event()
// ============================================================================

#[test]
fn panel_event_is_entity_event_positive() {
    assert!(PanelEvent::EntitySelected(1).is_entity_event());
    assert!(PanelEvent::EntityDeselected.is_entity_event());
    assert!(PanelEvent::CreateEntity.is_entity_event());
    assert!(PanelEvent::DeleteEntity(1).is_entity_event());
    assert!(PanelEvent::DuplicateEntity(1).is_entity_event());
}

#[test]
fn panel_event_is_entity_event_negative() {
    assert!(!PanelEvent::PanelClosed(PanelType::Viewport).is_entity_event());
    assert!(!PanelEvent::TransformPositionChanged {
        entity_id: 1,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }
    .is_entity_event());
    assert!(!PanelEvent::ConsoleCleared.is_entity_event());
}

// ============================================================================
// PanelEvent — is_transform_event()
// ============================================================================

#[test]
fn panel_event_is_transform_event_positive() {
    assert!(PanelEvent::TransformPositionChanged {
        entity_id: 1,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }
    .is_transform_event());
    assert!(PanelEvent::TransformRotationChanged {
        entity_id: 1,
        rotation: 0.0
    }
    .is_transform_event());
    assert!(PanelEvent::TransformScaleChanged {
        entity_id: 1,
        scale_x: 1.0,
        scale_y: 1.0,
        scale_z: 1.0,
    }
    .is_transform_event());
}

#[test]
fn panel_event_is_transform_event_negative() {
    assert!(!PanelEvent::EntitySelected(1).is_transform_event());
    assert!(!PanelEvent::PanelClosed(PanelType::Viewport).is_transform_event());
    assert!(!PanelEvent::ViewportGizmoModeChanged(0).is_transform_event());
}

// ============================================================================
// PanelEvent — is_viewport_event()
// ============================================================================

#[test]
fn panel_event_is_viewport_event_positive() {
    assert!(PanelEvent::ViewportViewModeChanged(0).is_viewport_event());
    assert!(PanelEvent::ViewportGizmoModeChanged(0).is_viewport_event());
    assert!(PanelEvent::ViewportGizmoSpaceChanged(0).is_viewport_event());
    assert!(PanelEvent::ViewportOverlayToggled {
        overlay: "g".into(),
        enabled: true
    }
    .is_viewport_event());
    assert!(PanelEvent::ViewportCameraChanged {
        fov: 60.0,
        near: 0.1,
        far: 1000.0,
        speed: 10.0
    }
    .is_viewport_event());
    assert!(PanelEvent::ViewportFocusOnSelection.is_viewport_event());
    assert!(PanelEvent::ViewportResetCamera.is_viewport_event());
    assert!(PanelEvent::ViewportCameraPreset("front".into()).is_viewport_event());
}

#[test]
fn panel_event_is_viewport_event_negative() {
    assert!(!PanelEvent::EntitySelected(1).is_viewport_event());
    assert!(!PanelEvent::TransformPositionChanged {
        entity_id: 1,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }
    .is_viewport_event());
    assert!(!PanelEvent::ResetLayout.is_viewport_event());
    assert!(!PanelEvent::ConsoleCleared.is_viewport_event());
}

// ============================================================================
// PanelEvent — entity_id()  (Some for 8 variants, None for all others)
// ============================================================================

#[test]
fn panel_event_entity_id_some() {
    assert_eq!(PanelEvent::EntitySelected(42).entity_id(), Some(42));
    assert_eq!(PanelEvent::DeleteEntity(55).entity_id(), Some(55));
    assert_eq!(PanelEvent::DuplicateEntity(66).entity_id(), Some(66));
    assert_eq!(
        PanelEvent::TransformPositionChanged {
            entity_id: 77,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
        .entity_id(),
        Some(77)
    );
    assert_eq!(
        PanelEvent::TransformRotationChanged {
            entity_id: 88,
            rotation: 0.0
        }
        .entity_id(),
        Some(88)
    );
    assert_eq!(
        PanelEvent::TransformScaleChanged {
            entity_id: 99,
            scale_x: 1.0,
            scale_y: 1.0,
            scale_z: 1.0,
        }
        .entity_id(),
        Some(99)
    );
    assert_eq!(
        PanelEvent::AddComponent {
            entity_id: 11,
            component_type: "c".into()
        }
        .entity_id(),
        Some(11)
    );
    assert_eq!(
        PanelEvent::RemoveComponent {
            entity_id: 22,
            component_type: "c".into()
        }
        .entity_id(),
        Some(22)
    );
}

#[test]
fn panel_event_entity_id_none() {
    assert_eq!(PanelEvent::EntityDeselected.entity_id(), None);
    assert_eq!(PanelEvent::CreateEntity.entity_id(), None);
    assert_eq!(
        PanelEvent::PanelClosed(PanelType::Viewport).entity_id(),
        None
    );
    assert_eq!(PanelEvent::ConsoleCleared.entity_id(), None);
    assert_eq!(
        PanelEvent::ThemeChanged(EditorTheme::Dark).entity_id(),
        None
    );
    assert_eq!(PanelEvent::ResetLayout.entity_id(), None);
    assert_eq!(PanelEvent::ViewportResetCamera.entity_id(), None);
    assert_eq!(PanelEvent::RefreshSceneStats.entity_id(), None);
    assert_eq!(PanelEvent::AssetSelected("x".into()).entity_id(), None);
    assert_eq!(PanelEvent::BehaviorNodeSelected(1).entity_id(), None);
    assert_eq!(PanelEvent::GraphNodeSelected(1).entity_id(), None);
    assert_eq!(
        PanelEvent::AnimationPlayStateChanged { is_playing: true }.entity_id(),
        None
    );
    assert_eq!(
        PanelEvent::AnimationFrameChanged { frame: 0 }.entity_id(),
        None
    );
    assert_eq!(
        PanelEvent::AnimationKeyframeAdded {
            track_index: 0,
            frame: 0,
            value: 0.0
        }
        .entity_id(),
        None
    );
    assert_eq!(
        PanelEvent::MaterialChanged {
            name: "a".into(),
            property: "b".into(),
            value: 0.0
        }
        .entity_id(),
        None
    );
    assert_eq!(
        PanelEvent::BuildRequested {
            target: "w".into(),
            profile: "r".into()
        }
        .entity_id(),
        None
    );
    assert_eq!(
        PanelEvent::HierarchySearchChanged("x".into()).entity_id(),
        None
    );
    assert_eq!(
        PanelEvent::ConsoleSearchChanged("x".into()).entity_id(),
        None
    );
    assert_eq!(PanelEvent::ViewportViewModeChanged(0).entity_id(), None);
    assert_eq!(PanelEvent::ViewportGizmoModeChanged(0).entity_id(), None);
    assert_eq!(PanelEvent::ViewportGizmoSpaceChanged(0).entity_id(), None);
    assert_eq!(
        PanelEvent::ViewportOverlayToggled {
            overlay: "g".into(),
            enabled: true
        }
        .entity_id(),
        None
    );
    assert_eq!(
        PanelEvent::ViewportCameraChanged {
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
            speed: 10.0
        }
        .entity_id(),
        None
    );
    assert_eq!(PanelEvent::ViewportFocusOnSelection.entity_id(), None);
    assert_eq!(
        PanelEvent::ViewportCameraPreset("f".into()).entity_id(),
        None
    );
}

// ============================================================================
// EditorTheme — Display
// ============================================================================

#[test]
fn editor_theme_display() {
    assert_eq!(EditorTheme::Dark.to_string(), "Dark");
    assert_eq!(EditorTheme::Light.to_string(), "Light");
    assert_eq!(EditorTheme::Nord.to_string(), "Nord");
    assert_eq!(EditorTheme::Solarized.to_string(), "Solarized");
}

// ============================================================================
// EditorTheme — name()
// ============================================================================

#[test]
fn editor_theme_name() {
    assert_eq!(EditorTheme::Dark.name(), "Dark");
    assert_eq!(EditorTheme::Light.name(), "Light");
    assert_eq!(EditorTheme::Nord.name(), "Nord");
    assert_eq!(EditorTheme::Solarized.name(), "Solarized");
}

// ============================================================================
// EditorTheme — is_dark()
// ============================================================================

#[test]
fn editor_theme_is_dark() {
    assert!(EditorTheme::Dark.is_dark());
    assert!(EditorTheme::Nord.is_dark());
    assert!(!EditorTheme::Light.is_dark());
    assert!(!EditorTheme::Solarized.is_dark());
}

// ============================================================================
// EditorTheme — is_light()
// ============================================================================

#[test]
fn editor_theme_is_light() {
    assert!(EditorTheme::Light.is_light());
    assert!(EditorTheme::Solarized.is_light());
    assert!(!EditorTheme::Dark.is_light());
    assert!(!EditorTheme::Nord.is_light());
}

// ============================================================================
// EditorTheme — all()
// ============================================================================

#[test]
fn editor_theme_all_count() {
    assert_eq!(EditorTheme::all().len(), 4);
}

#[test]
fn editor_theme_all_contains_each() {
    let all = EditorTheme::all();
    assert!(all.contains(&EditorTheme::Dark));
    assert!(all.contains(&EditorTheme::Light));
    assert!(all.contains(&EditorTheme::Nord));
    assert!(all.contains(&EditorTheme::Solarized));
}

// ============================================================================
// EditorTheme — Default
// ============================================================================

#[test]
fn editor_theme_default_is_dark() {
    assert_eq!(EditorTheme::default(), EditorTheme::Dark);
}

// ============================================================================
// BehaviorNodeType — Display
// ============================================================================

#[test]
fn behavior_node_type_display() {
    assert_eq!(BehaviorNodeType::Root.to_string(), "Root");
    assert_eq!(BehaviorNodeType::Sequence.to_string(), "Sequence");
    assert_eq!(BehaviorNodeType::Selector.to_string(), "Selector");
    assert_eq!(BehaviorNodeType::Condition.to_string(), "Condition");
    assert_eq!(BehaviorNodeType::Action.to_string(), "Action");
    assert_eq!(BehaviorNodeType::Decorator.to_string(), "Decorator");
}

// ============================================================================
// BehaviorNodeType — name()
// ============================================================================

#[test]
fn behavior_node_type_name() {
    assert_eq!(BehaviorNodeType::Root.name(), "Root");
    assert_eq!(BehaviorNodeType::Sequence.name(), "Sequence");
    assert_eq!(BehaviorNodeType::Selector.name(), "Selector");
    assert_eq!(BehaviorNodeType::Condition.name(), "Condition");
    assert_eq!(BehaviorNodeType::Action.name(), "Action");
    assert_eq!(BehaviorNodeType::Decorator.name(), "Decorator");
}

// ============================================================================
// BehaviorNodeType — icon()  (each must be unique & non-empty)
// ============================================================================

#[test]
fn behavior_node_type_icon_root() {
    assert_eq!(BehaviorNodeType::Root.icon(), "R");
}

#[test]
fn behavior_node_type_icon_sequence() {
    assert_eq!(BehaviorNodeType::Sequence.icon(), ">");
}

#[test]
fn behavior_node_type_icon_selector() {
    assert_eq!(BehaviorNodeType::Selector.icon(), "?");
}

#[test]
fn behavior_node_type_icon_condition() {
    assert_eq!(BehaviorNodeType::Condition.icon(), "?!");
}

#[test]
fn behavior_node_type_icon_action() {
    assert_eq!(BehaviorNodeType::Action.icon(), "A");
}

#[test]
fn behavior_node_type_icon_decorator() {
    assert_eq!(BehaviorNodeType::Decorator.icon(), "D");
}

#[test]
fn behavior_node_type_icons_all_unique() {
    let icons: Vec<&str> = BehaviorNodeType::all().iter().map(|t| t.icon()).collect();
    let mut deduped = icons.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(icons.len(), deduped.len(), "Icons must be unique");
}

// ============================================================================
// BehaviorNodeType — is_composite()
// ============================================================================

#[test]
fn behavior_node_type_is_composite() {
    assert!(BehaviorNodeType::Sequence.is_composite());
    assert!(BehaviorNodeType::Selector.is_composite());
    assert!(!BehaviorNodeType::Root.is_composite());
    assert!(!BehaviorNodeType::Condition.is_composite());
    assert!(!BehaviorNodeType::Action.is_composite());
    assert!(!BehaviorNodeType::Decorator.is_composite());
}

// ============================================================================
// BehaviorNodeType — is_leaf()
// ============================================================================

#[test]
fn behavior_node_type_is_leaf() {
    assert!(BehaviorNodeType::Condition.is_leaf());
    assert!(BehaviorNodeType::Action.is_leaf());
    assert!(!BehaviorNodeType::Root.is_leaf());
    assert!(!BehaviorNodeType::Sequence.is_leaf());
    assert!(!BehaviorNodeType::Selector.is_leaf());
    assert!(!BehaviorNodeType::Decorator.is_leaf());
}

// ============================================================================
// BehaviorNodeType — can_have_children()
// ============================================================================

#[test]
fn behavior_node_type_can_have_children() {
    assert!(BehaviorNodeType::Root.can_have_children());
    assert!(BehaviorNodeType::Sequence.can_have_children());
    assert!(BehaviorNodeType::Selector.can_have_children());
    assert!(BehaviorNodeType::Decorator.can_have_children());
    assert!(!BehaviorNodeType::Condition.can_have_children());
    assert!(!BehaviorNodeType::Action.can_have_children());
}

// ============================================================================
// BehaviorNodeType — all()
// ============================================================================

#[test]
fn behavior_node_type_all_count() {
    assert_eq!(BehaviorNodeType::all().len(), 6);
}

#[test]
fn behavior_node_type_all_contains_each() {
    let all = BehaviorNodeType::all();
    assert!(all.contains(&BehaviorNodeType::Root));
    assert!(all.contains(&BehaviorNodeType::Sequence));
    assert!(all.contains(&BehaviorNodeType::Selector));
    assert!(all.contains(&BehaviorNodeType::Condition));
    assert!(all.contains(&BehaviorNodeType::Action));
    assert!(all.contains(&BehaviorNodeType::Decorator));
}

// ============================================================================
// BehaviorNodeType — Default
// ============================================================================

#[test]
fn behavior_node_type_default_is_action() {
    assert_eq!(BehaviorNodeType::default(), BehaviorNodeType::Action);
}

// ============================================================================
// Cross-cutting: is_composite ⊕ is_leaf are disjoint and don't cover all
// ============================================================================

#[test]
fn behavior_node_type_composite_and_leaf_disjoint() {
    for nt in BehaviorNodeType::all() {
        // No type should be both composite and leaf
        assert!(
            !(nt.is_composite() && nt.is_leaf()),
            "{} is both composite and leaf",
            nt
        );
    }
}

#[test]
fn behavior_node_type_root_and_decorator_are_neither() {
    // Root and Decorator are neither composite nor leaf
    assert!(!BehaviorNodeType::Root.is_composite());
    assert!(!BehaviorNodeType::Root.is_leaf());
    assert!(!BehaviorNodeType::Decorator.is_composite());
    assert!(!BehaviorNodeType::Decorator.is_leaf());
}

// ============================================================================
// MaterialInfo — Default field values
// ============================================================================

#[test]
fn material_info_default_name() {
    let m = MaterialInfo::default();
    assert_eq!(m.name, "Default Material");
}

#[test]
fn material_info_default_albedo() {
    let m = MaterialInfo::default();
    assert_eq!(m.albedo_color, [0.8, 0.8, 0.8]);
}

#[test]
fn material_info_default_metallic_roughness() {
    let m = MaterialInfo::default();
    assert_eq!(m.metallic, 0.0);
    assert_eq!(m.roughness, 0.5);
}

#[test]
fn material_info_default_emission() {
    let m = MaterialInfo::default();
    assert_eq!(m.emission, [0.0, 0.0, 0.0]);
    assert_eq!(m.emission_strength, 1.0);
}

#[test]
fn material_info_default_normal_ao_alpha() {
    let m = MaterialInfo::default();
    assert_eq!(m.normal_strength, 1.0);
    assert_eq!(m.ao_strength, 1.0);
    assert_eq!(m.alpha, 1.0);
}

#[test]
fn material_info_default_double_sided_false() {
    let m = MaterialInfo::default();
    assert!(!m.double_sided);
}

#[test]
fn material_info_default_textures_all_none() {
    let m = MaterialInfo::default();
    assert!(m.albedo_texture.is_none());
    assert!(m.normal_texture.is_none());
    assert!(m.metallic_roughness_texture.is_none());
    assert!(m.emission_texture.is_none());
    assert!(m.ao_texture.is_none());
}

// ============================================================================
// AnimationState — Default field values
// ============================================================================

#[test]
fn animation_state_default_not_playing() {
    let s = AnimationState::default();
    assert!(!s.is_playing);
}

#[test]
fn animation_state_default_frames() {
    let s = AnimationState::default();
    assert_eq!(s.current_frame, 0);
    assert_eq!(s.total_frames, 120);
}

#[test]
fn animation_state_default_fps() {
    let s = AnimationState::default();
    assert_eq!(s.fps, 30.0);
}

#[test]
fn animation_state_default_playback_speed() {
    let s = AnimationState::default();
    assert_eq!(s.playback_speed, 1.0);
}

#[test]
fn animation_state_default_loop_enabled() {
    let s = AnimationState::default();
    assert!(s.loop_enabled);
}

#[test]
fn animation_state_default_ping_pong_off() {
    let s = AnimationState::default();
    assert!(!s.ping_pong);
}

#[test]
fn animation_state_default_no_selected_track() {
    let s = AnimationState::default();
    assert!(s.selected_track.is_none());
}

#[test]
fn animation_state_default_empty_tracks() {
    let s = AnimationState::default();
    assert!(s.tracks.is_empty());
}
