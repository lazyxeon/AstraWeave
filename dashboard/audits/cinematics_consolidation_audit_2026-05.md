# Cinematics Consolidation Audit (Sub-phase C.7.0)

| Field | Value |
|-------|-------|
| Date | 2026-05-24 |
| Scope | Read-only static source analysis across six crates + test infrastructure. No source files modified. |
| Sub-phase | C.7.0 (Unified Camera campaign — cinematics consolidation preliminary audit) |
| Companion to | `docs/audits/camera_system_architecture_audit_2026-05.md` (C.0; pre-campaign Phase 1 audit) and `docs/audits/camera_system_phase_2_audit_2026-05.md` (C.5; mid-campaign Phase 2 audit) |
| Audits | Cinematics surface as scoped by C.7 planning round's direction A (CameraKey canonical; absorb Cue::CameraTo + CameraKeyframe) |
| Forward chain | C.7.A planning round → execution sub-phases C.7.A–C.7.E |
| Doc artifact authorization | C.7.0 launch prompt |

## 0. Why this audit exists

The Unified Camera campaign has executed C.0 through C.6 inclusive. The renderer-side chapter (C.0–C.3.D), editor-side first-pass (C.4 + C.4.B), Phase 2 audit (C.5), and per-target migration queue (C.6) are all closed. After C.6, only cinematics-related rows remain open in `CAMERA_CONVENTIONS.md` §3 migration table.

C.5's audit finding L.5.18 surfaced that cinematics camera state exists in **three parallel systems** with no conversion functions between them:

- `astraweave_cinematics::CameraKey` (canonical-by-design; zero production callers via `Renderer::tick_cinematics`)
- `tools/aw_editor/src/panels/cinematics_panel.rs::CameraKeyframe` (editor UI-only; extra `roll` field)
- `astraweave_gameplay::cutscenes::Cue::CameraTo` (active production path via `examples/cutscene_render_demo`)

The C.7 planning round chose **direction A** (CameraKey canonical; Cue::CameraTo + CameraKeyframe consolidate into it) and scoped execution as C.7.A (Cue::CameraTo look_at migration) → C.7.B (`cutscene_render_demo` rewrite) → C.7.C (CameraKeyframe retirement + roll absorption) → C.7.D (`apply_camera_key` boundary hardening for L.5.17) → C.7.E (documentation closeout). Parity coverage deferred to C.8.

C.7.0 is the **preliminary audit sub-phase** for that multi-crate consolidation. Third audit-shaped sub-phase in the campaign (after C.0 pre-campaign and C.5 mid-campaign); its job is empirical inventory + dependency mapping + per-sub-phase scope estimation + one load-bearing investigation (roll dormancy) before C.7.A drafts. **No source code changes; no migration proposals locked; calibrated language throughout per C.5 Decision 5's α.**

C.7.0 also carries the load-bearing answer to the planning round's **Q2.γ empirical question**: *does `CameraKeyframe.roll` actually produce visible rendered output today, or is it currently UI-state-only dormant?* The finding affects whether γ is preservation-of-feature or addition-of-feature. **See §3 for the empirical verdict.**

## 1. Cinematics surface inventory across crates

### 1.A `astraweave-cinematics` (canonical home)

**File:line citations**:

- `astraweave-cinematics/src/lib.rs:7-92` — `Time(pub f32)` struct + arithmetic/conversion methods + Display + Add/Sub
- `astraweave-cinematics/src/lib.rs:94-201` — `Track` enum: `Camera { keyframes: Vec<CameraKey> }`, `Animation { target, clip, start }`, `Audio { clip, start, volume }`, `Fx { name, start, params }`
- `astraweave-cinematics/src/lib.rs:237-307` — **`CameraKey` struct**: `t: Time`, `pos: (f32, f32, f32)`, `look_at: (f32, f32, f32)`, `fov_deg: f32`. Methods: `new`, `at_origin`, `position`, `distance_to_target`, `fov_rad` (line 282: `self.fov_deg.to_radians()`), `is_typical_fov` (line 286-288: `(30.0..=120.0).contains(&self.fov_deg)` — validation method exists but is documentation-only per C.5 L.5.17 — `apply_camera_key` does NOT invoke it), `lerp` (line 291-306)
- `astraweave-cinematics/src/lib.rs:309-317` — `Display for CameraKey`
- `astraweave-cinematics/src/lib.rs:319-414` — `Timeline` struct: `name: String`, `duration: Time`, `tracks: Vec<Track>`; methods for adding/counting tracks
- `astraweave-cinematics/src/lib.rs:416-422` — `SeqError` enum
- `astraweave-cinematics/src/lib.rs:424-499` — `Sequencer` struct: `t: Time`; methods `new`, `seek`, `step(dt, &Timeline) -> Result<Vec<SequencerEvent>>`. The `step` method emits events whose `start.0 > from && start.0 <= to`.
- `astraweave-cinematics/src/lib.rs:502-518` — `SequencerEvent` enum: `CameraKey(CameraKey)`, `AnimStart { target, clip }`, `AudioPlay { clip, volume }`, `FxTrigger { name, params }`

**Field-level analysis of CameraKey** (the consolidation target):

| Field | Type | Unit | Notes |
|---|---|---|---|
| `t` | `Time` (newtype around `f32`) | seconds | timeline timestamp |
| `pos` | `(f32, f32, f32)` | world | tuple, not `glam::Vec3` (the crate has no glam dep — see Cargo.toml) |
| `look_at` | `(f32, f32, f32)` | world | tuple |
| `fov_deg` | `f32` | **degrees** | explicit name (better than ambiguous `fov`); `.fov_rad()` accessor converts at read boundary |

**Cargo.toml dependencies** (`astraweave-cinematics/Cargo.toml`):
```toml
[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
```

**No `astraweave-*` dependencies whatsoever.** No `glam` dependency. Self-contained data-layer crate; the tuple-based `(f32, f32, f32)` storage is precisely because the crate doesn't depend on glam. This is structurally relevant for §2's dependency-direction analysis: every other workspace crate can depend on `astraweave-cinematics` without circular-dependency risk.

### 1.B `astraweave-render` (cinematics integration via `awc`)

**File:line citations**:

- `astraweave-render/src/renderer.rs:15` — `use astraweave_cinematics as awc;` (the alias used throughout the file)
- `astraweave-render/src/renderer.rs:3371-3381` — **`Renderer::apply_camera_key`** (private function; verified by absence of `pub` keyword):
  ```rust
  fn apply_camera_key(cam: &mut FreeFly, k: &awc::CameraKey) {
      let pos = glam::Vec3::new(k.pos.0, k.pos.1, k.pos.2);
      let look = glam::Vec3::new(k.look_at.0, k.look_at.1, k.look_at.2);
      let dir = (look - pos).normalize_or_zero();
      let yaw = dir.z.atan2(dir.x);
      let pitch = dir.y.clamp(-1.0, 1.0).asin();
      cam.position = pos;
      cam.yaw = yaw;
      cam.pitch = pitch;
      cam.fovy = k.fov_deg.to_radians();
  }
  ```
  **C.5 L.5.17 reaffirmed**: `normalize_or_zero` silently returns `Vec3::ZERO` when `look_at == pos`; subsequent `atan2(0, 0) = 0` and `asin(0) = 0` yield a degenerate-but-non-NaN camera pointing along `+X`. `is_typical_fov()` validation is not invoked.

- `astraweave-render/src/renderer.rs:3407-3431` — **`Renderer::tick_cinematics`** (`pub fn`):
  ```rust
  pub fn tick_cinematics(&mut self, dt: f32, camera: &mut FreeFly) -> Vec<awc::SequencerEvent> {
      // … checks self.cin_playing, steps self.cin_seq against self.cin_tl,
      // dispatches CameraKey events via Self::apply_camera_key,
      // handles FxTrigger "fade-in" via self.overlay_params.fade = 0.0,
      // ignores other event types
  }
  ```

- `astraweave-render/src/renderer.rs:3383-3404` (just above `apply_camera_key`) — supporting cinematics state: `load_timeline_json`, `save_timeline_json`, `play_timeline`, `stop_timeline`, `seek_timeline` (all `pub fn`); inferred from grep `cin_tl`/`cin_seq`/`cin_playing` field references.

**Production callers of `tick_cinematics`** (verified by `grep -rn "tick_cinematics"`):

- `astraweave-render/src/renderer.rs:3407` — declaration
- `astraweave-render/tests/coverage_booster_render.rs:137, 6763` — two test-only callers

**Zero production callers.** Per C.5 L.5.18 and reaffirmed here.

**Cargo.toml dependencies** (relevant subset, from `astraweave-render/Cargo.toml`):
```toml
astraweave-camera = { path = "../astraweave-camera" }
astraweave-cinematics = { path = "../astraweave-cinematics" }
```

### 1.C `astraweave-gameplay` (active production cinematics path; parallel implementation)

**File:line citations**:

- `astraweave-gameplay/src/cutscenes.rs:1-20` — `Cue` enum (`#[non_exhaustive]`):
  ```rust
  pub enum Cue {
      CameraTo { pos: Vec3, yaw: f32, pitch: f32, time: f32 },
      Title { text: String, time: f32 },
      Wait { time: f32 },
  }
  ```
- `astraweave-gameplay/src/cutscenes.rs:22-25` — `Timeline { cues: Vec<Cue> }` (distinct type from `astraweave_cinematics::Timeline`)
- `astraweave-gameplay/src/cutscenes.rs:27-30` — `CutsceneState { idx: usize, t: f32 }`
- `astraweave-gameplay/src/cutscenes.rs:38-86` — `CutsceneState::tick(&mut self, dt: f32, tl: &Timeline) -> (Option<(Vec3, f32, f32)>, Option<String>, bool)`:
  - For `Cue::CameraTo`, returns `Some((pos, yaw, pitch))` as the first tuple element until the cue's `time` elapses
  - For `Cue::Title`, returns `Some(text.clone())` as the second tuple element
  - For `Cue::Wait`, returns no camera update
  - The third bool is "timeline done"
- `astraweave-gameplay/src/cutscenes.rs:88-276` — tests (no rendering; pure unit tests)

**Field-level analysis of Cue::CameraTo** vs target `CameraKey`:

| Cue::CameraTo (current) | CameraKey (canonical target) | Note |
|---|---|---|
| `pos: glam::Vec3` | `pos: (f32, f32, f32)` | Vec3 ↔ tuple boundary needed |
| `yaw: f32` | (no equivalent) | DROPPED in look_at model |
| `pitch: f32` | (no equivalent) | DROPPED in look_at model |
| (no equivalent) | `look_at: (f32, f32, f32)` | ADDED |
| (no equivalent) | `fov_deg: f32` | ADDED — Cue::CameraTo has no FOV field |
| `time: f32` (cue duration) | `t: Time` (timestamp) | **Semantic mismatch**: Cue's `time` is duration; CameraKey's `t` is absolute timestamp |

The `time` field semantic mismatch is a non-trivial C.7.A design point — see §6 question for the planning round.

**Cargo.toml dependencies** (relevant subset, from `astraweave-gameplay/Cargo.toml`):
```toml
glam = { workspace = true }
astraweave-core = { path = "../astraweave-core" }
astraweave-physics = { path = "../astraweave-physics" }
astraweave-nav = { path = "../astraweave-nav" }
astraweave-ecs = { workspace = true }
astraweave-input = { path = "../astraweave-input" }
astraweave-scene = { path = "../astraweave-scene" }
```

**No `astraweave-cinematics` dependency.** This is the structural finding: direction A requires `astraweave-gameplay` to add `astraweave-cinematics` as a dependency in C.7.A.

### 1.D `tools/aw_editor` (parallel editor UI implementation)

**File:line citations**:

- `tools/aw_editor/src/panels/cinematics_panel.rs:218-224` — **`CameraKeyframe` struct** (parallel to canonical):
  ```rust
  pub struct CameraKeyframe {
      pub time: f32,
      pub position: (f32, f32, f32),
      pub look_at: (f32, f32, f32),
      pub fov: f32,
      pub roll: f32,
  }
  ```
- `tools/aw_editor/src/panels/cinematics_panel.rs:226-236` — `Default` impl (`time: 0.0, position: (0.0, 5.0, -10.0), look_at: (0.0, 0.0, 0.0), fov: 60.0, roll: 0.0`)
- `tools/aw_editor/src/panels/cinematics_panel.rs:481-502` — `CinematicsPanel` struct (holds `camera_keyframes: Vec<CameraKeyframe>` at line 489)
- `tools/aw_editor/src/panels/cinematics_panel.rs:597-617` — keyframe creation sites (lines 601, 609 inside event handlers; both set `roll: 0.0`)
- `tools/aw_editor/src/panels/cinematics_panel.rs:953-998` — keyframe-add/edit UI loop
- `tools/aw_editor/src/panels/cinematics_panel.rs:1063` — FOV UI slider (`Slider::new(&mut keyframe.fov, 10.0..=120.0).suffix("°")`)
- `tools/aw_editor/src/panels/cinematics_panel.rs:1068` — **roll UI slider write** (`Slider::new(&mut keyframe.roll, -180.0..=180.0).suffix("°")`)
- `tools/aw_editor/src/panels/cinematics_panel.rs:1272-1302` — "Current Camera State" preview UI; lines 1284-1297 read keyframe.position/look_at/fov/roll for label display
- `tools/aw_editor/src/panels/cinematics_panel.rs:1296` — **roll UI label read** (`ui.label(format!("{:.1}°", keyframe.roll))`)
- `tools/aw_editor/src/panels/cinematics_panel.rs:1379-1437` — `get_interpolated_camera(&self) -> Option<CameraKeyframe>` lerp method
- `tools/aw_editor/src/panels/cinematics_panel.rs:1417` — **roll lerp** (`roll: b.roll + (a.roll - b.roll) * t` — read for production of another CameraKeyframe)
- `tools/aw_editor/src/panels/cinematics_panel.rs:1476-1479` — `add_camera_keyframe` API + sort-by-time

