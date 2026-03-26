// ============================================================================
// AstraWeave PBR Terrain Shader — Texture-Based with Multi-Scale Detail
// ============================================================================
// Samples real PBR textures (albedo, normal, MRA) from texture arrays using
// triplanar mapping with dual-scale blending, hash-based tile rotation to
// break repetition, amplified normal maps, slope blending, Cook-Torrance
// BRDF, weather effects, ACES tone mapping, and atmospheric fog.

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    shading_mode: u32,
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_enabled: u32,
    weather_type: u32,
    time: f32,
    water_level: f32,
    // Lighting uniforms
    sun_dir: vec3<f32>,
    sun_intensity: f32,
    sun_color: vec3<f32>,
    ambient_intensity: f32,
    ambient_color: vec3<f32>,
    exposure: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) biome_weights_0: vec4<f32>,
    @location(4) biome_weights_1: vec4<f32>,
    @location(5) material_ids: vec4<f32>,
    @location(6) material_weights: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) biome_weights_0: vec4<f32>,
    @location(4) biome_weights_1: vec4<f32>,
    @location(5) @interpolate(flat) material_ids: vec4<f32>,
    @location(6) material_weights: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var biome_textures: texture_2d_array<f32>;

@group(0) @binding(2)
var biome_sampler: sampler;

@group(0) @binding(3)
var biome_normals: texture_2d_array<f32>;

@group(0) @binding(4)
var biome_mra: texture_2d_array<f32>;

// ─── Hash / Noise Utilities ───────────────────────────────────────────────────

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let p4 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract(vec2<f32>((p4.x + p4.y) * p4.z, (p4.x + p4.z) * p4.y));
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// ─── Tiling-Break UV Rotation ─────────────────────────────────────────────────
// Rotates UV by a hash-derived angle per tile to break visible repetition.

fn rotate_uv(uv: vec2<f32>, tile_id: vec2<f32>) -> vec2<f32> {
    let angle = hash21(tile_id) * 6.2832; // 0..2π
    let s = sin(angle);
    let c = cos(angle);
    let center = tile_id + 0.5;
    let local = uv - center;
    return vec2<f32>(local.x * c - local.y * s, local.x * s + local.y * c) + center;
}

// ─── Triplanar Mapping ────────────────────────────────────────────────────────

fn triplanar_weights(normal: vec3<f32>) -> vec3<f32> {
    var w = abs(normal);
    w = pow(w, vec3<f32>(4.0));
    return w / (w.x + w.y + w.z + 0.0001);
}

fn triplanar_sample_albedo(pos: vec3<f32>, n: vec3<f32>, scale: f32, layer: i32) -> vec3<f32> {
    let w = triplanar_weights(n);
    let uv_xz = pos.xz * scale;
    let uv_xy = pos.xy * scale;
    let uv_yz = pos.yz * scale;
    let bias = -0.5;
    let t_xz = textureSampleBias(biome_textures, biome_sampler, uv_xz, layer, bias).rgb;
    let t_xy = textureSampleBias(biome_textures, biome_sampler, uv_xy, layer, bias).rgb;
    let t_yz = textureSampleBias(biome_textures, biome_sampler, uv_yz, layer, bias).rgb;
    return t_xz * w.y + t_xy * w.z + t_yz * w.x;
}

fn triplanar_sample_normal_raw(pos: vec3<f32>, n: vec3<f32>, scale: f32, layer: i32) -> vec3<f32> {
    let w = triplanar_weights(n);
    let uv_xz = pos.xz * scale;
    let uv_xy = pos.xy * scale;
    let uv_yz = pos.yz * scale;
    let bias = -0.5;
    let t_xz = textureSampleBias(biome_normals, biome_sampler, uv_xz, layer, bias).rgb;
    let t_xy = textureSampleBias(biome_normals, biome_sampler, uv_xy, layer, bias).rgb;
    let t_yz = textureSampleBias(biome_normals, biome_sampler, uv_yz, layer, bias).rgb;
    return t_xz * w.y + t_xy * w.z + t_yz * w.x;
}

