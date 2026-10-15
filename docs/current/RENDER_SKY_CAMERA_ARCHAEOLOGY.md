# Render, Sky, and Camera Archaeology

> **Workstream:** WS-RENDER-ENV-A, Phase 1 (exploratory). No feature code.
> **Evidence checkout:** `main` @ `a3fe3ecd` (2026-08-05). Aravath demo: PR #205, `cursor/aravath-greybox-b3de` @ `9e548c18`.
> **Status:** STOP for review. Do not implement the plan below until it is approved.
> **Traces read first:** `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` (v1.19), `docs/architecture/camera.md` (v1.1), `docs/current/PROJECT_STATUS.md`.

This pass maps what is already real, what only compiles, and what the Aravath demo actually calls. Line numbers for engine files are from `a3fe3ecd` unless the sentence names the PR #205 branch.

## Verdict

The engine has one production renderer (`astraweave-render::Renderer`) with two draw entries, `render` (`renderer.rs:5348`) and `draw_into` (`renderer.rs:5899`). `render_with` (`renderer.rs:7045`) is `draw_into` plus a caller overlay, then submit. The editor and the Aravath demo both land on `draw_into`.

The visible sky that actually draws is `SkyRenderer` in `environment.rs`: procedural gradient, then a baked cubemap, then an equirectangular HDRI when an IBL bake produced one. Image-based lighting is real on that same path (irradiance, specular prefilter, BRDF LUT, group 5 on static meshes, group 3 on terrain forward). Until something calls `bake_environment`, the GPU holds a 1×1 tinted cubemap, not a loaded HDRI (`renderer.rs:2142-2177`).

Bruneton atmosphere, froxel fog, screen-space light shafts, full volumetric cloud raymarch, the standalone BRDF-LUT pass, `weather_system`, `weather_gpu`, and the frame-graph sky/fog nodes are compiled and mostly tested for construction only. On `main` they have no production caller. PR #205 is the exception: it wires fog and god rays into both draw entries through `composite_atmosphere`, and it is already fixing the three Vulkan validation errors that black-framed the demo. This workstream must not edit those three sites while #205 is open.

The camera contract from the Unified Camera campaign still holds on `main`: `FreeFly` and editor `OrbitCamera` implement `CameraProducer`; `Renderer::update_view` is the only upload. PR #205 adds a third producer, `ThirdPersonCamera`, and the demo uses it. There is no fourth camera to build.

Rendering does not feed the replay hash today. Two hazards are real if a later phase gets careless: `TimeOfDay` advances from `Instant::now()` and ignores the `dt` the caller passes, and PR #205 stores a GPU timestamp sample on the fog pass (`volumetric_gpu_sample`) that nothing in the sim reads yet.

## Status table

Tags: **Real** = called from `render` / `draw_into` or the editor adapter on `main`. **Demo** = called only once PR #205's renderer is in the process. **Dormant** = compiled, tests may pass, zero non-test callers. **Stub** = declared pass with an empty body. **Duplicate** = a second implementation of a role that already has an owner.

