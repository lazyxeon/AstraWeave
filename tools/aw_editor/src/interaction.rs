use crate::command::{MoveEntityCommand, RotateEntityCommand, ScaleEntityCommand, UndoStack};
use crate::gizmo::state::{GizmoMode, GizmoState, TransformSnapshot};
use crate::prefab::PrefabManager;
use astraweave_core::{Entity, IVec2, World};
use glam::Quat;

/// High-level gizmo operation classification (used by telemetry/tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GizmoOperationKind {
    Translate,
    Rotate,
    Scale,
}

impl GizmoOperationKind {
    /// Returns all gizmo operation kinds.
    pub fn all() -> &'static [Self] {
        &[Self::Translate, Self::Rotate, Self::Scale]
    }

    /// Returns the display name for this operation.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Translate => "Translate",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
        }
    }

    /// Returns the icon for this operation.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Translate => "↔",
            Self::Rotate => "⟳",
            Self::Scale => "⤢",
        }
    }

    /// Returns the keyboard shortcut for this operation (Blender-style).
    pub fn shortcut(&self) -> &'static str {
        match self {
            Self::Translate => "G",
            Self::Rotate => "R",
            Self::Scale => "S",
        }
    }
}

impl std::fmt::Display for GizmoOperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Measurement payload captured when committing a gizmo operation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GizmoMeasurement {
    Translate {
        from: IVec2,
        to: IVec2,
    },
    Rotate {
        from: (f32, f32, f32),
        to: (f32, f32, f32),
    },
    Scale {
        from: f32,
        to: f32,
    },
}

impl GizmoMeasurement {
    /// Returns the operation kind for this measurement.
    pub fn kind(&self) -> GizmoOperationKind {
        match self {
            Self::Translate { .. } => GizmoOperationKind::Translate,
            Self::Rotate { .. } => GizmoOperationKind::Rotate,
            Self::Scale { .. } => GizmoOperationKind::Scale,
        }
    }

    /// Returns the magnitude of the change.
    pub fn magnitude(&self) -> f32 {
        match self {
            Self::Translate { from, to } => {
                let dx = (to.x - from.x) as f32;
                let dy = (to.y - from.y) as f32;
                (dx * dx + dy * dy).sqrt()
            }
            Self::Rotate { from, to } => {
                let dx = to.0 - from.0;
                let dy = to.1 - from.1;
                let dz = to.2 - from.2;
                (dx * dx + dy * dy + dz * dz).sqrt()
            }
            Self::Scale { from, to } => (to - from).abs(),
        }
    }

    /// Returns true if this represents a significant change.
    pub fn is_significant(&self) -> bool {
        self.magnitude() > 0.01
    }

    /// Returns a human-readable summary of this measurement.
    pub fn summary(&self) -> String {
        match self {
            Self::Translate { from, to } => {
                format!(
                    "Moved from ({}, {}) to ({}, {})",
                    from.x, from.y, to.x, to.y
                )
            }
            Self::Rotate { from, to } => {
                format!(
                    "Rotated from ({:.1}°, {:.1}°, {:.1}°) to ({:.1}°, {:.1}°, {:.1}°)",
                    from.0.to_degrees(),
                    from.1.to_degrees(),
                    from.2.to_degrees(),
                    to.0.to_degrees(),
                    to.1.to_degrees(),
                    to.2.to_degrees()
                )
            }
            Self::Scale { from, to } => {
                format!("Scaled from {:.2} to {:.2}", from, to)
            }
        }
    }
}

/// Metadata emitted when a gizmo operation commits successfully.
#[derive(Debug, Clone)]
pub struct GizmoCommitMetadata {
    pub entity: Entity,
    pub operation: GizmoOperationKind,
    pub measurement: GizmoMeasurement,
    pub constraint: Option<String>,
}

impl GizmoCommitMetadata {
    /// Returns true if this operation was constrained to an axis.
    pub fn is_constrained(&self) -> bool {
        self.constraint.is_some()
    }

    /// Returns a human-readable summary of this commit.
    pub fn summary(&self) -> String {
        let constraint_str = self.constraint.as_deref().unwrap_or("None");
        format!(
            "{} on entity {:?} (constraint: {})",
            self.measurement.summary(),
            self.entity,
            constraint_str
        )
    }
}

/// Metadata emitted when a gizmo operation is cancelled.
#[derive(Debug, Clone)]
pub struct GizmoCancelMetadata {
    pub entity: Entity,
    pub operation: GizmoOperationKind,
    pub snapshot: TransformSnapshot,
}

