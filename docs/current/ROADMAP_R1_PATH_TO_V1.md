# R.1 — Roadmap: Sequenced Path to v1.0 (RATIFIED)

> **Campaign**: R-series (Engine Roadmap to v1.0.0) · **Phase**: R.1 (the roadmap itself) · **Branch**: `campaign/roadmap`
> **Mode**: the **ratified plan** — the director reviewed the strawman and set the sequence (§6). This is no longer a strawman; the next phase executes M1.
> **Input**: the verified state map `docs/current/ROADMAP_R0B_STATE_MAP.md` (post-v1.1-split) and its §7 v1.0 critical-path gap list. This roadmap **consumes** the verified present; it does not re-derive it.
> **Status**: RATIFIED (2026-06-30). The sequence and scope are set. The next beat is **M1 execution** (its own bounded-recon → gate → build).
> **Date**: 2026-06-30

---

## 0. How to read this

This roadmap sequences the closure of the v1.0 gaps into three milestones. The ordering was a strawman the director reviewed and ratified (§6): the **difficulty ladder**, with **the physics defect pulled into M1** (highest propagation risk) and **networking ruled out of v1.0** (deferred to v1.1). Each milestone moves named crates toward VERIFIED-PRODUCTION and meets named E/A criteria; §7 is the done-state checklist. The rationale for the chosen sequence is recorded in §6.

---

## 1. The v1.0 definition (locked)

**v1.0 = "a clean and verified engine with an editor a game can be authored on."** A conjunction of two halves — both must be true:

- **Clean/verified half:** every crate on the v1.0 critical path is at **VERIFIED-PRODUCTION** — no `FALSE-PRODUCTION-READY`, nothing orphaned-on-the-critical-path, no `PRODUCTION-CAPABLE-FAILING-TESTS` (the failing tests fixed, the hollow crates wired-or-relabeled).
- **Authorable-editor + stable-AI half:** the **E1–E7** editor criteria and **A1–A6** AI-pipeline criteria (R.0.A §3) are met.

**Networking is explicitly NOT a v1.0 criterion (ratified):** real multiplayer wire-up is **v1.1 scope**. The `net`/`net-ecs`/aw-net-trio cluster drops off the v1.0 critical path; its only v1.0 work is the cheap **trace-honesty relabel** (§5.1, M1). This is a *recorded deferral*, not a dropped concern — see §5.3.

The roadmap's **done state** is both halves of the conjunction: the verdict board green on the (networking-excluded) critical path **and** the E1–E7 / A1–A6 criteria met.

---

## 2. Starting position — the verified present (R.0.B)

**The honest framing: the AI pillar is largely done; the editor pillar is the real work.** The audit found the foundation and the **wired deterministic AI path** (`RuleOrchestrator` + `GoapOrchestrator`) at VERIFIED-PRODUCTION; the v1.0 gaps cluster in the editor.

> **"AI pillar SOLID" — what it means (and does not).** It means the **wired deterministic AI path** (`RuleOrchestrator` + `GoapOrchestrator`) is VERIFIED-PRODUCTION and is what v1.0's "stable AI pipeline" rests on. It does **NOT** mean the full AI pipeline including the LLM ships in v1.0 — the hybrid LLM `AIArbiter` and its dormant ~15K-LoC hardening surface are **consciously post-v1.0** (M1.3 A2 recon, `992793c41`). A1 (core loop), A4 (determinism), A6 (stability) hold for the wired deterministic path; A2 is the determinism-consolidation test; A5 is the honesty constraint (§2.1). The AI pillar is production *for what v1.0 requires* (deterministic planning) — the LLM capability is **deferred, not shipped**.

### 2.1 Already green (record, don't re-do)

| Criterion | Crates | Status |
|---|---|---|
| **A1** core loop | `ai` ✅VP | **MET** |
| **A4** determinism | `ecs`, `core` ✅VP | **MET** |
| **A5** LLM honesty *(capability deferred, honesty required)* | `llm` ✅VP (crate); LLM *path* dormant/post-v1.0 | **HONESTY-MET when no v1.0 doc overclaims the LLM as production.** The LLM capability defers post-v1.0 with the arbiter; A5 requires only that docs/traces not claim a wired LLM path the engine lacks. The `phi3:medium`-vs-Qwen3 doc-drift folds in: don't document a dormant LLM path as live. |
| **A6** stability bar | `ai`, `behavior`, `llm`, `ecs`, `core` all ✅VP | **MET** for the wired deterministic path — zero island/stub/hollow on it (the LLM arbiter is consciously post-v1.0, not a regression). |
| **E6** camera | `camera` ✅VP, `cinematics` ✅VP | **MET** |
| Foundation gate | `core`, `ecs` ✅VP; `sdk` ✅VP (R3 boundary) | **MET** |