| Surface | Path | LoC | Tag on `main` | What it actually does | Tests |
|---|---|---:|---|---|---|
| `Renderer` | `astraweave-render/src/renderer.rs` | ~9k | Real | Device, sky, IBL, CSM, forward PBR, terrain forward (`draw_into` only), water, tonemap, present | Parity harness, shader validation, many `wave2_*` suites |
| `SkyRenderer` | `environment.rs` | 1524 | Real | Inverted-sphere sky. Priority: equirect, else env cube, else procedural (`environment.rs:602-721`). Drawn from `render` (`renderer.rs:5645`) and `draw_into` (`renderer.rs:6311`) | 5 in-module CPU tests |
| `TimeOfDay` | `environment.rs:15-143` | — | Real, wall-clock | Sun/moon/ambient from `current_time`. `update` uses `Instant`, not `dt` (`environment.rs:49-53`) | Covered by environment golden tests |
| `IblManager` | `ibl.rs` | 1649 | Real API, opt-in bake | Equirect → cube, irradiance, specular mips, BRDF LUT. Editor bakes at adapter init. Demo does not bake | 8 in-module; GPU `tests/ibl_integration.rs`, `tests/ibl_irradiance_faces.rs` |
| `HdriCatalog` | `hdri_catalog.rs` | 361 | Real when a biome transition bakes | TOML → path. Does not check the file exists (`resolve_sky_mode`, `biome_material.rs:133-138`) | 6 in-module |
| `SceneEnvironment` | `scene_environment.rs` | 1205 | Real | Distance fog, ambient, sun, exposure. Not froxel fog | 49 in-module CPU layout tests |
| `CloudShadowPass` | `volumetric_clouds.rs:600+` | part of 1030 | Real, default on | 512² `Rgba16Float` **write-only** storage map. Constructed in every `Renderer::new` (`renderer.rs:2121`). Submitted when `cloud_shadows_enabled` (default `true`, `renderer.rs:3503`) | 10 in-module, construction |
| `VolumetricCloudsPass` | `volumetric_clouds.rs` | same | Dormant | Raymarch + composite. Composite samples scene depth as filterable `texture_2d<f32>` | Same module tests |
| `VolumetricFogPass` | `volumetric_fog.rs` | 1070 | Dormant on `main`; **Demo** on #205 | Froxel density/scatter/integrate/apply. Integrate BGL binding 2 is filterable float (`volumetric_fog.rs:344-352`) while the bound texture is `Depth32Float` | 10 in-module. #205 adds `tests/volumetric_validation.rs` |
| `GodRayPass` | `god_rays.rs` | 467 | Dormant on `main`; **Demo** on #205 | On `main`: depth is `TextureSampleType::Depth`, output is write-only `Rgba16Float` (`god_rays.rs:165-204`). #205 adds a composite pass whose read-write storage was the Vulkan failure | 6 in-module |
| `AtmospherePass` | `atmosphere.rs` | 882 | Dormant | Bruneton transmittance LUT + sky + aerial perspective. #205 only fixes `AtmosphereParams` WGSL padding; still no `renderer.rs` caller | 9 in-module |
| `BrdfLutPass` | `brdf_lut.rs` | 284 | Dormant duplicate | Second BRDF LUT baker. Production LUT comes from `IblManager::bake_environment` | 5 in-module GPU init |
| `weather_system.rs` | — | 1083 | Dormant | Biome weather transitions. Not used by `Renderer` | 40 in-module |
| `weather_gpu.rs` | — | 416 | Dormant | Maps weather to `gpu_particles` emitters. Zero callers outside the file | 16 in-module |
| `effects::WeatherFx` | `effects.rs` | — | Real | Particle weather the renderer actually ticks | wave2 weather tests |
| `frame_graph` sky/fog nodes | `frame_graph.rs` | — | Stub | `SkyPassNode` returns `Ok(())` without drawing. `VolumetricFogNode` validates views and does not run the pass. `build_default_graph` is test-only | graph smoke |
| `shadow_csm.rs` + `shadow_quality.rs` + `shadow_sampling.wgsl` | — | — | Dormant duplicate | Parallel CSM. Live shadows are the 3-cascade path inside `renderer.rs` (trace v1.17–v1.19) | shadow integration tests hit the dormant types |
| `gpu_profiler.rs` | — | 245 | Real when timestamps exist | `end_frame` copies into `readback_buf` unconditionally (`gpu_profiler.rs:162-170`). `request_readback` maps it after submit. Next frame's copy hits a mapped buffer. #205 owns the fix | 0 `#[test]` on `main` |
| `astraweave-camera` | `freefly.rs`, `projection.rs`, `render_view.rs`, `producer.rs` | 771 in `freefly.rs` | Real | `FreeFly` + `CameraController`. No wgpu. +X at yaw 0 | 15 lib tests in `freefly.rs`, 10 in `tests/canonical_types.rs` |
| `OrbitCamera` | `tools/aw_editor/src/viewport/camera.rs` | — | Real, editor only | Second `CameraProducer`. Upload is world-relative `to_render_view` (`engine_adapter.rs`) | `orbit_camera_producer.rs`, picking tests |
| `ThirdPersonCamera` | `astraweave-camera/src/third_person.rs` on #205 only | 137 | **Demo** | Third `CameraProducer`. Not on `main` | tests inside that file on #205 |
| `CameraKey` | `astraweave-cinematics` | — | Real bridge, rare caller | `tick_cinematics` → private `apply_camera_key` → `FreeFly`. Production caller: `examples/cutscene_render_demo` | parity fixtures, GPU-gated |
| Editor HDRI bake | `tools/aw_editor/src/viewport/engine_adapter.rs:809-862`, `load_hdri` at `3934` | — | Real | Bakes tracked `kloppenheim_02_puresky_2k.hdr` at init | `l2_proof.rs` |
| `fluids_demo` skybox | `examples/fluids_demo/src/skybox_renderer.rs` | ~356 | Duplicate sandbox | Own equirect sphere, `Rgba32Float`, not `SkyRenderer` | example-only |
| `shadow_csm_demo` camera | `examples/shadow_csm_demo/src/main.rs` | — | Sandbox | Local `Camera`, no `update_view` | example-only |
| `astraweave-materials` | `astraweave-materials/src/lib.rs` | — | Not the GPU sky | Declarative material-graph crate. GPU PBR lives in `astraweave-render` | materials spec tests |
| glTF loader | `mesh_gltf.rs` (feature `gltf-assets`, not default) | — | Partial | Positions, normals, UVs, **base-color texture only** (`mesh_gltf.rs:70-85`). Float images become 1×1 white (`mesh_gltf.rs:142-146`) | asset tests when the feature is on |
| `ModelSurfaceMaps` | `renderer.rs:642-646` | — | Real upload API | Albedo + normal + metallic-roughness bytes. The demo does not call it | renderer upload tests |

