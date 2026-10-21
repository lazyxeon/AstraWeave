# AstraWeave Renderer Test Plan

## Objective
Ensure high confidence in `astraweave-render` functionality through comprehensive, automated testing, minimizing reliance on manual visual inspection.

## Testing Strategy

### 1. Unit Tests (Logic & Math)
Focus on pure functions and isolated components that don't require a GPU context.
- **Vertex Compression**: Test `pack_normal`, `unpack_normal`, `pack_uv`, `unpack_uv` with various inputs (edge cases, random values).
- **LOD Generation**: Test `generate_lods` with simple meshes (cube, plane) to verify vertex count reduction.
- **Math Helpers**: Verify frustum culling logic, matrix math (if custom).
- **Material System**: Test parsing of material TOML files, error handling for missing textures.

### 2. Integration Tests (GPU Context)
Verify that the renderer initializes and executes commands correctly using a real WGPU context.
- **Initialization**: Verify `Renderer::new` succeeds with valid config.
- **Resource Management**: Verify `create_mesh`, `create_material`, `create_texture` work and return valid IDs.
- **Pipeline Creation**: Verify pipelines are created without validation errors (already started with `renderer_integration_test`).
- **Buffer Updates**: Verify `update_instances`, `update_camera` correctly write to GPU buffers (using buffer readback).

### 3. Golden File Tests (Visual Regression)
Render specific scenes and compare the output against "golden" reference images.
- **Test Harness**: A headless test runner that:
    1. Initializes Renderer.
    2. Sets up a scene (Camera, Mesh, Light).
    3. Renders to a texture.
    4. Reads back the texture to a PNG.
    5. Compares with `tests/golden/expected_*.png`.
- **Scenarios**:
    - `basic_triangle`: Simple colored triangle (verifies pipeline).
    - `textured_cube`: Cube with test pattern texture (verifies UVs/sampling).
    - `lighting_point`: Sphere lit by point light (verifies lighting math).
    - `skinning_simple`: Cylinder bent by 90 degrees (verifies vertex shader skinning).
    - `instancing`: Grid of cubes (verifies instance buffer).

### 4. Property-Based Tests
Use randomized inputs to find edge cases.
- **Compression Fuzzing**: Random floats -> Pack -> Unpack -> Assert close to original.
- **Mesh Fuzzing**: Generate random valid meshes -> LOD generation -> Assert no panic and reduction in count.

## Implementation Plan

### Phase 1: Test Infrastructure
- Create `tests/renderer_test_harness.rs`: Shared code for initializing headless renderer and buffer readback.
- Add `image` and `proptest` (or similar) to `dev-dependencies`.

### Phase 2: Unit & Property Tests
- Implement `src/vertex_compression.rs` tests.
- Implement `src/lod_generator.rs` tests.

### Phase 3: Visual Regression Suite
- Implement `tests/visual_tests.rs`.
- Create "Golden" images (initially by running tests and manually verifying, then saving).

### Phase 4: Feature Coverage
- Systematically add tests for:
    - Shadows (CSM)
    - Post-processing (Bloom)
    - Particles

## Tools
- **WGPU**: Use `wgpu::Instance::new(Backends::PRIMARY)` for testing.
- **Image**: For saving/loading PNGs.
- **Approx**: For floating point comparisons.
