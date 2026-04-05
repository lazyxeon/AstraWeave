use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Atomically write data to a file (write to .tmp, then rename).
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data).context("Failed to write temp file")?;
    fs::rename(&tmp, path).context("Failed to rename temp file")?;
    Ok(())
}

/// Sanitize a filename by removing path traversal characters.
pub fn sanitize_filename(name: &str) -> String {
    name.replace(['/', '\\'], "")
        .replace("..", "")
        .trim_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_removes_slashes() {
        assert_eq!(sanitize_filename("foo/bar"), "foobar");
        assert_eq!(sanitize_filename("foo\\bar"), "foobar");
    }

    #[test]
    fn test_sanitize_filename_removes_dotdot() {
        assert_eq!(sanitize_filename("../etc/passwd"), "etcpasswd");
    }

    #[test]
    fn test_sanitize_filename_trims_leading_trailing_dots() {
        assert_eq!(sanitize_filename(".hidden"), "hidden");
        assert_eq!(sanitize_filename("file."), "file");
        // Interior dots are preserved (valid in filenames like "file.txt")
        assert_eq!(sanitize_filename("file.txt"), "file.txt");
    }

    #[test]
    fn test_atomic_write_roundtrip() {
        let dir = std::env::temp_dir().join("aw_editor_test_atomic");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test_file.txt");
        atomic_write(&path, b"hello world").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello world");
        let _ = fs::remove_dir_all(&dir);
    }
}
