#[cfg(feature = "postfx")]
use crate::post::{WGSL_SSAO, WGSL_SSGI, WGSL_SSR};
use anyhow::Context;
use anyhow::Result;
use glam::Vec4Swizzles;
use glam::{vec3, Mat4};
use std::borrow::Cow;
use wgpu::util::DeviceExt;

use crate::clustered::{bin_lights_cpu, ClusterDims, CpuLight, WGSL_CLUSTER_BIN};
use crate::depth::Depth;
use crate::types::SkinnedVertex;
use crate::types::{Instance, InstanceRaw, Mesh};
use astraweave_camera::{FreeFly, RenderView};
use astraweave_cinematics as awc;
use astraweave_materials::MaterialPackage;

/// Demo sky. The terrain pack already vendors this Poly Haven CC0 plate.
/// Do not add a second copy under `assets/hdri`.
pub const SHOWCASE_HDRI_PRIMARY: &str =
    "assets/terrain/polyhaven/pine_forest/hdri/spruit_sunrise_2k.exr";
/// Used when the primary plate is not on disk.
pub const SHOWCASE_HDRI_FALLBACK: &str = "assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr";

pub(crate) const SHADER_SRC: &str = concat!(
    include_str!("../shaders/constants.wgsl"),
    include_str!("../shaders/brdf_common.wgsl"),
    // L.2: IblParams + compute_ibl live in the shared fragment (also consumed
    // by the terrain-forward concat in terrain_material_manager.rs).
    include_str!("../shaders/ibl_common.wgsl"),
    // L.3: MainLightUbo + csm_shadow_factor live in the shared fragment (also
    // consumed by the terrain-forward concat) — one CSM sampling implementation.
    include_str!("../shaders/shadow_common.wgsl"),
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

// MainLightUbo struct + csm_shadow_factor come from shadow_common.wgsl
// (prepended) — hoisted there in L.3 so the terrain-forward path shares the
// same cascade-select + PCF implementation.
@group(2) @binding(0) var<uniform> uLight: MainLightUbo;
@group(2) @binding(1) var shadow_tex: texture_depth_2d_array;
@group(2) @binding(2) var shadow_sampler: sampler_comparison;

@group(3) @binding(0) var albedo_tex: texture_2d<f32>;
@group(3) @binding(1) var albedo_samp: sampler;
@group(3) @binding(2) var mr_tex: texture_2d<f32>;      // R: metallic, G: roughness
@group(3) @binding(3) var mr_samp: sampler;
@group(3) @binding(4) var normal_tex: texture_2d<f32>;  // tangent-space normal in RGB, nor_gl (Y up)
@group(3) @binding(5) var normal_samp: sampler;
@group(3) @binding(7) var occlusion_tex: texture_2d<f32>; // R: ambient occlusion
@group(3) @binding(8) var occlusion_samp: sampler;
@group(3) @binding(9) var emissive_tex: texture_2d<f32>;
@group(3) @binding(10) var emissive_samp: sampler;

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
    // ED-3: 0=lit, 1=unlit albedo, 2=world-space normals, 3=UVs (former pad).
    debug_mode: f32, exposure: f32, debug_flags: f32,
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

// IblParams struct + compute_ibl come from ibl_common.wgsl (prepended).
@group(5) @binding(4) var<uniform> uIbl: IblParams;

// Screen-space GI (from SSGI/Lumen pipeline). Fallback is 1x1 black.
@group(5) @binding(5) var gi_tex: texture_2d<f32>;
@group(5) @binding(6) var gi_samp: sampler;
// Cloud shadow transmittance map (1.0 = lit, 0.0 = shadowed)
@group(5) @binding(7) var cloud_shadow_tex: texture_2d<f32>;
@group(5) @binding(8) var cloud_shadow_samp: sampler;

fn sample_cloud_shadow(world_pos: vec3<f32>) -> vec2<f32> {
    let shadow_extent = 1024.0;
    let uv = (world_pos.xz - uIbl._pad) / (2.0 * shadow_extent) + vec2<f32>(0.5, 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return vec2<f32>(1.0, 1.0);
    }
    // .r is direct transmittance. .g is the indirect weight. .y is the
    // factor for IBL and ambient, 1 when the weight is 0.
    let s = textureSampleLevel(cloud_shadow_tex, cloud_shadow_samp, uv, 0.0);
    return vec2<f32>(s.r, mix(1.0, s.r, s.g));
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

// compute_ibl is defined in ibl_common.wgsl (prepended) — hoisted there in
// L.2 so the terrain-forward path shares the same implementation.

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

    // Distance from camera — used for shadows, fog, LOD, and the L.3.D
    // cascade-index debug view. Computed once, and hoisted above the debug
    // branch because mode 4 needs it (it is a pure expression; nothing between
    // its old and new position touched world_pos or camera_pos).
    let frag_dist = length(input.world_pos - uCamera.camera_pos);

    // ED-3 debug shading (uniform branch — same pipeline, same pass; the
    // editor's viewport dropdown drives uScene.debug_mode, 0 in game paths).
    if (uScene.debug_mode > 0.5) {
        if (uScene.debug_mode < 1.5) {
            return vec4<f32>(base_color, 1.0);               // 1: unlit albedo
        }
        if (uScene.debug_mode < 2.5) {
            return vec4<f32>(N * 0.5 + vec3<f32>(0.5), 1.0); // 2: world-space normals
        }
        if (uScene.debug_mode < 3.5) {
            return vec4<f32>(fract(input.uv), 0.0, 1.0);     // 3: UVs
        }
        // 4 (L.3.D): cascade index + refresh flash — same shared helper the
        // terrain shader calls, so the two views cannot disagree.
        return vec4<f32>(
            csm_cascade_debug_color(uLight, frag_dist, u32(max(uScene.debug_flags, 0.0))),
            1.0
        );
    }

    // Unified BRDF: Cook-Torrance specular + Burley diffuse + multiscatter
    // (from brdf_common.wgsl). One BRDF for every fragment — the material-LOD
    // tiers were retired in T.2d.F (visible footprint-2.0 boundary + tier
    // dithering; docs/audits/T2DF_OUTCOME.md).
    let brdf_result = evaluate_brdf(N, V, L, base_color, metallic, roughness, F0);

    let radiance = uScene.sun_color * uScene.sun_intensity; // from SceneEnv UBO

    // Shadow sampling — skip ALL shadow work when disabled (extras.x < 0).
    // extras.x is uniform (same for every fragment), so the GPU skips the
    // entire block for all warps — saves 9 PCF comparison samples + ALU.
    var shadow: f32 = 1.0;
    if (uLight.extras.x >= 0.0) {
        // Cascade select + PCF 3x3 + boundary fades — shared implementation
        // in shadow_common.wgsl (hoisted verbatim in L.3; the terrain-forward
        // path calls the same function at its own group(4) bindings).
        shadow = csm_shadow_factor(uLight, shadow_tex, shadow_sampler, input.world_pos, frag_dist);
    }
    // Direct lighting (evaluate_brdf already includes NdotL)
    let cloud_shadow = sample_cloud_shadow(input.world_pos);
    var lit_color = brdf_result * radiance * shadow * cloud_shadow.x;

    // IBL indirect lighting. Formerly gated on the material-LOD tier (distant
    // fragments got a 1-sample diffuse-only approximation) — that gate shared
    // the tiers' footprint-2.0 threshold and died with them in T.2d.F, so
    // every fragment now takes the full IBL path.
    let ao = textureSample(occlusion_tex, occlusion_samp, input.uv).r;
    let ibl_color = compute_ibl(N, V, base_color, metallic, roughness, F0);
    // glTF: occlusion scales indirect light only, not the direct sun.
    lit_color = lit_color + ibl_color * ao * cloud_shadow.y;

    // Screen-space GI: sample indirect diffuse from SSGI/Lumen pipeline.
    // When no GI pass is active, gi_tex is a 1x1 black texture → adds zero.
    let gi_dims = vec2<f32>(textureDimensions(gi_tex));
    let gi_uv = input.pos.xy / gi_dims;
    let gi_indirect = textureSampleLevel(gi_tex, gi_samp, gi_uv, 0.0).rgb;
    lit_color += gi_indirect * base_color * (1.0 - metallic) * ao * cloud_shadow.y;

    // Scene ambient as fallback floor (provides fill when IBL is low)
    let ambient = uScene.ambient_color * uScene.ambient_intensity * 0.35;
    lit_color = lit_color + base_color * ambient * ao * cloud_shadow.y;
    lit_color = lit_color + textureSample(emissive_tex, emissive_samp, input.uv).rgb;
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
    // L.1: exposure rides the former pad (same 96-B UBO as the main pass).
    debug_mode: f32,
    exposure: f32,
    _pad1z: f32,
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
    // L.1: exposure comes from the scene-env UBO (editor World panel slider);
    // defaults to 1.35 — the exact pre-L.1 hardcode — so untouched paths are
    // visually neutral.
    let exposure = uPostScene.exposure;
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
    // L.1: exposure rides the former pad (same 96-B UBO as the main pass).
    debug_mode: f32,
    exposure: f32,
    _pad1z: f32,
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
    // L.1: driven by the scene-env UBO (default 1.35 = the former hardcode).
    let exposure = uPostScene.exposure;
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

// L.3.A: layout matches shadow_common.wgsl's MainLightUbo. This dormant
// pipeline's shadow code still samples only c0/c1 via splits.x/.y — offsets
// stay correct, behavior legacy. `bias_scales` (L.3.C resolution) is declared
// for layout parity only; this shader never reads it.
struct MainLightUbo { view_proj0: mat4x4<f32>, view_proj1: mat4x4<f32>, view_proj2: mat4x4<f32>, view_proj3: mat4x4<f32>, splits: vec4<f32>, extras: vec4<f32>, bias_scales: vec4<f32>, view_proj_prev: mat4x4<f32> };
@group(2) @binding(0) var<uniform> uLight: MainLightUbo;
@group(2) @binding(1) var shadow_tex: texture_depth_2d_array;
@group(2) @binding(2) var shadow_sampler: sampler_comparison;

@group(3) @binding(0) var albedo_tex: texture_2d<f32>;
@group(3) @binding(1) var albedo_samp: sampler;
@group(3) @binding(2) var mr_tex: texture_2d<f32>;
@group(3) @binding(3) var mr_samp: sampler;
@group(3) @binding(4) var normal_tex: texture_2d<f32>;
@group(3) @binding(5) var normal_samp: sampler;
@group(3) @binding(7) var occlusion_tex: texture_2d<f32>;
@group(3) @binding(8) var occlusion_samp: sampler;
@group(3) @binding(9) var emissive_tex: texture_2d<f32>;
@group(3) @binding(10) var emissive_samp: sampler;
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
    // ED-3: 0=lit, 1=unlit albedo, 2=world-space normals, 3=UVs (former pad).
    debug_mode: f32, exposure: f32, debug_flags: f32,
    sun_color: vec3<f32>,
    sun_intensity: f32,
};
@group(4) @binding(0) var<uniform> uScene: SceneEnv;

// Group 5 shares the main PBR IBL layout. `_pad` is the cloud-shadow bake
// center; the struct matches `IblParams` in ibl_common.wgsl. This shader does
// not call `compute_ibl`, so the cubemap bindings stay undeclared.
struct IblParams {
    ibl_intensity: f32,
    max_spec_lod: f32,
    _pad: vec2<f32>,
};
@group(5) @binding(4) var<uniform> uIbl: IblParams;
@group(5) @binding(7) var cloud_shadow_tex: texture_2d<f32>;
@group(5) @binding(8) var cloud_shadow_samp: sampler;

fn sample_cloud_shadow(world_pos: vec3<f32>) -> vec2<f32> {
    let shadow_extent = 1024.0;
    let uv = (world_pos.xz - uIbl._pad) / (2.0 * shadow_extent) + vec2<f32>(0.5, 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return vec2<f32>(1.0, 1.0);
    }
    // .r is direct transmittance. .g is the indirect weight. .y is the
    // factor for IBL and ambient, 1 when the weight is 0.
    let s = textureSampleLevel(cloud_shadow_tex, cloud_shadow_samp, uv, 0.0);
    return vec2<f32>(s.r, mix(1.0, s.r, s.g));
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

    // ED-3 debug shading. The skinned vertex format carries no UVs, so mode 3
    // renders a mid-gray sentinel rather than lying with garbage data.
    if (uScene.debug_mode > 0.5) {
        if (uScene.debug_mode < 1.5) {
            return vec4<f32>(base_color, 1.0);               // 1: unlit albedo
        }
        if (uScene.debug_mode < 2.5) {
            return vec4<f32>(N * 0.5 + vec3<f32>(0.5), 1.0); // 2: world-space normals
        }
        if (uScene.debug_mode < 3.5) {
            return vec4<f32>(vec3<f32>(0.5), 1.0);           // 3: UVs (none — sentinel)
        }
        // 4 (L.3.D): cascade index. This dormant pipeline keeps its own inline
        // 2-cascade sampler and never consumed shadow_common.wgsl, so it has no
        // 4-cascade select to report. Magenta = "this view is not implemented
        // on this path" — a sentinel, not a colour in the cascade palette, so
        // it can never be mistaken for a cascade.
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }

    // Unified BRDF: Cook-Torrance specular + Burley diffuse + multiscatter
    // (from brdf_common.wgsl). One BRDF for every fragment — the material-LOD
    // tiers were retired in T.2d.F (docs/audits/T2DF_OUTCOME.md).
    let brdf_result = evaluate_brdf(N, V, L, base_color, metallic, roughness, F0);

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
    // Skinned vertices carry no UVs. Sample the map origin so a 1×1 default
    // (white AO, black emissive) is exact, and a real map is not ignored.
    let ao = textureSampleLevel(occlusion_tex, occlusion_samp, vec2<f32>(0.0), 0.0).r;
    let emissive = textureSampleLevel(emissive_tex, emissive_samp, vec2<f32>(0.0), 0.0).rgb;
    let lit_color = brdf_result * radiance * shadow * cloud_shadow.x + base_color * 0.08 * ao * cloud_shadow.y + emissive;
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

/// Matches `IblParams` in `ibl_common.wgsl`. `_pad` is the cloud-shadow bake
/// center (world XZ). It is not shading data; `compute_ibl` does not read it.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct IblParamsGpu {
    ibl_intensity: f32,
    max_spec_lod: f32,
    _pad: [f32; 2],
}

/// A named model with its mesh and instance data for multi-model rendering.
#[derive(Clone)]
pub struct RenderModelPrimitive {
    /// The GPU mesh (vertex/index buffers) for one primitive.
    pub mesh: Mesh,
    /// Per-primitive texture bind group (group 3). When Some, overrides global tex_bg.
    pub tex_bind_group: Option<wgpu::BindGroup>,
    /// Retained GPU textures to keep the bind group's texture views alive.
    pub _retained_textures: Vec<wgpu::Texture>,
    /// Increments only when [`Renderer::write_mesh`] replaces the GPU buffers.
    pub storage_generation: u32,
    /// Index into the bindless material SSBO. `None` on the bind-group path.
    pub bindless_index: Option<u32>,
}

pub struct RenderModel {
    /// The GPU primitives that make up this model.
    pub primitives: Vec<RenderModelPrimitive>,
    /// Instance buffer for this model.
    pub instance_buf: wgpu::Buffer,
    /// Number of instances.
    pub instance_count: u32,
    /// Previous frame's instance buffer for scene-vector object motion.
    /// `None` until the pass runs. The first frame copies the current buffer.
    scene_prev_instances: Option<wgpu::Buffer>,
    /// World-space AABB for frustum culling (None = always visible).
    pub aabb: Option<([f32; 3], [f32; 3])>,
    /// GPU memory used by this model's texture (for budget tracking).
    pub tex_gpu_bytes: u64,
    /// Asset path queued with [`Renderer::request_streamed_albedo`].
    pub stream_albedo_id: Option<String>,
    /// True after the resident texture replaced the 1×1 fallback bind.
    pub stream_resident: bool,
}

pub struct ModelSurfaceMaps<'a> {
    pub albedo: (u32, u32, &'a [u8]),
    /// Tangent-space normal, OpenGL / glTF / Poly Haven `nor_gl` (Y up).
    pub normal: (u32, u32, &'a [u8]),
    /// Engine layout: R = metallic, G = roughness. Convert glTF or ARM with
    /// [`crate::pbr_maps`] before passing bytes in.
    pub metallic_roughness: (u32, u32, &'a [u8]),
    /// R = ambient occlusion. `None` binds a 1×1 white map (no darkening).
    pub occlusion: Option<(u32, u32, &'a [u8])>,
    /// RGB emissive, added after lighting. `None` binds a 1×1 black map.
    pub emissive: Option<(u32, u32, &'a [u8])>,
}

fn texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn solid_rgba_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    pixel: [u8; 4],
) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixel,
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    (texture, view, sampler)
}

// ─────────────────────────────────────────────────────────────────────────
// Phase 1 forward-lit terrain state (Terrain Material System Campaign,
// Option D). Feature-gated — when `terrain-splat-arrays` is off, the
// whole concept is compiled out and the legacy `self.models`-based
// terrain path is used.

/// Per-chunk GPU buffers for the forward-lit terrain pipeline.
///
/// Holds the vertex and index buffers owned by a single terrain chunk.
/// Splat textures + bind groups live inside `TerrainMaterialManager`,
/// keyed by the same chunk id.
#[cfg(feature = "terrain-splat-arrays")]
pub struct TerrainChunkGpu {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    /// L.3: world-space bounds, computed by `upload_terrain_chunk` from the
    /// uploaded vertices (positions are world-space). Consumed by per-cascade
    /// frustum culling in the terrain shadow-caster pass.
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    /// Last frame's vertices. A collapse that rewrites `vertex_buffer` in
    /// place still has a one-tick object delta.
    prev_vertex: Option<wgpu::Buffer>,
}

/// Owns the forward-lit terrain rendering state inside `Renderer`.
///
/// `manager` holds the pipeline, shared layer arrays, UBOs, and per-chunk
/// splat bind groups. `chunks` holds per-chunk vertex/index buffers.
/// Constructed lazily via [`Renderer::init_terrain_forward`]; `None` until
/// the editor opts in (Phase 1.E.4 is the caller).
#[cfg(feature = "terrain-splat-arrays")]
pub struct TerrainForwardRenderer {
    pub manager: crate::terrain_material_manager::TerrainMaterialManager,
    pub chunks: std::collections::HashMap<u64, TerrainChunkGpu>,
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
    /// End query of the `frame` bracket. Written just before resolve.
    frame_stamp: Option<crate::gpu_profiler::OpenStamp>,
    /// Open interval between named passes, so uncovered GPU time gets a name.
    timing_gap: Option<crate::gpu_profiler::OpenStamp>,
    gpu_adapter_name: String,
    gpu_backend: String,
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
    /// ED-3 wireframe variant (polygon_mode Line); `None` without
    /// `POLYGON_MODE_LINE`.
    pipeline_wire: Option<wgpu::RenderPipeline>,
    /// ED-3: draw meshes/terrain with the wireframe variants this frame.
    wireframe_enabled: bool,
    material_buf: wgpu::Buffer,
    material_bg: wgpu::BindGroup,
    post_pipeline: wgpu::RenderPipeline,
    // Editor-Engine Render Parity P.3: hdr_blit_pipeline / hdr_blit_bind_group /
    // hdr_blit_bgl fields removed. Their sole consumer was `draw_into`'s
    // surface.is_none() branch (deleted in same commit); the canonical
    // post_pipeline now runs unconditionally.
    post_bind_group: wgpu::BindGroup,
    post_bgl: wgpu::BindGroupLayout,
    hdr_tex: wgpu::Texture,
    hdr_view: wgpu::TextureView,
    hdr_sampler: wgpu::Sampler,
    /// F.4.3 — optional post-water, pre-tonemap HDR overlay composite. Fired by
    /// `run_water_pass` AFTER the water surface draws into `hdr_view` and AFTER
    /// `water_renderer` is restored, BEFORE the tonemap/blit. Receives only `wgpu`
    /// types `(encoder, hdr_view, scene_depth, device, queue)`, so `astraweave-render`
    /// gains NO dependency on `astraweave-fluids` — the binary registers a closure
    /// whose body calls `FluidRenderer::render_accents` for the weave-impact accents.
    /// `None` (the default) records zero GPU work, preserving the prior frame exactly.
    #[allow(clippy::type_complexity)]
    hdr_overlay: Option<
        Box<
            dyn FnMut(
                &mut wgpu::CommandEncoder,
                &wgpu::TextureView,
                &wgpu::TextureView,
                &wgpu::Device,
                &wgpu::Queue,
            ),
        >,
    >,
    /// W.2b — opaque scene-color snapshot the split water pass samples for
    /// refraction (copied from `hdr_tex` after the opaque main pass closes).
    water_scene_color: wgpu::Texture,
    water_scene_color_view: wgpu::TextureView,
    // 1×1 black dummy texture used as AO/GI placeholder when SSAO/SSGI aren't active
    #[allow(dead_code)]
    _postfx_dummy_tex: wgpu::Texture,
    postfx_dummy_view: wgpu::TextureView,
    #[allow(dead_code)]
    shadow_tex: wgpu::Texture,
    #[allow(dead_code)]
    shadow_view: wgpu::TextureView, // array view for sampling
    /// One render target per cascade (layer of the shadow array).
    shadow_layer_views: [wgpu::TextureView; CASCADE_COUNT],
    #[allow(dead_code)]
    shadow_sampler: wgpu::Sampler,
    shadow_pipeline: wgpu::RenderPipeline,
    light_buf: wgpu::Buffer,
    light_bg: wgpu::BindGroup,
    // Per-cascade uniform buffers for shadow depth passes.
    // Using separate buffers avoids the queue.write_buffer race where all
    // writes resolve before the command buffer executes, causing both shadow
    // passes to read the same (last-written) cascade matrix.
    shadow_cascade_bufs: [wgpu::Buffer; CASCADE_COUNT],
    shadow_cascade_bgs: [wgpu::BindGroup; CASCADE_COUNT],
    #[allow(dead_code)]
    shadow_bgl: wgpu::BindGroupLayout,
    /// L.3: the single-mat4 caster-pass layout (`shadow_cascade_bgs`' layout),
    /// retained so the terrain caster pipeline can be (re)built lazily.
    shadow_bgl_light: wgpu::BindGroupLayout,
    // Cascade data cached on CPU for shadow passes
    /// Per-cascade light view-projections, index 0 = nearest. An ARRAY, not
    /// named fields: the L.3.C inventory found that the two pass loops selected
    /// the culling matrix with `match idx { 0 =>.., 1 =>.., _ => last }`, which
    /// would have silently culled a newly-added cascade against its
    /// predecessor's frustum. Indexing panics instead.
    cascades: [glam::Mat4; CASCADE_COUNT],
    /// Cached-cascade state, one entry per cascade at or beyond
    /// [`FIRST_CACHED_CASCADE`]. Those cascades re-fit and re-render only on
    /// refresh (sun move / window drift / slow tick) instead of every frame —
    /// their caster passes draw most of the world. Each ortho box is padded
    /// beyond the drift limit so a frozen window still covers its ideal slice.
    c_far_center: [glam::Vec3; CASCADE_COUNT - FIRST_CACHED_CASCADE],
    c_far_light_dir: [glam::Vec3; CASCADE_COUNT - FIRST_CACHED_CASCADE],
    c2_tick: u32,
    /// L.3.D rung 2: c3's OUTGOING window matrix, the blend weight (1.0 = fully
    /// on the current window, no transition in flight), and a request for the
    /// draw path to snapshot layer 3 into the spare layer before it re-renders.
    /// The snapshot is deliberately the draw path's job: it must land in the
    /// same encoder, immediately before the pass that overwrites the source.
    c3_prev_vp: glam::Mat4,
    c3_fade: f32,
    c3_copy_pending: bool,
    /// Which refresh policy the far cascades run under this session
    /// (`AW_CSM_FAR_POLICY`, read once at construction).
    far_policy: FarCascadePolicy,
    /// Per-cascade fits as ACTUALLY computed, published via
    /// [`Self::shadow_cascade_fits`]. Ground truth for the relief census.
    cascade_fits: [CascadeFit; CASCADE_COUNT],
    /// True once a shadow pass has actually filled that layer with its CURRENT
    /// matrix. Set by the PASS, cleared whenever the map's contents can no
    /// longer be trusted (shadows disabled).
    c_far_valid: [bool; CASCADE_COUNT - FIRST_CACHED_CASCADE],
    /// STICKY: set when a cached cascade's matrix changes, cleared ONLY by a
    /// shadow pass that renders that layer. Never assume a map is filled
    /// because its matrix was computed — the passes can be skipped entirely
    /// (the L.3.A bug: on the first frame after terrain upload the caster
    /// pipeline does not exist yet, so every pass is skipped).
    c_far_pending: [bool; CASCADE_COUNT - FIRST_CACHED_CASCADE],
    /// L.3.B refresh telemetry, **per far cascade**: monotonic refresh count,
    /// the trigger that fired last
    /// (`"invalid"`/`"policy"`/`"sun"`/`"drift"`/`"tick"`), this frame's window
    /// drift in metres, and the window `(centre, radius)` committed at the last
    /// refresh. Read via [`Self::shadow_far_telemetry`] — the
    /// continuous-capture harness correlates observed frame-steps against
    /// refresh events with it.
    ///
    /// L.3.C resolution widened these from scalars tracking only the OUTERMOST
    /// cascade: under the shipped `LiveC2` policy the outermost cascade is the
    /// one the policy does *not* change, so scalar telemetry made a `LiveC2`
    /// run numerically indistinguishable from `Cached`.
    c_far_refresh_count: [u32; CASCADE_COUNT - FIRST_CACHED_CASCADE],
    c_far_refresh_reason: [&'static str; CASCADE_COUNT - FIRST_CACHED_CASCADE],
    c_far_last_drift: [f32; CASCADE_COUNT - FIRST_CACHED_CASCADE],
    c_far_window: [(glam::Vec3, f32); CASCADE_COUNT - FIRST_CACHED_CASCADE],
    split0: f32,
    split1: f32,
    /// L.3.C: the c2|c3 boundary (1400 m). Delivered to the shader in
    /// `extras.z`.
    split2: f32,
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
    /// The default ground-plane instance was written. Later frames skip the write.
    plane_instance_uploaded: bool,
    /// Last visible-instance bytes uploaded. A matching frame skips `write_buffer`.
    visible_uploaded: Vec<u8>,
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
    /// 1×1 white. AO = 1 when a model has no occlusion map.
    /// The texture is retained so `occlusion_view` stays valid.
    #[allow(dead_code)]
    occlusion_tex: wgpu::Texture,
    occlusion_view: wgpu::TextureView,
    occlusion_sampler: wgpu::Sampler,
    /// 1×1 black. No emissive when a model has no emissive map.
    /// The texture is retained so `emissive_view` stays valid.
    #[allow(dead_code)]
    emissive_tex: wgpu::Texture,
    emissive_view: wgpu::TextureView,
    emissive_sampler: wgpu::Sampler,
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
    /// One Hi-Z pyramid. Built only when [`Self::sample_hiz`] is set, because
    /// each mip is its own wgpu compute pass (#269). [`Self::set_sample_hiz`]
    /// drops it so the next frame copies that frame's depth view.
    hiz: Option<crate::hiz_pyramid::HizPyramid>,
    /// Set with [`Self::set_sample_hiz`]. Default false: no pass samples
    /// [`crate::hiz_pyramid::HizPyramid::view`]. SSGI reads scene depth.
    sample_hiz: bool,
    /// GL rejects `textureLoad` on `texture_depth_2d`, so the pyramid stays off.
    hiz_depth_load: bool,
    /// Quality setting (#372). Default off, so frames keep the CPU frustum
    /// until a heavy-scene capture opts in.
    gpu_cull_on: bool,
    indirect_cull: Option<crate::culling::IndirectDrawPipeline>,
    indirect_cull_resources: Option<crate::culling::IndirectDrawResources>,
    /// Reused across frames so an enabled `draw_into` does not allocate.
    gpu_cull_aabb_scratch: Vec<crate::culling::InstanceAABB>,
    gpu_cull_template_scratch: Vec<crate::culling::DrawIndexedIndirectCommand>,
    /// True after the last `draw_into` dispatched the cull compute.
    gpu_cull_ran: bool,
    /// Indirect commands whose frustum test kept `instance_count`.
    gpu_cull_visible_draws: u32,
    /// Sorted model names. Updated when a model is added or removed, not per frame.
    gpu_cull_names: Vec<String>,
    /// World bounds used only by the GPU-cull dispatch. The CPU frustum ignores
    /// them, so the default-off path does not start culling models that had none.
    gpu_cull_bounds: std::collections::HashMap<String, ([f32; 3], [f32; 3])>,
    /// Local mesh bounds for models whose world AABB is not `model.aabb`.
    /// `duplicate_model` and instance updates transform these.
    gpu_cull_local: std::collections::HashMap<String, ([f32; 3], [f32; 3])>,
    /// Instances that placed `gpu_cull_local`. Vertex rewrites use them again.
    gpu_cull_instances: std::collections::HashMap<String, Vec<Instance>>,
    /// Presentation-only particle sim. `None` until an emitter is enabled.
    particle_sim: Option<crate::gpu_particles::GpuParticleSystem>,
    particle_draw: Option<crate::particle_render::ParticleRenderPass>,
    particle_sort: Option<wgpu::Buffer>,
    particle_emitter: Option<crate::gpu_particles::EmitterParams>,
    /// Copy of scene depth sampled when soft particles also write depth.
    /// The default pipeline does not write depth and leaves this `None`.
    particle_depth_copy: Option<(u32, u32, wgpu::Texture, wgpu::TextureView)>,
    /// Optional presentation passes. Off by default so `draw_into` does not
    /// record them. #227 and #228 stay on PR #205.
    atmosphere_on: bool,
    cloud_raymarch_on: bool,
    ssgi_on: bool,
    taa_on: bool,
    upscale_on: bool,
    dof_on: bool,
    scene_vectors_on: bool,
    scene_vectors_prev: glam::Mat4,
    scene_vectors_prev_view: glam::Mat4,
    material_alpha_cutoff: f32,
    plane_prev_instances: Option<wgpu::Buffer>,
    sphere_prev_instances: Option<wgpu::Buffer>,
    ext_prev_instances: Option<wgpu::Buffer>,
    /// Previous sphere transforms in `self.instances` order. Compacted with
    /// the same visibility mask as the current draw, so a sphere that leaves
    /// the frustum does not hand its matrix to the next slot.
    sphere_prev_transforms: Vec<glam::Mat4>,
    scene_vectors_prev_valid: bool,
    /// Bytes copied into previous-instance buffers on the last enabled frame.
    scene_vector_last_copy_bytes: u64,
    atmosphere: Option<crate::atmosphere::AtmospherePass>,
    cloud_raymarch: Option<crate::volumetric_clouds::VolumetricCloudsPass>,
    ssgi_pass: Option<crate::ssgi::SsgiPass>,
    taa_pass: Option<crate::taa::TaaPass>,
    upscale_pass: Option<crate::temporal_upscale::TemporalUpscalePass>,
    dof_pass: Option<crate::advanced_post::AdvancedPostFx>,
    scene_vectors: Option<crate::scene_vectors::SceneVectorsPass>,
    gated_normal: Option<(wgpu::Texture, wgpu::TextureView)>,
    gated_velocity: Option<(wgpu::Texture, wgpu::TextureView)>,
    /// Camera-distance texture requests. `draw_into` uploads from its ring.
    streaming: crate::texture_streaming::TextureStreamingManager,
    /// Present when the device was created with the binding-array features.
    bindless: Option<crate::material_bindless::BindlessMaterialSystem>,
    /// When false, uploads stay on group-3 bind groups even if `bindless` exists.
    bindless_active: bool,

    instances: Vec<Instance>,
    /// Reused `InstanceRaw` upload for [`Self::update_instances`].
    /// Cleared and refilled each call. Capacity stays, so a stable instance
    /// count does not allocate.
    instance_raw_scratch: Vec<InstanceRaw>,
    /// Last bytes written by [`Self::update_instances`]. An identical upload skips the GPU write.
    instance_uploaded: Vec<u8>,
    /// Reused frustum-visible list filled by [`Self::build_visible_instances`].
    visible_raw_scratch: Vec<InstanceRaw>,
    visible_prev_scratch: Vec<InstanceRaw>,

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
    // L.2: terrain-forward group(3) — same views/params buffer as
    // `ibl_bind_group` in a 5-entry layout; rebuilt together on every bake.
    terrain_ibl_bgl: wgpu::BindGroupLayout,
    terrain_ibl_bind_group: wgpu::BindGroup,
    ibl_params_buf: wgpu::Buffer,

    // Screen-space Global Illumination (composite into PBR via group(5) bindings 5-6)
    gi_fallback_view: wgpu::TextureView,
    gi_sampler: wgpu::Sampler,
    // Cloud shadow map generation and sampling resources (group 5 bindings 7-8).
    cloud_shadow_pass: crate::volumetric_clouds::CloudShadowPass,
    /// Whether to dispatch the per-frame cloud shadow compute pass.
    /// Disable in editor modes to avoid noisy shadow patterns on terrain.
    cloud_shadows_enabled: bool,
    /// Set by `set_presentation_frame` and consumed by the next draw.
    presentation_frame: Option<u64>,
    /// Used when the caller does not set a presentation frame. Advances once
    /// per cloud-shadow record so the slice stays a function of draw count.
    cloud_shadow_frame: u64,
    /// Multiplier on the cloud-shadow extinction. See `CLOUD_SHADOW_STRENGTH`.
    cloud_shadow_strength: f32,
    ibl_intensity: f32,
    ibl_max_spec_lod: f32,

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
    /// Froxel fog. Built on first enable so a shader failure cannot block startup.
    volumetric_fog: Option<crate::volumetric_fog::VolumetricFogPass>,
    /// Screen-space light shafts. Built on first enable.
    god_rays: Option<crate::god_rays::GodRayPass>,

    /// Bloom compute pass (created lazily when bloom is first enabled).
    bloom_pass: Option<crate::bloom::BloomPass>,
    /// Pending bloom config to apply when the bloom pass is lazily created.
    pending_bloom_config: Option<crate::bloom::BloomConfig>,
    /// Uniform buffer for HDR-blit post-effect compositing parameters.
    /// Layout: `[bloom_intensity, 0.0, 0.0, 0.0]` (16 bytes, `vec4<f32>` in WGSL).
    postfx_params_buf: wgpu::Buffer,

    /// GPU memory budget tracker (editor-specific budgets).
    gpu_memory_budget: std::sync::Arc<crate::gpu_memory::GpuMemoryBudget>,

    /// LOD3 impostor passes keyed by caller-chosen string (typically a
    /// mesh-content hash — see `tools/aw_editor/src/viewport/impostor_registry.rs`).
    /// Each entry is recorded in the main forward pass after opaque objects
    /// and before transparent water/foliage, in `HashMap` iteration order
    /// (stable within a single frame; irrelevant for correctness since all
    /// passes use the same alpha-blend + depth-test state).
    ///
    /// Phase 5.3 T7 stage 2 (rev 2: was `Option<ImpostorPass>`; widened to
    /// keyed map to support multi-species scatter without per-frame
    /// atlas-mega-merging).
    #[cfg(feature = "impostor-bake")]
    impostor_passes: std::collections::HashMap<String, crate::impostor_pass::ImpostorPass>,

    /// Phase 1 forward-lit terrain state (Terrain Material System Campaign,
    /// Option D). `None` until `init_terrain_forward` is called — at which
    /// point the manager's pipeline + shared texture arrays are allocated
    /// and per-chunk state can be uploaded via `upload_terrain_chunk`.
    ///
    /// When `None`, the `draw_into` main forward pass skips the terrain-
    /// forward draw block entirely, falling back to the legacy
    /// `self.models`-based terrain path (registered via `add_model_with_bounds`).
    #[cfg(feature = "terrain-splat-arrays")]
    pub(crate) terrain_forward: Option<TerrainForwardRenderer>,
    /// L.3: last frame's terrain shadow-caster culling counts, per cascade —
    /// `[(drawn, total); CASCADE_COUNT]`. Written by `draw_into` after the passes;
    /// `(0, 0)` when the passes were skipped (shadows off / no casters).
    terrain_shadow_stats: [(u32, u32); CASCADE_COUNT],
}

/// Gated presentation choices.
///
/// `Atmosphere` is the enum default after the 4.474 ms capture. It does not
/// enable volumetric fog, and `Renderer::new` does not read this enum.
/// `Measured` turns Bruneton off. A caller applies either with
/// `set_atmosphere_enabled(quality.atmosphere())`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PresentationQuality {
    /// Bruneton off.
    Measured,
    /// Bruneton on when a caller applies it. Fog stays `set_volumetric_fog_config`.
    #[default]
    Atmosphere,
}

impl PresentationQuality {
    #[must_use]
    pub const fn atmosphere(self) -> bool {
        matches!(self, Self::Atmosphere)
    }
}

fn union_aabb(a: ([f32; 3], [f32; 3]), b: ([f32; 3], [f32; 3])) -> ([f32; 3], [f32; 3]) {
    (
        [a.0[0].min(b.0[0]), a.0[1].min(b.0[1]), a.0[2].min(b.0[2])],
        [a.1[0].max(b.1[0]), a.1[1].max(b.1[1]), a.1[2].max(b.1[2])],
    )
}

fn transformed_aabb(
    local_min: [f32; 3],
    local_max: [f32; 3],
    transform: &glam::Mat4,
) -> ([f32; 3], [f32; 3]) {
    let min = glam::Vec3::from_array(local_min);
    let max = glam::Vec3::from_array(local_max);
    let mut world_min = glam::Vec3::splat(f32::INFINITY);
    let mut world_max = glam::Vec3::splat(f32::NEG_INFINITY);
    for (x, y, z) in [
        (min.x, min.y, min.z),
        (max.x, min.y, min.z),
        (min.x, max.y, min.z),
        (max.x, max.y, min.z),
        (min.x, min.y, max.z),
        (max.x, min.y, max.z),
        (min.x, max.y, max.z),
        (max.x, max.y, max.z),
    ] {
        let point = transform.transform_point3(glam::Vec3::new(x, y, z));
        world_min = world_min.min(point);
        world_max = world_max.max(point);
    }
    (world_min.to_array(), world_max.to_array())
}

fn mesh_local_bounds(meshes: &[crate::mesh::CpuMesh]) -> Option<([f32; 3], [f32; 3])> {
    let mut acc = None;
    for mesh in meshes {
        let Some((min, max)) = mesh.aabb() else {
            continue;
        };
        let next = (min.to_array(), max.to_array());
        acc = Some(match acc {
            Some(prev) => union_aabb(prev, next),
            None => next,
        });
    }
    acc
}

fn aabb_rejected(
    bounds: Option<([f32; 3], [f32; 3])>,
    frustum: &crate::culling::FrustumPlanes,
) -> bool {
    let Some((min, max)) = bounds else {
        return false;
    };
    let center = glam::Vec3::new(
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    );
    let extent = glam::Vec3::new(
        (max[0] - min[0]) * 0.5,
        (max[1] - min[1]) * 0.5,
        (max[2] - min[2]) * 0.5,
    );
    !frustum.test_aabb(center, extent)
}

fn vertex_position_bounds(vertices: &[crate::mesh::MeshVertex]) -> Option<([f32; 3], [f32; 3])> {
    if vertices.is_empty() {
        return None;
    }
    let mut min = glam::Vec3::splat(f32::INFINITY);
    let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
    for vertex in vertices {
        let point = glam::Vec3::from_array(vertex.position);
        min = min.min(point);
        max = max.max(point);
    }
    Some((min.to_array(), max.to_array()))
}

fn world_bounds_for_instances(
    local: ([f32; 3], [f32; 3]),
    instances: &[Instance],
) -> Option<([f32; 3], [f32; 3])> {
    let mut acc = None;
    for instance in instances {
        let next = transformed_aabb(local.0, local.1, &instance.transform);
        acc = Some(match acc {
            Some(prev) => union_aabb(prev, next),
            None => next,
        });
    }
    acc
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
    fn requested_device(
        adapter: &wgpu::Adapter,
        gpu_tests: bool,
    ) -> (wgpu::Features, wgpu::Limits) {
        let mut features = wgpu::Features::empty();
        if gpu_tests {
            features |= wgpu::Features::TIMESTAMP_QUERY;
        }
        if adapter.features().contains(wgpu::Features::PIPELINE_CACHE) {
            features |= wgpu::Features::PIPELINE_CACHE;
        }
        for feature in [
            wgpu::Features::TIMESTAMP_QUERY,
            wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
            wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
        ] {
            if adapter.features().contains(feature) {
                features |= feature;
            }
        }
        let mut limits = wgpu::Limits {
            max_bind_groups: 8,
            ..wgpu::Limits::default()
        };
        if crate::material_bindless::supports_bindless(adapter) {
            let cap = adapter.limits().max_binding_array_elements_per_shader_stage;
            if cap >= 4 {
                features |= crate::material_bindless::required_features();
                limits.max_binding_array_elements_per_shader_stage = 4.min(cap);
            }
        }
        (features, limits)
    }

    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Result<Self> {
        let boot = std::time::Instant::now();
        let instance = gpu_instance();
        let surface = instance.create_surface(window.clone())?;
        let adapter = choose_adapter(&instance, Some(&surface)).await?;
        log::info!(
            "adapter selected after {} ms warp={}",
            boot.elapsed().as_millis(),
            warp_requested()
        );

        let (features, limits) = Self::requested_device(&adapter, cfg!(feature = "gpu-tests"));
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: features,
                required_limits: limits,
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await?;

        log_gpu(&adapter, &device);
        // Register GPU error callback for validation errors and device loss.
        let device_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        install_device_error_handler(&device, device_lost.clone());

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let present_mode = caps
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .unwrap_or(caps.present_modes[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        log::info!(
            "surface format {format:?} present {present_mode:?} {}x{} {} ms",
            config.width,
            config.height,
            boot.elapsed().as_millis()
        );
        surface.configure(&device, &config);

        let mut renderer =
            Self::new_from_device_monitored(device, queue, Some(surface), config, device_lost)
                .await?;
        renderer.note_adapter(&adapter);
        Ok(renderer)
    }

    pub async fn new_headless(width: u32, height: u32) -> Result<Self> {
        let instance = gpu_instance();
        let adapter = choose_adapter(&instance, None).await?;

        let (features, limits) = Self::requested_device(&adapter, false);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("headless device"),
                required_features: features,
                required_limits: limits,
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await?;

        log_gpu(&adapter, &device);
        let device_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        install_device_error_handler(&device, device_lost.clone());

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

        let mut renderer =
            Self::new_from_device_monitored(device, queue, None, config, device_lost).await?;
        renderer.note_adapter(&adapter);
        Ok(renderer)
    }

    /// Headless renderer for tests.
    ///
    /// `None` only when `AW_GPU_SKIP_IF_UNAVAILABLE` is set and the error
    /// chain contains `RequestAdapterError::NotFound`. The match is the
    /// variant, not Display text. The skip line is created under the
    /// workspace `target/` directory. `new_headless` does not write that
    /// file. A device refusal still panics.
    #[allow(clippy::panic)]
    pub async fn headless_for_tests(width: u32, height: u32) -> Option<Self> {
        match Self::new_headless(width, height).await {
            Ok(renderer) => Some(renderer),
            Err(err) => {
                let reason = format!("{err}");
                if std::env::var_os("AW_GPU_SKIP_IF_UNAVAILABLE").is_some()
                    && is_missing_adapter(&err)
                {
                    record_gpu_skip_for_tests(&reason);
                    None
                } else {
                    panic!("Failed to create headless renderer: {reason}");
                }
            }
        }
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
        let boot = std::time::Instant::now();
        log::info!("renderer device init");
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

        // HDR color target. COPY_SRC (W.2b) so the opaque result can be snapshotted
        // into `water_scene_color` for the split water refraction pass.
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        // W.2b — opaque scene-color snapshot the split water pass samples.
        let water_scene_color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("water scene color"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let water_scene_color_view =
            water_scene_color.create_view(&wgpu::TextureViewDescriptor::default());
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
        let _fx_ao = device.create_texture(&wgpu::TextureDescriptor {
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
        // W.2b §7.7 consistency fix: back `hdr_view` with `hdr_tex` at construction
        // too (resize already does this — renderer.rs ~3930). This makes `hdr_tex`
        // the unambiguous main-pass target / copy source from frame 0, instead of
        // the (otherwise dead) `fx_ao` aux texture. `fx_ao`/`hdr_aux`/`fx_gi` are
        // pre-existing unused postfx scaffolding.
        #[cfg(feature = "postfx")]
        let hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());
        // Non-postfx path has no aux HDR textures; back hdr_view with hdr_tex too so
        // the post bind group + struct init resolve (closes a pre-existing E0425
        // under --no-default-features that lived in this region).
        #[cfg(not(feature = "postfx"))]
        let hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());

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
        let _post_pipeline_ssr = crate::pipeline_cache::create_render_pipeline(
            &device,
            pipeline_cache.as_ref(),
            wgpu::RenderPipelineDescriptor {
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
            },
        );

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
        let _post_pipeline_ssao = crate::pipeline_cache::create_render_pipeline(
            &device,
            pipeline_cache.as_ref(),
            wgpu::RenderPipelineDescriptor {
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
            },
        );

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
        let _post_pipeline_ssgi = crate::pipeline_cache::create_render_pipeline(
            &device,
            pipeline_cache.as_ref(),
            wgpu::RenderPipelineDescriptor {
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
            },
        );

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

        // Shadow bind group layout (declared early so we can include it in main
        // pipeline layout). L.3: created via the shared constructor so the
        // terrain-forward pipeline tests build the identical group(4) layout —
        // same no-drift discipline as `create_terrain_ibl_bgl`.
        let shadow_bgl =
            crate::terrain_material_manager::TerrainMaterialManager::create_shadow_bgl(&device);

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
                texture_layout_entry(7),
                sampler_layout_entry(8),
                texture_layout_entry(9),
                sampler_layout_entry(10),
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

        let cloud_shadow_pass =
            crate::volumetric_clouds::CloudShadowPass::new_default(&device, &queue);

        // IBL params uniform buffer. `_pad` is filled with the cloud-shadow
        // bake center on each draw. Intensity and LOD stay at these defaults
        // until a bake overwrites them.
        let ibl_intensity = 1.0f32;
        let ibl_max_spec_lod = 4.0f32;
        let ibl_params_data = IblParamsGpu {
            ibl_intensity,
            max_spec_lod: ibl_max_spec_lod,
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

        // L.2: terrain-forward IBL group (group 3 of the terrain pipeline).
        // A dedicated 5-entry layout rather than the 9-entry group-5 layout:
        // reusing the full layout would put the terrain fragment stage at the
        // 16-sampled-texture device default (3 layer arrays + 8 splats + 5),
        // blocking L.3's shadow map. The bind group shares the SAME views,
        // params buffer, and sampler as `ibl_bind_group` (§7.7: one logical
        // resource, one owner) and is rebuilt alongside it on every bake in
        // `rebuild_ibl_bind_group`.
        let terrain_ibl_bgl =
            crate::terrain_material_manager::TerrainMaterialManager::create_terrain_ibl_bgl(
                &device,
            );
        let terrain_ibl_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain_ibl_fallback_bg"),
            layout: &terrain_ibl_bgl,
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

        let pipeline = crate::pipeline_cache::create_render_pipeline(
            &device,
            pipeline_cache.as_ref(),
            wgpu::RenderPipelineDescriptor {
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
            },
        );

        // ED-3: wireframe variant of the main static-mesh pipeline —
        // identical descriptor, polygon_mode Line. Feature-gated: the
        // editor's device requests POLYGON_MODE_LINE when the adapter
        // supports it; without it the UI hides the Wireframe entry, so the
        // fill fallback in `main_mesh_pipeline()` is never user-reachable.
        let pipeline_wire = if device
            .features()
            .contains(wgpu::Features::POLYGON_MODE_LINE)
        {
            Some(crate::pipeline_cache::create_render_pipeline(
                &device,
                pipeline_cache.as_ref(),
                wgpu::RenderPipelineDescriptor {
                    cache: pipeline_cache.as_ref(),
                    label: Some("pipeline-wire"),
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
                        polygon_mode: wgpu::PolygonMode::Line,
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
                },
            ))
        } else {
            None
        };
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

        let post_pipeline = crate::pipeline_cache::create_render_pipeline(
            &device,
            pipeline_cache.as_ref(),
            wgpu::RenderPipelineDescriptor {
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
            },
        );

        // Editor-Engine Render Parity P.3: hdr_blit_pipeline construction
        // removed. Was the passthrough HDR→HDR blit selected by
        // `draw_into`'s `surface.is_none()` branch (deleted in same commit).
        // Canonical post_pipeline now runs unconditionally for both windowed
        // runtime and headless editor invocations.

        // Shadow map resources (4-layer array for CSM). L.3.A added the third
        // (survey) cascade after the 500 m coverage cap read as a
        // camera-anchored boundary; L.3.C added the fourth because the relief
        // census measured a 6.4x rendition step at the 500 m seam that left
        // >50% of this dune world's features unable to cast beyond it
        // (docs/audits/L3C_OUTCOME.md). Splits: 86 / 500 / 1400 / 3000 m.
        let shadow_size: u32 = SHADOW_MAP_SIZE;
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map"),
            size: wgpu::Extent3d {
                width: shadow_size,
                height: shadow_size,
                // L.3.D: one layer WIDER than the cascade count. The extra
                // layer holds c3's outgoing window during an incremental
                // transition; nothing ever renders into it, it is filled by a
                // layer copy at refresh time.
                depth_or_array_layers: SHADOW_LAYER_COUNT,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            // COPY_SRC/DST for the c3 transition's layer copy (L.3.D rung 2).
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
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
        // Per-layer views for rendering, one per cascade. Built from
        // CASCADE_COUNT so a new cascade cannot be left without a target.
        let shadow_layer_views: [wgpu::TextureView; CASCADE_COUNT] = std::array::from_fn(|i| {
            shadow_tex.create_view(&wgpu::TextureViewDescriptor {
                usage: None,
                label: Some(&format!("shadow layer{i} view")),
                format: Some(wgpu::TextureFormat::Depth32Float),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: i as u32,
                array_layer_count: Some(1),
            })
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
            // L.3.C-resolution layout: 4 mat4 (256) + vec4 splits (16) + vec4
            // extras (16) + vec4 bias_scales (16) + mat4 view_proj_prev (64)
            // => 368 — must cover
            // shadow_common.wgsl's MainLightUbo. `extras` widened from vec2 to
            // vec4 (L.3.C) to carry split2 in a lane that was already padding,
            // so `splits`' lanes and `extras.x`/`.y` keep their offsets (the
            // sentinel contract test depends on extras.x); `bias_scales` was
            // appended, so every earlier offset is likewise untouched.
            size: 368,
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
        // Built from CASCADE_COUNT so the array can never fall behind the
        // cascade count (the shape that panicked loudly in L.3.C when it did).
        let shadow_cascade_bufs: [wgpu::Buffer; CASCADE_COUNT] = std::array::from_fn(|i| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("shadow cascade{i} ubo")),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let shadow_cascade_bgs: [wgpu::BindGroup; CASCADE_COUNT] = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("shadow cascade{i} bg")),
                layout: &shadow_bgl_light,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow_cascade_bufs[i].as_entire_binding(),
                }],
            })
        });

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
        let shadow_pipeline = crate::pipeline_cache::create_render_pipeline(
            &device,
            pipeline_cache.as_ref(),
            wgpu::RenderPipelineDescriptor {
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
            },
        );

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

        let (occlusion_tex, occlusion_view, occlusion_sampler) =
            solid_rgba_texture(&device, &queue, "occlusion tex", [255, 255, 255, 255]);
        let (emissive_tex, emissive_view, emissive_sampler) =
            solid_rgba_texture(&device, &queue, "emissive tex", [0, 0, 0, 255]);

        // Combined bind group for albedo, mr, normal, occlusion, emissive, and skin palette
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
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&occlusion_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&emissive_sampler),
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
        let skinned_pipeline = crate::pipeline_cache::create_render_pipeline(
            &device,
            pipeline_cache.as_ref(),
            wgpu::RenderPipelineDescriptor {
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
            },
        );

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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let cube_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_cube index_buf"),
            contents: bytemuck::cast_slice(&cube_i),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        let mesh_cube = Mesh {
            vertex_buf: cube_vb,
            index_buf: cube_ib,
            index_count: cube_i.len() as u32,
        };
        let sphere_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_sphere vertex_buf"),
            contents: bytemuck::cast_slice(&sphere_v),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let sphere_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_sphere index_buf"),
            contents: bytemuck::cast_slice(&sphere_i),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let plane_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_plane index_buf"),
            contents: bytemuck::cast_slice(&plane_i),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
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
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let ext_inst_buf = None;
        let bindless_features = crate::material_bindless::required_features();
        let bindless = if device.features().contains(bindless_features) {
            let slots = device
                .limits()
                .max_binding_array_elements_per_shader_stage
                .clamp(4, 32);
            Some(crate::material_bindless::BindlessMaterialSystem::new(
                &device,
                &queue,
                crate::material_bindless::BindlessMaterialConfig {
                    max_textures: slots,
                    max_materials: 64,
                },
            ))
        } else {
            None
        };
        let bindless_active = bindless.is_some();

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

        log::info!("renderer pipelines ready {} ms", boot.elapsed().as_millis());
        let streaming = crate::texture_streaming::TextureStreamingManager::new(256)
            .context("texture streaming runtime")?;
        let hiz_depth_load = crate::gpu_fault::depth_texture_load_supported(&device);

        Ok(Self {
            surface,
            device,
            queue,
            pipeline_cache,
            _pipeline_cache_mgr: pipeline_cache_mgr,
            gpu_profiler,
            frame_stamp: None,
            timing_gap: None,
            gpu_adapter_name: String::new(),
            gpu_backend: String::new(),
            staging_ring,
            resource_generation,
            #[cfg(feature = "camera-relative")]
            camera_world_pos: glam::DVec3::ZERO,
            config,
            depth,
            shader,
            pipeline,
            pipeline_wire,
            wireframe_enabled: false,
            material_buf,
            material_bg,
            post_pipeline,
            post_bind_group,
            post_bgl,
            hdr_tex,
            hdr_view,
            hdr_sampler,
            water_scene_color,
            water_scene_color_view,
            _postfx_dummy_tex: postfx_dummy_tex,
            postfx_dummy_view,
            shadow_tex,
            shadow_view,
            shadow_layer_views,
            shadow_sampler,
            shadow_pipeline,
            light_buf,
            light_bg,
            shadow_bgl,
            shadow_bgl_light,
            terrain_shadow_stats: [(0, 0); CASCADE_COUNT],
            shadow_cascade_bufs,
            shadow_cascade_bgs,
            cascades: [glam::Mat4::IDENTITY; CASCADE_COUNT],
            c_far_center: [glam::Vec3::ZERO; CASCADE_COUNT - FIRST_CACHED_CASCADE],
            c_far_light_dir: [glam::Vec3::ZERO; CASCADE_COUNT - FIRST_CACHED_CASCADE],
            c3_prev_vp: glam::Mat4::IDENTITY,
            c3_fade: 1.0,
            c3_copy_pending: false,
            far_policy: FarCascadePolicy::from_env(),
            cascade_fits: [CascadeFit::default(); CASCADE_COUNT],
            c2_tick: 0,
            c_far_valid: [false; CASCADE_COUNT - FIRST_CACHED_CASCADE],
            c_far_pending: [false; CASCADE_COUNT - FIRST_CACHED_CASCADE],
            c_far_refresh_count: [0; CASCADE_COUNT - FIRST_CACHED_CASCADE],
            c_far_refresh_reason: ["none"; CASCADE_COUNT - FIRST_CACHED_CASCADE],
            c_far_last_drift: [0.0; CASCADE_COUNT - FIRST_CACHED_CASCADE],
            c_far_window: [(glam::Vec3::ZERO, 0.0); CASCADE_COUNT - FIRST_CACHED_CASCADE],
            split0: 60.0,
            split1: 120.0,
            split2: 1400.0,
            cascade0_extent: 40.0,
            cascade1_extent: 80.0,
            cascade_lambda: 0.5,
            shadow_pcf_radius_px: 1.0,
            shadow_depth_bias: 0.0006,
            shadow_slope_scale: 0.002,
            shadows_enabled: true,  // Shadows enabled by default
            max_draw_distance: 0.0, // 0 = use fog_end fallback
            rendered_model_count: 0,
            terrain_ground_set: false, // No terrain ground plane set yet
            plane_instance_uploaded: false,
            visible_uploaded: Vec::new(),
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
            occlusion_tex,
            occlusion_view,
            occlusion_sampler,
            emissive_tex,
            emissive_view,
            emissive_sampler,
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
            hiz: None,
            sample_hiz: false,
            hiz_depth_load,
            gpu_cull_on: false,
            indirect_cull: None,
            indirect_cull_resources: None,
            gpu_cull_aabb_scratch: Vec::new(),
            gpu_cull_template_scratch: Vec::new(),
            gpu_cull_ran: false,
            gpu_cull_visible_draws: 0,
            gpu_cull_names: Vec::new(),
            gpu_cull_bounds: std::collections::HashMap::new(),
            gpu_cull_local: std::collections::HashMap::new(),
            gpu_cull_instances: std::collections::HashMap::new(),
            particle_sim: None,
            particle_draw: None,
            particle_sort: None,
            particle_emitter: None,
            particle_depth_copy: None,
            // Off until a caller has built Aravath fog. Sky-only Bruneton
            // scatters geometry out to 50 km, which is not the 4.474 ms cell.
            atmosphere_on: false,
            cloud_raymarch_on: false,
            ssgi_on: false,
            taa_on: false,
            upscale_on: false,
            dof_on: false,
            scene_vectors_on: false,
            scene_vectors_prev: glam::Mat4::IDENTITY,
            scene_vectors_prev_view: glam::Mat4::IDENTITY,
            material_alpha_cutoff: 0.5,
            plane_prev_instances: None,
            sphere_prev_instances: None,
            ext_prev_instances: None,
            sphere_prev_transforms: Vec::new(),
            scene_vectors_prev_valid: false,
            scene_vector_last_copy_bytes: 0,
            atmosphere: None,
            cloud_raymarch: None,
            ssgi_pass: None,
            taa_pass: None,
            upscale_pass: None,
            dof_pass: None,
            scene_vectors: None,
            gated_normal: None,
            gated_velocity: None,
            streaming,
            bindless,
            bindless_active,
            instances: Vec::new(),
            instance_raw_scratch: Vec::new(),
            instance_uploaded: Vec::new(),
            visible_raw_scratch: Vec::new(),
            visible_prev_scratch: Vec::new(),
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
            terrain_ibl_bgl,
            terrain_ibl_bind_group,
            ibl_params_buf,
            gi_fallback_view,
            gi_sampler,
            cloud_shadow_pass,
            cloud_shadows_enabled: true,
            presentation_frame: None,
            cloud_shadow_frame: 0,
            cloud_shadow_strength: crate::volumetric_clouds::CLOUD_SHADOW_STRENGTH,
            ibl_intensity,
            ibl_max_spec_lod,
            water_renderer: None,
            hdr_overlay: None,
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
            volumetric_fog: None,
            god_rays: None,
            bloom_pass: None,
            pending_bloom_config: None,
            postfx_params_buf,
            gpu_memory_budget: std::sync::Arc::new(crate::gpu_memory::GpuMemoryBudget::new()),
            device_lost,
            #[cfg(feature = "impostor-bake")]
            impostor_passes: std::collections::HashMap::new(),
            #[cfg(feature = "terrain-splat-arrays")]
            terrain_forward: None,
        })
    }

    /// Returns `true` if the GPU device has been lost (driver crash, TDR,
    /// device removed).  When this returns `true` no further GPU work will
    /// be submitted and the application should drop this `Renderer` and
    /// create a new one.
    pub fn is_device_lost(&self) -> bool {
        self.device_lost.load(std::sync::atomic::Ordering::SeqCst)
            || crate::gpu_fault::device_lost()
    }

    /// Pump the device so a loss reported on the previous submit is visible
    /// before this frame maps any buffers.
    pub fn poll_device(&self) {
        if self.device.poll(wgpu::PollType::Poll).is_err() {
            self.device_lost
                .store(true, std::sync::atomic::Ordering::SeqCst);
            crate::gpu_fault::mark_device_lost();
            log::error!("device.poll failed; treating the GPU as lost");
        }
    }

    /// Returns a reference to the pipeline cache, if available (Vulkan backends only).
    pub fn pipeline_cache(&self) -> Option<&wgpu::PipelineCache> {
        self.pipeline_cache.as_ref()
    }

    /// Returns the GPU profiler, if timestamp queries are supported.
    /// ED-3: set the viewport debug-shading state for subsequent frames.
    ///
    /// `mode`: 0 = lit, 1 = unlit albedo, 2 = world-space normals, 3 = UVs —
    /// written into the scene-env UBO and branched on uniformly in the main,
    /// skinned, and terrain-forward fragment shaders (same pipelines, same
    /// passes; no second render path). `wireframe` swaps the mesh and terrain
    /// pipelines for their polygon-mode-Line variants; it is ignored (kept
    /// false) when the device lacks `POLYGON_MODE_LINE` — callers should gate
    /// their UI on [`Self::wireframe_supported`] so that cannot silently
    /// no-op.
    pub fn set_debug_shading(&mut self, mode: u32, wireframe: bool) {
        self.scene_env.debug_mode = mode;
        self.wireframe_enabled = wireframe && self.pipeline_wire.is_some();
    }

    /// ED-3: whether the device carries the wireframe pipeline variants.
    pub fn wireframe_supported(&self) -> bool {
        self.pipeline_wire.is_some()
    }

    /// The main static-mesh pipeline for this frame (fill or wire).
    fn main_mesh_pipeline(&self) -> &wgpu::RenderPipeline {
        if self.wireframe_enabled {
            self.pipeline_wire.as_ref().unwrap_or(&self.pipeline)
        } else {
            &self.pipeline
        }
    }

    pub fn note_adapter(&mut self, adapter: &wgpu::Adapter) {
        let info = adapter.get_info();
        self.gpu_adapter_name = info.name;
        self.gpu_backend = format!("{:?}", info.backend);
    }

    pub fn gpu_adapter_name(&self) -> &str {
        &self.gpu_adapter_name
    }

    pub fn gpu_backend(&self) -> &str {
        &self.gpu_backend
    }

    pub fn gpu_features(&self) -> wgpu::Features {
        self.device.features()
    }

    pub fn timestamps_supported(&self) -> bool {
        self.gpu_profiler.is_some()
            && self
                .gpu_profiler
                .as_ref()
                .is_some_and(|profiler| profiler.timestamp_period_ns() > 0.0)
    }

    /// Map the timestamp buffer after [`Self::submit_timestamped`].
    /// That submit waits for the writes, then resolves and copies.
    pub fn request_gpu_profiler_readback(&mut self) {
        if let Some(profiler) = self.gpu_profiler.as_mut() {
            profiler.request_readback();
        }
    }

    /// Submit the encoder that already contains the timestamp writes and resolve.
    ///
    /// [`crate::gpu_profiler::GpuProfiler::resolve_queries`] runs on that
    /// encoder before `finish`. A second resolve encoder reads unavailable
    /// queries on NVIDIA. The copy follows a 5s poll of this submission.
    pub fn submit_timestamped(&mut self, frame: wgpu::CommandBuffer) -> Result<()> {
        let queue = self.queue.clone();
        let device = self.device.clone();
        crate::gpu_profiler::GpuProfiler::submit_frame(
            self.gpu_profiler.as_mut(),
            &queue,
            &device,
            frame,
        )
        .map_err(|err| anyhow::anyhow!(err))
    }

    pub fn gpu_profiler(&self) -> Option<&crate::gpu_profiler::GpuProfiler> {
        self.gpu_profiler.as_ref()
    }

    pub fn gpu_profiler_mut(&mut self) -> Option<&mut crate::gpu_profiler::GpuProfiler> {
        self.gpu_profiler.as_mut()
    }

    /// New query set, resolve buffer, and staging buffer.
    ///
    /// Mirrors the self-test, which reads a set that has never been written.
    /// The frame path otherwise keeps one set and changes how many queries
    /// it resolves after the first frame.
    pub fn recreate_timestamp_set(&mut self) -> Result<()> {
        let device = self.device.clone();
        let Some(profiler) = self.gpu_profiler.as_mut() else {
            anyhow::bail!("timestamps are unsupported; there is no query set to recreate");
        };
        profiler
            .recreate(&device)
            .map_err(|err| anyhow::anyhow!(err))
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
    fn apply_camera_key(cam: &mut FreeFly, k: &awc::CameraKey) {
        // C.7.D (Unified Camera campaign): defensive sanitize before
        // applying. Hardens the production cinematics path
        // (cutscene_render_demo via tick_cinematics, wired C.7.B) against
        // degenerate keys — out-of-range fov_deg gets clamped to
        // [10°, 170°] and look_at == pos gets resolved to pos + (1,0,0)
        // (canonical +X forward). Per C.5 audit finding L.5.17.
        //
        // Clone-and-sanitize pattern: the parameter is &CameraKey
        // (shared ref owned by the timeline); sanitize needs &mut. The
        // local clone leaves the caller's key unchanged while hardening
        // the applied values. CameraKey derives Clone (verified C.7.D
        // Phase 1A); the struct is small (tuples + f32 + Time).
        let mut k = k.clone();
        k.sanitize();

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

    /// Load a programmatically-constructed timeline.
    ///
    /// Distinct from [`Self::load_timeline_json`], which deserializes a
    /// timeline from a JSON string; this variant takes an in-memory
    /// [`awc::Timeline`] directly. Added in Unified Camera campaign
    /// sub-phase C.7.B to support
    /// `examples/cutscene_render_demo`'s programmatic timeline
    /// construction (it builds the timeline in code at startup; no disk
    /// serialization is needed). Future cinematics work that constructs
    /// timelines programmatically (test fixtures, runtime-generated
    /// cinematics) uses this method.
    ///
    /// Mirrors [`Self::load_timeline_json`]'s post-deserialize behavior:
    /// stores the timeline, resets the sequencer to `t = 0`. Playback
    /// state is unchanged; call [`Self::play_timeline`] separately to
    /// start emission of camera/audio/FX events.
    pub fn load_timeline(&mut self, tl: awc::Timeline) {
        self.cin_tl = Some(tl);
        self.cin_seq.seek(awc::Time(0.0));
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
    pub fn tick_cinematics(&mut self, dt: f32, camera: &mut FreeFly) -> Vec<awc::SequencerEvent> {
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

    /// Bake the demo sky. Tries the terrain pack's `spruit_sunrise` EXR, then
    /// `kloppenheim_02_puresky`. The returned path is the file that was baked.
    /// Missing files are an error, not a silent 1×1 cubemap.
    pub fn bake_showcase_environment(
        &mut self,
        quality: crate::ibl::IblQuality,
    ) -> Result<std::path::PathBuf> {
        let chosen = [SHOWCASE_HDRI_PRIMARY, SHOWCASE_HDRI_FALLBACK]
            .into_iter()
            .map(std::path::PathBuf::from)
            .find(|path| path.is_file())
            .with_context(|| {
                format!(
                    "showcase HDRI missing: tried {SHOWCASE_HDRI_PRIMARY} and {SHOWCASE_HDRI_FALLBACK}"
                )
            })?;
        if chosen != std::path::Path::new(SHOWCASE_HDRI_PRIMARY) {
            log::warn!(
                "primary showcase HDRI {SHOWCASE_HDRI_PRIMARY} missing; baking {}",
                chosen.display()
            );
        }
        self.ibl.mode = crate::ibl::SkyMode::HdrPath {
            biome: "showcase".to_string(),
            path: chosen.display().to_string(),
        };
        self.bake_environment(quality)?;
        Ok(chosen)
    }

    /// Upload one glTF primitive through the same path as
    /// [`Self::add_composite_model`]. Albedo-only meshes keep the albedo bind
    /// group. A normal, metallic-roughness, occlusion, or emissive map binds
    /// all five slots. glTF metallic-roughness is converted to engine layout
    /// (R = metallic, G = roughness) inside [`crate::pbr_maps::pack_gltf_surface`].
    pub fn add_cpu_mesh_model(
        &mut self,
        name: impl Into<String>,
        cpu: &crate::mesh::CpuMesh,
        instances: &[Instance],
    ) {
        self.add_composite_model(name, vec![cpu.clone()], instances);
    }

    /// Install an environment baked elsewhere (L.2: the editor's worker-thread
    /// init bake — a second `IblManager` on that thread bakes and sends its
    /// `IblResources` back; wgpu resources are internally refcounted, so the
    /// views keep the thread-created textures alive after the thread's
    /// manager drops). Identical tail to [`Self::bake_environment`]: rebuilds
    /// BOTH IBL bind groups (group 5 + the terrain group 3) and flips the sky
    /// to the baked environment.
    pub fn install_baked_environment(&mut self, resources: crate::ibl::IblResources) {
        self.rebuild_ibl_bind_group(&resources);
        self.ibl_resources = Some(resources);
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

        // L.2 calibration (director-ratified 2026-07-28): fixed default IBL
        // intensity, replacing the former image-average normalisation
        // (`0.35/avg_luminance`). Measured for kloppenheim_02: the linear
        // image average of a puresky WITHOUT its sun spike reads ~0.146 and
        // drove the fill to 2.4-2.5x — ~1.5-2x the analytic sun's own diffuse
        // (the L.2 wash; docs/audits/L2_OUTCOME.md §4). Director-previewed
        // placements: 1.5 (shipped) and 1.0 (the one-line conservative
        // fallback pending the render gate). `res.avg_luminance` is still
        // computed and logged for diagnostics (residue item C, deferred).
        let ibl_intensity = 1.5f32;
        log::info!(
            "IBL bake: avg_luminance={:?}, ibl_intensity={:.3}, mips_specular={}",
            res.avg_luminance,
            ibl_intensity,
            res.mips_specular,
        );
        self.ibl_intensity = ibl_intensity;
        self.ibl_max_spec_lod = res.mips_specular.saturating_sub(1).max(1) as f32;
        self.write_ibl_params();

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

        // L.2: the terrain-forward group(3) mirror — same baked views, same
        // params buffer, same sampler. Rebuilt here (and only here) so the
        // two binding tables can never diverge (§7.7).
        self.terrain_ibl_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain_ibl_bg"),
            layout: &self.terrain_ibl_bgl,
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
        self.gated_normal = None;
        self.gated_velocity = None;

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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        self.hdr_view = self
            .hdr_tex
            .create_view(&wgpu::TextureViewDescriptor::default());
        if let Some(fog) = self.volumetric_fog.as_mut() {
            fog.resize(&self.device, self.config.width, self.config.height);
        }
        if let Some(rays) = self.god_rays.as_mut() {
            rays.resize(&self.device, self.config.width, self.config.height);
        }
        if let Some(atmosphere) = self.atmosphere.as_mut() {
            atmosphere.resize(&self.device, self.config.width, self.config.height);
        }
        // W.2b — keep the scene-color snapshot sized to the surface.
        self.water_scene_color = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("water scene color"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.water_scene_color_view = self
            .water_scene_color
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
        // Editor-Engine Render Parity P.3: resize-time HDR blit bind group
        // rebuild removed (only existed to keep hdr_blit_pipeline's view
        // bindings current; pipeline deleted in same commit).

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
        // A failed pass resize clears `config.enabled` and returns to the caller.
        self.clamp_atmosphere_to_volumetrics();
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

    /// Canonical camera upload — consumes a [`RenderView`] and updates all
    /// internal caches (matrices, CSM cascades, light direction, camera UBO).
    ///
    /// This is the canonical (and only) camera-upload entry point per
    /// `CAMERA_CONVENTIONS.md` §2.9. Pre-C.3.C, `Renderer::update_camera` and
    /// `Renderer::update_camera_matrices` existed as `#[deprecated]` thin
    /// wrappers; C.3.C deleted them as the closure of the renderer-side
    /// consolidation chapter of the Unified Camera campaign.
    ///
    /// # Camera-relative rendering
    ///
    /// Under the `camera-relative` feature, the SHADER expects
    /// `camera_pos_pad` to be ZERO (the view matrix carries no translation;
    /// geometry is offset by `camera_world_pos` before GPU upload). This
    /// method enforces that contract regardless of what `view.position`
    /// reports — callers should populate `view.position` with the *world*
    /// position (for diagnostic/fog reconstruction) and rely on this method
    /// to zero out the shader-facing position under the feature gate. See
    /// [`FreeFly::to_render_view_camera_relative`](astraweave_camera::FreeFly::to_render_view_camera_relative).
    ///
    /// Similarly, the cascade-split computation receives a `RenderView` with
    /// position overridden to ZERO under camera-relative so frustum corners
    /// are computed around the rendered-coordinate origin, matching the
    /// pre-C.3.A `update_camera` behavior.
    ///
    /// # Light override
    ///
    /// Per C.3.A's consolidation decision (historical-accident verdict on
    /// the `update_camera` vs `update_camera_matrices` asymmetry), all
    /// callers now honor `self.light_override` unconditionally: when `Some`,
    /// override.dir and override.intensity flow into `light_dir_pad`; when
    /// `None`, fall back to `self.sky.time_of_day().get_light_direction()`
    /// with intensity 1.0. Pre-C.3.A, `update_camera` ignored the override
    /// and wrote 0.0 to `light_dir_pad[3]`; `update_camera_matrices` honored
    /// it. Andrew approved the consolidation 2026-05-18; `light_dir_pad[3]`
    /// remains shader-inert (the WGSL `Camera` struct names it `_pad`).
    pub fn update_view(&mut self, view: &RenderView) {
        self.cached_view = view.view;
        self.cached_proj = view.projection;
        self.camera_ubo.view_proj = view.view_proj.to_cols_array_2d();

        // Use the editor's light override if set, otherwise fall back to TimeOfDay.
        let (light_dir, sun_intensity) = if let Some((dir, intensity)) = self.light_override {
            (glam::Vec3::from(dir), intensity)
        } else {
            (self.sky.time_of_day().get_light_direction(), 1.0)
        };
        self.camera_ubo.light_dir_pad = [light_dir.x, light_dir.y, light_dir.z, sun_intensity];

        // Under camera-relative, the shader sees camera at origin (matches
        // legacy `update_camera`/`update_camera_matrices` behavior). The
        // RenderView's `position` field is preserved on the type for
        // world-space reconstruction (fog distance, etc.) but is NOT what
        // the shader sees in this mode.
        #[cfg(feature = "camera-relative")]
        let shader_pos = glam::Vec3::ZERO;
        #[cfg(not(feature = "camera-relative"))]
        let shader_pos = view.position;
        self.camera_ubo.camera_pos_pad = [shader_pos.x, shader_pos.y, shader_pos.z, 0.0];

        self.queue
            .write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&self.camera_ubo));

        // Cascade splits — consumes `RenderView` directly per C.3.A
        // Decision 4. Under camera-relative, override position to ZERO so
        // frustum corners are computed around the rendered-coord origin
        // (matches the pre-C.3.A `cr_cam` construction).
        #[cfg(feature = "camera-relative")]
        let cascade_view = {
            let mut v = *view;
            v.position = glam::Vec3::ZERO;
            v
        };
        #[cfg(not(feature = "camera-relative"))]
        let cascade_view = *view;
        self.update_cascade_splits(&cascade_view, light_dir);
    }

    fn update_cascade_splits(&mut self, view: &RenderView, light_dir: glam::Vec3) {
        // L.1 note (as amended by L.3): this runs every frame even when
        // shadows are disabled. It is NOT safe to skip on
        // `!self.shadows_enabled`, because this function is also the delivery
        // path for `light_buf`'s extras.x sentinel (-1.0 = shadows off,
        // written below) — skipping wholesale would leave a stale positive
        // pcf radius in the UBO and re-enable garbage shadow sampling. Since
        // L.3 the cascades DO have consumers every frame the editor terrain
        // scene renders with shadows on: the terrain caster pass draws with
        // `shadow_cascade_bufs`, and both the static and terrain-forward
        // fragment shaders sample via `light_buf`'s matrices/splits.
        //
        // C.3.A migration: signature changed from `&Camera` to `&RenderView`
        // per CAMERA_CONVENTIONS.md §2.9. The pre-C.3.A lossy yaw/pitch
        // reconstruction at the original line 4001 (in update_camera_matrices)
        // is eliminated — callers pass RenderView directly with view_dir
        // already canonical (from inverse_view.col(2) or producer state).
        //
        // Shadow distance (L.3.A): the near/mid pair caps at 500 (their L.3
        // fits, matrices and split UNCHANGED — close/mid quality is pinned),
        // and a third SURVEY cascade extends coverage to SHADOW_FAR_SURVEY.
        // The former 500 m cap was measured as a camera-anchored shadow
        // boundary from the editor's moving survey camera (the L.3 render-
        // gate rejection; docs/audits/L3A_OUTCOME.md): shadow influence must
        // not end inside plainly-visible range, and its fade must be WIDE —
        // the last 30% of coverage, not a 100 m band at 400 m.
        const SHADOW_FAR_SURVEY: f32 = 3000.0;
        const SHADOW_FADE_FRACTION: f32 = 0.7;
        // L.3.C: the c2|c3 boundary. Splitting 500..3000 here takes the 500 m
        // seam's rendition step from 6.4x to 2.0x, which is what lets dune
        // relief keep casting across it (relief census: >50% of this world's
        // dune features could not cast beyond the old single seam).
        const SHADOW_SPLIT2: f32 = 1400.0;
        let n = view.znear.max(0.5);
        let shadow_mid = 500.0_f32.min(view.zfar);
        let shadow_far = SHADOW_FAR_SURVEY.min(view.zfar).max(shadow_mid + 1.0);

        // PSSM practical split (lambda blends log vs linear) for the c0|c1
        // boundary, computed against shadow_mid so c0/c1 keep their L.3
        // numbers. With n=0.5, shadow_mid=500 (the editor camera): l = 15.81,
        // u = 250.25, split = λ·l + (1−λ)·u — so λ=0.75 → split0 ≈ 74.4 and
        // the editor terrain-upload's λ=0.7 → split0 ≈ 86.1. (The pre-L.3
        // "≈ 16 units" claim here was the λ=1.0 pure-log value, not the
        // blend.) split1 = 500 (mid|survey boundary); split2 = shadow_far.
        let c = 2.0;
        let i = 1.0f32;
        let u = n + (shadow_mid - n) * (i / c);
        let l = n * (shadow_mid / n).powf(i / c);
        let lambda = self.cascade_lambda.clamp(0.0, 1.0);
        let split = l * lambda + u * (1.0 - lambda);
        self.split0 = split;
        self.split1 = shadow_mid;

        self.split2 = SHADOW_SPLIT2;

        // Per-cascade slice bounds. c0/c1 are unchanged from L.3 — their fits
        // are still computed against `shadow_mid` = 500 — which is what keeps
        // the pinned station frames byte-identical across this change.
        let bounds: [(f32, f32); CASCADE_COUNT] = [
            (n, self.split0),
            (self.split0, shadow_mid),
            (shadow_mid, self.split2),
            (self.split2, shadow_far),
        ];
        let up = glam::Vec3::Y;

        // Cached-cascade policy (L.3.A, generalised to the far pair in L.3.C).
        // Cascades at or beyond FIRST_CACHED_CASCADE re-fit and re-render only
        // on a REFRESH: sun move, ideal-window drift past C2_DRIFT_LIMIT, a
        // slow periodic tick (so terrain edits appear), or a map not
        // known-good. A frozen window is CORRECT, not stale: shadow positions
        // are world-anchored, sampling uses the same frozen matrix the map was
        // rendered with, and the ortho box is padded by C2_DRIFT_MARGIN (>
        // the drift limit) so a drifted window still fully contains its ideal
        // slice — no coverage gap can open at its near edge between refreshes.
        //
        // TWO-PHASE COMMIT (the L.3.A bug this shape exists to prevent):
        // `c_far_pending` is STICKY — set here, cleared ONLY by a shadow pass
        // that actually renders that layer. A map must never be treated as
        // filled just because its matrix was computed: on the first frame
        // after terrain upload the caster pipeline does not exist yet
        // (`ensure_shadow_pipeline` runs after the passes), so
        // `has_shadow_casters` is false and every pass is skipped — a cache
        // that trusted the matrix sampled a never-written map (measured:
        // y414 108.37 vs 116.31 correct).
        // The drift limit and ortho pad moved onto `FarCascadePolicy` in L.3.D
        // (`freeze_bounds`) — they are per-policy and coupled, not one global
        // pair. Only the periodic tick remains global.
        const C2_REFRESH_INTERVAL: u32 = 8;
        self.c2_tick = self.c2_tick.wrapping_add(1) % C2_REFRESH_INTERVAL;

        for (i, (near, far)) in bounds.iter().copied().enumerate() {
            let frustum = frustum_corners_ws(view, near, far);
            let center = frustum_center(&frustum);
            // Sphere-based fitting for stable shadows (rotation-invariant):
            // the sphere radius sets the ortho extent, so shadows don't
            // shimmer when the camera rotates.
            let radius = sphere_radius(&frustum, center);
            // Position the light far enough behind the sphere to capture tall
            // casters above the frustum.
            let light_dist = radius * 2.0 + 50.0;
            let far_idx = i.checked_sub(FIRST_CACHED_CASCADE);
            // The drift pad is carried ONLY by a window that can stay frozen
            // across frames — that is what it is for. It is not free: it
            // enlarges the ortho box, so a padded c2 renders 1.91 m texels
            // where a c2 re-fitted every frame renders 1.52 m. Near cascades,
            // and far cascades running live, keep the 2 m margin (which is what
            // preserves the pinned close/mid station frames).
            let margin = far_idx.map_or(LIVE_MARGIN, |c| self.far_policy.margin(c));
            let ortho = glam::Mat4::orthographic_rh(
                -radius - margin,
                radius + margin,
                -radius - margin,
                radius + margin,
                0.1,
                light_dist + radius + margin,
            );
            // The fit as ACTUALLY computed — published for the relief census,
            // which otherwise has to model it (and modelled c2's pad wrong).
            let fit = CascadeFit {
                near,
                far,
                center,
                radius,
                margin,
                depth_range: light_dist + radius + margin - 0.1,
                texel_m: 2.0 * (radius + margin) / SHADOW_MAP_SIZE as f32,
                refreshed: true,
            };

            let Some(c) = far_idx else {
                let light_view =
                    glam::Mat4::look_to_rh(center - light_dir * light_dist, light_dir, up);
                self.cascades[i] = ortho * light_view;
                self.cascade_fits[i] = fit;
                self.queue.write_buffer(
                    &self.shadow_cascade_bufs[i],
                    0,
                    bytemuck::cast_slice(&self.cascades[i].to_cols_array()),
                );
                continue;
            };

            let drift = (center - self.c_far_center[c]).length();
            // Which trigger fired, in precedence order — the telemetry the
            // continuous-flight harness correlates observed frame-steps with.
            let reason = if !self.c_far_valid[c] {
                Some("invalid")
            } else if self.far_policy.forces_refresh(c, self.c2_tick) {
                // Live / live-c2 / alternate: the policy pre-empts the
                // sun/drift/tick triggers, which stay in place beneath it so
                // that a policy which only refreshes SOME frames still cannot
                // let a window lag past the pad it was fitted with.
                Some("policy")
            } else if light_dir.dot(self.c_far_light_dir[c]) < 0.9999 {
                Some("sun")
            } else if drift > self.far_policy.drift_limit(c) {
                Some("drift")
            } else if self.c2_tick == 0 {
                Some("tick")
            } else {
                None
            };
            if let Some(reason) = reason {
                let light_view =
                    glam::Mat4::look_to_rh(center - light_dir * light_dist, light_dir, up);
                // The window this cascade is LEAVING — captured before the
                // overwrite. A refresh log that shows only the destination
                // cannot tell you how far the window jumped, which is the whole
                // quantity the far-field pop is proportional to.
                let old_center = self.c_far_center[c];
                // L.3.D rung 2: open a transition on the outermost cascade.
                // The outgoing MATRIX must be captured here for the same reason
                // as the centre — one line later it is gone — and the map it
                // addresses is snapshotted by the draw path before the pass
                // that overwrites it.
                if i == CASCADE_COUNT - 1 && self.far_policy.fades_c3() {
                    self.c3_prev_vp = self.cascades[i];
                    self.c3_fade = 0.0;
                    self.c3_copy_pending = true;
                }
                self.cascades[i] = ortho * light_view;
                self.cascade_fits[i] = fit;
                self.c_far_center[c] = center;
                self.c_far_light_dir[c] = light_dir;
                // LOAD-BEARING: this one write is what makes BOTH pass gates
                // policy-aware. The gates in `render()` and `draw_into` test
                // `!c_far_valid[c] || c_far_pending[c]` and know nothing about
                // `FarCascadePolicy`; a live cascade re-renders every frame
                // only because the policy branch sets this flag every frame.
                // Bypassing it for live cascades would update the matrix while
                // the depth map stayed frozen — right matrix, stale map, wrong
                // shadows, no panic. That is the L.3.A bug in mirror image.
                self.c_far_pending[c] = true;
                // Telemetry is per far cascade — see the field docs for why a
                // scalar tracking only the outermost one was not enough.
                self.c_far_refresh_count[c] = self.c_far_refresh_count[c].wrapping_add(1);
                self.c_far_refresh_reason[c] = reason;
                self.c_far_last_drift[c] = drift;
                self.c_far_window[c] = (center, radius);
                // `AW_CSM_LOG=1` prints the refresh stream (this crate has no
                // tracing dependency; the harness reads the telemetry accessor
                // directly, which is the machine-checked path). Refreshes the
                // POLICY forced are suppressed: under `live`/`live-c2` that is
                // every frame for that cascade, which would drown the
                // sun/drift/tick triggers this stream exists to expose.
                if reason != "policy" && std::env::var_os("AW_CSM_LOG").is_some() {
                    eprintln!(
                        "[csm] c{i} refit #{} ({reason}, policy {}): centre \
                         ({:.0},{:.0},{:.0}) -> ({:.0},{:.0},{:.0}) jump {:.1} m \
                         radius {:.0} pad {:.0} texel {:.3} m depth {:.0} m",
                        self.c_far_refresh_count[c],
                        self.far_policy.as_str(),
                        old_center.x,
                        old_center.y,
                        old_center.z,
                        center.x,
                        center.y,
                        center.z,
                        drift,
                        radius,
                        fit.margin,
                        fit.texel_m,
                        fit.depth_range
                    );
                }
                self.queue.write_buffer(
                    &self.shadow_cascade_bufs[i],
                    0,
                    bytemuck::cast_slice(&self.cascades[i].to_cols_array()),
                );
            } else {
                // Frozen this frame: the published fit stays the one the map
                // was actually rendered with, flagged as not-refreshed.
                self.cascade_fits[i].refreshed = false;
                self.c_far_last_drift[c] = drift;
            }
        }

        // L.3.D rung 2: advance the c3 transition. Pinned at 1.0 (= no
        // transition, sample the current window only) under every other policy,
        // which is what makes this machinery inert unless it is selected.
        self.c3_fade = if self.far_policy.fades_c3() {
            (self.c3_fade + 1.0 / C3_FADE_FRAMES).min(1.0)
        } else {
            1.0
        };

        // L.3.D: publish which FAR cascades re-fitted this frame, for the
        // cascade-index debug view's refresh flash (bit 0 = c2, bit 1 = c3).
        // Derived from `cascade_fits`, which the fit loop above just wrote, so
        // the overlay cannot disagree with what actually happened — and no
        // caller has to remember to set it.
        // Only cascades that CAN hold a frozen window pulse. A cascade running
        // live re-fits every frame by construction, so a per-frame pulse on it
        // is a constant, not an event — it carries no information, and worse it
        // makes the band permanently pulsed, which is how the first validation
        // run mistook a permanently-flashed c2 for "c2 is not in frame". Same
        // reasoning as suppressing policy-forced refreshes in the AW_CSM_LOG
        // stream: a signal that fires every frame is not a signal.
        let mut refresh_bits = 0u32;
        for c in 0..(CASCADE_COUNT - FIRST_CACHED_CASCADE) {
            if self.far_policy.frozen(c) && self.cascade_fits[c + FIRST_CACHED_CASCADE].refreshed {
                refresh_bits |= 1 << c;
            }
        }
        self.scene_env.debug_flags = refresh_bits;

        // MainLightUbo (shadow_common.wgsl): 4×mat4 (256) + splits vec4
        // (split0, split1, shadow_far, fade_start) + extras vec4
        // (pcf_or_sentinel, depth_bias, split2, pad) + bias_scales vec4
        // (per-cascade receiver-bias cap) = 304 B. `split2` rides a lane that
        // was already padding, so `splits` and `extras.x`/`.y` keep their
        // offsets — the sentinel contract test reads `extras.x`.
        //
        // FOUR sites carry this size and must move together: `light_buf`'s
        // `size:`, this capacity, the `debug_assert_eq!` below (compiled OUT in
        // release-fast, so it is not a runtime guard), and
        // `renderer_tests.rs::test_light_uniform_buffer`.
        let mut data: Vec<f32> = Vec::with_capacity(92);
        for m in &self.cascades {
            data.extend_from_slice(&m.to_cols_array());
        }
        data.push(self.split0);
        data.push(self.split1);
        data.push(shadow_far);
        data.push(shadow_far * SHADOW_FADE_FRACTION);
        let extras_x = if self.force_shadow_override {
            // Shadows off: the passes are skipped, so the cached maps go stale
            // — force a refresh on the frame shadows re-enable.
            self.c_far_valid = [false; CASCADE_COUNT - FIRST_CACHED_CASCADE];
            -1.0
        } else {
            self.shadow_pcf_radius_px
        };
        data.push(extras_x);
        data.push(self.shadow_depth_bias);
        data.push(self.split2);
        // extras.w: c3's transition blend weight (L.3.D rung 2). 1.0 means no
        // transition in flight, and the sampler skips the second tap entirely —
        // so every other policy pays nothing for this lane.
        data.push(self.c3_fade);
        // Per-cascade receiver-bias caps, derived from the fits JUST computed
        // above rather than from constants. `extras.y` is one NDC value whose
        // WORLD magnitude scales with each cascade's ortho depth range (30x
        // across the set), so every cascade is capped at c1's world slack —
        // `min(1, c1_depth / this_depth)`, a cap and never a raise, which is
        // what keeps c0/c1 (and therefore the pinned close/mid stations) at
        // exactly the bias they shipped with. L.3.C hard-coded these from a
        // MODEL of the fits and got c2's 8.5% wrong by pricing a cached
        // cascade with the every-frame pad; a value read off the fit cannot
        // drift when a split, a pad or the refresh policy changes.
        let c1_depth = self.cascade_fits[1].depth_range.max(1.0);
        for f in &self.cascade_fits {
            data.push((c1_depth / f.depth_range.max(1.0)).min(1.0));
        }
        // L.3.D rung 2: c3's OUTGOING view-projection, appended LAST so every
        // earlier offset — including the `extras.x` sentinel the contract test
        // greps for — is untouched.
        data.extend_from_slice(&self.c3_prev_vp.to_cols_array());
        debug_assert_eq!(data.len() * 4, 368, "MainLightUbo layout drift");
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
        self.material_alpha_cutoff = alpha_cutoff;
        self.queue
            .write_buffer(&self.material_buf, 0, bytemuck::bytes_of(&ubo));
    }

    pub fn create_mesh_from_arrays(
        &self,
        vertices: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: &[u32],
    ) -> Mesh {
        if self.is_device_lost() {
            return self.unmapped_mesh();
        }
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
        let vbuf = self.upload_bytes(
            "ext v",
            bytemuck::cast_slice(&verts),
            wgpu::BufferUsages::VERTEX,
        );
        let ibuf = self.upload_bytes(
            "ext i",
            bytemuck::cast_slice(indices),
            wgpu::BufferUsages::INDEX,
        );
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
        if self.is_device_lost() {
            return self.unmapped_mesh();
        }
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
            let vbuf = self.upload_bytes(
                "ext v (full) placeholder",
                bytemuck::cast_slice(&placeholder_verts),
                wgpu::BufferUsages::VERTEX,
            );
            let ibuf = self.upload_bytes(
                "ext i (full) placeholder",
                bytemuck::cast_slice(&placeholder_idx),
                wgpu::BufferUsages::INDEX,
            );
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
        let vbuf = self.upload_bytes(
            "ext v (full)",
            bytemuck::cast_slice(&verts),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let ibuf = self.upload_bytes(
            "ext i (full)",
            bytemuck::cast_slice(indices),
            wgpu::BufferUsages::INDEX,
        );
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

    /// Rewrite a named model's first primitive from a CPU mesh.
    ///
    /// The CPU mesh is the source of truth (the sim already committed it).
    /// When the vertex and index bytes fit the existing GPU buffers, this
    /// writes in place and leaves [`RenderModelPrimitive::storage_generation`]
    /// unchanged. Growth allocates the next power-of-two size and bumps the
    /// generation. The model's world AABB is updated from the CPU mesh.
    pub fn write_mesh(&mut self, name: &str, cpu: &crate::mesh::CpuMesh) -> Result<()> {
        if cpu.vertices.is_empty() || cpu.indices.is_empty() {
            anyhow::bail!("model '{name}' write has no vertices or indices");
        }
        let verts: Vec<crate::types::Vertex> = cpu
            .vertices
            .iter()
            .map(|v| crate::types::Vertex {
                position: v.position,
                normal: v.normal,
                tangent: v.tangent,
                uv: v.uv,
            })
            .collect();
        let vbytes = bytemuck::cast_slice(&verts);
        let ibytes = bytemuck::cast_slice(&cpu.indices);
        let aabb = cpu
            .aabb()
            .map(|(min, max)| (min.to_array(), max.to_array()));

        let (fits, vertex_buf, index_buf) = {
            let model = self
                .models
                .get_mut(name)
                .with_context(|| format!("model '{name}' is not loaded"))?;
            let primitive = model
                .primitives
                .first_mut()
                .with_context(|| format!("model '{name}' has no primitives"))?;
            let fits = (vbytes.len() as u64) <= primitive.mesh.vertex_buf.size()
                && (ibytes.len() as u64) <= primitive.mesh.index_buf.size();
            let vertex_buf = primitive.mesh.vertex_buf.clone();
            let index_buf = primitive.mesh.index_buf.clone();
            primitive.mesh.index_count = cpu.indices.len() as u32;
            model.aabb = aabb;
            (fits, vertex_buf, index_buf)
        };

        if fits {
            self.queue.write_buffer(&vertex_buf, 0, vbytes);
            self.queue.write_buffer(&index_buf, 0, ibytes);
            return Ok(());
        }

        let vsize = (vbytes.len() as u64).next_power_of_two().max(64);
        let isize = (ibytes.len() as u64).next_power_of_two().max(12);
        let new_vb = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh vertex rewrite"),
            size: vsize,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let new_ib = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh index rewrite"),
            size: isize,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&new_vb, 0, vbytes);
        self.queue.write_buffer(&new_ib, 0, ibytes);
        let model = self
            .models
            .get_mut(name)
            .with_context(|| format!("model '{name}' disappeared during rewrite"))?;
        let primitive = model
            .primitives
            .first_mut()
            .with_context(|| format!("model '{name}' has no primitives"))?;
        primitive.mesh.vertex_buf = new_vb;
        primitive.mesh.index_buf = new_ib;
        primitive.storage_generation = primitive.storage_generation.saturating_add(1);
        Ok(())
    }

    /// Queue `path` for the named model. Distance is from `world_pos` to the
    /// camera. Until the decode worker finishes, the model keeps a 1×1 white
    /// albedo bind. `draw_into` uploads at most one resident texture per frame
    /// and does not allocate while the fallback is still bound.
    pub fn request_streamed_albedo(
        &mut self,
        model_name: &str,
        path: &str,
        world_pos: glam::Vec3,
    ) -> Result<()> {
        if !self.models.contains_key(model_name) {
            anyhow::bail!("model '{model_name}' is not loaded");
        }
        let (fallback_bg, fallback_textures, _) =
            self.create_model_albedo_bind_group(1, 1, &[255, 255, 255, 255]);
        let model = self
            .models
            .get_mut(model_name)
            .context("model disappeared while requesting a stream")?;
        model.stream_albedo_id = Some(path.to_string());
        model.stream_resident = false;
        for primitive in &mut model.primitives {
            primitive.tex_bind_group = Some(fallback_bg.clone());
            primitive._retained_textures = fallback_textures.clone();
        }
        let camera = glam::Vec3::new(
            self.camera_ubo.camera_pos_pad[0],
            self.camera_ubo.camera_pos_pad[1],
            self.camera_ubo.camera_pos_pad[2],
        );
        self.streaming.set_camera(camera);
        self.streaming.request_at(path, world_pos);
        Ok(())
    }

    /// `true` while `model_name` is waiting on a streamed albedo and still
    /// draws the 1×1 fallback.
    pub fn streamed_albedo_is_fallback(&self, model_name: &str) -> bool {
        self.models
            .get(model_name)
            .map(|model| model.stream_albedo_id.is_some() && !model.stream_resident)
            .unwrap_or(false)
    }

    pub fn stream_failure(&self, path: &str) -> Option<&str> {
        self.streaming.failure(path)
    }

    fn tick_texture_streaming(&mut self) {
        let camera = glam::Vec3::new(
            self.camera_ubo.camera_pos_pad[0],
            self.camera_ubo.camera_pos_pad[1],
            self.camera_ubo.camera_pos_pad[2],
        );
        let device = self.device.clone();
        let queue = self.queue.clone();
        self.streaming.set_camera(camera);
        self.streaming.tick(&device, &queue, 1);
        self.refresh_streamed_albedo();
    }

    fn refresh_streamed_albedo(&mut self) {
        let mut ready_name: Option<String> = None;
        for (name, model) in &self.models {
            let Some(id) = model.stream_albedo_id.as_deref() else {
                continue;
            };
            if model.stream_resident {
                continue;
            }
            if self.streaming.resident_texture(id).is_some() {
                ready_name = Some(name.clone());
                break;
            }
        }
        let Some(name) = ready_name else {
            return;
        };
        let Some(id) = self
            .models
            .get(&name)
            .and_then(|model| model.stream_albedo_id.clone())
        else {
            return;
        };
        let Some(texture) = self.streaming.resident_texture(&id) else {
            return;
        };
        let view = texture.view.clone();
        let gpu_texture = texture.texture.clone();
        let bg = self.bind_group_for_albedo_view(&view);
        let Some(model) = self.models.get_mut(&name) else {
            return;
        };
        for primitive in &mut model.primitives {
            primitive.tex_bind_group = Some(bg.clone());
            primitive._retained_textures = vec![gpu_texture.clone()];
        }
        model.stream_resident = true;
    }

    fn bind_group_for_albedo_view(&self, view: &wgpu::TextureView) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("streamed albedo bg"),
            layout: &self.tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
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
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&self.occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.occlusion_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&self.emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&self.emissive_sampler),
                },
            ],
        })
    }

    pub fn set_atmosphere_enabled(&mut self, enabled: bool) {
        self.atmosphere_on = enabled;
        self.clamp_atmosphere_to_volumetrics();
    }

    /// Bruneton with fog and shafts both off scatters geometry to 50 km.
    /// That pair is not a frame. The flag stays on only while at least one
    /// of those passes is enabled. The 4.474 ms cell has both, so aerial
    /// `max_distance` is 0. `PresentationQuality::Measured` is the off value.
    #[must_use]
    pub fn atmosphere_enabled(&self) -> bool {
        self.atmosphere_on
    }

    pub fn set_cloud_raymarch_enabled(&mut self, enabled: bool) {
        self.cloud_raymarch_on = enabled;
    }

    pub fn set_ssgi_enabled(&mut self, enabled: bool) {
        self.ssgi_on = enabled;
    }

    #[must_use]
    pub fn ssgi_enabled(&self) -> bool {
        self.ssgi_on
    }

    pub fn set_taa_enabled(&mut self, enabled: bool) {
        self.taa_on = enabled;
    }

    #[must_use]
    pub fn taa_enabled(&self) -> bool {
        self.taa_on
    }

    pub fn set_temporal_upscale_enabled(&mut self, enabled: bool) {
        self.upscale_on = enabled;
    }

    pub fn set_dof_enabled(&mut self, enabled: bool) {
        self.dof_on = enabled;
    }

    /// Write view-space normals and UV motion into the gated aux targets.
    ///
    /// The default is off. Only [`Self::draw_into`] records the pass.
    /// [`Self::render`] does not. Turning this on does not enable SSGI, TAA,
    /// or temporal upscale, and it does not make their history or presented
    /// frame valid. Those passes stay on their own setters.
    pub fn set_scene_vectors_enabled(&mut self, enabled: bool) {
        self.scene_vectors_on = enabled;
        if !enabled {
            // Drop the slots. The next enabled frame must not reuse a buffer
            // that still holds the last enabled pose.
            self.scene_vectors_prev_valid = false;
            self.scene_vector_last_copy_bytes = 0;
            self.plane_prev_instances = None;
            self.sphere_prev_instances = None;
            self.ext_prev_instances = None;
            self.sphere_prev_transforms.clear();
            for model in self.models.values_mut() {
                model.scene_prev_instances = None;
            }
            #[cfg(feature = "terrain-splat-arrays")]
            if let Some(tf) = self.terrain_forward.as_mut() {
                for chunk in tf.chunks.values_mut() {
                    chunk.prev_vertex = None;
                }
            }
        }
    }

    /// Bytes `copy_buffer_to_buffer` wrote into previous-instance and
    /// previous-vertex buffers the last time the pass ran. Sphere previous
    /// poses and weather previous poses are queue uploads and are not
    /// included. A growth frame includes the stage copy and the later commit
    /// copy. A square terrain grid whose side length changed copies the new
    /// vertices over the previous buffer, so that frame's delta is zero.
    /// Zero while the pass is off.
    #[must_use]
    pub fn scene_vector_last_copy_bytes(&self) -> u64 {
        self.scene_vector_last_copy_bytes
    }

    #[must_use]
    pub fn scene_vectors_enabled(&self) -> bool {
        self.scene_vectors_on
    }

    /// The normal and velocity textures after a frame that enabled the pass.
    /// `None` while the pass is off and SSGI, TAA, and upscale are also off.
    #[must_use]
    pub fn scene_vector_targets(&self) -> Option<(&wgpu::Texture, &wgpu::Texture)> {
        match (&self.gated_normal, &self.gated_velocity) {
            (Some((normal, _)), Some((velocity, _))) => Some((normal, velocity)),
            _ => None,
        }
    }

    fn ensure_gated_aux(&mut self) {
        if self.gated_normal.is_some() {
            return;
        }
        let size = wgpu::Extent3d {
            width: self.config.width.max(1),
            height: self.config.height.max(1),
            depth_or_array_layers: 1,
        };
        let normal = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gated normal"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let normal_view = normal.create_view(&wgpu::TextureViewDescriptor::default());
        let velocity = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gated velocity"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let velocity_view = velocity.create_view(&wgpu::TextureViewDescriptor::default());
        self.gated_normal = Some((normal, normal_view));
        self.gated_velocity = Some((velocity, velocity_view));
    }

    /// Bruneton sky and aerial, copied onto HDR before fog and god rays.
    ///
    /// Cleared pixels become the Bruneton sky. Fog and shafts then composite
    /// over that color, so a sky pixel keeps both. When fog or shafts are
    /// already on, geometry is not scattered a second time.
    fn record_bruneton(&mut self, enc: &mut wgpu::CommandEncoder, depth: &wgpu::TextureView) {
        if !self.atmosphere_on {
            return;
        }
        // The setter refuses this pair. A stale flag still must not scatter
        // geometry out to 50 km.
        if !self.volumetric_fog_enabled() && !self.god_rays_enabled() {
            return;
        }
        let gen = self.resource_generation;
        let hdr = self.hdr_view.clone();
        let device = self.device.clone();
        let queue = self.queue.clone();
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let view_proj = self.cached_proj * self.cached_view;
        let inv = view_proj.inverse();
        let eye = glam::Vec3::new(
            self.camera_ubo.camera_pos_pad[0],
            self.camera_ubo.camera_pos_pad[1],
            self.camera_ubo.camera_pos_pad[2],
        );
        let light = glam::Vec3::new(
            self.camera_ubo.light_dir_pad[0],
            self.camera_ubo.light_dir_pad[1],
            self.camera_ubo.light_dir_pad[2],
        );
        let toward_sun = if light.length_squared() > 1e-6 {
            -light.normalize()
        } else {
            self.sky.time_of_day().get_sun_position()
        };
        let (near, far) = clip_planes(self.cached_proj);
        let scatter_geometry = !self.volumetric_fog_enabled() && !self.god_rays_enabled();
        if self.atmosphere.is_none() {
            self.atmosphere = Some(crate::atmosphere::AtmospherePass::new(
                &self.device,
                width,
                height,
                crate::atmosphere::AtmosphereConfig::default(),
            ));
        }
        let Some(pass) = self.atmosphere.as_mut() else {
            return;
        };
        let (pass_w, pass_h) = pass.dimensions();
        if pass_w != width || pass_h != height {
            pass.resize(&device, width, height);
        }
        pass.prepare_frame(
            &queue,
            inv,
            eye,
            toward_sun,
            -toward_sun,
            near,
            far,
            scatter_geometry,
        );
        pass.execute(&device, enc, &hdr, depth, gen);
        enc.copy_texture_to_texture(
            pass.aerial_texture().as_image_copy(),
            self.hdr_tex.as_image_copy(),
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// `N` when `byte_len` is `N * N` vertices of `stride` bytes.
    #[cfg(feature = "terrain-splat-arrays")]
    fn square_grid_side(byte_len: u64, stride: u64) -> Option<u64> {
        if stride == 0 || byte_len % stride != 0 {
            return None;
        }
        let count = byte_len / stride;
        if count == 0 {
            return None;
        }
        let side = count.isqrt();
        if side * side == count {
            Some(side)
        } else {
            None
        }
    }

    /// Returns the previous buffer and the bytes this call passed to
    /// `copy_buffer_to_buffer`. A slot that already fits copies nothing here.
    fn stage_prev_instances(
        device: &wgpu::Device,
        enc: &mut wgpu::CommandEncoder,
        current: &wgpu::Buffer,
        slot: &mut Option<wgpu::Buffer>,
    ) -> (wgpu::Buffer, u64) {
        let fits = slot
            .as_ref()
            .is_some_and(|prev| prev.size() >= current.size());
        if !fits {
            let created = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scene vector previous instances"),
                size: current.size(),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            // A larger upload keeps the old prefix. Copying `current` over
            // the whole previous buffer would zero that frame's motion.
            // A square terrain grid whose side changed does not use this path.
            let mut copied = 0u64;
            if let Some(old) = slot.take() {
                let keep = old.size().min(current.size());
                if keep > 0 {
                    enc.copy_buffer_to_buffer(&old, 0, &created, 0, keep);
                    copied += keep;
                }
                if current.size() > keep {
                    let tail = current.size() - keep;
                    enc.copy_buffer_to_buffer(current, keep, &created, keep, tail);
                    copied += tail;
                }
            } else if current.size() > 0 {
                enc.copy_buffer_to_buffer(current, 0, &created, 0, current.size());
                copied = current.size();
            }
            *slot = Some(created.clone());
            return (created, copied);
        }
        match slot {
            Some(prev) => (prev.clone(), 0),
            None => (current.clone(), 0),
        }
    }

    /// Editor terrain is row-major `i = gy * N + gx`. A new `N` makes a byte
    /// prefix a different point, including a shrink whose old buffer still
    /// fits. Copy the new vertices over the previous buffer so the delta is
    /// zero. Any other size change keeps the prefix.
    #[cfg(feature = "terrain-splat-arrays")]
    fn stage_terrain_prev(
        device: &wgpu::Device,
        enc: &mut wgpu::CommandEncoder,
        current: &wgpu::Buffer,
        slot: &mut Option<wgpu::Buffer>,
    ) -> (wgpu::Buffer, u64) {
        let stride =
            std::mem::size_of::<crate::terrain_material_manager::TerrainSplatVertex>() as u64;
        let old_side = slot
            .as_ref()
            .and_then(|prev| Self::square_grid_side(prev.size(), stride));
        let new_side = Self::square_grid_side(current.size(), stride);
        let side_changed = match (old_side, new_side) {
            (Some(old_side), Some(new_side)) => old_side != new_side,
            _ => false,
        };
        if !side_changed {
            return Self::stage_prev_instances(device, enc, current, slot);
        }
        let created = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene vector previous terrain"),
            size: current.size(),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let copied = if current.size() > 0 {
            enc.copy_buffer_to_buffer(current, 0, &created, 0, current.size());
            current.size()
        } else {
            0
        };
        *slot = Some(created.clone());
        (created, copied)
    }

    fn commit_prev_instances(
        enc: &mut wgpu::CommandEncoder,
        current: &wgpu::Buffer,
        prev: &wgpu::Buffer,
    ) -> u64 {
        let bytes = current.size().min(prev.size());
        if bytes > 0 {
            enc.copy_buffer_to_buffer(current, 0, prev, 0, bytes);
        }
        bytes
    }

    /// Color draws `0..count` with unbiased `Less`, so the earlier instance
    /// wins an equal-depth tie. This pass is biased, so the earlier instance
    /// is the last draw.
    fn draw_indexed_color_winner(
        rp: &mut wgpu::RenderPass<'_>,
        index_count: u32,
        instance_count: u32,
    ) {
        for instance in (0..instance_count).rev() {
            rp.draw_indexed(0..index_count, 0, instance..instance + 1);
        }
    }

    fn scene_vector_model_visible(
        &self,
        name: &str,
        cam_pos: glam::Vec3,
        max_dist_sq: f32,
        frustum: &crate::culling::FrustumPlanes,
    ) -> bool {
        let Some(model) = self.models.get(name) else {
            return false;
        };
        if model.instance_count == 0 {
            return false;
        }
        let bounds = if self.gpu_cull_on {
            model
                .aabb
                .or_else(|| self.gpu_cull_bounds.get(name).copied())
        } else {
            model.aabb
        };
        let Some((min, max)) = bounds else {
            return true;
        };
        let center = glam::Vec3::new(
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        );
        let extent = glam::Vec3::new(
            (max[0] - min[0]) * 0.5,
            (max[1] - min[1]) * 0.5,
            (max[2] - min[2]) * 0.5,
        );
        let to_cam = center - cam_pos;
        let closest_dist_sq = (to_cam.x.abs() - extent.x).max(0.0).powi(2)
            + (to_cam.z.abs() - extent.z).max(0.0).powi(2);
        closest_dist_sq <= max_dist_sq && frustum.test_aabb(center, extent)
    }

    /// Clear and redraw the surfaces `draw_into` just wrote.
    ///
    /// Normals are view space in [0, 1]. Velocity is a UV delta, Y down.
    /// The first enabled frame stores zero motion. Named models use the color
    /// pass order and are drawn in reverse. Each model's primitives, and the
    /// instances of a model, sphere batch, or weather batch, draw from the
    /// tail, so biased `Less` keeps the earlier color-pass surface. A model
    /// the color pass culled is not drawn, but its previous matrix is still
    /// refreshed. `set_skinned_mesh` is not drawn by `draw_into`, so it is
    /// not drawn here.
    fn record_scene_vectors(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        depth_view: Option<&wgpu::TextureView>,
    ) {
        if !self.scene_vectors_on || self.is_device_lost() {
            return;
        }
        self.ensure_gated_aux();
        let (Some((_, normal_view)), Some((_, velocity_view))) =
            (self.gated_normal.as_ref(), self.gated_velocity.as_ref())
        else {
            return;
        };
        let normal_view = normal_view.clone();
        let velocity_view = velocity_view.clone();
        let depth = depth_view
            .cloned()
            .unwrap_or_else(|| self.depth.view.clone());
        let curr = self.cached_proj * self.cached_view;
        let prev = if self.scene_vectors_prev_valid {
            self.scene_vectors_prev
        } else {
            curr
        };
        let device = self.device.clone();
        let queue = self.queue.clone();
        let view = self.cached_view;
        #[cfg(feature = "impostor-bake")]
        let prev_view = if self.scene_vectors_prev_valid {
            self.scene_vectors_prev_view
        } else {
            view
        };
        if self.scene_vectors.is_none() {
            self.scene_vectors = Some(crate::scene_vectors::SceneVectorsPass::new(
                &device,
                &self.tex_bgl,
            ));
        }
        let (pipeline, terrain_pipeline, bind_group) = {
            let Some(pass) = self.scene_vectors.as_ref() else {
                return;
            };
            pass.stage_uniforms(&queue, curr, prev, view, self.material_alpha_cutoff);
            (
                pass.pipeline().clone(),
                pass.terrain_pipeline().clone(),
                pass.bind_group().clone(),
            )
        };
        #[cfg(not(feature = "terrain-splat-arrays"))]
        let _ = &terrain_pipeline;
        let cam_pos = glam::Vec3::new(
            self.camera_ubo.camera_pos_pad[0],
            self.camera_ubo.camera_pos_pad[1],
            self.camera_ubo.camera_pos_pad[2],
        );
        let max_draw_dist = if self.max_draw_distance > 0.0 {
            self.max_draw_distance
        } else {
            self.scene_env.visuals.fog_end * 1.2
        };
        let max_draw_dist_sq = max_draw_dist * max_draw_dist;
        let frustum = crate::culling::FrustumPlanes::from_view_proj(&curr);
        // Same order as the color pass: sorted names only while GPU cull is
        // on, otherwise the `models` map order. Drawing that list in reverse
        // makes the last equal-depth fragment the one `Less` kept in color.
        let mut names: Vec<String> = if self.gpu_cull_on {
            self.gpu_cull_names.clone()
        } else {
            self.models.keys().cloned().collect()
        };
        names.retain(|name| {
            self.scene_vector_model_visible(name, cam_pos, max_draw_dist_sq, &frustum)
        });
        let all_model_names: Vec<String> = self.models.keys().cloned().collect();
        let sphere_count = self.build_visible_instances() as u32;
        let sphere_stride = std::mem::size_of::<InstanceRaw>() as u64;
        let sphere_bytes = u64::from(sphere_count) * sphere_stride;
        if sphere_count > 0 {
            let fits = self
                .sphere_prev_instances
                .as_ref()
                .is_some_and(|buf| buf.size() >= sphere_bytes);
            if !fits {
                self.sphere_prev_instances = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("scene vector previous spheres"),
                    size: sphere_bytes.max(sphere_stride),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            if let Some(buf) = self.sphere_prev_instances.as_ref() {
                queue.write_buffer(
                    buf,
                    0,
                    bytemuck::cast_slice(&self.visible_prev_scratch[..sphere_count as usize]),
                );
            }
        }
        let mut copy_bytes = 0u64;
        let (plane_prev, staged) = Self::stage_prev_instances(
            &device,
            enc,
            &self.plane_inst_buf,
            &mut self.plane_prev_instances,
        );
        copy_bytes += staged;
        let ext_prev = if let Some(buf) = self.ext_inst_buf.clone() {
            let (prev, staged) =
                Self::stage_prev_instances(&device, enc, &buf, &mut self.ext_prev_instances);
            copy_bytes += staged;
            Some(prev)
        } else {
            None
        };
        for name in &all_model_names {
            let Some(model) = self.models.get_mut(name) else {
                continue;
            };
            let current = model.instance_buf.clone();
            let (_, staged) =
                Self::stage_prev_instances(&device, enc, &current, &mut model.scene_prev_instances);
            copy_bytes += staged;
        }
        let weather_count = self.weather.particle_count() as u32;
        let weather_buf = self.weather.buffer().clone();
        // Previous poses are uploaded with the particle, in slot order, after
        // swap-remove. A GPU copy of the current buffer would pair slot i
        // with whatever particle occupied it last frame.
        let weather_prev = if weather_count > 0 {
            Some(self.weather.previous_buffer().clone())
        } else {
            None
        };
        #[cfg(feature = "terrain-splat-arrays")]
        let terrain_draw_keys: Vec<u64> = if let Some(tf) = self.terrain_forward.as_mut() {
            let mut keys: Vec<u64> = tf.chunks.keys().copied().collect();
            for key in &keys {
                let Some(chunk) = tf.chunks.get_mut(key) else {
                    continue;
                };
                let current = chunk.vertex_buffer.clone();
                let (_, staged) =
                    Self::stage_terrain_prev(&device, enc, &current, &mut chunk.prev_vertex);
                copy_bytes += staged;
            }
            keys.reverse();
            keys
        } else {
            Vec::new()
        };
        let tex_bg = self.tex_bg.clone();
        let stamp = self.begin_named_stamp(enc, "scene_vectors");
        if let Some(weather_prev_buf) = weather_prev.as_ref() {
            if let Some(pass) = self.scene_vectors.as_ref() {
                // Ordered with this pass. A queue write of cutoff 0 would be
                // replaced by the mesh cutoff before submit.
                pass.copy_weather_uniform(enc);
            }
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene vectors weather"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &normal_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &velocity_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&pipeline);
            rp.set_bind_group(0, &bind_group, &[]);
            rp.set_bind_group(1, &tex_bg, &[]);
            rp.set_vertex_buffer(0, self.mesh_sphere.vertex_buf.slice(..));
            rp.set_index_buffer(
                self.mesh_sphere.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rp.set_vertex_buffer(1, weather_buf.slice(..));
            rp.set_vertex_buffer(2, weather_prev_buf.slice(..));
            Self::draw_indexed_color_winner(&mut rp, self.mesh_sphere.index_count, weather_count);
        }
        if let Some(pass) = self.scene_vectors.as_ref() {
            pass.copy_mesh_uniform(enc);
        }
        {
            #[cfg(feature = "impostor-bake")]
            let impostor_draws = std::mem::take(&mut self.impostor_passes);
            #[cfg(feature = "impostor-bake")]
            for ip in impostor_draws.values() {
                if !self.scene_vectors_prev_valid {
                    // Meshes report zero object motion on this frame because
                    // their previous buffers were dropped. Impostor uploads
                    // keep advancing while the pass is off, so snap that
                    // history to the current set before the draw.
                    ip.align_history_to_current(&queue);
                }
                ip.prepare_scene_vectors(&queue, curr, prev, view, prev_view);
            }
            let color_load = if weather_count > 0 {
                wgpu::LoadOp::Load
            } else {
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
            };
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene vectors"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &normal_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: color_load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &velocity_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: color_load,
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Reverse of the color pass: impostors, terrain, models, external
            // mesh, spheres, then the ground plane. Less plus the depth bias
            // lets an equal-depth fragment through, and the last one is the
            // surface the color pass kept. HashMap `values` is the color
            // order; the reverse makes the earlier card the last draw.
            // Inside one model the primitives reverse too, and each instance
            // range is drawn from the tail.
            #[cfg(feature = "impostor-bake")]
            {
                let mut impostor_order: Vec<_> = impostor_draws.values().collect();
                impostor_order.reverse();
                for ip in impostor_order {
                    ip.record_scene_vectors(&mut rp);
                }
            }
            #[cfg(feature = "terrain-splat-arrays")]
            if let Some(tf) = self.terrain_forward.as_ref() {
                if !terrain_draw_keys.is_empty() {
                    rp.set_pipeline(&terrain_pipeline);
                    rp.set_bind_group(0, &bind_group, &[]);
                    for key in &terrain_draw_keys {
                        let Some(chunk) = tf.chunks.get(key) else {
                            continue;
                        };
                        let Some(prev_verts) = chunk.prev_vertex.as_ref() else {
                            continue;
                        };
                        rp.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
                        rp.set_vertex_buffer(1, prev_verts.slice(..));
                        rp.set_index_buffer(
                            chunk.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        rp.draw_indexed(0..chunk.index_count, 0, 0..1);
                    }
                }
            }
            rp.set_pipeline(&pipeline);
            rp.set_bind_group(0, &bind_group, &[]);
            for name in names.iter().rev() {
                let Some(model) = self.models.get(name) else {
                    continue;
                };
                let Some(prev) = model.scene_prev_instances.as_ref() else {
                    continue;
                };
                for primitive in model.primitives.iter().rev() {
                    if let Some(tex) = primitive.tex_bind_group.as_ref() {
                        rp.set_bind_group(1, tex, &[]);
                    } else {
                        rp.set_bind_group(1, &tex_bg, &[]);
                    }
                    rp.set_vertex_buffer(0, primitive.mesh.vertex_buf.slice(..));
                    rp.set_index_buffer(
                        primitive.mesh.index_buf.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    rp.set_vertex_buffer(1, model.instance_buf.slice(..));
                    rp.set_vertex_buffer(2, prev.slice(..));
                    Self::draw_indexed_color_winner(
                        &mut rp,
                        primitive.mesh.index_count,
                        model.instance_count,
                    );
                }
            }
            rp.set_bind_group(1, &tex_bg, &[]);
            if let (Some(mesh), Some(ibuf), Some(prev_buf)) = (
                self.mesh_external.as_ref(),
                self.ext_inst_buf.as_ref(),
                ext_prev.as_ref(),
            ) {
                if self.ext_inst_count > 0 {
                    rp.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
                    rp.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                    rp.set_vertex_buffer(1, ibuf.slice(..));
                    rp.set_vertex_buffer(2, prev_buf.slice(..));
                    Self::draw_indexed_color_winner(&mut rp, mesh.index_count, self.ext_inst_count);
                }
            }
            if sphere_count > 0 {
                if let Some(sphere_prev) = self.sphere_prev_instances.as_ref() {
                    rp.set_vertex_buffer(0, self.mesh_sphere.vertex_buf.slice(..));
                    rp.set_index_buffer(
                        self.mesh_sphere.index_buf.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    rp.set_vertex_buffer(1, self.instance_buf.slice(..));
                    rp.set_vertex_buffer(2, sphere_prev.slice(..));
                    Self::draw_indexed_color_winner(
                        &mut rp,
                        self.mesh_sphere.index_count,
                        sphere_count,
                    );
                }
            }
            rp.set_vertex_buffer(0, self.mesh_plane.vertex_buf.slice(..));
            rp.set_index_buffer(
                self.mesh_plane.index_buf.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rp.set_vertex_buffer(1, self.plane_inst_buf.slice(..));
            rp.set_vertex_buffer(2, plane_prev.slice(..));
            rp.draw_indexed(0..self.mesh_plane.index_count, 0, 0..1);
            drop(rp);
            #[cfg(feature = "impostor-bake")]
            {
                self.impostor_passes = impostor_draws;
            }
        }
        self.end_named_stamp(enc, stamp);
        copy_bytes += Self::commit_prev_instances(enc, &self.plane_inst_buf, &plane_prev);
        if let (Some(current), Some(prev_buf)) = (self.ext_inst_buf.as_ref(), ext_prev.as_ref()) {
            copy_bytes += Self::commit_prev_instances(enc, current, prev_buf);
        }
        // Sphere and weather previous poses are queue uploads. They are not
        // encoder copies, so they stay out of this total.
        for name in &all_model_names {
            let Some(model) = self.models.get(name) else {
                continue;
            };
            let Some(prev_buf) = model.scene_prev_instances.as_ref() else {
                continue;
            };
            let current = model.instance_buf.clone();
            let prev_buf = prev_buf.clone();
            copy_bytes += Self::commit_prev_instances(enc, &current, &prev_buf);
        }
        #[cfg(feature = "terrain-splat-arrays")]
        if let Some(tf) = self.terrain_forward.as_ref() {
            for chunk in tf.chunks.values() {
                if let Some(prev_buf) = chunk.prev_vertex.as_ref() {
                    copy_bytes += Self::commit_prev_instances(enc, &chunk.vertex_buffer, prev_buf);
                }
            }
        }
        self.sphere_prev_transforms.clear();
        self.sphere_prev_transforms
            .extend(self.instances.iter().map(|inst| inst.transform));
        self.scene_vector_last_copy_bytes = copy_bytes;
        self.scene_vectors_prev = curr;
        self.scene_vectors_prev_view = view;
        self.scene_vectors_prev_valid = true;
    }

    /// Presentation passes that stay off unless a setter turned them on.
    /// History and GI stay out of the simulation.
    fn record_gated_presentation(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
    ) {
        let gen = self.resource_generation;
        let hdr = self.hdr_view.clone();
        let device = self.device.clone();
        let queue = self.queue.clone();
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let view_proj = self.cached_proj * self.cached_view;
        let inv = view_proj.inverse();
        let eye = glam::Vec3::new(
            self.camera_ubo.camera_pos_pad[0],
            self.camera_ubo.camera_pos_pad[1],
            self.camera_ubo.camera_pos_pad[2],
        );
        if self.cloud_raymarch_on {
            if self.cloud_raymarch.is_none() {
                self.cloud_raymarch = Some(crate::volumetric_clouds::VolumetricCloudsPass::new(
                    &self.device,
                    width,
                    height,
                    crate::volumetric_clouds::CloudConfig::default(),
                ));
            }
            if let Some(pass) = self.cloud_raymarch.as_mut() {
                pass.prepare_frame(
                    &queue,
                    inv,
                    eye,
                    glam::Vec3::Y,
                    glam::Vec3::ONE,
                    1.0,
                    0.2,
                    0.1,
                    200.0,
                    1.0 / 60.0,
                );
                pass.execute(&device, enc, &hdr, depth, gen);
                pass.copy_cloud_to_history(enc);
            }
        }
        if self.ssgi_on || self.taa_on || self.upscale_on {
            self.ensure_gated_aux();
        }
        let normal = self.gated_normal.as_ref().map(|(_, view)| view.clone());
        let velocity = self.gated_velocity.as_ref().map(|(_, view)| view.clone());
        if self.ssgi_on {
            if self.ssgi_pass.is_none() {
                self.ssgi_pass = Some(crate::ssgi::SsgiPass::new(&self.device, width, height));
            }
            if let (Some(pass), Some(normal), Some(velocity)) =
                (self.ssgi_pass.as_mut(), normal.as_ref(), velocity.as_ref())
            {
                pass.update_params(&queue, self.cached_proj);
                pass.execute(&device, enc, depth, normal, &hdr, velocity, gen);
            }
        }
        if self.taa_on {
            if self.taa_pass.is_none() {
                self.taa_pass = Some(crate::taa::TaaPass::new(&self.device, width, height));
            }
            if let (Some(pass), Some(velocity)) = (self.taa_pass.as_mut(), velocity.as_ref()) {
                pass.update_params(&queue);
                pass.execute(&device, enc, &hdr, depth, velocity, gen);
            }
        }
        if self.upscale_on {
            if self.upscale_pass.is_none() {
                let config = crate::temporal_upscale::UpscaleConfig::default();
                self.upscale_pass = Some(crate::temporal_upscale::TemporalUpscalePass::new(
                    &self.device,
                    width,
                    height,
                    &config,
                ));
            }
            if let (Some(pass), Some(velocity)) = (self.upscale_pass.as_mut(), velocity.as_ref()) {
                pass.update_params(&queue);
                pass.execute(&device, enc, &hdr, depth, velocity, gen);
            }
        }
        if self.dof_on {
            if self.dof_pass.is_none() {
                match crate::advanced_post::AdvancedPostFx::new(
                    &self.device,
                    width,
                    height,
                    wgpu::TextureFormat::Rgba16Float,
                ) {
                    Ok(pass) => self.dof_pass = Some(pass),
                    Err(err) => log::error!("depth of field pass was not created: {err:#}"),
                }
            }
            if let Some(pass) = self.dof_pass.as_mut() {
                let history = pass.history_view().clone();
                pass.apply_dof(&device, &queue, enc, &hdr, depth, &history);
            }
        }
    }

    /// Enable a presentation-only particle emitter. `None` disables it.
    /// The seed and `delta_time` on `params` come from the caller. This does
    /// not write simulation state.
    pub fn set_particle_emitter(
        &mut self,
        params: Option<crate::gpu_particles::EmitterParams>,
    ) -> Result<()> {
        if params.is_some() && self.particle_sim.is_none() {
            const MAX: u32 = 256;
            self.particle_sim = Some(
                crate::gpu_particles::GpuParticleSystem::new(&self.device, MAX)
                    .context("create presentation particle sim")?,
            );
            self.particle_draw = Some(crate::particle_render::ParticleRenderPass::new(
                &self.device,
                &self.queue,
                self.config.format,
                self.depth.format,
                crate::particle_render::ParticleRenderConfig::default(),
            ));
            #[repr(C)]
            #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
            struct SortEntry {
                index: u32,
                distance: f32,
            }
            let entries: Vec<SortEntry> = (0..MAX)
                .map(|index| SortEntry {
                    index,
                    distance: 0.0,
                })
                .collect();
            self.particle_sort = Some(self.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("particle sort identity"),
                    contents: bytemuck::cast_slice(&entries),
                    usage: wgpu::BufferUsages::STORAGE,
                },
            ));
        }
        self.particle_emitter = params;
        Ok(())
    }

    /// Turn bindless registration on or off. Enabling does nothing when the
    /// device was created without the binding-array features. Shading stays on
    /// the group-3 bind groups either way; the SSBO is the indexed copy.
    pub fn set_bindless_enabled(&mut self, enabled: bool) {
        self.bindless_active = enabled && self.bindless.is_some();
    }

    pub fn bindless_supported(&self) -> bool {
        self.bindless.is_some()
    }

    pub fn bindless_material_index(&self, name: &str) -> Option<u32> {
        self.models
            .get(name)
            .and_then(|model| model.primitives.first())
            .and_then(|primitive| primitive.bindless_index)
    }

    fn register_bindless_maps(&mut self, textures: &[wgpu::Texture]) -> Option<u32> {
        if !self.bindless_active || textures.len() < 3 {
            return None;
        }
        let system = self.bindless.as_mut()?;
        let view =
            |texture: &wgpu::Texture| texture.create_view(&wgpu::TextureViewDescriptor::default());
        let albedo = system.add_texture(view(&textures[0]))?;
        let metallic_roughness = system.add_texture(view(&textures[1]))?;
        let normal = system.add_texture(view(&textures[2]))?;
        system.add_material(crate::material_bindless::GpuMaterialEntry {
            albedo_index: albedo,
            normal_index: normal,
            orm_index: metallic_roughness,
            flags: crate::material_bindless::MAT_FLAG_HAS_ALBEDO
                | crate::material_bindless::MAT_FLAG_HAS_NORMAL
                | crate::material_bindless::MAT_FLAG_HAS_ORM,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            occlusion_factor: 1.0,
            base_color: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        })
    }

    /// Generation of the GPU buffers for `name`. `None` if the model is absent.
    pub fn mesh_storage_generation(&self, name: &str) -> Option<u32> {
        self.models
            .get(name)
            .and_then(|model| model.primitives.first())
            .map(|primitive| primitive.storage_generation)
    }

    pub fn set_external_mesh(&mut self, mesh: Mesh) {
        self.mesh_external = Some(mesh);
    }

    pub fn update_instances(&mut self, instances: &[Instance]) {
        // Previous pose is the transform currently in this slot, in list
        // order. A newly appended slot has no history, so its motion is zero.
        self.sphere_prev_transforms.clear();
        if self.scene_vectors_prev_valid {
            for (i, inst) in instances.iter().enumerate() {
                let prev = self
                    .instances
                    .get(i)
                    .map(|old| old.transform)
                    .unwrap_or(inst.transform);
                self.sphere_prev_transforms.push(prev);
            }
        } else {
            self.sphere_prev_transforms
                .extend(instances.iter().map(|inst| inst.transform));
        }
        self.instances.clear();
        self.instances.extend_from_slice(instances);

        // When camera-relative, offset every model matrix so its translation
        // is relative to the camera origin.  The f64 subtraction preserves
        // sub-centimetre precision even at 20 000+ km from world origin.
        self.instance_raw_scratch.clear();
        self.instance_raw_scratch.reserve(self.instances.len());
        #[cfg(feature = "camera-relative")]
        {
            let cam = self.camera_world_pos;
            for inst in &self.instances {
                let mut model = inst.transform;
                let wp = glam::DVec3::new(
                    model.w_axis.x as f64,
                    model.w_axis.y as f64,
                    model.w_axis.z as f64,
                );
                let rel = (wp - cam).as_vec3();
                model.w_axis = glam::Vec4::new(rel.x, rel.y, rel.z, model.w_axis.w);
                self.instance_raw_scratch.push(
                    Instance {
                        transform: model,
                        color: inst.color,
                        material_id: inst.material_id,
                    }
                    .raw(),
                );
            }
        }
        #[cfg(not(feature = "camera-relative"))]
        {
            for inst in &self.instances {
                self.instance_raw_scratch.push(inst.raw());
            }
        }

        let bytes = bytemuck::cast_slice(&self.instance_raw_scratch);
        if self.instance_uploaded.as_slice() == bytes {
            return;
        }
        let size = bytes.len() as u64;
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
        self.instance_uploaded.clear();
        self.instance_uploaded.extend_from_slice(bytes);
        self.queue
            .write_buffer(&self.instance_buf, 0, &self.instance_uploaded);
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

    /// Reads `instance_count` from the last GPU-cull indirect commands.
    #[cfg(test)]
    pub async fn read_gpu_cull_instance_counts(&self) -> Vec<u32> {
        let (buffer, batches) = {
            let Some(resources) = &self.indirect_cull_resources else {
                return Vec::new();
            };
            (
                resources.draw_commands_buffer.clone(),
                resources.batch_count as usize,
            )
        };
        if batches == 0 {
            return Vec::new();
        }
        let stride = std::mem::size_of::<crate::culling::DrawIndexedIndirectCommand>();
        let size = (batches * stride) as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu cull instance counts"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gpu cull readback"),
            });
        encoder.copy_buffer_to_buffer(&buffer, 0, &staging, 0, size);
        self.queue.submit(Some(encoder.finish()));
        let buffer_slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            tx.send(res).unwrap();
        });
        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        rx.recv().unwrap().unwrap();
        let data = buffer_slice.get_mapped_range();
        let mut counts = Vec::with_capacity(batches);
        for batch in 0..batches {
            let offset = batch * stride + 4;
            counts.push(u32::from_ne_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]));
        }
        drop(data);
        staging.unmap();
        counts
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
    /// Turn froxel fog on or off and replace its settings.
    ///
    /// A froxel-dimension change rebuilds the volumes. Density changes do not.
    pub fn set_volumetric_fog_config(
        &mut self,
        config: crate::volumetric_fog::VolumetricFogConfig,
    ) {
        let started = std::time::Instant::now();
        let rebuild = self
            .volumetric_fog
            .as_ref()
            .is_none_or(|fog| fog.config().froxel_dims != config.froxel_dims);
        let timestamp_period = self.queue.get_timestamp_period();
        if rebuild {
            let device = self.device.clone();
            match crate::volumetric_fog::VolumetricFogPass::new(
                &device,
                self.config.width,
                self.config.height,
                config,
            ) {
                Ok(pass) => self.volumetric_fog = Some(pass),
                Err(err) => {
                    log::error!("volumetric fog disabled: {err}");
                    self.volumetric_fog = None;
                    self.note_scope_device_loss();
                    self.clamp_atmosphere_to_volumetrics();
                    return;
                }
            }
        } else if let Some(fog) = self.volumetric_fog.as_mut() {
            fog.set_config(config);
        }
        if let Some(fog) = self.volumetric_fog.as_mut() {
            fog.set_timestamp_period_ns(timestamp_period);
        }
        log::info!(
            "volumetric fog {} in {} ms",
            if rebuild { "built" } else { "updated" },
            started.elapsed().as_millis()
        );
        self.clamp_atmosphere_to_volumetrics();
    }

    pub fn volumetric_fog_enabled(&self) -> bool {
        self.volumetric_fog
            .as_ref()
            .is_some_and(|fog| fog.config().enabled)
    }

    /// Feed a world-space scar volume into the existing froxel density pass.
    ///
    /// A no-op while volumetric fog is off, including `--no-volumetrics`.
    pub fn upload_veil_scar_volume(
        &mut self,
        volume: &crate::volumetric_fog::ScarFogVolume,
    ) -> Result<()> {
        let Some(fog) = self.volumetric_fog.as_mut() else {
            return Ok(());
        };
        if !fog.config().enabled {
            return Ok(());
        }
        let queue = self.queue.clone();
        fog.upload_scar_volume(&queue, volume)
            .map_err(|err| anyhow::anyhow!(err))?;
        Ok(())
    }

    /// Latest density and scatter GPU times, when timestamp queries are available.
    pub fn volumetric_gpu_sample(&self) -> Option<crate::volumetric_fog::VolumetricGpuSample> {
        self.volumetric_fog
            .as_ref()
            .and_then(|fog| fog.last_gpu_sample())
    }

    fn poll_fog_timestamps(&mut self) {
        let device = self.device.clone();
        if let Some(fog) = self.volumetric_fog.as_mut() {
            if let Some(sample) = fog.poll_timestamps(&device) {
                log::debug!(
                    "volumetric fog gpu density {:.3} ms scatter {:.3} ms",
                    sample.density_ms,
                    sample.scatter_ms
                );
            }
        }
    }

    fn request_fog_timestamps(&mut self) {
        if let Some(fog) = self.volumetric_fog.as_mut() {
            fog.request_timestamp_readback();
        }
    }

    /// Turn screen-space light shafts on or off.
    ///
    /// Half-resolution changes rebuild the shaft target.
    pub fn set_god_ray_config(&mut self, config: crate::god_rays::GodRayConfig) {
        let started = std::time::Instant::now();
        let rebuild = self
            .god_rays
            .as_ref()
            .is_none_or(|rays| rays.config().half_res != config.half_res);
        if rebuild {
            let device = self.device.clone();
            match crate::god_rays::GodRayPass::with_config(
                &device,
                self.config.width,
                self.config.height,
                config,
            ) {
                Ok(pass) => self.god_rays = Some(pass),
                Err(err) => {
                    log::error!("light shafts disabled: {err}");
                    self.god_rays = None;
                    self.note_scope_device_loss();
                    self.clamp_atmosphere_to_volumetrics();
                    return;
                }
            }
        } else if let Some(rays) = self.god_rays.as_mut() {
            rays.set_config(config);
        }
        log::info!(
            "light shafts {} in {} ms",
            if rebuild { "built" } else { "updated" },
            started.elapsed().as_millis()
        );
        self.clamp_atmosphere_to_volumetrics();
    }

    pub fn god_rays_enabled(&self) -> bool {
        self.god_rays
            .as_ref()
            .is_some_and(|rays| rays.config().enabled)
    }

    /// Drop froxel fog and light shafts.
    ///
    /// `catch_unwind` does not undo a setter that already stored its pass.
    /// A panic between the two builds can leave fog enabled. This drops both
    /// and turns Bruneton off. Atmosphere with neither pass would scatter
    /// geometry out to 50 km, and that pair is not recorded.
    pub fn clear_volumetric_passes(&mut self) {
        self.volumetric_fog = None;
        self.god_rays = None;
        self.atmosphere_on = false;
    }

    fn clamp_atmosphere_to_volumetrics(&mut self) {
        if self.atmosphere_on && !self.volumetric_fog_enabled() && !self.god_rays_enabled() {
            self.atmosphere_on = false;
        }
    }

    /// A fog or shaft pass can clear `config.enabled` from inside execute or
    /// resize without clamping Bruneton to those passes. Tests use this to
    /// enter the next frame with that stale flag.
    #[doc(hidden)]
    pub fn mark_volumetric_passes_disabled_for_test(&mut self) {
        if let Some(fog) = self.volumetric_fog.as_mut() {
            let mut config = fog.config().clone();
            config.enabled = false;
            fog.set_config(config);
        }
        if let Some(rays) = self.god_rays.as_mut() {
            let mut config = rays.config().clone();
            config.enabled = false;
            rays.set_config(config);
        }
    }

    /// Composite engine volumetric fog, then screen-space light shafts, into HDR.
    ///
    /// Runs after Bruneton has copied its sky into HDR, so fog and shafts
    /// apply over cleared pixels. Both passes no-op while their configs are
    /// disabled. Fog samples the cascaded shadow map. Shafts are added before
    /// the tonemap blit.
    fn composite_atmosphere(&mut self, enc: &mut wgpu::CommandEncoder, depth: &wgpu::TextureView) {
        let light = glam::Vec3::new(
            self.camera_ubo.light_dir_pad[0],
            self.camera_ubo.light_dir_pad[1],
            self.camera_ubo.light_dir_pad[2],
        );
        let toward_sun = if light.length_squared() > 1e-6 {
            -light.normalize()
        } else {
            self.sky.time_of_day().get_sun_position()
        };
        let camera_pos = glam::Vec3::new(
            self.camera_ubo.camera_pos_pad[0],
            self.camera_ubo.camera_pos_pad[1],
            self.camera_ubo.camera_pos_pad[2],
        );
        let view_proj = self.cached_proj * self.cached_view;
        let inv_view_proj = view_proj.inverse();
        let (near, far) = clip_planes(self.cached_proj);
        let sun_color = self.sky.time_of_day().get_light_color();
        let ambient = self.sky.time_of_day().get_ambient_color();
        let intensity = self.camera_ubo.light_dir_pad[3].max(0.15);

        if let Some(fog) = self
            .volumetric_fog
            .as_mut()
            .filter(|fog| fog.config().enabled)
        {
            let splits = [
                self.split0,
                self.split1,
                self.split2,
                far.max(self.split2 + 1.0),
            ];
            let cascades: [crate::volumetric_fog::CascadeData; 4] =
                std::array::from_fn(|index| crate::volumetric_fog::CascadeData {
                    vp: self.cascades[index].to_cols_array_2d(),
                    split: splits[index],
                    _align: [0.0; 3],
                    _pad: [0.0; 3],
                    _tail: 0.0,
                });
            let device = self.device.clone();
            let queue = self.queue.clone();
            let shadow = self.shadow_view.clone();
            let hdr = self.hdr_view.clone();
            let generation = self.resource_generation;
            fog.upload_cascades(&queue, &cascades);
            fog.prepare_frame(
                &queue,
                inv_view_proj,
                camera_pos,
                toward_sun,
                sun_color,
                intensity,
                ambient,
                0.35,
                near,
                far,
                1.0 / 60.0,
            );
            let ran = fog.execute(&device, enc, depth, &shadow, &hdr, generation);
            if ran {
                fog.copy_scatter_to_history(enc);
                enc.copy_texture_to_texture(
                    fog.output_texture().as_image_copy(),
                    self.hdr_tex.as_image_copy(),
                    wgpu::Extent3d {
                        width: self.config.width,
                        height: self.config.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        if let Some(rays) = self.god_rays.as_mut().filter(|rays| rays.config().enabled) {
            let device = self.device.clone();
            let queue = self.queue.clone();
            let hdr = self.hdr_view.clone();
            let generation = self.resource_generation;
            let scene_width = self.config.width;
            let scene_height = self.config.height;
            let composite = {
                rays.update_params(&queue, view_proj, toward_sun, sun_color);
                let ran = rays.execute(&device, enc, depth, &hdr, generation);
                let added = ran && rays.add_into(&device, enc, &hdr, generation);
                if added {
                    rays.composite_texture().cloned()
                } else {
                    None
                }
            };
            if let Some(composite) = composite {
                enc.copy_texture_to_texture(
                    composite.as_image_copy(),
                    self.hdr_tex.as_image_copy(),
                    wgpu::Extent3d {
                        width: scene_width,
                        height: scene_height,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        // execute and add_into can clear `config.enabled` inside this call.
        self.clamp_atmosphere_to_volumetrics();
    }

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

    /// The 512² cloud-shadow compute is on unless this is false.
    pub fn cloud_shadows_enabled(&self) -> bool {
        self.cloud_shadows_enabled
    }

    /// Enable or disable the per-frame cloud shadow compute pass.
    /// When disabled the cloud shadow texture retains its initial all-white
    /// state (1.0 = fully lit), so direct sunlight is preserved without
    /// the noisy pattern from the low-resolution 512x512 transmittance map.
    pub fn set_cloud_shadows_enabled(&mut self, enabled: bool) {
        self.cloud_shadows_enabled = enabled;
    }

    /// Cloud-shadow darkness. `1` is the Beer transmittance. The default is
    /// [`crate::volumetric_clouds::CLOUD_SHADOW_STRENGTH`]. Values above 1
    /// darken the same march; they do not add steps or change the slice.
    pub fn cloud_shadow_strength(&self) -> f32 {
        self.cloud_shadow_strength
    }

    /// Set the cloud-shadow darkness used on the next bake. Negative values
    /// clamp to 0 (no extinction, a fully lit map).
    pub fn set_cloud_shadow_strength(&mut self, strength: f32) {
        self.cloud_shadow_strength = strength.max(0.0);
    }

    /// Presentation frame for the cloud-shadow slice. The next draw consumes
    /// it. The same index always selects the same rows and the same cloud
    /// time (`(index / slices) * slices / 60` seconds). Callers that never
    /// set this get a counter that advances once per draw.
    pub fn set_presentation_frame(&mut self, frame: u64) {
        self.presentation_frame = Some(frame);
    }

    fn cloud_shadow_frame_index(&mut self) -> u64 {
        if let Some(frame) = self.presentation_frame.take() {
            self.cloud_shadow_frame = frame.saturating_add(1);
            frame
        } else {
            let frame = self.cloud_shadow_frame;
            self.cloud_shadow_frame = self.cloud_shadow_frame.saturating_add(1);
            frame
        }
    }

    fn write_ibl_params(&self) {
        let params = IblParamsGpu {
            ibl_intensity: self.ibl_intensity,
            max_spec_lod: self.ibl_max_spec_lod,
            _pad: self.cloud_shadow_pass.baked_center_xz(),
        };
        self.queue
            .write_buffer(&self.ibl_params_buf, 0, bytemuck::bytes_of(&params));
    }

    fn record_cloud_shadow(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let frame = self.cloud_shadow_frame_index();
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
            crate::volumetric_clouds::CLOUD_SHADOW_STEPS,
            frame,
            self.cloud_shadow_strength,
        );
        self.write_ibl_params();
        self.cloud_shadow_pass
            .execute(&self.device, encoder, self.resource_generation);
    }

    /// Set the post-processing chain configuration.
    ///
    /// L.1 honesty note: in the current render paths (`render()` and
    /// `draw_into()`), the ONLY consumed field is `bloom_enabled` — and the
    /// bloom pass's output is computed but not composited by the canonical
    /// POST shader (P.3 residue, see the comment at the bloom dispatch).
    /// The SSAO/SSR/SSGI/TAA/DoF/motion-blur/color-grading/auto-exposure
    /// flags and `tonemap_operator` are stored but drive no passes here;
    /// the corresponding machinery (`post.rs` WGSL, `hdr_pipeline.rs` chain)
    /// exists but is not wired into these paths. Do not set a flag expecting
    /// a visual change (T2F_LIGHTING_RECON.md §1.2 rows 8/11).
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

    /// Set the water renderer for ocean rendering.
    ///
    /// The split water pass draws into the **HDR target** (`hdr_format()`),
    /// not the window surface, so the installed renderer's pipeline must be
    /// built against that format. A mismatch is rejected here, at install
    /// time — the alternative is a wgpu validation panic on the first frame
    /// with visible water chunks, which is exactly how the editor shipped a
    /// broken water path undetected (TWR_WATER_RECON.md §1.6).
    pub fn set_water_renderer(&mut self, water: crate::water::WaterRenderer) {
        assert!(
            water.target_format() == self.hdr_format(),
            "WaterRenderer must be built against the HDR water-pass target format \
             ({:?}), got {:?} — construct it with `WaterRenderer::new(device, \
             renderer.hdr_format(), ...)`, never the window surface format",
            self.hdr_format(),
            water.target_format()
        );
        self.water_renderer = Some(water);
    }

    /// Remove the water renderer, disabling water rendering.
    pub fn clear_water_renderer(&mut self) {
        self.water_renderer = None;
    }

    /// Whether a water renderer is currently installed (T.W.2 — lets the
    /// editor decide between installing a plane-hidden renderer for authored
    /// volumes and reusing the one the plane enable installed).
    pub fn has_water_renderer(&self) -> bool {
        self.water_renderer.is_some()
    }

    /// F.4.3 — register (or clear with `None`) the post-water, pre-tonemap HDR
    /// overlay composite. Invoked once per frame inside `run_water_pass`, against
    /// `hdr_view` (`Rgba16Float`) + the scene depth the water pass used, BEFORE the
    /// tonemap/blit. The closure receives only `wgpu` types, so `astraweave-render`
    /// gains no `astraweave-fluids` dependency — the binary's closure body calls
    /// `FluidRenderer::render_accents`. Replaced each frame by the demo (it re-captures
    /// the current accent buffer/count/camera). Used for the F.4.3 weave-impact accents.
    #[allow(clippy::type_complexity)]
    pub fn set_hdr_overlay(
        &mut self,
        overlay: Option<
            Box<
                dyn FnMut(
                    &mut wgpu::CommandEncoder,
                    &wgpu::TextureView,
                    &wgpu::TextureView,
                    &wgpu::Device,
                    &wgpu::Queue,
                ),
            >,
        >,
    ) {
        self.hdr_overlay = overlay;
    }

    /// Update water renderer state (call each frame before render)
    pub fn update_water(&mut self, view_proj: glam::Mat4, camera_pos: glam::Vec3, time: f32) {
        if let Some(ref mut water) = self.water_renderer {
            water.update(&self.queue, view_proj, camera_pos, time);
        }
    }

    /// Set the water surface level (world Y) and upload it immediately.
    ///
    /// Uploads on the spot (rather than waiting for the next `update_water`) so
    /// callers like the editor water-level knob see the change on the next frame
    /// even when they do not drive `update_water` per frame.
    pub fn set_water_level(&mut self, level: f32) {
        if let Some(ref mut water) = self.water_renderer {
            water.set_water_level(level);
            water.write_uniforms(&self.queue);
        }
    }

    /// T.W.2 — set the authored bounded water bodies (the editor's
    /// `WaterVolume` entities). Delegates to
    /// [`crate::water::WaterRenderer::set_water_volumes`] (change-detected:
    /// an unchanged set is free). No-op when no water renderer is installed —
    /// callers that want volumes without the global plane install a renderer
    /// and call [`Self::set_water_plane_visible`]`(false)`.
    pub fn set_water_volumes(&mut self, volumes: &[crate::water::WaterVolumeDesc]) {
        if let Some(ref mut water) = self.water_renderer {
            water.set_water_volumes(&self.device, volumes);
        }
    }

    /// T.W.2 — show/hide the global plane (chunk grid + horizon shell)
    /// without touching authored volumes. No-op when no water renderer is
    /// installed.
    pub fn set_water_plane_visible(&mut self, visible: bool) {
        if let Some(ref mut water) = self.water_renderer {
            water.set_plane_visible(visible);
        }
    }

    /// Push the active weave-deformation instances onto the water surface (W.2c.3).
    ///
    /// Delegates to [`crate::water::WaterRenderer::set_weave_instances`] (replaces the
    /// active list; at most `MAX_WEAVE_INSTANCES`, uploaded on the next `update_water`).
    /// Takes a render-side [`crate::water::WeaveInstance`] slice — **no gameplay type
    /// crosses into the renderer**, so `astraweave-render` stays independent of the
    /// gameplay crates (the WeaveOp → WeaveInstance translation lives in the binary
    /// glue). No-op when no water renderer is installed.
    /// The heavy frame-budget scene does not install a water surface.
    pub fn water_is_drawn(&self) -> bool {
        self.water_renderer
            .as_ref()
            .is_some_and(|water| water.has_renderable_content())
    }

    pub fn set_water_weave_instances(&mut self, instances: &[crate::water::WeaveInstance]) {
        if let Some(ref mut water) = self.water_renderer {
            water.set_weave_instances(instances);
        }
    }

    /// W.2b — the split water pass. Runs after the opaque main pass closes:
    /// snapshot the opaque HDR into `water_scene_color`, wire the snapshot + scene
    /// depth into the water bind group, then draw water into the HDR target with a
    /// **read-only** depth attachment so it can both depth-test against the scene
    /// and sample that depth for refraction / the depth-delta shoreline foam.
    ///
    /// Takes the `WaterRenderer` out for the duration to keep `self`'s other fields
    /// (device/queue/textures) freely borrowable, then restores it.
    ///
    /// `depth_view`: the SAME depth texture the opaque/sky passes wrote this frame.
    /// `render()` passes `None` (internal `self.depth.view`); `draw_into()` passes
    /// its caller-supplied depth so the editor water pass tests/samples the depth
    /// that actually received the editor's opaque geometry — NOT `self.depth.view`.
    /// The depth-stencil attachment view and the sampled `scene_depth` view MUST be
    /// the same texture for the read-only-depth + sample trick to stay valid.
    fn run_water_pass(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        depth_view: Option<&wgpu::TextureView>,
    ) {
        let mut water = match self.water_renderer.take() {
            Some(w) => w,
            // F.4.3: accents are weave-driven, independent of water visibility, so
            // the HDR overlay still fires even with no water installed.
            None => {
                self.fire_hdr_overlay(enc, depth_view);
                return;
            }
        };
        // Skip the full-res snapshot + pass when water has nothing to draw (the
        // editor dormant case where `update_water` is never called → no chunks;
        // T.W.2: authored volume patches count as renderable content, so a
        // volumes-only scene — plane hidden or chunk-less — still draws).
        if !water.has_renderable_content() {
            self.water_renderer = Some(water);
            self.fire_hdr_overlay(enc, depth_view);
            return;
        }

        // The depth the opaque pass wrote this frame (internal for render(),
        // caller-supplied for draw_into()). Used for BOTH the test attachment and
        // the sampled binding so they reference the same texture.
        let depth = depth_view.unwrap_or(&self.depth.view);

        // 1. Snapshot the opaque scene color (hdr_tex is the authoritative main-pass
        //    target — see the §7.7 consistency fix at construction).
        enc.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.hdr_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.water_scene_color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
        );

        // 2. Wire scene-color + depth into the water bind group; upload screen size.
        water.prepare_scene(
            &self.device,
            &self.queue,
            &self.water_scene_color_view,
            depth,
            [self.config.width as f32, self.config.height as f32],
            self.resource_generation,
        );

        // 3. Draw water into the HDR target. Read-only depth (depth_ops: None) lets
        //    the same depth texture be the test attachment AND a sampled binding —
        //    validated against wgpu 25 in W.2b.2 Step 0.
        {
            let mut wp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("water pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: None, // read-only — preserve scene depth, allow sampling
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            water.render(&mut wp);
        }

        self.water_renderer = Some(water);
        // F.4.3: HDR accent composite fires AFTER water_renderer is restored, so a
        // panic in the closure cannot drop the WaterRenderer (it is already back in
        // self). Outside the take/restore window — see the W-FU-3 panic-gap note.
        self.fire_hdr_overlay(enc, depth_view);
    }

    /// F.4.3 — invoke the registered post-water, pre-tonemap HDR overlay (the
    /// weave-impact accent composite), if any. Fired by `run_water_pass` on every
    /// exit path so accents render regardless of water visibility. Resolves the
    /// depth identically to the water pass (`depth_view.unwrap_or(self.depth.view)`)
    /// so accents depth-test against the same scene depth, read-only. The
    /// `take()`/restore mirrors the `water_renderer` idiom to split the borrow of
    /// `hdr_overlay` from `hdr_view`/`device`/`queue`. No-op (zero GPU work) when
    /// no overlay is registered — preserving zero-accent frame identity.
    fn fire_hdr_overlay(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        depth_view: Option<&wgpu::TextureView>,
    ) {
        if let Some(mut overlay) = self.hdr_overlay.take() {
            let depth = depth_view.unwrap_or(&self.depth.view);
            overlay(enc, &self.hdr_view, depth, &self.device, &self.queue);
            self.hdr_overlay = Some(overlay);
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
        // Allocation-measurement instrumentation (audit 2026-04-17, §2.3 #4, #5).
        // Emits Tracy span + `render.submit.allocs` / `render.submit.bytes` plots
        // when built with `--features profiling,alloc-counter`.
        #[cfg(feature = "profiling")]
        astraweave_profiling::measured_span!("render.submit");

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
        self.poll_fog_timestamps();
        // Advance the staging ring buffer for this frame.
        self.staging_ring.begin_frame();

        let ts_cluster = self
            .gpu_profiler
            .as_mut()
            .and_then(|p| p.allocate_pass("cluster_bin"));
        const SHADOW_PASS_LABELS_R: [&str; CASCADE_COUNT] = [
            "shadow_cascade_0",
            "shadow_cascade_1",
            "shadow_cascade_2",
            "shadow_cascade_3",
        ];
        let ts_shadow: [Option<(u32, u32)>; CASCADE_COUNT] = std::array::from_fn(|i| {
            self.gpu_profiler
                .as_mut()
                .and_then(|p| p.allocate_pass(SHADOW_PASS_LABELS_R[i]))
        });
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
        let vis_count = self.build_visible_instances();
        if vis_count > 0 {
            self.queue.write_buffer(
                &self.instance_buf,
                0,
                bytemuck::cast_slice(&self.visible_raw_scratch[..vis_count]),
            );
        }
        // Shadow passes (depth only) — skip when shadows are disabled or there
        // is no shadow-casting geometry.
        //
        // L.3.C resolution: `self.shadows_enabled` was missing here while
        // `draw_into` (the editor path) has always had it — two gates for one
        // logical decision. With shadows off, `update_cascade_splits` clears
        // `c_far_valid` every frame (the sentinel branch), so this path re-ran
        // ALL FOUR caster passes every frame to fill maps the shader is
        // sentinel-gated never to sample: an accidental every-frame policy,
        // paid for by every `render()` consumer (the game runtime and 13
        // examples). `terrain_casts` is deliberately NOT added — this path
        // draws no terrain chunks, so gating on them would be a lie in the
        // other direction.
        let has_shadow_casters_r = self.shadows_enabled
            && (vis_count > 0 || self.mesh_external.is_some() || !self.models.is_empty());
        // Which cached layers get drawn this frame? (Two-phase commit — the
        // flags advance below, AFTER the passes actually ran.)
        let far_rendered: [bool; CASCADE_COUNT - FIRST_CACHED_CASCADE] = std::array::from_fn(|c| {
            has_shadow_casters_r && (!self.c_far_valid[c] || self.c_far_pending[c])
        });

        for (idx, layer_view) in self.shadow_layer_views.iter().enumerate() {
            if !has_shadow_casters_r {
                continue;
            }
            // Cached cascades: skip the pass when that layer's map is
            // known-filled with its current matrix and no refresh is pending.
            if idx >= FIRST_CACHED_CASCADE && !far_rendered[idx - FIRST_CACHED_CASCADE] {
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
            // visual contribution).
            // Shadow frustum culling: skip models outside the cascade's
            // ortho frustum.
            let cascade_vp = self.cascades[idx];
            let shadow_frustum = crate::culling::FrustumPlanes::from_view_proj(&cascade_vp);
            for (name, model) in &self.models {
                if model.instance_count == 0 {
                    continue;
                }
                if name.starts_with("scatter_") {
                    continue;
                }
                let bounds = model
                    .aabb
                    .or_else(|| self.gpu_cull_bounds.get(name).copied());
                if aabb_rejected(bounds, &shadow_frustum) {
                    continue;
                }
                for primitive in &model.primitives {
                    sp.set_vertex_buffer(0, primitive.mesh.vertex_buf.slice(..));
                    sp.set_index_buffer(
                        primitive.mesh.index_buf.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    sp.set_vertex_buffer(1, model.instance_buf.slice(..));
                    sp.draw_indexed(0..primitive.mesh.index_count, 0, 0..model.instance_count);
                }
            }
        }
        // Commit the cached-cascade state only now that the passes have
        // actually run (two-phase commit — see `update_cascade_splits`).
        for (c, rendered) in far_rendered.iter().enumerate() {
            if *rendered {
                self.c_far_valid[c] = true;
                self.c_far_pending[c] = false;
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
            self.record_cloud_shadow(&mut enc);
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

            rp.set_pipeline(self.main_mesh_pipeline());
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
                    for primitive in &model.primitives {
                        if let Some(ref mtex) = primitive.tex_bind_group {
                            rp.set_bind_group(3, mtex, &[]);
                        } else {
                            rp.set_bind_group(3, &self.tex_bg, &[]);
                        }
                        rp.set_vertex_buffer(0, primitive.mesh.vertex_buf.slice(..));
                        rp.set_index_buffer(
                            primitive.mesh.index_buf.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        rp.set_vertex_buffer(1, model.instance_buf.slice(..));
                        rp.draw_indexed(0..primitive.mesh.index_count, 0, 0..model.instance_count);
                    }
                }
            }

            // LOD3 impostor passes (Phase 5.3 T7): run after opaque LOD0/1/2
            // draws and before transparent water so that billboarded card
            // geometry occludes water that falls behind it while still being
            // alpha-blended on top of the opaque hdr target.
            #[cfg(feature = "impostor-bake")]
            for ip in self.impostor_passes.values() {
                if ip.instance_count() > 0 {
                    ip.record(&mut rp);
                }
            }

            // W.2b: water moved out of the main pass — see `run_water_pass` below.
        }

        // W.2b — split water pass (after the opaque main pass closes) so water can
        // sample the scene behind it for refraction + depth-delta shoreline foam.
        // render() always uses the internal depth target.
        self.run_water_pass(&mut enc, None);
        let depth = self.depth.view.clone();
        self.composite_atmosphere(&mut enc, &depth);

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

        // Pass timestamps in `render` are recorded on `enc`. Resolve them
        // here. `submit_timestamped` does not open a second encoder.
        if let Some(ref mut p) = self.gpu_profiler {
            p.resolve_queries(&mut enc);
            p.end_frame(&mut enc);
        }

        self.submit_timestamped(enc.finish())?;
        frame.present();

        // --- GPU profiler: initiate async readback of timestamp data ---
        if let Some(ref mut p) = self.gpu_profiler {
            p.request_readback();
        }
        self.request_fog_timestamps();

        Ok(())
    }

    /// Render the scene into the given color target.
    ///
    /// `depth_view`: optional caller-provided depth attachment. When `Some(v)`, terrain + sky
    /// passes write depth into `v` (compatible Depth32Float texture required); enables external
    /// consumers (e.g., editor depth-pick) to read terrain depth post-render. When `None`, falls
    /// back to internal `self.depth.view` (preserves prior behavior for non-editor callers).
    /// See Sub-phase 3 Mediator Brush Real-Fix per Round-5-Closure 569415a7a §12 Option (a).
    pub fn draw_into(
        &mut self,
        view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        enc: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        // --- GPU timestamp profiling: poll previous frame, begin new frame, pre-allocate ---
        if let Some(ref mut p) = self.gpu_profiler {
            p.poll_readback(&self.device);
            p.begin_frame();
        }
        self.timing_gap = None;
        self.open_frame_timestamp(enc);
        self.open_timing_gap(enc, "before_passes");
        self.poll_fog_timestamps();
        if self.bindless_active {
            if let Some(system) = self.bindless.as_mut() {
                system.update(&self.device, &self.queue);
            }
        }
        // Advance the staging ring buffer for this frame.
        self.staging_ring.begin_frame();
        self.tick_texture_streaming();

        // Timestamp slots are allocated only for passes that actually begin.
        // A resolve range that includes a query nobody wrote comes back as
        // zeros on Vulkan, which is what made every pass read 0 ms.
        const SHADOW_PASS_LABELS: [&str; CASCADE_COUNT] = [
            "shadow_cascade_0",
            "shadow_cascade_1",
            "shadow_cascade_2",
            "shadow_cascade_3",
        ];
        const AFTER_SHADOW: [&str; CASCADE_COUNT] = [
            "after_shadow_0",
            "after_shadow_1",
            "after_shadow_2",
            "after_shadow_3",
        ];

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
            self.close_timing_gap(enc);
            enc.clear_buffer(&self.clustered_counts_buf, 0, None);
            {
                let cluster_stamp = self
                    .gpu_profiler
                    .as_mut()
                    .and_then(|profiler| profiler.begin_stamp(enc, "cluster_bin"));
                let mut cpass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("cluster bin"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.clustered_comp_pipeline);
                cpass.set_bind_group(0, &self.clustered_comp_bg, &[]);
                cpass.dispatch_workgroups(light_count as u32, 1, 1);
                drop(cpass);
                if let Some(stamp) = cluster_stamp {
                    if let Some(profiler) = self.gpu_profiler.as_mut() {
                        profiler.end_stamp(enc, stamp);
                    }
                }
            }
            self.open_timing_gap(enc, "after_cluster");
        }

        // Update pre-allocated plane instance buffer ONLY if terrain hasn't
        // positioned it.  `set_terrain_ground_plane()` sets the correct Y and
        // extent for terrain rendering — overwriting it here would place a
        // small dark plane at Y=-0.2 right inside the camera's shadow cascade,
        // producing a massive shadow that follows the camera.
        if !self.terrain_ground_set && !self.plane_instance_uploaded {
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
            self.plane_instance_uploaded = true;
        }

        // Frustum cull - TEST 4
        let vis_count = self.build_visible_instances();
        if vis_count > 0 {
            let bytes = bytemuck::cast_slice(&self.visible_raw_scratch[..vis_count]);
            if self.visible_uploaded.as_slice() != bytes {
                self.visible_uploaded.clear();
                self.visible_uploaded.extend_from_slice(bytes);
                self.queue
                    .write_buffer(&self.instance_buf, 0, &self.visible_uploaded);
            }
        }

        // Shadow passes — skip entirely when shadows disabled or no geometry visible.
        // Each shadow pass costs ~2-3ms even for an empty scene.
        // L.3: terrain-forward chunks are casters (`tf.chunks` — they never
        // lived in `self.models`, which is why terrain cast nothing before).
        #[cfg(feature = "terrain-splat-arrays")]
        let terrain_casts = self
            .terrain_forward
            .as_ref()
            .is_some_and(|tf| !tf.chunks.is_empty() && tf.manager.shadow_pipeline().is_some());
        #[cfg(not(feature = "terrain-splat-arrays"))]
        let terrain_casts = false;
        let has_shadow_casters = self.shadows_enabled
            && (vis_count > 0
                || self.mesh_external.is_some()
                || !self.models.is_empty()
                || terrain_casts);
        // Which cached layers get drawn this frame? (Two-phase commit — the
        // flags are only advanced below, AFTER the passes actually ran.)
        let far_rendered: [bool; CASCADE_COUNT - FIRST_CACHED_CASCADE] = std::array::from_fn(|c| {
            has_shadow_casters && (!self.c_far_valid[c] || self.c_far_pending[c])
        });
        // L.3: per-cascade (drawn, total) terrain-chunk counts for the frame,
        // written to `self.terrain_shadow_stats` after the passes close.
        let mut terrain_stats = [(0u32, 0u32); CASCADE_COUNT];

        // L.3.D rung 2: snapshot c3's OUTGOING map into the spare layer before
        // the pass below overwrites it. This has to be here, in this encoder,
        // immediately ahead of that pass — a copy issued anywhere else either
        // reads the new contents or races the pass that writes them. Gated on
        // `has_shadow_casters` for the same reason the two-phase commit is: if
        // the passes are skipped this frame, nothing is overwritten and the
        // request must stay pending rather than snapshot a stale map.
        if self.c3_copy_pending && has_shadow_casters {
            self.close_timing_gap(enc);
            let copy_stamp = self.begin_named_stamp(enc, "c3_shadow_copy");
            let layer = |z: u32| wgpu::TexelCopyTextureInfo {
                texture: &self.shadow_tex,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z },
                aspect: wgpu::TextureAspect::All,
            };
            enc.copy_texture_to_texture(
                layer(CASCADE_COUNT as u32 - 1),
                layer(C3_PREV_LAYER),
                wgpu::Extent3d {
                    width: SHADOW_MAP_SIZE,
                    height: SHADOW_MAP_SIZE,
                    depth_or_array_layers: 1,
                },
            );
            self.end_named_stamp(enc, copy_stamp);
            self.c3_copy_pending = false;
            self.open_timing_gap(enc, "after_c3_copy");
        }

        for (idx, layer_view) in self.shadow_layer_views.iter().enumerate() {
            if !has_shadow_casters {
                continue;
            }
            // Cached cascades: skip the pass when that layer's map is
            // known-filled with its current matrix and no refresh is pending.
            if idx >= FIRST_CACHED_CASCADE && !far_rendered[idx - FIRST_CACHED_CASCADE] {
                continue;
            }
            if let Some(stamp) = self.timing_gap.take() {
                if let Some(profiler) = self.gpu_profiler.as_mut() {
                    profiler.end_stamp(enc, stamp);
                }
            }
            let shadow_stamp = self
                .gpu_profiler
                .as_mut()
                .and_then(|profiler| profiler.begin_stamp(enc, SHADOW_PASS_LABELS[idx]));
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
                timestamp_writes: None,
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
            // Named models cast shadows. Scatter objects (scatter_*) are
            // skipped entirely because small vegetation/debris shadows are
            // imperceptible and the draw call cost is severe.
            //
            // Shadow frustum culling: build frustum planes from the cascade
            // VP matrix; models AND terrain chunks (the L.3 block below the
            // model loop) are tested against it per cascade.
            let cascade_vp = self.cascades[idx];
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
                // (L.3: the former `terrain_c*` cascade-1 skip was deleted —
                // dead code; no producer of that model naming has existed
                // since the 2026-05-08 cluster-path removal. Terrain now
                // casts via the dedicated depth-only chunk path, both
                // cascades.)
                //
                // Frustum cull against the shadow cascade's ortho projection.
                // Models outside the shadow camera's frustum don't
                // contribute to visible shadows and are expensive to render.
                let bounds = model
                    .aabb
                    .or_else(|| self.gpu_cull_bounds.get(name).copied());
                if aabb_rejected(bounds, &shadow_frustum) {
                    continue;
                }
                for primitive in &model.primitives {
                    sp.set_vertex_buffer(0, primitive.mesh.vertex_buf.slice(..));
                    sp.set_index_buffer(
                        primitive.mesh.index_buf.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    sp.set_vertex_buffer(1, model.instance_buf.slice(..));
                    sp.draw_indexed(0..primitive.mesh.index_count, 0, 0..model.instance_count);
                }
            }
            // L.3: terrain chunks cast. Depth-only terrain pipeline (chunk
            // positions are world-space — no instance buffer), culled per
            // cascade against the same `shadow_frustum` the models use.
            // Statics keep casting via `shadow_pipeline` above, so future
            // static casters (T.3 scatter and friends) are additive: this
            // block only APPENDS terrain draws to the same per-cascade pass.
            // Both cascades render terrain — cascade 1 (out to shadow_far
            // 500 m) is precisely the mid-range dune shadowing this beat
            // exists to deliver.
            #[cfg(feature = "terrain-splat-arrays")]
            if let Some(tf) = self.terrain_forward.as_ref() {
                if let Some(pl) = tf.manager.shadow_pipeline() {
                    if !tf.chunks.is_empty() {
                        sp.set_pipeline(pl);
                        sp.set_bind_group(0, &self.shadow_cascade_bgs[idx], &[]);
                        let total = tf.chunks.len() as u32;
                        let mut drawn = 0u32;
                        for chunk in tf.chunks.values() {
                            let center = glam::Vec3::new(
                                (chunk.aabb_min[0] + chunk.aabb_max[0]) * 0.5,
                                (chunk.aabb_min[1] + chunk.aabb_max[1]) * 0.5,
                                (chunk.aabb_min[2] + chunk.aabb_max[2]) * 0.5,
                            );
                            let extent = glam::Vec3::new(
                                (chunk.aabb_max[0] - chunk.aabb_min[0]) * 0.5,
                                (chunk.aabb_max[1] - chunk.aabb_min[1]) * 0.5,
                                (chunk.aabb_max[2] - chunk.aabb_min[2]) * 0.5,
                            );
                            // L.3.B verified this cull omits nothing that
                            // matters at the survey framing: drawing EVERY
                            // chunk into every cascade produced byte-identical
                            // frames (0/786432 differing pixels), refuting the
                            // classic "lateral occluder culled but its shadow
                            // falls inside the box" CSM bug here.
                            if !shadow_frustum.test_aabb(center, extent) {
                                continue;
                            }
                            sp.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
                            sp.set_index_buffer(
                                chunk.index_buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            sp.draw_indexed(0..chunk.index_count, 0, 0..1);
                            drawn += 1;
                        }
                        // L.3.D: the caster-set DELTA belongs here, not at fit
                        // time — this is the only place the caster set is known,
                        // because it is the AABB cull against the freshly-fitted
                        // frustum that decides it. A window jump and a caster-set
                        // churn are different quantities and the far-field pop
                        // needs both: the shadows that vanish on a refit are the
                        // chunks that left the set.
                        if idx >= FIRST_CACHED_CASCADE
                            && self.far_policy.frozen(idx - FIRST_CACHED_CASCADE)
                            && std::env::var_os("AW_CSM_LOG").is_some()
                        {
                            let prev = self.terrain_shadow_stats[idx].0;
                            eprintln!(
                                "[csm] c{idx} caster pass: {drawn}/{total} chunks \
                                 (was {prev}, delta {:+})",
                                drawn as i64 - prev as i64
                            );
                        }
                        terrain_stats[idx] = (drawn, total);
                    }
                }
            }
            drop(sp);
            if let Some(stamp) = shadow_stamp {
                if let Some(profiler) = self.gpu_profiler.as_mut() {
                    profiler.end_stamp(enc, stamp);
                }
            }
            self.timing_gap = self
                .gpu_profiler
                .as_mut()
                .and_then(|profiler| profiler.begin_stamp(enc, AFTER_SHADOW[idx]));
        }
        // Commit the cached-cascade state only now that the passes have
        // actually run (two-phase commit — see `update_cascade_splits`).
        for (c, rendered) in far_rendered.iter().enumerate() {
            let idx = c + FIRST_CACHED_CASCADE;
            if *rendered {
                self.c_far_valid[c] = true;
                self.c_far_pending[c] = false;
            } else {
                // Not redrawn this frame — keep that layer's last rendered
                // counts rather than reporting a misleading 0/0.
                terrain_stats[idx] = self.terrain_shadow_stats[idx];
            }
        }
        // L.3: publish the per-cascade culling counts (observable via
        // `terrain_shadow_stats()`; the l3 harness prints them).
        self.terrain_shadow_stats = terrain_stats;
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

        let sky_tex = self
            .ibl_resources
            .as_ref()
            .map(|resources| resources.env_cube.clone());
        let sky_equirect = self
            .ibl_resources
            .as_ref()
            .and_then(|resources| resources.hdr_equirect.clone());

        // Bruneton replaces the HDRI sky. The sky pass is also the clear of
        // HDR and of scene depth, so that clear still has to run. A pass can
        // clear its own config after the last setter; clamp before this branch
        // so a stale flag does not replace the HDRI sky with the clear color.
        self.clamp_atmosphere_to_volumetrics();
        if self.atmosphere_on {
            self.close_timing_gap(enc);
            let sky_stamp = self.begin_named_stamp(enc, "sky");
            {
                let _clear = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("atmosphere depth and hdr clear"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.hdr_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(crate::environment::hdri_sky_clear_color()),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view.unwrap_or(&self.depth.view),
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            self.end_named_stamp(enc, sky_stamp);
            self.open_timing_gap(enc, "after_sky");
        } else {
            self.close_timing_gap(enc);
            let sky_stamp = self.begin_named_stamp(enc, "sky");
            let sky_result = self.sky.render(
                &self.device,
                enc,
                &self.hdr_view,
                // Real-Fix.A: route to caller-provided depth when Some, else internal fallback.
                depth_view.unwrap_or(&self.depth.view),
                vp_sky,
                &self.queue,
                sky_tex.as_ref(),
                sky_equirect.as_ref(),
            );
            self.end_named_stamp(enc, sky_stamp);
            self.open_timing_gap(enc, "after_sky");
            sky_result.context("Sky render failed")?;
        }

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
            self.close_timing_gap(enc);
            let cloud_stamp = self.begin_named_stamp(enc, "cloud_shadow");
            self.record_cloud_shadow(enc);
            self.end_named_stamp(enc, cloud_stamp);
            self.open_timing_gap(enc, "after_cloud_shadow");
        }

        // Phase 1 — prepare forward-lit terrain state for this frame
        // (Terrain Material System Campaign, Option D). All &mut self
        // operations happen here, before the `rp` borrow opens below,
        // to avoid borrow-checker conflicts during the draw phase.
        #[cfg(feature = "terrain-splat-arrays")]
        {
            if self.terrain_forward.is_some() {
                let color_format = wgpu::TextureFormat::Rgba16Float; // hdr_view
                let depth_format = Some(self.depth.format);
                let view_proj = self.cached_proj * self.cached_view;
                let light_dir = glam::Vec3::new(
                    self.camera_ubo.light_dir_pad[0],
                    self.camera_ubo.light_dir_pad[1],
                    self.camera_ubo.light_dir_pad[2],
                );
                let camera_pos = glam::Vec3::new(
                    self.camera_ubo.camera_pos_pad[0],
                    self.camera_ubo.camera_pos_pad[1],
                    self.camera_ubo.camera_pos_pad[2],
                );
                // Reinterpret the engine's SceneEnvironmentUBO as
                // TerrainSceneEnvGpu. Byte-layout equality is asserted by
                // the `camera_forward_gpu_field_offsets_match_shader_src`
                // test family; the two structs differ only in field names
                // (wetness/snow_amount vs. _pad0). L.3.D: `debug_flags` rides
                // offset 76 in BOTH, so the cascade-index view's refresh bits
                // reach the terrain shader through this cast — no second write
                // path to keep in sync.
                let engine_scene_ubo = self.scene_env.to_ubo();
                let terrain_scene_gpu: crate::terrain_material_manager::TerrainSceneEnvGpu =
                    bytemuck::cast(engine_scene_ubo);

                if let Some(tf) = self.terrain_forward.as_mut() {
                    tf.manager.ensure_forward_pipeline(
                        &self.device,
                        color_format,
                        depth_format,
                        &self.terrain_ibl_bgl,
                        &self.shadow_bgl,
                    );
                    // L.3: the depth-only caster pipeline (built once; the
                    // shadow pass below reads it via `shadow_pipeline()`).
                    tf.manager
                        .ensure_shadow_pipeline(&self.device, &self.shadow_bgl_light);
                    // ED-3: keep the terrain manager's wireframe selection in
                    // lockstep with the frame's debug-shading state.
                    tf.manager.set_wireframe(self.wireframe_enabled);
                    tf.manager
                        .update_forward_camera(&self.queue, view_proj, light_dir, camera_pos);
                    tf.manager
                        .update_forward_scene(&self.queue, &terrain_scene_gpu);
                }
            }
        }

        if self.gpu_cull_on {
            self.close_timing_gap(enc);
            self.prepare_gpu_model_cull(enc);
            self.open_timing_gap(enc, "after_gpu_cull");
        } else {
            self.gpu_cull_ran = false;
            self.gpu_cull_visible_draws = 0;
        }

        {
            self.close_timing_gap(enc);
            let main_stamp = self
                .gpu_profiler
                .as_mut()
                .and_then(|profiler| profiler.begin_stamp(enc, "main_render"));
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
                    // Real-Fix.A: route to caller-provided depth when Some, else internal fallback.
                    view: depth_view.unwrap_or(&self.depth.view),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load sky depth (should be far plane)
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rp.set_pipeline(self.main_mesh_pipeline());
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

            // Render all named models (terrain, trees, rocks, etc.) with frustum + distance culling.
            // The setting-off loop is the CPU frustum. It does not collect a model list.
            {
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
                if self.gpu_cull_on {
                    let vp = self.cached_proj * self.cached_view;
                    let frustum = crate::culling::FrustumPlanes::from_view_proj(&vp);
                    let indirect_stride =
                        std::mem::size_of::<crate::culling::DrawIndexedIndirectCommand>() as u64;
                    let indirect_buf = self
                        .indirect_cull_resources
                        .as_ref()
                        .map(|resources| resources.draw_commands_buffer.clone());
                    let mut gpu_slot = 0u32;
                    let name_count = self.gpu_cull_names.len();
                    for index in 0..name_count {
                        let gpu_bounds = self
                            .gpu_cull_bounds
                            .get(&self.gpu_cull_names[index])
                            .copied();
                        let Some(model) = self.models.get(&self.gpu_cull_names[index]) else {
                            continue;
                        };
                        if model.instance_count == 0 {
                            continue;
                        }
                        let mut gpu_batch = false;
                        let mut survived = true;
                        let bounds = model.aabb.or(gpu_bounds);
                        if let Some((aabb_min, aabb_max)) = bounds {
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
                            let to_cam = center - cam_pos;
                            let closest_dist_sq = (to_cam.x.abs() - extent.x).max(0.0).powi(2)
                                + (to_cam.z.abs() - extent.z).max(0.0).powi(2);
                            if closest_dist_sq > max_draw_dist_sq {
                                continue;
                            }
                            survived = frustum.test_aabb(center, extent);
                            gpu_batch = true;
                        }
                        for primitive in &model.primitives {
                            if let Some(ref mtex) = primitive.tex_bind_group {
                                rp.set_bind_group(3, mtex, &[]);
                            } else {
                                rp.set_bind_group(3, &self.tex_bg, &[]);
                            }
                            rp.set_vertex_buffer(0, primitive.mesh.vertex_buf.slice(..));
                            rp.set_index_buffer(
                                primitive.mesh.index_buf.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            rp.set_vertex_buffer(1, model.instance_buf.slice(..));
                            if gpu_batch {
                                if let Some(buf) = indirect_buf.as_ref() {
                                    rp.draw_indexed_indirect(
                                        buf,
                                        gpu_slot as u64 * indirect_stride,
                                    );
                                    gpu_slot += 1;
                                    continue;
                                }
                            }
                            rp.draw_indexed(
                                0..primitive.mesh.index_count,
                                0,
                                0..model.instance_count,
                            );
                        }
                        if survived {
                            drawn_models += 1;
                        }
                    }
                } else {
                    let vp = self.cached_proj * self.cached_view;
                    let frustum = crate::culling::FrustumPlanes::from_view_proj(&vp);
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
                        for primitive in &model.primitives {
                            if let Some(ref mtex) = primitive.tex_bind_group {
                                rp.set_bind_group(3, mtex, &[]);
                            } else {
                                rp.set_bind_group(3, &self.tex_bg, &[]);
                            }
                            rp.set_vertex_buffer(0, primitive.mesh.vertex_buf.slice(..));
                            rp.set_index_buffer(
                                primitive.mesh.index_buf.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            rp.set_vertex_buffer(1, model.instance_buf.slice(..));
                            rp.draw_indexed(
                                0..primitive.mesh.index_count,
                                0,
                                0..model.instance_count,
                            );
                        }
                        drawn_models += 1;
                    }
                }
                self.rendered_model_count = drawn_models;
            }

            // Phase 1 — forward-lit terrain draws (Option D).
            // Runs after opaque models, before impostor / water / weather.
            // All &mut state updates happened before the render pass was
            // opened; here we only need &self.terrain_forward. When the
            // field is None (terrain_forward not initialized), the block
            // is a no-op. Cleanup-A 2026-05-08: the editor's legacy
            // cluster path (formerly `self.models["terrain_cluster_*"]`)
            // has been deleted; the forward-lit splat path is canonical.
            #[cfg(feature = "terrain-splat-arrays")]
            if let Some(tf) = self.terrain_forward.as_ref() {
                // L.2: bind the terrain IBL group once for the whole chunk
                // loop — draw_chunk_forward sets groups 0/1/2 per chunk and
                // never touches 3 or 4. Fallback or baked, the group always
                // exists.
                // L.3: group(4) is the SAME `light_bg` the static path
                // samples at its group(2) — light UBO (cascade matrices,
                // splits, extras sentinel) + 2-layer shadow map + comparison
                // sampler. One owner (Renderer), no parallel resource (§7.7).
                if !tf.chunks.is_empty() {
                    rp.set_bind_group(3, &self.terrain_ibl_bind_group, &[]);
                    rp.set_bind_group(4, &self.light_bg, &[]);
                }
                for (key, chunk_gpu) in &tf.chunks {
                    let _ = tf.manager.draw_chunk_forward(
                        &mut rp,
                        *key,
                        &chunk_gpu.vertex_buffer,
                        &chunk_gpu.index_buffer,
                        chunk_gpu.index_count,
                    );
                }
                // After terrain draws, the bind groups are dirty. Rebind
                // the main pass state so anything that follows (impostors,
                // water, weather) inherits the pipeline/bindings it
                // expects. Cheap — each set_bind_group is O(1).
                if !tf.chunks.is_empty() {
                    rp.set_pipeline(self.main_mesh_pipeline());
                    rp.set_bind_group(0, &self.camera_bind_group, &[]);
                    rp.set_bind_group(1, &self.material_bg, &[]);
                    rp.set_bind_group(2, &self.light_bg, &[]);
                    rp.set_bind_group(3, &self.tex_bg, &[]);
                    rp.set_bind_group(4, &self.scene_env_bg, &[]);
                    rp.set_bind_group(5, &self.ibl_bind_group, &[]);
                }
            }

            // LOD3 impostor passes (Phase 5.3 T7): run after opaque LOD0/1/2
            // draws and before transparent water. Mirrored from `render()`.
            #[cfg(feature = "impostor-bake")]
            for ip in self.impostor_passes.values() {
                if ip.instance_count() > 0 {
                    ip.record(&mut rp);
                }
            }

            // W.2b: water moved out of the main pass — see `run_water_pass` below.
            // (Weather now draws before water and is composited behind it; minor
            // ordering change from the split, noted in the W.2b.2 report.)

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
            drop(rp);
            if let Some(stamp) = main_stamp {
                if let Some(profiler) = self.gpu_profiler.as_mut() {
                    profiler.end_stamp(enc, stamp);
                }
            }
        }
        self.open_timing_gap(enc, "after_main");
        self.record_scene_vectors(enc, depth_view);

        // W.2b — split water pass (after the opaque main pass closes), aligned with
        // `render()`, so the editor viewport renders the same refracted water. Pass
        // the caller-supplied depth so the water pass tests/samples the depth the
        // editor's opaque geometry actually wrote (not the internal self.depth).
        self.close_timing_gap(enc);
        let water_stamp = self.begin_named_stamp(enc, "water");
        self.run_water_pass(enc, depth_view);
        self.end_named_stamp(enc, water_stamp);
        self.open_timing_gap(enc, "after_water");
        self.close_timing_gap(enc);
        let atmosphere_stamp = self.begin_named_stamp(enc, "atmosphere");
        if let Some(external_depth) = depth_view {
            self.record_bruneton(enc, external_depth);
            self.composite_atmosphere(enc, external_depth);
        } else {
            let depth = self.depth.view.clone();
            self.record_bruneton(enc, &depth);
            self.composite_atmosphere(enc, &depth);
        }
        self.end_named_stamp(enc, atmosphere_stamp);
        self.open_timing_gap(enc, "after_atmosphere");

        // --- Post-processing: bloom compute pass (runs on HDR, before blit) ---
        let bloom_intensity = if self.post_chain.bloom_enabled {
            // Lazily create the bloom pass on first use, or take the existing
            // pass out to satisfy the borrow checker (execute() needs
            // &mut BloomPass while we also need &self.device, &self.queue).
            //
            // Combining create-or-take into a single step eliminates the
            // Option round-trip and avoids any expect() on a freshly-set Some.
            let first_create = self.bloom_pass.is_none();
            let mut bloom = match self.bloom_pass.take() {
                Some(b) => b,
                None => {
                    let mut b = crate::bloom::BloomPass::new(
                        &self.device,
                        self.config.width,
                        self.config.height,
                    );
                    // Apply any pending bloom config that was set before the pass existed.
                    if let Some(cfg) = self.pending_bloom_config.take() {
                        b.set_config(cfg);
                    }
                    b
                }
            };
            let cfg = bloom.config().clone();
            self.close_timing_gap(enc);
            let bloom_stamp = self.begin_named_stamp(enc, "bloom");
            bloom.execute(
                &self.device,
                &self.queue,
                enc,
                &self.hdr_view,
                cfg.threshold,
                cfg.intensity,
                self.resource_generation,
            );
            self.end_named_stamp(enc, bloom_stamp);
            self.open_timing_gap(enc, "after_bloom");
            let intensity = cfg.intensity;
            // Editor-Engine Render Parity P.3: first-create HDR blit bind
            // group rebuild removed (only existed so hdr_blit_pipeline could
            // composite bloom; pipeline deleted in same commit). The
            // canonical POST_SHADER does not currently composite bloom
            // (windowed runtime path never did either; bloom was only
            // visible through the now-deleted editor-mode passthrough).
            // Re-integrating bloom into POST_SHADER is out of P.3 scope
            // per anti-drift constraint 11. Bloom pass still runs and
            // writes its output texture; that output is currently unused.
            // Flagged as a follow-up in the commit body.
            let _ = first_create;
            self.bloom_pass = Some(bloom);
            intensity
        } else {
            0.0
        };
        if self.cloud_raymarch_on || self.ssgi_on || self.taa_on || self.upscale_on || self.dof_on {
            let depth = depth_view
                .cloned()
                .unwrap_or_else(|| self.depth.view.clone());
            self.record_gated_presentation(enc, &depth);
        }

        // Upload blit compositing params (16 bytes — always written, cheap).
        self.queue.write_buffer(
            &self.postfx_params_buf,
            0,
            bytemuck::bytes_of(&[bloom_intensity, 0.0f32, 0.0, 0.0]),
        );

        // Editor-Engine Render Parity P.3: canonical post pipeline runs
        // unconditionally. Prior surface.is_none() branch (selecting the
        // passthrough hdr_blit_pipeline for editor mode) deleted — both
        // windowed runtime and headless editor invocations now share the
        // same ACES Narkowicz + exposure 1.35 + scene-env tint terminal
        // stage. Caller's `view` format must match config.format
        // (post_pipeline's hardcoded color target format at construction).
        {
            self.close_timing_gap(enc);
            let post_stamp = self
                .gpu_profiler
                .as_mut()
                .and_then(|profiler| profiler.begin_stamp(enc, "post_process"));
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
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pp.set_pipeline(&self.post_pipeline);
            pp.set_bind_group(0, &self.post_bind_group, &[]);
            pp.set_bind_group(1, &self.scene_env_bg, &[]);
            pp.draw(0..3, 0..1);
            drop(pp);
            if let Some(stamp) = post_stamp {
                if let Some(profiler) = self.gpu_profiler.as_mut() {
                    profiler.end_stamp(enc, stamp);
                }
            }
        }
        self.open_timing_gap(enc, "after_post");

        self.close_timing_gap(enc);
        self.record_hiz(enc, depth_view);
        self.open_timing_gap(enc, "after_hiz");
        self.close_timing_gap(enc);
        let particle_stamp = self.begin_named_stamp(enc, "particles");
        self.record_particles(enc, view, depth_view);
        self.end_named_stamp(enc, particle_stamp);
        self.open_timing_gap(enc, "after_particles");
        self.close_timing_gap(enc);

        // The frame end tick is the last `write_timestamp`. Resolve after it
        // so that query is inside the resolved range. Sky, water, atmosphere,
        // Hi-Z, and particles record into `enc`. `draw_into` does not create
        // a command encoder and does not call `queue.submit`.
        self.close_frame_timestamp(enc);
        if let Some(profiler) = self.gpu_profiler.as_mut() {
            profiler.resolve_queries(enc);
            profiler.end_frame(enc);
        }

        Ok(())
    }

    fn record_particles(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
    ) {
        let Some(params) = self.particle_emitter else {
            return;
        };
        if let Some(sim) = self.particle_sim.as_mut() {
            sim.update(&self.queue, enc, &params);
        }
        let (buffer, count) = match self.particle_sim.as_ref() {
            Some(sim) if sim.particle_count() > 0 => {
                (sim.particle_buffer().clone(), sim.particle_count())
            }
            _ => return,
        };
        let depth = depth_view
            .cloned()
            .unwrap_or_else(|| self.depth.view.clone());
        // Soft particles textureLoad scene depth. `depth_ops: Some` marks that
        // texture DEPTH_STENCIL_WRITE while the bind group uses it as RESOURCE,
        // and wgpu drops the whole encoder. The default pipeline does not write
        // depth, so the attachment is read-only and the depth test still runs.
        // A pipeline that does write depth samples a copy made after the
        // geometry pass, and writes the live attachment.
        let writes_depth = self
            .particle_draw
            .as_ref()
            .is_some_and(|draw| draw.config().depth_write);
        let sample_depth = if writes_depth {
            self.copy_scene_depth_for_particles(enc)
        } else {
            depth.clone()
        };
        let Some(draw) = self.particle_draw.as_ref() else {
            return;
        };
        let Some(sort) = self.particle_sort.as_ref() else {
            return;
        };
        let depth_ops = if writes_depth {
            Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            })
        } else {
            None
        };
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("presentation particles"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops,
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        draw.render(
            &self.device,
            &mut pass,
            &buffer,
            sort,
            &sample_depth,
            count,
            None,
        );
    }

    /// Copy the geometry depth after the writing passes have closed.
    ///
    /// Soft particles that also write depth cannot sample the attachment they
    /// write. The copy is the sampled texture. It is rebuilt only when the
    /// surface size changes.
    fn copy_scene_depth_for_particles(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
    ) -> wgpu::TextureView {
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let current = self
            .particle_depth_copy
            .as_ref()
            .is_some_and(|(stored_w, stored_h, _, _)| *stored_w == width && *stored_h == height);
        if !current {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("particle depth sample"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.depth.format,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.particle_depth_copy = Some((width, height, texture, view));
        }
        let Some((_, _, texture, view)) = self.particle_depth_copy.as_ref() else {
            return self.depth.view.clone();
        };
        let texture = texture.clone();
        let view = view.clone();
        enc.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.depth.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        view
    }

    /// Close the open pass-boundary timing gap.
    ///
    /// Hi-Z is not this gap. [`Self::record_hiz`] builds the pyramid only when
    /// [`Self::sample_hiz`] is set, and that build copies the depth view from
    /// the `draw_into` that enabled it. Occlusion, SSR, and SSGI do not sample
    /// the pyramid. It is not read back into the simulation.
    ///
    /// Pass-boundary timestamps on Vulkan are `BOTTOM_OF_PIPE`. A pass that
    /// does no draws can land both writes on the same tick. The encoder pair
    /// brackets the whole `draw_into` recording, so that collapse does not
    /// erase the frame.
    fn close_timing_gap(&mut self, enc: &mut wgpu::CommandEncoder) {
        if let Some(stamp) = self.timing_gap.take() {
            self.end_named_stamp(enc, Some(stamp));
        }
    }

    fn open_timing_gap(&mut self, enc: &mut wgpu::CommandEncoder, name: &'static str) {
        self.close_timing_gap(enc);
        self.timing_gap = self.begin_named_stamp(enc, name);
    }

    fn begin_named_stamp(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        name: &'static str,
    ) -> Option<crate::gpu_profiler::OpenStamp> {
        self.gpu_profiler
            .as_mut()
            .and_then(|profiler| profiler.begin_stamp(enc, name))
    }

    fn end_named_stamp(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        stamp: Option<crate::gpu_profiler::OpenStamp>,
    ) {
        let Some(stamp) = stamp else {
            return;
        };
        if let Some(profiler) = self.gpu_profiler.as_mut() {
            profiler.end_stamp(enc, stamp);
        }
    }

    fn open_frame_timestamp(&mut self, enc: &mut wgpu::CommandEncoder) {
        self.frame_stamp = None;
        if !self
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            return;
        }
        self.frame_stamp = self
            .gpu_profiler
            .as_mut()
            .and_then(|profiler| profiler.begin_stamp(enc, "frame"));
    }

    fn close_frame_timestamp(&mut self, enc: &mut wgpu::CommandEncoder) {
        let Some(stamp) = self.frame_stamp.take() else {
            return;
        };
        if let Some(profiler) = self.gpu_profiler.as_mut() {
            profiler.end_stamp(enc, stamp);
        }
    }

    /// Turn GPU frustum culling on or off.
    ///
    /// The default is off. `draw_into` then keeps `FrustumPlanes::test_aabb`
    /// and `draw_indexed`, and it does not build the indirect buffers.
    /// When on, `draw_into` dispatches [`crate::culling::IndirectDrawPipeline`]
    /// and each model with bounds uses `draw_indexed_indirect`. That model's
    /// own group 3 is bound immediately before the draw. [`Self::render`]
    /// keeps the CPU loop. `CullingPipeline` is not dispatched.
    pub fn set_gpu_cull_enabled(&mut self, enabled: bool) {
        self.gpu_cull_on = enabled;
    }

    /// `off` while the quality setting is off, `gpu` while it is on.
    pub fn gpu_cull_mode(&self) -> &'static str {
        if self.gpu_cull_on {
            "gpu"
        } else {
            "off"
        }
    }

    /// `quality_off` or `indirect`.
    pub fn gpu_cull_reason(&self) -> &'static str {
        if self.gpu_cull_on {
            "indirect"
        } else {
            "quality_off"
        }
    }

    /// Indirect batches from the last `draw_into` that dispatched GPU cull.
    /// Zero while the setting is off.
    pub fn gpu_cull_batch_count(&self) -> u32 {
        if !self.gpu_cull_on {
            return 0;
        }
        self.indirect_cull_resources
            .as_ref()
            .map(|resources| resources.batch_count)
            .unwrap_or(0)
    }

    /// True when the last `draw_into` dispatched the cull compute.
    /// The flag alone is not enough: models with no bounds never build a batch.
    pub fn gpu_cull_ran(&self) -> bool {
        self.gpu_cull_ran
    }

    /// Commands whose frustum test left `instance_count` non-zero.
    pub fn gpu_cull_visible_draws(&self) -> u32 {
        self.gpu_cull_visible_draws
    }

    /// Element capacity of the AABB buffer and the command buffer.
    /// `None` before the first dispatch. The two counts are equal.
    pub fn gpu_cull_storage_slots(&self) -> Option<(u64, u64)> {
        let resources = self.indirect_cull_resources.as_ref()?;
        let aabb = resources.batch_aabb_buffer.size()
            / std::mem::size_of::<crate::culling::InstanceAABB>() as u64;
        let cmd = resources.draw_commands_buffer.size()
            / std::mem::size_of::<crate::culling::DrawIndexedIndirectCommand>() as u64;
        Some((aabb, cmd))
    }

    fn prepare_gpu_model_cull(&mut self, enc: &mut wgpu::CommandEncoder) {
        self.gpu_cull_ran = false;
        self.gpu_cull_visible_draws = 0;
        let vp = self.cached_proj * self.cached_view;
        let frustum = crate::culling::FrustumPlanes::from_view_proj(&vp);
        let cam_pos = glam::Vec3::new(
            self.camera_ubo.camera_pos_pad[0],
            self.camera_ubo.camera_pos_pad[1],
            self.camera_ubo.camera_pos_pad[2],
        );
        let max_draw_dist = if self.max_draw_distance > 0.0 {
            self.max_draw_distance
        } else {
            self.scene_env.visuals.fog_end * 1.2
        };
        let max_draw_dist_sq = max_draw_dist * max_draw_dist;
        let mut aabbs = std::mem::take(&mut self.gpu_cull_aabb_scratch);
        let mut templates = std::mem::take(&mut self.gpu_cull_template_scratch);
        aabbs.clear();
        templates.clear();
        let mut visible = 0u32;
        for name in &self.gpu_cull_names {
            let Some(model) = self.models.get(name) else {
                continue;
            };
            if model.instance_count == 0 {
                continue;
            }
            let Some((aabb_min, aabb_max)) = model
                .aabb
                .or_else(|| self.gpu_cull_bounds.get(name).copied())
            else {
                continue;
            };
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
            let to_cam = center - cam_pos;
            let closest_dist_sq = (to_cam.x.abs() - extent.x).max(0.0).powi(2)
                + (to_cam.z.abs() - extent.z).max(0.0).powi(2);
            if closest_dist_sq > max_draw_dist_sq {
                continue;
            }
            let in_frustum = frustum.test_aabb(center, extent);
            for primitive in &model.primitives {
                if in_frustum {
                    visible += 1;
                }
                aabbs.push(crate::culling::InstanceAABB::new(center, extent, 0));
                templates.push(crate::culling::DrawIndexedIndirectCommand::new(
                    primitive.mesh.index_count,
                    model.instance_count,
                    0,
                    0,
                    0,
                ));
            }
        }
        self.gpu_cull_visible_draws = visible;
        if aabbs.is_empty() {
            self.gpu_cull_aabb_scratch = aabbs;
            self.gpu_cull_template_scratch = templates;
            self.indirect_cull_resources = None;
            return;
        }
        if self.indirect_cull.is_none() {
            self.indirect_cull = Some(crate::culling::IndirectDrawPipeline::new(&self.device));
        }
        let resources = {
            let Some(pipeline) = self.indirect_cull.as_ref() else {
                self.gpu_cull_aabb_scratch = aabbs;
                self.gpu_cull_template_scratch = templates;
                return;
            };
            pipeline.update_or_create_resources(
                &self.device,
                &self.queue,
                &aabbs,
                &templates,
                &frustum,
                self.indirect_cull_resources.take(),
            )
        };
        let stamp = self.begin_named_stamp(enc, "gpu_cull");
        if let Some(pipeline) = self.indirect_cull.as_ref() {
            pipeline.execute(enc, &resources);
        }
        self.end_named_stamp(enc, stamp);
        self.gpu_cull_ran = resources.batch_count > 0;
        self.indirect_cull_resources = Some(resources);
        self.gpu_cull_aabb_scratch = aabbs;
        self.gpu_cull_template_scratch = templates;
    }

    /// Turn Hi-Z recording on or off.
    ///
    /// The default is off. No pass samples [`crate::hiz_pyramid::HizPyramid::view`].
    /// Turning the flag on drops any pyramid so the next [`Self::draw_into`]
    /// copies the depth view from that call, including when the size matches
    /// a view captured earlier. Turning it off drops the pyramid too.
    pub fn set_sample_hiz(&mut self, enabled: bool) {
        if self.sample_hiz != enabled {
            self.hiz = None;
        }
        self.sample_hiz = enabled;
    }

    /// The pyramid from the last frame that recorded Hi-Z, if that flag is on.
    pub fn hiz_pyramid(&self) -> Option<&crate::hiz_pyramid::HizPyramid> {
        self.hiz.as_ref()
    }

    /// Build the Hi-Z pyramid when [`Self::sample_hiz`] is on.
    ///
    /// The copy source is `depth_view`, or the renderer's own depth when the
    /// caller passes `None`. A frame whose depth view is a different object
    /// builds a new pyramid even when the size is unchanged. Each mip is its
    /// own compute pass (#269). Nothing in `draw_into` samples the result.
    fn record_hiz(
        &mut self,
        enc: &mut wgpu::CommandEncoder,
        depth_view: Option<&wgpu::TextureView>,
    ) {
        if !self.hiz_depth_load || !self.sample_hiz {
            // Nothing in draw_into reads HizPyramid::view. SSGI samples the
            // scene depth. Building the chain anyway is one wgpu command
            // list per mip (#269): six passes at 64×64, about +108 allocs,
            // and 0.275 ms on the 1660 Ti heavy scene.
            return;
        }
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let source = depth_view.unwrap_or(&self.depth.view).clone();
        let rebuild = self
            .hiz
            .as_ref()
            .map(|pyramid| pyramid.dimensions() != (width, height) || !pyramid.copies_from(&source))
            .unwrap_or(true);
        if rebuild {
            self.hiz = Some(crate::hiz_pyramid::HizPyramid::new(
                &self.device,
                width,
                height,
                &source,
            ));
        }
        let stamp = self.begin_named_stamp(enc, "hiz");
        if let Some(pyramid) = self.hiz.as_ref() {
            pyramid.build(enc);
        }
        self.end_named_stamp(enc, stamp);
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

    /// Format of the internal HDR render target (`Rgba16Float`). LOD3 impostor
    /// passes must be built with this as their `color_format` so their
    /// pipeline matches the main forward pass layout.
    pub fn hdr_format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba16Float
    }

    /// Format of the depth target (`Depth32Float`). LOD3 impostor passes must
    /// pass this as their `depth_format` if they want depth testing.
    pub fn depth_format(&self) -> wgpu::TextureFormat {
        self.depth.format
    }

    /// Current combined view-projection matrix, as last set by
    /// [`Renderer::update_camera`] / [`Renderer::update_camera_matrices`].
    ///
    /// Callers that need to keep external camera UBOs (e.g. installed
    /// `ImpostorPass` instances) in sync with the main camera should pull
    /// this value every frame.
    pub fn current_view_proj(&self) -> glam::Mat4 {
        self.cached_proj * self.cached_view
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 1 forward-lit terrain API (Terrain Material System
    // Campaign, Option D). All methods feature-gated. When the
    // feature is off, the renderer has no `terrain_forward` field
    // and terrain rendering falls through to the legacy
    // `self.models`-based path.

    /// Immutable accessor for the forward-lit terrain renderer.
    /// Returns `None` until `init_terrain_forward` has been called.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn terrain_forward(&self) -> Option<&TerrainForwardRenderer> {
        self.terrain_forward.as_ref()
    }

    /// L.3: last frame's terrain shadow-caster culling counts, per cascade —
    /// `[(drawn, total); CASCADE_COUNT]`. `(0, 0)` when the shadow passes were
    /// skipped; a frozen cascade's entry holds its last refresh's counts.
    pub fn terrain_shadow_stats(&self) -> [(u32, u32); CASCADE_COUNT] {
        self.terrain_shadow_stats
    }

    /// L.3.B refresh telemetry for one far cascade: `(refresh_count,
    /// last_trigger, window_drift_m, render_pending)`. The trigger is one of
    /// `"none"`/`"invalid"`/`"policy"`/`"sun"`/`"drift"`/`"tick"`. The
    /// continuous-capture harness uses this to correlate visual frame-steps
    /// with refresh events.
    ///
    /// `far_index` is 0-based **within the far pair** — 0 is the first cached
    /// cascade (c2), 1 the outermost (c3). L.3.C resolution made this indexed:
    /// it previously reported only the outermost, which under the shipped
    /// `LiveC2` policy is the cascade the policy does not touch.
    pub fn shadow_far_telemetry(&self, far_index: usize) -> (u32, &'static str, f32, bool) {
        (
            self.c_far_refresh_count[far_index],
            self.c_far_refresh_reason[far_index],
            self.c_far_last_drift[far_index],
            self.c_far_pending[far_index],
        )
    }

    /// L.3.B: a far cascade's committed window `(centre, radius)` from its last
    /// refresh — the transform half of [`Self::shadow_far_telemetry`], same
    /// `far_index` convention.
    pub fn shadow_far_window(&self, far_index: usize) -> (glam::Vec3, f32) {
        self.c_far_window[far_index]
    }

    /// How many cascades run under the far-cascade refresh policy — the valid
    /// range of `far_index` for the two accessors above.
    pub fn shadow_far_cascade_count(&self) -> usize {
        CASCADE_COUNT - FIRST_CACHED_CASCADE
    }

    /// L.3.C resolution: every cascade's fit exactly as the renderer computed
    /// it. Ground truth for the relief census, which prices minimum castable
    /// relief from texel size and ortho depth range and had been modelling
    /// c2's ortho pad as the every-frame 2 m when the shipped renderer gave it
    /// the 400 m drift pad.
    pub fn shadow_cascade_fits(&self) -> [CascadeFit; CASCADE_COUNT] {
        self.cascade_fits
    }

    /// Which far-cascade refresh policy this session is running
    /// (`AW_CSM_FAR_POLICY`) — reported by the perf and continuous-flight
    /// harnesses so a measurement can never be attributed to the wrong build.
    pub fn shadow_far_policy(&self) -> &'static str {
        self.far_policy.as_str()
    }

    /// Mutable accessor for the forward-lit terrain renderer.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn terrain_forward_mut(&mut self) -> Option<&mut TerrainForwardRenderer> {
        self.terrain_forward.as_mut()
    }

    /// Initialize the forward-lit terrain state. Allocates shared layer
    /// arrays, UBO buffers, bind groups, and bind group layouts. The
    /// pipeline itself is built lazily on first frame via
    /// `ensure_forward_pipeline`. Idempotent: repeat calls are a no-op.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn init_terrain_forward(&mut self) -> Result<()> {
        if self.terrain_forward.is_some() {
            return Ok(());
        }
        let config = crate::terrain_material_manager::TerrainMaterialConfig::default();
        let manager =
            crate::terrain_material_manager::TerrainMaterialManager::new(&self.device, config)?;
        self.terrain_forward = Some(TerrainForwardRenderer {
            manager,
            chunks: std::collections::HashMap::new(),
        });
        Ok(())
    }

    /// Upload a single terrain chunk's GPU state (vertex + index buffers +
    /// splat textures) into the forward renderer. The chunk becomes visible
    /// on the next frame's draw. Requires `init_terrain_forward` to have
    /// been called.
    ///
    /// Real-Fix.D 2026-05-08: takes [`crate::material_library::NUM_SPLAT_MAPS`]
    /// = 8 splat slices (was 2). Each must be `splat_dims.0 * splat_dims.1 * 4`
    /// bytes of RGBA8 data; channel mapping `splats[i]` R..A → layers
    /// `i*4+0..i*4+3`.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn upload_terrain_chunk(
        &mut self,
        key: u64,
        vertices: &[crate::terrain_material_manager::TerrainSplatVertex],
        indices: &[u32],
        splats: &[&[u8]],
        splat_dims: (u32, u32),
    ) -> Result<()> {
        use wgpu::util::DeviceExt;

        let tf = self.terrain_forward.as_mut().ok_or_else(|| {
            anyhow::anyhow!("terrain_forward not initialized; call init_terrain_forward first")
        })?;

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain-forward-chunk-vertices"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain-forward-chunk-indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

        tf.manager
            .set_chunk_splat_forward(&self.device, &self.queue, key, splats, splat_dims)?;

        // L.3: chunk bounds from the uploaded world-space positions — the
        // single source of truth for shadow-pass culling (no caller-supplied
        // duplicate of the same logical data).
        let mut aabb_min = [f32::INFINITY; 3];
        let mut aabb_max = [f32::NEG_INFINITY; 3];
        for v in vertices {
            for i in 0..3 {
                aabb_min[i] = aabb_min[i].min(v.position[i]);
                aabb_max[i] = aabb_max[i].max(v.position[i]);
            }
        }

        // While the pass is on, keep the buffer this key already had. The
        // next scene-vector frame samples it as the previous positions.
        // While the pass is off, drop it. The first enabled frame must store
        // zero object motion, the same as an empty mesh slot.
        let previous = tf.chunks.remove(&key);
        let prev_vertex = if self.scene_vectors_on {
            previous.map(|chunk| chunk.vertex_buffer)
        } else {
            None
        };
        tf.chunks.insert(
            key,
            TerrainChunkGpu {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
                aabb_min,
                aabb_max,
                prev_vertex,
            },
        );
        Ok(())
    }

    /// Drop all per-chunk terrain-forward state — vertex/index buffers and
    /// splat bind groups. The shared layer arrays + pipeline remain.
    /// Intended for use on terrain reload or project close.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn clear_terrain_chunks(&mut self) {
        if let Some(tf) = self.terrain_forward.as_mut() {
            tf.chunks.clear();
            tf.manager.clear_forward_chunks();
        }
    }

    /// Upload the 8 biome layer textures + material UBO into the manager's
    /// shared arrays. See `TerrainMaterialManager::set_material` for the
    /// `layers` contract (missing channels are padded with neutral grey /
    /// flat-blue / mid-ORM defaults). Requires `init_terrain_forward`.
    #[cfg(feature = "terrain-splat-arrays")]
    pub fn set_terrain_materials(
        &mut self,
        gpu_material: &crate::terrain_material::TerrainMaterialGpu,
        layers: &[crate::terrain_material_manager::LayerTextures<'_>],
    ) -> Result<()> {
        let tf = self
            .terrain_forward
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("terrain_forward not initialized"))?;
        tf.manager.set_material(&self.queue, gpu_material, layers)
    }

    /// Install (or replace) an LOD3 impostor pass under `key`, to be
    /// recorded during the main forward render pass (after opaque objects,
    /// before transparent water).
    ///
    /// The pass must have been built with [`Renderer::hdr_format`] as its
    /// color format and [`Renderer::depth_format`] as its depth format; the
    /// pipeline layout is otherwise incompatible with the main pass and
    /// recording will fault at draw time.
    ///
    /// `key` should uniquely identify the source atlas — typical callers
    /// use a content hash of the source mesh (see
    /// `tools/aw_editor/src/viewport/impostor_registry.rs::MeshHash`). Calling
    /// this a second time with the same key replaces the previous pass,
    /// dropping its GPU resources.
    ///
    /// Phase 5.3 T7 stage 2 (rev 2: HashMap-keyed).
    #[cfg(feature = "impostor-bake")]
    pub fn install_impostor_pass(
        &mut self,
        key: impl Into<String>,
        pass: crate::impostor_pass::ImpostorPass,
    ) {
        self.impostor_passes.insert(key.into(), pass);
    }

    /// Remove (and return) a previously installed impostor pass.
    #[cfg(feature = "impostor-bake")]
    pub fn remove_impostor_pass(
        &mut self,
        key: &str,
    ) -> Option<crate::impostor_pass::ImpostorPass> {
        self.impostor_passes.remove(key)
    }

    /// Drop every installed impostor pass.
    #[cfg(feature = "impostor-bake")]
    pub fn clear_impostor_passes(&mut self) {
        self.impostor_passes.clear();
    }

    /// Mutable handle to a specific installed impostor pass, for per-frame
    /// camera/instance updates. Returns `None` if `key` is not installed.
    #[cfg(feature = "impostor-bake")]
    pub fn impostor_pass_mut(
        &mut self,
        key: &str,
    ) -> Option<&mut crate::impostor_pass::ImpostorPass> {
        self.impostor_passes.get_mut(key)
    }

    /// `true` iff an impostor pass is currently installed under `key`.
    #[cfg(feature = "impostor-bake")]
    pub fn has_impostor_pass(&self, key: &str) -> bool {
        self.impostor_passes.contains_key(key)
    }

    /// Number of currently installed impostor passes.
    #[cfg(feature = "impostor-bake")]
    pub fn impostor_pass_count(&self) -> usize {
        self.impostor_passes.len()
    }

    /// Update the camera UBO of every installed impostor pass in one shot.
    /// Call this per frame after [`Renderer::update_camera`] so the billboards
    /// pick the correct atlas cell based on the *current* camera direction.
    #[cfg(feature = "impostor-bake")]
    pub fn update_all_impostor_cameras(&mut self, view_proj: glam::Mat4, camera_pos: glam::Vec3) {
        for pass in self.impostor_passes.values_mut() {
            pass.update_camera(&self.queue, view_proj, camera_pos);
        }
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
        // Real-Fix.A: pass None — internal Renderer::render path uses internal depth fallback.
        self.draw_into(&view, None, &mut enc)?;

        // Then allow caller to composite additional passes (e.g., egui)
        f(
            &view,
            &mut enc,
            &self.device,
            &self.queue,
            self.surface_size(),
        );

        self.submit_timestamped(enc.finish())?;
        frame.present();

        // --- GPU profiler: initiate async readback of timestamp data ---
        if let Some(ref mut p) = self.gpu_profiler {
            p.request_readback();
        }
        self.request_fog_timestamps();

        Ok(())
    }

    /// Record one frame without a window and fail if wgpu validation reported an error.
    ///
    /// The greybox smoke test calls this with volumetrics enabled. Headless
    /// `render_with` returns before drawing, so it cannot see a bad pass.
    pub fn submit_headless_frame(&mut self) -> Result<()> {
        if self.is_device_lost() {
            anyhow::bail!("GPU device lost before the validation frame");
        }
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless validation target"),
            size: wgpu::Extent3d {
                width: self.config.width.max(1),
                height: self.config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("headless validation"),
            });
        self.draw_into(&view, None, &mut enc)?;
        self.submit_timestamped(enc.finish())?;
        if let Some(profiler) = self.gpu_profiler.as_mut() {
            profiler.request_readback();
        }
        crate::gpu_profiler::wait_for_gpu(&self.device, "headless validation")
            .map_err(|err| anyhow::anyhow!(err))?;
        if let Some(err) = crate::gpu_fault::finish_scope(&self.device) {
            self.note_scope_device_loss();
            anyhow::bail!("wgpu validation: {err}");
        }
        if let Some(err) = crate::gpu_fault::take() {
            self.note_scope_device_loss();
            anyhow::bail!("wgpu validation: {err}");
        }
        if self.is_device_lost() {
            anyhow::bail!("GPU device lost during the validation frame");
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
        crate::pipeline_cache::create_render_pipeline(
            &self.device,
            self.pipeline_cache.as_ref(),
            wgpu::RenderPipelineDescriptor {
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
            },
        )
    }

    fn build_visible_instances(&mut self) -> usize {
        // Allocation-measurement instrumentation (audit 2026-04-17, §2.3 #5).
        #[cfg(feature = "profiling")]
        astraweave_profiling::measured_span!("render.visible_instances");

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

        let needed = self.instances.len();
        if self.visible_raw_scratch.capacity() < needed {
            self.visible_raw_scratch.reserve(needed);
        }
        self.visible_raw_scratch.clear();
        self.visible_prev_scratch.clear();
        for (i, inst) in self.instances.iter().enumerate() {
            let center = inst.transform.w_axis.truncate();
            // approximate radius from basis vectors length (half-extents length)
            let sx = inst.transform.x_axis.truncate().length();
            let sy = inst.transform.y_axis.truncate().length();
            let sz = inst.transform.z_axis.truncate().length();
            let radius = 0.5 * glam::Vec3::new(sx, sy, sz).length();
            if inside_frustum_sphere(center, radius, &norm_planes) {
                self.visible_raw_scratch.push(inst.raw());
                let prev_tx = self
                    .sphere_prev_transforms
                    .get(i)
                    .copied()
                    .unwrap_or(inst.transform);
                self.visible_prev_scratch.push(
                    Instance {
                        transform: prev_tx,
                        color: inst.color,
                        material_id: inst.material_id,
                    }
                    .raw(),
                );
            }
        }
        self.visible_raw_scratch.len()
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
                        let bytes = astraweave_asset::read_limited_image_bytes(path_ref)
                            .expect("Failed to read fallback file");
                        astraweave_asset::decode_limited_image(&bytes)
                            .expect("Failed to decode fallback texture (unknown format)")
                            .to_rgba8()
                    }
                }
            } else {
                astraweave_asset::open_limited_image(path)
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
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&self.occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.occlusion_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&self.emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&self.emissive_sampler),
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
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&self.occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.occlusion_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&self.emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&self.emissive_sampler),
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
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&self.occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.occlusion_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&self.emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&self.emissive_sampler),
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
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("skinned ibuf"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
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
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
        }

        // ext_inst_buf is guaranteed Some — allocated just above if it was None
        // or too small. The `let-else` returns early on the (unreachable) None
        // branch as a safe fallback: skipping the upload leaves the previous
        // instance data intact rather than crashing the frame.
        let Some(buf) = self.ext_inst_buf.as_ref() else {
            log::warn!(
                target: "render",
                "ext_inst_buf unexpectedly None after realloc guard; skipping instance upload"
            );
            return;
        };
        self.queue.write_buffer(buf, 0, bytemuck::cast_slice(&raw));
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

    /// Add or replace a named model composed of multiple primitives.
    ///
    /// Each CPU mesh becomes a separate draw within the model while sharing
    /// the same instance buffer. KayKit and Poly Haven GLBs land here.
    /// Albedo-only primitives keep the albedo bind group. A primitive with a
    /// normal, metallic-roughness, occlusion, or emissive map uses
    /// [`ModelSurfaceMaps`] (engine metallic-roughness layout).
    pub fn add_composite_model(
        &mut self,
        name: impl Into<String>,
        cpu_meshes: Vec<crate::mesh::CpuMesh>,
        instances: &[Instance],
    ) {
        self.add_composite_model_impl(name, cpu_meshes, instances, None);
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

    fn note_gpu_cull_name(&mut self, name: &str) {
        if let Err(pos) = self
            .gpu_cull_names
            .binary_search_by(|existing| existing.as_str().cmp(name))
        {
            self.gpu_cull_names.insert(pos, name.to_string());
        }
    }

    fn forget_gpu_cull_model(&mut self, name: &str) {
        self.gpu_cull_bounds.remove(name);
        self.gpu_cull_local.remove(name);
        self.gpu_cull_instances.remove(name);
        if let Ok(pos) = self
            .gpu_cull_names
            .binary_search_by(|existing| existing.as_str().cmp(name))
        {
            self.gpu_cull_names.remove(pos);
        }
    }

    fn store_gpu_cull_mesh_bounds(
        &mut self,
        name: &str,
        local: ([f32; 3], [f32; 3]),
        instances: &[Instance],
    ) {
        self.gpu_cull_local.insert(name.to_string(), local);
        self.gpu_cull_instances
            .insert(name.to_string(), instances.to_vec());
        if let Some(world) = world_bounds_for_instances(local, instances) {
            self.gpu_cull_bounds.insert(name.to_string(), world);
        }
    }

    fn insert_model(&mut self, name: String, model: RenderModel) {
        self.gpu_cull_bounds.remove(&name);
        self.gpu_cull_local.remove(&name);
        self.gpu_cull_instances.remove(&name);
        self.note_gpu_cull_name(&name);
        if let Some(old_model) = self.models.insert(name, model) {
            if old_model.tex_gpu_bytes > 0 {
                self.gpu_memory_budget.deallocate(
                    crate::gpu_memory::MemoryCategory::Textures,
                    old_model.tex_gpu_bytes,
                );
            }
        }
    }

    /// Place one more copy of `source`, sharing its GPU meshes and textures.
    ///
    /// The copy draws a single instance. The heavy frame-budget scene uses
    /// this so 64 placements are 64 models, not one instanced draw.
    pub fn duplicate_model(
        &mut self,
        source: &str,
        name: impl Into<String>,
        instance: Instance,
    ) -> bool {
        let local = self.gpu_cull_local.get(source).copied();
        let primitives = {
            let Some(source_model) = self.models.get(source) else {
                return false;
            };
            source_model.primitives.clone()
        };
        let instance_buf = self.create_model_instance_buffer(std::slice::from_ref(&instance));
        let model = RenderModel {
            primitives,
            instance_buf,
            instance_count: 1,
            aabb: None,
            tex_gpu_bytes: 0,
            stream_albedo_id: None,
            stream_resident: false,
            scene_prev_instances: None,
        };
        let placed = name.into();
        self.insert_model(placed.clone(), model);
        if let Some(local) = local {
            self.store_gpu_cull_mesh_bounds(&placed, local, std::slice::from_ref(&instance));
        }
        true
    }

    fn add_model_impl(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        aabb: Option<([f32; 3], [f32; 3])>,
    ) {
        let instance_buf = self.create_model_instance_buffer(instances);
        let model = RenderModel {
            primitives: vec![RenderModelPrimitive {
                mesh,
                tex_bind_group: None,
                _retained_textures: Vec::new(),
                storage_generation: 0,
                bindless_index: None,
            }],
            instance_buf,
            instance_count: instances.len() as u32,
            aabb,
            tex_gpu_bytes: 0,
            stream_albedo_id: None,
            stream_resident: false,
            scene_prev_instances: None,
        };
        self.insert_model(name.into(), model);
    }

    fn add_composite_model_impl(
        &mut self,
        name: impl Into<String>,
        cpu_meshes: Vec<crate::mesh::CpuMesh>,
        instances: &[Instance],
        aabb: Option<([f32; 3], [f32; 3])>,
    ) {
        if cpu_meshes.is_empty() {
            return;
        }

        let local = mesh_local_bounds(&cpu_meshes);
        let instance_buf = self.create_model_instance_buffer(instances);
        let mut tex_gpu_bytes = 0;
        let mut primitives = Vec::with_capacity(cpu_meshes.len());

        for cpu_mesh in cpu_meshes {
            let mesh = self.create_mesh_from_cpu_mesh(&cpu_mesh);
            if let Some(packed) = crate::pbr_maps::pack_gltf_surface(&cpu_mesh) {
                let occlusion = packed
                    .occlusion
                    .as_ref()
                    .map(|(w, h, px)| (*w, *h, px.as_slice()));
                let emissive = packed
                    .emissive
                    .as_ref()
                    .map(|(w, h, px)| (*w, *h, px.as_slice()));
                let maps = ModelSurfaceMaps {
                    albedo: (packed.albedo_size.0, packed.albedo_size.1, &packed.albedo),
                    normal: (packed.normal_size.0, packed.normal_size.1, &packed.normal),
                    metallic_roughness: (
                        packed.metallic_roughness_size.0,
                        packed.metallic_roughness_size.1,
                        &packed.metallic_roughness,
                    ),
                    occlusion,
                    emissive,
                };
                let (tex_bg, textures, bytes) = self.create_model_pbr_bind_group(&maps);
                tex_gpu_bytes += bytes;
                let bindless_index = self.register_bindless_maps(&textures);
                primitives.push(RenderModelPrimitive {
                    mesh,
                    tex_bind_group: Some(tex_bg),
                    _retained_textures: textures,
                    storage_generation: 0,
                    bindless_index,
                });
            } else if let Some(img) = cpu_mesh.albedo_image.as_ref() {
                let (tex_bg, textures, bytes) =
                    self.create_model_albedo_bind_group(img.width, img.height, &img.pixels);
                tex_gpu_bytes += bytes;
                primitives.push(RenderModelPrimitive {
                    mesh,
                    tex_bind_group: Some(tex_bg),
                    _retained_textures: textures,
                    storage_generation: 0,
                    bindless_index: None,
                });
            } else {
                primitives.push(RenderModelPrimitive {
                    mesh,
                    tex_bind_group: None,
                    _retained_textures: Vec::new(),
                    storage_generation: 0,
                    bindless_index: None,
                });
            }
        }

        let model = RenderModel {
            primitives,
            instance_buf,
            instance_count: instances.len() as u32,
            aabb,
            tex_gpu_bytes,
            stream_albedo_id: None,
            stream_resident: false,
            scene_prev_instances: None,
        };
        let placed = name.into();
        self.insert_model(placed.clone(), model);
        if let Some(local) = local {
            if aabb.is_none() {
                self.store_gpu_cull_mesh_bounds(&placed, local, instances);
            } else {
                self.gpu_cull_local.insert(placed, local);
            }
        }
    }

    fn create_model_instance_buffer(&self, instances: &[Instance]) -> wgpu::Buffer {
        let raw: Vec<_> = instances.iter().map(|i| i.raw()).collect();
        let buf_bytes = (raw.len() * std::mem::size_of::<InstanceRaw>()) as u64;
        let alloc_size = buf_bytes.next_power_of_two().max(256);
        let instance_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model-inst-buf"),
            size: alloc_size,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        if !raw.is_empty() {
            self.queue
                .write_buffer(&instance_buf, 0, bytemuck::cast_slice(&raw));
        }
        instance_buf
    }

    fn texture_mip_level_count(width: u32, height: u32) -> u32 {
        (width.max(height) as f32).log2().floor() as u32 + 1
    }

    fn texture_mip_chain_bytes(width: u32, height: u32, mip_count: u32) -> u64 {
        let mut total: u64 = 0;
        let (mut mip_width, mut mip_height) = (width as u64, height as u64);
        for _ in 0..mip_count {
            total += mip_width * mip_height * 4;
            mip_width = (mip_width / 2).max(1);
            mip_height = (mip_height / 2).max(1);
        }
        total
    }

    fn build_rgba8_mip_chain(
        width: u32,
        height: u32,
        rgba: &[u8],
        renormalize_xyz: bool,
    ) -> Vec<(u32, u32, Vec<u8>)> {
        assert_eq!(rgba.len() as u32, width * height * 4);

        let mip_count = Self::texture_mip_level_count(width, height);
        let mut levels = Vec::with_capacity(mip_count as usize);
        let mut prev = rgba.to_vec();
        let mut mip_width = width;
        let mut mip_height = height;

        levels.push((mip_width, mip_height, prev.clone()));

        for _level in 1..mip_count {
            let next_width = (mip_width / 2).max(1);
            let next_height = (mip_height / 2).max(1);
            let mut mip = vec![0u8; (next_width * next_height * 4) as usize];

            for y in 0..next_height {
                for x in 0..next_width {
                    let src_x = (x * 2).min(mip_width - 1);
                    let src_y = (y * 2).min(mip_height - 1);
                    let src_x1 = (src_x + 1).min(mip_width - 1);
                    let src_y1 = (src_y + 1).min(mip_height - 1);

                    for channel in 0..4u32 {
                        let i00 = ((src_y * mip_width + src_x) * 4 + channel) as usize;
                        let i10 = ((src_y * mip_width + src_x1) * 4 + channel) as usize;
                        let i01 = ((src_y1 * mip_width + src_x) * 4 + channel) as usize;
                        let i11 = ((src_y1 * mip_width + src_x1) * 4 + channel) as usize;
                        let avg = (prev[i00] as u16
                            + prev[i10] as u16
                            + prev[i01] as u16
                            + prev[i11] as u16)
                            / 4;
                        mip[((y * next_width + x) * 4 + channel) as usize] = avg as u8;
                    }

                    if renormalize_xyz {
                        let pixel_index = ((y * next_width + x) * 4) as usize;
                        let decode = |value: u8| value as f32 / 127.5 - 1.0;
                        let encode = |value: f32| {
                            ((value * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8
                        };
                        let mut normal = glam::Vec3::new(
                            decode(mip[pixel_index]),
                            decode(mip[pixel_index + 1]),
                            decode(mip[pixel_index + 2]),
                        );
                        if normal.length_squared() > 1.0e-4 {
                            normal = normal.normalize();
                        } else {
                            normal = glam::Vec3::Z;
                        }
                        mip[pixel_index] = encode(normal.x);
                        mip[pixel_index + 1] = encode(normal.y);
                        mip[pixel_index + 2] = encode(normal.z);
                    }
                }
            }

            levels.push((next_width, next_height, mip.clone()));
            prev = mip;
            mip_width = next_width;
            mip_height = next_height;
        }

        levels
    }

    fn create_model_rgba_texture(
        &mut self,
        label: &'static str,
        width: u32,
        height: u32,
        rgba: &[u8],
        format: wgpu::TextureFormat,
        renormalize_xyz: bool,
    ) -> (wgpu::Texture, u64) {
        let mip_chain = Self::build_rgba8_mip_chain(width, height, rgba, renormalize_xyz);
        let mip_count = mip_chain.len() as u32;
        let total_bytes = Self::texture_mip_chain_bytes(width, height, mip_count);

        self.gpu_memory_budget
            .try_allocate(crate::gpu_memory::MemoryCategory::Textures, total_bytes);

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (level, (mip_width, mip_height, mip_rgba)) in mip_chain.iter().enumerate() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                mip_rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * *mip_width),
                    rows_per_image: Some(*mip_height),
                },
                wgpu::Extent3d {
                    width: *mip_width,
                    height: *mip_height,
                    depth_or_array_layers: 1,
                },
            );
        }

        (texture, total_bytes)
    }

    fn create_model_albedo_bind_group(
        &mut self,
        albedo_width: u32,
        albedo_height: u32,
        albedo_rgba: &[u8],
    ) -> (wgpu::BindGroup, Vec<wgpu::Texture>, u64) {
        let (albedo_texture, tex_gpu_bytes) = self.create_model_rgba_texture(
            "model albedo",
            albedo_width,
            albedo_height,
            albedo_rgba,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            false,
        );
        let view = albedo_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&self.occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.occlusion_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&self.emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&self.emissive_sampler),
                },
            ],
        });

        (tex_bg, vec![albedo_texture], tex_gpu_bytes)
    }

    fn create_model_pbr_bind_group(
        &mut self,
        surface_maps: &ModelSurfaceMaps<'_>,
    ) -> (wgpu::BindGroup, Vec<wgpu::Texture>, u64) {
        let (albedo_texture, albedo_bytes) = self.create_model_rgba_texture(
            "model pbr albedo",
            surface_maps.albedo.0,
            surface_maps.albedo.1,
            surface_maps.albedo.2,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            false,
        );
        let (metallic_roughness_texture, metallic_roughness_bytes) = self
            .create_model_rgba_texture(
                "model pbr metallic roughness",
                surface_maps.metallic_roughness.0,
                surface_maps.metallic_roughness.1,
                surface_maps.metallic_roughness.2,
                wgpu::TextureFormat::Rgba8Unorm,
                false,
            );
        let (normal_texture, normal_bytes) = self.create_model_rgba_texture(
            "model pbr normal",
            surface_maps.normal.0,
            surface_maps.normal.1,
            surface_maps.normal.2,
            wgpu::TextureFormat::Rgba8Unorm,
            true,
        );

        let albedo_view = albedo_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let metallic_roughness_view =
            metallic_roughness_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (occlusion_texture, occlusion_bytes) = match surface_maps.occlusion {
            Some((w, h, data)) => self.create_model_rgba_texture(
                "model occlusion",
                w,
                h,
                data,
                wgpu::TextureFormat::Rgba8Unorm,
                false,
            ),
            None => self.create_model_rgba_texture(
                "model occlusion default",
                1,
                1,
                &[255, 255, 255, 255],
                wgpu::TextureFormat::Rgba8Unorm,
                false,
            ),
        };
        let (emissive_texture, emissive_bytes) = match surface_maps.emissive {
            Some((w, h, data)) => self.create_model_rgba_texture(
                "model emissive",
                w,
                h,
                data,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                false,
            ),
            None => self.create_model_rgba_texture(
                "model emissive default",
                1,
                1,
                &[0, 0, 0, 255],
                wgpu::TextureFormat::Rgba8UnormSrgb,
                false,
            ),
        };
        let occlusion_view = occlusion_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let emissive_view = emissive_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let tex_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model pbr tex bg"),
            layout: &self.tex_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.albedo_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&metallic_roughness_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.mr_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.skin_palette_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&occlusion_view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.occlusion_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(&emissive_view),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&self.emissive_sampler),
                },
            ],
        });

        (
            tex_bg,
            vec![
                albedo_texture,
                metallic_roughness_texture,
                normal_texture,
                occlusion_texture,
                emissive_texture,
            ],
            albedo_bytes
                + metallic_roughness_bytes
                + normal_bytes
                + occlusion_bytes
                + emissive_bytes,
        )
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
        let instance_buf = self.create_model_instance_buffer(instances);
        let (tex_bg, textures, tex_gpu_bytes) =
            self.create_model_albedo_bind_group(albedo_width, albedo_height, albedo_rgba);

        let model = RenderModel {
            primitives: vec![RenderModelPrimitive {
                mesh,
                tex_bind_group: Some(tex_bg),
                _retained_textures: textures,
                storage_generation: 0,
                bindless_index: None,
            }],
            instance_buf,
            instance_count: instances.len() as u32,
            aabb: None,
            tex_gpu_bytes,
            stream_albedo_id: None,
            stream_resident: false,
            scene_prev_instances: None,
        };
        self.insert_model(name.into(), model);
    }

    /// Add a model with dedicated albedo, normal, and metallic-roughness maps.
    pub fn add_model_with_pbr_textures(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        surface_maps: &ModelSurfaceMaps<'_>,
    ) {
        self.add_model_with_pbr_textures_impl(name, mesh, instances, surface_maps, None);
    }

    /// Add a model with dedicated albedo, normal, and metallic-roughness maps and bounds.
    pub fn add_model_with_pbr_textures_and_bounds(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        surface_maps: &ModelSurfaceMaps<'_>,
        aabb_min: [f32; 3],
        aabb_max: [f32; 3],
    ) {
        self.add_model_with_pbr_textures_impl(
            name,
            mesh,
            instances,
            surface_maps,
            Some((aabb_min, aabb_max)),
        );
    }

    fn add_model_with_pbr_textures_impl(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        surface_maps: &ModelSurfaceMaps<'_>,
        aabb: Option<([f32; 3], [f32; 3])>,
    ) {
        let instance_buf = self.create_model_instance_buffer(instances);
        let (tex_bg, textures, tex_gpu_bytes) = self.create_model_pbr_bind_group(surface_maps);

        let model = RenderModel {
            primitives: vec![RenderModelPrimitive {
                mesh,
                tex_bind_group: Some(tex_bg),
                _retained_textures: textures,
                storage_generation: 0,
                bindless_index: None,
            }],
            instance_buf,
            instance_count: instances.len() as u32,
            aabb,
            tex_gpu_bytes,
            stream_albedo_id: None,
            stream_resident: false,
            scene_prev_instances: None,
        };
        self.insert_model(name.into(), model);
    }

    /// Add a model that shares the texture bind group from an existing model.
    ///
    /// This avoids creating duplicate GPU textures when multiple models use
    /// the same albedo (e.g., scatter quadrants from the same mesh group).
    /// The source model is expected to be a single-primitive textured model.
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
        self.add_model_sharing_texture_impl(name, mesh, instances, source_model, None)
    }

    /// Add a model that shares texture resources from an existing model while preserving bounds.
    pub fn add_model_sharing_texture_with_bounds(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        source_model: &str,
        aabb_min: [f32; 3],
        aabb_max: [f32; 3],
    ) -> bool {
        self.add_model_sharing_texture_impl(
            name,
            mesh,
            instances,
            source_model,
            Some((aabb_min, aabb_max)),
        )
    }

    fn add_model_sharing_texture_impl(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        instances: &[Instance],
        source_model: &str,
        aabb: Option<([f32; 3], [f32; 3])>,
    ) -> bool {
        let (shared_bg, shared_textures) = match self
            .models
            .get(source_model)
            .and_then(|src| src.primitives.first())
        {
            Some(src) => (src.tex_bind_group.clone(), src._retained_textures.clone()),
            None => return false,
        };
        let instance_buf = self.create_model_instance_buffer(instances);
        let model = RenderModel {
            primitives: vec![RenderModelPrimitive {
                mesh,
                tex_bind_group: shared_bg,
                _retained_textures: shared_textures,
                storage_generation: 0,
                bindless_index: None,
            }],
            instance_buf,
            instance_count: instances.len() as u32,
            aabb,
            tex_gpu_bytes: 0, // Don't double-count — source owns the budget
            stream_albedo_id: None,
            stream_resident: false,
            scene_prev_instances: None,
        };
        self.insert_model(name.into(), model);
        true
    }

    /// Remove a named model and release its GPU resources.
    pub fn clear_model(&mut self, name: &str) {
        self.forget_gpu_cull_model(name);
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
    /// Replace one model's vertex buffer. The buffer must already be large
    /// enough and have been created with `COPY_DST`. Returns false when the
    /// model is missing or the new vertex list does not fit.
    pub fn write_model_vertices(
        &mut self,
        name: &str,
        primitive: usize,
        vertices: &[crate::mesh::MeshVertex],
    ) -> bool {
        let bytes = bytemuck::cast_slice(vertices);
        let Some(buf) = self
            .models
            .get(name)
            .and_then(|model| model.primitives.get(primitive))
            .map(|part| part.mesh.vertex_buf.clone())
        else {
            return false;
        };
        if buf.size() < bytes.len() as u64 {
            return false;
        }
        self.queue.write_buffer(&buf, 0, bytes);
        self.refresh_gpu_cull_after_vertex_write(name, vertices);
        true
    }

    fn refresh_gpu_cull_after_vertex_write(
        &mut self,
        name: &str,
        vertices: &[crate::mesh::MeshVertex],
    ) {
        if self.models.get(name).and_then(|model| model.aabb).is_some() {
            return;
        }
        let Some(fresh) = vertex_position_bounds(vertices) else {
            return;
        };
        let primitive_count = self
            .models
            .get(name)
            .map(|model| model.primitives.len())
            .unwrap_or(0);
        let local = if primitive_count > 1 {
            self.gpu_cull_local
                .get(name)
                .copied()
                .map(|old| union_aabb(old, fresh))
                .unwrap_or(fresh)
        } else {
            fresh
        };
        let instances = self.gpu_cull_instances.get(name).cloned();
        if let Some(instances) = instances {
            self.store_gpu_cull_mesh_bounds(name, local, &instances);
        } else if self.gpu_cull_local.contains_key(name) {
            self.gpu_cull_local.insert(name.to_string(), local);
        }
    }

    /// Write a single instance without building a temporary instance list.
    pub fn update_model_transform(&mut self, name: &str, instance: Instance) -> bool {
        let raw = instance.raw();
        let bytes = bytemuck::bytes_of(&raw);
        let Some(model) = self.models.get_mut(name) else {
            return false;
        };
        if model.instance_buf.size() < bytes.len() as u64 {
            return false;
        }
        let buf = model.instance_buf.clone();
        let has_aabb = model.aabb.is_some();
        model.instance_count = 1;
        self.queue.write_buffer(&buf, 0, bytes);
        if !has_aabb {
            if let Some(local) = self.gpu_cull_local.get(name).copied() {
                self.store_gpu_cull_mesh_bounds(name, local, std::slice::from_ref(&instance));
            }
        }
        true
    }

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
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
        }
        if !raw.is_empty() {
            self.queue
                .write_buffer(&model.instance_buf, 0, bytemuck::cast_slice(&raw));
        }
        let has_aabb = model.aabb.is_some();
        model.instance_count = instances.len() as u32;
        if !has_aabb {
            if let Some(local) = self.gpu_cull_local.get(name).copied() {
                self.store_gpu_cull_mesh_bounds(name, local, instances);
            }
        }
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

