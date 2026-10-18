//! Mutation-resistant comprehensive tests for astraweave-persistence-ecs.
//!
//! These tests target exact return values, boundary conditions, operator swaps,
//! negation bugs, and off-by-one errors to achieve 90%+ mutation kill rate.

#![allow(clippy::unnecessary_to_owned)]

use astraweave_core::ecs_components::*;
use astraweave_core::IVec2;
use astraweave_ecs::{App, Plugin, Schedule, World};
use astraweave_persistence_ecs::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════
// SerializedEntity field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn serialized_entity_all_none() {
    let se = SerializedEntity {
        entity_raw: 0,
        pos: None,
        health: None,
        team: None,
        ammo: None,
        cooldowns: None,
        desired_pos: None,
        ai_agent: None,
        legacy_id: None,
        persona: None,
        memory: None,
    };
    assert_eq!(se.entity_raw, 0);
    assert!(se.pos.is_none());
    assert!(se.health.is_none());
    assert!(se.team.is_none());
    assert!(se.ammo.is_none());
    assert!(se.cooldowns.is_none());
    assert!(se.desired_pos.is_none());
    assert!(se.ai_agent.is_none());
    assert!(se.legacy_id.is_none());
    assert!(se.persona.is_none());
    assert!(se.memory.is_none());
}

#[test]
fn serialized_entity_with_pos() {
    let se = SerializedEntity {
        entity_raw: 42,
        pos: Some(CPos {
            pos: IVec2 { x: 5, y: 10 },
        }),
        health: None,
        team: None,
        ammo: None,
        cooldowns: None,
        desired_pos: None,
        ai_agent: None,
        legacy_id: None,
        persona: None,
        memory: None,
    };
    assert_eq!(se.entity_raw, 42);
    let p = se.pos.unwrap();
    assert_eq!(p.pos.x, 5);
    assert_eq!(p.pos.y, 10);
}

#[test]
fn serialized_entity_with_health() {
    let se = SerializedEntity {
        entity_raw: 1,
        pos: None,
        health: Some(CHealth { hp: 75 }),
        team: None,
        ammo: None,
        cooldowns: None,
        desired_pos: None,
        ai_agent: None,
        legacy_id: None,
        persona: None,
        memory: None,
    };
    assert_eq!(se.health.unwrap().hp, 75);
}

#[test]
fn serialized_entity_with_team() {
    let se = SerializedEntity {
        entity_raw: 1,
        pos: None,
        health: None,
        team: Some(CTeam { id: 3 }),
        ammo: None,
        cooldowns: None,
        desired_pos: None,
        ai_agent: None,
        legacy_id: None,
        persona: None,
        memory: None,
    };
    assert_eq!(se.team.unwrap().id, 3);
}

#[test]
fn serialized_entity_with_ammo() {
    let se = SerializedEntity {
        entity_raw: 1,
        pos: None,
        health: None,
        team: None,
        ammo: Some(CAmmo { rounds: 30 }),
        cooldowns: None,
        desired_pos: None,
        ai_agent: None,
        legacy_id: None,
        persona: None,
        memory: None,
    };
    assert_eq!(se.ammo.unwrap().rounds, 30);
}

#[test]
fn serialized_entity_with_desired_pos() {
    let se = SerializedEntity {
        entity_raw: 1,
        pos: None,
        health: None,
        team: None,
        ammo: None,
        cooldowns: None,
        desired_pos: Some(CDesiredPos {
            pos: IVec2 { x: -3, y: 7 },
        }),
        ai_agent: None,
        legacy_id: None,
        persona: None,
        memory: None,
    };
    let dp = se.desired_pos.unwrap();
    assert_eq!(dp.pos.x, -3);
    assert_eq!(dp.pos.y, 7);
}

#[test]
fn serialized_entity_clone_preserves_all_fields() {
    let se = SerializedEntity {
        entity_raw: 99,
        pos: Some(CPos {
            pos: IVec2 { x: 1, y: 2 },
        }),
        health: Some(CHealth { hp: 100 }),
        team: Some(CTeam { id: 5 }),
        ammo: Some(CAmmo { rounds: 50 }),
        cooldowns: Some(CCooldowns {
            map: BTreeMap::new(),
        }),
        desired_pos: Some(CDesiredPos {
            pos: IVec2 { x: 3, y: 4 },
        }),
        ai_agent: Some(CAiAgent),
        legacy_id: None,
        persona: None,
        memory: None,
    };
    let se2 = se.clone();
    assert_eq!(se2.entity_raw, 99);
    assert_eq!(se2.pos.unwrap().pos.x, 1);
    assert_eq!(se2.pos.unwrap().pos.y, 2);
    assert_eq!(se2.health.unwrap().hp, 100);
    assert_eq!(se2.team.unwrap().id, 5);
    assert_eq!(se2.ammo.unwrap().rounds, 50);
    assert!(se2.cooldowns.is_some());
    assert_eq!(se2.desired_pos.unwrap().pos.x, 3);
    assert!(se2.ai_agent.is_some());
}

