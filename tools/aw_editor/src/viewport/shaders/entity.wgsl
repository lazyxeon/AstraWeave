// Entity PBR Shader
//
// Physically-based rendering with Cook-Torrance BRDF for viewport entities.
// Supports scene lights (directional sun + up to 4 point lights).
// Material textures: albedo, normal, ORM (occlusion/roughness/metallic), emissive.
// Shading modes: 0=PBR Lit, 1=Unlit, 2=Wireframe
// ACES filmic tone mapping for HDR → LDR conversion.

const PI: f32 = 3.14159265359;

// ─── Group 0: Scene Uniforms (unchanged from legacy) ─────────────────────────
struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    shading_mode: u32,
    sun_dir_and_count: vec4<f32>,       // xyz=sun direction, w=point light count
    sun_color_and_intensity: vec4<f32>, // xyz=sun color, w=sun intensity
    ambient_color_and_intensity: vec4<f32>, // xyz=ambient color, w=ambient intensity
    light0_pos_range: vec4<f32>,
    light0_color_intensity: vec4<f32>,
    light1_pos_range: vec4<f32>,
    light1_color_intensity: vec4<f32>,
    light2_pos_range: vec4<f32>,
    light2_color_intensity: vec4<f32>,
    light3_pos_range: vec4<f32>,
    light3_color_intensity: vec4<f32>,
    // Shadow mapping (4-cascade CSM)
    shadow_vp_0: mat4x4<f32>,          // Cascade 0 light VP matrix (near)
    shadow_vp_1: mat4x4<f32>,          // Cascade 1 light VP matrix
    shadow_vp_2: mat4x4<f32>,          // Cascade 2 light VP matrix
    shadow_vp_3: mat4x4<f32>,          // Cascade 3 light VP matrix (far)
    cascade_splits: vec4<f32>,          // view-space far distances for cascades 0-3
    shadow_params: vec4<f32>,           // x=bias, y=normal_bias, z=enabled(0/1), w=texel_size
    // Color management
    exposure_params: vec4<f32>,         // x=exposure_ev (EV compensation), y/z/w=reserved
}

// ─── Group 2: Per-Material Parameters (vec4-packed for alignment safety) ──────
struct MaterialParams {
    base_color_factor: vec4<f32>,        // rgba base color multiplier
    emissive_and_metallic: vec4<f32>,    // xyz=emissive factor, w=metallic factor
    pbr_params: vec4<f32>,               // x=roughness, y=emissive_strength, z=occlusion_strength, w=alpha_cutoff
    extra_params: vec4<f32>,             // x=ior, y=clearcoat_factor, z=clearcoat_roughness, w=alpha_mode (0/1/2 as f32)
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(8) uv: vec2<f32>,
    @location(9) tangent: vec4<f32>,  // xyz=tangent direction, w=handedness
}

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
    @location(7) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) world_tangent: vec4<f32>,  // xyz=tangent, w=handedness
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// ─── Group 1: Material Textures ──────────────────────────────────────────────
@group(1) @binding(0) var albedo_texture: texture_2d<f32>;
@group(1) @binding(1) var material_sampler: sampler;
@group(1) @binding(2) var normal_texture: texture_2d<f32>;
@group(1) @binding(3) var orm_texture: texture_2d<f32>;     // R=occlusion, G=roughness, B=metallic
@group(1) @binding(4) var emissive_texture: texture_2d<f32>;

@group(2) @binding(0)
var<uniform> material: MaterialParams;

// ─── Group 3: Shadow Map (4-cascade array) ──────────────────────────────────
@group(3) @binding(0) var shadow_map: texture_depth_2d_array;
@group(3) @binding(1) var shadow_sampler: sampler_comparison;

