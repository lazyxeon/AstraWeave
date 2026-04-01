// SPDX-License-Identifier: MIT
//! Sparse set data structure for O(1) entity lookup
//!
//! Based on the classic sparse set pattern used by EnTT, Flecs, and Bevy.
//! Provides O(1) insert, get, and remove operations with packed iteration.

use crate::Entity;

/// Sparse set mapping Entity → dense index
///
/// This data structure provides:
/// - O(1) insert, get, remove
/// - Packed dense array for cache-friendly iteration
/// - Sparse array for fast entity → index lookup
///
/// Memory layout:
/// ```text
/// sparse: [None, Some(0), None, Some(1), None, Some(2), ...]
///              ↓              ↓              ↓
/// dense:  [Entity(1), Entity(3), Entity(5), ...]
/// ```
pub struct SparseSet {
    /// Sparse array: Entity ID → dense index
    /// Only allocated entries contain Some(index)
    sparse: Vec<Option<usize>>,

    /// Dense array: Packed list of entities
    /// This is what we iterate over for cache-friendly access
    dense: Vec<Entity>,
}

impl SparseSet {
    /// Create a new empty SparseSet
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
        }
    }

    /// Create a new SparseSet with capacity for `capacity` entities
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sparse: Vec::with_capacity(capacity),
            dense: Vec::with_capacity(capacity),
        }
    }

    /// Insert an entity into the set
    ///
    /// Returns the dense index where the entity was inserted.
    /// If the entity already exists, returns its existing index.
    pub fn insert(&mut self, entity: Entity) -> usize {
        let id = entity.id() as usize;

        // Check if entity already exists
        if let Some(&index) = self.sparse.get(id).and_then(|opt| opt.as_ref()) {
            return index;
        }

        // Expand sparse array if needed
        if id >= self.sparse.len() {
            self.sparse.resize(id + 1, None);
        }

        // Add to dense array
        let dense_index = self.dense.len();
        self.dense.push(entity);
        self.sparse[id] = Some(dense_index);

        dense_index
    }

    /// Get the dense index for an entity
    ///
    /// Returns None if the entity is not in the set.
    pub fn get(&self, entity: Entity) -> Option<usize> {
        let id = entity.id() as usize;
        self.sparse.get(id).and_then(|opt| *opt)
    }

    /// Check if the set contains an entity
    pub fn contains(&self, entity: Entity) -> bool {
        self.get(entity).is_some()
    }

    /// Remove an entity from the set
    ///
    /// Returns the dense index where the entity was located, or None if not found.
    /// Uses swap_remove for O(1) performance (order not preserved).
    pub fn remove(&mut self, entity: Entity) -> Option<usize> {
        let id = entity.id() as usize;

        let dense_index = self.sparse.get_mut(id)?.take()?;

        // Swap with last element for O(1) removal
        let last_index = self.dense.len() - 1;

        if dense_index != last_index {
            // Update the swapped entity's sparse index
            let swapped_entity = self.dense[last_index];
            self.dense.swap(dense_index, last_index);
            self.sparse[swapped_entity.id() as usize] = Some(dense_index);
        }

        self.dense.pop();

        Some(dense_index)
    }

    /// Get the number of entities in the set
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    /// Get the packed dense array of entities
    ///
    /// This is the array you should iterate over for cache-friendly access.
    pub fn entities(&self) -> &[Entity] {
        &self.dense
    }

    /// Clear all entities from the set
    pub fn clear(&mut self) {
        self.dense.clear();
        self.sparse.clear();
    }

    /// Get the capacity of the dense array
    pub fn capacity(&self) -> usize {
        self.dense.capacity()
    }

    /// Reserve space for at least `additional` more entities
    pub fn reserve(&mut self, additional: usize) {
        self.dense.reserve(additional);
    }
}

impl Default for SparseSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic sparse set for storing arbitrary data
///
/// This extends SparseSet to store data alongside entities.
pub struct SparseSetData<T> {
    /// Sparse array: Entity ID → dense index
    sparse: Vec<Option<usize>>,

