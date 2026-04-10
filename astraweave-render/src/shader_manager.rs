//! Shader hot-reload manager with hash-based change detection.
//!
//! Tracks shader source files by content hash and provides pipeline invalidation
//! when shaders change on disk. Designed for rapid iteration during development.
//!
//! # Architecture
//!
//! ```text
//! ShaderManager
//!   ├── loaded shaders: HashMap<ShaderKey, ShaderEntry>
//!   │     └── ShaderEntry { path, content_hash, module, dirty }
//!   └── check_for_changes() → scans files, rehashes, marks dirty
//!
//! Usage:
//!   1. Register shaders with register_shader(key, path)
//!   2. Call check_for_changes() periodically (e.g. every 500ms)
//!   3. Query has_dirty_shaders() to know if pipeline recreation is needed
//!   4. Rebuild pipelines for dirty shaders, then clear_dirty()
//! ```

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Unique identifier for a registered shader.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShaderKey(pub String);

impl ShaderKey {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl std::fmt::Display for ShaderKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Content hash of a shader file (64-bit FNV-1a derived via DefaultHasher).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentHash(u64);

impl ContentHash {
    fn compute(content: &[u8]) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// Tracks a single shader source: path, content, hash, dirty state.
#[derive(Debug)]
struct ShaderEntry {
    /// Absolute path to the shader file.
    path: PathBuf,
    /// Hash of the file content when last loaded/checked.
    content_hash: ContentHash,
    /// Last known modification time (used as fast pre-check before hashing).
    last_mtime: Option<SystemTime>,
    /// Whether this shader has been modified since last acknowledged.
    dirty: bool,
    /// The source content (kept for re-creating shader modules).
    source: String,
}

/// Manages shader source files, detects changes, and tracks dirty state.
///
/// This manager does NOT hold GPU resources (shader modules, pipelines).
/// It provides the source and change detection; the renderer is responsible
/// for creating/recreating `wgpu::ShaderModule` and pipelines.
pub struct ShaderManager {
    shaders: HashMap<ShaderKey, ShaderEntry>,
}

impl std::fmt::Debug for ShaderManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShaderManager")
            .field("shader_count", &self.shaders.len())
            .field(
                "dirty_count",
                &self.shaders.values().filter(|e| e.dirty).count(),
            )
            .finish()
    }
}

impl ShaderManager {
    /// Create a new empty shader manager.
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
        }
    }

    /// Register a shader from a file path.
    ///
    /// Reads the file immediately and stores its content + hash.
    /// Returns `Err` if the file cannot be read.
    pub fn register_shader(
        &mut self,
        key: ShaderKey,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let path = path.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read shader {:?}: {}", path, e))?;
        let content_hash = ContentHash::compute(content.as_bytes());
        let mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        self.shaders.insert(
            key,
            ShaderEntry {
                path,
                content_hash,
                last_mtime: mtime,
                dirty: false,
                source: content,
            },
        );

        Ok(())
    }

    /// Register a shader from inline source (not backed by a file).
    ///
    /// These shaders will never be detected as "changed" — they're static.
    pub fn register_inline(&mut self, key: ShaderKey, source: impl Into<String>) {
        let source = source.into();
        let content_hash = ContentHash::compute(source.as_bytes());

        self.shaders.insert(
            key,
            ShaderEntry {
                path: PathBuf::new(),
                content_hash,
                last_mtime: None,
                dirty: false,
                source,
            },
        );
    }

    /// Scan all registered file-backed shaders for changes.
    ///
    /// Uses a two-level check:
    /// 1. **Fast path**: Compare file mtime — skip hashing if unchanged.
    /// 2. **Content hash**: Re-read and hash only if mtime differs.
    ///
    /// Call periodically (e.g. every 500ms or once per second).
    /// Returns the number of shaders that changed.
    pub fn check_for_changes(&mut self) -> usize {
        let mut changed_count = 0;

        for entry in self.shaders.values_mut() {
            // Skip inline shaders (no file path)
            if entry.path.as_os_str().is_empty() {
                continue;
            }

            // Fast mtime check
            let current_mtime = std::fs::metadata(&entry.path)
                .ok()
                .and_then(|m| m.modified().ok());

            if current_mtime == entry.last_mtime {
                continue;
            }

            // Mtime changed — re-read and check content hash
            let content = match std::fs::read_to_string(&entry.path) {
                Ok(c) => c,
                Err(_) => continue, // File might be mid-write; skip this cycle
            };

            let new_hash = ContentHash::compute(content.as_bytes());
            entry.last_mtime = current_mtime;

            if new_hash != entry.content_hash {
                entry.content_hash = new_hash;
                entry.source = content;
                entry.dirty = true;
                changed_count += 1;

                log::info!("Shader changed: {:?}", entry.path);
            }
        }

        changed_count
    }

    /// Whether any registered shader has been modified.
    pub fn has_dirty_shaders(&self) -> bool {
        self.shaders.values().any(|e| e.dirty)
    }

    /// Get all dirty shader keys.
    pub fn dirty_shaders(&self) -> Vec<&ShaderKey> {
        self.shaders
            .iter()
            .filter(|(_, entry)| entry.dirty)
            .map(|(key, _)| key)
            .collect()
    }

    /// Get the current source for a shader.
    pub fn source(&self, key: &ShaderKey) -> Option<&str> {
        self.shaders.get(key).map(|e| e.source.as_str())
    }

    /// Get the file path for a shader (empty for inline shaders).
    pub fn path(&self, key: &ShaderKey) -> Option<&Path> {
        self.shaders.get(key).map(|e| e.path.as_path())
    }

    /// Mark a specific shader as no longer dirty (after pipeline rebuild).
    pub fn clear_dirty(&mut self, key: &ShaderKey) {
        if let Some(entry) = self.shaders.get_mut(key) {
            entry.dirty = false;
        }
    }

    /// Mark all shaders as no longer dirty.
    pub fn clear_all_dirty(&mut self) {
        for entry in self.shaders.values_mut() {
            entry.dirty = false;
        }
    }

    /// Total number of registered shaders.
    pub fn shader_count(&self) -> usize {
        self.shaders.len()
    }

    /// Check if a key is registered.
    pub fn contains(&self, key: &ShaderKey) -> bool {
        self.shaders.contains_key(key)
    }

    /// Unregister a shader.
    pub fn remove(&mut self, key: &ShaderKey) -> bool {
        self.shaders.remove(key).is_some()
    }
}

