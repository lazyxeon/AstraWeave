//! Killers for the nav-tile mutants missed on run 42753998469.
//!
//! Each assertion fails for one operator change. Helpers that only exist
//! so a loop can terminate are not retested here.

use super::{
    adjacent_tile, build_slot, cos_deg, dist2_milli, hash_sources, manhattan, mix, normal_y,
    portal_id, slot_overlaps, tri_normal, CoverSite, NavChange, NavChangeKind, NavScratch,
    NavTiles, Portal, Slot, SourceTri, TileSideBuffer, TileTri, Triangle, Vec3, MAX_NODES,
    MAX_TRIS, TILE_SIZE_M,
};

fn up_tri(origin: Vec3) -> [Vec3; 3] {
    [
        origin,
        origin + Vec3::new(0.0, 0.0, 1.0),
        origin + Vec3::new(1.0, 0.0, 0.0),
    ]
}

fn slot_with(cx: i32, cz: i32) -> Slot {
    let mut slot = Slot::empty();
    slot.occupied = true;
    slot.cx = cx;
    slot.cz = cz;
    slot
}

#[test]
fn tri_normal_is_the_cross_product() {
    let n = tri_normal([
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(3.0, 4.0, 6.0),
        Vec3::new(8.0, 12.0, 14.0),
    ]);
    assert_eq!(n, Vec3::new(-16.0, 9.0, 1.0));
}

#[test]
fn normal_y_is_the_unit_up_component() {
    let n = Vec3::new(-16.0, 9.0, 1.0);
    assert_eq!(normal_y(n).to_bits(), 1_056_613_476);
    assert_eq!(normal_y(Vec3::splat(1.0e-8)), 0.0);
    assert_ne!(normal_y(n), 1.0);
}

#[test]
fn cos_deg_matches_the_degree_polynomial() {
    let v = cos_deg(20.0);
    assert_eq!(v.to_bits(), 1_064_341_427);
    assert_ne!(v, 0.0);
    assert_ne!(v, 1.0);
    assert_ne!(v, -1.0);
}

#[test]
fn slot_overlap_uses_tile_edges() {
    let inside = NavChange::new(1, 0, [32_000, 0, 0], [32_000, 0, 0], NavChangeKind::Brush);
    assert!(slot_overlaps(1, 0, &inside));
    let west = NavChange::new(1, 0, [31_999, 0, 0], [31_999, 0, 0], NavChangeKind::Brush);
    assert!(!slot_overlaps(1, 0, &west));
    let east = NavChange::new(1, 0, [70_000, 0, 0], [70_000, 0, 0], NavChangeKind::Brush);
    assert!(!slot_overlaps(1, 0, &east));
    let north = NavChange::new(1, 0, [32_000, 0, -1], [32_000, 0, -1], NavChangeKind::Brush);
    assert!(!slot_overlaps(1, 0, &north));
    let south = NavChange::new(
        1,
        0,
        [32_000, 0, 70_000],
        [32_000, 0, 70_000],
        NavChangeKind::Brush,
    );
    assert!(!slot_overlaps(1, 0, &south));
    assert!(!slot_overlaps(1, 2, &inside));
}

#[test]
fn adjacent_tiles_share_one_axis_step() {
    assert!(adjacent_tile(2, 0, 1, 0));
    assert!(!adjacent_tile(0, 0, 2, 0));
    assert!(!adjacent_tile(0, 0, 1, 1));
    assert!(!adjacent_tile(1, 1, 1, 1));
}

#[test]
fn manhattan_and_dist2_sum_each_axis() {
    let a = Vec3::new(0.0, 0.0, 0.0);
    let b = Vec3::new(3.0, 4.0, 5.0);
    assert_eq!(manhattan(a, b), 3_000 + 4_000 + 5_000);
    assert_eq!(
        dist2_milli(a, b),
        3_000 * 3_000 + 4_000 * 4_000 + 5_000 * 5_000
    );
    assert_ne!(manhattan(a, b), 0);
    assert_ne!(manhattan(a, b), 1);
}

#[test]
fn portal_id_and_mix_are_not_constants() {
    let id = portal_id(2, 3, 4, 5, 6, 7);
    assert_ne!(id, 0);
    assert_ne!(id, 1);
    let hashed = mix(1, 2);
    assert_ne!(hashed, 0);
    assert_ne!(hashed, 1);
    assert_eq!(mix(1, 2), mix(1, 2));
}

