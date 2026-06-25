---
schema_version: 1
trace_id: input
title: "Input"
description: "Input"
primary_crate: astraweave-input
domain: gameplay
lifecycle_status: active
integration_status: wired
owns: [astraweave-input]
doc_version: "1.2"
last_verified_commit: a2474c5b7
---

# Architecture Trace: Input

## Metadata

| Field | Value |
|---|---|
| **System name** | Input |
| **Primary crates** | `astraweave-input` (sole production crate; pure facade over `winit` + `gilrs`) |
| **Document version** | 1.2 |
| **Last verified against commit** | `a2474c5b7` |
| **Last verified date** | 2026-05-12 |
| **Revision history** | 1.2 (2026-05-12): Deep investigation pass. **Discovered a major parallel implementation**: `tools/aw_editor/src/panels/input_bindings_panel.rs` (2511 lines) reinvents the entire input domain — its own `InputBindingAction`, `InputDevice`, `ActionCategory`, `BindingPreset`, `GamepadButton`, `KeyboardKey`, `MouseButton`, `ActionBinding`, `AxisBinding`, `InputTarget`, `InputBindingsPanel` types — with zero use of `astraweave_input` (the editor's `Cargo.toml` doesn't even declare the dep). Documented in §6 as a coexisting abstraction; added new §11 question. Also discovered the editor panel's `pending_actions` queue is never drained externally — confirmed via comprehensive grep showing `input_bindings_panel.take_actions()` has no call site. Enriched §11 Q1, Q2, Q5, Q8, Q9 with comprehensive evidence (commit dates, `git log -p --follow`, complete gilrs `Button` variant enumeration). Corrected §11 Q9: gilrs 0.10.10 has exactly 3 unmapped buttons (`C`, `Z`, `Mode`) — the prior "paddles" speculation was inaccurate (gilrs 0.10 does not define paddle buttons).<br><br>1.1 (2026-05-12): Verification pass. Resolved `[INFERRED]` on Miri workflow absence (direct workspace-grep confirmed). Recovered creation dates (2025-09-05 / 2026-02-09) for all six §7 Decision Log entries — every commit has an empty body, so "Alternatives considered" markers stand. Sharpened §2 Stage 1 and §11 `look_sensitivity` claims to acknowledge `bench_sensitivity_access` (benches/input_benchmarks.rs:153-160) does read the field while no production-path method does. Sharpened §9 hot-paths bench claim — `bench_process_window_event` is verified absent from `benches/input_benchmarks.rs:1-180`. Verified `gilrs::Button::LeftTrigger` ↔ bumper mapping via gilrs 0.10.10 source (`mapping/mod.rs:211-214`: `BTN_LT → leftshoulder`). |
| **Status** | Active |
| **Owner notes** | Pure facade crate. No ECS plugin, no system stage registration. Two workspace crates (`astraweave-gameplay`, `astraweave-ui`) declare the dependency in `Cargo.toml` but their `src/` directories contain **zero** `use astraweave_input` statements as of `a2474c5b7`. The crate has exactly one actual workspace consumer: `examples/ui_controls_demo`. |

---

## 1. Executive Summary

**What this system does:**
Maps raw input events (winit `WindowEvent` for keyboard/mouse; `gilrs` for gamepad; raw `Touch` events for virtual joystick) into a set of game-level `Action` enum values, exposes `is_down(Action)` / `just_pressed(Action)` / `move_axis` / `look_axis` queries, and persists user-configurable `BindingSet`s to JSON.

**Why it exists:**
Provides a single typed surface (`Action`, `Binding`, `BindingSet`, `InputManager`) so that game code does not have to pattern-match raw `KeyCode` / `MouseButton` / `gilrs::Button` values inline; supports rebindable controls across keyboard, mouse, gamepad, and touch.

**Where it primarily lives:**
- `astraweave-input/src/actions.rs` — `Action` enum (23 variants), `InputContext` (Gameplay / UI), `Axis2` 2D-vector helper (lines 1-318 production; lines 326-573 tests)
- `astraweave-input/src/bindings.rs` — `GamepadButton`, `AxisKind`, `Binding`, `AxisBinding`, `BindingSet` (default keyboard map, lines 488-672), serde-derived persistence types
- `astraweave-input/src/manager.rs` — `InputManager` (event handling, gamepad polling, touch joystick, state queries)
- `astraweave-input/src/save.rs` — `save_bindings(path, &BindingSet)` and `load_bindings(path) -> Option<BindingSet>`
- `astraweave-input/src/lib.rs` — flat re-export surface (`pub use actions::*; pub use bindings::*; pub use manager::*; pub use save::*;`)

**Status note:**
The actual API surface is small (~30 public types/functions) and stable. Significant divergence exists between (a) the implemented crate and (b) two external doc files (`docs/src/core-systems/input.md`, `docs/src/api/audio.md` and friends) that reference types like `InputSystem`, `InputConfig`, `ActionMap`, `BindingRecorder`, `BindingProfile`, `ContextPriority`, `InputBuffer`, `InputPredictor`, `InputRecorder`, etc. — **none** of which exist in `lib.rs`'s re-exports. Treat external docs as aspirational, not as API truth (see §6).

---

## 2. Authoritative Pipeline