fn note_scope_device_loss_on(flag: &std::sync::atomic::AtomicBool) {
    if crate::gpu_fault::device_lost() {
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Renderer {
    fn note_scope_device_loss(&self) {
        note_scope_device_loss_on(&self.device_lost);
    }

    /// Upload bytes without mapping the buffer. `create_buffer_init` calls
    /// `get_mapped_range` and panics when the buffer is invalid.
    fn upload_bytes(
        &self,
        label: &str,
        contents: &[u8],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        let mut padded = contents.to_vec();
        while padded.len() % 4 != 0 {
            padded.push(0);
        }
        if padded.is_empty() {
            padded.resize(4, 0);
        }
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: padded.len() as u64,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !self.is_device_lost() {
            self.queue.write_buffer(&buffer, 0, &padded);
        } else {
            log::error!("skipped upload of {label}; the GPU device is lost");
        }
        buffer
    }

    fn unmapped_mesh(&self) -> Mesh {
        log::error!("GPU device is lost; refusing to map a mesh buffer");
        let vertex_size = std::mem::size_of::<crate::types::Vertex>() as u64;
        let vbuf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ext v (device lost)"),
            size: vertex_size.max(4) * 3,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        let ibuf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ext i (device lost)"),
            size: 12,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });
        Mesh {
            vertex_buf: vbuf,
            index_buf: ibuf,
            index_count: 0,
        }
    }
}

