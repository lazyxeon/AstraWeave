use crate::entity_manager::{EditorEntity, EntityId};
use crate::gizmo::scene_viewport::Transform;
use crate::gizmo::state::TransformSnapshot;
use astraweave_core::{Entity, IVec2, Pose, World};
use glam::{EulerRot, Quat, Vec3};
use std::collections::HashMap;

/// Abstraction used by the viewport to read and mutate scene data
/// without knowing about the concrete storage implementation.
pub trait TransformableScene {
    /// Access the underlying simulation world (read-only).
    fn world(&self) -> &World;

    /// Access the underlying simulation world (mutable).
    fn world_mut(&mut self) -> &mut World;

    /// Rebuild every cached entity (expensive; use after bulk edits).
    fn sync_all(&mut self);

    /// Rebuild a cached snapshot for the given entity so UI surfaces stay
    /// in sync with the authoritative `World` after edits.
    fn sync_entity(&mut self, entity: Entity);

    /// Build a transform snapshot for the requested entity.
    fn snapshot_for(&self, entity: Entity) -> Option<TransformSnapshot>;

    /// Apply a previously captured snapshot back onto the world/cache.
    fn apply_snapshot(&mut self, entity: Entity, snapshot: &TransformSnapshot);
}

/// Canonical scene data for Edit mode.
///
/// `EditorSceneState` owns the authoritative `World` plus lightweight UI caches
/// that expose 3D transforms to egui panels and the gizmo pipeline.
pub struct EditorSceneState {
    world: World,
    cache: HashMap<Entity, EditorEntity>,
}

impl EditorSceneState {
    /// Construct scene state from an existing world snapshot.
    pub fn new(world: World) -> Self {
        let mut state = Self {
            world,
            cache: HashMap::new(),
        };
        state.sync_all();
        state
    }

    /// Immutable access to the ECS world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Mutable access to the ECS world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Get or create a cached EditorEntity for the given entity.
    /// Returns a mutable reference if the entity exists in the world.
    pub fn get_editor_entity_mut(&mut self, entity: Entity) -> Option<&mut EditorEntity> {
        self.upsert_cache_entry(entity)
    }

    /// Transform helper exposed to panels (position in 3D space).
    pub fn transform_for(&self, entity: Entity) -> Option<Transform> {
        self.world.pose(entity).map(pose_to_transform)
    }

    /// Apply a panel-authored transform back into the world/cache.
    pub fn apply_transform(&mut self, entity: Entity, transform: &Transform) {
        if let Some(pose) = self.world.pose_mut(entity) {
            pose.pos = IVec2 {
                x: transform.position.x.round() as i32,
                y: transform.position.z.round() as i32,
            };
            pose.height = transform.position.y;
            let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);
            pose.rotation_x = rx;
            pose.rotation = ry;
            pose.rotation_z = rz;
            pose.scale = transform.scale.x;
        }
        self.sync_entity(entity);
    }

    /// Sync every cached entity (used when loading a scene).
    pub fn sync_all(&mut self) {
        let entities = self.world.entities();
        for entity in entities {
            self.sync_entity(entity);
        }
    }

    fn upsert_cache_entry(&mut self, entity: Entity) -> Option<&mut EditorEntity> {
        let pose = match self.world.pose(entity) {
            Some(pose) => pose,
            None => {
                self.cache.remove(&entity);
                return None;
            }
        };

        let name = self
            .world
            .name(entity)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Entity_{}", entity));

        let entry = self.cache.entry(entity).or_insert_with(|| EditorEntity {
            id: entity as EntityId,
            name,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            mesh: None,
            material: crate::entity_manager::EntityMaterial::new(),
            components: HashMap::new(),
            parent: None,
        });

        entry.position = Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32);
        entry.rotation = Quat::from_euler(
            EulerRot::XYZ,
            pose.rotation_x,
            pose.rotation,
            pose.rotation_z,
        );
        entry.scale = Vec3::splat(pose.scale);

        Some(entry)
    }
}

impl TransformableScene for EditorSceneState {
    fn world(&self) -> &World {
        &self.world
    }

    fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    fn sync_all(&mut self) {
        EditorSceneState::sync_all(self);
    }

    fn sync_entity(&mut self, entity: Entity) {
        if self.upsert_cache_entry(entity).is_none() {
            self.cache.remove(&entity);
        }
    }

