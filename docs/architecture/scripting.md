---
schema_version: 1
trace_id: scripting
title: "Scripting and mod API"
description: "Sandboxed Rhai gameplay scripts, the mod loader, and tick-boundary reload"
primary_crate: astraweave-scripting
domain: gameplay
lifecycle_status: active
integration_status: wired
summary: "Gameplay and mod scripts run on the security sandbox. script_system does not inject PhysicsProxy or NavMeshProxy. The scripting examples no longer call them."
owns: [astraweave-scripting, astraweave-author]
doc_version: "1.7"
last_verified_commit: 49d289eb
runtime_edges:
  - to: astraweave-security
    note: "ScriptEngineResource and mod_script_system call sandbox::SandboxEngine / execute_deterministic"
  - to: astraweave-core
    note: "register_mod_runtime records ModScriptPlugin; build_app installs it before the legacy sim systems"
  - to: astraweave-weaving
    note: "queue_weave copies a template id and origin into WeaveScratch; the tick does not evaluate the field"
---

# Architecture Trace: Scripting and mod API

## Metadata

| Field | Value |
|---|---|
| **System name** | Scripting and mod API |
| **Primary crates** | `astraweave-scripting`, `astraweave-author` |
| **Document version** | 1.7 |
| **Last verified against commit** | `49d289eb` |
| **Last verified date** | 2026-10-21 |
| **Status** | Active for the sandboxed Rhai path. `register_mod_runtime` plus `build_app` is how the editor, `veilweaver_slice_runtime`, and `hello_companion` run mods. `script_system` does not inject physics or nav proxies (#318). The editor does not feed `CScript` entities. |
| **Owner notes** | WS-MOD. One sandbox implementation lives in `astraweave-security::sandbox`. This trace is the caller side. |

---

## 1. Executive Summary

**What this system does:**
Runs Rhai scripts that emit entity commands, and loads versioned mod manifests whose scripts move integer agent state on the simulation tick.

**Why it exists:**
Mods need a single sandboxed command set whose results are sim inputs. A second script engine is not allowed.

**Where it primarily lives:**
- `astraweave-security/src/sandbox.rs` — engine limits, allowlist, manifest parser, path reads, reload queue
- `astraweave-scripting/src/lib.rs` — `CScript` / `script_system`
- `astraweave-scripting/src/sim_mod.rs` — deterministic mod tick and the replay-hash case
- `astraweave-author/src/lib.rs` — `configure(meta)` author scripts
- `tools/aw_editor/src/rhai_reload.rs` — editor hot reload

**Status note:**
`script_system` no longer stats files during the tick. Reloads come from `ScriptReloadQueue::take_boundary` at the start of the system. `mod_script_system` uses a separate `ModReloadQueue` so the two systems do not steal each other's bytes. Neither tick places `PhysicsProxy` or `NavMeshProxy` in the script scope. The host still reads `PhysicsWorld` for collision events and for bodies spawned by script commands. A gameplay script that names `physics` or `nav` errors. `script_system_rejects_physics_and_nav_proxies` checks that: the script runs (`saw` becomes 1) and the assignment after the proxy call does not (`result` stays 0), even when both resources are in the world.

---

## 2. Authoritative Pipeline

```text
[mod.toml + scripts/*.rhai under a root]
    │
    │ parse_manifest_str / read_rhai_under_root     sandbox.rs
    ▼
[ModManifest + script bytes, load order by mod id]
    │
    │ SandboxEngine::build(SandboxProfile)          no time package, no import, finite ops
    ▼
[ScriptSandbox.allowed_functions]
    │
    ├─ author: call_deterministic(..., "configure")  astraweave-author
    │     validate_budget against EngineCeilings
    │
    └─ sim: mod_script_system                         sim_mod.rs
          entities sorted by id
          sim_roll from astraweave_core::sim::SimRng (mix_sim_seed stream, no private StdRng)
          move_to rejected outside ±100_000
          digest via mod_state_digest (SHA-256, little-endian)

[editor notify thread]
    │ classify_reload_path -> ReloadEvent::Script
    ▼
[RhaiHotReload::stage_file]          does not touch loaded sources
    │
    │ apply_at_tick_boundary           main.rs, immediately before runtime.tick
    ▼
[loaded BTreeMap]
```

---

## 3. Semantic Vocabulary

| Term | Meaning |
|---|---|
| `MOD_API_VERSION` | `1`. Manifest `api` must match. |
| `MOD_COMMAND_API_VERSION` | `2`. Also re-exported as `SCRIPT_COMMAND_API_VERSION`. |
| `COMMANDS` | The only host command names a profile may grant. |
| `Capability` | A group of those commands (`configure`, `move_entity`, `sim_rng`, …). |
| `EngineCeilings` | Caps for operations, strings, arrays, maps, script bytes, and director budget fields. |
| Tick boundary | `take_boundary` / `apply_at_tick_boundary`. Staging is not a sim write. |

---

## 4. Integration Touchpoints

| Caller | Entry | Role |
|---|---|---|
| `examples/rhai_authoring` | `run_author_script` | Example. Path must stay under the working directory. |
| `ScriptingPlugin` | `script_system` | Gameplay `CScript` tick. Engine is the gameplay sandbox plus `register_api`. |
| `ModScriptPlugin` | `mod_script_system` | Integer agent sim. Replay case `scripted_system`. Installed by `build_app` after `register_mod_runtime`. |
| `aw_editor` `main.rs` | `RhaiHotReload` | Stages `.rhai` and applies before `runtime.tick`. |
| `tools/aw_plugin_template` | README | Non-runtime scaffold. Not a loadable mod. |

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-security/src/sandbox.rs` | Sandbox, manifests, queue | Active | See `docs/architecture/security.md`. |
| `astraweave-scripting/src/lib.rs` | `CScript`, `script_system` | Active | Entity iteration is sorted. `script_state` is a `BTreeMap`. No `SystemTime` on the tick. Physics and nav proxies are not injected. |
| `astraweave-scripting/src/api.rs` | `ScriptCommand` set | Active | Re-exports `SCRIPT_COMMAND_API_VERSION`. |
| `astraweave-scripting/src/loader.rs` | Async relative `.rhai` load | Active | Rejects absolute paths and `..`. Size-capped. |
| `astraweave-scripting/src/sim_mod.rs` | Mod tick + digest | Active | `MOD_SYSTEM_ID = 0x4D4F4453`. `register_mod_runtime` is the engine-app installer. The success tick reuses entity, source, error, and Rhai scope buffers. Weave casts go into a fixed array. |
| `astraweave-scripting/tests/script_replay_hash.rs` | `scripted_system` SHA-256 | Active | Appends the case to `target/replay-hash.json`. |
| `astraweave-author/src/lib.rs` | `configure` | Active | Floats in hints are rejected. |
| `tools/aw_editor/src/rhai_reload.rs` | Editor boundary | Active | |
| `tools/aw_plugin_template/` | Scaffold | Non-runtime | README says so. |
| `examples/scripting_advanced_demo/` | `ScriptingPlugin` demo | Example | Host still inserts `PhysicsWorld` and `NavMesh`. `assets/advanced_ai.rhai` does not call them. Sight is `dist_to_player < 20.0`. A finished patrol point sets `path = []`. |
| `examples/scripting_playground/` | `ScriptingPlugin` demo | Example | Host still owns `PhysicsWorld`, dynamic boxes, and a sensor. The player script handles `on_trigger_enter` and `commands.apply_damage`. It does not raycast. |

---

## 6. Conflict Map / Residue

- `script_system` and `mod_script_system` are two callers of one sandbox, not two engines. They use different reload queues (`ScriptReloadQueue` vs `ModReloadQueue`).
- `execute_script_sandboxed` in `astraweave-security` is the async helper. Sim code does not call it.
- `register_api` still registers `PhysicsProxy` and `NavMeshProxy` types. `script_system` does not place either value in scope, so a gameplay script cannot call them. `physics` and `nav` stay host keys and are not written back into `script_state`.
- The editor publishes reloaded sources into `RhaiHotReload.loaded`. It does not insert `CScript` entities. Play mode does not execute those sources until a later wiring step.

---

## 7. Decision Log

### Decision: One sandbox in `astraweave-security::sandbox`
- **Date:** 2026-10-14
- **Status:** Accepted
- **Context:** `Engine::new()` installs `FileModuleResolver` and the standard package includes `timestamp` and `sleep`.
- **Decision:** Build from `Engine::new_raw`, register only the packages listed in security invariant 10, and put the allowlist on `ScriptSandbox`.
- **Alternatives considered:** Forking a third engine in `astraweave-author`. Rejected.
- **Consequences:** Author, gameplay, and mod ticks share limits. The async helper is unchanged.

### Decision: Operation budget, not a wall clock, stops scripts
- **Date:** 2026-10-14
- **Status:** Accepted
- **Context:** `tokio::time::timeout` depends on scheduling and is not a sim input.
- **Decision:** `set_max_operations` (default 50_000) and `OptimizationLevel::None`. `timeout_ms` on the production sandbox is 0 and is not read by `execute_deterministic`.
- **Consequences:** The same source and seed do the same work on every OS. A loop errors instead of sleeping.

### Decision: Gameplay scripts do not query physics or nav
- **Date:** 2026-10-15
- **Status:** Accepted
- **Context:** No shipped gameplay script called `PhysicsProxy` or `NavMeshProxy`. The only callers were `examples/scripting_advanced_demo`, `examples/scripting_playground`, and the unit tests that exercised the injection. The proxies held raw pointers into the live world.
- **Decision:** `script_system` does not inject them, on the main body or on event callbacks. Issue #317.
- **Consequences:** A script that names `physics` or `nav` errors and does not advance its result. The types remain registered for the mutation tests. #318 (`3971cbe1`) also changed the two demos that used to call the proxies. `advanced_ai.rhai` no longer calls `physics.raycast` or `nav.find_path`. `scripting_playground`'s player script no longer raycasts. The hosts still insert the resources.

### Decision: Hot reload is data staged for the next tick
- **Date:** 2026-10-14
- **Status:** Accepted
- **Context:** The old `script_system` compared file mtimes during the tick.
- **Decision:** Watchers enqueue bytes. The tick drains a `BTreeMap` at the start. Mtimes are not sim inputs.
- **Consequences:** Two worlds with the same staged bytes match. A stage after the system returns does not change that tick.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | Gameplay `script_system` iterates `CScript` entities in `Entity` order. Reloads come from `ScriptReloadQueue` and the content hash. The tick does not call `metadata.modified`. | Yes | `script_reload_applies_on_the_next_tick_only`, `script_reload_ignores_mtime_and_uses_the_hashed_queue` |
| 2 | Host scope keys (`commands`, `physics`, `nav`, `entity_id`, `position`, `health`, `delta_time`) are not stored in `script_state`. | Yes | `is_host_scope_key` |
| 3 | `mod_script_system` draws with `roll_i64`, which calls `SimRng::new(world_seed, tick, mix_sim_seed(system_id, entity, draw))`. | Yes | `roll_uses_canonical_sim_rng_and_is_bounded`, `two_runs_match_and_reload_waits_for_the_next_tick` |
| 4 | `move_to` outside `±DEFAULT_MAX_MOVE_DELTA` (100_000) leaves the agent in place and records an error. | Yes | `over_ceiling_move_does_not_change_position` |
| 5 | `scripted_system` SHA-256 is `362e4c6d3085d2a95d1219b988326beb4cbe898408de4a91e5b2abe3c3294d78` for seed `0xA11CE5E0`, 32 ticks, three agents, and the checked-in script. | Yes | `tests/script_replay_hash.rs` |
| 6 | A script swap recorded as `(tick, path, content hash, source)` and replayed from `ScriptReplayLog` matches the live digest. | Yes | `reload_mid_run_replays_to_the_identical_hash` |
| 8 | `build_app` installs `ModScriptPlugin` after `register_mod_runtime`. A mod loaded onto that app, including a mid-run swap in `ScriptReplayLog`, replays to the same digest. Manifest script map keys are the relative manifest paths. | Yes | `mod_loaded_through_build_app_replays_identically`, `load_mod_dir_keys_are_manifest_paths` |
| 9 | `queue_weave` is capability `Weave` at command API 2. The script passes a template id and an origin. The host fills tick, seq, and caster and calls `astraweave_weaving::queue_weave`. Ids 1 through 7 are engine ids. Ids outside `u16`, and cells outside ±2048, are rejected in the script call before a cast is stored. A failed `run_one` drops that call's casts. The tick walks sources, then entities (`sim_mod.rs:367-396`). `apply_weave_casts` runs only on `Ok`. An earlier script for the same agent that already returned `Ok` stays applied. `max_weaves` defaults to 64 and does not spend `terrain_edits`. Two runs enqueue the same commands. | Yes | `queue_weave_is_a_weave_command_and_weaves_do_not_spend_terrain`, `queue_weave_through_build_app_replays_identically`, `queue_weave_edges_fail_closed`, `failed_script_keeps_an_earlier_ok_for_the_same_agent` |
| 10 | The mod tick does not register `PhysicsProxy` or `NavMeshProxy`. The sim sandbox keeps the operation budget and `timeout_ms = 0`. `advertised_memory_bytes` is the structural product of the string, array, and map caps, not a process limit. | Yes | `mod_tick_rejects_physics_and_nav_proxies`, `sim_sandbox_keeps_the_op_budget_and_no_wall_clock` |
| 11 | Gameplay `script_system` does not inject `PhysicsProxy` or `NavMeshProxy`, even when `PhysicsWorld` and `NavMesh` resources are present. The event-callback scope is built the same way. | Yes | `script_system_rejects_physics_and_nav_proxies` |
| 7 | Author `configure` output that exceeds the director ceilings is an error. Hints cannot contain floats. | Yes | `sandboxed_configure_is_deterministic_and_capped` |

---

## 9. Performance & Resource Profile

Each mod script is compiled on the tick it runs. The replay case is 32 ticks of a short script. The operation cap is 50_000 per run. This is a correctness path, not a throughput claim.

---

## 10. Testing & Validation

- `cargo test -p astraweave-security --lib sandbox::`
- `cargo test -p astraweave-security --test mod_manifest_props`
- `cargo test -p astraweave-author --lib`
- `cargo test -p astraweave-scripting --lib`
- `cargo test -p astraweave-scripting --lib script_system_rejects_physics_and_nav_proxies`
- `cargo test -p astraweave-scripting --test script_replay_hash`
- `cargo test -p astraweave-scripting --lib mod_loaded_through_build_app_replays_identically`
- `cargo test -p aw_editor --lib -- file_watcher:: rhai_reload::`
- CI: `.github/workflows/determinism-replay.yml` requires `scripted_system` as one of the nineteen cross-OS cases. `target-cpu=native` is not set. Apple Silicon uses the default aarch64 baseline.

---

## 11. Open Questions / Parked Decisions

- **Play mode records the swap, then the mod system applies it on that tick.** `EditorRuntime::enter_play` calls `register_mod_runtime` and then `build_app`, which installs `ModScriptPlugin` before `sys_sim`. `queue_script_swap` stores the bytes. `run_fixed_steps` writes `(tick_count, path, sha256, source)` into the runtime log and the sim app's `ScriptReplayLog` only when `steps > 0`. `mod_script_system` stages `ModReloadQueue` from that log when `ModClock.tick` matches. A frame that runs zero steps does not assign a tick. Core does not name the plugin: scripting depends on core, so the installer is a function pointer recorded before `build_app`.
- **`execute_script_sandboxed` remains the async helper** and still ignores `allowed_functions`. New sim calls must use `execute_deterministic`.

---

## 12. Maintenance Notes

**Update this doc when:**
- A command is added to `COMMANDS` (bump `MOD_COMMAND_API_VERSION` and invariant 5 if the replay script changes).
- The editor starts applying staged scripts onto the play world.
- `scripted_system` is a required case in `determinism-replay.yml`.

## Revision history

| Version | Date | Notes |
|---|---|---|
| 1.7 | 2026-10-21 | A failed `run_one` drops that call only. An earlier script for the same agent stays applied. Section 10 says nineteen replay cases. `failed_script_keeps_an_earlier_ok_for_the_same_agent` locks the two-script case. |
| 1.6 | 2026-10-21 | A failed `run_one` drops only that agent's casts. An earlier `Ok` run stays applied. Verified at `49d289eb`. |
| 1.5 | 2026-10-21 | Script `queue_weave` rejects an origin outside ±2048 before storing the cast. `queue_weave_edges_fail_closed` covers ids, the island, the 64 cap, and replay equality. |
| 1.0 | 2026-10-14 | WS-MOD. Sandbox, mod loader, tick-boundary reload, `scripted_system` hash. |
| 1.1 | 2026-10-14 | `build_app` installs `ModScriptPlugin` via `register_mod_runtime`. Editor, game runtime, and `hello_companion` call it. |
| 1.3 | 2026-10-15 | Gameplay `script_system` does not inject `PhysicsProxy` or `NavMeshProxy`. #317, merged as #318. |
| 1.4 | 2026-10-16 | Demo follow-through. `advanced_ai.rhai` uses distance for sight and an empty patrol path. `scripting_playground` keeps trigger damage and drops the player raycast. Hosts still insert the physics and nav resources. |
| 1.2 | 2026-10-15 | `queue_weave` on capability `Weave`. Command API 2. Weave casts are a fixed buffer. |
