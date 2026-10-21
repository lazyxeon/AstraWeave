# AD.4 C7 re-acquisition evidence (2026-07-07)

Fresh AD.1-standard provenance for the six §6-untraceable material families, re-sourced from traced CC0 Poly Haven slugs (director-ratified 2026-07-07). These replace the untraceable `assets_src/materials/` C7 copies (which move pack→quarantine). Full trace table: `THIRD_PARTY_LICENSES.md` §13.1.

## Method

- `polyhaven_api/<slug>.json` — raw `https://api.polyhaven.com/info/<slug>` capture (plain-HTTP, embargo-safe; no LFS traffic). Each is `type: 1` (texture) with `name`/`authors`. Site license CC0: <https://polyhaven.com/license>.
- Acquisition: `tools/astraweave-assets fetch` (Poly Haven Files API) → `assets/_downloaded/polyhaven/<family>/<family>_{albedo,normal,roughness,ao,arm}.png` (gitignored).
- Cook: `tools/material_cook/cook_1k.py` → `assets/materials/derived_1k/<family>{,_n,_mra}.png` (1024² RGBA; mra = R=0/G=roughness/B=AO).

## Slugs

| family | slug | name / author | type |
|---|---|---|---|
| cloth | `fabric_leather_01` | Fabric Leather 01 / Rob Tuytel | texture |
| plaster | `plastered_wall_02` | Plastered Wall 02 / Charlotte Baglioni | texture |
| rock_lichen | `lichen_rock` | Lichen Rock / Rico Cilliers | texture |
| roof_tile | `roof_tiles_14` | Roof Tiles 14 / Rob Tuytel | texture |
| tree_bark | `tree_bark_03` | Tree Bark 03 / Rob Tuytel | texture |
| tree_leaves | `forest_leaves_02` | Forest Leaves 02 / Rob Tuytel | texture |

`rock_moss_set_02` was rejected (type-2 model pack, not a tileable texture); `lichen_rock` selected search-first as the exact texture match for the `rock_lichen` slot.
