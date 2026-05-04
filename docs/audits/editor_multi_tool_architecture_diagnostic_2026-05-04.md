# Editor Multi-Tool Architecture — Sub-phase 1 Diagnostic Audit

**Status**: Audit COMPLETE 2026-05-04. **All ten §2.X architectural commitments verified COMPATIBILITY-CONFIRMED.** Zero gap-evidence findings; 2 open-question findings (deferred). **No Andrew-gate triggered. Sub-phase 2 prompt drafting + execution may proceed with §2 commitments intact.**
**Author**: Sub-phase 1 Diagnostic session 2026-05-04.
**Scope**: Pre-implementation feasibility audit verifying §2's ten architectural commitments against AstraWeave editor code per Editor Multi-Tool Architecture campaign doc §3 + Q7 strategic factor.
**Predecessors**: Editor Multi-Tool Architecture campaign doc (`docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md`; commits `75b68e7c7` Design.A + `8fad61bd3` Design.B + `8c92890b9` Design.C hash-fixup) + research audit (`docs/audits/editor_multi_tool_architecture_research_2026-05-03.md`) + G-research/G-diagnostic + F.5-paint.E-diagnostic methodology revision.

---

## §1 — Methodology

### §1.1 Verification intent

Per campaign doc §0.1 + Q7 strategic factor: §2 decisions are research-informed (research audit §7 framework + Andrew Q1-Q10 strategic factors) but not AstraWeave-validated. This audit validates them against AstraWeave editor code before Sub-phase 2-5 + Mediator Removal session begin implementation.

The audit's value: prevent Sub-phases 2-5 + Mediator Removal session from discovering architectural mismatches mid-implementation (predecessor failure mode that produced Regional Archetype Variation's pause + this entire spinoff campaign).

### §1.2 Three-mode evidence framework (per Andrew Q2)

Each §2.X decision verified against one of three evidence modes:

- **Compatibility evidence**: code citations showing AstraWeave's current state is compatible with the §2 commitment. **Does NOT trigger Andrew-gate.**
- **Gap evidence**: code citations showing AstraWeave's current state has a structural feature that conflicts with the §2 commitment. **Triggers Andrew-gate at audit completion per Q4.**
- **Open-question evidence**: compatibility depends on context this audit can't determine without inspection beyond Sub-phase 1's scope. **Does NOT trigger Andrew-gate.**

### §1.3 Inspection scope per Q3 (c)

**Editor-only inspection** for §2.2-§2.8 + §2.10 (architecture-internal decisions):
- `tools/aw_editor/src/main.rs` — Editor::new() + main.rs:3833-3877 mediator code.
- `tools/aw_editor/src/viewport/widget.rs` — ViewportWidget struct + handle_input + per-tool fields.
- `tools/aw_editor/src/panels/terrain_panel.rs` — TerrainPanel struct + brush API.
- `tools/aw_editor/src/panels/regional_archetype_panel.rs` — RegionalArchetypePanel struct + paint API.
- `tools/aw_editor/src/tab_viewer/mod.rs` — EditorTabViewer + per-tool accessors.
- `tools/aw_editor/src/panel_type.rs` — PanelType enum.
- `tools/aw_editor/src/dock_panels.rs` — DockPanelContext + dock dispatch.

**Editor-adjacent inspection allowed** for §2.9 (compositional) + §2.11 (transactional):
- `tools/aw_editor/src/command.rs` — `EditorCommand` trait + `UndoStack` (located via grep; verified for §2.11 integration target).

### §1.4 Methodology inheritance from predecessor diagnostics

Per F.5-paint.E-diagnostic §3 methodology revision + G-diagnostic §7 four-option decision-request precedent: precedent-driven discovery (find existing call sites + similar patterns; assess compatibility) rather than first-principles enumeration. Code citation form: `<file>:<line-range>` references throughout.

---

## §2 — §2.2 ActiveTool trait shape verification

### §2.2.1 Verification target

Per campaign doc §2.2: trait surface mirrors Fyrox InteractionMode pattern — ~10 per-event default-implementation methods (mouse press/up/move/enter/leave + keyboard down/up + UI message) + 4 lifecycle methods (activate/deactivate/update/on_drop) + UI integration (`make_button`) + UUID identity (`uuid() -> Uuid` per Q5 mod-friendliness).

Default-implementation pattern: each per-event method defaults to `EventDisposition::PassThrough`; tools override only relevant methods.

### §2.2.2 AstraWeave evidence

