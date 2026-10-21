# Multi-Source Asset Pipeline – Implementation Plan

**Project**: AstraWeave AI-Native Gaming Engine  
**Feature**: Autonomous Multi-Source Asset Fetcher (PolyHaven + Poly Pizza + OpenGameArt)  
**Estimated Duration**: 8-10 hours  
**Status**: 📋 **PLANNING** – Architecture design phase

---

## 🎯 Mission

Expand the autonomous asset pipeline to support **three major CC0/open-source asset repositories**:

1. **PolyHaven** ✅ (Already integrated)
   - Textures (PBR materials)
   - HDRIs (environment lighting)
   - **License**: CC0 (Public Domain)

2. **Poly Pizza** 🍕 (New - Target 1)
   - 3D Models (low-poly characters, props, environments)
   - **License**: CC0 (Public Domain)
   - **API**: https://poly.pizza/api

3. **OpenGameArt** 🎮 (New - Target 2)
   - 2D Sprites, Tilesets, UI
   - 3D Models (characters, NPCs, animals)
   - Audio (music, sound effects)
   - **Licenses**: CC0, CC-BY, CC-BY-SA, GPL, OGA-BY (mixed)
   - **API**: REST API (requires research)

---

## 🏗️ Architecture Design

### Current State (PolyHaven Only)

```
polyhaven_manifest.toml
         ↓
  PolyHavenClient
         ↓
    Downloader
         ↓
    Organizer
         ↓
assets/_downloaded/
```

**Limitations**:
- ❌ Single provider only
- ❌ Hard-coded for textures/HDRIs
- ❌ Assumes CC0 license
- ❌ No model support

### Target State (Multi-Source)

```
asset_manifest.toml (unified)
         ↓
   ┌─────┴─────┬─────────┐
   ▼           ▼         ▼
PolyHaven   PolyPizza  OpenGameArt
 Client      Client      Client
   └─────┬─────┴─────────┘
         ▼
    Downloader (shared)
         ↓
    Organizer (provider-aware)
         ↓
assets/_downloaded/
├── polyhaven/
├── polypizza/
└── opengameart/
```

**Improvements**:
- ✅ **Trait-based providers** (extensible)
- ✅ **Multi-asset types** (textures, models, audio, sprites)
- ✅ **License tracking** (per-asset metadata)
- ✅ **Provider isolation** (separate directories)

---

## 📐 Technical Design

### 1. Provider Trait

**File**: `tools/astraweave-assets/src/provider.rs` (NEW)

```rust
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait AssetProvider: Send + Sync {
    /// Provider name (e.g., "polyhaven", "polypizza", "opengameart")
    fn name(&self) -> &str;

    /// Resolve asset metadata from provider API
    async fn resolve(
        &self,
        asset_id: &str,
        asset_type: AssetType,
        preferences: &AssetPreferences,
    ) -> Result<ResolvedAsset>;

    /// Get download URLs for asset
    async fn get_download_urls(
        &self,
        resolved: &ResolvedAsset,
    ) -> Result<HashMap<String, String>>;

    /// Get license information
    fn get_license(&self, resolved: &ResolvedAsset) -> LicenseInfo;
}

#[derive(Debug, Clone)]
pub enum AssetType {
    Texture,        // PBR material
    Hdri,           // Environment map
    Model3D,        // GLB/GLTF
    Audio,          // OGG/WAV/MP3
    Sprite2D,       // PNG sprite sheet
    Tileset,        // PNG tileset
}

#[derive(Debug, Clone)]
pub struct AssetPreferences {
    pub resolution: Option<String>,     // "1k", "2k", "4k", etc.
    pub format: Option<String>,          // "glb", "gltf", "fbx", "ogg", etc.
    pub lod: Option<u32>,                // Level of detail (0-3)
    pub maps: Option<Vec<String>>,       // Texture maps (albedo, normal, etc.)
}

#[derive(Debug, Clone)]
pub struct ResolvedAsset {
    pub id: String,
    pub provider: String,
    pub asset_type: AssetType,
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub license: LicenseInfo,
    pub urls: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub spdx_id: String,        // "CC0-1.0", "CC-BY-4.0", etc.
    pub name: String,            // "Creative Commons Zero"
    pub url: String,             // License text URL
    pub requires_attribution: bool,
    pub allow_commercial: bool,
    pub allow_derivatives: bool,
    pub share_alike: bool,
}
```

