use astraweave_ecs as ecs;
use std::collections::HashMap;

/// Public bridge resource mapping legacy `Entity` ids from `World` to ECS entity handles.
/// Also stores the reverse mapping (ecs -> legacy) for round-trip lookups.
/// Uses HashMap for O(1) lookups (no range queries needed).
#[derive(Default, Debug)]
pub struct EntityBridge {
    // legacy id -> ecs::Entity
    map: HashMap<crate::Entity, ecs::Entity>,
    // ecs::Entity -> legacy id
    rev: HashMap<ecs::Entity, crate::Entity>,
}

impl EntityBridge {
    /// Insert a pair mapping and maintain the reverse index.
    pub fn insert_pair(&mut self, legacy: crate::Entity, ecs_e: ecs::Entity) {
        // If this legacy id was previously mapped to another ecs entity, remove that reverse mapping
        if let Some(old_ecs) = self.map.get(&legacy).copied() {
            if old_ecs != ecs_e {
                self.rev.remove(&old_ecs);
            }
        }
        // If this ecs entity was previously mapped to another legacy id, remove that forward mapping
        if let Some(old_legacy) = self.rev.get(&ecs_e).copied() {
            if old_legacy != legacy {
                self.map.remove(&old_legacy);
            }
        }

        // Insert the new pair into both maps
        self.map.insert(legacy, ecs_e);
        self.rev.insert(ecs_e, legacy);
    }

    /// Alias with clearer naming: insert a mapping and maintain reverse index.
    pub fn insert(&mut self, legacy: crate::Entity, ecs_e: ecs::Entity) {
        self.insert_pair(legacy, ecs_e);
    }

    /// Remove mapping by legacy id, cleaning up reverse index.
    pub fn remove_legacy(&mut self, legacy: &crate::Entity) {
        if let Some(e) = self.map.remove(legacy) {
            self.rev.remove(&e);
        }
    }

    /// Remove mapping by ECS entity, cleaning up forward index.
    pub fn remove_ecs(&mut self, ecs_e: &ecs::Entity) {
        if let Some(legacy) = self.rev.remove(ecs_e) {
            self.map.remove(&legacy);
        }
    }

    pub fn get(&self, legacy: &crate::Entity) -> Option<ecs::Entity> {
        self.map.get(legacy).copied()
    }

    pub fn get_legacy(&self, ecs_e: &ecs::Entity) -> Option<crate::Entity> {
        self.rev.get(ecs_e).copied()
    }

    /// Get ECS entity by legacy id (named accessor)
    pub fn get_by_legacy(&self, legacy: &crate::Entity) -> Option<ecs::Entity> {
        self.get(legacy)
    }

    /// Get legacy id by ECS entity (named accessor)
    pub fn get_by_ecs(&self, ecs_e: &ecs::Entity) -> Option<crate::Entity> {
        self.get_legacy(ecs_e)
    }

    /// Remove mapping by legacy id (named)
    pub fn remove_by_legacy(&mut self, legacy: &crate::Entity) {
        self.remove_legacy(legacy);
    }

    /// Remove mapping by ECS entity (named)
    pub fn remove_by_ecs(&mut self, ecs_e: &ecs::Entity) {
        self.remove_ecs(ecs_e);
    }

    /// Return a list of ECS entities currently referenced by the bridge.
    pub fn ecs_entities(&self) -> Vec<ecs::Entity> {
        self.rev.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_bridge_default() {
        let bridge = EntityBridge::default();
        assert_eq!(bridge.map.len(), 0);
        assert_eq!(bridge.rev.len(), 0);
    }

    #[test]
    fn test_insert_pair_basic() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 42;
        let ecs_entity = unsafe { ecs::Entity::from_raw(100) };

        bridge.insert_pair(legacy_id, ecs_entity);

        assert_eq!(bridge.get(&legacy_id), Some(ecs_entity));
        assert_eq!(bridge.get_legacy(&ecs_entity), Some(legacy_id));
    }

    #[test]
    fn test_insert_alias() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 10;
        let ecs_entity = unsafe { ecs::Entity::from_raw(20) };

        bridge.insert(legacy_id, ecs_entity);

