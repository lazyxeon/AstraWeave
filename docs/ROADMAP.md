---
schema_version: 1
doc_kind: roadmap
title: "AstraWeave team roadmap"
description: "Vote 6 source of truth for Now / Next / Later lanes, owners, and review gates"
doc_version: "1.2"
content_owner: Astraea
file_owner: Astranox
last_verified_commit: 3b7fd1ee630595fff796da56dfb7ba6c38d70761
last_verified_date: 2026-10-30
---

# AstraWeave team roadmap

Team **Vote 6** (Andrew signed off 2026-10-30) makes this file the roadmap source of truth together with [ENGINE_STATUS.md](current/ENGINE_STATUS.md). Git history on this path is the roadmap log. **Astraea** owns the content; route changes through her. **Astranox** owns the file. The team's internal sitrep page (sign-in required) at [astraweave-sitrep-and-roadmap](https://astraweave-sitrep-and-roadmap-7wkx.pages.grok.bot/) tracks this document. Row text is verified against `main` at [`3b7fd1ee`](https://github.com/lazyxeon/AstraWeave/commit/3b7fd1ee630595fff796da56dfb7ba6c38d70761) (2026-10-30).

Lanes: **Astrix** fixes, merges, Aravath · **Astrid** RCA, supply chain, LLM plan quality · **Annubis** test harness, playtest · **Astranox** docs, perf dashboard, release notes · **Astraea** review gates, triage, roadmap · **Andrew** GPU runner, release, CODEOWNERS, required checks.

