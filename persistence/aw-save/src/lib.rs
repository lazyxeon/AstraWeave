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
/// Bump this when you change SaveBundle layout. Add explicit migrations below.
pub const SAVE_SCHEMA_VERSION: u16 = 2;
/// Compressed payload cap. A header that asks for more is rejected before allocation.
pub const MAX_COMPRESSED_BYTES: usize = 32 * 1024 * 1024;
/// Uncompressed postcard cap. Checked against the LZ4 size prefix before decompress.
pub const MAX_UNCOMPRESSED_BYTES: usize = 64 * 1024 * 1024;
/// `index.json` is a slot catalog, not a world snapshot.
pub const MAX_INDEX_BYTES: u64 = 1024 * 1024;
pub const MAX_INDEX_ENTRIES: usize = 4_096;
pub const MAX_PLAYER_ID_LEN: usize = 64;

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
        let player_id = validate_player_id(player_id)?;
        let dir = self.root.join(player_id);
        fs::create_dir_all(&dir)?;
        let stamp = timestamp()?;
        let fname = format!("slot{:02}_{}_{}.awsv", slot, stamp, bundle.save_id);
        validate_save_file_name(&fname)?;
        let path = dir.join(fname);
        write_awsv(&path, &bundle)?;
        // Also record/update an index.json for quick lookup
        write_or_update_index(&dir, &bundle, &path)?;
        Ok(path)
    }

    /// Load the *latest* file for a slot, or any file path directly.
    pub fn load_latest_slot(&self, player_id: &str, slot: u8) -> Result<(SaveBundleV2, PathBuf)> {
        let player_id = validate_player_id(player_id)?;
        let dir = self.root.join(player_id);
        reject_symlink(&dir, "save directory")?;
        let prefix = format!("slot{:02}_", slot);
        let mut candidates: Vec<_> = fs::read_dir(&dir)
            .context(format!("save directory not found: {}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".awsv"))
            })
            .collect();
        candidates.sort(); // lexicographic includes timestamp in name
        let path = candidates.last().cloned().context("no save for slot")?;
        ensure_inside(&dir, &path)?;
        let bundle = read_awsv(&path)?;
        Ok((bundle, path))
    }

    pub fn list_saves(&self, player_id: &str) -> Result<Vec<SaveMeta>> {
        let player_id = validate_player_id(player_id)?;
        let dir = self.root.join(player_id);
        read_index(&dir).or_else(|_| scan_dir_for_meta(&dir))
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
    let index_path = dir.join("index.json");
    if let Ok(meta) = fs::symlink_metadata(&index_path) {
        if meta.file_type().is_symlink() {
            bail!("refusing to write through a symlinked index.json");
        }
    }
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .context("save path has no file name")?
        .to_string();
    validate_save_file_name(&file_name)?;
    let mut list = read_index(dir).unwrap_or_default();
    list.retain(|m| m.save_id != v2.save_id);
    list.push(SaveMeta {
        save_id: v2.save_id,
        file: file_name,
        created_at: v2.created_at,
        player_id: v2.player_id.clone(),
        slot: v2.slot,
        schema: v2.schema,
    });
    if list.len() > MAX_INDEX_ENTRIES {
        bail!("save index has more than {MAX_INDEX_ENTRIES} entries");
    }
    list.sort_by_key(|m| (m.slot, m.created_at));
    let idx = serde_json::to_vec_pretty(&list)?;
    if idx.len() as u64 > MAX_INDEX_BYTES {
        bail!("save index exceeds {MAX_INDEX_BYTES} bytes");
    }
    fs::write(index_path, idx)?;
    Ok(())
}
fn read_index(dir: &Path) -> Result<Vec<SaveMeta>> {
    let p = dir.join("index.json");
    let meta = fs::symlink_metadata(&p).context("read index.json")?;
    if meta.file_type().is_symlink() {
        bail!("refusing to read a symlinked index.json");
    }
    if !meta.is_file() {
        bail!("index.json is not a regular file");
    }
    if meta.len() > MAX_INDEX_BYTES {
        bail!(
            "index.json is {} bytes; limit is {MAX_INDEX_BYTES}",
            meta.len()
        );
    }
    let bytes = fs::read(&p)?;
    if bytes.len() as u64 > MAX_INDEX_BYTES {
        bail!("index.json exceeds {MAX_INDEX_BYTES} bytes");
    }
    let list: Vec<SaveMeta> = serde_json::from_slice(&bytes).context("parse index.json")?;
    if list.len() > MAX_INDEX_ENTRIES {
        bail!("index.json has more than {MAX_INDEX_ENTRIES} entries");
    }
    for entry in &list {
        validate_save_file_name(&entry.file)?;
    }
    Ok(list)
}
fn scan_dir_for_meta(dir: &Path) -> Result<Vec<SaveMeta>> {
    let mut out = vec![];
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.extension().map(|e| e == "awsv").unwrap_or(false) {
            if let Ok(v2) = read_awsv(&p) {
                let Some(file) = p.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if validate_save_file_name(file).is_err() {
                    continue;
                }
                out.push(SaveMeta {
                    save_id: v2.save_id,
                    file: file.to_string(),
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
    let AnySave { version, blob } = read_any_version(path)?;
    match version {
        2 => Ok(postcard::from_bytes::<SaveBundleV2>(&blob)?),
        1 => {
            let v1: SaveBundleV1 = postcard::from_bytes(&blob)?;
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
    reject_symlink(path, "save file")?;
    let file_len = fs::metadata(path)?.len();
    // Header is 16 bytes. Reject a declared payload that cannot fit in the file
    // before allocating it.
    if file_len < 16 || file_len > 16 + MAX_COMPRESSED_BYTES as u64 {
        bail!(
            "save file is {file_len} bytes; limit is {}",
            16 + MAX_COMPRESSED_BYTES
        );
    }
    let mut f = File::open(path)?;
    let mut header = [0u8; 4 + 2 + 1 + 1 + 4 + 4];
    f.read_exact(&mut header)?;
    if &header[0..4] != MAGIC {
        bail!("bad magic");
    }
    let version = u16::from_le_bytes([header[4], header[5]]);
    let codec = header[6];
    let _reserved = header[7];
    let len = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
    let crc = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
    if len > MAX_COMPRESSED_BYTES {
        bail!("compressed save is {len} bytes; limit is {MAX_COMPRESSED_BYTES}");
    }
    let mut payload = vec![0u8; len];
    f.read_exact(&mut payload)?;

    // verify CRC
    let mut h = Crc32::new();
    h.update(&payload);
    let got = h.finalize();
    if got != crc {
        bail!("crc mismatch: expected {crc}, got {got}");
    }
    // decompress if needed
    let data = match codec {
        CODEC_LZ4 => decompress_capped(&payload)?,
        _ => bail!("unknown codec {codec}"),
    };
    Ok(AnySave {
        version,
        blob: data,
    })
}

fn decompress_capped(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.len() < 4 {
        bail!("lz4 payload is shorter than its size prefix");
    }
    let prefix: [u8; 4] = payload[0..4]
        .try_into()
        .map_err(|_| anyhow::anyhow!("lz4 size prefix"))?;
    let declared = u32::from_le_bytes(prefix) as usize;
    if declared > MAX_UNCOMPRESSED_BYTES {
        bail!("uncompressed save is {declared} bytes; limit is {MAX_UNCOMPRESSED_BYTES}");
    }
    let data = lz4_flex::decompress_size_prepended(payload).context("lz4 decompress save")?;
    if data.len() > MAX_UNCOMPRESSED_BYTES {
        bail!("decompressed save exceeds {MAX_UNCOMPRESSED_BYTES} bytes");
    }
    Ok(data)
}

// --------- Helpers ----------

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(MAX_PLAYER_ID_LEN)
        .collect()
}

fn validate_player_id(player_id: &str) -> Result<String> {
    if player_id.is_empty() || player_id.len() > MAX_PLAYER_ID_LEN {
        bail!("player id must be 1..={MAX_PLAYER_ID_LEN} bytes");
    }
    if player_id.contains("..")
        || player_id.contains('/')
        || player_id.contains('\\')
        || player_id.contains(':')
        || player_id.contains('\0')
    {
        bail!("player id contains a path separator");
    }
    let cleaned = sanitize(player_id);
    if cleaned != player_id {
        bail!("player id must be ASCII letters, digits, '-' or '_'");
    }
    if is_reserved_device(&cleaned) {
        bail!("player id is a reserved Windows device name");
    }
    Ok(cleaned)
}

fn is_reserved_device(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

fn validate_save_file_name(name: &str) -> Result<()> {
    if name.len() > 180 || !name.ends_with(".awsv") {
        bail!("save file name is not an .awsv file");
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') || name.contains(':') {
        bail!("save file name escapes the save directory");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        bail!("save file name has illegal characters");
    }
    Ok(())
}

fn timestamp() -> Result<String> {
    let format =
        format_description::parse_borrowed::<2>("[year]-[month]-[day]T[hour]-[minute]-[second]Z")
            .context("timestamp format")?;
    OffsetDateTime::now_utc()
        .format(&format)
        .context("format timestamp")
}

fn reject_symlink(path: &Path, what: &str) -> Result<()> {
    let meta =
        fs::symlink_metadata(path).with_context(|| format!("stat {what} {}", path.display()))?;
    if meta.file_type().is_symlink() {
        bail!("refusing to follow symlinked {what} {}", path.display());
    }
    Ok(())
}

fn ensure_inside(dir: &Path, file: &Path) -> Result<()> {
    let dir = dir
        .canonicalize()
        .with_context(|| format!("canonicalize {}", dir.display()))?;
    let file = file
        .canonicalize()
        .with_context(|| format!("canonicalize {}", file.display()))?;
    if !file.starts_with(&dir) {
        bail!("save file {} is outside {}", file.display(), dir.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_safe_timestamp() {
        // Test that the timestamp format doesn't contain invalid Windows filename characters
        let format = format_description::parse_borrowed::<2>(
            "[year]-[month]-[day]T[hour]-[minute]-[second]Z",
        )
        .expect("timestamp format");
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

    fn sample_bundle() -> SaveBundleV2 {
        SaveBundleV2 {
            schema: SAVE_SCHEMA_VERSION,
            save_id: Uuid::new_v4(),
            created_at: OffsetDateTime::now_utc(),
            player_id: "hero".into(),
            slot: 0,
            world: WorldState {
                tick: 1,
                ecs_blob: vec![1, 2, 3],
                state_hash: 7,
            },
            companions: Vec::new(),
            inventory: PlayerInventory {
                credits: 0,
                items: Vec::new(),
            },
            meta: Default::default(),
        }
    }

    #[test]
    fn player_ids_cannot_escape_the_save_root() {
        let dir = tempfile::tempdir().unwrap();
        let sm = SaveManager::new(dir.path());
        let bundle = sample_bundle();
        for id in [
            "",
            "..",
            "../etc",
            "a/b",
            r"a\b",
            "CON",
            "com1",
            "player id",
        ] {
            let err = sm.save(id, 0, bundle.clone()).unwrap_err();
            let text = err.to_string();
            assert!(text.contains("player id"), "{id:?} produced {text}");
        }
        sm.save("hero", 0, bundle).unwrap();
        assert!(dir.path().join("hero").is_dir());
        assert!(!dir.path().join("etc").exists());
    }

    #[test]
    fn oversized_compressed_length_is_rejected_before_use() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("slot00_bomb.awsv");
        let mut header = vec![0u8; 16];
        header[0..4].copy_from_slice(b"ASVS");
        header[4..6].copy_from_slice(&2u16.to_le_bytes());
        header[6] = 1;
        let claimed = (MAX_COMPRESSED_BYTES as u32).saturating_add(1);
        header[8..12].copy_from_slice(&claimed.to_le_bytes());
        fs::write(&path, &header).unwrap();
        let err = read_awsv(&path).unwrap_err();
        assert!(err.to_string().contains("limit"), "{}", err);
    }

    #[test]
    fn lz4_size_prefix_cannot_request_a_huge_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("slot00_expand.awsv");
        let mut payload = (MAX_UNCOMPRESSED_BYTES as u32 + 1).to_le_bytes().to_vec();
        payload.extend_from_slice(&[0, 0, 0, 0]);
        let mut crc = Crc32::new();
        crc.update(&payload);
        let crc = crc.finalize();
        let mut file = Vec::new();
        file.extend_from_slice(b"ASVS");
        file.extend_from_slice(&2u16.to_le_bytes());
        file.push(1);
        file.push(0);
        file.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        file.extend_from_slice(&crc.to_le_bytes());
        file.extend_from_slice(&payload);
        fs::write(&path, &file).unwrap();
        let err = read_awsv(&path).unwrap_err();
        assert!(err.to_string().contains("uncompressed"), "{err}");
    }

    #[test]
    fn oversized_index_is_not_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        fs::write(&path, vec![b' '; (MAX_INDEX_BYTES as usize) + 8]).unwrap();
        let err = read_index(dir.path()).unwrap_err();
        assert!(err.to_string().contains("limit"), "{err}");
    }

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
        // Locked against postcard 1.1 + time 0.3 binary OffsetDateTime tuple.
        // A dependency bump that changes this hex changes the save format.
        assert_eq!(
            hex::encode(&bytes),
            "021011111111222233334444555555555555ce1fbe02160d14000000000761726176617468000704010203048b1400000000"
        );
        let compressed = lz4_flex::compress_prepend_size(&bytes);
        let compressed_again = lz4_flex::compress_prepend_size(&bytes);
        assert_eq!(compressed, compressed_again, "lz4 output drifted");
        // lz4_flex 0.11.6 rejects invalid match offsets on decode. The
        // compressor for valid input is the same block format as 0.11.5.
        assert_eq!(
            hex::encode(&compressed),
            "32000000d1021011111111222233334444550100f011ce1fbe02160d14000000000761726176617468000704010203048b1400000000"
        );
        let restored = decompress_capped(&compressed).unwrap();
        assert_eq!(restored, bytes);
    }

    #[test]
    fn march_2026_awsv_files_still_decode() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut found = 0u32;
        for entry in fs::read_dir(&root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("awsv") {
                continue;
            }
            found += 1;
            let bundle = read_awsv(&path).unwrap_or_else(|err| {
                panic!(
                    "historical save {} no longer decodes: {err}",
                    path.display()
                )
            });
            assert_eq!(bundle.schema, SAVE_SCHEMA_VERSION);
            assert!(!bundle.player_id.is_empty());
            let again = read_awsv(&path).unwrap();
            assert_eq!(bundle.world.ecs_blob, again.world.ecs_blob);
            assert_eq!(bundle.world.tick, again.world.tick);
            assert_eq!(bundle.world.state_hash, again.world.state_hash);
            assert_eq!(bundle.save_id, again.save_id);
            assert_eq!(bundle.created_at, again.created_at);
        }
        assert!(found >= 5, "expected the checked-in March 2026 saves");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_save_is_refused() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let sm = SaveManager::new(dir.path());
        let saved = sm.save("hero", 0, sample_bundle()).unwrap();
        let outside = dir.path().join("outside.awsv");
        fs::rename(&saved, &outside).unwrap();
        symlink(&outside, &saved).unwrap();
        let err = sm.load_latest_slot("hero", 0).unwrap_err();
        let text = err.to_string();
        assert!(
            text.contains("symlink") || text.contains("outside"),
            "{text}"
        );
    }
}
