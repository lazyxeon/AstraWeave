// Aerial Perspective — Depth-based atmospheric scattering on scene geometry
//
// Applies atmospheric in-scattering and extinction to scene geometry based on
// pixel depth. Near objects appear normal; distant objects fade toward the
// sky color as light scatters along the view path.
//
// This replaces traditional distance fog with physically-correct atmosphere.

struct AerialParams {
    inv_view_proj:     mat4x4<f32>,
    view_pos:          vec3<f32>,
    planet_radius:     f32,
    atmosphere_height: f32,
    rayleigh_scale_h:  f32,
    mie_scale_h:       f32,
    mie_g:             f32,
    rayleigh_scatter:  vec3<f32>,
    mie_scatter:       f32,
    sun_dir:           vec3<f32>,
    sun_intensity:     f32,
    sun_color:         vec3<f32>,
    max_distance:      f32,     // max aerial perspective distance (km)
    resolution:        vec2<f32>,
    inv_resolution:    vec2<f32>,
    near_plane:        f32,
    far_plane:         f32,
    ozone_center_h:    f32,
    ozone_width:       f32,
    ozone_absorption:  vec3<f32>,
    mie_absorption:    f32,
};

@group(0) @binding(0) var<uniform>  params:          AerialParams;
@group(0) @binding(1) var           t_scene:         texture_2d<f32>;   // lit scene color
@group(0) @binding(2) var           t_depth:         texture_2d<f32>;   // scene depth
@group(0) @binding(3) var           t_transmittance: texture_2d<f32>;   // atmosphere LUT
@group(0) @binding(4) var           s_linear:        sampler;
@group(0) @binding(5) var           t_output:        texture_storage_2d<rgba16float, write>;

const PI: f32 = 3.14159265358979;
const AERIAL_STEPS: u32 = 16u;

fn phase_rayleigh(cos_theta: f32) -> f32 {
    return 3.0 / (16.0 * PI) * (1.0 + cos_theta * cos_theta);
}

fn phase_mie(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return (1.0 - g2) / (4.0 * PI * pow(denom, 1.5));
}

fn density_rayleigh(h: f32) -> f32 { return exp(-h / params.rayleigh_scale_h); }
fn density_mie(h: f32) -> f32 { return exp(-h / params.mie_scale_h); }
fn density_ozone(h: f32) -> f32 {
    return max(0.0, 1.0 - abs(h - params.ozone_center_h) / params.ozone_width);
}

fn extinction_at(h: f32) -> vec3<f32> {
    let rayleigh = params.rayleigh_scatter * density_rayleigh(h);
    let mie_ext = (params.mie_scatter + params.mie_absorption) * density_mie(h);
    let ozone = params.ozone_absorption * density_ozone(h);
    return rayleigh + vec3<f32>(mie_ext) + ozone;
}

fn sample_transmittance(h: f32, cos_zenith: f32) -> vec3<f32> {
    let u = (cos_zenith + 1.0) * 0.5;
    let v = sqrt(h / params.atmosphere_height);
    return textureSampleLevel(t_transmittance, s_linear, vec2<f32>(u, v), 0.0).rgb;
}

fn linearize_depth(d: f32) -> f32 {
    let near = params.near_plane;
    let far = params.far_plane;
    return near * far / (far - d * (far - near));
}

fn reconstruct_world_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let world_h = params.inv_view_proj * ndc;
    return world_h.xyz / world_h.w;
}

@compute @workgroup_size(8, 8)
fn aerial_perspective_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(params.resolution);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let pixel = vec2<i32>(gid.xy);
    let uv = (vec2<f32>(gid.xy) + 0.5) * params.inv_resolution;

    let scene_color = textureSampleLevel(t_scene, s_linear, uv, 0.0).rgb;
    let depth_raw = textureLoad(t_depth, pixel, 0).r;

    // Sky pixels: pass through unchanged (already rendered by sky pass)
    if (depth_raw >= 0.9999) {
        textureStore(t_output, pixel, vec4<f32>(scene_color, 1.0));
        return;
    }

    let world_pos = reconstruct_world_pos(uv, depth_raw);
    let ray_dir = normalize(world_pos - params.view_pos);
    let distance_m = length(world_pos - params.view_pos);
    let distance_km = distance_m * 0.001;

    // Clamp integration distance
    let max_dist = min(distance_km, params.max_distance);

    // Short-circuit for very close geometry
    if (max_dist < 0.001) {
        textureStore(t_output, pixel, vec4<f32>(scene_color, 1.0));
        return;
    }

    let view_h = max(params.view_pos.y * 0.001, 0.001); // m to km
    let cos_zenith = ray_dir.y;
    let cos_sun = dot(ray_dir, params.sun_dir);
    let phase_r = phase_rayleigh(cos_sun);
    let phase_m = phase_mie(cos_sun, params.mie_g);

    let step_size = max_dist / f32(AERIAL_STEPS);
    let r = params.planet_radius + view_h;

    var optical_depth = vec3<f32>(0.0);
    var in_scatter = vec3<f32>(0.0);

    for (var i = 0u; i < AERIAL_STEPS; i++) {
        let t = (f32(i) + 0.5) * step_size;
        let r_sample = sqrt(r * r + t * t + 2.0 * r * t * cos_zenith);
        let h_sample = r_sample - params.planet_radius;

        if (h_sample < 0.0) { break; }

        let ext = extinction_at(h_sample);
        optical_depth += ext * step_size;

        let transmittance_camera = exp(-optical_depth);
        let cos_sun_zenith = dot(normalize(vec3<f32>(0.0, r_sample, 0.0)), params.sun_dir);
        let transmittance_sun = sample_transmittance(h_sample, cos_sun_zenith);

        let rayleigh_s = params.rayleigh_scatter * density_rayleigh(h_sample);
        let mie_s = params.mie_scatter * density_mie(h_sample);

        let scatter = rayleigh_s * phase_r + vec3<f32>(mie_s) * phase_m;
        in_scatter += scatter * transmittance_camera * transmittance_sun * step_size;
    }

    let transmittance = exp(-optical_depth);
    in_scatter *= params.sun_intensity * params.sun_color;

    // Composite: scene * transmittance + in_scatter
    let result = scene_color * transmittance + in_scatter;

    textureStore(t_output, pixel, vec4<f32>(result, 1.0));
}
