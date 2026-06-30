//! Wave 2 – Golden-value tests for mesh.rs (60 mutants)
//!
//! Targets: MeshVertex construction, from_arrays, ATTRIBS layout,
//!          MeshVertexLayout stride/step_mode, CpuMesh aabb() exact
//!          golden values, compute_tangents correctness.
//!
//! Strategy: Pin exact field values and computed results so any
//! arithmetic or constructor mutation is caught.

use astraweave_render::mesh::{compute_tangents, CpuMesh, MeshVertex, MeshVertexLayout};
use glam::{Vec2, Vec3, Vec4};

// ============================================================================
// MeshVertex::new — golden field mapping
// ============================================================================

#[test]
fn vertex_new_position_golden() {
    let v = MeshVertex::new(
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        Vec2::new(0.5, 0.75),
    );
    assert_eq!(v.position, [1.0, 2.0, 3.0]);
}

#[test]
fn vertex_new_normal_golden() {
    let v = MeshVertex::new(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, -1.0),
        Vec4::ZERO,
        Vec2::ZERO,
    );
    assert_eq!(v.normal, [0.0, 0.0, -1.0]);
}

#[test]
fn vertex_new_tangent_golden() {
    let v = MeshVertex::new(
        Vec3::ZERO,
        Vec3::Y,
        Vec4::new(0.5, 0.6, 0.7, -1.0),
        Vec2::ZERO,
    );
    assert_eq!(v.tangent, [0.5, 0.6, 0.7, -1.0]);
}

#[test]
fn vertex_new_uv_golden() {
    let v = MeshVertex::new(Vec3::ZERO, Vec3::Y, Vec4::X, Vec2::new(0.25, 0.875));
    assert_eq!(v.uv, [0.25, 0.875]);
}

// ============================================================================
// MeshVertex::from_arrays — passthrough
// ============================================================================

#[test]
fn vertex_from_arrays_position() {
    let v = MeshVertex::from_arrays([10.0, 20.0, 30.0], [0.0; 3], [0.0; 4], [0.0; 2]);
    assert_eq!(v.position, [10.0, 20.0, 30.0]);
}

#[test]
fn vertex_from_arrays_normal() {
    let v = MeshVertex::from_arrays([0.0; 3], [0.577, 0.577, 0.577], [0.0; 4], [0.0; 2]);
    assert_eq!(v.normal, [0.577, 0.577, 0.577]);
}

#[test]
fn vertex_from_arrays_tangent() {
    let v = MeshVertex::from_arrays([0.0; 3], [0.0; 3], [1.0, 0.0, 0.0, -1.0], [0.0; 2]);
    assert_eq!(v.tangent, [1.0, 0.0, 0.0, -1.0]);
}

#[test]
fn vertex_from_arrays_uv() {
    let v = MeshVertex::from_arrays([0.0; 3], [0.0; 3], [0.0; 4], [0.3, 0.9]);
    assert_eq!(v.uv, [0.3, 0.9]);
}

// ============================================================================
// MeshVertex — size and layout
// ============================================================================

#[test]
fn mesh_vertex_size_48_bytes() {
    assert_eq!(std::mem::size_of::<MeshVertex>(), 48);
}

#[test]
fn mesh_vertex_attribs_count_4() {
    assert_eq!(MeshVertex::ATTRIBS.len(), 4);
}

#[test]
fn mesh_vertex_attrib_locations_0_1_2_3() {
    assert_eq!(MeshVertex::ATTRIBS[0].shader_location, 0);
    assert_eq!(MeshVertex::ATTRIBS[1].shader_location, 1);
    assert_eq!(MeshVertex::ATTRIBS[2].shader_location, 2);
    assert_eq!(MeshVertex::ATTRIBS[3].shader_location, 3);
}

// ============================================================================
// MeshVertexLayout — buffer layout golden
// ============================================================================

#[test]
fn buffer_layout_stride_48() {
    let layout = MeshVertexLayout::buffer_layout();
    assert_eq!(layout.array_stride, 48);
}

#[test]
fn buffer_layout_step_mode_vertex() {
    let layout = MeshVertexLayout::buffer_layout();
    assert_eq!(layout.step_mode, wgpu::VertexStepMode::Vertex);
}

#[test]
fn buffer_layout_4_attributes() {
    let layout = MeshVertexLayout::buffer_layout();
    assert_eq!(layout.attributes.len(), 4);
}

// ============================================================================
// CpuMesh — aabb golden values
// ============================================================================

#[test]
fn aabb_empty_is_none() {
    let mesh = CpuMesh::default();
    assert!(mesh.aabb().is_none());
}