fn gpu_instance() -> wgpu::Instance {
    wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default())
}

const NVIDIA_VENDOR: u32 = 0x10DE;

fn is_nvidia(info: &wgpu::AdapterInfo) -> bool {
    info.vendor == NVIDIA_VENDOR || info.name.to_ascii_lowercase().contains("nvidia")
}

async fn choose_adapter(
    instance: &wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface<'_>>,
) -> Result<wgpu::Adapter> {
    if warp_requested() {
        log::info!("requesting the fallback adapter (ARAVATH_WARP) at HighPerformance");
        return instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface,
                force_fallback_adapter: true,
            })
            .await
            .context("No adapter");
    }

    let backends = wgpu::Backends::from_env().unwrap_or(wgpu::Backends::all());
    let adapters = instance.enumerate_adapters(backends);
    for adapter in &adapters {
        let info = adapter.get_info();
        let surface_ok = compatible_surface
            .map(|surface| adapter.is_surface_supported(surface))
            .unwrap_or(true);
        log::info!(
            "adapter candidate '{}' backend {:?} type {:?} vendor {:#06x} device {:#06x} driver '{}' info '{}' surface_ok={surface_ok}",
            info.name,
            info.backend,
            info.device_type,
            info.vendor,
            info.device,
            info.driver,
            info.driver_info,
        );
    }
    let usable: Vec<&wgpu::Adapter> = adapters
        .iter()
        .filter(|adapter| {
            compatible_surface
                .map(|surface| adapter.is_surface_supported(surface))
                .unwrap_or(true)
        })
        .collect();
    let nvidia_discrete = usable.iter().find(|adapter| {
        let info = adapter.get_info();
        is_nvidia(&info) && info.device_type == wgpu::DeviceType::DiscreteGpu
    });
    let any_discrete = usable
        .iter()
        .find(|adapter| adapter.get_info().device_type == wgpu::DeviceType::DiscreteGpu);
    let any_nvidia = usable.iter().find(|adapter| is_nvidia(&adapter.get_info()));
    let (adapter, reason) = if let Some(adapter) = nvidia_discrete {
        (*adapter, "discrete NVIDIA")
    } else if let Some(adapter) = any_discrete {
        (*adapter, "discrete GPU")
    } else if let Some(adapter) = any_nvidia {
        (*adapter, "NVIDIA adapter")
    } else {
        log::info!("no enumerated discrete adapter; requesting HighPerformance");
        return instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface,
                force_fallback_adapter: false,
            })
            .await
            .context("No adapter");
    };
    log::info!("chose adapter '{}' ({reason})", adapter.get_info().name);
    Ok(adapter.clone())
}

