//! Wave 2 Mutation-Resistant Tests: Gizmo State, Snapping, Picking, Prefab & Panels
//!
//! Targets mutation-prone patterns across gizmo submodule, prefab.rs, panel_type.rs:
//! - GizmoMode variant classification (is_translate, is_rotate, is_scale, is_active)
//! - GizmoMode icon/name/shortcut strings
//! - GizmoMode constraint extraction
//! - AxisConstraint axis_vector arithmetic (x/y/z components)
//! - AxisConstraint cycle state machine (3-press cycle + axis switching)
//! - AxisConstraint is_planar vs is_single_axis classification
//! - AxisConstraint color RGB values (axis-specific)
//! - SnappingConfig snap_position grid math (round * grid_size)
//! - SnappingConfig snap_angle radian conversion
//! - SnappingConfig disabled bypass
//! - GizmoHandle axis mapping, color mapping, to_constraint, mode
//! - GizmoHandle is_translate/is_rotate/is_scale
//! - Ray point_at distance math
//! - apply_constraint masking
//! - PrefabStats percentage arithmetic, zero-division guards
//! - PrefabIssue classification (is_critical, is_file_issue, is_entity_issue)
//! - PrefabIssue path extraction
//! - EntityOverrides field counting
//! - PanelType category mapping, closable logic, scroll logic
//! - PanelCategory display format

use aw_editor_lib::gizmo::constraints::apply_constraint;
use aw_editor_lib::gizmo::picking::{GizmoHandle, Ray};
use aw_editor_lib::gizmo::snapping::SnappingConfig;
use aw_editor_lib::gizmo::state::{AxisConstraint, GizmoMode, TransformSnapshot};
use aw_editor_lib::panel_type::{PanelCategory, PanelType};
use aw_editor_lib::prefab::{EntityOverrides, PrefabHierarchySnapshot, PrefabIssue, PrefabStats};
use glam::{Quat, Vec3};
use std::path::PathBuf;

// ============================================================================
// Section A: GizmoMode — Variant Classification
// ============================================================================

#[test]
fn gizmo_mode_inactive_default() {
    assert_eq!(GizmoMode::default(), GizmoMode::Inactive);
}

#[test]
fn gizmo_mode_is_active_false_for_inactive() {
    assert!(!GizmoMode::Inactive.is_active());
}

#[test]
fn gizmo_mode_is_active_true_for_translate() {
    let m = GizmoMode::Translate {
        constraint: AxisConstraint::None,
    };
    assert!(m.is_active());
}

#[test]
fn gizmo_mode_is_active_true_for_rotate() {
    let m = GizmoMode::Rotate {
        constraint: AxisConstraint::X,
    };
    assert!(m.is_active());
}

#[test]
fn gizmo_mode_is_active_true_for_scale() {
    let m = GizmoMode::Scale {
        constraint: AxisConstraint::None,
        uniform: true,
    };
    assert!(m.is_active());
}

#[test]
fn gizmo_mode_is_translate_only_for_translate() {
    assert!(GizmoMode::Translate {
        constraint: AxisConstraint::None
    }
    .is_translate());
    assert!(!GizmoMode::Rotate {
        constraint: AxisConstraint::None
    }
    .is_translate());
    assert!(!GizmoMode::Scale {
        constraint: AxisConstraint::None,
        uniform: false
    }
    .is_translate());
    assert!(!GizmoMode::Inactive.is_translate());
}

#[test]
fn gizmo_mode_is_rotate_only_for_rotate() {
    assert!(!GizmoMode::Translate {
        constraint: AxisConstraint::None
    }
    .is_rotate());
    assert!(GizmoMode::Rotate {
        constraint: AxisConstraint::None
    }
    .is_rotate());
    assert!(!GizmoMode::Scale {
        constraint: AxisConstraint::None,
        uniform: false
    }
    .is_rotate());
    assert!(!GizmoMode::Inactive.is_rotate());
}

#[test]
fn gizmo_mode_is_scale_only_for_scale() {
    assert!(!GizmoMode::Translate {
        constraint: AxisConstraint::None
    }
    .is_scale());
    assert!(!GizmoMode::Rotate {
        constraint: AxisConstraint::None
    }
    .is_scale());
    assert!(GizmoMode::Scale {
        constraint: AxisConstraint::None,
        uniform: false
    }
    .is_scale());
    assert!(!GizmoMode::Inactive.is_scale());
}

#[test]
fn gizmo_mode_constraint_returns_none_for_inactive() {
    assert!(GizmoMode::Inactive.constraint().is_none());
}

#[test]
fn gizmo_mode_constraint_returns_some_for_active() {
    let t = GizmoMode::Translate {
        constraint: AxisConstraint::X,
    };
    assert_eq!(t.constraint(), Some(AxisConstraint::X));

    let r = GizmoMode::Rotate {
        constraint: AxisConstraint::YZ,
    };
    assert_eq!(r.constraint(), Some(AxisConstraint::YZ));

    let s = GizmoMode::Scale {
        constraint: AxisConstraint::Z,
        uniform: false,
    };
    assert_eq!(s.constraint(), Some(AxisConstraint::Z));
}

