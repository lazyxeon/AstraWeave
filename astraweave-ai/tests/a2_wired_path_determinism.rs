//! A2 (v1.0) CONTRACT TEST — wired-path determinism.
//!
//! Verifies the ratified A2 criterion (R-series M1.3; arbiter recon `992793c41`,
//! criteria sync `68eeb68d1`): **for an identical `WorldSnapshot`, the WIRED
//! production AI planners yield an identical `PlanIntent` across repeated runs.**
//!
//! ## Wired targets (the A2 criterion names exactly these)
//!
//! - `RuleOrchestrator::propose_plan` — the `ecs_ai_plugin::sys_ai_planning` production path (the orchestrator that the ECS AI-planning system constructs and calls every tick).
//! - `GoapOrchestrator::propose_plan` — the deterministic GOAP path used by the flagship `hello_companion` demo (`examples/hello_companion/src/main.rs`).
//!
//! ## Why this is a regression guard, not a nondeterminism discovery
//! The A2 recon (`992793c41`) established that both wired orchestrators are
//! deterministic *heuristics*: they read `enemies.first()`, a `BTreeMap` cooldown,
//! and integer positions, with **no HashMap iteration, no RNG, and no wall-clock**
//! (`snap.t` is an input). So determinism is guaranteed by construction, and this
//! test **pins that purity** — it fails the moment a future change introduces an
//! iteration-order- or RNG-dependent path into the wired planners. The A* GOAP
//! planner that *does* carry iteration-order risk (`core_loop::dispatch_goap` /
//! `GoapPlanner` / the dormant `AdvancedGOAP`) is a separate, **non-wired** surface
//! covered by `astraweave-ai/src/goap/tests.rs`; it is not on the v1.0 A2 path.
//!
//! The scenarios are non-trivial and branch-covering (multiple enemies; smoke
//! ready vs on-cooldown; enemy in-range vs out-of-range vs no-enemy), and each run
//! asserts the **full `PlanIntent`** (`plan_id` + every `ActionStep`) — not merely
//! a step count, which is the weaker form the pre-existing unit tests used.

use astraweave_ai::{GoapOrchestrator, Orchestrator, RuleOrchestrator};
use astraweave_core::{CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use std::collections::BTreeMap;

/// The cooldown key the orchestrators read (`orchestrator.rs` `COOLDOWN_THROW_SMOKE`,
/// which is crate-private — mirrored here as the literal it equals).
const COOLDOWN_THROW_SMOKE: &str = "throw:smoke";

/// A non-trivial, multi-enemy snapshot. `enemies.first()` selection, the cooldown
/// `BTreeMap` lookup, and integer-position arithmetic must all be order-stable.
fn rich_snapshot(smoke_cd: f32, first_enemy_x: i32) -> WorldSnapshot {
    WorldSnapshot {
        t: 4.2,
        player: PlayerState {
            hp: 73,
            pos: IVec2 { x: 0, y: 0 },
            stance: "crouch".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 7,
            cooldowns: BTreeMap::from([
                (COOLDOWN_THROW_SMOKE.to_string(), smoke_cd),
                ("dash".to_string(), 1.5),
            ]),
            morale: 0.6,
            pos: IVec2 { x: 2, y: 3 },
        },
        enemies: vec![
            EnemyState {
                id: 11,
                pos: IVec2 {
                    x: first_enemy_x,
                    y: 4,
                },
                hp: 50,
                cover: "low".into(),
                last_seen: 0.0,
            },
            EnemyState {
                id: 12,
                pos: IVec2 { x: 9, y: 1 },
                hp: 30,
                cover: "high".into(),
                last_seen: 0.0,
            },
            EnemyState {
                id: 13,
                pos: IVec2 { x: -4, y: -2 },
                hp: 80,
                cover: "none".into(),
                last_seen: 0.0,
            },
        ],
        pois: vec![],
        obstacles: vec![],
        objective: None,
    }
}

#[test]
fn a2_wired_path_determinism() {
    const N: usize = 8;

    // Branch-covering scenarios across the wired heuristics:
    //  - RuleOrchestrator branches on the smoke cooldown (ready -> Throw+MoveTo+CoverFire;
    //    on-cooldown -> MoveTo+CoverFire; no enemy -> empty).
    //  - GoapOrchestrator branches on Manhattan distance to the first enemy
    //    (<=2 -> CoverFire; otherwise MoveTo; no enemy -> empty).
    // `me` is at (2,3); a first enemy at x=3 -> (3,4) is distance 2 (in range),
    // at x=9 -> (9,4) is distance 8 (out of range).
    let scenarios = [
        rich_snapshot(0.0, 9), // smoke ready, first enemy far  -> Rule:Throw…  GOAP:MoveTo
        rich_snapshot(3.0, 9), // smoke on cooldown, enemy far   -> Rule:MoveTo… GOAP:MoveTo
        rich_snapshot(0.0, 3), // smoke ready, first enemy near  -> Rule:Throw…  GOAP:CoverFire
        rich_snapshot(2.5, 3), // smoke on cooldown, enemy near  -> Rule:MoveTo… GOAP:CoverFire
        {
            let mut s = rich_snapshot(0.0, 9);
            s.enemies.clear(); // no-enemy fallback branch (both -> empty plan)
            s
        },
    ];

    let rule = RuleOrchestrator;
    let goap = GoapOrchestrator;

    for (i, snap) in scenarios.iter().enumerate() {
        // The reference plan for this scenario.
        let rule_ref = rule.propose_plan(snap);
        let goap_ref = goap.propose_plan(snap);

        // Sanity: the snapshots are rich enough to produce real, multi-field plans
        // on the enemy scenarios (so the equality assertion is non-vacuous).
        if !snap.enemies.is_empty() {
            assert!(
                !rule_ref.steps.is_empty(),
                "scenario {i}: RuleOrchestrator should produce a non-empty plan when an enemy exists"
            );
            assert!(
                !goap_ref.steps.is_empty(),
                "scenario {i}: GoapOrchestrator should produce a non-empty plan when an enemy exists"
            );
        }

        // A2: identical snapshot => identical full PlanIntent, every run.
        for run in 0..N {
            assert_eq!(
                rule.propose_plan(snap),
                rule_ref,
                "A2 VIOLATION: RuleOrchestrator (wired ecs_ai_plugin path) is non-deterministic \
                 on scenario {i}, run {run}"
            );
            assert_eq!(
                goap.propose_plan(snap),
                goap_ref,
                "A2 VIOLATION: GoapOrchestrator (wired hello_companion path) is non-deterministic \
                 on scenario {i}, run {run}"
            );
        }
    }
}
