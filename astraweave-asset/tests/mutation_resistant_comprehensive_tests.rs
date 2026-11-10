//! Comprehensive mutation-resistant tests for astraweave-asset
//!
//! These tests are designed to achieve 90%+ mutation kill rate by:
//! - Testing all enum variants and their behaviors
//! - Verifying state transitions and side effects
//! - Checking boundary conditions and edge cases
//! - Testing return values from all public methods

#![allow(unused_imports, clippy::assertions_on_constants)]

use astraweave_asset::cell_loader::*;
use astraweave_asset::nanite_preprocess::*;
use astraweave_asset::*;
use glam::Vec3;

// ═══════════════════════════════════════════════════════════════════════════
// CELL LOADER ASSETKIND TESTS (not lib.rs AssetKind)
// ═══════════════════════════════════════════════════════════════════════════

mod cell_loader_asset_kind_tests {
    use astraweave_asset::cell_loader::AssetKind;

    #[test]
    fn test_mesh_variant() {
        let kind = AssetKind::Mesh;
        assert_eq!(kind, AssetKind::Mesh);
    }

    #[test]
    fn test_texture_variant() {
        let kind = AssetKind::Texture;
        assert_eq!(kind, AssetKind::Texture);
    }

    #[test]
    fn test_material_variant() {
        let kind = AssetKind::Material;
        assert_eq!(kind, AssetKind::Material);
    }

    #[test]
    fn test_audio_variant() {
        let kind = AssetKind::Audio;
        assert_eq!(kind, AssetKind::Audio);
    }

    #[test]
    fn test_animation_variant() {
        let kind = AssetKind::Animation;
        assert_eq!(kind, AssetKind::Animation);
    }

    #[test]
    fn test_other_variant() {
        let kind = AssetKind::Other;
        assert_eq!(kind, AssetKind::Other);
    }

    #[test]
    fn test_asset_kind_ne() {
        assert_ne!(AssetKind::Mesh, AssetKind::Texture);
        assert_ne!(AssetKind::Material, AssetKind::Audio);
    }

    #[test]
    fn test_asset_kind_clone() {
        let kind = AssetKind::Mesh;
        let cloned = kind.clone();
        assert_eq!(kind, cloned);
    }