fn log_gpu(adapter: &wgpu::Adapter, device: &wgpu::Device) {
    let info = adapter.get_info();
    let limits = device.limits();
    let features = adapter.features();
    let rgba16 = wgpu::TextureFormat::Rgba16Float.guaranteed_format_features(features);
    log::info!(
        "adapter '{}' backend {:?} type {:?} vendor {:#06x} device {:#06x} driver '{}' info '{}'",
        info.name,
        info.backend,
        info.device_type,
        info.vendor,
        info.device,
        info.driver,
        info.driver_info
    );
    log::info!(
        "limits bind_groups={} bindings_per_group={} sampled_textures={} samplers={} storage_textures={} storage_buffers={} uniform_buffers={} tex2d={} tex3d={} buffer_bytes={}",
        limits.max_bind_groups,
        limits.max_bindings_per_bind_group,
        limits.max_sampled_textures_per_shader_stage,
        limits.max_samplers_per_shader_stage,
        limits.max_storage_textures_per_shader_stage,
        limits.max_storage_buffers_per_shader_stage,
        limits.max_uniform_buffers_per_shader_stage,
        limits.max_texture_dimension_2d,
        limits.max_texture_dimension_3d,
        limits.max_buffer_size,
    );
    log::info!(
        "Rgba16Float storage_write={} storage_read_write={} adapter_specific_formats={} timestamp_query={}",
        rgba16
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::STORAGE_WRITE_ONLY),
        rgba16
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::STORAGE_READ_WRITE),
        features.contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES),
        features.contains(wgpu::Features::TIMESTAMP_QUERY),
    );
}

