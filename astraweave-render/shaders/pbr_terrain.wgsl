// pbr_terrain.wgsl — 32-layer PBR terrain splatting with triplanar projection
//
// Supports up to 32 material layers blended via splat maps (8 × RGBA textures).
// Features:
//   - Height-based blending for natural transitions
//   - Triplanar projection for steep slopes (avoids UV stretching)
//   - Reoriented Normal Mapping (RNM) for correct normal blending
//   - Per-layer UV scaling and PBR material properties
//
// Real-Fix.D 2026-05-08: bumped from 8 to 32 layers + 2 to 8 splat textures
// per Andrew-gate decision (h) Option D-2 (canonical material library). The
// 32-layer cap matches `astraweave_render::MAX_TERRAIN_LAYERS` and is the
// canonical authority for terrain layer capacity at every boundary.

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
    // Real-Fix.D 2026-05-08: 32 layers (was 8). Must match
    // `astraweave_render::MAX_TERRAIN_LAYERS` and `TerrainMaterialGpu` byte-
    // for-byte (32 × 64 = 2048 B for layer params + 64 B common = 2112 B).
    layers: array<TerrainLayer, 32>,
    splat_map_index_0: u32,   // dead field; preserved for byte layout
    splat_map_index_1: u32,   // dead field; preserved for byte layout
    splat_uv_scale:    f32,
    triplanar_enabled: u32,
    normal_blend_method: u32,
    triplanar_slope_threshold: f32,
    height_blend_enabled: u32,
    active_layer_count: u32,
    // WGSL uniform-address-space arrays require element stride >= 16.
    // Previously `array<u32, 8>` (stride 4); naga rightly rejected this.
    // `array<vec4<u32>, 2>` is 32 bytes with correct stride, matching the
    // Rust side's `[u32; 8]` layout byte-for-byte.
    _pad: array<vec4<u32>, 2>,
}

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad0: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(1) @binding(0) var<uniform> terrain: TerrainParams;
@group(2) @binding(0) var terrain_sampler: sampler;
// Real-Fix.D 2026-05-08: 8 splat textures (was 2) for 32-channel weights.
// splat_map_i carries layers (i*4)..(i*4+3) in channels R..A.
@group(2) @binding(1) var splat_map_0: texture_2d<f32>;   // layers 0..3
@group(2) @binding(2) var splat_map_1: texture_2d<f32>;   // layers 4..7
@group(2) @binding(3) var splat_map_2: texture_2d<f32>;   // layers 8..11
@group(2) @binding(4) var splat_map_3: texture_2d<f32>;   // layers 12..15
@group(2) @binding(5) var splat_map_4: texture_2d<f32>;   // layers 16..19
@group(2) @binding(6) var splat_map_5: texture_2d<f32>;   // layers 20..23
@group(2) @binding(7) var splat_map_6: texture_2d<f32>;   // layers 24..27
@group(2) @binding(8) var splat_map_7: texture_2d<f32>;   // layers 28..31
@group(2) @binding(9)  var layer_albedo: texture_2d_array<f32>;
@group(2) @binding(10) var layer_normal: texture_2d_array<f32>;
@group(2) @binding(11) var layer_orm:    texture_2d_array<f32>;
@group(2) @binding(12) var layer_height: texture_2d_array<f32>;

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

// Sample a texture layer using triplanar projection with EXPLICIT gradients.
//
// The gradients must be supplied by the caller (computed via dpdx/dpdy of the
// world-space coordinates in UNIFORM control flow — i.e. outside any loop
// whose iteration count depends on a uniform). This avoids the FXC / DXC
// failure mode where implicit derivatives inside a varying-count loop cannot
// be unrolled.
fn sample_triplanar(
    tex: texture_2d_array<f32>,
    samp: sampler,
    layer_index: u32,
    world_pos: vec3<f32>,
    uv_scale: vec2<f32>,
    tri_w: vec3<f32>,
    ddx_yz: vec2<f32>,
    ddy_yz: vec2<f32>,
    ddx_xz: vec2<f32>,
    ddy_xz: vec2<f32>,
    ddx_xy: vec2<f32>,
    ddy_xy: vec2<f32>,
) -> vec4<f32> {
    let uv_x = world_pos.yz * uv_scale;
    let uv_y = world_pos.xz * uv_scale;
    let uv_z = world_pos.xy * uv_scale;

    let s_x = textureSampleGrad(tex, samp, uv_x, layer_index, ddx_yz * uv_scale, ddy_yz * uv_scale);
    let s_y = textureSampleGrad(tex, samp, uv_y, layer_index, ddx_xz * uv_scale, ddy_xz * uv_scale);
    let s_z = textureSampleGrad(tex, samp, uv_z, layer_index, ddx_xy * uv_scale, ddy_xy * uv_scale);

    return s_x * tri_w.x + s_y * tri_w.y + s_z * tri_w.z;
}