## What runs on a frame today (`main`)

Order inside `render` / `draw_into` after the shadow cascades:

1. `SkyRenderer::render` into the HDR target, depth cleared to 1 (`environment.rs:564-598`). `render` calls it at `renderer.rs:5645`. `draw_into` calls it at `renderer.rs:6311` (the comment above that call still says "procedural", but the arguments include the equirect and the env cube).
2. `SceneEnvironment` UBO upload (distance fog and ambient).
3. `CloudShadowPass::execute` unless `cloud_shadows_enabled` is false (`renderer.rs:5668-5691`).
4. Forward PBR (static meshes, bind group 5 = IBL) and, on `draw_into` only, terrain-forward (IBL at group 3).
5. Water, optional bloom (default `bloom_enabled: false` in `hdr_pipeline.rs:113`), tonemap, present.

`Renderer::new` requests `PIPELINE_CACHE` and `TIMESTAMP_QUERY` only when the adapter already exposes them (`renderer.rs:1081-1092`). It does **not** request `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`.

Fallback IBL, used whenever nobody has baked, is a 1×1 sRGB cubemap at about (100, 115, 140) plus a brighter +Y face (`renderer.rs:2170-2176`). That is a constant ambient fill, not an environment map.

## How the Aravath demo uses this (PR #205)

Owned by the other agent. Do not reimplement these pieces.

| Demo call | Where on `9e548c18` | Engine piece |
|---|---|---|
| `Renderer::new` / `render_with` | `aravath_runtime/src/main.rs:120`, `:521` | `render_with` → `draw_into` → HUD callback → submit |
| `ThirdPersonCamera::to_render_view_at` then `update_view` | `main.rs:391`, `:425`; type in `astraweave-camera/src/third_person.rs` | New producer. Yaw 0 looks along +X, eye `distance` behind `FreeFly::dir` |
| `tick_environment(present_dt)` | `main.rs:424` | `SkyRenderer::update` **discards** `dt` (`environment.rs:524-526`) and steps `TimeOfDay` from the wall clock |
| `set_sky_config` + `scene_environment_mut` | `light_the_world`, `main.rs:977-996` | Procedural sky (top `0.20, 0.36, 0.48`, horizon sunset orange), grassland fog. **No `bake_environment` / `load_hdri`** |
| `set_cloud_shadows_enabled(false)` | `main.rs:975` | Avoids the 512² cloud-shadow map |
| `set_volumetric_fog_config` + `set_god_ray_config` | `main.rs:1002-1009` | Opt-in. `--no-volumetrics`, `--safe`, or `aravath.cfg` skips them |
| `upload_veil_scar_volume` | `main.rs:429-436` | CPU scar grid → GPU. Direction is sim → render |
| `composite_atmosphere` | `renderer.rs:5161`, called from `render` (`6080`) and `draw_into` (`6894` / `6897`) | Both entries. The demo hits the `draw_into` one |
| Meshes | `add_model` / `add_model_with_texture` | Albedo only. `props.rs` keeps the first glTF image. `figure.rs` reads `base_color_factor` and mesh normals, not MR/normal textures |
| Replay | `aravath_runtime/golden/replay.hash`, workflow `aravath-determinism.yml` | Sim hash. Camera matrices and GPU timestamps are not inputs |

`volumetric_gpu_sample` / `last_gpu_sample` on the PR branch stores fog-pass **timestamp milliseconds**. Grep shows no caller under `aravath_runtime`. It is not in the replay hash. It is still a GPU readback sitting on the renderer, which a later gameplay hook must not consume.

#205 also changes `AtmosphereParams` field padding so it matches WGSL (`atmosphere.rs` on that branch). The pass stays unwired.

## Camera

Closed campaign (C.0–C.9, 2026-06-01): one `RenderView`, one `Renderer::update_view` (`renderer.rs:4325` on `main`). `update_camera` / `update_camera_matrices` are gone. `perspective_rh_gl` is absent from production `.rs` sources.

Two producers on `main`:

- `FreeFly` (`astraweave-camera/src/freefly.rs:137`).
- `OrbitCamera` (`tools/aw_editor/src/viewport/camera.rs:929`).

`CameraController::CameraMode::Orbit` is a movement style on `FreeFly`, not the editor type. `to_render_view_camera_relative` is concrete, not on the trait, and the editor upload does not use it.

Sandboxes that are intentionally outside the contract: `shadow_csm_demo`, `fluids_demo`, `biome_showcase` (owns a `FreeFly` and never calls `update_view`). Do not migrate them in this workstream.

PR #205's `ThirdPersonCamera` is the producer the demo needs. Phase 2 should review it against `docs/current/CAMERA_CONVENTIONS.md` (+X, `fovy` radians, `perspective_rh`, `update_view` only) and update `docs/architecture/camera.md` in the same commit. It should not add a fourth type. Open questions already parked in `camera.md` §11 (orthographic, multi-view, cinematics CPU anchor, editor cinematics preview) stay parked.

