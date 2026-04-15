#[cfg(feature = "postfx")]
use crate::post::{WGSL_SSAO, WGSL_SSGI, WGSL_SSR};
use anyhow::Context;
use anyhow::Result;
use glam::Vec4Swizzles;
use glam::{vec3, Mat4};
use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::clustered::{bin_lights_cpu, ClusterDims, CpuLight, WGSL_CLUSTER_BIN};
use crate::depth::Depth;
use crate::types::SkinnedVertex;
use crate::types::{Instance, InstanceRaw, Mesh};
use astraweave_cinematics as awc;
use astraweave_materials::MaterialPackage;

pub(crate) const SHADER_SRC: &str = concat!(
    include_str!("../shaders/constants.wgsl"),
    include_str!("../shaders/brdf_common.wgsl"),
    r#"
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
};

struct Camera {
  view_proj: mat4x4<f32>,
  light_dir: vec3<f32>,
  _pad: f32,
  camera_pos: vec3<f32>,
  _pad2: f32,
};

@group(0) @binding(0) var<uniform> uCamera: Camera;

struct MaterialUbo {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    alpha_cutoff: f32,
    _pad: f32,
};

@group(1) @binding(0) var<uniform> uMaterial: MaterialUbo;

struct MainLightUbo {
    view_proj0: mat4x4<f32>,
    view_proj1: mat4x4<f32>,
    splits: vec2<f32>,
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

// ── Scene Environment (fog, ambient, tint) ─────────
struct SceneEnv {
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_start: f32,
    fog_end: f32,
    _pad0: vec2<f32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    tint_color: vec3<f32>,
    tint_alpha: f32,
    blend_factor: f32,
    _pad1x: f32, _pad1y: f32, _pad1z: f32,
    sun_color: vec3<f32>,
    sun_intensity: f32,
};
@group(4) @binding(0) var<uniform> uScene: SceneEnv;

// ── IBL (Image-Based Lighting) ─────────
// Prefiltered specular cubemap (mip levels encode roughness)
@group(5) @binding(0) var ibl_specular: texture_cube<f32>;
// Irradiance cubemap (diffuse IBL)
@group(5) @binding(1) var ibl_irradiance: texture_cube<f32>;
// BRDF integration LUT (split-sum approximation)
@group(5) @binding(2) var ibl_brdf_lut: texture_2d<f32>;
// IBL sampler
@group(5) @binding(3) var ibl_sampler: sampler;

struct IblParams {
    ibl_intensity: f32,
    max_spec_lod: f32,
    _pad: vec2<f32>,
};
@group(5) @binding(4) var<uniform> uIbl: IblParams;

// Screen-space GI (from SSGI/Lumen pipeline). Fallback is 1x1 black.
@group(5) @binding(5) var gi_tex: texture_2d<f32>;
@group(5) @binding(6) var gi_samp: sampler;
// Cloud shadow transmittance map (1.0 = lit, 0.0 = shadowed)
@group(5) @binding(7) var cloud_shadow_tex: texture_2d<f32>;
@group(5) @binding(8) var cloud_shadow_samp: sampler;

fn sample_cloud_shadow(world_pos: vec3<f32>) -> f32 {
    let shadow_extent = 1024.0;
    let uv = (world_pos.xz - uCamera.camera_pos.xz) / (2.0 * shadow_extent) + vec2<f32>(0.5, 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return 1.0;
    }
    return textureSampleLevel(cloud_shadow_tex, cloud_shadow_samp, uv, 0.0).r;
}

// Distance-based fog (linear + exponential blend)
fn apply_scene_fog(color: vec3<f32>, dist: f32) -> vec3<f32> {
    // Linear fog component
    let linear_fog = clamp((dist - uScene.fog_start) / max(uScene.fog_end - uScene.fog_start, 0.001), 0.0, 1.0);
    // Exponential fog component (denser -> more fog)
    let exp_fog = 1.0 - exp(-uScene.fog_density * dist);
    // Combine: use linear as primary, exponential adds density.
    // Cap at 0.92 so distant terrain retains some color — prevents
    // the "white void" effect at the horizon.
    let fog_factor = clamp(max(linear_fog, exp_fog), 0.0, 0.92);
    return mix(color, uScene.fog_color, fog_factor);
}

// Apply screen tint overlay
fn apply_scene_tint(color: vec3<f32>) -> vec3<f32> {
    return mix(color, uScene.tint_color, uScene.tint_alpha);
}



@vertex
fn vs(input: VSIn) -> VSOut {
  let model = mat4x4<f32>(input.m0, input.m1, input.m2, input.m3);
  let normal_mat = mat3x3<f32>(input.n0, input.n1, input.n2);
  let world = model * vec4<f32>(input.position, 1.0);
  var out: VSOut;
  out.pos = uCamera.view_proj * world;
    // Use inverse-transpose normal matrix for correct non-uniform scale handling
    let Nw = normalize(normal_mat * input.normal);
    let Tw = normalize(normal_mat * input.tangent.xyz);
    let Bw = normalize(cross(Nw, Tw)) * input.tangent.w;
    out.normal = Nw;
  out.world_pos = world.xyz;
    out.tbn0 = Tw; out.tbn1 = Bw; out.tbn2 = Nw;
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

// IBL uses fresnel_schlick_roughness from brdf_common.wgsl
// All samples use textureSampleLevel (explicit LOD) so this function
// is safe inside non-uniform control flow (e.g., LOD branches).
fn compute_ibl(
    N: vec3<f32>,
    V: vec3<f32>,
    base_color: vec3<f32>,
    metallic: f32,
    roughness: f32,
    F0: vec3<f32>,
) -> vec3<f32> {
    let NdotV = max(dot(N, V), 0.0);
    let F = fresnel_schlick_roughness(NdotV, F0, roughness);

    // Diffuse IBL: irradiance cubemap sampled by normal (pre-convolved, mip 0)
    let kd = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let irradiance = textureSampleLevel(ibl_irradiance, ibl_sampler, N, 0.0).rgb;
    let diffuse_ibl = kd * base_color * irradiance;

    // Specular IBL: prefiltered environment map + BRDF LUT
    let R = reflect(-V, N);
    let mip = roughness * uIbl.max_spec_lod;
    let prefiltered = textureSampleLevel(ibl_specular, ibl_sampler, R, mip).rgb;
    let brdf = textureSampleLevel(ibl_brdf_lut, ibl_sampler, vec2<f32>(NdotV, roughness), 0.0).rg;
    let specular_ibl = prefiltered * (F * brdf.x + brdf.y);

    return (diffuse_ibl + specular_ibl) * uIbl.ibl_intensity;
}

@fragment
fn fs(input: VSOut) -> @location(0) vec4<f32> {
    let V = normalize(uCamera.camera_pos - input.world_pos);
    let L = normalize(-uCamera.light_dir);
    // Base normal from geometry
    var N = normalize(input.normal);
    // Normal map sample using real UVs and TBN
    let nrm_rgb = textureSample(normal_tex, normal_samp, input.uv).rgb;
    let nrm_ts = normalize(nrm_rgb * 2.0 - vec3<f32>(1.0,1.0,1.0));
    let T = input.tbn0; let B = input.tbn1; let NN = input.tbn2;
    N = normalize(T * nrm_ts.x + B * nrm_ts.y + NN * nrm_ts.z);

    var base_color = (uMaterial.base_color.rgb * input.color.rgb);
    let tex = textureSample(albedo_tex, albedo_samp, input.uv);
    // Alpha cutoff: discard fully transparent fragments early to avoid
    // wasted shading and incorrect depth writes (vegetation canopies).
    if (tex.a < uMaterial.alpha_cutoff) { discard; }
    base_color = base_color * tex.rgb;
    var metallic = clamp(uMaterial.metallic, 0.0, 1.0);
    var roughness = clamp(uMaterial.roughness, 0.04, 1.0);
    let mr = textureSample(mr_tex, mr_samp, input.uv);
    metallic = clamp(max(metallic, mr.r), 0.0, 1.0);
    roughness = clamp(min(roughness, max(mr.g, 0.04)), 0.04, 1.0);

    let F0 = mix(vec3<f32>(0.04, 0.04, 0.04), base_color, metallic);

    // Material LOD: simplify shading for sub-pixel or distant fragments.
    let mat_lod = compute_material_lod(input.world_pos);

    // Unified BRDF: Cook-Torrance specular + Burley diffuse (from brdf_common.wgsl)
    let brdf_result = evaluate_brdf_lod(N, V, L, base_color, metallic, roughness, F0, mat_lod);

    let radiance = uScene.sun_color * uScene.sun_intensity; // from SceneEnv UBO

    // Distance from camera — used for shadows, fog, and LOD.
    // Computed once to avoid redundant length() per fragment.
    let frag_dist = length(input.world_pos - uCamera.camera_pos);

    // Shadow sampling — skip ALL shadow work when disabled (extras.x < 0).
    // extras.x is uniform (same for every fragment), so the GPU skips the
    // entire block for all warps — saves 9 PCF comparison samples + ALU.
    var shadow: f32 = 1.0;
    if (uLight.extras.x >= 0.0) {
        let shadow_far = uLight.splits.y;
        let use_c0 = frag_dist < uLight.splits.x;
        var lvp: mat4x4<f32>;
        if (use_c0) { lvp = uLight.view_proj0; } else { lvp = uLight.view_proj1; }
        let lp = lvp * vec4<f32>(input.world_pos, 1.0);
        let ndc_shadow = lp.xyz / lp.w;
        let uv = ndc_shadow.xy * 0.5 + vec2<f32>(0.5, 0.5);
        let depth = ndc_shadow.z;
        let base_bias = uLight.extras.y;
        let bias = max(base_bias, 0.00001);

        if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0 && frag_dist < shadow_far) {
            var layer: i32;
            if (use_c0) { layer = 0; } else { layer = 1; }
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

            // Fade shadow to 1.0 at cascade boundary edges to eliminate
            // the hard square cutoff. Fade in the outer 20% of shadow range.
            let fade_start = shadow_far * 0.8;
            if (frag_dist > fade_start) {
                let fade = (frag_dist - fade_start) / (shadow_far - fade_start);
                shadow = mix(shadow, 1.0, clamp(fade, 0.0, 1.0));
            }

            // Also fade at UV edges to soften the ortho projection boundary
            let edge_fade_x = min(uv.x, 1.0 - uv.x) * 10.0;
            let edge_fade_y = min(uv.y, 1.0 - uv.y) * 10.0;
            let edge_fade = clamp(min(edge_fade_x, edge_fade_y), 0.0, 1.0);
            shadow = mix(1.0, shadow, edge_fade);
        }
    }
    // Direct lighting (evaluate_brdf already includes NdotL)
    let cloud_shadow = sample_cloud_shadow(input.world_pos);
    var lit_color = brdf_result * radiance * shadow * cloud_shadow;

    // IBL indirect lighting — skip for distant (LOD 2) fragments.
    // At LOD 2, fragments are sub-pixel or far away: 3 cubemap samples
    // are wasted on invisible detail. Use a cheap ambient approximation.
    if (mat_lod < 2u) {
        let ibl_color = compute_ibl(N, V, base_color, metallic, roughness, F0);
        lit_color = lit_color + ibl_color;
    } else {
        // Approximate IBL as diffuse-only irradiance (1 cubemap sample).
        let kd = (1.0 - metallic);
        let approx_irr = textureSampleLevel(ibl_irradiance, ibl_sampler, N, 0.0).rgb;
        lit_color = lit_color + base_color * kd * approx_irr * uIbl.ibl_intensity;
    }

    // Screen-space GI: sample indirect diffuse from SSGI/Lumen pipeline.
    // When no GI pass is active, gi_tex is a 1x1 black texture → adds zero.
    let gi_dims = vec2<f32>(textureDimensions(gi_tex));
    let gi_uv = input.pos.xy / gi_dims;
    let gi_indirect = textureSampleLevel(gi_tex, gi_samp, gi_uv, 0.0).rgb;
    lit_color += gi_indirect * base_color * (1.0 - metallic);

    // Scene ambient as fallback floor (provides fill when IBL is low)
    let ambient = uScene.ambient_color * uScene.ambient_intensity * 0.35;
    lit_color = lit_color + base_color * ambient;
        // Clustered point lights accumulation (Lambert + simple attenuation)
    // Clustered lighting disabled for this example build; use lit_color directly

    // Apply distance-based fog from biome scene environment.
    lit_color = apply_scene_fog(lit_color, frag_dist);

    // Apply screen tint overlay (peaks during biome transitions)
    lit_color = apply_scene_tint(lit_color);

    return vec4<f32>(lit_color, uMaterial.base_color.a * input.color.a);
}
"#
);

#[cfg(not(feature = "postfx"))]
pub(crate) const POST_SHADER: &str = r#"
struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    // Flip UV.y: wgpu NDC Y+ is screen-top, texture V=0 is top.
    out.uv = vec2<f32>((pos[vid].x + 1.0) * 0.5, (1.0 - pos[vid].y) * 0.5);
    return out;
}

@group(0) @binding(0) var hdr_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

// Scene environment for screen-space tint overlay
struct PostSceneEnv {
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_start: f32,
    fog_end: f32,
    _pad0: vec2<f32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    tint_color: vec3<f32>,
    tint_alpha: f32,
    blend_factor: f32,
    _pad1: vec3<f32>,
};
@group(1) @binding(0) var<uniform> uPostScene: PostSceneEnv;

fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x*(a*x+b))/(x*(c*x+d)+e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let hdr = textureSampleLevel(hdr_tex, samp, in.uv, 0.0);
    let exposure = 1.35;
    var color = aces_tonemap(vec3<f32>(hdr.r, hdr.g, hdr.b) * exposure);
    // Screen-space tint overlay from biome transitions
    color = mix(color, uPostScene.tint_color, uPostScene.tint_alpha);
    return vec4<f32>(color, 1.0);
}
"#;

#[cfg(feature = "postfx")]
pub(crate) const POST_SHADER_FX: &str = r#"
struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    // Flip UV.y: wgpu NDC Y+ is screen-top, texture V=0 is top.
    out.uv = vec2<f32>((pos[vid].x + 1.0) * 0.5, (1.0 - pos[vid].y) * 0.5);
    return out;
}

@group(0) @binding(0) var hdr_tex: texture_2d<f32>;
@group(0) @binding(1) var ao_tex: texture_2d<f32>;
@group(0) @binding(2) var gi_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

// Scene environment for screen-space tint overlay
struct PostSceneEnv {
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_start: f32,
    fog_end: f32,
    _pad0: vec2<f32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    tint_color: vec3<f32>,
    tint_alpha: f32,
    blend_factor: f32,
    _pad1: vec3<f32>,
};
@group(1) @binding(0) var<uniform> uPostScene: PostSceneEnv;

fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51; let b = 0.03; let c = 2.43; let d = 0.59; let e = 0.14;
    return clamp((x*(a*x+b))/(x*(c*x+d)+e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let hdr = textureSampleLevel(hdr_tex, samp, in.uv, 0.0).rgb;
    let ao = textureSampleLevel(ao_tex, samp, in.uv, 0.0).r;
    let gi = textureSampleLevel(gi_tex, samp, in.uv, 0.0).rgb;
    let ao_strength = 0.6;
    let gi_strength = 0.2;
    let comp = hdr * (1.0 - ao * ao_strength) + gi * gi_strength;
    // Exposure boost before ACES gives the tonemapper proper HDR range,
    // producing richer contrast and more vivid highlights.
    let exposure = 1.35;
    var color = aces_tonemap(comp * exposure);
    // Screen-space tint overlay from biome transitions
    color = mix(color, uPostScene.tint_color, uPostScene.tint_alpha);
    return vec4<f32>(color, 1.0);
}
"#;

const SKINNED_SHADER_SRC: &str = concat!(
    include_str!("../shaders/constants.wgsl"),
    include_str!("../shaders/brdf_common.wgsl"),
    r#"
struct VSIn {
  @location(0) position: vec3<f32>,
  @location(1) normal:   vec3<f32>,
    @location(12) tangent:  vec4<f32>,
  @location(10) joints:  vec4<u32>,
  @location(11) weights: vec4<f32>,
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
  @location(2) color: vec4<f32>,
};

struct Camera { view_proj: mat4x4<f32>, light_dir: vec3<f32>, _pad: f32, camera_pos: vec3<f32>, _pad2: f32 };
@group(0) @binding(0) var<uniform> uCamera: Camera;

struct MaterialUbo { base_color: vec4<f32>, metallic: f32, roughness: f32, alpha_cutoff: f32, _pad: f32 };
@group(1) @binding(0) var<uniform> uMaterial: MaterialUbo;

struct MainLightUbo { view_proj0: mat4x4<f32>, view_proj1: mat4x4<f32>, splits: vec2<f32>, extras: vec2<f32> };
@group(2) @binding(0) var<uniform> uLight: MainLightUbo;
@group(2) @binding(1) var shadow_tex: texture_depth_2d_array;
@group(2) @binding(2) var shadow_sampler: sampler_comparison;

@group(3) @binding(0) var albedo_tex: texture_2d<f32>;
@group(3) @binding(1) var albedo_samp: sampler;
@group(3) @binding(2) var mr_tex: texture_2d<f32>;
@group(3) @binding(3) var mr_samp: sampler;
@group(3) @binding(4) var normal_tex: texture_2d<f32>;
@group(3) @binding(5) var normal_samp: sampler;
struct Skinning { mats: array<mat4x4<f32>> };
@group(3) @binding(6) var<storage, read> skin: Skinning;

// ── Scene Environment ─────────
struct SceneEnv {
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_start: f32,
    fog_end: f32,
    _pad0: vec2<f32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    tint_color: vec3<f32>,
    tint_alpha: f32,
    blend_factor: f32,
    _pad1x: f32, _pad1y: f32, _pad1z: f32,
    sun_color: vec3<f32>,
    sun_intensity: f32,
};
@group(4) @binding(0) var<uniform> uScene: SceneEnv;

// Group 5 shares the main PBR IBL layout; skinned shader uses cloud shadow bindings.
@group(5) @binding(7) var cloud_shadow_tex: texture_2d<f32>;
@group(5) @binding(8) var cloud_shadow_samp: sampler;

fn sample_cloud_shadow(world_pos: vec3<f32>) -> f32 {
    let shadow_extent = 1024.0;
    let uv = (world_pos.xz - uCamera.camera_pos.xz) / (2.0 * shadow_extent) + vec2<f32>(0.5, 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return 1.0;
    }
    return textureSampleLevel(cloud_shadow_tex, cloud_shadow_samp, uv, 0.0).r;
}

@vertex
fn vs(input: VSIn) -> VSOut {
  let model_inst = mat4x4<f32>(input.m0, input.m1, input.m2, input.m3);
  let j = input.joints;
  let w = input.weights;
  let m0 = skin.mats[u32(j.x)];
  let m1 = skin.mats[u32(j.y)];
  let m2 = skin.mats[u32(j.z)];
  let m3 = skin.mats[u32(j.w)];
  let pos4 = vec4<f32>(input.position, 1.0);
    let nrm4 = vec4<f32>(input.normal, 0.0);
  let skinned_pos = (m0 * pos4) * w.x + (m1 * pos4) * w.y + (m2 * pos4) * w.z + (m3 * pos4) * w.w;
  let skinned_nrm = (m0 * nrm4) * w.x + (m1 * nrm4) * w.y + (m2 * nrm4) * w.z + (m3 * nrm4) * w.w;
    let tan4 = vec4<f32>(input.tangent.xyz, 0.0);
    let skinned_tan = (m0 * tan4) * w.x + (m1 * tan4) * w.y + (m2 * tan4) * w.z + (m3 * tan4) * w.w;
  let world = model_inst * skinned_pos;
  let normal_mat = mat3x3<f32>(input.n0, input.n1, input.n2);
  var out: VSOut;
  out.pos = uCamera.view_proj * world;
    // Apply instance normal matrix (inverse-transpose) to skinned normals
    let Nw = normalize(normal_mat * skinned_nrm.xyz);
    let Tw = normalize(normal_mat * skinned_tan.xyz);
    let Bw = normalize(cross(Nw, Tw)) * input.tangent.w;
    out.normal = Nw;
  out.world_pos = world.xyz;
    out.tbn0 = Tw;
    out.tbn1 = Bw;
    out.tbn2 = Nw;
  out.color = input.color;
  return out;
}

@fragment
fn fs(input: VSOut) -> @location(0) vec4<f32> {
    let V = normalize(uCamera.camera_pos - input.world_pos);
    let L = normalize(-uCamera.light_dir);
    var N = normalize(input.normal);
    var base_color = (uMaterial.base_color.rgb * input.color.rgb);
    var metallic = clamp(uMaterial.metallic, 0.0, 1.0);
    var roughness = clamp(uMaterial.roughness, 0.04, 1.0);
    let F0 = mix(vec3<f32>(0.04, 0.04, 0.04), base_color, metallic);

    // Material LOD: simplify shading for distant / sub-pixel skinned fragments.
    let mat_lod = compute_material_lod(input.world_pos);

    // Unified BRDF: Cook-Torrance specular + Burley diffuse (from brdf_common.wgsl)
    let brdf_result = evaluate_brdf_lod(N, V, L, base_color, metallic, roughness, F0, mat_lod);

    let radiance = uScene.sun_color * uScene.sun_intensity;
    // Cascaded shadow sampling with edge fade (same as static path)
    let dist = length(input.world_pos - uCamera.camera_pos);
    let shadow_far = uLight.splits.y;
    let use_c0 = dist < uLight.splits.x;
    var lvp: mat4x4<f32>;
    if (use_c0) { lvp = uLight.view_proj0; } else { lvp = uLight.view_proj1; }
    let lp = lvp * vec4<f32>(input.world_pos, 1.0);
    let ndc = lp.xyz / lp.w;
    let uv = ndc.xy * 0.5 + vec2<f32>(0.5, 0.5);
    let depth = ndc.z;
    let slope = max(0.0, 1.0 - dot(N, L));
    let base_bias = uLight.extras.y;
    let bias = max(base_bias, 0.00001);
    var shadow: f32 = 1.0;
    if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0 && dist < shadow_far) {
        let layer = i32(select(1, 0, use_c0));
        let dims = vec2<f32>(textureDimensions(shadow_tex).xy);
        let texel = 1.0 / dims;
        var sum = 0.0;
        for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
            for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
                let o = vec2<f32>(f32(dx), f32(dy)) * texel * max(0.0, uLight.extras.x);
                sum = sum + textureSampleCompare(shadow_tex, shadow_sampler, uv + o, layer, depth - bias);
            }
        }
        shadow = sum / 9.0;
        let fade_start = shadow_far * 0.8;
        if (dist > fade_start) {
            let fade = (dist - fade_start) / (shadow_far - fade_start);
            shadow = mix(shadow, 1.0, clamp(fade, 0.0, 1.0));
        }
        let edge_fade_x = min(uv.x, 1.0 - uv.x) * 10.0;
        let edge_fade_y = min(uv.y, 1.0 - uv.y) * 10.0;
        let edge_fade = clamp(min(edge_fade_x, edge_fade_y), 0.0, 1.0);
        shadow = mix(1.0, shadow, edge_fade);
    }
    if (uLight.extras.x < 0.0) {
        shadow = 1.0;
    }
    // Direct lighting (evaluate_brdf already includes NdotL) + ambient lift
    let cloud_shadow = sample_cloud_shadow(input.world_pos);
    let lit_color = brdf_result * radiance * shadow * cloud_shadow + base_color * 0.08;
    return vec4<f32>(lit_color, uMaterial.base_color.a * input.color.a);
}
"#
);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUBO {
    view_proj: [[f32; 4]; 4],
    light_dir_pad: [f32; 4],
    camera_pos_pad: [f32; 4],
}

