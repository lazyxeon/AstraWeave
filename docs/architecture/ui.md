---
schema_version: 1
trace_id: ui
title: "UI System (HUD, Menus, Panels)"
description: "UI — HUD, menus, panels (egui)"
primary_crate: astraweave-ui
domain: gameplay
lifecycle_status: in_design
integration_status: example_only
owns: [astraweave-ui]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: UI System (HUD, Menus, Panels)

## Metadata

| Field | Value |
|---|---|
| **System name** | UI System (HUD, menus, panels) |
| **Primary crates** | `astraweave-ui` |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-24 |
| **Status** | Active (egui-based), with backup/residue source files on disk and several exported-but-unconsumed modules |
| **Owner notes** | Module-level doc-comments are dated by "Week N Day M" labels (e.g. "Week 3 Day 4", "Week 5 Day 2"), reflecting an incremental build campaign. The crate has no `tools/aw_editor` consumer; its only production callers are two examples (`ui_menu_demo`, `ui_controls_demo`). Generated from forensic read of source on 2026-06-24. |

---

## 1. Executive Summary

**What this system does:**
Provides the in-game UI stack for AstraWeave — a modal menu state machine (main/pause/settings with TOML persistence), a persistent HUD overlay (health bars, damage numbers, combos, quest tracker, minimap, dialogue, tooltips, notifications), and an egui↔wgpu integration layer that owns the per-frame egui pass and paints it onto a wgpu surface.

**Why it exists:**
Gives game examples a self-contained egui-based UI without each example re-implementing menu navigation, settings persistence, and HUD rendering.

**Where it primarily lives:**
- `astraweave-ui/src/layer.rs` — egui ↔ winit ↔ wgpu integration (`UiLayer`)
- `astraweave-ui/src/hud.rs` (4,699 LoC) — HUD data structures + `HudManager` rendering
- `astraweave-ui/src/menu.rs` — menu state machine + settings types (`MenuManager`, `SettingsState`)
- `astraweave-ui/src/menus.rs` — egui draw functions for main/pause/settings menus
- `astraweave-ui/src/panels.rs` — `draw_ui()` aggregate entry point (top bar, inventory/crafting/map/quest/settings windows, cinematics dev panel)
- `astraweave-ui/src/persistence.rs` — TOML settings save/load
- `astraweave-ui/src/{accessibility,gamepad,state}.rs` — supporting types

**Status note:**
The active path is egui (verified: `astraweave-ui/Cargo.toml:9-13` declares `egui`, `egui-winit`, `egui-wgpu`; `layer.rs:1-3` imports them). Two **identical backup source files** (`menus_broken.rs`, `menus_backup2.rs`) sit on disk but are **not declared as modules** in `lib.rs` and are therefore dead (see §6). The crate exports two subsystems — `accessibility` (colorblind transforms) and `gamepad` (gilrs controller) — that have **zero non-test consumers anywhere in the workspace** (see §4, §6). There are also **two different "settings" UIs** (`menus::show_settings_menu` and the accessibility window inside `panels::draw_ui`) that are reached through different entry points.

---

## 2. Authoritative Pipeline

The crate exposes **two distinct integration paths** that callers pick between; they are not layered on top of each other.

### Path A — `draw_ui()` aggregate (used by `ui_controls_demo`)

```text
[Caller per-frame]
    │
    │ UiLayer::begin(window)            (layer.rs:75-78)
    ▼
[egui frame begin]  egui_winit.take_egui_input → ctx.begin_pass
    │
    │ draw_ui(ctx, flags, menu_manager, acc, player_stats, pos, inventory, recipes, quests)
    ▼
[panels::draw_ui]                       (panels.rs:74-366)
    ├─ menu_manager.show(ctx)           → modal menu (menu.rs:215-224 → menus.rs)
    ├─ TopBottomPanel "top_bar"         → Menu/Inventory/Crafting/Map/Quests/Settings buttons + HP/STA/Pos
    ├─ TopBottomPanel "bottom" "hud"    → ability hints + accessibility status
    ├─ if flags.show_inventory  → Window "Inventory"
    ├─ if flags.show_crafting   → Window "Crafting" (craft_and_push)
    ├─ if flags.show_map        → Window "Map" (placeholder)
    ├─ if flags.show_quests     → Window "Quest Log"
    ├─ if flags.show_settings   → Window "Settings / Accessibility"  (acc fields)
    └─ Window "Cinematics"      → dev-only timeline load/save/step (statics)
    │
    │ returns UiResult { crafted, menu_action }
    ▼
[UiLayer::end_and_paint(...)]           (layer.rs:155-175)
    end_frame → tessellate → paint onto wgpu TextureView (LoadOp::Load)
```

### Path B — direct `MenuManager` + `HudManager` (used by `ui_menu_demo`)

