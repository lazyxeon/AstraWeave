#![forbid(unsafe_code)]
// Re-export main types for library usage
pub use config::{AssetManifest, HdriAsset, LockEntry, Lockfile, ModelAsset, TextureAsset};
pub use direct_url_provider::DirectUrlProvider;
pub use downloader::{DownloadResult, DownloadTask, Downloader};
pub use kenney_provider::KenneyProvider;
pub use organize::AssetOrganizer;
pub use polyhaven::{PolyHavenClient, ResolvedAsset};
pub use provider::{
    AssetProvider, AssetType, LicenseInfo, ProviderConfig, ProviderRegistry,
    ResolvedAsset as ResolvedAssetV2,
};
pub use summary::FetchSummary;

use std::path::Path;

pub mod config;
pub mod direct_url_provider;
pub mod downloader;
pub mod kenney_provider;
pub mod organize;
pub mod polyhaven;
pub mod polyhaven_provider;
pub mod provider;
pub mod summary;
pub mod unified_config;

/// Runtime asset fetcher for on-demand loading
pub mod ensure_asset {
    use super::*;
    use anyhow::{Context, Result};
    use std::path::PathBuf;

    /// Ensure asset exists locally, fetching if necessary
    pub async fn ensure_asset(manifest_path: &Path, handle: &str) -> Result<Vec<PathBuf>> {
        let base_url = std::env::var("POLYHAVEN_BASE_URL").ok();
        ensure_asset_with_base_url(manifest_path, handle, base_url.as_deref()).await
    }

    /// Ensure asset exists locally, fetching if necessary (optionally injecting PolyHaven base URL).
    ///
    /// This exists primarily for deterministic tests and offline usage.
    pub async fn ensure_asset_with_base_url(
        manifest_path: &Path,
        handle: &str,
        polyhaven_base_url: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        // Load manifest
        let manifest = AssetManifest::load(manifest_path).context("Failed to load manifest")?;

        let organizer =
            AssetOrganizer::new(manifest.output_dir.clone(), manifest.cache_dir.clone());

        // Check if cached
        if organizer.is_cached(handle).await {
            // Return cached paths
            let lockfile = organizer.load_lockfile().await?;
            if let Some(entry) = lockfile.assets.get(handle) {
                return Ok(entry.paths.values().cloned().collect());
            }
        }

        // Fetch asset
        let client = if let Some(base_url) = polyhaven_base_url {
            PolyHavenClient::new_with_base_url(base_url)?
        } else {
            PolyHavenClient::new()?
        };
        let downloader = Downloader::new()?;

        // Try texture
        if let Some(texture) = manifest.textures.get(handle) {
            let resolved = client
                .resolve_texture(&texture.id, &texture.res, &texture.maps)
                .await?;

            let mut downloads = std::collections::HashMap::new();

            for (map_name, url) in &resolved.urls {
                let temp_path = manifest
                    .cache_dir
                    .join(format!("_temp_{}_{}.tmp", handle, map_name));
                let result = downloader.download(url, &temp_path, false).await?;
                downloads.insert(map_name.clone(), result);
            }

            let entry = organizer.organize(handle, &resolved, &downloads).await?;
            return Ok(entry.paths.values().cloned().collect());
        }

        // Try HDRI
        if let Some(hdri) = manifest.hdris.get(handle) {
            let resolved = client.resolve_hdri(&hdri.id, &hdri.res).await?;

            let mut downloads = std::collections::HashMap::new();

            if let Some(url) = resolved.urls.get("hdri") {
                let temp_path = manifest
                    .cache_dir
                    .join(format!("_temp_{}_hdri.tmp", handle));
                let result = downloader.download(url, &temp_path, false).await?;
                downloads.insert("hdri".to_string(), result);
            }

            let entry = organizer.organize(handle, &resolved, &downloads).await?;
            return Ok(entry.paths.values().cloned().collect());
        }

        // Try model
        if let Some(model) = manifest.models.get(handle) {
            let resolved = client
                .resolve_model(&model.id, &model.res, &model.format)
                .await?;

            let mut downloads = std::collections::HashMap::new();

            if let Some(url) = resolved.urls.get("model") {
                let temp_path = manifest
                    .cache_dir
                    .join(format!("_temp_{}_model.tmp", handle));
                let result = downloader.download(url, &temp_path, false).await?;
                downloads.insert("model".to_string(), result);
            }

            let entry = organizer.organize(handle, &resolved, &downloads).await?;
            return Ok(entry.paths.values().cloned().collect());
        }

        Err(anyhow::anyhow!("Asset '{}' not found in manifest", handle))
    }

