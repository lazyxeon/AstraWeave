# Phase 8.3 Week 1 Day 1: ECS World Serialization - STARTED

**Date**: October 31, 2025  
**Session Start**: 3:00 PM  
**Current Task**: Component Serialization Infrastructure  
**Status**: 🚧 IN PROGRESS

---

## Session Goals

**Primary**: Add `Serialize + Deserialize` derives to all ECS components  
**Secondary**: Implement `serialize_ecs_world()` and `deserialize_ecs_world()`  
**Stretch**: Create basic serialization tests

---

## Component Inventory (astraweave-core/src/ecs_components.rs)

**Simple Components** (need derives):
1. ✅ `CPos` - Position (IVec2)
2. ✅ `CHealth` - Health (i32)
3. ✅ `CTeam` - Team ID (u8)
4. ✅ `CAmmo` - Ammo rounds (i32)
5. ✅ `CCooldowns` - Cooldown map (BTreeMap<CooldownKey, f32>)
6. ✅ `CDesiredPos` - Desired position (IVec2)
7. ✅ `CAiAgent` - AI agent marker
8. ✅ `CLegacyId` - Legacy entity ID mapping
9. ✅ `CPersona` - Companion personality
10. ✅ `CMemory` - Memory facts + episodes

**Dependencies**:
- `IVec2` (from astraweave-core) - needs Serialize/Deserialize
- `CooldownKey` enum - already has Serialize/Deserialize! ✅
- `Entity` (from astraweave-ecs) - needs custom serialization (ID remapping)

---

## Implementation Plan

### Step 1: Add Serde to Dependencies ✅

**File**: `astraweave-core/Cargo.toml`
```toml
serde = { version = "1.0", features = ["derive"] }
```

### Step 2: Add Derives to IVec2

**File**: `astraweave-core/src/lib.rs` (or ivec2.rs)
```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IVec2 {
    pub x: i32,
    pub y: i32,
}
```

### Step 3: Add Derives to All Components

**File**: `astraweave-core/src/ecs_components.rs`
```rust
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct CPos { pub pos: IVec2 }

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct CHealth { pub hp: i32 }

// ... repeat for all components
```

**Special Cases**:
- `CCooldowns`: BTreeMap already implements Serialize/Deserialize if K, V do ✅
- `CLegacyId`: Use `#[serde(skip)]` for Entity field (will remap on load)
- `CPersona`, `CMemory`: Nested structs need derives on inner types

### Step 4: Implement World Serialization

**File**: `astraweave-persistence-ecs/src/lib.rs`

**Pattern**: Collect all entities + components into serializable structure

```rust
#[derive(Serialize, Deserialize)]
struct SerializedWorld {
    entities: Vec<EntityId>,  // Stable IDs (u64)
    components: ComponentBlob, // Map<TypeName, Map<EntityId, ComponentData>>
}

pub fn serialize_ecs_world(world: &World) -> Result<Vec<u8>> {
    let mut serialized = SerializedWorld {
        entities: Vec::new(),
        components: ComponentBlob::new(),
    };
    
    // Iterate all entities
    for entity in world.entities() {
        serialized.entities.push(entity.id());
        
        // Serialize each component type
        if let Some(pos) = world.get::<CPos>(entity) {
            serialized.components.insert("CPos", entity.id(), pos);
        }
        if let Some(health) = world.get::<CHealth>(entity) {
            serialized.components.insert("CHealth", entity.id(), health);
        }
        // ... repeat for all component types
    }
    
    // Use postcard for compact binary
    postcard::to_allocvec(&serialized).map_err(Into::into)
}
```

### Step 5: Implement World Deserialization

```rust
pub fn deserialize_ecs_world(ecs_blob: &[u8], world: &mut World) -> Result<()> {
    let serialized: SerializedWorld = postcard::from_bytes(ecs_blob)?;
    
    // Entity ID remapping (old ID → new Entity)
    let mut id_map = HashMap::new();
    
    // Spawn all entities first (get new IDs)
    for old_id in &serialized.entities {
        let new_entity = world.spawn();
        id_map.insert(*old_id, new_entity);
    }
    
    // Insert all components with remapped entities
    for (type_name, entity_components) in &serialized.components {
        match type_name.as_str() {
            "CPos" => {
                for (old_id, data) in entity_components {
                    let entity = id_map[old_id];
                    let component: CPos = deserialize_component(data)?;
                    world.insert(entity, component);
                }
            }
            "CHealth" => { /* ... */ }
            // ... repeat for all types
        }
    }
    
    Ok(())
}
```

---

## Progress Tracker

### ✅ Completed
- [x] Read existing persistence-ecs infrastructure
- [x] Inventoried all component types (10 components)
- [x] Identified dependencies (IVec2, CooldownKey, Entity)
- [x] Created implementation plan

### 🚧 In Progress
- [ ] Add serde to astraweave-core dependencies
- [ ] Add Serialize/Deserialize to IVec2
- [ ] Add Serialize/Deserialize to all components
- [ ] Implement serialize_ecs_world()
- [ ] Implement deserialize_ecs_world()

### ⏸️ Pending
- [ ] Add comprehensive tests
- [ ] Benchmark serialization (target: <5ms @ 1000 entities)
- [ ] Document entity ID remapping pattern
- [ ] Update MASTER_BENCHMARK_REPORT with new tests

---

## Questions & Decisions

**Q1: How to handle Entity ID remapping?**
**A**: Use HashMap<u64, Entity> during deserialization. Serialize stable u64 IDs, remap to new Entity instances on load.

**Q2: What serialization format?**
**A**: **postcard** (already used in persistence benchmarks, compact binary, ~3.5ns deserialize performance validated)

**Q3: How to handle CLegacyId?**
**A**: Skip serialization with `#[serde(skip)]`, rebuild from entity relationships on load.

**Q4: Component registry for dynamic types?**
**A**: Start with manual match statement, refactor to registry pattern in Week 2 if needed.

---

**Session End**: TBD  
**Next Session**: Continue with serde derives implementation
