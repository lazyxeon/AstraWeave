//! Mutation-resistant tests for Asset Browser system
//!
//! These tests target boundary conditions, comparison operators, and boolean return paths
//! to achieve high mutation testing kill rates (90%+).

use aw_editor_lib::panels::asset_browser::{AssetCategory, AssetType};
use aw_editor_lib::panels::{AssetAction, TextureType};
use std::path::PathBuf;

// ============================================================================
// TEXTURE TYPE TESTS
// ============================================================================

mod texture_type_tests {
    use super::*;

    // Test all() returns correct count
    #[test]
    fn test_texture_type_all_count() {
        assert_eq!(TextureType::all().len(), 10);
    }

    // Test is_pbr_component - true for all except Unknown
    #[test]
    fn test_albedo_is_pbr_component() {
        assert!(TextureType::Albedo.is_pbr_component());
    }

    #[test]
    fn test_normal_is_pbr_component() {
        assert!(TextureType::Normal.is_pbr_component());
    }

    #[test]
    fn test_orm_is_pbr_component() {
        assert!(TextureType::ORM.is_pbr_component());
    }

    #[test]
    fn test_mra_is_pbr_component() {
        assert!(TextureType::MRA.is_pbr_component());
    }

    #[test]
    fn test_roughness_is_pbr_component() {
        assert!(TextureType::Roughness.is_pbr_component());
    }

    #[test]
    fn test_metallic_is_pbr_component() {
        assert!(TextureType::Metallic.is_pbr_component());
    }

    #[test]
    fn test_ao_is_pbr_component() {
        assert!(TextureType::AO.is_pbr_component());
    }

    #[test]
    fn test_emission_is_pbr_component() {
        assert!(TextureType::Emission.is_pbr_component());
    }

    #[test]
    fn test_height_is_pbr_component() {
        assert!(TextureType::Height.is_pbr_component());
    }

    #[test]
    fn test_unknown_is_not_pbr_component() {
        assert!(!TextureType::Unknown.is_pbr_component());
    }

    // Test is_packed - only ORM and MRA
    #[test]
    fn test_orm_is_packed() {
        assert!(TextureType::ORM.is_packed());
    }

    #[test]
    fn test_mra_is_packed() {
        assert!(TextureType::MRA.is_packed());
    }

    #[test]
    fn test_albedo_is_not_packed() {
        assert!(!TextureType::Albedo.is_packed());
    }

    #[test]
    fn test_normal_is_not_packed() {
        assert!(!TextureType::Normal.is_packed());
    }

    #[test]
    fn test_roughness_is_not_packed() {
        assert!(!TextureType::Roughness.is_packed());
    }

    #[test]
    fn test_metallic_is_not_packed() {
        assert!(!TextureType::Metallic.is_packed());
    }

    #[test]
    fn test_unknown_is_not_packed() {
        assert!(!TextureType::Unknown.is_packed());
    }

    // Test from_filename detection
    #[test]
    fn test_from_filename_normal_suffix_n() {
        assert_eq!(
            TextureType::from_filename("texture_n.png"),
            TextureType::Normal
        );
    }

    #[test]
    fn test_from_filename_normal_suffix_normal() {
        assert_eq!(
            TextureType::from_filename("texture_normal.png"),
            TextureType::Normal
        );
    }

    #[test]
    fn test_from_filename_normal_suffix_nrm() {
        assert_eq!(
            TextureType::from_filename("texture_nrm.png"),
            TextureType::Normal
        );
    }

    #[test]
    fn test_from_filename_orm() {
        assert_eq!(
            TextureType::from_filename("texture_orm.png"),
            TextureType::ORM
        );
    }

    #[test]
    fn test_from_filename_mra() {
        assert_eq!(
            TextureType::from_filename("texture_mra.png"),
            TextureType::MRA
        );
    }

    #[test]
    fn test_from_filename_roughness_r() {
        assert_eq!(
            TextureType::from_filename("texture_r.png"),
            TextureType::Roughness
        );
    }

    #[test]
    fn test_from_filename_roughness_rough() {
        assert_eq!(
            TextureType::from_filename("texture_rough.png"),
            TextureType::Roughness
        );
    }