    /// Check if asset is available locally (without fetching)
    pub async fn is_available(manifest_path: &Path, handle: &str) -> Result<bool> {
        let manifest = AssetManifest::load(manifest_path)?;
        let organizer = AssetOrganizer::new(manifest.output_dir, manifest.cache_dir);

        Ok(organizer.is_cached(handle).await)
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_asset;
    use serde_json::json;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_library_exports() {
        // Ensure all types are re-exported
        let _manifest: super::AssetManifest;
        let _client: super::PolyHavenClient;
        let _downloader: super::Downloader;
    }

    #[tokio::test]
    async fn test_ensure_asset_texture_downloads_and_caches() {
        let server = MockServer::start().await;
        let base = server.uri();

        // PolyHaven API mocks
        Mock::given(method("GET"))
            .and(path("/files/tex01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Diffuse": {
                    "2k": { "png": { "url": format!("{base}/dl/tex_diff_2k.png"), "size": 5, "md5": "" } }
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/info/tex01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "Test Texture",
                "categories": ["test"],
                "tags": ["albedo"],
                "download_count": 1
            })))
            .mount(&server)
            .await;

        // Download URL mock
        Mock::given(method("GET"))
            .and(path("/dl/tex_diff_2k.png"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
            .mount(&server)
            .await;

        // Manifest file
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("out");
        let cache_dir = temp.path().join("cache");
        let manifest_path = temp.path().join("manifest.toml");

        let mut textures = HashMap::new();
        textures.insert(
            "tex".to_string(),
            super::TextureAsset {
                id: "tex01".to_string(),
                kind: "texture".to_string(),
                res: "2k".to_string(),
                maps: vec!["albedo".to_string()],
                tags: vec![],
            },
        );

        let manifest = super::AssetManifest {
            output_dir: output_dir.clone(),
            cache_dir: cache_dir.clone(),
            textures,
            hdris: HashMap::new(),
            models: HashMap::new(),
        };
        manifest.save(&manifest_path).unwrap();

        // First call fetches and writes
        let paths =
            ensure_asset::ensure_asset_with_base_url(&manifest_path, "tex", Some(&server.uri()))
                .await
                .unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].exists());

        let reqs_after_first = server.received_requests().await.unwrap().len();
        assert!(reqs_after_first >= 3);

        // Second call should be cached (no more HTTP requests)
        let paths2 =
            ensure_asset::ensure_asset_with_base_url(&manifest_path, "tex", Some(&server.uri()))
                .await
                .unwrap();
        assert_eq!(paths2.len(), 1);
        assert!(paths2[0].exists());

        let reqs_after_second = server.received_requests().await.unwrap().len();
        assert_eq!(reqs_after_second, reqs_after_first);

        // is_available uses only lockfile + filesystem
        assert!(ensure_asset::is_available(&manifest_path, "tex")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_ensure_asset_hdri_downloads() {
        let server = MockServer::start().await;
        let base = server.uri();

        Mock::given(method("GET"))
            .and(path("/files/hdri01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "hdri": {
                    "1k": {
                        "exr": { "url": format!("{base}/dl/hdri_1k.exr"), "size": 3, "md5": "" }
                    }
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/info/hdri01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "Test HDRI",
                "categories": ["hdri"],
                "tags": [],
                "download_count": 1
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/dl/hdri_1k.exr"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hdr".to_vec()))
            .mount(&server)
            .await;

        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("out");
        let cache_dir = temp.path().join("cache");
        let manifest_path = temp.path().join("manifest.toml");

        let mut hdris = HashMap::new();
        hdris.insert(
            "sky".to_string(),
            super::HdriAsset {
                id: "hdri01".to_string(),
                kind: "hdri".to_string(),
                res: "2k".to_string(),
                tags: vec![],
            },
        );

        let manifest = super::AssetManifest {
            output_dir,
            cache_dir,
            textures: HashMap::new(),
            hdris,
            models: HashMap::new(),
        };
        manifest.save(&manifest_path).unwrap();

        let paths =
            ensure_asset::ensure_asset_with_base_url(&manifest_path, "sky", Some(&server.uri()))
                .await
                .unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].exists());
    }

