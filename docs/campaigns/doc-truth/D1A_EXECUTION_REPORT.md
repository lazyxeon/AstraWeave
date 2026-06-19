# D.1.A Execution Report — Registry Skeleton + Worst-Offender Corrections

**First write pass of the Documentation Truth campaign.** Built the `CLAIMS_REGISTRY.md` keystone, applied DELETE/CORRECT/REWRITE/LEAVE to the 16 worst-offender files, and routed the verified-live security findings to a new S-series. **HARD STOP** after this report for Andrew's review before D.1.B.

| | |
|---|---|
| Date | 2026-06-13 |
| HEAD | `9693649d8` |
| Files modified | 16 corpus + 1 verification-pass fix (CLAUDE_MD_HARDENING aliases) |
| New files | `CLAIMS_REGISTRY.md`, `docs/campaigns/security/S0_FINDINGS_SEED.md`, this report |
| Edits applied | **81** (26 DELETE · 29 CORRECT · 26 REWRITE) |
| Rows LEFT (D.2-deferred) | **91** |
| Commit | NOT committed — awaiting review |

## 1. Job 1 — CLAIMS_REGISTRY.md (keystone)

Created `docs/campaigns/doc-truth/CLAIMS_REGISTRY.md` — the single home for every load-bearing, recurring number (scope: flagged 📒 REGISTRY-CANDIDATE **and** appears in ≥2 docs or on the poison list).

- **11 VERIFIED-AT-HEAD** rows (value filled, re-checked this campaign): workspace-members (130), production-crates (~51), test-markers-total (~39,900), editor-test-markers (9,427), fluids-test-markers (2,560), fluids-loc (80,222), rust-loc-total (~892K code), kani-proofs (69), toolchain (1.89.0), dependency-versions (wgpu 25.0.2 / egui 0.32 / glam 0.30 / rapier3d 0.22 / rodio 0.17), ai-modes (7).
- **7 PENDING-D2** rows (value = PENDING-D2 + repro command, filled by D.2 measurement): agents-capacity-60fps (12,700), frame-time-1000-entities (CONTESTED), validation-checks-per-sec (6.48M), coverage-weighted (59.3%), miri-tests (977), mutation-kill-rate, dormant-loc-inventory (~200K).
- **Retired table** (deleted across the corpus, never homed): 103,500/103k, 610k, 10.4×-Unity/2.1-5.2×-Unreal and the invented Unity-9,900/Unreal-20k-50k baselines, 4,907 fluids, 128/49/82+ crates, 3,892/6,100 editor, 71+ Kani.
- **Zero FABRICATED numbers registered.** ✓

## 2. Job 3 — S0 security seed (routing only)

Created `docs/campaigns/security/S0_FINDINGS_SEED.md` — the three verified-live CODE-FINDINGs transcribed verbatim with evidence pointers, no investigation, no fixes:
- **S0-1** plaintext WebSocket server, `token != "dev"` only — `astraweave-net/src/lib.rs:535,786`
- **S0-2** LLM prompt/response `eprintln!` leak — `astraweave-llm/src/lib.rs:176-184,215-222`
- **S0-3** unconsulted Rhai `allowed_functions` allowlist — `astraweave-security/src/lib.rs:64`

Plus a note that the `sign16` weak-signature finding is STALE (HMAC-SHA256 landed). The corrections to `SECURITY_AUDIT_AND_HARDENING_PLAN.md`'s own prose are deferred to D.1.B per the prompt.

## 3. Job 2 — progress ledger (16 worst-offender files)

