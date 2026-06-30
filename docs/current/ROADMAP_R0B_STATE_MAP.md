# R.0.B — Engine-Wide Production-Readiness State Map (74 crates)

> **Campaign**: R-series (Engine Roadmap to v1.0.0) · **Phase**: R.0.B (exhaustive fan-out audit) · **Branch**: `campaign/roadmap`
> **Method**: the R.0.A-ratified methodology (`docs/current/ROADMAP_R0A_RECON.md` §1), applied by 11 cluster-coherent shards + an `aw_editor` sub-shard (12 agents), aggregated with a coverage gate + first-FPR second-agent confirmation. **Taxonomy v1.1 split applied** (§3) — deterministic recategorization from captured fields; no shard re-run.
> **Run**: workflow `wf_46cef23b-3f8` · 13 agents · 339 tool calls · 795987 tokens · ~102 min wall-clock.
> **Status**: AUDIT COMPLETE + RATIFIED (2026-06-29). The verified state map; verdicts are the shards' outputs, with the ratified v1.1 split recategorizing the wired-but-failing crates. No roadmap authored yet (next R-phase).
> **Date**: 2026-06-29 (v1.1 split + close: 2026-06-30)

---

## 1. Coverage + commensurability gate

- **Coverage: 74/74 — COMPLETE.** `coverage_complete=True`; missing=[]; extra=[]. Every shard returned ok (5,6,9,4,6,3,4,3,10,11,12,1 = 74).
- **Commensurability vs the R.0.A pilot (re-derived blind, pilot verdicts never revealed to shards): 4/4 AGREE.**

| crate | pilot verdict | fan-out verdict | agree |
|---|---|---|---|
| astraweave-ecs | VERIFIED-PRODUCTION | VERIFIED-PRODUCTION | ✅ |
| astraweave-fluids | DORMANT-HONEST | DORMANT-HONEST | ✅ |
| astraweave-math | COMPILES-BUT-ORPHANED | COMPILES-BUT-ORPHANED | ✅ |
| astraweave-persistence-ecs | COMPILES-BUT-ORPHANED | COMPILES-BUT-ORPHANED | ✅ |

The shards independently reproduced all four pilot verdicts → the methodology is commensurable on known-status crates. The one divergence the audit surfaced — at the FALSE-PRODUCTION-READY ↔ WIRED-BUT-UNTESTED boundary for *wired crates with failing tests* — is **resolved by the v1.1 split (§3)**, not a data problem.

## 2. Verdict distribution (post-v1.1-split)

| verdict | n | meaning |
|---|---|---|
| VERIFIED-PRODUCTION | 40 | claimed production, live caller(s), tests pass + exercise the feature |
| COMPILES-BUT-ORPHANED | 13 | compiles (often well-tested), but no live caller — island/example-only, no production claim attributable |
| **PRODUCTION-CAPABLE-FAILING-TESTS** | **5** | **(v1.1) wired to live callers, non-stub cores, BUT ≥1 failing test** — a real product with test-rot or a regression, NOT a hollow crate |
| WIRED-BUT-UNTESTED | 5 | used by live code but feature test coverage thin/absent (and tests are green) |
| DORMANT-HONEST | 7 | honestly labelled non-production, reality corroborates |
| **FALSE-PRODUCTION-READY** | **3** | **(v1.1, narrowed) claims production but is hollow/over-claimed — orphaned or stubbed against a production claim** |
| UNTRACED-UNKNOWN | 1 | untraced, checks did not resolve to a confident verdict |
| **total** | **74** | |

Flag tallies (unchanged by the split): {"CLAIM-MISMATCH": 10, "DECLARED-BUT-UNUSED": 7, "PATH-B-STALE-TESTS": 5}.

## 3. THE KEY FINDING — the v1.1 taxonomy split

The R.0.A methodology's rank-2 FALSE-PRODUCTION-READY rule fired on **any** production-claimed crate that failed verification — which conflated **a real wired product with a failing/stale test** and **a hollow crate that falsely advertises production**. Different shards even labelled the *same* situation differently (render/physics → FPR; terrain/aw_editor → WIRED-BUT-UNTESTED). The ratified fix splits rank-2 deterministically from the **already-captured fields** (`reachability` + own-docs-claim + `feature_exercising` + `stub_core` + test pass/fail). **No shard re-run — the data is sound; only the labels recategorize.**

