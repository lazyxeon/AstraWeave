// World normal and clip-space motion for the gated aux targets.
// The normal is the instance normal matrix times the vertex normal.
// Velocity is current NDC minus previous NDC. A missing previous matrix
// is the current matrix, so the first frame stores zero.

struct Matrices {
    curr: mat4x4<f32>,
    prev: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> mats: Matrices;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_n: vec3<f32>,
    @location(1) curr_clip: vec4<f32>,
    @location(2) prev_clip: vec4<f32>,
}

@vertex
fn vs(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) model0: vec4<f32>,
    @location(3) model1: vec4<f32>,
    @location(4) model2: vec4<f32>,
    @location(5) model3: vec4<f32>,
    @location(6) n0: vec3<f32>,
    @location(7) n1: vec3<f32>,
    @location(8) n2: vec3<f32>,
) -> VsOut {
    let model = mat4x4<f32>(model0, model1, model2, model3);
    let nmat = mat3x3<f32>(n0, n1, n2);
    let world = model * vec4<f32>(position, 1.0);
    var out: VsOut;
    out.curr_clip = mats.curr * world;
    out.prev_clip = mats.prev * world;
    out.clip = out.curr_clip;
    out.world_n = nmat * normal;
    return out;
}

struct FsOut {
    @location(0) normal: vec4<f32>,
    @location(1) velocity: vec2<f32>,
}

@fragment
fn fs(in: VsOut) -> FsOut {
    let curr_ndc = in.curr_clip.xy / in.curr_clip.w;
    let prev_ndc = in.prev_clip.xy / in.prev_clip.w;
    var out: FsOut;
    let n = normalize(in.world_n);
    out.normal = vec4<f32>(n, 1.0);
    out.velocity = curr_ndc - prev_ndc;
    return out;
}
