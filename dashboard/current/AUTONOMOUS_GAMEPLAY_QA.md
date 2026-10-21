# Autonomous gameplay QA

> **Status**: scenario 1 is implemented. Later scenarios are still design.
> **Date**: 2026-10-15
> **Branch**: `cursor/gameplay-qa-slice-0073` (rebased onto current `main`, draft, not for merge)
> **Read against**: `main` after #212, #243, #277, and #290

## Recorded decisions

Approved 2026-10-15, with two answers applied as written:

1. Scenario 1 stays the one-tick slice resolve. `run_scenario` calls `run_aravath_slice` once per trace. The scripted policy is the RON edge list. The goal policy `slice_sword_then_weave` emits the same edges from code. The trace hash covers those edges plus the slice fields (`sword_hp`, `weave_hp`, `cover_intact`, `cover_damage`). The function is not split into a tick loop.
2. `run_aravath_slice` is on `main`. #243 has landed, and `destruction` stays a required replay case next to `gameplay_qa_aravath_slice`.

`SaveReload` and `SameSeqTwice` stay specified below and are not in `scenarios/aravath_slice_sword.ron`. Both need the world the one-tick function does not return.

The minimizer runs only when `QA_MINIMIZE=1` and `GITHUB_EVENT_NAME` is not `pull_request`. The Linux test and `determinism-replay.yml` do not set `QA_MINIMIZE`. The required case `gameplay_qa_aravath_slice` is the SHA-256 of the passing scripted trace, `ef7e38b580f2d157041d2d58bc0cac41dcf1442ac0ff6064d6ba52039adfc089`, emitted by `replay_aravath_slice_scenario`. Required cases are `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, and `gameplay_qa_aravath_slice`.

Synthetic players will attempt objectives in headless, fixed-tick runs and report softlocks, unreachable objectives, save/reload breakage, and simple exploits. Every failure writes one artifact that replays to the same hash trace on Linux, Windows, and macOS. Scenario 1 is the runner. Later scenarios are still design.

Learning-based exploit search is out of scope. The first bots are scripted and goal-driven.

## What already exists

Three simulators are easy to confuse. The first scenario uses only the first.

| Simulator | Where it lives | What a bot can drive today |
|---|---|---|
| Aravath slice | `run_aravath_slice` in `astraweave-weaving/src/aravath.rs` on PR #212 (`5bc00302`) and, identically, on PR #275 | One call. Seed `0xA11C_E5EE`. Sword hit, weave school, cover bond. No per-tick input |
| Greybox | `aravath_runtime` on PR #205 (`5048c630`), combat call site on PR #275 | `Greybox::step(TickInput)` at 60 Hz. Hardcoded `recorded_script()`. End-state golden hash |
| Core world | `astraweave-core` on `main` | `World::tick` plus `replay_hash` cases. No player-objective loop |

The legacy slice runtime (workspace crate whose directory still uses the retired product name, `src/game_loop.rs`) is a fourth loop. It walks dialogue, cinematics, and a storm choice through `HashMap`s. It is not the Aravath slice, and the QA runner does not extend it.

### Aravath slice and the golden replay

`run_aravath_slice` builds an `astraweave-ecs` app, installs one RON combat table, and runs a single `combat_resolve_system` plus one destruction tick. The checked-in test expects sword hp 15 (25 from 40), weave hp 25 (rating 15 from 40), a broken cover bond, and cover damage at least 25. World seed and destruction seed are both `0xA11C_E5EE`. The example `astraweave-weaving/examples/aravath_slice.rs` prints that struct and asserts the same numbers. Hit points are `Fixed64` raw values divided by `Fixed64::SCALE`. This path does not call Rapier, winit, or the GPU.

PR #205 is the playable greybox (`cursor/aravath-greybox-b3de`, still open, base `main`). Its replay test is `replay_matches_golden_hash` in `aravath_runtime/src/sim.rs`. `play(&recorded_script())` spawns two enemies and steps a Rust `Vec<TickInput>`: 40 empty ticks, 50 ticks of move/sprint/jump/roll/attack/block/dig/lock-on/look, one place, 30 walk ticks, one fly toggle, 160 fly ticks. The end hash is `Greybox::state_hash()`, formatted as 16 hex digits, compared to `aravath_runtime/golden/replay.hash`:

```text
9b607d1a840ecd64
```

That hash is FNV-1a 64 (`Canon` in the same file: offset `0xcbf29ce484222325`, prime `0x100000001b3`) over little-endian `f32::to_bits` and integer fields. It is one digest of the final state. It is not a per-tick trace. PR #275 leaves the golden file unchanged.

`.github/workflows/aravath-determinism.yml` on #205 runs `cargo test -p aravath_runtime --lib replay_matches_golden_hash` on Ubuntu and Windows only. `RUSTFLAGS` is `-C target-cpu=x86-64 -C target-feature=-fma`. Timeout is 90 minutes. Checkout uses Git LFS because the greybox assets do. macOS is not in the matrix. `aravath_runtime` enables glam's `libm` feature; Rapier still integrates the character with `f32`. That workflow is evidence for x86_64 Linux and Windows. It is not the three-OS proof this QA artifact needs.

Other greybox tests already nearby, and worth reusing as oracles rather than rewriting:

- `replay_matches_across_independent_runs` saves and loads a checkpoint, then compares `state_hash` of two restores.
- `frame_schedule_does_not_change_fixed_ticks` shows 60 Hz, 30 Hz, and 120 Hz frame delivery spend the same ticks when the latch holds edges.
- `an_enemy_death_saves_and_loads_the_same_scar_hash` round-trips the scar field.
- Combat tests on the greybox include "a corpse must not inject twice".

### Combat wiring (PR #275)

#275 (`cursor/aravath-combat-wire-9a34`, base #205, one commit) calls `astraweave_weaving::run_aravath_slice` from `Combat::tick` when a sword swing is inside its active window and has not yet landed. It applies `40.0 - slice.sword_hp as f32` (25) to that enemy's `f32` health and sets `Combat.cover_broken` when the slice bond breaks. The slice's `DestructionWorld`, weave hp, and cover damage are dropped on the floor. The greybox world and the slice world are not the same entities.

`state_hash` on the greybox branch hashed `player_hp`, attack flags, and per-enemy health. `cover_broken` is new on #275. A later greybox scenario has to hash that flag or a cover break will not change the golden.

### `replay_hash` and `determinism-replay`

On `main`, `.github/workflows/determinism-replay.yml` is a required check with no path filter. Ubuntu, Windows, and macOS each run `cargo test -p astraweave-core --test replay_hash`. x86_64 pins `target-cpu=x86-64`. aarch64 stays on the default baseline (`target-cpu=native` is forbidden). `cargo test` is not retried. The `compare` job requires `planner_pure`, `perception_world`, `gameplay_rng`, and `fixed64` to be present and byte-identical. Artifacts are `target/replay-hash.json` rows of `{case, sha256}`.

The encoder is `astraweave-core/tests/replay_support/mod.rs`. Bytes are little-endian integers, length-prefixed UTF-8, and `f32::to_bits`. It does not sort: production code must already emit stable order. SHA-256 is the digest. The file comment says to promote the encoder to a library function when a second crate needs it. Gameplay QA is that second caller.

`SimRng` (`astraweave-core/src/sim.rs`) mixes `(world_seed, tick, system_id)` with an integer function and draws from `StdRng`. `unit_f32` is a dyadic rational, not libm. Bots that roll use this. They do not call `thread_rng`, `Uuid::new_v4`, or the wall clock.

PR #212 appends `combat_resolve`. Its combat test writes `target/replay-hash-combat.json` (SHA-256 of little-endian Q32.32 raw bits, entity ids sorted) and a node step merges it into `replay-hash.json` before upload. Required set becomes `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`.

PR #243 appends `destruction` and, on its current head `444b485c`, replaces the required set with `planner_pure`, `perception_world`, `fixed64`, `destruction`. That drops `gameplay_rng` and `combat_resolve`, sets `cancel-in-progress: true` for every event (including pushes to `main`), and swaps the shared `rust-ci-setup` action for a bare toolchain plus `Swatinem/rust-cache`. Destruction sources otherwise match #212. #212's `astraweave-destruction/Cargo.toml` says the crate there is a temporary copy of #243, to be dropped from #212 when #243 lands.

### Input recording

There is no input recorder.

`astraweave-input` stores bindings (`save_bindings` / `load_bindings`) and an `Action` enum (`MoveForward`, `AttackLight`, `Interact`, `UiAccept`, and the rest). It does not store a stream. The editor binding panel's recording test captures rebinding, not sim ticks.

The greybox boundary is `TickInput` in `aravath_runtime/src/sim.rs`: `MotorInput`, attack, block, lock-on, dig, place, place material, look deltas, respawn. `PlayInput` turns winit events into one `TickInput`. `FrameLatch` ORs edges that arrived on a frame which spent zero ticks, and `catch_up` spends wall time as 60 Hz steps (`FIXED_DT = 1/60`, `MAX_FRAME_DT = 0.25`, `MAX_SUBSTEPS = 8`), applying edges on the first step only. The QA runner calls `step` once per recorded record. It does not call `catch_up`, so a stall cannot drop or repeat inputs.

`recorded_script()` is the only "recording", and it is Rust source. The scenario format replaces it for new runs. The golden script stays where it is until someone moves it on purpose.

### Save and persistence

`persistence/aw-save` writes versioned `.awsv` files (`SaveBundleV2`, schema 2, LZ4, CRC32, atomic rename). `SaveManager::save` puts `Uuid` and `OffsetDateTime::now_utc()` into the file name and the bundle. Those bytes are not a replay identity. `WorldState` carries `tick`, an opaque `ecs_blob`, and `state_hash`.

The greybox checkpoint (`aravath_runtime/src/save.rs`) stores voxel edits, player position, yaw, stamina, hp, and the scar field in bundle `meta`. Floats are eight hex digits of `f32::to_bits`, with a decimal fallback for older files. `WorldState.tick` is hardcoded `0`. `state_hash` is `edits.len()`. The save id is `Uuid::new_v4()`. A load compares gameplay fields and scar hashes. It does not compare file bytes. Enemies, motor mode, attack phase, lock target, `cover_broken`, dialogue, and quests are absent. A combat save/reload check against this checkpoint will fail open: the fight is gone after load, and the current tests do not notice.

`astraweave-combat/src/save.rs` on #212 and #275 is the combat blob that can round-trip: `combat_tables` (table hash; mismatch refuses the blob) and `combat_vitals` (entity id, hp, hp max, poise, poise max, thread-strain, sorted by entity, little-endian, hex). It is not called from the greybox checkpoint.

`astraweave-persistence-ecs` registers `auto_save_system` (empty) and `replay_system` (advances `current_tick` and does not apply events). `start_replay` returns an empty event list. `save_game` also uses `Uuid::new_v4()` and `OffsetDateTime::now_utc()`, and fills inventory with placeholder credits. QA does not build on this plugin.

`DestructionWorld::state_hash` returns `[u8; 32]`, and the destruction crate already checks that a restored world matches. That is the cover-bond half of a slice save.

### Quests

`docs/architecture/quests.md` records two quest implementations.

- `astraweave-weaving::quest::QuestManager` is the gameplay one: `ObjectiveType` (kill, repair, fetch, explore, escort, defend, time trial, boss, collect), prerequisites, one active quest. Storage is a `HashMap` keyed by id, so a bot addresses quests by id and does not walk the map. Starter quests are `quest_stabilize_anchors`, `quest_clear_corruption` (requires the first), and `quest_restore_beacon` (requires the second), in `astraweave-weaving/src/starter_quests.rs`. They are not registered on the slice or the greybox.
- `astraweave-quests` is the LLM and terrain layer. The trace marks it in-design aside from the basic `Quest` / `QuestStep` pair the editor uses. `CActiveQuest`, journal entries, and generators call `chrono::Utc::now()`. `systems.rs` also times work with `Instant::now()`. Those clocks cannot enter a hashed sim.

A third quest type would violate the one-implementation rule. Scenario quests use `QuestManager`.

### Dialogue

`astraweave-dialogue::DialogueGraph::validate` checks that every `next_id` exists. It does not check that a node is reachable from the start, that a terminal node exists, or that choice order is stable. `DialogueRunner` is the sim walker: `start`, `choose(index)`, states `Idle` / `WaitingForChoice` / `Finished`. The editor has a second `DialogueGraph` in `tools/aw_editor/src/panels/dialogue_editor_panel.rs`. Bots use the crate runner, not the panel type.

No save blob stores the runner's current node or history. A dialogue save/reload check has to add that blob first. That addition is later work, after this design is accepted.

### CI tiers (PR #223, merged)

`docs/current/CI_ARCHITECTURE.md` is the contract. Pull requests require a short set: `PR Rust checks`, `determinism-replay / compare`, the three `hash (*)` jobs, cargo-deny, cargo-audit, pack guard, scorecard. `cancel-in-progress` is on for `pull_request` only. PBR, editor, and full integration tests do not run on pull requests. Docs-only diffs skip the Rust compile. Determinism still runs.

Profile `ci-pr` excludes the tests still listed in `scripts/ci/deferred_pr_tests.txt`. #333 removed the terrain goldens and the AI planning rows. The file still names GPU render passes, lavapipe fluid smokes, and `test_qwen3_variants`. #254 stays open. Profile `ci` on `main` and nightly does not exclude that file. Replay tests match `test(/determinism/) | test(/replay/) | test(/deterministic/)` and get `retries = 0`. The determinism workflow does not call nextest.

Measured on #223 head `7512c86a`: execution-only chain to `PR Rust checks` is 49.3 minutes, almost all of it the Windows archive build (36.3 minutes) plus three short partitions. Replay compare execution is 10.8 minutes, Windows hash 10.7 minutes. Queue on that chain was 2.5 minutes. A new required job, or a greybox LFS run on every pull request, spends the budget #223 just recovered.

### Harnesses that stay where they are

| Harness | Role for QA |
|---|---|
| `astraweave-core/tests/replay_hash.rs` | Three-OS digest compare. Future home of one cheap slice case |
| `astraweave-combat/tests/replay_hash.rs` (#212) | Pattern for a second JSON file merged before compare |
| `aravath_runtime` golden test (#205) | Greybox end hash. Stays Linux + Windows, LFS, not the QA gate |
| `tools/aw_editor/tests/play_mode.rs` `test_deterministic_replay` | Editor runtime, 100 ticks, in-process hash equality. No input stream |
| `examples/net_headless_sim` | Net snapshot replay. Different event type |
| `integration-tests.yml` | Push and 04:00 UTC schedule. Not on pull requests. Nightly catalog belongs here or beside it |
| Weaving and physics `determinism_tests` | In-process repeats. Not cross-OS artifacts |

## Bot API

One trait, two adapters later. The first implementation is the slice adapter. The greybox adapter is specified so it can be added without a second action language.

```rust
/// One fixed tick of intent. Edges are level for this tick, not "OR'd from a frame".
pub struct TickActions {
    pub edges: Vec<SimAction>,
}

