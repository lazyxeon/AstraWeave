#![allow(clippy::assertions_on_constants, clippy::useless_vec)]
//! Mutation-killing tests for astraweave-asset
//!
//! These tests are designed to detect common mutations:
//! - Boundary condition changes (< vs <=, > vs >=)
//! - Arithmetic operator swaps (+/-, */)
//! - Boolean logic inversions
//! - Constant replacement
//! - Return value modifications

use super::*;
use glam::Vec3;

// ============================================================================
// AssetKind Enum Tests
// ============================================================================

mod asset_kind_tests {
    use super::*;

    #[test]
    fn test_mesh_variant_identity() {
        let kind = AssetKind::Mesh;
        assert!(matches!(kind, AssetKind::Mesh));
        assert!(!matches!(kind.clone(), AssetKind::Texture));
        assert!(!matches!(kind.clone(), AssetKind::Audio));
        assert!(!matches!(kind.clone(), AssetKind::Other));
    }

    #[test]
    fn test_texture_variant_identity() {
        let kind = AssetKind::Texture;
        assert!(matches!(kind, AssetKind::Texture));
        assert!(!matches!(kind.clone(), AssetKind::Mesh));
        assert!(!matches!(kind.clone(), AssetKind::Material));
    }

    #[test]
    fn test_audio_variant_identity() {
        let kind = AssetKind::Audio;
        assert!(matches!(kind, AssetKind::Audio));
        assert!(!matches!(kind.clone(), AssetKind::Mesh));
        assert!(!matches!(kind.clone(), AssetKind::Dialogue));
    }

    #[test]
    fn test_dialogue_variant_identity() {
        let kind = AssetKind::Dialogue;
        assert!(matches!(kind, AssetKind::Dialogue));
        assert!(!matches!(kind.clone(), AssetKind::Audio));
    }

    #[test]
    fn test_material_variant_identity() {
        let kind = AssetKind::Material;
        assert!(matches!(kind, AssetKind::Material));
        assert!(!matches!(kind.clone(), AssetKind::Texture));
    }

    #[test]
    fn test_animation_variant_identity() {
        let kind = AssetKind::Animation;
        assert!(matches!(kind, AssetKind::Animation));
        assert!(!matches!(kind.clone(), AssetKind::Script));
    }

    #[test]
    fn test_script_variant_identity() {
        let kind = AssetKind::Script;
        assert!(matches!(kind, AssetKind::Script));
        assert!(!matches!(kind.clone(), AssetKind::Animation));
    }

    #[test]
    fn test_blender_source_variant_identity() {
        let kind = AssetKind::BlenderSource;
        assert!(matches!(kind, AssetKind::BlenderSource));
        assert!(!matches!(kind.clone(), AssetKind::Mesh));
    }

    #[test]
    fn test_other_variant_identity() {
        let kind = AssetKind::Other;
        assert!(matches!(kind, AssetKind::Other));
        assert!(!matches!(kind.clone(), AssetKind::Mesh));
    }

    #[test]
    fn test_asset_kind_equality() {
        assert_eq!(AssetKind::Mesh, AssetKind::Mesh);
        assert_ne!(AssetKind::Mesh, AssetKind::Texture);
        assert_ne!(AssetKind::Audio, AssetKind::Dialogue);
        assert_ne!(AssetKind::Material, AssetKind::Animation);
    }

    #[test]
    fn test_asset_kind_clone() {
        let original = AssetKind::Animation;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_all_variants_distinguishable() {
        let kinds = vec![
            AssetKind::Mesh,
            AssetKind::Texture,
            AssetKind::Audio,
            AssetKind::Dialogue,
            AssetKind::Material,
            AssetKind::Animation,
            AssetKind::Script,
            AssetKind::BlenderSource,
            AssetKind::Other,
        ];

        // Each variant should only match itself
        for (i, kind1) in kinds.iter().enumerate() {
            for (j, kind2) in kinds.iter().enumerate() {
                if i == j {
                    assert_eq!(kind1, kind2, "Same variant should be equal");
                } else {
                    assert_ne!(kind1, kind2, "Different variants should not be equal");
                }
            }
        }
    }
}

// ============================================================================
// AABB Tests (from nanite_preprocess)
// ============================================================================

mod aabb_tests {
    use super::*;
    use crate::nanite_preprocess::AABB;

