# Board State — 2026-07-19 (post-AD-campaign tail session)

Purpose: record the director's 2026-07-19 dispositions and the current board so the
terrain campaign's pre-flight reads a document, not a chat. Companion to
`AD6_CAMPAIGN_CLOSEOUT.md` (whose §4 handoff ledger this updates by reference,
append-only convention).

## 1. Director dispositions (2026-07-19, ratified — do not reopen)

1. **`examples/unified_showcase` is REMOVED from the workspace.** An aged demo; a
   proper showcase gets built later on finished foundations. Close-out §4 item 5
   ("showcase rehab") is superseded by a future item: *build a proper showcase when
   foundations are ready*. Executed this session (see the close-out's 2026-07-19
   addendum for the commit-level record).
2. **The distribution-proof ("Tier-2") consumer criterion moves from the showcase to
   the editor (`aw_editor`).** Machine core unchanged: `cargo xtask fetch-assets` →
   sha256 pins → `verify-assets` → `ci-guard`. Consumer-launch element: `aw_editor`
   launch + sample-set render (human-gated), with the `canonical_terrain_pack` loader
   tests in `tools/aw_editor` as the machine-checkable floor. Full wording: close-out
   addendum 2026-07-19.
3. **S2 "eyes hurt up close" render-quality item (close-out §4 item 3) folds into the
   terrain campaign's tuning beat (E3.3)** rather than standing alone.
4. **The terrain arc (E3 resumption) is the ratified next campaign.** A separate
   read-only pre-flight precedes it; no terrain work was done in this session.
5. **Editor Multi-Tool SP5 gate disposition is PENDING** the pre-flight's criteria
   verification.

## 2. Remaining DIRECTOR-personal tasks

1. **Submit D8** (GitHub Support LFS-purge ticket, close-out §5 text) — gh-pages is
   now pointer-free (this session), but see §3 below before submitting.
2. **External-media backup third copy** (scheduled, budget-gated; the backup
   destruction criterion of close-out §2 is unchanged).
3. **Delete the `lazyxeon/ad6-tag-validation` scratch repo** (one click; agent token
   lacks `delete_repo`).

## 3. New finding this session — `w0-pre-deprecation` tag anchors pre-rewrite history (DIRECTOR decision required; blocks D8)

Verified 2026-07-19: `git ls-remote --tags origin` shows `refs/tags/w0-pre-deprecation`
(annotated tag `1f2280af9` → commit `3a8296038`) live on GitHub. That commit is **not**
an ancestor of the rewritten `main`, and `git lfs ls-files 3a8296038` counts **91,290
LFS pointer entries** — it is the pre-rewrite history. The tag is the director-ratified
W.0 water-corpus recovery anchor (tagged 2026-06-20; cited by `docs/architecture/fluids.md`,
`water.md`, and the W-series campaign docs). The AD.6 push-set mirror carried only
`main` + `assets-v1` (`AD6_EXECUTION_OUTCOME.md`), the AD.6 recon never mentions this
tag, and the GitHub event log shows no tag push since June — it survived the force-push.

Consequences while it stands:

- A fresh clone fetches the tag and its full pre-rewrite history, so the D8 ticket's
  claim "`git lfs ls-files --all` is empty on a fresh clone" is **currently false**.
- The ~88K LFS objects are still *referenced* by fetchable history; GitHub support may
  decline the purge or the purge leaves 91,290 dangling pointers reachable via the tag.
- GitHub's repo `size` API read ~1.45 GB on 2026-07-19 vs the close-out's 776.30 MiB
  size-pack, consistent with retained pre-rewrite objects server-side.

Options (director's call — the tag was deliberately ratified as a recovery mechanism,
so its deletion is not an agent decision): (a) delete the remote tag (its corpus
remains recoverable in `C:\AD6_BACKUP`, which holds all pre-rewrite tips + tags, and
the local checkout still has it) and update the docs that cite it as a *remote*
recovery path; (b) keep the tag and amend D8's wording + accept permanent pre-rewrite
history on GitHub. Either way, **D8 is not submission-ready until dispositioned.**

## 4. Board summary after this session

- gh-pages: LFS-pointer-free at HEAD (orphan top-level `graphs/` pointers removed,
  commit `f09256b90`; live dashboard images verified HTTP 200). Close-out §4 item 9
  CLOSED.
- Close-out §4 item 11 (orphan baked sidecars): CLOSED — 36 deleted (ledger said 26;
  measured 36 = 12 materials × 3 channels; reference grep empty).
- AD5A outcome doc "AD.4 remains open" correction: NOT NEEDED — `grep -i "remains open"`
  over the doc returns nothing; the reported line does not exist.
- CI baseline: unchanged — the two named handoffs (Clippy Lint (Unwrap Prevention),
  Rust Cache Optimized Build) stay red pending their own beats; all other observed
  workflows green.

## 5. Post-push board read (appended after CI completed on `6fa70c740`)

All push-triggered workflows on `6fa70c740` (2026-07-20 UTC): **14 success** — incl.
Pack Guard, PBR Pipeline CI (with the showcase path-triggers removed), Editor CI,
Material Validation, Integration/Net Tests, Sanitizers, CodeQL, Trace Sync, Doc-Truth
Lint · **the two named handoffs red as expected** (Clippy Lint (Unwrap Prevention),
Rust Cache Optimized Build) · plus two findings:

1. **`Rust Toolchain Management` red — a baseline gap, not a new regression.** Failing
   identically on pre-rewrite `a17895904` (2026-07-03) and `5b2c6c8bd` (2026-06-29);
   it did not run on `360a4f125`, so the close-out §3 13-run read never saw it. Root
   signature: every Toolchain Validation job dies on `error: no such command:
   'check-all'` — the workflow invokes a cargo alias that `.cargo/config.toml` does
   not define. Owner: the workflow-hygiene/CI stream (same family as handoff §4
   item 2); NOT fixed in this session (outside the ratified scope).
2. **Close-out §4 item 12's escalation condition looks met.** Comprehensive CI
   Pipeline, Performance Benchmarks, and Benchmark Regression Alerts were cancelled
   again on `6fa70c740`, each running to the full second of its budget (~32/45/60
   min) — the second consecutive timeout-cancel data point since the freeze lifted.
   The "self-heal as caches repopulate" hypothesis is not confirming; candidate for
   the workflow-budget beat.

## 6. Appends (post-2026-07-19, append-only)

- **2026-07-21 (T.0): §3 RESOLVED — option (a) executed by the director 2026-07-20.**
  The remote `w0-pre-deprecation` tag was deleted from GitHub (`git ls-remote --tags
  origin` shows only `assets-v1`; safety bundle `d:\w0-pre-deprecation.bundle`,
  956 MiB, 2026-07-20 17:50; the corpus stays recoverable via the local tag +
  `C:\AD6_BACKUP`). **D8 is unblocked.** The doc sweep §3(a) required (docs citing
  the tag as a *remote* recovery path) was executed in T.0 — canonical recovery
  note: `docs/architecture/fluids.md` §0.5. First recorded: `E3_PREFLIGHT_2026-07.md` §1.
- **2026-07-21 (T.0): §5.2's candidate is ratified** — the CI-workshop beat
  (T-series ratification Q8, `T_SERIES_RATIFICATION_2026-07-20.md` §5) consolidates
  the benchmark-budget escalation + the §5.1 undefined-cargo-alias gap in the
  Rust Toolchain Management workflow + the Rust Cache toolchain-pin handoff, as a
  separate parallel beat.
