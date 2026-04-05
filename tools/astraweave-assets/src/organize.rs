use crate::config::{LockEntry, Lockfile};
use crate::downloader::DownloadResult;
use crate::polyhaven::ResolvedAsset;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// Asset organizer - manages file naming, lockfile, and attribution
pub struct AssetOrganizer {
    pub output_dir: PathBuf,
    lockfile_path: PathBuf,
    attribution_path: PathBuf,
}

impl AssetOrganizer {
    /// Create new organizer
    pub fn new(output_dir: PathBuf, cache_dir: PathBuf) -> Self {
        let lockfile_path = cache_dir.join("polyhaven.lock");
        let attribution_path = output_dir.join("ATTRIBUTION.txt");

        Self {
            output_dir,
            lockfile_path,
            attribution_path,
        }
    }

    /// Organize downloaded files and update lockfile
    pub async fn organize(
        &self,
        handle: &str,
        asset: &ResolvedAsset,
        downloads: &HashMap<String, DownloadResult>,
    ) -> Result<LockEntry> {
        // Create asset directory
        let asset_dir = self.output_dir.join(handle);
        fs::create_dir_all(&asset_dir).await?;

        // Organize files with normalized names
        let mut paths = HashMap::new();
        let mut hashes = HashMap::new();
        let mut urls = HashMap::new();

        for (map_name, download) in downloads {
            // Determine extension from URL (not temp file path which has .tmp)
            let ext = if let Some(url) = asset.urls.get(map_name) {
                // Extract extension from URL (e.g., ".../texture_2k.png" -> "png")
                url.rsplit('.')
                    .next()
                    .and_then(|s| s.split('?').next()) // Remove query params
                    .unwrap_or("png")
            } else {
                // Fallback based on asset type
                match asset.kind.as_str() {
                    "hdri" => "exr",
                    "model" => "glb",
                    _ => "png",
                }
            };

            // Normalized filename: <handle>_<map>.<ext>
            let normalized_name = format!("{}_{}.{}", handle, map_name, ext);
            let dest_path = asset_dir.join(&normalized_name);

            // Move/copy file to normalized location
            if download.path != dest_path {
                fs::copy(&download.path, &dest_path).await.context(format!(
                    "Failed to copy {} to {}",
                    download.path.display(),
                    dest_path.display()
                ))?;

                // Remove original temp file
                let _ = fs::remove_file(&download.path).await;
            }

            paths.insert(map_name.clone(), dest_path);
            hashes.insert(map_name.clone(), download.sha256.clone());

            // Store original URL
            if let Some(url) = asset.urls.get(map_name) {
                urls.insert(map_name.clone(), url.clone());
            }
        }

        // Create lock entry
        let entry = LockEntry {
            handle: handle.to_string(),
            id: asset.id.clone(),
            kind: asset.kind.clone(),
            urls,
            paths,
            hashes,
            timestamp: chrono::Utc::now().to_rfc3339(),
            resolved_res: asset.resolution.clone(),
        };

        // Update lockfile
        self.update_lockfile(&entry).await?;

        // Update attribution
        self.update_attribution(&entry, asset).await?;

        Ok(entry)
    }

    /// Update lockfile with new entry
    async fn update_lockfile(&self, entry: &LockEntry) -> Result<()> {
        // Ensure cache dir exists
        if let Some(parent) = self.lockfile_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Load existing lockfile
        let mut lockfile = Lockfile::load(&self.lockfile_path).unwrap_or_else(|_| Lockfile {
            version: 1,
            assets: HashMap::new(),
        });

        // Update entry
        lockfile.assets.insert(entry.handle.clone(), entry.clone());

        // Save lockfile
        lockfile.save(&self.lockfile_path)?;

        Ok(())
    }