fn triplanar_sample_mra(pos: vec3<f32>, n: vec3<f32>, scale: f32, layer: i32) -> vec3<f32> {
    let w = triplanar_weights(n);
    let uv_xz = pos.xz * scale;
    let uv_xy = pos.xy * scale;
    let uv_yz = pos.yz * scale;
    let bias = -0.5;
    let t_xz = textureSampleBias(biome_mra, biome_sampler, uv_xz, layer, bias).rgb;
    let t_xy = textureSampleBias(biome_mra, biome_sampler, uv_xy, layer, bias).rgb;
    let t_yz = textureSampleBias(biome_mra, biome_sampler, uv_yz, layer, bias).rgb;
    return t_xz * w.y + t_xy * w.z + t_yz * w.x;
}

// ─── Normal Map Decoding with Strength ────────────────────────────────────────

fn decode_normal_map(tex_normal: vec3<f32>, vertex_normal: vec3<f32>, strength: f32) -> vec3<f32> {
    // Decode from [0,1] to [-1,1]
    var tn = tex_normal * 2.0 - 1.0;

    // Amplify tangent-space XY for more visible detail
    tn = vec3<f32>(tn.x * strength, tn.y * strength, tn.z);

    // Build tangent frame from vertex normal
    let up = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(vertex_normal.y) < 0.999);
    let tangent = normalize(cross(up, vertex_normal));
    let bitangent = cross(vertex_normal, tangent);

    // Transform tangent-space normal to world space
    return normalize(tangent * tn.x + bitangent * tn.y + vertex_normal * tn.z);
}

// ─── Material Structure ───────────────────────────────────────────────────────

struct Material {
    albedo: vec3<f32>,
    roughness: f32,
    metallic: f32,
    ao: f32,
    normal: vec3<f32>,
    height_proxy: f32,
}

fn dominant_biome_layer(weights_0: vec4<f32>, weights_1: vec4<f32>) -> i32 {
    var best_index = 0i;
    var best_weight = weights_0.x;

    if weights_0.y > best_weight { best_weight = weights_0.y; best_index = 1; }
    if weights_0.z > best_weight { best_weight = weights_0.z; best_index = 2; }
    if weights_0.w > best_weight { best_weight = weights_0.w; best_index = 3; }
    if weights_1.x > best_weight { best_weight = weights_1.x; best_index = 4; }
    if weights_1.y > best_weight { best_weight = weights_1.y; best_index = 5; }
    if weights_1.z > best_weight { best_weight = weights_1.z; best_index = 6; }
    if weights_1.w > best_weight { best_weight = weights_1.w; best_index = 7; }

    return remap_biome_layer(best_index);
}

// Remap biome layer indices: Beach (6) shares sand texture at layer 1
fn remap_biome_layer(idx: i32) -> i32 {
    if idx == 6 { return 1; }
    return idx;
}

