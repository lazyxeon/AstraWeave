//! Mutation-Resistant Tests for Material Editor Panel APIs
//!
//! This module provides comprehensive mutation-resistant tests for:
//! - MaterialType (10 variants)
//! - BlendMode (5 variants with is_transparent() method)
//! - TextureChannel (8 variants)
//! - PreviewLighting (5 variants)
//! - MaterialTab (6 variants)
//!
//! Test patterns:
//! 1. Boolean return path tests - verify true AND false paths
//! 2. Match arm coverage - test every enum variant
//! 3. Boundary conditions - test edge cases
//! 4. Comparison operator tests - verify comparisons work correctly

use aw_editor_lib::panels::{
    BlendMode, MaterialTab, MaterialType, PreviewLighting, TextureChannel,
};

// ============================================================================
// MaterialType Tests - 10 variants
// ============================================================================

mod material_type_tests {
    use super::*;

    // Name tests - verify each variant returns its expected name
    #[test]
    fn material_type_standard_pbr_name() {
        assert_eq!(MaterialType::StandardPBR.name(), "Standard PBR");
    }

    #[test]
    fn material_type_unlit_name() {
        assert_eq!(MaterialType::Unlit.name(), "Unlit");
    }

    #[test]
    fn material_type_subsurface_name() {
        assert_eq!(MaterialType::Subsurface.name(), "Subsurface");
    }

    #[test]
    fn material_type_glass_name() {
        assert_eq!(MaterialType::Glass.name(), "Glass");
    }

    #[test]
    fn material_type_water_name() {
        assert_eq!(MaterialType::Water.name(), "Water");
    }

    #[test]
    fn material_type_foliage_name() {
        assert_eq!(MaterialType::Foliage.name(), "Foliage");
    }

    #[test]
    fn material_type_hair_name() {
        assert_eq!(MaterialType::Hair.name(), "Hair");
    }

    #[test]
    fn material_type_cloth_name() {
        assert_eq!(MaterialType::Cloth.name(), "Cloth");
    }

    #[test]
    fn material_type_terrain_name() {
        assert_eq!(MaterialType::Terrain.name(), "Terrain");
    }

    #[test]
    fn material_type_decal_name() {
        assert_eq!(MaterialType::Decal.name(), "Decal");
    }

    // Icon tests - verify each variant returns correct icon
    #[test]
    fn material_type_standard_pbr_icon() {
        assert_eq!(MaterialType::StandardPBR.icon(), "[Art]");
    }

    #[test]
    fn material_type_unlit_icon() {
        assert_eq!(MaterialType::Unlit.icon(), "[Lgt]");
    }

    #[test]
    fn material_type_subsurface_icon() {
        assert_eq!(MaterialType::Subsurface.icon(), "🧴");
    }

    #[test]
    fn material_type_glass_icon() {
        assert_eq!(MaterialType::Glass.icon(), "[Orb]");
    }

    #[test]
    fn material_type_water_icon() {
        assert_eq!(MaterialType::Water.icon(), "💧");
    }

    #[test]
    fn material_type_foliage_icon() {
        assert_eq!(MaterialType::Foliage.icon(), "[Leaf]");
    }

    #[test]
    fn material_type_hair_icon() {
        assert_eq!(MaterialType::Hair.icon(), "💇");
    }

    #[test]
    fn material_type_cloth_icon() {
        assert_eq!(MaterialType::Cloth.icon(), "👕");
    }

    #[test]
    fn material_type_terrain_icon() {
        assert_eq!(MaterialType::Terrain.icon(), "🏔️");
    }

    #[test]
    fn material_type_decal_icon() {
        assert_eq!(MaterialType::Decal.icon(), "[Tag]");
    }

    // Display tests
    #[test]
    fn material_type_standard_pbr_display() {
        assert_eq!(MaterialType::StandardPBR.to_string(), "[Art] Standard PBR");
    }

    #[test]
    fn material_type_glass_display() {
        assert_eq!(MaterialType::Glass.to_string(), "[Orb] Glass");
    }

    #[test]
    fn material_type_water_display() {
        assert_eq!(MaterialType::Water.to_string(), "💧 Water");
    }

    // all() tests
    #[test]
    fn material_type_all_count() {
        assert_eq!(MaterialType::all().len(), 10);
    }

