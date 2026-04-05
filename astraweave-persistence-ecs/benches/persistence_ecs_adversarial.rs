//! Adversarial benchmarks for astraweave-persistence-ecs
//!
//! Tests ECS persistence under extreme conditions:
//! - World serialization at massive scale
//! - Component save/load throughput
//! - Incremental snapshot performance
//! - Compression efficiency for game state
//! - Migration/versioning overhead
//! - Corruption detection and recovery

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box as std_black_box;

// ============================================================================
// LOCAL TYPE DEFINITIONS (Standalone benchmark - no crate imports)
// ============================================================================

/// Entity identifier for persistence
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct EntityId(u64);

/// Component identifier for type registry
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ComponentTypeId(u32);

/// Version for migration support
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}

impl Version {
    fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    fn serialize(&self) -> [u8; 6] {
        let mut bytes = [0u8; 6];
        bytes[0..2].copy_from_slice(&self.major.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.minor.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.patch.to_le_bytes());
        bytes
    }
}

/// Transform component
#[derive(Clone, Copy, Debug, Default)]
struct Transform {
    x: f32,
    y: f32,
    z: f32,
    rot_x: f32,
    rot_y: f32,
    rot_z: f32,
    rot_w: f32,
    scale_x: f32,
    scale_y: f32,
    scale_z: f32,
}

impl Transform {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            rot_w: 1.0,
            scale_x: 1.0,
            scale_y: 1.0,
            scale_z: 1.0,
            ..Default::default()
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(40);
        bytes.extend_from_slice(&self.x.to_le_bytes());
        bytes.extend_from_slice(&self.y.to_le_bytes());
        bytes.extend_from_slice(&self.z.to_le_bytes());
        bytes.extend_from_slice(&self.rot_x.to_le_bytes());
        bytes.extend_from_slice(&self.rot_y.to_le_bytes());
        bytes.extend_from_slice(&self.rot_z.to_le_bytes());
        bytes.extend_from_slice(&self.rot_w.to_le_bytes());
        bytes.extend_from_slice(&self.scale_x.to_le_bytes());
        bytes.extend_from_slice(&self.scale_y.to_le_bytes());
        bytes.extend_from_slice(&self.scale_z.to_le_bytes());
        bytes
    }

    fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 40 {
            return None;
        }
        Some(Self {
            x: f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            y: f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            z: f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            rot_x: f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            rot_y: f32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            rot_z: f32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            rot_w: f32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            scale_x: f32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            scale_y: f32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]),
            scale_z: f32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]),
        })
    }
}

/// Health component
#[derive(Clone, Copy, Debug)]
struct Health {
    current: f32,
    max: f32,
    regeneration: f32,
}

impl Health {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&self.current.to_le_bytes());
        bytes.extend_from_slice(&self.max.to_le_bytes());
        bytes.extend_from_slice(&self.regeneration.to_le_bytes());
        bytes
    }

    fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 {
            return None;
        }
        Some(Self {
            current: f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            max: f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            regeneration: f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        })
    }
}

/// Inventory component with variable-size data
#[derive(Clone, Debug)]
struct Inventory {
    slots: Vec<InventorySlot>,
    max_weight: f32,
    current_weight: f32,
}

#[derive(Clone, Debug)]
struct InventorySlot {
    item_id: u32,
    quantity: u16,
    durability: f32,
    custom_data: Vec<u8>,
}

impl Inventory {
    fn new(max_slots: usize) -> Self {
        Self {
            slots: Vec::with_capacity(max_slots),
            max_weight: 100.0,
            current_weight: 0.0,
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.slots.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.max_weight.to_le_bytes());
        bytes.extend_from_slice(&self.current_weight.to_le_bytes());

        for slot in &self.slots {
            bytes.extend_from_slice(&slot.item_id.to_le_bytes());
            bytes.extend_from_slice(&slot.quantity.to_le_bytes());
            bytes.extend_from_slice(&slot.durability.to_le_bytes());
            bytes.extend_from_slice(&(slot.custom_data.len() as u32).to_le_bytes());
            bytes.extend(&slot.custom_data);
        }

        bytes
    }

    fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 {
            return None;
        }

        let slot_count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let max_weight = f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let current_weight = f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        let mut slots = Vec::with_capacity(slot_count);
        let mut offset = 12;

