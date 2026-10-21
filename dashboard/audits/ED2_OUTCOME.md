# ED-2 — Editor visual-verification tooling: camera pinning + screenshot (outcome)

> **Beat:** ED-2 (editor tooling) · **Date:** 2026-07-25 · **Baseline commit:** `38c293106`
> **Why:** the editor could not pin a camera or capture a frame. T.2a had to build a private offscreen harness; T.2d's diagnosis stalled because it cannot instrument the editor's *live* render path. Every remaining visual gate (T.2b, T.2c re-judgment, T.2e, T.3, T.G) needs reproducible framing.
> **Anti-drift honoured:** editor tooling only — no terrain/water/shader/material work, no T.2d investigation, no new camera behaviours beyond the smoothing-target fix.

---

## 0. What shipped

| Concern | Result |
|---|---|
| 1 — `OrbitCamera` smoothing-target bug | **Fixed**, all setters swept, 4 regression tests **proven to fail pre-fix** |
| 2 — deterministic pin/restore | **`CameraState`** (complete state) + named `CameraStation`s persisted in `.editor_preferences.json`; restore reproduces the view **bit-identically** |
| 3 — screenshot | Captures the **live viewport texture** + a full-state sidecar; menu item and Ctrl+Shift+C share one code path; two captures from one restored station are **byte-identical** |
| 4 — A/B affordance | `restore_and_capture` + a **Capture ALL stations** command that queues one capture per frame |

**One state type, not two.** The pre-existing F1-F12 `CameraBookmark` was a partial copy of the camera (focal/distance/yaw/pitch, dropping fov/near/far) restored through the buggy setters. Rather than add a parallel named-slot system, `CameraBookmark` is now an alias for `CameraState` — so the hotkey bookmarks became complete and exact for free, and there is a single capture/apply surface.

---

## 1. Concern 1 — the smoothing-target bug

`smooth_update` interpolates four value/target pairs every frame. Two setters synced their target and two did not:

| pair | setter | before |
|---|---|---|
| `distance` / `zoom_target` | `set_distance` | correct |
| `focal_point` / `focal_point_target` | `set_focal_point` | correct |
| **`pitch` / `pitch_target`** | `set_pitch` | **value only — drifted back** |
| **`yaw` / `yaw_target`** | `set_yaw` | **value only — drifted back** |

Both fixed. `fovy`/`near`/`far`/`aspect` are **not** smoothed (no targets exist), confirmed by enumerating the struct — so the sweep is complete at four pairs, not "the two I noticed". The `sanitize` path (`camera.rs:~795-815`) already had the correct shape and was the model.

**The tests are proven to bite.** With the setters reverted to their pre-fix bodies:

```
test result: FAILED. 13 passed; 4 failed
  set_yaw_survives_smoothing                                  FAILED
  set_pitch_survives_smoothing                                FAILED
  restored_view_does_not_drift_when_the_smoother_runs         FAILED
  smooth_update_reports_not_animating_after_a_programmatic_set FAILED
```

The discrimination is exact: `capture_apply_round_trip_reproduces_the_view_matrix_exactly` and the serde/aspect tests still **pass** pre-fix, because they assert immediately without stepping the smoother. Only the drift-sensitive assertions fail. Fix restored → **17 passed; 0 failed**.

---

## 2. Concern 2 — pin / restore

### 2.1 The state, enumerated from the type

`CameraState` captures `focal_point`, `distance`, `yaw`, `pitch`, `fovy`, `near`, `far`, `aspect`.

Deliberate exclusions, documented at the type:
- **the four `*_target` fields** — smoothing scratch. Persisting them would let a half-finished animation be saved into a station; `apply_state` sets them equal to the value via the setters, which is the whole point of Concern 1.
- **`min/max_distance`, `min/max_pitch`** — rig constraints, not a viewpoint.

**`aspect` is captured but not applied.** It is owned by the current viewport size; forcing a stale value would distort the projection to something the user is not looking at. It is recorded so an A/B comparison can *detect* that two frames came from different window sizes — `CameraState::aspect_matches`. This is the one place where "restore exactly" and "restore usefully" conflict, and the split is deliberate.

### 2.2 Exactness

`capture_apply_round_trip_reproduces_the_view_matrix_exactly` asserts `to_cols_array()` equality — bit-identical, not epsilon-equal — because an A/B gate depends on it. `restored_view_does_not_drift_when_the_smoother_runs` then runs the smoother a full second and asserts the matrix is unchanged, tying Concern 1 to Concern 2.

### 2.3 Persistence

`EditorPreferences::camera_stations: Vec<CameraStation>` with `#[serde(default)]`, saved through the existing atomic tmp+rename `save()`. **No new persistence mechanism.** Two tests: a real file round-trip that also re-asserts view-matrix exactness after reload, and `preferences_without_camera_stations_still_load` for pre-ED-2 preference files.

### 2.4 UI

A **Camera** menu in the existing `MenuBar`/`MenuActionHandler` pattern: name field + Pin, then per-station **Go** / **Shot** / **X**, plus **Capture ALL stations** and **Capture viewport (Ctrl+Shift+C)**. Re-pinning an existing name overwrites it, which is what adjusting a station means.

---

## 3. Concern 3 — screenshot

### 3.1 It captures the live path, by construction

