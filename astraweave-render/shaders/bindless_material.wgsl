// Bindless Material System
//
// Uses TEXTURE_BINDING_ARRAY to index an array of individual texture_2d resources.
// Each material stores texture indices into that array, plus PBR factors.
// A single storage buffer holds all MaterialEntry structs.

// ─── GPU Material Entry ───

struct MaterialEntry {
    albedo_index:       u32,
    normal_index:       u32,
    orm_index:          u32,
    emissive_index:     u32,
    base_color:         vec4<f32>,
    metallic_factor:    f32,
    roughness_factor:   f32,
    occlusion_factor:   f32,
    emissive_strength:  f32,
    uv_scale:           vec2<f32>,
    flags:              u32,
    alpha_cutoff:       f32,
}

// ─── Sampled Result ───

struct MaterialSample {
    albedo:    vec4<f32>,
    normal:    vec3<f32>,
    metallic:  f32,
    roughness: f32,
    occlusion: f32,
    emissive:  vec3<f32>,
}

// ─── Bindings ───

@group(0) @binding(0) var textures: binding_array<texture_2d<f32>>;
@group(0) @binding(1) var material_sampler: sampler;
@group(0) @binding(2) var<storage, read> materials: array<MaterialEntry>;

// ─── Flag Constants ───

const MAT_FLAG_HAS_ALBEDO:   u32 = 1u;
const MAT_FLAG_HAS_NORMAL:   u32 = 2u;
const MAT_FLAG_HAS_ORM:      u32 = 4u;
const MAT_FLAG_HAS_EMISSIVE: u32 = 8u;

// ─── Material Sampling ───

fn sample_material(material_id: u32, uv: vec2<f32>) -> MaterialSample {
    let mat = materials[material_id];
    let scaled_uv = uv * mat.uv_scale;

    var result: MaterialSample;

    // Albedo
    if ((mat.flags & MAT_FLAG_HAS_ALBEDO) != 0u) {
        result.albedo = textureSample(textures[mat.albedo_index], material_sampler, scaled_uv) * mat.base_color;
    } else {
        result.albedo = mat.base_color;
    }

    // Normal (tangent space) — decode [0,1]→[-1,1] and normalize to avoid
    // denormalized normals from texture filtering/compression.
    if ((mat.flags & MAT_FLAG_HAS_NORMAL) != 0u) {
        let n = textureSample(textures[mat.normal_index], material_sampler, scaled_uv).xyz;
        result.normal = normalize(n * 2.0 - 1.0);
    } else {
        result.normal = vec3<f32>(0.0, 0.0, 1.0);
    }

    // ORM (Occlusion, Roughness, Metallic packed)
    if ((mat.flags & MAT_FLAG_HAS_ORM) != 0u) {
        let orm = textureSample(textures[mat.orm_index], material_sampler, scaled_uv);
        result.occlusion = orm.r * mat.occlusion_factor;
        result.roughness = orm.g * mat.roughness_factor;
        result.metallic  = orm.b * mat.metallic_factor;
    } else {
        result.occlusion = mat.occlusion_factor;
        result.roughness = mat.roughness_factor;
        result.metallic  = mat.metallic_factor;
    }

    // Emissive
    if ((mat.flags & MAT_FLAG_HAS_EMISSIVE) != 0u) {
        result.emissive = textureSample(textures[mat.emissive_index], material_sampler, scaled_uv).rgb * mat.emissive_strength;
    } else {
        result.emissive = vec3<f32>(0.0);
    }

    return result;
}
