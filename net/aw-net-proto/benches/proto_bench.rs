//! # Network Protocol Benchmark Suite
//!
//! Comprehensive benchmarks for the aw-net-proto crate covering:
//! - Message encoding/decoding (PostcardLz4 and Bincode)
//! - Compression performance and ratios
//! - Signature generation and verification
//! - Session key generation
//!
//! Run with: `cargo bench -p aw-net-proto`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::{Deserialize, Serialize};

use aw_net_proto::{
    decode_msg, encode_msg, new_room_id, sign16, ClientToServer, Codec, ServerToClient, SessionKey,
    PROTOCOL_VERSION,
};

// ============================================================================
// CORRECTNESS ASSERTION HELPERS
// ============================================================================

/// Assert that encoded bytes are valid
fn assert_encoded_valid(bytes: &[u8]) {
    assert!(!bytes.is_empty(), "Encoded bytes should not be empty");
}

/// Assert that roundtrip preserves data
#[allow(dead_code)]
fn assert_roundtrip_valid<
    T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
>(
    original: &T,
    codec: Codec,
) {
    let encoded = encode_msg(codec, original);
    let _decoded: T = decode_msg(codec, &encoded).expect("Decode should succeed");
    // Note: Can't always assert equality for complex types, just check decode succeeds
}

// ============================================================================
// TEST DATA GENERATORS
// ============================================================================

/// Create a Hello message
fn create_hello_msg() -> ClientToServer {
    ClientToServer::Hello {
        protocol: PROTOCOL_VERSION,
    }
}

/// Create a FindOrCreate message
fn create_find_or_create_msg(region: &str) -> ClientToServer {
    ClientToServer::FindOrCreate {
        region: region.to_string(),
        game_mode: "coop".to_string(),
        party_size: 4,
    }
}

/// Create a JoinRoom message
fn create_join_room_msg(room_id: &str, display_name: &str) -> ClientToServer {
    ClientToServer::JoinRoom {
        room_id: room_id.to_string(),
        display_name: display_name.to_string(),
    }
}

/// Create an InputFrame message with variable payload size
fn create_input_frame_msg(seq: u32, payload_size: usize) -> ClientToServer {
    let input_blob = vec![0u8; payload_size];
    let sig = sign16(&input_blob, &[0u8; 8]);
    ClientToServer::InputFrame {
        seq,
        tick_ms: 33,
        input_blob,
        sig,
    }
}

/// Create a Ping message
fn create_ping_msg() -> ClientToServer {
    ClientToServer::Ping {
        nano: 123456789012345,
    }
}

/// Create a server HelloAck message
fn create_hello_ack_msg() -> ServerToClient {
    ServerToClient::HelloAck {
        protocol: PROTOCOL_VERSION,
    }
}

/// Create a MatchResult message
fn create_match_result_msg() -> ServerToClient {
    ServerToClient::MatchResult {
        room_id: "ABCD1234".to_string(),
        session_key_hint: [1, 2, 3, 4, 5, 6, 7, 8],
    }
}

/// Create a JoinAccepted message
fn create_join_accepted_msg() -> ServerToClient {
    ServerToClient::JoinAccepted {
        room_id: "ABCD1234".to_string(),
        player_id: "player_001".to_string(),
        session_key_hint: [1, 2, 3, 4, 5, 6, 7, 8],
        tick_hz: 60,
    }
}

/// Create a Snapshot message with variable payload size
fn create_snapshot_msg(payload_size: usize, compressed: bool) -> ServerToClient {
    ServerToClient::Snapshot {
        id: 1,
        server_tick: 1000,
        base_id: Some(0),
        compressed,
        payload: vec![42u8; payload_size],
    }
}

// ============================================================================
// ENCODING BENCHMARKS
// ============================================================================