```text
[Caller per-frame]
    │
    │ UiLayer::begin(window)
    ▼
[egui frame begin]
    │
    ├─ hud_manager.render(ctx)          (hud.rs:1016 …)   — persistent overlay
    ├─ (example draws its own egui windows on ctx)
    └─ menu_manager.show(ctx)           (menu.rs:215-224) — modal menu, returns MenuAction
    │
    ▼
[UiLayer::end_and_paint(...)] → wgpu surface
```

### Stage-by-stage detail

#### Stage: egui integration layer (`UiLayer`)
**File:** [`astraweave-ui/src/layer.rs`](../../astraweave-ui/src/layer.rs)
**Role:** Owns `egui::Context`, `egui_winit::State`, and `egui_wgpu::Renderer`. Bridges winit window events, the egui pass lifecycle, and wgpu painting.
**Inputs:** `&Window`, `&wgpu::Device`, `wgpu::TextureFormat` (`new`, `layer.rs:49-67`); `WindowEvent` (`on_event`, `layer.rs:69-72`).
**Outputs:** Clipped primitives + textures deltas (`end_frame`, `layer.rs:82-97`); painted egui onto a provided `TextureView` (`paint`, `layer.rs:101-152`).
**Notes:** Uses `LoadOp::Load` so the egui pass composites on top of an already-rendered scene (`layer.rs:131`). Contains the crate's only `unsafe`: a `std::mem::transmute` to extend a `RenderPass` lifetime to `'static`, documented as required by egui-wgpu 0.32's API (`layer.rs:140-145`). `egui_wgpu::Renderer::new(... false ...)` passes `false` for srgb support (`layer.rs:59`).

#### Stage: menu state machine (`MenuManager`)
**File:** [`astraweave-ui/src/menu.rs`](../../astraweave-ui/src/menu.rs)
**Role:** Holds `MenuState` (`MainMenu`/`PauseMenu`/`SettingsMenu`/`None`), `SettingsState` (current + original for revert), and `rebinding_key`. Dispatches `show()` to the correct `menus::*` draw function and applies `MenuAction` transitions.
**Inputs:** `&egui::Context` (`show`, `menu.rs:215`); `MenuAction` (`handle_action`, `menu.rs:227`); ESC (`toggle_pause`, `menu.rs:280`).
**Outputs:** `MenuAction`; mutated `MenuState`; on apply, persisted settings via `persistence::save_settings` (`menu.rs:320-329`).
**Notes:** `MenuManager::new()` loads settings from disk on construction (`menu.rs:203-211`). Settings apply does **not** push values to the renderer/window — there is a `// In future: Apply settings to window/renderer here` comment (`menu.rs:328`); `MenuAction::ApplySettings` only writes the TOML and updates the revert baseline.

#### Stage: menu draw functions
**File:** [`astraweave-ui/src/menus.rs`](../../astraweave-ui/src/menus.rs)
**Role:** Pure egui draw functions: `show_main_menu` (`menus.rs:47-124`), `show_pause_menu` (`menus.rs:127-198`), `show_settings_menu` (`menus.rs:201-628`). Each draws a full-screen dark `Area` background + a centered `Window`, returning a `MenuAction`.
**Inputs:** `&egui::Context`; for settings, `&mut SettingsState` + `&mut Option<String>` rebinding key.
**Outputs:** `MenuAction`.
**Notes:** The settings menu builds graphics (resolution/quality/fullscreen/vsync), audio (4 volume sliders + mutes), and controls (10 key-binding buttons + sensitivity + invert-Y). Key rebinding here only sets `*rebinding_key = Some(id)` on click (`menus.rs:458`); **no code in this crate captures the next key press to complete the rebind** — the capture must be supplied by the caller.

#### Stage: aggregate panels (`draw_ui`)
**File:** [`astraweave-ui/src/panels.rs`](../../astraweave-ui/src/panels.rs)
**Role:** Single-call composite for the demo HUD/menus: draws the modal menu, a top bar, a bottom HUD strip, toggled feature windows, and a cinematics dev panel.
**Inputs:** `ctx`, `&mut UiFlags`, `&mut MenuManager`, `&mut Accessibility`, `&Stats`, `Vec3`, `&mut Inventory`, `Option<&RecipeBook>`, `Option<&mut QuestLog>` (`panels.rs:74-85`).
**Outputs:** `UiResult { crafted: Option<String>, menu_action: Option<MenuAction> }`.
**Notes:** Reads `astraweave-gameplay` types directly (`Stats`, `Inventory`, `RecipeBook`, `QuestLog`). The cinematics panel uses module-level `OnceLock<Mutex<...>>` statics for `Timeline`/`Sequencer`/filename (`panels.rs:241-248`) and reads/writes JSON to `assets/cinematics/` (`panels.rs:269-302`). Several of those file I/O sites discard `Result` with `let _ =` (`panels.rs:293,295`) — a dev-only panel, not the settings-persistence path.