```text
[Per-frame caller — currently only examples/ui_controls_demo]
    │
    │ App::new() → InputManager::new(InputContext, BindingSet::default())
    │              │
    │              ├─ Gilrs::new().ok() (gamepad backend; None if unavailable)
    │              ├─ pressed: HashSet<Action> = {}
    │              ├─ just_pressed: HashSet<Action> = {}
    │              ├─ move_axis = look_axis = Axis2::default()
    │              ├─ look_sensitivity = 0.12
    │              └─ touch_active = false, touch_origin = touch_current = None
    ▼

[Window event loop tick — winit dispatch]
    │
    ▼
WindowEvent::KeyboardInput   ──┐
WindowEvent::MouseInput       ─┼──► InputManager::process_window_event(&event)   (manager.rs:72-147)
WindowEvent::Touch            ──┘    │
                                     ├─ Keyboard: iterate bindings.actions (HashMap), find every Action
                                     │   whose Binding.key == Some(code); for each: set_action(a, pressed)
                                     ├─ Mouse: same pattern with Binding.mouse == Some(button)
                                     └─ Touch:
                                         · Started   → touch_origin = touch_current = location
                                         · Moved     → touch_current = location
                                         · Ended/Cancelled → reset, move_axis = Axis2::default()

[Per-frame caller polls gamepad + touch joystick]
    │
    ▼
InputManager::poll_gamepads()   (manager.rs:149-175)
    │
    ├─ Drain all gilrs events into Vec<Event> (borrow-conflict avoidance)
    │   For each event:
    │     ButtonPressed/Released  → handle_button(button, down)
    │     AxisChanged             → handle_axis(axis, value)
    │
    └─ Virtual joystick from touch (if touch_active):
         delta = (touch_current - touch_origin) / 80.0    // pixels → normalized
         move_axis.x = delta.x.clamp(-1.0, 1.0)
         move_axis.y = (-delta.y).clamp(-1.0, 1.0)         // Y screen-axis inverted

[Action / axis state update — internal]
    │
    ├─ handle_button(gilrs::Button, down)   (manager.rs:177-218)
    │    │ Maps gilrs::Button → GamepadButton (16 variants); unmapped buttons silently ignored
    │    │ Iterates bindings.actions, finds matches by Binding.gamepad == Some(gb)
    │    └─ For each match: set_action(action, down)
    │
    ├─ handle_axis(gilrs::Axis, val)        (manager.rs:220-243)
    │    │ Reads bindings.move_axes (LeftX/LeftY) and bindings.look_axes (RightX/RightY)
    │    │ Applies AxisBinding inline (invert then deadzone check, no normalization)
    │    └─ Writes to move_axis.x/.y or look_axis.x/.y; LT/RT and unknown axes ignored
    │
    └─ set_action(Action, down)             (manager.rs:245-254)
         · down: if not already pressed → insert into just_pressed; insert into pressed
         · up: remove from pressed (just_pressed is NOT touched on release)

[Caller queries state]
    │
    ├─ is_down(Action) → bool                  ── pressed.contains(&a)
    ├─ just_pressed(Action) → bool             ── just_pressed.contains(&a)
    ├─ move_axis: Axis2 (pub field)            ── current movement axis
    └─ look_axis: Axis2 (pub field)            ── current look axis

[Caller signals end of frame]
    │
    ▼
InputManager::clear_frame()   (manager.rs:68-70)
    │
    └─ just_pressed.clear()   // pressed is untouched (sticky)

──────────────────────────────────────────────────────────────────────
Persistence path:
    │
    ├─ save_bindings(path, &BindingSet) -> Result<()>   (save.rs:5-14)
    │     · serde_json::to_string_pretty
    │     · fs::create_dir_all on parent
    │     · fs::write
    │
    └─ load_bindings(path) -> Option<BindingSet>        (save.rs:16-19)
          · fs::read_to_string(path).ok()
          · serde_json::from_str.ok()
          · Errors are swallowed — caller gets None for either I/O or parse failure
```

### Stage-by-stage detail

#### Stage 1: Construction (`manager.rs:36-53`)
**Role:** Build an `InputManager` with an initial `InputContext`, a `BindingSet` (typically `BindingSet::default()`), an attempt at `Gilrs::new()` for gamepad support, and zero-initialized axes / state sets.
**Inputs:** `InputContext`, `BindingSet`.
**Outputs:** A live `InputManager`.
**Notes:** `Gilrs::new().ok()` discards any error and stores `None` — when gamepad backend init fails, the manager runs without gamepad but does not surface why. `look_sensitivity` defaults to `0.12` (`manager.rs:46`); this field is `pub` and is not read by any method in `astraweave-input/src/` (the bench `bench_sensitivity_access` at `benches/input_benchmarks.rs:153-160` reads it once for field-access timing but does not use it in any computation). Callers are expected to multiply by it at the query site.

#### Stage 2: Window event ingestion (`manager.rs:72-147`)
**Role:** Convert one `winit::WindowEvent` into zero-or-more action updates, or update the touch joystick state machine.
**Inputs:** `&WindowEvent`.
**Outputs:** Mutations to `pressed`, `just_pressed`, and touch state.
**Notes:** Keyboard and mouse branches each iterate over the entire `bindings.actions` HashMap looking for matches — O(N) per event where N = number of action bindings (`bindings.rs:415`). For the default `BindingSet` (21 bound actions), this is trivially fast. The iterator is collected into a `Vec<_>` first (lines 83-94, 101-112) to avoid borrow-checker conflicts before calling `set_action` (`&mut self`).

#### Stage 3: Gamepad polling (`manager.rs:149-175`)
**Role:** Once per caller-driven tick, drain the gilrs event queue and dispatch button / axis events; also update the virtual touch joystick.
**Inputs:** None (reads from `self.gilrs`, `self.touch_origin`, `self.touch_current`).
**Outputs:** Same as Stage 2 plus axis writes.
**Notes:** Events are collected into a `Vec<gilrs::Event>` before processing (lines 151-156) to side-step borrowing `self` across the iterator. If `self.gilrs` is `None`, the loop is skipped entirely (no gamepad backend = no events). The touch joystick step runs unconditionally on every call regardless of gamepad availability.

#### Stage 4: Gamepad button mapping (`manager.rs:177-218`)
**Role:** Translate `gilrs::Button` (which has ~24 variants) to the smaller `GamepadButton` enum (16 variants).
**Inputs:** `gilrs::Button`, `down: bool`.
**Outputs:** Calls `set_action` for every action bound to the mapped `GamepadButton`.
**Notes:** Returns silently for any `gilrs::Button` not in the mapping closure (e.g., `Button::C`, `Button::Z`, paddle buttons). Test `test_handle_button_unknown_ignored` (`manager.rs:486-493`) asserts this. Notable mapping inversion: `gilrs::Button::LeftTrigger` → `GamepadButton::L2` and `LeftTrigger2` → `L1` — that is, **gilrs's "Trigger"/"Trigger2" map to AstraWeave's "L2"/"L1"** respectively, swapping the conventional sense. The trigger/bumper test (`manager.rs:450-465`) verifies this inversion is intentional and stable.

#### Stage 5: Axis handling (`manager.rs:220-243`)
**Role:** Apply `AxisBinding` (deadzone + invert) to a raw gamepad axis value and store into `move_axis` or `look_axis`.
**Inputs:** `gilrs::Axis`, `f32` value in `[-1.0, 1.0]`.
**Outputs:** Updated `self.move_axis.{x,y}` or `self.look_axis.{x,y}`.
**Notes:** The axis-binding application here (`manager.rs:222-232`) inlines a **different formula** than `AxisBinding::apply` (`bindings.rs:387-400`). `AxisBinding::apply` normalizes after the deadzone: `(|v| - deadzone) / (1 - deadzone) * sign(v)`. `InputManager::handle_axis`'s inline `apply` closure does not normalize: if `|v| < deadzone`, returns 0; otherwise returns the original (signed) value. This is a divergence the caller cannot toggle. Triggers (`LT`/`RT`) and `gilrs::Axis::Unknown` fall through the match unhandled (line 241 `_ => {}`). Test `test_handle_axis_unknown_ignored` (`manager.rs:550-556`) confirms.

#### Stage 6: Touch virtual joystick (`manager.rs:170-174`)
**Role:** Convert a touch drag into a normalized 2D axis for the `move_axis`.
**Inputs:** `touch_origin` and `touch_current` (set by `WindowEvent::Touch` in Stage 2).
**Outputs:** `move_axis.x` and `move_axis.y`.
**Notes:** Fixed scale factor: 80 pixels = full deflection (line 171: `let delta = (c - o) / 80.0`). Y is inverted vs. screen coordinates so up-on-screen produces positive `move_axis.y`. Output clamped to `[-1, 1]` on each axis. **Only `move_axis` is touch-driven — `look_axis` has no touch path.** Two-finger or multi-touch scenarios use the first touch ID and ignore the rest (`manager.rs:131`: `self.touch_id == Some(*id)`).