// ═══════════════════════════════════════════════════════════════════════════
// SerializedWorld field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn serialized_world_empty() {
    let sw = SerializedWorld {
        entities: vec![],
        world_tick: 0,
    };
    assert!(sw.entities.is_empty());
    assert_eq!(sw.world_tick, 0);
}

#[test]
fn serialized_world_with_entities() {
    let sw = SerializedWorld {
        entities: vec![
            SerializedEntity {
                entity_raw: 1,
                pos: Some(CPos {
                    pos: IVec2 { x: 0, y: 0 },
                }),
                health: Some(CHealth { hp: 100 }),
                team: None,
                ammo: None,
                cooldowns: None,
                desired_pos: None,
                ai_agent: None,
                legacy_id: None,
                persona: None,
                memory: None,
            },
            SerializedEntity {
                entity_raw: 2,
                pos: Some(CPos {
                    pos: IVec2 { x: 5, y: 5 },
                }),
                health: Some(CHealth { hp: 50 }),
                team: None,
                ammo: None,
                cooldowns: None,
                desired_pos: None,
                ai_agent: None,
                legacy_id: None,
                persona: None,
                memory: None,
            },
        ],
        world_tick: 42,
    };
    assert_eq!(sw.entities.len(), 2);
    assert_eq!(sw.world_tick, 42);
    assert_eq!(sw.entities[0].entity_raw, 1);
    assert_eq!(sw.entities[1].entity_raw, 2);
}

#[test]
fn serialized_world_clone() {
    let sw = SerializedWorld {
        entities: vec![SerializedEntity {
            entity_raw: 7,
            pos: None,
            health: None,
            team: None,
            ammo: None,
            cooldowns: None,
            desired_pos: None,
            ai_agent: None,
            legacy_id: None,
            persona: None,
            memory: None,
        }],
        world_tick: 10,
    };
    let sw2 = sw.clone();
    assert_eq!(sw2.world_tick, 10);
    assert_eq!(sw2.entities.len(), 1);
    assert_eq!(sw2.entities[0].entity_raw, 7);
}

// ═══════════════════════════════════════════════════════════════════════════
// CReplayState field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn replay_state_initial() {
    let rs = CReplayState {
        is_replaying: true,
        current_tick: 0,
        total_ticks: 100,
        events: vec![],
    };
    assert!(rs.is_replaying);
    assert_eq!(rs.current_tick, 0);
    assert_eq!(rs.total_ticks, 100);
    assert!(rs.events.is_empty());
}

#[test]
fn replay_state_not_replaying() {
    let rs = CReplayState {
        is_replaying: false,
        current_tick: 50,
        total_ticks: 50,
        events: vec![],
    };
    assert!(!rs.is_replaying);
    assert_eq!(rs.current_tick, 50);
    assert_eq!(rs.total_ticks, 50);
}

#[test]
fn replay_state_with_events() {
    let rs = CReplayState {
        is_replaying: true,
        current_tick: 5,
        total_ticks: 100,
        events: vec![
            ReplayEvent {
                tick: 1,
                event_type: "spawn".into(),
                data: vec![1, 2, 3],
            },
            ReplayEvent {
                tick: 5,
                event_type: "move".into(),
                data: vec![4, 5],
            },
        ],
    };
    assert_eq!(rs.events.len(), 2);
    assert_eq!(rs.events[0].tick, 1);
    assert_eq!(rs.events[0].event_type, "spawn");
    assert_eq!(rs.events[0].data, vec![1, 2, 3]);
    assert_eq!(rs.events[1].tick, 5);
    assert_eq!(rs.events[1].event_type, "move");
}