// ─── Shadow Map Sampling (4-cascade CSM, 5-tap PCF) ─────────────────────────
fn sample_shadow(world_pos: vec3<f32>, n: vec3<f32>) -> f32 {
    let enabled = uniforms.shadow_params.z;
    if enabled < 0.5 { return 1.0; }

    let bias = uniforms.shadow_params.x;
    let normal_bias = uniforms.shadow_params.y;
    let texel_size = uniforms.shadow_params.w;

    // Select cascade based on distance from camera
    let dist = length(world_pos - uniforms.camera_pos);
    var cascade_index: i32 = 3;
    if dist < uniforms.cascade_splits.x {
        cascade_index = 0;
    } else if dist < uniforms.cascade_splits.y {
        cascade_index = 1;
    } else if dist < uniforms.cascade_splits.z {
        cascade_index = 2;
    }

    // Get the VP matrix for the selected cascade
    var shadow_vp: mat4x4<f32>;
    switch cascade_index {
        case 0: { shadow_vp = uniforms.shadow_vp_0; }
        case 1: { shadow_vp = uniforms.shadow_vp_1; }
        case 2: { shadow_vp = uniforms.shadow_vp_2; }
        default: { shadow_vp = uniforms.shadow_vp_3; }
    }

    // Apply normal offset bias
    let biased_pos = world_pos + n * normal_bias;

    // Project to light clip space
    let shadow_clip = shadow_vp * vec4<f32>(biased_pos, 1.0);
    let shadow_ndc = shadow_clip.xyz / shadow_clip.w;

    let shadow_uv = vec2<f32>(
        shadow_ndc.x * 0.5 + 0.5,
        -shadow_ndc.y * 0.5 + 0.5
    );
    let compare_depth = shadow_ndc.z - bias;

    // Outside shadow frustum → fully lit
    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 || compare_depth > 1.0 || compare_depth < 0.0 {
        return 1.0;
    }

    // 5-tap PCF on the selected cascade layer
    var shadow = textureSampleCompare(shadow_map, shadow_sampler, shadow_uv, cascade_index, compare_depth);
    shadow += textureSampleCompare(shadow_map, shadow_sampler, shadow_uv + vec2<f32>(-texel_size, 0.0), cascade_index, compare_depth);
    shadow += textureSampleCompare(shadow_map, shadow_sampler, shadow_uv + vec2<f32>(texel_size, 0.0), cascade_index, compare_depth);
    shadow += textureSampleCompare(shadow_map, shadow_sampler, shadow_uv + vec2<f32>(0.0, -texel_size), cascade_index, compare_depth);
    shadow += textureSampleCompare(shadow_map, shadow_sampler, shadow_uv + vec2<f32>(0.0, texel_size), cascade_index, compare_depth);
    return shadow / 5.0;
}

// ─── Group 4: IBL (Image-Based Lighting) ─────────────────────────────────────
// Diffuse IBL uses L2 spherical harmonics (9 vec4 coefficients) passed as uniform.
// Specular IBL uses BRDF LUT + reflection vector analytical approximation.
struct IblParams {
    // SH L2 irradiance coefficients (9 vec3 packed as vec4, w unused)
    sh0: vec4<f32>,     // L00
    sh1: vec4<f32>,     // L1-1
    sh2: vec4<f32>,     // L10
    sh3: vec4<f32>,     // L11
    sh4: vec4<f32>,     // L2-2
    sh5: vec4<f32>,     // L2-1
    sh6: vec4<f32>,     // L20
    sh7: vec4<f32>,     // L21
    sh8: vec4<f32>,     // L22
    ibl_intensity: vec4<f32>,  // x=diffuse_intensity, y=specular_intensity, z=max_spec_mip, w=enabled(0/1)
}

@group(4) @binding(0) var brdf_lut: texture_2d<f32>;
@group(4) @binding(1) var ibl_sampler: sampler;
@group(4) @binding(2) var<uniform> ibl: IblParams;
@group(4) @binding(3) var env_cubemap: texture_cube<f32>;
@group(4) @binding(4) var cubemap_sampler: sampler;

