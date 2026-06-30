#![forbid(unsafe_code)]
//! Asset Pipeline - Texture compression and mesh optimization for AstraWeave
//!
//! **Status: in-design / not wired into any live path.** These offline asset
//! transforms are implemented and unit-tested, but have **no live (non-test)
//! caller** in the workspace — the live bake path is `tools/aw_asset_cli`, which
//! reimplements compression + validation separately. This crate provides:
//! - **Texture Compression**: BC7 (desktop, via `intel_tex`), ASTC (mobile, via `basisu` CLI)
//! - **Mesh Optimization**: Vertex cache, overdraw reduction
//! - **Validation**: Quality checks, size verification
//!
//! The BC7/KTX2 **cook path is deferred to a post-v1.0 engine/compression-pipeline
//! owner** (R-series M2/E4, relabel-and-defer, 2026-06-30): the v1.0 render path
//! consumes raw PNG→RGBA8 and has no GPU-compressed-upload path, so cooking is a
//! deferrable VRAM/load optimization, not a v1.0 requirement. See
//! `docs/audits/e4_cook_path_recon_2026-06.md` and `docs/architecture/asset.md`.
//!
//! ## Features
//! - `bc7`: BC7 texture compression (default, desktop)
//! - `astc`: ASTC texture compression (mobile)
//!
//! ## Example
//! ```no_run
//! use astraweave_asset_pipeline::texture::compress_bc7;
//! use image::RgbaImage;
//!
//! # fn example() -> anyhow::Result<()> {
//! let rgba_image = image::open("texture.png")?.to_rgba8();
//! let compressed = compress_bc7(&rgba_image)?;
//!
//! println!("Compressed from {} to {} bytes ({:.1}% reduction)",
//!     rgba_image.len(),
//!     compressed.len(),
//!     100.0 * (1.0 - compressed.len() as f32 / rgba_image.len() as f32)
//! );
//! # Ok(())
//! # }
//! ```

pub mod mesh;
pub mod texture;
pub mod validator;

pub use mesh::{optimize_mesh, MeshOptimizationStats};
pub use texture::{compress_bc7, CompressionStats};
pub use validator::{AssetValidator, ValidationReport};

/// Re-export meshopt for direct access
pub use meshopt;
