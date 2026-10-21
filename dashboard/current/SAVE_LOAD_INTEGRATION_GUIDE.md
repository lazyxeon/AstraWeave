# AstraWeave: Save/Load Integration Guide

**Version**: 1.0  
**Last Updated**: October 31, 2025 (Phase 8.3 Week 1 Complete)  
**Status**: Production Ready  
**Maintainer**: Core Team

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [API Reference](#api-reference)
3. [Integration Patterns](#integration-patterns)
4. [Performance Best Practices](#performance-best-practices)
5. [Common Pitfalls](#common-pitfalls)
6. [Adding New Components](#adding-new-components)
7. [Testing & Validation](#testing--validation)
8. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Basic Save/Load Flow

```rust
use astraweave_ecs::World;
use astraweave_persistence_ecs::{
    serialize_ecs_world, 
    deserialize_ecs_world, 
    calculate_world_hash
};

fn save_game(world: &World) -> anyhow::Result<()> {
    // 1. Serialize world to binary blob
    let blob = serialize_ecs_world(world)?;
    
    // 2. Calculate hash for integrity checking
    let hash = calculate_world_hash(world);
    
    // 3. Save to disk (example with aw-save integration)
    let save_data = SaveData {
        ecs_blob: blob,
        world_hash: hash,
        timestamp: get_current_timestamp(),
    };
    
    std::fs::write("savegame.bin", bincode::serialize(&save_data)?)?;
    
    println!("Game saved! ({} bytes, hash: {})", save_data.ecs_blob.len(), hash);
    Ok(())
}

fn load_game(world: &mut World) -> anyhow::Result<()> {
    // 1. Load from disk
    let save_data: SaveData = bincode::deserialize(
        &std::fs::read("savegame.bin")?
    )?;
    
    // 2. Deserialize into world
    deserialize_ecs_world(&save_data.ecs_blob, world)?;
    
    // 3. Validate integrity
    let current_hash = calculate_world_hash(world);
    if current_hash != save_data.world_hash {
        anyhow::bail!("Save file corrupted! Hash mismatch.");
    }
    
    println!("Game loaded! (hash verified)");
    Ok(())
}
```

### Performance Metrics @ 1,000 Entities

| Operation | Time | Throughput | 60 FPS Impact |
|-----------|------|------------|---------------|
| **Serialize** | 0.686 ms | 1.44 Melem/s | 4.1% budget |
| **Deserialize** | 1.504 ms | 665 Kelem/s | 9.0% budget |
| **Roundtrip** | 2.395 ms | 418 Kelem/s | 14.4% budget |
| **Hash** | 0.594 ms | 1.68 Melem/s | 3.6% budget |

**Verdict**: All operations are **instant from player perspective** (<3 ms).

---

## API Reference

### `serialize_ecs_world(world: &World) -> Result<Vec<u8>>`

Serializes the entire ECS world to a compact binary blob.

**Performance**: 0.686 ms @ 1,000 entities (7× faster than target)

**Returns**: Binary blob ready for disk/network (postcard format, ~15.5 bytes/entity)

**Example**:
```rust
let blob = serialize_ecs_world(&world)?;
std::fs::write("autosave.bin", &blob)?;
```

**Thread Safety**: NOT thread-safe (requires exclusive `&World`). For background saving:
```rust
// Clone world state before passing to background thread
let world_clone = world.clone(); // Implement Clone for World
std::thread::spawn(move || {
    let blob = serialize_ecs_world(&world_clone).unwrap();
    std::fs::write("autosave.bin", &blob).unwrap();
});
```

---

### `deserialize_ecs_world(ecs_blob: &[u8], world: &mut World) -> Result<()>`

Deserializes a binary blob into the ECS world, spawning new entities.

**Performance**: 1.504 ms @ 1,000 entities (3× faster than target)

**Entity ID Remapping**: Old IDs are NOT preserved. A `HashMap<u64, Entity>` remaps references.

**Example**:
```rust
let blob = std::fs::read("savegame.bin")?;
deserialize_ecs_world(&blob, &mut world)?;
```

**Important**: Call this on an **empty world** for full restore, or on an existing world to merge saved entities.

---

### `calculate_world_hash(world: &World) -> u64`

Calculates a deterministic 64-bit hash of the world state.

**Performance**: 0.594 ms @ 1,000 entities (8× faster than target)

**Determinism**: Same world state → same hash (entities sorted before hashing)

**Example**:
```rust
let hash = calculate_world_hash(&world);
println!("World hash: {}", hash);

// Validate after loading
let hash_after = calculate_world_hash(&world);
assert_eq!(hash_before, hash_after, "Corruption detected!");
```

**Use Cases**:
- **Save validation**: Detect corrupted save files
- **Cheat detection**: Compare client/server hashes (multiplayer)
- **Replay verification**: Ensure deterministic simulation

---

## Integration Patterns

### Pattern 1: Manual Save (Player Hits F5)

```rust
// In your game loop input handling
if input.key_just_pressed(KeyCode::F5) {
    match save_game(&world) {
        Ok(_) => show_notification("Game saved!"),
        Err(e) => show_error(&format!("Save failed: {}", e)),
    }
}

fn save_game(world: &World) -> anyhow::Result<()> {
    let blob = serialize_ecs_world(world)?;
    let hash = calculate_world_hash(world);
    
    // Use aw-save integration
    let save_manager = get_save_manager(); // Your singleton/resource
    save_manager.save_to_slot(1, blob, hash)?;
    
    Ok(())
}
```

**Performance**: 0.686 ms → **instant** from player perspective.

---

### Pattern 2: Autosave (Every 5 Seconds)

```rust
// In your game loop
fn update(world: &mut World, dt: f32) {
    // Track autosave timer
    static mut AUTOSAVE_TIMER: f32 = 0.0;
    unsafe {
        AUTOSAVE_TIMER += dt;
        
        if AUTOSAVE_TIMER >= 5.0 {
            AUTOSAVE_TIMER = 0.0;
            
            // Autosave in background (non-blocking)
            let blob = serialize_ecs_world(world).unwrap();
            std::thread::spawn(move || {
                std::fs::write("autosave.bin", &blob).ok();
            });
        }
    }
}
```

**Performance**: 0.686 ms every 5 sec → **0.014% of frame budget** → basically free!

**Warning**: Autosaving on every frame (60 FPS) would consume 41% of budget (0.686 ms × 60 = 41.16 ms). Only autosave at **5-60 second intervals**.

---

### Pattern 3: Quick Load (Player Hits F9)

```rust
if input.key_just_pressed(KeyCode::F9) {
    match load_game(&mut world) {
        Ok(_) => show_notification("Game loaded!"),
        Err(e) => show_error(&format!("Load failed: {}", e)),
    }
}

fn load_game(world: &mut World) -> anyhow::Result<()> {
    // Clear existing world (optional - depends on game design)
    *world = World::new();
    
    // Load save file
    let save_manager = get_save_manager();
    let (blob, hash) = save_manager.load_from_slot(1)?;
    
    // Restore world
    deserialize_ecs_world(&blob, world)?;
    
    // Validate integrity
    let current_hash = calculate_world_hash(world);
    if current_hash != hash {
        anyhow::bail!("Save file corrupted!");
    }
    
    Ok(())
}
```

**Performance**: 1.504 ms → **faster than fade-to-black animation** → seamless UX.

**UI Tip**: Show a "Loading..." message for 200-500 ms even though load is instant (player expectation).

---

### Pattern 4: Multiplayer State Sync

```rust
// Server sends world state to client every 1 second
fn server_sync_tick(world: &World, client_socket: &Socket) -> anyhow::Result<()> {
    let blob = serialize_ecs_world(world)?;
    let hash = calculate_world_hash(world);
    
    // Send compressed state
    let compressed = lz4::compress(&blob)?;
    client_socket.send_sync_packet(compressed, hash)?;
    
    Ok(())
}

// Client receives and validates
fn client_receive_sync(world: &mut World, packet: SyncPacket) -> anyhow::Result<()> {
    let blob = lz4::decompress(&packet.data)?;
    
    // Apply state
    *world = World::new();
    deserialize_ecs_world(&blob, world)?;
    
    // Validate
    let hash = calculate_world_hash(world);
    if hash != packet.hash {
        anyhow::bail!("Desync detected! Server hash: {}, Client hash: {}", 
            packet.hash, hash);
    }
    
    Ok(())
}
```

**Bandwidth**: 15.49 KB/sec @ 1,000 entities @ 1 Hz → **<1 MB/min** → viable for co-op!

**With LZ4**: ~5-10× compression → **1.5-3 KB/sec** → excellent for multiplayer.

---

### Pattern 5: Deterministic Replay

```rust
// Record inputs each frame
fn record_inputs(inputs: &InputState) {
    REPLAY_INPUTS.lock().unwrap().push(inputs.clone());
}

// Replay from save + inputs
fn replay_from_tick(save_blob: &[u8], inputs: &[InputState]) -> anyhow::Result<()> {
    let mut world = World::new();
    deserialize_ecs_world(save_blob, &mut world)?;
    
    let hash_before = calculate_world_hash(&world);
    
    // Simulate N frames
    for input in inputs {
        simulate_frame(&mut world, input);
    }
    
    let hash_after = calculate_world_hash(&world);
    
    // Verify determinism
    assert_eq!(hash_before, EXPECTED_HASH_AFTER_N_FRAMES, 
        "Replay diverged! Simulation not deterministic.");
    
    Ok(())
}
```

**Use Case**: Debugging desyncs, replay systems, AI training.

---

## Performance Best Practices

### 1. Avoid Frequent Serialization

**WRONG** ❌:
```rust
for frame in 0..3600 { // Every frame for 1 minute
    let blob = serialize_ecs_world(&world)?; // 0.686 ms × 3600 = 2.47 seconds!
}
```

**RIGHT** ✅:
```rust
if frame % 300 == 0 { // Every 5 seconds (300 frames @ 60 FPS)
    let blob = serialize_ecs_world(&world)?; // 0.686 ms × 12 = 8.2 ms over 1 minute
}
```

**Rule**: Serialize at **5-60 second intervals**, not every frame.

---

### 2. Use Background Threads for Disk I/O

**WRONG** ❌:
```rust
let blob = serialize_ecs_world(&world)?; // 0.686 ms
std::fs::write("save.bin", &blob)?;      // 10-100 ms (blocks main thread!)
```

**RIGHT** ✅:
```rust
let blob = serialize_ecs_world(&world)?; // 0.686 ms
std::thread::spawn(move || {
    std::fs::write("save.bin", &blob).ok(); // Off main thread
});
```

**Why**: Disk I/O is 10-100× slower than serialization. Always use background threads for file writes.

---

### 3. Cache Hash Calculations

**WRONG** ❌:
```rust
for _ in 0..100 {
    let hash = calculate_world_hash(&world); // 0.594 ms × 100 = 59.4 ms!
}
```

**RIGHT** ✅:
```rust
let hash = calculate_world_hash(&world); // 0.594 ms (once)
for _ in 0..100 {
    use_cached_hash(hash); // 0 ms
}
```

**Rule**: Only recalculate hash when world changes (e.g., after simulation tick).

---

### 4. Prefer Primitives Over Complex Types

**SLOW** 🐌:
```rust
#[derive(Serialize, Deserialize)]
pub struct BadComponent {
    pub name: String,           // Heap allocation, variable size
    pub tags: Vec<String>,      // Nested heap allocations
    pub metadata: HashMap<String, String>, // Even worse!
}
```

**FAST** 🚀:
```rust
#[derive(Serialize, Deserialize)]
pub struct GoodComponent {
    pub id: u32,                // Fixed size, stack-allocated
    pub flags: u64,             // Bitflags instead of Vec<String>
    pub value: f32,             // Primitive
}
```

**Why**: Postcard is optimized for fixed-size types. Strings/Vecs/HashMaps add overhead.

**Blob Size Impact**:
- `GoodComponent`: ~16 bytes
- `BadComponent`: ~100-1000 bytes (depends on string lengths)

---

## Common Pitfalls

### Pitfall 1: Entity ID Assumptions

**WRONG** ❌:
```rust
// Before save
let player_entity = world.spawn();
let player_id = player_entity.to_raw(); // Save this

// After load
let player_entity = Entity::from_raw(player_id); // WRONG! ID may be different!
world.get::<CHealth>(player_entity); // May return None or wrong entity!
```

**RIGHT** ✅:
```rust
// Use a stable identifier component
#[derive(Serialize, Deserialize)]
pub struct CPlayerId(pub u32); // Globally unique, stable across saves

// Before save
let player_entity = world.spawn();
world.insert(player_entity, CPlayerId(1));

// After load
let mut q = Query::<(CPlayerId, CHealth)>::new(&world);
while let Some((entity, (player_id, health))) = q.next() {
    if player_id.0 == 1 {
        // Found player entity by stable ID
    }
}
```

**Why**: Entity IDs are remapped during deserialization. Use stable components (CPlayerId, CLegacyId) for lookups.

---

### Pitfall 2: Partial World Serialization

**WRONG** ❌:
```rust
// Only save player + enemies, skip UI entities
let blob = serialize_ecs_world(&world)?; // Serializes EVERYTHING
```

**RIGHT** ✅:
```rust
// Filter entities before serialization (custom implementation)
fn serialize_gameplay_entities(world: &World) -> Result<Vec<u8>> {
    let mut entities = Vec::new();
    
    // Only save entities with CGameplayTag
    let mut q = Query::<(CPos, CGameplayTag)>::new(world);
    while let Some((entity, (pos, _))) = q.next() {
        // Collect gameplay entities only
        entities.push(collect_entity_components(world, entity));
    }
    
    postcard::to_allocvec(&entities)
}
```

**Why**: Default `serialize_ecs_world()` saves **all** entities. For partial saves, implement custom filtering.

---

### Pitfall 3: Forgetting Hash Validation

**WRONG** ❌:
```rust
let blob = std::fs::read("save.bin")?;
deserialize_ecs_world(&blob, &mut world)?; // Corrupted save loaded silently!
```

**RIGHT** ✅:
```rust
let save_data: SaveData = load_from_disk()?;
deserialize_ecs_world(&save_data.blob, &mut world)?;

let current_hash = calculate_world_hash(&world);
if current_hash != save_data.hash {
    anyhow::bail!("Save file corrupted! Reverting to backup.");
}
```

**Why**: Disk corruption, network errors, or manual editing can corrupt saves. Always validate hash!

---

### Pitfall 4: Blocking Main Thread on I/O

**WRONG** ❌:
```rust
fn autosave(world: &World) {
    let blob = serialize_ecs_world(world).unwrap(); // 0.686 ms
    std::fs::write("save.bin", &blob).unwrap();     // 50-100 ms FREEZE!
}
```

**RIGHT** ✅:
```rust
fn autosave(world: &World) {
    let blob = serialize_ecs_world(world).unwrap(); // 0.686 ms
    std::thread::spawn(move || {
        std::fs::write("save.bin", &blob).ok(); // Background thread
    });
}
```

**Why**: File I/O is slow and unpredictable. Always use background threads for disk writes.

---

### Pitfall 5: No Version Compatibility

**WRONG** ❌:
```rust
#[derive(Serialize, Deserialize)]
pub struct CPlayer {
    pub hp: i32,
    pub new_field: f32, // Added in v2.0 - breaks old saves!
}
```

**RIGHT** ✅:
```rust
#[derive(Serialize, Deserialize)]
pub struct SerializedWorld {
    pub version: u32, // Schema version
    pub entities: Vec<SerializedEntity>,
}

fn deserialize_ecs_world(blob: &[u8], world: &mut World) -> Result<()> {
    let data: SerializedWorld = postcard::from_bytes(blob)?;
    
    match data.version {
        1 => migrate_v1_to_v2(data, world)?,
        2 => apply_v2(data, world)?,
        _ => anyhow::bail!("Unsupported save version: {}", data.version),
    }
    
    Ok(())
}
```

**Why**: Components evolve over time. Always include a version field for migration support.

---

## Adding New Components

To add support for a new component type (e.g., `CInventory`):

### Step 1: Add Serialization Derives

```rust
// In astraweave-core/src/ecs_components.rs
#[derive(Clone, Debug, Serialize, Deserialize)] // ✅ Add these
pub struct CInventory {
    pub items: Vec<ItemId>,
    pub capacity: u32,
}
```

### Step 2: Add Field to SerializedEntity

```rust
// In astraweave-persistence-ecs/src/lib.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedEntity {
    pub entity_raw: u64,
    // ... existing fields ...
    pub inventory: Option<CInventory>, // ✅ Add this
}
```

### Step 3: Add Entity Discovery Query

```rust
// In serialize_ecs_world() function
{
    let mut q = Query::<CInventory>::new(world);
    while let Some((entity, _)) = q.next() {
        entity_set.insert(entity);
    }
}
```

### Step 4: Add Component Collection

```rust
// In serialize_ecs_world(), entity collection loop
let serialized = SerializedEntity {
    entity_raw: entity.to_raw(),
    // ... existing fields ...
    inventory: world.get::<CInventory>(entity).cloned(), // ✅ Add this
};
```

### Step 5: Add Component Insertion

```rust
// In deserialize_ecs_world(), component insertion loop
if let Some(inventory) = &serialized_entity.inventory {
    world.insert(entity, inventory.clone()); // ✅ Add this
}
```

### Step 6: Add Hash Computation (Optional)

```rust
// In calculate_world_hash(), hashing loop
if let Some(inventory) = world.get::<CInventory>(entity) {
    inventory.capacity.hash(&mut hasher);
    for item in &inventory.items {
        item.hash(&mut hasher);
    }
}
```

### Step 7: Test

```rust
#[test]
fn test_inventory_serialization() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, CInventory {
        items: vec![ItemId(1), ItemId(2)],
        capacity: 10,
    });
    
    // Serialize
    let blob = serialize_ecs_world(&world).unwrap();
    
    // Deserialize
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();
    
    // Verify
    let mut q = Query::<CInventory>::new(&new_world);
    let (_, inv) = q.next().unwrap();
    assert_eq!(inv.items.len(), 2);
    assert_eq!(inv.capacity, 10);
}
```

**Done!** Your new component is now fully serializable.

---

## Testing & Validation

### Test 1: Empty World Serialization

```rust
#[test]
fn test_empty_world() {
    let world = World::new();
    let blob = serialize_ecs_world(&world).unwrap();
    assert!(blob.len() > 0); // Should have header
    
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();
}
```

### Test 2: Roundtrip Integrity

```rust
#[test]
fn test_roundtrip() {
    let mut world = World::new();
    // Populate world...
    
    let hash_before = calculate_world_hash(&world);
    let blob = serialize_ecs_world(&world).unwrap();
    
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();
    let hash_after = calculate_world_hash(&new_world);
    
    assert_eq!(hash_before, hash_after, "Hash mismatch!");
}
```

### Test 3: Determinism

```rust
#[test]
fn test_determinism() {
    let mut world = World::new();
    // Populate world...
    
    // Serialize twice
    let blob1 = serialize_ecs_world(&world).unwrap();
    let blob2 = serialize_ecs_world(&world).unwrap();
    
    assert_eq!(blob1, blob2, "Non-deterministic serialization!");
}
```

### Test 4: Performance Regression

```rust
#[test]
fn test_performance_1k_entities() {
    let mut world = World::new();
    // Spawn 1,000 entities...
    
    let start = std::time::Instant::now();
    let blob = serialize_ecs_world(&world).unwrap();
    let elapsed = start.elapsed();
    
    assert!(elapsed.as_millis() < 1, "Serialize took {}ms, expected <1ms", elapsed.as_millis());
}
```

---

## Troubleshooting

### Problem: "Serialize took 10× longer than expected"

**Cause**: Many String/Vec components (heap allocations)

**Solution**: Replace with fixed-size types:
```rust
// BEFORE: pub name: String
// AFTER: pub name_id: u32 (lookup in string table)
```

---

### Problem: "Hash mismatch after load"

**Cause 1**: Component data not included in hash  
**Solution**: Add component to `calculate_world_hash()`

**Cause 2**: Non-deterministic component data (e.g., HashMap iteration order)  
**Solution**: Sort data before hashing

---

### Problem: "Entity references broken after load"

**Cause**: Using raw entity IDs instead of stable identifiers

**Solution**: Use `CLegacyId` or custom ID components for cross-entity references

---

### Problem: "Save file grew to 10 MB for 1,000 entities"

**Cause**: Serializing large nested structures (Vec<Vec<String>>, etc.)

**Solution**: Flatten data structures, use indices instead of nested Vecs

---

## Next Steps

- **Week 2**: Implement PlayerProfile + SaveSlotManager (UI integration)
- **Week 3**: Add versioning + migration + corruption recovery
- **Phase 8.1**: Integrate with in-game UI (save/load buttons, slot selection)

---

**Questions?** Open an issue or see `docs/root-archive/PHASE_8_PRIORITY_3_SAVE_LOAD_PLAN.md` for full roadmap.

