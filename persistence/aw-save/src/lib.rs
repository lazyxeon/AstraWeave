#![forbid(unsafe_code)]
//! aw-save: versioned, atomic, checksummed, compressed save files for AstraWeave.
//!
//! Design goals:
//! - **Server/Client Friendly**: works for single-player or server-authoritative co-op.
//! - **Versioned**: schema u16; explicit migrations.
//! - **Atomic Writes**: write .tmp, fsync, then rename.
//! - **Data Integrity**: CRC32 over compressed payload.
//! - **Compression**: LZ4 (fast) around postcard payload.
//! - **Opaque ECS Blob**: engine can pack an arbitrary world snapshot blob.
//!
//! File format (.awsv):
//! magic[4]="ASVS" | version u16 | codec u8(1=LZ4) | reserved u8=0 | data_len u32 | crc32 u32 | data[data_len]
//! where `data` is postcard-serialized SaveBundleV{N} and compressed with LZ4 if codec==1.

use anyhow::{bail, Context, Result};
use crc32fast::Hasher as Crc32;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use time::{format_description, OffsetDateTime};
use uuid::Uuid;

const MAGIC: &[u8; 4] = b"ASVS";
const CODEC_LZ4: u8 = 1;
/// Compressed payload cap. Checked before the buffer is allocated.
/// The hostile-header proof claims 8 MiB, so this stays below that.
pub const MAX_AWSV_COMPRESSED_BYTES: usize = 4 * 1024 * 1024;
/// Uncompressed LZ4 output cap. Checked before `decompress_size_prepended` reserves it.
pub const MAX_AWSV_UNCOMPRESSED_BYTES: usize = 16 * 1024 * 1024;
/// Bump this when you change SaveBundle layout. Add explicit migrations below.
pub const SAVE_SCHEMA_VERSION: u16 = 2;

/// Public, stable entrypoint
#[derive(Debug, Clone)]
pub struct SaveManager {
    root: PathBuf,
}

impl SaveManager {
    /// Create a manager rooted at `<root>/<player_id>/`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Compute the directory for a player.
    pub fn player_dir(&self, player_id: &str) -> PathBuf {
        self.root.join(sanitize(player_id))
    }

    /// Save bundle to a slot (0..=255) or named id. Returns the file path.
    pub fn save(&self, player_id: &str, slot: u8, bundle: SaveBundleV2) -> Result<PathBuf> {
        let dir = self.player_dir(player_id);
        fs::create_dir_all(&dir)?;
        // Use Windows-safe timestamp format (replace colons with dashes)
        let format = format_description::parse("[year]-[month]-[day]T[hour]-[minute]-[second]Z")
            .unwrap_or_else(|_| {
                format_description::parse("[year][month][day]T[hour][minute][second]Z").unwrap()
            });
        let stamp = OffsetDateTime::now_utc()
            .format(&format)
            .unwrap_or_else(|_| "now".into());
        let fname = format!("slot{:02}_{}_{}.awsv", slot, stamp, bundle.save_id);
        let path = dir.join(fname);
        write_awsv(&path, &bundle)?;
        // Also record/update an index.json for quick lookup
        write_or_update_index(&dir, &bundle, &path)?;
        Ok(path)
    }

