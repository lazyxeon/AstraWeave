---
schema_version: 1
trace_id: asset
title: "Asset System"
description: "Asset + Asset-Pipeline — loading, cell loader, Nanite preprocess, validator/texture/mesh"
primary_crate: astraweave-asset
domain: rendering
lifecycle_status: active
integration_status: wired
owns: [astraweave-asset, astraweave-asset-pipeline]
doc_version: "1.27"
last_verified_commit: f59241e3
---

# Architecture Trace: Asset System

## Metadata

| Field | Value |
|---|---|
| **System name** | Asset System (loading, cell loader, Nanite preprocess, asset pipeline) |
| **Primary crates** | `astraweave-asset`, `astraweave-asset-pipeline` |
| **Document version** | 1.27 |
| **Last verified against commit** | `f59241e3` (parent of keeping the Windows root directory handle on its own and starting every URI from it; that parent kept the root handle in the held list, so a later URI started from the last held directory). Prior `0f5fbee7` (parent of opening each URI component from the held parent handle, checking that directory's final path, capping the held set at 64, and identifying a directory by its 128-bit file id and volume serial; that parent reopens each Windows intermediate from the root's final path, dedupes with the 64-bit file index, and does not cap the held set). Prior `a6706628` (parent of keeping intermediate directory handles on Windows only; that parent also keeps the URI descriptors on Unix). Prior `bc4a5791` (parent of holding each intermediate directory handle for the root lifetime; that parent drops those handles when `open_relative` returns). Prior `ac879829` (parent of delete sharing on the first open of each URI leaf file, with a second open only for a cloud placeholder; that parent shares delete on every Windows open, including each held directory handle). Prior `7425e8b3` (parent of one directory handle for a skinned load, the bytes already read, and directory components that do not need a readable ancestor; that parent opens each directory component read-only, and the cook path copies the model file by path after the jailed read). Prior `346e2b763` (parent of using one directory handle for the container read, every sibling URI, and every image URI; that parent opens the directory again for each image and for the siblings after the container read). Prior `39243c23` (parent of opening the model directory once per load, of opening the model file's parent without a second resolve, and of allowing a cloud-file reparse tag; that parent resolves the directory again for each read and refuses every reparse point). Prior `c5881afe4` (parent of opening each resolved component without following it, and of refusing a file with more than one link; that parent opens the resolved directory from the process directory in one call, and it accepts a file with more than one link). Prior `6d46355d` (parent of the exact final-path compare, the pre-pad buffer check, and resolving the model file once; that parent pads a buffer before it compares `byteLength`, compares final paths through a lossy string and ignore-case, and refuses a model file that is itself a symlink). Prior `9afd7dee6` (parent of resolving the model directory once; that parent opens the caller-supplied directory with no-follow or reparse-point semantics, so a directory that is itself a link fails the regular-file check). Prior `93e3a156` (parent of the directory-handle read and the shared external budget; that parent checks `metadata().is_file()` before `File::open`, caps each URI on its own, and `gltf_loader::load_uri_bytes` joins the process current directory). Prior `939325b5` (parent of the jailed external read; that parent did not reject absolute or percent-encoded traversal URIs before `safe_under`, then `import_buffers` / `import_images` call gltf `read_to_end`, which allocates `st_size + 1`). Prior `0ae2a37b` (parent of the external-URI import; that parent parses the capped slice with `gltf::import_slice`, which returns `external reference in slice only import` for a jailed sibling `.bin` or image). Prior `6eb6bfee` (parent of the capped GLB slice; that parent calls `reject_short_glb_file` and then `Gltf::open` / `gltf::import`, so `Glb::from_reader` still allocates `declared - 12` when the file is sparse or replaced after the check). Prior `2e2012eaa` (parent of the §4 `aravath_runtime` row; that parent already has the `astraweave-asset` dependency in `aravath_runtime/Cargo.toml`, added in `ac5a17dcf`, and omits the row). Prior `7ea1d200` (rebase base; no `reject_short_glb`). `99382af7` is the zstd bump, not the parent of this guard. `cd910720`, `40374e98`, and `a9e434fc` are not ancestors after the rebase onto `7ea1d200`. |
| **Last verified date** | 2026-11-11 |
| **Status** | Active (mixed): glTF CPU loaders + cell loader + AssetDatabase are wired (`astraweave-asset` is VERIFIED-PRODUCTION); Nanite preprocess and the entire `astraweave-asset-pipeline` crate are in-design / dormant scaffolding. **R-series M1 trace-honesty (2026-06-30):** `astraweave-asset-pipeline` was individually **FALSE-PRODUCTION-READY** (R.0.B) — a present-tense over-claim at `lib.rs:4` over an ISLAND crate (no live caller). **R-series M2/E4 relabel-and-defer (2026-06-30):** E4 **resolved by relabel** — the `lib.rs:4` over-claim was rewritten to honest in-design status and the unused editor dependency on `astraweave-asset-pipeline` was dropped, moving the crate **FALSE-PRODUCTION-READY → DORMANT-HONEST**. The BC7/KTX2 **cook path is NOT a v1.0 blocker** — the v1.0 render path consumes raw PNG→RGBA8 with no GPU-compressed-upload path, so cooking is a deferrable optimization **deferred to a post-v1.0 engine/compression-pipeline owner**. See `docs/audits/e4_cook_path_recon_2026-06.md`. |
| **Owner notes** | First trace for this subsystem. `astraweave-asset` is one of the most-consumed untraced crates (9+ Cargo consumers). Derived from forensic data-flow analysis on 2026-06-24. |

---

## 1. Executive Summary

**What this system does:**
Provides CPU-side asset ingestion for the engine — parsing glTF/GLB into mesh/material/skeleton/animation data, deserializing World-Partition cells from RON, maintaining a GUID-keyed asset metadata/dependency database, and offering offline mesh/texture transform utilities (meshlet preprocessing, BC7/ASTC compression, validation).

**Why it exists:**
Decouples raw on-disk asset formats from runtime consumers (render, scene streaming, gameplay), giving each a typed in-memory representation plus a deterministic GUID identity for caching and hot-reload.

**Where it primarily lives:**
- `astraweave-asset/src/lib.rs` — glTF loaders (`gltf_loader`). `reject_short_glb_header` refuses a declared length below 12 bytes. `read_glb` uses it before `Glb::from_slice`. `read_gltf` uses it before `Gltf::from_slice`, including a JSON chunk that starts with `glTF`. On the editor, cook, and runtime paths, one handle covers the container and every URI. The base-taking loaders open the base once per call for their URIs, and the caller supplies the container bytes. `read_gltf_container` returns that handle. `read_path` resolves the caller-supplied model file once, then opens that parent without resolving it again, so a symlink at that file loads. URI components are not resolved. On Linux each component of the opened directory is opened from `/` with `O_PATH`, `O_DIRECTORY`, and `O_NOFOLLOW`, so a load still works when an ancestor is not readable. Other Unix opens those components read-only with `O_DIRECTORY` and `O_NOFOLLOW`. Each URI is opened from that root descriptor, one component at a time, with `O_NOFOLLOW`, and those descriptors are not kept. An intermediate component uses directory flags (`O_PATH` and `O_DIRECTORY` on Linux). A FIFO is rejected instead of blocking. On Windows the held handle is the directory-handle chain plus the final path. Each component of the resolved root is opened with reparse-point semantics. A symlink, a junction, and a mount point are refused. A cloud-file reparse tag is allowed. Delete sharing is on the first open of each URI leaf file, so that name can be replaced while the handle is held; only a cloud placeholder is opened a second time, without delete sharing, and that second open must be the same file. That component's final path must stay under the previous component. `final_path` runs for each resolved-root component, each intermediate directory, and the opened file. Each URI component is opened from the held parent handle. The held set keeps one handle per directory id and fails closed past 64 directory handles. The budget counts the resolved-root chain, so a deep root leaves fewer URI directory slots. Directory identity is the 128-bit file id and the volume serial. On Windows the root directory chain and each distinct intermediate directory stay on `OpenedRoot` until that root drops. A repeated open of the same directory does not add another handle. Those directory handles do not share delete, so a directory the load holds cannot be renamed or removed while they are open. The file's final path must be under the root's final path, compared as exact UTF-16 components. A file with more than one link is refused, and the error names the file. It then refuses a file over 256 MiB before the read. `read_buffers` compares the bytes on hand with `byteLength` before the 4-byte pad. `reject_glb_reader` then refuses a declared length below 12 or past the bytes just read. `mesh_gltf::load_gltf` and `aw_asset_cli` parse that slice with `Gltf::from_slice`. `aravath_runtime` parses the bytes from its own `read_model_bytes` (32 MiB, `MAX_MODEL_BYTES`) after `reject_short_glb_header`. All three then load buffers and images with `gltf_external::load_jailed_gltf_with_root` and that handle. A `data:` URI stays in memory. A file URI is read from that handle, one path component at a time. Buffer and image URI bytes share the caller's cap (`MAX_GLB_CONTAINER_BYTES` on the editor and cook paths, `MAX_MODEL_BYTES` on the runtime), and more than 64 URIs is an error. `decode_gltf_image` uses `decode_limited_image`. The JSON loaders open a `ModelRoot` for that base and call `load_with_budget`. They do not read from the process current directory. They do not call `gltf::import_slice`, `gltf::import_buffers`, `gltf::import_images`, or `Glb::from_reader`. Also `AssetDatabase` + `AssetWatcher` hot-reload, `AssetCache`, deterministic GUIDs, optional `blend_import`/`blend_asset_integration`, `import_pipelines`.
- `astraweave-asset/src/gltf_external.rs` — `jail_gltf_uri`, `read_jailed_file`, `reject_external_uri_count`, `load_jailed_gltf`, and `load_jailed_gltf_with_root`: the jailed buffer and image read, with one shared byte cap and a 64-URI cap, that `mesh_gltf::load_gltf`, `aw_asset_cli`, and `aravath_runtime` call via `load_jailed_gltf_with_root`, and JSON loaders call via `load_with_budget` (behind the `gltf` feature). On the editor, cook, and runtime paths, one handle covers the container and every URI. The base-taking loaders open the base once per call for their URIs, and the caller supplies the container bytes. `read_jailed_file` makes a relative path absolute without following a link. `read_buffers` compares the bytes on hand with `byteLength` before the 4-byte pad. The URI read uses the directory handle. `safe_under` is not that read.
- `astraweave-asset/src/bounded_open.rs` — one directory handle per load (`OpenedRoot`). On the editor, cook, and runtime paths, `read_path` returns that handle for the container and every URI. A base-taking loader opens the base once for its URIs, and the caller supplies the container bytes. A hard-linked model is deliberately refused. That refusal is a breaking change. `read_path` resolves the caller-supplied model file once, then opens that parent without resolving it again. `read_unresolved` does not resolve its path. On Linux the handle is a directory file descriptor and each component is opened from `/` with `O_PATH`, `O_DIRECTORY`, and `O_NOFOLLOW`, so a load still works when an ancestor is not readable. Other Unix opens those components read-only with `O_DIRECTORY` and `O_NOFOLLOW`. Each URI is opened from that root descriptor with `rustix` `openat` and `O_NOFOLLOW`, and those descriptors are not kept. An intermediate component uses directory flags (`O_PATH` and `O_DIRECTORY` on Linux). On Windows the handle is the directory-handle chain plus the final path. Those directory handles do not share delete, so a directory the load holds cannot be renamed or removed while they are open. Each distinct intermediate directory `open_relative` opens stays on `OpenedRoot` until that root drops. A repeated open of the same directory does not add another handle. Each component of the resolved root is opened with reparse-point semantics. A symlink, a junction, and a mount point are refused. A cloud-file reparse tag is one of `IO_REPARSE_TAG_CLOUD` through `IO_REPARSE_TAG_CLOUD_F`. Delete sharing is on the first open of each URI leaf file, so that name can be replaced while the handle is held; only a cloud placeholder is opened a second time, without delete sharing, and the second open must be the same file. That component's final path must stay under the previous component. `final_path` runs for each resolved-root component, each intermediate directory, and the opened file. Each URI component is opened from the held parent handle. The held set keeps one handle per directory id and fails closed past 64 directory handles. The budget counts the resolved-root chain, so a deep root leaves fewer URI directory slots. Directory identity is the 128-bit file id and the volume serial. The file's final path must be under the root's final path, compared as exact UTF-16 components. A file with more than one link is refused, and the error names the file. File type and size come from that handle.
- `astraweave-asset/src/cell_loader.rs` (842 LoC) — World-Partition cell RON (de)serialization + per-asset byte loading.
- `astraweave-asset/src/nanite_preprocess.rs` (937 LoC) — meshlet clustering + QEM LOD hierarchy (CPU preprocess).
- `astraweave-asset-pipeline/src/{lib,texture,mesh,validator}.rs` — separate crate of offline transforms (BC7/ASTC, vertex-cache/overdraw optimization, CI validation).

**Status note:**
The wiring picture is split and must be understood before editing:
- **Wired into runtime:** `cell_loader::load_cell_from_ron` / `load_asset` (consumed by `astraweave-scene` streaming), `gltf_loader` mesh loaders (consumed by examples and the editor terrain integration), and `AssetDatabase` (consumed by `tools/aw_asset_cli` and `tools/aw_editor`).
- **Dormant / in-design-but-tested:** `nanite_preprocess` (no non-test/non-example caller; the render crate has a *separate* GPU-side Nanite path), `AssetWatcher` (zero production callers), `astraweave-render::residency::ResidencyManager` (re-exported, constructed only in render tests), and the **entire `astraweave-asset-pipeline` crate** (zero `astraweave_asset_pipeline::` references anywhere except its own doc-comments). See §5 and §6.

---

## 2. Authoritative Pipeline

The system is three loosely-coupled data paths. They share the `astraweave-asset` crate but do not feed each other at runtime.

### Path A — glTF/GLB → typed mesh/material/skeleton/animation (WIRED, on-demand)

```text
[GLB/.gltf bytes on disk]
    │
    │ fs::read(...) by caller (editor / example)
    ▼
[Stage A1: format detect + container parse]
    file: astraweave-asset/src/lib.rs  (`pub mod gltf_loader`)
    role: GLB magic check ("glTF"), split JSON + BIN chunks
    key data: gltf::Gltf document + buffer blobs
    │
    │ load_first_mesh_from_glb_bytes / load_all_meshes_merged /
    │ load_first_mesh_and_material / load_skinned_mesh_complete /
    │ load_skeleton / load_animations
    ▼
[Stage A2: accessor decode → CPU structs]
    file: astraweave-asset/src/lib.rs  (`pub mod gltf_loader`)
    role: decode positions/normals/tangents/uv/indices, PBR material factors
          + KHR extension fields, skeleton joints, animation channels
    key data: MeshData / MaterialData / ImageData / SkinnedMeshData /
              Skeleton / AnimationClip
    │
    ▼
[Consumer uploads to GPU / animation rig — outside this crate]
```

### Path B — World-Partition cell RON → entity/asset refs → asset bytes (WIRED, runtime streaming)

```text
[<x>_<y>_<z>.ron cell file]
    │
    │ cell_loader::load_cell_from_ron(path)   (async, tokio::fs)
    ▼
[Stage B1: RON deserialize]
    file: astraweave-asset/src/cell_loader.rs:161-172
    role: parse CellData { coord, entities, assets, metadata }
    key data: CellData (EntityData[], AssetRef[])
    │
    │ cell_loader::load_asset(asset_ref, assets_root)   (async)
    ▼
[Stage B2: per-asset byte load + header validation]
    file: astraweave-asset/src/cell_loader.rs:223-308
    role: read raw bytes; validate GLB/PNG/JPEG magic by extension
    key data: Vec<u8> raw asset bytes (NOT decoded into MeshData here)
    │
    ▼
[astraweave-scene WorldPartitionManager::update drives this]
    file: astraweave-scene/src/streaming.rs:100-271
```

### Path C — mesh → meshlet LOD hierarchy (DORMANT preprocess; example-only)

```text
[positions/normals/tangents/uvs/indices]
    │
    │ generate_lod_hierarchy(...) / preprocess_mesh_async(...)
    ▼
[Stage C1: LOD 0 meshlet clustering]
    file: astraweave-asset/src/nanite_preprocess.rs:309-353
    role: meshopt::clusterize::build_meshlets (spatial clustering)
    key data: Vec<Meshlet> { vertices, indices(u8), AABB, BoundingCone }
    │
    │ simplify_mesh (QEM edge collapse) per LOD level
    ▼
[Stage C2: simplified LOD levels + error metrics]
    file: astraweave-asset/src/nanite_preprocess.rs:356-441
    key data: MeshletHierarchy { meshlets, lod_ranges, positions... }
    │
    │ save_meshlet_hierarchy / load_meshlet_hierarchy (RON)
    ▼
[examples/nanite_demo only — no engine runtime consumer]
```

### Stage-by-stage detail

#### Stage A1/A2: glTF loaders (`gltf_loader` module)
**File:** `astraweave-asset/src/lib.rs` (`pub mod gltf_loader`).
**Role:** Pure-CPU glTF/GLB decode behind the default-on `gltf` feature.
**Inputs:** Raw `&[u8]` (GLB binary or `.gltf` JSON, including `data:` URI buffers). The JSON loaders also take an explicit base directory.
**Outputs:** `MeshData`, `MaterialData`, `ImageData`, `SkinnedMeshData`, `Skeleton`, and `AnimationClip` in `gltf_loader`.
**Notes:** `reject_short_glb_header` returns an error when a `glTF` header's declared length is below 12 bytes. `read_glb` uses it before `Glb::from_slice`. `read_gltf` uses it before `Gltf::from_slice`, so a GLB JSON chunk that starts with `glTF` and declares length 8 does not reach the debug subtract. `reject_short_glb_file` fills the 12-byte header until EOF (`read_header_or_eof`), then compares the declared length with the file length. A single short `read` does not skip the check. A `.gltf` that ends before 12 bytes stays `Ok`. A declaration below 12 bytes, or past the bytes on disk, is an error. `Glb::from_reader` allocates `declared - 12` before it reads the rest, so the file length is the bound. gltf 1.4.1 `Glb::from_reader` wraps `header.length - 12` as `u32` and allocates the wrapped length. `Gltf::open` and `gltf::import` call that reader. `decode_limited_image` (`image_limits.rs`) caps a decode at a 4096-pixel edge and 64 MiB once the bytes are already in memory. `read_limited_image_bytes` stops a file read at 64 MiB before that decode. A non-`data:` glTF image URI is jailed to the caller-supplied base. On the editor, cook, and runtime paths, one handle covers the container and every URI. The base-taking loaders open the base once per call for their URIs, and the caller supplies the container bytes. URI components are not followed. On Unix the file is then read from that directory handle. On Windows each URI component is opened from the held parent handle. A symlink, a junction, and a mount point are refused. A cloud-file reparse tag is allowed. The file's final path must be under the root's final path. `final_path` runs for each resolved-root component, each intermediate directory, and the opened file. The held set keeps one handle per directory id and fails closed past 64 directory handles. The budget counts the resolved-root chain, so a deep root leaves fewer URI directory slots. Directory identity is the 128-bit file id and the volume serial. A file with more than one link is refused, and the error names the file. The read is capped at 64 MiB, then decoded with `decode_limited_image`. A `data:` image URI stays in memory and uses the same decoder. JSON loaders call `load_with_budget` on an opened `ModelRoot`. An image buffer view is a slice of a buffer already charged to the shared cap. The editor path `mesh_gltf::load_gltf` reads the capped slice and jails URIs from `Gltf::from_slice`. `tools/aw_asset_cli` does the same after `read_jailed_gltf`. `aravath_runtime::import_validated_model` reads its own bytes with `read_model_bytes` (32 MiB), runs `validate_model_bytes` and `reject_short_glb_header` on them, then the same jailed buffer and image read with the model parent. `validate_model_bytes` accepts at most 64 relative sibling URIs, each a regular file of at most 32 MiB, and refuses when the sum of those lengths exceeds 32 MiB. `check_relative_uri` returns for a `data:` URI, which stays in memory and is not opened as a file, and refuses `%`, `:`, `\`, and NUL on every other URI. `confine_existing` refuses a sibling whose canonical path leaves the model directory. `gltf::import_slice` is not the loader: it has no base and rejects those URIs. `jail_gltf_uri` rejects a URI that contains `:`, `%`, `\`, or NUL, and `safe_under` rejects an absolute path or a `..` component. A `data:` URI stays in memory. `load_jailed_gltf` opens the base directory once and reads a non-`data:` URI from that handle. File type and size come from the opened handle. A non-regular file is rejected, and the read stops at the caller's cap. Buffer and image URI payloads share that cap. `reject_external_uri_count` refuses more than 64 URIs before the read. The editor and cook paths pass `MAX_GLB_CONTAINER_BYTES` (256 MiB) and call that count. The runtime passes `MAX_MODEL_BYTES` (32 MiB) and sums the sibling lengths against the same cap. This is not gltf 1.4.1 `read_to_end`, which would open a percent-decoded or `file:` path and allocate `st_size + 1` with no cap. `mesh_gltf::load_gltf` decodes glTF images with `decode_limited_image` through `gltf_external::load_jailed_gltf_with_root`. `load_rgba8` is still `image::open` (`mesh_gltf.rs:693-700`). `cell_loader::load_asset` for a texture is an uncapped `tokio::fs::read`. KTX2 `pixel_width` and `pixel_height` can size a pixel buffer with no `MAX_IMAGE_EDGE` check after the 64 MiB file stop. Those remaining reads are #408. `load_gltf_bytes` is a Phase-0 header-only validator that does not decode. Real decode entrypoints are `load_first_mesh_from_glb_bytes`, `load_all_meshes_merged`, `load_first_mesh_and_material`, and the skinned variants `load_skinned_mesh_complete` / `load_first_skinned_mesh_and_idle`. Each JSON entry point takes the base directory. Material defaults follow glTF spec defaults (e.g. `metallic_factor: 1.0`, `ior: 1.5` on `MaterialData` defaults in `gltf_loader`).

#### Stage B1: Cell RON deserialize
**File:** `astraweave-asset/src/cell_loader.rs:104-220`.
**Role:** (De)serialize `CellData` to/from RON, async (`tokio::fs`) and sync variants.
**Inputs:** RON cell file path.
**Outputs:** `CellData { coord:[i32;3], entities:Vec<EntityData>, assets:Vec<AssetRef>, metadata:Option<CellMetadata> }` (cell_loader.rs:104-115).
**Notes:** `EntityData` carries a transform (`position`/`rotation` quaternion/`scale`), an optional `mesh` path, an optional `material` layer index, and extensible `Vec<ComponentData>` (cell_loader.rs:49-66). `add_asset` deduplicates by `path` (cell_loader.rs:133-138). `cell_path_from_coord` formats `"{}_{}_{}.ron"` (cell_loader.rs:312). A texture `AssetKind` still uses an uncapped `tokio::fs::read` (#408).

#### Stage B2: Per-asset byte load + magic validation
**File:** `astraweave-asset/src/cell_loader.rs:223-308`.
**Role:** Read raw asset bytes from `safe_under(assets_root, asset_ref.path)` and run a lightweight magic-number check (GLB `b"glTF"`, PNG `\x89PNG\r\n\x1a\n`, JPEG `0xFF D8 FF`). Parent segments and absolute paths are rejected. glTF JSON loaders do not use this directory. They open a `ModelRoot` and call `load_with_budget`. `data:` URIs stay in memory.
**Inputs:** `&AssetRef`, `assets_root: &Path`.
**Outputs:** `Vec<u8>` raw bytes — NOT decoded into `MeshData`. Decode is the consumer's responsibility.
**Notes:** `cell_loader::AssetKind` (cell_loader.rs:12-21) is a *distinct enum* from the crate-root `AssetKind` (`pub enum AssetKind` in `lib.rs`). See §6 naming collision. A glTF image file URI in `decode_image_from_gltf` is jailed to the caller-supplied base. On the editor, cook, and runtime paths, one handle covers the container and every URI. The base-taking loaders open the base once per call for their URIs, and the caller supplies the container bytes. URI components are not followed. On Unix the file is then read from that directory handle. On Windows each URI component is opened from the held parent handle. A symlink, a junction, and a mount point are refused. A cloud-file reparse tag is allowed. The file's final path must be under the root's final path. `final_path` runs for each resolved-root component, each intermediate directory, and the opened file. The held set keeps one handle per directory id and fails closed past 64 directory handles. The budget counts the resolved-root chain, so a deep root leaves fewer URI directory slots. Directory identity is the 128-bit file id and the volume serial. A file with more than one link is refused, and the error names the file. The read stops at the 64 MiB image cap, then `decode_limited_image`. JSON loaders call `load_with_budget` on an opened `ModelRoot`. `data:` stays in memory. The cell texture read is still an uncapped `tokio::fs::read` (#408).

#### Stage C1/C2: Nanite meshlet preprocess
**File:** `astraweave-asset/src/nanite_preprocess.rs`.
**Role:** Convert a triangle mesh into a meshlet LOD hierarchy for virtualized-geometry rendering.
**Inputs:** position/normal/tangent/uv/index slices, `lod_count`.
**Outputs:** `MeshletHierarchy` (nanite_preprocess.rs:218-234).
**Notes:** LOD 0 uses `meshopt::clusterize::build_meshlets` with `MAX_MESHLET_VERTICES=64` / `MAX_MESHLET_TRIANGLES=124` / `cone_weight=0.5` (nanite_preprocess.rs:21-24, 330-336). Lower LODs use an in-house QEM edge-collapse simplifier (`QuadricError`, `simplify_mesh`, nanite_preprocess.rs:236-661), targeting ~50% triangle reduction per level. `preprocess_mesh_async` wraps the CPU work in `tokio::task::spawn_blocking` (nanite_preprocess.rs:671-694). **No engine-runtime consumer** — see §6.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **GUID** | 128-bit hex digest (`SHA-256` of lowercased, forward-slash-normalized path, first 16 bytes) identifying an asset. | `guid_for_path`; `AssetDatabase`, `AssetCache` |
| **`AssetKind` (root)** | Crate-root asset-category enum used by `AssetDatabase`: `Mesh, Texture, Audio, Dialogue, Material, Animation, Script, BlenderSource, Other`. `#[non_exhaustive]`. | crate-root `AssetKind` (`pub enum AssetKind`) |
| **`AssetKind` (cell)** | Cell-loader's own enum: `Mesh, Texture, Material, Audio, Animation, Other` (no Dialogue/Script/BlenderSource). `#[non_exhaustive]`. | cell_loader.rs:12-21 |
| **`AssetMetadata`** | DB record: guid, path, kind, content hash, deps, mtime, size. | `AssetMetadata` (`pub struct AssetMetadata`) |
| **`AssetRef`** | Cell-level reference to an asset (path + cell `AssetKind` + optional guid). | cell_loader.rs:23-47 |
| **`CellData`** | Serialized World-Partition cell (entities + asset refs + metadata). | cell_loader.rs:104-147 |
| **Meshlet** | Triangle cluster with local vertex list, `u8` triangle indices, `AABB`, `BoundingCone`, LOD level/error. | nanite_preprocess.rs:142-215 |
| **`MeshletHierarchy`** | All meshlets across LODs + original vertex streams + `lod_ranges`. | nanite_preprocess.rs:218-234 |
| **ACMR** | Average Cache Miss Ratio — vertex-cache efficiency metric (lower is better). | pipeline `mesh.rs:185-210`, `validator.rs:171-200` |
| **Residency** | Memory-budgeted loaded/evicted state of assets (LRU). | `astraweave-render::residency` |

### Terms to NOT confuse

- **Root `AssetKind` vs cell `AssetKind`:** Two independent enums with the same name in the same crate (`pub enum AssetKind` in `lib.rs` and `cell_loader.rs:14`). They are NOT interchangeable and have different variant sets. Code that bridges DB and cell layers must convert explicitly.
- **`MeshData` vs `Mesh`:** `gltf_loader::MeshData` is the decoded glTF result (separate position/normal/tangent/uv vectors). `astraweave-asset-pipeline::mesh::Mesh` (mesh.rs:38) is a flat `positions: Vec<f32>` + `indices` optimization input. Unrelated types in different crates.
- **`astraweave-asset` Nanite preprocess vs `astraweave-render` Nanite:** the CPU `nanite_preprocess::Meshlet`/`MeshletHierarchy` are *not* the render crate's `GpuMeshlet` (in `astraweave-render/src/nanite_visibility.rs`, behind the `nanite` feature). The render path does not consume this crate's meshlets. See §6.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source | Interface | Data | Notes |
|---|---|---|---|
| Filesystem | `std::fs` / `tokio::fs` (in-crate) | raw bytes, RON, JSON manifests | All I/O is direct; no VFS abstraction. |
| `astraweave-blend` (optional `blend` feature) | `blend_import` re-exports `BlendImporter` and the other `astraweave_blend` types. `BlendImportSystem::decompose_blend` is that system's method | `.blend` → glTF via Blender subprocess | Gated by the `blend` feature. `BlendImportSystem::initialize` leaves the importer unset when Blender is absent. Used by `tools/aw_editor`. |
| `meshopt` (vendored crate) | `clusterize::build_meshlets`, `optimize_vertex_cache`, `analyze_overdraw` | meshlet clustering + cache stats | Used by `nanite_preprocess.rs` and pipeline `mesh.rs`. |

### Downstream (what consumes this system's output)

Cargo consumers of `astraweave-asset` (excluding the crate itself, its `fuzz` member, and the root manifest), verified via `grep "astraweave-asset" */Cargo.toml`:

| Consumer | Interface used | Wired? | Notes |
|---|---|---|---|
| `astraweave-scene` | `cell_loader::{load_cell_from_ron, load_asset, CellData, EntityData, AssetRef, AssetKind, CellMetadata, ComponentData}` | **Yes** | Runtime streaming: `WorldPartitionManager::update` → `start_load_cell` → `load_cell_data`/`load_asset_data` (`streaming.rs:147-271`). |
| `astraweave-render` | `AssetDatabase`, `AssetKind`, `AssetMetadata` via `residency.rs` | **Partial** | `ResidencyManager` (render/src/residency.rs) takes `Arc<Mutex<AssetDatabase>>`; re-exported at `lib.rs:240` but constructed only in render tests/benches (no production caller found). |
| `astraweave-gameplay` | `cell_loader::ComponentData` (`use astraweave_asset::cell_loader::ComponentData` in `veilweaver_slice` tests) | **Test-only** | That `use` is inside `#[cfg(test)] mod tests`. No production-path use of `astraweave_asset` exists in `astraweave-gameplay/src`. Corrected during 1.1 verification (was "Yes / Single typed import"). |
| `astraweave-scripting` | declared dep, **no `use`** | **No** | Declared-but-unused Cargo dependency (`Cargo.toml:19`); scripting uses its own `loader::ScriptLoader`. Matches CLAUDE.md Key-Lesson-8 "declared-but-unused Cargo deps". |
| `tools/aw_asset_cli` | `AssetDatabase::{new, scan_directory, register_asset, save_manifest}` (`main.rs:166-202`) | **Yes** | CLI builds a manifest. Owns its OWN texture compression + validators (see §6). |
| `tools/aw_editor` | `AssetDatabase`, `blend_import::{BlendImporter, DecomposedAsset, DecompositionResult}` (`main.rs`), `gltf_loader::load_all_meshes_merged` (`terrain_integration.rs:545`) | **Yes** (asset) / **No** (pipeline) | Depends on `astraweave-asset` with `features=["blend"]`. The `astraweave-asset-pipeline` dependency was removed; the removal note is in `tools/aw_editor/Cargo.toml`. The editor never references that crate — see §6. |
| `examples/hello_companion` | `gltf_loader` (`visual_demo.rs:30`) | example | |
| `examples/veilweaver_demo` | `gltf_loader::load_all_meshes_merged` (`visual_renderer.rs`) | example | |
| `examples/visual_3d` | `gltf_loader as gl` | example | Passes the model file's parent into `load_first_mesh_and_material` and `load_skinned_mesh_complete`. |
| `examples/nanite_demo` | `nanite_preprocess::{generate_lod_hierarchy, MeshletHierarchy}` (`main.rs`) | example | **Only** consumer of `nanite_preprocess`. |
| `examples/skinning_demo` | declares dep; uses `astraweave_render::animation` `Skeleton` | example | Its `Skeleton` comes from the render/animation crate, not this crate's `gltf_loader::Skeleton`. |
| `aravath_runtime` | `gltf_loader::reject_short_glb_header` inside `import_validated_model` (`src/assets.rs`), then `gltf_external::load_jailed_gltf_with_root` on the directory handle from `read_model_file` and `MAX_MODEL_BYTES` | **Yes** | The game runtime's mesh import path. `read_model_bytes` calls `read_model_file`, which calls `bounded_open::read_path`. `validate_model_bytes` caps the URI count at 64 and the sum of sibling lengths at 32 MiB. A `data:` URI stays in memory and is not opened as a file. `checkpoint_replay` hashes this crate. |

`astraweave-asset-pipeline` Cargo consumers: the root manifest only. The editor dependency was removed. **No production caller of `astraweave_asset_pipeline::*` exists** (the only references in the workspace are doc-comment examples inside the crate). See §6.

### Bidirectional / Coupled

- **`AssetDatabase` ↔ hot-reload:** `AssetDatabase` owns a `tokio::sync::watch` channel (`hot_reload_tx`/`hot_reload_rx` on `AssetDatabase`); `AssetWatcher` wraps the DB in `Arc<Mutex<…>>`, runs a `notify` filesystem watcher with a debounced background thread, and calls `invalidate_asset` which fires the watch channel. `ResidencyManager::with_hot_reload` consumes a `watch::Receiver` to drain invalidations. This whole loop is exercised only by tests today.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-asset/src/lib.rs` | glTF loaders, `AssetDatabase`/`AssetWatcher`, GUID/cache, blend import, import_pipelines | Active (mixed) | glTF loaders + `AssetDatabase` are wired; `AssetWatcher` is dormant (no prod caller). JSON loaders take an explicit base directory. |
| `astraweave-asset/src/bounded_open.rs` | One directory handle per load (`OpenedRoot`, `read_path`, `read_unresolved`) | Active | On the editor, cook, and runtime paths, one handle covers the container and every URI. Base-taking loaders open the base once for their URIs. |
| `astraweave-asset/src/gltf_external.rs` | Jailed glTF buffer and image read (`jail_gltf_uri`, `read_jailed_file`, `reject_external_uri_count`, `load_jailed_gltf`) | Active | Called by `mesh_gltf::load_gltf`, `aw_asset_cli` `process_model`, and `aravath_runtime::import_validated_model` via `load_jailed_gltf_with_root`; JSON loaders call `load_with_budget` on an opened `ModelRoot`. URI payloads share one byte cap. |
| `astraweave-asset/src/cell_loader.rs` | World-Partition cell (de)serialize + per-asset byte load | Active | Wired into `astraweave-scene` streaming runtime. |
| `astraweave-asset/src/nanite_preprocess.rs` | meshlet clustering + QEM LOD hierarchy | Transitional / In-design | No non-test/non-example caller. Render Nanite is a separate path. |
| `astraweave-asset/src/mutation_tests.rs` | mutation-killing tests | Active (test-only) | `#[cfg(test)] mod mutation_tests`. |
| `astraweave-asset-pipeline/src/lib.rs` | crate root, re-exports | In-design | Zero production consumers. |
| `astraweave-asset-pipeline/src/texture.rs` | BC7 (`intel_tex`, `bc7` feature) + ASTC (basisu CLI / `basis_universal` transcode, `astc` feature) | In-design | `compress_bc7`/`compress_astc`/transcode funcs. Default feature `astc` requires `basis-universal` (declared `optional`). |
| `astraweave-asset-pipeline/src/mesh.rs` | vertex-cache + overdraw optimization (`meshopt`) | In-design | `optimize_mesh`, `Mesh::new`, ACMR. |
| `astraweave-asset-pipeline/src/validator.rs` | texture/mesh validation reports for CI | In-design | `AssetValidator`, `ValidationReport`, `BatchValidationReport`. |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit with care.
- **Transitional**: Active code but planned for change or not yet wired to its intended consumer.
- **In-design**: Implemented + tested but has no production caller (dormant per CLAUDE.md Key Lesson 8).

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition (factual) |
|---|---|---|---|
| CPU Nanite meshlet preprocess | `astraweave-asset/src/nanite_preprocess.rs` (`Meshlet`, `MeshletHierarchy`) | In-design | Example-only consumer (`nanite_demo`). |
| GPU Nanite visibility/culling | `astraweave-render/src/nanite_visibility.rs` (`GpuMeshlet`), `nanite_gpu_culling.rs`, `nanite_render.rs` (behind `nanite` feature) | Active (feature-gated) | Separate type set; does not deserialize this crate's `MeshletHierarchy`. The two Nanite halves are not connected by a shared on-disk format in current code. |
| Texture compression (offline) | `astraweave-asset-pipeline/src/texture.rs` (`compress_bc7`, `compress_astc`) | In-design (no caller) | — |
| Texture compression (CLI) | `tools/aw_asset_cli/src/main.rs:298+` (`process_texture` shells out to `toktx`/`basisu`), `tools/aw_asset_cli/src/texture_baker.rs` | Active | The CLI does NOT call `astraweave-asset-pipeline`; it implements its own compression by invoking external CLI tools. |
| Asset validation | `astraweave-asset-pipeline/src/validator.rs` (`AssetValidator`) | In-design (no caller) | — |
| Asset validation (CLI) | `tools/aw_asset_cli/src/validators.rs` (`validate_texture`, `validate_ktx2_mipmaps`, `validate_material_toml`) + `validators_original.rs` | Active | The CLI has its own validator family, parallel to the pipeline crate's. |
| Mesh format validation | `cell_loader::validate_mesh_format`/`validate_texture_format` (magic-number, cell_loader.rs:272-308); `gltf_loader::load_gltf_bytes` (header check) | Both Active | Two lightweight validators in the same crate for overlapping purposes (cell-load path vs generic glTF entry). |

### Naming collisions

- **`AssetKind`**: defined twice in `astraweave-asset` — crate root (`pub enum AssetKind`, 9 variants incl. Dialogue/Script/BlenderSource) and `cell_loader` (`cell_loader.rs:14`, 6 variants). Consumers import one or the other; `astraweave-scene` uses the cell variant. No conversion helper exists between them.
- **`Mesh` / `MeshData`**: `gltf_loader::MeshData` (decoded glTF) vs pipeline `mesh::Mesh` (flat optimization input) — unrelated.
- **`Meshlet`**: `astraweave-asset::nanite_preprocess::Meshlet` (CPU) vs `astraweave-render::nanite_visibility::GpuMeshlet` (GPU) — unrelated layouts.
- **`ResidencyManager`**: the real type lives in `astraweave-render/src/residency.rs`; an unrelated bench-local struct of the same name exists in `astraweave-render/benches/clustered_megalights_residency.rs:265` (benchmark scaffolding, not the production type).

### Known cognitive traps

- **Trap:** The `astraweave-asset-pipeline` crate looks like the production asset-processing pipeline.
  **Why it's confusing:** It has polished docs, tests, benches, and is declared as a dependency of `tools/aw_editor`.
  **What's actually true:** No code outside the crate references `astraweave_asset_pipeline::*` (verified by `grep` — only doc-comment occurrences). The editor's `astraweave-asset-pipeline` dependency was removed. The functioning asset-bake path is `tools/aw_asset_cli`, which reimplements compression/validation independently.

- **Trap:** `nanite_preprocess` appears to feed the render crate's Nanite pipeline.
  **Why it's confusing:** Both are called "Nanite" and both define meshlets.
  **What's actually true:** No data path connects them in current code; the only caller of `nanite_preprocess` is `examples/nanite_demo`.

- **Trap:** `gltf_loader::load_gltf_bytes` looks like the loader.
  **Why it's confusing:** Name + position.
  **What's actually true:** It only validates the GLB header / detects JSON (`load_gltf_bytes`, comment "Phase 0 scope"). The decoding entrypoints are the `load_first_mesh_*` / `load_*_skinned_*` functions.

- **Trap:** `astraweave-scripting` depends on `astraweave-asset`.
  **What's actually true:** It never `use`s it; the dependency is unused (Cargo.toml:19).

### Discarded-Result residue

- `astraweave-scene/src/streaming.rs:193` uses `let _ = Self::load_asset_data(asset_ref, assets_root).await;` — the per-asset byte-load Result is discarded in the streaming loop. This is a consumer-side pattern, surfaced here because it touches the cell-loader output. (CLAUDE.md flags `let _ =` on fallible asset I/O as a hazard; recorded as an open question in §11, not a recommendation.)
- `invalidate_asset` uses `.ok()` on the `hot_reload_tx` watch send; `AssetWatcher` uses `let _ =` on the watcher mpsc send — intentional fire-and-forget on channels with possibly-dropped receivers.

---

## 7. Decision Log

### Decision: Deterministic path-based GUIDs (SHA-256, 128-bit)
- **Date:** [Reasoning not recovered from available sources] (code comment "Phase 2 foundations" above `guid_for_path`)
- **Status:** Accepted.
- **Context:** Assets need a stable identity across runs for caching/dependency tracking.
- **Decision:** `guid_for_path` hashes the lowercased, `\`→`/`-normalized path with SHA-256 and keeps the first 16 bytes as hex (`guid_for_path`).
- **Consequences:** GUIDs are case-insensitive and separator-insensitive (verified by test `guid_is_deterministic_and_case_insensitive`). Renaming/moving a file changes its GUID. Content is not part of the GUID (content hash is tracked separately in `AssetMetadata.hash`).

### Decision: Cells serialized as RON, loaded async via `tokio::fs`
- **Date:** [Reasoning not recovered from available sources] (module docstring, cell_loader.rs:1-4)
- **Status:** Accepted.
- **Context:** World-Partition streaming needs human-readable, diffable cell files loadable off the main thread.
- **Decision:** `CellData` is `serde`-derived and (de)serialized with `ron`; async (`load_cell_from_ron`) and sync variants both exist (cell_loader.rs:161-220).
- **Consequences:** Matches the documented `load_cell_from_ron` async pattern in CLAUDE.md. Per-asset byte loading is decoupled from cell parsing (`load_asset` returns raw `Vec<u8>`, leaving decode to the consumer).

### Decision: Meshlet clustering delegated to `meshopt`; LOD simplification in-house QEM
- **Date:** [Reasoning not recovered from available sources] (code comments, nanite_preprocess.rs:335, 476)
- **Status:** Accepted (preprocess), but unwired.
- **Context:** Virtualized geometry needs meshlets + LOD chain.
- **Decision:** LOD-0 clustering uses `meshopt::clusterize::build_meshlets`; lower LODs use a hand-written quadric-error-metric edge-collapse simplifier (`QuadricError`, `simplify_mesh`).
- **Alternatives considered:** [Reasoning not recovered from available sources].
- **Consequences:** The simplifier uses a midpoint heuristic for the collapsed-vertex position rather than a full QEF solve (explicit comment, nanite_preprocess.rs:540-541).

### Decision: Asset transforms split into a separate `astraweave-asset-pipeline` crate
- **Date:** [Reasoning not recovered from available sources] (crate description: "Asset processing pipeline … texture compression, mesh optimization")
- **Status:** Accepted as a crate; not adopted by any consumer.
- **Context:** Offline asset processing (compression, optimization, CI validation).
- **Decision:** A standalone crate with feature-gated BC7 (`intel_tex`) / ASTC (`basis-universal`) compression, `meshopt` mesh optimization, and a CI validator.
- **Consequences:** Coexists with the independently-implemented compression/validation inside `tools/aw_asset_cli`. The two were not unified.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `guid_for_path` is deterministic and case/separator-insensitive. | Yes | Test `guid_is_deterministic_and_case_insensitive`. |
| 2 | Meshlet generation requires `indices.len() % 3 == 0`; empty input yields empty output. | Yes | `generate_meshlets` guard (nanite_preprocess.rs:316-322) + tests. |
| 3 | `Mesh::new` rejects positions not multiple of 3, indices not multiple of 3, and out-of-bounds indices. | Yes | pipeline `mesh.rs:47-73` + `test_mesh_validation`. |
| 4 | BC7 compression requires width and height divisible by 4. | Yes | `compress_bc7` guard (texture.rs:70-76) + `test_bc7_requires_multiple_of_4`. |
| 5 | `ValidationReport.add_error` sets `passed = false`; warnings do not. | Yes | `validator.rs:46-54` + `test_validation_report`. |
| 6 | `CellData::add_asset` does not insert a duplicate `path`. | Yes | `cell_loader.rs:133-138` + `test_cell_add_asset_no_duplicates`. |
| 7 | Cell `AssetKind` and root `AssetKind` are distinct types and must be converted explicitly when bridged. | No | doc-only (this trace). |
| 8 | `astraweave-asset` forbids `unsafe` except on Windows, where it denies it. The allowed calls are `bounded_open::final_path`, `bounded_open::windows_by_handle`, `bounded_open::windows_reparse_tag`, `bounded_open::windows_file_id`, and `bounded_open::open_at`. Final-path components compare as exact UTF-16 units. `final_path` runs for each resolved-root component, each intermediate directory, and the opened file. Directory identity is the 128-bit file id and the volume serial. `astraweave-asset-pipeline` forbids `unsafe`. | Yes | `lib.rs:1-2` and pipeline `lib.rs:1`. |

---

## 9. Performance & Resource Profile

### Hot paths
- **Cell streaming load** (`cell_loader::load_cell_from_ron` + `load_asset`): runs from `WorldPartitionManager::update` as the camera moves. RON parse + raw byte read per cell; budget governed by `StreamingConfig` in `astraweave-scene`. Async via `tokio::fs` to stay off the simulation thread.

### Cold paths
- **glTF decode** (`gltf_loader::load_*`): on-demand at load/import time, not per-frame. Allocates full vertex/material/animation buffers.
- **Meshlet preprocess** (`nanite_preprocess`): offline/preprocess; `preprocess_mesh_async` offloads to `tokio::task::spawn_blocking` (nanite_preprocess.rs:687). QEM simplifier builds per-vertex quadrics + an edge `BinaryHeap` — O(E log E)-ish, intended for bake time, not runtime.
- **Pipeline transforms** (BC7/ASTC/optimize): offline bake only (would be, if wired).

### Resource ownership
- `AssetDatabase`: owns all `AssetMetadata`, path→GUID map, and forward/reverse dependency graphs (fields on `AssetDatabase`). Typically shared as `Arc<Mutex<AssetDatabase>>` by `AssetWatcher` and `ResidencyManager`.
- `MeshletHierarchy` / `MeshData`: owned by the caller after load; this crate retains nothing.

---

## 10. Testing & Validation

- **Unit tests:** Extensive in-module `#[cfg(test)]` blocks: `lib.rs` (glTF/GUID/DB/watcher), `cell_loader.rs:316-842` (RON round-trip, magic validation, async load), `nanite_preprocess.rs:713-937` (AABB, meshlet gen, LOD hierarchy, QEM), pipeline `mesh.rs`/`texture.rs`/`validator.rs`.
- **Mutation testing:** `astraweave-asset/src/mutation_tests.rs` (~33 KB) — explicitly boundary/operator/constant mutation-killers. `astraweave-asset` is in the `sanitizers.yml` P1 array (per ARCHITECTURE_MAP.md:831).
- **Fuzzing:** `astraweave-asset/fuzz/` member present (declares the crate as a dep).
- **Benchmarks:** `astraweave-asset-pipeline` declares `benches/pipeline_adversarial` (Cargo.toml). (Render-side Nanite/residency benches live in `astraweave-render/benches/`, using their own types.)
- **Manual validation:** glTF loaders are exercised by `examples/visual_3d`, `hello_companion`, `veilweaver_demo`; meshlet preprocess by `examples/nanite_demo`.

---

## 11. Open Questions / Parked Decisions

- **[RESOLVED 2026-06-30, R-series M2/E4]** ~~Is `astraweave-asset-pipeline` intended to replace `tools/aw_asset_cli`'s in-house compression/validation, or are both intentional?~~ **Neither is wired.** The E4 recon found the cook capability fragmented across **three** caller-less/standalone implementations — real `intel_tex` BC7 in `astraweave-asset-pipeline::texture` (island, no container), the placeholder BC encoder + KTX2 writer in `aw_asset_cli/texture_baker.rs` (DFD sRGB bug at `:393`), and the external `toktx`/`basisu` path in `aw_asset_cli/main.rs` (never invoked live). Unifying them (the CLAUDE.md "never build a second implementation" resolution) is scope for the **deferred post-v1.0 cook build**, not v1.0. See `docs/audits/e4_cook_path_recon_2026-06.md`.
- **[RESOLVED 2026-06-30, R-series M2/E4]** ~~Why does `tools/aw_editor` depend on `astraweave-asset-pipeline` with no `use`?~~ **Stale / planned-but-unwired dep — now dropped** (relabel-and-defer beat). The editor authors via the `astraweave-asset` dependency in `tools/aw_editor/Cargo.toml` (load-bearing: `AssetDatabase`, `gltf_loader`, `blend_import`); it never referenced the pipeline crate.
- **Why does `astraweave-scripting` depend on `astraweave-asset` with no `use` (`Cargo.toml:19`)?** Declared-but-unused; candidate for the CLAUDE.md Key-Lesson-8 inventory.
- **Will the CPU `nanite_preprocess` output ever feed the render crate's GPU Nanite path?** No shared on-disk format connects `MeshletHierarchy` (RON) to `GpuMeshlet` today; the bridge, if intended, is not implemented.
- **Should `astraweave-scene/src/streaming.rs:193` discard the asset-load Result (`let _ = …`)?** CLAUDE.md flags this pattern on fallible asset I/O. Surfaced as a question for the scene/streaming owner.
- **Should the two `AssetKind` enums be unified or given an explicit conversion?** They diverge in variant sets and have no bridge function.
- **Pipeline default feature mismatch:** `astraweave-asset-pipeline` sets `default = ["astc"]` and `astc = ["basis-universal"]`, but `basis-universal` is declared `optional = true`. [NEEDS VERIFICATION] whether `default` builds pull it in correctly in all configurations.
  - *1.1 verification note (not a resolution — this is a parked decision):* The relevant lines are `Cargo.toml:45` (`default = ["astc"]`), `:47` (`astc = ["basis-universal"]`), `:18` (`basis-universal = { version = "0.3", optional = true }`). Because the `astc` feature lists the bare optional-dependency name `basis-universal` (the implicit-feature form, not `dep:basis-universal`), enabling `astc` activates the optional dependency, so a plain `cargo build -p astraweave-asset-pipeline` (default features) does pull in `basis-universal`. The marker is retained because no build matrix / CI run was executed to confirm behavior across all feature-flag combinations (e.g. `--no-default-features --features bc7`), per the read-only constraint of this pass.

---

## 12. Maintenance Notes

**Update this doc when:**
- A consumer wires (or unwires) `nanite_preprocess`, `AssetWatcher`, `ResidencyManager`, or `astraweave-asset-pipeline` — the wired/dormant status in §1, §4, §5 changes.
- The glTF loader public surface (`gltf_loader`) gains/loses entrypoints or material fields.
- `cell_loader` `CellData`/`AssetRef`/`AssetKind` shape changes (ripples into `astraweave-scene`).
- The two `AssetKind` enums are unified or bridged (§6, §8 #7).

**Verification process:**
- Re-run consumer grep: `grep -rl "astraweave-asset" */Cargo.toml` and `grep -rn "astraweave_asset::\|astraweave_asset_pipeline::" --include=*.rs .` to refresh §4 wired/dormant claims.
- Confirm `nanite_preprocess` / pipeline production callers with `rg '<fn>\(' --type rust -g '!*test*' -g '!*example*'`.
- Stamp the new commit hash and date in the Metadata table.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. `astraweave-asset-pipeline` has NO production caller — do not assume editing it changes engine behavior. The live bake path is `tools/aw_asset_cli` (which reimplements compression + validation).
2. `nanite_preprocess` is example-only and disconnected from the render crate's GPU Nanite (`GpuMeshlet`). Two different meshlet worlds.
3. There are TWO `AssetKind` enums in `astraweave-asset` (root vs `cell_loader`). Pick the right one; there's no conversion helper.
4. The wired, load-bearing surfaces are: `gltf_loader::load_*`, `cell_loader::{load_cell_from_ron, load_asset}`, and `AssetDatabase`.

**Files you'll most likely touch:**
- `astraweave-asset/src/lib.rs` (glTF loaders, `AssetDatabase`)
- `astraweave-asset/src/cell_loader.rs` (cell streaming — ripples into `astraweave-scene`)

**Files you should NOT touch without strong reason:**
- `astraweave-asset/src/nanite_preprocess.rs` — in-design, example-only; changes won't affect runtime rendering.
- `astraweave-asset-pipeline/src/*` — dormant crate; confirm a consumer before investing.

**Common mistakes when changing this system:**
- Editing `astraweave-asset-pipeline` expecting the editor/CLI to pick it up — they don't.
- Decoding a cell asset inside `cell_loader::load_asset` — by design it returns raw bytes; decode belongs to the consumer.
- Confusing `gltf_loader::load_gltf_bytes` (header validator) with a real loader.

---

## Appendix B: Historical context

The crate carries explicit "Phase 0 / Phase 2 foundations" markers on `load_gltf_bytes`, `AssetManifest::validate`, and the `guid_for_path` block, indicating an incremental build-out where header-only validators and GUID/cache scaffolding landed before full decoders. The glTF decoders, cell loader, and `AssetDatabase` matured into wired runtime use; `nanite_preprocess` and `astraweave-asset-pipeline` were built to completeness and tested but never adopted by a runtime consumer, while `tools/aw_asset_cli` grew its own independent compression/validation. The `render → aw_asset_cli` Cargo dependency (`aw_asset_cli` in `astraweave-render/Cargo.toml`, the line after `astraweave-asset`) is an unusual production→tool direction flagged in ARCHITECTURE_MAP.md (anomaly #2; that map cites `aw_asset_cli` at `astraweave-render/Cargo.toml:64`) and is orthogonal to this crate, but it is the reason the CLI's compression path — rather than `astraweave-asset-pipeline` — is reachable from the render-side asset story.

---

## Revision History

| Version | Date | Change |
|---|---|---|
| 1.27 | 2026-11-11 | **The Windows root directory handle is its own field.** Every URI starts from that handle. The 64-handle budget counts the resolved-root chain, so a deep root leaves fewer URI directory slots. `last_verified_commit` names the parent `f59241e3`. That parent kept the root handle in the held list, so a later URI started from the last held directory. |
| 1.0 | 2026-06-24 | First trace for the Asset System (forensic data-flow analysis). |
| 1.1 | 2026-06-25 | Verification pass; §11 pipeline-default-feature note; R-series M1 trace-honesty FALSE-PRODUCTION-READY annotation for `astraweave-asset-pipeline`. |
| 1.2 | 2026-06-30 | **R-series M2/E4 — relabel-and-defer.** Recorded E4 resolution: `astraweave-asset-pipeline` `lib.rs:4` over-claim relabeled to honest in-design status; the unused editor dependency on `astraweave-asset-pipeline` was dropped; verdict **FALSE-PRODUCTION-READY → DORMANT-HONEST**; BC7/KTX2 cook build **deferred post-v1.0** (the v1.0 render path consumes raw PNG→RGBA8, no GPU-compressed-upload path — cooking is a non-blocking optimization). Closed §11 Q1 (asset-pipeline-vs-CLI duplication — found 3 fragmented cook impls) + Q2 (unused editor dep — dropped). Updated Metadata Status. Source: `docs/audits/e4_cook_path_recon_2026-06.md`. |
| 1.9 | 2026-10-28 | **JSON-chunk and reader-length guards.** `read_gltf` refuses a `glTF` header whose declared length is below 12 bytes before `Gltf::from_slice`. A GLB JSON chunk that starts with `glTF` and declares length 8 returns that error instead of reaching gltf 1.4.1's debug subtract. The rebased parent of that guard is `7ea1d200`, which has no `reject_short_glb`. `a802aca2` remains an ancestor and is not that parent. |
| 1.10 | 2026-10-28 | **Declared length is bounded by the bytes on hand.** `reject_short_glb_file` returns an error when the declared length is below 12 or greater than the file length. The rebased parent is `b0f3d5d23`. `a9e434fc` is not an ancestor after the rebase. |
| 1.11 | 2026-10-28 | **Header read fills 12 bytes or stops at EOF.** `reject_short_glb_file` no longer trusts one `Read::read`. The rebased parent is `ac5a17dcf`. `40374e98` is not an ancestor after the rebase. |
| 1.12 | 2026-10-29 | **§4 lists `aravath_runtime`.** The downstream table in §4 names the runtime mesh import. The rebased parent is `2e2012eaa`, which already has the `aravath_runtime` Cargo dependency (added in `ac5a17dcf`) and omits the row. `cd910720` is not an ancestor after the rebase. |
| 1.13 | 2026-11-06 | **One capped slice.** `read_gltf_container` stops at 256 MiB and `reject_glb_reader` runs on those bytes. Callers parse with `from_slice` / `import_slice`. `last_verified_commit` names the parent `6eb6bfee`. That parent still calls `Gltf::open` and `gltf::import` after `reject_short_glb_file`. |
| 1.14 | 2026-11-06 | **Sibling URIs load from the model directory.** `Gltf::from_slice` still parses the capped bytes. Buffers and images use `import_buffers` / `import_images` with that directory, including a `data:` image on the editor and cook paths. `read_gltf_container` stats the path and requires `is_file()` before `File::open`. `last_verified_commit` names the parent `0ae2a37b`. That parent calls `gltf::import_slice` and returns `external reference in slice only import` after the jail accepts the URI, and it opens a FIFO before the regular-file check. |
| 1.15 | 2026-11-09 | **External URIs are jailed, then read with a cap.** `jail_gltf_uri` rejects `:`, `%`, `\`, and NUL, then `safe_under` rejects an absolute path or a `..` component. `read_jailed_file` checks `is_file()` before `File::open` and stops at the caller's cap. `data:` stays in memory. `last_verified_commit` names the parent `939325b5`. That parent did not jail absolute or percent-encoded traversal URIs before `safe_under`, and `import_buffers` / `import_images` still called gltf `read_to_end` with `st_size + 1` allocation. |
| 1.16 | 2026-11-10 | **Directory-handle reads, one external budget, and an explicit JSON base.** `read_gltf_container` and the external URI read, on Unix, open the directory, then the file from that handle. On Windows each component is reopened by path from the root and any reparse point is rejected. The root and directory handles stay open until the file is opened. The file's final path must be under the root's final path. Type and size come from the opened handle. Buffer and image URI bytes share the caller's cap. More than 64 URIs is an error. `decode_gltf_image` uses `decode_limited_image`. The JSON loaders take a base directory and call `load_jailed_gltf`. `validate_model_bytes` sums sibling lengths against `MAX_MODEL_BYTES`. `last_verified_commit` names the parent `93e3a156`. That parent checks `metadata().is_file()` before `File::open`, caps each URI alone, and reads JSON buffer URIs from the process current directory. |
| 1.17 | 2026-11-10 | **The model directory is resolved once.** On Unix the resolved directory is opened without `O_NOFOLLOW`, and each URI component still uses `O_NOFOLLOW`. On Windows the resolved root is opened without reparse-point semantics. Each URI component is reopened by path from that root and any reparse point is rejected. The root and directory handles stay open until the file is opened. The file's final path and the root's both come from the opened handles and are compared per component. `last_verified_commit` names the parent `9afd7dee6`. That parent opens the caller-supplied directory with no-follow or reparse-point semantics, so a directory that is itself a link fails the regular-file check. |
| 1.18 | 2026-11-10 | **Exact final paths, a pre-pad buffer check, and one resolve of the model file.** `read_buffers` compares the bytes on hand with `byteLength` before the 4-byte pad. Windows final-path components compare as exact UTF-16 units. `read_path` resolves the caller-supplied model file once before the leaf open, so a symlink at that file loads. URI components stay unfollowed. `last_verified_commit` names the parent `6d46355d`. That parent pads a buffer before the length check, compares final paths through a lossy string and ignore-case, and refuses a model file that is itself a symlink. |
| 1.19 | 2026-11-10 | **Resolved components stay unfollowed, and a file with more than one link is refused.** On Unix each component of the resolved absolute path is opened from `/` with `O_DIRECTORY` and `O_NOFOLLOW`, then each URI component is opened from that directory handle. On Windows each component of that path is opened with reparse-point semantics, a reparse point is rejected, and that component's final path must stay under the previous component. `last_verified_commit` names the parent `c5881afe4`. That parent opens the resolved directory from the process directory in one call, and it accepts a file with more than one link. |
| 1.20 | 2026-11-10 | **The model directory is opened once per load.** That handle is used for every URI read. `read_path` resolves the model file once and opens its parent without resolving the parent again. On Windows a cloud-file reparse tag is allowed. A symlink, a junction, and a mount point are refused. `final_path` runs for each component of the resolved root and for the opened file, not for an intermediate URI directory. A file with more than one link is refused, and the error names the file. A hard-linked model is deliberately refused. That refusal is a breaking change. `last_verified_commit` names the parent `39243c23`. That parent resolves the directory again for each read and refuses every reparse point. |
| 1.21 | 2026-11-10 | **One directory handle covers the container, every sibling, and every image.** One load opens the model directory once, and that handle is used for the container read, every sibling URI, and every image URI. Buffer and image URI bytes share one total, and each image file read is also limited to 64 MiB. An empty model base is the current directory. `read_unresolved` makes a relative path absolute without following a link. On Windows the second open of a cloud file must be the same file. A cloud reparse tag is one of `IO_REPARSE_TAG_CLOUD` through `IO_REPARSE_TAG_CLOUD_F`. `read_model_bytes` calls `bounded_open::read_path`. `last_verified_commit` names the parent `346e2b763`. That parent opens the directory again for each image and for the siblings after the container read. |
| 1.22 | 2026-11-10 | **One directory handle for a skinned load, the bytes already read, and directory components that do not need a readable ancestor.** A skinned load shares one directory handle and one byte budget. The cook path writes the container bytes it already read. On Linux each directory component is opened with `O_PATH`, `O_DIRECTORY`, and `O_NOFOLLOW`. `last_verified_commit` names the parent `7425e8b3`. That parent opens each directory component read-only, and the cook path copies the model file by path after the jailed read. |
| 1.23 | 2026-11-10 | **Held Windows directory handles do not share delete.** A directory the load holds cannot be renamed or removed while those handles are open. Delete sharing is on the first open of each URI leaf file, so that name can be replaced while the handle is held; only a cloud placeholder is opened a second time, without delete sharing, and that second open must be the same file. `last_verified_commit` names the parent `ac879829`. That parent shares delete on every Windows open, including each held directory handle. |
| 1.24 | 2026-11-11 | **Intermediate directory handles stay for the root lifetime.** Each directory handle `open_relative` opens stays on `OpenedRoot` until that root drops. `last_verified_commit` names the parent `bc4a5791`. That parent drops those handles when `open_relative` returns. |
| 1.25 | 2026-11-11 | **Intermediate directory handles are Windows-only.** Unix opens each URI from the root descriptor, one component at a time, with `O_NOFOLLOW`, and does not keep those descriptors. On Windows each distinct intermediate directory stays on `OpenedRoot` until that root drops. A repeated open of the same directory does not add another handle. `last_verified_commit` names the parent `a6706628`. That parent also keeps the URI descriptors on Unix. |
| 1.26 | 2026-11-11 | **URI components open from the held parent, and the held set is capped.** On Windows each URI component is opened from the held parent handle, and that directory's final path must stay under the root. The held set keeps one handle per directory id and fails closed past 64 directory handles. Directory identity is the 128-bit file id and the volume serial. Tests pin that on Linux an intermediate component uses directory flags, so an execute-only directory still loads and a non-directory is refused before a read-only open. Other Unix opens that intermediate read-only. `last_verified_commit` names the parent `0f5fbee7`. That parent reopens each Windows intermediate from the root's final path and does not cap the held set. |