| File | DEL | CORR | REWRITE | LEFT |
|---|---:|---:|---:|---:|
| `docs/audits/COMPREHENSIVE_AUDIT_REPORT.md` | 7 | 7 | 2 | 8 |
| `docs/current/RENDERER_DEEP_ANALYSIS_AND_MEGALIGHTS_PLAN.md` | 3 | 0 | 3 | 6 |
| `docs/reference/Interfaces.md` | 2 | 0 | 4 | 0 |
| `docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md` | 7 | 6 | 2 | 26 |
| `docs/pbr/PBR_F_DESIGN.md` | 2 | 0 | 1 | 5 |
| `docs/current/CLAUDE_MD_HARDENING_PROPOSAL.md` | 0 | 6 | 0 | 1 |
| `docs/current/RENDERING_INFRASTRUCTURE_AUDIT.md` | 0 | 0 | 1 | 8 |
| `docs/lessons/WHAT_DIDNT.md` | 0 | 4 | 1 | 1 |
| `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md` | 0 | 0 | 1 | 7 |
| `docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md` | 0 | 1 | 1 | 7 |
| `docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md` | 2 | 1 | 1 | 9 |
| `astraweave-fluids/README.md` | 0 | 0 | 4 | 1 |
| `docs/current/IMPLEMENTATION_PLANS_INDEX.md` | 0 | 2 | 0 | 3 |
| `tools/aw_editor/EDITOR_ROADMAP_TO_WORLD_CLASS.md` | 1 | 1 | 2 | 1 |
| `docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md` | 2 | 0 | 1 | 2 |
| `docs/lessons/WHAT_WORKED.md` | 0 | 1 | 2 | 6 |
| **Total** | **26** | **29** | **26** | **91** |

Every row in the 16 files' inventory mapped to exactly one of DELETE / CORRECT / REWRITE / LEAVE. Verbatim before/after for all 81 edits is preserved in the workflow ledger (available on request).

### 3.1 Special cases handled per the prompt
- **`Interfaces.md`** (−106 lines): fabricated types (`EntityView`/`HealthView`/`ObjectiveHint`/`HazardHint`, `MaterialGraph`/`CompileResult`/`ShaderTarget`, `NetMsg`) DELETED; real-but-fabricated-body types (`WorldSnapshot`/`PlanIntent`/`Orchestrator`) replaced with **code pointers** (not hand-retranscribed). The all-fabricated "Materials & Shaders" section removed with its heading. No dangling headers.
- **`F0_GROUND_TRUTH_AUDIT.md`**: treated as a campaign audit — **one dated reconciliation note** added (F.1 superseded UnifiedSolver/DFSPH/IISPH); historical findings left intact, not deleted.
- **`astraweave-fluids/README.md`**: REWRITE-only — "production-grade/world-class" → research-stage / zero-production-consumer; caustics/foam reframed to implemented-in-crate-not-wired. No SPH/FLIP/UnifiedSolver introduced.
- **Render plans + `COMPREHENSIVE_AUDIT_REPORT.md`**: invented competitor multipliers and scorecards DELETED (Unreal/Unity/Godot/Bevy rows + 10×/10.4×/2.1-5.2×), `103k`/`103,500` RETIRED to the agents-capacity-60fps registry pointer. The genuinely-cited Unity DOTS figure (1M entities, Reddit-sourced) was LEFT byte-identical.

## 4. D.2-deferred ledger (91 LEFT rows)

Contested or unverifiable rows left **byte-identical** (no §1.2 ground-truth arbiter). D.2 runs the repro and fills the value. Grouped by file.

