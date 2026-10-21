// Screen UV for this Y-up view. uv.y 0 is the top of the texture, NDC y +1.
// pbr.wgsl uses the same pair: ndc.y = 1 - 2*uv.y, uv.y = 0.5 - 0.5*ndc.y.

fn screen_uv_to_ndc(uv: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
}

fn ndc_to_screen_uv(ndc: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
}
