# AD.1.C residual-trace evidence — `assets/textures/{pbr,Fabrics,grass_hd,models}` (2026-07-06)

Evidence for the AD.1.C trace of the 100-file subdirectory residual that AD.1.B flagged out of scope inside `textures-environment` (the last untraced set blocking that pack). Scope is exactly those 100 files: `pbr/` (56), `Fabrics/` (24), `grass_hd/` (5 — 4 LFS + 1 stray), `models/` (15). Count reconfirmed at HEAD 2026-07-06 (`git ls-files`). Paths recorded in **index casing** (`Fabrics/` — one of the three case-mismatch directories; the registry and worktree spell it `fabrics/`).

Method = the ratified AD.1 standard, unchanged from AD.1.B:

- **Poly Haven name-linked tier** (AD.1 §3): an API-verified slug (`https://api.polyhaven.com/info/<slug>` → HTTP 200 with `name`/`type`/`authors`) + Poly Haven's site-wide CC0 = TRACED. The **ratified matching standard** (director, AD.1.C): a trailing-numeral variant name-link is admissible (`forrest_ground_01`/`moss_01` precedent); a word-substitution name-link is not (`pine_trunk` precedent).
- **First-party generator identity** (AD.1 §4 tier): a file emitted by an in-repo deterministic generator is first-party CC0/MIT — the same tier as the procedural `sand`/`snow` materials.
- Binary per family: TRACED or QUARANTINE. No "probably fine." All API calls are plain HTTP (embargo-permitted); no `git lfs` network command was run.

## Captures

- `polyhaven_api/` — raw `https://api.polyhaven.com/info/<slug>` JSON for the three new `Fabrics/` slugs (`fabric_leather_01`, `hessian_230`, `rough_linen`) and the (already-AD.1-verified) `grass_medium_01`, all captured 2026-07-06 at HTTP 200. `type:1` = texture, `type:2` = model.

## Verdicts

| subdir | LFS files | verdict | evidence |
|---|--:|---|---|
| `assets/textures/pbr/` (`PBR_{2K,4K}/{Dirt_Mud,Moss_Ground,Sand_Desert,Stone_Terrain_Rock}/*`) | 56 | **TRACED — first-party procedural CC0** | `tools/pbr_gen/generate_pbr_textures.py` header `# CC0 / Public Domain`; `MATERIALS = {Stone_Terrain_Rock, Dirt_Mud, Sand_Desert, Moss_Ground}` synthesised via deterministic `rand_sine_noise` with **fixed seeds** (200/500/600/700), `save_material` emitting exactly `{BaseColor,Normal,Roughness,Metallic,Height,AO,ORM}` at `--res 2048`/`4096`. On-disk dims confirmed 2048²/4096². Structural match is exact (4 materials × 7 channels × 2 resolutions = 56). |
| `assets/textures/Fabrics/` | 24 | **TRACED — Poly Haven CC0** | `fabric_leather_01` (Rob Tuytel), `hessian_230` (colormass/Rico Cilliers), `rough_linen` (colormass/Rico Cilliers) all API-200 (captures here). The two `*_1k.glb` are Poly Haven preview-mesh bundles inheriting the slug's CC0 (same pattern as the traced `coast_sand_01_1k.glb`). |
| `assets/textures/grass_hd/` | 4 | **TRACED — Poly Haven CC0** | `grass_medium_01` API-200 (Rob Tuytel/Rico Cilliers) — already AD.1-verified (§3.3/§11.1); the 4 LFS maps (`_alpha`/`_diff`/`_dry_diff`/`_rough`) inherit it. |
| `assets/textures/models/houses/` (`house1..5_tex{1,2,3}.png`) | 15 | **QUARANTINE** | textures for `house1..5.glb`, which AD.1.B QUARANTINED (no Kenney byte-twin; DIFFER hashes affirmatively rule out Kenney lineage). Their textures are equally untraceable — no slug, no source doc, single bulk commit `54d10f736`. |

**Non-LFS stray**: `assets/textures/grass_hd/grass` is a 1-byte plain git blob (a lone newline), not an asset — outside the 91,293-path LFS partition. Recommend deletion as a stray; no license question.

## Roll-up

Of the 99 LFS residual files: **84 TRACED / 226,760,432 B** (56 pbr first-party + 24 Fabrics + 4 grass_hd) · **15 QUARANTINE / 25,879,389 B** (models/houses). The 15 join the ratified quarantine; the 84 stay pack-eligible in `textures-environment`. After AD.1.C the `textures-environment` tree has **zero untraced files**.
