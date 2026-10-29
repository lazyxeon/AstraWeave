# Troubleshooting Examples

## Packages

These packages exist. A default `hello_companion` run does not panic.

| Package | Command |
| --- | --- |
| `hello_companion` | `cargo run -p hello_companion --release` |
| `fluids_demo` | `cargo run -p fluids_demo --release` |
| `physics_demo3d` | `cargo run -p physics_demo3d --release` |
| `navmesh_demo` | `cargo run -p navmesh_demo --release` |
| `audio_spatial_demo` | `cargo run -p audio_spatial_demo --release` |
| `adaptive_boss` | `cargo run -p adaptive_boss --release` |
| `ipc_loopback` | `cargo run -p ipc_loopback --release` |
| `shadow_csm_demo` | `cargo run -p shadow_csm_demo --release` |

`unified_showcase` is not a package. See [Rendering showcase](./unified-showcase.md).

This page does not keep a "known broken" list. Compilation of `debug_overlay`, `ui_controls_demo`, `visual_3d`, `rhai_authoring`, and `npc_town_demo` was not re-run for this edit. Those directories are still workspace members.

## Build

```bash
cargo build -p astraweave-core -p astraweave-ecs -p astraweave-math -p astraweave-ai
cargo run -p hello_companion --release
```

`hello_companion` without `--features visual` does not open a window. A missing graphics adapter is a failure of a windowed binary, not of the default companion demo.

`cargo editor` runs `aw_editor`. The aliases in `.cargo/config.toml` are `editor`, `editor-release`, and `editor-dev`.

## Related

- [Examples index](./index.md)
- [Troubleshooting](../resources/troubleshooting.md)
- [FAQ](../resources/faq.md)
