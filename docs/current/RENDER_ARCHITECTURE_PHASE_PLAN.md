# Rendering architecture phase — plan

Status: **plan only**. Nothing in this document is approved to build. The sky, IBL, PBR upload, and lavapipe validation work on PR #216 stays as it is.

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
| Frame time | `gpu_profiler.rs` records up to 16 timestamp pairs when `TIMESTAMP_QUERY` exists. PR #205 owns the mapped-readback bug. There is no 11 ms gate and no 1660 Ti capture. | Profiler object can be live. This environment has no GTX 1660 Ti. Lavapipe time is not a frame budget. |
| Heap traffic in the frame | `astraweave-ecs` `CountingAlloc` plus `astraweave-profiling` `alloc-counter` already count process-wide allocations when the feature is on. `draw_into` itself does not `collect` a fresh `Vec` per model, but `update_instances` does, and mesh upload does. | No test asserts zero allocations across a `draw_into`. |

The trace at `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` marks several of these rows Active. That means the module is compiled. It does not mean `draw_into` calls it.

## Decisions the plan is making

1. Extend the live forward renderer. Do not turn `NaniteCullingPipeline` into the scene renderer. Nanite stays a dormant virtualized-geometry path until a later phase says otherwise.
2. One culling owner. CPU `FrustumPlanes` stays the fallback and the reference. `CullingPipeline` becomes the GPU frustum path for the same AABBs. A new occlusion pass consumes a Hi-Z built for culling. `hiz_pyramid.rs` stays the SSR/SSGI pyramid until its depth binding is fixed (#230, #231). It is not the occlusion pyramid.
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

8. **1660 Ti capture, after 1–7.** Scene: 1920×1080, Windows, Vulkan, GTX 1660 Ti class, 30 warmup frames, median of 120 `draw_into` frames. Contents: showcase IBL at `IblQuality::Medium`, one textured GLB through `add_composite_model`, a fixed instance count, cloud shadows at the default, particles on, GPU cull on, volumetrics off. Record `gpu_profiler` pass times. The budget is under 11 ms for the sum of those passes. This VM cannot run that capture. The number is not invented. The CI gate remains the allocation test from step 1, plus lavapipe validation that the new compute passes submit with an empty error scope. A lavapipe millisecond is not the budget.

## What this phase will not do

- It will not merge.
- It will not edit `volumetric_fog.rs` or `god_rays.rs`.
- It will not wire Atmosphere, SSGI, TAA, DoF, or the cloud raymarch. Those stay #226–#233.
- It will not feed GPU readback into the replay hash.
- It will not add a second renderer, a second culling AABB layout, or a second texture cache beside `TextureStreamingManager`.
- It will not claim an 11 ms frame time without the 1660 Ti capture in step 8.

## Review before any code

The build order above is the proposal. Step 1 is the first code, and only after this plan is accepted. If the occlusion pyramid should instead be the existing `HizPyramid` once its depth binding is fixed, say so before step 5. If particles must affect gameplay, that is a CPU sim with a GPU draw, not `GpuParticleSystem` writing the world.
