# AD Campaign Close-Out — the asset-distribution series, complete

**Date:** 2026-07-18. **Terminal beat:** AD.6 (history rewrite, force-pushed 2026-07-17, rung-3 PASSED). **Full evidence chain:** `AD6_EXECUTION_OUTCOME.md` (execution), `AD6R_REWRITE_RECON.md` + `docs/audits/ad6r/` (recon + machine-readable inputs), and the per-beat outcome docs (AD.1 → AD.5.A). This document is the campaign's final deliverable, gate-reviewed by the director.

---

## 1. Final figures ledger

| figure | before | after |
|---|---|---|
| Fresh-clone `.git` | ~4.8 GiB git objects **+ 48.25 GB LFS payloads** (referenced; 88,130 objects) | **1.4 GB, no LFS anything** (measured GitHub clone, 664 s) |
| Repo size-pack (server-side push pack) | 4.73–4.8 GiB | **776.30 MiB** |
| Tracked files at HEAD | 119,223 | **26,677** |
| LFS pointers in history | 91,329 at HEAD / 88,130 unique objects | **0** |
| History paths purged | — | **104,406** (+ 4 surgical blob-version strips) |
| Asset distribution | committed LFS blobs | **19 release packs, sha256-pinned, `cargo xtask fetch-assets`** (starter 169–226 s; `--all` 60–69 min, 19/19 pins verified on every run) |
| `verify-assets` | 90,182 refs, 10-13 unmanaged warnings | **90,172 refs, 0 warnings** |
| `aw_asset_cli validate assets/materials` | 82 failures | **10** (the pre-existing arrays.toml nits — handoff §4) |
| Case-collision hazards (`Fabrics/`, `Tracks/`, `Loops/`) | 87 files, breaks case-sensitive clones | **absent** (purged; verified 0/0/0 on the fresh clone) |
| GitHub main / tag | `a17895904` / `5b2c6c8bd` | **`360a4f125`** / **`2f26ea2c`** (release published, 19 assets, URLs 200) |

## 2. Backup manifest + destruction criterion (NOT executed)

`C:\AD6_BACKUP` (Intel Optane SSD — physically distinct from the D: working disk): pristine pre-rewrite mirror (all 11 local branch tips + tags, ref-tip-identical, incl. `c4748b09c`) + complete LFS store copy (**88,588 objects / 53.717 GB, 10/10 spot content-hashes verified** across size classes) + the 19 staging zips (3/3 spot pins verified) + the live release itself.

**Destruction criterion (restated, standing):** the backup dies only after (a) the post-push GitHub fresh-clone Tier 1+2 pass — **done, green**; (b) the director's rung-3 — **done, PASS**; (c) D8 completes non-destructively (19/19 release URLs re-verified 200 after the purge) — **pending**; **and (d, director addendum) a third copy exists on external media** — pending, director task (§4).

## 3. Post-AD.6 CI baseline (the measuring stick for future regressions)

Final full-board read on `360a4f125` (13 runs): **8 success** — Material Validation (EX-6 fix confirmed loading + running), Pack Guard (the permanent anti-re-bloat rail, green on both live runs), Integration Tests, Net Tests, Sanitizers, SDK/C-Harness, CodeQL, OpenSSF · **2 failure = the two named handoffs (§4, expected red until their beats land)** · **3 cancelled** — Comprehensive CI + both benchmark workflows: cold-cache timeouts (jobs ran to the second of their 30/45-min budgets on freeze-expired caches; Windows lane passed in 4 min). Expected to self-heal as caches repopulate; escalates to a workflow-budget beat only if red persists warm.

**The stated baseline: every workflow green EXCEPT Clippy Unwrap-Prevention and Rust Cache Optimized Build.** Any other red is a new regression.

## 4. Handoff ledger (each item to its owning stream)

| # | item | owner / beat |
|---|---|---|
| 1 | `astraweave-terrain` production `expect()`s (spline_types.rs E3-era + regional_archetype_mask.rs + noise_gen.rs) — Clippy Unwrap-Prevention red | code-hygiene beat |
| 2 | `dtolnay/rust-toolchain@stable` ignores the repo's 1.89.0 pin (Rust Cache fmt failures on byte-identical blobs) | workflow-pin beat |
| 3 | S2 "eyes hurt up close" (NORMAL_XY_STRENGTH, hex-tile rotation, traced-9 flat rough/AO) | terrain-quality stream |
| 4 | 10 `arrays.toml` "Missing 'name' field" validator nits | hygiene / cook-path beat |
| 5 | unified_showcase rehab (orphan sources purged; scene modernization) | showcase stream |
| 6 | Option A — the 21-material palette loader (Option C remap shipped in AD.5.A) | post-campaign engine item |
| 7 | materials-src future re-cut (fold in the 9 ARM-order source fixes per AD.4.A §5b option ii) | next pack revision |
| 8 | **Backup third copy on external media** (per §2d) | **DIRECTOR** |
| 9 | gh-pages rebuild (orphan branch carries 7 LFS pointer entries / 4 OIDs — dashboard images 404 after D8) | docs-site beat, before/with D8 completion |
| 10 | `lazyxeon/ad6-tag-validation` disposable repo deletion (token lacks `delete_repo`) | DIRECTOR (one click) |
| 11 | 26 orphan `baked/*.ktx2.meta.json` metadata files (payloads purged) | next hygiene sweep |
| 12 | Comprehensive CI / benchmark cold-cache budgets — watch; escalate only if red warm | CI stream (conditional) |

