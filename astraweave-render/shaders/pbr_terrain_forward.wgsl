// pbr_terrain_forward.wgsl — Forward-lit splat terrain shader (Phase 1).
//
// Part of the Terrain Material System Campaign (Option D). Renders terrain
// chunks with per-fragment 8-biome splat-map blending AND lighting in a
// single pass that writes directly to the engine's forward HDR target
// (Rgba16Float). Unlike the companion `pbr_terrain.wgsl` (deferred-style,
// 3 color targets, unlit), this shader produces a single lit HDR color
// compatible with `astraweave_render::Renderer`'s forward pass.
//
// Reference: `astraweave-render/src/renderer.rs:18-328` (SHADER_SRC) — the
// engine's main PBR shader whose lighting model this file mirrors for the
// terrain pass. `TerrainMaterialManager::set_material` uploads 8 layer
// texture sets into the arrays bound at group(1) bindings 3-5. Per-chunk
// splat textures are bound at group(2).
//
// Phase 1 lighting scope (per TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md §3):
//   - Sun direct lighting: Cook-Torrance + Burley via evaluate_brdf_lod
//   - Ambient fallback: SHADER_SRC's 0.35×ambient_color formula
//   - Distance fog: SHADER_SRC's apply_scene_fog linear+exp blend
//   - NO shadows, IBL, SSGI, cloud shadows, or screen tint (Phase 3 re-adds
//     these once terrain is visibly working). Follows the plan's "plumbing
//     is the gate, not aesthetic" success criterion.
//
// The PI, INV_PI constants and all BRDF helpers are prepended at pipeline
// build time via `concat!(include_str!("constants.wgsl"),
// include_str!("brdf_common.wgsl"), include_str!("pbr_terrain_forward.wgsl"))`.

// ============================================================================
// Uniforms
// ============================================================================

// Camera UBO — mirrors `Camera` in SHADER_SRC (renderer.rs:48-54) byte-for-byte.
struct CameraTerrain {
    view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    _pad0: f32,
    camera_pos: vec3<f32>,
    _pad1: f32,
}

// Per-layer material parameters. Matches `TerrainLayerGpu` in
// `astraweave-render/src/terrain_material.rs:16-36` (64 bytes).
struct TerrainLayer {
    texture_indices: vec4<u32>,     // [albedo, normal, orm, height]
    uv_scale: vec2<f32>,
    height_range: vec2<f32>,
    blend_sharpness: f32,
    triplanar_power: f32,
    material_factors: vec2<f32>,    // [metallic, roughness]
    _pad: vec4<u32>,
}

// Terrain material UBO — mirrors `TerrainMaterialGpu` in
// `astraweave-render/src/terrain_material.rs:54-87` (576 bytes).
// Reused from the deferred pipeline; most fields (triplanar, normal_blend_method,
// height_blend_enabled) are ignored by Phase 1's simplified blending path.
struct TerrainParams {
    layers: array<TerrainLayer, 8>,
    splat_map_index_0: u32,
    splat_map_index_1: u32,
    splat_uv_scale: f32,
    triplanar_enabled: u32,
    normal_blend_method: u32,
    triplanar_slope_threshold: f32,
    height_blend_enabled: u32,
    active_layer_count: u32,
    // `array<u32, 8>` stride is 4 B; WGSL uniform address-space requires >=16 B
    // stride. Matches Rust-side `_pad: [u32; 8]` via two vec4<u32> entries.
    _pad: array<vec4<u32>, 2>,
}

// Scene env UBO — mirrors `SceneEnv` in SHADER_SRC (renderer.rs:86-100)
// **byte-for-byte** per Phase 1.E handoff §5 "Option 1". `tint_color`,
// `tint_alpha`, `blend_factor`, and the `_pad1` triplet are declared here
// even though Phase 1's fragment shader doesn't read them — preserving the
// layout keeps this UBO in sync with SHADER_SRC so future shader revisions
// adding screen tint won't need a silent byte-offset update.
// 96 bytes total, align 16.
struct TerrainSceneEnv {
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_start: f32,
    fog_end: f32,
    _pad0: vec2<f32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    tint_color: vec3<f32>,
    tint_alpha: f32,
    blend_factor: f32,
    _pad1x: f32,
    _pad1y: f32,
    _pad1z: f32,
    sun_color: vec3<f32>,
    sun_intensity: f32,
}

