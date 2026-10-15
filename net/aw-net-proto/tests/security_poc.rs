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