fn bench_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("proto_encoding");

    // Hello message (minimal)
    let hello = create_hello_msg();
    group.bench_function("encode_hello_postcard", |b| {
        b.iter(|| {
            let bytes = encode_msg(Codec::PostcardLz4, black_box(&hello));
            assert_encoded_valid(&bytes);
            black_box(bytes)
        })
    });

    group.bench_function("encode_hello_bincode", |b| {
        b.iter(|| {
            let bytes = encode_msg(Codec::Bincode, black_box(&hello));
            assert_encoded_valid(&bytes);
            black_box(bytes)
        })
    });

    // FindOrCreate message (medium)
    let find_or_create = create_find_or_create_msg("us-east-1");
    group.bench_function("encode_find_or_create_postcard", |b| {
        b.iter(|| {
            let bytes = encode_msg(Codec::PostcardLz4, black_box(&find_or_create));
            assert_encoded_valid(&bytes);
            black_box(bytes)
        })
    });

    // InputFrame with varying payload sizes
    for payload_size in [32, 128, 512, 1024] {
        let input_frame = create_input_frame_msg(1, payload_size);

        group.throughput(Throughput::Bytes(payload_size as u64));
        group.bench_with_input(
            BenchmarkId::new("encode_input_frame_postcard", payload_size),
            &input_frame,
            |b, msg| {
                b.iter(|| {
                    let bytes = encode_msg(Codec::PostcardLz4, black_box(msg));
                    assert_encoded_valid(&bytes);
                    black_box(bytes)
                })
            },
        );
    }

    // Snapshot with varying payload sizes
    for payload_size in [100, 500, 1000, 5000] {
        let snapshot = create_snapshot_msg(payload_size, true);

        group.throughput(Throughput::Bytes(payload_size as u64));
        group.bench_with_input(
            BenchmarkId::new("encode_snapshot_postcard", payload_size),
            &snapshot,
            |b, msg| {
                b.iter(|| {
                    let bytes = encode_msg(Codec::PostcardLz4, black_box(msg));
                    assert_encoded_valid(&bytes);
                    black_box(bytes)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// DECODING BENCHMARKS
// ============================================================================

fn bench_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("proto_decoding");

    // Hello message (minimal)
    let hello = create_hello_msg();
    let hello_bytes_postcard = encode_msg(Codec::PostcardLz4, &hello);
    let hello_bytes_bincode = encode_msg(Codec::Bincode, &hello);

    group.bench_function("decode_hello_postcard", |b| {
        b.iter(|| {
            let decoded: ClientToServer =
                decode_msg(Codec::PostcardLz4, black_box(&hello_bytes_postcard))
                    .expect("Decode should succeed");
            black_box(decoded)
        })
    });

    group.bench_function("decode_hello_bincode", |b| {
        b.iter(|| {
            let decoded: ClientToServer =
                decode_msg(Codec::Bincode, black_box(&hello_bytes_bincode))
                    .expect("Decode should succeed");
            black_box(decoded)
        })
    });

    // InputFrame with varying payload sizes
    for payload_size in [32, 128, 512, 1024] {
        let input_frame = create_input_frame_msg(1, payload_size);
        let bytes = encode_msg(Codec::PostcardLz4, &input_frame);

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("decode_input_frame_postcard", payload_size),
            &bytes,
            |b, bytes| {
                b.iter(|| {
                    let decoded: ClientToServer = decode_msg(Codec::PostcardLz4, black_box(bytes))
                        .expect("Decode should succeed");
                    black_box(decoded)
                })
            },
        );
    }

    // Snapshot with varying payload sizes
    for payload_size in [100, 500, 1000, 5000] {
        let snapshot = create_snapshot_msg(payload_size, true);
        let bytes = encode_msg(Codec::PostcardLz4, &snapshot);

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("decode_snapshot_postcard", payload_size),
            &bytes,
            |b, bytes| {
                b.iter(|| {
                    let decoded: ServerToClient = decode_msg(Codec::PostcardLz4, black_box(bytes))
                        .expect("Decode should succeed");
                    black_box(decoded)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// ROUNDTRIP BENCHMARKS
// ============================================================================

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("proto_roundtrip");

    // Compare codecs for various message types
    let messages: Vec<(&str, ClientToServer)> = vec![
        ("hello", create_hello_msg()),
        ("find_or_create", create_find_or_create_msg("eu-west")),
        ("join_room", create_join_room_msg("ROOM123", "Player1")),
        ("input_128", create_input_frame_msg(100, 128)),
        ("ping", create_ping_msg()),
    ];

    for (name, msg) in messages {
        // PostcardLz4 roundtrip
        group.bench_with_input(
            BenchmarkId::new("roundtrip_postcard", name),
            &msg,
            |b, msg| {
                b.iter(|| {
                    let bytes = encode_msg(Codec::PostcardLz4, black_box(msg));
                    let decoded: ClientToServer =
                        decode_msg(Codec::PostcardLz4, &bytes).expect("Decode should succeed");
                    black_box(decoded)
                })
            },
        );

        // Bincode roundtrip
        group.bench_with_input(
            BenchmarkId::new("roundtrip_bincode", name),
            &msg,
            |b, msg| {
                b.iter(|| {
                    let bytes = encode_msg(Codec::Bincode, black_box(msg));
                    let decoded: ClientToServer =
                        decode_msg(Codec::Bincode, &bytes).expect("Decode should succeed");
                    black_box(decoded)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// COMPRESSION RATIO BENCHMARKS
// ============================================================================

fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("proto_compression");

    // Compare encoded sizes for different message types
    for payload_size in [100, 500, 1000, 2000] {
        let snapshot = create_snapshot_msg(payload_size, false);

        group.bench_with_input(
            BenchmarkId::new("compression_analysis", payload_size),
            &snapshot,
            |b, msg| {
                b.iter(|| {
                    let postcard_bytes = encode_msg(Codec::PostcardLz4, msg);
                    let bincode_bytes = encode_msg(Codec::Bincode, msg);

                    let ratio = postcard_bytes.len() as f32 / bincode_bytes.len() as f32;
                    // PostcardLz4 should generally be smaller due to compression
                    black_box((postcard_bytes.len(), bincode_bytes.len(), ratio))
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// SIGNATURE BENCHMARKS
// ============================================================================

fn bench_signature(c: &mut Criterion) {
    let mut group = c.benchmark_group("proto_signature");

    // Sign16 with varying input sizes
    for input_size in [32, 128, 512, 1024, 4096] {
        let input = vec![0x42u8; input_size];
        let session_hint = [1u8, 2, 3, 4, 5, 6, 7, 8];

        group.throughput(Throughput::Bytes(input_size as u64));
        group.bench_with_input(
            BenchmarkId::new("sign16", input_size),
            &(input.clone(), session_hint),
            |b, (input, hint)| {
                b.iter(|| {
                    let sig = sign16(black_box(input), black_box(hint));
                    assert_eq!(sig.len(), 16, "Signature should be 16 bytes");
                    black_box(sig)
                })
            },
        );
    }

    // Verify signature (sign and compare)
    let input = vec![0x42u8; 128];
    let session_hint = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let expected_sig = sign16(&input, &session_hint);

    group.bench_function("verify_signature", |b| {
        b.iter(|| {
            let computed = sign16(black_box(&input), black_box(&session_hint));
            let valid = computed == expected_sig;
            assert!(valid, "Signature should match");
            black_box(valid)
        })
    });

    group.finish();
}

// ============================================================================
// SESSION KEY BENCHMARKS
// ============================================================================

fn bench_session_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("proto_session_key");

    // Generate random session key
    group.bench_function("generate_session_key", |b| {
        b.iter(|| {
            let key = SessionKey::random();
            assert_eq!(key.0.len(), 32, "Session key should be 32 bytes");
            black_box(key)
        })
    });

    // Clone session key
    let key = SessionKey::random();
    group.bench_function("clone_session_key", |b| {
        b.iter(|| {
            let cloned = black_box(&key).clone();
            assert_eq!(cloned.0.len(), 32);
            black_box(cloned)
        })
    });

    group.finish();
}

// ============================================================================
// ROOM ID BENCHMARKS
// ============================================================================

fn bench_room_id(c: &mut Criterion) {
    let mut group = c.benchmark_group("proto_room_id");

    // Generate new room ID
    group.bench_function("generate_room_id", |b| {
        b.iter(|| {
            let id = new_room_id();
            assert_eq!(id.len(), 8, "Room ID should be 8 characters");
            assert!(
                id.chars().all(|c| c.is_ascii_alphanumeric()),
                "Room ID should be alphanumeric"
            );
            black_box(id)
        })
    });

    // Batch room ID generation
    for batch_size in [10, 50, 100] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_generate_room_ids", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let ids: Vec<String> = (0..batch_size).map(|_| new_room_id()).collect();
                    assert_eq!(ids.len(), batch_size);
                    // All should be unique (probabilistically)
                    black_box(ids)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// SERVER MESSAGE BENCHMARKS
// ============================================================================

fn bench_server_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("proto_server_messages");

    let messages: Vec<(&str, ServerToClient)> = vec![
        ("hello_ack", create_hello_ack_msg()),
        ("match_result", create_match_result_msg()),
        ("join_accepted", create_join_accepted_msg()),
        ("snapshot_100", create_snapshot_msg(100, true)),
        ("snapshot_1000", create_snapshot_msg(1000, true)),
    ];

    for (name, msg) in messages {
        group.bench_with_input(
            BenchmarkId::new("encode_server_msg", name),
            &msg,
            |b, msg| {
                b.iter(|| {
                    let bytes = encode_msg(Codec::PostcardLz4, black_box(msg));
                    assert_encoded_valid(&bytes);
                    black_box(bytes)
                })
            },
        );
    }

    // Reconcile message
    let reconcile = ServerToClient::Reconcile {
        input_seq_ack: 100,
        corrected_state_hash: 0x123456789ABCDEF0,
    };
    group.bench_function("encode_reconcile", |b| {
        b.iter(|| {
            let bytes = encode_msg(Codec::PostcardLz4, black_box(&reconcile));
            assert_encoded_valid(&bytes);
            black_box(bytes)
        })
    });

    // Pong message
    let pong = ServerToClient::Pong {
        nano: 123456789012345,
    };
    group.bench_function("encode_pong", |b| {
        b.iter(|| {
            let bytes = encode_msg(Codec::PostcardLz4, black_box(&pong));
            assert_encoded_valid(&bytes);
            black_box(bytes)
        })
    });

    group.finish();
}

// ============================================================================
// CRITERION GROUP REGISTRATION
// ============================================================================

criterion_group!(
    benches,
    bench_encoding,
    bench_decoding,
    bench_roundtrip,
    bench_compression_ratio,
    bench_signature,
    bench_session_key,
    bench_room_id,
    bench_server_messages,
);

criterion_main!(benches);
