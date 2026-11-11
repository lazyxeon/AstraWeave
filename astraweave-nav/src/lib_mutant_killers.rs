//! Killers for the navmesh mutants missed on run 42753998469.

use super::{astar_tri, share_edge, Aabb, NavMesh, NavTri, Triangle};
use glam::Vec3;

fn tri(idx: usize, center: Vec3, neighbors: Vec<usize>) -> NavTri {
    NavTri {
        idx,
        verts: [center, center, center],
        normal: Vec3::Y,
        center,
        neighbors,
    }
}

fn flat(origin: Vec3) -> Triangle {
    Triangle::new(
        origin,
        origin + Vec3::new(0.0, 0.0, 1.0),
        origin + Vec3::new(1.0, 0.0, 0.0),
    )
}

#[test]
fn degenerate_triangles_are_the_ones_with_no_area() {
    let flat = flat(Vec3::ZERO);
    assert!(!flat.is_degenerate());
    assert!(Triangle::new(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO).is_degenerate());
    let baked = NavMesh::bake(
        &[Triangle::new(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO), flat],
        0.4,
        60.0,
    );
    assert_eq!(baked.triangle_count(), 1);
}

#[test]
fn share_edge_uses_squared_epsilon_and_a_height_difference() {
    let eps = 0.001;
    let max_step = 0.4;
    let separated = [
        NavTri::new(
            0,
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            Vec3::Y,
            Vec3::ZERO,
        ),
        NavTri::new(
            1,
            [
                Vec3::new(0.01, 0.0, 0.0),
                Vec3::new(1.01, 0.0, 0.0),
                Vec3::new(5.0, 0.0, 5.0),
            ],
            Vec3::Y,
            Vec3::ZERO,
        ),
    ];
    assert!(!share_edge(&separated[0], &separated[1], eps, max_step));

    let ledge = [
        NavTri::new(
            0,
            [
                Vec3::new(0.0, 0.3, 0.0),
                Vec3::new(1.0, 0.3, 0.0),
                Vec3::new(0.0, 5.0, 1.0),
            ],
            Vec3::Y,
            Vec3::ZERO,
        ),
        NavTri::new(
            1,
            [
                Vec3::new(0.0, -0.3, 0.0),
                Vec3::new(1.0, -0.3, 0.0),
                Vec3::new(5.0, 5.0, 5.0),
            ],
            Vec3::Y,
            Vec3::ZERO,
        ),
    ];
    assert!(!share_edge(&ledge[0], &ledge[1], eps, max_step));
    assert!(share_edge(
        &NavTri::new(
            0,
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            Vec3::Y,
            Vec3::ZERO,
        ),
        &NavTri::new(
            1,
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 1.0),
            ],
            Vec3::Y,
            Vec3::ZERO,
        ),
        eps,
        max_step,
    ));
}

#[test]
fn astar_prefers_the_shorter_center_path() {
    let tris = vec![
        tri(0, Vec3::new(0.0, 0.0, 0.0), vec![1, 2]),
        tri(1, Vec3::new(1.0, 0.0, 0.0), vec![0, 3]),
        tri(2, Vec3::new(0.0, 8.0, 0.0), vec![0, 3]),
        tri(3, Vec3::new(2.0, 0.0, 0.0), vec![1, 2]),
    ];
    assert_eq!(astar_tri(&tris, 0, 3), vec![0, 1, 3]);
}

#[test]
fn path_crosses_a_dirty_region_only_when_both_exist() {
    let mut mesh = NavMesh::bake(&[flat(Vec3::ZERO)], 0.4, 60.0);
    assert!(!mesh.path_crosses_dirty_region(&[Vec3::ZERO]));
    mesh.invalidate_region(Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::ONE));
    assert!(!mesh.path_crosses_dirty_region(&[]));
    assert!(mesh.path_crosses_dirty_region(&[Vec3::ZERO]));
    assert!(!mesh.path_crosses_dirty_region(&[Vec3::new(10.0, 0.0, 10.0)]));
}

#[test]
fn partial_rebake_relinks_only_shared_edges_up_to_capacity() {
    let mut tris = vec![Triangle::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 0.0),
    )];
    for k in 1..=9 {
        let z = k as f32 * 0.05;
        tris.push(Triangle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.5, 0.0, z),
            Vec3::new(1.0, 0.0, 0.0),
        ));
    }
    let mut nav = NavMesh::bake(&tris, 0.4, 80.0);
    assert_eq!(nav.triangle_count(), 10);
    for tri in &mut nav.tris {
        tri.neighbors = Vec::with_capacity(8);
    }
    nav.invalidate_region(Aabb::new(
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(2.0, 1.0, 2.0),
    ));
    let _ = nav.partial_rebake(&tris);
    assert_eq!(nav.tris[0].neighbors.len(), 8);
    assert!(!nav.tris[0].neighbors.contains(&0));
    assert!(nav.tris[0].neighbors.contains(&1));
}

#[test]
fn partial_rebake_drops_a_link_whose_vertices_moved() {
    let original = vec![
        Triangle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 0.0),
        ),
        Triangle::new(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(4.0, 0.0, 0.0),
        ),
    ];
    let mut nav = NavMesh::bake(&original, 0.4, 60.0);
    assert_eq!(nav.tris[0].neighbors, vec![1]);
    assert_eq!(nav.tris[1].neighbors, vec![0]);
    let moved = vec![
        Triangle::new(
            Vec3::new(10.0, 0.0, 10.0),
            Vec3::new(12.0, 0.0, 12.0),
            Vec3::new(12.0, 0.0, 10.0),
        ),
        original[1].clone(),
    ];
    nav.invalidate_region(Aabb::new(
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.5, 1.0, 2.5),
    ));
    let _affected = nav.partial_rebake(&moved);
    assert_eq!(nav.tris[0].verts[0], Vec3::new(10.0, 0.0, 10.0));
    assert!(nav.tris[1].neighbors.is_empty());
    assert!(!nav.tris[0].neighbors.contains(&1));
}

#[test]
fn partial_rebake_does_not_link_disjoint_triangles() {
    let tris = vec![flat(Vec3::ZERO), flat(Vec3::new(20.0, 0.0, 20.0))];
    let mut nav = NavMesh::bake(&tris, 0.4, 60.0);
    assert!(nav.tris[0].neighbors.is_empty());
    nav.invalidate_region(Aabb::new(
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(30.0, 1.0, 30.0),
    ));
    let _ = nav.partial_rebake(&tris);
    assert!(nav.tris[0].neighbors.is_empty());
    assert!(nav.tris[1].neighbors.is_empty());
}
