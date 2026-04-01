//! System parameter types for ECS queries.
//!
//! ## Performance Notes (Week 10)
//!
//! ### Current Performance (Post SparseSet Integration)
//!
//! With the SparseSet integration (Week 10 Day 2), entity lookups are now O(1) instead
//! of O(log n), providing 12-57× speedup over the old BTreeMap approach. This has
//! resulted in:
//!
//! - **Frame time**: 2.70ms → 1.144ms (2.4× faster)
//! - **Movement system**: 1,000µs → 106µs (9.4× faster)
//! - **FPS**: 370 → 944 (2.5× improvement)
//! - **Headroom**: 93.1% vs 60 FPS budget (16.67ms)
//!
//! ### Per-Entity Overhead Pattern
//!
//! Current Query implementations use a per-entity `archetype.get::<T>(entity)` pattern:
//!
//! ```rust,ignore
//! impl Iterator for Query<'w, T> {
//!     fn next(&mut self) -> Option<(Entity, &'w T)> {
//!         let entity = archetype.entities_vec()[self.entity_idx];
//!         let component = archetype.get::<T>(entity)?;
//!         // Each get() call:
//!         // 1. SparseSet lookup: O(1) - fast!
//!         // 2. HashMap lookup: O(1) - fast!
//!         // 3. Vec indexing: O(1) - fast!
//!         // 4. Box downcast: O(1) - fast!
//!         // Total: 4 operations per entity (1,000 entities = 4,000 ops)
//!     }
//! }
//! ```
//!
//! While each operation is O(1), the repeated overhead adds up for large entity counts.
//!
//! ### Why Batch Iteration is Difficult
//!
//! Ideally, we'd batch all operations at the archetype level:
//!
//! ```rust,ignore
//! // Dream API (blocked by borrow checker):
//! for (entity, component) in archetype.iter_components_mut::<Position>() {
//!     component.x += velocity.x;  // Direct mutable access, no per-entity lookups!
//! }
//! ```
//!
//! However, this is **not feasible** with Rust's current borrow checker due to lifetime
//! constraints. The issue:
//!
//! ```rust,ignore
//! pub fn iter_components_mut<T>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
//!     let column = self.components.get_mut(&TypeId::of::<T>())?;
//!     self.entities.iter().filter_map(|(idx, &entity)| {
//!         column.get_mut(idx)  // ❌ ERROR: captured variable escapes FnMut closure
//!         //                           Returns &mut T borrowed from captured column
//!     })
//! }
//! ```
//!
//! Rust's borrow checker prevents this because:
//! 1. The closure captures `column` (a mutable reference)
//! 2. The closure tries to return `&mut T` borrowed from `column`
//! 3. Rule: **References captured in closures cannot escape the closure scope**
//! 4. This prevents dangling references but blocks the optimization
//!
//! ### Future Optimizations (Week 11-12)
//!
//! **Week 11: SystemParam DSL**
//! - Compile-time borrow splitting with zero runtime cost
//! - Eliminate Query2Mut 70% overhead (Action 32 issue)
//! - Target: Movement <50µs (2× current performance)
//!
//! **Week 12: Parallel Execution**
//! - Rayon-based parallel system execution
//! - Dependency analysis for safe concurrent iteration
//! - Target: Physics 813µs → 200-400µs (2-4× faster)
//!
//! **Week 13+: Type Registry + BlobVec Integration**
//! - Runtime type registration system
//! - Replace Vec<Box<dyn Any>> with contiguous BlobVec storage
//! - This will enable ideal batch iteration (no Box overhead, no downcast)
//! - Expected: Additional 5-10× component access speedup

use crate::{archetype::ArchetypeId, Component, Entity, World};

/// Trait for types that can be system parameters
pub trait SystemParam: Sized {
    // This will be fleshed out later. For now, it's a marker trait.
}

// Read-only single-component query
pub struct Query<'w, T: Component> {
    world: &'w World,
    archetype_ids: Vec<ArchetypeId>,
    arch_idx: usize,
    entity_idx: usize,
    _m: std::marker::PhantomData<T>,
}

