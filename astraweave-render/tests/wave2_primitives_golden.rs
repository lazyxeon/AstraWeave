#![allow(unused, clippy::all)]
//! Wave 2 – Golden-value tests for primitives.rs (123 mutants)
//!
//! Targets: cube(), plane(), sphere() — exact vertex positions, normals,
//!          UVs, index counts, and geometric invariants.
//!
//! Strategy: Pin EXACT float values at specific vertex indices so any
//! arithmetic mutation (e.g., + → −, * → /) is immediately caught.

use astraweave_render::primitives::{cube, plane, sphere};

// ============================================================================
// cube() — 24 vertices, 36 indices, 6 faces × 4 verts
// ============================================================================

#[test]
fn cube_vertex_count() {
    let (v, i) = cube();
    assert_eq!(v.len(), 24, "6 faces × 4 = 24 vertices");
    assert_eq!(i.len(), 36, "6 faces × 6 indices = 36");
}

#[test]
fn cube_indices_multiple_of_six() {
    let (_, i) = cube();
    assert_eq!(i.len() % 6, 0, "Each face = 6 indices (2 triangles)");
}

#[test]
fn cube_face0_plus_x_positions() {
    let (v, _) = cube();
    // Face 0 (+X): vertices 0..3
    assert_eq!(v[0].position, [1.0, -1.0, -1.0]);
    assert_eq!(v[1].position, [1.0, 1.0, -1.0]);
    assert_eq!(v[2].position, [1.0, 1.0, 1.0]);
    assert_eq!(v[3].position, [1.0, -1.0, 1.0]);
}

#[test]
fn cube_face0_plus_x_normals() {
    let (v, _) = cube();
    for i in 0..4 {
        assert_eq!(v[i].normal, [1.0, 0.0, 0.0], "Face +X normal at v[{}]", i);
    }
}

#[test]
fn cube_face1_minus_x_positions() {
    let (v, _) = cube();
    // Face 1 (-X): vertices 4..7
    assert_eq!(v[4].position, [-1.0, -1.0, 1.0]);
    assert_eq!(v[5].position, [-1.0, 1.0, 1.0]);
    assert_eq!(v[6].position, [-1.0, 1.0, -1.0]);
    assert_eq!(v[7].position, [-1.0, -1.0, -1.0]);
}

#[test]
fn cube_face1_minus_x_normals() {
    let (v, _) = cube();
    for i in 4..8 {
        assert_eq!(v[i].normal, [-1.0, 0.0, 0.0], "Face -X normal at v[{}]", i);
    }
}

#[test]
fn cube_face2_plus_y_normals() {
    let (v, _) = cube();
    for i in 8..12 {
        assert_eq!(v[i].normal, [0.0, 1.0, 0.0], "Face +Y normal at v[{}]", i);
    }
}

#[test]
fn cube_face3_minus_y_normals() {
    let (v, _) = cube();
    for i in 12..16 {
        assert_eq!(v[i].normal, [0.0, -1.0, 0.0], "Face -Y normal at v[{}]", i);
    }
}

#[test]
fn cube_face4_plus_z_normals() {
    let (v, _) = cube();
    for i in 16..20 {
        assert_eq!(v[i].normal, [0.0, 0.0, 1.0], "Face +Z normal at v[{}]", i);
    }
}

#[test]
fn cube_face5_minus_z_normals() {
    let (v, _) = cube();
    for i in 20..24 {
        assert_eq!(v[i].normal, [0.0, 0.0, -1.0], "Face -Z normal at v[{}]", i);
    }
}

#[test]
fn cube_first_face_indices_golden() {
    let (_, i) = cube();
    // Face 0 (+X): base=0, indices=[0,1,2, 0,2,3]
    assert_eq!(&i[0..6], &[0, 1, 2, 0, 2, 3]);
}

#[test]
fn cube_second_face_indices_golden() {
    let (_, i) = cube();
    // Face 1 (-X): base=4, indices=[4,5,6, 4,6,7]
    assert_eq!(&i[6..12], &[4, 5, 6, 4, 6, 7]);
}

#[test]
fn cube_uv_corners_per_face() {
    let (v, _) = cube();
    // Each face has 4 verts with UV corners: (0,0),(1,0),(1,1),(0,1)
    for face in 0..6 {
        let base = face * 4;
        assert_eq!(v[base + 0].uv, [0.0, 0.0], "face {} vert 0 uv", face);
        assert_eq!(v[base + 1].uv, [1.0, 0.0], "face {} vert 1 uv", face);
        assert_eq!(v[base + 2].uv, [1.0, 1.0], "face {} vert 2 uv", face);
        assert_eq!(v[base + 3].uv, [0.0, 1.0], "face {} vert 3 uv", face);
    }
}

#[test]
fn cube_all_tangents_are_positive_x() {
    let (v, _) = cube();
    for (idx, vert) in v.iter().enumerate() {
        assert_eq!(
            vert.tangent,
            [1.0, 0.0, 0.0, 1.0],
            "All cube tangents should be (+X, +1 handedness) at v[{}]",
            idx
        );
    }
}

