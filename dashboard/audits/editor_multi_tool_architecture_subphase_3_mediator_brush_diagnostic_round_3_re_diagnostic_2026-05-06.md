# Editor Multi-Tool Architecture — Sub-phase 3 Mediator Brush Round 3 Re-Diagnostic Audit

**Status**: Synthesis of three rounds of prior runtime evidence. Defect class **rendering-pipeline failure between draw call and visible pixel** identified post-Round-2. 8 hypothesis classes enumerated; PLAUSIBLE: Classes 1 (world-to-screen projection off-screen), 2 (render target mismatch); PLAUSIBLE-MEDIUM: 3 (z-depth occlusion), 5 (coordinate-system mismatch), 6 (terrain mesh not regenerating); PLAUSIBLE-LOW: 4 (color/alpha invisibility), 7 (frame-timing); ACKNOWLEDGED: 8 (unexpected mechanism). Round 4 instrumentation specifications: 8-12 eprintln targets across hypothesis classes; coverage commitment that every PLAUSIBLE class has at least one runtime check. Mediator Brush Fix `8f4668599` retroactively classified as harmless redundancy (revert deferred to real-fix session for clean recovery narrative). 5 methodology lessons codified for future foundational architectural campaigns.

**Author**: Round 3 Re-Diagnostic session, 2026-05-06.