## Determinism

The rule for this workstream: rendering never writes the simulation, and the replay hash stays stable.

| Path | Feeds sim or `replay.hash`? | Evidence |
|---|---|---|
| `Renderer::update_view` | No | Upload to camera UBO / CSM only |
| Net `world_hash`, core `capture_replay`, editor `play_mode` hash | No camera fields | `astraweave-net`, `astraweave-core/src/capture_replay.rs`, `tools/aw_editor/tests/play_mode.rs` |
| `WorldSnapshot` | No camera fields | `astraweave-core` schema |
| GPU profiler timestamps | Logged only | `gpu_profiler.rs`, editor adapter log |
| Editor depth pick | Editor tools, not play-mode `World` | `tools/aw_editor/src/viewport/renderer.rs` |
| Veil scar volume | Sim → GPU | #205 `upload_veil_scar_volume` |
| Fog timestamp sample | Stored, unread by sim | #205 `volumetric_fog.rs` `poll_timestamps` |
| `TimeOfDay` | Presentation only today | Wall clock. If gameplay starts reading `get_light_direction`, the hash moves |

`SkyRenderer::update(&mut self, _delta_time)` (`environment.rs:524-526`) drops the frame `dt`. `TimeOfDay::update` (`environment.rs:49-53`) does `elapsed * time_scale / 3600` from `Instant::now()`, then resets `start_time`. The demo's `tick_environment(present_dt)` does not lock the sun to the 60 Hz sim. That keeps the replay hash stable and makes the sky non-replayable. A locked sunset has to be a sim-owned hour copied into the renderer, not a clock read inside the render crate.

## wgpu validation class

PR #205's reported RTX 4060 Ti / Vulkan failures, and their owners:

| Failure | On `main` @ `a3fe3ecd` | Owner |
|---|---|---|
| God-ray composite read-write `rgba16float` storage needs `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` | Composite pass does not exist. Dormant `GodRayPass` output is **write-only** (`god_rays.rs:198-200`) | #205 adds the pass and switches it to write-only + copy |
| `vol_integrate_bg` binding 2 is filterable float, runtime view is `Depth32Float` | Still true (`volumetric_fog.rs:344-352`). Not submitted on `main` | #205, `texture_depth_2d` |
| `gpu_profiler_readback` still mapped at submit | Still true, and **live** wherever the adapter has `TIMESTAMP_QUERY` (`gpu_profiler.rs:162-170`, `request_readback` at `175-185` ignores the map `Result`) | #205 |

Same class elsewhere. These are not the three lines #205 is editing.

**Live or constructed on every device**

- `CloudShadowPass` creates a `Rgba16Float` texture with `STORAGE_BINDING` and a bind-group entry whose helper is named `bgl_storage_2d_rw` but whose access is `StorageTextureAccess::WriteOnly` (`volumetric_clouds.rs:619-693`, `885-891`). The pass is built inside `Renderer::new` and **submitted by default**. The demo turns it off. Write-only `Rgba16Float` was not the error #205 logged; read-write was. It is still the same family, and this workstream should confirm it under `WGPU_VALIDATION=1` on Vulkan before treating the default-on path as clean. Bloom uses the same write-only format and runs only when `bloom_enabled` is set (default false).

**Dormant, will fail the same way if a later phase enables them**

Depth bound as filterable `texture_2d<f32>` (or sampled with a linear sampler) instead of `texture_depth_2d` / `TextureSampleType::Depth`:

- `VolumetricCloudsPass` composite (`volumetric_clouds.rs` around the filterable binding, shader `shaders/volumetrics/cloud_composite.wgsl`).
- `ssgi.rs`, `taa.rs`, `temporal_upscale.rs`, `advanced_post.rs` depth of field, `final_gather.rs`, aerial perspective in `atmosphere.rs`.

Read-write float storage without `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` on the device:

- `nanite_gpu_culling.rs:596-607` (`R32Float`).
- `gi/voxelization_pipeline.rs:216` (`Rgba16Float`).
- `vegetation_interaction.rs:127` is `ReadWrite` on `R8Unorm` (not the float-storage feature, still a storage hazard).

Patterns that already do depth correctly and should be copied: `god_rays.rs:165-174` on `main`, `water.rs` (`texture_depth_2d` + `textureLoad`).

`fluids_demo` is the rare example that already requests `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` (`examples/fluids_demo/src/main.rs:304`). The engine renderer does not.

## HDRI, EXR, and upcoming PBR glbs

**Decode.** `astraweave-render` depends on `image` 0.25.8 behind the default `textures` feature, with default image features left on. `Cargo.lock` lists the `exr` crate under `image`, so OpenEXR is linked. Radiance `.hdr` is decoded by `image` itself.

**Upload.** `load_hdr_equirectangular` (`ibl.rs:1362-1393`) then `create_hdr2d` (`ibl.rs:1397-1423`):