// Evaluate SH L2 irradiance from 9 coefficients
fn eval_sh_irradiance(n: vec3<f32>) -> vec3<f32> {
    // SH basis functions (L2) — convoluted with cosine kernel
    let c1 = 0.429043;
    let c2 = 0.511664;
    let c3 = 0.743125;
    let c4 = 0.886227;
    let c5 = 0.247708;

    return max(vec3<f32>(0.0),
        c4 * ibl.sh0.xyz
        + 2.0 * c2 * (ibl.sh1.xyz * n.y + ibl.sh2.xyz * n.z + ibl.sh3.xyz * n.x)
        + 2.0 * c1 * (ibl.sh4.xyz * n.x * n.y + ibl.sh5.xyz * n.y * n.z + ibl.sh7.xyz * n.x * n.z)
        + c3 * ibl.sh6.xyz * (n.z * n.z - 1.0 / 3.0)
        + c1 * ibl.sh8.xyz * (n.x * n.x - n.y * n.y)
    );
}

// ─── PBR: GGX Normal Distribution Function ───────────────────────────────────
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / max(PI * denom * denom, 1e-7);
}

// ─── PBR: Schlick Geometry Function ──────────────────────────────────────────
fn geometry_schlick(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k + 0.0001);
}

// ─── PBR: Smith Geometry Function ────────────────────────────────────────────
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return geometry_schlick(n_dot_v, roughness) * geometry_schlick(n_dot_l, roughness);
}

// ─── PBR: Fresnel-Schlick Approximation ──────────────────────────────────────
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// Roughness-aware Fresnel for IBL (prevents over-darkening of rough dielectrics at grazing angles)
fn fresnel_schlick_roughness(cos_theta: f32, f0: vec3<f32>, roughness: f32) -> vec3<f32> {
    return f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

// ─── Burley (Disney) Diffuse ─────────────────────────────────────────────────
fn diffuse_burley(n_dot_v: f32, n_dot_l: f32, l_dot_h: f32, roughness: f32) -> f32 {
    let f90 = 0.5 + 2.0 * roughness * l_dot_h * l_dot_h;
    let light_scatter = 1.0 + (f90 - 1.0) * pow(1.0 - n_dot_l, 5.0);
    let view_scatter = 1.0 + (f90 - 1.0) * pow(1.0 - n_dot_v, 5.0);
    return light_scatter * view_scatter / PI;
}

// ─── Height-correlated Smith-GGX Visibility ─────────────────────────────────
fn v_smith_ggx_correlated(n_dot_v: f32, n_dot_l: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let ggx_v = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let ggx_l = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    return 0.5 / (ggx_v + ggx_l + 0.0001);
}

// ─── Charlie sheen distribution (Conty & Kulla) ─────────────────────────────
fn d_charlie(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let inv_alpha = 1.0 / alpha;
    let sin2 = 1.0 - n_dot_h * n_dot_h;
    return (2.0 + inv_alpha) * pow(sin2, inv_alpha * 0.5) / (2.0 * PI);
}

// ─── Disney BRDF for a single directional light ─────────────────────────────
fn disney_brdf_directional(
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    n: vec3<f32>,
    v: vec3<f32>,
    l: vec3<f32>,
    light_color: vec3<f32>,
    f0_dielectric: f32,
    energy_comp: vec3<f32>,
) -> vec3<f32> {
    let h = normalize(l + v);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.001);
    let n_dot_h = max(dot(n, h), 0.0);
    let h_dot_v = max(dot(h, v), 0.0);
    let l_dot_h = max(dot(l, h), 0.0);
    let alpha = max(roughness * roughness, 0.002);

    let f0 = mix(vec3<f32>(f0_dielectric), albedo, metallic);

    // GGX specular with height-correlated Smith visibility + multi-scatter compensation
    let D = distribution_ggx(n_dot_h, roughness);
    let V = v_smith_ggx_correlated(n_dot_v, n_dot_l, alpha);
    let F = fresnel_schlick(h_dot_v, f0);
    let specular = D * V * F * energy_comp;

    // Burley diffuse with Fresnel energy conservation
    let fd = diffuse_burley(n_dot_v, n_dot_l, l_dot_h, roughness);
    let kD = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let diffuse = albedo * fd * kD;

    return (diffuse + specular) * light_color * n_dot_l;
}

