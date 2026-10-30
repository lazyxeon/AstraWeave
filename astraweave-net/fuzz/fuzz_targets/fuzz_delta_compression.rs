//! Fuzz target for delta compression/decompression.
//!
//! Tests that diff_snapshots and apply_delta are robust with arbitrary inputs.

#![no_main]

use std::collections::BTreeMap;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use astraweave_core::IVec2;
use astraweave_net::{apply_delta, diff_snapshots, EntityState, FullInterest, Snapshot};

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    base_tick: u64,
    target_tick: u64,
    entities_base: Vec<FuzzEntity>,
    entities_target: Vec<FuzzEntity>,
}

#[derive(Debug, Clone, Arbitrary)]
struct FuzzEntity {
    id: u32,
    x: i32,
    y: i32,
    hp: i32,
    team: u8,
    ammo: i32,
}

impl From<&FuzzEntity> for EntityState {
    fn from(e: &FuzzEntity) -> Self {
        EntityState {
            id: e.id,
            pos: IVec2 { x: e.x, y: e.y },
            hp: e.hp,
            team: e.team,
            ammo: e.ammo,
        }
    }
}

fn build_snapshot(tick: u64, entities: &[FuzzEntity]) -> Snapshot {
    Snapshot {
        version: 1,
        tick,
        t: tick as f32 / 60.0,
        seq: tick as u32,
        world_hash: 0,
        entities: entities.iter().map(EntityState::from).collect(),
    }
}

fuzz_target!(|input: FuzzInput| {
    let mut base = build_snapshot(input.base_tick, &input.entities_base);
    let target = build_snapshot(input.target_tick, &input.entities_target);
    let viewer = target
        .entities
        .first()
        .cloned()
        .or_else(|| base.entities.first().cloned())
        .unwrap_or(EntityState {
            id: 0,
            pos: IVec2 { x: 0, y: 0 },
            hp: 0,
            team: 0,
            ammo: 0,
        });

    // FullInterest keeps every entity. apply_delta writes into `base`.
    // Duplicate ids collapse to the last value, matching the diff map.
    let delta = diff_snapshots(&base, &target, &FullInterest, &viewer);
    apply_delta(&mut base, &delta);

    let mut expected: BTreeMap<u32, &EntityState> = BTreeMap::new();
    for entity in &target.entities {
        expected.insert(entity.id, entity);
    }
    let got: BTreeMap<u32, EntityState> = base.entities.iter().map(|e| (e.id, *e)).collect();
    assert_eq!(got.len(), expected.len());
    for (id, want) in expected {
        let got = got.get(&id).expect("entity missing after apply_delta");
        assert_eq!(got.pos, want.pos);
        assert_eq!(got.hp, want.hp);
        assert_eq!(got.team, want.team);
        assert_eq!(got.ammo, want.ammo);
    }
});
