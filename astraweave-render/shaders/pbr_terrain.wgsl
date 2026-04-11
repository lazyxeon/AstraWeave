// pbr_terrain.wgsl — 8-layer PBR terrain splatting with triplanar projection
//
// Supports up to 8 material layers blended via splat maps (2 × RGBA textures).
// Features:
//   - Height-based blending for natural transitions
//   - Triplanar projection for steep slopes (avoids UV stretching)
//   - Reoriented Normal Mapping (RNM) for correct normal blending
//   - Per-layer UV scaling and PBR material properties

// ============================================================================
// Uniforms
// ============================================================================

struct TerrainLayer {
    texture_indices: vec4<u32>,  // [albedo, normal, orm, height]
    uv_scale:        vec2<f32>,
    height_range:    vec2<f32>,
    blend_sharpness: f32,
    triplanar_power: f32,
    material_factors: vec2<f32>, // [metallic, roughness]
    _pad: vec4<u32>,
}

struct TerrainParams {
    layers: array<TerrainLayer, 8>,
    splat_map_index_0: u32,   // R=layer0..A=layer3
    splat_map_index_1: u32,   // R=layer4..A=layer7
    splat_uv_scale:    f32,
    triplanar_enabled: u32,
    normal_blend_method: u32,
    triplanar_slope_threshold: f32,
    height_blend_enabled: u32,
    active_layer_count: u32,
    _pad: array<u32, 8>,
}

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad0: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(1) @binding(0) var<uniform> terrain: TerrainParams;
@group(2) @binding(0) var terrain_sampler: sampler;
@group(2) @binding(1) var splat_map_0: texture_2d<f32>;
@group(2) @binding(2) var splat_map_1: texture_2d<f32>;
@group(2) @binding(3) var layer_albedo: texture_2d_array<f32>;
@group(2) @binding(4) var layer_normal: texture_2d_array<f32>;
@group(2) @binding(5) var layer_orm:    texture_2d_array<f32>;
@group(2) @binding(6) var layer_height: texture_2d_array<f32>;

// ============================================================================
// Vertex stage
// ============================================================================

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos:  vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// Vertex shader is provided by the clipmap geometry pass;
// this shader starts at the fragment stage.

// ============================================================================
// Helpers
// ============================================================================

// Compute triplanar blend weights from world normal.
fn triplanar_weights(normal: vec3<f32>, sharpness: f32) -> vec3<f32> {
    var w = abs(normal);
    w = pow(w, vec3<f32>(sharpness));
    let total = w.x + w.y + w.z;
    return w / max(total, 0.0001);
}

// Sample a texture layer using triplanar projection.
fn sample_triplanar(
    tex: texture_2d_array<f32>,
    samp: sampler,
    layer_index: u32,
    world_pos: vec3<f32>,
    uv_scale: vec2<f32>,
    tri_w: vec3<f32>,
) -> vec4<f32> {
    let uv_x = world_pos.yz * uv_scale;
    let uv_y = world_pos.xz * uv_scale;
    let uv_z = world_pos.xy * uv_scale;

    let s_x = textureSample(tex, samp, uv_x, layer_index);
    let s_y = textureSample(tex, samp, uv_y, layer_index);
    let s_z = textureSample(tex, samp, uv_z, layer_index);

    return s_x * tri_w.x + s_y * tri_w.y + s_z * tri_w.z;
}

// Sample a texture layer using standard UV.
fn sample_planar(
    tex: texture_2d_array<f32>,
    samp: sampler,
    layer_index: u32,
    uv: vec2<f32>,
    uv_scale: vec2<f32>,
) -> vec4<f32> {
    return textureSample(tex, samp, uv * uv_scale, layer_index);
}

// Height-based blending: sharpen weights using per-layer height map values.
fn height_blend(weights: array<f32, 8>, heights: array<f32, 8>, count: u32) -> array<f32, 8> {
    var max_h: f32 = -999.0;
    for (var i: u32 = 0u; i < count; i++) {
        let h = heights[i] + weights[i];
        max_h = max(max_h, h);
    }

    var result: array<f32, 8>;
    var total: f32 = 0.0;
    let blend_range: f32 = 0.2;
    for (var i: u32 = 0u; i < count; i++) {
        let h = heights[i] + weights[i];
        result[i] = max(h - max_h + blend_range, 0.0);
        total += result[i];
    }

    if (total > 0.0001) {
        for (var i: u32 = 0u; i < count; i++) {
            result[i] /= total;
        }
    }

    return result;
}

// Reoriented Normal Mapping (RNM) — correctly blend detail normal onto base.
fn rnm_blend(base: vec3<f32>, detail: vec3<f32>) -> vec3<f32> {
    let t = base + vec3<f32>(0.0, 0.0, 1.0);
    let u_n = detail * vec3<f32>(-1.0, -1.0, 1.0);
    return normalize(t * dot(t, u_n) - u_n * t.z);
}

