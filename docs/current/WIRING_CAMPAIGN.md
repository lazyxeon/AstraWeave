# Wiring campaign

Inventory of systems that compile and are not on the default runtime path. Base is `main` at `9fb72b85`. One pull request per system, smallest risk first.

A system is done when a runtime caller turns it on at the highest quality tier that keeps the heavy scene under 11 ms. The measured baseline is the GTX 1660 Ti Max-Q heavy scene at `939a21bb`: `median_gpu_frame_ms=3.688`, limit 11, headroom 7.312 ms. This machine is not that GPU. Costs below are either that capture or an estimate scaled from it. Lavapipe time is not a 1660 Ti number.

Presentation passes must not write simulation state and must not read a wall clock or an unseeded RNG.

#371 (heavy-scene water) and #372 (GPU culling CPU fallback) belong to the render workstream. This campaign does not edit them.

## Cost against the 3.688 ms baseline

The cloud-shadow fix dropped the same scene from 11.085 ms to 3.688 ms. The difference, 7.397 ms, is the measured cost of the unamortized volume march on that GPU. It is not a new capture.

| System | Cost | Basis | Default |
|---|---|---|---|
| Heavy scene as captured | 3.688 ms | Measured GPU frame | Stays the baseline |
| Cloud raymarch (`VolumetricCloudsPass`, Medium 48×4) | 7.397 ms | Measured, 11.085 − 3.688 | Off. 3.688 + 7.397 = 11.085, which is over 11. `set_cloud_raymarch_enabled` remains the quality switch. |
| Atmosphere (32-step sky + 16-step aerial, LUT once) | about 1 ms | Estimate. Analytic steps, not the density march. About one eighth of 7.397 ms. | On. 3.688 + 1 ≈ 4.7, under 11. |
| TAA (3×3 neighborhood) | under 0.3 ms | Estimate, next to the measured `post` pass at 0.086 ms | On, when that pull lands |
| Depth of field (16 taps) | under 0.5 ms | Estimate from tap count versus the post pass | On, when that pull lands |
| SSGI (4 rays × 32 steps + denoise) | about 2.5 ms | Estimate. A 2× miss is about 5 ms. 3.688 + 1 + 0.3 + 0.5 + 5 ≈ 10.5, still under 11 if the cloud raymarch stays off | On at the existing 4×32 preset, when that pull lands |
| Temporal upscale | saves time by rendering fewer pixels | Quality, not cost | Off. Native resolution is the higher tier, and the baseline already fits. |
| Fog (80×45×32) and god rays (half res, 32 samples) | about 1 ms and about 0.8 ms | Estimate. Froxels are not a 1080p march | Already on in Aravath. Not in the 3.688 ms capture |

Stack of the defaults above, cloud raymarch excluded: 3.688 + 1 + 0.3 + 0.5 + 2.5 + 1 + 0.8 ≈ 9.8 ms. That is the planning sum, not a new 1660 Ti capture. If a later measurement crosses 11, the next pull lowers that pass's preset instead of deleting it.

## Already on a runtime caller

| System | State | Owner | Default |
|---|---|---|---|
| Volumetric fog (`VolumetricFogPass`) | `aravath_runtime` calls `set_volumetric_fog_config` with `gameplay_config` unless `--no-volumetrics` or `--safe`. | `astraweave-render`, `aravath_runtime` | On in the game. Issue #227. Leave the heavy-scene capture as published until it is remeasured. |
| God rays (`GodRayPass`) | `aravath_runtime` calls `set_god_ray_config` (half resolution, 32 samples) on the same path. | `astraweave-render`, `aravath_runtime` | On in the game. Issue #228. |
| Cloud shadows | `CloudShadowPass` is built in `Renderer::new`. Strength is 4. Amortized cost in the capture is 0.323 ms. | `astraweave-render` | On. This is not the raymarch. |

## Wire next, one PR each

