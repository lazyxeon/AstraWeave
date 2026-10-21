# Wiring campaign

Inventory of systems that compile and are not on the default runtime path. Base is `main` at `8b4caa27`. One pull request per system.

A wiring pull request may add a runtime caller and a quality setting. The default stays off until `gpu_frame_budget --scene heavy` with that pass's flag prints `median_gpu_frame_ms` at or under 11 on a GTX 1660 Ti. The user runs those captures. An RTX 4060 Ti debug run is not that column. Lavapipe time is not that column. No pass is turned on from an estimate.

The published heavy-scene GPU frame is 3.688 ms (`939a21bb`, GTX 1660 Ti Max-Q, Vulkan, 1920×1080, 30 warmup + 120 samples). That number is the baseline row. It is not a sum we can add estimates to.

Presentation passes must not write simulation state and must not read a wall clock or an unseeded RNG.

#371 (heavy-scene water) and #372 (GPU culling CPU fallback) belong to the render workstream. This campaign does not edit them.

## What 7.397 ms is

The same heavy scene was 11.085 ms before the cloud-shadow amortization and 3.688 ms after it. The difference, 7.397 ms, is that `CloudShadowPass` change. `gpu_frame_budget` does not call `set_cloud_raymarch_enabled`, so 7.397 ms is not a `VolumetricCloudsPass` measurement. The raymarch cost is unmeasured.

## Measured column

A cell is filled only by a GTX 1660 Ti heavy-scene run whose validation scope is clean and whose command line enables exactly one gated pass. This binary rejects every gated flag before it builds the scene, including `--atmosphere`, `--ssgi`, `--taa`, `--upscale`, `--dof`, and `--clouds`. Every gated cell stays blank. Every gated default stays off.

| System | Flag | `median_gpu_frame_ms` on GTX 1660 Ti heavy | Default |
|---|---|---|---|
| Baseline (HDRI sky, no gated pass) | none | 3.688 | The captured scene |
| Atmosphere, replacing `SkyRenderer` | `--atmosphere` | | Off. This binary rejects the flag. |

Fog and god rays remain callers in `light_the_world` when volumetrics are on. #395 does not remove those setters. #395 owns the clear, the sky sample, and the sun. `parse_volumetrics_cfg` strips spaces and a `#` comment, then matches the remainder to `volumetrics=off` or `volumetrics=on`. The last matching key wins. A later line that is not one of those two keys leaves the value alone. A missing file stays on. A read or parse error stops startup. `launch_options` applies the file first. `--no-volumetrics` and `--safe` force off. `--volumetrics` forces on, including over a cfg-off file. They are not in the 3.688 ms capture. Their heavy-scene medians are blank. Do not add them into an unmeasured sum.

## Already on a runtime caller

| System | State | Owner | Default |
|---|---|---|---|
| Volumetric fog (`VolumetricFogPass`) | `light_the_world` calls `set_volumetric_fog_config` with `gameplay_config` unless the last matching `aravath.cfg` key is `volumetrics=off`, or the process is started with `--no-volumetrics` or `--safe`. `--volumetrics` forces the pass on. That call remains. #395 does not remove it. | `astraweave-render`, `aravath_runtime` | On in the game. Issue #227. Heavy-scene median is unmeasured. |
| God rays (`GodRayPass`) | `light_the_world` calls `set_god_ray_config` (half resolution, 32 samples) on that same path. That call remains. #395 does not remove it. | `astraweave-render`, `aravath_runtime` | On in the game. Issue #228. Heavy-scene median is unmeasured. |
| Cloud shadows | `CloudShadowPass` is built in `Renderer::new`. The 3.688 ms capture includes it at 0.323 ms. | `astraweave-render` | On. This is not `VolumetricCloudsPass`. |

## Prerequisite before SSGI and TAA

`record_gated_presentation` allocates `gated_normal` and `gated_velocity` when SSGI, TAA, or temporal upscale is on. Nothing writes those textures. SSGI and TAA sample them. Wiring either pass waits on a pass that writes both targets from the scene. That prerequisite is its own pull request, and it stays off the default until its own heavy-scene median is filled in.

| Order | System | Current state | Wiring plan | Owner crate | Exit test |
|---|---|---|---|---|---|
| 0 | Scene normal and velocity | Uninitialized `Rgba16Float` and `Rg16Float` targets. Nothing clears them. No draw call stores a normal or a motion vector. | A presentation pass clears both, draws them, and only then may SSGI or TAA sample them. Quality default off. | `astraweave-render` | With the pass enabled, a headless frame's normal and velocity textures hold the drawn values. With it disabled, SSGI and TAA stay off. Replay hashes unchanged. |

## Wire next, one PR each

Defaults stay off. A later pull flips a default only after the measured cell above is ≤ 11 on a GTX 1660 Ti.

