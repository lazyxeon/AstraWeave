//! End-to-end integration test for the Blend → BiomePack → Scatter pipeline.
//!
//! This test exercises the full import workflow using mock manifest data
//! (no Blender installation required):
//!
//! 1. Parse a realistic `manifest.json` (simulating Namaqualand.blend decomposition)
//! 2. Build a `BiomePack` from the manifest
//! 3. Convert to `BiomeConfig` + `ScatterConfig`
//! 4. Verify scatter entries, weights, ground texture detection

use astraweave_terrain::BiomePack;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Build a realistic Namaqualand-style manifest.json in a temp directory.
///
/// Returns the path to the manifest.json file.
fn build_namaqualand_manifest(dir: &std::path::Path) -> PathBuf {
    // Create mesh directory
    let meshes = dir.join("meshes");
    std::fs::create_dir_all(&meshes).unwrap();
    // Create stub glb files
    for name in &[
        "QuiverTree_01.glb",
        "QuiverTree_02.glb",
        "Succulent_Small_01.glb",
        "Succulent_Small_02.glb",
        "Flower_Daisy_01.glb",
        "Boulder_Large_01.glb",
        "Boulder_Medium_01.glb",
        "Stone_Small_01.glb",
        "Cliff_Section_01.glb",
        "Terrain_Ground_01.glb",
    ] {
        std::fs::write(meshes.join(name), b"stub").unwrap();
    }

    // Create texture directory with ground textures
    let textures = dir.join("textures");
    std::fs::create_dir_all(&textures).unwrap();
    for name in &[
        "cliff_diffuse.png",
        "cliff_normal.png",
        "cliff_roughness.png",
        "sand_diffuse.png",
        "sand_normal.png",
        "sand_roughness.png",
        "gravel_diffuse.png",
        "gravel_normal.png",
    ] {
        std::fs::write(textures.join(name), b"stub").unwrap();
    }

    // Create HDRI directory
    let hdri = dir.join("hdri");
    std::fs::create_dir_all(&hdri).unwrap();
    std::fs::write(hdri.join("namaqualand_sunset.hdr"), b"stub").unwrap();
    std::fs::write(hdri.join("namaqualand_overcast.hdr"), b"stub").unwrap();

    // Build realistic manifest JSON matching RawManifest schema:
    //   - top-level: blend_hash, source_file, total_objects, assets, hdris
    //   - assets[]: name, filename, category, vertex_count, dimensions, textures, materials, collections
    //   - textures[]: filename, channel, width, height
    //   - hdris[]: filename, original_name, width, height
    let manifest = serde_json::json!({
        "blend_hash": "e2e_namaqualand_test_hash_abc123",
        "source_file": "Namaqualand.blend",
        "total_objects": 10,
        "assets": [
            {
                "name": "QuiverTree_01",
                "filename": "meshes/QuiverTree_01.glb",
                "category": "vegetation",
                "vertex_count": 8500,
                "dimensions": [3.0, 6.2, 3.0],
                "textures": [
                    { "filename": "quivertree_diffuse.png", "channel": "diffuse", "width": 2048, "height": 2048 },
                    { "filename": "quivertree_normal.png", "channel": "normal", "width": 2048, "height": 2048 },
                    { "filename": "quivertree_roughness.png", "channel": "roughness", "width": 1024, "height": 1024 }
                ],
                "materials": ["QuiverTreeBark"],
                "collections": ["Trees"]
            },
            {
                "name": "QuiverTree_02",
                "filename": "meshes/QuiverTree_02.glb",
                "category": "vegetation",
                "vertex_count": 7200,
                "dimensions": [2.4, 5.0, 2.4],
                "textures": [
                    { "filename": "quivertree_diffuse.png", "channel": "diffuse", "width": 2048, "height": 2048 }
                ],
                "materials": ["QuiverTreeBark"],
                "collections": ["Trees"]
            },
            {
                "name": "Succulent_Small_01",
                "filename": "meshes/Succulent_Small_01.glb",
                "category": "vegetation",
                "vertex_count": 1200,
                "dimensions": [0.3, 0.2, 0.3],
                "textures": [
                    { "filename": "succulent_diffuse.png", "channel": "diffuse", "width": 512, "height": 512 }
                ],
                "materials": ["SucculentMat"],
                "collections": ["Succulents"]
            },
            {
                "name": "Succulent_Small_02",
                "filename": "meshes/Succulent_Small_02.glb",
                "category": "vegetation",
                "vertex_count": 900,
                "dimensions": [0.2, 0.15, 0.2],
                "textures": [],
                "materials": ["SucculentMat"],
                "collections": ["Succulents"]
            },
            {
                "name": "Flower_Daisy_01",
                "filename": "meshes/Flower_Daisy_01.glb",
                "category": "vegetation",
                "vertex_count": 400,
                "dimensions": [0.1, 0.12, 0.1],
                "textures": [],
                "materials": ["FlowerMat"],
                "collections": ["Flowers"]
            },
            {
                "name": "Boulder_Large_01",
                "filename": "meshes/Boulder_Large_01.glb",
                "category": "rock",
                "vertex_count": 3200,
                "dimensions": [5.0, 3.0, 4.0],
                "textures": [
                    { "filename": "boulder_diffuse.png", "channel": "diffuse", "width": 2048, "height": 2048 },
                    { "filename": "boulder_normal.png", "channel": "normal", "width": 2048, "height": 2048 }
                ],
                "materials": ["RockMat"],
                "collections": ["Rocks"]
            },
            {
                "name": "Boulder_Medium_01",
                "filename": "meshes/Boulder_Medium_01.glb",
                "category": "rock",
                "vertex_count": 1800,
                "dimensions": [1.6, 0.7, 1.6],
                "textures": [
                    { "filename": "boulder_diffuse.png", "channel": "diffuse", "width": 1024, "height": 1024 }
                ],
                "materials": ["RockMat"],
                "collections": ["Rocks"]
            },
            {
                "name": "Stone_Small_01",
                "filename": "meshes/Stone_Small_01.glb",
                "category": "rock",
                "vertex_count": 500,
                "dimensions": [0.3, 0.1, 0.3],
                "textures": [],
                "materials": ["StoneMat"],
                "collections": ["Rocks"]
            },
            {
                "name": "Cliff_Section_01",
                "filename": "meshes/Cliff_Section_01.glb",
                "category": "rock",
                "vertex_count": 6000,
                "dimensions": [10.0, 8.0, 2.0],
                "textures": [
                    { "filename": "cliff_diffuse.png", "channel": "diffuse", "width": 4096, "height": 4096 },
                    { "filename": "cliff_normal.png", "channel": "normal", "width": 4096, "height": 4096 }
                ],
                "materials": ["CliffMat"],
                "collections": ["Rocks"]
            },
            {
                "name": "Terrain_Ground_01",
                "filename": "meshes/Terrain_Ground_01.glb",
                "category": "terrain",
                "vertex_count": 25000,
                "dimensions": [100.0, 2.0, 100.0],
                "textures": [
                    { "filename": "sand_diffuse.png", "channel": "diffuse", "width": 4096, "height": 4096 },
                    { "filename": "sand_normal.png", "channel": "normal", "width": 4096, "height": 4096 }
                ],
                "materials": ["TerrainMat"],
                "collections": ["Terrain"]
            }
        ],
        "hdris": [
            { "filename": "namaqualand_sunset.hdr", "original_name": "Namaqualand Sunset", "width": 4096, "height": 2048 },
            { "filename": "namaqualand_overcast.hdr", "original_name": "Namaqualand Overcast", "width": 4096, "height": 2048 }
        ]
    });

    let manifest_path = dir.join("manifest.json");
    let mut f = std::fs::File::create(&manifest_path).unwrap();
    f.write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())
        .unwrap();

    manifest_path
}