#### Stage 7: State sets and frame boundary (`manager.rs:245-254, 68-70`)
**Role:** Maintain the `pressed` / `just_pressed` HashSet pair.
**Notes:** `just_pressed` only fires on the press edge (line 247: `if !self.pressed.contains(&a)`), so holding a key emits one just-press only; consecutive frames see only `is_down`. `clear_frame()` clears `just_pressed` but never touches `pressed` — `pressed` only goes down via an explicit release event (line 252). If the consumer forgets `clear_frame()`, `just_pressed` accumulates and never decays.

#### Stage 8: Persistence (`save.rs:5-19`)
**Role:** Persist a `BindingSet` to disk as JSON, and load it back.
**Inputs:** `path: &str`, `&BindingSet` (for save); `path: &str` (for load).
**Outputs:** `Result<()>` (save); `Option<BindingSet>` (load).
**Notes:** Save uses `serde_json::to_string_pretty` and `fs::create_dir_all` on the parent before writing. Load uses `.ok()` on both `fs::read_to_string` and `serde_json::from_str` — every error mode (file missing, permissions, corrupt JSON, empty file) collapses to `None`. No error type is exposed; callers cannot distinguish "no saved bindings yet" from "saved bindings are corrupt". Tests at `save.rs:81-156` cover the missing-file, invalid-JSON, and empty-file cases — all expect `None`.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Action** | Game-level intent (23 variants spanning movement, attack, ability, UI toggle, UI nav). Decoupled from physical input keys. | `actions.rs:46-75`; bound via `BindingSet.actions` |
| **InputContext** | Coarse mode label (`Gameplay` or `UI`). Each `Action` self-declares its context via `Action::context()` (`actions.rs:163-169`). | `actions.rs:5-8`; stored on `InputManager.context` |
| **Axis2** | `{x, y}` 2D vector with helpers (length, normalize, deadzone, clamped, angle). | `actions.rs:234-318` |
| **GamepadButton** | 16-variant gamepad button enum (face S/E/W/N, L1/R1/L2/R2, Select/Start, LStick/RStick, DPadUp/Down/Left/Right). | `bindings.rs:8-138` |
| **AxisKind** | 6-variant gamepad axis enum (LeftX/Y, RightX/Y, LT, RT). | `bindings.rs:148-238` |
| **Binding** | One Action's binding: `{ key: Option<KeyCode>, mouse: Option<MouseButton>, gamepad: Option<GamepadButton> }`. Multiple inputs may bind to the same Action. | `bindings.rs:246-251` |
| **AxisBinding** | One stick or trigger's binding: `{ axis: AxisKind, invert: bool, deadzone: f32 }`. Default deadzone = 0.15. | `bindings.rs:346-411` |
| **BindingSet** | Full input config: `actions: HashMap<Action, Binding>` + `move_axes: (AxisBinding, AxisBinding)` + `look_axes: (AxisBinding, AxisBinding)`. Default supplies WASD/mouse/keyboard bindings (`bindings.rs:488-672`). | `bindings.rs:413-418` |
| **InputManager** | Runtime input state: holds context, bindings, pressed/just-pressed sets, axes, look sensitivity, gilrs handle, and touch state. The class consumers actually drive. | `manager.rs:11-34` |
| **Pressed / just-pressed** | Two HashSet<Action>: `pressed` is "currently down", `just_pressed` is "down-edge this frame (until `clear_frame` is called)". | `manager.rs:16-17, 60-70` |
| **Virtual joystick** | A move_axis source backed by touch drag distance from a touch's start position. 80 pixels = full deflection, Y screen-inverted. | `manager.rs:30-33, 169-174` |

### Terms to NOT confuse

- **`InputContext` (the field on `InputManager`) vs. `Action::context()` (the method):** `InputManager.context` is a stored label set via `set_context` (`manager.rs:55-57`). `Action::context()` is a pure function returning a category for an action. `InputManager` **never reads its own `context` field** in event handling or state queries (`manager.rs` body) — input is dispatched into `pressed`/`just_pressed` regardless of context. Filtering by context is the caller's responsibility. See §6 trap.

- **`Binding.gamepad: Option<GamepadButton>` vs. `BindingSet.move_axes`/`look_axes`:** Buttons are bound per-action via `Binding`. Sticks and triggers are bound globally per-axis-pair via the four `AxisBinding` fields on `BindingSet`. Stick movements never appear as `Action`s — they only update `move_axis`/`look_axis`.

- **`AxisBinding::apply` (the method) vs. the inline closure in `InputManager::handle_axis`:** Different formulas. `apply` (`bindings.rs:388-400`) does `(|v| - deadzone) / (1 - deadzone) * sign(v)` (normalized linear ramp out of deadzone). `handle_axis`'s closure (`manager.rs:222-232`) returns `0` inside deadzone or the raw (signed) `v` outside. The `apply` method is **not invoked from the gamepad axis hot path** as of `a2474c5b7`; it is exercised by tests at `bindings.rs:969-990`.

