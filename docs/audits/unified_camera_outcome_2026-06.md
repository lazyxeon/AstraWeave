# Unified Camera — Campaign Outcome (C.0 → C.8)

| Field | Value |
|---|---|
| **Campaign** | Unified Camera |
| **Sub-phases** | C.0 (audit) → C.1 (conventions) → C.2 (crate) → C.3.A–E (renderer-side) → C.4 + C.4.B (editor producer) → C.5 (audit) → C.6 (per-target) → C.7.0–E (cinematics) → C.8 (parity harness expansion) → C.9 (this closeout) |
| **Start** | 2026-05-18 (C.0 audit doc, commit `6cfae29a1`) |
| **Close** | 2026-06-01 (C.8 `6bd7d40b4`; C.9 closeout this commit) |
| **Canonical convention reference** | `docs/current/CAMERA_CONVENTIONS.md` (established C.1) |
| **Standing regression guard** | `tools/aw_editor/tests/render_parity_harness.rs` (GPU SHA parity test + 16 C.8 matrix fixtures) |
| **Status** | Closed. One upload contract, two producers, one hardened cinematics keyframe path. Cleanup queue handed off (§5). |

This document is the durable record of the Unified Camera campaign. Like the audits and the Editor-Engine Render Parity outcome doc it mirrors (`docs/audits/editor_engine_render_parity_outcome_2026-05.md`), it is **immutable** once written: future campaigns reference it; they do not edit it. The campaign consolidated the camera system — it did not perfect it. §5's cleanup queue is the proof there is remaining work.

---

## 1. Campaign overview

### What the campaign set out to do

