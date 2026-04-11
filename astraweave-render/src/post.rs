// Post-processing WGSL placeholder modules and compile-only tests.
// These shaders are not wired into runtime yet; we just ensure they parse via naga.
//
// Production bloom is in `bloom.rs` (compute-based BloomPass).

// ---------------------------------------------------------------------------------
// Legacy placeholder shaders (SSR, SSAO, SSGI)
// ---------------------------------------------------------------------------------

pub const WGSL_SSR: &str = r#"
// Screen-space reflections placeholder
struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    out.uv = (pos[vid] + vec2<f32>(1.0,1.0)) * 0.5;
    return out;
}

@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var samp: sampler;

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let col = textureSampleLevel(color_tex, samp, in.uv, 0.0);
    let _d = textureLoad(depth_tex, vec2<i32>(i32(in.uv.x), i32(in.uv.y)), 0);
    // Placeholder: just passthrough
    return vec4<f32>(col.rgb, 1.0);
}
"#;

pub const WGSL_SSAO: &str = r#"
// Screen-space ambient occlusion placeholder
struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    out.uv = (pos[vid] + vec2<f32>(1.0,1.0)) * 0.5;
    return out;
}

@group(0) @binding(0) var depth_tex: texture_depth_2d;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // Placeholder: flat gray AO factor
    let ao = 0.2;
    return vec4<f32>(ao, ao, ao, 1.0);
}
"#;

pub const WGSL_SSGI: &str = r#"
// Screen-space global illumination placeholder
struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    out.uv = (pos[vid] + vec2<f32>(1.0,1.0)) * 0.5;
    return out;
}

@group(0) @binding(0) var normal_tex: texture_2d<f32>;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var samp: sampler;

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // Placeholder: tint by normals if provided; else white
    let nrm = textureSampleLevel(normal_tex, samp, in.uv, 0.0).xyz;
    return vec4<f32>(normalize(nrm), 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ssr() {
        let src = WGSL_SSR;
        let module = naga::front::wgsl::parse_str(src).expect("WGSL SSR should parse");
        assert!(module.entry_points.iter().any(|e| e.name == "fs_main"));
    }
    #[test]
    fn parse_ssao() {
        let src = WGSL_SSAO;
        let module = naga::front::wgsl::parse_str(src).expect("WGSL SSAO should parse");
        assert!(module.entry_points.iter().any(|e| e.name == "fs_main"));
    }
    #[test]
    fn parse_ssgi() {
        let src = WGSL_SSGI;
        let module = naga::front::wgsl::parse_str(src).expect("WGSL SSGI should parse");
        assert!(module.entry_points.iter().any(|e| e.name == "fs_main"));
    }

    #[test]
    fn ssgi_placeholder_has_expected_bindings() {
        assert!(WGSL_SSGI.contains("@group(0) @binding(0) var normal_tex"));
        assert!(WGSL_SSGI.contains("@group(0) @binding(1) var depth_tex"));
        assert!(WGSL_SSGI.contains("@group(0) @binding(2) var samp"));
    }
}
