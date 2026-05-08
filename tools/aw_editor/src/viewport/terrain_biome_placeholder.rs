//! Phase 1 placeholder biome materials (Option D, Terrain Material System
//! Campaign). Flat-color albedo swatches + shared flat-normal + shared
//! neutral-ORM maps used to populate `TerrainMaterialManager`'s 8 layer
//! arrays when the forward pipeline is first activated.
//!
//! Phase 3 replaces these with real material textures loaded from
//! `assets/materials/{biome}/` TOML configs. Until then the editor
//! renders terrain with these 8 colors, one per biome, so that splat
//! blending is visibly working without requiring production assets.
//!
//! The color ordering **must match** the editor's `TerrainVertex`
//! material layer indices (post-Real-Fix.C unification, 2026-05-08):
//!   layers 0-3 → [Grassland, Desert, Forest, Mountain]  (splat_0 RGBA)
//!   layers 4-7 → [Tundra,    Swamp,  Beach,  River]     (splat_1 RGBA)
//!
//! Pre-Real-Fix.C, this layout matched `biome_weights_0`/`biome_weights_1`
//! vertex fields. Those fields were eliminated by Real-Fix.C (Option C
//! attribute-set unification); the same layer-index ordering is preserved
//! in `material_ids` (now the canonical source).
//!
//! Colors are chosen for obvious at-a-glance distinction under sun +
//! ambient lighting, avoiding fully saturated or fully bright values so
//! the PBR lighting model doesn't blow them out.

/// Placeholder albedo color per biome, sRGB8 (RGBA). Slot order matches
/// the editor's `material_ids` layer-index packing (layers 0-7).
pub const BIOME_PLACEHOLDER_COLORS_SRGB: [[u8; 4]; 8] = [
    [85, 140, 60, 255],    // 0 Grassland: muted green
    [200, 170, 110, 255],  // 1 Desert: sandy tan
    [50, 95, 45, 255],     // 2 Forest: dark green
    [110, 105, 100, 255],  // 3 Mountain: cool gray
    [220, 225, 230, 255],  // 4 Tundra: near-white pale blue
    [75, 95, 70, 255],     // 5 Swamp: murky green-brown
    [230, 210, 170, 255],  // 6 Beach: pale sand
    [80, 120, 165, 255],   // 7 River: muted blue
];

/// Albedo texture resolution used by all placeholder biome swatches.
/// Must match `TerrainMaterialConfig::default().albedo_resolution` (1024) —
/// the manager's `set_material` validates the payload size.
pub const PLACEHOLDER_ALBEDO_RES: u32 = 1024;

/// Normal / ORM placeholder resolution. Must match
/// `TerrainMaterialConfig::default().aux_resolution` (512).
pub const PLACEHOLDER_AUX_RES: u32 = 512;

/// Generate the 8 per-biome solid-color albedo buffers, RGBA8 at
/// `PLACEHOLDER_ALBEDO_RES × PLACEHOLDER_ALBEDO_RES`.
///
/// Returns one `Vec<u8>` per biome, in slot order. Each is
/// `PLACEHOLDER_ALBEDO_RES * PLACEHOLDER_ALBEDO_RES * 4` bytes.
pub fn generate_biome_placeholder_albedos() -> [Vec<u8>; 8] {
    let texel_count = (PLACEHOLDER_ALBEDO_RES as usize) * (PLACEHOLDER_ALBEDO_RES as usize);
    std::array::from_fn(|biome| {
        let color = BIOME_PLACEHOLDER_COLORS_SRGB[biome];
        let mut buf = Vec::with_capacity(texel_count * 4);
        for _ in 0..texel_count {
            buf.extend_from_slice(&color);
        }
        buf
    })
}

