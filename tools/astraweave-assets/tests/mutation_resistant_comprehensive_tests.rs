//! Mutation-resistant comprehensive tests for astraweave-assets.
//!
//! Targets: AssetType, LicenseInfo, ProviderConfig, ProviderRegistry,
//! FetchSummary, config structs — exact values, boundary conditions.

use astraweave_assets::{AssetType, FetchSummary, LicenseInfo, ProviderConfig, ProviderRegistry};

// =========================================================================
// AssetType — enum variants
// =========================================================================

#[test]
fn asset_type_texture_eq() {
    assert_eq!(AssetType::Texture, AssetType::Texture);
}

#[test]
fn asset_type_hdri_eq() {
    assert_eq!(AssetType::Hdri, AssetType::Hdri);
}

#[test]
fn asset_type_model_eq() {
    assert_eq!(AssetType::Model, AssetType::Model);
}

#[test]
fn asset_type_audio_eq() {
    assert_eq!(AssetType::Audio, AssetType::Audio);
}

#[test]
fn asset_type_sprite_eq() {
    assert_eq!(AssetType::Sprite, AssetType::Sprite);
}

#[test]
fn asset_type_tileset_eq() {
    assert_eq!(AssetType::Tileset, AssetType::Tileset);
}

#[test]
fn asset_type_ne() {
    assert_ne!(AssetType::Texture, AssetType::Model);
    assert_ne!(AssetType::Hdri, AssetType::Audio);
}