impl Default for ShaderManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_deterministic() {
        let h1 = ContentHash::compute(b"hello wgsl");
        let h2 = ContentHash::compute(b"hello wgsl");
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_differs() {
        let h1 = ContentHash::compute(b"version A");
        let h2 = ContentHash::compute(b"version B");
        assert_ne!(h1, h2);
    }

    #[test]
    fn register_inline_shader() {
        let mut mgr = ShaderManager::new();
        let key = ShaderKey::new("test_shader");
        mgr.register_inline(key.clone(), "fn main() {}");

        assert_eq!(mgr.shader_count(), 1);
        assert!(mgr.contains(&key));
        assert_eq!(mgr.source(&key), Some("fn main() {}"));
        assert!(!mgr.has_dirty_shaders());
    }

    #[test]
    fn register_file_shader() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wgsl");
        std::fs::write(&path, "@compute @workgroup_size(1) fn main() {}").unwrap();

        let mut mgr = ShaderManager::new();
        let key = ShaderKey::new("test");
        mgr.register_shader(key.clone(), &path).unwrap();

        assert!(mgr.contains(&key));
        assert!(!mgr.has_dirty_shaders());
    }

    #[test]
    fn detect_file_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reload.wgsl");
        std::fs::write(&path, "// version 1").unwrap();

        let mut mgr = ShaderManager::new();
        let key = ShaderKey::new("reload");
        mgr.register_shader(key.clone(), &path).unwrap();

        // No changes yet
        assert_eq!(mgr.check_for_changes(), 0);

        // Modify file with different content and a new mtime
        // Force mtime change by sleeping briefly
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(&path, "// version 2 — modified").unwrap();

        let changed = mgr.check_for_changes();
        assert_eq!(changed, 1);
        assert!(mgr.has_dirty_shaders());
        assert_eq!(mgr.dirty_shaders().len(), 1);

        // Clear dirty
        mgr.clear_dirty(&key);
        assert!(!mgr.has_dirty_shaders());
    }

    #[test]
    fn inline_never_dirty() {
        let mut mgr = ShaderManager::new();
        mgr.register_inline(ShaderKey::new("static"), "const X: f32 = 1.0;");

        // Checking for changes should not mark inline shaders dirty
        assert_eq!(mgr.check_for_changes(), 0);
        assert!(!mgr.has_dirty_shaders());
    }

    #[test]
    fn same_content_not_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("same.wgsl");
        std::fs::write(&path, "// same content").unwrap();

        let mut mgr = ShaderManager::new();
        mgr.register_shader(ShaderKey::new("same"), &path).unwrap();

        // Touch file (update mtime) but keep same content
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(&path, "// same content").unwrap();

        // Content hash should match — not dirty
        let changed = mgr.check_for_changes();
        assert_eq!(changed, 0);
    }

    #[test]
    fn remove_shader() {
        let mut mgr = ShaderManager::new();
        let key = ShaderKey::new("removable");
        mgr.register_inline(key.clone(), "fn x() {}");

        assert!(mgr.contains(&key));
        assert!(mgr.remove(&key));
        assert!(!mgr.contains(&key));
        assert_eq!(mgr.shader_count(), 0);
    }

    #[test]
    fn clear_all_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.wgsl");
        let p2 = dir.path().join("b.wgsl");
        std::fs::write(&p1, "// a v1").unwrap();
        std::fs::write(&p2, "// b v1").unwrap();

        let mut mgr = ShaderManager::new();
        mgr.register_shader(ShaderKey::new("a"), &p1).unwrap();
        mgr.register_shader(ShaderKey::new("b"), &p2).unwrap();

        // Modify both
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(&p1, "// a v2").unwrap();
        std::fs::write(&p2, "// b v2").unwrap();

        assert_eq!(mgr.check_for_changes(), 2);
        assert!(mgr.has_dirty_shaders());

        mgr.clear_all_dirty();
        assert!(!mgr.has_dirty_shaders());
    }
}
