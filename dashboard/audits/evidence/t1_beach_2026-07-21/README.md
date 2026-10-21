# T.1 beach acquisition evidence (2026-07-21)

Fresh provenance for the distinct beach material (T-series ratification §2 row-6 amendment, director 2026-07-20). Trace row: `THIRD_PARTY_LICENSES.md` §14.

## Method

- `polyhaven_api/coast_sand_01.json` — raw `https://api.polyhaven.com/info/coast_sand_01` capture (2026-07-21, plain-HTTP): `type: 1` (texture), `authors {"Rob Tuytel": "All"}`, `max_resolution [8192, 8192]`, description "damp coastal sand with brown, rough grain, scattered pebbles and gravel". Site license CC0: <https://polyhaven.com/license>.
- `polyhaven_api/coast_sand_01_files.json` — raw `/files/coast_sand_01` capture: `Diffuse`/`nor_gl`/`Rough`/`AO`/`arm` all present at 1k-8k (suitability = binary PASS at the ≥2K requirement).
- Acquisition: `tools/astraweave-assets fetch` (Poly Haven Files API) → `assets/_downloaded/polyhaven/beach/beach_{albedo,normal,roughness,ao,arm}.png` (gitignored, manifest-pinned; the provider's "metallic" warning is expected — the slug has no separate metallic map).
- Cook: `tools/material_cook/cook_1k.py` → `assets/materials/derived_1k/beach{,_n,_mra}.png` (1024² RGBA; mra packed R=metallic(0)/G=roughness/B=AO from the acquired rough+ao maps — same path as the AD.4 C7 cooks, avoiding the ARM-order trap by construction). Channel measurements in `docs/audits/T1_BEACH_OUTCOME.md`.