#[test]
fn replay_state_clone() {
    let rs = CReplayState {
        is_replaying: true,
        current_tick: 10,
        total_ticks: 200,
        events: vec![ReplayEvent {
            tick: 3,
            event_type: "exec".into(),
            data: vec![99],
        }],
    };
    let rs2 = rs.clone();
    assert!(rs2.is_replaying);
    assert_eq!(rs2.current_tick, 10);
    assert_eq!(rs2.total_ticks, 200);
    assert_eq!(rs2.events.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// ReplayEvent field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn replay_event_fields() {
    let re = ReplayEvent {
        tick: 42,
        event_type: "damage".into(),
        data: vec![10, 20, 30],
    };
    assert_eq!(re.tick, 42);
    assert_eq!(re.event_type, "damage");
    assert_eq!(re.data.len(), 3);
    assert_eq!(re.data[0], 10);
    assert_eq!(re.data[1], 20);
    assert_eq!(re.data[2], 30);
}

#[test]
fn replay_event_empty_data() {
    let re = ReplayEvent {
        tick: 0,
        event_type: "init".into(),
        data: vec![],
    };
    assert_eq!(re.tick, 0);
    assert_eq!(re.event_type, "init");
    assert!(re.data.is_empty());
}

#[test]
fn replay_event_clone() {
    let re = ReplayEvent {
        tick: 7,
        event_type: "heal".into(),
        data: vec![50],
    };
    let re2 = re.clone();
    assert_eq!(re2.tick, 7);
    assert_eq!(re2.event_type, "heal");
    assert_eq!(re2.data, vec![50]);
}

#[test]
fn replay_event_debug() {
    let re = ReplayEvent {
        tick: 1,
        event_type: "test".into(),
        data: vec![],
    };
    let dbg = format!("{re:?}");
    assert!(dbg.contains("ReplayEvent"));
    assert!(dbg.contains("tick"));
}

// ═══════════════════════════════════════════════════════════════════════════
// SaveMetadata field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_metadata_fields() {
    let id = uuid::Uuid::new_v4();
    let now = time::OffsetDateTime::now_utc();
    let sm = SaveMetadata {
        player_id: "player1".into(),
        slot: 2,
        save_id: id,
        created_at: now,
        world_tick: 1000,
        world_hash: 9999,
    };
    assert_eq!(sm.player_id, "player1");
    assert_eq!(sm.slot, 2);
    assert_eq!(sm.save_id, id);
    assert_eq!(sm.world_tick, 1000);
    assert_eq!(sm.world_hash, 9999);
}

#[test]
fn save_metadata_json_roundtrip() {
    let sm = SaveMetadata {
        player_id: "tester".into(),
        slot: 0,
        save_id: uuid::Uuid::new_v4(),
        created_at: time::OffsetDateTime::now_utc(),
        world_tick: 500,
        world_hash: 12345,
    };
    let json = serde_json::to_string(&sm).unwrap();
    let back: SaveMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(back.player_id, "tester");
    assert_eq!(back.slot, 0);
    assert_eq!(back.world_tick, 500);
    assert_eq!(back.world_hash, 12345);
    assert_eq!(back.save_id, sm.save_id);
}

#[test]
fn save_metadata_clone() {
    let sm = SaveMetadata {
        player_id: "p".into(),
        slot: 3,
        save_id: uuid::Uuid::new_v4(),
        created_at: time::OffsetDateTime::now_utc(),
        world_tick: 10,
        world_hash: 42,
    };
    let sm2 = sm.clone();
    assert_eq!(sm2.player_id, "p");
    assert_eq!(sm2.slot, 3);
    assert_eq!(sm2.world_tick, 10);
    assert_eq!(sm2.world_hash, 42);
}

// ═══════════════════════════════════════════════════════════════════════════
// PersistencePlugin tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn persistence_plugin_new() {
    let dir = PathBuf::from("/tmp/saves");
    let _p = PersistencePlugin::new(dir);
    // Should not panic
}

#[test]
fn persistence_plugin_build() {
    let dir = PathBuf::from("/tmp/saves");
    let plugin = PersistencePlugin::new(dir);
    let mut app = astraweave_ecs::App::new();
    plugin.build(&mut app);
    // Should register systems without panic
}

// ═══════════════════════════════════════════════════════════════════════════
// serialize_ecs_world / deserialize_ecs_world tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn serialize_empty_world() {
    let world = World::new();
    let blob = serialize_ecs_world(&world).unwrap();
    assert!(!blob.is_empty(), "even empty world produces a blob header");
}

#[test]
fn deserialize_empty_blob() {
    let mut world = World::new();
    let result = deserialize_ecs_world(&[], &mut world);
    assert!(result.is_ok(), "empty blob should return Ok(())");
}

#[test]
fn serialize_deserialize_roundtrip_pos() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(
        entity,
        CPos {
            pos: IVec2 { x: 7, y: 13 },
        },
    );

    let blob = serialize_ecs_world(&world).unwrap();
    assert!(!blob.is_empty());

    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    // Query for CPos in new world
    let q = astraweave_ecs::Query::<CPos>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 1, "should have 1 entity with CPos");
    assert_eq!(entities[0].1.pos.x, 7);
    assert_eq!(entities[0].1.pos.y, 13);
}

#[test]
fn serialize_deserialize_roundtrip_health() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, CHealth { hp: 55 });

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CHealth>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].1.hp, 55);
}

