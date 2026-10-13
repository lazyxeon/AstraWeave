//! Where the demo reads models and where it writes saves.
//!
//! The directory that contains the executable wins. A `cargo run` binary
//! lives under `target/`, so that case falls back to the repo assets.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

const REQUIRED: &[&str] = &[
    "models/AnimationLibrary_Godot_Standard.gltf",
    "models/AnimationLibrary_Godot_Standard.bin",
    "models/figurine.glb",
    "models/weapon-sword.glb",
    "models/weapon-shield.glb",
    "models/Textures/colormap.png",
    "models/nature/tree_oak.glb",
    "models/nature/tree_pineDefaultA.glb",
    "models/nature/tree_palmShort.glb",
    "models/nature/rock_largeA.glb",
    "models/nature/grass.glb",
    "models/nature/plant_bush.glb",
    "textures/grass_diff_1k.jpg",
    "textures/dirt_diff_1k.jpg",
    "textures/stone_diff_1k.jpg",
    "textures/sand_diff_1k.jpg",
    "textures/wood_diff_1k.jpg",
];

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn asset_dir() -> PathBuf {
    let beside = exe_dir().join("assets").join("aravath");
    if beside.is_dir() {
        return beside;
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let in_target = exe.components().any(|part| part.as_os_str() == "target");
    if in_target {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/aravath");
    }
    beside
}

pub fn save_dir() -> PathBuf {
    exe_dir().join("saves")
}

pub fn verify() -> Result<PathBuf> {
    let root = asset_dir();
    let mut missing = Vec::new();
    for relative in REQUIRED {
        let path = root.join(relative);
        if !path.is_file() {
            missing.push(relative.to_string());
        }
    }
    if !missing.is_empty() {
        bail!(
            "Aravath assets are incomplete.\nLooked in {}\nExecutable {}\nMissing:\n{}",
            root.display(),
            std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "(unknown)".into()),
            missing.join("\n")
        );
    }
    Ok(root)
}

pub fn read_texture(file: &str) -> Result<(u32, u32, Vec<u8>)> {
    let path = asset_dir().join("textures").join(file);
    let image = image::open(&path)
        .with_context(|| format!("open {}", path.display()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok((width, height, image.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_dir_finds_the_character_library() {
        let path = asset_dir().join("models/AnimationLibrary_Godot_Standard.gltf");
        assert!(path.is_file(), "missing {path:?}");
        assert!(verify().is_ok(), "{}", verify().unwrap_err());
    }
}
