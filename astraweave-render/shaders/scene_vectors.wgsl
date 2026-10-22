// View-space normals and UV motion for the gated aux targets.
// SSGI decodes `normal * 2 - 1` and marches in view space.
// SSGI denoise, TAA, and temporal upscale subtract this velocity from UV
// (Y down, [0, 1]). NDC is computed in the vertex shader and interpolated
// without perspective correction. The first frame's previous matrix is the
// current one, so motion is zero.

struct Matrices {
    curr_vp: mat4x4<f32>,
    prev_vp: mat4x4<f32>,
    view: mat4x4<f32>,
    alpha_cutoff: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
}

@group(0) @binding(0)
var<uniform> mats: Matrices;

@group(1) @binding(0) var albedo_tex: texture_2d<f32>;
@group(1) @binding(1) var albedo_samp: sampler;
@group(1) @binding(4) var normal_tex: texture_2d<f32>;
@group(1) @binding(5) var normal_samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_n: vec3<f32>,
    @location(1) tbn0: vec3<f32>,
    @location(2) tbn1: vec3<f32>,
    @location(3) tbn2: vec3<f32>,
    @location(4) uv: vec2<f32>,
    @location(5) @interpolate(linear) curr_ndc: vec2<f32>,
    @location(6) @interpolate(linear) prev_ndc: vec2<f32>,
}

fn ndc_xy(clip: vec4<f32>) -> vec2<f32> {
    return clip.xy / clip.w;
}

fn normal_from_model(model: mat4x4<f32>) -> mat3x3<f32> {
    let c0 = model[0].xyz;
    let c1 = model[1].xyz;
    let c2 = model[2].xyz;
    let axis0 = cross(c1, c2);
    let inv_det = 1.0 / dot(c0, axis0);
    // Columns are the cofactors, so this is transpose(inverse(upper)).
    return mat3x3<f32>(axis0 * inv_det, cross(c2, c0) * inv_det, cross(c0, c1) * inv_det);
}

@vertex
fn vs(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) model0: vec4<f32>,
    @location(3) model1: vec4<f32>,
    @location(4) model2: vec4<f32>,
    @location(5) model3: vec4<f32>,
    @location(6) prev0: vec4<f32>,
    @location(7) prev1: vec4<f32>,
    @location(8) prev2: vec4<f32>,
    @location(10) prev3: vec4<f32>,
    @location(12) tangent: vec4<f32>,
    @location(13) uv: vec2<f32>,
) -> VsOut {
    let model = mat4x4<f32>(model0, model1, model2, model3);
    let prev_model = mat4x4<f32>(prev0, prev1, prev2, prev3);
    let nmat = normal_from_model(model);
    let curr_world = model * vec4<f32>(position, 1.0);
    let prev_world = prev_model * vec4<f32>(position, 1.0);
    let curr_clip = mats.curr_vp * curr_world;
    let prev_clip = mats.prev_vp * prev_world;
    let nw = normalize(nmat * normal);
    let tw = normalize(nmat * tangent.xyz);
    let bw = normalize(cross(nw, tw)) * tangent.w;
    var out: VsOut;
    out.clip = curr_clip;
    out.world_n = nw;
    out.tbn0 = tw;
    out.tbn1 = bw;
    out.tbn2 = nw;
    out.uv = uv;
    out.curr_ndc = ndc_xy(curr_clip);
    out.prev_ndc = ndc_xy(prev_clip);
    return out;
}

struct FsOut {
    @location(0) normal: vec4<f32>,
    @location(1) velocity: vec2<f32>,
}

fn pack_view_normal(world_n: vec3<f32>) -> vec4<f32> {
    let view_n = normalize((mats.view * vec4<f32>(world_n, 0.0)).xyz);
    return vec4<f32>(view_n * 0.5 + vec3<f32>(0.5), 1.0);
}

fn uv_delta(curr_ndc: vec2<f32>, prev_ndc: vec2<f32>) -> vec2<f32> {
    // Framebuffer UV is Y down. Consumers do `history_uv = uv - velocity`.
    let curr_uv = vec2<f32>(curr_ndc.x, -curr_ndc.y) * 0.5 + vec2<f32>(0.5);
    let prev_uv = vec2<f32>(prev_ndc.x, -prev_ndc.y) * 0.5 + vec2<f32>(0.5);
    return curr_uv - prev_uv;
}

@fragment
fn fs(in: VsOut) -> FsOut {
    let albedo = textureSample(albedo_tex, albedo_samp, in.uv);
    if (albedo.a < mats.alpha_cutoff) {
        discard;
    }
    let nrm = textureSample(normal_tex, normal_samp, in.uv).rgb;
    let tangent = normalize(nrm * 2.0 - vec3<f32>(1.0));
    let world_n = normalize(in.tbn0 * tangent.x + in.tbn1 * tangent.y + in.tbn2 * tangent.z);
    var out: FsOut;
    out.normal = pack_view_normal(world_n);
    out.velocity = uv_delta(in.curr_ndc, in.prev_ndc);
    return out;
}

// Terrain chunks are already in world space and have no instance matrix.
@vertex
fn vs_terrain(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
) -> VsOut {
    let curr_clip = mats.curr_vp * vec4<f32>(position, 1.0);
    let prev_clip = mats.prev_vp * vec4<f32>(position, 1.0);
    let n = normalize(normal);
    var out: VsOut;
    out.clip = curr_clip;
    out.world_n = n;
    out.tbn0 = n;
    out.tbn1 = n;
    out.tbn2 = n;
    out.uv = vec2<f32>(0.0);
    out.curr_ndc = ndc_xy(curr_clip);
    out.prev_ndc = ndc_xy(prev_clip);
    return out;
}

@fragment
fn fs_terrain(in: VsOut) -> FsOut {
    var out: FsOut;
    out.normal = pack_view_normal(in.world_n);
    out.velocity = uv_delta(in.curr_ndc, in.prev_ndc);
    return out;
}
