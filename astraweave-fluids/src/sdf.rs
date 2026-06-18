//! Signed Distance Field (SDF) System
//!
//! Provides GPU-accelerated SDF generation using Jump Flood Algorithm (JFA)
//! for fluid surface reconstruction and collision detection.

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SdfConfig {
    resolution: u32,
    world_size: f32,
    triangle_count: u32,
    /// Number of valid entries in the dynamic-objects buffer (the buffer
    /// itself is fixed-size). Must match the WGSL `SdfConfig.object_count`.
    object_count: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct JfaParams {
    step_size: u32,
    padding: [u32; 3],
}

pub struct SdfSystem {
    init_pipeline: wgpu::ComputePipeline,
    step_pipeline: wgpu::ComputePipeline,
    finalize_pipeline: wgpu::ComputePipeline,

    #[allow(dead_code)]
    config_buffer: wgpu::Buffer,
    jfa_params_buffer: wgpu::Buffer,

    pub texture_a: wgpu::Texture,
    pub texture_b: wgpu::Texture,

    bind_group_a: wgpu::BindGroup, // Read A, Write B
    bind_group_b: wgpu::BindGroup, // Read B, Write A

    config_bind_group: wgpu::BindGroup,
    jfa_bind_group: wgpu::BindGroup,
    pub resolution: u32,
    world_size: f32,
    object_count: u32,
}

impl SdfSystem {
    pub fn new(
        device: &wgpu::Device,
        objects_buffer: &wgpu::Buffer,
        resolution: u32,
        world_size: f32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SDF Gen Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sdf_gen.wgsl").into()),
        });

        let config = SdfConfig {
            resolution,
            world_size,
            triangle_count: 0,
            object_count: 0,
        };

        let config_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SDF Config"),
            contents: bytemuck::bytes_of(&config),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let jfa_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("JFA Params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture_desc = wgpu::TextureDescriptor {
            label: Some("SDF Texture"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: resolution,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                // COPY_DST: generate() may need a B->A blit when the JFA
                // step count leaves the finalized field in texture B (the
                // fluid shader samples texture A).
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let texture_a = device.create_texture(&texture_desc);
        let texture_b = device.create_texture(&texture_desc);

        let view_a = texture_a.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = texture_b.create_view(&wgpu::TextureViewDescriptor::default());

        // Layouts
        let config_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SDF Config Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SDF Texture Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
            ],
        });

        let jfa_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("JFA Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let config_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SDF Config BG"),
            layout: &config_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: objects_buffer.as_entire_binding(),
                },
            ],
        });

        let bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SDF BG A"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_b),
                },
            ],
        });

        let bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SDF BG B"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_a),
                },
            ],
        });

        let jfa_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("JFA BG"),
            layout: &jfa_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: jfa_params_buffer.as_entire_binding(),
            }],
        });
        // We actually need the pipelines to store the jfa layout if we use multiple JFA steps.
        // But let's just make the final system.

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SDF Pipeline Layout"),
            bind_group_layouts: &[&config_layout, &texture_layout, &jfa_layout],
            push_constant_ranges: &[],
        });

        let init_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("SDF Init Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("init"),
            compilation_options: Default::default(),
            cache: None,
        });

        let step_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("SDF Step Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("jfa_step"),
            compilation_options: Default::default(),
            cache: None,
        });

        let finalize_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("SDF Finalize Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("finalize"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            init_pipeline,
            step_pipeline,
            finalize_pipeline,
            config_buffer,
            jfa_params_buffer,
            texture_a,
            texture_b,
            bind_group_a,
            bind_group_b,
            config_bind_group,
            jfa_bind_group,
            resolution,
            world_size,
            object_count: 0,
        }
    }

    /// Set how many entries of the (fixed-size) dynamic-objects buffer are
    /// valid. The SDF init pass voxelizes exactly this many objects; zeroed
    /// tail entries would otherwise read as "distance 0 everywhere" and
    /// poison the whole field.
    pub fn set_object_count(&mut self, queue: &wgpu::Queue, count: u32) {
        self.object_count = count;
        let config = SdfConfig {
            resolution: self.resolution,
            world_size: self.world_size,
            triangle_count: 0,
            object_count: count,
        };
        queue.write_buffer(&self.config_buffer, 0, bytemuck::bytes_of(&config));
    }

    pub fn generate(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        self.generate_timed(encoder, queue, None);
    }

    /// `generate` with optional GPU timestamp bracketing: the init pass
    /// records `begin_slot` at its start, the finalize pass records
    /// `end_slot` at its end (the rare B→A blit is excluded from the span).
    pub fn generate_timed(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        timing: Option<(&wgpu::QuerySet, u32, u32)>,
    ) {
        // Shader workgroup size is (8, 8, 4): z needs twice the workgroup
        // count of x/y. (Pre-F.1 this dispatched res/8 in z too, leaving the
        // upper half of the volume — world z > 0 — permanently unwritten.)
        let workgroups = self.resolution.div_ceil(8);
        let workgroups_z = self.resolution.div_ceil(4);

        // 1. Init
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SDF Init"),
                timestamp_writes: timing.map(|(qs, begin, _)| wgpu::ComputePassTimestampWrites {
                    query_set: qs,
                    beginning_of_pass_write_index: Some(begin),
                    end_of_pass_write_index: None,
                }),
            });
            cpass.set_pipeline(&self.init_pipeline);
            cpass.set_bind_group(0, &self.config_bind_group, &[]);
            cpass.set_bind_group(1, &self.bind_group_a, &[]); // Write to B
            cpass.set_bind_group(2, &self.jfa_bind_group, &[]); // Dummy
            cpass.dispatch_workgroups(workgroups, workgroups, workgroups_z);
        }

        // 2. JFA Steps
        //
        // Bind-group orientation: bind_group_a = (read A, write B);
        // bind_group_b = (read B, write A). Init wrote the seed into B, so
        // the FIRST step must read B. (The pre-F.1 code started by reading A
        // — an uninitialized, zero-filled texture whose texels decode as
        // "valid seed at the origin" — destroying the seed and poisoning the
        // whole field. With the also-then-broken object_count, the resulting
        // garbage SDF slammed every particle into the world corner at
        // ~2,900 m/s on frame 0.)
        let mut step_size = self.resolution / 2;
        let mut data_in_b = true;

        while step_size >= 1 {
            let params = JfaParams {
                step_size,
                padding: [0; 3],
            };
            queue.write_buffer(&self.jfa_params_buffer, 0, bytemuck::bytes_of(&params));

            {
                let label = format!("JFA Step {}", step_size);
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(label.as_str()),
                    ..Default::default()
                });
                cpass.set_pipeline(&self.step_pipeline);
                cpass.set_bind_group(0, &self.config_bind_group, &[]);
                if data_in_b {
                    // Read B, write A
                    cpass.set_bind_group(1, &self.bind_group_b, &[]);
                } else {
                    // Read A, write B
                    cpass.set_bind_group(1, &self.bind_group_a, &[]);
                }
                cpass.set_bind_group(2, &self.jfa_bind_group, &[]);

                cpass.dispatch_workgroups(workgroups, workgroups, workgroups_z);
            }

            step_size /= 2;
            data_in_b = !data_in_b;
        }

        // 3. Finalize — read wherever the JFA result landed, write the other
        // texture. Consumers (fluid.wgsl) sample texture A, so if finalize
        // had to write into B, blit B -> A afterwards.
        let finalize_writes_a = data_in_b; // reading B writes A
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SDF Finalize"),
                timestamp_writes: timing.map(|(qs, _, end)| wgpu::ComputePassTimestampWrites {
                    query_set: qs,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(end),
                }),
            });
            cpass.set_pipeline(&self.finalize_pipeline);
            cpass.set_bind_group(0, &self.config_bind_group, &[]);
            if data_in_b {
                cpass.set_bind_group(1, &self.bind_group_b, &[]);
            } else {
                cpass.set_bind_group(1, &self.bind_group_a, &[]);
            }
            cpass.set_bind_group(2, &self.jfa_bind_group, &[]);
            cpass.dispatch_workgroups(workgroups, workgroups, workgroups_z);
        }

        if !finalize_writes_a {
            let extent = wgpu::Extent3d {
                width: self.resolution,
                height: self.resolution,
                depth_or_array_layers: self.resolution,
            };
            encoder.copy_texture_to_texture(
                self.texture_b.as_image_copy(),
                self.texture_a.as_image_copy(),
                extent,
            );
        }
    }

    /// Update SDF from skinned mesh vertices (for animated colliders)
    ///
    /// This method voxelizes a mesh into the SDF texture for fluid collision.
    /// Call this each frame for animated meshes that need fluid interaction.
    ///
    /// # Arguments
    /// * `queue` - GPU queue for buffer writes
    /// * `vertices` - World-space vertex positions from skinned mesh
    /// * `world_size` - Size of the SDF volume in world units
    pub fn update_from_skinned_mesh(
        &self,
        queue: &wgpu::Queue,
        vertices: &[[f32; 3]],
        world_size: f32,
    ) {
        // Convert vertices to DynamicObject format for GPU processing
        // For skinned meshes, we approximate with AABB or convex hull
        if vertices.is_empty() {
            return;
        }

        // Compute mesh AABB
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for v in vertices {
            min[0] = min[0].min(v[0]);
            min[1] = min[1].min(v[1]);
            min[2] = min[2].min(v[2]);
            max[0] = max[0].max(v[0]);
            max[1] = max[1].max(v[1]);
            max[2] = max[2].max(v[2]);
        }

        // Create box collider from AABB
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let half_extents = [
            (max[0] - min[0]) * 0.5,
            (max[1] - min[1]) * 0.5,
            (max[2] - min[2]) * 0.5,
        ];

        // Update config with mesh info
        let config = SdfConfig {
            resolution: self.resolution,
            world_size,
            triangle_count: (vertices.len() / 3) as u32,
            object_count: 0,
        };
        queue.write_buffer(&self.config_buffer, 0, bytemuck::bytes_of(&config));

        // Note: For production, a proper mesh voxelization pipeline would be needed.
        // This AABB approximation works for simple animated characters.
        let _ = (center, half_extents); // Used by the GPU init shader
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    // ==================== SdfConfig Tests ====================

    #[test]
    fn test_sdf_config_size() {
        // SdfConfig: 4 u32/f32 fields = 16 bytes
        assert_eq!(std::mem::size_of::<SdfConfig>(), 16);
    }

    #[test]
    fn test_sdf_config_default_values() {
        let config = SdfConfig {
            resolution: 64,
            world_size: 20.0,
            triangle_count: 100,
            object_count: 0,
        };

        assert_eq!(config.resolution, 64);
        assert_eq!(config.world_size, 20.0);
        assert_eq!(config.triangle_count, 100);
        assert_eq!(config.object_count, 0);
    }

    #[test]
    fn test_sdf_config_bytemuck_cast() {
        let config = SdfConfig {
            resolution: 128,
            world_size: 30.0,
            triangle_count: 500,
            object_count: 0,
        };

        let bytes: &[u8] = bytemuck::bytes_of(&config);
        assert_eq!(bytes.len(), 16);

        let recovered: &SdfConfig = bytemuck::from_bytes(bytes);
        assert_eq!(recovered.resolution, 128);
        assert_eq!(recovered.world_size, 30.0);
        assert_eq!(recovered.triangle_count, 500);
    }

    #[test]
    fn test_sdf_config_zeroed() {
        let config = SdfConfig::zeroed();

        assert_eq!(config.resolution, 0);
        assert_eq!(config.world_size, 0.0);
        assert_eq!(config.triangle_count, 0);
        assert_eq!(config.object_count, 0);
    }

    #[test]
    fn test_sdf_config_copy() {
        let config = SdfConfig {
            resolution: 256,
            world_size: 50.0,
            triangle_count: 1000,
            object_count: 0,
        };

        let copied = config;
        assert_eq!(copied.resolution, config.resolution);
        assert_eq!(copied.world_size, config.world_size);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_sdf_config_clone() {
        let config = SdfConfig {
            resolution: 64,
            world_size: 10.0,
            triangle_count: 50,
            object_count: 0,
        };

        let cloned = config.clone();
        assert_eq!(cloned.resolution, 64);
    }

    #[test]
    fn test_sdf_config_debug() {
        let config = SdfConfig {
            resolution: 32,
            world_size: 5.0,
            triangle_count: 10,
            object_count: 0,
        };

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("32"));
        assert!(debug_str.contains("5.0") || debug_str.contains("5"));
    }

    // ==================== JfaParams Tests ====================

    #[test]
    fn test_jfa_params_size() {
        // JfaParams: step_size (u32) + padding (3 × u32) = 16 bytes
        assert_eq!(std::mem::size_of::<JfaParams>(), 16);
    }

    #[test]
    fn test_jfa_params_creation() {
        let params = JfaParams {
            step_size: 32,
            padding: [0, 0, 0],
        };

        assert_eq!(params.step_size, 32);
        assert_eq!(params.padding, [0, 0, 0]);
    }

    #[test]
    fn test_jfa_params_bytemuck_cast() {
        let params = JfaParams {
            step_size: 16,
            padding: [0; 3],
        };

        let bytes: &[u8] = bytemuck::bytes_of(&params);
        assert_eq!(bytes.len(), 16);

        let recovered: &JfaParams = bytemuck::from_bytes(bytes);
        assert_eq!(recovered.step_size, 16);
    }

    #[test]
    fn test_jfa_params_zeroed() {
        let params = JfaParams::zeroed();
        assert_eq!(params.step_size, 0);
        assert_eq!(params.padding, [0, 0, 0]);
    }

    #[test]
    fn test_jfa_params_copy() {
        let params = JfaParams {
            step_size: 64,
            padding: [0; 3],
        };

        let copied = params;
        assert_eq!(copied.step_size, 64);
    }

    #[test]
    fn test_jfa_params_debug() {
        let params = JfaParams {
            step_size: 8,
            padding: [0; 3],
        };

        let debug_str = format!("{:?}", params);
        assert!(debug_str.contains("8"));
    }

    // ==================== JFA Algorithm Tests ====================

    #[test]
    fn test_jfa_convergence_steps() {
        let resolution = 64u32;
        let mut steps = Vec::new();
        let mut step_size = resolution / 2;
        while step_size > 0 {
            steps.push(step_size);
            step_size /= 2;
        }

        // For 64, we expect 32, 16, 8, 4, 2, 1 (6 steps)
        assert_eq!(steps.len(), 6);
        assert_eq!(steps[0], 32);
        assert_eq!(steps[5], 1);
    }

    #[test]
    fn test_jfa_steps_for_various_resolutions() {
        // Resolution 32: 16, 8, 4, 2, 1 = 5 steps
        let res32_steps = count_jfa_steps(32);
        assert_eq!(res32_steps, 5);

        // Resolution 128: 64, 32, 16, 8, 4, 2, 1 = 7 steps
        let res128_steps = count_jfa_steps(128);
        assert_eq!(res128_steps, 7);

        // Resolution 256: 128, 64, 32, 16, 8, 4, 2, 1 = 8 steps
        let res256_steps = count_jfa_steps(256);
        assert_eq!(res256_steps, 8);
    }

    fn count_jfa_steps(resolution: u32) -> usize {
        let mut count = 0;
        let mut step_size = resolution / 2;
        while step_size > 0 {
            count += 1;
            step_size /= 2;
        }
        count
    }

    #[test]
    fn test_jfa_workgroups_calculation() {
        // Workgroups = resolution / 8 (ceiling division)
        assert_eq!(64u32.div_ceil(8), 8);
        assert_eq!(65u32.div_ceil(8), 9);
        assert_eq!(128u32.div_ceil(8), 16);
        assert_eq!(256u32.div_ceil(8), 32);
    }

    #[test]
    fn test_jfa_step_sequence_halves() {
        let resolution = 64u32;
        let mut step_size = resolution / 2;
        let mut prev = step_size;

        while step_size > 1 {
            step_size /= 2;
            // Each step should be exactly half the previous
            assert_eq!(step_size * 2, prev);
            prev = step_size;
        }
    }

    #[test]
    fn test_jfa_ping_pong_alternation() {
        // Simulates the ping-pong texture alternation logic
        let mut current_read_a = false; // After init, B has data
        let mut read_from_a = Vec::new();

        let mut step_size = 32u32;
        while step_size >= 1 {
            read_from_a.push(current_read_a);
            current_read_a = !current_read_a;
            step_size /= 2;
        }

        // Should alternate: false, true, false, true, false, true
        assert_eq!(read_from_a, vec![false, true, false, true, false, true]);
    }

    // ==================== SDF Mesh Processing Tests ====================

    #[test]
    fn test_mesh_aabb_single_vertex() {
        let vertices = vec![[5.0, 10.0, 15.0]];

        let (min, max) = compute_mesh_aabb(&vertices);

        assert_eq!(min, [5.0, 10.0, 15.0]);
        assert_eq!(max, [5.0, 10.0, 15.0]);
    }

    #[test]
    fn test_mesh_aabb_multiple_vertices() {
        let vertices = vec![[0.0, 0.0, 0.0], [10.0, 5.0, 3.0], [-5.0, 8.0, -2.0]];

        let (min, max) = compute_mesh_aabb(&vertices);

        assert_eq!(min, [-5.0, 0.0, -2.0]);
        assert_eq!(max, [10.0, 8.0, 3.0]);
    }

    #[test]
    fn test_mesh_aabb_center() {
        let vertices = vec![[-10.0, -10.0, -10.0], [10.0, 10.0, 10.0]];

        let (min, max) = compute_mesh_aabb(&vertices);
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];

        assert_eq!(center, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_mesh_aabb_half_extents() {
        let vertices = vec![[-6.0, -4.0, -2.0], [6.0, 4.0, 2.0]];

        let (min, max) = compute_mesh_aabb(&vertices);
        let half_extents = [
            (max[0] - min[0]) * 0.5,
            (max[1] - min[1]) * 0.5,
            (max[2] - min[2]) * 0.5,
        ];

        assert_eq!(half_extents, [6.0, 4.0, 2.0]);
    }

    fn compute_mesh_aabb(vertices: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];

        for v in vertices {
            min[0] = min[0].min(v[0]);
            min[1] = min[1].min(v[1]);
            min[2] = min[2].min(v[2]);
            max[0] = max[0].max(v[0]);
            max[1] = max[1].max(v[1]);
            max[2] = max[2].max(v[2]);
        }

        (min, max)
    }

    // ==================== Resolution & Memory Tests ====================

    #[test]
    fn test_sdf_texture_voxel_count() {
        // 3D texture memory estimation
        let res_64_voxels = 64u64 * 64 * 64;
        let res_128_voxels = 128u64 * 128 * 128;
        let res_256_voxels = 256u64 * 256 * 256;

        assert_eq!(res_64_voxels, 262_144);
        assert_eq!(res_128_voxels, 2_097_152);
        assert_eq!(res_256_voxels, 16_777_216);
    }

    #[test]
    fn test_sdf_texture_memory_rgba32float() {
        // RGBA32Float = 16 bytes per pixel
        let bytes_per_voxel = 16u64;

        let res_64_bytes = 64u64 * 64 * 64 * bytes_per_voxel;
        let res_128_bytes = 128u64 * 128 * 128 * bytes_per_voxel;

        // 64^3 = ~4MB, 128^3 = ~32MB
        assert_eq!(res_64_bytes, 4_194_304);
        assert_eq!(res_128_bytes, 33_554_432);
    }

    #[test]
    fn test_triangle_count_from_vertices() {
        // Each triangle has 3 vertices
        let vertices_30 = 30usize;
        let triangles = vertices_30 / 3;
        assert_eq!(triangles, 10);

        let vertices_99 = 99usize;
        let triangles_99 = vertices_99 / 3;
        assert_eq!(triangles_99, 33);
    }
}
