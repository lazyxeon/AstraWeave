//! Post-decomposition texture processing pipeline.
//!
//! Converts HDR/EXR textures to engine-friendly PNG format, generates
//! thumbnails for the asset browser, and normalizes PBR channel naming.

#[cfg(test)]
use crate::decomposer::AssetTexture;
use crate::decomposer::{DecomposedAsset, DecompositionResult, ExtractedHdri};
use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Configuration for the texture processing pipeline.
#[derive(Debug, Clone)]
pub struct TextureProcessingConfig {
    /// Convert EXR/HDR textures to PNG.
    pub convert_hdr_to_png: bool,
    /// Generate thumbnails for the asset browser.
    pub generate_thumbnails: bool,
    /// Thumbnail size (width and height).
    pub thumbnail_size: u32,
    /// Maximum texture resolution (textures larger than this are downscaled).
    pub max_texture_resolution: u32,
    /// Keep original HDR/EXR files after conversion.
    pub keep_originals: bool,
    /// JPEG quality for non-alpha textures (1-100).
    pub jpeg_quality: u8,
    /// Output format preference for standard textures.
    pub output_format: TextureOutputFormat,
}

/// Output format for converted textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureOutputFormat {
    /// PNG (lossless, supports alpha).
    Png,
    /// JPEG (lossy, no alpha, smaller files).
    Jpeg,
    /// Auto: PNG for normal/alpha maps, JPEG for diffuse/roughness.
    Auto,
}

impl Default for TextureProcessingConfig {
    fn default() -> Self {
        Self {
            convert_hdr_to_png: true,
            generate_thumbnails: true,
            thumbnail_size: 128,
            max_texture_resolution: 4096,
            keep_originals: false,
            jpeg_quality: 90,
            output_format: TextureOutputFormat::Png,
        }
    }
}

/// Result of processing a decomposition's textures.
#[derive(Debug, Clone)]
pub struct TextureProcessingResult {
    /// Number of textures converted from HDR/EXR to standard format.
    pub hdr_conversions: usize,
    /// Number of thumbnails generated.
    pub thumbnails_generated: usize,
    /// Number of textures downscaled to fit max resolution.
    pub downscaled: usize,
    /// Paths to generated thumbnails.
    pub thumbnail_paths: Vec<PathBuf>,
    /// Paths to converted HDRI environment maps (PNG versions).
    pub converted_hdri_paths: Vec<PathBuf>,
    /// Errors encountered (non-fatal).
    pub warnings: Vec<String>,
}

/// Processes all textures from a scene decomposition result.
///
/// This is the main entry point — call it after `SceneDecomposer::execute()`.
pub fn process_decomposition_textures(
    result: &DecompositionResult,
    config: &TextureProcessingConfig,
) -> Result<TextureProcessingResult> {
    let mut processing_result = TextureProcessingResult {
        hdr_conversions: 0,
        thumbnails_generated: 0,
        downscaled: 0,
        thumbnail_paths: Vec::new(),
        converted_hdri_paths: Vec::new(),
        warnings: Vec::new(),
    };

    // 1. Convert HDR/EXR textures in the textures/ directory
    if config.convert_hdr_to_png {
        if let Some(textures_dir) = &result.textures_dir {
            convert_hdr_textures_in_dir(textures_dir, config, &mut processing_result)?;
        }

        // 2. Convert HDRI environment maps
        let hdri_dir = result.output_dir.join("hdri");
        if hdri_dir.exists() {
            convert_hdri_files(&hdri_dir, &result.hdris, config, &mut processing_result)?;
        }
    }

    // 3. Enforce max resolution
    if let Some(textures_dir) = &result.textures_dir {
        enforce_max_resolution(textures_dir, config, &mut processing_result)?;
    }

    // 4. Generate thumbnails
    if config.generate_thumbnails {
        let thumbnails_dir = result.output_dir.join("thumbnails");
        generate_asset_thumbnails(result, &thumbnails_dir, config, &mut processing_result)?;
    }

    info!(
        "Texture processing complete: {} HDR conversions, {} thumbnails, {} downscaled",
        processing_result.hdr_conversions,
        processing_result.thumbnails_generated,
        processing_result.downscaled,
    );

    Ok(processing_result)
}

