# AD.6 — Execution Outcome (the terminal beat)

**Session:** AD.6 (2026-07-16, continuing). **Ratified plan:** `docs/audits/AD6R_REWRITE_RECON.md` + director's slate (2026-07-16). **Rule in force:** nothing remote mutated before the Phase-5 director GO (sole pre-authorized exception: the disposable-repo tag validation, which never touches the canonical repo).

## Phase 0 — Preconditions (ALL PASS)

| # | check | result |
|---|---|---|
| 0.1 | Canonical remote | `lazyxeon/AstraWeave` confirmed (`gh repo view`); local `origin` already canonical; auth: **green** — after a real GitHub incident ("Degraded REST API Availability", githubstatus.com, resolved this session) which made both gh-keyring and GCM tokens return 503/`invalid`; post-recovery `gh api user` → `lazyxeon`, GCM token HTTP 200. The incident-vs-credentials distinction cost ~1 h and is recorded as finding EX-2. |
| 0.2 | Freeze-verify | `git ls-remote origin main` = `a17895904` — unchanged, no unexamined commits. |
| 0.3 | gh-pages | TRUE ORPHAN (no merge-base with HEAD) **but carries 7 LFS pointer entries / 4 unique OIDs** (`Astraweave_logo.jpg` + 3 dashboard graphs, duplicated at 2 path roots) → **FLAGGED: rebuild is a post-push step**; those images 404 on the docs site after D8 unless the dashboard regenerates them as plain files. |
| 0.4 | Concept-doc extraction | `docs/history/original-concept-2025.md` (103 lines, provenance header) committed `c4748b09c`. The Windows-invalid original path still purges. |
| 0.5 | **Tag-retarget validation (posture 3): PASS.** | Disposable public repo `lazyxeon/ad6-tag-validation`: baseline release download HTTP 200 → orphan-rewrite force-push of branch AND `git tag -f` + force-push of the tag → release stayed `draft:false`, assets intact, **unauthenticated `/releases/download/v1/` HTTP 200 byte-identical**, remote tag re-anchored to the rewritten commit. Cleanup blocked: token lacks `delete_repo` scope — **director: delete `lazyxeon/ad6-tag-validation` manually** (public, one junk file + tiny release). |
| 0.6 | Backup precondition | OID re-run: referenced 88,130 / store 88,588 / **missing 0** — no drift from the recon. |
| 0.7 | Local state | tree clean; 11 local branch tips recorded below (they live on in the backup only). |
| 0.8 | Disk | Two physical disks confirmed (D: T-FORCE 1TB / C: Intel Optane+477GB SSD). C: free 119,208,038,400 B (111 GiB) ≥ backup ~59 GiB. **Destination C:\AD6_BACKUP self-selected — the brief said director-designated but designated none; flagged.** |

**Local branch tips (preserved in backup only):** campaign/doc-truth-resume `3422687f4`, campaign/fluids-f2 `2ab548953`, campaign/fluids-f3 `0789248c3`, campaign/fluids-f3s `2a8a29dea`, campaign/roadmap `1ed0e4cf4`→`c4748b09c` (after 0.4), campaign/water-successor `f5d40a3c7`, chore-org-root-files-{JnWaW `af44f664c`, jsvlC `d04983c5f`, lvlzs `8ce9a30cd`}, chore-organize-root-files-u2RyJ `8e259f3b8`, main `63d0581a7`.

## Phase 1 — Backup manifest (VERIFIED)

| item | location | size | verification |
|---|---|---|---|
| Mirror clone (all refs) | `C:\AD6_BACKUP\astraweave-mirror.git` | 4.8 GiB | ref tips **identical** to source (incl. `c4748b09c`), real cross-volume copy (111 s) |
| LFS store copy | `C:\AD6_BACKUP\lfs-objects` | 53.717 GB, **88,588 objects** (robocopy 36:32, 1.5 GB/min; rc=1 = success) | count exact; **10/10 spot content-hashes OK** across size classes (55 B → 8,226,603,008 B) |
| AD.3/AD.4 staging zips | `d:/tmp/ad3_staging/zips` + `d:/tmp/ad4_staging` | 19 zips | 3/3 spot pins MATCH (hdri-catalog `3fe3df8e…`, kenney-audio `a6ec7a58…`, materials-src `652458f3…`) |
| Live release | `assets-v1` on GitHub | 19 assets | published, `immutable:false` (recon API check) |

**Destruction criterion (recorded, NOT executed):** only after the post-push GitHub fresh-clone Tier 1+2 pass AND D8 completes non-destructively (all 19 release URLs re-verified 200).

