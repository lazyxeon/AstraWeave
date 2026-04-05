//! Mutation-resistant comprehensive tests for aw-save.

use aw_save::*;
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// SAVE_SCHEMA_VERSION constant
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_schema_version_is_two() {
    assert_eq!(SAVE_SCHEMA_VERSION, 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// WorldState field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn world_state_fields() {
    let ws = WorldState {
        tick: 42,
        ecs_blob: vec![1, 2, 3],
        state_hash: 999,
    };
    assert_eq!(ws.tick, 42);
    assert_eq!(ws.ecs_blob, vec![1, 2, 3]);
    assert_eq!(ws.state_hash, 999);
}

#[test]
fn world_state_empty_blob() {
    let ws = WorldState {
        tick: 0,
        ecs_blob: vec![],
        state_hash: 0,
    };
    assert_eq!(ws.tick, 0);
    assert!(ws.ecs_blob.is_empty());
    assert_eq!(ws.state_hash, 0);
}

#[test]
fn world_state_clone() {
    let ws = WorldState {
        tick: 100,
        ecs_blob: vec![10],
        state_hash: 55,
    };
    let ws2 = ws.clone();
    assert_eq!(ws2.tick, 100);
    assert_eq!(ws2.ecs_blob, vec![10]);
    assert_eq!(ws2.state_hash, 55);
}

#[test]
fn world_state_json_roundtrip() {
    let ws = WorldState {
        tick: 77,
        ecs_blob: vec![4, 5],
        state_hash: 12345,
    };
    let json = serde_json::to_string(&ws).unwrap();
    let back: WorldState = serde_json::from_str(&json).unwrap();
    assert_eq!(back.tick, 77);
    assert_eq!(back.ecs_blob, vec![4, 5]);
    assert_eq!(back.state_hash, 12345);
}

// ═══════════════════════════════════════════════════════════════════════════
// PlayerInventory field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn player_inventory_fields() {
    let inv = PlayerInventory {
        credits: 1000,
        items: vec![],
    };
    assert_eq!(inv.credits, 1000);
    assert!(inv.items.is_empty());
}

#[test]
fn player_inventory_with_items() {
    let inv = PlayerInventory {
        credits: 500,
        items: vec![
            ItemStack {
                kind: "sword".into(),
                qty: 1,
                attrs: HashMap::new(),
            },
            ItemStack {
                kind: "potion".into(),
                qty: 5,
                attrs: HashMap::new(),
            },
        ],
    };
    assert_eq!(inv.credits, 500);
    assert_eq!(inv.items.len(), 2);
    assert_eq!(inv.items[0].kind, "sword");
    assert_eq!(inv.items[0].qty, 1);
    assert_eq!(inv.items[1].kind, "potion");
    assert_eq!(inv.items[1].qty, 5);
}

#[test]
fn player_inventory_clone() {
    let inv = PlayerInventory {
        credits: 42,
        items: vec![],
    };
    let inv2 = inv.clone();
    assert_eq!(inv2.credits, 42);
}

// ═══════════════════════════════════════════════════════════════════════════
// ItemStack field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn item_stack_fields() {
    let mut attrs = HashMap::new();
    attrs.insert("damage".into(), 50);
    let item = ItemStack {
        kind: "axe".into(),
        qty: 3,
        attrs,
    };
    assert_eq!(item.kind, "axe");
    assert_eq!(item.qty, 3);
    assert_eq!(item.attrs["damage"], 50);
}

#[test]
fn item_stack_empty_attrs() {
    let item = ItemStack {
        kind: "gem".into(),
        qty: 10,
        attrs: HashMap::new(),
    };
    assert!(item.attrs.is_empty());
    assert_eq!(item.qty, 10);
}

