//! Mutation-resistant comprehensive tests for astraweave-pcg.

#![allow(clippy::manual_range_contains)]

use astraweave_pcg::*;
use glam::IVec2;

// ═══════════════════════════════════════════════════════════════════════════
// SeedRng tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn seed_rng_deterministic_same_seed() {
    let mut r1 = SeedRng::new(42, "test");
    let mut r2 = SeedRng::new(42, "test");
    for _ in 0..100 {
        assert_eq!(r1.gen_range(0..1000i32), r2.gen_range(0..1000i32));
    }
}

#[test]
fn seed_rng_different_seed_different_output() {
    let mut r1 = SeedRng::new(1, "test");
    let mut r2 = SeedRng::new(2, "test");
    let v1: Vec<i32> = (0..20).map(|_| r1.gen_range(0..1000)).collect();
    let v2: Vec<i32> = (0..20).map(|_| r2.gen_range(0..1000)).collect();
    assert_ne!(v1, v2);
}

#[test]
fn seed_rng_layer_tracking() {
    let rng = SeedRng::new(0, "main");
    assert_eq!(rng.layer(), "main");
}

#[test]
fn seed_rng_fork_deterministic() {
    let mut r1 = SeedRng::new(42, "parent");
    let mut r2 = SeedRng::new(42, "parent");
    let mut f1 = r1.fork("child");
    let mut f2 = r2.fork("child");
    for _ in 0..50 {
        assert_eq!(f1.gen_range(0..1000i32), f2.gen_range(0..1000i32));
    }
}

#[test]
fn seed_rng_fork_independent() {
    let mut rng = SeedRng::new(99, "parent");
    let mut f1 = rng.fork("a");
    let mut f2 = rng.fork("b");
    let v1: Vec<i32> = (0..10).map(|_| f1.gen_range(0..1000)).collect();
    let v2: Vec<i32> = (0..10).map(|_| f2.gen_range(0..1000)).collect();
    assert_ne!(
        v1, v2,
        "different sublayers should produce different sequences"
    );
}

#[test]
fn seed_rng_fork_layer_name() {
    let mut rng = SeedRng::new(0, "root");
    let forked = rng.fork("sub");
    assert_eq!(forked.layer(), "root::sub");
}

#[test]
fn seed_rng_gen_bool() {
    let mut rng = SeedRng::new(42, "test");
    let mut trues = 0;
    let mut falses = 0;
    for _ in 0..1000 {
        if rng.gen_bool() {
            trues += 1;
        } else {
            falses += 1;
        }
    }
    assert!(trues > 0, "should produce some trues");
    assert!(falses > 0, "should produce some falses");
}

#[test]
fn seed_rng_gen_bool_with_prob_zero() {
    let mut rng = SeedRng::new(0, "test");
    for _ in 0..100 {
        assert!(
            !rng.gen_bool_with_prob(0.0),
            "prob 0.0 should always be false"
        );
    }
}

#[test]
fn seed_rng_gen_bool_with_prob_one() {
    let mut rng = SeedRng::new(0, "test");
    for _ in 0..100 {
        assert!(
            rng.gen_bool_with_prob(1.0),
            "prob 1.0 should always be true"
        );
    }
}

#[test]
fn seed_rng_choose_nonempty() {
    let mut rng = SeedRng::new(0, "test");
    let items = [10, 20, 30, 40, 50];
    let chosen = rng.choose(&items);
    assert!(chosen.is_some());
    assert!(items.contains(chosen.unwrap()));
}

#[test]
fn seed_rng_choose_empty() {
    let mut rng = SeedRng::new(0, "test");
    let empty: Vec<i32> = vec![];
    assert!(rng.choose(&empty).is_none());
}

#[test]
fn seed_rng_choose_mut_nonempty() {
    let mut rng = SeedRng::new(0, "test");
    let mut items = [1, 2, 3];
    let chosen = rng.choose_mut(&mut items);
    assert!(chosen.is_some());
}

