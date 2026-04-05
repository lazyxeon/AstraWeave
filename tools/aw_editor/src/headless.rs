use crate::command::UndoStack;
use crate::gizmo::snapping::SnappingConfig;
use crate::gizmo::state::GizmoState;
use crate::interaction::{self, GizmoCancelMetadata, GizmoOperationKind};
use crate::telemetry::{self, EditorTelemetryEvent};
use anyhow::{Context, Result};
use astraweave_core::{Entity, IVec2, World};

/// Minimal harness that drives gizmo operations without a GPU/egui session.
///
/// This enables deterministic regression tests that mirror the viewport flow
/// (select → start gizmo → drag → confirm/cancel) entirely in headless mode.
pub struct GizmoHarness {
    world: World,
    undo_stack: UndoStack,
    gizmo: GizmoState,
    snapping: SnappingConfig,
}

impl GizmoHarness {
    pub fn new(world: World) -> Self {
        Self {
            world,
            undo_stack: UndoStack::new(64),
            gizmo: GizmoState::new(),
            snapping: SnappingConfig::default(),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn into_world(self) -> World {
        self.world
    }

    pub fn undo_stack(&self) -> &UndoStack {
        &self.undo_stack
    }

    pub fn select(&mut self, entity: Entity) {
        self.gizmo.selected_entity = Some(entity);
        telemetry::record(EditorTelemetryEvent::SelectionChanged {
            primary: Some(entity),
            count: 1,
        });
    }

    fn selected_entity(&self) -> Result<Entity> {
        self.gizmo
            .selected_entity
            .context("no entity selected in harness")
    }

    pub fn begin_translate(&mut self) -> Result<()> {
        let entity = self.selected_entity()?;
        self.gizmo.start_translate();
        interaction::ensure_world_snapshot(&mut self.gizmo, &self.world);
        telemetry::record(EditorTelemetryEvent::GizmoStarted {
            entity,
            operation: GizmoOperationKind::Translate,
        });
        Ok(())
    }

    /// Begin a rotation operation on the selected entity
    pub fn begin_rotate(&mut self) -> Result<()> {
        let entity = self.selected_entity()?;
        self.gizmo.start_rotate();
        interaction::ensure_world_snapshot(&mut self.gizmo, &self.world);
        telemetry::record(EditorTelemetryEvent::GizmoStarted {
            entity,
            operation: GizmoOperationKind::Rotate,
        });
        Ok(())
    }

    /// Begin a scale operation on the selected entity
    pub fn begin_scale(&mut self) -> Result<()> {
        let entity = self.selected_entity()?;
        self.gizmo.start_scale(true); // Uniform scaling by default
        interaction::ensure_world_snapshot(&mut self.gizmo, &self.world);
        telemetry::record(EditorTelemetryEvent::GizmoStarted {
            entity,
            operation: GizmoOperationKind::Scale,
        });
        Ok(())
    }

    pub fn drag_translate(&mut self, delta: IVec2) -> Result<()> {
        let entity = self.selected_entity()?;
        interaction::ensure_world_snapshot(&mut self.gizmo, &self.world)
            .context("failed to capture start snapshot for drag")?;

        if let Some(pose) = self.world.pose_mut(entity) {
            // Apply delta to get new position
            let new_x = pose.pos.x + delta.x;
            let new_y = pose.pos.y + delta.y;

            // Apply grid snapping if enabled
            if self.snapping.grid_enabled && self.snapping.grid_size > 0.0 {
                let grid = self.snapping.grid_size;
                pose.pos.x = ((new_x as f32 / grid).round() * grid) as i32;
                pose.pos.y = ((new_y as f32 / grid).round() * grid) as i32;
            } else {
                pose.pos.x = new_x;
                pose.pos.y = new_y;
            }
        }
        Ok(())
    }

    /// Drag rotate by the given angle in radians
    pub fn drag_rotate(&mut self, angle_radians: f32) -> Result<()> {
        let entity = self.selected_entity()?;
        interaction::ensure_world_snapshot(&mut self.gizmo, &self.world)
            .context("failed to capture start snapshot for rotate")?;

        if let Some(pose) = self.world.pose_mut(entity) {
            let new_rotation = pose.rotation + angle_radians;

            // Apply angle snapping if enabled
            pose.rotation = self.snapping.snap_angle(new_rotation);
        }
        Ok(())
    }

    /// Drag scale by the given factor (1.0 = no change, 2.0 = double size)
    pub fn drag_scale(&mut self, factor: f32) -> Result<()> {
        let entity = self.selected_entity()?;
        interaction::ensure_world_snapshot(&mut self.gizmo, &self.world)
            .context("failed to capture start snapshot for scale")?;

        if let Some(pose) = self.world.pose_mut(entity) {
            pose.scale *= factor;
        }
        Ok(())
    }

    pub fn confirm(&mut self) -> Result<()> {
        self.gizmo.confirm_transform();
        if let Some(metadata) =
            interaction::commit_active_gizmo(&mut self.gizmo, &mut self.world, &mut self.undo_stack)
        {
            telemetry::record(EditorTelemetryEvent::from(metadata));
        }
        self.gizmo.confirmed = false;
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<()> {
        self.gizmo.cancel_transform();
        if let Some(GizmoCancelMetadata {
            entity, operation, ..
        }) = interaction::cancel_active_gizmo(&mut self.gizmo, &mut self.world)
        {
            telemetry::record(EditorTelemetryEvent::GizmoCancelled { entity, operation });
        }
        self.gizmo.cancelled = false;
        Ok(())
    }

    pub fn snapping_config_mut(&mut self) -> &mut SnappingConfig {
        &mut self.snapping
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_stack.cursor()
    }

    pub fn undo_last(&mut self) -> Result<(), String> {
        self.undo_stack
            .undo(&mut self.world, None)
            .map_err(|e| format!("Undo failed: {}", e))
    }

    pub fn redo_last(&mut self) -> Result<(), String> {
        self.undo_stack
            .redo(&mut self.world, None)
            .map_err(|e| format!("Redo failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{Team, World};

    fn create_test_world() -> World {
        World::default()
    }

    fn create_world_with_entity() -> (World, Entity) {
        let mut world = World::default();
        let entity = world.spawn("test_entity", IVec2::new(0, 0), Team { id: 1 }, 100, 50);
        (world, entity)
    }

    // === GizmoHarness Construction Tests ===

    #[test]
    fn test_gizmo_harness_new() {
        let world = create_test_world();
        let harness = GizmoHarness::new(world);
        assert!(harness.undo_stack().is_empty());
    }

    #[test]
    fn test_gizmo_harness_world_access() {
        let world = create_test_world();
        let harness = GizmoHarness::new(world);
        // Just verify we can access the world
        let _ = harness.world();
    }

    #[test]
    fn test_gizmo_harness_world_mut_access() {
        let world = create_test_world();
        let mut harness = GizmoHarness::new(world);
        let entity = harness
            .world_mut()
            .spawn("entity", IVec2::new(0, 0), Team { id: 1 }, 100, 50);
        // Just verify we can spawn an entity and get its pose back
        let pose = harness.world().pose(entity);
        assert!(pose.is_some());
    }

    #[test]
    fn test_gizmo_harness_into_world() {
        let world = create_test_world();
        let harness = GizmoHarness::new(world);
        let _world = harness.into_world();
    }

    #[test]
    fn test_gizmo_harness_undo_stack_access() {
        let world = create_test_world();
        let harness = GizmoHarness::new(world);
        let stack = harness.undo_stack();
        assert_eq!(stack.cursor(), 0);
    }

    // === Selection Tests ===

    #[test]
    fn test_gizmo_harness_select_entity() {
        let (world, entity) = create_world_with_entity();
        let _guard = telemetry::enable_capture();
        let mut harness = GizmoHarness::new(world);

        harness.select(entity);

        let events = telemetry::drain_captured_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            EditorTelemetryEvent::SelectionChanged { primary, count } => {
                assert_eq!(*primary, Some(entity));
                assert_eq!(*count, 1);
            }
            _ => panic!("Expected SelectionChanged event"),
        }
    }

    // === Translate Operation Tests ===

    #[test]
    fn test_gizmo_harness_begin_translate_without_selection() {
        let world = create_test_world();
        let mut harness = GizmoHarness::new(world);

        let result = harness.begin_translate();
        assert!(result.is_err());
    }

    #[test]
    fn test_gizmo_harness_begin_translate_with_selection() {
        let (world, entity) = create_world_with_entity();
        let _guard = telemetry::enable_capture();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);

        let result = harness.begin_translate();
        assert!(result.is_ok());

        let events = telemetry::drain_captured_events();
        assert!(events.len() >= 2); // Selection + GizmoStarted
        let gizmo_started = events
            .iter()
            .find(|e| matches!(e, EditorTelemetryEvent::GizmoStarted { .. }));
        assert!(gizmo_started.is_some());
    }

    #[test]
    fn test_gizmo_harness_drag_translate() {
        let (world, entity) = create_world_with_entity();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);
        harness.begin_translate().unwrap();

        let initial_pos = harness.world().pose(entity).map(|p| p.pos);
        let result = harness.drag_translate(IVec2::new(10, 5));
        assert!(result.is_ok());

        let new_pos = harness.world().pose(entity).map(|p| p.pos);
        if let (Some(initial), Some(new)) = (initial_pos, new_pos) {
            assert_eq!(new.x, initial.x + 10);
            assert_eq!(new.y, initial.y + 5);
        }
    }

    #[test]
    fn test_gizmo_harness_drag_translate_with_grid_snapping() {
        let (world, entity) = create_world_with_entity();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);
        harness.snapping_config_mut().grid_enabled = true;
        harness.snapping_config_mut().grid_size = 10.0;
        harness.begin_translate().unwrap();

        let result = harness.drag_translate(IVec2::new(7, 13));
        assert!(result.is_ok());

        // With grid size 10, 7 rounds to 10 and 13 rounds to 10
        let new_pos = harness.world().pose(entity).map(|p| p.pos);
        if let Some(pos) = new_pos {
            // Grid snapping rounds to nearest grid point
            assert_eq!(pos.x % 10, 0);
            assert_eq!(pos.y % 10, 0);
        }
    }

    // === Rotate Operation Tests ===

    #[test]
    fn test_gizmo_harness_begin_rotate_without_selection() {
        let world = create_test_world();
        let mut harness = GizmoHarness::new(world);

        let result = harness.begin_rotate();
        assert!(result.is_err());
    }

    #[test]
    fn test_gizmo_harness_begin_rotate_with_selection() {
        let (world, entity) = create_world_with_entity();
        let _guard = telemetry::enable_capture();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);

        let result = harness.begin_rotate();
        assert!(result.is_ok());

        let events = telemetry::drain_captured_events();
        let gizmo_started = events.iter().find(|e| {
            matches!(
                e,
                EditorTelemetryEvent::GizmoStarted {
                    operation: GizmoOperationKind::Rotate,
                    ..
                }
            )
        });
        assert!(gizmo_started.is_some());
    }

    #[test]
    fn test_gizmo_harness_drag_rotate() {
        let (world, entity) = create_world_with_entity();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);
        harness.begin_rotate().unwrap();

        let initial_rotation = harness
            .world()
            .pose(entity)
            .map(|p| p.rotation)
            .unwrap_or(0.0);
        let result = harness.drag_rotate(std::f32::consts::FRAC_PI_4); // 45 degrees
        assert!(result.is_ok());

        let new_rotation = harness
            .world()
            .pose(entity)
            .map(|p| p.rotation)
            .unwrap_or(0.0);
        assert!(
            (new_rotation - initial_rotation - std::f32::consts::FRAC_PI_4).abs() < 0.01
                || harness.snapping_config_mut().angle_enabled
        );
    }

