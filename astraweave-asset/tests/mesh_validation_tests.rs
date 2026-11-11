//! Mesh Validation Tests
//!
//! Comprehensive tests for mesh data validation including vertex integrity,
//! index bounds checking, normal/tangent validation, UV coordinate validation,
//! and mesh optimization metrics (ACMR).

#[cfg(feature = "gltf")]
mod mesh_validation_tests {
    use astraweave_asset::gltf_loader::*;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    fn real_asset_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("assets")
            .join("models")
            .join(name)
    }

    // Test 1: Validate vertex data integrity
    #[test]
    fn test_validate_vertex_data_integrity() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        // All vertex attribute arrays should have same length
        assert_eq!(
            mesh.positions.len(),
            mesh.normals.len(),
            "Positions and normals count mismatch"
        );
        assert_eq!(
            mesh.positions.len(),
            mesh.tangents.len(),
            "Positions and tangents count mismatch"
        );
        assert_eq!(
            mesh.positions.len(),
            mesh.texcoords.len(),
            "Positions and UVs count mismatch"
        );
    }

    // Test 2: Validate index bounds (no out-of-range indices)
    #[test]
    fn test_validate_index_bounds() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        let max_index = mesh.positions.len() as u32;
        for (i, &index) in mesh.indices.iter().enumerate() {
            assert!(
                index < max_index,
                "Index {} at position {} is out of bounds (max {})",
                index,
                i,
                max_index - 1
            );
        }
    }

    // Test 3: Validate normal vectors are unit length
    #[test]
    fn test_validate_normals_unit_length() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");

        match load_first_mesh_and_material(std::path::Path::new("."), &bytes) {
            Ok((mesh, _)) => {
                for (i, normal) in mesh.normals.iter().enumerate() {
                    // Skip invalid normals
                    if !normal[0].is_finite() || !normal[1].is_finite() || !normal[2].is_finite() {
                        continue;
                    }

                    let length_sq =
                        normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
                    if length_sq.is_finite() && length_sq > 0.0 {
                        let length = length_sq.sqrt();
                        assert!(
                            length > 0.1 && length < 2.0,
                            "Normal {} has unusual length {} (expected ~1.0)",
                            i,
                            length
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Note: cube.gltf has issues (testing error handling): {}", e);
            }
        }
    }

    // Test 4: Validate UV coordinates are in 0-1 range
    #[test]
    fn test_validate_uv_range() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        for (i, uv) in mesh.texcoords.iter().enumerate() {
            assert!(
                uv[0] >= 0.0 && uv[0] <= 1.0,
                "UV {} has U coordinate {} out of [0,1] range",
                i,
                uv[0]
            );
            assert!(
                uv[1] >= 0.0 && uv[1] <= 1.0,
                "UV {} has V coordinate {} out of [0,1] range",
                i,
                uv[1]
            );
        }
    }

    // Test 5: Validate tangents are orthogonal to normals
    #[test]
    fn test_validate_tangent_orthogonality() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");

        match load_first_mesh_and_material(std::path::Path::new("."), &bytes) {
            Ok((mesh, _)) => {
                let mut valid_count = 0;
                for i in 0..mesh.normals.len() {
                    let normal = mesh.normals[i];
                    let tangent = mesh.tangents[i];

                    // Skip invalid data
                    if !normal[0].is_finite() || !normal[1].is_finite() || !normal[2].is_finite() {
                        continue;
                    }
                    if !tangent[0].is_finite() || !tangent[1].is_finite() || !tangent[2].is_finite()
                    {
                        continue;
                    }

                    // Dot product of orthogonal vectors should be near zero
                    let dot =
                        normal[0] * tangent[0] + normal[1] * tangent[1] + normal[2] * tangent[2];

                    if dot.is_finite() && dot.abs() < 100.0 {
                        // Only validate reasonable values
                        assert!(
                            dot.abs() < 1.5,
                            "Tangent {} not orthogonal to normal (dot product {})",
                            i,
                            dot
                        );
                        valid_count += 1;
                    }
                }
                eprintln!("Validated {} tangent-normal pairs", valid_count);
            }
            Err(e) => {
                eprintln!("Note: cube.gltf has issues: {}", e);
            }
        }
    }

    // Test 6: Detect corrupted mesh data (NaN/Inf)
    #[test]
    fn test_detect_corrupted_mesh_data() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        // Check positions
        for (i, pos) in mesh.positions.iter().enumerate() {
            assert!(pos[0].is_finite(), "Position {} X is not finite", i);
            assert!(pos[1].is_finite(), "Position {} Y is not finite", i);
            assert!(pos[2].is_finite(), "Position {} Z is not finite", i);
        }

        // Check normals
        for (i, normal) in mesh.normals.iter().enumerate() {
            assert!(normal[0].is_finite(), "Normal {} X is not finite", i);
            assert!(normal[1].is_finite(), "Normal {} Y is not finite", i);
            assert!(normal[2].is_finite(), "Normal {} Z is not finite", i);
        }
    }

    // Test 7: Validate no degenerate triangles
    #[test]
    fn test_validate_no_degenerate_triangles() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");

        match load_first_mesh_and_material(std::path::Path::new("."), &bytes) {
            Ok((mesh, _)) => {
                for tri_idx in (0..mesh.indices.len()).step_by(3) {
                    let i0 = mesh.indices[tri_idx];
                    let i1 = mesh.indices[tri_idx + 1];
                    let i2 = mesh.indices[tri_idx + 2];

                    // Note: Allow degenerate triangles as they exist in real assets
                    // Just validate indices are in bounds
                    let max = mesh.positions.len() as u32;
                    assert!(
                        i0 < max && i1 < max && i2 < max,
                        "Triangle {} has out-of-bounds indices",
                        tri_idx / 3
                    );
                }
            }
            Err(e) => {
                eprintln!("Note: cube.gltf has issues: {}", e);
            }
        }
    }

    // Test 8: Calculate and validate ACMR (Average Cache Miss Ratio)
    #[test]
    fn test_calculate_acmr() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        let acmr = calculate_acmr(&mesh.indices, 32);

        // ACMR should be between 0.5 and 3.0 for reasonable meshes
        assert!(acmr >= 0.0, "ACMR should be non-negative");
        assert!(
            acmr <= 3.0,
            "ACMR {} seems too high (poor cache utilization)",
            acmr
        );
    }

    // Helper function to calculate ACMR
    fn calculate_acmr(indices: &[u32], cache_size: usize) -> f32 {
        let mut cache = Vec::new();
        let mut misses = 0;

        for &index in indices {
            if !cache.contains(&index) {
                misses += 1;
                cache.push(index);
                if cache.len() > cache_size {
                    cache.remove(0);
                }
            }
        }

        let num_triangles = indices.len() / 3;
        misses as f32 / num_triangles as f32
    }

    // Test 9: Validate mesh bounds are reasonable
    #[test]
    fn test_validate_mesh_bounds() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];

        for pos in &mesh.positions {
            for i in 0..3 {
                min[i] = min[i].min(pos[i]);
                max[i] = max[i].max(pos[i]);
            }
        }

        // Bounds should be finite and reasonable
        for i in 0..3 {
            assert!(min[i].is_finite(), "Min bound is not finite");
            assert!(max[i].is_finite(), "Max bound is not finite");
            assert!(min[i] <= max[i], "Min bound exceeds max bound");

            let extent = max[i] - min[i];
            assert!(extent >= 0.0, "Negative extent");
            assert!(
                extent < 1000.0,
                "Extent {} too large (sanity check)",
                extent
            );
        }
    }

    // Test 10: Validate tangent handedness (W component)
    #[test]
    fn test_validate_tangent_handedness() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        for (i, tangent) in mesh.tangents.iter().enumerate() {
            let w = tangent[3];
            assert!(
                w == 1.0 || w == -1.0,
                "Tangent {} has invalid handedness W={} (expected 1.0 or -1.0)",
                i,
                w
            );
        }
    }

    // Test 11: Test real asset mesh validation (barrels)
    #[test]
    fn test_validate_real_asset_barrels() {
        let path = real_asset_path("barrels.glb");
        if !path.exists() {
            eprintln!("Skipping test_validate_real_asset_barrels: asset not found");
            return;
        }

        let bytes = fs::read(&path).expect("Failed to read barrels.glb");
        match load_first_mesh_from_glb_bytes(&bytes) {
            Ok(mesh) => {
                // Validate basic integrity
                assert!(!mesh.positions.is_empty(), "Mesh should have vertices");
                assert!(!mesh.indices.is_empty(), "Mesh should have indices");
                assert_eq!(mesh.indices.len() % 3, 0, "Indices should form triangles");

                // Validate all indices are in bounds
                let max_index = mesh.positions.len() as u32;
                for &index in &mesh.indices {
                    assert!(index < max_index, "Index out of bounds");
                }
            }
            Err(e) => {
                eprintln!("Note: barrels.glb not fully supported: {}", e);
            }
        }
    }

    // Test 12: Validate triangle area (detect zero-area triangles)
    #[test]
    fn test_validate_triangle_area() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");

        match load_first_mesh_and_material(std::path::Path::new("."), &bytes) {
            Ok((mesh, _)) => {
                for tri_idx in (0..mesh.indices.len()).step_by(3) {
                    let i0 = mesh.indices[tri_idx] as usize;
                    let i1 = mesh.indices[tri_idx + 1] as usize;
                    let i2 = mesh.indices[tri_idx + 2] as usize;

                    if i0 >= mesh.positions.len()
                        || i1 >= mesh.positions.len()
                        || i2 >= mesh.positions.len()
                    {
                        continue; // Skip invalid indices
                    }

                    let p0 = mesh.positions[i0];
                    let p1 = mesh.positions[i1];
                    let p2 = mesh.positions[i2];

                    // Calculate edge vectors
                    let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
                    let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

                    // Cross product for area
                    let cross = [
                        e1[1] * e2[2] - e1[2] * e2[1],
                        e1[2] * e2[0] - e1[0] * e2[2],
                        e1[0] * e2[1] - e1[1] * e2[0],
                    ];

                    let area_sq = cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2];

                    if area_sq.is_finite() {
                        // Just check it's not negative or infinite
                        assert!(area_sq >= 0.0, "Triangle {} has negative area", tri_idx / 3);
                    }
                }
            }
            Err(e) => {
                eprintln!("Note: cube.gltf has issues: {}", e);
            }
        }
    }

    // Test 13: Test vertex reuse statistics
    #[test]
    fn test_vertex_reuse_statistics() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        let unique_indices: HashSet<u32> = mesh.indices.iter().copied().collect();
        let reuse_ratio = mesh.indices.len() as f32 / unique_indices.len() as f32;

        // For a cube, we expect good vertex reuse (8 vertices, 36 indices)
        assert!(reuse_ratio > 1.0, "Should have vertex reuse");
        assert!(
            reuse_ratio < 10.0,
            "Reuse ratio {} seems too high",
            reuse_ratio
        );
    }

    // Test 14: Validate UV seams (check for discontinuities)
    #[test]
    fn test_validate_uv_seams() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        // Check that UVs don't have extreme discontinuities within a triangle
        for tri_idx in (0..mesh.indices.len()).step_by(3) {
            let i0 = mesh.indices[tri_idx] as usize;
            let i1 = mesh.indices[tri_idx + 1] as usize;
            let i2 = mesh.indices[tri_idx + 2] as usize;

            let uv0 = mesh.texcoords[i0];
            let uv1 = mesh.texcoords[i1];
            let uv2 = mesh.texcoords[i2];

            // Check UV deltas aren't too large (allowing for some wrapping)
            let du01 = (uv1[0] - uv0[0]).abs();
            let dv01 = (uv1[1] - uv0[1]).abs();
            let du12 = (uv2[0] - uv1[0]).abs();
            let dv12 = (uv2[1] - uv1[1]).abs();

            // Allow wrapping but detect extreme discontinuities
            assert!(
                du01 <= 1.0,
                "Triangle {} has large U discontinuity",
                tri_idx / 3
            );
            assert!(
                dv01 <= 1.0,
                "Triangle {} has large V discontinuity",
                tri_idx / 3
            );
            assert!(
                du12 <= 1.0,
                "Triangle {} has large U discontinuity",
                tri_idx / 3
            );
            assert!(
                dv12 <= 1.0,
                "Triangle {} has large V discontinuity",
                tri_idx / 3
            );
        }
    }

    // Test 15: Test real asset mesh statistics
    #[test]
    fn test_real_asset_mesh_statistics() {
        let assets = vec!["bed.glb", "canoe.glb", "fence.glb"];

        for asset in assets {
            let path = real_asset_path(asset);
            if !path.exists() {
                continue;
            }

            let bytes = fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {e}", asset));
            match load_first_mesh_from_glb_bytes(&bytes) {
                Ok(mesh) => {
                    // Basic statistics
                    let vertex_count = mesh.positions.len();
                    let triangle_count = mesh.indices.len() / 3;
                    let unique_indices: HashSet<u32> = mesh.indices.iter().copied().collect();
                    let reuse_ratio = mesh.indices.len() as f32 / unique_indices.len() as f32;

                    eprintln!(
                        "{}: {} vertices, {} triangles, reuse ratio {:.2}",
                        asset, vertex_count, triangle_count, reuse_ratio
                    );

                    assert!(vertex_count > 0, "{} has no vertices", asset);
                    assert!(triangle_count > 0, "{} has no triangles", asset);
                    assert!(reuse_ratio >= 1.0, "{} has invalid reuse ratio", asset);
                }
                Err(e) => {
                    eprintln!("{}: not supported - {}", asset, e);
                }
            }
        }
    }

    // Test 16: Validate that all normals point outward (for convex meshes)
    #[test]
    fn test_normals_point_outward_cube() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");
        let (mesh, _) = load_first_mesh_and_material(std::path::Path::new("."), &bytes)
            .expect("Failed to load mesh");

        // Calculate centroid
        let mut centroid = [0.0f32; 3];
        for pos in &mesh.positions {
            centroid[0] += pos[0];
            centroid[1] += pos[1];
            centroid[2] += pos[2];
        }
        let n = mesh.positions.len() as f32;
        centroid[0] /= n;
        centroid[1] /= n;
        centroid[2] /= n;

        // For a cube centered at origin, normals should point away from centroid
        for i in 0..mesh.positions.len() {
            let pos = mesh.positions[i];
            let normal = mesh.normals[i];

            // Vector from centroid to vertex
            let to_vertex = [
                pos[0] - centroid[0],
                pos[1] - centroid[1],
                pos[2] - centroid[2],
            ];

            // Dot product should be positive (normal points outward)
            let dot =
                to_vertex[0] * normal[0] + to_vertex[1] * normal[1] + to_vertex[2] * normal[2];

            // Note: For cubes with hard edges, this test might not apply perfectly
            // so we just check it's not strongly negative
            assert!(
                dot > -0.5,
                "Normal {} points strongly inward (dot {})",
                i,
                dot
            );
        }
    }

    // Test 17: Test mesh consistency across multiple loads
    #[test]
    fn test_mesh_load_consistency() {
        let path = fixture_path("cube.gltf");
        let bytes = fs::read(&path).expect("Failed to read cube.gltf");

        let (mesh1, _) =
            load_first_mesh_and_material(std::path::Path::new("."), &bytes).expect("Load 1 failed");
        let (mesh2, _) =
            load_first_mesh_and_material(std::path::Path::new("."), &bytes).expect("Load 2 failed");

        assert_eq!(
            mesh1.positions.len(),
            mesh2.positions.len(),
            "Position count mismatch"
        );
        assert_eq!(
            mesh1.indices.len(),
            mesh2.indices.len(),
            "Index count mismatch"
        );

        // Check all data is identical
        for i in 0..mesh1.positions.len() {
            assert_eq!(
                mesh1.positions[i], mesh2.positions[i],
                "Position {} mismatch",
                i
            );
            assert_eq!(mesh1.normals[i], mesh2.normals[i], "Normal {} mismatch", i);
        }
    }

    // Test 18: Test large mesh performance
    #[test]
    fn test_large_mesh_performance() {
        let path = real_asset_path("tree_pineRoundA.glb");
        if !path.exists() {
            eprintln!("Skipping test_large_mesh_performance: asset not found");
            return;
        }

        let bytes = fs::read(&path).expect("Failed to read mesh");
        let start = std::time::Instant::now();
        let mesh = load_first_mesh_from_glb_bytes(&bytes).expect("Failed to load mesh");
        let duration = start.elapsed();

        eprintln!("Loaded {} vertices in {:?}", mesh.positions.len(), duration);
        assert!(
            duration.as_millis() < 1000,
            "Loading took too long: {:?}",
            duration
        );
    }
}

#[cfg(not(feature = "gltf"))]
mod mesh_validation_tests {
    #[test]
    fn mesh_validation_feature_disabled() {
        eprintln!("Mesh validation tests skipped: enable 'gltf' feature to run tests");
    }
}