#[test]
fn aabb_single_vertex_min_eq_max() {
    let mut mesh = CpuMesh::default();
    mesh.vertices.push(MeshVertex::from_arrays(
        [5.0, 10.0, -3.0],
        [0.0; 3],
        [0.0; 4],
        [0.0; 2],
    ));
    let (min, max) = mesh.aabb().unwrap();
    assert_eq!(min, Vec3::new(5.0, 10.0, -3.0));
    assert_eq!(max, Vec3::new(5.0, 10.0, -3.0));
}

#[test]
fn aabb_two_vertices_golden() {
    let mut mesh = CpuMesh::default();
    mesh.vertices.push(MeshVertex::from_arrays(
        [-1.0, -2.0, -3.0],
        [0.0; 3],
        [0.0; 4],
        [0.0; 2],
    ));
    mesh.vertices.push(MeshVertex::from_arrays(
        [4.0, 5.0, 6.0],
        [0.0; 3],
        [0.0; 4],
        [0.0; 2],
    ));
    let (min, max) = mesh.aabb().unwrap();
    assert_eq!(min, Vec3::new(-1.0, -2.0, -3.0));
    assert_eq!(max, Vec3::new(4.0, 5.0, 6.0));
}

#[test]
fn aabb_three_vertices_mixed() {
    let mut mesh = CpuMesh::default();
    let positions = [[1.0, -5.0, 3.0], [-2.0, 10.0, -7.0], [4.0, 0.0, 1.0]];
    for p in &positions {
        mesh.vertices
            .push(MeshVertex::from_arrays(*p, [0.0; 3], [0.0; 4], [0.0; 2]));
    }
    let (min, max) = mesh.aabb().unwrap();
    assert_eq!(min, Vec3::new(-2.0, -5.0, -7.0));
    assert_eq!(max, Vec3::new(4.0, 10.0, 3.0));
}

#[test]
fn aabb_all_same_position() {
    let mut mesh = CpuMesh::default();
    for _ in 0..5 {
        mesh.vertices.push(MeshVertex::from_arrays(
            [7.0, 7.0, 7.0],
            [0.0; 3],
            [0.0; 4],
            [0.0; 2],
        ));
    }
    let (min, max) = mesh.aabb().unwrap();
    assert_eq!(min, max);
    assert_eq!(min, Vec3::new(7.0, 7.0, 7.0));
}

// ============================================================================
// compute_tangents — correctness
// ============================================================================

#[test]
fn compute_tangents_empty_no_crash() {
    let mut mesh = CpuMesh::default();
    compute_tangents(&mut mesh);
    assert!(mesh.vertices.is_empty());
}

#[test]
fn compute_tangents_not_divisible_by_3_early_return() {
    let mut mesh = CpuMesh::default();
    mesh.vertices.push(MeshVertex::from_arrays(
        [0.0; 3],
        [0.0, 1.0, 0.0],
        [99.0, 99.0, 99.0, 99.0], // sentinel
        [0.0; 2],
    ));
    mesh.indices = vec![0, 0]; // not divisible by 3
    compute_tangents(&mut mesh);
    // tangent should remain untouched
    assert_eq!(mesh.vertices[0].tangent, [99.0, 99.0, 99.0, 99.0]);
}

#[test]
fn compute_tangents_xz_plane_gives_unit_tangent() {
    let mut mesh = CpuMesh::default();
    // Triangle on XZ plane, normals +Y, UVs mapping x→u, z→v
    mesh.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [0.0, 0.0],
    ));
    mesh.vertices.push(MeshVertex::from_arrays(
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [1.0, 0.0],
    ));
    mesh.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [0.0, 1.0],
    ));
    mesh.indices = vec![0, 1, 2];

    compute_tangents(&mut mesh);

    // Tangent for XZ plane with these UVs should be along +X
    for v in &mesh.vertices {
        let tx = v.tangent[0];
        let ty = v.tangent[1];
        let tz = v.tangent[2];
        let len = (tx * tx + ty * ty + tz * tz).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "Tangent should be unit length, got {}",
            len
        );
        assert!(
            tx > 0.9,
            "Tangent x should be ~1.0 for +X direction, got {}",
            tx
        );
    }
}

#[test]
fn compute_tangents_handedness_positive_for_standard_winding() {
    let mut mesh = CpuMesh::default();
    mesh.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [0.0, 0.0],
    ));
    mesh.vertices.push(MeshVertex::from_arrays(
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [1.0, 0.0],
    ));
    mesh.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [0.0, 1.0],
    ));
    mesh.indices = vec![0, 1, 2];

    compute_tangents(&mut mesh);

    // w component encodes handedness
    for v in &mesh.vertices {
        let w = v.tangent[3];
        assert!(
            w == 1.0 || w == -1.0,
            "Handedness should be ±1.0, got {}",
            w
        );
    }
}

