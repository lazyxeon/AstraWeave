---
schema_version: 1
trace_id: cinematics
title: "Cinematics System (Timeline / Sequencer)"
description: "Cinematics — timeline / sequencer"
primary_crate: astraweave-cinematics
domain: rendering
lifecycle_status: active
integration_status: wired
owns: [astraweave-cinematics]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Cinematics System (Timeline / Sequencer)

## Metadata

| Field | Value |
|---|---|
| **System name** | Cinematics System (timeline / sequencer / camera-track playback) |
| **Primary crates** | `astraweave-cinematics` (canonical data layer); integrated by `astraweave-render`, `astraweave-gameplay`, `astraweave-ui`, `tools/aw_editor` [NEEDS VERIFICATION — this list omits a sixth non-example Cargo consumer, `veilweaver_slice_runtime`, which has a *headless* runtime `CinematicPlayer` (`veilweaver_slice_runtime/src/cinematic_player.rs:7,49`) wrapping `Sequencer`/`Timeline`/`SequencerEvent` and driven each tick by `GameLoop::process_cinematics` (`src/game_loop.rs:365-379`). It does **not** use `tick_cinematics` (GPU path), so the "only `tick_cinematics` caller" claims stand, but the data-layer consumer inventory in this doc (metadata, §1, §4, §5) is incomplete. Whether to fold this consumer into the trace is a structural call.] |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | Active (data layer); the production render path was wired during the Unified Camera campaign sub-phases C.7.A–C.7.E |
| **Owner notes** | Shaped heavily by the Unified Camera campaign (sub-phases C.5, C.7.0 audit, C.7.A–C.7.E). See [`docs/audits/cinematics_consolidation_audit_2026-05.md`](../audits/cinematics_consolidation_audit_2026-05.md). |

---

## 1. Executive Summary

**What this system does:**
`astraweave-cinematics` is a small, self-contained **data layer** for time-keyed cutscene/cinematic playback: a `Timeline` holds a flat list of `Track`s (camera keyframes, animation triggers, audio cues, FX cues), and a `Sequencer` walks a playhead forward in time emitting `SequencerEvent`s whose start falls inside the stepped interval ([`astraweave-cinematics/src/lib.rs:480-538`](../../astraweave-cinematics/src/lib.rs)).

**Why it exists:**
It provides the canonical, glam-free, serializable keyframe/timeline vocabulary (notably `CameraKey` with explicit `look_at` + `fov_deg`) that every consumer crate converts to/from, so cinematics camera state has a single source of truth.

**Where it primarily lives:**
- `astraweave-cinematics/src/lib.rs` (1843 lines, ~half tests) — the entire production surface.
- `astraweave-cinematics/tests/mutation_resistant_comprehensive_tests.rs` and `astraweave-cinematics/src/mutation_tests.rs` — test suites.
- `astraweave-cinematics/benches/cinematics_benchmarks.rs` — criterion benches (inline a private copy of the types; see §6).
- Integration code lives in the consumer crates (`astraweave-render/src/renderer.rs`, `astraweave-gameplay/src/cutscenes.rs`, `astraweave-ui/src/panels.rs`, `tools/aw_editor/src/panels/cinematics_panel.rs`). [NEEDS VERIFICATION — a sixth consumer, `veilweaver_slice_runtime/src/cinematic_player.rs`, also integrates the data layer (headless `Sequencer`-based `CinematicPlayer`); it is not listed here. See the metadata-row note.]

**Status note:**
As of commit `7c29b8182` the Unified Camera campaign's cinematics-consolidation chapter (C.7.A–C.7.E) has landed: `CameraKey` is the canonical keyframe type, `astraweave-render::Renderer::tick_cinematics` has a live production caller (`examples/cutscene_render_demo`), `astraweave-gameplay::cutscenes::Cue::CameraTo` migrated from yaw/pitch to `look_at` storage and now emits `CameraKey`, the editor panel's parallel `CameraKeyframe` type was retired in favour of `CameraKey`, and `CameraKey::sanitize` hardens the apply boundary. The `astraweave-ui` "Simple Cinematics" dev panel and the editor `CinematicsPanel`'s keyframe data remain UI-only (no runtime render flow) — see §6 / §11.

---

## 2. Authoritative Pipeline

There are **two distinct entry paths** into the cinematics data layer that converge on `Sequencer::step` (or, for the gameplay bridge, on `CameraKey` directly). The canonical render path is shown first; the gameplay-cutscene bridge second.

### 2.A Canonical render path (wired into `cutscene_render_demo`)