- **`pressed` (sticky) vs. `just_pressed` (edge):** `pressed` only goes down on an explicit release event (`set_action(a, false)` → `pressed.remove`). `just_pressed` is reset to empty by `clear_frame()`. If the consumer does not call `clear_frame()` once per frame, `just_pressed` accumulates and is indistinguishable from `pressed`.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| winit (0.30 per `Cargo.toml:12`) | `WindowEvent::{KeyboardInput, MouseInput, Touch}` → `InputManager::process_window_event(&WindowEvent)` | Raw `KeyCode`, `MouseButton`, `Touch` (with `id`/`location`/`phase`) | Caller's event loop forwards every window event. The manager filters internally; unknown event variants fall through `_ => {}` at `manager.rs:145`. |
| gilrs (per `Cargo.toml:14`) | `Gilrs::next_event()` drained inside `InputManager::poll_gamepads()` | `gilrs::EventType::{ButtonPressed, ButtonReleased, AxisChanged}` | `Gilrs::new()` failures are silently absorbed (`manager.rs:38`: `let gilrs = Gilrs::new().ok();`). When `None`, gamepad input is unavailable but the manager continues to work. |
| Filesystem (JSON) | `load_bindings(path) -> Option<BindingSet>` (`save.rs:16-19`) | `BindingSet` deserialized from JSON | Single function call site is expected to be called once at startup; not enforced. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `examples/ui_controls_demo` | `InputManager::process_window_event`, `clear_frame`, `poll_gamepads` (calls at `main.rs:176, 234, 235`) | Forwards events into manager and ticks each frame | **Critical observation:** workspace grep for `is_down\|just_pressed\|move_axis\|look_axis` against `examples/ui_controls_demo/src/main.rs` returns **zero** hits inside the demo's logic. The demo feeds events to the manager but never reads back the manager's state for gameplay decisions — UI toggle keys are matched via direct `match code { KeyCode::KeyI => … }` blocks (`main.rs:196-215`). The output state is currently dormant. |
| Filesystem (JSON) | `save_bindings(path, &BindingSet) -> Result<()>` (`save.rs:5-14`) | `BindingSet` serialized via `serde_json::to_string_pretty` | No call site exists in any production crate as of `a2474c5b7` (workspace grep `save_bindings` returns matches only in this crate's own tests at `save.rs:34, 47, 60, 73, 105, 132, 137`). |

### Declared-but-unused dependencies

- **`astraweave-gameplay/Cargo.toml:21`** — `astraweave-input = { path = "../astraweave-input" }` is declared, but `grep -rn "use astraweave_input\|astraweave_input::" astraweave-gameplay/src` (2026-05-12) returns **zero** matches. The dependency is unused as of `a2474c5b7`.
- **`astraweave-ui/Cargo.toml:22`** — Same pattern: declared, unused. Zero `use astraweave_input` in `astraweave-ui/src`.

### Documentation references with no code backing (aspirational docs)

- **`docs/src/core-systems/input.md`** references many module paths that do not exist in the crate: `InputSystem`, `InputConfig`, `action::ActionType`, `mapping::ActionMap`, `mapping::InputBinding`, `device::{Key, MouseButton, GamepadButton, GamepadAxis}`, `modifier::{InputModifier, ModifierKey}`, `composite::Vec2Input`, `rebinding::BindingRecorder`, `persistence::BindingProfile`, `context::ContextPriority`, `gamepad::{Gamepad, GamepadEvent, VibrationEffect, VibrationDuration}`, `player::PlayerInput`, `replay::{InputRecorder, InputRecording, InputPlayer}`, `buffer::InputBuffer`, `prediction::InputPredictor`, `touch::{TouchEvent, TouchPhase}`. The actual `lib.rs` only re-exports from four modules: `actions`, `bindings`, `manager`, `save`.
- **`docs/src/reference/crates.md:159`** suggests `use astraweave_input::prelude::*;` — no `prelude` module exists.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-input/src/lib.rs` | Crate facade: `pub mod` declarations + flat re-exports | Active | `#![forbid(unsafe_code)]` (line 1). 76 lines total including inline serde tests. |
| `astraweave-input/src/actions.rs` | `Action` enum (23 variants), `InputContext`, `Axis2` | Active | 573 lines; ~232 production + 341 of `#[cfg(test)] mod tests` (30 tests). |
| `astraweave-input/src/bindings.rs` | `GamepadButton`, `AxisKind`, `Binding`, `AxisBinding`, `BindingSet` and its default keymap | Active | 1098 lines; ~487 production + 611 tests (51 tests). Default `BindingSet` provides WASD/mouse/keyboard map for all 21 default-bound actions. |
| `astraweave-input/src/manager.rs` | `InputManager` — event handling, gamepad polling, touch joystick, state queries | Active | 694 lines: ~279 production + 415 of `#[cfg(test)] mod manager_internal_tests` (31 tests). Includes 4 `#[cfg(test)] pub(crate) fn test_*` accessors at lines 257-277 for internal-state manipulation by tests. |
| `astraweave-input/src/save.rs` | `save_bindings`, `load_bindings` | Active | 157 lines: 19 production + 138 tests (9 tests). |
| `astraweave-input/src/manager_tests.rs` | Comprehensive `InputManager` test suite (Week 5 Day 1-3 campaign) | Active (tests) | 1112 lines, all `#[cfg(test)] mod input_manager_tests` (55 tests). Includes Day-1 unit tests, Day-2 stress tests, Day-2 edge cases, Day-2 save.rs tests. |
| `astraweave-input/tests/mutation_resistant_comprehensive_tests.rs` | Mutation-resistance harness | Active (tests) | 1023 lines (125 tests). Targets cargo-mutants survivor classes. |
| `astraweave-input/benches/input_benchmarks.rs` | Criterion benchmarks | Active | 180 lines, 14 benchmarks. Covers binding creation/serde, BindingSet construction, and likely InputManager hot paths. |
| `astraweave-input/README.md` | One-screen description of the crate's modules and deps | Active | 26 lines. The README's module list matches the actual `src/` exactly — unlike the aspirational `docs/src/core-systems/input.md`. |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit freely with care.
- **Active (tests)**: Carries no runtime weight but exercises invariants.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| `docs/src/core-systems/input.md` aspirational API (`InputSystem`, `InputConfig`, `mapping::ActionMap`, `rebinding::BindingRecorder`, `replay::InputRecorder`, `buffer::InputBuffer`, etc.) | `docs/src/core-systems/input.md`, `docs/src/reference/crates.md`, and external doc tree | Reference-only, code-absent | Origin: `git log --diff-filter=A` traces `docs/src/core-systems/input.md` to commit `28bc94f21` (2025-09-08, "Create comprehensive bespoke wiki with 51-section documentation structure (#34)") authored by `Copilot <198982749+Copilot@users.noreply.github.com>` — the same bulk-doc commit that introduced the aspirational audio docs. The same commit added ~80 doc files in one sweep with no corresponding code changes. The docs describe an imagined input subsystem, not the actual one. |
| Inline gamepad-axis formula in `InputManager::handle_axis` vs. `AxisBinding::apply` | `manager.rs:222-232` vs. `bindings.rs:387-400` | Active divergence | Two different applications of `(invert, deadzone)`: the manager's inline closure does not normalize past the deadzone; the `AxisBinding::apply` method does (subtract deadzone, scale by `1/(1-deadzone)`). The `apply` method is unit-tested (`bindings.rs:969-990`) but is not called from the production gamepad axis path. |
| Declared-but-unused workspace deps on `astraweave-input` | `astraweave-gameplay/Cargo.toml:21`, `astraweave-ui/Cargo.toml:22` | Unused | Both `Cargo.toml`s pull in the crate; neither `src/` directory imports anything from it. Compiles fine (unused dep is a warning at most). |
| `gilrs::Button::LeftTrigger` mapped to `GamepadButton::L2`, `LeftTrigger2` → `L1` | `manager.rs:185-188` | Intentional (test-locked) | gilrs's naming convention swaps "Trigger" / "Trigger2" relative to AstraWeave's `L1` / `L2`. `test_handle_button_triggers` (`manager.rs:450-456`) and `test_handle_button_bumpers` (`manager.rs:458-465`) lock the convention in: `LeftTrigger` (gilrs) maps to `L2` (AstraWeave), which game code expects to be Sprint by default. |
| **Editor `InputBindingsPanel` — parallel implementation of the entire input domain** | `tools/aw_editor/src/panels/input_bindings_panel.rs` (2511 lines) | Standalone, no shared types | The editor defines its own `InputBindingAction`, `InputDevice` (lines 51-), `ActionCategory` (101-), `BindingPreset` (169-), `GamepadButton` (231-), `KeyboardKey` (327-), `MouseButton` (672-), `ActionBinding` (714-), `AxisBinding` (744-), `BindingConflict` (768-), `InputBindingsPanel` (826-), `InputTarget` (932-) — 13 types total, none of which share definitions with `astraweave-input`. `tools/aw_editor/Cargo.toml` does not declare `astraweave-input` as a dependency. The panel renders in the dock view (`dock_panels.rs:215`, `tab_viewer/mod.rs:7736` both call `.show(ui)`) but its `pending_actions: Vec<InputBindingAction>` queue (`input_bindings_panel.rs:866`) is never drained externally — workspace grep for `input_bindings_panel.take_actions()` outside the panel returns zero hits. Effects: clicking "Save as Custom" / "Import" / "Export" / "Reset to Default" buttons (`input_bindings_panel.rs:1663-1675`) queues `InputBindingAction::SaveBindings` / `LoadBindings` / `ResetToDefaults` (lines 1685-1697), but no code reads them. |
| Dormant `AxisBinding::apply` (bindings.rs:387-400) | `bindings.rs:387-400` (method); `bindings.rs:969-990` (tests) | Reachable via API but not used in production path | The method is on the public surface (`pub fn apply`) but is called only from inline tests at `bindings.rs:972-988`. Workspace-wide grep for `\.apply(` against `astraweave-input/src` shows zero non-test call sites. The gamepad axis hot path uses a different inline closure (`manager.rs:222-232`). |

### Naming collisions

- **`MouseButton` (winit) vs. various aspirational `device::MouseButton` mentions in `docs/src/core-systems/input.md:144`:** Only winit's `MouseButton` exists in the codebase; `astraweave_input::device::MouseButton` is a phantom reference in aspirational docs.
- **`GamepadButton` (in this crate, `bindings.rs:10`) vs. `gilrs::Button` (in the dep):** Two distinct enums with overlapping variant names but different membership; the crate intentionally re-narrows gilrs's button set to 16 variants. `handle_button` (`manager.rs:177-218`) translates between them and drops unsupported variants.

### Known cognitive traps

- **Trap:** `InputContext` is stored on the manager but **never read** for input gating.
  - **Why it's confusing:** `set_context(InputContext::UI)` looks like it would suppress gameplay actions or scope events.
  - **What's actually true:** `manager.rs:55-57` only writes to the field. None of `process_window_event`, `poll_gamepads`, `handle_button`, `handle_axis`, `is_down`, or `just_pressed` reads `self.context`. Every event updates every bound action regardless of mode. Callers wanting context filtering must check `Action::context()` (`actions.rs:163-169`) at the query site themselves.

- **Trap:** `clear_frame()` is caller-driven. Skipping it makes `just_pressed` monotonic until the action is released.
  - **Why it's confusing:** "Just pressed" sounds frame-local by name.
  - **What's actually true:** Frame boundary is defined by the caller calling `clear_frame()` (`manager.rs:68-70`). If a caller drives `poll_gamepads()` per redraw but forgets `clear_frame()`, every press persists in `just_pressed` until the corresponding release event also runs through the release branch (`set_action(_, false)` does NOT add to `just_pressed`).

- **Trap:** `load_bindings(path)` returning `None` is ambiguous — could be missing file, permissions error, corrupt JSON, or empty file.
  - **Why it's confusing:** Callers cannot distinguish "first run, fall back to defaults" from "saved bindings are corrupt".
  - **What's actually true:** `save.rs:16-19` uses `.ok()` on both `fs::read_to_string` and `serde_json::from_str`. All four failure modes are tested (`save.rs:81-156`) — every one returns `None`.

- **Trap:** The dual axis formula (Stage 5 above) means a developer who customizes `AxisBinding.deadzone` and tests via `AxisBinding::apply` will see different behavior than the actual gamepad input path.
  - **Why it's confusing:** Both methods live in the same module and look like they should agree.
  - **What's actually true:** The hot path is `InputManager::handle_axis`'s inline closure (`manager.rs:222-232`), not `AxisBinding::apply`. The two formulas have different mathematical shapes outside the deadzone (raw vs. ramp).

- **Trap:** Touch virtual joystick uses a fixed 80-pixel scale and only writes `move_axis`. There is no touch path to `look_axis`.
  - **Why it's confusing:** A consumer porting to a touch device might expect dual-stick parity.
  - **What's actually true:** `manager.rs:170-174` is the entire touch → axis pipeline. Only `move_axis` is written. Implementing a right-side touch zone for `look_axis` would require new code.

---

## 7. Decision Log

### Decision: winit for keyboard/mouse, gilrs for gamepad
- **Date:** 2025-09-05 (commit `e1d77db90`, "Implement InputManager for handling user input"; verified via `git log -1 --format=%ad`). The Cargo.toml `e1004a2ff` "Add dependencies for UI and gamepad support" landed alongside on the same day.
- **Status:** Accepted
- **Context:** `Cargo.toml:12, 14` pin `winit` and `gilrs` at workspace versions. No `sdl2`, `rawinput`, or alternative input crate appears in workspace `Cargo.lock`.
- **Decision:** Use winit's `WindowEvent` for keyboard/mouse/touch (the windowing crate is already mandatory for the engine), and gilrs for gamepad.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Cross-platform gamepad support follows whatever gilrs supports — Windows (Xinput / DirectInput), Linux (evdev), macOS (IOKit).
  - `winit::keyboard::KeyCode` and `winit::event::MouseButton` are leaked through the public binding types' `Option` fields (`bindings.rs:248-250`), so any consumer of `astraweave-input` must also depend on `winit`.

### Decision: 23-action enum, hardcoded vocabulary
- **Date:** 2025-09-05, commit `3ad045c03` ("Add input context and action enums with Axis2 struct"). Empty commit body — no design rationale captured.
- **Status:** Accepted (`actions.rs:46-75`)
- **Context:** Actions span four categories: movement (4), gameplay verbs (8 including jump/crouch/sprint/interact/attack/ability), UI toggles (5), UI navigation (6).
- **Decision:** A closed `#[non_exhaustive]` enum of 23 named variants rather than a string-id system or a user-extensible registry.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Adding a new action requires modifying `Action`'s `match` arms in `name()`, `is_movement()`, etc., and the `all()` array sized 23.
  - Cross-game reuse of the same `Action` enum constrains what game-specific verbs can exist — no per-game extension point.

### Decision: `InputContext` is stored on the manager but not enforced
- **Date:** Type introduced in 2025-09-05 commit `3ad045c03` (same commit as the `Action` enum). The `InputManager.context` field landed in the manager commit `e1d77db90` later the same day. Both commit bodies are empty.
- **Status:** Accepted by construction
- **Context:** `InputContext` is `set` via `set_context` (`manager.rs:55-57`) and `Action::context()` (`actions.rs:163-169`) provides per-action categorization, but the manager itself does not gate event handling.
- **Decision:** Filtering by context is the caller's responsibility at the query site.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - A single `InputManager` can be queried for both gameplay and UI actions interchangeably.
  - Pause-menu code that wants to suppress gameplay actions must explicitly check `Action::is_gameplay()` or `action.context() == InputContext::UI` at the call site.

### Decision: HashMap-keyed bindings rather than per-action fields
- **Date:** 2025-09-05, commit `c555d52bef` ("Add gamepad and input bindings structures"). Empty commit body.
- **Status:** Accepted (`bindings.rs:415`: `actions: HashMap<Action, Binding>`)
- **Context:** `BindingSet::default()` (`bindings.rs:488-672`) populates the HashMap with 21 default entries.
- **Decision:** Store bindings in a `HashMap<Action, Binding>` rather than a struct with one field per action.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - O(1) action-binding lookup if needed (`bindings.rs:449`), but the event-handling path does the inverse search (find Actions matching a given input) and iterates the whole HashMap linearly (`manager.rs:83-94, 101-112, 201-212`).
  - Adding new actions requires no struct change — but means an action with no binding (no entry in the HashMap) is silently unbound, indistinguishable from a misspelled action.

### Decision: `load_bindings` returns `Option`, not `Result`
- **Date:** 2025-09-05, commit `0645bce8a18` ("Implement save and load functions for bindings"). Empty commit body.
- **Status:** Accepted (`save.rs:16-19`)
- **Context:** `save_bindings` returns `Result<()>`; `load_bindings` returns `Option<BindingSet>`.
- **Decision:** Swallow all load errors (file missing, permissions, corrupt JSON, empty file) and surface `None` for any failure.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:** Callers cannot distinguish "first run, no saved config" from "config is corrupt"; fallback behavior at the call site is always the same: use `BindingSet::default()`. Tests at `save.rs:81-156` lock in this behavior.

### Decision: `#![forbid(unsafe_code)]` at crate root
- **Date:** 2026-02-09, commit `745c100a8` (sweeping commit titled "Mutation-resistant test suites across ~35+ crates …" that also added the `#![forbid(unsafe_code)]` attribute to `lib.rs:1` as a small line within a much larger workspace-wide change; verified via `git log -L "1,5:astraweave-input/src/lib.rs"`). The attribute was *not* present in the original 2025-09-05 crate.
- **Status:** Accepted (`lib.rs:1`)
- **Context:** Input handling is event-driven and has no FFI concerns at this layer (winit and gilrs handle that).
- **Decision:** Zero unsafe in the crate.
- **Alternatives considered:** None reasonable for this layer.
- **Consequences:** No FFI escape hatch; any low-level platform input would have to live in winit/gilrs forks.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `Action::all()` has exactly 23 variants and each variant appears once in the array | Yes | `actions.rs:199-225` + test `test_action_all` (`actions.rs:460-463`) asserts `len() == 23`. |
| 2 | `Action::is_gameplay()` is the complement of `Action::is_ui_nav()` | Yes | `actions.rs:157-159` defines `is_gameplay` as `!self.is_ui_nav()`; test `test_action_is_gameplay` (`actions.rs:420-425`) covers boundary cases. |
| 3 | `GamepadButton::all()` has exactly 16 variants | Yes | `bindings.rs:118-137` + test `test_gamepad_button_all` (`bindings.rs:771-774`). |
| 4 | `AxisKind::all()` has exactly 6 variants and `paired()` is involutive on stick axes | Yes | `bindings.rs:207-215, 228-237` + tests `test_axis_kind_paired` (`bindings.rs:841-848`) and `test_axis_kind_all` (`bindings.rs:865-868`). |
| 5 | `AxisBinding::apply(v)` returns 0 when `|v| < deadzone` | Yes | `bindings.rs:388-393`; tests `test_axis_binding_apply_zero_in_deadzone` (`bindings.rs:970-975`) and `test_axis_binding_apply_outside_deadzone` (`bindings.rs:978-983`). |
| 6 | `BindingSet::default()` populates ≥21 actions (and `non_empty_binding_count > 1`) | Yes | `bindings.rs:488-672` defines defaults; mutation-killer test `non_empty_binding_count_default_is_not_one` (`bindings.rs:1078-1089`) asserts `> 1`. |
| 7 | `Binding::new()` is empty (`is_empty()` is `true`, `binding_count()` is 0) | Yes | `bindings.rs:254-257, 288-291`; tests at `bindings.rs:879-885`. |
| 8 | `Axis2::with_deadzone(v, d)` returns zero iff `v.length() < d` | Yes | `actions.rs:267-269, 304-311`; tests `test_axis2_is_in_deadzone` and `test_axis2_with_deadzone` (`actions.rs:500-547`). |
| 9 | `InputManager::set_action` adds to `just_pressed` only on the down-edge (not on repeated `true`) | Yes | `manager.rs:247-250`; test `test_set_action_double_press_no_duplicate_just_pressed` (`manager.rs:389-396`). |
| 10 | `InputManager::clear_frame` empties `just_pressed` but does not modify `pressed` | Yes | `manager.rs:68-70`. Implicitly covered by `test_set_action_double_press_no_duplicate_just_pressed` (which calls `clear_frame` between presses) and explicit lifetime contracts elsewhere in the test suite. |
| 11 | `InputManager::poll_gamepads` is no-op when `self.gilrs` is `None` (does not panic) | Yes (compile-time + runtime) | `manager.rs:152-156` (`if let Some(g) = ...`); demos that run without a gamepad never panic — every test that constructs an `InputManager` exercises this branch by default since CI runners typically lack gamepads. |
| 12 | `load_bindings` returns `None` on every failure mode (missing, empty, invalid JSON, unparseable) | Yes | `save.rs:16-19`; tests `test_load_bindings_nonexistent_file`, `test_load_bindings_invalid_json`, `test_load_bindings_empty_file` (`save.rs:81-156`). |
| 13 | `save_bindings` creates the parent directory if missing | Yes | `save.rs:7-11`; test `test_save_bindings_creates_directory` (`save.rs:40-50`). |
| 14 | Touch virtual joystick clamps `move_axis` components to `[-1.0, 1.0]` | Yes | `manager.rs:172-173`; test `test_touch_virtual_joystick_clamp` (`manager.rs:590-596`). |
| 15 | Y-axis virtual joystick output is inverted relative to screen Y | Yes | `manager.rs:173`; test `test_touch_virtual_joystick_y_inverted` (`manager.rs:581-587`). |
| 16 | `gilrs::Button::LeftTrigger` → `GamepadButton::L2` and `LeftTrigger2` → `L1` (the cross-mapping is intentional) | Yes | `manager.rs:185-188`; tests `test_handle_button_triggers` and `test_handle_button_bumpers` (`manager.rs:450-465`). |

---

## 9. Performance & Resource Profile

### Hot paths

- **`InputManager::process_window_event`** — called once per `WindowEvent` from the caller's event loop. Each event iterates the `bindings.actions` HashMap (O(N) over actions). For the default 21-entry set this is trivial (sub-microsecond). The bench file does not benchmark this method directly — `benches/input_benchmarks.rs:1-180` defines 14 benches (`bench_binding_*`, `bench_input_manager_creation`, `bench_context_switching`, `bench_is_down_query`, `bench_just_pressed_query`, `bench_clear_frame`, `bench_binding_lookup`, `bench_multiple_queries`, `bench_binding_set_clone`, `bench_action_insertion`, `bench_sensitivity_access`); none invoke `process_window_event`.
- **`InputManager::poll_gamepads`** — called once per frame from the caller's redraw handler. Drains all pending gilrs events into a `Vec` then processes them. Cost scales with input intensity, not with N.
- **HashMap lookups in `handle_button`** — `manager.rs:201-212` re-scans the bindings on every button event. Same O(N) characteristic.

### Cold paths

- **`save_bindings`** — JSON serialization + filesystem write. Once per user-driven save, not per-frame.
- **`load_bindings`** — Read + JSON parse. Typically once at startup.
- **`InputManager::new`** — Calls `Gilrs::new()` which scans connected devices; can take milliseconds. Once per process.

### Resource ownership

- **`Gilrs`** — owned by `InputManager.gilrs: Option<Gilrs>` (`manager.rs:27`). Initialized at construction; `None` if init failed. One per `InputManager`.
- **HashMaps and HashSets** — `BindingSet.actions`, `InputManager.pressed`, `InputManager.just_pressed` — all owned by their containing struct. No `Arc`/`Rc`.
- **No GPU resources, no I/O background threads.** Touch event ingestion uses no allocation per event (Vec2 by value).

---

## 10. Testing & Validation

- **Unit tests:** Inline `#[cfg(test)] mod tests` in each source file. Test counts (from `grep -c '#\[test\]'`):
  - `actions.rs`: 30 tests
  - `bindings.rs`: 51 tests
  - `manager.rs` (internal-tests submodule): 31 tests
  - `save.rs`: 9 tests
  - `lib.rs`: 4 tests (serde roundtrip smoke tests)
- **Integration / campaign tests:**
  - `src/manager_tests.rs`: 55 tests (Week 5 Day 1-3 campaign documented at `manager_tests.rs:1-13`; the header notes 89.13% coverage as of Day 2, 59 tests, 14 benchmarks).
  - `tests/mutation_resistant_comprehensive_tests.rs`: 125 tests targeting cargo-mutants survivor classes.
- **Total tests:** **305** in this crate.
- **Mutation testing:** Dedicated suite at `tests/mutation_resistant_comprehensive_tests.rs` (1023 lines). The bindings module also has explicit mutation-killer tests (`bindings.rs:1078-1097`) that target specific line-level mutations (e.g., `non_empty_binding_count_default_is_not_one` kills the `→ 1` mutation at `bindings.rs:409`).
- **Miri validation:** `#![forbid(unsafe_code)]` (`lib.rs:1`) leaves no in-crate UB surface. Confirmed not present in `.github/workflows/miri.yml`, `kani.yml`, `mutation-testing.yml`, or `coverage.yml` (verified 2026-05-12 via `grep -l "astraweave-input" .github/workflows/*.yml`); the crate appears only in `ci.yml` and `benchmark-dashboard.yml`. Any UB would have to live in winit or gilrs.
- **Benchmarks:** `benches/input_benchmarks.rs` — 180 lines, 14 criterion benches covering `binding_creation`, `binding_serialization`, `binding_deserialization`, `binding_set_creation`, and others.
- **Manual validation:** `examples/ui_controls_demo` runs an interactive demo (though see §4 note that the demo's logic does not actually consume the input state).

---

## 11. Open Questions / Parked Decisions

- **Why do `astraweave-gameplay` and `astraweave-ui` declare the dependency without using it?** Both Cargo.tomls pull in `astraweave-input` (lines 21 and 22 respectively), but workspace grep confirms zero `use astraweave_input` in either `src/`. Investigation (2026-05-12) traced addition commits: `astraweave-gameplay`'s dep was added in commit `3f0ab730d` (2025-10-01, "phase 5 implementation"); `astraweave-ui`'s dep landed at Cargo.toml creation in `dfbe059a4` (2025-09-05, same day as the input crate itself). Neither addition has a paired source-file `use` statement in git history (`git log -p --all -S "use astraweave_input"` on those crate roots returns no matches). Is this stale residue from a planned integration that was reverted, or a pre-positioning of the dep ahead of future wiring? Andrew's call.

- **Why doesn't the one consumer (`examples/ui_controls_demo`) read the `InputManager` state?** The demo feeds events into the manager (`main.rs:176, 234, 235`) and then makes gameplay decisions via direct `match code { KeyCode::KeyI => … }` blocks (`main.rs:196-215`) — never calling `is_down`, `just_pressed`, `move_axis`, or `look_axis`. Investigation (2026-05-12): `git log -p --all --follow examples/ui_controls_demo/src/main.rs | grep` for any `is_down(`, `just_pressed(`, `move_axis`, or `look_axis` token (added or removed) returns **zero matches**. The file was created on 2025-09-05 (commit `b702e71b2`) without state queries and has never contained them — this is not drift from a prior queried-state version. Is the demo intentionally a "ghost integration" (the engine exercises the input pipeline but doesn't depend on it), or did the developer always intend to wire state queries later? Andrew's call.

- **Disposition of AI-generated aspirational input docs.** `docs/src/core-systems/input.md` and friends were created in commit `28bc94f21` (2025-09-08, "Create comprehensive bespoke wiki with 51-section documentation structure (#34)") authored by GitHub Copilot bot — the same bulk-doc commit that introduced the aspirational audio docs. Should these docs be (a) deleted, (b) rewritten to match the actual `astraweave-input` API, or (c) retained as a roadmap for a future input API rewrite? Andrew's call.

- **`InputContext` is stored but never enforced.** The field is set-only; the manager dispatches input regardless of mode. Was this intended for future implementation (with a `set_context(UI)` call suppressing gameplay actions), or is the design intent "context is callers' problem"? Note that `Action::context()` already lets the caller filter at the query site — so the field on the manager may be vestigial.

- **Inline axis formula vs. `AxisBinding::apply`.** `manager.rs:222-232` and `bindings.rs:387-400` apply `(invert, deadzone)` differently — one normalizes past the deadzone, the other does not. Investigation (2026-05-12): comprehensive workspace grep for `\.apply(` against `astraweave-input/src` finds **only test call sites** (`bindings.rs:972-988`); zero production calls. The `apply` method is reachable through the public API surface (`pub fn apply`) but no code in the crate's hot path uses it. Is the divergence intentional (different shapes for different consumers, with the `apply` method available for callers that want normalized output) or vestigial code that was never removed after the inline formula was preferred?

- **`load_bindings` collapses all error modes to `None`.** Should the API expose a structured error type so callers can distinguish "first run" from "corrupt config"? Andrew's call.

- **`look_sensitivity` field is `pub` but no method in `src/` consumes it.** `manager.rs:46` defaults it to `0.12`. The only read site in the entire crate is `bench_sensitivity_access` (`benches/input_benchmarks.rs:153-160`) which loads the field for a Criterion timing measurement and does not use the value in any computation. The expectation is presumably that the caller queries `look_axis` then scales by `look_sensitivity` itself. Should this be enforced internally, documented as caller-side, or removed? Andrew's call.

- **`save_bindings` has no production call site.** Workspace grep for `save_bindings` returns matches only in this crate's tests (`save.rs:34, 47, 60, 73, 105, 132, 137`). The function is on the public API surface but unused outside tests. **Investigation finding (2026-05-12):** `tools/aw_editor/src/panels/input_bindings_panel.rs:1617, 1664, 1670, 1685` *does* contain `save_bindings = true` and "Save as Custom" / "Export" buttons — but those are local `bool` flags inside the editor's parallel implementation, not calls to `astraweave_input::save_bindings`. The editor's panel queues a `InputBindingAction::SaveBindings` variant onto `pending_actions`, which is then never drained externally (see §6 parallel-implementation row). So the actual `astraweave_input::save_bindings` function is genuinely dormant in production. Is wiring the rebinding UI through the real crate a parked feature, or is this dormant code? Andrew's call.

- **Gamepad mapping coverage.** `manager.rs:177-218` maps 16 gilrs `Button` variants to `GamepadButton`. Investigation (2026-05-12) enumerated gilrs 0.10.10's `Button` enum (`~/.cargo/registry/.../gilrs-0.10.10/src/ev/mod.rs:112-140`): 19 variants exist (excluding the `Unknown` default sentinel), so the manager drops exactly **3**: `Button::C`, `Button::Z`, and `Button::Mode`. gilrs 0.10 does not define paddle buttons (the prior trace doc's "paddles, etc." was speculation; corrected). `C` and `Z` originate from SNES-style legacy six-face-button controllers (Sega Genesis, classic arcade pads); `Mode` is the "guide/home" button on Xbox/PlayStation pads. Is this 3-button gap an intentional decision (these are rarely-used / not portable to standard AAA controllers), or a future TODO?