    /// Dense array: Packed entities
    entities: Vec<Entity>,

    /// Dense array: Packed data
    data: Vec<T>,
}

impl<T> SparseSetData<T> {
    /// Create a new empty SparseSetData
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            entities: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Create a new SparseSetData with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sparse: Vec::with_capacity(capacity),
            entities: Vec::with_capacity(capacity),
            data: Vec::with_capacity(capacity),
        }
    }

    /// Insert an entity with associated data
    ///
    /// If the entity already exists, its data is replaced.
    /// Returns the old data if it existed.
    pub fn insert(&mut self, entity: Entity, value: T) -> Option<T> {
        let id = entity.id() as usize;

        // Check if entity already exists
        if let Some(&index) = self.sparse.get(id).and_then(|opt| opt.as_ref()) {
            return Some(std::mem::replace(&mut self.data[index], value));
        }

        // Expand sparse array if needed
        if id >= self.sparse.len() {
            self.sparse.resize(id + 1, None);
        }

        // Add to dense arrays
        let dense_index = self.entities.len();
        self.entities.push(entity);
        self.data.push(value);
        self.sparse[id] = Some(dense_index);

        None
    }

    /// Get a reference to the data for an entity
    pub fn get(&self, entity: Entity) -> Option<&T> {
        let id = entity.id() as usize;
        let index = *self.sparse.get(id)?.as_ref()?;
        self.data.get(index)
    }

    /// Get a mutable reference to the data for an entity
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let id = entity.id() as usize;
        let index = *self.sparse.get(id)?.as_ref()?;
        self.data.get_mut(index)
    }

    /// Check if the set contains an entity
    pub fn contains(&self, entity: Entity) -> bool {
        let id = entity.id() as usize;
        self.sparse.get(id).and_then(|opt| *opt).is_some()
    }

    /// Remove an entity and return its data
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let id = entity.id() as usize;

        let dense_index = self.sparse.get_mut(id)?.take()?;

        // Swap with last element for O(1) removal
        let last_index = self.entities.len() - 1;

        if dense_index != last_index {
            // Update the swapped entity's sparse index
            let swapped_entity = self.entities[last_index];
            self.entities.swap(dense_index, last_index);
            self.data.swap(dense_index, last_index);
            self.sparse[swapped_entity.id() as usize] = Some(dense_index);
        }

        self.entities.pop();
        self.data.pop()
    }

    /// Get the number of entities
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get the packed entities array
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Get the dense array capacity (pre-allocated slots)
    pub fn capacity(&self) -> usize {
        self.entities.capacity()
    }

    /// Get the packed data array
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Get mutable packed data array
    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Iterate over (entity, data) pairs
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.entities.iter().copied().zip(self.data.iter())
    }

    /// Iterate mutably over (entity, data) pairs
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.entities.iter().copied().zip(self.data.iter_mut())
    }

    /// Clear all entities and data
    pub fn clear(&mut self) {
        self.entities.clear();
        self.data.clear();
        self.sparse.clear();
    }
}