> ### Methodology v1.1 amendment (recorded here; supersedes R.0.A §1.2 rank-2)
> **New verdict — `PRODUCTION-CAPABLE-FAILING-TESTS`:** a crate that is **wired to live callers** (`reachability` ∈ {live, bin}), has **non-stub advertised cores** (`stub_core=no`, `feature_exercising` ∈ {yes, partial}), but has **≥1 failing test**. It is a real, integrated product with test-rot or a regression — explicitly *distinct* from a hollow crate.
> **Narrowed verdict — `FALSE-PRODUCTION-READY`:** now reserved for the **hollow / over-claim** kind — a production claim (role-P active/wired, or an own-docs present-tense claim) **contradicted by orphaning** (ISLAND/ex-only, no live caller) **or a stubbed core**. This is the dangerous false-positive the audit exists to catch.
> **Selector (mechanical):** a production-claimed crate that fails verification → if `reachability` ∈ {ISLAND, ex-only} (boundary=no) **or** `stub_core=yes` **or** `feature_exercising=no` ⇒ **FALSE-PRODUCTION-READY**; else (wired + non-stub + failing test) ⇒ **PRODUCTION-CAPABLE-FAILING-TESTS**.
> **Propagation note:** R.0.A §1.2 still carries the superseded rank-2 rule; the canonical methodology-doc update (folding this split into R.0.A §1.2) is a tiny separate follow-on — deliberately *not* done in this beat to keep scope clean. The split is authoritative here.

### 3a. FALSE-PRODUCTION-READY (narrowed) — the 3 GENUINE hollow / over-claims

These are the dangerous false-positives R.0.A was built to catch: a production claim contradicted by reality.

> **[RESOLUTION — 2026-06-30, R-series M2/E4 relabel-and-defer]** One of these three — **`astraweave-asset-pipeline`** — has since been **resolved by relabel-and-defer**: the `lib.rs:4` over-claim was rewritten to honest in-design status and the unused `aw_editor` dep (`Cargo.toml:99`) dropped, moving it **FALSE-PRODUCTION-READY → DORMANT-HONEST**. The BC7/KTX2 cook **build** is deferred post-v1.0 (v1.0 authors on raw PNG/glTF; the render path has no GPU-compressed-upload path, so cooking is a non-blocking optimization). The §2 distribution + §4 tables below are preserved **as audited (2026-06-29)**; this banner records the post-audit resolution. See `docs/audits/e4_cook_path_recon_2026-06.md` and `ROADMAP_R1_PATH_TO_V1.md` §6.2. The other two (`astraweave-input`, `astraweave-net`) remain open per their milestones.

| crate | reach | tests | the contradiction | gap |
|---|---|---|---|---|
| **astraweave-asset-pipeline** | ISLAND | 0/0 | claim active/wired (role O) vs orphaned reality | Wire BC7/ASTC texture compression + mesh optimize_mesh + validator into at least one non-test production path (e.g. aw_asset_cli or aw_editor asset import), OR remove the present-tense production clai |
| **astraweave-input** | ISLAND | 305/0 | claim active/wired (role P) vs orphaned reality | Add >=1 real non-example source caller: have astraweave-ui or astraweave-gameplay (which already declare the dep) actually `use astraweave_input::{InputManager, BindingSet}` and drive the per-frame in |
| **astraweave-net** | ex-only | 357/0 | claim active/wired (role P) vs orphaned reality | Wire astraweave-net::GameServer/Msg/Snapshot into a non-example binary (engine runtime or tool), OR re-status the net.md trace to integration=example_only to make the honest label match reality. Curre |

- **`astraweave-asset-pipeline` is the cleanest catch** — and the ratified present-tense sharpening is what nailed it: `src/lib.rs:4` said *"This crate provides production-ready asset processing"* (present-tense own-docs claim) while the crate is a **caller-less island** (the editor declares the dep, uses nothing). **[Resolved 2026-06-30, M2/E4: claim removed — relabeled to in-design; dep dropped; verdict → DORMANT-HONEST. See the §3a resolution banner + §7.]**
- **`astraweave-input`** (305 green tests) and **`astraweave-net`** (357 green tests) *look* healthy on the test axis but are orphaned/example-only against an `active/wired` trace claim — the green-test trap (R2) firing as designed. `net.md`'s own body even admits "only example consumers exist."

