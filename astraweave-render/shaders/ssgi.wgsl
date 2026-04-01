// Screen-Space Global Illumination (SSGI)
//
// One-bounce indirect diffuse lighting via screen-space ray marching against
// the Hi-Z depth buffer. Traces rays in random hemisphere directions from each
// pixel's surface point, samples the color buffer at hit locations.

struct SsgiParams {
    inv_proj: mat4x4<f32>,
    proj: mat4x4<f32>,
    resolution: vec2<f32>,
    inv_resolution: vec2<f32>,
    max_ray_distance: f32,
    ray_step_size: f32,
    num_rays: u32,
    max_steps: u32,
    thickness: f32,
    intensity: f32,
    frame_index: u32,
    _pad: u32,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var color_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> params: SsgiParams;
@group(0) @binding(5) var gi_output: texture_storage_2d<rgba16float, write>;

const PI: f32 = 3.14159265;

// Interleaved Gradient Noise
fn ign(pixel: vec2<f32>, frame: f32) -> f32 {
    let magic = vec3<f32>(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(pixel + frame * vec2<f32>(5.0, 3.0), magic.xy)));
}

// Hash function for random direction generation
fn hash2(p: vec2<f32>) -> vec2<f32> {
    let n = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(n) * 43758.5453);
}

// Generate cosine-weighted hemisphere direction in tangent space
fn cosine_hemisphere(u: vec2<f32>) -> vec3<f32> {
    let r = sqrt(u.x);
    let theta = 2.0 * PI * u.y;
    let x = r * cos(theta);
    let z = r * sin(theta);
    let y = sqrt(max(0.0, 1.0 - u.x));
    return vec3<f32>(x, y, z);
}

// Reconstruct view-space position from UV and depth
fn reconstruct_view_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let view_pos = params.inv_proj * ndc;
    return view_pos.xyz / view_pos.w;
}

// Project view-space position to screen UV
fn project_to_uv(view_pos: vec3<f32>) -> vec3<f32> {
    let clip = params.proj * vec4<f32>(view_pos, 1.0);
    let ndc = clip.xyz / clip.w;
    return vec3<f32>(ndc.xy * 0.5 + 0.5, ndc.z);
}

// Screen-space ray march
fn ray_march(
    origin: vec3<f32>,
    direction: vec3<f32>,
) -> vec4<f32> {
    var ray_pos = origin;
    let step = direction * params.ray_step_size;

    for (var i = 0u; i < params.max_steps; i++) {
        ray_pos = ray_pos + step;

        let projected = project_to_uv(ray_pos);
        let screen_uv = vec2<f32>(projected.x, 1.0 - projected.y);

        // Bounds check
        if (screen_uv.x < 0.0 || screen_uv.x > 1.0 || screen_uv.y < 0.0 || screen_uv.y > 1.0) {
            break;
        }

        let sampled_depth = textureSampleLevel(depth_tex, samp, screen_uv, 0.0).r;
        let sampled_view_pos = reconstruct_view_pos(screen_uv, sampled_depth);

        // Depth comparison with thickness
        let depth_diff = ray_pos.z - sampled_view_pos.z;
        if (depth_diff > 0.0 && depth_diff < params.thickness) {
            // Hit! Sample the color at this location
            let hit_color = textureSampleLevel(color_tex, samp, screen_uv, 0.0).rgb;
            let fade = 1.0 - f32(i) / f32(params.max_steps);
            return vec4<f32>(hit_color * fade, 1.0);
        }
    }

    return vec4<f32>(0.0);
}

@compute @workgroup_size(8, 8, 1)
fn ssgi_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pixel = vec2<i32>(gid.xy);
    let dims = vec2<i32>(params.resolution);
    if (pixel.x >= dims.x || pixel.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(pixel) + 0.5) * params.inv_resolution;
    let depth = textureSampleLevel(depth_tex, samp, uv, 0.0).r;

    if (depth >= 1.0) {
        textureStore(gi_output, pixel, vec4<f32>(0.0));
        return;
    }

    let view_pos = reconstruct_view_pos(uv, depth);
    let normal_raw = textureSampleLevel(normal_tex, samp, uv, 0.0).rgb;
    let N = normalize(normal_raw * 2.0 - 1.0);

    // Build TBN
    let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(N.y) > 0.9);
    let T = normalize(cross(up, N));
    let B = cross(N, T);

    var total_gi = vec3<f32>(0.0);
    let frame_f = f32(params.frame_index);

    for (var ray = 0u; ray < params.num_rays; ray++) {
        // Random direction in hemisphere
        let noise = hash2(vec2<f32>(pixel) + vec2<f32>(f32(ray) * 7.0, frame_f * 11.0));
        let local_dir = cosine_hemisphere(noise);
        let world_dir = T * local_dir.x + N * local_dir.y + B * local_dir.z;

        let result = ray_march(view_pos + N * 0.05, world_dir);
        total_gi = total_gi + result.rgb * result.a;
    }

    total_gi = total_gi / f32(params.num_rays) * params.intensity;
    textureStore(gi_output, pixel, vec4<f32>(total_gi, 1.0));
}