- `ImageRgb32F` and `ImageRgba32F` stay float and land in `Rgba16Float`.
- Every other dynamic-image variant, including the resize branch, goes through `to_rgba8()`, which clamps radiance above 1.0.
- Resize runs when either side exceeds 8192 (`MAX_HDR_DIMENSION`, `ibl.rs:1359`). A 4k or 8k HDRI is under that cap. A 16k plate is not, and the downsample destroys the HDR range.
- `compute_hdr_avg_luminance` (`ibl.rs:1321-1330`) also averages `to_rgba8()` bytes. The shipped intensity is the fixed `1.5` from the L.2 calibration, so this function is diagnostic residue, not the live scale.

**Sky vs bake.** The visible sky prefers the unclamped equirect (`ibl.rs:1304-1311`). The bake cube clamps radiance to 2.0 so the sun disk does not firefly the irradiance and does not double-count the analytic sun. That split is deliberate. A new sky implementation should not collapse it.

**Assets on disk vs catalog.** `assets/hdri/hdri_catalog.toml` lists 11 entries. Four `.hdr` files exist:

- `polyhaven/kloppenheim_02_puresky_2k.hdr` (catalog default, grassland/river day)
- `polyhaven/kloppenheim/kloppenheim_06_puresky_2k.hdr` (beach day)
- `polyhaven/spruit_sunrise/spruit_sunrise_2k.hdr`
- `polyhaven/venice_sunset/venice_sunset_2k.hdr`

Missing: `goegap_2k.hdr`, `table_mountain_2_2k.hdr`, `rainforest_trail_2k.hdr`, `misty_pines_2k.hdr`, `qwantani_moonrise_puresky_2k.hdr`, `rogland_clear_night_2k.hdr`, and `sky_equirect.png`. No `.exr` files are in the tree. `HdriCatalog::validate_all` (`hdri_catalog.rs:231-237`) can name the gaps and is not called from `Renderer::new`. `resolve_sky_mode` returns `SkyMode::HdrPath` for a catalog hit without `exists()`. `transition_biome` (`renderer.rs:3882-3901`) will then fail inside image decode for forest, desert, mountain, swamp, and night. Grassland day resolves to a file that is present. The editor's init bake uses that present file.

**What the demo shows.** Procedural `SkyConfig` plus the 1×1 fallback cubemap. Characters and chunks are albedo textures or vertex colors. A PBR `.glb` dropped into the current demo loader will not light from an EXR, and metallic/roughness/normal maps will not upload.

**Engine hook that already exists for the glb sets.** `ModelSurfaceMaps` (`renderer.rs:642-646`) is albedo + normal + metallic-roughness. `mesh_gltf.rs` does not fill the last two, and it is feature-gated off by default. The demo's own glTF readers (`aravath_runtime` `figure.rs` / `props.rs` on #205) are a second, albedo-only parser. Phase 2 should extend one loader, not add a third.

`crates/astraweave-blend` converts `.exr` / `.hdr` to LDR PNG for the blend import pipeline. That path is not runtime IBL. Do not send showcase HDRIs through it.

## History (birth commits)

| Module | Introduced | Note |
|---|---|---|
| `environment.rs` | `3a845138` 2025-09-14 | Biome sky/weather |
| `ibl.rs` | `e778abed` 2025-09-29 | Material/texture iteration |
| `scene_environment.rs`, `hdri_catalog.rs` | `3fecad80` 2026-02-09 | Scene environment + catalog |
| `atmosphere.rs`, `god_rays.rs`, `volumetric_fog.rs` | `55735d27` 2026-04-03 | Post chain for the editor viewport; fog and shafts never hooked on `main` |
| `gpu_profiler.rs` | `07b3d213` 2026-04-10 | Editor console tracing |
| `volumetric_clouds.rs` | `96cb3538` 2026-04-10 | Shadow pass later wired; raymarch pass not |
| `astraweave-camera` `FreeFly` | `39db36f3` 2026-05-18 | Unified Camera C.3.A |
| IBL bake repairs + editor terrain IBL | `b1295b93`, `5b78fdd7`, `fcef1ba6` 2026-07-27/28 | Face-correct irradiance, un-flipped bake, intensity 1.5 |
| Live CSM (not `shadow_csm.rs`) | through `a3fe3ecd` 2026-08-05 | 4-cascade work, still inside `renderer.rs` |
| Aravath fog/shafts/profiler/third person | PR #205, 41 commits on `cursor/aravath-greybox-b3de` | Not on `main` |

## Proposed plan (not started)

Ordered so the demo gets a real sky and PBR lighting without forking #205's Vulkan fixes, without a second renderer, and without moving the replay hash.

### P0 — Leave #205's three fixes alone

Do not edit god-ray composite bindings, `vol_integrate` depth layout, or `gpu_profiler` map/copy while PR #205 is open.

Acceptance:

- This branch's diff does not touch `god_rays.rs`, `volumetric_fog.rs` integrate/composite bindings, or `gpu_profiler.rs` until #205 has merged.
- After merge, `cargo test -p astraweave-render --test volumetric_validation` and `--test wgsl_struct_layout` stay green, and the demo's Windows Vulkan smoke (`--smoke-test --volumetrics` and `--no-volumetrics`) stays green.
- `aravath_runtime/golden/replay.hash` is unchanged by anything in this workstream.

### P1 — Showcase the sky and IBL the editor already has

Call the existing bake (`IblManager::bake_environment` / the editor's tracked-HDRI init) from the Aravath lighting setup with `assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr`. Let `SkyRenderer` take its equirect branch. Leave procedural colors as the fallback when the file is absent, and surface that as a logged error, not a black frame.

Acceptance:

- A headless or windowed frame with the bake enabled samples `hdr_equirect` (equirect path taken; the 1×1 fallback cube is not the bound environment).
- Static-mesh group 5 and, if the demo ever uses terrain forward, group 3 see the baked irradiance, specular, and BRDF LUT.
- Replay hash matches the golden file. No new read of `volumetric_gpu_sample`, profiler results, or `TimeOfDay` inside `aravath_runtime` sim types.
- `WGPU_VALIDATION=1` submit does not add a validation error beyond what #205 already fixed.

### P2 — Make EXR a first-class input to that same bake

Keep `load_hdr_equirectangular` + `create_hdr2d`. Teach the resize path to stay in `Rgb32F` / `Rgba32F`. Reject or hard-error on a decode that is not float, instead of silently clamping.

Acceptance:

- A unit test builds a 2×1 EXR (or Radiance HDR) whose texel is greater than 1.0 and asserts the GPU-ready `f32` buffer is still greater than 1.0.
- The same assertion holds for an image whose long side is above 8192 (the current `to_rgba8` resize).
- No new texture pipeline. The blend-import PNG conversion is not on this path.

### P3 — One PBR upload for `.glb` material sets

Extend `mesh_gltf.rs` so metallic-roughness and normal textures reach `ModelSurfaceMaps`, and have the demo call that upload instead of growing a third parser beside `figure.rs` / `props.rs`. Float glTF images must not become 1×1 white without an error (`mesh_gltf.rs:142-146`).

Acceptance:

- One fixture glTF with base color, normal, and metallic-roughness produces three textures on the model.
- Albedo-only assets still render (missing maps keep today's defaults).
- `gltf-assets` either becomes part of the demo's feature set explicitly or the demo keeps using `gltf::import` and only the GPU upload struct is shared. Pick one in the implementation PR and test it. Do not leave both parsers half-updated.

### P4 — Validation sweep of what is actually submitted

Confirm `CloudShadowPass` under Vulkan validation. If write-only `Rgba16Float` storage fails the way read-write did, gate creation on the feature or on `cloud_shadows_enabled`, the same way #205 compiles volumetrics only when enabled. Add a layout test that fails when a shader declares `texture_2d<f32>` for a depth attachment. Do not turn on SSGI, TAA, DoF, Nanite storage, VXGI, or `VolumetricCloudsPass` in the demo.

Acceptance:

- A test or a documented Vulkan run shows the cloud-shadow pass either submits cleanly or stays uncreated when the feature is missing.
- Dormant depth-as-filterable sites listed above are named in the test (source scan or bind-group assertion), not wired into `draw_into`.

### P5 — One visible sky

`SkyRenderer` stays the sky the demo and the editor draw. `AtmospherePass` stays dormant. Do not composite Bruneton on top of the HDRI.

Acceptance:

- `rg AtmospherePass` production callers remain empty outside `atmosphere.rs` tests.
- `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` §5 gains a row that says so, in the same commit that would otherwise touch the pass. If P5 is "do nothing", the trace does not need a version bump.

### P6 — Clock ownership

If the showcase needs a stable time of day, the sim (or a demo config) owns the hour and copies it into `TimeOfDay.current_time` without calling the wall-clock `update`. If the showcase wants a moving sun, document it as presentation-only and keep it out of the hash.

Acceptance:

- A test sets `current_time`, calls the render tick with a fixed `dt` of 0, and either sees the hour unchanged (wall-clock path not used) or sees a pure function of that `dt` (if we replace `Instant`). The replay hash test does not depend on the hour.

### P7 — Adopt `ThirdPersonCamera`, do not fork it

After #205 merges, review `third_person.rs` against `CAMERA_CONVENTIONS.md`. Update `docs/architecture/camera.md` (file map, producer count, revision history) in that commit. Conventions tests gain a case: yaw 0 / pitch 0 view direction is `+X`, and `to_render_view` is what `update_view` receives.

Acceptance:

- `cargo test -p astraweave-camera --lib` and `cargo test -p astraweave-render --test camera_conventions` pass.
- No new `Renderer::update_camera`.

### P8 — Catalog matches disk

Call `validate_all` before a biome bake. A missing file is an error that names the entry, or the catalog stops advertising it. Do not point `SkyMode::HdrPath` at a path that is not on disk.

Acceptance:

- A test (or `validate_all` against `assets/hdri`) lists the seven missing names recorded above.
- `resolve_sky_mode` for forest does not return a path that fails `exists()`, unless the API is changed to `Result` and the test expects that error.

### Explicit non-goals

- Deleting dormant modules (`AtmospherePass`, `VolumetricCloudsPass`, `BrdfLutPass`, `weather_system`, `weather_gpu`, `shadow_csm.rs`, frame-graph stubs).
- Enabling FastPreview.
- A second IBL baker or a second sky sphere in `aravath_runtime`.
- Feeding any GPU readback into combat, fog gameplay, or the replay hash.
- Orthographic projection, TAA jitter fields, or multi-view producers (`camera.md` §11).

## Deferral records

Phase 1 does not change runtime behavior, so these are not fixed here. The three Vulkan defects are already in flight on PR #205; filing duplicates would split the fix. The asset and loader gaps below are the ones this workstream should turn into GitHub issues labeled `deferred-failure` once this plan is approved. They are written in issue form so the filing step is mechanical.

### DF-1 — HDRI catalog advertises files that are not on disk

**Root cause.** `assets/hdri/hdri_catalog.toml` has 11 entries. Seven paths are absent (listed in the HDRI section). `resolve_sky_mode` (`biome_material.rs:133-138`) returns `HdrPath` without checking `exists()`. `transition_biome` (`renderer.rs:3898-3901`) bakes that path and fails at image open. `HdriCatalog::validate_all` is unused by `Renderer::new`.

**Plan.** P8. Either fetch the CC0 plates or stop advertising them. Bake must not proceed with a missing path.

**Not a #205 duplicate.** The demo does not bake.

### DF-2 — HDR downsample and non-float decode clamp radiance to 8-bit

**Root cause.** `ibl.rs:1385-1390` resizes via `to_rgba8()`. `create_hdr2d` (`ibl.rs:1412-1422`) does the same for any image that is not `ImageRgb32F` or `ImageRgba32F`. A 16k EXR, or a decode that lands in 8-bit, loses every value above 1.0 before the GPU sees it.

**Plan.** P2. Float resize, and a test with a texel greater than 1.0.

### DF-3 — glTF float images and non-albedo PBR maps are dropped

**Root cause.** `mesh_gltf.rs:142-146` turns R16/R32 glTF images into 1×1 white. The loader never reads metallic-roughness or normal textures (`mesh_gltf.rs:70-85`). The demo's `props.rs` / `figure.rs` on #205 are a second albedo-only reader.

**Plan.** P3. One upload through `ModelSurfaceMaps`. Hard error on an unhandled float format instead of a white pixel.

### DF-4 — `CloudShadowPass` storage texture is created and submitted by default

**Root cause.** `Renderer::new` always builds the pass (`renderer.rs:2121`) with `Rgba16Float` storage (`volumetric_clouds.rs:620-635`). `cloud_shadows_enabled` defaults true (`renderer.rs:3503`). The device does not enable `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` (`renderer.rs:1081-1092`). #205's failure mode was read-write storage; this pass is write-only, so it may be legal on the 4060 Ti. It is unconfirmed.

**Plan.** P4. Vulkan validation run. If it fails, create the pass only when enabled and when the format is legal. The demo already sets the flag false; the editor and examples do not.

### Owned by PR #205 (do not refile)

- God-ray composite read-write `rgba16float` storage.
- `vol_integrate_bg` given `Depth32Float`.
- `gpu_profiler_readback` copied while mapped, and `map_async` failure still calling `get_mapped_range`.

## Decisions (approved) and what landed

Reviewed against `main` @ `85d09a2c`. The demo crate is still only on PR #205, so the engine API is what that branch should call.

| Decision | Landed |
|---|---|
| Primary sky is Poly Haven CC0 `spruit_sunrise`. `kloppenheim_02` is the fallback. | The bytes are the terrain pack's `assets/terrain/polyhaven/pine_forest/hdri/spruit_sunrise_2k.exr` (PR #215, SHA-256 `6db7045b…`). This branch does not vendor a second copy. `bake_showcase_environment` loads that path, then the kloppenheim `.hdr`. `exr_equirect_decode_keeps_radiance_above_one` writes a tiny EXR and reads it back through the HDRI loader as float above 1. |
| HDR resize must not clamp radiance above 1. | `prepare_hdr_image` resizes `Rgb32F` / `Rgba32F` in float. Test `hdr_resize_keeps_radiance_above_one`. |
| Missing catalog paths: fetch CC0 or remove. Hashes in the manifest. | Six Poly Haven CC0 2k `.hdr` files fetched. `sky_equirect.png` removed (not a Poly Haven asset, not on disk). SHA-256 on every catalog entry and in `assets/asset_manifest.toml`. Test `catalog_files_match_sha256`. `resolve_sky_mode` refuses a missing file. |
| PBR maps in `astraweave-render`, no third parser. | `ModelSurfaceMaps` gained occlusion and emissive. The forward shader samples AO on indirect light and adds emissive. `pbr_maps` converts glTF MR and Poly Haven ARM. `nor_gl` is not Y-flipped. `load_gltf` reads base color, normal, MR, occlusion, and emissive, then attaches sibling `*nor_gl*` and `*_arm*` files that share the model stem (`arm_to_gltf_mr_and_occlusion`, stored in glTF layout so upload converts once). `add_composite_model` — the KayKit / Poly Haven / editor path — uploads those maps through `pack_gltf_surface`. Albedo-only primitives stay on the albedo bind group. Scatter LOD0 uses the same pack when a primitive has surface maps; later scatter LODs stay albedo because the simplified and billboard meshes do not carry the maps. |
| Time of day from the sim tick. | `TimeOfDay` no longer stores `Instant`. `update(dt_seconds)` and `SkyRenderer::update` honor `dt`. Test `fixed_dt_is_replay_stable`. |
| Cloud-shadow validation without duplicating #205. | `tests/created_pass_validation.rs` submits one `draw_into` from `Renderer::new_headless` (that constructor builds `CloudShadowPass`). Workflow `.github/workflows/render-device-validation.yml` runs it on lavapipe and on `windows-latest`. It does not construct `VolumetricFogPass` or `GodRayPass`. |

DF-1 through DF-4 are fixed in this branch. The issue text below is the ledger of what was wrong, for filing if a label is still wanted. PR #217's CI ledger already covers the audio device, SDK `cstdarg` / `cl` PATH, `CpuImage` import, candle `phi3`, and bench timeouts. Those are not refiled here.

## Dormant pass decisions

Each pass is wired, deleted with the live path already covering it, or deferred. Nothing below is wired on this branch: the depth slots still use filterable `texture_2d<f32>` against `Depth32Float`, or the live owner is another branch or another module. Nothing below is deleted: the live path does not implement that capability, or tests still construct the dormant type, so a delete is not parity. Issues are labeled `deferred-failure`.

| Pass | Decision | Issue |
|---|---|---|
| `AtmospherePass` and its aerial-perspective depth sample | Deferred. Selectable sky mode beside HDRI only after the depth binding is `texture_depth_2d`. Do not composite it on the HDRI. | #226 |
| `VolumetricFogPass` | Deferred. Wiring stays on PR #205. This branch does not edit the file. | #227 |
| `GodRayPass` | Deferred. Wiring stays on PR #205. This branch does not edit the file. | #228 |
| `VolumetricCloudsPass` raymarch | Deferred. `CloudShadowPass` stays live and is in the startup validation submit. The raymarch is not deleted: a shadow map is not a cloud volume. | #229 |
| SSGI | Deferred until its depth slots are `TextureSampleType::Depth` and the shader uses `textureLoad`. | #230 |
| TAA | Deferred. Same depth-binding gate. History stays presentation-only. | #231 |
| DoF (`advanced_post.rs`) | Deferred. `depth_tex` is `texture_2d<f32>` today. | #232 |
| Temporal upscale | Deferred. Depth at binding 3 is filterable float. | #233 |
| Frame-graph `SkyPassNode` / `VolumetricFogNode` | Deferred deletion. Both `run` methods return `Ok(())`. Live sky is `SkyRenderer`. | #234 |
| `BrdfLutPass` | Deferred deletion. Production LUT is `IblManager::bake_environment`. | #235 |
| `weather_system.rs` | Deferred deletion. Live weather is `effects::WeatherFx` + `SceneEnvironment`. | #236 |
| `weather_gpu.rs` | Deferred deletion. No production caller. | #237 |
| `shadow_csm.rs`, `shadow_quality.rs`, `shadow_sampling.wgsl` | Deferred deletion. Live CSM is `renderer.rs` + `shadow_common.wgsl`. | #238 |

## Frame time

Target class: Windows, Vulkan, RTX 4060 Ti, 1920×1080, 30 warmup frames, median of 120 frames of `draw_into` after `bake_showcase_environment(Medium)`, cloud shadows at their default, volumetrics off (those are #205).

This VM has no RTX 4060 Ti. A lavapipe submit proves the passes are legal. It does not estimate that GPU. The L.2 editor note measured a cold Kloppenheim bake at about 2.5 s. That is bake time, not frame time. Do not treat it as a 4060 Ti frame time. After #205's profiler fix, the same scene should log `gpu_profiler` medians on the target machine. Until that run exists, there is no honest frame-time number.

## What Phase 2 needs from review

The four review questions are decided. Dormant passes are tracked by #226–#238. The next atmosphere change is the depth-binding fix in #226, on a branch that is not racing PR #205's fog and god-ray files.

The next rendering phase is planned in `docs/current/RENDER_ARCHITECTURE_PHASE_PLAN.md` (bindless with fallback, one pipeline-creation function, compute particles, GPU frustum and Hi-Z occlusion, async streaming, mutable meshes, an 11 ms 1660 Ti budget). That document is a plan. It is not built.
