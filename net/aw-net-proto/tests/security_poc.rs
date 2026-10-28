//! SEC-POC. See `docs/current/SECURITY_AUDIT.md` (SEC-NET-3).

use aw_net_proto::{decode_msg, Codec, ServerToClient};

/// Four bytes that claim a 16 MiB uncompressed body. The decoder must refuse
/// that claim before reserving it.
#[test]
fn poc_postcard_lz4_rejects_oversized_declared_output() {
    let claimed: u32 = 16 * 1024 * 1024;
    let err = decode_msg::<ServerToClient>(Codec::PostcardLz4, &claimed.to_le_bytes())
        .expect_err("hostile frame must not decode");
    let msg = err.to_string();
    assert!(
        msg.contains("too large") || msg.contains("cap"),
        "declared uncompressed size was not rejected by an application cap: {msg}"
    );
}

fn assert_limit_exceeded(err: aw_net_proto::WireError) {
    let msg = err.to_string();
    assert!(
        msg.contains("LimitExceeded"),
        "cap must refuse before the buffer is used: {msg}"
    );
}

/// `ServerToClient::ProtocolError` is variant 7. Its `String` length is a
/// usize varint. 32 MiB is claimed before any string bytes are read.
#[test]
fn bincode_refuses_a_32_mib_protocol_error_prefix() {
    let mut bytes = vec![7u8];
    let len: u32 = 32 * 1024 * 1024;
    bytes.push(252);
    bytes.extend_from_slice(&len.to_le_bytes());
    let err = decode_msg::<ServerToClient>(Codec::Bincode, &bytes)
        .expect_err("a 32 MiB string prefix must not allocate");
    assert_limit_exceeded(err);
}

/// usize claims 8 bytes before the varint, then each payload byte claims 1.
/// 4_194_297 payload bytes make the claim 4_194_305, one past the 4 MiB cap.
/// The bytes are present, so this is the cap and not a short read.
#[test]
fn bincode_refuses_a_vec_longer_than_the_cap() {
    let len: u32 = 4_194_297;
    let mut bytes = vec![252u8];
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.resize(bytes.len() + len as usize, 0);
    let err = decode_msg::<Vec<u8>>(Codec::Bincode, &bytes)
        .expect_err("a vec at the cap must not decode");
    assert_limit_exceeded(err);
}
