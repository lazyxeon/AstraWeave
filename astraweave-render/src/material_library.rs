//! Canonical terrain material library — single source of truth for material
//! identity at the UI/renderer boundary.
//!
//! Real-Fix.D 2026-05-08: established to resolve Defect Class 4 (Material
//! Library Coverage Gap; FOURTH §7.7 instance per Andrew-gate decision (h)
//! Option D-2). Both UI panel material rendering AND renderer texture-array
//! allocation AND splat-builder cap derive from this module — eliminating the
//! prior three-tier capacity boundary mismatch (UI=22; splat-build=8;
//! renderer=8).
//!
//! # Library shape
//!
//! 21 named materials + 11 reserved slots = 32 total layers (post-2026-05-15
//! Real-Fix.D follow-up cleanup: `default` removed as biome-unreferenced; see
//! revision history below). The canonical [`MAX_TERRAIN_LAYERS`] = 32 capacity
//! provides headroom: future content can occupy IDs 21..32 without reopening
//! the §7.7 wrapped-component resource identity trap.
//!
//! # Layer ID stability
//!
//! Layer IDs 0..7 preserve the historical biome→layer mapping from the
//! pre-Real-Fix.D 8-layer splat pipeline (grass=0, sand=1, forest_floor=2,
//! mountain_rock=3, snow=4, mud=5, wood_planks=6, stone=7). IDs 8..20 add the
//! materials that were previously dropped by the splat builder cap (Round-8
//! evidence; minus the biome-unreferenced `default` slot removed 2026-05-15
//! in the Real-Fix.D follow-up cleanup — IDs 12..20 are gravel, ice,
//! metal_rusted, moss, plaster, rock_lichen, roof_tile, tree_bark,
//! tree_leaves, renumbered from former IDs 13..21). IDs 21..31 are reserved
//! for future expansion.

/// Maximum number of terrain material layers supported by the splat pipeline.
///
/// 32 = 22 named materials + 10 reserved slots. The WGSL shaders
/// (`pbr_terrain.wgsl`, `pbr_terrain_forward.wgsl`) declare matching
/// `array<TerrainLayer, 32>` and 8 splat textures (32 channels @ RGBA each).
/// The Rust GPU mirrors (`TerrainMaterialGpu`) match byte-for-byte.
pub const MAX_TERRAIN_LAYERS: u32 = 32;

/// Number of RGBA8 splat textures used to encode per-vertex layer weights.
///
/// Each texture carries 4 channels of layer weights:
/// `splats[i]` channel R..A → layers `i*4+0 .. i*4+3`.
/// `MAX_TERRAIN_LAYERS / 4 = 32 / 4 = 8`.
pub const NUM_SPLAT_MAPS: usize = (MAX_TERRAIN_LAYERS as usize) / 4;

/// A single material entry in the canonical terrain material library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Material {
    /// Canonical layer ID (0..MAX_TERRAIN_LAYERS).
    pub id: u32,
    /// Asset path stem; texture loader expects `assets/materials/{name}.png`,
    /// `{name}_n.png`, `{name}_mra.png`.
    pub name: &'static str,
    /// Human-readable label for UI display.
    pub display_name: &'static str,
}

/// Canonical terrain material library. Single source of truth for material
/// identity at the UI/renderer boundary.
///
/// 21 named materials (IDs 0..21) + 11 reserved slots (IDs 21..32). Reserved
/// slots produce empty texture array layers; UI hides them; splat builder
/// accepts material IDs up to [`MAX_TERRAIN_LAYERS`] but reserved IDs render
/// as the fallback (layer 0).
pub struct MaterialLibrary;