#[test]
fn build_slot_keeps_flat_triangles_and_caps_neighbors() {
    let mut slot = Slot::empty();
    let up = up_tri(Vec3::new(2.0, 4.0, 6.0));
    let down = [
        Vec3::new(2.0, 0.0, 6.0),
        Vec3::new(3.0, 0.0, 6.0),
        Vec3::new(2.0, 0.0, 7.0),
    ];
    slot.source_count = 2;
    slot.sources[0] = SourceTri { verts: up };
    slot.sources[1] = SourceTri { verts: down };
    let built = build_slot(&slot, 0.4, 1.0);
    assert_eq!(built.1, 1);
    assert_eq!(
        built.2[0].center,
        Vec3::new(2.0 + 1.0 / 3.0, 4.0, 6.0 + 1.0 / 3.0)
    );

    let mut fan = Slot::empty();
    fan.source_count = 10;
    for i in 0..10 {
        let z = 0.2 + i as f32;
        fan.sources[i as usize] = SourceTri {
            verts: [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.5, 0.0, z),
                Vec3::new(1.0, 0.0, 0.0),
            ],
        };
    }
    let fan_built = build_slot(&fan, 0.4, 0.5);
    assert_eq!(fan_built.1, 10);
    assert_eq!(fan_built.2[0].neighbor_count, 8);
    for n in 0..fan_built.2[0].neighbor_count {
        assert_ne!(fan_built.2[0].neighbors[n as usize], 0);
    }
}

#[test]
fn heap_orders_by_cost_then_lower_node() {
    let tiles = NavTiles::new(0.4, 45.0);
    let mut scratch = NavScratch::new();
    let items = [(5, 2u32), (3, 4), (3, 1), (7, 0), (3, 0), (1, 9)];
    for (cost, node) in items {
        assert!(tiles.heap_push(&mut scratch, cost, node));
    }
    let mut got = Vec::new();
    for _ in 0..items.len() {
        let before = scratch.heap_len;
        assert!(before > 0);
        got.push(tiles.heap_pop(&mut scratch));
        assert!(scratch.heap_len < before);
    }
    assert_eq!(got, vec![9, 0, 1, 4, 2, 0]);
    assert_eq!(scratch.heap_len, 0);
}

#[test]
fn dirty_slots_sort_by_coord_and_keep_equal_keys() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    tiles.slots[0] = slot_with(1, 0);
    tiles.slots[1] = slot_with(0, 5);
    tiles.slots[2] = slot_with(0, 1);
    tiles.slots[3] = slot_with(1, 0);
    tiles.dirty = [0, 1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    tiles.dirty_len = 4;
    tiles.sort_dirty();
    assert_eq!(&tiles.dirty[..4], &[2, 1, 0, 3]);

    tiles.dirty[0] = 0;
    tiles.dirty[1] = 3;
    tiles.dirty_len = 2;
    tiles.sort_dirty();
    assert_eq!(tiles.dirty[0], 0);
    assert_eq!(tiles.dirty[1], 3);
}

#[test]
fn pending_changes_sort_by_tick_then_seq() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    let late = NavChange::new(2, 0, [0; 3], [1; 3], NavChangeKind::Brush);
    let early = NavChange::new(1, 5, [0; 3], [1; 3], NavChangeKind::Detach);
    let high = NavChange::new(1, 9, [0; 3], [1; 3], NavChangeKind::Weave);
    let low = NavChange::new(1, 1, [7, 0, 0], [1; 3], NavChangeKind::Explosion);
    tiles.mark(late);
    tiles.mark(high);
    tiles.mark(early);
    tiles.mark(low);
    assert_eq!(tiles.pending_len, 4);
    assert_eq!(tiles.pending[0].seq, 1);
    assert_eq!(tiles.pending[0].min_milli[0], 7);
    assert_eq!(tiles.pending[1].seq, 5);
    assert_eq!(tiles.pending[2].seq, 9);
    assert_eq!(tiles.pending[3].tick, 2);

    let mut tied = NavTiles::new(0.4, 45.0);
    tied.mark(NavChange::new(
        3,
        4,
        [9, 0, 0],
        [1; 3],
        NavChangeKind::Brush,
    ));
    tied.mark(NavChange::new(
        3,
        4,
        [1, 0, 0],
        [1; 3],
        NavChangeKind::Brush,
    ));
    assert_eq!(tied.pending[0].min_milli[0], 9);
    assert_eq!(tied.pending[1].min_milli[0], 1);
}

