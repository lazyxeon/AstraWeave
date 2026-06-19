# Fluids Integration Campaign — Plan of Record (Path B)

**Status**: ACTIVE — F.0 audit, F.1 (+hotfixes F.1.1–F.1.4), F.2, and F.3 complete; F.2 budget gate ratified 2026-06-12 (Option A floor / Option C target); F.3 (voxel integration) ready for review; F.3.S (sparsity + budget conversion) next.
**Canonical location**: `docs/campaigns/fluids-integration/CAMPAIGN_PLAN.md`
**Basis**: `F0_GROUND_TRUTH_AUDIT.md` (commit `8e1505dd8`), `F1_EXECUTION_REPORT.md` v1.4, `F2_EXECUTION_REPORT.md` v1.0
**Path decision**: **Path B — Layered facade** (deterministic CPU water core, pluggable backends, SPH demoted to visual backend)

> This file formalizes gate decisions that were carried inline through the F.1/F.2 briefs because this document did not yet exist. It is now the single source of truth; future phases cite it rather than reconstructing decisions from report text.

---

## Gate Decisions of Record

### F.0 path gate (2026-06-11)

| # | Question | Decision |
|---|---|---|
| 1 | Determinism carve-out | **ACCEPTED.** Particle fluid state is non-deterministic, presentation-only, permanently excluded from WorldSnapshot, world_hash, replay, replication. Gameplay water truth lives on a deterministic CPU layer. Policy documented in `fluids.md` and `net.md`; enforced at review. |
| 2 | T3 (Enshrouded-class voxel water) | **ASPIRATIONAL, Path B chosen.** Voxel backend ships bounded (F.3); world-scale work (sparsity, streaming, carve re-sim at scale) is out of campaign scope, reserved for a future campaign. |
| 3 | Solver consolidation | **GRANTED.** Delete vapor; gate dormant-real behind `experimental`. (Executed in F.1 WI-4.) |
| 4 | Glue placement | **New thin crate `astraweave-water`.** Fluids stays a Cargo leaf. (Executed in F.2 — `astraweave-water` is glam-only; `physics → water`.) |
| 5 | Render dependency | **Direct dep** `astraweave-render → astraweave-fluids` for V1, with a documented exit (split into a pass-crate if build times complain). Applies at F.4. |
| 6 | Editor V1 scope | **Minimal.** Volume placement + basic params, all mutations through `EditorCommand`; `ConfigHistory` demoted. Applies at F.5. |
| 7 | Frame budget | Ratified at the F.2 gate with real data — see below. |
| 8 | Physics water reconciliation | **GRANTED.** (Executed in F.2 — three abstractions retired onto `WaterQuery`; `reset_forces` resolved via impulse semantics.) |

### F.2 budget gate (2026-06-12) — Q7 ratified

**Decision: Option A as the ratified floor; Option C as the declared target, gated on F.3 sparsity benchmarks.**

- **RATIFIED NOW (the envelope F.4 builds against):** **Option A — iteration-capped GPU.** ~3.0 ms GPU for the particle path, PBD iterations capped at 3–4 (not 8), particle ceiling ~15–20k on min-spec class (GTX 1660 Ti Max-Q). Analytic `WaterQuery` layer reserved at ~0.05 ms (measured microsecond-class — does not contend). This is the honest, measured budget F.4's SSFR work must fit inside today.
- **DECLARED TARGET (the envelope the campaign converges to):** **Option C — voxel-first gameplay water**, GPU particles demoted to a ~1.5 ms detail layer (≤10k). CPU voxel gameplay water at ~1 ms.
- **CONVERSION CONDITION:** the campaign re-ratifies from A to C **only when F.3 produces real sparse-voxel benchmarks on min-spec class hardware demonstrating the ~1 ms voxel budget is achievable.** F.1 measured 64³ *dense* at 13.8 ms/tick; the 1 ms figure is unproven until active-cell sparsity exists and is measured. **No budget is ratified against unmeasured performance** — this is the campaign's standing evidence rule applied to its own planning.
- **Rationale of record:** the owner's appetite favored committing to C outright; campaign discipline (do not ratify against performance not yet measured) prevailed. A gives F.4 a number it can trust now; C gives F.3 a number it must *earn*. The budget converges to the T3 ambition exactly as fast as the evidence does.

---

## Standing Red Lines (cite at every review)

