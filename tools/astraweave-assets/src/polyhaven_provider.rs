// =============================================================================
// PolyHaven Provider - AssetProvider Implementation
// =============================================================================
//
// Wraps existing PolyHavenClient to implement the AssetProvider trait for
// use in the unified multi-provider system.
//
// =============================================================================

use crate::polyhaven::PolyHavenClient;
use crate::provider::{
    AssetProvider, AssetType, LicenseInfo, ProviderConfig, ResolvedAsset as ResolvedAssetV2,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;

/// PolyHaven provider (API-based, CC0 only)
pub struct PolyHavenProvider {
    client: PolyHavenClient,
}

impl PolyHavenProvider {
    /// Create new PolyHaven provider
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: PolyHavenClient::new()?,
        })
    }
}

#[async_trait]
impl AssetProvider for PolyHavenProvider {
    fn name(&self) -> &str {
        "polyhaven"
    }

    async fn resolve(&self, handle: &str, config: &ProviderConfig) -> Result<ResolvedAssetV2> {
        // Validate configuration
        self.validate_config(config)?;

        let id = config
            .id
            .as_ref()
            .context("Missing 'id' field for PolyHaven asset")?;

        // Resolve based on asset type
        let old_resolved = match config.asset_type {
            AssetType::Texture => {
                let res = config
                    .resolution
                    .as_ref()
                    .context("Missing 'resolution' field for PolyHaven texture")?;

                // Default maps if not specified.
                // "arm" added 2026-05-14 (Terrain Asset Quality Sub-phase A.0.A):
                // ARM-packed map (R=AO, G=Roughness, B=Metallic) matches
                // AstraWeave's ORM _mra.png runtime convention per audit §3.1.
                // Fetcher logs non-fatal warning if a particular asset does
                // not expose ARM in its PolyHaven Files response.
                let maps = vec![
                    "albedo".to_string(),
                    "normal".to_string(),
                    "roughness".to_string(),
                    "metallic".to_string(),
                    "ao".to_string(),
                    "arm".to_string(),
                ];

                self.client.resolve_texture(id, res, &maps).await?
            }
            AssetType::Hdri => {
                let res = config
                    .resolution
                    .as_ref()
                    .context("Missing 'resolution' field for PolyHaven HDRI")?;

                self.client.resolve_hdri(id, res).await?
            }
            AssetType::Model => {
                let res = config
                    .resolution
                    .as_ref()
                    .context("Missing 'resolution' field for PolyHaven model")?;
                let format = config
                    .format
                    .as_ref()
                    .context("Missing 'format' field for PolyHaven model")?;

                self.client.resolve_model(id, res, format).await?
            }
            _ => anyhow::bail!(
                "PolyHaven does not support asset type: {:?}",
                config.asset_type
            ),
        };

        // Convert old ResolvedAsset to new ResolvedAssetV2
        let license = LicenseInfo::cc0(None, Some(format!("https://polyhaven.com/a/{}", id)));

        let mut metadata = HashMap::new();
        metadata.insert("id".to_string(), id.clone());
        if let Some(res) = &config.resolution {
            metadata.insert("resolution".to_string(), res.clone());
        }
        if let Some(format) = &config.format {
            metadata.insert("format".to_string(), format.clone());
        }

        Ok(ResolvedAssetV2 {
            handle: handle.to_string(),
            provider: "polyhaven".to_string(),
            asset_type: config.asset_type,
            urls: old_resolved.urls,
            license,
            metadata,
        })
    }

    fn validate_config(&self, config: &ProviderConfig) -> Result<()> {
        // Check required fields
        if config.id.is_none() {
            anyhow::bail!(
                "Missing required field 'id' for PolyHaven asset '{}'",
                config.handle
            );
        }

        // Check resolution for textures/HDRIs/models
        match config.asset_type {
            AssetType::Texture | AssetType::Hdri | AssetType::Model => {
                if config.resolution.is_none() {
                    anyhow::bail!(
                        "Missing required field 'resolution' for PolyHaven {:?} '{}'",
                        config.asset_type,
                        config.handle
                    );
                }
            }
            _ => {}
        }

        // Check format for models
        if config.asset_type == AssetType::Model && config.format.is_none() {
            anyhow::bail!(
                "Missing required field 'format' for PolyHaven model '{}'",
                config.handle
            );
        }

        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_texture_config() {
        let provider = PolyHavenProvider::new().unwrap();

        // Valid config
        let valid_config = ProviderConfig {
            provider: "polyhaven".to_string(),
            asset_type: AssetType::Texture,
            handle: "test_texture".to_string(),
            id: Some("aerial_rocks_02".to_string()),
            resolution: Some("2k".to_string()),
            format: Some("png".to_string()),
            url: None,
            license: None,
            author: None,
            source_url: None,
        };

        assert!(provider.validate_config(&valid_config).is_ok());

        // Missing id
        let missing_id = ProviderConfig {
            id: None,
            ..valid_config.clone()
        };
        assert!(provider.validate_config(&missing_id).is_err());

        // Missing resolution
        let missing_res = ProviderConfig {
            resolution: None,
            ..valid_config.clone()
        };
        assert!(provider.validate_config(&missing_res).is_err());
    }

    #[test]
    fn test_validate_model_config() {
        let provider = PolyHavenProvider::new().unwrap();

        // Valid config
        let valid_config = ProviderConfig {
            provider: "polyhaven".to_string(),
            asset_type: AssetType::Model,
            handle: "test_model".to_string(),
            id: Some("rock_collection_a".to_string()),
            resolution: Some("2k".to_string()),
            format: Some("glb".to_string()),
            url: None,
            license: None,
            author: None,
            source_url: None,
        };

        assert!(provider.validate_config(&valid_config).is_ok());

        // Missing format
        let missing_format = ProviderConfig {
            format: None,
            ..valid_config.clone()
        };
        assert!(provider.validate_config(&missing_format).is_err());
    }
}
