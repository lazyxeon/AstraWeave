# Wiring campaign

Inventory of systems that compile and are not on the default runtime path. Base is `main` at `9fb72b85`. One pull request per system.

A wiring pull request may add a runtime caller and a quality setting. The default stays off until `gpu_frame_budget --scene heavy` with that pass's flag prints `median_gpu_frame_ms` at or under 11 on a GTX 1660 Ti. The user runs those captures. An RTX 4060 Ti debug run is not that column. Lavapipe time is not that column. No pass is turned on from an estimate.

The published heavy-scene GPU frame is 3.688 ms (`939a21bb`, GTX 1660 Ti Max-Q, Vulkan, 1920×1080, 30 warmup + 120 samples). That number is the baseline row. It is not a sum we can add estimates to.

Presentation passes must not write simulation state and must not read a wall clock or an unseeded RNG.

#371 (heavy-scene water) and #372 (GPU culling CPU fallback) belong to the render workstream. This campaign does not edit them.

## What 7.397 ms is

The same heavy scene was 11.085 ms before the cloud-shadow amortization and 3.688 ms after it. The difference, 7.397 ms, is that `CloudShadowPass` change. `gpu_frame_budget` does not call `set_cloud_raymarch_enabled`, so 7.397 ms is not a `VolumetricCloudsPass` measurement. The raymarch cost is unmeasured.

## Measured column

A cell is filled only by a GTX 1660 Ti heavy-scene run whose validation scope is clean and whose command line enables exactly one gated pass. `gpu_frame_budget` rejects a second flag. It also rejects `--ssgi`, `--taa`, `--upscale`, `--dof`, and `--clouds` before drawing: those paths sample or `LoadOp::Load` an uncleared texture, or they never write HDR, so the scope fails and no median is printed.

`--atmosphere` is the one flag that can finish. It replaces `SkyRenderer` (the HDRI sky clear and draw). The printed `median_gpu_frame_ms` is that whole frame. It is not a number to add to 3.688 ms.

| System | Flag | `median_gpu_frame_ms` on GTX 1660 Ti heavy | Default |
|---|---|---|---|
| Baseline (HDRI sky, no gated pass) | none | 3.688 | The captured scene |
| Atmosphere, replacing `SkyRenderer` | `--atmosphere` | | Off until this cell is ≤ 11 |

Fog and god rays are already on in Aravath (`set_volumetric_fog_config`, `set_god_ray_config`, unless `--no-volumetrics` or `--safe`). They are not in the 3.688 ms capture. Their heavy-scene medians are also blank. Do not add them into an unmeasured sum.

## Already on a runtime caller

| System | State | Owner | Default |
|---|---|---|---|
| Volumetric fog (`VolumetricFogPass`) | `aravath_runtime` calls `set_volumetric_fog_config` with `gameplay_config` unless `--no-volumetrics` or `--safe`. | `astraweave-render`, `aravath_runtime` | On in the game. Issue #227. Heavy-scene median is unmeasured. |
| God rays (`GodRayPass`) | `aravath_runtime` calls `set_god_ray_config` (half resolution, 32 samples) on that same path. | `astraweave-render`, `aravath_runtime` | On in the game. Issue #228. Heavy-scene median is unmeasured. |
| Cloud shadows | `CloudShadowPass` is built in `Renderer::new`. The 3.688 ms capture includes it at 0.323 ms. | `astraweave-render` | On. This is not `VolumetricCloudsPass`. |

## Prerequisite before SSGI and TAA

`record_gated_presentation` allocates `gated_normal` and `gated_velocity` when SSGI, TAA, or temporal upscale is on. Nothing writes those textures. SSGI and TAA sample them. Wiring either pass waits on a pass that writes both targets from the scene. That prerequisite is its own pull request, and it stays off the default until its own heavy-scene median is filled in.

| Order | System | Current state | Wiring plan | Owner crate | Exit test |
|---|---|---|---|---|---|
| 0 | Scene normal and velocity | Empty `Rgba16Float` and `Rg16Float` targets. No draw call stores a normal or a motion vector into them. | A presentation pass writes both before SSGI or TAA sample them. Quality default off. | `astraweave-render` | With the pass enabled, a headless frame's normal and velocity textures are not the cleared value. With it disabled, SSGI and TAA stay off. Replay hashes unchanged. |

## Wire next, one PR each

Defaults stay off. A later pull flips a default only after the measured cell above is ≤ 11 on a GTX 1660 Ti.

