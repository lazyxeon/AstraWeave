//! Asset validation module for Phase PBR-G
//!
//! Provides comprehensive validation for:
//! - ORM channel order (Occlusion=R, Roughness=G, Metallic=B)
//! - Mipmap presence and completeness
//! - Texture size limits and power-of-two constraints
//! - Color-space correctness (sRGB vs linear)
//! - Normal map format validation
//! - Manifest signature verification (Task 1.2)

use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Validation result for an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub asset_path: String,
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

impl ValidationResult {
    pub fn new(asset_path: impl Into<String>) -> Self {
        Self {
            asset_path: asset_path.into(),
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.passed = false;
        self.errors.push(msg.into());
    }

    pub fn warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.info.push(msg.into());
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.passed &= other.passed;
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.info.extend(other.info);
    }
}

/// Texture validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureValidationConfig {
    /// Require power-of-two dimensions
    pub require_pot: bool,
    /// Maximum texture dimension (e.g., 4096)
    pub max_dimension: u32,
    /// Minimum texture dimension (e.g., 4)
    pub min_dimension: u32,
    /// Require mipmaps for textures larger than this size
    pub mipmap_threshold: u32,
    /// Validate ORM channel order (Occlusion=R, Roughness=G, Metallic=B)
    pub validate_orm_channels: bool,
    /// Require specific format for normal maps
    pub validate_normal_format: bool,
}

impl Default for TextureValidationConfig {
    fn default() -> Self {
        Self {
            require_pot: false, // Modern GPUs support NPOT
            max_dimension: 8192,
            min_dimension: 4,
            mipmap_threshold: 256, // Require mips for textures >= 256×256
            validate_orm_channels: true,
            validate_normal_format: true,
        }
    }
}

/// Validate a texture file
pub fn validate_texture(path: &Path, config: &TextureValidationConfig) -> Result<ValidationResult> {
    let mut result = ValidationResult::new(path.display().to_string());

    // Load image
    let img = match astraweave_asset::open_limited_image(path) {
        Ok(img) => img,
        Err(e) => {
            result.error(format!("Failed to load image: {}", e));
            return Ok(result);
        }
    };

    let (width, height) = img.dimensions();
    result.info(format!("Dimensions: {}×{}", width, height));

    // Validate dimensions
    if width < config.min_dimension || height < config.min_dimension {
        result.error(format!(
            "Texture too small: {}×{} (minimum: {}×{})",
            width, height, config.min_dimension, config.min_dimension
        ));
    }

    if width > config.max_dimension || height > config.max_dimension {
        result.error(format!(
            "Texture too large: {}×{} (maximum: {}×{})",
            width, height, config.max_dimension, config.max_dimension
        ));
    }

    // Validate power-of-two
    if config.require_pot {
        if !is_power_of_two(width) || !is_power_of_two(height) {
            result.error(format!(
                "Non-power-of-two dimensions: {}×{} (required for compatibility)",
                width, height
            ));
        }
    } else if !is_power_of_two(width) || !is_power_of_two(height) {
        result.warning(format!(
            "Non-power-of-two dimensions: {}×{} (may not compress efficiently)",
            width, height
        ));
    }

    // Check if mipmaps are recommended
    let max_dim = width.max(height);
    if max_dim >= config.mipmap_threshold {
        result.info(format!(
            "Texture size {}×{} should have mipmaps (threshold: {})",
            width, height, config.mipmap_threshold
        ));
    }

    // Infer texture type from filename and validate accordingly
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let filename_lower = filename.to_lowercase();

    if filename_lower.contains("_n.") || filename_lower.contains("_normal.") {
        validate_normal_map(&img, &mut result, config);
    } else if filename_lower.contains("_mra.") || filename_lower.contains("_orm.") {
        validate_orm_map(&img, &mut result, config);
    } else if filename_lower.contains("_albedo.")
        || filename_lower.contains("_diffuse.")
        || filename_lower.contains("_basecolor.")
    {
        validate_albedo_map(&img, &mut result);
    }

    Ok(result)
}

