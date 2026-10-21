# Platform Matrix & Compile Flags (MVP)

## Graphics Backends
- Windows: DirectX 12 (primary), Vulkan (fallback)
- Linux: Vulkan (primary)
- macOS: Metal (primary)

## wgpu Feature Flags
- Optional: `wgpu/trace`, `wgpu/webgl` (off by default)
- Engine cfgs: `feature = "ssr"`, `feature = "ssao"`, `feature = "ddgi"` (all optional)

## Compatibility Table (starter)
- Tier 1 GPUs: RTX 20xx/30xx, RX 6000/7000, Apple M1+
- Tier 2 GPUs: GTX 10xx, RX 5000
- Shader Model: target WGSL with compatibility path for older drivers (avoid unsupported extensions)
