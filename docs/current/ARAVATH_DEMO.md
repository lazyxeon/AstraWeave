# Aravath demo

One window on the AstraWeave engine: a streamed voxel island, a rigged character, sword and shield combat, and three foes. Names and flavor follow Aravath world lore (internal brief). The 3D image is the engine renderer. `aravath_runtime` uploads meshes and drives simulation. It does not contain a shader, a shadow map, a fog volume, or a post chain. A death leaves a seepage mark where the Aravath is weak. The engine fog pass samples that mark. The foes read it through `astraweave-ai`. The field still lives in the `astraweave-veil` crate.

## Run it on Windows

The machine needs Rust 1.89 and an NVIDIA GPU from a GTX 1660 through an RTX 4070. From a checkout of this branch:

```text
cargo run -p aravath_runtime --bin aravath_greybox --release
```

`cargo run` finds `assets/aravath` because the binary is under `target/`. The GitHub artifact is a folder: `aravath_greybox.exe`, `README.txt`, `LICENSE`, and `assets/aravath` beside the exe. Extract the whole zip and start the exe from that folder. Saves are written under `%LOCALAPPDATA%\Aravath`. `aravath.log` and `aravath_crash.log` are written there and copied next to the exe. Startup prints both full paths. `aravath.cfg` is read from the exe folder. A fatal error also opens a Windows message box. The exe does not open a network connection or update itself. Distribution limits, checksums, and code signing are in `docs/current/ARAVATH_DISTRIBUTION.md`.

If the window fails while compiling shaders or turning on fog, relaunch with `aravath_greybox.exe --no-volumetrics`, or put `volumetrics=off` in `aravath.cfg` next to the exe. `--safe` is the same switch. The stage timings are in `aravath.log`.

The workflow `.github/workflows/aravath-windows.yml` builds that exe and runs `cargo test -p aravath_runtime --lib`. It does not launch the window.

## Controls

| Input | Action |
| --- | --- |
| WASD | Run |
| Alt | Walk |
| Mouse | Orbit the camera. The cursor is captured. |
| Shift | Sprint |
| Space | Jump, double jump, or climb while flying |
| Ctrl | Crouch, or descend while flying |
| Q | Dodge roll. Mid-roll frames ignore damage. |
| F | Toggle flight |
| E | Dig |
| R | Place the selected block |
| 1-9 | Stone, dirt, grass, sand, wood, snow, copper, silver, gold |
| Left mouse | Sword |
| Right mouse | Shield |
| Tab | Lock on the nearest living enemy |
| F5 | Save slot 0 |
| F9 | Load slot 0 |
| Enter | Respawn at the opening spawn after death |
| Escape | Release the mouse. Press again to quit |

Airborne runs keep 40% of ground speed. Flight keeps full speed and no gravity.

## Frame budget

The whole frame is budgeted at 11 ms on a GTX 1660 Ti-class GPU. From a release build:

```text
cargo run -p aravath_runtime --bin aravath_greybox --release -- --frame-times
```

That prints CPU sim-and-skin percentiles against 11.000 ms, then one headless submit on whatever adapter this machine has. The headless number is not the 1660 Ti measurement. A release build exits with an error when the CPU p95 is over the budget. A debug build prints the same line and does not fail on it.

A settled tick does not allocate for streaming, seepage marks, or the locomotion graph. Skinning reuses the pose buffers after the first sample. Rapier's physics step and the character raycasts still allocate inside `astraweave-physics`. That count stays flat from tick to tick.

## Dynamic world

