use crate::config::LockEntry;
use prettytable::{format, Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Summary of fetch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchSummary {
    pub total_assets: usize,
    pub downloaded: usize,
    pub cached: usize,
    pub failed: usize,
    pub assets: Vec<AssetSummary>,
}

/// Individual asset summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSummary {
    pub handle: String,
    pub id: String,
    pub kind: String,
    pub status: String, // "downloaded", "cached", "failed"
    pub resolved_res: String,
    pub paths: HashMap<String, PathBuf>,
    pub error: Option<String>,
}

impl FetchSummary {
    /// Create new summary
    pub fn new() -> Self {
        Self {
            total_assets: 0,
            downloaded: 0,
            cached: 0,
            failed: 0,
            assets: Vec::new(),
        }
    }

    /// Add successful download
    pub fn add_downloaded(&mut self, entry: &LockEntry) {
        self.total_assets += 1;
        self.downloaded += 1;

        self.assets.push(AssetSummary {
            handle: entry.handle.clone(),
            id: entry.id.clone(),
            kind: entry.kind.clone(),
            status: "downloaded".to_string(),
            resolved_res: entry.resolved_res.clone(),
            paths: entry.paths.clone(),
            error: None,
        });
    }

    /// Add cached asset
    pub fn add_cached(&mut self, handle: String, id: String, kind: String, res: String) {
        self.total_assets += 1;
        self.cached += 1;

        self.assets.push(AssetSummary {
            handle,
            id,
            kind,
            status: "cached".to_string(),
            resolved_res: res,
            paths: HashMap::new(),
            error: None,
        });
    }

    /// Add failed asset
    pub fn add_failed(&mut self, handle: String, id: String, kind: String, error: String) {
        self.total_assets += 1;
        self.failed += 1;

        self.assets.push(AssetSummary {
            handle,
            id,
            kind,
            status: "failed".to_string(),
            resolved_res: String::new(),
            paths: HashMap::new(),
            error: Some(error),
        });
    }

    /// Print as table
    pub fn print_table(&self) {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_BOX_CHARS);

        // Header
        table.add_row(Row::new(vec![
            Cell::new("Handle"),
            Cell::new("Asset ID"),
            Cell::new("Type"),
            Cell::new("Status"),
            Cell::new("Resolution"),
            Cell::new("Files"),
        ]));

        // Rows
        for asset in &self.assets {
            let status_icon = match asset.status.as_str() {
                "downloaded" => "✅",
                "cached" => "💾",
                "failed" => "❌",
                _ => "❓",
            };

            let status_text = format!("{} {}", status_icon, asset.status);
            let file_count = format!("{} maps", asset.paths.len());

            table.add_row(Row::new(vec![
                Cell::new(&asset.handle),
                Cell::new(&asset.id),
                Cell::new(&asset.kind),
                Cell::new(&status_text),
                Cell::new(&asset.resolved_res),
                Cell::new(&file_count),
            ]));
        }

        // Print table
        table.printstd();

        // Summary
        println!("\n📊 Summary:");
        println!("  Total assets: {}", self.total_assets);
        println!("  ✅ Downloaded: {}", self.downloaded);
        println!("  💾 Cached: {}", self.cached);
        println!("  ❌ Failed: {}", self.failed);
    }

    /// Export as JSON
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

impl Default for FetchSummary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_json() {
        let mut summary = FetchSummary::new();

        summary.add_cached(
            "test".to_string(),
            "test_asset".to_string(),
            "texture".to_string(),
            "2k".to_string(),
        );