```text
[Authored timeline]
    │  awc::Timeline::new(...) + add_camera_track(vec![CameraKey...])
    │  (built programmatically in examples/cutscene_render_demo/src/main.rs:91-105)
    ▼
[Renderer::load_timeline / load_timeline_json]
    file: astraweave-render/src/renderer.rs:3447-3473
    role: stores Timeline in Renderer.cin_tl, resets cin_seq to t=0
    key data: Option<awc::Timeline>, awc::Sequencer
    │  Renderer::play_timeline() sets cin_playing = true
    ▼
[Renderer::tick_cinematics(dt, &mut FreeFly)]   (per-frame)
    file: astraweave-render/src/renderer.rs:3492-3516
    role: steps the sequencer, dispatches emitted events
    │  self.cin_seq.step(dt, tl) -> Vec<SequencerEvent>
    ▼
[Sequencer::step(dt, &Timeline)]
    file: astraweave-cinematics/src/lib.rs:480-538
    role: advance playhead from..to; emit events whose start ∈ (from..=to]
    key data: Vec<SequencerEvent>; returns Err(SeqError::Range) past duration+0.001
    │  per-event match in tick_cinematics
    ▼
[Event dispatch]
    SequencerEvent::CameraKey(k) -> Renderer::apply_camera_key(camera, k)
        file: astraweave-render/src/renderer.rs:3420-3445
        role: clone-and-sanitize the key, convert look_at -> yaw/pitch, write FreeFly
    SequencerEvent::FxTrigger{ name:"fade-in", .. } -> overlay_params.fade = 0.0
    (AnimStart / AudioPlay are returned to caller but not handled by the renderer)
    │
    ▼
[FreeFly camera state] -> camera.to_render_view() -> Renderer::update_view -> GPU camera UBO
    (cutscene_render_demo/src/main.rs:231,260)
```

### 2.B Gameplay-cutscene bridge (`Cue` script → `CameraKey`)

```text
[Authored Cue script]
    astraweave_gameplay::cutscenes::Timeline { cues: Vec<Cue> }
    file: astraweave-gameplay/src/cutscenes.rs:98-101
    (Cue::CameraTo { pos, look_at, fov_deg, time }, Cue::Title, Cue::Wait)
    │  CutsceneState::tick(dt, &Timeline)
    ▼
[CutsceneState::tick]
    file: astraweave-gameplay/src/cutscenes.rs:132-182
    role: advance cue index/timer; for CameraTo, convert Vec3 -> (f32,f32,f32)
          and emit a canonical awc::CameraKey
    key data: CutsceneTickEvent::{ Camera(CameraKey), Title(String), Continue, Done }
    │
    ▼
[Caller consumes CutsceneTickEvent]
    (in cutscene_render_demo this state machine now drives only Title/Wait;
     camera cues live in the awc::Timeline of path 2.A — see §6 dual-timeline note)
```

### Stage-by-stage detail

#### Stage: `Sequencer::step`
**File(s):** [`astraweave-cinematics/src/lib.rs:480-538`](../../astraweave-cinematics/src/lib.rs)
**Role:** Advances the playhead by `dt` and emits events.
**Inputs:** `dt: f32`, `&Timeline`.
**Outputs:** `Result<Vec<SequencerEvent>, SeqError>`.
**Notes:** Emission window is the half-open interval `(from..=to]` — `k.t.0 > from && k.t.0 <= to` (lib.rs:494). A key exactly at `t = 0` is therefore **never emitted** by a `step` that starts at `from = 0` (it requires `start > from`). Stepping past `duration + 0.001` returns `Err(SeqError::Range(next_t))` (lib.rs:482-484); the 0.001 tolerance lets a step land exactly on `duration`. `step` does **not** clamp or stop at duration — it errors, and on error `self.t` has *already* been advanced (line 487 runs before the per-track loop; the range check at 482 returns before mutation, so on the error path `self.t` is unchanged — verified lib.rs:481-487).

#### Stage: `Renderer::apply_camera_key`
**File(s):** [`astraweave-render/src/renderer.rs:3420-3445`](../../astraweave-render/src/renderer.rs)
**Role:** Converts a `CameraKey` (position + `look_at` + `fov_deg`) into `FreeFly` yaw/pitch/position/fovy.
**Inputs:** `&mut FreeFly`, `&awc::CameraKey`.
**Outputs:** mutates the `FreeFly`.
**Notes:** Private (`fn`, not `pub fn`). It **clones the key and calls `k.sanitize()`** before applying (renderer.rs:3433-3434, added C.7.D) so degenerate input (`look_at == pos`, out-of-range `fov_deg`) is hardened on the production path without callers remembering to sanitize. Conversion: `dir = (look - pos).normalize_or_zero(); yaw = dir.z.atan2(dir.x); pitch = dir.y.clamp(-1,1).asin(); fovy = fov_deg.to_radians()`.