#### Stage: HUD rendering (`HudManager`)
**File:** [`astraweave-ui/src/hud.rs`](../../astraweave-ui/src/hud.rs)
**Role:** Owns all HUD state and a single `render(ctx)` that paints the overlay. Holds `player_stats`, `enemies`, `damage_numbers`, `combo_tracker`, `notification_queue`, `active_quest`, `poi_markers`, `ping_markers`, `active_dialogue`, `hovered_tooltip`, and audio callbacks (`hud.rs:692-724`).
**Inputs:** Mutated via public fields and methods (e.g. `start_dialogue`, `show_tooltip`, `set_minimap_zoom`); time via `update(dt)` (`hud.rs:898`).
**Outputs:** egui draw calls onto `ctx` (`render`, `hud.rs:1016`). Returns next dialogue node id from `select_dialogue_choice` (`hud.rs:862`).
**Notes:** HUD is documented as "separate from MenuManager (menu system is modal, HUD is persistent overlay)" (`hud.rs:6-7`). Health uses an eased animation (`HealthAnimation`, `hud.rs:35-124`). Audio is decoupled via optional callbacks `on_minimap_click` / `on_ping_spawn` (`hud.rs:718-720`) rather than a direct audio dependency.

#### Stage: persistence
**File:** [`astraweave-ui/src/persistence.rs`](../../astraweave-ui/src/persistence.rs)
**Role:** Save/load `SettingsState` as versioned TOML at the platform config dir (`AstraWeave/settings.toml`).
**Inputs/Outputs:** `save_settings(&SettingsState) -> Result<()>` (`persistence.rs:38-53`); `load_settings() -> SettingsState` (infallible, defaults on error, `persistence.rs:56-67`).
**Notes:** `SETTINGS_VERSION = 1` with a `SettingsFile { version, settings }` wrapper and a placeholder version-migration branch (`persistence.rs:82-91`). Properly uses `anyhow::Context` and `?` (not silent `let _ =`).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **HUD** | Persistent, non-modal in-game overlay (health, minimap, quest tracker, dialogue, tooltips, notifications). Never blocks input. | `hud.rs` |
| **Menu** | Modal screen (main/pause/settings) that overlays a dark full-screen background. Driven by a state machine. | `menu.rs`, `menus.rs` |
| **Panel** | A toggled egui `Window` or `TopBottomPanel` inside `draw_ui` (inventory, crafting, map, quest log, accessibility, cinematics). | `panels.rs` |
| **`MenuState`** | Enum: which modal menu is showing (`MainMenu`/`PauseMenu`/`SettingsMenu`/`None`). | `menu.rs:9-18` |
| **`MenuAction`** | Enum of menu-triggered intents (`NewGame`/`Resume`/`Settings`/`ApplySettings`/`Quit`/…). Returned up to the caller. | `menu.rs:166-187` |
| **`UiFlags`** | Per-panel boolean visibility toggles consumed by `draw_ui` (`show_inventory`, `show_map`, …). | `state.rs:28-36` |
| **`SettingsState`** | Serializable graphics + audio + controls settings persisted to TOML. | `menu.rs:155-163` |
| **`Accessibility`** | A simple serializable struct (`high_contrast_ui`, `reduce_motion`, `subtitles`, `subtitle_scale`, `colorblind_mode: Option<String>`) used by `draw_ui`. | `state.rs:7-26` |
| **`AccessibilitySettings`** | A **different** struct in `accessibility.rs` with a `ColorblindMode` enum and color-transform math. Not the same type as `Accessibility`. | `accessibility.rs:60-128` |
| **`UiLayer`** | egui↔winit↔wgpu integration object; not UI content, just the frame/paint plumbing. | `layer.rs:7-12` |

### Terms to NOT confuse

