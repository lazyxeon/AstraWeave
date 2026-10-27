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

/// Bincode's default config allocates a `Vec` from the length prefix before
/// any payload bytes exist. One past the 4 MiB cap must be a decode error.
#[test]
fn bincode_refuses_a_vec_longer_than_the_cap() {
    let len = (aw_net_proto::MAX_LZ4_DECLARED_OUTPUT as u32) + 1;
    let mut bytes = vec![252u8];
    bytes.extend_from_slice(&len.to_le_bytes());
    let err = decode_msg::<Vec<u8>>(Codec::Bincode, &bytes)
        .expect_err("a length past the cap must not allocate");
    let msg = err.to_string();
    assert!(msg.contains("bincode"), "{msg}");
}
