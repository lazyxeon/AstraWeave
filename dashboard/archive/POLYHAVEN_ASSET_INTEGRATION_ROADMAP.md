# PolyHaven Asset Integration Roadmap (High-Quality 3D Assets)

Status: Proposed • Owner: AstraWeave Copilot • Date: 2025-10-17

This roadmap delivers production-grade integration of PolyHaven’s CC0 textures, HDRIs, and (optionally) models into AstraWeave. It covers pipeline, tooling, runtime integration, performance budgets, licensing, and validation—end to end.

---

## Goals & Constraints

- High-quality, production-ready assets (PBR textures, HDRIs)
- CC0 license only (PolyHaven) – zero attribution required, redistributable
- Deterministic, offline-friendly pipeline with checksums and versioning
- Cross-platform (Windows/macOS/Linux); PowerShell scripts provided, bash variants optional
- GPU-friendly formats (BCn + KTX2) and channel packing (ARM: AO/Rough/Metal or MRA)
- Material arrays & stable indices compatible with existing renderer bindings (wgpu 25)

---

## Deliverables (12–18 days total)

- scripts/setup_assets.ps1 – Scaffolds asset tree and starter TOML
- scripts/download_polyhaven_samples.ps1 – Pulls curated starter set (10 textures, 3 HDRIs)
- Material TOML schema + examples: assets/materials/polyhaven/materials.toml
- Optional: Texture compression pipeline to KTX2 (toktx/basisu) with presets
- Renderer loader wiring using existing MaterialManager (TOML → GPU arrays)
- Runtime streaming + fallback behavior
- CI hooks for asset presence checks (no large binaries in git by default)
- Documentation (this roadmap + quick start)

---

## Directory Structure (authoritative)

```
assets/
  hdri/                       # HDR environment maps
    polyhaven/
      <name>/
        <name>_1k.hdr         # or .exr, multiple resolutions allowed
        <name>_2k.hdr
        LICENSE.txt           # CC0 summary (optional)
  materials/
    polyhaven/                # PBR material libraries 
      materials.toml          # library catalog
      arrays.toml             # optional pre-baked GPU array mapping (stable IDs)
      <material_name>/
        albedo.png            # or *_diff_*.png
        normal.png            # *_nor_gl_*.png
        ao.png                # optional; or packed
        roughness.png
        metallic.png
        arm.png               # optional packed AO/Rough/Metal
        LICENSE.txt
  models/                     # Optional: PolyHaven models (FBX/GLB)
    polyhaven/
      <model_name>/...
  cache/                      # Generated artifacts (ktx2, mips, thumbnails)
```

Design principles:
- Source-of-truth lives under assets/materials/polyhaven/<name>
- Generated artifacts go to assets/cache (never commit large binaries)
- TOML catalogs provide indirection (friendly names → concrete files) with stable indices

---

## Material TOML Schema

Example: assets/materials/polyhaven/materials.toml

```toml
# Material catalog for PolyHaven sets (CC0)
# Paths are relative to this TOML file

[material.rock_sand]           # Stable logical name
albedo = "rock_sand/albedo.png"
normal = "rock_sand/normal.png"
# Preferred packed order: ARM = AmbientOcclusion (A), Roughness (R), Metallic (M)
arm    = "rock_sand/arm.png"     # optional; else specify individual textures below
# metallic = "rock_sand/metallic.png"
# roughness = "rock_sand/roughness.png"
# ao = "rock_sand/ao.png"

[material.brushed_metal]
albedo = "brushed_metal/albedo.png"
normal = "brushed_metal/normal.png"
roughness = "brushed_metal/roughness.png"
metallic  = "brushed_metal/metallic.png"
```

Optional arrays.toml (pre-baked stable GPU indices):

```toml
# Assigns stable indices for texture array packing (renderer reads this)
[albedo]
rock_sand = 0
brushed_metal = 1

[normal]
rock_sand = 0
brushed_metal = 1

[arm]
rock_sand = 0
```

---

## Pipeline Overview (E2E)

1) Acquisition
- Download PolyHaven PBR sets (4K recommended; optionally 2K/8K tiers)
- Download HDRIs (2K/4K for runtime; keep 8K for high-quality capture)
- Scripted pulls via direct URLs (PolyHaven CDN), with SHA-256 verification