impl GizmoCancelMetadata {
    /// Get summary of cancelled operation
    pub fn summary(&self) -> String {
        format!(
            "{} cancelled on entity {:?}",
            self.operation.name(),
            self.entity
        )
    }

    /// Get the original position from snapshot
    pub fn original_position(&self) -> glam::Vec3 {
        self.snapshot.position
    }

    /// Get the original rotation from snapshot
    pub fn original_rotation(&self) -> glam::Quat {
        self.snapshot.rotation
    }

    /// Get the original scale from snapshot
    pub fn original_scale(&self) -> glam::Vec3 {
        self.snapshot.scale
    }
}

impl std::fmt::Display for GizmoCancelMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} cancelled", self.operation.name())
    }
}

impl std::fmt::Display for GizmoCommitMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} committed", self.operation.name())
    }
}

impl std::fmt::Display for GizmoMeasurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Translate { from, to } => {
                write!(f, "Move ({},{}) -> ({},{})", from.x, from.y, to.x, to.y)
            }
            Self::Rotate { from, to } => {
                write!(
                    f,
                    "Rotate ({:.1}°,{:.1}°,{:.1}°) -> ({:.1}°,{:.1}°,{:.1}°)",
                    from.0.to_degrees(),
                    from.1.to_degrees(),
                    from.2.to_degrees(),
                    to.0.to_degrees(),
                    to.1.to_degrees(),
                    to.2.to_degrees()
                )
            }
            Self::Scale { from, to } => {
                write!(f, "Scale {:.2} -> {:.2}", from, to)
            }
        }
    }
}

fn active_or_last_mode(state: &GizmoState) -> GizmoMode {
    match state.mode {
        GizmoMode::Inactive => state.last_mode,
        mode => mode,
    }
}

fn constraint_label(mode: GizmoMode) -> Option<String> {
    match mode {
        GizmoMode::Translate { constraint } | GizmoMode::Rotate { constraint } => {
            Some(format!("{:?}", constraint))
        }
        _ => None,
    }
}

fn mode_to_kind(mode: GizmoMode) -> Option<GizmoOperationKind> {
    match mode {
        GizmoMode::Translate { .. } => Some(GizmoOperationKind::Translate),
        GizmoMode::Rotate { .. } => Some(GizmoOperationKind::Rotate),
        GizmoMode::Scale { .. } => Some(GizmoOperationKind::Scale),
        GizmoMode::Inactive => None,
    }
}

/// Attempt to commit the active gizmo operation, pushing an undo command if needed.
///
/// **NEW (Nov 17)**: Optionally accepts a `PrefabManager` to auto-track overrides when
/// transforming prefab instances. If `prefab_manager` is `Some`, any committed transform
/// will automatically mark the entity as having overrides.
pub fn commit_active_gizmo(
    state: &mut GizmoState,
    world: &mut World,
    undo_stack: &mut UndoStack,
) -> Option<GizmoCommitMetadata> {
    commit_active_gizmo_with_prefab_tracking(state, world, undo_stack, None)
}

/// Extended version of `commit_active_gizmo` with auto-tracking for prefab overrides.
///
/// When `prefab_manager` is provided and the entity is part of a prefab instance,
/// successfully committed transforms will automatically call `track_override()` to
/// mark the entity as modified.
pub fn commit_active_gizmo_with_prefab_tracking(
    state: &mut GizmoState,
    world: &mut World,
    undo_stack: &mut UndoStack,
    prefab_manager: Option<&mut PrefabManager>,
) -> Option<GizmoCommitMetadata> {
    let entity = state.selected_entity?;
    let snapshot = state.start_transform?;
    let mode = active_or_last_mode(state);
    let operation = mode_to_kind(mode)?;
    let constraint = constraint_label(mode);

    let metadata = match (operation, world.pose(entity)) {
        (GizmoOperationKind::Translate, Some(pose)) => {
            let old_pos = IVec2 {
                x: snapshot.position.x.round() as i32,
                y: snapshot.position.z.round() as i32,
            };
            let new_pos = pose.pos;
            if old_pos == new_pos {
                None
            } else {
                undo_stack.push_executed(MoveEntityCommand::new(entity, old_pos, new_pos));
                Some(GizmoCommitMetadata {
                    entity,
                    operation,
                    measurement: GizmoMeasurement::Translate {
                        from: old_pos,
                        to: new_pos,
                    },
                    constraint,
                })
            }
        }
        (GizmoOperationKind::Rotate, Some(pose)) => {
            let old_rot = snapshot.rotation.to_euler(glam::EulerRot::XYZ);
            let new_rot = (pose.rotation_x, pose.rotation, pose.rotation_z);
            let changed = (old_rot.0 - new_rot.0).abs() > 0.01
                || (old_rot.1 - new_rot.1).abs() > 0.01
                || (old_rot.2 - new_rot.2).abs() > 0.01;
            if changed {
                undo_stack.push_executed(RotateEntityCommand::new(entity, old_rot, new_rot));
                Some(GizmoCommitMetadata {
                    entity,
                    operation,
                    measurement: GizmoMeasurement::Rotate {
                        from: old_rot,
                        to: new_rot,
                    },
                    constraint,
                })
            } else {
                None
            }
        }
        (GizmoOperationKind::Scale, Some(pose)) => {
            let old_scale = snapshot.scale.x;
            let new_scale = pose.scale;
            if (old_scale - new_scale).abs() > 0.01 {
                undo_stack.push_executed(ScaleEntityCommand::new(entity, old_scale, new_scale));
                Some(GizmoCommitMetadata {
                    entity,
                    operation,
                    measurement: GizmoMeasurement::Scale {
                        from: old_scale,
                        to: new_scale,
                    },
                    constraint: None,
                })
            } else {
                None
            }
        }
        _ => None,
    };

    state.start_transform = None;

    // Auto-track prefab override if entity is part of a prefab instance
    if let (Some(mgr), Some(meta)) = (prefab_manager, &metadata) {
        if let Some(instance) = mgr.find_instance_mut(meta.entity) {
            instance.track_override(meta.entity, world);
        }
    }

    metadata
}

