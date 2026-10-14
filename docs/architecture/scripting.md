---
schema_version: 1
trace_id: scripting
title: "Scripting and mod API"
description: "Sandboxed Rhai gameplay scripts, the mod loader, and tick-boundary reload"
primary_crate: astraweave-scripting
domain: gameplay
lifecycle_status: active
integration_status: partial
summary: "Gameplay and mod scripts run on the security sandbox. Author scripts and editor .rhai reload call the same loader. Native plugin template is not a runtime."
owns: [astraweave-scripting, astraweave-author]
doc_version: "1.0"
last_verified_commit: 85d09a2c
runtime_edges:
  - to: astraweave-security
    note: "ScriptEngineResource and mod_script_system call sandbox::SandboxEngine / execute_deterministic"
---

# Architecture Trace: Scripting and mod API

## Metadata

| Field | Value |
|---|---|
| **System name** | Scripting and mod API |
| **Primary crates** | `astraweave-scripting`, `astraweave-author` |
| **Document version** | 1.0 |
| **Last verified against commit** | `85d09a2c` |
| **Last verified date** | 2026-10-14 |
| **Status** | Active for the sandboxed Rhai path. Examples still exercise `ScriptingPlugin`. The editor stages `.rhai` but does not feed `CScript` entities. |
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
`script_system` no longer stats files during the tick. Reloads come from `ScriptReloadQueue::take_boundary` at the start of the system. `mod_script_system` uses a separate `ModReloadQueue` so the two systems do not steal each other's bytes.

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
| `MOD_COMMAND_API_VERSION` | `1`. Also re-exported as `SCRIPT_COMMAND_API_VERSION`. |
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
| `ModScriptPlugin` | `mod_script_system` | Integer agent sim. Replay case `scripted_system`. |
| `aw_editor` `main.rs` | `RhaiHotReload` | Stages `.rhai` and applies before `runtime.tick`. |
| `tools/aw_plugin_template` | README | Non-runtime scaffold. Not a loadable mod. |

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-security/src/sandbox.rs` | Sandbox, manifests, queue | Active | See `docs/architecture/security.md`. |
| `astraweave-scripting/src/lib.rs` | `CScript`, `script_system` | Active | Entity iteration is sorted. `script_state` is a `BTreeMap`. No `SystemTime` on the tick. |
| `astraweave-scripting/src/api.rs` | `ScriptCommand` set | Active | Re-exports `SCRIPT_COMMAND_API_VERSION`. |
| `astraweave-scripting/src/loader.rs` | Async relative `.rhai` load | Active | Rejects absolute paths and `..`. Size-capped. |
| `astraweave-scripting/src/sim_mod.rs` | Mod tick + digest | Active | `MOD_SYSTEM_ID = 0x4D4F4453`. |
| `astraweave-scripting/tests/script_replay_hash.rs` | `scripted_system` SHA-256 | Active | Appends the case to `target/replay-hash.json`. |
| `astraweave-author/src/lib.rs` | `configure` | Active | Floats in hints are rejected. |
| `tools/aw_editor/src/rhai_reload.rs` | Editor boundary | Active | |
| `tools/aw_plugin_template/` | Scaffold | Non-runtime | README says so. |

---

## 6. Conflict Map / Residue

- `script_system` and `mod_script_system` are two callers of one sandbox, not two engines. They use different reload queues (`ScriptReloadQueue` vs `ModReloadQueue`).
- `execute_script_sandboxed` in `astraweave-security` is the async helper. Sim code does not call it.
- `register_api` still exposes physics and nav proxies on the gameplay engine. Those proxies are host keys and are not written back into `script_state`.
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
| 1 | Gameplay `script_system` iterates `CScript` entities in `Entity` order and does not call `std::fs` or `SystemTime`. | Yes | `script_reload_applies_on_the_next_tick_only` |
| 2 | Host scope keys (`commands`, `physics`, `nav`, `entity_id`, `position`, `health`, `delta_time`) are not stored in `script_state`. | Yes | `is_host_scope_key` |
| 3 | `mod_script_system` draws with `roll_i64`, which calls `SimRng::new(world_seed, tick, mix_sim_seed(system_id, entity, draw))`. | Yes | `roll_uses_canonical_sim_rng_and_is_bounded`, `two_runs_match_and_reload_waits_for_the_next_tick` |
| 4 | `move_to` outside `±DEFAULT_MAX_MOVE_DELTA` (100_000) leaves the agent in place and records an error. | Yes | `over_ceiling_move_does_not_change_position` |
| 5 | `scripted_system` SHA-256 is `362e4c6d3085d2a95d1219b988326beb4cbe898408de4a91e5b2abe3c3294d78` for seed `0xA11CE5E0`, 32 ticks, three agents, and the checked-in script. | Yes | `tests/script_replay_hash.rs` |
| 6 | A script swap recorded as `(tick, path, content hash, source)` and replayed from `ScriptReplayLog` matches the live digest. | Yes | `reload_mid_run_replays_to_the_identical_hash` |
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
- `cargo test -p astraweave-scripting --test script_replay_hash`
- `cargo test -p aw_editor --lib -- file_watcher:: rhai_reload::`
- CI: `.github/workflows/determinism-replay.yml` requires `scripted_system` along with `planner_pure` and `perception_world`. `target-cpu=native` is not set. Apple Silicon uses the default aarch64 baseline.

---

## 11. Open Questions / Parked Decisions

- **Play mode records the swap, then commits it on the tick that runs.** `EditorRuntime::queue_script_swap` stores the bytes. `run_fixed_steps` writes `(tick_count, path, sha256, source)` into `ScriptReplayLog` and stages `ScriptReloadQueue` on the sim app immediately before the fixed steps. A frame that runs zero steps does not assign a tick. The play app from `build_app` does not register `ModScriptPlugin`; the log is what replay consumes.
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
| 1.0 | 2026-10-14 | WS-MOD. Sandbox, mod loader, tick-boundary reload, `scripted_system` hash. |