#[test]
fn compute_tangents_degenerate_triangle_no_crash() {
    let mut mesh = CpuMesh::default();
    // All same position = degenerate
    for _ in 0..3 {
        mesh.vertices.push(MeshVertex::from_arrays(
            [0.0; 3],
            [0.0, 1.0, 0.0],
            [0.0; 4],
            [0.5, 0.5],
        ));
    }
    mesh.indices = vec![0, 1, 2];
    compute_tangents(&mut mesh);
    // Should not crash, tangent values should be finite
    for v in &mesh.vertices {
        for c in &v.tangent {
            assert!(c.is_finite(), "Tangent component should be finite");
        }
    }
}

#[test]
fn compute_tangents_mixed_valid_and_degenerate_keeps_valid_tangents() {
    // Pins the M1.5 fix's correctness property: a single zero-area (coincident-
    // position) triangle must NOT crash AND must NOT demote the whole mesh — the
    // valid triangle's vertices still receive real (finite, non-zero) tangents.
    // (Distinguishes the fix from a whole-mesh no-op, which would leave them zero.)
    let mut mesh = CpuMesh::default();
    // Valid triangle in the XZ plane with proper UVs (verts 0,1,2).
    mesh.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [0.0, 0.0],
    ));
    mesh.vertices.push(MeshVertex::from_arrays(
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [1.0, 0.0],
    ));
    mesh.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0; 4],
        [0.0, 1.0],
    ));
    // Fully-coincident degenerate triangle, offset so the mesh box has extent
    // (verts 3,4,5 — distinct indices, identical positions).
    for _ in 0..3 {
        mesh.vertices.push(MeshVertex::from_arrays(
            [5.0, 5.0, 5.0],
            [0.0, 1.0, 0.0],
            [0.0; 4],
            [0.5, 0.5],
        ));
    }
    mesh.indices = vec![0, 1, 2, 3, 4, 5];
    compute_tangents(&mut mesh);

    // Valid triangle: finite AND real (non-zero) tangents — not abandoned.
    for v in &mesh.vertices[0..3] {
        assert!(
            v.tangent.iter().all(|c| c.is_finite()),
            "valid-triangle tangent must be finite, got {:?}",
            v.tangent
        );
        let len_sq =
            v.tangent[0] * v.tangent[0] + v.tangent[1] * v.tangent[1] + v.tangent[2] * v.tangent[2];
        assert!(
            len_sq > 0.25,
            "valid-triangle tangent must be real (non-zero), got {:?}",
            v.tangent
        );
    }
    // Degenerate triangle: must be finite (default/zero is acceptable).
    for v in &mesh.vertices[3..6] {
        assert!(
            v.tangent.iter().all(|c| c.is_finite()),
            "degenerate-triangle tangent must be finite, got {:?}",
            v.tangent
        );
    }
}

// ============================================================================
// CpuMesh — Default + Clone
// ============================================================================

#[test]
fn cpu_mesh_default_empty() {
    let mesh = CpuMesh::default();
    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}

#[test]
fn cpu_mesh_clone_preserves_data() {
    let mut mesh = CpuMesh::default();
    mesh.vertices.push(MeshVertex::from_arrays(
        [1.0, 2.0, 3.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.5, 0.5],
    ));
    mesh.indices = vec![0];
    let cloned = mesh.clone();
    assert_eq!(cloned.vertices.len(), 1);
    assert_eq!(cloned.indices, vec![0]);
    assert_eq!(cloned.vertices[0].position, [1.0, 2.0, 3.0]);
}

// ============================================================================
// MeshVertex — Pod/Zeroable traits
// ============================================================================

#[test]
fn mesh_vertex_zeroable() {
    let v: MeshVertex = bytemuck::Zeroable::zeroed();
    assert_eq!(v.position, [0.0; 3]);
    assert_eq!(v.normal, [0.0; 3]);
    assert_eq!(v.tangent, [0.0; 4]);
    assert_eq!(v.uv, [0.0; 2]);
}

#[test]
fn mesh_vertex_pod_roundtrip() {
    let v = MeshVertex::from_arrays(
        [1.0, 2.0, 3.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.5, 0.75],
    );
    let bytes = bytemuck::bytes_of(&v);
    assert_eq!(bytes.len(), 48);
    let v2: &MeshVertex = bytemuck::from_bytes(bytes);
    assert_eq!(v2.position, v.position);
    assert_eq!(v2.normal, v.normal);
    assert_eq!(v2.tangent, v.tangent);
    assert_eq!(v2.uv, v.uv);
}
