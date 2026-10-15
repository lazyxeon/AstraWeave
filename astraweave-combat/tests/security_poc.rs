//! SEC-POC tests. They assert the secure property and fail on today's parser.
//! See `docs/current/SECURITY_AUDIT.md` (SEC-SAVE-2).

use astraweave_combat::decode_vitals;

/// A record list past 4_096 entries must be rejected. Four thousand is above
/// any combat fixture in this crate. Raising the cap later is fine if a real
/// save is larger; dropping the check is not.
#[test]
fn poc_vitals_rejects_more_than_4096_records() {
    const COUNT: usize = 4_097;
    const STRIDE: usize = 8 * 9;
    let mut body = Vec::with_capacity(4 + COUNT * STRIDE);
    body.extend_from_slice(&(COUNT as u32).to_le_bytes());
    for i in 0..COUNT {
        body.extend_from_slice(&(i as u64).to_le_bytes());
        for _ in 0..8 {
            body.extend_from_slice(&1i64.to_le_bytes());
        }
    }
    let text = hex_encode(&body);
    let result = decode_vitals(&text);
    assert!(
        result.is_err(),
        "decoded {COUNT} vitals records with no record cap"
    );
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