// ============================================================================
// Section B: GizmoMode — String Properties
// ============================================================================

#[test]
fn gizmo_mode_name_matches() {
    assert_eq!(GizmoMode::Inactive.name(), "Inactive");
    assert_eq!(
        GizmoMode::Translate {
            constraint: AxisConstraint::X
        }
        .name(),
        "Translate"
    );
    assert_eq!(
        GizmoMode::Rotate {
            constraint: AxisConstraint::None
        }
        .name(),
        "Rotate"
    );
    assert_eq!(
        GizmoMode::Scale {
            constraint: AxisConstraint::None,
            uniform: false
        }
        .name(),
        "Scale"
    );
}

#[test]
fn gizmo_mode_icon_nonempty_per_variant() {
    for mode in GizmoMode::all() {
        assert!(!mode.icon().is_empty());
    }
}

#[test]
fn gizmo_mode_shortcut_none_for_inactive() {
    assert!(GizmoMode::Inactive.shortcut().is_none());
}

#[test]
fn gizmo_mode_shortcut_g_for_translate() {
    let m = GizmoMode::Translate {
        constraint: AxisConstraint::None,
    };
    assert_eq!(m.shortcut(), Some("G"));
}

#[test]
fn gizmo_mode_shortcut_r_for_rotate() {
    let m = GizmoMode::Rotate {
        constraint: AxisConstraint::None,
    };
    assert_eq!(m.shortcut(), Some("R"));
}

#[test]
fn gizmo_mode_shortcut_s_for_scale() {
    let m = GizmoMode::Scale {
        constraint: AxisConstraint::None,
        uniform: false,
    };
    assert_eq!(m.shortcut(), Some("S"));
}

#[test]
fn gizmo_mode_display_translate_includes_constraint() {
    let m = GizmoMode::Translate {
        constraint: AxisConstraint::X,
    };
    let d = format!("{}", m);
    assert!(d.contains("Translate"));
    assert!(d.contains("X"));
}

#[test]
fn gizmo_mode_display_scale_uniform() {
    let m = GizmoMode::Scale {
        constraint: AxisConstraint::None,
        uniform: true,
    };
    let d = format!("{}", m);
    assert!(d.contains("Uniform"));
}

#[test]
fn gizmo_mode_all_has_four_variants() {
    assert_eq!(GizmoMode::all().len(), 4);
}

// ============================================================================
// Section C: AxisConstraint — Vector Arithmetic
// ============================================================================

#[test]
fn axis_none_vector_is_one() {
    assert_eq!(AxisConstraint::None.axis_vector(), Vec3::ONE);
}

#[test]
fn axis_x_vector() {
    assert_eq!(AxisConstraint::X.axis_vector(), Vec3::X);
}

#[test]
fn axis_y_vector() {
    assert_eq!(AxisConstraint::Y.axis_vector(), Vec3::Y);
}

#[test]
fn axis_z_vector() {
    assert_eq!(AxisConstraint::Z.axis_vector(), Vec3::Z);
}

#[test]
fn axis_xy_vector() {
    let v = AxisConstraint::XY.axis_vector();
    assert_eq!(v, Vec3::new(1.0, 1.0, 0.0));
}

#[test]
fn axis_xz_vector() {
    let v = AxisConstraint::XZ.axis_vector();
    assert_eq!(v, Vec3::new(1.0, 0.0, 1.0));
}

#[test]
fn axis_yz_vector() {
    let v = AxisConstraint::YZ.axis_vector();
    assert_eq!(v, Vec3::new(0.0, 1.0, 1.0));
}

// ============================================================================
// Section D: AxisConstraint — Classification
// ============================================================================

#[test]
fn axis_is_planar_only_for_plane_constraints() {
    assert!(!AxisConstraint::None.is_planar());
    assert!(!AxisConstraint::X.is_planar());
    assert!(!AxisConstraint::Y.is_planar());
    assert!(!AxisConstraint::Z.is_planar());
    assert!(AxisConstraint::XY.is_planar());
    assert!(AxisConstraint::XZ.is_planar());
    assert!(AxisConstraint::YZ.is_planar());
}

#[test]
fn axis_is_single_axis_only_for_single() {
    assert!(!AxisConstraint::None.is_single_axis());
    assert!(AxisConstraint::X.is_single_axis());
    assert!(AxisConstraint::Y.is_single_axis());
    assert!(AxisConstraint::Z.is_single_axis());
    assert!(!AxisConstraint::XY.is_single_axis());
    assert!(!AxisConstraint::XZ.is_single_axis());
    assert!(!AxisConstraint::YZ.is_single_axis());
}