    #[test]
    fn test_asset_kind_debug() {
        let kind = AssetKind::Animation;
        let debug = format!("{:?}", kind);
        assert!(debug.contains("Animation"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ASSET REF TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod asset_ref_tests {
    use astraweave_asset::cell_loader::{AssetKind, AssetRef};

    #[test]
    fn test_asset_ref_new() {
        let asset = AssetRef::new("meshes/player.glb", AssetKind::Mesh);
        assert_eq!(asset.path, "meshes/player.glb");
        assert_eq!(asset.kind, AssetKind::Mesh);
        assert!(asset.guid.is_none());
    }

    #[test]
    fn test_asset_ref_with_guid() {
        let asset = AssetRef::new("textures/floor.png", AssetKind::Texture).with_guid("abc123");
        assert_eq!(asset.guid, Some("abc123".to_string()));
    }

    #[test]
    fn test_asset_ref_clone() {
        let asset = AssetRef::new("audio/music.ogg", AssetKind::Audio);
        let cloned = asset.clone();
        assert_eq!(asset.path, cloned.path);
        assert_eq!(asset.kind, cloned.kind);
    }

    #[test]
    fn test_asset_ref_each_kind() {
        let kinds = [
            AssetKind::Mesh,
            AssetKind::Texture,
            AssetKind::Material,
            AssetKind::Audio,
            AssetKind::Animation,
            AssetKind::Other,
        ];
        for kind in kinds {
            let asset = AssetRef::new("test/asset", kind.clone());
            assert_eq!(asset.kind, kind);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ENTITY DATA TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod entity_data_tests {
    use astraweave_asset::cell_loader::EntityData;

    #[test]
    fn test_entity_data_new() {
        let entity = EntityData::new([10.0, 5.0, 20.0]);
        assert_eq!(entity.position, [10.0, 5.0, 20.0]);
        assert_eq!(entity.rotation, [0.0, 0.0, 0.0, 1.0]); // Identity
        assert_eq!(entity.scale, [1.0, 1.0, 1.0]);
        assert!(entity.name.is_none());
        assert!(entity.mesh.is_none());
        assert!(entity.material.is_none());
        assert!(entity.components.is_empty());
    }

    #[test]
    fn test_entity_data_with_name() {
        let entity = EntityData::new([0.0, 0.0, 0.0]).with_name("Player");
        assert_eq!(entity.name, Some("Player".to_string()));
    }

    #[test]
    fn test_entity_data_with_mesh() {
        let entity = EntityData::new([0.0, 0.0, 0.0]).with_mesh("meshes/player.glb");
        assert_eq!(entity.mesh, Some("meshes/player.glb".to_string()));
    }

    #[test]
    fn test_entity_data_with_material() {
        let entity = EntityData::new([0.0, 0.0, 0.0]).with_material(5);
        assert_eq!(entity.material, Some(5));
    }

    #[test]
    fn test_entity_data_chained() {
        let entity = EntityData::new([1.0, 2.0, 3.0])
            .with_name("NPC")
            .with_mesh("meshes/npc.glb")
            .with_material(3);
        assert_eq!(entity.position, [1.0, 2.0, 3.0]);
        assert_eq!(entity.name, Some("NPC".to_string()));
        assert_eq!(entity.mesh, Some("meshes/npc.glb".to_string()));
        assert_eq!(entity.material, Some(3));
    }

    #[test]
    fn test_entity_data_clone() {
        let entity = EntityData::new([5.0, 5.0, 5.0]).with_name("Clone Test");
        let cloned = entity.clone();
        assert_eq!(entity.position, cloned.position);
        assert_eq!(entity.name, cloned.name);
    }

    #[test]
    fn test_entity_data_default_rotation_is_identity() {
        let entity = EntityData::new([0.0, 0.0, 0.0]);
        // quaternion identity is (0, 0, 0, 1)
        assert_eq!(entity.rotation[3], 1.0);
    }

    #[test]
    fn test_entity_data_default_scale_is_unit() {
        let entity = EntityData::new([0.0, 0.0, 0.0]);
        assert_eq!(entity.scale, [1.0, 1.0, 1.0]);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CELL DATA TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod cell_data_tests {
    use astraweave_asset::cell_loader::{AssetKind, AssetRef, CellData, EntityData};

    #[test]
    fn test_cell_data_new() {
        let cell = CellData::new([0, 0, 0]);
        assert_eq!(cell.coord, [0, 0, 0]);
        assert!(cell.entities.is_empty());
        assert!(cell.assets.is_empty());
        assert!(cell.metadata.is_none());
    }

    #[test]
    fn test_cell_data_add_entity() {
        let mut cell = CellData::new([1, 2, 3]);
        let entity = EntityData::new([10.0, 0.0, 0.0]);
        cell.add_entity(entity);
        assert_eq!(cell.entities.len(), 1);
        assert_eq!(cell.entities[0].position, [10.0, 0.0, 0.0]);
    }

    #[test]
    fn test_cell_data_add_multiple_entities() {
        let mut cell = CellData::new([0, 0, 0]);
        cell.add_entity(EntityData::new([1.0, 0.0, 0.0]));
        cell.add_entity(EntityData::new([2.0, 0.0, 0.0]));
        cell.add_entity(EntityData::new([3.0, 0.0, 0.0]));
        assert_eq!(cell.entities.len(), 3);
    }

    #[test]
    fn test_cell_data_add_asset() {
        let mut cell = CellData::new([0, 0, 0]);
        let asset = AssetRef::new("textures/ground.png", AssetKind::Texture);
        cell.add_asset(asset);
        assert_eq!(cell.assets.len(), 1);
    }

    #[test]
    fn test_cell_data_add_asset_deduplicates() {
        let mut cell = CellData::new([0, 0, 0]);
        let asset1 = AssetRef::new("textures/ground.png", AssetKind::Texture);
        let asset2 = AssetRef::new("textures/ground.png", AssetKind::Texture);
        cell.add_asset(asset1);
        cell.add_asset(asset2);
        assert_eq!(cell.assets.len(), 1); // Deduped
    }

    #[test]
    fn test_cell_data_add_different_assets() {
        let mut cell = CellData::new([0, 0, 0]);
        cell.add_asset(AssetRef::new("a.png", AssetKind::Texture));
        cell.add_asset(AssetRef::new("b.png", AssetKind::Texture));
        assert_eq!(cell.assets.len(), 2);
    }

    #[test]
    fn test_cell_data_memory_estimate_empty() {
        let cell = CellData::new([0, 0, 0]);
        let estimate = cell.memory_estimate();
        assert!(estimate > 0);
    }

    #[test]
    fn test_cell_data_memory_estimate_with_entities() {
        let mut cell = CellData::new([0, 0, 0]);
        let empty_estimate = cell.memory_estimate();
        cell.add_entity(EntityData::new([0.0, 0.0, 0.0]));
        let with_entity = cell.memory_estimate();
        assert!(with_entity > empty_estimate);
    }

    #[test]
    fn test_cell_data_clone() {
        let mut cell = CellData::new([5, 6, 7]);
        cell.add_entity(EntityData::new([1.0, 2.0, 3.0]));
        let cloned = cell.clone();
        assert_eq!(cell.coord, cloned.coord);
        assert_eq!(cell.entities.len(), cloned.entities.len());
    }

    #[test]
    fn test_cell_data_coord_values() {
        let cell = CellData::new([-10, 20, -30]);
        assert_eq!(cell.coord[0], -10);
        assert_eq!(cell.coord[1], 20);
        assert_eq!(cell.coord[2], -30);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CELL PATH FROM COORD TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod cell_path_tests {
    use astraweave_asset::cell_loader::cell_path_from_coord;
    use std::path::Path;

    #[test]
    fn test_cell_path_from_coord_origin() {
        let path = cell_path_from_coord([0, 0, 0], Path::new("cells"));
        assert!(path.to_string_lossy().contains("0_0_0.ron"));
    }

    #[test]
    fn test_cell_path_from_coord_positive() {
        let path = cell_path_from_coord([10, 20, 30], Path::new("data/cells"));
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("10_20_30.ron"));
    }

    #[test]
    fn test_cell_path_from_coord_negative() {
        let path = cell_path_from_coord([-5, -10, -15], Path::new("cells"));
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("-5_-10_-15.ron"));
    }

    #[test]
    fn test_cell_path_from_coord_mixed() {
        let path = cell_path_from_coord([1, -2, 3], Path::new("cells"));
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("1_-2_3.ron"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AABB ADDITIONAL TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod aabb_tests {
    use astraweave_asset::nanite_preprocess::AABB;
    use glam::Vec3;

    #[test]
    fn test_aabb_new() {
        let aabb = AABB::new(Vec3::ZERO, Vec3::ONE);
        assert_eq!(aabb.min, Vec3::ZERO);
        assert_eq!(aabb.max, Vec3::ONE);
    }

    #[test]
    fn test_aabb_from_points_single() {
        let points = [[1.0, 2.0, 3.0]];
        let aabb = AABB::from_points(&points);
        assert_eq!(aabb.min, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_aabb_from_points_multiple() {
        let points = [[0.0, 0.0, 0.0], [10.0, 5.0, 8.0], [3.0, 12.0, 1.0]];
        let aabb = AABB::from_points(&points);
        assert_eq!(aabb.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(aabb.max, Vec3::new(10.0, 12.0, 8.0));
    }

    #[test]
    fn test_aabb_center() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 10.0, 10.0));
        assert_eq!(aabb.center(), Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_aabb_extents() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 10.0, 10.0));
        assert_eq!(aabb.extents(), Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_aabb_diagonal() {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(3.0, 4.0, 0.0));
        // Diagonal should be 5.0 (3-4-5 triangle)
        assert!((aabb.diagonal() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_aabb_contains_inside() {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        assert!(aabb.contains(Vec3::new(5.0, 5.0, 5.0)));
    }

    #[test]
    fn test_aabb_contains_on_boundary() {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        assert!(aabb.contains(Vec3::ZERO));
        assert!(aabb.contains(Vec3::new(10.0, 10.0, 10.0)));
    }

    #[test]
    fn test_aabb_contains_outside() {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        assert!(!aabb.contains(Vec3::new(-1.0, 5.0, 5.0)));
        assert!(!aabb.contains(Vec3::new(11.0, 5.0, 5.0)));
    }

    #[test]
    fn test_aabb_merge() {
        let a = AABB::new(Vec3::ZERO, Vec3::new(5.0, 5.0, 5.0));
        let b = AABB::new(Vec3::new(3.0, 3.0, 3.0), Vec3::new(10.0, 10.0, 10.0));
        let merged = a.merge(&b);
        assert_eq!(merged.min, Vec3::ZERO);
        assert_eq!(merged.max, Vec3::new(10.0, 10.0, 10.0));
    }

    #[test]
    fn test_aabb_merge_disjoint() {
        let a = AABB::new(Vec3::ZERO, Vec3::ONE);
        let b = AABB::new(Vec3::new(10.0, 10.0, 10.0), Vec3::new(11.0, 11.0, 11.0));
        let merged = a.merge(&b);
        assert_eq!(merged.min, Vec3::ZERO);
        assert_eq!(merged.max, Vec3::new(11.0, 11.0, 11.0));
    }

    #[test]
    fn test_aabb_clone() {
        let aabb = AABB::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0));
        let cloned = aabb;
        assert_eq!(aabb.min, cloned.min);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BOUNDING CONE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod bounding_cone_tests {
    use astraweave_asset::nanite_preprocess::BoundingCone;
    use glam::Vec3;

    #[test]
    fn test_bounding_cone_from_empty_triangles() {
        let cone = BoundingCone::from_triangles(&[], &[], &[]);
        assert_eq!(cone.apex, Vec3::ZERO);
        assert_eq!(cone.axis, Vec3::Z);
        assert_eq!(cone.cutoff, -1.0);
    }

    #[test]
    fn test_bounding_cone_is_backfacing_parallel() {
        let cone = BoundingCone {
            apex: Vec3::ZERO,
            axis: Vec3::Z,
            cutoff: 0.0, // 90 degree cone
        };
        // Viewing from -Z direction (opposite to axis)
        assert!(cone.is_backfacing(-Vec3::Z));
    }

    #[test]
    fn test_bounding_cone_not_backfacing() {
        let cone = BoundingCone {
            apex: Vec3::ZERO,
            axis: Vec3::Z,
            cutoff: 0.0,
        };
        // Viewing from +Z direction (same as axis)
        assert!(!cone.is_backfacing(Vec3::Z));
    }

    #[test]
    fn test_bounding_cone_clone() {
        let cone = BoundingCone {
            apex: Vec3::ONE,
            axis: Vec3::Y,
            cutoff: 0.5,
        };
        let cloned = cone;
        assert_eq!(cone.apex, cloned.apex);
        assert_eq!(cone.axis, cloned.axis);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MESHLET TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod meshlet_tests {
    use astraweave_asset::nanite_preprocess::Meshlet;

    #[test]
    fn test_meshlet_new() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let vertices = vec![0, 1, 2];
        let indices = vec![0, 1, 2]; // One triangle

        let meshlet = Meshlet::new(vertices, indices, &positions, &normals, 0);
        assert_eq!(meshlet.lod_level, 0);
        assert_eq!(meshlet.triangle_count(), 1);
        assert_eq!(meshlet.vertex_count(), 3);
    }

    #[test]
    fn test_meshlet_triangle_count() {
        let positions = vec![[0.0; 3]; 6];
        let normals = vec![[0.0, 0.0, 1.0]; 6];
        let vertices = vec![0, 1, 2, 3, 4, 5];
        let indices = vec![0, 1, 2, 3, 4, 5]; // Two triangles

        let meshlet = Meshlet::new(vertices, indices, &positions, &normals, 0);
        assert_eq!(meshlet.triangle_count(), 2);
    }

    #[test]
    fn test_meshlet_vertex_count() {
        let positions = vec![[0.0; 3]; 10];
        let normals = vec![[0.0, 0.0, 1.0]; 10];
        let vertices: Vec<u32> = (0..10).collect();
        let indices = vec![0, 1, 2];

        let meshlet = Meshlet::new(vertices, indices, &positions, &normals, 0);
        assert_eq!(meshlet.vertex_count(), 10);
    }

    #[test]
    fn test_meshlet_lod_level() {
        let positions = [[0.0; 3]];
        let normals = [[0.0, 0.0, 1.0]];
        let meshlet = Meshlet::new(vec![0], vec![], &positions, &normals, 3);
        assert_eq!(meshlet.lod_level, 3);
    }

    #[test]
    fn test_meshlet_clone() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let meshlet = Meshlet::new(vec![0, 1, 2], vec![0, 1, 2], &positions, &normals, 0);
        let cloned = meshlet.clone();
        assert_eq!(meshlet.lod_level, cloned.lod_level);
        assert_eq!(meshlet.vertices.len(), cloned.vertices.len());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GUID AND ASSET CACHE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod guid_cache_tests {
    use astraweave_asset::{guid_for_path, AssetCache};

    #[test]
    fn test_guid_deterministic() {
        let a = guid_for_path("assets/test.png");
        let b = guid_for_path("assets/test.png");
        assert_eq!(a, b);
    }

    #[test]
    fn test_guid_case_insensitive() {
        let a = guid_for_path("Assets/Test.PNG");
        let b = guid_for_path("assets/test.png");
        assert_eq!(a, b);
    }

    #[test]
    fn test_guid_slash_normalized() {
        let a = guid_for_path("assets/test.png");
        let b = guid_for_path("assets\\test.png");
        assert_eq!(a, b);
    }

    #[test]
    fn test_guid_different_paths() {
        let a = guid_for_path("assets/a.png");
        let b = guid_for_path("assets/b.png");
        assert_ne!(a, b);
    }

    #[test]
    fn test_guid_length() {
        let guid = guid_for_path("test");
        assert_eq!(guid.len(), 32); // 128-bit hex
    }

    #[test]
    fn test_asset_cache_insert_get() {
        let mut cache = AssetCache::<i32>::default();
        let id = cache.insert("test.png", 42);
        assert_eq!(cache.get(&id), Some(&42));
    }

    #[test]
    fn test_asset_cache_len() {
        let mut cache = AssetCache::<i32>::default();
        assert_eq!(cache.len(), 0);
        cache.insert("a.png", 1);
        assert_eq!(cache.len(), 1);
        cache.insert("b.png", 2);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_asset_cache_is_empty() {
        let cache = AssetCache::<i32>::default();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_asset_cache_is_not_empty() {
        let mut cache = AssetCache::<i32>::default();
        cache.insert("test", 1);
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_asset_cache_get_nonexistent() {
        let cache = AssetCache::<i32>::default();
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_asset_cache_overwrite() {
        let mut cache = AssetCache::<i32>::default();
        let id1 = cache.insert("test.png", 10);
        let id2 = cache.insert("test.png", 20);
        assert_eq!(id1, id2); // Same GUID
        assert_eq!(cache.get(&id1), Some(&20)); // Overwritten
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ASSET DATABASE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod asset_database_tests {
    use astraweave_asset::AssetDatabase;

    #[test]
    fn test_asset_database_new() {
        let db = AssetDatabase::new();
        assert!(db.assets.is_empty());
        assert!(db.path_to_guid.is_empty());
    }

    #[test]
    fn test_asset_database_default() {
        let db = AssetDatabase::default();
        assert!(db.assets.is_empty());
    }

    #[test]
    fn test_asset_database_get_asset_nonexistent() {
        let db = AssetDatabase::new();
        assert!(db.get_asset("fake-guid").is_none());
    }

    #[test]
    fn test_asset_database_get_guid_by_path_nonexistent() {
        let db = AssetDatabase::new();
        assert!(db
            .get_guid_by_path(std::path::Path::new("fake.png"))
            .is_none());
    }

    #[test]
    fn test_asset_database_get_dependents_empty() {
        let db = AssetDatabase::new();
        assert!(db.get_dependents("fake-guid").is_none());
    }

    #[test]
    fn test_asset_database_get_dependencies_empty() {
        let db = AssetDatabase::new();
        assert!(db.get_dependencies("fake-guid").is_none());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ASSET KIND (LIB) TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod lib_asset_kind_tests {
    use astraweave_asset::AssetKind;

    #[test]
    fn test_asset_kind_mesh() {
        assert_eq!(AssetKind::Mesh, AssetKind::Mesh);
    }

    #[test]
    fn test_asset_kind_texture() {
        assert_eq!(AssetKind::Texture, AssetKind::Texture);
    }

    #[test]
    fn test_asset_kind_audio() {
        assert_eq!(AssetKind::Audio, AssetKind::Audio);
    }

    #[test]
    fn test_asset_kind_dialogue() {
        assert_eq!(AssetKind::Dialogue, AssetKind::Dialogue);
    }

    #[test]
    fn test_asset_kind_material() {
        assert_eq!(AssetKind::Material, AssetKind::Material);
    }

    #[test]
    fn test_asset_kind_animation() {
        assert_eq!(AssetKind::Animation, AssetKind::Animation);
    }

    #[test]
    fn test_asset_kind_script() {
        assert_eq!(AssetKind::Script, AssetKind::Script);
    }

    #[test]
    fn test_asset_kind_blender_source() {
        assert_eq!(AssetKind::BlenderSource, AssetKind::BlenderSource);
    }

    #[test]
    fn test_asset_kind_other() {
        assert_eq!(AssetKind::Other, AssetKind::Other);
    }

    #[test]
    fn test_asset_kind_all_different() {
        assert_ne!(AssetKind::Mesh, AssetKind::Texture);
        assert_ne!(AssetKind::Audio, AssetKind::Dialogue);
        assert_ne!(AssetKind::Material, AssetKind::Script);
    }

    #[test]
    fn test_asset_kind_clone() {
        let kind = AssetKind::Mesh;
        let cloned = kind.clone();
        assert_eq!(kind, cloned);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HOT RELOAD STATS TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod hot_reload_tests {
    use astraweave_asset::HotReloadStats;

    #[test]
    fn test_hot_reload_stats_clone() {
        let stats = HotReloadStats { pending_count: 5 };
        let cloned = stats.clone();
        assert_eq!(stats.pending_count, cloned.pending_count);
    }

    #[test]
    fn test_hot_reload_stats_debug() {
        let stats = HotReloadStats { pending_count: 10 };
        let debug = format!("{:?}", stats);
        assert!(debug.contains("10"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ASSET MANIFEST TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod asset_manifest_tests {
    use astraweave_asset::AssetManifest;

    #[test]
    fn test_asset_manifest_validate() {
        let result = AssetManifest::validate();
        assert!(result.is_ok());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod constants_tests {
    use astraweave_asset::nanite_preprocess::{MAX_MESHLET_TRIANGLES, MAX_MESHLET_VERTICES};

    #[test]
    fn test_max_meshlet_vertices() {
        assert_eq!(MAX_MESHLET_VERTICES, 64);
    }

    #[test]
    fn test_max_meshlet_triangles() {
        assert_eq!(MAX_MESHLET_TRIANGLES, 124);
    }

    #[test]
    fn test_constants_reasonable() {
        assert!(MAX_MESHLET_VERTICES > 0);
        assert!(MAX_MESHLET_TRIANGLES > 0);
        assert!(MAX_MESHLET_TRIANGLES >= MAX_MESHLET_VERTICES / 2);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GLTF LOADER MUTATION KILL TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "gltf")]
mod gltf_loader_mutation_kill_tests {
    use astraweave_asset::gltf_loader::*;

    /// Kills L240: `s.contains("meshes") && s.contains("accessors")` mutated to `||`
    /// With `||`, JSON having only "meshes" (no "accessors") would incorrectly pass.
    #[test]
    fn load_gltf_bytes_rejects_json_with_only_meshes() {
        let json = br#"{"asset":{"version":"2.0"},"meshes":[]}"#;
        let result = load_gltf_bytes(json);
        assert!(
            result.is_err(),
            "JSON with meshes but no accessors must fail"
        );
    }

    /// Also kills L240 from the other direction: only "accessors", no "meshes"
    #[test]
    fn load_gltf_bytes_rejects_json_with_only_accessors() {
        let json = br#"{"asset":{"version":"2.0"},"accessors":[]}"#;
        let result = load_gltf_bytes(json);
        assert!(
            result.is_err(),
            "JSON with accessors but no meshes must fail"
        );
    }

    /// Kills L432: `bytes.len() >= 12` mutated to `bytes.len() < 12`
    /// Short bytes (< 12) must NOT be treated as GLB — they should fall to JSON path and fail.
    #[test]
    fn load_first_mesh_short_bytes_not_treated_as_glb() {
        // 4 bytes of GLB magic but total length < 12: must not enter GLB branch
        let short = b"glTF\x02\x00\x00\x00";
        let result = load_first_mesh_and_material(std::path::Path::new("."), short);
        // This should fail (not enough data for GLB, and not valid JSON either)
        assert!(result.is_err(), "Short GLB-like bytes must fail");
    }

    /// Validates that valid JSON with both fields passes load_gltf_bytes
    #[test]
    fn load_gltf_bytes_accepts_json_with_both_fields() {
        let json = br#"{"asset":{"version":"2.0"},"meshes":[],"accessors":[]}"#;
        let result = load_gltf_bytes(json);
        assert!(
            result.is_ok(),
            "JSON with both meshes and accessors must pass"
        );
    }
}