// ============================================================================
// TESTS
// ============================================================================

#[test]
fn e2e_manifest_to_biome_pack() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    // Step 1: Parse manifest into BiomePack
    let pack = BiomePack::from_manifest(&manifest_path).expect("Failed to parse manifest");

    // Validate pack metadata
    assert!(!pack.name.is_empty());
    assert!(!pack.blend_hash.is_empty());
    assert_eq!(pack.assets.len(), 10);
    assert_eq!(pack.hdris.len(), 2);
}

#[test]
fn e2e_biome_pack_asset_classification() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();

    // Vegetation assets
    let vegetation = pack.assets_by_category("vegetation");
    assert_eq!(vegetation.len(), 5, "Expected 5 vegetation assets");

    // Rock assets
    let rocks = pack.assets_by_category("rock");
    assert_eq!(rocks.len(), 4, "Expected 4 rock assets");

    // Terrain assets
    let terrain = pack.assets_by_category("terrain");
    assert_eq!(terrain.len(), 1, "Expected 1 terrain asset");

    // Verify quiver trees are classified as large vegetation (height > 1.0m)
    let quiver1 = vegetation
        .iter()
        .find(|a| a.name == "QuiverTree_01")
        .expect("Should find QuiverTree_01");
    assert_eq!(quiver1.vertex_count, 8500);
    let dims = quiver1
        .dimensions
        .expect("QuiverTree should have dimensions");
    assert!(
        dims[1] > 1.0,
        "QuiverTree should be tall (height={})",
        dims[1]
    );

    // Verify succulents are small vegetation
    let succulent = vegetation
        .iter()
        .find(|a| a.name == "Succulent_Small_01")
        .expect("Should find Succulent_Small_01");
    let dims = succulent
        .dimensions
        .expect("Succulent should have dimensions");
    assert!(
        dims[1] < 1.0,
        "Succulent should be short (height={})",
        dims[1]
    );
}