    /// Load the *latest* file for a slot, or any file path directly.
    pub fn load_latest_slot(&self, player_id: &str, slot: u8) -> Result<(SaveBundleV2, PathBuf)> {
        let dir = self.player_dir(player_id);
        let mut candidates: Vec<_> = fs::read_dir(&dir)
            .context(format!("save directory not found: {}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "awsv").unwrap_or(false))
            .filter(|p| {
                p.file_name()
                    .map(|s| {
                        s.to_string_lossy()
                            .starts_with(&format!("slot{:02}_", slot))
                    })
                    .unwrap_or(false)
            })
            .collect();
        candidates.sort(); // lexicographic includes timestamp in name
        let path = candidates.last().cloned().context("no save for slot")?;
        let bundle = read_awsv(&path)?;
        Ok((bundle, path))
    }

    pub fn list_saves(&self, player_id: &str) -> Result<Vec<SaveMeta>> {
        read_index(&self.player_dir(player_id))
            .or_else(|_| scan_dir_for_meta(&self.player_dir(player_id)))
    }

    /// Migration: read any old file and produce current V2 bundle; optionally resave.
    pub fn migrate_file_to_latest(&self, path: &Path, resave: bool) -> Result<SaveBundleV2> {
        let AnySave { version, blob } = read_any_version(path)?;
        let v2 = match version {
            1 => {
                let v1: SaveBundleV1 = postcard::from_bytes(&blob).context("decode v1")?;
                v1.into_v2()
            }
            2 => postcard::from_bytes::<SaveBundleV2>(&blob).context("decode v2")?,
            other => bail!("unknown save version: {}", other),
        };
        if resave {
            write_awsv(path, &v2)?;
        }
        Ok(v2)
    }
}

/// What's inside the postcard payload (CURRENT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveBundleV2 {
    pub schema: u16,                // == SAVE_SCHEMA_VERSION
    pub save_id: Uuid,              // unique id for this save
    pub created_at: OffsetDateTime, // when file was created
    pub player_id: String,          // stable user id
    pub slot: u8,                   // user-visible slot (00..)
    // ---- Game data (engine owns the shape of inner fields) ----
    pub world: WorldState, // ECS/world snapshot container
    pub companions: Vec<CompanionProfile>,
    pub inventory: PlayerInventory,
    // Additional (free-form) metadata for future
    pub meta: HashMap<String, String>,
}

/// ECS/world snapshot container (opaque blob to engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub tick: u64,
    /// Arbitrary engine snapshot (e.g., bincode of ECS, nav, etc.), likely already compressed.
    pub ecs_blob: Vec<u8>,
    /// Optional hash for quick equality check
    pub state_hash: u64,
}

/// Player inventory (example)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInventory {
    pub credits: u64,
    pub items: Vec<ItemStack>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    pub kind: String,
    pub qty: u32,
    pub attrs: HashMap<String, i64>,
}

/// Companion profile (example; keep this aligned with your `.cprof` concepts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionProfile {
    pub id: String,
    pub name: String,
    pub level: u8,
    pub skills: Vec<String>,
    pub facts: Vec<String>,
    pub episodes_summarized: Vec<String>,
}

// --------- V1 schema + migration ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveBundleV1 {
    pub player_id: String,
    pub slot: u8,
    pub created_at: OffsetDateTime,
    pub world: WorldState,
    pub inventory: PlayerInventory,
    // V1 had a single companion (example legacy)
    pub companion: Option<CompanionProfile>,
    pub meta: HashMap<String, String>,
}

impl SaveBundleV1 {
    pub fn into_v2(self) -> SaveBundleV2 {
        SaveBundleV2 {
            schema: SAVE_SCHEMA_VERSION,
            save_id: Uuid::new_v4(),
            created_at: self.created_at,
            player_id: self.player_id,
            slot: self.slot,
            world: self.world,
            companions: self.companion.into_iter().collect(),
            inventory: self.inventory,
            meta: self.meta,
        }
    }
}

// --------- On-disk index (quality-of-life) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMeta {
    pub save_id: Uuid,
    pub file: String,
    pub created_at: OffsetDateTime,
    pub player_id: String,
    pub slot: u8,
    pub schema: u16,
}

fn write_or_update_index(dir: &Path, v2: &SaveBundleV2, file_path: &Path) -> Result<()> {
    let mut list = read_index(dir).unwrap_or_default();
    list.retain(|m| m.save_id != v2.save_id);
    list.push(SaveMeta {
        save_id: v2.save_id,
        file: file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        created_at: v2.created_at,
        player_id: v2.player_id.clone(),
        slot: v2.slot,
        schema: v2.schema,
    });
    list.sort_by_key(|m| (m.slot, m.created_at));
    let idx = serde_json::to_vec_pretty(&list)?;
    fs::write(dir.join("index.json"), idx)?;
    Ok(())
}
fn read_index(dir: &Path) -> Result<Vec<SaveMeta>> {
    let p = dir.join("index.json");
    let bytes = fs::read(p)?;
    Ok(serde_json::from_slice(&bytes)?)
}
fn scan_dir_for_meta(dir: &Path) -> Result<Vec<SaveMeta>> {
    let mut out = vec![];
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.extension().map(|e| e == "awsv").unwrap_or(false) {
            if let Ok(v2) = read_awsv(&p) {
                out.push(SaveMeta {
                    save_id: v2.save_id,
                    file: p.file_name().unwrap().to_string_lossy().into_owned(),
                    created_at: v2.created_at,
                    player_id: v2.player_id.clone(),
                    slot: v2.slot,
                    schema: v2.schema,
                });
            }
        }
    }
    out.sort_by_key(|m| (m.slot, m.created_at));
    Ok(out)
}