#[test]
fn asset_type_copy() {
    let a = AssetType::Texture;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn asset_type_clone() {
    let a = AssetType::Model;
    assert_eq!(a.clone(), AssetType::Model);
}

#[test]
fn asset_type_debug() {
    let s = format!("{:?}", AssetType::Texture);
    assert!(s.contains("Texture"), "debug={}", s);
}

#[test]
fn asset_type_serde_roundtrip() {
    let a = AssetType::Hdri;
    let json = serde_json::to_string(&a).unwrap();
    let b: AssetType = serde_json::from_str(&json).unwrap();
    assert_eq!(a, b);
}

#[test]
fn asset_type_serde_lowercase() {
    // serde(rename_all = "lowercase")
    let json = serde_json::to_string(&AssetType::Texture).unwrap();
    assert_eq!(json, "\"texture\"");
    let json = serde_json::to_string(&AssetType::Hdri).unwrap();
    assert_eq!(json, "\"hdri\"");
    let json = serde_json::to_string(&AssetType::Model).unwrap();
    assert_eq!(json, "\"model\"");
}

// =========================================================================
// LicenseInfo — CC0, CC-BY, CC-BY-SA
// =========================================================================

#[test]
fn license_cc0_fields() {
    let lic = LicenseInfo::cc0(Some("Author".into()), None);
    assert_eq!(lic.spdx_id, "CC0-1.0");
    assert!(!lic.requires_attribution);
    assert!(!lic.requires_sharealike);
    assert_eq!(lic.author, Some("Author".into()));
    assert_eq!(lic.source_url, None);
}

#[test]
fn license_cc0_no_attribution_text() {
    let lic = LicenseInfo::cc0(None, None);
    // CC0 does not require attribution
    assert!(lic.attribution_text("test").is_none());
}

#[test]
fn license_cc_by_fields() {
    let lic = LicenseInfo::cc_by("4.0", "John".into(), Some("https://example.com".into()));
    assert_eq!(lic.spdx_id, "CC-BY-4.0");
    assert!(lic.requires_attribution);
    assert!(!lic.requires_sharealike);
    assert_eq!(lic.author, Some("John".into()));
}

#[test]
fn license_cc_by_attribution_text() {
    let lic = LicenseInfo::cc_by("4.0", "Jane".into(), Some("https://src.com".into()));
    let text = lic.attribution_text("my_asset");
    assert!(text.is_some());
    let text = text.unwrap();
    assert!(text.contains("my_asset"), "text={}", text);
    assert!(text.contains("Jane"), "text={}", text);
    assert!(text.contains("https://src.com"), "text={}", text);
}

#[test]
fn license_cc_by_sa_fields() {
    let lic = LicenseInfo::cc_by_sa("4.0", "Bob".into(), None);
    assert_eq!(lic.spdx_id, "CC-BY-SA-4.0");
    assert!(lic.requires_attribution);
    assert!(lic.requires_sharealike);
}

#[test]
fn license_cc_by_version_3() {
    let lic = LicenseInfo::cc_by("3.0", "A".into(), None);
    assert_eq!(lic.spdx_id, "CC-BY-3.0");
}

#[test]
fn license_cc_by_sa_version_3() {
    let lic = LicenseInfo::cc_by_sa("3.0", "A".into(), None);
    assert_eq!(lic.spdx_id, "CC-BY-SA-3.0");
}

#[test]
fn license_from_spdx_cc0() {
    let lic = LicenseInfo::from_spdx("CC0-1.0", None, None).unwrap();
    assert_eq!(lic.spdx_id, "CC0-1.0");
    assert!(!lic.requires_attribution);
}

#[test]
fn license_from_spdx_cc_by_4() {
    let lic = LicenseInfo::from_spdx("CC-BY-4.0", Some("Auth".into()), None).unwrap();
    assert_eq!(lic.spdx_id, "CC-BY-4.0");
    assert!(lic.requires_attribution);
}

#[test]
fn license_from_spdx_cc_by_3() {
    let lic = LicenseInfo::from_spdx("CC-BY-3.0", Some("Auth".into()), None).unwrap();
    assert_eq!(lic.spdx_id, "CC-BY-3.0");
}

#[test]
fn license_from_spdx_cc_by_sa_4() {
    let lic = LicenseInfo::from_spdx("CC-BY-SA-4.0", Some("Auth".into()), None).unwrap();
    assert_eq!(lic.spdx_id, "CC-BY-SA-4.0");
    assert!(lic.requires_sharealike);
}

#[test]
fn license_from_spdx_cc_by_sa_3() {
    let lic = LicenseInfo::from_spdx("CC-BY-SA-3.0", Some("Auth".into()), None).unwrap();
    assert_eq!(lic.spdx_id, "CC-BY-SA-3.0");
}

#[test]
fn license_from_spdx_unknown_fails() {
    let result = LicenseInfo::from_spdx("MIT", None, None);
    assert!(result.is_err());
}

#[test]
fn license_from_spdx_cc_by_no_author_fails() {
    let result = LicenseInfo::from_spdx("CC-BY-4.0", None, None);
    assert!(result.is_err());
}

#[test]
fn license_validate_permissive_cc0_ok() {
    let lic = LicenseInfo::cc0(None, None);
    assert!(lic.validate_permissive().is_ok());
}

#[test]
fn license_validate_permissive_cc_by_ok() {
    let lic = LicenseInfo::cc_by("4.0", "A".into(), None);
    assert!(lic.validate_permissive().is_ok());
}

#[test]
fn license_validate_permissive_gpl_fails() {
    let lic = LicenseInfo {
        spdx_id: "GPL-3.0".into(),
        name: "GPL".into(),
        requires_attribution: true,
        requires_sharealike: true,
        author: None,
        source_url: None,
        license_url: String::new(),
    };
    assert!(lic.validate_permissive().is_err());
}

#[test]
fn license_validate_permissive_nc_fails() {
    let lic = LicenseInfo {
        spdx_id: "CC-BY-NC-4.0".into(),
        name: "CC BY-NC".into(),
        requires_attribution: true,
        requires_sharealike: false,
        author: None,
        source_url: None,
        license_url: String::new(),
    };
    assert!(lic.validate_permissive().is_err());
}

#[test]
fn license_validate_permissive_nd_fails() {
    let lic = LicenseInfo {
        spdx_id: "CC-BY-ND-4.0".into(),
        name: "CC BY-ND".into(),
        requires_attribution: true,
        requires_sharealike: false,
        author: None,
        source_url: None,
        license_url: String::new(),
    };
    assert!(lic.validate_permissive().is_err());
}

#[test]
fn license_attribution_no_author_returns_none() {
    let lic = LicenseInfo {
        spdx_id: "CC-BY-4.0".into(),
        name: "CC BY 4.0".into(),
        requires_attribution: true,
        requires_sharealike: false,
        author: None,
        source_url: None,
        license_url: String::new(),
    };
    // requires_attribution is true but author is None → should return None
    assert!(lic.attribution_text("x").is_none());
}

#[test]
fn license_cc0_license_url() {
    let lic = LicenseInfo::cc0(None, None);
    assert!(
        lic.license_url.contains("creativecommons.org"),
        "url={}",
        lic.license_url
    );
    assert!(lic.license_url.contains("zero"), "url={}", lic.license_url);
}

#[test]
fn license_clone() {
    let lic = LicenseInfo::cc_by("4.0", "Test".into(), None);
    let lic2 = lic.clone();
    assert_eq!(lic.spdx_id, lic2.spdx_id);
    assert_eq!(lic.requires_attribution, lic2.requires_attribution);
}

// =========================================================================
// ProviderConfig — configuration struct
// =========================================================================

#[test]
fn provider_config_serde_roundtrip() {
    let pc = ProviderConfig {
        provider: "polyhaven".into(),
        asset_type: AssetType::Texture,
        handle: "rock_01".into(),
        id: Some("aerial_rocks_02".into()),
        resolution: Some("2k".into()),
        format: Some("png".into()),
        url: None,
        license: None,
        author: None,
        source_url: None,
    };
    let json = serde_json::to_string(&pc).unwrap();
    let pc2: ProviderConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(pc2.provider, "polyhaven");
    assert_eq!(pc2.asset_type, AssetType::Texture);
    assert_eq!(pc2.handle, "rock_01");
    assert_eq!(pc2.id, Some("aerial_rocks_02".into()));
}

#[test]
fn provider_config_optional_fields_skip_none() {
    let pc = ProviderConfig {
        provider: "test".into(),
        asset_type: AssetType::Model,
        handle: "h".into(),
        id: None,
        resolution: None,
        format: None,
        url: None,
        license: None,
        author: None,
        source_url: None,
    };
    let json = serde_json::to_string(&pc).unwrap();
    // skip_serializing_if = "Option::is_none" means None fields are omitted
    assert!(!json.contains("\"id\""), "json={}", json);
    assert!(!json.contains("\"resolution\""), "json={}", json);
}

// =========================================================================
// ProviderRegistry
// =========================================================================

#[test]
fn provider_registry_new_empty() {
    let reg = ProviderRegistry::new();
    assert!(reg.list_providers().is_empty());
}

#[test]
fn provider_registry_default_empty() {
    let reg = ProviderRegistry::default();
    assert!(reg.list_providers().is_empty());
}

#[test]
fn provider_registry_get_nonexistent_fails() {
    let reg = ProviderRegistry::new();
    assert!(reg.get("nonexistent").is_err());
}

// =========================================================================
// FetchSummary — download tracking
// =========================================================================

#[test]
fn fetch_summary_new_defaults() {
    let s = FetchSummary::new();
    assert_eq!(s.total_assets, 0);
    assert_eq!(s.downloaded, 0);
    assert_eq!(s.cached, 0);
    assert_eq!(s.failed, 0);
    assert!(s.assets.is_empty());
}

#[test]
fn fetch_summary_default_same_as_new() {
    let s = FetchSummary::default();
    assert_eq!(s.total_assets, 0);
}

#[test]
fn fetch_summary_add_cached() {
    let mut s = FetchSummary::new();
    s.add_cached("h1".into(), "id1".into(), "texture".into(), "2k".into());
    assert_eq!(s.total_assets, 1);
    assert_eq!(s.cached, 1);
    assert_eq!(s.downloaded, 0);
    assert_eq!(s.failed, 0);
    assert_eq!(s.assets.len(), 1);
    assert_eq!(s.assets[0].status, "cached");
    assert_eq!(s.assets[0].handle, "h1");
}

#[test]
fn fetch_summary_add_failed() {
    let mut s = FetchSummary::new();
    s.add_failed("h2".into(), "id2".into(), "model".into(), "timeout".into());
    assert_eq!(s.total_assets, 1);
    assert_eq!(s.failed, 1);
    assert_eq!(s.downloaded, 0);
    assert_eq!(s.assets[0].status, "failed");
    assert_eq!(s.assets[0].error, Some("timeout".into()));
}

#[test]
fn fetch_summary_multiple_operations() {
    let mut s = FetchSummary::new();
    s.add_cached("a".into(), "1".into(), "t".into(), "1k".into());
    s.add_cached("b".into(), "2".into(), "t".into(), "2k".into());
    s.add_failed("c".into(), "3".into(), "m".into(), "err".into());
    assert_eq!(s.total_assets, 3);
    assert_eq!(s.cached, 2);
    assert_eq!(s.failed, 1);
    assert_eq!(s.downloaded, 0);
    assert_eq!(s.assets.len(), 3);
}

#[test]
fn fetch_summary_to_json() {
    let mut s = FetchSummary::new();
    s.add_cached("h".into(), "i".into(), "t".into(), "r".into());
    let json = s.to_json().unwrap();
    assert!(
        json.contains("\"total_assets\""),
        "json should have total_assets"
    );
    assert!(json.contains("\"cached\""), "json should have cached");
}

#[test]
fn fetch_summary_serde_roundtrip() {
    let mut s = FetchSummary::new();
    s.add_cached("handle".into(), "id".into(), "texture".into(), "2k".into());
    let json = serde_json::to_string(&s).unwrap();
    let s2: FetchSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(s2.total_assets, 1);
    assert_eq!(s2.cached, 1);
    assert_eq!(s2.assets[0].handle, "handle");
}