### 2. Unified Manifest Format

**File**: `assets/asset_manifest.toml` (REPLACES polyhaven_manifest.toml)

```toml
# Multi-Source Asset Manifest
# Supports PolyHaven, Poly Pizza, and OpenGameArt

version = 1
output_dir = "assets/_downloaded"
cache_dir = ".asset_cache"

# === POLYHAVEN TEXTURES ===
[[assets]]
handle = "aerial_rocks"
provider = "polyhaven"
type = "texture"
id = "aerial_rocks_02"
resolution = "2k"
maps = ["albedo", "normal", "roughness", "ao", "height"]
tags = ["biome:rocky", "usage:terrain"]

[[assets]]
handle = "metal_plate"
provider = "polyhaven"
type = "texture"
id = "metal_plate"
resolution = "2k"
maps = ["albedo", "normal", "roughness", "metallic", "ao"]
tags = ["material:metal"]

# === POLYHAVEN HDRIS ===
[[assets]]
handle = "spruit_sunrise"
provider = "polyhaven"
type = "hdri"
id = "spruit_sunrise"
resolution = "2k"
tags = ["sky:sunrise", "time:morning"]

# === POLY PIZZA MODELS ===
[[assets]]
handle = "character_knight"
provider = "polypizza"
type = "model"
id = "Low_poly_Knight-9zzjdYXlcwJ"
format = "glb"
lod = 0
tags = ["character:humanoid", "style:low-poly"]

[[assets]]
handle = "npc_villager"
provider = "polypizza"
type = "model"
id = "Villager-Ajn1nqKyiRS"
format = "glb"
lod = 0
tags = ["npc:friendly", "style:low-poly"]

[[assets]]
handle = "prop_tree"
provider = "polypizza"
type = "model"
id = "Tree_Beech-WW0JbmUVGXB"
format = "glb"
lod = 0
tags = ["environment:nature", "prop:vegetation"]

[[assets]]
handle = "animal_deer"
provider = "polypizza"
type = "model"
id = "Deer-kZlPqiFjaOJ"
format = "glb"
lod = 0
tags = ["animal:wildlife", "style:low-poly"]

# === OPENGAMEART AUDIO ===
[[assets]]
handle = "music_ambient_forest"
provider = "opengameart"
type = "audio"
id = "forest-ambience-loop"
format = "ogg"
tags = ["music:ambient", "biome:forest"]
# Note: Check license! OGA has mixed licenses

[[assets]]
handle = "sfx_footstep_grass"
provider = "opengameart"
type = "audio"
id = "footsteps-grass"
format = "ogg"
tags = ["sfx:footstep", "surface:grass"]

# === OPENGAMEART SPRITES ===
[[assets]]
handle = "sprite_character_rpg"
provider = "opengameart"
type = "sprite"
id = "lpc-base-character"
format = "png"
tags = ["character:2d", "style:pixel-art"]

[[assets]]
handle = "tileset_dungeon"
provider = "opengameart"
type = "tileset"
id = "dungeon-tileset-16x16"
format = "png"
tags = ["environment:dungeon", "tile:16x16"]
```

### 3. Provider Implementations

#### PolyPizza Client

**File**: `tools/astraweave-assets/src/polypizza.rs` (NEW)

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::provider::{AssetProvider, AssetType, AssetPreferences, ResolvedAsset, LicenseInfo};

