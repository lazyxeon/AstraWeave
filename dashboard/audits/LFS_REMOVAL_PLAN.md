# Git LFS Removal Plan — `.gitattributes` + ecosystem cleanup (AD.2 Phase 5; execution belongs to AD.6)

**Date**: 2026-07-06 | **Method**: local-only enumeration (`git lfs ls-files --name-only`, `git check-attr`, `git count-objects -vH`, ripgrep over workflows/docs/scripts/hooks). Zero network LFS operations (bandwidth embargo respected — `git lfs ls-files` reads the local index only). Every number from tool output this session.

**Headline**: the root `.gitattributes` is the **sole** LFS authority in the repo (no nested `.gitattributes`, no `.git/info/attributes`). Deleting its LFS lines de-LFSes all **91,293** routed files (of **119,079** tracked, 76.7%) — including **100% of the 153-row ratified sample set** — with **zero residual routing possible**. Ecosystem cleanup is small: 2 CI-workflow items, 4 local git hooks, a handful of stale doc lines, 2 test files with pointer-guard comments.

## 1. Rule enumeration (root `.gitattributes`, counts from `git lfs ls-files`)

The extension histogram sums to exactly 91,293 — every LFS-routed file maps to exactly one rule by extension; zero multi-rule files, zero routed-by-no-rule files. `core.ignorecase=true`, so `*.fbx` also absorbs the 6 `.FBX` + 2 `.Fbx` variants.

**21 active rules** (line | pattern | files): 1 `*.png` 56,231 · 2 `*.jpg` 206 · 3 `*.glb` 5,589 · 4 `*.blend` 166 · 5 `*.obj` 9,370 · 6 `*.gltf` 4,314 · 7 `*.tga` 81 · 8 `*.psd` 59 · 11 `*.exr` 105 · 12 `*.hdr` 15 · 13 `*.mp3` 20 · 14 `*.wav` 125 · 15 `*.fbx` 13,401 · 16 `*.ogg` 1,363 · 17 `*.ktx2` 4 · 19 `*.pdf` 7 · 20 `*.tif` 53 · 22 `*.asset` 38 · 28 `*.prefab` 49 · 31 `*.mat` 96 · 34 `*.flare` 1.

**15 zero-file rules** (removal is free; Unity/font/lib carry-overs): lines 9 `*.lib`, 10 `*.dll`, 18 `*.usdc`, 21 `*.tiff`, 23 `*.resS`, 24 `*.resource`, 25 `*.cubemap`, 26 `*.unity3d`, 27 `*.anim`, 29 `*.controller`, 30 `*.mask`, 32 `*.mesh`, 33 `*.physicMaterial`, 35 `*.ttf`, 36 `*.otf`. (Note: 114 tracked `.dll` and 53 `.tif` etc. under `assets/Forest Scene` predate their rules or entered before tracking — the counts above are what LFS actually routes today, which is the number that matters for the rewrite.)

**Negation sections** (bottom of the file): L38-39 `**/tests/fixtures/*.gltf` exempts 3 files (`astraweave-asset/tests/fixtures/*`); L41-43 `docs/src/assets/*.{jpg,png}` exempts 2. All 5 confirmed already-plain blobs. These lines become no-ops once the filter lines are gone — **delete them in the same edit**.

**Not LFS-routed (relevant confirmations)**: the editor splash video `assets/8-second_Cinematic_logo_opening.mp4` (`check-attr` → `unspecified`; `.mp4` has no rule) is a plain 19.1 MB blob and is untouched by the whole operation. `.svg` (4,847 tracked) has no rule either.

## 2. Everything else that assumes LFS (AD.6 cleanup items)

| # | Item | Class |
|---|---|---|
| 1 | `.github/workflows/material-validation.yml:30` — `actions/checkout@v5` with `lfs: true` | **MUST-REMOVE** — post-rewrite there are no pointers to smudge; until then, every CI run that hits it spends LFS bandwidth quota (the exact resource under embargo) |
| 2 | `.github/workflows/integration-tests.yml:156-160` — soft-guarded `git lfs install \|\| true` + `git lfs pull \|\| warning` step | **SHOULD-UPDATE** — soft-fail, won't break, but dead post-rewrite; delete the step |
| 3 | `.git/hooks/{pre-push,post-checkout,post-commit,post-merge}` — standard git-lfs stubs (local machine, not versioned) | **MUST-REMOVE** locally (`git lfs uninstall`) — `pre-push` hard-exits 2 if git-lfs is ever uninstalled before the hooks are removed, blocking all pushes |
| 4 | `.git/config` local LFS sections — endpoint entries for both the current repo URL **and** the stale pre-rename `…/AstraWeave.git` URL | SHOULD-UPDATE (local) — removed by `git lfs uninstall` + manual section deletion |
| 5 | `astraweave-asset/tests/{gltf_loading_tests.rs:298,449, texture_loading_tests.rs:421}` — "may be LFS pointer stubs on CI" skip-guards | SHOULD-UPDATE — post-rewrite the skip branch is dead; assertions can tighten |
| 6 | Docs: `docs/journey/daily/GIT_LFS_AND_FLUIDS_FIX_COMPLETE.md` (the migration being reversed), `docs/current/ASSET_CLEANUP_REPORT.md:205` | SHOULD-UPDATE (add a superseded-by-AD.6 note; journey docs are historical record, do not rewrite content) |
| 7 | `docs/archive/POLYHAVEN_*` (2 files), `docs/guides/ASSET_AND_TEXTURE_INDEX.md:5`, `docs/pbr/PBR_G_TASK5_CI_INTEGRATION_GUIDE.md:223` | INERT (archive/prose mentions) |
| 8 | `scripts/` — zero LFS hits; `README.md`/`CONTRIBUTING*` — zero hits; no `.husky/`, `core.hookspath` unset | none — clean |
| 9 | Campaign audit docs (`SAMPLE_SET_PROPOSAL.md` §10, `DISPOSITION_REPORT_imported.md`) reference LFS descriptively | INERT — correct-by-construction, self-referential |