        assert_eq!(bridge.get(&legacy_id), Some(ecs_entity));
        assert_eq!(bridge.get_legacy(&ecs_entity), Some(legacy_id));
    }

    #[test]
    fn test_insert_multiple_pairs() {
        let mut bridge = EntityBridge::default();
        let legacy1: crate::Entity = 1;
        let legacy2: crate::Entity = 2;
        let legacy3: crate::Entity = 3;
        let ecs1 = unsafe { ecs::Entity::from_raw(10) };
        let ecs2 = unsafe { ecs::Entity::from_raw(20) };
        let ecs3 = unsafe { ecs::Entity::from_raw(30) };

        bridge.insert_pair(legacy1, ecs1);
        bridge.insert_pair(legacy2, ecs2);
        bridge.insert_pair(legacy3, ecs3);

        assert_eq!(bridge.get(&legacy1), Some(ecs1));
        assert_eq!(bridge.get(&legacy2), Some(ecs2));
        assert_eq!(bridge.get(&legacy3), Some(ecs3));
        assert_eq!(bridge.map.len(), 3);
        assert_eq!(bridge.rev.len(), 3);
    }

    #[test]
    fn test_insert_pair_overwrites_existing_legacy() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 5;
        let ecs_entity1 = unsafe { ecs::Entity::from_raw(100) };
        let ecs_entity2 = unsafe { ecs::Entity::from_raw(200) };

        // Insert first mapping
        bridge.insert_pair(legacy_id, ecs_entity1);
        assert_eq!(bridge.get(&legacy_id), Some(ecs_entity1));

        // Overwrite with new ECS entity
        bridge.insert_pair(legacy_id, ecs_entity2);
        assert_eq!(bridge.get(&legacy_id), Some(ecs_entity2));

        // Old ECS entity should no longer have reverse mapping
        assert_eq!(bridge.get_legacy(&ecs_entity1), None);
        assert_eq!(bridge.get_legacy(&ecs_entity2), Some(legacy_id));
    }

    #[test]
    fn test_insert_pair_overwrites_existing_ecs() {
        let mut bridge = EntityBridge::default();
        let legacy_id1: crate::Entity = 1;
        let legacy_id2: crate::Entity = 2;
        let ecs_entity = unsafe { ecs::Entity::from_raw(100) };

        // Insert first mapping
        bridge.insert_pair(legacy_id1, ecs_entity);
        assert_eq!(bridge.get_legacy(&ecs_entity), Some(legacy_id1));

        // Overwrite with new legacy id
        bridge.insert_pair(legacy_id2, ecs_entity);
        assert_eq!(bridge.get_legacy(&ecs_entity), Some(legacy_id2));

        // Old legacy id should no longer have forward mapping
        assert_eq!(bridge.get(&legacy_id1), None);
        assert_eq!(bridge.get(&legacy_id2), Some(ecs_entity));
    }

    #[test]
    fn test_remove_legacy() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 42;
        let ecs_entity = unsafe { ecs::Entity::from_raw(100) };

        bridge.insert_pair(legacy_id, ecs_entity);
        assert_eq!(bridge.get(&legacy_id), Some(ecs_entity));

        bridge.remove_legacy(&legacy_id);

        assert_eq!(bridge.get(&legacy_id), None);
        assert_eq!(bridge.get_legacy(&ecs_entity), None);
        assert_eq!(bridge.map.len(), 0);
        assert_eq!(bridge.rev.len(), 0);
    }

    #[test]
    fn test_remove_ecs() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 42;
        let ecs_entity = unsafe { ecs::Entity::from_raw(100) };

        bridge.insert_pair(legacy_id, ecs_entity);
        assert_eq!(bridge.get_legacy(&ecs_entity), Some(legacy_id));

        bridge.remove_ecs(&ecs_entity);

        assert_eq!(bridge.get(&legacy_id), None);
        assert_eq!(bridge.get_legacy(&ecs_entity), None);
        assert_eq!(bridge.map.len(), 0);
        assert_eq!(bridge.rev.len(), 0);
    }

    #[test]
    fn test_remove_legacy_nonexistent() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 99;

        // Removing nonexistent entry should not panic
        bridge.remove_legacy(&legacy_id);
        assert_eq!(bridge.map.len(), 0);
    }

    #[test]
    fn test_remove_ecs_nonexistent() {
        let mut bridge = EntityBridge::default();
        let ecs_entity = unsafe { ecs::Entity::from_raw(999) };

        // Removing nonexistent entry should not panic
        bridge.remove_ecs(&ecs_entity);
        assert_eq!(bridge.rev.len(), 0);
    }

    #[test]
    fn test_get_nonexistent() {
        let bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 123;

        assert_eq!(bridge.get(&legacy_id), None);
    }

    #[test]
    fn test_get_legacy_nonexistent() {
        let bridge = EntityBridge::default();
        let ecs_entity = unsafe { ecs::Entity::from_raw(456) };

        assert_eq!(bridge.get_legacy(&ecs_entity), None);
    }

    #[test]
    fn test_get_by_legacy() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 7;
        let ecs_entity = unsafe { ecs::Entity::from_raw(77) };

        bridge.insert(legacy_id, ecs_entity);

        assert_eq!(bridge.get_by_legacy(&legacy_id), Some(ecs_entity));
    }

    #[test]
    fn test_get_by_ecs() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 8;
        let ecs_entity = unsafe { ecs::Entity::from_raw(88) };

        bridge.insert(legacy_id, ecs_entity);

        assert_eq!(bridge.get_by_ecs(&ecs_entity), Some(legacy_id));
    }

    #[test]
    fn test_remove_by_legacy() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 50;
        let ecs_entity = unsafe { ecs::Entity::from_raw(500) };

        bridge.insert(legacy_id, ecs_entity);
        bridge.remove_by_legacy(&legacy_id);

        assert_eq!(bridge.get(&legacy_id), None);
        assert_eq!(bridge.get_legacy(&ecs_entity), None);
    }

    #[test]
    fn test_remove_by_ecs() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 60;
        let ecs_entity = unsafe { ecs::Entity::from_raw(600) };

        bridge.insert(legacy_id, ecs_entity);
        bridge.remove_by_ecs(&ecs_entity);

        assert_eq!(bridge.get(&legacy_id), None);
        assert_eq!(bridge.get_legacy(&ecs_entity), None);
    }

    #[test]
    fn test_ecs_entities_empty() {
        let bridge = EntityBridge::default();
        let entities = bridge.ecs_entities();

        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn test_ecs_entities_single() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 1;
        let ecs_entity = unsafe { ecs::Entity::from_raw(10) };

        bridge.insert(legacy_id, ecs_entity);
        let entities = bridge.ecs_entities();

        assert_eq!(entities.len(), 1);
        assert!(entities.contains(&ecs_entity));
    }

    #[test]
    fn test_ecs_entities_multiple() {
        let mut bridge = EntityBridge::default();
        let ecs1 = unsafe { ecs::Entity::from_raw(10) };
        let ecs2 = unsafe { ecs::Entity::from_raw(20) };
        let ecs3 = unsafe { ecs::Entity::from_raw(30) };

        bridge.insert(1, ecs1);
        bridge.insert(2, ecs2);
        bridge.insert(3, ecs3);

        let entities = bridge.ecs_entities();

        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&ecs1));
        assert!(entities.contains(&ecs2));
        assert!(entities.contains(&ecs3));
    }

    #[test]
    fn test_bidirectional_consistency() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 123;
        let ecs_entity = unsafe { ecs::Entity::from_raw(456) };

        bridge.insert(legacy_id, ecs_entity);

        // Forward lookup
        let found_ecs = bridge.get(&legacy_id).unwrap();
        // Reverse lookup should return original legacy id
        let found_legacy = bridge.get_legacy(&found_ecs).unwrap();

        assert_eq!(found_legacy, legacy_id);
    }

    #[test]
    fn test_complex_insert_remove_sequence() {
        let mut bridge = EntityBridge::default();

        // Insert 5 pairs
        for i in 0..5 {
            bridge.insert(i, unsafe { ecs::Entity::from_raw((i * 10) as u64) });
        }
        assert_eq!(bridge.map.len(), 5);
        assert_eq!(bridge.rev.len(), 5);

        // Remove 2 by legacy
        bridge.remove_by_legacy(&1);
        bridge.remove_by_legacy(&3);
        assert_eq!(bridge.map.len(), 3);
        assert_eq!(bridge.rev.len(), 3);

        // Remove 1 by ecs
        bridge.remove_by_ecs(&unsafe { ecs::Entity::from_raw(20) });
        assert_eq!(bridge.map.len(), 2);
        assert_eq!(bridge.rev.len(), 2);

        // Verify remaining
        assert!(bridge.get(&0).is_some());
        assert!(bridge.get(&1).is_none());
        assert!(bridge.get(&2).is_none());
        assert!(bridge.get(&3).is_none());
        assert!(bridge.get(&4).is_some());
    }

    #[test]
    fn test_insert_same_pair_twice() {
        let mut bridge = EntityBridge::default();
        let legacy_id: crate::Entity = 77;
        let ecs_entity = unsafe { ecs::Entity::from_raw(777) };

        // Insert same pair twice
        bridge.insert(legacy_id, ecs_entity);
        bridge.insert(legacy_id, ecs_entity);

        // Should still have only one mapping
        assert_eq!(bridge.map.len(), 1);
        assert_eq!(bridge.rev.len(), 1);
        assert_eq!(bridge.get(&legacy_id), Some(ecs_entity));
        assert_eq!(bridge.get_legacy(&ecs_entity), Some(legacy_id));
    }
}