#[test]
fn clear_dirty_slot_moves_the_tail_into_the_hole() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    tiles.dirty_len = 3;
    tiles.dirty[0] = 4;
    tiles.dirty[1] = 7;
    tiles.dirty[2] = 9;
    tiles.clear_dirty_slot(7);
    assert_eq!(tiles.dirty_len, 2);
    assert_eq!(tiles.dirty[0], 4);
    assert_eq!(tiles.dirty[1], 9);
    tiles.clear_dirty_slot(4);
    assert_eq!(tiles.dirty_len, 1);
    assert_eq!(tiles.dirty[0], 9);
}

#[test]
fn ingest_counts_placed_triangles_and_stops_at_the_slot_cap() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    let one = [Triangle::new(
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 2.0),
        Vec3::new(2.0, 0.0, 1.0),
    )];
    assert_eq!(tiles.ingest(&one), 1);
    assert_ne!(tiles.slot_hash(0, 0), Some(0));
    assert_ne!(tiles.slot_hash(0, 0), Some(1));
    assert!(tiles.slot_hash(0, 0).is_some());

    let mut many = NavTiles::new(0.4, 45.0);
    let mut tris = Vec::new();
    for cx in 0..17 {
        let x = cx as f32 * TILE_SIZE_M + 1.0;
        tris.push(Triangle::new(
            Vec3::new(x, 0.0, 1.0),
            Vec3::new(x, 0.0, 2.0),
            Vec3::new(x + 1.0, 0.0, 1.0),
        ));
    }
    assert_eq!(many.ingest(&tris), 16);
    assert_eq!(many.dirty_len(), 0);
}

#[test]
fn third_slot_path_uses_a_node_past_sixteen_plus_twenty_four() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    let mut tris = Vec::new();
    for cx in 0..3 {
        let x = cx as f32 * TILE_SIZE_M + 1.0;
        tris.push(Triangle::new(
            Vec3::new(x, 0.0, 1.0),
            Vec3::new(x, 0.0, 3.0),
            Vec3::new(x + 2.0, 0.0, 1.0),
        ));
    }
    assert_eq!(tiles.ingest(&tris), 3);
    let mut scratch = NavScratch::new();
    let point = Vec3::new(2.0 * TILE_SIZE_M + 1.5, 0.0, 1.4);
    assert_eq!(tiles.query_path(&mut scratch, point, point), 2);
    assert_eq!(scratch.waypoint_len, 2);
}

#[test]
fn two_triangle_path_appends_the_goal() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    let tris = [
        Triangle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 0.0),
        ),
        Triangle::new(
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 0.0),
        ),
    ];
    assert_eq!(tiles.ingest(&tris), 2);
    let mut scratch = NavScratch::new();
    let n = tiles.query_path(
        &mut scratch,
        Vec3::new(0.6, 0.0, 0.6),
        Vec3::new(1.4, 0.0, 1.4),
    );
    assert_eq!(n, 2);
    assert_eq!(scratch.waypoints[0], Vec3::new(0.6, 0.0, 0.6));
    assert_eq!(scratch.waypoints[1], Vec3::new(1.4, 0.0, 1.4));
}

struct StepCapReset;

impl Drop for StepCapReset {
    fn drop(&mut self) {
        super::set_search_step_cap(None);
    }
}

#[test]
fn path_search_gives_up_at_the_step_cap() {
    let _reset = StepCapReset;
    let mut tiles = NavTiles::new(0.4, 45.0);
    let tris = [
        Triangle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 0.0),
        ),
        Triangle::new(
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 0.0),
        ),
        Triangle::new(
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(4.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 0.0),
        ),
    ];
    assert_eq!(tiles.ingest(&tris), 3);
    let start = Vec3::new(0.4, 0.0, 0.4);
    let goal = Vec3::new(3.0, 0.0, 1.2);
    let mut scratch = NavScratch::new();
    assert!(tiles.query_path(&mut scratch, start, goal) > 0);
    super::set_search_step_cap(Some(2));
    assert_eq!(tiles.query_path(&mut scratch, start, goal), 0);
}

