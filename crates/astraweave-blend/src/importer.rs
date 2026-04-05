//! Main Blender importer API.
//!
//! This module provides the primary public interface for importing .blend files
//! into AstraWeave, with automatic conversion, caching, and progress tracking.

use crate::cache::ConversionCache;
use crate::conversion::{ConversionJob, ConversionResult};
use crate::decomposer::{DecompositionResult, SceneDecomposer};
use crate::discovery::{BlenderDiscovery, BlenderDiscoveryConfig, BlenderInstallation};
use crate::error::{BlendError, BlendResult};
use crate::options::ConversionOptions;
use crate::progress::{CancellationToken, ConversionProgress, ProgressReceiver, ProgressTracker};
use crate::texture_processor::{
    process_decomposition_textures, TextureProcessingConfig, TextureProcessingResult,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Configuration for the BlendImporter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendImporterConfig {
    /// Blender discovery configuration.
    pub discovery: BlenderDiscoveryConfig,
    /// Default conversion options.
    pub default_options: ConversionOptions,
    /// Project root directory (for relative paths and cache location).
    pub project_root: Option<PathBuf>,
    /// Whether to enable caching.
    pub cache_enabled: bool,
}

impl Default for BlendImporterConfig {
    fn default() -> Self {
        Self {
            discovery: BlenderDiscoveryConfig::default(),
            default_options: ConversionOptions::default(),
            project_root: None,
            cache_enabled: true,
        }
    }
}

/// The main Blender file importer.
///
/// Provides a high-level API for importing .blend files with automatic
/// Blender discovery, caching, and progress tracking.
///
/// # Example
///
/// ```no_run
/// use astraweave_blend::{BlendImporter, ConversionOptions};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create importer with default settings
///     let mut importer = BlendImporter::new().await?;
///
///     // Import a .blend file
///     let result = importer
///         .import("path/to/model.blend")
///         .await?;
///
///     println!("Imported: {} ({} bytes)", result.output_path.display(), result.output_size);
///     Ok(())
/// }
/// ```
pub struct BlendImporter {
    /// Blender discovery manager.
    discovery: BlenderDiscovery,
    /// Conversion cache.
    cache: Option<Arc<RwLock<ConversionCache>>>,
    /// Default conversion options.
    default_options: ConversionOptions,
    /// Project root directory.
    project_root: Option<PathBuf>,
    /// Cached Blender installation.
    installation: Option<BlenderInstallation>,
}

impl BlendImporter {
    /// Creates a new BlendImporter with default configuration.
    ///
    /// Automatically discovers Blender and initializes the cache.
    pub async fn new() -> BlendResult<Self> {
        Self::with_config(BlendImporterConfig::default()).await
    }

    /// Creates a new BlendImporter with custom configuration.
    pub async fn with_config(config: BlendImporterConfig) -> BlendResult<Self> {
        let discovery = BlenderDiscovery::with_config(config.discovery);

        let cache = if config.cache_enabled {
            let cache_dir = config
                .project_root
                .as_ref()
                .map(|p| p.join(".astraweave/blend_cache"))
                .unwrap_or_else(|| {
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(".astraweave/blend_cache")
                });

            match ConversionCache::new(&cache_dir) {
                Ok(c) => {
                    let c = c
                        .with_max_size(config.default_options.cache.max_cache_size)
                        .with_max_age(config.default_options.cache.max_age);
                    Some(Arc::new(RwLock::new(c)))
                }
                Err(e) => {
                    warn!("Failed to initialize cache: {}. Caching disabled.", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            discovery,
            cache,
            default_options: config.default_options,
            project_root: config.project_root,
            installation: None,
        })
    }

    /// Creates a new BlendImporter for a specific project.
    pub async fn for_project(project_root: impl Into<PathBuf>) -> BlendResult<Self> {
        let project_root = project_root.into();
        let config = BlendImporterConfig {
            project_root: Some(project_root),
            ..Default::default()
        };
        Self::with_config(config).await
    }

    /// Sets a user-specified Blender executable path.
    pub fn set_blender_path(&mut self, path: impl Into<PathBuf>) {
        self.discovery.set_user_path(path);
        self.installation = None; // Invalidate cached installation
    }

    /// Clears the user-specified Blender path.
    pub fn clear_blender_path(&mut self) {
        self.discovery.clear_user_path();
        self.installation = None;
    }

    /// Returns the current Blender installation, discovering if needed.
    pub async fn blender_installation(&mut self) -> BlendResult<&BlenderInstallation> {
        if self.installation.is_none() {
            let installation = self.discovery.discover().await?.clone();
            self.installation = Some(installation);
        }
        Ok(self.installation.as_ref().unwrap())
    }

    /// Checks if Blender is available.
    pub async fn is_blender_available(&mut self) -> bool {
        self.blender_installation().await.is_ok()
    }

    /// Returns the Blender version if available.
    pub async fn blender_version(&mut self) -> Option<String> {
        self.blender_installation()
            .await
            .ok()
            .map(|i| i.version.to_string())
    }

    /// Imports a .blend file with default options.
    ///
    /// The output file will be placed next to the source file with the appropriate extension.
    pub async fn import(&mut self, source_path: impl AsRef<Path>) -> BlendResult<ConversionResult> {
        self.import_with_options(source_path, self.default_options.clone())
            .await
    }

    /// Imports a .blend file with custom options.
    pub async fn import_with_options(
        &mut self,
        source_path: impl AsRef<Path>,
        options: ConversionOptions,
    ) -> BlendResult<ConversionResult> {
        let source_path = source_path.as_ref();
        let output_path = self.default_output_path(source_path, &options);

        self.import_to_with_options(source_path, &output_path, options)
            .await
    }

    /// Imports a .blend file to a specific output path.
    pub async fn import_to(
        &mut self,
        source_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> BlendResult<ConversionResult> {
        self.import_to_with_options(source_path, output_path, self.default_options.clone())
            .await
    }

    /// Imports a .blend file to a specific output path with custom options.
    pub async fn import_to_with_options(
        &mut self,
        source_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        options: ConversionOptions,
    ) -> BlendResult<ConversionResult> {
        let source_path = source_path.as_ref();
        let output_path = output_path.as_ref();

        // Validate source
        if !source_path.exists() {
            return Err(BlendError::BlendFileNotFound {
                path: source_path.to_path_buf(),
            });
        }

        if source_path.extension().and_then(|e| e.to_str()) != Some("blend") {
            return Err(BlendError::InvalidBlendFile {
                path: source_path.to_path_buf(),
                message: "File does not have .blend extension".to_string(),
            });
        }

        // Get Blender installation
        let installation = self.blender_installation().await?.clone();

        info!(
            "Importing {} -> {} (Blender {})",
            source_path.display(),
            output_path.display(),
            installation.version
        );

        // Create conversion job
        let mut job = ConversionJob::new(source_path, output_path, options, installation);

        // Execute with cache
        let result = if let Some(ref cache) = self.cache {
            let mut cache_guard = cache.write().await;
            job.execute(Some(&mut *cache_guard)).await?
        } else {
            job.execute(None).await?
        };

        if result.from_cache {
            info!(
                "Loaded from cache: {} ({} bytes)",
                result.output_path.display(),
                result.output_size
            );
        } else {
            info!(
                "Conversion complete: {} ({} bytes, {:.2}s)",
                result.output_path.display(),
                result.output_size,
                result.duration.as_secs_f64()
            );
        }

        Ok(result)
    }

    /// Decomposes a .blend scene into individual per-object asset files.
    ///
    /// Uses `ConversionOptions::scene_decomposition()` defaults.
    /// Assets are written to `{output_dir}/{blend_stem}/`.
    pub async fn decompose(
        &mut self,
        source_path: impl AsRef<Path>,
        output_dir: impl AsRef<Path>,
    ) -> BlendResult<DecompositionResult> {
        self.decompose_with_options(
            source_path,
            output_dir,
            ConversionOptions::scene_decomposition(),
        )
        .await
    }

    /// Decomposes a .blend scene with custom options.
    pub async fn decompose_with_options(
        &mut self,
        source_path: impl AsRef<Path>,
        output_dir: impl AsRef<Path>,
        options: ConversionOptions,
    ) -> BlendResult<DecompositionResult> {
        let source_path = source_path.as_ref();
        let output_dir = output_dir.as_ref();

        if !source_path.exists() {
            return Err(BlendError::BlendFileNotFound {
                path: source_path.to_path_buf(),
            });
        }

        if source_path.extension().and_then(|e| e.to_str()) != Some("blend") {
            return Err(BlendError::InvalidBlendFile {
                path: source_path.to_path_buf(),
                message: "File does not have .blend extension".to_string(),
            });
        }

        let installation = self.blender_installation().await?.clone();

        info!(
            "Decomposing {} -> {} (Blender {})",
            source_path.display(),
            output_dir.display(),
            installation.version
        );

        let decomposer = SceneDecomposer::new(source_path, output_dir, options, installation);
        decomposer.execute().await
    }

    /// Decomposes a .blend scene and processes textures in one step.
    ///
    /// This is the recommended high-level API: it runs decomposition,
    /// then converts HDR/EXR textures to PNG, generates thumbnails,
    /// and enforces resolution limits.
    pub async fn decompose_and_process(
        &mut self,
        source_path: impl AsRef<Path>,
        output_dir: impl AsRef<Path>,
        options: ConversionOptions,
        texture_config: Option<TextureProcessingConfig>,
    ) -> BlendResult<(DecompositionResult, TextureProcessingResult)> {
        let result = self
            .decompose_with_options(source_path, output_dir, options)
            .await?;

        let tex_config = texture_config.unwrap_or_default();
        let tex_result = process_decomposition_textures(&result, &tex_config).map_err(|e| {
            BlendError::ConversionFailed {
                message: format!("Texture processing failed: {e}"),
                exit_code: None,
                stderr: String::new(),
                blender_output: None,
            }
        })?;

        Ok((result, tex_result))
    }

    /// Starts an import job that can be monitored and cancelled.
    ///
    /// Returns a handle that allows monitoring progress and cancelling the operation.
    pub async fn start_import(
        &mut self,
        source_path: impl AsRef<Path>,
    ) -> BlendResult<ImportHandle> {
        self.start_import_with_options(source_path, self.default_options.clone())
            .await
    }

    /// Starts an import job with custom options.
    pub async fn start_import_with_options(
        &mut self,
        source_path: impl AsRef<Path>,
        options: ConversionOptions,
    ) -> BlendResult<ImportHandle> {
        let source_path = source_path.as_ref().to_path_buf();
        let output_path = self.default_output_path(&source_path, &options);

        // Validate source
        if !source_path.exists() {
            return Err(BlendError::BlendFileNotFound {
                path: source_path.clone(),
            });
        }

        // Get Blender installation
        let installation = self.blender_installation().await?.clone();

        // Create conversion job
        let job = ConversionJob::new(&source_path, &output_path, options.clone(), installation);
        let progress = job.progress();
        let cancellation = job.cancellation_token();

        // Clone cache for async task
        let cache = self.cache.clone();

        // Spawn the conversion task
        let handle = tokio::spawn(async move {
            let mut job = job;
            if let Some(ref cache) = cache {
                let mut cache_guard = cache.write().await;
                job.execute(Some(&mut *cache_guard)).await
            } else {
                job.execute(None).await
            }
        });

        Ok(ImportHandle {
            source_path,
            output_path,
            progress,
            cancellation,
            handle,
        })
    }

    /// Invalidates the cache entry for a specific source file.
    pub async fn invalidate_cache(&mut self, source_path: impl AsRef<Path>) -> BlendResult<bool> {
        if let Some(ref cache) = self.cache {
            let mut cache_guard = cache.write().await;
            cache_guard.invalidate(source_path.as_ref())
        } else {
            Ok(false)
        }
    }

    /// Clears the entire conversion cache.
    pub async fn clear_cache(&mut self) -> BlendResult<()> {
        if let Some(ref cache) = self.cache {
            let mut cache_guard = cache.write().await;
            cache_guard.clear()
        } else {
            Ok(())
        }
    }

    /// Returns cache statistics.
    pub async fn cache_stats(&self) -> Option<crate::cache::CacheStats> {
        if let Some(ref cache) = self.cache {
            Some(cache.read().await.stats())
        } else {
            None
        }
    }

    /// Sets default conversion options.
    pub fn set_default_options(&mut self, options: ConversionOptions) {
        self.default_options = options;
    }

    /// Returns a reference to the default options.
    pub fn default_options(&self) -> &ConversionOptions {
        &self.default_options
    }

    /// Generates the default output path for a source file.
    fn default_output_path(&self, source_path: &Path, options: &ConversionOptions) -> PathBuf {
        let stem = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let ext = options.format.extension();

        if let Some(ref project_root) = self.project_root {
            // Output to project's assets directory
            project_root
                .join("assets")
                .join("models")
                .join(format!("{}.{}", stem, ext))
        } else {
            // Output next to source file
            source_path.with_extension(ext)
        }
    }
}

/// Handle for an in-progress import operation.
pub struct ImportHandle {
    /// Source .blend file path.
    pub source_path: PathBuf,
    /// Target output path.
    pub output_path: PathBuf,
    /// Progress tracker.
    progress: Arc<ProgressTracker>,
    /// Cancellation token.
    cancellation: CancellationToken,
    /// Tokio join handle for the conversion task.
    handle: tokio::task::JoinHandle<BlendResult<ConversionResult>>,
}

impl ImportHandle {
    /// Returns a progress receiver for monitoring the operation.
    pub fn progress(&self) -> ProgressReceiver {
        self.progress.subscribe()
    }

    /// Returns the current progress snapshot.
    pub fn current_progress(&self) -> ConversionProgress {
        self.progress.current()
    }

    /// Cancels the import operation.
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// Checks if the operation has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    /// Checks if the operation is complete.
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    /// Waits for the import to complete and returns the result.
    pub async fn wait(self) -> BlendResult<ConversionResult> {
        self.handle
            .await
            .map_err(|e| BlendError::ConversionFailed {
                message: format!("Import task panicked: {}", e),
                exit_code: None,
                stderr: String::new(),
                blender_output: None,
            })?
    }

    /// Waits for the import with a timeout.
    pub async fn wait_timeout(self, timeout: std::time::Duration) -> BlendResult<ConversionResult> {
        match tokio::time::timeout(timeout, self.handle).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => Err(BlendError::ConversionFailed {
                message: format!("Import task panicked: {}", e),
                exit_code: None,
                stderr: String::new(),
                blender_output: None,
            }),
            Err(_) => Err(BlendError::Timeout {
                operation: "Import".to_string(),
                duration: timeout,
                path: PathBuf::from("unknown"),
                timeout_secs: timeout.as_secs(),
            }),
        }
    }
}

/// Convenience function to quickly import a .blend file.
pub async fn import_blend(source_path: impl AsRef<Path>) -> BlendResult<ConversionResult> {
    let mut importer = BlendImporter::new().await?;
    importer.import(source_path).await
}

/// Convenience function to import a .blend file with options.
pub async fn import_blend_with_options(
    source_path: impl AsRef<Path>,
    options: ConversionOptions,
) -> BlendResult<ConversionResult> {
    let mut importer = BlendImporter::new().await?;
    importer.import_with_options(source_path, options).await
}

/// Checks if Blender is available on this system.
pub async fn is_blender_available() -> bool {
    let mut discovery = BlenderDiscovery::new();
    discovery.discover().await.is_ok()
}

/// Returns the discovered Blender version, if available.
pub async fn blender_version() -> Option<String> {
    let mut discovery = BlenderDiscovery::new();
    discovery
        .discover()
        .await
        .ok()
        .map(|i| i.version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importer_config_default() {
        let config = BlendImporterConfig::default();
        assert!(config.cache_enabled);
        assert!(config.project_root.is_none());
    }

    #[test]
    fn test_default_output_path() {
        let config = BlendImporterConfig::default();
        let importer = BlendImporter {
            discovery: BlenderDiscovery::new(),
            cache: None,
            default_options: config.default_options.clone(),
            project_root: None,
            installation: None,
        };

        let source = PathBuf::from("/test/model.blend");
        let output = importer.default_output_path(&source, &config.default_options);
        assert!(output.to_string_lossy().contains("model.glb"));
    }

    #[test]
    fn test_default_output_path_with_project() {
        let config = BlendImporterConfig::default();
        let importer = BlendImporter {
            discovery: BlenderDiscovery::new(),
            cache: None,
            default_options: config.default_options.clone(),
            project_root: Some(PathBuf::from("/project")),
            installation: None,
        };

        let source = PathBuf::from("/test/model.blend");
        let output = importer.default_output_path(&source, &config.default_options);
        assert!(output.starts_with("/project/assets/models"));
    }
}
