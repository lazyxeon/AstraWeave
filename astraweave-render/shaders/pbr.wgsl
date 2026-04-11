struct VSIn {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(12) tangent:  vec4<f32>,
    @location(13) uv:       vec2<f32>,
  @location(2) m0: vec4<f32>,
  @location(3) m1: vec4<f32>,
  @location(4) m2: vec4<f32>,
  @location(5) m3: vec4<f32>,
  @location(6) n0: vec3<f32>,
  @location(7) n1: vec3<f32>,
  @location(8) n2: vec3<f32>,
  @location(9) color: vec4<f32>,
};

struct VSOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) world_pos: vec3<f32>,
  @location(1) normal: vec3<f32>,
    @location(3) tbn0: vec3<f32>,
    @location(4) tbn1: vec3<f32>,
    @location(5) tbn2: vec3<f32>,
    @location(6) uv: vec2<f32>,
  @location(2) color: vec4<f32>,
  @location(7) clip_pos: vec4<f32>,
};

struct Camera {
  view_proj: mat4x4<f32>,
  light_dir: vec3<f32>,
  sun_intensity: f32,
  camera_pos: vec3<f32>,
  _pad2: f32,
};

@group(0) @binding(0) var<uniform> uCamera: Camera;

struct MaterialUbo {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _pad: vec2<f32>,
};

@group(1) @binding(0) var<uniform> uMaterial: MaterialUbo;

struct MainLightUbo {
    view_proj0: mat4x4<f32>,
    view_proj1: mat4x4<f32>,
    view_proj2: mat4x4<f32>,
    view_proj3: mat4x4<f32>,
    splits: vec4<f32>,
    extras: vec2<f32>, // x: pcf_radius_px, y: depth_bias; z: slope_scale in skinned path extras.x reused; keep 2 vec2s for alignment
};
@group(2) @binding(0) var<uniform> uLight: MainLightUbo;
@group(2) @binding(1) var shadow_tex: texture_depth_2d_array;
@group(2) @binding(2) var shadow_sampler: sampler_comparison;

@group(3) @binding(0) var albedo_tex: texture_2d<f32>;
@group(3) @binding(1) var albedo_samp: sampler;
@group(3) @binding(2) var mr_tex: texture_2d<f32>;      // R: metallic, G: roughness
@group(3) @binding(3) var mr_samp: sampler;
@group(3) @binding(4) var normal_tex: texture_2d<f32>;  // tangent-space normal in RGB
@group(3) @binding(5) var normal_samp: sampler;
@group(3) @binding(6) var height_tex: texture_2d<f32>;  // heightmap (R channel, 1=raised 0=depressed)
@group(3) @binding(7) var height_samp: sampler;

// Scene environment (fog, ambient, sun) — matches SceneEnvironmentUBO (96 bytes).
// Layout uses vec4 packing to avoid vec3 alignment mismatches between Rust and WGSL.
struct SceneEnv {
    fog_color_density: vec4<f32>,   // .xyz = fog_color, .w = fog_density     (offset 0)
    fog_range_pad: vec4<f32>,       // .x = fog_start, .y = fog_end, .z = wetness, .w = snow_amount (offset 16)
    ambient_color_intensity: vec4<f32>, // .xyz = ambient_color, .w = intensity (offset 32)
    tint_color_alpha: vec4<f32>,    // .xyz = tint_color, .w = tint_alpha      (offset 48)
    blend_pad: vec4<f32>,           // .x = blend_factor, .yzw = padding       (offset 64)
    sun_color_intensity: vec4<f32>, // .xyz = sun_color, .w = sun_intensity    (offset 80)
};
@group(4) @binding(0) var<uniform> uScene: SceneEnv;

@vertex
fn vs(input: VSIn) -> VSOut {
  let model = mat4x4<f32>(input.m0, input.m1, input.m2, input.m3);
  let world = model * vec4<f32>(input.position, 1.0);
  var out: VSOut;
  out.pos = uCamera.view_proj * world;
    // normal matrix simplified (assuming uniform scale); for accuracy pass and use 3x3
    let Nw = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    let Tw = normalize((model * vec4<f32>(input.tangent.xyz, 0.0)).xyz);
    let Bw = normalize(cross(Nw, Tw)) * input.tangent.w;
    out.normal = Nw;
  out.world_pos = world.xyz;
    out.tbn0 = Tw; out.tbn1 = Bw; out.tbn2 = Nw;
    out.uv = input.uv;
    out.color = input.color;
    out.clip_pos = out.pos;
    return out;
}

// Simple Cook-Torrance PBR with single directional light, no IBL
fn fresnel_schlick(cos_theta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (vec3<f32>(1.0,1.0,1.0) - F0) * pow(1.0 - cos_theta, 5.0);
}

fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;
    let denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (3.14159 * denom * denom + 1e-5);
}

fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx1 = NdotV / (NdotV * (1.0 - k) + k + 1e-5);
    let ggx2 = NdotL / (NdotL * (1.0 - k) + k + 1e-5);
    return ggx1 * ggx2;
}

// ============================================================================
// PARALLAX OCCLUSION MAPPING
// ============================================================================