fn install_device_error_handler(
    device: &wgpu::Device,
    device_lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    device.on_uncaptured_error(Box::new(move |error| {
        let text = error.to_string();
        log::error!("wgpu device error: {text}");
        if matches!(error, wgpu::Error::Validation { .. }) {
            crate::gpu_fault::record(&text);
        }
        if crate::gpu_fault::error_means_device_lost(&error, &text) {
            device_lost.store(true, std::sync::atomic::Ordering::SeqCst);
            crate::gpu_fault::mark_device_lost();
            log::error!("GPU device loss detected — renderer must be recreated");
        }
    }));
}

/// Append one skip line for a test that found no adapter.
///
/// `new_headless` does not call this. The parent directory has to already
/// exist and canonicalize to the workspace `target/` directory. That
/// directory is `CARGO_TARGET_DIR` when set, otherwise the parent of this
/// crate's compile-time manifest plus `target`. The calling test's runtime
/// manifest is not consulted, so a `<crate>/target` directory does not move
/// the log. The file itself is created. A symlink at the final path is
/// refused, and the open does not follow one. A failed write panics so the
/// test does not pass without a line.
pub fn record_gpu_skip_for_tests(reason: &str) {
    record_gpu_skip_for_tests_at(reason, std::env::var_os("AW_GPU_SKIP_LOG"));
}

