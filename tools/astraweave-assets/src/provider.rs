// =============================================================================
// Asset Provider Trait - Multi-Source Support
// =============================================================================
//
// This module defines the trait-based architecture for fetching assets from
// multiple providers (PolyHaven, Poly Pizza, OpenGameArt, etc.)
//
// Key design principles:
// - **License Compliance First**: All providers must track licenses
// - **Provider Isolation**: Assets organized by provider directory
// - **Attribution Automation**: Generate ATTRIBUTION.txt per provider
// - **Free Licenses Only**: CC0, CC-BY, CC-BY-SA supported (NO GPL)
//
// =============================================================================

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// =============================================================================
// Core Types
// =============================================================================

/// Resolved asset with all metadata and download URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAsset {
    /// Unique handle (user-defined in manifest)
    pub handle: String,

    /// Provider name ("polyhaven", "polypizza", "opengameart")
    pub provider: String,

    /// Asset type (texture, hdri, model, audio, sprite, tileset)
    pub asset_type: AssetType,

    /// Download URLs (map name → URL)
    /// Examples:
    /// - Texture: {"albedo": "https://...", "normal": "https://..."}
    /// - Model: {"model": "https://.../model.glb"}
    /// - Audio: {"audio": "https://.../sound.ogg"}
    pub urls: HashMap<String, String>,

    /// License information (CRITICAL for compliance)
    pub license: LicenseInfo,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Asset type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum AssetType {
    /// PBR texture maps (albedo, normal, roughness, metallic, ao, height)
    Texture,

    /// High Dynamic Range Image (environment lighting)
    Hdri,

    /// 3D model (GLB, GLTF, FBX, OBJ)
    Model,

    /// Audio file (OGG, WAV, MP3)
    Audio,

    /// 2D sprite sheet (PNG, WebP)
    Sprite,

    /// Tileset for 2D games (PNG, WebP)
    Tileset,
}

/// License information with SPDX identifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    /// SPDX license identifier (e.g., "CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0")
    pub spdx_id: String,

    /// Human-readable license name (e.g., "Creative Commons Zero")
    pub name: String,

    /// Does this license require attribution?
    pub requires_attribution: bool,

    /// Does this license require share-alike?
    pub requires_sharealike: bool,

    /// Author/creator name (required if requires_attribution = true)
    pub author: Option<String>,

    /// Source URL (where asset was obtained)
    pub source_url: Option<String>,

    /// License URL (full license text)
    pub license_url: String,
}

impl LicenseInfo {
    /// Create CC0 license (Public Domain, no attribution required)
    pub fn cc0(author: Option<String>, source_url: Option<String>) -> Self {
        Self {
            spdx_id: "CC0-1.0".to_string(),
            name: "Creative Commons Zero v1.0 Universal".to_string(),
            requires_attribution: false,
            requires_sharealike: false,
            author,
            source_url,
            license_url: "https://creativecommons.org/publicdomain/zero/1.0/".to_string(),
        }
    }

    /// Create CC-BY license (requires attribution)
    pub fn cc_by(version: &str, author: String, source_url: Option<String>) -> Self {
        let spdx_id = format!("CC-BY-{}", version);
        let name = format!("Creative Commons Attribution {} International", version);
        let license_url = format!(
            "https://creativecommons.org/licenses/by/{}/",
            version.replace('.', "")
        );

        Self {
            spdx_id,
            name,
            requires_attribution: true,
            requires_sharealike: false,
            author: Some(author),
            source_url,
            license_url,
        }
    }

    /// Create CC-BY-SA license (requires attribution + share-alike)
    pub fn cc_by_sa(version: &str, author: String, source_url: Option<String>) -> Self {
        let spdx_id = format!("CC-BY-SA-{}", version);
        let name = format!(
            "Creative Commons Attribution-ShareAlike {} International",
            version
        );
        let license_url = format!(
            "https://creativecommons.org/licenses/by-sa/{}/",
            version.replace('.', "")
        );

        Self {
            spdx_id,
            name,
            requires_attribution: true,
            requires_sharealike: true,
            author: Some(author),
            source_url,
            license_url,
        }
    }