    fn snapshot_for(&self, entity: Entity) -> Option<TransformSnapshot> {
        self.world.pose(entity).map(|pose| TransformSnapshot {
            position: Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                pose.rotation_x,
                pose.rotation,
                pose.rotation_z,
            ),
            scale: Vec3::splat(pose.scale),
        })
    }

    fn apply_snapshot(&mut self, entity: Entity, snapshot: &TransformSnapshot) {
        if let Some(pose) = self.world.pose_mut(entity) {
            pose.pos = IVec2 {
                x: snapshot.position.x.round() as i32,
                y: snapshot.position.z.round() as i32,
            };
            pose.height = snapshot.position.y;
            let (rx, ry, rz) = snapshot.rotation.to_euler(EulerRot::XYZ);
            pose.rotation_x = rx;
            pose.rotation = ry;
            pose.rotation_z = rz;
            pose.scale = snapshot.scale.x;
        }
        self.sync_entity(entity);
    }
}

fn pose_to_transform(pose: Pose) -> Transform {
    Transform {
        position: Vec3::new(pose.pos.x as f32, pose.height, pose.pos.y as f32),
        rotation: Quat::from_euler(
            EulerRot::XYZ,
            pose.rotation_x,
            pose.rotation,
            pose.rotation_z,
        ),
        scale: Vec3::splat(pose.scale),
    }
}

/// Scene state statistics
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneStateStats {
    /// Total entities in world
    pub entity_count: usize,
    /// Entities cached for editor display
    pub cached_entity_count: usize,
    /// Cache hit rate (cached / world entities)
    pub cache_coverage: f32,
}

/// Scene state validation issue
#[derive(Debug, Clone, PartialEq)]
pub struct SceneStateIssue {
    pub entity: Entity,
    pub message: String,
    pub is_error: bool,
}

impl SceneStateIssue {
    /// Create a new error issue
    pub fn error(entity: Entity, message: impl Into<String>) -> Self {
        Self {
            entity,
            message: message.into(),
            is_error: true,
        }
    }

    /// Create a new warning issue
    pub fn warning(entity: Entity, message: impl Into<String>) -> Self {
        Self {
            entity,
            message: message.into(),
            is_error: false,
        }
    }
}

impl EditorSceneState {
    /// Get statistics about the scene state
    pub fn stats(&self) -> SceneStateStats {
        let entity_count = self.world.entities().len();
        let cached_entity_count = self.cache.len();
        let cache_coverage = if entity_count == 0 {
            1.0
        } else {
            cached_entity_count as f32 / entity_count as f32
        };

        SceneStateStats {
            entity_count,
            cached_entity_count,
            cache_coverage,
        }
    }

    /// Validate the scene state and return any issues found
    pub fn validate(&self) -> Vec<SceneStateIssue> {
        let mut issues = Vec::new();

        // Check for stale cache entries (entity no longer in world)
        for &entity in self.cache.keys() {
            if self.world.pose(entity).is_none() {
                issues.push(SceneStateIssue::warning(
                    entity,
                    "Cached entity no longer exists in world",
                ));
            }
        }

        // Check for uncached world entities
        for entity in self.world.entities() {
            if !self.cache.contains_key(&entity) {
                issues.push(SceneStateIssue::warning(
                    entity,
                    "World entity not in cache",
                ));
            }
        }

        // Check for invalid transforms
        for (&entity, cached) in &self.cache {
            if !cached.scale.is_finite() || cached.scale.x <= 0.0 {
                issues.push(SceneStateIssue::error(
                    entity,
                    format!("Invalid scale: {:?}", cached.scale),
                ));
            }
            if !cached.position.is_finite() {
                issues.push(SceneStateIssue::error(
                    entity,
                    format!("Invalid position: {:?}", cached.position),
                ));
            }
            if !cached.rotation.is_finite() || !cached.rotation.is_normalized() {
                issues.push(SceneStateIssue::warning(
                    entity,
                    "Rotation quaternion is not normalized",
                ));
            }
        }

        issues
    }

    /// Check if the scene state is valid (no errors)
    pub fn is_valid(&self) -> bool {
        !self.validate().iter().any(|issue| issue.is_error)
    }

    /// Find entities within a radius of a position
    pub fn find_entities_near(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        let radius_sq = radius * radius;
        self.cache
            .iter()
            .filter(|(_, cached)| {
                let dist_sq = (cached.position - center).length_squared();
                dist_sq <= radius_sq
            })
            .map(|(&entity, _)| entity)
            .collect()
    }

