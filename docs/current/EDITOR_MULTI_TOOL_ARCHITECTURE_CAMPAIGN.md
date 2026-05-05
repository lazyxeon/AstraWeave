# Editor Multi-Tool Architecture Campaign — Phase 1.X

**Status**: Campaign-design pass COMPLETE 2026-05-04, commits `75b68e7c7` (Design.A campaign doc) + `8fad61bd3` (Design.B Regional Archetype Variation cross-reference) + `8c92890b9` (Design.C hash-fixup). **Sub-phase 1 — Diagnostic COMPLETE 2026-05-04**, commits `4556c267b` (Diagnostic.A audit) + `0a7df3cdf` (Diagnostic.B campaign doc update) + `6924e39db` (Diagnostic.C hash-fixup); audit at `docs/audits/editor_multi_tool_architecture_diagnostic_2026-05-04.md`; all ten §2.X commitments compatibility-confirmed; 2 open-questions deferred. **Sub-phase 2 — ActiveTool trait + dispatcher core + register_tool API COMPLETE 2026-05-04**, commits `813ac29a1` (Core.A trait + types + ToolContext) + `2c791fa39` (Core.B Dispatcher) + `ece7bb3b4` (Core.C 15 unit tests + MockActiveTool fixture) + `6016b3c8f` (Core.D campaign doc update); new module at `tools/aw_editor/src/active_tool/`; resolves §2.7 ToolContext open question via pre-computed world-XZ projection fields + method accessors; module isolated (no external usages in ViewportWidget/main.rs/panels/tab_viewer); 15 unit tests pass; code-level only (NOT Andrew-gated per Q9). **Sub-phase 3 — TerrainPanel ActiveTool implementation (additive) IN PROGRESS**: 3.A `0dea0bebc` (impl ActiveTool for TerrainPanel + TerrainAction::SetActiveTool variant + UI emission + TERRAIN_PANEL_UUID constant) + 3.B `41ec3b192` (tab_viewer SetActiveTool capture + EditorApp.dispatcher field + ViewportWidget cached-then-dispatch integration) landed; 3.C closeout DEFERRED pending mediator brush fix. **Sub-phase 3 Mediator Brush Diagnostic COMPLETE 2026-05-05**, commits `e5e32f486` (Diagnostic.A audit) + `f5c96836f` (this commit, campaign doc update); audit at `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_2026-05-05.md`; H5 (Mediator drain logic regression) CONFIRMED at commit `f84eb09049` (2026-03-17, "imported kaykit complete asset package") — `viewport.set_terrain_brush_active` + `viewport.set_terrain_brush_params` placed inside `if let (Some(world), Some(viewport))` gate at main.rs:3867, brush state never reaches viewport when no scene loaded; H1-H4 + H6-H8 REFUTED or non-causal; recommended fix small (~10 lines, single-commit, no architectural changes); defect predates Sub-phase 3 by ~7 weeks per Andrew Q1 verification. Sub-phase 4-6 + Dedicated Mediator Removal session + Sub-phase 3 Mediator Brush Fix + Sub-phase 3.C closeout NOT STARTED. Foundational dispatcher architecture campaign launched as spinoff from Regional Archetype Variation pause artifacts (commits `a64f12320` + `98fc063d9` + `13ef70132`); Andrew architectural decision 2026-05-03 + strategic-factors enumeration Q1-Q10 ground §2 architectural decisions. Research pass at `docs/audits/editor_multi_tool_architecture_research_2026-05-03.md` (commits `8ba6cd13e` + `29b8c53b3` + `c3bc7ca0c`) is load-bearing input to §2; G-research + G-diagnostic audits inherited as predecessor research per research audit §2.

**Scope**: Replace AstraWeave editor's approach (B) — viewport widget + main.rs per-frame mediator hardcoded for TerrainPanel — with canonical Approach I+II hybrid dispatcher architecture per research audit §7.7 synthesis (registry/manager owns trait-object collection; per-event dispatch on active trait-implementation; UUID identity for open-set extensibility). Production-readiness threshold per Q3: level (ii) — full multi-tool dispatcher with proper mutex arbitration + lifecycle + Pattern A regression test coverage for dispatcher class. Both TerrainPanel + RegionalArchetypePanel migrated to ActiveTool; mediator code removed; campaign closes with editor's foundational tool architecture canonical and forward-compatible for future paint tools (splat, scatter, vegetation override, weather zones) per Q1 timeline.

**Author**: Plan drafted 2026-05-04 by the campaign-design pass session, against research audit §7 5-approach taxonomy + audit §7.7 Approach I+II hybrid synthesis + Andrew strategic-factors enumeration Q1-Q10.

**Prior work**:
- `docs/audits/editor_multi_tool_architecture_research_2026-05-03.md` — Editor Multi-Tool Architecture research audit; Session 2 of spinoff sequence; load-bearing input to §2 architectural decisions.
- `docs/audits/g_pointer_events_research_2026-05-03.md` — G-research audit; predecessor research inherited by reference in Editor Multi-Tool Architecture research audit §2.
- `docs/audits/g_pointer_events_diagnostic_2026-05-03.md` — G-diagnostic audit; AstraWeave classified as approach (B) with main.rs mediator; single-reference for AstraWeave classification.
- `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` — paused parent campaign; pause.B §10 entry contains methodological lesson about §0 discipline pattern inheritance.
- `docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md` — F.5-paint.E-diagnostic audit; canonical reference for editor surface addition methodology.

**Outcome on completion**:
- Editor uses canonical `ActiveTool` trait + `Dispatcher` struct + explicit `register_tool(Box<dyn ActiveTool>)` API.
- TerrainPanel + RegionalArchetypePanel both registered tools; brush UX functional via dispatcher routing.
- main.rs:3833-3877 mediator code removed; ViewportWidget per-tool fields removed.
- Pattern A regression tests cover dispatcher class comprehensively (registration, activation, mutex enforcement, lifecycle transitions, EventDisposition routing, set_active_tool transitions, dispatch_pointer_event routing).
- Future paint tools register via `dispatcher.register_tool(...)` — tool addition becomes a registration session rather than an architectural debate.
- Regional Archetype Variation campaign resumes; G-pointer-events-fix likely subsumed by Sub-phase 5 of this campaign per research audit §8.4.

---

## §0 — How to use this document and anti-drift discipline

This plan is the authoritative design reference for the Editor Multi-Tool Architecture campaign. It inherits Regional Archetype Variation's §0 structure with multi-tool architecture-specific framing per Q10.

### Discipline imposed

- **Sub-phase completion**: each sub-phase's success criteria must be met before §11 status block advances. Code-level success (compile, tests pass) is necessary but not sufficient for visible-output sub-phases — those require Andrew-gate per §0 lesson application.
- **Andrew-gate authoritative for visible-output sub-phases**: per Q9, only changes requiring visual verification or architectural decisions are Andrew-gated. Visible-output sub-phases (Sub-phase 3 TerrainPanel ActiveTool implementation; Sub-phase 5 RegionalArchetypePanel ActiveTool registration; Mediator Removal session) require Andrew-gate. Architectural-decision touchpoints (Sub-phase 1 Diagnostic IF it surfaces gaps in §2 decisions) require Andrew-gate.
- **Status header maintenance**: §11 phase status block updated in same commit as sub-phase closeout; Status header at top of doc updated similarly.
- **§2 architectural commitments respected**: §2 decisions are load-bearing; sub-phase execution implements per §2; sub-phase execution does NOT revise §2 decisions without explicit halt-and-re-research per §0 scope-creep discipline.

### Lesson application — Andrew-gate authoritative for visual verification + architectural decisions (per Q9)

Per Q9 strategic factor: only changes requiring visual verification or architectural decisions are Andrew-gated. The discipline policy:

- **Sub-phase 1 — Diagnostic**: Andrew-gate REQUIRED if diagnostic surfaces gaps in §2 decisions (architectural-decision touchpoint). Andrew makes the call: proceed with adjusted §2; halt-and-re-research per §0; spin off another foundational campaign. If diagnostic confirms §2 decisions, no Andrew-gate (code-level verification only).
- **Sub-phase 2 — ActiveTool trait + dispatcher core + register_tool API**: NOT Andrew-gated (code-level only; no visual verification needed; no architectural decision beyond §2's existing commitments).
- **Sub-phase 3 — TerrainPanel ActiveTool implementation (additive)**: Andrew-gate REQUIRED. TerrainPanel's brush must still work post-implementation; no regression in any of the 8 sculpt/paint/lower/smooth/flatten/erode/noise/zoneblend brush modes.
- **Sub-phase 4 — Pattern A regression infrastructure for dispatcher class**: NOT Andrew-gated (code-level only; comprehensive test coverage validated by `cargo test`).
- **Sub-phase 5 — RegionalArchetypePanel ActiveTool implementation + registration**: Andrew-gate REQUIRED. RegionalArchetypePanel's brush UX must work end-to-end; click+drag in viewport with Paint mode active produces visible paint feedback.
- **Mediator Removal session** (dedicated; not numbered sub-phase per Q6): Andrew-gate REQUIRED. Both TerrainPanel + RegionalArchetypePanel brushes must work post-removal; comprehensive verification of all 8 TerrainPanel brush modes + RegionalArchetypePanel paint/erase modes.
- **Sub-phase 6 — Closeout**: NOT Andrew-gated (doc-only).

### Scope-creep discipline — research-pass-before-reframe (inherited from Regional Archetype Variation §0)

Per Q10 + Regional Archetype Variation §0: standing authorization for halt-and-spinoff if Editor Multi-Tool Architecture surfaces foundational architectural gaps it doesn't cover.

The discipline pattern: if a sub-phase surfaces an architectural gap that requires reframing the campaign's scope, treat that as evidence of insufficient research-pass depth and consider another research pass rather than continuing to expand the campaign in-flight.

**Regional Archetype Variation campaign's pause is canonical precedent**: F.5-paint hit this exact failure mode; pause.B §10 entry documents the discipline pattern's application; the existence of THIS campaign is the result of that discipline. Honoring it again if needed produces canonical reference material that compounds across future campaigns.

**Specific halt-and-spinoff scenarios for this campaign**:
- Sub-phase 1 Diagnostic surfaces architectural gaps in ViewportWidget integration that §2.7 didn't anticipate. Andrew-gate; halt; assess whether new research pass needed.
- Sub-phase 3 TerrainPanel migration surfaces tight coupling between viewport renderer + brush state that §2 didn't anticipate. Andrew-gate; halt; assess.
- Mediator Removal session surfaces coupling that §2.6 didn't anticipate. Andrew-gate; halt; assess.

This discipline is NOT a license to halt at minor friction. It's reserved for foundational architectural gaps — the same threshold Regional Archetype Variation §0 used.

### Anti-pattern this plan explicitly prevents

- **Research-uninformed campaign-design**: §2 decisions ground in research audit §7 framework + Andrew Q1-Q10 strategic factors. Not first-principles; not "what feels right." The research audit is load-bearing input.
- **Bundled high-risk refactoring**: per Q6, mediator removal is dedicated session with fresh context. Bundling it with sub-phase containing other concerns produces context muddying — F.5-paint cascade's failure mode.
- **Premature optimization**: tool composition (§2.9), tool state persistence patterns (§2.10), tool action transactionality (§2.11) deferred per Q3 production-readiness threshold (level ii not iii). Forward concerns; explicit deferral keeps campaign focused.
- **Per-tool dispatcher logic** (god-object failure mode per research audit §6.3): dispatcher routes events; tools implement logic. The Mediator pattern's god-object risk is mitigated by trait-object dispatch — dispatcher knows nothing tool-specific.
- **Stringly-typed tool registration**: Approach I-only registration uses string IDs (UE pattern) which lose compile-time checking. Approach I+II hybrid per §2.5 uses UUID + Box<dyn ActiveTool> + explicit `register_tool` calls; failures surface at registration site rather than at activation.

### Methodological inheritance from Regional Archetype Variation

This campaign inherits Regional Archetype Variation's §0 discipline pattern wholesale. Specifically:

- **"Research-pass-before-reframe"** standing authorization (§0 scope-creep discipline above).
- **Andrew-gate authoritative for visible-output sub-phases** lesson application (§0 lesson application above).
- **Sub-phase completion requires success criteria + Andrew-gate where applicable** discipline.
- **§2 architectural commitments respected** — sub-phase execution implements per §2; revisions require halt-and-re-research.
- **Status header + §11 + §12 maintenance discipline** mirroring Regional Archetype Variation's §9 + §10 (note: this campaign uses §11 for status + §12 for deviations to leave room for §10 out-of-scope; functionally equivalent).

The campaign-pause-and-spinoff workflow pattern Regional Archetype Variation demonstrated is canonical; future spinoffs from this campaign (if any) inherit it.

---

## §1 — Design summary

### §1.1 The problem being solved

Per G-pointer-events-diagnostic audit §3-§6 + research audit §1.1: AstraWeave editor uses approach (B) per research audit §7.1 — viewport widget checks active-tool state internally; main.rs:3833-3877 acts as per-frame mediator that syncs `TerrainPanel.is_brush_active()` → `viewport.set_terrain_brush_active()` pre-render and drains `viewport.take_terrain_brush_hits()` → `dock_tab_viewer.apply_terrain_brush_at(...)` post-render. ViewportWidget at `tools/aw_editor/src/viewport/widget.rs:163` has typed `terrain_brush_active: bool` field + 5 supporting tool-specific fields (`terrain_brush_radius`, `terrain_brush_is_paint`, `terrain_brush_hits`, `terrain_brush_stroke_ended`, `last_brush_time`). Hardcoded for TerrainPanel; no abstraction; doesn't scale to multi-tool without per-tool ViewportWidget edits.

F.5-paint hit this structural ceiling. RegionalArchetypePanel has all building blocks (`paint_active`, `queue_paint_op`, `apply_pending_paint_ops_to_owned`) but zero references in main.rs — the (B)-pattern plumbing was missing entirely; F.5-paint.A's panel was never wired into the viewport's pointer-event flow. G-fix's pre-pause B-extend scope would have entrenched the (B) pattern by mirroring TerrainPanel's plumbing for RegionalArchetypePanel; the next paint tool campaign (splat per Q1 timeline) would have hit the same architectural wall.

### §1.2 The target

Per research audit §7.7 Approach I+II hybrid synthesis: registry/manager owns trait-object collection; dispatcher uses per-event method calls on the active trait-implementation; UUID identity provides open-set extensibility. Fyrox `InteractionMode` trait per research audit §5.1 is the production-grade Rust + egui canonical reference.

Concrete architecture commitment per §2 below:

- **`ActiveTool` trait** with method surface mirroring Fyrox InteractionMode pattern (§2.2): per-event handlers (mouse, keyboard, UI), lifecycle (activate/deactivate/update/on_drop), UI integration (make_button), UUID identity.
- **`EventDisposition` enum** binary Consumed/PassThrough at campaign close; `#[non_exhaustive]` for forward-compatibility per Q4 (§2.3).
- **Dispatcher** push-based per-event subscription; tracks active tool by UUID; framework-enforced mutex (§2.4 + §2.8).
- **Explicit `register_tool(Box<dyn ActiveTool>)` API** per Q5 mod-friendliness (§2.5).
- **Mediator removal** as dedicated session with fresh context per Q6 (§2.6).
- **ViewportWidget integration**: ViewportWidget owns rendering + raw input capture; dispatcher owns tool arbitration (§2.7).
- **Tool composition + state persistence + action transactionality** deferred per Q1 + Q3 (§2.9 + §2.10 + §2.11).

### §1.3 Sub-phase breakdown

Per Q8 scope-driven sizing + Q2 + Q6 + Q7 + Q9 strategic factors. 6 numbered sub-phases + 1 dedicated Mediator Removal session = 7 distinct campaign sessions:

- **Sub-phase 1** — Diagnostic (per Q7). Andrew-gate IF gaps surface.
- **Sub-phase 2** — ActiveTool trait + dispatcher core + register_tool API. Code-level only.
- **Sub-phase 3** — TerrainPanel ActiveTool implementation (additive). Andrew-gate REQUIRED.
- **Sub-phase 4** — Pattern A regression infrastructure for dispatcher class. Code-level only.
- **Sub-phase 5** — RegionalArchetypePanel ActiveTool implementation + registration. Andrew-gate REQUIRED. Likely subsumes G-pointer-events-fix per research audit §8.4.
- **Dedicated Mediator Removal session** (per Q6). Andrew-gate REQUIRED.
- **Sub-phase 6** — Closeout. Doc-only; no Andrew-gate.

Detailed treatment in §3-§9 below.

### §1.4 Integration with existing AstraWeave editor

Sub-phase 3 implements ActiveTool for TerrainPanel additively (both ActiveTool path + main.rs mediator coexist). Sub-phase 5 implements ActiveTool for RegionalArchetypePanel + registers via `register_tool`. After both proven via Andrew-gate, Mediator Removal session removes main.rs:3833-3877 mediator code + ViewportWidget per-tool fields. Pattern A regression tests cover dispatcher class throughout.

Single AstraWeave reference points used for §2 grounding (per anti-anchoring discipline allowing G-diagnostic findings as single reference points without exploratory inspection):
- ViewportWidget at `tools/aw_editor/src/viewport/widget.rs:163` — current `terrain_brush_active` field site.
- main.rs at `tools/aw_editor/src/main.rs:3833-3877` — current mediator code site.
- TerrainPanel at `tools/aw_editor/src/panels/terrain_panel.rs:797` — `is_brush_active()` method site (called by main.rs mediator).
- RegionalArchetypePanel at `tools/aw_editor/src/panels/regional_archetype_panel.rs:75` — unused `paint_active: bool` field site (G-diagnostic §9.6 forward observation).
- ViewportWidget at `tools/aw_editor/src/viewport/widget.rs:1219-1234` — depth-buffer access for ray-plane projection (used by TerrainPanel's brush per §2.7).

---

## §2 — Technical architecture

The load-bearing section. Resolves research audit §8.1's ten architectural decisions per audit §7 framework + Andrew Q1-Q10 strategic factors. Each §2.X subsection corresponds to one audit §8.1.X decision.

### §2.1 Data flow at the end state

Post-campaign-closure end state:

1. Editor input event arrives (mouse press, mouse move, mouse release, key press, key release, hot key).
2. ViewportWidget captures raw input via existing `Sense::click_and_drag()` + pointer position tracking + depth-buffer access.
3. ViewportWidget builds an `InputEvent` payload + `EventContext` (depth-buffer query closure, pointer position in viewport-local coordinates, modifier states).
4. ViewportWidget calls `dispatcher.dispatch_pointer_event(event, context)`.
5. Dispatcher checks `active_tool: Option<Uuid>`. If `None`, returns `EventDisposition::PassThrough` immediately.
6. If `Some(uuid)`, dispatcher looks up `tools.get_mut(&uuid) -> Option<&mut Box<dyn ActiveTool>>`.
7. Dispatcher calls the matching trait method on the active tool (e.g., `tool.on_left_mouse_button_down(event, context)`).
8. Tool's per-event method updates internal state (e.g., RegionalArchetypePanel's `queue_paint_op(world_x, world_z)` per F.5-paint.B).
9. Tool returns `EventDisposition`.
10. Dispatcher returns `EventDisposition` to ViewportWidget.
11. ViewportWidget consumes raw input if `Disposition::Consumed`; passes to camera handler if `Disposition::PassThrough`.

No mediator code in main.rs. No per-tool fields in ViewportWidget. Tool state lives in tool's ActiveTool implementation. Dispatcher is the only mediation layer.

### §2.2 ActiveTool trait shape (resolves audit §8.1.1)

**Decision**: ActiveTool trait surface mirrors Fyrox InteractionMode pattern per research audit §5.1, with adjustments for AstraWeave's egui + wgpu stack composition.

**Trait surface** (concrete sketch; sub-phase 2 produces final form):

```rust
pub trait ActiveTool {
    /// UUID identity for open-set extensibility per Q5 mod-friendliness.
    /// Replaces enum-based identity (which would close the set to first-party tools).
    fn uuid(&self) -> Uuid;

    /// Display name for UI integration (toolbar button label, settings panel header, etc.).
    fn name(&self) -> &str;

    /// Lifecycle: activated when user selects this tool; transitions previous active tool's
    /// deactivate before this is called.
    fn activate(&mut self, context: &mut ToolContext) {}

    /// Lifecycle: deactivated when user selects another tool or sets active to None.
    fn deactivate(&mut self, context: &mut ToolContext) {}

    /// Per-frame update; called only when this tool is active.
    fn update(&mut self, context: &mut ToolContext) {}

    /// Lifecycle: tool dropped from registry (rare; e.g., editor shutdown or hot-reload scenarios).
    fn on_drop(&mut self, context: &mut ToolContext) {}

    /// Per-event handlers; called only when this tool is active.
    /// Each defaults to PassThrough so tools only override what they care about.
    fn on_left_mouse_button_down(
        &mut self, _event: &MouseEvent, _context: &mut ToolContext,
    ) -> EventDisposition { EventDisposition::PassThrough }

    fn on_left_mouse_button_up(
        &mut self, _event: &MouseEvent, _context: &mut ToolContext,
    ) -> EventDisposition { EventDisposition::PassThrough }

    fn on_mouse_move(
        &mut self, _event: &MouseEvent, _context: &mut ToolContext,
    ) -> EventDisposition { EventDisposition::PassThrough }

    fn on_mouse_enter(&mut self, _context: &mut ToolContext) {}
    fn on_mouse_leave(&mut self, _context: &mut ToolContext) {}

    fn on_key_down(
        &mut self, _key: &KeyEvent, _context: &mut ToolContext,
    ) -> EventDisposition { EventDisposition::PassThrough }

    fn on_key_up(
        &mut self, _key: &KeyEvent, _context: &mut ToolContext,
    ) -> EventDisposition { EventDisposition::PassThrough }

    /// UI integration: tool provides its own toolbar button widget.
    /// Mirrors Fyrox `make_button` pattern per research audit §5.1.
    fn make_button(&mut self, ui: &mut egui::Ui, selected: bool) {}
}
```

**`ToolContext`** carries dispatcher → tool → tool-state communication: depth-buffer query closure, viewport rect, world-XZ projection helpers, current modifier states, scene-state mutability. Sub-phase 2 produces final shape.

**`MouseEvent`** + **`KeyEvent`** carry per-event payload: button state, modifier flags, viewport-local coordinates, world-XZ projection (where applicable).

**Forward extensibility per Q4**: trait surface accepts additions via default-implementation methods. Future hover-feedback tools (per Q4 ConsumedSelective enum variant) add `on_hover_started` / `on_hover_ended` methods with default empty implementations; existing ActiveTool implementations don't need updating.

**Per-tool implementation**: each tool implements only the methods it cares about. TerrainPanel implements `on_left_mouse_button_down` + `on_mouse_move` + `on_left_mouse_button_up` + `on_mouse_enter` + `on_mouse_leave` + brush-mode-switching key handlers + lifecycle. RegionalArchetypePanel implements similar set, calling existing `queue_paint_op` from F.5-paint.B.

**Rationale**: Fyrox's surface is the production-grade Rust + egui canonical reference per research audit §5.1 + §7.7 synthesis. AstraWeave's stack composition matches Fyrox closely (Rust + egui + custom rendering pipeline); precedent inheritance is high-fidelity. Per-event default-implementation methods (returning PassThrough) match Fyrox's pattern of empty defaults; tool implementations override only relevant methods.

### §2.3 EventDisposition enum semantics (resolves audit §8.1.2)

**Decision**: binary `Consumed` / `PassThrough` at campaign close per Q4. Enum declared `#[non_exhaustive]` for forward-compatibility.

**Concrete enum**:

```rust
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDisposition {
    /// Tool claimed the event; block downstream handling (camera control, etc.).
    Consumed,
    /// Tool didn't claim; let camera/default handler process.
    PassThrough,
    // Future: ConsumedSelective (Godot 4 CUSTOM-analog) for hover-feedback tools per Q4.
    // Variant added via #[non_exhaustive] without breaking consumers using match guards.
}
```

**`#[non_exhaustive]` discipline**: consumers must use match guards rather than wildcard patterns:

```rust
// CORRECT (forward-compatible):
match disposition {
    EventDisposition::Consumed => { /* handle */ },
    EventDisposition::PassThrough => { /* handle */ },
    _ => unreachable!("EventDisposition is non_exhaustive; new variants must be handled"),
    // OR explicit handling for future variants when added.
}

// FRAGILE (will break when third variant lands):
match disposition {
    EventDisposition::Consumed => { /* handle */ },
    EventDisposition::PassThrough => { /* handle */ },
    // No catch-all; compile-fails when third variant lands. Acceptable IF intent is "audit when extending."
}
```

**Pattern A regression test for variant addition**: when the third variant lands (likely `ConsumedSelective` for hover-feedback tools per Q4 timeline), all existing ActiveTool implementations + the dispatcher must continue compiling without modification. Test verifies via compile-only check that existing code doesn't pattern-match on EventDisposition with wildcards that would silently swallow new variants. (Specific test pattern: add a hypothetical `#[cfg(test)] EventDisposition::__TestOnlyVariant`; confirm dispatcher + tools compile.)

**Rationale**: Q4 explicitly named binary as sufficient for current scope while requiring extensibility. Godot 4 `AfterGUIInput` `PASS/STOP/CUSTOM` evolution per research audit §3.3 demonstrates this is a real industry need (Godot specifically introduced CUSTOM in v4 to address hover-feedback semantics that bool-only couldn't express). `#[non_exhaustive]` encodes the forward-compatibility commitment in Rust's type system.

### §2.4 Dispatcher mechanism (resolves audit §8.1.3)

**Decision**: push-based event subscription per research audit §7.2 + §7.7 synthesis. Dispatcher tracks active tool by UUID; per-event method calls only on active tool. Pull-based per-frame iteration NOT used.

**Concrete dispatcher structure**:

```rust
pub struct Dispatcher {
    active_tool: Option<Uuid>,
    tools: HashMap<Uuid, Box<dyn ActiveTool>>,
}

impl Dispatcher {
    pub fn new() -> Self { /* ... */ }

    pub fn register_tool(&mut self, tool: Box<dyn ActiveTool>) {
        let uuid = tool.uuid();
        self.tools.insert(uuid, tool);
    }

    pub fn set_active_tool(&mut self, uuid: Option<Uuid>, context: &mut ToolContext) {
        // Deactivate previous if any
        if let Some(prev_uuid) = self.active_tool {
            if let Some(prev_tool) = self.tools.get_mut(&prev_uuid) {
                prev_tool.deactivate(context);
            }
        }
        // Activate new if any
        if let Some(new_uuid) = uuid {
            if let Some(new_tool) = self.tools.get_mut(&new_uuid) {
                new_tool.activate(context);
            }
        }
        self.active_tool = uuid;
    }

    pub fn dispatch_mouse_event(
        &mut self,
        event: &MouseEvent,
        kind: MouseEventKind,
        context: &mut ToolContext,
    ) -> EventDisposition {
        let Some(uuid) = self.active_tool else {
            return EventDisposition::PassThrough;
        };
        let Some(tool) = self.tools.get_mut(&uuid) else {
            return EventDisposition::PassThrough;
        };
        match kind {
            MouseEventKind::LeftButtonDown => tool.on_left_mouse_button_down(event, context),
            MouseEventKind::LeftButtonUp => tool.on_left_mouse_button_up(event, context),
            MouseEventKind::Move => tool.on_mouse_move(event, context),
        }
    }

    // Similar dispatch_key_event, dispatch_mouse_enter, etc.

    pub fn update_active_tool(&mut self, context: &mut ToolContext) {
        if let Some(uuid) = self.active_tool {
            if let Some(tool) = self.tools.get_mut(&uuid) {
                tool.update(context);
            }
        }
    }
}
```

**Inactive tools' methods are NOT called**: dispatcher routes only to active tool. This is push-based per research audit §7.2 push-based optimization. Performance: HashMap lookup is O(1); trait-object dispatch through `Box<dyn ActiveTool>` adds ~one virtual call indirection (negligible vs frame budget).

**Rationale**: Fyrox uses push-based; UE UInputRouter is functionally push-based (only the captured behavior receives events); Unity OnToolGUI is pull-based but has documented performance issues per research audit §3.2 (tool self-checks via `IsActiveTool`). AstraWeave's stack favors push-based per Rust trait-object dispatch ergonomics + frame-budget-conscious editor performance.

### §2.5 Registration model (resolves audit §8.1.4)

**Decision**: explicit `register_tool(Box<dyn ActiveTool>)` API at editor init per Q5 mod-friendliness.

**Registration call sites**:

```rust
// First-party tools registered at Editor::new():
impl Editor {
    pub fn new() -> Self {
        let mut dispatcher = Dispatcher::new();
        dispatcher.register_tool(Box::new(TerrainPanel::default()));
        dispatcher.register_tool(Box::new(RegionalArchetypePanel::default()));
        // Future: dispatcher.register_tool(Box::new(SplatPaintPanel::default())); etc.
        Self { dispatcher, /* ... */ }
    }
}

// Third-party tools (mod-friendly per Q5):
impl Editor {
    pub fn register_external_tool(&mut self, tool: Box<dyn ActiveTool>) {
        self.dispatcher.register_tool(tool);
    }
}
// Or expose dispatcher directly:
pub fn dispatcher_mut(&mut self) -> &mut Dispatcher {
    &mut self.dispatcher
}
```

**Approach I + II hybrid synthesis per research audit §7.7**: explicit registry (Approach I — `register_tool` API) owns trait-object collection (Approach II — `HashMap<Uuid, Box<dyn ActiveTool>>`). Per-event dispatch on active trait-implementation. Fyrox's actual implementation matches this hybrid per research audit §5.1.

**UUID identity** open-set per Q5: third-party UUIDs don't conflict with first-party (random UUID generation; collision probability negligible). First-party tools' UUIDs are documented constants (e.g., `TERRAIN_PANEL_UUID: Uuid = uuid!("...")`); third-party tools generate their own.

**Registration order doesn't matter**: HashMap-based lookup is order-independent. `set_active_tool(uuid)` activates regardless of registration order. Tool palette UI iterates `dispatcher.tools.values()` for display (sub-phase 5 produces final UI).

**Rationale**: explicit registration is debuggable (registration call sites are greppable; failures surface at registration site rather than at activation) + Rust-idiomatic (explicit `Box<dyn Trait>` is the canonical Rust pattern) + mod-support-friendly per Q5. Attribute-based discovery (Approach III via `inventory` crate) introduces a heavyweight dependency; compile-time const arrays (research audit §3.4 alternative) prevent future plugin support. Explicit `register_tool` calls are the right call.

### §2.6 Mediator pattern fate (resolves audit §8.1.5)

**Decision**: replace completely per Q6. Mediator removal is its own dedicated campaign session with fresh context, NOT a sub-phase bundled with other work.

**Implementation strategy**:

- **Sub-phase 3** implements ActiveTool for TerrainPanel additively. ActiveTool path coexists with main.rs mediator code; both work simultaneously. ViewportWidget calls `dispatcher.dispatch_*` AND existing `terrain_brush_active`-branched code. TerrainPanel's `on_mouse_*` methods do the actual brush work; existing main.rs mediator code becomes redundant but doesn't break.
- **Sub-phase 5** implements ActiveTool for RegionalArchetypePanel + registers via `register_tool`. Dispatcher proven with two registered tools.
- **Dedicated Mediator Removal session** (between Sub-phase 5 and Sub-phase 6): full fresh context for careful refactoring; removes main.rs:3833-3877 mediator code; removes ViewportWidget `terrain_brush_active` field + 5 supporting fields + setters; removes `is_terrain_brush_active`/`apply_terrain_brush_at`/`take_terrain_brush_hits`/etc. accessors on tab_viewer; verifies TerrainPanel + RegionalArchetypePanel still work via Andrew-gate.

**Rationale per Q6**: mediator removal is high-risk refactoring of working code. Bundling it with a sub-phase containing other concerns (e.g., RegionalArchetypePanel registration in Sub-phase 5) produces context muddying that's been the F.5-paint cascade's failure mode. Discrete dedicated session with full context budget honors the discipline pattern. The session has only one concern — removal — so the agent can focus carefully without being pulled toward "while-I'm-here" extensions.

**Forward implication**: campaign closeout (Sub-phase 6) confirms mediator code removed + dispatcher is sole mediation layer + Pattern A regression tests in place + ARCHITECTURE_MAP.md editor section updated to reflect post-removal state.

### §2.7 Integration with existing ViewportWidget (resolves audit §8.1.6)

**Decision**: ViewportWidget retains responsibility for raw input event capture (`Sense::click_and_drag()`; pointer position tracking; depth-buffer access for ray-plane projection). Dispatcher handles tool arbitration + routing.

**Integration pattern**:

```rust
impl ViewportWidget {
    pub fn handle_input(&mut self, /* ... */) {
        // Existing Sense::click_and_drag() rect allocation; preserved.
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());

        // Build context for dispatcher.
        let mut context = ToolContext {
            depth_buffer: self.depth_buffer.as_ref(),
            world_xz_at_pointer: |viewport_pointer| { /* depth-buffer + ray-plane projection */ },
            viewport_rect: rect,
            modifiers: ui.ctx().input(|i| i.modifiers),
            // ...
        };

        // Route mouse-press to dispatcher.
        if response.dragged_by(egui::PointerButton::Primary) {
            let mouse_event = MouseEvent { /* ... */ };
            let disposition = self.dispatcher.dispatch_mouse_event(
                &mouse_event,
                MouseEventKind::Move,  // dragged → continuous Move events
                &mut context,
            );
            match disposition {
                EventDisposition::Consumed => { /* tool handled it */ },
                EventDisposition::PassThrough => {
                    // Camera handles it
                    self.camera.orbit(response.drag_delta().x, response.drag_delta().y);
                },
                _ => { /* future variants */ },
            }
        }

        // Similar for click + release + key events + etc.
    }
}
```

**Post-Mediator-Removal end state**: ViewportWidget has NO per-tool fields. No `terrain_brush_active`. No `regional_archetype_paint_active`. Tool state lives entirely in tool's ActiveTool implementation (e.g., TerrainPanel's `brush_enabled` + `brush_mode`; RegionalArchetypePanel's `paint_active` + `paint_mode`).

**Depth-buffer access** for ray-plane projection (per G-diagnostic audit at viewport/widget.rs:1219-1234): ViewportWidget exposes depth-buffer query as a method on `ToolContext`. Tools that need depth-accurate world projection (TerrainPanel sculpt brush — surface-following) call `context.world_xz_at_pointer()` which uses depth-buffer + camera unprojection. Tools that don't (RegionalArchetypePanel paint per F.5-paint.B's `screen_to_world_xz_y0` ray-plane projection at y=0 plane) ignore it or use a simpler `context.world_xz_at_y0()` helper.

**Rationale**: ViewportWidget owns rendering + raw input capture; dispatcher owns tool arbitration. Clean separation of concerns; matches Fyrox precedent where Editor's scene_view owns rendering and the InteractionMode trait owns tool logic.

### §2.8 Mutex arbitration semantics (resolves audit §8.1.7)

**Decision**: framework-enforced mutex via dispatcher's `active_tool: Option<Uuid>` field. Single active tool at a time; `set_active_tool` transitions previous → new with proper `deactivate` → `activate` lifecycle calls.

Implementation per §2.4 dispatcher snippet above. `active_tool` starts `None` (no tool active; events pass through to camera). `set_active_tool(Some(uuid))` calls previous active tool's `deactivate()` (if any) then new tool's `activate()`. `set_active_tool(None)` calls active tool's `deactivate()`; subsequent events pass through to camera.

**Forward extensibility for multi-active scenarios** per research audit §3.1 (UE supports per-input-device active tools for VR): future `#[non_exhaustive]` dispatcher API additions can extend to multi-active. NOT in current scope per Q1 + Q3.

**Rationale**: framework-enforced mutex matches research audit §7.4 majority pattern. Tool-self-arbitrated mutex (Unity OnToolGUI's `IsActiveTool` check) has documented bugs per research audit §3.2; framework-enforced is universally preferred.

### §2.9 Tool composition rules (resolves audit §8.1.8)

**Decision**: composition support deferred to follow-up campaigns. Current campaign produces single-active-tool dispatcher; tool composition (nested tools, sub-tool delegation, parent-child tool relationships) is forward concern.

**Forward extensibility commitments** per Q4 + Q1: trait surface + dispatcher API designed to NOT preclude future composition extensions:

- **Sub-tool pattern** (parent tool delegates to child tool via parent's per-event methods calling child's): documentable as a pattern; doesn't require dispatcher changes. A parent ActiveTool implementation can hold a child `Box<dyn ActiveTool>` and delegate to its methods.
- **Per-input-device active tools** per research audit §3.1 (UE VR pattern): dispatcher's `active_tool: Option<Uuid>` extends to `HashMap<InputDevice, Option<Uuid>>` when needed. Out of scope for current campaign.
- **Tool-of-tools manager** per research audit §3.1 (UE meta-tool pattern): out of scope.

**Rationale per Q1 + Q3**: production-readiness threshold is level (ii) — full multi-tool dispatcher with mutex/lifecycle/Pattern A regression. Composition rules are level (iii) tooling beyond current scope. Deferring keeps campaign focused.

### §2.10 Tool state persistence (resolves audit §8.1.9)

**Decision**: per-tool persistence is each tool's responsibility; dispatcher doesn't enforce a persistence pattern.

Tools that need persistent state (TerrainPanel brush settings; RegionalArchetypePanel brush size + falloff radius + selected archetype) implement their own save/load via existing AstraWeave editor preferences mechanism. Dispatcher doesn't know or care about per-tool settings.

**Forward pattern reference** per research audit §3.1 (UE `UInteractiveToolPropertySet`) + §4.1 (Krita `KisPaintopPreset`): when AstraWeave needs per-tool preferences UX (preset save/load; settings panel layouts), follow these canonical references. Implementation deferred to per-tool sub-phases or follow-up campaigns.

**Rationale**: tool state persistence is orthogonal to dispatcher architecture. Coupling persistence into ActiveTool trait surface adds complexity without clear current need. Tools handle their own; dispatcher focuses on event routing + lifecycle.

### §2.11 Tool action transactionality (resolves audit §8.1.10)

**Decision**: action transactionality (Command pattern integration with undo/redo) is each tool's responsibility; dispatcher doesn't enforce a Command pattern.

Tools that need undo/redo (TerrainPanel brush actions per existing `TerrainBrushCommand`; RegionalArchetypePanel paint operations) emit Commands at appropriate transaction granularity (typically stroke end). Dispatcher doesn't know or care about Commands.

**Forward pattern reference** per research audit §6.2 Command + Memento composition: composite Commands for transactional bulk operations (e.g., "paint stroke = N hits as one undo action") is the canonical pattern. AstraWeave's existing undo/redo infrastructure (likely main.rs-level command history; needs verification by Sub-phase 1 Diagnostic per Q7) is the integration target.

**Pattern A regression test scope**: dispatcher class only (registration + activation + mutex + lifecycle + EventDisposition routing). NOT undo/redo integration tests; those belong to per-tool sub-phases or undo/redo follow-up campaigns.

**Rationale**: transactionality is orthogonal to dispatcher architecture. Coupling Command pattern into ActiveTool trait surface adds complexity without clear current need; existing AstraWeave undo/redo infrastructure may already serve this need (Sub-phase 1 Diagnostic confirms).

---

## §3 — Sub-phase 1 — Diagnostic

Per Q7. Compares AstraWeave's actual editor architecture against research audit pattern catalog. Confirms or surfaces gaps in §2 decisions.

### §3.1 Goal

Inspect editor code; classify against research audit §7.1 5-approach taxonomy; confirm AstraWeave matches expected approach (B) per G-diagnostic; surface any architectural gaps in §2 decisions; document observations + recommendations in audit at `docs/audits/editor_multi_tool_architecture_diagnostic_<YYYY-MM-DD>.md`.

### §3.2 Scope

**In-scope** (single AstraWeave references; not exploratory):

- `tools/aw_editor/src/main.rs` — main.rs:3833-3877 mediator code; broader Editor struct context.
- `tools/aw_editor/src/viewport/widget.rs` — ViewportWidget per-tool fields + handle_input dispatch; depth-buffer access pattern.
- `tools/aw_editor/src/panels/terrain_panel.rs` — TerrainPanel state machine; integration points for ActiveTool implementation.
- `tools/aw_editor/src/panels/regional_archetype_panel.rs` — RegionalArchetypePanel state machine; existing F.5-paint.B brush logic.
- `tools/aw_editor/src/tab_viewer/mod.rs` — EditorTabViewer struct; per-tool accessors.
- `tools/aw_editor/src/panel_type.rs` — PanelType enum; current tool registration pattern (panel-level, not tool-level).
- `tools/aw_editor/src/dock_panels.rs` — DockPanelContext; placeholder dispatch for non-field-based panels.
- Existing undo/redo infrastructure (locate; assess Command pattern integration target).

**Out-of-scope**:

- Implementation; recommendation of single approach (research audit §7 already produced taxonomy + tradeoff matrix; §2 already commits).
- Deep dive into wgpu integration beyond what's needed to ground §2 decisions.
- Diagnostic-level evaluation of additional concerns (tool composition, state persistence, action transactionality) — those are explicitly deferred per §2.9 + §2.10 + §2.11.

### §3.3 Success criteria

- Diagnostic audit produced at `docs/audits/editor_multi_tool_architecture_diagnostic_<YYYY-MM-DD>.md`.
- §2 decision validation: confirmed (audit §7 §2.1-§2.11 each marked validated against AstraWeave) OR gaps surfaced + §0 halt-and-re-research authorized + Andrew-gate triggered.
- No production code changes; no test changes.

### §3.4 Andrew-gate

Per Q9: architectural-decision touchpoint. If diagnostic surfaces gaps in §2 decisions, Andrew-gate triggers; Andrew makes the call (proceed with adjusted §2; halt-and-re-research per §0; spin off another foundational campaign). If diagnostic confirms §2 decisions, no Andrew-gate needed (code-level verification only).

### §3.5 Reversibility

Doc-only; trivial revert via `git revert`.

### §3.6 Expected commits

- **Diagnostic.A**: audit document.
- **Diagnostic.B**: campaign doc Status header + §11 update + §12 entry capturing diagnostic findings.
- Optional hash-fixup.

---

## §4 — Sub-phase 2 — ActiveTool trait + dispatcher core + register_tool API

### §4.1 Goal

Implement §2.2 ActiveTool trait + §2.3 EventDisposition enum + §2.4 dispatcher mechanism + §2.5 register_tool API. Produces working dispatcher infrastructure with no tools registered yet.

### §4.2 Scope

**In-scope**:

- New module `tools/aw_editor/src/active_tool/mod.rs` (or similar; Sub-phase 2 picks final path).
- `ActiveTool` trait + `EventDisposition` enum + `MouseEvent` + `KeyEvent` + `MouseEventKind` + `ToolContext` types.
- `Dispatcher` struct + `register_tool` + `set_active_tool` + `dispatch_mouse_event` + `dispatch_key_event` + `update_active_tool` + supporting methods.
- Module-level unit tests for dispatcher mechanics: registration, activation transitions (deactivate-then-activate), mutex enforcement (single active), default-implementation pass-through.
- No integration with ViewportWidget or main.rs yet.

**Out-of-scope**:

- Integration with existing editor runtime path (Sub-phase 3 + Sub-phase 5 + Mediator Removal session).
- Tool implementations (Sub-phase 3 + Sub-phase 5).
- Pattern A regression test infrastructure (Sub-phase 4).

### §4.3 Success criteria

- Trait + enum + dispatcher compile.
- Module-level unit tests pass: registration + activation transitions + mutex.
- `cargo check -p aw_editor` clean.
- No integration with editor's runtime path (verified by grep — `Dispatcher` referenced only in `active_tool/` module + its tests).

### §4.4 Andrew-gate

Per Q9: code-level only; no visual verification or architectural decision. NOT Andrew-gated.

### §4.5 Reversibility

New file additions; trivial revert via `git revert`. ViewportWidget + main.rs untouched.

### §4.6 Expected commits

- **Core.A**: ActiveTool trait + types.
- **Core.B**: Dispatcher struct + methods.
- **Core.C**: Module-level unit tests.
- **Core.D**: Closeout (campaign doc §11 update; §12 deviations entry if any).

Sub-phase prompts decide actual sub-commit shape; this is the expected breakdown.

---

## §5 — Sub-phase 3 — TerrainPanel ActiveTool implementation (additive)

### §5.1 Goal

Implement ActiveTool for TerrainPanel; both ActiveTool path + main.rs mediator coexist; verify TerrainPanel's brush still works via Andrew-gate.

### §5.2 Scope

**In-scope**:

- Modifications to `tools/aw_editor/src/panels/terrain_panel.rs` — `impl ActiveTool for TerrainPanel`. Uses Fyrox-style per-event method delegation: `on_left_mouse_button_down` triggers brush start; `on_mouse_move` continues brush stroke (with throttling per existing `last_brush_time`); `on_left_mouse_button_up` ends brush stroke + emits TerrainBrushCommand.
- Modifications to `tools/aw_editor/src/main.rs` — call `dispatcher.register_tool(Box::new(TerrainPanel))` at editor init (after dispatcher construction). Existing main.rs mediator code (line 3833-3877) is **preserved** — both paths coexist additively.
- Modifications to `tools/aw_editor/src/viewport/widget.rs` — `handle_input` calls `dispatcher.dispatch_*` for input events (alongside existing `terrain_brush_active`-branched code). Both ActiveTool path + existing `terrain_brush_active`-branching coexist; TerrainPanel's brush works via either path.
- TerrainPanel `is_brush_active()` → `set_active_tool(Some(TERRAIN_PANEL_UUID))` integration: when user enters brush mode in TerrainPanel UI, panel sets dispatcher's active tool to itself.

**Out-of-scope**:

- Removing existing main.rs mediator code (deferred to Mediator Removal session per §2.6).
- Removing ViewportWidget per-tool fields (deferred to Mediator Removal session).
- RegionalArchetypePanel changes (Sub-phase 5).
- Pattern A regression tests for dispatcher class (Sub-phase 4).

### §5.3 Success criteria

- `impl ActiveTool for TerrainPanel` compiles.
- Dispatcher registers TerrainPanel at editor init.
- ViewportWidget routes events to dispatcher (alongside existing main.rs mediator).
- TerrainPanel's brush works via dispatcher path: clicks/drags route to `on_*` methods → existing brush logic → TerrainBrushCommand emission preserved.
- TerrainPanel's brush ALSO works via existing main.rs mediator path (both paths active simultaneously per Q2 risk-bounding).
- All 8 brush modes (sculpt/paint/lower/smooth/flatten/erode/noise/zoneblend) functional.

### §5.4 Andrew-gate (REQUIRED per Q9)

Visible-output sub-phase. Andrew opens editor; selects TerrainPanel brush mode; click+drags in viewport. Expected:

- Brush feedback visible (cursor circle drape on terrain; brush hits accumulating).
- All 8 brush modes work as before (Andrew tests at least 3-4 modes).
- No regression in stroke timing, brush size visualization, depth-buffer-based hit detection.
- Stroke end produces undo entry (TerrainBrushCommand emitted to undo stack).

### §5.5 Reversibility

Additive code; revert removes ActiveTool impl + register_tool call + ViewportWidget dispatcher.dispatch_* calls. main.rs mediator code untouched (still works).

### §5.6 Expected commits

- **Sub-phase 3.A**: `impl ActiveTool for TerrainPanel` in terrain_panel.rs.
- **Sub-phase 3.B**: dispatcher registration in main.rs + ViewportWidget dispatcher.dispatch_* integration.
- **Sub-phase 3.C** (Andrew-gate PASS): closeout — campaign doc §11 update; §12 entry.

Sub-phase prompts decide actual sub-commit shape.

---

## §6 — Sub-phase 4 — Pattern A regression infrastructure for dispatcher class

### §6.1 Goal

Implement Pattern A regression tests covering dispatcher class — registration, activation, mutex enforcement, lifecycle transitions (activate/deactivate/update), EventDisposition routing, set_active_tool transitions, dispatch_pointer_event routing.

### §6.2 Scope

**In-scope**:

- New tests file `tools/aw_editor/src/active_tool/tests.rs` (or co-located in `mod.rs`).
- Test fixtures: synthetic `MockActiveTool` implementations exposing internal state for assertion.
- Coverage of dispatcher API:
  - `register_tool` adds tool to registry (verify via UUID lookup).
  - `register_tool` with same UUID overwrites (verify; document semantic).
  - `set_active_tool(Some)` transitions previous-deactivate → new-activate.
  - `set_active_tool(None)` deactivates current; subsequent dispatch returns PassThrough.
  - `dispatch_mouse_event` routes to active tool's matching method; returns tool's EventDisposition.
  - `dispatch_mouse_event` with no active tool returns PassThrough.
  - `dispatch_mouse_event` with active tool whose UUID is no longer registered returns PassThrough (graceful handling).
  - `update_active_tool` calls active tool's update() once per tick.
  - Lifecycle ordering: register → set_active → activate called; set_active(other) → deactivate-then-activate sequence.
  - Default-implementation pass-through: tool that doesn't override `on_left_mouse_button_down` returns PassThrough by default.
  - Mutex enforcement: only one active tool at a time; `set_active_tool` transitions properly.
- No production code changes beyond test infrastructure.

**Out-of-scope**:

- Tool-specific tests (TerrainPanel brush mode tests; RegionalArchetypePanel paint mode tests) — those belong to per-tool sub-phases.
- Integration tests that exercise the full editor runtime path — out of Pattern A regression scope (those would be sub-phase 3 / sub-phase 5 visual-verification Andrew-gate territory).

### §6.3 Success criteria

- Pattern A regression test suite covers dispatcher class comprehensively.
- All tests pass.
- Suite runs as part of `cargo test -p aw_editor` workflow.
- Coverage measurement: dispatcher class methods all exercised by at least one test.

### §6.4 Andrew-gate

Per Q9: code-level only. NOT Andrew-gated.

### §6.5 Reversibility

Test additions; trivial revert.

### §6.6 Expected commits

- **Sub-phase 4.A**: test fixtures + dispatcher mechanics tests.
- **Sub-phase 4.B**: lifecycle + EventDisposition tests.
- **Sub-phase 4.C**: closeout.

---

## §7 — Sub-phase 5 — RegionalArchetypePanel ActiveTool implementation + registration

### §7.1 Goal

Implement ActiveTool for RegionalArchetypePanel; register via `dispatcher.register_tool`; verify brush UX works (subsumes G-pointer-events-fix scope per research audit §8.4).

### §7.2 Scope

**In-scope**:

- Modifications to `tools/aw_editor/src/panels/regional_archetype_panel.rs`:
  - `impl ActiveTool for RegionalArchetypePanel`.
  - `on_left_mouse_button_down`: activate paint mode + start stroke (set internal state per existing F.5-paint.B `paint_active` flag at line 75; activate via Q5 forward observation).
  - `on_mouse_move`: continue stroke; call existing `queue_paint_op(world_x, world_z)` from F.5-paint.B.
  - `on_left_mouse_button_up`: end stroke; flush via `apply_pending_paint_ops_to_owned()`.
  - `activate`/`deactivate` lifecycle methods.
  - `make_button` UI integration.
  - UUID identity (constant `REGIONAL_ARCHETYPE_PANEL_UUID`).
- Modifications to `tools/aw_editor/src/main.rs` — register RegionalArchetypePanel alongside TerrainPanel at editor init.
- No new ViewportWidget fields (per §2.7 separation; ViewportWidget routes events to dispatcher; dispatcher routes to active tool).
- UI integration: when user enters Paint mode in RegionalArchetypePanel UI, panel sets dispatcher's active tool to itself.

**Out-of-scope**:

- TerrainPanel changes (Sub-phase 3 done; coexistence preserved).
- Removing existing main.rs mediator code (Mediator Removal session).
- H-saveload diagnostic / fix (separate session post-resumption).
- F.5-overlay-and-gate (separate session post-resumption).
- Pattern A regression for tool-specific behavior (out of Pattern A scope).

### §7.3 Success criteria

- `impl ActiveTool for RegionalArchetypePanel` compiles.
- Dispatcher registers RegionalArchetypePanel at editor init alongside TerrainPanel.
- Click+drag with Paint mode active routes events to RegionalArchetypePanel.queue_paint_op.
- Brush UX works end-to-end:
  - User opens RegionalArchetypePanel, selects archetype + paint mode.
  - Click+drag in viewport produces paint operations.
  - Mask state updates as expected.
  - Save/Load operations functional (per F.5-paint.C; not regressed).

### §7.4 Andrew-gate (REQUIRED per Q9)

Visible-output sub-phase. Andrew opens editor; selects RegionalArchetypePanel; selects archetype (e.g., Boreal); enters Paint mode; click+drags in viewport. Expected:

- Visual paint feedback (mask updates visible if rendered; or at minimum, painted regions accumulate in panel's owned mask).
- Click+drag routes to RegionalArchetypePanel's brush queue (not consumed by camera pan, which was the F.5-paint.F-fix Andrew-gate REGRESS).
- Switching to TerrainPanel brush mode (Sub-phase 3 still works; tools coexist via dispatcher).
- Brush UX click+drag does NOT route to camera pan (the original F.5-paint REGRESS is fixed).

### §7.5 Forward implication for Regional Archetype Variation resumption

Sub-phase 5 likely subsumes G-pointer-events-fix per research audit §8.4. If subsumed: Regional Archetype Variation resumes at H-saveload-diagnostic post-Sub-phase 5 + Mediator Removal + Sub-phase 6 closeout.

If not fully subsumed (e.g., Sub-phase 5 only registers + minimal integration; full G-pointer-events-fix needed for additional integration like undo/redo wiring): G-pointer-events-fix runs as small Regional Archetype Variation session post-this-campaign closeout. Estimated 1 small commit if needed.

### §7.6 Reversibility

Additive code; revert removes ActiveTool impl + register_tool call + Paint mode activation logic.

### §7.7 Expected commits

- **Sub-phase 5.A**: `impl ActiveTool for RegionalArchetypePanel` in regional_archetype_panel.rs.
- **Sub-phase 5.B**: dispatcher registration in main.rs + UI integration for Paint mode activation.
- **Sub-phase 5.C** (Andrew-gate PASS): closeout — campaign doc §11 update; §12 entry; cross-reference Regional Archetype Variation campaign doc with G-pointer-events-fix subsumption confirmation.

---

## §8 — Dedicated Session — Mediator Removal

Per Q6: NOT a sub-phase; dedicated session with fresh context for high-risk refactoring.

### §8.1 Goal

Remove main.rs:3833-3877 mediator code; remove ViewportWidget `terrain_brush_active` field + 5 supporting tool-specific fields + setters; verify TerrainPanel + RegionalArchetypePanel still work post-removal via Andrew-gate.

### §8.2 Scope

**In-scope** (pure removal; no new code):

- Modifications to `tools/aw_editor/src/main.rs`:
  - Remove pre-render terrain brush sync block (lines ~3834-3846 per G-diagnostic findings: `let brush_active = ...` through `viewport.set_terrain_brush_active(brush_active);` etc.).
  - Remove post-render hit drain block (lines ~3862-3877: `viewport.take_terrain_brush_hits()` through stroke end detection).
- Modifications to `tools/aw_editor/src/viewport/widget.rs`:
  - Remove `terrain_brush_active: bool` field (line 163).
  - Remove 5 supporting tool-specific fields (`terrain_brush_radius`, `terrain_brush_is_paint`, `terrain_brush_hits`, `terrain_brush_stroke_ended`, `last_brush_time`).
  - Remove setters (`set_terrain_brush_active`, `set_terrain_brush_params`, `take_terrain_brush_hits`, `take_terrain_brush_stroke_ended`).
  - Remove `terrain_brush_active`-branched code in `handle_input` (lines 1180-1255 per G-diagnostic findings).
- Modifications to `tools/aw_editor/src/tab_viewer/mod.rs`:
  - Remove `is_terrain_brush_active`, `apply_terrain_brush_at`, `terrain_brush_radius`, `terrain_brush_is_paint`, `terrain_brush_mode_name`, `end_terrain_brush_stroke` accessors (assess via grep; remove all that were main.rs-mediator-only).
- Build verification + Pattern A regression test runs.

**Out-of-scope**:

- New code (pure removal session; if new code emerges as needed, halt-and-surface per §0).
- Adding new dispatcher capabilities (sub-phase work; not removal session).
- Per-tool feature additions (sub-phase work).

### §8.3 Success criteria

- Mediator code removed (lines 3833-3877 of main.rs deleted; no `terrain_brush_*` accessor calls remain).
- ViewportWidget has no per-tool fields (grep `tools/aw_editor/src/viewport/widget.rs` for `terrain_*` returns nothing).
- TerrainPanel + RegionalArchetypePanel both work via dispatcher (no regression).
- Pattern A regression tests pass.
- Build clean (`cargo check -p aw_editor`; `cargo test -p aw_editor`).

### §8.4 Andrew-gate (REQUIRED per Q9)

Visible-output session. Comprehensive verification:

- Andrew opens editor; selects TerrainPanel; tests all 8 brush modes (sculpt/paint/lower/smooth/flatten/erode/noise/zoneblend). Each mode functional.
- Andrew selects RegionalArchetypePanel; tests Paint + Erase modes. Both functional.
- Andrew switches between TerrainPanel + RegionalArchetypePanel. Tools coexist correctly; activation transitions work.
- No new wgpu validation errors or panics on editor startup.
- FPS in expected range (no per-frame mediator overhead removed actually accounted for slight FPS regression? If so, log §12).
- Existing TerrainPanel functionality preserved (terrain regeneration, undo/redo, brush settings).

### §8.5 Why dedicated session

Per Q6: high-risk refactoring of working code. Bundling with sub-phase containing other concerns produces context muddying (F.5-paint cascade's failure mode). Discrete dedicated session honors discipline pattern; full context for careful work.

The session has only one concern — removal — so the agent can focus on correctness without being pulled toward "while-I'm-here" extensions or new features. If the removal surfaces unexpected coupling, halt-and-surface per §0; new code creation is out of session scope.

### §8.6 Reversibility

Pure removal; revert restores mediator code via `git revert`. Functional regression risk is contained by Andrew-gate verification.

### §8.7 Expected commits

- **Mediator-Removal.A**: main.rs mediator code removal.
- **Mediator-Removal.B**: ViewportWidget per-tool field removal + setter removal.
- **Mediator-Removal.C**: tab_viewer accessor cleanup.
- **Mediator-Removal.D** (Andrew-gate PASS): closeout — campaign doc §11 update; §12 entry.

Session prompt decides actual sub-commit shape.

---

## §9 — Sub-phase 6 — Closeout

### §9.1 Goal

Updates parent campaign references; updates ARCHITECTURE_MAP.md editor section; cross-references Regional Archetype Variation resumption; standard housekeeping.

### §9.2 Scope

**In-scope**:

- Updates to `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md`:
  - Status header line: Editor Multi-Tool Architecture campaign COMPLETE entry.
  - §9 status block: Editor Multi-Tool Architecture COMPLETE; Regional Archetype Variation resumption pointer (post-Sub-phase 5, G-pointer-events-fix likely subsumed).
  - §10 entry capturing Editor Multi-Tool Architecture campaign closure + Regional Archetype Variation resumption shape.
- Updates to `docs/current/ARCHITECTURE_MAP.md` editor section: reflect ActiveTool dispatcher + register_tool API + canonical pattern. Document the post-removal end-state architecture.
- Updates to this campaign's §11: mark all sub-phases COMPLETE.
- Updates to this campaign's Status header: COMPLETE.

**Out-of-scope**:

- New audit documents (campaign closes without producing additional audits).
- Production code changes (closeout is doc-only).
- Sub-phase prompt drafting for follow-up campaigns (those are separate sessions).

### §9.3 Success criteria

- Doc updates land.
- Campaign marked COMPLETE.
- Regional Archetype Variation resumption point unambiguous.
- ARCHITECTURE_MAP.md editor section accurately reflects post-campaign state.

### §9.4 Andrew-gate

Per Q9: doc-only; not Andrew-gated.

### §9.5 Reversibility

Doc-only; trivial revert.

### §9.6 Expected commits

- **Sub-phase 6.A**: campaign doc + Regional Archetype Variation cross-reference + ARCHITECTURE_MAP update.
- Optional hash-fixup.

---

## §10 — Out of scope for entire campaign

Items explicitly out of scope per Q1 + Q3 + Q9 production-readiness threshold:

- **Tool composition rules** (deferred per §2.9; future paint tool campaigns or follow-up). Sub-tool delegation, parent-child tool relationships, tool-of-tools manager — all forward concerns. Trait surface designed to NOT preclude these extensions per §2.9 forward extensibility commitment.

- **Tool state persistence pattern** (deferred per §2.10; per-tool responsibility). UI for preset save/load; per-tool settings panel layouts; cross-session preference persistence — all per-tool concerns. Dispatcher doesn't enforce a persistence pattern.

- **Tool action transactionality / Command pattern integration** (deferred per §2.11; per-tool responsibility). Composite Commands for transactional bulk operations; undo/redo integration — all per-tool concerns. Dispatcher doesn't enforce Command pattern.

- **Hover-feedback tool semantics** (deferred per Q4; future when hover-feedback tools land). `EventDisposition` extends to `ConsumedSelective` variant when needed; `#[non_exhaustive]` enables forward addition without breaking consumers.

- **ECS-style component-based tool registration** (Approach V per research audit §7.1; deferred per Q5 mod-friendliness via Approach I+II hybrid). AstraWeave's custom-ECS could theoretically support tools-as-components; not in scope for current campaign.

- **Per-input-device multi-active tools** (UE VR pattern per research audit §3.1; deferred per Q1 scope). Future `#[non_exhaustive]` dispatcher API extensions can support multi-active when needed.

- **Developer documentation tooling** (level (iii) per Q3; not in current scope). Tutorial documentation, code examples beyond §2 specification, developer guides — all forward concerns. Pattern A regression tests serve as canonical reference for dispatcher class behavior.

- **Wgpu integration beyond what's needed for ViewportWidget event capture** (deferred to wgpu-specific follow-up if needed). Sub-phase 3 + Sub-phase 5 use existing ViewportWidget depth-buffer access pattern; campaign doesn't reframe wgpu integration.

- **Editor preferences UX** (forward concern; per-tool implementations leverage existing preferences mechanism if/when needed).

- **Tool palette UI redesign** (forward concern; Sub-phase 5 + Mediator Removal session use existing UI structure; full palette UI redesign is separate campaign).

---

## §11 — Phase status

This section must be updated in the same commit that completes each sub-phase per §0 discipline.

```text
Editor Multi-Tool Architecture campaign-design pass: COMPLETE 2026-05-04, commits 75b68e7c7 + 8fad61bd3 + 8c92890b9.
Sub-phase 1 — Diagnostic: COMPLETE 2026-05-04, commits 4556c267b (Diagnostic.A audit) + 0a7df3cdf (Diagnostic.B campaign doc update) + 6924e39db (Diagnostic.C hash-fixup). All ten §2.X commitments compatibility-confirmed; zero gap-evidence; 2 open-questions deferred to Sub-phase 2 + Sub-phase 5 prompt drafting; NO Andrew-gate triggered.
Sub-phase 2 — ActiveTool trait + dispatcher core + register_tool API: COMPLETE 2026-05-04, commits 813ac29a1 (Core.A trait + types + ToolContext) + 2c791fa39 (Core.B Dispatcher) + ece7bb3b4 (Core.C 15 unit tests + MockActiveTool fixture) + 6016b3c8f (Core.D campaign doc update). New module at tools/aw_editor/src/active_tool/ with mod.rs (~280 lines: trait + types) + dispatcher.rs (~190 lines: Dispatcher impl) + tests.rs (~470 lines: MockActiveTool + DefaultOnlyTool fixtures + 15 test scenarios). §2.7 ToolContext open question resolved via pre-computed world-XZ projection fields + world_xz_at_pointer/world_xz_at_y0 method accessors. Module isolation verified: Dispatcher + ActiveTool referenced only within active_tool/ (no external usages). Code-level only; NOT Andrew-gated per Q9.
Sub-phase 3 — TerrainPanel ActiveTool implementation (additive): IN PROGRESS — 3.A 0dea0bebc + 3.B 41ec3b192 landed (build clean; 15/15 active_tool tests pass); Andrew-gate REGRESS verdict 2026-05-04 surfaced upstream mediator brush regression (NOT caused by Sub-phase 3 per Andrew Q1 verification at 79e483e6c); Sub-phase 3.C closeout DEFERRED pending mediator brush fix.
Sub-phase 3 Mediator Brush Diagnostic — TerrainPanel brush mediator path failure-diagnosis: COMPLETE 2026-05-05, commits e5e32f486 (Diagnostic.A audit) + f5c96836f (Diagnostic.B campaign doc update). Single confirmed root cause: H5 mediator drain logic regression at f84eb09049 (2026-03-17) — viewport.set_terrain_brush_active + viewport.set_terrain_brush_params at main.rs:3869-3870 placed inside `if let (Some(world), Some(viewport))` gate, brush state never reaches viewport when no scene loaded. H1 (F.5-paint cascade) REFUTED. Recommended fix: small (~10 lines), single-commit, no architectural changes.
Sub-phase 3 Mediator Brush Fix — relocate brush sync calls outside Some(world) gate: NOT STARTED (gated on Andrew review of diagnostic findings).
Sub-phase 3.C — closeout: NOT STARTED (gated on mediator brush fix Andrew-gate PASS).
Sub-phase 4 — Pattern A regression infrastructure for dispatcher class: NOT STARTED.
Sub-phase 5 — RegionalArchetypePanel ActiveTool implementation + registration: NOT STARTED.
Dedicated Session — Mediator Removal: NOT STARTED.
Sub-phase 6 — Closeout: NOT STARTED.
```

Format for completion updates: `<sub-phase>: COMPLETE <YYYY-MM-DD>, commit <hash>`.

---

## §12 — Deviations log

This section records any design decisions made during execution that deviate from this plan. Every deviation must be recorded here before or in the same commit as the deviation itself.

Format for entries:

```text
### <YYYY-MM-DD>, Sub-phase <N>, commit <hash>
**Deviation:** <short description>
**Rationale:** <why>
**Impact:** <what parts of later sub-phases or other systems are affected>
```

### 2026-05-04, Campaign-design pass, commits 75b68e7c7 + 8fad61bd3 + 8c92890b9 (hash-fixup)

**No deviations from research audit §7 framework or Andrew Q1-Q10 strategic factors.** Campaign-design pass committed to Approach I+II hybrid synthesis per research audit §7.7 + concrete §2 architectural decisions per audit §8.1's ten enumeration. Sub-phase chain sized per Q8 scope-driven framing (6 sub-phases + 1 dedicated Mediator Removal session = 7 distinct campaign sessions per Q2 + Q6 + Q7 + Q8 strategic factors).

**Inheritance from Regional Archetype Variation §0** preserved per Q10: research-pass-before-reframe discipline pattern; Andrew-gate authoritative for visible-output sub-phases; sub-phase completion + status header maintenance discipline; §2 architectural commitments respected.

### 2026-05-04, Sub-phase 1 Diagnostic, commits 4556c267b (Diagnostic.A audit) + 0a7df3cdf (Diagnostic.B campaign doc update)

**Sub-phase 1 Diagnostic — captures pre-implementation feasibility audit findings. Cross-reference entry; the diagnostic audit at `docs/audits/editor_multi_tool_architecture_diagnostic_2026-05-04.md` is the load-bearing artifact.**

**Pre-execution verification** (per Sub-phase 1 prompt §1):
- §1.1 Campaign doc re-read for §2 commitments: complete; §2.X verification map produced inline.
- §1.2 Predecessor diagnostic methodology re-read: complete; F.5-paint.E-diagnostic §3 methodology revision + G-diagnostic §7 four-option decision-request precedent applied (§2-decision-organized structure per Q1; precedent-driven discovery per F.5-paint.E precedent).
- §1.3 Campaign doc state confirmed at commits 75b68e7c7 + 8fad61bd3 + 8c92890b9.
- §1.4 Anti-drift discipline reaffirmation: held; no implementation sketches; no §2 revision; no exploratory inspection beyond §3.1 scope.

**Deliverables**:

- **Diagnostic.A** (commit `4556c267b`): audit document at `docs/audits/editor_multi_tool_architecture_diagnostic_2026-05-04.md` (~470 lines, §1-§13 structure with §2-decision-organized verification per Q1).
- **Diagnostic.B** (this commit): cross-reference §12 entry + Status header line + §11 phase status block update in this campaign doc.

**Findings summary**:

**All ten §2.X commitments verified COMPATIBILITY-CONFIRMED. Zero gap-evidence findings; NO Andrew-gate triggered. Sub-phase 2 prompt drafting + execution may proceed with §2 commitments intact.**

Per-decision verification:

- **§2.2 ActiveTool trait shape**: COMPATIBILITY CONFIRMED. TerrainPanel + RegionalArchetypePanel API surfaces map cleanly to Fyrox-style trait. Identical signature `fn(&mut self, world_x: f32, world_z: f32)` across `apply_brush_at` (terrain_panel.rs:819) + `queue_paint_op` (regional_archetype_panel.rs:138).
- **§2.3 EventDisposition enum**: COMPATIBILITY CONFIRMED. Existing ViewportWidget event-handling has binary tool-consumed-or-camera-pass-through semantics implicitly (viewport/widget.rs:1180-1255 per G-diagnostic).
- **§2.4 Dispatcher mechanism**: COMPATIBILITY CONFIRMED. egui `Sense::click_and_drag()` + `Response` API produces discrete events compatible with push-based per-event dispatch.
- **§2.5 Registration model**: COMPATIBILITY CONFIRMED. Editor::new() pattern (main.rs:472+) structurally additive for `register_tool` calls. PanelType enum and ActiveTool registration orthogonal concerns.
- **§2.6 Mediator removal**: COMPATIBILITY CONFIRMED. Extent bounded — 51 grep occurrences across 3 files; 44 lines of mediator code (main.rs:3833-3877) + 6 ViewportWidget fields (viewport/widget.rs:163-177) + 4 handle_input branches + 6 tab_viewer accessors. Dedicated session scope per Q6 appropriately sized.
- **§2.7 ViewportWidget integration**: COMPATIBILITY CONFIRMED. Post-removal end-state achievable.
- **§2.8 Mutex arbitration**: COMPATIBILITY CONFIRMED. Existing single-active-tool pattern (hardcoded `terrain_brush_active`) generalizes cleanly to `active_tool: Option<Uuid>`.
- **§2.9 Tool composition**: COMPATIBILITY CONFIRMED. No cross-tool composition patterns; deferral uncomplicated. TerrainPanel BrushMode (terrain_panel.rs:509-595) correctly classified as tool-internal state.
- **§2.10 State persistence**: COMPATIBILITY CONFIRMED. Per-panel state patterns already each panel's responsibility.
- **§2.11 Action transactionality**: COMPATIBILITY CONFIRMED. Existing `EditorCommand` trait (command.rs:71) + `UndoStack` (command.rs:236) + `TerrainBrushCommand` (command.rs:1691) is appropriate integration target.

**Open questions** (do NOT trigger Andrew-gate; deferred per audit §12.4):

1. **§2.7 ToolContext exposure mechanism for depth-buffer access**: concrete API shape for `ToolContext::world_xz_at_pointer()` closure wrapping the existing depth-buffer read at viewport/widget.rs:1219-1234. **Deferral target**: Sub-phase 2 prompt drafting (when ToolContext type is concretely defined).

2. **§2.11 undo_stack access mechanism from tool event methods**: ergonomic choice between (a) pass `&mut UndoStack` through `ToolContext`; (b) tool emits `EditorAction` analog of TerrainAction with main.rs draining; (c) tool returns `Box<dyn EditorCommand>` from event methods. **Deferral target**: Sub-phase 5 prompt drafting (when second tool's undo/redo plumbing forces the choice).

**Methodology lessons** (audit §13):

- **§13.1 Pre-implementation feasibility audit pattern as canonical for foundational architectural campaigns**: three-mode evidence framework (compatibility / gap / open-question) + Andrew-gate-only-on-gap behavior + open-question deferral to downstream sub-phases. Future foundational campaigns inherit this pattern.
- **§13.2 §2-decision-organized vs hypothesis-classification audit structure**: choice based on audit purpose. Pre-implementation feasibility audits use §2-decision-organized; failure-diagnosis audits (F.5-paint.E-diagnostic, G-diagnostic) use hypothesis-classification.
- **§13.3 Single-reference-points discipline**: predecessor findings are reference points; bounded inspection scope expansion per §2.X verification need.

**Forward chain**:

1. **Sub-phase 2 prompt drafting** (next session): drafts execution prompt for ActiveTool trait + dispatcher core + register_tool API. References this audit's §2.7 open-question for ToolContext design + §2.5 PanelType-orthogonal classification + Editor::new() additive pattern.
2. **Sub-phase 2 execution** (Sessions 5+): per the Sub-phase 2 prompt.
3. Through Sub-phase 6 + Mediator Removal session per campaign doc §3-§9.

**Scope held**: Sub-phase 1 Diagnostic session only modified `docs/audits/editor_multi_tool_architecture_diagnostic_2026-05-04.md` (commit `4556c267b`) and this Editor Multi-Tool Architecture campaign doc Status header + §11 phase status block + §12 (this commit). NO production code changes. NO test changes. NO implementation sketches. NO sub-phase prompt drafting.

### 2026-05-04, Sub-phase 2 ActiveTool trait + dispatcher core + register_tool API, commits 813ac29a1 (Core.A) + 2c791fa39 (Core.B) + ece7bb3b4 (Core.C) + 6016b3c8f (Core.D)

**Sub-phase 2 — captures execution of foundational dispatcher infrastructure per campaign doc §4. Code-level only per Q9 (NOT Andrew-gated). Resolves §2.7 ToolContext open question per Sub-phase 1 Diagnostic audit §12.4.**

**Pre-execution verification** (per Sub-phase 2 prompt §1):

- §1.1 Campaign doc re-read for §2.2-§2.5 + §2.7-§2.8 commitments: complete.
- §1.2 Sub-phase 1 Diagnostic audit re-read for §12.4 deferrals + §2.X verification findings: complete; §2.7 open question resolved this sub-phase, §2.11 deferred to Sub-phase 5.
- §1.3 ViewportWidget inspection for ToolContext design: complete; OrbitCamera::ray_from_screen + unproject_depth_to_world located at viewport/camera.rs:511 + 541; ViewportRenderer::read_depth_at_pixel located at viewport/renderer.rs:1145; existing depth-then-Y0-fallback pattern at viewport/widget.rs:1219-1247; F.5-paint.B's `screen_to_world_xz_y0` standalone function located at panels/regional_archetype_panel.rs:414. ToolContext design committed to pre-computed world-XZ values (fields) + method accessors (avoids Arc<Mutex<Renderer>> lock + tight camera coupling).
- §1.4 Anti-drift discipline reaffirmation: held; 9 specific drift temptations enumerated and resisted (no ViewportWidget/main.rs/panels modifications; no TerrainPanel/RegionalArchetypePanel impls; no Pattern A regression infrastructure; no §2.11 undo_stack via ToolContext; no multi-active-tool support; no hover-feedback methods; no tool composition primitives; no §2 architectural revisions).

**Deliverables**:

- **Core.A** (commit `813ac29a1`): ActiveTool trait + EventDisposition enum + MouseEvent/KeyEvent/MouseEventKind/KeyEventKind types + ToolContext struct at `tools/aw_editor/src/active_tool/mod.rs`. lib.rs gets single-line `pub mod active_tool;` addition. Default-implementation pattern per Andrew Q4 (a)(b); `#[non_exhaustive]` enum per Q4. ToolContext owned struct (no lifetime parameter); pre-computed world-XZ fields + method accessors per Andrew Q2 (b). Per-event handlers default to PassThrough; lifecycle methods + on_mouse_enter/on_mouse_leave default to no-op; make_button defaults to selectable_label using name() per Andrew Q4 (b). UUID identity per Q5 mod-friendliness.

- **Core.B** (commit `2c791fa39`): Dispatcher struct + register_tool/set_active_tool/dispatch_mouse_event/dispatch_key_event/dispatch_mouse_enter/dispatch_mouse_leave/update_active_tool methods + helper accessors (is_registered/active_tool_uuid/registered_uuids/tool_count) at `tools/aw_editor/src/active_tool/dispatcher.rs`. Approach I+II hybrid synthesis per research audit §7.7. HashMap<Uuid, Box<dyn ActiveTool>> tool collection. Push-based per-event dispatch routes to active tool only. Framework-enforced single-active-tool mutex via active_tool: Option<Uuid>. Graceful handling: dispatch returns PassThrough if active_tool is None OR active tool's UUID is not registered. set_active_tool no-op for re-setting same UUID (avoids spurious deactivate/activate).

- **Core.C** (commit `ece7bb3b4`): module-level unit tests + MockActiveTool fixture + DefaultOnlyTool fixture at `tools/aw_editor/src/active_tool/tests.rs`. 15 test scenarios (12 prompt-spec + 3 additional). MockActiveTool tracks lifecycle + per-event calls via Rc<RefCell<MockToolState>> shared state; configurable return_disposition for per-event method returns. DefaultOnlyTool overrides ONLY required methods (uuid, name) to verify default-implementation pass-through.

- **Core.D** (this commit): campaign doc Status header + §11 phase status block + §12 entry.

**§2.7 ToolContext open question resolution per audit §12.4**:

ToolContext shape (final):

```rust
pub struct ToolContext {
    pub viewport_rect: egui::Rect,
    pub pointer_pos: Option<egui::Pos2>,
    pub modifiers: egui::Modifiers,
    world_xz_at_pointer_cached: Option<(f32, f32)>,  // private; method accessor
    world_xz_at_y0_cached: Option<(f32, f32)>,        // private; method accessor
}

impl ToolContext {
    pub fn new(viewport_rect, pointer_pos, modifiers, world_xz_at_pointer, world_xz_at_y0) -> Self;
    #[cfg(test)] pub fn for_test() -> Self;
    pub fn world_xz_at_pointer(&self) -> Option<(f32, f32)>;
    pub fn world_xz_at_y0(&self) -> Option<(f32, f32)>;
}
```

Design rationale per §1.3 ViewportWidget inspection:

- **Pre-computed world-XZ projections** rather than exposing camera + depth_buffer references directly. ViewportWidget computes per-frame before dispatching events; tools just read via method accessors. Avoids exposing `Arc<Mutex<Renderer>>` lock pattern (viewport/widget.rs:333 `with_renderer` closure) + tight coupling to OrbitCamera type.
- **Owned struct (no lifetime parameter)** simplifies dispatcher API + enables synthetic test fixtures via `for_test()` constructor without borrow orchestration.
- **Mutable scene-state access deferred** to Sub-phase 3+ tools' choice. TerrainPanel + RegionalArchetypePanel may add ToolContext fields, use existing pending_actions queue patterns, or pass scene references through other mechanisms. Sub-phase 5 prompt drafting may resolve when RegionalArchetypePanel registration forces the decision (analogous to §2.11 undo_stack deferral).
- **Methods wrap two canonical projection patterns**: `world_xz_at_pointer()` wraps depth-buffer + unprojection per viewport/widget.rs:1219-1234; `world_xz_at_y0()` wraps F.5-paint.B's `screen_to_world_xz_y0` ray-plane projection per regional_archetype_panel.rs:414.

**Test scoreboard at Sub-phase 2 close**:

- 15 active_tool::tests scenarios: pass.
- All upstream tests: still green (regression check; no other code modified).
- `cargo check -p aw_editor --lib`: clean (only pre-existing nalgebra warning unrelated).
- Module isolation: `Dispatcher` + `ActiveTool` referenced only within `active_tool/` (verified via grep).

**Forward chain**:

1. **Sub-phase 3 — TerrainPanel ActiveTool implementation (additive)** (next session): implements `impl ActiveTool for TerrainPanel`; adds dispatcher field to EditorApp + register_tool call at Editor::new(); ViewportWidget routes events to dispatcher (alongside existing main.rs mediator). Andrew-gate REQUIRED per Q9. Both ActiveTool path + main.rs mediator coexist additively per Q2 risk-bounding.

2. **Sub-phase 4 — Pattern A regression infrastructure** (later): extends MockActiveTool fixture for comprehensive dispatcher class regression coverage. Code-level only.

3. **Sub-phase 5 — RegionalArchetypePanel ActiveTool implementation + registration**: implements ActiveTool for RegionalArchetypePanel; resolves §2.11 undo_stack access mechanism per audit §12.4. Andrew-gate REQUIRED. Likely subsumes G-pointer-events-fix per research audit §8.4.

4. **Dedicated Mediator Removal session** + **Sub-phase 6 Closeout**.

**Scope held**: Sub-phase 2 only modified `tools/aw_editor/src/active_tool/` (3 new files: mod.rs + dispatcher.rs + tests.rs) + `tools/aw_editor/src/lib.rs` (single `pub mod active_tool;` addition) + this Editor Multi-Tool Architecture campaign doc Status header + §11 + §12 (this commit). NO modifications to ViewportWidget, panels, tab_viewer, main.rs runtime path. NO Pattern A regression infrastructure beyond minimal MockActiveTool fixture (Sub-phase 4's job). NO §2 architectural revisions.

### 2026-05-04, Sub-phase 3.A + 3.B (TerrainPanel ActiveTool implementation — additive), commits 0dea0bebc (3.A) + 41ec3b192 (3.B)

**Sub-phase 3 in-progress entry — captures execution of TerrainPanel ActiveTool integration. Sub-phase 3.C closeout DEFERRED pending Sub-phase 3 Mediator Brush Diagnostic findings + fix (see next entry).**

**Pre-execution verification** (per Sub-phase 3 prompt §1):
- Sub-phase 1 Diagnostic + Sub-phase 2 commitments preserved unchanged.
- §2.7 ToolContext API as resolved by Sub-phase 2 (pre-computed world-XZ fields + accessor methods) used as-is.

**Deliverables**:

- **Sub-phase 3.A** (commit `0dea0bebc`): `impl ActiveTool for TerrainPanel` block at `tools/aw_editor/src/panels/terrain_panel.rs` (~165 lines added) + `TERRAIN_PANEL_UUID` constant (`uuid!("a3f1b8c2-7e4d-4a5b-9f3c-1d2e8b7a4c6f")`) + `TerrainAction::SetActiveTool { uuid: Option<Uuid> }` variant + UI emission at brush-mode toggle (line ~1180: pushes SetActiveTool action when brush_enabled flips). main.rs gets single `mod active_tool;` declaration. Per-event handlers: on_left_mouse_button_down + on_mouse_move route to existing `apply_brush_at(world_x, world_z)` via `ToolContext::world_xz_at_pointer()`. on_left_mouse_button_up returns PassThrough per option (b) coexistence — existing main.rs:3891 mediator handles stroke-end. Throttling preservation per Sub-phase 1 Diagnostic audit §2.2.2.

- **Sub-phase 3.B** (commit `41ec3b192`): tab_viewer SetActiveTool capture (`pending_set_active_tool: Vec<Option<Uuid>>` field + capture in TerrainAction drain sites + `take_pending_set_active_tool_actions` accessor) + EditorApp.dispatcher field (initialized with TerrainPanel::default() registered) + per-frame drain in main.rs that applies pending SetActiveTool to dispatcher.set_active_tool + ViewportWidget cached-then-dispatch integration (8 cache fields + `cache_active_tool_events` helper called from ui() + `dispatch_cached_events` public method drained from main.rs). Cached-then-dispatch design rationale (vs prompt §2.4 illustrative sketch with `&mut Dispatcher` parameter): avoids threading parameter through 5 ui() call sites; preserves additive coexistence with existing main.rs:3833-3877 mediator path; isolates dispatcher integration to single accessor.

**Coexistence design** (per Q2 additive risk-bounding):

- Existing main.rs mediator path remains canonical functional path during Sub-phase 3.
- Dispatcher path is wired-up but inert: `dispatcher.register_tool(Box::new(TerrainPanel::default()))` creates a structurally-distinct TerrainPanel from the dock-rendered instance; its `apply_brush_at` receives no terrain state, so dispatch produces no observable effect. Mediator Removal session per Q6 resolves dual ownership.

**Test scoreboard at Sub-phase 3.B close**:
- 15/15 active_tool::tests pass.
- 3990 other lib tests pass.
- 1 pre-existing test_terrain_panel_creation failure (chunk_radius drift documented in F-fix.A-supplement; UNRELATED to Sub-phase 3; not investigated/fixed per anti-drift discipline).
- cargo build -p aw_editor: clean.

**Andrew-gate REGRESS 2026-05-04**:

Andrew tested all 8 brush modes per Q9 verification criteria. Verdict: REGRESS. All 8 modes non-functional on TerrainPanel-generated terrain (no cursor change, no ring overlay, no terrain modification on click+drag; no panic; FPS spike on paint mode switch). Q1 confirmed: pre-Sub-phase 3 also broken (verified at commit `79e483e6c` Sub-phase 2 Core.E pre-Sub-phase 3 state). Q4: "scult and flatten worked at one point in past"; uncertain when last functional. Defect predates Sub-phase 3 — surfaced by Sub-phase 3 Andrew-gate's required comprehensive verification, not introduced by Sub-phase 3.

Sub-phase 3.C closeout DEFERRED. Sub-phase 3 Mediator Brush Diagnostic session triggered per Andrew Q2 (diagnostic-only, no fixes; fix prompt drafted post-Andrew-review).

**Forward chain**:

1. **Sub-phase 3 Mediator Brush Diagnostic** (next session, this entry): root-cause TerrainPanel brush mediator regression. See next §12 entry.
2. **Mediator Brush Fix session** (post-Andrew-review): per diagnostic findings.
3. **Sub-phase 3 Andrew-gate re-run** (post-fix): all 8 brush modes verified.
4. **Sub-phase 3.C closeout** (post-Andrew-gate-PASS).
5. Sub-phase 4 + Sub-phase 5 + Mediator Removal session + Sub-phase 6 per campaign doc §3-§9.

**Scope held**: Sub-phase 3.A + 3.B only modified `tools/aw_editor/src/panels/terrain_panel.rs` + `tools/aw_editor/src/main.rs` + `tools/aw_editor/src/tab_viewer/mod.rs` + `tools/aw_editor/src/viewport/widget.rs`. NO modifications to existing main.rs:3833-3877 mediator code (preserved unchanged for additive coexistence). NO Sub-phase 4/5/6 work scope. NO Mediator Removal work.

### 2026-05-05, Sub-phase 3 Mediator Brush Diagnostic, commits e5e32f486 (Diagnostic.A audit) + f5c96836f

**Sub-phase 3 sub-task — failure-diagnosis of TerrainPanel brush mediator path regression. Triggered by Sub-phase 3 Andrew-gate REGRESS 2026-05-04. Defect predates Sub-phase 3 per Andrew Q1 verification. Cross-reference entry; the diagnostic audit at `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_2026-05-05.md` is the load-bearing artifact.**

**Pre-execution verification** (per Sub-phase 3 Mediator Brush Diagnostic prompt §1):
- §1.1 Sub-phase 3 status confirmed (3.A `0dea0bebc` + 3.B `41ec3b192` landed; 3.C deferred; build clean; 15/15 active_tool tests pass).
- §1.2 Sub-phase 3 not-causal confirmed (3.A modifies `panels/terrain_panel.rs` + single `mod active_tool;` line in main.rs; 3.B adds new fields/methods to tab_viewer/main.rs/viewport/widget.rs but does NOT modify existing mediator branches at main.rs:3833-3902 nor handle_input branches at viewport/widget.rs brush sections).
- §1.3 Predecessor diagnostic methodology re-read: F.5-paint.E-diagnostic + G-pointer-events-diagnostic precedent applied.
- §1.4 F.5-paint cascade prime-suspect framing applied as initial investigation target — ultimately REFUTED (F.5-paint commits did NOT modify mediator-relevant files).
- §1.5 Anti-drift discipline reaffirmation: held; 5 specific drift temptations enumerated and resisted (no fix sketching beyond §5.3 recommended scope; no architectural refactoring; no scope expansion to non-mediator concerns; no FPS-spike optimization; no unrelated fixes).

**Deliverables**:

- **Diagnostic.A** (commit `e5e32f486`): audit document at `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_2026-05-05.md` (~470 lines, hypothesis-classification structure per Sub-phase 1 Diagnostic audit §13.2 + F.5-paint.E-diagnostic + G-pointer-events-diagnostic precedent).
- **Diagnostic.B** (this commit): cross-reference §12 entry + Status header line + §11 phase status block update.

**Findings summary — single confirmed root cause**:

H5 (Mediator drain logic regression) CONFIRMED at commit `f84eb09049` (2026-03-17, "imported kaykit complete asset package"). The brush sync calls at `tools/aw_editor/src/main.rs:3869-3870`:

```rust
viewport.set_terrain_brush_active(brush_active);
viewport.set_terrain_brush_params(brush_radius, brush_is_paint);
```

are placed inside an `if let (Some(world), Some(viewport)) = (world_opt, self.viewport.as_mut())` gate at line 3867. The `Some(world)` arm requires `self.scene_state` to be `Some`. When user generates terrain via TerrainPanel without first loading a scene, `scene_state` is `None`, `world_opt` is `None`, the `if let` arm doesn't execute, and `viewport.terrain_brush_active` remains its default `false` — silently defeating all three brush-active gates in viewport/widget.rs (lines 1365 hit collection, 1423 stroke-end, 1428 cursor + ring render).

Single defect explains ALL three observable symptoms parsimoniously:
- "No cursor change on brush activation" ← line 1428 gate false
- "No ring overlay on terrain hover" ← same line 1428 gate (cursor + ring share render block)
- "No terrain modification on click+drag" ← line 1365 gate false

Andrew's Q4 "scult and flatten worked at one point" memory consistent with path-conditional behavior: brush works when scene IS loaded; silently fails when terrain is generated standalone. Bug has been latent on main for ~7 weeks since `f84eb09049`.

**Hypotheses refuted**: H1 (F.5-paint cascade) — F.5-paint commits did NOT modify mediator-relevant files; H3 (TerrainPanel state machine) — internal logic correct; H4 (ViewportWidget input flow) — handle_input branches correct; H6 (tab_viewer accessor) — single-instance ownership + correct delegation; H8 (build-system / dependency) — no relevant Cargo or cfg changes.

**Hypothesis partially confirmed (secondary observation)**: H7 (FPS spike on paint mode switch) — `ensure_thumbnails_loaded` first-call cost at terrain_panel.rs:1257 is bounded one-frame quirk; NOT brush regression cause.

**Recommended fix scope per audit §5.3**: SMALL (~10 lines), single-commit, no architectural changes, no API changes. Relocate the two setter calls outside the `Some(world)` gate into a separate `if let Some(viewport) = self.viewport.as_mut()` block. Two sequential `&mut self.viewport` borrows in the same closure are non-overlapping; borrow checker accepts.

**Methodology lessons** (audit §7):
- §7.1 Sub-phase Andrew-gate as defect-discovery mechanism: comprehensive verification surfaces latent defects in surrounding code beyond current sub-phase scope.
- §7.2 Sub-phase Andrew-gate REGRESS-not-from-sub-phase pattern: verify causality before assuming sub-phase regression; route fix per causality (in-scope expansion vs separate session).
- §7.3 Symptom-to-code-path tracing as primary investigation tool when prime-suspect framing is wrong.
- §7.4 "Catch-all gate" anti-pattern: independent state syncs should not aggregate under a conjunction stricter than necessary.

**Forward chain**:

1. **Andrew reviews diagnostic findings** (next session): Andrew assesses confirmed root cause + recommended fix scope; decides on fix prompt drafting approach.
2. **Mediator brush fix session**: per Andrew's call. Likely small (1-3 commits) once root cause known. Fix re-runs Sub-phase 3 Andrew-gate post-fix.
3. **Sub-phase 3.C closeout**: lands after fix Andrew-gate PASSES.
4. **Sub-phase 4 + Sub-phase 5 + Mediator Removal session + Sub-phase 6**: per campaign doc.

**Scope held**: Sub-phase 3 Mediator Brush Diagnostic session only modified `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_2026-05-05.md` (commit `e5e32f486`) and this Editor Multi-Tool Architecture campaign doc Status header + §11 phase status block + §12 (this commit). NO production code changes. NO test changes. NO fixes. NO Sub-phase 3.C closeout. NO modifications to Sub-phase 3.A + 3.B commits (preserved unchanged).

---

*End of plan*