| Order | System | Issue | Current state | Wiring plan | Owner crate | Exit test |
|---|---|---|---|---|---|---|
| 1 | Atmosphere (`AtmospherePass`) | #226 | `record_gated_presentation` runs it only when `atmosphere_on`. No runtime caller sets that flag. Aerial depth is already `texture_depth_2d` / `textureLoad`. Sky pixels currently copy the scene color, and the HDRI sky is skipped while that flag is on, so the Bruneton sky never reaches the frame. | Bind the Bruneton sky into the aerial pass for sky-depth pixels. `Renderer` defaults the flag on. `aravath_runtime` calls `set_atmosphere_enabled(true)`. HDRI sky stays the path when the flag is off. | `astraweave-render`, `aravath_runtime` | A headless renderer starts with atmosphere enabled. Sky-depth pixels read the Bruneton sky. Sim replay hashes unchanged. |
| 2 | SSGI | #230 | `set_ssgi_enabled` defaults false. Depth is `texture_depth_2d`. | Default the existing 4×32 preset on. Runtime caller sets it. | `astraweave-render`, `aravath_runtime` | Default flag true. Device validation submit samples depth. Replay hashes unchanged. |
| 3 | TAA | #231 | `set_taa_enabled` defaults false. History is presentation-only. | Default on. History is not sim state. | `astraweave-render`, `aravath_runtime` | Default flag true. No sim component writes. |
| 4 | Depth of field | #232 | `set_dof_enabled` defaults false. | Default on after the depth view matches the other passes. Keep the creation-error log. | `astraweave-render`, `aravath_runtime` | Default flag true, or the creation error is logged and the frame still presents. |
| 5 | Cloud raymarch | #229 | Flag defaults false. `CloudShadowPass` stays the live shadow. | Leave the default off. The measured 7.397 ms march plus 3.688 ms is over 11. `set_cloud_raymarch_enabled` is the quality switch. | `astraweave-render` | Default flag false. The setter still builds the pass when called. |
| 6 | Temporal upscale | #233 | `upscale_on` defaults false. | Leave the default off. Native resolution is the higher tier and already fits. The setter stays. | `astraweave-render` | Default flag false. Enabled pass still uses a depth-typed binding. |

## Do not wire a second copy

The live owner already exists. A second pipeline is not a feature.

| System | Issue | Live owner | Why it stays unwired |
|---|---|---|---|
| Frame-graph `SkyPassNode` / `VolumetricFogNode` | #234 | `SkyRenderer`, `VolumetricFogPass` | Both `run` methods return `Ok(())`. |
| `BrdfLutPass` | #235 | `IblManager::bake_environment` | Second LUT. |
| `weather_system.rs` | #236 | `effects::WeatherFx` and `SceneEnvironment` | Second weather model. |
| `weather_gpu.rs` | #237 | none in the frame | No runtime caller. Deletion waits on a parity check, not a new call. |
| `shadow_csm.rs`, `shadow_quality.rs`, `shadow_sampling.wgsl` | #238 | `renderer.rs` and `shaders/shadow_common.wgsl` | Second cascade map. |

## Owned elsewhere

| System | Issue | State | This campaign |
|---|---|---|---|
| Heavy-scene water | #371 | Heavy scene does not draw water. Water time in the 3.688 ms capture is 0.003 ms. | Skip. Render workstream. |
| GPU culling CPU fallback | #372 | `gpu_cull=cpu_fallback` on every adapter. | Skip. Render workstream. |
| `AIArbiter`, advanced GOAP, `GoapPlanner` | #334, #383 | Feature-gated. Off `sys_ai_planning` and `sys_spatial_tick`. | Do not flip the features into the tick. `shadow_mode` and GOAP telemetry read `Instant`. The port is #383. Exit: replay hashes unchanged. |
| `recorded_line` | none new | Pure helper in `astraweave-dialogue`. Callers are its tests. | Later dialogue pull. Exit: one non-test caller that does not draw. |

## Order

1. This inventory, with the budget table above.
2. Atmosphere (#226), on by default.
3. SSGI (#230), TAA (#231), depth of field (#232), on by default at the presets in the cost table.
4. Cloud raymarch (#229) and temporal upscale (#233) stay available and off. The table says why.
5. Fog (#227) and god rays (#228) stay on in Aravath.
6. #234–#238 are not second implementations.
7. #371 and #372 are not this campaign.
8. #334 and #383 stay off the tick until the port in #383.