- **Editor's `InputBindingsPanel` reinvents the entire input domain.** Surfaced by deep-investigation pass on 2026-05-12. `tools/aw_editor/src/panels/input_bindings_panel.rs` (2511 lines) defines 13 input-related types (`InputBindingAction`, `InputDevice`, `ActionCategory`, `BindingPreset`, `GamepadButton`, `KeyboardKey`, `MouseButton`, `ActionBinding`, `AxisBinding`, `BindingConflict`, `InputBindingsPanel`, `InputTarget`, `InputTab`) that duplicate (with different membership and semantics) the types in `astraweave-input`. The editor's `Cargo.toml` does not depend on `astraweave-input`. The panel renders (the dock wires it via `.show(ui)` at `dock_panels.rs:215`, `tab_viewer/mod.rs:7736`) and queues `InputBindingAction` events onto `pending_actions: Vec<InputBindingAction>` (`input_bindings_panel.rs:866`), but no external code calls `panel.take_actions()` — so user button presses on "Save as Custom" / "Import" / "Export" / "Reset to Default" / preset selection have no effect beyond updating the panel's local state. Several questions for Andrew: (a) Should the editor be migrated to use `astraweave-input`'s types directly? (b) Should the parallel implementation be deleted as dead UI? (c) Was the parallel implementation a deliberate hedge against `astraweave-input` evolving and breaking the editor? (d) Should the `pending_actions` queue actually be drained somewhere?