#[test]
fn axis_all_has_seven() {
    assert_eq!(AxisConstraint::all().len(), 7);
}

#[test]
fn axis_default_is_none() {
    assert_eq!(AxisConstraint::default(), AxisConstraint::None);
}

// ============================================================================
// Section E: AxisConstraint — Cycle State Machine
// ============================================================================

#[test]
fn cycle_none_press_x_gives_x() {
    assert_eq!(
        AxisConstraint::None.cycle(AxisConstraint::X),
        AxisConstraint::X
    );
}

#[test]
fn cycle_none_press_y_gives_y() {
    assert_eq!(
        AxisConstraint::None.cycle(AxisConstraint::Y),
        AxisConstraint::Y
    );
}

#[test]
fn cycle_none_press_z_gives_z() {
    assert_eq!(
        AxisConstraint::None.cycle(AxisConstraint::Z),
        AxisConstraint::Z
    );
}

#[test]
fn cycle_x_press_x_gives_yz_plane() {
    assert_eq!(
        AxisConstraint::X.cycle(AxisConstraint::X),
        AxisConstraint::YZ
    );
}

#[test]
fn cycle_y_press_y_gives_xz_plane() {
    assert_eq!(
        AxisConstraint::Y.cycle(AxisConstraint::Y),
        AxisConstraint::XZ
    );
}

#[test]
fn cycle_z_press_z_gives_xy_plane() {
    assert_eq!(
        AxisConstraint::Z.cycle(AxisConstraint::Z),
        AxisConstraint::XY
    );
}

#[test]
fn cycle_yz_press_x_gives_none() {
    assert_eq!(
        AxisConstraint::YZ.cycle(AxisConstraint::X),
        AxisConstraint::None
    );
}

#[test]
fn cycle_xz_press_y_gives_none() {
    assert_eq!(
        AxisConstraint::XZ.cycle(AxisConstraint::Y),
        AxisConstraint::None
    );
}

#[test]
fn cycle_xy_press_z_gives_none() {
    assert_eq!(
        AxisConstraint::XY.cycle(AxisConstraint::Z),
        AxisConstraint::None
    );
}

#[test]
fn cycle_x_press_y_switches_to_y() {
    assert_eq!(
        AxisConstraint::X.cycle(AxisConstraint::Y),
        AxisConstraint::Y
    );
}

#[test]
fn cycle_full_triple_press_x() {
    // None → X → YZ → None
    let s1 = AxisConstraint::None.cycle(AxisConstraint::X);
    assert_eq!(s1, AxisConstraint::X);
    let s2 = s1.cycle(AxisConstraint::X);
    assert_eq!(s2, AxisConstraint::YZ);
    let s3 = s2.cycle(AxisConstraint::X);
    assert_eq!(s3, AxisConstraint::None);
}

// ============================================================================
// Section F: AxisConstraint — Color & Display
// ============================================================================

#[test]
fn axis_color_red_for_x() {
    let c = AxisConstraint::X.color();
    assert!(c[0] > 0.9); // Red
    assert!(c[1] < 0.3);
    assert!(c[2] < 0.3);
}

#[test]
fn axis_color_green_for_y() {
    let c = AxisConstraint::Y.color();
    assert!(c[0] < 0.3);
    assert!(c[1] > 0.9); // Green
    assert!(c[2] < 0.3);
}

#[test]
fn axis_color_blue_for_z() {
    let c = AxisConstraint::Z.color();
    assert!(c[0] < 0.5);
    assert!(c[1] < 0.5);
    assert!(c[2] > 0.9); // Blue
}

#[test]
fn axis_color_white_for_none() {
    let c = AxisConstraint::None.color();
    assert_eq!(c, [1.0, 1.0, 1.0]);
}

#[test]
fn axis_display_free_for_none() {
    assert_eq!(format!("{}", AxisConstraint::None), "Free");
}

#[test]
fn axis_display_x_for_x() {
    assert_eq!(format!("{}", AxisConstraint::X), "X");
}

#[test]
fn axis_display_plane_for_xy() {
    let d = format!("{}", AxisConstraint::XY);
    assert!(d.contains("XY") && d.contains("Plane"));
}

#[test]
fn axis_name_unique_per_variant() {
    let names: Vec<&str> = AxisConstraint::all().iter().map(|a| a.name()).collect();
    let unique: std::collections::HashSet<&&str> = names.iter().collect();
    assert_eq!(names.len(), unique.len());
}

#[test]
fn axis_key_some_for_xyz_none_for_others() {
    assert!(AxisConstraint::X.key().is_some());
    assert!(AxisConstraint::Y.key().is_some());
    assert!(AxisConstraint::Z.key().is_some());
    assert!(AxisConstraint::None.key().is_none());
    assert!(AxisConstraint::XY.key().is_none());
    assert!(AxisConstraint::XZ.key().is_none());
    assert!(AxisConstraint::YZ.key().is_none());
}