1. No particle state in WorldSnapshot, world_hash, replay, or net replication — ever.
2. Fluids stays a Cargo leaf; the cycle `physics → fluids → terrain → gameplay → physics` must not close. Glue lives in `astraweave-water` (a leaf below physics). *(F.2 verified acyclic: `physics → water → glam`.)*
3. No second implementation: `astraweave-water` / `WaterQuery` is the single logical owner of gameplay water truth. Anything adding a parallel water abstraction is rejected. *(F.2 retired the three prior abstractions onto it.)*
4. Prove it, don't hype it: no solver/budget claim ships without a recorded baseline + machine context. **Built is not run; run is not seen; counted is not rendered; measured-elsewhere is not measured-here.** *(The F.1 hotfix-chain lesson, now law.)*
5. Every public API element ships with a real caller, or it does not ship. *(The F.2 caller:method discipline, now law.)*

---

## Roadmap (Path B)

| Sub-phase | Scope | Status |
|---|---|---|
| **F.0** | Read-only ground-truth audit, path options | ✅ complete |
| **F.1** (+F.1.1–F.1.4) | FluidSystem correctness repair, solver consolidation, first GPU tests + baselines, demo runtime defects, capture infra | ✅ complete, merged (PR #192) |
| **F.2** | `astraweave-water` facade + `WaterQuery` (derived from physics need); buoyancy/drag reconciled onto it; determinism proof; budget ratification | ✅ complete, branch `campaign/fluids-f2` ready for review |
| **F.3** | Voxel backend behind `WaterQuery` (integration scope): `WaterVolumeGrid: WaterQuery`, gate-flag fix (Must-Fix #6), conservation/hydrostatic/U-bend/dt-stability tests, terrain heightmap glue (cycle-safe `&[f32]`), bounded carve reactivity, determinism proof. | ✅ complete, branch `campaign/fluids-f3` ready for review |
| **F.3.S** | **Sparsity + budget-conversion gate** (carved out of F.3): implement active-cell sparsity (the grid is dense today); produce min-spec benchmarks. **Starting point: F.1's 64³ dense = 13.8 ms/tick. The A→C budget conversion happens ONLY when F.3.S demonstrates the ~1 ms sparse voxel budget on min-spec class hardware.** Conversion-cost benchmarks. | next |
| **F.4** | Rendering: SSFR `draw_into` integration (within the **ratified Option-A envelope** unless F.3.S has converted to C), per-particle color, ocean converge-vs-extend decision, XSPH-viscosity exposure. F.1 ledger. | queued |
| **F.5** | Editor: minimal FluidsPanel (PREREQ: Editor Multi-Tool SP5 landed) | queued |
| **F.6** | Persistence of authoring data + determinism policy closeout + campaign-close methodology elevation | queued |

## Carried Open Items

- **Character-controller ground-tolerance defect** (`character_controller_stays_on_ground`, `y≈0.1` vs ~0): pre-existing, F.2-unrelated, **but in the same controller F.2-followup's swim branch will modify** — tracked separately so it does not ambush swim work.
- **`llm_integration` stale import** (`DEFAULT_QWEN_INSTRUCT_MODEL` path): pre-existing, trivial, fix when next in that crate. (Still failing `cargo check --workspace` as of F.3; unrelated to fluids — zero edges to water/fluids.)
- **Swim** (`CharState::Swimming` + controller branch): F.2-followup / F.3-adjacent; `WaterQuery::sample` already covers its needs; do not start until the ground-tolerance defect above is resolved. **This is the most likely source of the second `WaterQuery` consumer that triggers the deferred resource promotion.**
- **ECS water-resource promotion**: co-located in `PhysicsWorld`; F.3 confirmed NO second consumer exists yet (only physics buoyancy calls `sample`), so co-location correctly retained. Promote to standalone resource when a real second consumer (AI/gameplay/swim) appears — decided on call-graph evidence, never on prediction.
- **Voxel backend production-install owes an end-to-end exercise** (F.3 deviation D2): the voxel backend is contract-proven through the `WaterQuery` trait (tests) but is NOT yet installed in any production consumer — physics holds a concrete `AnalyticWater`, and a voxel install needs polymorphic physics water (no driver today). When that install happens, it owes a full end-to-end run under real conditions, not just green trait tests — per Red Line 4, "run is not seen." The gap between trait-passing and production-running is exactly where this campaign's defects have historically hidden.
- **Proportional gate throttle** (F.3 deviation D1): the voxel gate is binary-real (block/pass); `WaterGate::flow_multiplier()` computes a 0..1 throttle but `WaterCell` has no per-cell multiplier field to carry it. Adding one is a sim-data-model change deferred to a future phase.
- **F.1 verification-gate blind spot** (F.3 deviation D3, minor): test-code clippy under specific feature combos wasn't linted by F.1's `--all-features` gate (four pre-existing errors surfaced and fixed in F.3). If F.4/F.5 touches `astraweave-fluids`, widen the clippy invocation as cheap insurance.