impl<'w, T: Component> Query<'w, T> {
    pub fn new(world: &'w World) -> Self {
        let archetype_ids = world
            .archetypes
            .archetypes_with_component(std::any::TypeId::of::<T>())
            .map(|arch| arch.id)
            .collect();
        Self {
            world,
            archetype_ids,
            arch_idx: 0,
            entity_idx: 0,
            _m: Default::default(),
        }
    }
}

#[allow(clippy::expect_used)] // INVARIANT: archetype_ids sourced from World; entities guaranteed to have queried components
impl<'w, T: Component> Iterator for Query<'w, T> {
    type Item = (Entity, &'w T);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.arch_idx >= self.archetype_ids.len() {
                return None;
            }
            let archetype_id = self.archetype_ids[self.arch_idx];
            let archetype = self
                .world
                .archetypes
                .get_archetype(archetype_id)
                .expect("BUG: archetype should exist from archetype_ids");

            if self.entity_idx >= archetype.len() {
                self.arch_idx += 1;
                self.entity_idx = 0;
                continue;
            }

            let entity = archetype.entities_vec()[self.entity_idx];
            self.entity_idx += 1;

            // The borrow checker needs help here. Since we are iterating over disjoint archetypes
            // and entities, this is safe. We'll use unsafe to extend the lifetime.
            let component = archetype
                .get::<T>(entity)
                .expect("BUG: entity should have component T in archetype");
            let component_ptr = component as *const T;
            // SAFETY: We convert to raw pointer then back to extend the lifetime to 'w.
            // This is safe because the World reference outlives the iterator ('w lifetime),
            // and we only read from disjoint (entity, archetype) pairs.
            return Some((entity, unsafe { &*component_ptr }));
        }
    }
}

// Read-only two-component query
pub struct Query2<'w, A: Component, B: Component> {
    world: &'w World,
    archetype_ids: Vec<ArchetypeId>,
    arch_idx: usize,
    entity_idx: usize,
    _m: std::marker::PhantomData<(A, B)>,
}

impl<'w, A: Component, B: Component> Query2<'w, A, B> {
    pub fn new(world: &'w World) -> Self {
        let archetype_ids = world
            .archetypes
            .archetypes_with_component(std::any::TypeId::of::<A>())
            .filter(|arch| arch.signature.contains(std::any::TypeId::of::<B>()))
            .map(|arch| arch.id)
            .collect();

        Self {
            world,
            archetype_ids,
            arch_idx: 0,
            entity_idx: 0,
            _m: Default::default(),
        }
    }
}

#[allow(clippy::expect_used)] // INVARIANT: archetype_ids sourced from World; entities guaranteed to have queried components
impl<'w, A: Component, B: Component> Iterator for Query2<'w, A, B> {
    type Item = (Entity, &'w A, &'w B);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.arch_idx >= self.archetype_ids.len() {
                return None;
            }
            let archetype_id = self.archetype_ids[self.arch_idx];
            let archetype = self
                .world
                .archetypes
                .get_archetype(archetype_id)
                .expect("BUG: archetype should exist from archetype_ids");

            if self.entity_idx >= archetype.len() {
                self.arch_idx += 1;
                self.entity_idx = 0;
                continue;
            }

            let entity = archetype.entities_vec()[self.entity_idx];
            self.entity_idx += 1;

            // Unsafe is used to satisfy the borrow checker. This is safe because
            // we are only reading, and the iterator structure ensures we don't hold
            // references that outlive the world.
            let component_a = archetype
                .get::<A>(entity)
                .expect("BUG: entity should have component A in archetype");
            let component_b = archetype
                .get::<B>(entity)
                .expect("BUG: entity should have component B in archetype");
            let ptr_a = component_a as *const A;
            let ptr_b = component_b as *const B;

            // SAFETY: Raw pointers extend component lifetimes to 'w. This is safe because:
            // 1. World reference lives for 'w (borrowed in Query2::new)
            // 2. No mutation occurs (both are shared references)
            // 3. Each (entity, archetype) pair is visited exactly once
            return Some((entity, unsafe { &*ptr_a }, unsafe { &*ptr_b }));
        }
    }
}

