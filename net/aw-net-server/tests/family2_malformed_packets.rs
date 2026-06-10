//! W.3 Family-2: tampered and malformed packets — "reject and survive".
//!
//! The bar for this family is twofold and never relaxed:
//!
//! 1. A packet that fails HMAC-SHA256 verification (tampered MAC'd bytes,
//!    field-substituted, forged tag) MUST be KICKED under
//!    [`SignatureFailurePolicy::Kick`]: the server answers with a WebSocket
//!    Close frame, code 1008 (Policy), and the connection ends. Every kick is
//!    proven via the harness's bounded [`TestClient::recv_close`] (which panics
//!    on timeout) AND asserted to carry code 1008 — a connection that merely
//!    went quiet cannot pass.
//!
//! 2. A packet that fails to DECODE (truncated, garbage, oversized-undecodable,
//!    non-binary) is OUTSIDE the kick policy's mandate. The server's in-session
//!    loop guards binary frames with `if let Ok(m) = decode_msg(..)` and routes
//!    Text/Ping/other through `Some(Ok(_)) => {}` (Ping is answered with Pong).
//!    So a decode failure is SILENTLY IGNORED and the connection STAYS OPEN.
//!    "Stays open" is never proven by the absence of a Close — it is proven by
//!    subsequent successful AUTHENTICATED traffic (a signed frame followed by an
//!    in-order Ping→Pong), which under the Kick policy is itself a
//!    zero-verification-failure proof.
//!
//! Anti-vacuity is structural here: kick assertions use bounded reads that fail
//! on timeout, and each "stays open" claim is backed by real authenticated
//! round-trips, never by silence. The whole-process survival bar is enforced by
//! a FRESH client completing a full handshake + 5 signed frames + Ping→Pong
//! against the still-running server at the end of every multi-step test.
//!
//! Every server runs `SignatureFailurePolicy::Kick` (the default). Servers run
//! on ephemeral ports with unique sled temp dirs (parallel-safe), and every
//! awaited network op is bounded by the harness [`common::IO_TIMEOUT`].

mod common;

use aw_net_proto::{
    decode_msg, encode_msg, input_frame_sig_payload, sign, ClientToServer, ServerToClient,
    SigningKey,
};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, TestClient, TestServer, IO_TIMEOUT, TEST_CODEC};
use tokio_tungstenite::tungstenite::Message;

// ---------------------------------------------------------------------------
// Local helpers (the shared `common` harness is READ-ONLY; everything bespoke
// to Family-2 lives here).
// ---------------------------------------------------------------------------

/// Build an `InputFrame` with an EXPLICIT signature tag. Used for adversarial
/// frames where the tag must NOT match the transmitted fields (bit-flipped
/// blob, field substitution, tampered/zeroed/random tag). For correctly-signed
/// frames use the harness's `TestClient::send_input_frame` instead.
fn make_input_frame(seq: u32, tick_ms: u64, blob: &[u8], sig: [u8; 32]) -> ClientToServer {
    ClientToServer::InputFrame {
        seq,
        tick_ms,
        input_blob: blob.to_vec(),
        sig,
    }
}

/// Deterministic xorshift32 byte stream. Gives reproducible "garbage"/"random"
/// bytes without depending on `rand` (which the server crate's test target does
/// not declare); determinism also makes the decode-failure guard below stable
/// across runs.
fn pseudo_random_bytes(seed: u32, len: usize) -> Vec<u8> {
    let mut state: u32 = seed | 1; // keep state non-zero
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        out.push(state as u8);
    }
    out
}

/// Pseudo-random bytes whose first 4 bytes (the lz4 size-prefix that
/// `decode_msg` reads) are pinned to 0. With a zero claimed size,
/// `lz4_flex::decompress_size_prepended` allocates NOTHING and the random body
/// overflows the zero-capacity sink (or hits an invalid match offset) on the
/// very first token — a guaranteed decode error with zero server allocation.
///
/// We deliberately refuse to let the prefix drive the allocator: a hostile
/// `0xFFFFFFFF` prefix would make the server request ~4 GiB up front
/// (`decompress` does `vec![0; size]` BEFORE validating the body — a latent
/// memory-amplification DoS, reported separately). The bounded
/// [`lz4_size_amplification_frame`] exercises that allocation path safely.
fn bounded_garbage(seed: u32, len: usize) -> Vec<u8> {
    let mut b = pseudo_random_bytes(seed, len);
    if b.len() >= 4 {
        b[0..4].copy_from_slice(&0u32.to_le_bytes());
    }
    b
}