## Now: stabilize (get main green, close the P0s, make checks able to fail)
| # | Item | Owner | Exit criterion | Issues |
|---|---|---|---|---|
| N1 | Fix PBR CI: llm dead code under the reduced feature set, plus the LOD test | Astrix (fix), Astrid (RCA done) | PBR Pipeline CI green on main 2 nightlies running; `clippy -p astraweave-llm --no-default-features -D warnings` step added | [#572](https://github.com/lazyxeon/AstraWeave/issues/572), [#446](https://github.com/lazyxeon/AstraWeave/issues/446) |
| N3 | Comprehensive CI nightly red | Astrid (RCA), Astrix (fix) | Nightly green 2 runs in a row | [#274](https://github.com/lazyxeon/AstraWeave/issues/274) (dupe [#444](https://github.com/lazyxeon/AstraWeave/issues/444) closed) |
| N4 | BlobVec double-free when Drop panics | Astrix (fix), Astraea (gate) | Fix merged with a panic-in-Drop test that passes under Miri | [#531](https://github.com/lazyxeon/AstraWeave/issues/531) (fix: [PR #585](https://github.com/lazyxeon/AstraWeave/pull/585), open) |
| N5 | macOS Ollama slow-token drop | Astrid (RCA), Astrix (fix) | Test full (macos) passes the slow-token case | [#450](https://github.com/lazyxeon/AstraWeave/issues/450) |
| N8 | Make checks able to fail: Miri, sanitizers, PR benches, loom never runs | Astrix (fix), Annubis (verify) | Each job goes red on an injected failure; no `continue-on-error` on correctness steps | [#558](https://github.com/lazyxeon/AstraWeave/issues/558), [#562](https://github.com/lazyxeon/AstraWeave/issues/562), [#563](https://github.com/lazyxeon/AstraWeave/issues/563), [#559](https://github.com/lazyxeon/AstraWeave/issues/559), [#303](https://github.com/lazyxeon/AstraWeave/issues/303) |
| N9 | Mutation Testing nightly red (stops at the first failure, `--exclude-re` matches nothing) | Astrix (fix), Astrid (findings) | Mutation nightly finishes all shards; exclude regex matches `AIArbiter::metrics` | [#576](https://github.com/lazyxeon/AstraWeave/issues/576) |
| N10 | Required checks on main + CODEOWNERS (Vote 3: editor/coverage checks able to fail once they're green) | **Andrew** decides; Astranox drafts | Branch protection lists the required checks; CODEOWNERS merged | [#422](https://github.com/lazyxeon/AstraWeave/issues/422) |

## Next: harden and finish what's half-wired
| # | Item | Owner | Exit criterion | Issues |
|---|---|---|---|---|
| X1 | Run all 28 fuzz targets nightly with persistent corpora; fix the broken LLM targets | Astrix (fix), Annubis (find/review) | 28/28 compile and run nightly; corpus survives one failing target | [#508](https://github.com/lazyxeon/AstraWeave/issues/508), [#545](https://github.com/lazyxeon/AstraWeave/issues/545), [#570](https://github.com/lazyxeon/AstraWeave/issues/570) |
| X2 | Schema-constrained LLM plans; schema and parser agree | Astrid (design), Astrix (impl) | Ollama `format` + JSON Schema on; schema-legal tools parse; malformed-plan rate tracked | [#506](https://github.com/lazyxeon/AstraWeave/issues/506), [#569](https://github.com/lazyxeon/AstraWeave/issues/569) |
| X3 | Net server correctness: rate limiter, room cap, tick owner, starvation tests | Astrix (fix), Annubis (find/review) | [#524](https://github.com/lazyxeon/AstraWeave/pull/524) and [#526](https://github.com/lazyxeon/AstraWeave/pull/526) merged (done 2026-10-29); starvation guard test fails on the unfixed loop | [#547](https://github.com/lazyxeon/AstraWeave/issues/547), [#541](https://github.com/lazyxeon/AstraWeave/issues/541), [#542](https://github.com/lazyxeon/AstraWeave/issues/542), [#566](https://github.com/lazyxeon/AstraWeave/issues/566) |
| X4 | Cap every untrusted asset and image read | Astrix | All import paths capped on decoded size; fuzz covers GLB and images | [#530](https://github.com/lazyxeon/AstraWeave/issues/530), [#465](https://github.com/lazyxeon/AstraWeave/issues/465), [#399](https://github.com/lazyxeon/AstraWeave/issues/399), [#408](https://github.com/lazyxeon/AstraWeave/issues/408), [#564](https://github.com/lazyxeon/AstraWeave/issues/564) |
| X5 | Remaining P1 deferred CI failures (Windows, editor, SDK, benches) | Astrix | Each fixed or deferred with a dated plan; Windows Test full under 60 min | [#433](https://github.com/lazyxeon/AstraWeave/issues/433), [#391](https://github.com/lazyxeon/AstraWeave/issues/391), [#434](https://github.com/lazyxeon/AstraWeave/issues/434), [#265](https://github.com/lazyxeon/AstraWeave/issues/265), [#266](https://github.com/lazyxeon/AstraWeave/issues/266), [#240](https://github.com/lazyxeon/AstraWeave/issues/240), [#239](https://github.com/lazyxeon/AstraWeave/issues/239), [#222](https://github.com/lazyxeon/AstraWeave/issues/222), [#220](https://github.com/lazyxeon/AstraWeave/issues/220), [#221](https://github.com/lazyxeon/AstraWeave/issues/221), [#224](https://github.com/lazyxeon/AstraWeave/issues/224), [#554](https://github.com/lazyxeon/AstraWeave/issues/554) |
| X6 | Supply chain: unmaintained crates, cargo-deny as errors | Astrid | `cargo deny` fails on advisories; each RUSTSEC item fixed, or raised to Andrew by issue (no new ignores; AGENTS.md) | [#303](https://github.com/lazyxeon/AstraWeave/issues/303), [#305](https://github.com/lazyxeon/AstraWeave/issues/305), [#309](https://github.com/lazyxeon/AstraWeave/issues/309), [#310](https://github.com/lazyxeon/AstraWeave/issues/310), [#321](https://github.com/lazyxeon/AstraWeave/issues/321), [#322](https://github.com/lazyxeon/AstraWeave/issues/322), [#323](https://github.com/lazyxeon/AstraWeave/issues/323) |
| X7 | SDK bench leak and the perf dashboard off gh-pages | Astrix (bench), Astranox (dashboard) | Bench stays flat in memory; dashboard publishes from `benchmark-data` | [#573](https://github.com/lazyxeon/AstraWeave/issues/573), [#540](https://github.com/lazyxeon/AstraWeave/issues/540), [#184](https://github.com/lazyxeon/AstraWeave/issues/184) |
| X8 | GPU runner for frame-budget and real-model LLM eval | **Andrew** (machine), Astrix (fix), Annubis (find/review) | gpu-frame-budget and LLM eval run on real hardware each night | [#371](https://github.com/lazyxeon/AstraWeave/issues/371), [#241](https://github.com/lazyxeon/AstraWeave/issues/241) |
| X9 | First numbered release v0.1.0, with doc versions and changelog bumped once at merge time (Vote 5) | **Andrew** (go/no-go), Astranox (notes) | Tag cut from a green main; release notes published | [#536](https://github.com/lazyxeon/AstraWeave/issues/536) |
| X10 | Docs truth: rustdoc and README claims match the code | Astranox | doc-truth-lint is clean; HNSW, broadphase and arbiter claims fixed | [#514](https://github.com/lazyxeon/AstraWeave/issues/514), [#515](https://github.com/lazyxeon/AstraWeave/issues/515), [#516](https://github.com/lazyxeon/AstraWeave/issues/516), [#534](https://github.com/lazyxeon/AstraWeave/issues/534), [#537](https://github.com/lazyxeon/AstraWeave/issues/537), [#483](https://github.com/lazyxeon/AstraWeave/issues/483), [#496](https://github.com/lazyxeon/AstraWeave/issues/496) |

## Later: from Astrid's opportunity map ([#509](https://github.com/lazyxeon/AstraWeave/issues/509))
| # | Item | Owner | Exit criterion | Issues |
|---|---|---|---|---|
| L1 | GPU-native compressed textures (KTX2/BCn) | Astrid (note), Astrix (impl) | VRAM drops on the heavy scene and stays under budget on a 4 GB card | [#507](https://github.com/lazyxeon/AstraWeave/issues/507) |
| L2 | Wire or delete the unwired render passes (SSGI, TAA, DoF, volumetrics, atmosphere) | Astrix | Each pass is live with a golden, or removed | [#226](https://github.com/lazyxeon/AstraWeave/issues/226)–[#238](https://github.com/lazyxeon/AstraWeave/issues/238), [#403](https://github.com/lazyxeon/AstraWeave/issues/403), [#404](https://github.com/lazyxeon/AstraWeave/issues/404) |
| L3 | Unify the deterministic AI tick: GOAP/Arbiter into HTN, then retire AIArbiter | Astrid | Benches use the deterministic path; AIArbiter deleted | [#383](https://github.com/lazyxeon/AstraWeave/issues/383), [#334](https://github.com/lazyxeon/AstraWeave/issues/334) |
| L4 | Production networking: one wire stack, replay protection, signed messages | Astrix (impl), Astrid (design) | One proto stack; ENGINE_STATUS no longer lists these gaps | ENGINE_STATUS L168, [#552](https://github.com/lazyxeon/AstraWeave/issues/552) |
| L5 | Aravath determinism and Windows lanes trustworthy | Astrix | aravath-determinism and aravath-windows are required and green | [#549](https://github.com/lazyxeon/AstraWeave/issues/549), [#313](https://github.com/lazyxeon/AstraWeave/issues/313) |
| L6 | Restore editor UI tests and clear editor clippy | Astrix (fix), Annubis (find/review) | Click tests back; aw_editor clippy clean | [#366](https://github.com/lazyxeon/AstraWeave/issues/366), [#248](https://github.com/lazyxeon/AstraWeave/issues/248) |

## Done since the v1.0 draft

- N2 — Scene streaming `target/` guard panic on macOS Test full — done: [#568](https://github.com/lazyxeon/AstraWeave/issues/568) closed 2026-10-29
- N6 — macOS nav timing test failing on main (Toolchain Validation), P2 — done: [#575](https://github.com/lazyxeon/AstraWeave/issues/575) closed 2026-10-29
- N7 — `sanitize_plan` panics on i32::MIN; unblocks #511 — done: [PR #511](https://github.com/lazyxeon/AstraWeave/pull/511) merged 2026-10-29, [#567](https://github.com/lazyxeon/AstraWeave/issues/567) closed

## Review gates (Astraea)
Every Now item merges only after: a fresh-agent review where reverting the fix makes the test fail, a docs clear from Astranox, and a non-blocking retest from Annubis.

## Open questions for the team
- Should N10 (required checks) wait until N1 to N3 are green, so main isn't blocked on red jobs? I propose yes.
- Is the remaining 4-item net backlog (X3: [#547](https://github.com/lazyxeon/AstraWeave/issues/547), [#541](https://github.com/lazyxeon/AstraWeave/issues/541), [#542](https://github.com/lazyxeon/AstraWeave/issues/542), [#566](https://github.com/lazyxeon/AstraWeave/issues/566)) worth one batch PR?

---

## Revision history

### v1.2 (2026-10-30) — Annubis lane roles ([#617](https://github.com/lazyxeon/AstraWeave/issues/617))

Astraea ruled yes on [#617](https://github.com/lazyxeon/AstraWeave/issues/617). Owner cells on X1, X3, X8, and L6: `Astrix (fix), Annubis (find/review)`. X8 also keeps `**Andrew** (machine)`; #617 moved only Annubis's implementation work off those rows.

### v1.1 (2026-10-30) — stale-item rulings

Astraea review on [#616](https://github.com/lazyxeon/AstraWeave/pull/616). Stamped `main` at [`3b7fd1ee`](https://github.com/lazyxeon/AstraWeave/commit/3b7fd1ee630595fff796da56dfb7ba6c38d70761) (2026-10-30). Moved N2, N6, and N7 to **Done since the v1.0 draft**. N4 Issues: `#531 (fix: PR #585, open)`. N9 Owner: `Astrix (fix)`; Issues: `#576`. X3 Exit: `#524 and #526 merged (done 2026-10-29)` with pull links; Issues trimmed to #547, #541, #542, #566. X10: removed #532. L2 Issues: `#404` only (dropped closed #405). Open question: remaining 4-item X3 batching (replacing #526 batching).

### v1.0 (2026-10-30) — Vote 6 publication

Published from Astraea's owner-confirmed draft. Base commit [`b1e995cb`](https://github.com/lazyxeon/AstraWeave/commit/b1e995cb35bf2c16a48dbf0842891142e538870a) (2026-10-29), after the [#553](https://github.com/lazyxeon/AstraWeave/pull/553) merge. Inputs: [#504](https://github.com/lazyxeon/AstraWeave/issues/504)/[#509](https://github.com/lazyxeon/AstraWeave/issues/509) (Astrid's ranked opportunity map), BS-006 triage (124 issues, 6 P0), new issues [#567](https://github.com/lazyxeon/AstraWeave/issues/567) [#568](https://github.com/lazyxeon/AstraWeave/issues/568) [#570](https://github.com/lazyxeon/AstraWeave/issues/570) [#572](https://github.com/lazyxeon/AstraWeave/issues/572) [#573](https://github.com/lazyxeon/AstraWeave/issues/573) [#546](https://github.com/lazyxeon/AstraWeave/issues/546).