pub enum SimAction {
    /// Slice and combat. `seq` is the hit identity for the tick.
    Hit { attack_id: u32, defender: DefenderId, seq: u32, heal: bool },
    /// The weave-decay command the slice already enqueues after resolve.
    WeaveDecay,
    /// Greybox adapter only. Ignored by the slice adapter, which rejects the scenario at load.
    Motor(MotorAction),
    Interact,
    Choice { index: u32 },
    Save { slot: u8 },
    Load { slot: u8 },
}

pub trait SyntheticPlayer {
    fn decide(&mut self, obs: &Observation) -> TickActions;
}

pub enum Bot {
    Script { steps: Vec<TickActions> },
    Goal { objective: ObjectiveId, policy: GoalPolicy },
}
```

`Script` returns `steps[tick]` and then empty actions. `Goal` for the slice is a fixed policy, not a search: while the cover bond is intact, emit the sword hit (`attack_id` 1, seq 0); while weave hp is above the expected remainder, emit the weave bolt (`attack_id` 3, seq 1); then emit `WeaveDecay` once. Ties break by `seq`. The policy reads `Observation` fields that `run_aravath_slice` already computes. It does not call `RuleOrchestrator` or `GoapOrchestrator`: those plan a `WorldSnapshot`, and the slice is an `astraweave-ecs` world. Pointing them at the slice would be a second planner.

`Observation` for the slice is tick, the `AravathSlice` fields, and the destruction hash. It includes no wall time, no adapter name, and no save id.

The runner:

1. Scenario 1 does not rebuild the world. It calls `run_aravath_slice` once for the scripted trace and once for the goal trace. The edges are recorded in the trace. They are not applied as a second resolver.
2. The trace hash is the canonical bytes of those edges plus the slice fields.
3. On failure, the runner builds the JSON artifact. ddmin runs only when `minimize_enabled` is true.

`Greybox` adapter, when it exists, maps `Motor` onto `TickInput` and calls `Greybox::step`. `Hit` is not remapped onto the greybox sword: #275 already does that inside `Combat::tick` by calling the whole slice. A greybox scenario that also emits `Hit` would resolve the slice twice. Greybox scenarios use `Motor` only.

RNG, if a later scenario needs it, is `SimRng::new(seed, tick, system_id)` with a new `SIM_SYSTEM_*` constant next to the ones in `astraweave-core/src/sim.rs`. The slice's `CombatClock.world_seed` stays `0xA11C_E5EE` for the first scenario.

## Scenario format

RON, same family as the combat `TableFile`. One file, one scenario. No Rust in the scenario. The loader rejects unknown fields.

```ron
Scenario(
    schema: 1,
    id: "aravath_slice_sword",
    seed: 0xA11C_E5EE,
    max_ticks: 1,
    setup: Slice,
    bot: Script(steps: [
        (edges: [
            Hit(attack_id: 1, defender: Cover, seq: 0, heal: false),
            Hit(attack_id: 3, defender: Woven, seq: 1, heal: false),
            WeaveDecay,
        ]),
    ]),
    goal: (objective: "cover_broken", policy: "slice_sword_then_weave"),
    checks: [
        Reach(by_tick: 1, sword_hp: 15, weave_hp: 25, cover_intact: false, min_cover_damage: 25),
    ],
)
```

`setup: Slice` is the only setup the loader accepts. The checked-in file is `astraweave-weaving/scenarios/aravath_slice_sword.ron`. The goal block is required. Its policy emits `slice_program()` in code, and the script above must hash to the same trace.

Checks, and the failure condition each one writes:

| Check | Failure condition | When it is honest |
|---|---|---|
| `Reach` | `TerminalMismatch` | Slice, now |
| `NoSoftlock { window }` | `Softlock` | A real objective that can sit still. Window is in ticks. 600 ticks is 10 s at 60 Hz. Progress tag, hp, and position cell all unchanged, bot still has a legal action, objective not complete and not failed |
| `Reach` missed at `max_ticks` by a goal bot | `Unreached` | "This policy did not get there." Not a proof the objective is unreachable |
| Static graph | `Unreachable` | Dialogue `next_id` missing (`DialogueGraph::validate`), plus a start-node walk for nodes that validate still leaves orphaned. Quest prerequisite missing or cyclic. Objective type with no producer in the scenario's action set |
| `SaveReload` | `SaveReloadMismatch` | Hash of the interrupted-and-restored run differs from the twin that never saved, at the first diverging tick |
| `SameSeqTwice` and later named predicates | `Exploit` | The predicate is declared in the scenario. A learning search is not a predicate |

`Unreached` and `Unreachable` stay different words. A goal bot's timeout must not be labeled unreachable.

Softlock, dialogue, and quest checks are in the schema now. The slice has no locomotion and no quest, so the first scenario does not pretend to exercise them. Turning them on is a later scenario that registers `QuestManager` or a `DialogueRunner` in the same tick loop.

## Failure artifact and minimizer

One JSON object, UTF-8, trailing newline. The replay identity is the canonical byte encoding of `seed`, `setup`, `inputs`, and `hashes`, using the `replay_support` rules (little-endian, length-prefixed strings, no map walk). Host, timestamp, save id, and `created_at` are absent. SHA-256 of those bytes is what the three OS compare.

```json
{
  "schema": 1,
  "scenario_id": "aravath_slice_sword",
  "seed": "0xA11CE5EE",
  "setup": "slice",
  "inputs": [
    { "tick": 0, "edges": [
      { "hit": { "attack_id": 1, "defender": "cover", "seq": 0, "heal": false } },
      { "hit": { "attack_id": 3, "defender": "woven", "seq": 1, "heal": false } },
      { "weave_decay": {} }
    ] }
  ],
  "hashes": [
    { "tick": 0, "sha256": "<64 hex of the tick's canonical state bytes>" }
  ],
  "failure": {
    "kind": "terminal_mismatch",
    "tick": 0,
    "detail": "sword_hp 15 expected, got 40"
  },
  "original_len": 1,
  "minimized_len": 1
}
```

Per-tick state bytes for the slice are the combat replay's layout: sorted entity ids, `Fixed64` raw little-endian, then the 32-byte destruction hash. That is integer data, so Linux, Windows, and macOS can match without an FMA pin. The greybox FNV `Canon` digest may be stored in a later artifact as 16 hex digits. It is not a required three-OS case until a macOS run matches Ubuntu and Windows on the same pin. Putting `f32` physics bits into the required compare would fail macOS for a reason the greybox workflow has never measured.

Replay procedure, the only one the artifact supports:

1. Parse the JSON. Reject a second setup, a hole in tick numbers, or a hash row whose tick is missing from `inputs`.
2. Build `setup` at `seed`.
3. Apply `inputs[t]` and step once. Hash. Compare to `hashes[t]`.
4. Stop at the first mismatch. The message includes the tick, the expected hash, and the actual hash.
5. If the hashes match through the end, re-evaluate `failure` against the live world. A reproduced failure exits non-zero with the condition. A clean run of an artifact that claimed a failure exits non-zero with `artifact_did_not_reproduce`.

The minimizer runs only after a predicate fails, and only off the pull-request path. It is ddmin on the input list:

- Delete a chunk of ticks, replay from tick 0, keep the deletion when the failure `kind` is the same. The tick index inside the condition is allowed to move. The kind is not.
- When no chunk deletes, try clearing individual edges inside the remaining ticks.
- Cap at 64 replays. If the cap hits, keep the shortest candidate and set `minimized: "partial"`.
- Replay the winner twice in a fresh process. Both hash traces must equal the trace written into the artifact. If they differ, discard the minimization, keep the original stream, and set the condition to `UnstablePredicate`. An unstable predicate is itself a failed run.

The checked-in artifact is the minimized one. `original_len` records the pre-minimizer tick count.

## CI without growing wall time

No new required workflow. No LFS checkout. No renderer. No minimizer on pull requests. No entry in `scripts/ci/deferred_pr_tests.txt`.

After the slice is on the branch under test:

1. **Pull request, Linux, inside the weaving lib tests.** `replay_aravath_slice_scenario` loads the RON, checks Reach, and requires the scripted and goal traces to match. The name contains `replay`, so profile `ci-pr` does not retry it. This is the fast signal. It does not by itself prove macOS.
2. **Three OS, inside `determinism-replay.yml`, one extra case.** Name: `gameplay_qa_aravath_slice`. Value: SHA-256 of the canonical hash trace of the successful scripted run. Emit it the way `combat_resolve` is emitted: `target/replay-hash-gameplay-qa.json` is merged into `replay-hash.json` before upload. Duplicate case names are an error. The required set also keeps `destruction`.
3. **Nightly / `main`, not `pull_request`.** The scenario directory, the goal-bot twin, save/reload twins, and the minimizer when a predicate fails. Hang it on the same schedule as `integration-tests.yml` (push and 04:00 UTC), timeout 30 minutes, `retries = 0`. Three OS compare of those traces. This is where softlock windows of 600 ticks and quest scripts are allowed to live.

`replay_aravath_slice_scenario` writes `target/replay-hash-gameplay-qa.json`. The merge step appends that file next to the combat file. Core's replay test still emits `destruction`.

The greybox workflow on #205 stays as it is. QA does not call `aravath_greybox`, does not fetch KayKit blobs, and does not take its 90 minute budget.

## First scenario

Shipped file: `astraweave-weaving/scenarios/aravath_slice_sword.ron`. Seed `0xA11C_E5EE` (`ARAVATH_SLICE_SEED`).

- One scripted tick: sword hit id 1, weave hit id 3, then weave decay. `max_ticks` is 1.
- `Reach`: sword hp 15, weave hp 25, cover not intact, cover damage at least 25.
- The goal policy emits that same edge list. A difference writes `goal_bot_diverged`.
- `examples/aravath_slice.rs` calls `load_bundled_scenario` and `run_scenario`.

`SaveReload` and `SameSeqTwice` are not in this file. What this scenario does not claim: a softlock search, a quest, a dialogue, a greybox fight, or a save round-trip. Those need a world the one-tick function does not return.

## Later scenarios (specified, not built)

| Id | Waits on | Checks |
|---|---|---|
| `greybox_recorded` | #205 golden, and a macOS measurement before it is required | The existing script moved into RON, per-tick `Canon` hashes, still not a required three-OS case until macOS matches |
| `greybox_combat_save` | #275 wiring, and a checkpoint that stores enemies, attack phase, motor mode, tick, and `cover_broken` | `SaveReload` during a swing. Today's checkpoint will show `SaveReloadMismatch` because enemies are dropped. Fix the checkpoint in that implementation, or file `deferred-failure` with this plan if the fix is refused |
| `starter_quest_chain` | `QuestManager` driven by slice or greybox events, progress stored as tick counts | Prerequisite order of the three starter quests. `Unreachable` if a prerequisite id is missing. No `Utc::now` |
| `dialogue_choice` | `DialogueRunner` plus a save blob of current node id and history, `BTreeMap` or a `Vec` keyed by id | `validate` plus the reachability walk. `SaveReload` on a `WaitingForChoice` node. Choice index is the input edge |

Player-facing strings in those scenarios say Aravath, never the retired product name.

## Merge order

This pull request is the design. It does not merge as part of the work that wrote it.

1. **#223 is already on `main`.** QA fits its tiers. Do not add a required job, and do not cancel `main` pushes.
2. **#243 and #212 are on `main`.** `destruction` and `combat_resolve` stay required. This rebase keeps both and adds `gameplay_qa_aravath_slice`.
3. **#290 is on `main`.** It does not own the slice replay case.
4. **Scenario 1 is implemented on top of that main-line slice**, in a new branch, still not merged from this design. It adds `gameplay_qa_aravath_slice` to the required set in the same commit as the test that emits it.
5. **#205 stays the greybox.** #275 should shrink to the `Combat::tick` call site once #212 and #243 own the crates on `main`. Until then, #275 vendors those crates onto #205 and will conflict with both. Do not start the QA runner on #275 and on #212 at once.
6. **#244, #263, and #264** sit on #205 and change traversal, bridge collapse, and the local field. Greybox goal bots wait until that stack is quiet. A hash recorded against a moving motor is not a regression test.
7. **#217** owns the deferred-test list. QA tests stay on profile `ci` and `ci-pr` with no exclusion line.
8. **#252 and #251** do not own `run_aravath_slice`. The goal policy does not move onto the weaving-framework branch.

Learning-based hunters are not ordered behind any of these.

## Pre-existing gaps

These block a claim that "gameplay replay" is already cross-OS. They are not fixed in this pull request. The GitHub token available here cannot open issues (`gh` is read-only, and label edits on this repo have returned HTTP 403). Each row is the issue text: label `deferred-failure`, owner, plan.

| Gap | Owner | Plan |
|---|---|---|
| Greybox golden is Ubuntu + Windows x86_64 with FMA off. macOS is unmeasured. Physics state is `f32` | PR #205 | Keep `9b607d1a840ecd64` as the greybox end hash. Do not add it to `determinism-replay` until a macOS job prints the same digest. Slice QA uses `Fixed64` so it can be required immediately |
| `.awsv` names and bundles include `Uuid::new_v4()` and `OffsetDateTime::now_utc()`. Greybox `WorldState.tick` is 0 and `state_hash` is the edit count | `persistence/aw-save`, greybox `save.rs` on #205 | QA hashes restored gameplay state, never file bytes. A later save API takes the id, the timestamp, and the tick from the sim |
| Greybox checkpoint omits enemies, motor mode, attack phase, lock, tick, and `cover_broken` | PR #275 / #205 | `greybox_combat_save` is expected to fail until the checkpoint stores those fields. Implement the fields with the scenario, or accept the deferred issue and skip the check |
| `auto_save_system` is empty and `replay_system` does not apply events | `astraweave-persistence-ecs` | QA does not call `PersistencePlugin` |
| `astraweave-quests` stamps `Utc::now()` and `Instant::now()` | quests trace, crate `astraweave-quests` | Quest scenarios use `astraweave-weaving::QuestManager` and tick counts. The LLM quest layer stays out of the hashed sim until its clocks are tick fields |
| The audio rapid-volume test shared `tests/assets` with cleanup | this branch, same fix as #289 | The beep is written in a private temp directory |
| Two dialogue graphs (crate runner and editor panel) and two quest crates | quests trace | Bots bind the crate types named above. No third graph and no third quest type |

## Non-goals

- Splitting `run_aravath_slice` into a tick loop. Scenario 1 calls it once.
- A learned or fuzzing exploit hunter.
- Driving the legacy slice game loop, the editor play mode, or the net headless example.
- Replacing `recorded_script` or moving the golden hash.
- GPU, wall-clock frames, or `catch_up` inside the QA runner.
- New assets. The slice scenario uses no meshes. Anything added later is CC0, with a manifest row, same rule as `assets/aravath`.