/// A tiny wire frame whose 4-byte lz4 size-prefix CLAIMS `claimed_size`
/// decompressed bytes, followed by an invalid lz4 sequence (match offset 0).
/// `decode_msg` → `decompress_size_prepended` does `vec![0; claimed_size]`
/// before it discovers the body is invalid, so this drives the server's
/// allocation path at a BOUNDED magnitude and then fails → ignored.
///
/// An attacker could set `claimed_size = 0xFFFFFFFF` (~4 GiB); that latent
/// memory-amplification DoS is noted in the report and deliberately NOT
/// triggered here.
fn lz4_size_amplification_frame(claimed_size: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(7);
    v.extend_from_slice(&claimed_size.to_le_bytes());
    v.extend_from_slice(&[0x00, 0x00, 0x00]); // 0-literal token + offset 0 ⇒ invalid
    v
}

/// Self-validating guard: the SAME codec the server uses must FAIL to decode
/// `bytes`. That guarantees the server's `if let Ok(m) = decode_msg(..)` is
/// false, so the frame takes the decode-failure path (silently ignored,
/// connection stays open) and NOT the signature-failure path (which kicks).
/// Keeps the "garbage is ignored, not kicked" tests deterministic rather than
/// probabilistic.
fn assert_undecodable(bytes: &[u8]) {
    assert!(
        decode_msg::<ClientToServer>(TEST_CODEC, bytes).is_err(),
        "fixture must be undecodable (server takes the decode-failure path, not the kick path)"
    );
}

/// Assert the server kicked the client for a signature failure: a Close frame
/// (bounded — `recv_close` panics on timeout, so silence cannot masquerade as a
/// pass), code 1008 (Policy), reason citing the signature failure.
async fn expect_signature_kick(client: &mut TestClient) {
    let frame = client
        .recv_close()
        .await
        .expect("Kick policy must close with a Close frame on signature-verification failure");
    assert_eq!(
        u16::from(frame.code),
        1008,
        "signature-failure kick must use WebSocket Close code 1008 (Policy)"
    );
    assert!(
        frame.reason.as_str().contains("signature"),
        "kick Close reason must cite the signature failure, got: {:?}",
        frame.reason.as_str()
    );
}

/// Whole-process survival probe: a FRESH client completes the full protocol
/// handshake, has 5 signed frames accepted (under Kick, any verification
/// failure would disconnect it), and gets an in-order Pong. Proves the server
/// listener and connection machinery survived whatever abuse preceded this.
async fn assert_server_healthy(server: &TestServer, key: &SigningKey, region: &str) {
    let (mut fresh, info) = TestClient::connect_and_join(server.ws_addr(), region, "coop").await;
    assert!(
        !info.room_id.is_empty(),
        "fresh client must receive a room id"
    );
    assert!(
        !info.player_id.is_empty(),
        "fresh client must receive a player id"
    );
    for seq in 1u32..=5 {
        fresh
            .send_input_frame(seq, u64::from(seq) * 33, &[0xA5, seq as u8], key)
            .await;
    }
    fresh.assert_ping_pong(0x5EED_F00D).await;
    fresh.close().await;
}