impl<T> Default for SparseSetData<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_set_insert() {
        let mut set = SparseSet::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };
        let e3 = unsafe { Entity::from_raw(3) };

        let idx1 = set.insert(e1);
        let idx2 = set.insert(e2);
        let idx3 = set.insert(e3);

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 2);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_sparse_set_get() {
        let mut set = SparseSet::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1);
        set.insert(e2);

        assert_eq!(set.get(e1), Some(0));
        assert_eq!(set.get(e2), Some(1));
        assert_eq!(set.get(unsafe { Entity::from_raw(99) }), None);
    }

    #[test]
    fn test_sparse_set_contains() {
        let mut set = SparseSet::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1);

        assert!(set.contains(e1));
        assert!(!set.contains(e2));
    }

    #[test]
    fn test_sparse_set_remove() {
        let mut set = SparseSet::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };
        let e3 = unsafe { Entity::from_raw(15) };

        set.insert(e1);
        set.insert(e2);
        set.insert(e3);

        assert_eq!(set.len(), 3);

        // Remove middle element
        let removed_idx = set.remove(e2);
        assert_eq!(removed_idx, Some(1));
        assert_eq!(set.len(), 2);

        // e3 should have been swapped into e2's position
        assert_eq!(set.get(e3), Some(1));
        assert_eq!(set.get(e1), Some(0));
        assert_eq!(set.get(e2), None);
    }

    #[test]
    fn test_sparse_set_clear() {
        let mut set = SparseSet::new();

        set.insert(unsafe { Entity::from_raw(1) });
        set.insert(unsafe { Entity::from_raw(2) });

        assert_eq!(set.len(), 2);

        set.clear();

        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn test_sparse_set_data_insert() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1, "hello");
        set.insert(e2, "world");

        assert_eq!(set.get(e1), Some(&"hello"));
        assert_eq!(set.get(e2), Some(&"world"));
    }

    #[test]
    fn test_sparse_set_data_replace() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };

        let old = set.insert(e1, 42);
        assert_eq!(old, None);

        let old = set.insert(e1, 100);
        assert_eq!(old, Some(42));

        assert_eq!(set.get(e1), Some(&100));
    }

    #[test]
    fn test_sparse_set_data_remove() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };
        let e3 = unsafe { Entity::from_raw(15) };

        set.insert(e1, 1);
        set.insert(e2, 2);
        set.insert(e3, 3);

        let removed = set.remove(e2);
        assert_eq!(removed, Some(2));

        assert_eq!(set.get(e1), Some(&1));
        assert_eq!(set.get(e2), None);
        assert_eq!(set.get(e3), Some(&3));
    }

    #[test]
    fn test_sparse_set_data_iter() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };
        let e3 = unsafe { Entity::from_raw(15) };

        set.insert(e1, 100);
        set.insert(e2, 200);
        set.insert(e3, 300);

        let sum: i32 = set.iter().map(|(_, &value)| value).sum();
        assert_eq!(sum, 600);
    }

    #[test]
    fn test_sparse_set_data_iter_mut() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1, 10);
        set.insert(e2, 20);

        for (_, value) in set.iter_mut() {
            *value *= 2;
        }

        assert_eq!(set.get(e1), Some(&20));
        assert_eq!(set.get(e2), Some(&40));
    }

    // ====================
    // Day 3: Surgical Coverage Improvements
    // ====================

    #[test]
    fn test_sparse_set_with_capacity() {
        let set = SparseSet::with_capacity(100);
        assert_eq!(set.len(), 0);
        assert!(set.capacity() >= 100);
    }

    #[test]
    fn test_sparse_set_capacity_and_reserve() {
        let mut set = SparseSet::new();
        let initial_cap = set.capacity();

        set.reserve(50);
        assert!(set.capacity() >= initial_cap + 50);
    }

    #[test]
    fn test_sparse_set_insert_existing_entity() {
        let mut set = SparseSet::new();
        let e1 = unsafe { Entity::from_raw(5) };

        let idx1 = set.insert(e1);
        let idx2 = set.insert(e1); // Idempotent insert

        assert_eq!(idx1, idx2);
        assert_eq!(set.len(), 1); // Should not duplicate
    }

    #[test]
    fn test_sparse_set_remove_nonexistent() {
        let mut set = SparseSet::new();
        let e1 = unsafe { Entity::from_raw(5) };

        let removed = set.remove(e1);
        assert_eq!(removed, None);
    }

    #[test]
    fn test_sparse_set_large_entity_ids() {
        let mut set = SparseSet::new();

        // Large entity IDs force sparse array expansion
        let e1 = unsafe { Entity::from_raw(1000) };
        let e2 = unsafe { Entity::from_raw(5000) };

        set.insert(e1);
        set.insert(e2);

        assert_eq!(set.len(), 2);
        assert!(set.contains(e1));
        assert!(set.contains(e2));
    }

    #[test]
    fn test_sparse_set_remove_last_element() {
        let mut set = SparseSet::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1);
        set.insert(e2);

        // Remove last element (no swap needed)
        let removed = set.remove(e2);
        assert_eq!(removed, Some(1));
        assert_eq!(set.len(), 1);
        assert_eq!(set.get(e1), Some(0));
    }

    #[test]
    fn test_sparse_set_data_with_capacity() {
        let set = SparseSetData::<i32>::with_capacity(100);
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn test_sparse_set_data_get_mut() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };
        set.insert(e1, 42);

        if let Some(value) = set.get_mut(e1) {
            *value += 10;
        }

        assert_eq!(set.get(e1), Some(&52));
    }

    #[test]
    fn test_sparse_set_data_get_mut_nonexistent() {
        let mut set = SparseSetData::<i32>::new();
        let e1 = unsafe { Entity::from_raw(5) };

        assert!(set.get_mut(e1).is_none());
    }

    #[test]
    fn test_sparse_set_data_contains() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1, 42);

        assert!(set.contains(e1));
        assert!(!set.contains(e2));
    }

    #[test]
    fn test_sparse_set_data_clear() {
        let mut set = SparseSetData::new();

        set.insert(unsafe { Entity::from_raw(1) }, 10);
        set.insert(unsafe { Entity::from_raw(2) }, 20);

        assert_eq!(set.len(), 2);

        set.clear();

        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn test_sparse_set_data_arrays() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1, 100);
        set.insert(e2, 200);

        assert_eq!(set.entities().len(), 2);
        assert_eq!(set.data().len(), 2);

        // Mutate via data_mut()
        set.data_mut()[0] += 50;
        assert_eq!(set.get(e1), Some(&150));
    }

    #[test]
    fn test_sparse_set_data_remove_last() {
        let mut set = SparseSetData::new();

        let e1 = unsafe { Entity::from_raw(5) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1, 1);
        set.insert(e2, 2);

        // Remove last element (no swap needed)
        let removed = set.remove(e2);
        assert_eq!(removed, Some(2));
        assert_eq!(set.len(), 1);
    }

    // ===================================================================
    // Shard 3/6 Remediation: is_empty()→true, entities()→empty
    // ===================================================================

    #[test]
    fn test_sparse_set_is_empty_false_after_insert() {
        // Kills: SparseSet::is_empty() → true
        let mut set = SparseSet::new();
        let e1 = unsafe { Entity::from_raw(1) };

        set.insert(e1);
        assert!(
            !set.is_empty(),
            "is_empty() must return false when entities are present"
        );
    }

    #[test]
    fn test_sparse_set_entities_returns_inserted() {
        // Kills: SparseSet::entities() → Vec::leak(Vec::new()) (empty slice)
        let mut set = SparseSet::new();
        let e1 = unsafe { Entity::from_raw(3) };
        let e2 = unsafe { Entity::from_raw(7) };

        set.insert(e1);
        set.insert(e2);

        let ents = set.entities();
        assert_eq!(
            ents.len(),
            2,
            "entities() must return all inserted entities"
        );
        assert!(ents.contains(&e1), "entities() must contain e1");
        assert!(ents.contains(&e2), "entities() must contain e2");
    }

    #[test]
    fn test_sparse_set_data_is_empty_false_after_insert() {
        // Also kill: SparseSetData::is_empty() → true (if mutated)
        let mut set = SparseSetData::new();
        let e1 = unsafe { Entity::from_raw(5) };

        set.insert(e1, 42);
        assert!(
            !set.is_empty(),
            "SparseSetData::is_empty() must return false when populated"
        );
    }

    #[test]
    fn test_sparse_set_data_entities_returns_inserted() {
        // Also kill: SparseSetData::entities() → empty slice (if mutated)
        let mut set = SparseSetData::new();
        let e1 = unsafe { Entity::from_raw(2) };
        let e2 = unsafe { Entity::from_raw(9) };

        set.insert(e1, 100);
        set.insert(e2, 200);

        let ents = set.entities();
        assert_eq!(
            ents.len(),
            2,
            "entities() must return all inserted entities"
        );
        assert!(ents.contains(&e1), "entities() must contain e1");
        assert!(ents.contains(&e2), "entities() must contain e2");
    }
}
