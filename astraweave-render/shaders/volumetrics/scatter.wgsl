// Volumetric Light Scattering — In-scatter computation per froxel
//
// For each froxel, compute the in-scattered radiance from:
//   1. Directional light (sun) with CSM shadow sampling
//   2. Ambient sky contribution
//
// Uses Henyey-Greenstein phase function for anisotropic scattering.
// Output: RGBA16Float where RGB = in-scattered light, A = transmittance.

struct ScatterParams {
    inv_view_proj:   mat4x4<f32>,
    view_pos:        vec3<f32>,
    near_plane:      f32,
    far_plane:       f32,
    froxel_dims:     vec3<u32>,
    // Phase function
    anisotropy:      f32,      // g: [-1, 1], 0 = isotropic, positive = forward scatter
    // Sun
    sun_dir:         vec3<f32>,
    sun_intensity:   f32,
    sun_color:       vec3<f32>,
    // Ambient
    ambient_intensity: f32,
    ambient_color:   vec3<f32>,
    // Shadow
    temporal_blend:  f32,
    frame_index:     u32,
    _pad0:           f32,
    _pad1:           f32,
    _pad2:           f32,
};

struct CascadeData {
    vp:     mat4x4<f32>,
    split:  f32,
    _pad:   vec3<f32>,
};

@group(0) @binding(0) var<uniform>         params:      ScatterParams;
@group(0) @binding(1) var                  t_density:   texture_3d<f32>;    // fog density froxel
@group(0) @binding(2) var                  t_shadow:    texture_depth_2d;   // CSM shadow atlas
@group(0) @binding(3) var<storage, read>   cascades:    array<CascadeData>; // 4 cascades
@group(0) @binding(4) var                  s_linear:    sampler;
@group(0) @binding(5) var                  s_shadow:    sampler_comparison;
@group(0) @binding(6) var                  t_history:   texture_3d<f32>;    // previous frame scatter
@group(0) @binding(7) var                  t_output:    texture_storage_3d<rgba16float, write>;

// ---- Henyey-Greenstein phase function ----
fn phase_hg(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    // (1 - g^2) / (4π * (1 + g^2 - 2g·cosθ)^1.5)
    return (1.0 - g2) / (4.0 * 3.14159265 * pow(denom, 1.5));
}

// ---- Froxel → world position (same exponential distribution as density pass) ----
fn froxel_to_world(froxel: vec3<u32>) -> vec3<f32> {
    let dims = vec3<f32>(params.froxel_dims);
    let uvw = (vec3<f32>(froxel) + 0.5) / dims;
    let screen_uv = uvw.xy;
    let z_near = params.near_plane;
    let z_far = params.far_plane;
    let depth = z_near * pow(z_far / z_near, uvw.z);
    let ndc_xy = screen_uv * 2.0 - 1.0;
    let ndc_z = (z_far * (depth - z_near)) / (depth * (z_far - z_near));
    let clip = vec4<f32>(ndc_xy, ndc_z, 1.0);
    let world_h = params.inv_view_proj * clip;
    return world_h.xyz / world_h.w;
}

// ---- Shadow sampling (simplified PCF on cascade 0 or nearest cascade) ----
fn sample_shadow(world_pos: vec3<f32>) -> f32 {
    let view_dist = length(world_pos - params.view_pos);

    // Find appropriate cascade
    var cascade_idx = 0u;
    for (var i = 0u; i < 4u; i++) {
        if (view_dist > cascades[i].split) {
            cascade_idx = i + 1u;
        }
    }
    cascade_idx = min(cascade_idx, 3u);

    let shadow_clip = cascades[cascade_idx].vp * vec4<f32>(world_pos, 1.0);
    let shadow_ndc = shadow_clip.xyz / shadow_clip.w;
    let shadow_uv = shadow_ndc.xy * 0.5 + 0.5;

    // Out-of-bounds = fully lit
    if (any(shadow_uv < vec2<f32>(0.0)) || any(shadow_uv > vec2<f32>(1.0))) {
        return 1.0;
    }

    let compare_depth = shadow_ndc.z;

    // Single comparison sample for volumetric (performance-critical)
    // Use textureSampleCompareLevel (explicit LOD 0) as textureSampleCompare is fragment-only
    return textureSampleCompareLevel(t_shadow, s_shadow, shadow_uv, compare_depth);
}

// ---- Temporal jitter (Halton-like offset for temporal stability) ----
fn temporal_offset(frame: u32) -> vec3<f32> {
    let f = f32(frame % 8u);
    return vec3<f32>(
        fract(f * 0.618034),   // golden ratio fraction
        fract(f * 0.324919),
        fract(f * 0.220765),
    ) * 0.5 - 0.25; // [-0.25, 0.25] jitter
}

@compute @workgroup_size(4, 4, 4)
fn scatter_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = params.froxel_dims;
    if (gid.x >= dims.x || gid.y >= dims.y || gid.z >= dims.z) {
        return;
    }

    let world_pos = froxel_to_world(gid);

    // Read density from density volume
    let density_uvw = (vec3<f32>(gid) + 0.5) / vec3<f32>(dims);
    let density = textureSampleLevel(t_density, s_linear, density_uvw, 0.0).r;

    // Skip near-zero density froxels (optimization)
    if (density < 0.0001) {
        textureStore(t_output, vec3<i32>(gid), vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }

    // View direction for phase function
    let view_dir = normalize(world_pos - params.view_pos);
    let cos_theta = dot(view_dir, params.sun_dir);
    let phase = phase_hg(cos_theta, params.anisotropy);

    // Shadow visibility
    let shadow = sample_shadow(world_pos);

    // In-scattered light from sun
    let sun_scatter = params.sun_color * params.sun_intensity * phase * shadow * density;

    // Ambient in-scattering (isotropic, no phase needed)
    let ambient_scatter = params.ambient_color * params.ambient_intensity * density * 0.25;

    let total_scatter = sun_scatter + ambient_scatter;

    // Temporal reprojection: blend with history
    let history = textureSampleLevel(t_history, s_linear, density_uvw, 0.0);
    let blended = mix(vec4<f32>(total_scatter, density), history, params.temporal_blend);

    textureStore(t_output, vec3<i32>(gid), blended);
}