    /// Parse SPDX identifier from string (user input validation)
    pub fn from_spdx(
        spdx_id: &str,
        author: Option<String>,
        source_url: Option<String>,
    ) -> Result<Self> {
        match spdx_id {
            "CC0-1.0" => Ok(Self::cc0(author, source_url)),
            "CC-BY-3.0" => Ok(Self::cc_by(
                "3.0",
                author.context("CC-BY requires author")?,
                source_url,
            )),
            "CC-BY-4.0" => Ok(Self::cc_by(
                "4.0",
                author.context("CC-BY requires author")?,
                source_url,
            )),
            "CC-BY-SA-3.0" => Ok(Self::cc_by_sa(
                "3.0",
                author.context("CC-BY-SA requires author")?,
                source_url,
            )),
            "CC-BY-SA-4.0" => Ok(Self::cc_by_sa(
                "4.0",
                author.context("CC-BY-SA requires author")?,
                source_url,
            )),
            _ => anyhow::bail!(
                "Unsupported license '{}'. Only CC0, CC-BY, and CC-BY-SA are allowed.",
                spdx_id
            ),
        }
    }

    /// Validate license is permissive (no GPL, no restrictive licenses)
    pub fn validate_permissive(&self) -> Result<()> {
        if self.spdx_id.contains("GPL") {
            anyhow::bail!(
                "GPL licenses are not supported ({}). Use CC0, CC-BY, or CC-BY-SA instead.",
                self.spdx_id
            );
        }

        if self.spdx_id.contains("NC") {
            anyhow::bail!(
                "NonCommercial licenses are not supported ({}). Use CC0, CC-BY, or CC-BY-SA instead.",
                self.spdx_id
            );
        }

        if self.spdx_id.contains("ND") {
            anyhow::bail!(
                "NoDerivatives licenses are not supported ({}). Use CC0, CC-BY, or CC-BY-SA instead.",
                self.spdx_id
            );
        }

        Ok(())
    }

    /// Generate attribution text for this license
    pub fn attribution_text(&self, asset_handle: &str) -> Option<String> {
        if !self.requires_attribution {
            return None;
        }

        let author = self.author.as_ref()?;
        let source = self
            .source_url
            .as_ref()
            .map(|s| format!(" ({})", s))
            .unwrap_or_default();

        Some(format!(
            "\"{}\" by {}{}\nLicense: {} ({})",
            asset_handle, author, source, self.name, self.license_url
        ))
    }
}

// =============================================================================
// Asset Provider Trait
// =============================================================================

/// Trait for asset providers (PolyHaven, Poly Pizza, OpenGameArt, etc.)
#[async_trait]
pub trait AssetProvider: Send + Sync {
    /// Provider name (e.g., "polyhaven", "polypizza", "opengameart")
    fn name(&self) -> &str;

    /// Resolve asset metadata and download URLs
    ///
    /// # Arguments
    /// - `handle`: User-defined asset handle (e.g., "character_knight")
    /// - `config`: Provider-specific configuration
    ///
    /// # Returns
    /// - `ResolvedAsset` with all metadata and download URLs
    async fn resolve(&self, handle: &str, config: &ProviderConfig) -> Result<ResolvedAsset>;

    /// Validate asset configuration before resolution
    fn validate_config(&self, config: &ProviderConfig) -> Result<()>;

    /// Generate attribution file content for all assets
    fn generate_attribution(&self, assets: &[ResolvedAsset]) -> String {
        let mut lines = vec![
            format!("# Attribution - {}", self.name()),
            String::new(),
            "This directory contains assets from the following sources:".to_string(),
            String::new(),
        ];

        for asset in assets {
            if let Some(attribution) = asset.license.attribution_text(&asset.handle) {
                lines.push(attribution);
                lines.push(String::new());
            }
        }

        lines.push(String::new());
        lines.push("For full license texts, see URLs in each attribution above.".to_string());

        lines.join("\n")
    }
}

// =============================================================================
// Provider Configuration
// =============================================================================

