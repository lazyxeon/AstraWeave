# AD.1.B provenance-trace evidence — pack-bucket remainder (2026-07-06)

Evidence for the AD.1.B trace of the four upload-blocked pack-bucket scopes from the ratified AD.3.R partition (G-4 gap). Method: AD.1 §3 name-linked tier — a PolyHaven/ambientCG slug is TRACED CC0 when the asset API returns it (existence) and the source's site-wide CC0 license applies; a slug that 404s with no other source document is QUARANTINE. Kenney/Blocky-Characters files are TRACED via SHA-256 byte-identity to a licensed pack copy (AD.1 §2 method). All API calls are plain HTTP (not LFS operations); no `git lfs` network command was run (embargo).

## Captures

- `polyhaven_api/` — raw `https://api.polyhaven.com/info/<slug>` JSON responses for every **new** slug verified this session (pine_forest scope + textures-loose scope + `coast_sand_01`). Filename prefix denotes the scope (`pineforest_`, `textures_`, `models_`). A present JSON with `name`/`type`/`authors` = HTTP 200 (exists). The decisive 404s (`dead_tree`, `fir_bark`, `fir_trunk_01`, `pine_trunk_01`, `tree_trunk`, `tree_roots_01`, `montaigle_ruins_01`, `pine_cover_01`, `fir_twig`, `ivy`, `tiny_purple_succulant`) are retained where the agent captured the empty/negative response.
- `ambientcg_api/3DTreeStump001.json` — `https://ambientcg.com/api/v2/full_json?id=3DTreeStump001` capture; `assetId: 3DTreeStump001`, `dataType: 3D Model`, download-variant filenames matching the local `*_SQ-1K-JPG*` naming. ambientCG site-wide license is CC0.

## Site licenses (established in AD.1, not re-fetched)

- Poly Haven: <https://polyhaven.com/license> — *"Our assets are all licensed as CC0."*
- ambientCG: site-wide CC0.
- Kenney packs: in-repo `License.txt` per pack (CC0), see `THIRD_PARTY_LICENSES.md` §1.

## Verdict summary (full tables in `THIRD_PARTY_LICENSES.md` §11 and `DISPOSITION_REPORT_imported.md` §8)

| scope | denominator | TRACED | QUARANTINE (new) |
|---|--:|--:|--:|
| 1 `assets/textures/pine_forest/` | 133 / 2,678,108,345 B | 73 / 1,674,787,507 B | 60 / 1,003,320,838 B |
| 2 `assets/textures/` loose (332 of 347) | 332 / — | 313 / 3,860,950,718 B | 19 / 79,306,263 B |
| 3 `assets/models/` loose (530 of 560) | 530 / — | 449 / 24,224,374 B | 81 / 1,742,733 B |
| 4 `assets/tests/textures/` | 18 / 346,863 B | 18 / 346,863 B (Kenney Blocky Characters) | 0 |

New pack→quarantine moves: **160 files / 1,084,369,834 B**. Scope 4 moved 18 files gate-unclassified→pack (TRACED). Full-history bucket cross-foot remains 91,293 paths / 28,243,219,612 B.
