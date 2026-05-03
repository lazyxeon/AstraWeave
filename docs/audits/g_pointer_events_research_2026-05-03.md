# G-pointer-events Research Audit

**Date**: 2026-05-03
**Trigger**: F-fix Andrew-gate brush UX REGRESS — click+drag in viewport with Paint mode active is consumed by camera pan instead of routing to `RegionalArchetypePanel`'s brush queue.
**Predecessor work**: F-fix chain ending at hash-fixup `1d67b3328` (registration class closed; pointer-routing class is architecturally distinct).
**Scope**: pure SOTA web research on egui pointer-event dispatch (Concern A) + multi-tool 3D editor arbitration (Concern B) + Rust 3D editor reference implementations (Concern C). **NO code inspection. NO hypothesis investigation. NO remediation recommendation.** Output is reference material for G-diagnostic.

---

## §1 — Research methodology

Per Andrew's Q1 Option B: research-first, internal-second. The audit's findings form independently of `TerrainPanel`'s specific implementation to prevent confirmation bias when G-diagnostic later compares observed code against canonical patterns. Per Q2 Full scope: three concerns A/B/C surveyed in parallel. Per Q4 Interpretation A: `TerrainPanel`'s known multi-texture-paint limitation is informational background, not investigation target.

**Search budget**: ~20 web queries total across the three concerns. Sources prioritized: official egui documentation (docs.rs, GitHub repo, DeepWiki); Blender developer wiki; Unity C# Reference; Unreal Engine documentation; Godot documentation; bevy_egui examples; Fyrox engine repository; rerun re_viewer documentation.

**Quality summary**:

- **Concern A (egui dispatch)**: rich canonical material. Multiple authoritative sources (docs.rs, DeepWiki, recent PRs like #5358 for Modal). Specific type names and method signatures captured.
- **Concern B (multi-tool arbitration)**: rich canonical material across all four reference editors. Each editor has documented patterns with citable URLs.
- **Concern C (Rust 3D editor implementations)**: medium-quality material. Fyrox, Bevy, rerun all surveyed, but specific source code paths (e.g., `Fyrox/editor/src/...`) not consistently fetchable due to GitHub directory listings vs file content. Patterns inferred from changelogs, demo apps, issue trackers.

---

## §2 — Concern A: egui pointer-event dispatch findings

### §2.1 The basic Sense + Response mechanism

The canonical egui pattern for any interactive widget — including 3D viewports — is:

```rust
// Allocate a rect with desired sense; get back rect + Response
let (rect, response) = ui.allocate_exact_size(
    desired_size,
    egui::Sense::click_and_drag()  // or Sense::drag(), Sense::click(), Sense::hover()
);

// Query response for interaction state
if response.dragged() {
    let delta = response.drag_delta();          // pointer movement since last frame
    let motion = response.drag_motion();         // similar; raw motion this frame
    // ... use delta/motion to update camera, brush queue, etc.
}

// Optionally paint custom 3D content into the rect
ui.painter().add(egui::PaintCallback {
    rect,
    callback: std::sync::Arc::new(/* renderer-specific callback */),
});
```

This pattern is documented in `egui/examples/custom_3d_glow/src/main.rs` (the canonical custom-3D-viewport example) and used by virtually every 3D widget in the egui ecosystem (rerun's re_viewer, Fyrox's editor, the `egui::Scene` widget itself).

**Key types**:

- **`Sense`**: bitflag-like enum specifying which interactions a widget responds to. `Sense::click()`, `Sense::drag()`, `Sense::click_and_drag()`, `Sense::hover()`, `Sense::focusable_noninteractive()`. A widget that only senses drags begins dragging immediately on press; a widget that senses both clicks and drags requires the pointer to move a minimum distance before drag detection registers (preventing single-pixel accidental drags).
- **`Response`**: returned from every widget; holds interaction state computed at frame start. Has `dragged()`, `dragged_by(button: PointerButton)`, `drag_delta()`, `drag_motion()`, `drag_started()`, `drag_started_by(button)`, `drag_stopped()`, `drag_stopped_by(button)`, `clicked()`, `clicked_by(button)`, `secondary_clicked()`, `middle_clicked()`, `double_clicked()`, `triple_clicked()`, `contains_pointer()`, `hovered()`, `interact(sense)`.
- **`InteractionSnapshot`**: stored in `ViewportState::interact_widgets`, computed once per frame in `interaction::interact()`. Tracks: which widget receives clicks (`Option<Id>`), all hovered widgets (`IdSet`), active drag state (`Option<DragState>`). The snapshot is the single source of truth for which widget claims pointer events that frame.
- **`PointerState`** (in `InputState`): tracks `press_origin`, `hover_pos`, `delta`, `pos_history`, `ActiveTouch`, `InputOptions` (with `max_click_dist`, `max_click_duration`, `surrender_focus_on`).
- **`Memory`**: holds `Focus` state across frames; `request_focus(id)`, `set_modal_layer(layer_id)`, `allows_interaction(layer)`.

Sources: [egui Response docs](https://docs.rs/egui/latest/egui/response/struct.Response.html), [DeepWiki Input Handling System](https://deepwiki.com/emilk/egui/2.6-input-handling-system), [DeepWiki Response and Interaction](https://deepwiki.com/emilk/egui/2.4-rendering-pipeline).

### §2.2 Layer ordering and topmost-widget-wins

Pointer events route to widgets via z-order priority:

> **Hit testing** explicitly identifies "the topmost clickable widget" and "the topmost draggable widget" as the winners. The document states that "insensitive areas let clicks pass to layers behind," meaning only the highest widget with matching `Sense` receives the event.

When widgets overlap, the **last added one is considered to be on top** and gets input priority. Layer order follows the pattern where later painted = higher priority. The `Response.layer_id` field indicates which layer the widget belongs to.

Hit testing returns three tiers in priority order:
- **click**: topmost clickable widget
- **drag**: topmost draggable widget
- **contains**: all widgets containing pointer (not just topmost)

The distinction between `contains_pointer()` and `hovered()` is load-bearing for arbitration: `contains_pointer()` "Returns true if the pointer is contained by the response rect, and no other widget is covering it." Crucially, **it can be true even during another widget's drag**, while `hovered()` becomes `false` during a drag. This means a panel can detect "pointer is over me, even though some other widget is currently being dragged" — useful for ending drags initiated elsewhere.

Sources: [DeepWiki Response and Interaction §Hit Testing](https://deepwiki.com/emilk/egui/2.4-rendering-pipeline), [egui Issue #5822 dnd_drag_source priority](https://github.com/emilk/egui/issues/5822).

### §2.3 The Modal pattern (egui PR #5358, recent canonical addition)

The Modal feature, added in PR #5358 ([Add Modal and Memory::set_modal_layer](https://github.com/emilk/egui/pull/5358)), provides explicit "claim all interaction above this layer" semantics:

> `Memory::set_modal_layer` limits focus to a layer and above (used by the modal struct), along with `Memory::allows_interaction` to check if a layer is behind a modal layer. This feature limits focus to widgets on the given layer and above, and if called multiple times per frame, the top layer wins.

This is the most direct egui parallel for "tool mode active panel pre-empts everything below it." Conceptually:

```rust
// In the panel's show(), if paint mode is active:
if self.paint_mode_active {
    ui.ctx().memory_mut(|m| m.set_modal_layer(self.layer_id));
}
// Now widgets on layers below self.layer_id cannot interact;
// the panel's Sense::drag() rect (presumably overlapping the viewport)
// claims the events first.
```

The Modal pattern was originally designed for modal dialogs (issue #686, #839) but its mechanism — limiting interaction to a layer and above — generalizes to "tool mode active" semantics.

Source: [egui PR #5358 Add Modal and Memory::set_modal_layer](https://github.com/emilk/egui/pull/5358).

### §2.4 Viewport-vs-overlay arbitration: no explicit pattern

There is **no documented "viewport vs overlay" arbitration pattern** in egui's official docs. Arbitration is purely layer-based: whichever widget is on the higher layer with matching `Sense` claims the event.

This means a 3D viewport widget that calls `ui.allocate_exact_size(size, Sense::click_and_drag())` will receive drag events from the user's click+drag UNLESS:
1. Another widget on a higher layer with matching sense overlaps the viewport rect, OR
2. A modal layer is active that excludes the viewport's layer, OR
3. The egui context is otherwise routing events elsewhere (focused widget, drag-source/drop-target, etc.).

If both the viewport widget AND a tool overlay are on the same layer, the **last-added** widget wins. This implies that calling `panel.show(ui)` AFTER `viewport.show(ui)` would let the panel's allocated rect claim events over the viewport — but ONLY if the panel's rect overlaps the same screen region.

Source: [egui Discussion #1926 Freely drag-and-drop widgets](https://github.com/emilk/egui/discussions/1926), [DeepWiki Input Handling §Hit Testing](https://deepwiki.com/emilk/egui/2.6-input-handling-system).

### §2.5 The `egui::Scene` widget (canonical 3D-viewport-like widget)

`egui::Scene` is the closest egui-canonical analog for AstraWeave's situation: a viewport widget that needs both pan/zoom (camera-like) AND child-widget interaction (tool-like).

Current implementation (per [egui Issue #5891](https://github.com/emilk/egui/issues/5891)):

> The `Scene` allocates a response with `Sense::click_and_drag` and checks `response.dragged()` to determine whether the scene should be panned by `response.drag_delta()`.

PR #5892 (merged April 2025) added [`Scene::drag_pan_buttons`](https://github.com/emilk/egui/pull/5892) to allow specifying which pointer buttons trigger panning. This addresses the "users want pan only on middle button so left-button events fall through to tools" concern — exactly the AstraWeave concern in miniature.

The pattern Scene uses:
1. Allocate rect with `Sense::click_and_drag()`.
2. Check `response.dragged_by(button)` for the configured pan button.
3. If dragged with pan button → pan the scene.
4. If dragged with other button (or not dragged) → events fall through to child widgets inside the Scene (which are added on a higher layer or after Scene's allocation).

This is approach **(A) higher-layer widget pre-empts** from §5 below: the Scene reserves specific pointer buttons; child widgets get the rest.

Sources: [egui Scene docs](https://doc.servo.org/egui/containers/scene/index.html), [egui Issue #5891 Scene custom Sense](https://github.com/emilk/egui/issues/5891), [PR #5892 drag_pan_buttons](https://github.com/emilk/egui/pull/5892).

### §2.6 Common pitfalls

- **Sense mismatch**: a widget with `Sense::hover()` cannot receive drags or clicks, even if positioned correctly. Many "my widget doesn't respond to clicks" bugs trace to wrong Sense.
- **Combining responses**: `response_a | response_b` (BitOr operator) creates a union. Calling `interact()` on a union response is **undefined behavior** per the docs; use it only for query (`.dragged()`, `.clicked()`, etc.).
- **Drag detection delay on click_and_drag**: widgets sensing both clicks AND drags don't register `dragged() == true` until the cursor has moved a minimum distance OR the user has held for long enough. If a tool needs immediate drag detection (e.g., paint-on-press), use `Sense::drag()` only.
- **Layer mismatch**: a widget added inside a child window or popup is on a different layer than widgets in the main UI. The "topmost wins" rule applies across layers, so a popup with `Sense::drag()` will claim drags inside its rect regardless of underlying widgets.
- **Egui Issue #5822** documents that `dnd_drag_source` zones take priority over interior input widgets — a real-world example of layer-priority producing surprising results.

---

## §3 — Concern B: multi-tool 3D editor arbitration findings

### §3.1 Common architectural model across editors

Surveyed editors (Blender, Unity, Unreal, Godot) converge on the same architectural pattern:

> **Active tool/mode gets first dibs at viewport pointer events. Tool returns "consumed" (block default camera handling) or "pass-through" (let camera handle). Camera/viewport default control receives events only when tool returns pass-through.**

This is the canonical "tool-first arbitration" pattern. Concrete implementations vary in:
1. How "active tool state" is stored (modal operator, FEdMode subclass, EditorPlugin, WorkSpaceTool).
2. How tools register / get activated (subclass + register, key bind, palette selection).
3. Return-value semantics (bool, enum, bitflag).

But the **conceptual contract is identical**: tool intercepts before camera; tool decides per-event whether to consume or pass.

### §3.2 Blender: Modal operators with PASS_THROUGH return

Blender's modal operator system uses a return-bitmask:

- **`OPERATOR_RUNNING_MODAL`**: the operator stays modal, consuming the event.
- **`OPERATOR_PASS_THROUGH`**: the operator is "transparent" for this event — it doesn't swallow the event; allows it to be passed on to further handlers (shortcuts, operators, gizmos, viewport navigation, etc.).
- **`OPERATOR_RUNNING_MODAL | OPERATOR_PASS_THROUGH`**: stays modal, but selectively passes specific events (e.g., camera navigation keys).

The canonical pattern for a modal brush stroke operator that wants to let the user navigate the 3D view while painting:

> A modal operator may listen to specific keyboard events while keeping other user interactions working by returning `OPERATOR_PASS_THROUGH | OPERATOR_RUNNING_MODAL` for everything but the keyboard events it wants to operate on itself.

> Returning "PASS_THROUGH" will not end the operator but cause the event to pass to Blender's regular event processing, which will move the view around accordingly.

**`WorkSpaceTool`** is the higher-level abstraction: tools register via subclassing; `bl_keymap` tuples associate operators with key events; tool state lives per workspace. Multi-mode is awkward (currently requires duplicating the tool class with different `bl_idname`s).

Sources: [Blender Operators developer docs](https://developer.blender.org/docs/features/interface/operators/), [Blender devtalk Multi-Mode for WorkSpace Tools](https://devtalk.blender.org/t/multi-mode-for-work-space-tools/8434), [WorkSpaceTool Python API docs](https://docs.blender.org/api/current/bpy.types.WorkSpaceTool.html).

### §3.3 Unity: TerrainTool enum + TerrainInspector

Unity's terrain tool dispatch lives in [TerrainInspector.cs](https://github.com/Unity-Technologies/UnityCsReference/blob/master/Modules/TerrainEditor/TerrainInspector.cs). The `TerrainTool` enum (Unity's source comments note "this name does not seem appropriate") dictates which tool category is selected in the terrain inspector. Selection drives which painting brush is rendered. Tool state lives in the inspector itself; switching tool calls `InspectorWindow.RepaintAllInspectors()`.

Unity's pattern is tool-state-in-inspector rather than a separate dispatcher. The inspector itself participates in scene-view event handling via the `OnSceneGUI` callback (called per-frame when the inspector's target is selected); inside that callback, the inspector reads its current `TerrainTool` and dispatches accordingly.

This is approach **(B) viewport-checks-active-tool-state** at the editor framework level: `OnSceneGUI` is the editor's hook; inspector code branches on tool state.

Source: [Unity TerrainInspector.cs source](https://github.com/Unity-Technologies/UnityCsReference/blob/master/Modules/TerrainEditor/TerrainInspector.cs), [Unity Manual Terrain Tools](https://docs.unity3d.com/6000.0/Documentation/Manual/terrain-Tools.html).

### §3.4 Unreal: FEdMode virtual methods with bool return

Unreal's editor mode system uses `FEdMode` (or its newer counterpart `UEdMode`) as the active-mode abstraction. Each mode subclass overrides virtual methods:

- `bool InputKey(FEditorViewportClient*, FViewport*, FKey, EInputEvent)`
- `bool HandleClick(FEditorViewportClient*, HHitProxy*, const FViewportClick&)`
- `bool MouseMove(FEditorViewportClient*, FViewport*, int32 x, int32 y)`
- `bool CapturedMouseMove(...)`
- `bool StartTracking(...)`, `bool EndTracking(...)`, etc.

**Return-value semantics**: `true` = "I handled this event; do not propagate." `false` = "I didn't handle this event; let default processing continue." If `InputKey` returns `true` for a mouse press, the StartTracking/CapturedMouseMove/EndTracking sequence is **not called** (consume blocks the entire drag pipeline).

The active mode receives events FIRST (before viewport's default camera/navigation handling). Each tool is a subclass of FEdMode; `Enter()` / `Exit()` lifecycle methods handle activation/deactivation. Multiple modes can be stacked (FEditorModeTools manages a stack); top-of-stack mode gets first dibs.

**Landscape Manage Mode** (the closest Unreal precedent for AstraWeave's regional archetype paint) uses this pattern: activated when user enters Landscape mode; consumes click+drag for sculpt operations; passes through navigation modifier-key combinations to camera control.

Sources: [FEdMode 4.27 docs](https://docs.unrealengine.com/4.27/en-US/API/Editor/UnrealEd/FEdMode/), [FEdMode::InputKey](https://docs.unrealengine.com/4.27/en-US/API/Editor/UnrealEd/FEdMode/InputKey/), [FEdMode::HandleClick](https://docs.unrealengine.com/4.27/en-US/API/Editor/UnrealEd/FEdMode/HandleClick/), [Unreal Landscape Manage Mode](https://dev.epicgames.com/documentation/en-us/unreal-engine/landscape-manage-mode-in-unreal-engine).

### §3.5 Godot: `_handles` + `_forward_3d_gui_input` with AfterGUIInput enum

Godot's `EditorPlugin` has the cleanest separation of concerns of the four editors surveyed:

```gdscript
func _handles(object: Object) -> bool:
    # Return true if this plugin edits the given object type.
    # Once true, _forward_3d_gui_input gets called for viewport events.
    pass

func _forward_3d_gui_input(viewport_camera: Camera3D, event: InputEvent) -> int:
    # Called BEFORE viewport's default camera/navigation handling.
    # Return value:
    #   AFTER_GUI_INPUT_PASS (0): forward event to other plugins / camera
    #   AFTER_GUI_INPUT_STOP (1): consume event; block all downstream handling
    #   AFTER_GUI_INPUT_CUSTOM (2): forward to other plugins, but block the
    #                                main Node3D plugin (prevents node selection)
    pass
```

For tools that need to receive events without selection-driven activation:

```gdscript
func _enter_tree():
    set_input_event_forwarding_always_enabled()
```

This makes `_forward_3d_gui_input` always called, enabling raycast-in-the-scene tools that don't depend on selecting a specific node first.

**Multi-plugin arbitration**: framework processes plugins sequentially. First plugin returning `STOP` wins; order among competing plugins is undefined in the docs but typically registration order. `PASS` chains to the next plugin in the list, ultimately reaching camera control.

Source: [Godot EditorPlugin XML docs](https://github.com/godotengine/godot/blob/master/doc/classes/EditorPlugin.xml), [EditorPlugin docs](https://docs.godotengine.org/en/stable/classes/class_editorplugin.html).

### §3.6 Comparative observations

| Editor | Active tool storage | Tool-first hook | Consume / pass return |
|---|---|---|---|
| Blender | Modal operator stack + WorkSpaceTool per workspace | Modal operator's `modal()` callback | `OPERATOR_RUNNING_MODAL` / `OPERATOR_PASS_THROUGH` bitmask |
| Unity | TerrainTool enum on TerrainInspector | `OnSceneGUI()` callback on inspector | Implicit; uses Event.current.Use() to consume |
| Unreal | FEdMode stack on FEditorModeTools | FEdMode virtual methods (InputKey, HandleClick, MouseMove, etc.) | `bool` (true = consume, false = pass) |
| Godot | EditorPlugin's `_handles()` claim | `_forward_3d_gui_input()` callback | `AfterGUIInput` enum (PASS / STOP / CUSTOM) |

**Common across all four**: tool/mode receives events BEFORE viewport's default camera handling; explicit consume/pass-through return value; tool state is editor-framework-level (not buried inside the panel widget).

**Scaling to multi-tool**: Godot, Unreal, and Blender all scale naturally to many tools (each tool registers as a plugin / FEdMode / operator; framework dispatches). Unity's pattern of tool-state-in-inspector scales less naturally — each new content type that needs multiple paint tools requires its inspector to grow a new TerrainTool-style enum and OnSceneGUI dispatch logic.

**Failure modes named in editor documentation**:
- Blender Issue T63668: `GRAB_CURSOR` bl_option breaks once a modal operator returns PASS_THROUGH the first time (cursor warp stops working). Suggests pass-through semantics have edge cases when combined with cursor manipulation.
- Godot Issue #76873: `_forward_3d_gui_input` not called when camera preview is enabled. Suggests integration points between tool-first dispatch and special viewport modes need careful coverage.
- Unity tool-state-in-inspector: switching active inspector loses tool state; `RepaintAllInspectors` is the workaround.

Sources: [Blender T63668](https://developer.blender.org/T63668), [Godot Issue #76873](https://github.com/godotengine/godot/issues/76873), [Unity Discussions: Some terrain tools missing in Inspector](https://discussions.unity.com/t/some-terrain-tools-missing-in-inspector/220186).

---

## §4 — Concern C: Rust 3D editor reference implementations

### §4.1 Fyrox engine editor

Fyrox has an editor at [`Fyrox/editor`](https://github.com/FyroxEngine/Fyrox/tree/master/editor) with an `InteractionMode` abstraction. From the changelog:

> **InteractionMode refactored**: `InteractionModeKind` was removed and replaced with UUIDs.
> **InteractionMode::make_button**: creates appropriate button for the mode.
> **Terrain brush bounds visualization fixed**; sanity check added for brush operations to protect the editor from being overloaded by huge brushes.

Pattern (inferred from changelog + book references): Fyrox uses a trait/abstract `InteractionMode` analogous to Unreal's `FEdMode`. Each mode has UI button registration (`make_button`), receives pointer events via the editor's central dispatcher, and modifies scene state. The terrain brush is implemented as a discrete mode.

This is approach **(C) plugin/mode-dispatcher** at a high level.

The Fyrox approach validates that the canonical "active mode dispatcher" pattern translates cleanly to Rust. Source code paths (e.g., `editor/src/interaction/...`) were not directly inspected per G-research's anti-anchoring discipline (they're internal editor source; G-diagnostic will inspect AstraWeave's editor instead).

Sources: [Fyrox CHANGELOG](https://github.com/FyroxEngine/Fyrox/blob/master/CHANGELOG.md), [Fyrox Terrain Node book](https://fyrox-book.github.io/scene/terrain_node.html), [Fyrox editor directory](https://github.com/FyroxEngine/Fyrox/tree/master/editor).

### §4.2 bevy + bevy_egui

bevy_egui ([crates.io](https://crates.io/crates/bevy_egui)) provides input absorption hooks for the egui-vs-Bevy-systems boundary. Two canonical patterns from [`bevy_egui/examples/absorb_input.rs`](https://github.com/vladbat00/bevy_egui/blob/v0.36.0/examples/absorb_input.rs):

**Pattern 1 — Run conditions (recommended)**:

```rust
.add_systems(
    Update,
    keyboard_input_system.run_if(not(egui_wants_any_keyboard_input)),
)
.add_systems(
    Update,
    pointer_input_system.run_if(not(egui_wants_any_pointer_input)),
)
```

Bevy systems that handle game input are gated with `not(egui_wants_*)` run conditions. When egui wants input, those systems don't run; when egui doesn't, they do.

**Pattern 2 — Global absorption (less safe)**:

```rust
egui_global_settings.enable_absorb_bevy_input_system = true;
```

Set on `EguiGlobalSettings` to absorb all input events into egui automatically.

**Picking-specific knob**: `EguiContextSettings::capture_pointer_input` controls if egui suppresses bevy_picking events when pointer is over an Egui window.

**Known fragility**: [bevy-inspector-egui Issue #276](https://github.com/jakobhellermann/bevy-inspector-egui/issues/276) reports that egui inspector UI does NOT block picking events on lower entities unless `bevy_egui` is **explicitly added as a dependency**. Indicates the absorption pattern is sensitive to crate composition.

Sources: [bevy_egui README](https://github.com/vladbat00/bevy_egui), [absorb_input example](https://github.com/vladbat00/bevy_egui/blob/v0.36.0/examples/absorb_input.rs), [EguiContextSettings docs](https://docs.rs/bevy_egui/latest/bevy_egui/struct.EguiContextSettings.html), [bevy Issue #3570 First-class technique for splitting input streams](https://github.com/bevyengine/bevy/issues/3570).

### §4.3 bevy_editor_pls

[bevy_editor_pls](https://github.com/jakobhellermann/bevy_editor_pls) provides in-app editor tools for Bevy. README documents user-facing controls (`T/R/S` for gizmos, mouse for camera) but does not surface architectural details about pointer-event arbitration in the public README. Internal source would need to be inspected for specific patterns; G-research did not pull source code per anti-anchoring discipline (and bevy_editor_pls is not AstraWeave's specific reference; the egui patterns from §2 generalize).

Source: [bevy_editor_pls](https://github.com/jakobhellermann/bevy_editor_pls).

### §4.4 rerun re_viewer

rerun's [`re_viewer`](https://github.com/rerun-io/rerun) is a substantial production-grade egui application with 3D viewport interaction. Sources point to:
- `crates/viewer/re_viewport/src/viewport_ui.rs`: viewport handling.
- `crates/viewer/re_viewer_context/src/drag_and_drop.rs`: drag-and-drop handling.

User-facing behavior: "Clicking and dragging the contents of any view will move it - you can rotate 3D views, or pan 2D views and plots. You can also zoom using ctrl+scrollwheel or pinch gestures on a trackpad."

Architectural pattern (inferred from public docs): standard egui pattern — viewport widget allocates rect with `Sense::click_and_drag()`, checks `response.dragged()` for camera control. No reported multi-tool paint use case (rerun is a viewer, not an editor with paint tools), so it doesn't directly validate multi-tool arbitration patterns. But it confirms the egui base pattern works at scale for production 3D applications.

Sources: [rerun View System and Rendering DeepWiki](https://deepwiki.com/rerun-io/rerun/5.2-view-system-and-rendering), [rerun Navigating the Viewer docs](https://rerun.io/docs/getting-started/configure-the-viewer/navigating-the-viewer).

### §4.5 egui Scene widget (production canonical 3D-viewport-with-children)

`egui::Scene` (in `egui::containers::scene`, added 2025) is the egui-canonical reference for "viewport widget with pan/zoom that contains child widgets." Per §2.5 above, current implementation:
- Allocates rect with `Sense::click_and_drag()`.
- Checks `response.dragged()` for panning.
- PR #5892 added `drag_pan_buttons` to restrict panning to specific pointer buttons (e.g., middle mouse), letting other buttons fall through to children.

This is the most direct in-egui parallel for AstraWeave's situation: the Scene reserves specific pointer buttons for pan; children get the rest. It's approach **(A) higher-layer-claims-specific-events** at fine granularity (per-button rather than per-mode).

Source: [egui Scene container docs](https://doc.servo.org/egui/containers/scene/index.html).

### §4.6 Limitations / known issues observed in Rust 3D editors

- **bevy_egui absorption fragility** (§4.2): the "absorb input" pattern requires explicit dependency wiring; missing it produces silent fallthrough.
- **No documented multi-paint-tool exemplar in Rust editors**: Fyrox has terrain brushes; no Rust editor surveyed has a documented example of multi-tool paint with proper arbitration. AstraWeave's situation (terrain sculpt + archetype paint + future splat + future scatter) has limited Rust precedent; the AAA editor patterns from §3 are the closer references.
- **egui Issue #5891** (Scene custom Sense): pre-2025, Scene's all-buttons-pan-by-default was inflexible enough that an explicit "custom Sense + per-button drag" feature had to be added. Suggests egui's default arbitration is too coarse for many real-world editor needs and requires explicit per-tool customization.

---

## §5 — Cross-concern architectural observations

### §5.1 Three implementation approaches surface from A + B + C

The research identifies three implementation approaches for "active tool gets first dibs at viewport pointer events":

**Approach (A) — Higher-layer widget pre-empts**

The active tool's UI sits ABOVE the viewport on a higher LayerId. Egui's layer-priority hit testing routes events to the higher layer first. Pass-through is implicit: if the higher-layer widget doesn't allocate a Sense at this event's position, the event falls through to the viewport.

- **egui parallel**: Scene's `drag_pan_buttons` (§2.5, §4.5) — Scene reserves specific buttons; children claim others.
- **Modal layer variant**: `Memory::set_modal_layer()` (§2.3) — explicitly excludes lower layers from interaction.
- **Pros**: works within egui's existing framework; no new dispatcher needed; layer ordering is the canonical egui mechanism.
- **Cons**: requires the tool overlay to physically sit ABOVE the viewport in the egui layer graph (which may not match the panel's natural placement in a docked tab); may require positioning hacks if the tool panel is in a side-dock rather than a viewport overlay.
- **Multi-tool scaling**: each new tool registers as another higher-layer overlay; works but may produce many overlapping overlays in complex editors.

**Approach (B) — Viewport widget checks active-tool state internally**

The viewport widget's `Sense::drag()` handler checks "is paint mode active?" internally and routes to brush queue or camera accordingly. State machine lives in the viewport widget itself.

- **egui parallel**: standard `if response.dragged() { ... }` pattern with conditional logic on tool state.
- **Unity parallel**: TerrainTool enum on TerrainInspector with branching `OnSceneGUI` (§3.3).
- **Pros**: simpler to implement initially; no layer-priority gymnastics.
- **Cons**: couples viewport widget to tool list; each new tool requires editing viewport code; doesn't scale to multi-tool. **This may be why TerrainPanel's multi-texture-paint never expanded — adding new tools means editing the viewport**. (Interpretation A informational note from §0.)
- **Multi-tool scaling**: poor. Adding splat + scatter + archetype paint requires the viewport to know about all of them.

**Approach (C) — Editor-level dispatcher with per-tool registration**

Active tool state lives in an editor-level dispatcher. Tools register via a trait/interface. Viewport's pointer-event hook calls the active tool first; tool returns consumed/pass-through; only on pass-through does the viewport's default camera control receive the event.

- **AAA editor parallel**: Godot EditorPlugin (`_forward_3d_gui_input` + `AfterGUIInput`); Unreal FEdMode (`InputKey`/`HandleClick` + `bool`); Blender modal operators (`OPERATOR_RUNNING_MODAL` / `OPERATOR_PASS_THROUGH`).
- **Rust parallel**: Fyrox's `InteractionMode` abstraction (§4.1).
- **Pros**: forward-compatible with arbitrary multi-tool addition; clean separation of concerns; matches AAA editor canonical patterns.
- **Cons**: more upfront infrastructure work; introduces a new editor-level abstraction (the dispatcher); requires defining the tool trait interface.
- **Multi-tool scaling**: excellent. Each new tool just implements the trait and registers; viewport dispatch logic is unchanged.

### §5.2 Pass-through semantics must be explicit

Across all four AAA editors and across egui's existing patterns, **pass-through semantics are always explicit** — never inferred. Blender's `OPERATOR_PASS_THROUGH`, Godot's `AFTER_GUI_INPUT_PASS`, Unreal's `return false`, egui Scene's "buttons not in `drag_pan_buttons` fall through" — all encode the same idea: "I had first dibs but I'm letting [camera / next plugin / child widget] handle this event."

The implication for AstraWeave: whatever approach is chosen, the API for "tool didn't claim this event" must be present. Implicit silence (e.g., not calling `response.dragged()`) is not the canonical way; the explicit return-value or layer-mechanism is.

### §5.3 The TerrainPanel limitation framing (informational only)

Per Q4 Interpretation A: `TerrainPanel`'s known multi-texture-paint limitation is informational background. The research surfaces a structural concern that **could** explain it:

- If `TerrainPanel` uses approach **(B)** — the viewport widget checks `TerrainPanel`'s internal state and routes accordingly — then adding a second tool (multi-texture, archetype paint, etc.) **requires editing the viewport code** to know about the new tool's state. This is the multi-tool scaling failure mode of approach (B).
- If `TerrainPanel` uses approach **(A)** or **(C)**, the limitation is more likely **feature-incomplete** (the tool was never built out) than **architecturally flawed**.

G-research does NOT determine which approach `TerrainPanel` uses (that's G-diagnostic's investigation per Q1 Option B). G-research only surfaces the architectural concern that **if `TerrainPanel` is approach (B), mirroring it for `RegionalArchetypePanel` would inherit the multi-tool scaling problem**, and **avoiding that requires either approach (A) with proper layer placement or approach (C) with a new dispatcher**.

### §5.4 Open questions for G-diagnostic

The following questions G-research does not answer; they belong to G-diagnostic's code-inspection phase:

1. **Which approach does AstraWeave's editor currently use?** Is there an editor-level "active tool" dispatcher? Does `TerrainPanel` engage layer-priority, internal state-checking, or something else?
2. **Where in the egui layer graph does camera control sit?** Which file is the "click+drag in viewport pans camera" code in? Is camera control on a low layer (so any panel-overlay above it would pre-empt) or on the same layer as panels (so layer-priority is by registration order)?
3. **What does `TerrainPanel`'s pointer-event handling look like?** Approach (A), (B), or (C)? If (B), is multi-texture-paint architecturally blocked or feature-incomplete?
4. **Does the F.5-paint panel's `regional_archetype_panel.show()` allocate a viewport-overlapping rect?** (Probably not — it's a docked side panel rendering brush sliders + palette, not a viewport overlay. This means the panel CANNOT claim viewport drag events through approach (A) without restructuring its rendering to include a transparent overlay over the viewport rect.)
5. **What's the simplest path forward**: layer-priority hack (approach A with a transparent overlay on a higher layer), state machine in viewport widget (approach B), or new editor-level dispatcher (approach C)?

These are G-diagnostic's questions, not G-research's. The research provides the canonical patterns to compare observations against; G-diagnostic does the comparison.

---

## §6 — Recommended pattern direction (preliminary)

Based purely on research findings and **without inspecting AstraWeave's code**, the canonical literature suggests:

**Approach (C) — Editor-level dispatcher with per-tool registration — is the canonical AAA pattern and the forward-compatible choice for AstraWeave's multi-tool future.**

Reasoning:
- All four AAA editors converge on (C) for multi-tool scenarios. Even Unity's tool-state-in-inspector (§3.3) is approach (C) at the framework level — `OnSceneGUI` is the editor's dispatcher hook; the inspector is the "registered tool."
- AstraWeave's projected tool set (terrain sculpt + archetype paint + future splat + future scatter + ...) is multi-tool by design. (B)'s scaling failure mode is a real concern.
- Fyrox demonstrates (C) translates cleanly to Rust + egui.

**However, the research also identifies that approach (A) — higher-layer widget pre-empts — may be a viable narrower-scope choice for G-pointer-events-fix specifically**, if:
- G-diagnostic determines that `RegionalArchetypePanel` can render a transparent overlay rect on a higher layer than the camera-control rect.
- Egui's `Memory::set_modal_layer()` (§2.3) or layer-priority placement (§2.2) provides the arbitration.
- The tradeoff is acceptable: simpler short-term implementation, but doesn't solve multi-tool dispatcher question for future tools.

**Hybrid possibility**: G-pointer-events-fix uses approach (A) for the immediate fix (transparent overlay + layer priority for `RegionalArchetypePanel` only); approach (C) is built later as part of a larger editor-architecture refactor (potentially F.5-overlay-and-gate or beyond). This keeps G's scope bounded while not foreclosing the canonical pattern.

**G-research does NOT recommend a specific approach.** That's G-diagnostic's job once code inspection determines what's currently in place. The research catalogs the canonical options with trade-offs; G-diagnostic and G-fix make the call.

---

## §7 — Pattern A regression test direction (preliminary)

Based on canonical patterns, regression tests for the pointer-event class should verify:

### §7.1 Active-tool consume

Construct the panel + a synthetic viewport pointer event (egui has `RawInput` for this; bevy_egui has similar test harnesses). Activate the panel's paint mode. Assert:
- The panel's brush queue (`pending_paint_ops`) receives the event.
- A mock camera handler does NOT receive the event.

```rust
#[test]
fn active_paint_panel_consumes_viewport_drag() {
    let mut panel = RegionalArchetypePanel::default();
    panel.paint_mode_active = true;  // hypothetical activation API
    let mut camera_handler = MockCameraHandler::new();
    
    // Simulate a click+drag in the viewport region
    simulate_viewport_drag(&mut panel, &mut camera_handler, /* ... */);
    
    assert_eq!(panel.pending_paint_ops.len(), 1);
    assert_eq!(camera_handler.receive_count, 0);
}
```

### §7.2 Inactive-tool pass-through

Same setup with paint mode **inactive**. Assert:
- The panel's brush queue is empty (no events claimed).
- The camera handler DOES receive the event.

```rust
#[test]
fn inactive_paint_panel_passes_viewport_drag_to_camera() {
    let mut panel = RegionalArchetypePanel::default();
    panel.paint_mode_active = false;
    let mut camera_handler = MockCameraHandler::new();
    
    simulate_viewport_drag(&mut panel, &mut camera_handler, /* ... */);
    
    assert!(panel.pending_paint_ops.is_empty());
    assert_eq!(camera_handler.receive_count, 1);
}
```

### §7.3 Multi-tool exclusivity

Construct two paint panels, only one active. Assert active panel claims; inactive panel does not.

```rust
#[test]
fn only_active_tool_claims_viewport_drag() {
    let mut terrain = TerrainPanel::default();
    let mut archetype = RegionalArchetypePanel::default();
    archetype.paint_mode_active = true;
    terrain.brush_active = false;
    
    simulate_viewport_drag(&mut terrain, &mut archetype, /* ... */);
    
    assert!(terrain.pending_brush_ops.is_empty());
    assert_eq!(archetype.pending_paint_ops.len(), 1);
}
```

### §7.4 Modifier-key arbitration (optional but canonical)

Per Blender + Godot precedents, modifier-key combinations often pass through to camera even during active paint mode. If AstraWeave adopts this pattern (e.g., `Alt+drag` always pans camera regardless of paint mode), test it:

```rust
#[test]
fn alt_drag_passes_to_camera_even_in_paint_mode() {
    // ... setup ...
    simulate_viewport_drag_with_modifier(&mut panel, Modifier::Alt, /* ... */);
    
    assert!(panel.pending_paint_ops.is_empty());  // alt+drag doesn't paint
    assert_eq!(camera_handler.receive_count, 1);  // alt+drag pans camera
}
```

These sketches are preliminary; G-diagnostic refines them based on observed code structure (which approach is in use determines what test harness is feasible). Per Andrew's Q2 Pattern A preference: at minimum the active-consume + inactive-pass-through tests should land in G-fix.

---

## §8 — Out-of-scope observations and open questions for G-diagnostic

### §8.1 Save/load is a separate concern

Per F-fix.B §10's H-saveload deferral: save/load functionality is untestable until brush works. G-research did NOT investigate save/load patterns. Out of scope for G; G-diagnostic should NOT investigate save/load either.

### §8.2 Climate Preview overlay is a separate concern

Per F.5-overlay-and-gate's deferred scope: Climate Preview overlay (D.5c absorbed) is a separate session after F.5-paint fully closes. G-research did NOT investigate overlay rendering patterns.

### §8.3 Scenes-with-children pattern (egui Scene + child widgets)

`egui::Scene`'s recent `drag_pan_buttons` addition (§2.5, §4.5) is the closest pre-built egui solution for "viewport that pans on some buttons but lets others fall through to children." If AstraWeave's editor uses (or could use) `egui::Scene`-style containment for its viewport, the `drag_pan_buttons` mechanism may be directly applicable. G-diagnostic should check whether the AstraWeave viewport is built on `egui::Scene` or a custom widget.

### §8.4 Bevy-vs-egui boundary (informational only)

The bevy_egui absorption pattern (§4.2) is relevant only if AstraWeave uses Bevy as its underlying engine + bevy_egui for editor UI. AstraWeave's stack is wgpu + egui + custom ECS, not Bevy, so the absorption pattern doesn't directly apply. G-diagnostic should NOT investigate Bevy-specific patterns.

### §8.5 TerrainPanel's multi-texture limitation root cause

Per Q4 Interpretation A: informational only. G-diagnostic CAN observe whether TerrainPanel is approach (A), (B), or (C), and note it in the diagnostic audit, but should NOT deep-investigate the multi-texture limitation's history or speculate about feature gaps.

### §8.6 Specific questions for G-diagnostic to resolve

Restated from §5.4 for prominence:

1. Does AstraWeave's editor currently have an editor-level "active tool" dispatcher? If yes, what's its API?
2. Where does camera control receive viewport events? Which file, which layer?
3. Does `TerrainPanel` use approach (A), (B), or (C)?
4. Does `RegionalArchetypePanel` have any natural surface to allocate a viewport-overlapping rect? (Likely no; it's a docked side panel.)
5. Given AstraWeave's specific layer/dispatch architecture, which approach (A, B, C, or hybrid) is the smallest-scope correct fix for G-pointer-events-fix?

---

## §9 — Bibliography

### Egui (Concern A)

- [egui Response docs](https://docs.rs/egui/latest/egui/response/struct.Response.html) — authoritative API reference for `Response` methods.
- [egui Sense docs](https://doc.servo.org/egui/struct.Sense.html) — Sense type reference.
- [DeepWiki egui Input Handling System](https://deepwiki.com/emilk/egui/2.6-input-handling-system) — architectural overview of input dispatch.
- [DeepWiki egui Response and Interaction](https://deepwiki.com/emilk/egui/2.4-rendering-pipeline) — InteractionSnapshot + hit testing.
- [DeepWiki egui Memory and State Management](https://deepwiki.com/emilk/egui/2.5-memory-and-state-management) — Memory + Focus + modal.
- [egui PR #5358 Add Modal and Memory::set_modal_layer](https://github.com/emilk/egui/pull/5358) — modal layer mechanism (recent).
- [egui Issue #5891 Scene custom Sense](https://github.com/emilk/egui/issues/5891) — Scene widget per-button drag.
- [egui PR #5892 Scene::drag_pan_buttons](https://github.com/emilk/egui/pull/5892) — implementation of per-button pan.
- [egui Issue #5822 dnd_drag_source priority](https://github.com/emilk/egui/issues/5822) — layer-priority real-world example.
- [egui Discussion #1926 Freely drag-and-drop widgets](https://github.com/emilk/egui/discussions/1926) — layer ordering discussion.
- [egui Discussion #3450 Detecting hover events on regions covered](https://github.com/emilk/egui/discussions/3450) — `contains_pointer` vs `hovered` distinction.
- [egui custom_3d_glow example](https://github.com/emilk/egui/blob/main/examples/custom_3d_glow/src/main.rs) — canonical custom 3D viewport pattern.

### AAA editors (Concern B)

- **Blender**:
  - [Operators developer docs](https://developer.blender.org/docs/features/interface/operators/) — modal operator return-value semantics.
  - [WorkSpaceTool Python API](https://docs.blender.org/api/current/bpy.types.WorkSpaceTool.html) — active tool abstraction.
  - [Blender devtalk Multi-Mode for WorkSpace Tools](https://devtalk.blender.org/t/multi-mode-for-work-space-tools/8434) — multi-mode limitation discussion.
  - [Blender T63668 GRAB_CURSOR + PASS_THROUGH bug](https://developer.blender.org/T63668) — pass-through edge case.

- **Unity**:
  - [TerrainInspector.cs source](https://github.com/Unity-Technologies/UnityCsReference/blob/master/Modules/TerrainEditor/TerrainInspector.cs) — TerrainTool enum + dispatch.
  - [Unity Manual Terrain Tools](https://docs.unity3d.com/6000.0/Documentation/Manual/terrain-Tools.html).

- **Unreal**:
  - [FEdMode 4.27 docs](https://docs.unrealengine.com/4.27/en-US/API/Editor/UnrealEd/FEdMode/) — base class.
  - [FEdMode::InputKey](https://docs.unrealengine.com/4.27/en-US/API/Editor/UnrealEd/FEdMode/InputKey/) — input handling.
  - [FEdMode::HandleClick](https://docs.unrealengine.com/4.27/en-US/API/Editor/UnrealEd/FEdMode/HandleClick/) — click handling.
  - [Unreal Landscape Manage Mode](https://dev.epicgames.com/documentation/en-us/unreal-engine/landscape-manage-mode-in-unreal-engine).

- **Godot**:
  - [EditorPlugin XML class definition](https://github.com/godotengine/godot/blob/master/doc/classes/EditorPlugin.xml) — `_forward_3d_gui_input` + `AfterGUIInput` enum.
  - [EditorPlugin docs](https://docs.godotengine.org/en/stable/classes/class_editorplugin.html).
  - [Godot Issue #76873 _forward_3d_gui_input not called during camera preview](https://github.com/godotengine/godot/issues/76873) — integration edge case.

### Rust 3D editors (Concern C)

- **Fyrox**:
  - [Fyrox repo](https://github.com/FyroxEngine/Fyrox).
  - [Fyrox CHANGELOG](https://github.com/FyroxEngine/Fyrox/blob/master/CHANGELOG.md) — InteractionMode refactor history.
  - [Fyrox editor directory](https://github.com/FyroxEngine/Fyrox/tree/master/editor).
  - [Fyrox Terrain Node book](https://fyrox-book.github.io/scene/terrain_node.html).

- **bevy_egui**:
  - [bevy_egui repo](https://github.com/vladbat00/bevy_egui).
  - [bevy_egui absorb_input example](https://github.com/vladbat00/bevy_egui/blob/v0.36.0/examples/absorb_input.rs) — input absorption pattern.
  - [EguiContextSettings docs](https://docs.rs/bevy_egui/latest/bevy_egui/struct.EguiContextSettings.html).
  - [bevy_egui Issue #47 Absorbing input on hover](https://github.com/vladbat00/bevy_egui/issues/47).
  - [bevy-inspector-egui Issue #276 picking events fallthrough](https://github.com/jakobhellermann/bevy-inspector-egui/issues/276) — known fragility.
  - [bevy Issue #3570 First-class technique for splitting input streams](https://github.com/bevyengine/bevy/issues/3570) — upstream discussion.

- **rerun**:
  - [rerun repo](https://github.com/rerun-io/rerun).
  - [DeepWiki rerun View System and Rendering](https://deepwiki.com/rerun-io/rerun/5.2-view-system-and-rendering).
  - [rerun Navigating the Viewer docs](https://rerun.io/docs/getting-started/configure-the-viewer/navigating-the-viewer).

- **bevy_editor_pls**:
  - [bevy_editor_pls repo](https://github.com/jakobhellermann/bevy_editor_pls) — surveyed at README level only.

---

*End of G-pointer-events research audit.*