#[test]
fn cube_indices_all_in_bounds() {
    let (v, i) = cube();
    for &idx in &i {
        assert!(
            (idx as usize) < v.len(),
            "Index {} out of bounds ({})",
            idx,
            v.len()
        );
    }
}

// ============================================================================
// plane() — 4 vertices, 6 indices, XZ at y=0
// ============================================================================

#[test]
fn plane_vertex_count() {
    let (v, i) = plane();
    assert_eq!(v.len(), 4);
    assert_eq!(i.len(), 6);
}

#[test]
fn plane_exact_positions() {
    let (v, _) = plane();
    assert_eq!(v[0].position, [-1.0, 0.0, -1.0]);
    assert_eq!(v[1].position, [1.0, 0.0, -1.0]);
    assert_eq!(v[2].position, [1.0, 0.0, 1.0]);
    assert_eq!(v[3].position, [-1.0, 0.0, 1.0]);
}

#[test]
fn plane_all_normals_plus_y() {
    let (v, _) = plane();
    for vert in &v {
        assert_eq!(vert.normal, [0.0, 1.0, 0.0]);
    }
}

#[test]
fn plane_tangents_plus_x() {
    let (v, _) = plane();
    for vert in &v {
        assert_eq!(vert.tangent, [1.0, 0.0, 0.0, 1.0]);
    }
}

#[test]
fn plane_exact_indices() {
    let (_, i) = plane();
    assert_eq!(&i[..], &[0, 1, 2, 0, 2, 3]);
}

#[test]
fn plane_uv_corners() {
    let (v, _) = plane();
    assert_eq!(v[0].uv, [0.0, 0.0]);
    assert_eq!(v[1].uv, [1.0, 0.0]);
    assert_eq!(v[2].uv, [1.0, 1.0]);
    assert_eq!(v[3].uv, [0.0, 1.0]);
}

#[test]
fn plane_all_y_zero() {
    let (v, _) = plane();
    for vert in &v {
        assert_eq!(vert.position[1], 0.0);
    }
}

// ============================================================================
// sphere() — parametric vertex/index count, radius, normals
// ============================================================================

#[test]
fn sphere_vertex_count_formula() {
    // (stacks+1) × (slices+1)
    let (v, _) = sphere(4, 6, 1.0);
    assert_eq!(v.len(), 5 * 7, "4+1=5 rows × 6+1=7 cols = 35");
}

#[test]
fn sphere_index_count_formula() {
    // stacks × slices × 6
    let (_, i) = sphere(4, 6, 1.0);
    assert_eq!(i.len(), 4 * 6 * 6, "4×6×6 = 144");
}

#[test]
fn sphere_minimum_clamps_to_three() {
    let (v1, _) = sphere(1, 1, 1.0);
    let (v2, _) = sphere(3, 3, 1.0);
    assert_eq!(v1.len(), v2.len(), "Below-min should clamp to 3");
    assert_eq!(v1.len(), 16, "4×4 = 16");
}

#[test]
fn sphere_north_pole_golden() {
    // stack=0: phi=0, sin_phi=0, cos_phi=1 → position=(0, r, 0), normal=(0,1,0)
    let r = 2.0;
    let (v, _) = sphere(4, 4, r);
    // First row (stack 0) has slices+1=5 vertices, all at north pole
    for j in 0..5 {
        let vert = &v[j];
        assert!((vert.position[0]).abs() < 1e-5, "North pole x at col {}", j);
        assert!(
            (vert.position[1] - r).abs() < 1e-5,
            "North pole y at col {}",
            j
        );
        assert!((vert.position[2]).abs() < 1e-5, "North pole z at col {}", j);
        assert!(
            (vert.normal[1] - 1.0).abs() < 1e-5,
            "North pole ny at col {}",
            j
        );
    }
}

#[test]
fn sphere_south_pole_golden() {
    let r = 2.0;
    let (v, _) = sphere(4, 4, r);
    let row = 5; // slices+1
                 // Last row (stack 4) at south pole: phi=PI, sin_phi≈0, cos_phi=-1
    let base = 4 * row;
    for j in 0..5 {
        let vert = &v[base + j];
        assert!((vert.position[0]).abs() < 1e-4, "South pole x at col {}", j);
        assert!(
            (vert.position[1] + r).abs() < 1e-4,
            "South pole y at col {}",
            j
        );
        assert!((vert.position[2]).abs() < 1e-4, "South pole z at col {}", j);
    }
}

#[test]
fn sphere_equator_radius() {
    let r = 3.0;
    let (v, _) = sphere(8, 8, r);
    // Equator is stack 4 (halfway): phi = PI/2, sin_phi=1, cos_phi=0
    let row = 9; // slices+1
    let base = 4 * row;
    for j in 0..=8 {
        let vert = &v[base + j];
        let dist =
            (vert.position[0].powi(2) + vert.position[1].powi(2) + vert.position[2].powi(2)).sqrt();
        assert!(
            (dist - r).abs() < 1e-4,
            "Equator vertex {} dist={}",
            j,
            dist
        );
        // y should be ~0 at equator
        assert!(
            vert.position[1].abs() < 1e-4,
            "Equator y should be ~0, got {}",
            vert.position[1]
        );
    }
}