#[test]
fn e2e_biome_pack_scatter_weights() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();

    // Large vegetation (QuiverTrees) should have low weight (sparse placement)
    let quiver = pack
        .assets
        .iter()
        .find(|a| a.name == "QuiverTree_01")
        .unwrap();
    assert!(
        quiver.weight < 0.1,
        "Large vegetation should have low weight, got {}",
        quiver.weight
    );

    // Small vegetation (succulents, flowers) should have higher weight (dense)
    let succulent = pack
        .assets
        .iter()
        .find(|a| a.name == "Succulent_Small_01")
        .unwrap();
    assert!(
        succulent.weight > 0.5,
        "Small vegetation should have high weight, got {}",
        succulent.weight
    );

    // Boulder_Large_01 has dimensions [5.0, 3.0, 4.0] (max_dim=5.0),
    // which classifies as Medium rock (threshold is >5.0 for Large).
    // Medium rocks get weight 0.15.
    let boulder = pack
        .assets
        .iter()
        .find(|a| a.name == "Boulder_Large_01")
        .unwrap();
    assert!(
        boulder.weight <= 0.2,
        "Medium rock should have moderate weight, got {}",
        boulder.weight
    );

    // Small rocks should have moderate weight
    let stone = pack
        .assets
        .iter()
        .find(|a| a.name == "Stone_Small_01")
        .unwrap();
    assert!(
        stone.weight > 0.1,
        "Small rocks should have moderate weight, got {}",
        stone.weight
    );

    // Terrain should have zero weight (not scattered)
    let terrain = pack
        .assets
        .iter()
        .find(|a| a.name == "Terrain_Ground_01")
        .unwrap();
    assert!(
        terrain.weight < f32::EPSILON,
        "Terrain should have zero weight, got {}",
        terrain.weight
    );
}

#[test]
fn e2e_biome_pack_to_biome_config() {
    use astraweave_terrain::biome::BiomeType;

    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();
    let config = pack.to_biome_config(BiomeType::Desert);

    // BiomeConfig should have been created
    assert_eq!(config.biome_type, BiomeType::Desert);

    // Vegetation types should contain scatter-eligible assets (non-terrain)
    // to_biome_config filters to vegetation/rock/prop categories (excludes terrain)
    let veg_count = config.vegetation.vegetation_types.len();
    assert!(
        veg_count >= 1,
        "BiomeConfig should have at least 1 vegetation type, got {}",
        veg_count
    );

    // Density should be inherited from scatter params
    assert!(
        config.vegetation.density > 0.0,
        "Vegetation density should be positive"
    );

    // Conditions should be set (defaults from BiomeConditions::default())
    assert!(
        config.conditions.max_slope > 0.0,
        "Max slope should be positive"
    );
}

#[test]
fn e2e_biome_pack_to_scatter_config() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();
    let scatter = pack.to_scatter_config();

    // ScatterConfig should have Poisson disk enabled
    assert!(scatter.use_poisson_disk, "Should use Poisson disk");

    // Min distance should be positive
    assert!(
        scatter.min_distance > 0.0,
        "Min distance should be positive"
    );

    // Max slope should be reasonable
    assert!(
        scatter.max_slope > 0.0 && scatter.max_slope <= 90.0,
        "Max slope should be in valid range"
    );
}

#[test]
fn e2e_biome_config_vegetation_types() {
    use astraweave_terrain::biome::BiomeType;

    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();
    let config = pack.to_biome_config(BiomeType::Desert);

    // Vegetation types should include assets from vegetation and rock categories
    // but NOT terrain
    let veg_names: Vec<&str> = config
        .vegetation
        .vegetation_types
        .iter()
        .map(|v| v.name.as_str())
        .collect();

    assert!(
        veg_names.contains(&"QuiverTree_01"),
        "Should contain QuiverTree_01"
    );
    assert!(
        veg_names.contains(&"Boulder_Large_01"),
        "Should contain Boulder_Large_01 (rocks are scatter eligible)"
    );
    assert!(
        !veg_names.contains(&"Terrain_Ground_01"),
        "Should NOT contain Terrain_Ground_01"
    );

    // Model paths should be non-empty (joined with root_dir)
    for vt in &config.vegetation.vegetation_types {
        assert!(
            !vt.model_path.is_empty(),
            "Model path should not be empty for {}",
            vt.name
        );
    }
}