// Mutable two-component query (for Action 32 writeback optimization)
pub struct Query2Mut<'w, A: Component, B: Component> {
    world: *mut World,
    archetype_ids: Vec<ArchetypeId>,
    arch_idx: usize,
    entity_idx: usize,
    _m: std::marker::PhantomData<(&'w mut A, &'w B)>,
}

impl<'w, A: Component, B: Component> Query2Mut<'w, A, B> {
    pub fn new(world: &'w mut World) -> Self {
        let archetype_ids = world
            .archetypes
            .archetypes_with_component(std::any::TypeId::of::<A>())
            .filter(|arch| arch.signature.contains(std::any::TypeId::of::<B>()))
            .map(|arch| arch.id)
            .collect();

        Self {
            world,
            archetype_ids,
            arch_idx: 0,
            entity_idx: 0,
            _m: Default::default(),
        }
    }
}

#[allow(clippy::expect_used)] // INVARIANT: archetype_ids sourced from World; entities guaranteed to have queried components
impl<'w, A: Component, B: Component> Iterator for Query2Mut<'w, A, B> {
    type Item = (Entity, &'w mut A, &'w B);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.arch_idx >= self.archetype_ids.len() {
                return None;
            }
            let archetype_id = self.archetype_ids[self.arch_idx];

            // SAFETY: We hold *mut World for 'w lifetime. We reconstruct a single mutable
            // reference per iteration step. This is safe because:
            // 1. The world pointer is valid for 'w (borrowed in Query2Mut::new)
            // 2. We only access one entity at a time (sequential iteration)
            // 3. A and B are different types (separate columns, no aliasing)
            // 4. We take raw pointers to the components BEFORE releasing the borrow,
            //    and the returned references are valid for 'w since the World lives that long.
            let world_ref = unsafe { &mut *self.world };

            let archetype = world_ref
                .archetypes
                .get_archetype_mut(archetype_id)
                .expect("BUG: archetype should exist from archetype_ids");

            if self.entity_idx >= archetype.len() {
                self.arch_idx += 1;
                self.entity_idx = 0;
                continue;
            }

            let entity = archetype.entities_vec()[self.entity_idx];
            self.entity_idx += 1;

            // Get both components from the same archetype reference.
            // A and B are different types so their columns are disjoint memory.
            let component_a = archetype
                .get_mut::<A>(entity)
                .expect("BUG: entity should have component A in archetype");
            let ptr_a = component_a as *mut A;

            let component_b = archetype
                .get::<B>(entity)
                .expect("BUG: entity should have component B in archetype");
            let ptr_b = component_b as *const B;

            // SAFETY: ptr_a and ptr_b point to different component columns (A != B),
            // and the underlying storage lives for 'w. Sequential iteration ensures
            // we never hand out overlapping references to the same entity.
            return Some((entity, unsafe { &mut *ptr_a }, unsafe { &*ptr_b }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Health {
        current: i32,
        max: i32,
    }

    // ====================
    // Day 1: Query Tests (Single Component)
    // ====================

    #[test]
    fn test_query_single_component_empty() {
        let world = World::new();
        let query = Query::<Position>::new(&world);
        let results: Vec<_> = query.collect();
        assert_eq!(results.len(), 0, "Empty world should return no results");
    }

    #[test]
    fn test_query_single_component_one_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });

        let query = Query::<Position>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(results.len(), 1, "Should find one entity with Position");
        assert_eq!(results[0].0, entity);
        assert_eq!(results[0].1.x, 1.0);
        assert_eq!(results[0].1.y, 2.0);
    }

    #[test]
    fn test_query_single_component_multiple_entities() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        world.insert(e1, Position { x: 1.0, y: 1.0 });
        world.insert(e2, Position { x: 2.0, y: 2.0 });
        world.insert(e3, Position { x: 3.0, y: 3.0 });

        let query = Query::<Position>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(results.len(), 3, "Should find all three entities");

        // Verify all entities present (order may vary due to archetype iteration)
        let entities: Vec<Entity> = results.iter().map(|(e, _)| *e).collect();
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
        assert!(entities.contains(&e3));
    }

    #[test]
    fn test_query_filters_entities_without_component() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        world.insert(e1, Position { x: 1.0, y: 1.0 });
        world.insert(e2, Velocity { x: 5.0, y: 5.0 }); // No Position!
        world.insert(e3, Position { x: 3.0, y: 3.0 });

        let query = Query::<Position>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(results.len(), 2, "Should only find entities with Position");

        let entities: Vec<Entity> = results.iter().map(|(e, _)| *e).collect();
        assert!(entities.contains(&e1));
        assert!(!entities.contains(&e2), "e2 should not be in results");
        assert!(entities.contains(&e3));
    }

    #[test]
    fn test_query_multiple_archetypes() {
        let mut world = World::new();

        // Archetype 1: Position only
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        // Archetype 2: Position + Velocity
        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });
        world.insert(e2, Velocity { x: 1.0, y: 1.0 });

        // Archetype 3: Position + Health
        let e3 = world.spawn();
        world.insert(e3, Position { x: 3.0, y: 3.0 });
        world.insert(
            e3,
            Health {
                current: 100,
                max: 100,
            },
        );

        let query = Query::<Position>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(
            results.len(),
            3,
            "Should find entities across all archetypes with Position"
        );
    }

    // ====================
    // Day 1: Query2 Tests (Two Components)
    // ====================

    #[test]
    fn test_query2_empty_world() {
        let world = World::new();
        let query = Query2::<Position, Velocity>::new(&world);
        let results: Vec<_> = query.collect();
        assert_eq!(results.len(), 0, "Empty world should return no results");
    }

    #[test]
    fn test_query2_one_matching_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });
        world.insert(entity, Velocity { x: 0.5, y: 0.5 });

        let query = Query2::<Position, Velocity>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, entity);
        assert_eq!(results[0].1.x, 1.0);
        assert_eq!(results[0].2.x, 0.5);
    }

    #[test]
    fn test_query2_filters_partial_matches() {
        let mut world = World::new();

        // Entity with both components
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });
        world.insert(e1, Velocity { x: 0.5, y: 0.5 });

        // Entity with Position only
        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });

        // Entity with Velocity only
        let e3 = world.spawn();
        world.insert(e3, Velocity { x: 1.0, y: 1.0 });

        let query = Query2::<Position, Velocity>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(
            results.len(),
            1,
            "Should only find entity with both components"
        );
        assert_eq!(results[0].0, e1);
    }

    #[test]
    fn test_query2_multiple_matching_entities() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });
        world.insert(e1, Velocity { x: 0.1, y: 0.1 });

        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });
        world.insert(e2, Velocity { x: 0.2, y: 0.2 });

        let e3 = world.spawn();
        world.insert(e3, Position { x: 3.0, y: 3.0 });
        world.insert(e3, Velocity { x: 0.3, y: 0.3 });

        let query = Query2::<Position, Velocity>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_query2_across_archetypes() {
        let mut world = World::new();

        // Archetype 1: Position + Velocity
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });
        world.insert(e1, Velocity { x: 0.5, y: 0.5 });

        // Archetype 2: Position + Velocity + Health
        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });
        world.insert(e2, Velocity { x: 1.0, y: 1.0 });
        world.insert(
            e2,
            Health {
                current: 100,
                max: 100,
            },
        );

        let query = Query2::<Position, Velocity>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(results.len(), 2, "Should find entities across archetypes");
    }

    // ====================
    // Day 1: Query2Mut Tests (Mutable Queries)
    // ====================

    #[test]
    fn test_query2mut_empty_world() {
        let mut world = World::new();
        let query = Query2Mut::<Position, Velocity>::new(&mut world);
        let results: Vec<_> = query.collect();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_query2mut_mutation() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });
        world.insert(entity, Velocity { x: 0.5, y: 0.5 });

        {
            let query = Query2Mut::<Position, Velocity>::new(&mut world);
            for (_e, pos, vel) in query {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        // Verify mutation
        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.5);
        assert_eq!(pos.y, 2.5);
    }

    #[test]
    fn test_query2mut_multiple_entities() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, Position { x: 0.0, y: 0.0 });
        world.insert(e1, Velocity { x: 1.0, y: 1.0 });

        let e2 = world.spawn();
        world.insert(e2, Position { x: 5.0, y: 5.0 });
        world.insert(e2, Velocity { x: 2.0, y: 2.0 });

        {
            let query = Query2Mut::<Position, Velocity>::new(&mut world);
            for (_e, pos, vel) in query {
                pos.x += vel.x * 10.0;
                pos.y += vel.y * 10.0;
            }
        }

        let pos1 = world.get::<Position>(e1).unwrap();
        assert_eq!(pos1.x, 10.0);
        assert_eq!(pos1.y, 10.0);

        let pos2 = world.get::<Position>(e2).unwrap();
        assert_eq!(pos2.x, 25.0);
        assert_eq!(pos2.y, 25.0);
    }

    #[test]
    fn test_query2mut_filters_correctly() {
        let mut world = World::new();

        // Entity with both components
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });
        world.insert(e1, Velocity { x: 1.0, y: 1.0 });

        // Entity with Position only (should not be mutated)
        let e2 = world.spawn();
        world.insert(e2, Position { x: 2.0, y: 2.0 });

        {
            let query = Query2Mut::<Position, Velocity>::new(&mut world);
            for (_e, pos, vel) in query {
                pos.x += vel.x;
            }
        }

        let pos1 = world.get::<Position>(e1).unwrap();
        assert_eq!(pos1.x, 2.0, "e1 should be mutated");

        let pos2 = world.get::<Position>(e2).unwrap();
        assert_eq!(pos2.x, 2.0, "e2 should NOT be mutated");
    }

    // ====================
    // Day 1: Query Component Access Patterns
    // ====================

    #[test]
    fn test_query_read_only_access() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });

        let query = Query::<Position>::new(&world);

        // Verify we can read data
        let results: Vec<_> = query.collect();
        assert_eq!(results[0].1.x, 1.0);

        // Original data unchanged
        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.0);
    }

    #[test]
    fn test_query2_read_only_both_components() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });
        world.insert(entity, Velocity { x: 0.5, y: 0.5 });

        let query = Query2::<Position, Velocity>::new(&world);

        for (_e, pos, vel) in query {
            // Can read both
            let _ = pos.x + vel.x;
        }

        // Data unchanged
        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.0);
    }

    #[test]
    fn test_query2mut_mutable_first_immutable_second() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });
        world.insert(entity, Velocity { x: 0.5, y: 0.5 });

        {
            let query = Query2Mut::<Position, Velocity>::new(&mut world);
            for (_e, pos, vel) in query {
                // Can mutate first, read second
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.5);

        // Velocity unchanged (immutable)
        let vel = world.get::<Velocity>(entity).unwrap();
        assert_eq!(vel.x, 0.5);
    }

    // ====================
    // Day 1: Query Iterator Behavior
    // ====================

    #[test]
    fn test_query_iterator_exhaustion() {
        let mut world = World::new();
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        let mut query = Query::<Position>::new(&world);

        // First iteration
        assert!(query.next().is_some());

        // Iterator exhausted
        assert!(query.next().is_none());
        assert!(query.next().is_none());
    }

    #[test]
    fn test_query2_iterator_count() {
        let mut world = World::new();

        for i in 0..10 {
            let e = world.spawn();
            world.insert(
                e,
                Position {
                    x: i as f32,
                    y: i as f32,
                },
            );
            world.insert(e, Velocity { x: 1.0, y: 1.0 });
        }

        let query = Query2::<Position, Velocity>::new(&world);
        let count = query.count();

        assert_eq!(count, 10);
    }

    #[test]
    fn test_query_collect_into_vec() {
        let mut world = World::new();

        let e1 = world.spawn();
        let e2 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });
        world.insert(e2, Position { x: 2.0, y: 2.0 });

        let query = Query::<Position>::new(&world);
        let results: Vec<_> = query.collect();

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|(e, _)| *e == e1));
        assert!(results.iter().any(|(e, _)| *e == e2));
    }
}