const POM_MIN_STEPS: f32 = 8.0;
const POM_MAX_STEPS: f32 = 32.0;
const POM_BINARY_REFINEMENT_STEPS: u32 = 5u;

/// Compute parallax-displaced UV coordinates.
/// height_scale from uMaterial._pad.y: 0.0 = disabled, typical 0.02–0.08.
fn pom_offset_uv(
    uv: vec2<f32>,
    view_ts: vec3<f32>,
    height_scale: f32
) -> vec2<f32> {
    if (height_scale <= 0.0) {
        return uv;
    }

    let n_dot_v = max(abs(view_ts.z), 0.001);
    let num_steps = mix(POM_MAX_STEPS, POM_MIN_STEPS, n_dot_v);
    let step_size = 1.0 / num_steps;
    let max_offset = view_ts.xy / view_ts.z * height_scale;
    let delta_uv = max_offset * step_size;

    var current_uv = uv;
    var current_layer_depth: f32 = 0.0;
    var current_height = textureSampleLevel(height_tex, height_samp, current_uv, 0.0).r;

    var i: u32 = 0u;
    let max_i = u32(num_steps);
    while (i < max_i && current_layer_depth < current_height) {
        current_uv -= delta_uv;
        current_layer_depth += step_size;
        current_height = textureSampleLevel(height_tex, height_samp, current_uv, 0.0).r;
        i++;
    }

    // Binary refinement — converge on exact intersection
    var prev_uv = current_uv + delta_uv;
    var prev_depth = current_layer_depth - step_size;

    for (var j: u32 = 0u; j < POM_BINARY_REFINEMENT_STEPS; j++) {
        let mid_uv = (current_uv + prev_uv) * 0.5;
        let mid_depth = (current_layer_depth + prev_depth) * 0.5;
        let mid_height = textureSampleLevel(height_tex, height_samp, mid_uv, 0.0).r;

        if (mid_depth < mid_height) {
            prev_uv = mid_uv;
            prev_depth = mid_depth;
        } else {
            current_uv = mid_uv;
            current_layer_depth = mid_depth;
        }
    }

    return current_uv;
}

