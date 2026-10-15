---
schema_version: 1
trace_id: veil
title: "Fate-scars"
description: "Deterministic Q32.32 fate-scar field sampled by volumetric fog and AI perception"
primary_crate: astraweave-veil
domain: gameplay
lifecycle_status: active
integration_status: wired
summary: "Deaths write a sparse fixed-point scar. Fog and agents read it."
owns: [astraweave-veil]
doc_version: "1.1"
last_verified_commit: 79fcf579
runtime_edges:
  - to: astraweave-ai
    note: stance_at reads ScarField through sense
  - to: aravath_runtime
    note: enemy deaths inject scars and aw-save meta restores them
---

# Architecture Trace: Fate-scars

## Metadata

| Field | Value |
|---|---|
| **System name** | Fate-scars (veil) |
| **Primary crates** | `astraweave-veil` |
| **Document version** | 1.1 |
| **Last verified against commit** | `79fcf579` (parent of the feature; the code landed on `cursor/aravath-veil-scars-7098`) |
| **Last verified date** | 2026-10-14 |
| **Status** | Active |
| **Owner notes** | Inspired by the Chevel / Fate-Weave note. The spillway, floodgate, and guild-warden content was not built. |

---

## 1. Executive Summary

**What this system does:**
A death writes transient energy into a sparse world-space grid. That energy decays, each tick, onto a permanent floor that never falls. The grid is Q32.32 fixed-point. Agents read the tension at an integer position. The existing froxel fog pass samples a small f32 volume baked from the same grid.

**Why it exists:**
Deaths in the Aravath demo were a health flag and a frozen clip. The scar is the deterministic memory the world and the nearby agents both observe.

**Where it primarily lives:**
- `astraweave-veil` — field, percept, postcard blob, GPU bake
- `astraweave-ai/src/scar_sense.rs` — `sense` and `stance_at`
- `astraweave-render` volumetric density and scatter shaders — one extra 3D sample, no second fog pass
- `aravath_runtime` — inject on enemy death, save, steer

**Status note:**
Float conversion exists only in `gpu_f32`, used by `bake_gpu_volume`. The simulation state has no `f32`, no `HashMap`, and no clock.

---

## 2. Authoritative Pipeline

```text
enemy health crosses 0
    │  aravath_runtime combat::world_metres  (f32 physics → i32 metres, once)
    ▼
ScarField::inject_death
    │  home cell += 48, six faces += 12, all Q32.32
    ▼
ScarField::tick   (once per 60 Hz sim step, not per wall-clock frame)
    │  transient moves toward 0; 1/3 of each step raises floor
    ▼
┌─ ScarField::perceive_metres ──► astraweave_ai::stance_at ──► enemy steer
└─ ScarField::bake_gpu_volume ──► Renderer::upload_veil_scar_volume
                                      │
                                      ▼
                               fog_density.wgsl sample_scar(world_pos)
                                      │  R density, G forward weight, B tint
                                      ▼
                               scatter.wgsl mixes anisotropy and amber-violet
```

The froxel grid stays the engine preset (80×45×32 in this demo). The scar texture is 32³ and is indexed by world position, not by `froxel_idx % 64`.

---

## 3. Semantic Vocabulary

| Term | Meaning |
|---|---|
| Cell | 4 m cube. Index is `metre.div_euclid(4)`. |
| Transient | Surge from a death. Decays. |
| Floor | Permanent share of the surge. Never decreases. |
| Effective tension | `floor + transient`, Q32.32. |
| Wary | Effective tension ≥ 12. |
| Avoid | Effective tension ≥ 28. |
| Retreat | First horizontal neighbor in the order -X, -Z, +X, +Z whose tension is strictly lower. |
| GPU volume | 32³ window of f32 texels. Not part of the hashed state. |

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-veil/src/fixed.rs` | `Fixed64` Q32.32 | Active | `#![forbid(unsafe_code)]` via the crate root |
| `astraweave-veil/src/field.rs` | `ScarField`, `BTreeMap` cells, FNV-1a `state_hash` | Active | |
| `astraweave-veil/src/percept.rs` | `ScarPercept`, horizontal retreat | Active | |
| `astraweave-veil/src/gpu.rs` | `bake_gpu_volume`, `gpu_f32` | Active | The only float conversion |
| `astraweave-veil/src/save_blob.rs` | Postcard hex in `SaveBundleV2.meta["veil_scars"]` | Active | Missing key loads an empty field |
| `astraweave-ai/src/scar_sense.rs` | `sense`, `stance_at` | Active | |
| `astraweave-render/shaders/volumetrics/fog_density.wgsl` | Samples `t_scar` at the froxel's world position | Active | |
| `astraweave-render/shaders/volumetrics/scatter.wgsl` | Reads density G/B for anisotropy and tint | Active | |
| `aravath_runtime/src/combat.rs` | Death inject, avoid / wary steering | Active | |