// --------- File format I/O (atomic, checksummed, compressed) ----------

fn write_awsv(path: &Path, v2: &SaveBundleV2) -> Result<()> {
    let payload = postcard::to_allocvec(v2)?;
    // compress
    let payload = lz4_flex::compress_prepend_size(&payload);
    let mut crc = Crc32::new();
    crc.update(&payload);
    let crc = crc.finalize();

    let mut buf: Vec<u8> = Vec::with_capacity(4 + 2 + 1 + 1 + 4 + 4 + payload.len());
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&SAVE_SCHEMA_VERSION.to_le_bytes());
    buf.push(CODEC_LZ4);
    buf.push(0); // reserved
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(&crc.to_le_bytes());
    buf.extend_from_slice(&payload);

    // atomic write in the same directory
    let tmp = path.with_extension("tmp");
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(&buf)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

fn read_awsv(path: &Path) -> Result<SaveBundleV2> {
    bundle_from_any(read_any_version(path)?)
}

/// Decode an `.awsv` blob from memory. Same checks as the file loader.
pub fn decode_awsv_bytes(bytes: &[u8]) -> Result<SaveBundleV2> {
    bundle_from_any(read_any_from_slice(bytes)?)
}

fn bundle_from_any(any: AnySave) -> Result<SaveBundleV2> {
    match any.version {
        2 => Ok(postcard::from_bytes::<SaveBundleV2>(&any.blob)?),
        1 => {
            let v1: SaveBundleV1 = postcard::from_bytes(&any.blob)?;
            Ok(v1.into_v2())
        }
        other => bail!("unknown save version {other}"),
    }
}

struct AnySave {
    version: u16,
    blob: Vec<u8>,
}

fn read_any_version(path: &Path) -> Result<AnySave> {
    let mut f = File::open(path)?;
    let mut header = [0u8; 16];
    f.read_exact(&mut header)?;
    let parsed = parse_awsv_header(&header)?;
    if parsed.len > MAX_AWSV_COMPRESSED_BYTES {
        bail!(
            "declared payload length exceeds cap ({} > {MAX_AWSV_COMPRESSED_BYTES})",
            parsed.len
        );
    }
    let mut payload = vec![0u8; parsed.len];
    f.read_exact(&mut payload)?;
    finish_awsv(parsed, &payload)
}

fn read_any_from_slice(bytes: &[u8]) -> Result<AnySave> {
    if bytes.len() < 16 {
        bail!("awsv header is truncated");
    }
    let mut header = [0u8; 16];
    header.copy_from_slice(&bytes[..16]);
    let parsed = parse_awsv_header(&header)?;
    if parsed.len > MAX_AWSV_COMPRESSED_BYTES {
        bail!(
            "declared payload length exceeds cap ({} > {MAX_AWSV_COMPRESSED_BYTES})",
            parsed.len
        );
    }
    let rest = &bytes[16..];
    if rest.len() < parsed.len {
        bail!("awsv payload is truncated");
    }
    finish_awsv(parsed, &rest[..parsed.len])
}

#[derive(Clone, Copy)]
struct AwsvHeader {
    version: u16,
    codec: u8,
    len: usize,
    crc: u32,
}

fn parse_awsv_header(header: &[u8; 16]) -> Result<AwsvHeader> {
    if &header[0..4] != MAGIC {
        bail!("bad magic");
    }
    Ok(AwsvHeader {
        version: u16::from_le_bytes([header[4], header[5]]),
        codec: header[6],
        len: u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize,
        crc: u32::from_le_bytes([header[12], header[13], header[14], header[15]]),
    })
}

fn finish_awsv(parsed: AwsvHeader, payload: &[u8]) -> Result<AnySave> {
    let AwsvHeader {
        version,
        codec,
        len: _,
        crc,
    } = parsed;

    // verify CRC
    let mut h = Crc32::new();
    h.update(payload);
    let got = h.finalize();
    if got != crc {
        bail!("crc mismatch: expected {crc}, got {got}");
    }
    // decompress if needed
    let data = match codec {
        CODEC_LZ4 => decompress_lz4_capped(payload)?,
        _ => bail!("unknown codec {codec}"),
    };
    Ok(AnySave {
        version,
        blob: data,
    })
}