## Phase 2 — Final purge list (assembled + verified)

Components (each from the committed `docs/audits/ad6r/` artifacts + packlists): pack 90,013 · quarantine 1,112 · hygiene 583 · orphan 13 · windows-invalid 1 · history-only 11,861 · cohabitants 750 · fake-ktx2 85 · Unity-era .mat/.prefab 66. Sum 104,484 − union **104,406** = 78 overlap: 71 (quarantine∩hygiene, the recon class) + 1 (windows-invalid ⊂ history-only) + 6 (Symphonie cohabitants ∩ Unity-202).

**Safety battery:** purge∩sample = 0 · purge∩retained = 0 · `docs/history/` in purge = 0 · at-HEAD purge outside known components = 0 · keeplist join: **residue 0** — with finding **EX-1**: keeplist∩purge = **612**, not the brief's implied 750-follower model (only 10 of the 750 cohabitants are keeplist entries; the other 602 = 541 ratified-quarantine + 60 ratified-Unity/ktx2 + 1 ratified-hygiene that the mechanically-generated ci-guard keeplist had swept in at generation time). Every keeplist entry being purged is a ratified-component member; the keeplist is a rail artifact, not a disposition authority, and regenerates post-rewrite.

**Predicted post-rewrite HEAD file count: 26,678** (119,223 now − 92,545 at-HEAD purge; HEAD grew +13 over the recon = the committed ad6r artifacts + recon doc + concept doc — foots exactly).

## Phase 3 — The rewrite

**Executed on the push-set mirror** (`d:/tmp/ad6_exec/mirror.git`: `refs/heads/main` = `c4748b09c`, `refs/tags/assets-v1` = `5b2c6c8bd`; store hardlinked; local only — remote untouched).

| step | wall clock | result |
|---|---|---|
| filter-repo purge (104,406 paths) | **33:22** (2,002 s) | objects 4.8 GiB → 349 MiB |
| lfs migrate export `--everything` | **2:21** (141 s) | all surviving pointers materialized |
| reflog expire + gc --prune=now | **1:19** (79 s) | — |
| terminal commit `263dcad7` (worktree, pushed back into mirror) | 14 s | `.gitattributes` deleted; 2 CI LFS items removed; 2 test guards updated (3 files, −50 lines) |
| **final** | **~37 min total** | **objects 1.1 GiB du / size-pack 1.00 GiB** (beat the ~2 GiB projection) |