**Scope**: Failure-diagnosis re-diagnostic synthesizing three rounds of prior runtime evidence + enumerating remaining hypothesis space for rendering-pipeline defect class. Specifies Round 4 instrumentation targets per hypothesis class. NO production code changes; NO Round 4 instrumentation eprintlns in this session (deferred per Andrew's two-session-minimum directive); NO fix sketches.

**Predecessors**:
- Original Sub-phase 3 Mediator Brush Diagnostic audit at `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_2026-05-05.md`. **§4.4 H4 REFUTAL wrong; §4.5 H5 mechanism wrong; §4.7 H7 classification likely wrong.** Audit's confidence calibration is methodology-lesson territory.
- Round 1 captured output codified in `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Round-1-Closure §12 entry (commit `df4a50e74`).
- Round 2 captured output codified in this audit's load-bearing input + Round-2-Closure §12 entry (lands as part of Round-3-Re-Diagnostic.B campaign doc update).
- Mediator Brush Fix commit `8f4668599` preserved (harmless redundancy; revert deferred).
- Editor Multi-Tool Architecture campaign doc + Sub-phase 1 Diagnostic audit + research audit + G-pointer-events-diagnostic per established predecessor chain.

---

## §1 — Synthesis of Three Rounds of Prior Evidence

### §1.1 Original Sub-phase 3 Mediator Brush Diagnostic findings + retroactive classification

The original diagnostic audit (commit `e5e32f486`, 2026-05-05) used hypothesis-classification structure per Sub-phase 1 Diagnostic audit §13.2 methodology lesson. Eight hypotheses enumerated; H5 confidently CONFIRMED; H1-H4 + H6-H8 REFUTED or non-causal-secondary-observation. Recommended fix scope SMALL (~10 lines).

**Three load-bearing claims now retroactively identified as wrong**:

- **§4.4 H4 ("ViewportWidget input-flow regression") REFUTAL**. Original audit asserted: "Sense::click_and_drag() is correct sense for click+drag detection. handle_input branches correct; depth-buffer pick correct; default field values correct. Bug is upstream — main.rs never tells the widget about brush state." Round 2 evidence contradicts: egui DOES capture input correctly when `terrain_brush_active=true`; the "bug is upstream" framing was wrong. The §4.4 REFUTAL was code-reading-derived; never runtime-verified.

- **§4.5 H5 ("Mediator drain logic regression") CONFIRMATION**. Original audit asserted: "scene_state is None when no scene is loaded; world_opt is None; the if let arm doesn't execute; viewport.terrain_brush_active remains false." Round 1 §1.3 pre-execution discovery contradicts: `EditorApp::new()` at main.rs:2700-2703 auto-populates scene_state with default world during construction; main.rs:5598-5601 has secondary safety net. **scene_state is NEVER None after editor startup.** The H5 mechanism was wrong; the gate was never blocking at runtime; the Mediator Brush Fix at `8f4668599` was harmless but redundant.

- **§4.7 H7 ("FPS spike on paint mode switch") classification**. Original audit classified as secondary observation: `ensure_thumbnails_loaded` first-call cost. Post-Mediator-Brush-Fix, the FPS spike disappeared, suggesting H7 was wrong + something fix-related caused the disappearance. Round 2 evidence (FPS spike reappeared) suggests H7 was right after all; net classification: secondary observation regardless. Methodology lesson: behavioral changes following fixes can be coincidence; require runtime verification before concluding causation.

**Audit's load-bearing input was code-reading, not runtime verification.** The original audit's confidence calibration didn't distinguish between code-reading-derived and runtime-verified claims. All three wrong claims rested on code-reading. This is methodology-lesson territory (§7.1 + §7.4 below).

### §1.2 Round 1 captured output + outcome classification

Per Round-1-Closure §12 entry (commit `df4a50e74`, 2026-05-06):

Round 1 captured output (frames at 2026-05-06T01:13:18-20Z timestamps; ~60 frames; consistent every frame):

```
[BRUSH-DBG] scene_state.is_some()=true, world_opt.is_some()=true
[BRUSH-DBG] tab_viewer-gate: viewport=true, world=true, entity=true, undo=true, full=true
[BRUSH-DBG] handle_input-entry: terrain_brush_active=true, terrain_brush_radius=50, terrain_brush_is_paint=false
[BRUSH-DBG] hit-gate: brush_active=true, no_gizmo=true, pointer=false, full=false
[BRUSH-DBG] render-gate: brush_active=true, no_gizmo=true, full=true
```

AstraWeave's own Selection check log: `clicked=false, pointer_over=true, gizmo_active=false`.

**Hypotheses contradicted at Round 1**:
- Sub-phase 3 Mediator Brush Diagnostic H5 (scene_state-None-blocks-setter): **REFUTED**. scene_state always Some.
- tab_viewer:142 catch-all gate (Round 1 prime suspect surfaced in §1.3 pre-execution discovery): **REFUTED**. Gate fires every frame.
- (Initial conclusion at Round 1 close) Audit §4.4 H4 REFUTAL: "WRONG. egui response is broken — `pointer=false` despite confirmed click+drag." Round 2 retrospectively shows this conclusion was itself wrong (capture-timing artifact misled the framing).

**Defect framing at Round 1 close**:
- Defect A — Input routing (egui not capturing click+drag).
- Defect B — Render body silent no-op (render-gate full=true yet no visible cursor/ring).

**Round 1 forward chain**: Round 2 instrumentation supplement targeting Defect A + Defect B disambiguation.

### §1.3 Round 2 captured output + outcome classification

Per Round 3 prompt §0.1 + §4.1 (load-bearing input for this audit):

Round 2 captured output during active brush use:

```
[BRUSH-DBG] response-state: rect=(236,52)..(1250,614), contains_pointer=true,
  dragged_primary=true, clicked_primary=false, is_pointer_button_down_on=true,
  sense_click=true, sense_drag=true
[BRUSH-DBG] global-pointer: any_pressed=false, any_released=true,
  primary_clicked=false, primary_down=false, latest_pos=Some([860.8 296.0])
[BRUSH-DBG] render-body-pointer: hover_pos=Some([621.6 452.0]),
  response_rect=(236,52)..(1250,614)
[BRUSH-DBG] render-body-drawcall: cursor_center=(809.78,888.48,809.96),
  radius=50.0, line_count=48, depth_pick_succeeded=true, color=[0.2, 1.0, 0.3]
[BRUSH-DBG] layer-at-pointer: pos=(1134,138),
  layer_id=Some(LayerId { Foreground F84F }),
  viewport_layer=Some(LayerId { Background 5A3E })
```

AstraWeave's own logs during the same session: `Mouse released: press=[752.0 429.6], release=[860.8 296.0], drag_dist=172.3` + `Brush stroke end: 1 chunks modified` + `Recorded: Terrain Sculpt`.

**Hypotheses contradicted at Round 2**:
- Defect A (input routing) framing: **REFUTED**. egui captures input correctly during click+drag — `contains_pointer=true, dragged_primary=true, is_pointer_button_down_on=true, sense_click=true, sense_drag=true`. Sense flags correct.
- Defect B (render body silent no-op) framing: **PARTIALLY REFUTED**. Render body executes every frame with valid-looking parameters: cursor_center=(809.78,888.48,809.96), radius=50.0, line_count=48, depth_pick_succeeded=true, color=[0.2, 1.0, 0.3]. NOT silently skipped.
- Round 1 `pointer=false` data was misleading capture-timing artifact: Andrew's actual click+drag timing didn't align with the capture window when only Round 1's hit-gate filter was firing (`brush_active OR pointer`).

**Brush IS fully functional at the data layer**:
- Real input captured (drag_dist=172.3).
- Brush hits flow through mediator drain path (`apply_terrain_brush_at` called).
- Terrain chunk modified (1 chunks modified).
- Undo entry recorded (`Recorded: Terrain Sculpt`).

**Yet Andrew sees nothing visually**. No cursor change. No ring overlay. No visible terrain modification (despite "1 chunks modified" log).

**Defect class identified post-Round-2**: rendering-pipeline failure between draw call and visible pixel. Narrower than original "TerrainPanel brush mediator path regression" framing. The hypothesis space pivots from data-flow to rendering-pipeline.

**Outcome classification per Round 2 prompt §0.4**: Outcome 6 — unexpected mechanism. Original Defect A + Defect B framings revealed incomplete/wrong; the actual defect class wasn't enumerated in Round 2's outcome decision tree.

### §1.4 Cumulative evidence inventory (runtime-verified true / false / unknown)

Per Round 3 prompt §1.3 codified inventory:

**Runtime-verified true** (Round 1 + Round 2):
- `scene_state.is_some()=true` every frame (Round 1).
- tab_viewer:142 catch-all gate fires `full=true` every frame (Round 1).
- `handle_input` runs every frame; `terrain_brush_active=true, terrain_brush_radius=50` propagates correctly (Round 1).
- egui response captures input: `sense_click=true, sense_drag=true, contains_pointer=true, dragged_primary=true, is_pointer_button_down_on=true` during sustained click+drag (Round 2).
- Brush hits land in mediator drain path: `Mouse released ... drag_dist=172.3` + `Brush stroke end: 1 chunks modified` + `Recorded: Terrain Sculpt` (Round 2).
- Render body executes every frame with valid-looking parameters: `cursor_center=(world coords), radius=50.0, line_count=48, depth_pick_succeeded=true, color=[0.2, 1.0, 0.3]` (Round 2).
- Geometry: `response_rect=(236,52)..(1250,614)` is the active viewport rect during sustained interaction; matches AstraWeave's editor-frame default viewport position-and-size (Round 2).
- Layer at pointer alternates between viewport's Background layer (when pointer over viewport) and various Foreground layers (when pointer over panel headers / docked tabs) (Round 1+2).

**Runtime-verified false** (Round 1 + Round 2):
- Original H5 mechanism (`Some(world)` gate blocks setter when no scene loaded).
- Defect A framing (egui not capturing input).
- Defect B framing as "render body silent no-op" (render body executes; whatever's wrong is downstream of the draw call execution).

**Unknown** (the hypothesis space this audit enumerates for Round 4):
- Where (809.78, 888.48, 809.96) projects to in screen-space pixel coordinates given current camera matrices.
- Whether the ring's draw call (via `set_brush_cursor_lines` at viewport/widget.rs:~1527) reaches the same render target Andrew sees on screen.
- Whether the ring's z-depth places it correctly relative to terrain (potentially behind terrain due to z-fighting).
- Whether the alpha channel of color=[0.2, 1.0, 0.3] is non-zero in the actual render.
- Whether camera matrices used by ring drawing match those used by terrain rendering.
- Whether terrain mesh actually regenerates after `1 chunks modified` log (despite log firing).
- Whether painter clear-and-redraw cycles preserve the ring's draw call.

---

## §2 — Methodology

### §2.1 Hypothesis-classification structure with synthesis emphasis

Per Sub-phase 1 Diagnostic audit §13.2 methodology lesson: failure-diagnosis audits enumerate hypotheses; collect evidence per hypothesis; identify confirmed.

This re-diagnostic differs from the precedent in three ways:

1. **Prior rounds' evidence is the load-bearing input**, not new code reading. Three prior rounds proved code-reading-derived hypotheses fail in this stack. The synthesis identifies what's RULED OUT by prior evidence + enumerates remaining hypothesis CLASSES (not single mechanisms).

2. **Hypotheses are enumerated as CLASSES**. Each class names a defect mechanism class; specifies prior-rounds evidence relevant to the class; specifies Round 4 instrumentation targets that would confirm/refute. Specific mechanism within each class requires Round 4 runtime verification.

3. **Plausibility ratings** (PLAUSIBLE / PLAUSIBLE-LOW / PLAUSIBLE-MEDIUM / RULED OUT / ACKNOWLEDGED) replace per-hypothesis CONFIRMED / REFUTED classifications. The plausibility framing acknowledges that confirmation requires runtime verification not present in this re-diagnostic.

### §2.2 Inspection scope

Code-reading bounded to:
- Identifying instrumentation locations for Round 4.
- Cross-referencing prior audit findings against current code (e.g., does Sub-phase 1 Diagnostic's render body location match current viewport/widget.rs structure post-Sub-phase-3 + post-fix?).
- Identifying camera state location, render-pass-setup location, terrain-mesh-regeneration site for hypothesis classes that require these locations.

NOT to:
- Deriving new hypothesis mechanisms from code reading (three prior rounds proved this fails).
- Sketching fixes.
- Architectural critique.

Specific files for hypothesis-class identification:
- `tools/aw_editor/src/viewport/widget.rs:~1466-1530` — render body. Ring drawing via `set_brush_cursor_lines(lines)` at line 1527 + Vec::new() fallbacks at lines 1529, 1532, 1535.
- `tools/aw_editor/src/viewport/renderer.rs:~832` — `set_brush_cursor_lines` setter; appends to renderer's debug-line pipeline.
- `tools/aw_editor/src/viewport/renderer.rs:~558` — combined-debug-lines render path; brush_cursor_lines extended into shared lines Vec.
- `tools/aw_editor/src/viewport/camera.rs` — OrbitCamera::ray_from_screen + unproject_depth_to_world per Sub-phase 1 Diagnostic.
- `tools/aw_editor/src/panels/terrain_panel.rs:~836-877` — apply_brush_at + terrain_state.apply_brush + GPU mesh upload trigger path.

### §2.3 Hypothesis class enumeration approach

Each hypothesis class:
- Names defect mechanism CLASS.
- Cites prior-rounds evidence relevant to class.
- Specifies Round 4 instrumentation targets.
- Classifies plausibility.

Coverage commitment: every PLAUSIBLE class has at least one Round 4 instrumentation target.

Round 4 prompt drafting (separate session per Andrew's two-session-minimum directive) translates §3 instrumentation specifications into actual eprintln code with filter patterns + outcome decision tree.

---

## §3 — Hypothesis Classes for Rendering-Pipeline Defect

Eight classes; coverage commitment that PLAUSIBLE classes (1, 2) and PLAUSIBLE-MEDIUM classes (3, 5, 6) are instrumentation targets for Round 4.

### §3.1 Hypothesis Class 1 — World-to-screen projection lands ring outside visible viewport rect

**Mechanism class**: cursor_center world coords project to screen pixel coords outside the viewport's visible rect. Ring drawn at off-screen pixel coordinates; not visible.

**Evidence supporting plausibility**:
- cursor_center=(809.78, 888.48, 809.96) per Round 2 — Y=888 plausibly mountain peak per Andrew's context but world coords appear far from camera origin.
- response_rect=(236,52)..(1250,614) — viewport ~1014×562 pixels in middle-right of screen.
- No prior instrumentation captured world-to-screen projection of cursor_center.
- Camera state (position, view, projection) unverified at runtime; could place ring at off-screen.

**Round 4 instrumentation targets**:
- T1.A: Compute and log screen-pixel position of cursor_center via `camera.world_to_screen()` or equivalent (apply view-projection matrix; viewport transform). Throttled.
- T1.B: Log camera position + camera target + view direction at draw call time.
- T1.C: Log signed-distance from cursor_center to camera (if very large, ring may be outside far clip plane).
- T1.D: Compute screen-space radius (project world radius=50 to pixels at given camera distance) — relevant to Class 4.

**Confirmation criterion**: if screen position from T1.A is outside response_rect bounds → HYPOTHESIS CONFIRMED. If inside response_rect → HYPOTHESIS REFUTED for this class; investigate other classes.

**Plausibility**: **PLAUSIBLE**. Prime candidate. World coords appear valid but never confirmed correctly project. Consistent with "draw call executes correctly but invisible" pattern.

### §3.2 Hypothesis Class 2 — Render target mismatch (ring drawn to wgpu pass not composited into visible egui frame)

**Mechanism class**: AstraWeave editor uses both egui (UI) and wgpu (3D rendering). Ring's `set_brush_cursor_lines` adds to renderer's debug-line Vec. If debug-line pipeline renders to a target that isn't composited into egui's frame (e.g., a separate render target only displayed in legacy non-dock viewport at main.rs:5614), the ring lands in invisible target.

**Evidence supporting plausibility**:
- AstraWeave's editor architecture per Sub-phase 1 Diagnostic uses both egui dock + wgpu render-to-texture + viewport image painted into egui.
- viewport/widget.rs uses ViewportRenderer (wgpu) AND egui painter; dock-rendered viewport vs non-dock viewport at main.rs:5614 may differ.
- Per main.rs structure: dock-based viewport routes through tab_viewer:142 (confirmed firing via Round 1); non-dock viewport at main.rs:5614 may have different debug-line composite path.
- Unverified: whether `brush_cursor_lines` extension in renderer.rs:558 actually reaches the rendered output user sees on screen.

**Round 4 instrumentation targets**:
- T2.A: Log which render path (dock-based viewport.ui() vs main.rs:5614 standalone) is being rendered each frame.
- T2.B: Log the renderer's `brush_cursor_lines.len()` at composite time (in renderer's render-pass-setup function, not the setter).
- T2.C: Log whether the renderer's combined debug-line buffer is uploaded to GPU + bound for draw.

**Confirmation criterion**: if T2.B shows `brush_cursor_lines.len()=0` despite Round 2 evidence of `line_count=48` going to `set_brush_cursor_lines` → field mutation lost between setter and render. If T2.A reveals dock-viewport rendered but T2.C shows debug-line buffer not bound → render path issue.

**Plausibility**: **PLAUSIBLE**. Editor architecture allows for this mistake. Consistent with Round 2 evidence pattern.

### §3.3 Hypothesis Class 3 — Z-depth occlusion (ring drawn behind terrain mesh)

**Mechanism class**: Ring drawn at correct world coords + correct screen coords + correct render target, but at z-depth that places it behind terrain mesh in z-buffer. Terrain occludes ring even though ring is "on" the surface.

**Evidence supporting plausibility**:
- depth_pick_succeeded=true per Round 2 — ring's world Y comes from depth pick at terrain surface (cursor_center Y=888.48).
- Ring built at `y = center.y + 0.15` per viewport/widget.rs:1520 — only 0.15 units above surface; at world distance ~800+ from camera, this offset may be sub-pixel in z-space.
- If z-test mode is `LessOrEqual` and depth bias is zero, ring at near-identical depth to terrain z-fights.
- If z-test mode is `Less`, ring at exactly terrain depth fails the test entirely.

**Round 4 instrumentation targets**:
- T3.A: Log depth value (z-coord) used for ring drawing per-line in screen space (post-projection z, not world z).
- T3.B: Log depth-test mode used for debug-line pipeline (less, less-equal, always, disabled).
- T3.C: Log terrain mesh's depth-test mode for comparison.
- T3.D: Log depth-bias settings (if any) for debug-line pipeline.

**Confirmation criterion**: if T3.A's projected z ≈ terrain surface z AND T3.B is Less (not LessOrEqual) → ring fails depth test, occluded. If T3.B is LessOrEqual but no depth bias → z-fighting causes intermittent visibility.

**Plausibility**: **PLAUSIBLE-MEDIUM**. Common rendering bug; consistent with "ring exists in render but invisible". Lower than Class 1+2 because the 0.15 Y offset suggests intent to avoid occlusion.

### §3.4 Hypothesis Class 4 — Color/alpha/visibility properties effectively-invisible

**Mechanism class**: Ring drawn at correct location + correct render target + correct depth, but with properties that make it visually indistinguishable (alpha=0; color matches background; line width sub-pixel).

**Evidence supporting plausibility**:
- color=[0.2, 1.0, 0.3] is bright green per Round 2 — but **ALPHA NOT LOGGED**; could be 0 if astraweave_physics::DebugLine::new sets alpha to 0 by default.
- Line count=48 per Round 2 — but **LINE WIDTH not logged**; could be sub-pixel at world distance 800+.
- Ring at 800+ world units from camera could result in screen-space line width < 1 pixel.

**Round 4 instrumentation targets**:
- T4.A: Log full color including alpha channel at DebugLine creation.
- T4.B: Log debug-line pipeline's line width setting.
- T4.C: Compute and log effective screen radius (project world radius=50 to pixels via T1.D).
- T4.D: Log whether debug-line pipeline applies any color tinting / fog / alpha modulation between submission and render.

**Confirmation criterion**: if T4.A alpha=0 → ring invisible. If T4.B line width=1 + T4.C screen radius < 5 pixels → visible but easily missed.

**Plausibility**: **PLAUSIBLE-LOW**. Less likely than Classes 1-3 because color RGB values look reasonable; but worth verifying since alpha unlogged and astraweave_physics::DebugLine struct shape unknown.

### §3.5 Hypothesis Class 5 — Coordinate system mismatch (ring uses one camera; terrain uses another)

**Mechanism class**: Ring drawing uses camera matrices that differ from terrain rendering's matrices. Ring projects to wrong screen position even though world coords are correct.

**Evidence supporting plausibility**:
- AstraWeave's editor potentially has separate cameras for different rendering passes (e.g., gizmo overlay camera vs scene camera per Sub-phase 1 Diagnostic).
- ViewportRenderer maintains its own camera state; viewport widget's OrbitCamera maintains camera state used for `unproject_depth_to_world` and `ray_from_screen`. If these diverge, projections diverge.
- Matrix uniforms could be stale (set once but not updated each frame) or use different binding slots.

**Round 4 instrumentation targets**:
- T5.A: Log OrbitCamera position + view + projection matrices at ring draw call time.
- T5.B: Log ViewportRenderer's camera matrices at terrain render time.
- T5.C: Compare via float-tolerance equality.

**Confirmation criterion**: if T5.A != T5.B beyond floating-point tolerance → coordinate-system mismatch confirmed.

**Plausibility**: **PLAUSIBLE-MEDIUM**. Architectural complexity allows for this; consistent with "ring code valid but ring invisible". Lower than Class 1+2 because most editors use single shared camera state.

### §3.6 Hypothesis Class 6 — Terrain mesh not regenerating after modification

**Mechanism class**: `Brush stroke end: 1 chunks modified` log fires + terrain_state internal data updates, but terrain GPU mesh isn't regenerated (or is regenerated but using stale buffer, or uploaded but shader doesn't read updated buffer).

**Evidence supporting plausibility**:
- Andrew sees no visible terrain modification after brush stroke despite "1 chunks modified" log per Round 2.
- terrain_state.apply_brush at terrain_panel.rs:863 modifies CPU heightmap; subsequent GPU mesh upload via `take_dirty_chunks` (tab_viewer/mod.rs:1308) + viewport.upload_terrain_chunks at main.rs:5614+ region requires both calls to fire.
- If take_dirty_chunks is called but upload_terrain_chunks misses the dirty IDs, GPU stays stale.
- If GPU mesh updates but shader's index buffer / material binding is wrong, render uses old mesh.

**Round 4 instrumentation targets**:
- T6.A: Log terrain chunk dirty-flag state pre-brush-stroke + post-brush-stroke (chunk count + IDs).
- T6.B: Log terrain mesh upload calls (upload_terrain_chunks) — frame number + chunk count + chunk IDs.
- T6.C: Log shader binding state for terrain mesh buffer (binding slot + buffer ID + buffer size).

**Confirmation criterion**: if T6.A shows chunks dirtied but T6.B shows no upload → upload path broken. If T6.B shows uploads but T6.C shows old buffer bound → GPU binding stale.

**Plausibility**: **PLAUSIBLE-MEDIUM**. Explains "modification logged but invisible terrain change" but doesn't explain "ring not visible" — Class 6 may co-exist with Classes 1-5 (the ring-not-visible is one defect; terrain-not-updating is a second).

### §3.7 Hypothesis Class 7 — Frame-timing / state-mutation issue

**Mechanism class**: Ring drawing draw call queued correctly but discarded by subsequent state change in same frame; OR draw call queued for next frame but next frame overwrites; OR painter clear-and-redraw cycle clears ring before frame composite.

**Evidence supporting plausibility**:
- AstraWeave's editor renders multi-pass (egui + wgpu + dock + non-dock viewport). Ring's draw call could be cleared by later pass.
- `set_brush_cursor_lines(Vec::new())` is called at line 1529, 1532, 1535 in fallback paths; if any unexpectedly executes after the success path, the lines are wiped.
- `set_brush_cursor_lines(lines)` at line 1527 followed by an else branch at 1529 — control flow is exclusive but worth verifying.

**Round 4 instrumentation targets**:
- T7.A: Log every call to `set_brush_cursor_lines` with caller identification and lines.len() — captures both the success-path call and any fallback-path Vec::new() calls.
- T7.B: Log frame number / monotonic counter to verify ordering.

**Confirmation criterion**: if T7.A shows pattern `set_brush_cursor_lines(48 lines) → set_brush_cursor_lines(0 lines)` within same frame → success-path ring wiped by fallback-path call.

**Plausibility**: **PLAUSIBLE-LOW**. Less common bug pattern; control flow inspection of viewport/widget.rs:1466-1535 suggests exclusive branches; but worth verifying.

### §3.8 Hypothesis Class 8 — Something else not yet enumerated

**Mechanism class**: Defect class not covered by Classes 1-7. Round 4 instrumentation may surface unexpected mechanism (per Round 2 outcome 6 pattern that surprised the original framing).

**Evidence supporting plausibility**:
- Three rounds of being wrong about hypothesis space suggests humility. Past pattern: original Defect A + Defect B framing was incomplete; Round 2 surfaced Outcome 6 (unexpected).
- Round 4 instrumentation should be aggressive enough that unexpected mechanisms surface as anomalies in the data.

**Round 4 instrumentation targets**:
- T8.A: Log debug-line pipeline's full state (any flags / settings / context not covered by Classes 1-7) at render time.
- T8.B: Log any panic / wgpu error / shader compilation warning that fires during brush use (currently shadowed by tracing::error filtering).
- General: ensure Round 4 instrumentation captures enough "context" data that unexpected patterns become visible.

**Plausibility**: **ACKNOWLEDGED**. Cannot rule out without enumeration. Three-rounds-wrong precedent suggests this class is non-trivially likely.

---

## §4 — Round 4 Instrumentation Specifications

### §4.1 Aggressive instrumentation scope

Per Andrew's directive: aggressive (~8-12 points). Cost of "still inconclusive after Round 4" is high after three rounds. Better to instrument heavier this round and resolve with confidence.

Filter patterns prevent log spam:
- State-change filters (log only on transitions).
- Throttling (every Nth frame).
- First-N-frames-only (one-time geometry / camera-state-startup logging).
- "Interesting-events-only" guards (log only when brush-relevant action occurs).

### §4.2 Instrumentation targets per hypothesis class (12 points)

Aggregate target set across the eight classes; PLAUSIBLE + PLAUSIBLE-MEDIUM classes get full coverage; PLAUSIBLE-LOW classes get one-target coverage.

| # | Class | Location | Filter | What it tells us |
|---|-------|----------|--------|------------------|
| T1.A | 1 (world-to-screen) | viewport/widget.rs render body (~1527, before set_brush_cursor_lines) | 30-frame throttle | Screen-pixel projection of cursor_center. If outside response_rect → Class 1 confirmed. |
| T1.B | 1 (world-to-screen) | viewport/widget.rs render body | 30-frame throttle | Camera position + target + view dir. Sanity check on camera state. |
| T1.D | 1 (world-to-screen) + 4 (screen radius) | viewport/widget.rs render body | 30-frame throttle | Screen-space radius (radius=50 projected to pixels). |
| T2.A | 2 (render target) | main.rs (rendering setup) | Log on render-path-change | Whether dock-based viewport.ui() vs main.rs:5614 standalone path is firing. |
| T2.B | 2 (render target) | viewport/renderer.rs at composite time | 30-frame throttle | brush_cursor_lines.len() at render-time (vs set time). |
| T3.A | 3 (z-depth) | viewport/widget.rs render body | 30-frame throttle | Per-line projected screen-space z; comparison vs expected terrain surface z. |
| T3.B+T3.C | 3 (z-depth) | viewport/renderer.rs render-pass setup | First 3 frames | Depth-test modes for debug-line vs terrain pipelines. |
| T4.A | 4 (color/alpha) | viewport/widget.rs render body | First call only | Full DebugLine color including alpha channel. |
| T4.B | 4 (line width) | viewport/renderer.rs render-pass setup | First 3 frames | Debug-line pipeline's line width setting. |
| T5.A+T5.B | 5 (coord system) | viewport/widget.rs render body + viewport/renderer.rs at terrain render | 30-frame throttle each | OrbitCamera matrices vs ViewportRenderer matrices. |
| T6.A+T6.B | 6 (terrain mesh) | terrain_panel.rs apply_brush_at + main.rs upload_terrain_chunks | On-event (stroke-end) | Dirty chunk IDs + upload calls. |
| T7.A | 7 (frame timing) | viewport/widget.rs all set_brush_cursor_lines call sites | Every call | All set_brush_cursor_lines invocations with caller identification + lines.len(). |

12 instrumentation points; coverage spans Classes 1-7 (Class 8 is implicit via comprehensive state-logging in T1+T2+T5).

### §4.3 Round 4 instrumentation prompt deferred

Round 4 prompt is separate session per Andrew's two-session-minimum directive. Round 4 prompt will:
- Reference this audit's §3 hypothesis classes + §4.2 instrumentation specifications as load-bearing input.
- Draft eprintln code per §4.2 targets with explicit filter patterns.
- Specify Andrew's runtime capture protocol.
- Specify revert protocol (Round 4 Instrumentation.B).
- Specify outcome decision tree per which hypothesis classes confirmed by captured output.

---

## §5 — Cumulative Evidence Narrative for Future Readers

### §5.1 The recovery narrative

Three rounds of evidence-grounded narrowing produced cumulative evidence about:
- What the brush-not-visible defect is NOT (data flow; gate-condition failure; render-body-silent-no-op).
- What hypothesis classes remain (rendering-pipeline failures: Classes 1-8 above).

This re-diagnostic synthesizes the three rounds into a single load-bearing artifact. **The Mediator Brush Fix at `8f4668599` was harmless redundancy** (relocated setter calls outside a non-blocking gate; functionally a no-op). Revert deferred to eventual real-fix session for clean recovery narrative.

The defect is in **rendering pipeline, not data flow**. The narrative produces a clean audit trail for future readers: original diagnostic → fix attempt → Round 1 → Round 2 → Round 3 re-diagnostic → Round 4 instrumentation → real fix → closure.

### §5.2 Why this matters for the campaign chain

Multi-round instrument-and-narrow with formal synthesis at round-3 is canonical for AstraWeave's layered abstractions (egui dock → tab_viewer → viewport widget → wgpu renderer → mediator). Future foundational architectural campaigns inherit the pattern.

The Mediator Removal session per Q6 will collapse some of these layers; this experience adds substantial empirical weight to the architectural motivation. Specifically: the audit-confidence calibration failures (§7.4 below) trace back to layered abstractions hiding runtime behavior from code-reading.

### §5.3 Why three rounds before formal synthesis

Two rounds of instrumentation supplements with §12 entries are appropriate when:
- Each round narrows a clear hypothesis.
- §12 entries can capture Round-N-Closure findings adequately.

Three rounds warrants formal synthesizing audit when:
- Hypothesis space pivots between rounds (data-flow → input-routing → render-body → rendering-pipeline).
- Original diagnostic's load-bearing claims are retroactively wrong.
- Future readers would face fragmented evidence across multiple §12 entries + an outdated original audit.

The threshold is empirical, not prescriptive. Future campaigns may have different thresholds depending on hypothesis space complexity.

---

## §6 — Forward Observations (out-of-scope but documented)

Per anti-drift discipline §1.4: concerns surfaced during synthesis that aren't this audit's target but warrant documentation.

### §6.1 Original audit's confidence calibration (three wrong code-reading hypotheses)

The original Sub-phase 3 Mediator Brush Diagnostic audit's confidence calibration didn't distinguish between code-reading-derived and runtime-verified claims. All three retroactively-wrong claims (§4.4 H4 REFUTAL; §4.5 H5 mechanism; §4.7 H7 classification) rested on code reading. Future failure-diagnosis audits should explicitly state which mechanism claims rest on code-reading versus runtime verification (§7.4 methodology lesson below).

### §6.2 Architectural fragility of multi-painter rendering pipeline

AstraWeave's editor uses multiple rendering layers (egui dock + wgpu render-to-texture + viewport image painted into egui + non-dock viewport at main.rs:5614 + debug-line pipeline + gizmo overlay). The rendering-pipeline defect class enumeration (§3) reflects this fragility — Classes 1, 2, 3, 5, 6, 7 all exploit potential gaps in the multi-painter coordination. Mediator Removal session per Q6 motivates simplifying this stack; this experience adds empirical weight.

### §6.3 Capture-timing artifacts in Round 1 misled hypothesis space

Round 1's `pointer=false` data led to Defect A framing that was wrong. The capture-timing mismatch (filter conditions firing outside Andrew's actual click+drag windows) wasn't anticipated when Round 1 instrumentation was specified. Future instrumentation sessions should:
- Verify capture-timing alignment with user actions.
- Log when filter conditions fire (meta-instrumentation) to confirm captures happen during expected user activity.
- Use less-restrictive filters when a defect's runtime triggers are uncertain.

### §6.4 Observability gap — existing Selection check log line

AstraWeave's editor has a `Selection check: clicked=false, pointer_over=true, gizmo_active=false` log line that captured useful data for Round 1's analysis. Similar built-in observability for the rendering pipeline (e.g., debug-line pipeline state at composite time) would have prevented Round 4 from needing to add equivalent instrumentation. Future logging additions could prevent similar diagnostic burden.

---

## §7 — Methodology Lessons

### §7.1 Three-rounds-wrong-from-code-reading pattern

**Pattern**: Original Sub-phase 3 Mediator Brush Diagnostic + tab_viewer:142 prime suspect (Round 1 §1.3 discovery) + audit §4.4 H4 REFUTAL — three confident-but-wrong hypotheses derived from code reading. AstraWeave's editor has multiple layered abstractions where code-reading misses runtime behavior.

**Implication**: Code-reading-derived hypotheses for failure-diagnosis in layered-abstraction stacks should be treated as preliminary; runtime verification is mandatory before high-confidence assertions.

**Documented for future foundational architectural campaigns**: when fix-from-diagnostic fails, do not re-derive new hypotheses from code reading; run instrumentation supplements. Specifically AstraWeave's editor (egui dock + wgpu + mediator) and likely other AAA-game-engine editor abstractions inherit the pattern.

### §7.2 Multi-round instrument-and-narrow as canonical pattern

**Pattern**:
- Original diagnostic produces hypothesis-classification audit (precedent: F.5-paint.E + G-pointer-events + this audit's predecessor).
- If fix from diagnostic fails → Round N=1 instrumentation supplement narrows hypothesis space.
- If Round N inconclusive after evidence capture → Round N+1 instrumentation supplement narrows further.
- After N=2-3 rounds without converging fix: formal re-diagnostic audit synthesizes prior rounds + enumerates remaining hypothesis classes.

**Implication**: Two rounds is the soft threshold for formal synthesis; three rounds is the canonical threshold. Earlier formal synthesis (after Round 1) is wasteful; later synthesis (after Round 4+) is too late.

**Documented for future foundational architectural campaigns**: this pattern is now empirically validated. Round 3 audits inherit hypothesis-class enumeration + Round-N+1 instrumentation specifications as standard structure.

### §7.3 Pre-execution actual-code verification as discipline pattern

**Pattern**: Round 1 prompt §1.3 pre-execution verification surfaced the EditorApp::new() default-scene auto-populate finding that refuted H5 mechanism BEFORE Round 1 instrumentation locked in. Without §1.3 verification, Round 1 would have run with wrong assumption baked in.

**Implication**: Predecessor audit findings should be runtime-verified during pre-execution, not assumed-as-true.

**Documented for future failure-diagnosis sessions**: pre-execution §1.X actual-code verification is mandatory; if predecessor's mechanism claims contradict actual code, surface as deviation BEFORE main session work begins.

### §7.4 Audit-confidence-calibration honesty as methodology pattern

**Pattern**: Original diagnostic confidently asserted H5 mechanism CONFIRMED based on code reading. Three wrong code-reading-derived claims in a single audit; no markers distinguishing code-reading from runtime-verification.

**Implication**: Audits should explicitly state which mechanism claims rest on code-reading versus runtime verification. Confidence calibration honesty enables future readers to evaluate audit reliability.

**Documented for future failure-diagnosis sessions**: hypothesis-classification audits should annotate each CONFIRMED / REFUTED rating with evidence type (code-reading vs runtime-verified vs mixed). A future audit pattern:

```
H5 — Mediator drain logic regression
Status: CONFIRMED (mechanism: code-reading; severity: code-reading;
         no runtime verification)
[…]
```

vs:

```
H5 — Mediator drain logic regression
Status: CONFIRMED (mechanism: runtime-verified at capture session XYZ;
         severity: runtime-verified)
[…]
```

The honesty enables Round-1-style §1.X verification to know what claims need verification.

### §7.5 Synthesizing-artifact threshold (round-3-or-later)

**Pattern**: Round 1 + Round 2 §12 entries adequately captured per-round evidence. Round 3 needed formal synthesizing audit because:
- Hypothesis space pivoted twice (data-flow → input-routing → render-body → rendering-pipeline).
- Original diagnostic's load-bearing claims retroactively wrong.
- Future readers facing fragmented evidence across §12 entries + outdated original audit would struggle.

**Implication**: Synthesizing-artifact threshold is empirical; depends on hypothesis space complexity + audit reliability degradation. Two rounds with stable hypothesis space + reliable original audit may not need formal synthesis.

**Documented for future foundational architectural campaigns**: monitor hypothesis space pivots + original audit reliability; trigger formal re-diagnostic synthesis when both degrade.

---

*End of Sub-phase 3 Mediator Brush Round 3 Re-Diagnostic audit.*