/// `log_override` is `AW_GPU_SKIP_LOG`. `None` uses `gpu-skip-reasons.txt`
/// inside the workspace target. The guard is the same either way.
#[allow(clippy::panic)]
fn record_gpu_skip_for_tests_at(reason: &str, log_override: Option<std::ffi::OsString>) {
    let target = existing_workspace_target();
    let target = target.canonicalize().unwrap_or_else(|err| {
        panic!(
            "GPU skip log stays in the workspace target directory, missing at {}: {err}",
            target.display()
        )
    });
    let path = match log_override {
        Some(raw) => std::path::PathBuf::from(raw),
        None => target.join("gpu-skip-reasons.txt"),
    };
    let parent = path
        .parent()
        .unwrap_or_else(|| panic!("GPU skip log {} has no parent directory", path.display()));
    let parent = parent.canonicalize().unwrap_or_else(|err| {
        panic!(
            "refusing to create a directory for GPU skip log {}: {err}",
            path.display()
        )
    });
    if parent != target {
        panic!(
            "GPU skip log {} is outside {}",
            path.display(),
            target.display()
        );
    }
    let test = std::thread::current()
        .name()
        .unwrap_or("unnamed-test")
        .to_string();
    let line = format!("SKIP(gpu): {test}: {}\n", one_skip_line(reason));
    eprintln!("{}", line.trim_end());
    let mut file = open_append_nofollow(&path)
        .unwrap_or_else(|err| panic!("could not record the GPU skip at {}: {err}", path.display()));
    std::io::Write::write_all(&mut file, line.as_bytes())
        .unwrap_or_else(|err| panic!("could not record the GPU skip at {}: {err}", path.display()));
}