**Cargo.toml dependencies** (verified by `grep "astraweave-cinematics\|astraweave-camera" tools/aw_editor/Cargo.toml`):
```toml
astraweave-camera = { path = "../../astraweave-camera" }
```

**No `astraweave-cinematics` dependency.** Direction A's C.7.C requires adding it.

**No conversion functions between editor `CameraKeyframe` and canonical `astraweave_cinematics::CameraKey`** (verified by `grep -rn "astraweave_cinematics\|use awc" tools/aw_editor/` — zero matches).

### 1.E `examples/cutscene_render_demo` (the production cinematics demo)

**File:line citations** (`examples/cutscene_render_demo/src/main.rs`):

- Line 1: `use astraweave_gameplay::cutscenes::*;` (imports the parallel `Cue`, `Timeline`, `CutsceneState`)
- Line 2: `use astraweave_camera::{CameraController, CameraProducer, FreeFly as Camera};` (canonical camera via alias pattern)
- Line 3: `use astraweave_render::Renderer;`
- Lines 16-25: `CutsceneApp` struct (holds `camera: Camera`, `ctl: CameraController`, `tl: Timeline`, `cs: CutsceneState`)
- Lines 28-73: `CutsceneApp::new()` — constructs a Timeline with `Cue::Title`, `Cue::Wait`, and two `Cue::CameraTo` cues (lines 47-58)
- Lines 137-165: `about_to_wait` (the main tick loop):
  - Line 151: `let (cam, _title, _done) = self.cs.tick(dt, &self.tl);`
  - Lines 152-156: if Some, **directly assigns to FreeFly fields**:
    ```rust
    if let Some((pos, yaw, pitch)) = cam {
        self.camera.position = pos;
        self.camera.yaw = yaw;
        self.camera.pitch = pitch;
    } else {
        self.ctl.update_camera(&mut self.camera, dt);
    }
    ```
  - Line 160: `renderer.update_view(&self.camera.to_render_view());` (canonical upload)

**Critical structural observation**: this demo bypasses `astraweave_cinematics::CameraKey`, `Renderer::apply_camera_key`, and `Renderer::tick_cinematics` entirely. The yaw/pitch values flow directly from `Cue::CameraTo` into FreeFly's yaw/pitch fields without going through any canonical conversion. C.7.B's rewrite changes this to use `tick_cinematics`.

**Cargo.toml dependencies** (from `examples/cutscene_render_demo/Cargo.toml`):
```toml
astraweave-render = { path = "../../astraweave-render" }
astraweave-camera = { path = "../../astraweave-camera" }
astraweave-gameplay = { path = "../../astraweave-gameplay" }
```

**No `astraweave-cinematics` dependency.** Could remain absent post-C.7.B if `astraweave-gameplay` re-exports the necessary cinematics types, or added if the demo invokes `tick_cinematics` directly with `awc::Timeline`. Design decision for C.7.B planning round.

### 1.F `astraweave-ui` (dev-only Simple Cinematics panel)

**File:line citations** (`astraweave-ui/src/panels.rs`):

- Line 238-363: "Simple Cinematics panel (dev-only)" — egui window
- Line 242-244: `OnceLock<Mutex<Option<awc::Timeline>>>` and `OnceLock<Mutex<Option<awc::Sequencer>>>` static state
- Line 269-285: "Load File" button (deserializes `awc::Timeline` from JSON file)
- Line 287-303: "Save File" button (serializes `awc::Timeline` to JSON file at user-typed path; creates `assets/cinematics/` directory if missing)
- Line 304-311: "Play" button (initializes a new `awc::Sequencer`)
- Line 312-328: "Step 0.5s" button — calls `seqv.step(0.5, tlv)` and renders events as UI labels (line 324: `ui.label(format!("{:4.1}s: {:?}", seqv.t.0, e))`)
- Line 330-362: "Load Demo" button (constructs a sample Timeline with `awc::CameraKey` keyframes at lines 334-345, an `awc::Track::Audio` clip, and an `awc::Track::Fx { name: "fade-in", ... }` event)

**Cargo.toml dependencies**: `astraweave-cinematics = { path = "../astraweave-cinematics" }` (verified). The dev-only panel **does** use the canonical type, distinct from the editor's `cinematics_panel.rs` `CameraKeyframe`. **No renderer connection**: events are displayed as labels only; no `Renderer::tick_cinematics` invocation, no camera state update, no rendered output flow.

This is **the only canonical `CameraKey` usage in any UI surface** in the workspace — but it's a dev-only debug panel, not a feature for content authors. Editor's `cinematics_panel.rs` (the content-author-facing UI) uses the parallel `CameraKeyframe` instead.

### 1.G `examples/cinematics_timeline_demo` (timeline-construction demo, no rendering)

**File:line citations** (`examples/cinematics_timeline_demo/src/main.rs`, 43 lines total):

- Line 1: `use anyhow::Result;`
- Line 2: `use astraweave_cinematics::*;`
- Lines 5-21: Constructs `Timeline::new("cutscene", 5.0)` with one camera track containing two `CameraKey` keyframes (lines 8-13: `pos: (0.0, 1.5, 3.0), look_at: (0.0, 1.0, 0.0), fov_deg: 60.0`; lines 14-19: similar with `fov_deg: 55.0`)
- Lines 22-26: Audio track
- Lines 27-31: Fx track (`fade-in`)
- Lines 33-42: Constructs `Sequencer`, steps `0.5s` increments for 5 seconds, prints emitted events via `println!`

**Cargo.toml dependencies** (from `examples/cinematics_timeline_demo/Cargo.toml`):
```toml
astraweave-cinematics = { path = "../../astraweave-cinematics" }
```

Only `astraweave-cinematics`. No renderer, no window — pure data-layer demo. Demonstrates Timeline + Sequencer; does not exercise the rendering pipeline.

### 1.H Test infrastructure