@group(0) @binding(0) var<uniform> uCamera: CameraTerrain;

@group(1) @binding(0) var<uniform> uTerrain: TerrainParams;
@group(1) @binding(1) var<uniform> uScene: TerrainSceneEnv;
@group(1) @binding(2) var terrain_sampler: sampler;
@group(1) @binding(3) var layer_albedo: texture_2d_array<f32>;
@group(1) @binding(4) var layer_normal: texture_2d_array<f32>;
@group(1) @binding(5) var layer_orm:    texture_2d_array<f32>;

@group(2) @binding(0) var splat_map_0: texture_2d<f32>;
@group(2) @binding(1) var splat_map_1: texture_2d<f32>;

// ============================================================================
// Vertex stage
// ============================================================================

struct VSIn {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
}

struct VSOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos:  vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VSIn) -> VSOut {
    var out: VSOut;
    out.clip_pos    = uCamera.view_proj * vec4<f32>(in.position, 1.0);
    out.world_pos   = in.position;
    out.world_normal = in.normal;
    out.uv          = in.uv;
    return out;
}

// ============================================================================
// Helpers — fog (copied from SHADER_SRC to stay in sync with apply_scene_fog)
// ============================================================================

fn apply_terrain_fog(color: vec3<f32>, dist: f32) -> vec3<f32> {
    let linear_fog = clamp(
        (dist - uScene.fog_start) / max(uScene.fog_end - uScene.fog_start, 0.001),
        0.0, 1.0,
    );
    let exp_fog = 1.0 - exp(-uScene.fog_density * dist);
    // Cap at 0.92 to avoid the white-void horizon effect (mirrors SHADER_SRC).
    let fog_factor = clamp(max(linear_fog, exp_fog), 0.0, 0.92);
    return mix(color, uScene.fog_color, fog_factor);
}