    /// Find entities by name pattern (case-insensitive)
    pub fn find_entities_by_name(&self, pattern: &str) -> Vec<Entity> {
        let pattern_lower = pattern.to_lowercase();
        self.cache
            .iter()
            .filter(|(_, cached)| cached.name.to_lowercase().contains(&pattern_lower))
            .map(|(&entity, _)| entity)
            .collect()
    }

    /// Get the cached editor entity for display
    pub fn get_editor_entity(&self, entity: Entity) -> Option<&EditorEntity> {
        self.cache.get(&entity)
    }

    /// Get all cached entities
    pub fn all_cached_entities(&self) -> impl Iterator<Item = (Entity, &EditorEntity)> {
        self.cache.iter().map(|(&e, cached)| (e, cached))
    }

    /// Clear all cached entities (will be rebuilt on next sync)
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.world.entities().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_manager::EntityMaterial;
    use std::collections::HashMap as StdHashMap;

    /// Helper to create a minimal EditorEntity for testing
    fn make_test_entity(id: u64, name: &str, position: Vec3, scale: Vec3) -> EditorEntity {
        EditorEntity {
            id,
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
            scale,
            mesh: None,
            material: EntityMaterial::new(),
            components: StdHashMap::new(),
            parent: None,
        }
    }

    /// Helper to create EditorEntity with custom rotation
    fn make_test_entity_with_rotation(
        id: u64,
        name: &str,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) -> EditorEntity {
        EditorEntity {
            id,
            name: name.to_string(),
            position,
            rotation,
            scale,
            mesh: None,
            material: EntityMaterial::new(),
            components: StdHashMap::new(),
            parent: None,
        }
    }