**`astraweave-fluids/README.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 127 | unverifiable | cargo bench -p astraweave-fluids |

**`docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 14 | contested | D.2 coverage re-measure: cargo llvm-cov (71.37% overall vs 60-70% claim) |
| 43 | contested | D.2 coverage/test-count re-measure: cargo llvm-cov -p astraweave-ai (97.39% / 103 tests) |
| 50 | contested | D.2 coverage re-measure: cargo llvm-cov (71.37% overall, 96.43% infra, 94.71% core) |
| 52 | unverifiable | Tests/benches re-measure in D.2: cargo test --workspace 2>&1 \| rg -c 'test result'; cargo bench --workspace -- --list (counts 1,545 tests / 213 ECS / 350 rendering / 103 AI / 182 benches) |
| 63 | contested | D.2 coverage re-measure: cargo llvm-cov -p astraweave-ecs (96.67% / 213 tests) |
| 70 | unverifiable | D.2 frame-time re-measure: cargo bench -p astraweave-core (2.70 ms @ 1k entities; ECS/AI/Physics/Rendering ns-us figures L71-74) |
| 86 | contested | D.2 coverage re-measure: cargo llvm-cov -p astraweave-render (65.89% / 350 tests) |
| 102 | unverifiable | D.2 perf re-measure: cargo bench -p astraweave-physics; cargo bench -p astraweave-nav (533 rigid bodies, 26k char controllers, 142k QPS, 2.44 us) |
| 104 | contested | D.2 coverage re-measure: cargo llvm-cov -p astraweave-physics -p astraweave-nav (95.07% / 94.66%) |

**`docs/audits/COMPREHENSIVE_AUDIT_REPORT.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 21 | contested | cargo llvm-cov --workspace  # re-measure 1,545 tests / 71.37% / 94.71% |
| 22 | unverifiable | cargo bench -p astraweave-render  # re-measure 2.70ms frame time / 370 FPS @ 1k entities |
| 116 | contested | rg 'ToolSpec\|register_tool' astraweave-ai/src --type rust  # re-count 37-tool vocabulary |
| 130 | contested | ls astraweave-render/src/*.rs \| wc -l  # re-count rendering modules (claimed 45) |
| 139 | unverifiable | rg 'megalights\|vxgi\|nanite' astraweave-render/src -i  # confirm MegaLights/VXGI/Nanite capability claims |
| 302 | unverifiable | rg -c 'unwrap\(\)\|panic!\|unsafe' --type rust  # re-count unwrap/panic/unsafe (2340+/158+/204+) |
| 405 | contested | cargo llvm-cov --workspace  # re-measure 1,545 tests / 26/47 crates-with-tests |
| 1128 | contested | n/a - now correct ([x] HMAC done after L192/L356 collapse); left byte-identical per plan |

**`docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 13 | unverifiable | cargo llvm-cov --workspace --summary-only  # A-(92/100) weighted grade |
| 16 | unverifiable | cargo llvm-cov -p astraweave-ecs --summary-only  # 96.67% ECS coverage |
| 40 | unverifiable | cargo llvm-cov -p astraweave-ecs --summary-only  # 96.67% ECS coverage |
| 45 | unverifiable | cargo bench -p astraweave-ecs  # 192k entity-capacity estimate |
| 81 | unverifiable | cargo bench -p astraweave-render  # MegaLights 100k+ clustered lighting figure |
| 84 | unverifiable | cargo bench -p astraweave-render  # Nanite-inspired virtualized geometry claim |
| 90 | unverifiable | cargo bench -p astraweave-render  # 4.2k-5k draw-call budget measured |
| 91 | unverifiable | cargo bench -p astraweave-render  # 1.2-1.4ms frame time / 84% headroom |
| 95 | contested | cargo llvm-cov -p astraweave-render --summary-only  # 65.89% / 350 tests |
| 127 | unverifiable | cargo bench -p astraweave-ai  # 97.9% GOAP cache hit |
| 135 | contested | cargo llvm-cov -p astraweave-ai --summary-only  # 97.39% / 103 tests |
| 153 | unverifiable | cargo bench -p astraweave-ecs  # 0.104 µs ECS core / 99.99% headroom |
| 160 | unverifiable | cargo bench --workspace  # 2.70ms @ 1k entities / 84% headroom TOTAL |
| 169 | unverifiable | N/A — CITED competitor figure (Unity DOTS, Reddit/gamedev sources); byte-identical LEAVE per plan |
| 176 | unverifiable | cargo bench -p astraweave-ecs  # 192,000 entities extrapolation |
| 179 | unverifiable | cargo bench -p astraweave-ecs  # entity-capacity verdict / 192k estimate |
| 191 | unverifiable | cargo bench -p astraweave-persistence-ecs  # ~15.5 bytes/entity ECS storage |
| 216 | unverifiable | cargo bench -p astraweave-persistence-ecs  # 1.504 ms ECS world load @ 1k |
| 244 | contested | cargo llvm-cov --workspace --summary-only  # 71.37% overall coverage tier table |
| 264 | unverifiable | cargo bench --workspace 2>&1 \| grep -c '^test'  # 182 active benchmarks / 575 planned |
| 303 | contested | cargo test --workspace --list  # 215 integration / 1,545 total test counts |
| 304 | unverifiable | ls examples/ \| wc -l  # 27+ examples count |
| 327 | unverifiable | cargo test -p astraweave-ai tool_sandbox  # 37-tool sandbox validation count |
| 348 | unverifiable | cargo bench -p astraweave-net-ecs  # 77.5 ns/entity delta encoding |
| 374 | contested | cargo test --workspace --list  # 1,545 tests aggregate |
| 699 | unverifiable | cargo llvm-cov -p astraweave-ai --summary-only && cargo bench -p astraweave-ai  # 96.43% infra coverage / 60 FPS @ 12,700 agents |