// --------- Helpers ----------

fn decompress_lz4_capped(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.len() >= 4 {
        let declared =
            u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
        if declared > MAX_AWSV_UNCOMPRESSED_BYTES {
            bail!(
                "declared uncompressed size exceeds cap ({declared} > {MAX_AWSV_UNCOMPRESSED_BYTES})"
            );
        }
    }
    Ok(lz4_flex::decompress_size_prepended(payload)?)
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_safe_timestamp() {
        // Test that the timestamp format doesn't contain invalid Windows filename characters
        let format = format_description::parse("[year]-[month]-[day]T[hour]-[minute]-[second]Z")
            .unwrap_or_else(|_| {
                format_description::parse("[year][month][day]T[hour][minute][second]Z").unwrap()
            });
        let stamp = OffsetDateTime::now_utc()
            .format(&format)
            .unwrap_or_else(|_| "now".into());

        println!("Generated timestamp: {}", stamp);

        // Windows invalid filename characters: < > : " | ? *
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        let has_invalid_chars = stamp.chars().any(|c| invalid_chars.contains(&c));

        assert!(
            !has_invalid_chars,
            "Timestamp '{}' contains invalid Windows filename characters",
            stamp
        );
        assert!(
            !stamp.contains(':'),
            "Timestamp '{}' should not contain colons for Windows compatibility",
            stamp
        );

        // Verify it's still a reasonable timestamp format (contains expected separators)
        assert!(
            stamp.contains('-'),
            "Timestamp '{}' should contain date separators",
            stamp
        );
        assert!(
            stamp.contains('T'),
            "Timestamp '{}' should contain T separator",
            stamp
        );
        assert!(
            stamp.ends_with('Z'),
            "Timestamp '{}' should end with Z",
            stamp
        );
    }

    /// SEC-POC SEC-SAVE-1. A hostile `.awsv` header can name a multi-megabyte
    /// payload that is not in the file. The loader must reject that length
    /// as over the cap. Today it allocates the declared buffer and then fails
    /// the short read.
    #[test]

    fn fixed_bundle() -> SaveBundleV2 {
        SaveBundleV2 {
            schema: SAVE_SCHEMA_VERSION,
            save_id: Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap(),
            created_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
            player_id: "aravath".into(),
            slot: 0,
            world: WorldState {
                tick: 7,
                ecs_blob: vec![1, 2, 3, 4],
                state_hash: 0x0a0b,
            },
            companions: Vec::new(),
            inventory: PlayerInventory {
                credits: 0,
                items: Vec::new(),
            },
            meta: HashMap::new(),
        }
    }

    #[test]
    fn postcard_and_lz4_bytes_are_stable() {
        let bytes = postcard::to_allocvec(&fixed_bundle()).unwrap();
        let again = postcard::to_allocvec(&fixed_bundle()).unwrap();
        assert_eq!(bytes, again, "postcard encoding drifted");
        assert_eq!(
            hex::encode(&bytes),
            "021011111111222233334444555555555555ce1fbe02160d14000000000761726176617468000704010203048b1400000000"
        );
        let compressed = lz4_flex::compress_prepend_size(&bytes);
        let compressed_again = lz4_flex::compress_prepend_size(&bytes);
        assert_eq!(compressed, compressed_again, "lz4 output drifted");
        assert_eq!(
            hex::encode(&compressed),
            "32000000d1021011111111222233334444550100f011ce1fbe02160d14000000000761726176617468000704010203048b1400000000"
        );
        let restored = decompress_lz4_capped(&compressed).unwrap();
        assert_eq!(restored, bytes);
    }

    fn poc_awsv_rejects_oversized_declared_payload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("hostile.awsv");
        let claimed: u32 = 8 * 1024 * 1024;
        let mut header = Vec::new();
        header.extend_from_slice(b"ASVS");
        header.extend_from_slice(&2u16.to_le_bytes());
        header.push(CODEC_LZ4);
        header.push(0);
        header.extend_from_slice(&claimed.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
        std::fs::write(&path, &header).expect("write header");

        let err = read_awsv(&path).expect_err("hostile header must not load");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("too large") || msg.contains("exceeds"),
            "declared payload length was not rejected by a size cap: {msg}"
        );
    }
}