/// Bounded drain for the server's WebSocket Pong answering a WebSocket Ping we
/// sent. Skips interleaved binary snapshots; panics on timeout or stream end
/// (the server should answer a Ping, not drop the connection).
async fn expect_ws_pong(client: &mut TestClient, payload: &[u8]) {
    let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "no WS Pong for our Ping within {IO_TIMEOUT:?}"
        );
        match client.recv_raw().await {
            Some(Message::Pong(p)) if p.as_ref() == payload => return,
            Some(_) => continue,
            None => panic!("connection ended while awaiting a WS Pong (server must answer Ping)"),
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Bit-flipped MAC'd region (with in-test baseline twin).
// ---------------------------------------------------------------------------

/// Sign a frame correctly, then flip ONE bit in `input_blob` AFTER signing and
/// transmit the original signature. Because `input_blob` is inside the MAC'd
/// payload, the server recomputes the tag over the flipped blob and
/// verification fails → kick (Close 1008).
///
/// Anti-vacuity: a SECOND client first sends the UNFLIPPED twin (identical
/// seq/tick/blob/sig) and SURVIVES (in-order Pong), proving the frame would
/// have been accepted unflipped — so the kick is attributable to the bit flip
/// and nothing else. Both frames are built manually and sent via `send_msg`
/// (NOT the auto-signing helper), so the tampering survives to the wire.
#[tokio::test(flavor = "multi_thread")]
async fn bit_flipped_blob_kicks_with_baseline_twin_surviving() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let seq = 42u32;
    let tick_ms = 7u64;
    let blob = b"forward+jump+reload".to_vec();
    let sig = sign(&key, &input_frame_sig_payload(seq, tick_ms, &blob));

    // Baseline: unflipped twin from a second client must survive.
    let (mut baseline, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-bitflip-base", "coop").await;
    baseline
        .send_msg(&make_input_frame(seq, tick_ms, &blob, sig))
        .await;
    baseline.assert_ping_pong(0xBA_5E).await;
    baseline.close().await;

    // Attack: same signature, one bit flipped inside the MAC'd blob.
    let mut flipped_blob = blob.clone();
    flipped_blob[0] ^= 0x01;
    let (mut attacker, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-bitflip-atk", "coop").await;
    attacker
        .send_msg(&make_input_frame(seq, tick_ms, &flipped_blob, sig))
        .await;
    expect_signature_kick(&mut attacker).await;

    // The kick must not poison the server.
    assert_server_healthy(&server, &key, "f2-bitflip-fresh").await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 2. Field-substitution attacks (seq and tick_ms are inside the MAC'd region).
// ---------------------------------------------------------------------------

/// Sign the payload for `seq = 1` but transmit the frame with `seq = 2`
/// (blob/tick untouched). Since `seq` is the first field of
/// `input_frame_sig_payload`, the server recomputes the tag over `seq = 2` and
/// verification fails → kick. Proves `seq` is covered by the MAC.
///
/// Anti-vacuity: a correctly-signed frame is accepted FIRST on the same
/// connection (in-order Pong), so the kick is attributable to the seq
/// substitution alone.
#[tokio::test(flavor = "multi_thread")]
async fn field_substitution_seq_kicks() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-seq-sub", "coop").await;

    client.send_input_frame(1, 33, b"baseline", &key).await;
    client.assert_ping_pong(0x5E_01).await;

    let blob = b"substituted-seq".to_vec();
    let sig = sign(&key, &input_frame_sig_payload(1, 99, &blob)); // signed for seq=1
    client.send_msg(&make_input_frame(2, 99, &blob, sig)).await; // sent as seq=2
    expect_signature_kick(&mut client).await;

    assert_server_healthy(&server, &key, "f2-seq-fresh").await;
    server.shutdown();
}

/// Sign the payload for `tick_ms = 10` but transmit the frame with
/// `tick_ms = 20` (seq/blob untouched). `tick_ms` is the second field of
/// `input_frame_sig_payload`, so verification fails → kick. Proves `tick_ms`
/// is covered by the MAC. Same same-connection anti-vacuity proof as above.
#[tokio::test(flavor = "multi_thread")]
async fn field_substitution_tick_ms_kicks() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-tick-sub", "coop").await;

    client.send_input_frame(1, 33, b"baseline", &key).await;
    client.assert_ping_pong(0x71_01).await;

    let blob = b"substituted-tick".to_vec();
    let sig = sign(&key, &input_frame_sig_payload(2, 10, &blob)); // signed for tick_ms=10
    client.send_msg(&make_input_frame(2, 20, &blob, sig)).await; // sent as tick_ms=20
    expect_signature_kick(&mut client).await;

    assert_server_healthy(&server, &key, "f2-tick-fresh").await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 3-5. Tag tampering: bit-flipped, zeroed, random.
// ---------------------------------------------------------------------------

/// Correct payload, ONE bit flipped in the 32-byte HMAC tag → verification
/// fails → kick.
#[tokio::test(flavor = "multi_thread")]
async fn tag_one_bit_flip_kicks() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-tag-flip", "coop").await;

    client.send_input_frame(1, 33, b"baseline", &key).await;
    client.assert_ping_pong(0x7A_01).await;

    let blob = b"tag-bitflip".to_vec();
    let mut sig = sign(&key, &input_frame_sig_payload(2, 50, &blob));
    sig[0] ^= 0x01; // tamper the tag
    client.send_msg(&make_input_frame(2, 50, &blob, sig)).await;
    expect_signature_kick(&mut client).await;

    assert_server_healthy(&server, &key, "f2-tag-flip-fresh").await;
    server.shutdown();
}

/// Correct payload, all-zero tag (`[0u8; 32]`) → verification fails → kick.
#[tokio::test(flavor = "multi_thread")]
async fn tag_zeroed_kicks() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-tag-zero", "coop").await;

    client.send_input_frame(1, 33, b"baseline", &key).await;
    client.assert_ping_pong(0x7A_02).await;

    client
        .send_msg(&make_input_frame(2, 50, b"tag-zeroed", [0u8; 32]))
        .await;
    expect_signature_kick(&mut client).await;

    assert_server_healthy(&server, &key, "f2-tag-zero-fresh").await;
    server.shutdown();
}

/// Correct payload, 32 random bytes as the tag → probabilistic forgery must
/// NOT pass → kick.
#[tokio::test(flavor = "multi_thread")]
async fn tag_random_kicks() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-tag-rand", "coop").await;

    client.send_input_frame(1, 33, b"baseline", &key).await;
    client.assert_ping_pong(0x7A_03).await;

    let mut sig = [0u8; 32];
    sig.copy_from_slice(&pseudo_random_bytes(0xDEAD_BEEF, 32));
    client
        .send_msg(&make_input_frame(2, 50, b"tag-random", sig))
        .await;
    expect_signature_kick(&mut client).await;

    assert_server_healthy(&server, &key, "f2-tag-rand-fresh").await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 6. Truncated packets — decode-failure path: ignored, connection survives.
// ---------------------------------------------------------------------------

/// Take a correctly-encoded `InputFrame` binary frame and truncate it (1 byte,
/// half, len-1), then `send_raw` each. Truncation corrupts the lz4 stream, so
/// `decode_msg` fails — this is the DECODE-failure path (distinct from a
/// signature failure), which is OUTSIDE the kick policy's mandate. The server
/// silently ignores each frame and the connection STAYS OPEN.
///
/// Survival is proven (never assumed): after the truncated barrage the same
/// client sends a valid signed frame and completes an in-order Ping→Pong —
/// reject-and-survive without a kick.
#[tokio::test(flavor = "multi_thread")]
async fn truncated_frames_ignored_connection_survives() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) = TestClient::connect_and_join(server.ws_addr(), "f2-trunc", "coop").await;

    let blob = b"truncate-me-this-is-a-real-and-valid-input-frame".to_vec();
    let valid = make_input_frame(
        7,
        70,
        &blob,
        sign(&key, &input_frame_sig_payload(7, 70, &blob)),
    );
    let encoded = encode_msg(TEST_CODEC, &valid);
    assert!(
        encoded.len() > 8,
        "need a non-trivial encoded frame for meaningful truncation, got {}",
        encoded.len()
    );

    for cut in [1usize, encoded.len() / 2, encoded.len() - 1] {
        let truncated = encoded[..cut].to_vec();
        assert_undecodable(&truncated);
        client.send_raw(truncated).await;
    }

    client
        .send_input_frame(8, 80, b"post-truncation", &key)
        .await;
    client.assert_ping_pong(0x7717).await;

    client.close().await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 7. Garbage bytes — decode-failure path: ignored, connection survives.