impl MaterialLibrary {
    /// All canonical named materials, in canonical layer-ID order.
    ///
    /// 2026-05-15 Real-Fix.D follow-up: `default` (formerly id=12) removed as
    /// biome-unreferenced. IDs 12..20 renumbered (gravel=12, ice=13,
    /// metal_rusted=14, moss=15, plaster=16, rock_lichen=17, roof_tile=18,
    /// tree_bark=19, tree_leaves=20). IDs 0..11 unchanged.
    const MATERIALS: [Material; 21] = [
        Material {
            id: 0,
            name: "grass",
            display_name: "Grass",
        },
        Material {
            id: 1,
            name: "sand",
            display_name: "Sand",
        },
        Material {
            id: 2,
            name: "forest_floor",
            display_name: "Forest Floor",
        },
        Material {
            id: 3,
            name: "mountain_rock",
            display_name: "Mountain Rock",
        },
        Material {
            id: 4,
            name: "snow",
            display_name: "Snow",
        },
        Material {
            id: 5,
            name: "mud",
            display_name: "Mud",
        },
        Material {
            id: 6,
            name: "wood_planks",
            display_name: "Wood Planks",
        },
        Material {
            id: 7,
            name: "stone",
            display_name: "Stone",
        },
        Material {
            id: 8,
            name: "rock_slate",
            display_name: "Rock Slate",
        },
        Material {
            id: 9,
            name: "dirt",
            display_name: "Dirt",
        },
        Material {
            id: 10,
            name: "cobblestone",
            display_name: "Cobblestone",
        },
        Material {
            id: 11,
            name: "cloth",
            display_name: "Cloth",
        },
        Material {
            id: 12,
            name: "gravel",
            display_name: "Gravel",
        },
        Material {
            id: 13,
            name: "ice",
            display_name: "Ice",
        },
        Material {
            id: 14,
            name: "metal_rusted",
            display_name: "Metal Rusted",
        },
        Material {
            id: 15,
            name: "moss",
            display_name: "Moss",
        },
        Material {
            id: 16,
            name: "plaster",
            display_name: "Plaster",
        },
        Material {
            id: 17,
            name: "rock_lichen",
            display_name: "Rock Lichen",
        },
        Material {
            id: 18,
            name: "roof_tile",
            display_name: "Roof Tile",
        },
        Material {
            id: 19,
            name: "tree_bark",
            display_name: "Tree Bark",
        },
        Material {
            id: 20,
            name: "tree_leaves",
            display_name: "Tree Leaves",
        },
    ];

    /// Total slot count (named + reserved). Equals [`MAX_TERRAIN_LAYERS`].
    pub const fn len() -> usize {
        MAX_TERRAIN_LAYERS as usize
    }

    /// Number of named (non-reserved) materials.
    pub const fn named_count() -> usize {
        Self::MATERIALS.len()
    }

    /// All canonical named materials in slot order.
    pub const fn named() -> &'static [Material] {
        &Self::MATERIALS
    }

    /// Look up a material by canonical ID. Returns `None` for reserved slots
    /// (id ≥ [`MaterialLibrary::named_count()`]) and out-of-range IDs.
    pub fn get(id: u32) -> Option<&'static Material> {
        let idx = id as usize;
        Self::MATERIALS.get(idx).filter(|m| m.id == id)
    }

    /// Look up a material name (asset path stem) by canonical ID.
    pub fn name(id: u32) -> Option<&'static str> {
        Self::get(id).map(|m| m.name)
    }

    /// Look up the display label for a canonical ID.
    pub fn display_name(id: u32) -> Option<&'static str> {
        Self::get(id).map(|m| m.display_name)
    }

    /// True if `id` is a named (non-reserved) layer.
    pub fn is_named(id: u32) -> bool {
        Self::get(id).is_some()
    }

    /// True if `id` is within the canonical capacity (named OR reserved).
    pub fn is_in_range(id: u32) -> bool {
        id < MAX_TERRAIN_LAYERS
    }
}

/// Backwards-compatible flat name slice. Mirrors the historical
/// `MATERIAL_NAMES: [&str; 22]` from the editor before Real-Fix.D.
///
/// Prefer [`MaterialLibrary::name`] / [`MaterialLibrary::named`] for new code.
pub const MATERIAL_NAMES: [&str; 21] = [
    "grass",
    "sand",
    "forest_floor",
    "mountain_rock",
    "snow",
    "mud",
    "wood_planks",
    "stone",
    "rock_slate",
    "dirt",
    "cobblestone",
    "cloth",
    "gravel",
    "ice",
    "metal_rusted",
    "moss",
    "plaster",
    "rock_lichen",
    "roof_tile",
    "tree_bark",
    "tree_leaves",
];

