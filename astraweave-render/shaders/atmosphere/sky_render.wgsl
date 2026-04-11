// Bruneton Atmosphere — Sky Rendering + Sun/Moon Disc + Aerial Perspective
//
// Full-screen compute pass that renders the sky using precomputed transmittance LUT.
// Performs single-scattering integration along view rays with Rayleigh + Mie.
// Adds sun disc, moon disc, and stars for the night sky.

struct SkyRenderParams {
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
    sun_disk_size:     f32,     // angular radius in radians (~0.0093)
    moon_dir:          vec3<f32>,
    moon_intensity:    f32,
    moon_color:        vec3<f32>,
    exposure:          f32,
    resolution:        vec2<f32>,
    inv_resolution:    vec2<f32>,
    ozone_center_h:    f32,
    ozone_width:       f32,
    _pad0:             f32,
    _pad1:             f32,
    ozone_absorption:  vec3<f32>,
    mie_absorption:    f32,
};

@group(0) @binding(0) var<uniform>  params:          SkyRenderParams;
@group(0) @binding(1) var           t_transmittance: texture_2d<f32>; // precomputed LUT
@group(0) @binding(2) var           s_linear:        sampler;
@group(0) @binding(3) var           t_output:        texture_storage_2d<rgba16float, write>;

// PI, TWO_PI, HALF_PI, INV_PI provided by constants.wgsl (prepended on Rust side).
const NUM_SCATTER_STEPS: u32 = 32u;

// ---- Phase functions ----

fn phase_rayleigh(cos_theta: f32) -> f32 {
    return 3.0 / (16.0 * PI) * (1.0 + cos_theta * cos_theta);
}

fn phase_mie(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return (1.0 - g2) / (4.0 * PI * pow(denom, 1.5));
}

// ---- Density at altitude ----

fn density_rayleigh(h: f32) -> f32 {
    return exp(-h / params.rayleigh_scale_h);
}

fn density_mie(h: f32) -> f32 {
    return exp(-h / params.mie_scale_h);
}

fn density_ozone(h: f32) -> f32 {
    return max(0.0, 1.0 - abs(h - params.ozone_center_h) / params.ozone_width);
}

// ---- Extinction at altitude ----

fn extinction_at(h: f32) -> vec3<f32> {
    let rayleigh = params.rayleigh_scatter * density_rayleigh(h);
    let mie_ext = (params.mie_scatter + params.mie_absorption) * density_mie(h);
    let ozone = params.ozone_absorption * density_ozone(h);
    return rayleigh + vec3<f32>(mie_ext) + ozone;
}

// ---- Look up transmittance from LUT ----

fn sample_transmittance(h: f32, cos_zenith: f32) -> vec3<f32> {
    let u = (cos_zenith + 1.0) * 0.5;
    let v = sqrt(h / params.atmosphere_height);
    return textureSampleLevel(t_transmittance, s_linear, vec2<f32>(u, v), 0.0).rgb;
}

// ---- Ray-sphere intersection ----

fn ray_sphere_dist(origin_h: f32, cos_zenith: f32, sphere_r: f32) -> f32 {
    let r = params.planet_radius + origin_h;
    let b = 2.0 * r * cos_zenith;
    let c = r * r - sphere_r * sphere_r;
    let disc = b * b - 4.0 * c;
    if (disc < 0.0) { return -1.0; }
    return (-b + sqrt(disc)) * 0.5;
}

// ---- View ray direction from pixel ----

fn pixel_to_ray(pixel: vec2<f32>) -> vec3<f32> {
    let uv = pixel * params.inv_resolution;
    let ndc = vec4<f32>(uv * 2.0 - 1.0, 1.0, 1.0);
    let world_h = params.inv_view_proj * ndc;
    let world_pos = world_h.xyz / world_h.w;
    return normalize(world_pos - params.view_pos);
}

// ---- Single-scattering integration ----

