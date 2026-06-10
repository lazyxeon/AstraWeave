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
    let sig = [1u8; 32];
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
        assert_eq!(s, [1u8; 32]);
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
        sig: [7u8; 32],
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
        assert_eq!(sig, [7u8; 32]);
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
    };
    if let ServerToClient::MatchResult { room_id } = msg {
        assert_eq!(room_id, "ROOM42");
    } else {
        panic!("expected MatchResult");
    }
}

#[test]
fn server_join_accepted_fields() {
    let msg = ServerToClient::JoinAccepted {
        room_id: "R1".into(),
        player_id: "P1".into(),
        tick_hz: 60,
    };
    if let ServerToClient::JoinAccepted {
        room_id,
        player_id,
        tick_hz,
    } = msg
    {
        assert_eq!(room_id, "R1");
        assert_eq!(player_id, "P1");
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
// HMAC-SHA256 canonical signing surface
// ═══════════════════════════════════════════════════════════════════════════

/// RFC 4231 §4.2 Test Case 1: key = 20 bytes of 0x0b, data = "Hi There".
/// Vector verified against https://www.rfc-editor.org/rfc/rfc4231 §4.2.
#[test]
fn hmac_sha256_rfc4231_test_case_1() {
    let key = [0x0bu8; 20];
    let tag = hmac_sha256(&key, b"Hi There");
    assert_eq!(
        hex::encode(tag),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

/// RFC 4231 §4.3 Test Case 2: key = "Jefe", data = "what do ya want for
/// nothing?". Vector verified against https://www.rfc-editor.org/rfc/rfc4231 §4.3.
#[test]
fn hmac_sha256_rfc4231_test_case_2() {
    let tag = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
    assert_eq!(
        hex::encode(tag),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

#[test]
fn sign_verify_roundtrip_succeeds() {
    let key = SigningKey::dev_default();
    let payload = input_frame_sig_payload(42, 16667, &[1, 2, 3, 4]);
    let tag = sign(&key, &payload);
    assert_eq!(tag.len(), SIG_LEN);
    assert!(verify(&key, &payload, &tag), "fresh signature must verify");
}

#[test]
fn verify_fails_on_tampered_payload() {
    let key = SigningKey::dev_default();
    let mut payload = input_frame_sig_payload(42, 16667, &[1, 2, 3, 4]);
    let tag = sign(&key, &payload);
    payload[0] ^= 0x01; // flip one bit
    assert!(
        !verify(&key, &payload, &tag),
        "tampered payload must not verify"
    );
}

#[test]
fn verify_fails_on_tampered_tag() {
    let key = SigningKey::dev_default();
    let payload = input_frame_sig_payload(42, 16667, &[1, 2, 3, 4]);
    let mut tag = sign(&key, &payload);
    tag[0] ^= 0x01; // flip one bit in the tag
    assert!(
        !verify(&key, &payload, &tag),
        "tampered tag must not verify"
    );
}

#[test]
fn verify_fails_on_wrong_key() {
    let key = SigningKey::dev_default();
    let other = SigningKey([0x5au8; 32]);
    let payload = input_frame_sig_payload(42, 16667, &[1, 2, 3, 4]);
    let tag = sign(&key, &payload);
    assert!(
        !verify(&other, &payload, &tag),
        "signature must not verify under a different key"
    );
}

#[test]
fn signing_key_from_hex_valid_roundtrip() {
    let original = SigningKey([0xa7u8; 32]);
    let hex_str = hex::encode(original.0);
    assert_eq!(hex_str.len(), 64);
    let parsed = SigningKey::from_hex(&hex_str).unwrap();
    assert_eq!(parsed.0, original.0);
}

#[test]
fn signing_key_from_hex_rejects_wrong_length() {
    let short = "a".repeat(63);
    let long = "a".repeat(65);
    assert!(SigningKey::from_hex(&short).is_err(), "63 chars must fail");
    assert!(SigningKey::from_hex(&long).is_err(), "65 chars must fail");
    assert!(SigningKey::from_hex("").is_err(), "empty must fail");
}

#[test]
fn signing_key_from_hex_rejects_non_hex_chars() {
    let mut s = "a".repeat(64);
    s.replace_range(10..11, "g"); // 'g' is not a hex digit, length stays 64
    assert!(SigningKey::from_hex(&s).is_err(), "non-hex chars must fail");
}

#[test]
fn signing_key_debug_is_redacted() {
    let key = SigningKey([0xabu8; 32]);
    let dbg = format!("{key:?}");
    assert!(dbg.contains("redacted"), "Debug must say redacted: {dbg}");
    let key_hex = hex::encode(key.0); // "abab...ab"
    assert!(
        !dbg.to_lowercase().contains(&key_hex),
        "Debug must not leak key hex"
    );
    assert!(
        !dbg.contains("171") && !dbg.contains("0xab") && !dbg.contains("ab, ab"),
        "Debug must not leak raw key bytes: {dbg}"
    );
}

#[test]
fn signing_key_dev_default_deterministic_32_bytes() {
    let k1 = SigningKey::dev_default();
    let k2 = SigningKey::dev_default();
    assert_eq!(k1.0, k2.0, "dev_default must be deterministic");
    assert_eq!(k1.0.len(), 32);
}

#[test]
fn input_frame_sig_payload_layout_pin() {
    let blob = [0xDEu8, 0xAD, 0xBE, 0xEF, 0x99];
    let payload = input_frame_sig_payload(0x01020304, 0x1112131415161718, &blob);
    assert_eq!(
        payload.len(),
        12 + blob.len(),
        "4 (seq) + 8 (tick_ms) + blob"
    );
    assert_eq!(&payload[0..4], &0x01020304u32.to_le_bytes(), "seq LE first");
    assert_eq!(
        &payload[4..12],
        &0x1112131415161718u64.to_le_bytes(),
        "tick_ms LE next"
    );
    assert_eq!(&payload[12..], &blob, "blob is the tail");
}

#[test]
fn input_frame_sig_payload_differs_per_field() {
    let base = input_frame_sig_payload(1, 2, &[3]);
    assert_ne!(base, input_frame_sig_payload(9, 2, &[3]), "seq must matter");
    assert_ne!(
        base,
        input_frame_sig_payload(1, 9, &[3]),
        "tick_ms must matter"
    );
    assert_ne!(
        base,
        input_frame_sig_payload(1, 2, &[9]),
        "blob must matter"
    );
}

#[test]
fn input_frame_sig_payload_no_field_boundary_ambiguity() {
    // Fixed-width prefix means (seq=1, blob=[2]) and (seq=2, blob=[1]) can
    // never alias to the same MAC'd bytes.
    let key = SigningKey::dev_default();
    let p1 = input_frame_sig_payload(1, 0, &[2]);
    let p2 = input_frame_sig_payload(2, 0, &[1]);
    assert_ne!(p1, p2, "payloads must differ");
    assert_ne!(sign(&key, &p1), sign(&key, &p2), "signatures must differ");
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
        sig: [0xFF; 32],
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
        assert_eq!(sig, [0xFF; 32]);
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
            sig: [0; 32],
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
        },
        ServerToClient::JoinAccepted {
            room_id: "r".into(),
            player_id: "p".into(),
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
