# HMAC-SHA256 Quick Reference - Code Changes

## Step 1: Add Dependencies

**File: `net/aw-net-proto/Cargo.toml`**
```toml
[dependencies]
hmac = "0.12"
sha2 = "0.10"
# ... existing dependencies
```

## Step 2: Replace Functions in Protocol

**File: `net/aw-net-proto/src/lib.rs`**

Add these imports at the top:
```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;
type HmacSha256 = Hmac<Sha256>;
```

Replace `sign16` function (lines 145-159) with:
```rust
/// HMAC-SHA256 message authentication with replay protection
pub fn hmac_sign(
    input_blob: &[u8],
    seq: u32,
    timestamp_ms: u64,
    session_key: &[u8; 32],
) -> [u8; 16] {
    let mut mac = HmacSha256::new_from_slice(session_key)
        .expect("HMAC accepts keys of any size");
    mac.update(input_blob);
    mac.update(&seq.to_le_bytes());
    mac.update(&timestamp_ms.to_le_bytes());
    
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    let mut output = [0u8; 16];
    output.copy_from_slice(&code_bytes[0..16]);
    output
}

/// Verify HMAC-SHA256 signature with constant-time comparison
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
    
    // Constant-time comparison prevents timing attacks
    mac.verify_slice(received_sig).is_ok()
}
```

Update `InputFrame` message (around line 38):
```rust
InputFrame {
    seq: u32,
    tick_ms: u64,
    timestamp_ms: u64,  // ADD THIS LINE
    input_blob: Vec<u8>,
    sig: [u8; 16],
},
```

## Step 3: Update Server (Non-TLS)

**File: `net/aw-net-server/src/main.rs`**

Replace lines 649-656 with:
```rust
// HMAC-SHA256 tamper and replay protection
let server_time_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

// Verify signature with constant-time comparison
if !aw_net_proto::hmac_verify(&input_blob, seq, timestamp_ms, &room.session_key.0, &sig) {
    warn!("HMAC verification failed for pid={} seq={}", pid, seq);
    continue;
}

// Replay protection: timestamp window check (±5 seconds)
const TIMESTAMP_WINDOW_MS: u64 = 5000;
if server_time_ms.abs_diff(timestamp_ms) > TIMESTAMP_WINDOW_MS {
    warn!("Timestamp out of window for pid={} seq={}", pid, seq);
    continue;
}

// Replay protection: sequence number (already exists, keep this check)
if seq <= p.last_input_seq {
    warn!("Sequence replay for pid={} seq={}", pid, seq);
    continue;
}
```

## Step 4: Update Server (TLS)

**File: `net/aw-net-server/src/main.rs`**

Replace lines 766-771 with:
```rust
// HMAC-SHA256 tamper and replay protection
let server_time_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

// Verify signature
if !aw_net_proto::hmac_verify(&input_blob, seq, timestamp_ms, &room.session_key.0, &sig) {
    warn!("HMAC verification failed (TLS) for pid={} seq={}", pid, seq);
    continue;
}

// Timestamp window check
const TIMESTAMP_WINDOW_MS: u64 = 5000;
if server_time_ms.abs_diff(timestamp_ms) > TIMESTAMP_WINDOW_MS {
    warn!("Timestamp out of window (TLS) for pid={} seq={}", pid, seq);
    continue;
}

// Sequence number check
if seq <= p.last_input_seq {
    warn!("Sequence replay (TLS) for pid={} seq={}", pid, seq);
    continue;
}
```

## Step 5: Update Client

**File: `net/aw-net-client/src/main.rs`**

Update import (line 3):
```rust
use aw_net_proto::{ClientToServer, Codec, ServerToClient, PROTOCOL_VERSION};
```

Replace lines 85-95 with:
```rust
let blob = postcard::to_allocvec(&cmd).unwrap();

// Get current timestamp for replay protection
let timestamp_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

// Full session key needed (not just hint)
// Note: Client must receive full session_key from server during join
let session_key = /* TODO: get from server handshake */;

// Compute HMAC-SHA256 signature
let sig = aw_net_proto::hmac_sign(&blob, seq, timestamp_ms, &session_key);

send(
    &mut ws,
    &ClientToServer::InputFrame {
        seq,
        tick_ms: 33,
        timestamp_ms,
        input_blob: blob,
        sig,
    },
)
.await?;
```

## Step 6: Update Server to Send Full Session Key

**File: `net/aw-net-server/src/main.rs`**

Find where `JoinOk` is sent (around line 320-350) and change:
```rust
// OLD - only sent hint
let mut session_hint = [0u8; 8];
session_hint.copy_from_slice(&room.session_key.0[0..8]);
send(app, ws, &ServerToClient::JoinOk {
    room_id: rid.clone(),
    player_id: pid.clone(),
    session_key_hint: session_hint,
    // ...
}).await?;

// NEW - send full key (already encrypted by TLS/WSS)
send(app, ws, &ServerToClient::JoinOk {
    room_id: rid.clone(),
    player_id: pid.clone(),
    session_key: room.session_key.clone(),
    // ...
}).await?;
```

Update protocol message in `aw-net-proto/src/lib.rs`:
```rust
JoinOk {
    room_id: String,
    player_id: String,
    session_key: SessionKey,  // Changed from session_key_hint: [u8; 8]
    tick_hz: u32,
},
```

## Step 7: Update Client to Store Session Key

**File: `net/aw-net-client/src/main.rs`**

After receiving `JoinOk`, store the full key:
```rust
ServerToClient::JoinOk {
    room_id,
    player_id,
    session_key,  // Changed from session_key_hint
    tick_hz,
} => {
    info!("Joined room={room_id} pid={player_id} hz={tick_hz}");
    let session_key_stored = session_key.0;  // Store [u8; 32]
    // Use session_key_stored for signing messages
}
```

## Testing

```bash
# Build all
cd net
cargo build --all

# Run tests (add to aw-net-proto/src/lib.rs)
cargo test --package aw-net-proto

# Start server
cd aw-net-server
cargo run

# Run client in another terminal
cd aw-net-client
cargo run
```

## Security Checklist

- [ ] Added `hmac` and `sha2` dependencies
- [ ] Replaced `sign16` with `hmac_sign` and `hmac_verify`
- [ ] Added `timestamp_ms` to `InputFrame` message
- [ ] Server validates HMAC signature
- [ ] Server checks timestamp window (±5 seconds)
- [ ] Server checks sequence number ordering
- [ ] Client sends current timestamp with each message
- [ ] Client uses full 32-byte session key (not 8-byte hint)
- [ ] Server sends full session key over TLS/WSS
- [ ] Constant-time comparison used (via `verify_slice`)
- [ ] Tests added for sign/verify roundtrip
- [ ] Tests added for replay detection

## Key Security Improvements

| Feature | Old (sign16) | New (HMAC-SHA256) |
|---------|--------------|-------------------|
| Hash | Non-crypto | SHA-256 (crypto) |
| Key Size | 8 bytes | 32 bytes |
| Security | ~0 bits | ~128 bits |
| Replay Protection | None | Timestamp + Seq |
| Timing Attacks | Vulnerable | Protected |
| Standard | None | RFC 2104 |
