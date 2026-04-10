//! Complete Clustered Forward Rendering Implementation
//!
//! This module implements a full clustered forward rendering pipeline that supports
//! 100+ dynamic lights efficiently by dividing the screen into 3D clusters and
//! assigning lights to clusters for culling.

use glam::{Mat4, Vec3};

#[cfg(feature = "megalights")]
use crate::clustered_megalights::MegaLightsRenderer;

/// Configuration for clustered rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ClusterConfig {
    /// Number of clusters in X dimension (screen width)
    pub cluster_x: u32,
    /// Number of clusters in Y dimension (screen height)
    pub cluster_y: u32,
    /// Number of clusters in Z dimension (depth)
    pub cluster_z: u32,
    /// Near plane distance
    pub near: f32,
    /// Far plane distance
    pub far: f32,
    /// Padding for 16-byte alignment (total 32 bytes)
    pub _pad: [u32; 3],
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            cluster_x: 16,
            cluster_y: 9, // 16:9 aspect ratio approximation
            cluster_z: 24,
            near: 0.1,
            far: 100.0,
            _pad: [0; 3],
        }
    }
}

/// GPU-compatible light structure
/// Uses arrays instead of Vec4 for Pod/Zeroable compatibility
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuLight {
    /// Position in world space (w = radius)
    pub position: [f32; 4],
    /// Color and intensity (w = intensity)
    pub color: [f32; 4],
}

impl GpuLight {
    pub fn new(position: Vec3, radius: f32, color: Vec3, intensity: f32) -> Self {
        Self {
            position: [position.x, position.y, position.z, radius],
            color: [color.x, color.y, color.z, intensity],
        }
    }
}

/// Cluster data for GPU
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCluster {
    /// Min bounds of cluster in view space
    pub min_bounds: [f32; 4],
    /// Max bounds of cluster in view space
    pub max_bounds: [f32; 4],
    /// Offset into light index list
    pub light_offset: u32,
    /// Number of lights in this cluster
    pub light_count: u32,
    /// Padding
    pub _padding: [u32; 2],
}

/// Clustered forward renderer implementation
#[allow(dead_code)]
pub struct ClusteredForwardRenderer {
    config: ClusterConfig,

    // GPU resources
    light_buffer: wgpu::Buffer,
    cluster_buffer: wgpu::Buffer,
    light_indices_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    config_buffer: wgpu::Buffer,

    // MegaLights GPU culling buffers
    #[cfg(feature = "megalights")]
    light_counts_buffer: wgpu::Buffer,
    #[cfg(feature = "megalights")]
    light_offsets_buffer: wgpu::Buffer,
    #[cfg(feature = "megalights")]
    cluster_bounds_buffer: wgpu::Buffer,
    #[cfg(feature = "megalights")]
    params_buffer: wgpu::Buffer,
    #[cfg(feature = "megalights")]
    prefix_sum_params_buffer: wgpu::Buffer,

    // MegaLights renderer
    #[cfg(feature = "megalights")]
    megalights: Option<MegaLightsRenderer>,

    // Bind groups
    cluster_bind_group_layout: wgpu::BindGroupLayout,
    cluster_bind_group: wgpu::BindGroup,

    // CPU-side data
    lights: Vec<GpuLight>,
    clusters: Vec<GpuCluster>,
    light_indices: Vec<u32>,

    // Capacity
    _max_lights: usize,
    max_lights_per_cluster: usize,
}

