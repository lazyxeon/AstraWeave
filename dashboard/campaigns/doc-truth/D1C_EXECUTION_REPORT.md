# D.1.C Execution Report — Certify the Closed-Vocabulary Class Clean

**Bounded certification pass.** Measured the residual D.1.B left unmeasured: re-grepped both vocabularies at the current working tree, gave every occurrence a verdict, fixed the ACT-class, and emitted the standing allowlist. After this, D.1 (A+B+C) commits genuinely clean. **HARD STOP** for Andrew's review.

| | |
|---|---|
| Date | 2026-06-13 |
| HEAD | `9693649d8` |
| New files | `CLOSED_VOCABULARY_TRIAGE.md`, `CLOSED_VOCABULARY_ALLOWLIST.md`, this report |
| ACT edits | **8** (across 6 files) |
| Commit | NOT committed — awaiting review |

## 1. Reconciliation (the denominator is the current grep)

| | Count |
|---|---:|
| Pre-fix grep (working tree, post-A+B) | **594** (416 A + 178 B) |
| − ACT (fixed) | 8 |
| = Post-fix grep | **588** (416 A + 172 B) |
| KEEP (all post-fix occurrences, allowlisted) | **588** |
| Un-triaged | **0** |

`594 = 586 KEEP-from-start + 8 ACT`. Of the 8 ACT: **6** vocab-B superlatives were surgically removed (B: 178→172); **2** vocab-A were corrected *in place* (`cargo check-all`→`make check-all`; the Interfaces protobuf block caveated) so their token remains in the grep but is now KEEP-class. Post-fix grep = 588, every one allowlisted. The D.1.B report's "544 kept" figure was **not** reconciled against (per the prompt — it mixed grep-occurrence and inventory-row keeps); this phase's denominator is the current grep only.

## 2. ACT-class (8) — by action

| Action | n | Files |
|---|---:|---|
| REWRITE (surgical superlative removal / honest caveat) | 6 | PBR_D_EXECUTIVE_SUMMARY (×1), EXTERNAL_RESEARCH (×2), BENCHMARK_PRODUCTION_AUDIT, EDITOR_STATUS_REPORT, Interfaces (protobuf caveat) |
| DELETE (uncited comparative sentence) | 1 | PBR_D_EXECUTIVE_SUMMARY |
| CORRECT (phantom cargo alias → real make target) | 1 | QWEN3_MIGRATION_PLAN |

Full before→after for each is in `CLOSED_VOCABULARY_TRIAGE.md` §ACT. All were §1.2-ground-truth CORRECT or surgical adjective/clause removal — no new claim, number, or superlative introduced.

### 2.1 The over-correction guard worked
Two `6 AI modes` occurrences (`PROJECT_STATUS.md:216`, `AI_ORCHESTRATION_TIPS.md:212`) were initially ACT candidates but **reversed to KEEP** on reading context: both are Phase-6 *historical records* ("6 modes" was accurate then; the 7th came later). Correcting them would falsify the record — the exact failure this phase guards against. The Makefile check also caught that `make check-all`/`build-core`/`test-all`/`clippy-all` are **real targets** (only the `cargo` aliases are phantom), sparing 5 `gh-pages/setup.md` lines from a false ACT.

## 3. KEEP (588) — verdict distribution

| Reason | n | Lifetime |
|---|---:|:-:|
| honest-dormancy | 164 | permanent |
| subject-doc | 86 | permanent |
| competitor-cited | 63 | permanent |
| production-status-contested | 58 | pending-D2 |
| subject-doc-roadmap | 42 | permanent |
| false-positive-QUICK | 40 | permanent |
| future-target | 28 | permanent |
| chat-artifact | 22 | permanent |
| contested-pending-D2 | 14 | pending-D2 |
| keep-verified | 12 | permanent |
| historical-audit-denominator | 11 | permanent |
| real-module-reference | 11 | permanent |
| historical-dated | 10 | permanent |
| benchmark/api-subject | 7 | permanent |
| make-target | 5 | permanent |
| real-client-mention | 3 | permanent |
| production-shipped | 3 | permanent |
| design-sketch-caveated | 2 | permanent |
| project-thesis | 2 | permanent |
| fiction | 2 | permanent |
| terrain-measurement | 1 | permanent |
| attribution | 1 | permanent |
| false-positive | 1 | permanent |