#### Stage: `CameraKey::sanitize`
**File(s):** [`astraweave-cinematics/src/lib.rs:322-327`](../../astraweave-cinematics/src/lib.rs)
**Role:** Clamp pathological keyframes to valid ones at the cinematics layer.
**Inputs/Outputs:** `&mut self`.
**Notes:** Clamps `fov_deg` to `[10.0, 170.0]` (matching `astraweave_camera::FreeFly::sanitize`'s range from C.6.F); if `look_at == pos` *exactly*, sets `look_at = pos + (1,0,0)` (canonical +X forward). `pos` and `t` are never modified. **Exact-equality only** — near-degenerate (`look_at ≈ pos` but not equal) is explicitly out of scope (lib.rs docstring 313-316). The doc-comment (lib.rs:285-321) records the rationale and history (replaced the documentation-only `is_typical_fov` method).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| `Time` | Newtype `struct Time(pub f32)` — seconds. Carries arithmetic, conversion (`from_millis`), `lerp`, `clamp`, `Display`. | `astraweave-cinematics/src/lib.rs:7-92` |
| `Track` | One channel of a timeline: `Camera{keyframes}`, `Animation{target,clip,start}`, `Audio{clip,start,volume}`, `Fx{name,start,params}`. `#[non_exhaustive]`. | lib.rs:94-234 |
| `CameraKey` | Canonical camera keyframe: `t: Time`, `pos: (f32,f32,f32)`, `look_at: (f32,f32,f32)`, `fov_deg: f32`. **Tuple storage, no glam.** | lib.rs:236-356 |
| `Timeline` (cinematics) | `{ name: String, duration: Time, tracks: Vec<Track> }`. **Absolute-timestamp semantics** (track starts / keyframe `t` are timeline-absolute). | lib.rs:358-453 |
| `Sequencer` | Playhead `{ t: Time }`. `new`, `seek`, `step(dt,&Timeline)`. | lib.rs:463-539 |
| `SequencerEvent` | Emitted by `step`: `CameraKey(CameraKey)`, `AnimStart{target,clip}`, `AudioPlay{clip,volume}`, `FxTrigger{name,params}`. `#[non_exhaustive]`. | lib.rs:541-665 |
| `SeqError` | `#[non_exhaustive]` error; only variant `Range(Time)` (stepped past duration). | lib.rs:455-461 |
| `Cue` (gameplay) | `astraweave_gameplay::cutscenes::Cue` — a **cutscene script intention**: `CameraTo{pos,look_at,fov_deg,time}`, `Title`, `Wait`. Distinct type. | `astraweave-gameplay/src/cutscenes.rs:5-41` |
| `CutsceneTickEvent` | Gameplay tick output: `Camera(CameraKey)`, `Title(String)`, `Continue`, `Done`. Replaced the pre-C.7.A triple-Optional tuple. | cutscenes.rs:73-79 |
| `Timeline` (gameplay) | `astraweave_gameplay::cutscenes::Timeline { cues: Vec<Cue> }`. **Cue-duration semantics** (each cue's `time` is a duration, not an absolute timestamp). | cutscenes.rs:98-101 |

### Terms to NOT confuse

- **`Timeline` (cinematics) vs `Timeline` (gameplay):** Same name, two different types in two crates. Cinematics `Timeline` = `Vec<Track>` keyed by absolute timestamps; gameplay `Timeline` = `Vec<Cue>` keyed by per-cue durations. They coexist by design (cutscenes.rs:81-97); conversion happens inside `CutsceneState::tick`. Always qualify which crate you mean.
- **`CameraKey` (canonical) vs `CameraKeyframe` (retired):** `tools/aw_editor` formerly had a parallel `CameraKeyframe` with an extra `roll` field; it was retired in C.7.C and the editor now uses canonical `CameraKey` directly (cinematics_panel.rs:217-238). `CameraKeyframe` should no longer appear in the workspace.
- **`Cue::CameraTo.time` vs `CameraKey.t`:** `Cue::CameraTo.time` is a **cue duration**; `CameraKey.t` is an **absolute timestamp**. The gameplay-bridge conversion uses `Time(self.t)` (the cue-elapsed snapshot), documented as a deliberate semantic note at cutscenes.rs:144-156.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| Authoring code / demos | `awc::Timeline::new`, `add_camera_track`, `Track::camera/audio/animation/fx` factories | `Timeline` value | `examples/cinematics_timeline_demo`, `examples/cutscene_render_demo`, `astraweave-ui` "Load Demo" build timelines. |
| JSON assets | `serde_json::from_str::<Timeline>` via `Renderer::load_timeline_json` (renderer.rs:3447) and `astraweave-ui/src/panels.rs:270` | serialized `Timeline` | All cinematics types derive `Serialize`/`Deserialize`. |
| `astraweave-gameplay::cutscenes` | `CameraKey { .. }` constructed in `CutsceneState::tick` (cutscenes.rs:151-156) | `CameraKey` | The gameplay bridge produces canonical keys from `Cue::CameraTo`. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-render` | `Renderer::tick_cinematics` → `apply_camera_key` (renderer.rs:3492, 3420) | `SequencerEvent`, mutated `FreeFly` | Holds `cin_tl: Option<Timeline>`, `cin_seq: Sequencer`, `cin_playing: bool` (renderer.rs:882-884). Camera-key events drive `FreeFly`; `FxTrigger "fade-in"` clears `overlay_params.fade`; `AnimStart`/`AudioPlay` are returned to the caller unhandled. |
| `astraweave-camera` | `FreeFly` fields (`position`, `yaw`, `pitch`, `fovy`) written by `apply_camera_key`; `FreeFly::sanitize` mirrors `CameraKey::sanitize`'s `[10°,170°]` clamp range (freefly.rs:108-127) | camera pose | `astraweave-cinematics` has **no** dependency on `astraweave-camera`; the conversion lives in `astraweave-render`. |
| `examples/cutscene_render_demo` | `Renderer::load_timeline` + `play_timeline` + `tick_cinematics` (main.rs:154-155, 231) | rendered frames | **The only production runtime caller** of `tick_cinematics` (wired C.7.B). |
| `astraweave-ui` | `awc::Sequencer::step` → UI labels (panels.rs:316-324) | event strings | Dev-only "Simple Cinematics" panel; no renderer hook. |
| `tools/aw_editor` | `CinematicsPanel.camera_keyframes: Vec<CameraKey>`, `current_interpolated_key()` (cinematics_panel.rs:492, 1387) | preview-only `CameraKey` | UI-state only; `current_interpolated_key` is consumed only by the panel's own preview at cinematics_panel.rs:1272. |
| `veilweaver_slice_runtime` [NEEDS VERIFICATION — added during the v1.1 verification pass; not part of the original C.7 audit scope] | `CinematicPlayer::tick` → `Sequencer::step` (cinematic_player.rs:7,49); driven by `GameLoop::process_cinematics` (game_loop.rs:365-379) | `SequencerEvent` (discarded as `_events`) | **Headless** runtime consumer of the data layer. Loads `Timeline`s from RON (`load_from_ron`, cinematic_player.rs:106), steps the `Sequencer` each game tick, but the emitted events are currently dropped (`let _events = …`, game_loop.rs:369) — only playback lifecycle (`CinematicFinished`) is tracked. Does **not** use `tick_cinematics`/`apply_camera_key` (no GPU/render dep). |

### Bidirectional / Coupled

- **`astraweave-render` ↔ `astraweave-cinematics`:** The renderer owns the `Sequencer`/`Timeline` as renderer state and both steps it and applies its events. Tightly coupled via `tick_cinematics`. No reverse dependency (cinematics never references render).

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| [`astraweave-cinematics/src/lib.rs`](../../astraweave-cinematics/src/lib.rs) | Entire data layer: `Time`, `Track`, `CameraKey`, `Timeline`, `Sequencer`, `SequencerEvent`, `SeqError` + `#[cfg(test)]` tests | Active | `#![forbid(unsafe_code)]`. Lines ~1-665 production, ~667-1843 inline tests. |
| [`astraweave-cinematics/tests/mutation_resistant_comprehensive_tests.rs`](../../astraweave-cinematics/tests/mutation_resistant_comprehensive_tests.rs) | Mutation-resistant test suite (targets ~240 mutants) | Active | Imports the real crate (`use astraweave_cinematics::*`). |
| [`astraweave-cinematics/src/mutation_tests.rs`](../../astraweave-cinematics/src/mutation_tests.rs) | In-crate mutation test module (`mod mutation_tests`) | Active | Declared at lib.rs:4-5. |
| [`astraweave-cinematics/benches/cinematics_benchmarks.rs`](../../astraweave-cinematics/benches/cinematics_benchmarks.rs) | Criterion benchmarks | Active (bench) | **Inlines a private copy of the types** rather than importing the crate (file header lines 11-12). See §6. |
| [`astraweave-render/src/renderer.rs`](../../astraweave-render/src/renderer.rs) (cinematics region ~3419-3516) | `tick_cinematics`, `apply_camera_key`, `load_timeline[_json]`, `play/stop/seek_timeline` | Active | `use astraweave_cinematics as awc;` (renderer.rs:15). The canonical render integration. |
| [`astraweave-gameplay/src/cutscenes.rs`](../../astraweave-gameplay/src/cutscenes.rs) | `Cue`, `CutsceneTickEvent`, gameplay `Timeline`, `CutsceneState::tick` | Active | Bridge from cue-script → canonical `CameraKey`. Depends on `astraweave-cinematics` (added C.7.A). |
| [`examples/cutscene_render_demo/src/main.rs`](../../examples/cutscene_render_demo/src/main.rs) | Production demo; first caller of `tick_cinematics` | Active | Wired C.7.B. Two parallel timelines (awc camera + gameplay Title). |
| [`examples/cinematics_timeline_demo/src/main.rs`](../../examples/cinematics_timeline_demo/src/main.rs) | Data-layer demo (build Timeline, step Sequencer, print events) | Active | No renderer/window; pure data demo. |
| [`astraweave-ui/src/panels.rs`](../../astraweave-ui/src/panels.rs) (~238-363) | Dev-only "Simple Cinematics" panel (load/save JSON, step sequencer, label events) | Active (dev tool) | Uses canonical `awc` types; **no renderer flow** (events → UI labels only). |
| [`tools/aw_editor/src/panels/cinematics_panel.rs`](../../tools/aw_editor/src/panels/cinematics_panel.rs) | Editor cinematics authoring panel | Active (UI) | Post-C.7.C uses canonical `CameraKey` (`CameraKeyframe` retired, `roll` dropped). Keyframe data has **no runtime consumer** (preview-only). |
| [`tools/aw_editor/tests/render_parity_harness.rs`](../../tools/aw_editor/tests/render_parity_harness.rs) | Parity harness incl. `cinematics_driven` fixture exercising `tick_cinematics` | Active (test) | Lines ~1470, 2428-2495; gated on a live wgpu adapter. |

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| Cinematics `Timeline` (`Vec<Track>`, absolute timestamps) | `astraweave-cinematics/src/lib.rs:358-453` | Active | Canonical keyframe/timeline data layer. |
| Gameplay `Timeline` (`Vec<Cue>`, cue durations) | `astraweave-gameplay/src/cutscenes.rs:98-101` | Active | **Coexists by design** (cutscenes.rs:81-97 documents the dual-Timeline pattern as intentional: cue-script-level vs keyframe-state-level). Conversion at the `CutsceneState::tick` boundary. |
| Canonical `CameraKey` | `astraweave-cinematics/src/lib.rs:236-356` | Active | Canonical camera keyframe (look_at + fov_deg). |
| Editor `CameraKeyframe` (had extra `roll`) | (formerly) `tools/aw_editor/src/panels/cinematics_panel.rs` | Removed (C.7.C) | Retired; editor now uses `CameraKey`. `roll` was DORMANT (no render path) per the C.7.0 audit §3 and was dropped. |
| Benchmark inline type copies (`Time`/`Track`/`CameraKey`/`Timeline`) | `astraweave-cinematics/benches/cinematics_benchmarks.rs:11-50` | Active (bench-local) | The bench file deliberately **re-implements the types inline** ("to avoid dependency issues … crate is simple enough to inline", file header). A private mock, not a parallel production abstraction. |

### Naming collisions

- **`Timeline`**: In `astraweave-cinematics`, a `Vec<Track>` with absolute timestamps. In `astraweave-gameplay::cutscenes`, a `Vec<Cue>` with cue durations. Both are `pub struct Timeline`. The C.7.0 audit (L.5.18 / §1) and the C.7.A doc-comments treat these as intentionally distinct. Future direction: keep separate (documented decision).
- **`CameraKey` (cinematics) vs `Cue::CameraTo` (gameplay):** Related camera intents; `Cue::CameraTo` carries cue-duration `time` and is converted into a `CameraKey` (absolute `t`) inside `CutsceneState::tick`. Not a collision per se, but the `time`-vs-`t` semantic mismatch is a known footgun (cutscenes.rs:144-156).

### Known cognitive traps

- **Trap:** A `CameraKey` at `t = 0` is never emitted by a sequencer that starts at `from = 0`.
  **Why it's confusing:** The emission predicate is `start > from && start <= to` (lib.rs:494) — a half-open interval excluding the lower bound. A track/keyframe authored at exactly `t = 0` will silently never fire on the first step.
  **What's actually true:** This is by design (avoids double-emit at boundaries on successive steps), but means "fire immediately at timeline start" must be authored at a tiny positive `t`, or the consumer must apply the first key explicitly.

- **Trap:** The editor `CinematicsPanel` keyframe data and the `astraweave-ui` "Simple Cinematics" panel both *look* like they drive cinematics, but neither feeds a runtime renderer.
  **Why it's confusing:** Both have rich UI (sliders, load/save, sequencer step) suggesting a live preview.
  **What's actually true:** The editor panel's `current_interpolated_key()` is consumed only by its own preview label (cinematics_panel.rs:1272); the `astraweave-ui` panel renders sequencer events as text labels (panels.rs:316-324). Neither calls `Renderer::tick_cinematics`. The only live render caller is `examples/cutscene_render_demo`. (C.7.0 audit L.7.4 / L.7.5.)

- **Trap:** `Sequencer::step` returns `Err` (not a clamp) when stepping past the timeline duration.
  **Why it's confusing:** Many sequencers saturate at the end. This one returns `SeqError::Range`.
  **What's actually true:** Callers must handle the `Result`; `Renderer::tick_cinematics` swallows the error via `if let Ok(evs) = ...` (renderer.rs:3498), so a render that over-runs the timeline simply stops emitting (no panic, no advance).

---

## 7. Decision Log

### Decision: `CameraKey` is canonical; gameplay/editor consolidate onto it (direction A)
- **Date:** 2026-05-24 (audit) → executed C.7.A–C.7.E
- **Status:** Accepted
- **Context:** The C.5 audit finding L.5.18 found cinematics camera state in three parallel systems with no conversions: `astraweave_cinematics::CameraKey`, editor `CameraKeyframe` (extra `roll`), and `astraweave_gameplay::Cue::CameraTo` (yaw/pitch). ([`docs/audits/cinematics_consolidation_audit_2026-05.md`](../audits/cinematics_consolidation_audit_2026-05.md) §0, §1.)
- **Decision:** Make `CameraKey` canonical; migrate `Cue::CameraTo` to `look_at` storage (C.7.A) and the editor's `CameraKeyframe` into `CameraKey` (C.7.C); wire `cutscene_render_demo` to `tick_cinematics` (C.7.B); harden the apply boundary (C.7.D); close docs (C.7.E).
- **Alternatives considered:** Per the C.7 planning round, other "directions" existed but direction A (CameraKey canonical) was chosen. The audit (§2.D) notes `astraweave-cinematics` has zero `astraweave-*` deps, so any crate can depend on it without circular risk.
- **Consequences:** Two new dependency edges (`astraweave-gameplay → astraweave-cinematics`, `tools/aw_editor → astraweave-cinematics`); `tick_cinematics` gained its first production caller; `roll` was dropped (it was dormant — see below).

### Decision: Drop `roll` rather than absorb it (`α-drop`)
- **Date:** C.7.C (executed; audit recommendation 2026-05-24)
- **Status:** Accepted
- **Context:** The C.7.0 audit §3 empirically traced editor `CameraKeyframe.roll`: written by a slider, read for a label + a lerp producing another `CameraKeyframe`, but **never reaching any rendering path** (no `roll` on `CameraKey`, `FreeFly`, or `RenderView`). Verdict: **DORMANT, high confidence** (audit §3.C).
- **Decision:** Migrate `CameraKeyframe → CameraKey` without absorbing `roll`; remove the editor's roll slider/label. (cinematics_panel.rs:217-238, 1067-1069, 1295-1296.)
- **Alternatives considered:** (γ-as-feature-addition) implement a real roll rendering path; (γ-shim-only) add a serialized-but-inert `roll` field. The audit reframed the original Q2.γ "preserve a feature" assumption as false (it was UI-only), favouring the smaller `α-drop`.
- **Consequences:** `CameraKey` has no `roll`; authored roll values (if any existed) are not preserved forward.

### Decision: Sanitize at the apply boundary via `CameraKey::sanitize`, clamp range `[10°,170°]`
- **Date:** C.7.D (commit `17c73ae1b`, "Unified Camera C.7.D: apply_camera_key boundary hardening + CameraKey::sanitize")
- **Status:** Accepted
- **Context:** C.5 finding L.5.17: `apply_camera_key` silently accepted degenerate inputs (`look_at == pos` → `normalize_or_zero` → `atan2(0,0)=0`; out-of-range FOV unchecked). The pre-existing `is_typical_fov` (`30°..=120°`) was documentation-only with zero callers.
- **Decision:** Add `CameraKey::sanitize` (lib.rs:322-327): clamp `fov_deg` to `[10°,170°]` (harmonizing with `FreeFly::sanitize` from C.6.F) and resolve exact `look_at == pos` to `pos + (1,0,0)`. `apply_camera_key` clones-and-sanitizes before applying (renderer.rs:3433-3434). Removed `is_typical_fov`.
- **Alternatives considered:** Invoking `is_typical_fov` at the tighter `30°..=120°` range; rejecting/skipping degenerate keys with a warn. The wider canonical FreeFly range was chosen for cross-pipeline consistency (audit §6 Q6).
- **Consequences:** Production cinematics keys are hardened regardless of caller; near-degenerate (`look_at ≈ pos` not exactly equal) is explicitly out of scope (lib.rs:313-316).

### Decision: `CutsceneState::tick` returns a structured `CutsceneTickEvent` enum
- **Date:** C.7.A
- **Status:** Accepted
- **Context:** Pre-C.7.A the return was `(Option<(Vec3,f32,f32)>, Option<String>, bool)` — three Optionals in a tuple (C.7.0 audit L.7.2).
- **Decision:** Replace with `enum CutsceneTickEvent { Camera(CameraKey), Title(String), Continue, Done }` (cutscenes.rs:73-79). Per the audit §6 Q3, the structured-enum option was chosen over preserving the tuple shape.
- **Consequences:** At most one event per tick today; the type can evolve to `Vec<CutsceneTickEvent>` without changing the variant set (cutscenes.rs:68-72).

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `Sequencer::step` emits a track/keyframe iff its start ∈ `(from..=to]` (strictly `> from`, `<= to`). | Yes | Tests `seq_emits_events`, `seq_multiple_events_same_frame` (lib.rs:670-695, 808-835). |
| 2 | Stepping past `duration + 0.001` returns `Err(SeqError::Range)`; landing exactly on `duration` is `Ok`. | Yes | `sequencer_boundary_conditions`, `sequencer_out_of_range_error` (lib.rs:951-974). |
| 3 | `CameraKey::sanitize` clamps `fov_deg` to `[10.0,170.0]`, resolves exact `look_at==pos` to `pos+(1,0,0)`, never modifies `pos`/`t`. | Yes | `sanitize_*` tests (lib.rs:1416-1478). |
| 4 | `apply_camera_key` applies a *sanitized* key (clone-and-sanitize); the caller's key is unchanged. | Partial | Doc + code at renderer.rs:3428-3434; render_parity_harness `cinematics_driven` fixture. |
| 5 | `astraweave-cinematics` has zero `astraweave-*` dependencies (data-layer, glam-free). | Yes | `astraweave-cinematics/Cargo.toml` (serde/serde_json/anyhow/thiserror only). |
| 6 | All cinematics types round-trip through serde JSON. | Yes | `timeline_json_roundtrip`, `timeline_with_all_track_types_roundtrip` (lib.rs:697-716, 1001-1040). |
| 7 | `CameraKey`/`Timeline`/`Sequencer` use tuple `(f32,f32,f32)` storage, not `glam::Vec3`. | Yes | Struct definitions (lib.rs:236-242); no glam dep. |

---

## 9. Performance & Resource Profile

Not a hot-path system. `tick_cinematics` runs once per frame but `Sequencer::step` is an O(tracks × keyframes) linear scan over a typically tiny timeline; there is no spatial structure or per-frame allocation beyond the emitted `Vec<SequencerEvent>`. Benchmarks exist in [`astraweave-cinematics/benches/cinematics_benchmarks.rs`](../../astraweave-cinematics/benches/cinematics_benchmarks.rs) (timeline creation, sequencer step, JSON ser/de) but the crate is not performance-sensitive in the runtime budget. Resource ownership: the active `Timeline`/`Sequencer` are owned by `Renderer` (`cin_tl`, `cin_seq`, renderer.rs:882-884) for the render path, or by `OnceLock<Mutex<Option<...>>>` statics in the `astraweave-ui` dev panel (panels.rs:242-243).

---

## 10. Testing & Validation

- **Unit tests:** Extensive in-crate `#[cfg(test)] mod tests` (lib.rs:667-1843) — ~80 tests covering `Time`, `Track`, `CameraKey`, `Timeline`, `Sequencer`, `SequencerEvent`, sanitize, Display, factories, JSON round-trips.
- **Mutation testing:** `astraweave-cinematics/tests/mutation_resistant_comprehensive_tests.rs` (targets ~240 mutants) plus in-crate `src/mutation_tests.rs`. Commit `0ae5dfd24` records "complete mutation audit (99.12% raw, 100% adj)" for the crate.
- **Integration tests:** Gameplay-bridge tests in `astraweave-gameplay/src/cutscenes.rs:185-398` (cue progression, CameraTo→CameraKey conversion). Render-path parity in `tools/aw_editor/tests/render_parity_harness.rs` (`cinematics_driven` fixture exercising `tick_cinematics`, gated on a live wgpu adapter). Render coverage callers in `astraweave-render/tests/coverage_booster_render.rs`.
- **Miri validation:** Not applicable — `#![forbid(unsafe_code)]` (lib.rs:1).
- **Benchmarks:** `benches/cinematics_benchmarks.rs` (criterion; inlines type copies).
- **Manual validation:** `examples/cutscene_render_demo` (visual playback) and `examples/cinematics_timeline_demo` (console event dump).

---

## 11. Open Questions / Parked Decisions

- **Editor `CinematicsPanel` has no runtime consumer (L.7.4):** The panel authors `Vec<CameraKey>` and interpolates a preview key, but nothing emits this to a renderer or a saved `Timeline`. Is the panel intended to remain a preview-only authoring surface, or to eventually produce an `awc::Timeline` consumed by the editor viewport via `tick_cinematics`? (C.7.0 audit L.7.4, characterized as out-of-C.7-scope.)
- **Two parallel UI surfaces (L.7.5):** `astraweave-ui`'s dev "Simple Cinematics" panel and the editor's `CinematicsPanel` both operate on cinematics data with no renderer flow. Both now use canonical types post-C.7.C; whether they consolidate is parked (audit notes "post-C.9 cleanup queue candidate").
- **`AnimStart` / `AudioPlay` events are emitted but not handled by the renderer:** `tick_cinematics` returns them to the caller but only handles `CameraKey` and `FxTrigger "fade-in"` (renderer.rs:3500-3510). No production caller currently consumes the returned `AnimStart`/`AudioPlay` events. Is animation/audio dispatch intended to be wired, or left to the caller indefinitely?
- **`t = 0` keyframe emission:** The `(from..=to]` window means a key at exactly `t = 0` never fires on a step from `from = 0`. Is "fire at start" an intended non-feature, or should the first step special-case the lower bound? (No existing issue tracks this; surfaced here from code reading.)
- **Bench type duplication:** `cinematics_benchmarks.rs` inlines its own `Time`/`CameraKey`/`Timeline`/`Track` rather than importing the crate. This means benchmarks could drift from the real types silently. Intentional simplification per the file header; flagged for awareness only.

---

## 12. Maintenance Notes

**Update this doc when:**
- The cinematics region of `astraweave-render/src/renderer.rs` (`tick_cinematics`, `apply_camera_key`, `load_timeline*`) changes — especially event-dispatch arms.
- `astraweave-cinematics/src/lib.rs` adds/removes a `Track`/`SequencerEvent` variant or changes `Sequencer::step`'s emission window or `CameraKey::sanitize`.
- The gameplay bridge (`Cue`, `CutsceneTickEvent`, `CutsceneState::tick`) changes shape.
- L.7.4 / L.7.5 (the dormant UI surfaces) get wired to a runtime consumer.

**Verification process:**
- Re-grep production callers of `tick_cinematics` (`rg 'tick_cinematics' -g '!*test*' -g '!*bench*'`) to confirm the wired-vs-dormant status in §4/§6.
- Spot-check the §2 pipeline against `renderer.rs` and `cutscenes.rs` line ranges.
- Stamp the new commit hash and date in the metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. The data layer is canonical and glam-free (`(f32,f32,f32)` tuples, not `Vec3`). The Vec3↔tuple boundary lives in the *consumer* crates (`apply_camera_key`, `CutsceneState::tick`), never in `astraweave-cinematics`.
2. `Sequencer::step` emits on `(from..=to]` (excludes `from`) and **returns `Err` past duration** — it does not clamp. A `t = 0` key won't fire on the first step from `0`.
3. Only `examples/cutscene_render_demo` actually drives a renderer via `tick_cinematics`. The editor and `astraweave-ui` cinematics panels are UI/preview-only (dormant render flow).
4. `apply_camera_key` already clones-and-sanitizes; don't add a second sanitize at the call site, and don't assume the caller's `CameraKey` was mutated.

**Files you'll most likely touch:**
- `astraweave-cinematics/src/lib.rs` (the data layer)
- `astraweave-render/src/renderer.rs` (~3419-3516, the render integration)
- `astraweave-gameplay/src/cutscenes.rs` (the cue-script bridge)

**Files you should NOT touch without strong reason:**
- `astraweave-cinematics/benches/cinematics_benchmarks.rs` — inlines a private copy of the types; editing the real types does NOT update the bench copy (and vice versa).
- `astraweave-render/src/renderer.rs::apply_camera_key` — private, hardened C.7.D; the sanitize-before-apply ordering is load-bearing for L.5.17.

**Common mistakes when changing this system:**
- **Confusing the two `Timeline`s:** cinematics `Timeline` (timestamps, `Vec<Track>`) vs gameplay `Timeline` (durations, `Vec<Cue>`). They are intentionally distinct types in different crates.
- **Assuming the editor panel renders cinematics:** it does not; its keyframe data is preview-only with no runtime consumer.
- **Treating `Sequencer::step`'s `Err` as a stop signal silently:** the render path swallows it (`if let Ok`); other callers must decide whether over-running the timeline is an error or an end-of-playback.

---

## Appendix B: Historical context

`astraweave-cinematics` began as a minimal timeline/sequencer data crate (the original `seq_emits_events` test and the `(from..=to]` window predate the camera campaign). The Unified Camera campaign's Phase 2 audit (C.5) discovered that cinematics camera state had fragmented into three parallel representations with no conversion functions (`docs/audits/camera_system_phase_2_audit_2026-05.md`, finding L.5.18; boundary handling L.5.17). The C.7.0 consolidation audit ([`docs/audits/cinematics_consolidation_audit_2026-05.md`](../audits/cinematics_consolidation_audit_2026-05.md), 2026-05-24) inventoried the surface across six crates, empirically proved the editor's `roll` field was dormant, and scoped the consolidation as sub-phases C.7.A–C.7.E. Those sub-phases (all landed by commit `7c29b8182`) made `CameraKey` canonical, wired the first production caller of `tick_cinematics`, migrated the gameplay cue bridge to `look_at` storage with a structured event enum, retired the editor's parallel `CameraKeyframe`, and hardened the apply boundary with `CameraKey::sanitize`.
