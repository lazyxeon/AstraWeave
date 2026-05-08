# Editor Multi-Tool Architecture Campaign — Phase 1.X

**Status**: Campaign-design pass COMPLETE 2026-05-04, commits `75b68e7c7` (Design.A campaign doc) + `8fad61bd3` (Design.B Regional Archetype Variation cross-reference) + `8c92890b9` (Design.C hash-fixup). **Sub-phase 1 — Diagnostic COMPLETE 2026-05-04**, commits `4556c267b` (Diagnostic.A audit) + `0a7df3cdf` (Diagnostic.B campaign doc update) + `6924e39db` (Diagnostic.C hash-fixup); audit at `docs/audits/editor_multi_tool_architecture_diagnostic_2026-05-04.md`; all ten §2.X commitments compatibility-confirmed; 2 open-questions deferred. **Sub-phase 2 — ActiveTool trait + dispatcher core + register_tool API COMPLETE 2026-05-04**, commits `813ac29a1` (Core.A trait + types + ToolContext) + `2c791fa39` (Core.B Dispatcher) + `ece7bb3b4` (Core.C 15 unit tests + MockActiveTool fixture) + `6016b3c8f` (Core.D campaign doc update); new module at `tools/aw_editor/src/active_tool/`; resolves §2.7 ToolContext open question via pre-computed world-XZ projection fields + method accessors; module isolated (no external usages in ViewportWidget/main.rs/panels/tab_viewer); 15 unit tests pass; code-level only (NOT Andrew-gated per Q9). **Sub-phase 3 — TerrainPanel ActiveTool implementation (additive) IN PROGRESS**: 3.A `0dea0bebc` (impl ActiveTool for TerrainPanel + TerrainAction::SetActiveTool variant + UI emission + TERRAIN_PANEL_UUID constant) + 3.B `41ec3b192` (tab_viewer SetActiveTool capture + EditorApp.dispatcher field + ViewportWidget cached-then-dispatch integration) landed; 3.C closeout DEFERRED pending mediator brush fix. **Sub-phase 3 Mediator Brush Diagnostic COMPLETE 2026-05-05**, commits `e5e32f486` (Diagnostic.A audit) + `f5c96836f` (this commit, campaign doc update); audit at `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_2026-05-05.md`; H5 (Mediator drain logic regression) CONFIRMED at commit `f84eb09049` (2026-03-17, "imported kaykit complete asset package") — `viewport.set_terrain_brush_active` + `viewport.set_terrain_brush_params` placed inside `if let (Some(world), Some(viewport))` gate at main.rs:3867, brush state never reaches viewport when no scene loaded; H1-H4 + H6-H8 REFUTED or non-causal; recommended fix small (~10 lines, single-commit, no architectural changes); defect predates Sub-phase 3 by ~7 weeks per Andrew Q1 verification. **Sub-phase 3 Mediator Brush Fix landed but REGRESS-verdict 2026-05-05** at commit `8f4668599` (`Mediator-Brush-Fix.A` — single-file relocation of two setter calls outside `Some(world)` gate; +11/-2 lines). Path 1 brush UX unchanged post-fix; FPS spike on paint mode disappeared (signal that fix reached intended code path). **Sub-phase 3 Mediator Brush Diagnostic Instrumentation Round 1 COMPLETE 2026-05-06**, commits `e037d63b5` (Round 1 Instrumentation.A — 5 eprintlns) + `040d90b16` (Round 1 Instrumentation.B — revert) + `df4a50e74` (this commit). Round 1 captured runtime evidence contradicted three hypotheses: H5 (Mediator Brush Diagnostic root cause) WRONG (scene_state always Some); tab_viewer:142 catch-all gate (Round 1 prime suspect) WRONG (gate fires every frame); audit §4.4 H4 REFUTAL ("ViewportWidget input-flow regression") WRONG (egui response is broken — `pointer=false` despite confirmed click+drag). Two distinct defects identified: **Defect A** (input routing — `dragged_by(Primary)` + `clicked_by(Primary)` return false) + **Defect B** (render body silent no-op — render-gate full=true every frame yet no visible cursor/ring). Mediator Brush Fix `8f4668599` retroactively classified as harmless redundancy; revert deferred to eventual real-fix session for clean recovery narrative. **Sub-phase 3 Mediator Brush Diagnostic Instrumentation Round 2 COMPLETE 2026-05-06**, commits `0b6bc6f50` (Round 2 Instrumentation.A — 6 eprintlns) + `4a2aca24b` (Round 2 Instrumentation.B — revert). Round 2 captured output refuted Defect A (egui captures input correctly: `dragged_primary=true, sense_click=true, sense_drag=true` during click+drag) AND partially refuted Defect B (render body executes with valid-looking parameters: `cursor_center=(809.78,888.48,809.96), radius=50.0, line_count=48, depth_pick_succeeded=true, color=[0.2, 1.0, 0.3]`). Brush IS fully functional at data layer (`drag_dist=172.3`, `1 chunks modified`, `Recorded: Terrain Sculpt`) yet Andrew sees nothing visually. Defect class identified: **rendering-pipeline failure between draw call and visible pixel** — narrower hypothesis space than original framing. **Sub-phase 3 Mediator Brush Round 3 Re-Diagnostic COMPLETE 2026-05-06**, commits `7dc342a42` (Round-3-Re-Diagnostic.A audit) + `19eccad3c` (this commit, campaign doc Round-2-Closure + Round-3-Re-Diagnostic entries); audit at `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_round_3_re_diagnostic_2026-05-06.md`; honors Andrew's earlier Option B directive ("option a then b if a doesn't work"); 8 hypothesis classes enumerated for rendering-pipeline defect (Classes 1-2 PLAUSIBLE; Classes 3+5+6 PLAUSIBLE-MEDIUM; Classes 4+7 PLAUSIBLE-LOW; Class 8 ACKNOWLEDGED); 12 Round 4 instrumentation targets specified across hypothesis classes; 5 methodology lessons codified (three-rounds-wrong-from-code-reading; multi-round instrument-and-narrow as canonical pattern; pre-execution actual-code verification; audit-confidence-calibration honesty; synthesizing-artifact threshold round-3-or-later). Mediator Brush Fix `8f4668599` retroactively classified as harmless redundancy (revert deferred to eventual real-fix session for clean recovery narrative). **Sub-phase 3 Mediator Brush Diagnostic Round 4 Instrumentation COMPLETE 2026-05-06**, commits `de1986301` (Round 4 Instrumentation.A — 12 targeted eprintlns across 8 hypothesis classes per Round 3 audit §4.2) + `bc9c8d5bd` (Round 4 Instrumentation.B — revert). Round 4 captured `brush-dbg-r4.txt` (6,641 BRUSH-DBG lines) proving brush IS fully functional at data layer (T6.A `modified=true`, T6.B `dirty_count=1, dirty_indices=[242], viewport_is_some=true`, chunk 242 modified + upload event fires per stroke). Ring-not-visible + terrain-modification-not-visible are downstream symptoms of cursor_center being computed at cam_pos rather than at terrain surface. **Defect class narrows to depth pick chain**: `read_depth_at_pixel` (camera.rs:~1145) + `unproject_depth_to_world` (camera.rs:~541) + depth target binding lifecycle. Round 4 evidence — cursor_center stays invariantly within ~0.5 world units of cam_pos across hundreds of distinct screen positions; ndc_z=0.0000, clip_w=0.5000 directly indicates near-plane unprojection. **Sub-phase 3 Mediator Brush Round-4-Closure.A COMPLETE 2026-05-06**, commit `3bf425053` (this commit, campaign doc Status header + §11 entries + §12 Round-4-Closure entry); supersedes `bc9c8d5bd` commit-message body which codified wrong conclusion ("Class 3 confirmed at clip-plane level, root-caused by Andrew zooming in"); preserved as methodology-lesson exhibit on derived-value reasoning trap (`cam_dist=0.5` is derived from cam_pos and cursor_center at log-time, not a primary fact about camera position; first-frame fallback evidence shows cam_dist-to-target=1322 proving normal orbit distance). New methodology lesson surfaces (sibling to Round 3 audit §7.1-§7.5): when two values are unexpectedly equal, check whether one is being computed from the other before reasoning about the equality as causal. **Sub-phase 3 Mediator Brush Diagnostic Round 5 Instrumentation COMPLETE 2026-05-07**, commits `5203e7fbe` (Round 5 Instrumentation.A — 5 targeted eprintlns across depth pick chain per Round 4 closure §12 mechanism enumeration: T8.A read_depth_at_pixel raw return + T8.B depth target write/read identity + T8.C terrain pipeline depth-write state at adapter dispatch + T8.D pixel coords + viewport_rect alignment + T8.E unproject_depth_to_world math sanity) + `2eb0903a5` (Round 5 Instrumentation.B — clean revert; production code identical to post-bc9c8d5bd state). Captured `brush-dbg-r5.txt` (1,955 BRUSH-DBG lines filtered from `brush_dbg-r5.log`). **Outcome 1 per Round 5 instrumentation prompt §3.6**: single mechanism confirmed cleanly. **Sub-phase 3 Mediator Brush Round-5-Closure.A COMPLETE 2026-05-07**, commit `569415a7a` (this commit, campaign doc Status header + §11 entries + §12 Round-5-Closure entry). **Mechanism 1 (wrong texture, "different render target" sub-class) DEFINITIVELY CONFIRMED** per Round 4 closure §12 enumeration: engine adapter renders terrain to its own internal depth target via `render_to_texture(scene_target_view, encoder)` (no depth attachment passed); aw_editor's `read_depth_at_pixel` samples `self.depth_texture` ("Viewport Depth Texture", Depth32Float) which is written ONLY by aw_editor-local overlay passes (grid, gizmos, debug shapes). Reads return cleared/uninitialized 0.0 → near-plane unprojection → cursor_center ≈ cam_pos invariantly. **5 rounds of evidence-grounded narrowing converged on a single mechanism in one Round 5 capture session.** **Andrew-gate fix design decision REQUIRED**. Three candidate designs documented in §12 Round-5-Closure entry: (a) pass `self.depth_view` to `engine_adapter.render_to_texture` as depth attachment (cross-crate API change to `astraweave_render`; cleanest semantically; resolves latent overlay-vs-terrain depth-test bug as bonus; aligns with CLAUDE.md v0.10.1 Edit 2 no-second-implementation); (b) expose engine adapter's internal depth target via accessor (cross-crate accessor; works around API shape; leaves overlay-vs-terrain depth-test bug latent); (c) add aw_editor-local depth pre-pass re-rendering terrain depth into `self.depth_view` (aw_editor-local but requires cross-crate mesh data access; redundant rendering work; precision-mismatch risk). Small-fix prompt drafted post-Andrew-decision per two-session-minimum directive. New methodology lesson candidate surfaces (sibling to Round 4 closure §7.6 candidate derived-value reasoning trap): **§7.7 candidate — wrapped-component resource identity trap**. When component A wraps component B and both manage similar resources of the same logical type, the higher-level component's reads from its own resource don't reflect the lower-level component's writes to its own resource — both believe they're working with "the viewport depth buffer"; bug is at the boundary. Direct architectural sibling to CLAUDE.md v0.10.1 Edit 2 (no-second-implementation) extended to resource-management level. **Sub-phase 3 Mediator Brush Real-Fix.A LANDED 2026-05-07** at commit `0f569d212` per Andrew-gate Option (a) decision (cross-crate API addition: `Option<&wgpu::TextureView>` depth_view parameter to `Renderer::draw_into` + `engine_adapter::render_to_texture`; +33/-9 lines across 5 files). Andrew-gate verification 2026-05-07 partial PASS (ring renders at cursor; cursor follows click → Mechanism 1 / depth pick chain CONFIRMED FIXED) + partial REGRESS (brush effects don't visibly modify terrain; all 8 brush modes affected uniformly → NEW defect class downstream of Mechanism 1, NOT a Real-Fix.A regression). **Sub-phase 3 Mediator Brush Diagnostic Round 6 Instrumentation COMPLETE 2026-05-07**, commits `d7b9e7bc2` (Round 6 Instrumentation.A — 6 eprintlns across mesh-update chain: T9.A apply_brush_at world coords + T9.B emit/handle event + T9.C update_terrain_chunk 3-sub-path + T9.D terrain render source + T9.E chunk height delta + T9.F adapter instance identity) + `18f48f08b` (Round 6 Instrumentation.B clean revert; production code identical to post-`0f569d212`). Captured `brush-dbg-r6.log` (5,459 BRUSH-DBG lines). **Outcome 3 per Round 6 prompt §3.7**: Mechanism C (mesh resource identity trap, sibling §7.7 instance at mesh-data layer) DEFINITIVELY CONFIRMED via T9.C smoking gun #3 (1749/1749 success entries with `affected_clusters=0`) + T9.D paired confirmation (`terrain_cluster_models=0` across 238 samples) + T9.E heights changing measurably (delta ≈ 4.96, close to 5.0 sculpt strength) + T9.A/B/F refuted other mechanisms. **Sub-phase 3 Mediator Brush Round-6-Closure.A COMPLETE 2026-05-07**, commit `c7f3b50b3` (this commit, campaign doc Status header + §11 entries + §12 Round-6-Closure entry). **Architectural defect**: initial terrain upload routes through live path (`engine_adapter::upload_terrain_chunks` → `Renderer::upload_terrain_chunk` → `Renderer::terrain_forward.chunks` HashMap, drawn at `renderer.rs:5755`); incremental brush update routes through legacy dead path (`engine_adapter::update_terrain_chunk` → `rebuild_terrain_clusters_for_chunk` → `terrain_clusters` Vec / `self.models["terrain_cluster_*"]` — never read by render). Heights mutate correctly + 794 events fire correctly + 1749 chunk uploads succeed at slot_map level + legacy cluster rebuild is no-op every time → mesh stays stale forever despite all upstream chain components working. **§7.7 GRADUATES from CANDIDATE to CONFIRMED CANONICAL PATTERN with multi-layer evidence in same campaign**: Round 5 confirmed at depth-target layer (Mechanism 1 — Real-Fix.A fixed via Option (a) depth attachment plumbing); Round 6 confirms at mesh-data layer (Mechanism C — dual-variant: initial vs incremental routing to different resources). Same architectural anti-pattern; different layers; different sub-variants. Validates multi-round instrument-and-narrow methodology canonically (Round 3 audit §7.2 candidate strengthened to canonical by Round 6 evidence). **Andrew-gate fix design decision REQUIRED.** Two candidate designs documented in §12 Round-6-Closure entry: **Option 1 (refactor with shared helper)** — extract `upload_or_update_terrain_chunk_forward(chunk_index, vertices, indices)` called by both `upload_terrain_chunks` (initial) and `update_terrain_chunk` (incremental); aligns with CLAUDE.md v0.10.1 Edit 2 (no-second-implementation); medium fix size. **Option 2 (inline copy)** — copy splat-build + filter logic from `upload_terrain_chunks` into `update_terrain_chunk`; smaller cross-function change but creates the exact second-implementation anti-pattern Edit 2 forbids; ironic given Round 5 + Round 6 evidence just confirmed Edit 2 canonically. Real-Fix.B prompt drafted post-Andrew-decision per two-session-minimum directive. **Suggested CLAUDE.md amendment cycle**: extend Edit 2 with resource-identity corollary; closure surfaces but does NOT pre-apply. **Dead-code observation** (separate cleanup concern): Round 6 evidence proves legacy cluster path (`rebuild_terrain_clusters_for_chunk` + `terrain_clusters` Vec + `self.models["terrain_cluster_*"]` writes in `upload_terrain_chunks`) is dead code at this editor configuration — writes occur but nobody reads. Cleanup-A session deferred to post-Real-Fix.C-PASS per CLAUDE.md v0.10.1 Edit 3 spirit. Mediator Brush Fix `8f4668599` revert deferred to Cleanup-B session post-Real-Fix.C-PASS per audit §5.1. **Sub-phase 3 Mediator Brush Real-Fix.B LANDED 2026-05-07** at commit `eaaa53433` per Andrew-gate Option 1 decision (extract shared `upload_or_update_terrain_chunk_forward` helper called by both initial `upload_terrain_chunks` and incremental `update_terrain_chunk` paths; aligns with CLAUDE.md v0.10.1 Edit 2). Andrew-gate verification 2026-05-07 partial PASS (6/8 brush modes — sculpt, flatten, lower, smooth, erode, noise — visibly modify terrain → Mechanism C / mesh-data layer §7.7 instance CONFIRMED FIXED) + partial REGRESS (paint and zoneblend produce no visible texture change → NEW defect class downstream of Mechanism C, NOT a Real-Fix.B regression; tracks the height-vs-texture data axis cleanly). **Sub-phase 3 Mediator Brush Diagnostic Round 7 Instrumentation COMPLETE 2026-05-08**, commit `c4d09ee86` (Round 7 Instrumentation.A — 5 targeted eprintlns across paint/zoneblend texture-mutation chain: T10.A paint/zoneblend brush-apply data flow + T10.B helper-entry splat data state with biome_weights + material_* hash content tracking + T10.C renderer upload-pre splat content tracking + T10.D renderer draw-frame chunks survival + T10.E paint/zoneblend emit gating; throttled 5 Hz / once-per-12-frames except T10.D once-per-60-frames). Round-7-Instrumentation.B revert NOT YET LANDED (separate following session per discipline shift; closure session lands first). Captured `brush-dbg-r7.txt` (220,423 lines / 6,954 BRUSH-DBG entries). **Outcome 1 per Round 7 prompt §3.6**: Mechanism H1 confirmed cleanly via smoking-gun chunk_index=40 paint sequence (lines 202462-203435: 10 helper invocations with hash_biome=2d3d6543c332d0f4 invariant + hash_material varying + splat_0_hash=71cd3d8b30b4adce invariant). **Sub-phase 3 Mediator Brush Round-7-Closure.A COMPLETE 2026-05-08**, commit `[hash]` (this commit, campaign doc Status header + §11 entries + §12 Round-7-Closure entry). **Mechanism H1 (wrapped-component sibling-attribute drift; intra-component variant of §7.7 trap-pattern at texture-data layer) DEFINITIVELY CONFIRMED**: paint mutates `vertex.material_ids` and `vertex.material_weights`; `build_chunk_splat_maps` reads only `vertex.biome_weights_0/1`; the two attribute sets never converge, so paint's work product never reaches the rendered splat texture. **ZoneBlend independent mechanism** (orthogonal to §7.7): `BrushMode::ZoneBlend => current_h` no-op stub at `terrain_integration.rs:1915`. Missing-implementation defect; ZoneBlend has placeholder logic that returns current height unchanged. Different mechanism class entirely from paint's resource-identity trap; treated as separate fix concern. **§7.7 ELEVATES from CANONICAL PATTERN to FUNDAMENTAL ARCHITECTURAL ANTI-PATTERN with three-layer evidence + intra-component sub-variant**: Round 5 (depth-target inter-component; Real-Fix.A `0f569d212` fixed) + Round 6 (mesh-data inter-component dual-variant; Real-Fix.B `eaaa53433` fixed) + Round 7 (texture-data intra-component sibling-attribute drift; fix design pending Andrew-gate). Three-layer evidence + two granularity scales (component-boundary + struct-attribute-set) makes §7.7 axiomatic methodology. **Andrew-gate fix design decisions REQUIRED** (two): (a) paint fix design — three candidates documented in §12 Round-7-Closure entry: Option A (mirror writes; smallest, doesn't unify identity), Option B (switch reader; eliminates trap, biome generation may need updating), Option C (unify into single canonical attribute set; largest blast radius, most architecturally clean, aligns with elevated Edit 2); (b) ZoneBlend bundled-vs-split scoping — bundle with Real-Fix.C (8/8 culminates) OR split into Real-Fix.D (post-Real-Fix.C; ZoneBlend gets dedicated semantic discussion). **Suggested CLAUDE.md amendment cycle elevation** (closure surfaces; does not pre-apply): three-layer + multi-granularity-scale evidence may elevate the resource-identity rule from "candidate corollary appended to Edit 2" to "first-class case in Edit 2 itself, structured at multiple granularity scales." **Discipline shift documented**: Round 7 separates instrument / closure / revert / fix into single-concern sessions (vs Rounds 4-6's bundled .A+.B sessions); single-concern session pattern is canonical going forward. Round-7-Instrumentation.B revert + Real-Fix.C drafting + Real-Fix.C landing all deferred to separate following sessions. Sub-phase 4-6 + Dedicated Mediator Removal session + Real-Fix.C + Real-Fix.D (if (b) split) + Cleanup-A + Cleanup-B + Sub-phase 3.C closeout NOT STARTED. Foundational dispatcher architecture campaign launched as spinoff from Regional Archetype Variation pause artifacts (commits `a64f12320` + `98fc063d9` + `13ef70132`); Andrew architectural decision 2026-05-03 + strategic-factors enumeration Q1-Q10 ground §2 architectural decisions. Research pass at `docs/audits/editor_multi_tool_architecture_research_2026-05-03.md` (commits `8ba6cd13e` + `29b8c53b3` + `c3bc7ca0c`) is load-bearing input to §2; G-research + G-diagnostic audits inherited as predecessor research per research audit §2.

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
Sub-phase 3 Mediator Brush Fix — relocate brush sync calls outside Some(world) gate: LANDED 2026-05-05 commit 8f4668599 BUT REGRESS at Andrew-gate (Path 1 brush UX unchanged; FPS spike disappearance only). Retroactively classified as harmless redundancy per Round 1 evidence. Revert deferred to eventual real-fix session for clean recovery narrative.
Sub-phase 3 Mediator Brush Diagnostic Instrumentation Round 1 — 5-point eprintln supplement for runtime evidence capture: COMPLETE 2026-05-06, commits e037d63b5 (Round 1 Instrumentation.A) + 040d90b16 (Round 1 Instrumentation.B revert) + df4a50e74 (Round-1-Closure.A this commit). Defects A (input routing) + B (render body silent no-op) identified. Three hypotheses contradicted (H5; tab_viewer:142; audit §4.4 H4 REFUTAL).
Sub-phase 3 Mediator Brush Diagnostic Instrumentation Round 2 — 6-point eprintln supplement to disambiguate Defects A + B: COMPLETE 2026-05-06, commits 0b6bc6f50 (Round 2 Instrumentation.A) + 4a2aca24b (Round 2 Instrumentation.B revert). Defect A framing REFUTED (egui captures input correctly); Defect B framing PARTIALLY REFUTED (render body executes with valid-looking parameters); rendering-pipeline defect class identified.
Sub-phase 3 Mediator Brush Round 3 Re-Diagnostic — formal synthesizing audit per Andrew's Option B directive: COMPLETE 2026-05-06, commits 7dc342a42 (Round-3-Re-Diagnostic.A audit ~700 lines) + 19eccad3c (Round-3-Re-Diagnostic.B campaign doc update this commit). 8 hypothesis classes enumerated; 12 Round 4 instrumentation targets specified; 5 methodology lessons codified.
Sub-phase 3 Mediator Brush Round 4 Instrumentation — 12-point eprintln supplement targeting hypothesis classes 1-7: COMPLETE 2026-05-06, commits de1986301 (Round 4 Instrumentation.A — 10 functional targets, ~14 actual eprintln points after code-drift adjustments per Round-4-Instrumentation.A commit body; T3.B+C and T4.B skipped because pipeline state is in astraweave_physics crate; T5.B subsumed by T5.A since renderer uses same OrbitCamera reference) + bc9c8d5bd (Round 4 Instrumentation.B revert). Captured brush-dbg-r4.txt (6,641 BRUSH-DBG lines). Hypothesis classes 1, 2, 4, 5, 6, 7 all REFUTED. Class 3 (z-depth) DOWNSTREAM SYMPTOM not root cause (line0_ndc_z=1.0167 is consequence of cursor_center=cam_pos; ring vertices necessarily fall outside view frustum when their center IS the camera position). Outcome 5 per Round 4 prompt §5: unexpected mechanism — defect class is upstream of all eight Round 3 audit §3 enumerated mechanisms.
Sub-phase 3 Mediator Brush Round-4-Closure.A — campaign doc codification of corrected analysis: COMPLETE 2026-05-06, commit 3bf425053 (this commit). Defect class narrows to depth pick chain (read_depth_at_pixel + unproject_depth_to_world + depth target binding lifecycle). Brush IS fully functional at data layer; ring-not-visible + terrain-modification-not-visible are downstream symptoms of cursor_center invariantly equal to cam_pos regardless of screen click position. Supersedes bc9c8d5bd commit-message body (codifies wrong conclusion preserved as methodology-lesson exhibit on derived-value reasoning trap). New methodology lesson surfaces (sibling to Round 3 audit §7.1-§7.5).
Sub-phase 3 Mediator Brush Round 5 Instrumentation — 5-point eprintln supplement targeting depth pick chain per Round 4 closure §12 mechanism enumeration: COMPLETE 2026-05-07, commits 5203e7fbe (Round 5 Instrumentation.A — T8.A read_depth_at_pixel raw return + T8.B depth target write/read identity + T8.C terrain pipeline depth-write state at adapter dispatch + T8.D pixel coords + viewport_rect alignment + T8.E unproject_depth_to_world math sanity; throttle 5 Hz for high-frequency calls, once-per-frame for T8.B, once-at-init for T8.C) + 2eb0903a5 (Round 5 Instrumentation.B clean revert; production code identical to post-bc9c8d5bd). Tighter scope than Round 4's 12 targets per narrowed hypothesis space (1 defect class, 6 mechanisms vs Round 3's 8 classes). Captured brush-dbg-r5.txt (1,955 BRUSH-DBG lines).
Sub-phase 3 Mediator Brush Round-5-Closure.A — campaign doc codification of Round 5 outcome with three fix candidate designs for Andrew-gate: COMPLETE 2026-05-07, commit 569415a7a (this commit). Outcome 1 per Round 5 prompt §3.6: Mechanism 1 (wrong texture, "different render target" sub-class) DEFINITIVELY CONFIRMED. T8.A=0.0 invariant + T8.B handle mismatch every frame + T8.C confirms adapter API has no depth attachment param + T8.D inside_viewport=true 650/650 + T8.E math consistent (depth=0 → ndc_z=0 → near-plane unprojection). 5 rounds of evidence-grounded narrowing converged on single mechanism in one capture session. New methodology lesson candidate: §7.7 wrapped-component resource identity trap (sibling to §7.6 derived-value reasoning trap).
Sub-phase 3 Mediator Brush Real-Fix.A — Option (a) per Andrew-gate decision: pass Option<&wgpu::TextureView> depth_view parameter through engine_adapter::render_to_texture + Renderer::draw_into: LANDED 2026-05-07, commit 0f569d212. Cross-crate API addition; +33/-9 lines across 5 files (astraweave-render renderer.rs + tonemap_probe + coverage_booster_render; aw_editor viewport/engine_adapter.rs + viewport/renderer.rs). Andrew-gate verification 2026-05-07 partial PASS (ring renders at cursor; cursor follows click → Mechanism 1 fixed) + partial REGRESS (brush effects don't visibly modify terrain → NEW defect class downstream surfaced).
Sub-phase 3 Mediator Brush Round 6 Instrumentation — 6-point eprintln supplement targeting mesh-update chain per Round 5 closure §7.7 candidate framing extended to mesh-data layer: COMPLETE 2026-05-07, commits d7b9e7bc2 (Round 6 Instrumentation.A — T9.A apply_brush_at world coords + T9.E chunk height delta paired + T9.B emit/handle 2-paired sites + T9.C update_terrain_chunk 3-sub-path always-on + T9.D terrain render source 60-frame throttle + T9.F adapter instance identity once-only) + 18f48f08b (Round 6 Instrumentation.B clean revert; +118/-118 lines across 5 files). Captured brush-dbg-r6.log (5,459 BRUSH-DBG lines).
Sub-phase 3 Mediator Brush Round-6-Closure.A — campaign doc codification of Round 6 outcome with two fix candidate designs for Andrew-gate + §7.7 graduation to canonical: COMPLETE 2026-05-07, commit c7f3b50b3 (this commit). Outcome 3 per Round 6 prompt §3.7: Mechanism C (mesh resource identity trap, sibling §7.7 instance at mesh-data layer) DEFINITIVELY CONFIRMED with dual-variant sub-mechanism. T9.C smoking gun #3 (1749/1749 success entries with affected_clusters=0) + T9.D paired confirmation (terrain_cluster_models=0 across 238 samples) + T9.E heights changing correctly (delta ≈ 4.96) + T9.A/B/F refuted other mechanisms. 6 rounds of evidence-grounded narrowing converged on second of two §7.7 instances in same campaign. **§7.7 graduates from candidate to confirmed canonical pattern with multi-layer evidence** (depth-target Round 5 + mesh-data Round 6). Suggested CLAUDE.md amendment cycle: extend Edit 2 with resource-identity corollary.
Sub-phase 3 Mediator Brush Real-Fix.B — Option 1 (refactor with shared helper) per Andrew-gate decision: LANDED 2026-05-07 commit eaaa53433. Extract shared upload_or_update_terrain_chunk_forward helper called by both initial upload_terrain_chunks and incremental update_terrain_chunk paths; aligns with CLAUDE.md v0.10.1 Edit 2 (no-second-implementation). Andrew-gate verification 2026-05-07 partial PASS (6/8 brush modes — sculpt, flatten, lower, smooth, erode, noise — visibly modify terrain → Mechanism C / mesh-data layer §7.7 instance CONFIRMED FIXED) + partial REGRESS (paint and zoneblend produce no visible texture change → NEW defect class downstream surfaced; tracks height-vs-texture data axis cleanly).
Sub-phase 3 Mediator Brush Diagnostic Round 7 Instrumentation — 5-point eprintln supplement targeting paint/zoneblend texture-mutation chain per Round 6 closure §7.7 canonical-pattern framing extended to texture-data layer: COMPLETE 2026-05-08, commit c4d09ee86 (Round 7 Instrumentation.A — T10.A paint/zoneblend brush-apply data flow at terrain_panel.rs apply_brush_at + T10.B helper-entry splat data state at viewport/engine_adapter.rs upload_or_update_terrain_chunk_forward with biome_weights + material_* hash content tracking + T10.C renderer upload-pre at astraweave-render renderer.rs Renderer::upload_terrain_chunk + T10.D renderer draw-frame chunks survival at astraweave-render renderer.rs draw_into terrain pass + T10.E paint/zoneblend emit gating at terrain_panel.rs; throttled 5 Hz / once-per-12-frames except T10.D once-per-60-frames; +147 lines across 3 files; cargo check clean; 15/15 active_tool tests pass). Round-7-Instrumentation.B revert NOT YET LANDED (separate following session per discipline shift — Round 7 separates instrument / closure / revert / fix into single-concern sessions). Captured brush-dbg-r7.txt (220,423 lines / 6,954 BRUSH-DBG entries).
Sub-phase 3 Mediator Brush Round-7-Closure.A — campaign doc codification of Round 7 outcome with three fix candidate designs for paint + ZoneBlend bundled-vs-split scoping for Andrew-gate + §7.7 elevation to fundamental architectural anti-pattern: COMPLETE 2026-05-08, commit [hash] (this commit). Outcome 1 per Round 7 prompt §3.6: Mechanism H1 (wrapped-component sibling-attribute drift; intra-component variant of §7.7 at texture-data layer) DEFINITIVELY CONFIRMED via smoking-gun chunk_index=40 paint sequence (lines 202462-203435 of brush-dbg-r7.txt: 10 helper invocations with hash_biome=2d3d6543c332d0f4 invariant + hash_material varying + splat_0_hash=71cd3d8b30b4adce invariant). Paint mutates vertex.material_ids and vertex.material_weights; build_chunk_splat_maps reads only vertex.biome_weights_0/1; the two attribute sets never converge. ZoneBlend independent no-op-stub mechanism (orthogonal to §7.7) noted at terrain_integration.rs:1915 (BrushMode::ZoneBlend => current_h placeholder). 7 rounds of evidence-grounded narrowing converged on the third of three §7.7 instances in same campaign. **§7.7 ELEVATES from canonical pattern to fundamental architectural anti-pattern** with three-layer evidence (depth-target Round 5 + mesh-data Round 6 + texture-data Round 7) + intra-component sub-variant + two granularity scales (component-boundary + struct-attribute-set). Three fix candidate designs for paint (Option A mirror writes; Option B switch reader; Option C unify) + ZoneBlend bundled-vs-split scoping (Real-Fix.C or Real-Fix.D) presented for Andrew-gate decisions. Suggested CLAUDE.md amendment cycle elevation: resource-identity rule may elevate from corollary to first-class case in Edit 2.
Sub-phase 3 Mediator Brush Real-Fix.C — Option A / B / C per Andrew-gate decision (a) for paint + bundled or split per (b) for ZoneBlend: NOT STARTED (Andrew-gate decisions required; Real-Fix.C prompt drafted post-decisions per two-session-minimum directive; Round-7-Instrumentation.B revert lands first per discipline shift single-concern session pattern).
Sub-phase 3 Mediator Brush Real-Fix.D — ZoneBlend implementation if Andrew-gate (b) selects split scoping: NOT STARTED (conditional on (b) split; Real-Fix.C lands first if bundled, Real-Fix.D session merged into Real-Fix.C in that case).
Sub-phase 3 Mediator Brush Cleanup-A — delete dead-code legacy cluster path (rebuild_terrain_clusters_for_chunk + terrain_clusters Vec + self.models["terrain_cluster_*"] writes): NOT STARTED (gated on Real-Fix.C PASS — actual Sub-phase 3 completion criterion is 8/8 brush modes working; currently 6/8).
Sub-phase 3 Mediator Brush Cleanup-B — Mediator Brush Fix 8f4668599 revert per audit §5.1: NOT STARTED (gated on Real-Fix.C PASS for clean recovery narrative).
Sub-phase 3.C — closeout: NOT STARTED (gated on Real-Fix.C PASS + Cleanup-A + Cleanup-B).
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

### 2026-05-05, Sub-phase 3 Mediator Brush Fix.A, commit 8f4668599

**Sub-phase 3 sub-task — code fix per Sub-phase 3 Mediator Brush Diagnostic audit findings. Single-file +11/-2 line relocation of `viewport.set_terrain_brush_active` + `viewport.set_terrain_brush_params` setter calls outside the `if let (Some(world), Some(viewport))` gate at main.rs:3867 into a preceding `if let Some(viewport) = self.viewport.as_mut()` block.**

**Andrew-gate verdict 2026-05-05**: REGRESS in Path 1 brush UX. All three observable symptoms (no cursor change, no ring overlay, no terrain modification on click+drag) unchanged from pre-fix. **However**: FPS spike on paint mode switch disappeared post-fix — behavioral change indicating fix DID reach intended code path. This rules out fix-didn't-land-semantically-correctly; rules out H5-mechanism-entirely-wrong; points to H5 partial — second defect downstream of setter call.

**Forward chain**: Andrew Q4 directive 2026-05-05 — "option a and then b if that doesnt work". Option A = instrument-and-rerun (Round 1 supplement). Option B = re-open diagnostic with broader hypothesis space (deferred per Andrew's branching directive).

**Retroactive classification (per Round 1 closure below)**: harmless redundancy. Setter relocation operates on a gate that wasn't blocking at runtime (default scene auto-populates per main.rs:2700-2703 — Diagnostic H5 mechanism contradicted at runtime). Revert deferred to eventual real-fix session for clean recovery narrative.

**Scope held**: Sub-phase 3 Mediator Brush Fix.A session only modified `tools/aw_editor/src/main.rs` (commit `8f4668599`). NO investigation/fix of analogous catch-all gate bugs. NO ViewportWidget per-tool fields removal. NO RegionalArchetypePanel changes. NO test additions beyond Andrew-gate verification. NO architectural changes to mediator pattern.

### 2026-05-06, Sub-phase 3 Mediator Brush Diagnostic Instrumentation Round 1 closure, commits e037d63b5 (Round 1 Instrumentation.A) + 040d90b16 (Round 1 Instrumentation.B revert) + df4a50e74 (this commit)

**Sub-phase 3 sub-task — runtime evidence capture supplement to Sub-phase 3 Mediator Brush Diagnostic. Triggered by Mediator Brush Fix REGRESS verdict 2026-05-05 with FPS-spike disappearance indicating fix reached intended code path. Pure data-gathering; 5 eprintln points added (Round 1 Instrumentation.A) + reverted (Round 1 Instrumentation.B); production code returned to post-fix state. Round 1 closure entry codifies findings + outcome classification + methodology lessons.**

**Pre-execution discovery** (per Instrumentation.A prompt §1.3, surfaced before instrumentation):

The Sub-phase 3 Mediator Brush Diagnostic audit §4.5 H5 mechanism rests on `scene_state` being None in Path 1 (no scene loaded). Code inspection at `main.rs:2700-2703` revealed `EditorApp::new()` auto-populates `scene_state` with a default world during construction; `main.rs:5598-5601` has a secondary safety net that re-creates `scene_state` on every frame if somehow nulled. **scene_state is therefore NEVER None after editor startup.** The H5 mechanism contradicted at code-read level; "Path 1 vs Path 2" distinction collapsed (both functionally have a default scene). The Mediator Brush Fix at `8f4668599` was harmless but likely redundant.

New prime suspect surfaced: `tab_viewer/mod.rs:142-147` catch-all gate — `if let (Some(viewport), Some(world), Some(entity_manager), Some(undo_stack))` inside `EditorDrawContext::ui()` for `PanelType::Viewport`. This gate was visible during Sub-phase 1 Diagnostic but not classified as brush-relevant since it didn't appear in the explicit TerrainPanel→viewport mediator data-flow chain.

**Andrew approved refined Option α 2026-05-05**: instrumentation supplement with 5-point set retargeting at the new prime suspect, over Option β (immediate revert + re-diagnose). Runtime confirmation cheap; Points 3-5 capture the actual silent-failure site regardless of which hypothesis prevails.

**Round 1 captured output** (frames at 2026-05-06T01:13:18-20Z timestamps; ~60 frames; consistent every frame):

```
[BRUSH-DBG] scene_state.is_some()=true, world_opt.is_some()=true
[BRUSH-DBG] tab_viewer-gate: viewport=true, world=true, entity=true, undo=true, full=true
[BRUSH-DBG] handle_input-entry: terrain_brush_active=true, terrain_brush_radius=50, terrain_brush_is_paint=false
[BRUSH-DBG] hit-gate: brush_active=true, no_gizmo=true, pointer=false, full=false
[BRUSH-DBG] render-gate: brush_active=true, no_gizmo=true, full=true
```

AstraWeave's own `Selection check: clicked=false, pointer_over=true, gizmo_active=false` — egui sees pointer over rect but registers zero clicks.

**Hypothesis evaluation post-Round 1**:

- Sub-phase 3 Mediator Brush Diagnostic H5 (`Some(world)` gate blocks setter): **REFUTED**. scene_state always Some.
- tab_viewer:142 catch-all gate (Round 1 prime suspect): **REFUTED**. Gate fires every frame.
- Audit §4.4 H4 REFUTAL ("ViewportWidget input-flow regression" REFUTED): **WRONG**. egui response is broken — sense rect not capturing clicks despite `Sense::click_and_drag()` in source.

**Two distinct defects identified**:

- **Defect A — Input routing**: egui response's `dragged_by(Primary)` + `clicked_by(Primary)` return false despite confirmed user click+drag. Possible causes: overlay layer interception, sense rect vs render rect geometry mismatch, missing sense flags, layer ordering issue.
- **Defect B — Render body silent no-op**: render code body executes every frame yet produces no visible cursor or ring overlay. Render uses `set_brush_cursor_lines(lines)` (renderer-side debug-line pipeline at viewport/renderer.rs:832) NOT egui painter. Possible causes: invalid pointer position used for ring center, off-screen coords, zero alpha, lines never reaching renderer's combined output, debug-line render disabled.

May share root cause (e.g., geometry mismatch causing both lost clicks AND invisible ring) or be independent.

**Mediator Brush Fix `8f4668599` retroactive classification**: harmless redundancy. Relocated setter calls outside a gate that wasn't blocking them at runtime. Setters work either way. Revert deferred to eventual real-fix session for clean recovery narrative ("H5 fix landed, was redundant; real defect at <X>; H5 fix reverted as cleanup").

**FPS spike on paint mode**: post-fix disappearance was coincidence; reappeared in Round 1 capture. Audit §4.7 H7 (`ensure_thumbnails_loaded` first-call cost) was right after all.

**Methodology lesson — Code-reading-derived hypotheses keep being wrong in this stack**:

Three rounds where code-reading-based hypotheses produced confident-but-wrong root causes:
1. Sub-phase 3 Mediator Brush Diagnostic H5 (`Some(world)` gate) — wrong. Default scene auto-populates.
2. tab_viewer:142 catch-all gate (new prime suspect) — wrong. Gate fires correctly.
3. Audit §4.4 H4 REFUTAL ("ViewportWidget input-flow regression") — wrong. egui response is broken.

Pattern: AstraWeave's editor has multiple layered abstractions (egui dock → tab_viewer → viewport widget → renderer → mediator); code-reading misses runtime behavior in this stack. **Targeted instrumentation produces actionable evidence; code-reading-on-paper does not.**

**Future foundational architectural campaigns**: when paper-hypotheses miss runtime, runtime instrumentation is more reliable than another paper-round. Mediator Removal session per Q6 will collapse some of these layers; this experience adds empirical weight to that architectural motivation.

**Forward chain**:

1. **Round 2 instrumentation supplement** (next commits): 6 targeted eprintlns in viewport/widget.rs disambiguating Defect A (response state + global pointer + geometry + layer detection) + Defect B (render body pointer + render body draw call).
2. **Per Round 2 outcome**: small fix prompt (Outcomes 1-4 of Round 2 prompt §0.4); formal Option B re-diagnostic (Outcome 5); Round 3 instrumentation (Outcome 6).
3. **Mediator Brush Fix `8f4668599` revert** as cleanup at real-fix session closeout.
4. **Sub-phase 3.C closeout** post-real-fix Andrew-gate-PASS.
5. **Sub-phase 4 + Sub-phase 5 + Mediator Removal session + Sub-phase 6** per campaign doc.

**Scope held**: Round 1 instrumentation supplement session only modified `tools/aw_editor/src/main.rs` + `tools/aw_editor/src/tab_viewer/mod.rs` + `tools/aw_editor/src/viewport/widget.rs` (added at `e037d63b5`; reverted at `040d90b16`) + this Editor Multi-Tool Architecture campaign doc Status header + §11 phase status block + §12 (this commit). NO production behavior retained post-revert. NO fixes applied. NO Mediator Brush Fix revert (deferred). NO formal Option B re-diagnostic (held pending Round 2 outcome).

### 2026-05-06, Sub-phase 3 Mediator Brush Diagnostic Round 2 Instrumentation closure, commits 0b6bc6f50 (Round-2-Instrumentation.A) + 4a2aca24b (Round-2-Instrumentation.B) + 7dc342a42 (Round-3-Re-Diagnostic.A audit) + 19eccad3c (this commit, Round-2-Closure landed inline)

**Round 2 instrumentation supplement closure — captures runtime evidence + outcome classification + methodology lesson. Round-2-Closure entry deferred from Round 2 session per Round 2 prompt §3.4; landed in Round 3 Re-Diagnostic session inline with Round-3-Re-Diagnostic.B campaign doc update.**

**Round 2 captured output** (multiple sections; active brush use section is load-bearing):

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

**Hypothesis evaluation post-Round 2**:

- Defect A (input routing) framing: **REFUTED**. egui captures input correctly during click+drag — `contains_pointer=true, dragged_primary=true, is_pointer_button_down_on=true, sense_click=true, sense_drag=true`. Sense flags correct.
- Defect B (render body silent no-op) framing: **PARTIALLY REFUTED**. Render body executes every frame with valid-looking parameters; not silently skipped. Whatever is wrong is downstream of draw call execution.
- Round 1 `pointer=false` data was misleading capture-timing artifact: Andrew's actual click+drag timing didn't align with the capture window when only Round 1's hit-gate filter was firing.

**Defect class identified post-Round-2**: rendering-pipeline failure between draw call and visible pixel. Brush is fully functional at data layer (clicks captured; hits collected; chunks modified; undo recorded); ring drawing code executes with valid-looking parameters; yet Andrew sees nothing visually. Narrower hypothesis space than original "TerrainPanel brush mediator path regression" framing.

**Outcome classification per Round 2 prompt §0.4**: Outcome 6 — unexpected mechanism. Original Defect A + Defect B framings revealed incomplete/wrong; rendering-pipeline defect class wasn't enumerated in Round 2's outcome decision tree.

**Methodology lesson — capture-timing artifacts in Round 1 misled hypothesis space**: Round 1's `pointer=false` data led to Defect A framing that was wrong. Future instrumentation sessions should verify capture-timing alignment with user actions; log when filter conditions fire (meta-instrumentation) to confirm captures happen during expected user activity; use less-restrictive filters when defect's runtime triggers are uncertain.

**Forward chain post-Round 2**:

1. Round 3 formal re-diagnostic audit (`7dc342a42`) synthesizing three rounds + enumerating remaining hypothesis classes (rendering-pipeline failures).
2. Round 4 instrumentation prompt drafting (next session per Andrew's two-session-minimum directive).
3. Round 4 instrumentation execution (session after that).
4. Per Round 4 outcome: small fix prompt; further instrumentation; architectural reconsideration.

**Scope held**: Round 2 instrumentation supplement session only modified `tools/aw_editor/src/viewport/widget.rs` (added at `0b6bc6f50`; reverted at `4a2aca24b`) + this Editor Multi-Tool Architecture campaign doc §12 (this commit, Round-2-Closure landed inline with Round-3-Re-Diagnostic.B). NO production behavior retained post-revert. NO fixes applied. NO Mediator Brush Fix revert (deferred). NO Round 4 instrumentation in this entry.

### 2026-05-06, Sub-phase 3 Mediator Brush Round 3 Re-Diagnostic, commits 7dc342a42 (Round-3-Re-Diagnostic.A audit) + 19eccad3c (this commit)

**Sub-phase 3 sub-task — formal re-diagnostic audit synthesizing three rounds of prior runtime evidence + enumerating remaining hypothesis space for rendering-pipeline defect class. Cross-reference entry; the audit at `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_round_3_re_diagnostic_2026-05-06.md` is the load-bearing artifact.**

Honors Andrew's earlier Option B directive ("option a then b if a doesn't work") — Option A produced two rounds of instrumentation (Round 1 + Round 2); Round 2 revealed defect class is rendering-pipeline rather than data-flow; Option B re-diagnostic now warranted.

**Pre-execution verification** (per Round 3 prompt §1):

- §1.1 Predecessor re-read with synthesis intent: complete; original audit's §4.4 H4 REFUTAL + §4.5 H5 mechanism + §4.7 H7 classification all retroactively wrong (code-reading-derived; never runtime-verified).
- §1.2 Sub-phase 3 status confirmed (10 commits landed pre-Round-3) + Round 2 Instrumentation.B status drift surfaced (NOT yet landed; landed inline as `4a2aca24b` before audit lands per §1.2 directive).
- §1.3 Codified evidence inventory verified.
- §1.4 Anti-drift discipline reaffirmation: held (6 specific drift temptations enumerated).

**Deliverables**:

- **Round-3-Re-Diagnostic.A** (commit `7dc342a42`): audit document at `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_round_3_re_diagnostic_2026-05-06.md` (~700 lines, hypothesis-classification structure with synthesis emphasis).
- **Round-3-Re-Diagnostic.B** (this commit): cross-reference §12 entries (Round-2-Closure inline + Round-3-Re-Diagnostic) + Status header line + §11 phase status block update.

**Findings summary — 8 hypothesis classes for rendering-pipeline defect**:

- **Class 1 — World-to-screen projection lands ring outside visible viewport rect**: PLAUSIBLE (prime candidate). Round 4 targets T1.A-D.
- **Class 2 — Render target mismatch (ring drawn to wgpu pass not composited into visible egui frame)**: PLAUSIBLE. Round 4 targets T2.A-C.
- **Class 3 — Z-depth occlusion (ring drawn behind terrain)**: PLAUSIBLE-MEDIUM. Round 4 targets T3.A-D.
- **Class 4 — Color/alpha/visibility properties effectively-invisible**: PLAUSIBLE-LOW. Round 4 targets T4.A-D.
- **Class 5 — Coordinate system mismatch (ring uses one camera; terrain uses another)**: PLAUSIBLE-MEDIUM. Round 4 targets T5.A-C.
- **Class 6 — Terrain mesh not regenerating after modification**: PLAUSIBLE-MEDIUM. Round 4 targets T6.A-C.
- **Class 7 — Frame-timing / state-mutation issue**: PLAUSIBLE-LOW. Round 4 targets T7.A-B.
- **Class 8 — Something else not yet enumerated**: ACKNOWLEDGED. Comprehensive state-logging in T1+T2+T5 captures unexpected anomalies.

**Round 4 instrumentation specifications**: 12 eprintln points across 8 hypothesis classes. Coverage commitment: every PLAUSIBLE + PLAUSIBLE-MEDIUM class has at least one runtime check. Filter patterns prevent log spam (state-change + throttling + first-N-frames-only + interesting-events-only).

**Mediator Brush Fix `8f4668599` retroactive classification**: harmless redundancy (relocated setter calls outside non-blocking gate). Revert deferred to eventual real-fix session for clean recovery narrative.

**Methodology lessons (audit §7)**:

- §7.1 — Three-rounds-wrong-from-code-reading pattern. AstraWeave's editor has multiple layered abstractions (egui dock → tab_viewer → viewport widget → wgpu renderer → mediator) where code-reading misses runtime behavior.
- §7.2 — Multi-round instrument-and-narrow as canonical pattern. Two-rounds soft threshold; three-rounds canonical threshold for formal synthesis.
- §7.3 — Pre-execution actual-code verification as discipline pattern. Predecessor audit findings should be runtime-verified during pre-execution, not assumed-as-true.
- §7.4 — Audit-confidence-calibration honesty. Hypothesis-classification audits should annotate each CONFIRMED / REFUTED rating with evidence type (code-reading vs runtime-verified vs mixed).
- §7.5 — Synthesizing-artifact threshold (round-3-or-later). Empirical not prescriptive; depends on hypothesis space complexity + audit reliability degradation.

**Forward chain**:

1. Round 4 instrumentation prompt drafting (next session per Andrew's two-session-minimum directive). References this audit's §3 hypothesis classes + §4.2 instrumentation specifications as load-bearing input.
2. Round 4 instrumentation execution (session after that). Andrew runs editor; captures `[BRUSH-DBG]` output; agent reverts.
3. Per Round 4 outcome: small fix prompt; further instrumentation; architectural reconsideration.
4. Real-fix Andrew-gate-PASS → Mediator Brush Fix `8f4668599` reverted as cleanup → Sub-phase 3.C closeout → campaign chain proceeds (Sub-phase 4 + 5 + Mediator Removal session + Sub-phase 6).

**Forward observations** (per audit §6, NOT in this audit's scope):

- §6.1 Original audit's confidence calibration (three wrong code-reading hypotheses) — methodology-lesson territory; no fix in this audit.
- §6.2 Architectural fragility of multi-painter rendering pipeline — Mediator Removal session per Q6 motivates simplification.
- §6.3 Capture-timing artifacts in Round 1 misled hypothesis space — future instrumentation sessions should verify capture-timing alignment.
- §6.4 Observability gap (existing Selection check log line useful; similar built-in observability for rendering pipeline would prevent diagnostic burden) — future logging additions could prevent.

**Scope held**: Round 3 Re-Diagnostic session only modified `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_brush_diagnostic_round_3_re_diagnostic_2026-05-06.md` (commit `7dc342a42`) + `tools/aw_editor/src/viewport/widget.rs` (Round 2 Instrumentation.B revert at `4a2aca24b` per §1.2 deferred-precondition discipline) + this Editor Multi-Tool Architecture campaign doc Status header + §11 phase status block + §12 (Round-2-Closure entry above + Round-3-Re-Diagnostic entry, this commit). NO production behavior retained post-revert. NO Round 4 instrumentation. NO fixes. NO Mediator Brush Fix revert (deferred). NO architectural reconsideration. NO new code-reading-derived mechanism claims.

### 2026-05-06, Sub-phase 3 Mediator Brush Diagnostic Round 4 Closure, commit 3bf425053

**Round 4 instrumentation closure — captures runtime evidence resolving rendering-pipeline defect class to depth pick chain. 12 targeted eprintlns across 8 hypothesis classes per Round 3 audit §4.2; aggressive scope per Andrew's directive; ~14 actual eprintln points landed per code-drift adjustments in Round-4-Instrumentation.A `de1986301`. Captured output `brush-dbg-r4.txt` totals 6,641 BRUSH-DBG lines.**

**Defect class narrows to depth pick chain**: `read_depth_at_pixel` (viewport/camera.rs:~1145) + `unproject_depth_to_world` (viewport/camera.rs:~541) + depth target binding lifecycle. Brush IS fully functional at data layer; ring-not-visible + terrain-modification-not-visible are downstream symptoms of cursor_center being computed at cam_pos rather than at terrain surface.

**Round 4 captured signal lines** (representative samples from 6,641 total BRUSH-DBG lines):

T1.A+B+D first frame (depth pick falls back to Y=0 plane raycast):

```
[BRUSH-DBG] proj-cursor: world=(-123.49,0.00,885.05), screen=(384.8,567.2),
  ndc_z=0.9996, clip_w=970.27, cam_pos=(810.1,888.8,810.1),
  cam_target=(0.0,227.4,0.0), cam_dist=1291.2, screen_radius=18.1,
  viewport_size=(1014,562), response_rect=(236,52)..(1250,614)
```

T1.A+B+D subsequent frames (depth pick "succeeds" — but returns near-plane depth):

```
[BRUSH-DBG] proj-cursor: world=(809.77,888.45,809.99), screen=(592.5,495.5),
  ndc_z=0.0000, clip_w=0.5000, cam_pos=(810.1,888.8,810.1),
  cam_target=(0.0,227.4,0.0), cam_dist=0.5, screen_radius=615.5,
  viewport_size=(1014,562), response_rect=(236,52)..(1250,614)
```

T3.A every frame (line0 clip-z beyond far plane):

```
[BRUSH-DBG] ring-z: line0_start_ndc_z=1.0167, line0_end_ndc_z=1.0148,
  line0_y_offset=0.15
```

T2.A only one fire (dock-based path; standalone never fires):

```
[BRUSH-DBG] render-path: dock-based-viewport-ui (tab_viewer:142)
```

T2.B every render (composite reaches with 48 lines; renderer initialized):

```
[BRUSH-DBG] render-lines-at-composite: brush_cursor_lines.len()=48,
  gizmo_lines=0, phys_lines=0, zone_overlay_lines=0, total=48,
  physics_renderer_init=true
```

T4.A first call (RGB only; DebugLine has no alpha):

```
[BRUSH-DBG] line-color-rgb: r=0.200, g=1.000, b=0.300, line_count=48
  (NOTE: DebugLine has no alpha; alpha is shader-implicit)
```

T5.A every throttle (cam-widget matrices stable across frames):

```
[BRUSH-DBG] cam-widget: view_row0=(0.7071,0.0000,-0.7071,-0.0000),
  proj_row0=(0.9611,0.0000,0.0000,0.0000),
  proj_diag=(0.9611,1.7321,-1.0001,0.0000)
```

T6.A every brush apply (modified=true; chunk consistently 242):

```
[BRUSH-DBG] apply-brush-at: world_x=809.87, world_z=809.89, mode=Sculpt,
  radius=50.0, strength=1.00, modified=true, pending_actions_len=0
```

T6.B every brush event (chunk 242 dirty; viewport available):

```
[BRUSH-DBG] terrain-brush-update-event: dirty_count=1,
  dirty_indices=[242], viewport_is_some=true
```

T7.A every set-lines call (success-1489 dominates during active hover; fallback-1494-no-hover-pos when pointer leaves; no wipe pattern within frame):

```
[BRUSH-DBG] set-lines-call: site=success-1489, lines_count=48
[BRUSH-DBG] set-lines-call: site=fallback-1494-no-hover-pos, lines_count=0
[BRUSH-DBG] set-lines-call: site=fallback-1497-gate-off, lines_count=0,
  brush_active=false, gizmo_active=false
```

**Hypothesis class confirmation per Round 4 evidence**:

| Class | Status | Evidence |
|-------|--------|----------|
| 1 — World-to-screen projection off-screen | REFUTED at projection level | screen positions all INSIDE response_rect=(236,52)..(1250,614). The projection math itself is fine. |
| 2 — Render target mismatch | REFUTED | T2.A only fires dock-based path; T2.B shows brush_cursor_lines.len()=48 reaches composite; physics_renderer_init=true. |
| 3 — Z-depth occlusion | DOWNSTREAM SYMPTOM, not root cause | line0_ndc_z=1.0167 > 1.0 (beyond far plane) is downstream of cursor_center being placed at cam_pos. Ring vertices necessarily fall outside view frustum when their center IS the camera position. Class 3 framing assumed ring center was at terrain surface and vertices were near-plane occluded; actual mechanism is different. |
| 4 — Color/alpha | REFUTED | r=0.200, g=1.000, b=0.300 (bright green); 48 lines; DebugLine API has no alpha field — alpha is shader-implicit (set per-pipeline, not per-line). |
| 5 — Coordinate system mismatch | REFUTED | T5.A cam-widget matrices stable across hundreds of throttled samples; renderer uses same OrbitCamera reference. |
| 6 — Terrain mesh not regenerating | REFUTED at the data layer; downstream visibility issue remains | T6.A modified=true on every apply-brush-at; T6.B dirty_count=1, dirty_indices=[242] every event; viewport_is_some=true. Modification IS reaching the GPU — the chunk being modified happens to be at world (~810, ~810) which corresponds to camera position, so visual confirmation requires looking at chunk 242 specifically. Not a regen failure; a "modification happens at unexpected world location" consequence of upstream defect. |
| 7 — Set-lines wipe | REFUTED | T7.A shows clean control flow: `success-1489` during active hover; `fallback-1494-no-hover-pos` only when pointer leaves viewport (no hover position); no interleaved wipe pattern within a single frame. |
| 8 — Unenumerated mechanism | NOT NEEDED for primary root cause | Depth pick chain explanation sufficient. |

**Outcome classification per Round 4 prompt §5**: **Outcome 5 (unexpected mechanism)** — the rendering-pipeline defect class enumeration in Round 3 audit §3 was incomplete. None of the enumerated mechanisms within Classes 1-7 capture the actual root cause. Round 4 evidence narrows defect to a sub-class not separately enumerated: **depth pick chain (read_depth_at_pixel → unproject_depth_to_world → cursor_center) returns near-plane depth instead of terrain surface depth**. This is upstream of all eight enumerated rendering-pipeline classes; its consequences masquerade as Class 3 (z-depth) symptoms.

**Critical correction — supersedes `bc9c8d5bd` commit message body**:

`bc9c8d5bd` (Round-4-Instrumentation.B) commit body codifies the analysis: *"Class 3 confirmed at clip-plane level, root-caused by camera positioning. cam_dist=0.5 world units while ring radius=50 world units; camera is geometrically INSIDE the ring; Andrew zoomed in extremely close to a mountain peak."* This analysis is **wrong**. The §12 entry corrects it forward.

Why the agent's interpretation is wrong:

1. **Camera was not zoomed in.** cam_pos=(810.1, 888.8, 810.1) and cam_target=(0.0, 227.4, 0.0). Distance from cam_pos to cam_target is sqrt(810² + 661² + 810²) ≈ 1322 world units. The OrbitCamera is at default-orbit-distance from its pivot — not zoomed in. If Andrew had truly zoomed in to a mountain peak at (810, 888, 810), the orbit camera's `cam_target` would be at or near (810, 888, 810), not at world origin.

2. **First-frame evidence directly disproves the "zoomed in" interpretation.** The very first capture frame shows the depth-pick fallback path: `world=(-123.49, 0.00, 885.05), cam_dist=1291.2`. This is the Y=0 ground-plane raycast that fires when depth pick fails. cam_dist=1291.2 in this frame proves the camera was at normal orbit distance — nowhere near "zoomed in." Yet cam_pos and cam_target did NOT change between this first frame and subsequent frames showing cam_dist=0.5. Only the depth-pick result changed.

3. **`cam_dist=0.5` is a derived value, not a primary fact.** `cam_dist` is computed from cam_pos and cursor_center at log-time: `cam_dist = (cursor_center - cam_pos).length()`. When cursor_center ≈ cam_pos, cam_dist ≈ 0 by definition. The agent treated this derived equality as causal evidence ("camera close to brush") when it is actually a *consequence* of cursor_center being incorrectly computed at cam_pos.

4. **`cursor_center` is computed by the depth pick chain, not by Andrew's input.** Andrew's screen cursor position varies substantially across frames (screen=(592.5,495.5) → (706.5,499.5) → (740.4,495.5) → (1079.5,133.5) → many distinct positions during a 6,641-line capture). Yet cursor_center stays invariantly within a tiny range: x ∈ (809.5, 810.0), y ∈ (888.4, 888.6), z ∈ (809.7, 810.0) ≈ cam_pos to within ~0.5 world units. **Regardless of where Andrew clicks on screen, the brush position is computed at the camera position.** This is the diagnostic signal: the depth pick chain is broken.

5. **NDC z=0.0000, clip_w=0.5000 directly indicates near-plane unprojection.** Standard wgpu projection has NDC z ∈ [0, 1] where 0 is near plane and 1 is far plane. `unproject_depth_to_world(screen_pos, depth=0.0)` returns the world-space point on the near clip plane along the screen ray. For a near plane at ~0.5 world units in front of the camera, this returns a world position approximately at cam_pos plus a tiny screen-position-dependent offset — exactly the pattern observed in cursor_center.

6. **The agent's verification recommendation (zoom out and re-test) would not work.** Zooming out moves cam_pos farther from cam_target along the orbit radius. cursor_center would still equal cam_pos because the depth pick still returns ~0.0; the brush would still modify the chunk under the camera, just a *different* chunk. The agent's prediction that "brush will work visually at normal zoom levels" is falsified by Round 4's own data — the camera was already at normal zoom levels (cam_dist-to-target=1322) and the brush still didn't work.

**Updated defect framing**:

The brush has **always been working at the data layer**: T6.A modified=true on every apply-brush-at; T6.B dirty_count=1 dirty_indices=[242] on every event; viewport_is_some=true. The defect is upstream in the **depth pick chain**: when Andrew clicks on screen at pixel (px, py), the chain `read_depth_at_pixel(px, py) → unproject_depth_to_world(screen_uv, depth) → cursor_center` returns a world position at the camera's near plane (≈cam_pos) regardless of what's actually rendered at (px, py).

This explains every Andrew-gate symptom across all rounds simultaneously:

- **No visible ring** — ring center is at cam_pos; ring vertices project to behind near plane (negative w; discarded by GPU clip) and beyond far plane (line0_ndc_z=1.0167; clipped). Only a degenerate sliver of ring would be in NDC range; with line width 1px, invisible.
- **No visible terrain modification** — modification IS happening at world (~810, ~810) because that's what the broken depth pick returns; chunk 242 IS being dirtied; upload IS reaching GPU. But chunk 242 corresponds to a world position at the camera, not under Andrew's cursor. The modification is happening "under the camera" or behind it relative to view direction (since cam_target is far from cam_pos, the camera is looking AWAY from cam_pos toward origin).
- **No cursor change** — same mechanism as no visible ring.

**Possible mechanisms within depth pick chain** (Round 5 instrumentation will runtime-verify):

1. `read_depth_at_pixel` reads from a depth texture that doesn't contain terrain depth — perhaps a different render target, perhaps the depth for an overlay pass, perhaps an uninitialized buffer.
2. `read_depth_at_pixel` reads correctly but the depth buffer is cleared between terrain render and the depth read — reads return ~0 (cleared depth value).
3. `read_depth_at_pixel` reads correctly but the pixel coordinates being sampled are outside the terrain-rendered region — reads return cleared near-plane value.
4. `unproject_depth_to_world` has a math error that handles depth=0.0 incorrectly (e.g., reverse-Z assumption when buffer uses standard projection, or vice versa).
5. The depth buffer has correct values but `read_depth_at_pixel` reads the wrong mip level / wrong texture array slice / wrong region.
6. Some other unenumerated mechanism within the chain.

Round 5 instrumentation specifications target these candidates: log raw depth value returned by `read_depth_at_pixel`, log depth target binding lifecycle, log pixel coordinates used for depth pick + viewport rect alignment, log depth texture handle/binding identity.

**Forward chain**:

1. **Round 5 instrumentation prompt drafted next session** (per Andrew's two-session-minimum directive) — targets depth pick chain at 5-6 eprintln points: `read_depth_at_pixel` raw return value, depth texture identity at sample time, depth target binding lifecycle (create / clear / bind), terrain pipeline depth-write-enabled state, pixel coords + viewport rect alignment. Smaller scope than Round 4's 12 targets; one capture session expected to resolve.
2. **Round 5 Instrumentation.A lands** following session.
3. **Andrew runs editor + captures `brush-dbg-r5.txt`**.
4. **Round-5-Closure.A** classifies outcome — single mechanism confirmed → small fix prompt; multiple mechanisms → combined or sequential fix; all refuted → Round 6 instrumentation or architectural reconsideration of depth pick chain.
5. **Real-fix Andrew-gate-PASS** → Mediator Brush Fix `8f4668599` reverted as cleanup → Sub-phase 3.C closeout.
6. **Sub-phase 4 + 5 + Mediator Removal session + Sub-phase 6 closeout** per established forward chain.

**Methodology lesson surfacing — derived-value reasoning trap**:

This is a new lesson candidate, sibling to Round 3 audit §7.1-§7.5. Future re-diagnostic audits (Round 6+ if needed) can compile this into §7 audit lessons. Codified here for first-pass reference.

**Pattern**: When an evidence-collection session captures multiple values, some primary (read directly from runtime state) and some derived (computed from primary values at log-time), reasoning about value relationships requires distinguishing primary from derived. The agent treated `cam_dist=0.5` as primary causal evidence ("camera close to brush") when it is a derived quantity computed from cam_pos and cursor_center. The actual primary fact is: cursor_center ≈ cam_pos invariantly. The derived fact (cam_dist≈0) is a consequence.

This lesson generalizes: **when two values are unexpectedly equal, ask whether one is being computed from the other before reasoning about the equality as causal**. If yes, the equality is mathematical, not empirical — reasoning must trace upstream to whatever produced the input that, when computed, yielded the equality.

Sibling-lesson cross-references:

- **§7.1 (three-rounds-wrong-from-code-reading)** — both lessons concern misattribution of causation. §7.1: code-reading produces mechanism hypotheses that don't survive runtime; §7.6 candidate: derived values produce causation hypotheses that don't survive primary-value tracing.
- **§7.4 (audit-confidence-calibration honesty)** — both concern the importance of distinguishing evidence types. §7.4: code-reading vs runtime-verified claims; §7.6 candidate: primary vs derived evidence within a single runtime capture.

**Anti-drift safeguard for future closure-drafting agents**: the prior agent's `bc9c8d5bd` commit body is preserved as a methodology-lesson exhibit. Future agents reading this §12 entry should also read `git show bc9c8d5bd` to see the wrong analysis in full context — recognizing the trap-pattern is the lesson's intent.

**Scope held**: Round-4-Closure.A session only landed campaign-doc updates (Status header + §11 entries + this §12 Round-4-Closure entry). NO production code changes. NO fixes applied. NO Mediator Brush Fix `8f4668599` revert. NO modifications to prior commits including `bc9c8d5bd` (preserved as-landed; campaign doc supersedes its commit message body as authoritative source of truth).

### 2026-05-07, Sub-phase 3 Mediator Brush Diagnostic Round 5 Closure, commit 569415a7a

**Round 5 instrumentation closure — captures runtime evidence DEFINITIVELY CONFIRMING Mechanism 1 (wrong texture, "different render target" sub-class) per Round 4 closure §12 enumeration. 5 targeted eprintlns across depth pick chain per Round 4 audit. Captured output `brush-dbg-r5.txt` totals 1,955 BRUSH-DBG lines (filtered from `brush_dbg-r5.log`).**

**Outcome classification per Round 5 instrumentation prompt §3.6 outcome decision tree**: **Outcome 1** — single mechanism confirmed cleanly. Row matches: T8.A=0.0 (depth read returns near-plane invariantly) + T8.B=mismatch (terrain-pass-write target ≠ depth-pick-read source) + T8.E=consistent (math is correct for depth=0 input). 5 rounds of evidence-grounded narrowing converged in one Round 5 capture session.

**Round 5 captured signal patterns** (representative samples from 1,955 total BRUSH-DBG lines):

T8.A (`read_depth_at_pixel` raw return; 650/650 reads):

```
[BRUSH-DBG] depth-pick-raw: pixel=(<varies>, <varies>), depth_value=Some(0.000000)
```

Pattern: invariant `Some(0.000000)` regardless of pixel position. 650/650 reads return same value.

T8.B (depth target write/read identity per frame):

```
[BRUSH-DBG] depth-target-write: frame=N, target_handle=<engine_adapter_internal>, ...
[BRUSH-DBG] depth-target-read: frame=N, source_handle="Viewport Depth Texture", size=(W,H), format=Depth32Float
```

Pattern: write target is engine adapter's internal depth target (allocated within adapter; not the same handle as aw_editor's `self.depth_texture`). Read source is `self.depth_texture` ("Viewport Depth Texture", Depth32Float). **Handle mismatch every frame** — smoking gun.

T8.C (terrain pipeline depth-write state at adapter dispatch site):

```
[BRUSH-DBG] terrain-pipeline-depth-state: render_to_texture call site — scene_target_view passed as color attachment; NO depth attachment parameter
```

Pattern: `engine_adapter::render_to_texture(scene_target_view, encoder)` API has no depth attachment parameter. Adapter manages its own internal depth target inside the call. aw_editor's `self.depth_view` is NEVER reachable to the terrain pass.

T8.D (pixel coords + viewport_rect alignment; 650/650 samples):

```
[BRUSH-DBG] depth-pick-coords: pixel=(<varies>, <varies>), viewport_rect=(<x0>,<y0>)..(<x1>,<y1>), inside_viewport=true, depth_texture_present=true, depth_staging_present=true
```

Pattern: 650/650 inside_viewport=true. Mechanism 3 (coords outside terrain region) refuted definitively.

T8.E (`unproject_depth_to_world` math sanity; 649/649 samples):

```
[BRUSH-DBG] unproject-input: depth_value=0.0, ndc_z=0.0, world≈cam_pos, dist=0.51-0.52
```

Pattern: 649/649 with depth_value=0.0 → ndc_z=0.0 → world position on near plane ≈ cam_pos plus tiny screen-position offset (dist=0.51-0.52 from cam_pos). Math is consistent with standard projection convention; no reverse-Z flip needed; no math error. Mechanism 4 (math error) refuted definitively.

**Hypothesis confirmation per Round 5 evidence**:

| Mechanism (per Round 4 closure §12) | Round 5 Verdict | Evidence |
|-------------------------------------|----------------|----------|
| 1 — Wrong depth texture (different render target / overlay pass / uninitialized) | **CONFIRMED ("different render target" sub-class)** | T8.B handle mismatch + T8.A=0.0 invariant + T8.C confirms adapter API has no depth attachment param |
| 2 — Depth buffer cleared between terrain render and depth read | REFUTED | Already refuted by T8.B mismatch evidence — terrain never wrote to the read texture in the first place; "cleared between" is moot |
| 3 — Pixel coords sampled outside terrain-rendered region | REFUTED | T8.D 650/650 inside_viewport=true |
| 4 — `unproject_depth_to_world` math error | REFUTED | T8.E 649/649 math consistent with standard projection convention |
| 5 — Wrong mip / array slice / region of correct texture | REFUTED by elimination | T8.B identifies the issue at texture-handle level, not mip/slice level |
| 6 — Unenumerated mechanism within depth pick chain | NOT NEEDED | Mechanism 1 explanation sufficient |

**Root cause statement**:

`tools/aw_editor/src/viewport/renderer.rs` invokes `engine_adapter.render_to_texture(scene_target_view, encoder)` to render terrain. The adapter's API accepts only a color target (`scene_target_view`); the adapter manages its own internal depth target inside the call (terrain depth values written to that internal target).

`tools/aw_editor/src/viewport/camera.rs::read_depth_at_pixel` samples a separately-allocated depth texture, `self.depth_texture` ("Viewport Depth Texture", Depth32Float). This texture is written ONLY by aw_editor-local overlay passes — grid lines, debug shapes, gizmos. Most of the texture is never written; cleared/uninitialized pixels return depth=0.0 on sample.

When user clicks at (px, py): `read_depth_at_pixel` returns `Some(0.000000)` (cleared value at unwritten pixel). `unproject_depth_to_world(screen_uv, 0.0)` returns world position on the near clip plane ≈ cam_pos plus screen-position-dependent offset (~0.5 world units). Brush position becomes cam_pos. Brush modifies chunk under camera (chunk 242 in Round 4 capture); modification not visible to user looking at terrain elsewhere; ring vertices project to clip-volume edges → not visible.

**Latent companion bug** (not in scope for fix decision but worth recording): aw_editor's overlay passes (gizmos, debug shapes) currently depth-test against `self.depth_view`. Since terrain renders to a *different* depth target, overlay passes cannot depth-test against terrain — gizmos that should be hidden behind a mountain currently render in front of it. Fix candidates (a) and (c) resolve both the depth-pick bug AND this latent overlay-vs-terrain depth-test bug; fix candidate (b) leaves the latent bug in place.

**Three fix candidate designs (Andrew-gate decision required)**:

**Candidate (a) — pass depth attachment to `engine_adapter::render_to_texture`**:

API change: `render_to_texture(scene_target_view, encoder)` → `render_to_texture(scene_target_view, depth_view: Option<&TextureView>, encoder)`. When `Some(depth_view)` is passed, terrain pass binds it as depth attachment; when `None`, adapter falls back to internal depth target (preserving current behavior for any non-aw_editor callers).

aw_editor's `viewport/renderer.rs` updated to pass `Some(&self.depth_view)` at the adapter call site. `read_depth_at_pixel` continues sampling `self.depth_texture` (no change) — but now terrain pass has written terrain depth to it, so reads return correct values.

Pros:
- Architecturally correct: wgpu RenderPass canonical convention is "caller provides color + depth attachments." Adapter's internal depth target becomes optional fallback.
- Resolves latent overlay-vs-terrain depth-test bug as a side effect (overlay passes now share depth buffer with terrain).
- Aligns with `CLAUDE.md` v0.10.1 Edit 2 (no-second-implementation): collapses two depth targets serving the same logical "viewport depth buffer" role into one.
- Backwards-compatible: `Option<&TextureView>` with `None` preserves existing adapter behavior for any non-aw_editor callers.
- Other future depth-pick consumers benefit (screen-space effects, shadow caster culling, etc.).

Cons:
- Cross-crate API change touches `astraweave_render`.
- Adapter's internal multi-pass setup may need adjustment if other internal passes depend on the internal depth target's identity (needs verification at fix-prompt drafting time).
- Existing non-aw_editor callers need the new `None` parameter added to call sites (compiler error surfaces all callers; mechanical fix).

**Candidate (b) — expose engine adapter's internal depth target via accessor**:

API change: add `engine_adapter::terrain_depth_view() -> &TextureView` (or similar accessor) to `astraweave_render`.

aw_editor's `read_depth_at_pixel` updated to sample from `engine_adapter.terrain_depth_view()` instead of `self.depth_texture`. `self.depth_texture` continues serving overlay passes only.

Pros:
- No render-call API change.
- aw_editor's overlay passes can still use `self.depth_texture` without modification.
- Smaller cross-crate touch (accessor only, no signature change to `render_to_texture`).

Cons:
- aw_editor becomes coupled to adapter's internal depth target identity. If adapter evolves (multi-pass, MSAA resolve, deferred shadow setup with depth pyramid, etc.), aw_editor's depth-pick breaks silently.
- Latent overlay-vs-terrain depth-test bug NOT resolved: overlay passes still depth-test against `self.depth_texture`, which still doesn't contain terrain depth. Gizmos still render in front of mountains incorrectly.
- Two depth targets formally maintained in the viewport architecture (one in adapter, one in aw_editor) — anti-pattern per `CLAUDE.md` v0.10.1 Edit 2 spirit.
- Leaky abstraction: aw_editor pierces engine adapter's encapsulation to read internal state.

**Candidate (c) — aw_editor-local depth pre-pass re-rendering terrain depth into `self.depth_view`**:

After `engine_adapter::render_to_texture` runs, aw_editor performs a separate depth-only pass that re-renders terrain geometry (depth-only pipeline) into `self.depth_view`.

Requires aw_editor to access terrain mesh data from adapter (still cross-crate, just via different API — mesh data accessor instead of depth target accessor).

Pros:
- `self.depth_view` remains the authoritative viewport depth target.
- Overlay passes' depth-test against terrain works correctly (both share `self.depth_view`).
- No render-call API change for primary terrain pass.

Cons:
- Redundant rendering work: terrain rasterized twice (once for color in adapter pass, once for depth in aw_editor pre-pass). Performance cost scales with terrain complexity.
- Still cross-crate: requires mesh data accessor on engine adapter (different cross-crate change than (a) or (b)).
- Precision-mismatch risk: two separate depth passes with potentially different shaders / transform matrices / precision can produce slightly different depth values, leading to subtle inconsistencies in depth pick + overlay tests near terrain edges.
- Architectural duplication remains: terrain renders happen in two places.

**Andrew-gate decision required**: Andrew picks (a), (b), (c), or specifies a 4th alternative. Decision documented in chat or follow-up commit. Small-fix prompt drafted in subsequent session per two-session-minimum directive.

**Forward chain**:

1. **Andrew-gate fix design decision** — Andrew picks (a), (b), (c), or alternative.
2. **Next session**: Small-fix prompt drafted per chosen design (per two-session-minimum). Specifies API changes, callers to update, cargo check + build + active_tool tests verification, Andrew-gate verification protocol.
3. **Following session**: Small-fix lands as single commit. cargo check + build + tests pass.
4. **Andrew-gate-PASS verification**: Andrew runs editor; brush ring visible; cursor follows screen click; terrain modifies under cursor (not under camera); modification visible.
5. **Cleanup session**: Mediator Brush Fix `8f4668599` reverted as cleanup (clean recovery narrative — `8f4668599` was harmless redundancy per Round 1 closure and Round 3 audit §5.1; reverting now produces a clean diff narrative).
6. **Sub-phase 3.C closeout** lands.
7. **Sub-phase 4 + 5 + Mediator Removal session + Sub-phase 6 closeout** per established forward chain.

**Methodology lesson surfacing — wrapped-component resource identity trap**:

This is a new lesson candidate, sibling to Round 4 closure §12's derived-value reasoning trap (§7.6 candidate). Future re-diagnostic audits (Round 6+ if needed; or a future audit reflecting on the fix's success) can compile this into §7 audit lessons. Codified here for first-pass reference.

**Pattern**: When a higher-level component (aw_editor's viewport) wraps a lower-level component (engine_adapter) that manages its own internal resources of the same logical type as the wrapping component (depth target), the higher-level component's reads from its own resource don't reflect the lower-level component's writes to its own resource — even though both call themselves "the depth buffer for this viewport."

The Round 5 case: aw_editor allocates `self.depth_texture` ("Viewport Depth Texture") as the viewport's depth target. engine_adapter allocates its own internal depth target. Terrain rasterizes into adapter's internal target; depth pick samples aw_editor's target. Both components believe they're working with "the viewport depth buffer." Neither is wrong about its own scope. The bug is at the boundary: the wrapping component never verified the wrapped component's resource identity.

This lesson generalizes: **when component A wraps component B and both manage similar resources, verify the resource identity matches before reasoning about resource content**. Equivalent observation: shared logical state across component boundaries needs explicit identity verification, not just content-level reasoning.

Sibling-lesson cross-references:

- **§7.6 candidate (derived-value reasoning trap, Round 4 closure)** — both lessons concern misattribution of state. §7.6: derived values misattributed as primary causes (cam_dist=0.5 vs cursor_center=cam_pos). §7.7 candidate: resource identity at component boundaries misattributed as resource content.
- **`CLAUDE.md` v0.10.1 Edit 2 (no-second-implementation rule)** — direct architectural sibling: "no-second-resource" is the resource-level corollary of "no-second-pipeline." Round 5 case (two depth targets serving the same logical role) is exactly the kind of duplication that Edit 2 forbids in the implementation-level sense; this lesson extends the rule to the resource-management level.
- **`CLAUDE.md` v0.10.1 Edit 3 (Integration Completeness checklist)** — possible future bullet: "verify resource identity at component boundaries when wrapping components that manage their own resources." Could be added to Edit 3 list if this pattern recurs across audits.

**Anti-drift safeguard for future closure-drafting agents**: this closure entry is structurally similar to Round-4-Closure §12 entry (same campaign-archeology resilience, same methodology-lesson surfacing pattern, same forward-chain structure). The §3.6 outcome decision tree mapped Round 5 evidence directly to Outcome 1 — no analytical correction needed, unlike Round 4's `bc9c8d5bd` derived-value trap. This confirms the methodology pattern: pre-baked outcome decision trees in instrumentation prompts protect against derived-value reasoning traps in closure analysis. Future instrumentation prompts should include outcome decision trees as a standard section.

**Scope held**: Round-5-Closure.A session only landed campaign-doc updates (Status header + §11 entries + this §12 Round-5-Closure entry). NO production code changes. NO fixes applied. NO Mediator Brush Fix `8f4668599` revert. NO modifications to prior commits. NO fix-prompt drafting (deferred to subsequent session per two-session-minimum directive). NO pre-deciding fix design (Andrew-gate decision required).

### 2026-05-07, Sub-phase 3 Mediator Brush Diagnostic Round 6 Closure, commit c7f3b50b3

**Round 6 instrumentation closure — captures runtime evidence DEFINITIVELY CONFIRMING Mechanism C (mesh resource identity trap, sibling §7.7 instance at mesh-data layer) with dual-variant sub-mechanism per Round 6 prompt §3.7 outcome decision tree. 6 targeted eprintlns across mesh-update chain per Round 5 closure §7.7 candidate framing extended to mesh-data layer. Captured output `brush-dbg-r6.log` totals 5,459 BRUSH-DBG lines.**

**Outcome classification per Round 6 instrumentation prompt §3.7 outcome decision tree**: **Outcome 3** — Mechanism C confirmed cleanly via T9.C smoking gun #3 (`update_terrain_chunk-success, affected_clusters=0` invariant across 1749/1749 entries) + T9.D paired confirmation (terrain_cluster_models=0 in self.models across all 238 samples) + heights changing correctly (T9.E delta ≈ 4.96) + other mechanisms refuted. 6 rounds of evidence-grounded narrowing converged on the second of two §7.7 instances in the same campaign.

**Round 6 captured signal patterns** (representative samples from 5,459 total BRUSH-DBG lines):

T9.A (`apply_brush_at` world coords; 67 samples):

```
[BRUSH-DBG] brush-apply-at: world=(277.09, 499.67), mode=Sculpt, radius=50.0, strength=1.00, height_pre=Some(101.71081)
[BRUSH-DBG] brush-apply-at: world=(256.78, 526.38), mode=Sculpt, radius=50.0, strength=1.00, height_pre=Some(126.991425)
[BRUSH-DBG] brush-apply-at: world=(284.86, 568.80), mode=Sculpt, radius=50.0, strength=1.00, height_pre=Some(169.1598)
```

Pattern: 67/67 brush apply calls show world coords vary across viewport (Andrew clicked across distinct screen positions). Mechanism A (cursor source mismatch) refuted — brush world coords match where Andrew clicked.

T9.E (chunk height delta pre/post; 67 samples paired with T9.A):

```
[BRUSH-DBG] brush-height-delta: world=(277.09, 499.67), height_pre=Some(101.71081), height_post=Some(106.66818), delta=Some(4.9573746), modified=true
[BRUSH-DBG] brush-height-delta: world=(256.78, 526.38), height_pre=Some(126.991425), height_post=Some(131.95161), delta=Some(4.96019), modified=true
[BRUSH-DBG] brush-height-delta: world=(284.86, 568.80), height_pre=Some(169.1598), height_post=Some(174.12067), delta=Some(4.960861), modified=true
```

Pattern: 67/67 height deltas ≈ 4.96 (close to 5.0 uniform sculpt strength). Heights ARE changing measurably at data layer. Mechanism E (sub-perceptible delta) refuted; Mechanism 6 (brush logic regression) refuted.

T9.B (event emission + consumption; 794 paired entries):

```
[BRUSH-DBG] terrain-brush-update-emit: site=...
[BRUSH-DBG] terrain-brush-update-handle: received_dirty_count=<n>, dirty_indices=<vec>, viewport_is_some=true, upload_call_made_per_chunk=true
```

Pattern: 794 emit / 794 handle 1:1 ratio. Event chain intact. Mechanism B (event handler missing or skipping upload) refuted.

T9.C (`update_terrain_chunk` 3-sub-path tracing; 1749 success entries):

```
[BRUSH-DBG] mesh-upload: site=update_terrain_chunk-entry, ...
[BRUSH-DBG] mesh-upload: site=update_terrain_chunk-success, affected_clusters=0
```

Pattern: 1749/1749 entry events fire success path; 0 early-return-no-slot-map; 0 early-return-no-stored-chunk. **All 1749 success entries show affected_clusters=0.** Smoking gun #3 — the `terrain_clusters.filter(|c| c.chunk_indices.contains(stored))` filter at engine_adapter.rs:1898 returns empty in EVERY case → `rebuild_terrain_clusters_for_chunk` is a no-op every time → legacy cluster GPU buffers never updated.

T9.D (terrain render source identity; 238 samples):

```
[BRUSH-DBG] terrain-render-source: models_total=0, drawn_models=0, terrain_cluster_models=0  (startup)
[BRUSH-DBG] terrain-render-source: models_total=96, drawn_models=39, terrain_cluster_models=0  (steady-state)
```

Pattern: 238 samples, 2 unique line patterns. `self.models` HashMap NEVER contains entries with `terrain_cluster_*` prefix at any point during runtime. **The legacy cluster path is DEAD CODE at this editor's configuration** — writes occur via `update_terrain_chunk` → `rebuild_terrain_clusters_for_chunk` (T9.C 1749 success entries) but nobody reads `self.models["terrain_cluster_*"]` at render time.

T9.F (engine_adapter instance identity; single fire at first render):

```
[BRUSH-DBG] engine-adapter-id: site=render_to_texture, adapter_ptr=0x489943a3020, terrain_chunks_len=0, terrain_clusters_len=0
```

Pattern: single adapter_ptr identity. Multi-instance trap (Mechanism C variant) refuted.

**Hypothesis confirmation per Round 6 evidence**:

| Mechanism | Round 6 Verdict | Evidence |
|----|----|----|
| A — Cursor source mismatch | REFUTED | T9.A 67/67 brush_world coords vary correctly |
| B — Event handler missing/skipping upload | REFUTED | T9.B 794 emit / 794 handle 1:1 |
| C — Mesh resource identity trap (sibling §7.7 at mesh-data layer) | **CONFIRMED with dual-variant sub-mechanism** | T9.C 1749/1749 affected_clusters=0 (smoking gun #3); T9.D 238/238 terrain_cluster_models=0 (paired confirmation) |
| D — Renderer mesh staleness | n/a (subsumed by C) | T9.D shows legacy path is dead code; staleness moot when path is unread |
| E — Sub-perceptible visual delta | REFUTED | T9.E 67/67 delta ≈ 4.96 |
| 6 — Brush logic regression | REFUTED | Heights mutate correctly per brush parameters |
| 7 — Multi-instance trap | REFUTED | T9.F single adapter_ptr |

**Root cause statement — dual-variant §7.7 trap at mesh layer**:

The editor has TWO separate terrain rendering abstractions in the renderer:

1. **Live path**: `Renderer::terrain_forward.chunks` HashMap. Populated by `Renderer::upload_terrain_chunk` (at astraweave-render/src/renderer.rs:6050). Drawn at runtime under `#[cfg(feature="terrain-splat-arrays")]` at renderer.rs:5755 via `if let Some(tf) = self.terrain_forward.as_ref() { for (key, chunk_gpu) in &tf.chunks { ... } }`.

2. **Legacy path**: `engine_adapter`'s `terrain_clusters` Vec + `self.models["terrain_cluster_*"]` HashMap entries. Populated by `engine_adapter::rebuild_terrain_clusters_for_chunk`. **At the current editor configuration, T9.D evidence proves nobody reads `self.models["terrain_cluster_*"]` at render time** (terrain_cluster_models=0 across all 238 samples).

The dual-variant §7.7 sub-mechanism — initial-upload and incremental-update route to DIFFERENT abstractions:

- **Initial terrain upload** (`engine_adapter::upload_terrain_chunks` at engine_adapter.rs:1421) routes through `Renderer::upload_terrain_chunk` → populates live `Renderer::terrain_forward.chunks`. Initial render shows terrain correctly.
- **Incremental brush update** (`engine_adapter::update_terrain_chunk` at engine_adapter.rs:2460) mutates aw_editor's local `self.terrain_chunks` Vec + calls `rebuild_terrain_clusters_for_chunk` → legacy cluster path. T9.D proves dead code.

So the chain post-Real-Fix.A:
- Brush apply at cursor (Mechanism 1 fixed by Real-Fix.A) ✓
- Heights mutate correctly at data layer (T9.E delta ≈ 4.96) ✓
- 794 events fire correctly (T9.B) ✓
- 1749 chunk uploads succeed at slot_map level (T9.C) ✓
- BUT: `rebuild_terrain_clusters_for_chunk` finds affected_clusters=0 every time → no rebuild → legacy cluster GPU buffers stay empty
- Meanwhile, the live abstraction (`Renderer::terrain_forward.chunks`) is never touched by `update_terrain_chunk`
- Mesh stays stale forever despite all upstream chain components working

This explains every Andrew-gate symptom from Real-Fix.A's partial-REGRESS verification:
- Brush ring visible at cursor (Mechanism 1 fixed; depth pick chain works) ✓
- Cursor follows screen click (Mechanism 1 fixed) ✓
- **Brush effects don't visibly modify terrain** ✗ — because `update_terrain_chunk` writes to a dead-code legacy abstraction; the live abstraction never sees the modification.

**§7.7 graduation — methodology gold**:

Round-5-Closure §12 entry codified §7.7 candidate at depth-target layer:
- aw_editor allocates `self.depth_view`; engine_adapter manages internal depth target inside `render_to_texture`; trap at boundary.
- Real-Fix.A `0f569d212` resolved via Option (a) — pass `Option<&TextureView>` depth attachment through `render_to_texture` + `Renderer::draw_into`. Both depth targets unified at the boundary.

Round 6 confirms §7.7 at mesh-data layer with dual-variant:
- aw_editor's engine_adapter has `self.models["terrain_cluster_*"]` + `terrain_clusters` Vec (legacy, dead code).
- astraweave_render's Renderer has `terrain_forward.chunks` HashMap (live, used at render time).
- Trap with sub-variant: initial-upload routes to live path (works); incremental-update routes to legacy dead path (broken).
- Both believed to be "the terrain mesh"; neither component is wrong about its own scope; bug is at the boundary.

**Same architectural anti-pattern; different layers; different sub-variants. Two layers in the same campaign means it's not a one-off; it's a structural pattern in the editor's architecture.** §7.7 graduates from CANDIDATE (Round 5 single-layer evidence) to CONFIRMED CANONICAL PATTERN (Round 6 multi-layer evidence).

**Suggested CLAUDE.md amendment cycle** (closure surfaces; does not pre-apply): extend Edit 2 (no-second-implementation) with a resource-identity corollary. Suggested wording (one-sentence append):

> Same rule applies to **resources** (depth targets, vertex buffers, render targets) shared across component boundaries: when component A wraps component B, verify they reference the same resource (not parallel resources of the same logical role) — Sub-phase 3 mediator brush diagnostic took six instrumentation rounds to unwind two such resource-identity traps.

Approximately 50 words; tight, action-oriented, references empirical evidence, naturally extends Edit 2's existing pattern. Future CLAUDE.md amendment cycle decides whether and when to amend; this closure surfaces only.

**Two fix candidate designs (Andrew-gate decision required)**:

**Option 1 — refactor with shared helper**:

Extract a shared helper function `upload_or_update_terrain_chunk_forward(chunk_index, vertices, indices)` that builds the splat-vertex array, filters surface triangles, builds splat maps, and calls `self.renderer.upload_terrain_chunk(key, splat_vertices, indices, splat_0, splat_1, splat_dims)`. The helper is called by both:
- `upload_terrain_chunks` (initial upload; engine_adapter.rs:1421) — replaces the existing splat-build logic at engine_adapter.rs:1551-1670 with a call to the shared helper.
- `update_terrain_chunk` (incremental update; engine_adapter.rs:2460) — replaces the call to `rebuild_terrain_clusters_for_chunk` (or augments alongside it during transition) with a call to the shared helper.

`Renderer::upload_terrain_chunk` inserts into `terrain_forward.chunks` HashMap by key, so re-uploading with the same key replaces the existing entry (incremental update semantics work without additional API changes).

Pros:
- Architecturally clean: single canonical implementation; both paths use it.
- Aligns with CLAUDE.md v0.10.1 Edit 2 (no-second-implementation): collapses two routing paths to one canonical implementation.
- Easier to maintain: future modifications to splat-build / filter / upload logic happen in one place.
- Future depth-pick-style consumers (e.g., screen-space effects, shadow culling) benefit from canonical implementation.

Cons:
- Medium fix size: requires careful refactoring of existing splat-build logic (lines 1551-1670) + integration into shared helper + caller updates at both sites.
- Pre-execution analysis needed at Real-Fix.B drafting time to verify exact splat-build logic dependencies + HashMap re-upload semantics + whether legacy cluster path can be left in place during fix.

**Option 2 — inline copy of splat-build logic**:

Copy the splat-vertex build, surface triangle filter, splat map build, and `upload_terrain_chunk` call from `upload_terrain_chunks` (engine_adapter.rs:1551-1670) into `update_terrain_chunk` (engine_adapter.rs:2460). Two parallel implementations of identical splat-build logic.

Pros:
- Smaller cross-function change: only `update_terrain_chunk` is modified; `upload_terrain_chunks` is left untouched.
- Faster to write (mechanical copy-paste).

Cons:
- **Creates the exact second-implementation anti-pattern that CLAUDE.md v0.10.1 Edit 2 forbids**. Two parallel implementations of identical splat-build / filter / upload logic in the same crate. Future modifications would have to be applied in two places; drift between the two implementations is a sibling failure mode of the anti-pattern this campaign just confirmed at multi-layer.
- Ironic given Round 5 + Round 6 evidence just confirmed Edit 2 canonically. Choosing Option 2 immediately would be a methodology-violation precedent.
- Adds maintenance burden indefinitely.

**Andrew-gate decision required**: Andrew picks Option 1, Option 2, or specifies a 4th alternative. Decision documented in chat or follow-up commit. Real-Fix.B prompt drafted in subsequent session per two-session-minimum directive.

**Dead-code observation (separate cleanup concern)**:

T9.D evidence proves the legacy cluster path is dead code at this editor's configuration:
- 238/238 samples show `terrain_cluster_models=0` in `self.models`.
- Writes occur (T9.C 1749 success entries write to `terrain_clusters` Vec).
- But nothing reads `self.models["terrain_cluster_*"]` at render time.

This is worse-than-dormant code: it actively maintains state nobody consumes. CLAUDE.md v0.10.1 Edit 3 (Integration Completeness checklist) point 1 directly applies — production caller exists for the *write*, but no production consumer exists for the *read*.

**Cleanup-A session** (separate session post-Real-Fix.B-PASS per §0.2 forward chain):
- Delete `rebuild_terrain_clusters_for_chunk` (engine_adapter.rs:1880-1920 area).
- Delete `terrain_clusters` Vec field from engine_adapter.
- Delete `self.models["terrain_cluster_*"]` insertion calls in `upload_terrain_chunks`.
- Verify cargo check + build + tests still pass post-cleanup.

This is a focused single-concern commit; reviewable on its own; trivial revert if unexpected friction. Bundling cleanup with Real-Fix.B muddles the diff narrative.

**Forward chain**:

1. **Andrew-gate fix design decision** — Andrew picks Option 1 (refactor) or Option 2 (inline) or alternative.
2. **Next session**: Real-Fix.B prompt drafted per chosen design (per two-session-minimum). Specifies internal refactoring (Option 1 — helper is private; no API surface change), file modifications (engine_adapter.rs primarily; possibly renderer.rs if any helpers exposed), cargo check + build + active_tool tests verification protocol, Andrew-gate verification protocol (full §8 protocol of Real-Fix.A re-applied).
3. **Following session**: Real-Fix.B lands as single commit.
4. **Andrew-gate-PASS verification**: brush ring visible at cursor (preserved from Real-Fix.A); cursor follows click (preserved); **terrain modifies under cursor visibly** (the new Real-Fix.B target); all 8 brush modes functional; gizmos hidden behind terrain.
5. **Cleanup-A session**: delete dead-code legacy cluster path. Single commit.
6. **Cleanup-B session**: Mediator Brush Fix `8f4668599` revert per audit §5.1. Single commit.
7. **Sub-phase 3.C closeout**: campaign doc updates marking Sub-phase 3 COMPLETE.
8. **Sub-phase 4 (Pattern A regression infrastructure) + Sub-phase 5 (RegionalArchetypePanel ActiveTool) + Mediator Removal session + Sub-phase 6 closeout** per established forward chain.

**Methodology lessons accumulated post-Round 6**:

The campaign chain has now produced six methodology lessons codified at audit-§7-or-equivalent locations:

- §7.1 (three-rounds-wrong-from-code-reading) — Round 3 audit
- §7.2 (multi-round instrument-and-narrow as canonical) — Round 3 audit; **strengthened by Round 6 evidence**
- §7.3 (pre-execution actual-code verification) — Round 3 audit
- §7.4 (audit-confidence-calibration honesty) — Round 3 audit
- §7.5 (synthesizing-artifact threshold at round-3) — Round 3 audit
- §7.6 candidate (derived-value reasoning trap) — Round-4-Closure §12; awaits future audit absorption
- **§7.7 (wrapped-component resource identity trap) — Round-5-Closure candidate; Round-6-Closure CONFIRMED CANONICAL** with multi-layer evidence

§7.2 + §7.7 are the most architecturally load-bearing lessons. Future foundational architectural campaigns inherit them as canonical methodology. The pattern: when defect localization in a layered abstraction stack proves resistant to code-reading hypotheses, multi-round instrument-and-narrow with pre-baked outcome decision trees per round produces evidence-grounded convergence in 5-6 rounds; resource-identity traps at component boundaries are the canonical anti-pattern this methodology surfaces.

**Anti-drift safeguard for future closure-drafting agents**: this closure entry is structurally similar to Round-5-Closure §12 entry (same campaign-archeology resilience, same methodology-lesson surfacing pattern, same forward-chain structure, same fix-candidate-presentation pattern with Andrew-gate decision marker). The §3.7 outcome decision tree mapped Round 6 evidence directly to Outcome 3 — no analytical correction needed (matching Round 5; protecting against derived-value reasoning trap). This further confirms the methodology pattern: pre-baked outcome decision trees in instrumentation prompts protect against derived-value reasoning traps in closure analysis. Future instrumentation prompts include outcome decision trees as a STANDARD section (no longer a candidate practice; canonical from Round 6 onward).

**Scope held**: Round-6-Closure.A session only landed campaign-doc updates (Status header + §11 entries + this §12 Round-6-Closure entry). NO production code changes. NO fixes applied. NO Mediator Brush Fix `8f4668599` revert. NO modifications to prior commits. NO Real-Fix.B prompt drafting (deferred to subsequent session per two-session-minimum directive). NO pre-deciding fix design (Andrew-gate decision required). NO dead-code legacy cluster path deletion (Cleanup-A separate session). NO CLAUDE.md amendment application (separate amendment cycle). NO ARCHITECTURE_MAP.md updates (deferred).

### 2026-05-08, Sub-phase 3 Mediator Brush Diagnostic Round 7 Closure, commit [hash]

**Round 7 instrumentation closure — captures runtime evidence DEFINITIVELY CONFIRMING Mechanism H1 (wrapped-component sibling-attribute drift; intra-component variant of §7.7 at texture-data layer) per Round 7 prompt §3.6 outcome decision tree (Outcome 1). 5 targeted eprintlns across paint/zoneblend texture-mutation chain per Round-6-Closure §7.7 canonical-pattern framing extended to texture-data layer. Captured output `brush-dbg-r7.txt` totals 220,423 lines / 6,954 BRUSH-DBG entries.**

**Outcome classification per Round 7 instrumentation prompt §3.6 outcome decision tree**: **Outcome 1** — H1 confirmed cleanly via smoking-gun chunk_index=40 paint sequence (lines 202462-203435 of captured log). 7 rounds of evidence-grounded narrowing converged on the third of three §7.7 instances in the same campaign.

**Round 7 captured signal patterns** (representative samples from 6,954 total BRUSH-DBG entries):

T10.A (paint/zoneblend brush-apply data flow; 870 entries):

```
mode tally:
  Sculpt:    333 (height-mutation; baseline)
  Flatten:   275 (height-mutation; baseline)
  Erode:     181 (height-mutation; baseline)
  Paint:      65 (texture-mutation; defect target)
  Smooth:     16 (height-mutation; baseline)
  ZoneBlend:   0 in this run (separate mechanism; not exercised by Andrew during capture)
```

Pattern: paint dispatches fire normally with `mode=Paint, modified=true, emission_will_fire=true` (T10.E paired). Brush IS reaching code; reporting changes; emission gate not blocking. Mechanisms H3 (parallel code path) + H6 (parameter issue) refuted at this layer.

T10.B (helper-entry splat data state; 1041 entries) — **smoking gun**:

```
chunk_index=40 paint sequence (10 consecutive helper invocations, lines 202462-203434):
Helper #1: hash_biome=2d3d6543c332d0f4, hash_material=94e7422be132b19f, splat_0_hash=71cd3d8b30b4adce
Helper #2: hash_biome=2d3d6543c332d0f4, hash_material=522903686df0c06d, splat_0_hash=71cd3d8b30b4adce
Helper #3: hash_biome=2d3d6543c332d0f4, hash_material=266fd10e0f3184ec, splat_0_hash=71cd3d8b30b4adce
Helper #4: hash_biome=2d3d6543c332d0f4, hash_material=989f2754cb789d61, splat_0_hash=71cd3d8b30b4adce
Helper #5: hash_biome=2d3d6543c332d0f4, hash_material=9981a48399d11cc6, splat_0_hash=71cd3d8b30b4adce
Helper #6: hash_biome=2d3d6543c332d0f4, hash_material=260496ad0b427beb, splat_0_hash=71cd3d8b30b4adce
Helper #7: hash_biome=2d3d6543c332d0f4, hash_material=a8b6c7b516192b41, splat_0_hash=71cd3d8b30b4adce
Helper #8: hash_biome=2d3d6543c332d0f4, hash_material=596c09b7ff34cc26, splat_0_hash=71cd3d8b30b4adce
Helper #9: hash_biome=2d3d6543c332d0f4, hash_material=8aa6756e2c77a6bb, splat_0_hash=71cd3d8b30b4adce
Helper #10: hash_biome=2d3d6543c332d0f4, hash_material=a8811f1e471460bf, splat_0_hash=71cd3d8b30b4adce
```

**Pattern: hash_biome INVARIANT + hash_material CHANGES + splat_0_hash INVARIANT.** This is the smoking gun. Paint IS mutating vertex attributes (hash_material varies — paint writes to `vertex.material_ids`/`vertex.material_weights`). Paint is NOT touching biome attributes (hash_biome=2d3d6543c332d0f4 invariant — paint doesn't write to `vertex.biome_weights_0/1`). Splat texture stays the same (`build_chunk_splat_maps` reads only `vertex.biome_weights_*`, so produces identical splat output every call regardless of paint changes).

T10.C (renderer upload-pre; 1041 entries; 1:1 with T10.B):

Pattern: 1041 / 1041 entries paired with T10.B helper invocations. Upload chain at helper-to-renderer boundary is clean — every helper call produces a corresponding `Renderer::upload_terrain_chunk` call. Splat content hashes match T10.B values (e.g., `splat_0_hash=71cd3d8b30b4adce` invariant for chunk_index=40 across 10 paint dispatches). No upload-side defect.

T10.D (renderer draw-frame / draw-chunk; 783 / 2349 entries):

Pattern: chunks survive to draw with valid buffers; render-side reads correctly from `terrain_forward.chunks`; `chunks_count` and `index_count` non-zero across all sampled frames. Mechanism H5 (render-side splat sampling mismatch — third inter-component §7.7 layer) refuted: defect is upstream at helper-input layer, not render layer.

T10.E (paint/zoneblend event emission; 870 entries; 1:1 with T10.A):

Pattern: paint dispatches AND emit events at expected cadence. `modified=true, emission_will_fire=true` on every paint dispatch. Mechanism H4 (paint emission gated) refuted.

**Hypothesis confirmation per Round 7 evidence**:

| Mechanism (per Round 7 prompt §3.6) | Round 7 Verdict | Evidence |
|----|----|----|
| H1 — Paint writes to wrong attribute set; helper rebuilds invariant splat from sibling attribute | **CONFIRMED — smoking gun chunk_index=40 paint sequence** | T10.B hash_biome invariant + hash_material varying + splat_0_hash invariant across 10 consecutive helper calls; paint writes `material_*`; splat builder reads `biome_weights_*` only |
| H2 — Helper rebuilds invariant splat (subsumed by H1 sub-mechanism) | n/a (subsumed) | H1's specific shape includes H2's "invariant splat from invariant source" pattern |
| H3 — Paint takes parallel code path | REFUTED | T10.A 65 paint dispatches at instrumented site; paint reaches `apply_brush_at` chain normally |
| H4 — Paint events gated | REFUTED | T10.E 870 emissions with `emission_will_fire=true`; paint emits events normally |
| H5 — Render-side splat sampling mismatch (third §7.7 inter-component layer) | REFUTED at this layer | T10.D entries show chunks survive to draw with valid buffers; render-side reads correctly; defect is upstream at helper-input layer |
| H6 — Paint mode parameter issue | REFUTED | T10.A `modified=true` on every paint dispatch; brush parameters reach apply path correctly |

**Root cause statement — wrapped-component sibling-attribute drift (intra-component variant of §7.7)**:

The defect is at the **boundary between paint's write target and splat builder's read source**, *within* the engine_adapter component (intra-component, not inter-component like Layers 1+2). Specifically:

- **`apply_brush_paint_material`** (paint's write target, terrain_integration.rs:2048): mutates `vertex.material_ids` and `vertex.material_weights`. Paint's intent: change the material identity at the brush position.
- **`build_chunk_splat_maps`** (splat builder's read source, viewport/terrain_splat_builder.rs:64-70): reads `vertex.biome_weights_0` and `vertex.biome_weights_1`. Splat builder's intent: produce splat texture maps from biome attribute data.

These two functions operate on the **same `TerrainVertex` struct**, but on **different sibling attribute fields**. The struct holds both `biome_weights_*` and `material_*` as parallel attribute sets, each treated as authoritative for its own concept (biome vs material). Paint modifies one; splat reads the other; they never converge.

This is structurally identical to the Layer 1 (depth target) and Layer 2 (mesh data) §7.7 traps:
- Layer 1: aw_editor's `self.depth_view` vs engine_adapter's internal depth target. Two resources at component boundary.
- Layer 2: engine_adapter's `terrain_clusters` Vec / `self.models["terrain_cluster_*"]` (legacy dead path) vs Renderer's `terrain_forward.chunks` HashMap (live path). Two resources at component boundary, dual-variant routing.
- Layer 3: TerrainVertex's `biome_weights_*` (splat builder's read source) vs `material_*` (paint's write target). Two attribute sets at struct boundary, sibling-attribute drift.

Same root anti-pattern: state of the same logical role (terrain material identity at chunk pixels) split across multiple representations; bug at the boundary; fix shape unifies resource identity. Different granularity scale: Layers 1+2 at component boundaries; Layer 3 at struct attribute-set boundary.

**§7.7 elevation — methodology gold at fundamental level**:

Round 5 codified §7.7 as candidate at single-layer evidence (depth-target inter-component).

Round 6 graduated §7.7 from candidate to confirmed canonical with multi-layer evidence (depth-target + mesh-data; both inter-component; mesh-data dual-variant).

Round 7 (this commit) elevates §7.7 from canonical pattern to **fundamental architectural anti-pattern** with:
- **Three-layer evidence**: depth-target (Round 5) + mesh-data (Round 6) + texture-data (Round 7).
- **New intra-component sub-variant**: sibling-attribute drift within shared struct, in addition to inter-component manifestations at component boundaries.
- **Two granularity scales confirmed**: component-boundary + struct-attribute-set.
- **Same root anti-pattern at every scale**: state of same logical role split across multiple representations; bug at the boundary; fix unifies resource identity at the boundary.

This is methodology gold at fundamental level. §7.7 transitions from "canonical pattern" to "axiomatic methodology" — fundamental check during architecture review for any future foundational architectural campaign.

**Suggested CLAUDE.md amendment cycle elevation** (closure surfaces; does not pre-apply): three-layer + multi-granularity-scale evidence may elevate the resource-identity rule from "candidate corollary appended to Edit 2" to "first-class case in Edit 2 itself, structured at multiple granularity scales." Suggested elevated wording (one possible shape, ~130 words):

> **Edit 2 (canonical) — No second implementation of a logical system**: Never build a second implementation of a logical system that already exists (rendering path, vertex format, material pipeline, scheduler, tonemap chain, scene serializer). Before adding any such system, run `rg 'struct <Name>\|trait <Name>'` workspace-wide; if a peer implementation exists, extend it or surface the conflict to the user. The Fix-27 campaign and the editor-render-divergence audit each took weeks to unwind a duplicate pipeline that was created without this check.
>
> **Edit 2 sub-rule (canonical) — Same identity at every boundary**: The same rule applies to **resources at any granularity scale** — depth targets, vertex buffers, render targets, material pipelines, sampler sets across component boundaries; sibling attribute fields within shared structs; parallel data structures within a single component. When state of the same logical role is split across multiple representations (component A vs B; struct attribute X vs Y; storage P vs Q), the bug is at the boundary. Verify identity at every boundary. Sub-phase 3 mediator brush diagnostic took seven instrumentation rounds to unwind three such resource-identity traps spanning depth targets (inter-component), mesh-data routing (inter-component, dual-variant), and attribute-set drift (intra-component sibling).

Future CLAUDE.md amendment cycle decides shape + timing; this closure surfaces only.

**Three fix candidate designs for paint (Andrew-gate decision (a) required)**:

**Option A — `apply_brush_paint_material` mirrors writes to `biome_weights_*`**:

`apply_brush_paint_material` writes to `vertex.material_ids` and `vertex.material_weights` AND additionally writes the equivalent change to `vertex.biome_weights_0` and `vertex.biome_weights_1`. Splat builder unchanged.

Pros:
- Smallest fix size: localized to paint brush implementation.
- Splat builder unchanged: minimal risk to other code paths.
- Existing biome rendering pipeline unchanged.
- Fastest path to 7/8 brush modes (with ZoneBlend separate).

Cons:
- **Doesn't unify resource identity at the §7.7 boundary**; just patches over it. Two attribute sets stay independently maintained.
- **Drift risk persists**: future changes to either side could resurrect the same trap.
- **Conflates semantics**: "paint material" semantics get encoded into "biome" attribute fields; future biome-vs-material distinctions become harder.
- **Other authoring tools may also need updating**: any future tool that writes to `material_*` would need to mirror to `biome_weights_*`; pattern not enforced architecturally.
- **Anti-pattern**: violates the spirit of CLAUDE.md Edit 2's resource-identity-at-boundaries rule (suggested elevated form). Choosing Option A immediately after Round 7's three-layer §7.7 confirmation would be a methodology-violation precedent at fundamental level.

**Option B — `build_chunk_splat_maps` reads from `material_*` instead of `biome_weights_*`**:

`build_chunk_splat_maps` switches its read source from `vertex.biome_weights_0/1` to `vertex.material_ids/material_weights`. Paint's writes flow naturally to splat. Biome generation (whatever populates `biome_weights_*` initially at terrain creation) becomes irrelevant to splat output OR needs redirection to populate `material_*` instead.

Pros:
- **Eliminates the sibling-attribute trap at the §7.7 boundary**: splat output reflects what paint writes; resource identity unified at the read-write boundary.
- **Cleaner semantic model**: splat texture = material, period. "biome" becomes the abstract concept (used for terrain generation classification); "material" becomes the concrete renderable identity.
- **Aligns with elevated Edit 2 resource-identity rule**: same identity at the boundary.
- **Future paint tools work without mirror writes**: any tool that writes to `material_*` propagates to splat; pattern enforced architecturally.

Cons:
- **Initial biome generation needs updating**: whatever populates `biome_weights_*` at terrain creation must be redirected to populate `material_*` instead (or in addition to).
- **Other consumers of `biome_weights_*` may exist**: if any other system reads `biome_weights_*` for non-splat purposes (e.g., region classification, save/load, AI biome decisions), Option B may require those to switch reading from `material_*`. Pre-execution verification at Real-Fix.C drafting time enumerates consumers.
- **Larger blast radius than Option A**: touches biome generation + possibly other consumers.

**Option C — Unify into single canonical attribute set**:

Eliminate the `biome_weights_*` vs `material_*` distinction entirely. Choose one canonical attribute set (likely `material_*` since paint already uses it; possibly rename for semantic clarity, e.g., `terrain_material_*`); eliminate the other. Update all writers (biome generation, paint, ZoneBlend if implemented) to write the canonical attribute. Splat builder reads the canonical attribute.

Pros:
- **Most architecturally clean**: single canonical attribute; no possibility of drift.
- **Fully aligns with elevated Edit 2 resource-identity rule**: one resource for the logical role of "terrain material identity at chunk pixels."
- **Most thorough §7.7 fix**: eliminates the boundary entirely, not just unifies it.
- **Best maintenance posture for Sub-phase 4+ AAA-fidelity work**: terrain material system has clean, single-source-of-truth attribute model.

Cons:
- **Largest blast radius**: touches biome generation, paint, ZoneBlend (if implemented now), splat builder, possibly save/load, region classification, AI biome decisions, terrain authoring tools.
- **Semantic decision required**: is "biome" still a meaningful concept distinct from "material" post-fix? If yes (biome = abstract classification used for generation; material = concrete renderable identity), Option C requires defining the relationship clearly without redundant attributes. If no, the entire "biome" concept may need re-examination.
- **Risk of scope expansion**: Real-Fix.C session could become a multi-system refactor instead of a focused brush-fix. May need to be split across multiple Real-Fix sessions if blast radius is large.
- **May surface additional latent defects**: walking through every consumer of either attribute set during the unification could surface other §7.7 instances that haven't yet manifested as runtime bugs.

**Andrew-gate decision (a)**: Andrew picks Option A, B, C, or specifies a 4th alternative based on his read of the codebase semantics + biome-vs-material distinction in Veilweaver's terrain authoring model. Decision documented in chat or follow-up commit. Real-Fix.C prompt drafted in subsequent session per two-session-minimum directive.

**ZoneBlend independent fix shape (Andrew-gate decision (b) required: bundled-vs-split scoping)**:

Per Andrew's runtime analysis: `BrushMode::ZoneBlend => current_h` no-op stub at `terrain_integration.rs:1915`. Match arm explicitly returns current height unchanged. ZoneBlend has placeholder logic.

This is missing-implementation, not resource-identity-trap. Different mechanism class entirely from paint's §7.7 instance. Treated as orthogonal:

- **Paint defect**: §7.7 instance at texture-data layer (intra-component sibling-attribute drift). Real-Fix.C addresses via Option A/B/C.
- **ZoneBlend defect**: missing implementation. Fix is implementing ZoneBlend's intended semantics.

Bundled-vs-split scoping decision:

**Bundled (Real-Fix.C addresses both paint and ZoneBlend)**:
- Faster path to 8/8 brush modes + Sub-phase 3 closeout.
- Single fix culminates the campaign chain.
- Risk: ZoneBlend's intended semantics may need design discussion that doesn't fit cleanly with paint's mechanical fix.

**Split (Real-Fix.C for paint; Real-Fix.D for ZoneBlend, post-PASS)**:
- Cleaner separation of concerns: paint's §7.7 fix is mechanical; ZoneBlend's missing-implementation is design-driven.
- ZoneBlend gets dedicated semantic + design treatment (what does Veilweaver mean by "zone blend"? Height gradient? Material blend? Both?).
- Risk: extra round-trip; Sub-phase 3 closeout slightly delayed.

**Andrew-gate decision (b)**: Andrew picks bundled vs split based on his read of ZoneBlend's intended semantics. If Veilweaver has a clear "zone blend" concept (e.g., blending two biome materials at a zone boundary), bundle is fine — ZoneBlend's implementation is mechanical. If ZoneBlend's semantics are unclear or need design discussion, split for design clarity.

**Discipline shift documented — single-concern session pattern canonical from Round 7 onward**:

Rounds 4-6 bundled .A (instrument) + .B (revert) into single sessions; closure happened in subsequent session. This was efficient (fewer session transitions) but produced multi-concern commits / closer coupling between instrument-and-revert.

Round 7 separates these into four single-concern sessions:
- **.A landing**: instrument only (`c4d09ee86`).
- **Closure**: analytical synthesis only (this commit; campaign doc only; no production code).
- **.B revert**: mechanical revert only (separate following session).
- **Real-Fix.C**: surgical fix only (separate session post-Andrew-gate decision).

Why the shift now:
- Closure session has more analytical content (three-layer §7.7 elevation + dual-defect framing + three fix candidates) than prior closures; deserves dedicated session.
- Andrew-gate decision happens between closure and fix; cleanly bracketed by session boundaries.
- Each session's diff is purely focused on one concern (campaign-doc updates / production-code revert / production-code fix); reviewable independently.
- Establishes pattern for future round closures that surface multi-candidate fix decisions: single-concern sessions are canonical going forward.

**Forward chain**:

1. **Round-7-Closure.A** (this commit) lands.
2. **Andrew-gate decision (a)**: paint fix design (Option A vs B vs C vs alternative).
3. **Andrew-gate decision (b)**: ZoneBlend bundled-vs-split scoping.
4. **Following session — Round-7-Instrumentation.B**: revert 5 eprintlns from `c4d09ee86`. Production code returns to post-Real-Fix.B `eaaa53433` state. Single-purpose mechanical revert.
5. **Subsequent session — Real-Fix.C prompt drafted**: per Andrew's chosen paint fix design (and ZoneBlend bundled in if (b) bundled).
6. **Following session — Real-Fix.C lands**: single commit; cargo check + build + active_tool tests + Andrew-gate verification protocol (full §8 protocol of Real-Fix.A re-applied; **8/8 brush modes** PASS criterion this time).
7. **Andrew-gate-PASS verification**: brush ring at cursor + cursor follows click + gizmos hidden + height-mutation brushes work + paint visibly changes textures + zoneblend visibly works (if bundled) + no validation errors.
8. **If ZoneBlend split**: Real-Fix.D session implements ZoneBlend per Andrew-decided semantics.
9. **If 8/8 PASS** → **Cleanup-A session**: delete dead-code legacy cluster path. Single commit. Finally proceeds.
10. **Cleanup-B session**: revert `8f4668599` per audit §5.1.
11. **Sub-phase 3.C closeout**: campaign doc updates marking Sub-phase 3 COMPLETE.
12. **Sub-phase 4 + 5 + Mediator Removal session + Sub-phase 6** per established forward chain.

**Methodology lessons accumulated post-Round 7**:

The campaign chain has now produced seven methodology lessons codified at audit-§7-or-equivalent locations:

- §7.1 (three-rounds-wrong-from-code-reading) — Round 3 audit
- §7.2 (multi-round instrument-and-narrow as canonical) — Round 3 audit; **strengthened by Round 7 evidence to axiomatic-level**
- §7.3 (pre-execution actual-code verification) — Round 3 audit
- §7.4 (audit-confidence-calibration honesty) — Round 3 audit
- §7.5 (synthesizing-artifact threshold at round-3) — Round 3 audit
- §7.6 candidate (derived-value reasoning trap) — Round-4-Closure §12; awaits future audit absorption
- **§7.7 (wrapped-component resource identity trap) — Round-7-Closure ELEVATED to fundamental architectural anti-pattern** with three-layer evidence + intra-component sub-variant + two granularity scales

§7.2 + §7.7 are the most architecturally load-bearing lessons. Future foundational architectural campaigns inherit them as **axiomatic methodology**, not just canonical pattern. The pattern: when defect localization in a layered abstraction stack proves resistant to code-reading hypotheses, multi-round instrument-and-narrow with pre-baked outcome decision trees per round produces evidence-grounded convergence in 5-7 rounds; resource-identity traps at *any* granularity boundary (component / struct attribute / storage) are the canonical anti-pattern this methodology surfaces.

**Anti-drift safeguard for future closure-drafting agents**: this closure entry is structurally similar to Round-6-Closure §12 entry (same campaign-archeology resilience, same methodology-lesson surfacing pattern, same forward-chain structure, same fix-candidate-presentation pattern with Andrew-gate decision marker). Round 7 adds: dual-defect framing (paint §7.7 + ZoneBlend independent); three fix candidates instead of two (paint's complexity warrants Option C as architecturally clean alternative); discipline shift documentation (single-concern session pattern); §7.7 elevation framing (canonical → fundamental). Future closures with multi-mechanism + multi-candidate findings inherit Round-7-Closure shape.

**Scope held**: Round-7-Closure.A session only landed campaign-doc updates (Status header + §11 entries + this §12 Round-7-Closure entry). NO production code changes. NO fixes applied. NO Round-7-Instrumentation.B revert (separate following session per discipline shift). NO Mediator Brush Fix `8f4668599` revert. NO modifications to prior commits. NO Real-Fix.C prompt drafting (deferred to subsequent session per two-session-minimum directive). NO pre-deciding fix design (Andrew-gate decisions (a) + (b) required). NO dead-code legacy cluster path deletion (Cleanup-A separate session post-8/8-PASS). NO CLAUDE.md amendment elevation application (separate amendment cycle). NO ARCHITECTURE_MAP.md updates (deferred). NO modifications to Round-5-Closure or Round-6-Closure §12 entries (predecessor closure entries preserved as-landed; the elevation chronology is deliberately archeologically preserved across the three closure entries). NO revert of Real-Fix.A `0f569d212` or Real-Fix.B `eaaa53433` (both fixes are correct for what they targeted; partial regresses are sibling defects, not regressions).

---

*End of plan*
