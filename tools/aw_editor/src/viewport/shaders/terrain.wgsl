// ============================================================================
// AstraWeave Procedural PBR Terrain Shader
// ============================================================================
// Generates realistic terrain textures via procedural noise, triplanar mapping,
// slope/height blending, and Cook-Torrance BRDF.

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    shading_mode: u32,
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_enabled: u32,
    weather_type: u32,
    time: f32,
    water_level: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) biome_id: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) biome_id: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// ─── Noise Functions ──────────────────────────────────────────────────────────

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let p4 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract(vec2<f32>((p4.x + p4.y) * p4.z, (p4.x + p4.z) * p4.y));
}

fn hash31(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3.z);
}

fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(
            mix(hash31(i + vec3<f32>(0.0, 0.0, 0.0)), hash31(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash31(i + vec3<f32>(0.0, 1.0, 0.0)), hash31(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash31(i + vec3<f32>(0.0, 0.0, 1.0)), hash31(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash31(i + vec3<f32>(0.0, 1.0, 1.0)), hash31(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

fn fbm2d(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    let rot = mat2x2<f32>(0.8, 0.6, -0.6, 0.8);
    for (var i = 0; i < 5; i++) {
        value += amplitude * noise2d(pos);
        pos = rot * pos * 2.0;
        amplitude *= 0.5;
    }
    return value;
}

fn fbm3d(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < 4; i++) {
        value += amplitude * noise3d(pos);
        pos = pos * 2.03;
        amplitude *= 0.5;
    }
    return value;
}

fn voronoi2d(p: vec2<f32>) -> vec2<f32> {
    let n = floor(p);
    let f = fract(p);
    var md = 8.0;
    var md2 = 8.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let g = vec2<f32>(f32(i), f32(j));
            let o = hash22(n + g);
            let r = g + o - f;
            let d = dot(r, r);
            if d < md {
                md2 = md;
                md = d;
            } else if d < md2 {
                md2 = d;
            }
        }
    }
    return vec2<f32>(sqrt(md), sqrt(md2));
}

fn warped_fbm(p: vec2<f32>) -> f32 {
    let q = vec2<f32>(fbm2d(p), fbm2d(p + vec2<f32>(5.2, 1.3)));
    return fbm2d(p + 4.0 * q);
}

// ─── Triplanar Mapping ────────────────────────────────────────────────────────

fn triplanar_weights(normal: vec3<f32>) -> vec3<f32> {
    var w = abs(normal);
    w = pow(w, vec3<f32>(4.0));
    return w / (w.x + w.y + w.z + 0.0001);
}

fn triplanar_noise(pos: vec3<f32>, normal: vec3<f32>, scale: f32) -> f32 {
    let w = triplanar_weights(normal);
    let xy = noise2d(pos.xy * scale);
    let xz = noise2d(pos.xz * scale);
    let yz = noise2d(pos.yz * scale);
    return xy * w.z + xz * w.y + yz * w.x;
}

fn triplanar_fbm(pos: vec3<f32>, normal: vec3<f32>, scale: f32) -> f32 {
    let w = triplanar_weights(normal);
    let xy = fbm2d(pos.xy * scale);
    let xz = fbm2d(pos.xz * scale);
    let yz = fbm2d(pos.yz * scale);
    return xy * w.z + xz * w.y + yz * w.x;
}

// ─── Material ─────────────────────────────────────────────────────────────────

struct Material {
    albedo: vec3<f32>,
    roughness: f32,
    metallic: f32,
    ao: f32,
}

fn material_grassland(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let base = triplanar_fbm(pos, n, 0.8);
    let detail = triplanar_noise(pos, n, 3.0);
    let clump = triplanar_fbm(pos, n, 0.15);
    let green1 = vec3<f32>(0.04, 0.12, 0.01);
    let green2 = vec3<f32>(0.08, 0.18, 0.02);
    let green3 = vec3<f32>(0.06, 0.13, 0.005);
    let dirt = vec3<f32>(0.06, 0.03, 0.01);
    var color = mix(green1, green2, base);
    color = mix(color, green3, detail * 0.4);
    let dirt_f = smoothstep(0.25, 0.35, 1.0 - clump);
    color = mix(color, dirt, dirt_f * 0.6);
    let micro = triplanar_noise(pos, n, 12.0);
    color *= 0.85 + micro * 0.3;
    var mat: Material;
    mat.albedo = color;
    mat.roughness = 0.75 + detail * 0.2;
    mat.metallic = 0.0;
    mat.ao = 0.7 + base * 0.3;
    return mat;
}

fn material_desert(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let base = triplanar_fbm(pos, n, 0.3);
    let ripple = triplanar_noise(pos, n, 6.0);
    let large = triplanar_fbm(pos, n, 0.05);
    let sand1 = vec3<f32>(0.55, 0.32, 0.12);
    let sand2 = vec3<f32>(0.65, 0.42, 0.15);
    let rsand = vec3<f32>(0.48, 0.15, 0.04);
    let rock = vec3<f32>(0.18, 0.13, 0.08);
    var color = mix(sand1, sand2, base);
    color = mix(color, rsand, large * 0.4);
    color += vec3<f32>(0.02) * ripple;
    let slope = 1.0 - max(dot(n, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
    color = mix(color, rock, smoothstep(0.3, 0.6, slope));
    let micro = triplanar_noise(pos, n, 15.0);
    color *= 0.9 + micro * 0.2;
    var mat: Material;
    mat.albedo = color;
    mat.roughness = 0.55 + base * 0.25;
    mat.metallic = 0.0;
    mat.ao = 0.8 + base * 0.2;
    return mat;
}

fn material_forest(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let base = triplanar_fbm(pos, n, 0.5);
    let leaf = triplanar_noise(pos, n, 4.0);
    let moss = triplanar_fbm(pos, n, 1.5);
    let earth = vec3<f32>(0.02, 0.015, 0.005);
    let lg = vec3<f32>(0.01, 0.05, 0.005);
    let lb = vec3<f32>(0.08, 0.03, 0.005);
    let mg = vec3<f32>(0.015, 0.07, 0.01);
    var color = mix(earth, lg, base * 0.7);
    color = mix(color, lb, leaf * 0.35);
    let flat_f = max(dot(n, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
    color = mix(color, mg, moss * flat_f * 0.5);
    let micro = triplanar_noise(pos, n, 10.0);
    color *= 0.80 + micro * 0.4;
    var mat: Material;
    mat.albedo = color;
    mat.roughness = 0.85 + leaf * 0.1;
    mat.metallic = 0.0;
    mat.ao = 0.5 + base * 0.3;
    return mat;
}

fn material_mountain(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let strata = triplanar_fbm(pos, n, 0.4);
    let crack = voronoi2d(pos.xz * 2.0);
    let detail = triplanar_noise(pos, n, 5.0);
    let rock1 = vec3<f32>(0.12, 0.10, 0.09);
    let rock2 = vec3<f32>(0.22, 0.18, 0.14);
    let dark = vec3<f32>(0.04, 0.035, 0.03);
    var color = mix(rock1, rock2, strata);
    let crack_f = smoothstep(0.02, 0.08, crack.x);
    color = mix(dark, color, crack_f);
    let strata_line = sin(pos.y * 3.0 + strata * 4.0) * 0.5 + 0.5;
    color = mix(color, color * 0.8, smoothstep(0.4, 0.5, strata_line) * 0.3);
    let micro = triplanar_noise(pos, n, 8.0);
    color *= 0.85 + micro * 0.3;
    let height_snow = smoothstep(30.0, 50.0, pos.y);
    let flat_f = max(dot(n, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
    let snow_f = height_snow * smoothstep(0.5, 0.85, flat_f);
    color = mix(color, vec3<f32>(0.85, 0.87, 0.90), snow_f);
    var mat: Material;
    mat.albedo = color;
    mat.roughness = mix(0.65 + detail * 0.2, 0.3, snow_f);
    mat.metallic = 0.0;
    mat.ao = 0.6 + crack_f * 0.3;
    return mat;
}

fn material_tundra(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let base = triplanar_fbm(pos, n, 0.6);
    let ice = voronoi2d(pos.xz * 3.0);
    let frost = triplanar_noise(pos, n, 8.0);
    let snow = vec3<f32>(0.75, 0.78, 0.85);
    let ice_blue = vec3<f32>(0.38, 0.55, 0.72);
    let frozen = vec3<f32>(0.10, 0.09, 0.06);
    var color = mix(snow, frozen, base * 0.3);
    let ice_f = smoothstep(0.15, 0.25, ice.x);
    color = mix(ice_blue, color, ice_f);
    color += vec3<f32>(0.03) * frost;
    var mat: Material;
    mat.albedo = color;
    mat.roughness = mix(0.15, 0.80, ice_f);
    mat.metallic = mix(0.1, 0.0, ice_f);
    mat.ao = 0.85 + base * 0.15;
    return mat;
}

fn material_swamp(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let base = triplanar_fbm(pos, n, 0.4);
    let mud = warped_fbm(pos.xz * 0.5);
    let algae = triplanar_noise(pos, n, 2.0);
    let mud_brown = vec3<f32>(0.045, 0.025, 0.005);
    let swamp_green = vec3<f32>(0.015, 0.04, 0.005);
    let dark_water = vec3<f32>(0.005, 0.015, 0.005);
    let algae_green = vec3<f32>(0.03, 0.08, 0.005);
    var color = mix(mud_brown, swamp_green, base);
    let water_patch = smoothstep(0.4, 0.5, mud);
    color = mix(color, dark_water, water_patch * 0.7);
    color = mix(color, algae_green, algae * water_patch * 0.5);
    let micro = triplanar_noise(pos, n, 6.0);
    color *= 0.8 + micro * 0.4;
    var mat: Material;
    mat.albedo = color;
    mat.roughness = mix(0.9, 0.15, water_patch);
    mat.metallic = water_patch * 0.05;
    mat.ao = 0.5 + base * 0.3;
    return mat;
}

fn material_beach(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let base = triplanar_fbm(pos, n, 0.8);
    let shell = voronoi2d(pos.xz * 8.0);
    let wave_mark = sin(pos.x * 5.0 + pos.z * 0.5 + noise2d(pos.xz * 2.0) * 3.0);
    let dry = vec3<f32>(0.65, 0.50, 0.30);
    let wet = vec3<f32>(0.28, 0.21, 0.10);
    let pebble = vec3<f32>(0.14, 0.12, 0.10);
    let wet_f = smoothstep(2.0, -1.0, pos.y - uniforms.water_level);
    var color = mix(dry, wet, wet_f);
    color = mix(color, color * 1.1, base * 0.3);
    let shell_f = smoothstep(0.12, 0.08, shell.x);
    color = mix(color, pebble, shell_f * 0.5);
    let mark = smoothstep(0.3, 0.5, wave_mark) * wet_f;
    color = mix(color, color * 0.9, mark * 0.3);
    let micro = triplanar_noise(pos, n, 20.0);
    color *= 0.9 + micro * 0.2;
    var mat: Material;
    mat.albedo = color;
    mat.roughness = mix(0.7, 0.3, wet_f);
    mat.metallic = 0.0;
    mat.ao = 0.9;
    return mat;
}

fn material_river(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let stones = voronoi2d(pos.xz * 5.0);
    let base = triplanar_fbm(pos, n, 1.0);
    let wet_rock = vec3<f32>(0.08, 0.07, 0.05);
    let sand = vec3<f32>(0.22, 0.16, 0.08);
    let dark = vec3<f32>(0.02, 0.03, 0.02);
    var color = mix(sand, wet_rock, base);
    let stone_f = smoothstep(0.1, 0.15, stones.x);
    color = mix(dark, color, stone_f);
    let depth = max(0.0, uniforms.water_level - pos.y);
    let underwater_tint = vec3<f32>(0.01, 0.06, 0.08);
    color = mix(color, underwater_tint, smoothstep(0.0, 5.0, depth) * 0.5);
    var mat: Material;
    mat.albedo = color;
    mat.roughness = 0.25;
    mat.metallic = 0.02;
    mat.ao = 0.6 + stone_f * 0.3;
    return mat;
}

fn rock_material(pos: vec3<f32>, n: vec3<f32>) -> Material {
    let strata = triplanar_fbm(pos, n, 0.5);
    let crack = voronoi2d(pos.xz * 3.0);
    let detail = triplanar_noise(pos, n, 4.0);
    let rock1 = vec3<f32>(0.15, 0.12, 0.10);
    let rock2 = vec3<f32>(0.27, 0.22, 0.18);
    let dark = vec3<f32>(0.05, 0.04, 0.035);
    var color = mix(rock1, rock2, strata);
    let crack_f = smoothstep(0.03, 0.1, crack.x);
    color = mix(dark, color, crack_f);
    let micro = triplanar_noise(pos, n, 10.0);
    color *= 0.85 + micro * 0.3;
    var mat: Material;
    mat.albedo = color;
    mat.roughness = 0.6 + detail * 0.2;
    mat.metallic = 0.0;
    mat.ao = 0.5 + crack_f * 0.4;
    return mat;
}

fn get_biome_material(biome_id: u32, pos: vec3<f32>, n: vec3<f32>) -> Material {
    switch biome_id {
        case 0u: { return material_grassland(pos, n); }
        case 1u: { return material_desert(pos, n); }
        case 2u: { return material_forest(pos, n); }
        case 3u: { return material_mountain(pos, n); }
        case 4u: { return material_tundra(pos, n); }
        case 5u: { return material_swamp(pos, n); }
        case 6u: { return material_beach(pos, n); }
        case 7u: { return material_river(pos, n); }
        default: { return material_grassland(pos, n); }
    }
}

fn apply_slope_blend(biome_mat: Material, pos: vec3<f32>, n: vec3<f32>) -> Material {
    let slope = 1.0 - max(dot(n, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
    let rock_blend = smoothstep(0.5, 0.8, slope);
    if rock_blend < 0.01 {
        return biome_mat;
    }
    let rock = rock_material(pos, n);
    var result: Material;
    result.albedo = mix(biome_mat.albedo, rock.albedo, rock_blend);
    result.roughness = mix(biome_mat.roughness, rock.roughness, rock_blend);
    result.metallic = mix(biome_mat.metallic, rock.metallic, rock_blend);
    result.ao = mix(biome_mat.ao, rock.ao, rock_blend);
    return result;
}

// ─── PBR Lighting (Cook-Torrance BRDF) ────────────────────────────────────────

const PI: f32 = 3.14159265359;

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom + 0.0001);
}

fn geometry_schlick(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k + 0.0001);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return geometry_schlick(n_dot_v, roughness) * geometry_schlick(n_dot_l, roughness);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn perturb_normal(pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    let eps = 0.1;
    let scale = 2.0;
    let h0 = triplanar_fbm(pos, n, scale);
    let hx = triplanar_fbm(pos + vec3<f32>(eps, 0.0, 0.0), n, scale);
    let hz = triplanar_fbm(pos + vec3<f32>(0.0, 0.0, eps), n, scale);
    let dx = (hx - h0) / eps;
    let dz = (hz - h0) / eps;
    return normalize(n + vec3<f32>(-dx, 0.0, -dz) * 0.5);
}

fn pbr_lighting(mat: Material, pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let light_color = vec3<f32>(1.4, 1.3, 1.1);
    let view_dir = normalize(uniforms.camera_pos - pos);
    let half_dir = normalize(light_dir + view_dir);
    let n_dot_l = max(dot(n, light_dir), 0.0);
    let n_dot_v = max(dot(n, view_dir), 0.001);
    let n_dot_h = max(dot(n, half_dir), 0.0);
    let h_dot_v = max(dot(half_dir, view_dir), 0.0);
    let f0 = mix(vec3<f32>(0.04), mat.albedo, mat.metallic);
    let D = distribution_ggx(n_dot_h, mat.roughness);
    let G = geometry_smith(n_dot_v, n_dot_l, mat.roughness);
    let F = fresnel_schlick(h_dot_v, f0);
    let spec = (D * G * F) / (4.0 * n_dot_v * n_dot_l + 0.0001);
    let kS = F;
    let kD = (vec3<f32>(1.0) - kS) * (1.0 - mat.metallic);
    let direct = (kD * mat.albedo / PI + spec) * light_color * n_dot_l;
    let sky_c = vec3<f32>(0.5, 0.6, 0.8);
    let ground_c = vec3<f32>(0.15, 0.12, 0.08);
    let amb_blend = n.y * 0.5 + 0.5;
    let ambient = mix(ground_c, sky_c, amb_blend) * mat.albedo * mat.ao * 0.25;
    let rim = pow(1.0 - n_dot_v, 3.0) * 0.06;
    return direct + ambient + vec3<f32>(rim);
}

// ─── Vertex Shader ────────────────────────────────────────────────────────────

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(vertex.position, 1.0);
    output.world_position = vertex.position;
    output.world_normal = normalize(vertex.normal);
    output.uv = vertex.uv;
    output.biome_id = vertex.biome_id;
    return output;
}

// ─── Fragment Shader ──────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Unlit: raw albedo only
    if uniforms.shading_mode == 1u {
        let mat = get_biome_material(in.biome_id, in.world_position, in.world_normal);
        return vec4<f32>(mat.albedo, 1.0);
    }
    // Wireframe
    if uniforms.shading_mode == 2u {
        return vec4<f32>(0.1, 0.1, 0.1, 1.0);
    }

    // Full PBR lit
    var mat = get_biome_material(in.biome_id, in.world_position, in.world_normal);
    mat = apply_slope_blend(mat, in.world_position, in.world_normal);
    let perturbed_n = perturb_normal(in.world_position, in.world_normal);

    // Weather effects on material
    if uniforms.weather_type == 4u {
        let flat_f = max(dot(in.world_normal, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
        let snow_amt = smoothstep(0.4, 0.8, flat_f) * 0.7;
        mat.albedo = mix(mat.albedo, vec3<f32>(0.85, 0.87, 0.92), snow_amt);
        mat.roughness = mix(mat.roughness, 0.6, snow_amt);
        let grain = triplanar_noise(in.world_position, in.world_normal, 15.0);
        mat.albedo += vec3<f32>(grain * 0.02) * snow_amt;
    } else if uniforms.weather_type == 2u || uniforms.weather_type == 3u {
        let wet = select(0.4, 0.7, uniforms.weather_type == 3u);
        mat.roughness = mix(mat.roughness, 0.1, wet);
        mat.albedo *= 1.0 - wet * 0.2;
    } else if uniforms.weather_type == 6u {
        mat.albedo = mix(mat.albedo, vec3<f32>(0.48, 0.28, 0.08), 0.25);
        mat.roughness = min(mat.roughness + 0.15, 1.0);
    }

    var color = pbr_lighting(mat, in.world_position, perturbed_n);

    // Tone map (Reinhard)
    color = color / (color + vec3<f32>(1.0));

    // Fog (height-aware exponential)
    if uniforms.fog_enabled == 1u {
        let dist = distance(uniforms.camera_pos, in.world_position);
        let fog_base = 1.0 - exp(-uniforms.fog_density * dist);
        let height_att = smoothstep(0.0, 30.0, in.world_position.y);
        let fog_f = fog_base * mix(1.3, 0.7, height_att);
        color = mix(color, uniforms.fog_color, clamp(fog_f, 0.0, 1.0));
    }

    return vec4<f32>(color, 1.0);
}