// ─── Disney BRDF for a single point light ────────────────────────────────────
fn disney_brdf_point(
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    n: vec3<f32>,
    v: vec3<f32>,
    world_pos: vec3<f32>,
    light_pos_range: vec4<f32>,
    light_color_intensity: vec4<f32>,
    f0_dielectric: f32,
    energy_comp: vec3<f32>,
) -> vec3<f32> {
    let light_pos = light_pos_range.xyz;
    let light_range = light_pos_range.w;
    let light_color = light_color_intensity.xyz;
    let light_intensity = light_color_intensity.w;

    let light_vec = light_pos - world_pos;
    let dist = length(light_vec);
    if dist > light_range || dist < 0.001 {
        return vec3<f32>(0.0);
    }
    let l = light_vec / dist;

    // Smooth inverse-square attenuation with range cutoff
    let atten = saturate(1.0 - (dist / light_range));
    let attenuation = atten * atten;

    let radiance = light_color * light_intensity * attenuation;
    return disney_brdf_directional(albedo, metallic, roughness, n, v, l, radiance, f0_dielectric, energy_comp);
}

// ─── Full PBR lighting (sun + point lights + ambient + shadow) ─────────────────
fn calc_pbr_lighting(
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    ao: f32,
    world_pos: vec3<f32>,
    n: vec3<f32>,
    shadow: f32,
    f0_dielectric: f32,
) -> vec3<f32> {
    let v = normalize(uniforms.camera_pos - world_pos);
    let sun_dir = normalize(uniforms.sun_dir_and_count.xyz);
    let point_count = u32(uniforms.sun_dir_and_count.w);
    let sun_color = uniforms.sun_color_and_intensity.xyz * uniforms.sun_color_and_intensity.w;

    // Pre-compute multi-scatter energy compensation (Turquin 2019)
    // Sampled once, applied to all specular contributions (analytical + IBL).
    let n_dot_v = max(dot(n, v), 0.001);
    let f0 = mix(vec3<f32>(f0_dielectric), albedo, metallic);
    let dfg = textureSample(brdf_lut, ibl_sampler, vec2<f32>(n_dot_v, roughness)).rg;
    let energy_comp = 1.0 + f0 * (1.0 / max(dfg.y, 0.001) - 1.0);

    // Directional sun — Disney BRDF (attenuated by shadow)
    var color = disney_brdf_directional(albedo, metallic, roughness, n, v, sun_dir, sun_color, f0_dielectric, energy_comp) * shadow;

    // Point lights — Disney BRDF
    if point_count >= 1u {
        color += disney_brdf_point(albedo, metallic, roughness, n, v, world_pos, uniforms.light0_pos_range, uniforms.light0_color_intensity, f0_dielectric, energy_comp);
    }
    if point_count >= 2u {
        color += disney_brdf_point(albedo, metallic, roughness, n, v, world_pos, uniforms.light1_pos_range, uniforms.light1_color_intensity, f0_dielectric, energy_comp);
    }
    if point_count >= 3u {
        color += disney_brdf_point(albedo, metallic, roughness, n, v, world_pos, uniforms.light2_pos_range, uniforms.light2_color_intensity, f0_dielectric, energy_comp);
    }
    if point_count >= 4u {
        color += disney_brdf_point(albedo, metallic, roughness, n, v, world_pos, uniforms.light3_pos_range, uniforms.light3_color_intensity, f0_dielectric, energy_comp);
    }

    // IBL path: SH diffuse irradiance + BRDF LUT specular
    let ibl_enabled = ibl.ibl_intensity.w;
    if ibl_enabled > 0.5 {
        let diffuse_intensity = ibl.ibl_intensity.x;
        let specular_intensity = ibl.ibl_intensity.y;

        // Diffuse IBL from spherical harmonics
        let irradiance = eval_sh_irradiance(n);
        let kS_ibl = fresnel_schlick_roughness(n_dot_v, f0, roughness);
        let kD_ibl = (vec3<f32>(1.0) - kS_ibl) * (1.0 - metallic);
        let diffuse_ibl = irradiance * albedo * kD_ibl * diffuse_intensity;

        // Specular IBL from prefiltered cubemap (or SH fallback)
        let r = reflect(-v, n);
        let max_spec_mip = ibl.ibl_intensity.z;
        var specular_ibl: vec3<f32>;
        if max_spec_mip > 0.5 {
            // Prefiltered environment cubemap: mip 0 = mirror, higher mips = rougher
            let spec_mip = roughness * max_spec_mip;
            let spec_env = textureSampleLevel(env_cubemap, cubemap_sampler, r, spec_mip).rgb;
            specular_ibl = spec_env * (f0 * dfg.x + dfg.y) * specular_intensity * energy_comp;
        } else {
            // Fallback: SH irradiance at reflection direction (no cubemap loaded)
            let spec_env = eval_sh_irradiance(r) * (1.0 - roughness * 0.5);
            specular_ibl = spec_env * (f0 * dfg.x + dfg.y) * specular_intensity * energy_comp;
        }

        let ambient = (diffuse_ibl + specular_ibl) * ao;
        color += ambient;
    } else {
        // Fallback hemisphere ambient (when IBL not enabled)
        let ambient_color = uniforms.ambient_color_and_intensity.xyz;
        let ambient_intensity = uniforms.ambient_color_and_intensity.w;
        let ground_c = ambient_color * 0.40;
        let amb_blend = n.y * 0.5 + 0.5;
        let ambient = mix(ground_c, ambient_color, amb_blend) * albedo * ao * ambient_intensity;
        color += ambient;
    }

    // Minimal indirect fill
    let indirect = albedo * 0.025;

    // Subtle warm rim light
    let rim = pow(1.0 - n_dot_v, 4.0) * 0.025;
    let rim_color = vec3<f32>(rim * 1.0, rim * 0.9, rim * 0.7);

    return color + indirect + rim_color;
}

