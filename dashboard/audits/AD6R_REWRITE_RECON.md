# AD.6.R — Rewrite Recon: the irreversible operation, rehearsed

**Session:** AD.6.R (2026-07-15). **Constraints honored:** zero LFS network operations (local store reads + plain-HTTP API only), zero pushes, zero remote mutations; real worktree read-only except this doc + the machine-readable artifacts under `docs/audits/ad6r/`. All rewrite work on scratch mirrors under `d:/tmp/ad6r/`. Every figure measured this session or cited to a ratified doc.

---

## 0. #198 identification + disposition options (DIRECTOR RATIFIES)

**What it is.** `a17895904` "Campaign/roadmap (#198)" = the GitHub-web squash-merge (committer `GitHub <noreply@github.com>`, author lazyxeon, 2026-07-03) of the **R-series campaign branch** into `main` — 50 files, +2,234/−150: the R.0.A/R.0.B/R1 roadmap docs, M1.x diagnostics, the trace-honesty pass, and associated code/test changes. It appeared under the push freeze because no local push occurred: the branch (`origin/campaign/roadmap`) was already on GitHub pre-freeze, and PR #198 was merged through the web UI.

**Content status (blob-proven, this session).** All three underlying content commits (`77c79e003`, `68eeb68d1`, `431042782`) are **ancestors of local HEAD**. Diffing #198's tree against local HEAD across all 50 files: 46 identical; the 4 that differ (`Cargo.lock`, `shader_validation.rs`, `aw_editor/Cargo.toml`, `terrain_panel.rs`) each have #198's exact blob **byte-identical in HEAD's ancestry** (at `ae9328a20`/`bde48792a`/`0758658e5`) — local later superseded them. **#198 contains zero unique content.**

**Options:**
- **(a) Merge/cherry-pick before the rewrite.** Preserves #198's merge linkage inside the new history. Cost: a content-empty merge knot; the rewrite renames every hash anyway, so nothing of the original commit object survives either way.
- **(b) Explicitly discard (recorded).** The force-push overwrites `main`'s tip; the commit object and its merge-ref die; **zero content dies** (proven above). The PR #198 page itself remains viewable on GitHub after the force-push (PR pages persist independent of branch history; scrubbing them requires a deliberate GitHub Support request — see §4 citations).

Recon's note: (b) is the factually-clean option; (a) buys only sentimental lineage. **The director disposes.**

---

## 1. Final disposition inventory (every tracked path, exactly one destiny)

**Universes measured this session:** HEAD tracked = **119,210** paths; HEAD LFS-routed = **91,329** (= the AD.4 successor denominator exactly — the partition maps 1:1 onto HEAD); full-history path universe (diff-based, see method note) = **131,071**, of which **11,861** are history-only (deleted before HEAD).

> **Method finding (new, load-bearing):** `git lfs ls-files --all` **deduplicates paths by OID** — byte-identical files at different paths list only once (verified: OID `c54c7788…` backs 4 tile PNGs at HEAD; `--all` lists one). Its path output (87,783) is NOT a path universe. Same for `git rev-list --objects` (dedupes by object, including whole identical subtrees). The only path-complete enumeration is diff-based (`git log --all --name-only --no-renames`). OID-level enumeration (88,130 unique objects) remains correct for object-completeness checks. **Any AD.6 step needing full-history path lists must use the diff-based method.**

### 1.1 KEEP (pointers → plain blobs at export)

- **sample = 150** LFS files (AD.0 ratified sample set as refilled by AD.4: 36 `derived_1k/` + 27 traced-9 + the rest of tables A–F/H/I; the 153-rows→150-files reconciliation is exact: −2 debris retired, −1 `cobblestone.png` re-pointed).
- **retained = 54** (AD3R Phase-2 set: every LFS HEAD file outside `assets/`, minus `assets_src/`+`archive/` — astraweave-audio fixtures 16, examples 14, docs 11, gh-pages 4, render 3, tools 3, root screenshots 3). One AD3R prose off-by-one found and resolved: `assets/Astraweave_logo.jpg` is a SAMPLE row (Table C), not retained — with it in sample, both buckets foot exactly.
- All code/docs/config and every tracked non-LFS asset not named below (≈27.4K files), including the committed packlists + keeplist + generated ignore surfaces.