fn dominant_material_layer(material_ids: vec4<f32>, material_weights: vec4<f32>) -> i32 {
    var best_index = 0;
    var best_weight = material_weights.x;

    if material_weights.y > best_weight { best_weight = material_weights.y; best_index = 1; }
    if material_weights.z > best_weight { best_weight = material_weights.z; best_index = 2; }
    if material_weights.w > best_weight { best_weight = material_weights.w; best_index = 3; }

    // material_ids are flat-interpolated, but round as safety net
    switch best_index {
        case 0: { return i32(material_ids.x + 0.5); }
        case 1: { return i32(material_ids.y + 0.5); }
        case 2: { return i32(material_ids.z + 0.5); }
        case 3: { return i32(material_ids.w + 0.5); }
        default: { return i32(material_ids.x + 0.5); }
    }
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn material_weights_max(w: vec4<f32>) -> f32 {
    return max(max(w.x, w.y), max(w.z, w.w));
}

fn material_params(layer: i32) -> vec4<f32> {
    // (macro_scale, detail_scale, normal_strength, detail_mix)
    switch layer {
        case 0: { return vec4<f32>(0.20, 0.65, 1.5, 0.55); }  // grass
        case 1: { return vec4<f32>(0.14, 0.45, 1.2, 0.35); } // sand
        case 2: { return vec4<f32>(0.18, 0.60, 1.6, 0.55); } // forest floor
        case 3: { return vec4<f32>(0.12, 0.38, 1.8, 0.40); } // mountain rock
        case 4: { return vec4<f32>(0.11, 0.35, 1.0, 0.30); } // snow
        case 5: { return vec4<f32>(0.16, 0.50, 1.4, 0.45); } // mud
        case 6: { return vec4<f32>(0.14, 0.45, 1.4, 0.40); } // wood planks
        case 7: { return vec4<f32>(0.13, 0.42, 1.6, 0.40); } // stone
        case 8: { return vec4<f32>(0.12, 0.35, 1.8, 0.38); } // rock slate
        case 9: { return vec4<f32>(0.16, 0.52, 1.3, 0.48); } // dirt
        case 10: { return vec4<f32>(0.13, 0.42, 1.6, 0.40); } // cobblestone
        case 11: { return vec4<f32>(0.18, 0.60, 0.8, 0.45); } // cloth
        case 12: { return vec4<f32>(0.16, 0.52, 1.2, 0.42); } // default
        case 13: { return vec4<f32>(0.15, 0.48, 1.4, 0.45); } // gravel
        case 14: { return vec4<f32>(0.11, 0.35, 0.8, 0.30); } // ice
        case 15: { return vec4<f32>(0.12, 0.38, 1.8, 0.40); } // metal rusted
        case 16: { return vec4<f32>(0.18, 0.58, 1.2, 0.52); } // moss
        case 17: { return vec4<f32>(0.15, 0.48, 1.0, 0.40); } // plaster
        case 18: { return vec4<f32>(0.13, 0.40, 1.6, 0.42); } // rock lichen
        case 19: { return vec4<f32>(0.14, 0.45, 1.4, 0.42); } // roof tile
        case 20: { return vec4<f32>(0.15, 0.48, 1.6, 0.45); } // tree bark
        case 21: { return vec4<f32>(0.18, 0.65, 1.0, 0.50); } // tree leaves
        default: { return vec4<f32>(0.16, 0.52, 1.4, 0.42); }
    }
}

fn macro_variation_masks(pos: vec3<f32>, layer: i32) -> vec3<f32> {
    let layer_f = f32(layer);
    let n0 = noise2d(pos.xz * 0.008 + vec2<f32>(layer_f * 1.73, layer_f * 0.91));
    let n1 = noise2d(pos.xz * 0.017 + vec2<f32>(17.1 + layer_f * 0.61, 9.4 + layer_f * 1.27));
    let n2 = noise2d(pos.xz * 0.031 + vec2<f32>(33.7 + layer_f * 1.11, 4.8 + layer_f * 0.53));
    return vec3<f32>(n0, n1, n2);
}

fn warped_material_pos(pos: vec3<f32>, layer: i32) -> vec3<f32> {
    let layer_f = f32(layer);
    let warp_x = noise2d(pos.xz * 0.012 + vec2<f32>(layer_f * 2.13, 7.1)) - 0.5;
    let warp_z = noise2d((pos.xz + vec2<f32>(31.7, 18.4)) * 0.012 + vec2<f32>(5.2, layer_f * 1.71)) - 0.5;
    return pos + vec3<f32>(warp_x * 1.8, 0.0, warp_z * 1.8);
}

// ─── Multi-Scale PBR Sampling ─────────────────────────────────────────────────
// Samples textures at two scales and blends for visible detail at all distances.

fn sample_biome_material(pos: vec3<f32>, n: vec3<f32>, layer: i32) -> Material {
    let params = material_params(layer);
    let macro_scale = params.x;
    let detail_scale = params.y;
    let normal_strength = params.z;
    let detail_mix = params.w;
    let warped_pos = warped_material_pos(pos, layer);
    let variation = macro_variation_masks(pos, layer);

    // Distance-based LOD: fade detail at far range to save ALU
    let cam_dist = distance(uniforms.camera_pos, pos);
    let detail_fade = 1.0 - smoothstep(300.0, 800.0, cam_dist);

    // Macro samples (always present)
    let macro_albedo = triplanar_sample_albedo(warped_pos, n, macro_scale, layer);
    let macro_nm = triplanar_sample_normal_raw(warped_pos, n, macro_scale, layer);
    let macro_mra = triplanar_sample_mra(warped_pos, n, macro_scale, layer);

    // Detail samples (fade out at distance)
    var albedo = macro_albedo;
    var nm_raw = macro_nm;
    var roughness = macro_mra.g;
    var metallic = macro_mra.r;
    var ao = macro_mra.b;
    if detail_fade > 0.01 {
        let detail_albedo = triplanar_sample_albedo(warped_pos, n, detail_scale, layer);
        let detail_nm = triplanar_sample_normal_raw(warped_pos, n, detail_scale, layer);
        // Overlay detail: multiply-blend albedo for natural micro-variation
        let detail_overlay = detail_albedo / max(macro_albedo, vec3<f32>(0.01));
        let overlay_blended = mix(vec3<f32>(1.0), detail_overlay, detail_mix * detail_fade);
        albedo = macro_albedo * overlay_blended;
        // Blend normals: detail adds high-frequency perturbation
        nm_raw = mix(macro_nm, detail_nm, (0.30 + detail_mix * 0.35) * detail_fade);
    }

    // Cheap macro breakup for color and surface response without extra texture fetches.
    let tint = mix(vec3<f32>(0.92, 0.95, 0.98), vec3<f32>(1.08, 1.03, 0.96), variation.x);
    let hue_bias = mix(vec3<f32>(0.98, 0.99, 1.02), vec3<f32>(1.03, 1.00, 0.96), variation.y);
    albedo *= tint * hue_bias;
    roughness = clamp(roughness + (variation.z - 0.5) * 0.14, 0.04, 1.0);
    metallic = clamp(metallic + (variation.y - 0.5) * 0.03, 0.0, 1.0);
    ao = clamp(ao * mix(0.92, 1.08, variation.x), 0.0, 1.0);

    var mat: Material;
    mat.albedo = albedo;
    mat.normal = decode_normal_map(nm_raw, n, normal_strength);
    mat.metallic = metallic;
    mat.roughness = roughness;
    mat.ao = ao;
    mat.height_proxy = clamp(
        0.45 * luminance(macro_albedo)
            + 0.25 * macro_nm.z
            + 0.20 * (1.0 - roughness)
            + 0.10 * variation.x,
        0.0,
        1.0,
    );
    return mat;
}

// Helper: read biome weight by index without array indexing
fn biome_weight(w0: vec4<f32>, w1: vec4<f32>, idx: i32) -> f32 {
    switch idx {
        case 0: { return w0.x; }
        case 1: { return w0.y; }
        case 2: { return w0.z; }
        case 3: { return w0.w; }
        case 4: { return w1.x; }
        case 5: { return w1.y; }
        case 6: { return w1.z; }
        case 7: { return w1.w; }
        default: { return 0.0; }
    }
}

fn blend_biome_materials(pos: vec3<f32>, n: vec3<f32>, weights_0: vec4<f32>, weights_1: vec4<f32>) -> Material {
    // Performance: only sample the top 2 biome layers instead of all 8.
    var best_idx = 0i;
    var best_w = weights_0.x;
    var second_idx = -1i;
    var second_w = 0.0;

    // Find top 2 weights
    if weights_0.y > best_w { second_w = best_w; second_idx = best_idx; best_w = weights_0.y; best_idx = 1; } else if weights_0.y > second_w { second_w = weights_0.y; second_idx = 1; }
    if weights_0.z > best_w { second_w = best_w; second_idx = best_idx; best_w = weights_0.z; best_idx = 2; } else if weights_0.z > second_w { second_w = weights_0.z; second_idx = 2; }
    if weights_0.w > best_w { second_w = best_w; second_idx = best_idx; best_w = weights_0.w; best_idx = 3; } else if weights_0.w > second_w { second_w = weights_0.w; second_idx = 3; }
    if weights_1.x > best_w { second_w = best_w; second_idx = best_idx; best_w = weights_1.x; best_idx = 4; } else if weights_1.x > second_w { second_w = weights_1.x; second_idx = 4; }
    if weights_1.y > best_w { second_w = best_w; second_idx = best_idx; best_w = weights_1.y; best_idx = 5; } else if weights_1.y > second_w { second_w = weights_1.y; second_idx = 5; }
    if weights_1.z > best_w { second_w = best_w; second_idx = best_idx; best_w = weights_1.z; best_idx = 6; } else if weights_1.z > second_w { second_w = weights_1.z; second_idx = 6; }
    if weights_1.w > best_w { second_w = best_w; second_idx = best_idx; best_w = weights_1.w; best_idx = 7; } else if weights_1.w > second_w { second_w = weights_1.w; second_idx = 7; }

    // Remap biome indices to texture layers (Beach→Sand)
    best_idx = remap_biome_layer(best_idx);
    if second_idx >= 0 {
        second_idx = remap_biome_layer(second_idx);
    }

    if best_w < 0.0001 {
        return sample_biome_material(pos, n, 0);
    }

    let mat1 = sample_biome_material(pos, n, best_idx);

    // If second layer is negligible, return dominant only
    if second_w < 0.01 || second_idx < 0 {
        return mat1;
    }

    let mat2 = sample_biome_material(pos, n, second_idx);
    let blend = second_w / (best_w + second_w);

    var result: Material;
    result.albedo = mix(mat1.albedo, mat2.albedo, blend);
    result.roughness = mix(mat1.roughness, mat2.roughness, blend);
    result.metallic = mix(mat1.metallic, mat2.metallic, blend);
    result.ao = mix(mat1.ao, mat2.ao, blend);
    result.normal = normalize(mix(mat1.normal, mat2.normal, blend));
    result.height_proxy = mix(mat1.height_proxy, mat2.height_proxy, blend);
    return result;
}

// ─── Material-ID Slot Blending (replaces old channel-based splat) ──────────
// Blends up to 4 arbitrary material layers per vertex using explicit
// material_ids (layer index 0-21) and material_weights (sum to 1.0).

fn blend_material_slots(
    pos: vec3<f32>,
    n: vec3<f32>,
    material_ids: vec4<f32>,
    material_weights: vec4<f32>,
) -> Material {
    var albedo = vec3<f32>(0.0);
    var roughness = 0.0;
    var metallic = 0.0;
    var ao = 0.0;
    var normal_sum = vec3<f32>(0.0);
    var height_sum = 0.0;
    var total_weight = 0.0;

    // material_ids are @interpolate(flat) — use integer layer indices directly
    let id0 = i32(material_ids.x + 0.5);
    let id1 = i32(material_ids.y + 0.5);
    let id2 = i32(material_ids.z + 0.5);
    let id3 = i32(material_ids.w + 0.5);

    if material_weights.x > 0.001 {
        let mat = sample_biome_material(pos, n, id0);
        let w = material_weights.x;
        albedo += mat.albedo * w;
        roughness += mat.roughness * w;
        metallic += mat.metallic * w;
        ao += mat.ao * w;
        normal_sum += mat.normal * w;
        height_sum += mat.height_proxy * w;
        total_weight += w;
    }
    if material_weights.y > 0.001 {
        let mat = sample_biome_material(pos, n, id1);
        let w = material_weights.y;
        albedo += mat.albedo * w;
        roughness += mat.roughness * w;
        metallic += mat.metallic * w;
        ao += mat.ao * w;
        normal_sum += mat.normal * w;
        height_sum += mat.height_proxy * w;
        total_weight += w;
    }
    if material_weights.z > 0.001 {
        let mat = sample_biome_material(pos, n, id2);
        let w = material_weights.z;
        albedo += mat.albedo * w;
        roughness += mat.roughness * w;
        metallic += mat.metallic * w;
        ao += mat.ao * w;
        normal_sum += mat.normal * w;
        height_sum += mat.height_proxy * w;
        total_weight += w;
    }
    if material_weights.w > 0.001 {
        let mat = sample_biome_material(pos, n, id3);
        let w = material_weights.w;
        albedo += mat.albedo * w;
        roughness += mat.roughness * w;
        metallic += mat.metallic * w;
        ao += mat.ao * w;
        normal_sum += mat.normal * w;
        height_sum += mat.height_proxy * w;
        total_weight += w;
    }

    if total_weight < 0.0001 {
        return sample_biome_material(pos, n, 0);
    }

    var result: Material;
    result.albedo = albedo;
    result.roughness = roughness;
    result.metallic = metallic;
    result.ao = ao;
    result.normal = normalize(normal_sum);
    result.height_proxy = clamp(height_sum, 0.0, 1.0);
    return result;
}

// ─── Rock Material (for slope blending, uses mountain_rock layer 3) ───────────

fn rock_material(pos: vec3<f32>, n: vec3<f32>) -> Material {
    return sample_biome_material(pos, n, 3);
}

fn apply_slope_blend(biome_mat: Material, pos: vec3<f32>, n: vec3<f32>) -> Material {
    let slope = 1.0 - max(dot(n, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
    let rock_blend = smoothstep(0.72, 0.95, slope);
    if rock_blend < 0.01 {
        return biome_mat;
    }
    let rock = rock_material(pos, n);
    var result: Material;
    result.albedo = mix(biome_mat.albedo, rock.albedo, rock_blend);
    result.roughness = mix(biome_mat.roughness, rock.roughness, rock_blend);
    result.metallic = mix(biome_mat.metallic, rock.metallic, rock_blend);
    result.ao = mix(biome_mat.ao, rock.ao, rock_blend);
    result.normal = normalize(mix(biome_mat.normal, rock.normal, rock_blend));
    result.height_proxy = mix(biome_mat.height_proxy, rock.height_proxy, rock_blend);
    return result;
}

// ─── PBR Lighting (Cook-Torrance BRDF) ────────────────────────────────────────

const PI: f32 = 3.14159265359;

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom + 0.0001);
}

fn geometry_schlick(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k + 0.0001);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return geometry_schlick(n_dot_v, roughness) * geometry_schlick(n_dot_l, roughness);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn mix_material(a: Material, b: Material, t: f32) -> Material {
    var m: Material;
    m.albedo = mix(a.albedo, b.albedo, t);
    m.normal = normalize(mix(a.normal, b.normal, t));
    m.roughness = mix(a.roughness, b.roughness, t);
    m.metallic = mix(a.metallic, b.metallic, t);
    m.ao = mix(a.ao, b.ao, t);
    m.height_proxy = mix(a.height_proxy, b.height_proxy, t);
    return m;
}

fn pbr_lighting(mat: Material, pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(uniforms.sun_dir);
    let light_color = uniforms.sun_color * uniforms.sun_intensity;
    let view_dir = normalize(uniforms.camera_pos - pos);
    let half_dir = normalize(light_dir + view_dir);
    let n_dot_l = max(dot(n, light_dir), 0.0);
    let n_dot_v = max(dot(n, view_dir), 0.001);
    let n_dot_h = max(dot(n, half_dir), 0.0);
    let h_dot_v = max(dot(half_dir, view_dir), 0.0);
    let f0 = mix(vec3<f32>(0.04), mat.albedo, mat.metallic);
    let D = distribution_ggx(n_dot_h, mat.roughness);
    let G = geometry_smith(n_dot_v, n_dot_l, mat.roughness);
    let F = fresnel_schlick(h_dot_v, f0);
    let spec = (D * G * F) / (4.0 * n_dot_v * n_dot_l + 0.0001);
    let kS = F;
    let kD = (vec3<f32>(1.0) - kS) * (1.0 - mat.metallic);
    let direct = (kD * mat.albedo / PI + spec) * light_color * n_dot_l;
    // Hemisphere ambient from uniform colors — AO provides depth in crevices
    let ground_c = uniforms.ambient_color * 0.40;
    let amb_blend = n.y * 0.5 + 0.5;
    let ambient = mix(ground_c, uniforms.ambient_color, amb_blend) * mat.albedo * mat.ao * uniforms.ambient_intensity;
    // Minimal indirect fill — avoids washing out shadow contrast
    let indirect = mat.albedo * 0.04;
    // Subtle warm rim
    let rim = pow(1.0 - n_dot_v, 4.0) * 0.03;
    return direct + ambient + indirect + vec3<f32>(rim * 1.0, rim * 0.9, rim * 0.7);
}

// ─── Vertex Shader ────────────────────────────────────────────────────────────

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // Camera-relative transform: subtract camera_pos to avoid f32 jitter far from origin
    let rel_pos = vertex.position - uniforms.camera_pos;
    output.clip_position = uniforms.view_proj * vec4<f32>(rel_pos, 1.0);
    output.world_position = vertex.position;
    output.world_normal = normalize(vertex.normal);
    output.uv = vertex.uv;
    output.biome_weights_0 = vertex.biome_weights_0;
    output.biome_weights_1 = vertex.biome_weights_1;
    output.material_ids = vertex.material_ids;
    output.material_weights = vertex.material_weights;
    return output;
}