### 3b. PRODUCTION-CAPABLE-FAILING-TESTS (v1.1) — 5 wired products with a failing test

Real, integrated crates with non-stub cores and a failing test. **Recategorization table** (the captured-field logic that drove each move — legible, not silent):

| crate | was | reach | tests p/f | feat | stub | why PCFT (captured-field rationale) |
|---|---|---|---|---|---|---|
| **astraweave-blend** | WIRED-BUT-UNTESTED | live | 52/1 | partial | no | wired (live) + non-stub + 1 failing ⇒ not hollow |
| **astraweave-physics** | FALSE-PRODUCTION-READY | live | 1693/1 | partial | no | wired (live) + non-stub + 1 failing ⇒ not hollow |
| **astraweave-render** | FALSE-PRODUCTION-READY | live | 1277/2 | yes | no | wired (live) + non-stub + 2 failing ⇒ not hollow |
| **astraweave-terrain** | WIRED-BUT-UNTESTED | live | 936/8 | partial | no | wired (live) + non-stub + 8 failing ⇒ not hollow |
| **aw_editor** | WIRED-BUT-UNTESTED | bin | 9323/1 | yes | no | wired (bin) + non-stub + 1 failing ⇒ not hollow |

**Two sub-kinds within PCFT — important distinction:**
- **Test-rot (stale assertions, trivial fix):** `astraweave-render` (1277/2 — a dependency's `chunk_size` 256→512; a shader-string refactor; SHA-256 parity tests **pass**), `aw_editor` (9323/1), `astraweave-blend` (52/1). The product is correct; the *test* drifted.
- **Real functional defect (NOT test-rot — the more serious):** **`astraweave-physics`** (1693/1) — a genuine **character-controller grounding regression** (capsule sinks to y=0.1 instead of resting on the ground plane); a live consumer (`astraweave-npc`) inherits the bug. And **`astraweave-terrain`** (936/**8** failing) — eight failing tests, a real correctness/coverage gap, not a single stale assertion.

**§3c resolved:** before the split, `render`/`physics` landed FALSE-PRODUCTION-READY while the structurally-identical `terrain`/`aw_editor`/`blend` landed WIRED-BUT-UNTESTED — a pure shard-label divergence (the underlying data was consistent everywhere). After the split, **all five wired-failing crates carry `PRODUCTION-CAPABLE-FAILING-TESTS`; the divergence is gone.**

## 4. Full state map (all 74, grouped by verdict; shard preserved)

### FALSE-PRODUCTION-READY (3)

| crate | shard | verdict | reach | tests p/f | feat | flags |
|---|---|---|---|---|---|---|
| astraweave-asset-pipeline | E-Asset-Terrain-Water | FALSE-PRODUCTION-READY | ISLAND | 0/0 | n-a | DECLARED-BUT-UNUSED,CLAIM-MISMATCH |
| astraweave-input | H-UI-Audio-Input | FALSE-PRODUCTION-READY | ISLAND | 305/0 | yes | CLAIM-MISMATCH,DECLARED-BUT-UNUSED |
| astraweave-net | I-Net-Persist-Sec | FALSE-PRODUCTION-READY | ex-only | 357/0 | yes | CLAIM-MISMATCH |

### PRODUCTION-CAPABLE-FAILING-TESTS (5)

| crate | shard | verdict | reach | tests p/f | feat | flags |
|---|---|---|---|---|---|---|
| astraweave-render | D-Render | PRODUCTION-CAPABLE-FAILING-TESTS | live | 1277/2 | yes | PATH-B-STALE-TESTS |
| astraweave-blend | E-Asset-Terrain-Water | PRODUCTION-CAPABLE-FAILING-TESTS | live | 52/1 | partial | PATH-B-STALE-TESTS |
| astraweave-terrain | E-Asset-Terrain-Water | PRODUCTION-CAPABLE-FAILING-TESTS | live | 936/8 | partial | PATH-B-STALE-TESTS |
| astraweave-physics | F-Physics-Nav-PCG | PRODUCTION-CAPABLE-FAILING-TESTS | live | 1693/1 | partial | PATH-B-STALE-TESTS |
| aw_editor | K-Editor | PRODUCTION-CAPABLE-FAILING-TESTS | bin | 9323/1 | yes | PATH-B-STALE-TESTS |

### COMPILES-BUT-ORPHANED (13)

| crate | shard | verdict | reach | tests p/f | feat | flags |
|---|---|---|---|---|---|---|
| astraweave-math | A-Foundation | COMPILES-BUT-ORPHANED | ex-only | 195/0 | yes | CLAIM-MISMATCH,DECLARED-BUT-UNUSED |
| astraweave-coordination | B-AI-Core | COMPILES-BUT-ORPHANED | ISLAND | 149/0 | yes | CLAIM-MISMATCH,DECLARED-BUT-UNUSED |
| astraweave-npc | B-AI-Core | COMPILES-BUT-ORPHANED | ex-only | 113/0 | yes | CLAIM-MISMATCH |
| astraweave-ipc | C-AI-LLM | COMPILES-BUT-ORPHANED | ex-only | 64/0 | yes |  |
| astraweave-net-ecs | I-Net-Persist-Sec | COMPILES-BUT-ORPHANED | ISLAND | 31/0 | partial |  |
| astraweave-persistence-ecs | I-Net-Persist-Sec | COMPILES-BUT-ORPHANED | ISLAND | 135/0 | partial |  |
| astraweave-persistence-player | I-Net-Persist-Sec | COMPILES-BUT-ORPHANED | ISLAND | 24/1 | partial |  |
| astraweave-author | J-Infra-Misc | COMPILES-BUT-ORPHANED | ex-only | 23/0 | yes | DECLARED-BUT-UNUSED |
| astraweave-optimization | J-Infra-Misc | COMPILES-BUT-ORPHANED | ISLAND | 67/0 | yes |  |
| astraweave-scripting | J-Infra-Misc | COMPILES-BUT-ORPHANED | ex-only | 139/0 | yes |  |
| astraweave-steam | J-Infra-Misc | COMPILES-BUT-ORPHANED | ISLAND | 8/0 | partial |  |
| astraweave-stress-test | J-Infra-Misc | COMPILES-BUT-ORPHANED | ISLAND | 35/0 | yes |  |
| veilweaver_slice_runtime | J-Infra-Misc | COMPILES-BUT-ORPHANED | ex-only | 840/0 | yes |  |

### WIRED-BUT-UNTESTED (5)

| crate | shard | verdict | reach | tests p/f | feat | flags |
|---|---|---|---|---|---|---|
| astraweave-alloc | A-Foundation | WIRED-BUT-UNTESTED | live | 0/0 | no |  |
| aw_demo_builder | K-Tools | WIRED-BUT-UNTESTED | bin | 0/0 | no |  |
| aw_release | K-Tools | WIRED-BUT-UNTESTED | bin | 0/0 | no |  |
| aw_save_cli | K-Tools | WIRED-BUT-UNTESTED | bin | 0/0 | no |  |
| aw_texture_gen | K-Tools | WIRED-BUT-UNTESTED | bin | 0/0 | no |  |

### UNTRACED-UNKNOWN (1)

| crate | shard | verdict | reach | tests p/f | feat | flags |
|---|---|---|---|---|---|---|
| astraweave-llm-eval | C-AI-LLM | UNTRACED-UNKNOWN | bin | 48/0 | partial |  |

### DORMANT-HONEST (7)

| crate | shard | verdict | reach | tests p/f | feat | flags |
|---|---|---|---|---|---|---|
| astraweave-fluids | E-Asset-Terrain-Water | DORMANT-HONEST | ex-only | 677/0 | yes |  |
| astraweave-pcg | F-Physics-Nav-PCG | DORMANT-HONEST | ISLAND | 114/0 | yes | DECLARED-BUT-UNUSED |
| astraweave-weaving | G-Gameplay | DORMANT-HONEST | ex-only | 820/0 | yes |  |
| astraweave-ui | H-UI-Audio-Input | DORMANT-HONEST | ex-only | 824/0 | yes |  |
| astraweave-secrets | I-Net-Persist-Sec | DORMANT-HONEST | bin | 54/0 | yes |  |
| aw_debug | K-Tools | DORMANT-HONEST | ex-only | 15/0 | partial | DECLARED-BUT-UNUSED |
| ollama_probe | K-Tools | DORMANT-HONEST | bin | 0/0 | no |  |

### VERIFIED-PRODUCTION (40)

| crate | shard | verdict | reach | tests p/f | feat | flags |
|---|---|---|---|---|---|---|
| astraweave-core | A-Foundation | VERIFIED-PRODUCTION | live | 977/0 | yes |  |
| astraweave-ecs | A-Foundation | VERIFIED-PRODUCTION | live | 855/0 | yes |  |
| astraweave-sdk | A-Foundation | VERIFIED-PRODUCTION | ISLAND | 81/0 | yes |  |
| astraweave-ai | B-AI-Core | VERIFIED-PRODUCTION | live | 635/0 | yes |  |
| astraweave-behavior | B-AI-Core | VERIFIED-PRODUCTION | live | 459/0 | yes |  |
| astraweave-dialogue | B-AI-Core | VERIFIED-PRODUCTION | live | 222/0 | yes |  |
| astraweave-director | B-AI-Core | VERIFIED-PRODUCTION | live | 187/0 | yes |  |
| astraweave-context | C-AI-LLM | VERIFIED-PRODUCTION | live | 300/0 | yes |  |
| astraweave-embeddings | C-AI-LLM | VERIFIED-PRODUCTION | live | 221/0 | yes | CLAIM-MISMATCH |
| astraweave-llm | C-AI-LLM | VERIFIED-PRODUCTION | live | 973/0 | yes |  |
| astraweave-memory | C-AI-LLM | VERIFIED-PRODUCTION | live | 1022/0 | yes | CLAIM-MISMATCH |
| astraweave-persona | C-AI-LLM | VERIFIED-PRODUCTION | live | 308/0 | yes | CLAIM-MISMATCH |
| astraweave-prompts | C-AI-LLM | VERIFIED-PRODUCTION | live | 1376/0 | yes |  |
| astraweave-rag | C-AI-LLM | VERIFIED-PRODUCTION | live | 288/0 | yes | CLAIM-MISMATCH |
| astraweave-camera | D-Render | VERIFIED-PRODUCTION | live | 25/0 | yes |  |
| astraweave-materials | D-Render | VERIFIED-PRODUCTION | live | 250/0 | yes |  |
| astraweave-scene | D-Render | VERIFIED-PRODUCTION | live | 453/0 | yes |  |
| astraweave-asset | E-Asset-Terrain-Water | VERIFIED-PRODUCTION | live | 22/0 | yes |  |
| astraweave-water | E-Asset-Terrain-Water | VERIFIED-PRODUCTION | live | 9/0 | yes |  |
| astraweave-nav | F-Physics-Nav-PCG | VERIFIED-PRODUCTION | live | 496/0 | yes |  |
| astraweave-cinematics | G-Gameplay | VERIFIED-PRODUCTION | live | 341/0 | yes |  |
| astraweave-gameplay | G-Gameplay | VERIFIED-PRODUCTION | live | 730/0 | yes |  |
| astraweave-quests | G-Gameplay | VERIFIED-PRODUCTION | live | 227/0 | yes |  |
| astraweave-audio | H-UI-Audio-Input | VERIFIED-PRODUCTION | live | 558/0 | yes |  |
| astraweave-security | I-Net-Persist-Sec | VERIFIED-PRODUCTION | live | 490/0 | yes |  |
| aw-net-client | I-Net-Persist-Sec | VERIFIED-PRODUCTION | bin | 4/0 | yes |  |
| aw-net-proto | I-Net-Persist-Sec | VERIFIED-PRODUCTION | live | 59/0 | yes |  |
| aw-net-server | I-Net-Persist-Sec | VERIFIED-PRODUCTION | bin | 41/0 | yes |  |
| aw-save | I-Net-Persist-Sec | VERIFIED-PRODUCTION | live | 6/0 | yes |  |
| asset_signing | J-Infra-Misc | VERIFIED-PRODUCTION | live | 16/0 | yes |  |
| astract | J-Infra-Misc | VERIFIED-PRODUCTION | live | 170/0 | yes |  |
| astract-macro | J-Infra-Misc | VERIFIED-PRODUCTION | live | 7/0 | yes |  |
| astraweave-observability | J-Infra-Misc | VERIFIED-PRODUCTION | live | 132/0 | yes |  |
| astraweave-profiling | J-Infra-Misc | VERIFIED-PRODUCTION | live | 33/0 | yes |  |
| astraweave-assets | K-Tools | VERIFIED-PRODUCTION | bin | 270/0 | yes |  |
| aw_asset_cli | K-Tools | VERIFIED-PRODUCTION | live | 46/0 | yes |  |
| aw_build | K-Tools | VERIFIED-PRODUCTION | bin | 14/0 | yes |  |
| aw_doc_lint | K-Tools | VERIFIED-PRODUCTION | bin | 15/0 | yes |  |
| aw_headless | K-Tools | VERIFIED-PRODUCTION | bin | 23/0 | yes |  |
| aw_trace_sync | K-Tools | VERIFIED-PRODUCTION | bin | 1/0 | yes |  |

## 5. Second-agent confirmation (first FPR safeguard)

First FALSE-PRODUCTION-READY fired (pre-split) = **`astraweave-render`** (`astraweave-render`). Independent second agent (fresh context, not told the verdict):

- **shard verdict:** FALSE-PRODUCTION-READY · **second-agent verdict:** FALSE-PRODUCTION-READY · **agreement: True**
- The safeguard worked *and* surfaced the split: the second agent itself flagged render as "the softest possible FALSE-PRODUCTION-READY … production-capable but test-suite stale," which is precisely the distinction the v1.1 split formalizes. Post-split, render is `PRODUCTION-CAPABLE-FAILING-TESTS` (the second agent's own reading); the genuine FALSE-PRODUCTION-READY set is the 3 hollow crates in §3a.

## 6. Trace-correction backlog (CLAIM-MISMATCH ×10)

Traces whose group status over-claims at the crate granularity (R1 group-status hazard). These must be individually re-statused in their front-matter:

| crate | fan-out verdict | claimed | issue |
|---|---|---|---|
| astraweave-math | COMPILES-BUT-ORPHANED | active/wired (role O) | inherited status not individually verifiable |
| astraweave-npc | COMPILES-BUT-ORPHANED | active/wired (role O) | inherited status not individually verifiable |
| astraweave-coordination | COMPILES-BUT-ORPHANED | active/wired (role O) | inherited status not individually verifiable |
| astraweave-memory | VERIFIED-PRODUCTION | active/wired (role O) | inherited status not individually verifiable |
| astraweave-embeddings | VERIFIED-PRODUCTION | active/wired (role O) | inherited status not individually verifiable |
| astraweave-rag | VERIFIED-PRODUCTION | active/wired (role O) | inherited status not individually verifiable |
| astraweave-persona | VERIFIED-PRODUCTION | active/wired (role O) | inherited status not individually verifiable |
| astraweave-asset-pipeline | FALSE-PRODUCTION-READY | active/wired (role O) | inherited status not individually verifiable |
| astraweave-input | FALSE-PRODUCTION-READY | active/wired (role P) | inherited status not individually verifiable |
| astraweave-net | FALSE-PRODUCTION-READY | active/wired (role P) | inherited status not individually verifiable |

Note: `astraweave-memory/embeddings/rag/persona` are VERIFIED-PRODUCTION yet flagged — their `ai_pipeline` group `active/wired` status holds in substance but was flagged as inherited-not-individually-asserted; low-priority trace hygiene. `asset-pipeline`/`input`/`net` mismatches are the substantive ones (they coincide with the §3a hollow FPRs).

## 7. v1.0 critical-path read (against R.0.A §3 criteria)

**Pillar II — AI pipeline: SOLID.** A1 core loop `astraweave-ai` ✅VP; A2 arbiter `ai`+`behavior` ✅VP; A4 determinism `ecs`+`core` ✅VP; A5 `llm` ✅VP; A6 stability bar — **none of {ai, behavior, llm, ecs, core} is below VERIFIED-PRODUCTION**. The AI critical path has zero island/stub/hollow crates. (A2/A3 numeric bars still deferred; `director` ✅VP too.)

**Foundation gate: SOLID.** `core` ✅VP, `ecs` ✅VP, **`sdk` ✅VP via the R3 boundary exception** (island-by-design FFI surface judged on external contract — the carve-out worked). `math` COMPILES-BUT-ORPHANED — confirmed off the critical path (only `profiling_demo` consumes it). `alloc` WIRED-BUT-UNTESTED (0 tests).

**Pillar I — editor: the real gaps live here.**
| Criterion | Verdict picture | Gap |
|---|---|---|
| E1 scene authoring | `aw_editor` PRODUCTION-CAPABLE-FAILING-TESTS (9323/1), `scene` ✅VP | fix 1 stale editor test; thin feature coverage |
| E2 WYSIWYG parity | `render` PCFT (1277/2 — **stale tests**), `aw_editor` PCFT | parity SHA-256 tests already pass; near-met once 2 render assertions fixed |
| E3 terrain/material | `terrain` PCFT (936/**8 failing**), `materials` ✅VP | **8 failing terrain tests** — real correctness/coverage gap |
| E4 asset cook path | `asset` ✅VP, ~~`asset-pipeline` FALSE-PRODUCTION-READY~~ → **DORMANT-HONEST** | ✅ **RESOLVED 2026-06-30 via relabel-and-defer** — the `lib.rs:4` over-claim was rewritten to honest in-design status + the unused editor dep dropped; the BC7/KTX2 cook path is **NOT a v1.0 blocker** (render consumes raw PNG→RGBA8, no GPU-compressed-upload path) and its build is deferred post-v1.0. See `docs/audits/e4_cook_path_recon_2026-06.md` |
| E5 play-in-editor | seam not yet a single crate | locate/define the editor↔AI play-mode seam |
| E6 camera | `camera` ✅VP, `cinematics` ✅VP | **met** |
| E7 save/load | `aw-save` ✅VP, **`persistence-ecs` COMPILES-BUT-ORPHANED** (auto_save/replay stubs) | wire a live persistence path — the confirmed gap |

## 8. Recommended next step

The verified state map is complete: the foundation + AI pillars sit at VERIFIED-PRODUCTION across the board (zero island/stub/hollow crates on either critical path). The actionable v1.0 gaps are concentrated in the editor pillar and split into three remediation classes:
1. **Trivial test-rot** (`render` ×2 stale, `aw_editor` ×1, `blend` ×1) — fix stale assertions; the crates are real and wired (`PRODUCTION-CAPABLE-FAILING-TESTS`).
2. **Real defects/coverage** (`physics` character-controller grounding regression; `terrain` ×8 failing tests) — genuine bugs, also `PRODUCTION-CAPABLE-FAILING-TESTS` but functional, not stale.
3. **Hollow / over-claim** (`asset-pipeline`, `input`, `net` — `FALSE-PRODUCTION-READY`) — wire to a live caller or correct the over-claiming trace/docs; `persistence-ecs` orphan + stub is the E7 blocker.

Plus the trace-correction backlog (§6) and the canonical-methodology follow-on (fold the v1.1 split into R.0.A §1.2). **Next phase (its own beat): author the roadmap — turn the §7 gaps into sequenced work toward dogfood-ready.** This doc is read-only recon; no roadmap authored here.

## 9. Process lessons (durable — for future audits)

Two process findings the R.0.B run surfaced, logged so future test-running audits don't repeat them:

1. **`cargo test` is NOT read-only.** A shard's `cargo test -p astraweave-audio` **deleted 13 committed `.wav` test fixtures** (test-teardown side-effect) — the *second* time this crate's fixtures were removed by a test run (the doc-truth coverage run did it too). The fixtures were restored (`git restore astraweave-audio/tests/assets/`); the tree is clean. **Lesson:** a test-running "read-only" audit needs a **post-run `git status` check + fixture restoration** as standard hygiene — running tests mutates the tree.
2. **`aw_doc_lint` scans git-tracked files only.** The R.0.A pre-commit lint check ran **before `git add`**, so the untracked doc was never scanned — a false-clean that the enforce gate later caught (9 occurrences in the committed R.0.A doc). **Lesson:** doc-truth lint checks must **`git add` first, then lint** (`git ls-files` scope). The 15 legitimate R-series closed-vocabulary occurrences (methodology defining/quoting the vocabulary; the corrected `55 crates`→74 figure; the measured `977` core test count) are now **allowlisted** in `docs/campaigns/doc-truth/CLOSED_VOCABULARY_ALLOWLIST.md` — the sanctioned mechanism, setting the precedent for all future R-series docs.

---
*Generated from workflow `wf_46cef23b-3f8` state_map (74 records). Verdicts are the shard outputs with the ratified v1.1 split applied deterministically from captured fields; §3/§7 are aggregation analysis.*