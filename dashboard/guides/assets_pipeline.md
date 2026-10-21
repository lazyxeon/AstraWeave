# Asset Pipeline: Autonomous PolyHaven Integration

**TL;DR**: AstraWeave automatically fetches high-quality CC0 assets from PolyHaven using a manifest-driven pipeline. No manual downloads, no bloated repos, fully reproducible builds.

---

## 🎯 Overview

The **astraweave-assets** tool provides:

1. **Autonomous Fetching**: PolyHaven API client resolves download URLs automatically
2. **Manifest-Driven**: Declarative TOML manifest (no hardcoded URLs)
3. **On-Demand Loading**: Engine can auto-fetch missing assets at runtime
4. **Integrity & Caching**: SHA256 verification, lockfile tracking, deduplication
5. **CI/CD Ready**: GitHub Actions for prefetching artifacts
6. **Zero Manual Work**: Never download files by hand again!

---

## 📋 Quick Start (5 Minutes)

### 1. Run the Fetcher

```powershell
# From VS Code Command Palette:
Tasks: Run Build Task → "Fetch PolyHaven Assets"

# Or via terminal:
cargo run -p astraweave-assets -- fetch
```

**Output**:
```
🚀 AstraWeave Asset Fetcher
📋 Manifest: assets/polyhaven_manifest.toml
📁 Output: assets/_downloaded

🖼️  Fetching texture: aerial_rocks (aerial_rocks_02)
Downloading aerial_rocks_02_diff_2k.png
   ✅ Downloaded 5 maps

🌄 Fetching HDRI: kloppenheim (kloppenheim_06_puresky)
   ✅ Downloaded

╔═══════════════╦════════════════════╦═════════╦══════════════╦════════════╦══════╗
║ Handle        ║ Asset ID           ║ Type    ║ Status       ║ Resolution ║ Files║
╠═══════════════╬════════════════════╬═════════╬══════════════╬════════════╬══════╣
║ aerial_rocks  ║ aerial_rocks_02    ║ texture ║ ✅ downloaded ║ 2k         ║ 5 maps║
║ metal_plate   ║ metal_plate        ║ texture ║ ✅ downloaded ║ 2k         ║ 5 maps║
║ kloppenheim   ║ kloppenheim_06_... ║ hdri    ║ ✅ downloaded ║ 2k         ║ 1 maps║
╚═══════════════╩════════════════════╩═════════╩══════════════╩════════════╩══════╝

📊 Summary:
  Total assets: 8
  ✅ Downloaded: 8
  💾 Cached: 0
  ❌ Failed: 0
```

### 2. Files Are Now Available

```
assets/
├── _downloaded/
│   ├── aerial_rocks/
│   │   ├── aerial_rocks_albedo.png
│   │   ├── aerial_rocks_normal.png
│   │   ├── aerial_rocks_roughness.png
│   │   ├── aerial_rocks_ao.png
│   │   └── aerial_rocks_height.png
│   ├── metal_plate/
│   │   └── ... (5 maps)
│   ├── kloppenheim/
│   │   └── kloppenheim_hdri.exr
│   └── ATTRIBUTION.txt  # Auto-generated CC0 provenance
└── polyhaven_manifest.toml
```

### 3. Use in Your Game Code

```rust
use astraweave_assets::ensure_asset;
use std::path::PathBuf;

// Option A: Auto-fetch if missing (async)
let paths = ensure_asset(
    &PathBuf::from("assets/polyhaven_manifest.toml"),
    "aerial_rocks"
).await?;

// paths = [
//   "assets/_downloaded/aerial_rocks/aerial_rocks_albedo.png",
//   "assets/_downloaded/aerial_rocks/aerial_rocks_normal.png",
//   ...
// ]

// Option B: Load directly (assumes already fetched)
let albedo_path = PathBuf::from("assets/_downloaded/aerial_rocks/aerial_rocks_albedo.png");
```

---

## 📝 Manifest Format

**File**: `assets/polyhaven_manifest.toml`