    // === Scale Operation Tests ===

    #[test]
    fn test_gizmo_harness_begin_scale_without_selection() {
        let world = create_test_world();
        let mut harness = GizmoHarness::new(world);

        let result = harness.begin_scale();
        assert!(result.is_err());
    }

    #[test]
    fn test_gizmo_harness_begin_scale_with_selection() {
        let (world, entity) = create_world_with_entity();
        let _guard = telemetry::enable_capture();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);

        let result = harness.begin_scale();
        assert!(result.is_ok());

        let events = telemetry::drain_captured_events();
        let gizmo_started = events.iter().find(|e| {
            matches!(
                e,
                EditorTelemetryEvent::GizmoStarted {
                    operation: GizmoOperationKind::Scale,
                    ..
                }
            )
        });
        assert!(gizmo_started.is_some());
    }

    #[test]
    fn test_gizmo_harness_drag_scale() {
        let (world, entity) = create_world_with_entity();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);
        harness.begin_scale().unwrap();

        let initial_scale = harness.world().pose(entity).map(|p| p.scale).unwrap_or(1.0);
        let result = harness.drag_scale(2.0);
        assert!(result.is_ok());

        let new_scale = harness.world().pose(entity).map(|p| p.scale).unwrap_or(1.0);
        assert!((new_scale - initial_scale * 2.0).abs() < 0.01);
    }

    // === Confirm/Cancel Tests ===

    #[test]
    fn test_gizmo_harness_confirm() {
        let (world, entity) = create_world_with_entity();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);
        harness.begin_translate().unwrap();
        harness.drag_translate(IVec2::new(5, 5)).unwrap();

        let result = harness.confirm();
        assert!(result.is_ok());
    }

    #[test]
    fn test_gizmo_harness_cancel() {
        let (world, entity) = create_world_with_entity();
        let mut harness = GizmoHarness::new(world);
        harness.select(entity);
        harness.begin_translate().unwrap();
        harness.drag_translate(IVec2::new(5, 5)).unwrap();

        let result = harness.cancel();
        assert!(result.is_ok());
    }

    // === Snapping Config Tests ===

    #[test]
    fn test_gizmo_harness_snapping_config_mut() {
        let world = create_test_world();
        let mut harness = GizmoHarness::new(world);

        harness.snapping_config_mut().grid_enabled = true;
        harness.snapping_config_mut().grid_size = 5.0;

        assert!(harness.snapping_config_mut().grid_enabled);
        assert_eq!(harness.snapping_config_mut().grid_size, 5.0);
    }

    // === Undo/Redo Tests ===

    #[test]
    fn test_gizmo_harness_undo_depth_initial() {
        let world = create_test_world();
        let harness = GizmoHarness::new(world);
        assert_eq!(harness.undo_depth(), 0);
    }

    #[test]
    fn test_gizmo_harness_undo_on_empty_stack() {
        let world = create_test_world();
        let mut harness = GizmoHarness::new(world);

        // On empty stack, undo returns Ok() (no-op)
        let result = harness.undo_last();
        assert!(result.is_ok());
    }

    #[test]
    fn test_gizmo_harness_redo_on_empty_stack() {
        let world = create_test_world();
        let mut harness = GizmoHarness::new(world);

        // On empty stack, redo returns Ok() (no-op)
        let result = harness.redo_last();
        assert!(result.is_ok());
    }
}