// ============================================================================
// Section G: SnappingConfig
// ============================================================================

#[test]
fn snap_default_grid_size_one() {
    let cfg = SnappingConfig::default();
    assert_eq!(cfg.grid_size, 1.0);
    assert_eq!(cfg.angle_increment, 15.0);
    assert!(cfg.grid_enabled);
    assert!(cfg.angle_enabled);
}

#[test]
fn snap_position_rounds_to_grid() {
    let cfg = SnappingConfig::default();
    let snapped = cfg.snap_position(Vec3::new(1.7, 2.3, -0.4));
    assert_eq!(snapped, Vec3::new(2.0, 2.0, 0.0));
}

#[test]
fn snap_position_half_grid() {
    let cfg = SnappingConfig::default().with_grid_size(0.5);
    let snapped = cfg.snap_position(Vec3::new(1.7, 2.3, -0.4));
    assert_eq!(snapped, Vec3::new(1.5, 2.5, -0.5));
}

#[test]
fn snap_position_disabled_returns_original() {
    let mut cfg = SnappingConfig::default();
    cfg.grid_enabled = false;
    let p = Vec3::new(1.7, 2.3, -0.4);
    assert_eq!(cfg.snap_position(p), p);
}

#[test]
fn snap_position_zero_grid_size_returns_original() {
    let mut cfg = SnappingConfig::default();
    cfg.grid_size = 0.0;
    let p = Vec3::new(1.7, 2.3, -0.4);
    assert_eq!(cfg.snap_position(p), p);
}

#[test]
fn snap_position_negative_grid_size_returns_original() {
    let mut cfg = SnappingConfig::default();
    cfg.grid_size = -1.0;
    let p = Vec3::new(1.7, 2.3, -0.4);
    assert_eq!(cfg.snap_position(p), p);
}

#[test]
fn snap_angle_default_15_degrees() {
    let cfg = SnappingConfig::default();
    let snapped = cfg.snap_angle(23.0_f32.to_radians());
    assert!((snapped - 30.0_f32.to_radians()).abs() < 0.001);
}

#[test]
fn snap_angle_disabled_returns_original() {
    let mut cfg = SnappingConfig::default();
    cfg.angle_enabled = false;
    let angle = 23.0_f32.to_radians();
    assert_eq!(cfg.snap_angle(angle), angle);
}

#[test]
fn snap_angle_zero_increment_returns_original() {
    let mut cfg = SnappingConfig::default();
    cfg.angle_increment = 0.0;
    let angle = 23.0_f32.to_radians();
    assert_eq!(cfg.snap_angle(angle), angle);
}

#[test]
fn snap_rotation_snaps_angle() {
    let cfg = SnappingConfig::default();
    let rot = Quat::from_rotation_z(23.0_f32.to_radians());
    let snapped = cfg.snap_rotation(rot);
    let (_, snapped_angle) = snapped.to_axis_angle();
    assert!((snapped_angle - 30.0_f32.to_radians()).abs() < 0.001);
}

#[test]
fn snap_rotation_disabled_returns_original() {
    let mut cfg = SnappingConfig::default();
    cfg.angle_enabled = false;
    let rot = Quat::from_rotation_z(23.0_f32.to_radians());
    let snapped = cfg.snap_rotation(rot);
    // Should be the same (bitwise might differ so check angle)
    let (_, orig_angle) = rot.to_axis_angle();
    let (_, snap_angle) = snapped.to_axis_angle();
    assert!((orig_angle - snap_angle).abs() < 0.0001);
}

#[test]
fn snap_builder_pattern() {
    let cfg = SnappingConfig::new()
        .with_grid_size(2.5)
        .with_angle_increment(45.0);
    assert_eq!(cfg.grid_size, 2.5);
    assert_eq!(cfg.angle_increment, 45.0);
}

// ============================================================================
// Section H: GizmoHandle
// ============================================================================

#[test]
fn gizmo_handle_all_has_ten() {
    assert_eq!(GizmoHandle::all().len(), 10);
}

#[test]
fn gizmo_handle_is_translate() {
    assert!(GizmoHandle::TranslateX.is_translate());
    assert!(GizmoHandle::TranslateY.is_translate());
    assert!(GizmoHandle::TranslateZ.is_translate());
    assert!(!GizmoHandle::RotateX.is_translate());
    assert!(!GizmoHandle::ScaleX.is_translate());
    assert!(!GizmoHandle::ScaleUniform.is_translate());
}

#[test]
fn gizmo_handle_is_rotate() {
    assert!(!GizmoHandle::TranslateX.is_rotate());
    assert!(GizmoHandle::RotateX.is_rotate());
    assert!(GizmoHandle::RotateY.is_rotate());
    assert!(GizmoHandle::RotateZ.is_rotate());
    assert!(!GizmoHandle::ScaleX.is_rotate());
}