**`docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 29 | unverifiable | cargo test -p astraweave-fluids -- --list  (self-correcting test-count drift note per plan — LEAVE) |
| 61 | contested | VERIFIED ResearchFluidSystem finding accurate per plan — LEAVE; rg 'struct ResearchFluidSystem' astraweave-fluids/ |
| 178 | unverifiable | self-correcting test-count drift note per plan — LEAVE |
| 205 | contested | VERIFIED row per plan — LEAVE |
| 215 | contested | VERIFIED row per plan — LEAVE |
| 228 | contested | VERIFIED row per plan — LEAVE |
| 264 | contested | VERIFIED row per plan — LEAVE |

**`docs/current/CLAUDE_MD_HARDENING_PROPOSAL.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 530 | unverifiable | cargo +nightly miri test -p astraweave-ecs -p astraweave-math -p astraweave-core -p astraweave-sdk --lib -- --test-threads=1 (count tests) |

**`docs/current/IMPLEMENTATION_PLANS_INDEX.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 29 | unverifiable | rg '\.unwrap\(\)' --type rust -g '!*test*' -g '!benches/*' \| wc -l  (current production-path unwrap count not in D.1.A ground truth) |
| 50 | unverifiable | read astraweave-render/src/skinning_gpu.rs:242 to confirm whether the cited 'broken pipeline descriptor' line still exists |
| 179 | unverifiable | rg '\.unwrap\(\)' --type rust -g '!*test*' -g '!benches/*' \| wc -l  (dashboard 'Now' column unwrap count not in D.1.A ground truth) |

**`docs/current/RENDERER_DEEP_ANALYSIS_AND_MEGALIGHTS_PLAN.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 16 | unverifiable | cargo bench -p astraweave-render -- lod_simplification  # 68-2110 µs LOD figure, D.2 re-measures |
| 17 | unverifiable | cargo bench -p astraweave-render -- vertex_compression  # 16-29 ns / 37.5% reduction, D.2 re-measures |
| 18 | unverifiable | cargo bench -p astraweave-render -- draw_call_reduction  # 10-100× figure, D.2 re-measures |
| 23 | unverifiable | rg 'DefaultBroadPhase\|clustered' astraweave-render/src  # plan names L24 'clustered CPU-bound'; actual claim on L23. Dated plan, code-finding stale but LEAVE per plan; D.2 re-measures clustered binning |
| 62 | unverifiable | wc -l astraweave-render/src/clustered_forward.rs  # '462 lines' LoC, D.2 re-measures |
| 134 | unverifiable | cargo bench -p astraweave-render -- cluster_binning  # 0.5-2 ms binning figure, D.2 re-measures |