#[test]
fn item_stack_json_roundtrip() {
    let mut attrs = HashMap::new();
    attrs.insert("power".into(), 100);
    let item = ItemStack {
        kind: "ring".into(),
        qty: 1,
        attrs,
    };
    let json = serde_json::to_string(&item).unwrap();
    let back: ItemStack = serde_json::from_str(&json).unwrap();
    assert_eq!(back.kind, "ring");
    assert_eq!(back.qty, 1);
    assert_eq!(back.attrs["power"], 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// CompanionProfile field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn companion_profile_fields() {
    let cp = CompanionProfile {
        id: "comp_01".into(),
        name: "Luna".into(),
        level: 5,
        skills: vec!["heal".into(), "shield".into()],
        facts: vec!["likes_cats".into()],
        episodes_summarized: vec![],
    };
    assert_eq!(cp.id, "comp_01");
    assert_eq!(cp.name, "Luna");
    assert_eq!(cp.level, 5);
    assert_eq!(cp.skills.len(), 2);
    assert_eq!(cp.skills[0], "heal");
    assert_eq!(cp.skills[1], "shield");
    assert_eq!(cp.facts.len(), 1);
    assert!(cp.episodes_summarized.is_empty());
}

#[test]
fn companion_profile_clone() {
    let cp = CompanionProfile {
        id: "c".into(),
        name: "N".into(),
        level: 1,
        skills: vec![],
        facts: vec![],
        episodes_summarized: vec!["ep1".into()],
    };
    let cp2 = cp.clone();
    assert_eq!(cp2.id, "c");
    assert_eq!(cp2.level, 1);
    assert_eq!(cp2.episodes_summarized.len(), 1);
}

#[test]
fn companion_profile_json_roundtrip() {
    let cp = CompanionProfile {
        id: "x".into(),
        name: "Y".into(),
        level: 10,
        skills: vec!["fireball".into()],
        facts: vec![],
        episodes_summarized: vec![],
    };
    let json = serde_json::to_string(&cp).unwrap();
    let back: CompanionProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(back.level, 10);
    assert_eq!(back.skills[0], "fireball");
}

// ═══════════════════════════════════════════════════════════════════════════
// SaveBundleV2 field-level tests
// ═══════════════════════════════════════════════════════════════════════════

fn make_bundle() -> SaveBundleV2 {
    SaveBundleV2 {
        schema: SAVE_SCHEMA_VERSION,
        save_id: uuid::Uuid::new_v4(),
        created_at: time::OffsetDateTime::now_utc(),
        player_id: "player_1".into(),
        slot: 0,
        world: WorldState {
            tick: 100,
            ecs_blob: vec![1, 2],
            state_hash: 42,
        },
        companions: vec![CompanionProfile {
            id: "comp".into(),
            name: "Buddy".into(),
            level: 3,
            skills: vec![],
            facts: vec![],
            episodes_summarized: vec![],
        }],
        inventory: PlayerInventory {
            credits: 1000,
            items: vec![],
        },
        meta: HashMap::new(),
    }
}

#[test]
fn save_bundle_v2_schema() {
    let bundle = make_bundle();
    assert_eq!(bundle.schema, 2);
}

#[test]
fn save_bundle_v2_player_id() {
    let bundle = make_bundle();
    assert_eq!(bundle.player_id, "player_1");
}

#[test]
fn save_bundle_v2_slot() {
    let bundle = make_bundle();
    assert_eq!(bundle.slot, 0);
}

#[test]
fn save_bundle_v2_world_tick() {
    let bundle = make_bundle();
    assert_eq!(bundle.world.tick, 100);
}

#[test]
fn save_bundle_v2_world_ecs_blob() {
    let bundle = make_bundle();
    assert_eq!(bundle.world.ecs_blob, vec![1, 2]);
}

#[test]
fn save_bundle_v2_world_hash() {
    let bundle = make_bundle();
    assert_eq!(bundle.world.state_hash, 42);
}

#[test]
fn save_bundle_v2_companions() {
    let bundle = make_bundle();
    assert_eq!(bundle.companions.len(), 1);
    assert_eq!(bundle.companions[0].name, "Buddy");
    assert_eq!(bundle.companions[0].level, 3);
}

#[test]
fn save_bundle_v2_inventory_credits() {
    let bundle = make_bundle();
    assert_eq!(bundle.inventory.credits, 1000);
}

#[test]
fn save_bundle_v2_clone() {
    let b = make_bundle();
    let b2 = b.clone();
    assert_eq!(b2.schema, 2);
    assert_eq!(b2.player_id, "player_1");
    assert_eq!(b2.world.tick, 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// SaveBundleV1 → V2 migration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_bundle_v1_into_v2_schema() {
    let v1 = SaveBundleV1 {
        player_id: "old_player".into(),
        slot: 2,
        created_at: time::OffsetDateTime::now_utc(),
        world: WorldState {
            tick: 50,
            ecs_blob: vec![3, 4],
            state_hash: 7,
        },
        inventory: PlayerInventory {
            credits: 500,
            items: vec![],
        },
        companion: None,
        meta: HashMap::new(),
    };
    let v2 = v1.into_v2();
    assert_eq!(
        v2.schema, SAVE_SCHEMA_VERSION,
        "migrated schema must be {}",
        SAVE_SCHEMA_VERSION
    );
    assert_eq!(v2.player_id, "old_player");
    assert_eq!(v2.slot, 2);
    assert_eq!(v2.world.tick, 50);
    assert!(v2.companions.is_empty(), "None companion → empty vec");
}

#[test]
fn save_bundle_v1_into_v2_with_companion() {
    let comp = CompanionProfile {
        id: "c".into(),
        name: "Ally".into(),
        level: 7,
        skills: vec!["stealth".into()],
        facts: vec![],
        episodes_summarized: vec![],
    };
    let v1 = SaveBundleV1 {
        player_id: "p".into(),
        slot: 0,
        created_at: time::OffsetDateTime::now_utc(),
        world: WorldState {
            tick: 0,
            ecs_blob: vec![],
            state_hash: 0,
        },
        inventory: PlayerInventory {
            credits: 0,
            items: vec![],
        },
        companion: Some(comp),
        meta: HashMap::new(),
    };
    let v2 = v1.into_v2();
    assert_eq!(v2.companions.len(), 1);
    assert_eq!(v2.companions[0].name, "Ally");
    assert_eq!(v2.companions[0].level, 7);
}

#[test]
fn save_bundle_v1_into_v2_preserves_meta() {
    let mut meta = HashMap::new();
    meta.insert("engine_version".into(), "0.10.0".into());
    let v1 = SaveBundleV1 {
        player_id: "p".into(),
        slot: 0,
        created_at: time::OffsetDateTime::now_utc(),
        world: WorldState {
            tick: 0,
            ecs_blob: vec![],
            state_hash: 0,
        },
        inventory: PlayerInventory {
            credits: 0,
            items: vec![],
        },
        companion: None,
        meta,
    };
    let v2 = v1.into_v2();
    assert_eq!(v2.meta["engine_version"], "0.10.0");
}

// ═══════════════════════════════════════════════════════════════════════════
// SaveManager save/load roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_manager_player_dir() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let pdir = sm.player_dir("hero");
    assert!(pdir.ends_with("hero"));
}

#[test]
fn save_manager_save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let bundle = make_bundle();
    let save_path = sm.save("player_1", 0, bundle.clone()).unwrap();
    assert!(save_path.exists(), "save file should exist on disk");

    let (loaded, loaded_path) = sm.load_latest_slot("player_1", 0).unwrap();
    assert_eq!(loaded.schema, 2);
    assert_eq!(loaded.player_id, "player_1");
    assert_eq!(loaded.slot, 0);
    assert_eq!(loaded.world.tick, 100);
    assert_eq!(loaded.world.ecs_blob, vec![1, 2]);
    assert_eq!(loaded.world.state_hash, 42);
    assert_eq!(loaded.inventory.credits, 1000);
    assert!(!loaded_path.as_os_str().is_empty());
}

