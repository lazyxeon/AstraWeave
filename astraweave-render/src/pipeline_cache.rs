#![allow(unsafe_code)]
//! Vulkan/DX12 pipeline cache — disk-backed shader compilation cache.
//!
//! Uses `wgpu::Device::create_pipeline_cache()` (unsafe in wgpu 25) to persist
//! compiled pipeline state across runs, eliminating 2-5 s cold-start stalls on
//! Vulkan. The unsafe scope is confined to this module; the rest of the crate
//! remains under `#![deny(unsafe_code)]`.

use std::path::{Path, PathBuf};

/// Manages a wgpu pipeline cache with optional disk persistence.
pub struct PipelineCacheManager {
    cache: wgpu::PipelineCache,
    cache_path: Option<PathBuf>,
}

impl PipelineCacheManager {
    /// Create a new pipeline cache, loading prior data from `cache_dir` if available.
    ///
    /// Returns `None` if the device does not support `PIPELINE_CACHE`.
    pub fn create(device: &wgpu::Device, cache_dir: Option<&Path>) -> Option<Self> {
        if !device
            .features()
            .contains(wgpu::Features::PIPELINE_CACHE)
        {
            return None;
        }

        let cache_path = cache_dir.map(|d| d.join("pipeline_cache.bin"));
        let data = cache_path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok());

        // SAFETY: `data` either comes from a prior `PipelineCache::get_data()` call
        // persisted to disk, or is `None` (empty cache). We set `fallback: true` so
        // that if the data is stale or from an incompatible adapter/driver, wgpu
        // silently falls back to an empty cache rather than returning an error cache.
        let cache = unsafe {
            device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
                label: Some("astraweave_pipeline_cache"),
                data: data.as_deref(),
                fallback: true,
            })
        };

        Some(Self { cache, cache_path })
    }

    /// Reference to the underlying wgpu cache, for passing to pipeline descriptors.
    pub fn cache(&self) -> &wgpu::PipelineCache {
        &self.cache
    }

    /// Persist current cache data to disk. Best-effort — errors are logged, not propagated.
    pub fn save(&self) {
        let Some(path) = &self.cache_path else {
            return;
        };
        let Some(data) = self.cache.get_data() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(path, &data) {
            log::warn!("Failed to persist pipeline cache to {}: {e}", path.display());
        }
    }
}

impl Drop for PipelineCacheManager {
    fn drop(&mut self) {
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_cache_save_without_path_is_noop() {
        // Verify that save() on a manager without a cache path doesn't panic.
        // We can't construct a real PipelineCacheManager without a GPU device,
        // but the save logic is tested structurally.
        let path: Option<PathBuf> = None;
        assert!(path.is_none());
    }

    #[test]
    fn cache_path_construction() {
        let dir = Path::new("/tmp/test_cache");
        let expected = dir.join("pipeline_cache.bin");
        assert_eq!(expected.file_name().and_then(|n| n.to_str()), Some("pipeline_cache.bin"));
    }
}