    #[tokio::test]
    async fn test_ensure_asset_model_downloads() {
        let server = MockServer::start().await;
        let base = server.uri();

        Mock::given(method("GET"))
            .and(path("/files/model01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "gltf": {
                    "2k": {
                        "glb": { "url": format!("{base}/dl/model_2k.glb"), "size": 4, "md5": "" }
                    }
                }
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/info/model01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "Test Model",
                "categories": ["model"],
                "tags": [],
                "download_count": 1
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/dl/model_2k.glb"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"glb".to_vec()))
            .mount(&server)
            .await;

        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("out");
        let cache_dir = temp.path().join("cache");
        let manifest_path = temp.path().join("manifest.toml");

        let mut models = HashMap::new();
        models.insert(
            "m".to_string(),
            super::ModelAsset {
                id: "model01".to_string(),
                kind: "model".to_string(),
                res: "2k".to_string(),
                format: "glb".to_string(),
                tags: vec![],
            },
        );

        let manifest = super::AssetManifest {
            output_dir,
            cache_dir,
            textures: HashMap::new(),
            hdris: HashMap::new(),
            models,
        };
        manifest.save(&manifest_path).unwrap();

        let paths =
            ensure_asset::ensure_asset_with_base_url(&manifest_path, "m", Some(&server.uri()))
                .await
                .unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].exists());
    }

    #[tokio::test]
    async fn test_ensure_asset_not_found_in_manifest_errors() {
        let temp = TempDir::new().unwrap();
        let manifest_path = temp.path().join("manifest.toml");
        let manifest = super::AssetManifest {
            output_dir: temp.path().join("out"),
            cache_dir: temp.path().join("cache"),
            textures: HashMap::new(),
            hdris: HashMap::new(),
            models: HashMap::new(),
        };
        manifest.save(&manifest_path).unwrap();

        let err = ensure_asset::ensure_asset_with_base_url(&manifest_path, "nope", None)
            .await
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("not found"), "unexpected error: {msg}");
    }

    #[test]
    fn test_texture_asset_type() {
        let _asset: super::TextureAsset;
    }

    #[test]
    fn test_hdri_asset_type() {
        let _asset: super::HdriAsset;
    }

    #[test]
    fn test_model_asset_type() {
        let _asset: super::ModelAsset;
    }

    #[test]
    fn test_lock_entry_type() {
        let _entry: super::LockEntry;
    }

    #[test]
    fn test_lockfile_type() {
        let _file: super::Lockfile;
    }

    #[test]
    fn test_download_result_type() {
        let _result: super::DownloadResult;
    }

    #[test]
    fn test_download_task_type() {
        let _task: super::DownloadTask;
    }

    #[test]
    fn test_fetch_summary_type() {
        let _summary: super::FetchSummary;
    }

    #[test]
    fn test_direct_url_provider_type() {
        let _provider: super::DirectUrlProvider;
    }

    #[test]
    fn test_kenney_provider_type() {
        let _provider: super::KenneyProvider;
    }

    #[test]
    fn test_asset_organizer_type() {
        let _organizer: super::AssetOrganizer;
    }

    #[test]
    fn test_resolved_asset_type() {
        let _asset: super::ResolvedAsset;
    }

    #[test]
    fn test_asset_provider_trait() {
        // Just verify trait is accessible
        fn _accepts_provider<T: super::AssetProvider>(_: T) {}
    }

    #[test]
    fn test_asset_type_enum() {
        let texture = super::AssetType::Texture;
        let hdri = super::AssetType::Hdri;
        let model = super::AssetType::Model;

        assert!(matches!(texture, super::AssetType::Texture));
        assert!(matches!(hdri, super::AssetType::Hdri));
        assert!(matches!(model, super::AssetType::Model));
    }

    #[test]
    fn test_license_info_type() {
        let _license: super::LicenseInfo;
    }

    #[test]
    fn test_provider_config_type() {
        let _config: super::ProviderConfig;
    }

    #[test]
    fn test_provider_registry_type() {
        let _registry: super::ProviderRegistry;
    }
}
