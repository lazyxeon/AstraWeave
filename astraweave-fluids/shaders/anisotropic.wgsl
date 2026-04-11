// Anisotropic Kernel Compute Shader
// Calculates ellipsoid stretching based on particle velocity for improved visual quality

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    predicted_position: vec4<f32>,
    lambda: f32,
    density: f32,
    phase: u32,
    temperature: f32,
    color: vec4<f32>,
};

struct AnisotropicData {
    // Anisotropic matrix (3x3 stored as 3 vec4s for alignment)
    axis1: vec4<f32>,  // Major axis (velocity direction) + scale
    axis2: vec4<f32>,  // Minor axis 1 + scale
    axis3: vec4<f32>,  // Minor axis 2 + scale
};

struct SimParams {
    smoothing_radius: f32,
    _pad0: f32,
    _pad1: f32,
    particle_count: u32,
};

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;
@group(0) @binding(2) var<storage, read_write> aniso_data: array<AnisotropicData>;

// Minimum and maximum stretch factors
const MIN_STRETCH: f32 = 1.0;
const MAX_STRETCH: f32 = 4.0;
const VELOCITY_SCALE: f32 = 0.15;

@compute @workgroup_size(64)
fn compute_anisotropic(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if (id >= params.particle_count) { return; }

    let particle = particles[id];
    let pos = particle.position.xyz;
    let vel = particle.velocity.xyz;
    let speed = length(vel);
    
    // Base radius
    let r = params.smoothing_radius;
    
    // Calculate stretch factor based on velocity
    // Higher velocity = more stretched ellipsoid
    let stretch = clamp(1.0 + speed * VELOCITY_SCALE, MIN_STRETCH, MAX_STRETCH);
    
    // Primary axis: velocity direction (stretched)
    var axis1: vec3<f32>;
    if (speed > 0.01) {
        axis1 = normalize(vel);
    } else {
        // No velocity: use default up direction
        axis1 = vec3<f32>(0.0, 1.0, 0.0);
    }
    
    // Create orthonormal basis for ellipsoid
    // Find a vector not parallel to axis1
    var arbitrary: vec3<f32>;
    if (abs(axis1.y) < 0.9) {
        arbitrary = vec3<f32>(0.0, 1.0, 0.0);
    } else {
        arbitrary = vec3<f32>(1.0, 0.0, 0.0);
    }
    
    // Gram-Schmidt orthogonalization
    let axis2 = normalize(cross(axis1, arbitrary));
    let axis3 = normalize(cross(axis1, axis2));
    
    // Scale factors: stretch along velocity, compress perpendicular
    // Volume preservation: stretch * compress^2 ≈ 1
    let compress = 1.0 / sqrt(stretch);
    
    // Store anisotropic data
    // axis.w stores the scale factor for that axis
    aniso_data[id].axis1 = vec4<f32>(axis1, r * stretch);      // Stretched along velocity
    aniso_data[id].axis2 = vec4<f32>(axis2, r * compress);     // Compressed perpendicular
    aniso_data[id].axis3 = vec4<f32>(axis3, r * compress);     // Compressed perpendicular
}