#[test]
fn seed_rng_shuffle_deterministic() {
    let mut r1 = SeedRng::new(42, "test");
    let mut r2 = SeedRng::new(42, "test");
    let mut a1 = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut a2 = a1.clone();
    r1.shuffle(&mut a1);
    r2.shuffle(&mut a2);
    assert_eq!(a1, a2);
}

#[test]
fn seed_rng_gen_f32_in_range() {
    let mut rng = SeedRng::new(0, "test");
    for _ in 0..100 {
        let v = rng.gen_f32();
        assert!(v >= 0.0 && v < 1.0, "gen_f32 should be in [0,1): got {v}");
    }
}

#[test]
fn seed_rng_gen_f64_in_range() {
    let mut rng = SeedRng::new(0, "test");
    for _ in 0..100 {
        let v = rng.gen_f64();
        assert!(v >= 0.0 && v < 1.0, "gen_f64 should be in [0,1): got {v}");
    }
}

#[test]
fn seed_rng_gen_range_bounds() {
    let mut rng = SeedRng::new(42, "test");
    for _ in 0..1000 {
        let v: i32 = rng.gen_range(5..10);
        assert!(v >= 5 && v < 10, "gen_range(5..10) produced {v}");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Room tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn room_center_calculation() {
    let room = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(10, 10)),
        connections: vec![],
    };
    let center = room.center();
    assert_eq!(center.x, 5);
    assert_eq!(center.y, 5);
}

#[test]
fn room_center_non_origin() {
    let room = Room {
        bounds: (IVec2::new(4, 6), IVec2::new(8, 12)),
        connections: vec![],
    };
    let center = room.center();
    assert_eq!(center.x, 6);
    assert_eq!(center.y, 9);
}

#[test]
fn room_size() {
    let room = Room {
        bounds: (IVec2::new(2, 3), IVec2::new(12, 8)),
        connections: vec![],
    };
    let size = room.size();
    assert_eq!(size.x, 10);
    assert_eq!(size.y, 5);
}

#[test]
fn room_contains_inside() {
    let room = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(10, 10)),
        connections: vec![],
    };
    assert!(room.contains(IVec2::new(5, 5)));
    assert!(room.contains(IVec2::new(0, 0)));
}

#[test]
fn room_contains_outside() {
    let room = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(10, 10)),
        connections: vec![],
    };
    assert!(!room.contains(IVec2::new(-1, 5)));
    assert!(!room.contains(IVec2::new(11, 5)));
    assert!(!room.contains(IVec2::new(5, -1)));
    assert!(!room.contains(IVec2::new(5, 11)));
}

#[test]
fn room_overlaps_true() {
    let r1 = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(10, 10)),
        connections: vec![],
    };
    let r2 = Room {
        bounds: (IVec2::new(5, 5), IVec2::new(15, 15)),
        connections: vec![],
    };
    assert!(r1.overlaps(&r2));
    assert!(r2.overlaps(&r1));
}

#[test]
fn room_overlaps_false() {
    let r1 = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(5, 5)),
        connections: vec![],
    };
    let r2 = Room {
        bounds: (IVec2::new(10, 10), IVec2::new(15, 15)),
        connections: vec![],
    };
    assert!(!r1.overlaps(&r2));
    assert!(!r2.overlaps(&r1));
}

#[test]
fn room_connections() {
    let room = Room {
        bounds: (IVec2::ZERO, IVec2::new(5, 5)),
        connections: vec![1, 3, 7],
    };
    assert_eq!(room.connections.len(), 3);
    assert_eq!(room.connections[0], 1);
    assert_eq!(room.connections[1], 3);
    assert_eq!(room.connections[2], 7);
}

#[test]
fn room_clone() {
    let room = Room {
        bounds: (IVec2::new(1, 2), IVec2::new(3, 4)),
        connections: vec![0],
    };
    let r2 = room.clone();
    assert_eq!(r2.bounds, room.bounds);
    assert_eq!(r2.connections, room.connections);
}

