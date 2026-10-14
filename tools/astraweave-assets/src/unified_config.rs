// =============================================================================
// Unified Asset Manifest Parser
// =============================================================================
//
// This module parses the new unified asset_manifest.toml format that supports
// multiple providers (PolyHaven, Poly Pizza, OpenGameArt).
//
// Format example:
// ```toml
// [[assets]]
// handle = "aerial_rocks"
// provider = "polyhaven"
// type = "texture"
// id = "aerial_rocks_02"
// resolution = "2k"
// format = "png"
//
// [[assets]]
// handle = "character_knight"
// provider = "polypizza"
// type = "model"
// format = "glb"
// url = "https://poly.pizza/files/Low_poly_Knight.glb"
// license = "CC0-1.0"
// author = "Quaternius"
// source_url = "https://poly.pizza/m/Low_poly_Knight"
// ```
//
// =============================================================================

use crate::provider::{AssetType, ProviderConfig};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Unified asset manifest (new format)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnifiedManifest {
    /// Where to write downloaded assets (default: "assets/_downloaded")
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,

    /// Where to cache temp files and lockfile (default: ".asset_cache")
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,

    /// All assets (multi-provider)
    #[serde(default)]
    pub assets: Vec<UnifiedAssetEntry>,
}

/// Single asset entry in unified manifest
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnifiedAssetEntry {
    /// User-defined handle (e.g., "character_knight")
    pub handle: String,

    /// Provider name ("polyhaven", "polypizza", "opengameart")
    pub provider: String,

    /// Asset type (texture, hdri, model, audio, sprite, tileset)
    #[serde(rename = "type")]
    pub asset_type: AssetType,

    // =========================================================================
    // PolyHaven-specific fields
    // =========================================================================
    /// PolyHaven asset ID (e.g., "aerial_rocks_02")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Texture resolution ("1k", "2k", "4k", "8k")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,

    /// File format ("png", "jpg", "exr", "hdr", "glb")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    // =========================================================================
    // Direct URL fields (Poly Pizza, OpenGameArt)
    // =========================================================================
    /// Direct download URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// License SPDX identifier (e.g., "CC0-1.0", "CC-BY-4.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Author/creator name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Source URL (original page where asset was found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,

    /// SHA-256 of the checked-in file, when the bytes live in the repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

impl UnifiedManifest {
    /// Load manifest from TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest: {}", path.display()))?;

        let manifest: UnifiedManifest = toml::from_str(&content)
            .with_context(|| format!("Failed to parse manifest: {}", path.display()))?;

        Ok(manifest)
    }

    /// Get all assets for a specific provider
    pub fn assets_for_provider(&self, provider: &str) -> Vec<&UnifiedAssetEntry> {
        self.assets
            .iter()
            .filter(|a| a.provider == provider)
            .collect()
    }

    /// Get list of all unique providers in manifest
    pub fn providers(&self) -> Vec<String> {
        let mut providers: Vec<String> = self.assets.iter().map(|a| a.provider.clone()).collect();
        providers.sort();
        providers.dedup();
        providers
    }

    /// Count assets by provider
    pub fn asset_count_by_provider(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for asset in &self.assets {
            *counts.entry(asset.provider.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Convert UnifiedAssetEntry to ProviderConfig
    pub fn to_provider_config(entry: &UnifiedAssetEntry) -> ProviderConfig {
        ProviderConfig {
            provider: entry.provider.clone(),
            asset_type: entry.asset_type,
            handle: entry.handle.clone(),
            id: entry.id.clone(),
            resolution: entry.resolution.clone(),
            format: entry.format.clone(),
            url: entry.url.clone(),
            license: entry.license.clone(),
            author: entry.author.clone(),
            source_url: entry.source_url.clone(),
        }
    }
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("assets/_downloaded")
}

fn default_cache_dir() -> PathBuf {
    PathBuf::from(".asset_cache")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unified_manifest() {
        let toml_content = r#"
output_dir = "assets/_downloaded"
cache_dir = ".asset_cache"

[[assets]]
handle = "aerial_rocks"
provider = "polyhaven"
type = "texture"
id = "aerial_rocks_02"
resolution = "2k"
format = "png"

[[assets]]
handle = "character_knight"
provider = "polypizza"
type = "model"
format = "glb"
url = "https://poly.pizza/files/Low_poly_Knight.glb"
license = "CC0-1.0"
author = "Quaternius"
source_url = "https://poly.pizza/m/Low_poly_Knight"
"#;

        let manifest: UnifiedManifest = toml::from_str(toml_content).unwrap();

        assert_eq!(manifest.assets.len(), 2);
        assert_eq!(manifest.output_dir, PathBuf::from("assets/_downloaded"));
        assert_eq!(manifest.cache_dir, PathBuf::from(".asset_cache"));

        // Check first asset (PolyHaven texture)
        let texture = &manifest.assets[0];
        assert_eq!(texture.handle, "aerial_rocks");
        assert_eq!(texture.provider, "polyhaven");
        assert_eq!(texture.asset_type, AssetType::Texture);
        assert_eq!(texture.id, Some("aerial_rocks_02".to_string()));
        assert_eq!(texture.resolution, Some("2k".to_string()));
        assert_eq!(texture.format, Some("png".to_string()));

        // Check second asset (Poly Pizza model)
        let model = &manifest.assets[1];
        assert_eq!(model.handle, "character_knight");
        assert_eq!(model.provider, "polypizza");
        assert_eq!(model.asset_type, AssetType::Model);
        assert_eq!(model.format, Some("glb".to_string()));
        assert_eq!(
            model.url,
            Some("https://poly.pizza/files/Low_poly_Knight.glb".to_string())
        );
        assert_eq!(model.license, Some("CC0-1.0".to_string()));
        assert_eq!(model.author, Some("Quaternius".to_string()));
    }

    #[test]
    fn test_assets_for_provider() {
        let toml_content = r#"
[[assets]]
handle = "texture1"
provider = "polyhaven"
type = "texture"
id = "test1"

[[assets]]
handle = "model1"
provider = "polypizza"
type = "model"
format = "glb"
url = "https://poly.pizza/files/test.glb"
license = "CC0-1.0"

[[assets]]
handle = "texture2"
provider = "polyhaven"
type = "texture"
id = "test2"
"#;

        let manifest: UnifiedManifest = toml::from_str(toml_content).unwrap();

        let polyhaven_assets = manifest.assets_for_provider("polyhaven");
        assert_eq!(polyhaven_assets.len(), 2);
        assert_eq!(polyhaven_assets[0].handle, "texture1");
        assert_eq!(polyhaven_assets[1].handle, "texture2");

        let polypizza_assets = manifest.assets_for_provider("polypizza");
        assert_eq!(polypizza_assets.len(), 1);
        assert_eq!(polypizza_assets[0].handle, "model1");
    }

    #[test]
    fn test_providers_list() {
        let toml_content = r#"
[[assets]]
handle = "a1"
provider = "polyhaven"
type = "texture"
id = "test1"

[[assets]]
handle = "a2"
provider = "polypizza"
type = "model"
format = "glb"
url = "https://poly.pizza/files/test.glb"
license = "CC0-1.0"

[[assets]]
handle = "a3"
provider = "opengameart"
type = "audio"
format = "ogg"
url = "https://opengameart.org/files/test.ogg"
license = "CC0-1.0"

[[assets]]
handle = "a4"
provider = "polyhaven"
type = "hdri"
id = "test2"
"#;

        let manifest: UnifiedManifest = toml::from_str(toml_content).unwrap();

        let providers = manifest.providers();
        assert_eq!(providers.len(), 3);
        assert!(providers.contains(&"polyhaven".to_string()));
        assert!(providers.contains(&"polypizza".to_string()));
        assert!(providers.contains(&"opengameart".to_string()));
    }

    #[test]
    fn test_asset_count_by_provider() {
        let toml_content = r#"
[[assets]]
handle = "a1"
provider = "polyhaven"
type = "texture"
id = "test1"

[[assets]]
handle = "a2"
provider = "polyhaven"
type = "hdri"
id = "test2"

[[assets]]
handle = "a3"
provider = "polypizza"
type = "model"
format = "glb"
url = "https://poly.pizza/files/test.glb"
license = "CC0-1.0"
"#;

        let manifest: UnifiedManifest = toml::from_str(toml_content).unwrap();

        let counts = manifest.asset_count_by_provider();
        assert_eq!(counts.get("polyhaven"), Some(&2));
        assert_eq!(counts.get("polypizza"), Some(&1));
    }
}