The C.0 audit (`docs/audits/camera_system_architecture_audit_2026-05.md`) inventoried **8 active 3D camera codepaths** plus cinematics keyframes, and found major divergence on the yaw=0 forward direction across three competing conventions (+X, −Z, −X), dual renderer upload paths (`update_camera` / `update_camera_matrices`) whose side effects had to be kept aligned by hand, an editor `OrbitCamera` storing FOV in degrees under a non-canonical field name, and **three parallel cinematics camera systems** (`astraweave_cinematics::CameraKey`, the editor's `CameraKeyframe`, and `astraweave_gameplay::cutscenes::Cue::CameraTo`).

The campaign's goal: consolidate this fragmented surface into **a single canonical upload contract with two producers and a hardened cinematics keyframe path**, protected by documented conventions and contract tests.

### What it achieved

- **`RenderView` is the sole camera-upload payload**, and **`Renderer::update_view(&RenderView)` is the sole upload entry point.** The dual `update_camera` / `update_camera_matrices` paths and the `CameraUploadPath` enum were deleted (C.3.C).
- **Two `CameraProducer` implementations**: `FreeFly` (engine free-fly, `astraweave-camera`) and `OrbitCamera` (editor, `tools/aw_editor`). Both expose a world-relative `to_render_view()` (the trait method) and a concrete camera-relative `to_render_view_camera_relative()`.
- **One canonical cinematics keyframe** (`astraweave_cinematics::CameraKey`) flowing through a single hardened path: `CameraKey → Renderer::tick_cinematics → apply_camera_key (sanitizes) → FreeFly → RenderView → update_view`. The editor's parallel `CameraKeyframe` type was retired; gameplay's `Cue::CameraTo` migrated to `look_at` + `fov_deg`.
- **Conventions documented and contract-tested**: `docs/current/CAMERA_CONVENTIONS.md` (the canonical reference, C.1) plus `astraweave-render/tests/camera_conventions.rs` (8 contract tests asserting the production camera complies).
- **A standing regression guard**: the editor-engine parity harness, expanded in C.8 with 16 RenderView/matrix-level fixtures across four families (extreme pitch, non-square aspect, large world positions, cinematics-driven).

### The sub-phases and their commits

| Sub-phase | Commit | Title | Shape |
|---|---|---|---|
| C.0 | `6cfae29a1` | Architecture audit (8 codepaths, 3 forward conventions, dual upload paths, 3 cinematics systems) | audit |
| C.1 | `dbddfd52b` | Canonical convention lockdown + contract tests + bench-mock fix | conventions + contract-test |
| C.2 | `52b9e711c` | `astraweave-camera` crate with `Projection` / `RenderView` / `CameraProducer` (no consumers yet) | type creation |
| C.3.A | `807b823f0` | `Camera`→`FreeFly` migration into `astraweave-camera`; `update_view(&RenderView)` API added alongside deprecated wrappers | migration + additive API |
| C.3.B.1 | `3d666b47f` | Editor-side caller migration to `update_view` | caller migration |
| C.3.B.2 | `00a689d06` | Engine-side caller migration (20 sites — broader than the audit's inventory) | caller migration |
| C.3.C | `326d607c1` | Deprecated path deletion + workspace-wide `Camera`→`FreeFly` rename + shim removal | structural deletion + rename |
| C.3.D | `3a3adb81e` | Renderer-side chapter documentation closeout | documentation-accuracy |
| C.3.E | `25b6d7e87` | Maintenance (sanitizer-workflow nightly runs, heightmap import refactor, noise-config tests) — campaign-numbered, peripheral to camera | maintenance |
| C.4 | `20666fb46` | `OrbitCamera` adopts `CameraProducer` + picking precision fix + docs | multi-shape composite |
| C.4.B | `550236208` | `OrbitCamera.fov`(deg) → `fovy`(rad) field rename + serde backward-compat | structural rename |
| C.5 | `3c987e4d6` | Phase 2 audit (gizmo + per-example cameras + cinematics inventory) | audit |
| C.6 | `3897ba521` | Per-target migration queue: gizmo deletion, sandbox formalization, `FreeFly::sanitize()` (C.6.F) | multi-shape composite (6 sections) |
| C.7.0 | `4c10544fb` | Cinematics consolidation preliminary audit (roll-dormancy finding) | audit |
| C.7.A | `91eda3139` | `Cue::CameraTo` → look_at migration + `CutsceneTickEvent` | per-system (gameplay evolves fields) |
| C.7.B | `f6ff7f136` | `cutscene_render_demo` tick-loop rewrite (first `tick_cinematics` production caller) | per-system (demo adopts canonical) |
| C.7.D | `17c73ae1b` | `apply_camera_key` boundary hardening + `CameraKey::sanitize()` | boundary hardening |
| C.7.C | `61da62bdb` | `CameraKeyframe` retirement + roll drop | per-system (editor type retired) |
| C.7.E | `e61037cda` | Cinematics chapter documentation closeout (cinematics.md stale banner) | documentation-accuracy |
| C.8 | `6bd7d40b4` | Parity harness expansion (first dynamic-workflow sub-phase) | adversarial-refute workflow |
| C.9 | (this commit) | Campaign closeout: outcome doc + methodology codification + cleanup-queue hand-off + saved workflow | documentation-accuracy + record-completeness |

**Commit-order note (not a numbering error):** C.7.D (`17c73ae1b`) was committed *before* C.7.C (`61da62bdb`) — the hardening landed before the editor-type retirement that depends on it. The chapter's natural order is C.7.0 / A / B / D / C / E, which is how `CAMERA_CONVENTIONS.md` §3 logs it. C.3.E is a campaign-numbered maintenance commit whose content (CI sanitizer, heightmap, noise tests) is tangential to camera consolidation; it is recorded here for completeness, not because it advanced the camera surface.

### Why phase numbers are local

References like "C.7.D before C.7.C," "P.5-refinement," or "§7.11" reflect parallel campaign bookkeeping, not a single global timeline. The Unified Camera campaign's `C.x` numbering is independent of the Editor-Engine Render Parity campaign's `P.x` numbering, even though they share a methodology lineage (§6) and the same parity harness file.

---

## 2. The canonical architecture, final state

### The upload contract

```
Producer (FreeFly | OrbitCamera)
        │  to_render_view()            (world-relative, CameraProducer trait)
        │  to_render_view_camera_relative()   (concrete-only, large-position precision)
        ▼
   RenderView { view, projection, view_proj, inverse_view, inverse_view_proj,
                position, view_dir, fovy, aspect, znear, zfar }
        │
        ▼
   Renderer::update_view(&RenderView)   ── the SOLE upload entry point
```

- `RenderView` (`astraweave-camera/src/render_view.rs`) is the minimum upload contract per `CAMERA_CONVENTIONS.md` §2.9. The renderer consumes `RenderView` exclusively — there are no per-producer-type renderer APIs.
- `Projection::perspective(fovy, aspect, znear, zfar)` (`astraweave-camera/src/projection.rs`) builds the matrix via `Mat4::perspective_rh(fovy, aspect.max(0.01), znear, zfar)` (wgpu `[0,1]` depth, §2.2/§2.6) and stores the **pre-floor** aspect in its field.
- Camera-relative rendering is the **producer's** responsibility (it pre-subtracts position from the view matrix and reports `position` separately), not encoded in `RenderView`'s layout.

### The two producers

| Producer | Crate | View construction | Forward at yaw=0 |
|---|---|---|---|
| `FreeFly` | `astraweave-camera` | `look_to_rh(position, dir(yaw,pitch), Vec3::Y)` (§2.5 direction-based) | +X (`dir(0,0) = Vec3::X`, §2.8) |
| `OrbitCamera` | `tools/aw_editor` | `look_at_rh(position(), focal_point, Vec3::Y)` (§2.5 target-based) | derived from `focal_point − position()` |

Both store `fovy` in **radians** (§2.1). `OrbitCamera` keeps a degrees boundary at its UI surface (`set_fov(degrees)`, `fov_degrees()`) per C.4.B, converting at the boundary.

### The cinematics path

```
CameraKey { t, pos, look_at, fov_deg }      (astraweave-cinematics, data-layer keyframe)
        │  Renderer::tick_cinematics(dt, &mut FreeFly)   (the only public reach)
        ▼
   apply_camera_key (private, renderer.rs)
        │  clone + CameraKey::sanitize()      (clamp fov_deg∈[10,170]; look_at==pos → pos+(1,0,0))
        │  dir = (look-pos).normalize_or_zero(); yaw = atan2(dir.z,dir.x); pitch = asin(clamp(dir.y))
        │  cam.fovy = fov_deg.to_radians()    (degrees→radians at the producer boundary)
        ▼
   FreeFly  ──► RenderView  ──► Renderer::update_view
```

`apply_camera_key` is **private**; the only public reach is `Renderer::tick_cinematics`, which mutates a caller-supplied `&mut FreeFly` (its `aspect`/`znear`/`zfar` persist through the apply). The C.0-planned "CameraKey evolved into a continuous `RenderView` evaluator" was **not** the path taken — the chapter instead *routed* cinematics through the canonical `FreeFly` producer and *hardened* the boundary (C.7 #10-row resolution). `CameraKey.fov_deg` stays degrees in the serialized `Timeline` format; the degrees→radians conversion happens at the apply boundary, which §2.5/§2.6 permit (producer-internal convention is allowed provided the `RenderView` boundary is canonical).

### Three dependency edges and two sanitize sites

- **Dependency edges established** (all → `astraweave-cinematics`, which has zero `astraweave-*` deps, so no circular risk): `astraweave-gameplay` (C.7.A), `examples/cutscene_render_demo` (C.7.B), `tools/aw_editor` (C.7.C).
- **Two sanitize sites** (the degenerate-input hardening, see Pillar 3): `FreeFly::sanitize()` (C.6.F — caller-invoked only) and `CameraKey::sanitize()` (C.7.D — caller-invoked *and* defensively invoked inside `apply_camera_key`, because that conversion is a hot path every keyframe flows through). Both harmonize the FOV clamp to `[10°, 170°]`.

---

## 3. The chapter chain

The campaign ran as five chapters plus the parity-harness expansion, each closing with a closure-proof shape matched to what it was about (Pillar 1).

### 3.1 Renderer-side consolidation (C.0 → C.3.E)

Established the conventions (C.1), created the canonical types in their own crate (C.2), migrated the engine `Camera` to `FreeFly` and added `update_view` alongside `#[deprecated]` wrappers (C.3.A), migrated every caller off the wrappers (C.3.B.1 editor-side, C.3.B.2 engine-side — 20 sites, broader than the audit's inventory), then **deleted** the deprecated paths and renamed `Camera`→`FreeFly` workspace-wide with the shim removed (C.3.C: 50 files, +145/−364). C.3.D closed the chapter with a documentation sub-phase. **Closure proofs:** contract-test (C.1), byte-equivalence via the parity harness during the migration window (C.3.A/B), and structural-deletion (C.3.C).

### 3.2 Editor-side producer migration (C.4 + C.4.B)

`OrbitCamera` adopted `CameraProducer` with the world-relative trait method and the concrete camera-relative method (mirroring `FreeFly`'s C.3.A pattern), and the picking path migrated to the precision-stable camera-relative VP inversion (the C.0 §3.2 picking-vs-depth divergence, found on inspection to be a *float-precision* issue, not a coord-space mismatch). C.4.B renamed `fov`(deg)→`fovy`(rad) with a serde shadow type accepting both legacy and canonical field names. **Closure proofs:** multi-shape composite (C.4: parity byte-equivalence + new `picking_consistency.rs` + new `orbit_camera_producer.rs`); structural-rename with behavioral preservation (C.4.B).

### 3.3 Mid-campaign audit (C.5)

A pure-audit sub-phase that inventoried the gizmo `CameraController`, four per-example bespoke cameras, and the three parallel cinematics systems, and **falsified 75% of C.0's medium-confidence pitch-clamp findings** (shadow_csm_demo, fluids_demo, nanite_demo clamps were present; only unified_showcase's was genuinely missing). **Closure proof:** audit-document-completeness.

### 3.4 Per-target migration (C.6)

Six sectioned deliverables (+320/−1869): gizmo `CameraController` + `SceneViewport` deletion (A), unified_showcase sandbox formalization (B), shadow_csm_demo / fluids_demo targeted fixes (C/D), nanite_demo dead camera-state deletion (E), and `FreeFly::sanitize()` (F). The C.5 pitch-clamp falsifications were recorded post-hoc with file:line citations. **Closure proof:** multi-shape composite (six per-section proofs verified together).

### 3.5 Cinematics consolidation (C.7.0 → C.7.E)

A preliminary audit (C.7.0) that empirically determined the **roll-dormancy** finding (reframing a planning premise from feature-preservation to feature-addition), then per-system consolidation: gameplay evolved its `Cue::CameraTo` fields to `look_at` + `fov_deg` and emitted `CutsceneTickEvent` (C.7.A); `cutscene_render_demo` became the first `tick_cinematics` production caller (C.7.B); `apply_camera_key` was hardened and `CameraKey::sanitize()` added (C.7.D); the editor's `CameraKeyframe` retired into `CameraKey` with dormant roll dropped (C.7.C); and the chapter closed with a documentation sub-phase (C.7.E) that put a **stale-chapter banner** on the drifted 523-line `cinematics.md` rather than half-correcting it. **Closure proofs:** per-system (each system consolidated in its own shape), boundary-hardening, and documentation-accuracy.

### 3.6 Parity harness expansion (C.8)

The campaign's **first dynamic-workflow sub-phase**. A workflow fanned out across four fixture families (extreme pitch, non-square aspect, large world positions, cinematics-driven), generated 17 candidate RenderView/matrix-level fixtures, and adversarially refuted each baseline before landing; 15 survived (16 test functions), 2 were refuted-and-excluded. The adversarial-refute pass caught **two escalation-class baseline errors** (see Pillar 13). **Closure proof:** the green expanded parity suite, every landed fixture having survived adversarial refutation.

---

## 4. The parity contract

`tools/aw_editor/tests/render_parity_harness.rs` is the campaign's standing regression guard. It now carries two kinds of contract:

1. **The pre-existing GPU SHA parity test** (`editor_engine_render_parity`) — inherited from the Editor-Engine Render Parity campaign. It renders the editor path and the engine path and asserts SHA-256-identical LDR bytes, per-machine. This protects the *rendered-output* parity contract (loader / tonemap / quality preset / target format / overlay isolation).

2. **The 16 C.8 RenderView/matrix-level fixtures** across four families. These derive every baseline from camera math (`look_to_rh` / `look_at_rh` / `perspective_rh`, the `dir(yaw,pitch)` spherical formula, and f64 high-precision references) rather than from a SHA of GPU output — because a SHA of GPU output cannot be independently derived, which the campaign's anti-fabrication discipline forbids. They protect the *camera-math* contract: extreme-pitch behavior (FreeFly stays finite at the exact f32 ±π/2 because `cos(FRAC_PI_2 as f32) ≈ −4.37e-8`; OrbitCamera's target-based path can degenerate), the non-square-aspect floor discipline (matrix floors at `.max(0.01)`, field stores raw), large-world precision (world-relative vs camera-relative variants diverge *by design* — tested as expected-divergence, never byte-equivalence), and the cinematics path (GPU-gated, comparing `tick_cinematics` output against an independent `look_at_rh` baseline).

**What reopening looks like:** a SHA mismatch in the GPU test means one of the five render seams reopened (consult the parity outcome doc). A failure in a matrix fixture means a producer's camera math drifted from its derived baseline — the fixture's assertion message names the specific quantity (matrix entry, NDC error, clamped fov, translation magnitude) that diverged.

---

## 5. Post-campaign cleanup queue

These items were deferred during the campaign. This is a **record**, not work C.9 performs — C.9 executes none of them. Each is a legible hand-off with a pointer to where it is tracked.

### 5.1 C.7.F — `cinematics.md` coherent rewrite + aspirational-API-surface audit

**What:** The 523-line `docs/src/core-systems/cinematics.md` chapter (predating the C.7 chapter) teaches a wrong `rot`-based Euler camera model rather than the canonical look-at-target model, and references possibly-nonexistent APIs (`add_camera_track`, `Track::camera`, `SeqError::Range`, `set_position` / `set_rotation` / `set_fov`, etc.).

**Why deferred:** C.7.E's Phase 4.5.A entanglement assessment found ≥6 of the ~11–14 wrong-canonical-type references entangled in aspirational-API examples; half-correcting would make the chapter internally inconsistent (Pillar 10). C.7.E disclaimed it wholesale with a stale-banner pointing to the canonical surfaces.

**Pointer:** the banner in `cinematics.md` itself; `CAMERA_CONVENTIONS.md` §3 (C.7.E log). This is rewrite-shaped + audit-shaped work deserving its own planning round.

### 5.2 L.7.4 — editor `CinematicsPanel` runtime-flow wiring

**What:** The editor's `CinematicsPanel` is UI-state-only; wiring it to a renderer preview (so editing a keyframe updates a live viewport) is a feature addition.

**Why deferred:** Per the C.7 planning round's Q8 — it is feature work, not consolidation, and consolidation was the chapter's mandate.

**Pointer:** `CAMERA_CONVENTIONS.md` §3 (C.7.0 latent-issue list, L.7.4).

### 5.3 L.7.5 — `astraweave-ui` dev-only Simple Cinematics panel

**What:** The dev-only Simple Cinematics panel in `astraweave-ui` already uses canonical types; its remaining work is tool-integration, not type consolidation.

**Why deferred:** Documented in C.7.E as a *deliberate* deferral (a parallel tool, not a parallel type). Per Q8.

**Pointer:** `CAMERA_CONVENTIONS.md` §3 (C.7.E deferred-panel note, L.7.5).

### 5.4 The standalone follow-up pile

- **C.8 cinematics-family GPU gating** — the four cinematics fixtures are GPU-gated because `apply_camera_key` is private and `tick_cinematics` (its only public reach) needs a live `Renderer`. A GPU-free CPU anchor would require exposing a `CameraKey → FreeFly` conversion on `astraweave-render`'s public surface, which is out of C.8 scope (it would modify producer/renderer source). Recorded as the cinematics family's one structural limitation; the fixtures fully cover the family hazard whenever a GPU is present.
- **C.8 verifier-reliability hardening as standing practice** — the C.8 re-verification established (directionally, §6 Pillar 13) that the retry + minimal-fallback skeptic protocol is load-bearing under concurrency. Carried forward as the recommended default for any future adversarial-refute workflow.
- Any items still open from before this segment that the campaign touched remain tracked in their originating docs (`CAMERA_CONVENTIONS.md` §3 migration table — all §3 rows closed as of C.7; the parity outcome doc's §4 cleanup queue for the rendered-output seams).

---

## 6. Methodology pillars

The campaign accreted 13 methodology pillars. This section is the camera campaign's **inline empirical reference** for them — mirroring the Editor-Engine Render Parity outcome doc's §5, which kept its pillars inline and deferred the consolidated cross-campaign `§7.11` surface to a later boundary. Per the C.9 Phase 1 inventory, no materialized cross-campaign methodology doc exists (`§7.11` is referenced but never defined; `docs/src/performance/methodology.md` is the unrelated benchmark-methodology doc), and the precedent keeps methodology inline *even though its own pillars are cross-campaign*. C.9 matches that pattern: pillars inline here, cross-campaign kinship marked, and the consolidated `§7.11` cross-campaign methodology surface left to the **E-closeout** boundary the prior campaigns repeatedly named ("C.9 / E-closeout"). Materializing it now would mean duplicating or referencing the *immutable* parity outcome doc's pillars into a half-populated new surface — a bigger and lower-value move than matching the precedent.

Reinforcement counts are stated as accreted, not inflated (Pillar 4 applied to this codification): where a pillar has two genuine instances, it says two.

### Pillar 1 — Measurement-instrument-matched-to-seam (Pillar 5-refinement)

**Principle:** the closure proof's *shape* must match the *seam type* being closed — byte-equivalence for byte-level seams, structural-deletion for removals, contract-test for trait conformance, parameter-equality for state, documentation-accuracy for doc seams, expected-divergence for precision-mitigation paths.

**Instances:** the campaign's spine, reinforced across every closure-proof shape it produced. This continues the Editor-Engine Render Parity campaign's §5.3 Pillar-5-refinement (5 demonstrations there, P.2–P.6); the running tally reached the **"twelfth"** reinforcement explicitly logged at C.4 (`CAMERA_CONVENTIONS.md` §3) and continued through C.8's expected-divergence fixtures — ~17 reinforcements across the two campaigns. **Cross-campaign:** this *is* the parity campaign's pillar; see `editor_engine_render_parity_outcome_2026-05.md` §5.3. It is the parent of which most pillars below are specializations.

### Pillar 2 — Audit-shaped sub-phase

**Principle:** when the unknown is "what is the current state," the right sub-phase is an audit that inventories before any change.

**Instances:** three — C.0 (`6cfae29a1`, pre-campaign), C.5 (`3c987e4d6`, mid-campaign), C.7.0 (`4c10544fb`, chapter-internal).

### Pillar 3 — Sanitize pattern with per-type invocation model

**Principle:** degenerate-input hardening is a canonical *pattern* (a `sanitize()` clamping to valid state), but the *invocation model* is per-type, determined by where degenerate inputs originate.

**Instances:** two — `FreeFly::sanitize()` (C.6.F, in `3897ba521`): caller-invoked only, because its inputs come from external sources. `CameraKey::sanitize()` (C.7.D, `17c73ae1b`): caller-invoked *and* defensive-at-renderer-boundary, because `apply_camera_key` is a hot path every keyframe flows through.

### Pillar 4 — Falsification discipline

**Principle:** medium-confidence findings inferred from absence-at-one-site must be grep-verified across all sites before being recorded as true.

**Instances:** three, the third reflexive — C.5 (`3c987e4d6`, 75% false-positive on pitch-clamps), C.7.0 (`4c10544fb`, roll-dormancy verifying a planning premise), and C.7.E (`e61037cda`, the §3 #10-row recording what *actually* resolved a seam rather than what the plan *predicted* — falsification turned on its own plan).

### Pillar 5 — Audit-miss-via-cargo-check

**Principle:** audits inventory by topic/grep-pattern and are not exhaustive caller inventories; the complete caller inventory is `cargo check --workspace` (for type changes) or targeted grep (for deletions). Absorb the missed site when it fits the decided pattern; escalate when it doesn't.

**Instances:** three — C.3.B.2 (`00a689d06`, 7 engine-internal test/example sites the audit didn't grep), C.7.A (`91eda3139`, `quest_dialogue_demo`), C.7.C (`61da62bdb`, `ClipData::Camera`).

### Pillar 6 — Intentional-bridge lifecycle

**Principle:** transitional code that is functional, intentional, and single-sub-phase-lived is distinct from deprecated-wrappers (multi-phase, dual-test) and from dead-code (non-functional, awaiting deletion), and is marked with an explicit transitional comment so its short life is legible.

**Instances:** one full lifecycle — C.7.A (`91eda3139`) introduced the `cutscene_render_demo` bridge; C.7.B (`f6ff7f136`) retired it.

### Pillar 7 — Honest non-consolidation (dual-Timeline)

**Principle:** sometimes the correct move is *not* to consolidate. When two systems carry genuinely different concerns, keep them parallel with an explicit synchronization constraint rather than forcing a false merge.

**Instance:** C.7.B (`f6ff7f136`) — gameplay's cue-script `Timeline` (intentions) and cinematics' keyframe `Timeline` (states) were kept as parallel state machines with a duration-matching constraint, not merged. *Kin to Pillars 8 and 10 — see kinship note.*

### Pillar 8 — Per-system consolidation strategy

**Principle:** when consolidating N parallel systems, the strategy is per-system, fit to each system's role, not uniform.

**Instance:** the C.7 chapter consolidated three parallel cinematics systems three different ways — gameplay evolved its fields (C.7.A `91eda3139`), the demo adopted the canonical path (C.7.B `f6ff7f136`), the editor's parallel type retired entirely (C.7.C `61da62bdb`), and the dev-only `astraweave-ui` panel was deliberately deferred. *Kin to Pillars 7 and 10.*

### Pillar 9 — Documentation-accuracy closure as chapter-closing discipline

**Principle:** each major consolidation chapter closes with a documentation sub-phase that updates contributor-facing surfaces to the post-chapter state.

**Instances:** two — C.3.D (`3a3adb81e`, renderer-side), C.7.E (`e61037cda`, cinematics). C.9 (this document) is the third instance of the *closure-proof shape* (documentation-accuracy), applied at campaign scope rather than chapter scope.

### Pillar 10 — Banner-disclaim-over-half-correct

**Principle:** when documentation is coherently *wrong* (built on a wrong mental model, not merely wrong syntax), partial correction makes it *worse* — internally inconsistent — than a wholesale disclaimer pointing to canonical sources.

**Instance:** C.7.E (`e61037cda`) — `cinematics.md`'s `rot`-based model could not be half-fixed into the `look_at` model without inventing data, so the whole chapter got a stale-banner + deferral (C.7.F) instead of a partial patch. *Kin to Pillars 7 and 8.*

### Pillar 11 — Multi-shape composite closure

**Principle:** a sub-phase touching multiple concerns proves each concern with its own shape, verified together.

**Instances:** C.4 (`20666fb46`, 3 concerns: producer adoption + picking fix + docs), C.6 (`3897ba521`, 6 sectioned deliverables), and the C.7.x sub-phases.

### Pillar 12 — Conditional follow-on from in-flight escalation

**Principle:** when execution surfaces a problem the planning rounds didn't anticipate, the discipline is escalate-defer-flag: stop, surface it, defer cleanly to a new sub-phase or the cleanup queue, and flag it in the relevant surface — without scope-creeping the current sub-phase.

**Instance:** C.7.F was born this way from C.7.E's (`e61037cda`) Phase 1 discovery of the drifted `cinematics.md` chapter.

### Pillar 13 — Adversarial-refute as escalation substitute (with verifier reliability as a first-class property)

**Principle:** when a dynamic workflow removes the mid-run human escalation checkpoint, the adversarial-refutation pass *is* the structural replacement for that checkpoint — and the verifier's *reliability* therefore becomes a first-class property (a lossy verifier is a lossy escalation substitute).

**Instance:** C.8 (`6bd7d40b4`). The refute pass caught **two escalation-class baseline errors** with no human in the loop: a fixture asserting an idealized NaN the f32 producer does not actually produce (a falsified premise — `cos(FRAC_PI_2 as f32) ≈ −4.37e-8`, so the producer view is finite), and a fixture whose stated baseline had a sign-flipped right-vector (a self-contradictory derivation). **Honest reliability caveat:** the first refute run had a ~14% skeptic-null-rate; a hardened re-verification (retry + minimal-fallback skeptic protocol) found it **transient** — but this finding is **directional, not controlled** (established on a single campaign, not a controlled A/B). The retry + minimal-fallback hardening is **load-bearing insurance, not decoration**: it is what makes the verifier reliable under concurrency load.

### Kinship note (mandatory — completeness without dilution)

**Pillars 7, 8, and 10 are three expressions of a single underlying principle: the correct move is determined by the problem's structure, not by a uniform rule.** Non-consolidation (7) when concerns genuinely differ; per-system strategy (8) when parallel systems have different roles; disclaim-over-half-correct (10) when wrongness is coherent rather than superficial. Each is a distinct, citable discipline, but they descend from the same root — *resist the reflex to apply one uniform move (consolidate everything / correct everything) when the problem's structure calls for a fitted response.*

Drawing this line is itself an instance of **Pillar 1** (measurement-instrument-matched-to-seam) applied to methodology: matching the codification's shape to the relationship's actual structure, rather than flattening 13 pillars into a list where the weak borrow credibility from the strong.

A second kinship runs through the verification pillars: **Pillars 2, 4, and 5** are the campaign's epistemic discipline — audit before changing (2), verify before recording (4), and trust the compiler over the inventory (5). And **Pillar 13** is Pillar 4's discipline (verify before recording) transposed into a workflow where the verifier is itself an adversarial agent rather than a grep.

---

## 7. Forward chain after C.9

### The campaign is closed

After C.9 commits, the Unified Camera campaign is complete. The canonical architecture (§2) is in place and contract-tested; the parity harness (§4) guards it; the cleanup queue (§5) is the legible hand-off; the methodology pillars (§6) are codified for future campaigns to inherit; and the C.8 dynamic workflow is saved as a reusable command (`.claude/workflows/`, see the C.9 commit body).

### Terrain Asset Quality resumes

The **Terrain Asset Quality** campaign resumes at A.5 (doc reconciliation), now able to reference both this outcome doc and the Editor-Engine Render Parity outcome doc as architectural foundations in place, then proceeds to A.6+ per-biome wire-ups against the doubly-verified foundation. It inherits the codified methodology — the audit-shaped sub-phase (Pillar 2), measurement-matched-to-seam (Pillar 1), and, where a task has genuine fan-out, the adversarial-refute workflow pattern (Pillar 13), now a saved command.

### The §7.11 cross-campaign methodology surface

Consolidating the cross-campaign pillars (Pillar 1 spans this campaign and the parity campaign; Pillars 3/5/13 are general) into a single materialized `§7.11` methodology document remains deferred to the **E-closeout** boundary — the boundary the prior campaigns named and the natural owner of a surface that must draw on multiple campaigns' immutable outcome docs. This document is the camera campaign's empirical contribution to that eventual consolidation.

---

## Appendix A — Sub-phase commit summary

| Sub-phase | Commit | Files | Insertions | Deletions |
|---|---|---|---|---|
| C.0 | `6cfae29a1` | 1 | 548 | 0 |
| C.1 | `dbddfd52b` | 3 | 515 | 1 |
| C.2 | `52b9e711c` | 10 | 597 | 0 |
| C.3.A | `807b823f0` | 9 | 975 | 604 |
| C.3.B.1 | `3d666b47f` | 4 | 136 | 64 |
| C.3.B.2 | `00a689d06` | 17 | 54 | 35 |
| C.3.C | `326d607c1` | 50 | 145 | 364 |
| C.3.D | `3a3adb81e` | 5 | 200 | 35 |
| C.3.E | `25b6d7e87` | 5 | 20 | 5 |
| C.4 | `20666fb46` | 8 | 507 | 44 |
| C.4.B | `550236208` | 4 | 401 | 36 |
| C.5 | `3c987e4d6` | 2 | 944 | 1 |
| C.6 | `3897ba521` | 13 | 320 | 1869 |
| C.7.0 | `4c10544fb` | 2 | 719 | 1 |
| C.7.A | `91eda3139` | 5 | 290 | 112 |
| C.7.B | `f6ff7f136` | 4 | 141 | 48 |
| C.7.D | `17c73ae1b` | 5 | 148 | 33 |
| C.7.C | `61da62bdb` | 4 | 108 | 106 |
| C.7.E | `e61037cda` | 5 | 213 | 3 |
| C.8 | `6bd7d40b4` | 1 | 1341 | 1 |
| C.9 | (this commit) | (docs + saved workflow) | (see commit body) | — |

Commits listed in git-log order (C.7.D precedes C.7.C, as committed). C.3.E is a campaign-numbered maintenance commit peripheral to the camera surface, recorded for completeness.

---

*Unified Camera campaign — closed 2026-06-01. This document is immutable; future campaigns reference it.*
