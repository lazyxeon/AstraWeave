//! Adversarial IPC Benchmarks
//!
//! Stress testing for WebSocket server/client, serialization, and message handling.

#![allow(
    dead_code,
    unused_variables,
    unused_assignments,
    unused_parens,
    clippy::upper_case_acronyms,
    clippy::unnecessary_cast,
    clippy::double_parens,
    clippy::needless_borrow
)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::hint::black_box as std_black_box;

// ============================================================================
// LOCAL TYPES (Mirror astraweave-ipc API)
// ============================================================================

#[derive(Clone, Debug)]
struct WorldSnapshot {
    tick: u64,
    time: f64,
    entities: Vec<EntityState>,
    events: Vec<GameEvent>,
}

#[derive(Clone, Debug)]
struct EntityState {
    id: u64,
    position: [f32; 3],
    rotation: [f32; 4],
    health: f32,
    entity_type: EntityType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EntityType {
    Player,
    NPC,
    Enemy,
    Projectile,
    Item,
}

#[derive(Clone, Debug)]
struct GameEvent {
    id: u64,
    event_type: String,
    timestamp: f64,
    data: HashMap<String, String>,
}

#[derive(Clone, Debug)]
struct PlanIntent {
    plan_id: u64,
    agent_id: u64,
    actions: Vec<ActionStep>,
    priority: u32,
}

#[derive(Clone, Debug)]
struct ActionStep {
    action_type: String,
    target: Option<u64>,
    parameters: HashMap<String, f32>,
}

#[derive(Clone, Debug)]
enum Message {
    Snapshot(WorldSnapshot),
    Plan(PlanIntent),
    Command(String),
    Heartbeat(u64),
    Error(String),
}

fn generate_snapshot(entity_count: usize) -> WorldSnapshot {
    WorldSnapshot {
        tick: 12345,
        time: 123.456,
        entities: (0..entity_count)
            .map(|i| EntityState {
                id: i as u64,
                position: [
                    (i % 100) as f32,
                    ((i / 100) % 100) as f32,
                    (i / 10000) as f32,
                ],
                rotation: [0.0, 0.0, 0.0, 1.0],
                health: 100.0 - (i % 50) as f32,
                entity_type: match i % 5 {
                    0 => EntityType::Player,
                    1 => EntityType::NPC,
                    2 => EntityType::Enemy,
                    3 => EntityType::Projectile,
                    _ => EntityType::Item,
                },
            })
            .collect(),
        events: (0..10)
            .map(|i| GameEvent {
                id: i as u64,
                event_type: "damage".to_string(),
                timestamp: i as f64 * 0.1,
                data: [("amount".to_string(), "50".to_string())]
                    .into_iter()
                    .collect(),
            })
            .collect(),
    }
}

fn generate_plan() -> PlanIntent {
    PlanIntent {
        plan_id: 1,
        agent_id: 100,
        actions: vec![
            ActionStep {
                action_type: "move".to_string(),
                target: Some(200),
                parameters: [("speed".to_string(), 5.0)].into_iter().collect(),
            },
            ActionStep {
                action_type: "attack".to_string(),
                target: Some(300),
                parameters: [("damage".to_string(), 50.0)].into_iter().collect(),
            },
        ],
        priority: 1,
    }
}

// ============================================================================
// CATEGORY 1: SERIALIZATION
// ============================================================================

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_adversarial/serialization");

