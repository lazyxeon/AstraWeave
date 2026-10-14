# Aravath demo

One window on the AstraWeave engine: a streamed voxel island, a rigged character, sword and shield combat, and three enemies. The 3D image is the engine renderer. `aravath_runtime` uploads meshes and drives simulation. It does not contain a shader, a shadow map, a fog volume, or a post chain.

## Run it on Windows

The machine needs Rust 1.89 and an NVIDIA GPU from a GTX 1660 through an RTX 4070. From a checkout of this branch:

```text
cargo run -p aravath_runtime --bin aravath_greybox --release
```

`cargo run` finds `assets/aravath` because the binary is under `target/`. The GitHub artifact is a folder: `aravath_greybox.exe`, `README.txt`, and `assets/aravath` beside the exe. Extract the whole zip and start the exe from that folder. Saves and `aravath.log` / `aravath_crash.log` are written next to the exe. A fatal error also opens a Windows message box.

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
| 1-5 | Stone, dirt, grass, sand, wood |
| Left mouse | Sword |
| Right mouse | Shield |
| Tab | Lock on the nearest living enemy |
| F5 | Save slot 0 |
| F9 | Load slot 0 |
| Enter | Respawn at the opening spawn after death |
| Escape | Quit |

Airborne runs keep 40% of ground speed. Flight keeps full speed and no gravity.

## Determinism

Movement, stamina, combat, enemy steering, and voxel edits advance at a fixed 60 Hz. The window accumulates real frame time and spends it in slices of 1/60 s, at most eight slices in one frame. A short frame does not step the simulation. A jump, swing, or dig pressed during that frame waits for the next slice. The same recorded inputs produce the same state hash on independent runs, including the player and the edit log after a save and load. The save file's id and timestamp are not part of that hash. The sun clock and the HUD notice still follow the rendered frame.

The replay is also checked across machines. Rapier is built with `enhanced-determinism`. Sim `sin`, `cos`, `atan2`, `sqrt`, `pow`, and `exp` go through the pure-Rust `libm` crate. The sim crates refuse `target-cpu=native` and the `fma` target feature. `.github/workflows/aravath-determinism.yml` runs that replay on Linux and Windows and compares `Greybox::state_hash` to `aravath_runtime/golden/replay.hash`.

## What it shows

You stand on a heightfield that streams a Chebyshev radius of 3 chunks (32 m each, about 224 m across). Grass, sand, and rock biomes use Poly Haven albedo maps. Kenney trees, rocks, and grass tufts sit on the surface. Digging and placing write the same voxel grid. The body is the Quaternius rig. The sword, shield, and enemies are Kenney meshes. Clips follow walk, run, sprint, jump, crouch, roll, attack, block, hit, death, and flight.

Three enemies hold still past 18 m, steer apart as they close, wait through a short windup, then swing. A stagger steps them back. Lock-on drops when the target dies. Death freezes the body on `Death01` until Enter puts you back at the spawn with full health. The edit log is kept. F5 stores edits, position, facing, stamina, and health. F9 rebuilds the loaded chunks from the heightfield and that log.

## Engine systems the demo exercises

- **PBR forward pass** (`astraweave-render` `Renderer`, the main mesh shader). Terrain, the character, weapons, and props are named models with albedo textures or material colors. The blit is the engine ACES tonemap.
- **Time-of-day sun.** `tick_environment` advances `TimeOfDay`. The demo starts at 16:40 and does not pin a light-direction override, so the sun direction and color come from the engine clock.
- **Cascaded shadow maps.** `set_shadows_enabled(true)`. The demo's models are shadow casters in the engine's cascade passes, and the PBR shader samples those maps. The separate low-resolution cloud-shadow texture stays off so the cascades are the shadow you see.
- **Sky.** `SkyRenderer` with a `SkyConfig` (day colors and cloud cover), drawn into the HDR target before the meshes.
- **Fog.** A light exponential term from `SceneEnvironment`, plus `VolumetricFogPass` at the low froxel preset (80×45×32). Scatter reads the cascade shadow array. The result is copied back into the HDR target before tonemap.
- **Light shafts.** `GodRayPass` at half resolution, 32 radial samples, added onto the HDR target after the fog. `--no-volumetrics` leaves both the fog pass and the shafts off. The pipelines are built only when those effects are enabled, after the window has already been painted.
- **Camera, physics, terrain, input, save.** `ThirdPersonCamera`, the Rapier character controller, `DualContouring` chunk meshes, `astraweave-input`, and `aw-save` `SaveBundleV2`.

The egui overlay is the HUD only. It is drawn after the engine frame.

## Performance budget

Stay inside this box on a GTX 1660 at 1080p, and do not treat an RTX 4070 as a reason to raise it:

- Stream radius 3. Do not fill a multi-kilometre voxel volume.
- Five 1k albedo maps, one per material, plus the small Kenney colormap.
- Volumetric fog at 80×45×32 froxels. The medium grid (160×90×64) is the next step up and wants a capture on a 1660 before it becomes the default for this demo.
- Light shafts at half resolution and 32 samples.
- The engine's existing cascade map size. Do not add a second shadow implementation in the demo.

## Known limitations

- Enemy positions are not in the save. Load restores the island edits and the player, then the three enemies are the ones spawned at startup.
- Respawn is the opening spawn, not the last save point.
- One slot.
- The window test is not part of CI. Linux cannot link the MSVC `basis-universal` build (`lib.exe` is absent), which is why the Windows workflow exists.
- Cursor capture can fail on a remote desktop. The demo prints that and leaves the cursor free.
- Sword and shield are parented to the hand joints. They are not skinned as part of the body mesh.
- The nav mesh records a dirty region and does not path the enemies. They steer in a straight line with a separation push.
