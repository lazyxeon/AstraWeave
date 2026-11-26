// =============================================================================
// Kenney.nl Provider - CC0 Game Assets
// =============================================================================
//
// Kenney.nl provides 50,000+ free game assets under CC0 license (Public Domain).
// All assets are available at: https://kenney.nl/assets/<asset-name>
//
// Download URL Pattern: https://kenney.nl/content/<category>/<asset-name>.zip
// Example: https://kenney.nl/content/2-2d-assets/platformer-pack-redux.zip
//
// Since there's no public API, we use manual URL configuration (like Poly Pizza).
// =============================================================================

use crate::provider::{AssetProvider, AssetType, LicenseInfo, ProviderConfig, ResolvedAsset};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;

/// Provider for Kenney.nl game assets
///
/// All Kenney.nl assets are CC0 (Public Domain), no attribution required.
/// User must manually specify download URLs in manifest.
///
/// # Example Manifest Entry
///
/// ```toml
/// [[assets]]
/// handle = "platformer_pack"
/// provider = "kenney"
/// type = "sprite"
/// format = "zip"
/// url = "https://kenney.nl/content/2-2d-assets/platformer-pack-redux.zip"
/// license = "CC0-1.0"
/// source_url = "https://kenney.nl/assets/platformer-pack-redux"
/// ```
pub struct KenneyProvider {
    name: String,
    #[allow(dead_code)]
    license: LicenseInfo,
}

impl KenneyProvider {
    /// Create a new Kenney.nl provider
    ///
    /// All assets are CC0, so license is hardcoded.
    pub fn new() -> Self {
        Self {
            name: "Kenney.nl".into(),
            license: LicenseInfo::cc0(
                Some("Kenney Vleugels".into()),
                Some("https://kenney.nl".into()),
            ),
        }
    }

    /// Validate that URL is from kenney.nl domain
    fn validate_url(url: &str) -> Result<()> {
        if !url.starts_with("https://kenney.nl/") {
            anyhow::bail!(
                "Invalid Kenney.nl URL: {}. Must start with 'https://kenney.nl/'",
                url
            );
        }
        Ok(())
    }

    /// Validate license is CC0 (Kenney only provides CC0 assets)
    fn validate_license(license: &LicenseInfo) -> Result<()> {
        if license.spdx_id != "CC0-1.0" {
            anyhow::bail!(
                "Kenney.nl only provides CC0 assets. Got: {}",
                license.spdx_id
            );
        }
        Ok(())
    }

    /// Infer asset type from URL or format
    #[allow(dead_code)]
    fn infer_asset_type(url: &str, format: &str) -> AssetType {
        // Check URL path
        if url.contains("/2-2d-assets/") || url.contains("/ui/") {
            return AssetType::Sprite;
        }
        if url.contains("/3-3d-assets/") {
            return AssetType::Model;
        }
        if url.contains("/audio/") {
            return AssetType::Audio;
        }

        // Fallback to format
        match format.to_lowercase().as_str() {
            "png" | "svg" => AssetType::Sprite,
            "glb" | "gltf" | "obj" | "fbx" => AssetType::Model,
            "ogg" | "mp3" | "wav" => AssetType::Audio,
            _ => AssetType::Sprite, // Default
        }
    }
}

impl Default for KenneyProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AssetProvider for KenneyProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn resolve(&self, handle: &str, config: &ProviderConfig) -> Result<ResolvedAsset> {
        // Validate config
        self.validate_config(config)?;

        // Extract required fields
        let url = config
            .url
            .as_ref()
            .context("Kenney provider requires 'url' field")?;
        let format = config
            .format
            .as_ref()
            .context("Kenney provider requires 'format' field")?;
        let source_url = config
            .source_url
            .as_ref()
            .context("Kenney provider requires 'source_url' field")?;

        // Validate URL domain
        Self::validate_url(url)?;
        Self::validate_url(source_url)?;

