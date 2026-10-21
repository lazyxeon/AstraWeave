# HMAC-SHA256 Implementation for Game Message Authentication

## Research Summary

### 1. HMAC-SHA256 in Rust

**Recommended Crates:**
- `hmac = "0.12"` - Generic HMAC implementation
- `sha2 = "0.10"` - SHA-256 hash function
- `subtle = "2.5"` - Constant-time comparison utilities

**Key Benefits:**
- Industry-standard cryptographic primitive (RFC 2104)
- Prevents length-extension attacks (unlike plain SHA-256)
- Provides message authentication and integrity
- Works with any size key (recommended: 32 bytes minimum)

### 2. Session Key Management and Rotation

**Best Practices:**
- Generate unique session keys per room/match
- Rotate keys periodically (e.g., every 1 hour or 10,000 messages)
- Use cryptographically secure random number generator (CSRNG)
- Store keys in memory only, never persist to disk
- Derive per-player keys from room session key for additional isolation

**Key Rotation Strategy:**
- Implement key versioning with 2-key overlap window
- Send new key in advance before rotation
- Accept both old and new keys during transition period
- Reject old key after transition window expires

### 3. Replay Attack Prevention

**Timestamp-Based Protection:**
- Include Unix timestamp (milliseconds) in signed message
- Server validates timestamp is within acceptable window (e.g., ±5 seconds)
- Reject messages with timestamps too old or too far in future
- Pros: Simple, no server state needed
- Cons: Vulnerable to clock skew and replay within window

**Nonce-Based Protection:**
- Include monotonically increasing sequence number (nonce)
- Server tracks last valid nonce per player
- Reject messages with nonce ≤ last seen nonce
- Pros: Prevents all replay attacks
- Cons: Requires server state per player

**Hybrid Approach (Recommended for Games):**
- Use sequence number (already in `InputFrame`) as primary nonce
- Add timestamp as secondary protection against delayed replay
- Combine both in HMAC signature
- Server validates: `seq > last_seen_seq AND |timestamp - server_time| < window`

### 4. Constant-Time Comparison

**Why It Matters:**
- Standard `==` operator exits early on first mismatch
- Timing differences reveal information about correct MAC
- Attackers can iteratively guess MAC bytes using timing oracle

**Rust Solutions:**
- Use `subtle::ConstantTimeEq` trait for slices
- HMAC crate's `verify_slice()` method already uses constant-time comparison
- Never use `==` or `!=` for comparing MACs directly

**Example:**
```rust
// BAD - timing attack vulnerable
if computed_mac == received_mac { ... }

// GOOD - constant-time comparison
use hmac::Mac;
mac.verify_slice(&received_mac).is_ok()

// GOOD - using subtle crate directly
use subtle::ConstantTimeEq;
computed_mac.ct_eq(&received_mac).into()
```

---

## Code Implementation

### Dependencies to Add

Add to `net/aw-net-proto/Cargo.toml`:
```toml
[dependencies]
hmac = "0.12"
sha2 = "0.10"
```

### 1. Replace `sign16` in `aw-net-proto/src/lib.rs`

**Current Implementation (lines 145-159):**
```rust
/// Minimal tamper-evident signature (MVP): xor 16 bytes of input hash with key hint
pub fn sign16(input: &[u8], session_key_hint: &[u8; 8]) -> [u8; 16] {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    let h = hasher.finish();
    let mut out = [0u8; 16];
    out[0..8].copy_from_slice(&h.to_le_bytes());
    out[8..16].copy_from_slice(&(!h).to_le_bytes());
    for i in 0..8 {
        out[i] ^= session_key_hint[i];
        out[8 + i] ^= session_key_hint[i];
    }
    out
}
```