2) Normalization
- Rename/normalize files to engine conventions (albedo.png, normal.png, roughness.png, metallic.png, ao.png or arm.png)
- Optional color-space corrections (albedo: sRGB; normal/ARM: linear)

3) Packing & Compression
- Pack AO/Rough/Metal into single ARM texture (A=AO, R=Rough, M=Metal)
- Generate KTX2 with BC7 for albedo/ARM, BC5 for normals (or BC7 if needed)
- Generate mipmaps and optional 1K/2K/4K pyramid

4) Cataloging
- Update materials.toml and (optionally) arrays.toml with stable IDs

5) Runtime Loading
- MaterialManager parses TOML, loads textures (KTX2 preferred), and binds arrays
- Stable indices keep shader bindings deterministic across runs

6) Streaming & Fallback
- Attempt async prefetch on scene load; if missing assets, fallback to solid-color PBR
- Log warnings with resolution hints (exact path + expected name)

7) Editor UX (Phase 2+)
- Asset browser panel (thumbnails from cache), drag-and-drop onto meshes
- Live-reload when TOML or files change (notify-based)

---

## Tooling & Scripts

Provided by this roadmap (PowerShell):

1) scripts/setup_assets.ps1
- Creates the full assets/ tree and a starter materials.toml
- Safe to re-run (idempotent)

2) scripts/download_polyhaven_samples.ps1
- Downloads a curated starter set (10 materials, 3 HDRIs)
- Extracts and normalizes names into our conventions
- Writes materials.toml entries automatically

3) Optional (Phase 2): scripts/convert_to_ktx2.ps1
- Uses toktx (KTX-Software) to compress png → ktx2 with mipmaps
- Profiles: albedo/ARM → BC7; normals → BC5

---

## Renderer Integration (existing-compatible)

- Use the current MaterialManager TOML pattern and texture array bindings
- Ensure all textures respect expected bindings in WGSL (group=1):
  - 0: albedo array (sRGB), 1: sampler, 2: normal array (linear), 3: linear sampler, 4: ARM array (linear)
- Prefer KTX2 at runtime; support PNG as dev fallback
- Keep indices stable by arrays.toml or by deterministic ordering of materials.toml keys

Pseudocode wiring (no code changes required immediately):

```rust
// During renderer init:
material_manager
    .load_from_toml("assets/materials/polyhaven/materials.toml")?
    .with_arrays("assets/materials/polyhaven/arrays.toml")?
    .finalize_gpu_arrays(device, queue)?;
```

---

## Performance Targets & Budgets

- Texture memory budget (desktop): 512–1024 MB for materials, 64–128 MB for HDRIs
- Mipmap floor: keep 1K and 2K mips for distant use; stream 4K on demand
- Load time (cold): < 2.0 s to bind 20 materials (KTX2, async IO)
- Rebuild time (compression): 5–10 min for 10 materials on dev machine (one-time)
- Shader sampling: combined array sampling overhead within 0.2 ms/frame @ 1080p

---

## Security, Licensing & Provenance

- Only allow downloads from https://polyhaven.com/ and dl.polyhaven.org
- Embed LICENSE.txt (CC0 summary) alongside each material/HDR folder
- Store SHA-256 checksums in a manifest (materials.toml or separate materials.sha256)
- No git commit of large binaries by default – prefer local download
- Optionally support Git LFS if samples must be versioned

---

## Milestones & Timeline

Phase 1 (Day 1–2): Bootstrapping
- Add scripts/setup_assets.ps1 and scripts/download_polyhaven_samples.ps1
- Create assets/ tree + starter materials.toml
- Pull 3 HDRIs + 10 materials; normalize; run in unified_showcase
- Acceptance: renderer boots with PolyHaven assets; no blocking calls; fallback path logs once

Phase 2 (Day 3–5): Compression & Catalogs
- Integrate toktx into scripts/convert_to_ktx2.ps1
- Convert PNG → KTX2 (BC7/BC5), generate mipmaps, keep source PNGs
- Create arrays.toml with stable ordered indices
- Acceptance: VRAM usage within budget; KTX2 path is primary; perf within targets

