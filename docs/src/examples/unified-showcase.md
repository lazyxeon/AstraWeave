# Unified Showcase

There is no `unified_showcase` package in the workspace, and there is no `examples/unified_showcase` directory.

`cargo run -p unified_showcase` fails because Cargo cannot find that package.

## Packages that do open a window

These names are workspace members:

```bash
cargo run -p shadow_csm_demo --release
cargo run -p physics_demo3d --release
cargo run -p fluids_demo --release
cargo run -p terrain_demo --release
cargo run -p aravath_runtime --bin aravath_greybox --release
```

`shadow_csm_demo` is a cascaded-shadow demo in `examples/shadow_csm_demo`. `aravath_greybox` is the windowed greybox described on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page. `fluids_demo` links `astraweave-fluids`. That crate is not the game-loop solver.

This page used to document a self-contained wgpu app (2048 shadow maps, a custom GLTF loader, 4× MSAA, and a frame-time table). That source is not in the tree. Those figures are not reproduced here.

## Related

- [Examples index](./index.md)
- [Rendering](../core-systems/rendering.md)