**New HMAC-SHA256 Implementation:**
```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// HMAC-SHA256 message authentication with replay protection
/// 
/// Computes HMAC-SHA256 over: input_blob || seq || timestamp_ms
/// Returns first 16 bytes of 32-byte HMAC tag for space efficiency
/// 
/// Security guarantees:
/// - Message authentication (only holder of session_key can create valid tags)
/// - Integrity protection (any modification invalidates tag)
/// - Replay protection (seq and timestamp_ms bound in signature)
pub fn hmac_sign(
    input_blob: &[u8],
    seq: u32,
    timestamp_ms: u64,
    session_key: &[u8; 32],
) -> [u8; 16] {
    let mut mac = HmacSha256::new_from_slice(session_key)
        .expect("HMAC accepts keys of any size");
    
    // Sign input blob
    mac.update(input_blob);
    
    // Include sequence number to prevent replay attacks
    mac.update(&seq.to_le_bytes());
    
    // Include timestamp for temporal replay protection
    mac.update(&timestamp_ms.to_le_bytes());
    
    // Finalize and truncate to 16 bytes
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    
    let mut output = [0u8; 16];
    output.copy_from_slice(&code_bytes[0..16]);
    output
}

/// Verify HMAC-SHA256 signature with constant-time comparison
/// 
/// Returns true if signature is valid, false otherwise
/// Uses constant-time comparison to prevent timing attacks
pub fn hmac_verify(
    input_blob: &[u8],
    seq: u32,
    timestamp_ms: u64,
    session_key: &[u8; 32],
    received_sig: &[u8; 16],
) -> bool {
    let mut mac = HmacSha256::new_from_slice(session_key)
        .expect("HMAC accepts keys of any size");
    
    mac.update(input_blob);
    mac.update(&seq.to_le_bytes());
    mac.update(&timestamp_ms.to_le_bytes());
    
    // Verify using constant-time comparison (protects against timing attacks)
    // Only check first 16 bytes since we truncated the signature
    mac.verify_slice(received_sig).is_ok()
}

/// Legacy sign16 function - DEPRECATED, use hmac_sign instead
/// 
/// This function uses a weak non-cryptographic hash and should not be used
/// for production security. Kept only for backwards compatibility.
#[deprecated(
    since = "0.2.0",
    note = "Use hmac_sign instead for proper cryptographic message authentication"
)]
pub fn sign16(input: &[u8], session_key_hint: &[u8; 8]) -> [u8; 16] {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    let h = hasher.finish();
    let mut out = [0u8; 16];
    out[0..8].copy_from_slice(&h.to_le_bytes());
    out[8..16].copy_from_slice(&(!h).to_le_bytes());
    for i in 0..8 {
        out[i] ^= session_key_hint[i];
        out[8 + i] ^= session_key_hint[i];
    }
    out
}
```

### 2. Update Protocol Messages in `aw-net-proto/src/lib.rs`

**Modify `InputFrame` message (around line 38-45):**
```rust
/// Per-frame input payload (prediction).
InputFrame {
    seq: u32,
    tick_ms: u64,
    // Client timestamp in milliseconds since Unix epoch
    // Used for replay attack prevention and latency calculation
    timestamp_ms: u64,
    // e.g. movement vector, buttons; opaque to engine:
    input_blob: Vec<u8>,
    // HMAC-SHA256 signature (truncated to 16 bytes)
    // Computed over: input_blob || seq || timestamp_ms
    sig: [u8; 16],
},
```

### 3. Update Server Implementation in `net/aw-net-server/src/main.rs`

**Replace lines 649-656 (non-TLS handler):**
```rust
// HMAC-SHA256 tamper and replay protection
let client_timestamp_ms = // extract from InputFrame message
let server_time_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

// Verify signature with constant-time comparison
let sig_valid = aw_net_proto::hmac_verify(
    &input_blob,
    seq,
    client_timestamp_ms,
    &room.session_key.0,
    &sig,
);

if !sig_valid {
    warn!(
        "HMAC signature verification failed for pid={} seq={}",
        pid, seq
    );
    // Consider kick after N consecutive failures
    // For now, just warn and skip processing
    continue;
}

// Replay protection: timestamp window check (±5 seconds)
const TIMESTAMP_WINDOW_MS: u64 = 5000;
let time_diff = server_time_ms.abs_diff(client_timestamp_ms);

if time_diff > TIMESTAMP_WINDOW_MS {
    warn!(
        "Timestamp out of window for pid={} (diff={}ms, max={}ms)",
        pid, time_diff, TIMESTAMP_WINDOW_MS
    );
    // Potential replay attack or severe clock skew
    continue;
}

// Replay protection: sequence number check (already exists)
if seq <= p.last_input_seq {
    warn!(
        "Sequence number replay detected for pid={} (received={}, last={})",
        pid, seq, p.last_input_seq
    );
    continue;
}
```