#[test]
fn gizmo_handle_is_scale() {
    assert!(!GizmoHandle::TranslateX.is_scale());
    assert!(!GizmoHandle::RotateX.is_scale());
    assert!(GizmoHandle::ScaleX.is_scale());
    assert!(GizmoHandle::ScaleY.is_scale());
    assert!(GizmoHandle::ScaleZ.is_scale());
    assert!(GizmoHandle::ScaleUniform.is_scale());
}

#[test]
fn gizmo_handle_axis_char() {
    assert_eq!(GizmoHandle::TranslateX.axis(), 'X');
    assert_eq!(GizmoHandle::TranslateY.axis(), 'Y');
    assert_eq!(GizmoHandle::TranslateZ.axis(), 'Z');
    assert_eq!(GizmoHandle::RotateX.axis(), 'X');
    assert_eq!(GizmoHandle::RotateY.axis(), 'Y');
    assert_eq!(GizmoHandle::ScaleZ.axis(), 'Z');
    assert_eq!(GizmoHandle::ScaleUniform.axis(), 'U');
}

#[test]
fn gizmo_handle_color_red_for_x() {
    let c = GizmoHandle::TranslateX.color();
    assert!(c[0] > 0.9 && c[1] < 0.3 && c[2] < 0.3);
    assert_eq!(c, GizmoHandle::RotateX.color());
    assert_eq!(c, GizmoHandle::ScaleX.color());
}

#[test]
fn gizmo_handle_color_green_for_y() {
    let c = GizmoHandle::TranslateY.color();
    assert!(c[1] > 0.9);
    assert_eq!(c, GizmoHandle::RotateY.color());
}

#[test]
fn gizmo_handle_color_blue_for_z() {
    let c = GizmoHandle::TranslateZ.color();
    assert!(c[2] > 0.9);
}

#[test]
fn gizmo_handle_color_white_for_uniform() {
    assert_eq!(GizmoHandle::ScaleUniform.color(), [1.0, 1.0, 1.0]);
}

#[test]
fn gizmo_handle_to_constraint() {
    assert_eq!(GizmoHandle::TranslateX.to_constraint(), AxisConstraint::X);
    assert_eq!(GizmoHandle::TranslateY.to_constraint(), AxisConstraint::Y);
    assert_eq!(GizmoHandle::TranslateZ.to_constraint(), AxisConstraint::Z);
    assert_eq!(
        GizmoHandle::ScaleUniform.to_constraint(),
        AxisConstraint::None
    );
}

#[test]
fn gizmo_handle_mode_translate() {
    let mode = GizmoHandle::TranslateX.mode();
    assert!(mode.is_translate());
    assert_eq!(mode.constraint(), Some(AxisConstraint::X));
}

#[test]
fn gizmo_handle_mode_rotate() {
    let mode = GizmoHandle::RotateY.mode();
    assert!(mode.is_rotate());
    assert_eq!(mode.constraint(), Some(AxisConstraint::Y));
}

#[test]
fn gizmo_handle_mode_scale_uniform() {
    let mode = GizmoHandle::ScaleUniform.mode();
    assert!(mode.is_scale());
    assert_eq!(mode.constraint(), Some(AxisConstraint::None));
}

#[test]
fn gizmo_handle_name_matches_display() {
    for handle in GizmoHandle::all() {
        assert_eq!(handle.name(), &format!("{}", handle));
    }
}

// ============================================================================
// Section I: Ray & Constraint Application
// ============================================================================

#[test]
fn ray_point_at_origin_plus_direction() {
    let ray = Ray {
        origin: Vec3::ZERO,
        direction: Vec3::X,
    };
    assert_eq!(ray.point_at(5.0), Vec3::new(5.0, 0.0, 0.0));
}

