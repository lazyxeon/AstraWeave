//! Kani formal verification proofs for EntityAllocator
//!
//! These proofs verify critical properties of the generational entity system:
//! - Entity IDs are unique while alive
//! - Generational indices prevent use-after-free
//! - Entity::from_raw/to_raw roundtrip is lossless
//!
//! Run with: `cargo kani --package astraweave-ecs`

use crate::entity_allocator::{Entity, EntityAllocator};

/// Verify Entity::from_raw and to_raw are inverse operations
#[kani::proof]
fn entity_roundtrip_lossless() {
    let id: u32 = kani::any();
    let generation: u32 = kani::any();

    let entity = Entity::new(id, generation);
    let raw = entity.to_raw();
    let recovered = unsafe { Entity::from_raw(raw) };

    kani::assert(recovered.id() == id, "ID must survive roundtrip");
    kani::assert(
        recovered.generation() == generation,
        "Generation must survive roundtrip",
    );
}

/// Verify to_raw encoding is correct (id in low 32 bits, gen in high 32)
#[kani::proof]
fn entity_to_raw_encoding() {
    let id: u32 = kani::any();
    let generation: u32 = kani::any();

    let entity = Entity::new(id, generation);
    let raw = entity.to_raw();

    kani::assert((raw as u32) == id, "Low 32 bits must be ID");
    kani::assert(
        ((raw >> 32) as u32) == generation,
        "High 32 bits must be generation",
    );
}

/// Verify Entity::null() has expected properties
#[kani::proof]
fn entity_null_properties() {
    let null = Entity::null();

    kani::assert(null.is_null(), "Null entity must report is_null");
    kani::assert(null.id() == u32::MAX, "Null entity ID must be u32::MAX");
    kani::assert(
        null.generation() == u32::MAX,
        "Null entity generation must be u32::MAX",
    );
}

/// Verify spawn returns entities with generation 0 for new IDs
#[kani::proof]
#[kani::unwind(5)]
fn allocator_spawn_new_generation_zero() {
    let mut allocator = EntityAllocator::new();
    let count: usize = kani::any();
    kani::assume(count <= 3); // Bound for tractability

    for i in 0..count {
        let entity = allocator.spawn();
        kani::assert(
            entity.generation() == 0,
            "New entities must have generation 0",
        );
        kani::assert(
            entity.id() == i as u32,
            "IDs must be sequential for new allocations",
        );
    }
}

/// Verify spawn IDs are unique while alive
#[kani::proof]
#[kani::unwind(4)]
fn allocator_spawn_unique_ids() {
    let mut allocator = EntityAllocator::new();

    let e1 = allocator.spawn();
    let e2 = allocator.spawn();
    let e3 = allocator.spawn();

    // All entities have different IDs (while alive)
    kani::assert(e1.id() != e2.id(), "Entity IDs must be unique");
    kani::assert(e2.id() != e3.id(), "Entity IDs must be unique");
    kani::assert(e1.id() != e3.id(), "Entity IDs must be unique");
}

/// Verify despawn returns true for alive entity, false for dead
#[kani::proof]
#[kani::unwind(3)]
fn allocator_despawn_returns_correct_bool() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.spawn();

    kani::assert(
        allocator.is_alive(entity),
        "Just spawned entity must be alive",
    );

    let first_despawn = allocator.despawn(entity);
    kani::assert(first_despawn, "First despawn must return true");

    let second_despawn = allocator.despawn(entity);
    kani::assert(!second_despawn, "Second despawn must return false");
}

/// Verify is_alive returns false for despawned entities
#[kani::proof]
#[kani::unwind(3)]
fn allocator_is_alive_false_after_despawn() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.spawn();

    kani::assert(
        allocator.is_alive(entity),
        "Entity must be alive after spawn",
    );

    allocator.despawn(entity);

    kani::assert(
        !allocator.is_alive(entity),
        "Entity must be dead after despawn",
    );
}

/// Verify generation increments on despawn
#[kani::proof]
#[kani::unwind(3)]
fn allocator_generation_increments_on_despawn() {
    let mut allocator = EntityAllocator::new();
    let entity = allocator.spawn();
    let old_gen = entity.generation();

    allocator.despawn(entity);

    // Spawn again to reuse the ID
    let new_entity = allocator.spawn();

    kani::assert(
        new_entity.id() == entity.id(),
        "ID should be reused from free list",
    );
    kani::assert(
        new_entity.generation() == old_gen.wrapping_add(1),
        "Generation must increment on reuse",
    );
}

/// Verify recycled entities get incremented generation
#[kani::proof]
#[kani::unwind(5)]
fn allocator_recycled_entity_generation() {
    let mut allocator = EntityAllocator::new();

    // Spawn and despawn
    let e1 = allocator.spawn();
    allocator.despawn(e1);

    // Spawn again - should reuse e1's ID
    let e2 = allocator.spawn();

    kani::assert(e2.id() == e1.id(), "ID should be reused");
    kani::assert(
        e2.generation() == e1.generation() + 1,
        "Generation must be incremented",
    );

    // Original entity is now invalid
    kani::assert(!allocator.is_alive(e1), "Original entity must be dead");
    kani::assert(allocator.is_alive(e2), "New entity must be alive");
}

/// Verify is_alive rejects invalid entity ID
#[kani::proof]
fn allocator_is_alive_rejects_invalid_id() {
    let allocator = EntityAllocator::new();

    // Entity with ID 0 doesn't exist yet
    let fake_entity = unsafe { Entity::from_raw(0) };

    kani::assert(
        !allocator.is_alive(fake_entity),
        "Non-existent entity must report dead",
    );
}

/// Verify spawned_count tracks correctly
#[kani::proof]
#[kani::unwind(5)]
fn allocator_spawned_count_accurate() {
    let mut allocator = EntityAllocator::new();
    let count: usize = kani::any();
    kani::assume(count <= 3);

    for _ in 0..count {
        allocator.spawn();
    }

    kani::assert(
        allocator.spawned_count() == count as u64,
        "spawned_count must match actual spawns",
    );
}

/// Verify despawned_count tracks correctly
#[kani::proof]
#[kani::unwind(5)]
fn allocator_despawned_count_accurate() {
    let mut allocator = EntityAllocator::new();

    let e1 = allocator.spawn();
    let e2 = allocator.spawn();

    allocator.despawn(e1);
    kani::assert(
        allocator.despawned_count() == 1,
        "despawned_count must be 1",
    );

    allocator.despawn(e2);
    kani::assert(
        allocator.despawned_count() == 2,
        "despawned_count must be 2",
    );
}

/// Verify entity ordering is consistent
#[kani::proof]
fn entity_ordering_consistent() {
    let id1: u32 = kani::any();
    let id2: u32 = kani::any();
    let gen1: u32 = kani::any();
    let gen2: u32 = kani::any();

    let e1 = Entity::new(id1, gen1);
    let e2 = Entity::new(id2, gen2);

    // Verify Ord is consistent with PartialOrd
    if e1 < e2 {
        kani::assert(!(e2 < e1), "Ordering must be antisymmetric");
        kani::assert(!(e1 == e2), "Less than implies not equal");
    }
    if e1 == e2 {
        kani::assert(e2 == e1, "Equality must be symmetric");
    }
}