/// Cancel the active gizmo operation and revert to the snapshot if available.
pub fn cancel_active_gizmo(
    state: &mut GizmoState,
    world: &mut World,
) -> Option<GizmoCancelMetadata> {
    let entity = state.selected_entity?;
    let snapshot = state.start_transform?;
    let mode = active_or_last_mode(state);
    let operation = mode_to_kind(mode)?;

    if let Some(pose) = world.pose_mut(entity) {
        pose.pos = IVec2 {
            x: snapshot.position.x.round() as i32,
            y: snapshot.position.z.round() as i32,
        };
        pose.height = snapshot.position.y;
        let (rx, ry, rz) = snapshot.rotation.to_euler(glam::EulerRot::XYZ);
        pose.rotation_x = rx;
        pose.rotation = ry;
        pose.rotation_z = rz;
        pose.scale = snapshot.scale.x;
    }

    state.start_transform = None;
    Some(GizmoCancelMetadata {
        entity,
        operation,
        snapshot,
    })
}

/// Ensure the gizmo has a starting snapshot sourced from the World.
pub fn ensure_world_snapshot(state: &mut GizmoState, world: &World) -> Option<TransformSnapshot> {
    if state.start_transform.is_some() {
        return state.start_transform;
    }
    let entity = state.selected_entity?;
    let pose = world.pose(entity)?;
    let snapshot = TransformSnapshot {
        position: glam::Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32),
        rotation: Quat::from_euler(
            glam::EulerRot::XYZ,
            pose.rotation_x,
            pose.rotation,
            pose.rotation_z,
        ),
        scale: glam::Vec3::splat(pose.scale),
    };
    state.start_transform = Some(snapshot);
    Some(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::UndoStack;
    use crate::gizmo::state::{AxisConstraint, GizmoMode, TransformSnapshot};
    use astraweave_core::{Entity, IVec2, Team, World};
    use glam::{Quat, Vec3};

    fn create_test_world() -> (World, Entity) {
        let mut world = World::new();
        let entity = world.spawn("TestEntity", IVec2::new(0, 0), Team { id: 0 }, 100, 10);
        (world, entity)
    }

    #[test]
    fn test_commit_translate_generates_command() {
        let (mut world, entity) = create_test_world();
        let mut undo_stack = UndoStack::new(100);

        let start_pos = Vec3::new(10.0, 0.0, 10.0);
        let snapshot = TransformSnapshot {
            position: start_pos,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        let mut state = GizmoState {
            selected_entity: Some(entity),
            start_transform: Some(snapshot),
            last_mode: GizmoMode::Translate {
                constraint: AxisConstraint::X,
            },
            mode: GizmoMode::Inactive,
            ..Default::default()
        };

        if let Some(pose) = world.pose_mut(entity) {
            pose.pos = IVec2::new(20, 20);
        }

        let meta = commit_active_gizmo(&mut state, &mut world, &mut undo_stack);

        assert!(meta.is_some());
        assert!(undo_stack.can_undo());

        let meta = meta.unwrap();
        match meta.measurement {
            GizmoMeasurement::Translate { from, to } => {
                assert_eq!(from, IVec2::new(10, 10));
                assert_eq!(to, IVec2::new(20, 20));
            }
            _ => panic!("Expected Translate measurement"),
        }

        assert!(state.start_transform.is_none());
    }

    #[test]
    fn test_commit_no_change_does_not_generate_command() {
        let (mut world, entity) = create_test_world();
        let mut undo_stack = UndoStack::new(100);

        let start_pos = Vec3::ZERO;
        let snapshot = TransformSnapshot {
            position: start_pos,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        let mut state = GizmoState {
            selected_entity: Some(entity),
            start_transform: Some(snapshot),
            last_mode: GizmoMode::Translate {
                constraint: AxisConstraint::X,
            },
            mode: GizmoMode::Inactive,
            ..Default::default()
        };

        let meta = commit_active_gizmo(&mut state, &mut world, &mut undo_stack);

        assert!(meta.is_none());
        assert!(!undo_stack.can_undo());
        assert!(state.start_transform.is_none());
    }

    #[test]
    fn test_cancel_gizmo_reverts_transform() {
        let (mut world, entity) = create_test_world();

        let start_pos = Vec3::new(5.0, 0.0, 5.0);
        let snapshot = TransformSnapshot {
            position: start_pos,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        let mut state = GizmoState {
            selected_entity: Some(entity),
            start_transform: Some(snapshot),
            last_mode: GizmoMode::Translate {
                constraint: AxisConstraint::X,
            },
            mode: GizmoMode::Translate {
                constraint: AxisConstraint::X,
            },
            ..Default::default()
        };

        if let Some(pose) = world.pose_mut(entity) {
            pose.pos = IVec2::new(100, 100);
        }

        let meta = cancel_active_gizmo(&mut state, &mut world);

        assert!(meta.is_some());

        if let Some(pose) = world.pose(entity) {
            assert_eq!(pose.pos, IVec2::new(5, 5));
        }
    }

    // ====================================================================
    // GizmoOperationKind Tests
    // ====================================================================

    #[test]
    fn test_gizmo_operation_kind_all() {
        let all = GizmoOperationKind::all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_gizmo_operation_kind_name() {
        assert_eq!(GizmoOperationKind::Translate.name(), "Translate");
        assert_eq!(GizmoOperationKind::Rotate.name(), "Rotate");
        assert_eq!(GizmoOperationKind::Scale.name(), "Scale");
    }

    #[test]
    fn test_gizmo_operation_kind_icon_not_empty() {
        for kind in GizmoOperationKind::all() {
            assert!(!kind.icon().is_empty());
        }
    }

    #[test]
    fn test_gizmo_operation_kind_shortcut() {
        assert_eq!(GizmoOperationKind::Translate.shortcut(), "G");
        assert_eq!(GizmoOperationKind::Rotate.shortcut(), "R");
        assert_eq!(GizmoOperationKind::Scale.shortcut(), "S");
    }

    #[test]
    fn test_gizmo_operation_kind_display() {
        assert_eq!(format!("{}", GizmoOperationKind::Translate), "Translate");
    }

    // ====================================================================
    // GizmoMeasurement Tests
    // ====================================================================

    #[test]
    fn test_gizmo_measurement_kind() {
        let translate = GizmoMeasurement::Translate {
            from: IVec2::new(0, 0),
            to: IVec2::new(10, 10),
        };
        assert_eq!(translate.kind(), GizmoOperationKind::Translate);

        let rotate = GizmoMeasurement::Rotate {
            from: (0.0, 0.0, 0.0),
            to: (1.0, 0.0, 0.0),
        };
        assert_eq!(rotate.kind(), GizmoOperationKind::Rotate);

        let scale = GizmoMeasurement::Scale { from: 1.0, to: 2.0 };
        assert_eq!(scale.kind(), GizmoOperationKind::Scale);
    }

    #[test]
    fn test_gizmo_measurement_magnitude_translate() {
        let m = GizmoMeasurement::Translate {
            from: IVec2::new(0, 0),
            to: IVec2::new(3, 4),
        };
        assert!((m.magnitude() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_gizmo_measurement_magnitude_scale() {
        let m = GizmoMeasurement::Scale { from: 1.0, to: 3.0 };
        assert!((m.magnitude() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_gizmo_measurement_is_significant() {
        let small = GizmoMeasurement::Scale {
            from: 1.0,
            to: 1.001,
        };
        assert!(!small.is_significant());

        let large = GizmoMeasurement::Scale { from: 1.0, to: 2.0 };
        assert!(large.is_significant());
    }

    #[test]
    fn test_gizmo_measurement_summary() {
        let m = GizmoMeasurement::Translate {
            from: IVec2::new(0, 0),
            to: IVec2::new(5, 10),
        };
        let summary = m.summary();
        assert!(summary.contains("Moved"));
        assert!(summary.contains("5"));
        assert!(summary.contains("10"));
    }

    // ====================================================================
    // GizmoCommitMetadata Tests
    // ====================================================================

    #[test]
    fn test_gizmo_commit_metadata_is_constrained() {
        let (_, entity) = create_test_world();

        let unconstrained = GizmoCommitMetadata {
            entity,
            operation: GizmoOperationKind::Translate,
            measurement: GizmoMeasurement::Translate {
                from: IVec2::new(0, 0),
                to: IVec2::new(1, 1),
            },
            constraint: None,
        };
        assert!(!unconstrained.is_constrained());

        let constrained = GizmoCommitMetadata {
            entity,
            operation: GizmoOperationKind::Translate,
            measurement: GizmoMeasurement::Translate {
                from: IVec2::new(0, 0),
                to: IVec2::new(1, 0),
            },
            constraint: Some("X".to_string()),
        };
        assert!(constrained.is_constrained());
    }

    #[test]
    fn test_gizmo_commit_metadata_summary() {
        let (_, entity) = create_test_world();

        let meta = GizmoCommitMetadata {
            entity,
            operation: GizmoOperationKind::Scale,
            measurement: GizmoMeasurement::Scale { from: 1.0, to: 2.0 },
            constraint: None,
        };
        let summary = meta.summary();
        assert!(summary.contains("Scaled"));
    }

    // ====================================================================
    // GizmoCancelMetadata New Methods Tests
    // ====================================================================

    #[test]
    fn test_gizmo_cancel_metadata_summary() {
        let (_, entity) = create_test_world();
        let snapshot = TransformSnapshot {
            position: Vec3::new(5.0, 0.0, 5.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let meta = GizmoCancelMetadata {
            entity,
            operation: GizmoOperationKind::Translate,
            snapshot,
        };
        let summary = meta.summary();
        assert!(summary.contains("Translate"));
        assert!(summary.contains("cancelled"));
    }

    #[test]
    fn test_gizmo_cancel_metadata_original_values() {
        let (_, entity) = create_test_world();
        let snapshot = TransformSnapshot {
            position: Vec3::new(10.0, 20.0, 30.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(2.0),
        };
        let meta = GizmoCancelMetadata {
            entity,
            operation: GizmoOperationKind::Translate,
            snapshot,
        };

        assert_eq!(meta.original_position(), Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(meta.original_rotation(), Quat::IDENTITY);
        assert_eq!(meta.original_scale(), Vec3::splat(2.0));
    }

    #[test]
    fn test_gizmo_cancel_metadata_display() {
        let (_, entity) = create_test_world();
        let snapshot = TransformSnapshot {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let meta = GizmoCancelMetadata {
            entity,
            operation: GizmoOperationKind::Rotate,
            snapshot,
        };
        let display = format!("{}", meta);
        assert!(display.contains("Rotate"));
        assert!(display.contains("cancelled"));
    }

    // ====================================================================
    // Display Trait Tests
    // ====================================================================

    #[test]
    fn test_gizmo_commit_metadata_display() {
        let (_, entity) = create_test_world();
        let meta = GizmoCommitMetadata {
            entity,
            operation: GizmoOperationKind::Scale,
            measurement: GizmoMeasurement::Scale { from: 1.0, to: 2.0 },
            constraint: None,
        };
        let display = format!("{}", meta);
        assert!(display.contains("Scale"));
        assert!(display.contains("committed"));
    }

    #[test]
    fn test_gizmo_measurement_display_translate() {
        let m = GizmoMeasurement::Translate {
            from: IVec2::new(0, 0),
            to: IVec2::new(5, 10),
        };
        let display = format!("{}", m);
        assert!(display.contains("Move"));
        assert!(display.contains("5"));
    }

    #[test]
    fn test_gizmo_measurement_display_rotate() {
        let m = GizmoMeasurement::Rotate {
            from: (0.0, 0.0, 0.0),
            to: (1.57, 0.0, 0.0),
        };
        let display = format!("{}", m);
        assert!(display.contains("Rotate"));
    }

    #[test]
    fn test_gizmo_measurement_display_scale() {
        let m = GizmoMeasurement::Scale { from: 1.0, to: 2.5 };
        let display = format!("{}", m);
        assert!(display.contains("Scale"));
        assert!(display.contains("1.00"));
        assert!(display.contains("2.50"));
    }
}