#[test]
fn save_manager_multiple_slots() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());

    let mut b0 = make_bundle();
    b0.slot = 0;
    b0.world.tick = 100;
    sm.save("player_1", 0, b0).unwrap();

    let mut b1 = make_bundle();
    b1.slot = 1;
    b1.world.tick = 200;
    sm.save("player_1", 1, b1).unwrap();

    let (l0, _) = sm.load_latest_slot("player_1", 0).unwrap();
    assert_eq!(l0.world.tick, 100);

    let (l1, _) = sm.load_latest_slot("player_1", 1).unwrap();
    assert_eq!(l1.world.tick, 200);
}

#[test]
fn save_manager_load_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let result = sm.load_latest_slot("nobody", 0);
    assert!(result.is_err());
}

#[test]
fn save_manager_list_saves() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let bundle = make_bundle();
    sm.save("player_1", 0, bundle).unwrap();

    let saves = sm.list_saves("player_1").unwrap();
    assert!(!saves.is_empty());
    assert_eq!(saves[0].player_id, "player_1");
    assert_eq!(saves[0].slot, 0);
    assert_eq!(saves[0].schema, 2);
}

#[test]
fn save_manager_clone() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let sm2 = sm.clone();
    let pdir = sm2.player_dir("test");
    assert!(pdir.ends_with("test"));
}