#[test]
fn e2e_ground_texture_detection() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();

    // The manifest directory has cliff, sand, and gravel textures
    assert!(
        !pack.ground_textures.is_empty(),
        "Should detect ground textures"
    );

    // Check that cliff textures were found
    let has_cliff = pack
        .ground_textures
        .iter()
        .any(|gt| gt.name.contains("cliff"));
    assert!(has_cliff, "Should detect cliff ground textures");

    // Check sand textures
    let has_sand = pack
        .ground_textures
        .iter()
        .any(|gt| gt.name.contains("sand"));
    assert!(has_sand, "Should detect sand ground textures");
}

#[test]
fn e2e_biome_pack_serde_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&pack).unwrap();
    assert!(!json.is_empty());

    // Deserialize back
    let pack2: BiomePack = serde_json::from_str(&json).unwrap();

    assert_eq!(pack.name, pack2.name);
    assert_eq!(pack.assets.len(), pack2.assets.len());
    assert_eq!(pack.hdris.len(), pack2.hdris.len());
    assert_eq!(pack.ground_textures.len(), pack2.ground_textures.len());

    // Verify asset weights survive roundtrip
    for (a, b) in pack.assets.iter().zip(pack2.assets.iter()) {
        assert_eq!(a.name, b.name);
        assert!((a.weight - b.weight).abs() < f32::EPSILON);
    }
}

#[test]
fn e2e_biome_pack_save_load() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();

    // Save to file
    let save_path = dir.join("namaqualand.biomepack.json");
    pack.save(&save_path).unwrap();

    assert!(save_path.exists(), "Pack file should exist after save");

    // Load back
    let loaded = BiomePack::load(&save_path).unwrap();

    assert_eq!(loaded.name, pack.name);
    assert_eq!(loaded.assets.len(), 10);
    assert_eq!(loaded.hdris.len(), 2);
}

#[test]
fn e2e_biome_config_from_pack_constructor() {
    use astraweave_terrain::biome::{BiomeConfig, BiomeType};

    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();

    // Use the convenience constructor
    let config = BiomeConfig::from_biome_pack(&pack, BiomeType::Desert);
    assert_eq!(config.biome_type, BiomeType::Desert);
}

#[test]
fn e2e_scatter_config_from_pack_constructor() {
    use astraweave_terrain::scatter::ScatterConfig;

    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    let pack = BiomePack::from_manifest(&manifest_path).unwrap();

    // Use the convenience constructor
    let scatter = ScatterConfig::from_biome_pack(&pack);
    assert!(scatter.use_poisson_disk);
    assert!(scatter.min_distance > 0.0);
}

#[test]
fn e2e_full_pipeline_summary() {
    // This test validates the complete pipeline flow:
    //   manifest.json → BiomePack → BiomeConfig + ScatterConfig
    // and verifies all critical invariants hold across the pipeline.

    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let manifest_path = build_namaqualand_manifest(dir);

    // Step 1: Parse manifest
    let pack = BiomePack::from_manifest(&manifest_path).unwrap();
    assert!(!pack.name.is_empty());

    // Step 2: Verify asset classification
    let total = pack.assets.len();
    let veg = pack.assets_by_category("vegetation").len();
    let rock = pack.assets_by_category("rock").len();
    let terrain = pack.assets_by_category("terrain").len();
    assert_eq!(total, veg + rock + terrain);
    assert_eq!(veg, 5);
    assert_eq!(rock, 4);
    assert_eq!(terrain, 1);

    // Step 3: Generate BiomeConfig
    let config = pack.to_biome_config(astraweave_terrain::biome::BiomeType::Desert);
    assert_eq!(
        config.biome_type,
        astraweave_terrain::biome::BiomeType::Desert
    );

    // Step 4: Generate ScatterConfig
    let scatter = pack.to_scatter_config();
    assert!(scatter.use_poisson_disk);
    assert!(scatter.min_distance > 0.0);

    // Step 5: Verify vegetation types exclude terrain
    let veg_type_count = config.vegetation.vegetation_types.len();
    // Should be veg + rock (non-terrain scatter-eligible) = 5 + 4 = 9
    assert_eq!(
        veg_type_count,
        veg + rock,
        "Vegetation types should include veg+rock but not terrain"
    );

    // Step 6: Save and reload
    let save_path = dir.join("pipeline_output.biomepack.json");
    pack.save(&save_path).unwrap();
    let reloaded = BiomePack::load(&save_path).unwrap();
    assert_eq!(reloaded.assets.len(), total);

    // Step 7: Verify ground textures
    assert!(!pack.ground_textures.is_empty());

    // Step 8: Verify HDRI extraction
    assert_eq!(pack.hdris.len(), 2);

    // Step 9: Verify scatter params are sensible
    assert!(pack.scatter.density > 0.0);
    assert!(pack.scatter.use_poisson_disk);
    assert!(pack.scatter.min_distance > 0.0);
}