/// Provider-specific configuration (parsed from manifest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name ("polyhaven", "polypizza", "opengameart")
    pub provider: String,

    /// Asset type (texture, hdri, model, audio, sprite, tileset)
    #[serde(rename = "type")]
    pub asset_type: AssetType,

    /// Asset handle (user-defined, e.g., "character_knight")
    pub handle: String,

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
}

// =============================================================================
// Provider Registry
// =============================================================================

/// Registry of all available providers
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn AssetProvider>>,
}

impl ProviderRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider
    pub fn register(&mut self, provider: Box<dyn AssetProvider>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }

    /// Get provider by name
    pub fn get(&self, name: &str) -> Result<&dyn AssetProvider> {
        self.providers
            .get(name)
            .map(|p| p.as_ref())
            .context(format!("Unknown provider: {}", name))
    }

    /// List all registered provider names
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Attribution File Generator
// =============================================================================

/// Generate ATTRIBUTION.txt for a provider directory
pub fn generate_attribution_file(
    provider_name: &str,
    assets: &[ResolvedAsset],
    output_path: &Path,
) -> Result<()> {
    use std::fs;

    let mut lines = vec![
        format!("# Attribution - {}", provider_name.to_uppercase()),
        "=".repeat(80),
        String::new(),
        format!(
            "This directory contains {} assets from {}:",
            assets.len(),
            provider_name
        ),
        String::new(),
    ];

    // Group by license type
    let mut by_license: HashMap<String, Vec<&ResolvedAsset>> = HashMap::new();
    for asset in assets {
        by_license
            .entry(asset.license.spdx_id.clone())
            .or_default()
            .push(asset);
    }

    // Write summary
    lines.push("## License Summary".to_string());
    lines.push(String::new());
    for (spdx_id, assets_list) in &by_license {
        lines.push(format!("- {}: {} assets", spdx_id, assets_list.len()));
    }
    lines.push(String::new());
    lines.push("=".repeat(80));
    lines.push(String::new());

    // Write detailed attributions
    lines.push("## Detailed Attributions".to_string());
    lines.push(String::new());

    for asset in assets {
        lines.push(format!("### {}", asset.handle));
        lines.push(String::new());

        if let Some(attribution) = asset.license.attribution_text(&asset.handle) {
            lines.push(attribution);
        } else {
            lines.push(format!("License: {} (Public Domain)", asset.license.name));
            if let Some(source) = &asset.license.source_url {
                lines.push(format!("Source: {}", source));
            }
        }

        lines.push(String::new());
        lines.push("-".repeat(80));
        lines.push(String::new());
    }

    // Write footer
    lines.push(String::new());
    lines.push("For full license texts, see URLs above.".to_string());
    lines.push(format!("Generated: {}", chrono::Utc::now().to_rfc3339()));

    let content = lines.join("\n");
    fs::write(output_path, content).with_context(|| {
        format!(
            "Failed to write attribution file: {}",
            output_path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_asset_type_variants() {
        let texture = AssetType::Texture;
        let hdri = AssetType::Hdri;
        let model = AssetType::Model;
        let audio = AssetType::Audio;
        let sprite = AssetType::Sprite;
        let tileset = AssetType::Tileset;

        assert!(matches!(texture, AssetType::Texture));
        assert!(matches!(hdri, AssetType::Hdri));
        assert!(matches!(model, AssetType::Model));
        assert!(matches!(audio, AssetType::Audio));
        assert!(matches!(sprite, AssetType::Sprite));
        assert!(matches!(tileset, AssetType::Tileset));
    }

    #[test]
    fn test_asset_type_serialization() {
        let texture = AssetType::Texture;
        let json = serde_json::to_string(&texture).unwrap();
        assert!(json.contains("texture"));
    }

    #[test]
    fn test_license_info_cc0() {
        let license = LicenseInfo::cc0(None, Some("https://source.com".to_string()));
        assert_eq!(license.spdx_id, "CC0-1.0");
        assert!(!license.requires_attribution);
    }

    #[test]
    fn test_license_info_cc_by() {
        let license = LicenseInfo::cc_by(
            "4.0",
            "Artist Name".to_string(),
            Some("https://source.com".to_string()),
        );
        assert_eq!(license.spdx_id, "CC-BY-4.0");
        assert!(license.requires_attribution);
        assert_eq!(license.author, Some("Artist Name".to_string()));
    }

    #[test]
    fn test_license_info_cc_by_sa() {
        let license = LicenseInfo::cc_by_sa(
            "4.0",
            "Artist Name".to_string(),
            Some("https://source.com".to_string()),
        );
        assert_eq!(license.spdx_id, "CC-BY-SA-4.0");
        assert!(license.requires_attribution);
        assert!(license.requires_sharealike);
    }

    #[test]
    fn test_license_attribution_text_cc0() {
        let license = LicenseInfo::cc0(None, Some("https://source.com".to_string()));
        // CC0 doesn't require attribution
        assert!(license.attribution_text("test_asset").is_none());
    }

    #[test]
    fn test_license_attribution_text_cc_by() {
        let license = LicenseInfo::cc_by(
            "4.0",
            "Artist Name".to_string(),
            Some("https://source.com".to_string()),
        );
        let text = license.attribution_text("test_asset").unwrap();
        assert!(text.contains("test_asset"));
        assert!(text.contains("Artist Name"));
        // Check for the license name rather than spdx_id
        assert!(text.contains("Creative Commons Attribution"));
    }

    #[test]
    fn test_resolved_asset_creation() {
        let mut urls = HashMap::new();
        urls.insert(
            "albedo".to_string(),
            "https://example.com/albedo.png".to_string(),
        );

        let asset = ResolvedAsset {
            handle: "brick_wall".to_string(),
            provider: "polyhaven".to_string(),
            asset_type: AssetType::Texture,
            urls,
            license: LicenseInfo::cc0(None, Some("https://polyhaven.com".to_string())),
            metadata: HashMap::new(),
        };

        assert_eq!(asset.handle, "brick_wall");
        assert_eq!(asset.provider, "polyhaven");
        assert!(matches!(asset.asset_type, AssetType::Texture));
    }

    #[test]
    fn test_provider_config_creation() {
        let config = ProviderConfig {
            provider: "polyhaven".to_string(),
            asset_type: AssetType::Texture,
            handle: "test_texture".to_string(),
            id: Some("brick_wall_001".to_string()),
            resolution: Some("2k".to_string()),
            format: None,
            url: None,
            license: None,
            author: None,
            source_url: None,
        };

        assert_eq!(config.provider, "polyhaven");
        assert_eq!(config.id, Some("brick_wall_001".to_string()));
        assert_eq!(config.resolution, Some("2k".to_string()));
    }

    #[test]
    fn test_provider_registry_new() {
        let registry = ProviderRegistry::new();
        assert!(registry.providers.is_empty());
    }

    #[test]
    fn test_provider_registry_list_providers() {
        let registry = ProviderRegistry::new();
        let names = registry.list_providers();
        assert!(names.is_empty());
    }

    #[test]
    fn test_license_info_clone() {
        let license = LicenseInfo::cc_by(
            "4.0",
            "Author".to_string(),
            Some("https://source.com".to_string()),
        );
        let cloned = license.clone();
        assert_eq!(license.spdx_id, cloned.spdx_id);
        assert_eq!(license.author, cloned.author);
    }

    #[test]
    fn test_resolved_asset_clone() {
        let asset = ResolvedAsset {
            handle: "test".to_string(),
            provider: "test_provider".to_string(),
            asset_type: AssetType::Hdri,
            urls: HashMap::new(),
            license: LicenseInfo::cc0(None, Some("https://source.com".to_string())),
            metadata: HashMap::new(),
        };
        let cloned = asset.clone();
        assert_eq!(asset.handle, cloned.handle);
    }

    #[test]
    fn test_provider_config_serialization() {
        let config = ProviderConfig {
            provider: "polyhaven".to_string(),
            asset_type: AssetType::Texture,
            handle: "test".to_string(),
            id: Some("test_id".to_string()),
            resolution: Some("1k".to_string()),
            format: None,
            url: None,
            license: None,
            author: None,
            source_url: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("polyhaven"));
        assert!(json.contains("test_id"));
    }

    #[test]
    fn test_asset_type_debug() {
        let texture = AssetType::Texture;
        let debug_str = format!("{:?}", texture);
        assert!(debug_str.contains("Texture"));
    }

    #[test]
    fn test_license_info_debug() {
        let license = LicenseInfo::cc0(None, None);
        let debug_str = format!("{:?}", license);
        assert!(debug_str.contains("CC0-1.0"));
    }

    #[test]
    fn test_license_info_cc0_with_author() {
        let license = LicenseInfo::cc0(
            Some("Author".to_string()),
            Some("https://source.com".to_string()),
        );
        assert_eq!(license.spdx_id, "CC0-1.0");
        assert_eq!(license.author, Some("Author".to_string()));
    }

    #[test]
    fn test_resolved_asset_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("resolution".to_string(), "2k".to_string());
        metadata.insert("format".to_string(), "png".to_string());

        let asset = ResolvedAsset {
            handle: "test".to_string(),
            provider: "polyhaven".to_string(),
            asset_type: AssetType::Texture,
            urls: HashMap::new(),
            license: LicenseInfo::cc0(None, None),
            metadata,
        };

        assert_eq!(asset.metadata.len(), 2);
        assert_eq!(asset.metadata.get("resolution"), Some(&"2k".to_string()));
    }

    #[test]
    fn test_provider_config_with_format() {
        let config = ProviderConfig {
            provider: "polypizza".to_string(),
            asset_type: AssetType::Model,
            handle: "character".to_string(),
            id: Some("knight_001".to_string()),
            resolution: None,
            format: Some("glb".to_string()),
            url: None,
            license: None,
            author: None,
            source_url: None,
        };

        assert_eq!(config.format, Some("glb".to_string()));
        assert!(config.resolution.is_none());
    }

    #[test]
    fn test_resolved_asset_multiple_urls() {
        let mut urls = HashMap::new();
        urls.insert(
            "albedo".to_string(),
            "https://example.com/albedo.png".to_string(),
        );
        urls.insert(
            "normal".to_string(),
            "https://example.com/normal.png".to_string(),
        );
        urls.insert(
            "roughness".to_string(),
            "https://example.com/roughness.png".to_string(),
        );

        let asset = ResolvedAsset {
            handle: "pbr_material".to_string(),
            provider: "polyhaven".to_string(),
            asset_type: AssetType::Texture,
            urls,
            license: LicenseInfo::cc0(None, None),
            metadata: HashMap::new(),
        };

        assert_eq!(asset.urls.len(), 3);
        assert!(asset.urls.contains_key("albedo"));
        assert!(asset.urls.contains_key("normal"));
        assert!(asset.urls.contains_key("roughness"));
    }

    #[test]
    fn test_license_from_spdx_supported_variants() {
        let cc0 = LicenseInfo::from_spdx("CC0-1.0", None, None).unwrap();
        assert_eq!(cc0.spdx_id, "CC0-1.0");
        assert!(!cc0.requires_attribution);

        let by = LicenseInfo::from_spdx(
            "CC-BY-4.0",
            Some("Alice".to_string()),
            Some("https://example.com".to_string()),
        )
        .unwrap();
        assert_eq!(by.spdx_id, "CC-BY-4.0");
        assert!(by.requires_attribution);
        assert_eq!(by.author.as_deref(), Some("Alice"));

        let by_sa = LicenseInfo::from_spdx("CC-BY-SA-3.0", Some("Bob".to_string()), None).unwrap();
        assert_eq!(by_sa.spdx_id, "CC-BY-SA-3.0");
        assert!(by_sa.requires_sharealike);
        assert_eq!(by_sa.author.as_deref(), Some("Bob"));
    }

    #[test]
    fn test_license_from_spdx_requires_author_for_attribution_licenses() {
        let err = LicenseInfo::from_spdx("CC-BY-4.0", None, None).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("requires author"));

        let err = LicenseInfo::from_spdx("CC-BY-SA-4.0", None, None).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("requires author"));
    }

    #[test]
    fn test_license_from_spdx_rejects_unsupported() {
        let err = LicenseInfo::from_spdx("MIT", None, None).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("Unsupported license"));
    }

    #[test]
    fn test_validate_permissive_rejects_gpl_nc_nd() {
        let mut lic = LicenseInfo::cc0(None, None);

        lic.spdx_id = "GPL-3.0".to_string();
        assert!(lic.validate_permissive().is_err());

        lic.spdx_id = "CC-BY-NC-4.0".to_string();
        assert!(lic.validate_permissive().is_err());

        lic.spdx_id = "CC-BY-ND-4.0".to_string();
        assert!(lic.validate_permissive().is_err());
    }

    #[test]
    fn test_attribution_text_requires_author_field() {
        let lic = LicenseInfo {
            spdx_id: "CC-BY-4.0".to_string(),
            name: "Creative Commons Attribution 4.0 International".to_string(),
            requires_attribution: true,
            requires_sharealike: false,
            author: None,
            source_url: Some("https://source.example".to_string()),
            license_url: "https://creativecommons.org/licenses/by/40/".to_string(),
        };
        assert!(lic.attribution_text("asset").is_none());
    }

    struct DummyProvider;

    #[async_trait]
    impl AssetProvider for DummyProvider {
        fn name(&self) -> &str {
            "dummy"
        }

        async fn resolve(&self, _handle: &str, _config: &ProviderConfig) -> Result<ResolvedAsset> {
            anyhow::bail!("not implemented")
        }

        fn validate_config(&self, _config: &ProviderConfig) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_asset_provider_generate_attribution_includes_only_required_entries() {
        let provider = DummyProvider;

        let mut urls = HashMap::new();
        urls.insert("model".to_string(), "https://example.com/m.glb".to_string());

        let cc0_asset = ResolvedAsset {
            handle: "cc0_asset".to_string(),
            provider: "dummy".to_string(),
            asset_type: AssetType::Model,
            urls: urls.clone(),
            license: LicenseInfo::cc0(None, Some("https://src".to_string())),
            metadata: HashMap::new(),
        };

        let by_asset = ResolvedAsset {
            handle: "by_asset".to_string(),
            provider: "dummy".to_string(),
            asset_type: AssetType::Model,
            urls,
            license: LicenseInfo::cc_by(
                "4.0",
                "Author".to_string(),
                Some("https://src".to_string()),
            ),
            metadata: HashMap::new(),
        };

        let content = provider.generate_attribution(&[cc0_asset, by_asset]);
        assert!(content.contains("# Attribution - dummy"));
        assert!(content.contains("\"by_asset\""));
        assert!(!content.contains("cc0_asset\""));
        assert!(content.contains("For full license texts"));
    }

    #[test]
    fn test_provider_registry_register_and_get() {
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(DummyProvider));

        let providers = registry.list_providers();
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&"dummy".to_string()));

        let got = registry.get("dummy").unwrap();
        assert_eq!(got.name(), "dummy");

        assert!(registry.get("missing").is_err());
    }

    #[test]
    fn test_generate_attribution_file_writes_content() {
        let temp = TempDir::new().unwrap();
        let out_path = temp.path().join("ATTRIBUTION.txt");

        let asset_a = ResolvedAsset {
            handle: "a".to_string(),
            provider: "dummy".to_string(),
            asset_type: AssetType::Texture,
            urls: HashMap::new(),
            license: LicenseInfo::cc0(None, Some("https://src".to_string())),
            metadata: HashMap::new(),
        };

        let asset_b = ResolvedAsset {
            handle: "b".to_string(),
            provider: "dummy".to_string(),
            asset_type: AssetType::Texture,
            urls: HashMap::new(),
            license: LicenseInfo::cc_by(
                "4.0",
                "Author".to_string(),
                Some("https://src".to_string()),
            ),
            metadata: HashMap::new(),
        };

        generate_attribution_file("dummy", &[asset_a, asset_b], &out_path).unwrap();

        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("# Attribution - DUMMY"));
        assert!(content.contains("## License Summary"));
        assert!(content.contains("CC0-1.0"));
        assert!(content.contains("CC-BY-4.0"));
        assert!(content.contains("## Detailed Attributions"));
        assert!(content.contains("### a"));
        assert!(content.contains("### b"));
        assert!(content.contains("Generated:"));
    }
}