Dig calls `dig_sphere`, which applies a sphere `Subtract` brush. `apply_brush_now` loads every chunk the radius touches, then the island mesh is carved by the occupancy drop. Place still writes voxels. Both reshape the physics trimesh. Death writes a seepage mark through the weaving field, and the engine fog pass samples it. Traversal (#244, `a10780f4`) drives the motor from a 60 Hz tick sim in `astraweave-physics`: ground, air, crouch, sprint, and roll step once per tick. Roll invulnerability is ticks 5 through 19. Climb, swim, slide, prone, wall-run, glide, ledge, and shimmy are in that sim. Foot and hand IK stay on the presentation graph. The demo still takes input through `PlayInput`, `FrameLatch`, `DeviceEvent::MouseMotion`, Windows `CursorGrabMode::Confined`, and `InputManager::set_physical_key`.

## Determinism

Movement, stamina, combat, enemy steering, and voxel edits advance at a fixed 60 Hz. The window accumulates real frame time and spends it in slices of 1/60 s, at most eight slices in one frame. A short frame does not step the simulation. A jump, swing, or dig pressed during that frame waits for the next slice. The same recorded inputs produce the same state hash on independent runs, including the player and the edit log after a save and load. The save file's id and timestamp are not part of that hash. The sun clock and the HUD notice still follow the rendered frame.

The replay is also checked across machines. Rapier is built with `enhanced-determinism`. Sim `sin`, `cos`, `atan2`, `sqrt`, `pow`, and `exp` go through the pure-Rust `libm` crate. The sim crates refuse `target-cpu=native` and the `fma` target feature. `.github/workflows/aravath-determinism.yml` runs that replay on Linux and Windows and compares `Greybox::state_hash` to `aravath_runtime/golden/replay.hash`, which is `b6a9d5a2c5da28a6`. That golden is a separate Linux-and-Windows hash. #329 moved it to `0a576578512f25d2` for the scripted dig and the bridge route. #330 moved it again because `state_hash` includes `LocalSim::field_digest`. `Greybox::step` calls `BridgeAgent::step` after the motor and then ticks the local field. The nineteen three-OS cases include `local_sim`, `greybox_checkpoint`, `quest_ticks`, and `persist_replay`. `ColumnStream` keeps the loaded ring. The voxels in that ring still come from the island heightfield, which is the ground this golden hashes.

## What it shows

You stand in forested hill country on the western frontier, streamed at a Chebyshev radius of 3 chunks (32 m each, about 224 m across). A dirt road runs east, toward the village of Yaqar, which is not built here. Stone, dirt, grass, and sand take their albedo from KayKit Block Bits. Wood stays the Poly Haven plank map. Snow, copper, silver, and gold are placeable (keys 6-9) and are not generated by the heightfield. The road reuses dirt. The grass hills scatter KayKit forest trees, bushes, tufts, and rocks, plus crates, gems, ore nuggets, and loose weapons. Sand and rock ground still use Kenney palms, pines, and rocks. Digging and placing write the same voxel grid. Void-amber is not a placeable block. The body is the KayKit knight, with `sword_1handed` and `shield_round`. Clips are the Rig_Medium set: walk, run, sprint, jump, crouch, sneak, dodge roll, sword attack, block, hit, death, and flight. Asset paths, packs, and sha256 hashes are in `assets/aravath/ASSET_MANIFEST.tsv`. Load order is that sorted list and `BTreeMap` clip names. Prop instances are gathered by sorted chunk key.

The foes stay PLACEHOLDER seepage-touched beast (Skeleton Minion), PLACEHOLDER hollow-eyed wildlife (Skeleton Warrior), and PLACEHOLDER roadside brigand (Skeleton Golem, the taller mesh). A Skeleton Rogue stands on the hill and is not a fourth simulated body, so the replay hash does not move. They hold still past 18 m, steer apart as they close, wait through a short windup, then swing. A stagger steps them back. Lock-on names the target and drops when it dies. Greybox startup runs `run_aravath_slice` once (#275, #343). Each connecting sword hit reuses that cached `sword_hp`. The foe loses `40 - sword_hp`. The slice test locks that remainder at 15, so the hit deals 25. When the cover bond is broken, the window shows `Cover broke` for 2.5 seconds, once. Death freezes the body on `Death_A` until Enter puts you back at the spawn with full health. The edit log is kept. F5 stores edits, the demolition command log, position, facing, stamina, health, and the seepage marks. F9 rebuilds the loaded chunks from the heightfield, the voxel edits, and that command log, and puts the marks back.

A death writes Q32.32 energy into a 4 m cell and a quarter of that energy into the six neighboring cells. Each simulation step (not each wall-clock frame) decays the surge and keeps one third of the decayed slice as a permanent floor. While the surge is high, living foes step away from it and do not swing. Once it has settled, a foe standing on the floor is wary: it approaches at half speed and does not start a new attack. The field is a `BTreeMap` of fixed-point cells in `astraweave-veil`. Its cross-run FNV-1a hash for the scripted history is `0xb1cc29cc42401e83`. The AI stance hash for a death at the origin is `0x0eb14da1efc9e7b2`.

## Engine systems the demo exercises

- **PBR forward pass** (`astraweave-render` `Renderer`, the main mesh shader). Terrain, the character, weapons, and props are named models with albedo textures or material colors. The blit is the engine ACES tonemap.
- **Time-of-day sun.** `tick_environment` advances `TimeOfDay`. The demo starts at 16:40 and does not pin a light-direction override, so the sun direction and color come from the engine clock.
- **Cascaded shadow maps.** `set_shadows_enabled(true)`. The demo's models are shadow casters in the engine's cascade passes, and the PBR shader samples those maps. The separate low-resolution cloud-shadow texture stays off so the cascades are the shadow you see.
- **Sky.** `SkyRenderer` with a `SkyConfig` (day colors and cloud cover), drawn into the HDR target before the meshes.
- **Fog.** A light exponential term from `SceneEnvironment`, plus `VolumetricFogPass` at the low froxel preset (80×45×32). Scatter reads the cascade shadow array. The result is copied back into the HDR target before tonemap. Seepage marks are a 32³ world-space texture (256 KiB, RGBA16F) sampled by that same density pass at the froxel's world position. They add density, pull the Henyey-Greenstein `g` toward 0.86, and tint the in-scatter amber-violet. Violet is an engine choice, not a world color. There is no second volumetric pass. `--no-volumetrics` still builds neither the fog pass nor the mark texture.
- **Light shafts.** `GodRayPass` at half resolution, 32 radial samples, added onto the HDR target after the fog. `--no-volumetrics` leaves both the fog pass and the shafts off. The pipelines are built only when those effects are enabled, after the window has already been painted.
- **Camera, physics, terrain, input, save.** `ThirdPersonCamera`, the Rapier character controller, `DualContouring` chunk meshes, `astraweave-input`, and `aw-save` `SaveBundleV2`.

The egui overlay is the HUD only. It is drawn after the engine frame.

## Performance budget

Stay inside this box on a GTX 1660 at 1080p, and do not treat an RTX 4070 as a reason to raise it. The whole frame stays under 11 ms on a GTX 1660 Ti. `--frame-times` reports the CPU sim-and-skin time against that budget.

- Stream radius 3. Do not fill a multi-kilometre voxel volume.
- Block-bit albedo crops for the voxel materials, the wood plank map, and the small KayKit atlases. Props are instanced, not a mesh per blade of grass.
- Volumetric fog at 80×45×32 froxels, plus the 32³ scar volume inside that pass. The medium grid (160×90×64) is the next step up and wants a capture on a 1660 before it becomes the default for this demo. The scar upload does not change the froxel dimensions.
- Light shafts at half resolution and 32 samples.
- The engine's existing cascade map size. Do not add a second shadow implementation in the demo.

## Known limitations

- Foe positions are not in the save. Load restores the island edits, the player, and the seepage marks, then the three foes are the ones spawned at startup. A mark is still there if you walk back to the death.
- Respawn is the opening spawn, not the last save point.
- One slot.
- The window test is not part of CI. Linux cannot link the MSVC `basis-universal` build (`lib.exe` is absent), which is why the Windows workflow exists.
- Cursor capture can fail on a remote desktop. The demo prints that and leaves the cursor free.
- Sword and shield are parented to the hand joints. They are not skinned as part of the body mesh.
- The nav mesh records a dirty region and does not path the foes. They steer in a straight line with a separation push, and they sidestep a strong seepage mark using the integer retreat step from `astraweave-ai`.

## Seepage marks, and what was left out of the Chevel note

The note called Project Chevel is the inspiration for the scar, not a drop-in patch.

| Idea in the note | What the demo does instead |
| --- | --- |
| WGSL `array<i64>` | WGSL has no `i64`. The field stays Q32.32 on the CPU. `gpu_f32` runs only while baking the upload. |
| `froxel_idx % 64` | That index is not a place. The density shader samples the scar texture with the froxel's world position, and texels outside the 32³ window contribute nothing. |
| A 16×9×24 froxel grid | The demo keeps the engine low preset, 80×45×32. |
| A new volumetric pass | `VolumetricFogPass` gained one texture, one sampler, and one uniform. Scatter reads channels the density pass already writes. |
| GTX 1660 frame times and a Python 120-tick table | Those numbers are not reproducible from this repo. The sim hash above is. GPU timestamps wrap the existing density and scatter dispatches when the device supports them. This environment has no GTX 1660. wgpu fell back to software EGL (DRI3 unavailable). A cold dispatch reported 64.7 ms density and 47.8 ms scatter. A later run of the same test reported 2.24 ms and 2.03 ms. Both figures are the whole 80×45×32 passes, scar sample included, not a 1660 frame time. The scar does not add a dispatch. |
| Perception and planning stubs, with no decay | Transient energy decays onto a raised floor. `sense` / `stance_at` are the perception query. |
| Spillways, floodgates, guild wardens | The demo has no city and no water system. Those systems were not added. |

The seepage-mark simulation does not read the wall clock and does not iterate a `HashMap`. A save written before this system has no `veil_scars` meta entry and loads an empty field. `veil_scars` is the save key. The player-facing word is seepage mark.
