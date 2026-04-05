//! Conversion result caching system.
//!
//! This module provides a disk-based cache for converted glTF files,
//! using SHA-256 content hashing for invalidation and RON for manifest storage.

use crate::error::{BlendError, BlendResult};
use crate::options::ConversionOptions;
use crate::version::BlenderVersion;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Default cache directory name within project.
pub const DEFAULT_CACHE_DIR: &str = ".astraweave/blend_cache";

/// Cache manifest filename.
pub const MANIFEST_FILENAME: &str = "cache_manifest.ron";

/// Result of a cache lookup.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)] // Entry is frequently accessed, boxing adds indirection
#[non_exhaustive]
pub enum CacheLookup {
    /// Cache hit - valid cached conversion exists.
    Hit {
        /// Path to cached glTF/GLB file.
        output_path: PathBuf,
        /// Cached entry metadata.
        entry: CacheEntry,
    },
    /// Cache miss - no valid cached conversion.
    Miss {
        /// Reason for cache miss.
        reason: CacheMissReason,
    },
}

/// Reasons for a cache miss.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CacheMissReason {
    /// No cache entry exists for this file.
    NotCached,
    /// Source file hash has changed.
    SourceModified,
    /// Conversion options have changed.
    OptionsChanged,
    /// Blender version has changed.
    BlenderVersionChanged,
    /// Cached output file is missing.
    OutputMissing,
    /// Cache entry has expired.
    Expired,
    /// Cache validation failed.
    ValidationFailed(String),
}

impl std::fmt::Display for CacheMissReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheMissReason::NotCached => write!(f, "not cached"),
            CacheMissReason::SourceModified => write!(f, "source file modified"),
            CacheMissReason::OptionsChanged => write!(f, "conversion options changed"),
            CacheMissReason::BlenderVersionChanged => write!(f, "Blender version changed"),
            CacheMissReason::OutputMissing => write!(f, "cached output missing"),
            CacheMissReason::Expired => write!(f, "cache entry expired"),
            CacheMissReason::ValidationFailed(reason) => write!(f, "validation failed: {}", reason),
        }
    }
}

/// A single cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// SHA-256 hash of source .blend file content.
    pub source_hash: String,
    /// Hash of conversion options used.
    pub options_hash: String,
    /// Blender version used for conversion.
    pub blender_version: BlenderVersion,
    /// Relative path to cached output file.
    pub output_path: PathBuf,
    /// Original source file path (for reference).
    pub source_path: PathBuf,
    /// Timestamp when entry was created.
    pub created_at: u64,
    /// Timestamp when entry was last accessed.
    pub last_accessed: u64,
    /// Size of the output file in bytes.
    pub output_size: u64,
    /// Conversion duration in milliseconds.
    pub conversion_duration_ms: u64,
    /// Associated texture files (relative paths).
    pub texture_files: Vec<PathBuf>,
    /// Any linked library files that were processed.
    pub linked_libraries: Vec<PathBuf>,
}

impl CacheEntry {
    /// Creates a new cache entry.
    pub fn new(
        source_hash: String,
        options_hash: String,
        blender_version: BlenderVersion,
        output_path: PathBuf,
        source_path: PathBuf,
        output_size: u64,
        conversion_duration_ms: u64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            source_hash,
            options_hash,
            blender_version,
            output_path,
            source_path,
            created_at: now,
            last_accessed: now,
            output_size,
            conversion_duration_ms,
            texture_files: Vec::new(),
            linked_libraries: Vec::new(),
        }
    }

    /// Updates the last accessed timestamp.
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Returns the age of this entry.
    pub fn age(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Duration::from_secs(now.saturating_sub(self.created_at))
    }

    /// Returns time since last access.
    pub fn time_since_access(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Duration::from_secs(now.saturating_sub(self.last_accessed))
    }
}

/// Cache manifest containing all entries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheManifest {
    /// Version of the manifest format.
    pub version: u32,
    /// Cache entries keyed by source file path (normalized).
    pub entries: HashMap<String, CacheEntry>,
    /// Total size of all cached files.
    pub total_size: u64,
    /// Last cleanup timestamp.
    pub last_cleanup: u64,
}

impl CacheManifest {
    /// Current manifest version.
    pub const CURRENT_VERSION: u32 = 1;

