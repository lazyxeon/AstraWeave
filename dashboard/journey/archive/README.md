# Archived Crates

This directory contains crates that have been deprecated and consolidated.

## astraweave-render-bevy (Archived: 2024-12-04)

**Reason**: Renderer consolidation - keeping `astraweave-render` as the single wgpu-based renderer.

The Bevy-inspired renderer was an experimental approach that was superseded by the more complete `astraweave-render` crate, which has:
- 52 source files with complete PBR, clustered lighting, shadows, IBL
- 20+ examples using it
- Nanite virtualized geometry support
- GPU particles, decals, deferred rendering

## bevy_shadow_demo (Archived: 2024-12-04)

**Reason**: Used deprecated `astraweave-render-bevy` crate.

Shadow demos should now use `examples/shadow_csm_demo` with the main `astraweave-render` crate.