```toml
# Global settings
output_dir = "assets/_downloaded"
cache_dir = ".asset_cache"

# === TEXTURES ===
[textures."aerial_rocks"]  # Your custom handle
id = "aerial_rocks_02"      # PolyHaven asset ID
kind = "texture"
res = "2k"                  # 1k, 2k, 4k, 8k (fallbacks automatically)
maps = ["albedo", "normal", "roughness", "ao", "height"]
tags = ["biome:rocky", "usage:terrain"]  # Optional organization

# === HDRIs ===
[hdris."kloppenheim"]
id = "kloppenheim_06_puresky"
kind = "hdri"
res = "2k"

# === MODELS (future) ===
# [models."rock_set"]
# id = "rock_collection_a"
# kind = "model"
# res = "2k"
# format = "glb"
```

**Key Points**:
- **No URLs**: The CLI queries PolyHaven API to resolve URLs
- **Fallback Resolutions**: If 2k unavailable, tries 1k → 4k → 8k
- **Map Aliases**: "albedo" also tries "diff", "diffuse", "color"
- **Tags**: Optional metadata for organization (not used by fetcher)

---

## 🔗 MaterialManager Integration

### Loading Textures into wgpu

**Pattern 1: Batch Load (Startup)**

```rust
use astraweave_render::MaterialManager;
use std::path::Path;

// Ensure assets are downloaded
cargo run -p astraweave-assets -- fetch  // Run once before game starts

// Load into MaterialManager
let mut material_manager = MaterialManager::new();

// Load all textures from a biome
let stats = material_manager
    .load_biome(device, queue, Path::new("assets/_downloaded"))
    .await?;

println!("Loaded {} materials", stats.layers_loaded);

// Use in shader
let aerial_rocks_id = material_manager.get_material_id("aerial_rocks")?;
```

**Pattern 2: On-Demand Load (Runtime)**

```rust
use astraweave_assets::ensure_asset;
use std::path::PathBuf;

// Auto-fetch if missing, return paths
let paths = ensure_asset(
    &PathBuf::from("assets/polyhaven_manifest.toml"),
    "metal_plate"
).await?;

// Load individual textures
for path in paths {
    let texture = device.create_texture_from_file(&path)?;
    bind_group.insert(texture);
}
```

### HDRI Skybox Example

```rust
use astraweave_render::IblManager;

// Ensure HDRI downloaded
let paths = ensure_asset(
    &PathBuf::from("assets/polyhaven_manifest.toml"),
    "kloppenheim"
).await?;

let hdri_path = paths.iter()
    .find(|p| p.extension() == Some("exr".as_ref()))
    .ok_or_else(|| anyhow::anyhow!("HDRI not found"))?;

// Load into IBL manager
let mut ibl_manager = IblManager::new(device, queue);
ibl_manager.load_hdri(hdri_path)?;

// Use in renderer
render_pass.set_bind_group(2, &ibl_manager.bind_group(), &[]);
```

### GLB Model Example (Future)

```rust
use astraweave_asset::ModelLoader;

// Ensure model downloaded
let paths = ensure_asset(
    &PathBuf::from("assets/polyhaven_manifest.toml"),
    "rock_set"
).await?;

let glb_path = paths.iter()
    .find(|p| p.extension() == Some("glb".as_ref()))
    .ok_or_else(|| anyhow::anyhow!("GLB not found"))?;

// Load model
let model = ModelLoader::load_glb(glb_path)?;

// Spawn in ECS
world.spawn((
    Transform::from_xyz(0.0, 0.0, 0.0),
    Mesh(model.meshes[0].clone()),
    Material(model.materials[0].clone()),
));
```

---

## 🚀 CLI Commands

### `fetch` - Download Assets

```powershell
# Fetch all assets from manifest
cargo run -p astraweave-assets -- fetch

# Custom manifest path
cargo run -p astraweave-assets -- fetch --manifest path/to/manifest.toml

# JSON output (for CI)
cargo run -p astraweave-assets -- fetch --json

# Quiet mode (no progress bars)
cargo run -p astraweave-assets -- fetch --quiet

# Allow files >600MB
cargo run -p astraweave-assets -- fetch --allow-large
```

### `check` - Verify Integrity

```powershell
# Check all cached assets exist
cargo run -p astraweave-assets -- check

# JSON output
cargo run -p astraweave-assets -- check --json
```