// ═══════════════════════════════════════════════════════════════════════════
// SaveMeta field-level tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_meta_from_list() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let bundle = make_bundle();
    sm.save("player_1", 0, bundle.clone()).unwrap();

    let saves = sm.list_saves("player_1").unwrap();
    let meta = &saves[0];
    assert_eq!(meta.player_id, "player_1");
    assert_eq!(meta.slot, 0);
    assert_eq!(meta.schema, 2);
    assert!(!meta.file.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// Migration: migrate_file_to_latest
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn migrate_latest_version_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let bundle = make_bundle();
    let save_path = sm.save("player_1", 0, bundle.clone()).unwrap();

    let migrated = sm.migrate_file_to_latest(&save_path, false).unwrap();
    assert_eq!(migrated.schema, 2);
    assert_eq!(migrated.player_id, "player_1");
    assert_eq!(migrated.world.tick, 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// Boundary conditions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_large_ecs_blob() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let mut bundle = make_bundle();
    bundle.world.ecs_blob = vec![0xAB; 100_000];
    sm.save("stress", 0, bundle.clone()).unwrap();

    let (loaded, _) = sm.load_latest_slot("stress", 0).unwrap();
    assert_eq!(loaded.world.ecs_blob.len(), 100_000);
    assert!(loaded.world.ecs_blob.iter().all(|&b| b == 0xAB));
}

#[test]
fn save_with_many_items() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let mut bundle = make_bundle();
    for i in 0..100 {
        bundle.inventory.items.push(ItemStack {
            kind: format!("item_{i}"),
            qty: (i + 1) as u32,
            attrs: HashMap::new(),
        });
    }
    sm.save("items", 0, bundle.clone()).unwrap();

    let (loaded, _) = sm.load_latest_slot("items", 0).unwrap();
    assert_eq!(loaded.inventory.items.len(), 100);
    assert_eq!(loaded.inventory.items[0].kind, "item_0");
    assert_eq!(loaded.inventory.items[0].qty, 1);
    assert_eq!(loaded.inventory.items[99].kind, "item_99");
    assert_eq!(loaded.inventory.items[99].qty, 100);
}

#[test]
fn save_with_many_companions() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let mut bundle = make_bundle();
    bundle.companions.clear();
    for i in 0..10 {
        bundle.companions.push(CompanionProfile {
            id: format!("comp_{i}"),
            name: format!("Companion {i}"),
            level: (i as u8) + 1,
            skills: vec![format!("skill_{i}")],
            facts: vec![],
            episodes_summarized: vec![],
        });
    }
    sm.save("comps", 0, bundle).unwrap();

    let (loaded, _) = sm.load_latest_slot("comps", 0).unwrap();
    assert_eq!(loaded.companions.len(), 10);
    assert_eq!(loaded.companions[0].level, 1);
    assert_eq!(loaded.companions[9].level, 10);
}

#[test]
fn save_max_slot_255() {
    let dir = tempfile::tempdir().unwrap();
    let sm = SaveManager::new(dir.path());
    let mut bundle = make_bundle();
    bundle.slot = 255;
    sm.save("max_slot", 255, bundle).unwrap();

    let (loaded, _) = sm.load_latest_slot("max_slot", 255).unwrap();
    assert_eq!(loaded.slot, 255);
}