        // Parse license (should be CC0)
        let license = if let Some(license_str) = &config.license {
            LicenseInfo::from_spdx(
                license_str,
                Some("Kenney Vleugels".into()),
                Some(source_url.clone()),
            )?
        } else {
            LicenseInfo::cc0(Some("Kenney Vleugels".into()), Some(source_url.clone()))
        };

        // Validate license is CC0
        Self::validate_license(&license)?;

        // Use asset type from config (already validated)
        let asset_type = config.asset_type;

        // Build download URLs
        let mut urls = HashMap::new();
        let file_key = match asset_type {
            AssetType::Sprite | AssetType::Tileset => "sprites",
            AssetType::Model => "model",
            AssetType::Audio => "audio",
            AssetType::Texture => "texture",
            AssetType::Hdri => "hdri",
        };
        urls.insert(file_key.to_string(), url.clone());

        // Build metadata
        let mut metadata = HashMap::new();
        metadata.insert("format".into(), format.clone());
        metadata.insert("source_url".into(), source_url.clone());
        metadata.insert("author".into(), "Kenney Vleugels".into());

        Ok(ResolvedAsset {
            handle: handle.to_string(),
            provider: "kenney".to_string(),
            asset_type,
            urls,
            license,
            metadata,
        })
    }

    fn validate_config(&self, config: &ProviderConfig) -> Result<()> {
        // Required fields
        if config.url.is_none() {
            anyhow::bail!("Kenney provider requires 'url' field");
        }
        if config.format.is_none() {
            anyhow::bail!("Kenney provider requires 'format' field");
        }
        if config.source_url.is_none() {
            anyhow::bail!("Kenney provider requires 'source_url' field");
        }

        // Validate URL
        if let Some(url) = &config.url {
            Self::validate_url(url)?;
        }

        // Validate license if provided
        if let Some(license_str) = &config.license {
            let license = LicenseInfo::from_spdx(
                license_str,
                Some("Kenney Vleugels".into()),
                config.source_url.clone(),
            )?;
            Self::validate_license(&license)?;
        }

        Ok(())
    }

    fn generate_attribution(&self, assets: &[ResolvedAsset]) -> String {
        let mut output = String::new();

        output.push_str("# Attribution - KENNEY.NL\n");
        output.push_str(
            "================================================================================\n\n",
        );

        output.push_str(&format!(
            "This directory contains {} assets from kenney:\n\n",
            assets.len()
        ));

        // License summary (always CC0 for Kenney)
        output.push_str("## License Summary\n\n");
        output.push_str("- CC0-1.0: ");
        output.push_str(&assets.len().to_string());
        output.push_str(" assets\n\n");

        output.push_str(
            "================================================================================\n\n",
        );
        output.push_str("## Detailed Attributions\n\n");

        // Per-asset details
        for asset in assets {
            output.push_str(&format!("### {}\n\n", asset.handle));

            output.push_str("License: ");
            output.push_str(&asset.license.name);
            output.push_str(" (Public Domain)\n");

            if let Some(source) = asset.metadata.get("source_url") {
                output.push_str("Source: ");
                output.push_str(source);
                output.push('\n');
            }

            if let Some(author) = asset.metadata.get("author") {
                output.push_str("Author: ");
                output.push_str(author);
                output.push_str(" (https://kenney.nl)\n");
            }

            output.push('\n');
            output.push_str("--------------------------------------------------------------------------------\n\n");
        }

        output.push_str(
            "\nAll Kenney.nl assets are CC0 (Public Domain) - no attribution required.\n",
        );
        output.push_str(
            "However, attribution is appreciated: 'Assets by Kenney.nl (www.kenney.nl)'\n",
        );
        output.push_str(&format!(
            "Generated: {}\n",
            crate::utc_stamp::format_utc_rfc3339(time::OffsetDateTime::now_utc())
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kenney_provider_creation() {
        let provider = KenneyProvider::new();
        assert_eq!(provider.name(), "Kenney.nl");
    }

    #[tokio::test]
    async fn test_resolve_sprite_pack() {
        let provider = KenneyProvider::new();

        let config = ProviderConfig {
            provider: "kenney".into(),
            asset_type: AssetType::Sprite,
            handle: "platformer_pack".into(),
            id: None,
            resolution: None,
            format: Some("zip".into()),
            url: Some("https://kenney.nl/content/2-2d-assets/platformer-pack-redux.zip".into()),
            license: Some("CC0-1.0".into()),
            author: None,
            source_url: Some("https://kenney.nl/assets/platformer-pack-redux".into()),
        };

        let resolved = provider.resolve("platformer_pack", &config).await;
        assert!(resolved.is_ok());

        let asset = resolved.unwrap();
        assert_eq!(asset.handle, "platformer_pack");
        assert_eq!(asset.provider, "kenney");
        assert_eq!(asset.license.spdx_id, "CC0-1.0");
        assert_eq!(asset.license.author, Some("Kenney Vleugels".into()));
        assert!(asset.urls.contains_key("sprites"));
    }

    #[tokio::test]
    async fn test_resolve_3d_model_pack() {
        let provider = KenneyProvider::new();

        let config = ProviderConfig {
            provider: "kenney".into(),
            asset_type: AssetType::Model,
            handle: "fantasy_town".into(),
            id: None,
            resolution: None,
            format: Some("zip".into()),
            url: Some("https://kenney.nl/content/3-3d-assets/fantasy-town-kit.zip".into()),
            license: Some("CC0-1.0".into()),
            author: None,
            source_url: Some("https://kenney.nl/assets/fantasy-town-kit".into()),
        };

        let resolved = provider.resolve("fantasy_town", &config).await;
        assert!(resolved.is_ok());

        let asset = resolved.unwrap();
        assert_eq!(asset.asset_type, AssetType::Model);
        assert!(asset.urls.contains_key("model"));
    }

    #[tokio::test]
    async fn test_invalid_url_domain() {
        let provider = KenneyProvider::new();

        let config = ProviderConfig {
            provider: "kenney".into(),
            asset_type: AssetType::Sprite,
            handle: "test".into(),
            id: None,
            resolution: None,
            format: Some("zip".into()),
            url: Some("https://example.com/asset.zip".into()),
            license: None,
            author: None,
            source_url: Some("https://kenney.nl/assets/test".into()),
        };

        let result = provider.resolve("test", &config).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid Kenney.nl URL"));
    }

    #[tokio::test]
    async fn test_non_cc0_license_rejected() {
        let provider = KenneyProvider::new();

        let config = ProviderConfig {
            provider: "kenney".into(),
            asset_type: AssetType::Sprite,
            handle: "test".into(),
            id: None,
            resolution: None,
            format: Some("zip".into()),
            url: Some("https://kenney.nl/content/2-2d-assets/test.zip".into()),
            license: Some("CC-BY-4.0".into()),
            author: Some("Test Author".into()),
            source_url: Some("https://kenney.nl/assets/test".into()),
        };

        let result = provider.resolve("test", &config).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Kenney.nl only provides CC0"));
    }

    #[tokio::test]
    async fn test_missing_required_fields() {
        let provider = KenneyProvider::new();

        // Missing URL
        let config = ProviderConfig {
            provider: "kenney".into(),
            asset_type: AssetType::Sprite,
            handle: "test".into(),
            id: None,
            resolution: None,
            format: None,
            url: None,
            license: None,
            author: None,
            source_url: None,
        };
        let result = provider.validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires 'url'"));
    }

    #[tokio::test]
    async fn test_missing_format_field() {
        let provider = KenneyProvider::new();

        // Missing format
        let config = ProviderConfig {
            provider: "kenney".into(),
            asset_type: AssetType::Sprite,
            handle: "test".into(),
            id: None,
            resolution: None,
            format: None,
            url: Some("https://kenney.nl/content/2-2d-assets/test.zip".into()),
            license: None,
            author: None,
            source_url: Some("https://kenney.nl/assets/test".into()),
        };
        let result = provider.validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires 'format'"));
    }

    #[tokio::test]
    async fn test_missing_source_url_field() {
        let provider = KenneyProvider::new();

        // Missing source_url
        let config = ProviderConfig {
            provider: "kenney".into(),
            asset_type: AssetType::Sprite,
            handle: "test".into(),
            id: None,
            resolution: None,
            format: Some("zip".into()),
            url: Some("https://kenney.nl/content/2-2d-assets/test.zip".into()),
            license: None,
            author: None,
            source_url: None,
        };
        let result = provider.validate_config(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires 'source_url'"));
    }

    #[test]
    fn test_validate_license_non_cc0() {
        // Test that non-CC0 licenses are rejected
        let license = LicenseInfo {
            spdx_id: "CC-BY-4.0".into(),
            name: "Creative Commons Attribution 4.0".into(),
            requires_attribution: true,
            requires_sharealike: false,
            author: Some("Test Author".into()),
            source_url: Some("https://example.com".into()),
            license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
        };
        let result = KenneyProvider::validate_license(&license);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Kenney.nl only provides CC0"));
    }

    #[test]
    fn test_infer_asset_type_default_fallback() {
        // Test default fallback for unknown format
        assert_eq!(
            KenneyProvider::infer_asset_type("https://kenney.nl/content/test.zip", "xyz"),
            AssetType::Sprite
        );
        assert_eq!(
            KenneyProvider::infer_asset_type("https://kenney.nl/content/test.zip", "unknown"),
            AssetType::Sprite
        );
    }

    #[test]
    fn test_infer_asset_type() {
        // From URL path
        assert_eq!(
            KenneyProvider::infer_asset_type(
                "https://kenney.nl/content/2-2d-assets/test.zip",
                "zip"
            ),
            AssetType::Sprite
        );
        assert_eq!(
            KenneyProvider::infer_asset_type(
                "https://kenney.nl/content/3-3d-assets/test.zip",
                "zip"
            ),
            AssetType::Model
        );
        assert_eq!(
            KenneyProvider::infer_asset_type("https://kenney.nl/content/audio/test.zip", "zip"),
            AssetType::Audio
        );

        // From format
        assert_eq!(
            KenneyProvider::infer_asset_type("https://kenney.nl/content/test.zip", "png"),
            AssetType::Sprite
        );
        assert_eq!(
            KenneyProvider::infer_asset_type("https://kenney.nl/content/test.zip", "glb"),
            AssetType::Model
        );
    }

    #[test]
    fn test_generate_attribution() {
        let provider = KenneyProvider::new();

        let mut metadata = HashMap::new();
        metadata.insert(
            "source_url".into(),
            "https://kenney.nl/assets/platformer-pack-redux".into(),
        );
        metadata.insert("author".into(), "Kenney Vleugels".into());

        let assets = vec![ResolvedAsset {
            handle: "platformer_pack".into(),
            provider: "kenney".into(),
            asset_type: AssetType::Sprite,
            urls: HashMap::new(),
            license: LicenseInfo::cc0(
                Some("Kenney Vleugels".into()),
                Some("https://kenney.nl/assets/platformer-pack-redux".into()),
            ),
            metadata,
        }];

        let attribution = provider.generate_attribution(&assets);

        assert!(attribution.contains("# Attribution - KENNEY.NL"));
        assert!(attribution.contains("1 assets from kenney"));
        assert!(attribution.contains("CC0-1.0: 1 assets"));
        assert!(attribution.contains("platformer_pack"));
        assert!(attribution.contains("Kenney Vleugels"));
        assert!(attribution.contains("no attribution required"));
    }
}
