# R.0.A — Roadmap Recon: Verification Methodology + Topology + Pilot

> **Campaign**: R-series (Engine Roadmap to v1.0.0) · **Phase**: R.0.A (methodology + pilot — the gate that precedes the exhaustive fan-out R.0.B)
> **Branch**: `campaign/roadmap` (created from `origin/main` @ `5b2c6c8bd`, post-doc-truth)
> **Mode**: READ-ONLY RECON + METHODOLOGY DESIGN. No roadmap written. No fan-out executed. No code changed.
> **Status**: **RATIFIED (2026-06-29).** Methodology + verdict taxonomy + output schema, topology + 74-crate roster, v1.0 criteria (E1–E7 / A1–A6, A2/A3 bar numbers deferred non-blocking), the pilot (incl. R1/R2/R3), and the R.0.B fan-out plan are all accepted. **R.0.B is cleared to run** with the settled decisions in §5.5 — but the fan-out dispatch is its own action (it does not run as part of this commit).
> **Date**: 2026-06-29
>
> **Ratified sub-decisions (2026-06-29):** shard count = **11 (cluster-coherent)** [LOCKED]; networking v1.0 scope = **deferred to the post-R.0.B state map** [LOCKED]; per-shard effort = **Ultrathink on A–H + I, J/K one tier lower** [LOCKED]; special-handling = `sdk` / net-cluster / `aw_editor` (the last possibly its own sub-shard) [LOCKED]; `cargo test --no-run` permitted for slow LIGHT/MEDIUM dormant crates, **full `cargo test` mandatory for FULL-tier + trap-risk crates** [LOCKED]. One methodology sharpening ratified: the "own-docs production claim" is defined as a **present-tense assertion** (§1.2). The first FALSE-PRODUCTION-READY verdict in R.0.B gets **second-agent confirmation** (§4.5, §5.2).

---

## 0. Provenance & HEAD confirmation

| Item | Value |
|---|---|
| Director-referenced base | "main (post-doc-truth, `5b2c6c8`)" |
| Resolved | `origin/main` @ **`5b2c6c8bd`** "Campaign/doc truth resume (#197)" — the squash-merge of the doc-truth PR. **Identical tree** to the doc-truth branch tip `3422687f4`. |
| Local `main` | `63d0581a7` (aw_trace_sync v1.1) — **stale**, pre-merge. Not used. |
| Branch created | `campaign/roadmap` ← `origin/main` (`5b2c6c8bd`). **HEAD confirmed = `5b2c6c8bd`.** |

**Discrepancy resolved:** the director's `5b2c6c8` is `origin/main`, not local `main`. Local `main` had not fast-forwarded past the doc-truth merge. `campaign/roadmap` is based on the correct post-doc-truth state.

**Scope correction (claim-vs-reality, applied to the prompt itself):** the prompt says "~55 crates" and "26 status-unknown crates." The authoritative count from `cargo metadata` + `aw_trace_sync` on this HEAD is **133 workspace members = 59 examples + 74 non-example crates**, of which **47 are traced** and **27 are untraced** (the prompt's "26" drifted by one — `aw_doc_lint` was added during doc-truth). **The audit scope is the 74 non-example crates**, not ~55. The roster in §2.4 is the canonical, verified scope.

---

## DELIVERABLE 1 — The per-crate verification methodology (load-bearing)

The methodology produces, for each crate, a structured record with **exactly one verdict** plus evidence. It is specified so that **N independent agent contexts reach the same verdict on a same-status crate.** Every claim is treated as a hypothesis; reality is what the code does and what passing tests exercise.

### 1.1 The four checks (run identically by every shard)

Each check is a concrete command/inspection with a recorded result. `<C>` = crate name.

**Check A — Compiles.**

```
cargo check -p <C>            # records: PASS / FAIL(n errors)
```

A FAIL here short-circuits to verdict `BROKEN` (see 1.2) — fix-before-audit, not a roadmap line.

**Check B — Reachability / live callers (is it an island?).** Three independent signals; record all three:
1. **Declared-dep rdeps** (from the §2 topology graph): the set of *non-example* workspace crates that declare `<C>` as a dependency, and whether `<C>` is reachable from any **non-example binary** (tool / `aw-net-server` / lib-with-bin). Recorded as `live` / `ex-only` (reachable only via example bins) / `ISLAND` / `bin` (is itself a binary root).
2. **Source-caller grep** — the dep edge may be declared-but-unused. Confirm real usage:

   ```
   rg "<C_underscore>::" -t rust -g '!**/examples/**' -g '!<C>/**' -g '!**/tests/**' | wc -l
   ```

   Zero non-test, non-example source uses + a declared dep = **declared-but-unused dep** (the fluids→water pattern; the math pattern).
3. **Boundary-crate exception (R3, see 1.5):** if `<C>` is an FFI/SDK/`cdylib`/plugin-ABI surface, an internal-rdep count of 0 is **not** disqualifying — its callers are external by design. Confirm by checking for a C-ABI/`#[no_mangle]`/`crate-type=["cdylib"|"staticlib"]`/published-SDK intent. Record `boundary=yes/no`. A `boundary=yes` island is evaluated on its *external* contract, not internal rdeps.

**Check C — Tests: compile, pass, and exercise the claimed feature.** The deepest tier; subsumes A (a passing test is compiled) and B-source (a passing test is a caller). Four sub-results:
1. **Test targets compile** (the Path-B lesson — `cargo check` masks broken test targets; 8 test targets once compiled green while asserting against a removed API):

   ```
   cargo test -p <C> --no-run                    # or: cargo build -p <C> --tests --keep-going
   ```

   Record PASS / FAIL(target, n errors). **FAIL here with Check A PASS is the Path-B signal** — the crate "compiles" but its tests are stale/broken.
2. **Tests pass**:

   ```
   cargo test -p <C>            # record: total passed / failed / ignored, per target
   ```

3. **Tests exercise the *claimed feature*, not just scaffolding (R2, load-bearing).** Inspect the test bodies: do they assert the behavior the trace/docs claim is production (the GOAP plan output, the arbiter mode transition, the save/load round-trip, the splat blend), or only constructor/serialization smoke tests around a stubbed core? Record `feature-exercising=yes/partial/no` with file:line of the strongest feature-test. A crate with hundreds of green tests that all test scaffolding while the advertised feature is a `todo!()`/comment-stub is **not** production (the persistence-ecs result, §4.4).
4. **Stub scan** — confirm the advertised entry points have bodies:

   ```
   rg "todo!\(\)|unimplemented!\(\)|TODO|// stub|fn .*\{\s*\}" <C>/src/  # over public/advertised fns
   ```

**Check D — Claim vs reality.** Read the owning trace's front-matter `lifecycle_status` / `integration_status` (or `UNTRACED`). Record the claim and the **role** (`P` = the trace's `primary_crate`; `O` = an `owns:` member). Then state whether A–C corroborate or contradict the claim. **R1 (load-bearing): an `O` crate inherits the trace's *group* status, which is keyed on the primary crate and may not hold individually — verify each crate on its own, never by inheritance** (the math result, §4.3).