## 5. D8 — LFS storage reclamation (as-finalized text; submission = director via support.github.com)

> **Subject: Git LFS storage reclamation after complete LFS removal — lazyxeon/AstraWeave**
>
> We have fully removed Git LFS from this repository via a history rewrite (git filter-repo + git lfs migrate export, force-pushed 2026-07-17). The current history contains zero LFS pointers (`git lfs ls-files --all` is empty on a fresh clone), no `.gitattributes` LFS rules exist, and no workflow references LFS. Per "Removing files from Git Large File Storage," the now-unreferenced LFS objects continue to count against storage; since we cannot delete-and-recreate the repository (its releases, issues, and PR history must survive), we request purging all LFS objects for this repository.
>
> Our accounting expects roughly **48.25 GB** of referenced LFS storage (measured locally from the complete object store before the rewrite: 88,130 objects, 48,245,794,652 bytes). The current metered-billing UI does not expose a point-in-time stored-GB figure; the Git LFS storage SKU shows a prorated accrual of ≈$1.70 for Jul 1–18, 2026 (≈$0.10/day), consistent with roughly 43–50 GB stored at published GB-month rates. (An older internal note said ~22 GB; our measurement and the current accrual both contradict it, so we consider 48.25 GB the operative figure.) LFS bandwidth for this repository has been flat since ~Jul 10 — nothing fetches LFS objects anymore.
>
> Please confirm before purging that this will NOT affect: (1) the published release `assets-v1` and its 19 uploaded assets (release assets, not LFS objects — they must remain publicly downloadable at their `/releases/download/` URLs), and (2) repository issues and pull-request history.

Post-D8: re-verify all 19 release URLs return 200 (pre-D8 baseline: 19/19), then §2's criterion advances.

## 6. Lessons index (the campaign's methodology yield)

1. **Empty-filter test runs prove nothing** — `cargo test <filter>` exits 0 on zero matches; always cite the `N passed` line and the filter match count. (AD.4)
2. **Dry-run direction matters** — `git lfs push --dry-run` verified the wrong direction; the corrected backup check is OID-set-vs-store enumeration. (AD.2.A)
3. **Object-level enumerations dedupe paths** — `git lfs ls-files --all` AND `git rev-list --objects` both collapse byte-identical content; only diff-based (`git log --all --name-only --no-renames`) enumeration is path-complete. And any size/provenance claim about "a file" must be **joined to a commit** before it grounds a disposition. (AD.6.R; EX-5)
4. **ARM/MRA channel keys must be evidenced per file, never pattern-copied** — measure R/B means; a `_mra` name does not mean M-R-A order. (AD.4.A D1; AD.5.A Fixes 1/4)
5. **Directory-exclusive asset ownership was empirically false** — ownership is file-exact via committed packlists; cohabitant keeplists carry subset semantics and are rails, not disposition authorities. (AD.5; AD.6 EX-1)
6. **Platform constraints need local postcondition mirrors** — GitHub's 100 MB pre-receive cap binds plain blobs that were invisible as LFS pointers; the permanent ≥95 MiB history scan (P8) now models it. (AD.6 EX-5)
7. **"YAML-parses" is not "schema-valid"** — git-level postconditions do not exercise the Actions loader; an empty `with:` block loads as null YAML and a zero-jobs workflow failure. **Standing postcondition: any beat editing a workflow file runs `actionlint` before committing.** (AD.6 EX-6, director-ratified)
8. **A provider incident makes credentials untestable** — 503s during GitHub's REST degradation made valid tokens read as invalid; check githubstatus.com before concluding anything about auth, and never push a rewrite into a degraded window. (AD.6 EX-2)
9. **Validation environments must match production visibility** — a private test repo's release URLs 404 where the public repo's return 200; the posture-3 validation was only faithful after flipping the disposable repo public. (AD.6 EX-3)
10. **Install packs only AFTER final git state** — the AD.5 rehearsal's rebase-after-install clobbered installed packs; sequencing rail honored throughout AD.6.
11. **Wrong diagnoses stay in the record** — EX-5's misdiagnosis → catch-by-execution → correction sequence is preserved by append, not rewrite; the catch mechanism (the v2 build's own size assertion) is the audit trail's proof of function. (director-ratified)

## 7. Campaign arc (one paragraph for the history)