---

## 12. Maintenance Notes

**Update this doc when:**
- A new `Action` variant is added (§1 summary, §3 vocabulary, §8 invariants 1-2, default `BindingSet`).
- A new `GamepadButton` variant is added (§3, §8 invariants 3, 16).
- An ECS integration is introduced (§4 cross-system, §7 decision log).
- Either of the declared-but-unused workspace deps (`astraweave-gameplay`, `astraweave-ui`) starts actually importing the crate (§4 declared-but-unused-deps row, §11 first question).
- The aspirational `docs/src/core-systems/input.md` is rewritten or removed (§6 coexisting-abstractions row, §11 disposition question).
- `InputManager.context` field becomes read-active or is removed (§6 trap, §11 question).
- Axis-formula divergence is reconciled or further entrenched (§6 row, §11 question).
- `save_bindings` / `load_bindings` API gains a structured error type (§7 fifth decision, §11 question).

**Verification process:**
- Spot-check: `rg 'pub fn|pub struct|pub enum|pub trait' astraweave-input/src/` should show roughly the surface listed in §3 vocabulary and §5 file map.
- `cargo tree -p astraweave-input --depth 1` should list `winit`, `gilrs`, `glam`, `serde`, `serde_json`, `anyhow`, `thiserror`. Anything more or less indicates dependency drift since §7 first decision.
- `rg 'use astraweave_input' --type rust -g '!*test*' -g '!benches/*'` should find exactly one production hit (`examples/ui_controls_demo/src/main.rs:15`). New consumers must be added to §4.
- Stamp the new commit hash and date in the metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. The crate is **not** an ECS-citizen — there is no input system stage and no `Resource`. The only production consumer (`examples/ui_controls_demo`) owns an `InputManager` directly, and even *it* does not query the manager's state (UI toggles are handled via direct `match KeyCode` instead).
2. Two workspace crates (`astraweave-gameplay`, `astraweave-ui`) declare the dependency but never import it. Don't assume "they probably use it for X" — workspace grep confirms zero uses.
3. `docs/src/core-systems/input.md` describes an API that **does not exist** in this crate. The real API is the four modules re-exported by `lib.rs` (`actions`, `bindings`, `manager`, `save`).
4. `InputManager.context` is set but never read; `Action::context()` is the actual classification mechanism.