#[test]
fn ray_point_at_zero_returns_origin() {
    let ray = Ray {
        origin: Vec3::new(1.0, 2.0, 3.0),
        direction: Vec3::Y,
    };
    assert_eq!(ray.point_at(0.0), Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn ray_point_at_negative_goes_backward() {
    let ray = Ray {
        origin: Vec3::ZERO,
        direction: Vec3::Z,
    };
    assert_eq!(ray.point_at(-3.0), Vec3::new(0.0, 0.0, -3.0));
}

#[test]
fn apply_constraint_x_masks_yz() {
    let input = Vec3::new(5.0, 3.0, 2.0);
    let result = apply_constraint(input, AxisConstraint::X);
    assert_eq!(result, Vec3::new(5.0, 0.0, 0.0));
}

#[test]
fn apply_constraint_y_masks_xz() {
    let input = Vec3::new(5.0, 3.0, 2.0);
    let result = apply_constraint(input, AxisConstraint::Y);
    assert_eq!(result, Vec3::new(0.0, 3.0, 0.0));
}

#[test]
fn apply_constraint_z_masks_xy() {
    let input = Vec3::new(5.0, 3.0, 2.0);
    let result = apply_constraint(input, AxisConstraint::Z);
    assert_eq!(result, Vec3::new(0.0, 0.0, 2.0));
}

#[test]
fn apply_constraint_xy_masks_z() {
    let input = Vec3::new(5.0, 3.0, 2.0);
    let result = apply_constraint(input, AxisConstraint::XY);
    assert_eq!(result, Vec3::new(5.0, 3.0, 0.0));
}

#[test]
fn apply_constraint_none_passes_through() {
    let input = Vec3::new(5.0, 3.0, 2.0);
    let result = apply_constraint(input, AxisConstraint::None);
    assert_eq!(result, input);
}

// ============================================================================
// Section J: TransformSnapshot
// ============================================================================

#[test]
fn transform_snapshot_default() {
    let t = TransformSnapshot::default();
    assert_eq!(t.position, Vec3::ZERO);
    assert_eq!(t.rotation, Quat::IDENTITY);
    assert_eq!(t.scale, Vec3::ONE);
}

// ============================================================================
// Section K: PrefabStats
// ============================================================================

#[test]
fn prefab_stats_default_zeros() {
    let s = PrefabStats::default();
    assert_eq!(s.instance_count, 0);
    assert_eq!(s.total_prefab_entities, 0);
    assert!(!s.has_overrides());
}

#[test]
fn prefab_stats_avg_entities_zero_instances() {
    let s = PrefabStats {
        instance_count: 0,
        total_prefab_entities: 0,
        ..Default::default()
    };
    assert_eq!(s.avg_entities_per_instance(), 0.0);
}

#[test]
fn prefab_stats_avg_entities_calculation() {
    let s = PrefabStats {
        instance_count: 4,
        total_prefab_entities: 12,
        ..Default::default()
    };
    assert!((s.avg_entities_per_instance() - 3.0).abs() < 0.001);
}

#[test]
fn prefab_stats_override_percentage_zero() {
    let s = PrefabStats {
        total_prefab_entities: 0,
        ..Default::default()
    };
    assert_eq!(s.override_percentage(), 0.0);
}

#[test]
fn prefab_stats_override_percentage_calculation() {
    let s = PrefabStats {
        total_prefab_entities: 200,
        overridden_entity_count: 50,
        ..Default::default()
    };
    assert!((s.override_percentage() - 25.0).abs() < 0.001);
}

#[test]
fn prefab_stats_has_overrides() {
    let mut s = PrefabStats::default();
    assert!(!s.has_overrides());
    s.overridden_entity_count = 1;
    assert!(s.has_overrides());
}

// ============================================================================
// Section L: PrefabIssue — Classification
// ============================================================================

#[test]
fn prefab_issue_is_critical() {
    assert!(PrefabIssue::MissingFile {
        path: PathBuf::from("a.ron")
    }
    .is_critical());
    assert!(PrefabIssue::CyclicReference {
        path: PathBuf::from("b.ron")
    }
    .is_critical());
    assert!(PrefabIssue::InvalidRootIndex {
        path: PathBuf::from("c.ron"),
        index: 5,
        entity_count: 3
    }
    .is_critical());
    assert!(!PrefabIssue::OrphanedEntity {
        entity: 1,
        prefab: PathBuf::from("d.ron")
    }
    .is_critical());
    assert!(!PrefabIssue::EmptyPrefab {
        path: PathBuf::from("e.ron")
    }
    .is_critical());
    assert!(!PrefabIssue::EmptyMapping {
        prefab: PathBuf::from("f.ron")
    }
    .is_critical());
}

#[test]
fn prefab_issue_is_file_issue() {
    assert!(PrefabIssue::MissingFile {
        path: PathBuf::from("a.ron")
    }
    .is_file_issue());
    assert!(!PrefabIssue::OrphanedEntity {
        entity: 1,
        prefab: PathBuf::from("b.ron")
    }
    .is_file_issue());
    assert!(!PrefabIssue::CyclicReference {
        path: PathBuf::from("c.ron")
    }
    .is_file_issue());
}

#[test]
fn prefab_issue_is_entity_issue() {
    assert!(!PrefabIssue::MissingFile {
        path: PathBuf::from("a.ron")
    }
    .is_entity_issue());
    assert!(PrefabIssue::OrphanedEntity {
        entity: 1,
        prefab: PathBuf::from("b.ron")
    }
    .is_entity_issue());
    assert!(!PrefabIssue::EmptyPrefab {
        path: PathBuf::from("c.ron")
    }
    .is_entity_issue());
}

#[test]
fn prefab_issue_path_always_some() {
    let issues = vec![
        PrefabIssue::MissingFile {
            path: PathBuf::from("a.ron"),
        },
        PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: PathBuf::from("b.ron"),
        },
        PrefabIssue::EmptyPrefab {
            path: PathBuf::from("c.ron"),
        },
        PrefabIssue::EmptyMapping {
            prefab: PathBuf::from("d.ron"),
        },
        PrefabIssue::CyclicReference {
            path: PathBuf::from("e.ron"),
        },
        PrefabIssue::InvalidRootIndex {
            path: PathBuf::from("f.ron"),
            index: 5,
            entity_count: 3,
        },
    ];
    for issue in &issues {
        assert!(
            issue.path().is_some(),
            "path() should be Some for {:?}",
            issue
        );
    }
}

#[test]
fn prefab_issue_name_nonempty() {
    let issues = vec![
        PrefabIssue::MissingFile {
            path: PathBuf::from("a.ron"),
        },
        PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: PathBuf::from("b.ron"),
        },
        PrefabIssue::EmptyPrefab {
            path: PathBuf::from("c.ron"),
        },
        PrefabIssue::EmptyMapping {
            prefab: PathBuf::from("d.ron"),
        },
        PrefabIssue::CyclicReference {
            path: PathBuf::from("e.ron"),
        },
        PrefabIssue::InvalidRootIndex {
            path: PathBuf::from("f.ron"),
            index: 5,
            entity_count: 3,
        },
    ];
    for issue in &issues {
        assert!(!issue.name().is_empty());
    }
}

