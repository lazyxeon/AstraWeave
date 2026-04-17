//! Impostor atlas registry (Phase 5.3 T7 stage 3a — editor plumbing).
//!
//! Maps each unique scatter mesh (identified by SHA-256 of its source bytes)
//! to a baked impostor atlas persisted on disk under
//! `assets/cache/impostors/<hash>/atlas.{png,toml}`.
//!
//! # Stage scope
//!
//! This module provides **only** the plumbing needed to identify, cache, and
//! lazy-bake atlases. It does NOT yet assemble a multi-species
//! [`ImpostorPass`](astraweave_render::impostor_pass::ImpostorPass) or install
//! one on the renderer — that is stage 3b. The existing PBR-quad LOD3 code
//! path in `engine_adapter.rs` remains the visible fallback until stage 3b
//! replaces it.
//!
//! # Architecture
//!
//! ```text
//! scatter mesh bytes ──► MeshHash::from_bytes ─┐
//!                                              │
//!                                              ▼
//!                              ImpostorRegistry::ensure(hash, spec, bake_fn)
//!                                              │
//!                          ┌───────────────────┴──────────────────┐
//!                          ▼                                      ▼
//!                   disk cache hit                         disk cache miss
//!                 (PNG + sidecar match)                (invoke bake_fn, persist)
//! ```
//!
//! # Persistence layout
//!
//! ```text
//! <cache_root>/
//!   <hash>/
//!     atlas.png
//!     atlas.toml
//! ```
//!
//! `<hash>` is the full 64-character lowercase-hex SHA-256 of the canonical
//! content bytes (typically the source `.gltf`/`.glb`). Using a per-hash
//! subdirectory keeps writes atomic under concurrent access (one atlas per
//! dir) and makes manual inspection / cache eviction obvious.

use anyhow::{Context, Result};
use astraweave_render::impostor_bake::{load_or_bake_atlas, LoadedAtlas};
use astraweave_render::vegetation_lod::ImpostorAtlasSpec;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Content hash of a scatter mesh's source bytes, stored as a 64-character
/// lowercase-hex string (SHA-256).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MeshHash(String);

impl MeshHash {
    /// Hash the given byte slice with SHA-256 and render as lowercase hex.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        let mut s = String::with_capacity(64);
        for b in digest.iter() {
            use std::fmt::Write as _;
            let _ = write!(s, "{:02x}", b);
        }
        Self(s)
    }

    /// Construct a [`MeshHash`] from an already-formatted lowercase-hex
    /// string. Returns `None` if the input is not exactly 64 ASCII hex
    /// characters (this is a cheap sanity check — nothing more).
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 64 {
            return None;
        }
        if !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return None;
        }
        Some(Self(hex.to_owned()))
    }

    /// Hex view of this hash.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Resolve the canonical `(png, sidecar)` path pair for a given cache root
/// and mesh hash. Layout is `<cache_root>/<hash>/atlas.{png,toml}`.
pub fn atlas_paths(cache_root: &Path, hash: &MeshHash) -> (PathBuf, PathBuf) {
    let dir = cache_root.join(hash.as_str());
    (dir.join("atlas.png"), dir.join("atlas.toml"))
}

/// In-memory cache of baked atlases keyed by mesh hash.
///
/// A single [`ImpostorRegistry`] is owned by the editor's engine adapter and
/// is populated lazily on the first LOD3 encounter for each scatter mesh.
/// Subsequent encounters hit the in-memory entry with no disk I/O.
pub struct ImpostorRegistry {
    cache_root: PathBuf,
    entries: HashMap<MeshHash, LoadedAtlas>,
}