**Output**:
```
✅ aerial_rocks: OK
✅ metal_plate: OK
❌ cobblestone: Missing files

📊 Total: 3, Valid: 2, Invalid: 1
```

### `prune` - Clean Orphaned Files

```powershell
# Remove files not in manifest (dry run)
cargo run -p astraweave-assets -- prune --dry-run

# Actually delete
cargo run -p astraweave-assets -- prune
```

**Output**:
```
🗑️  Deleted: assets/_downloaded/old_texture/
🗑️  Deleted: assets/_downloaded/unused.png

📊 Total pruned: 2
```

---

## 🔒 Lockfile & Caching

**File**: `.asset_cache/polyhaven.lock`

```toml
version = 1

[assets."aerial_rocks"]
handle = "aerial_rocks"
id = "aerial_rocks_02"
kind = "texture"
resolved_res = "2k"
timestamp = "2025-10-17T12:00:00Z"

[assets."aerial_rocks".urls]
albedo = "https://dl.polyhaven.org/file/ph-assets/Textures/zip/2k/aerial_rocks_02_2k.zip"
normal = "..."

[assets."aerial_rocks".paths]
albedo = "assets/_downloaded/aerial_rocks/aerial_rocks_albedo.png"
normal = "..."

[assets."aerial_rocks".hashes]
albedo = "a1b2c3d4e5f6..."  # SHA256
normal = "..."
```

**Features**:
- **Idempotent Fetching**: Cached assets are skipped
- **Integrity Verification**: SHA256 hashes for all files
- **Provenance Tracking**: Original URLs and timestamps
- **Resume Support**: Partial downloads can be resumed

---

## 📦 CI/CD Integration (GitHub Actions)

**File**: `.github/workflows/assets-prefetch.yml`

```yaml
name: Prefetch PolyHaven Assets

on:
  workflow_dispatch:  # Manual trigger
  pull_request:
    paths:
      - 'assets/polyhaven_manifest.toml'  # Only when manifest changes

jobs:
  prefetch:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Cache PolyHaven Assets
        uses: actions/cache@v4
        with:
          path: |
            assets/_downloaded
            .asset_cache
          key: polyhaven-${{ hashFiles('assets/polyhaven_manifest.toml') }}
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Fetch Assets
        run: |
          cargo run -p astraweave-assets -- fetch --quiet --json > assets_summary.json
          cat assets_summary.json
      
      - name: Upload Asset Artifact
        uses: actions/upload-artifact@v4
        with:
          name: polyhaven-assets
          path: assets/_downloaded
          retention-days: 30
```

**Build Job (Consume Artifacts)**:

```yaml
jobs:
  build:
    needs: prefetch
    steps:
      - uses: actions/checkout@v4
      
      - name: Download Assets
        uses: actions/download-artifact@v4
        with:
          name: polyhaven-assets
          path: assets/_downloaded
      
      - name: Build Game
        run: cargo build --release -p my_game
```

**Benefits**:
- **Cache by Manifest Hash**: Reruns are instant if manifest unchanged
- **Artifact Sharing**: Build jobs download pre-fetched assets (no API calls)
- **Manual Trigger**: workflow_dispatch for on-demand prefetching

---

## 🏗️ On-Demand Runtime Fetching

**Use Case**: Dynamically load assets when entering new biomes/levels

```rust
use astraweave_assets::ensure_asset::ensure_asset;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Player enters "Desert" biome
    let biome = "desert_terrain";
    
    // Check if asset exists locally
    if !is_available(&PathBuf::from("assets/polyhaven_manifest.toml"), biome).await? {
        println!("📥 Downloading {} assets...", biome);
        
        // Auto-fetch in background
        let paths = ensure_asset(
            &PathBuf::from("assets/polyhaven_manifest.toml"),
            biome
        ).await?;
        
        println!("✅ Downloaded {} files", paths.len());
    }
    
    // Load into renderer
    material_manager.load_biome(device, queue, &paths).await?;
    
    Ok(())
}
```

**Integration with MaterialManager**:

```rust
use astraweave_render::MaterialManager;
use astraweave_assets::ensure_asset::ensure_asset;

impl MaterialManager {
    /// Load biome with auto-fetch fallback
    pub async fn load_biome_auto(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        biome_handle: &str,
        manifest_path: &PathBuf,
    ) -> anyhow::Result<MaterialLoadStats> {
        // Try to load from cache
        let biome_dir = PathBuf::from("assets/_downloaded").join(biome_handle);
        
        if !biome_dir.exists() {
            // Auto-fetch if missing
            eprintln!("📥 Biome '{}' not cached, fetching from PolyHaven...", biome_handle);
            ensure_asset(manifest_path, biome_handle).await?;
        }
        
        // Load into GPU
        self.load_biome(device, queue, &biome_dir).await
    }
}
```

**Usage**:

```rust
// In game code (e.g., level transition system)
material_manager.load_biome_auto(
    device,
    queue,
    "forest_ground",  // Asset handle from manifest
    &PathBuf::from("assets/polyhaven_manifest.toml")
).await?;
```

---

## 🛡️ Safety & Size Controls

### File Size Limits

**Default**: 600 MB per file (prevents accidental 8K downloads)

```powershell
# Override for large HDRIs
cargo run -p astraweave-assets -- fetch --allow-large
```

### .gitignore Integration

**File**: `.gitignore`

```gitignore
# Don't commit downloaded assets (reproducible via manifest)
assets/_downloaded/
.asset_cache/

# DO commit these:
# assets/polyhaven_manifest.toml
# .github/workflows/assets-prefetch.yml
```

**Why**:
- Keeps repo slim (<100 MB even with huge asset library)
- Reproducible builds (manifest + CI = same assets everywhere)
- No LFS needed (assets are external)

### Integrity & Security

**Hash Verification**:
```rust
use astraweave_assets::Downloader;

// Verify file integrity
let is_valid = Downloader::verify_hash(
    Path::new("assets/_downloaded/aerial_rocks/aerial_rocks_albedo.png"),
    "a1b2c3d4e5f6..."  // Expected SHA256
).await?;

if !is_valid {
    eprintln!("⚠️  Corrupted file detected, re-downloading...");
}
```

**Attribution Tracking**:

**File**: `assets/_downloaded/ATTRIBUTION.txt`

```
# AstraWeave Asset Attribution

All PolyHaven assets are licensed under CC0 (Public Domain).
Attribution is not required but we keep records for provenance.

---

## aerial_rocks (2025-10-17T12:00:00Z)

- **Asset ID**: aerial_rocks_02
- **Type**: texture
- **Resolution**: 2k
- **Source**: https://polyhaven.com/a/aerial_rocks_02
- **License**: CC0 (Public Domain)
- **Tags**: biome:rocky, usage:terrain
- **Downloads**: 1,234,567

**Files**:
  - `albedo`: https://dl.polyhaven.org/.../aerial_rocks_02_diff_2k.png
  - `normal`: https://dl.polyhaven.org/.../aerial_rocks_02_nor_gl_2k.png
  ...
```

---

## 🧪 Testing

### Unit Tests

```powershell
# Test config parsing
cargo test -p astraweave-assets -- config

# Test API URL resolution
cargo test -p astraweave-assets -- polyhaven
```

### Integration Tests (Mock API)

```powershell
# Test with mocked HTTP responses
cargo test -p astraweave-assets
```

### Live API Tests (Optional)

```powershell
# Test against real PolyHaven API
cargo test -p astraweave-assets --features live-api-tests
```

---

## 🔧 Advanced Configuration

### Custom Manifest Path

```rust
use astraweave_assets::AssetManifest;

let manifest = AssetManifest::load(Path::new("custom/path.toml"))?;
```

### Programmatic Manifest Generation

```rust
use astraweave_assets::{AssetManifest, TextureAsset};
use std::collections::HashMap;

let mut manifest = AssetManifest {
    output_dir: PathBuf::from("assets/_downloaded"),
    cache_dir: PathBuf::from(".asset_cache"),
    textures: HashMap::new(),
    hdris: HashMap::new(),
    models: HashMap::new(),
};

manifest.textures.insert(
    "my_texture".to_string(),
    TextureAsset {
        id: "aerial_rocks_02".to_string(),
        kind: "texture".to_string(),
        res: "2k".to_string(),
        maps: vec!["albedo".to_string(), "normal".to_string()],
        tags: vec![],
    }
);

manifest.save(Path::new("generated_manifest.toml"))?;
```