**Postconditions (ALL GREEN; three anomalies diagnosed to check-design, not defects):**
- P1 `git lfs ls-files --all` → **0**. P2 `filter=lfs` → 0, `.gitattributes` absent at HEAD.
- P3 purged-path spot checks 0 commits across every component class (pack/quarantine/hygiene/orphan/windows-invalid/history-only/Forest-Scene/pine_forest/impostors/polyhaven). Two directory checks showed `[1]`: **explained** — non-ratified plain siblings survive there by design (`assets/materials/baked/*.ktx2.meta.json` metadata; Mini Car Kit `License.txt` + `.mtl`); zero actual `.ktx2/.mat/.prefab` remain anywhere in HEAD.
- P4 keep-paths present as plain blobs (derived_1k tree_leaves 2,909,032 B; grass.png; packlists; `docs/history/original-concept-2025.md`; recon doc). One spot-list error on my side: `assets/hdri/sky_equirect.png` is an hdri-catalog **pack member** (in `hdri-catalog.txt`) — correctly purged, restored by fetch-assets.
- P5 HEAD file count: pre-terminal-commit **26,678 = prediction EXACT**; final 26,677 (the terminal commit's own `.gitattributes` deletion — the predicted count was defined pre-terminal-commit).
- P6 tag `assets-v1` rewritten to `ca05f42e` (the rewritten #197-equivalent) — ready for posture-3 push. P7 sizes above.

Residue note for close-out: 26 orphan `baked/*.ktx2.meta.json` metadata files survive (their ktx2 payloads purged) — not ratified for purge; candidates for a post-campaign hygiene sweep.

## Phase 4 — Pre-push verification

**ALL GREEN** — fresh `GIT_LFS_SKIP_SMUDGE=1` clone of the rewritten mirror (HEAD `263dcad7`, 26,677 files, 0 pointers, `.gitattributes` absent):

- **Tier 1:** `cargo check -p unified_showcase` OK; `cargo check -p aw_editor` OK (430 s); sample-render loader tests on exported plain blobs: `running 2 tests … test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 4035 filtered out` (both `loads_grassland_pack_when_present` and `loads_biomes_pack_forest_slot_from_derived_1k`).
- **Tier 2 (live from `assets-v1`):** starter rc=0 in 226 s (`0 up-to-date, 3 installed, 0 failed (of 3)`); `--all` rc=0 in 4,162 s (~69 min; `3 up-to-date, 16 installed, 0 failed (of 19)`, every sha256 pin verified); `verify-assets` **PASS — 90,172 references checked, all pack-managed resolve, 0 unmanaged warnings**; `ci-guard` **PASS**; `unified_showcase` launches with **0 fallback warnings** to "Floating island scene complete".
- **Validator collapse (AD.4 open item CLOSES):** `aw_asset_cli validate assets/materials` → **Total 113 / Passed 103 / Failed 10** — down from the AD.4-era **82 failures**: all 72 fake-ktx2 failures purged; the remaining 10 are the pre-existing `arrays.toml` "Missing 'name' field" schema nits (handed to the hygiene stream).

## Phase 5 — The gate

**GO received 2026-07-17** (with the condition: re-verify remote main == `a17895904` immediately before executing; abort if moved). Execution:

1. First attempt: freeze re-check PASSED (`a17895904` confirmed at push-time) → push started → killed at the 10-min tool cap mid-upload. Remote verified UNCHANGED (git per-ref atomicity held).
2. Second attempt (detached, freeze-guard re-run and PASSED again): **REJECTED by GitHub's pre-receive hook — GH001 large files.** The remote remains untouched (`main` = `a17895904`, tag = `5b2c6c8bd`).

**Finding EX-5 (the rejection, a recon gap):** GitHub hard-caps individual blobs at 100 MB — a constraint that never bound while these files were LFS pointers and which no local postcondition modeled. Full enumeration of the rewritten history (`cat-file --batch-all-objects`, ≥50 MB): exactly **4 blobs**, all at HEAD, all ratified SAMPLE-bucket members under `assets/imported/verdant_trail/meshes/`:

| size | file | class |
|---|---|---|
| 247.4 MB | `rock_07.001.glb` | HARD REJECT |
| 109.9 MB | `dead_tree_trunk_02.glb` | HARD REJECT |
| 109.6 MB | `stone_01_LOD0.001.glb` | HARD REJECT |
| 96.4 MB | `rock_08.001.glb` | warning-class (3.6 MB under the cap) |

These 3 files can never exist as plain blobs in a no-LFS GitHub repo — a permanent platform constraint, not a one-off. STOP-and-report per anti-drift; disposition options presented to the director (§ below). Also fixed: the push script's `PUSH_MAIN_RC` logging read `$?` after a command substitution (always 0) — corrected for the re-run.

**EX-5 CORRECTION (appended 2026-07-17, director-ratified; the original text above stands as the record of the wrong turn):**
1. **The wrong diagnosis:** the original EX-5 (and the Option-1 gate framing built on it) claimed the 4 offending blobs were "all at HEAD, all ratified SAMPLE-bucket members." That was false — the blob→path mapping (`rev-list --all --objects`) was never joined against *which commit* carried which version.
2. **The catch mechanism:** executing ratified Option 1 required reading the 4 files from the worktree to build the v2 pack; the build's own size print showed 37 KB–1.9 MB — three orders of magnitude off — halting the wrong fix before the release mutation happened. (The v2 zip was built deterministically, never uploaded, and is discarded; `gh release view` re-verified the asset list unchanged at 19.)
3. **The corrected diagnosis (per-commit proof):** each path has exactly two versions — a giant 2026-04-07 import (`e9d4d88d`/`f33db533`: 248/110/110/97 MB) superseded the next day by small re-exports (`87be59a9`: 71 KB/1.9 MB/1.2 MB/37 KB, which is what HEAD and the sample set actually hold). The offenders were one-day-stale historical versions riding in history.
4. **The ratified corrected fix (supersedes Option 1):** surgical `git filter-repo --strip-blobs-with-ids` on exactly the 4 enumerated OIDs — 7 s; HEAD commit renamed (`263dcad7` → `dee60b294`) but **`HEAD^{tree}` byte-identical (`8c171eda…`)**; tag re-rewritten to `2f26ea2c`; size-pack **1.00 GiB → 776.30 MiB**. Full postcondition battery re-ran green, including the **now-permanent P8: zero blobs ≥ 95 MiB in all history** and P5 exact (26,677 / HEAD~1 26,678). Tier 1 re-ran on a fresh clone; **Tier 2 carries on the tree-hash equality rule** (identical tree → identical verification inputs), per ratification.
5. **The lesson:** object-level enumerations (`rev-list --objects`, `cat-file --batch-all-objects`) name paths without commits — any size/provenance claim about "a file" must be joined to a ref before it grounds a disposition; and platform-side constraints (GH001) need a local postcondition mirror (P8) because no amount of local git validation models the remote's pre-receive hooks.

## Phase 6 — Post-push

**6.0 The push (2026-07-17):** GO #2 received with the dual-ref freeze amendment + postBuffer-retry as sole contingency. Freeze re-verified both refs at push-time (`main`=`a17895904`, tag=`5b2c6c8bd` — held) → `main` forced `a1789590 → dee60b294` (rc=0, no retry needed), tag forced `5b2c6c8b → 2f26ea2c` (rc=0). **Immediate verification:** `ls-remote` shows exactly the pushed tips; release `isDraft:false`, 19 assets; `hdri-catalog.zip` URL → HTTP 200 unauthenticated. **Posture 3 held on the real repo.** (GO #1's push attempt had been killed by a 10-min tool cap mid-upload — remote unchanged, atomicity held — then rejected by GH001 → EX-5 above; this final push followed the ratified strip.)

**6.1 Remote refs:** deleted `campaign/fluids-f3s` + `campaign/roadmap` (the other recon-era heads had already been auto-pruned remotely). Final remote ref set: exactly `main` + `gh-pages` (gh-pages LEFT per Phase 0 disposition; rebuild flagged for post-D8).

**6.2 CI:** the force-push auto-triggered the 8-workflow matrix including **Pack Guard's first live run**. Remaining workflow "lfs" grep hits = pack-guard.yml's own comments documenting its deliberately-LFS-free design (zero live references). First-board conclusions (on `dee60b294`): **8 success** (Pack Guard first live run, Integration Tests, Net Tests, Mutation Testing, Sanitizers, SDK/C-Harness, CodeQL, OpenSSF) · **3 failure** · **4 cancelled** (concurrency `cancel-in-progress` groups re-fired by the tag-push event + timeouts — benign).

The three failures, diagnosed:
1. **Clippy Unwrap-Prevention** — production `expect()`s in `astraweave-terrain` (`spline_types.rs` E3-era + `regional_archetype_mask.rs`/`noise_gen.rs`). **Proven pre-existing**: identical 7-error output on the pre-rewrite tree. → NAMED HANDOFF (code-hygiene beat), ratified not-an-AD.6-defect.
2. **Rust Cache Quick Check** — `cargo fmt --all --check` fails on CI while the pinned 1.89.0 fmt passes locally with zero diffs on **byte-identical blobs** (verified by blob hash): the workflow's `dtolnay/rust-toolchain@stable` ignores `rust-toolchain.toml`'s 1.89.0 pin and 2026-stable rustfmt applies newer rules. → NAMED HANDOFF (workflow-pin beat), ratified not-an-AD.6-defect.
3. **material-validation.yml — an AD.6 defect (finding EX-6, mine):** the terminal commit's `lfs: true` removal left an empty `with:` block; YAML parses it (`with: null`) but the Actions loader rejects the schema → zero-jobs workflow failure. **Fixed** on the rewritten main (`360a4f12`, fast-forward, director-authorized): orphaned line removed, **actionlint-validated pre-commit**. The workflow has no `workflow_dispatch` trigger; the fix push itself re-fires it (load-confirmation by push, reported as substitution). **NEW STANDING POSTCONDITION (director-ratified): any beat that edits a workflow file must schema-validate the result locally (`actionlint`) before committing** — "YAML-parses is not schema-valid; git-level postconditions do not exercise the Actions loader" (lessons index, attributed to EX-6).

**PUSH FREEZE FORMALLY LIFTED (director-ratified, 2026-07-17):** the freeze protected the rewrite window; the window closed when the push verified. Full-board re-read on `360a4f12`: 13 runs concluded — **8 success** (Material Validation ← EX-6 fix confirmed loading+running, Pack Guard, Integration Tests, Net Tests, Sanitizers, SDK/C-Harness, CodeQL, OpenSSF) · **2 failure** = exactly the two ratified named handoffs (Clippy Unwrap-Prevention → code-hygiene beat; Rust Cache Quick Check → workflow-pin beat) · **3 cancelled** = Comprehensive CI + the two benchmark workflows, diagnosed as **cold-cache timeouts** (ubuntu/macos Quick Check ran to the second of their 30-min budget; the freeze expired all runner caches; Windows passed in 4 min) — expected self-healing as caches repopulate, escalates to a workflow-budget beat only if it persists warm. **This is the stated post-AD.6 CI baseline** (director requirement #5): all green EXCEPT the two named handoffs; future regressions measure against this board.

**6.3 GitHub fresh-clone battery (the ratified real criterion): ALL GREEN** — clone FROM GITHUB in 664 s, **`.git` = 1.4 GB** (v. ~4.8 GiB git-objects + 48 GB LFS pre-campaign; no git-lfs involvement — zero pointers, `.gitattributes` absent, 26,677 files at `dee60b294`; the clone predates the `360a4f12` one-line workflow fix, content otherwise identical). **Case-collision paths absent exactly as predicted (Fabrics=0, Tracks=0, Loops=0).** Tier 1: both crates check OK; loader tests `running 2 tests … 2 passed; 0 failed; 4035 filtered out`. Tier 2 live from `assets-v1`: starter rc=0/169 s; `--all` rc=0/3,602 s (19/19, every pin verified); `verify-assets` **90,172 references, 0 unmanaged warnings**; `ci-guard` PASS; `unified_showcase` 0 fallback warnings to "Floating island scene complete". Clone lives at `d:/tmp/ad6_ghclone` for the rung-3 review.

**6.4 Rung-3 render review: PASS (director, 2026-07-18-era session).** All three checks green on the GitHub fresh clone: editor biomes (desert/forest/swamp/tundra as proper materials, no fallbacks, D1/D2 intact through the rewrite), paint mode (exactly 7 enabled entries painting their named materials, 14 greyed with tooltip), showcase to the floating island scene. **The campaign's ratified human criterion is met.**

**6.5 Local cleanup (executed per close-out ratification):** old repo `git lfs uninstall` (hooks + global filter config removed) + both stale `.git/config` LFS endpoint sections deleted (the pre-rename `AstraWeave-AI-Native-Gaming-Engine.git` URL and the `AstraWeave.git` URL). Residual `filter.lfs.*` keys exist only at SYSTEM level (`C:/Program Files/Git/etc/gitconfig`, Git-for-Windows bundled defaults — machine-wide, elevation-gated, inert for this repo; left in place, noted). **RE-CLONE ratified over reset:** fresh canonical clone at `d:\AstraWeave` (HEAD `360a4f125`, 26,677 files); the old directory is archived (renamed), final deletion left explicit; the local branch museum lives on in `C:\AD6_BACKUP` only.

**6.6 D8 (director-authorized post-rung-3):** the billing API needs the `user` scope (interactive grant), so per instruction the director supplied billing-page screenshots. Read: the metered platform exposes **no point-in-time stored-GB figure** — Git LFS storage is a prorated dollar accrual: ≈$1.70 accrued Jul 1–18 (≈$0.10/day) → **consistent with ~43–50 GB stored at published GB-month rates, i.e. with our measured 48.25 GB and NOT with the old ~22 GB note**; both 10 GB included allowances maxed; **LFS bandwidth flat since ~Jul 10 — independent confirmation the embargo held and the rewrite consumed zero LFS bandwidth**. GitHub Support tickets are submitted via support.github.com by the account holder: the finalized as-filed text is in the close-out doc §D8; submission is the director's action. Pre-D8 baseline: **19/19 release asset URLs return HTTP 200** (re-verify after D8 completes).

**6.7:** see the campaign close-out section + `docs/audits/ad6r/` artifacts. Director's rung-3 on the fresh clone closes the campaign.

## Findings

- **EX-1** (Phase 2): the brief's keeplist-battery formula assumed followers=750; the true keeplist∩purge is 612 with the decomposition above, residue 0 — purge list unchanged, formula corrected in evidence.
- **EX-2** (Phase 0): a live GitHub REST-API incident made valid credentials test as invalid (503s + gh "token is invalid" verdicts). Lesson: **an auth precondition is unevaluable during a provider incident — check githubstatus.com before concluding anything about credentials**, and never schedule an irreversible push into a degraded-API window.
- **EX-3** (Phase 0.5): validation-repo visibility matters — a private repo's release URLs 404 unauthenticated, which would have falsely failed the posture-3 check; the faithful test requires a public repo (matching the canonical repo's visibility).
- **EX-4** (Phase 1): Git-Bash robocopy needs native invocation (MSYS path conversion mangles `/flags` → rc=16); and robocopy rc=1 is success semantics.