    #[test]
    fn test_aabb_new() {
        let min = Vec3::new(-1.0, -2.0, -3.0);
        let max = Vec3::new(1.0, 2.0, 3.0);
        let aabb = AABB::new(min, max);
        assert_eq!(aabb.min, min);
        assert_eq!(aabb.max, max);
    }

    #[test]
    fn test_aabb_center_calculation() {
        let aabb = AABB::new(Vec3::new(-2.0, -4.0, -6.0), Vec3::new(2.0, 4.0, 6.0));
        let center = aabb.center();
        assert_eq!(center, Vec3::ZERO);
    }

    #[test]
    fn test_aabb_center_offset() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 8.0, 12.0));
        let center = aabb.center();
        assert_eq!(center.x, 2.0);
        assert_eq!(center.y, 4.0);
        assert_eq!(center.z, 6.0);
    }

    #[test]
    fn test_aabb_extents() {
        let aabb = AABB::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0));
        let extents = aabb.extents();
        assert_eq!(extents.x, 1.0);
        assert_eq!(extents.y, 2.0);
        assert_eq!(extents.z, 3.0);
    }

    #[test]
    fn test_aabb_diagonal() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(3.0, 4.0, 0.0));
        let diagonal = aabb.diagonal();
        assert!((diagonal - 5.0).abs() < 0.001); // 3-4-5 triangle
    }

    #[test]
    fn test_aabb_contains_inside() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(aabb.contains(Vec3::ZERO));
        assert!(aabb.contains(Vec3::new(0.5, 0.5, 0.5)));
    }

    #[test]
    fn test_aabb_contains_boundary() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(aabb.contains(Vec3::new(1.0, 1.0, 1.0)));
        assert!(aabb.contains(Vec3::new(-1.0, -1.0, -1.0)));
    }

    #[test]
    fn test_aabb_contains_outside() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(!aabb.contains(Vec3::new(2.0, 0.0, 0.0)));
        assert!(!aabb.contains(Vec3::new(0.0, -2.0, 0.0)));
        assert!(!aabb.contains(Vec3::new(0.0, 0.0, 1.01)));
    }

    #[test]
    fn test_aabb_merge() {
        let aabb1 = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let aabb2 = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(3.0, 3.0, 3.0));
        let merged = aabb1.merge(&aabb2);

        assert_eq!(merged.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(merged.max, Vec3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn test_aabb_from_points() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [-1.0, -2.0, -3.0]];
        let aabb = AABB::from_points(&points);
        assert_eq!(aabb.min, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_aabb_from_single_point() {
        let points = vec![[5.0, 5.0, 5.0]];
        let aabb = AABB::from_points(&points);
        assert_eq!(aabb.min, aabb.max);
        assert_eq!(aabb.diagonal(), 0.0);
    }
}

// ============================================================================
// BoundingCone Tests (from nanite_preprocess)
// ============================================================================

mod bounding_cone_tests {
    use super::*;
    use crate::nanite_preprocess::BoundingCone;

    #[test]
    fn test_bounding_cone_empty_triangles() {
        let positions: Vec<[f32; 3]> = vec![];
        let normals: Vec<[f32; 3]> = vec![];
        let indices: Vec<u32> = vec![];

        let cone = BoundingCone::from_triangles(&positions, &normals, &indices);
        assert_eq!(cone.apex, Vec3::ZERO);
        assert_eq!(cone.axis, Vec3::Z);
        assert_eq!(cone.cutoff, -1.0);
    }

    #[test]
    fn test_bounding_cone_cutoff_range() {
        // Cutoff should be in range [-1, 1] (cosine value)
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let indices = vec![0, 1, 2];

        let cone = BoundingCone::from_triangles(&positions, &normals, &indices);
        assert!(cone.cutoff >= -1.0 && cone.cutoff <= 1.0);
    }

    #[test]
    fn test_bounding_cone_is_backfacing() {
        let cone = BoundingCone {
            apex: Vec3::ZERO,
            axis: Vec3::Z,
            cutoff: 0.0,
        };

        // Looking opposite to cone axis
        assert!(cone.is_backfacing(-Vec3::Z));
    }

    #[test]
    fn test_bounding_cone_is_not_backfacing() {
        let cone = BoundingCone {
            apex: Vec3::ZERO,
            axis: Vec3::Z,
            cutoff: -0.5,
        };

        // Looking along cone axis (should be visible)
        assert!(!cone.is_backfacing(Vec3::Z));
    }
}

// ============================================================================
// Meshlet Constants Tests
// ============================================================================

mod meshlet_constants_tests {
    use crate::nanite_preprocess::{MAX_MESHLET_TRIANGLES, MAX_MESHLET_VERTICES};

    #[test]
    fn test_max_meshlet_vertices_value() {
        assert_eq!(MAX_MESHLET_VERTICES, 64);
        assert!(MAX_MESHLET_VERTICES > 0);
        assert!(MAX_MESHLET_VERTICES <= 256);
    }

    #[test]
    fn test_max_meshlet_triangles_value() {
        assert_eq!(MAX_MESHLET_TRIANGLES, 124);
        assert!(MAX_MESHLET_TRIANGLES > 0);
        assert!(MAX_MESHLET_TRIANGLES <= 256);
    }

    #[test]
    fn test_meshlet_constants_relationship() {
        // Triangles should be less than or around 2x vertices (typical ratio)
        assert!(MAX_MESHLET_TRIANGLES <= MAX_MESHLET_VERTICES * 3);
    }
}

// ============================================================================
// Meshlet Structure Tests
// ============================================================================

mod meshlet_tests {
    use crate::nanite_preprocess::{BoundingCone, Meshlet, AABB};
    use glam::Vec3;

    #[test]
    fn test_meshlet_vertex_count_method() {
        let meshlet = Meshlet {
            vertices: vec![0, 1, 2, 3, 4],
            indices: vec![0, 1, 2, 1, 2, 3],
            bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
            cone: BoundingCone {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: 0.5,
            },
            lod_level: 0,
            lod_error: 0.0,
            parent_index: None,
        };

        assert_eq!(meshlet.vertex_count(), 5);
    }

    #[test]
    fn test_meshlet_triangle_count_method() {
        let meshlet = Meshlet {
            vertices: vec![0, 1, 2, 3],
            indices: vec![0, 1, 2, 1, 2, 3],
            bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
            cone: BoundingCone {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: 0.5,
            },
            lod_level: 0,
            lod_error: 0.0,
            parent_index: None,
        };

        // 6 indices / 3 = 2 triangles
        assert_eq!(meshlet.triangle_count(), 2);
    }

    #[test]
    fn test_meshlet_empty() {
        let meshlet = Meshlet {
            vertices: vec![],
            indices: vec![],
            bounds: AABB::new(Vec3::ZERO, Vec3::ZERO),
            cone: BoundingCone {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: -1.0,
            },
            lod_level: 0,
            lod_error: 0.0,
            parent_index: None,
        };

        assert_eq!(meshlet.vertex_count(), 0);
        assert_eq!(meshlet.triangle_count(), 0);
    }

    #[test]
    fn test_meshlet_lod_level() {
        let lod0 = Meshlet {
            vertices: vec![0],
            indices: vec![],
            bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
            cone: BoundingCone {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: 0.0,
            },
            lod_level: 0,
            lod_error: 0.0,
            parent_index: None,
        };

        let lod1 = Meshlet {
            vertices: vec![0],
            indices: vec![],
            bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
            cone: BoundingCone {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: 0.0,
            },
            lod_level: 1,
            lod_error: 0.01,
            parent_index: Some(0),
        };

        assert_eq!(lod0.lod_level, 0);
        assert_eq!(lod1.lod_level, 1);
        assert!(lod0.lod_error <= lod1.lod_error);
    }

    #[test]
    fn test_meshlet_parent_index() {
        let root = Meshlet {
            vertices: vec![],
            indices: vec![],
            bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
            cone: BoundingCone {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: 0.0,
            },
            lod_level: 0,
            lod_error: 0.0,
            parent_index: None,
        };

        let child = Meshlet {
            vertices: vec![],
            indices: vec![],
            bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
            cone: BoundingCone {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: 0.0,
            },
            lod_level: 1,
            lod_error: 0.01,
            parent_index: Some(0),
        };

        assert!(root.parent_index.is_none());
        assert_eq!(child.parent_index, Some(0));
    }
}

// ============================================================================
// AssetDatabase Tests
// ============================================================================

mod asset_database_tests {
    use super::*;

    #[test]
    fn test_asset_database_new() {
        let db = AssetDatabase::new();
        assert!(db.assets.is_empty());
        assert!(db.path_to_guid.is_empty());
        assert!(db.dependency_graph.is_empty());
        assert!(db.reverse_deps.is_empty());
    }

    #[test]
    fn test_asset_database_default() {
        let db = AssetDatabase::default();
        assert!(db.assets.is_empty());
        assert!(db.path_to_guid.is_empty());
    }

    #[test]
    fn test_get_asset_nonexistent() {
        let db = AssetDatabase::new();
        assert!(db.get_asset("nonexistent-guid").is_none());
    }

    #[test]
    fn test_get_guid_by_path_nonexistent() {
        let db = AssetDatabase::new();
        let path = std::path::Path::new("/nonexistent/path.txt");
        assert!(db.get_guid_by_path(path).is_none());
    }

    #[test]
    fn test_get_dependents_nonexistent() {
        let db = AssetDatabase::new();
        assert!(db.get_dependents("nonexistent-guid").is_none());
    }

    #[test]
    fn test_get_dependencies_nonexistent() {
        let db = AssetDatabase::new();
        assert!(db.get_dependencies("nonexistent-guid").is_none());
    }
}

// ============================================================================
// AssetMetadata Tests
// ============================================================================

mod asset_metadata_tests {
    use super::*;

    #[test]
    fn test_asset_metadata_creation() {
        let meta = AssetMetadata {
            guid: "test-guid-123".to_string(),
            path: "/assets/test.mesh".to_string(),
            kind: AssetKind::Mesh,
            hash: "abc123hash".to_string(),
            dependencies: vec![],
            last_modified: 1234567890,
            size_bytes: 1024,
        };

        assert_eq!(meta.guid, "test-guid-123");
        assert_eq!(meta.path, "/assets/test.mesh");
        assert!(matches!(meta.kind, AssetKind::Mesh));
        assert_eq!(meta.hash, "abc123hash");
        assert!(meta.dependencies.is_empty());
        assert_eq!(meta.last_modified, 1234567890);
        assert_eq!(meta.size_bytes, 1024);
    }

    #[test]
    fn test_asset_metadata_with_dependencies() {
        let meta = AssetMetadata {
            guid: "child-guid".to_string(),
            path: "/assets/child.mesh".to_string(),
            kind: AssetKind::Mesh,
            hash: "hash".to_string(),
            dependencies: vec!["parent-1".to_string(), "parent-2".to_string()],
            last_modified: 0,
            size_bytes: 0,
        };

        assert_eq!(meta.dependencies.len(), 2);
        assert!(meta.dependencies.contains(&"parent-1".to_string()));
        assert!(meta.dependencies.contains(&"parent-2".to_string()));
    }

    #[test]
    fn test_asset_metadata_clone() {
        let original = AssetMetadata {
            guid: "guid".to_string(),
            path: "/path".to_string(),
            kind: AssetKind::Texture,
            hash: "hash".to_string(),
            dependencies: vec!["dep".to_string()],
            last_modified: 999,
            size_bytes: 512,
        };

        let cloned = original.clone();
        assert_eq!(original.guid, cloned.guid);
        assert_eq!(original.path, cloned.path);
        assert_eq!(original.hash, cloned.hash);
        assert_eq!(original.last_modified, cloned.last_modified);
        assert_eq!(original.size_bytes, cloned.size_bytes);
    }

    #[test]
    fn test_asset_metadata_size_bytes_boundary() {
        // Test zero size
        let meta_zero = AssetMetadata {
            guid: "".to_string(),
            path: "".to_string(),
            kind: AssetKind::Other,
            hash: "".to_string(),
            dependencies: vec![],
            last_modified: 0,
            size_bytes: 0,
        };
        assert_eq!(meta_zero.size_bytes, 0);

        // Test large size
        let meta_large = AssetMetadata {
            guid: "".to_string(),
            path: "".to_string(),
            kind: AssetKind::Other,
            hash: "".to_string(),
            dependencies: vec![],
            last_modified: 0,
            size_bytes: u64::MAX,
        };
        assert_eq!(meta_large.size_bytes, u64::MAX);
    }
}

// ============================================================================
// gltf_loader Module Tests
// ============================================================================

mod gltf_loader_tests {
    use crate::gltf_loader::{ImageData, MaterialData, MeshData};

    #[test]
    fn test_mesh_data_empty_positions() {
        let mesh = MeshData {
            positions: vec![],
            normals: vec![],
            tangents: vec![],
            texcoords: vec![],
            indices: vec![],
        };
        assert!(mesh.positions.is_empty());
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn test_mesh_data_positions() {
        let mesh = MeshData {
            positions: vec![[0.0, 1.0, 2.0], [3.0, 4.0, 5.0]],
            normals: vec![],
            tangents: vec![],
            texcoords: vec![],
            indices: vec![],
        };
        assert_eq!(mesh.positions.len(), 2);
        assert_eq!(mesh.positions[0], [0.0, 1.0, 2.0]);
        assert_eq!(mesh.positions[1], [3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_mesh_data_with_texcoords() {
        let mesh = MeshData {
            positions: vec![],
            normals: vec![],
            tangents: vec![],
            texcoords: vec![[0.0, 1.0], [0.5, 0.5]],
            indices: vec![],
        };
        assert_eq!(mesh.texcoords.len(), 2);
        assert_eq!(mesh.texcoords[0], [0.0, 1.0]);
    }

    #[test]
    fn test_mesh_data_indices() {
        let mesh = MeshData {
            positions: vec![],
            normals: vec![],
            tangents: vec![],
            texcoords: vec![],
            indices: vec![0, 1, 2, 2, 1, 3],
        };
        assert_eq!(mesh.indices.len(), 6);
        assert_eq!(mesh.indices[0], 0);
        assert_eq!(mesh.indices[5], 3);
    }

    #[test]
    fn test_material_data_default() {
        let mat = MaterialData::default();
        // Default should have PBR-correct values
        assert_eq!(mat.metallic_factor, 1.0);
        assert_eq!(mat.roughness_factor, 1.0);
        assert!(mat.base_color_texture.is_none());
    }

    #[test]
    fn test_material_data_base_color_factor() {
        let mat = MaterialData {
            base_color_factor: [1.0, 0.5, 0.25, 1.0],
            metallic_factor: 0.0,
            roughness_factor: 0.0,
            base_color_texture: None,
            metallic_roughness_texture: None,
            normal_texture: None,
            ..Default::default()
        };
        assert_eq!(mat.base_color_factor[0], 1.0);
        assert_eq!(mat.base_color_factor[1], 0.5);
        assert_eq!(mat.base_color_factor[2], 0.25);
        assert_eq!(mat.base_color_factor[3], 1.0);
    }

    #[test]
    fn test_material_data_metallic_roughness() {
        let mat = MaterialData {
            base_color_factor: [1.0; 4],
            metallic_factor: 1.0,
            roughness_factor: 0.3,
            base_color_texture: None,
            metallic_roughness_texture: None,
            normal_texture: None,
            ..Default::default()
        };
        assert_eq!(mat.metallic_factor, 1.0);
        assert_eq!(mat.roughness_factor, 0.3);
    }

    #[test]
    fn test_image_data_creation() {
        let img = ImageData {
            width: 256,
            height: 256,
            rgba8: vec![0u8; 256 * 256 * 4],
        };
        assert_eq!(img.width, 256);
        assert_eq!(img.height, 256);
        assert_eq!(img.rgba8.len(), 256 * 256 * 4);
    }

    #[test]
    fn test_image_data_dimensions() {
        let img = ImageData {
            width: 1024,
            height: 512,
            rgba8: vec![],
        };
        assert_eq!(img.width, 1024);
        assert_eq!(img.height, 512);
        assert!(img.width != img.height); // Non-square
    }

    #[test]
    fn test_mesh_data_clone() {
        let mesh = MeshData {
            positions: vec![[1.0, 2.0, 3.0]],
            normals: vec![[0.0, 1.0, 0.0]],
            tangents: vec![[1.0, 0.0, 0.0, 1.0]],
            texcoords: vec![[0.5, 0.5]],
            indices: vec![0],
        };

        let cloned = mesh.clone();
        assert_eq!(mesh.positions, cloned.positions);
        assert_eq!(mesh.normals, cloned.normals);
        assert_eq!(mesh.tangents, cloned.tangents);
        assert_eq!(mesh.texcoords, cloned.texcoords);
        assert_eq!(mesh.indices, cloned.indices);
    }
}

// ============================================================================
// Cell Loader Tests
// ============================================================================

mod cell_loader_tests {
    use crate::cell_loader::{
        AssetKind as CellAssetKind, AssetRef, CellData, CellMetadata, EntityData,
    };

    #[test]
    fn test_cell_data_new() {
        let cell = CellData::new([0, 0, 0]);
        assert_eq!(cell.coord, [0, 0, 0]);
        assert!(cell.entities.is_empty());
        assert!(cell.assets.is_empty());
        assert!(cell.metadata.is_none());
    }

    #[test]
    fn test_cell_data_coordinates() {
        let cell = CellData::new([-10, 20, 5]);
        assert_eq!(cell.coord[0], -10);
        assert_eq!(cell.coord[1], 20);
        assert_eq!(cell.coord[2], 5);
        // Coordinates can be negative
        assert!(cell.coord[0] < 0);
        assert!(cell.coord[1] > 0);
    }

    #[test]
    fn test_cell_data_add_entity() {
        let mut cell = CellData::new([0, 0, 0]);
        let entity = EntityData::new([0.0, 0.0, 0.0]).with_name("test_entity");

        cell.add_entity(entity);
        assert_eq!(cell.entities.len(), 1);
        assert_eq!(cell.entities[0].name, Some("test_entity".to_string()));
    }

    #[test]
    fn test_cell_data_add_asset() {
        let mut cell = CellData::new([0, 0, 0]);
        let asset = AssetRef::new("assets/mesh.glb", CellAssetKind::Mesh);

        cell.add_asset(asset);
        assert_eq!(cell.assets.len(), 1);
    }

    #[test]
    fn test_cell_data_add_duplicate_asset() {
        let mut cell = CellData::new([0, 0, 0]);
        let asset1 = AssetRef::new("assets/mesh.glb", CellAssetKind::Mesh);
        let asset2 = AssetRef::new("assets/mesh.glb", CellAssetKind::Mesh);

        cell.add_asset(asset1);
        cell.add_asset(asset2);
        // Duplicates should not be added
        assert_eq!(cell.assets.len(), 1);
    }

    #[test]
    fn test_cell_data_memory_estimate() {
        let cell = CellData::new([0, 0, 0]);
        let estimate = cell.memory_estimate();
        assert!(estimate > 0);
    }

    #[test]
    fn test_cell_metadata_creation() {
        let meta = CellMetadata {
            description: Some("Test cell".to_string()),
            tags: vec!["forest".to_string(), "spawn".to_string()],
            version: 1,
        };

        assert_eq!(meta.description, Some("Test cell".to_string()));
        assert_eq!(meta.tags.len(), 2);
        assert_eq!(meta.version, 1);
    }

    #[test]
    fn test_entity_data_creation() {
        let entity = EntityData::new([1.0, 2.0, 3.0])
            .with_name("player_spawn")
            .with_mesh("meshes/player.glb");

        assert_eq!(entity.name, Some("player_spawn".to_string()));
        assert!(entity.mesh.is_some());
        assert_eq!(entity.position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_entity_data_default_rotation() {
        let entity = EntityData::new([0.0, 0.0, 0.0]);
        // Default rotation should be identity quaternion
        assert_eq!(entity.rotation, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_entity_data_default_scale() {
        let entity = EntityData::new([0.0, 0.0, 0.0]);
        // Default scale should be uniform 1
        assert_eq!(entity.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_asset_ref_new() {
        let asset = AssetRef::new("path/to/asset.glb", CellAssetKind::Mesh);
        assert_eq!(asset.path, "path/to/asset.glb");
        assert!(matches!(asset.kind, CellAssetKind::Mesh));
        assert!(asset.guid.is_none());
    }

    #[test]
    fn test_asset_ref_with_guid() {
        let asset =
            AssetRef::new("path/to/asset.glb", CellAssetKind::Texture).with_guid("some-guid-123");
        assert_eq!(asset.guid, Some("some-guid-123".to_string()));
    }

    #[test]
    fn test_cell_asset_kind_variants() {
        let mesh = CellAssetKind::Mesh;
        let tex = CellAssetKind::Texture;
        let mat = CellAssetKind::Material;
        let audio = CellAssetKind::Audio;
        let anim = CellAssetKind::Animation;
        let other = CellAssetKind::Other;

        assert!(matches!(mesh, CellAssetKind::Mesh));
        assert!(matches!(tex, CellAssetKind::Texture));
        assert!(matches!(mat, CellAssetKind::Material));
        assert!(matches!(audio, CellAssetKind::Audio));
        assert!(matches!(anim, CellAssetKind::Animation));
        assert!(matches!(other, CellAssetKind::Other));
    }

    #[test]
    fn test_cell_asset_kind_equality() {
        assert_eq!(CellAssetKind::Mesh, CellAssetKind::Mesh);
        assert_ne!(CellAssetKind::Mesh, CellAssetKind::Texture);
    }
}

// ============================================================================
// Hash and GUID Tests
// ============================================================================

mod hash_tests {
    use super::*;

    #[test]
    fn test_guid_for_path_deterministic() {
        let path1 = "/assets/mesh.obj";
        let path2 = "/assets/mesh.obj";

        let guid1 = guid_for_path(path1);
        let guid2 = guid_for_path(path2);

        assert_eq!(guid1, guid2);
    }

    #[test]
    fn test_guid_for_path_unique_for_different_paths() {
        let guid1 = guid_for_path("/assets/mesh1.obj");
        let guid2 = guid_for_path("/assets/mesh2.obj");

        assert_ne!(guid1, guid2);
    }

    #[test]
    fn test_guid_not_empty() {
        let guid = guid_for_path("/some/path.txt");
        assert!(!guid.is_empty());
    }

    #[test]
    fn test_guid_format() {
        let guid = guid_for_path("/test");
        // GUID should be a hex string (32 chars from SHA256 truncated)
        assert_eq!(guid.len(), 32);
        assert!(guid.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_guid_case_insensitive() {
        let guid1 = guid_for_path("Assets/Characters/Hero.gltf");
        let guid2 = guid_for_path("assets/characters/hero.gltf");
        assert_eq!(guid1, guid2);
    }

    #[test]
    fn test_guid_path_separator_normalization() {
        let guid1 = guid_for_path("assets/characters/hero.gltf");
        let guid2 = guid_for_path("assets\\characters\\hero.gltf");
        assert_eq!(guid1, guid2);
    }
}

// ============================================================================
// Behavioral Correctness Tests - Asset System Invariants
// ============================================================================

#[cfg(test)]
mod behavioral_correctness_tests {
    use super::*;
    use crate::cell_loader::{AssetKind as CellAssetKind, AssetRef, CellData, EntityData};

    #[test]
    fn test_guid_is_deterministic() {
        // Behavioral: same path must always produce same GUID
        let path = "assets/models/character.glb";
        let guid1 = guid_for_path(path);
        let guid2 = guid_for_path(path);
        let guid3 = guid_for_path(path);

        assert_eq!(guid1, guid2, "GUID must be deterministic");
        assert_eq!(guid2, guid3, "GUID must be deterministic");
    }

    #[test]
    fn test_guid_different_for_different_paths() {
        // Behavioral: different paths must produce different GUIDs
        let paths = [
            "assets/a.glb",
            "assets/b.glb",
            "assets/c.glb",
            "models/a.glb",
            "textures/a.png",
        ];

        let guids: Vec<_> = paths.iter().map(|p| guid_for_path(p)).collect();

        for i in 0..guids.len() {
            for j in (i + 1)..guids.len() {
                assert_ne!(
                    guids[i], guids[j],
                    "Different paths must have different GUIDs: {} vs {}",
                    paths[i], paths[j]
                );
            }
        }
    }

    #[test]
    fn test_guid_length_is_consistent() {
        // Behavioral: all GUIDs should have same length
        let guids: Vec<_> = [
            "a.txt",
            "very/long/path/to/some/deeply/nested/file.glb",
            "x",
        ]
        .iter()
        .map(|p| guid_for_path(p))
        .collect();

        let expected_len = guids[0].len();
        for guid in &guids {
            assert_eq!(
                guid.len(),
                expected_len,
                "All GUIDs should have consistent length"
            );
        }
    }

    #[test]
    fn test_entity_data_identity_rotation_is_default() {
        // Behavioral: default rotation should be identity quaternion
        let entity = EntityData::new([0.0, 0.0, 0.0]);

        // Identity quaternion: (0, 0, 0, 1)
        assert_eq!(entity.rotation[0], 0.0, "X should be 0");
        assert_eq!(entity.rotation[1], 0.0, "Y should be 0");
        assert_eq!(entity.rotation[2], 0.0, "Z should be 0");
        assert_eq!(entity.rotation[3], 1.0, "W should be 1");
    }

    #[test]
    fn test_entity_data_uniform_scale_is_default() {
        // Behavioral: default scale should be uniform 1
        let entity = EntityData::new([0.0, 0.0, 0.0]);

        assert_eq!(entity.scale[0], 1.0, "X scale should be 1");
        assert_eq!(entity.scale[1], 1.0, "Y scale should be 1");
        assert_eq!(entity.scale[2], 1.0, "Z scale should be 1");
    }

    #[test]
    fn test_cell_data_position_matches_constructor() {
        // Behavioral: coord should match what was passed to constructor
        let pos = [42, -17, 99];
        let cell = CellData::new(pos);

        assert_eq!(cell.coord, pos, "Cell coord must match constructor");
    }

    #[test]
    fn test_cell_data_memory_estimate_is_positive() {
        // Behavioral: memory estimate should always be positive
        let cell = CellData::new([0, 0, 0]);
        let estimate = cell.memory_estimate();

        assert!(estimate > 0, "Memory estimate must be positive");
    }

    #[test]
    fn test_asset_kind_equality_is_reflexive() {
        // Behavioral: every value should equal itself
        let kinds = [
            AssetKind::Mesh,
            AssetKind::Texture,
            AssetKind::Audio,
            AssetKind::Material,
            AssetKind::Animation,
            AssetKind::Script,
            AssetKind::Dialogue,
            AssetKind::BlenderSource,
            AssetKind::Other,
        ];

        for kind in kinds {
            assert_eq!(kind.clone(), kind, "AssetKind equality must be reflexive");
        }
    }

    #[test]
    fn test_asset_ref_preserves_path() {
        // Behavioral: path should be preserved exactly
        let path = "assets/models/hero.glb";
        let asset = AssetRef::new(path, CellAssetKind::Mesh);

        assert_eq!(asset.path, path, "Asset path must be preserved exactly");
    }

    #[test]
    fn test_asset_ref_guid_is_optional() {
        // Behavioral: GUID should be None by default
        let asset = AssetRef::new("test.glb", CellAssetKind::Mesh);
        assert!(asset.guid.is_none(), "GUID should be None by default");
    }

    #[test]
    fn test_asset_ref_with_guid_sets_guid() {
        // Behavioral: with_guid should set the GUID
        let guid = "abc123";
        let asset = AssetRef::new("test.glb", CellAssetKind::Mesh).with_guid(guid);

        assert_eq!(
            asset.guid,
            Some(guid.to_string()),
            "with_guid should set GUID"
        );
    }
}