    // Test 1: Snapshot to JSON-like string
    for entity_count in [100, 500, 1000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::new("snapshot_serialize", entity_count),
            &entity_count,
            |bencher, &count| {
                let snapshot = generate_snapshot(count);

                bencher.iter(|| {
                    // Simulate JSON serialization
                    let mut json = String::with_capacity(count * 100);
                    json.push_str("{\"tick\":");
                    json.push_str(&snapshot.tick.to_string());
                    json.push_str(",\"time\":");
                    json.push_str(&snapshot.time.to_string());
                    json.push_str(",\"entities\":[");

                    for (i, e) in snapshot.entities.iter().enumerate() {
                        if i > 0 {
                            json.push(',');
                        }
                        json.push_str(&format!(
                            "{{\"id\":{},\"pos\":[{},{},{}],\"health\":{}}}",
                            e.id, e.position[0], e.position[1], e.position[2], e.health
                        ));
                    }

                    json.push_str("]}");
                    std_black_box(json.len())
                });
            },
        );
    }

    // Test 2: Plan serialization
    group.bench_function("plan_serialize_100", |bencher| {
        let plans: Vec<PlanIntent> = (0..100).map(|_| generate_plan()).collect();

        bencher.iter(|| {
            let serialized: Vec<String> = plans
                .iter()
                .map(|p| {
                    format!(
                        "{{\"plan_id\":{},\"agent_id\":{},\"actions\":{},\"priority\":{}}}",
                        p.plan_id,
                        p.agent_id,
                        p.actions.len(),
                        p.priority
                    )
                })
                .collect();

            let total_len: usize = serialized.iter().map(|s| s.len()).sum();
            std_black_box(total_len)
        });
    });

    // Test 3: Binary serialization (compact format)
    group.bench_function("binary_serialize_1000", |bencher| {
        let snapshot = generate_snapshot(1000);

        bencher.iter(|| {
            let mut buffer: Vec<u8> = Vec::with_capacity(snapshot.entities.len() * 32);

            // Header
            buffer.extend_from_slice(&snapshot.tick.to_le_bytes());
            buffer.extend_from_slice(&snapshot.time.to_le_bytes());
            buffer.extend_from_slice(&(snapshot.entities.len() as u32).to_le_bytes());

            // Entities
            for e in &snapshot.entities {
                buffer.extend_from_slice(&e.id.to_le_bytes());
                buffer.extend_from_slice(&e.position[0].to_le_bytes());
                buffer.extend_from_slice(&e.position[1].to_le_bytes());
                buffer.extend_from_slice(&e.position[2].to_le_bytes());
                buffer.extend_from_slice(&e.health.to_le_bytes());
            }

            std_black_box(buffer.len())
        });
    });

    // Test 4: Delta serialization
    group.bench_function("delta_serialize_1000", |bencher| {
        let old_snapshot = generate_snapshot(1000);
        let mut new_snapshot = old_snapshot.clone();

        // Modify 10% of entities
        for i in (0..new_snapshot.entities.len()).step_by(10) {
            new_snapshot.entities[i].position[0] += 1.0;
            new_snapshot.entities[i].health -= 10.0;
        }

        bencher.iter(|| {
            // Find changed entities
            let changes: Vec<(u64, &EntityState)> = new_snapshot
                .entities
                .iter()
                .zip(old_snapshot.entities.iter())
                .filter(|(new, old)| {
                    new.position != old.position || (new.health - old.health).abs() > 0.01
                })
                .map(|(new, _)| (new.id, new))
                .collect();

            std_black_box(changes.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 2: DESERIALIZATION
// ============================================================================

fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_adversarial/deserialization");

    // Test 1: Parse entity data
    group.bench_function("entity_parse_500", |bencher| {
        let data: Vec<String> = (0..500)
            .map(|i| {
                format!(
                    "{{\"id\":{},\"pos\":[{},{},{}],\"health\":{}}}",
                    i,
                    i % 100,
                    (i / 100) % 10,
                    i / 1000,
                    100 - i % 50
                )
            })
            .collect();

        bencher.iter(|| {
            let entities: Vec<EntityState> = data
                .iter()
                .filter_map(|s| {
                    // Simple parsing (not real JSON parser)
                    let id = s
                        .split("\"id\":")
                        .nth(1)?
                        .split(',')
                        .next()?
                        .parse::<u64>()
                        .ok()?;

                    let pos_str = s.split("\"pos\":[").nth(1)?.split(']').next()?;
                    let pos_parts: Vec<f32> = pos_str
                        .split(',')
                        .filter_map(|p| p.parse::<f32>().ok())
                        .collect();

                    if pos_parts.len() < 3 {
                        return None;
                    }

                    let health = s
                        .split("\"health\":")
                        .nth(1)?
                        .split('}')
                        .next()?
                        .parse::<f32>()
                        .ok()?;

                    Some(EntityState {
                        id,
                        position: [pos_parts[0], pos_parts[1], pos_parts[2]],
                        rotation: [0.0, 0.0, 0.0, 1.0],
                        health,
                        entity_type: EntityType::NPC,
                    })
                })
                .collect();

            std_black_box(entities.len())
        });
    });

    // Test 2: Binary deserialization
    group.bench_function("binary_deserialize_1000", |bencher| {
        // Create binary buffer
        let entity_count = 1000u32;
        let mut buffer: Vec<u8> = Vec::new();

        buffer.extend_from_slice(&12345u64.to_le_bytes()); // tick
        buffer.extend_from_slice(&123.456f64.to_le_bytes()); // time
        buffer.extend_from_slice(&entity_count.to_le_bytes());

        for i in 0..entity_count {
            buffer.extend_from_slice(&(i as u64).to_le_bytes());
            buffer.extend_from_slice(&(i as f32).to_le_bytes());
            buffer.extend_from_slice(&(i as f32 * 0.1).to_le_bytes());
            buffer.extend_from_slice(&0.0f32.to_le_bytes());
            buffer.extend_from_slice(&100.0f32.to_le_bytes());
        }

        bencher.iter(|| {
            let mut offset = 0;

            let tick = u64::from_le_bytes(buffer[offset..offset + 8].try_into().unwrap());
            offset += 8;

            let time = f64::from_le_bytes(buffer[offset..offset + 8].try_into().unwrap());
            offset += 8;

            let count = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
            offset += 4;

            let mut entities = Vec::with_capacity(count as usize);

            for _ in 0..count {
                let id = u64::from_le_bytes(buffer[offset..offset + 8].try_into().unwrap());
                offset += 8;

                let x = f32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
                offset += 4;
                let y = f32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
                offset += 4;
                let z = f32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
                offset += 4;
                let health = f32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
                offset += 4;

                entities.push(EntityState {
                    id,
                    position: [x, y, z],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    health,
                    entity_type: EntityType::NPC,
                });
            }

            std_black_box((tick, time, entities.len()))
        });
    });

    // Test 3: Message type detection
    group.bench_function("message_type_detection_1000", |bencher| {
        let messages: Vec<String> = (0..1000)
            .map(|i| match i % 5 {
                0 => "{\"type\":\"snapshot\",\"data\":{}}".to_string(),
                1 => "{\"type\":\"plan\",\"data\":{}}".to_string(),
                2 => "{\"type\":\"command\",\"cmd\":\"move\"}".to_string(),
                3 => "{\"type\":\"heartbeat\",\"ts\":12345}".to_string(),
                _ => "{\"type\":\"error\",\"msg\":\"test\"}".to_string(),
            })
            .collect();

        bencher.iter(|| {
            let types: Vec<&str> = messages
                .iter()
                .map(|m| {
                    if m.contains("\"type\":\"snapshot\"") {
                        "snapshot"
                    } else if m.contains("\"type\":\"plan\"") {
                        "plan"
                    } else if m.contains("\"type\":\"command\"") {
                        "command"
                    } else if m.contains("\"type\":\"heartbeat\"") {
                        "heartbeat"
                    } else {
                        "error"
                    }
                })
                .collect();

            let snapshot_count = types.iter().filter(|&&t| t == "snapshot").count();
            std_black_box(snapshot_count)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 3: MESSAGE HANDLING
// ============================================================================

fn bench_message_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_adversarial/message_handling");

    // Test 1: Message queue processing
    group.bench_function("queue_processing_5000", |bencher| {
        let messages: Vec<Message> = (0..5000)
            .map(|i| match i % 5 {
                0 => Message::Snapshot(generate_snapshot(10)),
                1 => Message::Plan(generate_plan()),
                2 => Message::Command("move".to_string()),
                3 => Message::Heartbeat(i as u64),
                _ => Message::Error("test".to_string()),
            })
            .collect();

        bencher.iter(|| {
            let mut snapshot_count = 0;
            let mut plan_count = 0;
            let mut command_count = 0;
            let mut heartbeat_count = 0;
            let mut error_count = 0;

            for msg in &messages {
                match msg {
                    Message::Snapshot(_) => snapshot_count += 1,
                    Message::Plan(_) => plan_count += 1,
                    Message::Command(_) => command_count += 1,
                    Message::Heartbeat(_) => heartbeat_count += 1,
                    Message::Error(_) => error_count += 1,
                }
            }

            std_black_box((snapshot_count, plan_count, command_count))
        });
    });

    // Test 2: Priority queue
    group.bench_function("priority_queue_1000", |bencher| {
        let mut messages: Vec<(u32, Message)> = (0..1000)
            .map(|i| {
                let priority = match i % 5 {
                    0 => 0, // Highest
                    1 => 1,
                    2 => 2,
                    3 => 3,
                    _ => 4, // Lowest
                };
                (priority, Message::Heartbeat(i as u64))
            })
            .collect();

        bencher.iter(|| {
            messages.sort_by_key(|(p, _)| *p);

            // Process top 100
            let processed: Vec<_> = messages.iter().take(100).collect();
            std_black_box(processed.len())
        });
    });

    // Test 3: Message batching
    group.bench_function("message_batching_2000", |bencher| {
        let messages: Vec<Message> = (0..2000).map(|i| Message::Heartbeat(i as u64)).collect();

        let batch_size = 50;

        bencher.iter(|| {
            let batches: Vec<Vec<&Message>> = messages
                .chunks(batch_size)
                .map(|c| c.iter().collect())
                .collect();

            std_black_box(batches.len())
        });
    });

    // Test 4: Duplicate detection
    group.bench_function("duplicate_detection_1000", |bencher| {
        let messages: Vec<(u64, String)> = (0..1000)
            .map(|i| (i % 100, format!("msg_{}", i % 100))) // 10 duplicates of each
            .collect();

        bencher.iter(|| {
            let mut seen: HashMap<u64, usize> = HashMap::new();
            let mut duplicates = 0;

            for (id, _) in &messages {
                let entry = seen.entry(*id).or_insert(0);
                if *entry > 0 {
                    duplicates += 1;
                }
                *entry += 1;
            }

            std_black_box(duplicates)
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 4: CONNECTION MANAGEMENT
// ============================================================================

fn bench_connection_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_adversarial/connection_management");

    // Test 1: Client tracking
    group.bench_function("client_tracking_500", |bencher| {
        let clients: Vec<(u64, String, u64)> = (0..500)
            .map(|i| (i as u64, format!("client_{}", i), i as u64 * 1000))
            .collect();

        bencher.iter(|| {
            let mut client_map: HashMap<u64, (String, u64)> = HashMap::new();

            for (id, name, last_seen) in &clients {
                client_map.insert(*id, (name.clone(), *last_seen));
            }

            // Find stale connections (> 30s)
            let stale: Vec<u64> = client_map
                .iter()
                .filter(|(_, (_, last_seen))| *last_seen < 470_000) // threshold
                .map(|(id, _)| *id)
                .collect();

            std_black_box(stale.len())
        });
    });

    // Test 2: Room/channel management
    group.bench_function("channel_management_100", |bencher| {
        let channels: Vec<(String, Vec<u64>)> = (0..100)
            .map(|i| {
                let members: Vec<u64> = (0..20).map(|j| (i * 20 + j) as u64).collect();
                (format!("channel_{}", i), members)
            })
            .collect();

        bencher.iter(|| {
            let mut channel_map: HashMap<String, Vec<u64>> = HashMap::new();

            for (name, members) in &channels {
                channel_map.insert(name.clone(), members.clone());
            }

            // Find channel for client
            let target_client = 150u64;
            let client_channels: Vec<&String> = channel_map
                .iter()
                .filter(|(_, members)| members.contains(&target_client))
                .map(|(name, _)| name)
                .collect();

            std_black_box(client_channels.len())
        });
    });

    // Test 3: Broadcast optimization
    group.bench_function("broadcast_optimization_1000_clients", |bencher| {
        let clients: Vec<u64> = (0..1000).collect();
        let message = generate_snapshot(10);

        bencher.iter(|| {
            // Simulate broadcast - would normally send to all clients
            let recipients: Vec<u64> = clients
                .iter()
                .filter(|&&id| id % 2 == 0) // Even IDs only (simulate filtering)
                .copied()
                .collect();

            std_black_box(recipients.len())
        });
    });

    // Test 4: Reconnection handling
    group.bench_function("reconnection_handling_100", |bencher| {
        let disconnected: Vec<(u64, u64, u64)> = (0..100)
            .map(|i| (i as u64, i as u64 * 1000, 5000u64)) // id, disconnect_time, timeout
            .collect();

        bencher.iter(|| {
            let current_time = 3000u64;

            let can_reconnect: Vec<u64> = disconnected
                .iter()
                .filter(|(_, disconnect_time, timeout)| current_time - disconnect_time < *timeout)
                .map(|(id, _, _)| *id)
                .collect();

            let expired: Vec<u64> = disconnected
                .iter()
                .filter(|(_, disconnect_time, timeout)| current_time - disconnect_time >= *timeout)
                .map(|(id, _, _)| *id)
                .collect();

            std_black_box((can_reconnect.len(), expired.len()))
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 5: COMPRESSION
// ============================================================================

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_adversarial/compression");

    // Test 1: Simple run-length encoding
    group.bench_function("rle_encode_10000", |bencher| {
        let data: Vec<u8> = (0..10000)
            .map(|i| ((i / 100) % 256) as u8) // Runs of 100
            .collect();

        bencher.iter(|| {
            let mut encoded: Vec<(u8, u16)> = Vec::new();
            let mut current = data[0];
            let mut count = 1u16;

            for &byte in data.iter().skip(1) {
                if byte == current && count < u16::MAX {
                    count += 1;
                } else {
                    encoded.push((current, count));
                    current = byte;
                    count = 1;
                }
            }
            encoded.push((current, count));

            std_black_box(encoded.len())
        });
    });

    // Test 2: Delta compression
    group.bench_function("delta_compress_5000", |bencher| {
        let values: Vec<i32> = (0..5000).map(|i| i * 10 + (i % 10) as i32).collect();

        bencher.iter(|| {
            let deltas: Vec<i32> = values.windows(2).map(|w| w[1] - w[0]).collect();

            // Count bytes needed (varint simulation)
            let bytes: usize = deltas
                .iter()
                .map(|&d| {
                    let abs = d.unsigned_abs();
                    if abs < 128 {
                        1
                    } else if abs < 16384 {
                        2
                    } else {
                        4
                    }
                })
                .sum();

            std_black_box(bytes)
        });
    });

    // Test 3: Dictionary compression
    group.bench_function("dictionary_compress_1000", |bencher| {
        let strings: Vec<String> = (0..1000)
            .map(|i| match i % 10 {
                0 => "player".to_string(),
                1 => "enemy".to_string(),
                2 => "npc".to_string(),
                3 => "item".to_string(),
                4 => "projectile".to_string(),
                5 => "effect".to_string(),
                6 => "trigger".to_string(),
                7 => "spawn".to_string(),
                8 => "death".to_string(),
                _ => "unknown".to_string(),
            })
            .collect();

        bencher.iter(|| {
            let mut dictionary: HashMap<&str, u8> = HashMap::new();
            let mut compressed: Vec<u8> = Vec::new();

            for s in &strings {
                let idx = if let Some(&idx) = dictionary.get(s.as_str()) {
                    idx
                } else {
                    let idx = dictionary.len() as u8;
                    dictionary.insert(s.as_str(), idx);
                    idx
                };
                compressed.push(idx);
            }

            std_black_box((dictionary.len(), compressed.len()))
        });
    });

    // Test 4: Position quantization
    group.bench_function("position_quantize_5000", |bencher| {
        let positions: Vec<[f32; 3]> = (0..5000)
            .map(|i| {
                [
                    (i % 1000) as f32 * 0.1,
                    ((i / 1000) % 100) as f32 * 0.1,
                    (i / 100000) as f32 * 0.1,
                ]
            })
            .collect();

        let scale = 100.0f32; // 1cm precision in 100m range

        bencher.iter(|| {
            let quantized: Vec<[i16; 3]> = positions
                .iter()
                .map(|p| {
                    [
                        (p[0] * scale) as i16,
                        (p[1] * scale) as i16,
                        (p[2] * scale) as i16,
                    ]
                })
                .collect();

            std_black_box(quantized.len())
        });
    });

    group.finish();
}

// ============================================================================
// CATEGORY 6: FLOW CONTROL
// ============================================================================

fn bench_flow_control(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_adversarial/flow_control");

    // Test 1: Rate limiting
    group.bench_function("rate_limiting_10000", |bencher| {
        let requests: Vec<(u64, u64)> = (0..10000)
            .map(|i| (i % 100, i as u64)) // client_id, timestamp
            .collect();

        let rate_limit = 100u32; // requests per second
        let window_ms = 1000u64;

        bencher.iter(|| {
            let mut client_requests: HashMap<u64, Vec<u64>> = HashMap::new();
            let mut allowed = 0;
            let mut denied = 0;

            for (client_id, timestamp) in &requests {
                let requests = client_requests.entry(*client_id).or_default();

                // Remove old requests
                requests.retain(|&t| *timestamp - t < window_ms);

                if requests.len() < rate_limit as usize {
                    requests.push(*timestamp);
                    allowed += 1;
                } else {
                    denied += 1;
                }
            }

            std_black_box((allowed, denied))
        });
    });

    // Test 2: Backpressure handling
    group.bench_function("backpressure_handling", |bencher| {
        let mut buffer_sizes: Vec<usize> = vec![0; 100]; // 100 clients
        let high_watermark = 1000usize;
        let low_watermark = 500usize;

        bencher.iter(|| {
            let mut paused: Vec<usize> = Vec::new();
            let mut resumed: Vec<usize> = Vec::new();

            for (i, size) in buffer_sizes.iter_mut().enumerate() {
                // Simulate receiving data
                *size += (i * 10) % 50;

                if *size > high_watermark {
                    paused.push(i);
                } else if *size < low_watermark {
                    resumed.push(i);
                }
            }

            std_black_box((paused.len(), resumed.len()))
        });
    });

    // Test 3: Congestion window adjustment
    group.bench_function("congestion_window_1000", |bencher| {
        let ack_times: Vec<(u64, u64, bool)> = (0..1000)
            .map(|i| (i as u64, i as u64 + (i % 50) as u64, i % 20 != 0)) // seq, ack_time, success
            .collect();

        bencher.iter(|| {
            let mut cwnd = 10.0f64; // congestion window
            let mut ssthresh = 64.0f64; // slow start threshold

            for (_, _, success) in &ack_times {
                if *success {
                    if cwnd < ssthresh {
                        // Slow start: exponential growth
                        cwnd += 1.0;
                    } else {
                        // Congestion avoidance: linear growth
                        cwnd += 1.0 / cwnd;
                    }
                } else {
                    // Packet loss: reduce window
                    ssthresh = cwnd / 2.0;
                    cwnd = ssthresh.max(1.0);
                }
            }

            std_black_box(cwnd as usize)
        });
    });

    // Test 4: Priority scheduling
    group.bench_function("priority_scheduling_500", |bencher| {
        let packets: Vec<(u32, usize, bool)> = (0..500)
            .map(|i| {
                let priority = (i % 4) as u32; // 0=highest
                let size = 100 + (i % 500);
                let reliable = i % 3 == 0;
                (priority, size, reliable)
            })
            .collect();

        let bandwidth_budget = 10000usize;

        bencher.iter(|| {
            let mut sorted = packets.clone();
            sorted.sort_by_key(|(p, _, reliable)| (*p, !*reliable));

            let mut scheduled: Vec<(u32, usize)> = Vec::new();
            let mut used = 0usize;

            for (priority, size, _) in sorted {
                if used + size <= bandwidth_budget {
                    scheduled.push((priority, size));
                    used += size;
                }
            }

            std_black_box(scheduled.len())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_serialization,
    bench_deserialization,
    bench_message_handling,
    bench_connection_management,
    bench_compression,
    bench_flow_control,
);

criterion_main!(benches);