## 3. Execution plan (inside AD.6's single combined rewrite — per ratified v3/D7)

Removing the rules **before** the rewrite would create the mixed state D7 was ratified to avoid (newly-added files as plain blobs while 91k history files remain pointers, with `git lfs push` semantics half-dead). Therefore:

1. **Pre-rewrite (order matters)**: full mirror backup **including LFS payloads** (`git lfs fetch --all` — ⚠ blocked until the bandwidth embargo lifts ~Aug 1, unless the local `.git/lfs` store is verified complete first: `git lfs push --dry-run origin main` showed zero missing objects on 2026-07-05, so a local-store backup may substitute; re-verify at execution time). Freeze pushes.
2. **The one rewrite** (`git lfs migrate export` combined with the filter-repo history purge, per D7): converts every pointer in history to the real blob (for surviving paths) while the purge drops the paths leaving the repo (quarantine clusters, pack-relegated assets). The staged AD.1.A evidence PNGs (currently 3 LFS pointers) convert to plain blobs here too — desirable; no interim `.gitattributes` exemption is added (a `.gitattributes` edit is explicitly out of AD.2 scope, and the interim cost is only ~3.7 MB of LFS storage on next push, not download quota).
3. **In the same rewrite commit**: delete all 36 `filter=lfs` lines AND the 3 now-no-op negation lines from `.gitattributes` (the file's remaining content, if any, is empty — delete the file if nothing else is added); remove the two CI workflow items (§2 #1, #2); update the two test files (§2 #5).
4. **Local machine cleanup** (each contributor / this dev box): `git lfs uninstall` (removes the 4 hooks + filter config), delete the stale `.git/config` LFS endpoint sections.
5. **Post-rewrite checklist** (all must hold before force-push):
   - `git lfs ls-files` → empty; `git grep -n "filter=lfs" -- .gitattributes` (or file absent) → zero.
   - Fresh clone **on a machine without git-lfs installed** builds and renders (the ratified success criterion; also covers the `hello_companion`/`unified_showcase` smoke).
   - `cargo xtask fetch-assets` restores the packs on that clone (mechanism proven this session — see AD.2 Phase 4 evidence).
   - CI green with no workflow referencing lfs.
   - Case-collision check on the fresh clone: the index-vs-worktree case mismatches (`assets/textures/Fabrics/` [24 files], `assets/audio/Tracks/` [31], `assets/audio/Loops/` [32]) must materialize under their tracked casing — path-referencing code/tests hard-coding the lowercase variants would break; verify editor asset scans still resolve. (Index is internally single-cased, so no checkout collision — the hazard is code assuming the other casing.)
   - GitHub-side LFS storage (~22 GB) is **not** reclaimed by the rewrite — that is AD.6's separate D8 step (GitHub Support ticket first, per ratified v3).

## 4. Sample-set coverage confirmation (the Phase-5.4 finding — CLEAN)

The 153 in-repo sample rows use exactly 5 extensions, all covered by active root rules: `.png` 69 (L1) · `.glb` 63 (L3) · `.gltf` 15 (L6) · `.hdr` 4 (L12) · `.jpg` 2 (L2). Spot-`check-attr` on real sample paths (`assets/textures/cobblestone.png`, `assets/materials/cloth.png`, `assets/models/greybox/echo_grove_greybox.gltf`, `assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr`, KayKit `Rogue.glb`) → all `filter: lfs`; splash `.mp4` → `unspecified` (already plain). No sample file falls under the two negation exemptions (checked — zero overlap), and per §1 no auxiliary attribute source exists. **Zero sample-set files would remain LFS-routed after the removal — no findings.**

## 5. Reference facts for AD.6 sizing

`git count-objects -vH`: size-pack 4.73 GiB (pointer blobs + code history; the ~22 GB of payloads live in `.git/lfs/` and GitHub LFS storage, not the object DB); 91,293 pointers / 27,786 plain blobs / 119,079 tracked. Rewrite tooling operates on the pointer blobs; `git lfs migrate export` needs the local `.git/lfs` store complete (verified 2026-07-05 via `git lfs push --dry-run`: zero missing objects — re-verify at execution).

## 6. Hard sequencing precondition — AD.4 precedes AD.6 (ratified 2026-07-06, AD.1.C)

**AD.4 (asset derivative re-cook) is a hard predecessor of AD.6 (this history rewrite).** 39 of the ratified sample rows currently point at files that are quarantine-destined (18 `assets/materials` C7 + `assets/textures/cobblestone.png` + the C6 runtime materials pending re-cook + the 2 debris glbs — see `THIRD_PARTY_LICENSES.md` §5/§6 and `SAMPLE_SET_PROPOSAL.md`). AD.4 is the beat that refills those slots with traced derivatives. If the AD.6 purge runs **before** the re-cook, those sample slots lose their backing files and the ratified **fresh-clone-builds-and-renders** success criterion (§3 step 5) breaks. Therefore the AD.6 rewrite must not execute until AD.4 has landed the replacement derivatives and the sample set has been re-pointed. (This is independent of the bandwidth-embargo gate in §3 step 1 — both must clear before the rewrite.)