        for _ in 0..slot_count {
            if offset + 14 > bytes.len() {
                return None;
            }

            let item_id = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            let quantity = u16::from_le_bytes([bytes[offset + 4], bytes[offset + 5]]);
            let durability = f32::from_le_bytes([
                bytes[offset + 6],
                bytes[offset + 7],
                bytes[offset + 8],
                bytes[offset + 9],
            ]);
            let custom_len = u32::from_le_bytes([
                bytes[offset + 10],
                bytes[offset + 11],
                bytes[offset + 12],
                bytes[offset + 13],
            ]) as usize;

            offset += 14;
            if offset + custom_len > bytes.len() {
                return None;
            }

            let custom_data = bytes[offset..offset + custom_len].to_vec();
            offset += custom_len;

            slots.push(InventorySlot {
                item_id,
                quantity,
                durability,
                custom_data,
            });
        }

        Some(Self {
            slots,
            max_weight,
            current_weight,
        })
    }
}

/// AI state component
#[derive(Clone, Debug)]
struct AIState {
    behavior_tree_id: u32,
    current_state: u32,
    blackboard: HashMap<String, Vec<u8>>,
    last_decision_time: f64,
}

impl AIState {
    fn new(behavior_tree: u32) -> Self {
        Self {
            behavior_tree_id: behavior_tree,
            current_state: 0,
            blackboard: HashMap::new(),
            last_decision_time: 0.0,
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.behavior_tree_id.to_le_bytes());
        bytes.extend_from_slice(&self.current_state.to_le_bytes());
        bytes.extend_from_slice(&self.last_decision_time.to_le_bytes());
        bytes.extend_from_slice(&(self.blackboard.len() as u32).to_le_bytes());

        for (key, value) in &self.blackboard {
            let key_bytes = key.as_bytes();
            bytes.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
            bytes.extend(key_bytes);
            bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
            bytes.extend(value);
        }

        bytes
    }
}

/// Serialized entity with all components
#[derive(Clone, Debug)]
struct SerializedEntity {
    id: EntityId,
    components: HashMap<ComponentTypeId, Vec<u8>>,
}

impl SerializedEntity {
    fn new(id: u64) -> Self {
        Self {
            id: EntityId(id),
            components: HashMap::new(),
        }
    }

    fn add_component(&mut self, type_id: u32, data: Vec<u8>) {
        self.components.insert(ComponentTypeId(type_id), data);
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.0.to_le_bytes());
        bytes.extend_from_slice(&(self.components.len() as u32).to_le_bytes());

        for (type_id, data) in &self.components {
            bytes.extend_from_slice(&type_id.0.to_le_bytes());
            bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }

        bytes
    }

    #[allow(dead_code)]
    fn byte_size(&self) -> usize {
        8 + 4
            + self
                .components
                .values()
                .map(|d| 4 + 4 + d.len())
                .sum::<usize>()
    }
}

/// World snapshot for incremental saves
#[derive(Clone, Debug)]
struct WorldSnapshot {
    version: Version,
    tick: u64,
    timestamp: u64,
    entities: Vec<SerializedEntity>,
    checksum: u32,
}

impl WorldSnapshot {
    fn new(version: Version, tick: u64) -> Self {
        Self {
            version,
            tick,
            timestamp: 0,
            entities: Vec::new(),
            checksum: 0,
        }
    }

    fn add_entity(&mut self, entity: SerializedEntity) {
        self.entities.push(entity);
    }

    fn compute_checksum(&mut self) {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        for entity in &self.entities {
            hasher.update(&entity.id.0.to_le_bytes());
            for (type_id, data) in &entity.components {
                hasher.update(&type_id.0.to_le_bytes());
                hasher.update(data);
            }
        }
        self.checksum = hasher.finalize();
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.serialize());
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&(self.entities.len() as u32).to_le_bytes());

        for entity in &self.entities {
            let entity_bytes = entity.serialize();
            bytes.extend_from_slice(&(entity_bytes.len() as u32).to_le_bytes());
            bytes.extend(entity_bytes);
        }

        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes
    }
}

/// Incremental delta between snapshots
#[derive(Clone, Debug)]
struct SnapshotDelta {
    base_tick: u64,
    target_tick: u64,
    added_entities: Vec<SerializedEntity>,
    modified_components: Vec<(EntityId, ComponentTypeId, Vec<u8>)>,
    removed_entities: Vec<EntityId>,
    removed_components: Vec<(EntityId, ComponentTypeId)>,
}