- **`Accessibility` (state.rs) vs `AccessibilitySettings` (accessibility.rs):** Two different types with overlapping intent. `draw_ui` and both demos use `state::Accessibility` (a string-tagged colorblind mode). `accessibility::AccessibilitySettings` + `ColorblindMode` enum + `transform_color()` is a separate, richer module that **no workspace code outside the crate consumes** (§4). They are not wired together.
- **`menu` (singular) vs `menus` (plural):** `menu.rs` is the state machine + data types; `menus.rs` is the egui draw functions it dispatches to. Both are live modules (`lib.rs:28-29`). Do not confuse either with the dead `menus_broken.rs` / `menus_backup2.rs` (§6).
- **The two "settings" surfaces:** `menus::show_settings_menu` (full graphics/audio/controls, reached via `MenuManager` when `MenuState::SettingsMenu`) vs. the "Settings / Accessibility" `Window` inside `draw_ui` (only accessibility toggles, reached via `UiFlags::show_settings`). They are independent.
- **"Menu" button ambiguity:** In `draw_ui`'s top bar the "Menu" button calls `menu_manager.toggle_pause()` (`panels.rs:37,95-97`), i.e. it toggles the pause menu, not a generic menu.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| winit | `UiLayer::on_event(&Window, &WindowEvent) -> bool` (`layer.rs:69-72`) | Window/input events | Returns `consumed`; caller is expected to suppress game input when egui consumed it. |
| wgpu | `UiLayer::new(window, device, format)` / `paint(device, queue, encoder, view, …)` (`layer.rs:49,101`) | GPU device/queue/encoder/target view | egui composites with `LoadOp::Load`. |
| `astraweave-gameplay` | `panels::draw_ui` params + `state::UiData` (`panels.rs:74-85`, `state.rs:38-45`) | `Stats`, `Inventory`, `RecipeBook`, `QuestLog`, `RecipeBook::craft` | `draw_ui` reads HP/STA/pos and mutates inventory on craft (`panels.rs:65-72`). |
| `astraweave-cinematics` | `awc::Timeline`/`Sequencer`/`Track` in the cinematics panel (`panels.rs:10,238-363`) | Timeline JSON, sequencer steps | Dev-only panel; state held in module statics. |
| Caller game loop | Public `HudManager` fields/methods | Player stats, enemies, quests, dialogue, ping markers | HUD is driven entirely by the caller pushing data in. |

### Downstream (what consumes this system's output)

| Consumer | Interface | Data | Notes |
|---|---|---|---|
| `examples/ui_controls_demo` | `draw_ui(...)`, `UiLayer`, `MenuManager`, `Accessibility`, `UiFlags`, `UiData` (`examples/ui_controls_demo/src/main.rs:17`) | `UiResult` | Uses **Path A** (`draw_ui`). Also the sole real consumer of `astraweave-input` in the workspace (see ARCHITECTURE_MAP note below). |
| `examples/ui_menu_demo` | `UiLayer`, `MenuManager`, `HudManager`, `DialogueNode`, `DialogueChoice`, `TooltipData`, `QuestNotification` (`examples/ui_menu_demo/src/main.rs:47-51,236,582,774`) | `MenuAction`, rendered HUD | Uses **Path B** (direct `HudManager::render` at `main.rs:758` + `MenuManager::show` at `main.rs:830`); does **not** call `draw_ui`. |
| `examples/hello_companion` | `astraweave-ui` as an **optional** dep (`examples/hello_companion/Cargo.toml:29,59`) | — | Gated behind the `visual` feature (`Cargo.toml:54-71`). Verified — even with `visual` on, no `use astraweave_ui` / `UiLayer` / `HudManager` / `MenuManager` / `draw_ui` appears anywhere in `src/`; `chat_ui.rs` and `visual_demo.rs` draw raw egui directly (`chat_ui.rs:109` takes `&egui::Context`). The dep is declared-but-unused on every build path. |

### Bidirectional / Coupled

- **`MenuManager` ↔ persistence:** `MenuManager::new()` loads settings on construction (`menu.rs:204`); `apply_settings()` writes them back (`menu.rs:322`). Settings are owned by `MenuManager`, persisted by `persistence`.

### Notable non-touchpoints (declared but unused)

