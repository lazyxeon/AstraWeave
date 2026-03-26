//! Blend asset scanner for discovering .blend files and their decomposition status.

use std::path::{Path, PathBuf};

/// A discovered .blend file with its decomposition status.
#[derive(Debug, Clone)]
pub struct DiscoveredBlend {
    /// Absolute path to the .blend file.
    pub path: PathBuf,
    /// Display name (stem of the file).
    pub name: String,
    /// Whether this .blend has been decomposed (has sibling manifest.json + meshes/).
    pub is_decomposed: bool,
    /// Path to the manifest.json if decomposed.
    pub manifest_path: Option<PathBuf>,
}

/// Scans configured directories for .blend files.
pub struct BlendAssetScanner {
    scan_directories: Vec<PathBuf>,
    discovered: Vec<DiscoveredBlend>,
}

impl BlendAssetScanner {
    /// Create a new scanner with the given directories to scan.
    pub fn new(directories: Vec<PathBuf>) -> Self {
        Self {
            scan_directories: directories,
            discovered: Vec::new(),
        }
    }

    /// Create from editor preference strings (relative paths resolved against project root).
    pub fn from_preferences(dirs: &[String], project_root: &Path) -> Self {
        let scan_dirs = dirs
            .iter()
            .map(|d| project_root.join(d))
            .collect();
        Self::new(scan_dirs)
    }

    /// Perform a full scan of all configured directories.
    /// Returns the number of .blend files discovered.
    pub fn scan(&mut self) -> usize {
        self.discovered.clear();

        for dir in &self.scan_directories.clone() {
            if dir.is_dir() {
                self.scan_directory(dir);
            }
        }

        self.discovered.len()
    }

    /// Get all discovered .blend files.
    pub fn discovered(&self) -> &[DiscoveredBlend] {
        &self.discovered
    }

    /// Get only decomposed .blend files (ready to use as zone sources).
    pub fn decomposed(&self) -> Vec<&DiscoveredBlend> {
        self.discovered.iter().filter(|d| d.is_decomposed).collect()
    }

    /// Get only non-decomposed .blend files (need import first).
    pub fn pending_import(&self) -> Vec<&DiscoveredBlend> {
        self.discovered
            .iter()
            .filter(|d| !d.is_decomposed)
            .collect()
    }

    fn scan_directory(&mut self, dir: &Path) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.scan_directory(&path);
            } else if let Some(ext) = path.extension() {
                if ext == "blend" {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let (is_decomposed, manifest_path) = check_decomposition_status(&path);

                    self.discovered.push(DiscoveredBlend {
                        path,
                        name,
                        is_decomposed,
                        manifest_path,
                    });
                }
            }
        }
    }
}

/// Check if a .blend file has been decomposed by looking for sibling artifacts.
///
/// A .blend file at `path/scene.blend` is considered decomposed if:
/// - `path/scene/manifest.json` exists, OR
/// - `path/scene_decomposed/manifest.json` exists
fn check_decomposition_status(blend_path: &Path) -> (bool, Option<PathBuf>) {
    let stem = match blend_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return (false, None),
    };

    let parent = match blend_path.parent() {
        Some(p) => p,
        None => return (false, None),
    };

    // Check: parent/stem/manifest.json
    let manifest_a = parent.join(stem).join("manifest.json");
    if manifest_a.exists() {
        return (true, Some(manifest_a));
    }

    // Check: parent/stem_decomposed/manifest.json
    let manifest_b = parent
        .join(format!("{}_decomposed", stem))
        .join("manifest.json");
    if manifest_b.exists() {
        return (true, Some(manifest_b));
    }

    (false, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scanner_empty_directory() {
        let tmp = TempDir::new().unwrap();
        let mut scanner = BlendAssetScanner::new(vec![tmp.path().to_path_buf()]);
        let count = scanner.scan();
        assert_eq!(count, 0);
        assert!(scanner.discovered().is_empty());
    }

    #[test]
    fn test_scanner_discovers_blend_files() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("forest.blend"), b"fake blend").unwrap();
        std::fs::write(tmp.path().join("desert.blend"), b"fake blend").unwrap();
        std::fs::write(tmp.path().join("not_a_blend.txt"), b"text").unwrap();

        let mut scanner = BlendAssetScanner::new(vec![tmp.path().to_path_buf()]);
        let count = scanner.scan();
        assert_eq!(count, 2);
        assert!(scanner.discovered().iter().any(|d| d.name == "forest"));
        assert!(scanner.discovered().iter().any(|d| d.name == "desert"));
    }

    #[test]
    fn test_scanner_detects_decomposition() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("scene.blend"), b"fake").unwrap();

        // Create decomposed output directory
        let decomp_dir = tmp.path().join("scene");
        std::fs::create_dir(&decomp_dir).unwrap();
        std::fs::write(decomp_dir.join("manifest.json"), b"{}").unwrap();

        let mut scanner = BlendAssetScanner::new(vec![tmp.path().to_path_buf()]);
        scanner.scan();

        let scene = scanner.discovered().iter().find(|d| d.name == "scene").unwrap();
        assert!(scene.is_decomposed);
        assert!(scene.manifest_path.is_some());
    }

    #[test]
    fn test_scanner_not_decomposed() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("raw.blend"), b"fake").unwrap();

        let mut scanner = BlendAssetScanner::new(vec![tmp.path().to_path_buf()]);
        scanner.scan();

        let raw = scanner.discovered().iter().find(|d| d.name == "raw").unwrap();
        assert!(!raw.is_decomposed);
        assert!(raw.manifest_path.is_none());
    }

    #[test]
    fn test_scanner_decomposed_filter() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("done.blend"), b"fake").unwrap();
        std::fs::write(tmp.path().join("pending.blend"), b"fake").unwrap();

        let decomp_dir = tmp.path().join("done");
        std::fs::create_dir(&decomp_dir).unwrap();
        std::fs::write(decomp_dir.join("manifest.json"), b"{}").unwrap();

        let mut scanner = BlendAssetScanner::new(vec![tmp.path().to_path_buf()]);
        scanner.scan();

        assert_eq!(scanner.decomposed().len(), 1);
        assert_eq!(scanner.pending_import().len(), 1);
    }

    #[test]
    fn test_scanner_recursive_subdirectory() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.blend"), b"fake").unwrap();

        let mut scanner = BlendAssetScanner::new(vec![tmp.path().to_path_buf()]);
        let count = scanner.scan();
        assert_eq!(count, 1);
        assert_eq!(scanner.discovered()[0].name, "nested");
    }

    #[test]
    fn test_scanner_nonexistent_directory() {
        let mut scanner = BlendAssetScanner::new(vec![PathBuf::from("/nonexistent/dir")]);
        let count = scanner.scan();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_from_preferences() {
        let tmp = TempDir::new().unwrap();
        let dirs = vec!["blends".to_string()];
        let scanner = BlendAssetScanner::from_preferences(&dirs, tmp.path());
        assert_eq!(scanner.scan_directories.len(), 1);
        assert_eq!(scanner.scan_directories[0], tmp.path().join("blends"));
    }
}