@fragment
fn fs(input: VSOut) -> @location(0) vec4<f32> {
    let V = normalize(uCamera.camera_pos - input.world_pos);
    let L = normalize(-uCamera.light_dir);
    let H = normalize(V + L);
    // Base normal from geometry
    var N = normalize(input.normal);

    // Parallax Occlusion Mapping: compute displaced UV before all texture samples.
    // height_scale stored in uMaterial._pad.y (0.0 = POM disabled).
    let T = input.tbn0; let B = input.tbn1; let NN = input.tbn2;
    let tbn_mat = mat3x3<f32>(T, B, NN);
    let view_ts = normalize(transpose(tbn_mat) * V);
    let height_scale = uMaterial._pad.y;
    let tex_uv = pom_offset_uv(input.uv, view_ts, height_scale);

    // Normal map sample using displaced UVs and TBN
    let nrm_rgb = textureSample(normal_tex, normal_samp, tex_uv).rgb;
    let nrm_ts = normalize(nrm_rgb * 2.0 - vec3<f32>(1.0,1.0,1.0));
    N = normalize(T * nrm_ts.x + B * nrm_ts.y + NN * nrm_ts.z);
    let NdotL = max(dot(N, L), 0.0);

    var base_color = (uMaterial.base_color.rgb * input.color.rgb);
    let tex = textureSample(albedo_tex, albedo_samp, tex_uv);
    base_color = base_color * tex.rgb;
    var metallic = clamp(uMaterial.metallic, 0.0, 1.0);
    var roughness = clamp(uMaterial.roughness, 0.04, 1.0);
    let mr = textureSample(mr_tex, mr_samp, tex_uv);
    metallic = clamp(max(metallic, mr.r), 0.0, 1.0);
    roughness = clamp(min(roughness, max(mr.g, 0.04)), 0.04, 1.0);

    // ── Wet-surface PBR modulation ──────────────────────────────────────
    // Only upward-facing surfaces accumulate water; wetness from scene env.
    let wetness_global = uScene.fog_range_pad.z;
    let up_facing = max(N.y, 0.0);
    let wet = wetness_global * up_facing;
    // Water fills micro-cavities → smoother surface (lower roughness).
    roughness = max(roughness * (1.0 - wet * 0.7), 0.04);
    // Wet surfaces absorb more light → darker albedo.
    base_color = base_color * (1.0 - wet * 0.3);

    // ── Snow accumulation PBR blend ─────────────────────────────────────
    // Global snow amount from scene env (.w), modulated by surface orientation.
    let snow_global = uScene.fog_range_pad.w;
    let snow_raw = snow_global * saturate(up_facing + 0.1); // slight bias so near-vertical gets dusting
    let snow_weight = saturate((snow_raw - 0.1) * 5.0);     // threshold 0.1, sharpness 5.0
    // Blend base material toward snow properties.
    let snow_albedo = vec3<f32>(0.95, 0.96, 0.98);          // slightly blue-white
    base_color = mix(base_color, snow_albedo, snow_weight);
    roughness = mix(roughness, 0.8, snow_weight);            // fresh snow roughness
    metallic = mix(metallic, 0.0, snow_weight);              // snow is non-metallic

    let F0_base = mix(vec3<f32>(0.04, 0.04, 0.04), base_color, metallic);
    // Blend toward water F0 (IOR ≈ 1.33 → F0 ≈ 0.02) when wet.
    let F0 = mix(F0_base, vec3<f32>(0.02, 0.02, 0.02), wet);
    let F = fresnel_schlick(max(dot(H, V), 0.0), F0);
    let D = distribution_ggx(N, H, roughness);
    let G = geometry_smith(N, V, L, roughness);

    let numerator = D * G * F;
    let denom = 4.0 * max(dot(N, V), 0.0) * NdotL + 1e-5;
    let specular = numerator / denom;

    let kd = (vec3<f32>(1.0,1.0,1.0) - F) * (1.0 - metallic);
    let diffuse = kd * base_color / 3.14159;

    // Sun color and intensity from SceneEnvironment UBO (set by world panel).
    let radiance = uScene.sun_color_intensity.xyz * uScene.sun_color_intensity.w;
        // Shadow sampling
        // Cascaded shadow mapping (2 cascades)
    let dist = length(input.world_pos);
    var cascade_idx: i32 = 3;
    var lvp: mat4x4<f32>;
    
    if (dist < uLight.splits.x) {
        cascade_idx = 0;
        lvp = uLight.view_proj0;
    } else if (dist < uLight.splits.y) {
        cascade_idx = 1;
        lvp = uLight.view_proj1;
    } else if (dist < uLight.splits.z) {
        cascade_idx = 2;
        lvp = uLight.view_proj2;
    } else {
        cascade_idx = 3;
        lvp = uLight.view_proj3;
    }

    let lp = lvp * vec4<f32>(input.world_pos, 1.0);
    let ndc_shadow = lp.xyz / lp.w;
    let uv = ndc_shadow.xy * 0.5 + vec2<f32>(0.5, 0.5);
    let depth = ndc_shadow.z;
    let slope = max(0.0, 1.0 - dot(N, L));
    let base_bias = uLight.extras.y;
    let bias = max(base_bias /* + slope_scale * slope */ , 0.00001);
        var shadow: f32 = 1.0;
        if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0) {
            let layer = cascade_idx;
            // PCF 3x3 (scaled by pcf radius in texels from extras.x)
            let dims = vec2<f32>(textureDimensions(shadow_tex).xy);
            let texel = 1.0 / dims;
            let r = max(0.0, uLight.extras.x);
            var sum = 0.0;
            for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
                for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
                    let o = vec2<f32>(f32(dx), f32(dy)) * texel * r;
                    sum = sum + textureSampleCompare(shadow_tex, shadow_sampler, uv + o, layer, depth - bias);
                }
            }
            shadow = sum / 9.0;
        }
        // Optional debug visualization: use uMaterial._pad.x > 0.5 to tint by cascade
        if (uMaterial._pad.x > 0.5) {
            var tint: vec3<f32>;
            if (cascade_idx == 0) { tint = vec3<f32>(1.0, 0.0, 0.0); }       // Red
            else if (cascade_idx == 1) { tint = vec3<f32>(0.0, 1.0, 0.0); }  // Green
            else if (cascade_idx == 2) { tint = vec3<f32>(0.0, 0.0, 1.0); }  // Blue
            else { tint = vec3<f32>(1.0, 1.0, 0.0); }                        // Yellow
            base_color = mix(base_color, tint, 0.35);
        }
    // Ambient lighting from scene environment.
    let ambient_term = uScene.ambient_color_intensity.xyz * uScene.ambient_color_intensity.w;
    var lit_color = (diffuse + specular) * radiance * NdotL * shadow + base_color * ambient_term;
    
    // Clustered point lights accumulation
    let ndc = input.clip_pos.xy / input.clip_pos.w;
    let screen_uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let view_z = input.clip_pos.w;

    let clustered_light = calculate_clustered_lighting(
        input.world_pos,
        N,
        uCamera.camera_pos,
        base_color,
        metallic,
        roughness,
        screen_uv,
        view_z
    );
    
    lit_color = lit_color + clustered_light;

    // VXGI indirect lighting (Group 5)
    let vxgi_light = calculate_vxgi_lighting(input.world_pos, N);
    // Combine with AO if available (currently AO is separate post-pass)
    // Multiply by albedo for diffuse reflection
    lit_color = lit_color + (vxgi_light * base_color * 1.0);

    // Distance fog — exponential with start distance threshold.
    // Only applies beyond fog_start to keep near geometry crisp.
    let view_dist = length(input.world_pos - uCamera.camera_pos);
    let effective_dist = max(view_dist - uScene.fog_range_pad.x, 0.0);
    let fog_amount = 1.0 - exp(-uScene.fog_color_density.w * effective_dist);
    lit_color = mix(lit_color, uScene.fog_color_density.xyz, fog_amount);

    return vec4<f32>(lit_color, uMaterial.base_color.a * input.color.a);
}