- **`astraweave-input`:** declared in `Cargo.toml:22` but there is **no `use astraweave_input`** anywhere in `astraweave-ui/src/`. The crate ships its own `gamepad.rs` using `gilrs` directly. Confirmed by [`ARCHITECTURE_MAP.md`](ARCHITECTURE_MAP.md) lines 121-123, 155, 414, 945 and [`input.md`](input.md) §metadata/§4/§11: "`astraweave-gameplay` and `astraweave-ui` both declare `astraweave-input` as a workspace dep, but neither imports it."
- **`accessibility` module:** `AccessibilitySettings` / `ColorblindMode` / `transform_color` / `get_health_colors` are exported (`lib.rs:65-68`) but have **zero non-test consumers** in `examples/` or `tools/` (grep returned none), and are **not referenced inside `hud.rs`** (no `accessibility::` use in HUD render).
- **`gamepad` module:** `GamepadManager` / `GamepadBindings` / `GamepadAction` are exported (`lib.rs:69`) but have **zero non-test consumers** in `examples/` or `tools/`. `ui_controls_demo` polls gamepads through `astraweave-input`'s `InputManager::poll_gamepads`, not this module.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| [`src/lib.rs`](../../astraweave-ui/src/lib.rs) | Module declarations + re-exports; declares 9 modules (`accessibility,gamepad,hud,layer,menu,menus,panels,persistence,state`) | Active | Does **not** declare `menus_broken`/`menus_backup2`. Re-exports egui (`lib.rs:72`). |
| [`src/layer.rs`](../../astraweave-ui/src/layer.rs) | egui↔winit↔wgpu integration (`UiLayer`) | Active | Holds the crate's only `unsafe` (lifetime transmute, `layer.rs:140-145`). |
| [`src/hud.rs`](../../astraweave-ui/src/hud.rs) (4,699 LoC) | HUD data + `HudManager::render` | Active | Largest file. Wired via `ui_menu_demo` Path B. |
| [`src/menu.rs`](../../astraweave-ui/src/menu.rs) | `MenuManager` state machine + settings types | Active | `MenuState`/`MenuAction`/`SettingsState`/`QualityPreset` etc. |
| [`src/menus.rs`](../../astraweave-ui/src/menus.rs) | egui draw fns for main/pause/settings | Active | Canonical menu-draw module. |
| [`src/panels.rs`](../../astraweave-ui/src/panels.rs) | `draw_ui()` aggregate + cinematics dev panel | Active | Wired via `ui_controls_demo` Path A. |
| [`src/persistence.rs`](../../astraweave-ui/src/persistence.rs) | TOML settings save/load | Active | Proper `Result`/`Context` handling. |
| [`src/state.rs`](../../astraweave-ui/src/state.rs) | `Accessibility`, `UiFlags`, `UiData` | Active | `Accessibility` (string colorblind tag) is the one consumed by `draw_ui`/demos. |
| [`src/accessibility.rs`](../../astraweave-ui/src/accessibility.rs) | `AccessibilitySettings`, `ColorblindMode`, color transforms | Active (in-design / tested) | Exported, tested, but **no non-test workspace consumer** (§4). |
| [`src/gamepad.rs`](../../astraweave-ui/src/gamepad.rs) | `GamepadManager` (gilrs), bindings | Active (in-design / tested) | Exported, tested, but **no non-test workspace consumer** (§4). |
| [`src/menus_broken.rs`](../../astraweave-ui/src/menus_broken.rs) (596 LoC) | Older condensed copy of menu draw fns | Legacy / residue | **Not a declared module.** Byte-identical to `menus_backup2.rs`. Not compiled. |
| [`src/menus_backup2.rs`](../../astraweave-ui/src/menus_backup2.rs) (596 LoC) | Older condensed copy of menu draw fns | Legacy / residue | **Not a declared module.** Byte-identical to `menus_broken.rs`. Not compiled. |
| [`src/mutation_tests.rs`](../../astraweave-ui/src/mutation_tests.rs) | `#[cfg(test)] mod mutation_tests` (`lib.rs:34-35`) | Active (test) | Compiled only under test. |

**Status definitions:** Active = canonical/load-bearing; Legacy/residue = on disk, not module-declared, not compiled.

---

## 6. Conflict Map / Residue

### Backup / residue source files (high-value)

| File | Module-declared? | Compiled? | Relationship | Status |
|---|---|---|---|---|
| `src/menus.rs` (687 LoC) | **Yes** (`lib.rs:29`) | Yes | Canonical. Expanded/rustfmt'd form. | Active |
| `src/menus_broken.rs` (596 LoC) | **No** | No | Older condensed copy of the same three draw fns. | Legacy / residue |
| `src/menus_backup2.rs` (596 LoC) | **No** | No | **Byte-identical** to `menus_broken.rs` (`diff` empty). | Legacy / residue |

**Forensic facts:**
- Only `mod menu;` and `mod menus;` exist in `lib.rs` (`lib.rs:28-29`). A workspace grep for `menus_broken` / `menus_backup` / `menu_broken` / `menu_backup` returns **no `mod` declaration anywhere** — these files are not part of the build.
- `menus_broken.rs` and `menus_backup2.rs` have **no diff between them** (verified with `diff`), and both differ from `menus.rs` only in formatting/expansion (same `MenuAction` logic; same `show_main_menu`/`show_pause_menu`/`show_settings_menu`/`styled_button` surface). The header doc-comment block is identical across all three.
- All three files (`menus.rs`, `menus_broken.rs`, `menus_backup2.rs`) were **added in the same commit** `2468b25f1` ("Replace Phi3 with Hermes2Pro and add UI fixes, latency optimizations, and advanced features") per `git log --follow` (verified — both backups show only that single commit in `git log --oneline --follow`, no subsequent history). The two backups have had no subsequent history. [Reasoning for keeping both backups not recovered from available sources — verified the originating commit message is a generic multi-feature message with no note about the duplicate backups; no design/audit doc references these files.]

**Canonical answer:** `menus.rs` is canonical; `menus_broken.rs` and `menus_backup2.rs` are inert residue (no compilation, no callers).

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| `state::Accessibility` (string `colorblind_mode`, used by `draw_ui` + demos) | `state.rs:7-26` | Active, wired | Canonical for the demo path. |
| `accessibility::AccessibilitySettings` + `ColorblindMode` enum + `transform_color` | `accessibility.rs:16-128` | Active code, **no non-test consumer** | Exported (`lib.rs:65-68`); not wired into HUD or `draw_ui`. |
| Two settings UIs: `menus::show_settings_menu` (graphics/audio/controls) vs. `draw_ui` "Settings / Accessibility" window (acc only) | `menus.rs:201-628`, `panels.rs:211-235` | Both Active | Reached via different entry points (`MenuState::SettingsMenu` vs `UiFlags::show_settings`). |
| Two integration paths: `draw_ui` aggregate vs. direct `MenuManager`+`HudManager` | `panels.rs` vs `ui_menu_demo` | Both Active | Path A and Path B in §2; not layered. |

