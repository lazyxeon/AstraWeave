#![forbid(unsafe_code)]
use hmac::Mac;
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROTOCOL_VERSION: u16 = 1;

/// Length in bytes of an HMAC-SHA256 signature tag.
pub const SIG_LEN: usize = 32;

type HmacSha256 = hmac::Hmac<sha2::Sha256>;

/// Shared symmetric signing key (32 bytes).
///
/// Keys the HMAC-SHA256 signatures carried in [`ClientToServer::InputFrame`].
/// Both ends of the connection must hold the same key out-of-band; the key is
/// never transmitted on the wire.
///
/// `Debug` is intentionally redacted — key material must never be printable.
///
/// The 32-byte field is private: the only paths to the raw bytes are the
/// explicit constructor [`SigningKey::from_bytes`] and the explicit accessor
/// [`SigningKey::as_bytes`]. Keeping the field private prevents callers from
/// reaching the bytes via a tuple-`.0` access and printing them, which would
/// defeat the redacted `Debug` impl below.
#[derive(Clone)]
pub struct SigningKey([u8; 32]);

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SigningKey(<redacted>)")
    }
}

impl SigningKey {
    /// Construct a signing key from raw bytes.
    ///
    /// This is the explicit, audited byte constructor. Prefer
    /// [`Self::from_hex`] for operator-supplied configuration; use this only
    /// where 32 bytes of key material already exist in memory.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        SigningKey(bytes)
    }

    /// Borrow the raw 32-byte key material.
    ///
    /// This is the single audited path to the key bytes (HMAC keying needs
    /// them). **Callers must never log, format, print, or otherwise emit the
    /// returned bytes** — doing so leaks the secret and defeats the redacted
    /// [`Debug`](std::fmt::Debug) impl. Every call site of this method is a
    /// place to scrutinise for accidental disclosure.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Parse a signing key from exactly 64 hexadecimal characters (32 bytes).
    ///
    /// **Timing note (accepted boundary):** the hex parse uses
    /// [`hex::decode`], which is not constant-time. This is acceptable because
    /// the key material is operator-supplied, out-of-band configuration parsed
    /// exactly once at process startup (from the `AW_SHARED_KEY` environment
    /// variable or the `--shared-key-hex` argument), never from
    /// attacker-reachable input. There is no chosen-input timing oracle here,
    /// so `hex::decode`'s non-constant-time behaviour is an accepted boundary
    /// rather than a vulnerability; a hand-rolled constant-time hex decoder
    /// would add fragile crypto-adjacent code for negligible benefit.
    pub fn from_hex(s: &str) -> Result<Self, WireError> {
        if s.len() != 64 {
            return Err(WireError::InvalidSigningKey(format!(
                "expected exactly 64 hex characters (32 bytes), got {}",
                s.len()
            )));
        }
        let bytes = hex::decode(s)
            .map_err(|e| WireError::InvalidSigningKey(format!("invalid hex: {e}")))?;
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| WireError::InvalidSigningKey("decoded key is not 32 bytes".to_string()))?;
        Ok(SigningKey(key))
    }

    /// Fixed development-only signing key.
    ///
    /// **DEVELOPMENT ONLY — NOT FOR PRODUCTION.** This key is a compile-time
    /// constant published in source; it provides zero secrecy. It exists so
    /// that client and server share exactly ONE definition of the out-of-box
    /// key instead of two drifting hardcoded constants. Production deployments
    /// must distribute a real key out-of-band and load it via [`Self::from_hex`].
    pub fn dev_default() -> Self {
        // 32 ASCII bytes, self-describing.
        SigningKey(*b"ASTRAWEAVE-DEV-KEY-NOT-FOR-PROD!")
    }
}

/// Compute HMAC-SHA256 of `payload` keyed by `key` (arbitrary length).
///
/// This is the core primitive backing [`sign`]/[`verify`]; it accepts an
/// arbitrary-length key so it can be validated directly against the RFC 4231
/// known-answer test vectors (which use non-32-byte keys).
pub fn hmac_sha256(key: &[u8], payload: &[u8]) -> [u8; 32] {
    // INFALLIBLE: HMAC accepts keys of any length (RFC 2104 §2 — keys longer
    // than the block size are hashed, shorter keys are zero-padded), so
    // `new_from_slice` cannot return `InvalidLength` for Hmac<Sha256>.
    let mut mac = HmacSha256::new_from_slice(key)
        .expect("HMAC-SHA256 accepts keys of any length; InvalidLength is unreachable");
    mac.update(payload);
    mac.finalize().into_bytes().into()
}

/// Sign `payload` with HMAC-SHA256 keyed by `key`.
pub fn sign(key: &SigningKey, payload: &[u8]) -> [u8; 32] {
    hmac_sha256(&key.0, payload)
}