#[test]
fn serialize_deserialize_roundtrip_team() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, CTeam { id: 4 });

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CTeam>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].1.id, 4);
}

#[test]
fn serialize_deserialize_roundtrip_ammo() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, CAmmo { rounds: 99 });

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CAmmo>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].1.rounds, 99);
}

#[test]
fn serialize_deserialize_roundtrip_cooldowns() {
    let mut world = World::new();
    let entity = world.spawn();
    let mut map = BTreeMap::new();
    map.insert(
        astraweave_core::ecs_components::cooldowns::CooldownKey::ThrowSmoke,
        3.5f32,
    );
    world.insert(entity, CCooldowns { map });

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CCooldowns>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 1);
    let cd_val = entities[0]
        .1
        .map
        .get(&astraweave_core::ecs_components::cooldowns::CooldownKey::ThrowSmoke);
    assert!(cd_val.is_some());
    assert!((cd_val.unwrap() - 3.5).abs() < f32::EPSILON);
}

#[test]
fn serialize_deserialize_roundtrip_desired_pos() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(
        entity,
        CDesiredPos {
            pos: IVec2 { x: -2, y: 8 },
        },
    );

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CDesiredPos>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].1.pos.x, -2);
    assert_eq!(entities[0].1.pos.y, 8);
}

#[test]
fn serialize_deserialize_roundtrip_ai_agent() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, CAiAgent);

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CAiAgent>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 1);
}

#[test]
fn serialize_deserialize_roundtrip_multiple_components() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(
        entity,
        CPos {
            pos: IVec2 { x: 3, y: 4 },
        },
    );
    world.insert(entity, CHealth { hp: 80 });
    world.insert(entity, CTeam { id: 1 });
    world.insert(entity, CAmmo { rounds: 25 });

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CPos>::new(&new_world);
    let pos_entities: Vec<_> = q.collect();
    assert_eq!(pos_entities.len(), 1);
    assert_eq!(pos_entities[0].1.pos.x, 3);
    assert_eq!(pos_entities[0].1.pos.y, 4);

    let q = astraweave_ecs::Query::<CHealth>::new(&new_world);
    let hp_entities: Vec<_> = q.collect();
    assert_eq!(hp_entities.len(), 1);
    assert_eq!(hp_entities[0].1.hp, 80);

    let q = astraweave_ecs::Query::<CTeam>::new(&new_world);
    let team_entities: Vec<_> = q.collect();
    assert_eq!(team_entities.len(), 1);
    assert_eq!(team_entities[0].1.id, 1);

    let q = astraweave_ecs::Query::<CAmmo>::new(&new_world);
    let ammo_entities: Vec<_> = q.collect();
    assert_eq!(ammo_entities.len(), 1);
    assert_eq!(ammo_entities[0].1.rounds, 25);
}

#[test]
fn serialize_deserialize_multiple_entities() {
    let mut world = World::new();
    for i in 0..10 {
        let entity = world.spawn();
        world.insert(
            entity,
            CPos {
                pos: IVec2 { x: i, y: i * 2 },
            },
        );
        world.insert(entity, CHealth { hp: 100 - i });
    }

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CPos>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 10);

    let q = astraweave_ecs::Query::<CHealth>::new(&new_world);
    let entities: Vec<_> = q.collect();
    assert_eq!(entities.len(), 10);
}

#[test]
fn serialize_blob_is_compact() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(
        entity,
        CPos {
            pos: IVec2 { x: 1, y: 1 },
        },
    );
    world.insert(entity, CHealth { hp: 100 });

    let blob = serialize_ecs_world(&world).unwrap();
    // postcard binary format should be much smaller than JSON
    let json = serde_json::to_string(&SerializedWorld {
        entities: vec![SerializedEntity {
            entity_raw: 0,
            pos: Some(CPos {
                pos: IVec2 { x: 1, y: 1 },
            }),
            health: Some(CHealth { hp: 100 }),
            team: None,
            ammo: None,
            cooldowns: None,
            desired_pos: None,
            ai_agent: None,
            legacy_id: None,
            persona: None,
            memory: None,
        }],
        world_tick: 0,
    })
    .unwrap();
    assert!(
        blob.len() < json.len(),
        "binary should be smaller than JSON: {} vs {}",
        blob.len(),
        json.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// calculate_world_hash tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn world_hash_empty_world() {
    let world = World::new();
    let _hash = calculate_world_hash(&world);
    // Should not panic; value is deterministic for empty world
}