pub struct PolyPizzaClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct PolyPizzaAsset {
    id: String,
    name: String,
    author: String,
    thumbnail: String,
    download_url: String,
    format: String,       // "glb", "gltf"
    poly_count: u32,
    file_size: u64,
}

impl PolyPizzaClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent("AstraWeave/0.1.0")
                .build()?,
            base_url: "https://poly.pizza/api".to_string(),
        })
    }

    async fn fetch_asset(&self, asset_id: &str) -> Result<PolyPizzaAsset> {
        let url = format!("{}/models/{}", self.base_url, asset_id);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Poly Pizza API error: {}", response.status());
        }
        
        let asset = response.json().await?;
        Ok(asset)
    }
}

#[async_trait::async_trait]
impl AssetProvider for PolyPizzaClient {
    fn name(&self) -> &str {
        "polypizza"
    }

    async fn resolve(
        &self,
        asset_id: &str,
        asset_type: AssetType,
        preferences: &AssetPreferences,
    ) -> Result<ResolvedAsset> {
        let asset = self.fetch_asset(asset_id).await?;
        
        Ok(ResolvedAsset {
            id: asset.id.clone(),
            provider: "polypizza".to_string(),
            asset_type,
            name: asset.name,
            description: format!("3D model from Poly Pizza ({})", asset.author),
            author: Some(asset.author),
            license: self.get_license(&asset),
            urls: [("model".to_string(), asset.download_url)]
                .into_iter()
                .collect(),
            metadata: [
                ("format".to_string(), asset.format),
                ("poly_count".to_string(), asset.poly_count.to_string()),
                ("file_size".to_string(), asset.file_size.to_string()),
            ]
            .into_iter()
            .collect(),
        })
    }

    async fn get_download_urls(
        &self,
        resolved: &ResolvedAsset,
    ) -> Result<HashMap<String, String>> {
        Ok(resolved.urls.clone())
    }