AD.0 proposed a ratified sample set from a 91K-file audit; AD.1 traced provenance (Poly Haven/Kenney/ambientCG CC0 chains, quarantine for the untraceable); AD.2 built the fetcher and proved the release mechanism; AD.2.A verified the backup store; AD.3 cut and uploaded the 18-pack partition (AD.3.R ratifying file-exact ownership); AD.4 re-cooked the quarantine-destined sample slots from traced sources (derived_1k) and cut materials-src; AD.4.A diagnosed the render gate to root cause (D1/D2 channel fixes landed, S1 palette decision ratified); AD.5 built the transactional original-path installer with journal crash-recovery and rehearsed Tier-2 live; AD.5.A closed the consumer-hygiene gaps (forest re-point, showcase degradation, the 7-of-21 palette remap, ARM annotations); AD.6.R rehearsed the irreversible operation end-to-end on a scratch mirror and measured everything; AD.6 executed it — 104,406 paths purged, every surviving pointer materialized, force-pushed behind a twice-re-verified freeze gate, verified by machine battery and human rung-3 on a 1.4 GB fresh clone that builds, fetches, and renders. The repository is LFS-free, the assets are release-distributed and rail-guarded, and the history carries the whole forensic trail.

---

## Addendum 2026-07-19 — Tier-2 consumer criterion moves to the editor; showcase removed (director-ratified)

Two director decisions of 2026-07-19, recorded here so the close-out stays the authoritative reference:

1. **`examples/unified_showcase` is removed from the workspace** (ordinary `git rm` commit; no history operation). It was an aged demo; a proper showcase gets built later on finished foundations. Handoff §4 item 5 ("unified_showcase rehab") is **superseded** by a future item: *build a proper showcase when foundations are ready*.
2. **The distribution-proof ("Tier-2") consumer criterion moves from the showcase to the editor (`aw_editor`).** The machine core of Tier-2 is unchanged: `cargo xtask fetch-assets` → sha256 pins → `verify-assets` → `ci-guard`. The consumer-launch element is now: **`aw_editor` launches and renders the sample set (human-gated)**, with the `canonical_terrain_pack` loader tests in `tools/aw_editor` (`src/viewport/canonical_terrain_pack.rs` `mod tests`, e.g. `loads_grassland_pack_when_present`, plus the `tests/render_parity_harness.rs` suite that drives `load_canonical_terrain_pack` on both editor and engine paths) as the machine-checkable floor.

Append-only per the project's standing correction convention; §4 item 5's original text above is preserved as the record of what was.

---

## Addendum 2026-07-21 — §4 item 1 correction (T.0 record reconciliation; facts re-verified at `8232b150b`)

§4 item 1's text ("`astraweave-terrain` production `expect()`s (spline_types.rs E3-era + regional_archetype_mask.rs + noise_gen.rs) — Clippy Unwrap-Prevention red") is wrong in two ways; original text preserved above per convention:

1. **Locations.** The production-path `expect()`s are **six, all in `astraweave-terrain/src/spline_types.rs`** (`:571`, `:590`, `:605` in `climate_driven_spline_set` + `:637`, `:647`, `:652` in `archetype_spline_set` — each unwraps `Spline1D::from_control_points` on hardcoded control points; the file's only other `expect()` at `:1003` sits after the `#[cfg(test)]` boundary at `:762`). `regional_archetype_mask.rs` and `noise_gen.rs` were misattributed — every `expect()` in those files is `#[cfg(test)]`-only (boundaries at `:742` and `:1138` respectively).
2. **Attribution.** The Clippy Unwrap-Prevention workflow (`.github/workflows/clippy-unwrap-prevention.yml`) **did not scan `astraweave-terrain`** at the verification commit `8232b150b` — the crate appeared in neither the P0 matrix (`:22-33`) nor the warn-only P1 matrix (`:109-113`). Its standing red therefore has a different cause and is owned by the **CI-workshop beat** (T-series ratification 2026-07-20, Q8), not by the terrain crate. Fixing the six spline `expect()`s will not flip this workflow green. *Delta 2026-07-21:* the CI-workshop commit `1df55f19f` ("ci: retire workflow exceptions" — toolchain pins to 1.89.0, the toolchain-management undefined-alias fix, benchmark budgets) subsequently **added `astraweave-terrain` to the warn-only P1 matrix**, so the crate is now scanned in warn mode; the attribution conclusion is unchanged (P1 does not gate, and the P0 red cause remains outside this crate).

The hygiene fix itself stays queued in terrain beat **T.2** (`T_SERIES_RATIFICATION_2026-07-20.md` §4). Full derivation: `E3_PREFLIGHT_2026-07.md` §4.4.

*CI-W amendment 2026-07-21:* run `29794346156` proves P0 `astraweave-render` transitively linted its `astraweave-terrain` dependency and failed on those six findings; the missing `--no-deps` was the mechanism, so fixing the six findings would have flipped that P0 lane. CI-W now limits every lane with `--no-deps` and keeps terrain explicitly visible in warn-only P1.
*Director amendment 2026-07-22:* Per `CI_WORKSHOP_2026-07.md` §4, the P0 Clippy red **was caused by the six `astraweave-terrain/src/spline_types.rs` `expect()`s**, reached when `astraweave-render` transitively linted its dependency because the workflow omitted `--no-deps`; the earlier conclusion that the red was “not caused by the terrain crate” is superseded.