    #[test]
    fn test_scene_state_stats_default() {
        let stats = SceneStateStats {
            entity_count: 0,
            cached_entity_count: 0,
            cache_coverage: 0.0,
        };
        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.cached_entity_count, 0);
        assert_eq!(stats.cache_coverage, 0.0);
    }

    #[test]
    fn test_scene_state_issue_error() {
        let issue = SceneStateIssue::error(42, "test error");
        assert_eq!(issue.entity, 42);
        assert_eq!(issue.message, "test error");
        assert!(issue.is_error);
    }

    #[test]
    fn test_scene_state_issue_warning() {
        let issue = SceneStateIssue::warning(99, "test warning");
        assert_eq!(issue.entity, 99);
        assert_eq!(issue.message, "test warning");
        assert!(!issue.is_error);
    }

    #[test]
    fn test_editor_scene_state_new() {
        let world = World::default();
        let state = EditorSceneState::new(world);
        assert_eq!(state.entity_count(), 0);
    }

    #[test]
    fn test_editor_scene_state_stats_empty() {
        let world = World::default();
        let state = EditorSceneState::new(world);
        let stats = state.stats();
        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.cached_entity_count, 0);
        // Empty world has 100% coverage (0/0 = 1.0)
        assert_eq!(stats.cache_coverage, 1.0);
    }

    #[test]
    fn test_editor_scene_state_validate_empty() {
        let world = World::default();
        let state = EditorSceneState::new(world);
        let issues = state.validate();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_editor_scene_state_is_valid_empty() {
        let world = World::default();
        let state = EditorSceneState::new(world);
        assert!(state.is_valid());
    }

    #[test]
    fn test_editor_scene_state_clear_cache() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        // Manually add to cache
        let entity: Entity = 1;
        state
            .cache
            .insert(entity, make_test_entity(1, "Test", Vec3::ZERO, Vec3::ONE));

        assert_eq!(state.cache.len(), 1);
        state.clear_cache();
        assert!(state.cache.is_empty());
    }

    #[test]
    fn test_editor_scene_state_get_editor_entity() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        let entity: Entity = 5;
        state.cache.insert(
            entity,
            make_test_entity(5, "Player", Vec3::new(1.0, 2.0, 3.0), Vec3::ONE),
        );

        let result = state.get_editor_entity(entity);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Player");

        let missing = state.get_editor_entity(999);
        assert!(missing.is_none());
    }

    #[test]
    fn test_editor_scene_state_find_entities_near() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        // Add entities at various positions
        state.cache.insert(
            1,
            make_test_entity(1, "Near1", Vec3::new(1.0, 0.0, 0.0), Vec3::ONE),
        );
        state.cache.insert(
            2,
            make_test_entity(2, "Near2", Vec3::new(0.0, 1.0, 0.0), Vec3::ONE),
        );
        state.cache.insert(
            3,
            make_test_entity(3, "Far", Vec3::new(100.0, 100.0, 100.0), Vec3::ONE),
        );

        let near = state.find_entities_near(Vec3::ZERO, 2.0);
        assert_eq!(near.len(), 2);

        let all = state.find_entities_near(Vec3::ZERO, 200.0);
        assert_eq!(all.len(), 3);

        let none = state.find_entities_near(Vec3::new(-1000.0, 0.0, 0.0), 1.0);
        assert!(none.is_empty());
    }

    #[test]
    fn test_editor_scene_state_find_entities_by_name() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        state
            .cache
            .insert(1, make_test_entity(1, "Player", Vec3::ZERO, Vec3::ONE));
        state
            .cache
            .insert(2, make_test_entity(2, "Enemy_Alpha", Vec3::ZERO, Vec3::ONE));
        state
            .cache
            .insert(3, make_test_entity(3, "Enemy_Beta", Vec3::ZERO, Vec3::ONE));

        let enemies = state.find_entities_by_name("enemy");
        assert_eq!(enemies.len(), 2);

        let alpha = state.find_entities_by_name("Alpha");
        assert_eq!(alpha.len(), 1);

        let player = state.find_entities_by_name("PLAYER");
        assert_eq!(player.len(), 1);

        let missing = state.find_entities_by_name("nonexistent");
        assert!(missing.is_empty());
    }

    #[test]
    fn test_editor_scene_state_all_cached_entities() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        state
            .cache
            .insert(1, make_test_entity(1, "A", Vec3::ZERO, Vec3::ONE));
        state
            .cache
            .insert(2, make_test_entity(2, "B", Vec3::ZERO, Vec3::ONE));

        let all: Vec<_> = state.all_cached_entities().collect();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_editor_scene_state_stats_with_cache() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        state
            .cache
            .insert(1, make_test_entity(1, "Test", Vec3::ZERO, Vec3::ONE));

        let stats = state.stats();
        assert_eq!(stats.cached_entity_count, 1);
    }

    #[test]
    fn test_editor_scene_state_validate_negative_scale() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        state.cache.insert(
            1,
            make_test_entity(1, "BadScale", Vec3::ZERO, Vec3::new(-1.0, 1.0, 1.0)),
        );

        let issues = state.validate();
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.is_error));
    }

    #[test]
    fn test_editor_scene_state_validate_nan_position() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        state.cache.insert(
            1,
            make_test_entity(1, "BadPosition", Vec3::new(f32::NAN, 0.0, 0.0), Vec3::ONE),
        );

        let issues = state.validate();
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.is_error));
    }

    #[test]
    fn test_editor_scene_state_validate_unnormalized_rotation() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        // Create an unnormalized quaternion
        state.cache.insert(
            1,
            make_test_entity_with_rotation(
                1,
                "BadRotation",
                Vec3::ZERO,
                Quat::from_xyzw(1.0, 1.0, 1.0, 1.0), // Not normalized
                Vec3::ONE,
            ),
        );

        let issues = state.validate();
        assert!(!issues.is_empty());
        // Unnormalized rotation is a warning, not an error
        assert!(issues.iter().any(|i| !i.is_error));
    }

    #[test]
    fn test_editor_scene_state_is_valid_with_warnings_only() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        // Unnormalized quaternion produces a warning, not an error
        state.cache.insert(
            1,
            make_test_entity_with_rotation(
                1,
                "WarningOnly",
                Vec3::ZERO,
                Quat::from_xyzw(1.0, 1.0, 1.0, 1.0),
                Vec3::ONE,
            ),
        );

        // is_valid should return true if there are only warnings
        assert!(state.is_valid());
    }

    #[test]
    fn test_editor_scene_state_is_valid_with_errors() {
        let world = World::default();
        let mut state = EditorSceneState::new(world);

        state.cache.insert(
            1,
            make_test_entity(1, "Error", Vec3::ZERO, Vec3::new(-1.0, 0.0, 1.0)),
        );

        assert!(!state.is_valid());
    }
}