    #[test]
    fn test_from_filename_roughness_full() {
        assert_eq!(
            TextureType::from_filename("texture_roughness.png"),
            TextureType::Roughness
        );
    }

    #[test]
    fn test_from_filename_metallic_m() {
        assert_eq!(
            TextureType::from_filename("texture_m.png"),
            TextureType::Metallic
        );
    }

    #[test]
    fn test_from_filename_metallic_metal() {
        assert_eq!(
            TextureType::from_filename("texture_metal.png"),
            TextureType::Metallic
        );
    }

    #[test]
    fn test_from_filename_metallic_full() {
        assert_eq!(
            TextureType::from_filename("texture_metallic.png"),
            TextureType::Metallic
        );
    }

    #[test]
    fn test_from_filename_ao() {
        assert_eq!(
            TextureType::from_filename("texture_ao.png"),
            TextureType::AO
        );
    }

    #[test]
    fn test_from_filename_occlusion() {
        assert_eq!(
            TextureType::from_filename("texture_occlusion.png"),
            TextureType::AO
        );
    }

    #[test]
    fn test_from_filename_emission_e() {
        assert_eq!(
            TextureType::from_filename("texture_e.png"),
            TextureType::Emission
        );
    }

    #[test]
    fn test_from_filename_emission_emit() {
        assert_eq!(
            TextureType::from_filename("texture_emit.png"),
            TextureType::Emission
        );
    }

    #[test]
    fn test_from_filename_emission_glow() {
        assert_eq!(
            TextureType::from_filename("texture_glow.png"),
            TextureType::Emission
        );
    }

    #[test]
    fn test_from_filename_height_h() {
        assert_eq!(
            TextureType::from_filename("texture_h.png"),
            TextureType::Height
        );
    }

    #[test]
    fn test_from_filename_height_full() {
        assert_eq!(
            TextureType::from_filename("texture_height.png"),
            TextureType::Height
        );
    }

    #[test]
    fn test_from_filename_height_disp() {
        assert_eq!(
            TextureType::from_filename("texture_disp.png"),
            TextureType::Height
        );
    }

    #[test]
    fn test_from_filename_albedo() {
        assert_eq!(
            TextureType::from_filename("texture_albedo.png"),
            TextureType::Albedo
        );
    }

    #[test]
    fn test_from_filename_diffuse() {
        assert_eq!(
            TextureType::from_filename("texture_diffuse.png"),
            TextureType::Albedo
        );
    }

    #[test]
    fn test_from_filename_basecolor() {
        assert_eq!(
            TextureType::from_filename("texture_basecolor.png"),
            TextureType::Albedo
        );
    }

    #[test]
    fn test_from_filename_unknown() {
        assert_eq!(
            TextureType::from_filename("texture.png"),
            TextureType::Unknown
        );
    }

    #[test]
    fn test_from_filename_case_insensitive() {
        assert_eq!(
            TextureType::from_filename("Texture_NORMAL.png"),
            TextureType::Normal
        );
    }