// ---------------------------------------------------------------------------

/// Random byte blobs of several sizes, INCLUDING zero bytes. A zero-length
/// binary WebSocket frame IS sendable (tungstenite permits an empty binary
/// payload); the server reads it, `decode_msg` fails (lz4's size prefix needs
/// ≥4 bytes), and it is ignored. All blobs are guarded by [`assert_undecodable`]
/// so they exercise the decode-failure path, not the kick path. Survival is
/// proven by a subsequent valid signed frame + in-order Ping→Pong.
#[tokio::test(flavor = "multi_thread")]
async fn garbage_bytes_ignored_connection_survives() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-garbage", "coop").await;

    let sizes: [(u32, usize); 5] = [
        (0x00C0_FFEE, 0), // zero-length binary frame
        (0x0000_0001, 1),
        (0x0000_0002, 7),
        (0x0000_0003, 64),
        (0x0000_0004, 1000),
    ];
    for (seed, len) in sizes {
        let g = bounded_garbage(seed, len);
        assert_undecodable(&g);
        client.send_raw(g).await;
    }

    client.send_input_frame(9, 90, b"post-garbage", &key).await;
    client.assert_ping_pong(0x6A6A).await;

    client.close().await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 8. Oversized packets — process must survive; fresh client must round-trip.
// ---------------------------------------------------------------------------