impl SnapshotDelta {
    fn new(base: u64, target: u64) -> Self {
        Self {
            base_tick: base,
            target_tick: target,
            added_entities: Vec::new(),
            modified_components: Vec::new(),
            removed_entities: Vec::new(),
            removed_components: Vec::new(),
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.base_tick.to_le_bytes());
        bytes.extend_from_slice(&self.target_tick.to_le_bytes());

        // Added entities
        bytes.extend_from_slice(&(self.added_entities.len() as u32).to_le_bytes());
        for entity in &self.added_entities {
            let entity_bytes = entity.serialize();
            bytes.extend_from_slice(&(entity_bytes.len() as u32).to_le_bytes());
            bytes.extend(entity_bytes);
        }

        // Modified components
        bytes.extend_from_slice(&(self.modified_components.len() as u32).to_le_bytes());
        for (entity_id, type_id, data) in &self.modified_components {
            bytes.extend_from_slice(&entity_id.0.to_le_bytes());
            bytes.extend_from_slice(&type_id.0.to_le_bytes());
            bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }

        // Removed entities
        bytes.extend_from_slice(&(self.removed_entities.len() as u32).to_le_bytes());
        for entity_id in &self.removed_entities {
            bytes.extend_from_slice(&entity_id.0.to_le_bytes());
        }

        // Removed components
        bytes.extend_from_slice(&(self.removed_components.len() as u32).to_le_bytes());
        for (entity_id, type_id) in &self.removed_components {
            bytes.extend_from_slice(&entity_id.0.to_le_bytes());
            bytes.extend_from_slice(&type_id.0.to_le_bytes());
        }

        bytes
    }

    #[allow(dead_code)]
    fn byte_size(&self) -> usize {
        self.serialize().len()
    }
}

/// LZ4-style compression simulation
fn compress_lz4(data: &[u8]) -> Vec<u8> {
    // Simple RLE for benchmarking
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        let mut run: u8 = 1;
        while i + (run as usize) < data.len() && data[i] == data[i + run as usize] && run < 255 {
            run += 1;
        }

        if run > 3 {
            result.push(0x00);
            result.push(run);
            result.push(data[i]);
            i += run as usize;
        } else {
            result.push(data[i]);
            i += 1;
        }
    }

    result
}

fn decompress_lz4(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < data.len() {
        if data[i] == 0x00 && i + 2 < data.len() {
            let run = data[i + 1] as usize;
            let byte = data[i + 2];
            for _ in 0..run {
                result.push(byte);
            }
            i += 3;
        } else {
            result.push(data[i]);
            i += 1;
        }
    }

    result
}

/// Simple checksum for CRC32 simulation
struct Crc32Hasher {
    state: u32,
}

impl Crc32Hasher {
    fn new() -> Self {
        Self { state: 0xFFFFFFFF }
    }

    fn update(&mut self, data: &[u8]) {
        for byte in data {
            self.state ^= *byte as u32;
            for _ in 0..8 {
                if self.state & 1 != 0 {
                    self.state = (self.state >> 1) ^ 0xEDB88320;
                } else {
                    self.state >>= 1;
                }
            }
        }
    }

    fn finalize(self) -> u32 {
        !self.state
    }
}

// CRC32 implementation for checksum
mod crc32fast {
    pub struct Hasher {
        state: u32,
    }

    impl Hasher {
        pub fn new() -> Self {
            Self { state: 0xFFFFFFFF }
        }

        pub fn update(&mut self, data: &[u8]) {
            for byte in data {
                self.state ^= *byte as u32;
                for _ in 0..8 {
                    if self.state & 1 != 0 {
                        self.state = (self.state >> 1) ^ 0xEDB88320;
                    } else {
                        self.state >>= 1;
                    }
                }
            }
        }

        pub fn finalize(self) -> u32 {
            !self.state
        }
    }
}

// ============================================================================
// BENCHMARK GROUPS
// ============================================================================

