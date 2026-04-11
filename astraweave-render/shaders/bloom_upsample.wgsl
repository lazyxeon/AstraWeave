// Bloom upsample pass (tent filter for smooth energy distribution)
//
// Uses a 9-tap tent filter to upsample and blend the lower mip with the
// current mip, progressively building up the bloom contribution.

struct BloomUpsampleParams {
    inv_resolution: vec2<f32>,  // 1/dst_width, 1/dst_height
    intensity: f32,              // Per-mip intensity multiplier
    _pad: f32,
};

@group(0) @binding(0) var lower_mip: texture_2d<f32>;   // Lower resolution bloom mip
@group(0) @binding(1) var current_mip: texture_2d<f32>;  // Current resolution (accumulator)
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var<uniform> params: BloomUpsampleParams;
@group(0) @binding(4) var dst_tex: texture_storage_2d<rgba16float, write>;

override WG_X: u32 = 8u;
override WG_Y: u32 = 8u;

@compute @workgroup_size(WG_X, WG_Y, 1)
fn bloom_upsample(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dst_dims = textureDimensions(dst_tex);
    if (gid.x >= dst_dims.x || gid.y >= dst_dims.y) {
        return;
    }

    let uv = (vec2<f32>(gid.xy) + 0.5) / vec2<f32>(dst_dims);
    let d = params.inv_resolution;

    // 9-tap tent filter on the lower mip
    var bloom = vec3<f32>(0.0);
    bloom += textureSampleLevel(lower_mip, samp, uv + vec2<f32>(-d.x, -d.y), 0.0).rgb * 1.0;
    bloom += textureSampleLevel(lower_mip, samp, uv + vec2<f32>( 0.0, -d.y), 0.0).rgb * 2.0;
    bloom += textureSampleLevel(lower_mip, samp, uv + vec2<f32>( d.x, -d.y), 0.0).rgb * 1.0;
    bloom += textureSampleLevel(lower_mip, samp, uv + vec2<f32>(-d.x,  0.0), 0.0).rgb * 2.0;
    bloom += textureSampleLevel(lower_mip, samp, uv,                         0.0).rgb * 4.0;
    bloom += textureSampleLevel(lower_mip, samp, uv + vec2<f32>( d.x,  0.0), 0.0).rgb * 2.0;
    bloom += textureSampleLevel(lower_mip, samp, uv + vec2<f32>(-d.x,  d.y), 0.0).rgb * 1.0;
    bloom += textureSampleLevel(lower_mip, samp, uv + vec2<f32>( 0.0,  d.y), 0.0).rgb * 2.0;
    bloom += textureSampleLevel(lower_mip, samp, uv + vec2<f32>( d.x,  d.y), 0.0).rgb * 1.0;
    bloom /= 16.0;

    // Additive blend with current mip (energy-conserving)
    let current = textureSampleLevel(current_mip, samp, uv, 0.0).rgb;
    let result = current + bloom * params.intensity;

    textureStore(dst_tex, vec2<i32>(gid.xy), vec4<f32>(result, 1.0));
}
