// pbr_terrain_vs.wgsl — Vertex stage companion for pbr_terrain.wgsl
//
// The fragment stage in `pbr_terrain.wgsl` expects the `VertexOutput`
// interpolated attributes (clip_pos, world_pos, world_normal, uv) and the
// group(0) binding(0) `CameraUniforms`. This file provides the matching
// vertex function — concat these two files to form a complete shader
// module for the splat-array terrain pipeline.
//
// Vertex attributes (match `TerrainMaterialManager::VERTEX_LAYOUT`):
//   @location(0) vec3<f32> position (world-space)
//   @location(1) vec3<f32> normal   (world-space, unit)
//   @location(2) vec2<f32> uv       (chunk-local 0..1 or world-scaled)

struct TerrainVSInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
}

@vertex
fn vs_main(in: TerrainVSInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos     = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.world_pos    = in.position;
    out.world_normal = in.normal;
    out.uv           = in.uv;
    return out;
}