    // Test name() uniqueness
    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = TextureType::all().iter().map(|t| t.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test icon() uniqueness
    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = TextureType::all().iter().map(|t| t.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", TextureType::Albedo);
        assert!(display.contains("🎨"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", TextureType::Albedo);
        assert!(display.contains("Albedo"));
    }
}

// ============================================================================
// ASSET CATEGORY TESTS
// ============================================================================

mod asset_category_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(AssetCategory::all().len(), 8);
    }

    // Test matches() for AssetCategory::All
    #[test]
    fn test_all_matches_model() {
        assert!(AssetCategory::All.matches(&AssetType::Model));
    }

    #[test]
    fn test_all_matches_texture() {
        assert!(AssetCategory::All.matches(&AssetType::Texture));
    }

    #[test]
    fn test_all_matches_unknown() {
        assert!(AssetCategory::All.matches(&AssetType::Unknown));
    }

    // Test matches() for specific categories
    #[test]
    fn test_models_matches_model() {
        assert!(AssetCategory::Models.matches(&AssetType::Model));
    }

    #[test]
    fn test_models_not_matches_texture() {
        assert!(!AssetCategory::Models.matches(&AssetType::Texture));
    }

    #[test]
    fn test_textures_matches_texture() {
        assert!(AssetCategory::Textures.matches(&AssetType::Texture));
    }

    #[test]
    fn test_textures_not_matches_model() {
        assert!(!AssetCategory::Textures.matches(&AssetType::Model));
    }

    #[test]
    fn test_materials_matches_material() {
        assert!(AssetCategory::Materials.matches(&AssetType::Material));
    }

    #[test]
    fn test_prefabs_matches_prefab() {
        assert!(AssetCategory::Prefabs.matches(&AssetType::Prefab));
    }

    #[test]
    fn test_scenes_matches_scene() {
        assert!(AssetCategory::Scenes.matches(&AssetType::Scene));
    }

    #[test]
    fn test_audio_matches_audio() {
        assert!(AssetCategory::Audio.matches(&AssetType::Audio));
    }

    #[test]
    fn test_configs_matches_config() {
        assert!(AssetCategory::Configs.matches(&AssetType::Config));
    }

    // Test name() uniqueness
    #[test]
    fn test_all_category_names_unique() {
        let names: Vec<&str> = AssetCategory::all().iter().map(|c| c.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test icon() uniqueness
    #[test]
    fn test_all_category_icons_unique() {
        let icons: Vec<&str> = AssetCategory::all().iter().map(|c| c.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }
}

// ============================================================================
// ASSET TYPE TESTS
// ============================================================================

mod asset_type_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_all_count() {
        assert_eq!(AssetType::all().len(), 10);
    }

    // Test is_content()
    #[test]
    fn test_model_is_content() {
        assert!(AssetType::Model.is_content());
    }

    #[test]
    fn test_texture_is_content() {
        assert!(AssetType::Texture.is_content());
    }

    #[test]
    fn test_scene_is_content() {
        assert!(AssetType::Scene.is_content());
    }

    #[test]
    fn test_material_is_content() {
        assert!(AssetType::Material.is_content());
    }

    #[test]
    fn test_audio_is_content() {
        assert!(AssetType::Audio.is_content());
    }

    #[test]
    fn test_prefab_is_content() {
        assert!(AssetType::Prefab.is_content());
    }

    #[test]
    fn test_config_is_not_content() {
        assert!(!AssetType::Config.is_content());
    }

    #[test]
    fn test_directory_is_not_content() {
        assert!(!AssetType::Directory.is_content());
    }

    #[test]
    fn test_unknown_is_not_content() {
        assert!(!AssetType::Unknown.is_content());
    }

    // Test from_path for model extensions
    #[test]
    fn test_from_path_glb() {
        assert_eq!(
            AssetType::from_path(Path::new("model.glb")),
            AssetType::Model
        );
    }

    #[test]
    fn test_from_path_gltf() {
        assert_eq!(
            AssetType::from_path(Path::new("model.gltf")),
            AssetType::Model
        );
    }

    #[test]
    fn test_from_path_obj() {
        assert_eq!(
            AssetType::from_path(Path::new("model.obj")),
            AssetType::Model
        );
    }

    #[test]
    fn test_from_path_fbx() {
        assert_eq!(
            AssetType::from_path(Path::new("model.fbx")),
            AssetType::Model
        );
    }

    // Test from_path for texture extensions
    #[test]
    fn test_from_path_png() {
        assert_eq!(
            AssetType::from_path(Path::new("texture.png")),
            AssetType::Texture
        );
    }

    #[test]
    fn test_from_path_jpg() {
        assert_eq!(
            AssetType::from_path(Path::new("texture.jpg")),
            AssetType::Texture
        );
    }

    #[test]
    fn test_from_path_jpeg() {
        assert_eq!(
            AssetType::from_path(Path::new("texture.jpeg")),
            AssetType::Texture
        );
    }

    #[test]
    fn test_from_path_ktx2() {
        assert_eq!(
            AssetType::from_path(Path::new("texture.ktx2")),
            AssetType::Texture
        );
    }

    #[test]
    fn test_from_path_dds() {
        assert_eq!(
            AssetType::from_path(Path::new("texture.dds")),
            AssetType::Texture
        );
    }

    // Test from_path for audio extensions
    #[test]
    fn test_from_path_wav() {
        assert_eq!(
            AssetType::from_path(Path::new("sound.wav")),
            AssetType::Audio
        );
    }

    #[test]
    fn test_from_path_ogg() {
        assert_eq!(
            AssetType::from_path(Path::new("sound.ogg")),
            AssetType::Audio
        );
    }

    #[test]
    fn test_from_path_mp3() {
        assert_eq!(
            AssetType::from_path(Path::new("sound.mp3")),
            AssetType::Audio
        );
    }

    // Test from_path for config extensions
    #[test]
    fn test_from_path_toml() {
        assert_eq!(
            AssetType::from_path(Path::new("config.toml")),
            AssetType::Config
        );
    }

    #[test]
    fn test_from_path_json() {
        assert_eq!(
            AssetType::from_path(Path::new("config.json")),
            AssetType::Config
        );
    }

    // Test from_path for scene and prefab
    #[test]
    fn test_from_path_ron() {
        assert_eq!(
            AssetType::from_path(Path::new("scene.ron")),
            AssetType::Scene
        );
    }

    #[test]
    fn test_from_path_prefab_ron() {
        assert_eq!(
            AssetType::from_path(Path::new("entity.prefab.ron")),
            AssetType::Prefab
        );
    }

    // Test from_path for unknown
    #[test]
    fn test_from_path_unknown() {
        assert_eq!(
            AssetType::from_path(Path::new("file.xyz")),
            AssetType::Unknown
        );
    }

    #[test]
    fn test_from_path_no_extension() {
        assert_eq!(AssetType::from_path(Path::new("file")), AssetType::Unknown);
    }

    // Test name() uniqueness
    #[test]
    fn test_all_type_names_unique() {
        let names: Vec<&str> = AssetType::all().iter().map(|t| t.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test icon() uniqueness
    #[test]
    fn test_all_type_icons_unique() {
        let icons: Vec<&str> = AssetType::all().iter().map(|t| t.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }
}

// ============================================================================
// ASSET ACTION TESTS
// ============================================================================

mod asset_action_tests {
    use super::*;

    fn make_path(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn test_all_variants_count() {
        assert_eq!(AssetAction::all_variants().len(), 10);
    }

    // Test is_modifying()
    #[test]
    fn test_import_model_is_modifying() {
        let action = AssetAction::ImportModel {
            path: make_path("model.glb"),
        };
        assert!(action.is_modifying());
    }

    #[test]
    fn test_apply_texture_is_modifying() {
        let action = AssetAction::ApplyTexture {
            path: make_path("texture.png"),
            texture_type: TextureType::Albedo,
        };
        assert!(action.is_modifying());
    }

    #[test]
    fn test_apply_material_is_modifying() {
        let action = AssetAction::ApplyMaterial {
            path: make_path("material.ron"),
        };
        assert!(action.is_modifying());
    }

    #[test]
    fn test_spawn_prefab_is_modifying() {
        let action = AssetAction::SpawnPrefab {
            path: make_path("prefab.ron"),
        };
        assert!(action.is_modifying());
    }

    #[test]
    fn test_load_to_viewport_not_modifying() {
        let action = AssetAction::LoadToViewport {
            path: make_path("model.glb"),
        };
        assert!(!action.is_modifying());
    }

    #[test]
    fn test_load_scene_not_modifying() {
        let action = AssetAction::LoadScene {
            path: make_path("scene.ron"),
        };
        assert!(!action.is_modifying());
    }

    #[test]
    fn test_open_external_not_modifying() {
        let action = AssetAction::OpenExternal {
            path: make_path("file.txt"),
        };
        assert!(!action.is_modifying());
    }

    #[test]
    fn test_inspect_asset_not_modifying() {
        let action = AssetAction::InspectAsset {
            path: make_path("asset.ron"),
        };
        assert!(!action.is_modifying());
    }

    // Test is_viewing()
    #[test]
    fn test_load_to_viewport_is_viewing() {
        let action = AssetAction::LoadToViewport {
            path: make_path("model.glb"),
        };
        assert!(action.is_viewing());
    }

    #[test]
    fn test_open_external_is_viewing() {
        let action = AssetAction::OpenExternal {
            path: make_path("file.txt"),
        };
        assert!(action.is_viewing());
    }

    #[test]
    fn test_inspect_asset_is_viewing() {
        let action = AssetAction::InspectAsset {
            path: make_path("asset.ron"),
        };
        assert!(action.is_viewing());
    }

    #[test]
    fn test_import_model_not_viewing() {
        let action = AssetAction::ImportModel {
            path: make_path("model.glb"),
        };
        assert!(!action.is_viewing());
    }

    #[test]
    fn test_load_scene_not_viewing() {
        let action = AssetAction::LoadScene {
            path: make_path("scene.ron"),
        };
        assert!(!action.is_viewing());
    }

    #[test]
    fn test_spawn_prefab_not_viewing() {
        let action = AssetAction::SpawnPrefab {
            path: make_path("prefab.ron"),
        };
        assert!(!action.is_viewing());
    }

    // Test is_scene_action()
    #[test]
    fn test_load_scene_is_scene_action() {
        let action = AssetAction::LoadScene {
            path: make_path("scene.ron"),
        };
        assert!(action.is_scene_action());
    }

    #[test]
    fn test_import_model_not_scene_action() {
        let action = AssetAction::ImportModel {
            path: make_path("model.glb"),
        };
        assert!(!action.is_scene_action());
    }

    #[test]
    fn test_spawn_prefab_not_scene_action() {
        let action = AssetAction::SpawnPrefab {
            path: make_path("prefab.ron"),
        };
        assert!(!action.is_scene_action());
    }

    // Test path()
    #[test]
    fn test_import_model_path() {
        let action = AssetAction::ImportModel {
            path: make_path("model.glb"),
        };
        assert_eq!(action.path(), &make_path("model.glb"));
    }

    #[test]
    fn test_apply_texture_path() {
        let action = AssetAction::ApplyTexture {
            path: make_path("texture.png"),
            texture_type: TextureType::Normal,
        };
        assert_eq!(action.path(), &make_path("texture.png"));
    }

    #[test]
    fn test_load_scene_path() {
        let action = AssetAction::LoadScene {
            path: make_path("scene.ron"),
        };
        assert_eq!(action.path(), &make_path("scene.ron"));
    }

    // Test name() uniqueness
    #[test]
    fn test_all_action_names_unique() {
        let actions = vec![
            AssetAction::ImportModel {
                path: make_path("a"),
            },
            AssetAction::LoadToViewport {
                path: make_path("b"),
            },
            AssetAction::ApplyTexture {
                path: make_path("c"),
                texture_type: TextureType::Albedo,
            },
            AssetAction::ApplyMaterial {
                path: make_path("d"),
            },
            AssetAction::LoadScene {
                path: make_path("e"),
            },
            AssetAction::SpawnPrefab {
                path: make_path("f"),
            },
            AssetAction::OpenExternal {
                path: make_path("g"),
            },
            AssetAction::InspectAsset {
                path: make_path("h"),
            },
        ];
        let names: Vec<&str> = actions.iter().map(|a| a.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test icon() uniqueness
    #[test]
    fn test_all_action_icons_unique() {
        let actions = vec![
            AssetAction::ImportModel {
                path: make_path("a"),
            },
            AssetAction::LoadToViewport {
                path: make_path("b"),
            },
            AssetAction::ApplyTexture {
                path: make_path("c"),
                texture_type: TextureType::Albedo,
            },
            AssetAction::ApplyMaterial {
                path: make_path("d"),
            },
            AssetAction::LoadScene {
                path: make_path("e"),
            },
            AssetAction::SpawnPrefab {
                path: make_path("f"),
            },
            AssetAction::OpenExternal {
                path: make_path("g"),
            },
            AssetAction::InspectAsset {
                path: make_path("h"),
            },
        ];
        let icons: Vec<&str> = actions.iter().map(|a| a.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let action = AssetAction::ImportModel {
            path: make_path("model.glb"),
        };
        let display = format!("{}", action);
        assert!(display.contains("📥"));
    }

    #[test]
    fn test_display_contains_name() {
        let action = AssetAction::ImportModel {
            path: make_path("model.glb"),
        };
        let display = format!("{}", action);
        assert!(display.contains("Import Model"));
    }
}
