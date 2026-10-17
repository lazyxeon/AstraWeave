// ibl_common.wgsl — shared IBL evaluation (split-sum) for every forward path.
//
// Hoisted VERBATIM from SHADER_SRC's inline `compute_ibl` (L.2, 2026-07-27) so
// the terrain-forward shader can consume the same single implementation
// instead of minting a second one (CLAUDE.md: never build a second
// implementation of a logical system).
//
// CONTRACT: this file is a concatenation fragment, not a standalone module.
// Each consumer must declare, BEFORE or AFTER this fragment in its concat
// (WGSL module-scope declarations are order-independent), the following
// globals with exactly these names (group/binding indices are the consumer's
// choice — static PBR uses group(5), terrain-forward uses group(3)):
//
//   var ibl_specular:   texture_cube<f32>;  // prefiltered, mips = roughness
//   var ibl_irradiance: texture_cube<f32>;
//   var ibl_brdf_lut:   texture_2d<f32>;
//   var ibl_sampler:    sampler;
//   var<uniform> uIbl:  IblParams;
//
// `fresnel_schlick_roughness` comes from brdf_common.wgsl, which every
// consumer already prepends.

struct IblParams {
    ibl_intensity: f32,
    max_spec_lod: f32,
    // Cloud-shadow bake center, world XZ. `compute_ibl` does not read this.
    // Sampling the transmittance map with the live camera would slide the
    // shadow between slice updates; the bake center holds it.
    _pad: vec2<f32>,
};

// IBL uses fresnel_schlick_roughness from brdf_common.wgsl
// All samples use textureSampleLevel (explicit LOD) so this function
// is safe inside non-uniform control flow (e.g., LOD branches).
fn compute_ibl(
    N: vec3<f32>,
    V: vec3<f32>,
    base_color: vec3<f32>,
    metallic: f32,
    roughness: f32,
    F0: vec3<f32>,
) -> vec3<f32> {
    let NdotV = max(dot(N, V), 0.0);
    let F = fresnel_schlick_roughness(NdotV, F0, roughness);

    // Diffuse IBL: irradiance cubemap sampled by normal (pre-convolved, mip 0)
    let kd = (vec3<f32>(1.0) - F) * (1.0 - metallic);
    let irradiance = textureSampleLevel(ibl_irradiance, ibl_sampler, N, 0.0).rgb;
    let diffuse_ibl = kd * base_color * irradiance;

    // Specular IBL: prefiltered environment map + BRDF LUT
    let R = reflect(-V, N);
    let mip = roughness * uIbl.max_spec_lod;
    let prefiltered = textureSampleLevel(ibl_specular, ibl_sampler, R, mip).rgb;
    let brdf = textureSampleLevel(ibl_brdf_lut, ibl_sampler, vec2<f32>(NdotV, roughness), 0.0).rg;
    let specular_ibl = prefiltered * (F * brdf.x + brdf.y);

    return (diffuse_ibl + specular_ibl) * uIbl.ibl_intensity;
}