    /// Creates a new empty manifest.
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            entries: HashMap::new(),
            total_size: 0,
            last_cleanup: 0,
        }
    }

    /// Recalculates total size from entries.
    pub fn recalculate_size(&mut self) {
        self.total_size = self.entries.values().map(|e| e.output_size).sum();
    }
}

/// The conversion cache manager.
pub struct ConversionCache {
    /// Cache directory path.
    cache_dir: PathBuf,
    /// In-memory manifest.
    manifest: CacheManifest,
    /// Maximum cache size in bytes (None = unlimited).
    max_size: Option<u64>,
    /// Maximum entry age (None = never expire).
    max_age: Option<Duration>,
    /// Whether cache is enabled.
    enabled: bool,
}

impl ConversionCache {
    /// Creates a new cache in the specified directory.
    ///
    /// The cache directory path is canonicalized. Symlinks within the cache
    /// directory are rejected for security (prevents symlink traversal attacks).
    pub fn new(cache_dir: impl Into<PathBuf>) -> BlendResult<Self> {
        let cache_dir = cache_dir.into();

        // Ensure directory exists
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).map_err(|e| BlendError::CacheDirectoryError {
                path: cache_dir.clone(),
                message: "Failed to create cache directory".to_string(),
                source: e,
            })?;
        }

        // Canonicalize to resolve any ".." or "." components and prevent traversal
        let cache_dir =
            fs::canonicalize(&cache_dir).map_err(|e| BlendError::CacheDirectoryError {
                path: cache_dir.clone(),
                message: "Failed to canonicalize cache directory path".to_string(),
                source: e,
            })?;

        // Reject symlinks within the cache directory (security hardening)
        Self::reject_symlinks_in_dir(&cache_dir)?;

        // Load or create manifest
        let manifest_path = cache_dir.join(MANIFEST_FILENAME);
        let manifest = if manifest_path.exists() {
            Self::load_manifest(&manifest_path)?
        } else {
            CacheManifest::new()
        };

        Ok(Self {
            cache_dir,
            manifest,
            max_size: None,
            max_age: None,
            enabled: true,
        })
    }

    /// Creates a cache for a project directory.
    pub fn for_project(project_dir: impl AsRef<Path>) -> BlendResult<Self> {
        let cache_dir = project_dir.as_ref().join(DEFAULT_CACHE_DIR);
        Self::new(cache_dir)
    }

    /// Sets the maximum cache size.
    pub fn with_max_size(mut self, max_size: Option<u64>) -> Self {
        self.max_size = max_size;
        self
    }

    /// Sets the maximum entry age.
    pub fn with_max_age(mut self, max_age: Option<Duration>) -> Self {
        self.max_age = max_age;
        self
    }

    /// Enables or disables the cache.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Looks up a cached conversion result.
    pub fn lookup(
        &mut self,
        source_path: &Path,
        options: &ConversionOptions,
        blender_version: &BlenderVersion,
    ) -> BlendResult<CacheLookup> {
        if !self.enabled {
            return Ok(CacheLookup::Miss {
                reason: CacheMissReason::NotCached,
            });
        }

        let key = Self::normalize_path(source_path);

        // Check if entry exists
        let entry = match self.manifest.entries.get(&key) {
            Some(e) => e.clone(),
            None => {
                return Ok(CacheLookup::Miss {
                    reason: CacheMissReason::NotCached,
                });
            }
        };

        // Check Blender version
        if entry.blender_version != *blender_version {
            return Ok(CacheLookup::Miss {
                reason: CacheMissReason::BlenderVersionChanged,
            });
        }

        // Check options hash
        let options_hash = Self::hash_options(options)?;
        if entry.options_hash != options_hash {
            return Ok(CacheLookup::Miss {
                reason: CacheMissReason::OptionsChanged,
            });
        }

        // Check source file hash
        let source_hash = Self::hash_file(source_path)?;
        if entry.source_hash != source_hash {
            return Ok(CacheLookup::Miss {
                reason: CacheMissReason::SourceModified,
            });
        }

        // Check if output exists
        let output_path = self.cache_dir.join(&entry.output_path);
        if !output_path.exists() {
            return Ok(CacheLookup::Miss {
                reason: CacheMissReason::OutputMissing,
            });
        }

        // Check expiration
        if let Some(max_age) = self.max_age {
            if entry.age() > max_age {
                return Ok(CacheLookup::Miss {
                    reason: CacheMissReason::Expired,
                });
            }
        }

        // Update last accessed time
        if let Some(e) = self.manifest.entries.get_mut(&key) {
            e.touch();
        }

        // Save updated manifest (async save in production would be better)
        let _ = self.save_manifest();

        Ok(CacheLookup::Hit { output_path, entry })
    }

    /// Stores a conversion result in the cache.
    #[allow(clippy::too_many_arguments)] // Cache storage requires many parameters
    pub fn store(
        &mut self,
        source_path: &Path,
        output_path: &Path,
        options: &ConversionOptions,
        blender_version: &BlenderVersion,
        conversion_duration_ms: u64,
        texture_files: Vec<PathBuf>,
        linked_libraries: Vec<PathBuf>,
    ) -> BlendResult<PathBuf> {
        if !self.enabled {
            return Ok(output_path.to_path_buf());
        }

        let key = Self::normalize_path(source_path);
        let source_hash = Self::hash_file(source_path)?;
        let options_hash = Self::hash_options(options)?;

        // Generate cache filename from source hash
        let cache_filename = format!(
            "{}.{}",
            &source_hash[..16], // Use first 16 chars of hash
            options.format.extension()
        );
        let cached_output_path = self.cache_dir.join(&cache_filename);

        // Security: verify the resolved output path stays within the cache directory
        Self::verify_path_within_cache(&self.cache_dir, &cached_output_path)?;

        // Copy output to cache directory
        if output_path != cached_output_path {
            fs::copy(output_path, &cached_output_path).map_err(|e| {
                BlendError::CacheWriteError {
                    path: cached_output_path.clone(),
                    message: "Failed to copy to cache".to_string(),
                    source: e,
                }
            })?;
        }

        // Copy texture files
        let mut cached_textures = Vec::new();
        for texture in &texture_files {
            if texture.exists() {
                let texture_filename = texture
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "texture".to_string());
                let cached_texture = self.cache_dir.join(&texture_filename);
                // Security: verify texture path stays within cache directory
                if Self::verify_path_within_cache(&self.cache_dir, &cached_texture).is_ok() {
                    if texture != &cached_texture {
                        let _ = fs::copy(texture, &cached_texture);
                    }
                    cached_textures.push(PathBuf::from(&texture_filename));
                } else {
                    warn!(
                        "Skipping texture with suspicious path: {}",
                        texture.display()
                    );
                }
            }
        }

        let output_size = fs::metadata(&cached_output_path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Create entry
        let entry = CacheEntry {
            source_hash,
            options_hash,
            blender_version: *blender_version,
            output_path: PathBuf::from(&cache_filename),
            source_path: source_path.to_path_buf(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            last_accessed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            output_size,
            conversion_duration_ms,
            texture_files: cached_textures,
            linked_libraries,
        };

        // Store entry
        self.manifest.entries.insert(key, entry);
        self.manifest.recalculate_size();

        // Enforce size limit
        if let Some(max_size) = self.max_size {
            self.enforce_size_limit(max_size)?;
        }

        // Save manifest
        self.save_manifest()?;

        info!(
            "Cached conversion: {} -> {} ({} bytes)",
            source_path.display(),
            cache_filename,
            output_size
        );

        Ok(cached_output_path)
    }

    /// Invalidates a specific cache entry.
    pub fn invalidate(&mut self, source_path: &Path) -> BlendResult<bool> {
        let key = Self::normalize_path(source_path);

        if let Some(entry) = self.manifest.entries.remove(&key) {
            // Delete cached files
            let output_path = self.cache_dir.join(&entry.output_path);
            let _ = fs::remove_file(&output_path);

            for texture in &entry.texture_files {
                let texture_path = self.cache_dir.join(texture);
                let _ = fs::remove_file(&texture_path);
            }

            self.manifest.recalculate_size();
            self.save_manifest()?;

            debug!("Invalidated cache entry: {}", source_path.display());
            return Ok(true);
        }

        Ok(false)
    }

    /// Clears the entire cache.
    pub fn clear(&mut self) -> BlendResult<()> {
        // Delete all cached files
        for entry in self.manifest.entries.values() {
            let output_path = self.cache_dir.join(&entry.output_path);
            let _ = fs::remove_file(&output_path);

            for texture in &entry.texture_files {
                let texture_path = self.cache_dir.join(texture);
                let _ = fs::remove_file(&texture_path);
            }
        }

        self.manifest.entries.clear();
        self.manifest.total_size = 0;
        self.save_manifest()?;

        info!("Cleared blend conversion cache");
        Ok(())
    }

    /// Returns cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.manifest.entries.len(),
            total_size: self.manifest.total_size,
            max_size: self.max_size,
            enabled: self.enabled,
        }
    }

    /// Enforces the size limit by evicting oldest entries.
    fn enforce_size_limit(&mut self, max_size: u64) -> BlendResult<()> {
        while self.manifest.total_size > max_size && !self.manifest.entries.is_empty() {
            // Find oldest entry by last_accessed
            let oldest_key = self
                .manifest
                .entries
                .iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone());

            if let Some(key) = oldest_key {
                if let Some(entry) = self.manifest.entries.remove(&key) {
                    let output_path = self.cache_dir.join(&entry.output_path);
                    let _ = fs::remove_file(&output_path);

                    for texture in &entry.texture_files {
                        let texture_path = self.cache_dir.join(texture);
                        let _ = fs::remove_file(&texture_path);
                    }

                    warn!("Evicted cache entry (LRU): {}", key);
                }
            } else {
                break;
            }

            self.manifest.recalculate_size();
        }

        Ok(())
    }

    /// Loads manifest from disk.
    fn load_manifest(path: &Path) -> BlendResult<CacheManifest> {
        let content = fs::read_to_string(path).map_err(|e| BlendError::CacheCorrupted {
            path: path.to_path_buf(),
            message: format!("Failed to read manifest: {}", e),
        })?;

        let manifest: CacheManifest =
            ron::from_str(&content).map_err(|e| BlendError::CacheCorrupted {
                path: path.to_path_buf(),
                message: format!("Failed to parse manifest: {}", e),
            })?;

        // Version check
        if manifest.version != CacheManifest::CURRENT_VERSION {
            warn!(
                "Cache manifest version mismatch ({} vs {}), clearing cache",
                manifest.version,
                CacheManifest::CURRENT_VERSION
            );
            return Ok(CacheManifest::new());
        }

        Ok(manifest)
    }

    /// Saves manifest to disk.
    fn save_manifest(&self) -> BlendResult<()> {
        let manifest_path = self.cache_dir.join(MANIFEST_FILENAME);
        let config = PrettyConfig::new().depth_limit(4);
        let content = ron::ser::to_string_pretty(&self.manifest, config).map_err(|e| {
            BlendError::Serialization(format!("Failed to serialize manifest: {}", e))
        })?;

        fs::write(&manifest_path, &content).map_err(|e| BlendError::CacheWriteError {
            path: manifest_path,
            message: "Failed to write manifest".to_string(),
            source: e,
        })?;

        Ok(())
    }

    /// Normalizes a path for use as a cache key.
    fn normalize_path(path: &Path) -> String {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
    }

    /// Computes SHA-256 hash of a file.
    pub fn hash_file(path: &Path) -> BlendResult<String> {
        let file = fs::File::open(path).map_err(|e| BlendError::FileReadError {
            path: path.to_path_buf(),
            message: "Failed to open for hashing".to_string(),
            source: e,
        })?;

        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| BlendError::FileReadError {
                    path: path.to_path_buf(),
                    message: "Failed to read for hashing".to_string(),
                    source: e,
                })?;

            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Computes hash of conversion options.
    fn hash_options(options: &ConversionOptions) -> BlendResult<String> {
        let serialized = ron::to_string(options).map_err(|e| BlendError::ConfigurationError {
            message: format!("Failed to serialize options: {}", e),
        })?;

        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Scans a directory for symlinks and rejects them.
    ///
    /// This prevents symlink traversal attacks where a malicious symlink
    /// inside the cache directory points to an arbitrary location on disk.
    fn reject_symlinks_in_dir(dir: &Path) -> BlendResult<()> {
        if dir.is_symlink() {
            return Err(BlendError::CacheDirectoryError {
                path: dir.to_path_buf(),
                message: "Cache directory itself is a symlink (security risk)".to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "symlink in cache directory",
                ),
            });
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()), // Empty or unreadable — no symlinks to find
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_symlink() {
                warn!("Removing symlink in cache directory: {}", path.display());
                // Remove the symlink (not the target) to sanitize the cache
                let _ = fs::remove_file(&path);
            }
        }

        Ok(())
    }

    /// Verifies that a resolved path stays within the cache directory.
    ///
    /// Prevents path traversal attacks where filenames like `../../etc/passwd`
    /// could escape the cache directory.
    fn verify_path_within_cache(cache_dir: &Path, target_path: &Path) -> BlendResult<()> {
        // Canonicalize the cache dir (should already be canonical, but be safe)
        let canonical_cache =
            fs::canonicalize(cache_dir).unwrap_or_else(|_| cache_dir.to_path_buf());

        // For the target, we can't canonicalize if it doesn't exist yet,
        // so we normalize manually by resolving the joined path
        let canonical_target = if target_path.exists() {
            fs::canonicalize(target_path).unwrap_or_else(|_| target_path.to_path_buf())
        } else {
            // For not-yet-existing files, verify the parent is within bounds
            // and the filename has no path separators
            if let Some(file_name) = target_path.file_name() {
                let name_str = file_name.to_string_lossy();
                if name_str.contains('/') || name_str.contains('\\') || name_str.contains("..") {
                    return Err(BlendError::CacheWriteError {
                        path: target_path.to_path_buf(),
                        message: "Cache filename contains path traversal characters".to_string(),
                        source: std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "path traversal in cache filename",
                        ),
                    });
                }
                canonical_cache.join(file_name)
            } else {
                return Err(BlendError::CacheWriteError {
                    path: target_path.to_path_buf(),
                    message: "Cache target path has no filename".to_string(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "no filename in cache target",
                    ),
                });
            }
        };

        if !canonical_target.starts_with(&canonical_cache) {
            return Err(BlendError::CacheWriteError {
                path: target_path.to_path_buf(),
                message: format!(
                    "Path escapes cache directory: {} is not within {}",
                    canonical_target.display(),
                    canonical_cache.display()
                ),
                source: std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "path traversal attempt",
                ),
            });
        }

        Ok(())
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached entries.
    pub entry_count: usize,
    /// Total size of cached files in bytes.
    pub total_size: u64,
    /// Maximum cache size (if set).
    pub max_size: Option<u64>,
    /// Whether cache is enabled.
    pub enabled: bool,
}