// ─── Fragment Shader ──────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let pos = in.world_position;
    let cam_dist = distance(uniforms.camera_pos, pos);
    // Noise-perturbed distance breaks the perfect circular LOD boundary
    let lod_noise = noise2d(pos.xz * 0.04 + vec2<f32>(5.3, 11.7));
    let perturbed_dist = cam_dist + (lod_noise - 0.5) * 30.0;
    // Smooth LOD blend factors — wide transition zones prevent visible rings
    let near_blend = 1.0 - smoothstep(400.0, 1200.0, perturbed_dist);  // 1 close, 0 far
    let mid_blend  = 1.0 - smoothstep(1000.0, 2500.0, perturbed_dist); // 1 close, 0 far

    // Unlit: quick sample at macro scale only
    if uniforms.shading_mode == 1u {
        let mat = sample_biome_material(pos, n, dominant_biome_layer(in.biome_weights_0, in.biome_weights_1));
        return vec4<f32>(mat.albedo, 1.0);
    }
    // Wireframe
    if uniforms.shading_mode == 2u {
        return vec4<f32>(0.1, 0.1, 0.1, 1.0);
    }

    // Full PBR with multi-scale texture sampling — smooth crossfade at LOD boundaries
    let dominant_biome_idx = dominant_biome_layer(in.biome_weights_0, in.biome_weights_1);
    let biome_far = sample_biome_material(pos, n, dominant_biome_idx);
    var biome_mat: Material;
    if mid_blend > 0.01 {
        let biome_full = blend_biome_materials(pos, n, in.biome_weights_0, in.biome_weights_1);
        biome_mat = mix_material(biome_far, biome_full, mid_blend);
    } else {
        biome_mat = biome_far;
    }

    let dominant_mat_idx = dominant_material_layer(in.material_ids, in.material_weights);
    let max_mat_w = material_weights_max(in.material_weights);
    var splat_mat: Material;
    // Always compute splat_mat so painted materials are visible at every distance
    if near_blend > 0.01 {
        let splat_near = blend_material_slots(
            pos,
            n,
            in.material_ids,
            in.material_weights,
        );
        if mid_blend > 0.01 {
            let splat_mid = sample_biome_material(pos, n, dominant_mat_idx);
            splat_mat = mix_material(splat_mid, splat_near, near_blend);
        } else {
            splat_mat = splat_near;
        }
    } else {
        splat_mat = sample_biome_material(pos, n, dominant_mat_idx);
    }
    // Compute local breakup from material weight diversity
    let local_breakup = clamp(
        1.0 - max_mat_w,
        0.0,
        1.0,
    );
    let transition_noise = noise2d(pos.xz * 0.022 + vec2<f32>(8.7, 2.3));
    let height_factor = smoothstep(-8.0, 120.0, pos.y);
    let height_transition = smoothstep(-0.18, 0.22, splat_mat.height_proxy - biome_mat.height_proxy);
    let base_local_mix = clamp(0.34 + 0.28 * local_breakup + 0.18 * transition_noise + 0.10 * height_factor, 0.26, 0.86);
    // tier_mix: ensure material detail persists at distance
    let tier_mix = clamp(near_blend + (1.0 - near_blend) * mid_blend * 0.5, 0.0, 1.0);
    var local_mix = clamp((base_local_mix * 0.82 + height_transition * 0.18) * max(tier_mix, 0.3), 0.0, 1.0);
    // Paint boost: when multiple material slots are active (painted), increase blend
    // Detect painting by weight diversity: unpainted has max_w≈1.0, painted has max_w<0.95
    let paint_diversity = 1.0 - max_mat_w; // 0 = single material, >0 = blended/painted
    let paint_boost = smoothstep(0.02, 0.25, paint_diversity);
    local_mix = mix(local_mix, max(local_mix, 0.85), paint_boost);

    var mat: Material;
    mat.albedo = mix(biome_mat.albedo, splat_mat.albedo, local_mix);
    mat.roughness = mix(biome_mat.roughness, splat_mat.roughness, min(0.75, local_mix + 0.16));
    mat.metallic = mix(biome_mat.metallic, splat_mat.metallic, min(0.72, local_mix + 0.12));
    mat.ao = mix(biome_mat.ao, splat_mat.ao, min(0.72, local_mix + 0.12));
    mat.normal = normalize(mix(biome_mat.normal, splat_mat.normal, min(0.82, local_mix + 0.22)));
    mat.height_proxy = mix(biome_mat.height_proxy, splat_mat.height_proxy, local_mix);

    // Slope-based rock blending (smoothly faded at distance, noise-perturbed)
    let slope_blend_factor = 1.0 - smoothstep(1200.0, 2800.0, perturbed_dist);
    if slope_blend_factor > 0.01 {
        let slope_mat = apply_slope_blend(mat, pos, n);
        mat.albedo = mix(mat.albedo, slope_mat.albedo, slope_blend_factor);
        mat.roughness = mix(mat.roughness, slope_mat.roughness, slope_blend_factor);
        mat.metallic = mix(mat.metallic, slope_mat.metallic, slope_blend_factor);
        mat.ao = mix(mat.ao, slope_mat.ao, slope_blend_factor);
        mat.normal = normalize(mix(mat.normal, slope_mat.normal, slope_blend_factor));
        mat.height_proxy = mix(mat.height_proxy, slope_mat.height_proxy, slope_blend_factor);
    }

    // Weather effects on material
    if uniforms.weather_type == 4u {
        let flat_f = max(dot(n, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
        let snow_amt = smoothstep(0.4, 0.8, flat_f) * 0.7;
        mat.albedo = mix(mat.albedo, vec3<f32>(0.85, 0.87, 0.92), snow_amt);
        mat.roughness = mix(mat.roughness, 0.6, snow_amt);
    } else if uniforms.weather_type == 2u || uniforms.weather_type == 3u {
        let wet = select(0.4, 0.7, uniforms.weather_type == 3u);
        mat.roughness = mix(mat.roughness, 0.1, wet);
        mat.albedo *= 1.0 - wet * 0.15;
    } else if uniforms.weather_type == 6u {
        mat.albedo = mix(mat.albedo, vec3<f32>(0.42, 0.25, 0.07), 0.2);
        mat.roughness = min(mat.roughness + 0.1, 1.0);
    }

    var color = pbr_lighting(mat, pos, mat.normal);

    // Tone map — ACES with exposure control
    color *= uniforms.exposure;
    let a = color * (2.51 * color + vec3<f32>(0.03));
    let b = color * (2.43 * color + vec3<f32>(0.59)) + vec3<f32>(0.14);
    color = clamp(a / b, vec3<f32>(0.0), vec3<f32>(1.0));

    // Fog (gentle atmospheric, height-aware) — capped to prevent white-out circle
    if uniforms.fog_enabled == 1u {
        let dist = distance(uniforms.camera_pos, pos);
        let fog_base = 1.0 - exp(-uniforms.fog_density * dist);
        let height_att = smoothstep(0.0, 60.0, pos.y);
        let fog_f = fog_base * mix(0.7, 0.35, height_att);
        // Subtle warm tint — minimal color shift so fog doesn't wash to white
        let warm_fog = uniforms.fog_color * vec3<f32>(1.03, 1.01, 0.97);
        color = mix(color, warm_fog, clamp(fog_f, 0.0, 0.65));
    }

    return vec4<f32>(color, 1.0);
}
