# E4 Cook-Path Recon — What's Actually There (build vs. wire vs. relabel-and-defer)

| Field | Value |
|---|---|
| **Campaign** | R-series · **M2 / E4 recon** (lead M2 capability item) |
| **Mode** | READ-ONLY recon. Zero code changed. |
| **Branch / base** | `campaign/roadmap` (`4310427`, M1 complete) |
| **Date** | 2026-06-30 |
| **Authority** | R.1 roadmap M2 E4: *"asset-pipeline → VP-or-relabel: wire the BC7/KTX2 cook path to a live caller (e.g. aw_asset_cli or the editor's asset import), OR drop the present-tense lib.rs:4 claim + the unused editor dep."* R.0.B verdict: `astraweave-asset` FALSE-PRODUCTION-READY (hollow). |
| **Verdict status** | Stays FALSE-PRODUCTION-READY until E4 resolves it — but the **subject crate is `astraweave-asset-pipeline`, not `astraweave-asset`** (naming precision, §0 below). |
| **Load-bearing finding** | The cook code **exists and is partly real, but is fragmented across 3 disconnected implementations, none wired end-to-end**; the wired render path consumes **raw PNG → RGBA8** and never uploads a GPU-compressed format. **v1.0 does not require cooking.** → **RELABEL-AND-DEFER is viable** (the cheapest route). |
| **Gate** | HARD STOP for director review. E4's scope is **proposed, not enacted**. |

This note is the recon's diagnostic deliverable. The build/wire/relabel work is a **later beat after the scope is ratified** — it is **not** done here.

---

## §0. Naming precision (resolve before anything else)

The R.0.B verdict says **"`astraweave-asset` FALSE-PRODUCTION-READY"**, but the evidence puts the false claim on a *different crate*:

- **`astraweave-asset`** (the loader crate): `lib.rs:4` is `use notify::Watcher;` — **no production-ready claim anywhere** in it. It is a **legitimately-working, widely-consumed** loader/watcher/manifest crate (glTF/GLB mesh loading, format *recognition*, PNG import, `.blend` import, hot-reload watching, GUID/manifest DB, Nanite preprocess, cell loading). Its only "complete/fully" strings are technical or honest *under*-claims (e.g. `lib.rs:930` *"CubicSpline, // Not fully implemented yet"*). It is **not** hollow on the cook axis — it never claims to cook.
- **`astraweave-asset-pipeline`** (the compression crate): `lib.rs:4` reads verbatim **`//! This crate provides production-ready asset processing:`** ([lib.rs:4](../../astraweave-asset-pipeline/src/lib.rs#L4)). This is the FALSE-PRODUCTION-READY anchor and the **exact `lib.rs:4` the roadmap names**.

The roadmap **header is correct** ("asset-pipeline → VP-or-relabel"; "lib.rs:4 claim"; "unused editor dep" — all point to `astraweave-asset-pipeline`); the R.0.B crate *name* ("astraweave-asset") is shorthand/imprecise. **Recommendation: re-anchor the verdict to `astraweave-asset-pipeline`** so the FALSE-PRODUCTION-READY label sits on the hollow crate and does not mislabel the working loader.

---

## §1. The actual present-tense overclaim (FALSE-PRODUCTION-READY anchor) — FOUND, current

**[astraweave-asset-pipeline/src/lib.rs:2-7](../../astraweave-asset-pipeline/src/lib.rs#L2-L7):**

```rust
//! Asset Pipeline - Texture compression and mesh optimization for AstraWeave
//!
//! This crate provides production-ready asset processing:
//! - **Texture Compression**: BC7 (desktop), ASTC (mobile)
//! - **Mesh Optimization**: Vertex cache, overdraw reduction
//! - **Validation**: Quality checks, size verification
```

The claim **did not shift** — it is live at exactly `lib.rs:4`. It is false **not** because the code is a stub (the BC7 *is* real, §2) but because the crate is an **island with zero production callers** — "production-ready" implies a production caller, and there is none. The `asset.md` trace (rev 1.1, 2026-06-25) already names this: **§6 "Trap: The `astraweave-asset-pipeline` crate looks like the production asset-processing pipeline … polished docs, tests, benches, declared as a dep of `tools/aw_editor`."**

> The recon's parallel asset-crate agent reported "overclaim shifted_or_absent" — that was a **false negative**: it searched `astraweave-asset`, not `astraweave-asset-pipeline`. Verified directly here.

---

## §2. What the cook-path actually is — THREE fragmented implementations, none wired

The "wire the cook-path" framing assumes one cook path. There are **three**, plus a fourth (consume) half — all disconnected:

### (1) `astraweave-asset-pipeline/src/texture.rs` — REAL BC7, but island + no container
- **`compress_bc7`** ([texture.rs:63-103](../../astraweave-asset-pipeline/src/texture.rs#L63-L103)) calls `intel_tex::bc7::compress_blocks` ([:88-89](../../astraweave-asset-pipeline/src/texture.rs#L88-L89)) — **real, production-quality BC7** (feature `bc7`, default-on). **Not a stub.**
- **`compress_astc`** ([:122-178](../../astraweave-asset-pipeline/src/texture.rs#L122-L178)) shells out to `basisu` CLI; **`transcode_basis_to_bc7`/`_astc`** ([:184-247](../../astraweave-asset-pipeline/src/texture.rs#L184-L247)) use the `basis_universal` crate. Real but external-tool / transcode-only.
- **Limits:** emits only a **raw BC7 block stream — no KTX2/DDS container, no mipmaps, no manifest**. **Zero callers** anywhere (editor grep for `astraweave_asset_pipeline` = no matches; `asset.md` §5:229-232 "In-design", §6:249 "In-design (no caller)"). One core test is `#[ignore]`d ([:298-316](../../astraweave-asset-pipeline/src/texture.rs#L298-L316) — *"BC7 mode byte validation needs investigation"*).

### (2) `tools/aw_asset_cli/src/texture_baker.rs` — STUBBED encoder + real container
- **`compress_to_bc`** ([texture_baker.rs:412-519](../../tools/aw_asset_cli/src/texture_baker.rs#L412-L519)) is an explicit **placeholder**: doc *"Simple BC block compression (placeholder implementation) … In production, use intel_tex, basis_universal"* ([:412-413](../../tools/aw_asset_cli/src/texture_baker.rs#L412-L413)); *"TODO: Replace with proper BC encoder"* ([:435](../../tools/aw_asset_cli/src/texture_baker.rs#L435)). It stores each 4×4 block's **average color** with endpoints duplicated and all indices zeroed → **flat/monochrome blocks**, not valid BC7.
- **`write_texture_with_mipmaps`** ([:218-364](../../tools/aw_asset_cli/src/texture_baker.rs#L218-L364)) is a **real, hand-rolled KTX2 container writer** (magic, header, level index, DFD, mip data) — but carries the **DFD sRGB bug**: transfer function hardcoded to `1u32` (sRGB) regardless of `ColorSpace::Linear` ([:392-393](../../tools/aw_asset_cli/src/texture_baker.rs#L392-L393)). Normal/data maps are mislabeled sRGB.
- The `bake_texture` subcommand is **uncalled and untested** (no `test_*bake`).

### (3) `tools/aw_asset_cli/src/main.rs` — external-tool cook, hard-errors without them, never invoked live
- `process_texture` ([main.rs:298-370](../../tools/aw_asset_cli/src/main.rs#L298-L370)) shells out to `toktx`/`basisu`; if **neither is on PATH it hard-errors** (no fallback). `cook_pipeline` invokes it but **is never called by engine/editor/CI** (CI builds the binary but doesn't run cook). Standalone CLI only.

### (4) Consume side — `astraweave-render/src/material_loader.rs` — its OWN decoder, RGBA8-only upload
- Decodes PNG → RGBA8 (`load_rgba`, [:284-295](../../astraweave-render/src/material_loader.rs#L284-L295)); **even decodes KTX2/BC → RGBA8** (`load_ktx2_to_rgba`, [:308-508](../../astraweave-render/src/material_loader.rs#L308-L508)). `build_arrays` Lanczos3-resizes to a **hardcoded 1024²** ([:523-524](../../astraweave-render/src/material_loader.rs#L523-L524)) and uploads `Rgba8UnormSrgb`/`Rg8Unorm`/`Rgba8Unorm` ([:596-611](../../astraweave-render/src/material_loader.rs#L596-L611)). **No `Bc7RgbaUnorm`/`Bc5`/ASTC GPU-compressed upload exists.** It imports `aw_asset_cli::{ColorSpace, TextureMetadata}` only to **read** `.meta.json` ([:7, :52-53](../../astraweave-render/src/material_loader.rs#L52-L53)) — a data consumer, not a cook caller.

**Cook-path classification:** *partly real, wholly unwired.* The one crate that does real in-process BC7 (asset-pipeline) writes no container; the crate that writes a container (aw_asset_cli) uses a fake encoder or external CLIs; the consume path decompresses everything back to RGBA8 and never uploads compressed. **No single path runs raw-texture → real-BC7 → KTX2-container → manifest → GPU-compressed-upload.**

> **Scope-discipline flag (pre-existing, surface-don't-fix):** this is the CLAUDE.md "never build a second implementation of a logical system" axiom already violated — **3 parallel cook impls + 1 parallel decoder** of the texture-compression pipeline. `asset.md` §11 already carries this as an open question (*"Is `astraweave-asset-pipeline` intended to replace `aw_asset_cli`'s in-house compression, or are both intentional? … the CLI duplicates its function"*).

---

## §3. The unused editor dep — confirmed, and it's asset-**pipeline** (not asset)

- [tools/aw_editor/Cargo.toml:82](../../tools/aw_editor/Cargo.toml#L82) — `astraweave-asset = { path = …, features = ["blend"] }` → **USED**: `AssetDatabase` (`main.rs:92/221/440`), `gltf_loader::load_all_meshes_merged` (`terrain_integration.rs:471`), `blend_import::{BlendImporter, …}` (`main.rs:6799/7224`). Load-bearing.
- [tools/aw_editor/Cargo.toml:99](../../tools/aw_editor/Cargo.toml#L99) — `astraweave-asset-pipeline = { path = … }` → **UNUSED**: workspace grep for `astraweave_asset_pipeline`/`asset_pipeline` in `tools/aw_editor` = **no matches**. This is the roadmap's "unused editor dep." `asset.md` §11:363 flags it: *"Either a planned-but-unwired integration or a stale dependency line."*
- Footnote: the editor's `import_texture_with_compression()` (`main.rs:1416-1511`) is **misnamed** — it `rgba.save()`s **PNG only** (`:1474`), no compression. Tangential honesty gap, not E4-blocking.

---

## §4. v1.0-scope finding — does the wired render path require cooked BC7/KTX2? **NO.**

**Evidence the render path consumes raw/uncompressed:**
- All texture upload normalizes to **uncompressed RGBA8 at a hardcoded 1024²** (§2.4). Even a cooked KTX2 is decoded back to RGBA8 on load — the renderer has **no GPU-compressed upload path at all**.
- Live biome layers are **PNGs without `.meta.json`**, so `format_from_metadata` falls back to RGBA8 defaults.
- Editor authoring uses raw paths throughout (glTF via `gltf_loader`, `.blend` via `blend_import`, PNG via the `image` crate for previews) — none require cooked formats.

**Independent corroboration (TAQ campaign, closed 2026-06-02, `docs/audits/terrain_asset_quality_outcome_2026-06.md`):** D-core verdicted the uncompressed footprint **ACCEPTABLE** — 80 MiB/active 5-layer pack = **31% of the 256 MB soft texture budget**. TAQ §5.1 explicitly hands the cook-path fix to *"its own engine/compression-pipeline session — not asset-quality … an **optimization, not a correctness gap**."*

**Conclusion:** v1.0 can fully **author and render with raw PNG/glTF**. Cooking (BC7/KTX2 + GPU-compressed upload) is a **VRAM/load-time optimization, deferrable post-v1.0**. And it is a *bigger* deferral than "fix the encoder": realizing the win also requires a **new GPU-compressed upload path** in `astraweave-render` (which does not exist today), so cooking pays off **zero** until the consume side changes too.

---

## §5. E4's real nature & proposed v1.0 scope (PROPOSED — for director ratification)

**Nature:** The roadmap's "wire the cook-path" framing is **partly mis-shaped** (the M1-recurring pattern). The cook code is **not absent** and **not merely stubbed** — it is **fragmented and unwired**, with the real BC7 already written (asset-pipeline) but disconnected from the container writer (aw_asset_cli) and from a consume-as-compressed path (render has none). A genuine **build** of E4 is therefore *large and multi-crate*: unify the 3 cook impls into one path **+** add a GPU-compressed upload path in `astraweave-render` **+** wire a live caller (editor import or a build step). That is real capability work — and **v1.0 does not need it** (§4).

**Proposed v1.0 scope: RELABEL-AND-DEFER** (cheapest, correct — the A2-arbiter-deferral analogue). Concretely, a doc-and-metadata-only beat:

1. **Relabel the overclaim** — change `astraweave-asset-pipeline/src/lib.rs:4` from *"This crate provides production-ready asset processing"* to an honest in-design / not-wired status (align the doc-comment with `asset.md`'s existing "In-design" classification).
2. **Resolve the unused editor dep** — remove `astraweave-asset-pipeline` from `aw_editor/Cargo.toml:99` (clean), or comment it as planned-but-unwired. Closes `asset.md` §11:363.
3. **Re-anchor the verdict** to `astraweave-asset-pipeline` (§0), and confirm `astraweave-asset` (loader) is wired/working — it should *not* carry a FALSE-PRODUCTION-READY label.
4. **Defer the cook-path build** (real BC7 → container → manifest → GPU-compressed upload → live caller) to the **post-v1.0 engine/compression-pipeline session TAQ §5.1 already owns**. Optionally fold in the DFD sRGB bug fix (`texture_baker.rs:393`) and the duplicate-impl consolidation (`asset.md` §11:362) as that session's scope.
5. **Close the two `asset.md` §11 open questions** (replace-aw_asset_cli? / unused-editor-dep?) in the same doc-only commit, and bump the trace's revision history.

**Why relabel-and-defer over build:** v1.0 authoring + rendering is fully functional on raw PNG/glTF (§4); the compressed-texture optimization yields nothing until a render-side GPU-compressed upload path is also built; and TAQ already verdicted the uncompressed footprint acceptable and assigned the optimization to a different owner. Building E4 now would be scope-creep against a non-blocking optimization.

**This is a director-ratifiable scope decision — proposed, not decided.**

---

## §6. Evidence ledger (all personally verified or cross-checked)

| Claim | Evidence |
|---|---|
| Overclaim location + wording | `astraweave-asset-pipeline/src/lib.rs:4` — *"This crate provides production-ready asset processing"* |
| `astraweave-asset` has no overclaim | `lib.rs:4` = `use notify::Watcher;`; only honest under-claim at `lib.rs:930` |
| Real BC7 exists (not a stub) | `astraweave-asset-pipeline/src/texture.rs:88-89` `intel_tex::bc7::compress_blocks` |
| Real BC7 is an island | editor grep `astraweave_asset_pipeline` = no matches; `asset.md` §5:229-232, §6:249, §11:362; agent: zero workspace callers |
| Placeholder encoder | `aw_asset_cli/texture_baker.rs:412-435` ("placeholder implementation", "TODO: Replace with proper BC encoder", avg-color flat blocks) |
| DFD sRGB bug | `aw_asset_cli/texture_baker.rs:392-393` (transfer function hardcoded `1u32`/sRGB) |
| Real KTX2 container writer | `aw_asset_cli/texture_baker.rs:218-364` |
| External-tool cook, never invoked live | `aw_asset_cli/main.rs:298-370` (toktx/basisu, hard-error without); cook_pipeline no live caller |
| Render consumes raw RGBA8 | `astraweave-render/src/material_loader.rs:284-295`, `:523-524` (1024² hardcoded), `:596-611` (RGBA8 defaults) |
| Render decodes KTX2→RGBA8, no compressed upload | `material_loader.rs:308-508` (decode path); no `Bc7RgbaUnorm` GPU upload anywhere |
| Editor asset dep USED | `aw_editor/Cargo.toml:82`; `main.rs:92/221/440`, `terrain_integration.rs:471`, `main.rs:6799/7224` |
| Editor asset-pipeline dep UNUSED | `aw_editor/Cargo.toml:99`; grep no matches |
| Footprint acceptable / cooking = optimization | TAQ outcome 2026-06-02 §4, §5.1; `asset.md` rev 1.1 §5/§6/§11 |

*Recon complete. E4 scope awaits director ratification. No code changed.*
