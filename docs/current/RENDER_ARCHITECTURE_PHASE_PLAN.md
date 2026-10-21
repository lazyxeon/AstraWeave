# Rendering architecture phase — plan

Status: **steps 1–7 are on this branch**. Step 8 (GTX 1660 Ti capture) is not in this series. The inventory table below is the pre-build snapshot. Decision 2 was overridden: one fixed `HizPyramid`, not a second occlusion pyramid. `draw_into` builds that pyramid only when `sample_hiz` is set. Occlusion, SSR, and SSGI do not sample it.

## What landed

1. `draw_into` reuses instance and visible-list scratch. After warmup the lavapipe recording floor is `[119, 116, 115, 116, 115, 115]` (`tests/frame_alloc.rs`), the same six numbers on five validation-on runs and five validation-off runs. Spread inside a run is 4. The gate is min 100, max 130, spread 4: the ceiling is 11 above the high sample, and one extra pass (about 18) still fails. Engine scratch adds nothing. The pyramid is not built: no pass samples `HizPyramid::view`. The remaining floor is wgpu-core command lists for the passes that still record (#269).
2. `pipeline_cache::create_render_pipeline` is the production helper. Forward, shadow, and skinned passes pass the real cache. Other production creators pass `None`. `volumetric_fog.rs` and `god_rays.rs` were not edited.
3. Mesh vertex and index buffers are `COPY_DST`. `Renderer::write_mesh` rewrites in place when the counts fit.
4. Bindless registration runs when the device has the binding-array features. Group 3 still shades. Engine metallic-roughness is R = metallic, G = roughness.
5. `HizPyramid` copies `texture_depth_2d` with `textureLoad` into R32Float and downsamples that copy. `draw_into` builds it only when `sample_hiz` is set. `set_sample_hiz(true)` drops any pyramid and the next frame copies the depth view from that `draw_into`, including when the size is unchanged (`tests/hiz_rebind.rs`). The default stays false. SSGI samples scene depth, and no pass calls `HizPyramid::view`. A uniform depth clear stays uniform (`tests/hiz_depth_copy.rs`). CPU `FrustumPlanes` remains the draw filter; `tests/frustum_agree.rs` checks it against `test_aabb`. GPU indirect compaction is not the model loop: each model binds its own group 3. The pyramid is not read back into the sim.
6. `set_particle_emitter` turns on `GpuParticleSystem::update` and `particle_render` from `draw_into`. Emit fills dead slots. Seed and `dt` come from the caller. `tests/particle_present.rs` checks the readback fingerprint. That fingerprint is not in the world hash. With no emitter, the alloc samples stay on the floor above.
7. `TextureStreamingManager::tick` uploads at most one decoded image per frame from a ring reserved in `new`. A tokio worker decodes off the sim tick. `request_streamed_albedo` keeps a 1×1 fallback bind until the image is resident. Load failures are recorded, not discarded. `tests/stream_fallback.rs` uses the same gate as `frame_alloc`: min 100, max 130, spread 4.

## Follow-ups after steps 1–7

### Particles repeat for one seed

`tests/particle_present.rs` runs seed 7 twice and seed 9 twice. Each call is a new device and two updates. Buffers are zero-filled at creation. The emit and update shaders have no atomics. Seed and `dt` are uniforms. Seed 9 matching seed 7 is a failure. Seed 9 differing from a second seed-9 run is a failure. An earlier report said "seed 9 does not" and meant "does not match seed 7".

### Allocation floor (#269 correction)

The unused pyramid is no longer recorded. Lavapipe samples are `[119, 116, 115, 116, 115, 115]` on five validation-on runs and five validation-off runs. Spread inside a run is 4. The gate is min 100, max 130, spread 4. The ceiling is 11 above the high sample. One extra pass is about 18 allocations, so that still fails. The floor is 15 under the low sample, so a dropped pass fails too. `update_instances` stays at 0.

The earlier jump to `[219, 217, 217, 216, 217, 216]` was `HizPyramid::build`: one compute pass per mip, six passes at 64×64, about +108 allocations. No `draw_into` pass samples `HizPyramid::view`, so those passes were wgpu command lists with no reader. The default stays false. `set_sample_hiz(true)` drops any pyramid and the next frame copies that frame's depth view. Barriers are still pass-scoped, so a future consumer still pays one pass per mip.

What remains is wgpu-core 25 `BasePass::new` for the passes that still record. Current wgpu master still does `commands: Vec::new()` per pass. Reusing the command encoder does not remove those allocations. The encoder is created before `reset_allocs`.

`set_sample_hiz(true)` is the switch. It drops any pyramid, and the next frame copies that frame's depth view. The default stays false, so this gate does not include the six mip passes.

Plan, owner `astraweave-render`:

1. When a pass starts sampling the pyramid, call `set_sample_hiz(true)` and expect the gate to rise by about six passes.
2. Merge the live shadow cascades into one layered depth pass (`render_layer` / `gl_Layer`) if the residual floor has to drop further. Do this without a second CSM.
3. Do not fork wgpu-core for a command recycler. Upstream has not grown one.

### GPU culling after bindless shading

Owner: `astraweave-render`. Not done in this change. Group 3 is still what the forward shader samples, and each model has its own group 3. `CullingPipeline` can write indirect args, but one indirect buffer cannot bind those groups.

Order:

1. A forward-shader permutation samples the bindless array when `supports_bindless` is on and the caller has not forced it off. Group 3 stays the compiled fallback. One shared bind group for every bindless model.
2. Per model, a power-of-two indirect command (`index_count`, `instance_count`). `CullingPipeline` writes `instance_count` from the same AABBs as `FrustumPlanes::test_aabb`. The previous frame's `HizPyramid` may zero `instance_count`. The first frame skips occlusion. The CPU does not read the list back.
3. The model loop binds the mesh and the shared bindless group, then `draw_indexed_indirect`. When bindless is off, the loop keeps today's CPU `test_aabb` and `draw_indexed`.
4. `gpu_frame_budget` prints `gpu_cull=cpu_fallback` until step 3 is the draw path.

### Step 8 capture

`cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --glb "assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Rogue.glb"`

The binary is 1920×1080, 30 warmup frames, and the median of 120 frames. `--scene heavy` places 64 instances of that GLB on an 8×8 grid, turns shadows on, and uploads one terrain chunk when `terrain-splat-arrays` is enabled. Showcase IBL is `IblQuality::Medium`. Particles stay on. Volumetrics are not called. It prints `median_draw_into_ms` (CPU encode), `median_cpu_frame_ms` (encode plus submit and the timestamp wait), `median_gpu_pass_sum_ms`, and `median_gpu_frame_ms`. `budget_11ms` is `pass` when that GPU-frame median is ≤ 11, `fail` when it is over, and `not_decided` only when the frame bracket was not measured. Lavapipe time is not the min-spec record. The min-spec record is below.

This phase is the rendering slice of the 2026-10-15 user directives. It does not start the weaving, AI, traversal, or fluid directives. Those stay out of scope until their own archaeology lands.

## What already exists

Checked against `astraweave-render` on this branch. "Live" means `Renderer::draw_into` or `Renderer::render` calls it. "Dormant" means the type compiles and tests construct it, and no production caller does.

| Need | What is in the tree | Live? |
|---|---|---|
| Bindless materials | `material_bindless.rs`: `BindlessMaterialSystem`, `supports_bindless`, `required_features`. Texture binding array plus a 64-byte `GpuMaterialEntry` SSBO. Shader `shaders/bindless_material.wgsl`. Requires `TEXTURE_BINDING_ARRAY` and `SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING`. | No. Callers are the module's own GPU tests. `draw_into` binds group 3 per primitive (`ModelSurfaceMaps`). |
| Pipeline creation | `create_render_pipeline` is called inline in `renderer.rs`, `environment.rs`, `ibl.rs`, `water.rs`, `terrain_material_manager.rs`, `shadow_csm.rs`, `grass_blade.rs`, `impostor_bake.rs`, `skinning_gpu.rs`. `pipeline_cache.rs` already wraps `Device::create_pipeline_cache` and returns `None` when `PIPELINE_CACHE` is missing. `Renderer` holds `_pipeline_cache_mgr` so the cache lives until drop. | The cache object is live. The descriptors are not created through one function, and most descriptors do not pass the cache in. |
| Compute particles | `gpu_particles.rs`: ping-pong storage buffers, an update compute pipeline, an emit pipeline that copies the input particle. `particle_render.rs` is a separate billboard pipeline. `particle_forces.rs` extends the same system and is also unwired. | No. `renderer.rs` does not name either module. Tests in `coverage_booster_render.rs` construct `GpuParticleSystem`. |
| Frustum culling | `culling.rs` `FrustumPlanes::from_view_proj` plus `test_aabb`. `draw_into` and the shadow pass call that on the CPU for models that have an AABB. `CullingPipeline` and `IndirectDrawPipeline` in the same file compact visible instances on the GPU and write indirect args. | CPU AABB tests are live. The compute pipeline and indirect draws are not. `renderer.rs` has no `draw_indirect`. |
| Occlusion / Hi-Z | `hiz_pyramid.rs` builds a min-depth mip chain for SSR and SSGI ray marching. `shaders/hiz_pyramid.wgsl` declares `texture_2d<f32>` and uses `textureLoad`. `nanite_gpu_culling.rs` runs frustum, occlusion, and backface culls against its own Hi-Z and is covered by `nanite_gpu_culling_tests.rs`. | Neither pyramid is built from `draw_into`. The Nanite pipeline is a second geometry path. The SSR pyramid's depth binding is the filterable-float class tracked in #230 and #231. |
| Async streaming | `texture_streaming.rs` (`TextureStreamingManager`, LRU, priority, tokio channel). `virtual_texture.rs` (page cache). `residency.rs` (GPU residency). `texture.rs::load_texture_async`. | No production caller outside tests. The editor and `hello_companion` upload textures up front. |
| Runtime mesh change | `create_mesh_from_full_arrays` creates vertex and index buffers with `VERTEX` and `INDEX` usage only. No `COPY_DST`. `create_mesh_from_cpu_mesh` allocates four intermediate `Vec`s and a fifth interleaved `Vec` on every call. Instance buffers already use `COPY_DST` and grow by powers of two (`set_external_instances`). | A deformed or destroyed mesh is a new allocation, not a rewrite of the existing buffer. |
| Frame time | `gpu_profiler.rs` records up to 16 timestamp pairs when `TIMESTAMP_QUERY` exists and the period is positive. `draw_into` allocates a slot when the pass begins. `gpu_frame_budget` reads that sum after `poll(Wait)`. There is no 11 ms gate. | The profiler is live on a device that supports timestamps. This environment has no GTX 1660 Ti. Lavapipe time is not a frame budget. |
| Heap traffic in the frame | `astraweave-ecs` `CountingAlloc` plus `astraweave-profiling` `alloc-counter` already count process-wide allocations when the feature is on. `draw_into` itself does not `collect` a fresh `Vec` per model, but `update_instances` does, and mesh upload does. | No test asserts zero allocations across a `draw_into`. |

The trace at `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` marks several of these rows Active. That means the module is compiled. It does not mean `draw_into` calls it.

## Decisions the plan is making

1. Extend the live forward renderer. Do not turn `NaniteCullingPipeline` into the scene renderer. Nanite stays a dormant virtualized-geometry path until a later phase says otherwise.
2. One culling owner. CPU `FrustumPlanes` stays the draw filter and the reference. There is one `HizPyramid`. It copies `texture_depth_2d` into R32Float with `textureLoad`. `draw_into` builds it only when `sample_hiz` is set, from the depth view of that frame. Enabling the flag rebuilds against the current view. Occlusion, SSR, and SSGI do not sample it. SSGI reads scene depth. `CullingPipeline` remains the GPU frustum implementation and is not dispatched from `draw_into`, because a compacted indirect list cannot bind a different group 3 per model. Visibility is not read back into the sim.
3. One material fallback. Bindless is selected when `supports_bindless` is true. Otherwise the current group-3 bind groups stay. Both paths read the same `ModelSurfaceMaps` bytes. Bindless `GpuMaterialEntry` calls its packed map ORM. The engine map is R = metallic, G = roughness. The upload converts once, in `pbr_maps`, and the bindless shader samples that same layout. No third packing.
4. Particles are presentation. `EmitterParams.random_seed` and `delta_time` come from the caller. The shader's xorshift is not a sim RNG. Particle state does not write ECS, nav, or physics. A weave that must move the world does that on the CPU with `SimRng`, and the GPU only draws the result.
5. GPU culling does not write simulation state. Visibility is a draw filter. The replay hash does not include it.
6. Mesh mutation writes the buffer the sim already committed. The CPU `CpuMesh` (or the physics/voxel mesh that produced it) is the source of truth. The GPU buffer is a copy. If the vertex count fits, `queue.write_buffer` into a `COPY_DST` buffer. Growth is power-of-two and happens when the count changes, not every frame.
7. Streaming does not invent a second asset crate. `TextureStreamingManager` is the residency owner. Bytes still come from the existing image and glTF loaders. A failed load stays an error with context. The last resident image, or the 1×1 fallback already used for missing maps, remains bound. No `.ok()` on the load result.
8. Pipeline creation becomes one function in `pipeline_cache.rs`: shader module, layout, targets, primitive state, and the optional cache. Existing call sites switch to it. When `PIPELINE_CACHE` is absent, creation is the same `create_render_pipeline` with `cache: None`. That is the fallback. No second pipeline type.

## Build order, after approval

Each step is one commit series, with `cargo check` and the tests named here. Later steps do not start while an earlier step's invariant is open.

1. **Frame scratch and the allocation gate.** Give `Renderer` reusable scratch for instance upload and AABB lists. `update_instances` stops allocating when the instance count fits. Add a test, feature `alloc-counter`, that runs `new_headless`, warms 2 frames, then asserts `draw_into` performs zero heap allocations for a fixed scene of N models. This is the CI budget check that GitHub runners can actually enforce. Lavapipe is not asked to render in 11 ms.

2. **Pipeline function.** Move the forward, shadow, and skinned `create_render_pipeline` calls in `renderer.rs` onto the helper. Pass `_pipeline_cache_mgr`'s cache when it exists. Leave water, IBL, and terrain on their current calls until this helper has one production caller and a unit test that the fallback path still creates a pipeline without `PIPELINE_CACHE`. Then move the other files. Do not touch `volumetric_fog.rs` or `god_rays.rs` while PR #205 owns them.

3. **Mutable meshes.** Add `COPY_DST` to mesh vertex and index buffers. Add `Renderer::write_mesh(name, cpu)` that rewrites in place when the counts fit and reallocates only when they do not. Update the model's AABB in the same call. A test deforms a quad, draws, deforms again within capacity, and checks the buffer was not recreated. Destruction of a mesh is the same write with the post-fracture vertex list, not a permanent "static after upload" flag.

4. **Bindless with fallback.** At device creation, record `supports_bindless`. When true, upload `ModelSurfaceMaps` into the binding array and write a `GpuMaterialEntry`. When false, keep today's bind groups. One test on a device that lacks the features (or a forced-off flag) still draws via group 3. One test on a device that has them checks the SSBO index. The forward shader gains a permutation, selected at pipeline creation, not a second material system.

5. **GPU frustum, then Hi-Z occlusion.** Feed the same AABBs `FrustumPlanes::test_aabb` uses into `CullingPipeline`. Draw indirect from the compacted list. CPU cull remains the path when compute is unavailable, and a test checks both paths agree on a fixed set of boxes. Occlusion is a second compute pass over the previous frame's depth, copied into an `R32Float` pyramid with `textureLoad` from a `texture_depth_2d` source. Do not sample the depth attachment as filterable float. The first frame has no pyramid and skips occlusion. Results are not read back into the sim.

6. **Compute particles.** Call `GpuParticleSystem::update` from `draw_into` only when the caller has enabled an emitter. Draw with `particle_render`. Finish or delete the emit shader that currently copies input to output; a copy is not emission. Seed and `dt` are arguments. No `Instant`. A determinism test runs two updates with the same seed and `dt` and compares the staging readback only as a presentation fingerprint. That fingerprint is not in the world hash.

7. **Streaming.** `TextureStreamingManager` requests by camera distance, with a fixed number of uploads per frame from a pre-sized ring. `draw_into` only binds what is already resident. The worker task is tokio, off the sim tick. A test queues one texture, ticks until resident, and checks the frame that bound the old fallback did not allocate.

8. **1660 Ti capture.** Recorded 2026-10-17 on commit `939a21bb`, GTX 1660 Ti Max-Q, Vulkan, heavy scene, 1920×1080, 30 warmup + 120 samples, exit 0, timestamp self-test ok. `median_gpu_frame_ms=3.688` (11.085 before the cloud-shadow fix), `median_gpu_pass_sum_ms=3.616`, `median_cpu_frame_ms=8.821`. Pass medians: `cloud_shadow` 0.323, `main_render` 2.207, `hiz` 0.275, `shadow_cascade_0` 0.201, `shadow_cascade_1` 0.172, `shadow_cascade_2` 0.180, `shadow_cascade_3` 0.178, `sky` 0.096, `post` 0.086, `particles` 0.030, `water` 0.003. `budget_11ms=pass` on `budget_basis=gpu_frame`. Water is not drawn (`water_drawn=no`, #371). `gpu_cull=cpu_fallback` because group 3 is per model (#372). RTX 4060 Ti, same heavy scene, before that amortization: Vulkan `31d45ffc` frame 4.252 ms, pass sum 1.050, unstamped remainder 3.202; Vulkan `60c9aa28` frame 4.234, pass sum 0.886, CPU frame 7.819; DX12 `60c9aa28` frame 2.966, pass sum 0.610, CPU frame 7.097.

## What this phase will not do

- It will not merge.
- It will not edit `volumetric_fog.rs` or `god_rays.rs`.
- Atmosphere, SSGI, TAA, DoF, temporal upscale, and the cloud raymarch are gated presentation passes. They stay off in `draw_into` unless a setter enables them. #227 and #228 stay on PR #205.
- It will not feed GPU readback into the replay hash.
- It will not add a second renderer, a second culling AABB layout, or a second texture cache beside `TextureStreamingManager`.
- It will not claim an 11 ms frame time without the 1660 Ti capture in step 8.

## Review before any code

The build order above is the proposal. Step 1 is the first code, and only after this plan is accepted. If the occlusion pyramid should instead be the existing `HizPyramid` once its depth binding is fixed, say so before step 5. If particles must affect gameplay, that is a CPU sim with a GPU draw, not `GpuParticleSystem` writing the world.
