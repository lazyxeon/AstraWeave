<!--
  Input System page — replaced 2026-05-15 as part of the post-trace-campaign
  reconciliation.
  Source: ARCHITECTURE_MAP.md §7.1 (Documentation Hazards) and `input.md` trace.
  Pre-trace version (commit 28bc94f21, 2025-09-08, automated documentation pass)
  described InputSystem / InputConfig / ActionMap / BindingRecorder / BindingProfile /
  ContextPriority / InputBuffer / InputPredictor / InputRecorder and submodules
  mapping / rebinding / replay / buffer / device. None of those types or submodules
  exist in astraweave-input/src/lib.rs re-exports. Actual API is Action / Binding /
  BindingSet / InputManager / Axis2.
-->

# Input System

```admonish warning title="Documentation under reconciliation"
This page was rewritten on 2026-05-15 to reflect the engineering reality surfaced by the
architecture trace campaign. A prior version (added in commit 28bc94f21,
2025-09-08, by an automated documentation pass) described `InputSystem`, `InputConfig`,
`ActionMap`, `BindingRecorder`, `BindingProfile`, `ContextPriority`, `InputBuffer`,
`InputPredictor`, `InputRecorder` and a `mapping` / `rebinding` / `replay` / `buffer` /
`device` submodule layout. **None of those types or submodules exist** in
`astraweave-input/src/lib.rs` re-exports.
```

## Actual public surface

<!-- Source: input.md §1, §5 -->

`astraweave-input` is a pure facade over `winit` (window/keyboard/mouse events) and
`gilrs` (gamepad). The `lib.rs` re-exports are:

* `Action` — high-level input action enum.
* `Binding` — single key/button/axis-to-action mapping.
* `BindingSet` — collection of bindings (load / save / merge).
* `InputManager` — runtime state, polls device events, drains an action queue.
* `Axis2` — 2D axis value (gamepad sticks, mouse delta).

That is the complete external surface. There is **no** `InputSystem`, no `InputConfig`,
no `BindingRecorder`, no `InputBuffer`, no `InputPredictor`, no `InputRecorder`.

## Declared-but-unused dependency hazard

<!-- Source: ARCHITECTURE_MAP.md §2.3 anomaly 8 + input.md §4, §11 -->

Two workspace crates declare `astraweave-input` as a Cargo dependency but **never
import it** in any source file:

* `astraweave-gameplay/Cargo.toml`
* `astraweave-ui/Cargo.toml`

Verified by workspace-grep for `use astraweave_input`. The dep additions left no
source-file imports in git history. This is documented in `input.md` §4 and §11
and tracked as Q21 in `ARCHITECTURE_MAP.md` §14.

The single in-tree consumer of `astraweave-input` is `examples/ui_controls_demo`,
and even that demo does not read the `InputManager` state — it `match`es raw
`winit` `KeyCode` directly.

## Editor reinvents the input domain

<!-- Source: input.md §6 (§7.7 wrapped-component trap) and aw_editor.md §6 -->

The visual editor's input-bindings panel
(`tools/aw_editor/src/panels/input_bindings_panel.rs`, 2,511 LoC, 13 types)
reimplements the entire input vocabulary in-place, **without depending on
`astraweave-input`**. This is one of the four confirmed instances of the §7.7
wrapped-component resource identity trap surfaced by the Editor Multi-Tool
Architecture campaign — the editor's input layer and the engine's input layer
manage the same logical resource but neither delegates to the other.

Tracked as Q22 in `ARCHITECTURE_MAP.md` §14.

## Silent-failure shape

<!-- Source: ARCHITECTURE_MAP.md §4.3 -->

`load_bindings` in `astraweave-input/src/save.rs:16-19` collapses every error
mode (file not found, parse error, schema mismatch) to `None`. Users get a default
binding set with no diagnostic indication of what failed.

## Where to actually look in the code

| Need | File |
|------|------|
| Public re-exports | `astraweave-input/src/lib.rs` |
| Action & Binding types | `astraweave-input/src/` |
| Editor's parallel input domain | `tools/aw_editor/src/panels/input_bindings_panel.rs` |
| In-tree consumer (raw KeyCode usage) | `examples/ui_controls_demo/src/main.rs` |

## Further reading

* [`input.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/input.md) — full input-system trace (file map, conflict map, decision log,
  invariants, open questions).
* [`ARCHITECTURE_MAP.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/ARCHITECTURE_MAP.md) §2.3 (anomaly 8), §7.1, §14 (Q21, Q22).
* **Interactive workspace map** — select `astraweave-input` to see the
  declared-but-unused dependency edges from `astraweave-gameplay` and
  `astraweave-ui` rendered as dashed/tee-terminated lines.