- `astraweave-render/tests/coverage_booster_render.rs:137` — `renderer.tick_cinematics(0.1, &mut cam);` (no timeline loaded; tick is essentially a no-op)
- `astraweave-render/tests/coverage_booster_render.rs:6763` — `let events = renderer.tick_cinematics(0.016, &mut camera);` followed by `let _ = events;` (also no timeline loaded; coverage-only test)
- `astraweave-cinematics/benches/cinematics_benchmarks.rs:42` — defines a parallel `CameraKey` for benchmarking purposes (similar pattern to render's bench-mock camera per C.0 §1.H #29). Not in scope for C.7 consolidation.
- `astraweave-gameplay/src/cutscenes.rs:88-276` — 9 unit tests covering `CutsceneState::tick` for all `Cue` variants. C.7.A's migration must preserve these test passing (or update fixtures intentionally).

## 2. Cross-crate dependency mapping

### 2.A Current dependency graph (cinematics-related edges only)

```
                  ┌────────────────────────────────┐
                  │   astraweave-cinematics        │
                  │   (no astraweave-* deps)       │
                  └────┬───────────────┬────┬──────┘
                       │               │    │
       ┌───────────────┴──┐         ┌──┴────┴──────────┐
       │ astraweave-render│         │ astraweave-ui    │
       │ (tick_cinematics,│         │ (dev-only panel; │
       │  apply_camera_key│         │  no renderer hook│
       │  — 0 prod call)  │         │  — JSON load/save│
       └──────────────────┘         │   + Sequencer    │
                                    │   step into UI   │
                                    │   labels)        │
                                    └──────────────────┘
                       │
                       └─── examples/cinematics_timeline_demo
                            (data-layer demo; no rendering)

   ┌────────────────────────────┐
   │   astraweave-gameplay      │      [NO DEP ON CINEMATICS]
   │   (Cue::CameraTo,          │
   │   CutsceneState::tick      │
   │   returns (Vec3, f32, f32))│
   └──────────────┬─────────────┘
                  │
                  └─── examples/cutscene_render_demo
                       (uses gameplay + render + camera;
                       writes pos/yaw/pitch directly to
                       FreeFly fields; bypasses canonical
                       CameraKey + apply_camera_key entirely)

   ┌────────────────────────────┐
   │   tools/aw_editor          │      [NO DEP ON CINEMATICS]
   │   (CameraKeyframe parallel │
   │   type; UI editing only;   │
   │   no renderer flow; no     │
   │   conversion to/from       │
   │   canonical CameraKey)     │
   └────────────────────────────┘
```

### 2.B Production cinematics data flow (current)

```
authored Cue::CameraTo (gameplay::cutscenes::Timeline)
    ↓
CutsceneState::tick → (Option<(Vec3, f32, f32)>, Option<String>, bool)
    ↓
cutscene_render_demo:152-155
    self.camera.position = pos;
    self.camera.yaw = yaw;
    self.camera.pitch = pitch;
    ↓
renderer.update_view(&self.camera.to_render_view())
    ↓
GPU camera UBO
```

**The canonical path is unused**: `awc::Timeline` → `Sequencer::step` → `SequencerEvent::CameraKey` → `Renderer::tick_cinematics` → `apply_camera_key` → FreeFly update. Zero production callers exercise this path; only test coverage and `astraweave-ui`'s dev panel touch parts of it.

### 2.C Editor UI flow (current)

```
editor cinematics_panel UI (slider/numeric inputs)
    ↓
camera_keyframes: Vec<CameraKeyframe>
    ↓
get_interpolated_camera() → CameraKeyframe
    ↓
"Current Camera State" preview labels (lines 1278-1297)
    ↓
[DEAD END — no flow to a renderer or Sequencer]
```

The editor's keyframe data has no runtime consumer. UI-state-only.

### 2.D Target dependency graph (post-direction-A, hypothetical)

```
                  ┌────────────────────────────────┐
                  │   astraweave-cinematics        │
                  │   (no astraweave-* deps)       │
                  └─┬──────┬──────┬───────┬────────┘
                    │      │      │       │
      ┌─────────────┘      │      │       └──────────────┐
      │ astraweave-render  │      │       astraweave-gameplay
      │ (tick_cinematics   │      │       [NEW DEP via C.7.A;
      │  gets prod caller  │      │        Cue::CameraTo → look_at]
      │  via C.7.B)        │      │            │
      └────────────────────┘      │            │
                                  │            └─── examples/cutscene_render_demo
                                  │                 (rewrite via C.7.B; may add
                                  │                  cinematics dep)
                                  │
                              tools/aw_editor [NEW DEP via C.7.C;
                                              CameraKeyframe → CameraKey;
                                              roll handling per Q2.γ revisit]
                                  │
                                  └─── astraweave-ui (unchanged)
```

**No circular dependency risk** under direction A. `astraweave-cinematics` has zero `astraweave-*` dependencies, so any other crate can depend on it freely.

**Two new dependency edges** are added:
1. `astraweave-gameplay → astraweave-cinematics` (in C.7.A)
2. `tools/aw_editor → astraweave-cinematics` (in C.7.C)

One existing dependency edge becomes more meaningful: `astraweave-render → astraweave-cinematics` (gains production usage via C.7.B's rewrite of `cutscene_render_demo`).

## 3. Roll dormancy investigation (load-bearing for C.7.C scope; Q2.γ empirical question)

### 3.A Investigation methodology

Per Phase 4's execution plan:

1. `grep -rn "\.roll\b" --include="*.rs" tools/aw_editor/ astraweave-cinematics/` — every `.roll` field access in editor + canonical cinematics crates.
2. For each read site, inspect surrounding code to determine: does this read produce a runtime camera transformation?
3. Trace from the editor's roll slider write to any rendering path.

### 3.B Empirical findings

**Workspace-wide grep for `\.roll\b` matches related to `CameraKeyframe.roll`** (filtering out `spline_editor_panel`, `wave2_physics_spline_tests`, `wave2_cinematics_profiler_hierarchy_tests`'s default test):

| File:line | Site | Type |
|---|---|---|
| `tools/aw_editor/src/panels/cinematics_panel.rs:223` | `pub roll: f32` field declaration | declaration |
| `tools/aw_editor/src/panels/cinematics_panel.rs:233` | Default impl: `roll: 0.0` | initialization |
| `tools/aw_editor/src/panels/cinematics_panel.rs:606` | Constructor site (add-keyframe event handler): `roll: 0.0` | initialization |
| `tools/aw_editor/src/panels/cinematics_panel.rs:614` | Second constructor site: `roll: 0.0` | initialization |
| `tools/aw_editor/src/panels/cinematics_panel.rs:1068` | **UI slider write**: `Slider::new(&mut keyframe.roll, -180.0..=180.0).suffix("°")` | write |
| `tools/aw_editor/src/panels/cinematics_panel.rs:1296` | **UI label read**: `ui.label(format!("{:.1}°", keyframe.roll))` | display read |
| `tools/aw_editor/src/panels/cinematics_panel.rs:1417` | **Lerp**: `roll: b.roll + (a.roll - b.roll) * t` | interpolation read (produces another `CameraKeyframe`) |
| `tools/aw_editor/tests/wave2_cinematics_profiler_hierarchy_tests.rs:187` | `assert!((k.roll).abs() < 0.001)` | Default-impl test |

**Cross-crate search for `roll` as an outbound flow from CameraKeyframe** (i.e., is there any conversion of `keyframe.roll` to a runtime camera transformation?):

- `grep -rn "astraweave_cinematics\|use awc" tools/aw_editor/` returns **zero matches**. The editor does not import or use canonical cinematics types.
- `grep -rn "CameraKeyframe" --include="*.rs"` returns matches only inside `tools/aw_editor/src/panels/cinematics_panel.rs` and its test (`wave2_cinematics_profiler_hierarchy_tests.rs`).
- `astraweave_cinematics::CameraKey` has no `roll` field (verified at `astraweave-cinematics/src/lib.rs:237-242`).
- `astraweave_render::Renderer::apply_camera_key` has no roll-handling code (verified at `renderer.rs:3371-3381` — assigns only `position`, `yaw`, `pitch`, `fovy`).
- `FreeFly` (the canonical engine producer) has no roll field (verified at `astraweave-camera/src/freefly.rs:27-35`).
- `RenderView` (the canonical upload contract) has no roll field (verified at `astraweave-camera/src/render_view.rs:51-108`).

**There is no rendering pipeline that consumes `CameraKeyframe.roll`. The editor's keyframe-editing UI itself never emits data to any runtime consumer.**

### 3.C Verdict

**DORMANT** (high confidence).

`CameraKeyframe.roll` is UI state with **zero downstream runtime effect**. Roll values authored in the editor:

1. Are written via the slider at line 1068.
2. Are read for UI label display at line 1296.
3. Are read for lerp at line 1417 (producing another `CameraKeyframe` that goes back through display/lerp; never escapes the `CinematicsPanel`).

The dormancy is structural, not incidental: there is no API surface on `CinematicsPanel` that produces an output containing the roll value. `get_interpolated_camera() -> Option<CameraKeyframe>` returns the type, but the type's consumers are limited to the panel's own UI (preview labels).

### 3.D Implication for Q2.γ

The C.7 planning round's Q2.γ (CameraKey absorbs `roll`; CameraKeyframe retires) was framed as preservation-of-feature. **The empirical evidence contradicts that framing**: there is no feature to preserve. Absorbing `roll` into `CameraKey` is **feature addition**, not preservation.

This empirical finding should reframe the C.7.C planning round's options:

- **(γ-as-feature-addition)**: Add `roll: f32` to `CameraKey`; implement roll's rendering path (likely a post-multiplied rotation in `apply_camera_key`, requiring either a new `FreeFly.roll` field, a new `RenderView` post-rotation, or a different runtime model). Significant scope expansion vs the original γ framing.
- **(α-drop)**: Migrate `CameraKeyframe` to `CameraKey` without absorbing roll; drop the roll slider from the editor UI. Smaller scope; closes the parallel-type concern without expanding feature surface. **Treats the C.7 planning round's Q2.γ as a misjudgment given the empirical evidence.**
- **(γ-shim-only)**: Add `roll: f32` to `CameraKey` as a serialized-but-runtime-inert field; preserves authored data forward without committing to a rendering implementation. Smallest semantic change; defers the rendering decision to a future sub-phase.

This is **§6 Question #1** for the C.7.A planning round (per anti-drift constraint 16, C.7.0 does not lock the decision; the audit surfaces and characterizes).

## 4. Per-sub-phase scope estimates

Each estimate is **suggested**, not locked. The relevant sub-phase's planning round refines based on findings and any new information.

### 4.A C.7.A — `Cue::CameraTo` look_at migration

**Migration goal** (per planning round direction A + Q1.α): `Cue::CameraTo` migrates from yaw/pitch storage to look_at storage; the variant carries `CameraKey`-shaped data so that subsequent C.7.B's rewrite of `cutscene_render_demo` can flow `Cue::CameraTo` data through canonical `apply_camera_key`.

**Touch surface (estimated)**:

- `astraweave-gameplay/src/cutscenes.rs` — `Cue::CameraTo` field redesign (drop `yaw`, `pitch`; add `look_at: Vec3` and `fov_deg: f32`). `CutsceneState::tick` return-type evolution (the `(Option<(Vec3, f32, f32)>, …)` becomes either `(Option<awc::CameraKey>, …)` or a structured event enum). Unit test fixture updates (~9 tests in `#[cfg(test)] mod tests`).
- `astraweave-gameplay/Cargo.toml` — add `astraweave-cinematics = { path = "../astraweave-cinematics" }`.
- `examples/cutscene_render_demo/src/main.rs` — fixture updates for `Cue::CameraTo` construction (lines 47-58) since field names change. **Note**: full rewrite of the tick loop is C.7.B's scope; C.7.A only updates field-construction sites.

Estimated file count: **3 files** (1 gameplay source + 1 Cargo.toml + 1 example fixture).

**Latent issues affecting this sub-phase**:

- L.7.2 (see §5) — `CutsceneState::tick`'s return-type contract is awkward (three Optionals squashed into a tuple). C.7.A's redesign creates an opportunity to address this; the planning round may choose (a) preserve the tuple shape, just swap inner types; or (b) introduce a structured event enum.

**Suggested closure proof shape**:

- Structural-change verification: `grep -n "Cue::CameraTo {" --include="*.rs"` shows the new field names; `grep -n "yaw:\|pitch:" astraweave-gameplay/src/cutscenes.rs` returns zero CameraTo-related matches.
- Behavioral preservation: `cargo test -p astraweave-gameplay --lib cutscenes::tests` passes (post-fixture updates).
- New behavioral assertion: `Cue::CameraTo` constructed from `(pos, look_at, fov_deg, time)` and consumed by an updated `CutsceneState::tick` produces an `awc::CameraKey` (or equivalent canonical-shaped event) that downstream code can pass to `apply_camera_key`.

**Dependencies on prior sub-phases**: none; C.7.A is first.

### 4.B C.7.B — `cutscene_render_demo` rewrite

**Migration goal**: `cutscene_render_demo` uses `Renderer::tick_cinematics` (canonical path) instead of writing pos/yaw/pitch directly to FreeFly. The demo becomes the first production caller of `tick_cinematics`.

**Touch surface (estimated)**:

- `examples/cutscene_render_demo/src/main.rs` — `about_to_wait` tick loop rewrite (lines 137-165). The current `self.cs.tick(dt, &self.tl)` → field assignment pattern becomes `renderer.tick_cinematics(dt, &mut self.camera)` (after converting `Cue::CameraTo` data into a loaded `awc::Timeline` via `renderer.load_timeline_json` or a new `renderer.load_timeline(awc::Timeline)` API if needed).
- Possibly `examples/cutscene_render_demo/Cargo.toml` — add `astraweave-cinematics` if the demo invokes `awc::Timeline::new` directly. Alternative: `astraweave-gameplay` re-exports the necessary cinematics types and the demo continues to import only via `astraweave_gameplay::cutscenes::*`. C.7.B planning round decides.

Estimated file count: **1-2 files**.

**Latent issues**: none new.

**Suggested closure proof shape**:

- Structural verification: `renderer.tick_cinematics` is called from the demo's tick loop; the direct-field-assignment pattern at lines 152-156 is removed.
- Behavioral verification: the demo still produces the same rendered output (the two camera cues still move the camera to the same world positions). Pre/post comparison is judgment-only since the demo doesn't have a tested parity harness — C.8 will add cinematics-driven parity fixtures per Q4.β's deferral.
- L.5.18 status update: `tick_cinematics` gains its first production caller; the "zero production callers" framing becomes historical.

**Dependencies on prior sub-phases**: C.7.A must close first (Cue::CameraTo's look_at field must exist before the demo's tick loop can produce CameraKey-shaped data).

### 4.C C.7.C — `CameraKeyframe` retirement + roll absorption (scope subject to §3's dormancy finding)

**Migration goal**: editor's parallel `CameraKeyframe` type retires; `CinematicsPanel` operates on canonical `awc::CameraKey` directly. Roll handling depends on Q2.γ revisit per §3.D.

**Touch surface (estimated) — three scenarios per §3.D**:

| Scenario | Files | Description |
|---|---|---|
| γ-as-feature-addition | ~5-7 | astraweave-cinematics (add roll to CameraKey + lerp); astraweave-render (apply_camera_key writes roll); astraweave-camera (add roll to FreeFly or RenderView); editor panel (use CameraKey); editor Cargo.toml |
| α-drop | ~2-3 | editor panel (CameraKeyframe → CameraKey + drop roll slider); editor Cargo.toml |
| γ-shim-only | ~3-4 | astraweave-cinematics (add roll: f32 but no runtime use); editor panel (use CameraKey); editor Cargo.toml |

The planning round decides based on §3's empirical finding and Andrew's preference for feature scope.

**Latent issues**: depends on scenario.

**Suggested closure proof shape**: depends on scenario.

- α-drop: structural-deletion verification (`grep "CameraKeyframe"` returns zero workspace matches post-migration); behavioral preservation of editor panel's UI (sliders/labels still work for position/look_at/fov/time).
- γ-shim-only: structural-presence (`CameraKey` has roll field; serialization preserves it; runtime ignores it); editor UI behaves as before but writes the canonical type.
- γ-as-feature-addition: contract test for roll producing a view-axis rotation in some rendering path; significant integration testing.

**Dependencies on prior sub-phases**: independent of C.7.A/B; could execute first if planning round prefers. The dormancy finding (§3) is C.7.C's load-bearing input.

### 4.D C.7.D — `apply_camera_key` boundary hardening (L.5.17)

**Migration goal**: address `apply_camera_key`'s silent acceptance of degenerate inputs (look_at == pos, out-of-range fov_deg). Mirror C.6.F's FreeFly `sanitize()` pattern at the cinematics layer.

**Touch surface (estimated)**:

- `astraweave-cinematics/src/lib.rs` — add `CameraKey::sanitize(&mut self)` analogous to `FreeFly::sanitize()` from C.6.F (clamp fov_deg to is_typical_fov range or similar; handle look_at == pos).
- `astraweave-render/src/renderer.rs` — `apply_camera_key` calls `k.sanitize()` (or a clone-and-sanitize) before applying; or the renderer validates and skips degenerate keys with a tracing::warn. Design decision for C.7.D planning round.
- Contract tests for degenerate input handling (~3-5 tests).

Estimated file count: **2 source + 1 test file** = ~3 files.

**Latent issues**: closes L.5.17.

**Suggested closure proof shape**: contract test family analogous to C.6.F's 6 sanitize tests (look_at == pos handled; fov_deg below/above thresholds; idempotence; etc.).

**Dependencies on prior sub-phases**: independent of C.7.A/B/C. Could execute first if the planning round prefers, but the planning round may sequence after C.7.B so that the first production caller (`cutscene_render_demo` post-C.7.B) gets the hardened semantics from the start.

### 4.E C.7.E — documentation closeout

**Migration goal**: update `CAMERA_CONVENTIONS.md` §3 with C.7.A–D entries; update any Jekyll/mdBook cinematics-related pages.

**Touch surface (estimated)**:

- `docs/current/CAMERA_CONVENTIONS.md` — status log entries for C.7.A through C.7.D; migration table row updates (#10 cinematics rows marked CLOSED).
- Possibly `gh-pages/rendering.md` (if it documents cinematics — Phase 1 of C.7.E will inventory).
- Possibly `docs/src/core-systems/rendering.md` or a dedicated cinematics chapter (if one exists post-C.7.x).
- Possibly cross-references in this audit and the C.5 audit if the documentation surfaces want bidirectional links.

Estimated file count: **1-4 files** depending on existing cinematics documentation surface.

**Latent issues**: none.

**Suggested closure proof shape**: documentation-accuracy closure (per C.3.D's pattern). Grep for any stale references to old cinematics types (`CameraKeyframe`, parallel `Cue::CameraTo` yaw/pitch fields, etc.) returns zero matches in committed doc surfaces.

**Dependencies on prior sub-phases**: C.7.A through C.7.D must close first.

### 4.F Sequencing summary

```
C.7.A (Cue::CameraTo look_at)
     │
     └──► C.7.B (cutscene_render_demo rewrite)
                  │
                  └──► (optional) C.7.D (boundary hardening)
                            │
                            └──► C.7.E (docs closeout)

C.7.C (CameraKeyframe retirement)
     │
     └─► independent; can execute parallel or interleaved
         (sequence decided by planning round per Q2.γ outcome)
```

C.7.D could execute either before C.7.E (sequential) or in parallel with C.7.C (independent). The planning round decides per the chosen sequencing of C.7.C's scope (which affects timing).

## 5. Latent issues catalog (additive to L.5.x)

Issue IDs prefixed `L.7.<index>`. New issues C.7.0's empirical investigation surfaces. **Suggested resolutions are not locked; the relevant sub-phase planning round decides.**

### L.7.1 — `tick_cinematics` zero production callers reaffirmed

- **Description**: `astraweave_render::Renderer::tick_cinematics` (`astraweave-render/src/renderer.rs:3407`) has zero production callers. Only test-coverage callers exist at `astraweave-render/tests/coverage_booster_render.rs:137, 6763`, neither of which exercises an actual camera-key event flow (both invoke `tick_cinematics` against an empty timeline).
- **Confidence**: high (verified by grep).
- **Type**: dormant API.
- **Status post-C.7**: closes via C.7.B (cutscene_render_demo rewrite gains first production caller).
- **Suggested resolution**: C.7.B closes this implicitly; no separate action needed.

### L.7.2 — `CutsceneState::tick` return-type contract awkwardness

- **Description**: `astraweave-gameplay/src/cutscenes.rs:44-48` returns `(Option<(Vec3, f32, f32)>, Option<String>, bool)` — three Optionals squashed into a tuple. Caller (cutscene_render_demo:151) destructures with `let (cam, _title, _done) = ...`. Type evolution opportunity exists during C.7.A's redesign.
- **Location**: `astraweave-gameplay/src/cutscenes.rs:44-48`.
- **Confidence**: high (verified by inspection).
- **Type**: API design / structural inconsistency.
- **Suggested resolution**: optionally addressed in C.7.A's planning round (introduce a structured event enum like `CutsceneTickEvent::Camera(awc::CameraKey)`, `Title(String)`, `Done`, etc.) OR preserve the tuple shape and just swap inner types. Decision for the planning round.

### L.7.3 — `astraweave-cinematics` tuple-vs-Vec3 boundary

- **Description**: `astraweave-cinematics` uses `(f32, f32, f32)` for `pos` and `look_at` fields (no glam dependency by design — see `astraweave-cinematics/Cargo.toml`). `astraweave-gameplay::Cue::CameraTo` uses `glam::Vec3`. C.7.A's migration of `Cue::CameraTo` to look_at storage involves a boundary conversion that the planning round should address explicitly: either (a) keep `Cue::CameraTo`'s fields as `Vec3` and convert at the gameplay→cinematics boundary, or (b) keep them as tuples to match `CameraKey` directly (loses glam ergonomics in the gameplay layer).
- **Location**: `astraweave-cinematics/src/lib.rs:239-240` (tuple) vs `astraweave-gameplay/src/cutscenes.rs:8` (Vec3).
- **Confidence**: high (verified by inspection).
- **Type**: API design / convention mismatch.
- **Suggested resolution**: C.7.A planning round design decision; the prompt for C.7.A's launch should specify which boundary form `Cue::CameraTo` adopts.

### L.7.4 — Editor `CinematicsPanel` doesn't emit to a runtime consumer

- **Description**: The editor's `CinematicsPanel` (`tools/aw_editor/src/panels/cinematics_panel.rs`) holds `camera_keyframes: Vec<CameraKeyframe>` (line 489), provides full editing UI (slider/label/lerp), but has no API surface that emits this data to a runtime consumer. There is no save-to-disk path, no `to_timeline()` method, no event emission. The dormancy is structural: keyframe data is purely UI state.
- **Location**: `tools/aw_editor/src/panels/cinematics_panel.rs:218-1480` (the entire panel surface).
- **Confidence**: high (verified by grep — no production caller of `get_interpolated_camera` outside the panel itself; no `to_*` conversion methods on CameraKeyframe or CinematicsPanel that flow data out).
- **Type**: dormant UI subsystem.
- **Suggested resolution**: characterized as part of C.7.C's planning. The dormancy means C.7.C's migration of `CameraKeyframe` to `CameraKey` either (a) preserves the dormant pattern (rewires UI to operate on `awc::CameraKey`, no runtime flow) or (b) wires the panel to a runtime flow (e.g., produces an `awc::Timeline` that the editor's preview viewport consumes via `Renderer::tick_cinematics`). The latter is a feature addition; not necessarily in C.7.C scope.

### L.7.5 — `astraweave-ui` Simple Cinematics panel dev-only status

- **Description**: `astraweave-ui/src/panels.rs:238-363` provides a dev-only "Simple Cinematics" panel that uses canonical `awc::Timeline`/`Sequencer`/`CameraKey`. The panel has JSON load/save, sequencer step button, event display via UI labels — but no renderer connection. It's the only canonical `CameraKey` usage in any UI surface in the workspace, but it doesn't serve content authors. The editor's `cinematics_panel.rs` (the content-author UI) uses the parallel `CameraKeyframe` instead.
- **Location**: `astraweave-ui/src/panels.rs:238-363`.
- **Confidence**: high (verified by inspection).
- **Type**: parallel UI subsystems.
- **Suggested resolution**: out of C.7 scope. After C.7.C closes (CameraKeyframe retired), the editor's panel and `astraweave-ui`'s panel both operate on canonical `CameraKey`; potential consolidation is a future planning concern (post-C.9 cleanup queue candidate).

## 6. Open questions for the C.7.A planning round

The C.7.A planning round needs to lock decisions on the following before C.7.A's prompt drafts. Numbered for reference; "high importance" indicators noted where the decision is load-bearing for downstream sub-phases.

### Q1 — Roll dormancy / Q2.γ revisit (**HIGH IMPORTANCE; load-bearing for C.7.C scope**)

§3's empirical finding is **DORMANT**: `CameraKeyframe.roll` has zero downstream runtime consumers. The original Q2.γ framing (roll absorbed into CameraKey as preservation-of-feature) is contradicted by the evidence. Three options per §3.D:

- (γ-as-feature-addition) Add roll to CameraKey + implement runtime rendering path (~5-7 files, design decision about where roll lands in the camera pipeline)
- (α-drop) Migrate CameraKeyframe to CameraKey without roll; drop the editor's roll slider (~2-3 files)
- (γ-shim-only) Add `roll: f32` to CameraKey as a serialized-but-runtime-inert field; defer rendering decision (~3-4 files)

**Andrew's call**: which option, and if (γ-as-feature-addition), what's the runtime rendering path for roll?

### Q2 — `Cue::CameraTo` field-level migration design

What is the exact field shape of post-migration `Cue::CameraTo`?

- Field names: `look_at: Vec3` or `look_at: (f32, f32, f32)` (per L.7.3)?
- FOV field: add `fov_deg: f32` (matching CameraKey) or omit (and use a fixed default in the conversion)?
- The `time: f32` field's role: keep as cue duration, or migrate to absolute timestamp matching `CameraKey.t: Time`? (Note the semantic mismatch from §1.C.)

### Q3 — `CutsceneState::tick` return type evolution (L.7.2)

Per L.7.2, the current `(Option<(Vec3, f32, f32)>, Option<String>, bool)` return type is awkward. Options for C.7.A:

- (a) Preserve tuple shape; swap inner type: `(Option<awc::CameraKey>, Option<String>, bool)`
- (b) Introduce structured event enum: `CutsceneTickEvent::Camera(awc::CameraKey)`, `Title(String)`, `Done`, `Continue`, etc.

The planning round decides based on caller impact (currently one production caller in `cutscene_render_demo`) and API surface preferences.

### Q4 — `astraweave-cinematics` dependency direction in `astraweave-gameplay`

C.7.A adds `astraweave-cinematics` as a dependency of `astraweave-gameplay`. Confirm direction (gameplay → cinematics; no circular risk per §2.D). Any specific concerns about pulling cinematics into gameplay's dependency closure?

### Q5 — `examples/cutscene_render_demo`'s C.7.B dependency choice

For C.7.B's rewrite, the demo will use `Renderer::tick_cinematics`. Does the demo gain a direct `astraweave-cinematics` dependency, or does `astraweave-gameplay` re-export the necessary cinematics types so the demo's `use astraweave_gameplay::cutscenes::*;` continues to suffice?

### Q6 — `is_typical_fov` invocation in C.7.D

`CameraKey::is_typical_fov` (`astraweave-cinematics/src/lib.rs:286-288`) validates `30°..=120°` but is documentation-only (not invoked by `apply_camera_key`). For C.7.D's boundary hardening (L.5.17), does `apply_camera_key` invoke `is_typical_fov` (rejecting/clamping out-of-range FOVs) or use a different range (e.g., FreeFly's `sanitize` uses `10°..=170°` per C.6.F)? Consistency across the camera pipeline matters; the planning round picks the canonical range.

### Q7 — C.7.C / C.7.D sequencing

C.7.D is independent of C.7.A/B/C. Does the planning round sequence C.7.D before C.7.B (so the first production caller of `tick_cinematics` post-C.7.B gets hardened semantics from the start), in parallel with C.7.C, or after C.7.C?

### Q8 — L.7.4 (editor CinematicsPanel runtime-flow) and L.7.5 (`astraweave-ui` parallel panel) disposition

C.7.0's investigation surfaced two parallel-UI-subsystem findings. Are these in scope for C.7 (additional sub-phase or expanded C.7.C scope), or out of scope (post-C.9 cleanup queue)?

## 7. Methodology observations (§7.11 candidates for C.9 / E-closeout)

### 7.1 Audit-shaped-sub-phase pattern (third instance)

C.7.0 is the **third audit-shaped sub-phase** in the campaign:

- **C.0** — pre-campaign Phase 1 audit (before any consolidation work).
- **C.5** — mid-campaign Phase 2 audit (after renderer-side and editor-side first-pass; before second-pass execution).
- **C.7.0** — chapter-internal preliminary audit (before the cinematics-consolidation chapter's execution sub-phases).

Three concrete instances at distinct phase boundaries, each producing inventory documents that feed subsequent planning rounds. The pattern is now strong enough to codify as a named methodology pattern. C.9 / E-closeout can document this as a candidate pillar (perhaps "audit-shaped sub-phases at phase boundaries" or similar).

Distinct from execution sub-phases (byte-equivalence / structural-deletion / structural-rename / contract closures) and from documentation sub-phases (C.3.D — documentation-accuracy closure). Audit sub-phases produce audit-document-completeness closure verified by empirical citation.

### 7.2 Roll dormancy investigation reinforces "empirical verification of medium-confidence claims" discipline (C.5 §7.2 reinforcement)

C.5 §7.2 noted that medium-confidence findings inferred from absence-of-evidence at one site warrant grep verification across all sites before recording — the canonical example being C.5's 75% false-positive rate on C.0's pitch-clamp findings.

C.7.0 §3's roll dormancy investigation is a different application of the same discipline: the planning round's Q2.γ framing implicitly assumed roll was a real feature; empirical investigation (grep + read-site tracing) verified it's UI-only. The discipline that improves accuracy: **when a feature-affecting assumption is implicit in a planning-round decision, an empirical verification step before locking the execution scope is load-bearing**.

C.7.0's success here suggests the discipline should be standard for any planning round that makes feature-existence assumptions. Banked as §7.11 candidate.

### 7.3 Cross-crate consolidation pattern observation

C.7.0 is the first audit-shaped sub-phase that maps consolidation across **multiple crates** (cinematics, render, gameplay, aw_editor, cutscene_render_demo, ui). Prior audits (C.0, C.5) mostly scoped to a single crate's surface or a small handful. The cross-crate mapping in §2 surfaced a structural finding (the `astraweave-gameplay` → `astraweave-cinematics` dependency edge does not yet exist) that wouldn't have been visible from any single-crate inspection.

The pattern observation: **for multi-crate consolidation work, the preliminary audit should explicitly map dependency direction(s) and identify any required new edges before the execution sub-phases**. C.7.0 demonstrated this; future multi-crate consolidation campaigns may benefit from formalizing it as a methodology step.

### 7.4 Calibrated-language discipline (no new pattern; reinforcement)

C.7.0 continues the C.5 pattern of calibrated language throughout: "DORMANT" / "high confidence" / "verified" only for empirically-confirmed findings; "suggested" / "estimated" / "may" for items the planning round will lock. The discipline maps to Decision 5's α and is now standard across all audit-shaped sub-phases.

## 8. Cross-references

- **C.0 audit (pre-campaign Phase 1)**: `docs/audits/camera_system_architecture_audit_2026-05.md`. The original audit that inventoried pre-campaign camera state and identified `CameraKey` / `apply_camera_key` issues (#10 in C.0's inventory; §3.9 in C.0's per-implementation correctness audit).
- **C.5 audit (mid-campaign Phase 2)**: `docs/audits/camera_system_phase_2_audit_2026-05.md`. The Phase 2 audit that surfaced L.5.18 (three parallel cinematics systems) and L.5.17 (apply_camera_key boundary handling) — both of which C.7's chapter addresses.
- **C.6 closeout context**: commit `3897ba521`; the C.6 status log entry in `CAMERA_CONVENTIONS.md` documents post-C.6 state and identifies cinematics as the only remaining open §3 rows.
- **CAMERA_CONVENTIONS.md**: canonical conventions; §3 migration table tracks pre/post-migration status for every non-canonical convention. Cinematics-related rows (#10 family) remain open as of C.6 close; C.7.E will close them.
- **C.7.0 launch prompt** (this document's authorization): comprehensive enumeration of the audit's scope, decisions, and execution plan.

---

**Revision history**:

- v1.0 (2026-05-24) — initial publication. C.7.0 closure. Cinematics consolidation preliminary audit covering 6 crates + test infrastructure. Empirical roll dormancy finding (DORMANT, high confidence) reframes Q2.γ as feature-addition rather than feature-preservation. Cross-crate dependency map identifies two new dependency edges required for direction A (gameplay→cinematics for C.7.A; aw_editor→cinematics for C.7.C). 5 new latent issues catalogued (L.7.1–L.7.5). 8 open questions for the C.7.A planning round. Third audit-shaped sub-phase in the campaign; same closure proof family (audit-document-completeness via empirical citation) as C.0 and C.5.