#[test]
fn world_hash_deterministic() {
    let mut w1 = World::new();
    let e1 = w1.spawn();
    w1.insert(
        e1,
        CPos {
            pos: IVec2 { x: 3, y: 4 },
        },
    );
    w1.insert(e1, CHealth { hp: 100 });

    let mut w2 = World::new();
    let e2 = w2.spawn();
    w2.insert(
        e2,
        CPos {
            pos: IVec2 { x: 3, y: 4 },
        },
    );
    w2.insert(e2, CHealth { hp: 100 });

    let h1 = calculate_world_hash(&w1);
    let h2 = calculate_world_hash(&w2);
    assert_eq!(h1, h2, "same world state must produce same hash");
}

#[test]
fn world_hash_changes_with_pos() {
    let mut w1 = World::new();
    let e1 = w1.spawn();
    w1.insert(
        e1,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    w1.insert(e1, CHealth { hp: 100 });

    let mut w2 = World::new();
    let e2 = w2.spawn();
    w2.insert(
        e2,
        CPos {
            pos: IVec2 { x: 1, y: 0 },
        },
    );
    w2.insert(e2, CHealth { hp: 100 });

    let h1 = calculate_world_hash(&w1);
    let h2 = calculate_world_hash(&w2);
    assert_ne!(h1, h2, "different positions must produce different hash");
}

#[test]
fn world_hash_changes_with_hp() {
    let mut w1 = World::new();
    let e1 = w1.spawn();
    w1.insert(
        e1,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    w1.insert(e1, CHealth { hp: 100 });

    let mut w2 = World::new();
    let e2 = w2.spawn();
    w2.insert(
        e2,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    w2.insert(e2, CHealth { hp: 99 });

    let h1 = calculate_world_hash(&w1);
    let h2 = calculate_world_hash(&w2);
    assert_ne!(h1, h2, "different HP must produce different hash");
}

#[test]
fn world_hash_changes_with_team() {
    let mut w1 = World::new();
    let e1 = w1.spawn();
    w1.insert(
        e1,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    w1.insert(e1, CHealth { hp: 100 });
    w1.insert(e1, CTeam { id: 0 });

    let mut w2 = World::new();
    let e2 = w2.spawn();
    w2.insert(
        e2,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    w2.insert(e2, CHealth { hp: 100 });
    w2.insert(e2, CTeam { id: 1 });

    let h1 = calculate_world_hash(&w1);
    let h2 = calculate_world_hash(&w2);
    assert_ne!(h1, h2, "different team must produce different hash");
}

#[test]
fn world_hash_changes_with_ammo() {
    let mut w1 = World::new();
    let e1 = w1.spawn();
    w1.insert(
        e1,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    w1.insert(e1, CHealth { hp: 100 });
    w1.insert(e1, CAmmo { rounds: 10 });

    let mut w2 = World::new();
    let e2 = w2.spawn();
    w2.insert(
        e2,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    w2.insert(e2, CHealth { hp: 100 });
    w2.insert(e2, CAmmo { rounds: 11 });

    let h1 = calculate_world_hash(&w1);
    let h2 = calculate_world_hash(&w2);
    assert_ne!(h1, h2, "different ammo must produce different hash");
}

#[test]
fn world_hash_consistent_across_calls() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(
        e,
        CPos {
            pos: IVec2 { x: 5, y: 5 },
        },
    );
    world.insert(e, CHealth { hp: 100 });

    let h1 = calculate_world_hash(&world);
    let h2 = calculate_world_hash(&world);
    let h3 = calculate_world_hash(&world);
    assert_eq!(h1, h2);
    assert_eq!(h2, h3);
}

#[test]
fn world_hash_multiple_entities() {
    let mut world = World::new();
    for i in 0..5 {
        let e = world.spawn();
        world.insert(
            e,
            CPos {
                pos: IVec2 { x: i, y: 0 },
            },
        );
        world.insert(e, CHealth { hp: 100 });
    }
    let h = calculate_world_hash(&world);
    // Hash should be non-trivial for a populated world
    // Just ensure it doesn't panic and returns a value
    let _ = h;
}

// ═══════════════════════════════════════════════════════════════════════════
// CPersistenceManager tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn persistence_manager_set_player() {
    let dir = tempfile::tempdir().unwrap();
    let mut pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: String::new(),
    };
    assert_eq!(pm.current_player, "");
    pm.set_player("hero_42");
    assert_eq!(pm.current_player, "hero_42");
}

#[test]
fn persistence_manager_set_player_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let mut pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: "old".into(),
    };
    pm.set_player("new");
    assert_eq!(pm.current_player, "new");
    assert_ne!(pm.current_player, "old");
}

#[test]
fn persistence_manager_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let mut pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: String::new(),
    };
    pm.set_player("test_player");

    // Save with some blob data
    let blob = vec![1, 2, 3, 4, 5];
    let result = pm.save_game(0, 100, 999, blob.clone());
    assert!(result.is_ok(), "save should succeed: {:?}", result.err());

    // Load it back
    let (bundle, _path) = pm.load_game(0).unwrap();
    assert_eq!(bundle.world.tick, 100);
    assert_eq!(bundle.world.state_hash, 999);
    assert_eq!(bundle.world.ecs_blob, blob);
    assert_eq!(bundle.slot, 0);
    assert_eq!(bundle.player_id, "test_player");
}