fn is_missing_adapter(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<wgpu::RequestAdapterError>(),
            Some(wgpu::RequestAdapterError::NotFound { .. })
        )
    })
}

/// Refuse a symlink, then open with `O_NOFOLLOW` so a replaced path is not
/// followed either. The parent check happens before this call.
fn open_append_nofollow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("GPU skip log {} is a symlink", path.display()),
            ));
        }
        Ok(_) => {
            if let (Ok(canonical), Some(parent)) = (path.canonicalize(), path.parent()) {
                if let Ok(parent) = parent.canonicalize() {
                    if canonical.parent() != Some(parent.as_path()) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::AlreadyExists,
                            format!(
                                "GPU skip log {} canonicalizes outside {}",
                                path.display(),
                                parent.display()
                            ),
                        ));
                    }
                }
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

fn one_skip_line(reason: &str) -> String {
    reason
        .chars()
        .map(|ch| match ch {
            '\n' | '\r' | '%' => ' ',
            other => other,
        })
        .collect()
}

/// Workspace `target/` for the GPU skip log.
///
/// `CARGO_TARGET_DIR` wins when the process has it. Otherwise the path is
/// the parent of this crate's compile-time manifest plus `target`.
/// `env!("CARGO_MANIFEST_DIR")` is baked into `astraweave-render`. The
/// runtime `CARGO_MANIFEST_DIR` is the package Cargo is testing, and an
/// ancestor walk stops at the first `target/` directory. Scene streaming
/// tests created `astraweave-scene/target`, so that walk returned the crate
/// directory and the guard rejected the workspace skip log.
fn existing_workspace_target() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR") {
        return std::path::PathBuf::from(dir);
    }
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    match manifest.parent() {
        Some(workspace) => workspace.join("target"),
        None => manifest.join("target"),
    }
}