So the roadmap **starts with the AI pillar and the foundation largely closed.** What remains is one AI verification task (**A2**), one AI measurement task (**A3**), and the editor pillar (**E1–E5, E7** + the wired/relabel work).

### 2.2 The critical-path verdict board (what's not yet green)

| Criterion | Crate(s) | Current verdict | Target | Milestone |
|---|---|---|---|---|
| E1 scene authoring | `aw_editor` | PRODUCTION-CAPABLE-FAILING-TESTS (9323/1) | VP | **M1** |
| E2 WYSIWYG parity | `render` | PRODUCTION-CAPABLE-FAILING-TESTS (1277/2) | VP | **M1** |
| (asset import) | `blend` | PRODUCTION-CAPABLE-FAILING-TESTS (52/1) | VP | **M1** |
| (physics defect) | `physics` | PRODUCTION-CAPABLE-FAILING-TESTS (1693/1) | VP | **M1** (pulled forward — Rapier-integration fix, §5.1) |
| A2 wired-path determinism | `ai`/`RuleOrchestrator`+`GoapOrchestrator` (✅VP) | determinism test not yet written | A2 determinism test green | **M1** |
| E3 terrain/material | `terrain` | PRODUCTION-CAPABLE-FAILING-TESTS (936/**8**) | VP | **M2** |
| E4 asset cook path | `asset-pipeline` | FALSE-PRODUCTION-READY (hollow) | VP-or-relabel | **M2** (lead capability item) |
| E7 save/load | `persistence-ecs` | COMPILES-BUT-ORPHANED (auto_save/replay stubs) | VP / live path | **M3** |
| E5 play-in-editor | `aw_editor`↔AI seam | not yet a single crate | seam defined + wired | **M3** |
| (input) | `input` | FALSE-PRODUCTION-READY (hollow) | VP-or-relabel | **M3** |
| A3 throughput | `ai` tool_sandbox (✅VP) | bar not yet measured | measure → set bar | **M3** |
| (networking — **v1.1, off-path**) | `net` (+ cluster) | FALSE-PRODUCTION-READY (hollow) | **relabel trace → example-only** | **M1** trace track |

**Off the critical path (record, do not chase for v1.0):** `math` (COMPILES-BUT-ORPHANED — only `profiling_demo` consumes it; confirmed off-path); `alloc` (WIRED-BUT-UNTESTED, 0 tests — foundation allocator; a minor "add coverage to reach VP" item, parked in M1 if deemed critical-path, otherwise noted as acceptable); **the entire `net` cluster** (v1.1, §5.3). The 13 COMPILES-BUT-ORPHANED and 7 DORMANT-HONEST crates are not v1.0 blockers unless a criterion names them.

---

## 3. The gap list → remediation classes

From §2.2, the gaps sort into three engineering classes + a non-feature track:

- **Class 1 — test-rot (cheap, clears verification noise):** the *stale-assertion* failures in PCFT crates — `render` (2 stale: the `terrain` `chunk_size` 256→512 drift; the shader-string refactor — parity SHA-256 passes), `aw_editor` (1 stale), `blend` (1 stale). Near-free; moves 3 crates toward VP; greens E1/E2.
- **Class 2 — real defects + coverage (genuine but bounded):** `physics` — the **character-controller grounding regression** (capsule grounds at y=0.1 instead of resting on the surface) — *the serious one, pulled into M1, re-characterized as a Rapier-integration fix in §5.1*; `terrain` (8 failing tests — real correctness/coverage gap, E3, stays in M2). Localized (the audit pinpointed them) but real fixes.
- **Class 3 — capability gaps + hollow/over-claim (the largest; the v1.0-defining work):** `asset-pipeline` (cook-path wiring — E4 blocker, leads M2), `persistence-ecs` (live persistence path — E7 blocker, M3), the play-in-editor seam (E5, M3), `input` (wire-or-relabel, M3). This is what makes "an editor a game can be authored on" *true*.
- **Non-feature tracks (cheap, clarifying — all M1):** the **10-crate CLAIM-MISMATCH trace-correction backlog** (R.0.B §6, now **+ the arbiter-as-post-v1.0 mismatch**, §6.1) **+ the `net`-cluster relabel** (§5.1); **folding the v1.1 taxonomy split into the canonical R.0.A §1.2**; **the A2/A5 wording sync** (§4.1, done this beat); the **A5 honesty** note (`phi3`/Qwen3 — don't document the dormant LLM path as live).

> **Observation — dependency-drift / Path-B staleness (a future hygiene beat).** A recurring class surfaced across M1: code or tests **lagging a changed API/dependency**. Confirmed cases + their scope:
>
> - **`render` (M1 test-rot):** stale assertions against a dependency's `chunk_size` 256→512 change and a shader-string refactor — the canonical dependency-drift case.
> - **`physics` (M1.2, hypothesis CORRECTED):** the §5.1 fork was diagnosed (`b8d298d0f`) as fork **(b) wrapper-config, NOT Rapier-version-drift** — a *long-standing wrapper-logic bug* (the snap ignored the capsule half-extent), with `rapier3d` pinned at 0.22 throughout, no version bump. So `physics` is a wrapper bug, **not** a drift case; the earlier "possibly physics" hypothesis is closed.
>
> **Structural scope note (the important part — M1.2 incidental findings).** R.0.B's audit verified the **`cargo test` default-features surface**, so its green verdicts are accurate *for what `cargo test` builds*. But two adjacent surfaces are **outside that methodology** and carry their own Path-B staleness:
> - **Benches (not built by `cargo test`):** `aw-net-proto` bench — `SigningKey::as_bytes`/`from_bytes` gone (`E0599`); `astraweave-core` bench — `Pose` missing fields (`E0063`). Stale APIs the bench code didn't follow, invisible to the test-surface audit.
> - **Feature-gated code (not compiled under default features):** `astraweave-physics --all-features` — `async_scheduler.rs:297` uses `serde_json` as an **undeclared dependency** under a feature flag (compile error under `--all-features`).
>
> These are the **same Path-B class** as the `render` test-rot (code lagging a changed API/dependency), just on surfaces the audit's `cargo test` default-features pass did not reach. **A dependency-currency hygiene beat must sweep benches + `--all-features` compile, not only the test surface, to be complete** — the same shape as the architecture-trace-currency check, but for upstream API/dependency drift across all build surfaces. Not a v1.0 milestone.
>
> **Deferred item (own beat) — `create_ground_plane` + the joint instability it unmasks.** `create_ground_plane` (`astraweave-physics/src/lib.rs:1153`) hardcodes the ground's y half-extent to `0.1`, ignoring the caller's `half.y`. Fixing it (measured in M1.2) **unmasks a pre-existing `fixed_joint_bodies_remain_rigid` instability**: two fixed-jointed boxes spawned 2.0 apart collapse to ~0.291, which the test currently masks via settle-timing luck. This is its own beat — the fix must **handle the unmasked joint instability properly (investigate the instability, not re-tune the test timing to re-mask it)**, a joint-stability concern distinct from grounding. The `character_controller_stays_on_ground` assertion carries a `1.4→1.8` update note for when this lands.

---

## 4. The two AI stability-bar criteria — resolved

### 4.1 A2 — wired-path determinism *(RATIFIED, revised — M1.3 A2 recon `992793c41`)*

The original A2 phrasing ("GOAP↔BT↔LLM mode transitions fire on documented triggers") was **mis-targeted**: it described the hybrid `AIArbiter`, which the A2 recon found is **tested-but-not-production-wired**, feature-gated (`llm_orchestrator` off by default), and live-LLM-requiring. The **wired** production AI path is the deterministic `RuleOrchestrator` (+ `GoapOrchestrator` in `hello_companion`). Revised criterion:

> **A2 (v1.0) is met when the wired deterministic production AI path is provably deterministic:** identical `WorldSnapshot` ⇒ identical `PlanIntent` across repeated runs, for `RuleOrchestrator::propose_plan` (the `ecs_ai_plugin` production path) and `GoapOrchestrator` (the deterministic GOAP path used by the flagship `hello_companion` demo). **No latency bound** — the wired path is synchronous and off any LLM dependency for v1.0.

**The hybrid `AIArbiter` is post-v1.0.** Its real GOAP↔ExecutingLLM transitions (+ GOAP→BT one-way sink) are **already test-covered** ("Phase 7 complete"); wiring it (enable `llm_orchestrator` by default, accept the live-LLM dependency, connect the dormant ~15K-LoC LLM hardening, fix the BT-stub / dead-`fast_executor` / one-way-BT) is a **post-v1.0 capability build** (§6.1 backlog), not a v1.0 A2 requirement. A2 is now a small **determinism-consolidation** test on already-VP crates — both halves were already substantially tested (`goap/tests.rs:92`, `orchestrator.rs:992`).

### 4.2 A3 (validation throughput) — a scheduled measurement task, NOT a declared number

**A3 is not a guessed bar.** Declaring an unmeasured target is exactly the aspirational-number-as-fact the doc-truth campaign forbade. A3 is a **roadmap item that produces the number**:
1. Benchmark `tool_sandbox` validation throughput at realistic concurrent-agent counts.
2. Establish the actual cost.
3. Set the budget against **measured frame-time headroom** — the doc-truth anchor: System allocator `0.965 ms` / mimalloc `0.709 ms` @ 1000 entities against the `16.67 ms` 60 Hz tick gives ~15.7–16.0 ms of headroom to budget validation against.

A3's milestone is **"measure, then set the bar"** — the number is deferred to the measurement's result. A3 lands in M3 (alongside the remaining capability work; the AI path it measures is already stable).

---

## 5. The ratified roadmap (M1 → M2 → M3)

Each milestone lists **what it closes** (crates → VP, criteria met), the **rationale** (why here), and **dependencies**.

### M1 — Clean the board + fix the dangerous defect + trace honesty

**Closes:**
- `render` → VP, `aw_editor` → VP, `blend` → VP (Class-1 test-rot: 4 stale assertions across the three).
- **`physics` → VP** — the Rapier-integration fix (§5.1); `npc`'s inherited defect resolved as a side effect.
- **A2** met (write the **wired-path determinism** test — `RuleOrchestrator` + `GoapOrchestrator`, §4.1; a small consolidation of existing determinism tests under a named A2 contract).
- Non-feature trace honesty: the 10-crate CLAIM-MISMATCH trace re-status (now **+ the arbiter-as-post-v1.0 mismatch in `ai_pipeline.md`**, §6.1) **+ relabel the `net`-cluster trace(s) to example-only**; fold the v1.1 split into R.0.A §1.2; sync the A2/A5 wording (§4.1, this beat done); the **A5 honesty** note (`phi3`/Qwen3 — don't document the dormant LLM path as live); (optionally) `alloc` coverage → VP.
- **E1** (scene authoring) green (`aw_editor` VP + `scene` VP); **E2** (parity) green (`render` VP + `aw_editor` VP); **A2** met.

**Rationale:** M1 **banks the verified half** of the conjunction and **fixes the one propagating defect** before any capability work builds on it. The test-rot is near-free and clears the verification noise; the physics fix is pulled forward because it is the **highest-propagation-risk defect** (`npc` inherits it, and M2/M3 should not build on broken character grounding); A2 closes the last AI criterion; the trace-corrections (including the `net` relabel) make every future state-check cleaner. A legible, verified base.

**Dependencies:** none (test-rot targets are near-VP; A2's crates are VP; the physics fix is localized to the wrapper boundary).

### 5.1 The physics item — a Rapier-integration fix (corrected framing)

**Fix the Rapier `KinematicCharacterController` integration in `astraweave-physics::CharacterController` / `add_character`.** The capsule grounds at y=0.1 instead of resting on the surface. **`astraweave-physics` wraps `rapier3d`** — it is the in-house *integration layer over Rapier*, **not an in-house physics engine** (Rapier does the solving). So the fix lives at the **wrapper boundary**, not in physics math AstraWeave owns.

**First diagnostic fork** (the fix differs by which):
- **(a) Rapier-version-drift** — a `rapier3d` workspace-version behavior change (e.g. a `KinematicCharacterController` default offset / `snap_to_ground` change) the wrapper didn't track. *This is the same dependency-drift class as the `render` test-rot* (§3 observation).
- **(b) Wrapper-configuration bug** — the offset, the `snap_to_ground` setting, or the collider half-height / translation in `add_character`.

`astraweave-npc` inherits the bug; fixing it resolves npc's inherited defect. Moves `physics` → VERIFIED-PRODUCTION. *(This beat only writes the corrected plan; the verdict stays `PRODUCTION-CAPABLE-FAILING-TESTS` until M1 executes the fix.)*

### M2 — Capability, led by the biggest blocker

**Closes:**
- `asset-pipeline` → VP-or-relabel — **the lead item:** wire the BC7/KTX2 cook path to a live caller (e.g. `aw_asset_cli` or the editor's asset import) OR drop the present-tense `lib.rs:4` claim + the unused editor dep. **E4** (asset cook path) green — the biggest single Veilweaver authoring blocker.
- `terrain` → VP (fix the 8 failing tests). **E3** (terrain/material) green (`terrain` VP + `materials` VP).

**Rationale:** with the board clean and physics correct (M1), M2 leads with the **cook path** — the biggest authoring unblock — on a now-verified base. `terrain`'s 8 failing tests ride alongside (E3 is the terrain/material criterion, adjacent to the cook-path/asset work).

**Dependencies:** none hard. Built on M1's verified base (clean board + correct physics).

### M3 — Remaining capability

**Closes:**
- `persistence-ecs` → VP / live path: implement + wire `auto_save`/`replay` (or a live persistence path). **E7** (save/load) green (`aw-save` VP + a live persistence path).
- The **play-in-editor seam** defined + wired (editor ↔ AI play mode). **E5** green.
- `input` → VP-or-relabel (wire into the editor/gameplay, OR re-status the trace to example-only if the editor's own input path is canonical).
- **A3** measured + bar set (§4.2).

**Rationale:** the remaining capability work — persistence and play-in-editor — plus the `input` wire-or-relabel and the A3 measurement (most meaningful once capability is in place). Smaller than the strawman's M3 because **`net` is removed** (now a v1.0-out relabel in M1).

**Dependencies:** E5 (play-in-editor) wants E1/E2 green (M1) and the AI pillar green (already met) — satisfied by M1. E7/input are independent.

### 5.3 Networking — deferred to v1.1 (resolved)

The §5.3 net-scope question is **resolved: networking is v1.1.** `net`/`net-ecs`/the aw-net-trio drop off the v1.0 critical path. Their v1.0 work is **only the trace-honesty relabel** (make the honest label match the example-only reality), not a multiplayer wire-up — so it moves into M1's cheap trace-correction track, **not** M3's capability work. Apply the relabel to whichever traces carry the *example-only-reality vs. `active/wired`-claim* mismatch (primarily `net.md`); the VERIFIED-PRODUCTION net crates (`aw-net-server`/`-client`/`-proto`) stay as-is — they are honestly wired for their bin/example role. **Recorded deferral:** real multiplayer wire-up is v1.1 scope, not dropped.

### 5.4 Dependency summary

The milestones build on each other but have few hard blockers. M1 banks the verified base (clean board + correct physics + AI verified). M2 leads with the cook path on that base. M3's E5 (play-in-editor) wants E1/E2 (M1, satisfied). Otherwise the capability items are independent.

---

## 6. The sequence decision (recorded)

The strawman offered two orderings; **the director chose the difficulty ladder, with the physics defect pulled into M1 and networking ruled out of v1.0.**

**Chosen (ratified):** M1 (clean board + the Rapier-integration physics fix + trace honesty) → M2 (capability led by the cook path + terrain) → M3 (persistence + play-in-editor + input + A3).

**Why (the rationale):** the v1.0 definition is **conjunctive** (clean+verified **and** authorable), so a **verified base first** is favored — clear the board and fix the one *propagating* defect (the physics regression `npc` inherits) before capability work builds on it. The **biggest-blocker-first** alternative (front-loading the cook path) was considered and **partially adopted**: the cook path does not wait until last — it **leads M2**, the very next milestone — so the biggest authoring blocker is addressed early, but on a verified, physics-correct base rather than on top of a known character-grounding regression. Pulling physics into M1 is the one deviation from the pure difficulty ladder, justified by its propagation risk. Networking out of v1.0 (§5.3) shrinks M3 and removes an undecided scope question.

### 6.1 Post-v1.0 backlog — the hybrid LLM `AIArbiter` (deferred, not dropped)

The A2 recon (`992793c41`) established the hybrid GOAP+LLM `AIArbiter` as **tested-but-dormant** and **consciously post-v1.0**. Recorded so the post-v1.0 arbiter-wiring beat has the full list:

- **Wire the arbiter** — enable `llm_orchestrator` by default, accept the live-LLM (Ollama) dependency, connect the dormant ~15K-LoC LLM hardening surface (rate limiting / circuit breaking / retry / ToolGuard / 4-tier fallback — all currently bypassed), and a production constructor (none exists today).
- **Honesty gaps the recon found** (wire / relabel / remove each — arbiter-scope, NOT v1.0 blockers):
  - `PlannerMode::BehaviorTree` is a **stub** — `dispatch_bt` → `bail!("BehaviorTree integration not yet implemented")` (`core_loop.rs:278`).
  - `fast_executor` is **dead** — stored at construction, never read (`ai_arbiter.rs:209`).
  - The arbiter's **BT is a one-way terminal sink** — `GOAP→BT` on empty-plan, but no recovery edge out of BT.
- **Trace CLAIM-MISMATCH (logged to the M1 trace-honesty track, not done here):** `ai_pipeline.md` should record the `AIArbiter` as **in-design/post-v1.0** (tested-but-not-wired) — the "stable AI pipeline" v1.0 claim rests on `RuleOrchestrator`/`GoapOrchestrator`, not the arbiter. Add to the 10-crate CLAIM-MISMATCH backlog.

---

## 7. v1.0 done-state checklist (both halves of the conjunction)

**Clean/verified half — the critical-path board is green:**
- [ ] `render`, `aw_editor`, `blend`, `physics` → VERIFIED-PRODUCTION (M1)
- [ ] `asset-pipeline`, `terrain` → VERIFIED-PRODUCTION-or-relabeled (M2)
- [ ] `persistence-ecs`, `input` → VERIFIED-PRODUCTION-or-relabeled (M3)
- [ ] `net` cluster → trace relabeled to example-only (M1) — **not a critical-path-green requirement (v1.1)**
- [ ] (already green: `core`, `ecs`, `sdk`, `ai`, `behavior`, `llm`, `scene`, `materials`, `camera`, `cinematics`, `asset`, `aw-save`)

**Authorable-editor + stable-AI half — E/A criteria met:**
- [ ] E1 scene authoring · E2 parity (M1)
- [ ] E3 terrain/material · E4 asset cook path (M2)
- [ ] E5 play-in-editor · E7 save/load (M3)
- [x] E6 camera (already met)
- [ ] A2 **wired-path determinism** test (M1) · A3 throughput measured + bar set (M3)
- [x] A1, A4, A6 (wired deterministic path — already met)
- [ ] A5 = **honesty constraint** (no v1.0 doc overclaims the LLM as production) — capability deferred post-v1.0
- **(networking: NOT a v1.0 criterion — v1.1; the hybrid LLM `AIArbiter`: NOT a v1.0 criterion — post-v1.0, §6.1)**

v1.0 is reached when every box is checked — the board green on the (networking-excluded) critical path **and** every E/A criterion met.

---

## 8. What this is NOT

- **NOT doing the work** — this is the ratified plan; the milestones describe the fixes, they do not execute them. Zero code changes in this beat. **The next beat is M1 execution** (its own bounded-recon → gate → build).
- **NOT calendar dates** — sequence and dependency, not a timeline. (Effort estimates can be added later if wanted; the default is ordering.)
- **NOT re-auditing** — the R.0.B state map is the verified input; this roadmap consumes it.
- **NOT declaring the A3 number** — A3 is the measurement task; the number comes from running it.
- **NOT changing the physics verdict** — the §5.1 re-characterization is an accurate *framing* of the fix; `physics` stays `PRODUCTION-CAPABLE-FAILING-TESTS` until M1 executes the fix.

---

*Ratified roadmap, folded from the R.1 strawman per director decision (2026-06-30): difficulty ladder + physics→M1 (Rapier-integration framing) + networking→v1.1. The next phase executes M1.*