**Replace lines 766-771 (TLS handler):**
```rust
// HMAC-SHA256 tamper and replay protection
let client_timestamp_ms = // extract from InputFrame message
let server_time_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

// Verify signature with constant-time comparison
let sig_valid = aw_net_proto::hmac_verify(
    &input_blob,
    seq,
    client_timestamp_ms,
    &room.session_key.0,
    &sig,
);

if !sig_valid {
    warn!(
        "HMAC signature verification failed (TLS) for pid={} seq={}",
        pid, seq
    );
    continue;
}

// Replay protection: timestamp window check (±5 seconds)
const TIMESTAMP_WINDOW_MS: u64 = 5000;
let time_diff = server_time_ms.abs_diff(client_timestamp_ms);

if time_diff > TIMESTAMP_WINDOW_MS {
    warn!(
        "Timestamp out of window (TLS) for pid={} (diff={}ms)",
        pid, time_diff
    );
    continue;
}

// Replay protection: sequence number check
if seq <= p.last_input_seq {
    warn!(
        "Sequence replay (TLS) for pid={} (received={}, last={})",
        pid, seq, p.last_input_seq
    );
    continue;
}
```

### 4. Update Client Implementation in `net/aw-net-client/src/main.rs`

**Replace lines 85-95:**
```rust
let blob = postcard::to_allocvec(&cmd).unwrap();

// Get current timestamp for replay protection
let timestamp_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

// Compute HMAC-SHA256 signature
let sig = aw_net_proto::hmac_sign(&blob, seq, timestamp_ms, &session_key);

send(
    &mut ws,
    &ClientToServer::InputFrame {
        seq,
        tick_ms: 33,
        timestamp_ms,  // Add this field
        input_blob: blob,
        sig,
    },
)
.await?;
```

**Update imports (line 3):**
```rust
use aw_net_proto::{hmac_sign, ClientToServer, Codec, ServerToClient, PROTOCOL_VERSION};
```

---

## Advanced: Session Key Rotation

Add to `net/aw-net-server/src/main.rs` Room struct:

```rust
#[derive(Clone)]
struct Room {
    id: RoomId,
    region: String,
    game_mode: String,
    session_key: SessionKey,
    session_key_version: u16,          // NEW: track key version
    next_session_key: Option<SessionKey>,  // NEW: for rotation
    key_rotation_time: Option<Instant>,    // NEW: when to rotate
    tick_hz: u32,
    players: HashMap<PlayerId, Player>,
    tick: u64,
    snap_id: u32,
}

impl Room {
    /// Initiate key rotation: generate new key, set rotation time
    fn begin_key_rotation(&mut self, rotation_delay: Duration) {
        self.next_session_key = Some(SessionKey::random());
        self.key_rotation_time = Some(Instant::now() + rotation_delay);
        info!(
            "Room {} initiated key rotation v{} -> v{}",
            self.id,
            self.session_key_version,
            self.session_key_version + 1
        );
        // TODO: Send ServerToClient::KeyRotation message to all clients
    }
    
    /// Complete key rotation: swap keys, increment version
    fn complete_key_rotation(&mut self) {
        if let Some(new_key) = self.next_session_key.take() {
            self.session_key = new_key;
            self.session_key_version += 1;
            self.key_rotation_time = None;
            info!("Room {} completed key rotation to v{}", self.id, self.session_key_version);
        }
    }
    
    /// Verify HMAC with both current and next key (during rotation)
    fn verify_hmac_with_rotation(
        &self,
        input_blob: &[u8],
        seq: u32,
        timestamp_ms: u64,
        sig: &[u8; 16],
    ) -> bool {
        // Try current key
        if aw_net_proto::hmac_verify(input_blob, seq, timestamp_ms, &self.session_key.0, sig) {
            return true;
        }
        
        // During rotation window, also accept next key
        if let Some(ref next_key) = self.next_session_key {
            if aw_net_proto::hmac_verify(input_blob, seq, timestamp_ms, &next_key.0, sig) {
                return true;
            }
        }
        
        false
    }
}
```

Add new protocol message for key rotation:
```rust
// In aw-net-proto/src/lib.rs ServerToClient enum
/// Notify client of session key rotation
KeyRotation {
    new_key_version: u16,
    new_key: SessionKey,
    activation_time_ms: u64,  // Unix timestamp when new key becomes active
},
```

---

## Security Comparison

