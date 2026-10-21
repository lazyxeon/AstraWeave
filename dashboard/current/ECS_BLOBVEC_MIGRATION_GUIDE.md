# ECS Performance Fix: BlobVec Integration Implementation Guide

**Date**: January 7, 2026  
**Status**: Phase 1 Infrastructure COMPLETE, Phase 2 Archetype Migration READY  
**Priority**: P0 - Critical Production Blocker

---

## Current State

### Completed Infrastructure

1. **`component_meta.rs`** (NEW - 160 LOC)
   - `ComponentMeta` struct with layout, drop_fn, clone_fn
   - `ComponentMetaRegistry` for runtime type lookup
   - Full test coverage (7 tests)

2. **`blob_vec.rs`** (UPDATED)
   - Added `from_layout()` constructor for type-erased creation
   - Added `from_layout_with_capacity()` for pre-allocation
   - Enables BlobVec creation from ComponentMeta

### Remaining Work: Archetype Migration

The Archetype struct must be updated to use BlobVec instead of `Vec<Box<dyn Any>>`:

```rust
// CURRENT (slow):
pub struct Archetype {
    components: HashMap<TypeId, Vec<Box<dyn std::any::Any + Send + Sync>>>,
    // ...
}

// TARGET (fast):
pub struct Archetype {
    components: HashMap<TypeId, BlobVec>,
    meta_registry: ComponentMetaRegistry,  // For runtime type info
    // ...
}
```

---

## Phase 2: Archetype Migration Steps

### Step 1: Update Archetype struct

**File**: `astraweave-ecs/src/archetype.rs`

```rust
use crate::blob_vec::BlobVec;
use crate::component_meta::{ComponentMeta, ComponentMetaRegistry};

pub struct Archetype {
    pub id: ArchetypeId,
    pub signature: ArchetypeSignature,
    entities: Vec<Entity>,
    entity_index: SparseSet,
    
    // NEW: Type-erased contiguous storage (no heap allocation per component)
    components: HashMap<TypeId, BlobVec>,
    
    // NEW: Runtime type information for safe BlobVec operations
    meta: HashMap<TypeId, ComponentMeta>,
}
```

### Step 2: Update `add_entity()`

**Current** (allocates on heap):
```rust
pub fn add_entity(
    &mut self,
    entity: Entity,
    mut component_data: HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>,
) {
    // ...
    for ty in &self.signature.components {
        if let Some(data) = component_data.remove(ty) {
            let column = self.components.get_mut(ty).unwrap();
            column.push(data);  // Box allocation!
        }
    }
}
```

**Target** (direct memory):
```rust
pub fn add_entity_typed<T: Component + Clone>(&mut self, entity: Entity, component: T) {
    self.entity_index.insert(entity);
    self.entities.push(entity);
    
    let type_id = TypeId::of::<T>();
    let column = self.components.get_mut(&type_id)
        .expect("component column should exist for signature type");
    
    // SAFETY: BlobVec was created with ComponentMeta::of::<T>()
    unsafe { column.push(component) };
}
```

### Step 3: Update `get<T>()` and `get_mut<T>()`

**Current** (indirection + downcast):
```rust
pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
    let row = self.entity_index.get(entity)?;
    let column = self.components.get(&TypeId::of::<T>())?;
    let boxed = column.get(row)?;
    boxed.downcast_ref::<T>()  // Runtime type check + pointer chase
}
```

**Target** (direct access):
```rust
pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
    let row = self.entity_index.get(entity)?;
    let column = self.components.get(&TypeId::of::<T>())?;
    
    // SAFETY: We only push T to columns keyed by TypeId::of::<T>()
    unsafe { column.get::<T>(row) }
}
```

### Step 4: Update `remove_entity_components()`

This is the trickiest part - moving components between archetypes during transitions.

**Current** (clones Boxes):
```rust
pub fn remove_entity_components(&mut self, entity: Entity) 
    -> HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>> 
{
    // ... swap_remove each column, return Boxes
}
```

**Target** (raw memory copy):
```rust
/// Remove entity and move components to destination archetype.
/// 
/// This avoids creating intermediate heap allocations by copying
/// directly from source to destination BlobVecs.
pub fn move_entity_to(
    &mut self,
    entity: Entity,
    dest: &mut Archetype,
    dest_entity: Entity,
) {
    let row = self.entity_index.remove(entity).expect("entity should exist");
    
    // For each component type in source signature...
    for (type_id, src_column) in &mut self.components {
        if let Some(dest_column) = dest.components.get_mut(type_id) {
            let meta = self.meta.get(type_id).expect("meta should exist");
            
            // Clone from src to dest using type-erased clone_fn
            unsafe {
                let src_ptr = src_column.data.as_ptr()
                    .add(row * meta.layout.size());
                
                dest_column.reserve(1);
                let dst_ptr = dest_column.data.as_ptr()
                    .add(dest_column.len * meta.layout.size());
                
                (meta.clone_fn)(src_ptr, dst_ptr);
                dest_column.len += 1;
            }
        }
        
        // Remove from source (swap_remove)
        unsafe { src_column.swap_remove_untyped(row, &meta) };
    }
    
    // ... update entity indices
}
```

