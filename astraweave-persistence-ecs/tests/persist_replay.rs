//! `persist_replay` digest. Replay applies events in order. Autosave ids come from the sim tick.

use std::path::PathBuf;

use astraweave_core::ecs_components::{CAmmo, CHealth};
use astraweave_ecs::{App, Plugin, Schedule, World};
use astraweave_persistence_ecs::{
    deserialize_ecs_world, serialize_ecs_world, AppliedReplay, CPersistenceManager, CReplayState,
    CSimClock, PersistencePlugin, ReplayEvent, SerializedWorld,
};
use aw_save::{sim_save_id, SaveManager};
use sha2::{Digest, Sha256};

fn replay_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash-persist.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/replay-hash-persist.json")
}

fn app() -> App {
    let schedule = Schedule::default()
        .with_stage("pre_simulation")
        .with_stage("post_simulation");
    App {
        world: World::new(),
        schedule,
    }
}

fn scenario_bytes() -> Vec<u8> {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut app = app();
    let plugin = PersistencePlugin::new(dir.path().to_path_buf());
    plugin.build(&mut app);
    let entity = app.world.spawn();
    app.world.insert(
        entity,
        CReplayState {
            is_replaying: true,
            current_tick: 0,
            total_ticks: 2,
            events: vec![
                ReplayEvent {
                    tick: 0,
                    event_type: "hit".into(),
                    data: vec![2],
                },
                ReplayEvent {
                    tick: 0,
                    event_type: "hit".into(),
                    data: vec![1],
                },
                ReplayEvent {
                    tick: 1,
                    event_type: "cover".into(),
                    data: vec![1],
                },
            ],
        },
    );
    let saver = app.world.spawn();
    app.world.insert(
        saver,
        CPersistenceManager {
            save_manager: SaveManager::new(dir.path()),
            current_player: "aravath".into(),
        },
    );
    app.world.insert_resource(CSimClock {
        world_seed: 0xA2A7,
        tick: 10,
        save_every: 10,
        last_saved_tick: None,
        slot: 0,
    });
    app.schedule.run(&mut app.world);
    app.schedule.run(&mut app.world);
    let applied = app
        .world
        .get_resource::<AppliedReplay>()
        .expect("applied replay");
    assert_eq!(applied.records.len(), 3);
    assert_eq!(applied.records[0].data, vec![1]);
    assert_eq!(applied.records[1].data, vec![2]);
    assert_eq!(applied.records[2].event_type, "cover");
    let (bundle, _) = SaveManager::new(dir.path())
        .load_latest_slot("aravath", 0)
        .expect("autosave");
    let player_seed = "aravath".bytes().fold(0u64, |acc, byte| {
        acc.wrapping_mul(0x1000_0000_01B3)
            .wrapping_add(u64::from(byte))
    });
    assert_eq!(bundle.save_id, sim_save_id(player_seed, 10, 0));
    assert_eq!(bundle.created_at.unix_timestamp(), 10);
    assert_eq!(bundle.world.tick, 10);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(bundle.save_id.as_bytes());
    bytes.extend_from_slice(&bundle.world.tick.to_le_bytes());
    for record in &applied.records {
        bytes.extend_from_slice(&record.tick.to_le_bytes());
        bytes.extend_from_slice(&(record.event_type.len() as u64).to_le_bytes());
        bytes.extend_from_slice(record.event_type.as_bytes());
        bytes.extend_from_slice(&record.data);
    }
    bytes
}

#[test]
fn persist_replay_is_stable_and_written() {
    let first = scenario_bytes();
    let second = scenario_bytes();
    assert_eq!(first, second);
    let digest = hex_encode(&Sha256::digest(&first));
    let path = replay_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("target dir");
    }
    let body = format!("[\n  {{ \"case\": \"persist_replay\", \"sha256\": \"{digest}\" }}\n]\n");
    std::fs::write(&path, body).expect("write replay hash");
    println!("persist_replay {digest}");
}

fn payload_hash(world: &SerializedWorld) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mix = |hash: &mut u64, bytes: &[u8]| {
        for byte in bytes {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(0x1000_0000_01B3);
        }
    };
    mix(&mut hash, &world.world_tick.to_le_bytes());
    for entity in &world.entities {
        if let Some(health) = entity.health {
            mix(&mut hash, &health.hp.to_le_bytes());
        }
        if let Some(ammo) = entity.ammo {
            mix(&mut hash, &ammo.rounds.to_le_bytes());
        }
    }
    hash
}

#[test]
fn restored_health_hashes_like_the_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut app = app();
    let plugin = PersistencePlugin::new(dir.path().to_path_buf());
    plugin.build(&mut app);
    let actor = app.world.spawn();
    app.world.insert(actor, CHealth { hp: 100 });
    app.world.insert(actor, CAmmo { rounds: 7 });
    app.world.insert(
        actor,
        CReplayState {
            is_replaying: true,
            current_tick: 0,
            total_ticks: 1,
            events: vec![ReplayEvent {
                tick: 0,
                event_type: "health".into(),
                data: 40i32.to_le_bytes().to_vec(),
            }],
        },
    );
    let saver = app.world.spawn();
    app.world.insert(
        saver,
        CPersistenceManager {
            save_manager: SaveManager::new(dir.path()),
            current_player: "aravath".into(),
        },
    );
    app.world.insert_resource(CSimClock {
        world_seed: 0xA2A7,
        tick: 8,
        save_every: 1,
        last_saved_tick: None,
        slot: 0,
    });
    app.schedule.run(&mut app.world);
    assert_eq!(app.world.get::<CHealth>(actor).expect("health").hp, 40);

    let (bundle, _) = SaveManager::new(dir.path())
        .load_latest_slot("aravath", 0)
        .expect("autosave");
    let replay = CPersistenceManager {
        save_manager: SaveManager::new(dir.path()),
        current_player: "aravath".into(),
    }
    .start_replay(0)
    .expect("replay");
    assert_eq!(replay.events.len(), 1);
    assert_eq!(replay.events[0].event_type, "health");

    let source: SerializedWorld = postcard::from_bytes(&bundle.world.ecs_blob).expect("blob");
    let mut fresh = World::new();
    deserialize_ecs_world(&bundle.world.ecs_blob, &mut fresh).expect("restore");
    let again = serialize_ecs_world(&fresh).expect("reserialize");
    let restored: SerializedWorld = postcard::from_bytes(&again).expect("restored blob");
    assert_eq!(source.world_tick, 8);
    assert_eq!(payload_hash(&source), payload_hash(&restored));
    assert_eq!(
        restored
            .entities
            .iter()
            .filter_map(|entity| entity.health)
            .next()
            .expect("hp")
            .hp,
        40
    );
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