impl ClusteredForwardRenderer {
    /// Create a new clustered forward renderer
    pub fn new(device: &wgpu::Device, config: ClusterConfig) -> Self {
        let max_lights = 256;
        let max_lights_per_cluster = 128;
        let total_clusters = (config.cluster_x * config.cluster_y * config.cluster_z) as usize;

        // Create buffers
        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Clustered Light Buffer"),
            size: (max_lights * std::mem::size_of::<GpuLight>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cluster_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cluster Buffer"),
            size: (total_clusters * std::mem::size_of::<GpuCluster>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_indices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Indices Buffer"),
            size: (total_clusters * max_lights_per_cluster * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let config_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cluster Config Buffer"),
            size: std::mem::size_of::<ClusterConfig>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        config_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::bytes_of(&config));
        config_buffer.unmap();

        // Create bind group layout
        let cluster_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Cluster Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create bind group
        let cluster_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cluster Bind Group"),
            layout: &cluster_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: cluster_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: light_indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: config_buffer.as_entire_binding(),
                },
            ],
        });

        // MegaLights GPU culling buffers (feature-gated)
        #[cfg(feature = "megalights")]
        let light_counts_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MegaLights Counts Buffer"),
            size: (total_clusters * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[cfg(feature = "megalights")]
        let light_offsets_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MegaLights Offsets Buffer"),
            size: (total_clusters * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[cfg(feature = "megalights")]
        let cluster_bounds_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MegaLights Cluster Bounds Buffer"),
            size: (total_clusters * 32) as u64, // 32 bytes per ClusterBounds
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[cfg(feature = "megalights")]
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MegaLights Params Buffer"),
            size: 32, // ClusterParams struct (32 bytes)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        #[cfg(feature = "megalights")]
        let prefix_sum_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MegaLights Prefix Sum Params Buffer"),
            size: 16, // PrefixSumParams struct (16 bytes)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize MegaLights renderer
        #[cfg(feature = "megalights")]
        let mut megalights = MegaLightsRenderer::new(
            device,
            (config.cluster_x, config.cluster_y, config.cluster_z),
            max_lights,
        )
        .ok(); // Use .ok() to convert Result to Option (log errors in production)

        // Update bind groups with buffers
        #[cfg(feature = "megalights")]
        if let Some(ref mut ml) = megalights {
            ml.update_bind_groups(
                device,
                &light_buffer,
                &cluster_bounds_buffer,
                &light_counts_buffer,
                &light_offsets_buffer,
                &light_indices_buffer,
                &params_buffer,
                &prefix_sum_params_buffer,
            );
        }

        Self {
            config,
            light_buffer,
            cluster_buffer,
            light_indices_buffer,
            config_buffer,

            #[cfg(feature = "megalights")]
            light_counts_buffer,
            #[cfg(feature = "megalights")]
            light_offsets_buffer,
            #[cfg(feature = "megalights")]
            cluster_bounds_buffer,
            #[cfg(feature = "megalights")]
            params_buffer,
            #[cfg(feature = "megalights")]
            prefix_sum_params_buffer,
            #[cfg(feature = "megalights")]
            megalights,

            cluster_bind_group_layout,
            cluster_bind_group,
            lights: Vec::new(),
            clusters: vec![
                GpuCluster {
                    min_bounds: [0.0; 4],
                    max_bounds: [0.0; 4],
                    light_offset: 0,
                    light_count: 0,
                    _padding: [0; 2],
                };
                total_clusters
            ],
            light_indices: Vec::new(),
            _max_lights: max_lights,
            max_lights_per_cluster,
        }
    }

    /// Update lights for the current frame
    pub fn update_lights(&mut self, lights: Vec<GpuLight>) {
        self.lights = lights;
    }

    /// Build clusters and assign lights
    pub fn build_clusters(
        &mut self,
        queue: &wgpu::Queue,
        view_matrix: Mat4,
        _proj_matrix: Mat4,
        screen_size: (u32, u32),
    ) {
        // Upload lights to GPU first (needed by both CPU and GPU paths)
        if !self.lights.is_empty() {
            queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&self.lights));
        }

        // CPU path (GPU requires device + encoder which we don't have here)
        // For GPU dispatch, use build_clusters_with_encoder()
        self.build_clusters_cpu(queue, view_matrix, screen_size);
    }

    /// Build clusters with GPU dispatch (requires device + encoder for compute shaders)
    #[cfg(feature = "megalights")]
    pub fn build_clusters_with_encoder(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view_matrix: Mat4,
        _proj_matrix: Mat4,
        screen_size: (u32, u32),
    ) {
        // Upload lights to GPU
        if !self.lights.is_empty() {
            queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&self.lights));
        }

        // Try GPU path if MegaLights available
        if self.megalights.is_some() && !self.lights.is_empty() {
            if self
                .build_clusters_gpu(device, queue, encoder, view_matrix, screen_size)
                .is_ok()
            {
                return; // GPU path succeeded
            }
            log::warn!("MegaLights GPU dispatch failed, falling back to CPU");
        }

        // CPU fallback
        self.build_clusters_cpu(queue, view_matrix, screen_size);
    }

    /// CPU light binning (fallback when MegaLights unavailable)
    fn build_clusters_cpu(
        &mut self,
        queue: &wgpu::Queue,
        view_matrix: Mat4,
        screen_size: (u32, u32),
    ) {
        // Clear previous data
        self.light_indices.clear();

        let (_width, _height) = screen_size;
        let _total_clusters =
            (self.config.cluster_x * self.config.cluster_y * self.config.cluster_z) as usize;

        // Calculate cluster bounds in view space
        for z in 0..self.config.cluster_z {
            for y in 0..self.config.cluster_y {
                for x in 0..self.config.cluster_x {
                    let cluster_idx = self.cluster_index(x, y, z);

                    // Calculate cluster bounds
                    let min_x = (x as f32 / self.config.cluster_x as f32) * 2.0 - 1.0;
                    let max_x = ((x + 1) as f32 / self.config.cluster_x as f32) * 2.0 - 1.0;
                    let min_y = (y as f32 / self.config.cluster_y as f32) * 2.0 - 1.0;
                    let max_y = ((y + 1) as f32 / self.config.cluster_y as f32) * 2.0 - 1.0;

                    // Exponential depth slicing for better distribution
                    let near = self.config.near;
                    let far = self.config.far;
                    let min_z = near * (far / near).powf(z as f32 / self.config.cluster_z as f32);
                    let max_z =
                        near * (far / near).powf((z + 1) as f32 / self.config.cluster_z as f32);

                    self.clusters[cluster_idx].min_bounds = [min_x, min_y, min_z, 1.0];
                    self.clusters[cluster_idx].max_bounds = [max_x, max_y, max_z, 1.0];
                    self.clusters[cluster_idx].light_offset = self.light_indices.len() as u32;
                    self.clusters[cluster_idx].light_count = 0;

                    // Assign lights to this cluster
                    for (light_idx, light) in self.lights.iter().enumerate() {
                        if self.light_intersects_cluster(
                            light,
                            &self.clusters[cluster_idx],
                            view_matrix,
                        ) && self.clusters[cluster_idx].light_count
                            < self.max_lights_per_cluster as u32
                        {
                            self.light_indices.push(light_idx as u32);
                            self.clusters[cluster_idx].light_count += 1;
                        }
                    }
                }
            }
        }

        // Upload cluster data and light indices to GPU
        queue.write_buffer(
            &self.cluster_buffer,
            0,
            bytemuck::cast_slice(&self.clusters),
        );
        if !self.light_indices.is_empty() {
            queue.write_buffer(
                &self.light_indices_buffer,
                0,
                bytemuck::cast_slice(&self.light_indices),
            );
        }
    }

    /// GPU light binning using MegaLights compute shaders
    #[cfg(feature = "megalights")]
    fn build_clusters_gpu(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        _view_matrix: Mat4,
        _screen_size: (u32, u32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Calculate cluster bounds in view space
        // TODO: Move this to GPU compute shader for better performance
        for z in 0..self.config.cluster_z {
            for y in 0..self.config.cluster_y {
                for x in 0..self.config.cluster_x {
                    let cluster_idx = self.cluster_index(x, y, z);

                    // Calculate cluster bounds (same as CPU path)
                    let min_x = (x as f32 / self.config.cluster_x as f32) * 2.0 - 1.0;
                    let max_x = ((x + 1) as f32 / self.config.cluster_x as f32) * 2.0 - 1.0;
                    let min_y = (y as f32 / self.config.cluster_y as f32) * 2.0 - 1.0;
                    let max_y = ((y + 1) as f32 / self.config.cluster_y as f32) * 2.0 - 1.0;

                    let near = self.config.near;
                    let far = self.config.far;
                    let min_z = near * (far / near).powf(z as f32 / self.config.cluster_z as f32);
                    let max_z =
                        near * (far / near).powf((z + 1) as f32 / self.config.cluster_z as f32);

                    self.clusters[cluster_idx].min_bounds = [min_x, min_y, min_z, 1.0];
                    self.clusters[cluster_idx].max_bounds = [max_x, max_y, max_z, 1.0];
                }
            }
        }

        // Upload cluster bounds to GPU (for MegaLights shaders)
        // Convert GpuCluster to ClusterBounds format
        let cluster_bounds: Vec<[f32; 8]> = self
            .clusters
            .iter()
            .map(|c| {
                [
                    c.min_bounds[0],
                    c.min_bounds[1],
                    c.min_bounds[2],
                    0.0, // min + padding
                    c.max_bounds[0],
                    c.max_bounds[1],
                    c.max_bounds[2],
                    0.0, // max + padding
                ]
            })
            .collect();

        queue.write_buffer(
            &self.cluster_bounds_buffer,
            0,
            bytemuck::cast_slice(&cluster_bounds),
        );

        // Upload params for GPU shaders
        let total_clusters = self.config.cluster_x * self.config.cluster_y * self.config.cluster_z;
        let cluster_params = [
            self.config.cluster_x,
            self.config.cluster_y,
            self.config.cluster_z,
            0, // padding
            total_clusters,
            self.lights.len() as u32,
            0, // padding
            0, // padding
        ];
        queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::cast_slice(&cluster_params),
        );

        let prefix_sum_params = [
            total_clusters,
            256, // workgroup_size
            0,   // padding
            0,   // padding
        ];
        queue.write_buffer(
            &self.prefix_sum_params_buffer,
            0,
            bytemuck::cast_slice(&prefix_sum_params),
        );

        // Dispatch MegaLights GPU compute pipeline
        if let Some(ref megalights) = self.megalights {
            megalights.dispatch(encoder, self.lights.len() as u32)?;
        }

        // Note: Results are written directly to light_indices_buffer by GPU
        // No need to read back or update CPU-side data structures

        Ok(())
    }

    /// Get cluster index from 3D coordinates
    fn cluster_index(&self, x: u32, y: u32, z: u32) -> usize {
        (x + y * self.config.cluster_x + z * self.config.cluster_x * self.config.cluster_y) as usize
    }

    /// Check if a light intersects a cluster
    fn light_intersects_cluster(
        &self,
        light: &GpuLight,
        cluster: &GpuCluster,
        view_matrix: Mat4,
    ) -> bool {
        // Transform light position to view space
        let light_pos_4 =
            Vec3::new(light.position[0], light.position[1], light.position[2]).extend(1.0);
        let light_pos_view_4 = view_matrix * light_pos_4;
        let light_pos_view = Vec3::new(light_pos_view_4.x, light_pos_view_4.y, light_pos_view_4.z);
        let radius = light.position[3];

        // Simple sphere-AABB intersection test
        let closest_point = Vec3::new(
            light_pos_view
                .x
                .clamp(cluster.min_bounds[0], cluster.max_bounds[0]),
            light_pos_view
                .y
                .clamp(cluster.min_bounds[1], cluster.max_bounds[1]),
            light_pos_view
                .z
                .clamp(cluster.min_bounds[2], cluster.max_bounds[2]),
        );

        let distance_sq = (light_pos_view - closest_point).length_squared();
        distance_sq <= radius * radius
    }

    /// Get the bind group layout
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.cluster_bind_group_layout
    }

    /// Get the bind group
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.cluster_bind_group
    }

    /// Get configuration
    pub fn config(&self) -> &ClusterConfig {
        &self.config
    }

    /// Get light count
    pub fn light_count(&self) -> usize {
        self.lights.len()
    }
}