### 1.2 PURGE (path leaves history entirely) — assembled list: **91,651 paths**

| component | count | source |
|---|--:|---|
| pack-relegated packlist members | 90,013 | `assets/packlists/*.txt` (19 lists, committed) |
| ratified quarantine | 1,112 | derived file-level this session (`docs/audits/ad6r/bucket_quarantine.txt`), 25 clusters, every cluster doc-cited (`bucket_derivation_notes.md`); includes the 36 C6/C7 old occupants + `assets/textures/cobblestone.png` + 2 debris glbs (verified present) |
| hygiene | 583 | `archive/**` 448 + `assets/cache/impostors/**` 134 + `assets/textures/grass_hd/grass` (1-byte stray) |
| AD.5.A orphan cluster | 13 | 6 `examples/unified_showcase/src/main_*.rs` orphan sources (re-verified consumer-less at HEAD this session: zero mod-decls, `autobins=false`, single `[[bin]]`) + `assets/materials/polyhaven/**` (7 files — the mission named the toml; the dir's arrays.toml/READMEs are its support files, same consumer-less status: recon addition, flag at gate) |
| Windows-invalid historical path | 1 | `AI Engine: AstraWeave.md` — see §3 finding F1 |
| *(overlap dedup)* | −71 | quarantine ∩ hygiene (67 impostor LFS files + 4) |

Safety checks: purge ∩ sample = **0**; purge ∩ retained = **0**. Bucket lists are pairwise-disjoint, union == the 1,316 non-pack LFS remainder exactly, and quarantine ∩ pack = 0 (independent check).

**Cross-foot:** 119,210 tracked = 91,650 purged-at-HEAD (the invalid path is history-only) + 27,560 kept. LFS partition: 91,329 = pack 90,013 + quarantine 1,112 + sample 150 + retained 54 (exact, zero ungrounded assignments). Derivation-notes findings (count reconciliations, not ambiguities): Forest Scene 409 vs DISPOSITION §6's "150" (that figure was the scene-art subset; whole tree untraceable → quarantine); AlkaKrab 63 = 60 audio + 3 duplicate license PDFs; fine_leaf 6 loose textures grounded by the family rule.

### 1.3 DECIDE (proposals only — director ratifies)

**(i) The 202 LFS-attributed plain-blob files** (AD.5 finding 5.3). Computed exactly this session: 207 extension-matched plain blobs = **202 genuinely attributed** + 5 negation-exempted (3 `astraweave-asset/tests/fixtures/*.gltf`, 2 `docs/src/assets/*`) — reconciles AD.5's count. Per-cluster proposal:

| cluster | n | proposal |
|---|--:|---|
| `.ktx2` malformed ("fake AW_TEX2"): `assets/materials/baked` 36 + `assets/materials` 36 + `baked_materials` 13 | 85 | **PURGE** — the AD.4 open-item 72 malformed + 13 siblings; dead bake output nothing reads |
| Unity-era `.mat`/`.prefab` inside quarantined/archive kits (Mini Car Kit 28, Nature Kit Classic 20, Space Kit Legacy 12, Symphonie 6, misc) | ~69 | **PURGE** — follow their parent clusters |
| Fonts `.ttf`/`.otf` (`assets/Other/Fonts` et al.) | 50 | **KEEP-as-plain** (AD.1: fonts clean); attributes deletion resolves them |
| UI/icons (`Input Prompts` 28, UI packs 6) | 34 | **KEEP-as-plain** |
| `assets/models/3DTreeStump001_SQ-1K-JPG.usdc` | 1 | **KEEP-as-plain** — the perpetually-modified status heals when the `filter=lfs` attribute dies: **HEALED** — 0 modified in the fresh clone of the rewritten mirror, both before the terminal commit (the export's `* !text !filter !merge !diff` override already disarms the filter) and after `.gitattributes` deletion (rehearsal check, §3) |
| loose png/jpg/gltf | 3 | KEEP-as-plain |

**(ii) Historical residue (new DECIDE cluster, recon-found).** 11,861 history-only paths; dominated by committed-then-deleted junk: `target/debug` 2,923, rustdoc `api/**` ~2,600, `mutants.out.old` 1,094, `docs/root-archive` 270, `assets/imported` 253, `assets/audio__files` 231 (deleted AlkaKrab-era audio — LFS payloads!), `cov_html`, `temp_outputs`, more (`docs/audits/ad6r/history_only_clusters.txt`). Un-purged residue stays in the rewritten history; LFS-pointer residue gets its payload **re-embedded as plain blobs** by the export. Proposal: add the residue clusters above to the purge (they are unreachable-at-HEAD baggage); the rehearsal (§3) measured the ratified-scope-only rewrite, so the size figure below is the CONSERVATIVE (residue-kept) number — a second variant can be measured on ratification.

Machine-readable artifacts (committed): `docs/audits/ad6r/{bucket_sample,bucket_retained,bucket_quarantine,hygiene_purge,orphan_purge,windows_invalid_paths,plain_but_attributed,history_only_clusters}.txt` + `bucket_derivation_notes.md`. Assembly = those + the 19 committed packlists, `sort -u`.

---

## 2. Backup verification (AD.2.A method, re-run at execution-era HEAD)

**Enumeration (this session):** referenced full-history LFS objects = **88,130 unique OIDs** (`git lfs ls-files --all`, OID column — the OID side of the AD.2.A method is unaffected by the path-dedup finding). Local store = **88,588 objects / 57,678,445,259 B (53.72 GiB)**. Set comparison: **missing-from-store = 0** — **Verdict A re-confirmed**. Extras = 458 orphan objects (9.43 GB, local-only fetch artifacts).

**Figure reconciliation:** referenced-objects byte-sum = **48,245,794,652 B = 48.25 GB** ≈ AD.2.A's 48.13 GB (delta = the AD.4-era derived_1k/traced-9 additions). The old "~22 GB" note remains contradicted; the D8 ticket (§4.3) instructs reading the live billing figure at ticket time.

**Backup set for AD.6 (definition):**
1. **Pristine local mirror + full LFS store copy** — `git clone --mirror` (4.8 GiB objects) + store copy (54 GiB; hardlink-copy costs ~35 s / zero disk on same volume, but the real backup must be a **physical copy on a different volume/medium**). Taken AFTER the freeze-verify, BEFORE any rewrite.
2. **The AD.3 staging zips** (still held per the AD.3 gate): `d:/tmp/ad3_staging/zips` + `d:/tmp/ad4_staging` — all 19 sha256-match the manifest pins (verified in AD.5).
3. **The live `assets-v1` release itself** (19 assets, GitHub-side).

**Destruction criterion:** backups die only after BOTH post-push verification tiers pass on a fresh clone from the real rewritten remote (§4.4), plus the D8 storage-reclamation outcome is confirmed non-destructive to the release assets.

---

## 3. The rewrite, rehearsed (scratch mirror; measured)

**Modeled push set.** The scratch mirror (`d:/tmp/ad6r/mirror.git`, mirror-clone of local `.git`) was pruned to exactly what AD.6 pushes: `refs/heads/main` = local `campaign/roadmap` tip (`1f09f1556`), `refs/tags/assets-v1` (= `5b2c6c8bd`, an ancestor — filter-repo rewrites branch and tag consistently). All other local branches (9) and remote-tracking refs were dropped from the rehearsal set; their fate is a gate item (default: not pushed, preserved in the backup mirror only).

> **Tool-semantics facts that shaped the sequence** (from `git lfs migrate` docs, verified locally): `--everything` examines all refs but **rewrites only local refs** — remote-tracking refs keep stale pre-rewrite state, so the working mirror must contain ONLY the push-set refs or postconditions read stale pointers. And `git lfs migrate export` requires the payload objects present under `<git-dir>/lfs/objects` (hardlinked from the verified-complete store; 88,588 links in ~35 s).

**The sequence (D7 combined operation, resolved):**
1. `git filter-repo --force --invert-paths --paths-from-file purge_list.txt` (91,651 literal paths) — purge FIRST so the export materializes payloads only for surviving paths.
2. `git lfs migrate export --everything --include='*' --yes` — pointer→blob for everything surviving; observed `.gitattributes` end-state: the original 36 rule lines REMAIN, with a global `* !text !filter !merge !diff` override APPENDED as the last line (last-match-wins → functionally LFS-free); the terminal commit still deletes the file per plan §3.3.
3. `git reflog expire --expire=now --all && git gc --prune=now`.
4. Terminal commit (worktree clone): delete remaining `.gitattributes` LFS/negation lines (or the file), remove the 2 CI-workflow LFS items, tighten the 2 pointer-guard test files (plan §2 #1/#2/#5).

**Findings:**
- **F1 (blocking, fixed in-rehearsal):** the first run **crashed `git fast-import`**: `fatal: invalid path 'AI Engine: AstraWeave.md'` — the repo's original concept doc (added `0fb845c28`, deleted `84f68d0a5`, both web-UI commits, Sept 2025) has a **colon in its path**, invalid on Windows; modern git refuses to replay it. It is the only NTFS-invalid path in all 131,071 (full scan committed as `windows_invalid_paths.txt`). Resolution used: add it to the purge (it can never be checked out on Windows). Alternative for the gate: `core.protectNTFS=false` during the rewrite keeps it in history (and leaves the same landmine for every future history operation). **Recommend purge.**
- **F2:** filter-repo's failure mode on Windows when fast-import dies is an opaque `OSError: [Errno 22]` on a pipe flush — the real error is in `mirror.git/fast_import_crash_*`. Recorded for the AD.6 prompt's troubleshooting notes.

**Measured results (rehearsal, ratified-scope purge):**

| step | wall clock | result |
|---|---|---|
| filter-repo purge (91,651 paths, bare mirror) | 1,792 s (~30 min) | objects 4.8 GiB → **935 MiB** |
| lfs migrate export (--everything) | 892 s (~15 min) | pointers→blobs for all survivors; worked on the bare mirror directly |
| reflog expire + gc --prune=now | 1,002 s (~17 min) | **final objects: 13 GiB packed** (pre-state 4.8 GiB; see the composition analysis below) |

**Postconditions (mirror):**
- `git lfs ls-files --all` → **0** ✓ (no pointer anywhere in rewritten history).
- Purged-path spot checks → 0 commits each ✓ (pine_forest, archive/, impostors, orphan sources, polyhaven toml). One initial non-zero: `assets/models/Amber-Npc` still had 1 commit — the non-LFS `Amber.json` cohabitant, which exposed the cohabitant finding below.
- Keep-path spot checks: derived_1k/tree_leaves.png 2,909,032 B plain ✓, grass.png ✓, packlists ✓, docs ✓. `assets/hdri/sky_equirect.png` → correctly ABSENT (it is an hdri-catalog pack member — pack-relegated; the AD.5.A showcase fix's graceful sky degradation covers exactly this).
- Rewritten HEAD file count = **27,560 = 119,210 − 91,650 exactly** (the purge executed to spec).
- **Case-collision hazard EVAPORATES**: zero `Fabrics/`/`Tracks/`/`Loops/` paths at rewritten HEAD (all were pack/quarantine content) — ratified checklist §3.5's item auto-resolves.

**F3 — quarantine cohabitants (new inventory finding):** the ratified partition is LFS-file-level; wholly-quarantined cluster dirs carry **750 non-LFS sibling files** that survive the rewrite (Forest Scene 701 Unity project files, Symphonie 38, Miniguides 9, Amber.json, Goodies 1 — list: `docs/audits/ad6r/quarantine_cohabitants.txt`). Dead weight pointing at purged content. **Proposal: extend the purge dir-level for wholly-quarantined roots** (gate item).

**F4 — the 13 GiB explained (composition measured on the rewritten mirror):** total unique blob bytes 16.66 GiB, of which **14.02 GiB sits at HISTORY-ONLY paths** (path-purgeable): `assets/imported` 6.97 (deleted 8K-era verdant/Namaqualand originals), `assets/audio__files` 3.10 (deleted AlkaKrab-era audio), historical `assets/textures` 2.39, `target/debug` 1.12 (committed build artifacts), tails (`PBR_4K/`, `Infinite Ocean`, `mutants.out.old`, `test_output`). Only **2.64 GiB** belongs to kept-at-HEAD paths incl. their full version history. **Adding the history-only-path purge component (11,861 paths, `docs/audits/ad6r/history_only_paths.txt`) takes the rewritten repo from 13 GiB to ≈2 GiB** — this converts DECIDE (ii) into a strongly-recommended, numbers-backed gate item.

**Verification tiers on the rewritten tree:**

*Terminal commit rehearsed* (worktree clone, 19 s): `.gitattributes` deleted; `material-validation.yml` `lfs: true` removed; `integration-tests.yml` LFS step removed; `git grep filter=lfs` → 0; working tree clean before AND after (the perpetually-modified `.usdc` is healed).

*Tier 1 (PASS):* `cargo check -p unified_showcase` OK; `cargo check -p aw_editor` OK (441 s cold); sample-render-path load test on the EXPORTED plain blobs: `running 2 tests … loads_biomes_pack_forest_slot_from_derived_1k ... ok … 2 passed; 0 failed` — the derived_1k KEEP files decode correctly post-export.

*Tier 2 (starter + `--all` LIVE from `assets-v1` + rails + showcase launch):* **ALL GREEN** (live from the release, on the rewritten tree):
- starter (default invocation): rc=0, **178 s** — 3 packs, 0 failed (kaykit-complete 17,443 files restored to original paths).
- `--all`: rc=0, **3,206 s (~53 min)** — 19/19 (3 up-to-date + 16 installed), 0 failed, every sha256 pin verified.
- Rails: `verify-assets` **PASS — 90,172 references checked, all pack-managed resolve, ZERO unmanaged warnings** (the AD.5-era 10-warning class died with the purged consumer-less polyhaven toml); `ci-guard` **PASS**.
- `unified_showcase` (AD.5.A-upgraded criterion): launches, **0 fallback warnings**, reaches "Floating island scene complete" steady state (40 s run).

---

## 4. Push + reclamation plan (D8) — DRAFT for ratification

### 4.1 Force-push plan

- **Push set:** `refs/heads/main` (the rewritten history) — a single force-push. Other refs: NOT pushed. Remote stale branches (`origin/campaign/*`, dependabot, `gh-pages`) — gate item: delete remotely or leave (they keep old history reachable server-side and in `--mirror` clones, though NOT in default clones, which fetch `refs/heads/*` + tags only... note `origin/campaign/roadmap` IS a head: **remote branch deletion list is a required gate decision**; default-clone reachability includes every remote branch head).
- **Branch protection (measured, live API):** `main` carries ruleset "Admin Authorization" (id 7941616) including `non_fast_forward` + `pull_request` rules — but repository roles write/maintain/admin are all `bypass_mode: always`, so **the owner's force-push proceeds with no settings changes**.
- **The `assets-v1` tag (the load-bearing decision).** Live state (API, this session): release published, `immutable: false`, 19 assets. Three postures:
  1. **Leave the remote tag untouched** — docs-safe for the release, but the tag keeps the ENTIRE pre-rewrite history reachable and **every fresh clone downloads it** (clones fetch all tags): defeats the operation. Rejected.
  2. **Delete the tag** — docs/staff-confirmed: the release **demotes to draft**; draft asset binaries are NOT served at the public `/releases/download/<tag>/` URLs (observed 404s even with PATs) → **breaks every sha256-pinned manifest URL**. Rejected.
  3. **Force-push the rewritten tag under the same name** (re-anchor; filter-repo already rewrites it to the rewritten `#197`-equivalent) — anecdotally the release follows the tag name with URLs unchanged, **but this is NOT docs-confirmed** (GitHub docs actively recommend not attaching releases to movable tags; confidence Medium-Low). **REQUIRED AD.6 PRE-STEP: empirically validate on a disposable repo** (publish a tiny release, `git tag -f` + force-push the tag, `curl -I` the download URL → expect 200/302 and release still published). Only if validation passes does posture 3 get ratified; the fallback is a re-upload of the 19 assets to a fresh release on the rewritten history (uploads are permitted under the embargo; ~15.4 GiB).
- **`pack-guard.yml` activation + freeze-lift:** force-push → verify CI green → lift the push freeze → the guard rail is live on the rewritten default branch.
- **Point of no return (named): the `main` force-push itself.** Every step before it is locally reversible (scratch mirrors, backups intact, remote untouched). After it, recovery = force-pushing the backup mirror's old `main` back (possible until backups are destroyed and as long as the LFS objects still exist server-side — which they do until D8 executes; therefore **D8 runs ONLY after both tiers pass**, making D8 the second, storage-level point of no return).

### 4.2 Stop conditions

Abort (before the force-push) if: postconditions fail on the final pre-push mirror; the tag-retarget validation fails (→ re-plan with the fresh-release fallback); the fresh-clone tiers fail on the local rewritten mirror; the backup set is incomplete (OID enumeration ≠ 0 missing).

### 4.3 D8 ticket draft (GitHub Support)

> Subject: LFS storage reclamation after history rewrite — repo lazyxeon/AstraWeave-AI-Native-Gaming-Engine
>
> We have rewritten this repository's history to remove Git LFS entirely (git filter-repo + git lfs migrate export; `.gitattributes` LFS rules deleted). `git lfs ls-files --all` on the current history is empty, and no ref reaches any LFS pointer. Per "Removing files from Git Large File Storage", removed LFS objects continue to count against storage; since we cannot delete-and-recreate the repository (releases, issues, PR history must survive), we request purging the now-unreferenced LFS objects for this repository.
> Our accounting expects roughly **48.25 GB** of referenced LFS storage before the rewrite (measured locally from the object store; our earlier internal note said ~22 GB — the current billing figure as shown in our account at ticket time is: **[READ AND INSERT THE LIVE BILLING FIGURE HERE; if it materially differs from 48.25 GB, state both numbers and this sentence]**).
> Please confirm before purging that this does NOT affect: (1) the published release `assets-v1` and its 19 uploaded assets (release assets are not LFS objects — they must remain downloadable), (2) repository issues/PRs.

### 4.4 Post-push checklist

1. CI green on the rewritten remote (no workflow references LFS).
2. Fresh clone **from GitHub** on a machine without git-lfs: builds; `cargo xtask fetch-assets` (starter) then `--all`; rails green (`verify-assets`, `ci-guard`); `unified_showcase` launches to "Floating island scene complete" (the AD.5.A-upgraded Tier-2 criterion); editor sample render check (director, rung 3).
3. Case-collision check on the fresh clone (`Fabrics/`, `Tracks/`, `Loops/` materialize under tracked casing).
4. D8 ticket filed; on completion, re-verify the release's 19 download URLs return 200.
5. Backup destruction gate: only after 1–4 all green.

---

## 5. Gate items (director ratifies before the AD.6 prompt is drafted)

1. #198 disposition — option (a) or **(b, recommended)** (§0).
2. The purge list as assembled (91,651) incl. the recon additions: full `assets/materials/polyhaven/` dir (§1.2) and the Windows-invalid path (§3 F1).
3. DECIDE (i): the 202 plain-blob dispositions (§1.3 table).
4. DECIDE (ii): historical-residue purge clusters (§1.3) — recommend adding; a second rehearsal variant can measure the size delta on request.
5. Local branch + remote stale-branch fate (delete remote `origin/campaign/*` etc., or leave — reachability consequences in §4.1).
6. The `assets-v1` tag posture (§4.1 posture 3 + its disposable-repo validation pre-step; fallback = fresh release re-upload).
7. Other-orphan-source candidates NOT in the ratified list (e.g. `material_integration.rs`, `material.rs` in unified_showcase — dead per AD.4 liveness facts but not in the AD.5.A finding): include in purge or leave.
8. The AD.6 execution prompt gets drafted only against the ratified version of this plan.

---

## 6. Figures ledger (all measured this session unless cited)

| figure | value | source |
|---|---|---|
| HEAD tracked paths | 119,210 | `git ls-files` |
| HEAD LFS-routed | 91,329 | `git lfs ls-files` (= AD.4 successor denominator) |
| Full-history paths (diff-based) | 131,071 | `git log --all --name-only --no-renames` |
| History-only paths | 11,861 | set difference |
| Full-history unique LFS objects | 88,130 | `git lfs ls-files --all` OIDs |
| Local store | 88,588 objects / 53.72 GiB | walk + byte-sum |
| Missing from store | **0** | OID set-compare |
| Store orphans | 458 / 9.43 GB | set-compare + stat |
| Referenced-objects byte-sum | 48.25 GB | 57.68 − 9.43 (≈ AD.2.A 48.13) |
| Purge list | 91,651 paths | assembled (§1.2) |
| Pre-rewrite git objects | 4.8 GiB | `du` mirror objects/ |
| Rehearsal timings/sizes | §3 table | rehearsal.log |
| Release assets-v1 | published, immutable:false, 19 assets | live API |
| Ruleset 7941616 | non_fast_forward + pull_request, bypass always (write/maintain/admin) | live API |