/// Validate KTX2 texture file for mipmap presence
/// Supports both true KTX2 and legacy AWTEX2 formats during migration
pub fn validate_ktx2_mipmaps(path: &Path) -> Result<ValidationResult> {
    let mut result = ValidationResult::new(path.display().to_string());

    // Read file header
    let data = std::fs::read(path).context("Failed to read texture file")?;

    if data.len() < 12 {
        result.error("File too small to be valid texture file");
        return Ok(result);
    }

    // Check magic bytes to determine format
    let identifier = &data[0..12];
    let ktx2_id = &[
        0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
    ];
    let awtex2_id = b"AWTEX2\0\0";

    if identifier == ktx2_id {
        // True KTX2 file
        result.info("Format: True KTX2 (standard)");

        if data.len() < 80 {
            result.error("KTX2 file too small (< 80 bytes)");
            return Ok(result);
        }

        // Read level count from true KTX2 header
        // KTX2 header: vkFormat(12), typeSize(16), width(20), height(24),
        //              depth(28), layerCount(32), faceCount(36), levelCount(40)
        let level_count = u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
        result.info(format!("Mip levels: {}", level_count));

        if level_count == 1 {
            result.warning("No mipmaps present (level count = 1)");
        } else if level_count == 0 {
            result.error("Invalid mip level count (0)");
        } else {
            result.info(format!("Has {} mip levels (full chain)", level_count));
        }
    } else if &identifier[0..8] == awtex2_id {
        // Legacy AWTEX2 file (temporary during migration)
        result.warning("Format: Legacy AWTEX2 (should migrate to true KTX2)");

        if data.len() < 28 {
            result.error("AWTEX2 file too small (< 28 bytes)");
            return Ok(result);
        }

        // Read mip count from AWTEX2 header (offset 24)
        let level_count = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        result.info(format!("Mip levels: {}", level_count));

        if level_count == 1 {
            result.warning("No mipmaps present (level count = 1)");
        } else if level_count == 0 {
            result.error("Invalid mip level count (0)");
        } else {
            result.info(format!("Has {} mip levels (full chain)", level_count));
        }
    } else {
        result.error("Unknown texture format (not KTX2 or AWTEX2)");
    }

    Ok(result)
}

/// Validate normal map (should be BC5/RG format, values centered around 0.5)
fn validate_normal_map(
    img: &DynamicImage,
    result: &mut ValidationResult,
    config: &TextureValidationConfig,
) {
    if !config.validate_normal_format {
        return;
    }

    result.info("Detected as normal map");

    let rgb_img = img.to_rgb8();
    let (width, height) = rgb_img.dimensions();

    // Sample center of texture (avoid edges which might have artifacts)
    let sample_x = width / 2;
    let sample_y = height / 2;
    let pixel = rgb_img.get_pixel(sample_x, sample_y);

    // Normal maps should have values around 127-128 (0.5 in [0,1] space)
    // Pure blue (0, 0, 255) or mid-range (128, 128, 255) are common
    let r = pixel[0];
    let g = pixel[1];
    let b = pixel[2];

    // Check if Z channel (blue) is dominant
    if b < 200 {
        result.warning(format!(
            "Normal map Z channel (blue) unexpectedly low: {} (expected ~255)",
            b
        ));
    }

    // Check if X/Y channels (red/green) are centered
    let r_centered = (r as i32 - 128).abs() < 100;
    let g_centered = (g as i32 - 128).abs() < 100;

    if !r_centered || !g_centered {
        result.info(format!(
            "Normal map center sample: R={}, G={}, B={} (XY should be ~128, Z should be ~255)",
            r, g, b
        ));
    }

    // Check format: BC5 stores 2 channels (RG), so if file has 3+ channels, suggest BC5
    result.info("Normal maps should use BC5 compression (2-channel RG format) for best quality");
}

