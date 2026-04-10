struct Light {
    position: vec4<f32>,  // w = radius
    color: vec4<f32>,     // w = intensity
}

struct Cluster {
    min_bounds: vec4<f32>,
    max_bounds: vec4<f32>,
    light_offset: u32,
    light_count: u32,
    padding: vec2<u32>,
}

struct ClusterConfig {
    cluster_x: u32,
    cluster_y: u32,
    cluster_z: u32,
    near: f32,
    far: f32,
    _pad: vec3<u32>,
}

@group(4) @binding(0) var<storage, read> lights: array<Light>;
@group(4) @binding(1) var<storage, read> clusters: array<Cluster>;
@group(4) @binding(2) var<storage, read> light_indices: array<u32>;
@group(4) @binding(3) var<uniform> uConfig: ClusterConfig;

fn get_cluster_index(uv: vec2<f32>, view_z: f32) -> u32 {
    let x = u32(uv.x * f32(uConfig.cluster_x));
    let y = u32(uv.y * f32(uConfig.cluster_y));
    
    // Exponential depth mapping
    // z_slice = log(z / near) / log(far / near) * cluster_z
    // view_z is positive here (distance from camera)
    let z_slice = log2(max(view_z, uConfig.near) / uConfig.near) / log2(uConfig.far / uConfig.near);
    let z = u32(max(0.0, z_slice * f32(uConfig.cluster_z)));
    
    // Clamp to ensure we don't go out of bounds
    let cx = min(x, uConfig.cluster_x - 1u);
    let cy = min(y, uConfig.cluster_y - 1u);
    let cz = min(z, uConfig.cluster_z - 1u);
    
    return cx + cy * uConfig.cluster_x + cz * uConfig.cluster_x * uConfig.cluster_y;
}

fn calculate_clustered_lighting(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    view_pos: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    uv: vec2<f32>,
    view_z: f32
) -> vec3<f32> {
    let cluster_idx = get_cluster_index(uv, view_z);
    let cluster = clusters[cluster_idx];
    
    var total_light = vec3<f32>(0.0);
    
    // Iterate through lights in this cluster
    for (var i = 0u; i < cluster.light_count; i = i + 1u) {
        let light_idx = light_indices[cluster.light_offset + i];
        let light = lights[light_idx];
        
        let light_dir = light.position.xyz - world_pos;
        let distance = length(light_dir);
        let radius = light.position.w;
        
        // Skip if outside light radius
        if (distance > radius) {
            continue;
        }
        
        let L = normalize(light_dir);
        let V = normalize(view_pos - world_pos);
        
        let NdotL = max(dot(normal, L), 0.0);
        
        // Attenuation (UE4-style inverse-square falloff with radius)
        let attenuation = 1.0 - pow(distance / radius, 4.0);
        let attenuation_clamped = max(attenuation, 0.0) / (distance * distance + 1.0);
        
        // PBR Cook-Torrance + Burley diffuse (from brdf_common.wgsl)
        let F0 = mix(vec3<f32>(0.04), albedo, metallic);
        let brdf = evaluate_brdf(normal, V, L, albedo, metallic, roughness, F0);

        let light_contribution = brdf * light.color.rgb * light.color.w * attenuation_clamped;
        total_light = total_light + light_contribution;
    }
    
    return total_light;
}