The remainder is overwhelmingly **honest-dormancy** (164 — the correction layer this campaign produced: traces, CLAUDE.md, F0/F1 audits, "Not QUIC", corrected "phi3 default; Hermes opt-in"), **subject-doc** (86+42 — QWEN3_* / fluids-research-plan about their own topic), **competitor-cited** (63 — named-competitor comparisons in analysis docs), and **false-positive-QUICK** (40 — "QUIC" matching "QUICK"). 72 entries are **pending-D2** (`contested-pending-D2` + `production-status-contested`) — kept byte-identical now, re-evaluated when D.2 resolves the number.

## 4. The certification (this is what D.1.B did not produce)

Re-running the closed-vocabulary lint at the post-fix working tree, **with `CLOSED_VOCABULARY_ALLOWLIST.md` applied**:

```
post-fix occurrences : 588 (416 Vocab-A + 172 Vocab-B)
allowlist keys       : 264 distinct (file, match, reason)
un-allowlisted hits  : 0
un-triaged           : 0
ACT-class remaining  : 0
```

**Every surviving occurrence has an allowlist entry with a defensible keep-reason.** Zero un-caveated AstraWeave superlatives and zero live-poison assertions remain. The closed-vocabulary class is certified clean at this working tree.

## 5. Notes for the lint spec (D.3)
- **`.zencoder/chats/`** (22 occurrences — chat-session process artifacts, like `journey/`) should be added to the D.3 lint **exclusion** scope alongside `journey/archive/doc-truth`. They are saved AI chat logs, not current-accuracy docs.
- **`make check-all` etc. are real Makefile targets** — the lint must not flag `make <target>`, only `cargo <phantom-alias>`. The Vocab-A alias entries should be scoped to the `cargo ` prefix.
- **pending-D2 allowlist entries** (`contested-pending-D2`, `production-status-contested`) carry an expiry: D.2 re-evaluates them when it resolves the underlying number.

## 6. Verification gate
- [x] Every grep occurrence has exactly one verdict; 594 = 8 ACT + 586 KEEP-from-start; 0 un-triaged
- [x] Lint with allowlist applied returns **0 un-allowlisted** (§4)
- [x] Every KEEP left byte-identical (only the 8 ACT lines changed; honest-dormancy/trace/CLAUDE lines untouched)
- [x] Every ACT used §1.2 ground truth (the alias CORRECT) or surgical removal; no new claim
- [x] pending-D2 entries tagged to expire post-D.2
- [x] `docs/journey/**` + `docs/archive/**` **untouched by the campaign** (zero content edits). *Caveat:* 3 `docs/journey/` files show modified in the working tree from the **environment's repo-wide URL-normalization linter** (`…/AstraWeave-AI-Native-Gaming-Engine/…` → `…/AstraWeave/…` in GitHub Pages links) — the same linter that touched README/GITHUB_PAGES (flagged "intentional, don't revert"). These are not closed-vocabulary edits; Andrew can stage them separately from the D.1 commit.
- [x] `git status`: the 6 ACT files + 3 new D.1.C docs (plus the environment linter's URL-normalization changes)

## 7. HARD STOP
Stopping. **Not committed.** Andrew reviews the D.1.C diff — the 8 ACT fixes (were any KEEPs wrongly acted on? is the allowlist's keep-reason defensible per entry?) — then commits the full **D.1 = A + B + C** as the certified-clean closed-vocabulary corpus.

**Forward chain:** D.1.C → **D.2** (fill PENDING-D2 registry rows + resolve D.2-deferred contested rows; flip the `pending-D2` allowlist entries as resolved) → **D.3** (regression lint: manifest + `CLOSED_VOCABULARY_LINT.md` honoring `CLOSED_VOCABULARY_ALLOWLIST.md`; add `.zencoder/chats/` to exclusions; scope alias entries to `cargo `) + CLAUDE.md §7.x. Separately: **S.1** scopes the routed security findings.