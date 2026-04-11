// Geometry clipmap mesh generation for terrain LOD.
//
// Generates concentric "rings" of terrain geometry at increasing LOD levels,
// centered on the camera. Inner rings have higher vertex density; outer rings
// are coarser. Shared edges use transition/stitch geometry to avoid T-junctions.
//
// Reference: Strugar (2009) "Continuous Distance-Dependent Level of Detail for
// Rendering Heightmaps." Losasso & Hoppe (2004) "Geometry Clipmaps."

struct ClipmapUniforms {
    view_proj:        mat4x4<f32>,
    camera_pos:       vec3<f32>,
    clipmap_scale:    f32,          // world-space size of finest grid cell
    morph_constants:  vec4<f32>,    // (morph_start, 1/(morph_end - morph_start), 0, 0)
    heightmap_size:   vec2<f32>,    // width, height in texels
    inv_heightmap_size: vec2<f32>,  // 1/width, 1/height
    // True world-space camera XZ. In camera-relative mode, vertices are near
    // the origin so heightmap UV must be offset by this. In standard mode set
    // to (0,0) — world_xz already contains the absolute world position.
    heightmap_origin: vec2<f32>,
    _pad:             vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: ClipmapUniforms;
@group(0) @binding(1) var heightmap_tex: texture_2d<f32>;
@group(0) @binding(2) var heightmap_sampler: sampler;

struct VertexInput {
    @location(0) grid_pos: vec2<f32>,   // integer grid coordinates within this ring
    @location(1) ring_info: vec4<f32>,  // (ring_level, ring_scale, morph_start_dist, morph_range_inv)
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos:  vec3<f32>,
    @location(1) uv:         vec2<f32>,
    @location(2) normal:     vec3<f32>,
    @location(3) morph_factor: f32,
}

// Sample height from the heightmap texture at world XZ coordinates.
// The `offset` parameter is added to world_xz for UV computation — pass
// u.heightmap_origin to account for camera-relative rendering.
fn sample_height(world_xz: vec2<f32>, offset: vec2<f32>) -> f32 {
    let uv = (world_xz + offset) * u.inv_heightmap_size + 0.5;
    return textureSampleLevel(heightmap_tex, heightmap_sampler, uv, 0.0).r;
}

// Compute normal from heightmap via central differences (2 taps).
fn compute_normal(world_xz: vec2<f32>, cell_size: f32, offset: vec2<f32>) -> vec3<f32> {
    let step = cell_size;
    let hL = sample_height(world_xz - vec2<f32>(step, 0.0), offset);
    let hR = sample_height(world_xz + vec2<f32>(step, 0.0), offset);
    let hD = sample_height(world_xz - vec2<f32>(0.0, step), offset);
    let hU = sample_height(world_xz + vec2<f32>(0.0, step), offset);
    return normalize(vec3<f32>(hL - hR, 2.0 * step, hD - hU));
}

// Snap grid position to the next coarser LOD grid to enable morphing.
fn snap_to_parent(grid_pos: vec2<f32>) -> vec2<f32> {
    return floor(grid_pos * 0.5 + 0.5) * 2.0;
}

// Compute morph factor based on distance: 0 = full detail, 1 = fully morphed to parent LOD.
fn compute_morph_factor(distance: f32, morph_start: f32, morph_range_inv: f32) -> f32 {
    return clamp((distance - morph_start) * morph_range_inv, 0.0, 1.0);
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let ring_level = input.ring_info.x;
    let cell_size  = u.clipmap_scale * input.ring_info.y; // scale doubles per ring

    // Snap to camera-aligned grid: the grid follows the camera at the
    // resolution of the current ring level.
    let grid_snap = cell_size * 2.0;
    let snapped_camera = floor(u.camera_pos.xz / grid_snap + 0.5) * grid_snap;

    // World position from grid coordinates
    let world_xz = snapped_camera + input.grid_pos * cell_size;

    // Distance-based morph toward parent LOD grid
    let dist = length(world_xz - u.camera_pos.xz);
    let morph = compute_morph_factor(dist, input.ring_info.z, input.ring_info.w);

    // Interpolate between current and parent grid position
    let parent_xz = snapped_camera + snap_to_parent(input.grid_pos) * cell_size;
    let morphed_xz = mix(world_xz, parent_xz, morph);

    // Sample height at morphed position (offset by heightmap_origin for camera-relative)
    let height = sample_height(morphed_xz, u.heightmap_origin);
    let world_pos = vec3<f32>(morphed_xz.x, height, morphed_xz.y);

    out.world_pos = world_pos;
    out.clip_pos  = u.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv        = (morphed_xz + u.heightmap_origin) * u.inv_heightmap_size + 0.5;
    out.normal    = compute_normal(morphed_xz, cell_size, u.heightmap_origin);
    out.morph_factor = morph;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple shading: directional light + ambient.
    // In production this would feed into the full PBR pipeline.
    let light_dir = normalize(vec3<f32>(0.3, 1.0, 0.5));
    let ndl = max(dot(input.normal, light_dir), 0.0);
    let ambient = 0.15;
    let diffuse = ndl * 0.85;
    let color = vec3<f32>(0.4, 0.55, 0.3) * (ambient + diffuse);
    return vec4<f32>(color, 1.0);
}