// ─── ACES Filmic Tone Mapping ────────────────────────────────────────────────
fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = color * (2.51 * color + vec3<f32>(0.03));
    let b = color * (2.43 * color + vec3<f32>(0.59)) + vec3<f32>(0.14);
    return clamp(a / b, vec3<f32>(0.0), vec3<f32>(1.0));
}

// ─── Tangent-space normal map → world-space normal ───────────────────────────
// Uses MikkTSpace tangent attribute when available; falls back to cotangent frame.
fn perturb_normal_tbn(world_normal: vec3<f32>, world_tangent: vec4<f32>, world_pos: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let ts_normal = textureSample(normal_texture, material_sampler, uv).xyz * 2.0 - 1.0;
    let n = normalize(world_normal);

    // Use vertex tangent if available (w != 0 indicates valid tangent)
    let tangent_valid = abs(world_tangent.w) > 0.5;
    if tangent_valid {
        // MikkTSpace TBN from vertex attribute
        let t = normalize(world_tangent.xyz);
        let b = cross(n, t) * world_tangent.w; // w = handedness (+1 or -1)
        return normalize(t * ts_normal.x + b * ts_normal.y + n * ts_normal.z);
    }

    // Fallback: cotangent frame from screen-space derivatives
    let dp1 = dpdx(world_pos);
    let dp2 = dpdy(world_pos);
    let duv1 = dpdx(uv);
    let duv2 = dpdy(uv);

    let dp2perp = cross(dp2, n);
    let dp1perp = cross(n, dp1);

    let det_check = abs(dot(dp1, dp2perp) * duv1.x - dot(dp2, dp1perp) * duv1.y);
    if det_check < 0.000001 {
        return n;
    }
    let det_inv = 1.0 / (dot(dp1, dp2perp) * duv1.x - dot(dp2, dp1perp) * duv1.y + 0.0001);

    let t = (dp2perp * duv1.x + dp1perp * duv2.x) * det_inv;
    let b = (dp2perp * duv1.y + dp1perp * duv2.y) * det_inv;

    return normalize(t * ts_normal.x + b * ts_normal.y + n * ts_normal.z);
}

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_position = model_matrix * vec4<f32>(vertex.position, 1.0);
    // Compute inverse-transpose for correct normals under non-uniform scale.
    // adjugate(M) = det(M) * inverse(M)^T — since we normalize, det cancels out.
    let m3 = mat3x3<f32>(model_matrix[0].xyz, model_matrix[1].xyz, model_matrix[2].xyz);
    let it_col0 = cross(m3[1], m3[2]);
    let it_col1 = cross(m3[2], m3[0]);
    let it_col2 = cross(m3[0], m3[1]);
    let world_normal = mat3x3<f32>(it_col0, it_col1, it_col2) * vertex.normal;

    var output: VertexOutput;
    // Camera-relative transform: subtract camera_pos to avoid f32 jitter far from origin
    let rel_pos = world_position.xyz - uniforms.camera_pos;
    output.clip_position = uniforms.view_proj * vec4<f32>(rel_pos, 1.0);
    output.world_position = world_position.xyz;
    output.world_normal = normalize(world_normal);
    // Transform tangent direction by model matrix (not inverse-transpose — tangents transform normally)
    let world_tangent_dir = normalize((m3 * vertex.tangent.xyz));
    output.world_tangent = vec4<f32>(world_tangent_dir, vertex.tangent.w);
    // Multiply vertex color by instance tint (white tint = pass-through vertex colors)
    output.color = vertex.vertex_color * instance.color;
    output.uv = vertex.uv;
    return output;
}

