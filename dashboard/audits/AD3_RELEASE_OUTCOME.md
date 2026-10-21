# AD.3 — Pack build → local-loop → release upload → live-loop (outcome)

**Date**: 2026-07-07 | **Session**: AD.3 (execute the ratified partition). | **Method**: every count/byte/sha256 from tool output this session (`git lfs ls-files` HEAD enumeration, `zipfile`/`sha256` build, `cargo xtask fetch-assets`, `gh release`). Zero git-LFS network operations (embargo). Zero `git push` of any kind — the `assets-v1` tag is a **server-side tag created by the release API** on an already-remote commit.

**Release**: <https://github.com/lazyxeon/AstraWeave/releases/tag/assets-v1> — published, tag `assets-v1` → **`5b2c6c8bd`** (#197, the ratified anchor; predates the planned AD.6 history rewrite). 18 assets, all `state=uploaded`, sizes byte-matched.

## 1. Scope + Phase-0 preconditions

Scope = the pack bucket minus the ratified `materials-src` deferral: **89,965 files / 19,827,393,808 B**. `materials-src` (66 files, `assets_src/`) got no zip, no entry, no placeholder (ratified: cut after AD.4, `THIRD_PARTY_LICENSES.md` §8.4).

| # | precondition | result |
|---|---|---|
| 0.1 | `gh auth` + `repo` scope | PASS (`lazyxeon`) |
| 0.2 | ratified commits `20aadc5bc`+`7b564254c`; G-3 amendment in manifest | PASS |
| 0.3 | `cargo test -p xtask` | PASS (8/8) |
| 0.4 | staging disk ≥ 45 GB | PASS (314 GB free on D:) |
| 0.5 | release-tag ancestry | **reconciled** — `origin/main` (#198) is not an ancestor of HEAD, but #198 is **asset-neutral** and the 89,965-file pack bucket is byte-identical at `origin/main`, `5b2c6c8bd`, and HEAD. Director ratified the tag anchor as **`5b2c6c8bd`** (#197 — on remote, ancestor of HEAD, asset-identical to `origin/main`). |

## 2. Phase-1 cross-foot + split map

Member list rebuilt from the ratified record: 91,293 LFS paths − quarantine 1,094 − sample 114 − retained 54 − `assets_src` 66 = **89,965 / 19,827,393,808 B** (verified exact; each AD.1.B/C quarantine subset matched documented count **and** bytes). All 89,965 files confirmed **smudged** on disk (disk size == LFS size for every one) → build read real content, embargo-safe.

18 zips (G-3 name-prefix split within >2 GiB directory boundaries; **0 slug-families straddle zips**; all ≤ 2 GiB):

| pack (zip) | files | uncompressed B | zip B | sha256 (prefix) |
|---|--:|--:|--:|---|
| `hdri-catalog` | 8 | 59,567,345 | 47,141,993 | `3fe3df8e3e8dde40…` |
| `kaykit-complete` | 17,443 | 1,138,596,888 | 495,990,095 | `f63c732b14787377…` |
| `kenney-2d-ui` | 50,073 | 169,239,218 | 163,196,103 | `8ab6a2696b30be1d…` |
| `kenney-3d` | 19,408 | 502,195,740 | 159,378,023 | `afebe5a16635d3fc…` |
| `kenney-audio` | 1,302 | 28,615,951 | 24,942,874 | `a6ec7a588d3c0630…` |
| `models-props` | 469 | 1,206,599,979 | 628,284,687 | `5c90bd1370baa262…` |
| `road-to-vostok` | 225 | 1,548,181,668 | 1,091,569,518 | `e1e8031420b00399…` |
| `scene-namaqualand-meshes-a` | 86 | 1,895,095,524 | 1,884,197,790 | `73d5e8c5a8a5b61c…` |
| `scene-namaqualand-meshes-b` | 53 | 1,555,000,156 | 1,444,511,605 | `d35e443d4ca304cb…` |
| `scene-namaqualand-textures-a` | 75 | 1,898,234,659 | 1,898,206,572 | `b4dd4215113911e8…` |
| `scene-namaqualand-textures-b` | 48 | 436,006,170 | 434,317,261 | `64a73a205fa8a2b5…` |
| `scene-verdant-trail-meshes` | 128 | 739,853,056 | 359,288,885 | `889165f3793ba5e6…` |
| `scene-verdant-trail-textures-a` | 77 | 1,899,300,634 | 1,897,159,875 | `c7b879f307130273…` |
| `scene-verdant-trail-textures-b` | 61 | 248,292,977 | 246,560,786 | `0acf874c2d180ef9…` |
| `textures-environment-a` | 86 | 1,899,806,861 | 1,129,109,662 | `06662f13ab33dc5f…` |
| `textures-environment-b` | 159 | 1,899,559,406 | 1,505,386,660 | `7d1edbb56d3f0667…` |
| `textures-environment-c` | 191 | 1,028,460,069 | 895,276,470 | `5ad2e11855d67acd…` |
| `textures-pine-forest` | 73 | 1,674,787,507 | 1,655,407,274 | `be97a962febaa6e0…` |
| **total (18)** | **89,965** | **19,827,393,808** | **15,959,926,133** | |

Full sha256 (64-hex) pins live in `assets/packs.manifest.toml` and the release notes. Each zip stores members at their **full repo-relative path** and embeds `_PACK_MANIFEST.txt` + `_ATTRIBUTION.txt` (the RtV zip's attribution carries the Wayback + unpublication note). `unpack_to` = the zip name (distinct dest per split zip; the tool wipes dest per pack).

## 3. Local-loop evidence (pre-upload, against local zips)

- **Run 1** (clean → fetch): 18/18 verified + unpacked + stamped. One environmental retry: `kaykit-complete` (17,443 files) failed the atomic `fs::rename` with Windows "Access is denied (os error 5)" — Defender/indexer handle-race on the freshly-written tree; `--pack kaykit-complete` retry succeeded immediately.
- **Run 2** (idempotency): 18 up-to-date, 0 fetched.
- **Run 3** (corrupt one stamp): `textures-pine-forest` stamp zeroed → that pack alone re-fetched, others skipped; steady-state back to 18 up-to-date.
- **Content proof**: all 18 packs member-count-exact, **89,965 / 89,965** unpacked assets, 36/36 spot sha256 vs originals — PASS.

## 4. Live-loop evidence (post-publish, real download from release URLs)

Full protocol ran (not the fallback): `assets/packs/` cleaned → `cargo xtask fetch-assets` downloaded all 18 from `https://github.com/lazyxeon/AstraWeave/releases/download/assets-v1/…` (~15.96 GB real HTTPS traffic — release assets, not LFS, embargo-safe).

- **Live Run 1**: 18 fetched, **18 sha256-verified against the unchanged pins**, 0 failed, 18 stamped. (The pins are the trust anchor; the source moving local→release did not change them — proven.)
- **Live Run 2**: 18 up-to-date, 0 fetched — idempotent.
- **Content proof (on downloaded packs)**: **89,965 / 89,965**, 36/36 spot sha256 vs originals — PASS.

## 5. Worktree unchanged + nothing pushed

- `git status` tracked change: **only `assets/packs.manifest.toml`** (the sanctioned live entries). No asset file moved/modified/deleted; `assets/packs/` is gitignored. No `.gitattributes` change.
- Local `HEAD` = `20aadc5bc`, on no remote branch — push-freeze intact. The only remote mutation is the `assets-v1` tag + release (created via the release API, not `git push`).

## 6. Open items handed forward

1. **`materials-src` cut waits on AD.4** (the 66 `assets_src/` files; ratified deferral).
2. **1-byte `assets/textures/grass_hd/grass` stray** joins the next hygiene pass (non-asset, outside the LFS partition).
3. **D8 — GitHub-side LFS storage reclamation** (~22 GB) still pending at AD.6 (separate from this release).
4. **Tool-hardening (fetch_assets.rs)**: add retry-with-backoff on `fs::rename` `os error 5` (the Windows Defender race that hit `kaykit-complete` on the local Run 1). Not fixed here (out of AD.3 scope); the staged-unpack design is sound, only the final atomic swap races.
5. **AD.6 restore-location decision** (out of AD.3): packs restore under `assets/packs/<name>/<repo-relative-path>`; how restored assets reconnect to the engine's original load paths (relocate/symlink/config) is an AD.6 design question, not yet resolved.