    /// Update attribution file
    async fn update_attribution(&self, entry: &LockEntry, asset: &ResolvedAsset) -> Result<()> {
        // Ensure output directory exists
        if let Some(parent) = self.attribution_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Load existing attribution
        let mut content = if self.attribution_path.exists() {
            fs::read_to_string(&self.attribution_path).await?
        } else {
            String::from("# AstraWeave Asset Attribution\n\n")
                + "All PolyHaven assets are licensed under CC0 (Public Domain).\n"
                + "Attribution is not required but we keep records for provenance.\n\n"
                + "---\n\n"
        };

        // Check if already documented
        if content.contains(&format!("## {}", entry.handle)) {
            // Update timestamp
            content = content.replace(
                &format!("## {}", entry.handle),
                &format!("## {} (Updated: {})", entry.handle, entry.timestamp),
            );
        } else {
            // Add new entry
            let attribution_entry = format!("## {} ({})\n\n", entry.handle, entry.timestamp)
                + &format!("- **Asset ID**: {}\n", entry.id)
                + &format!("- **Type**: {}\n", entry.kind)
                + &format!("- **Resolution**: {}\n", entry.resolved_res)
                + &format!("- **Source**: https://polyhaven.com/a/{}\n", entry.id)
                + "- **License**: CC0 (Public Domain)\n"
                + &format!("- **Tags**: {}\n", asset.info.tags.join(", "))
                + &format!("- **Downloads**: {}\n\n", asset.info.download_count)
                + "**Files**:\n";

            let mut file_list = attribution_entry;
            for (map, url) in &entry.urls {
                file_list += &format!("  - `{}`: {}\n", map, url);
            }
            file_list += "\n---\n\n";

            content += &file_list;
        }

        // Write attribution
        fs::write(&self.attribution_path, content).await?;

        Ok(())
    }

    /// Prune orphaned files (not in lockfile or manifest)
    pub async fn prune(&self, manifest_handles: &[String]) -> Result<Vec<PathBuf>> {
        let lockfile = Lockfile::load(&self.lockfile_path).unwrap_or_else(|_| Lockfile {
            version: 1,
            assets: HashMap::new(),
        });

        let mut pruned = Vec::new();

        // Collect all paths in lockfile
        let mut valid_paths: Vec<PathBuf> = lockfile
            .assets
            .values()
            .flat_map(|entry| entry.paths.values().cloned())
            .collect();

        // Add asset directories that should exist
        for handle in manifest_handles {
            valid_paths.push(self.output_dir.join(handle));
        }

        // Scan output directory
        if self.output_dir.exists() {
            let mut entries = fs::read_dir(&self.output_dir).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                // Check if path or its parent is valid
                let is_valid = valid_paths
                    .iter()
                    .any(|vp| path == *vp || path.starts_with(vp) || vp.starts_with(&path));

                if !is_valid {
                    // Prune this file/directory
                    if path.is_dir() {
                        fs::remove_dir_all(&path).await?;
                    } else {
                        fs::remove_file(&path).await?;
                    }

                    pruned.push(path);
                }
            }
        }