/// A named model with its mesh and instance data for multi-model rendering.
pub struct RenderModel {
    /// The GPU mesh (vertex/index buffers).
    pub mesh: Mesh,
    /// Instance buffer for this model.
    pub instance_buf: wgpu::Buffer,
    /// Number of instances.
    pub instance_count: u32,
    /// World-space AABB for frustum culling (None = always visible).
    pub aabb: Option<([f32; 3], [f32; 3])>,
    /// Per-model texture bind group (group 3). When Some, overrides global tex_bg.
    pub tex_bind_group: Option<wgpu::BindGroup>,
    /// Retained GPU texture to keep the bind group's texture view alive.
    pub _retained_tex: Option<wgpu::Texture>,
    /// GPU memory used by this model's texture (for budget tracking).
    pub tex_gpu_bytes: u64,
}

pub struct Renderer {
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// Set to `true` by the error callback when an internal GPU error is
    /// detected (driver crash, TDR, device removed).  Once set, the
    /// renderer stops issuing GPU work and [`is_device_lost`] returns
    /// `true` so the application can recreate it.
    device_lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Vulkan pipeline cache for faster shader compilation on subsequent launches.
    pipeline_cache: Option<wgpu::PipelineCache>,
    /// Keeps PipelineCacheManager alive for its Drop impl (persists cache to disk).
    _pipeline_cache_mgr: Option<crate::pipeline_cache::PipelineCacheManager>,
    /// Per-pass GPU timestamp profiler; `None` when `TIMESTAMP_QUERY` is unsupported.
    gpu_profiler: Option<crate::gpu_profiler::GpuProfiler>,
    /// Ring buffer for transient per-frame GPU allocations (uniforms, storage).
    staging_ring: crate::staging_ring::StagingRing,
    /// Monotonic counter bumped on resize and resource invalidation; drives
    /// [`CachedBindGroup`] rebuild decisions across all render passes.
    resource_generation: crate::bind_group_cache::Generation,
    /// Double-precision world position of the camera.  When the
    /// `camera-relative` feature is active the renderer offsets all
    /// geometry and light positions by this value so the GPU only ever
    /// works with small camera-relative coordinates.
    #[cfg(feature = "camera-relative")]
    camera_world_pos: glam::DVec3,
    config: wgpu::SurfaceConfiguration,
    depth: Depth,

    #[allow(dead_code)]
    shader: wgpu::ShaderModule,
    pipeline: wgpu::RenderPipeline,
    material_buf: wgpu::Buffer,
    material_bg: wgpu::BindGroup,
    post_pipeline: wgpu::RenderPipeline,
    /// Passthrough blit pipeline for editor mode (copies HDR without tonemapping).
    /// Uses `Rgba16Float` target format so it's compatible with the editor's HDR buffer.
    hdr_blit_pipeline: wgpu::RenderPipeline,
    /// Bind group for the HDR blit pass (hdr_tex + sampler, independent of postfx).
    hdr_blit_bind_group: wgpu::BindGroup,
    hdr_blit_bgl: wgpu::BindGroupLayout,
    post_bind_group: wgpu::BindGroup,
    post_bgl: wgpu::BindGroupLayout,
    hdr_tex: wgpu::Texture,
    hdr_view: wgpu::TextureView,
    hdr_sampler: wgpu::Sampler,
    // 1×1 black dummy texture used as AO/GI placeholder when SSAO/SSGI aren't active
    #[allow(dead_code)]
    _postfx_dummy_tex: wgpu::Texture,
    postfx_dummy_view: wgpu::TextureView,
    #[allow(dead_code)]
    shadow_tex: wgpu::Texture,
    #[allow(dead_code)]
    shadow_view: wgpu::TextureView, // array view for sampling
    shadow_layer0_view: wgpu::TextureView,
    shadow_layer1_view: wgpu::TextureView,
    #[allow(dead_code)]
    shadow_sampler: wgpu::Sampler,
    shadow_pipeline: wgpu::RenderPipeline,
    light_buf: wgpu::Buffer,
    light_bg: wgpu::BindGroup,
    // Per-cascade uniform buffers for shadow depth passes.
    // Using separate buffers avoids the queue.write_buffer race where all
    // writes resolve before the command buffer executes, causing both shadow
    // passes to read the same (last-written) cascade matrix.
    shadow_cascade_bufs: [wgpu::Buffer; 2],
    shadow_cascade_bgs: [wgpu::BindGroup; 2],
    #[allow(dead_code)]
    shadow_bgl: wgpu::BindGroupLayout,
    // Cascade data cached on CPU for shadow passes
    cascade0: glam::Mat4,
    cascade1: glam::Mat4,
    split0: f32,
    split1: f32,
    // Tunable cascade ortho extents (half-width/height)
    cascade0_extent: f32,
    cascade1_extent: f32,
    // CSM tuning
    cascade_lambda: f32, // split distribution (0..1)
    shadow_pcf_radius_px: f32,
    shadow_depth_bias: f32,
    shadow_slope_scale: f32,
    /// Whether shadows are enabled for rendering
    shadows_enabled: bool,
    /// Maximum draw distance for model culling (0.0 = use fog_end * 1.2 fallback).
    /// When set to a positive value, models beyond this distance are skipped.
    max_draw_distance: f32,
    /// Number of models actually rendered in the last frame (after culling).
    rendered_model_count: u32,
    /// Whether `set_terrain_ground_plane()` has positioned the plane for terrain.
    /// When true, `draw_into()` skips the default plane overwrite.
    terrain_ground_set: bool,
    /// Cached clustered lighting offsets to skip recomputation when lights are static.
    clustered_offsets_cache: Option<Vec<u32>>,
    /// Debug flag: when true, force shadow factor to 1.0 (shadows off) in the shader.
    /// Defaults to `false` — normal runtime uses computed PCF shadows.
    pub force_shadow_override: bool,

    // Albedo (base color) texture and sampler
    albedo_tex: wgpu::Texture,
    albedo_view: wgpu::TextureView,
    albedo_sampler: wgpu::Sampler,
    tex_bgl: wgpu::BindGroupLayout,
    tex_bg: wgpu::BindGroup,
    // Metallic-Roughness texture and sampler
    mr_tex: wgpu::Texture,
    mr_view: wgpu::TextureView,
    mr_sampler: wgpu::Sampler,
    // Normal map texture and sampler
    normal_tex: wgpu::Texture,
    normal_view: wgpu::TextureView,
    normal_sampler: wgpu::Sampler,
    // Extra textures bind group layout and group (for future extensibility)
    // extra texture bind group layout/bg removed; combined tex_bgl/tex_bg used
    camera_ubo: CameraUBO,
    camera_buf: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    // Cached matrices for skybox (manual translation removal)
    cached_view: glam::Mat4,
    cached_proj: glam::Mat4,

    #[allow(dead_code)]
    mesh_cube: Mesh,
    mesh_sphere: Mesh,
    mesh_plane: Mesh,
    mesh_external: Option<Mesh>,
    /// Named models for multi-model rendering (terrain, trees, rocks, etc.)
    models: std::collections::HashMap<String, RenderModel>,

    instances: Vec<Instance>,

    /// Optional light direction/intensity override from the editor's world panel.
    /// When Some, `update_camera_matrices` uses this instead of TimeOfDay.
    light_override: Option<([f32; 3], f32)>,
    instance_buf: wgpu::Buffer,

    #[allow(dead_code)]
    overlay: crate::overlay::OverlayFx,
    pub overlay_params: crate::overlay::OverlayParams,
    pub weather: crate::effects::WeatherFx,
    /// Material bind group for weather particles (bright white, non-metallic).
    weather_material_bg: wgpu::BindGroup,
    // Environment & sky
    sky: crate::environment::SkyRenderer,

    // Skinning (v0)
    #[allow(dead_code)]
    skin_bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    skin_bg: wgpu::BindGroup,
    skin_palette_buf: wgpu::Buffer,
    #[allow(dead_code)]
    skinned_pipeline: wgpu::RenderPipeline,
    skinned_mesh: Option<(wgpu::Buffer, wgpu::Buffer, u32)>, // (vbuf, ibuf, index_count)

    // Clustered lighting resources
    clustered_dims: ClusterDims,
    clustered_params_buf: wgpu::Buffer,
    clustered_lights_buf: wgpu::Buffer,
    clustered_offsets_buf: wgpu::Buffer,
    clustered_counts_buf: wgpu::Buffer,
    #[allow(dead_code)]
    clustered_indices_buf: wgpu::Buffer,
    #[allow(dead_code)]
    clustered_bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    clustered_bg: wgpu::BindGroup,
    #[allow(dead_code)]
    clustered_comp_bgl: wgpu::BindGroupLayout,
    clustered_comp_bg: wgpu::BindGroup,
    clustered_comp_pipeline: wgpu::ComputePipeline,
    point_lights: Vec<CpuLight>,
    #[cfg(feature = "gpu-tests")]
    timestamp_query_set: wgpu::QuerySet,
    #[cfg(feature = "gpu-tests")]
    timestamp_buf: wgpu::Buffer,

    // Cinematics integration
    cin_tl: Option<awc::Timeline>,
    cin_seq: awc::Sequencer,
    cin_playing: bool,

    // Persistent instance buffers
    pub plane_inst_buf: wgpu::Buffer,
    pub ext_inst_buf: Option<wgpu::Buffer>,
    pub ext_inst_count: u32,

    // IBL
    pub ibl: crate::ibl::IblManager,
    pub ibl_resources: Option<crate::ibl::IblResources>,
    ibl_bind_group: wgpu::BindGroup,
    ibl_params_buf: wgpu::Buffer,

    // Screen-space Global Illumination (composite into PBR via group(5) bindings 5-6)
    gi_fallback_view: wgpu::TextureView,
    gi_sampler: wgpu::Sampler,
    // Cloud shadow map generation and sampling resources (group 5 bindings 7-8).
    cloud_shadow_pass: crate::volumetric_clouds::CloudShadowPass,
    /// Whether to dispatch the per-frame cloud shadow compute pass.
    /// Disable in editor modes to avoid noisy shadow patterns on terrain.
    cloud_shadows_enabled: bool,

    // Water rendering
    water_renderer: Option<crate::water::WaterRenderer>,

    // Biome material system — bridges terrain BiomeType → materials + IBL
    biome_system: crate::biome_material::BiomeMaterialSystem,

    // Biome transition detection and visual blending
    biome_detector: crate::biome_detector::BiomeDetector,
    transition_effect: crate::biome_transition::TransitionEffect,
    scene_env: crate::scene_environment::SceneEnvironment,

    // Scene environment GPU resources (fog, ambient, tint UBO)
    scene_env_buf: wgpu::Buffer,
    #[allow(dead_code)]
    scene_env_bgl: wgpu::BindGroupLayout,
    scene_env_bg: wgpu::BindGroup,

    // Terrain material manager — loads biome texture arrays onto GPU
    pub material_manager: crate::material::MaterialManager,

    /// Post-processing chain configuration (controls which effects are active).
    post_chain: crate::hdr_pipeline::PostProcessChain,

    /// Bloom compute pass (created lazily when bloom is first enabled).
    bloom_pass: Option<crate::bloom::BloomPass>,
    /// Pending bloom config to apply when the bloom pass is lazily created.
    pending_bloom_config: Option<crate::bloom::BloomConfig>,
    /// Uniform buffer for HDR-blit post-effect compositing parameters.
    /// Layout: `[bloom_intensity, 0.0, 0.0, 0.0]` (16 bytes, `vec4<f32>` in WGSL).
    postfx_params_buf: wgpu::Buffer,

    /// GPU memory budget tracker (editor-specific budgets).
    gpu_memory_budget: std::sync::Arc<crate::gpu_memory::GpuMemoryBudget>,
}