fn bench_entity_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_serialization");

    for entity_count in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        // Create entities with various components
        let entities: Vec<SerializedEntity> = (0..entity_count)
            .map(|i| {
                let mut entity = SerializedEntity::new(i as u64);

                // Transform (always present)
                entity.add_component(0, Transform::new(i as f32, 0.0, i as f32 * 0.5).serialize());

                // Health (50% of entities)
                if i % 2 == 0 {
                    entity.add_component(
                        1,
                        Health {
                            current: 100.0 - (i as f32 % 50.0),
                            max: 100.0,
                            regeneration: 0.5,
                        }
                        .serialize(),
                    );
                }

                // Inventory (20% of entities)
                if i % 5 == 0 {
                    let mut inv = Inventory::new(20);
                    for j in 0..5 {
                        inv.slots.push(InventorySlot {
                            item_id: (i * 10 + j) as u32,
                            quantity: ((i + j) % 99 + 1) as u16,
                            durability: 0.5 + (j as f32 * 0.1),
                            custom_data: vec![0u8; 16],
                        });
                    }
                    entity.add_component(2, inv.serialize());
                }

                // AI State (10% of entities)
                if i % 10 == 0 {
                    let mut ai = AIState::new((i % 5) as u32);
                    ai.blackboard.insert("target".to_string(), vec![0u8; 8]);
                    ai.blackboard.insert("state".to_string(), vec![1u8; 4]);
                    entity.add_component(3, ai.serialize());
                }

                entity
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("serialize_entities", entity_count),
            &entities,
            |b, entities| {
                b.iter(|| {
                    let bytes: Vec<Vec<u8>> = entities.iter().map(|e| e.serialize()).collect();
                    std_black_box(bytes)
                })
            },
        );

        // Total byte size
        let total_bytes: usize = entities.iter().map(|e| e.byte_size()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize_with_compression", entity_count),
            &entities,
            |b, entities| {
                b.iter(|| {
                    let bytes: Vec<u8> = entities.iter().flat_map(|e| e.serialize()).collect();
                    let compressed = compress_lz4(&bytes);
                    std_black_box(compressed)
                })
            },
        );
    }

    group.finish();
}

fn bench_world_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_snapshot");

    for entity_count in [100, 500, 1000, 5000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::new("create_snapshot", entity_count),
            &entity_count,
            |b, &count| {
                b.iter(|| {
                    let mut snapshot = WorldSnapshot::new(Version::new(1, 0, 0), 12345);

                    for i in 0..count {
                        let mut entity = SerializedEntity::new(i as u64);
                        entity.add_component(
                            0,
                            Transform::new(i as f32, (i as f32).sin() * 10.0, i as f32 * 0.5)
                                .serialize(),
                        );
                        if i % 2 == 0 {
                            entity.add_component(
                                1,
                                Health {
                                    current: 75.0,
                                    max: 100.0,
                                    regeneration: 0.5,
                                }
                                .serialize(),
                            );
                        }
                        snapshot.add_entity(entity);
                    }

                    snapshot.compute_checksum();
                    std_black_box(snapshot.serialize())
                })
            },
        );
    }

    // Snapshot with checksum validation
    group.bench_function("checksum_1000_entities", |b| {
        let mut snapshot = WorldSnapshot::new(Version::new(1, 0, 0), 0);
        for i in 0..1000 {
            let mut entity = SerializedEntity::new(i);
            entity.add_component(0, Transform::new(i as f32, 0.0, 0.0).serialize());
            snapshot.add_entity(entity);
        }

        b.iter(|| {
            let mut s = snapshot.clone();
            s.compute_checksum();
            std_black_box(s.checksum)
        })
    });

    group.finish();
}

