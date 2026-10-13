//! Where the demo reads models and where it writes saves.
//!
//! `cargo run` keeps the files in the repo. The Windows artifact ships
//! `assets/aravath` next to the exe, and that copy wins when it is present.

use std::path::PathBuf;

pub fn asset_dir() -> PathBuf {
    if let Some(bundled) = bundled_root() {
        return bundled;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/aravath")
}

pub fn save_dir() -> PathBuf {
    if bundled_root().is_some() {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join("saves");
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../saves")
}

fn bundled_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let bundled = dir.join("assets").join("aravath");
    if bundled.is_dir() {
        Some(bundled)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_dir_finds_the_character_library() {
        let path = asset_dir().join("models/AnimationLibrary_Godot_Standard.gltf");
        assert!(path.is_file(), "missing {path:?}");
    }
}