### 1.2 Verdict taxonomy (exactly one per crate; precise entry criteria)

A crate resolves to the **first** verdict whose criteria it fully meets, evaluated top to bottom.

| Verdict | Entry criteria (ALL must hold) |
|---|---|
| **BROKEN** | Check A FAILs (does not compile). *Not a roadmap status — a fix-before-audit blocker.* |
| **FALSE-PRODUCTION-READY** | (1) A production-ready claim **attributable to the crate itself** — i.e. role `P` with `lifecycle=active` AND `integration∈{wired,mixed}`, **or** an **own-docs production claim** (defined below); AND (2) verification fails that claim: `ISLAND` with `boundary=no` (Check B), **OR** test targets don't compile / tests fail (Check C1/C2), **OR** `feature-exercising=no` (Check C3 — tests don't exercise the claimed feature), **OR** the advertised core is a stub (Check C4). *The claim of production that reality contradicts is the defining feature.* **First firing in R.0.B → second-agent confirmation (§4.5).** |
| **VERIFIED-PRODUCTION** | Claim is production (role `P`, `active`+`wired`/`mixed`, or `O` individually verified), AND Check A PASS, AND Check C1+C2 PASS, AND `feature-exercising=yes` (tests assert the claimed behavior), AND Check B = `live` (≥1 non-example caller) **or** `boundary=yes` with a confirmed external contract. |
| **WIRED-BUT-UNTESTED** | Check B = `live` (used by live code), AND Check A PASS, BUT Check C is weak: `feature-exercising∈{partial,no}` or tests absent/thin for the feature (passes C1, but C2 thin or C3 not `yes`). *Wired and real, but the feature is not test-backed — a coverage gap, not a dormancy gap.* |
| **COMPILES-BUT-ORPHANED** | Check A PASS, Check B = `ISLAND` or `ex-only` with `boundary=no`, AND the crate does **not** carry an attributable production claim (role `O` with inherited status, **or** `UNTRACED`, **or** an honest non-production claim). *The false-production-ready danger zone's quieter cousin: compiles, may even be well-tested of itself, but no live caller. If it carried a `P`+production claim it would escalate to FALSE-PRODUCTION-READY.* Tests may pass (math: 195 green) — orphaning is about callers, not test count. |
| **DORMANT-HONEST** | The trace/docs honestly label it non-production (`lifecycle∈{dormant,in_design}` or `integration∈{example_only,test_only}`), AND reality corroborates (orphaned or example/test-only), AND no production claim is made. *Honestly-labelled non-production — fine. Not a defect; a roadmap input only if it's on the v1.0 critical path.* |
| **UNTRACED-UNKNOWN** | One of the 27 untraced crates **and** Checks A–C do not yet resolve it to a confident production/orphan verdict (genuinely unestablished). If an untraced crate's checks clearly show production (live + feature-tested) or clear dormancy, prefer the substantive verdict and note "was untraced." |