/// Validate ORM map (Occlusion=R, Roughness=G, Metallic=B)
fn validate_orm_map(
    img: &DynamicImage,
    result: &mut ValidationResult,
    config: &TextureValidationConfig,
) {
    if !config.validate_orm_channels {
        return;
    }

    result.info("Detected as ORM/MRA map");

    let rgb_img = img.to_rgb8();
    let (width, height) = rgb_img.dimensions();

    // Sample multiple points to check channel usage
    let samples = [
        (width / 4, height / 4),
        (width / 2, height / 2),
        (3 * width / 4, 3 * height / 4),
    ];

    let mut r_used = false;
    let mut g_used = false;
    let mut b_used = false;

    for (x, y) in samples.iter() {
        let pixel = rgb_img.get_pixel(*x, *y);
        // Check if channels have variation (not all 0 or all 255)
        if pixel[0] > 10 && pixel[0] < 245 {
            r_used = true;
        }
        if pixel[1] > 10 && pixel[1] < 245 {
            g_used = true;
        }
        if pixel[2] > 10 && pixel[2] < 245 {
            b_used = true;
        }
    }

    // Warn if channels appear unused
    if !r_used {
        result
            .warning("R channel (Occlusion) appears unused (all values near 0 or 255)".to_string());
    }
    if !g_used {
        result
            .warning("G channel (Roughness) appears unused (all values near 0 or 255)".to_string());
    }
    if !b_used {
        result
            .warning("B channel (Metallic) appears unused (all values near 0 or 255)".to_string());
    }

    result.info(
        "Expected channel order: R=Occlusion/AO, G=Roughness, B=Metallic (ORM/MRA format)"
            .to_string(),
    );
}

/// Validate albedo map (should have color variation, reasonable brightness)
fn validate_albedo_map(img: &DynamicImage, result: &mut ValidationResult) {
    result.info("Detected as albedo/diffuse map");

    let rgb_img = img.to_rgb8();
    let (width, height) = rgb_img.dimensions();

    // Sample center
    let pixel = rgb_img.get_pixel(width / 2, height / 2);
    let r = pixel[0] as f32 / 255.0;
    let g = pixel[1] as f32 / 255.0;
    let b = pixel[2] as f32 / 255.0;

    // Calculate luminance
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    // Check for unrealistic albedo values (PBR guidelines: 30-240 sRGB)
    if luminance < 0.1 {
        result.warning(format!(
            "Albedo appears very dark (luminance: {:.2}) - may be too dark for PBR",
            luminance
        ));
    } else if luminance > 0.95 {
        result.warning(format!(
            "Albedo appears very bright (luminance: {:.2}) - pure white is uncommon in nature",
            luminance
        ));
    }

    // Check for pure grayscale (all channels equal)
    if (r - g).abs() < 0.05 && (g - b).abs() < 0.05 {
        result.info(
            "Albedo appears grayscale (all channels similar) - consider adding color variation",
        );
    }

    result.info("Albedo should be in sRGB color space for correct rendering");
}

/// Check if a number is a power of two
fn is_power_of_two(n: u32) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

/// Validate material TOML file structure
pub fn validate_material_toml(path: &Path) -> Result<ValidationResult> {
    let mut result = ValidationResult::new(path.display().to_string());

    let content = std::fs::read_to_string(path).context("Failed to read TOML file")?;

    // Try to parse as generic TOML first
    let parsed: Result<toml::Value, _> = toml::from_str(&content);
    if let Err(e) = parsed {
        result.error(format!("TOML parse error: {}", e));
        return Ok(result);
    }

    let parsed = parsed.unwrap();

    // Check for required sections based on material type
    if let Some(table) = parsed.as_table() {
        // Check for terrain materials (Phase PBR-F) FIRST - they have "layers" array
        if table.contains_key("layers") {
            result.info("Detected as terrain material (Phase PBR-F)");
            validate_terrain_material_structure(table, &mut result);
        }
        // Check for biome materials - they have "biome" field and "layer" array
        else if table.contains_key("biome") {
            result.info("Detected as biome material");
            validate_biome_material_structure(table, &mut result);
        }
        // Check for advanced materials (Phase PBR-E)
        else if table.contains_key("clearcoat") || table.contains_key("anisotropy") {
            result.info("Detected as advanced material (Phase PBR-E)");
            validate_advanced_material_structure(table, &mut result);
        } else {
            result.warning("Unknown material type - cannot validate structure");
        }
    }

    Ok(result)
}