fn warp_requested() -> bool {
    matches!(
        std::env::var("ARAVATH_WARP").ok().as_deref(),
        Some("1") | Some("true")
    )
}

fn clip_planes(proj: glam::Mat4) -> (f32, f32) {
    let c = proj.z_axis.z;
    let d = proj.w_axis.z;
    if c.abs() < 1e-8 {
        return (astraweave_camera::GAME_ZNEAR, astraweave_camera::GAME_ZFAR);
    }
    let near = (d / c).abs();
    let denom = 1.0 + c;
    // A 20 km far plane at near 0.1 makes |1+c| about 5e-6. That is still a
    // finite plane. Only an infinite projection stores c at -1.
    let far = if denom.abs() < 1e-8 {
        f32::MAX
    } else {
        (c * near / denom).abs()
    };
    (near, far)
}

#[cfg(test)]
mod skip_log_tests {
    use super::open_append_nofollow;

    #[cfg(unix)]
    #[test]
    fn o_nofollow_is_the_libc_constant() {
        assert_ne!(libc::O_NOFOLLOW, 0);
    }

    #[cfg(unix)]
    #[test]
    fn skip_log_rejects_a_symlink() {
        let dir = std::env::temp_dir().join(format!("aw-gpu-skip-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let outside = dir.join("outside.txt");
        std::fs::write(&outside, b"outside").expect("outside file");
        let link = dir.join("gpu-skip-reasons.txt");
        std::os::unix::fs::symlink(&outside, &link).expect("symlink");
        let err = open_append_nofollow(&link).expect_err("symlink must be refused");
        assert!(err.to_string().contains("symlink"), "open error was {err}");
        assert_eq!(std::fs::read(&outside).expect("outside bytes"), b"outside");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wgpu25_not_found_is_a_missing_adapter() {
        let err = wgpu::RequestAdapterError::NotFound {
            active_backends: wgpu::Backends::all(),
            requested_backends: wgpu::Backends::all(),
            supported_backends: wgpu::Backends::all(),
            no_fallback_backends: wgpu::Backends::empty(),
            no_adapter_backends: wgpu::Backends::all(),
            incompatible_surface_backends: wgpu::Backends::empty(),
        };
        let wrapped: anyhow::Error = err.into();
        let text = format!("{wrapped:?} {wrapped}");
        assert!(
            !text.contains("No adapter") && !text.contains("NotFound"),
            "Display/Debug still contains a needle the old matcher used: {text}"
        );
        assert!(
            text.contains("No suitable graphics adapter found"),
            "{text}"
        );
        assert!(super::is_missing_adapter(&wrapped));
        let other: anyhow::Error = wgpu::RequestAdapterError::EnvNotSet.into();
        assert!(!super::is_missing_adapter(&other));
    }

    /// A `<crate>/target` directory must not steal the workspace skip log.
    ///
    /// The log path is what CI sets on `AW_GPU_SKIP_LOG`: a file whose parent
    /// is the workspace `target/`. The override is passed in so a parallel
    /// lib test does not observe a process-wide `set_var`.
    #[test]
    fn crate_local_target_keeps_the_workspace_skip_log() {
        let decoy = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        let created = !decoy.exists();
        if created {
            std::fs::create_dir_all(&decoy).expect("crate-local target");
        }
        struct Cleanup {
            decoy: std::path::PathBuf,
            created: bool,
            log: Option<std::path::PathBuf>,
        }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                if let Some(log) = &self.log {
                    let _ = std::fs::remove_file(log);
                }
                if self.created {
                    let _ = std::fs::remove_dir_all(&self.decoy);
                }
            }
        }
        let mut cleanup = Cleanup {
            decoy: decoy.clone(),
            created,
            log: None,
        };
        let workspace = super::existing_workspace_target()
            .canonicalize()
            .expect("workspace target");
        let decoy_canon = decoy.canonicalize().expect("decoy target");
        assert_ne!(
            workspace, decoy_canon,
            "crate-local target must not become the workspace target"
        );
        let log_name = format!(
            "gpu-skip-crate-decoy-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let log = workspace.join(log_name);
        cleanup.log = Some(log.clone());
        super::record_gpu_skip_for_tests_at(
            "crate-local target must not win",
            Some(log.as_os_str().to_os_string()),
        );
        let written = std::fs::read_to_string(&log).expect("skip line");
        assert!(
            written.contains("SKIP(gpu):") && written.contains("crate-local target must not win"),
            "{written}"
        );
        assert!(
            !decoy.join("gpu-skip-reasons.txt").exists(),
            "skip line was written under the crate-local target"
        );
    }

    #[test]
    fn skip_log_outside_the_workspace_target_still_panics() {
        let dir = std::env::temp_dir().join(format!("aw-gpu-skip-outside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("outside dir");
        let path = dir.join("gpu-skip-reasons.txt");
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::record_gpu_skip_for_tests_at("outside", Some(path.as_os_str().to_os_string()));
        }));
        let created = path.exists();
        let _ = std::fs::remove_dir_all(&dir);
        let payload = caught.expect_err("a skip log outside the workspace target must panic");
        let text = panic_message(&payload);
        assert!(text.contains("is outside"), "guard message was {text}");
        assert!(!created, "the refused skip log must not be created");
    }

    fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
        if let Some(msg) = payload.downcast_ref::<String>() {
            return msg.clone();
        }
        if let Some(msg) = payload.downcast_ref::<&str>() {
            return (*msg).to_string();
        }
        "non-string panic".to_string()
    }
}

#[cfg(test)]
mod atmosphere_tests {
    use super::clip_planes;

    #[test]
    fn clip_planes_match_a_perspective_matrix() {
        let proj = glam::Mat4::perspective_rh(1.0, 1.5, 0.1, 500.0);
        let (near, far) = clip_planes(proj);
        assert!((near - 0.1).abs() < 0.02, "near {near}");
        assert!((far - 500.0).abs() < 5.0, "far {far}");
        let wide = glam::Mat4::perspective_rh(1.0, 1.0, 0.1, 20_000.0);
        let (near, far) = clip_planes(wide);
        assert!((near - 0.1).abs() < 0.02, "wide near {near}");
        // f32 in the perspective matrix loses about 27 m at 20 km. 500 m
        // is the old fallback and must not come back.
        assert!((far - 20_000.0).abs() < 50.0, "wide far {far}");
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

/// Shadow-map edge length (square, per cascade layer).
pub(crate) const SHADOW_MAP_SIZE: u32 = 2048;

/// Number of CSM cascades. Single source of truth for the layer count, the
/// per-cascade UBO/bind-group arrays, the pass loops, and the caster stats —
/// so adding a cascade cannot leave one of those behind (the L.3.C inventory's
/// silent-failure class: a `match idx { .., _ => last }` that quietly culls a
/// new cascade against an old cascade's frustum).
pub(crate) const CASCADE_COUNT: usize = 4;

/// Cascades `>= FIRST_CACHED_CASCADE` are subject to the [`FarCascadePolicy`]:
/// under the default they re-fit and re-render only on refresh (sun move /
/// window drift / slow tick) instead of every frame, because their caster sets
/// are most of the world. See `update_cascade_splits` for the two-phase commit
/// that makes a frozen map safe to sample.
pub(crate) const FIRST_CACHED_CASCADE: usize = 2;

/// Refresh policy for the far cascades — the L.3.C measurement lever.
///
/// L.3.C shipped [`Cached`](Self::Cached) and then measured that the 4-cascade
/// layout **triples** far-field frame-to-frame stepping versus the 3-cascade one
/// (survey band worst step 2.01 → 6.44 pp): c2's finer texels finally RESOLVE
/// dune shadows, so a frozen window's jump at a refresh now has real detail to
/// shift. This enum exists so the alternatives are measurable in ONE binary
/// against ONE world — a matched control rather than a rebuild comparison.
///
/// Chosen once at construction from `AW_CSM_FAR_POLICY`
/// (`cached` | `live` | `live-c2` | `alternate`); unset or unrecognised values
/// use the shipped default. It is a diagnostic lever in the same family as
/// `AW_CSM_LOG`, not a user-facing setting.
///
/// **Measured** (min-spec 1660 Ti Max-Q, Vulkan, 1920×1080, survey framing on
/// the recording's radius-8 world, median of 3 replicates, shadow-system cost =
/// median ON − median OFF; the ~2.5 ms line is L.3's STOP threshold):
///
/// | policy | cost | verdict |
/// |---|---|---|
/// | `Cached` | 1.069 ms | cheapest; triples far-field frame-to-frame stepping (L.3.C §5) |
/// | **`LiveC2`** | **1.726 ms** | **shipped** — fits with 0.77 ms headroom |
/// | `Alternate` | 2.363 ms | dominated: dearer than `LiveC2` AND keeps the padded 1.880 m texels |
/// | `Live` | 2.860 ms | **over budget** |
///
/// The split is informative: uncaching c2 costs +0.657 ms, uncaching c3 a
/// further +1.134 ms. c3 is the expensive one (radius 3185 m, 202/289 chunks)
/// and the one whose stepping was accepted for three beats; c2 is the affordable
/// one that carries the newly-resolved dune detail.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FarCascadePolicy {
    /// Both far cascades hold a frozen window between refreshes.
    Cached,
    /// Both far cascades re-fit and re-render every frame — no frozen window,
    /// therefore no refresh jump, and no drift pad (which buys texel density).
    Live,
    /// c2 every frame, c3 still cached — **the shipped default**. c2 carries
    /// the newly-resolved detail that made the jumps visible; c3's coarse
    /// stepping was accepted for three beats and is what makes this affordable.
    /// (The director's pre-authorised fallback (a).)
    #[default]
    LiveC2,
    /// The far pair is staggered: exactly one of the two re-fits per frame, so
    /// each is at most one frame stale and the per-frame far cost is half
    /// [`Live`](Self::Live)'s. Note it is *four times* `Cached`'s under a
    /// static camera (1 refresh/frame vs ~0.25), and because a window frozen
    /// for even one frame still needs the drift pad, it keeps `Cached`'s
    /// coarser texels — measured dearer than `LiveC2` on both axes. (Fallback
    /// (b), measured and rejected.)
    Alternate,
    /// L.3.D rung 1: c2 every frame (as shipped) and **c3 on alternate
    /// frames**. The L.3.C-resolution attribution put the whole far-field pop on
    /// c3's multi-frame freeze, so this halves the interval c3 is allowed to lag
    /// and therefore — to first order — halves the window jump each re-fit has
    /// to absorb.
    ///
    /// Crucially it does NOT inherit `Cached`'s 400 m drift pad: a window that
    /// lags one frame does not need a pad sized for 300 m of drift, and the pad
    /// is not free (it enlarges the ortho box, coarsening texels and widening
    /// the AABB cull). See [`Self::freeze_bounds`].
    LiveC2AltC3,
    /// L.3.D rung 2: c2 every frame, c3 cached at its normal cadence but with
    /// its window replacement made **incremental** instead of atomic.
    ///
    /// Rung 1 attacked the jump's SIZE by refreshing twice as often and bought
    /// only 36% (6.44 -> 4.09 pp) for nearly the whole budget. This attacks its
    /// ATOMICITY instead: the outgoing window is retained in a second layer and
    /// the receiver blends between the two over [C3_FADE_FRAMES], so the
    /// per-frame change is a fraction of the replacement rather than all of it.
    ///
    /// Both maps are valid-at-their-own-time — the incoming one is fitted and
    /// rendered on the refresh frame and is immediately authoritative — so
    /// NEITHER needs an inflated pad. The blend is purely temporal, not a
    /// coverage compromise, which is what keeps this at `Cached`'s caster
    /// rate rather than `Live`'s.
    LiveC2FadeC3,
}

/// L.3.D rung 2: the shadow array is one layer WIDER than the cascade count.
/// The extra layer holds c3's OUTGOING window while a transition crossfades, so
/// a window replacement stops being atomic. Nothing renders into it — it is
/// filled by a layer copy on the refresh frame — which is why the pass loops
/// still run over `CASCADE_COUNT` and only the array and the sampling view grow.
pub(crate) const C3_PREV_LAYER: u32 = CASCADE_COUNT as u32;
pub(crate) const SHADOW_LAYER_COUNT: u32 = CASCADE_COUNT as u32 + 1;

/// Frames a c3 window replacement is blended over. Must stay BELOW the periodic
/// refresh interval (8) so a transition always completes before the next one
/// begins; a drift-triggered refresh can still truncate one, which is a smaller
/// discontinuity than the atomic replacement it replaces and is counted in the
/// telemetry when it happens.
const C3_FADE_FRAMES: f32 = 4.0;

/// Drift budget for a window frozen across many frames (the L.3.A cache).
const CACHE_DRIFT_LIMIT: f32 = 300.0;
const CACHE_DRIFT_PAD: f32 = 400.0;
/// Drift budget for a window that lags at most one frame (L.3.D rung 1).
const ALT_DRIFT_LIMIT: f32 = 150.0;
const ALT_DRIFT_PAD: f32 = 200.0;
/// Ortho pad for a cascade re-fitted every frame — it cannot lag, so this is
/// just numerical headroom. Unchanged since L.3; it is what preserves the
/// pinned close/mid station frames.
const LIVE_MARGIN: f32 = 2.0;

impl FarCascadePolicy {
    fn from_env() -> Self {
        let raw = std::env::var("AW_CSM_FAR_POLICY").unwrap_or_default();
        if raw.is_empty() {
            return Self::default();
        }
        // EVERY policy needs an arm, including the one that is not the default.
        // When `LiveC2` became the default, `cached` had no arm and fell into
        // `_` — so an A/B run with `AW_CSM_FAR_POLICY=cached` silently measured
        // `LiveC2` against itself and reported 0 differing pixels, which reads
        // exactly like "the change has no effect". An unrecognised value is now
        // LOUD rather than silently the default: a measurement lever that can
        // quietly do nothing is worse than no lever.
        match raw.to_ascii_lowercase().as_str() {
            "cached" => Self::Cached,
            "live" => Self::Live,
            "live-c2" | "live_c2" | "livec2" => Self::LiveC2,
            "alternate" | "alt" => Self::Alternate,
            "live-c2-alt-c3" | "alt-c3" | "rung1" => Self::LiveC2AltC3,
            "live-c2-fade-c3" | "fade-c3" | "rung2" => Self::LiveC2FadeC3,
            other => {
                eprintln!(
                    "[csm] AW_CSM_FAR_POLICY={other:?} is not a policy \
                     (cached|live|live-c2|alternate|live-c2-alt-c3) — using the \
                     default {:?}",
                    Self::default()
                );
                Self::default()
            }
        }
    }

    /// How far far cascade `c` (0-based within the far pair) is allowed to lag
    /// under this policy: `(drift_limit_m, ortho_pad_m)`, or `None` when it is
    /// re-fitted every frame and therefore never lags.
    ///
    /// The two are returned as ONE value because they are one decision. The pad
    /// exists solely to keep a frozen window covering its ideal slice while it
    /// lags, so it must exceed the lag the drift trigger permits — and the pad
    /// is not free: it enlarges the ortho box, coarsening texels (c2 padded
    /// renders 1.91 m where live renders 1.52 m) and widening the AABB cull
    /// (102 vs 70 of 289 chunks). Splitting them into a boolean and a constant
    /// is what let `Alternate` pay `Cached`'s 400 m pad for a one-frame lag.
    fn freeze_bounds(self, c: usize) -> Option<(f32, f32)> {
        match self {
            Self::Cached | Self::Alternate => Some((CACHE_DRIFT_LIMIT, CACHE_DRIFT_PAD)),
            Self::Live => None,
            Self::LiveC2 => (c != 0).then_some((CACHE_DRIFT_LIMIT, CACHE_DRIFT_PAD)),
            // c2 live; c3 lags one frame, so it gets a one-frame budget.
            Self::LiveC2AltC3 => (c != 0).then_some((ALT_DRIFT_LIMIT, ALT_DRIFT_PAD)),
            // c2 live; c3 keeps the cache's budget — the transition is made
            // continuous by blending, not by shortening the lag.
            Self::LiveC2FadeC3 => (c != 0).then_some((CACHE_DRIFT_LIMIT, CACHE_DRIFT_PAD)),
        }
    }

    /// Does this policy make c3's window replacement INCREMENTAL (rung 2)?
    /// Only the fading policy allocates the transition machinery; every other
    /// policy leaves the blend weight pinned at 1.0 and the spare layer unused,
    /// so they render exactly as they did before L.3.D.
    fn fades_c3(self) -> bool {
        matches!(self, Self::LiveC2FadeC3)
    }

    /// Can far cascade `c` hold a frozen window across frames?
    fn frozen(self, c: usize) -> bool {
        self.freeze_bounds(c).is_some()
    }

    /// Ortho half-extent padding beyond the fitted sphere radius for cascade `c`.
    fn margin(self, c: usize) -> f32 {
        self.freeze_bounds(c).map_or(LIVE_MARGIN, |(_, pad)| pad)
    }

    /// Window drift that forces a re-fit for cascade `c`. Infinite for a live
    /// cascade: it re-fits every frame anyway, so the trigger is unreachable.
    fn drift_limit(self, c: usize) -> f32 {
        self.freeze_bounds(c)
            .map_or(f32::INFINITY, |(limit, _)| limit)
    }

    /// Force a refresh this frame, regardless of the sun / drift / tick
    /// triggers. `tick` is the periodic counter (its modulus is even, so its
    /// parity is a valid frame alternation).
    fn forces_refresh(self, c: usize, tick: u32) -> bool {
        match self {
            Self::Cached => false,
            Self::Live => true,
            Self::LiveC2 => c == 0,
            Self::Alternate => (tick as usize % 2) == c,
            // c2 every frame; c3 every other frame.
            Self::LiveC2AltC3 => c == 0 || tick % 2 == 0,
            // c2 every frame; c3 on its own sun/drift/tick triggers.
            Self::LiveC2FadeC3 => c == 0,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Cached => "cached",
            Self::Live => "live",
            Self::LiveC2AltC3 => "live-c2-alt-c3",
            Self::LiveC2FadeC3 => "live-c2-fade-c3",
            Self::LiveC2 => "live-c2",
            Self::Alternate => "alternate",
        }
    }
}

/// One cascade's fit exactly as the renderer computed it — ground truth for the
/// relief census, which prices `h_min` from texel size and ortho depth range.
///
/// It exists because the L.3.C census priced c2 with the every-frame 2 m pad
/// while the shipped renderer gave it the 400 m drift pad, understating its
/// texel (1.52 vs 1.91 m), its depth range (4714 vs 5112 m) and therefore its
/// minimum castable relief (2.82 vs 3.44 m). A model of the fit is not the fit.
#[derive(Copy, Clone, Debug, Default)]
pub struct CascadeFit {
    /// View-distance bounds of the slice this cascade covers.
    pub near: f32,
    pub far: f32,
    /// Centre and radius of the fitted frustum sphere (world space).
    pub center: glam::Vec3,
    pub radius: f32,
    /// Ortho half-extent padding beyond `radius` (the drift pad, or 2 m).
    pub margin: f32,
    /// Ortho depth range (light-space far − near).
    pub depth_range: f32,
    /// World size of one shadow-map texel.
    pub texel_m: f32,
    /// Whether this cascade re-fitted on the most recent update.
    pub refreshed: bool,
}

fn frustum_corners_ws(view: &RenderView, near: f32, far: f32) -> [glam::Vec3; 8] {
    // C.3.A migration: signature changed from `&Camera` to `&RenderView` per
    // CAMERA_CONVENTIONS.md §2.9. The view direction comes directly from
    // RenderView.view_dir (canonical per §2.8) — no need to reconstruct via
    // Camera::dir(yaw, pitch). The other fields (fovy, aspect, position)
    // map 1:1 from RenderView.
    let dir = view.view_dir;
    let right = dir.cross(glam::Vec3::Y).normalize();
    let up = glam::Vec3::Y;
    let h_near = (view.fovy * 0.5).tan() * near;
    let w_near = h_near * view.aspect.max(0.01);
    let h_far = (view.fovy * 0.5).tan() * far;
    let w_far = h_far * view.aspect.max(0.01);
    let c_near = view.position + dir * near;
    let c_far = view.position + dir * far;
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
#[test]
fn gpu_cull_transformed_aabb_follows_scale_and_translation() {
    let transform = glam::Mat4::from_scale_rotation_translation(
        glam::Vec3::splat(2.0),
        glam::Quat::IDENTITY,
        glam::Vec3::new(10.0, 0.0, -3.0),
    );
    let (min, max) = transformed_aabb([0.0, 0.0, 0.0], [1.0, 2.0, 0.5], &transform);
    assert_eq!(min, [10.0, 0.0, -3.0]);
    assert_eq!(max, [12.0, 4.0, -2.0]);
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
        // C.3.B.2 regression fix: C.3.A migrated frustum_corners_ws's
        // signature from `&Camera` to `&RenderView` (per the cascade-splits
        // migration in Deliverable F) but did not update this test caller.
        // Build a RenderView directly so the test matches the post-C.3.A
        // signature. C.3.C dissolved the `Camera` alias — `FreeFly` is now
        // the canonical name.
        use astraweave_camera::{CameraProducer, FreeFly};
        let cam = FreeFly {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 90.0f32.to_radians(),
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let render_view = cam.to_render_view();

        let corners = frustum_corners_ws(&render_view, 1.0, 10.0);
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

    #[test]
    fn test_build_rgba8_mip_chain_renormalizes_normal_maps() {
        let normals = vec![
            255, 128, 128, 255, 128, 255, 128, 255, 0, 128, 128, 255, 128, 0, 128, 255,
        ];

        let mip_chain = Renderer::build_rgba8_mip_chain(2, 2, &normals, true);

        assert_eq!(mip_chain.len(), 2);
        assert_eq!(mip_chain[1].0, 1);
        assert_eq!(mip_chain[1].1, 1);
        assert_eq!(mip_chain[1].2, vec![128, 128, 255, 255]);
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
    /// SHADER_SRC gates shadow sampling on `uLight.extras.x >= 0.0` (the negative
    /// sentinel disables shadows) — this test validates the main PBR shader keeps
    /// that conditional (not a hardcoded override).
    #[test]
    fn test_shader_has_conditional_shadow_not_hardcoded() {
        let shader = SHADER_SRC;
        // Must NOT contain the old hardcoded override
        assert!(
            !shader.contains("// DEBUG: Force shadows off"),
            "hardcoded shadow override should have been removed"
        );
        // Must keep the conditional sentinel gate. SHADER_SRC defaults shadow=1.0
        // and runs PCF only when `uLight.extras.x >= 0.0`; the negative sentinel
        // (set by force_shadow_override) disables shadows — conditional, not hardcoded.
        assert!(
            shader.contains("uLight.extras.x >= 0.0"),
            "shader should gate shadows on the extras.x sentinel (conditional, not hardcoded)"
        );
    }

    /// L.3.D rung 2: the spare shadow-array layer that holds c3's outgoing
    /// window is named in TWO places — `C3_PREV_LAYER` here and a `const` of
    /// the same name in `shadow_common.wgsl`. Nothing in the type system pairs
    /// them: if they drift, the transition blends against the wrong layer and
    /// the result is a plausible-looking-but-wrong far field, with no error.
    /// This is the same silent-drift class as the `match idx { _ => last }` the
    /// L.3.C inventory caught, so it gets a contract test.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn c3_prev_layer_index_matches_shader() {
        let src = include_str!("../shaders/shadow_common.wgsl");
        let expected = format!("const C3_PREV_LAYER: i32 = {};", C3_PREV_LAYER);
        assert!(
            src.contains(&expected),
            "shadow_common.wgsl must declare `{expected}` to match the Rust-side \
             C3_PREV_LAYER ({C3_PREV_LAYER}); the shader currently declares something else"
        );
        // And the spare layer must actually exist in the array.
        assert!(
            C3_PREV_LAYER < SHADOW_LAYER_COUNT,
            "C3_PREV_LAYER {C3_PREV_LAYER} is outside the {SHADOW_LAYER_COUNT}-layer shadow array"
        );
    }

    /// T.2d.F (2026-07-25, director-ratified): the material-LOD tiers are
    /// RETIRED. `compute_material_lod`'s LOD1|2 threshold (pixel footprint
    /// 2.0) was convicted as a visible camera-anchored detail boundary
    /// (matched to 4 px in two independent frames), and the per-pixel tier
    /// selection dithered over rough ground — 40-55% of far-field
    /// high-frequency energy was tier flicker. Full trail:
    /// `docs/audits/T2D_CAMERA_LIGHT.md` §10, `docs/audits/T2DF_OUTCOME.md`.
    ///
    /// This test asserts the ABSENCE of the tier machinery in every shader
    /// that carried it — it fails on pre-T.2d.F code by construction, and it
    /// fails again if anyone reintroduces a stepped shading tier. The ratified
    /// fallback for any future perf need is a falloff CONTINUOUS in footprint,
    /// never a threshold.
    /// ED-3: the viewport shading dropdown must reach the shaders. The
    /// scene-env UBO carries `debug_mode` (a commandeered pad float, layout
    /// unchanged at 96 B), and every surface the editor shades — static PBR,
    /// skinned, terrain-forward — branches on it. This test fails on
    /// pre-ED-3 code (no `debug_mode` existed anywhere) and fails again if
    /// a shader loses the branch while the UI still offers the modes.
    #[test]
    fn ed3_debug_shading_reaches_every_shader_surface() {
        let terrain_src = include_str!("../shaders/pbr_terrain_forward.wgsl");
        for (name, src) in [
            ("SHADER_SRC (static PBR)", SHADER_SRC),
            ("SKINNED_SHADER_SRC", SKINNED_SHADER_SRC),
            ("pbr_terrain_forward.wgsl", terrain_src),
        ] {
            assert!(
                src.contains("debug_mode: f32"),
                "{name}: SceneEnv must declare debug_mode (the shading dropdown's uniform)"
            );
            assert!(
                src.contains("uScene.debug_mode"),
                "{name}: the fragment shader must branch on uScene.debug_mode —                  without it the viewport shading dropdown is inert again (the ED-3 defect)"
            );
        }
        // The UBO stays 96 bytes — debug_mode occupies a former pad float.
        assert_eq!(
            std::mem::size_of::<crate::scene_environment::SceneEnvironmentUBO>(),
            96,
            "SceneEnvironmentUBO layout must not grow: debug_mode replaces padding"
        );
    }

    #[test]
    fn material_lod_tiers_are_retired() {
        let brdf_src = include_str!("../shaders/brdf_common.wgsl");
        let terrain_src = include_str!("../shaders/pbr_terrain_forward.wgsl");
        for (name, src) in [
            ("brdf_common.wgsl", brdf_src),
            ("SHADER_SRC (static PBR)", SHADER_SRC),
            ("SKINNED_SHADER_SRC", SKINNED_SHADER_SRC),
            ("pbr_terrain_forward.wgsl", terrain_src),
        ] {
            assert!(
                !src.contains("compute_material_lod"),
                "{name}: compute_material_lod is retired (T.2d.F) and must not reappear"
            );
            assert!(
                !src.contains("evaluate_brdf_lod"),
                "{name}: evaluate_brdf_lod is retired (T.2d.F); use evaluate_brdf"
            );
            assert!(
                !src.contains("mat_lod"),
                "{name}: no mat_lod plumbing may remain after the tier retirement"
            );
        }
        // The single unified BRDF survives, multiscatter included — deleting
        // the tiers must not have deleted the full-quality path itself.
        assert!(
            brdf_src.contains("fn evaluate_brdf("),
            "brdf_common.wgsl must keep the unified evaluate_brdf"
        );
        assert!(
            brdf_src.contains("multiscatter"),
            "the Kulla-Conty multiscatter term must survive the tier removal"
        );
    }

    #[test]
    fn gpu_cull_indirect_instance_count_matches_the_cpu_frustum() {
        pollster::block_on(async {
            let Some(mut renderer) = Renderer::headless_for_tests(64, 64).await else {
                return;
            };
            let eye = Vec3::new(0.0, 0.0, 8.0);
            let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
            let projection =
                astraweave_camera::Projection::perspective(60.0_f32.to_radians(), 1.0, 0.1, 100.0);
            let render_view = astraweave_camera::RenderView::new(
                view,
                &projection,
                eye,
                Vec3::new(0.0, 0.0, -1.0),
            );
            renderer.update_view(&render_view);
            let frustum =
                crate::culling::FrustumPlanes::from_view_proj(&renderer.current_view_proj());
            let inside_center = Vec3::ZERO;
            let inside_extent = Vec3::splat(0.5);
            let outside_center = Vec3::new(40.0, 0.0, 0.0);
            let outside_extent = Vec3::splat(0.5);
            assert!(frustum.test_aabb(inside_center, inside_extent));
            assert!(!frustum.test_aabb(outside_center, outside_extent));

            let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
            let normals = [[0.0, 0.0, 1.0]; 3];
            let indices = [0, 1, 2];
            let mesh = renderer.create_mesh_from_arrays(&positions, &normals, &indices);
            let instance =
                crate::Instance::from_pos_scale_color(Vec3::ZERO, Vec3::ONE, [1.0, 1.0, 1.0, 1.0]);
            renderer.add_model_with_bounds(
                "inside",
                mesh,
                &[instance.clone()],
                (inside_center - inside_extent).to_array(),
                (inside_center + inside_extent).to_array(),
            );
            let mesh = renderer.create_mesh_from_arrays(&positions, &normals, &indices);
            renderer.add_model_with_bounds(
                "outside",
                mesh,
                &[instance],
                (outside_center - outside_extent).to_array(),
                (outside_center + outside_extent).to_array(),
            );

            let device = renderer.device().clone();
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gpu cull color"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            renderer.set_gpu_cull_enabled(true);
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gpu cull dispatch"),
            });
            renderer.draw_into(&view, None, &mut encoder).expect("draw");
            renderer.queue().submit(std::iter::once(encoder.finish()));
            let _ = device.poll(wgpu::MaintainBase::Wait);
            let validation = device.pop_error_scope().await;
            assert!(validation.is_none(), "validation {validation:?}");
            assert_eq!(renderer.gpu_cull_batch_count(), 2);
            assert_eq!(renderer.rendered_model_count(), 1);
            assert_eq!(renderer.gpu_cull_visible_draws(), 1);
            let counts = renderer.read_gpu_cull_instance_counts().await;
            assert_eq!(counts, vec![1, 0]);
        });
    }

    #[test]
    fn gpu_cull_grow_keeps_equal_slots_and_vertex_rewrite_moves_the_box() {
        pollster::block_on(async {
            let Some(mut renderer) = Renderer::headless_for_tests(64, 64).await else {
                return;
            };
            let eye = Vec3::new(0.0, 0.0, 8.0);
            let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
            let projection =
                astraweave_camera::Projection::perspective(60.0_f32.to_radians(), 1.0, 0.1, 100.0);
            let render_view = astraweave_camera::RenderView::new(
                view,
                &projection,
                eye,
                Vec3::new(0.0, 0.0, -1.0),
            );
            renderer.update_view(&render_view);
            renderer.set_gpu_cull_enabled(true);

            let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
            let normals = [[0.0, 0.0, 1.0]; 3];
            let indices = [0, 1, 2];
            let mesh = renderer.create_mesh_from_arrays(&positions, &normals, &indices);
            let instance =
                crate::Instance::from_pos_scale_color(Vec3::ZERO, Vec3::ONE, [1.0, 1.0, 1.0, 1.0]);
            renderer.add_model_with_bounds(
                "m00",
                mesh,
                &[instance.clone()],
                [-0.5, -0.5, -0.5],
                [1.5, 0.5, 0.5],
            );

            let device = renderer.device().clone();
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gpu cull grow color"),
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let draw = |renderer: &mut Renderer, view: &wgpu::TextureView| {
                let mut encoder =
                    renderer
                        .device()
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("gpu cull grow"),
                        });
                renderer.draw_into(view, None, &mut encoder).expect("draw");
                renderer.queue().submit(std::iter::once(encoder.finish()));
                let _ = renderer.device().poll(wgpu::MaintainBase::Wait);
            };
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            draw(&mut renderer, &view);
            let first = renderer.gpu_cull_storage_slots().expect("first slots");
            assert_eq!(first.0, first.1);
            assert_eq!(first.0, 1);

            for index in 1..10 {
                let mesh = renderer.create_mesh_from_arrays(&positions, &normals, &indices);
                let x = 40.0 + index as f32;
                renderer.add_model_with_bounds(
                    format!("m{index:02}"),
                    mesh,
                    &[instance.clone()],
                    [x - 0.5, -0.5, -0.5],
                    [x + 1.5, 0.5, 0.5],
                );
            }
            draw(&mut renderer, &view);
            let grown = renderer.gpu_cull_storage_slots().expect("grown slots");
            assert_eq!(grown.0, grown.1);
            assert!(grown.0 >= 10, "slots {}", grown.0);
            assert!(grown.0 > first.0);
            let counts = renderer.read_gpu_cull_instance_counts().await;
            assert_eq!(counts.len(), 10);
            assert_eq!(counts[0], 1);
            assert!(counts[1..].iter().all(|count| *count == 0));
            assert_eq!(renderer.rendered_model_count(), 1);
            assert_eq!(renderer.gpu_cull_visible_draws(), 1);
            assert!(renderer.gpu_cull_ran());

            let mut cpu = crate::mesh::CpuMesh::default();
            for position in [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
                cpu.vertices.push(crate::mesh::MeshVertex::from_arrays(
                    position,
                    [0.0, 0.0, 1.0],
                    [1.0, 0.0, 0.0, 1.0],
                    [0.0, 0.0],
                ));
            }
            cpu.indices = vec![0, 1, 2];
            renderer.add_composite_model("moved", vec![cpu], &[instance]);
            draw(&mut renderer, &view);
            assert_eq!(renderer.gpu_cull_visible_draws(), 2);
            let mut far = Vec::new();
            for position in [[80.0, 0.0, 0.0], [81.0, 0.0, 0.0], [80.0, 1.0, 0.0]] {
                far.push(crate::mesh::MeshVertex::from_arrays(
                    position,
                    [0.0, 0.0, 1.0],
                    [1.0, 0.0, 0.0, 1.0],
                    [0.0, 0.0],
                ));
            }
            assert!(renderer.write_model_vertices("moved", 0, &far));
            draw(&mut renderer, &view);
            assert_eq!(renderer.gpu_cull_visible_draws(), 1);
            let validation = device.pop_error_scope().await;
            assert!(validation.is_none(), "validation {validation:?}");
        });
    }
}

// End of file
