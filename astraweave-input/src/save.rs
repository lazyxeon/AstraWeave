use crate::bindings::BindingSet;
use anyhow::Result;
use std::fs;

pub fn save_bindings(path: &str, b: &BindingSet) -> Result<()> {
    let txt = serde_json::to_string_pretty(b)?;
    fs::create_dir_all(
        std::path::Path::new(path)
            .parent()
            .unwrap_or(std::path::Path::new(".")),
    )?;
    fs::write(path, txt)?;
    Ok(())
}

pub fn load_bindings(path: &str) -> Option<BindingSet> {
    let txt = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&txt).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_save_bindings_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bindings.json");
        let path_str = path.to_str().unwrap();

        let bindings = BindingSet::default();
        let result = save_bindings(path_str, &bindings);

        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_save_bindings_creates_directory() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("bindings.json");
        let path_str = path.to_str().unwrap();

        let bindings = BindingSet::default();
        let result = save_bindings(path_str, &bindings);

        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_save_bindings_valid_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bindings.json");
        let path_str = path.to_str().unwrap();

        let bindings = BindingSet::default();
        save_bindings(path_str, &bindings).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_load_bindings_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bindings.json");
        let path_str = path.to_str().unwrap();

        let bindings = BindingSet::default();
        save_bindings(path_str, &bindings).unwrap();

        let loaded = load_bindings(path_str);
        assert!(loaded.is_some());
    }

    #[test]
    fn test_load_bindings_nonexistent_file() {
        let result = load_bindings("/nonexistent/path/bindings.json");
        assert!(result.is_none());
    }

    #[test]
    fn test_load_bindings_invalid_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bindings.json");
        let path_str = path.to_str().unwrap();

        fs::write(&path, "not valid json").unwrap();

        let loaded = load_bindings(path_str);
        assert!(loaded.is_none());
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bindings.json");
        let path_str = path.to_str().unwrap();

        let original = BindingSet::default();
        save_bindings(path_str, &original).unwrap();
        let loaded = load_bindings(path_str).unwrap();

        // BindingSet should roundtrip correctly - compare action counts
        assert_eq!(original.actions.len(), loaded.actions.len());

        // Verify all action keys are present
        for key in original.actions.keys() {
            assert!(loaded.actions.contains_key(key), "Missing action: {}", key);
        }

        // Verify move_axes (tuple comparison)
        assert_eq!(original.move_axes.0.axis, loaded.move_axes.0.axis);
        assert_eq!(original.move_axes.1.axis, loaded.move_axes.1.axis);

        // Verify look_axes (tuple comparison)
        assert_eq!(original.look_axes.0.axis, loaded.look_axes.0.axis);
        assert_eq!(original.look_axes.1.axis, loaded.look_axes.1.axis);
    }

    #[test]
    fn test_save_bindings_overwrites_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bindings.json");
        let path_str = path.to_str().unwrap();

        // Save first time
        let bindings1 = BindingSet::default();
        save_bindings(path_str, &bindings1).unwrap();
        let content1 = fs::read_to_string(&path).unwrap();

        // Save second time (should overwrite)
        let bindings2 = BindingSet::default();
        save_bindings(path_str, &bindings2).unwrap();
        let content2 = fs::read_to_string(&path).unwrap();

        // Both should be valid JSON
        let _: serde_json::Value = serde_json::from_str(&content1).unwrap();
        let _: serde_json::Value = serde_json::from_str(&content2).unwrap();
    }

    #[test]
    fn test_load_bindings_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bindings.json");
        let path_str = path.to_str().unwrap();

        fs::write(&path, "").unwrap();

        let loaded = load_bindings(path_str);
        assert!(loaded.is_none());
    }
}
