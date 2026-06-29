# astraweave-render

GPU rendering pipeline for AstraWeave, built on **wgpu 25**.

## Overview

Complete rendering solution with PBR materials, clustered forward lighting, cascaded shadow maps, post-processing, skeletal animation, LOD generation, deferred rendering, GPU particles, and dynamic weather.

## Key Types

| Type | Description |
|------|-------------|
| `Renderer` | Core rendering state and draw loop |
| `Camera` / `CameraController` | View management |
| `MaterialManager` | TOML-driven PBR material pipeline |
| `Skeleton` / `AnimationClip` | GPU skeletal animation |
| `AdvancedPostFx` | Bloom, TAA, DoF, motion blur, color grading |
| `WaterRenderer` | Gerstner wave water surface |
| `GpuParticleSystem` | GPU-accelerated particle effects |
| `DecalSystem` | Projected decals with atlas |
| `DeferredRenderer` | G-buffer deferred path |

## Feature Flags

| Feature | Description |
|---------|-------------|
| `postfx` | Post-processing effects (default) |
| `textures` | Texture loading (default) |
| `bloom` | HDR bloom pipeline |
| `ibl` | Image-based lighting |
| `megalights` | GPU-accelerated light culling |
| `deferred` | Deferred rendering path |
| `gpu-particles` | GPU particle system |
| `decals` | Projected decal system |
| `advanced-post` | TAA, motion blur, DoF, color grading |
| `ssao` | Screen-space ambient occlusion |
| `skinning-gpu` | GPU skeletal animation |

## Performance

- Frame time: 0.965 ms @ 1,000 entities (~1,036 FPS, System allocator); 0.709 ms (~1,410 FPS) with mimalloc. The 2.70 ms / 370 FPS figure was the Week-8 target.
- Vertex compression: 37.5% memory reduction
- GPU mesh instancing: 10–100× draw call reduction

## License

MIT