// ============================================================================
// Fragment stage
// ============================================================================

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) orm: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let N = normalize(in.world_normal);
    let slope = 1.0 - N.y;
    let use_triplanar = terrain.triplanar_enabled != 0u
        && slope > (terrain.triplanar_slope_threshold / 90.0);

    // Splat weights from two RGBA maps
    let splat_uv = in.uv * terrain.splat_uv_scale;
    let splat0 = textureSample(splat_map_0, terrain_sampler, splat_uv);
    let splat1 = textureSample(splat_map_1, terrain_sampler, splat_uv);

    var raw_weights: array<f32, 8>;
    raw_weights[0] = splat0.r;
    raw_weights[1] = splat0.g;
    raw_weights[2] = splat0.b;
    raw_weights[3] = splat0.a;
    raw_weights[4] = splat1.r;
    raw_weights[5] = splat1.g;
    raw_weights[6] = splat1.b;
    raw_weights[7] = splat1.a;

    // Normalize weights for active layers
    let count = terrain.active_layer_count;
    var total_w: f32 = 0.0;
    for (var i: u32 = 0u; i < count; i++) {
        total_w += raw_weights[i];
    }
    if (total_w > 0.0001) {
        for (var i: u32 = 0u; i < count; i++) {
            raw_weights[i] /= total_w;
        }
    }

    // Triplanar blend weights (if applicable)
    let tri_w = triplanar_weights(N, 4.0);

    // Sample height maps for height blending
    var layer_heights: array<f32, 8>;
    for (var i: u32 = 0u; i < count; i++) {
        let layer = terrain.layers[i];
        let h_idx = layer.texture_indices.w;
        if (use_triplanar) {
            layer_heights[i] = sample_triplanar(layer_height, terrain_sampler, h_idx, in.world_pos, layer.uv_scale, tri_w).r;
        } else {
            layer_heights[i] = sample_planar(layer_height, terrain_sampler, h_idx, in.uv, layer.uv_scale).r;
        }
    }

    // Apply height blending
    var weights = raw_weights;
    if (terrain.height_blend_enabled != 0u) {
        weights = height_blend(raw_weights, layer_heights, count);
    }

    // Accumulate PBR properties from all active layers
    var final_albedo = vec3<f32>(0.0);
    var final_normal = vec3<f32>(0.0, 0.0, 1.0);
    var final_ao: f32 = 0.0;
    var final_roughness: f32 = 0.0;
    var final_metallic: f32 = 0.0;

    for (var i: u32 = 0u; i < count; i++) {
        let w = weights[i];
        if (w < 0.001) {
            continue;
        }

        let layer = terrain.layers[i];
        let a_idx = layer.texture_indices.x;
        let n_idx = layer.texture_indices.y;
        let o_idx = layer.texture_indices.z;

        var albedo: vec4<f32>;
        var normal_sample: vec4<f32>;
        var orm_sample: vec4<f32>;

        if (use_triplanar) {
            albedo = sample_triplanar(layer_albedo, terrain_sampler, a_idx, in.world_pos, layer.uv_scale, tri_w);
            normal_sample = sample_triplanar(layer_normal, terrain_sampler, n_idx, in.world_pos, layer.uv_scale, tri_w);
            orm_sample = sample_triplanar(layer_orm, terrain_sampler, o_idx, in.world_pos, layer.uv_scale, tri_w);
        } else {
            albedo = sample_planar(layer_albedo, terrain_sampler, a_idx, in.uv, layer.uv_scale);
            normal_sample = sample_planar(layer_normal, terrain_sampler, n_idx, in.uv, layer.uv_scale);
            orm_sample = sample_planar(layer_orm, terrain_sampler, o_idx, in.uv, layer.uv_scale);
        }

        final_albedo += albedo.rgb * w;

        // Unpack tangent-space normal
        let tbn = normal_sample.rgb * 2.0 - 1.0;
        if (terrain.normal_blend_method == 1u) {
            // RNM blend
            final_normal = rnm_blend(final_normal, tbn * w + vec3<f32>(0.0, 0.0, 1.0 - w));
        } else {
            // Linear blend
            final_normal += tbn * w;
        }

        // ORM: R=AO, G=Roughness, B=Metallic (standard packing)
        final_ao += orm_sample.r * w;
        final_roughness += (orm_sample.g * layer.material_factors.y) * w;
        final_metallic += (orm_sample.b * layer.material_factors.x) * w;
    }

    final_normal = normalize(final_normal);

    var out: FragmentOutput;
    out.color = vec4<f32>(final_albedo, 1.0);
    out.normal = vec4<f32>(final_normal * 0.5 + 0.5, 1.0); // pack to [0,1]
    out.orm = vec4<f32>(final_ao, final_roughness, final_metallic, 1.0);
    return out;
}
