//! Deterministic replacement for the old arbiter benches.
//!
//! A fixed proposal is planned, then the tool sandbox accepts or rejects it.
//! No model is polled.

use astraweave_ai::htn::legacy_goap_plan;
use astraweave_ai::{validate_tool_action, ToolVerb, ValidationContext};
use astraweave_core::{CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;
use std::hint::black_box;

fn snapshot() -> WorldSnapshot {
    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: "standing".into(),
            orders: vec![],
        },
        me: CompanionState {
            pos: IVec2 { x: 0, y: 0 },
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
        },
        enemies: vec![EnemyState {
            id: 1,
            pos: IVec2 { x: 5, y: 0 },
            hp: 50,
            cover: "none".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        obstacles: vec![],
        objective: Some("Test objective".into()),
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn bench_propose_and_validate(c: &mut Criterion) {
    let snap = snapshot();
    let context = ValidationContext::new();
    c.bench_function("propose_then_validate", |b| {
        b.iter(|| {
            let plan = legacy_goap_plan(black_box(&snap));
            let accepted = validate_tool_action(
                1,
                ToolVerb::MoveTo,
                black_box(&snap),
                &context,
                Some(IVec2 { x: 1, y: 0 }),
            );
            black_box((plan, accepted.is_ok()))
        });
    });
}

criterion_group!(validated_proposal, bench_propose_and_validate);
criterion_main!(validated_proposal);