### Naming collisions

- **"Accessibility":** `state::Accessibility` (a flat serializable settings struct) vs `accessibility::AccessibilitySettings` (a different struct with transform math) vs `accessibility::ColorblindMode` (enum) vs `state::Accessibility::colorblind_mode` (an `Option<String>`). The wired path uses the string-tagged `state::Accessibility`; the enum-based module is dormant. Future direction: not recorded.
- **"Settings":** `SettingsState` (graphics/audio/controls, persisted) vs the "Settings / Accessibility" accessibility-only window in `draw_ui`. Same word, different content and persistence behavior.
- **"menu" vs "menus":** state machine module vs draw-function module (both live), distinct from the dead `menus_*` backups.

### Known cognitive traps

- **Trap:** Editing a settings menu and only changing `menus_backup2.rs` / `menus_broken.rs`.
  **Why confusing:** They sit next to the canonical `menus.rs`, share the same header comment, and look authoritative.
  **What's actually true:** They are not module-declared and never compile. All menu changes must land in `menus.rs`.
- **Trap:** Assuming key rebinding in the settings menu actually rebinds.
  **Why confusing:** The UI shows "Press any key…" and sets `rebinding_key` (`menus.rs:437-459`).
  **What's actually true:** This crate never captures the subsequent key press to write the new binding; the capture is the caller's responsibility (none of the in-crate code completes the rebind).
- **Trap:** Assuming `MenuAction::ApplySettings` changes the live resolution/quality.
  **Why confusing:** The settings menu has resolution/quality/vsync controls.
  **What's actually true:** Apply only writes TOML and updates the revert baseline; there is an explicit `// In future: Apply settings to window/renderer here` (`menu.rs:328`). No renderer/window plumbing exists in this crate.
- **Trap:** Assuming `astraweave-ui` uses `astraweave-input` for gamepad/input.
  **What's actually true:** The dep is declared but unimported; gamepad support is the crate's own `gamepad.rs` (gilrs direct), which itself has no consumers. See §4.

---

## 7. Decision Log

### Decision: egui as the UI backend
- **Date:** [pre-dates this trace; not pinned]
- **Status:** Accepted
- **Context:** Crate-level docs and README state the UI is "built on egui" (`lib.rs:3`, `README.md:3`); `Cargo.toml:9-13` pulls `egui`/`egui-winit`/`egui-wgpu`.
- **Decision:** Use egui (immediate-mode) with `egui-wgpu`/`egui-winit` for integration.
- **Alternatives considered:** [Reasoning not recovered from available sources — the originating commits (`dfbe059a4` "Create Cargo.toml for astraweave-ui", `9d284eb3a` "Add UiLayer for egui integration") record no alternatives discussion; no design doc was found weighing egui against other backends.]
- **Consequences:** UI is immediate-mode and re-emitted each frame; integration requires the `UiLayer` pass/paint plumbing and a per-frame `begin`/`end_and_paint`.

### Decision: HUD separate from menu system
- **Date:** [Week 3 build campaign, exact date not pinned]
- **Status:** Accepted
- **Context:** Doc-comment: "Separate from MenuManager (menu system is modal, HUD is persistent overlay)" (`hud.rs:6-7`).
- **Decision:** Keep `HudManager` (persistent overlay) and `MenuManager` (modal) as independent objects rendered onto the same `egui::Context`.
- **Consequences:** Callers compose both manually (Path B). HUD never blocks input; menus draw a dark full-screen background.

### Decision: settings persisted as versioned TOML
- **Date:** [pre-dates this trace]
- **Status:** Accepted
- **Context:** `persistence.rs` wraps `SettingsState` in `SettingsFile { version, settings }` with `SETTINGS_VERSION = 1` and a placeholder migration branch (`persistence.rs:13-21,82-91`).
- **Decision:** Versioned TOML at the platform config dir; `load_settings` is infallible and defaults on any error.
- **Consequences:** Corrupt/missing settings degrade to defaults silently (logged at `warn`). Future format changes can hook the migration branch.