**Disambiguation rules baked in (from the pilot):**
- **R1 group-status:** `O`-role crates never inherit `P`'s production claim. An orphaned `O` crate carrying an inherited `active/wired` status → `COMPILES-BUT-ORPHANED` + **`CLAIM-MISMATCH` flag** (the trace must individually status the owned crate). It escalates to `FALSE-PRODUCTION-READY` only if the crate's *own* docs assert production. (math → COMPILES-BUT-ORPHANED+CLAIM-MISMATCH, not FALSE-PRODUCTION-READY, because the foundation trace's `primary` is `core`, not `math`.)
- **R2 green-tests≠production:** test count never lifts a crate to VERIFIED-PRODUCTION without Check B `live` AND Check C3 `feature-exercising=yes`. (persistence-ecs: 138 green tests → COMPILES-BUT-ORPHANED, because no live caller + auto_save/replay are stubs.)
- **R3 boundary exception:** `boundary=yes` islands (SDK/FFI/cdylib) are judged on external contract; `ISLAND` alone does not orphan them. Flag `boundary-unverified` if the external contract can't be confirmed in-repo.
- **"Own-docs production claim" — a checkable test, not a judgment (ratified sharpening, mirrors the doc-truth present-tense-vs-dated discipline).** A crate makes an own-docs production claim iff its **own** `README.md` or top-of-`lib.rs` doc-comment contains a **present-tense assertion of completeness/production-readiness** — e.g. "is production-ready", "fully supports", "implements the complete X", "X is wired into Y". Dated, past-tense, or roadmap-framed statements do **not** count — "as of 2026-03 we added", "planned", "will support", "experimental", "WIP", a changelog entry, or a `TODO` are not claims. The shard records the exact quote + file:line in `evidence`. This makes the FALSE-PRODUCTION-READY trigger (1b) and the R1 escalation ("the crate's *own* docs assert production") a grep-able boolean — `rg -i "is production|fully (supports|implements)|production-ready|complete(ly)? (implement|support)" <C>/README.md <C>/src/lib.rs` then confirm present-tense — rather than a reviewer's gut call, so two shards agree.

### 1.3 The structured output schema (one record per crate — what makes shards aggregate)

Each shard emits, per crate, a record with these exact keys (JSON or the equivalent markdown row):

```yaml
crate: <name>
shard: <A..K>
loc: <int>                      # tokei/loc of crate src
category: lib|tool|net|persist|crate
# Check A
compiles: PASS|FAIL
compile_errors: <int>
# Check B
rdeps_nonexample: <int>
reachability: live|ex-only|ISLAND|bin
source_callers: <int>           # non-test, non-example uses
boundary: yes|no
declared_but_unused: yes|no
# Check C
test_targets_compile: PASS|FAIL
tests_passed: <int>
tests_failed: <int>
tests_ignored: <int>
feature_exercising: yes|partial|no
strongest_feature_test: <file:line or "none">
stub_core: yes|no               # advertised entry point is todo!/empty
# Check D
trace: <trace_id or UNTRACED>
trace_role: P|O|-
claimed_lifecycle: <…>
claimed_integration: <…>
claim_matches_reality: yes|no|partial
# Verdict
verdict: VERIFIED-PRODUCTION|WIRED-BUT-UNTESTED|COMPILES-BUT-ORPHANED|DORMANT-HONEST|FALSE-PRODUCTION-READY|UNTRACED-UNKNOWN|BROKEN
flags: [CLAIM-MISMATCH, BOUNDARY-UNVERIFIED, PATH-B-STALE-TESTS, …]
evidence: "<commands run + file:line + test result>"
gap_to_production: "<what is missing to reach VERIFIED-PRODUCTION, or N/A>"
```

`gap_to_production` is the roadmap's raw material: for every crate that is not VERIFIED-PRODUCTION (and is on the v1.0 critical path, §3), it states the concrete delta.

### 1.4 The triage rule (depth scales with the claim)

| Claim profile | Tier | What the shard does |
|---|---|---|
| **Claimed production** (role `P`, `active`+`wired`/`mixed`; or own-docs "production") | **FULL** | All four checks, **feature-level** scrutiny (Check C3 mandatory, read the test bodies, scan for stub cores). This is where false-positives hide — spend the budget here. |
| **Claimed in_design / partial / test_only** | **MEDIUM** | Checks A, B, C1, C2; C3 only to confirm the honest label isn't hiding a live dependency. Verify the label is honest; record gap_to_production. |
| **Claimed dormant / example_only** | **LIGHT** | Checks A + B (+ C1 if cheap). Confirm the honest label is honest (orphaned / example-only / superseded). **Do not exhaustively disprove a crate that doesn't claim to be done** — confirm dormancy and move on. |
| **UNTRACED** | **MEDIUM** | Checks A, B, C1, C2, C3. Establish a substantive verdict; status was never claimed, so there is no claim to scrutinize — but there is reality to establish. |

The pilot exercises the rule: fluids gets LIGHT (confirm dormancy), ecs gets FULL (deep feature scrutiny), persistence-ecs/math get FULL/MEDIUM (the claim demands it).

### 1.5 The three pilot-derived refinements (baked into 1.1–1.4 above)

- **R1 — Verify per-crate, never by trace inheritance.** Front-matter status is keyed on `primary_crate`; `owns:` members inherit a group label that may be false at their granularity. Every shard verifies every crate individually (Check D + the R1 disambiguation rule).
- **R2 — Green tests are not production.** A crate can have 100+ passing tests and still be orphaned/stubbed. Check B (`live`) and Check C3 (`feature-exercising`) are mandatory and load-bearing; they are what stop the methodology from being a green-test rubber stamp.
- **R3 — Boundary crates are islands by design.** SDK/FFI/cdylib surfaces (e.g. `astraweave-sdk`) have external callers; `ISLAND` is not disqualifying for them. The `boundary` check carves them out.

---

## DELIVERABLE 2 — Crate-level topology graph

Derived from `cargo metadata --format-version 1` (full resolve) + cross-checked against `docs/architecture/workspace_map.html`.

### 2.1 Headline structure

- **133 workspace members** = 59 examples + **74 non-example crates** (audit scope).
- **77 binary roots** (18 non-example: the 13 tools, `aw-net-client`, `aw-net-server`, `aw_editor`, plus lib-bins `astraweave-render`/`astraweave-secrets`/`astraweave-llm-eval`/`astraweave-assets`).
- Reachability is computed two ways: **`live`** = reachable from a non-example binary; **`ex-only`** = reachable only through example binaries (weak liveness); **`ISLAND`** = reachable from no binary at all.

### 2.2 Islands (reachable from NO binary) — 9 crates, all status-suspect

| Crate | rdeps | Note |
|---|---|---|
| `astraweave-sdk` | 0 | **Boundary candidate** — FFI/SDK surface, Kani-proven; island may be *by-design* (external C-ABI callers). R3 applies — verify external contract before orphaning. |
| `astraweave-coordination` | 0 | AI coordination crate; CLAUDE.md lists it "in-design-but-tested ~5.3K." |
| `astraweave-net-ecs` | 1 (`stress-test`, itself an island) | Trace claims `unknown/unknown`. |
| `astraweave-persistence-ecs` | 1 (`stress-test`, island) | Trace `unknown/unknown`; core fns are stubs (pilot §4.4). |
| `astraweave-persistence-player` | 0 | UNTRACED. |
| `astraweave-optimization` | 0 | UNTRACED. |
| `astraweave-steam` | 0 | UNTRACED. |
| `astraweave-stress-test` | 0 | UNTRACED test harness; itself the sole "dependent" of two other islands (a dormant cluster). |
| `veilweaver_slice_runtime` | 0 | UNTRACED; 8 deps but nothing consumes it. |

### 2.3 Weak liveness (`ex-only` — reachable only via example bins) — 10 crates

`astraweave-director`, `astraweave-fluids`, `astraweave-ipc`, **`astraweave-math`**, `astraweave-net`, `astraweave-npc`, `astraweave-pcg`, `astraweave-scripting`, `astraweave-ui`, `astraweave-weaving`.

These are "live" only because a demo consumes them — not a tool or production binary. `astraweave-math` is the standout: a *foundation* crate whose sole consumer is `profiling_demo` (most crates use `glam` directly). Weak-liveness crates are prime middle-case territory for the fan-out.

### 2.4 Canonical roster (topology ⋈ trace-claims ⋈ shard) — the fan-out input table

`reach`: `live`/`ex-only`/`ISLAND`/`bin`. `role`: P=trace primary, O=owned. **74/74 rows; coverage-verified (§5.4).**

| crate | shard | cat | rdeps(all/nonex) | reach | trace | role | claim life/integ |
|---|---|---|---|---|---|---|---|
| astraweave-core | A-Foundation | lib | 41/18 | live | ecs_math_core_sdk | P | active/wired |
| astraweave-ecs | A-Foundation | lib | 21/13 | live | ecs_math_core_sdk | O | active/wired |
| astraweave-math | A-Foundation | lib | 1/0 | ex-only | ecs_math_core_sdk | O | active/wired |
| astraweave-sdk | A-Foundation | lib | 0/0 | ISLAND | ecs_math_core_sdk | O | active/wired |
| astraweave-alloc | A-Foundation | crate | 4/2 | live | ecs_math_core_sdk | O | active/wired |
| astraweave-ai | B-AI-Core | lib | 13/5 | live | ai_pipeline | P | active/wired |
| astraweave-behavior | B-AI-Core | lib | 6/4 | live | ai_pipeline | O | active/wired |
| astraweave-director | B-AI-Core | lib | 4/1 | ex-only | ai_pipeline | O | active/wired |
| astraweave-npc | B-AI-Core | lib | 1/0 | ex-only | ai_pipeline | O | active/wired |
| astraweave-dialogue | B-AI-Core | lib | 3/2 | live | ai_pipeline | O | active/wired |
| astraweave-coordination | B-AI-Core | lib | 0/0 | ISLAND | ai_pipeline | O | active/wired |
| astraweave-llm | C-AI-LLM | lib | 20/13 | live | ai_pipeline | O | active/wired |
| astraweave-llm-eval | C-AI-LLM | lib | 0/0 | live(bin) | UNTRACED | - | - |
| astraweave-context | C-AI-LLM | lib | 8/6 | live | ai_pipeline | O | active/wired |
| astraweave-prompts | C-AI-LLM | lib | 8/6 | live | ai_pipeline | O | active/wired |
| astraweave-memory | C-AI-LLM | lib | 4/2 | live | ai_pipeline | O | active/wired |
| astraweave-embeddings | C-AI-LLM | lib | 7/5 | live | ai_pipeline | O | active/wired |
| astraweave-rag | C-AI-LLM | lib | 8/6 | live | ai_pipeline | O | active/wired |
| astraweave-persona | C-AI-LLM | lib | 4/1 | live | ai_pipeline | O | active/wired |
| astraweave-ipc | C-AI-LLM | lib | 1/0 | ex-only | UNTRACED | - | - |
| astraweave-render | D-Render | lib | 21/2 | live | render | P | active/wired |
| astraweave-materials | D-Render | lib | 2/2 | live | render | O | active/wired |
| astraweave-camera | D-Render | lib | 14/2 | live | camera | P | active/wired |
| astraweave-scene | D-Render | lib | 4/2 | live | scene | P | active/partial |
| astraweave-asset | E-Asset-Terrain-Water | lib | 8/6 | live | asset | P | active/wired |
| astraweave-asset-pipeline | E-Asset-Terrain-Water | lib | 1/1 | live | asset | O | active/wired |
| astraweave-terrain | E-Asset-Terrain-Water | lib | 8/2 | live | terrain | P | active/mixed |
| astraweave-water | E-Asset-Terrain-Water | lib | 1/1 | live | water | P | active/mixed |
| astraweave-fluids | E-Asset-Terrain-Water | lib | 2/0 | ex-only | fluids | P | dormant/example_only |
| astraweave-blend | E-Asset-Terrain-Water | crate | 2/2 | live | UNTRACED | - | - |
| astraweave-physics | F-Physics-Nav-PCG | lib | 14/4 | live | physics | P | active/wired |
| astraweave-nav | F-Physics-Nav-PCG | lib | 8/4 | live | nav | P | active/wired |
| astraweave-pcg | F-Physics-Nav-PCG | lib | 1/1 | ex-only | pcg | P | active/partial |
| astraweave-gameplay | G-Gameplay | lib | 14/5 | live | gameplay | P | active/mixed |
| astraweave-quests | G-Gameplay | lib | 1/1 | live | quests | P | in_design/partial |
| astraweave-weaving | G-Gameplay | lib | 2/0 | ex-only | weaving | P | in_design/example_only |
| astraweave-cinematics | G-Gameplay | lib | 7/5 | live | cinematics | P | active/wired |
| astraweave-ui | H-UI-Audio-Input | lib | 2/0 | ex-only | ui | P | in_design/example_only |
| astraweave-audio | H-UI-Audio-Input | lib | 7/2 | live | audio | P | active/wired |
| astraweave-input | H-UI-Audio-Input | lib | 3/2 | live | input | P | active/wired |
| astraweave-net | I-Net-Persist-Sec | lib | 2/0 | ex-only | net | P | active/wired |
| astraweave-net-ecs | I-Net-Persist-Sec | lib | 1/1 | ISLAND | net_ecs | P | unknown/unknown |
| aw-net-client | I-Net-Persist-Sec | net | 0/0 | live(bin) | net_ecs | O | unknown/unknown |
| aw-net-proto | I-Net-Persist-Sec | net | 3/3 | live | net_ecs | O | unknown/unknown |
| aw-net-server | I-Net-Persist-Sec | net | 1/1 | live(bin) | net_ecs | O | unknown/unknown |
| astraweave-persistence-ecs | I-Net-Persist-Sec | lib | 1/1 | ISLAND | persistence_ecs | P | unknown/unknown |
| aw-save | I-Net-Persist-Sec | persist | 4/3 | live | persistence_ecs | O | unknown/unknown |
| astraweave-persistence-player | I-Net-Persist-Sec | crate | 0/0 | ISLAND | UNTRACED | - | - |
| astraweave-security | I-Net-Persist-Sec | lib | 4/4 | live | security | P | in_design/test_only |
| astraweave-secrets | I-Net-Persist-Sec | lib | 0/0 | live | security | O | in_design/test_only |
| astraweave-observability | J-Infra-Misc | lib | 3/3 | live | UNTRACED | - | - |
| astraweave-profiling | J-Infra-Misc | lib | 4/2 | live | UNTRACED | - | - |
| astraweave-optimization | J-Infra-Misc | lib | 0/0 | ISLAND | UNTRACED | - | - |
| astraweave-steam | J-Infra-Misc | lib | 0/0 | ISLAND | UNTRACED | - | - |
| astraweave-stress-test | J-Infra-Misc | lib | 0/0 | ISLAND | UNTRACED | - | - |
| astraweave-scripting | J-Infra-Misc | lib | 1/0 | ex-only | UNTRACED | - | - |
| astraweave-author | J-Infra-Misc | lib | 3/1 | live | UNTRACED | - | - |
| astract | J-Infra-Misc | crate | 2/1 | live | UNTRACED | - | - |
| astract-macro | J-Infra-Misc | crate | 1/1 | live | UNTRACED | - | - |
| veilweaver_slice_runtime | J-Infra-Misc | lib | 0/0 | ISLAND | UNTRACED | - | - |
| asset_signing | J-Infra-Misc | tool | 1/1 | live | UNTRACED | - | - |
| aw_editor | K-Tools | tool | 0/0 | live(bin) | aw_editor | P | active/wired |
| aw_asset_cli | K-Tools | tool | 1/1 | live(bin) | UNTRACED | - | - |
| aw_build | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| aw_trace_sync | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| aw_doc_lint | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| aw_headless | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| aw_debug | K-Tools | tool | 1/0 | live(bin) | UNTRACED | - | - |
| aw_release | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| aw_save_cli | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| aw_texture_gen | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| aw_demo_builder | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| ollama_probe | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |
| astraweave-assets | K-Tools | tool | 0/0 | live(bin) | UNTRACED | - | - |

### 2.5 Dependency clusters (the shard backbone)

The 11 shards (§5.1) are cut along trace-ownership + dep-cluster lines so that cross-crate reachability is coherent *within* a shard:

- **A Foundation** (core/ecs/math/sdk/alloc) — the most-depended-on cluster (core has 41 rdeps); everything downstream rests here.
- **B/C AI** split into AI-Core (orchestration: ai/behavior/director/npc/dialogue/coordination) and AI-LLM stack (llm/context/prompts/memory/embeddings/rag/persona + llm-eval/ipc) — together the `ai_pipeline` trace's 13 owned crates plus 2 untraced.
- **D Render** (render/materials/camera/scene) — render has 21 rdeps; the parity harness rests here.
- **E Asset/Terrain/Water** (asset/asset-pipeline/terrain/water/fluids/blend) — the authoring-asset cluster.
- **F Physics/Nav/PCG**, **G Gameplay** (gameplay/quests/weaving/cinematics — the Veilweaver gameplay cluster), **H UI/Audio/Input**.
- **I Net/Persist/Security** — the two `unknown/unknown` traces + the island cluster live here.
- **J Infra/Misc** (observability/profiling/optimization/steam/stress-test/scripting/author/astract/slice-runtime/asset_signing) — mostly UNTRACED + islands.
- **K Tools** — the 13 binaries.

---

## DELIVERABLE 3 — Concrete v1.0.0 "dogfood-ready" criteria (draft for ratification)

**Thesis:** *the editor is good enough to author Veilweaver, on top of a stable AI pipeline.* Decomposed into verifiable targets; each is a specific crate/capability that must reach a named verdict. The roadmap's gap = the criteria not yet met.

### 3.1 Pillar I — Editor good enough to author Veilweaver

| # | Criterion | Verification target | Anchor (current state) |
|---|---|---|---|
| E1 | **Scene authoring** — create/place/transform/save/load a scene with the multi-tool dispatcher | `aw_editor` VERIFIED-PRODUCTION on scene CRUD + `astraweave-scene` VERIFIED-PRODUCTION (currently `active/partial`) | Editor Multi-Tool SP5 in progress; scene trace = partial |
| E2 | **WYSIWYG render parity** — editor viewport bit-identical to runtime `Renderer::draw_into` | `tools/aw_editor/tests/render_parity_harness.rs` green + `astraweave-render` VERIFIED-PRODUCTION | **Already enforced** (Parity P.1–P.7, SHA-256 LDR equality) — likely the first criterion already met |
| E3 | **Terrain + material editing** — paint terrain & assign across the 32-layer library; persists & renders | `astraweave-terrain` (`active/mixed`) + `terrain_materials` slice VERIFIED-PRODUCTION; editor paint commands + undo wired | SP3 brush architecture complete; BC7/KTX2 cook path broken (uploads uncompressed RGBA8) — a known gap |
| E4 | **Asset pipeline** — import mesh/texture/material; the cook path produces runtime-correct assets | `astraweave-asset`+`asset-pipeline` VERIFIED-PRODUCTION; **the BC7/KTX2 cook path fixed** (currently placeholder encoder) | Terrain Asset Quality campaign flagged the broken cook path — a v1.0 blocker |
| E5 | **Play-in-editor** — author → run the AI/gameplay loop without leaving the editor | `aw_editor` ↔ AI pipeline (Pillar II) wired; a play mode that ticks the 8-stage schedule | Not yet established; fan-out must locate the seam |
| E6 | **Camera authoring** — freefly/orbit + cinematics keyframes, single canonical path | `astraweave-camera` VERIFIED-PRODUCTION + `cinematics` | Unified Camera C.0–C.9 complete (canonical `astraweave-camera`) — likely met |
| E7 | **Save/Load of authored content** — round-trip scene + game state deterministically | `aw-save` VERIFIED-PRODUCTION + a *wired* persistence path (not `persistence-ecs` island/stub) | aw-save is live (4 rdeps); persistence-ecs is ISLAND+stub — the wiring gap is a v1.0 blocker |

### 3.2 Pillar II — Stable AI pipeline

| # | Criterion | Verification target | Anchor |
|---|---|---|---|
| A1 | **Core loop** Perception→Reasoning→Planning→Action over `WorldSnapshot`/`PlanIntent` | `astraweave-ai` (`orchestrator`/`core_loop`/`tool_sandbox`) VERIFIED-PRODUCTION | ai_pipeline trace active/wired; `astraweave-ai` live (13 rdeps) |
| A2 | **Wired-path determinism** *(REVISED — see M1.3 A2 recon `992793c41`; supersedes the original "GOAP↔BT↔LLM transitions" phrasing)* — identical `WorldSnapshot` ⇒ identical `PlanIntent` for `RuleOrchestrator::propose_plan` (the `ecs_ai_plugin` production path) + `GoapOrchestrator` (the deterministic GOAP path in `hello_companion`) | the wired sync planners VERIFIED-PRODUCTION + a named A2 determinism test | **The hybrid `AIArbiter` (GOAP↔ExecutingLLM transitions + async LLM) is post-v1.0** — tested ("Phase 7 complete") but feature-gated (`llm_orchestrator` off by default), live-LLM-requiring, not production-wired |
| A3 | **Tool validation throughput** — the sandbox validates plans at the engine tick budget (≤ frame) | `astraweave-ai::tool_sandbox` VERIFIED-PRODUCTION + a benchmark establishing the throughput bar | Bench bar to be set in the roadmap; MASTER_BENCHMARK_REPORT is the home |
| A4 | **Determinism** — same snapshot ⇒ same plan (the 60 Hz single-threaded contract) | A determinism test in `astraweave-ai`/`ecs` at VERIFIED-PRODUCTION | ECS deterministic schedule confirmed (pilot §4.1); arbiter determinism to verify |
| A5 | **LLM honesty — capability deferred, honesty required** *(REVISED — see M1.3 A2 recon)* — the LLM capability defers post-v1.0 with the arbiter (no wired/production LLM path in v1.0, `llm_orchestrator` off by default), so A5 is NOT a v1.0 capability requirement; A5 requires only the **honesty constraint**: docs/traces must NOT claim a wired/production LLM path the engine lacks | no v1.0 doc overclaims the LLM as production; the "stable AI pipeline" claim rests on `RuleOrchestrator`/`GoapOrchestrator`, with the arbiter labeled in-design/post-v1.0 | the `phi3:medium`-vs-Qwen3 doc-drift folds in: don't document a dormant LLM path as live |
| A6 | **Stability bar** — the AI stack does not regress: no `ISLAND`/stub in the A1–A5 critical path | none of {ai, behavior, llm, ecs, core} below VERIFIED-PRODUCTION | math (foundation) is COMPILES-BUT-ORPHANED — confirm it is *not* on the AI critical path |

### 3.3 Foundation gate (implicit prerequisite for both pillars)

`astraweave-core`, `astraweave-ecs` at VERIFIED-PRODUCTION (✅ confirmed in pilot). `astraweave-sdk` resolved (boundary-verified, not false-orphan). `astraweave-math`'s status reconciled (orphaned but likely off the critical path — confirm and either wire or relabel).

**Ratification of Deliverable 3 (2026-06-29):** (a) E1–E7 / A1–A6 **accepted** as the decomposition. (b) The **stability-bar numbers** for A2 (arbiter) and A3 (validation throughput) are **deferred — non-blocking for R.0.B**: the audit establishes *which* crates are VERIFIED-PRODUCTION; the numeric bars are set in a later R-phase against `MASTER_BENCHMARK_REPORT` once the state map exists. (c) Networking **deferred to the post-R.0.B state map** — the net cluster (`net`/`net-ecs`/aw-net-trio, all `unknown/unknown` + islands) is audited in R.0.B but its v1.0 critical-path status is decided once its real readiness is known; it is **not** a critical-path gap in §3 today; E7's persistence path + the net cluster are marked "scope-pending-state-map."

---

## DELIVERABLE 4 — The pilot (does the methodology catch false-production-ready?)

Four crates, each run through §1's checks at the triage-appropriate tier. **All commands were executed on `campaign/roadmap` @ `5b2c6c8bd`.**

### 4.1 `astraweave-ecs` — PRODUCTION CONFIRM (FULL tier) → **VERIFIED-PRODUCTION** ✅

| Check | Result |
|---|---|
| A compiles | PASS (1.10s) |
| B reachability | `live`, **21 rdeps** (core, ai, gameplay, … depend on it) — deeply wired |
| C1 test targets compile | PASS (6 test executables built) |
| C2 tests pass | **~900+ pass, 0 fail** (448 + 152 + 36 + 28 + 27 + 25 + 22 + … across archetype/sparse_set/system_param/world_app/zero_alloc/panic_safety/stress); a handful `ignored` |
| C3 feature-exercising | **yes** — tests assert ECS behavior (archetype storage, system params, zero-alloc iteration), not smoke |
| D claim vs reality | trace `active/wired` (role O, but individually verified) — **matches** |

**Verdict: VERIFIED-PRODUCTION**, evidence-graded. The methodology confirmed production *with* evidence (live callers + feature-exercising passing tests), not by trusting the claim.

### 4.2 `astraweave-fluids` — DORMANT TEST (LIGHT tier) → **DORMANT-HONEST** ✅ (the critical test)

| Check | Result |
|---|---|
| A compiles | PASS (38.95s) |
| B reachability | `ex-only` (consumers: `fluids_demo`, `weaving_playground` — both examples); **0 non-example source uses** of `astraweave_fluids::`; the one non-example Cargo.toml hit (`astraweave-water`) is a **comment** noting the W.1 removal of the optional fluids backend |
| D claim vs reality | trace `dormant/example_only` — **matches**; `WaterRenderer` (`astraweave-render/src/water.rs`) confirmed as the live water path that superseded it |

**Verdict: DORMANT-HONEST.** **The methodology did NOT mark the known-dormant crate production** — the must-pass test. It compiles and could be made to look healthy, but Check B (no live caller, example-only, superseded) correctly holds it at non-production, and the LIGHT tier confirmed the honest label without wasting budget disproving a crate that never claimed to be done.

### 4.3 `astraweave-math` — DISCRIMINATOR (claimed foundation active/wired) → **COMPILES-BUT-ORPHANED + CLAIM-MISMATCH** ⚠️

| Check | Result |
|---|---|
| A compiles | PASS (6.45s) |
| B reachability | `ex-only`; **rdeps = 1, and it is `profiling_demo` (an example)**. The only non-example reference to `astraweave-math` is the **root `Cargo.toml` workspace declaration itself** — not a consumer. **0 source uses** of `astraweave_math::` outside the crate/examples. (Most crates use `glam` directly.) |
| C2 tests pass | **195 pass, 0 fail** (109 + 67 + 19) — well-tested *of itself* |
| D claim vs reality | trace `active/wired`, **role O** (foundation trace's primary is `core`, not `math`) — **CONTRADICTED**: claimed wired, actually orphaned |

**Verdict: COMPILES-BUT-ORPHANED + CLAIM-MISMATCH.** This is the methodology's **false-production-ready *detection* in action**: a crate that looks production by its (inherited) trace status — `active/wired` foundation, 195 green tests — but is orphaned. Per R1, because `math` is an `O` member (not the trace primary), the verdict is COMPILES-BUT-ORPHANED with a CLAIM-MISMATCH flag (the foundation trace must individually status its owned crates) rather than FALSE-PRODUCTION-READY. **Had the shard inherited the trace's group status, it would have rubber-stamped math as production — R1 is what stops that.**

### 4.4 `astraweave-persistence-ecs` — DISCRIMINATOR (claimed unknown/unknown, island) → **COMPILES-BUT-ORPHANED** ⚠️ (the green-test trap)

| Check | Result |
|---|---|
| A compiles | PASS (1m 15s) |
| B reachability | **ISLAND** — only non-example caller is `astraweave-stress-test`, itself a 0-rdep island |
| C2 tests pass | **138 pass, 0 fail** (28 + 64 + 13 + 11 + 11 + 8 + 3) — looks healthy on the test axis |
| C3 feature-exercising | **no** — the advertised core is stubbed: `auto_save_system` (lib.rs:72) is a comment-only body, replay event application (lib.rs:96) unimplemented, multiple ECS-query TODOs (lib.rs:121/125/169/361). The 138 tests exercise serialization scaffolding, **not** the claimed auto-save/replay loop |
| D claim vs reality | trace `unknown/unknown` (honest non-claim) |

**Verdict: COMPILES-BUT-ORPHANED.** **The single most important discrimination result.** A shard running only `cargo test` sees 138 green tests and rubber-stamps it. The full methodology — Check B (`ISLAND`, no live caller) **and** Check C3 (`feature-exercising=no`, stubbed core) — correctly holds it at non-production. **R2 is what stops the green-test rubber stamp.** (Trace claims `unknown`, so this is COMPILES-BUT-ORPHANED, not FALSE-PRODUCTION-READY — there is no production claim to contradict.)

### 4.5 Pilot verdict — does the methodology work?

| Test the pilot had to pass | Result |
|---|---|
| **Catch the dormant crate** (fluids → NOT production) | ✅ DORMANT-HONEST. Did not false-positive. |
| **Confirm the production crate with evidence** (ecs) | ✅ VERIFIED-PRODUCTION with 900+ feature-exercising tests + 21 live callers. |
| **Discriminate the middle cases** | ✅✅ math → COMPILES-BUT-ORPHANED+CLAIM-MISMATCH (caught the inherited-status over-claim); persistence-ecs → COMPILES-BUT-ORPHANED (caught the green-test trap). Four crates → **four distinct evidence profiles → not a uniform rubber stamp.** |

**The methodology PASSES.** It catches false-production-ready precursors, confirms genuine production with evidence, and discriminates the middle. **No blocking revision needed before fan-out.** The pilot *surfaced* three refinements (R1/R2/R3) which are now baked into §1 — the methodology was strengthened, not invalidated. One honest gap, flagged and ratified: **the pilot sample contained no clean FALSE-PRODUCTION-READY** (a role-`P` crate claiming production that fails). The worst case sampled was owned-crate-orphaned (math) — the methodology is proven to detect the *precursor* (CLAIM-MISMATCH on orphaned crates) and the *trap* (green-test orphan); whether a true FALSE-PRODUCTION-READY exists across all 74 is exactly what R.0.B determines. The `I-Net-Persist-Sec` and the `ex-only` foundation crates are where the fan-out is most likely to find one.

> **Ratified safeguard — second-agent confirmation on the first FALSE-PRODUCTION-READY firing.** Because the pilot never actually fired a FALSE-PRODUCTION-READY verdict, the verdict's *failure path* is untested by the pilot. Therefore, in R.0.B, **the first crate any shard verdicts FALSE-PRODUCTION-READY is independently re-verified by a second agent** (fresh context, same §1 methodology, same crate) before the verdict is trusted. If the two agents agree → the verdict stands and subsequent FALSE-PRODUCTION-READY firings proceed normally (the failure path is now proven once under real conditions). If they disagree → the divergence is reported to the director as a methodology-commensurability signal (the FALSE-PRODUCTION-READY entry criteria need tightening) before the aggregation is trusted. This is the doc-truth "verify the verdict the pilot couldn't exercise" move.

---

## DELIVERABLE 5 — R.0.B fan-out plan

### 5.1 Shard partition (11 shards, 74 crates — coverage-verified)

| Shard | Crates | n | FULL-tier crates (claimed production — where false-positives hide) |
|---|---|---|---|
| A Foundation | core, ecs, math, sdk, alloc | 5 | core, ecs (+ math/sdk/alloc individually, R1) |
| B AI-Core | ai, behavior, director, npc, dialogue, coordination | 6 | ai, behavior, dialogue (director/npc ex-only; coordination ISLAND) |
| C AI-LLM | llm, llm-eval, context, prompts, memory, embeddings, rag, persona, ipc | 9 | llm, context, prompts, memory, embeddings, rag, persona |
| D Render | render, materials, camera, scene | 4 | render, materials, camera, scene |
| E Asset/Terrain/Water | asset, asset-pipeline, terrain, water, fluids, blend | 6 | asset, asset-pipeline, terrain, water (fluids LIGHT) |
| F Physics/Nav/PCG | physics, nav, pcg | 3 | physics, nav (pcg MEDIUM) |
| G Gameplay | gameplay, quests, weaving, cinematics | 4 | gameplay, cinematics (quests/weaving MEDIUM/LIGHT) |
| H UI/Audio/Input | ui, audio, input | 3 | audio, input (ui MEDIUM) |
| I Net/Persist/Security | net, net-ecs, aw-net-{client,proto,server}, persistence-ecs, aw-save, persistence-player, security, secrets | 10 | net (rest unknown/in_design → MEDIUM; the FALSE-PRODUCTION-READY hotspot) |
| J Infra/Misc | observability, profiling, optimization, steam, stress-test, scripting, author, astract, astract-macro, veilweaver_slice_runtime, asset_signing | 11 | all UNTRACED/island → MEDIUM/LIGHT |
| K Tools | aw_editor + 12 tool binaries | 13 | aw_editor FULL (claimed active/wired); other tools MEDIUM |

**Why this cut:** each shard is a coherent dependency/trace cluster, so a shard verifies related crates together and the within-shard reachability is self-consistent. Heavy shards (I=10, J=11, K=13) are mostly MEDIUM/LIGHT-tier (fast); the FULL-tier load is concentrated in A–H. **Settled (2026-06-29): 11 shards.** Per-shard effort: **Ultrathink on A–H + I** (the FULL-tier + the FALSE-PRODUCTION-READY hotspot), **J/K one tier lower** (UNTRACED/LIGHT). `aw_editor` (21 workspace deps, the heaviest single crate) may be split into its own sub-shard within K if its FULL-tier scrutiny dominates K's wall-clock.

### 5.2 Per-shard instructions (identical template)

Each shard agent receives: (1) the ratified §1 methodology verbatim; (2) its crate list from §5.1; (3) the §2.4 roster rows for its crates (topology + claim pre-filled); (4) the §1.3 output schema. It then, **per crate, at the §1.4 triage tier**:
1. Runs Check A (`cargo check -p <C>`), Check C1 (`cargo test -p <C> --no-run`), Check C2 (`cargo test -p <C>`) — capturing exact counts.
2. Runs Check B grep (`rg "<C>::"` non-test/non-example) and confirms/refines the roster's `reach` value; sets `boundary` (R3).
3. For FULL-tier crates, reads the strongest test bodies for Check C3 (`feature-exercising`) and scans for stub cores (C4).
4. Reads the trace front-matter for Check D; applies R1 (no inherited status). For any FALSE-PRODUCTION-READY trigger, captures the own-docs present-tense quote + file:line per the §1.2 definition.
5. Emits one §1.3 record per crate, with `verdict`, `flags`, `evidence` (commands + file:line + counts), `gap_to_production`.

**Test-run cost policy (settled 2026-06-29):** `cargo test --no-run` (compile-only) is permitted for **slow LIGHT/MEDIUM-tier dormant crates** (rely on Check B + C1 for the verdict); **full `cargo test` is mandatory for all FULL-tier crates and all trap-risk crates** (any crate with a green-test surface that could mask a stubbed core, à la persistence-ecs).

**Second-agent confirmation (settled 2026-06-29):** the **first** FALSE-PRODUCTION-READY verdict produced by any shard is re-verified by an independent second agent before it is trusted (see §4.5). The aggregator holds the first such verdict as PROVISIONAL until the confirmation returns.

**Output contract:** a shard returns a JSON array of records (one per its crates) + a one-line coverage assertion listing the crate names it covered. No prose beyond evidence strings.

### 5.3 Aggregation plan

1. Concatenate the 11 shard arrays → the **engine-wide state map** (74 records).
2. Group by verdict → the headline distribution (how many VERIFIED-PRODUCTION / WIRED-BUT-UNTESTED / COMPILES-BUT-ORPHANED / DORMANT-HONEST / FALSE-PRODUCTION-READY / UNTRACED-UNKNOWN).
3. Join against Deliverable 3's E1–E7 / A1–A6 critical-path crates → the **v1.0 gap list** (every criterion crate not at VERIFIED-PRODUCTION, with its `gap_to_production`). *This is the roadmap's spine — produced in a later R-phase, not R.0.B.*
4. Collect all `CLAIM-MISMATCH` flags → the **trace-correction backlog** (front-matter that over/under-claims; e.g. the foundation trace must individually status math/sdk).

### 5.4 Verification-of-aggregation (no crate falls between shards)

- **Pre-run (done):** §2.4 roster generated from `cargo metadata`; shard union computed; **`roster − ⋃shards = ∅` and `⋃shards − roster = ∅` and no duplicates** (verified by `roster.py`). The 74-crate roster is the canonical denominator.
- **Post-run:** aggregator asserts `{crate names across all 11 shard outputs} == {74 roster names}` exactly. Any crate present in the roster but absent from outputs = a coverage hole → re-dispatch that crate. Any crate with two records = a partition bug. Any record whose `crate` is not in the roster = scope leak (e.g. an example slipped in).
- **Determinism guard:** because the methodology is command-driven, two shards re-running the same crate must produce the same `compiles`/`tests_passed`/`reachability`. A spot re-run of 2–3 crates by a second agent (commensurability audit) confirms shards are not diverging before the full map is trusted.

### 5.5 Director decisions for R.0.B — ALL SETTLED (2026-06-29)

1. **Shard count:** **LOCKED — 11 (cluster-coherent).**
2. **Model/effort:** **LOCKED — Opus-4.8-Ultrathink on shards A–H + I; J/K one tier lower** (UNTRACED/LIGHT crates don't need the top tier).
3. **Special-handling crates:** **LOCKED —** `astraweave-sdk` (R3 boundary: verify external contract, do not orphan on internal-rdep=0); the `I-Net-Persist-Sec` `unknown/unknown` cluster (FALSE-PRODUCTION-READY hotspot — Ultrathink, extra scrutiny); `aw_editor` (21 workspace deps — may become its own sub-shard within K, full `cargo test`).
4. **Networking in/out of v1.0:** **LOCKED — deferred to the post-R.0.B state map.** Shard I is still fully audited (MEDIUM tier, FULL-tier scrutiny on any crate whose checks suggest a real production claim); its v1.0 critical-path weight is set after the state map exists.
5. **Test-run cost ceiling:** **LOCKED —** `cargo test --no-run` permitted for slow LIGHT/MEDIUM dormant crates; **full `cargo test` mandatory for all FULL-tier and trap-risk crates** (green-test surfaces that could mask a stubbed core).
6. **Second-agent confirmation:** **LOCKED —** the first FALSE-PRODUCTION-READY firing is independently re-verified before it is trusted (§4.5, §5.2).

---

## Gate — RATIFIED (2026-06-29)

| # | Deliverable | Disposition |
|---|---|---|
| 1 | **Methodology + verdict taxonomy + output schema** (§1) | **Accepted** — precise enough to fan out. One sharpening folded in: "own-docs production claim" defined as a checkable present-tense assertion (§1.2). |
| 2 | **Topology + shard partition** (§2, §5.1) | **Accepted** — coverage-verified 74/74; 11 cluster-coherent shards. |
| 3 | **v1.0.0 criteria** (§3) | **Accepted** — E1–E7 / A1–A6; A2/A3 stability-bar numbers deferred (non-blocking); networking deferred to state map. |
| 4 | **Pilot results** (§4) | **Accepted** — caught dormant fluids ✅, confirmed ecs with evidence ✅, discriminated math + persistence-ecs ✅✅; R1/R2/R3 ratified. Honest gap (no clean FALSE-PRODUCTION-READY fired) → second-agent confirmation safeguard added (§4.5). |
| 5 | **R.0.B fan-out plan** (§5) | **Accepted** — all six §5.5 decisions settled. |

**Headline:** the methodology, proven on four known-status crates, **correctly catches false-production-ready precursors and does not false-positive the dormant crate** — and it is command-driven enough that parallel shards will be commensurable. **R.0.B is cleared to run.** The fan-out dispatch is its own action (the big parallel run gets its own go) — it does not execute as part of the R.0.A close commit.