#[test]
fn heap_push_gives_up_on_the_385th_entry() {
    let tiles = NavTiles::new(0.4, 45.0);
    let mut scratch = NavScratch::new();
    for i in 0..MAX_NODES {
        assert!(tiles.heap_push(&mut scratch, i as u32, i as u32));
    }
    assert!(!tiles.heap_push(&mut scratch, 0, 0));
    assert_eq!(scratch.heap_len as usize, MAX_NODES);
}

#[test]
fn commits_count_accepted_buffers() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    let _ = tiles.ingest(&[Triangle::new(
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 2.0),
        Vec3::new(2.0, 0.0, 1.0),
    )]);
    assert_eq!(tiles.commits(), 0);
    tiles.mark(NavChange::new(
        1,
        0,
        [0, -1, 0],
        [3_000, 1_000, 3_000],
        NavChangeKind::Brush,
    ));
    assert!(tiles.commit_tick() >= 1);
    tiles.mark(NavChange::new(
        2,
        0,
        [0, -1, 0],
        [3_000, 1_000, 3_000],
        NavChangeKind::Detach,
    ));
    assert!(tiles.commit_tick() >= 1);
    assert_eq!(tiles.commits(), 2);
}

#[test]
fn dirty_coord_rejects_the_length_itself() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    tiles.slots[0] = slot_with(3, 4);
    tiles.dirty[0] = 0;
    tiles.dirty_len = 1;
    assert_eq!(tiles.dirty_coord(0), Some((3, 4)));
    assert_eq!(tiles.dirty_coord(1), None);
}

#[test]
fn empty_side_buffer_is_not_ready_and_is_rejected() {
    let tiles = NavTiles::new(0.4, 45.0);
    let buf = tiles.fill_side_buffer(0);
    assert!(!buf.ready);
    let mut tiles = tiles;
    assert!(!tiles.accept_side_buffer(&buf));
    let occupied = {
        let _ = tiles.ingest(&[Triangle::new(
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 1.0),
        )]);
        let mut not_ready = tiles.fill_side_buffer(0);
        not_ready.ready = false;
        not_ready
    };
    assert!(!tiles.accept_side_buffer(&occupied));
    let expected = hash_sources(&tiles.slots[3].sources, 0, tiles.max_step, tiles.min_up_dot);
    let foreign = TileSideBuffer {
        slot: 3,
        input_hash: expected,
        ready: true,
        tri_count: 0,
        tris: [TileTri::empty(); MAX_TRIS],
        generation: 1,
    };
    assert!(!tiles.accept_side_buffer(&foreign));
}

#[test]
fn closest_node_keeps_the_earlier_equal_distance() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    tiles.slot_count = 1;
    tiles.slots[0].occupied = true;
    tiles.slots[0].tri_count = 2;
    tiles.slots[0].tris[0] = TileTri::empty();
    tiles.slots[0].tris[0].live = true;
    tiles.slots[0].tris[0].center = Vec3::new(0.0, 0.0, 0.0);
    tiles.slots[0].tris[1] = TileTri::empty();
    tiles.slots[0].tris[1].live = true;
    tiles.slots[0].tris[1].center = Vec3::new(2.0, 0.0, 0.0);
    assert_eq!(tiles.closest_node(Vec3::new(1.0, 0.0, 0.0)), Some(0));
}

#[test]
fn relax_skips_a_dead_triangle_and_a_partial_portal() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    tiles.slot_count = 2;
    tiles.slots[0].occupied = true;
    tiles.slots[0].tri_count = 1;
    tiles.slots[0].tris[0] = TileTri::empty();
    tiles.slots[0].tris[0].live = false;
    tiles.slots[0].tris[0].center = Vec3::ZERO;
    let mut scratch = NavScratch::new();
    scratch.parent[0] = 7;
    tiles.relax(&mut scratch, 0, 0, 0, 0, 1);
    assert_eq!(scratch.parent[0], 7);

    tiles.slots[0].tris[0].live = true;
    tiles.slots[1].occupied = true;
    tiles.slots[1].tri_count = 1;
    tiles.slots[1].tris[0] = TileTri::empty();
    tiles.slots[1].tris[0].live = true;
    tiles.slots[1].tris[0].center = Vec3::new(1.0, 0.0, 0.0);
    tiles.portal_count = 1;
    tiles.portals[0] = Portal {
        a_slot: 0,
        a_tri: 4,
        b_slot: 1,
        b_tri: 0,
        id: 1,
        x_milli: 0,
        z_milli: 0,
        live: true,
    };
    scratch.parent[MAX_TRIS as u32 as usize] = 9;
    tiles.relax_neighbors(&mut scratch, 0, 0, MAX_TRIS as u32, 1);
    assert_eq!(scratch.parent[MAX_TRIS], 9);

    tiles.portals[0].a_tri = 0;
    tiles.relax_neighbors(&mut scratch, 0, 0, MAX_TRIS as u32, 1);
    assert_eq!(scratch.parent[MAX_TRIS], 0);
    assert!(scratch.g[MAX_TRIS] > 0);
}