/// WGSL shader code for clustered forward rendering.
/// Consuming shaders must prepend `brdf_common.wgsl` before this code.
pub const CLUSTERED_LIGHTING_SHADER: &str = r#"
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

@group(2) @binding(0) var<storage, read> lights: array<Light>;
@group(2) @binding(1) var<storage, read> clusters: array<Cluster>;
@group(2) @binding(2) var<storage, read> light_indices: array<u32>;

struct ClusterConfig {
    cluster_x: u32,
    cluster_y: u32,
    cluster_z: u32,
    near: f32,
    far: f32,
}

fn get_cluster_index(frag_coord: vec3<f32>, config: ClusterConfig) -> u32 {
    let x = u32(frag_coord.x / config.cluster_x);
    let y = u32(frag_coord.y / config.cluster_y);
    
    // Exponential depth mapping
    let depth = frag_coord.z;
    let z_slice = log2(depth / config.near) / log2(config.far / config.near);
    let z = u32(z_slice * f32(config.cluster_z));
    
    return x + y * config.cluster_x + z * config.cluster_x * config.cluster_y;
}

fn calculate_clustered_lighting(
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    view_pos: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    frag_coord: vec3<f32>,
    config: ClusterConfig
) -> vec3<f32> {
    let cluster_idx = get_cluster_index(frag_coord, config);
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
        
        // Attenuation
        let attenuation = 1.0 - pow(distance / radius, 4.0);
        let attenuation_clamped = max(attenuation, 0.0);
        
        // PBR Cook-Torrance + Burley diffuse (from brdf_common.wgsl)
        let F0 = mix(vec3<f32>(0.04), albedo, metallic);
        let brdf = evaluate_brdf(normal, V, L, albedo, metallic, roughness, F0);

        let light_contribution = brdf * light.color.rgb * light.color.w * attenuation_clamped;
        total_light = total_light + light_contribution;
    }
    
    return total_light;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn test_cluster_config_default() {
        let config = ClusterConfig::default();
        assert_eq!(config.cluster_x, 16);
        assert_eq!(config.cluster_y, 9);
        assert_eq!(config.cluster_z, 24);
    }

    #[test]
    fn test_gpu_light_creation() {
        let light = GpuLight::new(
            Vec3::new(1.0, 2.0, 3.0),
            10.0,
            Vec3::new(1.0, 0.5, 0.0),
            2.0,
        );

        assert_eq!(light.position[0], 1.0);
        assert_eq!(light.position[3], 10.0);
        assert_eq!(light.color[3], 2.0);
    }

    // --- Mutation-resistant tests ---

    #[test]
    fn cluster_config_default_near_far() {
        let c = ClusterConfig::default();
        assert!((c.near - 0.1).abs() < f32::EPSILON);
        assert!((c.far - 100.0).abs() < f32::EPSILON);
        assert_eq!(c._pad, [0; 3]);
    }

    #[test]
    fn cluster_config_total_clusters() {
        let c = ClusterConfig::default();
        let total = c.cluster_x * c.cluster_y * c.cluster_z;
        assert_eq!(total, 16 * 9 * 24);
        assert_eq!(total, 3456);
    }

    #[test]
    fn cluster_config_struct_size() {
        // 5 fields × 4 bytes + 3 padding × 4 bytes = 32 bytes
        assert_eq!(std::mem::size_of::<ClusterConfig>(), 32);
    }

    #[test]
    fn gpu_light_stores_all_fields() {
        let light = GpuLight::new(
            Vec3::new(10.0, 20.0, 30.0),
            5.0,
            Vec3::new(0.1, 0.2, 0.3),
            7.5,
        );
        assert_eq!(light.position, [10.0, 20.0, 30.0, 5.0]);
        assert_eq!(light.color, [0.1, 0.2, 0.3, 7.5]);
    }

    #[test]
    fn gpu_light_struct_size() {
        // 2 × vec4<f32> = 32 bytes
        assert_eq!(std::mem::size_of::<GpuLight>(), 32);
    }

    #[test]
    fn gpu_cluster_struct_size() {
        // min_bounds(16) + max_bounds(16) + offset(4) + count(4) + pad(8) = 48 bytes
        assert_eq!(std::mem::size_of::<GpuCluster>(), 48);
    }

    #[test]
    fn gpu_light_zeroed_is_valid() {
        let light = GpuLight::zeroed();
        assert_eq!(light.position, [0.0; 4]);
        assert_eq!(light.color, [0.0; 4]);
    }
}