    fn get_license(&self, _resolved: &ResolvedAsset) -> LicenseInfo {
        // Poly Pizza is CC0 only
        LicenseInfo {
            spdx_id: "CC0-1.0".to_string(),
            name: "Creative Commons Zero v1.0 Universal".to_string(),
            url: "https://creativecommons.org/publicdomain/zero/1.0/".to_string(),
            requires_attribution: false,
            allow_commercial: true,
            allow_derivatives: true,
            share_alike: false,
        }
    }
}
```

#### OpenGameArt Client

**File**: `tools/astraweave-assets/src/opengameart.rs` (NEW)

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::provider::{AssetProvider, AssetType, AssetPreferences, ResolvedAsset, LicenseInfo};

pub struct OpenGameArtClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct OgaAsset {
    id: u32,
    name: String,
    author: String,
    preview_url: String,
    download_urls: Vec<OgaDownload>,
    licenses: Vec<OgaLicense>,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OgaDownload {
    url: String,
    filename: String,
    format: String,
}

#[derive(Debug, Deserialize)]
struct OgaLicense {
    name: String,        // "CC-BY 3.0", "CC0", "GPL 3.0", "OGA-BY 3.0"
    url: String,
    commercial: bool,
    derivatives: bool,
    share_alike: bool,
}

impl OpenGameArtClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent("AstraWeave/0.1.0")
                .build()?,
            base_url: "https://opengameart.org/api".to_string(),
        })
    }

    async fn fetch_asset(&self, asset_id: &str) -> Result<OgaAsset> {
        // Note: OpenGameArt API may require authentication or have rate limits
        let url = format!("{}/content/{}", self.base_url, asset_id);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            anyhow::bail!("OpenGameArt API error: {}", response.status());
        }
        
        let asset = response.json().await?;
        Ok(asset)
    }

    fn parse_license(&self, oga_license: &OgaLicense) -> LicenseInfo {
        // Map OGA license names to SPDX identifiers
        let spdx_id = match oga_license.name.as_str() {
            "CC0" => "CC0-1.0",
            "CC-BY 3.0" => "CC-BY-3.0",
            "CC-BY 4.0" => "CC-BY-4.0",
            "CC-BY-SA 3.0" => "CC-BY-SA-3.0",
            "CC-BY-SA 4.0" => "CC-BY-SA-4.0",
            "GPL 3.0" => "GPL-3.0",
            "GPL 2.0" => "GPL-2.0",
            "OGA-BY 3.0" => "OGA-BY-3.0",  // OpenGameArt-specific
            _ => "UNKNOWN",
        };

        LicenseInfo {
            spdx_id: spdx_id.to_string(),
            name: oga_license.name.clone(),
            url: oga_license.url.clone(),
            requires_attribution: oga_license.name.contains("BY"),
            allow_commercial: oga_license.commercial,
            allow_derivatives: oga_license.derivatives,
            share_alike: oga_license.share_alike,
        }
    }
}

#[async_trait::async_trait]
impl AssetProvider for OpenGameArtClient {
    fn name(&self) -> &str {
        "opengameart"
    }

    async fn resolve(
        &self,
        asset_id: &str,
        asset_type: AssetType,
        preferences: &AssetPreferences,
    ) -> Result<ResolvedAsset> {
        let asset = self.fetch_asset(asset_id).await?;
        
        // Select preferred download URL by format
        let preferred_format = preferences.format.as_deref().unwrap_or("ogg");
        let download = asset.download_urls.iter()
            .find(|d| d.format == preferred_format)
            .or_else(|| asset.download_urls.first())
            .context("No download URLs available")?;

        // Use first license (prefer CC0 if multiple)
        let license = asset.licenses.iter()
            .find(|l| l.name == "CC0")
            .or_else(|| asset.licenses.first())
            .context("No license information")?;

        Ok(ResolvedAsset {
            id: asset.id.to_string(),
            provider: "opengameart".to_string(),
            asset_type,
            name: asset.name,
            description: format!("Asset from OpenGameArt by {}", asset.author),
            author: Some(asset.author),
            license: self.parse_license(license),
            urls: [(asset_type.to_string(), download.url.clone())]
                .into_iter()
                .collect(),
            metadata: [
                ("format".to_string(), download.format.clone()),
                ("filename".to_string(), download.filename.clone()),
            ]
            .into_iter()
            .collect(),
        })
    }

    async fn get_download_urls(
        &self,
        resolved: &ResolvedAsset,
    ) -> Result<HashMap<String, String>> {
        Ok(resolved.urls.clone())
    }

    fn get_license(&self, resolved: &ResolvedAsset) -> LicenseInfo {
        resolved.license.clone()
    }
}
```

### 4. Enhanced Organizer

**File**: `tools/astraweave-assets/src/organize.rs` (UPDATE)