**TerrainPanel API surface mapping** (inspected at [terrain_panel.rs:797-882](../../tools/aw_editor/src/panels/terrain_panel.rs#L797)):

- `is_brush_active(&self) -> bool` at [line 797](../../tools/aw_editor/src/panels/terrain_panel.rs#L797): returns `self.brush_enabled && self.terrain_state.has_terrain()`. Maps to dispatcher's `active_tool == Some(TERRAIN_PANEL_UUID)` invariant — when brush is enabled + terrain exists, tool is active. Lifecycle methods `activate()` / `deactivate()` set internal `brush_enabled` flag; dispatcher transitions enforce mutex.
- `apply_brush_at(&mut self, world_x: f32, world_z: f32)` at [line 819](../../tools/aw_editor/src/panels/terrain_panel.rs#L819): receives world-XZ coordinates; auto-begins stroke; routes to brush mode (Sculpt/Paint/Lower/Smooth/Flatten/Erode/Noise/ZoneBlend). Maps cleanly to ActiveTool's `on_left_mouse_button_down` (stroke start) + `on_mouse_move` (continuation) handlers — both call `apply_brush_at(world_x, world_z)` with the world coordinates from `ToolContext::world_xz_at_pointer()` per §2.7.
- `end_brush_stroke(&mut self) -> Option<Vec<...>>` at [line 864](../../tools/aw_editor/src/panels/terrain_panel.rs#L864): returns undo deltas. Maps cleanly to ActiveTool's `on_left_mouse_button_up` handler — calls `end_brush_stroke()` and emits `TerrainBrushCommand` (per §2.11 integration target).
- Getters (`brush_mode_name`, `brush_radius`, `is_paint_mode`) at [lines 871-882](../../tools/aw_editor/src/panels/terrain_panel.rs#L871): tool-internal state for UI integration; map to ActiveTool's `make_button` method (TerrainPanel provides its own toolbar widget) + `update` method for per-frame state polling if needed.

**RegionalArchetypePanel API surface mapping** (inspected at [regional_archetype_panel.rs:75-279](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L75)):

- `paint_active: bool` field at [line 75](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L75): currently unused per F.5-paint.E-diagnostic §9.6 forward observation. Maps to ActiveTool's `activate()` lifecycle — set `paint_active = true` in `activate()`; `paint_active = false` in `deactivate()`. The field's anticipatory documentation aligns with ActiveTool semantics.
- `queue_paint_op(&mut self, world_x: f32, world_z: f32)` at [line 138](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L138): receives world-XZ; pushes PaintOp to `pending_paint_ops` queue. Maps cleanly to ActiveTool's `on_left_mouse_button_down` + `on_mouse_move` handlers — both call `queue_paint_op(world_x, world_z)` (matching TerrainPanel.apply_brush_at signature exactly).
- `apply_pending_paint_ops_to_owned(&mut self)` at [line 233](../../tools/aw_editor/src/panels/regional_archetype_panel.rs#L233): drains queue + applies ops + calls `recompute_falloff()` once. Maps cleanly to ActiveTool's `on_left_mouse_button_up` handler — flushes queue at stroke end.

**Compatibility synthesis**: both TerrainPanel and RegionalArchetypePanel have API surfaces that map structurally to the proposed ActiveTool trait shape. The signature `fn(&mut self, world_x: f32, world_z: f32)` is identical across `apply_brush_at` and `queue_paint_op`, confirming the §2.2 trait surface is correctly grain-sized for AstraWeave's panel patterns. UUID identity for both is straightforward (constants `TERRAIN_PANEL_UUID` and `REGIONAL_ARCHETYPE_PANEL_UUID` per §2.5).

### §2.2.3 Verification finding

**COMPATIBILITY CONFIRMED.** Both TerrainPanel and RegionalArchetypePanel API surfaces map cleanly to the §2.2 ActiveTool trait shape. The Fyrox-style per-event default-implementation pattern + lifecycle methods + UI integration via `make_button` + UUID identity all align with existing panel patterns.

---

## §3 — §2.3 EventDisposition enum semantics verification

### §3.1 Verification target

Per campaign doc §2.3: binary `Consumed` / `PassThrough` at campaign close per Q4. Enum declared `#[non_exhaustive]` for forward-compatibility (future `ConsumedSelective` variant for hover-feedback tools per Godot 4 CUSTOM-analog precedent without breaking consumers using match guards).

### §3.2 AstraWeave evidence

**Existing event-handling pattern in ViewportWidget** (inspected at [viewport/widget.rs:1167-1255](../../tools/aw_editor/src/viewport/widget.rs#L1167) per G-diagnostic findings):

- Camera orbit at line 1167 (middle-button drag): unconditional; not gated on tool state.
- Camera orbit at line 1180 (primary-button drag): gated `!self.terrain_brush_active` — if terrain brush is active, primary-drag does NOT route to camera. This is functionally a binary "tool consumed" / "camera passes through" semantic.
- Terrain brush dispatch at line 1200: gated `self.terrain_brush_active && (dragged_by(Primary) || clicked_by(Primary))` — when active, tool handles primary events; else passes through to camera (which is gated negatively).

**Compatibility synthesis**: existing ViewportWidget handle_input pattern uses binary tool-consumed-or-camera-pass-through semantics implicitly. The §2.3 binary `Consumed` / `PassThrough` enum makes this explicit + adds `#[non_exhaustive]` for future variant additions (Godot 4 CUSTOM-analog).

No tri-state semantics found in existing code. The middle-button drag (line 1167; unconditional camera) is conceptually tri-state ("camera always handles, regardless of tool"), but in practice this is just "passes through to camera handler always" — the dispatcher's `active_tool` doesn't see middle-button events because they're handled before dispatch reaches the active tool. Post-§2 implementation, ViewportWidget calls `dispatcher.dispatch_mouse_event` only for primary-button events (or with button discrimination passed through to the active tool); middle-button camera control remains in ViewportWidget's existing handler.

### §3.3 Verification finding

**COMPATIBILITY CONFIRMED.** Existing ViewportWidget event-handling has binary tool-consumed-or-camera-pass-through semantics implicitly. The §2.3 binary `Consumed` / `PassThrough` enum + `#[non_exhaustive]` for forward extensibility maps directly to existing patterns. No tri-state semantics conflict.

---

## §4 — §2.4 Dispatcher mechanism verification

### §4.1 Verification target

Per campaign doc §2.4: push-based per-event subscription per research audit §7.2 + §7.7. Dispatcher tracks active tool by UUID; `HashMap<Uuid, Box<dyn ActiveTool>>` collection; per-event method calls only on active tool.

### §4.2 AstraWeave evidence

**egui input flow in ViewportWidget** (inspected at [viewport/widget.rs:439-515](../../tools/aw_editor/src/viewport/widget.rs#L439) per G-research §2.5 inheritance + G-diagnostic findings):

The viewport allocates `Sense::click_and_drag()` at line 439 and uses egui's `Response` API:
- `response.clicked()` — discrete click event.
- `response.dragged_by(button)` — discrete drag event per button.
- `response.drag_delta()` — per-frame drag delta during drag.
- `response.drag_stopped_by(button)` — discrete drag-end event.
- `response.hover_pos()` — current pointer position when hovered.

These are **discrete event-shaped queries**. Each frame, ViewportWidget can determine which discrete events occurred (click, drag start, drag continuing, drag end) and dispatch each to the active tool via push-based method calls.

**Compatibility synthesis**: egui's `Sense::click_and_drag()` + `Response` API is structurally compatible with push-based per-event dispatch. ViewportWidget's `handle_input` builds events from egui's response, calls `dispatcher.dispatch_mouse_event(event, MouseEventKind::*, &mut context)` for each discrete event detected per frame, and returns `EventDisposition` to drive subsequent camera handler decisions.

The pattern matches Fyrox's InteractionMode dispatch (research audit §5.1) and avoids Unity's pull-based `OnToolGUI` pattern (research audit §3.2 documented performance + correctness issues).

### §4.3 Verification finding

**COMPATIBILITY CONFIRMED.** egui's `Sense::click_and_drag()` + `Response` API produces discrete events that map cleanly to push-based per-event dispatch. No pull-based-only constraint surfaces. `HashMap<Uuid, Box<dyn ActiveTool>>` is structurally additive (no existing tool registry pattern blocks it).

---

## §5 — §2.5 Registration model verification

### §5.1 Verification target

Per campaign doc §2.5: explicit `register_tool(Box<dyn ActiveTool>)` API at editor init per Q5 mod-friendliness. UUID open-set identity (third-party UUIDs don't conflict with first-party).

### §5.2 AstraWeave evidence

**Editor::new() panel construction pattern** (inspected at [main.rs:472-580](../../tools/aw_editor/src/main.rs#L472)):

```rust
// Excerpt from EditorApp::new():
// Line 482: entity_manager: EntityManager::new(),
// Line 485: undo_stack: command::UndoStack::new(100),
// Line 492: world_panel: WorldPanel::new(),
// Line 493: entity_panel: EntityPanel::new(),
// ... (each panel constructed via XxxPanel::new())
// Line 574: dock_tab_viewer: EditorTabViewer::new(),
```

Pattern: each panel is a struct field on `EditorApp`; constructed via `XxxPanel::new()` calls during `EditorApp::new()`. There is no central tool registry; panels are field-by-field allocations. The `EditorTabViewer` (at line 574, full struct in [tab_viewer/mod.rs:495+](../../tools/aw_editor/src/tab_viewer/mod.rs#L495) per F.5-paint.F-fix.A registration) holds panel instances for tab dispatching.

**PanelType enum** (inspected at [panel_type.rs:107-228](../../tools/aw_editor/src/panel_type.rs#L107) per F.5-paint.F-fix.A registration): closed-set enum of ~40 PanelType variants. Used for tab dispatch + View/Window menu population. NOT used as a tool registration mechanism (panels register via struct fields on EditorTabViewer; PanelType is the tab identifier).

**Compatibility synthesis**: adding `dispatcher: Dispatcher` as a new field on EditorApp + calling `dispatcher.register_tool(Box::new(TerrainPanel::default()))` etc. at construction is structurally additive. No existing pattern conflicts. The `register_tool` call sites land alongside existing `XxxPanel::new()` constructions; first-party tools register at fixed UUIDs (constants); third-party tools generate their own UUIDs (open-set per Q5).

PanelType enum is ORTHOGONAL to ActiveTool registration. PanelType identifies tabs (where panels render); ActiveTool identifies tools (what receives viewport input events). Some panels are both (TerrainPanel, RegionalArchetypePanel), some are only PanelType (Hierarchy, Inspector). The dispatcher registers the subset of panels that are tools.

### §5.3 Verification finding

**COMPATIBILITY CONFIRMED.** Editor::new() pattern is structurally additive for `register_tool` calls. No existing tool registry pattern blocks adoption. PanelType enum and ActiveTool registration are orthogonal concerns. UUID open-set identity is straightforward (random UUID generation; first-party uses constants; third-party generates).

---

## §6 — §2.6 Mediator pattern fate verification

### §6.1 Verification target

Per campaign doc §2.6: replace completely; dedicated Mediator Removal session per Q6. main.rs:3833-3877 mediator code + ViewportWidget per-tool fields removed.

### §6.2 AstraWeave evidence

**main.rs mediator code extent** (per G-diagnostic findings; verified via grep this session): mediator code at [main.rs:3833-3877](../../tools/aw_editor/src/main.rs#L3833) is 44 lines (pre-render terrain brush sync + post-render hit drain + stroke-end detection + TerrainBrushCommand emission).

**ViewportWidget per-tool fields extent** (verified via grep this session at [viewport/widget.rs:163-177](../../tools/aw_editor/src/viewport/widget.rs#L163)):

- `terrain_brush_active: bool` (line 163)
- `terrain_brush_radius: f32` (line 166)
- `terrain_brush_is_paint: bool` (line 168)
- `terrain_brush_hits: Vec<[f32; 2]>` (line 171)
- `terrain_brush_stroke_ended: bool` (line 174)
- `last_brush_time: std::time::Instant` (line 177)

6 fields total; default initialization at lines 264-268 + 318-322; setters at lines 352-359 + 363-368.

**ViewportWidget terrain_brush handle_input branching extent** (per G-diagnostic findings): 4 branch sites at lines 1180, 1200, 1258, 1263. Lines 1167-1255 region per G-diagnostic.

**tab_viewer accessor extent** (verified via grep this session at [tab_viewer/mod.rs:1340-1425](../../tools/aw_editor/src/tab_viewer/mod.rs#L1340)):

- `is_terrain_brush_active(&self) -> bool` (line 1340)
- `apply_terrain_brush_at(&mut self, x, z)` (line 1400)
- `end_terrain_brush_stroke(&mut self) -> Option<...>` (line 1406)
- `terrain_brush_mode_name(&self) -> &'static str` (line 1413)
- `terrain_brush_radius(&self) -> f32` (line 1418)
- `terrain_brush_is_paint(&self) -> bool` (line 1423)

6 accessors total.

**Total mediator-related grep count**: 51 occurrences of `terrain_brush` across 3 files (main.rs:10, viewport/widget.rs:35, tab_viewer/mod.rs:6). Bounded scope.

**Compatibility synthesis**: removal scope is well-defined and bounded. main.rs:3833-3877 (44 lines), ViewportWidget 6 fields + setters + 4 handle_input branches, tab_viewer 6 accessors. No references to `terrain_brush` outside these three files (grep verified). Dedicated Mediator Removal session per Q6 has full context budget for careful refactoring without context muddying from other concerns.

### §6.3 Verification finding

**COMPATIBILITY CONFIRMED.** Mediator code extent is bounded (51 grep occurrences across 3 files; 44 lines + 6 fields + 4 branches + 6 accessors). Dedicated session scope per Q6 is appropriately sized. Removal is structurally simple (deletion + ensuring TerrainPanel still works via dispatcher path).

---

## §7 — §2.7 ViewportWidget integration verification

### §7.1 Verification target

Per campaign doc §2.7: ViewportWidget owns rendering + raw input capture + depth-buffer access (preserved from current pattern at viewport/widget.rs:1219-1234); dispatcher owns tool arbitration. Post-Mediator-Removal end-state: no per-tool fields on ViewportWidget.

### §7.2 AstraWeave evidence

**ViewportWidget current responsibilities** (per G-diagnostic findings + this session's struct inspection at [viewport/widget.rs:103-200](../../tools/aw_editor/src/viewport/widget.rs#L103)):

The struct holds:
- Rendering state (renderer, camera, gizmo state, scene state).
- Raw input capture state (last_viewport_rect, focus tracking, brush hit collection).
- Per-tool fields (the 6 `terrain_brush_*` fields per §6 above).

The per-tool fields are the ONLY tool-specific state on ViewportWidget. After Mediator Removal session per §2.6, ViewportWidget retains rendering + raw input capture only.

**Depth-buffer access for ray-plane projection** (inspected at [viewport/widget.rs:1219-1234](../../tools/aw_editor/src/viewport/widget.rs#L1219) per G-diagnostic): existing pattern reads depth buffer at pointer pixel; unprojects to world coordinates. Used by TerrainPanel sculpt brush for surface-following. RegionalArchetypePanel doesn't need depth-buffer (uses ray-plane projection at y=0 plane via F.5-paint.B's `screen_to_world_xz_y0`).

**Compatibility synthesis**: post-removal end-state achievable. ViewportWidget already owns rendering + raw input capture; tool-state coupling is bounded to the 6 fields slated for removal in Mediator Removal session per §2.6. The depth-buffer access pattern at lines 1219-1234 is preserved + exposed via `ToolContext::world_xz_at_pointer()` closure (per §2.7's commitment); tools that need depth access call it; tools that don't use simpler `world_xz_at_y0()` projection.

The §2.7 commitment "ViewportWidget owns rendering + raw input capture; dispatcher owns tool arbitration" is structurally achievable with the existing code post-removal.

### §7.3 Verification finding

**COMPATIBILITY CONFIRMED.** ViewportWidget currently owns rendering + raw input capture + (transient, soon-to-be-removed) per-tool fields. Post-Mediator-Removal end-state has clean separation. Depth-buffer access pattern preserved via ToolContext exposure.

---

## §8 — §2.8 Mutex arbitration semantics verification

### §8.1 Verification target

Per campaign doc §2.8: framework-enforced mutex via dispatcher's `active_tool: Option<Uuid>` field. Single active tool at a time; `set_active_tool` transitions previous deactivate → new activate.

### §8.2 AstraWeave evidence

**Existing tool arbitration pattern**: per main.rs:3834 mediator code, current arbitration is a single boolean (`is_terrain_brush_active`) hardcoded for TerrainPanel. This is functionally a single-active-tool mutex check (just hardcoded instead of generalized). No multi-active-tool semantics found anywhere in editor code (grep for `active_tool` returns only the prospective dispatcher concern; existing code has only `terrain_brush_active`).

**Compatibility synthesis**: §2.8 framework-enforced mutex generalizes the existing single-active-tool pattern. The `active_tool: Option<Uuid>` field is the abstraction that replaces the hardcoded `terrain_brush_active: bool`. Transition from "hardcoded boolean" to "Option<Uuid>" is structurally additive — no multi-active semantics break (none exist).

### §8.3 Verification finding

**COMPATIBILITY CONFIRMED.** Existing single-active-tool pattern (hardcoded `terrain_brush_active`) generalizes cleanly to dispatcher's `active_tool: Option<Uuid>` framework-enforced mutex. No multi-active semantics conflict.

---

## §9 — §2.9 Tool composition rules verification (editor-adjacent inspection allowed)

### §9.1 Verification target

Per campaign doc §2.9: composition deferred to follow-up; current campaign produces single-active-tool dispatcher; trait surface designed to NOT preclude future composition extensions.

### §9.2 AstraWeave evidence

**TerrainPanel BrushMode sub-modes** (inspected at [terrain_panel.rs:509-595](../../tools/aw_editor/src/panels/terrain_panel.rs#L509) per F.5-paint.E-diagnostic): TerrainPanel has 8 brush modes (Sculpt, Lower, Smooth, Flatten, Paint, Erode, Noise, ZoneBlend) accessed via internal `brush_mode: BrushMode` enum. Each mode has different behavior in `apply_brush_at()`.

This is a **sub-tool pattern within a single tool** — TerrainPanel is one ActiveTool with internal sub-modes, NOT 8 separate tools. The campaign-design pass correctly classified this as panel-internal state (per §2.2 "tool-internal state for UI integration") rather than dispatcher-level composition.

**No cross-tool composition patterns found**: grep for cross-panel state references (e.g., TerrainPanel referencing RegionalArchetypePanel state, or vice versa) returns nothing. Panels are independent; no nested-tool / parent-child / tool-of-tools patterns currently exist.

**Forward observation (informational; NOT a gate-triggering gap)**: future splat painting + scatter painting tools per Q1 timeline may surface composition concerns the current trait surface doesn't anticipate. For example, a "splat painter" might want to delegate to multiple sub-painters per material. Current trait design allows this via panel-internal sub-mode pattern (TerrainPanel precedent); explicit composition primitives are deferred per §2.9.

### §9.3 Verification finding

**COMPATIBILITY CONFIRMED.** No cross-tool composition patterns exist in current editor; deferral of composition rules per §2.9 is uncomplicated. TerrainPanel BrushMode sub-modes are correctly classified as tool-internal state, not dispatcher-level composition. Forward observation about future splat/scatter tools documented; not gate-triggering.

---

## §10 — §2.10 Tool state persistence verification

### §10.1 Verification target

Per campaign doc §2.10: per-tool persistence is each tool's responsibility; dispatcher doesn't enforce a persistence pattern.

### §10.2 AstraWeave evidence

**Per-panel state patterns** (inspected at terrain_panel.rs:400-470 + regional_archetype_panel.rs:60-110):

TerrainPanel state (~40 fields):
- Brush settings (radius, strength, mode, falloff, paint material).
- Generation parameters (seed, biome, archetype, chunk_radius, octaves, etc.).
- Erosion parameters.
- Fluid parameters.
- UI state (auto_regenerate, show_advanced).

RegionalArchetypePanel state (~10 fields):
- Brush size + falloff radius.
- Selected archetype + paint mode.
- Mask + path + IO status.

**No central preferences mechanism for per-tool settings**: search for editor-wide preferences mechanism returns `prefs.show_grid` etc. (UI prefs), not tool prefs. Per-tool settings are panel-internal state; no central registration required.

**Compatibility synthesis**: per-tool persistence is straightforwardly each tool's responsibility — TerrainPanel's brush settings live in TerrainPanel; RegionalArchetypePanel's settings live in RegionalArchetypePanel. Adding ActiveTool trait does NOT change this; the trait doesn't enforce a persistence pattern. Future preset save/load UX (per UE `UInteractiveToolPropertySet` reference) is deferred per §2.10 informational note.

### §10.3 Verification finding

**COMPATIBILITY CONFIRMED.** Per-panel state patterns are already each panel's responsibility; ActiveTool trait surface doesn't enforce a persistence pattern; no conflict. Future preset save/load UX deferred per §2.10.

---

## §11 — §2.11 Tool action transactionality verification (editor-adjacent inspection allowed)

### §11.1 Verification target

Per campaign doc §2.11: per-tool responsibility; dispatcher doesn't enforce Command pattern; existing AstraWeave undo/redo infrastructure is integration target.

### §11.2 AstraWeave evidence

**`EditorCommand` trait + `UndoStack`** (inspected at [command.rs:71-95 + 236-560 + 1691-1714](../../tools/aw_editor/src/command.rs#L71)):

```rust
// command.rs:71
pub trait EditorCommand: Send + fmt::Debug + std::any::Any {
    fn execute(&mut self, world: &mut World, entities: Option<&mut EntityManager>) -> Result<()>;
    fn undo(&mut self, world: &mut World, entities: Option<&mut EntityManager>) -> Result<()>;
    // ...
}

// command.rs:236
pub struct UndoStack { /* fields */ }

// command.rs:551
pub fn push_executed(&mut self, command: Box<dyn EditorCommand>) { /* ... */ }

// command.rs:1691
pub struct TerrainBrushCommand { /* fields */ }

// command.rs:1714
impl EditorCommand for TerrainBrushCommand { /* ... */ }
```

The `EditorCommand` trait is the existing Command pattern infrastructure. `UndoStack::push_executed(Box<dyn EditorCommand>)` accepts any Command. `TerrainBrushCommand` is the existing terrain-brush undo/redo Command — emitted at stroke end per main.rs:3868-3876 mediator code; will be emitted by TerrainPanel directly post-Mediator-Removal session.

**Compatibility synthesis**: AstraWeave's existing undo/redo infrastructure accepts tool-emitted Commands without dispatcher coupling. Tools emit Commands by calling `editor_app.undo_stack.push_executed(Box::new(MyCommand::new(...)))` at appropriate transaction granularity (typically stroke end). Dispatcher doesn't see Commands; dispatcher only routes events. Per-tool responsibility per §2.11.

**Open question (per §0.3)**: how does the tool obtain access to `editor_app.undo_stack` from inside its `on_*` event methods? Options:
- Pass `&mut UndoStack` through `ToolContext`.
- Tool emits an `EditorAction` (analog of TerrainAction); main.rs drains the action queue and applies to undo_stack (similar to existing `pending_actions` pattern in TerrainPanel).
- Tool returns a `Box<dyn EditorCommand>` from `on_left_mouse_button_up`; dispatcher routes to undo_stack.

**Deferral target**: Sub-phase 5 (RegionalArchetypePanel ActiveTool implementation + registration) — this is where the second tool's undo/redo plumbing forces the choice. Sub-phase 5 prompt drafting will resolve based on which option is most ergonomic; or a follow-up session if ergonomic decision is non-trivial.

### §11.3 Verification finding

**COMPATIBILITY CONFIRMED with one OPEN-QUESTION.** Existing `EditorCommand` trait + `UndoStack` is appropriate integration target; per-tool responsibility per §2.11 is structurally compatible. **Open question**: ergonomic mechanism for tool to access undo_stack from inside event methods (deferred to Sub-phase 5 prompt drafting or follow-up).

---

## §12 — Findings summary

### §12.1 Verification finding counts

Total: 10 §2.X decisions verified.

- **Compatibility**: 10 (all decisions).
- **Gap**: 0.
- **Open-question**: 2 (within compatibility-confirmed decisions: §2.7 ToolContext exposure mechanism for depth-buffer access; §2.11 undo_stack access mechanism from tool event methods).

### §12.2 Compatibility-confirmed decisions

All ten §2.X decisions verified compatible with AstraWeave editor code:

- **§2.2 ActiveTool trait shape** (§2 of this audit): TerrainPanel + RegionalArchetypePanel API surfaces map cleanly to Fyrox-style trait surface. Identical signature `fn(&mut self, world_x: f32, world_z: f32)` across `apply_brush_at` and `queue_paint_op`.
- **§2.3 EventDisposition enum** (§3): existing ViewportWidget event-handling has binary tool-consumed-or-camera-pass-through semantics implicitly; binary `Consumed`/`PassThrough` enum + `#[non_exhaustive]` maps directly.
- **§2.4 Dispatcher mechanism** (§4): egui's `Sense::click_and_drag()` + `Response` API produces discrete events compatible with push-based per-event dispatch. `HashMap<Uuid, Box<dyn ActiveTool>>` is structurally additive.
- **§2.5 Registration model** (§5): Editor::new() pattern is structurally additive for `register_tool` calls. PanelType enum and ActiveTool registration are orthogonal concerns. UUID open-set identity straightforward.
- **§2.6 Mediator removal** (§6): mediator code extent bounded (51 grep occurrences across 3 files; 44 lines + 6 fields + 4 branches + 6 accessors). Dedicated session scope per Q6 appropriately sized.
- **§2.7 ViewportWidget integration** (§7): post-Mediator-Removal end-state achievable. ViewportWidget already owns rendering + raw input capture; tool-state coupling bounded.
- **§2.8 Mutex arbitration** (§8): existing single-active-tool pattern (hardcoded `terrain_brush_active`) generalizes cleanly to `active_tool: Option<Uuid>`.
- **§2.9 Tool composition** (§9): no cross-tool composition patterns exist; deferral uncomplicated. TerrainPanel BrushMode sub-modes correctly classified as tool-internal state.
- **§2.10 State persistence** (§10): per-panel state patterns already each panel's responsibility.
- **§2.11 Action transactionality** (§11): existing `EditorCommand` trait + `UndoStack` is appropriate integration target.

### §12.3 Gap-evidence decisions

**None.** No gap-evidence findings. **No Andrew-gate triggered.**

### §12.4 Open-question decisions

Two open questions documented within compatibility-confirmed decisions; do NOT trigger Andrew-gate:

1. **§2.7 ToolContext exposure mechanism for depth-buffer access** — concrete API shape for `ToolContext::world_xz_at_pointer()` closure that wraps the existing depth-buffer read at viewport/widget.rs:1219-1234. Deferral target: Sub-phase 2 prompt drafting (when ToolContext type is concretely defined).

2. **§2.11 undo_stack access mechanism from tool event methods** — ergonomic choice between (a) pass `&mut UndoStack` through `ToolContext`; (b) tool emits `EditorAction` analog of TerrainAction with main.rs draining; (c) tool returns `Box<dyn EditorCommand>` from event methods. Deferral target: Sub-phase 5 prompt drafting (when second tool's undo/redo plumbing forces the choice).

### §12.5 Overall recommendation

**Sub-phase 2 prompt drafting + execution may proceed; §2 commitments confirmed.**

All ten §2.X architectural commitments verified compatible with AstraWeave editor code. Two open questions documented for downstream sub-phase resolution; both deferral targets are concrete (Sub-phase 2 + Sub-phase 5 prompt drafting). No Andrew-gate required.

The campaign's foundational architectural commitments hold against AstraWeave's actual editor architecture. Implementation can proceed with §2 intact.

---

## §13 — Methodology lessons

### §13.1 Pre-implementation feasibility audit pattern (canonical for future campaigns)

This audit demonstrates the canonical form of "pre-implementation feasibility audit" for foundational architectural campaigns. Pattern:

1. **Campaign-design pass commits to architectural decisions** (campaign doc §2; informed by research audit).
2. **Diagnostic sub-phase verifies decisions against target codebase** (this audit).
3. **Three-mode evidence framework** (compatibility / gap / open-question) classifies findings.
4. **Andrew-gate triggers ONLY on gap-evidence**; compatibility + open-question don't gate.
5. **Open-question deferral** allows non-blocking findings to surface for downstream sub-phase resolution.

The pattern's value: prevent Sub-phase 2-N from discovering architectural mismatches mid-implementation. The cost (this audit, ~4-6 hours) is amortized against the benefit of feasibility-verified architectural commitments + a pre-implementation reference document for the entire campaign.

Future foundational architectural campaigns inherit this pattern.

### §13.2 §2-decision-organized audit structure

This audit's section structure (one §X per §2.Y commitment) is an alternative to F.5-paint.E-diagnostic / G-diagnostic's hypothesis-classification structure (one §X per Hi). Both patterns valid; choice depends on audit purpose:

- **Hypothesis-classification structure** (F.5-paint.E-diagnostic, G-diagnostic): when audit's purpose is **diagnose a concrete failure**. Hypotheses enumerate failure modes; one is confirmed via evidence.
- **§2-decision-organized structure** (this audit): when audit's purpose is **verify architectural commitments against target codebase**. Each §2.X commitment is a verification target; evidence categorizes finding.

Future audits adopt structure based on purpose; both patterns canonical.

### §13.3 Single-reference points discipline (per anti-anchoring inheritance)

This audit cites G-diagnostic findings (viewport/widget.rs:163, main.rs:3833-3877, etc.) as single reference points but does NOT exploratorily inspect beyond G-diagnostic's existing findings except where §2.X verification specifically requires. This discipline is inherited from research audit §1.2 anti-anchoring framing — research informs but does not constrain inspection scope.

The three-file grep for `terrain_brush` (51 occurrences across 3 files) is an example of bounded scope expansion: G-diagnostic mentioned the mediator code; this audit's §6 verification needed extent measurement; grep was scoped to that single concern.

Future diagnostic audits inherit this discipline: predecessor findings are reference points; expand inspection scope only as §2.X verification requires.

---

*End of Editor Multi-Tool Architecture Sub-phase 1 Diagnostic audit.*
