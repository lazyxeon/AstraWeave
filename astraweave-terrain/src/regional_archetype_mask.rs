//! Phase 1.X-F.4: paintable 2D archetype mask + falloff distance field for
//! the Regional Archetype Variation campaign's multi-archetype data path.
//!
//! Per campaign doc §2.4: each pixel of the 1024×1024 mask carries an
//! archetype ID (uint8) + a precomputed Euclidean distance-to-nearest-other-
//! archetype (uint8 normalized by `falloff_radius_pixels`). At Target B's
//! ~11264 WU world extent, this gives ~11 WU per pixel — sufficient for
//! archetype-region authoring; transitions handled by falloff at
//! sampling time, not by mask resolution.
//!
//! F.4.A scope (this commit):
//! - [`RegionalArchetypeMask`] type + unpainted constructor + paint helpers.
//! - [`RegionalArchetypeMaskMetadata`] for serde (RON) persistence.
//! - [`crate::world_archetypes::WorldArchetypeId`] mask-id bidirectional mapping.
//!
//! F.4.B adds save/load file I/O + Euclidean distance transform.
//! F.4.C adds [`RegionalArchetypeBlend`] sampler.
//! F.4.D adds blend math (`blend_bootstrap_params`).
//! F.4.E wires into `WorldGenerator`.

use serde::{Deserialize, Serialize};

/// Phase 1.X-F.4.A: paintable 2D archetype mask with precomputed
/// Euclidean distance falloff field. Stored as two uint8 grids:
/// `ids` (archetype identity per pixel; 0 = unpainted) and `falloff`
/// (distance-to-nearest-other-archetype per pixel, normalized by
/// `falloff_radius_pixels` to uint8; 0 = on a boundary; 255 = deep
/// interior).
///
/// Default 1024×1024 resolution at Target B's 11264 WU world extent
/// gives ~11 WU per pixel. Memory cost: 2 MB total (1 MB per channel).
/// Disk cost (uncompressed): ~10-100 KB compressed since archetype IDs
/// have low entropy.
///
/// IDs 0 reserved for unpainted (sample-time fallback to Continental
/// Temperate). IDs 1-6 map to the 6 D.5 catalog archetypes via
/// [`crate::world_archetypes::WorldArchetypeId::to_mask_id`]. IDs 7-255
/// reserved for future expansion.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionalArchetypeMask {
    /// Mask resolution per side (default `DEFAULT_RESOLUTION = 1024`).
    pub resolution: u32,
    /// World extent in WU per side (default `DEFAULT_WORLD_EXTENT_WU = 11264.0`
    /// at Target B's radius-10 chunk-512 configuration).
    pub world_extent_wu: f32,
    /// Falloff distance in pixels (default `DEFAULT_FALLOFF_RADIUS_PIXELS = 32`,
    /// = ~352 WU at default resolution + extent). Pixels closer than this to
    /// a boundary fall in the transition zone; pixels farther are deep interior.
    pub falloff_radius_pixels: u32,
    /// Archetype ID per pixel (uint8). 0 = unpainted; 1-6 = catalog archetypes;
    /// 7-255 = reserved. Length = `resolution × resolution`.
    pub ids: Vec<u8>,
    /// Euclidean distance-to-nearest-other-archetype per pixel (uint8 normalized).
    /// 0 = on a boundary; 255 = deep interior (distance ≥ falloff_radius_pixels).
    /// Computed by [`Self::recompute_falloff`] (F.4.B); F.4.A's paint helpers
    /// leave this at default (255) until distance transform runs.
    /// Length = `resolution × resolution`.
    pub falloff: Vec<u8>,
}

impl RegionalArchetypeMask {
    /// Default resolution: 1024×1024 (per campaign doc §2.4).
    pub const DEFAULT_RESOLUTION: u32 = 1024;
    /// Default world extent: 11264 WU per side (Target B, radius 10 × chunk 512 × 2.2).
    /// Slightly larger than the radius-10 world's 11264 WU per side to absorb
    /// rounding in mask-pixel conversion.
    pub const DEFAULT_WORLD_EXTENT_WU: f32 = 11264.0;
    /// Default falloff radius: 32 pixels = ~352 WU at default resolution + extent.
    pub const DEFAULT_FALLOFF_RADIUS_PIXELS: u32 = 32;

    /// Construct an unpainted mask: all pixels' archetype ID = 0 (unpainted),
    /// all falloff = 255 (deep interior; no transition zones since there are
    /// no archetypes painted).
    pub fn new_unpainted(resolution: u32, world_extent_wu: f32) -> Self {
        let n = (resolution as usize).saturating_mul(resolution as usize);
        Self {
            resolution,
            world_extent_wu,
            falloff_radius_pixels: Self::DEFAULT_FALLOFF_RADIUS_PIXELS,
            ids: vec![0u8; n],
            falloff: vec![255u8; n],
        }
    }