#[test]
fn persistence_manager_save_has_credits() {
    let dir = tempfile::tempdir().unwrap();
    let mut pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: String::new(),
    };
    pm.set_player("p");
    let mut world = astraweave_ecs::World::new();
    let entity = world.spawn();
    world.insert(entity, CAmmo { rounds: 7 });
    let blob = serialize_ecs_world(&world).unwrap();
    pm.save_game(0, 0, 0, blob).unwrap();
    let (bundle, _) = pm.load_game(0).unwrap();
    assert_eq!(
        bundle.inventory.credits, 7,
        "credits come from CAmmo in the blob"
    );
}

#[test]
fn persistence_manager_save_has_engine_version() {
    let dir = tempfile::tempdir().unwrap();
    let mut pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: String::new(),
    };
    pm.set_player("p");
    pm.save_game(0, 0, 0, vec![]).unwrap();
    let (bundle, _) = pm.load_game(0).unwrap();
    assert!(bundle.meta.contains_key("engine_version"));
    assert_eq!(bundle.meta["engine_version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn persistence_manager_different_slots() {
    let dir = tempfile::tempdir().unwrap();
    let mut pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: String::new(),
    };
    pm.set_player("multi_slot");

    pm.save_game(0, 100, 1, vec![10]).unwrap();
    pm.save_game(1, 200, 2, vec![20]).unwrap();

    let (b0, _) = pm.load_game(0).unwrap();
    assert_eq!(b0.world.tick, 100);
    assert_eq!(b0.world.ecs_blob, vec![10]);

    let (b1, _) = pm.load_game(1).unwrap();
    assert_eq!(b1.world.tick, 200);
    assert_eq!(b1.world.ecs_blob, vec![20]);
}

#[test]
fn persistence_manager_load_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: "nobody".into(),
    };
    let result = pm.load_game(0);
    assert!(result.is_err(), "loading from empty save dir should fail");
}

#[test]
fn persistence_manager_list_saves_no_player_dir_errors() {
    let dir = tempfile::tempdir().unwrap();
    let pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: "nonexistent_player".into(),
    };
    // Player directory doesn't exist yet, so list_saves should error
    let result = pm.list_saves();
    assert!(
        result.is_err(),
        "listing saves for non-existent player dir should fail"
    );
}

#[test]
fn persistence_manager_list_saves_after_save() {
    let dir = tempfile::tempdir().unwrap();
    let mut pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: String::new(),
    };
    pm.set_player("lister");
    pm.save_game(0, 50, 0, vec![]).unwrap();
    let saves = pm.list_saves().unwrap();
    assert!(!saves.is_empty(), "should have at least 1 save");
}

// ═══════════════════════════════════════════════════════════════════════════
// Serialization roundtrip: JSON for SerializedEntity/World
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn serialized_entity_json_roundtrip() {
    let se = SerializedEntity {
        entity_raw: 123,
        pos: Some(CPos {
            pos: IVec2 { x: -1, y: 5 },
        }),
        health: Some(CHealth { hp: 42 }),
        team: Some(CTeam { id: 2 }),
        ammo: Some(CAmmo { rounds: 15 }),
        cooldowns: None,
        desired_pos: None,
        ai_agent: None,
        legacy_id: None,
        persona: None,
        memory: None,
    };
    let json = serde_json::to_string(&se).unwrap();
    let back: SerializedEntity = serde_json::from_str(&json).unwrap();
    assert_eq!(back.entity_raw, 123);
    assert_eq!(back.pos.unwrap().pos.x, -1);
    assert_eq!(back.pos.unwrap().pos.y, 5);
    assert_eq!(back.health.unwrap().hp, 42);
    assert_eq!(back.team.unwrap().id, 2);
    assert_eq!(back.ammo.unwrap().rounds, 15);
}

#[test]
fn serialized_world_json_roundtrip() {
    let sw = SerializedWorld {
        entities: vec![SerializedEntity {
            entity_raw: 7,
            pos: Some(CPos {
                pos: IVec2 { x: 0, y: 0 },
            }),
            health: None,
            team: None,
            ammo: None,
            cooldowns: None,
            desired_pos: None,
            ai_agent: None,
            legacy_id: None,
            persona: None,
            memory: None,
        }],
        world_tick: 77,
    };
    let json = serde_json::to_string(&sw).unwrap();
    let back: SerializedWorld = serde_json::from_str(&json).unwrap();
    assert_eq!(back.world_tick, 77);
    assert_eq!(back.entities.len(), 1);
    assert_eq!(back.entities[0].entity_raw, 7);
}