        Ok(pruned)
    }

    /// Get lockfile
    pub async fn load_lockfile(&self) -> Result<Lockfile> {
        Lockfile::load(&self.lockfile_path)
    }

    /// Check if asset is cached and valid
    pub async fn is_cached(&self, handle: &str) -> bool {
        if let Ok(lockfile) = self.load_lockfile().await {
            if let Some(entry) = lockfile.assets.get(handle) {
                // Check all paths exist
                for path in entry.paths.values() {
                    if !path.exists() {
                        return false;
                    }
                }
                return true;
            }
        }
        false
    }

    /// Organize downloaded files (V2 - for new ResolvedAsset format)
    pub async fn organize_v2(
        &self,
        handle: &str,
        asset: &crate::provider::ResolvedAsset,
        downloads: &HashMap<String, DownloadResult>,
    ) -> Result<LockEntry> {
        // Create provider-specific subdirectory
        let provider_dir = self.output_dir.join(&asset.provider);
        let asset_dir = provider_dir.join(handle);
        fs::create_dir_all(&asset_dir).await?;

        // Organize files with normalized names
        let mut paths = HashMap::new();
        let mut hashes = HashMap::new();
        let mut urls = HashMap::new();

        for (map_name, download) in downloads {
            // Determine extension from URL
            let ext = if let Some(url) = asset.urls.get(map_name) {
                url.rsplit('.')
                    .next()
                    .and_then(|s| s.split('?').next())
                    .unwrap_or("bin")
            } else {
                // Fallback based on format metadata
                asset
                    .metadata
                    .get("format")
                    .map(|s| s.as_str())
                    .unwrap_or("bin")
            };

            // Normalized filename: <handle>_<map>.<ext>
            let normalized_name = format!("{}_{}.{}", handle, map_name, ext);
            let dest_path = asset_dir.join(&normalized_name);

            // Move/copy file to normalized location
            if download.path != dest_path {
                fs::copy(&download.path, &dest_path).await.context(format!(
                    "Failed to copy {} to {}",
                    download.path.display(),
                    dest_path.display()
                ))?;

                // Remove original temp file
                let _ = fs::remove_file(&download.path).await;
            }

            paths.insert(map_name.clone(), dest_path);
            hashes.insert(map_name.clone(), download.sha256.clone());

            // Store original URL
            if let Some(url) = asset.urls.get(map_name) {
                urls.insert(map_name.clone(), url.clone());
            }
        }

        // Create lock entry
        let entry = LockEntry {
            handle: handle.to_string(),
            id: asset
                .metadata
                .get("id")
                .cloned()
                .unwrap_or_else(|| handle.to_string()),
            kind: format!("{:?}", asset.asset_type).to_lowercase(),
            urls,
            paths,
            hashes,
            timestamp: chrono::Utc::now().to_rfc3339(),
            resolved_res: asset
                .metadata
                .get("resolution")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
        };

        // Update lockfile
        self.update_lockfile(&entry).await?;

        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_lockfile_update() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("output");
        let cache_dir = temp.path().join("cache");

        let organizer = AssetOrganizer::new(output_dir.clone(), cache_dir);

        let _asset = ResolvedAsset {
            id: "test_asset".to_string(),
            kind: "texture".to_string(),
            resolution: "2k".to_string(),
            urls: HashMap::new(),
            info: crate::polyhaven::InfoResponse {
                name: "Test Asset".to_string(),
                categories: vec![],
                tags: vec![],
                download_count: 0,
            },
        };

        let entry = LockEntry {
            handle: "test".to_string(),
            id: "test_asset".to_string(),
            kind: "texture".to_string(),
            urls: HashMap::new(),
            paths: HashMap::new(),
            hashes: HashMap::new(),
            timestamp: "2025-10-17T00:00:00Z".to_string(),
            resolved_res: "2k".to_string(),
        };

        organizer.update_lockfile(&entry).await.unwrap();

        // Verify lockfile exists and contains entry
        let lockfile = organizer.load_lockfile().await.unwrap();
        assert!(lockfile.assets.contains_key("test"));
    }

    #[tokio::test]
    async fn test_organize_normalizes_names_and_updates_attribution() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("output");
        let cache_dir = temp.path().join("cache");

        let organizer = AssetOrganizer::new(output_dir.clone(), cache_dir.clone());

        let mut urls = HashMap::new();
        urls.insert(
            "albedo".to_string(),
            "https://example.com/tex_albedo.png?token=1".to_string(),
        );

        let asset = ResolvedAsset {
            id: "test_asset".to_string(),
            kind: "texture".to_string(),
            resolution: "2k".to_string(),
            urls,
            info: crate::polyhaven::InfoResponse {
                name: "Test Asset".to_string(),
                categories: vec![],
                tags: vec!["tag1".to_string()],
                download_count: 123,
            },
        };

        // Create a fake downloaded temp file
        let dl_dir = temp.path().join("downloads");
        tokio::fs::create_dir_all(&dl_dir).await.unwrap();
        let src_path = dl_dir.join("random.tmp");
        tokio::fs::write(&src_path, b"content").await.unwrap();

        let mut downloads = HashMap::new();
        downloads.insert(
            "albedo".to_string(),
            DownloadResult {
                path: src_path.clone(),
                sha256: "deadbeef".to_string(),
                size: 7,
            },
        );

        let entry = organizer
            .organize("my_handle", &asset, &downloads)
            .await
            .unwrap();

        let dest_path = output_dir.join("my_handle").join("my_handle_albedo.png");
        assert!(dest_path.exists());
        assert!(entry
            .paths
            .get("albedo")
            .unwrap()
            .ends_with("my_handle_albedo.png"));

        // Lockfile exists
        let lockfile = Lockfile::load(&cache_dir.join("polyhaven.lock")).unwrap();
        assert!(lockfile.assets.contains_key("my_handle"));

        // Attribution written
        let attribution_path = output_dir.join("ATTRIBUTION.txt");
        let content = tokio::fs::read_to_string(&attribution_path).await.unwrap();
        assert!(content.contains("## my_handle"));
        assert!(content.contains("https://polyhaven.com/a/test_asset"));
        assert!(content.contains("**Files**"));
    }

    #[tokio::test]
    async fn test_update_attribution_updates_existing_entry() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("output");
        let cache_dir = temp.path().join("cache");
        let organizer = AssetOrganizer::new(output_dir.clone(), cache_dir);

        tokio::fs::create_dir_all(&output_dir).await.unwrap();

        let asset = ResolvedAsset {
            id: "id".to_string(),
            kind: "texture".to_string(),
            resolution: "2k".to_string(),
            urls: HashMap::new(),
            info: crate::polyhaven::InfoResponse {
                name: "Name".to_string(),
                categories: vec![],
                tags: vec![],
                download_count: 0,
            },
        };

        let entry = LockEntry {
            handle: "h".to_string(),
            id: "id".to_string(),
            kind: "texture".to_string(),
            urls: HashMap::new(),
            paths: HashMap::new(),
            hashes: HashMap::new(),
            timestamp: "2025-10-17T00:00:00Z".to_string(),
            resolved_res: "2k".to_string(),
        };

        organizer.update_attribution(&entry, &asset).await.unwrap();
        organizer.update_attribution(&entry, &asset).await.unwrap();

        let content = tokio::fs::read_to_string(output_dir.join("ATTRIBUTION.txt"))
            .await
            .unwrap();
        assert!(content.contains("## h"));
        assert!(content.contains("Updated:"));
    }

    #[tokio::test]
    async fn test_is_cached_false_when_missing_files() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("output");
        let cache_dir = temp.path().join("cache");
        let organizer = AssetOrganizer::new(output_dir.clone(), cache_dir.clone());

        // Write lockfile that references a missing path
        let mut entry = LockEntry {
            handle: "h".to_string(),
            id: "id".to_string(),
            kind: "texture".to_string(),
            urls: HashMap::new(),
            paths: HashMap::new(),
            hashes: HashMap::new(),
            timestamp: "t".to_string(),
            resolved_res: "2k".to_string(),
        };
        entry.paths.insert(
            "albedo".to_string(),
            output_dir.join("h").join("missing.png"),
        );

        let mut lockfile = Lockfile {
            version: 1,
            assets: HashMap::new(),
        };
        lockfile.assets.insert("h".to_string(), entry);
        tokio::fs::create_dir_all(&cache_dir).await.unwrap();
        lockfile.save(&cache_dir.join("polyhaven.lock")).unwrap();

        assert!(!organizer.is_cached("h").await);
    }

    #[tokio::test]
    async fn test_prune_removes_orphaned_paths() {
        let temp = TempDir::new().unwrap();
        let output_dir = temp.path().join("output");
        let cache_dir = temp.path().join("cache");
        let organizer = AssetOrganizer::new(output_dir.clone(), cache_dir);

        tokio::fs::create_dir_all(output_dir.join("keep"))
            .await
            .unwrap();
        tokio::fs::create_dir_all(output_dir.join("junk"))
            .await
            .unwrap();
        tokio::fs::write(output_dir.join("junk").join("x.txt"), b"x")
            .await
            .unwrap();

        let pruned = organizer.prune(&["keep".to_string()]).await.unwrap();

        assert!(output_dir.join("keep").exists());
        assert!(!output_dir.join("junk").exists());
        assert!(!pruned.is_empty());
    }
}