impl Renderer {
    /// Compose a standalone fragment shader from a `MaterialPackage` for validation/pipeline creation.
    /// Returns a `ShaderModule` ready to be used in a pipeline (caller wires layouts/bindings).
    pub fn shader_from_material_package(&self, pkg: &MaterialPackage) -> wgpu::ShaderModule {
        // Declare group(0) bindings based on `bindings` ids collected by the compiler (tex/sampler pairs)
        let mut decls = String::new();
        let mut idx = 0u32;
        for id in pkg.bindings.iter() {
            decls.push_str(&format!(
                "@group(0) @binding({}) var tex_{}: texture_2d<f32>;\n",
                idx, id
            ));
            idx += 1;
            decls.push_str(&format!(
                "@group(0) @binding({}) var samp_{}: sampler;\n",
                idx, id
            ));
            idx += 1;
        }
        // Compose WGSL: eval_material + a tiny VS/FS pair.
        let full = format!(
            "{}\n{}\nstruct VSOut {{ @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }};\n@vertex fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {{\n  var pos = array<vec2<f32>,3>(vec2<f32>(-1.0,-3.0), vec2<f32>(3.0,1.0), vec2<f32>(-1.0,1.0));\n  var o: VSOut; o.pos = vec4<f32>(pos[vid], 0.0, 1.0); o.uv = (pos[vid]+vec2<f32>(1.0,1.0))*0.5; return o; }}\n@fragment fn fs_main(i: VSOut) -> @location(0) vec4<f32> {{ let m = eval_material(i.uv); return vec4<f32>(m.base, 1.0); }}\n",
            decls, pkg.wgsl
        );
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("material composed shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(full)),
            })
    }
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("No adapter")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: {
                    let mut f = wgpu::Features::empty();
                    #[cfg(feature = "gpu-tests")]
                    {
                        f |= wgpu::Features::TIMESTAMP_QUERY;
                    }
                    if adapter.features().contains(wgpu::Features::PIPELINE_CACHE) {
                        f |= wgpu::Features::PIPELINE_CACHE;
                    }
                    if adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
                        f |= wgpu::Features::TIMESTAMP_QUERY;
                    }
                    f
                },
                required_limits: wgpu::Limits {
                    max_bind_groups: 8,
                    ..wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await?;

        // Register GPU error callback for validation errors and device loss.
        let device_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let dl = device_lost.clone();
            device.on_uncaptured_error(Box::new(move |error| {
                log::error!("wgpu device error: {error}");
                // Internal errors typically signal GPU device loss (TDR, driver
                // crash, device removed).  Flag the renderer so the application
                // can recreate it instead of continuing to submit broken work.
                if matches!(error, wgpu::Error::Internal { .. }) {
                    dl.store(true, std::sync::atomic::Ordering::SeqCst);
                    log::error!("GPU device loss detected — renderer must be recreated");
                }
            }));
        }

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self::new_from_device_monitored(device, queue, Some(surface), config, device_lost).await
    }

    pub async fn new_headless(width: u32, height: u32) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .context("No adapter")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("headless device"),
                required_features: {
                    let mut f = wgpu::Features::empty();
                    if adapter.features().contains(wgpu::Features::PIPELINE_CACHE) {
                        f |= wgpu::Features::PIPELINE_CACHE;
                    }
                    if adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
                        f |= wgpu::Features::TIMESTAMP_QUERY;
                    }
                    f
                },
                required_limits: wgpu::Limits {
                    max_bind_groups: 8,
                    ..wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await?;

        // Register GPU error callback for validation errors and device loss.
        let device_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let dl = device_lost.clone();
            device.on_uncaptured_error(Box::new(move |error| {
                log::error!("wgpu device error (headless): {error}");
                if matches!(error, wgpu::Error::Internal { .. }) {
                    dl.store(true, std::sync::atomic::Ordering::SeqCst);
                    log::error!("GPU device loss detected — renderer must be recreated");
                }
            }));
        }

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Self::new_from_device_monitored(device, queue, None, config, device_lost).await
    }

    pub async fn new_from_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: Option<wgpu::Surface<'static>>,
        config: wgpu::SurfaceConfiguration,
    ) -> Result<Self> {
        let device_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self::new_from_device_monitored(device, queue, surface, config, device_lost).await
    }

    async fn new_from_device_monitored(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: Option<wgpu::Surface<'static>>,
        config: wgpu::SurfaceConfiguration,
        device_lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<Self> {
        #[cfg(feature = "gpu-tests")]
        let timestamp_query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        #[cfg(feature = "gpu-tests")]
        let timestamp_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ts readback"),
            size: 16,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Pipeline cache — Vulkan/DX12 pipeline cache for faster shader compilation.
        // Loads prior cache data from disk if available; saves on drop.
        let pipeline_cache_mgr = crate::pipeline_cache::PipelineCacheManager::create(&device, None);
        let pipeline_cache = pipeline_cache_mgr.as_ref().map(|m| m.cache().clone());

        // GPU timestamp profiler — created when TIMESTAMP_QUERY is available.
        let gpu_profiler = if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            Some(crate::gpu_profiler::GpuProfiler::new(&device, &queue))
        } else {
            None
        };

        // Per-frame ring buffer for transient uniform/storage uploads.
        let staging_ring = crate::staging_ring::StagingRing::new(
            &device,
            crate::staging_ring::DEFAULT_RING_SIZE,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
        );
        let resource_generation: crate::bind_group_cache::Generation = 1;

        // Depth
        let depth = crate::depth::Depth::create(&device, &config);

        // Shaders / pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("basic shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SRC)),
        });

        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera ubo"),
            size: std::mem::size_of::<CameraUBO>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bg"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }],
        });

        // Material buffer
        let material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let material_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material ubo"),
            size: 32, // vec4 + 2 f32 + padding
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Seed the material buffer with default PBR values:
        // base_color = off-white (0.85, 0.78, 0.72, 1.0)
        // metallic = 0.0 (fully dielectric — terrain and most objects)
        // roughness = 1.0 (fully rough — prevents unwanted specular on terrain)
        // ao = 1.0, pad = 0.0
        let default_material: [f32; 8] = [0.85, 0.78, 0.72, 1.0, 0.0, 1.0, 1.0, 0.0];
        queue.write_buffer(&material_buf, 0, bytemuck::cast_slice(&default_material));

        // Scene environment bind group layout (created early so it can be used in pipeline layout)
        let scene_env_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene env bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let material_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material bg"),
            layout: &material_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: material_buf.as_entire_binding(),
            }],
        });

        // Weather particle material: bright white, non-metallic, fully rough so
        // rain/snow/sand particles are lit uniformly without dark shadowed streaks.
        // [base_color.rgba, metallic, roughness, pad, pad]
        let weather_mat_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("weather material ubo"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let weather_material: [f32; 8] = [1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        queue.write_buffer(&weather_mat_buf, 0, bytemuck::cast_slice(&weather_material));
        let weather_material_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("weather material bg"),
            layout: &material_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: weather_mat_buf.as_entire_binding(),
            }],
        });

        // HDR color target
        let hdr_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr tex"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let _hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let hdr_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hdr sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // 1×1 black dummy for AO/GI placeholders in the postfx compositing pass.
        // Without real SSAO/SSGI data, using this avoids the negative-brightness
        // artefact that occurs when the HDR scene texture is sampled as AO input.
        let postfx_dummy_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("postfx dummy black"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &postfx_dummy_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0u8; 8], // 4 × f16 = 8 bytes, all zeros → ao=0 (no occlusion), gi=black
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let postfx_dummy_view =
            postfx_dummy_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Uniform buffer for HDR-blit post-effect compositing (bloom intensity, etc.).
        let postfx_params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("postfx params"),
            size: 16, // vec4<f32>
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[cfg(feature = "postfx")]
        let hdr_aux = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr aux tex"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        #[cfg(feature = "postfx")]
        let _hdr_view = hdr_aux.create_view(&wgpu::TextureViewDescriptor::default());
        #[cfg(feature = "postfx")]
        let fx_gi = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fx gi tex"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        #[cfg(feature = "postfx")]
        let _hdr_view = fx_gi.create_view(&wgpu::TextureViewDescriptor::default());
        #[cfg(feature = "postfx")]
        let fx_ao = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fx ao tex"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        #[cfg(feature = "postfx")]
        let hdr_view = fx_ao.create_view(&wgpu::TextureViewDescriptor::default());

        // Postprocess pipeline
        #[cfg(not(feature = "postfx"))]
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(POST_SHADER)),
        });
        #[cfg(not(feature = "postfx"))]
        let post_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        #[cfg(not(feature = "postfx"))]
        let post_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post bg"),
            layout: &post_bgl,
            entries: &[
                #[cfg(not(feature = "postfx"))]
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                #[cfg(feature = "postfx")]
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&hdr_sampler),
                },
            ],
        });

        // Feature-gated SSR pass (passthrough using color + depth)
        #[cfg(feature = "postfx")]
        let ssr_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssr shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL_SSR)),
        });
        #[cfg(feature = "postfx")]
        let ssr_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssr bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    // color_tex
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // depth_tex
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    // sampler
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        #[cfg(feature = "postfx")]
        let _post_bind_group_ssr = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssr bg"),
            layout: &ssr_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&hdr_sampler),
                },
            ],
        });
        // Create a placeholder normal view for postfx initialization to avoid use-before-def
        let placeholder_normal_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("placeholder normal"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let normal_view =
            placeholder_normal_tex.create_view(&wgpu::TextureViewDescriptor::default());

        #[cfg(feature = "postfx")]
        let ssr_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssr layout"),
            bind_group_layouts: &[&ssr_bgl],
            push_constant_ranges: &[],
        });
        #[cfg(feature = "postfx")]
        let _post_pipeline_ssr = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: pipeline_cache.as_ref(),
            label: Some("ssr pipeline"),
            layout: Some(&ssr_pl),
            vertex: wgpu::VertexState {
                module: &ssr_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ssr_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // SSAO
        #[cfg(feature = "postfx")]
        let ssao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssao shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL_SSAO)),
        });
        #[cfg(feature = "postfx")]
        let ssao_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssao bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        #[cfg(feature = "postfx")]
        let _post_bind_group_ssao = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao bg"),
            layout: &ssao_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&hdr_sampler),
                },
            ],
        });
        #[cfg(feature = "postfx")]
        let ssao_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssao layout"),
            bind_group_layouts: &[&ssao_bgl],
            push_constant_ranges: &[],
        });
        #[cfg(feature = "postfx")]
        let _post_pipeline_ssao = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: pipeline_cache.as_ref(),
            label: Some("ssao pipeline"),
            layout: Some(&ssao_pl),
            vertex: wgpu::VertexState {
                module: &ssao_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ssao_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // SSGI
        #[cfg(feature = "postfx")]
        let ssgi_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssgi shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL_SSGI)),
        });
        #[cfg(feature = "postfx")]
        let ssgi_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssgi bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        #[cfg(feature = "postfx")]
        let _post_bind_group_ssgi = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssgi bg"),
            layout: &ssgi_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&hdr_sampler),
                },
            ],
        });
        #[cfg(feature = "postfx")]
        let ssgi_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssgi layout"),
            bind_group_layouts: &[&ssgi_bgl],
            push_constant_ranges: &[],
        });
        #[cfg(feature = "postfx")]
        let _post_pipeline_ssgi = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: pipeline_cache.as_ref(),
            label: Some("ssgi pipeline"),
            layout: Some(&ssgi_pl),
            vertex: wgpu::VertexState {
                module: &ssgi_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ssgi_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Post-fx composition pipeline
        #[cfg(feature = "postfx")]
        let post_fx_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post fx shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(POST_SHADER_FX)),
        });
        #[cfg(feature = "postfx")]
        let post_fx_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post fx bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        #[cfg(feature = "postfx")]
        let post_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post fx bg"),
            layout: &post_fx_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&postfx_dummy_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&postfx_dummy_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&hdr_sampler),
                },
            ],
        });
        #[cfg(feature = "postfx")]
        let post_fx_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post fx layout"),
            bind_group_layouts: &[&post_fx_bgl, &scene_env_bgl],
            push_constant_ranges: &[],
        });

        // When postfx is enabled, self.post_bgl must use the 4-binding layout
        // so that resize() recreates the bind group with the correct layout.
        #[cfg(feature = "postfx")]
        let post_bgl = post_fx_bgl;

        // Shadow bind group layout (declared early so we can include it in main pipeline layout)
        let shadow_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        // Combined textures + skin bind group layout (group 3): albedo, mr, normal textures + samplers, plus optional skin storage buffer
        // bindings: 0: albedo tex, 1: albedo samp, 2: mr tex, 3: mr samp, 4: normal tex, 5: normal samp, 6: skin palette (storage)
        let tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("combined tex+skin bgl"),
            entries: &[
                // albedo texture + sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // metallic-roughness texture + sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // normal texture + sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // skin palette (storage) - vertex-stage visibility but kept in same group to reduce group count
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // extra_tex_bgl is no longer needed; MR and normal are merged into tex_bgl

        // Clustered lighting bind group layout (fragment reads)
        let clustered_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("clustered bgl (frag)"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // IBL bind group layout (group 5): specular cube, irradiance cube, BRDF LUT, sampler, params
        let ibl_params_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ibl-params-bgl"),
            entries: &[
                // 0: specular cube
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 1: irradiance cube
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 2: BRDF LUT
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 3: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 4: IBL params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 5: GI texture (screen-space indirect diffuse from SSGI/Lumen)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 6: GI sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 7: cloud shadow transmittance texture
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 8: cloud shadow sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let cloud_shadow_pass = crate::volumetric_clouds::CloudShadowPass::new_default(&device, &queue);

        // IBL params uniform buffer
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct IblParamsGpu {
            ibl_intensity: f32,
            max_spec_lod: f32,
            _pad: [f32; 2],
        }
        let ibl_params_data = IblParamsGpu {
            ibl_intensity: 1.0,
            max_spec_lod: 4.0,
            _pad: [0.0; 2],
        };
        let ibl_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ibl_params_buf"),
            contents: bytemuck::bytes_of(&ibl_params_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Fallback 1x1 black cubemap for when no IBL environment is loaded
        let fallback_cube_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fallback_ibl_cube"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &fallback_cube_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // Moderate sky-fill cubemap (sRGB encoded → GPU decodes to linear).
            // Provides soft indirect light when no HDRI environment is loaded.
            // Values tuned to work with the corrected sun direction (NdotL≈0.7):
            //   Direct ≈ 1.4, IBL ≈ 0.15, Ambient ≈ 0.04 → total ≈ 1.59.
            //   After exposure(1.35) → ACES(2.15) ≈ 0.90 for sunlit white.
            //   Mid-tone (0.5) → ACES(1.07) ≈ 0.70 — good contrast.
            &[
                100, 115, 140, 255, // +X  (linear ~0.13, 0.17, 0.26)
                100, 115, 140, 255, // -X
                140, 160, 200, 255, // +Y  (sky — brighter, linear ~0.26, 0.35, 0.58)
                70, 65, 50, 255, // -Y  (ground bounce, linear ~0.06, 0.05, 0.03)
                100, 115, 140, 255, // +Z
                100, 115, 140, 255, // -Z
            ],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
        );
        let fallback_cube_view = fallback_cube_tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        // Fallback 1x1 BRDF LUT
        let fallback_brdf_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fallback_brdf_lut"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &fallback_brdf_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // Reasonable BRDF LUT fallback: brdf.x ≈ 1.0, brdf.y ≈ 0.0
            // so specular IBL = prefiltered * F (correct upper bound).
            &[255u8, 0u8, 0u8, 255u8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let fallback_brdf_view =
            fallback_brdf_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let ibl_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ibl_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Fallback 1x1 black texture for screen-space GI (SSGI/Lumen).
        // When no GI pass is active, this contributes zero indirect light.
        let gi_fallback_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gi_fallback"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gi_fallback_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0u8; 8], // 4 x f16 = 8 bytes, all zero → black
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let gi_fallback_view = gi_fallback_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let gi_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gi_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let ibl_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ibl_fallback_bg"),
            layout: &ibl_params_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_cube_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&fallback_cube_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&fallback_brdf_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&ibl_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: ibl_params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&gi_fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&gi_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(cloud_shadow_pass.shadow_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(cloud_shadow_pass.shadow_sampler()),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            // Group indices: 0: camera, 1: material, 2: shadow/light, 3: textures, 4: scene env, 5: IBL
            bind_group_layouts: &[
                &bind_layout,
                &material_bgl,
                &shadow_bgl,
                &tex_bgl,
                &scene_env_bgl,
                &ibl_params_bgl,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: pipeline_cache.as_ref(),
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[
                    crate::types::Vertex::layout(),
                    crate::types::InstanceRaw::layout(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth.format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        // after instance_buf creation
        let weather = crate::effects::WeatherFx::new(&device, 800);

        // Sky/environment
        let mut sky = crate::environment::SkyRenderer::new(Default::default());
        sky.init_gpu_resources(&device, wgpu::TextureFormat::Rgba16Float)?;

        // Post pipeline uses surface format
        #[cfg(not(feature = "postfx"))]
        let post_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post layout"),
            bind_group_layouts: &[&post_bgl, &scene_env_bgl],
            push_constant_ranges: &[],
        });
        #[cfg(feature = "postfx")]
        let post_pipeline_layout = post_fx_pl;

        let post_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: pipeline_cache.as_ref(),
            label: Some("post pipeline"),
            layout: Some(&post_pipeline_layout),
            vertex: wgpu::VertexState {
                #[cfg(not(feature = "postfx"))]
                module: &post_shader,
                #[cfg(feature = "postfx")]
                module: &post_fx_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                #[cfg(not(feature = "postfx"))]
                module: &post_shader,
                #[cfg(feature = "postfx")]
                module: &post_fx_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // HDR passthrough blit pipeline — used in editor mode (surface=None) to copy
        // the internal HDR buffer to an Rgba16Float external target without tonemapping.
        // When bloom is active, additively composites the bloom output into the HDR
        // colour before writing.  When bloom is off, bloom_tex is a 1×1 black dummy
        // and bloom_intensity is 0.0, so the shader path is a no-op passthrough.
        let hdr_blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hdr blit shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                r#"
struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    var p = array<vec2<f32>, 3>(vec2(-1.0,-3.0), vec2(3.0,1.0), vec2(-1.0,1.0));
    var out: VSOut;
    out.pos = vec4<f32>(p[vid], 0.0, 1.0);
    // Flip UV.y: wgpu NDC Y+ is top, but texture V=0 is top.
    // Without the flip the blit renders the scene upside-down.
    out.uv = vec2((p[vid].x + 1.0) * 0.5, (1.0 - p[vid].y) * 0.5);
    return out;
}
@group(0) @binding(0) var hdr_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var bloom_tex: texture_2d<f32>;
struct PostfxParams { bloom_intensity: f32, _pad1: f32, _pad2: f32, _pad3: f32, };
@group(0) @binding(3) var<uniform> pfx: PostfxParams;
@fragment fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    var color = textureSampleLevel(hdr_tex, samp, in.uv, 0.0);
    let bloom = textureSampleLevel(bloom_tex, samp, in.uv, 0.0);
    color = vec4(color.rgb + bloom.rgb * pfx.bloom_intensity, color.a);
    return color;
}
"#,
            )),
        });
        let hdr_blit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hdr blit bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let hdr_blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hdr blit bg"),
            layout: &hdr_blit_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&hdr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&postfx_dummy_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: postfx_params_buf.as_entire_binding(),
                },
            ],
        });
        let hdr_blit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hdr blit layout"),
            bind_group_layouts: &[&hdr_blit_bgl],
            push_constant_ranges: &[],
        });
        let hdr_blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: pipeline_cache.as_ref(),
            label: Some("hdr blit pipeline"),
            layout: Some(&hdr_blit_layout),
            vertex: wgpu::VertexState {
                module: &hdr_blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &hdr_blit_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Shadow map resources (2-layer array for CSM)
        let shadow_size: u32 = 2048;
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map"),
            size: wgpu::Extent3d {
                width: shadow_size,
                height: shadow_size,
                depth_or_array_layers: 2,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        // Array view for sampling
        let shadow_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor {
            usage: None,
            label: Some("shadow array view"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });
        // Per-layer views for rendering
        let shadow_layer0_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor {
            usage: None,
            label: Some("shadow layer0 view"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: Some(1),
        });
        let shadow_layer1_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor {
            usage: None,
            label: Some("shadow layer1 view"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 1,
            array_layer_count: Some(1),
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // shadow_bgl already created above
        let light_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light ubo"),
            // 2 mat4 (128 bytes) + vec2 splits + pad (16 bytes) => 144; round to 160
            size: 160,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let light_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light bg"),
            layout: &shadow_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });

        // Minimal layout for shadow-only pass: only the light uniform buffer (binding 0).
        let shadow_bgl_light = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow bgl light-only"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        // Per-cascade uniform buffers: each holds a single mat4x4 (64 bytes)
        // that the shadow vertex shader reads as its view_proj. Written once per
        // frame in update_camera(), avoiding the queue.write_buffer race that
        // previously caused both shadow passes to use the same cascade matrix.
        let shadow_cascade_bufs = [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shadow cascade0 ubo"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shadow cascade1 ubo"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        ];
        let shadow_cascade_bgs = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow cascade0 bg"),
                layout: &shadow_bgl_light,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow_cascade_bufs[0].as_entire_binding(),
                }],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow cascade1 bg"),
                layout: &shadow_bgl_light,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow_cascade_bufs[1].as_entire_binding(),
                }],
            }),
        ];

        // Shadow map pipeline (depth-only)
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                r#"
struct VSIn {
  @location(0) position: vec3<f32>,
  @location(1) normal:   vec3<f32>,
    @location(12) tangent:  vec4<f32>,
    @location(13) uv:       vec2<f32>,
  @location(2) m0: vec4<f32>,
  @location(3) m1: vec4<f32>,
  @location(4) m2: vec4<f32>,
  @location(5) m3: vec4<f32>,
};
struct VSOut { @builtin(position) pos: vec4<f32> };
struct Light { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> uLight: Light;
@vertex
fn vs(input: VSIn) -> VSOut {
  let model = mat4x4<f32>(input.m0, input.m1, input.m2, input.m3);
  var out: VSOut;
  out.pos = uLight.view_proj * (model * vec4<f32>(input.position, 1.0));
  return out;
}
@fragment fn fs() { }
"#,
            )),
        });
        // Shadow-only pipeline uses a light-only bind group layout so the
        // depth-only pass doesn't require bindings for the shadow texture/sampler.
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow layout"),
                bind_group_layouts: &[&shadow_bgl_light],
                push_constant_ranges: &[],
            });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: pipeline_cache.as_ref(),
            label: Some("shadow pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs"),
                buffers: &[
                    crate::types::Vertex::layout(),
                    crate::types::InstanceRaw::layout(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Default 1x1 white albedo
        let albedo_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("albedo tex"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let albedo_view = albedo_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let albedo_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("albedo sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        // Initialize albedo with a 1x1 white texel so sampling yields visible color
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &albedo_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8, 255u8, 255u8, 255u8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        // Skin palette storage buffer (max 64 bones) - create before bind group so it can be referenced
        let skin_palette_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skin palette"),
            size: (64 * 64) as u64, // 64 mat4 (16 floats * 4 bytes) = 1024 bytes; allocate 4096 (rounded)
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Default extra textures (create MR and normal before building combined bind group)
        let mr_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mr tex"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mr_view = mr_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mr_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mr samp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            ..Default::default()
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &mr_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0u8, 255u8, 0u8, 255u8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let normal_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("normal tex"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let normal_view = normal_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let normal_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("normal samp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            ..Default::default()
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &normal_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[128u8, 128u8, 255u8, 255u8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        // Combined bind group for albedo, mr, normal, and skin palette (bindings 0..6)
        let tex_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("combined tex bg"),
            layout: &tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&albedo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&mr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&mr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: skin_palette_buf.as_entire_binding(),
                },
            ],
        });

        // Skin palette storage buffer (max 64 bones)
        let skin_palette_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skin palette"),
            size: (64 * 64) as u64, // 64 mat4 (16 floats * 4 bytes) = 1024 bytes; allocate 4096 (rounded)
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let skin_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skin bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let skin_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skin bg"),
            layout: &skin_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: skin_palette_buf.as_entire_binding(),
            }],
        });

        // Skinned pipeline (skin storage is now in combined tex_bgl at binding 6)
        let skinned_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("skinned pipeline layout"),
                bind_group_layouts: &[
                    &bind_layout,
                    &material_bgl,
                    &shadow_bgl,
                    &tex_bgl,
                    &scene_env_bgl,
                    &ibl_params_bgl,
                ],
                push_constant_ranges: &[],
            });
        let skinned_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("skinned shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SKINNED_SHADER_SRC)),
        });
        let skinned_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: pipeline_cache.as_ref(),
            label: Some("skinned pipeline"),
            layout: Some(&skinned_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &skinned_shader,
                entry_point: Some("vs"),
                buffers: &[
                    crate::types::SkinnedVertex::layout(),
                    crate::types::InstanceRaw::layout(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &skinned_shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth.format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Default extra textures
        let mr_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mr tex"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mr_view = mr_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mr_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mr samp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            ..Default::default()
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &mr_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0u8, 255u8, 0u8, 255u8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let normal_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("normal tex"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let normal_view = normal_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let normal_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("normal samp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            ..Default::default()
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &normal_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[128u8, 128u8, 255u8, 255u8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        // extra_tex_bg removed; MR/normal are in combined tex_bg

        // Clustered resources default allocs
        // Create real meshes from built-in primitives
        let (cube_v, cube_i) = crate::primitives::cube();
        let (sphere_v, sphere_i) = crate::primitives::sphere(24, 24, 1.0);
        let cube_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_cube vertex_buf"),
            contents: bytemuck::cast_slice(&cube_v),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cube_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_cube index_buf"),
            contents: bytemuck::cast_slice(&cube_i),
            usage: wgpu::BufferUsages::INDEX,
        });
        let mesh_cube = Mesh {
            vertex_buf: cube_vb,
            index_buf: cube_ib,
            index_count: cube_i.len() as u32,
        };
        let sphere_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_sphere vertex_buf"),
            contents: bytemuck::cast_slice(&sphere_v),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sphere_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_sphere index_buf"),
            contents: bytemuck::cast_slice(&sphere_i),
            usage: wgpu::BufferUsages::INDEX,
        });
        let mesh_sphere = Mesh {
            vertex_buf: sphere_vb,
            index_buf: sphere_ib,
            index_count: sphere_i.len() as u32,
        };

        let (plane_v, plane_i) = crate::primitives::plane();
        let plane_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_plane vertex_buf"),
            contents: bytemuck::cast_slice(&plane_v),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let plane_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_plane index_buf"),
            contents: bytemuck::cast_slice(&plane_i),
            usage: wgpu::BufferUsages::INDEX,
        });
        let mesh_plane = Mesh {
            vertex_buf: plane_vb,
            index_buf: plane_ib,
            index_count: plane_i.len() as u32,
        };
        // Dummy instance buffer
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dummy instance_buf"),
            size: 256,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let clustered_dims = ClusterDims { x: 8, y: 4, z: 8 };
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        // Use explicit 16-byte slots to match WGSL uniform layout: three vec4-sized slots = 48 bytes
        struct CParams {
            screen: [u32; 4],
            clusters: [u32; 4],
            params: [f32; 4],
        }
        let cparams_init = CParams {
            screen: [config.width.max(1), config.height.max(1), 0, 0],
            clusters: [clustered_dims.x, clustered_dims.y, clustered_dims.z, 0],
            params: [0.1, 200.0, std::f32::consts::FRAC_PI_3, 0.0],
        };
        let clustered_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cparams"),
            contents: bytemuck::bytes_of(&cparams_init),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let clustered_lights_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("clights"),
            size: 64 * 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let clusters_total = (clustered_dims.x * clustered_dims.y * clustered_dims.z) as usize;
        let clustered_offsets_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("coffsets"),
            size: ((clusters_total + 1) * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let clustered_counts_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ccounts"),
            size: (clusters_total * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Reserve indices buffer capacity: lights * 64 as an upper bound placeholder
        let clustered_indices_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cindices"),
            size: (64 * 64 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Fragment path doesn't use clustered data in this build; create a minimal bind group matching the layout (binding 4 as uniform).
        let clustered_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("clustered bg"),
            layout: &clustered_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 4,
                resource: clustered_params_buf.as_entire_binding(),
            }],
        });

        // Compute pipeline for clustered binning
        let clustered_comp_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("clustered comp"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(WGSL_CLUSTER_BIN)),
        });
        let clustered_comp_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("clustered comp bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let clustered_comp_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("clustered comp pl"),
            bind_group_layouts: &[&clustered_comp_bgl],
            push_constant_ranges: &[],
        });
        let clustered_comp_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                cache: pipeline_cache.as_ref(),
                label: Some("clustered comp pipeline"),
                layout: Some(&clustered_comp_pl),
                module: &clustered_comp_shader,
                entry_point: Some("cs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            });
        let clustered_comp_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("clustered comp bg"),
            layout: &clustered_comp_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: clustered_lights_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: clustered_params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: clustered_offsets_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: clustered_counts_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: clustered_indices_buf.as_entire_binding(),
                },
            ],
        });

        // Create overlay resources now while `device` and `config` are still available.
        let overlay = crate::overlay::OverlayFx::new(&device, config.format);
        let overlay_params = crate::overlay::OverlayParams {
            fade: 0.0,
            letterbox: 0.0,
            _pad: [0.0; 2],
        };

        // Persistent buffers
        let plane_xform = glam::Mat4::from_translation(glam::vec3(0.0, -0.2, 0.0))
            * glam::Mat4::from_scale(glam::vec3(50.0, 1.0, 50.0));
        let plane_inst = Instance {
            transform: plane_xform,
            color: [0.1, 0.12, 0.14, 1.0],
            material_id: 0,
        }
        .raw();
        let plane_inst_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("plane inst"),
            contents: bytemuck::bytes_of(&plane_inst),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let ext_inst_buf = None;

        let ibl = crate::ibl::IblManager::new(&device, crate::ibl::IblQuality::Medium)
            .context("Failed to init IBL")?;

        // ── Scene Environment UBO (fog, ambient, tint) ──────────────────
        let scene_env_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene env ubo"),
            size: std::mem::size_of::<crate::scene_environment::SceneEnvironmentUBO>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_env_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene env bg"),
            layout: &scene_env_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_env_buf.as_entire_binding(),
            }],
        });
        // Seed with sensible defaults
        {
            let default_ubo = crate::scene_environment::SceneEnvironmentUBO::default();
            queue.write_buffer(&scene_env_buf, 0, bytemuck::bytes_of(&default_ubo));
        }

        Ok(Self {
            surface,
            device,
            queue,
            pipeline_cache,
            _pipeline_cache_mgr: pipeline_cache_mgr,
            gpu_profiler,
            staging_ring,
            resource_generation,
            #[cfg(feature = "camera-relative")]
            camera_world_pos: glam::DVec3::ZERO,
            config,
            depth,
            shader,
            pipeline,
            material_buf,
            material_bg,
            post_pipeline,
            hdr_blit_pipeline,
            hdr_blit_bind_group,
            hdr_blit_bgl,
            post_bind_group,
            post_bgl,
            hdr_tex,
            hdr_view,
            hdr_sampler,
            _postfx_dummy_tex: postfx_dummy_tex,
            postfx_dummy_view,
            shadow_tex,
            shadow_view,
            shadow_layer0_view,
            shadow_layer1_view,
            shadow_sampler,
            shadow_pipeline,
            light_buf,
            light_bg,
            shadow_bgl,
            shadow_cascade_bufs,
            shadow_cascade_bgs,
            cascade0: glam::Mat4::IDENTITY,
            cascade1: glam::Mat4::IDENTITY,
            split0: 60.0,
            split1: 120.0,
            cascade0_extent: 40.0,
            cascade1_extent: 80.0,
            cascade_lambda: 0.5,
            shadow_pcf_radius_px: 1.0,
            shadow_depth_bias: 0.0006,
            shadow_slope_scale: 0.002,
            shadows_enabled: true,         // Shadows enabled by default
            max_draw_distance: 0.0,        // 0 = use fog_end fallback
            rendered_model_count: 0,
            terrain_ground_set: false,     // No terrain ground plane set yet
            clustered_offsets_cache: None, // Force first-frame computation
            force_shadow_override: false,  // Normal runtime: use computed PCF shadows
            albedo_tex,
            albedo_view,
            albedo_sampler,
            tex_bgl,
            tex_bg,
            mr_tex,
            mr_view,
            mr_sampler,
            normal_tex,
            normal_view,
            normal_sampler,
            // combined tex_bgl/tex_bg used
            camera_ubo: CameraUBO {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                light_dir_pad: [0.5, 1.0, 0.8, 0.0],
                camera_pos_pad: [0.0, 0.0, 0.0, 0.0],
            },
            camera_buf,
            camera_bind_group,
            mesh_cube,
            mesh_sphere,
            mesh_plane,
            mesh_external: None,
            models: std::collections::HashMap::new(),
            instances: Vec::new(),
            light_override: None,
            instance_buf,
            overlay,
            overlay_params,
            weather,
            weather_material_bg,
            sky,
            skin_bgl,
            skin_bg,
            skin_palette_buf,
            skinned_pipeline,
            skinned_mesh: None,
            clustered_dims,
            clustered_params_buf,
            clustered_lights_buf,
            clustered_offsets_buf,
            clustered_counts_buf,
            clustered_indices_buf,
            clustered_bgl,
            clustered_bg,
            clustered_comp_bgl,
            clustered_comp_bg,
            clustered_comp_pipeline,
            point_lights: Vec::new(),
            #[cfg(feature = "gpu-tests")]
            timestamp_query_set,
            #[cfg(feature = "gpu-tests")]
            timestamp_buf,
            cin_tl: None,
            cin_seq: awc::Sequencer::new(),
            cin_playing: false,
            plane_inst_buf,
            ext_inst_buf,
            cached_view: glam::Mat4::IDENTITY,
            cached_proj: glam::Mat4::IDENTITY,
            ext_inst_count: 0,
            ibl,
            ibl_resources: None,
            ibl_bind_group,
            ibl_params_buf,
            gi_fallback_view,
            gi_sampler,
            cloud_shadow_pass,
            cloud_shadows_enabled: true,
            water_renderer: None,
            biome_system: crate::biome_material::BiomeMaterialSystem::new(
                crate::biome_material::BiomeMaterialConfig::default(),
            ),
            biome_detector: crate::biome_detector::BiomeDetector::new(
                crate::biome_detector::BiomeDetectorConfig::default(),
            ),
            transition_effect: crate::biome_transition::TransitionEffect::new(
                crate::biome_transition::TransitionConfig::default(),
            ),
            scene_env: crate::scene_environment::SceneEnvironment::default(),
            scene_env_buf,
            scene_env_bgl,
            scene_env_bg,
            material_manager: crate::material::MaterialManager::new(),
            post_chain: crate::hdr_pipeline::PostProcessChain::default(),
            bloom_pass: None,
            pending_bloom_config: None,
            postfx_params_buf,
            gpu_memory_budget: std::sync::Arc::new(crate::gpu_memory::GpuMemoryBudget::new()),
            device_lost,
        })
    }

    /// Returns `true` if the GPU device has been lost (driver crash, TDR,
    /// device removed).  When this returns `true` no further GPU work will
    /// be submitted and the application should drop this `Renderer` and
    /// create a new one.
    pub fn is_device_lost(&self) -> bool {
        self.device_lost.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Returns a reference to the pipeline cache, if available (Vulkan backends only).
    pub fn pipeline_cache(&self) -> Option<&wgpu::PipelineCache> {
        self.pipeline_cache.as_ref()
    }

    /// Returns the GPU profiler, if timestamp queries are supported.
    pub fn gpu_profiler(&self) -> Option<&crate::gpu_profiler::GpuProfiler> {
        self.gpu_profiler.as_ref()
    }

    /// Returns a reference to the per-frame staging ring buffer.
    pub fn staging_ring(&self) -> &crate::staging_ring::StagingRing {
        &self.staging_ring
    }

    /// Returns the current resource generation counter (bumped on resize).
    pub fn resource_generation(&self) -> crate::bind_group_cache::Generation {
        self.resource_generation
    }

    /// Persist the pipeline cache to disk so subsequent launches compile shaders faster.
    pub fn save_pipeline_cache(&self) {
        if let Some(ref cache) = self.pipeline_cache {
            if let Some(data) = cache.get_data() {
                let cache_dir = std::path::Path::new("cache");
                let _ = std::fs::create_dir_all(cache_dir);
                // Atomic write: write to temp then rename to avoid corruption on crash.
                let tmp = cache_dir.join("pipeline_cache.bin.tmp");
                let dst = cache_dir.join("pipeline_cache.bin");
                if std::fs::write(&tmp, &data).is_ok() {
                    let _ = std::fs::rename(&tmp, &dst);
                }
            }
        }
    }

    // --- Cinematics wiring ---
    fn apply_camera_key(cam: &mut Camera, k: &awc::CameraKey) {
        let pos = glam::Vec3::new(k.pos.0, k.pos.1, k.pos.2);
        let look = glam::Vec3::new(k.look_at.0, k.look_at.1, k.look_at.2);
        let dir = (look - pos).normalize_or_zero();
        let yaw = dir.z.atan2(dir.x);
        let pitch = dir.y.clamp(-1.0, 1.0).asin();
        cam.position = pos;
        cam.yaw = yaw;
        cam.pitch = pitch;
        cam.fovy = k.fov_deg.to_radians();
    }

    pub fn load_timeline_json(&mut self, json: &str) -> Result<()> {
        let tl: awc::Timeline = serde_json::from_str(json)?;
        self.cin_tl = Some(tl);
        self.cin_seq.seek(awc::Time(0.0));
        Ok(())
    }

    pub fn save_timeline_json(&self) -> Option<String> {
        self.cin_tl
            .as_ref()
            .and_then(|tl| serde_json::to_string_pretty(tl).ok())
    }

    pub fn play_timeline(&mut self) {
        self.cin_playing = true;
    }
    pub fn stop_timeline(&mut self) {
        self.cin_playing = false;
    }
    pub fn seek_timeline(&mut self, t: f32) {
        self.cin_seq.seek(awc::Time(t));
    }

    /// Step the sequencer and apply camera keys; returns emitted events (for audio/FX handling by caller)
    pub fn tick_cinematics(&mut self, dt: f32, camera: &mut Camera) -> Vec<awc::SequencerEvent> {
        let mut out = Vec::new();
        if !self.cin_playing {
            return out;
        }
        if let Some(tl) = self.cin_tl.as_ref() {
            if let Ok(evs) = self.cin_seq.step(dt, tl) {
                for e in evs.iter() {
                    match e {
                        awc::SequencerEvent::CameraKey(k) => Self::apply_camera_key(camera, k),
                        awc::SequencerEvent::FxTrigger { name, params } => {
                            // Minimal FX: support fade-in by instantly clearing letterbox/fade
                            if name == "fade-in" {
                                let _ = params; // reserved
                                self.overlay_params.fade = 0.0;
                            }
                        }
                        _ => {}
                    }
                }
                out = evs;
            }
        }
        out
    }

    pub fn ibl_mut(&mut self) -> &mut crate::ibl::IblManager {
        &mut self.ibl
    }

    pub fn bake_environment(&mut self, quality: crate::ibl::IblQuality) -> Result<()> {
        let resources = self
            .ibl
            .bake_environment(&self.device, &self.queue, quality)?;
        self.rebuild_ibl_bind_group(&resources);
        self.ibl_resources = Some(resources);
        Ok(())
    }

    /// Rebuild the IBL bind group from loaded IBL resources.
    fn rebuild_ibl_bind_group(&mut self, res: &crate::ibl::IblResources) {
        // IBL texture pointers are changing — invalidate the sky renderer's
        // cached bind groups so they get rebuilt with the new texture views.
        self.sky.invalidate_sky_cache();

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ibl_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct IblParamsGpu {
            ibl_intensity: f32,
            max_spec_lod: f32,
            _pad: [f32; 2],
        }
        // Normalise IBL intensity so different HDRIs produce similar terrain
        // brightness.  Target average luminance ≈ 0.35 (mid-grey).  For
        // procedural sky (avg_luminance = None) use 1.0.
        let ibl_intensity = match res.avg_luminance {
            Some(avg) if avg > 0.01 => (0.35 / avg).clamp(0.3, 3.0),
            _ => 1.0,
        };
        log::info!(
            "IBL bake: avg_luminance={:?}, ibl_intensity={:.3}, mips_specular={}",
            res.avg_luminance,
            ibl_intensity,
            res.mips_specular,
        );
        let params = IblParamsGpu {
            ibl_intensity,
            max_spec_lod: res.mips_specular.saturating_sub(1).max(1) as f32,
            _pad: [0.0; 2],
        };
        self.queue
            .write_buffer(&self.ibl_params_buf, 0, bytemuck::bytes_of(&params));

        // Recreate bind group with actual IBL textures.
        // We need the layout — extract from the current bind group's pipeline.
        // Use the IBL bind group layout from the pipeline.
        let bgl = self.pipeline.get_bind_group_layout(5);
        self.ibl_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ibl_bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&res.specular_cube),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&res.irradiance_cube),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&res.brdf_lut),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.ibl_params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.gi_fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.gi_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(
                        self.cloud_shadow_pass.shadow_view(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(
                        self.cloud_shadow_pass.shadow_sampler(),
                    ),
                },
            ],
        });
    }

    // ── Biome Material System ────────────────────────────────────────────

    /// Get immutable reference to the biome material system.
    pub fn biome_system(&self) -> &crate::biome_material::BiomeMaterialSystem {
        &self.biome_system
    }

    /// Get mutable reference to the biome material system.
    pub fn biome_system_mut(&mut self) -> &mut crate::biome_material::BiomeMaterialSystem {
        &mut self.biome_system
    }

    /// Transition to a new biome.
    ///
    /// This:
    /// 1. Checks if a transition is needed (same biome → no-op).
    /// 2. Resolves the best HDRI for the biome + current time-of-day.
    /// 3. Updates `IblManager` sky mode from the HDRI catalog.
    /// 4. Rebakes environment maps (irradiance + specular).
    /// 5. Marks the biome as loaded in the tracking state.
    ///
    /// For the full pipeline (HDRI + terrain textures), use
    /// [`transition_biome_full`] which also calls `MaterialManager::load_biome()`.
    ///
    /// Returns `true` if a transition occurred, `false` if already in that biome.
    pub fn transition_biome(
        &mut self,
        biome: astraweave_terrain::biome::BiomeType,
        quality: crate::ibl::IblQuality,
    ) -> Result<bool> {
        if !self.biome_system.needs_transition(biome) {
            return Ok(false);
        }

        // Resolve HDRI for this biome + time
        let sky_mode = self.biome_system.resolve_sky_mode(biome)?;
        let hdri_path = self.biome_system.resolve_hdri_path(biome)?;

        // Update IBL manager
        self.ibl.mode = sky_mode;

        // Rebake environment
        let resources = self
            .ibl
            .bake_environment(&self.device, &self.queue, quality)?;
        self.rebuild_ibl_bind_group(&resources);
        self.ibl_resources = Some(resources);

        // Track state
        self.biome_system.mark_loaded(biome, hdri_path);

        log::info!(
            "Biome transition → {:?} (HDRI: {:?})",
            biome,
            self.biome_system.current_biome()
        );

        Ok(true)
    }

    /// Full biome transition: HDRI environment + terrain material textures.
    ///
    /// This is the batteries-included version of [`transition_biome`]. It:
    /// 1. Resolves + loads the HDRI for the biome + time-of-day.
    /// 2. Rebakes the IBL environment maps.
    /// 3. Loads the biome's terrain material texture arrays via [`MaterialManager`].
    ///
    /// Returns `Ok(None)` if already in that biome (no-op).
    /// Returns `Ok(Some(stats))` with material load statistics on success.
    #[cfg(feature = "textures")]
    pub async fn transition_biome_full(
        &mut self,
        biome: astraweave_terrain::biome::BiomeType,
        quality: crate::ibl::IblQuality,
    ) -> Result<Option<crate::material::MaterialLoadStats>> {
        if !self.biome_system.needs_transition(biome) {
            return Ok(None);
        }

        // 1. HDRI + IBL
        let sky_mode = self.biome_system.resolve_sky_mode(biome)?;
        let hdri_path = self.biome_system.resolve_hdri_path(biome)?;
        self.ibl.mode = sky_mode;
        let resources = self
            .ibl
            .bake_environment(&self.device, &self.queue, quality)?;
        self.rebuild_ibl_bind_group(&resources);
        self.ibl_resources = Some(resources);

        // 2. Terrain material textures
        let mat_dir = self.biome_system.material_dir_for(biome);
        let stats = self
            .material_manager
            .load_biome(&self.device, &self.queue, &mat_dir)
            .await?;

        // 3. Track state
        self.biome_system.mark_loaded(biome, hdri_path);

        log::info!(
            "Full biome transition → {:?} (materials: {} layers, {:.2} MiB)",
            biome,
            stats.layers_total,
            stats.gpu_memory_bytes as f64 / (1024.0 * 1024.0),
        );

        Ok(Some(stats))
    }

    /// Update the biome system's time-of-day from the renderer's continuous
    /// `TimeOfDay` hours. If the discrete `DayPeriod` changed, rebake the
    /// environment HDRI.
    ///
    /// Call this once per frame (or less frequently) to keep sky lighting in
    /// sync with the time-of-day system.
    pub fn sync_biome_time_of_day(&mut self, quality: crate::ibl::IblQuality) -> Result<bool> {
        let hours = self.sky.time_of_day().current_time;
        let period = crate::hdri_catalog::DayPeriod::from_game_hours(hours);

        if !self.biome_system.set_time_of_day(period) {
            return Ok(false); // No change
        }

        // Period changed — if we have a loaded biome, refresh the HDRI
        if let Some(biome) = self.biome_system.current_biome() {
            let sky_mode = self.biome_system.resolve_sky_mode(biome)?;
            let hdri_path = self.biome_system.resolve_hdri_path(biome)?;
            self.ibl.mode = sky_mode;
            let resources = self
                .ibl
                .bake_environment(&self.device, &self.queue, quality)?;
            self.rebuild_ibl_bind_group(&resources);
            self.ibl_resources = Some(resources);
            self.biome_system.mark_loaded(biome, hdri_path);

            log::info!("Day period changed → {:?} for biome {:?}", period, biome);
        }

        Ok(true)
    }

    // ── Biome Transition Pipeline ────────────────────────────────────────

    /// Access the biome detector for direct queries.
    pub fn biome_detector(&self) -> &crate::biome_detector::BiomeDetector {
        &self.biome_detector
    }

    /// Access the transition effect for direct queries.
    pub fn transition_effect(&self) -> &crate::biome_transition::TransitionEffect {
        &self.transition_effect
    }

    /// Access the current scene environment (fog, ambient, tint).
    pub fn scene_environment(&self) -> &crate::scene_environment::SceneEnvironment {
        &self.scene_env
    }

    /// Mutably access the scene environment (e.g. to set weather multipliers).
    pub fn scene_environment_mut(&mut self) -> &mut crate::scene_environment::SceneEnvironment {
        &mut self.scene_env
    }

    /// Override the directional light direction and sun intensity.
    ///
    /// Used by the editor to apply the world panel's sun settings instead of the
    /// internal TimeOfDay system. The intensity is packed into `light_dir_pad[3]`.
    pub fn set_light_direction_override(&mut self, dir: glam::Vec3, intensity: f32) {
        self.light_override = Some(([dir.x, dir.y, dir.z], intensity));
    }

    /// Get the GPU-ready scene environment UBO for the current frame.
    ///
    /// This applies weather multipliers and returns the final uniform buffer
    /// data ready for `queue.write_buffer()`.
    pub fn scene_environment_ubo(&self) -> crate::scene_environment::SceneEnvironmentUBO {
        self.scene_env.to_ubo()
    }

    /// Update the player's world position and tick the biome transition
    /// pipeline. Call this once per frame (or whenever the player moves).
    ///
    /// This:
    /// 1. Feeds position to the `BiomeDetector` to check for biome changes.
    /// 2. On biome change: starts a `TransitionEffect` crossfade.
    /// 3. Advances the transition effect by `delta_time`.
    /// 4. Updates `SceneEnvironment` with interpolated fog/ambient.
    ///
    /// Returns `Some(biome)` if a new biome transition was started this frame.
    pub fn update_player_biome(
        &mut self,
        climate: &astraweave_terrain::climate::ClimateMap,
        x: f64,
        z: f64,
        height: f32,
        delta_time: f32,
    ) -> Option<astraweave_terrain::biome::BiomeType> {
        // 1. Check for biome transition
        let new_biome = if let Some(transition) = self.biome_detector.update(climate, x, z, height)
        {
            self.transition_effect
                .start(transition.old_biome, transition.new_biome);
            log::info!(
                "Biome transition detected: {:?} → {:?}",
                transition.old_biome,
                transition.new_biome,
            );
            // Trigger HDRI / IBL swap for the new biome (best-effort; non-fatal)
            if let Err(e) =
                self.transition_biome(transition.new_biome, crate::ibl::IblQuality::Medium)
            {
                log::warn!(
                    "IBL transition to {:?} failed (non-fatal): {e}",
                    transition.new_biome,
                );
            }
            Some(transition.new_biome)
        } else {
            None
        };

        // 2. Tick transition effect
        if self.transition_effect.is_active() {
            self.transition_effect.update(delta_time);
        }

        // 3. Update scene environment
        self.scene_env
            .update_from_transition(&self.transition_effect);

        // 4. Sync sky + water colours from the interpolated BiomeVisuals
        self.sync_biome_sky_water();

        new_biome
    }

    /// Configure the transition effect (duration, easing, etc.).
    pub fn set_transition_config(&mut self, config: crate::biome_transition::TransitionConfig) {
        self.transition_effect = crate::biome_transition::TransitionEffect::new(config);
    }

    /// Configure the biome detector (distance threshold, hysteresis).
    pub fn set_biome_detector_config(
        &mut self,
        config: crate::biome_detector::BiomeDetectorConfig,
    ) {
        self.biome_detector = crate::biome_detector::BiomeDetector::new(config);
    }

    /// Push current BiomeVisuals sky/water colours to the SkyRenderer and
    /// WaterRenderer.  Called automatically from [`Self::update_player_biome`]
    /// every frame while a transition is active (or at rest, so the colours
    /// stay clamped to the final biome).
    fn sync_biome_sky_water(&mut self) {
        let vis = &self.scene_env.visuals;

        // Sky — merge biome colours and cloud parameters into SkyConfig.
        let mut sky = self.sky.config().clone();
        sky.day_color_top = vis.sky_day_top;
        sky.day_color_horizon = vis.sky_day_horizon;
        sky.sunset_color_top = vis.sky_sunset_top;
        sky.sunset_color_horizon = vis.sky_sunset_horizon;
        sky.night_color_top = vis.sky_night_top;
        sky.night_color_horizon = vis.sky_night_horizon;
        sky.cloud_coverage = vis.cloud_coverage;
        sky.cloud_speed = vis.cloud_speed;
        self.sky.set_config(sky);

        // Water — update colours; they reach the GPU on the next
        // `WaterRenderer::update()` call done by `update_water()`.
        if let Some(ref mut water) = self.water_renderer {
            water.set_water_colors(vis.water_deep, vis.water_shallow, vis.water_foam);
        }

        // Weather particles — set biome-specific density and tint.
        self.weather.set_density(vis.weather_particle_density);

        // Use fog colour as a subtle tint so rain/wind matches the atmosphere.
        // Normalize to avoid darkening too much.
        let fog_avg = (vis.fog_color.x + vis.fog_color.y + vis.fog_color.z) / 3.0;
        let tint = if fog_avg > 0.01 {
            vis.fog_color / fog_avg * 0.9 + glam::Vec3::splat(0.1)
        } else {
            glam::Vec3::ONE
        };
        self.weather.set_biome_tint(tint);
    }

    pub fn resize(&mut self, new_w: u32, new_h: u32) {
        if new_w == 0 || new_h == 0 {
            return;
        }

        // Bump resource generation so all cached bind groups rebuild.
        self.resource_generation += 1;

        // Deallocate old render target memory from budget
        let old_rt_bytes = (self.config.width as u64) * (self.config.height as u64) * (8 + 4); // HDR(8) + Depth(4)
        self.gpu_memory_budget.deallocate(
            crate::gpu_memory::MemoryCategory::RenderTargets,
            old_rt_bytes,
        );

        self.config.width = new_w;
        self.config.height = new_h;
        if let Some(surface) = &self.surface {
            surface.configure(&self.device, &self.config);
        }
        self.depth = crate::depth::Depth::create(&self.device, &self.config);

        // Allocate new render target memory in budget
        let new_rt_bytes = (new_w as u64) * (new_h as u64) * (8 + 4); // HDR(8) + Depth(4)
        self.gpu_memory_budget.try_allocate(
            crate::gpu_memory::MemoryCategory::RenderTargets,
            new_rt_bytes,
        );

        // Recreate HDR target and refresh the post-processing bind group.
        self.hdr_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr tex"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.hdr_view = self
            .hdr_tex
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.post_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            #[cfg(not(feature = "postfx"))]
            label: Some("post bg"),
            #[cfg(feature = "postfx")]
            label: Some("post fx bg"),
            layout: &self.post_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.hdr_view),
                },
                #[cfg(feature = "postfx")]
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.postfx_dummy_view),
                },
                #[cfg(feature = "postfx")]
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.postfx_dummy_view),
                },
                wgpu::BindGroupEntry {
                    #[cfg(not(feature = "postfx"))]
                    binding: 1,
                    #[cfg(feature = "postfx")]
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.hdr_sampler),
                },
            ],
        });
        // Resize bloom pass if active
        if let Some(ref mut bloom) = self.bloom_pass {
            bloom.resize(&self.device, new_w, new_h);
        }
        // Recreate HDR blit bind group (references the new hdr_view + bloom view)
        let bloom_view = self
            .bloom_pass
            .as_ref()
            .and_then(|b| b.bloom_view())
            .unwrap_or(&self.postfx_dummy_view);
        self.hdr_blit_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hdr blit bg"),
            layout: &self.hdr_blit_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.hdr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(bloom_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.postfx_params_buf.as_entire_binding(),
                },
            ],
        });
        // Update clustered params screen size
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct CParams {
            screen: [u32; 4],
            clusters: [u32; 4],
            params: [f32; 4],
        }
        let data: CParams = CParams {
            screen: [new_w.max(1), new_h.max(1), 0, 0],
            clusters: [
                self.clustered_dims.x,
                self.clustered_dims.y,
                self.clustered_dims.z,
                0,
            ],
            params: [0.1, 200.0, std::f32::consts::FRAC_PI_3, 0.0],
        };
        self.queue
            .write_buffer(&self.clustered_params_buf, 0, bytemuck::bytes_of(&data));
    }

    /// Set the camera's true world-space position in double precision.
    ///
    /// When the `camera-relative` feature is active this value is subtracted
    /// from every model matrix and light position before GPU upload, keeping
    /// all GPU-side coordinates small and free from f32 jitter.  Call this
    /// **before** [`update_camera`] / [`update_camera_matrices`] each frame.
    #[cfg(feature = "camera-relative")]
    pub fn set_camera_world_position(&mut self, pos: glam::DVec3) {
        self.camera_world_pos = pos;
    }

    /// Returns the current camera-relative rendering origin.
    #[cfg(feature = "camera-relative")]
    pub fn camera_world_position(&self) -> glam::DVec3 {
        self.camera_world_pos
    }

    /// Update camera from pre-computed view and projection matrices.
    /// Use this when the caller already has correct matrices (e.g., editor orbit camera).
    ///
    /// When the `camera-relative` feature is active, the view matrix is
    /// automatically adjusted to remove translation, and `camera_pos` in the
    /// GPU UBO is set to zero.  Call [`set_camera_world_position`] first.
    #[allow(clippy::too_many_arguments)]
    pub fn update_camera_matrices(
        &mut self,
        view: glam::Mat4,
        proj: glam::Mat4,
        position: glam::Vec3,
        znear: f32,
        zfar: f32,
        fovy: f32,
        aspect: f32,
    ) {
        // When camera-relative, strip the translation from the view matrix so
        // that the camera sits at the world origin on the GPU.  All geometry is
        // offset by `camera_world_pos` during instance upload instead.
        #[cfg(feature = "camera-relative")]
        let (view, position) = {
            let _ = position; // suppress unused-variable warning for the parameter
            let mut v = view;
            v.w_axis = glam::Vec4::W; // zero out translation column
            (v, glam::Vec3::ZERO)
        };

        self.cached_view = view;
        self.cached_proj = proj;
        let vp = proj * view;
        self.camera_ubo.view_proj = vp.to_cols_array_2d();

        // Use the editor's light override if set, otherwise fall back to TimeOfDay.
        let (light_dir, sun_intensity) = if let Some((dir, intensity)) = self.light_override {
            (glam::Vec3::from(dir), intensity)
        } else {
            (self.sky.time_of_day().get_light_direction(), 1.0)
        };
        self.camera_ubo.light_dir_pad = [light_dir.x, light_dir.y, light_dir.z, sun_intensity];

        self.camera_ubo.camera_pos_pad = [position.x, position.y, position.z, 0.0];
        self.queue
            .write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&self.camera_ubo));

        // Build a temporary Camera for cascade computation
        // Extract forward direction from the view matrix (negative Z row)
        let fwd = -glam::Vec3::new(view.x_axis.z, view.y_axis.z, view.z_axis.z);
        let (yaw, pitch) = (fwd.z.atan2(fwd.x), fwd.y.asin());
        let tmp_cam = Camera {
            position,
            yaw,
            pitch,
            fovy,
            aspect,
            znear,
            zfar,
        };
        self.update_cascade_splits(&tmp_cam, light_dir);
    }

    pub fn update_camera(&mut self, camera: &Camera) {
        // When camera-relative, use rotation-only view (camera at origin).
        #[cfg(feature = "camera-relative")]
        let (view, cam_pos) = { (camera.view_matrix_camera_relative(), glam::Vec3::ZERO) };
        #[cfg(not(feature = "camera-relative"))]
        let (view, cam_pos) = (camera.view_matrix(), camera.position);

        self.cached_view = view;
        self.cached_proj = camera.proj_matrix();
        self.camera_ubo.view_proj = (camera.proj_matrix() * view).to_cols_array_2d();
        let light_dir = self.sky.time_of_day().get_light_direction();
        self.camera_ubo.light_dir_pad = [light_dir.x, light_dir.y, light_dir.z, 0.0];
        self.camera_ubo.camera_pos_pad = [cam_pos.x, cam_pos.y, cam_pos.z, 0.0];
        self.queue
            .write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&self.camera_ubo));

        // Cascade computation uses camera-relative position (ZERO when active).
        #[cfg(feature = "camera-relative")]
        {
            let cr_cam = Camera {
                position: glam::Vec3::ZERO,
                ..*camera
            };
            self.update_cascade_splits(&cr_cam, light_dir);
        }
        #[cfg(not(feature = "camera-relative"))]
        self.update_cascade_splits(camera, light_dir);
    }

    fn update_cascade_splits(&mut self, camera: &Camera, light_dir: glam::Vec3) {
        // Shadow distance: cap at a reasonable range regardless of the camera's
        // far plane (which can be 50 km for skybox visibility).  Shadows beyond
        // ~500 units are imperceptible and waste shadow-map resolution.
        let n = camera.znear.max(0.5);
        let shadow_far = 500.0_f32.min(camera.zfar);

        // PSSM practical split (lambda blends log vs linear).
        // With n=0.5, shadow_far=500, lambda=0.75:
        //   split0 ≈ 16 units — close-up detail
        //   split1 = 500      — landscape shadows
        let c = 2.0;
        let i = 1.0f32;
        let u = n + (shadow_far - n) * (i / c);
        let l = n * (shadow_far / n).powf(i / c);
        let lambda = self.cascade_lambda.clamp(0.0, 1.0);
        let split = l * lambda + u * (1.0 - lambda);
        self.split0 = split;
        self.split1 = shadow_far;

        let frustum0 = frustum_corners_ws(camera, n, self.split0);
        let frustum1 = frustum_corners_ws(camera, self.split0, shadow_far);
        let up = glam::Vec3::Y;
        let center0 = frustum_center(&frustum0);
        let center1 = frustum_center(&frustum1);

        // Use sphere-based fitting for stable shadows (rotation-invariant).
        // The sphere radius determines the ortho projection extent, so shadows
        // don't shimmer when the camera rotates.
        let radius0 = sphere_radius(&frustum0, center0);
        let radius1 = sphere_radius(&frustum1, center1);

        // Position the light far enough behind the sphere to capture tall
        // shadow casters (trees, buildings) above the frustum.
        let light_dist0 = radius0 * 2.0 + 50.0;
        let light_dist1 = radius1 * 2.0 + 50.0;

        let view0 = glam::Mat4::look_to_rh(center0 - light_dir * light_dist0, light_dir, up);
        let view1 = glam::Mat4::look_to_rh(center1 - light_dir * light_dist1, light_dir, up);

        // Sphere-based orthographic bounds (rotationally stable)
        let margin = 2.0_f32;
        let proj0 = glam::Mat4::orthographic_rh(
            -radius0 - margin,
            radius0 + margin,
            -radius0 - margin,
            radius0 + margin,
            0.1,
            light_dist0 + radius0 + margin,
        );
        let proj1 = glam::Mat4::orthographic_rh(
            -radius1 - margin,
            radius1 + margin,
            -radius1 - margin,
            radius1 + margin,
            0.1,
            light_dist1 + radius1 + margin,
        );
        self.cascade0 = proj0 * view0;
        self.cascade1 = proj1 * view1;

        self.queue.write_buffer(
            &self.shadow_cascade_bufs[0],
            0,
            bytemuck::cast_slice(&self.cascade0.to_cols_array()),
        );
        self.queue.write_buffer(
            &self.shadow_cascade_bufs[1],
            0,
            bytemuck::cast_slice(&self.cascade1.to_cols_array()),
        );

        let mut data: Vec<f32> = Vec::with_capacity(36);
        data.extend_from_slice(&self.cascade0.to_cols_array());
        data.extend_from_slice(&self.cascade1.to_cols_array());
        data.push(self.split0);
        data.push(self.split1);
        let extras_x = if self.force_shadow_override {
            -1.0
        } else {
            self.shadow_pcf_radius_px
        };
        data.push(extras_x);
        data.push(self.shadow_depth_bias);
        self.queue
            .write_buffer(&self.light_buf, 0, bytemuck::cast_slice(&data));
    }

    // --- CSM Tuning API ---
    pub fn set_cascade_splits(&mut self, split0: f32, split1: f32) {
        self.split0 = split0.max(0.01);
        self.split1 = split1.max(self.split0 + 0.01);
    }
    pub fn set_cascade_extents(&mut self, extent0: f32, extent1: f32) {
        self.cascade0_extent = extent0.max(1.0);
        self.cascade1_extent = extent1.max(self.cascade0_extent + 1.0);
    }

    /// Controls the split distribution between uniform (0) and logarithmic (1)
    pub fn set_cascade_lambda(&mut self, lambda: f32) {
        self.cascade_lambda = lambda.clamp(0.0, 1.0);
    }

    /// Sets shadow filtering and bias values. radius is in texels for 3x3 PCF when >=1.
    pub fn set_shadow_filter(&mut self, radius_px: f32, depth_bias: f32, slope_scale: f32) {
        self.shadow_pcf_radius_px = radius_px.max(0.0);
        self.shadow_depth_bias = depth_bias.max(0.0);
        self.shadow_slope_scale = slope_scale.max(0.0);
    }

    pub fn set_material_params(&mut self, base_color: [f32; 4], metallic: f32, roughness: f32) {
        self.set_material_params_full(base_color, metallic, roughness, 0.5);
    }

    /// Set material parameters including per-material alpha cutoff threshold.
    pub fn set_material_params_full(
        &mut self,
        base_color: [f32; 4],
        metallic: f32,
        roughness: f32,
        alpha_cutoff: f32,
    ) {
        let ubo =
            crate::material::MaterialUboData::new(base_color, metallic, roughness, alpha_cutoff);
        self.queue
            .write_buffer(&self.material_buf, 0, bytemuck::bytes_of(&ubo));
    }

    pub fn create_mesh_from_arrays(
        &self,
        vertices: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: &[u32],
    ) -> Mesh {
        // Interleave into Vertex, derive simple defaults for tangent (+X) and uv (planar XZ)
        let verts: Vec<crate::types::Vertex> = vertices
            .iter()
            .zip(normals.iter())
            .map(|(p, n)| crate::types::Vertex {
                position: *p,
                normal: *n,
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [p[0] * 0.5 + 0.5, p[2] * 0.5 + 0.5],
            })
            .collect();
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ext v"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ext i"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        Mesh {
            vertex_buf: vbuf,
            index_buf: ibuf,
            index_count: indices.len() as u32,
        }
    }

    pub fn create_mesh_from_full_arrays(
        &self,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        tangents: &[[f32; 4]],
        uvs: &[[f32; 2]],
        indices: &[u32],
    ) -> Mesh {
        assert!(
            positions.len() == normals.len()
                && positions.len() == tangents.len()
                && positions.len() == uvs.len()
        );

        let vertex_size = std::mem::size_of::<crate::types::Vertex>();
        let vbuf_bytes = vertex_size * positions.len();
        let ibuf_bytes = std::mem::size_of_val(indices);
        let max_buf = self.device.limits().max_buffer_size as usize;

        if vbuf_bytes > max_buf || ibuf_bytes > max_buf {
            log::error!(
                "Mesh buffer exceeds wgpu max_buffer_size ({max_buf} bytes): \
                 vertex_buf={vbuf_bytes} ({} verts), index_buf={ibuf_bytes} ({} idx). \
                 Returning empty placeholder.",
                positions.len(),
                indices.len(),
            );
            let placeholder_verts = [crate::types::Vertex {
                position: [0.0; 3],
                normal: [0.0, 1.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0],
                uv: [0.0; 2],
            }; 3];
            let placeholder_idx: [u32; 3] = [0, 1, 2];
            let vbuf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ext v (full) placeholder"),
                    contents: bytemuck::cast_slice(&placeholder_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            let ibuf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ext i (full) placeholder"),
                    contents: bytemuck::cast_slice(&placeholder_idx),
                    usage: wgpu::BufferUsages::INDEX,
                });
            return Mesh {
                vertex_buf: vbuf,
                index_buf: ibuf,
                index_count: 3,
            };
        }

        let verts: Vec<crate::types::Vertex> = positions
            .iter()
            .zip(normals.iter())
            .zip(tangents.iter())
            .zip(uvs.iter())
            .map(|(((p, n), t), uv)| crate::types::Vertex {
                position: *p,
                normal: *n,
                tangent: *t,
                uv: *uv,
            })
            .collect();
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ext v (full)"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ext i (full)"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        Mesh {
            vertex_buf: vbuf,
            index_buf: ibuf,
            index_count: indices.len() as u32,
        }
    }

    pub fn create_mesh_from_cpu_mesh(&self, cpu_mesh: &crate::mesh::CpuMesh) -> Mesh {
        let positions: Vec<_> = cpu_mesh.vertices.iter().map(|v| v.position).collect();
        let normals: Vec<_> = cpu_mesh.vertices.iter().map(|v| v.normal).collect();
        let tangents: Vec<_> = cpu_mesh.vertices.iter().map(|v| v.tangent).collect();
        let uvs: Vec<_> = cpu_mesh.vertices.iter().map(|v| v.uv).collect();
        self.create_mesh_from_full_arrays(&positions, &normals, &tangents, &uvs, &cpu_mesh.indices)
    }

    pub fn set_external_mesh(&mut self, mesh: Mesh) {
        self.mesh_external = Some(mesh);
    }

    pub fn update_instances(&mut self, instances: &[Instance]) {
        self.instances.clear();
        self.instances.extend_from_slice(instances);

        // When camera-relative, offset every model matrix so its translation
        // is relative to the camera origin.  The f64 subtraction preserves
        // sub-centimetre precision even at 20 000+ km from world origin.
        #[cfg(feature = "camera-relative")]
        let raws: Vec<InstanceRaw> = {
            let cam = self.camera_world_pos;
            self.instances
                .iter()
                .map(|inst| {
                    let mut model = inst.transform;
                    let wp = glam::DVec3::new(
                        model.w_axis.x as f64,
                        model.w_axis.y as f64,
                        model.w_axis.z as f64,
                    );
                    let rel = (wp - cam).as_vec3();
                    model.w_axis = glam::Vec4::new(rel.x, rel.y, rel.z, model.w_axis.w);
                    Instance {
                        transform: model,
                        color: inst.color,
                        material_id: inst.material_id,
                    }
                    .raw()
                })
                .collect()
        };
        #[cfg(not(feature = "camera-relative"))]
        let raws: Vec<InstanceRaw> = self.instances.iter().map(|i| i.raw()).collect();

        let size = (raws.len() * std::mem::size_of::<InstanceRaw>()) as u64;

        if size > self.instance_buf.size() {
            self.instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instance buf (resized)"),
                size: size.next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
        }
        self.queue
            .write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&raws));
    }

    /// Reads back the instance buffer from the GPU.
    /// This is intended for testing and validation.
    #[cfg(test)]
    pub async fn read_instance_buffer(&self) -> Vec<crate::types::InstanceRaw> {
        let size = self.instance_buf.size();
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance staging"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("instance read encoder"),
            });
        encoder.copy_buffer_to_buffer(&self.instance_buf, 0, &staging, 0, size);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            tx.send(res).unwrap();
        });
        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        // We only want the part that contains actual instances
        let count = self.instances.len();
        let byte_len = count * std::mem::size_of::<crate::types::InstanceRaw>();
        let result = bytemuck::cast_slice(&data[..byte_len]).to_vec();
        drop(data);
        staging.unmap();
        result
    }

    pub fn set_weather(&mut self, kind: crate::effects::WeatherKind) {
        self.weather.set_kind(kind);
        // Bridge: update scene environment fog/ambient from weather kind
        self.scene_env.apply_weather(kind);
    }

    pub fn tick_weather(&mut self, dt: f32) {
        let cam_pos = glam::Vec3::new(
            self.camera_ubo.camera_pos_pad[0],
            self.camera_ubo.camera_pos_pad[1],
            self.camera_ubo.camera_pos_pad[2],
        );
        self.weather.update(&self.queue, dt, cam_pos);
    }

    /// Mutable access to the weather particle system (e.g. to set wind).
    pub fn weather_mut(&mut self) -> &mut crate::effects::WeatherFx {
        &mut self.weather
    }

    /// Set the maximum weather particle count, reallocating the GPU buffer if needed.
    pub fn set_weather_max(&mut self, max: usize) {
        self.weather.set_max(&self.device, max);
    }

    pub fn tick_environment(&mut self, dt: f32) {
        // Advance time-of-day; derive sky params
        self.sky.update(dt);
        // Bridge: feed time-of-day ambient into scene environment
        let tod = self.sky.time_of_day().clone();
        self.scene_env.apply_time_of_day(&tod);
    }

    /// Get immutable reference to time-of-day system
    pub fn time_of_day(&self) -> &crate::environment::TimeOfDay {
        self.sky.time_of_day()
    }

    pub fn time_of_day_mut(&mut self) -> &mut crate::environment::TimeOfDay {
        self.sky.time_of_day_mut()
    }

    pub fn sky_config(&self) -> crate::environment::SkyConfig {
        self.sky.config().clone()
    }

    pub fn set_sky_config(&mut self, cfg: crate::environment::SkyConfig) {
        self.sky.set_config(cfg);
    }

    /// Check if shadows are enabled
    pub fn shadows_enabled(&self) -> bool {
        self.shadows_enabled
    }

    /// Enable or disable shadow rendering
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.shadows_enabled = enabled;
        // When shadows are disabled, set the force_shadow_override sentinel so
        // the PBR shader skips PCF sampling entirely (extras.x = -1.0 → shader
        // guard forces shadow = 1.0).  Without this, every fragment still
        // executes 9 textureSampleCompare calls against the uncleared shadow
        // map, wasting ~50% of per-fragment texture bandwidth AND producing
        // grain from the undefined depth values.
        self.force_shadow_override = !enabled;
    }

    /// Enable or disable the per-frame cloud shadow compute pass.
    /// When disabled the cloud shadow texture retains its initial all-white
    /// state (1.0 = fully lit), so direct sunlight is preserved without
    /// the noisy pattern from the low-resolution 512x512 transmittance map.
    pub fn set_cloud_shadows_enabled(&mut self, enabled: bool) {
        self.cloud_shadows_enabled = enabled;
    }

    /// Set the post-processing chain configuration.
    ///
    /// Controls which post-processing effects are active (SSAO, SSR, Bloom, TAA,
    /// DoF, Motion Blur, Color Grading) and the tonemapping operator.
    /// Disabled effects are skipped entirely with zero GPU cost.
    pub fn set_post_process_chain(&mut self, chain: crate::hdr_pipeline::PostProcessChain) {
        self.post_chain = chain;
    }

    /// Get the current post-processing chain configuration.
    pub fn post_process_chain(&self) -> &crate::hdr_pipeline::PostProcessChain {
        &self.post_chain
    }

    /// Update the bloom pass configuration (threshold, intensity, etc.).
    ///
    /// If no bloom pass exists yet (lazy creation), the config is stored and
    /// applied on the next frame when the pass is created.
    pub fn set_bloom_config(&mut self, config: crate::bloom::BloomConfig) {
        if let Some(bloom) = self.bloom_pass.as_mut() {
            bloom.set_config(config);
        } else {
            // Store for lazy init — next draw_into() will create the pass
            // and it will pick up `post_chain.bloom_enabled` to decide whether
            // to instantiate. We cache the config on a temporary field so it
            // can be applied after creation.
            self.pending_bloom_config = Some(config);
        }
    }

    /// Get a shared reference to the GPU memory budget tracker.
    pub fn gpu_memory_budget(&self) -> &std::sync::Arc<crate::gpu_memory::GpuMemoryBudget> {
        &self.gpu_memory_budget
    }

    /// Set the water renderer for ocean rendering
    pub fn set_water_renderer(&mut self, water: crate::water::WaterRenderer) {
        self.water_renderer = Some(water);
    }

    /// Remove the water renderer, disabling water rendering.
    pub fn clear_water_renderer(&mut self) {
        self.water_renderer = None;
    }

    /// Update water renderer state (call each frame before render)
    pub fn update_water(&mut self, view_proj: glam::Mat4, camera_pos: glam::Vec3, time: f32) {
        if let Some(ref mut water) = self.water_renderer {
            water.update(&self.queue, view_proj, camera_pos, time);
        }
    }

    /// Acquire the current surface texture with robust error handling.
    ///
    /// Returns `Ok(None)` when no surface is configured or if the surface was
    /// lost (after reconfiguration). Returns `Err` on OutOfMemory or other
    /// fatal errors.
    fn acquire_surface_texture(&self) -> Result<Option<(wgpu::SurfaceTexture, wgpu::TextureView)>> {
        if self.is_device_lost() {
            return Err(anyhow::anyhow!(
                "GPU device lost — renderer must be recreated"
            ));
        }

        let surface = if let Some(s) = &self.surface {
            s
        } else {
            return Ok(None);
        };

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost) => {
                surface.configure(&self.device, &self.config);
                return Ok(None);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(anyhow::anyhow!("Swapchain OutOfMemory"));
            }
            Err(e) => return Err(e.into()),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Some((frame, view)))
    }

    pub fn render(&mut self) -> Result<()> {
        let (frame, view) = match self.acquire_surface_texture()? {
            Some(pair) => pair,
            None => return Ok(()),
        };

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        // --- GPU timestamp profiling: poll previous frame, begin new frame, pre-allocate ---
        if let Some(ref mut p) = self.gpu_profiler {
            p.poll_readback(&self.device);
            p.begin_frame();
        }
        // Advance the staging ring buffer for this frame.
        self.staging_ring.begin_frame();

        let ts_cluster = self
            .gpu_profiler
            .as_mut()
            .and_then(|p| p.allocate_pass("cluster_bin"));
        let ts_shadow = [
            self.gpu_profiler
                .as_mut()
                .and_then(|p| p.allocate_pass("shadow_cascade_0")),
            self.gpu_profiler
                .as_mut()
                .and_then(|p| p.allocate_pass("shadow_cascade_1")),
        ];
        let ts_main = self
            .gpu_profiler
            .as_mut()
            .and_then(|p| p.allocate_pass("main_pass"));
        let ts_post = self
            .gpu_profiler
            .as_mut()
            .and_then(|p| p.allocate_pass("post_pass"));

        // Update plane buffer (DISABLE to fix interference with TerrainSystem)
        /*
        let plane_xform = glam::Mat4::from_translation(glam::vec3(0.0, -0.2, 0.0))
            * glam::Mat4::from_scale(glam::vec3(50.0, 1.0, 50.0));
        let plane_inst = Instance {
            transform: plane_xform,
            color: [0.1, 0.12, 0.14, 1.0],
            material_id: 0,
        }
        .raw();
        self.queue
            .write_buffer(&self.plane_inst_buf, 0, bytemuck::bytes_of(&plane_inst));
        */

        // Render sky first into HDR
        // TODO: Replace with the correct color target view for sky rendering (e.g., main color target or postprocess output)
        // self.sky.render(&mut enc, &self.main_color_view, &self.depth.view, Mat4::from_cols_array_2d(&self.camera_ubo.view_proj), &self.queue)?;

        if !self.point_lights.is_empty() {
            // CPU pre-pass builds offsets array (exclusive scan) we share to GPU
            let (_counts_cpu, _indices_cpu, offsets_cpu) = bin_lights_cpu(
                &self.point_lights,
                self.clustered_dims,
                (self.config.width, self.config.height),
                0.1,
                200.0,
                std::f32::consts::FRAC_PI_3,
            );
            // Upload lights and offsets; zero counts and indices
            #[repr(C)]
            #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
            struct GpuLight {
                pos_radius: [f32; 4],
            }
            let glights: Vec<GpuLight> = self
                .point_lights
                .iter()
                .map(|l| {
                    #[cfg(feature = "camera-relative")]
                    let pos = {
                        let wp = glam::DVec3::new(l.pos.x as f64, l.pos.y as f64, l.pos.z as f64);
                        (wp - self.camera_world_pos).as_vec3()
                    };
                    #[cfg(not(feature = "camera-relative"))]
                    let pos = l.pos;
                    GpuLight {
                        pos_radius: [pos.x, pos.y, pos.z, l.radius],
                    }
                })
                .collect();
            if !glights.is_empty() {
                self.queue.write_buffer(
                    &self.clustered_lights_buf,
                    0,
                    bytemuck::cast_slice(&glights),
                );
            }
            self.queue.write_buffer(
                &self.clustered_offsets_buf,
                0,
                bytemuck::cast_slice(&offsets_cpu),
            );
            // Zero counts — GPU-side clear, no CPU allocation
            enc.clear_buffer(&self.clustered_counts_buf, 0, None);
            // Run compute to fill counts/indices
            #[cfg(feature = "gpu-tests")]
            {
                enc.write_timestamp(&self.timestamp_query_set, 0);
            }
            let cluster_ts = ts_cluster.and_then(|(b, e)| {
                Some(wgpu::ComputePassTimestampWrites {
                    query_set: self.gpu_profiler.as_ref()?.query_set(),
                    beginning_of_pass_write_index: Some(b),
                    end_of_pass_write_index: Some(e),
                })
            });
            let mut cpass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cluster bin"),
                timestamp_writes: cluster_ts,
            });
            cpass.set_pipeline(&self.clustered_comp_pipeline);
            cpass.set_bind_group(0, &self.clustered_comp_bg, &[]);
            cpass.dispatch_workgroups(glights.len() as u32, 1, 1);
            drop(cpass);
            #[cfg(feature = "gpu-tests")]
            {
                enc.write_timestamp(&self.timestamp_query_set, 1);
                enc.resolve_query_set(&self.timestamp_query_set, 0..2, &self.timestamp_buf, 0);
            }
        }
        // Update external mesh single-instance buffer if needed
        if let Some(buf) = &self.ext_inst_buf {
            let inst = Instance {
                transform: glam::Mat4::IDENTITY,
                color: [1.0, 1.0, 1.0, 1.0],
                material_id: 0,
            }
            .raw();
            self.queue.write_buffer(buf, 0, bytemuck::bytes_of(&inst));
        }
        // Frustum cull instances
        let (vis_raws, vis_count) = self.build_visible_instances();
        if vis_count > 0 {
            self.queue
                .write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&vis_raws));
        }
        // Shadow passes (depth only) — skip when no shadow-casting geometry.
        let has_shadow_casters_r =
            vis_count > 0 || self.mesh_external.is_some() || !self.models.is_empty();

        for (idx, layer_view) in [&self.shadow_layer0_view, &self.shadow_layer1_view]
            .iter()
            .enumerate()
        {
            if !has_shadow_casters_r {
                continue;
            }
            let shadow_ts = ts_shadow[idx].and_then(|(b, e)| {
                Some(wgpu::RenderPassTimestampWrites {
                    query_set: self.gpu_profiler.as_ref()?.query_set(),
                    beginning_of_pass_write_index: Some(b),
                    end_of_pass_write_index: Some(e),
                })
            });
            let mut sp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: layer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: shadow_ts,
                occlusion_query_set: None,
            });
            sp.set_pipeline(&self.shadow_pipeline);
            sp.set_bind_group(0, &self.shadow_cascade_bgs[idx], &[]);
            // Draw plane
            sp.set_vertex_buffer(0, self.mesh_plane.vertex_buf.slice(..));
            sp.set_index_buffer(
                self.mesh_plane.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            sp.set_vertex_buffer(1, self.plane_inst_buf.slice(..));
            sp.draw_indexed(0..self.mesh_plane.index_count, 0, 0..1);
            // Draw tokens as spheres in shadow pass
            sp.set_vertex_buffer(0, self.mesh_sphere.vertex_buf.slice(..));
            sp.set_index_buffer(
                self.mesh_sphere.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            sp.set_vertex_buffer(1, self.instance_buf.slice(..));
            let inst_count = vis_count as u32;
            if inst_count > 0 {
                sp.draw_indexed(0..self.mesh_sphere.index_count, 0, 0..inst_count);
            }
            // External mesh (use ext_inst_count for consistency with main pass)
            if let (Some(mesh), Some(ibuf)) = (&self.mesh_external, &self.ext_inst_buf) {
                sp.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                sp.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                sp.set_vertex_buffer(1, ibuf.slice(..));
                if self.ext_inst_count > 0 {
                    sp.draw_indexed(0..mesh.index_count, 0, 0..self.ext_inst_count);
                }
            }
            // Named models cast shadows — scatter skipped (negligible
            // visual contribution), terrain only in cascade 0.
            // Shadow frustum culling: skip models outside the cascade's
            // ortho frustum to avoid rendering distant terrain into shadows.
            let cascade_vp = if idx == 0 {
                self.cascade0
            } else {
                self.cascade1
            };
            let shadow_frustum = crate::culling::FrustumPlanes::from_view_proj(&cascade_vp);
            for (name, model) in &self.models {
                if model.instance_count == 0 {
                    continue;
                }
                if name.starts_with("scatter_") {
                    continue;
                }
                if idx == 1 && name.starts_with("terrain_c") {
                    continue;
                }
                if let Some((aabb_min, aabb_max)) = &model.aabb {
                    let center = glam::Vec3::new(
                        (aabb_min[0] + aabb_max[0]) * 0.5,
                        (aabb_min[1] + aabb_max[1]) * 0.5,
                        (aabb_min[2] + aabb_max[2]) * 0.5,
                    );
                    let extent = glam::Vec3::new(
                        (aabb_max[0] - aabb_min[0]) * 0.5,
                        (aabb_max[1] - aabb_min[1]) * 0.5,
                        (aabb_max[2] - aabb_min[2]) * 0.5,
                    );
                    if !shadow_frustum.test_aabb(center, extent) {
                        continue;
                    }
                }
                sp.set_vertex_buffer(0, model.mesh.vertex_buf.slice(..));
                sp.set_index_buffer(model.mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                sp.set_vertex_buffer(1, model.instance_buf.slice(..));
                sp.draw_indexed(0..model.mesh.index_count, 0, 0..model.instance_count);
            }
        }
        // light_buf already contains the full [cascade0, cascade1, splits, extras]
        // data from update_camera(); no restore needed since shadow passes now use
        // their own per-cascade buffers (shadow_cascade_bufs).

        // Render sky first into HDR target so we can layer geometry on top
        // Construct rotation-only VP for skybox (aligned with draw_into path)
        let mut vp_sky = self.cached_view;
        vp_sky.w_axis.x = 0.0;
        vp_sky.w_axis.y = 0.0;
        vp_sky.w_axis.z = 0.0;
        vp_sky = self.cached_proj * vp_sky;

        let sky_tex = self.ibl_resources.as_ref().map(|r| &r.env_cube);
        self.sky.render(
            &self.device,
            &mut enc,
            &self.hdr_view,
            &self.depth.view,
            vp_sky,
            &self.queue,
            sky_tex,
            self.ibl_resources
                .as_ref()
                .and_then(|r| r.hdr_equirect.as_ref()),
        )?;

        // Upload scene environment UBO (fog, ambient, tint) for this frame
        {
            let scene_ubo = self.scene_env.to_ubo();
            self.queue
                .write_buffer(&self.scene_env_buf, 0, bytemuck::bytes_of(&scene_ubo));
        }

        // Update cloud shadow map for terrain/PBR sampling.
        // Skipped when cloud_shadows_enabled is false — the 512×512 transmittance
        // map produces a noisy pattern on terrain at this resolution.
        if self.cloud_shadows_enabled {
            let cam_pos = glam::Vec3::new(
                self.camera_ubo.camera_pos_pad[0],
                self.camera_ubo.camera_pos_pad[1],
                self.camera_ubo.camera_pos_pad[2],
            );
            let sun_dir = -glam::Vec3::new(
                self.camera_ubo.light_dir_pad[0],
                self.camera_ubo.light_dir_pad[1],
                self.camera_ubo.light_dir_pad[2],
            )
            .normalize_or_zero();
            let cloud_cfg = crate::volumetric_clouds::CloudConfig::default();
            self.cloud_shadow_pass.prepare_frame(
                &self.queue,
                cam_pos,
                sun_dir,
                &cloud_cfg,
                cloud_cfg.extinction_coeff,
                32,
                1.0 / 60.0,
            );
            self.cloud_shadow_pass
                .execute(&self.device, &mut enc, self.resource_generation);
        }

        {
            let main_ts = ts_main.and_then(|(b, e)| {
                Some(wgpu::RenderPassTimestampWrites {
                    query_set: self.gpu_profiler.as_ref()?.query_set(),
                    beginning_of_pass_write_index: Some(b),
                    end_of_pass_write_index: Some(e),
                })
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main pass"),
                // Render the main scene into the HDR color target; a post-pass will tonemap to the surface.
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_view,
                    resolve_target: None,
                    // Preserve sky color drawn earlier
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Preserve sky depth (aligned with draw_into)
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: main_ts,
                occlusion_query_set: None,
            });

            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &self.camera_bind_group, &[]);
            rp.set_bind_group(1, &self.material_bg, &[]);
            rp.set_bind_group(2, &self.light_bg, &[]);
            rp.set_bind_group(3, &self.tex_bg, &[]);
            rp.set_bind_group(4, &self.scene_env_bg, &[]);
            rp.set_bind_group(5, &self.ibl_bind_group, &[]);

            // Ground plane (scaled) - DISABLED (Interferes with Terrain)
            /*
            rp.set_vertex_buffer(0, self.mesh_plane.vertex_buf.slice(..));
            rp.set_index_buffer(
                self.mesh_plane.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rp.set_vertex_buffer(1, self.plane_inst_buf.slice(..));
            rp.draw_indexed(0..self.mesh_plane.index_count, 0, 0..1);
            */

            // Tokens as lit spheres (instances)
            rp.set_vertex_buffer(0, self.mesh_sphere.vertex_buf.slice(..));
            rp.set_index_buffer(
                self.mesh_sphere.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rp.set_vertex_buffer(1, self.instance_buf.slice(..));
            let inst_count = vis_count as u32;
            if inst_count > 0 {
                rp.draw_indexed(0..self.mesh_sphere.index_count, 0, 0..inst_count);
            }

            // External mesh if present (aligned with draw_into: use ext_inst_count)
            if let (Some(mesh), Some(ibuf)) = (&self.mesh_external, &self.ext_inst_buf) {
                rp.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                rp.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                rp.set_vertex_buffer(1, ibuf.slice(..));
                if self.ext_inst_count > 0 {
                    rp.draw_indexed(0..mesh.index_count, 0, 0..self.ext_inst_count);
                }
            }

            // Render all named models (terrain, trees, rocks, etc.) with frustum culling
            {
                let vp = self.cached_proj * self.cached_view;
                let frustum = crate::culling::FrustumPlanes::from_view_proj(&vp);
                for model in self.models.values() {
                    if model.instance_count == 0 {
                        continue;
                    }
                    if let Some((aabb_min, aabb_max)) = &model.aabb {
                        let center = glam::Vec3::new(
                            (aabb_min[0] + aabb_max[0]) * 0.5,
                            (aabb_min[1] + aabb_max[1]) * 0.5,
                            (aabb_min[2] + aabb_max[2]) * 0.5,
                        );
                        let extent = glam::Vec3::new(
                            (aabb_max[0] - aabb_min[0]) * 0.5,
                            (aabb_max[1] - aabb_min[1]) * 0.5,
                            (aabb_max[2] - aabb_min[2]) * 0.5,
                        );
                        if !frustum.test_aabb(center, extent) {
                            continue;
                        }
                    }
                    // Bind per-model texture if available, otherwise use global fallback
                    // to prevent bind group bleed from the previous model.
                    if let Some(ref mtex) = model.tex_bind_group {
                        rp.set_bind_group(3, mtex, &[]);
                    } else {
                        rp.set_bind_group(3, &self.tex_bg, &[]);
                    }
                    rp.set_vertex_buffer(0, model.mesh.vertex_buf.slice(..));
                    rp.set_index_buffer(model.mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                    rp.set_vertex_buffer(1, model.instance_buf.slice(..));
                    rp.draw_indexed(0..model.mesh.index_count, 0, 0..model.instance_count);
                }
            }

            // Render water (transparent, after all opaque objects)
            if let Some(ref water) = self.water_renderer {
                water.render(&mut rp);
            }
        }

        // Optional feature-gated post chain — gated by post_chain flags
        #[cfg(feature = "postfx")]
        {
            // NOTE: SSR and SSAO passes are disabled because the current
            // implementation reuses `post_pipeline` (tonemap shader) and
            // `post_bind_group` which reads from `hdr_view`.  Writing back
            // to `hdr_view` with a tonemap pass causes double-tonemapping
            // (crushing contrast, dark muddy output).  These should be
            // re-enabled only when proper SSR/SSAO compute shaders and
            // dedicated render targets are implemented.
        }

        // Postprocess HDR to surface
        {
            let post_ts = ts_post.and_then(|(b, e)| {
                Some(wgpu::RenderPassTimestampWrites {
                    query_set: self.gpu_profiler.as_ref()?.query_set(),
                    beginning_of_pass_write_index: Some(b),
                    end_of_pass_write_index: Some(e),
                })
            });
            let mut pp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: post_ts,
                occlusion_query_set: None,
            });
            #[cfg(feature = "postfx")]
            {
                pp.set_pipeline(&self.post_pipeline);
                pp.set_bind_group(0, &self.post_bind_group, &[]);
                pp.set_bind_group(1, &self.scene_env_bg, &[]);
            }
            #[cfg(not(feature = "postfx"))]
            {
                pp.set_pipeline(&self.post_pipeline);
                pp.set_bind_group(0, &self.post_bind_group, &[]);
                pp.set_bind_group(1, &self.scene_env_bg, &[]);
            }
            pp.draw(0..3, 0..1);
        }

        // --- GPU profiler: resolve timestamp queries into readback buffer ---
        if let Some(ref p) = self.gpu_profiler {
            p.end_frame(&mut enc);
        }

        self.queue.submit(Some(enc.finish()));
        frame.present();

        // --- GPU profiler: initiate async readback of timestamp data ---
        if let Some(ref mut p) = self.gpu_profiler {
            p.request_readback();
        }

        Ok(())
    }

    pub fn draw_into(
        &mut self,
        view: &wgpu::TextureView,
        enc: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        // --- GPU timestamp profiling: poll previous frame, begin new frame, pre-allocate ---
        if let Some(ref mut p) = self.gpu_profiler {
            p.poll_readback(&self.device);
            p.begin_frame();
        }
        // Advance the staging ring buffer for this frame.
        self.staging_ring.begin_frame();

        let di_ts_cluster = self
            .gpu_profiler
            .as_mut()
            .and_then(|p| p.allocate_pass("cluster_bin"));
        let di_ts_shadow = [
            self.gpu_profiler
                .as_mut()
                .and_then(|p| p.allocate_pass("shadow_cascade_0")),
            self.gpu_profiler
                .as_mut()
                .and_then(|p| p.allocate_pass("shadow_cascade_1")),
        ];
        let di_ts_main = self
            .gpu_profiler
            .as_mut()
            .and_then(|p| p.allocate_pass("main_render"));
        let di_ts_post = self
            .gpu_profiler
            .as_mut()
            .and_then(|p| p.allocate_pass("post_process"));

        // Clustered lighting setup — only recompute when lights change.
        // The default two hardcoded lights never change, so after the first
        // frame the cached offsets are reused (saves ~0.5-1ms CPU per frame).
        if self.point_lights.is_empty() {
            self.point_lights.push(CpuLight {
                pos: glam::Vec3::new(2.0, 2.0, 3.0),
                radius: 6.0,
            });
            self.point_lights.push(CpuLight {
                pos: glam::Vec3::new(-3.0, 1.0, 8.0),
                radius: 5.0,
            });
            // Invalidate cache when lights are first added
            self.clustered_offsets_cache = None;
        }

        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct GpuLight {
            pos_radius: [f32; 4],
        }

        let light_count = self.point_lights.len();

        if self.clustered_offsets_cache.is_none() {
            // Full recomputation: bin lights + upload everything
            let (_counts_cpu, _indices_cpu, offsets_cpu) = bin_lights_cpu(
                &self.point_lights,
                self.clustered_dims,
                (self.config.width, self.config.height),
                0.1,
                200.0,
                std::f32::consts::FRAC_PI_3,
            );
            let glights: Vec<GpuLight> = self
                .point_lights
                .iter()
                .map(|l| {
                    #[cfg(feature = "camera-relative")]
                    let pos = {
                        let wp = glam::DVec3::new(l.pos.x as f64, l.pos.y as f64, l.pos.z as f64);
                        (wp - self.camera_world_pos).as_vec3()
                    };
                    #[cfg(not(feature = "camera-relative"))]
                    let pos = l.pos;
                    GpuLight {
                        pos_radius: [pos.x, pos.y, pos.z, l.radius],
                    }
                })
                .collect();
            if !glights.is_empty() {
                self.queue.write_buffer(
                    &self.clustered_lights_buf,
                    0,
                    bytemuck::cast_slice(&glights),
                );
            }
            self.queue.write_buffer(
                &self.clustered_offsets_buf,
                0,
                bytemuck::cast_slice(&offsets_cpu),
            );
            self.clustered_offsets_cache = Some(offsets_cpu);

            // GPU compute pass only needed when lights changed (offsets were recomputed above)
            enc.clear_buffer(&self.clustered_counts_buf, 0, None);
            {
                let cluster_ts = di_ts_cluster.and_then(|(b, e)| {
                    Some(wgpu::ComputePassTimestampWrites {
                        query_set: self.gpu_profiler.as_ref()?.query_set(),
                        beginning_of_pass_write_index: Some(b),
                        end_of_pass_write_index: Some(e),
                    })
                });
                let mut cpass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("cluster bin"),
                    timestamp_writes: cluster_ts,
                });
                cpass.set_pipeline(&self.clustered_comp_pipeline);
                cpass.set_bind_group(0, &self.clustered_comp_bg, &[]);
                cpass.dispatch_workgroups(light_count as u32, 1, 1);
            }
        }

        // Update pre-allocated plane instance buffer ONLY if terrain hasn't
        // positioned it.  `set_terrain_ground_plane()` sets the correct Y and
        // extent for terrain rendering — overwriting it here would place a
        // small dark plane at Y=-0.2 right inside the camera's shadow cascade,
        // producing a massive shadow that follows the camera.
        if !self.terrain_ground_set {
            let plane_xform = glam::Mat4::from_translation(vec3(0.0, -0.2, 0.0))
                * glam::Mat4::from_scale(vec3(50.0, 1.0, 50.0));
            let plane_inst = Instance {
                transform: plane_xform,
                color: [0.1, 0.12, 0.14, 1.0],
                material_id: 0,
            }
            .raw();
            self.queue
                .write_buffer(&self.plane_inst_buf, 0, bytemuck::bytes_of(&plane_inst));
        }

        // Frustum cull - TEST 4
        let (vis_raws, vis_count) = self.build_visible_instances();
        if vis_count > 0 {
            self.queue
                .write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&vis_raws));
        }

        // Shadow passes — skip entirely when shadows disabled or no geometry visible.
        // Each shadow pass costs ~2-3ms even for an empty scene.
        let has_shadow_casters = self.shadows_enabled
            && (vis_count > 0 || self.mesh_external.is_some() || !self.models.is_empty());

        for (idx, layer_view) in [&self.shadow_layer0_view, &self.shadow_layer1_view]
            .iter()
            .enumerate()
        {
            if !has_shadow_casters {
                continue;
            }
            let shadow_ts = di_ts_shadow[idx].and_then(|(b, e)| {
                Some(wgpu::RenderPassTimestampWrites {
                    query_set: self.gpu_profiler.as_ref()?.query_set(),
                    beginning_of_pass_write_index: Some(b),
                    end_of_pass_write_index: Some(e),
                })
            });
            let mut sp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: layer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: shadow_ts,
                occlusion_query_set: None,
            });
            sp.set_pipeline(&self.shadow_pipeline);
            sp.set_bind_group(0, &self.shadow_cascade_bgs[idx], &[]);
            // Ground fill plane always renders in shadow pass
            sp.set_vertex_buffer(0, self.mesh_plane.vertex_buf.slice(..));
            sp.set_index_buffer(
                self.mesh_plane.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            sp.set_vertex_buffer(1, self.plane_inst_buf.slice(..));
            sp.draw_indexed(0..self.mesh_plane.index_count, 0, 0..1);
            // Draw sphere instances into shadow map (aligned with render() path)
            sp.set_vertex_buffer(0, self.mesh_sphere.vertex_buf.slice(..));
            sp.set_index_buffer(
                self.mesh_sphere.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            sp.set_vertex_buffer(1, self.instance_buf.slice(..));
            let inst_count = vis_count as u32;
            if inst_count > 0 {
                sp.draw_indexed(0..self.mesh_sphere.index_count, 0, 0..inst_count);
            }
            // External mesh shadow (aligned with render() path)
            if let (Some(mesh), Some(ibuf)) = (&self.mesh_external, &self.ext_inst_buf) {
                sp.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                sp.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                sp.set_vertex_buffer(1, ibuf.slice(..));
                if self.ext_inst_count > 0 {
                    sp.draw_indexed(0..mesh.index_count, 0, 0..self.ext_inst_count);
                }
            }
            // Named models cast shadows — but only terrain (terrain_c*) in
            // cascade 0 (near).  Scatter objects (scatter_*) are skipped
            // entirely because small vegetation/debris shadows are
            // imperceptible and the draw call cost is severe (~25+ calls ×
            // 2 cascades at 4M+ triangles total).
            //
            // Shadow frustum culling: build frustum planes from the cascade
            // VP matrix and skip terrain chunks outside the shadow camera's
            // view. This typically culls ~90% of terrain chunks since the
            // near cascade covers only ~16 units around the camera.
            let cascade_vp = if idx == 0 {
                self.cascade0
            } else {
                self.cascade1
            };
            let shadow_frustum = crate::culling::FrustumPlanes::from_view_proj(&cascade_vp);
            for (name, model) in &self.models {
                if model.instance_count == 0 {
                    continue;
                }
                // Skip scatter models from shadow pass — negligible visual
                // contribution, significant GPU cost.
                if name.starts_with("scatter_") {
                    continue;
                }
                // Cascade 1 (far) skips terrain — self-shadows on distant
                // terrain are invisible and cost millions of triangles.
                if idx == 1 && name.starts_with("terrain_c") {
                    continue;
                }
                // Frustum cull against the shadow cascade's ortho projection.
                // Terrain chunks outside the shadow camera's frustum don't
                // contribute to visible shadows and are expensive to render.
                if let Some((aabb_min, aabb_max)) = &model.aabb {
                    let center = glam::Vec3::new(
                        (aabb_min[0] + aabb_max[0]) * 0.5,
                        (aabb_min[1] + aabb_max[1]) * 0.5,
                        (aabb_min[2] + aabb_max[2]) * 0.5,
                    );
                    let extent = glam::Vec3::new(
                        (aabb_max[0] - aabb_min[0]) * 0.5,
                        (aabb_max[1] - aabb_min[1]) * 0.5,
                        (aabb_max[2] - aabb_min[2]) * 0.5,
                    );
                    if !shadow_frustum.test_aabb(center, extent) {
                        continue;
                    }
                }
                sp.set_vertex_buffer(0, model.mesh.vertex_buf.slice(..));
                sp.set_index_buffer(model.mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                sp.set_vertex_buffer(1, model.instance_buf.slice(..));
                sp.draw_indexed(0..model.mesh.index_count, 0, 0..model.instance_count);
            }
        }
        // light_buf already contains the full [cascade0, cascade1, splits, extras]
        // data from update_camera(); no restore needed since shadow passes now use
        // their own per-cascade buffers (shadow_cascade_bufs).

        // Render procedural skybox (time-of-day gradient)
        // Use view-only matrix (no translation) constructed on CPU for reliability
        // Sky pass (using rotation-only view-projection)
        // Note: Construct logic handles translation ensuring skybox center = camera
        let mut vp_sky = self.cached_view;
        vp_sky.w_axis.x = 0.0;
        vp_sky.w_axis.y = 0.0;
        vp_sky.w_axis.z = 0.0;
        vp_sky = self.cached_proj * vp_sky;

        let sky_tex = self.ibl_resources.as_ref().map(|r| &r.env_cube);

        self.sky
            .render(
                &self.device,
                enc,
                &self.hdr_view,
                &self.depth.view,
                vp_sky,
                &self.queue,
                sky_tex,
                self.ibl_resources
                    .as_ref()
                    .and_then(|r| r.hdr_equirect.as_ref()),
            )
            .context("Sky render failed")?;

        // Upload scene environment UBO (fog, ambient, tint) — aligned with render() path
        {
            let scene_ubo = self.scene_env.to_ubo();
            self.queue
                .write_buffer(&self.scene_env_buf, 0, bytemuck::bytes_of(&scene_ubo));
        }

        // Update cloud shadow map for terrain/PBR sampling.
        // Skipped when cloud_shadows_enabled is false — the 512×512 transmittance
        // map produces a noisy pattern on terrain at this resolution.
        if self.cloud_shadows_enabled {
            let cam_pos = glam::Vec3::new(
                self.camera_ubo.camera_pos_pad[0],
                self.camera_ubo.camera_pos_pad[1],
                self.camera_ubo.camera_pos_pad[2],
            );
            let sun_dir = -glam::Vec3::new(
                self.camera_ubo.light_dir_pad[0],
                self.camera_ubo.light_dir_pad[1],
                self.camera_ubo.light_dir_pad[2],
            )
            .normalize_or_zero();
            let cloud_cfg = crate::volumetric_clouds::CloudConfig::default();
            self.cloud_shadow_pass.prepare_frame(
                &self.queue,
                cam_pos,
                sun_dir,
                &cloud_cfg,
                cloud_cfg.extinction_coeff,
                32,
                1.0 / 60.0,
            );
            self.cloud_shadow_pass
                .execute(&self.device, enc, self.resource_generation);
        }

        {
            let main_ts = di_ts_main.and_then(|(b, e)| {
                Some(wgpu::RenderPassTimestampWrites {
                    query_set: self.gpu_profiler.as_ref()?.query_set(),
                    beginning_of_pass_write_index: Some(b),
                    end_of_pass_write_index: Some(e),
                })
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load sky result
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load sky depth (should be far plane)
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: main_ts,
                occlusion_query_set: None,
            });

            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &self.camera_bind_group, &[]);
            rp.set_bind_group(1, &self.material_bg, &[]);
            rp.set_bind_group(2, &self.light_bg, &[]);
            rp.set_bind_group(3, &self.tex_bg, &[]);
            rp.set_bind_group(4, &self.scene_env_bg, &[]);
            rp.set_bind_group(5, &self.ibl_bind_group, &[]);

            // Ground fill plane — always rendered.
            // When terrain is loaded, set_terrain_ground_plane() repositions it
            // below the terrain to block sky bleed-through.
            rp.set_vertex_buffer(0, self.mesh_plane.vertex_buf.slice(..));
            rp.set_index_buffer(
                self.mesh_plane.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rp.set_vertex_buffer(1, self.plane_inst_buf.slice(..));
            rp.draw_indexed(0..self.mesh_plane.index_count, 0, 0..1);

            // Tokens as spheres - TEST 6
            rp.set_vertex_buffer(0, self.mesh_sphere.vertex_buf.slice(..));
            rp.set_index_buffer(
                self.mesh_sphere.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rp.set_vertex_buffer(1, self.instance_buf.slice(..));
            let inst_count = vis_count as u32;
            if inst_count > 0 {
                rp.draw_indexed(0..self.mesh_sphere.index_count, 0, 0..inst_count);
            }

            // External mesh if present (e.g., GLB models)
            if let (Some(mesh), Some(ibuf)) = (&self.mesh_external, &self.ext_inst_buf) {
                rp.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                rp.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                rp.set_vertex_buffer(1, ibuf.slice(..));
                if self.ext_inst_count > 0 {
                    rp.draw_indexed(0..mesh.index_count, 0, 0..self.ext_inst_count);
                }
            }

            // Render all named models (terrain, trees, rocks, etc.) with frustum + distance culling
            {
                let vp = self.cached_proj * self.cached_view;
                let frustum = crate::culling::FrustumPlanes::from_view_proj(&vp);
                let cam_pos = glam::Vec3::new(
                    self.camera_ubo.camera_pos_pad[0],
                    self.camera_ubo.camera_pos_pad[1],
                    self.camera_ubo.camera_pos_pad[2],
                );
                // Max draw distance: use explicit limit if set, otherwise fall back
                // to fog_end * 1.2. Objects beyond this are fully fogged / invisible.
                let max_draw_dist = if self.max_draw_distance > 0.0 {
                    self.max_draw_distance
                } else {
                    self.scene_env.visuals.fog_end * 1.2
                };
                let max_draw_dist_sq = max_draw_dist * max_draw_dist;
                let mut drawn_models = 0u32;
                for model in self.models.values() {
                    if model.instance_count == 0 {
                        continue;
                    }
                    // Skip models whose AABB is entirely outside the frustum or beyond draw distance
                    if let Some((aabb_min, aabb_max)) = &model.aabb {
                        let center = glam::Vec3::new(
                            (aabb_min[0] + aabb_max[0]) * 0.5,
                            (aabb_min[1] + aabb_max[1]) * 0.5,
                            (aabb_min[2] + aabb_max[2]) * 0.5,
                        );
                        let extent = glam::Vec3::new(
                            (aabb_max[0] - aabb_min[0]) * 0.5,
                            (aabb_max[1] - aabb_min[1]) * 0.5,
                            (aabb_max[2] - aabb_min[2]) * 0.5,
                        );
                        // Distance cull: skip models entirely beyond draw distance
                        let to_cam = center - cam_pos;
                        let closest_dist_sq = (to_cam.x.abs() - extent.x).max(0.0).powi(2)
                            + (to_cam.z.abs() - extent.z).max(0.0).powi(2);
                        if closest_dist_sq > max_draw_dist_sq {
                            continue;
                        }
                        if !frustum.test_aabb(center, extent) {
                            continue;
                        }
                    }
                    if let Some(ref mtex) = model.tex_bind_group {
                        rp.set_bind_group(3, mtex, &[]);
                    } else {
                        rp.set_bind_group(3, &self.tex_bg, &[]);
                    }
                    rp.set_vertex_buffer(0, model.mesh.vertex_buf.slice(..));
                    rp.set_index_buffer(model.mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                    rp.set_vertex_buffer(1, model.instance_buf.slice(..));
                    rp.draw_indexed(0..model.mesh.index_count, 0, 0..model.instance_count);
                    drawn_models += 1;
                }
                self.rendered_model_count = drawn_models;
            }

            // Render water (transparent, after all opaque objects) — aligned with render()
            if let Some(ref water) = self.water_renderer {
                water.render(&mut rp);
            }

            // Weather particles — render as instanced spheres after transparent objects.
            // Use dedicated weather material (bright white, non-metallic) and default
            // textures to prevent inheriting the last model's material/texture state.
            let weather_count = self.weather.particle_count() as u32;
            if weather_count > 0 {
                rp.set_bind_group(1, &self.weather_material_bg, &[]);
                rp.set_bind_group(3, &self.tex_bg, &[]);
                rp.set_vertex_buffer(0, self.mesh_sphere.vertex_buf.slice(..));
                rp.set_index_buffer(
                    self.mesh_sphere.index_buf.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                rp.set_vertex_buffer(1, self.weather.buffer().slice(..));
                rp.draw_indexed(0..self.mesh_sphere.index_count, 0, 0..weather_count);
            }
        }

        // --- Post-processing: bloom compute pass (runs on HDR, before blit) ---
        let bloom_intensity = if self.post_chain.bloom_enabled {
            // Lazily create the bloom pass on first use.
            let first_create = self.bloom_pass.is_none();
            if first_create {
                self.bloom_pass = Some(crate::bloom::BloomPass::new(
                    &self.device,
                    self.config.width,
                    self.config.height,
                ));
                // Apply any pending bloom config that was set before the pass existed.
                if let Some(cfg) = self.pending_bloom_config.take() {
                    self.bloom_pass.as_mut().expect("just created").set_config(cfg);
                }
            }
            // Take the bloom pass out to satisfy the borrow checker — execute()
            // needs &mut BloomPass while we also need &self.device, &self.queue, etc.
            let mut bloom = self
                .bloom_pass
                .take()
                .expect("bloom_pass was just ensured to be Some");
            let cfg = bloom.config().clone();
            bloom.execute(
                &self.device,
                &self.queue,
                enc,
                &self.hdr_view,
                cfg.threshold,
                cfg.intensity,
                self.resource_generation,
            );
            let intensity = cfg.intensity;
            // Rebuild the HDR blit bind group on first creation so it references
            // the real bloom output texture instead of the dummy.
            if first_create {
                let bloom_view = bloom.bloom_view().unwrap_or(&self.postfx_dummy_view);
                self.hdr_blit_bind_group =
                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("hdr blit bg"),
                        layout: &self.hdr_blit_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&self.hdr_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.hdr_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(bloom_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: self.postfx_params_buf.as_entire_binding(),
                            },
                        ],
                    });
            }
            self.bloom_pass = Some(bloom);
            intensity
        } else {
            0.0
        };
        // Upload blit compositing params (16 bytes — always written, cheap).
        self.queue.write_buffer(
            &self.postfx_params_buf,
            0,
            bytemuck::bytes_of(&[bloom_intensity, 0.0f32, 0.0, 0.0]),
        );

        // Blit internal HDR → external view.
        // Editor mode (surface=None): use passthrough blit (no tonemapping) —
        // the editor has its own tonemap pass.
        // Standalone mode: use the full post pipeline (ACES tonemap + tint).
        {
            let post_ts = di_ts_post.and_then(|(b, e)| {
                Some(wgpu::RenderPassTimestampWrites {
                    query_set: self.gpu_profiler.as_ref()?.query_set(),
                    beginning_of_pass_write_index: Some(b),
                    end_of_pass_write_index: Some(e),
                })
            });
            let mut pp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post pass (external)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: post_ts,
                occlusion_query_set: None,
            });
            if self.surface.is_none() {
                // Editor: passthrough blit (HDR → HDR, no tonemap)
                pp.set_pipeline(&self.hdr_blit_pipeline);
                pp.set_bind_group(0, &self.hdr_blit_bind_group, &[]);
                pp.draw(0..3, 0..1);
            } else {
                // Standalone: full post-processing (tonemap + tint)
                pp.set_pipeline(&self.post_pipeline);
                pp.set_bind_group(0, &self.post_bind_group, &[]);
                pp.set_bind_group(1, &self.scene_env_bg, &[]);
                pp.draw(0..3, 0..1);
            }
        }

        // --- GPU profiler: resolve timestamp queries into readback buffer ---
        if let Some(ref p) = self.gpu_profiler {
            p.end_frame(enc);
        }

        Ok(())
    }

    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn surface(&self) -> Option<&wgpu::Surface<'static>> {
        self.surface.as_ref()
    }

    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn render_with<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(
            &wgpu::TextureView,
            &mut wgpu::CommandEncoder,
            &wgpu::Device,
            &wgpu::Queue,
            (u32, u32),
        ),
    {
        let (frame, view) = match self.acquire_surface_texture()? {
            Some(pair) => pair,
            None => return Ok(()),
        };
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        // First render the 3D scene into the frame (draw_into posts to view)
        self.draw_into(&view, &mut enc)?;

        // Then allow caller to composite additional passes (e.g., egui)
        f(
            &view,
            &mut enc,
            &self.device,
            &self.queue,
            self.surface_size(),
        );

        self.queue.submit(std::iter::once(enc.finish()));
        frame.present();

        // --- GPU profiler: initiate async readback of timestamp data ---
        if let Some(ref mut p) = self.gpu_profiler {
            p.request_readback();
        }

        Ok(())
    }

    /// Render with callback for overlay-only use (skips 3D scene rendering).
    /// Clears to black and allows caller to render overlays like egui.
    pub fn render_with_simple<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(
            &wgpu::TextureView,
            &mut wgpu::CommandEncoder,
            &wgpu::Device,
            &wgpu::Queue,
            (u32, u32),
        ),
    {
        // Poll previous frame's GPU profiler readback even in simple (overlay-only) mode.
        if let Some(ref mut p) = self.gpu_profiler {
            p.poll_readback(&self.device);
        }

        let (frame, view) = match self.acquire_surface_texture()? {
            Some(pair) => pair,
            None => return Ok(()),
        };
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("simple encoder"),
            });

        // Just clear to black - no 3D rendering
        {
            let _clear_pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        // Allow caller to composite overlays
        f(
            &view,
            &mut enc,
            &self.device,
            &self.queue,
            self.surface_size(),
        );

        self.queue.submit(std::iter::once(enc.finish()));
        frame.present();
        Ok(())
    }

    /// Create a bind group layout deriving entries from a `MaterialPackage` bindings list.
    pub fn bgl_from_material_package(&self, pkg: &MaterialPackage) -> wgpu::BindGroupLayout {
        let mut entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
        let mut binding = 0u32;
        for _id in pkg.bindings.iter() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            binding += 1;
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
            binding += 1;
        }
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("material bgl (derived)"),
                entries: &entries,
            })
    }

    /// Create a simple full-screen pipeline from a `MaterialPackage` (for previews or tests).
    pub fn pipeline_from_material_package(
        &self,
        pkg: &MaterialPackage,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let shader = self.shader_from_material_package(pkg);
        let bgl = self.bgl_from_material_package(pkg);
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("material pipeline layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: self.pipeline_cache.as_ref(),
                label: Some("material preview pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
    }

    fn build_visible_instances(&self) -> (Vec<InstanceRaw>, usize) {
        let m = Mat4::from_cols_array_2d(&self.camera_ubo.view_proj);
        let mt = m.transpose();
        let r0 = mt.x_axis;
        let r1 = mt.y_axis;
        let r2 = mt.z_axis;
        let r3 = mt.w_axis;
        let planes = [
            r3 + r0, // left
            r3 - r0, // right
            r3 + r1, // bottom
            r3 - r1, // top
            r3 + r2, // near
            r3 - r2, // far
        ];
        // Normalize frustum planes inline (avoids small Vec allocation)
        let norm_planes: [(glam::Vec3, f32); 6] = std::array::from_fn(|i| {
            let p = planes[i];
            let n = glam::Vec3::new(p.x, p.y, p.z);
            let len = n.length().max(1e-6);
            (n / len, p.w / len)
        });

        let mut out = Vec::with_capacity(self.instances.len());
        for inst in &self.instances {
            let center = inst.transform.w_axis.truncate();
            // approximate radius from basis vectors length (half-extents length)
            let sx = inst.transform.x_axis.truncate().length();
            let sy = inst.transform.y_axis.truncate().length();
            let sz = inst.transform.z_axis.truncate().length();
            let radius = 0.5 * glam::Vec3::new(sx, sy, sz).length();
            if inside_frustum_sphere(center, radius, &norm_planes) {
                out.push(inst.raw());
            }
        }
        let count = out.len();
        (out, count)
    }

    /// Load a texture from disk for smoke testing / dev validation.
    /// INVARIANT: Texture I/O failures here are unrecoverable dev-time errors.
    #[allow(clippy::expect_used)]
    pub fn set_smoke_test_texture(&mut self, path: &str) {
        #[cfg(feature = "textures")]
        {
            use std::path::Path;
            let path_ref = Path::new(path);

            let rgba = if path_ref.extension().and_then(|s| s.to_str()) == Some("ktx2") {
                match crate::material_loader::material_loader_impl::load_ktx2_to_rgba(path_ref) {
                    Ok(img) => img,
                    Err(e) => {
                        log::warn!(
                            "Failed to load KTX2 texture '{}': {}. Falling back to standard image loading.",
                            path,
                            e
                        );
                        // Fallback: manually read and guess format because image::open fails on .ktx2 extensions it doesn't know
                        let bytes = std::fs::read(path).expect("Failed to read fallback file");
                        image::load_from_memory(&bytes)
                            .expect("Failed to decode fallback texture (unknown format)")
                            .to_rgba8()
                    }
                }
            } else {
                image::open(path)
                    .expect("Failed to load smoke test texture")
                    .to_rgba8()
            };

            let (width, height) = (rgba.width(), rgba.height());
            self.set_albedo_from_rgba8(width, height, &rgba);
        }
        #[cfg(not(feature = "textures"))]
        {
            log::warn!("Textures feature disabled, ignoring set_smoke_test_texture");
        }
    }

    pub fn set_albedo_from_rgba8(&mut self, width: u32, height: u32, data: &[u8]) {
        assert_eq!(data.len() as u32, width * height * 4);
        let mip_count = if width > 1 && height > 1 {
            (width.max(height) as f32).log2().floor() as u32 + 1
        } else {
            1
        };
        // Recreate texture with full mip chain
        self.albedo_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("albedo"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.albedo_view = self
            .albedo_tex
            .create_view(&wgpu::TextureViewDescriptor::default());
        // Upload mip 0 (base level)
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.albedo_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        // Generate and upload remaining mip levels (sRGB-aware box filter)
        if mip_count > 1 {
            #[inline]
            fn srgb_to_linear(v: u8) -> f32 {
                let f = v as f32 / 255.0;
                if f <= 0.04045 {
                    f / 12.92
                } else {
                    ((f + 0.055) / 1.055).powf(2.4)
                }
            }
            #[inline]
            fn linear_to_srgb(v: f32) -> u8 {
                let f = if v <= 0.0031308 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                };
                (f * 255.0).round().clamp(0.0, 255.0) as u8
            }

            let mut prev = data.to_vec();
            let mut mip_w = width;
            let mut mip_h = height;
            for mip in 1..mip_count {
                let new_w = (mip_w / 2).max(1);
                let new_h = (mip_h / 2).max(1);
                let mut next = vec![0u8; (new_w * new_h * 4) as usize];
                for y in 0..new_h {
                    for x in 0..new_w {
                        let sx = x * 2;
                        let sy = y * 2;
                        let sx1 = (sx + 1).min(mip_w - 1);
                        let sy1 = (sy + 1).min(mip_h - 1);
                        // Sample 2x2 block indices
                        let i00 = ((sy * mip_w + sx) * 4) as usize;
                        let i10 = ((sy * mip_w + sx1) * 4) as usize;
                        let i01 = ((sy1 * mip_w + sx) * 4) as usize;
                        let i11 = ((sy1 * mip_w + sx1) * 4) as usize;
                        let dst = ((y * new_w + x) * 4) as usize;
                        // Average RGB in linear space (sRGB-aware)
                        for c in 0..3 {
                            let s0 = srgb_to_linear(prev[i00 + c]);
                            let s1 = srgb_to_linear(prev[i10 + c]);
                            let s2 = srgb_to_linear(prev[i01 + c]);
                            let s3 = srgb_to_linear(prev[i11 + c]);
                            next[dst + c] = linear_to_srgb((s0 + s1 + s2 + s3) * 0.25);
                        }
                        // Alpha averaged directly (linear)
                        let a = (prev[i00 + 3] as u32
                            + prev[i10 + 3] as u32
                            + prev[i01 + 3] as u32
                            + prev[i11 + 3] as u32)
                            / 4;
                        next[dst + 3] = a as u8;
                    }
                }
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.albedo_tex,
                        mip_level: mip,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &next,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(new_w * 4),
                        rows_per_image: Some(new_h),
                    },
                    wgpu::Extent3d {
                        width: new_w,
                        height: new_h,
                        depth_or_array_layers: 1,
                    },
                );
                prev = next;
                mip_w = new_w;
                mip_h = new_h;
            }
        }
        // Rebuild the combined tex+skin bind group with current views/samplers and skin buffer
        self.tex_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("combined tex bg"),
            layout: &self.tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.albedo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.mr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.mr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.skin_palette_buf.as_entire_binding(),
                },
            ],
        });
    }

    pub fn set_metallic_roughness_from_rgba8(&mut self, width: u32, height: u32, data: &[u8]) {
        assert_eq!(data.len() as u32, width * height * 4);
        self.mr_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mr tex"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.mr_view = self
            .mr_tex
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.mr_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        // Rebuild combined tex_bg so MR/normal updates are reflected
        self.tex_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("combined tex bg"),
            layout: &self.tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.albedo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.mr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.mr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.skin_palette_buf.as_entire_binding(),
                },
            ],
        });
    }

    pub fn set_normal_from_rgba8(&mut self, width: u32, height: u32, data: &[u8]) {
        assert_eq!(data.len() as u32, width * height * 4);
        self.normal_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("normal tex"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.normal_view = self
            .normal_tex
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.normal_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        // Rebuild combined tex_bg so MR/normal updates are reflected
        self.tex_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("combined tex bg"),
            layout: &self.tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.albedo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.mr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.mr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.skin_palette_buf.as_entire_binding(),
                },
            ],
        });
    }

    // --- Skinning API (v0) ---
    pub fn set_skinned_mesh(&mut self, vertices: &[SkinnedVertex], indices: &[u32]) {
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("skinned vbuf"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("skinned ibuf"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        self.skinned_mesh = Some((vbuf, ibuf, indices.len() as u32));
    }

    pub fn update_skin_palette(&mut self, mats: &[glam::Mat4]) {
        // Upload contiguous mat4 array

        let mut data: Vec<f32> = Vec::with_capacity(mats.len() * 16);
        for m in mats {
            data.extend_from_slice(&m.to_cols_array());
        }
        self.queue
            .write_buffer(&self.skin_palette_buf, 0, bytemuck::cast_slice(&data));
    }

    // --- External Mesh API (additional helpers) ---
    /// Clear the external mesh, reverting to default sphere rendering.
    pub fn clear_external_mesh(&mut self) {
        self.mesh_external = None;
        self.ext_inst_buf = None;
    }

    /// Set instances for external mesh rendering.
    /// Each instance requires a transform and color.
    /// Uses grow-on-demand: only re-allocates when count exceeds buffer capacity.
    pub fn set_external_instances(&mut self, instances: &[Instance]) {
        if instances.is_empty() {
            self.ext_inst_count = 0;
            return;
        }

        let raw: Vec<_> = instances.iter().map(|i| i.raw()).collect();
        let required_bytes = (raw.len() * std::mem::size_of::<InstanceRaw>()) as u64;

        let needs_realloc = match &self.ext_inst_buf {
            Some(buf) => buf.size() < required_bytes,
            None => true,
        };

        if needs_realloc {
            let alloc_size = required_bytes.next_power_of_two().max(256);
            self.ext_inst_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ext-inst-buf"),
                size: alloc_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        // SAFETY: ext_inst_buf is guaranteed Some — allocated just above if it was None or too small.
        self.queue.write_buffer(
            self.ext_inst_buf
                .as_ref()
                .expect("ext_inst_buf allocated above"),
            0,
            bytemuck::cast_slice(&raw),
        );
        self.ext_inst_count = instances.len() as u32;
    }

    /// Check if an external mesh is currently set.
    pub fn has_external_mesh(&self) -> bool {
        self.mesh_external.is_some()
    }

    // --- Multi-Model API ---
    /// Add or replace a named model with the given mesh and instances.
    pub fn add_model(&mut self, name: impl Into<String>, mesh: Mesh, instances: &[Instance]) {
        self.add_model_impl(name, mesh, instances, None);
    }

    /// Add or replace a named model with an explicit world-space AABB for
    /// frustum culling. Models outside the camera frustum are skipped.
    pub fn add_model_with_bounds(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        aabb_min: [f32; 3],
        aabb_max: [f32; 3],
    ) {
        self.add_model_impl(name, mesh, instances, Some((aabb_min, aabb_max)));
    }

    fn add_model_impl(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        aabb: Option<([f32; 3], [f32; 3])>,
    ) {
        let raw: Vec<_> = instances.iter().map(|i| i.raw()).collect();
        let buf_bytes = (raw.len() * std::mem::size_of::<InstanceRaw>()) as u64;
        let alloc_size = buf_bytes.next_power_of_two().max(256);
        let instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model-inst-buf"),
            size: alloc_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !raw.is_empty() {
            self.queue
                .write_buffer(&instance_buf, 0, bytemuck::cast_slice(&raw));
        }
        let model = RenderModel {
            mesh,
            instance_buf,
            instance_count: instances.len() as u32,
            aabb,
            tex_bind_group: None,
            _retained_tex: None,
            tex_gpu_bytes: 0,
        };
        self.models.insert(name.into(), model);
    }

    /// Add a model with its own albedo texture (extracted from glTF).
    ///
    /// Creates a per-model texture bind group so the model renders with its
    /// own albedo instead of the global terrain texture. The texture is
    /// retained in the `RenderModel` to keep the bind group valid.
    pub fn add_model_with_texture(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        albedo_width: u32,
        albedo_height: u32,
        albedo_rgba: &[u8],
    ) {
        let raw: Vec<_> = instances.iter().map(|i| i.raw()).collect();
        let buf_bytes = (raw.len() * std::mem::size_of::<InstanceRaw>()) as u64;
        let alloc_size = buf_bytes.next_power_of_two().max(256);
        let instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model-inst-buf"),
            size: alloc_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !raw.is_empty() {
            self.queue
                .write_buffer(&instance_buf, 0, bytemuck::cast_slice(&raw));
        }

        // Create per-model albedo texture with mipmaps.
        // Pre-compute GPU memory needed for the full mip chain so we can
        // track it in the memory budget and log a warning if it's large.
        let mip_count = (albedo_width.max(albedo_height) as f32).log2().floor() as u32 + 1;
        {
            let mut total_bytes: u64 = 0;
            let (mut mw, mut mh) = (albedo_width as u64, albedo_height as u64);
            for _ in 0..mip_count {
                total_bytes += mw * mh * 4;
                mw = (mw / 2).max(1);
                mh = (mh / 2).max(1);
            }
            self.gpu_memory_budget
                .try_allocate(crate::gpu_memory::MemoryCategory::Textures, total_bytes);
        }
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("model albedo"),
            size: wgpu::Extent3d {
                width: albedo_width,
                height: albedo_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload mip level 0 (full resolution)
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            albedo_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * albedo_width),
                rows_per_image: Some(albedo_height),
            },
            wgpu::Extent3d {
                width: albedo_width,
                height: albedo_height,
                depth_or_array_layers: 1,
            },
        );

        // Generate and upload mip chain via 2×2 box filter
        {
            let mut prev = albedo_rgba.to_vec();
            let mut w = albedo_width;
            let mut h = albedo_height;
            for level in 1..mip_count {
                let nw = (w / 2).max(1);
                let nh = (h / 2).max(1);
                let mut mip = vec![0u8; (nw * nh * 4) as usize];
                for y in 0..nh {
                    for x in 0..nw {
                        let sx = (x * 2).min(w - 1);
                        let sy = (y * 2).min(h - 1);
                        let sx1 = (sx + 1).min(w - 1);
                        let sy1 = (sy + 1).min(h - 1);
                        for c in 0..4u32 {
                            let i00 = ((sy * w + sx) * 4 + c) as usize;
                            let i10 = ((sy * w + sx1) * 4 + c) as usize;
                            let i01 = ((sy1 * w + sx) * 4 + c) as usize;
                            let i11 = ((sy1 * w + sx1) * 4 + c) as usize;
                            let avg = (prev[i00] as u16
                                + prev[i10] as u16
                                + prev[i01] as u16
                                + prev[i11] as u16)
                                / 4;
                            mip[((y * nw + x) * 4 + c) as usize] = avg as u8;
                        }
                    }
                }
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &tex,
                        mip_level: level,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &mip,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * nw),
                        rows_per_image: Some(nh),
                    },
                    wgpu::Extent3d {
                        width: nw,
                        height: nh,
                        depth_or_array_layers: 1,
                    },
                );
                prev = mip;
                w = nw;
                h = nh;
            }
        }
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Build bind group matching tex_bgl (group 3): albedo, MR, normal, skin palette
        let tex_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model tex bg"),
            layout: &self.tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.albedo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.mr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.mr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.skin_palette_buf.as_entire_binding(),
                },
            ],
        });

        // Compute total GPU memory for this texture (all mip levels, 4 bytes per pixel)
        let tex_gpu_bytes = {
            let mut total: u64 = 0;
            let (mut mw, mut mh) = (albedo_width as u64, albedo_height as u64);
            for _ in 0..mip_count {
                total += mw * mh * 4;
                mw = (mw / 2).max(1);
                mh = (mh / 2).max(1);
            }
            total
        };

        let model = RenderModel {
            mesh,
            instance_buf,
            instance_count: instances.len() as u32,
            aabb: None,
            tex_bind_group: Some(tex_bg),
            _retained_tex: Some(tex),
            tex_gpu_bytes,
        };
        self.models.insert(name.into(), model);
    }

    /// Add a model that shares the texture bind group from an existing model.
    ///
    /// This avoids creating duplicate GPU textures when multiple models use
    /// the same albedo (e.g., scatter quadrants from the same mesh group).
    /// The texture bind group and retained GPU texture are cloned (wgpu
    /// resources are reference-counted, so this is cheap).
    ///
    /// Returns `true` if the source model was found and the new model was
    /// created, `false` if the source model does not exist (caller should
    /// fall back to `add_model`).
    pub fn add_model_sharing_texture(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        source_model: &str,
    ) -> bool {
        let (shared_bg, shared_tex) = match self.models.get(source_model) {
            Some(src) => (src.tex_bind_group.clone(), src._retained_tex.clone()),
            None => return false,
        };
        let raw: Vec<_> = instances.iter().map(|i| i.raw()).collect();
        let buf_bytes = (raw.len() * std::mem::size_of::<InstanceRaw>()) as u64;
        let alloc_size = buf_bytes.next_power_of_two().max(256);
        let instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model-inst-buf"),
            size: alloc_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !raw.is_empty() {
            self.queue
                .write_buffer(&instance_buf, 0, bytemuck::cast_slice(&raw));
        }
        let model = RenderModel {
            mesh,
            instance_buf,
            instance_count: instances.len() as u32,
            aabb: None,
            tex_bind_group: shared_bg,
            _retained_tex: shared_tex,
            tex_gpu_bytes: 0, // Don't double-count — source owns the budget
        };
        self.models.insert(name.into(), model);
        true
    }

    /// Remove a named model and release its GPU resources.
    pub fn clear_model(&mut self, name: &str) {
        if let Some(model) = self.models.remove(name) {
            if model.tex_gpu_bytes > 0 {
                self.gpu_memory_budget.deallocate(
                    crate::gpu_memory::MemoryCategory::Textures,
                    model.tex_gpu_bytes,
                );
            }
        }
    }

    /// Update only the instance buffer for an existing named model.
    ///
    /// This is **much** cheaper than `clear_model` + `add_model` because it
    /// reuses the existing mesh GPU buffers and texture bind group. The
    /// instance buffer is only reallocated when the new count exceeds the
    /// current capacity.
    ///
    /// Returns `true` if the model existed and was updated, `false` if the
    /// model name was not found (caller should fall back to `add_model`).
    pub fn update_model_instances(&mut self, name: &str, instances: &[Instance]) -> bool {
        let Some(model) = self.models.get_mut(name) else {
            return false;
        };
        let raw: Vec<_> = instances.iter().map(|i| i.raw()).collect();
        let needed = (raw.len() * std::mem::size_of::<InstanceRaw>()) as u64;
        if needed > model.instance_buf.size() {
            // Reallocate with headroom to avoid repeated resizes
            model.instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("model-inst-buf (resized)"),
                size: needed.next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !raw.is_empty() {
            self.queue
                .write_buffer(&model.instance_buf, 0, bytemuck::cast_slice(&raw));
        }
        model.instance_count = instances.len() as u32;
        true
    }

    /// Set or update the AABB for an existing named model (enables frustum culling).
    pub fn set_model_bounds(&mut self, name: &str, aabb_min: [f32; 3], aabb_max: [f32; 3]) {
        if let Some(model) = self.models.get_mut(name) {
            model.aabb = Some((aabb_min, aabb_max));
        }
    }

    /// Reposition the ground fill plane for terrain mode.
    ///
    /// Places the plane at `y_offset` (below terrain minimum) with a large
    /// scale to block the sky dome from showing through gaps. A dark ground
    /// color prevents the "inverted sky" appearance.
    pub fn set_terrain_ground_plane(&mut self, y_offset: f32, half_extent: f32) {
        let xform = glam::Mat4::from_translation(glam::vec3(0.0, y_offset, 0.0))
            * glam::Mat4::from_scale(glam::vec3(half_extent, 1.0, half_extent));
        let inst = Instance {
            transform: xform,
            color: [0.08, 0.09, 0.07, 1.0], // dark earth tone
            material_id: 0,
        }
        .raw();
        self.queue
            .write_buffer(&self.plane_inst_buf, 0, bytemuck::bytes_of(&inst));
        self.terrain_ground_set = true;
    }

    /// Reset the ground fill plane to its default position and scale.
    pub fn reset_ground_plane(&mut self) {
        let xform = glam::Mat4::from_translation(glam::vec3(0.0, -0.2, 0.0))
            * glam::Mat4::from_scale(glam::vec3(50.0, 1.0, 50.0));
        let inst = Instance {
            transform: xform,
            color: [0.1, 0.12, 0.14, 1.0],
            material_id: 0,
        }
        .raw();
        self.queue
            .write_buffer(&self.plane_inst_buf, 0, bytemuck::bytes_of(&inst));
        self.terrain_ground_set = false;
    }

    /// Check if a named model exists.
    pub fn has_model(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    /// Get the number of loaded models.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Set the maximum draw distance for model culling.
    /// Models beyond this distance from the camera are skipped entirely.
    /// Set to 0.0 to fall back to fog_end * 1.2 (the default).
    pub fn set_max_draw_distance(&mut self, dist: f32) {
        self.max_draw_distance = dist;
    }

    /// Get the current max draw distance setting (0.0 means fog-based fallback).
    pub fn max_draw_distance(&self) -> f32 {
        self.max_draw_distance
    }

    /// Number of models actually drawn in the last frame (after frustum + distance culling).
    pub fn rendered_model_count(&self) -> u32 {
        self.rendered_model_count
    }

    /// Get names of all loaded models.
    pub fn model_names(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }

    /// Collect names of models matching a prefix, without cloning all keys.
    pub fn model_names_with_prefix(&self, prefix: &str) -> Vec<String> {
        self.models
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect()
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.save_pipeline_cache();
    }
}

#[cfg(test)]
mod mat_integration_tests {
    use astraweave_materials::{Graph, MaterialPackage, Node};

    #[test]
    fn material_package_composes_valid_shader() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(
            "uv".into(),
            Node::Constant3 {
                value: [0.0, 0.0, 0.0],
            },
        );
        nodes.insert(
            "base_tex".into(),
            Node::Texture2D {
                id: "albedo".into(),
                uv: "uv".into(),
            },
        );
        let g = Graph {
            nodes,
            base_color: "base_tex".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let pkg = MaterialPackage::from_graph(&g).expect("compile");
        // Compose shader text and validate via naga
        let mut decls = String::new();
        let mut idx = 0u32;
        for id in pkg.bindings.iter() {
            decls.push_str(&format!(
                "@group(0) @binding({}) var tex_{}: texture_2d<f32>;\n",
                idx, id
            ));
            idx += 1;
            decls.push_str(&format!(
                "@group(0) @binding({}) var samp_{}: sampler;\n",
                idx, id
            ));
            idx += 1;
        }
        let full = format!(
            "{}\n{}\n@fragment fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {{ let m = eval_material(uv); return vec4<f32>(m.base,1.0); }}\n",
            decls, pkg.wgsl
        );
        let res = naga::front::wgsl::parse_str(&full);
        assert!(
            res.is_ok(),
            "Material-composed WGSL failed to parse: {:?}",
            res.err()
        );
    }
}

// --- Simple CPU frustum culling for instances ---

fn inside_frustum_sphere(center: glam::Vec3, radius: f32, planes: &[(glam::Vec3, f32)]) -> bool {
    for (n, d) in planes.iter() {
        if n.dot(center) + d < -radius {
            return false;
        }
    }
    true
}

// --- CSM utilities ---
fn frustum_corners_ws(cam: &crate::camera::Camera, near: f32, far: f32) -> [glam::Vec3; 8] {
    let dir = crate::camera::Camera::dir(cam.yaw, cam.pitch);
    let right = dir.cross(glam::Vec3::Y).normalize();
    let up = glam::Vec3::Y;
    let h_near = (cam.fovy * 0.5).tan() * near;
    let w_near = h_near * cam.aspect.max(0.01);
    let h_far = (cam.fovy * 0.5).tan() * far;
    let w_far = h_far * cam.aspect.max(0.01);
    let c_near = cam.position + dir * near;
    let c_far = cam.position + dir * far;
    [
        c_near + up * h_near - right * w_near, // near TL
        c_near + up * h_near + right * w_near, // near TR
        c_near - up * h_near - right * w_near, // near BL
        c_near - up * h_near + right * w_near, // near BR
        c_far + up * h_far - right * w_far,    // far TL
        c_far + up * h_far + right * w_far,    // far TR
        c_far - up * h_far - right * w_far,    // far BL
        c_far - up * h_far + right * w_far,    // far BR
    ]
}

fn frustum_center(corners: &[glam::Vec3; 8]) -> glam::Vec3 {
    let mut acc = glam::Vec3::ZERO;
    for c in corners.iter() {
        acc += *c;
    }
    acc / 8.0
}

/// Compute the bounding sphere radius for a set of frustum corners.
/// Used for rotationally-stable cascade shadow map projections.
fn sphere_radius(corners: &[glam::Vec3; 8], center: glam::Vec3) -> f32 {
    corners
        .iter()
        .map(|c| (*c - center).length())
        .fold(0.0_f32, f32::max)
}

#[allow(dead_code)]
fn aabb_in_view_space(view: &glam::Mat4, corners_ws: &[glam::Vec3; 8]) -> (glam::Vec3, glam::Vec3) {
    let mut min = glam::Vec3::splat(f32::INFINITY);
    let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
    for &c in corners_ws.iter() {
        let v = *view * glam::Vec4::new(c.x, c.y, c.z, 1.0);
        let p = v.xyz();
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{vec3, Mat4, Vec3};

    #[test]
    fn test_inside_frustum_sphere() {
        let planes = vec![
            (vec3(1.0, 0.0, 0.0), 1.0),  // x + 1 = 0 -> x = -1
            (vec3(-1.0, 0.0, 0.0), 1.0), // -x + 1 = 0 -> x = 1
        ];

        // Inside
        assert!(inside_frustum_sphere(vec3(0.0, 0.0, 0.0), 0.5, &planes));
        // Outside
        assert!(!inside_frustum_sphere(vec3(2.0, 0.0, 0.0), 0.5, &planes));
        // Intersecting
        assert!(inside_frustum_sphere(vec3(1.2, 0.0, 0.0), 0.5, &planes));
    }

    #[test]
    fn test_frustum_corners_ws() {
        let cam = crate::camera::Camera {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 90.0f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let corners = frustum_corners_ws(&cam, 1.0, 10.0);
        assert_eq!(corners.len(), 8);

        // Center of corners should be along the forward axis (X+)
        let center = frustum_center(&corners);
        assert!(center.x > 0.0);
        assert!(center.y.abs() < 0.001);
        assert!(center.z.abs() < 0.001);
    }

    #[test]
    fn test_aabb_in_view_space() {
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::Z, Vec3::Y);
        let corners = [
            vec3(-1.0, -1.0, 1.0),
            vec3(1.0, -1.0, 1.0),
            vec3(-1.0, 1.0, 1.0),
            vec3(1.0, 1.0, 1.0),
            vec3(-1.0, -1.0, 2.0),
            vec3(1.0, -1.0, 2.0),
            vec3(-1.0, 1.0, 2.0),
            vec3(1.0, 1.0, 2.0),
        ];

        let (min, max) = aabb_in_view_space(&view, &corners);
        assert!(min.x < max.x);
        assert!(min.y < max.y);
        assert!(min.z < max.z);
    }

    /// Validates the shadow override sentinel logic:
    /// When `force_shadow_override` is false (default), extras.x should be the
    /// normal pcf_radius_px (>= 0). When true, it should be -1.0.
    #[test]
    fn test_shadow_override_sentinel_logic() {
        // Default: shadows should use the real pcf radius
        let pcf_radius = 1.5_f32;
        let force_shadow_override = false;
        let extras_x = if force_shadow_override {
            -1.0_f32
        } else {
            pcf_radius
        };
        assert!(
            extras_x >= 0.0,
            "default path should pass non-negative extras.x = pcf_radius"
        );
        assert!((extras_x - pcf_radius).abs() < 1e-6);

        // Override active: sentinel should be -1.0
        let force_shadow_override = true;
        let extras_x = if force_shadow_override {
            -1.0_f32
        } else {
            pcf_radius
        };
        assert!(
            extras_x < 0.0,
            "override path should pass negative sentinel"
        );
        assert!((extras_x - (-1.0)).abs() < 1e-6);
    }

    /// Ensures the force_shadow_override field doesn't affect the WGSL shader source.
    /// The shader checks `uLight.extras.x < 0.0` — this test validates that the main
    /// PBR shader string contains the conditional (not a hardcoded override).
    #[test]
    fn test_shader_has_conditional_shadow_not_hardcoded() {
        let shader = SHADER_SRC;
        // Must NOT contain the old hardcoded override
        assert!(
            !shader.contains("// DEBUG: Force shadows off"),
            "hardcoded shadow override should have been removed"
        );
        // Must contain the conditional sentinel check
        assert!(
            shader.contains("uLight.extras.x < 0.0"),
            "shader should check sentinel for debug shadow override"
        );
    }

    #[test]
    fn brdf_common_contains_material_lod_functions() {
        let brdf_src = include_str!("../shaders/brdf_common.wgsl");
        assert!(
            brdf_src.contains("compute_material_lod"),
            "brdf_common.wgsl must contain compute_material_lod"
        );
        assert!(
            brdf_src.contains("evaluate_brdf_lod"),
            "brdf_common.wgsl must contain evaluate_brdf_lod"
        );
        // evaluate_brdf should delegate to evaluate_brdf_lod
        assert!(
            brdf_src
                .contains("evaluate_brdf_lod(N, V, L, base_color, metallic, roughness, F0, 0u)"),
            "evaluate_brdf should delegate to evaluate_brdf_lod with LOD 0"
        );
    }

    #[test]
    fn shader_uses_material_lod() {
        let shader = SHADER_SRC;
        assert!(
            shader.contains("compute_material_lod"),
            "static PBR shader should compute material LOD"
        );
        assert!(
            shader.contains("evaluate_brdf_lod"),
            "static PBR shader should call evaluate_brdf_lod"
        );
        let skinned = SKINNED_SHADER_SRC;
        assert!(
            skinned.contains("compute_material_lod"),
            "skinned PBR shader should compute material LOD"
        );
        assert!(
            skinned.contains("evaluate_brdf_lod"),
            "skinned PBR shader should call evaluate_brdf_lod"
        );
    }
}

// End of file