### Step 5: Update ArchetypeStorage

Need to pass ComponentMetaRegistry through the storage:

```rust
pub struct ArchetypeStorage {
    // ...existing fields...
    
    /// Global component metadata registry
    meta_registry: ComponentMetaRegistry,
}

impl ArchetypeStorage {
    pub fn get_or_create_archetype(&mut self, signature: ArchetypeSignature) -> ArchetypeId {
        // ... existing lookup logic ...
        
        // Create archetype with proper BlobVecs
        let mut components = HashMap::new();
        let mut meta = HashMap::new();
        
        for &type_id in &signature.components {
            let component_meta = self.meta_registry.get(type_id)
                .expect("component type should be registered");
            
            components.insert(type_id, component_meta.create_blob_vec());
            meta.insert(type_id, component_meta.clone());
        }
        
        let archetype = Archetype {
            id,
            signature,
            entities: Vec::new(),
            entity_index: SparseSet::new(),
            components,
            meta,
        };
        
        // ...
    }
}
```

### Step 6: Update World::insert()

Component type must be registered before insertion:

```rust
pub fn insert<T: Component + Clone>(&mut self, e: Entity, c: T) {
    if !self.is_alive(e) {
        return;
    }
    
    // Auto-register component type (idempotent)
    self.archetypes.meta_registry.register::<T>();
    
    // ... rest of insert logic using typed path
}
```

---

## Expected Performance Improvements

| Operation | Current | After Fix | Improvement |
|-----------|---------|-----------|-------------|
| Component insert | ~30 ns | ~5 ns | **6×** faster |
| Component get | ~15 ns | ~2 ns | **7.5×** faster |
| Component remove | ~25 ns | ~3 ns | **8×** faster |
| Archetype transition | ~100 ns | ~15 ns | **6.7×** faster |

**Total benchmark improvement**:
- component_add/single/100: 41.8 µs → ~7 µs (**6×** faster)
- component_remove/single/100: 39.8 µs → ~5 µs (**8×** faster)
- spawn/with_position/1000: 809 µs → ~135 µs (**6×** faster)

---

## Breaking Changes

### API Changes

1. **Component trait bound**: Components must now implement `Clone`
   - Most game components already do (Position, Velocity, Health, etc.)
   - If not clonable, can use `Arc<T>` wrapper

2. **Type registration**: Components are auto-registered on first use
   - No user action required
   - Registration is idempotent (safe to call multiple times)

### Migration Path

1. Add `#[derive(Clone)]` to any component structs missing it
2. No other changes needed - API remains compatible

---

## Testing Plan

1. **Unit tests**: component_meta.rs (7 tests) ✅
2. **Unit tests**: blob_vec.rs (existing) ✅
3. **Integration tests**: archetype.rs (update existing)
4. **Property tests**: property_tests.rs (validate determinism)
5. **Benchmark validation**: ecs_benchmarks.rs (must show improvement)

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Unsafe code bugs | Medium | High | Miri, fuzzing, property tests |
| Clone requirement breaking | Low | Medium | Most types already Clone |
| Performance worse than expected | Low | Low | Benchmark-driven development |
| Memory leaks | Low | High | Drop function testing, ASAN |

---

## Timeline

| Phase | Task | Estimate | Status |
|-------|------|----------|--------|
| 1 | Infrastructure (component_meta.rs, blob_vec.rs) | 2h | ✅ COMPLETE |
| 2 | Archetype migration | 4-6h | 🔄 READY |
| 3 | World/ArchetypeStorage updates | 2-3h | ⏳ PENDING |
| 4 | Testing & validation | 2-3h | ⏳ PENDING |
| 5 | Benchmark verification | 1h | ⏳ PENDING |

**Total remaining**: 9-13 hours

---

## Files to Modify

1. `astraweave-ecs/src/archetype.rs` - Main storage migration
2. `astraweave-ecs/src/lib.rs` - World::insert/remove/move updates
3. `astraweave-ecs/src/blob_vec.rs` - Add swap_remove_untyped helper
4. `astraweave-ecs/tests/ecs_core_tests.rs` - Update tests
5. `astraweave-ecs/benches/ecs_benchmarks.rs` - Verify improvements

---

## Success Criteria

1. ✅ All existing tests pass
2. ✅ Benchmark results within ±10% of October 2025 baseline
3. ✅ No new unsafe code without SAFETY comments
4. ✅ No memory leaks (verified with Miri/ASAN)
5. ✅ Grade restored from B- to A+ in MASTER_BENCHMARK_REPORT.md

---

**Document Version**: 1.0  
**Author**: GitHub Copilot NASA-Grade Audit
