# Wiring campaign

Inventory of systems that compile and are not on the default runtime path. Base is `main` at `9fb72b85`. One pull request per system, smallest risk first. This document does not wire them.

Presentation passes stay off the simulation tick. They must not read a wall clock or an unseeded RNG, and they must not write simulation state. The heavy-scene GPU frame on a GTX 1660 Ti Max-Q is 3.688 ms with cloud shadows on and volumetrics off (`budget_11ms=pass`). A pass that is not in that measurement stays behind a quality setting whose default matches that scene. Measure before changing the default.

#371 (heavy-scene water) and #372 (GPU culling CPU fallback) belong to the render workstream. This campaign does not edit them.

## Already on a runtime caller

| System | State | Owner | Exit already met |
|---|---|---|---|
| Volumetric fog (`VolumetricFogPass`) | `Renderer::set_volumetric_fog_config` builds the pass. `aravath_runtime` calls it with `gameplay_config`. Default `Renderer` field is `None`. Not in the 3.688 ms heavy scene. | `astraweave-render`, `aravath_runtime` | Greybox calls the setter. Issue #227. Do not enable it on the heavy-scene benchmark. |
| God rays (`GodRayPass`) | `Renderer::set_god_ray_config` builds the pass. `aravath_runtime` calls it. Default field is `None`. | `astraweave-render`, `aravath_runtime` | Greybox calls the setter. Issue #228. Same budget rule as fog. |
| Cloud shadows | `CloudShadowPass` is built in `Renderer::new`. Strength constant is 4. Amortized cost in the heavy scene is 0.323 ms. | `astraweave-render` | Live. Not the raymarch. |

## Wire next, one PR each

Depth slots on several of these passes are still filterable `texture_2d<f32>` against `Depth32Float`. The wiring PR for each one fixes that binding, or it does not enable the pass. Quality default stays off so the 3.688 ms path does not change.

| Order | System | Issue | Current state | Wiring plan | Owner crate | Exit test |
|---|---|---|---|---|---|---|
| 1 | Atmosphere (`AtmospherePass`) | #226 | `record_gated_presentation` runs it only when `atmosphere_on`. No runtime caller sets that flag. Aerial depth sample is not `texture_depth_2d`. | Fix the depth binding. Add a quality value defaulting to off. Aravath reads that value and calls the setter only when the value asks for atmosphere. Do not composite it onto the HDRI sky. | `astraweave-render` | Headless submit with the flag off does not create `AtmospherePass`. A second submit with the flag on records the pass. Sim replay hashes unchanged. |
| 2 | Cloud raymarch (`VolumetricCloudsPass`) | #229 | Flag `cloud_raymarch_on` defaults false. `CloudShadowPass` is the live shadow. The raymarch is a different image. | Quality value default off. Setter already builds the pass. Do not replace cloud shadows. | `astraweave-render` | Default frame leaves `cloud_raymarch` empty. Explicit enable creates the pass. Heavy-scene default stays the shadow pass only. |
| 3 | SSGI | #230 | `set_ssgi_enabled` defaults false. `SsgiPass::new` runs only after the flag. Depth slots are not `TextureSampleType::Depth`. Shader does not `textureLoad` that depth. | Fix depth sampling first. Quality default off. | `astraweave-render` | `gated_passes` still proves the flag gates creation. Default `ssgi_on` is false. Device validation submit with the flag on samples depth legally. |
| 4 | TAA | #231 | `set_taa_enabled` defaults false. History is presentation-only. Same depth-binding gap as SSGI. | Fix depth. Quality default off. History is not sim state. | `astraweave-render` | Default flag false. Enabled pass records without writing a sim component. |
| 5 | Depth of field | #232 | `set_dof_enabled` defaults false. `advanced_post` `depth_tex` is `texture_2d<f32>`. Creation failure is logged and the pass stays empty. | Fix the depth view. Quality default off. Keep the log on creation failure. | `astraweave-render` | Default flag false. Enabled path applies DoF or surfaces the creation error. |
| 6 | Temporal upscale | #233 | `upscale_on` defaults false. Depth at binding 3 is filterable float. | Fix that binding. Quality default off. | `astraweave-render` | Default flag false. Enabled pass uses a depth-typed binding. |

## Do not wire a second copy

The live owner already exists. A wiring PR would be a second pipeline. Exit is parity, then deletion, in its own issue. Not this campaign's first pulls.

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
| `AIArbiter`, advanced GOAP, `GoapPlanner` | #334, #383 | Feature-gated. Off `sys_ai_planning` and `sys_spatial_tick`. `GatherGoal` is the tick path when `ai-goap` is on. `AIArbiter` polls a model. `shadow_mode` and GOAP telemetry read `Instant`. | Do not flip the features into the tick. The port is #383. Exit: replay hashes unchanged, and no `Instant` or unseeded RNG on the tick. |
| `recorded_line` | none new | Pure helper in `astraweave-dialogue`. Callers are its tests. | Later dialogue PR. Not a render pass. Exit: one non-test caller that does not draw, or leave it until that caller exists. |

## Order

1. This inventory.
2. Atmosphere (#226).
3. Cloud raymarch (#229).
4. SSGI (#230), TAA (#231), depth of field (#232), temporal upscale (#233), each after its depth binding is legal.
5. #227 and #228 stay as they are: greybox on, heavy-scene benchmark off.
6. #234–#238 are not second implementations.
7. #371 and #372 are not this campaign.
8. #334 and #383 stay off the tick until the port in #383.
