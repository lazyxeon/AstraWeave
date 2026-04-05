#![forbid(unsafe_code)]
//! # AstraWeave Blend Import System
//!
//! Production-grade Blender `.blend` file import system for the AstraWeave game engine.
//!
//! ## Overview
//!
//! This crate provides seamless integration of Blender files into the AstraWeave asset pipeline
//! by leveraging Blender's own export capabilities through subprocess invocation. This approach
//! ensures 100% feature coverage and accurate conversion while maintaining reasonable performance
//! through intelligent caching.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │   .blend file   │────▶│  BlendImporter   │────▶│   .glb cache    │
//! └─────────────────┘     └──────────────────┘     └─────────────────┘
//!                                  │                        │
//!                                  ▼                        ▼
//!                         ┌──────────────────┐     ┌─────────────────┐
//!                         │ BlenderDiscovery │     │  gltf_loader    │
//!                         │ (find Blender)   │     │ (existing)      │
//!                         └──────────────────┘     └─────────────────┘
//! ```
//!
//! ## Features
//!
//! - **Cross-platform Blender discovery**: Windows registry, macOS mdfind, Linux which/paths
//! - **Version validation**: Minimum Blender 2.93+ required for modern glTF export
//! - **Intelligent caching**: SHA-256 content-based cache invalidation
//! - **Progress reporting**: Real-time conversion progress via channel
//! - **Cancellation support**: Abort long-running conversions
//! - **Timeout handling**: Configurable timeout (default 120s)
//! - **Embedded texture unpacking**: Deterministic texture extraction
//! - **Linked library support**: Recursive processing with dependency tracking
//!
//! ## Usage
//!
//! ```rust,ignore
//! use astraweave_blend::{BlendImporter, ConversionOptions};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create importer (auto-discovers Blender)
//!     let mut importer = BlendImporter::new().await?;
//!     
//!     // Import a .blend file
//!     let handle = importer.import(
//!         Path::new("character.blend"),
//!         None, // Use default options
//!         None, // No custom output path
//!     ).await?;
//!     
//!     let result = handle.await_result().await?;
//!     println!("Imported to: {}", result.output_path.display());
//!     Ok(())
//! }
//! ```
//!
//! ## Requirements
//!
//! - Blender 2.93 or later installed on the system
//! - Blender must be accessible via PATH or standard installation locations
//!
//! ## Cache Directory
//!
//! Converted files are cached in `{project}/.astraweave/blend_cache/`:
//! - `manifest.ron` - Cache metadata and file hashes
//! - `{hash}.glb` - Converted GLB files
//! - `textures/{hash}_{name}.png` - Unpacked textures

#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_code)]

pub mod cache;
pub mod conversion;
pub mod decomposer;
pub mod discovery;
pub mod error;
pub mod export_script;
pub mod heightmap_raster;
pub mod importer;
pub mod options;
pub mod progress;
pub mod texture_processor;
pub mod version;

/// Test utilities, mock implementations, and property generators.
///
/// Enable the `test-utils` feature to use these utilities.
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

// Re-exports for convenience - core types
pub use discovery::{
    BlenderDiscovery, BlenderDiscoveryConfig, BlenderInstallation, DiscoveryMethod,
};
pub use error::{BlendError, BlendResult};
pub use version::{
    BlenderCapabilities, BlenderVersion, MINIMUM_BLENDER_VERSION, RECOMMENDED_BLENDER_VERSION,
};

// Re-exports - conversion pipeline
pub use cache::{CacheLookup, CacheMissReason, CacheStats, ConversionCache};
pub use conversion::{ConversionJob, ConversionJobBuilder, ConversionResult};
pub use decomposer::{DecomposedAsset, DecompositionResult, SceneDecomposer};
pub use heightmap_raster::{
    ExtractedTerrainMesh, FixedPlacement, RasterizedHeightmap, TerrainBounds,
};
pub use options::{
    ConversionOptions, ConversionOptionsBuilder, OutputFormat, SceneDecompositionOptions,
};
pub use progress::{
    CancellationToken, ConversionProgress, ConversionStage, ProgressReceiver, ProgressTracker,
};
pub use texture_processor::{
    normalize_channel_name, process_decomposition_textures, TextureOutputFormat,
    TextureProcessingConfig, TextureProcessingResult,
};

// Re-exports - high-level API
pub use importer::{
    blender_version, import_blend, import_blend_with_options, is_blender_available,
};
pub use importer::{BlendImporter, BlendImporterConfig, ImportHandle};