#[test]
fn record_portal_requires_the_full_endpoint_pair() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    tiles.portal_count = 1;
    tiles.portals[0] = Portal {
        a_slot: 0,
        a_tri: 1,
        b_slot: 1,
        b_tri: 2,
        id: 99,
        x_milli: 0,
        z_milli: 0,
        live: true,
    };
    let left = 1u32;
    let right = MAX_TRIS as u32 + 2;
    let mut scratch = NavScratch::new();
    tiles.record_portal(&mut scratch, left, right);
    assert_eq!(scratch.portal_len, 1);
    assert_eq!(scratch.portal_ids[0], 99);
    let mut reverse = NavScratch::new();
    tiles.record_portal(&mut reverse, right, left);
    assert_eq!(reverse.portal_len, 1);
    assert_eq!(reverse.portal_ids[0], 99);

    let mut same = NavScratch::new();
    tiles.portals[0].b_slot = 0;
    tiles.portals[0].b_tri = 2;
    tiles.record_portal(&mut same, left, 2);
    assert_eq!(same.portal_len, 0);

    tiles.portals[0] = Portal {
        a_slot: 0,
        a_tri: 1,
        b_slot: 1,
        b_tri: 2,
        id: 99,
        x_milli: 0,
        z_milli: 0,
        live: true,
    };
    for (a_tri, b_slot, b_tri) in [(3u16, 1u16, 2u16), (1, 4, 2), (1, 1, 5)] {
        tiles.portals[0].a_tri = a_tri;
        tiles.portals[0].b_slot = b_slot;
        tiles.portals[0].b_tri = b_tri;
        let mut miss = NavScratch::new();
        tiles.record_portal(&mut miss, left, right);
        assert_eq!(miss.portal_len, 0);
    }
}

#[test]
fn collect_sites_tracks_new_breaches_cover_and_losses() {
    let mut tiles = NavTiles::new(0.4, 45.0);
    tiles.breach_count = 7;
    tiles.portal_count = 1;
    tiles.portals[0] = Portal {
        a_slot: 0,
        a_tri: 0,
        b_slot: 1,
        b_tri: 0,
        id: 42,
        x_milli: 5,
        z_milli: 6,
        live: true,
    };
    tiles.slot_count = 2;
    tiles.slots[1].occupied = true;
    tiles.slots[1].tri_count = 2;
    tiles.slots[1].tris[0] = TileTri::empty();
    tiles.slots[1].tris[0].live = true;
    tiles.slots[1].tris[0].neighbor_count = 2;
    tiles.slots[1].tris[0].center = Vec3::new(1.25, 0.0, 2.5);
    tiles.slots[1].tris[1] = TileTri::empty();
    tiles.slots[1].tris[1].live = false;
    tiles.slots[1].tris[1].neighbor_count = 0;
    tiles.slots[1].tris[1].center = Vec3::new(9.0, 0.0, 9.0);
    tiles.prev_cover_len = 1;
    tiles.prev_cover[0] = CoverSite {
        id: 999,
        x_milli: 1,
        z_milli: 2,
    };
    tiles.collect_sites();
    assert_eq!(tiles.breach_count, 1);
    assert_eq!(tiles.breaches[0].portal_id, 42);
    assert_eq!(tiles.breaches[0].x_milli, 5);
    assert_eq!(tiles.cover_count, 1);
    assert_eq!(tiles.cover[0].id, 1u32 << 8);
    assert_eq!(tiles.lost_cover_count, 1);
    assert_eq!(tiles.lost_cover[0].id, 999);

    tiles.prev_portal_len = 1;
    tiles.prev_portal_ids[0] = 42;
    tiles.prev_cover[0] = CoverSite {
        id: 1u32 << 8,
        x_milli: 1,
        z_milli: 2,
    };
    tiles.collect_sites();
    assert_eq!(tiles.breach_count, 0);
    assert_eq!(tiles.lost_cover_count, 0);
}