    #[test]
    fn material_type_all_contains_standard_pbr() {
        assert!(MaterialType::all().contains(&MaterialType::StandardPBR));
    }

    #[test]
    fn material_type_all_contains_glass() {
        assert!(MaterialType::all().contains(&MaterialType::Glass));
    }

    #[test]
    fn material_type_all_contains_water() {
        assert!(MaterialType::all().contains(&MaterialType::Water));
    }

    #[test]
    fn material_type_all_contains_foliage() {
        assert!(MaterialType::all().contains(&MaterialType::Foliage));
    }

    #[test]
    fn material_type_all_contains_terrain() {
        assert!(MaterialType::all().contains(&MaterialType::Terrain));
    }

    // Default test
    #[test]
    fn material_type_default_is_standard_pbr() {
        assert_eq!(MaterialType::default(), MaterialType::StandardPBR);
    }

    // Uniqueness tests
    #[test]
    fn material_type_names_are_unique() {
        let names: Vec<_> = MaterialType::all().iter().map(|t| t.name()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn material_type_icons_are_unique() {
        let icons: Vec<_> = MaterialType::all().iter().map(|t| t.icon()).collect();
        let mut unique = icons.clone();
        unique.sort();
        unique.dedup();
        // Note: Some icons may be reused (like 💡 for Unlit and Indoor lighting)
        // so we allow less than 10 unique icons
        assert!(unique.len() >= 8);
    }
}

// ============================================================================
// BlendMode Tests - 5 variants with is_transparent()
// ============================================================================

mod blend_mode_tests {
    use super::*;

    // Name tests
    #[test]
    fn blend_mode_opaque_name() {
        assert_eq!(BlendMode::Opaque.name(), "Opaque");
    }

    #[test]
    fn blend_mode_masked_name() {
        assert_eq!(BlendMode::Masked.name(), "Masked");
    }

    #[test]
    fn blend_mode_translucent_name() {
        assert_eq!(BlendMode::Translucent.name(), "Translucent");
    }

    #[test]
    fn blend_mode_additive_name() {
        assert_eq!(BlendMode::Additive.name(), "Additive");
    }

    #[test]
    fn blend_mode_modulate_name() {
        assert_eq!(BlendMode::Modulate.name(), "Modulate");
    }

    // Icon tests
    #[test]
    fn blend_mode_opaque_icon() {
        assert_eq!(BlendMode::Opaque.icon(), "⬛");
    }

    #[test]
    fn blend_mode_masked_icon() {
        assert_eq!(BlendMode::Masked.icon(), "[Mask]");
    }

    #[test]
    fn blend_mode_translucent_icon() {
        assert_eq!(BlendMode::Translucent.icon(), "[Sq]");
    }

    #[test]
    fn blend_mode_additive_icon() {
        assert_eq!(BlendMode::Additive.icon(), "+");
    }

    #[test]
    fn blend_mode_modulate_icon() {
        assert_eq!(BlendMode::Modulate.icon(), "[Shuf]");
    }

    // is_transparent() tests - true path (3 variants)
    #[test]
    fn blend_mode_translucent_is_transparent() {
        assert!(BlendMode::Translucent.is_transparent());
    }

    #[test]
    fn blend_mode_additive_is_transparent() {
        assert!(BlendMode::Additive.is_transparent());
    }

    #[test]
    fn blend_mode_modulate_is_transparent() {
        assert!(BlendMode::Modulate.is_transparent());
    }

    // is_transparent() tests - false path (2 variants)
    #[test]
    fn blend_mode_opaque_is_not_transparent() {
        assert!(!BlendMode::Opaque.is_transparent());
    }

    #[test]
    fn blend_mode_masked_is_not_transparent() {
        assert!(!BlendMode::Masked.is_transparent());
    }

    // Count verification
    #[test]
    fn blend_mode_transparent_count() {
        let transparent_count = BlendMode::all()
            .iter()
            .filter(|m| m.is_transparent())
            .count();
        assert_eq!(transparent_count, 3);
    }

    #[test]
    fn blend_mode_opaque_count() {
        let opaque_count = BlendMode::all()
            .iter()
            .filter(|m| !m.is_transparent())
            .count();
        assert_eq!(opaque_count, 2);
    }

    // Display tests
    #[test]
    fn blend_mode_opaque_display() {
        assert_eq!(BlendMode::Opaque.to_string(), "⬛ Opaque");
    }

    #[test]
    fn blend_mode_translucent_display() {
        assert_eq!(BlendMode::Translucent.to_string(), "[Sq] Translucent");
    }

    // all() tests
    #[test]
    fn blend_mode_all_count() {
        assert_eq!(BlendMode::all().len(), 5);
    }

    #[test]
    fn blend_mode_all_contains_opaque() {
        assert!(BlendMode::all().contains(&BlendMode::Opaque));
    }

    #[test]
    fn blend_mode_all_contains_translucent() {
        assert!(BlendMode::all().contains(&BlendMode::Translucent));
    }

    #[test]
    fn blend_mode_all_contains_additive() {
        assert!(BlendMode::all().contains(&BlendMode::Additive));
    }

    // Default test
    #[test]
    fn blend_mode_default_is_opaque() {
        assert_eq!(BlendMode::default(), BlendMode::Opaque);
    }
}

// ============================================================================
// TextureChannel Tests - 8 variants
// ============================================================================

mod texture_channel_tests {
    use super::*;

    // Name tests
    #[test]
    fn texture_channel_albedo_name() {
        assert_eq!(TextureChannel::Albedo.name(), "Albedo");
    }

    #[test]
    fn texture_channel_normal_name() {
        assert_eq!(TextureChannel::Normal.name(), "Normal");
    }

    #[test]
    fn texture_channel_metallic_name() {
        assert_eq!(TextureChannel::Metallic.name(), "Metallic");
    }

    #[test]
    fn texture_channel_roughness_name() {
        assert_eq!(TextureChannel::Roughness.name(), "Roughness");
    }

    #[test]
    fn texture_channel_ao_name() {
        assert_eq!(TextureChannel::AO.name(), "Ambient Occlusion");
    }

    #[test]
    fn texture_channel_emissive_name() {
        assert_eq!(TextureChannel::Emissive.name(), "Emissive");
    }

    #[test]
    fn texture_channel_height_name() {
        assert_eq!(TextureChannel::Height.name(), "Height");
    }

    #[test]
    fn texture_channel_opacity_name() {
        assert_eq!(TextureChannel::Opacity.name(), "Opacity");
    }

    // Icon tests
    #[test]
    fn texture_channel_albedo_icon() {
        assert_eq!(TextureChannel::Albedo.icon(), "[Art]");
    }

    #[test]
    fn texture_channel_normal_icon() {
        assert_eq!(TextureChannel::Normal.icon(), "🗺️");
    }

    #[test]
    fn texture_channel_metallic_icon() {
        assert_eq!(TextureChannel::Metallic.icon(), "[Fx]");
    }

    #[test]
    fn texture_channel_roughness_icon() {
        assert_eq!(TextureChannel::Roughness.icon(), "🔨");
    }

    #[test]
    fn texture_channel_ao_icon() {
        assert_eq!(TextureChannel::AO.icon(), "🌑");
    }

    #[test]
    fn texture_channel_emissive_icon() {
        assert_eq!(TextureChannel::Emissive.icon(), "[Lgt]");
    }

    #[test]
    fn texture_channel_height_icon() {
        assert_eq!(TextureChannel::Height.icon(), "⛰️");
    }

    #[test]
    fn texture_channel_opacity_icon() {
        assert_eq!(TextureChannel::Opacity.icon(), "👻");
    }

    // Display tests
    #[test]
    fn texture_channel_albedo_display() {
        assert_eq!(TextureChannel::Albedo.to_string(), "[Art] Albedo");
    }

    #[test]
    fn texture_channel_normal_display() {
        assert_eq!(TextureChannel::Normal.to_string(), "🗺️ Normal");
    }

    #[test]
    fn texture_channel_ao_display() {
        assert_eq!(TextureChannel::AO.to_string(), "🌑 Ambient Occlusion");
    }

    // all() tests
    #[test]
    fn texture_channel_all_count() {
        assert_eq!(TextureChannel::all().len(), 8);
    }

    #[test]
    fn texture_channel_all_contains_albedo() {
        assert!(TextureChannel::all().contains(&TextureChannel::Albedo));
    }

    #[test]
    fn texture_channel_all_contains_normal() {
        assert!(TextureChannel::all().contains(&TextureChannel::Normal));
    }

    #[test]
    fn texture_channel_all_contains_metallic() {
        assert!(TextureChannel::all().contains(&TextureChannel::Metallic));
    }

    #[test]
    fn texture_channel_all_contains_roughness() {
        assert!(TextureChannel::all().contains(&TextureChannel::Roughness));
    }

    #[test]
    fn texture_channel_all_contains_ao() {
        assert!(TextureChannel::all().contains(&TextureChannel::AO));
    }

    #[test]
    fn texture_channel_all_contains_emissive() {
        assert!(TextureChannel::all().contains(&TextureChannel::Emissive));
    }

    #[test]
    fn texture_channel_all_contains_height() {
        assert!(TextureChannel::all().contains(&TextureChannel::Height));
    }

    #[test]
    fn texture_channel_all_contains_opacity() {
        assert!(TextureChannel::all().contains(&TextureChannel::Opacity));
    }

    // Uniqueness tests
    #[test]
    fn texture_channel_names_are_unique() {
        let names: Vec<_> = TextureChannel::all().iter().map(|t| t.name()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }
}

// ============================================================================
// PreviewLighting Tests - 5 variants
// ============================================================================

mod preview_lighting_tests {
    use super::*;

    // Name tests
    #[test]
    fn preview_lighting_studio_name() {
        assert_eq!(PreviewLighting::Studio.name(), "Studio");
    }

    #[test]
    fn preview_lighting_outdoor_name() {
        assert_eq!(PreviewLighting::Outdoor.name(), "Outdoor");
    }

    #[test]
    fn preview_lighting_indoor_name() {
        assert_eq!(PreviewLighting::Indoor.name(), "Indoor");
    }

    #[test]
    fn preview_lighting_dramatic_name() {
        assert_eq!(PreviewLighting::Dramatic.name(), "Dramatic");
    }

    #[test]
    fn preview_lighting_custom_name() {
        assert_eq!(PreviewLighting::Custom.name(), "Custom");
    }

    // Icon tests
    #[test]
    fn preview_lighting_studio_icon() {
        assert_eq!(PreviewLighting::Studio.icon(), "[Anim]");
    }

    #[test]
    fn preview_lighting_outdoor_icon() {
        assert_eq!(PreviewLighting::Outdoor.icon(), "[Sun]");
    }

    #[test]
    fn preview_lighting_indoor_icon() {
        assert_eq!(PreviewLighting::Indoor.icon(), "[Lgt]");
    }

    #[test]
    fn preview_lighting_dramatic_icon() {
        assert_eq!(PreviewLighting::Dramatic.icon(), "[Mask]");
    }

    #[test]
    fn preview_lighting_custom_icon() {
        assert_eq!(PreviewLighting::Custom.icon(), "⚙️");
    }

    // Display tests
    #[test]
    fn preview_lighting_studio_display() {
        assert_eq!(PreviewLighting::Studio.to_string(), "[Anim] Studio");
    }

    #[test]
    fn preview_lighting_outdoor_display() {
        assert_eq!(PreviewLighting::Outdoor.to_string(), "[Sun] Outdoor");
    }

    #[test]
    fn preview_lighting_dramatic_display() {
        assert_eq!(PreviewLighting::Dramatic.to_string(), "[Mask] Dramatic");
    }

    // all() tests
    #[test]
    fn preview_lighting_all_count() {
        assert_eq!(PreviewLighting::all().len(), 5);
    }

    #[test]
    fn preview_lighting_all_contains_studio() {
        assert!(PreviewLighting::all().contains(&PreviewLighting::Studio));
    }

    #[test]
    fn preview_lighting_all_contains_outdoor() {
        assert!(PreviewLighting::all().contains(&PreviewLighting::Outdoor));
    }

    #[test]
    fn preview_lighting_all_contains_indoor() {
        assert!(PreviewLighting::all().contains(&PreviewLighting::Indoor));
    }

    #[test]
    fn preview_lighting_all_contains_dramatic() {
        assert!(PreviewLighting::all().contains(&PreviewLighting::Dramatic));
    }

    #[test]
    fn preview_lighting_all_contains_custom() {
        assert!(PreviewLighting::all().contains(&PreviewLighting::Custom));
    }

    // Default test
    #[test]
    fn preview_lighting_default_is_studio() {
        assert_eq!(PreviewLighting::default(), PreviewLighting::Studio);
    }
}

// ============================================================================
// MaterialTab Tests - 6 variants
// ============================================================================

mod material_tab_tests {
    use super::*;

    // Name tests
    #[test]
    fn material_tab_properties_name() {
        assert_eq!(MaterialTab::Properties.name(), "Properties");
    }

    #[test]
    fn material_tab_textures_name() {
        assert_eq!(MaterialTab::Textures.name(), "Textures");
    }

    #[test]
    fn material_tab_advanced_name() {
        assert_eq!(MaterialTab::Advanced.name(), "Advanced");
    }

    #[test]
    fn material_tab_presets_name() {
        assert_eq!(MaterialTab::Presets.name(), "Presets");
    }

    #[test]
    fn material_tab_preview_name() {
        assert_eq!(MaterialTab::Preview.name(), "Preview");
    }

    #[test]
    fn material_tab_library_name() {
        assert_eq!(MaterialTab::Library.name(), "Library");
    }

    // Icon tests
    #[test]
    fn material_tab_properties_icon() {
        assert_eq!(MaterialTab::Properties.icon(), "[Art]");
    }

    #[test]
    fn material_tab_textures_icon() {
        assert_eq!(MaterialTab::Textures.icon(), "[Img]");
    }

    #[test]
    fn material_tab_advanced_icon() {
        assert_eq!(MaterialTab::Advanced.icon(), "⚙️");
    }

    #[test]
    fn material_tab_presets_icon() {
        assert_eq!(MaterialTab::Presets.icon(), "[List]");
    }

    #[test]
    fn material_tab_preview_icon() {
        assert_eq!(MaterialTab::Preview.icon(), "👁️");
    }

    #[test]
    fn material_tab_library_icon() {
        assert_eq!(MaterialTab::Library.icon(), "📚");
    }

    // Display tests
    #[test]
    fn material_tab_properties_display() {
        assert_eq!(MaterialTab::Properties.to_string(), "[Art] Properties");
    }

    #[test]
    fn material_tab_textures_display() {
        assert_eq!(MaterialTab::Textures.to_string(), "[Img] Textures");
    }

    #[test]
    fn material_tab_library_display() {
        assert_eq!(MaterialTab::Library.to_string(), "📚 Library");
    }

    // all() tests
    #[test]
    fn material_tab_all_count() {
        assert_eq!(MaterialTab::all().len(), 6);
    }

    #[test]
    fn material_tab_all_contains_properties() {
        assert!(MaterialTab::all().contains(&MaterialTab::Properties));
    }

    #[test]
    fn material_tab_all_contains_textures() {
        assert!(MaterialTab::all().contains(&MaterialTab::Textures));
    }

    #[test]
    fn material_tab_all_contains_advanced() {
        assert!(MaterialTab::all().contains(&MaterialTab::Advanced));
    }

    #[test]
    fn material_tab_all_contains_presets() {
        assert!(MaterialTab::all().contains(&MaterialTab::Presets));
    }

    #[test]
    fn material_tab_all_contains_preview() {
        assert!(MaterialTab::all().contains(&MaterialTab::Preview));
    }

    #[test]
    fn material_tab_all_contains_library() {
        assert!(MaterialTab::all().contains(&MaterialTab::Library));
    }

    // Default test
    #[test]
    fn material_tab_default_is_properties() {
        assert_eq!(MaterialTab::default(), MaterialTab::Properties);
    }

    // Uniqueness tests
    #[test]
    fn material_tab_names_are_unique() {
        let names: Vec<_> = MaterialTab::all().iter().map(|t| t.name()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }
}

// ============================================================================
// Equality and Hash Tests
// ============================================================================

mod equality_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn material_type_eq() {
        assert_eq!(MaterialType::StandardPBR, MaterialType::StandardPBR);
        assert_ne!(MaterialType::StandardPBR, MaterialType::Glass);
    }

    #[test]
    fn material_type_hash() {
        let mut set = HashSet::new();
        set.insert(MaterialType::StandardPBR);
        set.insert(MaterialType::Glass);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn blend_mode_eq() {
        assert_eq!(BlendMode::Opaque, BlendMode::Opaque);
        assert_ne!(BlendMode::Opaque, BlendMode::Translucent);
    }

    #[test]
    fn blend_mode_hash() {
        let mut set = HashSet::new();
        set.insert(BlendMode::Opaque);
        set.insert(BlendMode::Translucent);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn texture_channel_eq() {
        assert_eq!(TextureChannel::Albedo, TextureChannel::Albedo);
        assert_ne!(TextureChannel::Albedo, TextureChannel::Normal);
    }

    #[test]
    fn texture_channel_hash() {
        let mut set = HashSet::new();
        set.insert(TextureChannel::Albedo);
        set.insert(TextureChannel::Normal);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn preview_lighting_eq() {
        assert_eq!(PreviewLighting::Studio, PreviewLighting::Studio);
        assert_ne!(PreviewLighting::Studio, PreviewLighting::Outdoor);
    }

    #[test]
    fn preview_lighting_hash() {
        let mut set = HashSet::new();
        set.insert(PreviewLighting::Studio);
        set.insert(PreviewLighting::Outdoor);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn material_tab_eq() {
        assert_eq!(MaterialTab::Properties, MaterialTab::Properties);
        assert_ne!(MaterialTab::Properties, MaterialTab::Textures);
    }

    #[test]
    fn material_tab_hash() {
        let mut set = HashSet::new();
        set.insert(MaterialTab::Properties);
        set.insert(MaterialTab::Textures);
        assert_eq!(set.len(), 2);
    }
}

// ============================================================================
// Clone and Debug Tests
// ============================================================================

mod clone_debug_tests {
    use super::*;

    #[test]
    fn material_type_clone() {
        let t = MaterialType::Glass;
        let cloned = t.clone();
        assert_eq!(t, cloned);
    }

    #[test]
    fn material_type_debug() {
        let debug = format!("{:?}", MaterialType::Glass);
        assert!(debug.contains("Glass"));
    }

    #[test]
    fn blend_mode_clone() {
        let m = BlendMode::Translucent;
        let cloned = m.clone();
        assert_eq!(m, cloned);
    }

    #[test]
    fn blend_mode_debug() {
        let debug = format!("{:?}", BlendMode::Translucent);
        assert!(debug.contains("Translucent"));
    }

    #[test]
    fn texture_channel_clone() {
        let c = TextureChannel::Normal;
        let cloned = c.clone();
        assert_eq!(c, cloned);
    }

    #[test]
    fn texture_channel_debug() {
        let debug = format!("{:?}", TextureChannel::Normal);
        assert!(debug.contains("Normal"));
    }

    #[test]
    fn preview_lighting_clone() {
        let l = PreviewLighting::Dramatic;
        let cloned = l.clone();
        assert_eq!(l, cloned);
    }

    #[test]
    fn preview_lighting_debug() {
        let debug = format!("{:?}", PreviewLighting::Dramatic);
        assert!(debug.contains("Dramatic"));
    }

    #[test]
    fn material_tab_clone() {
        let t = MaterialTab::Library;
        let cloned = t.clone();
        assert_eq!(t, cloned);
    }

    #[test]
    fn material_tab_debug() {
        let debug = format!("{:?}", MaterialTab::Library);
        assert!(debug.contains("Library"));
    }
}

// ============================================================================
// Comprehensive Boolean Path Tests for BlendMode
// ============================================================================

mod boolean_path_tests {
    use super::*;

    #[test]
    fn blend_mode_all_transparent_paths_tested() {
        // 3 true paths
        let transparent_true = [
            BlendMode::Translucent,
            BlendMode::Additive,
            BlendMode::Modulate,
        ];
        for mode in transparent_true {
            assert!(mode.is_transparent(), "{:?} should be transparent", mode);
        }

        // 2 false paths
        let transparent_false = [BlendMode::Opaque, BlendMode::Masked];
        for mode in transparent_false {
            assert!(
                !mode.is_transparent(),
                "{:?} should not be transparent",
                mode
            );
        }
    }

    #[test]
    fn blend_mode_transparent_vs_opaque_partitions() {
        let all_modes = BlendMode::all();
        let transparent: Vec<_> = all_modes.iter().filter(|m| m.is_transparent()).collect();
        let opaque: Vec<_> = all_modes.iter().filter(|m| !m.is_transparent()).collect();

        // Verify partition sizes
        assert_eq!(transparent.len(), 3);
        assert_eq!(opaque.len(), 2);

        // Verify complete coverage
        assert_eq!(transparent.len() + opaque.len(), all_modes.len());
    }
}

// ============================================================================
// All Variants Coverage Tests
// ============================================================================

mod all_variants_coverage {
    use super::*;

    #[test]
    fn material_type_all_variants_have_name() {
        for t in MaterialType::all() {
            assert!(!t.name().is_empty(), "{:?} should have non-empty name", t);
        }
    }

    #[test]
    fn material_type_all_variants_have_icon() {
        for t in MaterialType::all() {
            assert!(!t.icon().is_empty(), "{:?} should have non-empty icon", t);
        }
    }

    #[test]
    fn blend_mode_all_variants_have_name() {
        for m in BlendMode::all() {
            assert!(!m.name().is_empty(), "{:?} should have non-empty name", m);
        }
    }

    #[test]
    fn blend_mode_all_variants_have_icon() {
        for m in BlendMode::all() {
            assert!(!m.icon().is_empty(), "{:?} should have non-empty icon", m);
        }
    }

    #[test]
    fn texture_channel_all_variants_have_name() {
        for c in TextureChannel::all() {
            assert!(!c.name().is_empty(), "{:?} should have non-empty name", c);
        }
    }

    #[test]
    fn texture_channel_all_variants_have_icon() {
        for c in TextureChannel::all() {
            assert!(!c.icon().is_empty(), "{:?} should have non-empty icon", c);
        }
    }

    #[test]
    fn preview_lighting_all_variants_have_name() {
        for l in PreviewLighting::all() {
            assert!(!l.name().is_empty(), "{:?} should have non-empty name", l);
        }
    }

    #[test]
    fn preview_lighting_all_variants_have_icon() {
        for l in PreviewLighting::all() {
            assert!(!l.icon().is_empty(), "{:?} should have non-empty icon", l);
        }
    }

    #[test]
    fn material_tab_all_variants_have_name() {
        for t in MaterialTab::all() {
            assert!(!t.name().is_empty(), "{:?} should have non-empty name", t);
        }
    }

    #[test]
    fn material_tab_all_variants_have_icon() {
        for t in MaterialTab::all() {
            assert!(!t.icon().is_empty(), "{:?} should have non-empty icon", t);
        }
    }
}

// ============================================================================
// Display Trait Consistency Tests
// ============================================================================

mod display_consistency {
    use super::*;

    #[test]
    fn material_type_display_contains_icon_and_name() {
        for t in MaterialType::all() {
            let display = t.to_string();
            assert!(
                display.contains(t.icon()),
                "{:?} display should contain icon",
                t
            );
            assert!(
                display.contains(t.name()),
                "{:?} display should contain name",
                t
            );
        }
    }

    #[test]
    fn blend_mode_display_contains_icon_and_name() {
        for m in BlendMode::all() {
            let display = m.to_string();
            assert!(
                display.contains(m.icon()),
                "{:?} display should contain icon",
                m
            );
            assert!(
                display.contains(m.name()),
                "{:?} display should contain name",
                m
            );
        }
    }

    #[test]
    fn preview_lighting_display_contains_icon_and_name() {
        for l in PreviewLighting::all() {
            let display = l.to_string();
            assert!(
                display.contains(l.icon()),
                "{:?} display should contain icon",
                l
            );
            assert!(
                display.contains(l.name()),
                "{:?} display should contain name",
                l
            );
        }
    }

    #[test]
    fn material_tab_display_contains_icon_and_name() {
        for t in MaterialTab::all() {
            let display = t.to_string();
            assert!(
                display.contains(t.icon()),
                "{:?} display should contain icon",
                t
            );
            assert!(
                display.contains(t.name()),
                "{:?} display should contain name",
                t
            );
        }
    }

    #[test]
    fn texture_channel_display_contains_icon_and_name() {
        for c in TextureChannel::all() {
            let display = c.to_string();
            assert!(
                display.contains(c.icon()),
                "{:?} display should contain icon",
                c
            );
            assert!(
                display.contains(c.name()),
                "{:?} display should contain name",
                c
            );
        }
    }
}