#[test]
fn sphere_all_normals_unit_length() {
    let (v, _) = sphere(8, 8, 1.0);
    for (idx, vert) in v.iter().enumerate() {
        let len = (vert.normal[0].powi(2) + vert.normal[1].powi(2) + vert.normal[2].powi(2)).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "Normal at {} has length {}",
            idx,
            len
        );
    }
}

#[test]
fn sphere_all_vertices_at_radius() {
    let r = 5.0;
    let (v, _) = sphere(10, 10, r);
    for (idx, vert) in v.iter().enumerate() {
        let dist =
            (vert.position[0].powi(2) + vert.position[1].powi(2) + vert.position[2].powi(2)).sqrt();
        assert!(
            (dist - r).abs() < 0.01,
            "Vertex {} dist={}, expected {}",
            idx,
            dist,
            r
        );
    }
}

#[test]
fn sphere_normals_point_outward() {
    let (v, _) = sphere(8, 8, 1.0);
    for (idx, vert) in v.iter().enumerate() {
        let dot = vert.normal[0] * vert.position[0]
            + vert.normal[1] * vert.position[1]
            + vert.normal[2] * vert.position[2];
        assert!(
            dot > 0.99,
            "Normal at {} should point outward, dot={}",
            idx,
            dot
        );
    }
}

#[test]
fn sphere_uvs_in_range() {
    let (v, _) = sphere(8, 8, 1.0);
    for (idx, vert) in v.iter().enumerate() {
        assert!(
            vert.uv[0] >= 0.0 && vert.uv[0] <= 1.0,
            "UV.x at {} = {}",
            idx,
            vert.uv[0]
        );
        assert!(
            vert.uv[1] >= 0.0 && vert.uv[1] <= 1.0,
            "UV.y at {} = {}",
            idx,
            vert.uv[1]
        );
    }
}

#[test]
fn sphere_uv_v_decreases_with_stack() {
    let (v, _) = sphere(8, 8, 1.0);
    let row = 9; // slices+1
                 // UV.y (stored as 1-v) should decrease from top (stack 0) to bottom (stack 8)
                 // Actually uv = [u, 1.0 - v], so at stack 0 (v=0), uv.y = 1.0
                 // At stack 8 (v=1), uv.y = 0.0
    let top_uv_y = v[0].uv[1];
    let bottom_uv_y = v[8 * row].uv[1];
    assert!(
        (top_uv_y - 1.0).abs() < 1e-5,
        "Top UV.y should be 1.0, got {}",
        top_uv_y
    );
    assert!(
        (bottom_uv_y).abs() < 1e-5,
        "Bottom UV.y should be 0.0, got {}",
        bottom_uv_y
    );
}

#[test]
fn sphere_indices_all_in_bounds() {
    let (v, i) = sphere(8, 8, 1.0);
    for &idx in &i {
        assert!((idx as usize) < v.len(), "Index {} out of bounds", idx);
    }
}

#[test]
fn sphere_indices_form_triangles() {
    let (_, i) = sphere(8, 8, 1.0);
    assert_eq!(i.len() % 3, 0, "Indices must form complete triangles");
}

#[test]
fn sphere_tangent_handedness_positive() {
    let (v, _) = sphere(8, 8, 1.0);
    for (idx, vert) in v.iter().enumerate() {
        assert_eq!(
            vert.tangent[3], 1.0,
            "Tangent handedness at {} should be +1",
            idx
        );
    }
}

#[test]
fn sphere_first_quad_indices_golden() {
    // First quad: (stack=0, slice=0) → a=0, b=1, c=row, d=row+1 → [0,row,1, 1,row,row+1]
    let (_, i) = sphere(4, 4, 1.0);
    let row = 5u32; // slices+1
    assert_eq!(i[0], 0);
    assert_eq!(i[1], row);
    assert_eq!(i[2], 1);
    assert_eq!(i[3], 1);
    assert_eq!(i[4], row);
    assert_eq!(i[5], row + 1);
}

#[test]
fn sphere_equator_first_vertex_golden() {
    // At equator (stack=4 of 8): phi=PI/2, first slice (j=0): theta=0
    // nx=sin(PI/2)*cos(0)=1, ny=cos(PI/2)=0, nz=sin(PI/2)*sin(0)=0
    // px=r*1=r, py=r*0=0, pz=r*0=0
    let r = 1.0;
    let (v, _) = sphere(8, 8, r);
    let row = 9;
    let idx = 4 * row; // equator, first vertex
    assert!((v[idx].position[0] - r).abs() < 1e-4, "Equator v0 x");
    assert!((v[idx].position[1]).abs() < 1e-4, "Equator v0 y");
    assert!((v[idx].position[2]).abs() < 1e-4, "Equator v0 z");
    assert!((v[idx].normal[0] - 1.0).abs() < 1e-4, "Equator v0 nx");
}
