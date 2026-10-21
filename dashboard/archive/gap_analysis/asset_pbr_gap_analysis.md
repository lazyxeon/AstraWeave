# Asset & Texture PBR Pipeline Gap Analysis

## Current State Overview
- **Texture compression** relies on placeholder BC7/BCn encoders and stub ASTC paths, producing deterministic but low-fidelity output that cannot match production-grade encoders.【F:astraweave-asset-pipeline/src/texture.rs†L95-L268】【F:tools/aw_asset_cli/src/texture_baker.rs†L306-L412】
- **Texture packaging** writes a custom `AW_TEX2` container instead of industry-standard KTX2/UASTC assets, preventing direct ingestion by external DCC tools and engine runtime paths that expect Khronos formats.【F:tools/aw_asset_cli/src/texture_baker.rs†L147-L292】
- **Model ingestion** through `aw_asset_cli` simply copies glTF/GLB payloads without baking GPU-ready mesh buffers, tangents, or meshlet data, and does not capture material dependencies for streaming or validation.【F:tools/aw_asset_cli/src/main.rs†L251-L375】
- **glTF loader** only supports embedded BIN/data URIs and fails on external buffer or texture references, blocking interchange workflows from Blender, Maya, or Substance Painter that emit referenced assets.【F:astraweave-asset/src/lib.rs†L84-L198】
- **Validation tooling** reports heuristic compression/quality estimates but lacks histogram, color-space, or channel-specific audits that would catch authoring regressions in high-resolution maps.【F:astraweave-asset-pipeline/src/validator.rs†L82-L189】
- **Advanced PBR features (Phase PBR-E)** were removed to satisfy bind group limits, leaving clearcoat, anisotropy, sheen, and related shading models unavailable in the shipping renderer.【F:docs/root-archive/BIND_GROUP_CONSOLIDATION_COMPLETE.md†L541-L559】
- **Phase PBR-G** (tooling, validation, documentation) remains unfinished—GPU hot-reload integration is only designed on paper, and the documentation consolidation milestone (Task 6) never started, preventing production onboarding of the current tools.【F:docs/root-archive/PHASE_PBR_G_PROGRESS_REPORT.md†L1-L191】

## Gap Assessment
1. **Texture Fidelity & Portability**
   - Missing production encoders for BC7/ASTC/UASTC and standardized KTX2 emission.
   - No normal-map or ORM channel validation beyond filename inference.
   - Mipmap generation does not enforce power-of-two or resize-to-block alignment.
2. **Model & Material Authoring**
   - Lack of mesh optimization stages (meshopt, Nanite preprocessing) in the CLI pipeline.
   - No automatic tangent/bitangent rebuild or MikkTSpace alignment for imported assets.
   - Dependency graph for textures/materials is empty, so runtime hot-reload and package builds cannot reason about asset bundles.
3. **Interchange Compatibility**
   - Inability to resolve external buffers/images from common DCC exports (Blender, Houdini, Substance, Quixel) blocks adoption.
   - No USD/FBX ingest path or conversion strategy.
4. **Renderer Feature Parity**
   - PBR-E shading is removed; pipeline lacks support for layered advanced materials or clearcoat-style authoring.
   - GPU hot-reload path is specified but not implemented, so authoring iteration requires full rebuilds.
5. **Quality Assurance & Documentation**
   - Validators depend on heuristic thresholds without histogram or channel statistics.
   - Phase PBR-G documentation consolidation outstanding, leaving fragmented onboarding material.

## Phased Roadmap

### Phase 1 – Texture Pipeline Hardening (2–3 sprints)
1. Integrate `basis-universal` or `intel-tex` for deterministic BC7/BC5 encoding with SIMD acceleration; add ASTC 4×4/6×6/8×8 via BasisU transcoding for mobile SKUs.【F:astraweave-asset-pipeline/src/texture.rs†L95-L268】
2. Replace the custom `AW_TEX2` writer with a real KTX2/UASTC exporter (libktx-rs) supporting supercompression (Zstd) and metadata blocks (color space, authoring tool).【F:tools/aw_asset_cli/src/texture_baker.rs†L147-L292】
3. Expand validator metrics to compute per-channel histograms, detect normal-map swizzle errors, and enforce mip dimension alignment; surface failures via CLI strict mode and CI.【F:astraweave-asset-pipeline/src/validator.rs†L82-L189】

### Phase 2 – Asset Ingestion & Dependency Graph (2 sprints)
1. Implement mesh baking in `process_model`: generate GPU vertex/index buffers, MikkTSpace tangents, meshopt cache optimization, and optional nanite meshlet export for high-poly assets.【F:tools/aw_asset_cli/src/main.rs†L295-L375】
2. Populate dependency metadata by scanning glTF materials and texture references so manifests capture texture/mesh relationships for streaming and validation.【F:tools/aw_asset_cli/src/main.rs†L251-L292】
3. Extend glTF loader to resolve external buffers/images, handle sparse accessors, and support glTF extensions used by DCC tools; add regression tests covering Blender, Substance, and Quixel exports.【F:astraweave-asset/src/lib.rs†L84-L198】

### Phase 3 – Interchange & High-Poly Authoring (3 sprints)
1. Add ingestion adapters for FBX (via `fbxcel`) and USD (via `usd-rs`) translating to glTF + material packs, ensuring round-trip from Maya/Houdini pipelines.
2. Wire Nanite preprocessing into the CLI so high-poly meshes automatically emit meshlets and LOD hierarchies for runtime streaming.【F:astraweave-asset/src/nanite_preprocess.rs†L1-L200】
3. Provide Substance/Quixel template configs that map exported texture sets into the standardized bake (naming conventions, color-space tags, channel packing).

### Phase 4 – Renderer Feature Restoration & Iteration Loop (2–3 sprints)
1. Reintroduce PBR-E materials using a dedicated pipeline or bind-group refactor so clearcoat, anisotropy, sheen, and transmission authoring paths are available again.【F:docs/root-archive/BIND_GROUP_CONSOLIDATION_COMPLETE.md†L541-L559】
2. Implement the GPU-side hot-reload flow described in PBR-G Task 3, enabling live material/texture updates in editor builds; gate via automated smoke tests.【F:docs/root-archive/PHASE_PBR_G_PROGRESS_REPORT.md†L74-L104】
3. Integrate layered/material blending workflows (terrain + decal stacks) with authoring presets and validation scenes.

### Phase 5 – Documentation, QA, and Release (1–2 sprints)
1. Complete Phase PBR-G Task 6 documentation consolidation and create an authoritative authoring guide covering CLI usage, interchange caveats, and troubleshooting.【F:docs/root-archive/PHASE_PBR_G_PROGRESS_REPORT.md†L172-L191】
2. Establish golden-render regression suites that compare high-resolution reference captures for hero assets to ensure fidelity regressions are detected before release.
3. Publish SBOM and security review for external encoder dependencies; automate validation in CI.

### Success Criteria
- Engine consumes standardized KTX2/GLTF packages emitted by CLI without manual fixes.
- Advanced PBR materials render at parity with UE5/Unity HDRP reference shots.
- Asset validation CI enforces histogram, compression, and dependency checks on every merge.
- Artists can iterate from Blender/Substance to in-engine preview in under one minute via GPU hot reload.