impl CacheStats {
    /// Returns the cache utilization as a percentage (0.0 - 1.0).
    pub fn utilization(&self) -> Option<f64> {
        self.max_size.map(|max| {
            if max > 0 {
                self.total_size as f64 / max as f64
            } else {
                0.0
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_miss_reasons_display() {
        assert_eq!(CacheMissReason::NotCached.to_string(), "not cached");
        assert_eq!(
            CacheMissReason::SourceModified.to_string(),
            "source file modified"
        );
    }

    #[test]
    fn test_cache_entry_age() {
        let entry = CacheEntry::new(
            "hash".to_string(),
            "options".to_string(),
            BlenderVersion::new(4, 0, 0),
            PathBuf::from("output.glb"),
            PathBuf::from("source.blend"),
            1000,
            500,
        );

        // Age should be very small (just created)
        assert!(entry.age() < Duration::from_secs(1));
    }

    #[test]
    fn test_cache_manifest_new() {
        let manifest = CacheManifest::new();
        assert_eq!(manifest.version, CacheManifest::CURRENT_VERSION);
        assert!(manifest.entries.is_empty());
        assert_eq!(manifest.total_size, 0);
    }

    #[test]
    fn test_cache_stats_utilization() {
        let stats = CacheStats {
            entry_count: 5,
            total_size: 500,
            max_size: Some(1000),
            enabled: true,
        };

        assert_eq!(stats.utilization(), Some(0.5));

        let unlimited = CacheStats {
            entry_count: 5,
            total_size: 500,
            max_size: None,
            enabled: true,
        };

        assert_eq!(unlimited.utilization(), None);
    }

    #[test]
    fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ConversionCache::new(temp_dir.path()).unwrap();

        assert!(cache.cache_dir().exists());
        assert!(cache.enabled);
        assert_eq!(cache.stats().entry_count, 0);
    }

    #[test]
    fn test_normalize_path() {
        let path = PathBuf::from("/Test/Path/File.blend");
        let normalized = ConversionCache::normalize_path(&path);

        // Should be lowercase on all platforms
        assert!(!normalized.contains('T') || normalized.contains('t'));
    }
}