// Sample a texture layer using standard UV with EXPLICIT gradients.
fn sample_planar(
    tex: texture_2d_array<f32>,
    samp: sampler,
    layer_index: u32,
    uv: vec2<f32>,
    uv_scale: vec2<f32>,
    ddx_uv: vec2<f32>,
    ddy_uv: vec2<f32>,
) -> vec4<f32> {
    return textureSampleGrad(tex, samp, uv * uv_scale, layer_index, ddx_uv * uv_scale, ddy_uv * uv_scale);
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

    // Splat weights from eight RGBA maps (Real-Fix.D 2026-05-08)
    let splat_uv = in.uv * terrain.splat_uv_scale;
    let splat0 = textureSample(splat_map_0, terrain_sampler, splat_uv);
    let splat1 = textureSample(splat_map_1, terrain_sampler, splat_uv);
    let splat2 = textureSample(splat_map_2, terrain_sampler, splat_uv);
    let splat3 = textureSample(splat_map_3, terrain_sampler, splat_uv);
    let splat4 = textureSample(splat_map_4, terrain_sampler, splat_uv);
    let splat5 = textureSample(splat_map_5, terrain_sampler, splat_uv);
    let splat6 = textureSample(splat_map_6, terrain_sampler, splat_uv);
    let splat7 = textureSample(splat_map_7, terrain_sampler, splat_uv);

    var raw_weights: array<f32, 32>;
    raw_weights[0]  = splat0.r;  raw_weights[1]  = splat0.g;
    raw_weights[2]  = splat0.b;  raw_weights[3]  = splat0.a;
    raw_weights[4]  = splat1.r;  raw_weights[5]  = splat1.g;
    raw_weights[6]  = splat1.b;  raw_weights[7]  = splat1.a;
    raw_weights[8]  = splat2.r;  raw_weights[9]  = splat2.g;
    raw_weights[10] = splat2.b;  raw_weights[11] = splat2.a;
    raw_weights[12] = splat3.r;  raw_weights[13] = splat3.g;
    raw_weights[14] = splat3.b;  raw_weights[15] = splat3.a;
    raw_weights[16] = splat4.r;  raw_weights[17] = splat4.g;
    raw_weights[18] = splat4.b;  raw_weights[19] = splat4.a;
    raw_weights[20] = splat5.r;  raw_weights[21] = splat5.g;
    raw_weights[22] = splat5.b;  raw_weights[23] = splat5.a;
    raw_weights[24] = splat6.r;  raw_weights[25] = splat6.g;
    raw_weights[26] = splat6.b;  raw_weights[27] = splat6.a;
    raw_weights[28] = splat7.r;  raw_weights[29] = splat7.g;
    raw_weights[30] = splat7.b;  raw_weights[31] = splat7.a;

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

    // Pre-compute screen-space derivatives ONCE, in uniform control flow.
    // `textureSampleGrad` inside the per-layer loops consumes these, which
    // (a) matches WGSL spec requirements for derivatives at non-uniform call
    // sites and (b) avoids the FXC "gradient instruction in varying loop"
    // unroll failure on DX11 / fallback software adapters.
    let ddx_uv = dpdx(in.uv);
    let ddy_uv = dpdy(in.uv);
    let ddx_yz = dpdx(in.world_pos.yz);
    let ddy_yz = dpdy(in.world_pos.yz);
    let ddx_xz = dpdx(in.world_pos.xz);
    let ddy_xz = dpdy(in.world_pos.xz);
    let ddx_xy = dpdx(in.world_pos.xy);
    let ddy_xy = dpdy(in.world_pos.xy);

    // Sample height maps for height blending (32 layers per Real-Fix.D)
    var layer_heights: array<f32, 32>;
    for (var i: u32 = 0u; i < count; i++) {
        let layer = terrain.layers[i];
        let h_idx = layer.texture_indices.w;
        if (use_triplanar) {
            layer_heights[i] = sample_triplanar(
                layer_height, terrain_sampler, h_idx, in.world_pos, layer.uv_scale, tri_w,
                ddx_yz, ddy_yz, ddx_xz, ddy_xz, ddx_xy, ddy_xy,
            ).r;
        } else {
            layer_heights[i] = sample_planar(
                layer_height, terrain_sampler, h_idx, in.uv, layer.uv_scale,
                ddx_uv, ddy_uv,
            ).r;
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
            albedo = sample_triplanar(
                layer_albedo, terrain_sampler, a_idx, in.world_pos, layer.uv_scale, tri_w,
                ddx_yz, ddy_yz, ddx_xz, ddy_xz, ddx_xy, ddy_xy,
            );
            normal_sample = sample_triplanar(
                layer_normal, terrain_sampler, n_idx, in.world_pos, layer.uv_scale, tri_w,
                ddx_yz, ddy_yz, ddx_xz, ddy_xz, ddx_xy, ddy_xy,
            );
            orm_sample = sample_triplanar(
                layer_orm, terrain_sampler, o_idx, in.world_pos, layer.uv_scale, tri_w,
                ddx_yz, ddy_yz, ddx_xz, ddy_xz, ddx_xy, ddy_xy,
            );
        } else {
            albedo = sample_planar(
                layer_albedo, terrain_sampler, a_idx, in.uv, layer.uv_scale, ddx_uv, ddy_uv,
            );
            normal_sample = sample_planar(
                layer_normal, terrain_sampler, n_idx, in.uv, layer.uv_scale, ddx_uv, ddy_uv,
            );
            orm_sample = sample_planar(
                layer_orm, terrain_sampler, o_idx, in.uv, layer.uv_scale, ddx_uv, ddy_uv,
            );
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
