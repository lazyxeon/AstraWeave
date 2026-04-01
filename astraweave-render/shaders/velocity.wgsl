// Motion vector / velocity buffer generation
//
// Computes per-pixel screen-space velocity by comparing the current frame's
// clip-space position against the previous frame's. The output is stored in
// an RG16Float render target where R = horizontal delta, G = vertical delta,
// both in UV space (range approximately -1..1).
//
// Used by: TAA, motion blur, temporal upscaling, temporal reprojection.

struct VSIn {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    // Instance data (model matrix columns)
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
};

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) curr_pos_cs: vec4<f32>,
    @location(1) prev_pos_cs: vec4<f32>,
};

struct VelocityUniforms {
    curr_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uVelocity: VelocityUniforms;

// Previous frame model matrix (for per-object motion).
// When objects are static, this equals the current model matrix.
struct PrevTransform {
    prev_m0: vec4<f32>,
    prev_m1: vec4<f32>,
    prev_m2: vec4<f32>,
    prev_m3: vec4<f32>,
};

@group(0) @binding(1) var<uniform> uPrevTransform: PrevTransform;

@vertex
fn vs_velocity(input: VSIn) -> VSOut {
    // Current frame: world position via current model matrix
    let model = mat4x4<f32>(input.m0, input.m1, input.m2, input.m3);
    let world_curr = model * vec4<f32>(input.position, 1.0);
    let clip_curr = uVelocity.curr_view_proj * world_curr;

    // Previous frame: world position via previous model matrix
    let prev_model = mat4x4<f32>(
        uPrevTransform.prev_m0,
        uPrevTransform.prev_m1,
        uPrevTransform.prev_m2,
        uPrevTransform.prev_m3,
    );
    let world_prev = prev_model * vec4<f32>(input.position, 1.0);
    let clip_prev = uVelocity.prev_view_proj * world_prev;

    var out: VSOut;
    out.pos = clip_curr;
    out.curr_pos_cs = clip_curr;
    out.prev_pos_cs = clip_prev;
    return out;
}

@fragment
fn fs_velocity(input: VSOut) -> @location(0) vec2<f32> {
    // Convert clip space to NDC
    let curr_ndc = input.curr_pos_cs.xy / input.curr_pos_cs.w;
    let prev_ndc = input.prev_pos_cs.xy / input.prev_pos_cs.w;

    // Convert NDC (-1..1) to UV space (0..1)
    let curr_uv = curr_ndc * 0.5 + vec2<f32>(0.5, 0.5);
    let prev_uv = prev_ndc * 0.5 + vec2<f32>(0.5, 0.5);

    // Velocity = current - previous (in UV space)
    let velocity = curr_uv - prev_uv;

    return velocity;
}
