// tools/aw_editor/src/ui/branding.rs - AW branding assets
//
// Loads the AstraWeave logo art for the window/taskbar icon and the small
// top-bar brand mark. All loaders are fallible-but-non-essential: every
// failure path logs a `tracing::warn!` and returns `None` so the editor runs
// unbranded rather than failing to start.

use egui::ColorImage;
use std::sync::OnceLock;

const LOGO_FILE: &str = "Astraweave_logo.jpg";

/// Fractional crop of the AW glyph band inside the square logo art
/// (x0, y0, x1, y1 as fractions of the source dimensions). The source is a
/// square marketing image: glyph band in the upper-middle, wordmark below —
/// only the glyph band reads at icon sizes.
const GLYPH_BAND: (f32, f32, f32, f32) = (0.11, 0.21, 0.92, 0.67);

/// The 2048² JPEG decode is expensive (especially in dev profile) and both
/// the window icon and the brand mark need it — decode exactly once.
static LOGO_RGBA: OnceLock<Option<image::RgbaImage>> = OnceLock::new();

fn load_logo_rgba() -> Option<&'static image::RgbaImage> {
    LOGO_RGBA
        .get_or_init(|| {
            // Resolve through the same walk-up logic as all other editor
            // assets so branding survives launches from outside repo root.
            let path = crate::viewport::types::find_assets_dir().join(LOGO_FILE);
            let data = match std::fs::read(&path) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Branding: failed to read {}: {e}", path.display());
                    return None;
                }
            };
            match astraweave_asset::decode_limited_image(&data) {
                Ok(img) => Some(img.to_rgba8()),
                Err(e) => {
                    tracing::warn!("Branding: failed to decode {}: {e}", path.display());
                    None
                }
            }
        })
        .as_ref()
}

fn crop_glyph_band(rgba: &image::RgbaImage) -> Option<image::RgbaImage> {
    let (w, h) = rgba.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    let (fx0, fy0, fx1, fy1) = GLYPH_BAND;
    let x0 = (w as f32 * fx0) as u32;
    let y0 = (h as f32 * fy0) as u32;
    let cw = ((w as f32 * (fx1 - fx0)) as u32).clamp(1, w.saturating_sub(x0).max(1));
    let ch = ((h as f32 * (fy1 - fy0)) as u32).clamp(1, h.saturating_sub(y0).max(1));
    Some(image::imageops::crop_imm(rgba, x0, y0, cw, ch).to_image())
}

/// Window/taskbar icon: the AW glyph band letterboxed onto a transparent
/// square, downscaled to 256x256.
pub fn load_window_icon() -> Option<egui::IconData> {
    let rgba = load_logo_rgba()?;
    let band = crop_glyph_band(rgba)?;
    let (bw, bh) = band.dimensions();
    let side = bw.max(bh);
    let mut square = image::RgbaImage::from_pixel(side, side, image::Rgba([0, 0, 0, 0]));
    image::imageops::overlay(
        &mut square,
        &band,
        ((side - bw) / 2) as i64,
        ((side - bh) / 2) as i64,
    );
    // Triangle: ~10x cheaper than CatmullRom and indistinguishable at 256².
    // This runs before window creation, so it must stay fast in dev builds.
    let icon = image::imageops::resize(&square, 256, 256, image::imageops::FilterType::Triangle);
    Some(egui::IconData {
        rgba: icon.into_raw(),
        width: 256,
        height: 256,
    })
}

/// Small top-bar brand mark: the AW glyph band at thumbnail resolution.
/// Render at `(height * aspect, height)` where `aspect = size[0] / size[1]`.
pub fn load_brand_mark() -> Option<ColorImage> {
    let rgba = load_logo_rgba()?;
    let band = crop_glyph_band(rgba)?;
    let (bw, bh) = band.dimensions();
    let target_h = 40u32;
    let target_w = (((bw as f32 / bh as f32) * target_h as f32).round() as u32).max(1);
    // Triangle keeps the first post-splash frame hitch-free in dev builds.
    let small = image::imageops::resize(
        &band,
        target_w,
        target_h,
        image::imageops::FilterType::Triangle,
    );
    let pixels = small
        .pixels()
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    Some(ColorImage::new(
        [target_w as usize, target_h as usize],
        pixels,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_glyph_band_stays_in_bounds() {
        let src = image::RgbaImage::from_pixel(64, 64, image::Rgba([10, 20, 30, 255]));
        let band = crop_glyph_band(&src).expect("crop must succeed on a valid image");
        let (w, h) = band.dimensions();
        assert!(w >= 1 && w <= 64);
        assert!(h >= 1 && h <= 64);
        // Wider than tall: it's a band, not a square.
        assert!(w > h);
    }

    #[test]
    fn crop_glyph_band_rejects_empty_image() {
        let src = image::RgbaImage::new(0, 0);
        assert!(crop_glyph_band(&src).is_none());
    }
}
