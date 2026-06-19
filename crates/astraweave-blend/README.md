# AstraWeave Blend Import System

Blender `.blend` file import system for the AstraWeave game engine.

## Overview

This crate enables seamless import of `.blend` files into AstraWeave by leveraging Blender's own export capabilities through subprocess invocation. This ensures 100% feature coverage and accurate conversion while maintaining excellent performance through intelligent caching.

## Architecture

```text
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   .blend file   │────▶│  BlendImporter   │────▶│   .glb cache    │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                 │                        │
                                 ▼                        ▼
                        ┌──────────────────┐     ┌─────────────────┐
                        │ BlenderDiscovery │     │  gltf_loader    │
                        │ (find Blender)   │     │ (existing)      │
                        └──────────────────┘     └─────────────────┘
```

## Features

- **Cross-platform Blender discovery**: Windows registry, macOS mdfind, Linux which/paths
- **Version validation**: Minimum Blender 2.93+ required for modern glTF export
- **Intelligent caching**: SHA-256 content-based cache invalidation
- **Progress reporting**: Real-time conversion progress via channel
- **Cancellation support**: Abort long-running conversions
- **Timeout handling**: Configurable timeout (default 120s)
- **Embedded texture unpacking**: Deterministic texture extraction
- **Linked library detection**: Automatically tracks external .blend dependencies from Blender output

## Quick Start

```rust
use astraweave_blend::{BlendImporter, ConversionOptions};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create importer (auto-discovers Blender)
    let mut importer = BlendImporter::new().await?;
    
    // Import a .blend file
    let result = importer.import(Path::new("character.blend")).await?;
    println!("Imported to: {}", result.output_path.display());
    
    Ok(())
}
```

## Progress Tracking

For long imports, you can monitor progress:

```rust
use astraweave_blend::{BlendImporter, ConversionProgress};

async fn import_with_progress() -> anyhow::Result<()> {
    let mut importer = BlendImporter::new().await?;
    
    // Start async import
    let handle = importer.start_import("large_scene.blend").await?;
    
    // Get progress receiver
    let mut progress_rx = handle.progress();
    
    // Monitor in background
    tokio::spawn(async move {
        while let Ok(progress) = progress_rx.recv().await {
            println!(
                "[{:?}] {}: {:.1}%",
                progress.stage,
                progress.message,
                progress.fraction * 100.0
            );
        }
    });
    
    // Wait for result
    let result = handle.wait().await?;
    Ok(())
}
```

## Conversion Options

```rust
use astraweave_blend::{ConversionOptions, OutputFormat};

let options = ConversionOptions::builder()
    .output_format(OutputFormat::Glb)       // Binary glTF (default)
    .apply_transforms(true)                  // Bake transforms
    .export_animations(true)                 // Include animations
    .export_materials(true)                  // Include materials
    .export_textures(true)                   // Include textures
    .texture_embed_mode_embedded()           // Embed textures in GLB
    .timeout_secs(180)                       // 3 minute timeout
    .build();
```

## Cache Directory

Converted files are cached in `{project}/.astraweave/blend_cache/`:
- `manifest.ron` - Cache metadata and file hashes
- `{hash}.glb` - Converted GLB files
- `textures/{hash}_{name}.png` - Unpacked textures

## Requirements

- Blender 2.93 or later installed on the system
- Blender must be accessible via PATH or standard installation locations

## Module Organization

- `discovery` - Cross-platform Blender installation discovery
- `error` - Error types with detailed diagnostics
- `version` - Blender version parsing and capability detection
- `conversion` - Core conversion job execution
- `cache` - SHA-256 content-based caching
- `importer` - High-level import API
- `options` - Conversion configuration
- `progress` - Progress tracking and cancellation
- `export_script` - Python script generation for Blender

## Integration with astraweave-asset

Enable the `blend` feature in `astraweave-asset` to integrate:

```toml
[dependencies]
astraweave-asset = { version = "0.1", features = ["blend"] }
```

Then use `BlendAssetIntegration`:

```rust
use astraweave_asset::blend_asset_integration::BlendAssetIntegration;

async fn import_blend_assets() -> anyhow::Result<()> {
    let mut integration = BlendAssetIntegration::new();
    integration.initialize(Some("./my_project".into())).await?;
    
    let mut db = AssetDatabase::new();
    
    // Import and register in one step
    let (source_guid, mesh_guid) = integration
        .import_and_register(&mut db, Path::new("hero.blend"))
        .await?;
    
    Ok(())
}
```

## License

MIT License - see LICENSE file for details.