| Aspect | `sign16` (Old) | HMAC-SHA256 (New) |
|--------|----------------|-------------------|
| **Hash Function** | `DefaultHasher` (non-cryptographic) | SHA-256 (cryptographic) |
| **Key Size** | 8 bytes (hint) | 32 bytes (full key) |
| **Tag Size** | 16 bytes | 16 bytes (truncated from 32) |
| **Security Level** | ~0 bits (trivial to forge) | ~128 bits (truncated from 256) |
| **Replay Protection** | None | Timestamp + sequence number |
| **Timing Attack Protection** | No | Yes (constant-time comparison) |
| **Standards Compliance** | No | RFC 2104 (HMAC) |
| **Collision Resistance** | No | Yes |
| **Length Extension Attack** | Vulnerable | Immune |

---

## Performance Considerations

**HMAC-SHA256 Performance:**
- Modern CPUs: ~500-1000 MB/s throughput
- Typical game input: ~100-500 bytes per frame
- Per-message overhead: ~1-2 microseconds on modern CPU
- Negligible impact for 60Hz game tick rate

**Optimization Tips:**
1. Reuse `Hmac<Sha256>` instance per thread (reset between uses)
2. Batch verification if processing multiple messages
3. Consider SIMD-accelerated SHA-256 implementation
4. Profile before optimizing - crypto is rarely the bottleneck

**Benchmark Command:**
```bash
cd net/aw-net-proto
cargo bench --bench hmac_bench
```

---

## Testing Recommendations

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sign_verify_roundtrip() {
        let key = SessionKey::random();
        let input = b"test input data";
        let seq = 12345;
        let timestamp = 1700000000000;
        
        let sig = hmac_sign(input, seq, timestamp, &key.0);
        assert!(hmac_verify(input, seq, timestamp, &key.0, &sig));
    }
    
    #[test]
    fn test_hmac_wrong_key_fails() {
        let key1 = SessionKey::random();
        let key2 = SessionKey::random();
        let sig = hmac_sign(b"data", 1, 1000, &key1.0);
        
        assert!(!hmac_verify(b"data", 1, 1000, &key2.0, &sig));
    }
    
    #[test]
    fn test_hmac_modified_data_fails() {
        let key = SessionKey::random();
        let sig = hmac_sign(b"original", 1, 1000, &key.0);
        
        assert!(!hmac_verify(b"modified", 1, 1000, &key.0, &sig));
    }
    
    #[test]
    fn test_hmac_replay_seq_fails() {
        let key = SessionKey::random();
        let sig = hmac_sign(b"data", 100, 1000, &key.0);
        
        // Different sequence number invalidates signature
        assert!(!hmac_verify(b"data", 101, 1000, &key.0, &sig));
    }
    
    #[test]
    fn test_hmac_replay_timestamp_fails() {
        let key = SessionKey::random();
        let sig = hmac_sign(b"data", 1, 1000, &key.0);
        
        // Different timestamp invalidates signature
        assert!(!hmac_verify(b"data", 1, 2000, &key.0, &sig));
    }
}
```

---

## Migration Path

1. **Phase 1: Add HMAC functions alongside sign16** (backwards compatible)
   - Add `hmac` and `sha2` dependencies
   - Implement `hmac_sign` and `hmac_verify`
   - Keep `sign16` but mark as deprecated
   - Add tests

2. **Phase 2: Update protocol** (breaking change)
   - Add `timestamp_ms` field to `InputFrame`
   - Increment `PROTOCOL_VERSION`
   - Update client and server to use HMAC

3. **Phase 3: Deploy and monitor**
   - Deploy server with HMAC validation
   - Monitor for signature failures
   - Add metrics for timing attack detection

4. **Phase 4: Remove legacy code**
   - After all clients upgraded, remove `sign16`
   - Clean up deprecated code
   - Audit security logs

---

## External References

**HMAC Specification:**
- RFC 2104: HMAC: Keyed-Hashing for Message Authentication
- https://datatracker.ietf.org/doc/html/rfc2104

**Rust Crate Documentation:**
- hmac crate: https://docs.rs/hmac
- sha2 crate: https://docs.rs/sha2
- subtle crate: https://docs.rs/subtle

**Security Best Practices:**
- OWASP API Security: https://owasp.org/www-project-api-security/
- Preventing Replay Attacks: https://cheatsheetseries.owasp.org/cheatsheets/Transaction_Authorization_Cheat_Sheet.html

**Game Networking Security:**
- "Fast-Paced Multiplayer" by Gabriel Gambetta (security chapter)
- Valve's Source Engine networking security model
- Riot Games' network security talks (GDC)