#[test]
fn prefab_issue_icon_nonempty() {
    let issues = vec![
        PrefabIssue::MissingFile {
            path: PathBuf::from("a.ron"),
        },
        PrefabIssue::OrphanedEntity {
            entity: 1,
            prefab: PathBuf::from("b.ron"),
        },
        PrefabIssue::CyclicReference {
            path: PathBuf::from("c.ron"),
        },
        PrefabIssue::InvalidRootIndex {
            path: PathBuf::from("d.ron"),
            index: 1,
            entity_count: 0,
        },
    ];
    for issue in &issues {
        assert!(!issue.icon().is_empty());
    }
}

#[test]
fn prefab_issue_display_contains_path() {
    let issue = PrefabIssue::MissingFile {
        path: PathBuf::from("my_prefab.ron"),
    };
    let d = format!("{}", issue);
    assert!(d.contains("my_prefab.ron"));
}

#[test]
fn prefab_issue_display_invalid_root_contains_index() {
    let issue = PrefabIssue::InvalidRootIndex {
        path: PathBuf::from("test.ron"),
        index: 5,
        entity_count: 3,
    };
    let d = format!("{}", issue);
    assert!(d.contains("5"));
    assert!(d.contains("3"));
}

#[test]
fn prefab_issue_all_variants_count() {
    assert_eq!(PrefabIssue::all_variants().len(), 6);
}

// ============================================================================
// Section M: EntityOverrides
// ============================================================================

#[test]
fn entity_overrides_default_empty() {
    let o = EntityOverrides::default();
    assert!(!o.has_any_override());
    assert!(!o.has_pose_override());
    assert!(!o.has_health_override());
    assert_eq!(o.override_count(), 0);
}

#[test]
fn entity_overrides_pose_x_only() {
    let o = EntityOverrides {
        pos_x: Some(10),
        ..Default::default()
    };
    assert!(o.has_pose_override());
    assert!(!o.has_health_override());
    assert!(o.has_any_override());
    assert_eq!(o.override_count(), 1);
}

#[test]
fn entity_overrides_pose_y_only() {
    let o = EntityOverrides {
        pos_y: Some(20),
        ..Default::default()
    };
    assert!(o.has_pose_override());
    assert_eq!(o.override_count(), 1);
}

#[test]
fn entity_overrides_health_only() {
    let o = EntityOverrides {
        health: Some(50),
        ..Default::default()
    };
    assert!(!o.has_pose_override());
    assert!(o.has_health_override());
    assert!(o.has_any_override());
    assert_eq!(o.override_count(), 1);
}

#[test]
fn entity_overrides_max_health_only() {
    let o = EntityOverrides {
        max_health: Some(200),
        ..Default::default()
    };
    assert!(o.has_health_override());
    assert_eq!(o.override_count(), 1);
}

#[test]
fn entity_overrides_all_fields() {
    let o = EntityOverrides {
        pos_x: Some(1),
        pos_y: Some(2),
        health: Some(50),
        max_health: Some(100),
    };
    assert!(o.has_pose_override());
    assert!(o.has_health_override());
    assert!(o.has_any_override());
    assert_eq!(o.override_count(), 4);
}

// ============================================================================
// Section N: PrefabHierarchySnapshot
// ============================================================================

#[test]
fn hierarchy_snapshot_empty() {
    let snap = PrefabHierarchySnapshot::new();
    assert!(snap.children_of(42).is_empty());
}

#[test]
fn hierarchy_snapshot_insert_children() {
    let mut snap = PrefabHierarchySnapshot::new();
    snap.insert_children(1, vec![2, 3, 4]);
    assert_eq!(snap.children_of(1), &[2, 3, 4]);
    assert!(snap.children_of(99).is_empty());
}

