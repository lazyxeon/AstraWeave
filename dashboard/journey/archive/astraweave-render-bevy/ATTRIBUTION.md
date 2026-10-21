# Attribution and Licensing

## Bevy Engine Components

This crate (`astraweave-render-bevy`) incorporates code and shaders from the **Bevy Engine** project.

**Original Source**: https://github.com/bevyengine/bevy  
**License**: MIT OR Apache-2.0  
**Copyright**: (c) 2020 Carter Anderson and Bevy contributors

### Dual License Notice

Bevy is dual-licensed under:
- MIT License (see LICENSE-MIT or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 (see LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

You may choose either license when using Bevy-derived code in this crate.

### Files Derived from Bevy

The following files are derived from Bevy Engine `v0.14.0`:

#### WGSL Shaders (from `bevy_pbr`)
- `shaders/pbr_fragment.wgsl` - PBR fragment shader  
  Original: `bevy/crates/bevy_pbr/src/render/pbr_fragment.wgsl`

- `shaders/pbr_functions.wgsl` - PBR lighting functions  
  Original: `bevy/crates/bevy_pbr/src/render/pbr_functions.wgsl`

- `shaders/shadows.wgsl` - Shadow sampling and CSM  
  Original: `bevy/crates/bevy_pbr/src/render/shadows.wgsl`

- `shaders/shadow_sampling.wgsl` - PCF shadow filtering  
  Original: `bevy/crates/bevy_pbr/src/render/shadow_sampling.wgsl`

- `shaders/mesh_view_bindings.wgsl` - View/projection uniforms  
  Original: `bevy/crates/bevy_pbr/src/render/mesh_view_bindings.wgsl`

- `shaders/tonemapping.wgsl` - Tonemapping operators (ACES, Reinhard, AgX)  
  Original: `bevy/crates/bevy_pbr/src/render/tonemapping.wgsl`

#### Rust Source Files (from `bevy_pbr`)
- `src/render/mesh.rs` - Mesh pipeline (adapted)  
  Original: `bevy/crates/bevy_pbr/src/render/mesh.rs`

- `src/render/material.rs` - Material system (adapted)  
  Original: `bevy/crates/bevy_pbr/src/material.rs`

- `src/render/light.rs` - Lighting system (adapted)  
  Original: `bevy/crates/bevy_pbr/src/render/light.rs`

- `src/render/shadow.rs` - Shadow rendering (adapted)  
  Original: `bevy/crates/bevy_pbr/src/render/shadows.rs`

### Modifications

Bevy-derived code has been adapted for AstraWeave's custom ECS:
- Removed `bevy_ecs` dependencies (replaced with trait abstractions)
- Adapted to `astraweave-ecs` component system via `RenderAdapter`
- Simplified features (removed prepass, wireframe, fog for Phase 1)
- Integrated with AstraWeave's material system

### AstraWeave Extensions (Original Work)

The following components are **original work** by AstraWeave contributors (MIT license):

- `src/extensions/megalights.rs` - GPU-accelerated light culling (100k+ lights)  
  Ported from `astraweave-render/src/clustered_megalights.rs`

- `src/extensions/nanite.rs` - Virtualized geometry system (10M+ polys)  
  Ported from `astraweave-render/src/nanite_*.rs`

- `src/adapter.rs` - ECS bridge (AstraWeave ECS ↔ Bevy renderer)

### License Compliance

**For AstraWeave Users**:
- You may use this crate under AstraWeave's MIT license
- Bevy-derived components remain dual-licensed (MIT OR Apache-2.0)
- You must include this ATTRIBUTION.md file when distributing

**For Bevy Contributors**:
- Thank you for creating an exceptional rendering engine!
- All Bevy-derived code retains original copyright and dual licensing
- Modifications are documented above

---

## Full License Texts

### MIT License (Bevy)

```
MIT License

Copyright (c) 2020 Carter Anderson

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### Apache License 2.0 (Bevy)

See: https://www.apache.org/licenses/LICENSE-2.0

---

**Version**: 1.0  
**Last Updated**: November 5, 2025  
**Bevy Version**: 0.14.0
