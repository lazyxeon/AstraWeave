//! Mutation-resistant comprehensive tests for aw-net-proto.

use aw_net_proto::*;

// ═══════════════════════════════════════════════════════════════════════════
// PROTOCOL_VERSION constant
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn protocol_version_is_one() {
    assert_eq!(PROTOCOL_VERSION, 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// SessionKey
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn session_key_random_is_32_bytes() {
    let key = SessionKey::random();
    assert_eq!(key.0.len(), 32);
}

#[test]
fn session_key_random_not_all_zeros() {
    let key = SessionKey::random();
    assert!(
        key.0.iter().any(|&b| b != 0),
        "random key should not be all zeros"
    );
}

#[test]
fn session_key_two_randoms_differ() {
    let k1 = SessionKey::random();
    let k2 = SessionKey::random();
    assert_ne!(k1.0, k2.0, "two random keys should differ");
}

#[test]
fn session_key_clone() {
    let k1 = SessionKey::random();
    let k2 = k1.clone();
    assert_eq!(k1.0, k2.0);
}

#[test]
fn session_key_debug() {
    let key = SessionKey::random();
    let dbg = format!("{key:?}");
    assert!(dbg.contains("SessionKey"));
}

#[test]
fn session_key_json_roundtrip() {
    let key = SessionKey::random();
    let json = serde_json::to_string(&key).unwrap();
    let back: SessionKey = serde_json::from_str(&json).unwrap();
    assert_eq!(key.0, back.0);
}

// ═══════════════════════════════════════════════════════════════════════════
// ClientToServer variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn client_hello_protocol_field() {
    let msg = ClientToServer::Hello {
        protocol: PROTOCOL_VERSION,
    };
    if let ClientToServer::Hello { protocol } = msg {
        assert_eq!(protocol, 1);
    } else {
        panic!("expected Hello variant");
    }
}

#[test]
fn client_find_or_create_fields() {
    let msg = ClientToServer::FindOrCreate {
        region: "us-east".into(),
        game_mode: "deathmatch".into(),
        party_size: 4,
    };
    if let ClientToServer::FindOrCreate {
        region,
        game_mode,
        party_size,
    } = msg
    {
        assert_eq!(region, "us-east");
        assert_eq!(game_mode, "deathmatch");
        assert_eq!(party_size, 4);
    } else {
        panic!("expected FindOrCreate");
    }
}

#[test]
fn client_join_room_fields() {
    let msg = ClientToServer::JoinRoom {
        room_id: "ABC12345".into(),
        display_name: "Player1".into(),
    };
    if let ClientToServer::JoinRoom {
        room_id,
        display_name,
    } = msg
    {
        assert_eq!(room_id, "ABC12345");
        assert_eq!(display_name, "Player1");
    } else {
        panic!("expected JoinRoom");
    }
}

#[test]
fn client_input_frame_fields() {
    let sig = [1u8; 16];
    let msg = ClientToServer::InputFrame {
        seq: 42,
        tick_ms: 16667,
        input_blob: vec![1, 2, 3],
        sig,
    };
    if let ClientToServer::InputFrame {
        seq,
        tick_ms,
        input_blob,
        sig: s,
    } = msg
    {
        assert_eq!(seq, 42);
        assert_eq!(tick_ms, 16667);
        assert_eq!(input_blob, vec![1, 2, 3]);
        assert_eq!(s, [1u8; 16]);
    } else {
        panic!("expected InputFrame");
    }
}

#[test]
fn client_ping_nano() {
    let msg = ClientToServer::Ping { nano: 123456789 };
    if let ClientToServer::Ping { nano } = msg {
        assert_eq!(nano, 123456789);
    } else {
        panic!("expected Ping");
    }
}

#[test]
fn client_ack_last_snapshot_id() {
    let msg = ClientToServer::Ack {
        last_snapshot_id: 99,
    };
    if let ClientToServer::Ack { last_snapshot_id } = msg {
        assert_eq!(last_snapshot_id, 99);
    } else {
        panic!("expected Ack");
    }
}

#[test]
fn client_to_server_clone() {
    let msg = ClientToServer::Hello { protocol: 1 };
    let msg2 = msg.clone();
    if let ClientToServer::Hello { protocol } = msg2 {
        assert_eq!(protocol, 1);
    }
}

#[test]
fn client_to_server_json_roundtrip_hello() {
    let msg = ClientToServer::Hello { protocol: 1 };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ClientToServer = serde_json::from_str(&json).unwrap();
    if let ClientToServer::Hello { protocol } = back {
        assert_eq!(protocol, 1);
    } else {
        panic!("deserialized wrong variant");
    }
}

#[test]
fn client_to_server_json_roundtrip_input_frame() {
    let msg = ClientToServer::InputFrame {
        seq: 10,
        tick_ms: 5000,
        input_blob: vec![4, 5, 6],
        sig: [7u8; 16],
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ClientToServer = serde_json::from_str(&json).unwrap();
    if let ClientToServer::InputFrame {
        seq,
        tick_ms,
        input_blob,
        sig,
    } = back
    {
        assert_eq!(seq, 10);
        assert_eq!(tick_ms, 5000);
        assert_eq!(input_blob, vec![4, 5, 6]);
        assert_eq!(sig, [7u8; 16]);
    } else {
        panic!("wrong variant");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ServerToClient variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn server_hello_ack_protocol() {
    let msg = ServerToClient::HelloAck { protocol: 1 };
    if let ServerToClient::HelloAck { protocol } = msg {
        assert_eq!(protocol, 1);
    } else {
        panic!("expected HelloAck");
    }
}

#[test]
fn server_match_result_fields() {
    let msg = ServerToClient::MatchResult {
        room_id: "ROOM42".into(),
        session_key_hint: [9u8; 8],
    };
    if let ServerToClient::MatchResult {
        room_id,
        session_key_hint,
    } = msg
    {
        assert_eq!(room_id, "ROOM42");
        assert_eq!(session_key_hint, [9u8; 8]);
    } else {
        panic!("expected MatchResult");
    }
}

#[test]
fn server_join_accepted_fields() {
    let msg = ServerToClient::JoinAccepted {
        room_id: "R1".into(),
        player_id: "P1".into(),
        session_key_hint: [0u8; 8],
        tick_hz: 60,
    };
    if let ServerToClient::JoinAccepted {
        room_id,
        player_id,
        session_key_hint,
        tick_hz,
    } = msg
    {
        assert_eq!(room_id, "R1");
        assert_eq!(player_id, "P1");
        assert_eq!(session_key_hint, [0u8; 8]);
        assert_eq!(tick_hz, 60);
    } else {
        panic!("expected JoinAccepted");
    }
}

#[test]
fn server_snapshot_fields() {
    let msg = ServerToClient::Snapshot {
        id: 5,
        server_tick: 1000,
        base_id: Some(4),
        compressed: true,
        payload: vec![10, 20, 30],
    };
    if let ServerToClient::Snapshot {
        id,
        server_tick,
        base_id,
        compressed,
        payload,
    } = msg
    {
        assert_eq!(id, 5);
        assert_eq!(server_tick, 1000);
        assert_eq!(base_id, Some(4));
        assert!(compressed);
        assert_eq!(payload, vec![10, 20, 30]);
    } else {
        panic!("expected Snapshot");
    }
}

#[test]
fn server_snapshot_no_base() {
    let msg = ServerToClient::Snapshot {
        id: 1,
        server_tick: 0,
        base_id: None,
        compressed: false,
        payload: vec![],
    };
    if let ServerToClient::Snapshot {
        base_id,
        compressed,
        payload,
        ..
    } = msg
    {
        assert!(base_id.is_none());
        assert!(!compressed);
        assert!(payload.is_empty());
    }
}

#[test]
fn server_reconcile_fields() {
    let msg = ServerToClient::Reconcile {
        input_seq_ack: 42,
        corrected_state_hash: 999,
    };
    if let ServerToClient::Reconcile {
        input_seq_ack,
        corrected_state_hash,
    } = msg
    {
        assert_eq!(input_seq_ack, 42);
        assert_eq!(corrected_state_hash, 999);
    } else {
        panic!("expected Reconcile");
    }
}

#[test]
fn server_pong_nano() {
    let msg = ServerToClient::Pong { nano: 888 };
    if let ServerToClient::Pong { nano } = msg {
        assert_eq!(nano, 888);
    } else {
        panic!("expected Pong");
    }
}

#[test]
fn server_rate_limited_is_unit() {
    let msg = ServerToClient::RateLimited;
    assert!(matches!(msg, ServerToClient::RateLimited));
}

#[test]
fn server_protocol_error_msg() {
    let msg = ServerToClient::ProtocolError {
        msg: "bad frame".into(),
    };
    if let ServerToClient::ProtocolError { msg } = msg {
        assert_eq!(msg, "bad frame");
    } else {
        panic!("expected ProtocolError");
    }
}

#[test]
fn server_to_client_clone() {
    let msg = ServerToClient::Pong { nano: 42 };
    let msg2 = msg.clone();
    assert!(matches!(msg2, ServerToClient::Pong { nano: 42 }));
}

#[test]
fn server_to_client_json_roundtrip() {
    let msg = ServerToClient::JoinAccepted {
        room_id: "R".into(),
        player_id: "P".into(),
        session_key_hint: [1, 2, 3, 4, 5, 6, 7, 8],
        tick_hz: 30,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: ServerToClient = serde_json::from_str(&json).unwrap();
    if let ServerToClient::JoinAccepted {
        room_id, tick_hz, ..
    } = back
    {
        assert_eq!(room_id, "R");
        assert_eq!(tick_hz, 30);
    } else {
        panic!("wrong variant");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// WireError
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wire_error_protocol_mismatch_display() {
    let err = WireError::ProtocolMismatch {
        client: 1,
        server: 2,
    };
    let msg = format!("{err}");
    assert!(msg.contains("protocol mismatch"), "got: {msg}");
    assert!(msg.contains("1"), "should contain client version");
    assert!(msg.contains("2"), "should contain server version");
}

#[test]
fn wire_error_decode_display() {
    let err = WireError::Decode("corrupt data".into());
    let msg = format!("{err}");
    assert!(msg.contains("decode error"), "got: {msg}");
    assert!(msg.contains("corrupt data"));
}

#[test]
fn wire_error_debug() {
    let err = WireError::ProtocolMismatch {
        client: 3,
        server: 5,
    };
    let dbg = format!("{err:?}");
    assert!(dbg.contains("ProtocolMismatch"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Codec enum
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn codec_postcard_lz4_copy() {
    let c = Codec::PostcardLz4;
    let c2 = c;
    assert!(matches!(c2, Codec::PostcardLz4));
}

#[test]
fn codec_bincode_copy() {
    let c = Codec::Bincode;
    let c2 = c;
    assert!(matches!(c2, Codec::Bincode));
}

// ═══════════════════════════════════════════════════════════════════════════
// encode_msg / decode_msg roundtrips
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn encode_decode_postcard_lz4_client_hello() {
    let msg = ClientToServer::Hello { protocol: 1 };
    let bytes = encode_msg(Codec::PostcardLz4, &msg);
    assert!(!bytes.is_empty());
    let back: ClientToServer = decode_msg(Codec::PostcardLz4, &bytes).unwrap();
    if let ClientToServer::Hello { protocol } = back {
        assert_eq!(protocol, 1);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn encode_decode_bincode_client_hello() {
    let msg = ClientToServer::Hello { protocol: 1 };
    let bytes = encode_msg(Codec::Bincode, &msg);
    assert!(!bytes.is_empty());
    let back: ClientToServer = decode_msg(Codec::Bincode, &bytes).unwrap();
    if let ClientToServer::Hello { protocol } = back {
        assert_eq!(protocol, 1);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn encode_decode_postcard_server_snapshot() {
    let msg = ServerToClient::Snapshot {
        id: 10,
        server_tick: 500,
        base_id: Some(9),
        compressed: true,
        payload: vec![1, 2, 3, 4, 5],
    };
    let bytes = encode_msg(Codec::PostcardLz4, &msg);
    let back: ServerToClient = decode_msg(Codec::PostcardLz4, &bytes).unwrap();
    if let ServerToClient::Snapshot {
        id,
        server_tick,
        base_id,
        compressed,
        payload,
    } = back
    {
        assert_eq!(id, 10);
        assert_eq!(server_tick, 500);
        assert_eq!(base_id, Some(9));
        assert!(compressed);
        assert_eq!(payload, vec![1, 2, 3, 4, 5]);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn encode_decode_bincode_server_reconcile() {
    let msg = ServerToClient::Reconcile {
        input_seq_ack: 100,
        corrected_state_hash: 0xDEADBEEF,
    };
    let bytes = encode_msg(Codec::Bincode, &msg);
    let back: ServerToClient = decode_msg(Codec::Bincode, &bytes).unwrap();
    if let ServerToClient::Reconcile {
        input_seq_ack,
        corrected_state_hash,
    } = back
    {
        assert_eq!(input_seq_ack, 100);
        assert_eq!(corrected_state_hash, 0xDEADBEEF);
    }
}

#[test]
fn decode_garbage_postcard_fails() {
    let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB];
    let result: Result<ClientToServer, WireError> = decode_msg(Codec::PostcardLz4, &garbage);
    assert!(result.is_err());
}

#[test]
fn decode_garbage_bincode_fails() {
    let garbage = vec![0xFF, 0xFE, 0xFD];
    let result: Result<ClientToServer, WireError> = decode_msg(Codec::Bincode, &garbage);
    assert!(result.is_err());
}

#[test]
fn decode_empty_postcard_fails() {
    let result: Result<ClientToServer, WireError> = decode_msg(Codec::PostcardLz4, &[]);
    assert!(result.is_err());
}

#[test]
fn postcard_lz4_more_compact_for_large_payload() {
    let msg = ServerToClient::Snapshot {
        id: 1,
        server_tick: 1,
        base_id: None,
        compressed: false,
        payload: vec![42; 1000], // repeated data compresses well
    };
    let postcard_bytes = encode_msg(Codec::PostcardLz4, &msg);
    let bincode_bytes = encode_msg(Codec::Bincode, &msg);
    // LZ4 compression should make postcard smaller for repeated data
    assert!(
        postcard_bytes.len() < bincode_bytes.len(),
        "postcard+lz4 ({}) should be smaller than bincode ({}) for compressible data",
        postcard_bytes.len(),
        bincode_bytes.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// sign16 function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn sign16_returns_16_bytes() {
    let sig = sign16(b"hello", &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(sig.len(), 16);
}

#[test]
fn sign16_deterministic() {
    let key = [1u8; 8];
    let s1 = sign16(b"data", &key);
    let s2 = sign16(b"data", &key);
    assert_eq!(s1, s2, "same input+key must produce same signature");
}

#[test]
fn sign16_different_data_different_sig() {
    let key = [1u8; 8];
    let s1 = sign16(b"data1", &key);
    let s2 = sign16(b"data2", &key);
    assert_ne!(s1, s2);
}

#[test]
fn sign16_different_key_different_sig() {
    let s1 = sign16(b"data", &[1u8; 8]);
    let s2 = sign16(b"data", &[2u8; 8]);
    assert_ne!(s1, s2);
}

#[test]
fn sign16_empty_input() {
    let sig = sign16(b"", &[0u8; 8]);
    assert_eq!(sig.len(), 16);
    // Empty input should still produce a valid signature
}

#[test]
fn sign16_not_all_zeros() {
    let sig = sign16(b"test data for signing", &[5u8; 8]);
    assert!(
        sig.iter().any(|&b| b != 0),
        "signature should not be all zeros"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// new_room_id function
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn new_room_id_length_8() {
    let id = new_room_id();
    assert_eq!(id.len(), 8);
}

#[test]
fn new_room_id_alphanumeric() {
    let id = new_room_id();
    assert!(
        id.chars().all(|c| c.is_ascii_alphanumeric()),
        "room id must be alphanumeric: {id}"
    );
}

#[test]
fn new_room_id_two_differ() {
    let id1 = new_room_id();
    let id2 = new_room_id();
    assert_ne!(id1, id2, "two room ids should differ");
}

// ═══════════════════════════════════════════════════════════════════════════
// Boundary & serialization stress
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn client_input_frame_large_blob() {
    let blob = vec![0xAB; 10000];
    let msg = ClientToServer::InputFrame {
        seq: u32::MAX,
        tick_ms: u64::MAX,
        input_blob: blob.clone(),
        sig: [0xFF; 16],
    };
    let bytes = encode_msg(Codec::PostcardLz4, &msg);
    let back: ClientToServer = decode_msg(Codec::PostcardLz4, &bytes).unwrap();
    if let ClientToServer::InputFrame {
        seq,
        tick_ms,
        input_blob,
        sig,
    } = back
    {
        assert_eq!(seq, u32::MAX);
        assert_eq!(tick_ms, u64::MAX);
        assert_eq!(input_blob.len(), 10000);
        assert_eq!(sig, [0xFF; 16]);
    }
}

#[test]
fn server_snapshot_large_payload() {
    let payload = vec![0xCD; 50000];
    let msg = ServerToClient::Snapshot {
        id: u32::MAX,
        server_tick: u64::MAX,
        base_id: Some(u32::MAX - 1),
        compressed: true,
        payload: payload.clone(),
    };
    let bytes = encode_msg(Codec::Bincode, &msg);
    let back: ServerToClient = decode_msg(Codec::Bincode, &bytes).unwrap();
    if let ServerToClient::Snapshot {
        id,
        server_tick,
        payload: p,
        ..
    } = back
    {
        assert_eq!(id, u32::MAX);
        assert_eq!(server_tick, u64::MAX);
        assert_eq!(p.len(), 50000);
    }
}

#[test]
fn roundtrip_all_client_variants() {
    let variants: Vec<ClientToServer> = vec![
        ClientToServer::Hello { protocol: 1 },
        ClientToServer::FindOrCreate {
            region: "r".into(),
            game_mode: "m".into(),
            party_size: 1,
        },
        ClientToServer::JoinRoom {
            room_id: "id".into(),
            display_name: "n".into(),
        },
        ClientToServer::InputFrame {
            seq: 0,
            tick_ms: 0,
            input_blob: vec![],
            sig: [0; 16],
        },
        ClientToServer::Ping { nano: 0 },
        ClientToServer::Ack {
            last_snapshot_id: 0,
        },
    ];
    for v in &variants {
        let bytes = encode_msg(Codec::PostcardLz4, v);
        let _back: ClientToServer = decode_msg(Codec::PostcardLz4, &bytes).unwrap();
    }
    assert_eq!(variants.len(), 6, "all 6 ClientToServer variants");
}

#[test]
fn roundtrip_all_server_variants() {
    let variants: Vec<ServerToClient> = vec![
        ServerToClient::HelloAck { protocol: 1 },
        ServerToClient::MatchResult {
            room_id: "r".into(),
            session_key_hint: [0; 8],
        },
        ServerToClient::JoinAccepted {
            room_id: "r".into(),
            player_id: "p".into(),
            session_key_hint: [0; 8],
            tick_hz: 60,
        },
        ServerToClient::Snapshot {
            id: 0,
            server_tick: 0,
            base_id: None,
            compressed: false,
            payload: vec![],
        },
        ServerToClient::Reconcile {
            input_seq_ack: 0,
            corrected_state_hash: 0,
        },
        ServerToClient::Pong { nano: 0 },
        ServerToClient::RateLimited,
        ServerToClient::ProtocolError { msg: "".into() },
    ];
    for v in &variants {
        let bytes = encode_msg(Codec::PostcardLz4, v);
        let _back: ServerToClient = decode_msg(Codec::PostcardLz4, &bytes).unwrap();
    }
    assert_eq!(variants.len(), 8, "all 8 ServerToClient variants");
}
