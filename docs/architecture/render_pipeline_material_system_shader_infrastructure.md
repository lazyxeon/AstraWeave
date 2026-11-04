---
schema_version: 1
trace_id: render_pipeline_material_system_shader_infrastructure
title: "Render Pipeline + Material System + Shader Infrastructure"
description: "Render Pipeline + Material System + Shader Infrastructure"
primary_crate: astraweave-render
domain: rendering
lifecycle_status: active
integration_status: wired
owns: [astraweave-materials, astraweave-render]
doc_version: "1.79"
last_verified_commit: b1e995cb35bf2c16a48dbf0842891142e538870a
---

# Architecture Trace: Render Pipeline + Material System + Shader Infrastructure

## Metadata

| Field | Value |
|---|---|
| **System name** | Render Pipeline + Material System + Shader Infrastructure |
| **Primary crates** | `astraweave-render` (123 Rust files / ~78K LoC + 71 WGSL files), `astraweave-materials` (single-file material-graph crate), `tools/aw_editor/src/viewport/` (editor-side renderer + engine adapter) |
| **Document version** | 1.79 |
| **Last verified against commit** | `b1e995cb35bf2c16a48dbf0842891142e538870a` (parent of the compile-time workspace-target notice; that parent still walks runtime `CARGO_MANIFEST_DIR` ancestors and stops at the first `target/` directory, including `astraweave-scene/target`). Prior `72c175483aa0883a2914b6316a5df02e0fa89411` (parent of the golden-backend census notice; that parent still leaves `test_backend_detection`, `golden_triangle_render`, and `golden_determinism_check` off `scripts/ci/gpu-skip-required-macos.txt`, and its unit test asserts those names are absent). Prior `35566609ed700697180f64e829348420cd499508` (parent of the device-refusal, depth-textureLoad, and golden-skip notice; that parent still records a device refusal from `request_headless` as a missing adapter, returns true from `skip_without_depth_texture_load` at `upscale_pass_creation`, `cloud_pass_creation`, and `enabling_sample_hiz_rebinds_to_the_current_depth_view`, and panics `golden_gradient_matches_with_tolerance` on `NotFound`). Prior `431f30bc0329007774315ba22443176ad0e70371` (parent of the hardware-fallthrough and skip-census notice; that parent matches `RequestAdapterError::NotFound` and still requests only the software adapter when `prefer_fallback` is set). Prior `99382af7` (parent of the splat `height_blend` width fix; that parent still declares `height_blend` as `array<f32, 8>` and calls it with the 32-element `raw_weights` and `layer_heights`). Prior `53d38a6b` (parent of the scene-vector tie and terrain-grid fixes; that parent still draws model, sphere, and weather instances as one `0..count` range with primitives in forward order, prefix-copies a larger square terrain grid by index, counts `sphere_bytes` and misses the growth-frame stage copy, and keeps a terrain vertex delta from an upload while the pass is off). Prior `a9de4ef4` (parent of the scene-vector order and motion fixes; that parent draws impostors in color-pass order under a biased `Less`, applies this frame's facing to `prev_world`, zeroes motion on a larger terrain upload, and counts weather bytes that were not copied). Prior `e1bc6ffc` (parent of the scene-vector submit fixes; that parent still copies weather and terrain without `COPY_SRC`, overwrites the cutoff uniform, and uses `slope_scale` -1). Prior `0a9e059b` (`draw_into` clamps Bruneton, and a sky-only request stays off; the pre-squash stamp `2707012a` is not an ancestor). The floor move to 110 is in `b1a0c4c2`; its parent `0784b204` still uses `min >= 100`. Prior `7b7a21e4` (log capture; `continue-on-error` is still on the three PBR test steps there and off in its child `0784b204`). Prior `039b031a` (parent of the log capture). Prior `794d816b` (lavapipe ICD pin on this branch). Prior `93c448d6` (counter-only floor on this branch). Prior `e8b2bde0` (parent of `93c448d6`; that parent builds the lavapipe `frame_alloc` step with `-C force-frame-pointers=yes`). Prior `ada51429` (parent of `e8b2bde0`; that parent copies frame-pointer return addresses and does not call `backtrace`). Prior `69422eb1` (v1.58 parent: the review-gap follow-up's parent still returns the poll error with `?`, which drops the cubes while the fence is open, and cites `test_sample_count_by_quality` for the wait). Prior `3855715e` (v1.57 parent: the IBL fence fix's parent still loops hundreds of mip-0 samples and does not poll inside `bake_environment`). Prior `f279eec9` (v1.56 parent: the Bruneton and cull-label notice). Prior `d0539b08` (v1.55 parent: the GPU-cull review fixes; those fixes are not in this parent). Prior `f3ca15b6` (v1.54 parent: the GPU-cull dispatch). Prior `417f267f` (v1.53 parent: the bounded image read). Prior `8d6e9867` (v1.52 parent: the single-gate notice). Prior `c0e291f1` (v1.51 parent: the atmosphere-sun notice). Prior `f72defda` (v1.50 parent: the flag-parser notice). Prior `83798b48` (v1.49 parent: the timestamp row's reject sentence). Prior `36757dad` (v1.48 parent; verified at that PR head; stamp updated on merge). Prior `2ce0da5d` (v1.46 behavior: `decode_texture` caps the read and `shutdown_timeout` is 500 ms; the extra byte past a full fifo is still there). Prior `596eed69` (v1.45 parent: Hi-Z contracts, lavapipe `stream_fallback`, decode runtime polls). Prior `f56b2fa4` (v1.45: the forward `HizPyramid` is gated on `sample_hiz`; `NaniteCullingPipeline` keeps a separate pyramid). Prior `09d49dfe` (v1.41: flat sample on the shard 13 and 14 targets that hit 600s). Prior `0758171bf` (lifecycle RSS); prior `154d723aa` (v1.19, 2026-08-02: L.3.C-resolution — 4-cascade CSM + `FarCascadePolicy` + fit-derived bias cap; trace updated in the same commit as the code, so this names its parent); prior `d4c4479d4` (v1.12, 2026-07-25: T.2d.F tier retirement — see revision notice); prior `8232b150b` (v1.10, 2026-07-21: E3 terrain render-path additions verified first-hand — see revision notice); prior `7c29b8182` (v1.8 verification pass: §2.6 Water surface render path re-verified end-to-end against `water.rs` / `water.wgsl` / `renderer.rs` — every file:line citation confirmed, zero corrections, no markers needed; v1.7 added the subsection + cross-link to `docs/architecture/water.md`); prior: `67c9de7e1` (full trace); water rows W.2a + W.2b.2 + W.2c.2 weave-deformation + W.2c.3 weave producer (2026-06-22) + F.4.2 weave-impact accent machinery + F.4.3 live HDR-overlay composite & combined-frame measurement (2026-06-24) |
| **Last verified date** | 2026-08-02 (L.3.C-resolution: 4-cascade CSM, far-cascade refresh policy, fit-derived receiver-bias cap, invariant 20 (e)-(i)); 2026-07-21 (E3 terrain render-path, T.0 trace-sync); 2026-06-25 (§2.6 water render-path verification pass); 2026-06-24 (water render-path subsection authored); 2026-05-10 (full trace) |
| **Status** | **ACTIVE WORKZONE** — Editor Multi-Tool Architecture Campaign Sub-phase 3 (Mediator Brush) is in flight as of campaign-doc commit `e3d07f366` (2026-05-08, Round-8-Closure). Fix 27 Unified Pipeline Campaign is structurally complete (per CLAUDE.md) but deeper editor↔runtime unification continues. Treat this trace as a **navigational map**; per-subsystem detailed traces are follow-up work. |
| **Owner notes** | This trace covers an unusually large system (~78K LoC source + 71 WGSL files + editor viewport). Per the template's "One last thing" rule on scale, this doc is intentionally structured as a **subsystem map + load-bearing-aggregator detail**, not an exhaustive per-file trace. Sub-systems like Lumen GI, MegaLights, Nanite, Atmosphere, GPU Particles, Volumetric Fog, IBL, and TAA each warrant their own dedicated trace if and when they enter focused work. The doc covers terrain materials by reference to `docs/architecture/terrain_materials.md`. |

