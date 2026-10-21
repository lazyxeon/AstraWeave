# AD.5 — Path reconnection: transactional installer, profiles, rails (outcome)

**Date:** 2026-07-08  **Branch:** `campaign/roadmap`  **Ratified design:** director, 2026-07-08.
**Commits (local-only, push freeze):** `ecbbe44d0` (installer + tests), `f6c0caab7` (manifest v2 + packlists + ignore surfaces + CI guard), `8b0213ef2` (lockfile), `d1900a172` (H1 cross-pack prune guard + M2 coverage), `cf4576829` (verify-assets gate scoping).
**Verification doubles as the AD.6 Tier-2 dress rehearsal** — the full protocol ran live against the `assets-v1` release in a simulated post-rewrite tree.

---

## 1. What shipped

`cargo xtask fetch-assets` now transactionally restores pack contents to their **original repo paths** (superseding the AD.2/AD.3 `assets/packs/<unpack_to>` interim model), plus `verify-assets`, `gen-ignore`, `gen-keeplist`, `ci-guard` subcommands, `[profiles]` (default = `starter`, `--all`, `--pack` unchanged), `--force|--repair`, `--adopt`.

**Ratified semantics → implementation → test (deliverable 1):**

| semantic | implementation | cited passing test |
|---|---|---|
| valid stamp → no-op | stamp check precedes all work (`install_one`) | `stamped_noop_leaves_everything_alone` |
| incomplete txn → repair/roll forward | journal + `recover_from_journal` (both branches) | `journal_roll_forward_completes_interrupted_install`, `journal_refetch_when_staging_lost` |
| version change → replace + prune | sha256 = version anchor; ledger-diff prune | `version_change_replaces_and_prunes` |
| `--repair`/`--force` → pristine | force reinstall path | `force_restores_pristine_without_touching_unknown_files` (+ live rehearsal §3) |
| unknown file at destination → FAIL unless overridden | member-exact ownership check, `--adopt` | `unknown_preexisting_file_refused_then_adopted` |
| no surprising destructive work | no-op touches nothing; prune is ledger-scoped + cross-pack-guarded | same tests + `prune_never_deletes_a_file_migrated_to_another_pack` |
| default = starter; `--all`; `--pack` | `[profiles]` + Selection | `profiles_starter_default_and_all`, `pack_filter_selects_and_rejects_unknown` |
| generated ignore rules; root entry only if outside assets/ | `gen-ignore` two surfaces | `gen_ignore_generates_and_ci_guard_enforces`, `outside_assets_root_gets_root_gitignore_block` |

Plus: 12-step lifecycle with the **complete member battery pre-download** (path safety, root coverage, Windows reserved-names/case-collision/MAX_PATH, git-index collision, ownership), single-instance lock, same-volume staging, three-way staged agreement (staged == zip `_PACK_MANIFEST` == committed packlist), retry-on-os-error-5 rename backoff (closes the AD.3 Defender-race item — `retry_on_access_denied_engages_and_gives_up_correctly`), destination-scoped git rail, existence verification. **24/24 tests** (all 8 AD.2 fixtures ported), clippy `-D warnings` clean.

## 2. The ownership model — validated against reality and corrected

The rehearsal's manifest-driven deletion (mission: "this also validates the manifest's ownership model against reality") was preceded by an empirical check that **falsified directory-exclusive ownership**: every candidate root has tracked non-pack cohabitants (audio `License.txt` AD.1 evidence, `hdri_catalog.toml`, sample HDRIs, imported-scene JSON manifests, and ~23K non-LFS `.mtl/.svg/.bin/.dae/.stl` siblings that were never in the LFS partition). The corrected model:

- **Ownership is FILE-exact**, anchored by committed packlists (`assets/packlists/<pack>.txt` == the zip's `_PACK_MANIFEST.txt`). Their 19 lists total **90,013 members — exactly the ratified pack-bucket denominator** (AD.4 §13.5 cross-foot). Disjointness verified (0 collisions); root coverage verified (0 outside).
- **Manifest `roots` are coarse** ignore/rail coverage (containers legally shared).
- **Cohabitant keeplist** `assets/packlists/_tracked_keep.txt` (23,205 entries; 0 overlap with members) with **subset semantics** — AD.6's quarantine departures need no regeneration; only NEW tracked files under roots turn the guard red.
- **All-roots-under-assets verification (deliverable 4): 18 of 19 packs under `assets/`; `materials-src` at `assets_src/` falsified the "expected none"** — handled by the ratified root-entry mechanism: the generated AW-PACK-IGNORE marked block in the root `.gitignore` (tested end-to-end incl. hand-content survival + tamper drift).

## 3. Dress rehearsal (deliverable 2) — live against `assets-v1`

Simulated post-AD.6 tree: `GIT_LFS_SKIP_SMUDGE=1` local clone → **deletion driven by the committed packlists** (90,013 members removed from index + worktree; 23,205 cohabitants verifiably still tracked) → sim commit. `ci-guard` PASS pre-install (0 tracked members, 0 strays, surfaces match). Editor proxy: `cargo check -p aw_editor` in the sim tree **exit 0** (6m58s; only the known pre-existing unused-import warning).

| run | result | wall clock | bytes |
|---|---|---|---|
| `fetch-assets` (default → starter) | 3 packs, all sha256 verified, **18,753 files restored**, rail clean | **3m09s** (incl. cold build) | 568,074,962 B (541.8 MiB) |
| starter rerun | announced no-op, `3 up-to-date` | **4.3s** | 0 |
| `fetch-assets --all` | 19/19 packs, **every sha256 pin verified live**, 90,013 files restored, rail clean | **≈66m49s** (19:42:22→20:49:11; incl. the §4 incident tail) | 16,530,736,702 B (15.4 GiB) |
| `--repair` (starter) | ratified pristine-restore exercised on real damage (§4) | **3m21s** | 541.8 MiB |
| `--all` rerun | **`19 up-to-date, 0 installed, 0 failed`** | **1.8s** | 0 |
| verify-assets (full tree) | **90,182 references checked — all pack-managed resolve** (13 unmanaged warnings, §5.1) | — | — |

**Onboarding numbers:** starter = **541.8 MiB / ~3 minutes**; `--all` = **15.4 GiB / ~1 hour** (3.5% ratio).

**Induced-crash rehearsal (deliverable 2):** `kill -9` of a `--force` kaykit-complete install, landed in journal phase `renaming` (staging present, 17,443 files mid-storm). Recovery run: `! incomplete transaction found (phase 'renaming', 17443 files) — recovering` → `rolled forward: 17443 files placed, transaction completed` → journal+staging clean, stamp present, **17,443/17,443 ledger files on disk**, follow-up run a no-op. One documented crash-cleanup step: a hard kill leaves the stale single-instance lock (RAII can't run on `kill -9`); the tool's error message instructs deletion.

### 3a. Incident during rehearsal (operational, not an installer defect)

After the starter install, the sim tree was **rebased** to pick up the audit fixes — the rebase checked out a commit where members are still *tracked*, clobbering the 3 installed starter packs with pointer stubs which the replayed deletion commit then deleted. Stamps survived (state dir is ignored), so the subsequent `--all` correctly no-op'd the starter packs per ratified stamp semantics — and **`verify-assets` caught the stamp-vs-reality divergence exactly as designed** (18,753 missing = precisely the 3 starter ledgers), failing loudly. `--repair` restored pristine in 3m21s. Two lessons minted for AD.6 (§6).

## 4. Adversarial verification (deliverable 6)

Independent audit (opus, read-only) on `ecbbe44d0`/`f6c0caab7`: full crash-window walk (every step-3→10 window + recovery re-entrancy + journal-vs-stamp precedence — all sound), path-safety bypass hunt (none; `is_safe_relative`/`under_root`/`remove_empty_parents` verified incl. Windows semantics), pin-drift check (all 19 sha256/size **byte-identical** to pre-AD.5), packlist integrity recomputed independently. **Verdict: GO (conditional)** — conditions all met before the 16 GB run:

- **H1 (HIGH, latent) — FIXED `d1900a172`:** prune could delete a file whose ownership migrated to another pack (unfavorable `--all` order). Both prune sites now consult the union of all current packlists. Regression: `prune_never_deletes_a_file_migrated_to_another_pack`.
- **M2 — FIXED `d1900a172`:** the AW-PACK-IGNORE surface had no coverage → full round-trip test added.
- **M1** — met by construction (rehearsal tree pack-content-untracked). **M3** — pre-verified offline: all 19 local zips == pins == committed packlists (zero wasted-download risk).
- Accepted as documented: **L1** disk check over-conservative (×2.5 of selection; sequential installs peak lower); **L2** ownership gate is preflight-only (lock + ignored destinations mitigate; window = one download); **L3** roll-forward integrity is existence-only (content was verified pre-journal); **L4** `--repair` CLI alias untested at arg-parse layer (semantics tested; exercised live §3); **I1** ci-guard is root-scoped (a blob under `assets/` outside every root is out of this rail's scope); **I2** `pack-guard.yml` red on pre-AD.6 trees by design (activates when pushes resume post-rewrite).

## 5. Findings for the director

1. **`_downloaded` consumer gap (real onboarding hole, found by step 12):** the live editor's 8-slot biomes pack forest slot references `assets/_downloaded/polyhaven/forest_leaves/*` — gitignored fetcher output that **no pack ships and no fresh clone carries** (13 refs across 2 biome tomls). The gate scopes these as loud warnings (`cf4576829`); fixing is consumer-side: re-point the forest slot to pack/sample content, or add an `astraweave-assets fetch` step to onboarding. Director decision.
2. **`unified_showcase` startup panic (pre-existing):** `main.rs:944-967` hardcodes `"assets/textures/pine forest textures/…"` (nonexistent; real dir is `textures/pine_forest/`) with a panicking `load_texture` — no pack selection can fix it; the flagship demo crashes regardless. Repair candidate for a demo-hygiene beat.
3. **202 LFS-attributed plain-blob files (pre-existing):** committed before LFS migration (.mat/.ttf/.otf/.ktx2/.prefab/.usdc); one (`assets/models/3DTreeStump001_SQ-1K-JPG.usdc`) shows perpetually-modified in every fresh checkout. AD.6 rewrite queue item (migrate or exempt).
4. Old `assets/packs/` trees from the AD.3 model on developer machines are orphaned (gitignored); deletable at will.

## 6. What AD.6 inherits (deliverable 5)

- **Tier-2 checklist = §3's runs 2–3 verbatim on the real post-rewrite clone:** `cargo xtask fetch-assets` (starter; correct subset, rails green) then `cargo xtask fetch-assets --all` (19 live downloads, every pin verifies, rail + verify-assets green, second run all-stamps no-op).
- **Operational rails minted by §3a:** (i) run pack installs only AFTER the tree's final git state — never rebase/checkout across the members-tracked boundary post-install (or `--repair` afterward); (ii) after a hard crash, delete the stale `.pack-state/lock` (the error message instructs this).
- The deletion list for the rewrite is exactly the committed packlists (validated: 90,013 removals + 23,205 cohabitants retained = clean guard).
- `pack-guard.yml` activates on push resume; keeplist shrinks automatically as quarantine leaves (subset semantics).
- The 202 plain-blob files (§5.3) and the AlkaKrab/quarantine removals per the ratified partition.

## 7. Starter-profile proposal (deliverable 3 — RATIFY AT GATE; veto = one-line manifest edit)

| pack | size | rationale (consumer evidence) |
|---|---|---|
| `hdri-catalog` | 45.0 MiB | `hdri_catalog.rs`/`biome_material.rs` parse `assets/hdri/hdri_catalog.toml` and resolve its biome/time-of-day HDRI matrix from this pack |
| `kenney-audio` | 23.8 MiB | `audio_bridge.rs` scans `assets/audio` at editor startup; without it the Audio panel is silent |
| `kaykit-complete` | 473.0 MiB | `entity_panel.rs` archetype `default_mesh()` hardcodes KayKit for every archetype; `entity_catalog.rs`'s first scan target — without it "Add Entity" shows placeholders |

**Total: 541.8 MiB (3.5% of --all).** Minimal fallback if you want smaller: drop kaykit → **68.7 MiB** (entity catalog shows placeholders). Notable exclusions with reasons in the manifest `[profiles]` comment (road-to-vostok 1.0 GiB on-demand-only; models-props 599 MiB no net-new consumer; scene-* 7.7 GiB biome-gated; textures-environment 3.4 GiB no live consumer found; pine-forest 1.5 GiB consumer is the §5.2 broken path; kenney-3d/2d-ui browser fodder; materials-src archival).

---

*Zero LFS network operations (release downloads only; sim clone used `GIT_LFS_SKIP_SMUDGE=1` and local-store reads). Zero pushes; all commits local, hashes above. The rehearsal tree lives at `d:/tmp/ad5_rehearsal/tree` (session-local, disposable); logs at `d:/tmp/ad5_rehearsal/logs/`.*