fn bench_incremental_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_delta");

    // Create base state
    let _base_entities: Vec<SerializedEntity> = (0..1000)
        .map(|i| {
            let mut entity = SerializedEntity::new(i);
            entity.add_component(0, Transform::new(i as f32, 0.0, 0.0).serialize());
            entity
        })
        .collect();

    for change_percent in [1, 5, 10, 25, 50] {
        group.throughput(Throughput::Elements(1000));

        let changed_count = (1000 * change_percent) / 100;

        group.bench_with_input(
            BenchmarkId::new("compute_delta", format!("{}pct_change", change_percent)),
            &changed_count,
            |b, &changed| {
                b.iter(|| {
                    let mut delta = SnapshotDelta::new(0, 1);

                    // Simulate changes
                    for i in 0..changed {
                        delta.modified_components.push((
                            EntityId(i as u64),
                            ComponentTypeId(0),
                            Transform::new(i as f32 + 0.1, 0.5, 0.0).serialize(),
                        ));
                    }

                    // Add some new entities (5% of changed)
                    for i in 0..(changed / 20).max(1) {
                        let mut entity = SerializedEntity::new(1000 + i as u64);
                        entity.add_component(0, Transform::new(0.0, 0.0, 0.0).serialize());
                        delta.added_entities.push(entity);
                    }

                    // Remove some entities (2% of changed)
                    for i in 0..(changed / 50).max(1) {
                        delta.removed_entities.push(EntityId(900 + i as u64));
                    }

                    std_black_box(delta.serialize())
                })
            },
        );
    }

    // Delta vs full snapshot comparison
    group.bench_function("delta_vs_full_comparison", |b| {
        b.iter(|| {
            // Full snapshot
            let mut snapshot = WorldSnapshot::new(Version::new(1, 0, 0), 0);
            for i in 0..1000 {
                let mut entity = SerializedEntity::new(i);
                entity.add_component(0, Transform::new(i as f32, 0.0, 0.0).serialize());
                snapshot.add_entity(entity);
            }
            let full_bytes = snapshot.serialize();

            // Delta (10% changes)
            let mut delta = SnapshotDelta::new(0, 1);
            for i in 0..100 {
                delta.modified_components.push((
                    EntityId(i),
                    ComponentTypeId(0),
                    Transform::new(i as f32 + 0.1, 0.0, 0.0).serialize(),
                ));
            }
            let delta_bytes = delta.serialize();

            let ratio = full_bytes.len() as f32 / delta_bytes.len() as f32;
            std_black_box(ratio)
        })
    });

    group.finish();
}

fn bench_compression_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_efficiency");

    // Generate typical game state data
    let game_state_sizes = [1024, 10240, 102400, 1048576]; // 1KB, 10KB, 100KB, 1MB

    for size in game_state_sizes {
        group.throughput(Throughput::Bytes(size as u64));

        // Simulated game state with patterns
        let data: Vec<u8> = (0..size)
            .map(|i| {
                // Mix of patterns: repeated bytes, sequential, random-ish
                match i % 16 {
                    0..=3 => 0u8,                     // Zero runs
                    4..=7 => (i & 0xFF) as u8,        // Sequential
                    8..=11 => ((i * 7) & 0xFF) as u8, // Pseudo-random
                    _ => 0xFFu8,                      // High runs
                }
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("compress", format!("{}bytes", size)),
            &data,
            |b, data| b.iter(|| std_black_box(compress_lz4(data))),
        );

        let compressed = compress_lz4(&data);
        group.bench_with_input(
            BenchmarkId::new("decompress", format!("{}bytes", size)),
            &compressed,
            |b, compressed| b.iter(|| std_black_box(decompress_lz4(compressed))),
        );

        // Compression ratio calculation
        group.bench_with_input(
            BenchmarkId::new("ratio_analysis", format!("{}bytes", size)),
            &data,
            |b, data| {
                b.iter(|| {
                    let compressed = compress_lz4(data);
                    let ratio = data.len() as f32 / compressed.len() as f32;
                    std_black_box(ratio)
                })
            },
        );
    }

    group.finish();
}

fn bench_component_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_deserialization");

    // Pre-serialize components
    let transforms: Vec<Vec<u8>> = (0..1000)
        .map(|i| Transform::new(i as f32, (i as f32).sin() * 10.0, i as f32 * 0.5).serialize())
        .collect();

    group.throughput(Throughput::Elements(1000));

    group.bench_function("deserialize_transforms_1000", |b| {
        b.iter(|| {
            let deserialized: Vec<Option<Transform>> = transforms
                .iter()
                .map(|t| Transform::deserialize(t))
                .collect();
            std_black_box(deserialized)
        })
    });

    // Health deserialization
    let healths: Vec<Vec<u8>> = (0..1000)
        .map(|i| {
            Health {
                current: 100.0 - (i as f32 % 50.0),
                max: 100.0,
                regeneration: 0.5,
            }
            .serialize()
        })
        .collect();

    group.bench_function("deserialize_healths_1000", |b| {
        b.iter(|| {
            let deserialized: Vec<Option<Health>> =
                healths.iter().map(|h| Health::deserialize(h)).collect();
            std_black_box(deserialized)
        })
    });

    // Inventory (complex, variable size)
    let inventories: Vec<Vec<u8>> = (0..100)
        .map(|i| {
            let mut inv = Inventory::new(20);
            for j in 0..10 {
                inv.slots.push(InventorySlot {
                    item_id: (i * 10 + j) as u32,
                    quantity: ((i + j) % 99 + 1) as u16,
                    durability: 0.8,
                    custom_data: vec![0u8; 32],
                });
            }
            inv.serialize()
        })
        .collect();

    group.throughput(Throughput::Elements(100));
    group.bench_function("deserialize_inventories_100", |b| {
        b.iter(|| {
            let deserialized: Vec<Option<Inventory>> = inventories
                .iter()
                .map(|i| Inventory::deserialize(i))
                .collect();
            std_black_box(deserialized)
        })
    });

    group.finish();
}

