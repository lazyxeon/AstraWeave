use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PolyHaven API client
pub struct PolyHavenClient {
    client: reqwest::Client,
    base_url: String,
}

/// Response from /files/{asset_id} endpoint
/// Structure varies by asset type:
/// Textures: { "Diffuse": { "2k": { "png": { "url": "...", "size": 123 } } } }
/// HDRIs: { "hdri": { "2k": { "exr": { "url": "...", "size": 123 } } }, "tonemapped": { "url": "..." } }
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilesResponse {
    #[serde(flatten)]
    pub maps: HashMap<String, serde_json::Value>,
}

/// File information (URL, size, MD5)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileInfo {
    pub url: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub md5: String,
}

/// Response from /info/{asset_id} endpoint
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InfoResponse {
    pub name: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub download_count: u64,
}

/// Resolved asset with download URLs
#[derive(Debug, Clone)]
pub struct ResolvedAsset {
    pub id: String,
    pub kind: String,
    pub resolution: String,
    pub urls: HashMap<String, String>,
    pub info: InfoResponse,
}

impl PolyHavenClient {
    /// Create new client
    pub fn new() -> Result<Self> {
        Self::new_with_base_url("https://api.polyhaven.com")
    }

    /// Create new client with custom base URL (for testing)
    pub fn new_with_base_url(base_url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("AstraWeave-Assets/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.to_string(),
        })
    }

    /// Fetch files metadata for an asset
    pub async fn get_files(&self, asset_id: &str) -> Result<FilesResponse> {
        let url = format!("{}/files/{}", self.base_url, asset_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context(format!("Failed to fetch files for asset: {}", asset_id))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "PolyHaven API error for {}: HTTP {}",
                asset_id,
                response.status()
            ));
        }

        let files: FilesResponse = response
            .json()
            .await
            .context("Failed to parse files response")?;

        Ok(files)
    }

    /// Fetch info metadata for an asset
    pub async fn get_info(&self, asset_id: &str) -> Result<InfoResponse> {
        let url = format!("{}/info/{}", self.base_url, asset_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context(format!("Failed to fetch info for asset: {}", asset_id))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "PolyHaven API error for {}: HTTP {}",
                asset_id,
                response.status()
            ));
        }

        let info: InfoResponse = response
            .json()
            .await
            .context("Failed to parse info response")?;

        Ok(info)
    }

    /// Resolve texture asset with fallback resolutions
    pub async fn resolve_texture(
        &self,
        asset_id: &str,
        requested_res: &str,
        requested_maps: &[String],
    ) -> Result<ResolvedAsset> {
        let files = self.get_files(asset_id).await?;
        let info = self.get_info(asset_id).await?;

        // Resolution fallback priority: requested → 2k → 1k → whatever's available
        let fallback_order = self.resolution_fallback_order(requested_res);

        // Extract URLs for requested maps
        let mut urls = HashMap::new();
        let mut selected_res = requested_res.to_string();

        for map_name in requested_maps {
            // Get PolyHaven map names (capitalized)
            let ph_map_names = self.polyhaven_map_names(map_name);
            let mut found = false;

            // Try each alternative name
            for ph_name in &ph_map_names {
                if let Some(map_value) = files.maps.get(*ph_name) {
                    // Parse nested JSON structure: { "2k": { "png": { "url": "..." } } }
                    if let Some(resolutions_map) = map_value.as_object() {
                        // Try each resolution in fallback order
                        for res in &fallback_order {
                            if let Some(formats_value) = resolutions_map.get(*res) {
                                if let Some(formats_map) = formats_value.as_object() {
                                    // Prefer PNG > EXR > JPG
                                    for format in &["png", "exr", "jpg"] {
                                        if let Some(file_info_value) = formats_map.get(*format) {
                                            if let Ok(file_info) = serde_json::from_value::<FileInfo>(
                                                file_info_value.clone(),
                                            ) {
                                                urls.insert(map_name.clone(), file_info.url);
                                                selected_res = res.to_string();
                                                found = true;
                                                break;
                                            }
                                        }
                                    }

                                    if found {
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if found {
                        break;
                    }
                }
            }

            if !found {
                eprintln!(
                    "⚠️  Map '{}' not available for {} (tried: {:?})",
                    map_name, asset_id, ph_map_names
                );
            }
        }

        if urls.is_empty() {
            return Err(anyhow!("No maps found for texture {}", asset_id));
        }

        Ok(ResolvedAsset {
            id: asset_id.to_string(),
            kind: "texture".to_string(),
            resolution: selected_res,
            urls,
            info,
        })
    }

    /// Resolve HDRI asset
    pub async fn resolve_hdri(&self, asset_id: &str, requested_res: &str) -> Result<ResolvedAsset> {
        let files = self.get_files(asset_id).await?;
        let info = self.get_info(asset_id).await?;

        let fallback_order = self.resolution_fallback_order(requested_res);
        let mut hdri_url = None;
        let mut selected_res = requested_res.to_string();

        // HDRIs have structure: { "hdri": { "2k": { "exr": {...}, "hdr": {...} } } }
        if let Some(hdri_value) = files.maps.get("hdri") {
            if let Some(resolutions_map) = hdri_value.as_object() {
                for res in &fallback_order {
                    if let Some(formats_value) = resolutions_map.get(*res) {
                        if let Some(formats_map) = formats_value.as_object() {
                            // Prefer EXR > HDR for HDRIs
                            for format in &["exr", "hdr"] {
                                if let Some(file_info_value) = formats_map.get(*format) {
                                    if let Ok(file_info) =
                                        serde_json::from_value::<FileInfo>(file_info_value.clone())
                                    {
                                        hdri_url = Some(file_info.url);
                                        selected_res = res.to_string();
                                        break;
                                    }
                                }
                            }

                            if hdri_url.is_some() {
                                break;
                            }
                        }
                    }
                }
            }
        }

        match hdri_url {
            Some(url) => {
                let mut urls = HashMap::new();
                urls.insert("hdri".to_string(), url);

                Ok(ResolvedAsset {
                    id: asset_id.to_string(),
                    kind: "hdri".to_string(),
                    resolution: selected_res,
                    urls,
                    info,
                })
            }
            None => Err(anyhow!("No HDRI file found for asset: {}", asset_id)),
        }
    }

    /// Resolve model asset
    pub async fn resolve_model(
        &self,
        asset_id: &str,
        requested_res: &str,
        format: &str,
    ) -> Result<ResolvedAsset> {
        let files = self.get_files(asset_id).await?;
        let info = self.get_info(asset_id).await?;

        let fallback_order = self.resolution_fallback_order(requested_res);
        let mut model_urls = HashMap::new();
        let mut selected_res = requested_res.to_string();

        // Look for model formats: gltf, glb, blend, fbx
        for (map_name, map_value) in &files.maps {
            if map_name.to_lowercase().contains(format)
                || map_name.to_lowercase().contains("gltf")
                || map_name.to_lowercase().contains("glb")
                || map_name.to_lowercase().contains("blend")
            {
                if let Some(resolutions_map) = map_value.as_object() {
                    for res in &fallback_order {
                        if let Some(formats_value) = resolutions_map.get(*res) {
                            if let Some(formats_map) = formats_value.as_object() {
                                // Prefer GLB > GLTF > BLEND
                                for model_format in &["glb", "gltf", "blend"] {
                                    if let Some(file_info_value) = formats_map.get(*model_format) {
                                        if let Ok(file_info) = serde_json::from_value::<FileInfo>(
                                            file_info_value.clone(),
                                        ) {
                                            model_urls.insert("model".to_string(), file_info.url);
                                            selected_res = res.to_string();
                                            break;
                                        }
                                    }
                                }

                                if !model_urls.is_empty() {
                                    break;
                                }
                            }
                        }
                    }
                }

                if !model_urls.is_empty() {
                    break;
                }
            }
        }

        if model_urls.is_empty() {
            return Err(anyhow!(
                "No model file found for asset {} (format: {})",
                asset_id,
                format
            ));
        }

        Ok(ResolvedAsset {
            id: asset_id.to_string(),
            kind: "model".to_string(),
            resolution: selected_res,
            urls: model_urls,
            info,
        })
    }

    /// Resolution fallback order
    fn resolution_fallback_order(&self, requested: &str) -> Vec<&str> {
        match requested {
            "8k" => vec!["8k", "4k", "2k", "1k"],
            "4k" => vec!["4k", "2k", "1k", "8k"],
            "2k" => vec!["2k", "1k", "4k", "8k"],
            "1k" => vec!["1k", "2k", "4k", "8k"],
            _ => vec!["2k", "1k", "4k", "8k"], // Default to 2k
        }
    }

    /// PolyHaven API map names (capitalized)
    fn polyhaven_map_names(&self, user_map_name: &str) -> Vec<&str> {
        match user_map_name {
            "albedo" => vec!["Diffuse", "diff", "diffuse", "Color"],
            "normal" => vec!["nor_gl", "nor_dx", "Normal"],
            "roughness" => vec!["Rough", "Roughness"],
            "metallic" => vec!["Metal", "Metallic", "Metalness"],
            "ao" => vec!["AO", "ao", "ambient_occlusion"],
            // ARM = AO+Roughness+Metallic single-image packing per PolyHaven CDN
            // (channel layout R=AO, G=Roughness, B=Metallic matches AstraWeave's
            // ORM convention; see pbr_terrain.wgsl:334-338 + audit §3.1).
            // Terrain Asset Quality Sub-phase A.0.A 2026-05-14.
            "arm" => vec!["arm", "ARM", "Arm"],
            "height" | "displacement" => vec!["Displacement", "disp", "Bump", "Height"],
            _ => vec![],
        }
    }
}

impl Default for PolyHavenClient {
    fn default() -> Self {
        Self::new().expect("Failed to create PolyHaven client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_client_creation() {
        let client = PolyHavenClient::new();
        assert!(client.is_ok(), "Should create client successfully");
    }

    #[test]
    fn test_resolution_fallback_order_known_resolutions() {
        let client = PolyHavenClient::new().unwrap();

        assert_eq!(
            client.resolution_fallback_order("8k"),
            vec!["8k", "4k", "2k", "1k"]
        );
        assert_eq!(
            client.resolution_fallback_order("4k"),
            vec!["4k", "2k", "1k", "8k"]
        );
        assert_eq!(
            client.resolution_fallback_order("2k"),
            vec!["2k", "1k", "4k", "8k"]
        );
        assert_eq!(
            client.resolution_fallback_order("1k"),
            vec!["1k", "2k", "4k", "8k"]
        );
    }

    #[test]
    fn test_resolution_fallback_order_unknown_defaults_to_2k() {
        let client = PolyHavenClient::new().unwrap();
        assert_eq!(
            client.resolution_fallback_order("banana"),
            vec!["2k", "1k", "4k", "8k"]
        );
    }

    #[test]
    fn test_polyhaven_map_names_mappings() {
        let client = PolyHavenClient::new().unwrap();
        let albedo = client.polyhaven_map_names("albedo");
        assert!(albedo.contains(&"Diffuse"));
        assert!(albedo.contains(&"Color"));

        let normal = client.polyhaven_map_names("normal");
        assert!(normal.contains(&"nor_gl"));
        assert!(normal.contains(&"Normal"));

        // Terrain Asset Quality Sub-phase A.0.A: ARM-packed map type for
        // AstraWeave's ORM _mra.png runtime convention (audit §3.1).
        let arm = client.polyhaven_map_names("arm");
        assert!(arm.contains(&"arm"));
        assert!(arm.contains(&"ARM"));

        let unknown = client.polyhaven_map_names("does_not_exist");
        assert!(unknown.is_empty());
    }

    #[tokio::test]
    async fn test_get_info_http_error_propagates() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/info/bad_asset"))
            .respond_with(ResponseTemplate::new(500).set_body_string("nope"))
            .mount(&server)
            .await;

        let client = PolyHavenClient::new_with_base_url(&server.uri()).unwrap();
        let err = client.get_info("bad_asset").await.unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("HTTP"), "unexpected error: {msg}");
    }

    #[tokio::test]
    async fn test_resolve_texture_selects_fallback_resolution_and_urls() {
        let server = MockServer::start().await;

        let base = server.uri();
        let files_body = json!({
            "Diffuse": {
                "2k": {
                    "png": { "url": format!("{base}/dl/tex_diff_2k.png"), "size": 5, "md5": "" }
                }
            },
            "nor_gl": {
                "2k": {
                    "png": { "url": format!("{base}/dl/tex_nor_2k.png"), "size": 5, "md5": "" }
                }
            }
        });

        let info_body = json!({
            "name": "Test Texture",
            "categories": ["test"],
            "tags": ["albedo", "normal"],
            "download_count": 42
        });

        Mock::given(method("GET"))
            .and(path("/files/tex01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(files_body))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/info/tex01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(info_body))
            .mount(&server)
            .await;

        let client = PolyHavenClient::new_with_base_url(&server.uri()).unwrap();
        let requested_maps = vec!["albedo".to_string(), "normal".to_string()];

        let resolved = client
            .resolve_texture("tex01", "8k", &requested_maps)
            .await
            .unwrap();

        assert_eq!(resolved.id, "tex01");
        assert_eq!(resolved.kind, "texture");
        assert_eq!(resolved.resolution, "2k");
        assert_eq!(resolved.info.name, "Test Texture");

        assert_eq!(
            resolved.urls.get("albedo").map(|s| s.as_str()),
            Some(format!("{base}/dl/tex_diff_2k.png").as_str())
        );
        assert_eq!(
            resolved.urls.get("normal").map(|s| s.as_str()),
            Some(format!("{base}/dl/tex_nor_2k.png").as_str())
        );
    }

    #[tokio::test]
    async fn test_resolve_hdri_falls_back_and_picks_preferred_format() {
        let server = MockServer::start().await;

        let base = server.uri();
        let files_body = json!({
            "hdri": {
                "1k": {
                    "hdr": { "url": format!("{base}/dl/hdri_1k.hdr"), "size": 3, "md5": "" },
                    "exr": { "url": format!("{base}/dl/hdri_1k.exr"), "size": 4, "md5": "" }
                }
            }
        });

        let info_body = json!({
            "name": "Test HDRI",
            "categories": ["hdri"],
            "tags": ["sky"],
            "download_count": 7
        });

        Mock::given(method("GET"))
            .and(path("/files/hdri01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(files_body))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/info/hdri01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(info_body))
            .mount(&server)
            .await;

        let client = PolyHavenClient::new_with_base_url(&server.uri()).unwrap();
        let resolved = client.resolve_hdri("hdri01", "2k").await.unwrap();

        assert_eq!(resolved.kind, "hdri");
        assert_eq!(resolved.resolution, "1k");
        assert_eq!(
            resolved.urls.get("hdri").unwrap(),
            &format!("{base}/dl/hdri_1k.exr")
        );
    }

    #[tokio::test]
    async fn test_resolve_model_picks_preferred_model_format() {
        let server = MockServer::start().await;

        let base = server.uri();
        let files_body = json!({
            "gltf": {
                "2k": {
                    "gltf": { "url": format!("{base}/dl/model_2k.gltf"), "size": 10, "md5": "" },
                    "glb": { "url": format!("{base}/dl/model_2k.glb"), "size": 11, "md5": "" }
                }
            }
        });

        let info_body = json!({
            "name": "Test Model",
            "categories": ["model"],
            "tags": ["mesh"],
            "download_count": 9
        });

        Mock::given(method("GET"))
            .and(path("/files/model01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(files_body))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/info/model01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(info_body))
            .mount(&server)
            .await;

        let client = PolyHavenClient::new_with_base_url(&server.uri()).unwrap();
        let resolved = client.resolve_model("model01", "4k", "fbx").await.unwrap();

        assert_eq!(resolved.kind, "model");
        assert_eq!(resolved.resolution, "2k");
        assert_eq!(
            resolved.urls.get("model").unwrap(),
            &format!("{base}/dl/model_2k.glb")
        );
    }

    #[tokio::test]
    #[cfg(feature = "live-api-tests")]
    async fn test_real_api_call() {
        let client = PolyHavenClient::default();

        // Test with known asset
        let result = client.get_info("aerial_rocks_02").await;
        assert!(result.is_ok(), "Failed to fetch real asset info");

        let info = result.unwrap();
        assert_eq!(info.name, "Aerial Rocks 02");
    }
}
