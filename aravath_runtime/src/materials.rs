//! Voxel materials 1–5 sample the pine-forest Poly Haven set.
//!
//! `diff` / `nor_gl` / `arm` are converted with `astraweave_render::pbr_maps`
//! into [`astraweave_render::ModelSurfaceMaps`]. KayKit props stay on the
//! greybox colored-mesh path.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::assets;

/// One material after the Poly Haven ARM split.
pub struct VoxelSurface {
    pub width: u32,
    pub height: u32,
    pub albedo: Vec<u8>,
    pub normal: Vec<u8>,
    pub metallic_roughness: Vec<u8>,
    pub occlusion: Vec<u8>,
}

/// Stone, dirt (road), grass, sand, wood.
pub fn material_slug(material: u16) -> &'static str {
    match material {
        2 => "rocky_trail",
        3 => "forest_ground_04",
        4 => "coast_land_rocks_01",
        5 => "pine_bark",
        _ => "aerial_rocks_01",
    }
}

/// Build engine maps from already-decoded RGBA8 images of the same size.
pub fn maps_from_polyhaven(
    diff: &[u8],
    nor_gl: &[u8],
    arm: &[u8],
    width: u32,
    height: u32,
) -> VoxelSurface {
    let (metallic_roughness, occlusion) = astraweave_render::pbr_maps::split_polyhaven_arm(arm);
    let normal = astraweave_render::pbr_maps::nor_gl_unchanged(nor_gl);
    VoxelSurface {
        width,
        height,
        albedo: diff.to_vec(),
        normal,
        metallic_roughness,
        occlusion,
    }
}

pub fn pine_forest_dir() -> PathBuf {
    let beside = assets::exe_dir().join("assets/terrain/polyhaven/pine_forest");
    if beside.is_dir() {
        return beside;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/terrain/polyhaven/pine_forest")
}

pub fn load_slug(slug: &str) -> Result<VoxelSurface> {
    let dir = pine_forest_dir();
    let diff = read_rgba(&dir.join(format!("{slug}_diff_2k.jpg")))?;
    let nor = read_rgba(&dir.join(format!("{slug}_nor_gl_2k.jpg")))?;
    let arm = read_rgba(&dir.join(format!("{slug}_arm_2k.jpg")))?;
    if diff.0 != nor.0 || diff.1 != nor.1 || diff.0 != arm.0 || diff.1 != arm.1 {
        bail!("{slug} maps are not the same size");
    }
    Ok(maps_from_polyhaven(&diff.2, &nor.2, &arm.2, diff.0, diff.1))
}

fn read_rgba(path: &Path) -> Result<(u32, u32, Vec<u8>)> {
    let meta = std::fs::metadata(path).with_context(|| format!("open {}", path.display()))?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > assets::MAX_TEXTURE_BYTES {
        bail!("texture {} is missing or too large", path.display());
    }
    let image = image::ImageReader::open(path)
        .with_context(|| format!("open {}", path.display()))?
        .with_guessed_format()
        .context("guess texture format")?
        .decode()
        .with_context(|| format!("decode {}", path.display()))?
        .into_rgba8();
    let width = image.width();
    let height = image.height();
    if width == 0
        || height == 0
        || width > assets::MAX_TEXTURE_EDGE
        || height > assets::MAX_TEXTURE_EDGE
    {
        bail!("texture {} has an invalid size", path.display());
    }
    Ok((width, height, image.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyhaven_arm_feeds_the_engine_layout() {
        let surface =
            maps_from_polyhaven(&[1, 2, 3, 255], &[9, 8, 7, 255], &[10, 80, 250, 255], 1, 1);
        assert_eq!(surface.albedo, vec![1, 2, 3, 255]);
        assert_eq!(surface.normal, vec![9, 8, 7, 255]);
        assert_eq!(surface.metallic_roughness, vec![250, 80, 0, 255]);
        assert_eq!(surface.occlusion, vec![10, 10, 10, 255]);
        assert_eq!(material_slug(1), "aerial_rocks_01");
        assert_eq!(material_slug(2), "rocky_trail");
        assert_eq!(material_slug(3), "forest_ground_04");
        assert_eq!(material_slug(4), "coast_land_rocks_01");
        assert_eq!(material_slug(5), "pine_bark");
    }

    #[test]
    fn pine_forest_2k_maps_are_vendored() {
        let dir = pine_forest_dir();
        for material in 1..=5 {
            let slug = material_slug(material);
            for role in ["diff", "nor_gl", "arm"] {
                let path = dir.join(format!("{slug}_{role}_2k.jpg"));
                assert!(path.is_file(), "missing {}", path.display());
            }
        }
    }
}
