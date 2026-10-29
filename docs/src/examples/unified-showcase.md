# Rendering showcase

There is no `unified_showcase` package and no `examples/unified_showcase` directory. `cargo run -p unified_showcase` cannot resolve a package.

The windowed rendering exercise in this tree is `shadow_csm_demo`. The game window is `aravath_greybox`. This page walks the shadow demo from the command to the keys in its source, then the greybox command.

## Shadow demo

Package `shadow_csm_demo`, file `examples/shadow_csm_demo/src/main.rs`. The file header lists `S` as shadows and `+/-` as zoom. The key handler does not. From `input` in that file:

- **W** moves along the camera forward. **S** moves back along that forward.
- **A** and **D** move on the camera right.
- **Space** raises the camera. **ShiftLeft** lowers it.
- **C** toggles cascade visualization (`show_cascade_colors`).
- **X** toggles shadows (`enable_shadows`).
- **V** sets `debug_mode = (debug_mode + 1) % 6`.
- There is no `+` or `-` arm.

Left mouse sets `mouse_pressed`. The header's shadow and zoom lines are not this handler.

Run it from the repository root. It opens a window, so it needs a GPU wgpu can use.

```bash
cargo run -p shadow_csm_demo --release
```

What the source sets up before the loop:

- `CsmRenderer` from `astraweave_render::shadow_csm`
- A `Vertex` of `position: [f32; 3]` and `normal: [f32; 3]`, with `wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3]`
- `CameraUniform` (`view_proj`, `view_pos`)
- `LightUniform` (`direction`, `color`)
- `RenderSettings` with `enable_shadows`, `show_cascade_colors`, and `debug_mode`. **V** cycles six names: `0` normal, `1` shadow UVs, `2` receiver depth, `3` shadow atlas depth, `4` raw shadow sample, `5` direct atlas view

`Camera.fovy` is the vertical field of view in radians. The comment in that struct says the field was renamed from `fov` so radians stay named `fovy`.

After it runs, **C** flips cascade colors and **X** flips shadows. Quit the window with the platform close control. This demo is not the frame-budget capture. That command and the measured sample are on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page.

## Greybox window

```bash
cargo xtask fetch-assets
cargo run -p aravath_runtime --bin aravath_greybox --release
```

`aravath_greybox` is a `[[bin]]` of package `aravath_runtime`. Fetch assets first. The engine status page describes what that window shows (streamed chunks, sword hit, dig). `cargo editor` opens `aw_editor` and is not this binary.

## Other windows

| Package | Command | Role |
| --- | --- | --- |
| `physics_demo3d` | `cargo run -p physics_demo3d --release` | Rapier demo |
| `fluids_demo` | `cargo run -p fluids_demo --release` | `FluidSystem`. Not a PCISPH solver. [Fluids demo](./fluids-demo.md) |
| `terrain_demo` | `cargo run -p terrain_demo --release` | Terrain example package |

The old page's 2048 shadow map, custom GLTF loader, 4× MSAA table, and 8–12 ms frame time described source that is not in the tree. Those figures are not repeated.

## Related

- [Examples index](./index.md)
- [Rendering](../core-systems/rendering.md)
- [Hello Companion](./hello-companion.md) for the headless companion, which does not open a window