| Order | System | Issue | Current state | Wiring plan | Owner crate | Exit test |
|---|---|---|---|---|---|---|
| 1 | Atmosphere (`AtmospherePass`) | #226 | Flag defaults false. No runtime caller. Aerial depth is `texture_depth_2d` and the shader uses `textureLoad`. Sky-depth pixels copy the scene color while the HDRI sky is skipped, so the Bruneton sky does not reach the frame. | Sky-depth pixels load the Bruneton sky. Aravath calls `set_atmosphere_enabled` from a quality setting whose default is off. | `astraweave-render`, `aravath_runtime` | Default flag false. The quality setting can turn it on. `gpu_frame_budget --scene heavy --atmosphere` is the measurement, not an estimate. Replay hashes unchanged. |
| 2 | Depth of field | #232 | `set_dof_enabled` defaults false. Live shader is `advanced_post.rs` `DOF_SHADER`: `texture_depth_2d`, center sample plus 8 offsets (9 taps). `apply_dof` uses `LoadOp::Load` on a history texture `AdvancedPostFx::new` never clears, and the fragment result stays in that history view. The post pass still reads HDR. `shaders/dof.wgsl` is a different 16-tap `texture_2d<f32>` shader and is not this pass. | Clear or discard the history target on the first frame and make the post pass read the 9-tap output. Then add `--dof`. Not before. | `astraweave-render` | A clean validation scope and a median that includes the 9-tap image. Default stays off until that cell is ≤ 11. |
| 3 | Cloud raymarch (`VolumetricCloudsPass`) | #229 | `execute` samples `cloud_history` and writes `composited_texture`. `copy_cloud_to_history` runs after the sample. `record_gated_presentation` does not copy the composite onto HDR. | Write history before the sample, and copy the composite onto HDR. Then add `--clouds`. Not the 7.397 ms shadow delta. | `astraweave-render` | A clean scope and a median of a frame that shows the composite. Default stays off until that cell is ≤ 11. |
| 4 | SSGI | #230 | `set_ssgi_enabled` defaults false. Depth is `texture_depth_2d`. Normal, velocity, and `ssgi_history` are not written. `copy_to_history` is not called. | After the normal and velocity row, and after history is written before it is sampled. | `astraweave-render` | No `--ssgi` cell until then. |
| 5 | TAA | #231 | `set_taa_enabled` defaults false. `next_frame` and `apply_jitter_to_projection` have no caller. `taa_history` is sampled and never copied. | After the normal and velocity row, with jitter applied and history written first. | `astraweave-render` | No `--taa` cell until then. |
| 6 | Temporal upscale | #233 | `upscale_on` defaults false. It reads the same unwritten velocity target. | After the normal and velocity row. No `--upscale` until then. | `astraweave-render` | Default stays false. |

## Do not add a second pipeline

| System | Issue | What is in the tree | Why it is not a wiring target |
|---|---|---|---|
| `SkyPassNode` | #234 | `frame_graph.rs`. `run` returns `Ok(())`. | Live sky is `SkyRenderer`, or `AtmospherePass` when that flag is on. |
| `weather_system.rs` | #236 | Still compiled. Tests construct `WeatherTransition`. | Live weather is `effects::WeatherFx` and `SceneEnvironment`. A second weather model is not a feature. |
| `shadow_csm.rs`, `shadow_quality.rs`, `shadow_sampling.wgsl` | #238 | Still compiled. | Live cascades are `renderer.rs` and `shaders/shadow_common.wgsl`. |

`VolumetricFogNode`, `BrdfLutPass`, and `weather_gpu.rs` are not in the tree. They are not rows.

## Owned elsewhere

| System | Issue | State | This campaign |
|---|---|---|---|
| Heavy-scene water | #371 | The heavy scene does not draw water. The capture's water pass is 0.003 ms. | Skip. Render workstream. |
| GPU culling CPU fallback | #372 | `gpu_cull=cpu_fallback` on every adapter. | Skip. Render workstream. |
| `AIArbiter`, advanced GOAP, `GoapPlanner` | #334, #383 | Feature-gated. Off `sys_ai_planning` and `sys_spatial_tick`. | Do not flip the features into the tick. `shadow_mode` and GOAP telemetry read `Instant`. The port is #383. Exit: replay hashes unchanged. |
| `recorded_line` | none new | Pure helper in `astraweave-dialogue`. Callers are its tests. | Later dialogue pull. Exit: one non-test caller that does not draw. |

## Order

1. This inventory. `gpu_frame_budget --scene heavy --atmosphere` is the only gated measurement. It replaces `SkyRenderer`. The other gated flags are rejected until their frames are valid.
2. Atmosphere caller and quality setting, default off (#226).
3. Normal and velocity targets, default off.
4. Depth of field, cloud raymarch, SSGI, TAA, temporal upscale. Each default stays off until its measured cell is ≤ 11 on a GTX 1660 Ti.
5. Fog and god rays stay as Aravath already calls them.
6. #234, #236, and #238 are not second implementations.
7. #371 and #372 are not this campaign.
8. #334 and #383 stay off the tick until the port in #383.
