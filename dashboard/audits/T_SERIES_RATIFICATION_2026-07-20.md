# T-Series Ratification Record — 2026-07-20 (director)

> **Purpose:** the gate record for `docs/audits/E3_PREFLIGHT_2026-07.md` (commit `8232b150b`). The director read the pre-flight's §5 (SP5 disposition), §6 (biome→slot mapping table), and §7 (proposed plan + 8 open questions) and dispositioned them as recorded below on 2026-07-20. **These decisions are made** — recorded here so future beats read a document, not a chat. Committed by session T.0 (2026-07-21); do not reopen without a new director gate.

---

## 1. Editor Multi-Tool SP5 Andrew-gate: **PASS** (declared 2026-07-20)

Director's declaration: *"paint works properly; fine-tuning needed but fundamentally works."* Polish was not part of the recorded §7.4 criteria (`EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md:772-779` — click+drag routes to RegionalArchetypePanel's brush queue, not consumed by camera pan), so the gate is met.

**Consequence:** Multi-Tool **5.C closeout is unblocked** as its own doc beat (NOT part of T.0). 5.C's scope per the campaign doc §7.7, plus two additions from the pre-flight: fix the stale campaign-doc §11 status block (still reads "Sub-phase 5 — NOT STARTED" though SP5.A/`2f6f853a5` + SP5.B/`9bae816e3` landed 2026-06-06), and add a dedicated automated regression test asserting the real panel's paint `EventDisposition::Consumed` (currently only `MockActiveTool`-generic coverage exists).

## 2. Biome→slot mapping (E3-PF §6): **RATIFIED AS SHIPPED, with amendments**

| Row | Disposition |
|---|---|
| Row 2 — forest 5→1 collapse | **ACCEPTED as-is.** Forest legibility re-judged at **T.3** when tree scatter returns (floor texture is currently the only forest signal) |
| Row 4 — tundra-as-snow | **ACCEPTED as-is** |
| Row 6 — beach ≡ desert (shared `sand.png`) | **AMENDED: distinct beach material.** The acquire-and-cook path exists (AD.4's `cook_1k.py` pipeline); traced `coast_sand_01` is a candidate source. Lands in **T.1/T.2** |
| Row 7 — Ocean/River on gravel | **RE-DISPOSITIONED: gravel is ratified as honest RIVERBED.** The water defect is answered by full water-system integration (§3 below) — **no interim wet-material workaround** |
| All other rows | Ratified as shipped (`terrain_integration.rs:1250-1274`) |

## 3. Water in scope (director, 2026-07-20)

> "a true and fundamental water slot — the engine's existing water system fully surfaced in the editor as a paintable and usable material."

Beat pair: **T.W-R** (recon: WaterRenderer consumption state, editor-viewport integration path, paint-semantics design options) → **T.W** (execution against the ratified design). T.W-R is a read-only recon with its own gate; T.W is drafted only against the ratified design.

## 4. Ratified T-series sequence

**T.0** (record reconciliation — this session) → **T.1** (beach material + mapping bookkeeping) → **T.W-R** → **T.W** → **T.2** (tuning: S2 fold; data-first before any aux-resolution purchase; `expect()` hygiene rides here) → **T.3** (judging-aids revert + scatter re-enable + forest/amplitude re-judgment) → **T.G** (amplitude-finality confirmation, then golden re-bake of the ~63 rotted assertions; E3.b disposition rides here).

## 5. Open-question answers (E3-PF §7 Q1-Q8)

| Q | Answer |
|---|---|
| Q1 mapping | See §2 above |
| Q2 SP5 | See §1 above |
| Q3 amplitude finality | **Provisionally final**; confirmed at T.3's gate (post-scatter re-judgment), unheld goldens bake at T.G |
| Q4 aux-resolution vs memory | **No pre-committed purchase.** T.2 fixes data first (flat rough/AO channels, material content) and must **earn** any memory spend with evidence that data fixes are insufficient |
| Q5 Boreal balance | Tuned for **taiga-threading character** (forest threading through snow), not a percentage target |
| Q6 Desert oasis | **Deferred** — foundations before polish |
| Q7 trace-sync depth | **Full four-trace sync** (executed in T.0, this session) |
| Q8 CI workflows | **CI-workshop is a separate parallel beat** consolidating: the benchmark-budget escalation (close-out §4 item 12, trigger met per board-state §5.2), the Rust Toolchain Management workflow's undefined-cargo-alias gap (board-state §5.1), and the Rust Cache toolchain-pin handoff (close-out §4 item 2). Not part of the terrain series |

---

*Recorded verbatim from the director's 2026-07-20 dispositions by session T.0. Gate artifacts: this record + the `terrain.md` v1.2 diff.*
