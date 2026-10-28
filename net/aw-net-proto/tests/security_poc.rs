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

/// Both callers must return `LimitExceeded`. A `String` length prefix is
/// refused before that buffer is filled. A `Vec<u8>` decode fills 4,194,296
/// bytes and then returns `LimitExceeded`. The capacity is 4,194,304.
fn assert_limit_exceeded(err: aw_net_proto::WireError) {
    let msg = err.to_string();
    assert!(
        msg.contains("LimitExceeded"),
        "expected LimitExceeded from the 4 MiB cap: {msg}"
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

/// The decoder claims 8 bytes for the length prefix. The wire prefix is 5
/// bytes. Each payload byte claims 1 more. This fixture asks for 4_194_297
/// payload bytes. The decoder fills 4,194,296 of them and then returns
/// `LimitExceeded`. The capacity is 4,194,304.
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