#[test]
fn room_json_roundtrip() {
    let room = Room {
        bounds: (IVec2::new(5, 5), IVec2::new(15, 15)),
        connections: vec![2, 4],
    };
    let json = serde_json::to_string(&room).unwrap();
    let back: Room = serde_json::from_str(&json).unwrap();
    assert_eq!(back.bounds.0.x, 5);
    assert_eq!(back.bounds.1.x, 15);
    assert_eq!(back.connections.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// LayoutGenerator tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn layout_generator_defaults() {
    let lg = LayoutGenerator::new(IVec2::new(100, 100));
    assert_eq!(lg.grid_size.x, 100);
    assert_eq!(lg.grid_size.y, 100);
    assert_eq!(lg.room_min_size.x, 5);
    assert_eq!(lg.room_min_size.y, 5);
    assert_eq!(lg.room_max_size.x, 15);
    assert_eq!(lg.room_max_size.y, 15);
    assert_eq!(lg.max_placement_attempts, 100);
}

#[test]
fn layout_generator_deterministic() {
    let lg = LayoutGenerator::new(IVec2::new(100, 100));
    let mut r1 = SeedRng::new(42, "layout");
    let mut r2 = SeedRng::new(42, "layout");
    let rooms1 = lg.generate_rooms(&mut r1, 5);
    let rooms2 = lg.generate_rooms(&mut r2, 5);
    assert_eq!(rooms1.len(), rooms2.len());
    for (a, b) in rooms1.iter().zip(rooms2.iter()) {
        assert_eq!(a.bounds, b.bounds);
    }
}

#[test]
fn layout_generator_rooms_in_bounds() {
    let lg = LayoutGenerator::new(IVec2::new(100, 100));
    let mut rng = SeedRng::new(99, "layout");
    let rooms = lg.generate_rooms(&mut rng, 10);
    for room in &rooms {
        assert!(room.bounds.0.x >= 0);
        assert!(room.bounds.0.y >= 0);
        assert!(room.bounds.1.x <= 100);
        assert!(room.bounds.1.y <= 100);
    }
}

#[test]
fn layout_generator_no_overlaps() {
    let lg = LayoutGenerator::new(IVec2::new(200, 200));
    let mut rng = SeedRng::new(7, "layout");
    let rooms = lg.generate_rooms(&mut rng, 10);
    for i in 0..rooms.len() {
        for j in (i + 1)..rooms.len() {
            assert!(!rooms[i].overlaps(&rooms[j]), "rooms {i} and {j} overlap");
        }
    }
}

#[test]
fn layout_generator_rooms_connected() {
    let lg = LayoutGenerator::new(IVec2::new(200, 200));
    let mut rng = SeedRng::new(123, "layout");
    let rooms = lg.generate_rooms(&mut rng, 5);
    if rooms.len() > 1 {
        // At least some rooms should have connections
        let total_connections: usize = rooms.iter().map(|r| r.connections.len()).sum();
        assert!(total_connections > 0, "rooms should be connected");
    }
}

#[test]
fn layout_generator_zero_count() {
    let lg = LayoutGenerator::new(IVec2::new(100, 100));
    let mut rng = SeedRng::new(0, "layout");
    let rooms = lg.generate_rooms(&mut rng, 0);
    assert!(rooms.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// EncounterConstraints default values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn encounter_constraints_default_bounds() {
    let ec = EncounterConstraints::default();
    assert_eq!(ec.bounds.0, IVec2::ZERO);
    assert_eq!(ec.bounds.1.x, 100);
    assert_eq!(ec.bounds.1.y, 100);
}

#[test]
fn encounter_constraints_default_min_spacing() {
    let ec = EncounterConstraints::default();
    assert!((ec.min_spacing - 10.0).abs() < f32::EPSILON);
}

#[test]
fn encounter_constraints_default_difficulty_range() {
    let ec = EncounterConstraints::default();
    assert!((ec.difficulty_range.0 - 1.0).abs() < f32::EPSILON);
    assert!((ec.difficulty_range.1 - 5.0).abs() < f32::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════════════
// EncounterKind variants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn encounter_kind_combat() {
    let kind = EncounterKind::Combat {
        enemy_types: vec!["goblin".into(), "orc".into()],
        count: 5,
    };
    if let EncounterKind::Combat { enemy_types, count } = kind {
        assert_eq!(enemy_types.len(), 2);
        assert_eq!(enemy_types[0], "goblin");
        assert_eq!(count, 5);
    } else {
        panic!("expected Combat");
    }
}

#[test]
fn encounter_kind_loot() {
    let kind = EncounterKind::Loot {
        items: vec!["gold".into()],
    };
    if let EncounterKind::Loot { items } = kind {
        assert_eq!(items[0], "gold");
    } else {
        panic!("expected Loot");
    }
}

#[test]
fn encounter_kind_ambient() {
    let kind = EncounterKind::Ambient {
        event_id: "rain".into(),
    };
    if let EncounterKind::Ambient { event_id } = kind {
        assert_eq!(event_id, "rain");
    } else {
        panic!("expected Ambient");
    }
}

#[test]
fn encounter_kind_clone() {
    let kind = EncounterKind::Combat {
        enemy_types: vec!["wolf".into()],
        count: 3,
    };
    let kind2 = kind.clone();
    if let EncounterKind::Combat { count, .. } = kind2 {
        assert_eq!(count, 3);
    }
}

#[test]
fn encounter_kind_json_roundtrip() {
    let kind = EncounterKind::Loot {
        items: vec!["gem".into(), "coin".into()],
    };
    let json = serde_json::to_string(&kind).unwrap();
    let back: EncounterKind = serde_json::from_str(&json).unwrap();
    if let EncounterKind::Loot { items } = back {
        assert_eq!(items.len(), 2);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Encounter struct tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn encounter_fields() {
    let enc = Encounter {
        kind: EncounterKind::Combat {
            enemy_types: vec!["skeleton".into()],
            count: 2,
        },
        position: IVec2::new(50, 50),
        difficulty: 3.5,
        metadata: std::collections::BTreeMap::new(),
    };
    assert_eq!(enc.position.x, 50);
    assert_eq!(enc.position.y, 50);
    assert!((enc.difficulty - 3.5).abs() < f32::EPSILON);
    assert!(enc.metadata.is_empty());
}

#[test]
fn encounter_with_metadata() {
    let mut meta = std::collections::BTreeMap::new();
    meta.insert("zone".into(), "forest".into());
    let enc = Encounter {
        kind: EncounterKind::Ambient {
            event_id: "wind".into(),
        },
        position: IVec2::new(10, 20),
        difficulty: 1.0,
        metadata: meta,
    };
    assert_eq!(enc.metadata["zone"], "forest");
}

#[test]
fn encounter_clone() {
    let enc = Encounter {
        kind: EncounterKind::Loot {
            items: vec!["key".into()],
        },
        position: IVec2::new(5, 5),
        difficulty: 2.0,
        metadata: std::collections::BTreeMap::new(),
    };
    let enc2 = enc.clone();
    assert_eq!(enc2.position, enc.position);
    assert!((enc2.difficulty - 2.0).abs() < f32::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════════════
// EncounterGenerator tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn encounter_generator_deterministic() {
    let constraints = EncounterConstraints::default();
    let gen = EncounterGenerator::new(constraints.clone());
    let mut r1 = SeedRng::new(42, "enc");
    let mut r2 = SeedRng::new(42, "enc");
    let e1 = gen.generate(&mut r1, 5);
    let gen2 = EncounterGenerator::new(constraints);
    let e2 = gen2.generate(&mut r2, 5);
    assert_eq!(e1.len(), e2.len());
    for (a, b) in e1.iter().zip(e2.iter()) {
        assert_eq!(a.position, b.position);
        assert!((a.difficulty - b.difficulty).abs() < f32::EPSILON);
    }
}

#[test]
fn encounter_generator_bounds_respected() {
    let constraints = EncounterConstraints {
        bounds: (IVec2::new(10, 10), IVec2::new(50, 50)),
        min_spacing: 5.0,
        difficulty_range: (1.0, 3.0),
    };
    let gen = EncounterGenerator::new(constraints);
    let mut rng = SeedRng::new(0, "enc");
    let encounters = gen.generate(&mut rng, 10);
    for enc in &encounters {
        assert!(enc.position.x >= 10, "x out of bounds: {}", enc.position.x);
        assert!(enc.position.x <= 50, "x out of bounds: {}", enc.position.x);
        assert!(enc.position.y >= 10, "y out of bounds: {}", enc.position.y);
        assert!(enc.position.y <= 50, "y out of bounds: {}", enc.position.y);
    }
}

#[test]
fn encounter_generator_difficulty_in_range() {
    let constraints = EncounterConstraints {
        difficulty_range: (2.0, 4.0),
        ..EncounterConstraints::default()
    };
    let gen = EncounterGenerator::new(constraints);
    let mut rng = SeedRng::new(123, "enc");
    let encounters = gen.generate(&mut rng, 20);
    for enc in &encounters {
        assert!(
            enc.difficulty >= 2.0,
            "difficulty too low: {}",
            enc.difficulty
        );
        assert!(
            enc.difficulty <= 4.0,
            "difficulty too high: {}",
            enc.difficulty
        );
    }
}

#[test]
fn encounter_generator_zero_count() {
    let gen = EncounterGenerator::new(EncounterConstraints::default());
    let mut rng = SeedRng::new(0, "enc");
    let encounters = gen.generate(&mut rng, 0);
    assert!(encounters.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// Mutation kill tests
// ═══════════════════════════════════════════════════════════════════════════

/// Kill: SeedRng::shuffle → ()
#[test]
fn shuffle_actually_reorders() {
    let mut rng = SeedRng::new(42, "test");
    let original: Vec<i32> = (1..=20).collect();
    let mut shuffled = original.clone();
    rng.shuffle(&mut shuffled);
    assert_ne!(
        shuffled, original,
        "shuffle must reorder a 20-element array"
    );
}

/// Kill: SeedRng::gen_f32 → 0.0
#[test]
fn gen_f32_produces_nonzero() {
    let mut rng = SeedRng::new(42, "test");
    let sum: f32 = (0..100).map(|_| rng.gen_f32()).sum();
    assert!(
        sum > 1.0,
        "100 gen_f32 calls should produce sum > 1.0, got {sum}"
    );
}

/// Kill: SeedRng::gen_f64 → 0.0
#[test]
fn gen_f64_produces_nonzero() {
    let mut rng = SeedRng::new(42, "test");
    let sum: f64 = (0..100).map(|_| rng.gen_f64()).sum();
    assert!(
        sum > 1.0,
        "100 gen_f64 calls should produce sum > 1.0, got {sum}"
    );
}

/// Kill: Room::overlaps || → && mutations — rooms separated in X only
#[test]
fn overlaps_false_when_separated_in_x_only() {
    let r1 = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(5, 10)),
        connections: vec![],
    };
    let r2 = Room {
        bounds: (IVec2::new(20, 0), IVec2::new(25, 10)),
        connections: vec![],
    };
    // Same Y range, different X — should NOT overlap
    assert!(!r1.overlaps(&r2));
}

/// Kill: Room::overlaps || → && mutations — rooms separated in Y only
#[test]
fn overlaps_false_when_separated_in_y_only() {
    let r1 = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(10, 5)),
        connections: vec![],
    };
    let r2 = Room {
        bounds: (IVec2::new(0, 20), IVec2::new(10, 25)),
        connections: vec![],
    };
    // Same X range, different Y — should NOT overlap
    assert!(!r1.overlaps(&r2));
}

/// Kill: Room::overlaps boundary mutations (< → <=, > → >=, == etc)
#[test]
fn overlaps_true_when_edge_touching() {
    // Rooms share an edge at x=5
    let r1 = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(5, 10)),
        connections: vec![],
    };
    let r2 = Room {
        bounds: (IVec2::new(5, 0), IVec2::new(10, 10)),
        connections: vec![],
    };
    // With `<`, 5 < 5 is false, so !(false||...) depends on other conditions
    // These rooms share the x=5 line, so they DO overlap (touching counts as overlap)
    assert!(r1.overlaps(&r2), "edge-touching rooms should overlap");
}

/// Kill: Room::overlaps boundary mutations — Y edge touching
#[test]
fn overlaps_true_when_y_edge_touching() {
    let r1 = Room {
        bounds: (IVec2::new(0, 0), IVec2::new(10, 5)),
        connections: vec![],
    };
    let r2 = Room {
        bounds: (IVec2::new(0, 5), IVec2::new(10, 10)),
        connections: vec![],
    };
    assert!(r1.overlaps(&r2), "Y-edge-touching rooms should overlap");
}

/// Kill: LayoutGenerator::generate_rooms → vec![]
#[test]
fn generate_rooms_produces_rooms() {
    let lg = LayoutGenerator::new(IVec2::new(200, 200));
    let mut rng = SeedRng::new(42, "layout");
    let rooms = lg.generate_rooms(&mut rng, 5);
    assert!(
        !rooms.is_empty(),
        "generate_rooms must produce at least one room"
    );
    assert!(
        rooms.len() >= 3,
        "with ample space, should place at least 3 of 5 rooms"
    );
}

/// Kill: LayoutGenerator::try_place_room → None (via generate_rooms returning few/no rooms)
/// Kill: try_place_room delete ! (would generate overlapping rooms)
/// Kill: try_place_room + → - (negative room dimensions)
#[test]
fn generated_rooms_have_positive_dimensions() {
    let lg = LayoutGenerator::new(IVec2::new(200, 200));
    let mut rng = SeedRng::new(99, "layout");
    let rooms = lg.generate_rooms(&mut rng, 8);
    assert!(rooms.len() >= 2, "should generate rooms in ample space");
    for (i, room) in rooms.iter().enumerate() {
        let size = room.size();
        assert!(
            size.x > 0,
            "room {i} width must be positive, got {}",
            size.x
        );
        assert!(
            size.y > 0,
            "room {i} height must be positive, got {}",
            size.y
        );
    }
}

/// Kill: connect_rooms + → * on line 115 (connects to self instead of i+1)
#[test]
fn connected_rooms_dont_connect_to_self() {
    let lg = LayoutGenerator::new(IVec2::new(200, 200));
    let mut rng = SeedRng::new(7, "layout");
    let rooms = lg.generate_rooms(&mut rng, 5);
    assert!(rooms.len() >= 2);
    // Chain connections should go to i+1, not i itself
    for (i, room) in rooms.iter().enumerate() {
        for &conn in &room.connections {
            assert_ne!(conn, i, "room {i} should not connect to itself");
        }
    }
}

/// Kill: connect_rooms != → == and delete ! on line 124
/// Kill: connect_rooms && → || on line 124
#[test]
fn chain_connections_are_sequential() {
    let lg = LayoutGenerator::new(IVec2::new(300, 300));
    let mut rng = SeedRng::new(42, "layout");
    let rooms = lg.generate_rooms(&mut rng, 4);
    assert!(rooms.len() >= 3, "need at least 3 rooms for chain test");
    // First room should connect to room 1 (chain)
    assert!(
        rooms[0].connections.contains(&1),
        "room 0 should have chain connection to room 1"
    );
    // Second room should connect to both room 0 and room 2
    assert!(
        rooms[1].connections.contains(&0),
        "room 1 should connect back to room 0"
    );
    assert!(
        rooms[1].connections.contains(&2),
        "room 1 should connect forward to room 2"
    );
}

/// Kill: EncounterGenerator::generate * → + on line 68 (max_attempts calc)
/// Kill: EncounterGenerator::generate < → <= on loop conditions
#[test]
fn encounter_generator_produces_requested_count() {
    let constraints = EncounterConstraints {
        bounds: (IVec2::new(0, 0), IVec2::new(1000, 1000)),
        min_spacing: 1.0,
        difficulty_range: (1.0, 3.0),
    };
    let gen = EncounterGenerator::new(constraints);
    let mut rng = SeedRng::new(42, "enc");
    let encounters = gen.generate(&mut rng, 5);
    assert_eq!(
        encounters.len(),
        5,
        "with ample space, should produce exactly 5 encounters"
    );
}