/// Generate a neutral-normal tangent-space map: every texel encodes
/// `(0, 0, 1)` in tangent space, stored as `(0.5, 0.5, 1.0)` in sRGB bytes.
/// Shared across all 8 biomes in Phase 1 — no per-biome normal detail.
/// Resolution is `PLACEHOLDER_AUX_RES × PLACEHOLDER_AUX_RES`, RGBA8.
pub fn generate_flat_normal_map() -> Vec<u8> {
    let texel_count = (PLACEHOLDER_AUX_RES as usize) * (PLACEHOLDER_AUX_RES as usize);
    let mut buf = Vec::with_capacity(texel_count * 4);
    // (128, 128, 255, 255) = tangent-space (0, 0, 1) after decoding.
    for _ in 0..texel_count {
        buf.extend_from_slice(&[128, 128, 255, 255]);
    }
    buf
}

/// Generate a neutral ORM map: `(occlusion=1.0, roughness=0.85,
/// metallic=0.0, alpha=1.0)` per texel. High roughness avoids mirror
/// reflections on placeholder surfaces; zero metallic keeps them dielectric
/// so the placeholder colors show their authored hue rather than reflecting
/// the scene. Shared across all 8 biomes. Resolution is
/// `PLACEHOLDER_AUX_RES × PLACEHOLDER_AUX_RES`, RGBA8.
pub fn generate_neutral_orm_map() -> Vec<u8> {
    let texel_count = (PLACEHOLDER_AUX_RES as usize) * (PLACEHOLDER_AUX_RES as usize);
    let mut buf = Vec::with_capacity(texel_count * 4);
    // R=255 (AO=1), G≈217 (roughness≈0.85 × 255), B=0 (metallic=0), A=255.
    for _ in 0..texel_count {
        buf.extend_from_slice(&[255, 217, 0, 255]);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_albedos_match_expected_size_and_color() {
        let albedos = generate_biome_placeholder_albedos();
        let expected_bytes =
            (PLACEHOLDER_ALBEDO_RES as usize) * (PLACEHOLDER_ALBEDO_RES as usize) * 4;

        for (biome, buf) in albedos.iter().enumerate() {
            assert_eq!(
                buf.len(),
                expected_bytes,
                "biome {biome} albedo size mismatch",
            );
            let expected_color = BIOME_PLACEHOLDER_COLORS_SRGB[biome];
            assert_eq!(
                &buf[..4],
                &expected_color,
                "biome {biome} first texel does not match expected color",
            );
            // Also verify a texel deep in the buffer (catches "only first
            // row populated" bugs).
            let deep_offset = expected_bytes / 2;
            assert_eq!(
                &buf[deep_offset..deep_offset + 4],
                &expected_color,
                "biome {biome} mid-buffer texel does not match expected color",
            );
        }
    }

    #[test]
    fn flat_normal_encodes_plus_z_in_tangent_space() {
        let buf = generate_flat_normal_map();
        assert_eq!(
            buf.len(),
            (PLACEHOLDER_AUX_RES as usize) * (PLACEHOLDER_AUX_RES as usize) * 4,
        );
        assert_eq!(&buf[..4], &[128, 128, 255, 255]);
    }

    #[test]
    fn neutral_orm_packs_ao_rough_metallic() {
        let buf = generate_neutral_orm_map();
        assert_eq!(
            buf.len(),
            (PLACEHOLDER_AUX_RES as usize) * (PLACEHOLDER_AUX_RES as usize) * 4,
        );
        // R = AO (full), G = roughness (high), B = metallic (zero)
        assert_eq!(&buf[..4], &[255, 217, 0, 255]);
    }

    #[test]
    fn biome_colors_are_pairwise_distinct() {
        // Sanity: placeholder colors must differ enough that biomes are
        // visually distinguishable. Pairwise L1 distance >= 50 (out of
        // max 3*255 = 765).
        let colors = BIOME_PLACEHOLDER_COLORS_SRGB;
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                let d = (colors[i][0] as i32 - colors[j][0] as i32).abs()
                    + (colors[i][1] as i32 - colors[j][1] as i32).abs()
                    + (colors[i][2] as i32 - colors[j][2] as i32).abs();
                assert!(
                    d >= 50,
                    "biomes {i} and {j} are too close: L1 distance = {d}",
                );
            }
        }
    }
}