```rust
// Add provider-aware organization
pub struct AssetOrganizer {
    output_dir: PathBuf,
    cache_dir: PathBuf,
}

impl AssetOrganizer {
    pub async fn organize_multi_provider(
        &self,
        handle: &str,
        provider: &str,
        resolved: &ResolvedAsset,
        downloads: &HashMap<String, DownloadResult>,
    ) -> Result<LockEntry> {
        // Organize into provider-specific subdirectory
        let provider_dir = self.output_dir.join(provider);
        let asset_dir = provider_dir.join(handle);
        
        fs::create_dir_all(&asset_dir).await?;
        
        // Move files and generate lockfile entry
        // (similar to existing organize logic)
        
        // Add license tracking
        self.update_attribution(provider, handle, resolved).await?;
        
        Ok(entry)
    }

    async fn update_attribution(
        &self,
        provider: &str,
        handle: &str,
        resolved: &ResolvedAsset,
    ) -> Result<()> {
        let provider_dir = self.output_dir.join(provider);
        let attr_file = provider_dir.join("ATTRIBUTION.txt");
        
        let mut content = if attr_file.exists() {
            fs::read_to_string(&attr_file).await?
        } else {
            format!("# {} Assets Attribution\n\n", provider.to_uppercase())
        };
        
        content.push_str(&format!(
            "## {}\n",
            handle
        ));
        content.push_str(&format!(
            "- **Name**: {}\n",
            resolved.name
        ));
        if let Some(author) = &resolved.author {
            content.push_str(&format!(
                "- **Author**: {}\n",
                author
            ));
        }
        content.push_str(&format!(
            "- **License**: {} ({})\n",
            resolved.license.name,
            resolved.license.spdx_id
        ));
        content.push_str(&format!(
            "- **URL**: {}\n",
            resolved.license.url
        ));
        if resolved.license.requires_attribution {
            content.push_str("- **Attribution Required**: YES\n");
        }
        content.push_str("\n");
        
        fs::write(&attr_file, content).await?;
        Ok(())
    }
}
```

---

## 📊 Implementation Phases

### Phase 1: Research APIs (30 min) ⏳

**Objectives**:
1. Research Poly Pizza API documentation
2. Research OpenGameArt API (check if public API exists)
3. Identify authentication requirements
4. Test sample API calls

**Deliverables**:
- API endpoint documentation
- Sample responses (JSON)
- Authentication requirements
- Rate limit policies

**Commands**:
```bash
# Test Poly Pizza API
curl https://poly.pizza/api/models/Low_poly_Knight-9zzjdYXlcwJ

# Test OpenGameArt API
curl https://opengameart.org/api/content/12345
```

---

### Phase 2: Architecture Design (45 min) ⏳

**Objectives**:
1. Define `AssetProvider` trait
2. Design unified manifest format
3. Plan provider isolation strategy
4. Design license tracking system

**Deliverables**:
- `provider.rs` trait definition
- `asset_manifest.toml` schema
- License data structures
- Provider directory layout

**Files Created**:
- `tools/astraweave-assets/src/provider.rs`
- `assets/asset_manifest.toml` (example)
- Updated `tools/astraweave-assets/src/config.rs`

---

### Phase 3: Poly Pizza Integration (2 hours) ⏳

**Objectives**:
1. Implement `PolyPizzaClient`
2. Add model downloading support
3. Test with 5 sample models
4. Validate CC0 license tracking

**Deliverables**:
- `polypizza.rs` implementation
- Model organizer logic
- Unit tests (5+ tests)
- Sample models downloaded

**Success Criteria**:
- ✅ Download knight character
- ✅ Download villager NPC
- ✅ Download tree prop
- ✅ Download deer animal
- ✅ All files organized correctly

---

### Phase 4: OpenGameArt Integration (3 hours) ⏳

**Objectives**:
1. Implement `OpenGameArtClient`
2. Add audio downloading support
3. Add sprite/tileset support
4. Implement license tracking

**Deliverables**:
- `opengameart.rs` implementation
- Audio organizer logic
- Sprite organizer logic
- License attribution system
- Unit tests (8+ tests)

**Success Criteria**:
- ✅ Download ambient music (CC0)
- ✅ Download footstep SFX (CC-BY)
- ✅ Download character sprite
- ✅ Download dungeon tileset
- ✅ Attribution file generated with licenses

---

### Phase 5: Unified CLI (1 hour) ⏳

**Objectives**:
1. Update `main.rs` to support all providers
2. Add `--provider` filter flag
3. Update progress reporting
4. Update summary output

**Deliverables**:
- Enhanced `fetch` command
- Provider filtering
- Multi-provider summary

**Commands**:
```bash
# Fetch all assets
cargo run -p astraweave-assets -- fetch

# Fetch only Poly Pizza models
cargo run -p astraweave-assets -- fetch --provider polypizza

# Fetch only OpenGameArt audio
cargo run -p astraweave-assets -- fetch --provider opengameart --type audio
```