fn validate_biome_material_structure(
    table: &toml::map::Map<String, toml::Value>,
    result: &mut ValidationResult,
) {
    // Check for required fields
    if !table.contains_key("biome") {
        result.error("Missing 'biome' field");
    }

    if !table.contains_key("layer") {
        result.error("Missing 'layer' array");
    } else if let Some(layers) = table.get("layer") {
        if let Some(layer_arr) = layers.as_array() {
            result.info(format!("Has {} material layers", layer_arr.len()));

            // Validate each layer
            for (i, layer) in layer_arr.iter().enumerate() {
                if let Some(layer_table) = layer.as_table() {
                    validate_material_layer(layer_table, i, result);
                }
            }
        }
    }
}

fn validate_terrain_material_structure(
    table: &toml::map::Map<String, toml::Value>,
    result: &mut ValidationResult,
) {
    // Check for required terrain material fields
    if !table.contains_key("name") {
        result.error("Missing 'name' field");
    }

    if !table.contains_key("layers") {
        result.error("Missing 'layers' array");
    } else if let Some(layers) = table.get("layers") {
        if let Some(layer_arr) = layers.as_array() {
            let layer_count = layer_arr.len();
            result.info(format!("Has {} terrain layers", layer_count));

            if layer_count > 4 {
                result.error(format!(
                    "Too many terrain layers: {} (maximum: 4)",
                    layer_count
                ));
            } else if layer_count == 0 {
                result.error("No terrain layers defined");
            }

            // Validate each terrain layer
            for (i, layer) in layer_arr.iter().enumerate() {
                if let Some(layer_table) = layer.as_table() {
                    validate_terrain_layer(layer_table, i, result);
                }
            }
        }
    }

    // Check for optional but recommended fields
    if table.contains_key("triplanar_enabled") {
        result.info("Triplanar projection configured");
    }
    if table.contains_key("height_blend_enabled") {
        result.info("Height-based blending configured");
    }
}

fn validate_advanced_material_structure(
    table: &toml::map::Map<String, toml::Value>,
    result: &mut ValidationResult,
) {
    let mut feature_count = 0;

    if table.contains_key("clearcoat") {
        result.info("Clearcoat feature detected");
        feature_count += 1;
    }
    if table.contains_key("anisotropy") {
        result.info("Anisotropy feature detected");
        feature_count += 1;
    }
    if table.contains_key("subsurface") {
        result.info("Subsurface scattering feature detected");
        feature_count += 1;
    }
    if table.contains_key("sheen") {
        result.info("Sheen feature detected");
        feature_count += 1;
    }
    if table.contains_key("transmission") {
        result.info("Transmission feature detected");
        feature_count += 1;
    }

    if feature_count == 0 {
        result.warning("No advanced material features detected");
    } else {
        result.info(format!("{} advanced features configured", feature_count));
    }
}

fn validate_material_layer(
    layer: &toml::map::Map<String, toml::Value>,
    index: usize,
    result: &mut ValidationResult,
) {
    // Check for required texture fields
    let required_fields = ["albedo", "normal", "mra"];
    for field in &required_fields {
        if !layer.contains_key(*field) {
            result.warning(format!("Layer {}: missing '{}' texture", index, field));
        } else if let Some(path) = layer.get(*field).and_then(|v| v.as_str()) {
            // Check if path looks reasonable
            if !path.ends_with(".png") && !path.ends_with(".ktx2") {
                result.warning(format!(
                    "Layer {}: '{}' has unusual extension: {}",
                    index, field, path
                ));
            }
        }
    }
}