impl ImpostorRegistry {
    /// Create a new registry rooted at `cache_root`. The directory is created
    /// on first bake (not eagerly); passing a path that does not yet exist is
    /// fine.
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            cache_root,
            entries: HashMap::new(),
        }
    }

    /// Filesystem root under which per-hash atlas directories live.
    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    /// Number of atlases currently cached in memory.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` iff no atlases are cached in memory yet.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `true` iff `hash` is currently resident in the in-memory cache.
    pub fn contains(&self, hash: &MeshHash) -> bool {
        self.entries.contains_key(hash)
    }

    /// Direct lookup without triggering a bake.
    pub fn get(&self, hash: &MeshHash) -> Option<&LoadedAtlas> {
        self.entries.get(hash)
    }

    /// Remove an atlas from the in-memory cache (does NOT delete on-disk
    /// files). Useful when a scatter mesh is live-edited and its atlas needs
    /// to be rebuilt.
    pub fn evict(&mut self, hash: &MeshHash) -> Option<LoadedAtlas> {
        self.entries.remove(hash)
    }

    /// Ensure an atlas is available for `hash`. Resolution order:
    ///
    /// 1. In-memory cache hit → return reference.
    /// 2. Delegate to [`load_or_bake_atlas`], which does its own disk-cache
    ///    check at `<cache_root>/<hash>/atlas.{png,toml}` and invokes
    ///    `bake_fn` on miss, persisting the result.
    ///
    /// `bake_fn` is only called on full cache miss. It receives the canonical
    /// [`ImpostorAtlasSpec`] and must return `(pixels, width, height)` in
    /// RGBA8 layout matching the spec's dimensions.
    pub fn ensure<F>(
        &mut self,
        hash: &MeshHash,
        spec: &ImpostorAtlasSpec,
        bake_fn: F,
    ) -> Result<&LoadedAtlas>
    where
        F: FnOnce(&ImpostorAtlasSpec) -> Result<(Vec<u8>, u32, u32)>,
    {
        if !self.entries.contains_key(hash) {
            let (png, sidecar) = atlas_paths(&self.cache_root, hash);
            let loaded = load_or_bake_atlas(&png, &sidecar, spec, bake_fn)
                .with_context(|| {
                    format!("lazy-bake atlas for mesh {}", hash.as_str())
                })?;
            self.entries.insert(hash.clone(), loaded);
        }
        Ok(self
            .entries
            .get(hash)
            .expect("entry inserted above or already present"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_render::impostor_bake::AtlasSource;
    use tempfile::TempDir;

    fn make_spec(w: u32, h: u32, species: &str) -> ImpostorAtlasSpec {
        ImpostorAtlasSpec::uniform(w, h, 4, &[species])
    }

    #[test]
    fn mesh_hash_is_deterministic_sha256() {
        let a = MeshHash::from_bytes(b"hello world");
        let b = MeshHash::from_bytes(b"hello world");
        let c = MeshHash::from_bytes(b"hello world!");
        assert_eq!(a, b);
        assert_ne!(a, c);
        // SHA-256("hello world") == b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(
            a.as_str(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn mesh_hash_from_hex_round_trips_valid_hashes() {
        let h = MeshHash::from_bytes(b"scatter-mesh");
        let round = MeshHash::from_hex(h.as_str()).expect("valid hex");
        assert_eq!(h, round);
    }

    #[test]
    fn mesh_hash_from_hex_rejects_wrong_length_and_non_hex() {
        assert!(MeshHash::from_hex("").is_none());
        assert!(MeshHash::from_hex("abc").is_none());
        // uppercase disallowed — we store lowercase
        assert!(MeshHash::from_hex(&"A".repeat(64)).is_none());
        // right length, wrong alphabet
        assert!(MeshHash::from_hex(&"z".repeat(64)).is_none());
    }

    #[test]
    fn atlas_paths_nest_under_hash_subdir() {
        let root = PathBuf::from("/cache/root");
        let hash = MeshHash::from_bytes(b"m");
        let (png, toml_) = atlas_paths(&root, &hash);
        assert!(png.ends_with("atlas.png"));
        assert!(toml_.ends_with("atlas.toml"));
        assert_eq!(png.parent(), toml_.parent());
        assert_eq!(
            png.parent().unwrap().file_name().unwrap().to_str().unwrap(),
            hash.as_str()
        );
    }

    #[test]
    fn ensure_bakes_on_miss_then_hits_memory_cache() {
        let tmp = TempDir::new().unwrap();
        let mut reg = ImpostorRegistry::new(tmp.path().to_path_buf());
        let hash = MeshHash::from_bytes(b"scatter-xyz");
        let spec = make_spec(8, 8, "oak");

        let call_count = std::cell::Cell::new(0u32);
        reg.ensure(&hash, &spec, |s| {
            call_count.set(call_count.get() + 1);
            let px = vec![0u8; (s.atlas_width * s.atlas_height * 4) as usize];
            Ok((px, s.atlas_width, s.atlas_height))
        })
        .unwrap();
        assert_eq!(call_count.get(), 1, "first ensure triggers bake");

        // Second ensure on the SAME hash must hit the in-memory cache — bake
        // closure must not be called again.
        reg.ensure(&hash, &spec, |_| {
            panic!("bake_fn must not run on in-memory hit")
        })
        .unwrap();
        assert_eq!(reg.len(), 1);
        assert!(reg.contains(&hash));
    }

    #[test]
    fn ensure_persists_atlas_to_disk_cache() {
        let tmp = TempDir::new().unwrap();
        let mut reg = ImpostorRegistry::new(tmp.path().to_path_buf());
        let hash = MeshHash::from_bytes(b"persist-me");
        let spec = make_spec(4, 4, "pine");

        reg.ensure(&hash, &spec, |s| {
            Ok((
                vec![0u8; (s.atlas_width * s.atlas_height * 4) as usize],
                s.atlas_width,
                s.atlas_height,
            ))
        })
        .unwrap();

        let (png, sidecar) = atlas_paths(reg.cache_root(), &hash);
        assert!(png.exists(), "atlas PNG must be written to disk");
        assert!(sidecar.exists(), "atlas sidecar must be written to disk");
    }

    #[test]
    fn ensure_uses_disk_cache_across_fresh_registries() {
        let tmp = TempDir::new().unwrap();
        let hash = MeshHash::from_bytes(b"reload-me");
        let spec = make_spec(4, 4, "birch");

        // Cold: bake + persist.
        {
            let mut reg = ImpostorRegistry::new(tmp.path().to_path_buf());
            reg.ensure(&hash, &spec, |s| {
                Ok((
                    vec![0u8; (s.atlas_width * s.atlas_height * 4) as usize],
                    s.atlas_width,
                    s.atlas_height,
                ))
            })
            .unwrap();
            assert_eq!(reg.get(&hash).unwrap().source, AtlasSource::Baked);
        }

        // Warm: fresh registry, same cache_root — must load from disk, NOT
        // invoke bake_fn.
        {
            let mut reg = ImpostorRegistry::new(tmp.path().to_path_buf());
            let loaded = reg
                .ensure(&hash, &spec, |_| {
                    panic!("bake_fn must not run when disk cache is warm")
                })
                .unwrap();
            assert_eq!(loaded.source, AtlasSource::LoadedFromDisk);
        }
    }

    #[test]
    fn evict_drops_memory_entry_but_leaves_disk() {
        let tmp = TempDir::new().unwrap();
        let mut reg = ImpostorRegistry::new(tmp.path().to_path_buf());
        let hash = MeshHash::from_bytes(b"evictable");
        let spec = make_spec(4, 4, "oak");
        reg.ensure(&hash, &spec, |s| {
            Ok((
                vec![0u8; (s.atlas_width * s.atlas_height * 4) as usize],
                s.atlas_width,
                s.atlas_height,
            ))
        })
        .unwrap();

        let dropped = reg.evict(&hash);
        assert!(dropped.is_some());
        assert!(!reg.contains(&hash));
        let (png, _) = atlas_paths(reg.cache_root(), &hash);
        assert!(png.exists(), "evict must not delete on-disk artifacts");
    }
}