// ─── Legacy point light (Lambertian, for non-textured path) ──────────────────
fn calc_point_light(light_pos_range: vec4<f32>, light_color_intensity: vec4<f32>, world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let light_pos = light_pos_range.xyz;
    let light_range = light_pos_range.w;
    let light_color = light_color_intensity.xyz;
    let light_intensity = light_color_intensity.w;

    let light_vec = light_pos - world_pos;
    let dist = length(light_vec);
    if dist > light_range || dist < 0.001 {
        return vec3<f32>(0.0);
    }
    let light_dir = light_vec / dist;
    let ndotl = max(dot(normal, light_dir), 0.0);
    let atten = saturate(1.0 - (dist / light_range)) * saturate(1.0 - (dist / light_range));
    return light_color * light_intensity * ndotl * atten;
}

// ─── Legacy scene lighting (Lambertian, for non-textured path) ───────────────
fn calc_lighting(world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let sun_dir = normalize(uniforms.sun_dir_and_count.xyz);
    let point_count = u32(uniforms.sun_dir_and_count.w);
    let sun_color = uniforms.sun_color_and_intensity.xyz;
    let sun_intensity = uniforms.sun_color_and_intensity.w;
    let ambient_color = uniforms.ambient_color_and_intensity.xyz;
    let ambient_intensity = uniforms.ambient_color_and_intensity.w;

    var lighting = ambient_color * ambient_intensity;
    let sun_ndotl = max(dot(normal, sun_dir), 0.0);
    lighting += sun_color * sun_intensity * sun_ndotl;

    if point_count >= 1u {
        lighting += calc_point_light(uniforms.light0_pos_range, uniforms.light0_color_intensity, world_pos, normal);
    }
    if point_count >= 2u {
        lighting += calc_point_light(uniforms.light1_pos_range, uniforms.light1_color_intensity, world_pos, normal);
    }
    if point_count >= 3u {
        lighting += calc_point_light(uniforms.light2_pos_range, uniforms.light2_color_intensity, world_pos, normal);
    }
    if point_count >= 4u {
        lighting += calc_point_light(uniforms.light3_pos_range, uniforms.light3_color_intensity, world_pos, normal);
    }

    return lighting;
}