fn validate_terrain_layer(
    layer: &toml::map::Map<String, toml::Value>,
    index: usize,
    result: &mut ValidationResult,
) {
    // Check for required terrain layer fields
    let required_fields = ["name", "albedo", "normal", "orm", "uv_scale"];
    for field in &required_fields {
        if !layer.contains_key(*field) {
            result.warning(format!("Terrain layer {}: missing '{}'", index, field));
        }
    }

    // Validate uv_scale is an array of 2 floats
    if let Some(uv_scale) = layer.get("uv_scale") {
        if let Some(arr) = uv_scale.as_array() {
            if arr.len() != 2 {
                result.error(format!(
                    "Terrain layer {}: uv_scale must be [f32; 2], got {} elements",
                    index,
                    arr.len()
                ));
            }
        }
    }

    // Validate height_range is valid
    if let Some(height_range) = layer.get("height_range") {
        if let Some(arr) = height_range.as_array() {
            if arr.len() == 2 {
                if let (Some(min), Some(max)) = (arr[0].as_float(), arr[1].as_float()) {
                    if min >= max {
                        result.error(format!(
                            "Terrain layer {}: height_range[0] ({}) must be < height_range[1] ({})",
                            index, min, max
                        ));
                    }
                }
            }
        }
    }
}

/// Verify manifest signature (Task 1.2: Persistent Asset Signing Keys)
///
/// # Arguments
/// * `manifest_path` - Path to signed manifest JSON file
///
/// # Returns
/// ValidationResult indicating signature verification status
pub fn verify_manifest_signature(manifest_path: &Path) -> Result<ValidationResult> {
    use asset_signing::{KeyStore, VerifyKey};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let mut result = ValidationResult::new(manifest_path.display().to_string());

    // Read manifest file
    let manifest_content =
        std::fs::read_to_string(manifest_path).context("Failed to read manifest file")?;

    // Parse manifest JSON
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_content).context("Failed to parse manifest JSON")?;

    // Extract signature
    let signature = manifest_value
        .get("signature")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'signature' field in manifest"))?;

    // Extract public key
    let public_key_pem = manifest_value
        .get("public_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'public_key' field in manifest"))?;

    // Extract entries for canonical JSON
    let entries = manifest_value
        .get("entries")
        .ok_or_else(|| anyhow::anyhow!("Missing 'entries' field in manifest"))?;

    // Parse public key from PEM format
    let key_data: String = public_key_pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");

    let key_bytes = BASE64
        .decode(key_data.trim())
        .context("Failed to decode public key")?;

    if key_bytes.len() != 32 {
        result.error(format!(
            "Invalid public key length: {} (expected 32)",
            key_bytes.len()
        ));
        return Ok(result);
    }

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&key_bytes);

    let verifying_key = VerifyKey::from_bytes(&key_array)
        .map_err(|e| anyhow!("Invalid Ed25519 public key: {}", e))?;

    // Reconstruct canonical JSON (entries array only, for signature verification)
    let entries_json = serde_json::to_string(&entries)?;

    // Verify signature
    match KeyStore::verify(&verifying_key, &entries_json, signature) {
        Ok(true) => {
            result.info("Signature verification: PASSED".to_string());
            if let Some(signed_at) = manifest_value.get("signed_at").and_then(|v| v.as_str()) {
                result.info(format!("Signed at: {}", signed_at));
            }
        }
        Ok(false) => {
            result.error("Signature verification: FAILED - manifest has been tampered with");
        }
        Err(e) => {
            result.error(format!("Signature verification error: {}", e));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_power_of_two() {
        assert!(is_power_of_two(1));
        assert!(is_power_of_two(2));
        assert!(is_power_of_two(4));
        assert!(is_power_of_two(256));
        assert!(is_power_of_two(1024));
        assert!(is_power_of_two(4096));

        assert!(!is_power_of_two(0));
        assert!(!is_power_of_two(3));
        assert!(!is_power_of_two(100));
        assert!(!is_power_of_two(1023));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new("test.png");
        assert!(result.passed);

        result.warning("Test warning");
        assert!(result.passed);

        result.error("Test error");
        assert!(!result.passed);
    }
}
