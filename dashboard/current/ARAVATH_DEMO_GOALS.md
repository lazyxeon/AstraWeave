# Aravath demo goals

This is the campaign after the greybox island. The original phase plan stays as it was. This document is the end state the rest of the branch builds toward. Demo lore, names, enemies, and flavor text follow Aravath world lore (internal brief). The older in-repo lore bible is not a source for this demo.

The demo is one playable loop: an open, mineable world, Enshrouded-style digging and placement, free movement, a few simple enemies, sword and shield combat with lock-on and invincible dodge rolls, and save/load of edits plus the player. Visuals use CC0 assets where a download is viable, and geometric primitives where it is not. Every borrowed file is listed in `assets/aravath/CREDITS.md`.

## Performance and platform

Target machines are Windows desktops with an NVIDIA GPU from a GTX 1660 through an RTX 4070. The demo does not fill a 10 km voxel volume. Chunks stream in a small radius around the player. `wgpu` picks DX12 or Vulkan on Windows and Vulkan on this Linux VM. Rust stays free of Linux-only calls.

Build on Windows from a repo checkout with Rust 1.89:

```text
cargo run -p aravath_runtime --bin aravath_greybox
```

## Remaining phases

2. Movement. Walk, run, sprint, jump, double jump, crouch, dodge roll with a mid-roll invincibility window, and a creative-mode fly toggle. Stamina is an f32 on the motor. Sprint and roll spend it. It does not regenerate during a roll. Airborne runs keep 40% of ground speed. The demo camera sits closer than the default third-person rig, and the window captures the mouse.
3. One locomotion graph over `astraweave_render::animation::AnimationClip`. The visible body stays a primitive until a clip is skinned. A test samples a hand-built clip at the start and at the duration.
4. Combat. Sword swings have startup, active, and recovery. A hit lands once, and only during the active window. The shield reduces damage while held. Lock-on yaws the third-person camera toward one enemy and clears when that enemy dies. Enemies ignore the player past 18 m, steer apart, wait through a short windup, then swing, and step back while staggered. A roll's invincibility window ignores a hit.
5. Placement, support, and save. `place_sphere` writes solid voxels through the same dirty-chunk sync. A voxel with no solid 6-neighbor and above the floor is rejected. Edits, position, facing, and stamina go through `aw-save` as a `SaveBundleV2` built directly. Load replays the edit log onto a fresh world. The edited bounds are marked dirty on `astraweave-nav`. No nav agents spawn.
6. Open world. Chunks generate from a deterministic height around the player and unload past the radius. Digging and placing use the same voxel grid. The phase 1 island remains the fast test fixture.
7. Assets. Download CC0 textures or models from Poly Haven, Kenney, or itch.io when the host answers. Record the URL and license. Where a download is not viable, keep geometric primitives and say so in the credits file.
8. Windows check, recorded here. `cargo check -p aravath_runtime --target x86_64-pc-windows-msvc` on this Linux VM stops in `cc-rs` with `failed to find tool "lib.exe"`. The MSVC librarian is not installed here, and `basis-universal` (pulled in by `astraweave-render`) compiles C++. The greybox Rust does not call a Linux window system. `wgpu` selects DX12 or Vulkan on Windows from the same sources. On this VM the demo process stayed up under Vulkan/EGL until it was stopped.

## Controls

| Input | Action |
| --- | --- |
| WASD | Move (run) |
| Alt | Walk |
| Mouse | Orbit camera |
| Shift | Sprint |
| Space | Jump, double jump, or fly up |
| Ctrl | Crouch, or fly down |
| Q | Dodge roll |
| F | Toggle creative flight |
| E | Dig |
| R | Place the selected block |
| 1-5 | Select stone, dirt, grass, sand, or wood |
| F5 | Save |
| F9 | Load |
| Left mouse | Sword |
| Right mouse | Shield |
| Tab | Lock on |
| Escape | Quit |