---

### Phase 6: Testing & Documentation (1 hour) ⏳

**Objectives**:
1. Integration tests (all providers)
2. License compliance tests
3. Update README
4. Create example manifests

**Deliverables**:
- 20+ integration tests
- License validation tests
- Comprehensive README
- Example manifest with 20+ assets

**Test Scenarios**:
- Multi-provider fetch
- License attribution
- Error handling (API failures)
- Cache behavior
- Provider isolation

---

## 🎯 Success Criteria

### Functional Requirements

| Requirement | Target | Validation |
|-------------|--------|------------|
| PolyHaven support | Existing | ✅ Working |
| Poly Pizza integration | 5+ models | Run fetch, verify downloads |
| OpenGameArt integration | 4+ assets | Run fetch, check licenses |
| License tracking | 100% accurate | Check ATTRIBUTION.txt |
| Provider isolation | Separate dirs | Verify file organization |
| Multi-provider fetch | Single command | `cargo run -- fetch` |
| Tests passing | 100% | `cargo test` |

### Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test coverage | >40% | Lines covered |
| API success rate | >95% | Download success |
| License compliance | 100% | Attribution check |
| Documentation | Comprehensive | README + examples |
| Performance | <2min for 20 assets | Timing |

---

## 📁 File Organization (Final)

```
assets/
├── _downloaded/
│   ├── polyhaven/                   # ✅ Existing
│   │   ├── aerial_rocks/
│   │   ├── metal_plate/
│   │   └── ATTRIBUTION.txt
│   │
│   ├── polypizza/                   # 🆕 New
│   │   ├── character_knight/
│   │   │   └── character_knight.glb
│   │   ├── npc_villager/
│   │   │   └── npc_villager.glb
│   │   ├── prop_tree/
│   │   │   └── prop_tree.glb
│   │   ├── animal_deer/
│   │   │   └── animal_deer.glb
│   │   └── ATTRIBUTION.txt          # CC0 only
│   │
│   └── opengameart/                 # 🆕 New
│       ├── music_ambient_forest/
│       │   └── music_ambient_forest.ogg
│       ├── sfx_footstep_grass/
│       │   └── sfx_footstep_grass.ogg
│       ├── sprite_character_rpg/
│       │   └── sprite_character_rpg.png
│       ├── tileset_dungeon/
│       │   └── tileset_dungeon.png
│       └── ATTRIBUTION.txt          # Mixed licenses!
│
├── asset_manifest.toml              # 🆕 Unified manifest
└── polyhaven_manifest.toml          # ⚠️ Deprecated (backward compat)

.asset_cache/
├── polyhaven.lock
├── polypizza.lock                   # 🆕 New
└── opengameart.lock                 # 🆕 New
```

---

## ⚠️ Known Challenges

### 1. OpenGameArt API Limitations

**Challenge**: OpenGameArt may not have a well-documented public API

**Mitigation**:
- **Plan A**: Use official API if available
- **Plan B**: Parse HTML/RSS feeds
- **Plan C**: Manual URL configuration in manifest

**Impact**: May require 1-2 extra hours for scraping logic

---

### 2. License Complexity

**Challenge**: OpenGameArt has 10+ license types, some incompatible

**Mitigation**:
- Strict license parsing
- Attribution file per provider
- Validation warnings for GPL/restrictive licenses
- User confirmation for non-CC0 assets

**Impact**: Critical for legal compliance

---

### 3. Rate Limiting

**Challenge**: APIs may have rate limits (especially OpenGameArt)

**Mitigation**:
- Respect rate limits (exponential backoff)
- Batch requests where possible
- Cache API responses
- Implement `--dry-run` mode

**Impact**: May slow down large fetches

---

### 4. File Format Variations

**Challenge**: OGA has many formats (WAV, MP3, OGG, FLAC)