> **Revision notice — 2026-10-29 (v1.79).** `record_gpu_skip_for_tests` and `tests/support/gpu_capability.rs` resolve the workspace `target/` from this crate's compile-time manifest (its parent joined with `target`), or from `CARGO_TARGET_DIR` when that is set. The calling test's runtime `CARGO_MANIFEST_DIR` is not walked. A `<crate>/target` directory no longer becomes the skip directory, and a log whose parent is the workspace `target/` still passes the guard. A path outside that directory still panics. Scene streaming tests write cell files under `CARGO_TARGET_TMPDIR` instead of `astraweave-scene/target`. `last_verified_commit` names the parent `b1e995cb35bf2c16a48dbf0842891142e538870a`. That parent walks runtime ancestors and panics once `astraweave-scene/tests/streaming_integration.rs` has created `astraweave-scene/target`.
>
> **Revision notice — 2026-10-28 (v1.78).** `last_verified_commit` names the parent `72c175483aa0883a2914b6316a5df02e0fa89411`. The pre-rebase stamps `e15d7dfa` and `e06b4f11` are not ancestors after the rebase onto `0213d1cf`. `golden_backend_tests` sets `cfg(test)`, so `test_backend_detection`, `golden_triangle_render`, and `golden_determinism_check` run under profile ci. Each requests a software adapter and records a skip when `AW_GPU_SKIP_IF_UNAVAILABLE` is set. Those three names are in `scripts/ci/gpu-skip-required-macos.txt`, and `required_macos_list_names_the_software_only_tests_profile_ci_runs` requires them. `gpu-tests` binaries stay off the list. That parent still omits the three names and asserts they are absent.
>
> **Revision notice — 2026-10-28 (v1.77).** `last_verified_commit` names the parent `35566609ed700697180f64e829348420cd499508`. `request_headless` panics on a device refusal even when `AW_GPU_SKIP_IF_UNAVAILABLE` is set. `NotFound` still records a skip only when that variable is set, and the caller name is captured before the first await. `upscale_pass_creation`, `cloud_pass_creation`, and `enabling_sample_hiz_rebinds_to_the_current_depth_view` record a skip or panic when depth `textureLoad` is absent. `golden_gradient_matches_with_tolerance` records `NotFound` under its own thread name. `scripts/ci/gpu-skip-required-macos.txt` adds the default-feature software-only tests profile ci runs. It does not name `gpu-tests` binaries or the `#[cfg(test)]` golden integration module. That parent still records a device refusal from `request_headless` as a missing adapter, returns true from `skip_without_depth_texture_load` at those three sites, and panics the golden compare on `NotFound`.
>
> **Revision notice — 2026-10-28 (v1.76).** `last_verified_commit` names the parent `431f30bc0329007774315ba22443176ad0e70371`. `prefer_fallback` tries a software adapter and then hardware, the order main `968ed8ba` used. It stays on the software adapter only when `AW_GPU_SKIP_IF_UNAVAILABLE` is set. A macOS runner then does not treat Apple Paravirtual Metal as the software probe, and a host whose software probe fails still keeps a working hardware adapter. The open flag is `libc::O_NOFOLLOW` in `renderer.rs`, `tests/support/gpu_capability.rs`, and `tools/aw_headless`. Waves 13–18 of `coverage_booster_render` call `headless_for_tests`. `inject_resident` and its three resident tests, `upscale_pass_creation`, both cloud adapter arms, the veil-fog scar upload, `test_water_renderer_new_and_update`, and `astraweave-fluids::try_create_test_device` record a skip or panic. `hiz_depth_copy` and `volumetric_validation` do the same when depth `textureLoad` is absent after a device exists. The Test full summary fails when a name in `scripts/ci/gpu-skip-required-macos.txt` is missing, including when the log already contains another line. That parent matches `RequestAdapterError::NotFound` and still requests only the software adapter when `prefer_fallback` is set. `6f725f62` and `33115cb1` are not ancestors of this head.
>
> **Revision notice — 2026-10-27 (v1.75).** `height_blend` in `shaders/pbr_terrain.wgsl` takes and returns `array<f32, 32>`, the same length as `raw_weights` and `layer_heights`. The loop still stops at `count`. `test_pbr_terrain_splat_validates` parses `pbr_terrain.wgsl` plus `pbr_terrain_vs.wgsl`, the `TERRAIN_SPLAT_SHADER` order. `last_verified_commit` names the parent `99382af7`. That parent still declares the function as `array<f32, 8>` and calls it with the 32-element arrays, which naga rejects (`Argument 0` type mismatch). The PBR job's `terrain_manager_full_pipeline_records_draw_without_validation_errors` panics on that mismatch.
>
> **Revision notice — 2026-10-25 (v1.74).** Scene-vector models, spheres, and weather draw one instance from the tail, and a model's primitives draw in reverse, so biased `Less` keeps the earlier color-pass surface. A square terrain grid whose side length changes copies the new vertices over the previous buffer, so that frame's delta is zero; a non-square growth still keeps the old prefix. `scene_vector_last_copy_bytes` counts `copy_buffer_to_buffer` only, including the growth-frame stage copy, and not the sphere or weather queue uploads. A terrain upload while the pass is off drops the replaced vertex buffer, so the first enabled frame stores zero object motion. `fs_terrain` still stores the geometric normal; the splat normal (`NORMAL_XY_STRENGTH` 1.4) is deferred issue #428. The pass stays off by default. `last_verified_commit` names the parent `53d38a6b`. That parent still draws those instances as one `0..count` range, prefix-copies a larger square grid, counts `sphere_bytes`, and keeps a terrain delta from an upload while the pass is off.
>
> **Revision notice — 2026-10-25 (v1.73).** Scene-vector impostors draw in reverse color-pass order, one instance from the tail. `prev_world` uses the previous view and `prev_position_scale.w`. A larger terrain upload keeps the old vertex prefix as the previous positions. `scene_vector_last_copy_bytes` counts encoder copies only. `fs_terrain` still stores the geometric normal; the splat normal (`NORMAL_XY_STRENGTH` 1.4) is deferred issue #428. `last_verified_commit` names the parent `a9de4ef4`. That parent draws impostors in color-pass order under a biased `Less`, applies this frame's facing to `prev_world`, zeroes motion on a larger terrain upload, and counts weather bytes that were not copied.
>
> **Revision notice — 2026-10-25 (v1.72).** Weather and terrain vertex buffers include `COPY_SRC`. Weather previous poses travel with the particle across swap-remove and are uploaded with the live count. The weather cutoff 0 and the mesh cutoff are copied into the bound uniform inside the encoder. Scene-vector depth bias is constant -2 and `slope_scale` 0. SSR loads Hi-Z mip 0 at the pixel. GTAO view Y is `ndc.y * z * tan(fov_y/2)`. The first enabled frame snaps impostor history to the current set. The pass stays off by default. `last_verified_commit` names the parent `e1bc6ffc`. That parent still fails submit when particles or a terrain chunk are drawn. `2707012a` is not an ancestor.
>
> **Revision notice — 2026-10-25 (v1.71).** `draw_into` clamps Bruneton before the sky branch. `composite_atmosphere` and `Renderer::resize` clamp again after a pass can clear `config.enabled`. A stale flag no longer clears HDR to the debug color with no HDRI and no Bruneton. `last_verified_commit` names the parent `2707012a`. That parent still clears HDR when a pass disables itself and never clamps. `2707012a` is not an ancestor of this branch.
>
> **Revision notice — 2026-10-24 (v1.70).** `set_atmosphere_enabled(true)` stays false while fog and shafts are both off. `record_bruneton` returns before the 50 km scatter in that pair. `clear_volumetric_passes` turns Bruneton off with the passes. The 4.474 ms cell still builds both passes first. `last_verified_commit` names the parent `968ed8ba`. That parent still enables Bruneton for a sky-only request.
>
> **Revision notice — 2026-10-23 (v1.69).** `4412e4d0` and `ada1aebe` are not ancestors. The metadata cell no longer lists them as `Prior`. After `93c448d6` the parent walk is `e8b2bde0`, then `ada51429`. `last_verified_commit` names the parent `d4e1ffab`. That parent still lists `4412e4d0` and `ada1aebe` as `Prior`.
>
> **Revision notice — 2026-10-22 (v1.68).** Stamps from the pre-rebase history are replaced with commits on this branch. `last_verified_commit` names the parent `b1a0c4c2`. `273a3811`, `46be9024`, and `fcf8f187` are not ancestors.
>
> **Revision notice — 2026-10-22 (v1.67).** The floor is min 110, max 130, spread 4. The `--all-features` low sample is 121. One pass is about 18, and `121 - 18 = 103` passed `min >= 100`. `110` keeps the steady lows 115 and 121 and fails that drop. `stream_fallback` on lavapipe is four samples: alloc-counter `[119, 116, 115, 116]`, `--all-features` `[125, 122, 121, 122]`, and it asserts `llvmpipe`. The full render step keeps the SIGSEGV comment: do not put `continue-on-error` back. The parent of that floor change is `0784b204`.
>
> **Revision notice — 2026-10-22 (v1.66).** `continue-on-error` is off the PBR steps for the full render suite, terrain materials, and advanced materials. A crash in those steps fails the job. Those lines are still present in `7b7a21e4` and gone in its child `0784b204`.
>
> **Revision notice — 2026-10-22 (v1.65).** Steps that write under `target/` run `mkdir -p target` first. The floor log is `cargo test … 2>&1 | tee`, so `adapter=llvmpipe` from stderr is in the file the grep reads. The PBR summary names `steps.quiet`, `steps.floor`, `steps.fallback`, `steps.render`, `steps.terrain`, and `steps.advanced`. `frame_alloc` and `stream_fallback` include `linux_test_threads.rs`. The lavapipe job exits if neither ICD file exists. That log-capture commit is `7b7a21e4`. Its parent is `039b031a`.
>
> **Revision notice — 2026-10-22 (from main).** `tests/instance_debug_flags.rs` includes `linux_test_threads.rs` on Linux. That change is `c1714ff7`, an ancestor. Main HEAD is `8938318f`. This branch's older v1.63 notice is the lavapipe ICD pin, not that hook.
>
> **Revision notice — 2026-10-22 (v1.63).** The hard `frame_alloc` step sets `VK_ICD_FILENAMES` and `VK_DRIVER_FILES` to the lavapipe ICD and fails if that file is absent. `WGPU_BACKEND=vulkan` stays. The stamp on this notice named `72bd4477`.
>
> **Revision notice — 2026-10-22 (from main `70053875`, v1.62).** The terrain and advanced PBR steps run `--test test_terrain_material` and `--test test_pbr_advanced`. Each step fails unless the log contains `test result: ok` with at least one pass. The summary check is that step's outcome. `last_verified_commit` on main names `8400bb06`.
>
> **Revision notice — 2026-10-22 (from main `70053875`, v1.61).** The toolchain's Linux lib-test step is `if: runner.os == 'Linux'`. `aw_headless` is nextest group `aw-headless-serial` with `max-threads = 1`. `tests/instance_debug_flags.rs` sets `WGPU_DEBUG` with `Command::env`. Integration tests include `tests/support/linux_test_threads.rs`. The loader exit is libvulkan ≥ 1.4.345.
>
> **Revision notice — 2026-10-22 (v1.62, historical).** This notice used to say the current tree still passed `-C force-frame-pointers` and named parent `c086feb3`. That commit is not an ancestor. The frame-pointer walk is not in this tree. The floor test reads `allocs()` only.
>
> **Revision notice — 2026-10-22 (v1.61, historical).** This notice claimed the allocator copied frame-pointer return addresses and that the hard step passed `-C force-frame-pointers=yes`. Neither is in this tree. The stamp named `4412e4d0`, which is not an ancestor.
>
> **Revision notice — 2026-10-22 (v1.60, historical).** This notice claimed the measured frame recorded stacks with `begin_alloc_trace` / `end_alloc_trace` and wrote `target/frame-alloc-trace.txt`. Those functions are not in this tree. The stamp named `ada1aebe`, which is not an ancestor. The +48 was `LockTrace::enter` when `RUST_BACKTRACE=1`. `RUST_LIB_BACKTRACE=0` before `main` is what keeps the ceiling at 130.
>
> **Revision notice — 2026-10-22 (v1.59, historical).** This notice claimed `continue-on-error` stays until a run names its adapter and the count is inside 100–130. That is not the current policy. v1.66 turns `continue-on-error` off the full render suite, terrain, and advanced materials. The lavapipe series it records is alloc-counter `[119, 116, 115, 116, 115, 115]` and `--all-features` `[125, 122, 121, 122, 121, 121]`. The stamp named parent `8fc15bd9`.
>
> **Revision notice — 2026-10-22 (v1.58).** `retire_submission` keeps the `SubmissionIndex` from `queue.submit` and matches `PollError::Timeout` on its own. Device loss does not return as `PollError`. A timeout parks the cubes on `IblManager`; the `?` path does not drop them while the fence is open. The next bake waits on that index before replacing them. `PollType::WaitForSubmissionIndex` blocks for wgpu-core `CLEANUP_WAIT_MS` (60s) and then returns `Timeout`. That is not the 6.3s `Queue::drop` budget (`100+200+400+800+1600+3200`), which panics. The duplicate invariant rows are 26 (struct layout) and 27 (fate-scars). Invariant 25 follows 24. It cites `test_prefilter_poll_finishes`, which bakes Low quality, runs `SPECULAR_PREFILTER_WGSL`, and counts a finished poll per mip face. `test_sample_count_by_quality` still only checks the Rust sample counts. `last_verified_commit` names the parent `69422eb1`. That parent still `?`s the poll error and cites `test_sample_count_by_quality` for the wait.
>
> **Revision notice — 2026-10-22 (v1.57).** `IblQuality::prefilter_sample_count` returns 1 at mip 0, because roughness 0 is a GGX delta. `bake_environment` polls `Maintain::Wait` after every submission so `Queue::drop` is not left waiting on the DX12 software adapter's 6.3s fence. `last_verified_commit` names the parent `3855715e`. That parent still loops 128–512 samples at mip 0 and does not poll inside the bake.
>
> **Revision notice — 2026-10-22 (v1.56).** The 4.474 ms capture's binary hardcoded `gpu_cull=cpu_fallback` and `gpu_cull_reason=group3_per_model` and had no `set_gpu_cull_enabled`. This binary calls `set_gpu_cull_enabled(true)`. After the samples it prints `gpu_cull=gpu` only when a `draw_into` dispatched the cull. A new run is not that measurement. Greybox leaves the flag off, so its `draw_into` stays on the CPU frustum. The constructor stays off. The game enables Bruneton only after fog and shafts both build. `last_verified_commit` is `f279eec9`, the parent of this notice.
>
> **Revision notice — 2026-10-21 (v1.55).** The indirect buffers grow to one element capacity, and the cull shader returns at the logical batch count. The dispatch is stamped `gpu_cull`, after the cloud-shadow gap closes. `rendered_model_count` skips a model the frustum rejects. Shadow cascades test `gpu_cull_bounds` as well as `model.aabb`. `write_model_vertices` rebuilds a stored mesh AABB from the new vertices. `draw_into` reuses the cull batch vectors. `gpu_frame_budget` prints `gpu_cull=gpu` only after a dispatch, and prints `gpu_cull_visible`. Nanite's Hi-Z `mip_level_count` is at least 1 and at most the valid chain. `last_verified_commit` names the parent `d0539b08`. That parent still rounds the AABB and command buffers separately, stamps the dispatch inside the open gap, and passes the unclamped Nanite mip count.
>
> **Revision notice — 2026-10-21 (v1.54).** `set_gpu_cull_enabled` defaults off. `draw_into` then keeps the CPU frustum and does not build the indirect buffers. When on, it dispatches `IndirectDrawPipeline` and draws each model with `draw_indexed_indirect` after binding that model's group 3. A composite with no explicit AABB stores a mesh AABB transformed by its instances outside `model.aabb`, and `duplicate_model` copies it. `add_model`, `add_model_with_texture`, `add_model_with_pbr_textures`, and `add_model_sharing_texture` do not, so those models are not frustum tested while the flag is on. `Renderer::render` keeps the CPU loop. `gpu_frame_budget` opts in. The 1660 Ti record at `939a21bb` is still the CPU-frustum capture. `last_verified_commit` names the parent `f3ca15b6`. This dispatch is not in that parent.
>
> **Revision notice — 2026-10-21 (v1.53).** `read_limited_image_bytes` is the sync 64 MiB read. `open_limited_image`, `Texture::from_file_with_usage`, `load_ktx2_to_rgba`, the ktx2 smoke-test fallback, a non-`data:` glTF image file URI, and async `read_capped_image` call it. A 1×1 `HizPyramid` has one mip. `last_verified_commit` names the parent `417f267f`. That parent does not contain this read helper or the 1×1 mip fix.
>
> **Revision notice — 2026-10-21 (v1.52).** `gpu_frame_budget` calls `reject_gated_flags`. That function accepts `--atmosphere` alone and describes the HDR and depth clear plus `record_bruneton`. `screen_ndc.wgsl` is deleted. `screen_uv_to_ndc` stays inlined in `fog_density.wgsl` and `scatter.wgsl`. `last_verified_commit` is `8d6e9867`, the parent of this notice.
>
> **Revision notice — 2026-10-21 (v1.51).** Rebased onto main `7427bc05`. The timestamp row keeps the Hi-Z sentence and the false-flag sentence. `--atmosphere` alone is the measurement: `atmosphere_with_aravath_fog` prints only when fog and shafts both build, and any other enabled gated flag is rejected. `record_bruneton` passes the light direction as the sun, or the time-of-day sun when that vector is empty, and the bound camera's near and far. The game camera and the budget camera use `GAME_ZNEAR` and `GAME_ZFAR`. `last_verified_commit` is `c0e291f1`, the parent of this notice.
>
> **Revision notice — 2026-10-21 (v1.50).** `flag_on` lives in `budget_gate.rs` and is covered by lib tests. Bare `--name`, `--name=true`, and `--name true` are on. `--name=false` and `--name false` are off and do not bail. The timestamp row keeps the Hi-Z sentence and the reject sentence. `last_verified_commit` is `f72defda`, the parent of this notice.
>
> **Revision notice — 2026-10-21 (v1.49).** The timestamp row records that `gpu_frame_budget` rejects every gated flag, including `--atmosphere`, before it builds the scene. `last_verified_commit` is `83798b48`, the parent of this notice. Main's v1.48 wording stays.
>
> **Revision notice — 2026-10-21 (v1.48).** `astraweave-render` enables tokio `io-util` on its own dependency line. Verified at the PR head. `last_verified_commit` names the parent `36757dad`: the trace convention is that the edit lands in the child, so the stamp cannot be this commit, and it does not claim `36757dad` contains this notice. The stamp is updated on merge.
>
> **Revision notice — 2026-10-21 (v1.47).** Corrected in v1.48. The shared `read_capped_image`, the stop at the cap with no further read, and the unix fifo cleanup are `36757dad`. They are not in `2ce0da5d`. The 1660 Ti `hiz` 0.275 ms figure is that capture; the default frame does not record Hi-Z unless `sample_hiz` is set.
>
> **Revision notice — 2026-10-21 (v1.46).** Corrected in v1.48. `596eed69` does not contain the 64 MiB cap or `shutdown_timeout`. Those are its child `2ce0da5d`: `decode_texture` reads at most `MAX_IMAGE_ALLOC` and decodes with `decode_limited_image`, and `Drop` waits at most 500 ms. `stream_fallback` releases its fifo writer on any exit, including a failing alloc assert.
>
> **Revision notice — 2026-10-21 (v1.45).** Invariant 21 names the Nanite pyramid in `nanite_gpu_culling.rs` (`src/shaders/nanite_hiz_pyramid.wgsl`). `draw_into` does not record it, and its copy from the previous depth is still a placeholder. §11 no longer says occlusion reads `HizPyramid`. The shader headers on `hiz_pyramid.wgsl` and `hiz_pyramid_copy.wgsl` match the gate. `render-device-validation.yml` runs `tests/stream_fallback.rs` on lavapipe, and the decode runtime polls on its thread, in the child of `f56b2fa4` (`596eed69`). The stamp names that parent and does not claim the parent contains those edits.
>
> **Revision notice — 2026-10-20 (v1.44).** `set_sample_hiz(true)` drops any pyramid and the next `draw_into` copies that frame's depth view, including when the size matches a view captured earlier (`tests/hiz_rebind.rs`). Occlusion, SSR, and SSGI still do not sample it. §5 marks the pyramid gated, not an unconditional `draw_into` build. Invariant 22 records Hi-Z only when `sample_hiz` is set. Five lavapipe runs with validation on, and five with it off, each recorded `[119, 116, 115, 116, 115, 115]`. Spread inside a run is 4. The gate is min 100, max 130, spread 4. 130 is 11 above the high sample. One extra pass is about 18 allocations, so that still fails.
>
> **Revision notice — 2026-10-20 (v1.43).** `draw_into` does not build the Hi-Z pyramid unless `sample_hiz` is set. No pass samples `HizPyramid::view`. The 64×64 alloc samples are `[119, 116, 115, 116, 115, 115]`. The 200–230 band was the six unused mip passes. The remaining floor is wgpu-core `BasePass` for the passes that still record (#269).
>
> **Revision notice — 2026-10-18 (v1.42).** Cloud-shadow darkness is `CLOUD_SHADOW_STRENGTH` (default 4). The march still takes 16 steps and 32 slices. The weather weight and the extinction are multiplied by that value. Indirect light follows `(strength - 1) / strength` of the same shadow, stored in the map's G channel. A weather weight of 0 stays clear, and strength 1 leaves skylight alone. Aravath enables the pass. The 1660 Ti cloud-shadow cost stays the 0.323 ms record; this change does not add dispatches.
>
> **Revision notice — 2026-10-18 (v1.41).** `camera_primitives_instancing`, `gpu_memory_terrain_skinning_depth_overlay`, `graph_mesh_material_texture`, `ibl_deferred`, and `transparency_environment_msaa` use `SamplingMode::Flat`, `sample_size(10)`, a 1 ms warm-up, and a 1 s measurement. On main `ceff979a` (run 39089221901) and on #374 (run 39091616710) the first two targets were killed at 600s. On #377 run 39100674332 those two finished, then `graph_mesh_material_texture` (shard 13) and `ibl_deferred` (shard 14) hit 600s and shard 14 was cancelled before `transparency_environment_msaa` finished. Every function still runs. The render targets are split across shards 13, 14, and 15 from the per-target walls on run 39109443441, so each job stays near 40 minutes. The 600s cap and the 60-minute job timeout are unchanged. The aggregate expects 16 shard files and 53 package records. `gh-repository` for github-action-benchmark is `github.com/owner/repo`, because the action clones `https://x-access-token:$TOKEN@$gh-repository.git`.
>
> **Revision notice — 2026-10-17 (v1.27).** `test_renderer_lifecycle_stress_test` keeps one wgpu Instance and Adapter, treats the first create/destroy as the warm baseline, and asserts VmRSS growth after that cycle stays under 50 MB. It also requires `generate_report` to show zero user-held buffers, textures, texture views, devices, and queues after each cycle. Lavapipe measured about 5.2 MB after the warm cycle.
>
> **Revision notice — 2026-10-16 (v1.26).** `IblResources` is `Clone`. The views are `wgpu` handles, so one bake can be installed on two renderers without a second convolution. The editor parity harness does that; the live adapter still bakes on its worker thread.
> **Revision notice — 2026-10-16 (v1.26).** Rebased onto main `cdea968c`. Volumetric fog and god rays stay the #205 passes. Timestamp slots are still allocated only when the pass is recorded, and `gpu_frame_budget --scene heavy` still prints CPU and GPU times with `budget_11ms=not_decided`. `TextureStreamingManager::new` returns `Result`.
> **Revision notice — 2026-10-16 (v1.26).** Rebased onto main `cdea968c`. Volumetric fog and god rays stay the #205 passes. Timestamp slots are still allocated only when the pass is recorded, and `gpu_frame_budget --scene heavy` still prints CPU and GPU times with `budget_11ms=not_decided`. `TextureStreamingManager::new` returns `Result`, and its decode runtime lives on its own thread. `draw_into` skips the Hi-Z pyramid when the device cannot `textureLoad` a depth texture. GL rejects that load.
>
> **Revision notice — 2026-10-17 (v1.40).** GTX 1660 Ti Max-Q, Vulkan, `939a21bb`, heavy, 1080p, 30 warmup + 120 samples: `median_gpu_frame_ms=3.688`, pass sum 3.616, CPU frame 8.821. `budget_11ms=pass` against that GPU frame (limit 11). The heavy scene still does not draw water. `gpu_cull=cpu_fallback` on every adapter. RTX 4060 Ti heavy-scene frames before the cloud amortization stay on record: Vulkan `31d45ffc` 4.252 ms, Vulkan `60c9aa28` 4.234 ms, DX12 `60c9aa28` 2.966 ms.
>
> **Revision notice — 2026-10-17 (v1.39).** Each frame writes 1/32 of the cloud-shadow rows (`CLOUD_SHADOW_SLICES`). The slice and the cloud time come from the presentation frame index, `(index / 32) * 32 / 60` seconds, not from wall time. A finished cycle matches a one-shot bake at that time and camera XZ. `IblParams._pad` holds the cycle's bake center so the shadow does not follow the camera between slices. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-17 (v1.38).** The cloud-shadow march hoists per-column coverage and wind, and takes 16 steps instead of 32. A 32-step and a 16-step bake of the same noon column differ by at most one 8-bit level on the tonemapped ground (0.18% of texels). Worley stays. The map stays `Rgba16Float` (R16Float is not a core storage format; R32Float is not linearly filterable). Workgroup stays 8×8. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-17 (v1.37).** The span between sky and the main pass is `cloud_shadow`: the default-on 512² transmittance compute. It is not an IBL rebake. The heavy scene still does not draw water. `gpu_cull=cpu_fallback` is the group-3 draw path on every adapter. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-17 (v1.36).** Named intervals cover the encoder between the frame stamps (`before_passes`, `after_shadow_*`, `sky_to_main`, `after_main`, and the same pattern through `after_particles`). The heavy scene does not install `WaterRenderer`, so the water stamp brackets a no-op. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-17 (v1.35).** `draw_into` timestamps sky, water, atmosphere, bloom, Hi-Z, and particles in addition to cluster, shadows, main, and post. `cluster_bin` is stamped only on the frame that recomputes light bins. The timestamp copy and the map are CPU waits outside `median_gpu_frame_ms`. `budget_basis=gpu_frame`. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-17 (v1.34).** Soft particles sample scene depth with a read-only depth attachment. The default pipeline does not write depth. A depth-writing particle pipeline samples a copy taken after the geometry pass. The lavapipe heavy-scene test enables that emitter and fails if the frame scope reports an error or `gpu_exec` is not `0xA1B2C3D4`. Fog and light shafts stay compute passes that sample depth after the writing pass has closed.
>
> **Revision notice — 2026-10-17 (v1.33).** `gpu_frame_budget` installs a logger, exits on an uncaptured wgpu error, and pops a validation scope and an out-of-memory scope after every frame. The frame encoder copies a known `u32` into a readback buffer. `0` means that command buffer did not run. The tool also prints renderer-dependent features and limits for the Vulkan and DX12 adapters. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-17 (v1.32).** `resolve_query_set` is recorded on the `CommandEncoder` that wrote the frame's timestamps. `submit_frame` does not open a second resolve encoder: wgpu tracks query state per encoder, and NVIDIA returns 0 for a resolve that encoder did not write. The submission log prints each stamp's encoder id and `encoder_count` for the writes plus the resolve. The copy stays a later submission. `draw_into` does not create an encoder or call `queue.submit`. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-17 (v1.31).** The submission log also records the resolve buffer, the copy source and destination, the mapped staging buffer, the map callback status, and the first 16 raw ticks. `--fresh-query-set` replaces the query set and both timestamp buffers before every frame so a sample can match the self-test's unused set.
>
> **Revision notice — 2026-10-17 (v1.30).** `draw_into` brackets each timestamped pass with `GpuProfiler::begin_stamp` / `end_stamp`, the same `write_timestamp` the self-test uses. Pass descriptors no longer carry `timestamp_writes`, so those writes stay on the encoder that is submitted with the resolve. The submission log records the query-set token, index, and submit id.
>
> **Revision notice — 2026-10-16 (v1.29).** Timestamp resolve is the second command buffer of the same `queue.submit` as the writes. Waiting for the writes and resolving in a later submission deadlocks NVIDIA: `VK_QUERY_RESULT_WAIT_BIT` runs while those writes are still queued, and `device.poll(Wait)` does not return. Every profiler wait is a non-blocking poll with a 5s limit. A pass whose begin and end ticks match and are non-zero is empty, not a failed self-test.
>
> **Revision notice — 2026-10-16 (v1.28).** `draw_into` does not resolve timestamp queries. One wgpu encoder becomes a Vulkan command buffer per pass, and resolving in that submission reads all zeros on NVIDIA, including the `frame` bracket. A one-pass self-test still succeeds. `submit_timestamped` waits for the write submission, resolves, waits, then copies. The heavy scene places one model per instance via `duplicate_model`, so `rendered_models` matches the instance count. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-16 (v1.27).** Timestamp readback resolves queries in the frame encoder and copies them to the staging buffer in a second command buffer submitted with it. Vulkan returns all zeros, including the encoder `frame` bracket, when `copy_buffer_to_buffer` follows `resolve_query_set` in one command buffer (wgpu#6406). `timestamp_period_ns=1` skips wgpu's timestamp normalizer, so nothing else orders that copy. `GpuProfiler::prove_readback` runs that write/resolve/copy/wait/read path before `gpu_frame_budget` measures, and fails if any pair is `0, 0`. `budget_11ms` stays `not_decided`.
>
> **Revision notice — 2026-10-16 (v1.26).** Rebased onto main `cdea968c`. Volumetric fog and god rays stay the #205 passes. Timestamp slots are still allocated only when the pass is recorded, and `gpu_frame_budget --scene heavy` still prints CPU and GPU times with `budget_11ms=not_decided`. `TextureStreamingManager::new` returns `Result`, and its decode runtime lives on its own thread. `draw_into` skips the Hi-Z pyramid when the device cannot `textureLoad` a depth texture. GL rejects that load. A pass whose begin and end timestamps are the same non-zero tick is empty and is left out of the pass sum. An unwritten or inverted pair still fails, and the error prints the pass name, both ticks, and `timestamp_period_ns`. `draw_into` also writes an encoder `frame` pair when `TIMESTAMP_QUERY_INSIDE_ENCODERS` is present.
>
> **Revision notice — 2026-10-15 (v1.21).** Texture, material, impostor, smoke-test, and HDRI file reads decode through `astraweave_asset::open_limited_image` / `decode_limited_image` (4096-pixel edge, 64 MiB allocation).
>
> **Revision notice — 2026-10-16 (v1.25).** `draw_into` allocates a timestamp slot only when that pass is recorded. Resolving a slot that cluster or a skipped shadow cascade never wrote made every pass duration read 0 on the 1660 Ti. `timestamp_duration_ms` rejects a non-positive period, an unwritten pair, and a non-positive span. `gpu_frame_budget --scene heavy` is 64 instances of the GLB, shadows on, and one terrain chunk when `terrain-splat-arrays` is on. It prints `median_draw_into_ms`, `median_cpu_frame_ms`, and `median_gpu_pass_sum_ms`, and still prints `budget_11ms=not_decided`. `TextureStreamingManager::new` returns `Result` if the decode runtime cannot start. Invariant 22.
>
> **Revision notice — 2026-10-15 (v1.24).** Atmosphere, cloud raymarch, SSGI, TAA, DoF, and temporal upscale sample scene depth as `texture_depth_2d` with `textureLoad`. `draw_into` records them only when the matching setter is on, so the allocation gate is unchanged. `VolumetricFogNode`, `BrdfLutPass`, and `weather_gpu` are removed. Live fog and god rays stay on PR #205. `SkyPassNode` still reserves the graph's HDR and depth targets; the sky draw is `SkyRenderer`. `weather_system.rs` and `shadow_csm.rs` stay, because wiring them would be a second weather state and a second CSM.
>
> **Revision notice — 2026-10-15 (v1.23).** `gpu_frame_budget` prints the median `draw_into` time for the 1920×1080 capture scene and does not decide the 11 ms budget. The alloc gate's jump from the #269 110–120 floor to about 217 is the 6-pass Hi-Z pyramid (`ceil(log2(64))` at the 64×64 test). GPU indirect culling stays planned until the forward shader samples bindless; group 3 is still per model. Particle seed 9 is required to match a second seed-9 run.
>
> **Revision notice — 2026-10-15 (v1.22, rendering architecture steps 1–7).** `draw_into` keeps instance and visible-list scratch and builds one `HizPyramid`: `shaders/hiz_pyramid_copy.wgsl` loads `texture_depth_2d` into R32Float mip 0, and `shaders/hiz_pyramid.wgsl` downsamples that copy with `textureLoad`. SSR and SSGI take `HizPyramid::view`. CPU `FrustumPlanes` stays the model draw filter. `GpuParticleSystem::update` emits into dead slots and integrates only when `Renderer::set_particle_emitter` is set; the billboard pass samples depth as `texture_depth_2d`. `TextureStreamingManager` decodes on a one-thread tokio runtime and `tick` uploads at most one image per frame from a ring reserved in `new`. `Renderer::request_streamed_albedo` binds a 1×1 fallback until that upload lands. Load failures are stored as `Failed`. Invariant 21. The pre-build inventory is `docs/current/RENDER_ARCHITECTURE_PHASE_PLAN.md`.
>
> **Revision notice — 2026-10-15 (v1.21, WS-RENDER-ENV-A).** KayKit and Poly Haven GLBs upload through `Renderer::add_composite_model`. `pbr_maps::pack_gltf_surface` builds `ModelSurfaceMaps` (engine metallic-roughness) when a primitive has a normal, metallic-roughness, occlusion, or emissive image; albedo-only primitives stay on the albedo bind group. `mesh_gltf::attach_sidecar_pbr_maps` fills empty slots from sibling `*nor_gl*` and `*_arm*` files that share the model stem. ARM is stored in glTF layout and converted once at upload. `nor_gl` is not Y-flipped. Scatter LOD0 uses the same pack. Dormant passes stay unwired; each deferral is a `deferred-failure` issue (see `docs/current/RENDER_SKY_CAMERA_ARCHAEOLOGY.md`).
>
> **Revision notice — 2026-10-15 (v1.20).** `mesh_gltf::load_gltf` opens the document with `Gltf::open`, then rejects buffer and image file URIs that fail `astraweave_security::path::safe_under` against the glTF's parent directory. `gltf::import` runs only after that check. `data:` URIs are left in memory.
>
> **Revision notice — 2026-10-15 (v1.25).** `depth_texture_load_supported` compiles a one-binding compute probe that `textureLoad`s a `texture_depth_2d`. The GL backend rejects that load. The three volumetrics tests that need the integrate pass log that reason and return. Lavapipe still builds the pass and submits both frames.
>
>
> **Revision notice — 2026-10-15 (v1.24).** `compute_joint_matrices` sizes both buffers with `Vec::with_capacity(joint_count)` before `compute_joint_matrices_into`. `Vec::new` plus `resize` used amortized growth, so a 3-joint skeleton returned capacity 4 and `test_stress_memory_stability` treated that as a leak. The returned vec's capacity now equals its length. The into path is unchanged.
>
>
> **Revision notice — 2026-10-15 (v1.23).** `AnimationClip::sample_into` and `compute_joint_matrices_into` write into caller-owned buffers so a repeat pose does not allocate. `Renderer::write_model_vertices` and `Renderer::update_model_transform` update an existing model. Full-array mesh vertex buffers include `COPY_DST` so that rewrite is valid. The Aravath greybox uses both on the character path.
>
>
> **Revision notice — 2026-10-14 (v1.22).** God-ray composite no longer uses `texture_storage_2d<rgba16float, read_write>`. That access is outside the WebGPU guaranteed feature set for `Rgba16Float` (`STORAGE_READ_ONLY | STORAGE_WRITE_ONLY` only), so pipeline creation failed on DX12 and every later use reported `god_rays_composite_pipeline is invalid`. The pass now samples the HDR scene and writes a separate write-only target, then copies it onto HDR. Volumetric integrate binds scene depth as `texture_depth_2d` / `TextureSampleType::Depth`; binding `Depth32Float` to a filterable `texture_2d<f32>` made `vol_integrate_bg` invalid. If either pass fails its error scope, or `Rgba16Float` write-only storage is unsupported, the renderer drops that pass and keeps drawing. The first wgpu error is logged with the prefix `FIRST wgpu error`. Adapter selection prefers a discrete NVIDIA device and logs name, backend, driver, limits, and the rgba16float storage flags. The GPU profiler does not copy into `gpu_profiler_readback` while that buffer is mapped, and it does not call `get_mapped_range` when `map_async` fails. Mesh uploads use `queue.write_buffer` instead of a mapped-at-creation buffer.
>
>
> **Revision notice — 2026-10-14 (v1.21).** Fate-scars are an input of the existing froxel fog pass, not a new volumetric pass. `fog_density.wgsl` samples a 32³ world-space RGBA16F volume (`t_scar`) at the froxel's world position and adds density. G/B of the density texture carry a forward-scatter weight and an amber-violet tint weight, which `scatter.wgsl` applies. The froxel grid is unchanged (low preset stays 80×45×32). `VolumetricFogPass` timestamps the density and scatter dispatches when `TIMESTAMP_QUERY` is available. `--no-volumetrics` still skips pass creation. `FogDensityParams` and `ScatterParams` gained padding so the CPU buffers match WGSL `vec3` alignment (160 and 176 bytes). A debug dispatch was rejected before that padding: the shader expected 160 bytes and the density buffer was 144.
>
>
> **Revision notice — 2026-10-13 (v1.20).** `Renderer::draw_into` and `Renderer::render` composite engine volumetric fog and screen-space light shafts into the HDR target before the ACES blit. Fog scatter samples the live CSM depth array (`scatter.wgsl` is `texture_depth_2d_array`). Both passes stay disabled until `set_volumetric_fog_config` / `set_god_ray_config`. The Aravath demo turns them on with the low froxel preset (80×45×32) and half-resolution shafts (32 samples). The sun still comes from `TimeOfDay` unless `set_light_direction_override` is set.
>
>
> **Revision notice — 2026-10-14 (v1.20, WS-RENDER-ENV-A).** Showcase IBL bake is `Renderer::bake_showcase_environment` (terrain pack `assets/terrain/polyhaven/pine_forest/hdri/spruit_sunrise_2k.exr`, fallback `kloppenheim_02`). HDR downsample stays in float. `TimeOfDay::update(dt)` no longer reads `Instant`. Group 3 gained occlusion (binding 7) and emissive (binding 9); AO scales indirect only. glTF loader reads normal, metallic-roughness, occlusion, and emissive. Startup-pass validation is `tests/created_pass_validation.rs`. Dormant-pass dispositions are in `docs/current/RENDER_SKY_CAMERA_ARCHAEOLOGY.md`.
>
> **Revision notice — 2026-07-29 (v1.18, beat L.3.A).** The L.3 render gate was REJECTED from a moving survey camera — "shadows attached to the camera boundary" — and the cause was the CSM's own coverage geometry: `shadow_far = 500` view-distance cap + a 100 m fade (400→500), a camera-centered bubble that static-station verification could not see (L.3's y414 zero-pixel result was its fingerprint). **The lesson minted: camera-fitted systems get camera-MOTION verification legs** (`tools/aw_editor/tests/l3a_proof.rs` — radial profile at two survey altitudes + a 600 m lateral sweep with world-anchored shadow features tracked by reprojection; assertions armed post-fix). **The fix: a third (survey) cascade** — coverage 500→3000 m (`SHADOW_FAR_SURVEY`), fade widened to the last 30% (2100→3000); c0/c1 fits still computed against 500 so their matrices are bit-identical to L.3 (close/mid stations byte-match: 0 differing pixels). Shadow map array 2→3 layers (2048², +16.8 MB); `MainLightUbo` = 3×mat4 + `splits: vec4 (split0, split1, shadow_far, fade_start)` + extras (224 B); `csm_shadow_factor` selects 3-way — both static and terrain shaders inherit through the single shared implementation; the dormant skinned module's inline struct updated for layout compatibility (still samples c0/c1 only). Both pass loops render 3 layers with per-cascade AABB culling (measured at the survey framing: c2 draws 293–372 of 441 chunks); per-cascade UBOs stay separate (invariant 3). Off-state bit-identical (transitively to pre-L.3). y414 now shows its relief shadowed (147.19→116.27 / rake 130.07→103.71). Invariant 20 amended (3 cascades). Evidence: `docs/audits/L3A_OUTCOME.md`.
>
> **Revision notice — 2026-07-28 (v1.17, beat L.3).** Terrain casts into and receives from the CSM — the live 2-cascade machinery gains its terrain consumer, and the L.1/v1.14 promise ("cascades gain a consumer in L.3") comes due. (1) **Receiver**: the static SHADER_SRC's inline cascade-select + PCF-3×3 block is hoisted VERBATIM into `shaders/shadow_common.wgsl` (`MainLightUbo` + `csm_shadow_factor`; textures/samplers as function params, consumers declare their own globals — the ibl_common contract); SHADER_SRC calls it at group(2), `pbr_terrain_forward.wgsl` at its new group(4), whose bound group is the renderer's EXISTING `light_bg` (one owner, §7.7 — zero new GPU resources; layout via the shared `TerrainMaterialManager::create_shadow_bgl`, now also used by `Renderer::new` and the pipeline tests). The shadow term multiplies the DIRECT sun only — IBL stays unshadowed (shadowed ground remains skylit). Terrain fragment stage: **15/16 sampled textures** (the slot L.2 reserved). Terrain pipeline layout is now 5 bind groups (above wgpu's downlevel default 4 — `tests/test_utils.rs` raised to 5). (2) **Caster**: new depth-only `terrain-shadow-pipeline` (`ensure_shadow_pipeline`: `TerrainSplatVertex` position-only, world-space, no instancing, bias constant 2/slope 2.0) draws `tf.chunks` into BOTH cascade layers of `draw_into`'s existing shadow passes with per-cascade AABB culling (`TerrainChunkGpu` gained `aabb_min/max`, computed in `upload_terrain_chunk` — single source of chunk bounds); `has_shadow_casters` includes terrain; per-cascade (drawn, total) counts observable via `Renderer::terrain_shadow_stats()`. Measured culling at the pinned stations: c0 draws 0–7 of 441/169 chunks, c1 18–33. Statics-additive: statics already cast via `shadow_pipeline` in the same passes. (3) **Sentinel unchanged and proven**: `extras.x < 0` disables both shaders' sampling and skips the passes; off-state vs pre-L.3 = 0 differing pixels (bit-exact at the toggle stations). (4) **Corrections**: the live splits at the editor terrain camera are ≈86.1 m/500 m (upload-path λ=0.7; the in-source "split0 ≈ 16" comment was the λ=1.0 value — fixed); `set_cascade_extents` is a NO-OP (fields never read — sphere fitting superseded them; T2F's "80 m extent" figure came from this dead call); `shadow_csm.rs`/`shadow_quality.rs`/`shadow_sampling.wgsl` are a DORMANT parallel CSM (zero production callers) — §5 rows corrected, §11 question opened; dead `terrain_c*` guards deleted from both shadow passes. New **Invariant 20**. Evidence: `docs/audits/L3_OUTCOME.md`.
>
> **Revision notice — 2026-07-28 (v1.16, beat L.2 calibration).** Director-ratified calibration shipped: (A) `EQUIRECT_TO_CUBE_WGSL` clamps bake-cube radiance to `min(rgb, 2.0)` — the BAKE path only; the visible sky reads the unclamped `hdr_equirect` and keeps its sun disk. Kills the sun-core quadrature fireflies (irradiance neighbour-cluster spread 15.9% unclamped → 2.4% clamped, now a HARD assertion `< 5%` in `tests/ibl_irradiance_faces.rs` — proven to fail with the clamp removed) and the env-sun double-count against the analytic sun. (B) `rebuild_ibl_bind_group` ships a FIXED `ibl_intensity = 1.5` replacing the image-average normalisation (measured over-drive 2.4–2.5×; 1.0 documented as the one-line fallback); `avg_luminance` still computed for diagnostics (residue C deferred). (E) New `Renderer::install_baked_environment(IblResources)` — the tail of `bake_environment` exposed so an environment baked on another thread (the editor's worker-thread init bake, aw_editor.md v1.13) can be installed without blocking; rebuilds both IBL bind groups (group 5 + terrain group 3) and flips the sky. Final A/B at the shipped calibration: desert +33 / grass +22 mean luma over the L.1 baseline, sd 9.9–15.6, glitter-free. Evidence: `docs/audits/L2_OUTCOME.md` §0.1.

> **Revision notice — 2026-07-27 (v1.15, beat L.2).** IBL wired into editor terrain — and the bake chain repaired first. (1) **Two bake defects fixed** (`b1295b93f`, regression-locked by `tests/ibl_irradiance_faces.rs` which run-and-FAILED pre-fix): the irradiance convolution was face-blind (N.z pinned to 1.0, no face uniform — all six faces held the same +Z convolution) and **every bake write pass stored its target vertically flipped** (uv from clip space without the v-flip; cube faces v-flipped per face, BRDF LUT roughness-flipped, specular only *looking* right via a double-flip cancellation). Post-fix all six irradiance face centres agree with an independent CPU reference to ≤2.1% and +Y (sky) > −Y. (2) **`compute_ibl` hoisted verbatim into `shaders/ibl_common.wgsl`** (shared IblParams + function), consumed by both `SHADER_SRC` and `TERRAIN_FORWARD_SHADER` (concat order: constants, brdf_common, ibl_common, stochastic_tiling, terrain_forward — mirrored in `shader_validation.rs`; `ibl_common.wgsl` registered as a concatenation fragment). (3) **Terrain-forward consumes IBL at group(3)**: 5-entry `terrain_ibl_bgl` (`TerrainMaterialManager::create_terrain_ibl_bgl`, shared by `Renderer::new` and the pipeline tests), bind group renderer-owned and rebuilt ONLY inside `rebuild_ibl_bind_group` alongside group 5 (same views, same `ibl_params_buf`, same sampler — §7.7 one-owner rule); `ensure_forward_pipeline` gained the `ibl_bgl` param; bound once before the chunk loop in `draw_into`. Fragment stage at 14/16 sampled textures (the 9-entry group-5 layout would hit the 16 limit and block L.3's shadow map). The flat ambient at `pbr_terrain_forward.wgsl` is REPLACED by `compute_ibl(...) * final_ao` (AO on indirect only; `uScene.ambient_*` no longer read by terrain — still drives the statics ambient floor). (4) **Editor bakes at adapter init** (`bake_default_environment`: tracked `kloppenheim_02_puresky_2k.hdr`, Medium, warn-and-skip on absence; 2.48 s cold). (5) **Calibration STOPPED pending director ratification**: the image-average `ibl_intensity` scheme over-drives (~2.4–2.5×) and the sun core aliases into irradiance fireflies (35% tilt-fan spread) — measured, previewed remedies (bake-source sun-clamp + fixed intensity) in `docs/audits/L2_OUTCOME.md` §4–5. Shader-validation vacuity floor recalibrated 60→55 (stale-red at baseline, stash-proven). Evidence: `docs/audits/L2_OUTCOME.md`.

> **Revision notice — 2026-07-26 (v1.14, beat L.1).** Exposure control, additive and visually neutral at defaults: `SceneEnvironmentUBO`/`SceneEnvironment` gained `exposure` (the second commandeered pad float, offset 72 — 96-B layout unchanged, asserted by test; mirrored in `TerrainSceneEnvGpu.exposure`, never read by the terrain shader). The POST shaders (`POST_SHADER` + `POST_SHADER_FX`) now read `uPostScene.exposure` instead of the hardcoded `1.35`; `scene_environment::DEFAULT_EXPOSURE = 1.35` pins the default to the former hardcode so untouched paths render byte-identically (proven at 4 stations). `set_post_process_chain`'s doc now states the consumption truth (only `bloom_enabled` drives a pass in `render()`/`draw_into()`; bloom output uncomposited — P.3 residue), and `update_cascade_splits` carries the L.1 note explaining why it must keep running when shadows are off (it delivers the `extras.x` shadows-off sentinel; cascades gain a consumer in L.3). Editor-side: the World panel's lighting defaults are pinned to the delivered state and the frame-1 push race is fixed in `aw_editor` (see aw_editor.md v1.11). Evidence: `docs/audits/L1_OUTCOME.md`.
>
> **Revision notice — 2026-07-26 (v1.13, beat ED-3).** Editor debug shading, additive: `SceneEnvironmentUBO`/`SceneEnvironment` gained `debug_mode` (a commandeered pad float at offset 68 — 96-B layout unchanged, asserted by test; flows to terrain via the existing `bytemuck::cast` into `TerrainSceneEnvGpu`, whose pad was split the same way). The static, skinned, and terrain-forward fragment shaders branch uniformly on `uScene.debug_mode` (0 lit / 1 unlit albedo / 2 world-space normals / 3 UVs — skinned has no UVs and renders a mid-gray sentinel). `Renderer::set_debug_shading(mode, wireframe)` + `wireframe_supported()`; wireframe = polygon-mode-Line variants (`pipeline-wire`, `terrain-forward-pipeline-wire`), feature-gated on `POLYGON_MODE_LINE`, selected via `main_mesh_pipeline()` / the terrain manager's `set_wireframe`. Game/runtime paths keep `debug_mode = 0`. Enforced by `ed3_debug_shading_reaches_every_shader_surface`. Evidence: `docs/audits/ED3_OUTCOME.md`.
>
> **Revision notice — 2026-07-25 (v1.12, beat T.2d.F).** The material-LOD tiers are **deleted**, director-ratified, after the T.2d diagnosis identified the LOD1|2 footprint threshold as the observed camera-anchored terrain boundary and its per-pixel selection as far-field tier dithering. Consumers removed at four surfaces (brdf_common's two functions; SHADER_SRC's BRDF call **and its LOD-gated IBL branch**; SKINNED_SHADER_SRC; `pbr_terrain_forward.wgsl`). New **Invariant 19** (single BRDF, no stepped tier, enforced by a fails-on-old-code absence test). Before/after proof at five pinned ED-2 stations + min-spec timing: `docs/audits/T2DF_OUTCOME.md`; stations persist in `.editor_preferences.json` for the director's closing re-check.
>
> **Revision notice — 2026-07-24 (v1.11, beat T.2a).** Both terrain-shader constants v1.10 flagged
> as open are now **measured and retuned**, each with isolated A/B evidence at pinned camera
> stations (`tools/aw_editor/tests/terrain_ab_stations.rs`; full ledger in
> `docs/audits/T2A_OUTCOME.md`). **(a) `NORMAL_XY_STRENGTH` 1.8 -> 1.4** (`pbr_terrain_forward.wgsl:331`,
> applied `:332-333`). Correcting v1.10's phrasing: it is **not** mip-gated — there is no mip check,
> LOD branch or distance term anywhere near it; it is a function-local `let` inside the per-layer
> loop applied unconditionally to every fragment, and the distance falloff is an *emergent* property
> of the hardware mip chain. Its effect is a slope steepening, `atan(k * tan theta)`, on the
> hex-blended tangent normal's XY only. Measured mean-|Laplacian| reduction scales with how much
> relief the normal map carries (grassland -14.6%, mountain -14.5%, forest -9.7% at 1.4; -31%/-28%/-18%
> at 1.0) and is near-nil on near-flat normals (desert -2.8%) — i.e. it amplifies authored data
> rather than adding structure. **(b) hex weight sharpening `pow(hex.w, 4.0)` -> `2.0`**
> (`:264-266`). The exponent *is* the blend width (no separate constant), and with `tiling = 128`
> over a 512 WU chunk one hex cell is ~4 m of world. Its A/B signature is the opposite of (a): a
> near-uniform **-9 to -11% at every station**, including two materials with essentially no content
> of their own — the signature of a structural lattice superimposed on the surface. This is
> confirmed to be the same artifact the T.W.1.A water session saw refracting through shallow water
> (`TW1A_OUTCOME.md` §6, "checkered diamonds", handed forward as terrain-owned). The two knobs are
> near-independent (joint pass predicted from the isolated deltas to within 94-101%), so either can
> be amended one line at a time. **Two structural findings recorded, not fixed:**
> `downsample_rgba8_box` (`terrain_material_manager.rs:1519`) box-averages the *normal* array as raw
> RGBA8 with no renormalization, and `canonical_terrain_pack.rs:231-246` resizes normals to 512 with
> a triangle filter, also without renormalizing — both shrink normal XY toward flat, which is the
> root cause `NORMAL_XY_STRENGTH` was invented to compensate; and `in.uv` is per-chunk normalized
> `[0,1]` (`engine_adapter.rs:2382-2395`), so `hex_cells` hashes chunk-local coordinates and **every
> 512 m chunk receives a byte-identical hex random field** — the de-tiling mechanism itself tiles.
> The ratified Q4 aux-resolution question is **retired without a purchase**: the close-up defect
> decomposed into channel data + these two constants, raising disk resolution buys nothing (the
> loader clamps unconditionally), and an aux 512 -> 1024 increase would take the 32-slice allocation
> from 256 MiB to 512 MiB while making the largest residual defect sharper.

> **Revision notice — 2026-07-21 (v1.10, commit `8232b150b`, T.0 trace-sync).** Records the render-side of the E3 terrain build (`d506658d8`, 2026-07-03 — no trace was updated in that commit; reconstruction at `docs/audits/E3_PREFLIGHT_2026-07.md`). Three changes to the 32-layer terrain forward path, all verified first-hand at HEAD: **(a) stochastic hex-tile sampling wired** — the formerly zero-consumer `shaders/stochastic_tiling.wgsl` is now composed into `TERRAIN_FORWARD_SHADER` (`terrain_material_manager.rs:171-176`, used at `:1025`); `pbr_terrain_forward.wgsl` takes 3 taps per layer via `hex_cells()` (`stochastic_tiling.wgsl:183`) with per-cell random **rotation** (the load-bearing decorrelator — translation-only preserves periodicity) + translation, pow-4-sharpened weights (`pbr_terrain_forward.wgsl:264-266`), gradients rotated for correct mip/aniso selection, and tangent-space normals decoded per tap and counter-rotated by the cell conjugate. **(b) Layer arrays gained full mip chains** — albedo + 3 aux arrays build real mip counts with a CPU box-filter chain at upload (`terrain_material_manager.rs:1350,1364,1378,1392`; the pre-fix `mip_level_count: 1` grain/shimmer is recorded at `:1337`); the layer sampler is `anisotropy_clamp: 8` (`:1421`); per-chunk splat textures stay deliberately unmipped/`anisotropy_clamp: 1` (`:824-825`, `:1168-1169`, `:1440` — untiled, ClampToEdge). **(c) `NORMAL_XY_STRENGTH = 1.8`** (`pbr_terrain_forward.wgsl:331`, applied `:332-333`) — an interim constant compensating relief flattening from 512² aux resolution + box mips; its retirement-or-keep decision is beat T.2 of the terrain campaign (data-first, no pre-committed aux-resolution increase — director ratification 2026-07-20). Editor-side consumption (per-vertex biome-driven `material_ids`/`material_weights`) is traced in `aw_editor.md`; the pack/loader slice in `terrain_materials.md`.
>
> **Revision notice — 2026-06-24 (v1.7, commit `7c29b8182`).** Added **§2.6 Water surface render path (W-series)** — the W-series water rendering flow (chunked-LOD Gerstner surface → split post-opaque refraction pass → weave-response deformation → live HDR-overlay accent composite) is now traced as a proper subsection rather than only the dense §5 file-map row and §8 invariants 14–18. **The water *system* now has a dedicated end-to-end trace at [`docs/architecture/water.md`](./water.md)** (truth facade + physics buoyancy + gameplay producers + accents); this render trace covers ONLY the render-side path (`astraweave-render/src/{water.rs, shaders/water.wgsl, renderer.rs}`). Cross-link added; no prior content rewritten. The §5 water row, the §4 corrected `astraweave-fluids` row, and §8 invariants 14–18 are unchanged and remain authoritative.
>
> **Verification stamp — 2026-06-25 (v1.8, commit `7c29b8182`).** §2.6 was re-verified end-to-end against the code with zero corrections. Confirmed: (a) the post-opaque split pass + scene-color refraction tap + depth-foam (`renderer.rs:4679-4768` `run_water_pass`: opaque-color copy `:4708-4726`, read-only-depth attachment `depth_ops: None` `:4752-4756`; 4-entry `{uniform, scene_color, scene_depth, sampler}` BGL `water.rs:330-373`; shader refraction tap `water.wgsl:319-326`, Beer-Lambert thickness `:328-342`, Profile-C depth-delta foam `:358-363`, opaque output `:378-381`); (b) `WaterUniforms` is `#[repr(C)]` (`water.rs:151-152`) and exactly 512 B, asserted by `test_uniforms_size` (`water.rs:874-886`, with offset asserts `inv_view_proj@160`, `weave_count@240`, `weave_instances@256`, `WeaveInstanceRaw==32 B`); (c) the `hdr_overlay` closure field is the pure-`wgpu` `FnMut(&mut CommandEncoder, &TextureView, &TextureView, &Device, &Queue)` at `renderer.rs:743-745`, fired by `fire_hdr_overlay` (`:4778-4788`) on all three `run_water_pass` exit paths (`:4689` no-water, `:4697` no-visible-chunks, `:4767` post-draw), and `run_water_pass` is itself called by both `render()` (`:5279`) and `draw_into()` (`:5932`); (d) there is NO render↔fluids crate edge — `astraweave-render/Cargo.toml` declares no `astraweave-fluids`/`-water`/`-gameplay`/`-weaving` dependency and `astraweave-fluids/Cargo.toml` declares no `astraweave-render` dependency; the accent closure body is registered by the `weaving_playground` binary (`examples/weaving_playground/src/main.rs:561-565`), which depends on both crates and calls `FluidRenderer::render_accents`. Wired-caller line citations also confirmed (`hello_companion/src/visual_demo.rs:720-725`, `veilweaver_demo/src/visual_renderer.rs:625-636`, `weaving_playground/src/main.rs:148-153`, `aw_editor/src/viewport/engine_adapter.rs:3739-3802`).
>
> **Revision notice — 2026-06-28 (v1.9).** D-series doc-truth campaign **Path-B.1** brought 8 stale **test targets** (7 in `astraweave-render` + `astraweave-blend/tests/property_tests`) current with the live API — **test-only, zero library/source change**. The drift was invisible to `cargo check --workspace` (which does not build test targets). See the new §10 "Test-target API-sync" entry and the new §11 open question (real PBR `bloom::BloomConfig` coverage). No §1–§9 pipeline content changed; this is a test-currency stamp, not a capability change.

---

## 1. Executive Summary

**What this system does:**
Rasterizes a 3D scene through a wgpu-25-based pipeline: vertex skinning → shadow cascades → clustered-forward main pass → optional deferred path → post-processing chain (bloom/TAA/SSAO/tonemap) → HDR composite → swapchain. Supports PBR materials (Disney BRDF with optional clearcoat/anisotropy/SSS/sheen/transmission), MegaLights GPU light culling, Lumen-style GI, terrain splatting (32-layer canonical material library), GPU particles, atmosphere/sky, water, weather, decals, and impostor/Nanite LOD. The editor viewport in `tools/aw_editor/src/viewport/` shares this pipeline through an adapter so the editor and runtime render paths converge on a single canonical renderer.

**Why it exists:**
Provides a unified, AAA-parity rendering pipeline shared by runtime examples, the editor viewport, and out-of-tree embedders. Single source of truth for PBR, shadows, IBL, and post-processing — eliminating the prior dual-pipeline drift (FastPreview vs. EnginePBR) that motivated the Fix 27 Unified Pipeline Campaign (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`).

**Where it primarily lives:**
- `astraweave-render/src/renderer.rs` (7,809 LoC) — canonical `Renderer` aggregator; owns surface, device, queue, pipelines, bind groups, and `render()` / `draw_into(...)` entry points
- `astraweave-render/src/frame_graph.rs` (868 LoC) — DAG-based pass orchestration with topological ordering and resource lifetime/aliasing analysis
- `astraweave-render/src/graph.rs` + `graph_adapter.rs` — minimal render-graph scaffolding the frame_graph builds on
- `astraweave-render/src/material*.rs` (5 files, ~3,000 LoC) — material system: `MaterialManager`, `MaterialLibrary`, `MaterialGpuExtended`, bindless system, material loader
- `astraweave-render/src/shader_manager.rs` (391 LoC) + `shader_permutation.rs` + `pipeline_cache.rs` — shader infrastructure
- `astraweave-render/shaders/` (71 WGSL files, ~14K LoC) — shader programs
- `astraweave-materials/src/lib.rs` (single file) — declarative material-graph node enum (foundational, Phase 2)
- `tools/aw_editor/src/viewport/renderer.rs` + `engine_adapter.rs` — editor-side viewport that delegates scene rendering to `astraweave_render::Renderer` and adds editor overlays (grid, gizmos, physics debug, blueprint overlay)

**Status note (read first):**
1. **Active campaign work**: The Editor Multi-Tool Architecture Campaign Sub-phase 3 (Mediator Brush) is the highest-velocity work area. Eight diagnostic rounds (2026-05-04 → 2026-05-08) confirmed and elevated the §7.7 wrapped-component resource identity trap to a structural axiom characterizing the editor↔renderer boundary. Real-Fix.A, .B, .C have landed; .D pending Andrew-gate.
2. **Two render paths coexisted historically** — the FastPreview editor-only path and the EnginePBR path through `astraweave_render::Renderer`. Per `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` and CLAUDE.md, the structural unification phase is complete; the `RenderMode::FastPreview` enum variant remains at `tools/aw_editor/src/viewport/engine_adapter.rs:25-28` as residue.
3. **The terrain material system has its own canonical trace** at `docs/architecture/terrain_materials.md`. Cross-referenced rather than duplicated here.
4. **Scale warning**: 123 source files + 71 WGSL files. This trace gives the navigational map and load-bearing detail; per-subsystem traces (Lumen GI, MegaLights, Nanite, Atmosphere, GPU Particles, Volumetric Fog, IBL, TAA, Vegetation, Water, Decals, Impostor/LOD) are follow-up work.

---

## 2. Authoritative Pipeline

The render system has multiple entry points and parallel sub-pipelines. The three load-bearing flows are:

### 2.1 Runtime per-frame render (`Renderer::render`)

```text
[Caller: example main loop, integration test, or windowed app]
    │
    │ renderer.update_camera(...) + renderer.update_instances(...) (or set_terrain_materials/etc.)
    │ renderer.render()
    ▼
[Stage R0: Surface acquire]
    file: astraweave-render/src/renderer.rs:4708-4720
    role: Pull SurfaceTexture from swapchain; handle Lost/OOM
    │
    ▼
[Stage R1: GPU profiler + staging ring begin_frame]
    file: astraweave-render/src/renderer.rs:4726-4732
    role: Begin the profiler frame. A timestamp slot is allocated when that pass is recorded, not for a skipped cluster or cascade. Advance the per-frame transient allocator.
    │
    ▼
[Stage R2: Clustered light bin]
    file: astraweave-render/src/clustered.rs (CPU bin: bin_lights_cpu),
          astraweave-render/src/clustered_forward.rs (GPU bin pipeline)
    role: Cluster lights into froxel grid; produce offset/index buffers
    │
    ▼
[Stage R3: Shadow cascades (CSM)]
    files: astraweave-render/src/renderer.rs (the LIVE machinery: 2048²×3-layer
           Depth32Float array — near/mid + the L.3.A survey cascade to 3000 m,
           update_cascade_splits PSSM λ-blend + sphere-fitted per-cascade
           matrices, per-cascade UBOs, static `shadow_pipeline` +
           L.3 terrain `terrain-shadow-pipeline` caster draws, shadow_common.wgsl
           receiver sampling)
    NOTE (L.3): shadow_csm.rs (4-cascade `CsmRenderer`, splits 10/50/200/1000),
           shadow_quality.rs (PCSS/Poisson), and shaders/shadow_sampling.wgsl are
           a DORMANT parallel implementation — zero production callers
           (tests/examples only). Do not cite their parameters as the engine's.
    role: Render directional-light depth into a texture_2d_array (one cascade per layer);
          per-cascade uniform buffers prevent the queue.write_buffer race (renderer.rs:752-757).
          L.3/L.3.A: in `draw_into`, terrain-forward chunks ALSO draw into all three
          cascade layers (depth-only, per-cascade AABB culling); in `render()` terrain
          does not draw at all (the terrain forward path itself is draw_into-only —
          recorded parity gap).
    │
    ▼
[Stage R4: Main scene pass (forward+ clustered)]
    file: astraweave-render/src/renderer.rs (main pipeline, inline WGSL at SHADER_SRC line 18+)
    bind groups:
        0: camera UBO
        1: material UBO (legacy single-material; bindless system in material_bindless.rs is the modern path)
        2: shadow uniforms + texture_2d_array + comparison sampler
        3: PBR material textures (albedo / mr / normal)
    role: Rasterize geometry; sample shadow + cluster light list; evaluate Disney BRDF
    │
    ▼
[Stage R5: Post-FX chain]
    files: astraweave-render/src/hdr_pipeline.rs (orchestrator),
           astraweave-render/src/bloom.rs, gtao.rs, taa.rs, auto_exposure.rs,
           astraweave-render/src/advanced_post.rs (TAA / MotionBlur / DoF / ColorGrading)
    role: HDR → tonemap → LDR; bloom / SSAO / SSGI / SSR / DOF / motion blur where enabled
    │
    ▼
[Stage R6: Composite + present]
    file: astraweave-render/src/renderer.rs (post_pipeline + hdr_blit_pipeline at renderer.rs:725-731)
    role: Blit HDR to swapchain (or LDR for editor); submit + present
```

### 2.2 Editor draw into texture (`Renderer::draw_into`)

```text
[Caller: tools/aw_editor/src/viewport/engine_adapter.rs:render_to_texture]
    │
    │ renderer.draw_into(scene_target_view, depth_view, encoder)
    │   depth_view: Option<&wgpu::TextureView>  ← added 2026-05-07 Real-Fix.A (commit 0f569d212)
    ▼
[Same R1-R5 stages as Renderer::render, but writes to caller-provided target]
    file: astraweave-render/src/renderer.rs:5234-…
    role: Editor-side rendering; `depth_view` parameter lets the editor share its own depth target
          (resolves Sub-phase 3 Mediator Brush Mechanism 1 — wrong-texture / different-render-target
           §7.7 instance — per docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md Round-5-Closure)
```

### 2.3 Frame graph build (declarative DAG, parallel scaffolding)

```text
[FrameGraphConfig { width, height, shadow_resolution, cascade_count, feature toggles }]
    │
    ▼
[FrameGraphBuilder declares typed pass nodes]
    file: astraweave-render/src/frame_graph.rs
    role: Declare PassDecl with explicit resource inputs/outputs;
          cluster_bin / shadow / sky / main_scene / tonemap topology (frame_graph.rs:9-15)
    │
    │ RenderGraph::compile() → topological order + resource lifetimes + alias analysis
    ▼
[GraphContext records pass execution]
    file: astraweave-render/src/graph.rs, graph_adapter.rs
    role: Validates resource availability; current pass nodes delegate actual GPU work
          back to Renderer methods (frame_graph.rs:18-25, "Migration Status" comment block)
```

### 2.4 Material upload flows

```text
[A] TOML / RON authoring → material_loader (texture array build) → MaterialManager (GPU arrays + UBO)
    files: astraweave-render/src/material_loader.rs (1,113 LoC; feature-gated on "textures"),
           astraweave-render/src/material.rs:949 (MaterialManager)
    output: MaterialGpu storage buffer, texture arrays bound at group 3

[B] Canonical 32-layer terrain material library (Real-Fix.D 2026-05-08; E3 texturing 2026-07-03)
    files: astraweave-render/src/material_library.rs (MAX_TERRAIN_LAYERS = 32, NUM_SPLAT_MAPS = 8),
           astraweave-render/src/terrain_material.rs (TerrainLayerGpu, TerrainMaterialGpu),
           astraweave-render/src/terrain_material_manager.rs (forward path: draw_chunk_forward;
             TERRAIN_FORWARD_SHADER concat incl. stochastic_tiling.wgsl at :171-176;
             mipped layer arrays + aniso-8 layer sampler — see v1.10 revision notice)
    output: Per-chunk splat textures + shared layer arrays. See docs/architecture/terrain_materials.md

[C] Bindless material system (alternative modern path)
    file: astraweave-render/src/material_bindless.rs
    output: BindlessMaterialSystem; GpuMaterialEntry indexed by material_id

[D] Extended material flags (clearcoat / anisotropy / SSS / sheen / transmission)
    file: astraweave-render/src/material_extended.rs
    flags: MATERIAL_FLAG_CLEARCOAT, MATERIAL_FLAG_ANISOTROPY, MATERIAL_FLAG_SHEEN,
           MATERIAL_FLAG_SUBSURFACE, MATERIAL_FLAG_TRANSMISSION (re-exported at lib.rs:222-226)
```

### 2.5 Shader pipeline / permutation infrastructure

```text
[ShaderManager registers shader files by ShaderKey (string)]
    file: astraweave-render/src/shader_manager.rs:74-…
    role: Path → content_hash → dirty flag; check_for_changes() rescans on a timer
    │
    │ Renderer recreates wgpu::ShaderModule + RenderPipeline when dirty
    ▼
[ShaderPermutation compiles BRDF lobe permutations]
    file: astraweave-render/src/shader_permutation.rs
    role: Compile-time enum of permutations for Disney BRDF lobes
    │
    ▼
[PipelineCache persists wgpu::PipelineCache to disk]
    file: astraweave-render/src/pipeline_cache.rs
    role: Vulkan/DX12 pipeline cache; eliminates cold-start shader-compile stalls;
          held by Renderer as _pipeline_cache_mgr to keep alive for Drop persistence (renderer.rs:701-703)
```

### 2.6 Water surface render path (W-series)

> **Scope.** This subsection traces only the **render-side** water path inside `astraweave-render`. The water *system* as a whole — the `astraweave-water` `WaterQuery` gameplay-truth facade, `astraweave-physics` buoyancy consumer, the `WeaveOp` gameplay source, and the example-layer producers — has its own canonical end-to-end trace at [`docs/architecture/water.md`](./water.md). When a change spans gameplay/physics/render, read that trace first; when a change is render-internal (`water.rs` / `water.wgsl` / `run_water_pass`), this subsection is sufficient.

The water surface renders in a **dedicated raster pass that runs after the opaque main pass closes**, not inside the shared forward pass. This pass-split (W.2b.2) is what lets the surface sample the opaque scene color (for refraction) and the scene depth (for shoreline foam) — neither is sampleable while the main pass still owns them as attachments.

```text
[Per-frame caller: Renderer::render() (runtime) OR Renderer::draw_into() (editor)]
    │
    │ Renderer::update_water(view_proj, camera_pos, time)        → WaterRenderer::update
    │ Renderer::set_water_weave_instances(&[WeaveInstance])      → WaterRenderer::set_weave_instances   (W.2c.3; optional)
    │ Renderer::set_water_level(level) / set_water_colors / set_rain_intensity                          (optional)
    ▼
[W0: WaterRenderer::update — CPU chunk/LOD assignment + uniform upload]
    file: astraweave-render/src/water.rs:673-714
    role: For the (2·GRID_RADIUS+1)²=289-chunk block centred on the camera, assign each
          chunk an LOD band by camera distance (LOD_DISTANCES); write per-LOD instance
          offset buffers; upload WaterUniforms (view_proj, inv_view_proj, time, camera, weaves)
    key data: per-LOD instance_counts + instance_buffers; WaterUniforms (512 B)
    │
    │ … opaque main pass + sky + impostors + weather record FIRST (render()/draw_into()) …
    │   water draw is NO LONGER inside that pass (W.2b.2)
    ▼
[W1: Renderer::run_water_pass — snapshot opaque scene color]
    file: astraweave-render/src/renderer.rs:4679-4768
    role: enc.copy_texture_to_texture(hdr_tex → water_scene_color) AFTER the opaque pass
          closes. Early-returns (no snapshot, no pass) when WaterRenderer::has_visible_chunks()
          is false (editor dormant case) OR no WaterRenderer installed — but still fires the
          HDR overlay (W2c below) on every exit path.
    key data: water_scene_color (full-res Rgba16Float copy of the opaque HDR target)
    │
    ▼
[W2: WaterRenderer::prepare_scene — wire 4-entry bind group + screen size]
    file: astraweave-render/src/water.rs:721-746
    role: Rebuild the bind group (gen-gated on resource_generation) to bind
          {uniform@0, scene_color@1, scene_depth@2, sampler@3}; patch screen_size uniform.
          scene_depth is the SAME depth texture the opaque pass wrote (caller-supplied for
          draw_into(); internal self.depth.view for render()).
    │
    ▼
[W3: WaterRenderer::render — instanced surface draw into the HDR target]
    file: astraweave-render/src/water.rs:823-836 (one draw_indexed per active LOD band)
          shader: astraweave-render/src/shaders/water.wgsl
    pass:  color = hdr_view (LoadOp::Load); depth = same scene depth, depth_ops: None (READ-ONLY)
    vs_main:  world-XZ Gerstner sum (4 waves, Q≤1 cap) → weave deform (W.2c) → world Y from
              water_level uniform; skirt verts drop by skirt_depth to hide LOD-boundary cracks
    fs_main:  Fresnel + depth-tinted body colour + screen-space refraction tap of scene_color
              (distorted by surface normal XZ) + Beer-Lambert absorption by reconstructed
              thickness + Profile-C depth-delta shoreline foam + freeze ice material-state.
              Outputs OPAQUE (alpha=1) — see-through comes from the refraction tap, not blend.
    │
    ▼
[W4: fire_hdr_overlay — post-water, pre-tonemap accent composite (F.4.3)]
    file: astraweave-render/src/renderer.rs:4770-4788
    role: Invoke the registered hdr_overlay closure (if any) AFTER water_renderer is restored,
          against hdr_view + the same scene depth (read-only). The closure body (registered by
          the binary, NOT astraweave-render) draws weave-impact splash/spray accents via
          astraweave-fluids FluidRenderer::render_accents. No-op / zero GPU work when None.
    │
    ▼
[… post-FX chain (R5) + composite/present (R6) as in §2.1/§2.2 …]
```

#### W0 — chunked-LOD surface + uniforms
**File(s):** `astraweave-render/src/water.rs:280-518` (`WaterRenderer` + `new`), `:673-714` (`update`).
**Role:** The surface is a discrete grid of `CHUNK_SIZE = 64.0`-unit square chunks (`water.rs:31`); a `GRID_RADIUS = 8` block (`water.rs:33`, 289 chunks) around the camera is active each frame. Four pre-baked tile meshes (`LOD_SUBDIVS = [32, 16, 8, 4]`, `water.rs:37`) are drawn **instanced — one `draw_indexed` per LOD band** (`water.rs:823-836`), independent of chunk count per band. Each chunk's LOD comes from its camera distance (`LOD_DISTANCES = [110, 220, 360, ∞]`, `water.rs:40`, via `lod_for_distance` `water.rs:664-669`). Wave displacement is a pure function of world XZ, so chunk meshes are world-stable (no swimming) and shared LOD-boundary vertices agree exactly; the only mismatch (curve-vs-chord on a coarse neighbour's edge, ≤ ~1.65-unit total wave amplitude) is covered by **outward-facing per-chunk skirts** that drop `SKIRT_DEPTH = 8.0` (`water.rs:44`, `generate_tile` `water.rs:630-654`). This replaced the former single hardcoded `generate_water_plane(500,128)` plane (W.2a, `docs/campaigns/water-successor/W2A_EXECUTION_REPORT.md`).
**Notes:** World Y comes entirely from `WaterUniforms.water_level` (`water.rs:166`, default `2.0` matching the old baked plane) — there is no baked mesh Y. `WaterUniforms` is `#[repr(C)]` and exactly **512 B** (asserted by `test_uniforms_size`, `water.rs:874-886`): the W.2b refraction block (`inv_view_proj` mat4 at offset 160, `screen_size`, `refraction_strength`, `foam_depth_band`) and the W.2c weave array (8 × 32 B `WeaveInstanceRaw`, 16-aligned at offset 256) were appended without disturbing the std140 alignment.

#### W1/W2 — pass-split, scene-color snapshot, read-only-depth bind group
**File(s):** `astraweave-render/src/renderer.rs:4679-4768` (`run_water_pass`), `:746-749` + `:1313-1329` + `:3963-3978` (`water_scene_color` texture/view + resize), `astraweave-render/src/water.rs:721-746` (`prepare_scene`), `:330-373` (4-entry BGL).
**Role:** After the opaque main pass closes, `run_water_pass` copies `hdr_tex → water_scene_color` (`renderer.rs:4708-4726`) and binds that snapshot plus the scene depth into the water bind group (1→4 entries; `water.rs:330-373`). The water draw uses a **read-only depth attachment** (`depth_ops: None`, `renderer.rs:4752-4756`) so the *same* depth texture is both the depth-test attachment and a sampled binding — a capability validated against wgpu 25 by `examples/depth_sample_capability_probe.rs` (writable-depth + same-texture sampling is a hazard wgpu rejects; read-only + sample is accepted; W.2b.2 Step 0).
**Notes:** The reuse recon (`docs/campaigns/water-successor/W2B1_RECON.md`) explicitly rejected reusing `ssr.rs` (`SsrPass`, dormant, compute/G-buffer-shaped — zero callers) and `frame_graph.rs` (dormant, absent from the live `render()`/`draw_into()` path) for this; the water path is net-new fragment-raster glue. `run_water_pass` `take()`/restores the `WaterRenderer` out of `self` to keep the other fields freely borrowable; the take/restore window is documented as not panic-safe (W-FU-3, robustness-only) in the W.2b.2 report.

#### W3 — surface shading (refraction, depth-foam, weave deform/freeze)
**File(s):** `astraweave-render/src/shaders/water.wgsl` (`vs_main` `:229-293`, `fs_main` `:295-382`).
**Role:** `vs_main` sums 4 Gerstner waves at world XZ (each with a `Q = min(steepness/(freq·amp·WAVE_COUNT), 1.0)` steepness cap, `water.wgsl:146-148`, to prevent crest self-intersection), then applies weave deformation (W3-weave below) AFTER the Gerstner sum so the per-wave Q-cap stays untouched (`water.wgsl:265-282`). `fs_main` computes Fresnel, a depth-tinted body colour, a **screen-space refraction tap** of `scene_color` distorted by the surface-normal XZ (`water.wgsl:319-326`), Beer-Lambert-ish absorption blended by reconstructed water thickness (opaque-scene world pos from `inv_view_proj` + sampled depth, `water.wgsl:328-342`), Profile-C scrolling depth-delta shoreline foam (`water.wgsl:358-363`), and a freeze ice material-state shift (`water.wgsl:368-376`). The fragment outputs **opaque** (`alpha = 1.0`, `water.wgsl:378-381`) — the see-through look comes from the refraction sample, not framebuffer alpha (which would double-count the background under `ALPHA_BLENDING`).
**Notes (W.2c weave-response).** `WaterRenderer` owns a weave-deformation instance list (`WeaveInstance`/`WeaveKind`, ceiling `MAX_WEAVE_INSTANCES = 8`, `water.rs:54`) set via `set_weave_instances`/`clear_weave_instances` (`water.rs:790-806`), riding in `WaterUniforms.weave_instances`. The profiles are **normalized, position-agnostic analytical shapes** — location lives only in the instance; the shader maps `world → local = rotate(world_xz − position, −orientation)/radius` (`weave_accumulate` `water.wgsl:199-227`) and evaluates `weave_profile` (`water.wgsl:188-191`). `Part`/`Raise` are additive ±height (bounded to ±`WEAVE_MAX_DEFORM = SKIRT_DEPTH = 8.0` both CPU-side in `to_raw` `water.rs:106-120` and shader-side net-clamped `water.wgsl:224`, so a deformation can never outrun the skirt and re-expose a seam); `Freeze` is a dual-channel mask (suppresses waves + flattens the normal in `vs_main`, then shifts fragment material-state toward ice in `fs_main`). At zero instances the loop is `weave_count`-gated and the surface is bit-identical to pre-W.2c (identity).

#### W3-weave producers — where the weave instances come from
**File(s):** `astraweave-render/src/renderer.rs:4658-4662` (`set_water_weave_instances`), `examples/weaving_playground/src/weave_producer.rs` (`WaterWeaveProducer`).
**Role:** The gameplay→render seam is **one-way**: `Renderer::set_water_weave_instances` takes only the render-side `crate::water::WeaveInstance` type, so `astraweave-render` has **no** dependency on `astraweave-gameplay`/`astraweave-weaving`. The `WeaveOp → WeaveInstance` translation (`LowerWater → Part`, `RaisePlatform → Raise`, `FreezeWater → Freeze`) lives in **binary glue** (`weave_producer.rs`), reading only `op.a.xz` for position and applying a per-kind default intensity + an age-driven ramp/hold/fade envelope. **This producer currently lives only in the `weaving_playground` example/binary**, not a library crate (see §6 and `water.md` §5/§6). The ops' gameplay-truth behavior is unmodified and runs in parallel (coexist).

#### W4 — live HDR accent composite (F.4.2 machinery / F.4.3 live)
**File(s):** `astraweave-render/src/renderer.rs:735-745` (`hdr_overlay` field), `:4614-4628` (`set_hdr_overlay`), `:4770-4788` (`fire_hdr_overlay`), `examples/weaving_playground/src/{main.rs:561, weave_accent_producer.rs}`.
**Role:** `Renderer` stores an optional `hdr_overlay` closure of pure-`wgpu` type `FnMut(&mut CommandEncoder, &TextureView /*hdr_view*/, &TextureView /*scene_depth*/, &Device, &Queue)` (`renderer.rs:743-745`). `fire_hdr_overlay` invokes it inside `run_water_pass` on **every exit path** (`renderer.rs:4689, 4697, 4767`) — after the water surface draws into `hdr_view` and after `water_renderer` is restored, before tonemap. Because both `render()` (`renderer.rs:5279`) and `draw_into()` (`renderer.rs:5932`) call `run_water_pass`, the *pass mechanism* is automatically available to the editor; the editor producer-feed wiring is the one logged follow-on. The closure body — registered by the binary, never by `astraweave-render` — calls `astraweave-fluids` `FluidRenderer::render_accents` (additive-billboard-only; the SSFR depth/smooth/shade chain is never invoked for accents), so there is **no render↔fluids crate edge**.
**Notes:** This is the canonical "extend an existing system at a typed boundary instead of building a second one" shape — `astraweave-render` and `astraweave-fluids` stay mutually independent (neither `Cargo.toml` references the other). Zero-accent identity holds: `render_accents` early-returns at count 0 and a `None` overlay records zero GPU work → byte-identical frame.

#### Wired vs dormant (Integration Completeness)
The water surface render path is **wired into the runtime**: `Renderer::set_water_renderer` + `WaterRenderer::new` have non-test, non-example production-equivalent callers across `examples/hello_companion` (`visual_demo.rs:720-725`), `examples/veilweaver_demo` (`visual_renderer.rs:625-636`), `examples/weaving_playground` (`main.rs:148-153`), and `tools/aw_editor` (`engine_adapter.rs:3739-3802`) — i.e. the editor and three example binaries all install and drive the surface. (Per CLAUDE.md these in-tree examples + the editor are the canonical runtime consumers of the render crate; there is no non-example library caller because water is a leaf render feature.) The **weave-instance feed** (`set_water_weave_instances`) and the **HDR accent overlay** (`set_hdr_overlay`) are wired only in `weaving_playground` — the producers are example-layer, not a shared library system; the editor receives the *pass mechanism* automatically but not the producer feed. See `water.md` §5/§6 for the full per-layer wired/dormant inventory.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Renderer** | The canonical aggregator: owns wgpu Device / Queue / Surface + all pipelines + all bind groups. Single Rust struct, ~7,800 LoC. | `astraweave-render/src/renderer.rs:691-…` |
| **`render()`** | Per-frame top-level entry: acquires swapchain texture and runs the full pipeline. | `renderer.rs:4708` |
| **`draw_into(view, depth_view, enc)`** | Editor entry: render into a caller-supplied texture view using a caller-supplied depth view (added 2026-05-07 per Round-5-Closure). | `renderer.rs:5234` |
| **Frame graph** | DAG-based declarative pass orchestration. Each `PassDecl` has explicit resource I/O; `RenderGraph::compile()` produces topological order + resource lifetime + alias analysis. Currently scaffolded — pass nodes delegate GPU recording back to `Renderer`. | `astraweave-render/src/frame_graph.rs`, `graph.rs`, `graph_adapter.rs` |
| **Clustered forward** | Lights binned into a 3D froxel grid; fragment shader samples the cluster offsets/indices buffer. | `clustered.rs`, `clustered_forward.rs`, `clustered_megalights.rs` |
| **MegaLights** | GPU-accelerated light culling via compute-shader prefix-sum and write-indices passes. | `clustered_megalights.rs`, `shaders/megalights/*.wgsl` |
| **CSM (Cascaded Shadow Maps)** | Directional-light shadow rendering with cascaded depth slices. The LIVE implementation is `renderer.rs`'s **3-cascade** 2048² array (L.3.A): near/mid fits PSSM λ-blended against 500 (editor terrain scene: ≈86.1 m / 500 m — unchanged from L.3) + a survey cascade 500→3000 m with a wide far fade (2100→3000); sphere-fitted, rotation-stable. Since L.3 terrain casts (depth-only chunk pipeline, all cascades, AABB-culled) and receives (shared `shadow_common.wgsl` `csm_shadow_factor`, direct-sun term only). `shadow_csm.rs`/`shadow_quality.rs`/`shaders/shadow_sampling.wgsl` are a dormant parallel implementation (zero production callers). | `renderer.rs`, `shaders/shadow_common.wgsl`, `terrain_material_manager.rs` (dormant: `shadow_csm.rs`, `shadow_quality.rs`, `shaders/shadow_sampling.wgsl`) |
| **Lumen GI** | Lumen-style global illumination orchestrator (distance-field AO + surface cache + final gather). | `lumen.rs`, `distance_field.rs`, `surface_cache.rs`, `final_gather.rs`, `shaders/lumen/*.wgsl` |
| **HDR pipeline** | The HDR → tonemap → LDR chain, with bloom / GTAO / SSGI / SSR / DOF / TAA / motion blur / color grading composed in sequence. | `hdr_pipeline.rs` (orchestrator), `bloom.rs`, `gtao.rs`, `taa.rs`, `auto_exposure.rs`, `advanced_post.rs` |
| **Material library** | The canonical 32-layer terrain material registry. UI-side `MATERIAL_NAMES: [&str; 22]` was earlier capacity; Real-Fix.D bumped to 32. | `material_library.rs` (`MAX_TERRAIN_LAYERS = 32`, `NUM_SPLAT_MAPS = 8`) |
| **MaterialGpu / MaterialGpuExtended** | The GPU representation of a material entry (texture indices, factors, flags). Extended adds clearcoat / anisotropy / SSS / sheen / transmission. | `material.rs:6-43`, `material_extended.rs` |
| **MaterialManager** | Owns the GPU material arrays + UBO uploaded from TOML/RON. | `material.rs:949-…` (`pub struct MaterialManager`) |
| **BindlessMaterialSystem** | Alternative material binding path using bindless texture arrays + a `GpuMaterialEntry` storage buffer. | `material_bindless.rs` |
| **ShaderManager** | Hash-tracked shader source registry with dirty-flag for hot-reload. Does NOT own GPU resources. | `shader_manager.rs:74-…` |
| **PipelineCache** | Disk-backed wgpu pipeline cache (Vulkan/DX12), bound to Renderer's lifetime via the `_pipeline_cache_mgr` field. | `pipeline_cache.rs`, held at `renderer.rs:701-703` |
| **Frame graph node** | A `RenderNode` implementation that owns resource declarations and a record-callback. | `graph.rs`, `frame_graph.rs:78-…` |
| **Resource generation** | Monotonic counter bumped on resize/resource invalidation; drives `CachedBindGroup` rebuild decisions. | `renderer.rs:708-710`, `bind_group_cache.rs` |
| **Staging ring** | Per-frame ring buffer for transient GPU uniforms / storage allocations. | `staging_ring.rs`, used at `renderer.rs:707, 4732, 5246` |
| **GpuProfiler** | Per-pass GPU timestamp profiler. None when `TIMESTAMP_QUERY` is unsupported or the timestamp period is not positive. Slots are allocated at pass begin. `resolve_queries` runs on the write encoder. The staging copy is a later submission. | `gpu_profiler.rs` |
| **RenderMode** | Editor-side enum for switching between `EnginePBR` (default, full PBR through `Renderer`) and `FastPreview` (legacy cube placeholders). Residue from Fix 27 dual-pipeline era. | `tools/aw_editor/src/viewport/engine_adapter.rs:25-28` |
| **EngineRenderAdapter** | Editor-side adapter that wraps `Renderer` and bridges editor scene data into the render pipeline. | `tools/aw_editor/src/viewport/engine_adapter.rs` |
| **ViewportRenderer** | Editor-side multi-pass coordinator. Delegates scene rendering to `EngineRenderAdapter` (which delegates to `Renderer`); owns editor overlays (grid, entity, gizmo, physics debug). | `tools/aw_editor/src/viewport/renderer.rs:60-…` |
| **WaterRenderer** | The render-side water surface owner: chunked-LOD Gerstner heightfield + refraction + depth-foam + weave-response deformation. Held by `Renderer` as `Option<water_renderer>`; installed via `Renderer::set_water_renderer`. NOT the gameplay-truth facade (that is `astraweave-water`'s `WaterQuery` — see [`water.md`](./water.md)). | `astraweave-render/src/water.rs:280-837` |
| **`run_water_pass`** | The split post-opaque water raster pass: snapshots opaque scene color, binds it + read-only scene depth, draws the water surface into the HDR target, then fires the HDR overlay. Called by both `render()` and `draw_into()`. | `renderer.rs:4679-4768` |
| **WaterUniforms** | The 512-B `#[repr(C)]` std140 uniform block (view/inv-view-proj, colours, water level, refraction params, and the 8-slot weave array at offset 256). Size asserted by `test_uniforms_size`. | `water.rs:150-217` |
| **WeaveInstance / WeaveKind** | A render-side, position-agnostic weave-deformation instance (`None`/`Part`/`Raise`/`Freeze`) applied in `water.wgsl` `vs_main` after the Gerstner sum; ceiling `MAX_WEAVE_INSTANCES = 8`. Set via `Renderer::set_water_weave_instances` (a render type — no gameplay dependency). | `water.rs:54-148`, `water.wgsl:25-227` |
| **`hdr_overlay` / `set_hdr_overlay` / `fire_hdr_overlay`** | The F.4.3 post-water, pre-tonemap HDR composite hook: a pure-`wgpu` `FnMut` closure stored on `Renderer`, fired inside `run_water_pass` on every exit. The binary registers a closure that draws fluid accents via `FluidRenderer::render_accents`, keeping render↔fluids crate-independent. | `renderer.rs:735-745, 4614-4628, 4770-4788` |
| **§7.7 wrapped-component resource identity trap** | The canonical architectural anti-pattern surfaced by the Editor Multi-Tool Architecture Campaign Sub-phase 3. When component A wraps component B and both manage resources of the same logical type, reads from A's resource don't reflect writes to B's resource. Per the campaign-doc Status header at `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md`, this is now a **structural axiom** with four-layer evidence: depth-target (Round 5), mesh-data (Round 6), texture-data attribute-set (Round 7), UI-vs-renderer capacity boundary (Round 8). | Editor↔renderer boundary across `engine_adapter.rs`, `renderer.rs`, `terrain_material_manager.rs`, `terrain_splat_builder.rs` |

### Terms to NOT confuse

- **`Renderer::render` vs `Renderer::draw_into`**: `render()` (`renderer.rs:4708`) acquires the surface texture and presents to the swapchain; `draw_into(view, depth_view, enc)` (`renderer.rs:5234`) writes to a caller-supplied texture view without presenting and accepts an optional depth view. The editor uses `draw_into`; runtime examples use `render`.
- **`MaterialManager` vs `MaterialLibrary` vs `BindlessMaterialSystem`**: Three different material binding paths coexist. `MaterialManager` (`material.rs:949`) is the TOML-driven legacy path. `MaterialLibrary` (`material_library.rs`) is the canonical 32-layer **terrain** material registry. `BindlessMaterialSystem` (`material_bindless.rs`) is the bindless modern path. Section 6 documents the coexistence.
- **`types::Material` vs `material_library::Material`**: Re-exported at `lib.rs:147` (`types::Material`) but the inner `material_library::Material` is **intentionally NOT re-exported** at the crate root to avoid the collision (see `lib.rs:251-253` for the explicit no-export comment).
- **Frame graph vs render pipeline**: The frame graph is declarative DAG scaffolding (`frame_graph.rs`); the actual GPU work still flows through `Renderer` methods. Per `frame_graph.rs:18-25`, "full delegation is designed for incremental adoption" — the frame graph today validates topology but does not yet drive command recording.
- **Editor render path vs runtime render path**: Both call into `astraweave_render::Renderer` (post Fix 27 structural completion). The remaining divergence lives in the editor's `EngineRenderAdapter` (`tools/aw_editor/src/viewport/engine_adapter.rs`) which manages editor-specific scene data and the FastPreview residue. See §6.
- **Shadow uniforms (single buffer) vs per-cascade buffers**: The renderer uses **per-cascade uniform buffers** (`renderer.rs:752-757`) explicitly to avoid `queue.write_buffer` race conditions where all writes resolve before the command buffer executes, causing both shadow passes to read the same (last-written) cascade matrix. This is a deliberate non-obvious choice — do not "simplify" to a single shadow uniform buffer.
- **`WaterRenderer` (render-side surface) vs `WaterQuery`/`AnalyticWater` (gameplay truth)**: `astraweave-render::WaterRenderer` (`water.rs`) is the *presentation* — the visible Gerstner surface, refraction, and weave deformation. `astraweave-water::WaterQuery` (the `AnalyticWater` backend) is the *gameplay-truth facade* (is-there-water / height / density), consumed by `astraweave-physics` buoyancy. They are intentionally separate layers in separate crates; `astraweave-render` has **no** dependency on `astraweave-water`. The weave instances the surface deforms by are a render type (`WeaveInstance`) translated from gameplay `WeaveOp`s by binary-layer glue, not a shared library. Full layering in [`water.md`](./water.md).

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `wgpu` 25.x | Device / Queue / Surface APIs | GPU command submission | `Renderer::new_from_device` is the canonical constructor; CLAUDE.md ARCHITECTURE_REFERENCE.md lists key methods |
| `astraweave-math` | SIMD vec/mat/quat ops | Transform math (CPU side) | Auto-used by `glam` re-exports |
| `astraweave-asset` + `astraweave-asset-pipeline` | Asset index, texture/mesh loading | Material TOMLs, glTF, OBJ | `material_loader.rs` consumes asset paths; gated on `textures` feature |
| `astraweave-cinematics` | Cinematic camera frames | Camera transforms | `renderer.rs:15` `use astraweave_cinematics as awc;` |
| `astraweave-materials` | `Node` material-graph enum | Authored material graph data | `astraweave-materials/src/lib.rs:8-19` — currently used as foundation, not yet wired into runtime material instances. `renderer.rs:16` `use astraweave_materials::MaterialPackage;` |
| `astraweave-terrain` | `WorldGenerator`, `TerrainChunk`, `Heightmap` | Heightmap geometry + biome assignment | Consumed by `astraweave-render/src/terrain.rs` (legacy single-`biome_id` path) and the canonical `terrain_material_manager` (32-layer path, see `docs/architecture/terrain_materials.md`) |
| `astraweave-scene` | Scene partition data | Scene graph + streaming | Used by streaming-aware rendering paths |
| ~~`astraweave-fluids`~~ | — | — | **Row corrected F.1 (2026-06-11): the previous entry ("SPH water particle data → water.rs") was aspirational.** `astraweave-render` has no Cargo dependency on and zero imports from `astraweave-fluids`; `water.rs` is a self-contained Gerstner-wave renderer. SPH render integration is future Fluids-Integration campaign work (seam mapped in `docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md` §Seam 4). |
| `astraweave-ecs` | `Entity`, `World`, components | Scene data source | Editor and example mains pull entity data and feed instance buffers |
| `tools/aw_asset_cli` | Cooked asset blobs | Texture / mesh runtime assets | Build-time pipeline (cross-references in CLAUDE.md note this is a `tools/` dep) |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `examples/hello_companion`, `unified_showcase`, `ecs_ai_showcase`, `weaving_playground`, `biome_weather_demo`, `phi3_demo`, ~20 example crates | `Renderer::render()` + `Renderer::update_camera/update_instances/etc.` | Final frame on swapchain | Examples are the primary in-tree runtime consumers |
| `tools/aw_editor` | `Renderer::draw_into(view, depth_view, enc)` via `EngineRenderAdapter::render_to_texture` | HDR scene + LDR composite for egui display | The editor is the most demanding in-tree consumer and the active campaign target |
| `astraweave-render/src/bin/aw_impostor_bake.rs` | `impostor_bake` module | Pre-baked impostor atlases (offline) | Feature-gated on `impostor-bake` |
| `astraweave-stress-test`, benchmarks | `Renderer` + headless surface | Performance measurement | Stress-test crates rely on the public Renderer API |
| Out-of-tree embedders via `astraweave-sdk` | None directly — SDK wraps the legacy World, not the renderer | n/a | The SDK does not currently expose render APIs; renderer is in-process only |

### Bidirectional / Coupled

- **Editor ViewportRenderer ↔ Renderer**: The editor passes its own depth view into `Renderer::draw_into` (added 2026-05-07 Real-Fix.A `0f569d212`) so editor overlay passes (grid, gizmos, physics debug, blueprint overlay) and engine terrain depth-write into the same depth target. This is the canonical fix shape for §7.7 wrapped-component resource identity trap at the depth-target layer. Source: `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Round-5-Closure.
- **Terrain material upload coupling**: The editor's brush emits `TerrainAction` events that flow into `engine_adapter::upload_terrain_chunks` (initial path) **or** `engine_adapter::update_terrain_chunk` (incremental path). Real-Fix.B (`eaaa53433`, 2026-05-07) extracted `upload_or_update_terrain_chunk_forward` so both paths route to the same `Renderer::terrain_forward.chunks` HashMap (mesh-data layer §7.7 instance per Round-6-Closure).
- **Shader hot-reload ↔ Pipeline rebuild**: `ShaderManager::check_for_changes()` flips dirty flags; `Renderer` is responsible for recreating `wgpu::ShaderModule` + `RenderPipeline` (`shader_manager.rs:69-73` documents this split).
- **`bind_group_cache` ↔ `Renderer.resource_generation`**: The monotonic generation counter at `renderer.rs:708-710` is bumped on resize / resource invalidation; `CachedBindGroup` rebuilds against the new generation (`bind_group_cache.rs`).

---

## 5. Active File Map

This map enumerates the load-bearing files. Per-file traces of every subsystem are out of scope here — see metadata note on follow-up work. Subsystems are grouped functionally.

### Core aggregator / orchestration

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/renderer.rs` | Canonical `Renderer` aggregator: device/queue/surface ownership + all pipelines + `render()` / `draw_into()` entry points | Active | 7,809 LoC. Single-largest source file in the crate. Inline WGSL `SHADER_SRC` at lines 18-… for the main forward+ pipeline |
| `astraweave-render/src/lib.rs` | Module declarations + `pub use` re-exports | Active | Module declarations and the public re-export surface. Linux lib tests also pull in `serialize_lib_tests.rs`. |
| `astraweave-render/src/serialize_lib_tests.rs` | Linux `.init_array` that defaults `RUST_TEST_THREADS` to 1 | Test-only (Linux) | Stops the libvulkan 1.4.341 SIGSEGV when every Mesa ICD is loaded (#434). Lib tests compile it under `cfg(test)`. Integration tests include it from `tests/support/linux_test_threads.rs`. Does not run in the production library. |
| `astraweave-render/tests/instance_debug_flags.rs` | `WGPU_DEBUG` vs `InstanceDescriptor::default()` | Test-only | The parent spawns a child with `Command::env`. The parent does not call `set_var`. On Linux the harness includes `linux_test_threads.rs`. |
| `astraweave-render/tests/support/gpu_capability.rs` | Shared headless probe and skip log | Test-only | `prefer_fallback` tries software, then hardware. With `AW_GPU_SKIP_IF_UNAVAILABLE` set it stays on the software adapter. A device refusal panics even when that variable is set. The caller name is taken before the first await. The append uses `libc::O_NOFOLLOW`. The required-name test includes `test_backend_detection`, `golden_triangle_render`, and `golden_determinism_check`. |
| `astraweave-render/src/frame_graph.rs` | Frame render graph: DAG builder + pass nodes for cluster_bin / shadow / sky / main_scene / tonemap | Active (scaffolded; not yet command-driving per `frame_graph.rs:18-25`) | 868 LoC. Topology validation works; pass nodes delegate GPU recording back to `Renderer` |
| `astraweave-render/src/graph.rs`, `graph_adapter.rs` | Minimal render-graph scaffolding (`RenderGraph`, `RenderNode`, `PassDecl`, `GraphContext`) | Active | Foundation for `frame_graph.rs` |
| `astraweave-render/src/error.rs` | `RenderError`, `RenderResult` typed errors | Active | Re-exported at `lib.rs:143` |
| `astraweave-render/src/pipeline_cache.rs` | Disk-backed pipeline cache | Active | `_pipeline_cache_mgr` held by Renderer for Drop persistence |
| `astraweave-render/src/bind_group_cache.rs` | Generation-tracked bind-group cache | Active | Driven by `Renderer.resource_generation` |
| `astraweave-render/src/staging_ring.rs` | Per-frame transient GPU allocator | Active | `Renderer.staging_ring` field |
| `astraweave-render/src/gpu_memory.rs` | Memory budget tracker | Active | `GpuMemoryBudget`, `MemoryCategory` |
| `astraweave-render/src/gpu_profiler.rs` | Per-pass GPU timestamp profiler | Active (optional — None when `TIMESTAMP_QUERY` is unsupported or the period is not positive) | Resolve is on the write encoder. The staging copy is a later submission. The log prints encoder ids and `encoder_count`. `prove_readback` must read a non-zero tick. `tests/gpu_timestamp.rs` |
| `astraweave-render/src/budget_gate.rs` | Gated-flag parser for `gpu_frame_budget` | Active | `flag_on` treats `--name=false` and `--name false` as off. `reject_gated_flags` accepts `--atmosphere` alone. The bin calls that function. Lib tests are the list. |
| `astraweave-render/tests/frame_alloc.rs` | Warm `draw_into` allocation floor | Active (`alloc-counter`) | Six `allocs()` samples. Gate is min 110, max 130, spread 4, so an 18-alloc pass drop on the `--all-features` low sample of 121 fails. Adapter must be llvmpipe. On Linux the harness includes `linux_test_threads.rs`. `RUST_LIB_BACKTRACE=0` is read from the process environment. |

### Material system

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/material.rs` | `MaterialGpu`, `MaterialLayerDesc`, `MaterialPackDesc`, `MaterialManager`, `ArrayLayout` | Active | 949 LoC; flags at lines 25-31 |
| `astraweave-render/src/material_library.rs` | Canonical 32-layer terrain material registry (`MAX_TERRAIN_LAYERS = 32`, `NUM_SPLAT_MAPS = 8`) | Active | Real-Fix.D 2026-05-08 — see `terrain_materials.md` |
| `astraweave-render/src/material_loader.rs` | TOML-driven texture array build pipeline (feature `textures`) | Active | 1,113 LoC; gated on `textures` feature flag |
| `astraweave-render/src/material_bindless.rs` | `BindlessMaterialSystem`, `GpuMaterialEntry` — bindless texture-array material path | Active | 585 LoC; alternative modern path to `MaterialManager` |
| `astraweave-render/src/material_extended.rs` | Extended PBR materials (clearcoat, anisotropy, SSS, sheen, transmission); flags at `material_extended.rs:lib.rs:222-226` re-export | Active | Phase PBR-E |
| `astraweave-render/src/disney_material.rs` | Disney principled BRDF evaluation + WGSL source export | Active | `evaluate_disney_brdf`, `BRDF_LUT_WGSL`, `DISNEY_BRDF_WGSL` exports |
| `astraweave-render/src/brdf_lut.rs` | Split-sum BRDF integration LUT for IBL (Phase 9) | Active | `BrdfLutPass`, `BrdfLutConfig` |
| `astraweave-render/src/parallax.rs` | Parallax Occlusion Mapping (steep ray-march + binary refinement) | Active | `PomConfig` |
| `astraweave-render/src/terrain_material.rs` | `TerrainLayerGpu`, `TerrainMaterialGpu`, `TerrainLayerDesc`, `TerrainMaterialDesc` — GPU schema | Active | 32-layer; see `docs/architecture/terrain_materials.md` |
| `astraweave-render/src/terrain_material_manager.rs` | 32-layer splat-array terrain pipeline (forward path) | Active (`#[cfg(feature = "terrain-splat-arrays")]` per `lib.rs:108-109, 260-264`) | Per `docs/architecture/terrain_materials.md` |
| `astraweave-render/shaders/pbr_terrain.wgsl` | Splat fragment half. `height_blend` takes and returns `array<f32, 32>` | Active | Concatenated with `pbr_terrain_vs.wgsl` as `TERRAIN_SPLAT_SHADER`. `tests/shader_validation.rs` validates that pair |
| `astraweave-render/src/biome_material.rs` | `BiomeMaterialSystem`, `BiomeMaterialConfig` | Active | Biome-aware material selection |
| `astraweave-render/src/biome_detector.rs` | `BiomeDetector` runtime biome classification | Active | Companion to `biome_material.rs` |
| `astraweave-render/src/biome_transition.rs` | `BiomeVisuals`, `TransitionConfig`, easing functions | Active | Cross-biome blend logic |
| `astraweave-render/src/biome_audio.rs` | Audio cues tied to biome transitions | Active | Cross-system into audio crate |
| `astraweave-materials/src/lib.rs` | Single-file declarative material-graph node enum (`Node`) | Active | Phase 2 foundation; `renderer.rs:16` uses `MaterialPackage` from this crate |

### Lighting

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/clustered.rs` | CPU cluster light binning (`bin_lights_cpu`, `ClusterDims`, `CpuLight`); WGSL placeholder `WGSL_CLUSTER_BIN` | Active | Loaded by `renderer.rs:11` |
| `astraweave-render/src/clustered_forward.rs` | Complete clustered forward rendering | Active | |
| `astraweave-render/src/clustered_megalights.rs` | MegaLights GPU light culling (Phase 1) | Active | Shaders at `shaders/megalights/*.wgsl` |
| `astraweave-render/src/shadow_csm.rs` | Cascaded Shadow Mapping (Phase 2) | **Dormant** | L.3 verified: zero production callers (tests/examples only); a PARALLEL implementation of the live CSM in `renderer.rs` (4 cascades, splits 10/50/200/1000 — do not cite as the engine's). Drift-cleanup residue, L3_OUTCOME §9. |
| `astraweave-render/src/shadow_point.rs` | Point/spot light shadow maps with priority selection | Dormant | Declared in lib.rs; no renderer wiring found (L.3 recon). |
| `astraweave-render/src/shadow_quality.rs` | PCSS, Poisson PCF, cascade stabilization, normal-offset bias | **Dormant** | L.3 verified: no live consumer; `shaders/shadow_sampling.wgsl` never enters a pipeline (parsed only by its own test). L.3 consumed the live inline PCF (hoisted to `shaders/shadow_common.wgsl`) instead. |
| `astraweave-render/shaders/shadow_common.wgsl` | **L.3 (NEW):** shared `MainLightUbo` + `csm_shadow_factor` (cascade select + PCF 3×3 + boundary fades) — concatenation fragment consumed by SHADER_SRC (static, group(2)) and TERRAIN_FORWARD_SHADER (group(4)) | Active | The single CSM sampling implementation; textures/samplers passed as function params, consumers declare their own globals (ibl_common contract). |
| `astraweave-render/src/ltc_area_lights.rs` | LTC area lights (rectangular, disk, tube) — Heitz et al. 2016 | Active | `AreaLightManager`, `AreaLightType`, `GpuAreaLight` |
| `astraweave-render/src/ibl.rs` | Image-based lighting manager | Active | `IblManager`, `IblQuality`, `IblResources`, `SkyMode`. Mip 0 prefilter is one sample. Each bake submission is polled to completion. |
| `astraweave-render/src/lumen.rs` + `distance_field.rs` + `surface_cache.rs` + `final_gather.rs` | Lumen GI orchestrator (Phase 5) | Active | Distance-field AO + surface cache + multi-bounce final gather |
| `astraweave-render/src/gi/mod.rs`, `vxgi.rs`, `voxelization_pipeline.rs` | VXGI global illumination | Active | Alternative GI path to Lumen |
| `astraweave-render/src/ssgi.rs` | Screen-Space Global Illumination (temporal denoise) | Active | |
| `astraweave-render/src/ssr.rs` | Screen-Space Reflections (Hi-Z ray marching) | Active | |
| `astraweave-render/src/hiz_pyramid.rs` | Shared Hi-Z min-depth pyramid | Gated | `draw_into` builds it only when `sample_hiz` is set. Enabling that drops any earlier pyramid and the next frame copies the depth view from that call. Occlusion, SSR, and SSGI do not sample it. Copy is `texture_depth_2d` → R32Float (`hiz_pyramid_copy.wgsl`); downsample is `textureLoad` on that copy |
| `astraweave-render/src/gtao.rs` | Ground Truth Ambient Occlusion (visibility bitmask) | Active | |
| `astraweave-render/src/atmosphere.rs` | Bruneton physically-based atmosphere (Phase 8) | Active | Shaders at `shaders/atmosphere/*.wgsl` |
| `astraweave-render/src/god_rays.rs` | Screen-space god rays / crepuscular shafts (Phase 6). v1.22: composite is sampled scene + write-only rgba16float, then a copy onto HDR | Active | `sun_to_screen`, `GodRayPass` |
| `astraweave-render/src/volumetric_fog.rs` | Froxel-based volumetric fog + light scattering (Phase 6). v1.21: optional 32³ scar volume sampled in the density pass. v1.22: integrate samples `texture_depth_2d` | Active | Scar upload is `upload_scar_volume`; timestamps via `poll_timestamps`. `screen_uv_to_ndc` is inlined in `shaders/volumetrics/fog_density.wgsl` and `scatter.wgsl`. `shaders/screen_ndc.wgsl` is deleted. |
| `astraweave-render/src/gpu_fault.rs` | First wgpu validation error for the process | Active | `scope` / `record`; logged as `FIRST wgpu error` |
| `astraweave-render/src/volumetric_clouds.rs` | Perlin-Worley volumetric cloud raymarching (Phase 3). Cloud shadows: 512² rgba16float, 16-step march, 32 row-slices per cycle keyed by the presentation frame (`CLOUD_SHADOW_STEPS`, `CLOUD_SHADOW_SLICES`). Darkness is `CLOUD_SHADOW_STRENGTH` (default 4) on the extinction | Active | `shaders/volumetrics/cloud_shadow_map.wgsl`. Bake center is `IblParams._pad`. `cloud_pass_creation` records a skip or panics when depth `textureLoad` is absent. |
| `astraweave-render/src/scene_environment.rs` | Scene-env UBO (fog/ambient/tint/sun); WGSL shared snippets `WGSL_FOG_FUNCTIONS`, `WGSL_SCENE_ENVIRONMENT` | Active | |

### Post-processing

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/hdr_pipeline.rs` | HDR pipeline orchestration: tonemap + color grading + post-FX chain | Active | 776 LoC |
| `astraweave-render/src/bloom.rs` | Physically-based bloom (13-tap downsample + tent upsample) | Active | |
| `astraweave-render/src/auto_exposure.rs` | Luminance-histogram auto-exposure with temporal adaptation | Active | Has subgroup-optimized variant in `subgroup_ops.rs` |
| `astraweave-render/src/taa.rs` | Temporal Anti-Aliasing (neighborhood clamping + RCAS sharpening) | Active | |
| `astraweave-render/src/temporal_upscale.rs` | TAA-U: reduced internal-res render with native-res resolve | Active | `upscale_pass_creation` records a skip or panics when depth `textureLoad` is absent. |
| `astraweave-render/src/velocity.rs` | Motion vector / velocity buffer | Active | Feeds TAA, motion blur, TSR |
| `astraweave-render/src/advanced_post.rs` | TAA + motion blur + DOF + color grading | Active (feature `advanced-post`) | `AdvancedPostFx`, `ColorGradingConfig`, `DofConfig`, `MotionBlurConfig` |
| `astraweave-render/src/post.rs` | Compile-only WGSL placeholders + tests; `WGSL_SSAO`, `WGSL_SSGI`, `WGSL_SSR` referenced from `renderer.rs:1-2` under `feature = "postfx"` | Active | |
| `astraweave-render/src/oit.rs` | Weighted Blended Order-Independent Transparency | Active | `OitBuffers`, `WboitRenderer` |
| `astraweave-render/src/transparency.rs` | Transparency depth sorting and render pass | Active | |
| `astraweave-render/src/msaa.rs` | MSAA anti-aliasing resources | Active | `MsaaMode`, `MsaaRenderTarget` |
| `astraweave-render/src/effects.rs` | `WeatherFx`, `WeatherKind` weather-effect overlay | Active | |
| `astraweave-render/src/overlay.rs` | Cutscene-fade / letterbox overlay | Active | |

### Geometry, culling, LOD

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/mesh.rs` | `CpuMesh`, `MeshVertex`, `MeshVertexLayout` | Active | |
| `astraweave-render/src/mesh_registry.rs` | `MeshRegistry`, `MeshHandle`, `MeshKey` | Active | Mesh lookup by stable key |
| `astraweave-render/src/mesh_gltf.rs`, `mesh_obj.rs` | glTF and OBJ loaders | Active (feature-gated on `assets` / `gltf-assets` / `obj-assets`) | `load_gltf` rejects buffer and image file URIs that escape the glTF directory via `safe_under` before `gltf::import`. `data:` URIs stay in memory. |
| `astraweave-render/src/mesh_gltf.rs`, `mesh_obj.rs`, `pbr_maps.rs` | glTF and OBJ loaders. glTF reads base color, normal, MR, occlusion, and emissive. `attach_sidecar_pbr_maps` fills empty normal/MR/occlusion from sibling `nor_gl` and `_arm` files. `pack_gltf_surface` is what `add_composite_model` uploads. | Active (`gltf-assets` / `obj-assets`; sidecar image load also needs `textures`) | |
| `astraweave-render/src/primitives.rs` | Primitive mesh generation | Active | |
| `astraweave-render/src/instancing.rs` | GPU instancing for draw call reduction | Active | |
| `astraweave-render/src/vertex_compression.rs` | Vertex compression (octahedral normals, half-float UVs); 37.5% memory reduction per `lib.rs:14` | Active | Week 5 Action 19 |
| `astraweave-render/src/lod_generator.rs` | LOD chain generation (quadric error metrics) | Active | Week 5 Action 19 |
| `astraweave-render/src/culling.rs` + `culling_node.rs` | GPU-driven frustum culling (Phase 2 Task 3) | Active | `FrustumPlanes` is the `draw_into` filter while `set_gpu_cull_enabled` is off (the default). When on, `draw_into` dispatches `IndirectDrawPipeline` and binds each model's group 3 before `draw_indexed_indirect`. `CullingPipeline` is not dispatched |
| `astraweave-render/src/nanite_gpu_culling.rs`, `nanite_render.rs`, `nanite_visibility.rs` | Nanite virtualized geometry | Active (feature `nanite`) | Per `lib.rs:131-137` |
| `astraweave-render/src/impostor_bake.rs`, `impostor_lod3.rs`, `impostor_pass.rs` | Phase 5.3 impostor atlas + LOD3 live-draw + reusable draw helper | Active (feature `impostor-bake`) | Per `lib.rs:159-164` |
| `astraweave-render/src/vegetation_gpu.rs` | GPU-instanced vegetation scatter + frustum cull | Active | |
| `astraweave-render/src/vegetation_interaction.rs` | Entity-proximity grass-bending stamp | Active | |
| `astraweave-render/src/vegetation_lod.rs` | Tree LOD chain with billboard/impostor support | Active | |
| `astraweave-render/src/grass_blade.rs` | Per-blade procedural grass geometry | Active | |
| `astraweave-render/src/stochastic_tiling.rs` | Hex-tile stochastic sampling (anti-tiling for terrain textures); exports `STOCHASTIC_TILING_WGSL` (`:30`) | Active — **consumed since E3** | `shaders/stochastic_tiling.wgsl` is concat'd into `TERRAIN_FORWARD_SHADER` (`terrain_material_manager.rs:171-176`) and its `hex_cells()`/`rotate_dir()` drive the per-layer 3-tap rotated sampling in `pbr_terrain_forward.wgsl` (pre-E3 this module was zero-consumer scaffolding) |

### Animation

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/animation.rs` | `Skeleton`, `AnimationClip`, `AnimationState`, `Joint`, `JointPalette`, `JointMatrixGPU`, `MAX_JOINTS`, `compute_joint_matrices`, `skin_vertex_cpu` | Active | Phase 2 Task 5 |
| `astraweave-render/src/skinning_gpu.rs` | GPU skinning pipeline (`JointPaletteManager`, `JointPaletteHandle`, `SKINNING_GPU_SHADER`) | Active (feature `skinning-gpu`) | Phase 2 Task 5 Phase D |

### Particles + weather

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/gpu_particles.rs` | GPU compute-based particle system | Active, presentation-only | `draw_into` calls `update` only after `Renderer::set_particle_emitter`. Emit fills dead slots. Seed and `dt` come from the caller |
| `astraweave-render/src/particle_forces.rs` | Enhanced particle simulation (forces, curves, emission shapes) | Active | Phase 7 |
| `astraweave-render/src/particle_render.rs` | Billboard particle render pipeline with blending | Active, presentation-only | Depth binding is `texture_depth_2d` + `textureLoad`. Drawn from `draw_into` when an emitter is set |
| `astraweave-render/src/particle_sort.rs` | GPU bitonic sort for depth-ordered transparency | Active | Phase 7 |
| `astraweave-render/src/weather_system.rs` | Weather system orchestrator | Active | 1,083 LoC |
| `astraweave-render/src/weather_gpu.rs` | GPU weather particle emitter configs | Active | |
| `astraweave-render/src/rain_occlusion.rs` | GPU rain/weather particle occlusion via depth buffer | Active | |
| `astraweave-render/src/rain_splash.rs` | Rain-impact splash spawner | Active | |
| `astraweave-render/src/puddle_accumulation.rs` | Rain-driven puddle formation in terrain concavities | Active | |
| `astraweave-render/src/snow_accumulation.rs`, `snow_footprint.rs` | Per-chunk snow accumulation + entity footprint depression | Active | |
| `astraweave-render/src/environment.rs` | `SkyConfig`, `SkyRenderer`, `TimeOfDay`, `WeatherParticles`, `WeatherSystem`, `WeatherType` | Active | Re-exported via `lib.rs:140-142` |
| `astraweave-render/src/hdri_catalog.rs` | `HdriCatalog`, `HdriEntry`, `DayPeriod` | Active | |

### Decals, water, special effects

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/decals.rs` | Screen-space decal system | Active (feature `decals`) | `DecalAtlas`, `DecalSystem`, `DecalBlendMode`, `GpuDecal`, `DECAL_SHADER` |
| `astraweave-render/src/water.rs` | Chunked-LOD ocean surface (Gerstner waves) + refraction + weave deformation | Active | ~1020 LoC. **W.2a**: camera-distance discrete chunk grid, per-LOD instanced tiles + crack-hiding skirts; world Y from a `water_level` uniform (no baked mesh Y); `set_water_level` live (incl. editor knob). **W.2b.2**: water is now a SEPARATE post-opaque raster pass (`Renderer::run_water_pass`), no longer inside the shared main pass — it samples a scene-color snapshot for screen-space refraction and read-only scene depth for depth-delta shoreline foam; bind group is 4 entries (uniform + scene_color + scene_depth + sampler); `prepare_scene` wires the snapshot/depth each frame. **W.2c.2**: `WaterRenderer` now owns a **weave-deformation instance list** (`WeaveInstance`/`WeaveKind`, ceiling `MAX_WEAVE_INSTANCES = 8`) via `set_weave_instances`/`clear_weave_instances`, riding in `WaterUniforms` (now 512 B; the weave array is 16-aligned at offset 256). The instances are normalized, **position-agnostic analytical profiles** (location lives only in the instance: `world → local = rotate(world_xz − position, −orientation)/radius`), applied view-side in `water.wgsl` `vs_main` **after** the Gerstner sum (Q-cap untouched): `part`/`raise` as additive ±height, `freeze` as a **unified dual-channel** mask (suppresses waves + flattens normal + fragment material-state shift toward ice). Bounded to `±skirt_depth` (CPU intensity-clamp + shader net-clamp) and world-XZ sampled so it never re-exposes a LOD seam; **identity at zero instances** (runtime consumers set none). Editor-side test scaffolding (one part/raise/freeze) lives in `viewport/engine_adapter.rs::set_water_enabled` — the W.2c.3 plug-in seam for gameplay-emitted instances. Measured free on the 1660 Ti (~0.0005 ms/instance; full surface ~0.19–0.21 ms vs the 2.0 ms ceiling). **W.2c.3**: weave instances now arrive from a **binary-glue producer** (not just the editor scaffolding) via the new `Renderer::set_water_weave_instances(&[WeaveInstance])` delegation (renderer.rs; forwards to the private `water_renderer`). The producer (`examples/weaving_playground/src/weave_producer.rs::WaterWeaveProducer`) translates **positional gameplay `WeaveOp`s** — `LowerWater → part`, `RaisePlatform → raise`, `FreezeWater → freeze` (new `#[non_exhaustive]` variant, presentation-only truth) — into `WeaveInstance`s, reading only `op.a.xz` for position, with per-kind default intensity and an **age-driven ramp-in/hold/fade envelope** modulating `intensity` (so instantaneous ops get a visible duration; the `phase` field carries raw age as forward-provision — the W.2c.2 shader reads `intensity`, not `phase`, yet). The gameplay→render direction is **one-way**: the renderer takes only the render `WeaveInstance` type, so `astraweave-render` stays independent of the gameplay crates. **Coexist:** the ops' gameplay-truth behavior is unmodified (`LowerWater` still `clear_water()`s; `RaisePlatform` still Fortifies terrain); the render layer reads them in parallel. The render path (`water.rs`/`water.wgsl`) is byte-identical to W.2c.2 — the only render-crate change is the delegation. **F.4.2 (accent layer — machinery built, NOT yet visually live):** the splash/spray *accents* that garnish this surface exist but are **not yet drawn in a running frame**. `astraweave-fluids` now exposes `FluidRenderer::render_accents` (additive-billboard-only — the SSFR depth/smooth/shade chain is **not** invoked) and `FluidSystem::set_secondary_particles` (+ the `secondary_particle_count()` live-count fix — was hardcoded `65536`, which also stopped `fluids_demo` drawing 65k garbage billboards). A binary-glue `WaterAccentProducer` (`examples/weaving_playground/src/weave_accent_producer.rs`) — sibling to the weave producer, fed the same ops — translates weave triggers (`LowerWater → Part` silt spray / `RaisePlatform → Raise` lift-burst / `FreezeWater → Freeze` one-shot shimmer-then-suppress) into ballistically-aged `SecondaryParticle`s, style as tunable params (A1 procedural shape + B2 floaty motion in the producer; C3 per-kind tint + HDR glow in `secondary.wgsl`'s `info.y` LUT). Composition is **binary-orchestrated**: the binary depends on BOTH render and fluids, but `astraweave-render` and `astraweave-fluids` stay mutually independent (**no render↔fluids edge**; render byte-identical). **F.4.3 made this LIVE in-frame:** the renderer gained a stored `hdr_overlay` callback (pure-`wgpu` closure) fired inside `run_water_pass` on every exit path — after the water surface draws into `hdr_view` and `water_renderer` is restored, before tonemap. Because both `render()` (5220) and `draw_into()` (5873) call `run_water_pass`, the *pass mechanism* is automatically available to the editor too (its producer-feed wiring is the one logged follow-on). The demo registers the closure each frame (`Rc<FluidRenderer>` + an Arc-cheap `wgpu::Buffer` clone + the camera), uploading the live accent set via `set_secondary_particles` first. Measured min-spec (1660 Ti Max-Q, 1080p, 512 accents): accent composite ~0.006–0.013 ms, combined water+accents ~0.18–0.26 ms — ~8× under the 2.0 ms budget. **Canonical end-to-end water trace:** [`docs/architecture/water.md`](./water.md) (truth facade + physics buoyancy + gameplay producers + accents); this row + §2.6 cover only the render-side path. See `W2C1_RECON.md`, `W2C3_1_RECON.md`, `F4_0_RECON.md`, `F4_1_RECON.md`, `W2_DECISIONS.md` §B.1/§F, `W2A_EXECUTION_REPORT.md`, `W2B2_EXECUTION_REPORT.md`, `W2B1_RECON.md`, `F4_3_EXECUTION_REPORT.md`. |
| `astraweave-render/src/deferred.rs` | Deferred rendering pipeline (`DeferredRenderer`, `GBuffer`, `GBufferFormats`) | Active (feature `deferred`) | |
| `astraweave-render/src/debug_quad.rs`, `depth.rs` | Debug visualization + depth resources | Active | |
| `astraweave-render/src/clipmap_terrain.rs` | Clipmap-based terrain rendering | Active | |
| `astraweave-render/src/terrain.rs` | Legacy single-`biome_id` terrain rendering path | Transitional | One in-tree caller (`examples/weaving_playground`) — see `docs/architecture/terrain_materials.md` |
| `astraweave-render/src/terrain_gpu_bridge.rs` | Render-side `TerrainGpuAccelerator` impl (GPU noise + erosion bridge) | Active | |
| `astraweave-render/src/gpu_erosion.rs` | GPU compute SWE erosion | Active | |
| `astraweave-render/src/compute_noise.rs` | GPU compute noise generation (Perlin/fBM/Ridged/Billow/DomainWarped) | Active | |

### Shader infrastructure, streaming, low-level

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/shader_manager.rs` | Hot-reload shader source registry (`ShaderKey`, `ShaderManager`) | Active | 391 LoC; does NOT own GPU resources |
| `astraweave-render/src/shader_permutation.rs` | Compile-time permutation system for Disney BRDF lobes | Active | |
| `astraweave-render/src/subgroup_ops.rs` | Subgroup-optimized shader variants (auto-exposure, prefix sum, bitonic sort) | Active | `SubgroupCapabilities` |
| `astraweave-render/src/asset_index.rs` | `AssetIndex`, `MaterialSetEntry`, `TextureEntry`, `HdriRef as AssetHdriRef` | Active | |
| `astraweave-render/src/residency.rs` | GPU residency manager (`ResidencyManager`) | Active | Streaming-aware texture/mesh lifetime |
| `astraweave-render/src/texture_streaming.rs` | LRU-cache + priority-based texture streaming (`TextureStreamingManager`, `TextureStreamingStats`) | Active | `Renderer` owns it. `new` returns `Result` if the one-thread tokio runtime cannot start. `tick` uploads from a pre-sized ring. `request_at` sorts by camera distance. `read_capped_image` caps `decode_texture` and `Texture::load_texture_async` at 64 MiB. A non-regular file is not read past that cap. Decode uses `decode_limited_image`. Shutdown waits at most 500 ms |
| `astraweave-render/src/virtual_texture.rs` | Sparse virtual texturing (page cache, feedback, LRU) | Active | |
| `astraweave-render/src/texture.rs`, `types.rs` | Core texture / vertex / instance types | Active | `Instance`, `Material`, `Mesh`, `SkinnedVertex`, `Vertex` (re-exported at `lib.rs:147`) |
| `astraweave-render/src/camera.rs` | `Camera`, `CameraController` | Active | |

### Editor-side viewport + adapter

| File | Role | Status | Notes |
|---|---|---|---|
| `tools/aw_editor/src/viewport/renderer.rs` | `ViewportRenderer` multi-pass coordinator: grid → entities → gizmos → selection outline. HDR target Rgba16Float, LDR Rgba8UnormSrgb (lines 31-34) | Active | Post-Fix 27 simplified architecture per `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` |
| `tools/aw_editor/src/viewport/engine_adapter.rs` | `EngineRenderAdapter` wrapping `astraweave_render::Renderer`; carries `RenderMode` enum (`EnginePBR`, `FastPreview`); `EditorQualityPreset` for shadow/post-FX quality | Active | `RenderMode::FastPreview` is Fix 27 residue (lines 25-28). `EditorQualityPreset` at lines 40-… |
| `tools/aw_editor/src/viewport/widget.rs` | `ViewportWidget`: egui input + viewport lifecycle | Active | |
| `tools/aw_editor/src/viewport/terrain_splat.rs` | `EditorTerrainSplat` (superseded by `Renderer::terrain_forward` per `engine_adapter.rs:6-10`) | Transitional (reference material) | See `docs/architecture/terrain_materials.md` |
| `tools/aw_editor/src/viewport/impostor_wiring.rs` | Editor-side impostor pipeline wiring | Active | |
| `tools/aw_editor/src/gizmo/rendering.rs` | Transform-handle gizmo rendering | Active | |

**Status definitions used here:**
- **Active**: Canonical, load-bearing, edit with care
- **Transitional**: In active code paths but tagged for migration (e.g. `terrain.rs` legacy path; `terrain_splat.rs` editor reference material; `FastPreview` residue)
- **Feature-gated Active**: Compiles only when a Cargo feature is enabled; canonical when enabled

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| Three material binding paths | `MaterialManager` (`material.rs`), `MaterialLibrary` (`material_library.rs`), `BindlessMaterialSystem` (`material_bindless.rs`) | Coexisting | Each serves a different purpose: TOML-driven legacy / canonical terrain / bindless modern. No single "the" material system |
| FastPreview vs EnginePBR | `RenderMode::FastPreview` enum variant at `tools/aw_editor/src/viewport/engine_adapter.rs:25-28` | Transitional (Fix 27 residue) | Per `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`: the structural unification phase is complete and deeper unification continues; `FastPreview` mode persists for fallback / weak-GPU scenarios |
| Frame graph vs Renderer | `frame_graph.rs` declarative DAG vs `renderer.rs` imperative command recording | Coexisting (intentional, mid-migration) | `frame_graph.rs:18-25` documents "full delegation is designed for incremental adoption" |
| Lumen GI vs VXGI | `lumen.rs` + `distance_field.rs` + `surface_cache.rs` + `final_gather.rs` vs `gi/vxgi.rs` + `gi/voxelization_pipeline.rs` | Coexisting | Two complete GI implementations; the choice is configurable per scene |
| Terrain rendering paths | Canonical 32-layer `terrain_material_manager.rs` vs legacy single-`biome_id` `terrain.rs` | Coexisting | See `docs/architecture/terrain_materials.md` for full forensic analysis |
| Particle render paths | `particle_render.rs` (Phase 7 modern) vs older inline particle handling in `effects.rs` and `weather_system.rs` | [NEEDS VERIFICATION on whether the older paths are still load-bearing or pure transitional] | |
| `types::Material` vs `material_library::Material` | Both exist as types named `Material` | Coexisting with explicit no-export guard | `lib.rs:251-253` documents the intentional non-re-export of `material_library::Material` to avoid the collision |
| Shadow-uniform single-buffer vs per-cascade buffers | Per-cascade buffers (`renderer.rs:752-757`) are the active path | Single-buffer historical | Per-cascade chosen explicitly to avoid `queue.write_buffer` race; don't "simplify" |

### Naming collisions

- **`Material`**: `astraweave_render::types::Material` (re-exported at `lib.rs:147`) vs `astraweave_render::material_library::Material` (intentionally NOT re-exported at root, per the comment block at `lib.rs:251-253`). Always namespace fully.
- **`TerrainVertex`**: `astraweave_render::TerrainVertex` (legacy single-`biome_id` format at `terrain.rs:18-23`) vs `tools/aw_editor/src/viewport/types.rs::TerrainVertex` (96-byte editor format with `material_ids[4]` / `material_weights[4]`). The `to_engine_vertex()` adapter collapses the rich format to the simple format. See `docs/architecture/terrain_materials.md`.
- **`Renderer`**: `astraweave_render::Renderer` (the canonical aggregator) vs `tools/aw_editor/src/viewport/renderer.rs::ViewportRenderer` (editor coordinator that wraps the canonical). Different responsibilities, sometimes confused in conversation.
- **"PBR shader"**: At least three distinct WGSL programs claim this name in shader files: `pbr.wgsl`, `pbr/disney_brdf.wgsl`, `pbr_terrain.wgsl` / `pbr_terrain_forward.wgsl` / `pbr_terrain_vs.wgsl`. Always cite the exact file when referring to "the PBR shader."
- **"Forward"**: `clustered_forward.rs` (cluster-bin forward), `pbr_terrain_forward.wgsl` (the 32-layer terrain forward path), `Renderer::terrain_forward` field (the live terrain-chunk map at `renderer.rs:5755`). Different scopes.

### Known cognitive traps

- **Trap (§7.7 wrapped-component resource identity)**: When the editor's `EngineRenderAdapter` wraps `Renderer`, the adapter's own resources (e.g. local depth target, terrain-cluster map, splat-build buffers, UI material library) are NOT the same as the underlying `Renderer`'s resources of the same logical role. Reads from the wrapper layer don't reflect writes to the engine layer (or vice versa). **Four confirmed instances in the Editor Multi-Tool Architecture Campaign Sub-phase 3**:
  - Round 5 (2026-05-07, Mechanism 1): depth-target layer — editor's `read_depth_at_pixel` sampled its own local depth texture that engine adapter's `render_to_texture` never wrote to. Fixed by Real-Fix.A `0f569d212` (`Option<&wgpu::TextureView>` depth_view parameter on `Renderer::draw_into`).
  - Round 6 (2026-05-07, Mechanism C): mesh-data layer — initial chunk upload routed to live `Renderer::terrain_forward.chunks`; incremental brush update routed to legacy `terrain_clusters` Vec that no rendering path reads. Fixed by Real-Fix.B `eaaa53433` (shared `upload_or_update_terrain_chunk_forward` helper).
  - Round 7 (2026-05-08, Mechanism H1): texture-data attribute-set layer — paint mutated `vertex.material_ids` + `vertex.material_weights`; `build_chunk_splat_maps` read only `vertex.biome_weights_0/1`. Fixed by Real-Fix.C `ded9a0457` (Option C unified `TerrainVertex` into single canonical `material_*` attribute set).
  - Round 8 (2026-05-08, Mechanism H8.1): UI/renderer capacity boundary — UI offers 22 materials (`MATERIAL_NAMES: [&str; 22]`); splat-build caps at 8; renderer texture array allocates 8 slots. Fix design pending Andrew-gate (Option D-2: canonical material library shared between UI + renderer).
  - Source of truth: `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header (commit `e3d07f366`, 2026-05-08).
- **Trap**: Treating the frame graph as the command-driving path.
  **What's actually true**: `frame_graph.rs:18-25` explicitly states the frame graph "validates resource flow, and exercises the automatic topological ordering, resource lifetime, and aliasing analysis" but pass nodes "delegate actual GPU command recording to `Renderer` methods. Full delegation is designed for incremental adoption." `Renderer::render` is the actual command-recording path today.
- **Trap**: Adding new render features to `renderer.rs` as inline WGSL.
  **What's actually true**: `renderer.rs:18-…` contains a giant inline `SHADER_SRC` constant via `concat!` + `include_str!`. New shaders should go in `shaders/*.wgsl` and be loaded via `ShaderManager` + included into pipelines explicitly. The inline `SHADER_SRC` is historical.
- **Trap**: Adding a new "FastPreview" path or any editor-only render duplicate.
  **What's actually true**: Per `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`, the campaign exists explicitly because dual pipelines diverged across 12 dimensions. CLAUDE.md Scope Discipline says "Never build a second implementation of a logical system that already exists (rendering path, vertex format, material pipeline, scheduler, tonemap chain, scene serializer)."
- **Trap**: Adding a new render module that duplicates an existing subsystem.
  **What's actually true**: The 123-file render crate has many similar-named modules (e.g. `bloom.rs` vs older bloom in `advanced_post.rs`; `gtao.rs` vs older SSAO in `post.rs`'s `WGSL_SSAO`). Before adding a new module, `rg 'struct <Name>\|trait <Name>'` workspace-wide per CLAUDE.md Scope Discipline.

---

## 7. Decision Log

### Decision: God-ray composite does not use read-write rgba16float storage
- **Date:** 2026-10-14
- **Status:** Accepted
- **Context:** `texture_storage_2d<rgba16float, read_write>` is not in the WebGPU guaranteed feature set. Creation failed on NVIDIA DX12, the pipeline was marked invalid, and later submits lost the device. Integrate also bound a depth texture as a filterable float sample.
- **Decision:** Sample the HDR scene, write shafts into a separate write-only `Rgba16Float` texture, and copy that onto HDR. Sample depth as `texture_depth_2d`. If the error scope reports a failure, drop the pass and keep drawing.
- **Alternatives considered:** Enabling `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` and keeping read-write storage. That still fails on adapters that do not expose the access.
- **Consequences:** One extra full-resolution texture and a copy per frame while shafts are on. The scar volume stays a sampled 3D input of the density pass.

### Decision: Single canonical `Renderer` struct as aggregator
- **Date:** [Reasoning not recovered from available sources — predates current trace]
- **Status:** Accepted (in active code)
- **Context:** `astraweave-render/src/renderer.rs:691-…` defines a single `Renderer` struct that owns wgpu device/queue/surface plus all pipelines. The choice was reaffirmed by the Fix 27 Unified Pipeline Campaign (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`) which eliminated the dual editor pipeline.
- **Decision:** All scene rendering goes through one `Renderer`. The editor wraps it via `EngineRenderAdapter` (`tools/aw_editor/src/viewport/engine_adapter.rs`); examples instantiate it directly.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** 7,800-line file with high blast radius for changes. Mitigated by extensive integration tests at `astraweave-render/tests/`. CLAUDE.md Scope Discipline explicitly forbids a second rendering path.

### Decision: Frame graph as declarative scaffolding with incremental adoption
- **Date:** Not recovered from commit messages; `frame_graph.rs:18-25` "Migration Status" comment documents the current state.
- **Status:** Accepted (transitional; full delegation deferred)
- **Context:** A pure imperative `Renderer::render` is brittle to refactor; a declarative frame graph (DAG of `PassDecl` with explicit resource I/O) enables topology validation, resource-lifetime analysis, and aliasing decisions.
- **Decision:** Build the frame graph in `frame_graph.rs` + `graph.rs` + `graph_adapter.rs`, but keep pass nodes delegating actual GPU command recording to `Renderer` methods. Migration to fully delegated nodes is incremental.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Two parallel orchestration models coexist (frame graph for topology + `Renderer::render` for commands). Adding a new pass currently requires touching both.

### Decision: Unify editor and runtime rendering through a single Renderer (Fix 27)
- **Date:** Plan dated 2026-04-05 (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`). Status per CLAUDE.md: "structural phase complete (astraweave-render non-optional, entity rendering through engine); deeper unification (shadow/IBL/post-processing alignment) ongoing."
- **Status:** Accepted (in active execution)
- **Context:** Per the campaign plan executive summary, the editor maintained two parallel rendering pipelines (FastPreview + EnginePBR) that "diverged across 12 dimensions (vertex format, materials, shadows, IBL, post-processing, etc.)". Each had its own PBR shader, CSM, BRDF LUT, IBL, glTF loader, tonemap, and texture pipeline.
- **Decision:** Eliminate the FastPreview path. Route ALL scene rendering through `astraweave_render::Renderer`. Preserve editor-specific overlays (grid, gizmo, physics debug, blueprint overlay) via a formal overlay injection protocol.
- **Alternatives considered:** Documented in the campaign plan (`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`): FastPreview-as-permanent fallback was considered and rejected because the dual path created semantic drift across 12 dimensions.
- **Consequences:** ~4,000 lines of duplicated rendering code targeted for deletion. The `RenderMode::FastPreview` enum variant remains (`engine_adapter.rs:25-28`) as transitional residue. Visual parity between editor and runtime is now guaranteed by construction for the unified parts of the pipeline.

### Decision: Per-cascade shadow uniform buffers (not a single buffer)
- **Date:** Not directly recovered; comment block at `renderer.rs:752-757`.
- **Status:** Accepted (in active code)
- **Context:** Using a single shadow uniform buffer caused a `queue.write_buffer` race where all writes resolved before the command buffer executed, causing both shadow passes to read the same (last-written) cascade matrix.
- **Decision:** Use separate `wgpu::Buffer` + `wgpu::BindGroup` per cascade (`shadow_cascade_bufs: [wgpu::Buffer; 2]`, `shadow_cascade_bgs: [wgpu::BindGroup; 2]`).
- **Alternatives considered:** [Reasoning not recovered — the race-condition discovery is documented in the inline comment; alternative mitigations like explicit submission ordering weren't recorded.]
- **Consequences:** Two cascade-buffers worth of GPU memory vs one. Trivial cost; correct behavior.

### Decision: `Renderer::draw_into` accepts `Option<&wgpu::TextureView>` for depth (Real-Fix.A)
- **Date:** 2026-05-07, commit `0f569d212`. Andrew-gate Option (a) per `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Round-5-Closure.
- **Status:** Accepted (in active code)
- **Context:** Editor's `read_depth_at_pixel` sampled its own local depth texture; engine adapter's `render_to_texture` wrote to its own internal depth target (no depth attachment passed). Reads returned cleared/uninitialized 0.0 → near-plane unprojection → cursor_center ≈ cam_pos invariantly. The §7.7 wrapped-component resource identity trap at the depth-target layer.
- **Decision:** Cross-crate API addition — pass `Option<&wgpu::TextureView>` depth_view as a `Renderer::draw_into` parameter. Editor supplies its own depth view; engine writes terrain depth into it.
- **Alternatives considered:** Option (b) — expose engine adapter's internal depth target via accessor (works around the API shape; leaves overlay-vs-terrain depth-test bug latent). Option (c) — aw_editor-local depth pre-pass re-rendering terrain depth (redundant work; precision-mismatch risk). Documented in Round-5-Closure §12.
- **Consequences:** +33/-9 lines across 5 files. Resolves Mechanism 1 (depth-target §7.7). Aligns with CLAUDE.md v0.10.1 Edit 2 (no-second-implementation).

### Decision: Shared `upload_or_update_terrain_chunk_forward` helper (Real-Fix.B)
- **Date:** 2026-05-07, commit `eaaa53433`. Andrew-gate Option 1.
- **Status:** Accepted (in active code)
- **Context:** Initial terrain upload (engine adapter `upload_terrain_chunks`) routed correctly to `Renderer::terrain_forward.chunks`. Incremental brush update (engine adapter `update_terrain_chunk` → `rebuild_terrain_clusters_for_chunk` → legacy `terrain_clusters` Vec) routed to a path no rendering ever read. §7.7 at the mesh-data layer.
- **Decision:** Extract a shared helper called by both paths.
- **Alternatives considered:** Option 2 — inline copy of splat-build + filter logic into `update_terrain_chunk` (creates the exact second-implementation anti-pattern Edit 2 forbids; documented as rejected in the Round-6-Closure entry).
- **Consequences:** Mesh-data §7.7 instance fixed. Dead-code observation in Round-6-Closure: legacy cluster path is dead, cleanup deferred.

### Decision: Unified `TerrainVertex` material attribute set (Real-Fix.C)
- **Date:** 2026-05-08, commit `ded9a0457`. Andrew-gate Option C.
- **Status:** Accepted (in active code)
- **Context:** Paint brush mutated `vertex.material_ids` + `vertex.material_weights`; `build_chunk_splat_maps` read only `vertex.biome_weights_0/1`. §7.7 at the texture-data attribute-set layer (intra-component sub-variant).
- **Decision:** Option C — unify into a single canonical `material_*` attribute set. New `biome_weights_8_to_material_slots` helper preserves visual blending semantics.
- **Alternatives considered:** Option A (mirror writes — smallest, doesn't unify identity) and Option B (switch reader — eliminates trap but biome generation may need updating). Documented in Round-7-Closure §12.
- **Consequences:** +297/-140 lines across 9 files. Texture-data §7.7 instance fixed. Andrew-gate verification 2026-05-08 found 8/22 materials paint correctly, 14/22 produce "pixelated green splotches" — surfaced Round 8 Mechanism H8.1 (UI/renderer capacity boundary).

### Decision: 32-layer canonical material library (Real-Fix.D 2026-05-08)
- **Date:** 2026-05-08. See `docs/architecture/terrain_materials.md` for the canonical decision record.
- **Status:** Accepted (in active code)
- **Notes:** Detailed in the terrain materials trace. Mentioned here because it directly resolves the Round-8 Mechanism H8.1 (UI/renderer capacity boundary) at the material library layer.

### Decision: Two GI implementations coexist (Lumen + VXGI)
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted (in active code)
- **Notes:** `lumen.rs` orchestrates Lumen-style GI; `gi/vxgi.rs` implements Voxel GI. Both are first-class modules. Whether one is preferred for new scenes, or whether they serve different use cases, is not recorded in available sources.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | All scene rendering goes through `astraweave_render::Renderer`; the editor wraps it via `EngineRenderAdapter` | Partial | Compile-time for entity rendering; FastPreview residue at `engine_adapter.rs:25-28` is a documented exception |
| 2 | `Renderer::draw_into(view, depth_view, enc)` writes depth to the caller-supplied `depth_view` when provided | Yes | Code path at `renderer.rs:5234-…`; behavioural verification via editor brush ring rendering (Andrew-gate 2026-05-07 partial PASS) |
| 3 | Per-cascade shadow uniforms use separate `wgpu::Buffer` instances to avoid `queue.write_buffer` race | Yes | `renderer.rs:752-757` (per-cascade arrays + the comment explaining the race) |
| 4 | The frame graph in `frame_graph.rs` validates topology but delegates GPU command recording to `Renderer` methods | Yes | `frame_graph.rs:18-25` "Migration Status" block plus the absence of command-encoder calls in `frame_graph.rs` pass-node implementations |
| 5 | `MAX_TERRAIN_LAYERS = 32` and `NUM_SPLAT_MAPS = 8` are the canonical material library capacity | Yes | `material_library.rs` constants; mirrored in `pbr_terrain.wgsl` (`array<TerrainLayer, 32>`, and `height_blend`'s weight, height, and result arrays), `terrain_material.rs` (`TerrainMaterialGpu`), and `terrain_splat_builder.rs`. Detailed in `docs/architecture/terrain_materials.md` |
| 6 | `MaterialGpu` flags (`FLAG_HAS_ALBEDO`, `FLAG_HAS_NORMAL`, `FLAG_HAS_ORM`, `FLAG_TRIPLANAR`) and extended-material flags (`MATERIAL_FLAG_CLEARCOAT`, etc.) are stable bit positions | Yes | `material.rs:25-31` and `material_extended.rs` (`MATERIAL_FLAG_*` constants re-exported at `lib.rs:223-226`) |
| 7 | `types::Material` is re-exported at the crate root; `material_library::Material` is intentionally NOT, to avoid the collision | Yes | `lib.rs:147` (re-export) + `lib.rs:251-253` (explicit no-export comment) |
| 8 | `Renderer::resource_generation` is monotonically incremented on resize / resource invalidation; `CachedBindGroup` rebuilds against the new generation | Yes | `renderer.rs:708-710` (field doc) + `bind_group_cache.rs` API |
| 9 | All shader files under `astraweave-render/shaders/` are registered via `ShaderManager` (for hot-reload) OR inlined via `include_str!` in pipeline construction | Partial | [NEEDS VERIFICATION — workspace audit of shader registration coverage] |
| 10 | `#![deny(unsafe_code)]` is enforced on the crate root (`astraweave-render/src/lib.rs:1`) — the crate is unsafe-free | Yes | Compile-time deny lint at `lib.rs:1` |
| 11 | `astraweave-materials::Node` enum is `#[non_exhaustive]` so external adders cannot break exhaustive matches | Yes | `astraweave-materials/src/lib.rs:7` |
| 12 | Editor `RenderMode::EnginePBR` is the default; `FastPreview` exists for fallback only | Yes | `engine_adapter.rs:31-34` `impl Default for RenderMode` returns `EnginePBR` |
| 13 | All Sub-phase 3 Mediator Brush §7.7 fixes preserve visual parity with the canonical pipeline — i.e. Real-Fix.A/B/C/D each verified via Andrew-gate | Yes | Campaign-doc Status header at `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` records each gate verdict |
| 14 | The water surface renders as a chunked-LOD discrete tile grid (per-LOD instanced tiles + skirts) with world Y from `WaterUniforms.water_level` — no baked mesh Y, no single fixed plane | Yes | `water.rs` `WaterRenderer::update`/`render` (camera-distance LOD assignment + per-LOD instanced draw) + `WaterUniforms.water_level`; `water.wgsl` `vs_main` applies the uniform; W.2a (`docs/campaigns/water-successor/W2A_EXECUTION_REPORT.md`) |
| 15 | The water pass runs in its OWN raster pass AFTER the opaque main pass closes AND after the `hdr_tex → water_scene_color` snapshot copy; it writes `hdr_view` while sampling `water_scene_color` (a copy, never the live attachment) and binds the scene depth READ-ONLY (`depth_ops: None`) for both depth-test and sampling. The depth attachment view and the sampled `scene_depth` binding must be the same texture; `draw_into()` threads the caller-supplied depth so the editor tests/samples the depth its opaque geometry wrote. | Yes | `renderer.rs` `run_water_pass` + its call sites in `render()` (depth `None`) and `draw_into()` (caller depth); read-only-depth+sample capability proven by `examples/depth_sample_capability_probe.rs`; W.2b.2 (`docs/campaigns/water-successor/W2B2_EXECUTION_REPORT.md`) |
| 16 | Weave-response (part/freeze/raise) is **render-side presentation owned by `WaterRenderer`** (NOT the `astraweave-water` gameplay-truth facade — see `W2_DECISIONS.md` §B.1); `astraweave-render` has no dependency on `astraweave-water`. Profiles are **normalized, position-agnostic** (location lives only in the runtime `WeaveInstance`); deformation applies in `water.wgsl` `vs_main` AFTER the Gerstner sum (per-wave Q-cap untouched), is bounded to `±skirt_depth` and world-XZ sampled (no new LOD-seam mechanism), and is **identity at zero instances** (the loop is `weave_count`-gated, so the surface is bit-identical to pre-W.2c with no weaves). Ceiling `MAX_WEAVE_INSTANCES = 8`. | Yes | `water.rs` `WeaveInstance`/`set_weave_instances` + `WaterUniforms` 512 B layout (`test_uniforms_size`); `water.wgsl` `weave_accumulate`/`weave_profile`; zero-instance identity + per-instance cost measured by `examples/water_budget_probe.rs` (PROBE D); W.2c.2 (`docs/campaigns/water-successor/W2C1_RECON.md`) |
| 17 | The gameplay→render weave seam is **one-way**: the renderer exposes only `Renderer::set_water_weave_instances(&[WeaveInstance])` (a render type), and the `WeaveOp → WeaveInstance` translation lives in **binary glue** (`weave_producer.rs`), so `astraweave-render` gains **no** dependency on `astraweave-gameplay`/`astraweave-weaving`. The ops' gameplay-truth behavior is unmodified (coexist) — `FreezeWater` is presentation-only (no walkable-ice/buoyancy this arc). | Yes | `renderer.rs` `set_water_weave_instances` (takes `crate::water::WeaveInstance`); `astraweave-render/Cargo.toml` has no gameplay edge; `weaving.rs::apply_weave_op` `FreezeWater` arm (budget+log only); W.2c.3 (`docs/campaigns/water-successor/W2C3_1_RECON.md`) |
| 18 | The weave-impact **accent** layer reuses `astraweave-fluids`' secondary-billboard machinery and stays **crate-independent**, and (F.4.3) is now **LIVE in-frame**: accents are produced by a binary-glue `WaterAccentProducer` and drawn by `FluidRenderer::render_accents` (additive-billboard-only — the SSFR surface chain is **never** invoked for accents), composited over the `WaterRenderer` surface **after the water pass, before tonemap, into the HDR target** via the renderer's stored `hdr_overlay` callback fired inside `run_water_pass` on every exit path. The callback type is pure `wgpu` `(encoder, hdr_view, scene_depth, device, queue)`, so `astraweave-render` gains **no** `astraweave-fluids` dependency — the binary registers a closure whose body calls `render_accents`. Both `render()` (demo) and `draw_into()` (editor) fire it because both call `run_water_pass`, so editor parity is automatic for the **pass mechanism** (the editor *producer feed* is a logged follow-on). **Zero-accent identity:** `render_accents` early-returns at count 0 and a `None` overlay records zero GPU work → byte-identical frame. **Measured (1660 Ti Max-Q, Vulkan, 1080p, 512 accents):** accent composite **0.006–0.013 ms** (median); water surface 0.18–0.24 ms; **combined 0.18–0.26 ms** — accents ~40–80× under the 0.5 ms target, total ~8× under the 2.0 ms water budget. | Yes (live composite; editor producer-feed deferred) | `astraweave-render/src/renderer.rs` `hdr_overlay` field + `set_hdr_overlay` + `fire_hdr_overlay` (called in `run_water_pass` all 3 exits, after `water_renderer` restore); `render_accents` (no depth/smooth/shade); `set_secondary_particles` + live-count `secondary_particle_count`; `examples/weaving_playground/src/{main.rs (per-frame upload + Rc-closure overlay), weave_accent_producer.rs}`; neither crate's `Cargo.toml` references the other; combined measurement `examples/weaving_playground/examples/accent_budget_probe.rs`; `gpu_render_accents_smoke`; F.4.3 (`docs/campaigns/water-successor/F4_3_EXECUTION_REPORT.md`) |
| 19 | **One BRDF for every fragment — no stepped shading tier.** T.2d.F (2026-07-25, director-ratified) deleted the material-LOD tiers: the LOD1\|2 threshold (pixel footprint 2.0) was the T.2d-convicted camera-anchored detail boundary (matched to 4 px in two frames), and because `fwidth(world_pos)` includes the height derivative the per-pixel tier selection dithered over rough ground (40-55% of far-field high-frequency energy was tier flicker; measured removed: -48.6%/-51.7% at the boundary stations). `evaluate_brdf` (GGX + height-correlated Smith + Burley + Kulla-Conty multiscatter) is the single shading path in SHADER_SRC, SKINNED_SHADER_SRC and `pbr_terrain_forward.wgsl`; the static shader's former LOD2 IBL approximation died with the shared threshold (full IBL everywhere). Close range measured unchanged (1 px/786,432 at the 20 m station); far field brightens up to +9.5% (accepted consequence — T.2a/T.2c far-field frames rot); min-spec frame cost -1.0% (27.055 -> 26.773 ms median, 1660 Ti Max-Q, 1080p, n=300). A stepped tier may NOT return under any perf outcome — the ratified fallback is a falloff CONTINUOUS in footprint. | Yes | `renderer.rs::material_lod_tiers_are_retired` (asserts symbol ABSENCE across all four shader surfaces; proven to fail on pre-fix code); tombstone comment in `brdf_common.wgsl`; `docs/audits/T2DF_OUTCOME.md` |
| 21 | **One forward Hi-Z pyramid, presentation particles, and streamed albedo.** (a) The forward renderer has one `HizPyramid`. `draw_into` builds it from that frame's depth view into R32Float only when `sample_hiz` is set. `set_sample_hiz(true)` drops any pyramid so the next frame copies the current depth view, including when the size is unchanged. The default is false. SSGI samples scene depth, and no pass calls `HizPyramid::view`. `NaniteCullingPipeline` (feature `nanite`) builds a separate pyramid in `build_hiz_pyramid` from `src/shaders/nanite_hiz_pyramid.wgsl`. Its copy from the previous depth is a placeholder (`_src_depth` is unused), and `draw_into` does not record it. The forward pyramid is not read back into the simulation. (b) CPU `FrustumPlanes::test_aabb` remains the `draw_into` model filter while `set_gpu_cull_enabled` is off, and that path does not build indirect buffers. When the flag is on, `IndirectDrawPipeline` writes one `DrawIndexedIndirectCommand` per primitive and the model loop binds that model's group 3 before `draw_indexed_indirect`. `CullingPipeline` is not dispatched. `Renderer::render` keeps the CPU loop. A composite with no explicit AABB keeps a mesh AABB transformed by its instances outside `model.aabb`, and `duplicate_model` copies it. `add_model`, `add_model_with_texture`, `add_model_with_pbr_textures`, and `add_model_sharing_texture` do not, so those models take `draw_indexed` with no frustum test while the flag is on. The default-off main loop ignores the side channel. Shadow cascades test it. `write_model_vertices` refreshes a stored mesh AABB. The indirect AABB and command buffers share one element capacity, and the shader stops at the batch count. The dispatch stamp is `gpu_cull`. `rendered_model_count` skips a frustum miss. (c) `GpuParticleSystem` runs only when an emitter is set, uses the caller seed and `dt`, and does not write ECS, nav, or physics. (d) A streamed albedo stays on the 1×1 fallback bind until `TextureStreamingManager` admits the decoded image. The worker reads at most 64 MiB and decodes with `decode_limited_image` (4096-pixel edge). `Drop` waits at most 500 ms, so a blocked read does not hold the caller. A failed load is stored; it is not dropped with `.ok()`. The last resident image, or that fallback, stays bound. `TextureStreamingManager::new` returns `Result` when the decode runtime cannot start. | Yes | `hiz_pyramid.rs` + `tests/hiz_depth_copy.rs` + `tests/hiz_rebind.rs`; `tests/frustum_agree.rs`; `gpu_cull_indirect_instance_count_matches_the_cpu_frustum`; `tests/particle_present.rs`; `texture_streaming.rs` `tick` / `admit`; `tests/stream_fallback.rs` |
| 22 | **A GPU timestamp slot exists only for a pass that is recorded, and `resolve_query_set` is on that write encoder.** `draw_into` does not reserve a skipped pass, does not create a command encoder, and does not call `queue.submit`. Sky, water, atmosphere, and particles record into the caller's encoder. Hi-Z records only when `sample_hiz` is set, and that build copies the depth view passed to that `draw_into`. `submit_timestamped` rejects a resolve that is missing or recorded on a different encoder, submits that one buffer, polls without blocking for at most 5s, then copies. The copy is a later submission (wgpu#6406). A resolve in a later submission deadlocks NVIDIA. Equal non-zero ticks are an empty pass and stay out of the sum. Soft particles sample scene depth through a read-only attachment. `gpu_frame_budget` prints per-frame validation and out-of-memory scopes, and reads back `0xA1B2C3D4` copied in that same encoder. The lavapipe heavy-scene test fails if that scope is an error or the word is not `0xA1B2C3D4`. The heavy scene is one model per instance. `budget_11ms` is `pass` when `median_gpu_frame_ms` is ≤ 11 and `fail` when it is over. The min-spec record is the GTX 1660 Ti Max-Q heavy scene at `939a21bb`: 3.688 ms, pass. `gpu_frame_budget` accepts `--atmosphere` alone. That frame prints `atmosphere_with_aravath_fog` only when fog and shafts both build; otherwise it bails before a median. Every other enabled gated flag is rejected before the scene. `--flag=false` and `--flag false` are off and do not bail. With no enabled flag the printed median is the HDRI sky with Hi-Z off, and that median is blank. It is not the `939a21bb` 3.688 ms row. The constructor leaves Bruneton off. `set_atmosphere_enabled(true)` stays false while fog and shafts are both off, and `record_bruneton` returns before that pair can scatter to 50 km. `light_the_world` turns it on only after fog and shafts both build. A panic during those builds drops both stored passes and leaves Bruneton off. The `--atmosphere` capture at `417f267f` is 4.474 ms. Its atmosphere stamp is 1.774 ms and wraps `record_bruneton` and `composite_atmosphere`. The 4.474 ms capture's binary hardcoded `gpu_cull=cpu_fallback` and `gpu_cull_reason=group3_per_model` and had no `set_gpu_cull_enabled`. This binary calls `set_gpu_cull_enabled(true)`. After the samples it prints `gpu_cull=gpu` and `gpu_cull_reason=indirect` only when a `draw_into` dispatched the cull, and `cpu_fallback` / `no_dispatch` otherwise. A new run is not that measurement. Greybox never calls the setter, so its `draw_into` stays on the CPU frustum. `record_bruneton` passes the light direction as the sun, or the time-of-day sun when that vector is empty, and the bound camera's near and far. The game camera and the budget camera use `GAME_ZNEAR` and `GAME_ZFAR`. | Yes | `gpu_profiler.rs` `resolve_queries`, `submit_frame`, `wait_for_gpu`; `tests/gpu_timestamp.rs`; `budget_gate.rs` `reject_gated_flags` (the bin calls it; `--atmosphere` alone is accepted); `gpu_frame_budget --scene heavy` |
| 20 | **Terrain is a CSM caster AND receiver, through the live machinery only, with one owner per resource (L.3).** (a) One CSM sampling implementation: `shaders/shadow_common.wgsl`'s `csm_shadow_factor` is the ONLY cascade-select+PCF code; SHADER_SRC (group 2) and `pbr_terrain_forward.wgsl` (group 4) both call it — adding a third sampling implementation (or resurrecting the dormant `shadow_csm.rs` path alongside it) violates this. (b) The terrain receiver binds the renderer's EXISTING `light_bg` at group(4) — layout from the shared `TerrainMaterialManager::create_shadow_bgl` used by `Renderer::new` and the pipeline tests; no second shadow bind group of the same logical role may exist (§7.7). (c) The `extras.x < 0` sentinel disables shadow work in BOTH shaders and skips the caster passes; the off-state is byte-identical to pre-L.3 (proven: 0 differing pixels at the toggle stations). (d) The shadow term multiplies the DIRECT sun only — never the IBL/ambient terms (shadowed ground stays skylit; same term discipline as L.2's AO-on-indirect-only). (e) Terrain casts into ALL cascade layers (**4 since L.3.C**: 0.5→86.1, →500, →1400, →3000 m; `CASCADE_COUNT` is the single source of truth and every per-cascade array is `array::from_fn` over it) with per-cascade AABB culling (`TerrainChunkGpu.aabb_*`, computed at upload — the single source of chunk bounds). (f — L.3.A) Coverage must not end inside plainly-visible range at ANY supported camera regime, its fade must be WIDE (the last 30% of coverage, not a fixed narrow band). (g — L.3.A/L.3.C) Cascades `>= FIRST_CACHED_CASCADE` run under `FarCascadePolicy` (shipped: `LiveC2` — c2 re-fits every frame, c3 stays cached). A frozen window is safe to sample ONLY through the **two-phase commit**: `c_far_pending` is set when the matrix changes and cleared ONLY by a pass that actually rendered that layer — never treat a cached GPU resource as filled because its CPU parameters were updated. That one write is also what makes both pass gates policy-aware; bypassing it for a live cascade yields right-matrix/stale-map. A cascade carries the drift pad **iff** its window can freeze — the pad is not free (padded c2 renders 1.880 m texels against 1.491 m live). (h — L.3.C resolution) The receiver-bias cap is DERIVED from the fits the renderer just computed (`min(1, c1_depth / this_depth)`, delivered in `MainLightUbo.bias_scales`), never a constant: constants derived from a *model* of the fits shipped 8.5% wrong. (i — verification, L.3.A→L.3.C, director-ratified) Camera-fitted or temporally-cached shadow work is verified by **continuous per-frame legs** (no settle, no toggling), with **refresh telemetry per far cascade** wherever any cache survives, and measured by **twin deterministic flights differenced** (`off[i] − on[i]`) so view-dependent shading cancels; static stations alone are never sufficient, and no metric is trusted until it has been shown to RESPOND to a change whose sign is already known. | Yes | `shadow_common.wgsl` (single implementation + `bias_scales`); `create_shadow_bgl` (shared constructor); the sentinel contract tests (`test_shadow_override_sentinel_logic`, `test_shader_has_conditional_shadow_not_hardcoded`); `renderer.rs` `FarCascadePolicy` / `CascadeFit` / `shadow_cascade_fits` / `shadow_far_telemetry`; `docs/audits/L3_OUTCOME.md`, `L3A_OUTCOME.md`, `L3B_OUTCOME.md`, `L3C_OUTCOME.md`, `L3C_RESOLUTION_OUTCOME.md` |
| 23 | **Volumetric storage is write-only `Rgba16Float`.** God-ray composite samples the HDR scene and writes a separate storage texture, then copies onto HDR. It does not use `read_write` storage. Integrate samples scene depth with `texture_depth_2d`. A failed error scope or missing write-only support disables that pass and leaves the rest of the frame running. | Yes | `composite_shader_writes_a_separate_target`; `integrate_shader_samples_depth`; `gpu_fault::storage_write_supported` |
| 24 | **The cloud-shadow map is one 512² `Rgba16Float` transmittance texture, updated one slice per presentation frame.** The march is 16 steps (`CLOUD_SHADOW_STEPS`). Thirty-two slices (`CLOUD_SHADOW_SLICES`) cover the map. Slice and time are `cloud_shadow_slice(frame_index)`, not wall time, and every slice in a cycle shares that time and the camera XZ captured on the cycle's first frame. `IblParams._pad` is that center. The PBR shaders sample `.r` for the direct sun and `.g` for how much indirect light follows that shadow. Darkness is `CLOUD_SHADOW_STRENGTH` (default 4): the weather weight and the extinction are both multiplied by it, and `.g` is `(strength - 1) / strength`. A weather weight of 0 stays clear. Strength 1 leaves skylight alone. R16Float is not used: wgpu does not guarantee it as storage. The pass stays on unless `set_cloud_shadows_enabled(false)`. Aravath leaves it on. | Yes | `cloud_shadow_slice_is_a_function_of_frame_index`; `cloud_shadow_slices_match_a_full_bake`; `cloud_shadow_strength_darkens_without_touching_clear_sky`; `sample_cloud_shadow` |
| 25 | **An IBL bake retires each submission before it returns.** Specular mip 0 uses one GGX sample. Later mips use 64 / 128 / 256 samples for Low / Medium / High. Each submission is polled with `WaitForSubmissionIndex`. The wait is wgpu-core's 60s `CLEANUP_WAIT_MS` and returns `PollError::Timeout` while the fence is still open. That is not the 6.3s `Queue::drop` budget, which panics. On timeout the cubes stay on `IblManager` with that `SubmissionIndex`. Device loss is not `PollError`. `Queue::drop` must not be the first wait on that work. | Yes | `test_prefilter_poll_finishes` (Low bake runs `SPECULAR_PREFILTER_WGSL` and polls every mip face to `wait_finished`); `IblQuality::prefilter_sample_count`; `test_sample_count_by_quality` (Rust sample counts only; it does not poll) |
| 26 | Same-named `#[repr(C)]` Rust structs and WGSL `var<uniform>` / `var<storage>` structs share size and field offsets. `vec3` is 16-byte aligned in WGSL. `FogDensityParams` is 160 bytes (`froxel_dims` at 96, `wind_dir` at 144). `ScatterParams` is 176 bytes. `CascadeData` storage stride is 96. | Yes | `tests/wgsl_struct_layout.rs` walks `shaders/` and `src/`. |
| 27 | **Fate-scars do not add a volumetric pass.** A 32³ world-space texture is sampled by `fog_density.wgsl` at the froxel's world position and written into the existing density volume (R density, G forward weight, B tint). `scatter.wgsl` consumes those channels. The froxel dimensions stay the quality preset. WGSL does not read `i64`, and the scar is not addressed by `froxel_idx % 64`. Fog-off (`enabled == false`, or no pass) uploads nothing. | Yes | `scar_shader_samples_world_space_and_has_no_i64`; `Renderer::upload_veil_scar_volume` returns before work when fog is absent |
| 28 | **Linux render tests default to one thread until libvulkan ≥ 1.4.345.** When `RUST_TEST_THREADS` is unset, `serialize_lib_tests.rs` sets it to `1` before libtest reads it. `--test-threads` and an already-set variable still win. Lib tests compile the module. Integration tests include `tests/support/linux_test_threads.rs`, including `frame_alloc.rs`, `stream_fallback.rs`, and `instance_debug_flags.rs`. The production library does not. `InstanceDescriptor::default()` ignores `WGPU_DEBUG` and still enables debug utils in debug builds. The check is a child process whose environment is `Command::env`; the parent does not call `set_var`. Remove the hook after a parallel `cargo test -p astraweave-render` with every Mesa ICD loaded finishes on that loader (Vulkan-Loader #1863 / #1866; tag `v1.4.344` does not contain `5ee27b3`). | Yes | `serialize_lib_tests::lib_tests_default_to_one_thread`; `tests/instance_debug_flags.rs`; `tests/frame_alloc.rs`; `tests/stream_fallback.rs` |
| 29 | **A missing headless adapter is a skip line or a panic.** `NotFound` appends `SKIP(gpu)` under the workspace `target/` directory only when `AW_GPU_SKIP_IF_UNAVAILABLE` is set. That directory is `CARGO_TARGET_DIR` when set, otherwise the parent of the render crate's compile-time manifest joined with `target` (`renderer.rs` and `tests/support/gpu_capability.rs`). The calling test's runtime manifest is not walked, so `<crate>/target` does not move the log. A parent outside that directory still panics. When `AW_GPU_SKIP_IF_UNAVAILABLE` is unset, `NotFound` panics. A device refusal panics, including from `request_headless` while that variable is set. `prefer_fallback` tries software and then hardware unless that variable is set, in which case it does not continue. The append flag is `libc::O_NOFOLLOW`. `upscale_pass_creation`, `cloud_pass_creation`, and `enabling_sample_hiz_rebinds_to_the_current_depth_view` record a skip or panic when depth `textureLoad` is absent. `golden_gradient_matches_with_tolerance` records `NotFound` under its own thread name. On macOS Test full, `scripts/ci/check-gpu-skip-log.sh` fails if any name in `scripts/ci/gpu-skip-required-macos.txt` is absent, even when the log already has another line. That list is the default-feature software-only tests profile ci runs. `gpu-tests` binaries stay off it. `test_backend_detection`, `golden_triangle_render`, and `golden_determinism_check` are on it: an integration test sets `cfg(test)`, and those three request only a software adapter. `choose_adapter` is unchanged. | Yes | `probe_stop`; `depth_texture_load_gate`; `fallback_probes`; `headless_for_tests`; `record_gpu_skip_for_tests`; `crate_local_target_keeps_the_workspace_skip_log`; `skip_log_outside_the_workspace_target_still_panics`; `scripts/ci/check-gpu-skip-log.sh` |

---

## 9. Performance & Resource Profile

### Hot paths
- **Main scene pass** (`Renderer::render` and `Renderer::draw_into`): runs once per frame at ~60 Hz target. Cost dominated by fragment-shader light + shadow + material sampling.
- **Per-fragment cluster lookup + light list iteration**: bounded by clustered-light bin density and per-cluster light count.
- **Shadow cascade rasterization**: 2 cascades by default per `FrameGraphConfig::default()` (`frame_graph.rs:67`). L.3 measured the full terrain caster+receiver cost on min-spec (1660 Ti, editor terrain scene, same-build sentinel on/off): **+0.766 ms** at the distance-heavy 1080p framing, **+0.397 ms** at the 20 m close-up — per-cascade AABB culling holds caster draws to ≤40 chunks/frame of 441.
- **Bloom downsample + tent upsample**: 13-tap downsample, fixed chain length. Per `bloom.rs` and `lib.rs:65` header.
- **GTAO bilateral upsample, SSGI denoise, SSR Hi-Z ray march**: per-frame, fixed-cost; shaders at `shaders/{gtao.wgsl, ssgi*.wgsl, ssr.wgsl}`.

### Cold paths
- **Shader / pipeline creation**: cached via `PipelineCache` (`pipeline_cache.rs`) and `bind_group_cache.rs` (generation-tracked). First-launch cold-start cost mitigated by disk-persisted Vulkan/DX12 pipeline cache.
- **glTF / OBJ loading**: feature-gated; loaded at scene/asset boundaries, not per-frame.
- **Material library upload**: per material-set change; texture array build is O(layers × resolution²).
- **Terrain chunk upload**: per chunk-update; CPU-side splat bake + GPU upload through `Renderer::upload_terrain_chunks` (initial) or `update_terrain_chunk` (incremental, via the shared `upload_or_update_terrain_chunk_forward` helper post Real-Fix.B).

### Resource ownership
- **`wgpu::Device` / `wgpu::Queue` / `wgpu::Surface`**: owned by `Renderer` (`renderer.rs:692-694`). Lifetime = renderer lifetime.
- **All render pipelines, bind groups, depth/shadow/HDR textures**: owned by `Renderer` (extensive field list at `renderer.rs:691-…`).
- **Per-frame transients**: allocated through `staging_ring` (`renderer.rs:707`).
- **Cached bind groups**: live in `bind_group_cache.rs`, indexed by generation.
- **Pipeline cache disk file**: persisted via `_pipeline_cache_mgr`'s `Drop` (`renderer.rs:701-703`).
- **GPU timestamps**: `GpuProfiler` is `None` when `TIMESTAMP_QUERY` is unsupported or the timestamp period is not positive. A slot is allocated when the pass is recorded. `resolve_queries` runs on the write encoder after the last timestamp, including after Hi-Z and particles. The staging copy is a later submission. `prove_readback` fails the frame-budget tool before warmup when that path reads zeros or the writes and the resolve do not share one encoder.
- **Terrain chunk forward path**: `Renderer::terrain_forward.chunks: HashMap<ChunkKey, …>` (referenced at `renderer.rs:5755` per Round-6 evidence). Lifetime = renderer lifetime; entries written by upload/update paths.

---

## 10. Testing & Validation

- **Inline `#[cfg(test)]` modules:**
  - `astraweave-render/src/renderer.rs` — see `renderer_tests.rs` sibling file (referenced at `lib.rs:???`); inline tests count not fully enumerated here
  - `astraweave-render/src/mutation_tests.rs` (declared `#[cfg(test)]` at `lib.rs:129`)
  - `astraweave-render/src/animation_extra_tests.rs` (`lib.rs:126`)
  - `astraweave-render/src/renderer_tests.rs` (sibling file)
  - `astraweave-render/src/nanite_gpu_culling_tests.rs` (sibling file)
- **Integration tests (`astraweave-render/tests/`):**
  - Per workspace audit, this crate has multiple `tests/*.rs` files including `coverage_booster_render.rs`, `terrain_splat_pipeline.rs`, `test_terrain_material.rs`, `wave2_*` remediation suites covering camera, decals, HDRI, mesh, scene env, SSAO, terrain quad registry, transparency, weather. Exact count not enumerated here.
- **WGSL shader review:** The CLAUDE.md `shader-wgsl-reviewer` agent serves as the GPU code verifier — since no formal WGSL verification tooling exists at the Miri/Kani level, this agent fills that gap through rigorous pattern-based review. Used when `.wgsl` files are created or modified.
- **Visual validation gate:**
  - The Editor Multi-Tool Architecture Campaign Sub-phase 3 is governed by Andrew-gate visual verification per `EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md §0`. Real-Fix.A/B/C each received partial-PASS verdicts narrowing the defect class. Real-Fix.D pending.
  - The Editor Behavioral Correctness Audit (`docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`, completed 2026-04-05) covered 37 fixes including visual math, data pipeline, undo system, silent failures, integration seams.
- **Mutation testing:** Wave 2 mutation testing campaigns covered renderer with `tests/wave2_*` suites. Specific kill rates not enumerated here.
- **Manual / example validation:** Every render example crate (`hello_companion`, `unified_showcase`, `weaving_playground`, `biome_weather_demo`, `phi3_demo`, `ecs_ai_showcase`, etc.) exercises portions of the pipeline at runtime. The `unified_showcase` example is the canonical end-to-end visual exercise per CLAUDE.md.
- **Test-target API-sync (D-series doc-truth Path-B.1, 2026-06-28).** [Test-only — zero library change.] 8 test targets had drifted from the live API (drift `cargo check --workspace` masks because it does not build test targets); brought current as follows. **Family A — obsolete placeholder-bloom deletion:** the *placeholder* `post::BloomConfig` (which had a `validate()` method and defaults intensity 0.05 / mip_count 5) was removed from the library and **superseded by the real PBR `bloom::BloomConfig`** (`bloom.rs` — energy-conserving 13-tap; no `validate()`, defaults intensity 0.3 / mip_count 6, extra `enabled`/`soft_knee`/`mip_weights` fields). The test sections written against the removed placeholder (`validate()` calls, placeholder-default asserts, placeholder constructions) were **deleted as obsolete** from `wave2_culling_anim_misc_tests`, `wave2_material_post_streaming_remediation`, `mutation_resistant_comprehensive_tests` (its whole `post_mutations` module), and `coverage_booster_render` (the bloom block inside `test_post_fx_types_wave22`) — NOT rewritten (see §11). **Families B–E — straight syncs:** `advanced_post::TaaConfig` (private) → top-level `astraweave_render::TaaConfig`; `CpuMesh` gained `albedo_image`/`texture_source_hint` (constructions take `..Default::default()`); `WeatherFx::update` gained a `Vec3 camera_pos` arg and `InstanceBatch::update_buffer`/`InstanceManager::update_buffers` gained a `&wgpu::Queue` arg; `GBufferFormats.position` was removed (position reconstructed from depth → test reads `.depth`); the terrain layer array grew `8` → `32` (matches `MAX_TERRAIN_LAYERS = 32`, Appendix A #7). After the fixes, `cargo build --workspace --tests --keep-going` returns **0 failures** (was 8 targets / 31 errors).

---

## 11. Open Questions / Parked Decisions

- **macOS GPU skip census.** [Closed on this head.] A non-empty `gpu-skip-reasons.txt` is not success. `scripts/ci/gpu-skip-required-macos.txt` names the default-feature software-only tests profile ci runs, and `check-gpu-skip-log.sh` fails when one of those names is missing. Hardware probes stay off that list because Metal still runs them. `gpu-tests` binaries stay off it because Test full does not pass `--all-features`. `test_backend_detection`, `golden_triangle_render`, and `golden_determinism_check` are on it. An integration test sets `cfg(test)`, and those three request only a software adapter.

- **11 ms frame budget.** [Closed for the heavy scene on the GPU-frame clock.] GTX 1660 Ti Max-Q, Vulkan, `939a21bb`, 1920×1080, 30 warmup + 120 samples, exit 0: `median_gpu_frame_ms=3.688` (11.085 before the cloud-shadow fix), pass sum 3.616, CPU frame 8.821. `budget_11ms=pass`. Lavapipe time is not that record. RTX 4060 Ti, same scene, before the amortization: Vulkan `31d45ffc` 4.252 ms (pass sum 1.050, unstamped remainder 3.202), Vulkan `60c9aa28` 4.234 ms, DX12 `60c9aa28` 2.966 ms. Water is not drawn in the heavy scene (#371). The 3.688 ms capture printed `gpu_cull=cpu_fallback`. `gpu_frame_budget` opts into GPU cull and prints `gpu_cull=gpu` only after the compute dispatch, plus `gpu_cull_visible`. That capture has not been rerun.
- **GPU indirect cull vs per-model group 3.** [Dispatch is behind `set_gpu_cull_enabled`, default off, owner `astraweave-render`.] `draw_into` keeps the CPU frustum until the flag is on. The flag dispatches `IndirectDrawPipeline` and binds each model's group 3 immediately before `draw_indexed_indirect`. `CullingPipeline` stays undispatched: a compacted index list cannot rebind group 3. Bindless shading is still not the forward path. Occlusion does not read `HizPyramid`. The 1660 Ti record at `939a21bb` is still the CPU-frustum frame. The next heavy-scene capture is what accepts the default.
- **Frame graph migration: when does it move from scaffold to command-driver?** [Decisional.] `frame_graph.rs:18-25` documents "full delegation is designed for incremental adoption." The current state — pass nodes delegating back to `Renderer` methods — is intentional. The question is whether/when to migrate command recording into the frame graph itself, and what triggers that work.
- **Lumen GI vs VXGI — coexisting or eventual single choice?** [Decisional.] Both are fully implemented (`lumen.rs` + `distance_field.rs` + `surface_cache.rs` + `final_gather.rs` vs `gi/vxgi.rs` + `gi/voxelization_pipeline.rs`). Whether to keep both as configurable alternatives or to deprecate one has not been recorded in available sources.
- **`RenderMode::FastPreview` long-term disposition.** [Decisional.] Per `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`, FastPreview was targeted for elimination but the enum variant remains at `engine_adapter.rs:25-28` as a transitional fallback. Whether to fully remove it (and what the new fallback path is for weak GPUs / very large scenes) is open.
- **Three material binding paths — eventual consolidation?** [Decisional.] `MaterialManager` (TOML-driven), `MaterialLibrary` (canonical terrain), and `BindlessMaterialSystem` (bindless modern) coexist. Each currently serves a distinct purpose; whether to consolidate to one canonical path is not recorded.
- **Inline `SHADER_SRC` in `renderer.rs:18-…` vs registered shaders.** [Decisional.] The main forward+ pipeline's WGSL is inlined in `renderer.rs` via `concat!` + `include_str!`. Most newer shaders live in separate files registered via `ShaderManager`. Whether to migrate the inline shader to a registered file (enabling hot-reload for the main pass) is an open question with non-trivial blast radius.
- **§7.7 wrapped-component resource identity trap — preventive instrumentation?** [Decisional, surfaced by Editor Multi-Tool Architecture Campaign Sub-phase 3.] Four distinct §7.7 instances have been confirmed during Sub-phase 3. The campaign Status header (commit `e3d07f366`) suggests a CLAUDE.md amendment cycle is pending — potentially elevating the resource-identity rule from "candidate corollary appended to Edit 2" to a first-class top-level Edit. The decisional question is whether to encode the rule architecturally (lint, build-script, or trait-pattern enforcement) rather than relying on documentation alone.
- **`shader-wgsl-reviewer` agent vs formal verification.** [Decisional.] CLAUDE.md describes the agent as filling the Miri/Kani gap for WGSL code. Whether to invest in formal WGSL verification tooling (Naga validation, third-party shader analyzers) for safety-critical shaders is open.
- **Round 8 Andrew-gate decision (h) for Real-Fix.D — Option D-1 vs D-2 vs D-3 — was made via the canonical material library landing 2026-05-08 (commit `7067cc03d` per repo `git log` recent commit list).** Factual portion closed; the decisional follow-up (does the same approach extend to non-terrain material catalogs?) is not currently a campaign agenda item but worth flagging.
- **Dormant parallel CSM — deprecate or converge?** [Decided 2026-10-15: defer, then delete. Not a second live CSM.] `shadow_csm.rs` (`CsmRenderer`: 4 cascades, splits 10/50/200/1000), `shadow_quality.rs` (PCSS/Poisson/normal-offset), and `shaders/shadow_sampling.wgsl` stay unwired. The live CSM is `renderer.rs` plus `shaders/shadow_common.wgsl` (invariant 20). Deletion waits on a parity pass for the quality features the dormant tests still cover. Tracked as GitHub #238 (`deferred-failure`). The other dormant render passes from the same decision are #226–#237, listed in `docs/current/RENDER_SKY_CAMERA_ARCHAEOLOGY.md`.
- **Statics casting onto terrain** [Recorded follow-up from L.3's ratified scope.] The caster passes are additive-ready (statics already draw via `shadow_pipeline` in the same per-cascade passes); the open work is content-side — which statics exist in terrain scenes (T.3 scatter) and the perf budget for their draws. Also in this family: the `render()` gate asymmetry (missing `shadows_enabled` check) and the skinned shader's missing sentinel gate (L3_OUTCOME §9).
- **Does the real PBR `bloom::BloomConfig` warrant dedicated test coverage?** [Test-authoring item, surfaced by D-series Path-B.1 2026-06-28.] The bloom test sections deleted in Path-B.1 (§10) covered the *removed* placeholder `post::BloomConfig` (the one with `validate()` + 0.05/5 defaults), **not** the surviving energy-conserving `bloom::BloomConfig` (`bloom.rs:23`), which today has no test coverage beyond a derived `Default`. Authoring fresh coverage for the real config (default values 0.3/6, the `mip_weights[8]` curve, `BloomPass` resource lifecycle) is a future test-authoring task. It was explicitly out of scope for Path-B.1, which removed stale tests but did not author new ones.

---

## 12. Maintenance Notes

**Update this doc when:**
- A new top-level module is added to `astraweave-render/` (touch §5)
- The `Renderer` struct gains or loses major fields, especially `render()` / `draw_into()` signatures (§2, §3, §6)
- A new §7.7 wrapped-component resource identity instance is confirmed (§6 trap list, §7 decision log if a Real-Fix lands)
- A new shader file is added to `astraweave-render/shaders/` (§5, possibly §3 if it introduces vocabulary)
- The frame graph transitions from scaffold to command-driver (§7 decision log, §11 question)
- A material binding path is consolidated or removed (§5, §6, §11)
- The Fix 27 Unified Pipeline Campaign or Editor Multi-Tool Architecture Campaign advances a sub-phase or closes
- The `RenderMode::FastPreview` variant is removed (§5, §6, §11)
- The water render path changes — `water.rs` / `water.wgsl` / `run_water_pass` / `WaterUniforms` layout / the weave or HDR-overlay seams (§2.6, §3, §5 water row, §8 invariants 14–18) — **AND** mirror the change into [`docs/architecture/water.md`](./water.md), the canonical end-to-end water trace
- A decision in §7 is superseded by new code or audit

**Verification process:**
- Spot-check the pipeline diagrams in §2 against `renderer.rs::render` (around line 4708), `renderer.rs::draw_into` (around line 5234), `frame_graph.rs::build` (in `frame_graph.rs:80+`), and `renderer.rs::run_water_pass` (`renderer.rs:4679-4768`) for §2.6.
- Verify the file map in §5 against `astraweave-render/src/lib.rs` `pub mod` declarations (lines 34-178) and `pub use` re-exports (lines 139-279).
- Verify invariants in §8 against actual code locations cited.
- Check `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header for any new sub-phase or §7.7 instance landing.
- Update metadata commit hash and date after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **All scene rendering goes through `astraweave_render::Renderer`.** Per CLAUDE.md Scope Discipline ("Never build a second implementation of a logical system that already exists — rendering path, vertex format, material pipeline, scheduler, tonemap chain, scene serializer") and per the Fix 27 Unified Pipeline Campaign. The editor wraps the Renderer via `EngineRenderAdapter`; do not introduce a third path.
2. **§7.7 wrapped-component resource identity trap is the single most-cited failure mode in this layer.** Four confirmed instances during Sub-phase 3 Mediator Brush diagnostics across depth-target, mesh-data, texture-attribute-set, and UI-vs-renderer-capacity boundaries. Before adding a new wrapper layer around `Renderer`, ask: "do I create a parallel resource of the same logical role?" If yes, design the unification at the boundary, not in code that reads its own resource.
3. **Frame graph is scaffolded, not command-driving.** Pass nodes delegate GPU command recording back to `Renderer` methods. Don't try to record commands directly inside `frame_graph.rs` pass nodes today.
4. **Three material binding paths coexist intentionally.** `MaterialManager` (TOML), `MaterialLibrary` (canonical 32-layer terrain), `BindlessMaterialSystem` (bindless modern). Each serves a different purpose.
5. **Per-cascade shadow uniform buffers are deliberate.** Do not "simplify" to a single shadow uniform buffer — the comment at `renderer.rs:752-757` documents the `queue.write_buffer` race.
6. **`Renderer::draw_into` accepts an optional depth view.** Editor callers pass their own depth target so overlays and engine terrain share depth. Pass `None` only if the caller does not need depth coherence.
7. **`MAX_TERRAIN_LAYERS = 32`.** Anything terrain-material-related crosses the canonical material library — see `docs/architecture/terrain_materials.md`.
8. **`#![deny(unsafe_code)]` on the crate root.** Don't ship `unsafe` to `astraweave-render`. Use `wgpu`'s safe API or push the `unsafe` boundary into a different crate.

**Files you'll most likely touch:**
- `astraweave-render/src/renderer.rs` — the canonical aggregator (high blast radius)
- `astraweave-render/src/lib.rs` — `pub mod` + `pub use` (always check before adding a new public type)
- `astraweave-render/src/material*.rs` — material system (choose the right path)
- `astraweave-render/src/frame_graph.rs` + `graph.rs` — DAG topology
- `astraweave-render/shaders/*.wgsl` — shader programs (invoke the `shader-wgsl-reviewer` agent on changes per CLAUDE.md)
- `tools/aw_editor/src/viewport/{renderer.rs, engine_adapter.rs}` — editor entry into the pipeline

**Files you should NOT touch without strong reason:**
- `astraweave-render/src/renderer.rs:18-…` `SHADER_SRC` inline WGSL — historical inline shader for the main pass. Adding to it bloats the already-huge file; if changing the main pass shader, consider migrating to a registered file (see §11).
- `astraweave-render/src/renderer.rs:752-757` per-cascade shadow uniform buffers — don't "simplify"; the race is real.
- `astraweave-render/src/material_library.rs` — canonical 32-layer terrain material registry; changes affect the editor, the splat builder, the shader, and the renderer simultaneously.
- `astraweave-render/src/terrain_material_manager.rs` — feature-gated forward terrain pipeline, see `docs/architecture/terrain_materials.md`.
- `tools/aw_editor/src/viewport/engine_adapter.rs:25-28` `RenderMode::FastPreview` — Fix 27 residue. Removing it has consequences (see §11).
- `tools/aw_editor/src/viewport/terrain_splat.rs` — superseded reference material; do not extend.

**Common mistakes when changing this system:**
- **Mistake**: Adding a new editor-side render path "just for previewing" something quickly.
  **Why wrong**: Fix 27 exists explicitly because dual pipelines diverged across 12 dimensions. CLAUDE.md Scope Discipline forbids this. Use `Renderer::draw_into` + an editor overlay hook instead.
- **Mistake**: Creating a wrapper around `Renderer` that holds its own depth target, terrain chunks, splat textures, or material library.
  **Why wrong**: §7.7. The wrapper's resources won't reflect the underlying `Renderer`'s state (or vice versa). Either share resources via cross-crate API changes (Real-Fix.A pattern) or unify at the boundary (Real-Fix.C pattern).
- **Mistake**: Routing initial-upload through one code path and incremental-update through another.
  **Why wrong**: §7.7 at the data-routing layer (Real-Fix.B pattern). Extract a shared helper called by both.
- **Mistake**: Writing to one attribute (`vertex.material_ids`) and reading from another (`vertex.biome_weights_0/1`) of the same logical role.
  **Why wrong**: §7.7 intra-component sub-variant (Real-Fix.C pattern). Unify into one canonical attribute set.
- **Mistake**: Sizing UI capacity and renderer capacity to different values for the same logical role.
  **Why wrong**: §7.7 UI-vs-renderer capacity boundary (Round-8 / Real-Fix.D pattern). Share the capacity from a single canonical source.
- **Mistake**: Adding a new shader as inline WGSL inside `renderer.rs`.
  **Why wrong**: That file is already 7,800 LoC and the `SHADER_SRC` inline block is historical. New shaders go in `shaders/*.wgsl` and are registered via `ShaderManager` or `include_str!`'d at pipeline creation.
- **Mistake**: Skipping the `shader-wgsl-reviewer` agent on a new `.wgsl` file.
  **Why wrong**: CLAUDE.md mandates this for `.wgsl` changes. There is no Miri/Kani equivalent for shaders; the agent is the safety net.

---

## Appendix B: Historical context

The render system grew in three eras.

The earliest era produced the core `Renderer` aggregator and the inline `SHADER_SRC` main-pass shader in `renderer.rs`. This established the single-aggregator pattern and the wgpu device/queue/surface ownership.

The middle era added subsystems in phase-numbered waves (visible in module comments: Phase PBR-E/-F for materials, Phase 2 for clustered forward and CSM, Phase 3 for volumetric clouds, Phase 5 for Lumen GI + impostor bake, Phase 6 for god rays + volumetric fog, Phase 7 for particles, Phase 8 for atmosphere, Phase 9 for IBL/BRDF LUT). Each phase brought new shader files and new `pub use` re-exports at the crate root. The `frame_graph.rs` declarative DAG scaffolding was added as part of this phase but kept in delegated mode pending incremental migration.

The recent era is dominated by **two campaigns**:

1. **Fix 27 Unified Pipeline Campaign** (planned 2026-04-05, `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`). The editor maintained a `FastPreview` path independent of the engine renderer — its own PBR shader, its own CSM, its own BRDF LUT, its own IBL, its own glTF loader, its own tonemap. Twelve dimensions of divergence. The campaign re-routed all scene rendering through `astraweave_render::Renderer` and formalized editor overlay injection. Per CLAUDE.md current status: "structural phase complete (astraweave-render non-optional, entity rendering through engine); deeper unification (shadow/IBL/post-processing alignment) ongoing." `RenderMode::FastPreview` survives as transitional residue.

2. **Editor Multi-Tool Architecture Campaign Sub-phase 3 Mediator Brush** (2026-05-04 onwards, `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md`). Eight diagnostic rounds across 4 days converged on a single architectural anti-pattern that recurs at every editor↔renderer boundary: when component A wraps component B and both manage resources of the same logical role, A's reads don't reflect B's writes. Round 5 confirmed it at the depth-target layer (Real-Fix.A `0f569d212`). Round 6 confirmed it at the mesh-data layer (Real-Fix.B `eaaa53433`). Round 7 confirmed it at the texture-data attribute-set layer (Real-Fix.C `ded9a0457`). Round 8 confirmed it at the UI-vs-renderer-capacity boundary (Real-Fix.D 2026-05-08, commit `7067cc03d`). Four distinct layers, four distinct fixes, one canonical anti-pattern — now codified as **§7.7 structural axiom** of the editor's resource-management at every architectural boundary. The campaign's working hypothesis is that a CLAUDE.md amendment cycle is pending to elevate the resource-identity rule from a corollary to a first-class top-level Edit.

The Sub-phase 3 work pattern — multi-round instrument-and-narrow with single-concern sessions — is itself canonical for this layer going forward.