---

## 6. Conflict Map

- Do not add a second volumetric pass. The scar is an input of `VolumetricFogPass`.
- Do not store the scar in a `HashMap`. Iteration for the hash and the postcard list is `BTreeMap` key order.
- Do not put `f32` on `ScarField`, `ScarCell`, or `ScarPercept`.
- `world_metres` in the demo is the physics boundary. It is not part of the hashed field. Rapier positions are not a cross-machine contract; the field given the same integers is.
- `aw-save` file names contain a clock. The scar contract is `state_hash`, not the `.awsv` bytes.

---

## 7. Decision Log

- **Fixed-point grid, not the Chevel WGSL.** The supplied compute shader used `array<i64>` (WGSL has no `i64`) and `froxel_idx % 64` (not spatial). The sim stays integer. The GPU sees f32 texels.
- **Feed the existing pass.** A 16×9×24 pass would have duplicated `VolumetricFogPass` and ignored the demo's 80×45×32 low preset.
- **No spillway.** The demo has no city and no water network. Hydraulic grounding was not implemented.
- **Save via meta, schema stays 2.** `SaveBundleV2.meta["veil_scars"]` holds the hex postcard. Old saves omit the key and load an empty field. `ecs_blob` remains the edit log the demo already writes.
- **Cell-list cap stays on this branch.** `from_postcard` rejects more than 4096 cells before insert. The parser is not copied onto `main`.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | Crate is unsafe-free | Yes | `#![forbid(unsafe_code)]` |
| 2 | Sim modules do not mention `f32`, `f64`, `HashMap`, `SystemTime`, or `Instant` | Yes | Source scans in `fixed.rs`, `field.rs`, `percept.rs` |
| 3 | Two runs of `reference_history` plus three percepts hash to `0xb1cc29cc42401e83` | Yes | `cross_run_reference_history_matches_the_golden_hash` |
| 4 | Inject order of two deaths does not change the hash | Yes | `inject_order_does_not_change_the_hash` |
| 5 | After 400 ticks a death's transient is 0 and the floor is raised but below the peak | Yes | `transient_decays_onto_a_raised_floor` |
| 6 | `astraweave_ai::stance_at` on a scripted field hashes to `0x0eb14da1efc9e7b2` | Yes | `cross_run_stance_matches_the_golden_hash` |
| 7 | Postcard bytes round-trip through `SaveManager` with the same field hash | Yes | `aw_save_roundtrip_keeps_the_field_hash` |
| 8 | The GPU bake writes the injected cell at the center texel of a focus on that cell, not at `index % 64` | Yes | `density_sits_on_the_world_cell_that_was_injected` |
| 9 | `from_postcard` rejects `wire.cells` longer than `MAX_SCAR_CELLS` (4096) before `insert_loaded` | Yes | `postcard_rejects_more_than_4096_cells_before_insert` |

---

## 9. Performance & Resource Profile

- Sim tick walks occupied cells only. Three deaths are a handful of cells.
- GPU upload is 32³ RGBA16F = 256 KiB, copied when fog is on. `--no-volumetrics` does not build the fog pass and does not upload.
- Density and scatter still dispatch the existing froxel grid. Timestamps wrap those two passes when `TIMESTAMP_QUERY` is available (`VolumetricFogPass::poll_timestamps`). This environment has no GTX 1660. A software EGL device reported 64.7 ms / 47.8 ms on a cold dispatch and 2.24 ms / 2.03 ms on a later run, for the whole 80×45×32 density and scatter passes.

---

## 10. Testing & Validation

- `cargo test -p astraweave-veil --lib`
- `cargo test -p astraweave-ai --lib scar_sense`
- `cargo test -p aravath_runtime --lib` covers death injection, wary hold, avoid step, and save/load of the hash
- `scar_upload_is_timed_on_the_low_froxel_grid_when_the_device_can` in `astraweave-render` records density and scatter milliseconds

---

## 11. Open Questions / Parked Decisions

- **Should a settled floor ever fade?** Not in this version. The floor is the permanent scar. A later campaign can add a second, much slower decay if the island fills up.
- **Vertical retreat.** Agents only step on XZ. A scar directly above or below does not produce a walk direction.

---

## 12. Maintenance Notes

Update this trace when the cell size, the retain fraction, the wary/avoid thresholds, the GPU window, or the save meta key changes. Bump `doc_version` and re-lock the golden hashes in the same commit. Run `cargo run -p aw_trace_sync -- --write` after front-matter edits.