| Order | System | Issue | Current state | Wiring plan | Owner crate | Exit test |
|---|---|---|---|---|---|---|
| 1 | Atmosphere (`AtmospherePass`) | #226 | Flag defaults false. This binary rejects `--atmosphere`. `record_gated_presentation` calls `prepare_frame` with sun `Vec3::Y`, moon `Vec3::NEG_Y`, near `0.1`, and far `200.0`. | #395 is the one caller. It owns the clear, the sky sample, and the sun. The zenith sun on this head is that pull's gap. #226 tracks the pull. Fog and god rays stay on `light_the_world`. Default stays off. | `astraweave-render` | This inventory does not enable the flag. Replay hashes unchanged. |
| 2 | Depth of field | #232 | `set_dof_enabled` defaults false. Live shader is `advanced_post.rs` `DOF_SHADER`: `texture_depth_2d`, center sample plus 8 offsets (9 taps). `apply_dof` writes `AdvancedPostFx`'s texture labeled `"TAA History"` (`history_view`) with `LoadOp::Load`. `AdvancedPostFx::new` never clears it. That texture is not `TaaPass`'s `taa_history`. The post pass still reads HDR. `shaders/dof.wgsl` is a different 16-tap `texture_2d<f32>` shader and is not this pass. | The shared history name is a conflict. DoF and TAA stay at their defaults until those histories are separate, the DoF target is cleared, and the post pass reads the 9-tap output. No `--dof` before that. | `astraweave-render` | Default stays off. No measured cell until the histories are separate. |
| 3 | Cloud raymarch (`VolumetricCloudsPass`) | #229 | `execute` samples `cloud_history` and writes `composited_texture`. `copy_cloud_to_history` runs after the sample. `record_gated_presentation` does not copy the composite onto HDR. | Write history before the sample, and copy the composite onto HDR. Then add `--clouds`. Not the 7.397 ms shadow delta. | `astraweave-render` | A clean scope and a median of a frame that shows the composite. Default stays off until that cell is ≤ 11. |
| 4 | SSGI | #230 | `set_ssgi_enabled` defaults false. Depth is `texture_depth_2d`. Normal, velocity, and `ssgi_history` are not written. `copy_to_history` is not called. | After the normal and velocity row, and after history is written before it is sampled. | `astraweave-render` | No `--ssgi` cell until then. |
| 5 | TAA | #231 | `set_taa_enabled` defaults false. `next_frame` and `apply_jitter_to_projection` have no caller. `TaaPass` samples its own `taa_history` and never copies it. DoF writes a different texture labeled `"TAA History"`. | After the normal and velocity row, with jitter applied and `TaaPass` history written first. The DoF texture is not that history. The name collision blocks a TAA default. | `astraweave-render` | Default stays off. No `--taa` cell until the histories are separate. |
| 6 | Temporal upscale | #233 | `upscale_on` defaults false. It reads the same unwritten velocity target. | After the normal and velocity row. No `--upscale` until then. | `astraweave-render` | Default stays false. |

## Do not add a second pipeline

| System | Issue | What is in the tree | Why it is not a wiring target |
|---|---|---|---|
| `SkyPassNode` | #234 | `frame_graph.rs`. `run` returns `Ok(())`. | Live sky is `SkyRenderer`. `AtmospherePass` is not a second sky on screen. |
| `weather_system.rs` | #236 | Still compiled. Tests construct `WeatherTransition`. | Live weather is `effects::WeatherFx` and `SceneEnvironment`. A second weather model is not a feature. |
| `shadow_csm.rs` | #238 | Still compiled. This is a second cascade map. | Live cascades are `renderer.rs` and `shaders/shadow_common.wgsl`. |
| `shadow_quality.rs`, `shadow_sampling.wgsl` | #238 | Still compiled. This is the PCSS / Poisson filter and `ShadowParamsGpu`. It is not another cascade atlas. | The live cascade path does not sample this filter. |

`VolumetricFogNode`, `BrdfLutPass`, and `weather_gpu.rs` are not in the tree. They are not rows.

## Owned elsewhere

| System | Issue | State | This campaign |
|---|---|---|---|
| Heavy-scene water | #371 | The heavy scene does not draw water. The capture's water pass is 0.003 ms. | Skip. Render workstream. |
| GPU culling CPU fallback | #372 | `gpu_cull=cpu_fallback` on every adapter. | Skip. Render workstream. |
| `AIArbiter`, advanced GOAP, `GoapPlanner` | #334, #383 | Feature-gated. Off `sys_ai_planning` and `sys_spatial_tick`. | Do not flip the features into the tick. `shadow_mode` and GOAP telemetry read `Instant`. The port is #383. Exit: replay hashes unchanged. |
| `recorded_line` | none new | Pure helper in `astraweave-dialogue`. Callers are its tests. | Later dialogue pull. Exit: one non-test caller that does not draw. |

## Order

1. This inventory. `gpu_frame_budget` rejects every gated flag, including `--atmosphere`. Every gated cell stays blank. Defaults stay off.
2. No second atmosphere caller. #395 is that caller, and #226 tracks it.
3. Normal and velocity targets, default off.
4. Depth of field, cloud raymarch, SSGI, TAA, temporal upscale. Each default stays off until its measured cell is ≤ 11 on a GTX 1660 Ti.
5. Fog and god rays remain callers in `light_the_world`. #395 does not remove them.
6. #234, #236, and #238 are not second implementations.
7. #371 and #372 are not this campaign.
8. #334 and #383 stay off the tick until the port in #383.