Phase 3 (Day 6–8): Streaming & Fallback Polish
- Async prefetch for scene’s required materials; progress logging
- Missing asset UI indicator (console + optional overlay)
- Automated integrity checks (checksum verify job)
- Acceptance: missing assets never crash; fallback graceful; streaming stutter-free

Phase 4 (Day 9–12): Editor & DevX
- Asset browser (thumbnails from cache), click-to-assign materials
- Live-reload on TOML/file changes (notify)
- Documentation for artists (naming conventions, how to add a new material)
- Acceptance: end-to-end workflow from download → compress → assign within 10 min

Optional Phase 5 (Day 13–18): Models (if needed)
- Add GLB/FBX loader path for PolyHaven models
- Mesh preprocessing (tangents, LOD, compression), thumbnails
- Acceptance: 3–5 hero props integrated; metrics within budgets

---

## Acceptance Criteria (per phase)

Functional
- Materials load via TOML; arrays bound without errors
- Renderer shows PBR surfaces and HDR lighting from PolyHaven
- Fallback path operates when files missing (solid-color PBR)

Performance
- Bind 20 materials < 2.0 s cold; frame sampling < 0.2 ms
- VRAM 512–1024 MB budget respected at 4K mips streamed

Quality
- No unwraps in loading path; errors use anyhow::Result + context
- Clear, actionable logs on missing/corrupt assets
- All scripts idempotent, documented, and cross-platform notes provided

---

## Quick Start (Windows PowerShell)

```powershell
# 1) Create folder structure + starter TOML
./scripts/setup_assets.ps1

# 2) Download curated PolyHaven samples (10 materials, 3 HDRIs)
./scripts/download_polyhaven_samples.ps1 -Verbose

# 3) (Optional) Convert PNG → KTX2 with mipmaps
# Requires toktx (KTX-Software) in PATH
# ./scripts/convert_to_ktx2.ps1 -InputDir assets/materials/polyhaven -OutDir assets/cache/ktx2

# 4) Run a demo that uses materials (after wiring, if needed)
cargo run -p unified_showcase --release
```

---

## Risks & Mitigations

- Large downloads/time: start with 2K sets, enable on-demand 4K fetch
- KTX2 toolchain availability: document toktx install; keep PNG fallback path
- Git repo bloat: never commit large binaries; CI checks; optionally Git LFS
- Platform variance: prefer relative paths, avoid case-sensitive mismatches on Windows

---

## Next Actions (Actionable Checklist)

- [ ] Run setup_assets.ps1 to scaffold directories
- [ ] Run download_polyhaven_samples.ps1 to fetch starter library
- [ ] Point MaterialManager to assets/materials/polyhaven/materials.toml
- [ ] Validate renderer output; adjust exposure/tonemapping if needed
- [ ] Decide on KTX2 (enable script + toktx install)
- [ ] Add arrays.toml once first batch stabilized
- [ ] Document artist workflow in docs/ASSET_AUTHORING_GUIDE.md (future)

---

## Appendix A – Naming & Color Space

- albedo.png (sRGB)
- normal.png (Linear, +Y OpenGL tangent space preferred)
- metallic.png (Linear)
- roughness.png (Linear)
- ao.png (Linear)
- arm.png (Linear) channel packing: A=AO, R=Roughness, M=Metallic
- HDRIs: *.hdr/*.exr (Linear); store unclamped, with mips generated by toktx

## Appendix B – toktx Presets

- Albedo/ARM: `toktx --bcmp --uastc 4 --zcmp 18 --genmipmap --assign_oetf srgb` (albedo) or linear (ARM)
- Normal: `toktx --bcmp --normal_map --genmipmap`
- HDRI: `toktx --t2 --genmipmap --assign_oetf linear` (inspect tone mapping later)

## Appendix C – Sample Materials List

- rock_sand, rock_face, cobblestone, brushed_metal, rusted_metal, painted_plaster, oak_wood, walnut_wood, linen_fabric, asphalt

## Appendix D – Sample HDRIs

- spruit_sunrise, kloppenheim_06, venice_sunset