#[test]
fn replay_state_json_roundtrip() {
    let rs = CReplayState {
        is_replaying: true,
        current_tick: 5,
        total_ticks: 100,
        events: vec![ReplayEvent {
            tick: 3,
            event_type: "test".into(),
            data: vec![1, 2],
        }],
    };
    let json = serde_json::to_string(&rs).unwrap();
    let back: CReplayState = serde_json::from_str(&json).unwrap();
    assert!(back.is_replaying);
    assert_eq!(back.current_tick, 5);
    assert_eq!(back.total_ticks, 100);
    assert_eq!(back.events.len(), 1);
    assert_eq!(back.events[0].tick, 3);
}

// ═══════════════════════════════════════════════════════════════════════════
// Boundary / stress tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn serialize_deserialize_100_entities() {
    let mut world = World::new();
    for i in 0..100 {
        let entity = world.spawn();
        world.insert(
            entity,
            CPos {
                pos: IVec2 { x: i, y: i * 3 },
            },
        );
        world.insert(entity, CHealth { hp: 100 + i });
        world.insert(entity, CTeam { id: (i % 3) as u8 });
        world.insert(entity, CAmmo { rounds: i * 10 });
    }

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q = astraweave_ecs::Query::<CPos>::new(&new_world);
    let count: usize = q.count();
    assert_eq!(count, 100);
}

#[test]
fn world_hash_zero_hp_entity() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(
        e,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    world.insert(e, CHealth { hp: 0 });
    let _h = calculate_world_hash(&world);
    // Should not panic with zero HP
}

#[test]
fn world_hash_negative_pos() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(
        e,
        CPos {
            pos: IVec2 { x: -100, y: -200 },
        },
    );
    world.insert(e, CHealth { hp: 50 });

    let h1 = calculate_world_hash(&world);
    let h2 = calculate_world_hash(&world);
    assert_eq!(h1, h2, "hash must be deterministic for negative coords");
}

#[test]
fn deserialized_entities_have_correct_component_count() {
    let mut world = World::new();
    // Entity 1: only pos
    let e1 = world.spawn();
    world.insert(
        e1,
        CPos {
            pos: IVec2 { x: 1, y: 1 },
        },
    );
    // Entity 2: pos + health + ammo
    let e2 = world.spawn();
    world.insert(
        e2,
        CPos {
            pos: IVec2 { x: 2, y: 2 },
        },
    );
    world.insert(e2, CHealth { hp: 50 });
    world.insert(e2, CAmmo { rounds: 20 });

    let blob = serialize_ecs_world(&world).unwrap();
    let mut new_world = World::new();
    deserialize_ecs_world(&blob, &mut new_world).unwrap();

    let q_pos = astraweave_ecs::Query::<CPos>::new(&new_world);
    assert_eq!(q_pos.count(), 2, "both entities have CPos");

    let q_hp = astraweave_ecs::Query::<CHealth>::new(&new_world);
    assert_eq!(q_hp.count(), 1, "only entity 2 has CHealth");

    let q_ammo = astraweave_ecs::Query::<CAmmo>::new(&new_world);
    assert_eq!(q_ammo.count(), 1, "only entity 2 has CAmmo");
}

// ═══════════════════════════════════════════════════════════════════════════
// start_replay test
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn start_replay_creates_correct_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: String::new(),
    };
    pm.set_player("replayer");
    pm.save_game(0, 500, 0, vec![]).unwrap();

    let replay = pm.start_replay(0).unwrap();
    assert!(replay.is_replaying, "replay should start as replaying");
    assert_eq!(replay.current_tick, 0, "replay starts at tick 0");
    assert_eq!(
        replay.total_ticks, 500,
        "total_ticks from save's world tick"
    );
    assert!(replay.events.is_empty(), "events start empty");
}