/// Convert all HDR/EXR files in a directory to PNG.
fn convert_hdr_textures_in_dir(
    textures_dir: &Path,
    config: &TextureProcessingConfig,
    result: &mut TextureProcessingResult,
) -> Result<()> {
    let entries = match std::fs::read_dir(textures_dir) {
        Ok(e) => e,
        Err(e) => {
            result
                .warnings
                .push(format!("Could not read textures directory: {e}"));
            return Ok(());
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "exr" || ext == "hdr" {
            match convert_hdr_file(&path, config) {
                Ok(output_path) => {
                    result.hdr_conversions += 1;
                    debug!(
                        "Converted HDR texture: {} → {}",
                        path.display(),
                        output_path.display()
                    );

                    if !config.keep_originals {
                        if let Err(e) = std::fs::remove_file(&path) {
                            result
                                .warnings
                                .push(format!("Could not remove original {}: {e}", path.display()));
                        }
                    }
                }
                Err(e) => {
                    result
                        .warnings
                        .push(format!("Failed to convert {}: {e}", path.display()));
                }
            }
        }
    }

    Ok(())
}

/// Convert a single HDR/EXR file to PNG (or JPEG based on config).
fn convert_hdr_file(path: &Path, config: &TextureProcessingConfig) -> Result<PathBuf> {
    let img = image::open(path)
        .with_context(|| format!("Failed to open HDR/EXR file: {}", path.display()))?;

    // Tonemap from HDR to LDR (simple Reinhard)
    let tonemapped = tonemap_reinhard(&img);

    // Choose output format based on config
    let (ext, format) = match config.output_format {
        TextureOutputFormat::Jpeg => ("jpg", ImageFormat::Jpeg),
        TextureOutputFormat::Png | TextureOutputFormat::Auto => ("png", ImageFormat::Png),
    };

    let output_path = path.with_extension(ext);
    tonemapped
        .save_with_format(&output_path, format)
        .with_context(|| {
            format!(
                "Failed to save converted texture: {}",
                output_path.display()
            )
        })?;

    Ok(output_path)
}

/// Convert HDRI environment maps to LDR PNG for preview/fallback.
fn convert_hdri_files(
    hdri_dir: &Path,
    hdris: &[ExtractedHdri],
    config: &TextureProcessingConfig,
    result: &mut TextureProcessingResult,
) -> Result<()> {
    for hdri in hdris {
        let hdri_path = hdri_dir.join(&hdri.filename);
        if !hdri_path.exists() {
            result
                .warnings
                .push(format!("HDRI file not found: {}", hdri_path.display()));
            continue;
        }

        let ext = hdri_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "exr" || ext == "hdr" {
            match convert_hdr_file(&hdri_path, config) {
                Ok(output_path) => {
                    result.hdr_conversions += 1;
                    result.converted_hdri_paths.push(output_path);
                    debug!("Converted HDRI: {}", hdri.original_name);
                }
                Err(e) => {
                    result.warnings.push(format!(
                        "Failed to convert HDRI {}: {e}",
                        hdri.original_name
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Enforce maximum texture resolution by downscaling oversized textures.
fn enforce_max_resolution(
    textures_dir: &Path,
    config: &TextureProcessingConfig,
    result: &mut TextureProcessingResult,
) -> Result<()> {
    let max = config.max_texture_resolution;

    let entries = match std::fs::read_dir(textures_dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Only process standard image formats
        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "tga" | "bmp") {
            continue;
        }

        match image::open(&path) {
            Ok(img) => {
                if img.width() > max || img.height() > max {
                    // Triangle filter is ~5x faster than Lanczos3 with acceptable quality
                    let resized = img.resize(max, max, image::imageops::FilterType::Triangle);
                    if let Err(e) = resized.save(&path) {
                        result
                            .warnings
                            .push(format!("Failed to downscale {}: {e}", path.display()));
                    } else {
                        result.downscaled += 1;
                        debug!(
                            "Downscaled {}×{} → {}×{}: {}",
                            img.width(),
                            img.height(),
                            resized.width(),
                            resized.height(),
                            path.display()
                        );
                    }
                }
            }
            Err(e) => {
                result
                    .warnings
                    .push(format!("Could not read {}: {e}", path.display()));
            }
        }
    }

    Ok(())
}

/// Generate thumbnails for all assets in the decomposition result.
fn generate_asset_thumbnails(
    decomp_result: &DecompositionResult,
    thumbnails_dir: &Path,
    config: &TextureProcessingConfig,
    result: &mut TextureProcessingResult,
) -> Result<()> {
    std::fs::create_dir_all(thumbnails_dir).with_context(|| {
        format!(
            "Failed to create thumbnails directory: {}",
            thumbnails_dir.display()
        )
    })?;

    let size = config.thumbnail_size;

    for asset in &decomp_result.assets {
        // Try to find the asset's diffuse/albedo texture for the thumbnail
        if let Some(thumb_path) =
            generate_thumbnail_for_asset(asset, &decomp_result.output_dir, thumbnails_dir, size)
        {
            result.thumbnail_paths.push(thumb_path);
            result.thumbnails_generated += 1;
        }
    }

    // Generate thumbnails for HDRIs
    let hdri_dir = decomp_result.output_dir.join("hdri");
    for hdri in &decomp_result.hdris {
        let source_path = hdri_dir.join(&hdri.filename);
        // Try the PNG-converted version first, then the original
        let png_path = source_path.with_extension("png");
        let effective_source = if png_path.exists() {
            png_path
        } else {
            source_path
        };

        if !effective_source.exists() {
            continue;
        }

        match generate_thumbnail(&effective_source, thumbnails_dir, &hdri.filename, size) {
            Ok(thumb_path) => {
                result.thumbnail_paths.push(thumb_path);
                result.thumbnails_generated += 1;
            }
            Err(e) => {
                result.warnings.push(format!(
                    "HDRI thumbnail failed for {}: {e}",
                    hdri.original_name
                ));
            }
        }
    }

    Ok(())
}

/// Generate a thumbnail for a single decomposed asset.
///
/// Uses the first diffuse/albedo texture found, or falls back to a
/// colored placeholder based on asset category.
fn generate_thumbnail_for_asset(
    asset: &DecomposedAsset,
    output_dir: &Path,
    thumbnails_dir: &Path,
    size: u32,
) -> Option<PathBuf> {
    // Try to use the diffuse texture as the thumbnail source
    let textures_dir = output_dir.join("textures");
    let diffuse_texture = asset
        .textures
        .iter()
        .find(|t| matches!(t.channel.as_str(), "diffuse" | "albedo" | "base_color"));

    if let Some(tex) = diffuse_texture {
        let tex_path = textures_dir.join(&tex.filename);
        if tex_path.exists() {
            let thumb_name = format!("{}.png", sanitize_filename(&asset.name));
            match generate_thumbnail(&tex_path, thumbnails_dir, &thumb_name, size) {
                Ok(path) => return Some(path),
                Err(e) => {
                    warn!("Thumbnail from texture failed for {}: {e}", asset.name);
                }
            }
        }
    }

    // Fallback: generate a colored placeholder
    let thumb_name = format!("{}.png", sanitize_filename(&asset.name));
    let thumb_path = thumbnails_dir.join(&thumb_name);
    let placeholder = generate_category_placeholder(&asset.category, size);
    match placeholder.save(&thumb_path) {
        Ok(()) => Some(thumb_path),
        Err(e) => {
            warn!("Placeholder thumbnail failed for {}: {e}", asset.name);
            None
        }
    }
}

/// Generate a thumbnail from a source image.
fn generate_thumbnail(
    source: &Path,
    thumbnails_dir: &Path,
    output_name: &str,
    size: u32,
) -> Result<PathBuf> {
    let img = image::open(source)
        .with_context(|| format!("Failed to open image for thumbnail: {}", source.display()))?;

    let thumbnail = img.thumbnail(size, size);
    let thumb_filename = Path::new(output_name).with_extension("png");
    let thumb_path = thumbnails_dir.join(thumb_filename);
    thumbnail
        .save(&thumb_path)
        .with_context(|| format!("Failed to save thumbnail: {}", thumb_path.display()))?;

    Ok(thumb_path)
}

/// Generate a colored placeholder thumbnail based on asset category.
fn generate_category_placeholder(category: &str, size: u32) -> DynamicImage {
    let (r, g, b) = match category {
        "vegetation" => (34, 139, 34),  // Forest green
        "rock" => (139, 137, 137),      // Gray
        "terrain" => (160, 120, 60),    // Earth brown
        "billboard" => (100, 149, 237), // Cornflower blue
        "prop" => (218, 165, 32),       // Goldenrod
        _ => (128, 128, 128),           // Neutral gray
    };

    let mut img = RgbaImage::new(size, size);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgba([r, g, b, 255]);
    }

    // Draw a simple border (2px darker edge)
    let dark_r = (r as u16 * 7 / 10) as u8;
    let dark_g = (g as u16 * 7 / 10) as u8;
    let dark_b = (b as u16 * 7 / 10) as u8;
    let border = image::Rgba([dark_r, dark_g, dark_b, 255]);

    for x in 0..size {
        for y in 0..size {
            if x < 2 || y < 2 || x >= size - 2 || y >= size - 2 {
                img.put_pixel(x, y, border);
            }
        }
    }

    DynamicImage::ImageRgba8(img)
}

/// Simple Reinhard tonemapping for HDR → LDR conversion.
///
/// Maps HDR values to [0, 255] using: L_out = L / (1 + L)
/// Uses a pre-computed gamma LUT (256 entries) instead of per-pixel powf() calls.
fn tonemap_reinhard(img: &DynamicImage) -> DynamicImage {
    // Pre-compute gamma correction LUT: index [0..255] → sRGB byte
    // Avoids 3× powf() per pixel (massive speedup for 4K+ textures)
    let gamma_lut: Vec<u8> = (0..=255)
        .map(|i| {
            let linear = i as f32 / 255.0;
            let srgb = linear.powf(1.0 / 2.2);
            (srgb * 255.0).clamp(0.0, 255.0) as u8
        })
        .collect();

    let rgb32f = img.to_rgb32f();
    let (w, h) = (rgb32f.width(), rgb32f.height());
    let raw = rgb32f.as_raw();
    let mut out = vec![0u8; (w * h * 4) as usize];

    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 3) as usize;
            let r = raw[idx];
            let g = raw[idx + 1];
            let b = raw[idx + 2];

            // Reinhard tonemapping per channel
            let tr = (r / (1.0 + r)).clamp(0.0, 1.0);
            let tg = (g / (1.0 + g)).clamp(0.0, 1.0);
            let tb = (b / (1.0 + b)).clamp(0.0, 1.0);

            // Gamma via LUT
            let out_idx = ((y * w + x) * 4) as usize;
            out[out_idx] = gamma_lut[(tr * 255.0) as usize];
            out[out_idx + 1] = gamma_lut[(tg * 255.0) as usize];
            out[out_idx + 2] = gamma_lut[(tb * 255.0) as usize];
            out[out_idx + 3] = 255;
        }
    }

    DynamicImage::ImageRgba8(RgbaImage::from_raw(w, h, out).expect("valid image dimensions"))
}

/// Sanitize a filename by replacing non-alphanumeric chars.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Normalize a PBR channel name from various Blender conventions.
///
/// Maps common texture suffixes to canonical channel names used by
/// the engine's material system.
pub fn normalize_channel_name(channel: &str) -> &'static str {
    let lower = channel.to_lowercase();
    let lower = lower.as_str();

    if lower.contains("diffuse")
        || lower.contains("albedo")
        || lower.contains("base_color")
        || lower.contains("basecolor")
        || lower.contains("color")
    {
        return "diffuse";
    }

    if lower.contains("normal") || lower.contains("nor") || lower == "n" {
        return "normal";
    }

    if lower.contains("roughness") || lower.contains("rough") {
        return "roughness";
    }

    if lower.contains("metallic") || lower.contains("metal") || lower.contains("metalness") {
        return "metallic";
    }

    if lower.contains("ao") || lower.contains("occlusion") || lower.contains("ambient") {
        return "ao";
    }

    if lower.contains("emission") || lower.contains("emissive") || lower.contains("emit") {
        return "emission";
    }

    if lower.contains("height") || lower.contains("displacement") || lower.contains("disp") {
        return "displacement";
    }

    if lower.contains("alpha") || lower.contains("opacity") || lower.contains("mask") {
        return "alpha";
    }

    if lower.contains("orm") {
        return "orm";
    }

    if lower.contains("mra") {
        return "mra";
    }

    "unknown"
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Boulder 01"), "Boulder_01");
        assert_eq!(sanitize_filename("tree.quiver"), "tree_quiver");
        assert_eq!(sanitize_filename("rock/cliff"), "rock_cliff");
        assert_eq!(sanitize_filename("simple_name"), "simple_name");
    }

    #[test]
    fn test_normalize_channel_name() {
        assert_eq!(normalize_channel_name("diffuse"), "diffuse");
        assert_eq!(normalize_channel_name("Albedo"), "diffuse");
        assert_eq!(normalize_channel_name("Base_Color"), "diffuse");
        assert_eq!(normalize_channel_name("basecolor"), "diffuse");
        assert_eq!(normalize_channel_name("Normal"), "normal");
        assert_eq!(normalize_channel_name("roughness"), "roughness");
        assert_eq!(normalize_channel_name("Metallic"), "metallic");
        assert_eq!(normalize_channel_name("ao"), "ao");
        assert_eq!(normalize_channel_name("Ambient_Occlusion"), "ao");
        assert_eq!(normalize_channel_name("emission"), "emission");
        assert_eq!(normalize_channel_name("height"), "displacement");
        assert_eq!(normalize_channel_name("displacement"), "displacement");
        assert_eq!(normalize_channel_name("alpha"), "alpha");
        assert_eq!(normalize_channel_name("opacity"), "alpha");
        assert_eq!(normalize_channel_name("ORM"), "orm");
        assert_eq!(normalize_channel_name("MRA"), "mra");
        assert_eq!(normalize_channel_name("unknown_thing"), "unknown");
    }

    #[test]
    fn test_category_placeholder_generation() {
        let img = generate_category_placeholder("vegetation", 64);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 64);

        // Check center pixel is green-ish
        let rgba = img.to_rgba8();
        let pixel = rgba.get_pixel(32, 32);
        assert_eq!(pixel[0], 34); // R
        assert_eq!(pixel[1], 139); // G
        assert_eq!(pixel[2], 34); // B
    }

    #[test]
    fn test_tonemap_reinhard() {
        // Create a 2x2 HDR image
        let mut rgb = image::Rgb32FImage::new(2, 2);
        rgb.put_pixel(0, 0, image::Rgb([1.0, 1.0, 1.0]));
        rgb.put_pixel(1, 0, image::Rgb([10.0, 0.0, 0.0])); // Bright red
        rgb.put_pixel(0, 1, image::Rgb([0.0, 0.0, 0.0])); // Black
        rgb.put_pixel(1, 1, image::Rgb([0.5, 0.5, 0.5]));

        let hdr = DynamicImage::ImageRgb32F(rgb);
        let ldr = tonemap_reinhard(&hdr);

        // Check it's a valid RGBA8 image
        let rgba = ldr.to_rgba8();
        assert_eq!(rgba.width(), 2);
        assert_eq!(rgba.height(), 2);

        // Black pixel should remain ~0
        let black = rgba.get_pixel(0, 1);
        assert_eq!(black[0], 0);
        assert_eq!(black[1], 0);
        assert_eq!(black[2], 0);
        assert_eq!(black[3], 255);

        // Bright red should be tonemapped (not clipped to 255)
        let red = rgba.get_pixel(1, 0);
        assert!(
            red[0] > 200,
            "Tonemapped red should be bright but not clipped: {}",
            red[0]
        );
        assert!(red[1] < 10, "Green channel of bright red should be near 0");
    }

    #[test]
    fn test_texture_processing_config_defaults() {
        let config = TextureProcessingConfig::default();
        assert!(config.convert_hdr_to_png);
        assert!(config.generate_thumbnails);
        assert_eq!(config.thumbnail_size, 128);
        assert_eq!(config.max_texture_resolution, 4096);
        assert!(!config.keep_originals);
    }

    #[test]
    fn test_process_with_empty_result() {
        let dir = TempDir::new().unwrap();
        let result = DecompositionResult {
            output_dir: dir.path().to_path_buf(),
            assets: vec![],
            empties: vec![],
            hdris: vec![],
            total_objects: 0,
            manifest_path: None,
            textures_dir: None,
            duration: std::time::Duration::ZERO,
            blender_version: "4.0.0".to_string(),
        };

        let config = TextureProcessingConfig::default();
        let proc_result = process_decomposition_textures(&result, &config).unwrap();
        assert_eq!(proc_result.hdr_conversions, 0);
        assert_eq!(proc_result.thumbnails_generated, 0);
        assert_eq!(proc_result.downscaled, 0);
    }

    #[test]
    fn test_process_with_png_textures() {
        let dir = TempDir::new().unwrap();
        let textures_dir = dir.path().join("textures");
        std::fs::create_dir_all(&textures_dir).unwrap();

        // Create a small PNG test texture
        let img = RgbaImage::new(256, 256);
        img.save(textures_dir.join("test_diffuse.png")).unwrap();

        let result = DecompositionResult {
            output_dir: dir.path().to_path_buf(),
            assets: vec![DecomposedAsset {
                name: "test_object".to_string(),
                filename: "meshes/test_object.glb".to_string(),
                category: "prop".to_string(),
                vertex_count: 100,
                file_size: 1000,
                bounds: None,
                dimensions: Some([1.0, 1.0, 1.0]),
                position: [0.0; 3],
                rotation: [0.0; 3],
                scale: [1.0; 3],
                textures: vec![AssetTexture {
                    filename: "test_diffuse.png".to_string(),
                    channel: "diffuse".to_string(),
                    original_name: "test_diffuse".to_string(),
                    width: 256,
                    height: 256,
                }],
                materials: vec!["TestMaterial".to_string()],
                collections: vec![],
            }],
            empties: vec![],
            hdris: vec![],
            total_objects: 1,
            manifest_path: None,
            textures_dir: Some(textures_dir),
            duration: std::time::Duration::ZERO,
            blender_version: "4.0.0".to_string(),
        };

        let config = TextureProcessingConfig::default();
        let proc_result = process_decomposition_textures(&result, &config).unwrap();

        // Should have generated a thumbnail (from diffuse texture)
        assert_eq!(proc_result.thumbnails_generated, 1);
        assert_eq!(proc_result.hdr_conversions, 0);

        // Thumbnail should exist
        let thumbnails_dir = dir.path().join("thumbnails");
        assert!(thumbnails_dir.exists());
    }

    #[test]
    fn test_enforce_max_resolution_downscales() {
        let dir = TempDir::new().unwrap();

        // Create an oversized texture (512x512, max 256)
        let img = RgbaImage::new(512, 512);
        let tex_path = dir.path().join("oversized.png");
        img.save(&tex_path).unwrap();

        let config = TextureProcessingConfig {
            max_texture_resolution: 256,
            ..Default::default()
        };

        let mut result = TextureProcessingResult {
            hdr_conversions: 0,
            thumbnails_generated: 0,
            downscaled: 0,
            thumbnail_paths: Vec::new(),
            converted_hdri_paths: Vec::new(),
            warnings: Vec::new(),
        };

        enforce_max_resolution(dir.path(), &config, &mut result).unwrap();
        assert_eq!(result.downscaled, 1);

        // Verify the image was actually resized
        let resized = image::open(&tex_path).unwrap();
        assert!(resized.width() <= 256);
        assert!(resized.height() <= 256);
    }

    #[test]
    fn test_generate_thumbnail() {
        let dir = TempDir::new().unwrap();
        let source_dir = dir.path().join("source");
        let thumb_dir = dir.path().join("thumbs");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(&thumb_dir).unwrap();

        // Create a source image
        let img = RgbaImage::new(512, 512);
        let source_path = source_dir.join("test.png");
        img.save(&source_path).unwrap();

        let result = generate_thumbnail(&source_path, &thumb_dir, "test_thumb.png", 64).unwrap();
        assert!(result.exists());

        let thumb = image::open(&result).unwrap();
        assert!(thumb.width() <= 64);
        assert!(thumb.height() <= 64);
    }
}
