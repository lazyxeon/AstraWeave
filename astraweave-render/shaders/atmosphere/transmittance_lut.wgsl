// Bruneton Atmosphere — Transmittance LUT Generation
//
// Precomputes optical depth along view rays through the atmosphere.
// LUT is parameterized by (cos_zenith, altitude) → RGB transmittance.
// Only needs recomputation when atmosphere parameters change.
//
// Reference: Bruneton & Neyret 2008, Hillaire 2020 (Epic's sky model)

struct AtmosphereParams {
    // Planet
    planet_radius:       f32,     // km (Earth ≈ 6371)
    atmosphere_height:   f32,     // km above surface (Earth ≈ 100)
    // Rayleigh scattering
    rayleigh_scale_h:    f32,     // scale height in km (Earth ≈ 8.0)
    rayleigh_scatter:    vec3<f32>, // scattering coefficients at sea level
    _pad0:               f32,
    // Mie scattering
    mie_scale_h:         f32,     // scale height in km (Earth ≈ 1.2)
    mie_scatter:         f32,     // scattering coefficient at sea level
    mie_absorption:      f32,     // absorption coefficient (extinction - scatter)
    mie_g:               f32,     // Henyey-Greenstein asymmetry (Earth ≈ 0.8)
    // Ozone absorption
    ozone_center_h:      f32,     // center altitude in km (Earth ≈ 25)
    ozone_width:         f32,     // layer width in km (Earth ≈ 15)
    ozone_absorption:    vec3<f32>, // absorption coefficients
    _pad1:               f32,
    // LUT dimensions
    lut_width:           u32,     // cos_zenith resolution (256)
    lut_height:          u32,     // altitude resolution (64)
    _pad2:               u32,
    _pad3:               u32,
};

@group(0) @binding(0) var<uniform>  params: AtmosphereParams;
@group(0) @binding(1) var           t_output: texture_storage_2d<rgba16float, write>;

// PI, TWO_PI, HALF_PI, INV_PI provided by constants.wgsl (prepended on Rust side).
const NUM_STEPS: u32 = 40u;

fn atmosphere_top(params_r: f32, params_h: f32) -> f32 {
    return params_r + params_h;
}

// Ray-sphere intersection: returns distance to nearest intersection
// with sphere of given radius centered at origin.
// Returns -1.0 if no intersection.
fn ray_sphere_intersect(origin_h: f32, cos_zenith: f32, sphere_radius: f32) -> f32 {
    // Origin at (0, planet_radius + origin_h), direction = (sin_zenith, cos_zenith)
    let r = params.planet_radius + origin_h;
    let b = 2.0 * r * cos_zenith;
    let c = r * r - sphere_radius * sphere_radius;
    let discriminant = b * b - 4.0 * c;
    if (discriminant < 0.0) {
        return -1.0;
    }
    return (-b + sqrt(discriminant)) * 0.5;
}

// Density at a given altitude for exponential distribution
fn density_rayleigh(h: f32) -> f32 {
    return exp(-h / params.rayleigh_scale_h);
}

fn density_mie(h: f32) -> f32 {
    return exp(-h / params.mie_scale_h);
}

fn density_ozone(h: f32) -> f32 {
    return max(0.0, 1.0 - abs(h - params.ozone_center_h) / params.ozone_width);
}

// Compute extinction coefficients at altitude h
fn extinction_at(h: f32) -> vec3<f32> {
    let rayleigh = params.rayleigh_scatter * density_rayleigh(h);
    let mie_ext = (params.mie_scatter + params.mie_absorption) * density_mie(h);
    let ozone = params.ozone_absorption * density_ozone(h);
    return rayleigh + vec3<f32>(mie_ext) + ozone;
}

// Compute transmittance from altitude h looking at cos_zenith angle
fn compute_transmittance(h: f32, cos_zenith: f32) -> vec3<f32> {
    let top_r = atmosphere_top(params.planet_radius, params.atmosphere_height);
    let ray_length = ray_sphere_intersect(h, cos_zenith, top_r);

    if (ray_length < 0.0) {
        return vec3<f32>(1.0);
    }

    let step_size = ray_length / f32(NUM_STEPS);
    var optical_depth = vec3<f32>(0.0);

    let r = params.planet_radius + h;

    for (var i = 0u; i < NUM_STEPS; i++) {
        let t = (f32(i) + 0.5) * step_size;
        // Height at sample point along ray
        // Using the law of cosines: r_sample² = r² + t² + 2·r·t·cos_zenith
        let r_sample = sqrt(r * r + t * t + 2.0 * r * t * cos_zenith);
        let h_sample = r_sample - params.planet_radius;

        if (h_sample < 0.0) {
            // Below planet surface — total extinction
            return vec3<f32>(0.0);
        }

        optical_depth += extinction_at(h_sample) * step_size;
    }

    return exp(-optical_depth);
}

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y)
fn transmittance_lut(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.lut_width || gid.y >= params.lut_height) {
        return;
    }

    // Map pixel to (cos_zenith, altitude) parameter space
    let u = (f32(gid.x) + 0.5) / f32(params.lut_width);
    let v = (f32(gid.y) + 0.5) / f32(params.lut_height);

    // Non-linear mapping for better precision near horizon
    // u → cos_zenith: use quadratic mapping for more samples near horizon
    let cos_zenith = 2.0 * u - 1.0;

    // v → altitude: quadratic mapping for more samples near ground
    let h = v * v * params.atmosphere_height;

    let transmittance = compute_transmittance(h, cos_zenith);

    textureStore(t_output, vec2<i32>(gid.xy), vec4<f32>(transmittance, 1.0));
}