**Mitigation**:
- Prefer OGG for audio (universal, CC0-friendly)
- Auto-convert if needed (via ffmpeg)
- Store original + converted versions

**Impact**: +500MB storage for audio conversions

---

## 🔮 Future Enhancements

### Short-Term (After Initial Release)

1. **Format Conversion** (1 hour)
   ```rust
   // Auto-convert audio to OGG
   convert_audio(Path::new("input.wav"), Path::new("output.ogg"))?;
   ```

2. **Model Optimization** (2 hours)
   ```rust
   // Auto-decimate high-poly models
   optimize_model(input_glb, output_glb, target_poly_count)?;
   ```

3. **Batch Operations** (30 min)
   ```bash
   # Fetch by tag
   cargo run -- fetch --tag "character:humanoid"
   ```

### Long-Term (Optional)

4. **Web UI** (8 hours)
   - Browse assets from all providers
   - Preview 3D models/audio
   - One-click add to manifest

5. **Asset Registry** (4 hours)
   - SQLite database for fast search
   - Full-text search across providers
   - Dependency tracking

6. **CI/CD Integration** (2 hours)
   - GitHub Actions cache
   - Automated license validation
   - Asset version pinning

---

## 📊 Time Estimate

| Phase | Duration | Priority |
|-------|----------|----------|
| **Phase 1**: API Research | 30 min | 🔥 Critical |
| **Phase 2**: Architecture | 45 min | 🔥 Critical |
| **Phase 3**: Poly Pizza | 2 hours | 🔥 Critical |
| **Phase 4**: OpenGameArt | 3 hours | 🥇 High |
| **Phase 5**: Unified CLI | 1 hour | 🥈 Medium |
| **Phase 6**: Testing/Docs | 1 hour | 🥈 Medium |
| **Total** | **8.25 hours** | - |

**Buffer**: +1.75 hours for unexpected issues (API changes, debugging)

**Grand Total**: **10 hours** (1.25 work days)

---

## 🚀 Getting Started

### Immediate Next Steps

1. **API Research** (30 min):
   ```bash
   # Test Poly Pizza
   curl -v https://poly.pizza/api/models/Low_poly_Knight-9zzjdYXlcwJ | jq

   # Test OpenGameArt
   curl -v https://opengameart.org/api/content/12345 | jq
   ```

2. **Create Provider Trait** (45 min):
   - Create `tools/astraweave-assets/src/provider.rs`
   - Define `AssetProvider` trait
   - Update `lib.rs` exports

3. **Implement Poly Pizza** (2 hours):
   - Create `tools/astraweave-assets/src/polypizza.rs`
   - Implement `AssetProvider` for `PolyPizzaClient`
   - Test with 5 models

---

## 📝 Acceptance Criteria

**Phase Complete When**:
- ✅ Can fetch assets from all 3 providers
- ✅ License attribution 100% accurate
- ✅ Provider isolation working
- ✅ 20+ tests passing
- ✅ Documentation comprehensive
- ✅ Example manifest with 20+ assets

**Demo Command**:
```bash
# One command fetches everything!
cargo run -p astraweave-assets -- fetch

# Output:
# ✅ Downloaded 5 textures (polyhaven)
# ✅ Downloaded 3 HDRIs (polyhaven)
# ✅ Downloaded 4 models (polypizza)
# ✅ Downloaded 2 audio (opengameart)
# ✅ Downloaded 2 sprites (opengameart)
# 
# 📊 Summary:
#   Total assets: 16
#   ✅ Downloaded: 16
#   ❌ Failed: 0
#   
# ⚖️ License Summary:
#   CC0: 12 assets
#   CC-BY: 3 assets
#   CC-BY-SA: 1 asset
```

---

**Status**: 📋 **READY TO START** – Architecture designed, plan approved, ready for implementation!

**Next Command**: Start Phase 1 (API Research)