    /// Index a pixel by `(x, z)` in mask coordinates. Returns `None` for
    /// out-of-range coords.
    #[inline]
    pub fn pixel_index(&self, x: u32, z: u32) -> Option<usize> {
        if x < self.resolution && z < self.resolution {
            Some(z as usize * self.resolution as usize + x as usize)
        } else {
            None
        }
    }

    /// Read the archetype ID at a mask pixel. Returns 0 for out-of-range.
    #[inline]
    pub fn id_at(&self, x: u32, z: u32) -> u8 {
        self.pixel_index(x, z)
            .map(|idx| self.ids[idx])
            .unwrap_or(0)
    }

    /// Read the falloff value at a mask pixel. Returns 255 (deep interior)
    /// for out-of-range.
    #[inline]
    pub fn falloff_at(&self, x: u32, z: u32) -> u8 {
        self.pixel_index(x, z)
            .map(|idx| self.falloff[idx])
            .unwrap_or(255)
    }

    /// Phase 1.X-F.4.A test helper: paint a circular region with the given
    /// archetype ID. Builder-pattern; returns `Self` for chaining.
    ///
    /// Uses pixel-coordinate radius (NOT world units). Pixels outside the
    /// mask's bounds are silently skipped. Existing pixels overwritten.
    /// Falloff field NOT recomputed; call [`Self::recompute_falloff`]
    /// after all paint operations to update the distance field.
    pub fn with_painted_circle(
        mut self,
        center_x_px: u32,
        center_y_px: u32,
        radius_px: u32,
        archetype_id: u8,
    ) -> Self {
        let r2 = (radius_px as i64).pow(2);
        let cx = center_x_px as i64;
        let cy = center_y_px as i64;
        let res = self.resolution as i64;
        let min_x = (cx - radius_px as i64).max(0);
        let max_x = (cx + radius_px as i64).min(res - 1);
        let min_y = (cy - radius_px as i64).max(0);
        let max_y = (cy + radius_px as i64).min(res - 1);
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= r2 {
                    let idx = py as usize * self.resolution as usize + px as usize;
                    self.ids[idx] = archetype_id;
                }
            }
        }
        self
    }

    /// Phase 1.X-F.4.A test helper: paint a rectangular region with the given
    /// archetype ID. Builder-pattern; returns `Self` for chaining.
    ///
    /// Pixel coordinates inclusive on min, exclusive on max (standard
    /// half-open convention). Out-of-range coords clamped. Falloff field
    /// NOT recomputed; call [`Self::recompute_falloff`] after all paint
    /// operations.
    pub fn with_painted_rect(
        mut self,
        min_x_px: u32,
        min_y_px: u32,
        max_x_px: u32,
        max_y_px: u32,
        archetype_id: u8,
    ) -> Self {
        let res = self.resolution;
        let max_x = max_x_px.min(res);
        let max_y = max_y_px.min(res);
        let min_x = min_x_px.min(max_x);
        let min_y = min_y_px.min(max_y);
        for py in min_y..max_y {
            for px in min_x..max_x {
                let idx = py as usize * res as usize + px as usize;
                self.ids[idx] = archetype_id;
            }
        }
        self
    }
}

/// Phase 1.X-F.4.A: serde-serializable metadata for [`RegionalArchetypeMask`]
/// persistence. Pairs with raw uint8 binary files (`<base>.id.bin` and
/// `<base>.falloff.bin`) for full mask state.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RegionalArchetypeMaskMetadata {
    pub resolution: u32,
    pub world_extent_wu: f32,
    pub falloff_radius_pixels: u32,
    /// Format version. F.4.A ships version 1. Future format changes
    /// (multi-channel masks, different falloff encodings) bump this with
    /// explicit migration logic in [`RegionalArchetypeMask::load_from_files`].
    pub format_version: u32,
}

impl Default for RegionalArchetypeMaskMetadata {
    fn default() -> Self {
        Self {
            resolution: RegionalArchetypeMask::DEFAULT_RESOLUTION,
            world_extent_wu: RegionalArchetypeMask::DEFAULT_WORLD_EXTENT_WU,
            falloff_radius_pixels: RegionalArchetypeMask::DEFAULT_FALLOFF_RADIUS_PIXELS,
            format_version: 1,
        }
    }
}