**Files you'll most likely touch:**
- `astraweave-input/src/actions.rs` — adding/removing actions (also update `Action::all()`, every `match` arm).
- `astraweave-input/src/bindings.rs::BindingSet::default` — when adding default keybinds.
- `astraweave-input/src/manager.rs` — event-routing changes, gamepad-mapping changes.

**Files you should NOT touch without strong reason:**
- `astraweave-input/tests/mutation_resistant_comprehensive_tests.rs` — mutation-resistance assertions; changes here can mask real bugs.
- `astraweave-input/src/manager_tests.rs` — Week 5 campaign suite locked in current behavior; deltas need cross-checking.

**Common mistakes when changing this system:**
- **Adding an `Action` variant without updating every `match` arm and `Action::all()`.** Compile errors will catch most arms, but the `all()` array length and `name()` arm need manual updates.
- **Assuming `set_context(UI)` will suppress gameplay actions in the manager.** It will not — see §6 trap. Filter at the query site.
- **Calling `is_down` / `just_pressed` without `clear_frame`.** `just_pressed` accumulates between frames if `clear_frame` is never called.
- **Conflating `AxisBinding::apply` with the gamepad-axis path.** The hot path uses a different inline formula (§6 axis-formula row).
- **Trusting `load_bindings`'s `None` to mean "missing file".** It also means "corrupt JSON" or "empty file" — handle by falling back to `BindingSet::default()` unconditionally.