#[test]
fn start_replay_nonexistent_slot_fails() {
    let dir = tempfile::tempdir().unwrap();
    let pm = CPersistenceManager {
        save_manager: aw_save::SaveManager::new(dir.path().to_path_buf()),
        current_player: "ghost".into(),
    };
    let result = pm.start_replay(0);
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// Mutation kill tests: replay_system via PersistencePlugin
// ═══════════════════════════════════════════════════════════════════════════

/// Helper: build an App whose schedule includes "pre_simulation" and
/// "post_simulation" stages so PersistencePlugin::build can register its
/// systems.  Without these stages the plugin's add_system calls are silent
/// no-ops.
fn app_with_persistence_stages() -> App {
    let schedule = Schedule::default()
        .with_stage("pre_simulation")
        .with_stage("post_simulation");
    let mut app = App {
        world: World::new(),
        schedule,
    };
    let plugin = PersistencePlugin::new(PathBuf::from("."));
    plugin.build(&mut app);
    app
}

/// Kills: `PersistencePlugin::build → ()` (miss 1)
///        `replay_system → ()` (miss 2)
///        `current_tick += 1  →  -= 1` (miss 6)
///        `current_tick += 1  →  *= 1` (miss 7)
///
/// Spawn an entity with CReplayState at tick 0 / total 3, run one tick,
/// assert current_tick == 1.  If plugin build is empty the system never
/// runs (tick stays 0).  If replay_system body is empty, same.  If += is
/// replaced with -=, tick wraps to u64::MAX.  If += is replaced with *=,
/// 0*1 == 0 (stays 0).
#[test]
fn replay_system_advances_tick_from_zero() {
    let mut app = app_with_persistence_stages();
    let entity = app.world.spawn();
    app.world.insert(
        entity,
        CReplayState {
            is_replaying: true,
            current_tick: 0,
            total_ticks: 3,
            events: vec![],
        },
    );

    app.schedule.run(&mut app.world);

    let replay = app.world.get::<CReplayState>(entity).unwrap();
    assert_eq!(replay.current_tick, 1, "tick must advance from 0 to 1");
    assert!(replay.is_replaying, "still replaying (1 < 3)");
}

/// Kills: `current_tick < total_ticks  →  == total_ticks` (miss 3)
///        `current_tick < total_ticks  →  >  total_ticks` (miss 4)
///
/// With total_ticks=2: tick 0→1→2 over two runs, then a third run sees
/// 2 < 2 = false and sets is_replaying = false.
/// The `==` mutant only advances when tick==total, so tick 0 doesn't match
/// and tick stays 0.  The `>` mutant never enters the branch.
#[test]
fn replay_system_advances_midway() {
    let mut app = app_with_persistence_stages();
    let entity = app.world.spawn();
    app.world.insert(
        entity,
        CReplayState {
            is_replaying: true,
            current_tick: 0,
            total_ticks: 2,
            events: vec![],
        },
    );

    // Run 1: 0 → 1
    app.schedule.run(&mut app.world);
    let r = app.world.get::<CReplayState>(entity).unwrap();
    assert_eq!(r.current_tick, 1);
    assert!(r.is_replaying);

    // Run 2: 1 → 2 (still replaying — the else branch is checked next run)
    app.schedule.run(&mut app.world);
    let r = app.world.get::<CReplayState>(entity).unwrap();
    assert_eq!(r.current_tick, 2);
    assert!(
        r.is_replaying,
        "increment happens in this pass, is_replaying cleared next pass"
    );

    // Run 3: 2 < 2 is false → is_replaying = false
    app.schedule.run(&mut app.world);
    let r = app.world.get::<CReplayState>(entity).unwrap();
    assert_eq!(r.current_tick, 2, "tick must not advance past total");
    assert!(!r.is_replaying, "replay must be done when tick >= total");
}

/// Kills: `current_tick < total_ticks  →  <= total_ticks` (miss 5)
///
/// With total_ticks=1, run once (0→1).  Original code: 1 < 1 is false,
/// so is_replaying = false.  The `<=` mutant would treat 1<=1 as true and
/// advance again to 2, keeping is_replaying=true.  We assert tick==1 and
/// is_replaying==false after one tick.
#[test]
fn replay_system_stops_exactly_at_total_ticks() {
    let mut app = app_with_persistence_stages();
    let entity = app.world.spawn();
    app.world.insert(
        entity,
        CReplayState {
            is_replaying: true,
            current_tick: 0,
            total_ticks: 1,
            events: vec![],
        },
    );

    // tick 0→1, then check 1<1 → false → is_replaying = false
    app.schedule.run(&mut app.world);
    let r = app.world.get::<CReplayState>(entity).unwrap();
    assert_eq!(r.current_tick, 1, "tick must advance exactly once");
    // After the increment, the *next* iteration's check (or a second
    // schedule run) will determine is_replaying.  In the current code
    // the update happens in one pass per entity — the first get_mut
    // increments to 1, then the function returns.  On the *next* run,
    // 1 < 1 is false → set is_replaying = false.
    app.schedule.run(&mut app.world);
    let r = app.world.get::<CReplayState>(entity).unwrap();
    assert_eq!(r.current_tick, 1, "tick must NOT advance past total_ticks");
    assert!(!r.is_replaying, "replay must be done");
}