/// Three oversized angles, all chosen UNDER the tungstenite transport caps
/// (default `max_frame_size` = 16 MiB, `max_message_size` = 64 MiB) so they
/// exercise the APPLICATION layer rather than being rejected by the transport
/// itself:
///
/// (a) A properly-signed 8 MiB blob: the server decompresses it, verifies the
///     (valid) signature, and processes it. The connection survives.
/// (b) An unsigned/garbage 8 MiB raw frame: large on the wire but undecodable →
///     ignored. The connection survives.
/// (c) A bounded size-amplification probe: a 7-byte frame claiming 8 MiB of
///     decompressed output. The server allocates that up front, then errors on
///     the invalid body → ignored. (A real attacker could claim ~4 GiB — a
///     latent memory-amplification DoS noted in the report, NOT triggered here.)
///
/// A frame at/above 16 MiB on the wire would be rejected by tungstenite's
/// per-frame cap on BOTH ends before the application ever sees it, so 8 MiB is
/// chosen deliberately. The closing assertion is the process-survival bar: a
/// FRESH client must connect and round-trip afterwards.
#[tokio::test(flavor = "multi_thread")]
async fn oversized_packets_do_not_panic_server() {
    const BIG: usize = 8 * 1024 * 1024; // 8 MiB < 16 MiB frame cap < 64 MiB msg cap

    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "f2-oversize", "coop").await;

    // (a) Properly signed oversized blob → processed, survives.
    let big_blob = pseudo_random_bytes(0x0000_ABCD, BIG);
    client.send_input_frame(1, 33, &big_blob, &key).await;
    client.assert_ping_pong(0x0B16).await;

    // (b) Oversized garbage raw frame → undecodable, ignored, survives.
    let big_garbage = bounded_garbage(0x0000_1234, BIG);
    assert_undecodable(&big_garbage);
    client.send_raw(big_garbage).await;
    client.assert_ping_pong(0x0B17).await;

    // (c) Bounded size-amplification probe → allocates 8 MiB then errors,
    //     ignored, survives.
    let amp = lz4_size_amplification_frame(BIG as u32);
    assert_undecodable(&amp);
    client.send_raw(amp).await;
    client.assert_ping_pong(0x0B18).await;

    client.close().await;

    // Process-survival bar.
    assert_server_healthy(&server, &key, "f2-oversize-fresh").await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 9. Non-binary WebSocket frames — ignored / answered; connection survives.
// ---------------------------------------------------------------------------