**`docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 31 | unverifiable | cargo bench -p astraweave-render -- lod_simplification # re-measure 68-2110 µs quadric simplification |
| 32 | unverifiable | cargo bench -p astraweave-render -- vertex_compression # re-measure 16-29 ns, 37.5% memory reduction |
| 33 | unverifiable | CODE-FINDING: status claim 'production-ready' inside dated plan; D.2 to re-judge GPU-driven culling maturity |
| 36 | unverifiable | cargo bench -p astraweave-render -- indirect_draw # re-measure 10-100x draw call reduction |
| 40 | unverifiable | CODE-FINDING: 'CPU-bound light culling' gap row stale — astraweave-render/src/clustered_megalights.rs now exists; plan is dated, LEAVE per plan |
| 50 | unverifiable | CODE-FINDING: '✅ DONE' status inside dated capability-matrix; D.2 to re-judge |
| 54 | unverifiable | CODE-FINDING: 'CPU-only' GPU Light Culling matrix row stale — clustered_megalights.rs exists; plan is dated, LEAVE per plan |

**`docs/current/RENDERING_INFRASTRUCTURE_AUDIT.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 13 | contested | manual: self-assigned 'Overall Grade: A (92/100)' is a contested-LEAVE grade; D.2 re-judges. No edit. |
| 30 | unverifiable | wc -l astraweave-render/src/clustered_megalights.rs |
| 42 | unverifiable | cargo bench -p astraweave-render (cluster config 16x16x32 / light capacity) |
| 44 | unverifiable | cargo bench -p astraweave-render (megalights dispatch/GPU-execute perf table) |
| 64 | unverifiable | wc -l astraweave-render/src/shadow_csm.rs |
| 104 | unverifiable | wc -l astraweave-render/src/ssao.rs  (NOTE: file ABSENT at HEAD — see notes) |
| 140 | unverifiable | wc -l astraweave-render/src/post.rs |
| 289 | unverifiable | sum wc -l of astraweave-render/src/{clustered_megalights,shadow_csm,ssao,post,advanced_post}.rs + shaders (aggregate ~3,700 LOC) |

**`docs/lessons/WHAT_DIDNT.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 21 | unverifiable | cargo bench -p astraweave-ecs (measure single-threaded frame time / FPS for 1000 entities; compare to claimed 2.70 ms / 370 FPS) — registry slug frame-time-1000-entities |

**`docs/lessons/WHAT_WORKED.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 114 | unverifiable | cargo run -p hello_companion --release -- --demo-all  # re-measure 12,700 agents @ 60 FPS / 18.8x / 370 FPS (slug agents-capacity-60fps PENDING-D2) |
| 147 | unverifiable | cargo test -p astraweave-ecs 2>&1 \| tail -5  # re-count '242 tests passing' |
| 165 | unverifiable | cargo bench -p astraweave-ai 2>&1 \| rg 'plan\|ns'  # re-measure 87-202 ns planning (number left untouched in rewritten line) |
| 183 | unverifiable | cargo bench -p astraweave-core 2>&1 \| rg 'validation\|checks'  # re-measure 6.48M validation checks/sec |
| 197 | unverifiable | cargo bench -p astraweave-render 2>&1 \| rg 'mesh\|memory'  # re-measure 37.5% memory reduction |
| 214 | unverifiable | cargo bench -p astraweave-math 2>&1 \| rg 'simd\|movement'  # re-measure 2.08x SIMD speedup |