fn compute_sky_color(ray_dir: vec3<f32>) -> vec3<f32> {
    let view_h = max(params.view_pos.y * 0.001, 0.001); // convert m to km, clamp above surface

    // Cosine of angle between view ray and zenith (vertical)
    let cos_zenith = ray_dir.y;

    let top_r = params.planet_radius + params.atmosphere_height;
    let ray_length = ray_sphere_dist(view_h, cos_zenith, top_r);

    if (ray_length < 0.0) {
        return vec3<f32>(0.0); // no intersection with atmosphere
    }

    // Check for planet intersection (below horizon)
    let planet_dist = ray_sphere_dist(view_h, cos_zenith, params.planet_radius);
    var max_dist = ray_length;
    if (planet_dist > 0.0) {
        max_dist = planet_dist;
    }

    let step_size = max_dist / f32(NUM_SCATTER_STEPS);
    let r = params.planet_radius + view_h;

    let cos_sun = dot(ray_dir, params.sun_dir);
    let phase_r = phase_rayleigh(cos_sun);
    let phase_m = phase_mie(cos_sun, params.mie_g);

    var total_scatter = vec3<f32>(0.0);
    var optical_depth = vec3<f32>(0.0);

    for (var i = 0u; i < NUM_SCATTER_STEPS; i++) {
        let t = (f32(i) + 0.5) * step_size;

        // Sample altitude along ray
        let r_sample = sqrt(r * r + t * t + 2.0 * r * t * cos_zenith);
        let h_sample = r_sample - params.planet_radius;

        if (h_sample < 0.0) { break; }

        // Extinction for this step
        let ext = extinction_at(h_sample);
        optical_depth += ext * step_size;

        // Transmittance from camera to this point
        let transmittance_camera = exp(-optical_depth);

        // Transmittance from this point to sun (from LUT)
        let cos_sun_zenith = dot(normalize(vec3<f32>(0.0, r_sample, 0.0)), params.sun_dir);
        let transmittance_sun = sample_transmittance(h_sample, cos_sun_zenith);

        // Scattering at this point
        let rayleigh_s = params.rayleigh_scatter * density_rayleigh(h_sample);
        let mie_s = params.mie_scatter * density_mie(h_sample);

        let scatter = (rayleigh_s * phase_r + vec3<f32>(mie_s) * phase_m);
        total_scatter += scatter * transmittance_camera * transmittance_sun * step_size;
    }

    return total_scatter * params.sun_intensity * params.sun_color;
}

// ---- Sun/Moon disc ----

fn sun_disc(ray_dir: vec3<f32>) -> vec3<f32> {
    let cos_angle = dot(ray_dir, params.sun_dir);
    let angle = acos(clamp(cos_angle, -1.0, 1.0));

    if (angle < params.sun_disk_size) {
        // Smooth edge (limb darkening)
        let edge = 1.0 - smoothstep(params.sun_disk_size * 0.8, params.sun_disk_size, angle);
        return params.sun_color * params.sun_intensity * 100.0 * edge;
    }
    return vec3<f32>(0.0);
}

fn moon_disc(ray_dir: vec3<f32>) -> vec3<f32> {
    let cos_angle = dot(ray_dir, params.moon_dir);
    let angle = acos(clamp(cos_angle, -1.0, 1.0));

    if (angle < params.sun_disk_size * 1.05) {
        let edge = 1.0 - smoothstep(params.sun_disk_size * 0.85, params.sun_disk_size * 1.05, angle);
        return params.moon_color * params.moon_intensity * edge;
    }
    return vec3<f32>(0.0);
}

// ---- Stars ----

fn stars(ray_dir: vec3<f32>) -> vec3<f32> {
    // Simple procedural stars based on direction hash
    let p = ray_dir * 500.0;
    let ip = floor(p);
    var h = dot(ip, vec3<f32>(127.1, 311.7, 74.7));
    h = fract(sin(h) * 43758.5453);
    if (h > 0.998) {
        let brightness = (h - 0.998) / 0.002;
        return vec3<f32>(brightness * brightness * 3.0);
    }
    return vec3<f32>(0.0);
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y)
fn sky_render_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<u32>(params.resolution);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let ray_dir = pixel_to_ray(vec2<f32>(gid.xy) + 0.5);

    // Atmosphere scattering
    var color = compute_sky_color(ray_dir);

    // Sun disc (only if above horizon)
    if (params.sun_dir.y > -0.05) {
        color += sun_disc(ray_dir);
    }

    // Moon disc and stars (night)
    if (params.sun_dir.y < 0.1) {
        let night_factor = saturate(-params.sun_dir.y * 5.0);
        color += moon_disc(ray_dir) * night_factor;
        color += stars(ray_dir) * night_factor;
    }

    // Apply exposure
    color *= params.exposure;

    textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(color, 1.0));
}