### Decision: HUD audio via optional callbacks
- **Date:** "Week 5 Day 2" (`hud.rs:718`)
- **Status:** Accepted
- **Context:** `HudManager` carries `on_minimap_click` / `on_ping_spawn` callbacks instead of an audio crate dependency (`hud.rs:718-720`).
- **Decision:** Decouple HUD from audio via caller-supplied closures.
- **Consequences:** `astraweave-ui` has no `astraweave-audio` dependency; audio wiring is the caller's job.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | The compiled menu surface comes only from `menus.rs`; `menus_broken.rs` / `menus_backup2.rs` are never built. | Yes | `lib.rs` has no `mod menus_broken/menus_backup2`; grep confirms. |
| 2 | `load_settings()` never panics and never returns `Err`; it returns defaults on any failure. | Yes | `persistence.rs:56-67` + `test_corrupted_file_fallback` (`persistence.rs:131-138`). |
| 3 | `MenuManager` settings round-trip: `apply_settings` makes `settings_modified()` false; `revert_settings` restores the baseline. | Yes | tests `test_menu_manager_apply_settings`, `test_menu_manager_settings_modified` (`menu.rs:474-505`). |
| 4 | `UiLayer::on_event` returns egui's `consumed` so the caller can gate game input. | Yes (contract) | `layer.rs:69-72`; consumer must honor it (`ui_menu_demo/src/main.rs:930`). |
| 5 | egui composites over the existing frame (does not clear it): the paint pass uses `LoadOp::Load`. | Yes | `layer.rs:131`. |
| 6 | `colorblind_mode` index round-trips between `state::Accessibility` string and the `draw_ui` combo index. | Yes | `test_colorblind_mode_index_roundtrip` (`panels.rs:423-444`). |

---

## 9. Performance & Resource Profile

Immediate-mode UI re-emitted every frame; no persistent retained scene. Per-frame cost is egui layout + tessellation + a single wgpu render pass (`layer.rs:101-152`). There are two criterion bench targets — `ui_benchmarks` and `ui_adversarial` (`Cargo.toml:30-36`) — but no MASTER_BENCHMARK figures were cross-checked for this trace. The crate holds no large GPU resources of its own beyond egui's texture atlas managed by `egui_wgpu::Renderer`. The cinematics dev panel keeps `Timeline`/`Sequencer` in process-wide `OnceLock<Mutex<…>>` statics (`panels.rs:241-248`) — global, not per-instance.

---

## 10. Testing & Validation

- **Unit tests (in-crate):** Each module has a `#[cfg(test)] mod tests` (e.g. `menu.rs:353-559`, `menus.rs:630-687`, `panels.rs:368-599`, `persistence.rs:97-138`, `layer.rs:185-336`, `state.rs:47-123`). `menus.rs`/`panels.rs` tests run egui frames headlessly via a `run_frame` helper.
- **`#[cfg(test)] mod mutation_tests`** declared in `lib.rs:34-35` (`mutation_tests.rs`, 1,193 LoC).
- **Integration / dedicated test files** under `astraweave-ui/tests/`: `hud_tests`, `hud_logic_tests`, `hud_priority1/2/3_tests`, `menu_tests`, `menus_tests`, `panel_tests`, `input_tests`, `integration_ui_rendering`, `nan_infinity_tests`, `mutation_hardening_tests`, `mutation_resistant_comprehensive_tests`, plus `tests/fixtures/mod.rs`.
- **Mutation testing:** Multiple mutation-oriented suites present (`mutation_tests.rs`, `mutation_hardening_tests.rs`, `mutation_resistant_comprehensive_tests.rs`). [Coverage figures not cross-checked for this trace.]
- **Miri / Kani:** Not listed among the Miri-validated crates in CLAUDE.md (`ecs`, `math`, `core`, `sdk`). The crate's only `unsafe` is the `UiLayer` lifetime transmute (`layer.rs:140-145`) with a `// SAFETY:` comment. Verified — `astraweave-ui` is in **neither** verification pipeline: `.github/workflows/miri.yml` runs Miri only on `astraweave-ecs`/`-core`/`-physics`/`-ai` (`miri.yml:34-173`), and `.github/workflows/kani.yml` runs Kani only on `astraweave-ecs`/`-math`/`-sdk`/`-core` (`kani.yml:46,67,88,109`). `ci.yml`'s references to `astraweave-ui` (`ci.yml:180,295,317`) are plain build/test jobs, not Miri/Kani. This `unsafe` transmute is therefore not exercised under any formal-verification pipeline.
- **Manual validation:** `examples/ui_controls_demo` (Path A) and `examples/ui_menu_demo` (Path B) are the runnable integration surfaces.

---

## 11. Open Questions / Parked Decisions