        let json = summary.to_json().unwrap();
        assert!(json.contains("\"total_assets\": 1"));
        assert!(json.contains("\"cached\": 1"));
    }

    #[test]
    fn test_fetch_summary_new() {
        let summary = FetchSummary::new();
        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.downloaded, 0);
        assert_eq!(summary.cached, 0);
        assert_eq!(summary.failed, 0);
        assert!(summary.assets.is_empty());
    }

    #[test]
    fn test_fetch_summary_default() {
        let summary = FetchSummary::default();
        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.downloaded, 0);
    }

    #[test]
    fn test_add_downloaded() {
        let mut summary = FetchSummary::new();
        let mut paths = HashMap::new();
        paths.insert("diffuse".to_string(), PathBuf::from("/tmp/diffuse.png"));

        let entry = LockEntry {
            handle: "brick_wall".to_string(),
            id: "brick_wall_001".to_string(),
            kind: "texture".to_string(),
            resolved_res: "2k".to_string(),
            urls: HashMap::new(),
            paths,
            hashes: HashMap::new(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        summary.add_downloaded(&entry);

        assert_eq!(summary.total_assets, 1);
        assert_eq!(summary.downloaded, 1);
        assert_eq!(summary.cached, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.assets[0].status, "downloaded");
        assert_eq!(summary.assets[0].handle, "brick_wall");
    }

    #[test]
    fn test_add_cached() {
        let mut summary = FetchSummary::new();

        summary.add_cached(
            "stone_floor".to_string(),
            "stone_floor_001".to_string(),
            "texture".to_string(),
            "4k".to_string(),
        );

        assert_eq!(summary.total_assets, 1);
        assert_eq!(summary.downloaded, 0);
        assert_eq!(summary.cached, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.assets[0].status, "cached");
        assert_eq!(summary.assets[0].resolved_res, "4k");
    }

    #[test]
    fn test_add_failed() {
        let mut summary = FetchSummary::new();

        summary.add_failed(
            "missing_asset".to_string(),
            "missing_001".to_string(),
            "hdri".to_string(),
            "404 Not Found".to_string(),
        );

        assert_eq!(summary.total_assets, 1);
        assert_eq!(summary.downloaded, 0);
        assert_eq!(summary.cached, 0);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.assets[0].status, "failed");
        assert_eq!(summary.assets[0].error, Some("404 Not Found".to_string()));
    }

    #[test]
    fn test_mixed_operations() {
        let mut summary = FetchSummary::new();

        // Add one of each
        let mut paths = HashMap::new();
        paths.insert("diffuse".to_string(), PathBuf::from("/tmp/diffuse.png"));
        let entry = LockEntry {
            handle: "downloaded_asset".to_string(),
            id: "asset_001".to_string(),
            kind: "texture".to_string(),
            resolved_res: "2k".to_string(),
            urls: HashMap::new(),
            paths,
            hashes: HashMap::new(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        summary.add_downloaded(&entry);
        summary.add_cached(
            "cached_asset".to_string(),
            "asset_002".to_string(),
            "hdri".to_string(),
            "1k".to_string(),
        );
        summary.add_failed(
            "failed_asset".to_string(),
            "asset_003".to_string(),
            "model".to_string(),
            "Network error".to_string(),
        );

        assert_eq!(summary.total_assets, 3);
        assert_eq!(summary.downloaded, 1);
        assert_eq!(summary.cached, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.assets.len(), 3);
    }

    #[test]
    fn test_asset_summary_clone() {
        let asset = AssetSummary {
            handle: "test".to_string(),
            id: "test_id".to_string(),
            kind: "texture".to_string(),
            status: "downloaded".to_string(),
            resolved_res: "2k".to_string(),
            paths: HashMap::new(),
            error: None,
        };
        let cloned = asset.clone();
        assert_eq!(asset.handle, cloned.handle);
        assert_eq!(asset.status, cloned.status);
    }

    #[test]
    fn test_fetch_summary_clone() {
        let mut summary = FetchSummary::new();
        summary.add_cached(
            "test".to_string(),
            "id".to_string(),
            "type".to_string(),
            "1k".to_string(),
        );

        let cloned = summary.clone();
        assert_eq!(summary.total_assets, cloned.total_assets);
        assert_eq!(summary.assets.len(), cloned.assets.len());
    }

    #[test]
    fn test_asset_summary_serialization() {
        let asset = AssetSummary {
            handle: "test_handle".to_string(),
            id: "test_id".to_string(),
            kind: "texture".to_string(),
            status: "cached".to_string(),
            resolved_res: "4k".to_string(),
            paths: HashMap::new(),
            error: None,
        };
        let json = serde_json::to_string(&asset).unwrap();
        assert!(json.contains("test_handle"));
        assert!(json.contains("cached"));
    }

    #[test]
    fn test_fetch_summary_empty_json() {
        let summary = FetchSummary::new();
        let json = summary.to_json().unwrap();
        assert!(json.contains("\"total_assets\": 0"));
        assert!(json.contains("\"assets\": []"));
    }

    #[test]
    fn test_asset_summary_with_error() {
        let asset = AssetSummary {
            handle: "error_asset".to_string(),
            id: "err_id".to_string(),
            kind: "hdri".to_string(),
            status: "failed".to_string(),
            resolved_res: String::new(),
            paths: HashMap::new(),
            error: Some("Connection timeout".to_string()),
        };
        assert!(asset.error.is_some());
        assert_eq!(asset.error.as_ref().unwrap(), "Connection timeout");
    }

    #[test]
    fn test_asset_summary_with_paths() {
        let mut paths = HashMap::new();
        paths.insert("diffuse".to_string(), PathBuf::from("/assets/diffuse.png"));
        paths.insert("normal".to_string(), PathBuf::from("/assets/normal.png"));

        let asset = AssetSummary {
            handle: "multi_map".to_string(),
            id: "mm_001".to_string(),
            kind: "texture".to_string(),
            status: "downloaded".to_string(),
            resolved_res: "2k".to_string(),
            paths,
            error: None,
        };
        assert_eq!(asset.paths.len(), 2);
        assert!(asset.paths.contains_key("diffuse"));
        assert!(asset.paths.contains_key("normal"));
    }

    #[test]
    fn test_print_table() {
        // Just verify it doesn't panic
        let mut summary = FetchSummary::new();
        summary.add_cached(
            "test".to_string(),
            "id".to_string(),
            "texture".to_string(),
            "1k".to_string(),
        );
        summary.add_failed(
            "fail".to_string(),
            "fail_id".to_string(),
            "hdri".to_string(),
            "error".to_string(),
        );

        // print_table writes to stdout, just ensure it doesn't panic
        summary.print_table();
    }
}