// ─── Fragment: Non-textured (vertex colors + Lambertian, group(0) only) ──────
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if uniforms.shading_mode == 1u {
        return in.color;
    }

    if uniforms.shading_mode == 2u {
        let dn = fwidth(in.world_normal);
        let edge = length(dn);
        let edge_factor = smoothstep(0.1, 0.5, edge);
        let fill_color = vec4<f32>(0.15, 0.15, 0.18, 0.6);
        let edge_color = vec4<f32>(0.9, 0.95, 1.0, 1.0);
        return mix(fill_color, edge_color, edge_factor);
    }

    let lighting = calc_lighting(in.world_position, in.world_normal);
    let lit_color = in.color.rgb * lighting;
    return vec4<f32>(lit_color, in.color.a);
}

// ─── Fragment: Full PBR textured (group 0 + 1 + 2 + 3 + 4) ─────────────────
@fragment
fn fs_textured(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    // ── Shading mode overrides ───────────────────────────────────────────
    if uniforms.shading_mode == 1u {
        let tex_color = textureSample(albedo_texture, material_sampler, in.uv);
        return tex_color * in.color;
    }
    if uniforms.shading_mode == 2u {
        let dn = fwidth(in.world_normal);
        let edge = length(dn);
        let edge_factor = smoothstep(0.1, 0.5, edge);
        let fill_color = vec4<f32>(0.15, 0.15, 0.18, 0.6);
        let edge_color = vec4<f32>(0.9, 0.95, 1.0, 1.0);
        return mix(fill_color, edge_color, edge_factor);
    }

    // ── Unpack material parameters ───────────────────────────────────────
    let metallic_factor = material.emissive_and_metallic.w;
    let roughness_factor = material.pbr_params.x;
    let emissive_strength = material.pbr_params.y;
    let occlusion_strength = material.pbr_params.z;
    let alpha_cutoff = material.pbr_params.w;
    let alpha_mode = u32(material.extra_params.w);

    // ── Sample textures ──────────────────────────────────────────────────
    let albedo_sample = textureSample(albedo_texture, material_sampler, in.uv);
    let albedo = albedo_sample.rgb * material.base_color_factor.rgb * in.color.rgb;
    var alpha = albedo_sample.a * material.base_color_factor.a * in.color.a;

    // Alpha test (Mask mode)
    if alpha_mode == 1u && alpha < alpha_cutoff {
        discard;
    }

    // ORM texture: R=occlusion, G=roughness, B=metallic
    let orm = textureSample(orm_texture, material_sampler, in.uv).rgb;
    let ao = mix(1.0, orm.r, occlusion_strength);
    let roughness = clamp(orm.g * roughness_factor, 0.04, 1.0);
    let metallic = clamp(orm.b * metallic_factor, 0.0, 1.0);

    // ── Normal mapping ───────────────────────────────────────────────────
    var n = perturb_normal_tbn(in.world_normal, in.world_tangent, in.world_position, in.uv);

    // ── Double-sided: flip normal for back faces ─────────────────────
    if !is_front {
        n = -n;
    }

    // ── IOR-based dielectric F0 ──────────────────────────────────────
    let ior = material.extra_params.x;
    let f0_ior = pow((ior - 1.0) / (ior + 1.0), 2.0);

    // ── Shadow ───────────────────────────────────────────────────────
    let shadow = sample_shadow(in.world_position, n);

    // ── PBR lighting ─────────────────────────────────────────────────
    var color = calc_pbr_lighting(albedo, metallic, roughness, ao, in.world_position, n, shadow, f0_ior);

    // ── Emissive ─────────────────────────────────────────────────────────
    let emissive_tex = textureSample(emissive_texture, material_sampler, in.uv).rgb;
    let emissive = emissive_tex * material.emissive_and_metallic.xyz * emissive_strength;
    color += emissive;

    // ── Clearcoat (Disney: separate GGX lobe with IOR 1.5, Kelemen visibility) ──
    let cc_factor = material.extra_params.y;
    if cc_factor > 0.001 {
        let cc_roughness = max(material.extra_params.z, 0.04);
        let cc_alpha = cc_roughness * cc_roughness;
        let v = normalize(uniforms.camera_pos - in.world_position);
        let sun_dir = normalize(uniforms.sun_dir_and_count.xyz);
        let sun_rad = uniforms.sun_color_and_intensity.xyz * uniforms.sun_color_and_intensity.w;
        let h = normalize(sun_dir + v);
        let cc_ndoth = max(dot(n, h), 0.0);
        let cc_ndotl = max(dot(n, sun_dir), 0.0);
        let cc_ldoth = max(dot(sun_dir, h), 0.0);
        let cc_D = distribution_ggx(cc_ndoth, cc_roughness);
        // Kelemen visibility (cheap, good for clearcoat)
        let cc_V = 0.25 / (cc_ldoth * cc_ldoth + 0.0001);
        // Fixed IOR 1.5 Fresnel
        let cc_f0 = f0_ior;
        let cc_F_scalar = cc_f0 + (1.0 - cc_f0) * pow(1.0 - cc_ldoth, 5.0);
        let cc_spec = cc_D * cc_V * cc_F_scalar;
        color += vec3<f32>(cc_spec) * sun_rad * cc_ndotl * cc_factor * shadow;
    }

    // ── Sheen (velvet/fabric edge glow via Charlie distribution) ──────────
    // Re-use extra_params: we can pack sheen into the unused w bits or a dedicated uniform
    // For now, apply subtle sheen on non-metallic surfaces when clearcoat is zero
    if metallic < 0.5 && cc_factor < 0.001 {
        let v_sh = normalize(uniforms.camera_pos - in.world_position);
        let sun_dir_sh = normalize(uniforms.sun_dir_and_count.xyz);
        let h_sh = normalize(sun_dir_sh + v_sh);
        let n_dot_h_sh = max(dot(n, h_sh), 0.0);
        let n_dot_l_sh = max(dot(n, sun_dir_sh), 0.0);
        let l_dot_h_sh = max(dot(sun_dir_sh, h_sh), 0.0);
        let sheen_d = d_charlie(n_dot_h_sh, max(roughness, 0.3));
        let sheen_f = fresnel_schlick(l_dot_h_sh, vec3<f32>(0.04));
        let sun_rad_sh = uniforms.sun_color_and_intensity.xyz * uniforms.sun_color_and_intensity.w;
        color += sheen_d * sheen_f * sun_rad_sh * n_dot_l_sh * 0.15 * shadow;
    }

    // ── HDR output vs legacy LDR path ────────────────────────────────────
    // exposure_params.y > 0.5 signals that the post-process chain handles
    // exposure and tonemapping; shader outputs linear HDR directly.
    // Apply exposure in both paths — EV 0.0 = no change (exp2(0) = 1.0)
    let exposure_ev = uniforms.exposure_params.x;
    color *= exp2(exposure_ev);

    let hdr_mode = uniforms.exposure_params.y;
    if hdr_mode < 0.5 {
        // Legacy path: inline ACES tonemap (fallback when no post chain)
        color = aces_tonemap(color);
    }
    // else: output exposed HDR — post chain handles tonemapping

    // Opaque mode forces alpha=1.0
    if alpha_mode == 0u {
        alpha = 1.0;
    }

    return vec4<f32>(color, alpha);
}