- **Are `accessibility` (`AccessibilitySettings`/`ColorblindMode`/`transform_color`) and `gamepad` (`GamepadManager`) intended to be wired, or are they in-design-but-tested scaffolding?** Both are exported and tested but have zero non-test consumers, and `accessibility::` is not used inside `hud.rs`. Per CLAUDE.md Key Lesson 8 they currently classify as "in-design-but-tested." Resolution needed: wire them into the HUD/render path or mark them dormant.
- **Why are two byte-identical backup files (`menus_broken.rs`, `menus_backup2.rs`) retained on disk?** Both were added in `2468b25f1` and never touched since; neither is module-declared. [Reasoning not recovered.]
- **Should the unimported `astraweave-input` dependency remain declared?** ARCHITECTURE_MAP §5 and input.md §11 already flag this as a declared-but-unused dep; the crate uses gilrs directly via `gamepad.rs`.
- **Does the settings-menu key-rebind ever complete?** The UI sets `rebinding_key` but no in-crate code captures the next key press to write the binding. Is the capture expected from a caller, and does any caller implement it? (`ui_controls_demo`/`ui_menu_demo` not confirmed to.)
- **Does `MenuAction::ApplySettings` ever push graphics settings to a renderer/window?** Currently only TOML is written (`menu.rs:320-329`, with a "In future" comment). Is a renderer-apply path planned?
- **Is `astraweave-ui` reachable from `hello_companion`?** It is an optional dependency (`hello_companion/Cargo.toml:29`) but no `use astraweave_ui` was found in its `src/`. [NEEDS VERIFICATION.] *(Verification note 2026-06-24: the dep is gated behind the `visual` feature (`Cargo.toml:54-71`); a workspace grep for `astraweave_ui`/`UiLayer`/`HudManager`/`MenuManager`/`draw_ui` across `hello_companion/src/` returns zero hits even on the `visual` path — `chat_ui.rs`/`visual_demo.rs` use raw egui. So it is a declared-but-unused dep on every build path. The parked question of whether it should remain declared is left to the owner.)*

---

## 12. Maintenance Notes

**Update this doc when:**
- A campaign touches any Active file in §5 (especially `hud.rs`, `menu.rs`, `menus.rs`, `panels.rs`, `layer.rs`).
- The `menus_broken.rs` / `menus_backup2.rs` residue is deleted or one is promoted (update §5, §6).
- `accessibility` or `gamepad` gain a real workspace consumer (flip their status in §4/§5/§6 from dormant to wired).
- The `astraweave-input` dependency is removed or actually imported.
- `MenuAction::ApplySettings` gains renderer/window plumbing (update §7 decision + §6 trap).

**Verification process:**
- Re-grep `mod menus`/`mod menu` in `lib.rs` to confirm which menu files compile.
- Re-grep `HudManager`/`UiLayer`/`draw_ui`/`GamepadManager`/`AccessibilitySettings` across `examples/` and `tools/` to re-confirm wired-vs-dormant status.
- Spot-check the two integration paths in §2 against `ui_controls_demo` (Path A) and `ui_menu_demo` (Path B).
- Stamp the new commit hash and date in the Metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. Menu changes go in `menus.rs` — NOT `menus_broken.rs` / `menus_backup2.rs` (those are uncompiled byte-identical residue).
2. There are two integration paths and two "settings" UIs (§2, §3). Identify which one the caller uses before changing behavior.
3. `accessibility.rs` and `gamepad.rs` are exported but have no production consumers — treat them as dormant/in-design unless you are the one wiring them.
4. `astraweave-input` is a declared-but-unused dependency; the crate's gamepad code is its own `gamepad.rs` (gilrs direct).

**Files you'll most likely touch:**
- `astraweave-ui/src/hud.rs` (HUD content/rendering)
- `astraweave-ui/src/menus.rs` (menu draw functions)
- `astraweave-ui/src/menu.rs` (menu state machine + settings types)
- `astraweave-ui/src/panels.rs` (`draw_ui` aggregate)

**Files you should NOT touch without strong reason:**
- `astraweave-ui/src/layer.rs` — holds the only `unsafe` (egui-wgpu lifetime transmute); changing it risks UB.
- `astraweave-ui/src/menus_broken.rs`, `astraweave-ui/src/menus_backup2.rs` — uncompiled residue; do not add features here.

**Common mistakes when changing this system:**
- Editing a backup menu file and seeing no effect (it's not in the build).
- Assuming settings "Apply" changes resolution/quality at runtime (it only writes TOML).
- Assuming key rebinding completes inside the crate (it sets a flag; capture is the caller's job).
- Reaching for `accessibility::AccessibilitySettings` thinking it is what `draw_ui` uses (it uses `state::Accessibility`).

---

## Appendix B: Historical context

Module doc-comments label features by an incremental "Week N Day M" campaign (e.g. HUD foundation "Week 3 Day 1", health animation "Week 4 Day 1", audio callbacks "Week 5 Day 2"), indicating the UI was built feature-by-feature over a multi-week sprint. The canonical `menus.rs` and its two byte-identical backups (`menus_broken.rs`, `menus_backup2.rs`) all entered the tree in a single commit `2468b25f1` ("Replace Phi3 with Hermes2Pro and add UI fixes…"), suggesting the backups were saved alongside an edit-in-progress and never cleaned up. The `accessibility` and `gamepad` modules were exported into the public API but, as of this trace, never picked up by a workspace consumer.