`ViewportWidget` renders into `self.render_texture` and hands that same texture to egui for display. The capture is serviced **immediately after that render call**, reading back that texture. There is no second render and no parallel path — which is the specific failure ED-2 exists to prevent (T.2d's offscreen harness reproduced a gradient but not the boundary).

`create_render_texture` gained `COPY_SRC`; `ViewportRenderer::capture_frame_png` does the aligned readback and PNG write.

### 3.2 Sidecar, not filename encoding

Each capture writes `<name>.camera.json` beside the PNG with the full `CameraState`. Chosen over filename encoding because the full state — including `aspect` — is what lets a later comparison assert two frames are comparable; a filename would drop it.

### 3.3 One path for hotkey and menu

Both `Ctrl+Shift+C` and the menu item call `ViewportWidget::request_capture`. There is no second capture routine to diverge — the 5.C lesson (a save hotkey that bypassed the fixed save path). `Ctrl+Z/Y/C/V/D` were already bound in this handler, so Shift disambiguates from Ctrl+C copy.

### 3.4 Determinism — measured, and the answer is byte-identical

`ed2_two_captures_from_one_restored_station_are_identical` restores a station, captures, **flies the camera 900 units away**, restores the same state, runs the smoother 30 frames, re-renders and re-captures:

```
[ed2] A: ...\station_a.png (33599 bytes)
[ed2] B: ...\station_b.png (33599 bytes)
[ed2] differing pixels: 0/307200 (0.0000%), max channel delta 0
test result: ok. 2 passed; 0 failed
```

**TAA does not prevent byte-identity here.** It is enabled by default, but a *static* restored camera settles, so no jitter survives into the capture. No disable-jitter escape hatch was needed; the test measures the delta rather than assuming, so if that ever changes the number is reported instead of a bare failure.

Note this is also Concern 1 asserted through pixels: had the restore drifted, leg B would differ.

---

## 4. Concern 4 — the A/B affordance

`ViewportWidget::restore_and_capture(state, path)` is the primitive. **Capture ALL stations** fills a queue drained **one per frame** — required, not conservative: the widget captures the texture rendered by the frame in which the request was pending, so issuing several in one frame would overwrite the same image.

Station names become file stems via `sanitize_station_filename`, so `captures/<station>.png` + `captures/<station>.camera.json` is the A/B artifact set.

**Scriptable route: not shipped, and here is the honest reason.** The beat says to prefer it *if cheap given the existing structure*. It is not: the editor's `headless.rs` does not construct a viewport widget or a GPU surface, so a config-driven capture list would need new plumbing to stand up a renderer outside `eframe`. Recommended as a follow-on. In the meantime an agent drives this through the same public API the menu uses (`capture_camera_state` / `restore_camera_state` / `restore_and_capture` / `request_capture`), which is exactly what `tests/ed2_capture.rs` does.

---

## 5. How to use this for a visual gate (read this first, future agents)

The T.2a pinned-station method, now without a private harness:

1. **Pin the stations once.** Fly to each viewpoint → **Camera → name → Pin**. They persist in `.editor_preferences.json`, so they survive restart and are shared by every later beat.
2. **Capture the "before" leg.** **Camera → Capture ALL stations** → `captures/<station>.png` + `.camera.json` each. Move that directory aside (e.g. `d:/tmp/<beat>_staging/before/`).
3. **Make the change** (one knob per leg — the T.2a discipline).
4. **Capture the "after" leg** the same way.
5. **Compare.** Frames from the same station are byte-comparable, so a per-pixel diff is meaningful; the sidecars let you assert both legs came from the same view (and the same `aspect`).

**What this buys that the offscreen harness did not:** these frames come from the editor's own render path, including anything the editor sets up that a standalone harness does not. That difference is exactly what stalled T.2d.

**Two cautions.** (a) A station is only as valid as the world it was pinned against — T.2d found a T.2a station that no longer framed its own biome after a classification change; re-survey when world generation changes. (b) `aspect` is not restored, so resizing the window between legs invalidates the comparison; the sidecar will show it.

---

## 6. Verification

| rung | result |
|---|---|
| `cargo fmt -p aw_editor` | clean |
| `cargo check --workspace` | **exit 0** |
| `cargo test -p aw_editor --lib` | **4035 passed; 0 failed; 5 ignored** |
| camera suite | **17 passed; 0 failed** (4021 filtered) |
| pre-fix proof | **4 failed / 13 passed** with the setters reverted |
| preferences suite | **12 passed; 0 failed** (4028 filtered) |
| `--test ed2_capture` (GPU) | **2 passed; 0 failed** — 0/307200 differing pixels |
| `cargo clippy -p aw_editor --lib` | no new findings on touched files (62 pre-existing warnings unchanged) |
| `cargo build -p aw_editor --profile release-fast` | **exit 0** |

### 6.1 What was NOT verified, stated plainly

**I did not drive the editor's GUI interactively** — I cannot click menu items in a running window. So the following are verified *by construction and by test*, not by a human-in-the-loop session:

- the menu items call the handler methods (compile-checked, single code path);
- the hotkey sets the same request (compile-checked);
- the capture reads the live `render_texture` (the code services the request inline after `renderer.render`);
- capture, restore-exactness, non-drift and byte-identity are all covered by the GPU test above, through the same `ViewportRenderer` methods the widget calls.

The editor **builds and links** at `release-fast`. What remains unproven is the click-through: pin via the menu, fly, restore via the menu, capture twice, restart, confirm the slot reloaded. **That is a five-minute manual pass and it is the right first use of this tooling** — §5 is the script for it.

---

## 7. Residue

- **Scriptable/headless capture** (§4) — the follow-on. `headless.rs` would need a GPU-backed viewport to host it.
- **Interactive click-through** (§6.1) — owed, cheap, and doubles as the tooling's first real use.
- `min_distance` clamping means a station pinned below the rig's minimum distance restores clamped. Not hit in practice (the default minimum is 0.02 m) and not worked around, but it is the one place `apply_state` is not literally exact.