impl RegionalArchetypeMaskMetadata {
    /// Construct metadata matching a given mask's parameters.
    pub fn from_mask(mask: &RegionalArchetypeMask) -> Self {
        Self {
            resolution: mask.resolution,
            world_extent_wu: mask.world_extent_wu,
            falloff_radius_pixels: mask.falloff_radius_pixels,
            format_version: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_archetypes::WorldArchetypeId;

    /// Unpainted constructor produces all-zero IDs and all-255 falloff.
    /// 1024×1024 = 1048576 bytes per channel.
    #[test]
    fn regional_mask_unpainted_default_state() {
        let mask = RegionalArchetypeMask::new_unpainted(
            RegionalArchetypeMask::DEFAULT_RESOLUTION,
            RegionalArchetypeMask::DEFAULT_WORLD_EXTENT_WU,
        );
        assert_eq!(mask.resolution, 1024);
        assert_eq!(mask.world_extent_wu, 11264.0);
        assert_eq!(mask.falloff_radius_pixels, 32);
        assert_eq!(mask.ids.len(), 1024 * 1024);
        assert_eq!(mask.falloff.len(), 1024 * 1024);
        assert!(mask.ids.iter().all(|&v| v == 0));
        assert!(mask.falloff.iter().all(|&v| v == 255));
    }

    /// `with_painted_circle` writes 1 to all pixels within radius 8 of
    /// (32, 32) in a 64×64 mask; leaves pixels outside as 0.
    #[test]
    fn regional_mask_painted_circle_writes_pixels() {
        let mask = RegionalArchetypeMask::new_unpainted(64, 100.0)
            .with_painted_circle(32, 32, 8, 1);

        // Center pixel painted.
        assert_eq!(mask.id_at(32, 32), 1);
        // Pixels within radius 8.
        assert_eq!(mask.id_at(40, 32), 1); // dist = 8
        assert_eq!(mask.id_at(32, 40), 1); // dist = 8
        // Pixels just outside radius 8 (dist² = 81 > 64).
        assert_eq!(mask.id_at(41, 32), 0); // dist = 9
        // Far pixel.
        assert_eq!(mask.id_at(0, 0), 0);
        assert_eq!(mask.id_at(63, 63), 0);
    }

    /// `with_painted_rect` writes archetype ID to pixels in `[min, max)`.
    #[test]
    fn regional_mask_painted_rect_writes_pixels() {
        let mask = RegionalArchetypeMask::new_unpainted(64, 100.0)
            .with_painted_rect(10, 10, 30, 30, 5);

        // Inside rect.
        assert_eq!(mask.id_at(10, 10), 5);
        assert_eq!(mask.id_at(15, 15), 5);
        assert_eq!(mask.id_at(29, 29), 5);
        // On exclusive max boundary.
        assert_eq!(mask.id_at(30, 30), 0);
        // Outside rect.
        assert_eq!(mask.id_at(9, 10), 0);
        assert_eq!(mask.id_at(10, 9), 0);
        assert_eq!(mask.id_at(40, 40), 0);
    }

    /// `RegionalArchetypeMaskMetadata` roundtrips through RON
    /// byte-identically.
    #[test]
    fn regional_mask_metadata_roundtrip_via_ron() {
        let original = RegionalArchetypeMaskMetadata {
            resolution: 512,
            world_extent_wu: 5632.0,
            falloff_radius_pixels: 16,
            format_version: 1,
        };
        let ron_string = ron::to_string(&original).expect("RON serialize");
        let roundtripped: RegionalArchetypeMaskMetadata =
            ron::from_str(&ron_string).expect("RON deserialize");
        assert_eq!(original, roundtripped);
    }

    /// Default `RegionalArchetypeMaskMetadata` ships format_version = 1
    /// per F.4.A invariant.
    #[test]
    fn regional_mask_metadata_format_version_is_one() {
        let meta = RegionalArchetypeMaskMetadata::default();
        assert_eq!(meta.format_version, 1);
    }

    /// Each of the 6 D.5 catalog archetypes roundtrips through
    /// `to_mask_id` → `from_mask_id` byte-identically.
    #[test]
    fn world_archetype_id_to_mask_id_roundtrip() {
        for &id in WorldArchetypeId::all() {
            let mask_id = id.to_mask_id();
            let recovered = WorldArchetypeId::from_mask_id(mask_id);
            assert_eq!(
                recovered,
                Some(id),
                "{:?} mask_id={} did not roundtrip",
                id,
                mask_id
            );
        }
    }

    /// `WorldArchetypeId::from_mask_id(0)` returns `None` (0 reserved for
    /// unpainted; sample-time fallback to Continental Temperate handled
    /// by sampler, not by enum lookup).
    #[test]
    fn world_archetype_id_unpainted_is_none() {
        assert!(WorldArchetypeId::from_mask_id(0).is_none());
    }

    /// `WorldArchetypeId::from_mask_id` for a reserved-but-unassigned ID
    /// returns `None`.
    #[test]
    fn world_archetype_id_unknown_is_none() {
        assert!(WorldArchetypeId::from_mask_id(200).is_none());
        assert!(WorldArchetypeId::from_mask_id(255).is_none());
    }
}