fn bench_checksum_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("checksum_verification");

    for size in [1024, 10240, 102400, 1048576] {
        group.throughput(Throughput::Bytes(size as u64));

        let data: Vec<u8> = (0..size).map(|i| (i & 0xFF) as u8).collect();

        group.bench_with_input(
            BenchmarkId::new("crc32_compute", format!("{}bytes", size)),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut hasher = Crc32Hasher::new();
                    hasher.update(data);
                    std_black_box(hasher.finalize())
                })
            },
        );
    }

    // Verify snapshot integrity
    group.bench_function("verify_snapshot_integrity_1000", |b| {
        let mut snapshot = WorldSnapshot::new(Version::new(1, 0, 0), 0);
        for i in 0..1000 {
            let mut entity = SerializedEntity::new(i);
            entity.add_component(0, Transform::new(i as f32, 0.0, 0.0).serialize());
            snapshot.add_entity(entity);
        }
        snapshot.compute_checksum();
        let stored_checksum = snapshot.checksum;

        b.iter(|| {
            let mut verify = snapshot.clone();
            verify.compute_checksum();
            let valid = verify.checksum == stored_checksum;
            std_black_box(valid)
        })
    });

    group.finish();
}

fn bench_version_migration(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_migration");

    // Simulate version migration scenarios
    group.bench_function("version_check_1000", |b| {
        let current_version = Version::new(1, 5, 0);
        let saved_versions: Vec<Version> = (0..1000)
            .map(|i| Version::new(1, (i % 6) as u16, 0))
            .collect();

        b.iter(|| {
            let needs_migration: Vec<bool> = saved_versions
                .iter()
                .map(|v| *v < current_version)
                .collect();
            std_black_box(needs_migration)
        })
    });

    // Component migration (add new fields)
    group.bench_function("migrate_transforms_add_field_1000", |b| {
        // Old format: 28 bytes (no scale)
        let old_transforms: Vec<Vec<u8>> = (0..1000)
            .map(|i| {
                let mut bytes = Vec::with_capacity(28);
                bytes.extend_from_slice(&(i as f32).to_le_bytes()); // x
                bytes.extend_from_slice(&0.0f32.to_le_bytes()); // y
                bytes.extend_from_slice(&(i as f32 * 0.5).to_le_bytes()); // z
                bytes.extend_from_slice(&0.0f32.to_le_bytes()); // rot_x
                bytes.extend_from_slice(&0.0f32.to_le_bytes()); // rot_y
                bytes.extend_from_slice(&0.0f32.to_le_bytes()); // rot_z
                bytes.extend_from_slice(&1.0f32.to_le_bytes()); // rot_w
                bytes
            })
            .collect();

        b.iter(|| {
            // Migrate to new format with scale
            let new_transforms: Vec<Vec<u8>> = old_transforms
                .iter()
                .map(|old| {
                    let mut new = old.clone();
                    new.extend_from_slice(&1.0f32.to_le_bytes()); // scale_x
                    new.extend_from_slice(&1.0f32.to_le_bytes()); // scale_y
                    new.extend_from_slice(&1.0f32.to_le_bytes()); // scale_z
                    new
                })
                .collect();
            std_black_box(new_transforms)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_entity_serialization,
    bench_world_snapshot,
    bench_incremental_delta,
    bench_compression_efficiency,
    bench_component_deserialization,
    bench_checksum_verification,
    bench_version_migration,
);

criterion_main!(benches);