/// Backwards-compatible flat display-name slice.
///
/// Prefer [`MaterialLibrary::display_name`] for new code.
pub const MATERIAL_DISPLAY_NAMES: [&str; 21] = [
    "Grass",
    "Sand",
    "Forest Floor",
    "Mountain Rock",
    "Snow",
    "Mud",
    "Wood Planks",
    "Stone",
    "Rock Slate",
    "Dirt",
    "Cobblestone",
    "Cloth",
    "Gravel",
    "Ice",
    "Metal Rusted",
    "Moss",
    "Plaster",
    "Rock Lichen",
    "Roof Tile",
    "Tree Bark",
    "Tree Leaves",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_capacity_is_32() {
        assert_eq!(MAX_TERRAIN_LAYERS, 32);
        assert_eq!(MaterialLibrary::len(), 32);
        assert_eq!(NUM_SPLAT_MAPS, 8);
    }

    #[test]
    fn library_has_21_named_materials() {
        // 2026-05-15 Real-Fix.D follow-up: `default` removed (biome-unreferenced).
        // Allocation shifted 22 named + 10 reserved -> 21 named + 11 reserved.
        // 32-slot MAX_TERRAIN_LAYERS capacity invariant preserved.
        assert_eq!(MaterialLibrary::named_count(), 21);
        assert_eq!(MaterialLibrary::named().len(), 21);
        assert_eq!(MATERIAL_NAMES.len(), 21);
        assert_eq!(MATERIAL_DISPLAY_NAMES.len(), 21);
    }

    #[test]
    fn library_ids_match_slot_order() {
        for (slot, material) in MaterialLibrary::named().iter().enumerate() {
            assert_eq!(material.id as usize, slot);
        }
    }

    #[test]
    fn flat_arrays_match_canonical_library() {
        for (i, material) in MaterialLibrary::named().iter().enumerate() {
            assert_eq!(material.name, MATERIAL_NAMES[i]);
            assert_eq!(material.display_name, MATERIAL_DISPLAY_NAMES[i]);
        }
    }

    #[test]
    fn get_returns_named_for_valid_ids() {
        for id in 0..21 {
            assert!(MaterialLibrary::get(id).is_some());
            assert!(MaterialLibrary::is_named(id));
        }
    }

    #[test]
    fn get_returns_none_for_reserved_ids() {
        for id in 21..32 {
            assert!(MaterialLibrary::get(id).is_none());
            assert!(!MaterialLibrary::is_named(id));
            assert!(MaterialLibrary::is_in_range(id));
        }
    }

    #[test]
    fn get_returns_none_for_out_of_range_ids() {
        assert!(MaterialLibrary::get(32).is_none());
        assert!(MaterialLibrary::get(100).is_none());
        assert!(MaterialLibrary::get(u32::MAX).is_none());
        assert!(!MaterialLibrary::is_in_range(32));
    }

    #[test]
    fn name_lookup_examples() {
        assert_eq!(MaterialLibrary::name(0), Some("grass"));
        assert_eq!(MaterialLibrary::name(8), Some("rock_slate"));
        // 2026-05-15: tree_leaves renumbered 21 -> 20 after `default` removal.
        assert_eq!(MaterialLibrary::name(20), Some("tree_leaves"));
        assert_eq!(MaterialLibrary::name(21), None);
    }

    #[test]
    fn display_name_lookup_examples() {
        assert_eq!(MaterialLibrary::display_name(0), Some("Grass"));
        assert_eq!(MaterialLibrary::display_name(8), Some("Rock Slate"));
        assert_eq!(MaterialLibrary::display_name(20), Some("Tree Leaves"));
        assert_eq!(MaterialLibrary::display_name(21), None);
    }
}