**`docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 92 | unverifiable | Profile shader runtime overhead at 1080p in unified_showcase; D.2 re-measures the ~0.5ms figure. |
| 185 | unverifiable | Re-measure ~0.5ms/frame @1080p perf claim under D.2 perf pass. |

**`docs/pbr/PBR_F_DESIGN.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 12 | unverifiable | Read astraweave-render/src/terrain_material.rs:59,544 — code is 32 layers / 2112 bytes (MAX_TERRAIN_LAYERS=32), doc says '4 material layers'. Correct value not in D.1.A §1.2 ground truth; D.2 to correct from code size assert. |
| 16 | unverifiable | cargo test -p astraweave-render terrain_material -- --list (re-measure 36/36 test count); D.2. |
| 229 | unverifiable | Read astraweave-render/src/terrain_material.rs:544 — assert_eq!(size_of::<TerrainMaterialGpu>(), 2112); doc says '320 bytes'. Not in §1.2 ground truth; D.2 corrects from code. |
| 706 | contested | VERIFIED-correct per plan (wgpu 25.0); leave byte-identical. |
| 762 | unverifiable | Read astraweave-render/src/material_library.rs MAX_TERRAIN_LAYERS (=32) — doc says 'Hard-coded to 4 layers'. Not in §1.2 ground truth; D.2 corrects from code. |

**`tools/aw_editor/EDITOR_ROADMAP_TO_WORLD_CLASS.md`**

| Line | Reason | D.2 repro |
|---:|---|---|
| 104 | contested | git -C d:/AstraWeave-AI-Native-Gaming-Engine grep -c '#\[test\]\\|#\[tokio::test\]\\|#\[rstest\]' -- tools/aw_editor/ |

Dominant deferred classes: coverage %% (need `cargo llvm-cov`), runnable test counts (need `cargo test --workspace --list`), per-file LoC (need `wc -l` at HEAD), perf numbers (need `cargo bench`), and Miri/mutation counts. The contested editor sub-counts and the `1,545`-tests aggregate are here — NOT corrected, because ground truth supplies test *markers* (~39,900), not the runnable counts these claim.

## 5. Verification pass (my checks beyond the agents' ledgers)

I reviewed every diff against the gate. Findings and actions:

1. **Contested values preserved** — grepped the removed (`-`) lines for every contested figure (`1,545`, `96.67%`, `71.37%`, per-file LoC `722/534/634/964`, `36/36`, perf numbers). None were deleted; where a contested value shared a line with a corrected one (e.g. `82 crates, … 96.67% coverage`), the contested value is preserved verbatim on the new line. ✓
2. **Fabrications excised** — confirmed `10.4×`/`2.1-5.2×`, the competitor scorecards, the per-device FPS table, the invented API types, and "parity with AAA"/"competitive with UE5" are gone from the edited files (they survive only in `CLAIMS_REGISTRY.md`'s Retired table and the D0/D01 inventories — the audit trail). ✓
3. **Two concrete fabrications the sweep missed — fixed by me:**
   - `CLAUDE_MD_HARDENING_PROPOSAL.md:319-322`: a code block showing `cargo check-all`/`build-core`/`test-all`/`clippy-all` (the agent fixed the L296 *instruction* but the D01 sweep never inventoried this sibling block). Corrected to the real `cargo check --workspace` / explicit-per-crate commands.
   - `astraweave-fluids/README.md:57`: the heading `## Production-Ready Optimization` still asserted "Production-Ready" after its body was rewritten. Changed to `## Optimization (Auto-Tuning)`.
4. **No dangling headers / broken lists** from deletions (spot-checked Interfaces.md, PBR_F_DESIGN.md, EDITOR_ROADMAP). ✓
5. **`docs/journey/**` and `docs/archive/**` untouched** (`git diff --name-only` shows zero). ✓
6. **No corrected value came from outside §1.2 ground truth** — every CORRECT used a registry/ground-truth value (130, 7 modes, 8 stages, phi3:medium, HMAC-SHA256, 1.89.0, 9,427, build-clean). Per-file LoC and coverage were LEFT even though re-countable, per the "CORRECT only from §1.2" discipline. ✓

### 5.1 Open finding for D.1.B — un-inventoried sibling superlatives

The D.0.1 sweep capped each group at ~40 load-bearing claims, which **under-sampled high-frequency superlatives**. Three files carry surviving "world-class" / "world's first" / "exceeds industry standards" occurrences that were not inventory rows and so were not in any agent's action plan:
- `COMPREHENSIVE_AUDIT_REPORT.md`: L559, L665, L1022 ("world's first 100% AI-generated …" — mirrors the corrected L13).
- `EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md`: L449, L483, L635, L656, L666, L709 (mixed — some are about *competitors'* editors, which must stay).
- `COMPETITIVE_ANALYSIS_SUMMARY.md`: L11, L15, L43, L114, L217 (L15/L114 reference *competitors* — keep).

I did **not** edit these — they are outside the inventory, and several are legitimately about competitors or documentation, requiring per-occurrence judgment. **Recommendation:** D.1.B re-sweeps these three files for complete superlative coverage (distinguishing AstraWeave-superlative → delete from competitor-reference → keep). This also argues for a D.3 lint rule on provenance-free superlatives, since per-claim sampling will always miss recurrences.

## 6. git diff --stat

```
astraweave-fluids/README.md                        |  10 +-
docs/audits/COMPETITIVE_ANALYSIS_SUMMARY.md        |  14 +--
docs/audits/COMPREHENSIVE_AUDIT_REPORT.md          |  46 ++++-----
docs/audits/EXTERNAL_RESEARCH_COMPETITIVE_ANALYSIS.md |  30 +++---
docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md |   2 +
docs/current/CLAUDE_MD_HARDENING_PROPOSAL.md       |  24 +++--
docs/current/IMPLEMENTATION_PLANS_INDEX.md         |   4 +-
docs/current/RENDERER_DEEP_ANALYSIS_AND_MEGALIGHTS_PLAN.md |  18 ++--
docs/current/RENDERER_MASTER_IMPLEMENTATION_PLAN.md |   4 +-
docs/current/RENDERING_INFRASTRUCTURE_AUDIT.md     |   2 +-
docs/lessons/WHAT_DIDNT.md                         |  12 +--
docs/lessons/WHAT_WORKED.md                        |   6 +-
docs/pbr/PBR_D_EXECUTIVE_SUMMARY.md                |   6 +-
docs/pbr/PBR_F_DESIGN.md                           |  22 +----
docs/reference/Interfaces.md                       | 106 +--------------------
tools/aw_editor/EDITOR_ROADMAP_TO_WORLD_CLASS.md   |  10 +-
16 files changed, 91 insertions(+), 225 deletions(-)
```

Plus new files: `CLAIMS_REGISTRY.md`, `docs/campaigns/security/S0_FINDINGS_SEED.md`, this report. (`D0_CLAIMS_INVENTORY.md` and `D01_GAP_INVENTORY.md` are prior-phase deliverables also still untracked.)

## 7. Verification gate

- [x] `CLAIMS_REGISTRY.md` exists; every entry has a repro; zero FABRICATED numbers registered
- [x] Every row in the 16 files mapped to exactly one action; ledger proves it
- [x] Zero contested-number lines modified (grep-confirmed unchanged)
- [x] No corrected value from anywhere but §1.2 ground truth
- [x] No dangling headers / broken links from deletions
- [x] `docs/journey/**` and `docs/archive/**` untouched
- [x] S0 seed contains exactly the three findings, transcribed, no fixes
- [x] `git status`: only the registry, the 16 files, the S0 seed, and this report modified/created

## 8. HARD STOP

Stopping here. **Not committed.** Andrew reviews the diff — deletion cleanliness (is anything softened-not-deleted? — no: competitor verdicts were excised, e.g. "matches/exceeds Unity HDRP" → "lacks UE5's GPU-driven rendering") and rewrite faithfulness (does any rewrite introduce a new claim? — no new numbers or superlatives added). The §5.1 superlative-coverage gap is the main scoping decision for D.1.B.

**Forward chain:** D.1.A (this) → D.1.B (remaining in-scope files, same method + the §5.1 superlative sweep) → D.2 (fill every PENDING-D2 registry row + resolve the 91 D.2-deferred contested rows) → D.3 (regression lint + CLAUDE.md §7.x). Separately: S.1 scopes the routed security findings.