#[test]
fn hierarchy_snapshot_add_child() {
    let mut snap = PrefabHierarchySnapshot::new();
    snap.add_child(1, 10);
    snap.add_child(1, 20);
    assert_eq!(snap.children_of(1), &[10, 20]);
}

#[test]
fn hierarchy_snapshot_from_iter() {
    let snap: PrefabHierarchySnapshot = vec![(1, vec![2, 3]), (4, vec![5])].into_iter().collect();
    assert_eq!(snap.children_of(1), &[2, 3]);
    assert_eq!(snap.children_of(4), &[5]);
}

// ============================================================================
// Section O: PanelType — Category & Properties
// ============================================================================

#[test]
fn panel_type_all_nonempty() {
    assert!(PanelType::all().len() >= 20);
}

#[test]
fn panel_type_viewport_not_closable() {
    assert!(!PanelType::Viewport.is_closable());
}

#[test]
fn panel_type_others_closable() {
    assert!(PanelType::Hierarchy.is_closable());
    assert!(PanelType::Inspector.is_closable());
    assert!(PanelType::Console.is_closable());
    assert!(PanelType::Profiler.is_closable());
}

#[test]
fn panel_type_viewport_no_scroll() {
    assert!(!PanelType::Viewport.has_scroll());
}

#[test]
fn panel_type_graph_no_scroll() {
    assert!(!PanelType::Graph.has_scroll());
    assert!(!PanelType::BehaviorGraph.has_scroll());
}

#[test]
fn panel_type_others_have_scroll() {
    assert!(PanelType::Hierarchy.has_scroll());
    assert!(PanelType::Console.has_scroll());
    assert!(PanelType::Inspector.has_scroll());
}

#[test]
fn panel_type_hierarchy_in_scene_category() {
    assert_eq!(PanelType::Hierarchy.category(), PanelCategory::Scene);
    assert_eq!(PanelType::Inspector.category(), PanelCategory::Scene);
    assert_eq!(PanelType::Viewport.category(), PanelCategory::Scene);
}

#[test]
fn panel_type_console_in_debug_category() {
    assert_eq!(PanelType::Console.category(), PanelCategory::Debug);
    assert_eq!(PanelType::Profiler.category(), PanelCategory::Debug);
}

#[test]
fn panel_type_terrain_in_content_category() {
    assert_eq!(PanelType::Terrain.category(), PanelCategory::Content);
    assert_eq!(PanelType::MaterialEditor.category(), PanelCategory::Content);
}

#[test]
fn panel_type_is_debug_panel() {
    assert!(PanelType::Console.is_debug_panel());
    assert!(PanelType::Profiler.is_debug_panel());
    assert!(!PanelType::Hierarchy.is_debug_panel());
}

#[test]
fn panel_type_is_content_panel() {
    assert!(PanelType::Terrain.is_content_panel());
    assert!(PanelType::MaterialEditor.is_content_panel());
    assert!(!PanelType::Console.is_content_panel());
}

#[test]
fn panel_type_in_category_returns_correct_panels() {
    let debug = PanelType::in_category(PanelCategory::Debug);
    assert!(debug.contains(&PanelType::Console));
    assert!(debug.contains(&PanelType::Profiler));
    assert!(!debug.contains(&PanelType::Hierarchy));
}

#[test]
fn panel_type_title_nonempty_for_all() {
    for p in PanelType::all() {
        assert!(!p.title().is_empty());
    }
}

#[test]
fn panel_type_icon_nonempty_for_all() {
    for p in PanelType::all() {
        assert!(!p.icon().is_empty());
    }
}

#[test]
fn panel_type_description_nonempty_for_all() {
    for p in PanelType::all() {
        assert!(!p.description().is_empty());
    }
}

#[test]
fn panel_type_display_contains_icon_and_title() {
    let p = PanelType::Hierarchy;
    let d = format!("{}", p);
    assert!(d.contains("Hierarchy"));
    assert!(d.contains("[H]"));
}

#[test]
fn panel_type_default_layout_nonempty() {
    assert!(!PanelType::default_left_panels().is_empty());
    assert!(!PanelType::default_right_panels().is_empty());
    assert!(!PanelType::default_bottom_panels().is_empty());
    assert_eq!(PanelType::default_center_panel(), PanelType::Viewport);
}

// ============================================================================
// Section P: PanelCategory
// ============================================================================

#[test]
fn panel_category_all_has_six() {
    assert_eq!(PanelCategory::all().len(), 6);
}

#[test]
fn panel_category_name_nonempty_for_all() {
    for c in PanelCategory::all() {
        assert!(!c.name().is_empty());
    }
}

#[test]
fn panel_category_icon_nonempty_for_all() {
    for c in PanelCategory::all() {
        assert!(!c.icon().is_empty());
    }
}

#[test]
fn panel_category_display_contains_icon_and_name() {
    let d = format!("{}", PanelCategory::Scene);
    assert!(d.contains("Scene"));
    assert!(d.contains("[Scn]"));
}