/// Verify an HMAC-SHA256 `tag` over `payload` keyed by `key`.
///
/// Uses the `hmac` crate's constant-time comparison ([`Mac::verify_slice`]).
/// Never compare MAC tags with `==`/byte-equality — that leaks timing
/// information about how many prefix bytes of the tag are correct.
pub fn verify(key: &SigningKey, payload: &[u8], tag: &[u8; 32]) -> bool {
    // INFALLIBLE: see `hmac_sha256` — HMAC accepts keys of any length.
    let mut mac = HmacSha256::new_from_slice(&key.0)
        .expect("HMAC-SHA256 accepts keys of any length; InvalidLength is unreachable");
    mac.update(payload);
    mac.verify_slice(tag).is_ok()
}

/// Build THE canonical MAC'd byte range for [`ClientToServer::InputFrame`].
///
/// Layout: `seq.to_le_bytes() ++ tick_ms.to_le_bytes() ++ input_blob`
/// (4-byte little-endian `seq`, 8-byte little-endian `tick_ms`, then the raw
/// input blob as the unambiguous variable-length tail).
///
/// Both the client (when signing) and the server (when verifying) MUST build
/// the payload via this function — never hand-roll the concatenation. A single
/// shared definition is what prevents signed-byte-range divergence between the
/// two ends of the connection.
pub fn input_frame_sig_payload(seq: u32, tick_ms: u64, input_blob: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(4 + 8 + input_blob.len());
    payload.extend_from_slice(&seq.to_le_bytes());
    payload.extend_from_slice(&tick_ms.to_le_bytes());
    payload.extend_from_slice(input_blob);
    payload
}

/// Simple wire messages (focus on MVP end-to-end).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ClientToServer {
    Hello {
        protocol: u16,
    },
    /// Ask matchmaker for (or create) a room in a region.
    FindOrCreate {
        region: String,
        game_mode: String,
        party_size: u8,
    },
    /// Join a specific room if known (room_id from matchmaker).
    JoinRoom {
        room_id: String,
        display_name: String,
    },
    /// Per-frame input payload (prediction).
    InputFrame {
        seq: u32,
        tick_ms: u64,
        // e.g. movement vector, buttons; opaque to engine:
        input_blob: Vec<u8>,
        /// HMAC-SHA256 tag over [`input_frame_sig_payload`]`(seq, tick_ms,
        /// &input_blob)`, keyed by the shared [`SigningKey`].
        sig: [u8; 32],
    },
    /// Reliable pings for RTT estimate
    Ping {
        nano: u128,
    },
    /// Client acknowledges snapshot / reconciliation id
    Ack {
        last_snapshot_id: u32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ServerToClient {
    HelloAck {
        protocol: u16,
    },
    MatchResult {
        room_id: String,
    },
    JoinAccepted {
        room_id: String,
        player_id: String,
        tick_hz: u32,
    },
    /// Snapshot can contain either a full or delta state (opaque to engine)
    Snapshot {
        id: u32,
        server_tick: u64,
        base_id: Option<u32>,
        compressed: bool,
        payload: Vec<u8>, // engine-controlled data (bincode/postcard)
    },
    /// Correction vector for client reconciliation
    Reconcile {
        input_seq_ack: u32,
        corrected_state_hash: u64,
    },
    Pong {
        nano: u128,
    },
    /// Basic moderation / anti-cheat feedback
    RateLimited,
    ProtocolError {
        msg: String,
    },
}

#[derive(Debug, Error)]
#[non_exhaustive]
#[must_use]
pub enum WireError {
    #[error("protocol mismatch (client={client}, server={server})")]
    ProtocolMismatch { client: u16, server: u16 },
    #[error("decode error: {0}")]
    Decode(String),
    #[error("invalid signing key: {0}")]
    InvalidSigningKey(String),
}

#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum Codec {
    /// Compact CBOR-like; great for small messages.
    PostcardLz4,
    /// Fallback / compatibility
    Bincode,
}

pub fn encode_msg(codec: Codec, msg: &impl Serialize) -> Vec<u8> {
    match codec {
        Codec::PostcardLz4 => {
            let raw = postcard::to_allocvec(msg).expect("serialize");
            lz4_flex::compress_prepend_size(&raw)
        }
        Codec::Bincode => {
            use bincode::config::standard;
            use bincode::serde::encode_to_vec;
            encode_to_vec(msg, standard()).expect("serialize")
        }
    }
}

pub fn decode_msg<T: for<'de> Deserialize<'de>>(
    codec: Codec,
    bytes: &[u8],
) -> Result<T, WireError> {
    match codec {
        Codec::PostcardLz4 => {
            let decompressed = lz4_flex::decompress_size_prepended(bytes)
                .map_err(|e| WireError::Decode(format!("lz4: {e}")))?;
            postcard::from_bytes(&decompressed)
                .map_err(|e| WireError::Decode(format!("postcard: {e}")))
        }
        Codec::Bincode => {
            use bincode::config::standard;
            use bincode::serde::decode_from_slice;
            let (val, _len) = decode_from_slice(bytes, standard())
                .map_err(|e| WireError::Decode(format!("bincode: {e}")))?;
            Ok(val)
        }
    }
}

/// Generate a short, URL-safe room id.
pub fn new_room_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect()
}
