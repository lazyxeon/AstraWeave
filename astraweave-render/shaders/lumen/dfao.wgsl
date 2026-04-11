// Distance Field Ambient Occlusion (DFAO) — Lumen GI
//
// Computes long-range ambient occlusion by sphere-tracing through a 3D signed
// distance field volume. Produces soft, contact-hardening AO that extends far
// beyond screen-space methods (GTAO/SSAO).

struct DfaoParams {
    inv_view_proj:  mat4x4<f32>,
    view_pos:       vec3<f32>,
    max_distance:   f32,     // maximum trace distance in world units
    resolution:     vec2<f32>,
    inv_resolution: vec2<f32>,
    sdf_origin:     vec3<f32>,
    sdf_inv_size:   f32,     // 1.0 / sdf_world_size
    sdf_dims:       vec3<u32>,
    num_steps:      u32,     // sphere trace iterations
    ao_power:       f32,     // contrast exponent
    ao_intensity:   f32,     // multiplier
    near_plane:     f32,
    far_plane:      f32,
};

@group(0) @binding(0) var<uniform>        params:    DfaoParams;
@group(0) @binding(1) var                 t_depth:   texture_2d<f32>;
@group(0) @binding(2) var                 t_normal:  texture_2d<f32>;
@group(0) @binding(3) var                 t_sdf:     texture_3d<f32>;
@group(0) @binding(4) var                 s_linear:  sampler;
@group(0) @binding(5) var                 t_output:  texture_storage_2d<rgba16float, write>;

// Reconstruct world position from depth buffer
fn reconstruct_world_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let world_h = params.inv_view_proj * ndc;
    return world_h.xyz / world_h.w;
}

// Linearize depth
fn linearize_depth(d: f32) -> f32 {
    let near = params.near_plane;
    let far = params.far_plane;
    return near * far / (far - d * (far - near));
}

// Sample the SDF volume at a world position, returning the signed distance
fn sample_sdf(world_pos: vec3<f32>) -> f32 {
    let uvw = (world_pos - params.sdf_origin) * params.sdf_inv_size;
    // Clamp to valid volume range
    let clamped = clamp(uvw, vec3<f32>(0.001), vec3<f32>(0.999));
    return textureSampleLevel(t_sdf, s_linear, clamped, 0.0).r;
}

// Cone-traced AO: trace a widening cone through the SDF
// Returns occlusion in [0, 1] where 1 = fully lit
fn trace_ao_cone(origin: vec3<f32>, dir: vec3<f32>, cone_angle: f32) -> f32 {
    var occlusion = 0.0;
    var t = 0.02; // small initial offset to avoid self-intersection

    for (var i = 0u; i < params.num_steps; i++) {
        let pos = origin + dir * t;
        let dist = sample_sdf(pos);

        // Cone radius at this distance
        let cone_radius = t * cone_angle;

        // If SDF distance is less than cone radius, we have occlusion
        if (dist < cone_radius) {
            // Soft occlusion proportional to how much the cone is occluded
            let soft_occ = 1.0 - saturate(dist / cone_radius);
            // Weight by inverse distance (closer occlusion matters more)
            let weight = 1.0 / (1.0 + t * t);
            occlusion += soft_occ * weight;
        }

        // Advance by at least the SDF distance (sphere tracing)
        t += max(dist, 0.05);

        if (t > params.max_distance) {
            break;
        }
    }

    return saturate(1.0 - occlusion * params.ao_intensity);
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y)
fn dfao_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(textureDimensions(t_depth));
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;
    let depth_raw = textureLoad(t_depth, vec2<i32>(gid.xy), 0).r;

    // Skip sky pixels
    if (depth_raw >= 1.0) {
        textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }

    let world_pos = reconstruct_world_pos(uv, depth_raw);
    let normal = normalize(textureLoad(t_normal, vec2<i32>(gid.xy), 0).xyz * 2.0 - 1.0);

    // Trace multiple cones around the normal hemisphere
    // 4 cones at 45° from normal + 1 along normal
    let cone_half_angle = 0.4; // ~23 degrees

    // Build tangent frame
    let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(normal.y) > 0.99);
    let tangent = normalize(cross(up, normal));
    let bitangent = cross(normal, tangent);

    var ao = 0.0;

    // Center cone along normal
    ao += trace_ao_cone(world_pos, normal, cone_half_angle);

    // 4 tilted cones
    let tilt = 0.707; // sin(45°)
    let lift = 0.707; // cos(45°)
    let dirs = array<vec3<f32>, 4>(
        normalize(normal * lift + tangent * tilt),
        normalize(normal * lift - tangent * tilt),
        normalize(normal * lift + bitangent * tilt),
        normalize(normal * lift - bitangent * tilt),
    );

    for (var i = 0; i < 4; i++) {
        ao += trace_ao_cone(world_pos, dirs[i], cone_half_angle);
    }

    ao /= 5.0;
    ao = pow(ao, params.ao_power);

    textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(ao, 0.0, 0.0, 0.0));
}
