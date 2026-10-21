## Bind Group Layouts (canonical)

This summarizes the stable bind group indices and bindings used by the default renderer and examples. See `astraweave-render/MATERIALS.md` for materials specifics.

- Group 0: Camera/Scene/Debug
	- binding(0) Camera (uniform): mat4 view_proj
	- binding(1) Post params (uniform): exposure, etc.
	- binding(2) Scene params (uniform): time, camera_height
	- binding(4) Debug params (uniform)

- Group 1: Materials (texture arrays)
	- binding(0) material_albedo: texture_2d_array<f32> (RGBA8 sRGB)
	- binding(1) material_albedo_sampler: sampler (filtering)
	- binding(2) material_normal: texture_2d_array<f32> (RG8; Z reconstructed)
	- binding(3) material_normal_sampler: sampler (filtering; also used for MRA)
	- binding(4) material_mra: texture_2d_array<f32> (RGBA8: R=metal, G=roughness, B=AO)

- Group 2: Shadows
	- binding(0) shadow_map: texture_depth_2d
	- binding(1) shadow_sampler: sampler_comparison

- Group 3: Light/Shadow params
	- binding(0) Light camera (uniform): mat4 view_proj (light space)
	- binding(1) Shadow params (uniform): resolution, cascade_count, softness, bias, splits

- Group 4: Material uniform (optional per-draw)
	- binding(0) Material data (uniform): albedo/emissive/roughness_metallic/flags

- Group 5: IBL
	- binding(0) ibl_specular: texture_cube<f32> (prefiltered; mipmapped)
	- binding(1) ibl_irradiance: texture_cube<f32>
	- binding(2) brdf_lut: texture_2d<f32>
	- binding(3) ibl_sampler: sampler (filtering)

These bindings are consumed by the default WGSL shaders in examples; maintain these indices for cross-example compatibility.

# Engine API (Overview)

This is a living overview of the public engine interfaces. Link specific crate docs as they stabilize.

- **World & Entities**: ECS snapshot and deterministic simulation loop.
- **Navigation**: navmesh bake, path requests, movement constraints.
- **Physics**: Rapier3D integration, character controller.
- **Rendering**: WGPU forward pipeline, camera rig, scene graph primitives.
- **Audio**: spatial sound + VO mapping.

> Tip: keep Rustdoc comments on public structs/functions in `astraweave-core`, `-physics`, `-render` etc. `cargo doc --open` renders the API locally.