// ============================================================================
// Fragment stage
// ============================================================================

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    // 1. Sample both splat textures at the fragment UV.
    let splat_uv = in.uv * uTerrain.splat_uv_scale;
    let splat0 = textureSample(splat_map_0, terrain_sampler, splat_uv);
    let splat1 = textureSample(splat_map_1, terrain_sampler, splat_uv);

    var raw_weights: array<f32, 8>;
    raw_weights[0] = splat0.r;
    raw_weights[1] = splat0.g;
    raw_weights[2] = splat0.b;
    raw_weights[3] = splat0.a;
    raw_weights[4] = splat1.r;
    raw_weights[5] = splat1.g;
    raw_weights[6] = splat1.b;
    raw_weights[7] = splat1.a;

    // 2. Normalize weights over the active layer count.
    let count = uTerrain.active_layer_count;
    var total_w: f32 = 0.0;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        total_w = total_w + raw_weights[i];
    }
    if (total_w > 0.0001) {
        for (var i: u32 = 0u; i < count; i = i + 1u) {
            raw_weights[i] = raw_weights[i] / total_w;
        }
    } else {
        // Fallback: if no biome authored at this fragment, pin to layer 0.
        raw_weights[0] = 1.0;
    }

    // 3. Pre-compute screen-space derivatives in uniform control flow so
    //    textureSampleGrad inside the per-layer loop is well-defined on
    //    every backend (FXC/DXC don't unroll gradient-dependent dynamic
    //    loops cleanly; see the pattern in pbr_terrain.wgsl).
    let ddx_uv = dpdx(in.uv);
    let ddy_uv = dpdy(in.uv);

    // 4. Accumulate PBR inputs from each contributing layer using planar
    //    projection. Phase 1 skips triplanar — terrain is generally close
    //    to horizontal in the editor's test scenes, and triplanar adds
    //    ~30 ALU per fragment for marginal gain at this stage.
    var final_albedo = vec3<f32>(0.0);
    var final_normal_ts = vec3<f32>(0.0, 0.0, 0.0);
    var final_ao: f32 = 0.0;
    var final_roughness: f32 = 0.0;
    var final_metallic: f32 = 0.0;

    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let w = raw_weights[i];
        if (w < 0.001) {
            continue;
        }

        let layer = uTerrain.layers[i];
        let a_idx = layer.texture_indices.x;
        let n_idx = layer.texture_indices.y;
        let o_idx = layer.texture_indices.z;
        let scaled_uv = in.uv * layer.uv_scale;
        let scaled_ddx = ddx_uv * layer.uv_scale;
        let scaled_ddy = ddy_uv * layer.uv_scale;

        let albedo_s = textureSampleGrad(
            layer_albedo, terrain_sampler,
            scaled_uv, a_idx, scaled_ddx, scaled_ddy,
        );
        let normal_s = textureSampleGrad(
            layer_normal, terrain_sampler,
            scaled_uv, n_idx, scaled_ddx, scaled_ddy,
        );
        let orm_s = textureSampleGrad(
            layer_orm, terrain_sampler,
            scaled_uv, o_idx, scaled_ddx, scaled_ddy,
        );

        final_albedo = final_albedo + albedo_s.rgb * w;

        // Tangent-space normal: linear-blend XY, keep Z coherent.
        // Phase 1 uses this simple UDN-style blend; full RNM is a Phase 3
        // setting (§2.6 "Normal blend formulas" in the campaign plan).
        let n_ts = normal_s.rgb * 2.0 - 1.0;
        final_normal_ts = final_normal_ts + vec3<f32>(n_ts.xy * w, n_ts.z * w);

        // ORM: R=AO, G=Roughness, B=Metallic (standard packing).
        final_ao = final_ao + orm_s.r * w;
        final_roughness = final_roughness + (orm_s.g * layer.material_factors.y) * w;
        final_metallic = final_metallic + (orm_s.b * layer.material_factors.x) * w;
    }

    // Assemble tangent-space normal. The world-space surface normal is
    // in.world_normal; for a terrain mesh we construct a TBN using the
    // geometric normal + arbitrary tangent (the XZ plane's +X direction),
    // then project the accumulated tangent-space perturbation into world.
    let N_geom = normalize(in.world_normal);
    // Derive tangent as the horizontal direction perpendicular to N_geom.
    // For a near-vertical normal (y ≈ 1), this picks +X; for steep slopes
    // it remains well-defined via the cross-product.
    var T_world: vec3<f32>;
    if (abs(N_geom.y) < 0.99) {
        T_world = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), N_geom));
    } else {
        T_world = vec3<f32>(1.0, 0.0, 0.0);
    }
    let B_world = normalize(cross(N_geom, T_world));
    let n_ts_norm = normalize(final_normal_ts + vec3<f32>(0.0, 0.0, 0.001));
    let N = normalize(T_world * n_ts_norm.x + B_world * n_ts_norm.y + N_geom * n_ts_norm.z);

    let V = normalize(uCamera.camera_pos - in.world_pos);
    let L = normalize(-uCamera.light_dir);
    let base_color = final_albedo;
    // Clamp to the range SHADER_SRC uses (metallic full, roughness >= 0.04
    // to avoid singular GGX at perfect mirrors).
    let metallic = clamp(final_metallic, 0.0, 1.0);
    let roughness = clamp(final_roughness, 0.04, 1.0);
    let F0 = mix(vec3<f32>(0.04), base_color, metallic);

    // 5. LOD-aware BRDF (Cook-Torrance + Burley) — same helper SHADER_SRC uses.
    let mat_lod = compute_material_lod(in.world_pos);
    let brdf_result = evaluate_brdf_lod(
        N, V, L, base_color, metallic, roughness, F0, mat_lod,
    );

    // 6. Direct sun lighting (no shadow cascade in Phase 1).
    let radiance = uScene.sun_color * uScene.sun_intensity;
    var lit_color = brdf_result * radiance;

    // 7. Ambient fallback — mirrors SHADER_SRC's 0.35× ambient fill (line 314).
    let ambient = uScene.ambient_color * uScene.ambient_intensity * 0.35;
    lit_color = lit_color + base_color * ambient * final_ao;

    // 8. Distance fog — matches SHADER_SRC's formula.
    let frag_dist = length(in.world_pos - uCamera.camera_pos);
    lit_color = apply_terrain_fog(lit_color, frag_dist);

    return vec4<f32>(lit_color, 1.0);
}