/// Text frame with garbage, Text frame with valid-looking JSON, and a WS Ping
/// with a payload. The server's in-session loop only decodes `Message::Binary`;
/// Text falls into `Some(Ok(_)) => {}` (ignored — Text is never parsed as a
/// protocol message), and `Message::Ping(p)` is answered with `Message::Pong(p)`.
/// The connection stays open throughout, proven by an authenticated round-trip
/// before and after the non-binary barrage.
#[tokio::test(flavor = "multi_thread")]
async fn non_binary_ws_frames_ignored_connection_survives() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) = TestClient::connect_and_join(server.ws_addr(), "f2-nonbin", "coop").await;

    client.send_input_frame(1, 33, b"baseline", &key).await;
    client.assert_ping_pong(0x9A9A).await;

    client
        .send_ws(Message::Text("garbage-text-not-a-protocol-frame".into()))
        .await;
    client
        .send_ws(Message::Text(r#"{"Hello":{"protocol":1}}"#.into()))
        .await;

    let ping_payload = b"aw-net-ws-ping-probe".to_vec();
    client
        .send_ws(Message::Ping(ping_payload.clone().into()))
        .await;
    expect_ws_pong(&mut client, &ping_payload).await;

    client
        .send_input_frame(2, 66, b"post-nonbinary", &key)
        .await;
    client.assert_ping_pong(0x9B9B).await;

    client.close().await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 10. Malformed during handshake (pre-join) — pin the REAL behavior.
// ---------------------------------------------------------------------------

/// Two pre-join malformed first-frame shapes, with the server's ACTUAL behavior
/// pinned from `handle_socket`:
///
/// - Undecodable garbage as the first frame: the handshake `recv` decodes the
///   first message and an undecodable binary frame makes it return `Err`, so
///   `handle_socket` returns `Err` and the connection task drops the socket
///   with NO `ProtocolError` (this is the decode-failure path; the server does
///   not answer it). Pinned behavior: silent connection drop, no panic.
/// - A decodable-but-WRONG first message (a valid `Ping` where `Hello` is
///   required): hits the handshake's `_ =>` arm, which sends an explicit
///   `ProtocolError { msg: "expected Hello" }` and then ends the connection.
///
/// Both must leave the server able to serve a FRESH client afterwards.
#[tokio::test(flavor = "multi_thread")]
async fn malformed_handshake_first_frame_pins_behavior_and_server_survives() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    // Phase 1: undecodable garbage instead of Hello → silent drop, no answer.
    let mut raw = TestClient::connect(server.ws_addr()).await;
    let g = bounded_garbage(0x0000_5151, 32);
    assert_undecodable(&g);
    raw.send_raw(g).await;
    assert!(
        raw.recv_raw().await.is_none(),
        "undecodable handshake frame must drop the connection with no ProtocolError"
    );

    // Phase 2: decodable-but-wrong first message → explicit ProtocolError, then end.
    let mut wrong = TestClient::connect(server.ws_addr()).await;
    wrong.send_msg(&ClientToServer::Ping { nano: 7 }).await;
    match wrong.recv_msg().await {
        ServerToClient::ProtocolError { msg } => assert!(
            msg.contains("expected Hello"),
            "wrong first message must yield 'expected Hello', got: {msg}"
        ),
        other => panic!("expected ProtocolError for a non-Hello first message, got {other:?}"),
    }
    assert!(
        wrong.recv_raw().await.is_none(),
        "server must end the connection after the handshake ProtocolError"
    );

    // The server must still serve a fresh client after both malformed handshakes.
    assert_server_healthy(&server, &key, "f2-handshake-fresh").await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 11. Cross-cutting survival sweep — mixed assault on one server instance.
// ---------------------------------------------------------------------------

/// Throw the whole Family-2 gamut at ONE server instance from several dirty
/// connections — truncated + garbage binary (ignored), a tampered-tag frame
/// (kicked), an undecodable handshake (silently dropped), and non-binary frames
/// (ignored/answered) — then prove the still-running server serves a FRESH
/// client doing a full handshake + 5 signed frames + Ping→Pong. This is the
/// integrated reject-and-survive proof: no malformed input took the listener
/// or any peer connection down.
#[tokio::test(flavor = "multi_thread")]
async fn cross_cutting_survival_sweep_under_mixed_assault() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    // Dirty conn 1 (joined): truncated + garbage binary → ignored, survives.
    let (mut c1, _) = TestClient::connect_and_join(server.ws_addr(), "f2-sweep-a", "coop").await;
    let blob = b"sweep-frame-payload".to_vec();
    let enc = encode_msg(
        TEST_CODEC,
        &make_input_frame(
            1,
            33,
            &blob,
            sign(&key, &input_frame_sig_payload(1, 33, &blob)),
        ),
    );
    let trunc = enc[..enc.len() / 2].to_vec();
    assert_undecodable(&trunc);
    c1.send_raw(trunc).await;
    let g1 = bounded_garbage(0x0000_9001, 128);
    assert_undecodable(&g1);
    c1.send_raw(g1).await;
    c1.send_input_frame(2, 66, b"c1-alive", &key).await;
    c1.assert_ping_pong(0xC1C1).await;

    // Dirty conn 2 (joined): tampered (zeroed) tag → kicked (Close 1008).
    let (mut c2, _) = TestClient::connect_and_join(server.ws_addr(), "f2-sweep-b", "coop").await;
    c2.send_msg(&make_input_frame(1, 33, b"forge", [0u8; 32]))
        .await;
    expect_signature_kick(&mut c2).await;

    // Dirty conn 3 (raw): undecodable handshake → silent drop.
    let mut c3 = TestClient::connect(server.ws_addr()).await;
    let g3 = bounded_garbage(0x0000_9003, 40);
    assert_undecodable(&g3);
    c3.send_raw(g3).await;
    assert!(
        c3.recv_raw().await.is_none(),
        "undecodable handshake frame must drop the connection"
    );

    // Dirty conn 4 (joined): non-binary frames → ignored / answered, survives.
    let (mut c4, _) = TestClient::connect_and_join(server.ws_addr(), "f2-sweep-d", "coop").await;
    c4.send_ws(Message::Text("sweep-text".into())).await;
    let pp = b"sweep-ws-ping".to_vec();
    c4.send_ws(Message::Ping(pp.clone().into())).await;
    expect_ws_pong(&mut c4, &pp).await;
    c4.send_input_frame(1, 33, b"c4-alive", &key).await;
    c4.assert_ping_pong(0xC4C4).await;

    // Drop the still-open dirty connections abruptly (no close handshake).
    drop(c1);
    drop(c4);

    // Cross-cutting survival proof against the still-running server.
    assert_server_healthy(&server, &key, "f2-sweep-final").await;
    server.shutdown();
}