### Custom Download Locations

```toml
# Per-biome manifests
output_dir = "assets/biomes/desert"
cache_dir = ".cache/desert"
```

---

## 📊 Performance Metrics

**Typical Fetch Times** (2K resolution, good internet):

| Asset Type | Maps | Size | Time |
|------------|------|------|------|
| Texture | 5 maps | ~50 MB | 15-30s |
| HDRI | 1 file | ~30 MB | 10-20s |
| Model (future) | 1 GLB + textures | ~100 MB | 30-60s |

**Cache Hit Performance**:
- **Cold Start**: 2-5 minutes for 10 textures + 3 HDRIs (first run)
- **Warm Start**: <1 second (lockfile validation, no downloads)

**Parallel Downloads**:
- Current: Sequential (one map at a time)
- Future: Parallel downloads with `tokio::spawn` (5-10× faster)

---

## 🚨 Troubleshooting

### Problem: "Asset not found" error

**Cause**: PolyHaven asset ID incorrect or asset removed

**Solution**:
```powershell
# Verify asset exists on PolyHaven
https://polyhaven.com/a/<asset_id>

# Try alternative ID or check PolyHaven website
```

### Problem: "HTTP 404" during download

**Cause**: PolyHaven URL structure changed or resolution unavailable

**Solution**:
```toml
# Try lower resolution
res = "1k"  # Instead of "4k"

# Check API directly
curl https://api.polyhaven.com/files/<asset_id>
```

### Problem: "Corrupted file" or hash mismatch

**Cause**: Network issue during download

**Solution**:
```powershell
# Delete corrupted file and re-fetch
rm -r assets/_downloaded/<handle>
rm .asset_cache/polyhaven.lock  # Force re-download

cargo run -p astraweave-assets -- fetch
```

### Problem: Progress bars not showing

**Cause**: Terminal doesn't support ANSI colors

**Solution**:
```powershell
# Use quiet mode
cargo run -p astraweave-assets -- fetch --quiet
```

---

## 📚 API Reference

### Library Functions

```rust
// Ensure asset exists (fetch if missing)
pub async fn ensure_asset(
    manifest_path: &PathBuf,
    handle: &str
) -> Result<Vec<PathBuf>>

// Check if asset is cached
pub async fn is_available(
    manifest_path: &PathBuf,
    handle: &str
) -> Result<bool>
```

### CLI Commands

```bash
# Fetch all assets from manifest
astraweave-assets fetch [OPTIONS]
  --manifest <PATH>   # Manifest path (default: assets/polyhaven_manifest.toml)
  --json              # JSON output
  --quiet             # No progress bars
  --allow-large       # Allow files >600MB

# Check asset integrity
astraweave-assets check [OPTIONS]
  --manifest <PATH>
  --json

# Prune orphaned files
astraweave-assets prune [OPTIONS]
  --manifest <PATH>
  --dry-run          # Don't actually delete
```

---

## 🎯 Next Steps

1. **Run Your First Fetch**:
   ```powershell
   cargo run -p astraweave-assets -- fetch
   ```

2. **Integrate with MaterialManager**:
   ```rust
   use astraweave_assets::ensure_asset;
   let paths = ensure_asset(&manifest, "aerial_rocks").await?;
   ```

3. **Add to CI/CD**:
   - Copy `.github/workflows/assets-prefetch.yml` example
   - Cache by manifest hash for fast builds

4. **Explore PolyHaven**:
   - Browse https://polyhaven.com/textures
   - Find asset IDs, add to manifest
   - Run `fetch` and enjoy!

---

## 📝 License & Attribution

- **PolyHaven Assets**: CC0 (Public Domain) - No attribution required
- **astraweave-assets Tool**: MIT License (same as AstraWeave)
- **Auto-generated Attribution**: `assets/_downloaded/ATTRIBUTION.txt`

**Provenance**: All downloads are tracked with URLs, timestamps, and hashes for complete reproducibility.

---

**Questions?** Check `tools/astraweave-assets/README.md` or open an issue!
